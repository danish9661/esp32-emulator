// Translated from: src/peripherals/common/uart.js
// Do NOT modify the logic — match the JS line-for-line
//
// WASM bridge hooks (only the callback/scheduling plumbing differs from JS):
//  - on_tx is fn(uart_idx, byte) -> js_uart_tx_byte FFI

use crate::peripherals::types::*;
use crate::peripherals::common::peripheral::{PeripheralBase, FieldDesc};

// ============================================================
// Helper functions (JS line 3-8)
// ============================================================

fn read_field_value(val: u32, field: &FieldDesc) -> u32 {
    (val >> field.shift) & field.mask
}

fn clear_field(val: u32, field: &FieldDesc) -> u32 {
    val & !(field.mask << field.shift)
}

// ============================================================
// UartPeripheralType enum (JS line 10-13)
// ============================================================

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum UartPeripheralType {
    Uart = 3,
}

pub const UART_PERIPHERAL_TYPE: u32 = UartPeripheralType::Uart as u32;

// `hc()` helper — same as `clear_field()` (JS line 6-8)
pub fn hc(val: u32, field: &FieldDesc) -> u32 {
    clear_field(val, field)
}

// ============================================================
// GpioSignalDefs (JS line 15-106)
// ============================================================

pub struct GpioSignalDef {
    pub name: &'static str,
    pub peripheral: u32,
    pub index: u32,
    pub signal: &'static str,
}

pub const U0RXD: GpioSignalDef = GpioSignalDef {
    name: "U0RXD",
    peripheral: UART_PERIPHERAL_TYPE,
    index: 0,
    signal: "RX",
};

pub const U0TXD: GpioSignalDef = GpioSignalDef {
    name: "U0TXD",
    peripheral: UART_PERIPHERAL_TYPE,
    index: 0,
    signal: "TX",
};

pub const U0CTS: GpioSignalDef = GpioSignalDef {
    name: "U0CTS",
    peripheral: UART_PERIPHERAL_TYPE,
    index: 0,
    signal: "CTS",
};

pub const U1RXD: GpioSignalDef = GpioSignalDef {
    name: "U1RXD",
    peripheral: UART_PERIPHERAL_TYPE,
    index: 1,
    signal: "RX",
};

pub const U1TXD: GpioSignalDef = GpioSignalDef {
    name: "U1TXD",
    peripheral: UART_PERIPHERAL_TYPE,
    index: 1,
    signal: "TX",
};

pub const U1CTS: GpioSignalDef = GpioSignalDef {
    name: "U1CTS",
    peripheral: UART_PERIPHERAL_TYPE,
    index: 1,
    signal: "CTS",
};

pub const U2RXD: GpioSignalDef = GpioSignalDef {
    name: "U2RXD",
    peripheral: UART_PERIPHERAL_TYPE,
    index: 2,
    signal: "RX",
};

pub const U2TXD: GpioSignalDef = GpioSignalDef {
    name: "U2TXD",
    peripheral: UART_PERIPHERAL_TYPE,
    index: 2,
    signal: "TX",
};

pub const U2CTS: GpioSignalDef = GpioSignalDef {
    name: "U2CTS",
    peripheral: UART_PERIPHERAL_TYPE,
    index: 2,
    signal: "CTS",
};

pub const U3RXD: GpioSignalDef = GpioSignalDef {
    name: "U3RXD",
    peripheral: UART_PERIPHERAL_TYPE,
    index: 3,
    signal: "RX",
};

pub const U3TXD: GpioSignalDef = GpioSignalDef {
    name: "U3TXD",
    peripheral: UART_PERIPHERAL_TYPE,
    index: 3,
    signal: "TX",
};

pub const U3CTS: GpioSignalDef = GpioSignalDef {
    name: "U3CTS",
    peripheral: UART_PERIPHERAL_TYPE,
    index: 3,
    signal: "CTS",
};

pub const U4RXD: GpioSignalDef = GpioSignalDef {
    name: "U4RXD",
    peripheral: UART_PERIPHERAL_TYPE,
    index: 4,
    signal: "RX",
};

pub const U4TXD: GpioSignalDef = GpioSignalDef {
    name: "U4TXD",
    peripheral: UART_PERIPHERAL_TYPE,
    index: 4,
    signal: "TX",
};

pub const U4CTS: GpioSignalDef = GpioSignalDef {
    name: "U4CTS",
    peripheral: UART_PERIPHERAL_TYPE,
    index: 4,
    signal: "CTS",
};

// ============================================================
// Register/Mask Constants (JS line 108-137)
// ============================================================

const FIFO_SIZE: usize = 128;
const REG_FIFO_DATA: u32 = 0;
const REG_INT_RAW: u32 = 4;
const REG_INT_ST: u32 = 8;
const REG_INT_ENA: u32 = 12;
const REG_INT_CLR: u32 = 16;
const REG_CLKDIV: u32 = 20;
const REG_CONF0: u32 = 32;
const REG_CONF1: u32 = 36;
const REG_STATUS: u32 = 28;
const BIT_AUTOBAUD_EN: u32 = 1;

// Fixed-size FIFO for no_std compatibility
pub struct SimpleFifo {
    data: [u8; 128],
    len: usize,
}
impl SimpleFifo {
    pub const fn new() -> Self { SimpleFifo { data: [0; 128], len: 0 } }
    pub fn push(&mut self, byte: u8) { if self.len < 128 { self.data[self.len] = byte; self.len += 1; } }
    pub fn len(&self) -> usize { self.len }
    pub fn is_empty(&self) -> bool { self.len == 0 }
    pub fn clear(&mut self) { self.len = 0; }
    pub fn remove(&mut self, index: usize) -> u8 {
        if index < self.len {
            let val = self.data[index];
            self.data.copy_within(index + 1..self.len, index);
            self.len -= 1;
            val
        } else { 0 }
    }
}

pub struct SmallI32Vec {
    data: [i32; 4],
    len: usize,
}
impl SmallI32Vec {
    pub const fn new() -> Self { SmallI32Vec { data: [0; 4], len: 0 } }
    pub fn push(&mut self, val: i32) { if self.len < 4 { self.data[self.len] = val; self.len += 1; } }
    pub fn is_empty(&self) -> bool { self.len == 0 }
    pub fn first(&self) -> i32 { if self.len > 0 { self.data[0] } else { -1 } }
}

pub struct SmallFnVec {
    data: [Option<fn()>; 4],
    len: usize,
}
impl SmallFnVec {
    pub const fn new() -> Self { SmallFnVec { data: [None; 4], len: 0 } }
    pub fn push(&mut self, f: fn()) { if self.len < 4 { self.data[self.len] = Some(f); self.len += 1; } }
}
const SHIFT_BIT_NUM: u32 = 2;
const MASK_BIT_NUM: u32 = 3;
const BIT_CONF_FLAG_4: u32 = 4;
const BIT_TICK_REF_ON: u32 = 0x8000000;
const MASK_RX_FIFO_CNT: u32 = 255;
const SHIFT_RX_FIFO_CNT: u32 = 0;
const MASK_TX_FIFO_CNT: u32 = 255;
const SHIFT_TX_FIFO_CNT: u32 = 16;
const MASK_TX_STATE: u32 = 15;
const SHIFT_TX_STATE: u32 = 24;
const MASK_AT_CHAR: u32 = 255;
const SHIFT_AT_CHAR: u32 = 0;
const MASK_AT_NUM: u32 = 255;
const SHIFT_AT_NUM: u32 = 8;
const BIT_AT_CMD_DET: u32 = 262144;
const BIT_TX_DONE: u32 = 16384;
const BIT_RX_TIMEOUT: u32 = 256;
const BIT_TX_EMPTY_INT: u32 = 2;
const BIT_RX_FULL_INT: u32 = 1;

// ============================================================
// TxStateMachine (JS line 139-156)
// ============================================================

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum TxStateMachine {
    TxIdle = 0,
    TxStrt = 1,
    TxDat0 = 2,
    TxDat1 = 3,
    TxDat2 = 4,
    TxDat3 = 5,
    TxDat4 = 6,
    TxDat5 = 7,
    TxDat6 = 8,
    TxDat7 = 9,
    TxPrty = 10,
    TxStp1 = 11,
    TxStp2 = 12,
    TxDl0 = 13,
    TxDl1 = 14,
}

// ============================================================
// Config types (matching JS constructor `regVal` parameter)
// ============================================================

#[derive(Clone, Copy)]
pub struct UartRegisterMap {
    pub autobaud: i32,
    pub reg_update: i32,
    pub id: i32,
    pub at_cmd_char: i32,
    pub mem_rx_status: i32,
    pub rxd_cnt: i32,
    pub lowpulse: i32,
    pub highpulse: i32,
    pub negpulse: i32,
    pub pospulse: i32,
    pub rx_filt: i32,
    pub clk_conf: i32,
}

#[derive(Clone, Copy)]
pub struct UartFieldMap {
    pub rxfifo_rst: FieldDesc,
    pub txfifo_rst: FieldDesc,
    pub loopback: FieldDesc,
    pub tx_flow_en: FieldDesc,
    pub irda_en: FieldDesc,
    pub irda_tx_en: FieldDesc,
    pub lowpulse_min_cnt: FieldDesc,
    pub highpulse_min_cnt: FieldDesc,
    pub negedge_min_cnt: FieldDesc,
    pub posedge_min_cnt: FieldDesc,
    pub rx_tout_thrhd: FieldDesc,
    pub rxfifo_full_thrhd: FieldDesc,
    pub txfifo_empty_thrhd: FieldDesc,
    pub autobaud_en: Option<FieldDesc>,
    pub sclk_sel: Option<FieldDesc>,
    pub sclk_div_num: Option<FieldDesc>,
    pub glitch_filt: Option<FieldDesc>,
    pub glitch_filt_en: Option<FieldDesc>,
}

#[derive(Clone, Copy)]
pub struct UartConfig {
    pub has_tx_state: bool,
    pub tout_multiply: bool,
    pub reg_map: UartRegisterMap,
    pub fields: UartFieldMap,
    pub clock_source_frequency: Option<u32>,
}

// ============================================================
// UartController (JS class line 158-576)
// ============================================================

pub struct UartController {
    pub base: PeripheralBase,
    pub index: u32,
    pub irq: u32,
    pub config: UartConfig,
    pub rx_fifo: SimpleFifo,
    pub tx_fifo: SimpleFifo,
    pub rx_full_threshold: u32,
    pub tx_empty_threshold: u32,
    pub tx_state: TxStateMachine,
    pub data_received: bool,
    pub baud_rate_val: u32,
    pub rx_pin: i32,
    pub tx_pins: SmallI32Vec,
    pub at_char: u32,
    pub at_num: u32,
    pub at_counter: u32,
    pub loopback_byte: i32,
    pub autobaud_enabled: bool,
    pub autobaud_rate: u32,
    pub glitch_filter_enabled: bool,
    pub glitch_filter_threshold: u32,
    pub rx_timeout_threshold: u32,
    // WASM bridge: fn(uart_idx, byte) — routes through js_uart_tx_byte FFI.
    // JS onTX becomes a native bridge call instead of a JS callback.
    pub on_tx: Option<fn(u32, u32)>,
    pub on_configuration_updated: Option<fn()>,
    pub on_pins_updated: Option<fn()>,
    pub on_auto_baud_updated: Option<fn()>,
    pub on_glitch_filter_updated: Option<fn()>,
    pub gpio_listeners: SmallFnVec,
}

impl UartController {
    // JS constructor (line 158-236)
    pub fn new(ctx: &mut CpuContext, base_addr: u32, name: &'static str, index: u32, irq: u32, config: UartConfig) -> Self {
        let base = PeripheralBase::new(base_addr, name);
        let mut uart = UartController {
            base,
            index,
            irq,
            config,
            rx_fifo: SimpleFifo::new(),
            tx_fifo: SimpleFifo::new(),
            rx_full_threshold: 0,
            tx_empty_threshold: 0,
            rx_timeout_threshold: 10,
            tx_state: TxStateMachine::TxIdle,
            data_received: false,
            baud_rate_val: 0,
            rx_pin: -1,
            tx_pins: SmallI32Vec::new(),
            at_char: 0,
            at_num: 0,
            at_counter: 0,
            loopback_byte: -1,
            autobaud_enabled: false,
            autobaud_rate: 115200,
            glitch_filter_enabled: false,
            glitch_filter_threshold: 0,
            on_tx: None,
            on_configuration_updated: None,
            on_pins_updated: None,
            on_auto_baud_updated: None,
            on_glitch_filter_updated: None,
            gpio_listeners: SmallFnVec::new(),
        };
        // GPIO function listeners — stored for integration to connect
        uart.gpio_listeners.push(|| {});
        uart.gpio_listeners.push(|| {});

        // CTS listener and clock source frequency listener handled by integration layer
        uart
    }

    // JS get baudRate() (line 237-239)
    pub fn baud_rate(&self) -> u32 {
        self.baud_rate_val
    }

    // JS get rxPin() (line 240-242)
    pub fn rx_pin(&self) -> i32 {
        self.rx_pin
    }

    // JS get txPin() (line 243-245)
    pub fn tx_pin(&self) -> i32 {
        self.tx_pins.first()
    }

    // JS get txPins() (line 246-248)
    pub fn tx_pins(&self) -> &[i32] {
        &self.tx_pins.data[..self.tx_pins.len]
    }

    // JS get intStatus() (line 249-251)
    pub fn int_status(&self) -> u32 {
        self.read_register(REG_INT_RAW) & self.read_register(REG_INT_ENA)
    }

    // JS checkInterrupt() (line 252-259)
    pub fn check_interrupt(&mut self, ctx: &mut CpuContext) {
        let mut raw = self.read_register(REG_INT_RAW) & !(BIT_RX_FULL_INT | BIT_TX_EMPTY_INT);
        // JS uses `>=`, but the real hardware raises RX FULL only when the FIFO
        // count EXCEEDS the threshold. With the JS form, an empty FIFO + threshold
        // 0 asserts FULL forever (interrupt storm) — `>` is hardware-correct.
        if self.rx_fifo.len() as u32 > self.rx_full_threshold {
            raw |= BIT_RX_FULL_INT;
        }
        if self.tx_fifo.len() as u32 <= self.tx_empty_threshold {
            raw |= BIT_TX_EMPTY_INT;
        }
        self.write_register(REG_INT_RAW, raw);
        ctx.interrupt(self.irq, self.int_status() != 0);
    }

    // JS get clock() (line 261-278)
    pub fn clock_frequency(&self, ctx: &CpuContext) -> u32 {
        if let Some(freq) = self.config.clock_source_frequency {
            return freq;
        }
        if let Some(sclk_sel) = &self.config.fields.sclk_sel {
            match self.read_field(sclk_sel) {
                1 => ctx.clocks.apb.frequency,
                2 => ctx.clocks.rc_fast.frequency,
                3 => ctx.clocks.xtal.frequency,
                _ => ctx.clocks.apb.frequency,
            }
        } else {
            if ctx.clocks.ref_tick.frequency != 0 {
                if self.read_register(REG_CONF0) & BIT_TICK_REF_ON != 0 {
                    ctx.clocks.apb.frequency
                } else {
                    ctx.clocks.ref_tick.frequency
                }
            } else {
                ctx.clocks.apb.frequency
            }
        }
    }

    // JS get clockForAutoBaud() (line 280-283)
    pub fn clock_for_autobaud_frequency(&self, ctx: &CpuContext) -> u32 {
        if ctx.clocks.ref_tick.frequency != 0 {
            ctx.clocks.apb.frequency
        } else {
            self.clock_frequency(ctx)
        }
    }

    // JS get clockDiv() (line 284-288)
    pub fn clock_div(&self) -> u32 {
        if self.config.clock_source_frequency.is_some() {
            return 1;
        }
        if let Some(sclk_div_num) = &self.config.fields.sclk_div_num {
            1 + self.read_field(sclk_div_num)
        } else {
            1
        }
    }

    // JS get glitchFilterThresholdNs() (line 289-294)
    pub fn glitch_filter_threshold_ns(&self, ctx: &CpuContext) -> u32 {
        if !self.glitch_filter_enabled || self.glitch_filter_threshold == 0 {
            return 0;
        }
        let clock_freq = self.clock_for_autobaud_frequency(ctx) / self.clock_div();
        if clock_freq == 0 { return 0; }
        ((1_000_000_000u64 * self.glitch_filter_threshold as u64 + (clock_freq as u64 / 2)) / clock_freq as u64) as u32
    }

    // JS get glitchFilterEnabled() (line 295-297)
    pub fn glitch_filter_enabled(&self) -> bool {
        self.glitch_filter_enabled
    }

    // JS updateGlitchFilter(cpuVal, tmpVal) (line 298-306)
    pub fn update_glitch_filter(&mut self, enabled: bool, threshold: Option<u32>) {
        let mut changed = false;
        if enabled != self.glitch_filter_enabled {
            self.glitch_filter_enabled = enabled;
            changed = true;
        }
        if let Some(threshold) = threshold {
            if threshold != self.glitch_filter_threshold {
                self.glitch_filter_threshold = threshold;
                changed = true;
            }
        }
        if changed {
            if let Some(cb) = self.on_glitch_filter_updated {
                cb();
            }
        }
    }

    // JS updateBaudRate() (line 307-315)
    pub fn update_baud_rate(&mut self, ctx: &CpuContext) {
        let clkdiv = self.read_register(REG_CLKDIV);
        let int_div = clkdiv & 1048575;
        let frac_div = (clkdiv >> 20) & 15;
        let clock_freq = self.clock_frequency(ctx);
        let div = self.clock_div() as f64 * (int_div as f64 + frac_div as f64 / 16.0);
        self.baud_rate_val = if div != 0.0 {
            (clock_freq as f64 / div) as u32
        } else {
            0
        };
        if let Some(cb) = self.on_configuration_updated {
            cb();
        }
    }

    // JS get txBusy() (line 316-318)
    pub fn tx_busy(&self) -> bool {
        self.tx_state != TxStateMachine::TxIdle
    }

    // JS get irdaTxSuppressed() (line 319-324)
    pub fn irda_tx_suppressed(&self) -> bool {
        self.read_field(&self.config.fields.irda_en) != 0
            && self.read_field(&self.config.fields.irda_tx_en) == 0
    }

    // JS txUpdated() (line 325-346)
    pub fn tx_updated(&mut self, ctx: &mut CpuContext) {
        if !self.tx_busy() && !self.tx_fifo.is_empty() {
            if self.irda_tx_suppressed() {
                self.tx_fifo.clear();
                self.loopback_byte = -1;
                self.set_register_bits(REG_INT_RAW, BIT_TX_DONE);
                self.check_interrupt(ctx);
                return;
            }
            if self.read_field(&self.config.fields.tx_flow_en) != 0 && self.cts_asserted() {
                return;
            }
            let byte = self.tx_fifo.remove(0);
            self.clear_register_bits(REG_INT_RAW, BIT_TX_DONE);
            if self.read_field(&self.config.fields.loopback) != 0 {
                self.loopback_byte = byte as i32;
            } else {
                self.loopback_byte = -1;
            }
            self.tx_state = TxStateMachine::TxStrt;
            if let Some(cb) = self.on_tx {
                cb(self.index, byte as u32);
            }
            self.tx_complete(ctx);
        } else {
            if self.tx_fifo.is_empty() {
                self.set_register_bits(REG_INT_RAW, BIT_TX_DONE);
            }
        }
        self.check_interrupt(ctx);
    }

    // JS check: this._ctsInput?.value  — placeholder, integration should set
    pub fn cts_asserted(&self) -> bool {
        false
    }

    // JS txComplete() (line 347-354)
    pub fn tx_complete(&mut self, ctx: &mut CpuContext) {
        if self.read_field(&self.config.fields.loopback) != 0 && self.loopback_byte >= 0 {
            self.feed_byte(ctx, self.loopback_byte as u8, true);
            self.loopback_byte = -1;
        }
        self.tx_state = TxStateMachine::TxIdle;
        self.tx_updated(ctx);
    }

    // JS startDetected() (line 355-357)
    pub fn start_detected(&mut self, ctx: &mut CpuContext) {
        ctx.unschedule_event(EventTag::UartTimeout { uart_idx: self.index });
    }

    // WASM bridge: JS clock event fired the RX timeout (JS rxTimeout handler, line 174-177)
    pub fn rx_timeout_fired(&mut self, ctx: &mut CpuContext) {
        self.set_register_bits(REG_INT_RAW, BIT_RX_TIMEOUT);
        self.check_interrupt(ctx);
    }

    // JS get irdaRxSuppressed() (line 358-363)
    pub fn irda_rx_suppressed(&self) -> bool {
        self.read_field(&self.config.fields.irda_en) != 0
            && self.read_field(&self.config.fields.irda_tx_en) != 0
    }

    // JS feedByte(cpuVal, tmpVal = false) (line 364-388)
    pub fn feed_byte(&mut self, ctx: &mut CpuContext, byte: u8, force: bool) -> bool {
        if !force
            && (self.read_field(&self.config.fields.loopback) != 0 || self.irda_rx_suppressed())
        {
            return false;
        }
        if byte as u32 == self.at_char {
            self.at_counter += 1;
            if self.at_counter == self.at_num {
                self.at_counter = 0;
                self.set_register_bits(REG_INT_RAW, BIT_AT_CMD_DET);
                self.check_interrupt(ctx);
            }
        } else {
            self.at_counter = 0;
        }
        if self.rx_fifo.len() != FIFO_SIZE {
            self.rx_fifo.push(byte);
            self.data_received = true;
            self.check_interrupt(ctx);
            if self.baud_rate_val != 0 {
                let delay = (self.rx_timeout_threshold as u64 * 1_000_000_000) / self.baud_rate_val as u64;
                // Native clock event: rx_timeout_fired fires on the shared EventQueue
                // (pumped by native_process_events). 80MHz APB ticks.
                ctx.schedule_event((delay * 80) / 1000, EventTag::UartTimeout { uart_idx: self.index });
            }
            return true;
        }
        false
    }

    // JS get bitNum() (line 389-400)
    pub fn bit_num(&self) -> u32 {
        match (self.read_register(REG_CONF0) >> SHIFT_BIT_NUM) & MASK_BIT_NUM {
            0 => 5,
            1 => 6,
            2 => 7,
            _ => 8,
        }
    }

    // JS updateAutoBaud(cpuVal) (line 401-404)
    pub fn update_auto_baud(&mut self, enabled: bool) {
        if enabled != self.autobaud_enabled {
            self.autobaud_enabled = enabled;
            if let Some(cb) = self.on_auto_baud_updated {
                cb();
            }
        }
    }

    // JS readUint8(cpuVal) (line 405-407)
    pub fn read_uint8(&mut self, ctx: &mut CpuContext, addr: u32) -> u8 {
        (self.read_uint32(ctx, addr) & 255) as u8
    }

    // JS readUint16(cpuVal) (line 408-410)
    pub fn read_uint16(&mut self, ctx: &mut CpuContext, addr: u32) -> u16 {
        (self.read_uint32(ctx, addr) & 65535) as u16
    }

    // JS readUint32(cpuVal) (line 411-456)
    pub fn read_uint32(&mut self, ctx: &mut CpuContext, addr: u32) -> u32 {
        let offset = addr - self.base.base_addr;
        let reg_map = &self.config.reg_map;
        let fields = &self.config.fields;
        match offset {
            REG_FIFO_DATA => {
                if self.rx_fifo.is_empty() {
                    return 238;
                }
                let byte = self.rx_fifo.remove(0);
                self.check_interrupt(ctx);
                return byte as u32;
            }
            REG_INT_ST => return self.int_status(),
            _ if offset as i32 == self.config.reg_map.lowpulse || offset as i32 == self.config.reg_map.highpulse => {
                let clock_freq = self.clock_for_autobaud_frequency(ctx) as u64;
                let val = ((2 * clock_freq) / self.autobaud_rate as u64 - 2) / 2;
                return core::cmp::min(self.config.fields.lowpulse_min_cnt.mask, val as u32);
            }
            _ if offset as i32 == self.config.reg_map.negpulse || offset as i32 == self.config.reg_map.pospulse => {
                let clock_freq = self.clock_for_autobaud_frequency(ctx) as u64;
                let val = (2 * clock_freq) / self.autobaud_rate as u64 - 1;
                return core::cmp::min(self.config.fields.negedge_min_cnt.mask, val as u32);
            }
            _ if offset as i32 == self.config.reg_map.rxd_cnt => {
                if self.data_received {
                    return 255;
                }
            }
            REG_STATUS => {
                let mut val = ((self.tx_fifo.len() as u32 & MASK_TX_FIFO_CNT) << SHIFT_TX_FIFO_CNT)
                    | ((self.rx_fifo.len() as u32 & MASK_RX_FIFO_CNT) << SHIFT_RX_FIFO_CNT);
                if self.config.has_tx_state {
                    val |= (self.tx_state as u32 & MASK_TX_STATE) << SHIFT_TX_STATE;
                }
                return val;
            }
            _ if offset as i32 == self.config.reg_map.mem_rx_status => {
                let len = self.rx_fifo.len() as u32;
                return if len < FIFO_SIZE as u32 { len << 13 } else { 0 };
            }
            _ if offset as i32 == self.config.reg_map.id => {
                return 0x7fffffff & self.read_register(offset);
            }
            _ if offset as i32 == self.config.reg_map.reg_update => {
                return 0;
            }
            _ => {}
        }
        self.base.read_uint32(addr)
    }

    // JS writeUint8(cpuVal, tmpVal) (line 457-464)
    pub fn write_uint8(&mut self, ctx: &mut CpuContext, addr: u32, val: u8) {
        if addr - self.base.base_addr == REG_FIFO_DATA {
            if self.tx_fifo.len() < FIFO_SIZE {
                self.tx_fifo.push(val);
                self.tx_updated(ctx);
            }
            return;
        }
        self.base.write_uint8(addr, val);
    }

    // JS writeUint32(cpuVal, tmpVal) (line 465-555)
    pub fn write_uint32(&mut self, ctx: &mut CpuContext, addr: u32, val: u32) {
        let offset = addr - self.base.base_addr;

        if offset as i32 == self.config.fields.rx_tout_thrhd.reg as i32 {
            let raw = (val >> self.config.fields.rx_tout_thrhd.shift) & self.config.fields.rx_tout_thrhd.mask;
            if self.config.tout_multiply {
                if self.read_register(REG_CONF0) & BIT_TICK_REF_ON != 0 {
                    self.rx_timeout_threshold = 8 * raw;
                } else {
                    self.rx_timeout_threshold = raw >> 3;
                }
            } else {
                self.rx_timeout_threshold = raw;
            }
        }

        let mut tmp_val = val;
        if offset as i32 == self.config.fields.rxfifo_rst.reg as i32 && read_field_value(tmp_val, &self.config.fields.rxfifo_rst) != 0 {
            self.rx_fifo.clear();
            self.data_received = false;
            self.check_interrupt(ctx);
            tmp_val = hc(tmp_val, &self.config.fields.rxfifo_rst);
        }
        if offset as i32 == self.config.fields.txfifo_rst.reg as i32 && read_field_value(tmp_val, &self.config.fields.txfifo_rst) != 0 {
            self.tx_fifo.clear();
            self.tx_updated(ctx);
            tmp_val = hc(tmp_val, &self.config.fields.txfifo_rst);
        }
        if let Some(autobaud_en) = &self.config.fields.autobaud_en {
            if offset as i32 == autobaud_en.reg as i32 {
                self.update_auto_baud(read_field_value(tmp_val, autobaud_en) != 0);
            }
        }

        let sclk_sel_reg = if let Some(sclk_sel) = &self.config.fields.sclk_sel { sclk_sel.reg as i32 } else { -1 };

        match offset {
            REG_FIFO_DATA => {
                if self.tx_fifo.len() < FIFO_SIZE {
                    self.tx_fifo.push((tmp_val & 255) as u8);
                    self.tx_updated(ctx);
                }
                return;
            }
            REG_INT_ENA => {
                self.write_register(REG_INT_ENA, tmp_val);
                // Native clock event: int-recheck fires on the shared EventQueue
                // (JS parity: 100ns delay at 80MHz APB = 8 ticks).
                ctx.schedule_event(8, EventTag::UartIntCheck { uart_idx: self.index });
                return;
            }
            REG_INT_CLR => {
                self.clear_register_bits(REG_INT_RAW, tmp_val);
                self.check_interrupt(ctx);
                ctx.interrupt(self.irq, self.int_status() != 0);
                return;
            }
            REG_CONF0 => {
                self.base.write_uint32(addr, tmp_val);
                self.update_baud_rate(ctx);
            }
            REG_CONF1 => {
                self.rx_full_threshold = read_field_value(tmp_val, &self.config.fields.rxfifo_full_thrhd);
                self.tx_empty_threshold = read_field_value(tmp_val, &self.config.fields.txfifo_empty_thrhd);
            }
            REG_CLKDIV => {
                self.base.write_uint32(addr, tmp_val);
                self.update_baud_rate(ctx);
                return;
            }
            _ if offset as i32 == sclk_sel_reg => {
                self.base.write_uint32(addr, tmp_val);
                self.update_baud_rate(ctx);
                return;
            }
            _ if offset as i32 == self.config.reg_map.rx_filt => {
                if self.config.reg_map.rx_filt >= 0 {
                    let glitch_val = self.config.fields.glitch_filt
                        .map(|f| read_field_value(tmp_val, &f))
                        .unwrap_or(0);
                    let glitch_en = self.config.fields.glitch_filt_en
                        .map(|f| read_field_value(tmp_val, &f) != 0)
                        .unwrap_or(glitch_val > 0);
                    self.update_glitch_filter(glitch_en, Some(glitch_val));
                    self.base.write_uint32(addr, tmp_val);
                    return;
                }
            }
            _ if offset as i32 == self.config.reg_map.at_cmd_char => {
                self.at_char = (tmp_val >> SHIFT_AT_CHAR) & MASK_AT_CHAR;
                self.at_num = (tmp_val >> SHIFT_AT_NUM) & MASK_AT_NUM;
                self.at_counter = 0;
                self.clear_register_bits(REG_INT_RAW, BIT_AT_CMD_DET);
                self.check_interrupt(ctx);
                return;
            }
            _ if offset as i32 == self.config.reg_map.autobaud => {
                self.update_auto_baud((tmp_val & BIT_AUTOBAUD_EN) != 0);
                if let Some(glitch_filt) = &self.config.fields.glitch_filt {
                    let glitch_val = read_field_value(tmp_val, glitch_filt);
                    self.update_glitch_filter((tmp_val & BIT_AUTOBAUD_EN) != 0, Some(glitch_val));
                }
            }
            _ => {}
        }
        self.base.write_uint32(addr, tmp_val);
    }

    // JS reset() (line 556-573)
    pub fn reset(&mut self) {
        self.base.reset();
        self.write_register(
            REG_CONF0,
            BIT_TICK_REF_ON | (3 << SHIFT_BIT_NUM) | (1 << BIT_CONF_FLAG_4),
        );
        self.write_register(REG_CLKDIV, 694);
        let tmp_val = self.read_register(REG_CONF1);
        self.rx_full_threshold = read_field_value(tmp_val, &self.config.fields.rxfifo_full_thrhd);
        self.tx_empty_threshold = read_field_value(tmp_val, &self.config.fields.txfifo_empty_thrhd);
        self.rx_fifo.clear();
        self.tx_fifo.clear();
        self.at_counter = 0;
    }

    // Handle events fired by the simulation (EventTag dispatch)
    pub fn handle_event(&mut self, ctx: &mut CpuContext, tag: EventTag) {
        match tag {
            EventTag::UartTimeout { uart_idx } if uart_idx == self.index => {
                self.set_register_bits(REG_INT_RAW, BIT_RX_TIMEOUT);
                self.check_interrupt(ctx);
            }
            EventTag::UartIntCheck { uart_idx } if uart_idx == self.index => {
                self.check_interrupt(ctx);
            }
            _ => {}
        }
    }
}

// Forwarding methods to PeripheralBase
impl UartController {
    pub fn read_register(&self, reg: u32) -> u32 { self.base.read_register(reg) }
    pub fn write_register(&mut self, reg: u32, val: u32) { self.base.write_register(reg, val); }
    fn read_field(&self, f: &FieldDesc) -> u32 { self.base.read_field(f) }
    fn write_field(&mut self, f: &FieldDesc, val: u32) { self.base.write_field(f, val); }
    fn set_register_bits(&mut self, reg: u32, bits: u32) { self.base.set_register_bits(reg, bits); }
    fn clear_register_bits(&mut self, reg: u32, bits: u32) { self.base.clear_register_bits(reg, bits); }
}

// Implement MmioPeripheral for UartController
impl crate::peripherals::types::MmioPeripheral for UartController {
    fn read_u32(&mut self, ctx: &mut CpuContext, addr: u32) -> u32 {
        self.read_uint32(ctx, addr)
    }

    fn write_u32(&mut self, ctx: &mut CpuContext, addr: u32, val: u32) {
        self.write_uint32(ctx, addr, val);
    }

    fn reset(&mut self) {
        self.reset();
    }
}
