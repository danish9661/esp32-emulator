// Translated from: src/peripherals/common/timers.js
// Do NOT modify the logic — match the JS line-for-line

use crate::peripherals::types::*;
use crate::peripherals::common::peripheral::{PeripheralBase, FieldDesc};
use core::cmp;

// ============================================================
// Constants (match JS let declarations)
// ============================================================

pub const TIM_REG11: u32 = 0;
pub const TIM_REG12: u32 = 4;
pub const TIM_REG13: u32 = 8;
pub const TIM_REG14: u32 = 12;
pub const TIM_REG15: u32 = 16;
pub const TIM_REG16: u32 = 256;
pub const TIM_REG17: u32 = 128;
pub const TIM_REG18: u32 = 64;

pub const ID: u32 = 0;
pub const DS_REG23: u32 = 4;
pub const DS_REG24: u32 = 8;
pub const DS_REG25: u32 = 12;
pub const DS_REG26: u32 = 16;
pub const DS_REG27: u32 = 20;
pub const TIMER_LOAD_LO_OFFSET: u32 = 24;
pub const DS_REG28: u32 = 28;
pub const DS_REG29: u32 = 32;
pub const DS_REG30: u32 = 0x80000000;
pub const DS_REG31: u32 = 0x40000000;
pub const DS_REG32: u32 = 0x20000000;
pub const DS_REG33: u32 = 13;
pub const DS_REG34: u32 = 65535;
pub const DS_REG35: u32 = 1024;
pub const DS_REG36: u32 = 512;
pub const DS_REG37: u32 = 1;
pub const DS_REG38: u32 = 2;
pub const DS_REG39: u32 = 8;

pub const DS_REG40: u32 = 0;
pub const DS_REG41: u32 = 1;
pub const DS_REG42: u32 = 2;
pub const DS_REG43: u32 = 3;
pub const DS_REG44: u32 = 0x50d83aa1;

// ============================================================
// Helpers (from helpers.js / bit-helpers.js)
// ============================================================

fn prescaler_to_divider(val: u32) -> u32 {
    val + 1
}

fn sign_extend(val: u32, bits: u32) -> i32 {
    if bits == 0 { return val as i32; }
    let mask = 1u32 << (bits - 1);
    if val & mask != 0 {
        (val | !((1u32 << bits) - 1)) as i32
    } else {
        val as i32
    }
}

fn read_field_value(val: u32, shift: u32, mask: u32) -> u32 {
    (val >> shift) & mask
}

// JS: timReg5(val, field) — clears field bits in val
fn clear_field(val: u32, shift: u32, mask: u32) -> u32 {
    let field_mask = mask << shift;
    val & !field_mask
}

fn read_field_val(val: u32, field: &FieldDesc) -> u32 {
    (val >> field.shift) & field.mask
}

fn clear_field_val(val: u32, field: &FieldDesc) -> u32 {
    val & !(field.mask << field.shift)
}

// ============================================================
// Timer32Alarm — JS class
// ============================================================

pub struct Timer32Alarm {
    pub timer: *mut Timer32Counter,
    pub target_value: u32,
    pub enabled: bool,
    pub event_tag: EventTag,
}

impl Timer32Alarm {
    pub fn new(timer: *mut Timer32Counter, tag: EventTag) -> Self {
        Timer32Alarm { timer, target_value: 0, enabled: false, event_tag: tag }
    }

    pub fn get_enable(&self) -> bool { self.enabled }

    pub fn set_enable(&mut self, ctx: &mut CpuContext, val: bool, timer_enable: bool) {
        if val != self.enabled {
            self.enabled = val;
            if val && timer_enable { self.schedule(ctx); } else { self.cancel(ctx); }
        }
    }

    pub fn get_target(&self) -> u32 { self.target_value }

    pub fn set_target(&mut self, ctx: &mut CpuContext, val: u32, timer_enable: bool) {
        if val != self.target_value {
            self.target_value = val;
            if self.enabled && timer_enable {
                self.cancel(ctx);
                self.schedule(ctx);
            }
        }
    }

    // JS schedule()
    pub fn schedule(&mut self, ctx: &mut CpuContext) {
        let timer = unsafe { &*self.timer };
        let top = timer.top_value;
        let mode = timer.timer_mode;
        let raw_counter = timer.counter(ctx.apb_ticks());
        let mut reg_val = self.target_value.wrapping_sub(raw_counter);

        if mode == TimerMode::ZigZag && (reg_val as i32) < 0 {
            if (reg_val as i32) < -(top as i32) {
                reg_val = reg_val.wrapping_add(2 * top);
            } else {
                reg_val = (2 * top).wrapping_sub(self.target_value).wrapping_sub(raw_counter);
            }
        }

        if top != 0xFFFF_FFFF {
            if (reg_val as i32) <= 0 {
                reg_val = reg_val.wrapping_add(top + 1);
            }
            if self.target_value > top { return; }
        }

        if mode == TimerMode::Decrement {
            reg_val = top.wrapping_sub(reg_val);
        }

        let arg_val = reg_val & timer.mask_value();
        let timer_ticks = timer.to_ticks(arg_val) as u64;
        ctx.schedule_event(timer_ticks, self.event_tag);
    }

    // JS cancel()
    pub fn cancel(&self, ctx: &mut CpuContext) {
        ctx.unschedule_event(self.event_tag);
    }
}

// ============================================================
// FrcTimerChannel — JS class
// ============================================================

pub struct FrcTimerChannel {
    pub index: u32,
    pub irq: u32,
    pub load_reg: u32,
    pub ctrl_reg: u32,
    pub alarm_reg: u32,
    pub alarm_scheduled: bool,
    pub timer: Timer32Counter,
}

impl FrcTimerChannel {
    // JS constructor(cpuVal, tmpVal, idxVal)
    //   cpuVal = cpu (ignored, ctx passed in methods)
    //   tmpVal = index, idxVal = irq
    pub fn new(index: u32, irq: u32) -> Self {
        let mut timer = Timer32Counter::new();
        let bits = if index == 0 { 23 } else { 32 };
        timer.set_bits(bits);
        FrcTimerChannel {
            index,
            irq,
            load_reg: 0,
            ctrl_reg: 0,
            alarm_reg: 0,
            alarm_scheduled: false,
            timer,
        }
    }

    // JS get prescaler()
    pub fn prescaler(&self) -> u32 {
        match (self.ctrl_reg >> 1) & 7 {
            2 => 16,
            4 => 256,
            _ => 1,
        }
    }

    // JS readUint32(cpuVal)
    pub fn read_u32(&mut self, ctx: &mut CpuContext, addr: u32) -> u32 {
        match addr {
            TIM_REG11 => self.load_reg,
            TIM_REG13 => self.ctrl_reg,
            TIM_REG12 => self.timer.counter(ctx.apb_ticks()),
            TIM_REG15 => self.alarm_reg,
            _ => 0,
        }
    }

    // JS scheduleAlarm()
    pub fn schedule_alarm(&mut self, ctx: &mut CpuContext) {
        if self.ctrl_reg & TIM_REG17 == 0 { return; }
        // FRC1 (index 0) has NO alarm register (alarm reg only present for i==1);
        // its interrupt fires at the countdown zero-cross, i.e. `load` timer ticks
        // after the load. This is what the ROM's ets_timer (used by the wifi lib)
        // relies on.
        let val = if self.index == 0 {
            self.load_reg
        } else {
            (self.alarm_reg.wrapping_sub(self.timer.counter(ctx.apb_ticks()))) & self.timer.mask_value()
        };
        if self.alarm_scheduled {
            ctx.unschedule_event(EventTag::FrcTimerAlarm { channel: self.index });
        }
        ctx.schedule_event(self.timer.to_ticks(val) as u64, EventTag::FrcTimerAlarm { channel: self.index });
        self.alarm_scheduled = true;
    }

    // JS updateInterrupts()
    pub fn update_interrupts(&mut self, ctx: &mut CpuContext) {
        let val = self.ctrl_reg & TIM_REG16;
        ctx.interrupt(self.irq, val != 0);
    }

    // JS writeUint32(cpuVal, tmpVal)
    pub fn write_u32(&mut self, ctx: &mut CpuContext, addr: u32, val: u32) {
        match addr {
            TIM_REG11 => {
                self.load_reg = val & self.timer.mask_value();
                self.timer.set(self.load_reg, ctx.apb_ticks());
                self.schedule_alarm(ctx);
            }
            TIM_REG13 => {
                let old_enable = self.ctrl_reg & TIM_REG17;
                let new_enable = val & TIM_REG17;
                self.ctrl_reg = val;
                // this.timer.prescaler = this.prescaler
                let p = self.prescaler();
                self.timer.set_prescaler(p, ctx.apb_ticks());
                // this.timer.enable = !!idxVal
                self.timer.set_enable(new_enable != 0, ctx.apb_ticks());
                if new_enable != 0 && old_enable == 0 {
                    self.schedule_alarm(ctx);
                }
            }
            TIM_REG14 => {
                if val & 1 != 0 {
                    self.ctrl_reg &= !TIM_REG16;
                    self.update_interrupts(ctx);
                }
            }
            TIM_REG15 => {
                self.alarm_reg = val;
                self.schedule_alarm(ctx);
            }
            _ => {}
        }
    }

    // JS handleAlarm() — the onAlarm closure
    pub fn handle_alarm(&mut self, ctx: &mut CpuContext) {
        self.ctrl_reg |= TIM_REG16;
        self.alarm_scheduled = false;
        if self.ctrl_reg & TIM_REG18 != 0 {
            self.timer.set(self.load_reg, ctx.apb_ticks());
            self.schedule_alarm(ctx);
        }
        self.update_interrupts(ctx);
    }
}

// ============================================================
// FrcTimerPeripheral — JS class extends PeripheralBase
// ============================================================

pub struct FrcTimerPeripheral {
    pub base: PeripheralBase,
    pub irq: [u32; 2],
    pub timers: [FrcTimerChannel; 2],
}

impl FrcTimerPeripheral {
    // JS constructor(cpuVal, tmpVal, idxVal, ClockEvent)
    //   cpuVal = cpu, tmpVal = baseAddr, idxVal = name, ClockEvent = irq[2]
    pub fn new(base_addr: u32, name: &'static str, irq0: u32, irq1: u32) -> Self {
        FrcTimerPeripheral {
            base: PeripheralBase::new(base_addr, name),
            irq: [irq0, irq1],
            timers: [
                FrcTimerChannel::new(0, irq0),
                FrcTimerChannel::new(1, irq1),
            ],
        }
    }

    // JS readUint32(cpuVal)
    pub fn read_u32(&mut self, ctx: &mut CpuContext, addr: u32) -> u32 {
        let offset = addr.wrapping_sub(self.base.base_addr);
        if offset < 32 {
            self.timers[0].read_u32(ctx, offset)
        } else if offset < 64 {
            self.timers[1].read_u32(ctx, offset - 32)
        } else {
            self.base.read_uint32(addr)
        }
    }

    // JS writeUint32(cpuVal, tmpVal)
    pub fn write_u32(&mut self, ctx: &mut CpuContext, addr: u32, val: u32) {
        self.base.write_uint32(addr, val);
        let offset = addr.wrapping_sub(self.base.base_addr);
        if offset < 32 {
            self.timers[0].write_u32(ctx, offset, val);
        } else if offset < 64 {
            self.timers[1].write_u32(ctx, offset - 32, val);
        } else {
            self.base.write_uint32(addr, val);
        }
    }

    // JS reset()
    pub fn reset(&mut self) {
        self.base.reset();
    }

    pub fn handle_event(&mut self, ctx: &mut CpuContext, tag: EventTag) {
        if let EventTag::FrcTimerAlarm { channel } = tag {
            let ch = channel as usize;
            if ch < 2 {
                self.timers[ch].handle_alarm(ctx);
            }
        }
    }
}

impl MmioPeripheral for FrcTimerPeripheral {
    fn read_u32(&mut self, ctx: &mut CpuContext, addr: u32) -> u32 {
        self.read_u32(ctx, addr)
    }
    fn write_u32(&mut self, ctx: &mut CpuContext, addr: u32, val: u32) {
        self.write_u32(ctx, addr, val);
    }
    fn reset(&mut self) {
        self.reset();
    }
}

// ============================================================
// Timer64Counter — JS class
// ============================================================

pub struct Timer64Counter {
    pub bits: u32,
    pub prescaler: u32,
    pub base_low: u32,
    pub base_high: u32,
    pub base_ticks: u64,
    pub timer_mode: TimerMode,
    pub enabled: bool,
    pub high_mask: u32,
}

impl Timer64Counter {
    // JS constructor(cpuVal, tmpVal = 64)
    pub fn new(bits: u32) -> Self {
        if bits < 32 || bits > 64 {
            panic!("Invalid number of bits");
        }
        let high_mask = if bits == 64 { 0xFFFF_FFFF } else { (1u32 << (bits - 32)) - 1 };
        Timer64Counter {
            bits,
            prescaler: 1,
            base_low: 0,
            base_high: 0,
            base_ticks: 0,
            timer_mode: TimerMode::Increment,
            enabled: false,
            high_mask,
        }
    }

    // JS reset()
    pub fn reset(&mut self) {
        self.base_low = 0;
        self.base_high = 0;
    }

    // JS set(cpuVal, tmpVal) — cpuVal = high, tmpVal = low
    pub fn set(&mut self, high: u32, low: u32, current_ticks: u64) {
        self.base_high = high & self.high_mask;
        self.base_low = low;
        self.base_ticks = current_ticks;
    }

    // JS get counter()
    pub fn counter(&self, current_ticks: u64) -> f64 {
        if self.prescaler == 0 || !self.enabled { return 0.0; }
        let delta = (current_ticks.wrapping_sub(self.base_ticks)) as f64 / self.prescaler as f64;
        if self.timer_mode == TimerMode::Increment { delta } else { -delta }
    }

    // JS get enable()
    pub fn get_enable(&self) -> bool { self.enabled }

    // JS set enable(cpuVal)
    pub fn set_enable(&mut self, val: bool, current_ticks: u64) {
        if val != self.enabled {
            if val {
                self.base_ticks = current_ticks;
            } else {
                let l = self.low(current_ticks);
                let h = self.high(current_ticks);
                self.base_low = l;
                self.base_high = h;
            }
            self.enabled = val;
        }
    }

    // JS get mode()
    pub fn get_mode(&self) -> TimerMode { self.timer_mode }

    // JS set mode(cpuVal)
    pub fn set_mode(&mut self, val: TimerMode, current_ticks: u64) {
        if self.timer_mode != val {
            let h = self.high(current_ticks);
            let l = self.low(current_ticks);
            self.timer_mode = val;
            self.set(h, l, current_ticks);
        }
    }

    // JS get high()
    // JS: (Math.trunc((this.counter + this.baseLow) / 0x100000000) + this.baseHigh) & this.highMask
    pub fn high(&self, current_ticks: u64) -> u32 {
        let total = self.counter(current_ticks) + self.base_low as f64;
        let high_part = (total / 4294967296.0) as i64;
        let high_raw = high_part.wrapping_add(self.base_high as i64);
        (high_raw as u64 & self.high_mask as u64) as u32
    }

    // JS get low()
    // JS: ((this.counter >>> 0) + this.baseLow) >>> 0
    pub fn low(&self, current_ticks: u64) -> u32 {
        let c = self.counter(current_ticks);
        let c_u32 = if c < 0.0 {
            let abs = (-c) as u64;
            abs.wrapping_neg() as u32
        } else {
            c as u32
        };
        c_u32.wrapping_add(self.base_low)
    }

    // JS get frequency()
    pub fn frequency(&self, _current_ticks: u64) -> f64 {
        0.0 // stub — clock frequency not directly available
    }

    // JS toTicks(cpuVal, tmpVal) — high, low
    pub fn to_ticks(&self, high: u32, low: u32) -> u64 {
        (((high as u64) << 32) | low as u64) * self.prescaler as u64
    }

    // JS addListener/removeListener — no-op in Rust (handled via EventQueue)
}

// ============================================================
// Timer64Alarm — JS class
// ============================================================

pub struct Timer64Alarm {
    pub timer: *mut Timer64Counter,
    pub trigger_on_past_alarm: bool,
    pub high_value: u32,
    pub low_value: u32,
    pub enabled: bool,
    pub event_tag: EventTag,
}

impl Timer64Alarm {
    // JS constructor(cpuVal, tmpVal, idxVal = false)
    //   cpuVal = Timer64Counter, tmpVal = callback (replaced by event_tag)
    pub fn new(timer: *mut Timer64Counter, tag: EventTag, trigger_on_past_alarm: bool) -> Self {
        Timer64Alarm {
            timer,
            trigger_on_past_alarm,
            high_value: 0,
            low_value: 0,
            enabled: false,
            event_tag: tag,
        }
    }

    // JS get low()
    pub fn get_low(&self) -> u32 { self.low_value }

    // JS set low(cpuVal)
    pub fn set_low(&mut self, ctx: &mut CpuContext, val: u32) {
        self.low_value = val;
        self.reschedule(ctx);
    }

    // JS get high()
    pub fn get_high(&self, timer_high_mask: u32) -> u32 { self.high_value & timer_high_mask }

    // JS set high(cpuVal)
    pub fn set_high(&mut self, ctx: &mut CpuContext, val: u32, timer_high_mask: u32) {
        self.high_value = val & timer_high_mask;
        self.reschedule(ctx);
    }

    // JS get enable()
    pub fn get_enable(&self) -> bool { self.enabled }

    // JS set enable(cpuVal)
    pub fn set_enable(&mut self, ctx: &mut CpuContext, val: bool, timer_enable: bool) {
        if val != self.enabled {
            self.enabled = val;
            if val && timer_enable { self.schedule(ctx); } else { self.cancel(ctx); }
        }
    }

    // JS schedule()
    pub fn schedule(&mut self, ctx: &mut CpuContext) {
        let timer = unsafe { &*self.timer };
        let direction: i64 = if timer.timer_mode == TimerMode::Increment { 1 } else { -1 };
        let mut idx = direction * (self.low_value as i64 - timer.low(ctx.apb_ticks()) as i64);
        let clock_event = idx as u32;
        let mut simulation_clock = direction * (self.high_value as i64 - timer.high(ctx.apb_ticks()) as i64);

        if clock_event as i64 != idx {
            if idx < 0 { simulation_clock -= 1; } else { simulation_clock += 1; }
        }
        simulation_clock = (simulation_clock as u32 & timer.high_mask) as i64;

        if self.trigger_on_past_alarm && sign_extend(simulation_clock as u32, timer.bits - 32) < 0 {
            ctx.schedule_event(0, self.event_tag);
            return;
        }
        let reg_val = timer.to_ticks(simulation_clock as u32, clock_event);
        ctx.schedule_event(reg_val, self.event_tag);
    }

    // JS cancel()
    pub fn cancel(&self, ctx: &mut CpuContext) {
        ctx.unschedule_event(self.event_tag);
    }

    // JS reschedule()
    pub fn reschedule(&mut self, ctx: &mut CpuContext) {
        let timer = unsafe { &*self.timer };
        if self.enabled && timer.enabled {
            self.cancel(ctx);
            self.schedule(ctx);
        }
    }
}

// ============================================================
// TimerGroupChannel — JS class
// ============================================================

pub struct TimerGroupChannel {
    pub xtal_clock_sel: bool,
    pub bits: u32,
    pub config: u32,
    pub lo_value: u32,
    pub hi_value: u32,
    pub load_lo: u32,
    pub load_hi: u32,
    pub timer: Timer64Counter,
    pub alarm: Timer64Alarm,
    pub event_tag: EventTag,
    pub alarm_callback_tag: u32, // bit mask (DS_REG37 or DS_REG38)
}

impl TimerGroupChannel {
    // JS constructor(cpuVal, tmpVal, idxVal, ClockEvent, SimulationClock)
    //   cpuVal = cpu (replaced by ctx), tmpVal = alarmCallback (bit mask)
    //   idxVal = xtalClockSel, ClockEvent = clock, SimulationClock = bits
    pub fn new(
        alarm_callback_tag: u32,
        xtal_clock_sel: bool,
        bits: u32,
        alarm_tag: EventTag,
    ) -> Self {
        let timer = Timer64Counter::new(bits);
        TimerGroupChannel {
            xtal_clock_sel,
            bits,
            config: 0,
            lo_value: 0,
            hi_value: 0,
            load_lo: 0,
            load_hi: 0,
            timer,
            alarm: Timer64Alarm::new(core::ptr::null_mut(), alarm_tag, true),
            event_tag: alarm_tag,
            alarm_callback_tag,
        }
    }

    // Called after construction to set up self-referential pointer
    pub fn init_alarm_timer(&mut self) {
        self.alarm.timer = &mut self.timer as *mut Timer64Counter;
    }

    // JS readUint32(cpuVal)
    pub fn read_u32(&mut self, ctx: &mut CpuContext, addr: u32) -> u32 {
        match addr {
            ID => self.config,
            DS_REG23 => self.lo_value,
            DS_REG24 => self.hi_value,
            DS_REG25 => 0,
            DS_REG26 => self.alarm.low_value,
            DS_REG27 => self.alarm.high_value,
            TIMER_LOAD_LO_OFFSET => self.load_lo,
            DS_REG28 => self.load_hi,
            _ => 0,
        }
    }

    // JS writeUint32(cpuVal, tmpVal)
    pub fn write_u32(&mut self, ctx: &mut CpuContext, addr: u32, val: u32) {
        let timer = &mut self.timer;
        match addr {
            ID => {
                self.config = val;
                timer.set_enable((val & DS_REG30) != 0, ctx.apb_ticks());
                timer.timer_mode = if val & DS_REG31 != 0 { TimerMode::Increment } else { TimerMode::Decrement };
                let prescaler_val = prescaler_to_divider(read_field_value(val, DS_REG33, DS_REG34));
                timer.prescaler = prescaler_val;
                self.alarm.set_enable(ctx, (val & DS_REG35) != 0, timer.enabled);
                // Clock selection is handled externally — caller must route ticks
            }
            DS_REG25 => {
                self.lo_value = timer.low(ctx.apb_ticks());
                self.hi_value = timer.high(ctx.apb_ticks());
            }
            DS_REG26 => {
                self.alarm.set_low(ctx, val);
            }
            DS_REG27 => {
                self.alarm.set_high(ctx, val, self.timer.high_mask);
            }
            TIMER_LOAD_LO_OFFSET => {
                self.load_lo = val;
            }
            DS_REG28 => {
                self.load_hi = val;
            }
            DS_REG29 => {
                timer.set(self.load_hi, self.load_lo, ctx.apb_ticks());
            }
            _ => {}
        }
    }

    // JS reset()
    pub fn reset(&mut self, ctx: &mut CpuContext) {
        // JS: this.writeUint32(ID, dsReg31 | dsReg32 | (1 << dsReg33))
        self.write_u32(ctx, ID, DS_REG31 | DS_REG32 | (1 << DS_REG33));
    }

    // JS onAlarm() — handles timer alarm
    pub fn on_alarm(&mut self, ctx: &mut CpuContext, cb_bits: u32) {
        // from JS: this.onAlarm = () => {
        //   this.alarm.enable = false;
        //   this.config &= ~dsReg35;
        //   this.config & dsReg32 && this.timer.set(this.loadHi, this.loadLo);
        //   this.alarmCallback();
        // }
        self.alarm.enabled = false;
        self.config &= !DS_REG35;
        if self.config & DS_REG32 != 0 {
            self.timer.set(self.load_hi, self.load_lo, ctx.apb_ticks());
        }
        // alarmCallback is handled externally via cb_bits
    }
}

// ============================================================
// TimerGroupPeripheral — JS class extends PeripheralBase
// ============================================================

pub struct TimerGroupPeripheral {
    pub base: PeripheralBase,
    pub timers: [TimerGroupChannel; 2],
    pub lact: f64,
    pub int_raw: u32,
    pub int_enable: u32,
    pub wdt_current_stage: u32,
    pub wdt_write_protect: bool,
    pub wdt_int_mask: u32,
    pub group: u32, // 0 = TIMG0, 1 = TIMG1 (selects EventTag namespace)
    pub irq_t0: u32,
    pub irq_t1: i32,
    pub irq_wdt: u32,
    pub irq_lact: i32,
    pub lact_divisor: u32,
    pub wdt_reset_reason: Option<u32>,
    pub wdt_clock_freq: u32,
    pub xtal_clock_sel: bool,
    pub bits: u32,
    pub has_wdt_use_xtal: bool,
    pub regs: TimgRegOffsets,
    pub fields: TimgFieldDescs,
    pub cal_clocks: [u32; 4],
}

pub struct TimgRegOffsets {
    pub int_raw: u32,
    pub int_ena: u32,
    pub int_st: u32,
    pub int_clr: u32,
    pub wdtconfig0: u32,
    pub wdtconfig1: u32,
    pub wdtconfig2: u32,
    pub wdtconfig3: u32,
    pub wdtconfig4: u32,
    pub wdtconfig5: u32,
    pub wdtfeed: u32,
    pub wdtwprotect: u32,
    pub lactconfig: u32,
    pub lactupdate: u32,
    pub lactlo: u32,
    pub lacthi: u32,
    pub lactalarmlo: u32,
    pub lactalarmhi: u32,
    pub rtccalicfg: u32,
}

impl TimerGroupPeripheral {
    pub fn new(
        group: u32,
        base_addr: u32,
        name: &'static str,
        config: TimgConfig,
        irq_t0: u32,
        irq_t1: i32,
        irq_wdt: u32,
        irq_lact: i32,
    ) -> Self {
        let bits = config.bits;
        let (tag_t0, tag_t1) = if group == 1 {
            (EventTag::Timg1Timer0Alarm, EventTag::Timg1Timer1Alarm)
        } else {
            (EventTag::Timg0Timer0Alarm, EventTag::Timg0Timer1Alarm)
        };
        let ch0 = TimerGroupChannel::new(DS_REG37, config.xtal_clock, config.bits, tag_t0);
        let ch1 = TimerGroupChannel::new(DS_REG38, config.xtal_clock, config.bits, tag_t1);
        // NOTE: init_alarm_timer is NOT called here — self-referential pointers must be
        // fixed up AFTER the struct reaches its final memory location (see init_alarms()).
        TimerGroupPeripheral {
            base: PeripheralBase::new(base_addr, name),
            timers: [ch0, ch1],
            lact: 0.0,
            int_raw: 0,
            int_enable: 0,
            wdt_current_stage: 0,
            wdt_write_protect: false,
            wdt_int_mask: 1 << config.fields.wdt_int_raw.shift,
            group,
            irq_t0,
            irq_t1,
            irq_wdt,
            irq_lact,
            lact_divisor: 1,
            wdt_reset_reason: config.wdt_reset_reason,
            wdt_clock_freq: config.wdt_clock_freq,
            xtal_clock_sel: config.xtal_clock,
            bits,
            has_wdt_use_xtal: config.has_wdt_use_xtal,
            regs: config.regs,
            fields: config.fields,
            cal_clocks: config.cal_clocks,
        }
    }

    // EventTag for this group's WDT stage (Timg0Wdt vs Timg1Wdt)
    pub fn wdt_tag(&self, stage: u32) -> EventTag {
        if self.group == 1 {
            EventTag::Timg1Wdt { stage }
        } else {
            EventTag::Timg0Wdt { stage }
        }
    }

    // EventTag for this group's LACT alarm (Timg0LactAlarm vs Timg1LactAlarm)
    pub fn lact_tag(&self) -> EventTag {
        if self.group == 1 {
            EventTag::Timg1LactAlarm
        } else {
            EventTag::Timg0LactAlarm
        }
    }

    // Fix self-referential alarm.timer pointers AFTER struct is in final position
    pub fn init_alarms(&mut self) {
        self.timers[0].alarm.timer = &mut self.timers[0].timer as *mut Timer64Counter;
        self.timers[1].alarm.timer = &mut self.timers[1].timer as *mut Timer64Counter;
    }

    // JS get wdtConfig0()
    pub fn wdt_config0(&self) -> u32 {
        self.base.read_register(self.regs.wdtconfig0)
    }

    // JS get wdtEnabled()
    pub fn wdt_enabled(&self) -> bool {
        (self.wdt_config0() & 0x80000000) != 0
    }

    // JS get wdtFlashbootEnabled()
    pub fn wdt_flashboot_enabled(&self) -> bool {
        (self.wdt_config0() & 16384) != 0
    }

    // JS get wdtIntMask()
    pub fn wdt_int_mask(&self) -> u32 { self.wdt_int_mask }

    // JS getWdtStageAction(cpuVal)
    pub fn get_wdt_stage_action(&self, stage: u32) -> u32 {
        let val = self.wdt_config0();
        match stage {
            0 => (val >> 29) & 3,
            1 => (val >> 27) & 3,
            2 => (val >> 25) & 3,
            3 => (val >> 23) & 3,
            _ => DS_REG40,
        }
    }

    // JS getWdtStageTimeout(cpuVal)
    pub fn get_wdt_stage_timeout(&self, stage: u32) -> u32 {
        let offset = match stage {
            0 => self.regs.wdtconfig2,
            1 => self.regs.wdtconfig3,
            2 => self.regs.wdtconfig4,
            3 => self.regs.wdtconfig5,
            _ => return 0,
        };
        self.base.read_register(offset)
    }

    // JS getWdtPrescaler()
    pub fn get_wdt_prescaler(&self) -> u32 {
        (self.base.read_register(self.regs.wdtconfig1) >> 16) & 65535
    }

    // JS scheduleWdtStage()
    pub fn schedule_wdt_stage(&mut self, ctx: &mut CpuContext) {
        ctx.unschedule_event(self.wdt_tag(self.wdt_current_stage));
        if !self.wdt_enabled() && !self.wdt_flashboot_enabled() { return; }
        if self.get_wdt_stage_action(self.wdt_current_stage) == DS_REG40 {
            for i in (self.wdt_current_stage + 1)..=3 {
                if self.get_wdt_stage_action(i) != DS_REG40 {
                    self.wdt_current_stage = i;
                    self.schedule_wdt_stage(ctx);
                    break;
                }
            }
            return;
        }
        let timeout = self.get_wdt_stage_timeout(self.wdt_current_stage);
        let prescaler = self.get_wdt_prescaler();
        let prescaler_val = if prescaler == 0 { 1 } else { prescaler };
        let mut freq = self.wdt_clock_freq;
        // JS: if wdtClock is null, use APB or XTAL based on WDT_USE_XTAL bit
        if freq == 0 {
            freq = ctx.clocks.apb.frequency;
            if let Some(field) = &self.fields.wdt_use_xtal {
                if self.base.read_field(field) != 0 {
                    freq = ctx.clocks.xtal.frequency;
                }
            }
        }
        let delta_ticks = (timeout as u64) * (prescaler_val as u64);
        let nanos = if freq != 0 {
            (delta_ticks as u128 * 1_000_000_000 / freq as u128) as u64
        } else {
            delta_ticks
        };
        ctx.schedule_event(nanos, self.wdt_tag(self.wdt_current_stage));
    }

    // JS feedWdt()
    pub fn feed_wdt(&mut self, ctx: &mut CpuContext) {
        self.wdt_current_stage = 0;
        self.schedule_wdt_stage(ctx);
    }

    // JS get divisor()
    pub fn divisor(&self) -> u32 {
        let lact_config = self.base.read_register(self.regs.lactconfig);
        prescaler_to_divider(read_field_value(lact_config, DS_REG33, DS_REG34))
    }

    // JS getTimerValue()
    pub fn get_timer_value(&self, ctx: &CpuContext) -> f64 {
        (ctx.clock_nanos() as f64 / 1e6) * ctx.clocks.apb.frequency as f64 / 1e3 / self.divisor() as f64
    }

    // JS readUint32(cpuVal)
    pub fn read_u32(&mut self, ctx: &mut CpuContext, addr: u32) -> u32 {
        let offset = addr.wrapping_sub(self.base.base_addr);
        if offset < 36 { return self.timers[0].read_u32(ctx, offset); }
        if offset < 72 { return self.timers[1].read_u32(ctx, offset - 36); }
        match offset {
            a if a == self.regs.lactlo => self.lact as u32,
            a if a == self.regs.lacthi => (self.lact / (0x1_0000_0000u64 as f64)) as u32,
            a if a == self.regs.int_raw => self.int_raw,
            a if a == self.regs.int_ena => self.int_enable,
            a if a == self.regs.int_st => self.int_raw & self.int_enable,
            _ => self.base.read_uint32(addr),
        }
    }

    // JS writeUint32(cpuVal, tmpVal)
    pub fn write_u32(&mut self, ctx: &mut CpuContext, addr: u32, val: u32) {
        self.base.write_uint32(addr, val);
        let offset = addr.wrapping_sub(self.base.base_addr);
        if offset < 36 { self.timers[0].write_u32(ctx, offset, val); return; }
        if offset < 72 { self.timers[1].write_u32(ctx, offset - 36, val); return; }
        match offset {
            a if a == self.regs.wdtconfig0 => {
                if self.wdt_write_protect { return; }
                let was_enabled = self.wdt_enabled() || self.wdt_flashboot_enabled();
                self.base.write_uint32(addr, val);
                let now_enabled = self.wdt_enabled() || self.wdt_flashboot_enabled();
                if now_enabled && !was_enabled {
                    self.wdt_current_stage = 0;
                    self.schedule_wdt_stage(ctx);
                } else if !now_enabled && was_enabled {
                    ctx.unschedule_event(self.wdt_tag(self.wdt_current_stage));
                } else if now_enabled {
                    self.schedule_wdt_stage(ctx);
                }
            }
            a if a == self.regs.wdtconfig1
              || a == self.regs.wdtconfig2
              || a == self.regs.wdtconfig3
              || a == self.regs.wdtconfig4
              || a == self.regs.wdtconfig5 => {
                if self.wdt_write_protect { return; }
            }
            a if a == self.regs.wdtfeed => {
                if self.wdt_write_protect { return; }
                self.feed_wdt(ctx);
                return;
            }
            a if a == self.regs.wdtwprotect => {
                self.wdt_write_protect = val != DS_REG44;
            }
            a if a == self.regs.lactconfig => {
                if val & DS_REG35 != 0 { self.update_lact_alarm(ctx); }
            }
            a if a == self.regs.lactupdate => {
                if val != 0 { self.lact = self.get_timer_value(ctx); }
                return;
            }
            a if a == self.regs.rtccalicfg => {
                // JS: this.writeRegister(RTCCALICFG, timReg5(timReg5(val, RTC_CALI_START), RTC_CALI_START_CYCLING))
                let f = &self.fields;
                let cleared = clear_field_val(clear_field_val(val, &f.rtc_cali_start), &f.rtc_cali_start_cycling);
                self.base.write_register(self.regs.rtccalicfg, cleared);
                let start_cycling = read_field_val(val, &f.rtc_cali_start_cycling);
                if read_field_val(val, &f.rtc_cali_start) != 0 || start_cycling != 0 {
                    let clk_sel = read_field_val(val, &f.rtc_cali_clk_sel);
                    let cal_max = read_field_val(val, &f.rtc_cali_max);
                    let clk_idx = clk_sel as usize;
                    let cal_clk_freq = if clk_idx < 4 && self.cal_clocks[clk_idx] != 0 {
                        self.cal_clocks[clk_idx]
                    } else {
                        ctx.clocks.apb.frequency
                    };
                    let cal_value = ((cal_max as u64) * (ctx.clocks.xtal.frequency as u64)) / (cal_clk_freq as u64);
                    self.base.write_field(&f.rtc_cali_value, cal_value as u32);
                    self.base.write_field(&f.rtc_cali_rdy, 1);
                    if start_cycling != 0 {
                        if let Some(cycling_vld) = &f.rtc_cali_cycling_data_vld {
                            self.base.write_field(cycling_vld, 1);
                        }
                    }
                }
                return;
            }
            a if a == self.regs.int_ena => {
                self.int_enable = val;
                self.update_interrupts(ctx);
            }
            a if a == self.regs.int_clr => {
                self.int_raw &= !val;
                self.update_interrupts(ctx);
                return;
            }
            _ => { self.base.write_uint32(addr, val); }
        }
    }

    // JS updateLactAlarm()
    // JS: ((0x100000000 * LACTALARMHI + LACTALARMLO - this.lact) / apbFreq) * 1e9 * divisor
    pub fn update_lact_alarm(&mut self, ctx: &mut CpuContext) {
        let alarm_lo = self.base.read_register(self.regs.lactalarmlo);
        let alarm_hi = self.base.read_register(self.regs.lactalarmhi);
        let alarm_val = ((alarm_hi as u64) << 32 | alarm_lo as u64) as f64;
        let diff = alarm_val - self.lact; // f64, may be negative (JS same)
        let freq = ctx.clocks.apb.frequency as f64;
        let delta_nanos = if freq != 0.0 {
            (diff / freq) * 1e9 * self.divisor() as f64
        } else { 0.0 };
        let delta = if delta_nanos <= 0.0 { 0 } else { delta_nanos as u64 };
        ctx.schedule_event(delta, self.lact_tag());
    }

    // JS onAlarm(cpuVal)
    pub fn on_alarm(&mut self, ctx: &mut CpuContext, bits: u32) {
        self.int_raw |= bits;
        self.update_interrupts(ctx);
    }

    // JS updateInterrupts()
    pub fn update_interrupts(&mut self, ctx: &mut CpuContext) {
        let status = self.int_raw & self.int_enable;
        if self.irq_lact >= 0 {
            ctx.interrupt(self.irq_lact as u32, (status & DS_REG39) != 0);
        }
    }

    // JS reset()
    pub fn reset(&mut self, ctx: &mut CpuContext) {
        self.base.reset();
        self.int_raw = 0;
        self.int_enable = 0;
        ctx.unschedule_event(self.wdt_tag(self.wdt_current_stage));
        self.wdt_current_stage = 0;
        self.wdt_write_protect = false;
        self.timers[0].reset(ctx);
        self.timers[1].reset(ctx);
    }

    // Handle an EventQueue event for THIS group (Timg0 or Timg1 namespace).
    // The tag is normalized to its Timg0 equivalent so one code path serves both.
    pub fn handle_group_event(&mut self, ctx: &mut CpuContext, tag: EventTag) {
        let normalized = match tag {
            EventTag::Timg1Timer0Alarm => EventTag::Timg0Timer0Alarm,
            EventTag::Timg1Timer1Alarm => EventTag::Timg0Timer1Alarm,
            EventTag::Timg1Wdt { stage } => EventTag::Timg0Wdt { stage },
            EventTag::Timg1LactAlarm => EventTag::Timg0LactAlarm,
            other => other,
        };
        self.handle_event(ctx, normalized);
    }

    pub fn handle_event(&mut self, ctx: &mut CpuContext, tag: EventTag) {
        match tag {
            EventTag::Timg0Timer0Alarm => {
                self.timers[0].on_alarm(ctx, DS_REG37);
                // alarm callback: reschedule if needed — handled by Timer64Alarm.reschedule
                self.on_alarm(ctx, DS_REG37);
            }
            EventTag::Timg0Timer1Alarm => {
                self.timers[1].on_alarm(ctx, DS_REG38);
                self.on_alarm(ctx, DS_REG38);
            }
            EventTag::Timg0Wdt { stage } => {
                match self.get_wdt_stage_action(stage) {
                    DS_REG41 => {
                        self.int_raw |= self.wdt_int_mask;
                        self.update_interrupts(ctx);
                    }
                    DS_REG42 => {
                        if let Some(reason) = self.wdt_reset_reason {
                            ctx.set_reset_reason(reason);
                        }
                        ctx.zero_sensitive_memory();
                        if ctx.on_reset() { ctx.reset_core(0); }
                        return;
                    }
                    DS_REG43 => {
                        if let Some(reason) = self.wdt_reset_reason {
                            ctx.set_reset_reason(reason);
                        }
                        ctx.zero_sensitive_memory();
                        if ctx.on_reset() { ctx.reset_soc(); }
                        return;
                    }
                    _ => {}
                }
                self.wdt_current_stage += 1;
                if self.wdt_current_stage <= 3 {
                    self.schedule_wdt_stage(ctx);
                }
            }
            EventTag::Timg0LactAlarm => {
                self.on_alarm(ctx, DS_REG39);
            }
            _ => {}
        }
    }
}

// ============================================================
// TimgConfig — configuration for TimerGroupPeripheral
// ============================================================

pub struct TimgFieldDescs {
    pub wdt_int_raw: FieldDesc,
    pub wdt_use_xtal: Option<FieldDesc>,
    pub rtc_cali_start: FieldDesc,
    pub rtc_cali_start_cycling: FieldDesc,
    pub rtc_cali_clk_sel: FieldDesc,
    pub rtc_cali_max: FieldDesc,
    pub rtc_cali_value: FieldDesc,
    pub rtc_cali_rdy: FieldDesc,
    pub rtc_cali_cycling_data_vld: Option<FieldDesc>,
}

pub struct TimgConfig {
    pub regs: TimgRegOffsets,
    pub fields: TimgFieldDescs,
    pub xtal_clock: bool,
    pub bits: u32,
    pub wdt_reset_reason: Option<u32>,
    pub wdt_clock_freq: u32,
    pub has_wdt_use_xtal: bool,
    pub cal_clocks: [u32; 4], // RTC calibration clock frequencies (Hz), indexed by clk_sel
}

impl MmioPeripheral for TimerGroupPeripheral {
    fn read_u32(&mut self, ctx: &mut CpuContext, addr: u32) -> u32 {
        self.read_u32(ctx, addr)
    }
    fn write_u32(&mut self, ctx: &mut CpuContext, addr: u32, val: u32) {
        self.write_u32(ctx, addr, val);
    }
    fn reset(&mut self) {
        // reset requires ctx — handled externally
    }
}

// ============================================================
// CircularFifoBuffer — JS class
// ============================================================

pub struct CircularFifoBuffer {
    pub start: usize,
    pub used: usize,
    pub buffer: [u32; 4096], // fixed max size
    pub capacity: usize,
}

impl CircularFifoBuffer {
    // JS constructor(cpuVal) — cpuVal = size
    pub fn new(capacity: usize) -> Self {
        CircularFifoBuffer { start: 0, used: 0, buffer: [0u32; 4096], capacity: cmp::min(capacity, 4096) }
    }

    // JS get size()
    pub fn size(&self) -> usize { self.capacity }

    // JS get itemCount()
    pub fn item_count(&self) -> usize { self.used }

    // JS push(cpuVal)
    pub fn push(&mut self, val: u32) {
        if self.used < self.capacity {
            self.buffer[(self.start + self.used) % self.capacity] = val;
            self.used += 1;
        }
    }

    // JS pushByte(cpuVal)
    pub fn push_byte(&mut self, val: u32) {
        self.push(val & 255);
    }

    // JS pull()
    pub fn pull(&mut self) -> u32 {
        if self.used == 0 { return 0; }
        let val = self.buffer[self.start];
        self.start = (self.start + 1) % self.capacity;
        self.used -= 1;
        val
    }

    // JS peek(cpuVal = 0)
    pub fn peek(&self, offset: u32) -> u32 {
        let off = offset as usize;
        if off < self.used {
            self.buffer[(self.start + off) % self.capacity]
        } else {
            0
        }
    }

    // JS reset()
    pub fn reset(&mut self) {
        self.used = 0;
    }

    // JS get empty()
    pub fn empty(&self) -> bool { self.used == 0 }

    // JS get available()
    pub fn available(&self) -> usize { self.capacity - self.used }

    // JS get full()
    pub fn full(&self) -> bool { self.used == self.capacity }

    // JS get items()
    pub fn items(&self, dest: &mut [u32]) -> usize {
        let count = cmp::min(self.used, dest.len());
        for i in 0..count {
            dest[i] = self.buffer[(self.start + i) % self.capacity];
        }
        count
    }
}
