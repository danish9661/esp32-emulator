// Translated from: src/peripherals/common/ledc-pcnt.js
// Do NOT modify the logic — match the JS line-for-line

use crate::peripherals::types::*;
use crate::peripherals::common::peripheral::PeripheralBase;

// ============================================================
// Register constants (match JS let/const declarations)
// ============================================================

pub const LED_REG1: u32 = 0;
pub const LED_REG2: u32 = 0;
pub const LED_REG3: u32 = 3;
pub const LED_REG4: u32 = 4;
pub const LED_REG5: u32 = 8;
pub const LED_REG6: u32 = 4;
pub const LED_REG7: u32 = 1048575;
pub const LED_REG8: u32 = 8;
pub const LED_REG9: u32 = 12;
pub const LED_REG10: u32 = 0;
pub const LED_REG11: u32 = 1023;
pub const LED_REG12: u32 = 0x80000000;
pub const LED_REG13: u32 = 16;
pub const LED_REG14: u32 = 3;
pub const LED_REG15: u32 = 0;
pub const LED_REG16: u32 = 1;
pub const LED_REG17: u32 = 2;
pub const LED_REG18: u32 = 3;
pub const LED_REG19: u32 = 0;
pub const LED_REG20: u32 = 255;
pub const LED_REG21: u32 = 24;
pub const LED_REG22: u32 = 15;
pub const LED_REG23: u32 = 1;
pub const LED_REG24: u32 = 4;
pub const LED_REG25: u32 = 8;
pub const LED_REG26: u32 = 32;
pub const LED_REG27: u32 = 64;
pub const LED_REG28: u32 = 131072;
pub const LED_REG29: u32 = 262144;
pub const LED_REG30: u32 = 524288;
pub const LED_REG31: u32 = 2;
pub const LED_REG32: u32 = 4;
pub const LED_REG33: u32 = 255;
pub const LED_REG34: u32 = 12;
pub const LED_REG35: u32 = 63;
pub const LED_REG36: u32 = 18;
pub const LED_REG37: u32 = 63;
pub const LED_REG38: u32 = 24;
pub const LED_REG39: u32 = 3;
pub const LED_REG40: u32 = 0;
pub const LED_REG41: u32 = 24;
pub const LED_REG42: u32 = 1;
pub const LED_REG43: u32 = 2;
pub const LED_REG44: u32 = 8;
pub const LED_REG45: u32 = 16;
pub const LED_REG46: u32 = 32;
pub const LED_REG47: u32 = 64;
pub const LED_REG48: u32 = 8;
pub const LED_REG49: u32 = 255;
pub const LED_REG50: u32 = 16;
pub const LED_REG51: u32 = 15;
pub const LED_REG52: u32 = 8;

// ============================================================
// PcntRegister constants (JS enum PcntRegister)
// ============================================================

pub const PCNT_CONF0: u32 = 0;
pub const PCNT_CONF1: u32 = 1;
pub const PCNT_CONF2: u32 = 2;
pub const PCNT_CNT: u32 = 3;
pub const PCNT_STATUS: u32 = 4;

// ============================================================
// LedcTimerConfig — JS: { CONF_REG, VALUE_REG, F, hsTimer, lsTimer }
// ============================================================

#[derive(Clone, Copy)]
pub struct LedcTimerConfig {
    pub conf_reg: u32,
    pub value_reg: u32,
    pub hs_timer: bool,
    pub ls_timer: bool,
}

// ============================================================
// LedcTimerCounter — JS Timer32Counter as used by LEDC
// (f64 prescaler for CLK_DIV/256; no listener mechanism;
//  the LEDC peripheral coordinates alarm updates directly)
// ============================================================

#[derive(Clone, Copy)]
pub struct LedcTimerCounter {
    pub base_value: u32,
    pub base_ticks: f64,
    pub bit_count: u32,
    pub mask_val: u32,
    pub top_value: u32,
    pub prescaler_val: f64,
    pub enabled: bool,
    pub timer_mode: TimerMode,
}

impl LedcTimerCounter {
    pub fn new() -> Self {
        LedcTimerCounter {
            base_value: 0,
            base_ticks: 0.0,
            bit_count: 32,
            mask_val: 0xFFFF_FFFF,
            top_value: 0xFFFF_FFFF,
            prescaler_val: 1.0,
            enabled: true,
            timer_mode: TimerMode::Increment,
        }
    }

    // JS get rawCounter
    pub fn raw_counter(&self, clock_ticks: f64) -> u32 {
        if self.prescaler_val == 0.0 || !self.enabled {
            return self.base_value;
        }
        let is_zigzag = self.timer_mode == TimerMode::ZigZag;
        let delta_ticks = clock_ticks - self.base_ticks;
        let arg_val = delta_ticks / self.prescaler_val;
        let register_type = if is_zigzag {
            2 * self.top_value
        } else {
            self.top_value + 1
        };
        let mut h_val = if self.timer_mode == TimerMode::Decrement {
            let register_type_f = register_type as f64;
            let remainder = arg_val % register_type_f;
            self.base_value as f64 + (register_type_f - remainder)
        } else {
            self.base_value as f64 + arg_val
        };
        // libm::round — nearest integer, half away from zero (matches JS Math.round for positive duty values)
        h_val = if h_val >= 0.0 {
            (h_val + 0.5) as u64 as f64
        } else {
            -((0.5 - h_val) as u64 as f64)
        };
        if self.top_value != 0xFFFF_FFFF {
            let register_type_f = register_type as f64;
            h_val = h_val % register_type_f;
        }
        h_val as u32
    }

    // JS get counter
    pub fn counter(&self, clock_ticks: f64) -> u32 {
        let mut val = self.raw_counter(clock_ticks);
        if self.timer_mode == TimerMode::ZigZag && val > self.top_value {
            val = 2 * self.top_value - val;
        }
        val & self.mask_val
    }

    // JS set(cpuVal, tmpVal = false)
    pub fn set(&mut self, val: u32, clock_ticks: f64, is_zigzag_complement: bool) {
        self.base_value = if is_zigzag_complement {
            2 * self.top_value - val
        } else {
            val
        } & self.mask_val;
        self.base_ticks = clock_ticks;
    }

    // JS bits setter
    pub fn set_bits(&mut self, bits: u32) {
        self.bit_count = bits;
        self.mask_val = if bits == 32 || bits == 0 {
            0xFFFF_FFFF
        } else {
            (1u32 << bits) - 1
        };
    }

    // JS prescaler setter
    pub fn set_prescaler(&mut self, val: f64, clock_ticks: f64) {
        self.base_value = self.counter(clock_ticks);
        self.base_ticks = clock_ticks;
        self.prescaler_val = val;
    }

    // JS enable setter
    pub fn set_enable(&mut self, val: bool, clock_ticks: f64) {
        if val != self.enabled {
            if val {
                self.base_ticks = clock_ticks;
            } else {
                self.base_value = self.counter(clock_ticks);
            }
            self.enabled = val;
        }
    }

    // JS toTicks
    pub fn to_ticks(&self, counter_val: u32) -> f64 {
        counter_val as f64 * self.prescaler_val
    }

    // JS reset()
    pub fn reset(&mut self, clock_ticks: f64) {
        self.base_ticks = clock_ticks;
        self.base_value = 0;
    }

    // JS get enable
    pub fn enable(&self) -> bool { self.enabled }

    // JS get bits
    pub fn bits(&self) -> u32 { self.bit_count }

    // JS get mask
    pub fn mask(&self) -> u32 { self.mask_val }

    // JS get top
    pub fn top(&self) -> u32 { self.top_value }
}

// ============================================================
// LedcTimer — JS class LedcTimer
// ============================================================

#[derive(Clone, Copy)]
pub struct LedcTimer {
    pub config: LedcTimerConfig,
    pub conf: u32,
    pub timer32: LedcTimerCounter,
    pub clock_divider: u32,
    pub clock_source_is_ref: bool,
}

impl LedcTimer {
    // JS constructor(cpuVal, tmpVal)
    pub fn new(config: LedcTimerConfig) -> Self {
        LedcTimer {
            config,
            conf: 0,
            timer32: LedcTimerCounter::new(),
            clock_divider: 1,
            clock_source_is_ref: false,
        }
    }

    // JS readRegister(cpuVal)
    pub fn read_register(&mut self, ctx: &CpuContext, offset: u32) -> u32 {
        let tmp_val = self.config.conf_reg;
        let idx_val = self.config.value_reg;
        match offset {
            a if a == tmp_val => self.conf,
            a if a == idx_val => {
                let clock_ticks = self.compute_clock_ticks(ctx);
                self.timer32.counter(clock_ticks)
            }
            _ => 0,
        }
    }

    // JS writeRegister(cpuVal, tmpVal)
    pub fn write_register(&mut self, ctx: &mut CpuContext, offset: u32, val: u32) {
        let has_ref_tick = ctx.clocks.ref_tick.frequency != 0;
        let clock_event = self.config.conf_reg;
        let simulation_clock = self.config.value_reg;

        // Field bit masks — match JS config.F shifts:
        //   TICK_SEL(352,25,1), PAUSE(352,23,1), RST(352,24,1), PARA_UP(352,26,1)
        let len_val = 1u32 << 25; // TIMERn_TICK_SEL bit
        let val_val = 1u32 << 26; // TIMERn_PARA_UP bit
        let flag = 1u32 << 23;   // TIMERn_PAUSE bit
        let _t_val = 1u32 << 24; // TIMERn_RST bit — JS branch is a no-op

        match offset {
            a if a == clock_event => {
                self.conf = val & !val_val;
                if self.config.hs_timer || (val & val_val) != 0 {
                    // JS: timer32.bits = (tmpVal >> 0) & 0x1F
                    let duty_res = (val >> 0) & 0x1F;
                    // JS: timer32.prescaler = ((tmpVal >> 5) & 0x3FFFF) / 256
                    let raw_clk_div = (val >> 5) & 0x3FFFF;
                    self.timer32.set_bits(duty_res);
                    let prescaler = raw_clk_div as f64 / 256.0;
                    let clock_ticks = self.compute_clock_ticks(ctx);
                    self.timer32.set_prescaler(prescaler, clock_ticks);
                }
                if has_ref_tick {
                    // TRM + ledc_ll (tick_sel = (clk == APB)): TICK_SEL set
                    // selects the fast (APB/slow_clk) parent on both HS and
                    // LS timers; clear selects ref_tick. (The old LS-inverted
                    // polarity made fades ~80x too slow.)
                    self.clock_source_is_ref = (val & len_val) == 0;
                } else {
                    self.clock_source_is_ref = false;
                }
                // JS: if (tmpVal & flag) { this.timer32.enable = false; this.timer32.reset(); }
                //     if (tmpVal & TVal) { /* retry */ ; } — no-op
                if (val & flag) != 0 {
                    let clock_ticks = self.compute_clock_ticks(ctx);
                    self.timer32.set_enable(false, clock_ticks);
                    self.timer32.reset(clock_ticks);
                }
            }
            a if a == simulation_clock => {
                let clock_ticks = self.compute_clock_ticks(ctx);
                self.timer32.set(val, clock_ticks, false);
            }
            _ => {}
        }
    }

    // JS createAlarm(cpuVal) — returns alarm config, caller manages schedule
    pub fn create_alarm(&self) -> LedcAlarmState {
        LedcAlarmState::new()
    }

    // JS getOutputFrequency()
    pub fn get_output_frequency(&self, ctx: &CpuContext) -> f64 {
        let parent_freq = if self.clock_source_is_ref {
            ctx.clocks.ref_tick.frequency
        } else {
            ctx.clocks.apb.frequency
        } as f64;
        let prescaler = if self.timer32.prescaler_val != 0.0 {
            self.timer32.prescaler_val
        } else {
            1.0
        };
        let bit_count = if self.timer32.bits() != 0 {
            self.timer32.bits()
        } else {
            1
        };
        let tmp_val = prescaler * (1u32 << bit_count) as f64;
        if tmp_val > 0.0 { parent_freq / tmp_val } else { 0.0 }
    }

    // Compute the clock's current ticks in its time domain
    pub fn compute_clock_ticks(&self, ctx: &CpuContext) -> f64 {
        let apb_ticks = ctx.apb_ticks() as f64;
        if self.clock_source_is_ref {
            apb_ticks
        } else {
            apb_ticks / self.clock_divider as f64
        }
    }
}

// ============================================================
// LedcAlarmState — JS Timer32Alarm state (inlined, no closures)
// ============================================================

#[derive(Clone, Copy)]
pub struct LedcAlarmState {
    pub enabled: bool,
    pub target_value: u32,
}

impl LedcAlarmState {
    pub fn new() -> Self {
        LedcAlarmState { enabled: false, target_value: 0 }
    }
}

// ============================================================
// LedcChannel — JS class LedcChannel
// ============================================================

pub const MAX_LEDC_TIMERS: usize = 8;

#[derive(Clone, Copy)]
pub struct LedcChannel {
    pub index: u32,
    pub matrix_signal: u32,
    pub registers: LedcChannelRegisters,
    pub timer_index: u32,
    pub duty_mask: u32,
    pub hpoint: u32,
    pub duty: u32,
    pub current_duty: u32,
    pub conf0: u32,
    pub conf1: u32,
    pub timer_count: usize,
    pub alarm_high: [LedcAlarmState; MAX_LEDC_TIMERS],
    pub alarm_low: [LedcAlarmState; MAX_LEDC_TIMERS],
    // Hardware fade engine: DUTY_START latches a ramp from current_duty
    // toward the target (start +/- DUTY_NUM steps) over fade_end_tick;
    // DUTY_R interpolates while active and a LedcFadeEnd event finalizes +
    // raises FADE_END (DUTY_CHNG_END). The driver never writes the DUTY
    // register for fades — the target lives in CONF1's DUTY_NUM step count.
    pub fade_active: bool,
    pub fade_start_duty: u32,
    pub fade_target: u32,
    pub fade_start_tick: u64,
    pub fade_end_tick: u64,
}

#[derive(Clone, Copy)]
pub struct LedcChannelRegisters {
    pub conf0: u32,
    pub conf1: u32,
    pub hpoint: u32,
    pub duty: u32,
    pub duty_r: u32,
}

impl LedcChannel {
    // JS constructor(cpuVal, tmpVal, idxVal, ClockEvent, SimulationClock, regVal)
    pub fn new(
        index: u32,
        matrix_signal: u32,
        registers: LedcChannelRegisters,
        timer_count: usize,
        duty_bits: u32,
    ) -> Self {
        LedcChannel {
            index,
            matrix_signal,
            registers,
            timer_index: 0,
            duty_mask: if duty_bits >= 32 { 0xFFFF_FFFF } else { (1u32 << duty_bits) - 1 },
            hpoint: 0,
            duty: 0,
            current_duty: 0,
            conf0: 0,
            conf1: 0,
            timer_count,
            alarm_high: [LedcAlarmState::new(); MAX_LEDC_TIMERS],
            alarm_low: [LedcAlarmState::new(); MAX_LEDC_TIMERS],
            fade_active: false,
            fade_start_duty: 0,
            fade_target: 0,
            fade_start_tick: 0,
            fade_end_tick: 0,
        }
    }

    // JS readRegister(cpuVal)
    pub fn read_register(&self, ctx: &CpuContext, offset: u32) -> u32 {
        let tmp_val = &self.registers;
        match offset {
            a if a == tmp_val.conf0 => self.conf0,
            a if a == tmp_val.conf1 => self.conf1,
            a if a == tmp_val.hpoint => self.hpoint,
            a if a == tmp_val.duty => self.duty,
            a if a == tmp_val.duty_r => self.effective_duty(ctx),
            _ => 0,
        }
    }

    // Current hardware duty for DUTY_RD reads. NOTE: LEDC_DUTY_RD
    // carries the integer duty in bits[29:4] (ledc_hal_get_duty shifts
    // right by 4), so the interpolated/latched duty is returned << 4.
    fn effective_duty(&self, ctx: &CpuContext) -> u32 {
        self.effective_duty_inner(ctx) << 4
    }

    fn effective_duty_inner(&self, ctx: &CpuContext) -> u32 {
        if !self.fade_active {
            return self.current_duty;
        }
        let now = ctx.apb_ticks();
        if now >= self.fade_end_tick || self.fade_end_tick <= self.fade_start_tick {
            return self.fade_target;
        }
        // saturating_sub: never wrap on time warp; final clamp keeps the
        // value inside [start, target] so the driver always observes the
        // exact target (it keys completion off equality).
        let el = now.saturating_sub(self.fade_start_tick);
        let span = self.fade_end_tick - self.fade_start_tick;
        let v = if self.fade_target >= self.fade_start_duty {
            self.fade_start_duty + ((self.fade_target - self.fade_start_duty) as u64 * el / span) as u32
        } else {
            self.fade_start_duty - ((self.fade_start_duty - self.fade_target) as u64 * el / span) as u32
        };
        let lo = self.fade_start_duty.min(self.fade_target);
        let hi = self.fade_start_duty.max(self.fade_target);
        v.clamp(lo, hi) & self.duty_mask
    }

    // DUTY_START (CONF1 bit31): begin a hardware fade from current_duty
    // toward start +/- DUTY_NUM. Returns true if it completed inline
    // (degenerate params).
    fn start_fade(&mut self, ctx: &mut CpuContext, val: u32, timer: &LedcTimer) -> bool {
        let start = self.current_duty;
        let num = ((val >> 20) & 0x3FF) as u32;
        let up = (val >> 30) & 1 != 0;
        // Target = start +/- step count (HW clamps at the duty range).
        let target = if up {
            start.saturating_add(num).min(self.duty_mask)
        } else {
            start.saturating_sub(num)
        };
        self.fade_start_duty = start;
        self.fade_target = target;
        self.fade_start_tick = ctx.apb_ticks();
        let cycle = ((val >> 10) & 0x3FF) as u64;
        let scale = (val & 0x3FF) as u32;
        let total_pwm = cycle * num as u64 * (1u64 << scale.min(40));
        let freq = timer.get_output_frequency(ctx);
        if freq <= 0.0 || total_pwm == 0 {
            // Degenerate (plain duty set with START): complete immediately,
            // preserving the old instant-interrupt behavior.
            self.current_duty = self.duty;
            self.fade_active = false;
            return true;
        }
        let total_apb = (total_pwm as f64 * 80_000_000.0 / freq) as u64;
        if total_apb == 0 {
            self.current_duty = self.fade_target;
            self.fade_active = false;
            return true;
        }
        self.fade_end_tick = self.fade_start_tick + total_apb;
        self.fade_active = true;
        ctx.schedule_event(total_apb, EventTag::LedcFadeEnd { channel: self.index });
        false
    }

    // JS writeRegister(cpuVal, tmpVal)
    pub fn write_register(
        &mut self,
        ctx: &mut CpuContext,
        offset: u32,
        val: u32,
        timer: &LedcTimer,
        camera_active: bool,
    ) -> bool {
        let idx_val = &self.registers;
        match offset {
            a if a == idx_val.conf0 => {
                let old_sigma = self.conf0 & LED_REG4;
                self.conf0 = val;
                let timer_idx = (val >> LED_REG2) & LED_REG3;
                if timer_idx != self.timer_index {
                    let old_timer = self.timer_index as usize;
                    self.alarm_high[old_timer].enabled = false;
                    self.alarm_low[old_timer].enabled = false;
                    self.timer_index = timer_idx;
                }
                let clock_event = (val & LED_REG5) != 0;
                // JS: cpuVal || this.matrix.setOutput(this.matrixSignal, true, ClockEvent)
                if old_sigma == 0 {
                    ctx.gpio_matrix.set_output(self.matrix_signal, true, clock_event);
                }
                self.update_alarms(ctx, timer, camera_active);
                false
            }
            a if a == idx_val.conf1 => {
                self.conf1 = val & !LED_REG12;
                // DUTY_START (bit31): begin a hardware fade; the FADE_END
                // interrupt fires at completion (or inline if degenerate).
                if val & LED_REG12 != 0 {
                    return self.start_fade(ctx, val, timer);
                }
                false
            }
            a if a == idx_val.hpoint => {
                self.hpoint = val & LED_REG7;
                self.update_alarms(ctx, timer, camera_active);
                false
            }
            a if a == idx_val.duty => {
                self.duty = val & self.duty_mask;
                self.update_alarms(ctx, timer, camera_active);
                false
            }
            _ => false,
        }
    }

    // JS updateAlarms() — also handles scheduling/cancelling Timer32Alarm events
    pub fn update_alarms(
        &mut self,
        ctx: &mut CpuContext,
        timer: &LedcTimer,
        camera_active: bool,
    ) {
        let timer_idx = self.timer_index as usize;
        let tmp_val = self.duty >> 4;
        let high_target = self.hpoint;
        let low_target = self.hpoint + tmp_val;

        // Set alarm targets (mimics JS Timer32Alarm.target setter)
        let high_changed = self.alarm_high[timer_idx].target_value != high_target;
        let low_changed = self.alarm_low[timer_idx].target_value != low_target;
        if high_changed {
            self.alarm_high[timer_idx].target_value = high_target;
        }
        if low_changed {
            self.alarm_low[timer_idx].target_value = low_target;
        }

        let idx_val = (self.conf0 & LED_REG4) != 0;
        let clock_event = tmp_val == 0;
        let bits = timer.timer32.bits();
        let simulation_clock = if bits >= 32 {
            tmp_val >= (1u32 << 31)
        } else {
            tmp_val >= (1u32 << bits)
        };
        let reg_val = clock_event || simulation_clock;

        let freq = timer.get_output_frequency(ctx);
        let cfg_val = camera_active;
        if (freq > 1e7 || (cfg_val && freq > 1e6)) && idx_val {
            // JS: setOutput(matrixSignal, true, true) — disable alarms, output high
            ctx.gpio_matrix.set_output(self.matrix_signal, true, true);
            self.alarm_high[timer_idx].enabled = false;
            self.alarm_low[timer_idx].enabled = false;
            return;
        }

        let should_enable = idx_val && !reg_val;
        let high_should_enable = should_enable;
        let low_should_enable = should_enable;

        // Apply enable changes and schedule/cancel events
        // (mirrors JS Timer32Alarm.enable setter)
        if high_should_enable != self.alarm_high[timer_idx].enabled {
            self.alarm_high[timer_idx].enabled = high_should_enable;
            if high_should_enable && timer.timer32.enabled {
                self.schedule_alarm(ctx, timer, timer_idx, true);
            }
        } else if high_should_enable && high_changed && timer.timer32.enabled {
            // target changed while enabled: cancel + reschedule
            ctx.unschedule_event(EventTag::LedcTimerAlarm {
                channel: self.index, timer_idx: self.timer_index, is_high: true,
            });
            self.schedule_alarm(ctx, timer, timer_idx, true);
        }

        if low_should_enable != self.alarm_low[timer_idx].enabled {
            self.alarm_low[timer_idx].enabled = low_should_enable;
            if low_should_enable && timer.timer32.enabled {
                self.schedule_alarm(ctx, timer, timer_idx, false);
            }
        } else if low_should_enable && low_changed && timer.timer32.enabled {
            ctx.unschedule_event(EventTag::LedcTimerAlarm {
                channel: self.index, timer_idx: self.timer_index, is_high: false,
            });
            self.schedule_alarm(ctx, timer, timer_idx, false);
        }

        // JS: idxVal && regVal && setOutput(this.matrixSignal, true, SimulationClock)
        if idx_val && reg_val {
            ctx.gpio_matrix.set_output(self.matrix_signal, true, simulation_clock);
        }
    }

    // JS Timer32Alarm.schedule() — compute delay and schedule event
    fn schedule_alarm(
        &self,
        ctx: &mut CpuContext,
        timer: &LedcTimer,
        timer_idx: usize,
        is_high: bool,
    ) {
        let alarm = if is_high {
            &self.alarm_high[timer_idx]
        } else {
            &self.alarm_low[timer_idx]
        };
        let clock_ticks = timer.compute_clock_ticks(ctx);
        let raw_counter = timer.timer32.raw_counter(clock_ticks);
        let mut reg_val = alarm.target_value.wrapping_sub(raw_counter);

        // JS: ClockEvent === TimerMode.ZigZag handling
        if timer.timer32.timer_mode == TimerMode::ZigZag && (reg_val as i32) < 0 {
            let idx = timer.timer32.top() as i32;
            if (reg_val as i32) < -idx {
                reg_val = reg_val.wrapping_add(2 * timer.timer32.top());
            } else {
                reg_val = 2 * timer.timer32.top() - alarm.target_value - raw_counter;
            }
        }

        // JS: top limit handling
        if timer.timer32.top() != 0xFFFF_FFFF {
            if (reg_val as i32) <= 0 {
                reg_val = reg_val.wrapping_add(timer.timer32.top() + 1);
            }
            if alarm.target_value > timer.timer32.top() {
                return;
            }
        }

        // JS: Decrement mode
        if timer.timer32.timer_mode == TimerMode::Decrement {
            reg_val = timer.timer32.top() - reg_val;
        }

        let arg_val = reg_val & timer.timer32.mask();
        let timer_ticks = timer.timer32.to_ticks(arg_val);
        let apb_delta = (timer_ticks * 1.0) as u64; // clock_divider = 1

        // Only schedule if delta > 0
        if apb_delta > 0 {
            ctx.schedule_event(apb_delta, EventTag::LedcTimerAlarm {
                channel: self.index,
                timer_idx: self.timer_index,
                is_high,
            });
        }
    }

    // Called when a LEDC timer alarm event fires
    // JS: this.handleAlarm = () => { (this.callback(), this.enabled && this.timer.enable && this.schedule()); }
    pub fn handle_alarm_fired(
        &mut self,
        ctx: &mut CpuContext,
        timer: &LedcTimer,
        timer_idx: u32,
        is_high: bool,
    ) {
        // JS: highPoint → setOutput(matrixSignal, true, true)
        //     lowPoint  → setOutput(matrixSignal, true, false)
        if is_high {
            ctx.gpio_matrix.set_output(self.matrix_signal, true, true);
        } else {
            ctx.gpio_matrix.set_output(self.matrix_signal, true, false);
        }
        let alarm = if is_high {
            &mut self.alarm_high[timer_idx as usize]
        } else {
            &mut self.alarm_low[timer_idx as usize]
        };
        if alarm.enabled && timer.timer32.enabled {
            self.schedule_alarm(ctx, timer, timer_idx as usize, is_high);
        }
    }
}

// ============================================================
// LedcPeripheral — JS class LedcPeripheral extends PeripheralBase
// ============================================================

pub const MAX_LEDC_CHANNELS: usize = 16;
pub const MAX_LEDC_TIMER_MAP: usize = 96;
pub const MAX_LEDC_HS_TIMERS: usize = 4;
pub const MAX_LEDC_LS_TIMERS: usize = 4;

#[derive(Clone, Copy)]
pub enum LedcMapEntry {
    Timer(u32),
    Channel(u32),
}

pub struct LedcPeripheral {
    pub base: PeripheralBase,
    pub config: LedcConfig,
    pub int_ena: u32,
    pub int_raw: u32,
    pub ls_timers: [LedcTimer; MAX_LEDC_LS_TIMERS],
    pub hs_timers: [LedcTimer; MAX_LEDC_HS_TIMERS],
    pub ls_timer_count: usize,
    pub hs_timer_count: usize,
    pub channels: [LedcChannel; MAX_LEDC_CHANNELS],
    pub channel_count: usize,
    pub reg_map: [(u32, LedcMapEntry); MAX_LEDC_TIMER_MAP],
    pub reg_map_count: usize,
    pub duty_bits: u32,
}

#[derive(Clone, Copy)]
pub struct LedcConfig {
    pub hs_channels: usize,
    pub ls_channels: usize,
    pub hs_timers: usize,
    pub ls_timers: usize,
    pub ch0_matrix_out: u32,
    pub irq: u32,
    pub duty0_int: u32,
    pub duty_bits: u32,
    pub rmt: RmtChannelRegisterOffsets,
    pub has_clock_sources_array: bool,
}

#[derive(Clone, Copy)]
pub struct RmtChannelRegisterOffsets {
    pub int_raw: u32,
    pub int_ena: u32,
    pub int_st: u32,
    pub int_clr: u32,
    pub conf: u32,
    pub hstimer0_conf: u32,
    pub hstimer1_conf: u32,
    pub hstimer0_value: u32,
    pub hstimer1_value: u32,
    pub timer0_conf: u32,
    pub timer1_conf: u32,
    pub timer0_value: u32,
    pub timer1_value: u32,
}

impl LedcPeripheral {
    // JS constructor(cpuVal, tmpVal, idxVal, ClockEvent)
    pub fn new(
        base_addr: u32,
        name: &'static str,
        config: LedcConfig,
    ) -> Self {
        // Build default timers (will be overwritten in init_timers)
        let default_timer = LedcTimer::new(LedcTimerConfig {
            conf_reg: 0, value_reg: 0, hs_timer: false, ls_timer: false,
        });
        let default_channel = LedcChannel::new(0, 0, LedcChannelRegisters {
            conf0: 0, conf1: 0, hpoint: 0, duty: 0, duty_r: 0,
        }, 0, config.duty_bits);

        let mut p = LedcPeripheral {
            base: PeripheralBase::new(base_addr, name),
            config,
            int_ena: 0,
            int_raw: 0,
            ls_timers: [default_timer; MAX_LEDC_LS_TIMERS],
            hs_timers: [default_timer; MAX_LEDC_HS_TIMERS],
            ls_timer_count: 0,
            hs_timer_count: 0,
            channels: [default_channel; MAX_LEDC_CHANNELS],
            channel_count: 0,
            reg_map: [(0, LedcMapEntry::Timer(0)); MAX_LEDC_TIMER_MAP],
            reg_map_count: 0,
            duty_bits: config.duty_bits,
        };
        p.init_timers_and_channels();
        p
    }

    fn init_timers_and_channels(&mut self) {
        // Build hsTimers
        let hs_timer_count = self.config.hs_timers;
        for i in 0..hs_timer_count {
            let stride = self.config.rmt.hstimer1_conf - self.config.rmt.hstimer0_conf;
            let tmp = stride * i as u32;
            let timer_config = LedcTimerConfig {
                conf_reg: self.config.rmt.hstimer0_conf + tmp,
                value_reg: self.config.rmt.hstimer0_value + tmp,
                hs_timer: true,
                ls_timer: false,
            };
            self.reg_map[self.reg_map_count] = (timer_config.conf_reg, LedcMapEntry::Timer(i as u32));
            self.reg_map_count += 1;
            self.reg_map[self.reg_map_count] = (timer_config.value_reg, LedcMapEntry::Timer(i as u32));
            self.reg_map_count += 1;
            self.hs_timers[i] = LedcTimer::new(timer_config);
        }
        self.hs_timer_count = hs_timer_count;

        // Build lsTimers
        let ls_timer_count = self.config.ls_timers;
        for i in 0..ls_timer_count {
            let stride = self.config.rmt.timer1_conf - self.config.rmt.timer0_conf;
            let tmp = stride * i as u32;
            let timer_config = LedcTimerConfig {
                conf_reg: self.config.rmt.timer0_conf + tmp,
                value_reg: self.config.rmt.timer0_value + tmp,
                hs_timer: false,
                ls_timer: hs_timer_count > 0,
            };
            let map_idx = (i + hs_timer_count) as u32;
            self.reg_map[self.reg_map_count] = (timer_config.conf_reg, LedcMapEntry::Timer(map_idx));
            self.reg_map_count += 1;
            self.reg_map[self.reg_map_count] = (timer_config.value_reg, LedcMapEntry::Timer(map_idx));
            self.reg_map_count += 1;
            self.ls_timers[i] = LedcTimer::new(timer_config);
        }
        self.ls_timer_count = ls_timer_count;

        // Build channels
        let total_channels = self.config.hs_channels + self.config.ls_channels;
        for i in 0..total_channels {
            let tmp = 20 * i as u32;
            let use_hs = i < self.config.hs_channels;
            let timer_count = if use_hs { hs_timer_count } else { ls_timer_count };
            let registers = LedcChannelRegisters {
                conf0: LED_REG1 + tmp,
                conf1: LED_REG9 + tmp,
                hpoint: LED_REG6 + tmp,
                duty: LED_REG8 + tmp,
                duty_r: LED_REG13 + tmp,
            };
            let channel = LedcChannel::new(
                i as u32,
                self.config.ch0_matrix_out + i as u32,
                registers,
                timer_count,
                self.config.duty_bits,
            );
            // Register channel register offsets in the map
            for &reg_offset in &[registers.conf0, registers.conf1, registers.hpoint,
                                 registers.duty, registers.duty_r] {
                self.reg_map[self.reg_map_count] = (reg_offset, LedcMapEntry::Channel(i as u32));
                self.reg_map_count += 1;
            }
            self.channels[i] = channel;
        }
        self.channel_count = total_channels;
    }

    // JS get cameraActive
    pub fn camera_active(&self) -> bool { false }

    // JS get intStatus
    pub fn int_status(&self) -> u32 { self.int_raw & self.int_ena }

    // JS readUint32(cpuVal)
    pub fn read_uint32(&mut self, ctx: &CpuContext, addr: u32) -> u32 {
        let tmp_val = addr - self.base.base_addr;

        // Search reg map
        for i in 0..self.reg_map_count {
            if self.reg_map[i].0 == tmp_val {
                match self.reg_map[i].1 {
                    LedcMapEntry::Timer(idx) => {
                        let idx = idx as usize;
                        let timer = if idx < self.hs_timer_count {
                            &mut self.hs_timers[idx]
                        } else {
                            &mut self.ls_timers[idx - self.hs_timer_count]
                        };
                        return timer.read_register(ctx, tmp_val);
                    }
                    LedcMapEntry::Channel(idx) => {
                        return self.channels[idx as usize].read_register(ctx, tmp_val);
                    }
                }
            }
        }

        let rmt = &self.config.rmt;
        match tmp_val {
            a if a == rmt.int_raw => self.int_raw,
            a if a == rmt.int_ena => self.int_ena,
            a if a == rmt.int_st => self.int_status(),
            _ => self.base.read_uint32(addr),
        }
    }

    // JS writeUint32(cpuVal, tmpVal)
    pub fn write_uint32(&mut self, ctx: &mut CpuContext, addr: u32, val: u32) {
        self.base.write_uint32(addr, val);
        let clock_event = addr - self.base.base_addr;

        // Search reg map
        for i in 0..self.reg_map_count {
            if self.reg_map[i].0 == clock_event {
                match self.reg_map[i].1 {
                    LedcMapEntry::Timer(idx) => {
                        let idx = idx as usize;
                        let timer = if idx < self.hs_timer_count {
                            &mut self.hs_timers[idx]
                        } else {
                            &mut self.ls_timers[idx - self.hs_timer_count]
                        };
                        timer.write_register(ctx, clock_event, val);
                    }
                    LedcMapEntry::Channel(idx) => {
                        let ch_idx = idx as usize;
                        let camera_active = self.camera_active();
                        let timer_index = self.channels[ch_idx].timer_index as usize;
                        let is_hs = (idx as usize) < self.config.hs_channels;
                        let timer = if is_hs {
                            &self.hs_timers[timer_index]
                        } else {
                            &self.ls_timers[timer_index]
                        };
                        let needs_interrupt = self.channels[ch_idx].write_register(
                            ctx, clock_event, val, timer, camera_active,
                        );
                        if needs_interrupt {
                            self.channel_interrupt(idx);
                            self.update_interrupts(ctx);
                        }
                    }
                }
                self.base.write_uint32(addr, val);
                return;
            }
        }

        let rmt = &self.config.rmt;
        match clock_event {
            a if a == rmt.conf => {
                if self.config.has_clock_sources_array {
                    let _clk_src_idx = val & LED_REG14;
                    // clock source array handling — stub
                }
            }
            a if a == rmt.int_clr => {
                self.int_raw &= !val;
                self.update_interrupts(ctx);
                self.base.write_uint32(addr, val);
                return;
            }
            a if a == rmt.int_ena => {
                self.int_ena = val;
                self.update_interrupts(ctx);
            }
            _ => {}
        }
        self.base.write_uint32(addr, val);
    }

    // JS channelInterrupt(cpuVal)
    pub fn channel_interrupt(&mut self, channel: u32) {
        self.int_raw |= 1 << (channel + self.config.duty0_int);
    }

    // JS updateInterrupts
    pub fn update_interrupts(&self, ctx: &mut CpuContext) {
        ctx.interrupt(self.config.irq, self.int_status() != 0);
    }

    // Handle alarm event firing (called by simulation loop)
    pub fn handle_event(&mut self, ctx: &mut CpuContext, tag: EventTag) {
        if let EventTag::LedcTimerAlarm { channel, timer_idx, is_high } = tag {
            let ch = channel as usize;
            if ch < self.channel_count {
                let timer_index = timer_idx as usize;
                let is_hs = ch < self.config.hs_channels;
                let timer = if is_hs {
                    &self.hs_timers[timer_index]
                } else {
                    &self.ls_timers[timer_index]
                };
                self.channels[ch].handle_alarm_fired(ctx, timer, timer_idx, is_high);
            }
        } else if let EventTag::LedcFadeEnd { channel } = tag {
            // Hardware fade completion: latch target duty, raise FADE_END.
            let ch = channel as usize;
            if ch < self.channel_count && self.channels[ch].fade_active {
                self.channels[ch].current_duty = self.channels[ch].fade_target;
                self.channels[ch].fade_active = false;
                self.channel_interrupt(channel);
                self.update_interrupts(ctx);
            }
        }
    }
}

// Helper to collect all timers as a slice reference
fn concat_timers<'a>(hs: &'a [LedcTimer; MAX_LEDC_HS_TIMERS], hs_count: usize,
                     ls: &'a [LedcTimer; MAX_LEDC_LS_TIMERS], ls_count: usize) -> &'a [LedcTimer] {
    if hs_count > 0 { &hs[..hs_count] } else { &ls[..ls_count] }
}

// ============================================================
// MmioPeripheral impl for LedcPeripheral
// ============================================================

impl MmioPeripheral for LedcPeripheral {
    fn read_u32(&mut self, ctx: &mut CpuContext, addr: u32) -> u32 {
        self.read_uint32(ctx, addr)
    }
    fn write_u32(&mut self, ctx: &mut CpuContext, addr: u32, val: u32) {
        self.write_uint32(ctx, addr, val);
    }
    fn reset(&mut self) {
        self.base.reset();
        self.int_raw = 0;
        self.int_ena = 0;
    }
}

// ============================================================
// PcntUnit — JS class PcntUnit
// ============================================================

#[derive(Clone, Copy)]
pub struct PcntUnit {
    pub index: u32,
    pub cnt: u32,
    pub status: u32,
    pub conf0: u32,
    pub filter_thres: u32,
    pub filter_en: bool,
    pub thr_zero_en: bool,
    pub h_lim_en: bool,
    pub l_lim_en: bool,
    pub thres0_en: bool,
    pub thres1_en: bool,
    pub ch0_neg_mode: u32,
    pub ch0_pos_mode: u32,
    pub ch0_h_ctrl_mode: u32,
    pub ch0_l_ctrl_mode: u32,
    pub ch1_neg_mode: u32,
    pub ch1_pos_mode: u32,
    pub ch1_h_ctrl_mode: u32,
    pub ch1_l_ctrl_mode: u32,
    pub thres0: u32,
    pub thres1: u32,
    pub thr_l_lim: u32,
    pub thr_h_lim: u32,
    pub thr_l_lim_buf: u32,
    pub thr_h_lim_buf: u32,
    pub prev_update_ticks: u64,
    pub pause: bool,
    pub ctrl_input_0: u32,
    pub ctrl_input_1: u32,
}

impl PcntUnit {
    // JS constructor(cpuVal, tmpVal, idxVal, ClockEvent)
    pub fn new(index: u32) -> Self {
        PcntUnit {
            index,
            cnt: 0,
            status: 0,
            conf0: 0,
            filter_thres: 0,
            filter_en: false,
            thr_zero_en: false,
            h_lim_en: false,
            l_lim_en: false,
            thres0_en: false,
            thres1_en: false,
            ch0_neg_mode: 0,
            ch0_pos_mode: 0,
            ch0_h_ctrl_mode: 0,
            ch0_l_ctrl_mode: 0,
            ch1_neg_mode: 0,
            ch1_pos_mode: 0,
            ch1_h_ctrl_mode: 0,
            ch1_l_ctrl_mode: 0,
            thres0: 0,
            thres1: 0,
            thr_l_lim: 0,
            thr_h_lim: 0,
            thr_l_lim_buf: 0,
            thr_h_lim_buf: 0,
            prev_update_ticks: 0,
            pause: false,
            ctrl_input_0: 0,
            ctrl_input_1: 0,
        }
    }

    // JS readRegister(cpuVal)
    pub fn read_register(&self, reg: u32) -> u32 {
        match reg {
            PCNT_CONF0 => self.conf0,
            PCNT_CONF1 => (self.thres1 << 16) | (0xFFFF & self.thres0),
            PCNT_CONF2 => (self.thr_l_lim << 16) | (0xFFFF & self.thr_h_lim),
            PCNT_CNT => self.cnt,
            PCNT_STATUS => self.status,
            _ => 0,
        }
    }

    // JS writeRegister(cpuVal, tmpVal)
    pub fn write_register(&mut self, reg: u32, val: u32) {
        match reg {
            PCNT_CONF0 => {
                self.conf0 = val;
                self.filter_thres = 1023 & val;
                self.filter_en = (1024 & val) != 0;
                self.thr_zero_en = (2048 & val) != 0;
                self.h_lim_en = (4096 & val) != 0;
                self.l_lim_en = (8192 & val) != 0;
                self.thres0_en = (16384 & val) != 0;
                self.thres1_en = (32768 & val) != 0;
                self.ch0_neg_mode = (val >> 16) & 3;
                self.ch0_pos_mode = (val >> 18) & 3;
                self.ch0_h_ctrl_mode = (val >> 20) & 3;
                self.ch0_l_ctrl_mode = (val >> 22) & 3;
                self.ch1_neg_mode = (val >> 24) & 3;
                self.ch1_pos_mode = (val >> 26) & 3;
                self.ch1_h_ctrl_mode = (val >> 28) & 3;
                self.ch1_l_ctrl_mode = (val >> 30) & 3;
            }
            PCNT_CONF1 => {
                self.thres0 = 0xFFFF & val;
                self.thres1 = val >> 16;
            }
            PCNT_CONF2 => {
                self.thr_l_lim = val >> 16;
                self.thr_h_lim = 0xFFFF & val;
            }
            _ => {}
        }
    }

    // JS updateBufferedThresholds()
    fn update_buffered_thresholds(&mut self) {
        self.thr_l_lim_buf = self.thr_l_lim;
        self.thr_h_lim_buf = self.thr_h_lim;
    }

    // JS set rst(cpuVal)
    pub fn set_rst(&mut self, val: u32) {
        if val != 0 {
            self.cnt = 0;
            self.update_buffered_thresholds();
        }
    }

    // JS sigInputChanged(cpuVal, tmpVal)
    pub fn sig_input_changed(&mut self, channel: u32, value: u32, apb_ticks: u64) {
        if self.pause {
            return;
        }
        let neg_mode = if channel == 0 { self.ch0_neg_mode } else { self.ch1_neg_mode };
        let pos_mode = if channel == 0 { self.ch0_pos_mode } else { self.ch1_pos_mode };
        let h_ctrl = if channel == 0 { self.ch0_h_ctrl_mode } else { self.ch1_h_ctrl_mode };
        let l_ctrl = if channel == 0 { self.ch0_l_ctrl_mode } else { self.ch1_l_ctrl_mode };
        let ctrl_val = if channel == 0 { self.ctrl_input_0 } else { self.ctrl_input_1 };
        let register_type = if value != 0 { pos_mode } else { neg_mode };
        let cfg_val = if ctrl_val != 0 { h_ctrl } else { l_ctrl };

        if cfg_val == 2 || cfg_val == 3 || register_type == 0 || register_type == 3 {
            return;
        }

        let h_val = register_type == 1;
        let off_val = if cfg_val == 1 { !h_val } else { h_val };
        let len_val = self.cnt;
        if off_val {
            self.cnt = (self.cnt.wrapping_add(1)) & 65535;
        } else {
            self.cnt = (self.cnt.wrapping_sub(1)) & 65535;
        }

        let val = (0x8000 & self.cnt) != 0;
        let flag = (0x8000 & len_val) != 0;
        let t_val = if val {
            LED_REG18
        } else if self.cnt != 0 {
            LED_REG17
        } else if flag {
            LED_REG16
        } else {
            LED_REG15
        };

        // JS: this.prevUpdateTicks !== this.clock.ticks && (this.status = 0)
        if self.prev_update_ticks != apb_ticks {
            self.status = 0;
        }
        self.prev_update_ticks = apb_ticks;
        self.status = (!4 & self.status) | t_val;
        self.update_status_interrupts();

        if off_val && self.cnt == self.thr_h_lim_buf {
            self.cnt = 0;
            self.update_status_interrupts();
        } else if !off_val && self.cnt == self.thr_l_lim_buf {
            self.cnt = 0;
            self.update_status_interrupts();
        }
    }

    // JS updateStatusInterrupts()
    fn update_status_interrupts(&mut self) {
        let bit_thres1 = 4u32;
        let bit_thres0 = 8u32;
        let bit_l_lim = 16u32;
        let bit_h_lim = 32u32;
        let bit_thr_zero = 64u32;
        let _reg_val = 124u32;

        if self.cnt == 0 && self.thr_zero_en {
            self.status |= bit_thr_zero;
        }
        if self.cnt == self.thres0 && self.thres0_en {
            self.status |= bit_thres0;
        }
        if self.cnt == self.thres1 && self.thres1_en {
            self.status |= bit_thres1;
        }
        if self.cnt == self.thr_l_lim_buf && self.l_lim_en {
            self.status |= bit_l_lim;
        }
        if self.cnt == self.thr_h_lim_buf && self.h_lim_en {
            self.status |= bit_h_lim;
        }

        // if self.status & reg_val != 0 { this.pcnt.unitInterrupt(this.index); }
        // Handled by the peripheral calling unitInterrupt when needed

        if self.cnt == 0
            || self.cnt == self.thres0
            || self.cnt == self.thres1
            || self.cnt == self.thr_l_lim_buf
            || self.cnt == self.thr_h_lim_buf
        {
            self.update_buffered_thresholds();
        }
    }

    // JS set pause(cpuVal)
    pub fn set_pause(&mut self, val: bool) {
        self.pause = val;
    }

    // Returns whether an interrupt should be raised
    pub fn has_pending_interrupt(&self) -> bool {
        (self.status & 124) != 0
    }
}

// ============================================================
// PcntPeripheral — JS class PcntPeripheral extends PeripheralBase
// ============================================================

pub const MAX_PCNT_UNITS: usize = 8;
pub const MAX_PCNT_MAP: usize = 40;

#[derive(Clone, Copy)]
pub struct PcntRegMapEntry {
    pub offset: u32,
    pub unit: u32,
    pub pcnt_reg: u32,
}

pub struct PcntPeripheral {
    pub base: PeripheralBase,
    pub config: PcntConfig,
    pub irq: u32,
    pub units: [PcntUnit; MAX_PCNT_UNITS],
    pub unit_count: usize,
    pub reg_map: [PcntRegMapEntry; MAX_PCNT_MAP],
    pub reg_map_count: usize,
    pub int_raw: u32,
    pub int_ena: u32,
}

#[derive(Clone, Copy)]
pub struct PcntConfig {
    pub unit_count: u32,
    pub un_conf0_first: u32,
    pub un_conf0_stride: u32,
    pub un_conf1_first: u32,
    pub un_conf1_stride: u32,
    pub un_conf2_first: u32,
    pub un_conf2_stride: u32,
    pub un_status_first: u32,
    pub un_status_stride: u32,
    pub un_cnt_first: u32,
    pub un_cnt_stride: u32,
    pub ctrl: u32,
    pub int_ena: u32,
    pub int_clr: u32,
    pub int_raw: u32,
    pub int_st: u32,
}

impl PcntPeripheral {
    // JS constructor(cpuVal, tmpVal, idxVal, ClockEvent, SimulationClock)
    // NOTE: GPIO signal connection (findGpioSignal/getInput/addListener)
    //       is omitted — requires GPIO matrix integration.
    //       The unit's ctrlInputs must be updated externally via set_ctrl_input().
    pub fn new(
        base_addr: u32,
        name: &'static str,
        config: PcntConfig,
        irq: u32,
    ) -> Self {
        let default_unit = PcntUnit::new(0);
        let mut p = PcntPeripheral {
            base: PeripheralBase::new(base_addr, name),
            config,
            irq,
            units: [default_unit; MAX_PCNT_UNITS],
            unit_count: 0,
            reg_map: [PcntRegMapEntry { offset: 0, unit: 0, pcnt_reg: 0 }; MAX_PCNT_MAP],
            reg_map_count: 0,
            int_raw: 0,
            int_ena: 0,
        };

        let unit_count = config.unit_count as usize;
        p.unit_count = unit_count;
        for i in 0..unit_count {
            let unit_idx = i as u32;
            p.units[i] = PcntUnit::new(unit_idx);

            let entries = [
                (config.un_conf0_first + unit_idx * config.un_conf0_stride, PCNT_CONF0),
                (config.un_conf1_first + unit_idx * config.un_conf1_stride, PCNT_CONF1),
                (config.un_conf2_first + unit_idx * config.un_conf2_stride, PCNT_CONF2),
                (config.un_status_first + unit_idx * config.un_status_stride, PCNT_STATUS),
                (config.un_cnt_first + unit_idx * config.un_cnt_stride, PCNT_CNT),
            ];
            for &(offset, pcnt_reg) in &entries {
                p.reg_map[p.reg_map_count] = PcntRegMapEntry { offset, unit: unit_idx, pcnt_reg };
                p.reg_map_count += 1;
            }
        }

        p
    }

    // JS unitInterrupt(cpuVal)
    pub fn unit_interrupt(&mut self, unit: u32) {
        self.int_raw |= 1 << unit;
    }

    // JS updateInterrupts
    fn update_interrupts(&self, ctx: &mut CpuContext) {
        let status = self.int_raw & self.int_ena;
        ctx.interrupt(self.irq, status != 0);
    }

    // JS readUint32(cpuVal)
    pub fn read_uint32(&mut self, _ctx: &CpuContext, addr: u32) -> u32 {
        let tmp_val = addr - self.base.base_addr;

        for i in 0..self.reg_map_count {
            if self.reg_map[i].offset == tmp_val {
                return self.units[self.reg_map[i].unit as usize].read_register(
                    self.reg_map[i].pcnt_reg,
                );
            }
        }

        match tmp_val {
            a if a == self.config.int_raw => self.int_raw,
            a if a == self.config.int_st => self.int_raw & self.int_ena,
            a if a == self.config.int_ena => self.int_ena,
            _ => self.base.read_uint32(addr),
        }
    }

    // JS writeUint32(cpuVal, tmpVal)
    pub fn write_uint32(&mut self, ctx: &mut CpuContext, addr: u32, val: u32) {
        self.base.write_uint32(addr, val);
        let idx_val = addr - self.base.base_addr;

        for i in 0..self.reg_map_count {
            if self.reg_map[i].offset == idx_val {
                self.units[self.reg_map[i].unit as usize].write_register(
                    self.reg_map[i].pcnt_reg, val,
                );
                return;
            }
        }

        match idx_val {
            a if a == self.config.ctrl => {
                for i in 0..self.unit_count {
                    let idx = 2 * i as u32;
                    let clock_event = 1u32 << idx;
                    let simulation_clock = 1u32 << (idx + 1);
                    self.units[i].set_rst(val & clock_event);
                    self.units[i].set_pause((val & simulation_clock) != 0);
                }
            }
            a if a == self.config.int_ena => {
                self.int_ena = val;
                self.update_interrupts(ctx);
            }
            a if a == self.config.int_clr => {
                self.int_raw &= !val;
                self.update_interrupts(ctx);
            }
            _ => {}
        }
        self.base.write_uint32(addr, val);
    }

    // Set control input for a PCNT unit (replaces GPIO addListener)
    pub fn set_ctrl_input(&mut self, unit: usize, channel: usize, value: u32) {
        if unit < self.unit_count {
            if channel == 0 {
                self.units[unit].ctrl_input_0 = value;
            } else {
                self.units[unit].ctrl_input_1 = value;
            }
        }
    }

    // Pulse input from GPIO-matrix routing (replaces GPIO addListener).
    // Feeds the edge through the unit counter and raises the unit interrupt
    // when an enabled threshold/limit trips (safe without int_ena: the IRQ
    // line only asserts when the driver enables it).
    pub fn pulse_input(
        &mut self,
        unit: usize,
        channel: u32,
        level: u32,
        apb_ticks: u64,
        ctx: &mut CpuContext,
    ) {
        if unit < self.unit_count {
            self.units[unit].sig_input_changed(channel, level, apb_ticks);
            if self.units[unit].has_pending_interrupt() {
                self.unit_interrupt(unit as u32);
                self.update_interrupts(ctx);
            }
        }
    }
}

impl MmioPeripheral for PcntPeripheral {
    fn read_u32(&mut self, ctx: &mut CpuContext, addr: u32) -> u32 {
        self.read_uint32(ctx, addr)
    }
    fn write_u32(&mut self, ctx: &mut CpuContext, addr: u32, val: u32) {
        self.write_uint32(ctx, addr, val);
    }
    fn reset(&mut self) {
        // empty in JS
    }
}
