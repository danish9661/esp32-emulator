use crate::peripherals::types::*;

const RSA_REG47: u32 = 97;
const RSA_REG48: u32 = 3;
const RSA_REG49: u32 = 1;
const RSA_REG50: u32 = 8;
const RSA_REG51: u32 = 0;
const RSA_REG52: u32 = 0x1000000;
const RSA_REG53: u32 = 16;
const RSA_REG54: u32 = 255;
const RSA_REG55: u32 = 8;
const RSA_REG56: u32 = 255;
const RSA_REG57: u32 = 0;
const RSA_REG58: u32 = 255;
const RSA_REG59: u32 = 128;
const TX_COMPLETE_EVENT_CHANNEL: u32 = 128;

const REG_COUNT: usize = 1024; // JS parity: PeripheralBase.memory = 4096 bytes (offsets 0x000-0xFFC)

// ESP-NOW TX scratch: holds one outbound 802.11 action MPDU while the
// js_espnow_tx_frame FFI hands it to the host medium hook (same pattern as
// WIFI_AP_SCRATCH; synchronous use, no cross-call state).
static mut ESPNOW_TX_SCRATCH: [u8; 1600] = [0u8; 1600];

pub fn analog_i2c_read_response(cpu_val: u32) -> u32 {
    if cpu_val == RSA_REG48 { RSA_REG49 | RSA_REG50 } else { 0 }
}

pub fn wifi_analog_i2c_read_table(reg: u32) -> u32 {
    match reg {
        36 => 1, 51 => 2, 66 => 3, 81 => 4, 96 => 5, 111 => 6,
        126 => 7, 141 => 8, 156 => 9, 171 => 10, 186 => 11,
        201 => 12, 216 => 13, 252 => 14, _ => 0,
    }
}

pub fn wifi_analog_i2c_read_table_high(reg: u32) -> u32 {
    match reg {
        12 => 1, 17 => 2, 22 => 3, 27 => 4, 32 => 5, 37 => 6,
        42 => 7, 47 => 8, 52 => 9, 57 => 10, 62 => 11, 67 => 12,
        72 => 13, 84 => 14, _ => 0,
    }
}

fn crc8(data: &[u8]) -> u16 {
    let mut crc: u8 = 0;
    for &b in data {
        crc ^= b;
        for _ in 0..8 {
            if crc & 0x80 != 0 {
                crc = (crc << 1) ^ 0x07;
            } else {
                crc <<= 1;
            }
        }
    }
    crc as u16
}

pub struct AnalogRfRegisters {
    pub reg_ff: u32,
    pub reg_freq: u32,
}

pub struct AnalogRfConfig {
    pub rmt_channel_register: AnalogRfRegisters,
    pub freq_mask: u32,
    pub freq_shift: u32,
    pub freq_flag: u32,
    pub table: [u32; 256],
}

pub struct AnalogRfPeripheral {
    pub base_addr: u32,
    pub config: AnalogRfConfig,
    pub last_freq: u32,
    pub wifi_channel: u32,
    pub registers: [u32; REG_COUNT],
}

impl AnalogRfPeripheral {
    pub fn new(base_addr: u32, config: AnalogRfConfig) -> Self {
        AnalogRfPeripheral {
            base_addr,
            config,
            last_freq: 0,
            wifi_channel: 0,
            registers: [0; REG_COUNT],
        }
    }

    fn read_register(&self, offset: u32) -> u32 {
        let idx = (offset >> 2) as usize;
        if idx < REG_COUNT { self.registers[idx] } else { 0 }
    }

    fn write_register(&mut self, offset: u32, val: u32) {
        let idx = (offset >> 2) as usize;
        if idx < REG_COUNT { self.registers[idx] = val; }
    }

    fn write_i2c_register(&mut self, _cpu_val: u32, _tmp_val: u32, _idx_val: u32) {}

    fn read_i2c_register(&mut self, cpu_val: u32, tmp_val: u32) -> u32 {
        if cpu_val == RSA_REG47 { analog_i2c_read_response(tmp_val) } else { 0 }
    }
}

impl MmioPeripheral for AnalogRfPeripheral {
    fn read_u32(&mut self, _ctx: &mut CpuContext, addr: u32) -> u32 {
        let offset = addr.wrapping_sub(self.base_addr);
        let tmp_val = &self.config.rmt_channel_register;
        match offset {
            4 => 0xfdffffff,
            64 | 68 | 76 => 0xffffffff,
            _ if offset == tmp_val.reg_ff => 0xffffffff,
            128 => 255,
            204 => 256,
            368 => self.last_freq << 17,
            _ => self.read_register(offset),
        }
    }

    fn write_u32(&mut self, _ctx: &mut CpuContext, addr: u32, tmp_val: u32) {
        let offset = addr.wrapping_sub(self.base_addr);
        let idx_val = &self.config.rmt_channel_register;
        let clock_event = self.config.freq_mask;
        let simulation_clock = self.config.freq_shift;
        let reg_val = self.config.freq_flag;
        if offset == RSA_REG51 {
            let cpu_val = (tmp_val >> RSA_REG57) & RSA_REG58;
            let idx_val_2 = (tmp_val >> RSA_REG55) & RSA_REG56;
            let clock_event_2 = (tmp_val >> RSA_REG53) & RSA_REG54;
            if tmp_val & RSA_REG52 != 0 {
                self.write_i2c_register(cpu_val, idx_val_2, clock_event_2);
            } else {
                let clock_event_3 = self.read_i2c_register(cpu_val, idx_val_2);
                self.write_register(RSA_REG51, (0xff00ffff & tmp_val) | ((255 & clock_event_3) << 16));
            }
            return;
        }
        if offset == idx_val.reg_freq {
            let cpu_val = (tmp_val >> simulation_clock) & clock_event;
            let idx_val_2 = self.config.table[cpu_val as usize];
            if idx_val_2 != 0 && tmp_val & reg_val != 0 {
                self.wifi_channel = idx_val_2;
                self.last_freq = cpu_val;
            }
        }
        self.write_register(offset, tmp_val);
    }

    fn reset(&mut self) {
        self.last_freq = 0;
        self.wifi_channel = 0;
        self.registers = [0; REG_COUNT];
    }
}

pub struct WifiMacRegisters {
    pub rx_ctrl: u32,
    pub mac_event: u32,
    pub rx_dscr_addr_base: u32,
    pub tx_fifo_complete: u32,
    pub mac_ctrl: u32,
    pub fiq_status: Option<u32>,
    pub tx_ack_status: Option<u32>,
    pub tx_fifo_complete_hal: Option<u32>,
    pub mac_rx_iface0: u32,
    pub mac_rx_iface1: u32,
    pub rx_dscr_base: u32,
    pub mac_event_clear: u32,
    pub tx_fifo_clear: u32,
    pub dma_txbuf0: u32,
    pub dma_txbuf1: u32,
    pub dma_txbuf2: u32,
    pub dma_txbuf3: u32,
    pub dma_txbuf4: u32,
    pub tx_fifo_clear_hal: Option<u32>,
    pub tx_ack_clear: Option<u32>,
    pub rx_last_dscr: Option<u32>,
    pub mac_addr_hi: u32,
    pub mac_addr_lo: u32,
}

pub struct WifiMacConfig {
    pub registers: WifiMacRegisters,
    pub rx_interface_en_bitmask: u32,
    pub header_size: u32,
    pub gpio_pin_nmi_int_ena_pro_dualcore_mask: u32,
    pub gpio_pin_int_ena_pro_singlecore_mask: u32,
    pub pad_rx: u32,
    pub length_offset: u32,
    pub channel_byte_offset: Option<u32>,
    pub rx_event: u32,
    pub irq: u32,
    pub rx_enable_bit: Option<u32>,
    pub tx_header: bool,
    pub efuse_mac_addr: u32,
    pub efuse_mac_no_crc: bool,
}

pub struct DmaDescriptor {
    pub header: u32,
    pub size: u32,
    pub length: u32,
    pub eof: bool,
    pub owner_dma: bool,
    pub buffer_ptr: u32,
    pub next_ptr: u32,
}

pub struct DmaBuffer {
    pub buffer: [u8; 4096],
    pub length: u32,
    pub next_ptr: Option<u32>,
}

pub struct WifiMacPeripheral {
    pub base_addr: u32,
    pub config: WifiMacConfig,
    pub event: u32,
    pub rx_buffer: u32,
    pub rx_interface: u32,
    pub tx_complete_bits: u32,
    pub tx_complete_hal_bits: u32,
    pub tx_ack_status: u32,
    pub rx_enabled: bool,
    pub enabled: bool,
    pub channel: u32,
    pub dma_base: u32,
    pub on_tx: Option<fn(&mut CpuContext, &[u8])>,
    pub on_tx_modem_status: Option<fn(&mut CpuContext)>,
    pub core_read_u8: fn(u32) -> u8,
    pub core_write_u8: fn(u32, u8),
    pub core_read_u32: fn(u32) -> u32,
    pub core_write_u32: fn(u32, u32),
    pub registers: [u32; REG_COUNT],
}

impl WifiMacPeripheral {
    pub fn new(base_addr: u32, config: WifiMacConfig) -> Self {
        WifiMacPeripheral {
            base_addr,
            config,
            event: 0,
            rx_buffer: 0,
            rx_interface: 0,
            tx_complete_bits: 0,
            tx_complete_hal_bits: 0,
            tx_ack_status: 0,
            rx_enabled: false,
            enabled: false,
            channel: 0,
            dma_base: 0x3FF00000, // JS parity: this.cpu.dmaBase (esp32.js) — DMA descriptor base
            on_tx: None,
            on_tx_modem_status: None,
            core_read_u8: |_| 0,
            core_write_u8: |_, _| {},
            core_read_u32: |_| 0,
            core_write_u32: |_, _| {},
            registers: [0; REG_COUNT],
        }
    }

    fn read_register(&self, offset: u32) -> u32 {
        let idx = (offset >> 2) as usize;
        if idx < REG_COUNT { self.registers[idx] } else { 0 }
    }

    // pub: native_wifi_mac_set_mac (native_mmio.rs) writes the MAC regs from
    // the host config — the register file owns all MAC state.
    pub fn write_register(&mut self, offset: u32, val: u32) {
        let idx = (offset >> 2) as usize;
        if idx < REG_COUNT { self.registers[idx] = val; }
    }

    pub fn mac_bytes(&self) -> [u8; 6] {
        let regs = &self.config.registers;
        let tmp_val = self.read_register(regs.mac_addr_hi);
        let idx_val = self.read_register(regs.mac_addr_lo);
        [
            (tmp_val & 255) as u8,
            ((tmp_val >> 8) & 255) as u8,
            ((tmp_val >> 16) & 255) as u8,
            ((tmp_val >> 24) & 255) as u8,
            (idx_val & 255) as u8,
            ((idx_val >> 8) & 255) as u8,
        ]
    }

    pub fn send_frame(&mut self, ctx: &mut CpuContext, cpu_val: &[u8], tmp_val: u32) -> bool {
        let header_size = self.config.header_size as usize;
        let gpio_dualcore_mask = self.config.gpio_pin_nmi_int_ena_pro_dualcore_mask;
        let gpio_singlecore_mask = self.config.gpio_pin_int_ena_pro_singlecore_mask;
        let pad_rx = self.config.pad_rx;
        let length_offset = self.config.length_offset as usize;

        let mut register_type = [0u8; 256];
        if header_size <= 256 {
            for i in 0..header_size {
                register_type[i] = 0;
            }
        }
        let cfg_val = cpu_val.len() as u32 + 4;
        let h_val: u8 = if self.rx_interface != 0 { 32 } else { 16 };
        register_type[0] = ((tmp_val + 96) & 255) as u8;
        register_type[1] = (gpio_dualcore_mask & 255) as u8;
        register_type[2] = (gpio_singlecore_mask & 255) as u8;
        register_type[3] = h_val;

        if let Some(off_val) = self.config.channel_byte_offset {
            let off = off_val as usize;
            if off < 256 && self.channel > 0 {
                register_type[off] = (self.channel & 255) as u8;
            }
        }

        if header_size >= length_offset + 2 {
            let idx = header_size - length_offset;
            let idx2 = idx + 1;
            if idx < 256 {
                register_type[idx] = (cfg_val & 255) as u8;
            }
            if idx2 < 256 {
                register_type[idx2] = ((cfg_val >> 8) & 15) as u8;
            }
            if length_offset > 4 {
                let idx3 = idx + 2;
                let idx4 = idx + 3;
                if idx3 < 256 {
                    register_type[idx3] = ((cfg_val + 4) & 255) as u8;
                }
                if idx4 < 256 {
                    register_type[idx4] = (((cfg_val + 4) >> 8) & 15) as u8;
                }
            }
        }

        if !self.enabled || !self.rx_enabled {
            return false;
        }

        let pad_len = 4 - (cfg_val % 4);
        let len_val = [0u8; 4];
        let val_val = self.rx_buffer;

        let mut combined = [0u8; 2048];
        let mut combined_len = 0usize;
        let copy_len = header_size.min(256);
        combined[..copy_len].copy_from_slice(&register_type[..copy_len]);
        combined_len += copy_len;
        let frame_len = cpu_val.len().min(2048 - combined_len);
        combined[combined_len..combined_len + frame_len].copy_from_slice(&cpu_val[..frame_len]);
        combined_len += frame_len;
        if pad_rx != 0 && pad_len > 0 {
            let pad_copy = (pad_len as usize).min(4).min(2048 - combined_len);
            combined[combined_len..combined_len + pad_copy].copy_from_slice(&len_val[..pad_copy]);
            combined_len += pad_copy;
        }

        self.rx_buffer = self.write_dma_buffer(self.rx_buffer, &combined[..combined_len]);

        if let Some(rx_last_dscr) = self.config.registers.rx_last_dscr {
            self.write_register(rx_last_dscr, val_val);
        }

        self.set_event(ctx, self.config.rx_event);
        true
    }

    pub fn set_event(&mut self, ctx: &mut CpuContext, cpu_val: u32) {
        self.event |= cpu_val;
        if self.enabled {
            ctx.interrupt(self.config.irq, true);
        }
    }

    pub fn read_dma_buffer(&mut self, cpu_val: u32) -> DmaBuffer {
        let desc = self.read_dma_descriptor(cpu_val);
        let sim_clock = desc.length;
        let mut reg_val = [0u8; 4096];
        let read_len = (sim_clock as usize).min(4096);
        for i in 0..read_len {
            reg_val[i] = (self.core_read_u8)(desc.buffer_ptr.wrapping_add(i as u32));
        }
        let next_ptr = if desc.eof { None } else { Some(desc.next_ptr) };
        DmaBuffer { buffer: reg_val, length: sim_clock, next_ptr }
    }

    pub fn write_dma_buffer(&mut self, cpu_val: u32, tmp_val: &[u8]) -> u32 {
        if cpu_val == 0 {
            return 0;
        }
        let desc = self.read_dma_descriptor(cpu_val);
        let simulation_clock = desc.next_ptr;
        if simulation_clock == 0 {
            return cpu_val;
        }
        let clock_event = desc.buffer_ptr;
        for i in 0..tmp_val.len() {
            (self.core_write_u8)(clock_event.wrapping_add(i as u32), tmp_val[i]);
        }
        let arg_val = 0x40000000;
        let register_type = (0x3f000fff & desc.header) | ((4095 & tmp_val.len() as u32) << 12) | arg_val;
        (self.core_write_u32)(cpu_val, register_type);
        simulation_clock
    }

    pub fn read_dma_descriptor(&self, cpu_val: u32) -> DmaDescriptor {
        let tmp_val = (self.core_read_u32)(cpu_val);
        let idx_val = tmp_val;
        DmaDescriptor {
            header: idx_val,
            size: 4095 & idx_val,
            length: (idx_val >> 12) & 4095,
            eof: (0x40000000 & idx_val) != 0,
            owner_dma: (0x80000000 & idx_val) != 0,
            buffer_ptr: (self.core_read_u32)(cpu_val.wrapping_add(4)),
            next_ptr: (self.core_read_u32)(cpu_val.wrapping_add(8)),
        }
    }

    /// ESP-NOW action-frame snoop on the TXDMA buffer. Layout here is the
    /// raw 802.11 MPDU at buf[0] (this config has tx_header=false; the
    /// shared strip below only removes the trailing 4-byte FCS).
    /// Management/action (type 0 / subtype 13) with category 0x7F and the
    /// Espressif OUI (18:FE:34) is an ESP-NOW frame: copy the MPDU to the
    /// scratch and report it to the host medium hook. Returns nothing;
    /// normal AP-port processing continues (it drops mgmt frames).
    pub fn snoop_espnow_action(&self, buf: &[u8]) {
        if buf.len() < 24 + 4 + 4 {
            return;
        }
        let fc = buf[0];
        if ((fc >> 2) & 3) != 0 || ((fc >> 4) & 15) != 13 {
            return;
        }
        if buf[24] != 0x7F || buf[25] != 0x18 || buf[26] != 0xFE || buf[27] != 0x34 {
            return;
        }
        // Strip the trailing 4-byte FCS, mirroring the shared strip below.
        let end = buf.len().saturating_sub(4);
        if end <= 24 {
            return;
        }
        let mpdu = &buf[..end];
        let take = mpdu.len().min(1600);
        unsafe {
            for i in 0..take {
                ESPNOW_TX_SCRATCH[i] = mpdu[i];
            }
            crate::peripherals::common::ffi::js_espnow_tx_frame(
                ESPNOW_TX_SCRATCH.as_ptr() as u32,
                take as u32,
            );
        }
    }

    pub fn set_mac_address(&self, cpu_val: &[u8; 6]) {        let idx_val = crc8(cpu_val);
        let clock_event = self.config.efuse_mac_addr;
        (self.core_write_u32)(clock_event, (cpu_val[2] as u32) << 24 | (cpu_val[3] as u32) << 16 | (cpu_val[4] as u32) << 8 | cpu_val[5] as u32);
        if self.config.efuse_mac_no_crc {
            let idx_val_2 = (self.core_read_u32)(clock_event.wrapping_add(4));
            (self.core_write_u32)(clock_event.wrapping_add(4), (0xffff0000 & idx_val_2) | (cpu_val[0] as u32) << 8 | cpu_val[1] as u32);
        } else {
            (self.core_write_u32)(clock_event.wrapping_add(4), (idx_val as u32) << 16 | (cpu_val[0] as u32) << 8 | cpu_val[1] as u32);
        }
    }

    pub fn on_tx_complete(&mut self, ctx: &mut CpuContext) {
        self.set_event(ctx, RSA_REG59);
        if let Some(cb) = self.on_tx_modem_status {
            cb(ctx);
        }
    }

    // JS sendFrame tail: after writing the RX DMA buffer, the JS peripheral
    // mirrors the RX event + RX_LAST_DSCR into the native register file so a
    // native-routed driver (WifiMac PTE) sees the same MAC_EVENT bits and the
    // same last-descriptor pointer it would read with the JS path.
    pub fn rx_complete(&mut self, last_dscr: u32) {
        self.event |= self.config.rx_event;
        if let Some(off) = self.config.registers.rx_last_dscr {
            self.write_register(off, last_dscr);
        }
    }
}

impl MmioPeripheral for WifiMacPeripheral {
    fn read_u32(&mut self, _ctx: &mut CpuContext, addr: u32) -> u32 {
        let offset = addr.wrapping_sub(self.base_addr);
        let tmp_val = &self.config.registers;
        match offset {
            _ if offset == tmp_val.rx_ctrl => {
                if self.rx_enabled { self.config.rx_enable_bit.unwrap_or(0x80000000) } else { 0 }
            }
            _ if offset == tmp_val.mac_event => self.event,
            _ if offset == tmp_val.rx_dscr_addr_base => self.dma_base,
            _ if offset == tmp_val.tx_fifo_complete => self.tx_complete_bits,
            _ if offset == tmp_val.mac_ctrl => 1 | self.read_register(tmp_val.mac_ctrl),
            _ if offset == tmp_val.fiq_status.unwrap_or(u32::MAX) => 0,
            _ if offset == tmp_val.tx_ack_status.unwrap_or(u32::MAX) => self.tx_ack_status,
            _ if offset == tmp_val.tx_fifo_complete_hal.unwrap_or(u32::MAX) => self.tx_complete_hal_bits,
            _ => self.read_register(offset),
        }
    }

    fn write_u32(&mut self, ctx: &mut CpuContext, addr: u32, tmp_val: u32) {
        let offset = addr.wrapping_sub(self.base_addr);
        let idx_val = &self.config.registers;
        let clock_event = self.config.rx_interface_en_bitmask;
        let simulation_clock = offset;
        if simulation_clock == idx_val.tx_fifo_clear {
            self.tx_complete_bits &= !(31 & tmp_val);
            return;
        }
        if simulation_clock == idx_val.tx_fifo_clear_hal.unwrap_or(u32::MAX) {
            self.tx_complete_hal_bits &= !(31 & tmp_val);
            return;
        }
        if simulation_clock == idx_val.tx_ack_clear.unwrap_or(u32::MAX) {
            self.tx_ack_status &= !tmp_val;
            return;
        }
        match simulation_clock {
            _ if simulation_clock == idx_val.mac_ctrl => {
                self.enabled = (2 & tmp_val) != 0;
            }
            _ if simulation_clock == idx_val.rx_ctrl => {
                self.rx_enabled = (tmp_val & self.config.rx_enable_bit.unwrap_or(0x80000000)) != 0;
            }
            _ if simulation_clock == idx_val.mac_rx_iface0 => {
                if tmp_val & clock_event != 0 { self.rx_interface = 0; }
            }
            _ if simulation_clock == idx_val.mac_rx_iface1 => {
                if tmp_val & clock_event != 0 { self.rx_interface = 1; }
            }
            _ if simulation_clock == idx_val.rx_dscr_base => {
                if tmp_val != 0xffffffff {
                    self.rx_buffer = self.dma_base | (1048575 & tmp_val);
                }
            }
            _ if simulation_clock == idx_val.mac_event_clear => {
                self.event &= !tmp_val;
                if self.event == 0 {
                    ctx.interrupt(self.config.irq, false);
                }
            }
            _ if simulation_clock == idx_val.dma_txbuf0
                || simulation_clock == idx_val.dma_txbuf1
                || simulation_clock == idx_val.dma_txbuf2
                || simulation_clock == idx_val.dma_txbuf3
                || simulation_clock == idx_val.dma_txbuf4 =>
            {
                if tmp_val & 0xc0000000 != 0 {
                    let cpu_val = idx_val.dma_txbuf0 - idx_val.dma_txbuf1;
                    if cpu_val != 8 && cpu_val != 16 {
                        panic!("Unexpected DMA_TXBUF delta: {}", cpu_val);
                    }
                    let clock_event_2 = if cpu_val == 16 { 4 } else { 3 };
                    let reg_val = (idx_val.dma_txbuf0 - simulation_clock) >> clock_event_2;
                    let arg_val = self.dma_base | (1048575 & tmp_val);
                    let dma_buf = self.read_dma_buffer(arg_val);
                    let buf = &dma_buf.buffer[..dma_buf.length as usize];
                    // ESP-NOW action frames (mgmt type 0 / subtype 13 /
                    // category 0x7F / OUI 18:FE:34) are NOT internet traffic:
                    // hand the raw 802.11 MPDU to the host medium hook
                    // (two-node delivery via the shared gateway room) instead
                    // of the AP-port data path below, which would drop them.
                    self.snoop_espnow_action(buf);
                    let h_val = if self.config.tx_header {
                        if buf.len() > 8 {
                            let cpu_len = buf[0] as usize + ((buf[1] as usize) << 8);
                            let end = (cpu_len + 8).min(buf.len());
                            &buf[8..end]
                        } else {
                            &[]
                        }
                    } else {
                        buf
                    };
                    let h_val = if h_val.len() > 4 {
                        &h_val[..h_val.len() - 4]
                    } else {
                        &[]
                    };
                    if let Some(cb) = self.on_tx {
                        cb(ctx, h_val);
                    }
                    self.tx_complete_bits |= 1 << reg_val;
                    self.tx_complete_hal_bits |= 1 << reg_val;
                    self.tx_ack_status |= 1 << (24 + reg_val);
                    unsafe { crate::peripherals::common::ffi::js_wifi_tx_complete(1000) };
                }
            }
            _ => {
                self.write_register(offset, tmp_val);
                return;
            }
        }
        self.write_register(offset, tmp_val);
    }

    fn reset(&mut self) {
        self.event = 0;
        self.rx_buffer = 0;
        self.rx_interface = 0;
        self.tx_complete_bits = 0;
        self.tx_complete_hal_bits = 0;
        self.tx_ack_status = 0;
        self.rx_enabled = false;
        self.enabled = false;
        self.channel = 0;
        self.registers = [0; REG_COUNT];
    }
}

pub fn apply_peripheral_reset_values(ctx: &mut CpuContext, tmp_val: &[u32]) {
    // JS: cpuVal.peripherals.forEach(p => p.zeroMemory()) — requires ctx.zero_all_peripheral_memory()
    // JS: writes reset values [baseAddr, [quad, ...], ...] to cpuVal.cores[0] — requires ctx.mem_write_u32()
    // TODO: implement when CpuContext gains peripheral/core memory access
}

pub fn apply_single_reset_values(cpu_val: &mut WifiMacPeripheral, tmp_val: &[u32]) {
    cpu_val.registers = [0; REG_COUNT];
    // JS format: [baseAddr, [quad, ...], baseAddr, [quad, ...], ...]
    // RS flat encoding: [baseAddr, quadCount, offset0, val0, cnt0, stride0, ...]
    let mut i = 0;
    while i + 1 < tmp_val.len() {
        let base = tmp_val[i];
        let quad_count = tmp_val[i + 1] as usize;
        i += 2;
        if base == cpu_val.base_addr {
            for _q in 0..quad_count {
                if i + 3 >= tmp_val.len() { break; }
                let offset = tmp_val[i];
                let val = tmp_val[i + 1];
                let count = tmp_val[i + 2];
                let stride = tmp_val[i + 3];
                i += 4;
                for j in 0..count {
                    cpu_val.write_register(offset + j * stride, val);
                }
            }
        } else {
            i += quad_count * 4;
        }
    }
}
