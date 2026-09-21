// Translated from: src/peripherals/common/clocks.js
// Do NOT modify the logic — match the JS line-for-line
// NOTE: no_std — no Vec, String, HashMap — fixed arrays + Copy types

use crate::peripherals::types::*;

// ============================================================
// Constants (match JS let declarations)
// ============================================================

pub const MS_TO_NS_FACTOR: u64 = 1_000;
pub const US_TO_NS_FACTOR: u64 = 1_000_000;
pub const MAX_PENDING_EVENTS: usize = 32;
pub const MAX_FREQ_LISTENERS: usize = 32;
pub const MAX_ENABLE_LISTENERS: usize = 32;

// ============================================================
// CpuClockSource — matches JS IIFE enum
// ============================================================

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CpuClockSource {
    XtlClk = 0,
    PllClk = 1,
    Rtc8mClk = 2,
    ApllClk = 3,
}

// ============================================================
// ClockPendingEvent — fixed-array entry for enable-listener reschedule
// ============================================================

#[derive(Clone, Copy)]
pub struct ClockPendingEvent {
    pub tag: EventTag,
    pub scheduled_target_ticks: u64,
    pub valid: bool,
}

// ============================================================
// ClockScheduledEvent — matches JS class
// ============================================================

pub struct ClockScheduledEvent {
    pub scheduled_target_ticks: Option<u64>,
    pub event_tag: EventTag,
}

impl ClockScheduledEvent {
    pub fn new(tag: EventTag) -> Self {
        ClockScheduledEvent { scheduled_target_ticks: None, event_tag: tag }
    }

    /// JS reschedule()
    pub fn reschedule(&mut self, ctx: &mut CpuContext, current_ticks: u64, freq: u32) {
        if let Some(target) = self.scheduled_target_ticks {
            let remaining = target.wrapping_sub(current_ticks);
            if remaining == 0 || remaining > target {
                ctx.schedule_event(0, self.event_tag);
            } else {
                let nanos = (remaining as u128 * 1_000_000_000 / freq as u128) as u64;
                ctx.schedule_event(nanos, self.event_tag);
            }
        }
    }

    /// JS clearPending() — removes enable listener and clears target
    pub fn clear_pending(&mut self, clock: &mut RootClock) {
        if self.scheduled_target_ticks.is_some() {
            RootClock::remove_pending(clock, self.event_tag);
            self.scheduled_target_ticks = None;
        }
    }

    /// JS schedule(cpuVal) — cpuVal = delta_ticks
    pub fn schedule(&mut self, ctx: &mut CpuContext, clock: &mut RootClock,
                     current_ticks: u64, delta_ticks: u64, freq: u32) {
        let target = current_ticks + delta_ticks;

        // JS: (undefined === this.scheduledTargetTicks && this.clock.addEnableListener(this.onEnableChange), ...)
        if self.scheduled_target_ticks.is_none() {
            // pending_events replaces JS enable-listener registration
            RootClock::add_pending(clock, ClockPendingEvent {
                tag: self.event_tag, scheduled_target_ticks: target, valid: true,
            });
        } else {
            RootClock::update_pending_target(clock, self.event_tag, target);
        }

        self.scheduled_target_ticks = Some(target);

        // JS: let tmpVal = (1e9 * cpuVal) / this.clock.frequency; this.rootEvent.schedule(tmpVal);
        let nanos = (delta_ticks as u128 * 1_000_000_000 / freq as u128) as u64;
        ctx.schedule_event(nanos, self.event_tag);
    }

    /// JS unschedule()
    pub fn unschedule(&mut self, ctx: &mut CpuContext, clock: &mut RootClock) {
        self.clear_pending(clock);
        ctx.unschedule_event(self.event_tag);
    }
}

// ============================================================
// RootClock — matches JS class (base for all clocks)
// ============================================================

/// Listener types — standalone functions dispatched when frequency/enable changes.
/// `listener_id` is an opaque token registered by the caller (e.g. a clock-tree index).
pub type FreqListenerFn = fn(ctx: &mut CpuContext, listener_id: u32, new_freq: u32, old_freq: u32);
pub type EnableListenerFn = fn(ctx: &mut CpuContext, listener_id: u32, enabled: bool);

#[derive(Clone, Copy)]
pub struct RootClock {
    pub frequency_value: u32,
    pub base_ticks: u64,
    pub base_nanos: u64,
    pub enabled_value: bool,
    // — frequency listeners (matches JS this.listeners = new Set()) —
    pub freq_listener_ids: [u32; MAX_FREQ_LISTENERS],
    pub freq_listener_dispatch: Option<FreqListenerFn>,
    pub freq_listener_count: u32,
    // — enable listeners (matches JS this.enableListeners = new Set()) —
    pub enable_listener_ids: [u32; MAX_ENABLE_LISTENERS],
    pub enable_listener_dispatch: Option<EnableListenerFn>,
    pub enable_listener_count: u32,
    // — pending events (replaces per-event onEnableChange) —
    pub pending_events: [ClockPendingEvent; MAX_PENDING_EVENTS],
    pub pending_count: u32,
}

impl RootClock {
    pub const fn new(freq: u32) -> Self {
        RootClock {
            frequency_value: freq,
            base_ticks: 0,
            base_nanos: 0,
            enabled_value: true,
            freq_listener_ids: [0; MAX_FREQ_LISTENERS],
            freq_listener_dispatch: None,
            freq_listener_count: 0,
            enable_listener_ids: [0; MAX_ENABLE_LISTENERS],
            enable_listener_dispatch: None,
            enable_listener_count: 0,
            pending_events: [ClockPendingEvent {
                tag: EventTag::FrcTimerAlarm { channel: 0 },
                scheduled_target_ticks: 0,
                valid: false,
            }; MAX_PENDING_EVENTS],
            pending_count: 0,
        }
    }

    /// JS get ticks()
    pub fn ticks(&self, ctx: &CpuContext) -> u64 {
        if !self.enabled_value { return self.base_ticks; }
        let root_nanos = ctx.clock_nanos();
        let elapsed_nanos = root_nanos.wrapping_sub(self.base_nanos);
        if self.frequency_value == 0 { return self.base_ticks; }
        (elapsed_nanos as u128 * self.frequency_value as u128 / 1_000_000_000) as u64 + self.base_ticks
    }

    /// JS get frequency()
    pub fn frequency(&self) -> u32 { self.frequency_value }

    /// JS get enabled()
    pub fn enabled(&self) -> bool { self.enabled_value }

    // — frequency listener helpers (JS Set<Function>) —

    pub fn add_frequency_listener(&mut self, listener_id: u32) {
        if (self.freq_listener_count as usize) < MAX_FREQ_LISTENERS {
            let i = self.freq_listener_count as usize;
            self.freq_listener_ids[i] = listener_id;
            self.freq_listener_count += 1;
        }
    }

    pub fn remove_frequency_listener(&mut self, listener_id: u32) {
        let mut i = 0;
        while i < self.freq_listener_count as usize {
            if self.freq_listener_ids[i] == listener_id {
                let last = (self.freq_listener_count - 1) as usize;
                self.freq_listener_ids[i] = self.freq_listener_ids[last];
                self.freq_listener_count -= 1;
                return;
            }
            i += 1;
        }
    }

    pub fn set_freq_listener_dispatch(&mut self, dispatch: Option<FreqListenerFn>) {
        self.freq_listener_dispatch = dispatch;
    }

    // — enable listener helpers (JS Set<Function>) —

    pub fn add_enable_listener(&mut self, listener_id: u32) {
        if (self.enable_listener_count as usize) < MAX_ENABLE_LISTENERS {
            let i = self.enable_listener_count as usize;
            self.enable_listener_ids[i] = listener_id;
            self.enable_listener_count += 1;
        }
    }

    pub fn remove_enable_listener(&mut self, listener_id: u32) {
        let mut i = 0;
        while i < self.enable_listener_count as usize {
            if self.enable_listener_ids[i] == listener_id {
                let last = (self.enable_listener_count - 1) as usize;
                self.enable_listener_ids[i] = self.enable_listener_ids[last];
                self.enable_listener_count -= 1;
                return;
            }
            i += 1;
        }
    }

    pub fn set_enable_listener_dispatch(&mut self, dispatch: Option<EnableListenerFn>) {
        self.enable_listener_dispatch = dispatch;
    }

    // — pending events helpers (replace Vec operations) —

    pub fn add_pending(&mut self, event: ClockPendingEvent) {
        if (self.pending_count as usize) < MAX_PENDING_EVENTS {
            self.pending_events[self.pending_count as usize] = event;
            self.pending_count += 1;
        }
    }

    pub fn remove_pending(&mut self, tag: EventTag) {
        let mut i = 0;
        while i < self.pending_count as usize {
            if self.pending_events[i].valid && self.pending_events[i].tag == tag {
                self.pending_events[i].valid = false;
                break;
            }
            i += 1;
        }
        // Compact
        let mut write: usize = 0;
        for read in 0..self.pending_count as usize {
            if self.pending_events[read].valid {
                if write != read {
                    self.pending_events[write] = self.pending_events[read];
                }
                write += 1;
            }
        }
        self.pending_count = write as u32;
    }

    pub fn update_pending_target(&mut self, tag: EventTag, target: u64) {
        for i in 0..self.pending_count as usize {
            if self.pending_events[i].valid && self.pending_events[i].tag == tag {
                self.pending_events[i].scheduled_target_ticks = target;
                return;
            }
        }
    }

    pub fn has_pending(&self, tag: EventTag) -> bool {
        for i in 0..self.pending_count as usize {
            if self.pending_events[i].valid && self.pending_events[i].tag == tag {
                return true;
            }
        }
        false
    }

    pub fn iter_pending(&self) -> impl Iterator<Item = &ClockPendingEvent> {
        self.pending_events[..self.pending_count as usize]
            .iter()
            .filter(|e| e.valid)
    }

    pub fn clone_pending_vec(&self) -> [ClockPendingEvent; MAX_PENDING_EVENTS] {
        let mut copy = [ClockPendingEvent {
            tag: EventTag::FrcTimerAlarm { channel: 0 },
            scheduled_target_ticks: 0,
            valid: false,
        }; MAX_PENDING_EVENTS];
        let mut idx = 0;
        for i in 0..self.pending_count as usize {
            if self.pending_events[i].valid {
                copy[idx] = self.pending_events[i];
                idx += 1;
            }
        }
        copy
    }

    pub fn pending_count(&self) -> u32 {
        self.pending_count
    }

    /// JS setEnabled(cpuVal)
    /// Matches JS comma-operator semantics:
    ///   Number.isNaN(tmpVal) || (cpuVal || (baseTicks = ticks), (baseNanos = tmpVal))
    /// → if root_nanos is valid:
    ///     base_nanos = root_nanos (always)
    ///     base_ticks = ticks       (only when disabling, i.e. !val)
    pub fn set_enabled(&mut self, ctx: &mut CpuContext, val: bool) {
        if val != self.enabled_value {
            let root_nanos = ctx.clock_nanos();
            // JS: Number.isNaN(tmpVal) || (...)
            if root_nanos != 0 {
                // JS: (cpuVal || (this.baseTicks = this.ticks), (this.baseNanos = tmpVal))
                // The comma: first eval cpuVal || baseTicks=ticks, then baseNanos=tmpVal
                if !val {
                    self.base_ticks = self.ticks(ctx);
                }
                self.base_nanos = root_nanos;
            }
            self.enabled_value = val;

            // JS: for (let i of this.enableListeners) i(cpuVal);
            if let Some(dispatch) = self.enable_listener_dispatch {
                for i in 0..self.enable_listener_count as usize {
                    dispatch(ctx, self.enable_listener_ids[i], val);
                }
            }

            // RS: pending_events reschedule (replaces onEnableChange closures)
            if val {
                let current_ticks = self.ticks(ctx);
                let freq = self.frequency_value;
                let pending_copy = self.clone_pending_vec();
                for i in 0..self.pending_count as usize {
                    let pending = pending_copy[i];
                    if !pending.valid { continue; }
                    let remaining = pending.scheduled_target_ticks.wrapping_sub(current_ticks);
                    if remaining == 0 || remaining > pending.scheduled_target_ticks {
                        ctx.schedule_event(0, pending.tag);
                    } else {
                        let nanos = (remaining as u128 * 1_000_000_000 / freq as u128) as u64;
                        ctx.schedule_event(nanos, pending.tag);
                    }
                }
            }
        }
    }

    /// JS setFrequency(cpuVal)
    pub fn set_frequency(&mut self, ctx: &mut CpuContext, new_freq: u32) -> Option<u32> {
        let old_freq = self.frequency_value;
        if new_freq != old_freq {
            let root_nanos = ctx.clock_nanos();
            // JS: Number.isNaN(idxVal) || ((this.baseTicks = this.ticks), (this.baseNanos = idxVal))
            if root_nanos != 0 {
                self.base_ticks = self.ticks(ctx);
                self.base_nanos = root_nanos;
            }
            self.frequency_value = new_freq;

            // JS: for (let cb of this.listeners) cb(this.frequencyValue, tmpVal);
            if let Some(dispatch) = self.freq_listener_dispatch {
                for i in 0..self.freq_listener_count as usize {
                    dispatch(ctx, self.freq_listener_ids[i], new_freq, old_freq);
                }
            }

            Some(old_freq)
        } else {
            None
        }
    }

    /// Compute ticks from a raw base_nanos (no CpuContext, for ClockTree reconfig)
    pub fn ticks_raw(&self, base_nanos_val: u64) -> u64 {
        if !self.enabled_value { return self.base_ticks; }
        let elapsed_nanos = base_nanos_val.wrapping_sub(self.base_nanos);
        if self.frequency_value == 0 { return self.base_ticks; }
        (elapsed_nanos as u128 * self.frequency_value as u128 / 1_000_000_000) as u64 + self.base_ticks
    }

    /// JS createEvent(cpuVal)
    pub fn create_event(&mut self, tag: EventTag) -> ClockScheduledEvent {
        ClockScheduledEvent::new(tag)
    }
}

// ============================================================
// DerivedClock — extends RootClock in JS (composition here)
// ============================================================

pub struct DerivedClock {
    pub base: RootClock,
    pub parent_idx: u32,
    pub multiplier_value: f64,
    pub own_enabled: bool,
}

impl DerivedClock {
    pub fn new(parent_idx: u32, parent_freq: u32, parent_enabled: bool, multiplier: f64) -> Self {
        let freq = (parent_freq as f64 * multiplier) as u32;
        let mut base = RootClock::new(freq);
        base.enabled_value = parent_enabled;
        DerivedClock { base, parent_idx, multiplier_value: multiplier, own_enabled: true }
    }

    pub fn ticks(&self, ctx: &CpuContext) -> u64 { self.base.ticks(ctx) }
    pub fn frequency(&self) -> u32 { self.base.frequency_value }
    pub fn enable(&self) -> bool { self.own_enabled }

    pub fn set_enable(&mut self, ctx: &mut CpuContext, val: bool, parent_enabled: bool) {
        if val != self.own_enabled {
            self.own_enabled = val;
            self.base.set_enabled(ctx, val && parent_enabled);
        }
    }

    pub fn update_frequency(&mut self, ctx: &mut CpuContext, parent_freq: u32) {
        let new_freq = (parent_freq as f64 * self.multiplier_value) as u32;
        if new_freq != self.base.frequency_value {
            self.base.set_frequency(ctx, new_freq);
        }
    }

    pub fn update_enable(&mut self, ctx: &mut CpuContext, parent_enabled: bool) {
        self.base.set_enabled(ctx, self.own_enabled && parent_enabled);
    }

    pub fn multiplier(&self) -> f64 { self.multiplier_value }
    pub fn set_multiplier(&mut self, ctx: &mut CpuContext, val: f64, parent_freq: u32) {
        self.multiplier_value = val;
        self.update_frequency(ctx, parent_freq);
    }

    pub fn divider(&self) -> f64 {
        if self.multiplier_value != 0.0 { 1.0 / self.multiplier_value } else { 0.0 }
    }

    pub fn set_divider(&mut self, ctx: &mut CpuContext, val: f64, parent_freq: u32) {
        self.multiplier_value = if val != 0.0 { 1.0 / val } else { 0.0 };
        self.update_frequency(ctx, parent_freq);
    }

    pub fn get_parent_idx(&self) -> u32 { self.parent_idx }
    pub fn set_parent_idx(&mut self, idx: u32) { self.parent_idx = idx; }
    pub fn create_event(&mut self, tag: EventTag) -> ClockScheduledEvent {
        self.base.create_event(tag)
    }
}

// ============================================================
// MutableClock — extends RootClock in JS
// ============================================================

pub struct MutableClock {
    pub base: RootClock,
}

impl MutableClock {
    pub fn new(freq: u32) -> Self { MutableClock { base: RootClock::new(freq) } }
    pub fn frequency(&self) -> u32 { self.base.frequency_value }
    pub fn set_frequency(&mut self, ctx: &mut CpuContext, val: u32) { self.base.set_frequency(ctx, val); }
    pub fn enable(&self) -> bool { self.base.enabled_value }
    pub fn set_enable(&mut self, ctx: &mut CpuContext, val: bool) { self.base.set_enabled(ctx, val); }
    pub fn ticks(&self, ctx: &CpuContext) -> u64 { self.base.ticks(ctx) }
    pub fn create_event(&mut self, tag: EventTag) -> ClockScheduledEvent { self.base.create_event(tag) }
}

// ============================================================
// SimulationClock — the root of all time
// JS: extends RootClock with this.rootClock = this
// ============================================================

pub struct SimulationClock {
    pub base: RootClock,
}

impl SimulationClock {
    /// Typically frequency = 1_000_000_000 so tick = 1 ns
    pub fn new(freq: u32) -> Self { SimulationClock { base: RootClock::new(freq) } }
    /// JS get nanos() — ctx.clock_nanos()
    pub fn nanos(&self, ctx: &CpuContext) -> u64 { ctx.clock_nanos() }
    pub fn ticks(&self, ctx: &CpuContext) -> u64 { self.base.ticks(ctx) }
    pub fn frequency(&self) -> u32 { self.base.frequency_value }
    pub fn create_event(&mut self, tag: EventTag) -> ClockScheduledEvent { self.base.create_event(tag) }
    pub fn set_frequency(&mut self, ctx: &mut CpuContext, val: u32) { self.base.set_frequency(ctx, val); }
}

// ============================================================
// ClockTree — matches JS class
// Uses fixed-size arrays for no_std compatibility
// ============================================================

pub const CLK_SIMULATION: usize = 0;
pub const CLK_RC_FAST: usize = 1;
pub const CLK_RC_FAST_DIV: usize = 2;
pub const CLK_RC_SLOW: usize = 3;
pub const CLK_XTAL: usize = 4;
pub const CLK_XTAL32K: usize = 5;
pub const CLK_FREQ80MHZ: usize = 6;
pub const CLK_APLL: usize = 7;
pub const CLK_SYSCLK: usize = 8;
pub const CLK_CPU: usize = 9;
pub const CLK_APB: usize = 10;
pub const CLK_REF: usize = 11;
pub const CLK_TG0_TIMER: usize = 12;
pub const CLK_TG0_WDT: usize = 13;
pub const CLK_TG1_TIMER: usize = 14;
pub const CLK_TG1_WDT: usize = 15;
pub const CLK_LEDC: usize = 16;
pub const CLK_RMT: usize = 17;
pub const CLK_UART0: usize = 18;
pub const CLK_UART1: usize = 19;
pub const CLK_UART2: usize = 20;
pub const CLK_SPI2: usize = 21;
pub const CLK_I2C0: usize = 22;
pub const TOTAL_CLOCKS: usize = 23;

pub fn divider_to_multiplier(div: u32) -> f64 {
    if div != 0 { 1.0 / div as f64 } else { 0.0 }
}

pub struct ClockTree {
    pub clocks: [RootClock; TOTAL_CLOCKS],
    pub parent_indices: [Option<u32>; TOTAL_CLOCKS],
    pub multipliers: [f64; TOTAL_CLOCKS],
    pub own_enabled_flags: [bool; TOTAL_CLOCKS],
    pub is_derived: [bool; TOTAL_CLOCKS],
    pub cpu_clock_source: CpuClockSource,
    pub cpu_clock_period: u32,
    pub sysclk_pre_div: u32,
    pub xtal_ticks: u32,
    pub pll_ticks: u32,
    pub rtc8m_ticks: u32,
    pub apll_ticks: u32,
    pub saved_tg0_wdt_enabled: bool,
    pub saved_tg1_wdt_enabled: bool,
}

impl ClockTree {
    pub fn new(sim_freq: u32) -> Self {
        let clocks = [RootClock::new(0); TOTAL_CLOCKS];
        let parent_indices: [Option<u32>; TOTAL_CLOCKS] = [None; TOTAL_CLOCKS];
        let multipliers: [f64; TOTAL_CLOCKS] = [0.0; TOTAL_CLOCKS];
        let own_enabled_flags: [bool; TOTAL_CLOCKS] = [true; TOTAL_CLOCKS];
        let is_derived: [bool; TOTAL_CLOCKS] = [false; TOTAL_CLOCKS];

        let mut ct = ClockTree {
            clocks, parent_indices, multipliers,
            own_enabled_flags, is_derived,
            cpu_clock_source: CpuClockSource::XtlClk,
            cpu_clock_period: 0,
            sysclk_pre_div: 0,
            xtal_ticks: 0,
            pll_ticks: 0,
            rtc8m_ticks: 0,
            apll_ticks: 0,
            saved_tg0_wdt_enabled: true,
            saved_tg1_wdt_enabled: true,
        };

        // Initialize each clock matching JS constructor
        // simulation clock — index 0
        ct.clocks[CLK_SIMULATION] = RootClock::new(sim_freq);
        ct.is_derived[CLK_SIMULATION] = false;

        // rcFast — index 1: MutableClock(root, 8 * UsToNsFactor)
        ct.clocks[CLK_RC_FAST] = RootClock::new(8_000_000);
        ct.is_derived[CLK_RC_FAST] = false;

        // rcFastDiv — index 2: DerivedClock(rcFast, 1/256)
        {
            let pf = ct.clocks[CLK_RC_FAST].frequency_value;
            let pe = ct.clocks[CLK_RC_FAST].enabled_value;
            let freq = (pf as f64 * (1.0/256.0)) as u32;
            let mut base = RootClock::new(freq);
            base.enabled_value = pe;
            ct.clocks[CLK_RC_FAST_DIV] = base;
            ct.parent_indices[CLK_RC_FAST_DIV] = Some(CLK_RC_FAST as u32);
            ct.multipliers[CLK_RC_FAST_DIV] = 1.0/256.0;
            ct.own_enabled_flags[CLK_RC_FAST_DIV] = true;
            ct.is_derived[CLK_RC_FAST_DIV] = true;
        }

        // rcSlow — index 3: MutableClock(root, 150 * MsToNsFactor)
        ct.clocks[CLK_RC_SLOW] = RootClock::new(150_000);
        ct.is_derived[CLK_RC_SLOW] = false;

        // xtal — index 4: MutableClock(root, 40 * UsToNsFactor)
        ct.clocks[CLK_XTAL] = RootClock::new(40_000_000);
        ct.is_derived[CLK_XTAL] = false;

        // xtal32k — index 5: MutableClock(root, 32768)
        ct.clocks[CLK_XTAL32K] = RootClock::new(32768);
        ct.is_derived[CLK_XTAL32K] = false;

        // freq80mhz — index 6: MutableClock(root, 80 * UsToNsFactor)
        ct.clocks[CLK_FREQ80MHZ] = RootClock::new(80_000_000);
        ct.is_derived[CLK_FREQ80MHZ] = false;

        // apll — index 7: MutableClock(root, 100 * UsToNsFactor)
        ct.clocks[CLK_APLL] = RootClock::new(100_000_000);
        ct.is_derived[CLK_APLL] = false;

        // sysclk — index 8: DerivedClock(xtal, 1)
        {
            let pf = ct.clocks[CLK_XTAL].frequency_value;
            let pe = ct.clocks[CLK_XTAL].enabled_value;
            let mut base = RootClock::new(pf);
            base.enabled_value = pe;
            ct.clocks[CLK_SYSCLK] = base;
            ct.parent_indices[CLK_SYSCLK] = Some(CLK_XTAL as u32);
            ct.multipliers[CLK_SYSCLK] = 1.0;
            ct.own_enabled_flags[CLK_SYSCLK] = true;
            ct.is_derived[CLK_SYSCLK] = true;
        }

        // cpu — index 9: DerivedClock(sysclk, 1)
        {
            let pf = ct.clocks[CLK_SYSCLK].frequency_value;
            let pe = ct.clocks[CLK_SYSCLK].enabled_value;
            let mut base = RootClock::new(pf);
            base.enabled_value = pe;
            ct.clocks[CLK_CPU] = base;
            ct.parent_indices[CLK_CPU] = Some(CLK_SYSCLK as u32);
            ct.multipliers[CLK_CPU] = 1.0;
            ct.own_enabled_flags[CLK_CPU] = true;
            ct.is_derived[CLK_CPU] = true;
        }

        // apb — index 10: DerivedClock(sysclk, 1)
        {
            let pf = ct.clocks[CLK_SYSCLK].frequency_value;
            let pe = ct.clocks[CLK_SYSCLK].enabled_value;
            let mut base = RootClock::new(pf);
            base.enabled_value = pe;
            ct.clocks[CLK_APB] = base;
            ct.parent_indices[CLK_APB] = Some(CLK_SYSCLK as u32);
            ct.multipliers[CLK_APB] = 1.0;
            ct.own_enabled_flags[CLK_APB] = true;
            ct.is_derived[CLK_APB] = true;
        }

        // ref — index 11: DerivedClock(apb, 1)
        {
            let pf = ct.clocks[CLK_APB].frequency_value;
            let pe = ct.clocks[CLK_APB].enabled_value;
            let mut base = RootClock::new(pf);
            base.enabled_value = pe;
            ct.clocks[CLK_REF] = base;
            ct.parent_indices[CLK_REF] = Some(CLK_APB as u32);
            ct.multipliers[CLK_REF] = 1.0;
            ct.own_enabled_flags[CLK_REF] = true;
            ct.is_derived[CLK_REF] = true;
        }

        // tg0Timer — index 12: DerivedClock(xtal, 1)
        ct.set_derived(CLK_TG0_TIMER, CLK_XTAL, 1.0);

        // tg0WDT — index 13: DerivedClock(freq80mhz, 1)
        ct.set_derived(CLK_TG0_WDT, CLK_FREQ80MHZ, 1.0);

        // tg1Timer — index 14: DerivedClock(xtal, 1)
        ct.set_derived(CLK_TG1_TIMER, CLK_XTAL, 1.0);

        // tg1WDT — index 15: DerivedClock(freq80mhz, 1)
        ct.set_derived(CLK_TG1_WDT, CLK_FREQ80MHZ, 1.0);

        // ledc — index 16
        ct.set_derived(CLK_LEDC, CLK_XTAL, 1.0);

        // rmt — index 17
        ct.set_derived(CLK_RMT, CLK_XTAL, 1.0);

        // uart0 — index 18
        ct.set_derived(CLK_UART0, CLK_XTAL, 1.0);

        // uart1 — index 19
        ct.set_derived(CLK_UART1, CLK_XTAL, 1.0);

        // uart2 — index 20
        ct.set_derived(CLK_UART2, CLK_XTAL, 1.0);

        // spi2 — index 21
        ct.set_derived(CLK_SPI2, CLK_XTAL, 1.0);

        // i2c0 — index 22
        ct.set_derived(CLK_I2C0, CLK_XTAL, 1.0);

        // JS: this.update() at end of constructor
        ct.sync_all_derived();
        ct
    }

    fn set_derived(&mut self, idx: usize, parent: usize, multiplier: f64) {
        let pf = self.clocks[parent].frequency_value;
        let pe = self.clocks[parent].enabled_value;
        let freq = (pf as f64 * multiplier) as u32;
        let mut base = RootClock::new(freq);
        base.enabled_value = pe;
        self.clocks[idx] = base;
        self.parent_indices[idx] = Some(parent as u32);
        self.multipliers[idx] = multiplier;
        self.own_enabled_flags[idx] = true;
        self.is_derived[idx] = true;
    }

    fn sync_all_derived(&mut self) {
        let root_nanos = self.clocks[CLK_SIMULATION].base_nanos;
        for i in 0..TOTAL_CLOCKS {
            if !self.is_derived[i] { continue; }
            if let Some(parent_idx) = self.parent_indices[i] {
                let p = parent_idx as usize;
                let parent_freq = self.clocks[p].frequency_value;
                let parent_enabled = self.clocks[p].enabled_value;

                let new_freq = (parent_freq as f64 * self.multipliers[i]) as u32;
                if new_freq != self.clocks[i].frequency_value {
                    if root_nanos != 0 {
                        self.clocks[i].base_ticks = self.clocks[i].ticks_raw(root_nanos);
                        self.clocks[i].base_nanos = root_nanos;
                    }
                    self.clocks[i].frequency_value = new_freq;
                }

                let combined = self.own_enabled_flags[i] && parent_enabled;
                if combined != self.clocks[i].enabled_value {
                    if !combined {
                        self.clocks[i].base_ticks = self.clocks[i].ticks_raw(root_nanos);
                        self.clocks[i].base_nanos = root_nanos;
                    }
                    self.clocks[i].enabled_value = combined;
                }
            }
        }
    }

    /// JS update() — reconfigures clock tree based on cpuClockSource
    pub fn update(&mut self) {
        self.multipliers[CLK_APB] = 1.0;
        self.multipliers[CLK_CPU] = 1.0;

        match self.cpu_clock_source {
            CpuClockSource::XtlClk => {
                self.parent_indices[CLK_SYSCLK] = Some(CLK_XTAL as u32);
                self.multipliers[CLK_SYSCLK] = divider_to_multiplier(self.sysclk_pre_div + 1);
                self.multipliers[CLK_REF] = divider_to_multiplier(1 + self.xtal_ticks);
            }
            CpuClockSource::PllClk => {
                self.parent_indices[CLK_SYSCLK] = Some(CLK_FREQ80MHZ as u32);
                self.multipliers[CLK_SYSCLK] = 1.0;
                let cpu_mul = if self.cpu_clock_period == 2 { 3.0 }
                              else if self.cpu_clock_period == 1 { 2.0 }
                              else { 1.0 };
                self.multipliers[CLK_CPU] = cpu_mul;
                self.multipliers[CLK_REF] = divider_to_multiplier(1 + self.pll_ticks);
            }
            CpuClockSource::Rtc8mClk => {
                self.parent_indices[CLK_SYSCLK] = Some(CLK_RC_FAST as u32);
                self.multipliers[CLK_SYSCLK] = divider_to_multiplier(self.sysclk_pre_div + 1);
                self.multipliers[CLK_REF] = divider_to_multiplier(1 + self.rtc8m_ticks);
            }
            CpuClockSource::ApllClk => {
                self.parent_indices[CLK_SYSCLK] = Some(CLK_APLL as u32);
                let sys_div = if self.cpu_clock_period == 0 { 4.0 } else { 2.0 };
                self.multipliers[CLK_SYSCLK] = 1.0 / sys_div;
                self.multipliers[CLK_APB] = 0.5;
                self.multipliers[CLK_REF] = divider_to_multiplier(1 + self.apll_ticks);
            }
        }

        self.sync_all_derived();
    }

    /// JS pauseApbClocks()
    pub fn pause_apb_clocks(&mut self) {
        self.saved_tg0_wdt_enabled = self.clocks[CLK_TG0_WDT].enabled_value;
        self.saved_tg1_wdt_enabled = self.clocks[CLK_TG1_WDT].enabled_value;
        self.clocks[CLK_TG0_WDT].enabled_value = false;
        self.clocks[CLK_TG1_WDT].enabled_value = false;
    }

    /// JS resumeApbClocks()
    pub fn resume_apb_clocks(&mut self) {
        self.clocks[CLK_TG0_WDT].enabled_value = self.saved_tg0_wdt_enabled;
        self.clocks[CLK_TG1_WDT].enabled_value = self.saved_tg1_wdt_enabled;
    }

    pub fn set_clock_frequency(&mut self, idx: usize, ctx: &mut CpuContext, new_freq: u32) {
        self.clocks[idx].set_frequency(ctx, new_freq);
        for i in 0..TOTAL_CLOCKS {
            if !self.is_derived[i] { continue; }
            if let Some(parent_idx) = self.parent_indices[i] {
                if (parent_idx as usize) == idx {
                    let pf = self.clocks[parent_idx as usize].frequency_value;
                    let nf = (pf as f64 * self.multipliers[i]) as u32;
                    self.clocks[i].set_frequency(ctx, nf);
                }
            }
        }
    }

    pub fn set_clock_enabled(&mut self, idx: usize, ctx: &mut CpuContext, val: bool) {
        self.clocks[idx].set_enabled(ctx, val);
        for i in 0..TOTAL_CLOCKS {
            if !self.is_derived[i] { continue; }
            if let Some(parent_idx) = self.parent_indices[i] {
                if (parent_idx as usize) == idx {
                    let pe = self.clocks[parent_idx as usize].enabled_value;
                    let combined = self.own_enabled_flags[i] && pe;
                    self.clocks[i].set_enabled(ctx, combined);
                }
            }
        }
    }

    pub fn ticks_for(&self, idx: usize, ctx: &CpuContext) -> u64 {
        self.clocks[idx].ticks(ctx)
    }

    pub fn frequency_for(&self, idx: usize) -> u32 {
        self.clocks[idx].frequency_value
    }

    pub fn update_clocks_snapshot(&self, clk: &mut Clocks, ctx: &CpuContext) {
        clk.apb.ticks = self.clocks[CLK_APB].ticks(ctx);
        clk.apb.frequency = self.clocks[CLK_APB].frequency_value;
        clk.xtal.ticks = self.clocks[CLK_XTAL].ticks(ctx);
        clk.xtal.frequency = self.clocks[CLK_XTAL].frequency_value;
        clk.rc_fast.ticks = self.clocks[CLK_RC_FAST].ticks(ctx);
        clk.rc_fast.frequency = self.clocks[CLK_RC_FAST].frequency_value;
        clk.ref_tick.ticks = self.clocks[CLK_REF].ticks(ctx);
        clk.ref_tick.frequency = self.clocks[CLK_REF].frequency_value;
        clk.cpu_clock_period = self.cpu_clock_period;
    }
}
