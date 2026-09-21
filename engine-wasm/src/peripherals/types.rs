use core::cmp::Ordering;

// Peripheral type identifier — matches PeripheralType enum discriminants
// (u32 used here to avoid circular dependency with common::enums/common::gpio_core)
pub type PeripheralId = u32;
pub const PERIPH_ID_GPIO: PeripheralId = 0;
pub const PERIPH_ID_SPI: PeripheralId = 1;
pub const PERIPH_ID_I2C: PeripheralId = 2;
pub const PERIPH_ID_UART: PeripheralId = 3;
pub const PERIPH_ID_LEDC: PeripheralId = 4;
pub const PERIPH_ID_PCNT: PeripheralId = 5;
pub const PERIPH_ID_TWAI: PeripheralId = 6;
pub const PERIPH_ID_OTHER: PeripheralId = 7;
pub const PERIPH_ID_NONE: PeripheralId = 8;

// ============================================================
// Clock and Time Types
// ============================================================

#[derive(Clone, Copy)]
pub struct ClockRef {
    pub ticks: u64,
    pub frequency: u32, // Hz
}

impl ClockRef {
    pub const fn new(freq: u32) -> Self {
        ClockRef { ticks: 0, frequency: freq }
    }
    pub fn nanos(&self) -> u64 {
        if self.frequency == 0 { return 0; }
        (self.ticks * 1_000_000_000) / self.frequency as u64
    }
}

pub struct Clocks {
    pub apb: ClockRef,
    pub xtal: ClockRef,
    pub rc_fast: ClockRef,
    pub ref_tick: ClockRef,
    pub cpu_clock_period: u32,
}

impl Clocks {
    pub fn update(&mut self) {}
}

// ============================================================
// Event System
// ============================================================

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EventTag {
    FrcTimerAlarm { channel: u32 },
    Timg0Timer0Alarm,
    Timg0Timer1Alarm,
    Timg0Wdt { stage: u32 },
    Timg0LactAlarm,
    Timg1Timer0Alarm,
    Timg1Timer1Alarm,
    Timg1Wdt { stage: u32 },
    Timg1LactAlarm,
    UartTimeout { uart_idx: u32 },
    UartIntCheck { uart_idx: u32 },
    EfuseCmdDone,
    GpioPulse { pin: u32 },
    LedcTimerAlarm { channel: u32, timer_idx: u32, is_high: bool },
    LedcFadeEnd { channel: u32 },
    PcntUnitEvent { unit: u32 },
    I2sTx { idx: u32 },
    I2sRx { idx: u32 },
    SdmmcCmdComplete,
    SpiFlashEraseDone { idx: u32 },
    RtcSlowWakeup,
    AdcDone { unit: u32 },
}

#[derive(Clone, Copy)]
struct EventEntry {
    fire_tick: u64,
    tag: EventTag,
    scheduled: bool,
}

const MAX_EVENTS: usize = 128;

pub struct EventQueue {
    events: [EventEntry; MAX_EVENTS],
    count: usize,
    next_id: u64,
}

impl EventQueue {
    pub const fn new() -> Self {
        EventQueue {
            events: [EventEntry { fire_tick: 0, tag: EventTag::FrcTimerAlarm { channel: 0 }, scheduled: false }; MAX_EVENTS],
            count: 0,
            next_id: 0,
        }
    }

    pub fn schedule(&mut self, delta_ticks: u64, tag: EventTag, current_tick: u64) -> usize {
        if self.count >= MAX_EVENTS { return usize::MAX; }
        let fire_tick = current_tick + delta_ticks;
        let id = self.next_id as usize;
        self.next_id += 1;
        self.events[self.count] = EventEntry { fire_tick, tag, scheduled: true };
        self.count += 1;
        id
    }

    pub fn unschedule_all(&mut self, tag: EventTag) {
        for i in 0..self.count {
            if self.events[i].tag == tag {
                self.events[i].scheduled = false;
            }
        }
    }

    pub fn has_scheduled(&self, tag: EventTag) -> bool {
        for i in 0..self.count {
            if self.events[i].tag == tag && self.events[i].scheduled {
                return true;
            }
        }
        false
    }

    pub fn fire_pending<F: FnMut(EventTag)>(&mut self, current_tick: u64, mut handler: F) {
        let mut i = 0;
        while i < self.count {
            if self.events[i].scheduled && self.events[i].fire_tick <= current_tick {
                self.events[i].scheduled = false;
                let tag = self.events[i].tag;
                handler(tag);
            }
            i += 1;
        }
        // Compact: remove fired events
        let mut write = 0;
        for read in 0..self.count {
            if self.events[read].scheduled {
                if write != read {
                    self.events[write] = self.events[read];
                }
                write += 1;
            }
        }
        self.count = write;
    }

    pub fn next_fire_tick(&self) -> u64 {
        let mut min = u64::MAX;
        for i in 0..self.count {
            if self.events[i].scheduled && self.events[i].fire_tick < min {
                min = self.events[i].fire_tick;
            }
        }
        if min == u64::MAX { 0 } else { min }
    }

    pub fn clear(&mut self) {
        self.count = 0;
    }
}

// ============================================================
// Interrupt Matrix
// ============================================================

pub struct InterruptMatrix;

impl InterruptMatrix {
    pub const fn new() -> Self { InterruptMatrix }
    pub fn interrupt(&mut self, irq: u32, level: bool) {
        // Route through JS InterruptMatrixPeripheral which has the correct IRQ→CPU pin mapping
        unsafe { crate::peripherals::common::ffi::js_interrupt(irq, if level { 1 } else { 0 }); }
    }
}

// ============================================================
// GPIO Matrix — fully replaces the old stub
// ============================================================

pub const MAX_OUT_PERIPHERALS: usize = 256;
pub const MAX_INPUT_SIGNAL_ENTRIES: usize = 64;
/// Peripheral input signals addressable through GPIO_FUNCn_IN_SEL_CFG
/// (n = signal 0..255).
pub const MAX_IN_SIGNALS: usize = 256;
pub const MAX_PIN_SIGNAL_SETS: usize = 64;
pub const MAX_SIGNALS_IN_SET: usize = 16;
pub const MAX_GPIO_MAP_ENTRIES: usize = 64;
pub const GPIO_NO_MATRIX: i32 = -1;

// ── ConstPins ──
#[derive(Clone, Copy)]
pub struct ConstPins {
    pub const_one_input: u32,
    pub const_zero_input: u32,
}

// ── MatrixEntryOut ──
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct MatrixEntryOut {
    pub peripheral: PeripheralId,
    pub signal: u32,
}

// ── MatrixEntry ──
#[derive(Clone, Copy)]
pub struct MatrixEntry {
    pub i: u32,
    pub out: Option<MatrixEntryOut>,
    pub in_val: Option<u32>,
    pub ind: bool,
}

// ── GpioMatrixOutputSignal ──
#[derive(Clone, Copy)]
pub struct GpioMatrixOutputSignal {
    pub sig: Option<MatrixEntryOut>,
    pub value: bool,
    pub enable: bool,
    pub gpios: [i32; MAX_OUT_PERIPHERALS],
    pub gpio_count: u32,
}

impl GpioMatrixOutputSignal {
    pub const fn new(sig: Option<MatrixEntryOut>) -> Self {
        GpioMatrixOutputSignal {
            sig,
            value: false,
            enable: false,
            gpios: [GPIO_NO_MATRIX; MAX_OUT_PERIPHERALS],
            gpio_count: 0,
        }
    }

    pub fn add_gpio(&mut self, gpio_num: i32) {
        if (self.gpio_count as usize) < MAX_OUT_PERIPHERALS {
            self.gpios[self.gpio_count as usize] = gpio_num;
            self.gpio_count += 1;
        }
    }

    pub fn remove_gpio(&mut self, gpio_num: i32) {
        let mut found = false;
        for i in 0..(self.gpio_count as usize) {
            if found {
                self.gpios[i - 1] = self.gpios[i];
            } else if self.gpios[i] == gpio_num {
                found = true;
            }
        }
        if found {
            self.gpio_count -= 1;
            self.gpios[self.gpio_count as usize] = GPIO_NO_MATRIX;
        }
    }
}

// ── GpioInputSignal (JS class) ──
pub type SignalListener = fn(bool);

#[derive(Clone, Copy)]
pub struct GpioInputSignal {
    pub const_pin: ConstPins,
    pub invert: bool,
    pub pin: u32,
    pub value: bool,
}

impl GpioInputSignal {
    pub fn new(const_pin: ConstPins, initial: bool) -> Self {
        let pin = if initial {
            const_pin.const_one_input
        } else {
            const_pin.const_zero_input
        };
        GpioInputSignal {
            const_pin,
            invert: false,
            pin,
            value: initial,
        }
    }

    /// JS: update(cpuVal) — notifications are dispatched via on_update callback
    pub fn update(&mut self, val: Option<bool>) -> Option<bool> {
        let cpu_val = match val {
            Some(v) => v,
            None => match self.pin {
                x if x == self.const_pin.const_one_input => true,
                x if x == self.const_pin.const_zero_input => false,
                _ => return None,
            },
        };
        let mut new_val = cpu_val;
        if self.invert {
            new_val = !new_val;
        }
        if self.value == new_val {
            return None;
        }
        self.value = new_val;
        Some(new_val)
    }
}

#[derive(Clone, Copy)]
pub struct SignalInfo {
    pub pin: u32,
    pub signal: u32,
    pub invert: bool,
}

// ── PinSignalSet (replaces JS Set<u32>) ──
#[derive(Clone, Copy)]
pub struct PinSignalSet {
    pub pin: u32,
    pub signals: [u32; MAX_SIGNALS_IN_SET],
    pub signal_count: u32,
}

impl PinSignalSet {
    pub fn has(&self, reg_val: u32) -> bool {
        for i in 0..(self.signal_count as usize) {
            if self.signals[i] == reg_val {
                return true;
            }
        }
        false
    }

    pub fn add(&mut self, reg_val: u32) -> bool {
        if self.has(reg_val) {
            return false;
        }
        if (self.signal_count as usize) >= MAX_SIGNALS_IN_SET {
            return false;
        }
        self.signals[self.signal_count as usize] = reg_val;
        self.signal_count += 1;
        true
    }

    pub fn delete(&mut self, reg_val: u32) -> bool {
        let mut found = false;
        for i in 0..(self.signal_count as usize) {
            if found {
                self.signals[i - 1] = self.signals[i];
            } else if self.signals[i] == reg_val {
                found = true;
            }
        }
        if found {
            self.signal_count -= 1;
            self.signals[self.signal_count as usize] = 0;
            true
        } else {
            false
        }
    }
}

// ── InputSignalEntry ──
#[derive(Clone, Copy)]
pub struct InputSignalEntry {
    pub reg_val: u32,
    pub signal: GpioInputSignal,
}

// ── GpioMapEntry ──
#[derive(Clone, Copy)]
pub struct GpioMapEntry {
    pub gpio_num: i32,
    pub out_idx: i32,
}

// ── Callbacks for matrix-out and signal-changed notifications ──
pub type MatrixOutFn = fn(i32, bool, bool);
pub type SignalChangedFn = fn(u32, i32);

// ── GpioMatrix (replaces stub with full implementation) ──
pub struct GpioMatrix {
    pub const_pins: ConstPins,
    pub matrix_entries: [MatrixEntry; MAX_OUT_PERIPHERALS],
    pub matrix_entry_count: u32,
    pub out_peripherals: [GpioMatrixOutputSignal; MAX_OUT_PERIPHERALS],
    pub gpio_map: [GpioMapEntry; MAX_GPIO_MAP_ENTRIES],
    pub gpio_map_count: u32,
    pub pin_input_sets: [PinSignalSet; MAX_PIN_SIGNAL_SETS],
    pub pin_input_set_count: u32,
    /// Authoritative input route table: in_routes[signal] = gpio pad number,
    /// or -1 when unrouted. Maintained by func_in_select().
    pub in_routes: [i32; MAX_IN_SIGNALS],
    pub input_signals: [Option<InputSignalEntry>; MAX_INPUT_SIGNAL_ENTRIES],
    pub input_signal_count: u32,
    pub on_matrix_out: MatrixOutFn,
    pub on_signal_changed: SignalChangedFn,
}

impl GpioMatrix {
    /// Creates an empty/default GpioMatrix (backward compat for legacy callers).
    /// Real initialization should use `init_from_entries()`.
    pub const fn new() -> Self {
        GpioMatrix {
            const_pins: ConstPins { const_one_input: 0, const_zero_input: 0 },
            matrix_entries: [MatrixEntry { i: 0, out: None, in_val: None, ind: false }; MAX_OUT_PERIPHERALS],
            matrix_entry_count: 0,
            out_peripherals: [GpioMatrixOutputSignal::new(None); MAX_OUT_PERIPHERALS],
            gpio_map: [GpioMapEntry { gpio_num: -1, out_idx: -1 }; MAX_GPIO_MAP_ENTRIES],
            gpio_map_count: 0,
            pin_input_sets: [PinSignalSet { pin: 0, signals: [0; MAX_SIGNALS_IN_SET], signal_count: 0 }; MAX_PIN_SIGNAL_SETS],
            pin_input_set_count: 0,
            in_routes: [-1; MAX_IN_SIGNALS],
            input_signals: [None; MAX_INPUT_SIGNAL_ENTRIES],
            input_signal_count: 0,
            on_matrix_out: |_, _, _| {},
            on_signal_changed: |_, _| {},
        }
    }

    /// Full initialization from matrix entries and const pins (matching JS constructor).
    pub fn from_entries(matrix_entries: &[MatrixEntry], const_pins: ConstPins) -> Self {
        let mut m = GpioMatrix {
            const_pins,
            matrix_entries: [MatrixEntry { i: 0, out: None, in_val: None, ind: false }; MAX_OUT_PERIPHERALS],
            matrix_entry_count: 0,
            out_peripherals: [GpioMatrixOutputSignal::new(None); MAX_OUT_PERIPHERALS],
            gpio_map: [GpioMapEntry { gpio_num: -1, out_idx: -1 }; MAX_GPIO_MAP_ENTRIES],
            gpio_map_count: 0,
            pin_input_sets: [PinSignalSet { pin: 0, signals: [0; MAX_SIGNALS_IN_SET], signal_count: 0 }; MAX_PIN_SIGNAL_SETS],
            pin_input_set_count: 0,
            in_routes: [-1; MAX_IN_SIGNALS],
            input_signals: [None; MAX_INPUT_SIGNAL_ENTRIES],
            input_signal_count: 0,
            on_matrix_out: |_, _, _| {},
            on_signal_changed: |_, _| {},
        };

        let copy_count = MAX_OUT_PERIPHERALS.min(matrix_entries.len());
        for i in 0..copy_count {
            m.matrix_entries[i] = matrix_entries[i];
        }
        m.matrix_entry_count = copy_count as u32;

        for tmp_val in 0..MAX_OUT_PERIPHERALS {
            let idx_val = matrix_entries.iter().find(|e| e.i == tmp_val as u32);
            let out_per = idx_val.and_then(|e| e.out);
            m.out_peripherals[tmp_val] = GpioMatrixOutputSignal::new(out_per);
            if let Some(ref o) = out_per {
                if o.peripheral == PERIPH_ID_I2C {
                    m.out_peripherals[tmp_val].value = true;
                }
            }
        }

        for entry in matrix_entries {
            if let Some(in_val) = entry.in_val {
                if (m.input_signal_count as usize) < MAX_INPUT_SIGNAL_ENTRIES {
                    m.input_signals[m.input_signal_count as usize] = Some(InputSignalEntry {
                        reg_val: in_val,
                        signal: GpioInputSignal::new(const_pins, entry.ind),
                    });
                    m.input_signal_count += 1;
                }
            }
        }

        m
    }

    /// JS: setOutput(cpuVal, tmpVal, idxVal) → (signal, enable, value)
    /// Stores enable/value on the output signal and calls matrixOut on each
    /// attached GPIO pin via the on_matrix_out callback.
    pub fn set_output(&mut self, signal: u32, enable: bool, value: bool) {
        if (signal as usize) >= MAX_OUT_PERIPHERALS {
            return;
        }
        let out = &mut self.out_peripherals[signal as usize];
        out.enable = enable;
        out.value = value;
        for i in 0..(out.gpio_count as usize) {
            let gpio_num = out.gpios[i];
            if gpio_num >= 0 {
                (self.on_matrix_out)(gpio_num, enable, value);
            }
        }
    }

    // Legacy 4-arg set_output — kept for callers that haven't migrated
    pub fn set_output_legacy(&mut self, _pin: u32, _signal: u32, _invert: bool, _oe_invert: bool) {
        // DEPRECATED: use set_output(signal, enable, value) instead
    }

    pub fn get_output(&self, signal: u32) -> Option<&GpioMatrixOutputSignal> {
        if (signal as usize) < MAX_OUT_PERIPHERALS {
            Some(&self.out_peripherals[signal as usize])
        } else {
            None
        }
    }

    /// JS: getInput(cpuVal) — returns the GpioInputSignal for the given signal id
    pub fn get_input(&self, signal: u32) -> Option<&GpioInputSignal> {
        for i in 0..(self.input_signal_count as usize) {
            if let Some(ref entry) = self.input_signals[i] {
                if entry.reg_val == signal {
                    return Some(&entry.signal);
                }
            }
        }
        None
    }

    /// JS: funcInSelect(cpuVal, tmpVal, idxVal, ClockEvent, SimulationClock)
    /// cpuVal=gpio pad number, tmpVal=route enable, idxVal=PERIPHERAL SIGNAL
    /// number (FUNCn_IN_SEL_CFG is indexed by signal 0..255; its [5:0] field
    /// holds the gpio pad), ClockEvent=invert, SimulationClock=level.
    pub fn func_in_select(&mut self, gpio: u32, enable: bool, signal: u32, invert: bool, value: Option<bool>) {
        // Authoritative route table: signal -> gpio pad (-1 = unrouted).
        // Updated FIRST (before any early return below) so peripheral input
        // dispatch (PCNT/RMT/...) always sees live routing.
        if (signal as usize) < MAX_IN_SIGNALS {
            if enable {
                self.in_routes[signal as usize] = gpio as i32;
            } else if self.in_routes[signal as usize] == gpio as i32 {
                self.in_routes[signal as usize] = -1;
            }
        }
        // Find matrix entry for this gpio
        let mut reg_val = None;
        for i in 0..(self.matrix_entry_count as usize) {
            if self.matrix_entries[i].i == gpio {
                reg_val = self.matrix_entries[i].in_val;
                break;
            }
        }
        let reg_val = match reg_val {
            Some(v) => v,
            None => return,
        };

        // Find the input signal
        let arg_idx = self.find_input_signal_index(reg_val);
        let arg_idx = match arg_idx {
            Some(i) => i,
            None => return,
        };
        let arg_val = &mut self.input_signals[arg_idx as usize];
        let arg_val = match arg_val {
            Some(ref mut e) => e,
            None => return,
        };

        // JS: argVal.pin = idxVal; argVal.invert = ClockEvent;
        arg_val.signal.pin = signal;
        arg_val.signal.invert = invert;

        // JS: argVal.update(tmpVal ? SimulationClock : undefined)
        arg_val.signal.update(if enable { value } else { None });

        // Legacy reverse map (kept for is_input_routed parity): PinSignalSet
        // keyed by GPIO pin, storing RAW hardware signal IDs.
        if enable {
            for i in 0..(self.pin_input_set_count as usize) {
                self.pin_input_sets[i].delete(signal);
            }
        }
        let set_idx = self.find_or_create_pin_set(gpio);

        // JS: tmpVal ? add : delete routing
        if enable {
            if self.pin_input_sets[set_idx as usize].add(signal) {
                (self.on_signal_changed)(reg_val, signal as i32);
            }
        } else {
            if self.pin_input_sets[set_idx as usize].delete(signal) {
                (self.on_signal_changed)(reg_val, -1);
            }
        }
    }

    /// JS: funcOutSelect(cpuVal, tmpVal) → (gpioObj, signal)
    pub fn func_out_select(&mut self, gpio: u32, signal: u32, invert: bool, oe_invert: bool) {
        let _ = invert;
        let _ = oe_invert;

        let map_idx = self.find_gpio_map_index(gpio as i32);
        let prev_signal_idx = map_idx.and_then(|idx| {
            let out_idx = self.gpio_map[idx as usize].out_idx;
            if out_idx >= 0 { Some(out_idx as usize) } else { None }
        });

        let new_out = if (signal as usize) < MAX_OUT_PERIPHERALS {
            Some(signal as usize)
        } else {
            None
        };

        let prev_is_same = match (prev_signal_idx, new_out) {
            (Some(p), Some(n)) => p == n,
            (None, None) => true,
            _ => false,
        };

        if !prev_is_same {
            self.set_gpio_map_entry(gpio as i32, new_out.map(|x| x as i32).unwrap_or(-1));

            if let Some(prev_idx) = prev_signal_idx {
                self.out_peripherals[prev_idx].remove_gpio(gpio as i32);
            }

            if let Some(prev_idx) = prev_signal_idx {
                if let Some(ref out_sig) = self.out_peripherals[prev_idx].sig {
                    (self.on_signal_changed)(out_sig.signal, -1);
                }
            }

            if let Some(new_idx) = new_out {
                self.out_peripherals[new_idx].add_gpio(gpio as i32);
                // JS calls cpuVal.matrixOut(enable, value) on each gpio at setOutput time
                if let Some(ref out_sig) = self.out_peripherals[new_idx].sig {
                    (self.on_signal_changed)(out_sig.signal, gpio as i32);
                }
            }
        }
    }

    /// JS: inputChanged(cpuVal, tmpVal) → (pin, value)
    /// Passes value through to each affected input signal's update.
    pub fn input_changed(&mut self, pin: u32, value: Option<bool>) {
        for i in 0..(self.pin_input_set_count as usize) {
            if self.pin_input_sets[i].pin == pin {
                for j in 0..(self.pin_input_sets[i].signal_count as usize) {
                    let sig = self.pin_input_sets[i].signals[j];
                    for k in 0..(self.input_signal_count as usize) {
                        if let Some(ref mut entry) = self.input_signals[k] {
                            if entry.reg_val == sig {
                                entry.signal.update(value);
                            }
                        }
                    }
                }
            }
        }
    }

    pub fn is_input_routed(&self, signal: u32) -> bool {
        for i in 0..(self.pin_input_set_count as usize) {
            if self.pin_input_sets[i].has(signal) {
                return true;
            }
        }
        false
    }

    pub fn pin_signal_in_info(&self, pin: u32) -> &[SignalInfo] {
        let _ = pin;
        &[]
    }

    pub fn pin_signal_out_info(&self, pin: u32) -> &[SignalInfo] {
        let _ = pin;
        &[]
    }

    // ── Internal helpers ──

    fn find_input_signal_index(&self, reg_val: u32) -> Option<u32> {
        for i in 0..(self.input_signal_count as usize) {
            if let Some(ref entry) = self.input_signals[i] {
                if entry.reg_val == reg_val {
                    return Some(i as u32);
                }
            }
        }
        None
    }

    fn find_or_create_pin_set(&mut self, pin: u32) -> u32 {
        for i in 0..(self.pin_input_set_count as usize) {
            if self.pin_input_sets[i].pin == pin {
                return i as u32;
            }
        }
        if (self.pin_input_set_count as usize) < MAX_PIN_SIGNAL_SETS {
            let idx = self.pin_input_set_count;
            self.pin_input_sets[idx as usize] = PinSignalSet {
                pin,
                signals: [0; MAX_SIGNALS_IN_SET],
                signal_count: 0,
            };
            self.pin_input_set_count += 1;
            idx
        } else {
            0
        }
    }

    fn find_gpio_map_index(&self, gpio_num: i32) -> Option<u32> {
        for i in 0..(self.gpio_map_count as usize) {
            if self.gpio_map[i].gpio_num == gpio_num {
                return Some(i as u32);
            }
        }
        None
    }

    fn set_gpio_map_entry(&mut self, gpio_num: i32, out_idx: i32) {
        if let Some(idx) = self.find_gpio_map_index(gpio_num) {
            self.gpio_map[idx as usize].out_idx = out_idx;
        } else if (self.gpio_map_count as usize) < MAX_GPIO_MAP_ENTRIES {
            self.gpio_map[self.gpio_map_count as usize] = GpioMapEntry { gpio_num, out_idx };
            self.gpio_map_count += 1;
        }
    }
}

// ============================================================
// GPIO Pin State
// ============================================================

#[derive(Clone, Copy)]
pub struct GpioPinState {
    pub input_value: u32,
}

impl GpioPinState {
    pub const fn new() -> Self { GpioPinState { input_value: 0 } }
}

// ============================================================
// Timer Types
// ============================================================

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum TimerMode {
    Increment = 0,
    Decrement = 1,
    ZigZag = 2,
}

pub struct Timer32Counter {
    pub base_value: u32,
    pub base_ticks: u64,
    pub bit_count: u32,
    pub mask: u32,
    pub top_value: u32,
    pub prescaler_value: u32,
    pub timer_mode: TimerMode,
    pub enabled: bool,
}

impl Timer32Counter {
    pub fn new() -> Self {
        Timer32Counter {
            base_value: 0,
            base_ticks: 0,
            bit_count: 32,
            mask: 0xFFFF_FFFF,
            top_value: 0xFFFF_FFFF,
            prescaler_value: 1,
            timer_mode: TimerMode::Increment,
            enabled: false,
        }
    }

    pub fn counter(&self, current_ticks: u64) -> u32 {
        if self.prescaler_value == 0 || !self.enabled {
            return self.base_value;
        }
        let delta_ticks = current_ticks.wrapping_sub(self.base_ticks);
        let elapsed = delta_ticks / self.prescaler_value as u64;
        let raw = if self.timer_mode == TimerMode::Decrement {
            self.base_value.wrapping_sub(elapsed as u32)
        } else {
            self.base_value.wrapping_add(elapsed as u32)
        };
        let modulus = if self.top_value != 0xFFFF_FFFF { self.top_value + 1 } else { 0xFFFF_FFFF };
        let val = if modulus != 0 && self.top_value != 0xFFFF_FFFF {
            raw % modulus
        } else {
            raw
        };
        val & self.mask
    }

    pub fn set(&mut self, val: u32, current_ticks: u64) {
        self.base_value = val & self.mask;
        self.base_ticks = current_ticks;
    }

    pub fn set_bits(&mut self, bits: u32) {
        self.bit_count = bits;
        self.mask = if bits >= 32 { 0xFFFF_FFFF } else { (1u32 << bits) - 1 };
    }

    pub fn prescaler(&self) -> u32 { self.prescaler_value }
    pub fn set_prescaler(&mut self, val: u32, current_ticks: u64) {
        self.base_value = self.counter(current_ticks);
        self.base_ticks = current_ticks;
        self.prescaler_value = val;
    }

    pub fn enable(&self) -> bool { self.enabled }
    pub fn set_enable(&mut self, en: bool, current_ticks: u64) {
        if en != self.enabled {
            if en {
                self.base_ticks = current_ticks;
            } else {
                self.base_value = self.counter(current_ticks);
            }
            self.enabled = en;
        }
    }

    pub fn to_ticks(&self, counter_val: u32) -> u32 {
        counter_val * self.prescaler_value
    }

    pub fn mask_value(&self) -> u32 { self.mask }
}

// ============================================================
// CpuContext — passed to every peripheral read_u32/write_u32
// ============================================================

pub struct CpuContext<'a> {
    pub apb_ticks_val: u64,
    pub clock_nanos_val: u64,
    pub cpu_tick: u64,
    pub chip_name: &'a str,
    pub int_matrix: &'a mut InterruptMatrix,
    pub gpio_matrix: &'a mut GpioMatrix,
    pub gpio_pins: &'a mut [GpioPinState],
    pub event_queue: &'a mut EventQueue,
    pub clocks: &'a mut Clocks,
}

impl CpuContext<'_> {
    pub fn interrupt(&mut self, irq: u32, level: bool) {
        self.int_matrix.interrupt(irq, level);
    }

    pub fn interrupt_target(&mut self, irq: u32, level: bool, _target: u32) {
        self.int_matrix.interrupt(irq, level);
    }

    pub fn apb_ticks(&self) -> u64 { self.apb_ticks_val }
    pub fn clock_nanos(&self) -> u64 { self.clock_nanos_val }

    pub fn schedule_event(&mut self, delta_ticks: u64, tag: EventTag) -> usize {
        self.event_queue.schedule(delta_ticks, tag, self.apb_ticks_val)
    }

    pub fn unschedule_event(&mut self, tag: EventTag) {
        self.event_queue.unschedule_all(tag);
    }

    pub fn has_event(&self, tag: EventTag) -> bool {
        self.event_queue.has_scheduled(tag)
    }

    pub fn reset_core(&mut self, core_idx: u32) {
        unsafe { crate::peripherals::common::ffi::js_reset_core(core_idx) }
    }
    pub fn reset_soc(&mut self) {
        unsafe { crate::peripherals::common::ffi::js_reset_soc() }
    }
    pub fn on_reset(&mut self) -> bool { true }
    pub fn zero_sensitive_memory(&mut self) {}
    pub fn set_reset_reason(&mut self, reason: u32) {
        unsafe { crate::peripherals::common::ffi::js_set_reset_reason(reason) }
    }
    pub fn reset_reason(&self) -> u32 {
        unsafe { crate::peripherals::common::ffi::js_get_reset_reason() }
    }
    pub fn is_cpu_stalled(&self, _core_idx: u32) -> bool { false }
    pub fn set_core_enabled(&mut self, idx: u32, enabled: bool) {
        unsafe { crate::peripherals::common::ffi::js_set_core_enabled(idx, enabled as u32) }
    }
    pub fn enter_light_sleep(&mut self, idx: u32) {
        unsafe { crate::peripherals::common::ffi::js_core_enter_light_sleep(idx) }
    }
    pub fn exit_light_sleep(&mut self, idx: u32) {
        unsafe { crate::peripherals::common::ffi::js_core_exit_light_sleep(idx) }
    }
    pub fn update_clocks(&mut self) { self.clocks.update(); }
    pub fn cpu_clock_period(&self) -> u32 { self.clocks.cpu_clock_period }
    pub fn set_cpu_clock_period(&mut self, val: u32) { self.clocks.cpu_clock_period = val; }

    pub fn mem_read_u32(&self, _addr: u32) -> u32 { 0 }
    pub fn mem_read_u8(&self, _addr: u32) -> u8 { 0 }
    pub fn mem_write_u8(&mut self, _addr: u32, _val: u8) {}
    pub fn mem_write_u32(&mut self, _addr: u32, _val: u32) {}
    pub fn gdb_read_register(&self, _core: u32, _reg: u32) -> u32 { 0 }
    pub fn is_xtensa_core(&self, _core: u32) -> bool { false }
    pub fn gdb_register_type(&self, _core: u32, _reg: u32) -> u32 { 0 }
}

// ============================================================
// MmioPeripheral trait
// ============================================================

pub trait MmioPeripheral {
    fn read_u32(&mut self, ctx: &mut CpuContext, addr: u32) -> u32;
    fn write_u32(&mut self, ctx: &mut CpuContext, addr: u32, val: u32);
    fn reset(&mut self);
}
