// Bidirectional enum helper: keeps both `Name: value` and `value: "Name"` lookups.
function defineEnum(map) {
  for (const [key, value] of Object.entries(map)) map[value] = key;
  return map;
}

export let CpuClockSource = defineEnum({
  XTL_CLK: 0,
  PLL_CLK: 1,
  RTC8M_CLK: 2,
  APLL_CLK: 3,
});

export const MsToNsFactor = 1e3;
export const UsToNsFactor = 1e6;

class ClockScheduledEvent {
  constructor(clock, callback) {
    this.clock = clock;
    this.rootClock = clock.rootClock;
    this.onEnableChange = (enabled) => {
      if (enabled && this.scheduledTargetTicks !== undefined) this.reschedule();
    };
    this.rootEvent = this.rootClock.createEvent(() => {
      if (this.clock.enabled) {
        this.clearPending();
        callback();
      }
    });
  }
  reschedule() {
    const deltaTicks = this.scheduledTargetTicks - this.clock.ticks;
    if (deltaTicks <= 0) this.rootEvent.schedule(0);
    else this.rootEvent.schedule((1e9 * deltaTicks) / this.clock.frequency);
  }
  clearPending() {
    if (this.scheduledTargetTicks !== undefined) {
      this.clock.removeEnableListener(this.onEnableChange);
      this.scheduledTargetTicks = undefined;
    }
  }
  schedule(ticks) {
    if (this.scheduledTargetTicks === undefined)
      this.clock.addEnableListener(this.onEnableChange);
    this.scheduledTargetTicks = this.clock.ticks + ticks;
    this.rootEvent.schedule((1e9 * ticks) / this.clock.frequency);
  }
  unschedule() {
    this.clearPending();
    this.rootEvent.unschedule();
  }
}

class RootClock {
  constructor(rootClock, frequency) {
    this.rootClock = rootClock;
    this.frequencyValue = frequency;
    this.listeners = new Set();
    this.enableListeners = new Set();
    this.baseTicks = 0;
    this.baseNanos = 0;
    this.enabledValue = true;
  }
  get enabled() {
    return this.enabledValue;
  }
  setEnabled(enabled) {
    if (enabled !== this.enabledValue) {
      const nanos = this.rootClock.nanos;
      if (!Number.isNaN(nanos)) {
        if (!enabled) this.baseTicks = this.ticks;
        this.baseNanos = nanos;
      }
      this.enabledValue = enabled;
      for (const cb of this.enableListeners) cb(enabled);
    }
  }
  get ticks() {
    if (!this.enabledValue) return this.baseTicks;
    const { baseTicks, baseNanos } = this;
    return ((this.rootClock.nanos - baseNanos) / 1e9) * this.frequency + baseTicks;
  }
  setFrequency(frequency) {
    if (frequency !== this.frequencyValue) {
      const oldFrequency = this.frequencyValue;
      const nanos = this.rootClock.nanos;
      if (!Number.isNaN(nanos)) {
        this.baseTicks = this.ticks;
        this.baseNanos = nanos;
      }
      this.frequencyValue = frequency;
      for (const cb of this.listeners) cb(this.frequencyValue, oldFrequency);
    }
  }
  get frequency() {
    return this.frequencyValue;
  }
  createEvent(callback) {
    return new ClockScheduledEvent(this, callback);
  }
  addFrequencyListener(cb) {
    this.listeners.add(cb);
  }
  removeFrequencyListener(cb) {
    this.listeners.delete(cb);
  }
  addEnableListener(cb) {
    this.enableListeners.add(cb);
  }
  removeEnableListener(cb) {
    this.enableListeners.delete(cb);
  }
}

class DerivedClock extends RootClock {
  constructor(parentClock, multiplier) {
    super(parentClock.rootClock, parentClock.frequency * multiplier);
    this.parentClock = parentClock;
    this.multiplierValue = multiplier;
    this.ownEnabled = true;
    this.updateFrequency = () => {
      const current = this.frequencyValue;
      const next = this.parentClock.frequency * this.multiplierValue;
      if (next !== current) this.setFrequency(next);
    };
    this.updateEnable = () => {
      this.setEnabled(this.ownEnabled && this.parentClock.enabled);
    };
    this.enabledValue = parentClock.enabled;
    parentClock.addFrequencyListener(this.updateFrequency);
    parentClock.addEnableListener(this.updateEnable);
  }
  get enable() {
    return this.ownEnabled;
  }
  set enable(enabled) {
    if (enabled !== this.ownEnabled) {
      this.ownEnabled = enabled;
      this.setEnabled(enabled && this.parentClock.enabled);
    }
  }
  get multiplier() {
    return this.multiplierValue;
  }
  set multiplier(value) {
    this.multiplierValue = value;
    this.updateFrequency();
  }
  get divider() {
    return this.multiplierValue ? 1 / this.multiplierValue : 0;
  }
  set divider(value) {
    this.multiplierValue = value ? 1 / value : 0;
    this.updateFrequency();
  }
  get parent() {
    return this.parentClock;
  }
  set parent(value) {
    if (value !== this.parentClock) {
      this.parentClock.removeFrequencyListener(this.updateFrequency);
      this.parentClock.removeEnableListener(this.updateEnable);
      this.parentClock = value;
      value.addFrequencyListener(this.updateFrequency);
      value.addEnableListener(this.updateEnable);
      this.updateFrequency();
      this.updateEnable();
    }
  }
}

class MutableClock extends RootClock {
  get frequency() {
    return this.frequencyValue;
  }
  set frequency(value) {
    this.setFrequency(value);
  }
  get enable() {
    return this.enabledValue;
  }
  set enable(value) {
    this.setEnabled(value);
  }
}

// ChipRootClock — the base clock of the ClockTree. Replaces the old
// SimulationClock: nanos is derived directly from chip.cycles (the cycle
// counter, now authoritative in WASM as CLK_CYCLES), and it owns a small
// event queue for ClockScheduledEvent (ccompare / beacon / wifi-tx). Due
// events are pumped from the worker's 512-step cadence (fireDueEvents),
// not per emulated step, so the JS clock stays out of the execution hot path.
class RootClockEvent {
  constructor(clock, callback) {
    this.clock = clock;
    this.callback = callback;
    this.cycles = 0;
    this.next = null;
  }
  schedule(nanos) {
    this.clock.unschedule(this.callback);
    const delta = Math.max(1, Math.round((nanos / 1e9) * this.clock.frequencyValue));
    const target = this.clock.chip.cycles + delta;
    this.cycles = target;
    let cur = this.clock.nextClockEvent;
    let prev = null;
    while (cur && cur.cycles < target) {
      prev = cur;
      cur = cur.next;
    }
    if (prev) prev.next = this;
    else this.clock.nextClockEvent = this;
    this.next = cur;
  }
  unschedule() {
    this.clock.unschedule(this.callback);
  }
}

class ChipRootClock {
  constructor(chip) {
    this.chip = chip;
    this.frequencyValue = 16e7;
    this.nextClockEvent = null;
    this.clockEventPool = [];
  }
  get frequency() {
    return this.frequencyValue;
  }
  get nanos() {
    return (this.chip.cycles / this.frequencyValue) * 1e9;
  }
  createEvent(callback) {
    return new RootClockEvent(this, callback);
  }
  unschedule(callback) {
    let cur = this.nextClockEvent;
    let prev = null;
    while (cur) {
      if (cur.callback === callback) {
        if (prev) prev.next = cur.next;
        else this.nextClockEvent = cur.next;
        if (this.clockEventPool.length < 10) this.clockEventPool.push(cur);
        return true;
      }
      prev = cur;
      cur = cur.next;
    }
    return false;
  }
  fireDueEvents() {
    let ev = this.nextClockEvent;
    while (ev && ev.cycles <= this.chip.cycles) {
      this.nextClockEvent = ev.next;
      if (this.clockEventPool.length < 10) this.clockEventPool.push(ev);
      ev.callback();
      ev = this.nextClockEvent;
    }
  }
}

class ClockTree {
  constructor(root) {
    this.root = root;
    this.rcFast = new MutableClock(root, 8 * UsToNsFactor);
    this.rcFastDiv = new DerivedClock(this.rcFast, 1 / 256);
    this.rtc8m = this.rcFast;
    this.rtc8mDiv256 = this.rcFastDiv;
    this.rcSlow = new MutableClock(root, 150 * MsToNsFactor);
    this.xtal = new MutableClock(root, 40 * UsToNsFactor);
    this.xtal32k = new MutableClock(root, 32768);
    this.freq80mhz = new MutableClock(root, 80 * UsToNsFactor);
    this.apll = new MutableClock(root, 100 * UsToNsFactor);
    this.sysclk = new DerivedClock(this.xtal, 1);
    this.cpu = new DerivedClock(this.sysclk, 1);
    this.apb = new DerivedClock(this.sysclk, 1);
    this.ref = new DerivedClock(this.apb, 1);
    this.tg0Timer = new DerivedClock(this.xtal, 1);
    this.tg0WDT = new DerivedClock(this.freq80mhz, 1);
    this.tg1Timer = new DerivedClock(this.xtal, 1);
    this.tg1WDT = new DerivedClock(this.freq80mhz, 1);
    this.ledc = new DerivedClock(this.xtal, 1);
    this.rmt = new DerivedClock(this.xtal, 1);
    this.uart0 = new DerivedClock(this.xtal, 1);
    this.uart1 = new DerivedClock(this.xtal, 1);
    this.uart2 = new DerivedClock(this.xtal, 1);
    this.spi2 = new DerivedClock(this.xtal, 1);
    this.i2c0 = new DerivedClock(this.xtal, 1);
    this.cpuClockSource = CpuClockSource.XTL_CLK;
    this.cpuClockPeriod = 0;
    this.sysclkPreDiv = 0;
    this.xtalTicks = 0;
    this.pllTicks = 0;
    this.rtc8mTicks = 0;
    this.apllTicks = 0;
    this.savedTg0WdtEnabled = true;
    this.savedTg1WdtEnabled = true;
    this.update();
  }
  update() {
    this.apb.divider = 1;
    this.cpu.divider = 1;
    switch (this.cpuClockSource) {
      case CpuClockSource.XTL_CLK:
        this.sysclk.parent = this.xtal;
        this.sysclk.divider = this.sysclkPreDiv + 1;
        this.ref.divider = 1 + this.xtalTicks;
        break;
      case CpuClockSource.PLL_CLK:
        this.sysclk.parent = this.freq80mhz;
        this.sysclk.divider = 1;
        this.cpu.multiplier =
          this.cpuClockPeriod === 2 ? 3 : this.cpuClockPeriod === 1 ? 2 : 1;
        this.ref.divider = 1 + this.pllTicks;
        break;
      case CpuClockSource.RTC8M_CLK:
        this.sysclk.parent = this.rcFast;
        this.sysclk.divider = this.sysclkPreDiv + 1;
        this.ref.divider = 1 + this.rtc8mTicks;
        break;
      case CpuClockSource.APLL_CLK:
        this.sysclk.parent = this.apll;
        this.sysclk.divider = this.cpuClockPeriod === 0 ? 4 : 2;
        this.apb.divider = 2;
        this.ref.divider = 1 + this.apllTicks;
        break;
    }
  }
  pauseApbClocks() {
    this.savedTg0WdtEnabled = this.tg0WDT.enable;
    this.savedTg1WdtEnabled = this.tg1WDT.enable;
    this.tg0WDT.enable = false;
    this.tg1WDT.enable = false;
  }
  resumeApbClocks() {
    this.tg0WDT.enable = this.savedTg0WdtEnabled;
    this.tg1WDT.enable = this.savedTg1WdtEnabled;
  }
}

export {
  ClockScheduledEvent,
  RootClock,
  DerivedClock,
  MutableClock,
  ChipRootClock,
  ClockTree,
};
