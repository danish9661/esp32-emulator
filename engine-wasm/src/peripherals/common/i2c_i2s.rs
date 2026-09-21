// Translated from: src/peripherals/common/i2c-i2s.js
// Do NOT modify the logic — match the JS line-for-line

use crate::peripherals::types::{ClockRef, CpuContext, EventTag, MmioPeripheral};
use crate::peripherals::common::peripheral::*;

// ============================================================
// Register/Mask Constants (match JS let/const declarations)
// ============================================================

pub const TIM_REG21: u32 = 4;
pub const TIM_REG22: u32 = 8;
pub const TIM_REG23: u32 = 24;
pub const I2C_REG1: u32 = 0;
pub const I2C_REG2: u32 = 31;
pub const I2C_REG3: u32 = 5;
pub const I2C_REG4: u32 = 31;
pub const I2C_REG5: u32 = 10;
pub const I2C_REG6: u32 = 31;
pub const I2C_REG7: u32 = 15;
pub const I2C_REG8: u32 = 31;
pub const I2C_REG9: u32 = 32;
pub const I2C_REG10: u32 = 36;
pub const I2C_REG11: u32 = 40;
pub const I2C_REG12: u32 = 44;
pub const I2C_REG13: u32 = 88;
pub const OF: u32 = 28;
pub const I2C_REG14: u32 = 32;
pub const I2C_REG15: u32 = 256;
pub const I2C_REG16: u32 = 128;
pub const I2C_REG17: u32 = 32;
pub const I2C_REG18: u32 = 18;
pub const I2C_REG19: u32 = 8;
pub const I2C_REG20: u32 = 1;
pub const I2C_REG21: u32 = 20;
pub const I2C_REG22: u32 = 63;
pub const I2C_REG23: u32 = 14;
pub const I2C_REG24: u32 = 63;
pub const I2C_REG25: u32 = 8192;
pub const I2C_REG26: u32 = 4096;
pub const I2C_REG27: u32 = 1;
pub const I2C_REG28: u32 = 0x8000_0000;
pub const I2C_REG29: u32 = 11;
pub const I2C_REG30: u32 = 7;
pub const I2C_REG31: u32 = 1024;
pub const I2C_REG32: u32 = 512;
pub const I2C_REG33: u32 = 256;
pub const I2C_REG34: u32 = 0;
pub const I2C_REG35: u32 = 255;
pub const I2C_REG36: u32 = 512;
pub const I2C_REG37: u32 = 4096;
pub const I2C_REG38: u32 = 65536;
// ADC DIG (continuous/DMA) pattern feed for I2S0 RX DMA. The DIG controller
// routes samples to I2S when SYSCON.saradc_ctrl.data_to_i2s (SYSCON+0x10
// bit26) is set. Pattern tables: SAR1 SYSCON+0x1C..0x28, SAR2 +0x2C..0x38;
// slot byte = {atten[1:0], bit_width[1:0], channel[3:0]} at word bits
// [31-8k, 24-8k] for slot%4==k (IDF adc_ll_digi_set_pattern_table).
static mut ADC_DIGI_SLOT: u32 = 0;
const ADC1_GPIO_MAP: [u32; 8] = [36, 37, 38, 39, 32, 33, 34, 35];
const ADC2_GPIO_MAP: [u32; 8] = [4, 0, 2, 15, 13, 12, 14, 27];

fn adc_digi_to_i2s() -> bool {
    (crate::native_mmio::syscon_read_raw(0x10) >> 26) & 1 == 1
}

fn adc_digi_sample(slot: u32) -> u16 {
    let ctrl = crate::native_mmio::syscon_read_raw(0x10);
    // ONLY_ADC2 = single mode (work_mode 0) + sar_sel 1; anything else (incl.
    // BOTH/ALTER 11-bit formats) falls back to the SAR1 table.
    let only_adc2 = ((ctrl >> 3) & 3) == 0 && ((ctrl >> 5) & 1) == 1;
    let (tab_base, patt_len) = if only_adc2 {
        (0x2Cu32, ((ctrl >> 16) & 0xF) + 1)
    } else {
        (0x1Cu32, ((ctrl >> 12) & 0xF) + 1)
    };
    let idx = slot % patt_len;
    let word = crate::native_mmio::syscon_read_raw(tab_base + 4 * (idx / 4));
    let pat = (word >> (24 - 8 * (idx % 4))) & 0xFF;
    let atten = pat & 3;
    let ch = (pat >> 4) & 0xF;
    let map = if only_adc2 { ADC2_GPIO_MAP } else { ADC1_GPIO_MAP };
    let pin = if (ch as usize) < 8 { map[ch as usize] } else { map[0] };
    let counts = unsafe { crate::peripherals::common::ffi::js_on_analog_read(pin, 9 + atten) };
    (((ch & 0xF) << 12) | (counts & 0xFFF)) as u16
}
pub const I2C_REG39: u32 = 1048575;
pub const I2C_REG40: u32 = 0x1000_0000;
pub const I2C_REG41: u32 = 0x2000_0000;
pub const I2C_REG42: u32 = 0x4000_0000;
pub const I2C_REG43: u32 = 0x1000_0000;
pub const I2C_REG44: u32 = 0x2000_0000;
pub const I2C_REG45: u32 = 0x4000_0000;
pub const I2C_REG46: u32 = 1;
pub const I2C_REG47: u32 = 2;
pub const I2C_REG49: u32 = 128;
pub const I2C_REG50: u32 = 256;
pub const I2C_REG51: u32 = 512;
pub const I2C_REG52: u32 = 12;
pub const I2C_REG53: u32 = 7;

// I2C interrupt type constants — HARDWARE BIT NUMBERS from i2c_reg.h
// (RXFIFO_FULL=0, TXFIFO_EMPTY=1, RXFIFO_OVF=2, END_DETECT=3,
// SLAVE_TRAN_COMP=4, ARBITRATION_LOST=5, MASTER_TRAN_COMP=6,
// TRANS_COMPLETE=7, TIME_OUT=8, TRANS_START=9, ACK_ERR=10,
// RX_REC_FULL=11, TX_SEND_EMPTY=12). An earlier revision used sequential
// 0..8 IDs, so no interrupt ever matched what drivers wait for (every master
// transaction died: the new driver's ISR needs MST_COMPLETE=bit7 for DONE).
pub const I2C_INT_RXFIFO_FULL: u32 = 0;
pub const I2C_INT_TXFIFO_EMPTY: u32 = 1;
pub const I2C_INT_RXFIFO_OVF: u32 = 2;
pub const I2C_INT_END_DETECT: u32 = 3;
pub const I2C_INT_SLAVE_TRAN_COMP: u32 = 4;
pub const I2C_INT_ARBITRATION_LOST: u32 = 5;
pub const I2C_INT_MASTER_TRAN_COMP: u32 = 6;
pub const I2C_INT_TRANS_COMPLETE: u32 = 7;
pub const I2C_INT_TIME_OUT: u32 = 8;
pub const I2C_INT_TRANS_START: u32 = 9;
pub const I2C_INT_ACK_ERR: u32 = 10;
pub const I2C_INT_RX_REC_FULL: u32 = 11;
pub const I2C_INT_TX_SEND_EMPTY: u32 = 12;

// I2C command constants — hardware opcodes from the TRM COMDn op_code field
// (0=RSTART, 1=WRITE, 2=READ, 3=STOP, 4=END). The old values here were shifted
// (WRITE=0, RSTART=1, STOP=4, END=7), so no master transaction ever decoded
// and every transfer died silently (the worker test tolerated any error).
// The _NEW variants are unused on ESP32 (modern=false) — kept for shape parity.
pub const I2C_CMD_WRITE: u32 = 1;
pub const I2C_CMD_RSTART: u32 = 0;
pub const I2C_CMD_READ: u32 = 2;
pub const I2C_CMD_RSTART_NEW: u32 = 3;
pub const I2C_CMD_STOP: u32 = 3;
pub const I2C_CMD_READ_NEW: u32 = 5;
pub const I2C_CMD_STOP_NEW: u32 = 6;
pub const I2C_CMD_END: u32 = 4;

// ============================================================
// I2cCommandState (JS lines 49-58)
// ============================================================

pub struct I2cCommandState {
    pub offset: u32,
    pub ack_value: bool,
    pub ack_check: bool,
    pub ack_expected: bool,
    pub num_bytes: u32,
    pub current_byte: u32,
}

impl I2cCommandState {
    pub fn new() -> Self {
        I2cCommandState {
            offset: 0,
            ack_value: false,
            ack_check: false,
            ack_expected: false,
            num_bytes: 0,
            current_byte: 0,
        }
    }
}

// ============================================================
// I2C Config types
// ============================================================

#[derive(Clone, Copy)]
pub struct I2cRmtChannelRegister {
    pub fifo_st: u32,
    pub scl_sp_conf: u32,
    pub clk_conf: i32,
    pub comd8: i32,
}

#[derive(Clone, Copy)]
pub struct I2cF {
    pub scl_low_period: FieldDesc,
    pub scl_high_period: FieldDesc,
    pub scl_wait_high_period: Option<FieldDesc>,
    pub scl_filter_thres: FieldDesc,
    pub scl_filter_en: Option<FieldDesc>,
    pub ref_always_on: Option<FieldDesc>,
    pub sclk_sel: Option<FieldDesc>,
    pub sclk_div_num: Option<FieldDesc>,
    pub sclk_div_a: Option<FieldDesc>,
    pub sclk_div_b: Option<FieldDesc>,
}

#[derive(Clone, Copy)]
pub struct I2cConfig {
    pub modern: bool,
    pub clock_source: Option<ClockRef>,
    pub rmt_channel_register: I2cRmtChannelRegister,
    pub f: I2cF,
    pub filter_thres_timing: bool,
}

// ============================================================
// I2cPeripheral (JS lines 67-332)
// ============================================================

pub struct I2cPeripheral {
    pub base: PeripheralBase,
    pub config: I2cConfig,
    pub irq_num: u32,
    pub cmd_index: u32,
    pub rx_index: u32,
    pub tx_index: u32,
    pub tx_fifo_index: u32,
    pub rx_fifo_index: u32,
    pub rx_fifo_threshold: u32,
    pub tx_fifo_threshold: u32,
    pub command: I2cCommandState,
    pub internal_clock_parent: ClockRef,
    pub internal_clock_divider: f64,
    pub ack_received: bool,
    /// Controller index (0 = I2C0, 1 = I2C1) for virtual-bus routing.
    pub index: usize,
    /// Virtual I2C bus: true while the next WRITE byte is the address byte.
    pub bus_expect_addr: bool,
    /// Last address byte target (7-bit) and direction of this transaction.
    pub bus_addr: u32,
    pub bus_is_read: bool,
    /// Matched slave controller index (virtual bus), if any.
    pub bus_slave: Option<usize>,
    /// Matched virtual sensor (virtual bus: scripted slave device with no
    /// controller, e.g. the OV2640 camera). Mutually exclusive with
    /// bus_slave; ACKs and participates like a real slave.
    pub bus_virt: bool,
}

impl I2cPeripheral {
    pub fn new(base_addr: u32, name: &'static str, config: I2cConfig, irq_num: u32, xtal: ClockRef) -> Self {
        I2cPeripheral {
            base: PeripheralBase::new(base_addr, name),
            config,
            irq_num,
            cmd_index: 0,
            rx_index: 0,
            tx_index: 0,
            tx_fifo_index: 0,
            rx_fifo_index: 0,
            rx_fifo_threshold: 0,
            tx_fifo_threshold: 0,
            command: I2cCommandState::new(),
            internal_clock_parent: xtal,
            internal_clock_divider: 1.0,
            ack_received: false,
            index: 0,
            bus_expect_addr: true,
            bus_addr: 0,
            bus_is_read: false,
            bus_slave: None,
            bus_virt: false,
        }
    }

    // JS get intStatus (lines 87-89)
    pub fn int_status(&self) -> u32 {
        self.base.read_register(I2C_REG9) & self.base.read_register(I2C_REG11)
    }

    // JS get rxFIFOCount (lines 90-92)
    pub fn rx_fifo_count(&self) -> u32 {
        // Producer is rx_index (RX delivery advances it), consumer is
        // rx_fifo_index (OF reads advance it). (An earlier revision had the
        // subtraction backwards, reporting 29 instead of 3 after a 3-byte
        // delivery and confusing the slave ISR's RX drain.)
        (((self.rx_index.wrapping_sub(self.rx_fifo_index)) >> 2) + I2C_REG14) % I2C_REG14
    }

    // JS get txFIFOCount (lines 93-95)
    pub fn tx_fifo_count(&self) -> u32 {
        (((self.tx_fifo_index.wrapping_sub(self.tx_index)) >> 2) + I2C_REG14) % I2C_REG14
    }

    // JS get sourceClock (lines 96-126)
    pub fn source_clock(&mut self, ctx: &mut CpuContext) -> ClockRef {
        if let Some(cs) = self.config.clock_source {
            return cs;
        }
        let f = &self.config.f;
        if self.config.rmt_channel_register.clk_conf >= 0
            && f.sclk_sel.is_some()
            && f.sclk_div_num.is_some()
            && f.sclk_div_a.is_some()
            && f.sclk_div_b.is_some()
        {
            let tmp_val = self.base.read_register(self.config.rmt_channel_register.clk_conf as u32);
            let sel_val = read_field_value(tmp_val, f.sclk_sel.as_ref().unwrap());
            let div_a = read_field_value(tmp_val, f.sclk_div_a.as_ref().unwrap());
            let div_b = read_field_value(tmp_val, f.sclk_div_b.as_ref().unwrap());
            let reg_val = read_field_value(tmp_val, f.sclk_div_num.as_ref().unwrap());
            let divider = reg_val as f64 + 1.0 + div_a as f64 / (div_b as f64 + 1.0);
            let parent = if sel_val != 0 {
                ctx.clocks.rc_fast
            } else {
                ctx.clocks.xtal
            };
            self.internal_clock_parent = parent;
            self.internal_clock_divider = divider;
            let freq = if divider != 0.0 {
                (parent.frequency as f64 / divider) as u32
            } else {
                0
            };
            return ClockRef { ticks: parent.ticks, frequency: freq };
        }
        {
            let ref_always_on = f.ref_always_on.as_ref();
            if let Some(field) = ref_always_on {
                let val = self.base.read_field(field);
                if val == 0 {
                    let ref_clk = ctx.clocks.ref_tick;
                    if ref_clk.frequency != 0 {
                        return ref_clk;
                    }
                }
            }
            return ctx.clocks.apb;
        }
    }

    // JS get busFrequency (lines 127-150)
    pub fn bus_frequency(&mut self, ctx: &mut CpuContext) -> u32 {
        let f = &self.config.f;
        let tmp_val = self.base.read_field(&f.scl_low_period);
        let idx_val = self.base.read_field(&f.scl_high_period);
        let clock_event = if let Some(ref field) = f.scl_wait_high_period {
            self.base.read_field(field)
        } else {
            0
        };
        let sim_clock = self.base.read_field(&f.scl_filter_thres);
        if tmp_val == 0 || idx_val == 0 {
            return 0;
        }
        let arg_val = tmp_val
            + idx_val
            + clock_event
            + {
                if let Some(ref field) = f.scl_filter_en {
                    if self.config.filter_thres_timing {
                        1 + if self.base.read_field(field) != 0 {
                            core::cmp::min(6 + sim_clock, 8)
                        } else {
                            7
                        }
                    } else {
                        1
                    }
                } else {
                    1 + if self.config.filter_thres_timing { sim_clock } else { 0 }
                }
            };
        let src = self.source_clock(ctx);
        if arg_val != 0 {
            src.frequency / arg_val
        } else {
            0
        }
    }

    // JS executeCommand (lines 151-182)
    pub fn execute_command(&mut self, ctx: &mut CpuContext) {
        let tmp_val = self.config.modern;
        let idx_val = if self.config.rmt_channel_register.comd8 >= 0 { 16 } else { 8 };
        self.command.offset = I2C_REG13 + 4 * self.cmd_index;
        let clock_event = self.base.read_register(self.command.offset);
        self.cmd_index = (self.cmd_index + 1) % idx_val;
        let sim_clock = (clock_event >> I2C_REG29) & I2C_REG30;
        self.command.num_bytes = (clock_event >> I2C_REG34) & I2C_REG35;
        self.command.current_byte = 0;
        let cmd_rstart = if tmp_val { I2C_CMD_RSTART_NEW } else { I2C_CMD_RSTART };
        let cmd_read = if tmp_val { I2C_CMD_READ_NEW } else { I2C_CMD_READ };
        let cmd_stop = if tmp_val { I2C_CMD_STOP_NEW } else { I2C_CMD_STOP };
        match sim_clock {
            x if x == cmd_rstart => {
                // this.onStart?.() → startComplete(true)
                self.start_complete(true, ctx);
            }
            I2C_CMD_WRITE => {
                self.command.ack_check = (clock_event & I2C_REG33) != 0;
                self.command.ack_expected = (clock_event & I2C_REG32) == 0;
                self.write_next(ctx);
            }
            x if x == cmd_read => {
                self.command.ack_value = (clock_event & I2C_REG31) == 0;
                self.read_next(ctx);
            }
            x if x == cmd_stop => {
                // this.onStop?.() → stopComplete
                self.stop_complete(ctx);
            }
            I2C_CMD_END => {
                self.set_interrupt(I2C_INT_END_DETECT, ctx);
            }
            _ => {}
        }
    }

    // JS writeNext (lines 183-188) — onWrite?.(cpuVal) always calls writeComplete(false)
    pub fn write_next(&mut self, ctx: &mut CpuContext) {
        // Byte is read from FIFO but discarded by the default onWrite callback
        let byte = 255 & self.base.read_register(I2C_REG15 + self.tx_index);
        self.tx_index = (self.tx_index + 4) % I2C_REG16;
        self.tx_fifo_updated(ctx);
        // Virtual I2C bus: route the byte to a matched slave (address snoop
        // + RX delivery). A matched slave ACKs (writeComplete(true) = clean
        // completion); with no match the historical NACK-ish path
        // (writeComplete(false)) is preserved bit-for-bit.
        // NOTE: the ACK comes back as the call's RETURN value, NOT via
        // self.bus_slave — self already mutably borrows the I2CS slot the
        // bus call mutates through the static, so re-reading self here is
        // aliased-&mut UB and the compiler is free to cache the stale None
        // (this silently flipped to always-NACK after an unrelated rebuild).
        let acked = crate::native_mmio::i2c_bus_write_byte(self.index, byte as u8, ctx);
        // onWrite is () => this.writeComplete(false) — ignores the byte argument
        self.write_complete(acked, ctx);
    }

    // JS readNext (lines 189-191) — onRead always calls readComplete(255)
    pub fn read_next(&mut self, ctx: &mut CpuContext) {
        // Virtual I2C bus: serve from a matched slave's TX FIFO when present
        // (0xFF otherwise — the historical no-device behavior).
        let byte = crate::native_mmio::i2c_bus_read_byte(self.index, ctx);
        self.read_complete(byte as u32, ctx);
    }

    // JS readComplete (lines 193-205)
    pub fn read_complete(&mut self, cpu_val: u32, ctx: &mut CpuContext) {
        self.base.set_register_bits(self.command.offset, I2C_REG28);
        self.base.write_register(I2C_REG15 + self.rx_index, cpu_val);
        self.rx_index = (self.rx_index + 4) % I2C_REG16;
        if self.rx_index == self.rx_fifo_index {
            self.set_interrupt(I2C_INT_RXFIFO_OVF, ctx);
        }
        self.command.current_byte += 1;
        if self.command.current_byte < self.command.num_bytes {
            self.read_next(ctx);
        } else {
            self.execute_command(ctx);
        }
        self.rx_fifo_updated(ctx);
    }

    // JS writeComplete (lines 206-220)
    pub fn write_complete(&mut self, cpu_val: bool, ctx: &mut CpuContext) {
        self.base.set_register_bits(self.command.offset, I2C_REG28);
        self.ack_received = cpu_val;
        if self.command.ack_check && cpu_val != self.command.ack_expected {
            self.set_interrupt(I2C_INT_ACK_ERR, ctx);
            self.stop_complete(ctx);
            return;
        }
        self.command.current_byte += 1;
        if self.command.current_byte < self.command.num_bytes {
            self.write_next(ctx);
        } else {
            self.execute_command(ctx);
        }
    }

    // JS startComplete (lines 221-226)
    pub fn start_complete(&mut self, cpu_val: bool, ctx: &mut CpuContext) {
        self.base.set_register_bits(self.command.offset, I2C_REG28);
        if cpu_val {
            // Virtual I2C bus: a (repeated) START opens a new transaction —
            // the next WRITE byte is the address byte.
            crate::native_mmio::i2c_bus_start(self.index);
            self.execute_command(ctx);
        } else {
            self.set_interrupt(I2C_INT_ARBITRATION_LOST, ctx);
        }
    }

    // JS stopComplete (lines 227-230)
    pub fn stop_complete(&mut self, ctx: &mut CpuContext) {
        self.base.set_register_bits(self.command.offset, I2C_REG28);
        // Virtual I2C bus: STOP closes the transaction (slave RX event).
        crate::native_mmio::i2c_bus_stop(self.index, ctx);
        self.set_interrupt(I2C_INT_TRANS_COMPLETE, ctx);
    }

    // JS rxFIFOUpdated (lines 231-236)
    // Note: `>` is hardware-correct here. The JS's `>=` with rx_fifo_threshold 0 and
    // an empty FIFO asserts RX_REC_FULL forever (interrupt storm) — same class as the
    // UART RX-FULL fix.
    pub fn rx_fifo_updated(&mut self, ctx: &mut CpuContext) {
        let cpu_val = self.rx_fifo_count();
        if cpu_val > self.rx_fifo_threshold {
            self.set_interrupt(I2C_INT_RX_REC_FULL, ctx);
        }
        if cpu_val == I2C_REG14 {
            self.set_interrupt(I2C_INT_RXFIFO_FULL, ctx);
        }
    }

    // JS txFIFOUpdated (lines 237-242)
    pub fn tx_fifo_updated(&mut self, ctx: &mut CpuContext) {
        let cpu_val = self.tx_fifo_count();
        if cpu_val < self.tx_fifo_threshold {
            self.set_interrupt(I2C_INT_TX_SEND_EMPTY, ctx);
        }
        if cpu_val == 0 {
            self.set_interrupt(I2C_INT_TXFIFO_EMPTY, ctx);
        }
    }

    // JS readUint16 (lines 274-276)
    pub fn read_uint16(&mut self, ctx: &mut CpuContext, addr: u32) -> u16 {
        (self.read_u32(ctx, addr) & 0xFFFF) as u16
    }

    // JS readUint8 (lines 277-279)
    pub fn read_uint8(&mut self, ctx: &mut CpuContext, addr: u32) -> u8 {
        (self.read_u32(ctx, addr) & 0xFF) as u8
    }

    // JS writeUint8 (lines 280-282)
    pub fn write_uint8(&mut self, ctx: &mut CpuContext, addr: u32, val: u8) {
        self.write_u32(ctx, addr, val as u32);
    }

    // JS setInterrupt (lines 320-323)
    pub fn set_interrupt(&mut self, cpu_val: u32, ctx: &mut CpuContext) {
        self.base.set_register_bits(I2C_REG9, 1 << cpu_val);
        if self.int_status() != 0 {
            ctx.interrupt(self.irq_num, true);
        }
    }

    // JS reset (lines 324-331)
    pub fn reset_peripheral(&mut self) {
        self.base.reset();
        self.cmd_index = 0;
        self.rx_index = 0;
        self.rx_fifo_index = 0;
        self.tx_index = 0;
        self.tx_fifo_index = 0;
        self.bus_expect_addr = true;
        self.bus_addr = 0;
        self.bus_is_read = false;
        self.bus_slave = None;
        self.bus_virt = false;
    }
}

impl MmioPeripheral for I2cPeripheral {
    // JS readUint32 (lines 243-273)
    fn read_u32(&mut self, ctx: &mut CpuContext, addr: u32) -> u32 {
        let offset = addr.wrapping_sub(self.base.base_addr);
        let fifo_st = self.config.rmt_channel_register.fifo_st;
        match offset {
            TIM_REG22 => {
                // STATUS_REG: live FIFO counts + ACK bit, ORed with slave-mode
                // sticky bits the virtual bus maintains in the backing file
                // (slave_rw/busy/addressed/main-state; bits 1-7,24-30).
                // Count fields (8-23) always come from the live indices.
                let sticky = self.base.read_register(8) & 0x7F0000FE;
                (self.tx_fifo_count() << I2C_REG18)
                    | (self.rx_fifo_count() << I2C_REG19)
                    | (if self.ack_received { I2C_REG20 } else { 0 })
                    | sticky
            }
            OF => {
                let cpu_val = self.base.read_register(I2C_REG15 + self.rx_fifo_index);
                if self.rx_fifo_index != self.rx_index {
                    self.rx_fifo_index = (self.rx_fifo_index + 4) % I2C_REG16;
                    self.rx_fifo_updated(ctx);
                }
                cpu_val
            }
            x if x == fifo_st => {
                (((self.rx_fifo_index >> 2) & I2C_REG2) << I2C_REG1)
                    | (((self.rx_index >> 2) & I2C_REG4) << I2C_REG3)
                    | (((self.tx_index >> 2) & I2C_REG6) << I2C_REG5)
                    | (((self.tx_fifo_index >> 2) & I2C_REG8) << I2C_REG7)
            }
            I2C_REG12 => self.int_status(),
            _ => self.base.read_uint32(addr),
        }
    }

    // JS writeUint32 (lines 283-319)
    fn write_u32(&mut self, ctx: &mut CpuContext, addr: u32, mut val: u32) {
        // Line 284: super.writeUint32(cpuVal, tmpVal)
        self.base.write_uint32(addr, val);

        let offset = addr.wrapping_sub(self.base.base_addr);
        let clock_event_conf = self.config.rmt_channel_register.scl_sp_conf;
        match offset {
            TIM_REG21 => {
                // Line 289: super.writeUint32(cpuVal, tmpVal & ~i2cReg17)
                self.base.write_uint32(addr, val & !I2C_REG17);
                if val & I2C_REG17 != 0 {
                    self.cmd_index = 0;
                    self.execute_command(ctx);
                }
                return;
            }
            TIM_REG23 => {
                if val & I2C_REG26 != 0 {
                    self.rx_index = 0;
                    self.rx_fifo_index = 0;
                }
                if val & I2C_REG25 != 0 {
                    self.tx_index = 0;
                    self.tx_fifo_index = 0;
                }
                self.tx_fifo_threshold = (val >> I2C_REG21) & I2C_REG22;
                self.rx_fifo_threshold = (val >> I2C_REG23) & I2C_REG24;
                self.rx_fifo_updated(ctx);
                self.tx_fifo_updated(ctx);
            }
            OF => {
                self.base.write_register(I2C_REG15 + self.tx_fifo_index, val);
                self.tx_fifo_index = (self.tx_fifo_index + 4) % I2C_REG16;
                self.tx_fifo_updated(ctx);
                return;
            }
            I2C_REG11 => {
                self.base.write_register(I2C_REG11, val);
                if self.int_status() != 0 {
                    ctx.interrupt(self.irq_num, true);
                }
            }
            I2C_REG10 => {
                // INT_CLR is write-1-to-clear on the latched bits. Do NOT
                // re-evaluate FIFO levels here: that synchronously re-asserts
                // live status (e.g. TXFIFO_EMPTY on an empty FIFO) and traps
                // `while(status){clear}` ISR loops forever. Live status
                // re-pends on the next FIFO event (OF read/write) or ENA write.
                self.base.clear_register_bits(I2C_REG9, val);
                if self.int_status() == 0 {
                    ctx.interrupt(self.irq_num, false);
                }
                return;
            }
            x if x == clock_event_conf => {
                val &= !I2C_REG27;
            }
            _ => {}
        }
        // Line 318: super.writeUint32(cpuVal, tmpVal)
        self.base.write_uint32(addr, val);
    }

    // JS reset (lines 324-331)
    fn reset(&mut self) {
        self.reset_peripheral();
    }
}

// ============================================================
// DmaDescriptorChain (JS lines 333-461)
// ============================================================
//
// NOTE: This struct requires two CpuContext methods not yet in types.rs:
//   - fn dma_base(&self) -> u32         (maps to this.cpu.dmaBase)
//   - fn map_address(&mut self, addr: u32, size: u32) -> &mut [u8]
//     (maps to this.cpu.mapAddress, returns memory interface for DMA)
//
// Until these are added, the DMA chain tracks pointer/offset state but
// cannot read/write DMA descriptor memory. The copyIn/copyOut stubs
// return 0 (no data transferred). This preserves the state machine
// but actual data movement requires the missing CpuContext methods.

pub struct DmaDescriptorChain {
    pub pointer: u32,
    pub offset: u32,
    /// Descriptors filled by the most recent copy_in that have not been
    /// reported yet. copy_in reports a completion (and advances) the moment
    /// a descriptor fills, so the frame-final descriptor is never stranded
    /// waiting for a next drain that never comes. Camera in_suc_eof fires
    /// per rx_eof_num threshold; legacy RX fires per completion.
    pub completed: u32,
    pub completed_addr: u32,
}

// JS: this.cpu.dmaBase (esp32.js) — OR'd with the 20-bit link address.
const DMA_BASE: u32 = 0x3ff00000;

fn dma_map_read_u32(addr: u32) -> u32 {
    crate::xtensa::memory::dma_read_u32(addr)
}

fn dma_map_write_u32(addr: u32, val: u32) {
    crate::xtensa::memory::dma_write_u32(addr, val)
}

impl DmaDescriptorChain {
    pub fn new() -> Self {
        DmaDescriptorChain {
            pointer: 0,
            offset: 0,
            completed: 0,
            completed_addr: 0,
        }
    }

    // JS start(cpuVal) — lines 340-345
    pub fn start(&mut self, addr: u32, _ctx: &mut CpuContext) {
        // cpuVal = this.cpu.dmaBase | (1048575 & cpuVal)
        // this.memory = this.cpu.mapAddress(cpuVal, 0)
        // this._pointer = cpuVal
        // this.offset = 0
        self.pointer = DMA_BASE | (I2C_REG39 & addr);
        self.offset = 0;
        self.completed = 0;
        self.completed_addr = 0;
    }

    // JS get size — lines 349-351
    pub fn get_size(&self) -> u32 {
        // 4095 & this.memory.readUint32(this._pointer)
        if self.pointer == 0 { 0 } else { 4095 & dma_map_read_u32(self.pointer) }
    }

    // JS get length — lines 352-354
    pub fn get_length(&self) -> u32 {
        // (this.memory.readUint32(this._pointer) >> 12) & 4095
        if self.pointer == 0 { 0 } else { (dma_map_read_u32(self.pointer) >> 12) & 4095 }
    }

    // JS set length — lines 355-358
    pub fn set_length(&mut self, cpu_val: u32) {
        if self.pointer == 0 { return; }
        // tmpVal = 0x7f000fff & this.memory.readUint32(this._pointer)
        // this.memory.writeUint32(this._pointer, tmpVal | ((4095 & cpuVal) << 12))
        let tmp_val = 0x7f000fff & dma_map_read_u32(self.pointer);
        dma_map_write_u32(self.pointer, tmp_val | ((4095 & cpu_val) << 12));
    }

    // JS get bufferPtr — lines 359-361
    pub fn buffer_ptr(&self) -> u32 {
        // this.memory.readUint32(this._pointer + 4)
        if self.pointer == 0 { 0 } else { dma_map_read_u32(self.pointer + 4) }
    }

    // JS copyIn (lines 362-383)
    pub fn copy_in(&mut self, src: &[u32]) -> u32 {
        if self.pointer == 0 { return 0; }
        let mut tmp_val = self.get_size() >> 2;
        let mut idx_val = self.buffer_ptr();
        let mut simulation_clock = self.offset >> 2;
        let mut r = 0usize;
        while r < src.len() {
            if simulation_clock >= tmp_val {
                if !self.next() {
                    return r as u32;
                }
                tmp_val = self.get_size() >> 2;
                idx_val = self.buffer_ptr();
                simulation_clock = 0;
            }
            dma_map_write_u32(idx_val + (simulation_clock << 2), src[r]);
            simulation_clock += 1;
            r += 1;
            // Descriptor just filled: report the completion and advance NOW.
            // The next drain may never come (frame-final descriptor), and a
            // lazy advance would strand its EOF forever (exactly one short
            // per frame: 9 EOFs, 3840 bytes stranded).
            if tmp_val != 0 && simulation_clock >= tmp_val {
                self.set_length(simulation_clock << 2);
                self.completed = self.completed.wrapping_add(1);
                self.completed_addr = self.pointer;
                if !self.next() {
                    self.offset = simulation_clock << 2;
                    return r as u32;
                }
                tmp_val = self.get_size() >> 2;
                idx_val = self.buffer_ptr();
                simulation_clock = 0;
            }
        }
        // this.length = SimulationClock << 2
        self.set_length(simulation_clock << 2);
        self.offset = simulation_clock << 2;
        src.len() as u32
    }

    // JS copyOut (lines 406-425)
    pub fn copy_out(&mut self, dst: &mut [u32]) -> u32 {
        if self.pointer == 0 { return 0; }
        let mut idx_val = self.get_size();
        // 3 & idxVal && console.warn("DMA unaligned!") — silently proceed
        idx_val >>= 2;
        let mut clock_event = self.buffer_ptr();
        let mut reg_val = self.offset >> 2;
        let mut a = 0usize;
        while a < dst.len() {
            if reg_val >= idx_val {
                if !self.next() { return a as u32; }
                idx_val = self.get_size() >> 2;
                clock_event = self.buffer_ptr();
                reg_val = 0;
            }
            dst[a] = dma_map_read_u32(clock_event + (reg_val << 2));
            reg_val += 1;
            a += 1;
        }
        self.offset = reg_val << 2;
        dst.len() as u32
    }

    // JS next (lines 454-460)
    pub fn next(&mut self) -> bool {
        // this._pointer = this.memory.readUint32(this._pointer + 8)
        // this._pointer && (this.memory = this.cpu.mapAddress(this._pointer, 0))
        // return this._pointer > 0
        if self.pointer == 0 { return false; }
        self.pointer = dma_map_read_u32(self.pointer + 8);
        self.pointer > 0
    }

    /// Take pending descriptor completions: (count, last completed addr).
    pub fn take_completed(&mut self) -> (u32, u32) {
        let c = (self.completed, self.completed_addr);
        self.completed = 0;
        self.completed_addr = 0;
        c
    }
}

// ============================================================
// I2S Config types
// ============================================================

#[derive(Clone, Copy)]
pub struct I2sRmtChannelRegister {
    pub int_raw: u32,
    pub int_ena: u32,
    pub int_st: u32,
    pub int_clr: u32,
    pub conf: u32,
    pub fifo_conf: u32,
    pub lc_conf: u32,
    pub out_link: u32,
    pub in_link: u32,
    pub conf2: u32,
    pub out_eof_des_addr: u32,
    pub in_eof_des_addr: u32,
    pub outlink_dscr: u32,
    pub inlink_dscr: u32,
    pub lc_state0: u32,
    pub lc_state1: u32,
    pub state: u32,
}

#[derive(Clone, Copy)]
pub struct I2sF {
    pub tx_reset: Option<FieldDesc>,
    pub rx_reset: Option<FieldDesc>,
    pub tx_fifo_reset: Option<FieldDesc>,
    pub rx_fifo_reset: Option<FieldDesc>,
    pub tx_start: Option<FieldDesc>,
    pub rx_start: Option<FieldDesc>,
    pub camera_en: Option<FieldDesc>,
    pub dscr_en: Option<FieldDesc>,
    pub tx_chan_mod: Option<FieldDesc>,
    pub rx_chan_mod: Option<FieldDesc>,
}

#[derive(Clone, Copy)]
pub struct I2sConfig {
    pub rmt_channel_register: I2sRmtChannelRegister,
    pub f: I2sF,
    pub camera_sample_mode: &'static str,
    pub camera_descriptors_per_half: u32,
}

// ============================================================
// I2sPeripheral (JS lines 477-943)
// ============================================================

pub struct I2sPeripheral {
    pub base: PeripheralBase,
    pub config: I2sConfig,
    pub irq: u32,
    pub index: u32,
    pub tx_data_hook: bool,
    pub int_raw: u32,
    pub int_ena: u32,
    pub dma_out: DmaDescriptorChain,
    pub dma_in: DmaDescriptorChain,
    pub tx_active: bool,
    pub rx_active: bool,
    pub dma_in_link: u32,
    pub dma_out_link: u32,
    pub rx_buffer: [u32; 65536],
    pub rx_buffer_len: usize,
    pub camera_mode: bool,
    pub camera_frame_queue: [u32; 4096],
    pub camera_frame_queue_len: usize,
    pub camera_frame_offset: u32,
    pub pending_camera_frame: u32,
    pub pending_camera_frame_len: u32,
    pub camera_chunk_size: u32,
    pub camera_descriptor_count: u32,
    /// Virtual-camera sensor byte offset (ramp: byte[i] = i & 0xFF).
    /// Reset on every new capture (in_link START in camera mode), so every
    /// frame starts at 0 and fb contents are verifiable. Counts sensor
    /// bytes = DMA elements (one byte per element in SM_0A00_0B00).
    pub camera_pattern_offset: u64,
    /// Camera EOF element accumulator (see process_rx: fires in_suc_eof per
    /// rx_eof_num DMA elements/words, HW parity).
    pub camera_eof_acc: u32,
    /// Virtual-camera frame size in SENSOR bytes (host-configured: the
    /// sensor emits exactly this many bytes per capture, then idles till the
    /// next capture start — real-sensor finite-frame parity; 0 = unbounded
    /// legacy). In SM_0A00_0B00 each sensor byte becomes one DMA element.
    pub camera_frame_bytes: u32,
    pub out_eof_des_addr: u32,
    pub in_eof_des_addr: u32,
    pub outlink_dscr: u32,
    pub inlink_dscr: u32,
}

impl I2sPeripheral {
    pub fn new(base_addr: u32, name: &'static str, config: I2sConfig, irq: u32, index: u32) -> Self {
        I2sPeripheral {
            base: PeripheralBase::new(base_addr, name),
            config,
            irq,
            index,
            tx_data_hook: false,
            int_raw: 0,
            int_ena: 0,
            dma_out: DmaDescriptorChain::new(),
            dma_in: DmaDescriptorChain::new(),
            tx_active: false,
            rx_active: false,
            dma_in_link: 0,
            dma_out_link: 0,
            rx_buffer: [0u32; 65536],
            rx_buffer_len: 0,
            camera_mode: false,
            camera_frame_queue: [0u32; 4096],
            camera_frame_queue_len: 0,
            camera_frame_offset: 0,
            pending_camera_frame: 0,
            pending_camera_frame_len: 0,
            camera_chunk_size: 3072,
            camera_descriptor_count: 0,
            camera_pattern_offset: 0,
            camera_eof_acc: 0,
            // Default finite frame (QQVGA RGB565); host may override via
            // native_i2s_cam_frame_bytes. Harmless for non-camera use: the
            // pattern feed only runs when camera_mode is set.
            camera_frame_bytes: 38400,
            out_eof_des_addr: 0,
            in_eof_des_addr: 0,
            outlink_dscr: 0,
            inlink_dscr: 0,
        }
    }

    // JS get intStatus (lines 521-523)
    pub fn int_status(&self) -> u32 {
        self.int_raw & self.int_ena
    }

    // JS get isDMAEnabled (lines 524-527)
    pub fn is_dma_enabled(&self) -> bool {
        if let Some(ref field) = self.config.f.dscr_en {
            self.base.read_field(field) != 0
        } else {
            false
        }
    }

    // JS readUint32 (lines 528-554)
    pub fn read_uint32(&mut self, addr: u32) -> u32 {
        let offset = addr.wrapping_sub(self.base.base_addr);
        let regs = &self.config.rmt_channel_register;
        match offset {
            x if x == regs.int_raw => self.int_raw,
            x if x == regs.int_ena => self.int_ena,
            x if x == regs.int_st => {
                self.int_status()
            }
            x if x == regs.out_eof_des_addr => self.out_eof_des_addr,
            x if x == regs.in_eof_des_addr => self.in_eof_des_addr,
            x if x == regs.outlink_dscr => self.outlink_dscr,
            x if x == regs.inlink_dscr => self.inlink_dscr,
            x if x == regs.lc_state0 => {
                if self.tx_active { 1 } else { 0 }
            }
            x if x == regs.lc_state1 => {
                if self.rx_active { 1 } else { 0 }
            }
            x if x == regs.state => {
                if self.tx_active || self.rx_active { 0 } else { 1 }
            }
            _ => self.base.read_uint32(addr),
        }
    }

    // JS injectCameraFrame (lines 623-625)
    pub fn inject_camera_frame(&mut self, cpu_val: u32, ctx: &mut CpuContext) {
        // this.cameraFrameQueue.push(cpuVal)
        if self.camera_frame_queue_len < 4096 {
            self.camera_frame_queue[self.camera_frame_queue_len] = cpu_val;
            self.camera_frame_queue_len += 1;
        }
        self.try_deliver_camera_frame(ctx);
    }

    // JS tryDeliverCameraFrame (lines 626-661), reworked: the original
    // pushed ZERO words from a never-fed queue (dead stub). The virtual
    // OV2640 instead synthesizes the sensor byte ramp (byte[i] = i & 0xFF),
    // packed into DMA elements per the configured SM_0A00_0B00 FIFO mode
    // (one sensor byte per element in sample1); process_rx drains the
    // elements through the normal RX DMA descriptors and in_suc_eof fires
    // per rx_eof_num elements, so the driver's filter reproduces the ramp.
    pub fn try_deliver_camera_frame(&mut self, ctx: &mut CpuContext) {
        if !self.camera_mode || !self.rx_active || self.dma_in.pointer == 0 {
            return;
        }
        // Keep the DMA fed well ahead (bounded by the finite frame size like
        // process_rx; process_rx drops consumed words, schedule_rx_processing
        // re-invokes while RX is active). Packing matches the configured
        // SM_0A00_0B00 FIFO mode: one sensor byte per 32-bit DMA element in
        // sample1 (bits 23:16); the driver's yuyv_highspeed filter extracts
        // exactly that byte.
        while self.rx_buffer_len < 4096
            && (self.camera_frame_bytes == 0
                || self.camera_pattern_offset < self.camera_frame_bytes as u64)
        {
            let o = self.camera_pattern_offset;
            let w = ((o & 0xFF) as u32) << 16;
            self.rx_buffer[self.rx_buffer_len] = w;
            self.rx_buffer_len += 1;
            self.camera_pattern_offset = o.wrapping_add(1);
        }
        self.process_rx(ctx);
    }

    // JS resetTX (lines 662-664)
    pub fn reset_tx(&mut self, ctx: &mut CpuContext) {
        self.stop_tx(ctx);
    }

    // JS resetRX (lines 665-667)
    pub fn reset_rx(&mut self, ctx: &mut CpuContext) {
        self.stop_rx(ctx);
        self.rx_buffer_len = 0;
    }

    // JS resetTXFIFO (lines 668-669)
    pub fn reset_tx_fifo(&mut self) {}

    // JS resetRXFIFO (lines 669-670)
    pub fn reset_rx_fifo(&mut self) {}

    // JS startTX (lines 670-674)
    pub fn start_tx(&mut self, ctx: &mut CpuContext) {
        if !self.tx_active {
            self.tx_active = true;
            if self.is_dma_enabled() && self.dma_out.pointer != 0 {
                self.process_tx(ctx);
            }
        }
    }

    // JS stopTX (lines 675-679)
    pub fn stop_tx(&mut self, ctx: &mut CpuContext) {
        if self.tx_active {
            self.flush_tx(ctx);
        }
        self.tx_active = false;
        // this.txClockEvent.unschedule() — requires EventTag scheduling
    }

    // JS flushTX (lines 680-701)
    pub fn flush_tx(&mut self, ctx: &mut CpuContext) {
        let mut cpu_val = 0u32;
        let tmp_val = 10000u32;
        while self.dma_out.pointer != 0 && cpu_val < tmp_val {
            cpu_val += 1;
            let mut _tmp_buf = [0u32; 8];
            let idx_val = self.dma_out.pointer;
            // this.dmaOut.copyOut(tmpVal, tmpVal.length) — stubbed, returns 0
            let clock_event = self.dma_out.copy_out(&mut _tmp_buf);
            let sim_clock = self.dma_out.pointer;
            if clock_event > 0 {
                // this.onTxData(tmpVal.subarray(0, ClockEvent))
                self.emit_tx_data(&_tmp_buf[..clock_event as usize]);
                if idx_val != sim_clock {
                    self.out_eof_des_addr = idx_val;
                    self.set_interrupt_i2s(I2C_REG37, ctx);
                    if sim_clock == 0 {
                        self.set_interrupt_i2s(I2C_REG38, ctx);
                        return;
                    }
                }
                self.outlink_dscr = sim_clock;
            } else {
                break;
            }
        }
    }

    // JS startRX (lines 702-714)
    pub fn start_rx(&mut self, ctx: &mut CpuContext) {
        if !self.rx_active {
            self.rx_active = true;
            if self.camera_mode {
                // this._cameraFrameRequestCallback check — stored as fn pointer
                // Callback returning u32 is not directly representable; skipped
                // if callback returns a value, injectCameraFrame would be called
            }
            if self.camera_mode {
                self.try_deliver_camera_frame(ctx);
            }
            if self.is_dma_enabled() && self.dma_in.pointer != 0 {
                self.process_rx(ctx);
            }
        }
    }

    // JS stopRX (lines 715-719)
    pub fn stop_rx(&mut self, ctx: &mut CpuContext) {
        if self.rx_active {
            // Camera mode: skip the stop-flush. Flushing here would push
            // buffered bytes into DMA WITHOUT EOF accounting (siphoning the
            // frame short of its exact byte count); the next capture start
            // drops residue explicitly instead.
            if !self.camera_mode {
                self.flush_rx(ctx);
            }
        }
        self.rx_active = false;
        // this.rxClockEvent.unschedule() — requires EventTag scheduling
    }

    // JS flushRX (lines 720-753)
    pub fn flush_rx(&mut self, ctx: &mut CpuContext) {
        let mut cpu_val = 0u32;
        let tmp_val = 10000u32;
        while self.rx_buffer_len > 0 && self.dma_in.pointer != 0 && cpu_val < tmp_val {
            cpu_val += 1;
            let chunk_len = if self.rx_buffer_len >= 8 { 8 } else { self.rx_buffer_len };
            if chunk_len == 0 {
                break;
            }
            let mut chunk = [0u32; 8];
            for i in 0..chunk_len {
                chunk[i] = self.rx_buffer[i];
            }
            let idx_val = self.dma_in.pointer;
            // this.dmaIn.copyIn(chunk) — stubbed, returns 0
            let clock_event = self.dma_in.copy_in(&chunk[..chunk_len]);
            let sim_clock = self.dma_in.pointer;
            if clock_event > 0 {
                // Remove copied items from rx_buffer
                if clock_event as usize <= self.rx_buffer_len {
                    for i in clock_event as usize..self.rx_buffer_len {
                        self.rx_buffer[i - clock_event as usize] = self.rx_buffer[i];
                    }
                    self.rx_buffer_len -= clock_event as usize;
                }
                if idx_val != sim_clock {
                    self.in_eof_des_addr = idx_val;
                    self.set_interrupt_i2s(I2C_REG36, ctx);
                    if sim_clock == 0 {
                        return;
                    }
                }
                self.inlink_dscr = sim_clock;
            } else {
                break;
            }
        }
        // Completions already reported via the pointer-change gate above;
        // discard so a later window never re-fires them.
        self.dma_in.take_completed();
    }

    // JS resetOutDMA (lines 754-755)
    pub fn reset_out_dma(&mut self) {}

    // JS resetInDMA (lines 755-756)
    pub fn reset_in_dma(&mut self) {}

    // JS startOutDMA (lines 756-760)
    pub fn start_out_dma(&mut self, cpu_val: u32, ctx: &mut CpuContext) {
        self.dma_out.start(cpu_val, ctx);
        self.outlink_dscr = cpu_val;
        if self.tx_active && self.is_dma_enabled() {
            self.process_tx(ctx);
        }
    }

    // JS stopOutDMA (line 761)
    pub fn stop_out_dma(&mut self) {}

    // JS restartOutDMA (lines 762-764)
    pub fn restart_out_dma(&mut self) {
        if self.outlink_dscr != 0 {
            // this.dmaOut.start(this.outlinkDscr) — needs ctx
        }
    }

    // JS startInDMA (lines 765-781)
    pub fn start_in_dma(&mut self, cpu_val: u32, ctx: &mut CpuContext) {
        self.dma_in.start(cpu_val, ctx);
        self.inlink_dscr = cpu_val;
        self.inlink_dscr = cpu_val;
        if self.camera_mode {
            // New capture: restart the test-pattern ramp so every frame
            // starts at byte 0. Also drop any stale buffered bytes (real HW
            // discards FIFO contents on DMA restart; otherwise residue from
            // the previous window mixes into the new frame and EOF
            // accounting drifts short).
            self.camera_pattern_offset = 0;
            self.camera_eof_acc = 0;
            self.rx_buffer_len = 0;
        }
        if self.camera_mode && self.rx_active {
            if self.camera_frame_queue_len == 0 {
                // this._cameraFrameRequestCallback check — fn pointer; skipped
            }
            self.try_deliver_camera_frame(ctx);
        }
        if self.rx_active && self.is_dma_enabled() {
            self.process_rx(ctx);
        }
    }

    // JS stopInDMA (line 782)
    pub fn stop_in_dma(&mut self) {}

    // JS restartInDMA (lines 783-785)
    pub fn restart_in_dma(&mut self) {
        if self.inlink_dscr != 0 {
            // this.dmaIn.start(this.inlinkDscr) — needs ctx
        }
    }

    // JS scheduleTXProcessing (lines 786-788) — this.txClockEvent.schedule(1e4)
    pub fn schedule_tx_processing(&self) {
        if self.tx_active {
            // Native clock event: 10us at 80MHz APB = 800 ticks
            crate::peripherals::common::spi_syscon::schedule_global(800, EventTag::I2sTx { idx: self.index });
        }
    }

    // JS scheduleRXProcessing (lines 789-791) — this.rxClockEvent.schedule(1e4)
    pub fn schedule_rx_processing(&self) {
        // Native clock event: 10us at 80MHz APB = 800 ticks
        crate::peripherals::common::spi_syscon::schedule_global(800, EventTag::I2sRx { idx: self.index });
    }

    // JS this.onTxData(chunk) — dispatched only when a host hook is attached
    // (native_i2s_set_tx_hook), matching the JS default no-op callback.
    fn emit_tx_data(&self, chunk: &[u32]) {
        if !self.tx_data_hook {
            return;
        }
        static mut I2S_TX_SCRATCH: [u32; 8] = [0u32; 8];
        unsafe {
            let len = core::cmp::min(chunk.len(), 8);
            for (i, v) in chunk[..len].iter().enumerate() {
                I2S_TX_SCRATCH[i] = *v;
            }
            crate::peripherals::common::ffi::js_i2s_tx_data(
                self.index,
                I2S_TX_SCRATCH.as_ptr() as u32,
                len as u32,
            );
        }
    }

    // JS processTX (lines 792-814)
    pub fn process_tx(&mut self, ctx: &mut CpuContext) {
        if !self.tx_active {
            return;
        }
        let mut cpu_buf = [0u32; 8];
        let tmp_val = self.dma_out.pointer;
        // this.dmaOut.copyOut(cpuVal, cpuVal.length) — stubbed, returns 0
        let idx_val = self.dma_out.copy_out(&mut cpu_buf);
        let clock_event = self.dma_out.pointer;
        if idx_val > 0 {
            // this.onTxData(cpuVal.subarray(0, idxVal))
            self.emit_tx_data(&cpu_buf[..idx_val as usize]);
            if tmp_val != clock_event {
                self.out_eof_des_addr = tmp_val;
                self.set_interrupt_i2s(I2C_REG37, ctx);
                if clock_event == 0 {
                    self.set_interrupt_i2s(I2C_REG38, ctx);
                    return;
                }
            }
            self.outlink_dscr = clock_event;
        } else {
            if clock_event != 0 {
                return;
            }
            self.set_interrupt_i2s(I2C_REG37, ctx);
            self.set_interrupt_i2s(I2C_REG38, ctx);
            return;
        }
        self.schedule_tx_processing();
    }

    // JS processRX (lines 815-853)
    pub fn process_rx(&mut self, ctx: &mut CpuContext) {
        if !self.rx_active {
            return;
        }
        // ADC continuous (DIG/DMA) mode: the DIG controller routes samples to
        // I2S0's RX DMA (SYSCON.saradc_ctrl.data_to_i2s, SYSCON+0x10 bit26).
        // Synthesize TYPE1 frames ({channel[15:12], data[11:0]}) from the DIG
        // pattern tables + host analog inputs so the driver's DMA descriptors
        // fill and its RX_EOF ISR feeds the read ringbuffer.
        if self.index == 0 && adc_digi_to_i2s() {
            let mut slot = unsafe { ADC_DIGI_SLOT };
            while self.rx_buffer_len < 64 {
                let s0 = adc_digi_sample(slot);
                slot += 1;
                let s1 = adc_digi_sample(slot);
                slot += 1;
                self.rx_buffer[self.rx_buffer_len] = (s0 as u32) | ((s1 as u32) << 16);
                self.rx_buffer_len += 1;
            }
            unsafe { ADC_DIGI_SLOT = slot; }
        }
        // Virtual camera: synthesize the test-pattern ramp directly here so
        // the feed is self-sustaining (try_deliver_camera_frame only runs on
        // start edges; without this the buffer drains once and EOFs stop).
        // Bounded by camera_frame_bytes (finite sensor frame; 0 = unbounded).
        if self.camera_mode {
            while self.rx_buffer_len < 4096
                && (self.camera_frame_bytes == 0
                    || self.camera_pattern_offset < self.camera_frame_bytes as u64)
            {
                let o = self.camera_pattern_offset;
                // SM_0A00_0B00 packing: one sensor byte per DMA element in
                // sample1 (bits 23:16); see try_deliver_camera_frame.
                let w = ((o & 0xFF) as u32) << 16;
                self.rx_buffer[self.rx_buffer_len] = w;
                self.rx_buffer_len += 1;
                self.camera_pattern_offset = o.wrapping_add(1);
            }
        }
        let chunk_len = if self.rx_buffer_len >= 8 { 8 } else { self.rx_buffer_len };
        if chunk_len == 0 {
            if self.camera_mode && self.pending_camera_frame_len > 0 {
                // In JS: this.cpu.clock.createEvent(() => { ... }).schedule(5e4)
                // Requires clock event creation — not implemented yet
                self.schedule_rx_processing();
            } else {
                self.schedule_rx_processing();
            }
            return;
        }
        let mut cpu_chunk = [0u32; 8];
        for i in 0..chunk_len {
            cpu_chunk[i] = self.rx_buffer[i];
        }
        // this.dmaIn.copyIn(cpuVal) — stubbed, returns 0
        let idx_val = self.dma_in.copy_in(&cpu_chunk[..chunk_len]);
        let clock_event = self.dma_in.pointer;
        // Descriptors completed by this drain. HW fires in_suc_eof per
        // COMPLETED descriptor — the frame-final one completes without any
        // pointer advance (no data follows), so completion is the trigger;
        // gating on the advance starves exactly one EOF per frame.
        let (completed, completed_addr) = self.dma_in.take_completed();
        if idx_val > 0 {
            // Drop consumed words (flushRX parity — otherwise the same chunk
            // is re-sent to DMA forever).
            if (idx_val as usize) <= self.rx_buffer_len {
                for i in idx_val as usize..self.rx_buffer_len {
                    self.rx_buffer[i - idx_val as usize] = self.rx_buffer[i];
                }
                self.rx_buffer_len -= idx_val as usize;
            }
            // Camera EOF accounting counts DMA elements (words) drained, not
            // bytes: rx_eof_num is in elements (half_buffer_size /
            // sizeof(dma_elem_t)). in_suc_eof fires per threshold, which is
            // exactly one driver ping-pong half — not per descriptor.
            if self.camera_mode {
                self.camera_eof_acc =
                    self.camera_eof_acc.wrapping_add(idx_val);
            }
            if self.camera_mode {
                // HW-parity EOF: in_suc_eof fires per rx_eof_num DMA
                // elements (one ping-pong half). The driver copies that half
                // and its filter extracts one sensor byte per element, so the
                // frame closes with byte-exact contents.
                let eof_num = self.base.read_register(36);
                let eof_num = if eof_num == 0 { 256 } else { eof_num };
                while self.camera_eof_acc >= eof_num {
                    self.camera_eof_acc -= eof_num;
                    self.in_eof_des_addr = if completed_addr != 0 {
                        completed_addr
                    } else {
                        clock_event
                    };
                    self.set_interrupt_i2s(I2C_REG36, ctx);
                }
                if clock_event == 0 {
                    return;
                }
            } else if completed > 0 {
                self.in_eof_des_addr = completed_addr;
                self.set_interrupt_i2s(I2C_REG36, ctx);
                if clock_event == 0 {
                    return;
                }
            }
            self.inlink_dscr = clock_event;
        }
        self.schedule_rx_processing();
    }

    // JS setInterrupt (lines 854-856)
    pub fn set_interrupt_i2s(&mut self, cpu_val: u32, ctx: &mut CpuContext) {
        self.int_raw |= cpu_val;
        self.update_interrupts(ctx);
    }

    // JS updateInterrupts (lines 857-859)
    pub fn update_interrupts(&mut self, ctx: &mut CpuContext) {
        ctx.interrupt(self.irq, self.int_status() != 0);
    }

    // JS getRxChannelMode (lines 860-863)
    pub fn get_rx_channel_mode(&self) -> u32 {
        if let Some(ref field) = self.config.f.rx_chan_mod {
            self.base.read_field(field)
        } else {
            0
        }
    }

    // JS getTxChannelMode (lines 864-867)
    pub fn get_tx_channel_mode(&self) -> u32 {
        if let Some(ref field) = self.config.f.tx_chan_mod {
            self.base.read_field(field)
        } else {
            0
        }
    }

    // JS feedRxData (lines 868-916)
    pub fn feed_rx_data(&mut self, cpu_val: &[u32], ctx: &mut CpuContext) {
        let tmp_val = self.get_rx_channel_mode();
        let idx_val = self.get_tx_channel_mode();
        let clock_event: &[u32];
        let mut expanded_buf: [u32; 8192] = [0u32; 8192];
        let expanded_len: usize;
        if idx_val != 0 {
            let max_len = core::cmp::min(cpu_val.len(), 4096);
            for idx in 0..max_len {
                let ch_left = cpu_val[idx] & 0xFFFF;
                let ch_right = (cpu_val[idx] >> 16) & 0xFFFF;
                expanded_buf[2 * idx] = (ch_left << 16) | ch_left;
                expanded_buf[2 * idx + 1] = (ch_right << 16) | ch_right;
            }
            expanded_len = 2 * max_len;
            clock_event = &expanded_buf[..expanded_len];
        } else {
            clock_event = cpu_val;
        }
        if tmp_val == 0 {
            for i in 0..clock_event.len() {
                if self.rx_buffer_len < 65536 {
                    self.rx_buffer[self.rx_buffer_len] = clock_event[i];
                    self.rx_buffer_len += 1;
                }
            }
        } else if tmp_val == 1 {
            let is_esp32 = ctx.chip_name == "esp32";
            let mut i = 0;
            while i + 1 < clock_event.len() {
                let ch_left = (clock_event[i] >> 16) & 0xFFFF;
                let ch_right = if i + 1 < clock_event.len() {
                    (clock_event[i + 1] >> 16) & 0xFFFF
                } else {
                    0
                };
                if self.rx_buffer_len < 65536 {
                    self.rx_buffer[self.rx_buffer_len] = if is_esp32 {
                        (ch_left << 16) | ch_right
                    } else {
                        (ch_right << 16) | ch_left
                    };
                    self.rx_buffer_len += 1;
                }
                i += 2;
            }
        } else if tmp_val == 2 {
            let is_esp32 = ctx.chip_name == "esp32";
            let mut i = 0;
            while i + 1 < clock_event.len() {
                let ch_left = clock_event[i] & 0xFFFF;
                let ch_right = if i + 1 < clock_event.len() {
                    clock_event[i + 1] & 0xFFFF
                } else {
                    0
                };
                if self.rx_buffer_len < 65536 {
                    self.rx_buffer[self.rx_buffer_len] = if is_esp32 {
                        (ch_left << 16) | ch_right
                    } else {
                        (ch_right << 16) | ch_left
                    };
                    self.rx_buffer_len += 1;
                }
                i += 2;
            }
        } else {
            for i in 0..clock_event.len() {
                if self.rx_buffer_len < 65536 {
                    self.rx_buffer[self.rx_buffer_len] = clock_event[i];
                    self.rx_buffer_len += 1;
                }
            }
        }
        // Trim to max 65536 entries (JS: splice from front)
        if self.rx_buffer_len > 65536 {
            let excess = self.rx_buffer_len - 65536;
            for i in 0..(self.rx_buffer_len - excess) {
                self.rx_buffer[i] = self.rx_buffer[i + excess];
            }
            self.rx_buffer_len = 65536;
        }
    }

    // JS dmaSetIn (lines 933-937)
    pub fn dma_set_in(&mut self, cpu_val: u32, _tmp_val: u32, ctx: &mut CpuContext) {
        self.dma_in_link = cpu_val;
        // this.dmaInChannel = tmpVal — stored but not used in this translation
        if cpu_val != 0 {
            self.dma_in.start(cpu_val, ctx);
        }
    }

    // JS dmaSetOut (lines 938-942)
    pub fn dma_set_out(&mut self, cpu_val: u32, _tmp_val: u32, ctx: &mut CpuContext) {
        self.dma_out_link = cpu_val;
        // this.dmaOutChannel = tmpVal — stored but not used in this translation
        if cpu_val != 0 {
            self.dma_out.start(cpu_val, ctx);
        }
    }
}

impl MmioPeripheral for I2sPeripheral {
    // JS readUint32 (lines 528-554)
    fn read_u32(&mut self, _ctx: &mut CpuContext, addr: u32) -> u32 {
        self.read_uint32(addr)
    }

    // JS writeUint32 (lines 555-622)
    fn write_u32(&mut self, ctx: &mut CpuContext, addr: u32, val: u32) {
        // Snapshot BEFORE the store: the CONF arm's TX/RX edge detection
        // needs the pre-write value (reading after the store always yields
        // `val`, so no edge could ever fire and RX/TX could never start).
        let pre_write_val = self.base.read_uint32(addr);
        // Line 556: super.writeUint32(cpuVal, tmpVal)
        self.base.write_uint32(addr, val);

        let offset = addr.wrapping_sub(self.base.base_addr);
        let regs = self.config.rmt_channel_register;
        // Copy field values before match to avoid borrow conflicts
        let tx_reset = self.config.f.tx_reset;
        let rx_reset = self.config.f.rx_reset;
        let tx_fifo_reset = self.config.f.tx_fifo_reset;
        let rx_fifo_reset = self.config.f.rx_fifo_reset;
        let tx_start = self.config.f.tx_start;
        let rx_start = self.config.f.rx_start;
        let camera_en = self.config.f.camera_en;
        match offset {
            x if x == regs.int_clr => {
                self.int_raw &= !val;
                self.update_interrupts(ctx);
                return;
            }
            x if x == regs.int_ena => {
                self.int_ena = val;
                self.update_interrupts(ctx);
                return;
            }
            x if x == regs.conf => {
                // Read old value before write
                let old_val = pre_write_val;
                self.base.write_uint32(addr, val);
                if let Some(field) = tx_reset {
                    if val & (1 << field.shift) != 0 {
                        self.reset_tx(ctx);
                    }
                }
                if let Some(field) = rx_reset {
                    if val & (1 << field.shift) != 0 {
                        self.reset_rx(ctx);
                    }
                }
                if let Some(field) = tx_fifo_reset {
                    if val & (1 << field.shift) != 0 {
                        self.reset_tx_fifo();
                    }
                }
                if let Some(field) = rx_fifo_reset {
                    if val & (1 << field.shift) != 0 {
                        self.reset_rx_fifo();
                    }
                }
                let tx_start_bit = tx_start.map(|x| 1 << x.shift).unwrap_or(0);
                let rx_start_bit = rx_start.map(|x| 1 << x.shift).unwrap_or(0);
                let old_tx = (old_val & tx_start_bit) != 0;
                let new_tx = (val & tx_start_bit) != 0;
                let old_rx = (old_val & rx_start_bit) != 0;
                let new_rx = (val & rx_start_bit) != 0;
                if new_tx && !old_tx {
                    self.start_tx(ctx);
                } else if !new_tx && old_tx {
                    self.stop_tx(ctx);
                }
                if new_rx && !old_rx {
                    self.start_rx(ctx);
                } else if !new_rx && old_rx {
                    self.stop_rx(ctx);
                }
                return;
            }
            x if x == regs.fifo_conf => {
                self.base.write_uint32(addr, val);
                return;
            }
            x if x == regs.lc_conf => {
                self.base.write_uint32(addr, val);
                if val & I2C_REG46 != 0 {
                    self.reset_out_dma();
                }
                if val & I2C_REG47 != 0 {
                    self.reset_in_dma();
                }
                return;
            }
            x if x == regs.out_link => {
                self.base.write_uint32(addr, val);
                if val & I2C_REG40 != 0 {
                    self.stop_out_dma();
                }
                if val & I2C_REG41 != 0 {
                    self.start_out_dma(val & I2C_REG39, ctx);
                }
                if val & I2C_REG42 != 0 {
                    self.restart_out_dma();
                }
                return;
            }
            x if x == regs.in_link => {
                self.base.write_uint32(addr, val);
                if val & I2C_REG43 != 0 {
                    self.stop_in_dma();
                }
                if val & I2C_REG44 != 0 {
                    self.start_in_dma(val & I2C_REG39, ctx);
                }
                if val & I2C_REG45 != 0 {
                    self.restart_in_dma();
                }
                return;
            }
            x if x == regs.conf2 => {
                self.base.write_uint32(addr, val);
                if let Some(field) = camera_en {
                    let cam_en = (val & (1 << field.shift)) != 0;
                    if cam_en != self.camera_mode {
                        self.camera_mode = cam_en;
                    }
                }
                return;
            }
            _ => {}
        }
        // Line 621: super.writeUint32(cpuVal, tmpVal)
        self.base.write_uint32(addr, val);
    }

    // JS reset (lines 917-932)
    fn reset(&mut self) {
        self.base.reset();
        self.int_raw = 0;
        self.int_ena = 0;
        self.tx_active = false;
        self.rx_active = false;
        self.rx_buffer_len = 0;
        self.camera_mode = false;
        self.camera_frame_queue_len = 0;
        self.out_eof_des_addr = 0;
        self.in_eof_des_addr = 0;
        self.outlink_dscr = 0;
        self.inlink_dscr = 0;
        // this.txClockEvent.unschedule() — requires EventTag
        // this.rxClockEvent.unschedule() — requires EventTag
    }
}

// ============================================================
// i2cReg48 lookup table — JS lines 944-981
// ============================================================

fn i2c_reg48_lookup(reg_val: u32) -> i32 {
    match reg_val {
        68 => 0, 136 => 1, 64 => 2, 132 => 3,
        72 => 4, 108 => 5, 96 => 6, 100 => 7,
        104 => 8, 84 => 9, 88 => 10, 92 => 11,
        52 => 12, 56 => 13, 48 => 14, 60 => 15,
        76 => 16, 80 => 17, 112 => 18, 116 => 19,
        120 => 20, 124 => 21, 128 => 22, 140 => 23,
        144 => 24, 36 => 25, 40 => 26, 44 => 27,
        28 => 32, 32 => 33, 20 => 34, 24 => 35,
        4 => 36, 8 => 37, 12 => 38, 16 => 39,
        _ => -1,
    }
}

// ============================================================
// IoMuxConfig (JS lines 987-1020)
// ============================================================

#[derive(Clone, Copy)]
pub struct IoMuxConfig {
    pub use_table: bool,
    pub start: u32,
    pub count: u32,
    pub pin_reset_values: Option<&'static [u32]>,
}

// ============================================================
// IoMuxPeripheral (JS lines 987-1020)
// ============================================================
//
// NOTE: The JS accesses this.gpio.pins[argVal] fields:
//   internalPullDown, internalPullUp, muxFunction, inputEnable
// and calls this.gpio.updateGPIO(), this.gpio.muxConfigChanged(pin).
//
// These require GpioPinState to have these extra fields:
//   internal_pull_down: u32, internal_pull_up: u32,
//   mux_function: u32, input_enable: u32
// and a way to notify the GPIO controller of pin config changes
// (e.g., via ctx.gpio_matrix.input_changed(pin_idx)).
//
// The current GpioPinState in types.rs only has input_value.
// The field writes below are commented out until GpioPinState is extended.

pub struct IoMuxPeripheral {
    pub base: PeripheralBase,
    pub config: IoMuxConfig,
}

impl IoMuxPeripheral {
    pub fn new(base_addr: u32, name: &'static str, config: IoMuxConfig) -> Self {
        IoMuxPeripheral {
            base: PeripheralBase::new(base_addr, name),
            config,
        }
    }
}

impl MmioPeripheral for IoMuxPeripheral {
    // JS readUint32 — not overridden, uses default register read
    fn read_u32(&mut self, _ctx: &mut CpuContext, addr: u32) -> u32 {
        self.base.read_uint32(addr)
    }

    // JS writeUint32 (lines 993-1011)
    fn write_u32(&mut self, ctx: &mut CpuContext, addr: u32, val: u32) {
        let use_table = self.config.use_table;
        let start = self.config.start;
        let count = self.config.count;
        let reg_val = addr.wrapping_sub(self.base.base_addr);
        let arg_val = if use_table {
            i2c_reg48_lookup(reg_val)
        } else {
            ((reg_val.wrapping_sub(start)) >> 2) as i32
        };
        if arg_val >= 0 && (arg_val as u32) < count {
            let pin_idx = arg_val as usize;
            if pin_idx < ctx.gpio_pins.len() {
                // JS: cpuVal.internalPullDown = !!(tmpVal & i2cReg49)
                //     cpuVal.internalPullUp = !!(tmpVal & i2cReg50)
                //     cpuVal.muxFunction = (tmpVal >> i2cReg52) & i2cReg53
                //     cpuVal.inputEnable = !!(tmpVal & i2cReg51)
                //     this.gpio.updateGPIO()
                //     this.gpio.muxConfigChanged(cpuVal)}
                //
                // These fields are not yet in GpioPinState.
                // Use ctx.gpio_matrix.input_changed as a notification stub:
                ctx.gpio_matrix.input_changed(arg_val as u32, None);
            }
        }
        self.base.write_uint32(addr, val);
    }

    // JS reset (lines 1012-1019)
    fn reset(&mut self) {
        self.base.reset();
        if let Some(reset_vals) = self.config.pin_reset_values {
            for (e, &reset_val) in reset_vals.iter().enumerate() {
                let pin_addr = self.base.base_addr + self.config.start + 4 * e as u32;
                self.base.write_uint32(pin_addr, reset_val);
            }
        }
    }
}

// ============================================================
// Helper: readFieldValue (from helpers.js)
// ============================================================

fn read_field_value(val: u32, field: &FieldDesc) -> u32 {
    (val >> field.shift) & field.mask
}
