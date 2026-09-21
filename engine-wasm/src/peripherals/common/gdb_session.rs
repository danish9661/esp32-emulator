use crate::peripherals::types::*;
use crate::peripherals::common::helpers::{bytes_to_hex, hex_to_bytes};
use core::cmp;

const GDB_BUF_SIZE: usize = 8192;
const MAX_CORES: u32 = 2;

fn hex_char(nibble: u8) -> u8 {
    let n = nibble & 0x0F;
    if n < 10 { b'0' + n } else { b'a' + n - 10 }
}

fn hex_val(c: u8) -> u8 {
    match c {
        b'0'..=b'9' => c - b'0',
        b'a'..=b'f' => c - b'a' + 10,
        b'A'..=b'F' => c - b'A' + 10,
        _ => 0,
    }
}

fn compute_checksum(payload: &[u8]) -> u8 {
    let mut sum = 0u32;
    for &b in payload {
        sum = sum.wrapping_add(b as u32);
    }
    (sum & 0xFF) as u8
}

fn write_gdb_packet(payload: &[u8], out: &mut [u8]) -> usize {
    let plen = payload.len();
    let need = plen + 5;
    if out.len() < need { return 0; }
    out[0] = b'+';
    out[1] = b'$';
    out[2..2 + plen].copy_from_slice(payload);
    let csum = compute_checksum(payload);
    out[2 + plen] = b'#';
    out[2 + plen + 1] = hex_char(csum >> 4);
    out[2 + plen + 2] = hex_char(csum & 0x0F);
    need
}

fn parse_hex(bytes: &[u8]) -> u32 {
    let mut val = 0u32;
    for &c in bytes {
        val = (val << 4) | hex_val(c) as u32;
    }
    val
}

fn write_xml_escaped(data: &[u8], out: &mut [u8], pos: &mut usize) {
    for &b in data {
        if *pos >= out.len() { return; }
        match b {
            b'&' => {
                if *pos + 4 >= out.len() { return; }
                out[*pos..*pos + 5].copy_from_slice(b"&amp;");
                *pos += 5;
            }
            b'<' => {
                if *pos + 3 >= out.len() { return; }
                out[*pos..*pos + 4].copy_from_slice(b"&lt;");
                *pos += 4;
            }
            b'>' => {
                if *pos + 3 >= out.len() { return; }
                out[*pos..*pos + 4].copy_from_slice(b"&gt;");
                *pos += 4;
            }
            _ => { out[*pos] = b; *pos += 1; }
        }
    }
}

fn write_uint32_hex_minimal(val: u32, out: &mut [u8], pos: &mut usize) {
    let mut started = false;
    for i in (0..8).rev() {
        let nibble = (val >> (i * 4)) & 0xF;
        if started || nibble != 0 || i == 0 {
            if *pos >= out.len() { return; }
            out[*pos] = hex_char(nibble as u8);
            *pos += 1;
            started = true;
        }
    }
}

fn write_uint32_hex_full(val: u32, out: &mut [u8], pos: &mut usize) {
    for i in (0..8).rev() {
        if *pos >= out.len() { return; }
        out[*pos] = hex_char(((val >> (i * 4)) & 0xF) as u8);
        *pos += 1;
    }
}

fn write_str(s: &[u8], out: &mut [u8], pos: &mut usize) {
    for &b in s {
        if *pos >= out.len() { return; }
        out[*pos] = b;
        *pos += 1;
    }
}

pub trait GdbTarget {
    fn num_cores(&self) -> u32;
    fn core_enabled(&self, idx: u32) -> u32;
    fn core_gdb_register_count(&self, idx: u32) -> u32;
    fn core_gdb_read_register(&self, idx: u32, reg: u32) -> u32;
    fn core_gdb_write_register(&mut self, idx: u32, reg: u32, val: u32);
    fn core_read_u32(&self, idx: u32, addr: u32) -> u32;
    fn core_read_u8(&self, idx: u32, addr: u32) -> u8;
    fn core_write_u32(&mut self, idx: u32, addr: u32, val: u32);
    fn core_write_u8(&mut self, idx: u32, addr: u32, val: u8);
    fn core_run_instruction(&mut self, idx: u32);
    fn core_is_window_instruction(&self, idx: u32) -> u32;
    fn core_set_breakpoint(&mut self, idx: u32, addr: u32);
    fn core_clear_breakpoint(&mut self, idx: u32, addr: u32);
    fn sim_execute(&mut self);
    fn sim_stop(&mut self);
    fn transport_write(&mut self, data: &[u8]);
    fn target_reset(&mut self);
    fn target_on_break(&mut self, core_idx: u32);
    fn watch_add(&mut self, addr: u32);
    fn watch_del(&mut self, addr: u32);
    fn target_xml(&self) -> (&[u8], u32);
    fn readonly_override(&self) -> u32;
    fn set_readonly_override(&mut self, val: u32);
    fn thread_count(&self) -> u32;
    fn thread_find_by_core(&self, core: u32) -> i32;
    fn thread_get_alias(&self, idx: u32) -> u32;
    fn thread_find_by_alias(&self, alias: u32) -> bool;
    fn thread_read_register(&self, alias: u32, reg: u32) -> u32;
    fn thread_get_name(&self, idx: u32, out: &mut [u8]) -> usize;
}

pub struct GdbSession {
    pub current_core: u32,
    pub current_thread: u32,
    pub buf: [u8; GDB_BUF_SIZE],
    pub buf_len: u32,
}

impl GdbSession {
    pub fn new() -> Self {
        GdbSession {
            current_core: 0,
            current_thread: 0,
            buf: [0u8; GDB_BUF_SIZE],
            buf_len: 0,
        }
    }

    pub fn update_current_thread(&mut self, target: &dyn GdbTarget, core_idx: u32) {
        self.current_core = core_idx;
        self.current_thread = core_idx + 1;
        let found = target.thread_find_by_core(core_idx);
        if found >= 0 {
            self.current_thread = target.thread_get_alias(found as u32);
        }
    }

    pub fn on_break(&mut self, target: &mut dyn GdbTarget, core_idx: u32) {
        self.update_current_thread(target, core_idx);
        let mut payload = [0u8; 64];
        let mut pos = 0usize;
        write_str(b"T05thread:", &mut payload, &mut pos);
        write_uint32_hex_full(self.current_thread, &mut payload, &mut pos);
        payload[pos] = b';'; pos += 1;
        let mut packet = [0u8; 128];
        let pkt_len = write_gdb_packet(&payload[..pos], &mut packet);
        if pkt_len > 0 {
            target.transport_write(&packet[..pkt_len]);
        }
    }

    pub fn handle_command(&mut self, target: &mut dyn GdbTarget, cmd: &[u8], out: &mut [u8]) -> usize {
        if cmd.starts_with(b"qSupported") {
            let mut payload = [0u8; 512];
            let mut pos = 0usize;
            write_str(b"PacketSize=1000;qXfer:threads:read+", &mut payload, &mut pos);
            let (_, xml_len) = target.target_xml();
            if xml_len > 0 {
                write_str(b";qXfer:features:read+", &mut payload, &mut pos);
            }
            write_gdb_packet(&payload[..pos], out)
        } else if cmd.starts_with(b"qAttached") {
            write_gdb_packet(b"1", out)
        } else if cmd.starts_with(b"qfThreadInfo") {
            let tc = target.thread_count();
            if tc > 0 {
                let mut payload = [0u8; 1024];
                let mut pos = 0usize;
                payload[pos] = b'm'; pos += 1;
                for i in 0..tc {
                    if i > 0 { payload[pos] = b','; pos += 1; }
                    let alias = target.thread_get_alias(i);
                    write_uint32_hex_minimal(alias, &mut payload, &mut pos);
                }
                write_gdb_packet(&payload[..pos], out)
            } else if target.num_cores() > 1 {
                write_gdb_packet(b"m 1,2", out)
            } else {
                write_gdb_packet(b"m 1", out)
            }
        } else if cmd.starts_with(b"qsThreadInfo") {
            write_gdb_packet(b"l", out)
        } else if cmd.starts_with(b"qXfer:threads:read::") {
            let tc = target.thread_count();
            if tc > 0 {
                let mut payload = [0u8; 4096];
                let mut pos = 0usize;
                write_str(b"l<?xml version=\"1.0\"?>\n<threads>\n", &mut payload, &mut pos);
                for i in 0..tc {
                    write_str(b"<thread IntStatusAlias=\"", &mut payload, &mut pos);
                    let alias = target.thread_get_alias(i);
                    write_uint32_hex_minimal(alias, &mut payload, &mut pos);
                    write_str(b"\">", &mut payload, &mut pos);
                    let mut name_buf = [0u8; 128];
                    let name_len = target.thread_get_name(i, &mut name_buf);
                    write_xml_escaped(&name_buf[..name_len], &mut payload, &mut pos);
                    write_str(b"</thread>\n", &mut payload, &mut pos);
                }
                write_str(b"</threads>\n", &mut payload, &mut pos);
                write_gdb_packet(&payload[..pos], out)
            } else if target.num_cores() > 1 {
                write_gdb_packet(b"l<?xml version=\"1.0\"?>\n<threads>\n<thread IntStatusAlias=\"1\">Name: esp32.PRO</thread>\n<thread IntStatusAlias=\"2\">Name: esp32.APP</thread>\n</threads>", out)
            } else {
                write_gdb_packet(b"l<?xml version=\"1.0\"?>\n<threads>\n</threads>", out)
            }
        } else if cmd.starts_with(b"qXfer:features:read:target.xml:") {
            let params = &cmd[b"qXfer:features:read:target.xml:".len()..];
            let comma = params.iter().position(|&b| b == b',');
            if comma.is_none() { return write_gdb_packet(b"E00", out); }
            let cm = comma.unwrap();
            let offset = parse_hex(&params[..cm]);
            let byte_len = parse_hex(&params[cm + 1..]);
            let (xml, xml_len) = target.target_xml();
            let more = if xml_len > offset + byte_len { b"m" } else { b"l" };
            let mut payload = [0u8; 8192];
            let mut pos = 0usize;
            payload[pos] = more[0]; pos += 1;
            let available = if offset < xml_len { xml_len - offset } else { 0 };
            let copy_len = cmp::min(byte_len, available) as usize;
            if copy_len > 0 {
                payload[pos..pos + copy_len].copy_from_slice(&xml[offset as usize..offset as usize + copy_len]);
                pos += copy_len;
            }
            write_gdb_packet(&payload[..pos], out)
        } else if cmd.starts_with(b"qRcmd,") {
            let hex_part = &cmd[6..];
            let mut decoded = [0u8; 256];
            let dlen = hex_to_bytes(hex_part, &mut decoded);
            if dlen == 5 && &decoded[..5] == b"reset" {
                target.target_reset();
                write_gdb_packet(b"OK", out)
            } else if dlen == 11 && &decoded[..11] == b"reset halt" {
                target.target_reset();
                write_gdb_packet(b"OK", out)
            } else if dlen == 13 && &decoded[..13] == b"system_reset" {
                target.target_reset();
                write_gdb_packet(b"OK", out)
            } else {
                write_gdb_packet(b"E00", out)
            }
        } else if cmd.len() == 1 && cmd[0] == b'?' {
            write_gdb_packet(b"S05", out)
        } else if cmd == b"qC" {
            let mut payload = [0u8; 20];
            let mut pos = 0usize;
            write_str(b"QC ", &mut payload, &mut pos);
            write_uint32_hex_full(self.current_thread, &mut payload, &mut pos);
            write_gdb_packet(&payload[..pos], out)
        } else if cmd == b"qOffsets" {
            write_gdb_packet(b"Text=0;Data=0;Bss=0", out)
        } else if cmd.starts_with(b"Hc") {
            let thread_id_str = &cmd[2..];
            let thread_id = parse_hex(thread_id_str);
            let num_cores = target.num_cores();
            if thread_id == 0 || (thread_id >= 1 && thread_id <= num_cores) {
                write_gdb_packet(b"OK", out)
            } else if target.thread_find_by_alias(thread_id) {
                write_gdb_packet(b"OK", out)
            } else {
                write_gdb_packet(b"E00", out)
            }
        } else if cmd.starts_with(b"Hg") {
            let thread_id_str = &cmd[2..];
            let thread_id = parse_hex(thread_id_str);
            if target.thread_find_by_alias(thread_id) {
                self.current_thread = thread_id;
            } else if thread_id >= 1 && thread_id <= target.num_cores() {
                self.current_core = thread_id - 1;
                self.current_thread = thread_id;
            } else {
                return write_gdb_packet(b"E00", out);
            }
            write_gdb_packet(b"OK", out)
        } else if cmd.len() == 1 && cmd[0] == b's' {
            let core = self.current_core;
            loop {
                target.core_run_instruction(core);
                if target.core_is_window_instruction(core) == 0 {
                    break;
                }
            }
            write_gdb_packet(b"S05", out)
        } else if cmd.len() == 1 && cmd[0] == b'c' {
            target.sim_execute();
            0
        } else if cmd.len() == 1 && cmd[0] == b'g' {
            let reg_count = target.core_gdb_register_count(self.current_core);
            let use_thread_regs = target.thread_count() > 0 && self.current_thread > target.num_cores();
            let mut regs = [0u32; 256];
            for i in 0..reg_count {
                regs[i as usize] = if target.thread_count() > 0 && use_thread_regs {
                    target.thread_read_register(self.current_thread, i)
                } else {
                    target.core_gdb_read_register(self.current_core, i)
                };
            }
            let mut data = [0u8; 1024];
            for i in 0..reg_count {
                let le = regs[i as usize].to_le_bytes();
                let off = i as usize * 4;
                data[off] = le[0];
                data[off + 1] = le[1];
                data[off + 2] = le[2];
                data[off + 3] = le[3];
            }
            let byte_len = reg_count as usize * 4;
            let mut hex_out = [0u8; 2048];
            let hex_len = bytes_to_hex(&data[..byte_len], &mut hex_out);
            write_gdb_packet(&hex_out[..hex_len], out)
        } else if cmd.starts_with(b"G") {
            let hex_data = &cmd[1..];
            let mut bytes = [0u8; 1024];
            let byte_len = hex_to_bytes(hex_data, &mut bytes);
            let reg_count = byte_len / 4;
            if reg_count < target.core_gdb_register_count(self.current_core) as usize {
                return write_gdb_packet(b"E00", out);
            }
            for i in 0..reg_count {
                let val = u32::from_le_bytes([
                    bytes[i * 4],
                    bytes[i * 4 + 1],
                    bytes[i * 4 + 2],
                    bytes[i * 4 + 3],
                ]);
                target.core_gdb_write_register(self.current_core, i as u32, val);
            }
            write_gdb_packet(b"OK", out)
        } else if cmd.starts_with(b"m") {
            let rest = &cmd[1..];
            let comma = rest.iter().position(|&b| b == b',');
            if comma.is_none() { return write_gdb_packet(b"E05", out); }
            let cm = comma.unwrap();
            let addr = parse_hex(&rest[..cm]);
            let len = parse_hex(&rest[cm + 1..]);
            if len == 0 || len >= 65536 {
                return write_gdb_packet(b"E05", out);
            }
            let aligned = (addr & 3) == 0 && (len & 3) == 0;
            let mut raw = [0u8; 65536];
            if aligned {
                let words = len >> 2;
                for i in 0..words {
                    let val = target.core_read_u32(self.current_core, addr + 4 * i);
                    let le = val.to_le_bytes();
                    let off = i as usize * 4;
                    raw[off] = le[0];
                    raw[off + 1] = le[1];
                    raw[off + 2] = le[2];
                    raw[off + 3] = le[3];
                }
            } else {
                for i in 0..len {
                    raw[i as usize] = target.core_read_u8(self.current_core, addr + i);
                }
            }
            let mut hex_out = [0u8; 131072];
            let hex_len = bytes_to_hex(&raw[..len as usize], &mut hex_out);
            write_gdb_packet(&hex_out[..hex_len], out)
        } else if cmd.starts_with(b"M") {
            let rest = &cmd[1..];
            let colon = rest.iter().position(|&b| b == b':');
            if colon.is_none() { return write_gdb_packet(b"E00", out); }
            let cl = colon.unwrap();
            let addr_len_part = &rest[..cl];
            let comma = addr_len_part.iter().position(|&b| b == b',');
            if comma.is_none() { return write_gdb_packet(b"E00", out); }
            let addr = parse_hex(&addr_len_part[..comma.unwrap()]);
            let len = parse_hex(&addr_len_part[comma.unwrap() + 1..]);
            let data_hex = &rest[cl + 1..];
            let mut bytes = [0u8; 65536];
            let byte_len = hex_to_bytes(data_hex, &mut bytes);
            let copy_len = cmp::min(byte_len, len as usize);
            let saved = target.readonly_override();
            target.set_readonly_override(1);
            let aligned = (addr & 3) == 0 && (len & 3) == 0;
            if aligned {
                let words = copy_len >> 2;
                for i in 0..words {
                    let val = u32::from_le_bytes([
                        bytes[i * 4],
                        bytes[i * 4 + 1],
                        bytes[i * 4 + 2],
                        bytes[i * 4 + 3],
                    ]);
                    target.core_write_u32(self.current_core, addr + 4 * i as u32, val);
                }
            } else {
                for i in 0..copy_len {
                    target.core_write_u8(self.current_core, addr + i as u32, bytes[i]);
                }
            }
            target.set_readonly_override(saved);
            write_gdb_packet(b"OK", out)
        } else if cmd.starts_with(b"T") {
            let thread_id_str = &cmd[1..];
            let thread_id = parse_hex(thread_id_str);
            let found = target.thread_find_by_alias(thread_id);
            let app_core_enabled = if target.num_cores() > 1 { target.core_enabled(1) } else { 0 };
            if thread_id == 1 || found || (thread_id == 2 && app_core_enabled != 0) {
                write_gdb_packet(b"OK", out)
            } else {
                write_gdb_packet(b"XtsState 00", out)
            }
        } else if cmd.starts_with(b"Z0,") {
            let addr_str = &cmd[3..];
            let addr = parse_hex(addr_str);
            for i in 0..target.num_cores() {
                target.core_set_breakpoint(i, addr);
            }
            write_gdb_packet(b"OK", out)
        } else if cmd.starts_with(b"z0,") {
            let addr_str = &cmd[3..];
            let addr = parse_hex(addr_str);
            for i in 0..target.num_cores() {
                target.core_clear_breakpoint(i, addr);
            }
            write_gdb_packet(b"OK", out)
        } else if cmd.starts_with(b"Z2,") {
            let addr_str = &cmd[3..];
            let addr = parse_hex(addr_str);
            target.watch_add(addr);
            write_gdb_packet(b"OK", out)
        } else if cmd.starts_with(b"z2,") {
            let addr_str = &cmd[3..];
            let addr = parse_hex(addr_str);
            target.watch_del(addr);
            write_gdb_packet(b"OK", out)
        } else {
            write_gdb_packet(b"", out)
        }
    }

    pub fn on_data(&mut self, target: &mut dyn GdbTarget, data: &[u8]) {
        let mut current_data = data;
        if current_data.len() > 0 && current_data[0] == 3 {
            target.sim_stop();
            let mut br = [0u8; 16];
            let br_len = write_gdb_packet(b"S02", &mut br);
            target.transport_write(&br[..br_len]);
            self.update_current_thread(target, 0);
            current_data = &current_data[1..];
        }
        let append_len = cmp::min(current_data.len(), GDB_BUF_SIZE - self.buf_len as usize);
        if append_len > 0 {
            self.buf[self.buf_len as usize..self.buf_len as usize + append_len].copy_from_slice(&current_data[..append_len]);
            self.buf_len += append_len as u32;
        }
        loop {
            let mut start = None;
            for i in 0..self.buf_len as usize {
                if self.buf[i] == b'$' {
                    start = Some(i);
                    break;
                }
            }
            let s = match start { Some(v) => v, None => return };
            let mut end = None;
            for i in s..self.buf_len as usize {
                if self.buf[i] == b'#' {
                    end = Some(i);
                    break;
                }
            }
            let e = match end { Some(v) => v, None => return };
            if e < s || e + 2 > self.buf_len as usize {
                return;
            }
            let payload_len = e - s - 1;
            let mut payload_buf = [0u8; 4096];
            let copy_len = cmp::min(payload_len, payload_buf.len());
            payload_buf[..copy_len].copy_from_slice(&self.buf[s + 1..s + 1 + copy_len]);
            let csum_hi = hex_val(self.buf[e + 1]);
            let csum_lo = hex_val(self.buf[e + 2]);
            let expected_csum = (csum_hi << 4) | csum_lo;
            let remaining = self.buf_len as usize - (e + 2);
            if remaining > 0 {
                for i in 0..remaining {
                    self.buf[i] = self.buf[e + 2 + i];
                }
            }
            self.buf_len = remaining as u32;
            if compute_checksum(&payload_buf[..copy_len]) != expected_csum {
                target.transport_write(b"-");
            } else {
                target.transport_write(b"+");
                let mut resp = [0u8; 131088];
                let written = self.handle_command(target, &payload_buf[..copy_len], &mut resp);
                if written > 0 {
                    target.transport_write(&resp[..written]);
                }
            }
        }
    }
}
