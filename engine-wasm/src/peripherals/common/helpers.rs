use crate::peripherals::types::*;
use core::sync::atomic::{AtomicU32, Ordering};

#[derive(Clone, Copy)]
pub struct FieldDescriptor {
    pub reg: u32,
    pub shift: u32,
    pub mask: u32,
}

const TIM_REG3: [&str; 8] = ["esp32", "esp32s2", "esp32s3", "esp32c3", "esp32c2", "esp32c6", "esp32h2", "esp32p4"];

fn hex_char(nibble: u8) -> u8 {
    let n = nibble & 0x0F;
    match n {
        0..=9 => 0x30 + n,
        _ => 0x57 + n,
    }
}

fn hex_nibble(c: u8) -> u8 {
    match c {
        b'0'..=b'9' => c - b'0',
        b'a'..=b'f' => c - b'a' + 10,
        b'A'..=b'F' => c - b'A' + 10,
        _ => 0,
    }
}

pub fn byte_to_hex(val: u32) -> u32 {
    let hi_val = (val >> 4) & 0xF;
    let lo_val = val & 0xF;
    let hi = if hi_val > 9 { 0x57 + hi_val } else { 0x30 + hi_val };
    let lo = if lo_val > 9 { 0x57 + lo_val } else { 0x30 + lo_val };
    (hi << 8) | lo
}

pub fn uint16_to_hex(val: u32) -> u32 {
    (byte_to_hex(val >> 8) << 16) | byte_to_hex(val)
}

pub fn uint32_to_hex(val: u32) -> u64 {
    ((byte_to_hex(val >> 24) as u64) << 48)
    | ((byte_to_hex(val >> 16) as u64) << 32)
    | ((byte_to_hex(val >> 8) as u64) << 16)
    | byte_to_hex(val) as u64
}

pub fn hex_to_bytes(hex: &[u8], out: &mut [u8]) -> usize {
    let len = hex.len() / 2;
    for i in 0..len {
        let hi = hex_nibble(hex[2 * i]);
        let lo = hex_nibble(hex[2 * i + 1]);
        out[i] = (hi << 4) | lo;
    }
    len
}

pub fn bytes_to_hex(data: &[u8], out: &mut [u8]) -> usize {
    for i in 0..data.len() {
        out[2 * i] = hex_char(data[i] >> 4);
        out[2 * i + 1] = hex_char(data[i] & 0x0F);
    }
    data.len() * 2
}

pub fn prescaler_to_divider(val: u32) -> u32 {
    if val == 0 { 65536 } else if val == 1 { 2 } else { val }
}

pub fn crc8(data: &[u8]) -> u8 {
    let mut tmp = 0u8;
    for &byte in data {
        tmp ^= byte;
        for _ in 0..8 {
            let lsb = tmp & 1;
            tmp >>= 1;
            if lsb != 0 {
                tmp ^= 0x8Cu8;
            }
        }
    }
    tmp
}

static RNG_STATE: AtomicU32 = AtomicU32::new(1);

pub fn random_u32() -> u32 {
    let mut state = RNG_STATE.load(Ordering::Relaxed);
    state = state.wrapping_mul(1103515245).wrapping_add(12345);
    RNG_STATE.store(state, Ordering::Relaxed);
    state
}

pub fn filter_out_chips(chips: &[&str]) -> ([&'static str; 8], usize) {
    let mut result = [""; 8];
    let mut count = 0;
    'outer: for i in 0..TIM_REG3.len() {
        for j in 0..chips.len() {
            if TIM_REG3[i] == chips[j] {
                continue 'outer;
            }
        }
        result[count] = TIM_REG3[i];
        count += 1;
    }
    (result, count)
}

pub fn tim_reg4<'a>(list: &[&'a str], exclude: &[&str]) -> ([&'a str; 8], usize) {
    let mut result = [""; 8];
    let mut count = 0;
    'outer: for i in 0..list.len() {
        for j in 0..exclude.len() {
            if list[i] == exclude[j] {
                continue 'outer;
            }
        }
        result[count] = list[i];
        count += 1;
    }
    (result, count)
}

pub fn create_field_descriptor(reg: u32, shift: u32, idx: u32) -> FieldDescriptor {
    FieldDescriptor { reg, shift, mask: (1u32 << idx) - 1 }
}

pub fn read_field_value(val: u32, desc: &FieldDescriptor) -> u32 {
    (val >> desc.shift) & desc.mask
}

pub fn tim_reg5(val: u32, desc: &FieldDescriptor) -> u32 {
    val & !(desc.mask << desc.shift)
}

pub fn extract_arg_ssid(ssid: &str, out: &mut [u8]) -> usize {
    let bytes = ssid.as_bytes();
    let n = bytes.len().min(out.len());
    out[..n].copy_from_slice(&bytes[..n]);
    n
}
