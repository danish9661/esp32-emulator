// Translated from: src/peripherals/common/spi-syscon.js
// Do NOT modify the logic — match the JS line-for-line

use crate::peripherals::types::*;
use crate::peripherals::common::peripheral::{PeripheralBase, FieldDesc};
use crate::peripherals::common::clocks::DerivedClock;
use core::cmp;

// ============================================================
// Constants from sha-ecc-key.js (eccReg21–eccReg59, dsReg1–dsReg14)
// ============================================================

pub const ECC_REG21: u32 = 0;
pub const ECC_REG22: u32 = 16448;
pub const ECC_REG23: u32 = 32896;
pub const ECC_REG26: u32 = 12_000_000;
pub const ECC_REG27: u32 = 16_000_000;
pub const ECC_REG29: u32 = 0;
pub const ECC_REG30: u32 = 4;
pub const ECC_REG31: u32 = 8;
pub const ECC_REG32: u32 = 368;
pub const ECC_REG33: u32 = 0x8000_0000;
pub const ECC_REG34: u32 = 376;
pub const ECC_REG35: u32 = 892;
pub const ECC_REG36: u32 = 896;
pub const ECC_REG37: u32 = 900;
pub const ECC_REG38: u32 = 3;
pub const ECC_REG39: u32 = 3;
pub const ECC_REG40: u32 = 768;
pub const ECC_REG41: u32 = 832;
pub const ECC_REG42: u32 = 836;
pub const ECC_REG43: u32 = 840;
pub const ECC_REG44: u32 = 844;
pub const ECC_REG45: u32 = 848;
pub const ECC_REG46: u32 = 852;
pub const ECC_REG47: u32 = 856;
pub const ECC_REG48: u32 = 860;
pub const ECC_REG49: u32 = 0x8000_0000;
pub const ECC_REG50: u32 = 32768;
pub const ECC_REG51: u32 = 0x0800_0000;
pub const ECC_REG52: u32 = 0x1000_0000;
pub const ECC_REG53: u32 = 0x2000_0000;
pub const ECC_REG54: u32 = 0x4000_0000;
pub const ECC_REG55: u32 = 0x8000_0000;
pub const ECC_REG56: u32 = 1;
pub const ECC_REG57: u32 = 0x2000_0000;
pub const ECC_REG58: u32 = 0x2000_0000;
pub const ECC_REG59: u32 = 16;

pub const DS_REG1: u32 = 512;
pub const DS_REG2: u32 = 1024;
pub const DS_REG3: u32 = 2048;
pub const DS_REG4: u32 = 0x0100_0000;
pub const DS_REG5: u32 = 0x0200_0000;
pub const DS_REG6: u32 = 0x0800_0000;
pub const DS_REG7: u32 = 0x1000_0000;
pub const DS_REG8: u32 = 0x2000_0000;
pub const DS_REG9: u32 = 0x4000_0000;
pub const DS_REG10: u32 = 0x8000_0000;
pub const DS_REG11: u32 = 1;
pub const DS_REG12: u32 = 2;

// Lookup tables matching JS eccReg24, eccReg28, eccReg25
const FLASH_ID_TABLE: &[(u32, u32)] = &[
    (2, 1392879), (4, 1458376), (8, 1523951), (16, 1589448), (32, 1655023),
];
const DEFAULT_FLASH_ID: u32 = 1458376;

const PSRAM_ID_TABLE: &[(u32, u32)] = &[
    (2, 23821), (4, 2186509), (8, 4218125), (16, 9657613),
];
const DEFAULT_PSRAM_ID: u32 = 0;

const PSRAM_JEDEC_TABLE: &[(u32, u32)] = &[
    (4, 1), (8, 3), (16, 5), (32, 7),
];
const DEFAULT_PSRAM_JEDEC: u32 = 1;

// ============================================================
// Constants from spi-syscon.js exported section
// ============================================================

pub const DS_REG15: u32 = 0;
pub const DS_REG16: u32 = 4;
pub const DS_REG17: u32 = 8;
pub const DS_REG18: u32 = 12;
pub const DS_REG19: u32 = 60;
pub const DS_REG20: u32 = 124;
pub const DS_REG21: u32 = 1023;
pub const DS_REG22: u32 = 0;
pub const SYSCON_TICK_COUNT_MASK: u32 = 255;

// JS: let genVal1 = false
static mut GEN_VAL1: bool = false;

// ============================================================
// Helper functions matching bit-helpers.js and helpers.js
// ============================================================

fn byte_swap32(val: u32) -> u32 {
    ((255 & val) << 24) | ((0xFF00 & val) << 8)
        | ((val >> 8) & 0xFF00) | ((val >> 24) & 255)
}

fn mask_low_bits(val: u32, bits: u32) -> u32 {
    if bits >= 32 { val } else { val & ((1u32 << bits) - 1) }
}

fn read_field_value(val: u32, field: &FieldDesc) -> u32 {
    (val >> field.shift) & field.mask
}

fn table_lookup(table: &[(u32, u32)], key: u32, default: u32) -> u32 {
    for &(k, v) in table { if k == key { return v; } }
    default
}

// ============================================================
// XtsState enum — from enums.js
// ============================================================

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum XtsState {
    Idle = 0,
    Busy = 1,
    Done = 2,
    Visible = 3,
}

// ============================================================
// XtsEncryptionState — JS class XtsEncryptionState
// ============================================================

pub struct XtsEncryptionState {
    pub state: XtsState,
    pub line_size: u32,
    pub destination: u32,
    pub physical_address: u32,
}

impl XtsEncryptionState {
    pub fn new() -> Self {
        XtsEncryptionState { state: XtsState::Idle, line_size: 0, destination: 0, physical_address: 0 }
    }
    pub fn reset(&mut self) {
        self.state = XtsState::Idle;
        self.line_size = 0;
        self.destination = 0;
        self.physical_address = 0;
    }
}

// ============================================================
// Stub: DmaDescriptorChain — from i2c-i2s.js
// ============================================================

pub struct DmaDescriptorChain {
    pub active: bool,
    pub index: u32,
    pub pointer: u32,
}

impl DmaDescriptorChain {
    pub fn new() -> Self { DmaDescriptorChain { active: false, index: 0, pointer: 0 } }
    pub fn start(&mut self, _link_addr: u32) { self.active = true; self.index = 0; }
    pub fn read_uint32(&mut self) -> u32 { 0xFFFFFFFF }
    pub fn copy_out_bytes(&mut self, _data: &mut [u8]) {}
    pub fn copy_in_bytes(&mut self, _data: &[u8]) {}
    pub fn reset(&mut self) { self.active = false; self.index = 0; }
}

// ============================================================
// Stub: DmaReader — from sha-ecc-key.js
// ============================================================

pub struct DmaReader {
    pub active: bool,
    pub index: u32,
}

impl DmaReader {
    pub fn new() -> Self { DmaReader { active: false, index: 0 } }
    pub fn start(&mut self, _source: &mut DmaDescriptorChain, _channel: Option<u32>) {
        self.active = true; self.index = 0;
    }
    pub fn start_slice(&mut self, _slice: &[u8]) { self.active = true; self.index = 0; }
    pub fn reset(&mut self) { self.active = false; self.index = 0; }
}

// ============================================================
// Stub: DmaWriter — from sha-ecc-key.js
// ============================================================

pub struct DmaWriter {
    pub active: bool,
    pub index: u32,
    pub length: u32,
}

impl DmaWriter {
    pub fn new() -> Self { DmaWriter { active: false, index: 0, length: 0 } }
    pub fn start(&mut self, _target: &mut DmaDescriptorChain, _channel: Option<u32>) {
        self.active = true; self.index = 0; self.length = 0;
    }
    pub fn start_slice(&mut self, _slice: &[u8]) { self.active = true; self.index = 0; self.length = 0; }
    pub fn reset(&mut self) { self.active = false; self.index = 0; self.length = 0; }
}

// ============================================================
// Stub: BitWriter — from sha-ecc-key.js
// ============================================================

pub struct BitWriter {
    pub buffer: [u8; 128],
    pub msb_first: bool,
    pub index: u32,
}

impl BitWriter {
    pub fn new(msb_first: bool) -> Self {
        BitWriter { buffer: [0u8; 128], msb_first, index: 0 }
    }
    pub fn write_value(&mut self, val: u32, bits: u32) {
        let mut idx = self.index;
        let mut remaining = bits;
        let mut v = val;
        if self.msb_first {
            let mut byte_val = (255 & v) as u8;
            let mut shift_count = 0u32;
            while remaining > 0 {
                let bit = 0x80 & byte_val;
                if (bit >> (7 & idx)) != 0 {
                    let byte_idx = (idx >> 3) as usize;
                    if byte_idx < 128 { self.buffer[byte_idx] |= bit >> (7 & idx); }
                }
                byte_val <<= 1; idx += 1; shift_count += 1;
                if shift_count == 7 { shift_count = 0; v >>= 8; byte_val = (255 & v) as u8; }
                remaining -= 1;
            }
        } else {
            while remaining > 0 {
                let bit = 1 & v;
                if bit != 0 {
                    let byte_idx = (idx >> 3) as usize;
                    if byte_idx < 128 { self.buffer[byte_idx] |= (bit << (7 - (7 & idx))) as u8; }
                }
                v >>= 1; idx += 1; remaining -= 1;
            }
        }
        self.index = idx;
    }
    pub fn write_uint8(&mut self, val: u8) {
        if self.index % 8 == 0 {
            let byte_idx = (self.index >> 3) as usize;
            if byte_idx < 128 { self.buffer[byte_idx] = val; }
            self.index += 8;
        } else {
            self.write_value(val as u32, 8);
        }
    }
    pub fn byte_index(&self) -> u32 { self.index >> 3 }
}

// ============================================================
// Stub: XtsAes — from sha-ecc-key.js
// ============================================================

pub struct XtsAes;

impl XtsAes {
    pub fn new(_key: &[u8]) -> Self { XtsAes }
}

// ============================================================
// SpiRegOffsets — holds register offset values (JS: RmtChannelRegister)
// ============================================================

#[derive(Clone, Copy)]
pub struct SpiRegOffsets {
    pub cmd: i32,
    pub addr: i32,
    pub slv_wr_status: i32,
    pub ctrl: i32,
    pub ctrl1: i32,
    pub ctrl2: i32,
    pub clock: i32,
    pub clk_gate: i32,
    pub user: i32,
    pub user1: i32,
    pub user2: i32,
    pub mosi_dlen: i32,
    pub miso_dlen: i32,
    pub ms_dlen: i32,
    pub rd_status: i32,
    pub din_mode: i32,
    pub din_num: i32,
    pub dout_mode: i32,
    pub pin: i32,
    pub misc: i32,
    pub slave: i32,
    pub w0: i32,
    pub dma_conf: i32,
    pub dma_in_link: i32,
    pub dma_out_link: i32,
    pub dma_int_ena: i32,
    pub dma_int_clr: i32,
    pub dma_int_raw: i32,
    pub dma_int_st: i32,
}

impl SpiRegOffsets {
    pub const fn default_esp32() -> Self {
        SpiRegOffsets {
            cmd: 0, addr: 4, slv_wr_status: 80,
            ctrl: 8, ctrl1: 12, ctrl2: 16,
            clock: 0x18, clk_gate: 0x1C,
            user: 0x20, user1: 0x24, user2: 0x28,
            mosi_dlen: 0x2C, miso_dlen: 0x30, ms_dlen: -1,
            rd_status: 0x34,
            din_mode: -1, din_num: -1, dout_mode: -1,
            pin: 0x38, misc: -1,
            slave: 0x40,
            w0: 0x80,
            dma_conf: 0x100,
            dma_in_link: 0x104, dma_out_link: 0x108,
            dma_int_ena: 0x10C, dma_int_clr: 0x110,
            dma_int_raw: 0x114, dma_int_st: 0x118,
        }
    }
}

// ============================================================
// SpiFieldDescs — holds field descriptor references (JS: F)
// ============================================================

#[derive(Clone, Copy)]
pub struct SpiFieldDescs {
    pub usr: Option<FieldDesc>,
    pub wr_bit_order: Option<FieldDesc>,
    pub mode: Option<FieldDesc>,
    pub doutdin: Option<FieldDesc>,
    pub usr_dummy_cyclelen: Option<FieldDesc>,
    pub usr_addr_bitlen: Option<FieldDesc>,
    pub usr_command_value: Option<FieldDesc>,
    pub usr_command_bitlen: Option<FieldDesc>,
    pub usr_conf: Option<FieldDesc>,
    pub dma_seg_magic_value: Option<FieldDesc>,
    pub clkcnt_n: Option<FieldDesc>,
    pub clkdiv_pre: Option<FieldDesc>,
    pub mst_clk_sel: Option<FieldDesc>,
    pub trans_done_int_raw: Option<FieldDesc>,
    pub dma_seg_trans_done: Option<FieldDesc>,
    pub dma_seg_trans_done_int_raw: Option<FieldDesc>,
    pub seg_magic_err: Option<FieldDesc>,
    pub seg_magic_err_int_raw: Option<FieldDesc>,
    pub slv_data_bytelen: Option<FieldDesc>,
    pub slv_data_bitlen: Option<FieldDesc>,
}

impl SpiFieldDescs {
    pub const fn new() -> Self {
        SpiFieldDescs {
            usr: None, wr_bit_order: None, mode: None,
            doutdin: None, usr_dummy_cyclelen: None,
            usr_addr_bitlen: None, usr_command_value: None,
            usr_command_bitlen: None, usr_conf: None,
            dma_seg_magic_value: None, clkcnt_n: None,
            clkdiv_pre: None, mst_clk_sel: None,
            trans_done_int_raw: None, dma_seg_trans_done: None,
            dma_seg_trans_done_int_raw: None, seg_magic_err: None,
            seg_magic_err_int_raw: None, slv_data_bytelen: None,
            slv_data_bitlen: None,
        }
    }
}

// ============================================================
// SpiConfig — JS this.config { RmtChannelRegister, F, mmuTable, irq, ... }
// ============================================================

pub struct SpiConfig {
    pub regs: SpiRegOffsets,
    pub fields: SpiFieldDescs,
    pub mmu_table: Option<&'static mut [u32]>,
    pub irq: u32,
    pub cs_bits: u32,
    pub flash: bool,
    pub oct: bool,
    pub psram_size: u32,
    pub clock_source: Option<u32>,
    pub addr_left_aligned: bool,
    pub pms_addr_split: bool,
    pub mmu_page_size_updated: Option<fn(u32)>,
}

impl SpiConfig {
    pub fn new() -> Self {
        SpiConfig {
            regs: SpiRegOffsets::default_esp32(),
            fields: SpiFieldDescs::new(),
            mmu_table: None, irq: 0, cs_bits: 1,
            flash: false, oct: false, psram_size: 0,
            clock_source: None, addr_left_aligned: false,
            pms_addr_split: false,
            mmu_page_size_updated: None,
        }
    }
}

// ============================================================
// SpiSignals — JS this.signals { cs: [], clk, mosi, miso }
// ============================================================

pub struct SpiSignals {
    pub cs: [u32; 6],
    pub cs_count: u32,
    pub clk: u32,
    pub mosi: u32,
    pub miso: u32,
}

impl SpiSignals {
    pub const fn new() -> Self {
        SpiSignals { cs: [0; 6], cs_count: 0, clk: 0, mosi: 0, miso: 0 }
    }
}

// ============================================================
// SpiPeripheral — JS class SpiPeripheral extends PeripheralBase
// ============================================================

pub struct SpiPeripheral {
    pub base: PeripheralBase,
    pub config: SpiConfig,
    pub status_reg: u32,
    pub psram_selected: bool,
    pub dma_in: DmaDescriptorChain,
    pub dma_out: DmaDescriptorChain,
    pub transaction_active: bool,
    pub cmd_in_progress: u32,
    pub dma_in_link: Option<u32>,
    pub dma_out_link: Option<u32>,
    pub slave_active: bool,
    pub internal_clock: DerivedClock,
    pub slave_reader: DmaReader,
    pub slave_writer: DmaWriter,
    pub signals: SpiSignals,
    pub flash_id: u32,
    pub psram_id: u32,
    pub octal_mode: bool,
    pub transaction_buffer: Option<[u8; 128]>,
    pub transaction_buffer_len: u32,
    pub dma_in_channel: Option<u32>,
    pub dma_out_channel: Option<u32>,
    pub xts: XtsEncryptionState,
    pub flash_buffer: *mut u8,
    pub flash_buffer_len: u32,
    pub idx: u32,
}

impl SpiPeripheral {
    pub fn new(
        base_addr: u32, name: &'static str, config: SpiConfig,
        flash_buffer: *mut u8, flash_buffer_len: u32, idx_val: u32,
    ) -> Self {
        let flash_size_mb = flash_buffer_len >> 20;
        let flash_id = table_lookup(FLASH_ID_TABLE, flash_size_mb, DEFAULT_FLASH_ID);
        let psram_id = table_lookup(PSRAM_ID_TABLE, config.psram_size, DEFAULT_PSRAM_ID);
        let oct_mode = config.oct || 16 == config.psram_size;

        // JS: this.internalClock = new DerivedClock(this.cpu.clocks.apb, 1)
        let internal_clock = DerivedClock::new(10, 80_000_000, true, 1.0);

        SpiPeripheral {
            base: PeripheralBase::new(base_addr, name),
            config,
            status_reg: 0,
            psram_selected: false,
            dma_in: DmaDescriptorChain::new(),
            dma_out: DmaDescriptorChain::new(),
            transaction_active: false,
            cmd_in_progress: 0,
            dma_in_link: None,
            dma_out_link: None,
            slave_active: false,
            internal_clock,
            slave_reader: DmaReader::new(),
            slave_writer: DmaWriter::new(),
            signals: SpiSignals::new(),
            flash_id,
            psram_id,
            octal_mode: oct_mode,
            transaction_buffer: None,
            transaction_buffer_len: 0,
            dma_in_channel: None,
            dma_out_channel: None,
            xts: XtsEncryptionState::new(),
            flash_buffer,
            flash_buffer_len,
            idx: idx_val,
        }
    }

    /// Host PSRAM size in MB (from the board config). Recomputes the JEDEC
    /// ID served by the PSRAM ID-read path so firmware PSRAM detection
    /// (`psramFound()`) sees the configured chip; size 0 disables it.
    pub fn set_psram_size(&mut self, mb: u32) {
        self.config.psram_size = mb;
        self.psram_id = table_lookup(PSRAM_ID_TABLE, mb, DEFAULT_PSRAM_ID);
        self.octal_mode = self.config.oct || mb == 16;
    }

    // JS get clockNanos()
    pub fn clock_nanos(&self) -> u64 {
        let clock_val = self.read_register(self.config.regs.clock as u32);
        let freq = self.config.clock_source.map_or_else(
            || self.internal_clock.frequency(),
            |_| 80_000_000,
        );
        let base_nanos = if freq != 0 { 1_000_000_000u64 / freq as u64 } else { 0 };
        if clock_val & ECC_REG49 != 0 {
            return base_nanos;
        }
        let clkdiv_pre = self.config.fields.clkdiv_pre
            .map_or(0, |f| read_field_value(clock_val, &f));
        let clkcnt_n = self.config.fields.clkcnt_n
            .map_or(0, |f| read_field_value(clock_val, &f));
        base_nanos * ((clkdiv_pre + 1) as u64) * ((clkcnt_n + 1) as u64)
    }

    // JS get csPins()
    pub fn cs_pins(&self) -> u32 { self.config.cs_bits }

    // JS get activeCS()
    pub fn active_cs(&self) -> u32 {
        let misc_or_pin = if self.config.regs.misc >= 0 {
            self.config.regs.misc
        } else {
            self.config.regs.pin
        };
        if misc_or_pin < 0 { return 0; }
        let pin_val = self.read_register(misc_or_pin as u32);
        let mut active = 0u32;
        for i in 0..self.config.cs_bits {
            if pin_val & (ECC_REG56 << i) == 0 {
                active |= 1 << i;
            }
        }
        active
    }

    // JS get slaveMode()
    pub fn slave_mode(&self) -> bool {
        self.config.fields.mode.map_or(false, |f| self.read_field(&f) != 0)
    }

    // JS readUint32(cpuVal)
    pub fn mmio_read_u32(&mut self, _ctx: &mut CpuContext, addr: u32) -> u32 {
        let offset = addr.wrapping_sub(self.base.base_addr);
        let offset_signed = offset as i32;

        match offset_signed {
            x if x == ECC_REG29 as i32 => {
                if self.cmd_in_progress != 0 { return self.cmd_in_progress; }
                let usr_shift = self.config.fields.usr.map_or(0, |f| f.shift);
                if self.transaction_active { return 1 << usr_shift; }
                0
            }
            x if x == self.config.regs.rd_status => self.status_reg,
            x if x == self.config.regs.dma_int_st => {
                self.read_register(self.config.regs.dma_int_raw as u32)
                    & self.read_register(self.config.regs.dma_int_ena as u32)
            }
            x if x == ECC_REG32 as i32 || x == ECC_REG34 as i32 => ECC_REG33,
            x if x == ECC_REG35 as i32 => {
                if let Some(table) = &self.config.mmu_table {
                    let idx = self.read_register(ECC_REG36) as usize;
                    if idx < table.len() { return table[idx]; }
                }
                0
            }
            x if x == ECC_REG41 as i32 => self.xts.line_size,
            x if x == ECC_REG42 as i32 => self.xts.destination,
            x if x == ECC_REG43 as i32 => self.xts.physical_address,
            x if x == ECC_REG47 as i32 => self.xts.state as u32,
            x if x == ECC_REG48 as i32 => 0x20201010,
            _ => self.base.read_uint32(addr),
        }
    }

    // JS writeUint32(cpuVal, tmpVal)
    pub fn mmio_write_u32(&mut self, ctx: &mut CpuContext, addr: u32, val: u32) {
        self.base.write_uint32(addr, val);
        let offset = addr.wrapping_sub(self.base.base_addr);
        let offset_signed = offset as i32;

        match offset_signed {
            x if x == ECC_REG29 as i32 => {
                if self.config.fields.usr.map_or(false, |f| read_field_value(val, &f) != 0) {
                    self.start_transaction(ctx);
                } else if self.config.flash {
                    self.flash_cmd_write(val);
                } else if val & DS_REG8 != 0 {
                    self.cmd_in_progress = val;
                    self.complete_transmit(ctx, None);
                } else if val & DS_REG9 != 0 {
                    self.cmd_in_progress = val;
                    self.complete_transmit(ctx, None);
                }
            }
            x if x == self.config.regs.pin || x == self.config.regs.misc => {
                self.psram_selected = (val & ECC_REG56) != 0;
                self.base.write_uint32(addr, val);
            }
            x if x == self.config.regs.clk_gate => {
                if self.config.fields.mst_clk_sel.is_some() {
                    let sel = self.config.fields.mst_clk_sel
                        .map_or(0, |f| read_field_value(val, &f));
                    if sel != 0 {
                        self.internal_clock.set_parent_idx(10);
                    } else {
                        self.internal_clock.set_parent_idx(4);
                    }
                }
                self.base.write_uint32(addr, val);
            }
            x if x == self.config.regs.slave || x == self.config.regs.dma_int_ena => {
                self.base.write_uint32(addr, val);
                self.update_interrupt(ctx);
            }
            x if x == self.config.regs.dma_int_clr => {
                self.clear_register_bits(self.config.regs.dma_int_raw as u32, val);
                self.update_interrupt(ctx);
            }
            x if x == self.config.regs.dma_int_raw => {
                self.base.write_uint32(addr, val);
                self.update_interrupt(ctx);
            }
            x if x == ECC_REG35 as i32 => {
                let idx = self.read_register(ECC_REG36) as usize;
                if let Some(table) = self.config.mmu_table.as_mut() {
                    if idx < table.len() {
                        table[idx] = val;
                    }
                }
            }
            x if x == ECC_REG37 as i32 => {
                if let Some(cb) = self.config.mmu_page_size_updated {
                    cb((val >> ECC_REG38) & ECC_REG39);
                }
                self.base.write_uint32(addr, val);
            }
            x if x == ECC_REG41 as i32 => { self.xts.line_size = 3 & val; }
            x if x == ECC_REG42 as i32 => { self.xts.destination = 1 & val; }
            x if x == ECC_REG43 as i32 => { self.xts.physical_address = 0x3FF_FFFF & val; }
            x if x == ECC_REG44 as i32 => {
                if self.xts.state == XtsState::Idle && (1 & val) != 0 {
                    self.xts.state = XtsState::Done;
                }
            }
            x if x == ECC_REG45 as i32 => {
                if self.xts.state == XtsState::Done && (1 & val) != 0 {
                    self.perform_xts_encryption_and_write();
                    self.xts.state = XtsState::Visible;
                }
            }
            x if x == ECC_REG46 as i32 => {
                if (1 & val) != 0 { self.xts.state = XtsState::Idle; }
            }
            _ => { self.base.write_uint32(addr, val); }
        }
    }

    // ================================================================
    // Internal methods (JS prototype methods) — all take ctx where needed
    // ================================================================

    // Flash byte accessors. When no native flash buffer is attached (null),
    // bytes go through the JS chip.flash SAB via FFI (js_spi_flash_get/set_byte).
    // Mirrors JS `this.cpu.flash` reads/writes (OOB reads → 0, OOB writes dropped).
    fn flash_set_byte(&mut self, idx: usize, val: u8) {
        unsafe {
            if self.flash_buffer != core::ptr::null_mut() {
                if idx < self.flash_buffer_len as usize {
                    *self.flash_buffer.add(idx) = val;
                }
            } else {
                crate::peripherals::common::ffi::js_spi_flash_set_byte(idx as u32, val as u32);
            }
        }
    }

    fn flash_get_byte(&self, idx: usize) -> u8 {
        unsafe {
            if self.flash_buffer != core::ptr::null_mut() {
                if idx < self.flash_buffer_len as usize {
                    return *self.flash_buffer.add(idx);
                }
                return 0;
            }
            crate::peripherals::common::ffi::js_spi_flash_get_byte(idx as u32) as u8
        }
    }

    // JS flashCmdWrite(cpuVal) — no ctx needed (doesn't call interrupt or schedule)
    fn flash_cmd_write(&mut self, cmd: u32) {
        if cmd & DS_REG9 != 0 {
            self.status_reg |= DS_REG12;
        } else if cmd & DS_REG8 != 0 {
            self.status_reg &= !DS_REG12;
        } else if cmd & DS_REG4 != 0 {
            let addr = 0xFFF000 & self.read_register(ECC_REG30);
            let start = addr as usize;
            let end = (addr + 4096) as usize;
            for i in start..end { self.flash_set_byte(i, 255); }
        } else if cmd & DS_REG5 != 0 {
            let addr = 0xFFFFFF & self.read_register(ECC_REG30);
            let count = cmp::min(self.read_register(ECC_REG30) >> 24, 64);
            if self.xts.state == XtsState::Visible && addr == self.xts.physical_address {
                return;
            }
            let w0 = self.config.regs.w0 as u32;
            for i in 0..(count as usize) {
                let reg_idx = w0 + ((i / 4) * 4) as u32;
                let word = self.read_register(reg_idx);
                let byte = ((word >> ((i % 4) * 8)) & 0xFF) as u8;
                let dest = (addr as usize) + i;
                self.flash_set_byte(dest, byte);
            }
        } else if cmd & DS_REG7 != 0 {
            self.write_register(self.config.regs.w0 as u32, self.flash_id);
        } else if cmd & DS_REG9 != 0 {
            self.status_reg |= DS_REG12;
        } else if cmd & DS_REG8 != 0 {
            self.status_reg &= !DS_REG12;
        } else if cmd & DS_REG6 != 0 {
        } else if cmd & DS_REG10 != 0 {
            let addr = 0xFFFFFF & self.read_register(ECC_REG30);
            let count = self.read_register(ECC_REG30) >> 24;
            let w0 = self.config.regs.w0 as u32;
            for i in 0..(count as usize) {
                let src = (addr as usize) + i;
                let byte = self.flash_get_byte(src);
                let reg_idx = w0 + ((i / 4) * 4) as u32;
                let existing = self.read_register(reg_idx);
                let shift = (i % 4) * 8;
                let new_word = (existing & !(0xFFu32 << shift)) | ((byte as u32) << shift);
                self.write_register(reg_idx, new_word);
            }
        }
    }

    // JS performXtsEncryptionAndWrite() — no ctx needed
    fn perform_xts_encryption_and_write(&mut self) {
        let tmp_val = 16 << (3 & self.xts.line_size);
        for i in 0..(tmp_val / 4) {
            let src = self.read_register(ECC_REG40 + i * 4);
            let dest = self.xts.physical_address as usize + (i as usize) * 4;
            self.flash_set_byte(dest, (src & 0xFF) as u8);
            self.flash_set_byte(dest + 1, ((src >> 8) & 0xFF) as u8);
            self.flash_set_byte(dest + 2, ((src >> 16) & 0xFF) as u8);
            self.flash_set_byte(dest + 3, ((src >> 24) & 0xFF) as u8);
        }
    }

    // JS readSCTConf(cpuVal)
    fn read_sct_conf(&mut self, ctx: &mut CpuContext, addr: u32) -> bool {
        let magic_field = self.config.fields.dma_seg_magic_value;
        if magic_field.is_none() {
            return false;
        }
        let magic_field = magic_field.unwrap();
        self.dma_out.start(addr);
        let arg_val = self.dma_out.read_uint32();
        if arg_val >> 28 != self.read_field(&magic_field) {
            if let Some(f) = self.config.fields.seg_magic_err_int_raw {
                self.write_field(&f, 1);
            }
            if let Some(f) = self.config.fields.dma_seg_trans_done_int_raw {
                self.write_field(&f, 1);
            }
            self.update_interrupt(ctx);
            return false;
        }
        true
    }

    // JS startTransaction()
    fn start_transaction(&mut self, ctx: &mut CpuContext) {
        let wr_bit_order = self.config.fields.wr_bit_order.map_or(false, |f| self.read_field(&f) != 0);
        let doutdin = self.config.fields.doutdin.map_or(true, |f| self.read_field(&f) != 0);

        if self.slave_mode() {
            self.begin_slave_transaction(ctx);
            return;
        }

        let dma_out_link_val = if self.config.regs.dma_out_link >= 0 {
            self.read_register(self.config.regs.dma_out_link as u32)
        } else { 0 };
        let dma_out_addr = if dma_out_link_val & ECC_REG57 != 0 {
            dma_out_link_val
        } else if let Some(link) = self.dma_out_link {
            link
        } else { 0 };

        let usr_conf = self.config.fields.usr_conf.map_or(false, |f| self.read_field(&f) != 0);
        let mut reg_val = false;
        if usr_conf && dma_out_addr != 0 {
            if !self.read_sct_conf(ctx, dma_out_addr) { return; }
            reg_val = true;
        }

        let user_val = self.read_register(self.config.regs.user as u32);
        let cmd_bits = if user_val & ECC_REG55 != 0 {
            self.config.fields.usr_command_bitlen.map_or(0, |f| self.read_field(&f) + 1)
        } else { 0 };
        let cmd_val = self.config.fields.usr_command_value.map_or(0, |f| self.read_field(&f));
        let addr_bits = if user_val & ECC_REG54 != 0 {
            self.config.fields.usr_addr_bitlen.map_or(0, |f| self.read_field(&f) + 1)
        } else { 0 };
        let addr_val = self.read_register(ECC_REG30);
        let mosi_len = self.read_register(
            if self.config.regs.mosi_dlen >= 0 { self.config.regs.mosi_dlen as u32 } else { self.config.regs.ms_dlen as u32 }
        );
        let miso_len = self.read_register(
            if self.config.regs.miso_dlen >= 0 { self.config.regs.miso_dlen as u32 } else { self.config.regs.ms_dlen as u32 }
        );
        let send_bytes = if user_val & ECC_REG51 != 0 { (mosi_len + 1) >> 3 } else { 0 };
        let recv_bytes = if user_val & ECC_REG52 != 0 { (miso_len + 1) >> 3 } else { 0 };

        if self.config.flash {
            let tmp_addr = if self.config.addr_left_aligned {
                addr_val >> (32 - (-8i32 as u32 & addr_bits))
            } else {
                mask_low_bits(addr_val, addr_bits)
            };
            if self.psram_selected {
                if self.octal_mode {
                    self.psram_octal_command(cmd_val, tmp_addr, send_bytes, recv_bytes);
                } else {
                    self.psram_command(cmd_val, tmp_addr);
                }
            } else {
                self.flash_command(ctx, cmd_val, tmp_addr, send_bytes, recv_bytes);
            }
            if self.config.regs.slave >= 0 {
                self.set_register_bits(self.config.regs.slave as u32, ECC_REG59);
            }
        } else {
            self.transaction_active = true;
            let dummy_bits = if user_val & ECC_REG53 != 0 {
                self.config.fields.usr_dummy_cyclelen.map_or(0, |f| self.read_field(&f) + 1)
            } else { 0 };
            let total_bits = cmd_bits + addr_bits + dummy_bits;
            let _data_len_size = (total_bits >> 3)
                + if doutdin { cmp::max(send_bytes, recv_bytes) } else { send_bytes + recv_bytes }
                + if total_bits % 8 != 0 { 1 } else { 0 };
            let mut writer = BitWriter::new(!wr_bit_order);

            if cmd_bits != 0 { writer.write_value(cmd_val, cmd_bits); }
            if addr_bits != 0 {
                writer.write_value(byte_swap32(addr_val), cmp::min(addr_bits, 32));
                if addr_bits > 32 && self.config.regs.slv_wr_status >= 0 {
                    writer.write_value(
                        byte_swap32(self.read_register(self.config.regs.slv_wr_status as u32)),
                        addr_bits - 32,
                    );
                }
            }
            if dummy_bits != 0 { writer.write_value(0, dummy_bits); }

            let r_val = writer.byte_index();
            if send_bytes != 0 {
                if dma_out_addr != 0 {
                    if !reg_val {
                        self.dma_out.start(dma_out_addr);
                    }
                    self.dma_out.copy_out_bytes(&mut writer.buffer);
                } else {
                    let w0 = self.config.regs.w0 as u32;
                    for i in 0..send_bytes {
                        let reg_idx = w0 + (i / 4) * 4;
                        let word = self.read_register(reg_idx);
                        let byte = ((word >> ((i % 4) * 8)) & 0xFF) as u8;
                        writer.write_uint8(byte);
                    }
                }
            }

            let x_val = if doutdin { r_val } else { r_val + send_bytes };
            let mut buf = [0u8; 128];
            let start = cmp::min(x_val as usize, 128);
            let end = cmp::min(start + recv_bytes as usize, 128);
            for i in start..end {
                buf[i] = writer.buffer[i];
            }
            self.transaction_buffer = Some(buf);
            self.transaction_buffer_len = if recv_bytes != 0 { recv_bytes } else { 0 };
            // Virtual SPI bus: if a sibling controller is slave-enabled,
            // exchange data-phase bytes with it (MOSI -> slave RX, slave TX
            // -> our MISO). CPU (W-reg) mode only; DMA keeps loopback.
            let slave_miso = if dma_out_addr == 0 && (send_bytes != 0 || recv_bytes != 0) {
                let mut mosi = [0u8; 128];
                let n = cmp::min(send_bytes as usize, 128);
                let w0 = self.config.regs.w0 as u32;
                for i in 0..n {
                    let word = self.read_register(w0 + ((i / 4) * 4) as u32);
                    mosi[i] = ((word >> ((i % 4) * 8)) & 0xFF) as u8;
                }
                crate::native_mmio::spi_bus_exchange(self.idx as usize, &mosi[..n], recv_bytes, ctx)
            } else {
                None
            };
            self.complete_transmit(ctx, slave_miso);
        }
    }

    // JS completeTransmit()
    fn complete_transmit(&mut self, ctx: &mut CpuContext, slave_miso: Option<([u8; 128], u32)>) {
        if self.cmd_in_progress != 0 {
            self.cmd_in_progress = 0;
            return;
        }
        if self.transaction_buffer.is_none() { return; }

        if self.config.regs.dma_out_link >= 0 {
            self.clear_register_bits(self.config.regs.dma_out_link as u32, ECC_REG57);
        }

        let dma_in_link_val = if self.config.regs.dma_in_link >= 0 {
            self.read_register(self.config.regs.dma_in_link as u32)
        } else { 0 };

        if dma_in_link_val & ECC_REG58 != 0 {
            self.dma_in.start(dma_in_link_val);
            let buf = self.transaction_buffer.take();
            if let Some(ref buf) = buf {
                self.dma_in.copy_in_bytes(&buf[..]);
            }
            self.clear_register_bits(self.config.regs.dma_in_link as u32, ECC_REG58);
        } else if let Some(link) = self.dma_in_link {
            self.dma_in.start(link);
            let buf = self.transaction_buffer.take();
            if let Some(ref buf) = buf {
                self.dma_in.copy_in_bytes(&buf[..]);
            }
        } else {
            let buf_taken = self.transaction_buffer.take();
            if let Some(buf) = buf_taken.as_ref() {
                let len = cmp::min(self.transaction_buffer_len, 64) as usize;
                let w0 = self.config.regs.w0 as u32;
                // Virtual-bus MISO override: slave bytes replace the loopback
                // echo in the receive range (slave_miso.1 = valid length).
                let (miso, miso_len) = slave_miso
                    .as_ref()
                    .map(|(b, n)| (b.as_slice(), *n as usize))
                    .unwrap_or((&[][..], 0));
                for i in (0..len).step_by(4) {
                    let mut word = 0u32;
                    for j in 0..4 {
                        let idx = i + j;
                        let b = if idx < len && idx < miso_len {
                            miso[idx]
                        } else if idx < len {
                            buf[idx]
                        } else {
                            0
                        };
                        if idx < len {
                            word |= (b as u32) << (j * 8);
                        }
                    }
                    self.write_register(w0 + (i as u32) * 4, word);
                }
            }
        }

        let usr_conf = self.config.fields.usr_conf.map_or(false, |f| self.read_field(&f) != 0);
        let seg_done = self.config.fields.dma_seg_trans_done;
        let seg_done_int = self.config.fields.dma_seg_trans_done_int_raw;
        let trans_done = self.config.fields.trans_done_int_raw;
        if usr_conf {
            if let Some(f) = seg_done { self.write_field(&f, 1); }
            if let Some(f) = seg_done_int { self.write_field(&f, 1); }
        } else if let Some(f) = trans_done {
            self.write_field(&f, 1);
        } else if self.config.regs.slave >= 0 {
            self.set_register_bits(self.config.regs.slave as u32, ECC_REG59);
        }

        self.update_interrupt(ctx);
        self.transaction_active = false;
        self.transaction_buffer = None;
        self.dma_out_link = None;

        let user_val = self.read_register(self.config.regs.user as u32);
        if usr_conf && user_val & ECC_REG50 != 0 {
            self.start_transaction(ctx);
        }
    }

    // JS beginSlaveTransaction()
    // REAL-HW PARITY (spi slave): the driver's mount ends with
    // spi_slave_hal_user_start() = SLAVE[DONE]-clear + CMD[USR]=1.
    // That only ARMS the slave; the transaction completes when the master
    // clocks (spi_bus_exchange raises SLAVE DONE + the RX bit counter).
    // Completing here fires a phantom TRANS_DONE whose store runs before
    // any MOSI arrives (hw[100] RX counter still 0), so the driver copies
    // 0 bytes and reports trans_len=0 with an empty rx_buffer.
    fn begin_slave_transaction(&mut self, _ctx: &mut CpuContext) {
        self.slave_active = true;
    }

    // JS completeSlaveTransaction()
    // Unreachable: slave CMD[USR] is an arm (see above); completion is
    // driven by spi_bus_exchange. Kept for reference.
    #[allow(dead_code)]
    fn complete_slave_transaction(&mut self, ctx: &mut CpuContext) {
        if !self.slave_mode() || !self.slave_active { return; }
        if self.config.regs.dma_out_link >= 0 {
            self.clear_register_bits(self.config.regs.dma_out_link as u32, ECC_REG57);
        }
        if self.config.regs.dma_in_link >= 0 {
            self.clear_register_bits(self.config.regs.dma_in_link as u32, ECC_REG58);
        }
        if let Some(f) = self.config.fields.slv_data_bytelen {
            self.write_field(&f, self.slave_writer.length);
        }
        if let Some(f) = self.config.fields.slv_data_bitlen {
            self.write_field(&f, 8 * self.slave_writer.length);
        }
        self.slave_reader.reset();
        self.slave_writer.reset();
        let trans_done = self.config.fields.trans_done_int_raw;
        if let Some(f) = trans_done {
            self.write_field(&f, 1);
        } else if self.config.regs.slave >= 0 {
            self.set_register_bits(self.config.regs.slave as u32, ECC_REG59);
        }
        self.update_interrupt(ctx);
        self.slave_active = false;
    }

    // JS dmaSetIn / dmaSetOut
    pub fn dma_set_in(&mut self, link: u32, channel: Option<u32>) {
        self.dma_in_link = Some(link);
        self.dma_in_channel = channel;
    }
    pub fn dma_set_out(&mut self, link: u32, channel: Option<u32>) {
        self.dma_out_link = Some(link);
        self.dma_out_channel = channel;
    }

    // JS eraseFlash(cpuVal, tmpVal, idxVal) — WIP cleared via JS clock event bridge
    pub fn erase_flash(&mut self, ctx: &mut CpuContext, offset: u32, size: u32, delay_ticks: u64) {
        let start = offset as usize;
        let end = (offset + size) as usize;
        for i in start..end { self.flash_set_byte(i, 255); }
        self.status_reg |= DS_REG11;
        // Native clock event: clear DS_REG11 when the erase completes (JS parity:
        // delay_ticks is in nanoseconds, converted to 80MHz APB ticks).
        crate::peripherals::common::spi_syscon::schedule_global(
            (delay_ticks * 80) / 1000,
            EventTag::SpiFlashEraseDone { idx: self.idx },
        );
        let _ = ctx;
    }

    // JS flashEraseDoneEvent callback: () => { this.statusReg &= ~DS_REG11 }
    pub fn erase_done(&mut self) {
        self.status_reg &= !DS_REG11;
    }

    // JS flashCommand(cpuVal, tmpVal, idxVal, ClockEvent)
    fn flash_command(&mut self, ctx: &mut CpuContext, cmd: u32, addr: u32, send: u32, recv: u32) {
        match cmd {
            1 => {
                self.status_reg = (0xFF01 & self.status_reg) | self.read_register(self.config.regs.w0 as u32);
            }
            49 => {
                self.status_reg = (255 & self.status_reg) | (self.read_register(self.config.regs.w0 as u32) << 8);
            }
            2 => {
                if self.xts.state == XtsState::Visible { return; }
                let w0 = self.config.regs.w0 as u32;
                for i in 0..(send as usize) {
                    let reg_idx = w0 + ((i / 4) * 4) as u32;
                    let word = self.read_register(reg_idx);
                    let byte = ((word >> ((i % 4) * 8)) & 0xFF) as u8;
                    let dest = (addr as usize) + i;
                    self.flash_set_byte(dest, byte);
                }
            }
            4 => { self.status_reg &= !DS_REG12; }
            5 => { self.write_register(self.config.regs.w0 as u32, 255 & self.status_reg); }
            53 => { self.write_register(self.config.regs.w0 as u32, self.status_reg >> 8); }
            6 => { self.status_reg |= DS_REG12; }
            32 | 82 | 216 => {
                let erase_addr = if self.config.pms_addr_split && send > 0 {
                    (addr << 8) | (255 & self.read_register(self.config.regs.w0 as u32))
                } else {
                    addr
                };
                let (reg_size, delay) = match cmd {
                    32 => (4096, ECC_REG26),
                    82 => (32768, ECC_REG27),
                    _ => (65536, ECC_REG27),
                };
                self.erase_flash(ctx, erase_addr & !(reg_size - 1), reg_size, delay as u64);
            }
            159 => { self.write_register(self.config.regs.w0 as u32, self.flash_id); }
            3 | 11 | 59 | 107 | 187 | 235 => {
                let w0 = self.config.regs.w0 as u32;
                for i in 0..(recv as usize) {
                    let src = (addr as usize) + i;
                    let byte = self.flash_get_byte(src);
                    let reg_idx = w0 + ((i / 4) * 4) as u32;
                    let existing = self.read_register(reg_idx);
                    let shift = (i % 4) * 8;
                    let new_word = (existing & !(0xFFu32 << shift)) | ((byte as u32) << shift);
                    self.write_register(reg_idx, new_word);
                }
            }
            _ => {}
        }
    }

    // JS psramCommand(cpuVal, tmpVal)
    fn psram_command(&mut self, cmd: u32, val: u32) {
        match cmd {
            0 => {
                if 0x9F00_0000 == val {
                    let p0 = (self.psram_id) & 0xFF;
                    let p1 = (self.psram_id >> 8) & 0xFF;
                    let p2 = (self.psram_id >> 16) & 0xFF;
                    let word0 = p0 | (p1 << 8) | (p2 << 16) | (0xAA << 24);
                    let word1 = 0xBB | (0xCC << 8) | (0xDD << 16) | (0xEE << 24);
                    self.write_register(self.config.regs.w0 as u32, word0);
                    self.write_register(self.config.regs.w0 as u32 + 4, word1);
                }
            }
            159 => { self.write_register(self.config.regs.w0 as u32, self.psram_id); }
            _ => {}
        }
    }

    // JS psramOctalCommand(cpuVal, tmpVal, idxVal, ClockEvent)
    fn psram_octal_command(&mut self, cmd: u32, _addr: u32, _len: u32, _clock: u32) {
        match cmd {
            ECC_REG21 | ECC_REG23 => {}
            ECC_REG22 => {
                let jedec = table_lookup(
                    PSRAM_JEDEC_TABLE, self.flash_buffer_len >> 20, DEFAULT_PSRAM_JEDEC,
                );
                self.write_register(self.config.regs.w0 as u32, 0x00_40_60 | (jedec << 16) | 0x0D_28_00);
            }
            _ => {}
        }
    }

    // JS updateInterrupt()
    pub fn update_interrupt(&mut self, ctx: &mut CpuContext) {
        let mut irq_val = false;

        if self.config.fields.trans_done_int_raw.is_some() {
            irq_val = (self.read_register(self.config.regs.dma_int_raw as u32)
                & self.read_register(self.config.regs.dma_int_ena as u32)) != 0;
        } else {
            let slave_val = self.read_register(self.config.regs.slave as u32);
            irq_val = irq_val || (slave_val & DS_REG1 != 0 && slave_val & ECC_REG59 != 0);

            if let Some(f) = &self.config.fields.dma_seg_trans_done {
                irq_val = irq_val || (slave_val & DS_REG2 != 0 && self.read_field(f) != 0);
            }
            if let Some(f) = &self.config.fields.seg_magic_err {
                irq_val = irq_val || (slave_val & DS_REG3 != 0 && self.read_field(f) != 0);
            }
        }

        ctx.interrupt(self.config.irq, irq_val);
    }

    // JS reset()
    pub fn mmio_reset(&mut self) {
        self.base.reset();
        self.xts.reset();
        self.dma_out_link = None;
        let trans_done = self.config.fields.trans_done_int_raw;
        let seg_done = self.config.fields.dma_seg_trans_done;
        let seg_done_int = self.config.fields.dma_seg_trans_done_int_raw;
        if let Some(f) = trans_done { self.write_field(&f, 1); }
        if let Some(f) = seg_done { self.write_field(&f, 1); }
        if let Some(f) = seg_done_int { self.write_field(&f, 1); }
        self.slave_active = false;
        self.slave_reader.reset();
        self.slave_writer.reset();
    }

    // Register access helpers — delegate to PeripheralBase.
    // Public: the native virtual-SPI-bus bridge (native_mmio.rs) drives the
    // slave side (W-reg data exchange + TRANS_DONE) from the master side.
    pub fn read_register(&self, offset: u32) -> u32 { self.base.read_register(offset) }
    pub fn write_register(&mut self, offset: u32, val: u32) { self.base.write_register(offset, val); }
    pub fn read_field(&self, field: &FieldDesc) -> u32 { self.base.read_field(field) }
    pub fn write_field(&mut self, field: &FieldDesc, val: u32) { self.base.write_field(field, val); }
    pub fn set_register_bits(&mut self, offset: u32, bits: u32) -> u32 { self.base.set_register_bits(offset, bits) }
    pub fn clear_register_bits(&mut self, offset: u32, bits: u32) -> u32 { self.base.clear_register_bits(offset, bits) }
}

// ============================================================
// MmioPeripheral impl for SpiPeripheral
// ============================================================

impl MmioPeripheral for SpiPeripheral {
    fn read_u32(&mut self, ctx: &mut CpuContext, addr: u32) -> u32 {
        self.mmio_read_u32(ctx, addr)
    }
    fn write_u32(&mut self, ctx: &mut CpuContext, addr: u32, val: u32) {
        self.mmio_write_u32(ctx, addr, val);
    }
    fn reset(&mut self) {
        self.mmio_reset();
    }
}

// ============================================================
// SysconPeripheral — JS class SysconPeripheral extends PeripheralBase
// ============================================================

pub struct SysconPeripheral {
    pub base: PeripheralBase,
}

impl SysconPeripheral {
    pub fn new(base_addr: u32, name: &'static str) -> Self {
        SysconPeripheral { base: PeripheralBase::new(base_addr, name) }
    }

    // JS readUint32(cpuVal)
    pub fn mmio_read_u32(&mut self, _ctx: &mut CpuContext, addr: u32) -> u32 {
        if addr.wrapping_sub(self.base.base_addr) == DS_REG20 {
            return 0x9604_2000;
        }
        self.base.read_uint32(addr)
    }

    // JS writeUint32(cpuVal, tmpVal)
    pub fn mmio_write_u32(&mut self, ctx: &mut CpuContext, addr: u32, val: u32) {
        let offset = addr.wrapping_sub(self.base.base_addr);
        match offset {
            x if x == DS_REG15 => {
                // JS: ClockEvent.sysclkPreDiv = (tmpVal >> dsReg22) & dsReg21; ClockEvent.update()
                // ctx.clocks does not have sysclkPreDiv; would need ClockTree reference.
                ctx.update_clocks();
            }
            x if x == DS_REG16 => {
                // JS: ClockEvent.xtalTicks = tmpVal & SysconTickCountMask; ClockEvent.update()
                ctx.update_clocks();
                return;
            }
            x if x == DS_REG17 => {
                // JS: ClockEvent.pllTicks = tmpVal & SysconTickCountMask; ClockEvent.update()
                ctx.update_clocks();
                return;
            }
            x if x == DS_REG18 => {
                // JS: ClockEvent.rtc8mTicks = tmpVal & SysconTickCountMask; ClockEvent.update()
                ctx.update_clocks();
                return;
            }
            x if x == DS_REG19 => {
                // JS: ClockEvent.apllTicks = tmpVal & SysconTickCountMask; ClockEvent.update()
                ctx.update_clocks();
                return;
            }
            _ => {}
        }
        self.base.write_uint32(addr, val);
    }

    // JS reset()
    pub fn mmio_reset(&mut self) {
        self.base.reset();
        // Write reset values to registers. In JS, this calls writeUint32 which triggers ClockEvent.update().
        // In Rust reset() there's no ctx, so we write raw register values.
        self.base.write_uint32(self.base.base_addr + DS_REG15, 0);
        self.base.write_uint32(self.base.base_addr + DS_REG16, 39);
        self.base.write_uint32(self.base.base_addr + DS_REG17, 79);
        self.base.write_uint32(self.base.base_addr + DS_REG18, 11);
        self.base.write_uint32(self.base.base_addr + DS_REG19, 99);
    }
}

// ============================================================
// MmioPeripheral impl for SysconPeripheral
// ============================================================

impl MmioPeripheral for SysconPeripheral {
    fn read_u32(&mut self, ctx: &mut CpuContext, addr: u32) -> u32 {
        self.mmio_read_u32(ctx, addr)
    }
    fn write_u32(&mut self, ctx: &mut CpuContext, addr: u32, val: u32) {
        self.mmio_write_u32(ctx, addr, val);
    }
    fn reset(&mut self) {
        self.mmio_reset();
    }
}

// ============================================================
// CpuContext::dummy() — minimal stub for internal use
// Only used by complete_slave_transaction which is called from begin_slave_transaction
// which is called from start_transaction which has a real ctx. In practice this should
// be replaced by threading ctx through. The dummy exists for compile-time compatibility.
//
// The global EVENT_QUEUE is shared by every make_ctx() CpuContext, so events
// scheduled by MMIO handlers (UART timeout, I2S, SDMMC, ...) persist across
// FFI calls and are pumped by native_process_events() from the host loop.
// Event deadlines are in APB ticks (80MHz) against js_apb_ticks().
// ============================================================

pub(crate) static mut EVENT_QUEUE: core::cell::UnsafeCell<EventQueue> =
    core::cell::UnsafeCell::new(EventQueue::new());

pub(crate) fn schedule_global(delta_apb_ticks: u64, tag: EventTag) -> usize {
    unsafe {
        let current = crate::native_mmio::clk_apb();
        EVENT_QUEUE.get().as_mut().unwrap().schedule(delta_apb_ticks, tag, current)
    }
}

pub(crate) fn unschedule_global(tag: EventTag) {
    unsafe { EVENT_QUEUE.get().as_mut().unwrap().unschedule_all(tag); }
}

pub(crate) fn fire_pending_global<F: FnMut(EventTag)>(current_tick: u64, mut handler: F) {
    unsafe { EVENT_QUEUE.get().as_mut().unwrap().fire_pending(current_tick, &mut handler); }
}

impl CpuContext<'_> {
    pub fn dummy() -> Self {
        use core::cell::UnsafeCell;
        static mut INT_MATRIX: UnsafeCell<InterruptMatrix> = UnsafeCell::new(InterruptMatrix::new());
        static mut GPIO_MATRIX: UnsafeCell<GpioMatrix> = UnsafeCell::new(GpioMatrix::new());
        static mut CLOCKS: UnsafeCell<Clocks> = UnsafeCell::new(Clocks {
            apb: ClockRef::new(0), xtal: ClockRef::new(0),
            rc_fast: ClockRef::new(0), ref_tick: ClockRef::new(0),
            cpu_clock_period: 0,
        });
        static mut GPIO_PINS: UnsafeCell<[GpioPinState; 40]> = UnsafeCell::new([GpioPinState::new(); 40]);
        unsafe {
            CpuContext {
                apb_ticks_val: 0,
                clock_nanos_val: 0,
                cpu_tick: 0,
                chip_name: "",
                int_matrix: &mut *INT_MATRIX.get(),
                gpio_matrix: &mut *GPIO_MATRIX.get(),
                gpio_pins: &mut *GPIO_PINS.get(),
                event_queue: &mut *EVENT_QUEUE.get(),
                clocks: &mut *CLOCKS.get(),
            }
        }
    }
}
