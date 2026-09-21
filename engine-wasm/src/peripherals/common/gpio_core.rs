// Translated from: src/peripherals/common/gpio-core.js
// Do NOT modify the logic — match the JS line-for-line
// NOTE: no_std — no Vec, String, HashMap, HashSet, format! — fixed arrays + Copy types

use crate::peripherals::types::*;

// ── Constants for fixed-size arrays ──
pub const MAX_GPIO_PINS: usize = 64;
pub const REG_FILE_SIZE: usize = 256; // 1024 bytes of register space
pub const MAX_LISTENERS_PER_PIN: usize = 8;
pub const MAX_FUNC_LISTENERS: usize = 16;
pub const MAX_MUX_FUNCS: usize = 8;
pub const MAX_STRAP_PINS: usize = 16;

// ── Helper: readFieldValue ──
fn read_field_value(val: u32, field: &FieldDef) -> u32 {
    (val >> field.shift) & field.mask
}

// ── Helper: convert SignalInfo (from GpioMatrix) to GpioSignalInfo ──
fn signal_info_to_gpio_signal_info(sig: &SignalInfo, default_index: u32) -> GpioSignalInfo {
    GpioSignalInfo {
        name_tag: 1,
        name_id: sig.signal,
        peripheral: PeripheralType::Other,
        index: default_index,
        signal_id: sig.signal,
    }
}

// ── Enums (JS IIFE enums) ──
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PinState {
    Low = 0,
    High = 1,
    Input = 2,
    PullUp = 3,
    PullDown = 4,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SignalDirection {
    Input = 1,
    Output = 2,
    Both = 3,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum InterruptTrigger {
    Disable = 0,
    RisingEdge = 1,
    FallingEdge = 2,
    Edge = 3,
    LowLevel = 4,
    HighLevel = 5,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PeripheralType {
    Gpio = 0,
    Spi = 1,
    I2c = 2,
    Uart = 3,
    Ledc = 4,
    Pcnt = 5,
    Twai = 6,
    Other = 7,
    None = 8,
}

// ── Constants ──
pub const CPU_TARGET_PRO: u32 = 1;
pub const CPU_TARGET_APP: u32 = 2;
pub const GPIO_BOTH_DIR: u32 = 3;
pub const GPIO_OUT_REG_OFFSET: u32 = 4;
pub const GPIO_OUT_W1TS_REG_OFFSET: u32 = 8;
pub const GPIO_OUT_W1TC_REG_OFFSET: u32 = 12;
pub const GPIO_PIN_PAD_DRIVER_MASK: u32 = 4;
pub const FUNC_OUT_SEL_OEN_INV_MASK: u32 = 2048;
pub const FUNC_OUT_SEL_SEL_MASK: u32 = 1024;
pub const FUNC_OUT_SEL_INV_MASK: u32 = 512;
pub const FUNC_OUT_SEL_FUNC_SHIFT: u32 = 0;
pub const FUNC_OUT_SEL_FUNC_MASK: u32 = 511;
pub const GPIO_PIN_INT_TYPE_SHIFT: u32 = 7;
pub const GPIO_PIN_INT_TYPE_MASK: u32 = 7;
pub const GPIO_PIN_INT_ENA_APP_MASK: u32 = 8192;
pub const GPIO_PIN_NMI_INT_ENA_APP_MASK: u32 = 16384;
pub const GPIO_PIN_INT_ENA_PRO_DUALCORE_MASK: u32 = 32768;
pub const GPIO_PIN_NMI_INT_ENA_PRO_DUALCORE_MASK: u32 = 65536;
pub const GPIO_PIN_INT_ENA_PRO_SINGLECORE_MASK: u32 = 8192;
pub const GPIO_PIN_NMI_INT_ENA_PRO_SINGLECORE_MASK: u32 = 16384;

pub struct GpioSignalDefs;
impl GpioSignalDefs {
    pub const GPIO: u32 = 0xFFFF_FFFF;
}

// ── Helper types (all Copy, no heap) ──
#[derive(Clone, Copy, Debug)]
pub struct FieldDef {
    pub shift: u32,
    pub mask: u32,
}

/// Signal info without heap strings.
/// name_tag: 0 = auto "GPIO{N}" (name_id = gpio_num), 1 = named signal (name_id = signal id)
/// signal_id: signal identifier
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct GpioSignalInfo {
    pub name_tag: u32,
    pub name_id: u32,
    pub peripheral: PeripheralType,
    pub index: u32,
    pub signal_id: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MuxFnEntry {
    Gpio,
    Named(u32),
    Signal(GpioSignalInfo),
}

#[derive(Clone, Copy, Debug)]
pub struct MuxConfig {
    pub pin_num: u32,
    pub fn_entries: [MuxFnEntry; MAX_MUX_FUNCS],
    pub fn_count: u32,
}

#[derive(Clone, Copy, Debug)]
pub struct FunctionListener {
    pub signal: GpioSignalInfo,
    pub pins: [u32; MAX_GPIO_PINS],
    pub pin_count: u32,
    pub listener: u32,
    pub direction: u32,
}

// ── GpioConfig (no heap, uses slices) ──
#[derive(Clone, Copy, Debug)]
pub struct GpioConfig<'a> {
    pub gpio_count: u32,
    pub out_function_max: u32,
    pub strap_value: u32,
    pub strap_boot: u32,
    pub strap_pins: &'a [i32],
    pub irq: u32,
    pub nmi_irq: u32,
    pub reg_enable: i32,
    pub reg_out: i32,
    pub reg_out1: i32,
    pub reg_enable1: i32,
    pub reg_out1_w1ts: i32,
    pub reg_out1_w1tc: i32,
    pub reg_enable_w1ts: i32,
    pub reg_enable_w1tc: i32,
    pub reg_enable1_w1ts: i32,
    pub reg_enable1_w1tc: i32,
    pub reg_status_w1ts: i32,
    pub reg_status_w1tc: i32,
    pub reg_status1_w1ts: i32,
    pub reg_status1_w1tc: i32,
    pub reg_strap: i32,
    pub reg_syscon_tick_count_mask: i32,
    pub reg_in1: i32,
    pub reg_status: i32,
    pub reg_status1: i32,
    pub reg_acpu_int: i32,
    pub reg_acpu_nmi_int: i32,
    pub reg_pcpu_int: i32,
    pub reg_pcpu_nmi_int: i32,
    pub reg_acpu_int1: i32,
    pub reg_acpu_nmi_int1: i32,
    pub reg_pcpu_int1: i32,
    pub reg_pcpu_nmi_int1: i32,
    pub reg_intr_0: i32,
    pub reg_intr1_0: i32,
    pub reg_intr_1: i32,
    pub reg_intr1_1: i32,
    pub reg_pin0: i32,
    pub reg_func0_out_sel_cfg: i32,
    pub reg_funcn_in_sel_cfg: i32,
    pub funcn_in_sel_cfg_first: i32,
    pub funcn_in_sel_cfg_count: u32,
    pub funcn_in_sel_cfg_start_index: u32,
    pub f_in_sel: FieldDef,
    pub f_in_inv_sel: FieldDef,
    pub f_sel: FieldDef,
    pub iomux_table: &'a [MuxConfig],
}

// ── GpioPin (no Vec, fixed-size arrays) ──
#[derive(Clone, Copy)]
pub struct GpioPin {
    pub gpio_num: u32,
    pub mux_config_idx: i32,
    pub listeners: [u32; MAX_LISTENERS_PER_PIN],
    pub listener_count: u32,
    pub int_type: u32,
    pub output_enable_value: u32,
    pub state: u32,
    pub open_drain: u32,
    pub internal_pull_down: u32,
    pub internal_pull_up: u32,
    pub mux_function: u32,
    pub rtc_pull_down: u32,
    pub rtc_pull_up: u32,
    pub input_enable: u32,
    pub input_value_raw: u32,
    pub gpio_output: u32,
    pub gpio_output_enable: u32,
    pub matrix_enable: u32,
    pub matrix_select_output_enable: u32,
    pub matrix_output_enable_invert: u32,
    pub matrix_output_invert: u32,
    pub matrix_output: u32,
    pub matrix_output_enable: u32,
    pub bank: u32,
    pub lp_pin_override: i32,
}

impl GpioPin {
    pub fn new(gpio_num: u32, mux_index: i32) -> Self {
        let bank = if gpio_num < 32 { 0 } else { 1 };
        GpioPin {
            gpio_num,
            mux_config_idx: mux_index,
            listeners: [0; MAX_LISTENERS_PER_PIN],
            listener_count: 0,
            int_type: InterruptTrigger::Disable as u32,
            output_enable_value: 0,
            state: PinState::Input as u32,
            open_drain: 0,
            internal_pull_down: 0,
            internal_pull_up: 0,
            mux_function: 0,
            rtc_pull_down: 0,
            rtc_pull_up: 0,
            input_enable: 0,
            input_value_raw: 0,
            gpio_output: 0,
            gpio_output_enable: 0,
            matrix_enable: 0,
            matrix_select_output_enable: 0,
            matrix_output_enable_invert: 0,
            matrix_output_invert: 0,
            matrix_output: 0,
            matrix_output_enable: 0,
            bank,
            lp_pin_override: -1,
        }
    }

    pub fn output_enable(&self) -> u32 {
        self.output_enable_value
    }

    pub fn reset(&mut self) {
        self.state = PinState::Input as u32;
        self.internal_pull_down = 0;
        self.internal_pull_up = 0;
        self.rtc_pull_down = 0;
        self.rtc_pull_up = 0;
        self.gpio_output = 0;
        self.gpio_output_enable = 0;
        self.open_drain = 0;
        self.matrix_enable = 0;
        self.matrix_select_output_enable = 0;
        self.matrix_output_enable_invert = 0;
        self.matrix_output_invert = 0;
        self.matrix_output = 0;
        self.matrix_output_enable = 0;
    }

    pub fn input_value(&self, input_values: &[u32; 2]) -> u32 {
        let bit = self.gpio_num % 32;
        (input_values[self.bank as usize] >> bit) & 1
    }

    pub fn internal_update_state(&mut self, state: u32) {
        let _old_state = self.state;
        self.state = state;
        for i in 0..(self.listener_count as usize) {
            let _listener_id = self.listeners[i];
            // Integration: dispatch listener(_listener_id, state, old_state)
            // JS: each listener(state, oldState) was called via function reference
            // In no_std, listeners are u32 IDs requiring a dispatch table.
        }
    }

    pub fn update(&mut self) {
        if self.lp_pin_override >= 0 {
            return;
        }
        let val = if self.internal_pull_up != 0 || self.rtc_pull_up != 0 {
            PinState::PullUp as u32
        } else if self.internal_pull_down != 0 || self.rtc_pull_down != 0 {
            PinState::PullDown as u32
        } else {
            PinState::Input as u32
        };
        let tmp_val = if self.open_drain != 0 { val } else { PinState::High as u32 };
        // JS: (this.matrixEnable ? this.matrixOutput : this.gpioOutput) ? tmpVal : PinState.Low
        let cond = if self.matrix_enable != 0 { self.matrix_output } else { self.gpio_output };
        let idx_val = if cond != 0 { tmp_val } else { PinState::Low as u32 };
        let new_oe = if self.matrix_enable != 0 && self.matrix_select_output_enable != 0 {
            self.matrix_output_enable
        } else {
            self.gpio_output_enable
        };
        let actual_state = if new_oe != 0 { idx_val } else { val };
        self.output_enable_value = new_oe;
        if self.state != actual_state {
            self.internal_update_state(actual_state);
        }
    }

    pub fn internal_set_interrupt(&mut self, int_type: u32, int_status: &mut [u32; 2], input_values: &[u32; 2]) {
        let bank = self.bank as usize;
        let bit = self.gpio_num % 32;
        self.int_type = int_type;
        let mut triggered = 0u32;
        let mut recheck = 0u32;
        // JS uses this.inputValue (getter from inputValues register), not inputValueRaw
        let current_input = self.input_value(input_values);
        match int_type {
            t if t == InterruptTrigger::LowLevel as u32 => {
                triggered = if current_input == 0 { 1 } else { 0 };
                recheck = 1;
            }
            t if t == InterruptTrigger::HighLevel as u32 => {
                triggered = current_input;
                recheck = 1;
            }
            _ => {}
        }
        if recheck != 0 {
            if triggered != 0 {
                int_status[bank] |= 1 << bit;
            } else {
                int_status[bank] &= !(1 << bit);
            }
        }
    }

    pub fn add_listener(&mut self, listener: u32) -> i32 {
        if (self.listener_count as usize) < MAX_LISTENERS_PER_PIN {
            let idx = self.listener_count;
            self.listeners[idx as usize] = listener;
            self.listener_count += 1;
            idx as i32
        } else {
            -1
        }
    }

    pub fn get_mux_signal(&self, iomux_table: &[MuxConfig]) -> GpioSignalInfo {
        let mux = if self.mux_config_idx >= 0 {
            iomux_table.get(self.mux_config_idx as usize)
        } else {
            None
        };
        let fn_entry = mux.and_then(|m| {
            if (self.mux_function as usize) < (m.fn_count as usize) {
                Some(&m.fn_entries[self.mux_function as usize])
            } else {
                None
            }
        });
        match fn_entry {
            None | Some(MuxFnEntry::Gpio) => GpioSignalInfo {
                name_tag: 0,
                name_id: self.gpio_num,
                peripheral: PeripheralType::None,
                index: self.gpio_num,
                signal_id: self.gpio_num,
            },
            Some(MuxFnEntry::Named(id)) => GpioSignalInfo {
                name_tag: 1,
                name_id: *id,
                peripheral: PeripheralType::Other,
                index: self.gpio_num,
                signal_id: *id,
            },
            Some(MuxFnEntry::Signal(info)) => *info,
        }
    }

    pub fn get_signal_info(&self, ctx: &CpuContext, iomux_table: &[MuxConfig], pins: &[GpioPin]) -> GpioSignalInfo {
        if self.lp_pin_override >= 0 {
            let lp = self.lp_pin_override as usize;
            return pins[lp].get_signal_info(ctx, iomux_table, pins);
        }
        let mux = if self.mux_config_idx >= 0 {
            iomux_table.get(self.mux_config_idx as usize)
        } else {
            None
        };
        let fn_entry = mux.and_then(|m| {
            if (self.mux_function as usize) < (m.fn_count as usize) {
                Some(&m.fn_entries[self.mux_function as usize])
            } else {
                None
            }
        });
        let mut val: Option<MuxFnEntry> = match fn_entry {
            None => None,
            Some(e) => Some(*e),
        };
        if let Some(MuxFnEntry::Gpio) = val {
            if self.matrix_enable != 0 {
                let out_info = ctx.gpio_matrix.pin_signal_out_info(self.gpio_num);
                if !out_info.is_empty() {
                    val = Some(MuxFnEntry::Named(GpioSignalDefs::GPIO));
                }
            }
        }
        match val {
            None => GpioSignalInfo {
                name_tag: 0,
                name_id: self.gpio_num,
                peripheral: PeripheralType::None,
                index: self.gpio_num,
                signal_id: self.gpio_num,
            },
            Some(MuxFnEntry::Gpio) => {
                let in_info = ctx.gpio_matrix.pin_signal_in_info(self.gpio_num);
                if !in_info.is_empty() {
                    GpioSignalInfo {
                        name_tag: 0,
                        name_id: 0,
                        peripheral: PeripheralType::Gpio,
                        index: self.gpio_num,
                        signal_id: 0,
                    }
                } else {
                    GpioSignalInfo {
                        name_tag: 0,
                        name_id: self.gpio_num,
                        peripheral: PeripheralType::Gpio,
                        index: self.gpio_num,
                        signal_id: self.gpio_num,
                    }
                }
            }
            Some(MuxFnEntry::Named(id)) => GpioSignalInfo {
                name_tag: 1,
                name_id: id,
                peripheral: PeripheralType::Other,
                index: self.gpio_num,
                signal_id: id,
            },
            Some(MuxFnEntry::Signal(info)) => info,
        }
    }

    pub fn active_signals(&self, direction: u32, ctx: &CpuContext, iomux_table: &[MuxConfig], out: &mut [GpioSignalInfo; MAX_GPIO_PINS]) -> u32 {
        let mut count = 0u32;
        let mux = if self.mux_config_idx >= 0 {
            iomux_table.get(self.mux_config_idx as usize)
        } else {
            None
        };
        let fn_entry = mux.and_then(|m| {
            if (self.mux_function as usize) < (m.fn_count as usize) {
                Some(&m.fn_entries[self.mux_function as usize])
            } else {
                None
            }
        });
        if let Some(MuxFnEntry::Gpio) = fn_entry {
            let out_info = ctx.gpio_matrix.pin_signal_out_info(self.gpio_num);
            if !out_info.is_empty() && self.matrix_enable != 0 && (direction & SignalDirection::Output as u32) != 0 {
                out[count as usize] = signal_info_to_gpio_signal_info(&out_info[0], self.gpio_num);
                count += 1;
            }
            if (direction & SignalDirection::Input as u32) != 0 {
                let in_info = ctx.gpio_matrix.pin_signal_in_info(self.gpio_num);
                for sig in in_info {
                    out[count as usize] = signal_info_to_gpio_signal_info(sig, self.gpio_num);
                    count += 1;
                }
            }
        } else if let Some(entry) = fn_entry {
            match entry {
                MuxFnEntry::Named(id) => {
                    out[count as usize] = GpioSignalInfo {
                        name_tag: 1,
                        name_id: *id,
                        peripheral: PeripheralType::Other,
                        index: self.gpio_num,
                        signal_id: *id,
                    };
                    count += 1;
                }
                MuxFnEntry::Signal(info) => {
                    out[count as usize] = *info;
                    count += 1;
                }
                MuxFnEntry::Gpio => {}
            }
        }
        count
    }

    pub fn matrix_out(&mut self, oe: u32, val: u32) {
        self.matrix_output = if self.matrix_output_invert != 0 { if val != 0 { 0 } else { 1 } } else { val };
        self.matrix_output_enable = if self.matrix_output_enable_invert != 0 { if oe != 0 { 0 } else { 1 } } else { oe };
        self.update();
    }
}

// ── GpioController ──
pub struct GpioController<'a> {
    pub base_addr: u32,
    pub config: &'a GpioConfig<'a>,
    pub pins: [GpioPin; MAX_GPIO_PINS],
    pub pin_count: u32,
    pub strap_value: u32,
    pub function_listeners: [FunctionListener; MAX_FUNC_LISTENERS],
    pub function_listener_count: u32,
    pub input_values: [u32; 2],
    pub int_status: [u32; 2],
    pub pro_int_enable: [u32; 2],
    pub pro_nmi_int_enable: [u32; 2],
    pub app_int_enable: [u32; 2],
    pub app_nmi_int_enable: [u32; 2],
    pub registers: [u32; REG_FILE_SIZE],
}

impl<'a> GpioController<'a> {
    pub fn new(base_addr: u32, config: &'a GpioConfig<'a>) -> Self {
        let gpio_count = config.gpio_count as usize;
        let mut pins = [GpioPin::new(0, -1); MAX_GPIO_PINS];
        for i in 0..gpio_count.min(MAX_GPIO_PINS) {
            let mux_idx = config.iomux_table.iter().position(|p| p.pin_num == i as u32);
            pins[i] = GpioPin::new(i as u32, mux_idx.map(|x| x as i32).unwrap_or(-1));
        }
        GpioController {
            base_addr,
            config,
            pins,
            pin_count: gpio_count as u32,
            strap_value: config.strap_value,
            function_listeners: [FunctionListener {
                signal: GpioSignalInfo { name_tag: 0, name_id: 0, peripheral: PeripheralType::None, index: 0, signal_id: 0 },
                pins: [0; MAX_GPIO_PINS],
                pin_count: 0,
                listener: 0,
                direction: 0,
            }; MAX_FUNC_LISTENERS],
            function_listener_count: 0,
            input_values: [0, 0],
            int_status: [0, 0],
            pro_int_enable: [0, 0],
            pro_nmi_int_enable: [0, 0],
            app_int_enable: [0, 0],
            app_nmi_int_enable: [0, 0],
            registers: [0; REG_FILE_SIZE],
        }
    }

    fn reg_offset(offset: u32) -> usize {
        (offset as usize >> 2) & (REG_FILE_SIZE - 1)
    }

    fn read_register(&self, offset: u32) -> u32 {
        self.registers[Self::reg_offset(offset)]
    }

    fn write_register(&mut self, offset: u32, val: u32) {
        let idx = Self::reg_offset(offset);
        self.registers[idx] = val;
    }

    fn set_register_bits(&mut self, offset: u32, mask: u32) {
        let idx = Self::reg_offset(offset);
        self.registers[idx] |= mask;
    }

    fn clear_register_bits(&mut self, offset: u32, mask: u32) {
        let idx = Self::reg_offset(offset);
        self.registers[idx] &= !mask;
    }

    fn get_mux_config(&self, pin: &GpioPin) -> Option<&MuxConfig> {
        if pin.mux_config_idx >= 0 {
            self.config.iomux_table.get(pin.mux_config_idx as usize)
        } else {
            None
        }
    }

    pub fn update_gpio(&mut self) {
        let cfg = self.config;
        let gpio_count = (cfg.gpio_count as usize).min(MAX_GPIO_PINS);

        let enable = self.read_register(cfg.reg_enable as u32);
        let enable1 = if cfg.reg_enable1 >= 0 {
            self.read_register(cfg.reg_enable1 as u32)
        } else {
            0
        };
        let out = self.read_register(cfg.reg_out as u32);
        let out1 = if cfg.reg_out1 >= 0 {
            self.read_register(cfg.reg_out1 as u32)
        } else {
            0
        };

        let reg_val = [enable, enable1];
        let arg_val = [out, out1];

        for i in 0..gpio_count {
            let bank_idx = i >> 5;
            let mask = 1u32 << (i & 31);
            let pin_reg = self.read_register(cfg.reg_pin0 as u32 + 4 * i as u32);
            let pin = &mut self.pins[i];
            pin.open_drain = (pin_reg & GPIO_PIN_PAD_DRIVER_MASK) >> 2;
            pin.gpio_output = (arg_val[bank_idx] & mask) >> (i & 31);
            pin.gpio_output_enable = (reg_val[bank_idx] & mask) >> (i & 31);
            pin.update();
        }
    }

    pub fn set_boot(&mut self, val: u32) {
        if val != 0 {
            self.strap_value |= self.config.strap_boot;
        } else {
            self.strap_value &= !self.config.strap_boot;
        }
    }

    pub fn capture_strap(&mut self) {
        let strap_pins = self.config.strap_pins;
        for t in 0..strap_pins.len() {
            let idx = strap_pins[t];
            if idx < 0 {
                continue;
            }
            if self.pins[idx as usize].input_value(&self.input_values) != 0 {
                self.strap_value |= 1 << t;
            } else {
                self.strap_value &= !(1 << t);
            }
        }
    }

    pub fn set_pin_input_value(&mut self, ctx: &mut CpuContext, pin_idx: usize, new_level: u32) {
        self.pins[pin_idx].input_value_raw = new_level;

        let lp_idx = self.pins[pin_idx].lp_pin_override;
        if lp_idx >= 0 {
            self.set_pin_input_value(ctx, lp_idx as usize, new_level);
        }

        let input_enable = self.pins[pin_idx].input_enable;
        if input_enable == 0 {
            return;
        }

        let pin = self.pins[pin_idx].gpio_num;
        let bank = self.pins[pin_idx].bank as usize;
        let bit = pin % 32;
        let current = (self.input_values[bank] >> bit) & 1;
        if current == new_level {
            return;
        }

        if new_level != 0 {
            self.input_values[bank] |= 1 << bit;
        } else {
            self.input_values[bank] &= !(1 << bit);
        }

        let int_type = self.pins[pin_idx].int_type;
        let mut reg_val = 0u32;
        let mut arg_val = 0u32;
        match int_type {
            t if t == InterruptTrigger::FallingEdge as u32 => {
                reg_val = if new_level == 0 { 1 } else { 0 };
            }
            t if t == InterruptTrigger::RisingEdge as u32 => {
                reg_val = new_level;
            }
            t if t == InterruptTrigger::Edge as u32 => {
                reg_val = 1;
            }
            t if t == InterruptTrigger::LowLevel as u32 => {
                reg_val = if new_level == 0 { 1 } else { 0 };
                arg_val = new_level;
            }
            t if t == InterruptTrigger::HighLevel as u32 => {
                reg_val = new_level;
                arg_val = if new_level == 0 { 1 } else { 0 };
            }
            _ => {}
        }

        if reg_val != 0 {
            self.int_status[bank] |= 1 << bit;
            self.interrupts_updated(ctx);
        }
        if arg_val != 0 {
            self.int_status[bank] &= !(1 << bit);
            self.interrupts_updated(ctx);
        }

        ctx.gpio_matrix.input_changed(self.pins[pin_idx].gpio_num, Some(new_level != 0));
    }

    pub fn internal_lp_pin_override(&mut self, ctx: &mut CpuContext, pin_idx: usize, other: i32) {
        if self.pins[pin_idx].lp_pin_override == other {
            return;
        }
        if self.pins[pin_idx].lp_pin_override >= 0 {
            let prev = self.pins[pin_idx].lp_pin_override as usize;
            self.pins[prev].listener_count = 0;
        }
        self.pins[pin_idx].lp_pin_override = other;
        if other >= 0 {
            let lp = other as usize;
            let count = self.pins[pin_idx].listener_count;
            self.pins[lp].listener_count = count;
            for j in 0..(count as usize) {
                self.pins[lp].listeners[j] = self.pins[pin_idx].listeners[j];
            }
            let iv = self.pins[pin_idx].input_value(&self.input_values);
            self.set_pin_input_value(ctx, lp, iv);
        } else {
            self.pins[pin_idx].update();
        }
    }

    pub fn interrupts_updated(&self, ctx: &mut CpuContext) {
        let status0 = self.int_status[0];
        let status1 = self.int_status[1];
        let app_pending = (status0 & self.app_int_enable[0]) | (status1 & self.app_int_enable[1]);
        let pro_pending = (status0 & self.pro_int_enable[0]) | (status1 & self.pro_int_enable[1]);
        let nmi_pending = (status0 & self.pro_nmi_int_enable[0]) | (status1 & self.pro_nmi_int_enable[1]);
        let has_dual = self.config.reg_acpu_int >= 0 && self.config.reg_pcpu_int >= 0;
        if has_dual {
            ctx.interrupt_target(self.config.irq, pro_pending != 0, CPU_TARGET_PRO);
            ctx.interrupt_target(self.config.irq, app_pending != 0, CPU_TARGET_APP);
            ctx.interrupt_target(self.config.nmi_irq, nmi_pending != 0, CPU_TARGET_PRO);
            ctx.interrupt_target(self.config.nmi_irq, pro_pending != 0, CPU_TARGET_APP);
        } else {
            ctx.interrupt(self.config.irq, pro_pending != 0);
            ctx.interrupt(self.config.nmi_irq, nmi_pending != 0);
        }
    }

    pub fn mux_config_changed(&mut self, ctx: &mut CpuContext, pin_idx: usize) {
        let needs_set = self.pins[pin_idx].input_enable != 0
            && self.pins[pin_idx].input_value(&self.input_values) != self.pins[pin_idx].input_value_raw;
        if needs_set {
            let raw = self.pins[pin_idx].input_value_raw;
            self.set_pin_input_value(ctx, pin_idx, raw);
        }
        self.update_functions(ctx);
    }

    pub fn update_functions(&mut self, ctx: &CpuContext) {
        for ei in 0..(self.function_listener_count as usize) {
            let mut pins = [0u32; MAX_GPIO_PINS];
            let mut pin_count = 0u32;
            for pi in 0..(self.pin_count as usize) {
                let gpio_pin = &self.pins[pi];
                // Compare mux signal (GpioSignalInfo == GpioSignalInfo)
                let mux_sig = gpio_pin.get_mux_signal(self.config.iomux_table);
                if mux_sig == self.function_listeners[ei].signal {
                    pins[pin_count as usize] = gpio_pin.gpio_num;
                    pin_count += 1;
                }
                // Output direction: match matrix out signals against entry.signal
                if (self.function_listeners[ei].direction & SignalDirection::Output as u32) != 0 {
                    let out_info = ctx.gpio_matrix.pin_signal_out_info(gpio_pin.gpio_num);
                    for sig in out_info {
                        let converted = signal_info_to_gpio_signal_info(sig, gpio_pin.gpio_num);
                        if converted == self.function_listeners[ei].signal {
                            pins[pin_count as usize] = gpio_pin.gpio_num;
                            pin_count += 1;
                        }
                    }
                }
                // Input direction: match matrix in signals + mux fn[0] against entry.signal
                if (self.function_listeners[ei].direction & SignalDirection::Input as u32) != 0 {
                    let in_info = ctx.gpio_matrix.pin_signal_in_info(gpio_pin.gpio_num);
                    for sig in in_info {
                        let converted = signal_info_to_gpio_signal_info(sig, gpio_pin.gpio_num);
                        if converted == self.function_listeners[ei].signal {
                            pins[pin_count as usize] = gpio_pin.gpio_num;
                            pin_count += 1;
                        }
                    }
                    if gpio_pin.input_enable != 0 {
                        let mux = self.get_mux_config(gpio_pin);
                        if let Some(ref m) = mux {
                            if m.fn_count > 0 {
                                let sig_info = m.fn_entries[0];
                                // JS: sigInfo && "string" != typeof sigInfo && JSON.stringify(sigInfo) === signalKey && !isInputRouted(sigInfo)
                                match sig_info {
                                    MuxFnEntry::Signal(ref info) => {
                                        if *info == self.function_listeners[ei].signal
                                            && !ctx.gpio_matrix.is_input_routed(info.signal_id)
                                        {
                                            pins[pin_count as usize] = gpio_pin.gpio_num;
                                            pin_count += 1;
                                        }
                                    }
                                    _ => {}
                                }
                            }
                        }
                    }
                }
            }
            // JS: (pins.size !== entry.pins.size || Array.from(pins).some((pin) => !entry.pins.has(pin)))
            let size_changed = pin_count != self.function_listeners[ei].pin_count;
            let mut content_changed = false;
            if !size_changed && pin_count > 0 {
                for j in 0..(self.function_listeners[ei].pin_count as usize) {
                    let old_pin = self.function_listeners[ei].pins[j];
                    let mut found = false;
                    for k in 0..(pin_count as usize) {
                        if pins[k] == old_pin {
                            found = true;
                            break;
                        }
                    }
                    if !found {
                        content_changed = true;
                        break;
                    }
                }
            }
            if size_changed || content_changed {
                let entry = &mut self.function_listeners[ei];
                entry.pins = pins;
                entry.pin_count = pin_count;
                // JS: entry.listener(pins)
                // listener callback stubbed — integration should dispatch entry.listener(entry.listener, pins)
            }
        }
    }

    pub fn add_function_listener(&mut self, signal: GpioSignalInfo, listener: u32, direction: u32) {
        if (self.function_listener_count as usize) < MAX_FUNC_LISTENERS {
            let idx = self.function_listener_count as usize;
            self.function_listeners[idx] = FunctionListener {
                signal,
                pins: [0; MAX_GPIO_PINS],
                pin_count: 0,
                listener,
                direction,
            };
            self.function_listener_count += 1;
        }
    }
}

impl<'a> MmioPeripheral for GpioController<'a> {
    fn read_u32(&mut self, _ctx: &mut CpuContext, addr: u32) -> u32 {
        let offset = addr.wrapping_sub(self.base_addr);
        let r = self.config;
        match offset as i32 {
            x if x == r.reg_strap => self.strap_value,
            x if x == r.reg_syscon_tick_count_mask => self.input_values[0],
            x if x == r.reg_in1 => self.input_values[1],
            x if x == r.reg_status => self.int_status[0],
            x if x == r.reg_status1 => self.int_status[1],
            x if x == r.reg_acpu_int => self.int_status[0] & self.app_int_enable[0],
            x if x == r.reg_acpu_nmi_int => self.int_status[0] & self.app_nmi_int_enable[0],
            x if x == r.reg_pcpu_int => self.int_status[0] & self.pro_int_enable[0],
            x if x == r.reg_pcpu_nmi_int => self.int_status[0] & self.pro_nmi_int_enable[0],
            x if x == r.reg_acpu_int1 => self.int_status[1] & self.app_int_enable[1],
            x if x == r.reg_acpu_nmi_int1 => self.int_status[1] & self.app_nmi_int_enable[1],
            x if x == r.reg_pcpu_int1 => self.int_status[1] & self.pro_int_enable[1],
            x if x == r.reg_pcpu_nmi_int1 => self.int_status[1] & self.pro_nmi_int_enable[1],
            x if x == r.reg_intr_0 => self.int_status[0] & self.pro_int_enable[0],
            x if x == r.reg_intr1_0 => self.int_status[1] & self.pro_int_enable[1],
            x if x == r.reg_intr_1 => self.int_status[0] & self.pro_nmi_int_enable[0],
            x if x == r.reg_intr1_1 => self.int_status[1] & self.pro_nmi_int_enable[1],
            _ => self.read_register(offset),
        }
    }

    fn write_u32(&mut self, ctx: &mut CpuContext, addr: u32, reg_value: u32) {
        let offset = addr.wrapping_sub(self.base_addr);
        let r = self.config;

        match offset as i32 {
            x if x == r.reg_out || x == r.reg_out1 || x == r.reg_enable || x == r.reg_enable1 => {
                self.write_register(offset, reg_value);
                self.update_gpio();
                return;
            }
            x if x == GPIO_OUT_W1TS_REG_OFFSET as i32 => {
                self.set_register_bits(GPIO_OUT_REG_OFFSET, reg_value);
                self.update_gpio();
                return;
            }
            x if x == GPIO_OUT_W1TC_REG_OFFSET as i32 => {
                self.clear_register_bits(GPIO_OUT_REG_OFFSET, reg_value);
                self.update_gpio();
                return;
            }
            x if x == r.reg_out1_w1ts => {
                self.set_register_bits(r.reg_out1 as u32, reg_value);
                self.update_gpio();
                return;
            }
            x if x == r.reg_out1_w1tc => {
                self.clear_register_bits(r.reg_out1 as u32, reg_value);
                self.update_gpio();
                return;
            }
            x if x == r.reg_enable_w1ts => {
                self.set_register_bits(r.reg_enable as u32, reg_value);
                self.update_gpio();
                return;
            }
            x if x == r.reg_enable_w1tc => {
                self.clear_register_bits(r.reg_enable as u32, reg_value);
                self.update_gpio();
                return;
            }
            x if x == r.reg_enable1_w1ts => {
                self.set_register_bits(r.reg_enable1 as u32, reg_value);
                self.update_gpio();
                return;
            }
            x if x == r.reg_enable1_w1tc => {
                self.clear_register_bits(r.reg_enable1 as u32, reg_value);
                self.update_gpio();
                return;
            }
            x if x == r.reg_status_w1ts => {
                self.int_status[0] |= reg_value;
                self.interrupts_updated(ctx);
                return;
            }
            x if x == r.reg_status_w1tc => {
                self.int_status[0] &= !reg_value;
                self.interrupts_updated(ctx);
                return;
            }
            x if x == r.reg_status1_w1ts => {
                self.int_status[1] |= reg_value;
                self.interrupts_updated(ctx);
                return;
            }
            x if x == r.reg_status1_w1tc => {
                self.int_status[1] &= !reg_value;
                self.interrupts_updated(ctx);
                return;
            }
            _ => {}
        }

        // FUNCn_IN_SEL_CFG range — indexed by PERIPHERAL SIGNAL number
        // (n = signal 0..255, NOT gpio: the 256-entry table runs 0x130..0x52F,
        // right up to FUNC0_OUT_SEL_CFG at 0x530). Fields: [5:0] = gpio pad
        // number, [6] = invert, [7] = route enable. (An earlier revision had
        // the polarity backwards — pin as signal and vice versa.)
        let fn_in_first = r.funcn_in_sel_cfg_first as u32;
        let fn_in_count = r.funcn_in_sel_cfg_count;
        if offset >= fn_in_first && offset < fn_in_first + 4 * fn_in_count {
            let sig_idx = ((offset - fn_in_first) >> 2) + r.funcn_in_sel_cfg_start_index;
            let gpio_num = read_field_value(reg_value, &r.f_in_sel);
            let inv_sel = read_field_value(reg_value, &r.f_in_inv_sel) != 0;
            let sel = read_field_value(reg_value, &r.f_sel) != 0;
            let pin_input = if (gpio_num as usize) < (self.pin_count as usize) {
                Some(self.pins[gpio_num as usize].input_value(&self.input_values) != 0)
            } else {
                None
            };
            ctx.gpio_matrix.func_in_select(gpio_num, sel, sig_idx, inv_sel, pin_input);
        }

        // FUNC0_OUT_SEL_CFG range
        let func0_offset = r.reg_func0_out_sel_cfg as u32;
        if offset >= func0_offset && offset < func0_offset + 4 * r.gpio_count {
            let pin = (offset - func0_offset) >> 2;
            let func_sel = (reg_value >> FUNC_OUT_SEL_FUNC_SHIFT) & FUNC_OUT_SEL_FUNC_MASK;
            let pin_obj = &mut self.pins[pin as usize];
            pin_obj.matrix_enable = if func_sel != r.out_function_max { 1 } else { 0 };
            pin_obj.matrix_output_invert = (reg_value & FUNC_OUT_SEL_INV_MASK) >> 9;
            pin_obj.matrix_select_output_enable = (reg_value & FUNC_OUT_SEL_SEL_MASK) >> 10;
            pin_obj.matrix_output_enable_invert = (reg_value & FUNC_OUT_SEL_OEN_INV_MASK) >> 11;
            ctx.gpio_matrix.func_out_select(
                pin,
                func_sel,
                pin_obj.matrix_output_invert != 0,
                pin_obj.matrix_output_enable_invert != 0,
            );
        }

        // PIN0 range
        let pin0_offset = r.reg_pin0 as u32;
        if offset >= pin0_offset && offset < pin0_offset + 4 * r.gpio_count {
            let pin = (offset - pin0_offset) >> 2;
            let core = if pin < 32 { 0usize } else { 1 };
            let bit = pin % 32;
            let has_dual = r.reg_acpu_int >= 0;
            if has_dual {
                if (reg_value & GPIO_PIN_INT_ENA_APP_MASK) != 0 {
                    self.app_int_enable[core] |= 1 << bit;
                } else {
                    self.app_int_enable[core] &= !(1 << bit);
                }
                if (reg_value & GPIO_PIN_NMI_INT_ENA_APP_MASK) != 0 {
                    self.app_nmi_int_enable[core] |= 1 << bit;
                } else {
                    self.app_nmi_int_enable[core] &= !(1 << bit);
                }
            }
            let cfg_val = if has_dual {
                GPIO_PIN_INT_ENA_PRO_DUALCORE_MASK
            } else {
                GPIO_PIN_INT_ENA_PRO_SINGLECORE_MASK
            };
            let nmi_cfg = if has_dual {
                GPIO_PIN_NMI_INT_ENA_PRO_DUALCORE_MASK
            } else {
                GPIO_PIN_NMI_INT_ENA_PRO_SINGLECORE_MASK
            };
            if (reg_value & cfg_val) != 0 {
                self.pro_int_enable[core] |= 1 << bit;
            } else {
                self.pro_int_enable[core] &= !(1 << bit);
            }
            if (reg_value & nmi_cfg) != 0 {
                self.pro_nmi_int_enable[core] |= 1 << bit;
            } else {
                self.pro_nmi_int_enable[core] &= !(1 << bit);
            }
            self.pins[pin as usize].internal_set_interrupt(
                (reg_value >> GPIO_PIN_INT_TYPE_SHIFT) & GPIO_PIN_INT_TYPE_MASK,
                &mut self.int_status,
                &self.input_values,
            );
            self.interrupts_updated(ctx);
        }

        self.write_register(offset, reg_value);
    }

    fn reset(&mut self) {
        for i in 0..(self.pin_count as usize) {
            self.pins[i].reset();
        }
        let cfg = self.config;
        let func0_offset = cfg.reg_func0_out_sel_cfg as u32;
        for i in 0..cfg.gpio_count {
            self.write_register(func0_offset + 4 * i, cfg.out_function_max);
        }
    }
}
