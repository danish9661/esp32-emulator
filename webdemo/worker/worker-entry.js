var __defProp = Object.defineProperty;
var __getOwnPropNames = Object.getOwnPropertyNames;
var __esm = (fn, res, err) => function __init() {
  if (err) throw err[0];
  try {
    return fn && (res = (0, fn[__getOwnPropNames(fn)[0]])(fn = 0)), res;
  } catch (e) {
    throw err = [e], e;
  }
};
var __export = (target, all) => {
  for (var name in all)
    __defProp(target, name, { get: all[name], enumerable: true });
};

// src/engine/wasm-memory-layout.js
var wasm_memory_layout_exports = {};
__export(wasm_memory_layout_exports, {
  CORES: () => CORES,
  CORE_STATE_SIZE: () => CORE_STATE_SIZE,
  FLASH_DATA_OFFSET: () => FLASH_DATA_OFFSET,
  MMU_TABLE_REGION_ID: () => MMU_TABLE_REGION_ID,
  NATIVE_HANDLER_FLAG: () => NATIVE_HANDLER_FLAG,
  PAGE_ENTRIES: () => PAGE_ENTRIES2,
  PAGE_SHIFT: () => PAGE_SHIFT2,
  PAGE_SIZE: () => PAGE_SIZE2,
  PAGE_TABLE_OFFSET: () => PAGE_TABLE_OFFSET,
  PAGE_TABLE_SIZE: () => PAGE_TABLE_SIZE,
  RAM_BUDGET: () => RAM_BUDGET,
  RAM_DATA_OFFSET: () => RAM_DATA_OFFSET,
  REGION_TABLE_ENTRIES: () => REGION_TABLE_ENTRIES,
  REGION_TABLE_OFFSET: () => REGION_TABLE_OFFSET,
  REGION_TABLE_SIZE: () => REGION_TABLE_SIZE,
  SHA_DATA_OFFSET: () => SHA_DATA_OFFSET,
  SHA_DATA_SIZE: () => SHA_DATA_SIZE,
  STATIC_ZONE: () => STATIC_ZONE,
  TOTAL_SIZE: () => TOTAL_SIZE
});
var CORE_STATE_SIZE, CORES, PAGE_SHIFT2, PAGE_SIZE2, PAGE_ENTRIES2, PAGE_TABLE_SIZE, SHA_DATA_SIZE, SHA_DATA_OFFSET, STATIC_ZONE, PAGE_TABLE_OFFSET, REGION_TABLE_ENTRIES, REGION_TABLE_SIZE, REGION_TABLE_OFFSET, RAM_DATA_OFFSET, RAM_BUDGET, FLASH_DATA_OFFSET, MMU_TABLE_REGION_ID, TOTAL_SIZE, NATIVE_HANDLER_FLAG;
var init_wasm_memory_layout = __esm({
  "src/engine/wasm-memory-layout.js"() {
    CORE_STATE_SIZE = 4096;
    CORES = 20;
    PAGE_SHIFT2 = 12;
    PAGE_SIZE2 = 1 << PAGE_SHIFT2;
    PAGE_ENTRIES2 = 1048576;
    PAGE_TABLE_SIZE = PAGE_ENTRIES2 * 8;
    SHA_DATA_SIZE = 128;
    SHA_DATA_OFFSET = CORE_STATE_SIZE * CORES;
    STATIC_ZONE = 4 * 1024 * 1024;
    PAGE_TABLE_OFFSET = STATIC_ZONE;
    REGION_TABLE_ENTRIES = 32;
    REGION_TABLE_SIZE = REGION_TABLE_ENTRIES * 8;
    REGION_TABLE_OFFSET = PAGE_TABLE_OFFSET + PAGE_TABLE_SIZE;
    RAM_DATA_OFFSET = REGION_TABLE_OFFSET + REGION_TABLE_SIZE;
    RAM_BUDGET = 6 * 1024 * 1024;
    FLASH_DATA_OFFSET = RAM_DATA_OFFSET + RAM_BUDGET;
    MMU_TABLE_REGION_ID = 5;
    TOTAL_SIZE = FLASH_DATA_OFFSET + 4 * 1024 * 1024;
    NATIVE_HANDLER_FLAG = 2147483648;
  }
});

// src/peripherals/common/trace.js
var ESPTrace = class {
  constructor() {
    this.traces = [];
    this.uartBuf = ["", ""];
    this.traceReturns = false;
    this.traceMemWrites = false;
    this.dumpThreshold = 1e5;
  }
  traceLog(direction, data, meta) {
    this.traces.push(
      `SignalDirection,${direction},${JSON.stringify(data)},${JSON.stringify(meta)}`
    );
  }
  traceUartTx(core, byte) {
    if (byte === 10) {
      const line = this.uartBuf[core];
      this.traces.push(`ClockEvent,${core},${JSON.stringify(line)}`);
      this.uartBuf[core] = "";
    } else if (this.uartBuf[core].length < 255) {
      this.uartBuf[core] += String.fromCharCode(byte);
    }
  }
  traceEntry(pc, insn, meta) {
    if (!this.traceReturns) return;
    this.traces.push(`e,${pc},${insn},${meta}`);
    if (this.traces.length > this.dumpThreshold) this.dumpTrace();
  }
  traceReturn(pc, insn, meta, cycles) {
    if (!this.traceReturns) return;
    this.traces.push(`r,${pc},${insn},${meta},${cycles}`);
    if (this.traces.length > this.dumpThreshold) this.dumpTrace();
  }
  int(_pc, _insn) {
  }
  iret(_pc, _insn) {
  }
  traceMemWrite(addr, size, value, oldValue, newValue) {
    if (!this.traceMemWrites) return;
    this.traces.push(`w,${addr},${size},${value},${oldValue},${newValue}`);
    if (this.traces.length > this.dumpThreshold) this.dumpTrace();
  }
  dumpTrace() {
    this.traces = [];
  }
};

// src/peripherals/common/clocks.js
function defineEnum(map) {
  for (const [key, value] of Object.entries(map)) map[value] = key;
  return map;
}
var CpuClockSource = defineEnum({
  XTL_CLK: 0,
  PLL_CLK: 1,
  RTC8M_CLK: 2,
  APLL_CLK: 3
});
var MsToNsFactor = 1e3;
var UsToNsFactor = 1e6;
var ClockScheduledEvent = class {
  constructor(clock, callback) {
    this.clock = clock;
    this.rootClock = clock.rootClock;
    this.onEnableChange = (enabled) => {
      if (enabled && this.scheduledTargetTicks !== void 0) this.reschedule();
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
    else this.rootEvent.schedule(1e9 * deltaTicks / this.clock.frequency);
  }
  clearPending() {
    if (this.scheduledTargetTicks !== void 0) {
      this.clock.removeEnableListener(this.onEnableChange);
      this.scheduledTargetTicks = void 0;
    }
  }
  schedule(ticks) {
    if (this.scheduledTargetTicks === void 0)
      this.clock.addEnableListener(this.onEnableChange);
    this.scheduledTargetTicks = this.clock.ticks + ticks;
    this.rootEvent.schedule(1e9 * ticks / this.clock.frequency);
  }
  unschedule() {
    this.clearPending();
    this.rootEvent.unschedule();
  }
};
var RootClock = class {
  constructor(rootClock, frequency) {
    this.rootClock = rootClock;
    this.frequencyValue = frequency;
    this.listeners = /* @__PURE__ */ new Set();
    this.enableListeners = /* @__PURE__ */ new Set();
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
    return (this.rootClock.nanos - baseNanos) / 1e9 * this.frequency + baseTicks;
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
};
var DerivedClock = class extends RootClock {
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
};
var MutableClock = class extends RootClock {
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
};
var RootClockEvent = class {
  constructor(clock, callback) {
    this.clock = clock;
    this.callback = callback;
    this.cycles = 0;
    this.next = null;
  }
  schedule(nanos) {
    this.clock.unschedule(this.callback);
    const delta = Math.max(1, Math.round(nanos / 1e9 * this.clock.frequencyValue));
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
};
var ChipRootClock = class {
  constructor(chip2) {
    this.chip = chip2;
    this.frequencyValue = 16e7;
    this.nextClockEvent = null;
    this.clockEventPool = [];
  }
  get frequency() {
    return this.frequencyValue;
  }
  get nanos() {
    return this.chip.cycles / this.frequencyValue * 1e9;
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
};
var ClockTree = class {
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
        this.cpu.multiplier = this.cpuClockPeriod === 2 ? 3 : this.cpuClockPeriod === 1 ? 2 : 1;
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
};

// src/peripherals/common/memory.js
var Memory = class _Memory {
  /** @param {Uint8Array} data @param {number} base */
  constructor(data, base) {
    this.data = data, this.base = base;
  }
  /** @returns {number} absolute base address */
  get baseAddr() {
    return this.base;
  }
  /** @param {number} addr @returns {boolean} */
  contains(addr) {
    return addr >= this.base && addr < this.base + this.data.length;
  }
  /** @param {number} addr @returns {number} */
  readUint8(addr) {
    let { data: tmpVal, base: idxVal } = this;
    return tmpVal[addr - idxVal];
  }
  /** @param {number} addr @returns {number} */
  readUint16(addr) {
    let { data: tmpVal, base: idxVal } = this;
    return tmpVal[addr - idxVal] | tmpVal[addr - idxVal + 1] << 8;
  }
  /** @param {number} addr @returns {number} */
  readUint32(addr) {
    let { data: tmpVal, base: idxVal } = this, off = addr - idxVal;
    return (tmpVal[off] | tmpVal[off + 1] << 8 | tmpVal[off + 2] << 16 | tmpVal[off + 3] << 24) >>> 0;
  }
  /** @param {number} addr @param {number} val */
  writeUint8(addr, val) {
    let { data: idxVal, base: ClockEvent } = this;
    idxVal[addr - ClockEvent] = val;
  }
  /** @param {number} addr @param {number} val */
  writeUint16(addr, val) {
    let { data: idxVal, base: ClockEvent } = this;
    idxVal[addr - ClockEvent] = 255 & val, idxVal[addr - ClockEvent + 1] = val >> 8 & 255;
  }
  /** @param {number} addr @param {number} val */
  writeUint32(addr, val) {
    let { data: idxVal, base: ClockEvent } = this, off = addr - ClockEvent;
    idxVal[off] = 255 & val, idxVal[off + 1] = val >> 8 & 255, idxVal[off + 2] = val >> 16 & 255, idxVal[off + 3] = val >> 24 & 255;
  }
  /** @param {Uint8Array} src @param {number} srcBase */
  set(src, srcBase) {
    this.data.set(src, srcBase - this.base);
  }
  /** @param {Memory} src @param {number} srcAddr @param {number} dstAddr @param {number} len */
  copy(src, srcAddr, dstAddr, len) {
    this.data.set(
      src.data.subarray(
        srcAddr - src.base,
        srcAddr - src.base + len
      ),
      dstAddr - this.base
    );
  }
  /** @param {number} base @param {number} len @returns {Memory} */
  createView(base, len) {
    return new _Memory(this.data.subarray(len, len + base), base);
  }
  /** @param {number} base @returns {Memory} */
  remap(base) {
    return new _Memory(this.data, base);
  }
};
var ReadonlyMemory = class _ReadonlyMemory extends Memory {
  /** @param {number} addr @param {number} val */
  writeUint8(addr, val) {
    if (_ReadonlyMemory.override) return super.writeUint8(addr, val);
    console.error("Invalid write to read-only memory at", addr.toString(16));
  }
  /** @param {number} addr @param {number} val */
  writeUint16(addr, val) {
    if (_ReadonlyMemory.override) return super.writeUint16(addr, val);
    console.error("Invalid write to read-only memory at", addr.toString(16));
  }
  /** @param {number} addr @param {number} val */
  writeUint32(addr, val) {
    if (_ReadonlyMemory.override) return super.writeUint32(addr, val);
    console.error("Invalid write to read-only memory at", addr.toString(16));
  }
  /** @param {number} base @param {number} len @returns {ReadonlyMemory} */
  createView(base, len) {
    return new _ReadonlyMemory(
      this.data.subarray(len, len + base),
      base
    );
  }
};
ReadonlyMemory.override = false;
var InvalidMemory = class {
  /** @param {number} [defaultValue=0xFFFFFFFF] */
  constructor(defaultValue = 4294967295) {
    this.defaultValue = defaultValue;
  }
  /** @returns {number} */
  readUint8() {
    return 255 & this.defaultValue;
  }
  /** @returns {number} */
  readUint16() {
    return 65535 & this.defaultValue;
  }
  /** @returns {number} */
  readUint32() {
    return 0 | this.defaultValue;
  }
  writeUint8() {
  }
  writeUint16() {
  }
  writeUint32() {
  }
};
var MemoryTranslator = class {
  /** @param {Memory} base @param {number} delta */
  constructor(base, delta) {
    this.base = base, this.delta = delta;
  }
  /** @param {number} addr @returns {number} */
  readUint8(addr) {
    return this.base.readUint8(addr - this.delta);
  }
  /** @param {number} addr @returns {number} */
  readUint16(addr) {
    return this.base.readUint16(addr - this.delta);
  }
  /** @param {number} addr @returns {number} */
  readUint32(addr) {
    return this.base.readUint32(addr - this.delta);
  }
  /** @param {number} addr @param {number} val */
  writeUint8(addr, val) {
    this.base.writeUint8(addr - this.delta, val);
  }
  /** @param {number} addr @param {number} val */
  writeUint16(addr, val) {
    this.base.writeUint16(addr - this.delta, val);
  }
  /** @param {number} addr @param {number} val */
  writeUint32(addr, val) {
    this.base.writeUint32(addr - this.delta, val);
  }
};
var ReverseMemory = class {
  /** @param {Uint8Array} data @param {number} base */
  constructor(data, base) {
    this.data = data, this.base = base;
  }
  /** @param {number} addr @returns {number} byte-reversed offset */
  translateAddress(addr) {
    let off = addr - this.base;
    return this.data.length - 4 - (4294967292 & off) + (3 & off);
  }
  /** @param {number} addr @returns {number} */
  readUint8(addr) {
    return this.data[this.translateAddress(addr)];
  }
  /** @param {number} addr @returns {number} */
  readUint16(addr) {
    let lo = this.translateAddress(addr), hi = this.translateAddress(addr + 1);
    return this.data[lo] | this.data[hi] << 8;
  }
  /** @param {number} addr @returns {number} */
  readUint32(addr) {
    let off = this.translateAddress(addr);
    return (this.data[off] | this.data[off + 1] << 8 | this.data[off + 2] << 16 | this.data[off + 3] << 24) >>> 0;
  }
  /** @param {number} addr @param {number} val */
  writeUint8(addr, val) {
    this.data[this.translateAddress(addr)] = val;
  }
  /** @param {number} addr @param {number} val */
  writeUint16(addr, val) {
    let lo = this.translateAddress(addr), hi = this.translateAddress(addr + 1);
    this.data[lo] = 255 & val, this.data[hi] = val >> 8 & 255;
  }
  /** @param {number} addr @param {number} val */
  writeUint32(addr, val) {
    let off = this.translateAddress(addr);
    this.data[off] = 255 & val, this.data[off + 1] = val >> 8 & 255, this.data[off + 2] = val >> 16 & 255, this.data[off + 3] = val >> 24 & 255;
  }
};
var MMUMemory = class extends Memory {
  /** @param {Uint8Array} data @param {number} base */
  constructor(data, base) {
    super(data, base), this.table = new Uint32Array(0), this.tableOffset = 0;
  }
  /** @type {Uint32Array} */
  table;
  /** @type {number} */
  tableOffset;
  /** @param {Uint32Array} table @param {number} offset */
  setTable(table, offset) {
    this.table = table;
    this.tableOffset = offset;
  }
  /** @param {number} addr @returns {number} physical address */
  mapAddress(addr) {
    let off = addr - this.baseAddr, idx = off >>> 15, entry = this.table[this.tableOffset + idx] ?? idx;
    return this.baseAddr + (entry << 15) + (off & 32767);
  }
  /** @param {number} addr @returns {number} */
  readUint8(addr) {
    return addr = this.mapAddress(addr), super.readUint8(addr);
  }
  /** @param {number} addr @returns {number} */
  readUint16(addr) {
    return addr = this.mapAddress(addr), super.readUint16(addr);
  }
  /** @param {number} addr @returns {number} */
  readUint32(addr) {
    return addr = this.mapAddress(addr), super.readUint32(addr);
  }
  /** @param {number} addr @param {number} val */
  writeUint8(addr, val) {
    addr = this.mapAddress(addr), super.writeUint8(addr, val);
  }
  /** @param {number} addr @param {number} val */
  writeUint16(addr, val) {
    addr = this.mapAddress(addr), super.writeUint16(addr, val);
  }
  /** @param {number} addr @param {number} val */
  writeUint32(addr, val) {
    addr = this.mapAddress(addr), super.writeUint32(addr, val);
  }
};
var PAGE_SHIFT = 12;
var PAGE_SIZE = 1 << PAGE_SHIFT;
var PAGE_ENTRIES = 1048576;
var PT_SIZE = PAGE_ENTRIES * 2;
var PTE_TYPE_RAM = 1;
var PTE_TYPE_MMIO = 2;
var PTE_TYPE_FLASH = 3;
var PageTable = class {
  constructor() {
    this.table = new Uint32Array(PT_SIZE);
  }
  /** @param {number} addr @returns {number} */
  pageIndex(addr) {
    return addr >>> PAGE_SHIFT;
  }
  /** @param {number} addr @returns {number} PTE type */
  getType(addr) {
    return this.table[this.pageIndex(addr) * 2];
  }
  /** @param {number} addr @returns {number} PTE data (RAM offset or MMIO handler ID) */
  getData(addr) {
    return this.table[this.pageIndex(addr) * 2 + 1];
  }
  /** @param {number} baseAddr @param {number} size @param {number} type @param {number} data */
  setRange(baseAddr, size, type, data) {
    const start = this.pageIndex(baseAddr);
    const end = this.pageIndex(baseAddr + size - 1);
    for (let i = start; i <= end; i++) {
      this.table[i * 2] = type;
      this.table[i * 2 + 1] = data;
    }
  }
  /** @param {number} addr @param {number} type @param {number} data */
  setPage(addr, type, data) {
    const i = this.pageIndex(addr);
    this.table[i * 2] = type;
    this.table[i * 2 + 1] = data;
  }
};
var MMIOHandlerRegistry = class {
  constructor() {
    this.handlers = [];
  }
  /** @param {object} handler @returns {number} handler ID */
  register(handler) {
    const id = this.handlers.length;
    this.handlers.push(handler);
    return id;
  }
  /** @param {number} id @returns {object|undefined} */
  get(id) {
    return this.handlers[id];
  }
  /** @param {number} handlerId @param {number} addr @param {number} size @returns {number} */
  read(handlerId, addr, size) {
    const h = this.handlers[handlerId];
    if (!h) return 0;
    switch (size) {
      case 1:
        return h.mmio_read?.(addr, 1) ?? h.readUint8?.(addr) ?? 0;
      case 2:
        return h.mmio_read?.(addr, 2) ?? h.readUint16?.(addr) ?? 0;
      case 4:
        return h.mmio_read?.(addr, 4) ?? h.readUint32?.(addr) ?? 0;
      default:
        return 0;
    }
  }
  /** @param {number} handlerId @param {number} addr @param {number} val @param {number} size */
  write(handlerId, addr, val, size) {
    const h = this.handlers[handlerId];
    if (!h) return;
    switch (size) {
      case 1:
        h.mmio_write?.(addr, val, 1) ?? h.writeUint8?.(addr, val);
        break;
      case 2:
        h.mmio_write?.(addr, val, 2) ?? h.writeUint16?.(addr, val);
        break;
      case 4:
        h.mmio_write?.(addr, val, 4) ?? h.writeUint32?.(addr, val);
        break;
    }
  }
};

// src/peripherals/common/peripheral.js
var RegisterField = class {
  constructor(memory) {
    this.memory = memory;
  }
  read() {
    return this.memory[0];
  }
  write(value) {
    this.memory[0] = value;
  }
  bit(mask) {
    return !!(this.memory[0] & mask);
  }
  setBits(mask) {
    this.memory[0] |= mask;
  }
  clearBits(mask) {
    this.memory[0] &= ~mask;
  }
  field(shift, mask) {
    return this.memory[0] >> shift & mask;
  }
  setField(shift, mask, value) {
    const next = this.memory[0] & ~mask | (value & mask) << shift;
    this.memory[0] = next;
    return next;
  }
};
var PeripheralBase = class _PeripheralBase {
  constructor(cpu, baseAddr, name) {
    this.cpu = cpu;
    this.baseAddr = baseAddr;
    this.name = name;
    this.memory = new Uint8Array(new SharedArrayBuffer(4096));
    this.memoryView = new DataView(this.memory.buffer);
    this.registers = [];
  }
  readUint8(addr) {
    if (_PeripheralBase.debug)
      console.log(`"${this.name}".readUint8 ${(addr - this.baseAddr).toString(16)}`);
    return this.memory[addr - this.baseAddr];
  }
  readUint16(addr) {
    if (_PeripheralBase.debug)
      console.log(`"${this.name}".readUint16 ${(addr - this.baseAddr).toString(16)}`);
    return this.memoryView.getUint16(addr - this.baseAddr, true);
  }
  readUint32(addr) {
    if (_PeripheralBase.debug)
      console.log(`"${this.name}".readUint32 ${(addr - this.baseAddr).toString(16)}`);
    return this.memoryView.getUint32(addr - this.baseAddr, true);
  }
  writeUint8(addr, value) {
    if (_PeripheralBase.debug)
      console.log(
        `"${this.name}".writeUint8 ${(addr - this.baseAddr).toString(16)} ${value.toString(16)}`
      );
    this.memory[addr - this.baseAddr] = value;
  }
  writeUint16(addr, value) {
    if (_PeripheralBase.debug)
      console.log(
        `"${this.name}".writeUint16 ${(addr - this.baseAddr).toString(16)} ${value.toString(16)}`
      );
    this.memoryView.setUint16(addr - this.baseAddr, value, true);
  }
  writeUint32(addr, value) {
    if (_PeripheralBase.debug)
      console.log(
        `"${this.name}".writeUint32 ${(addr - this.baseAddr).toString(16)} ${value.toString(16)}`
      );
    this.memoryView.setUint32(addr - this.baseAddr, value, true);
  }
  zeroMemory() {
    this.memory.fill(0);
  }
  reset() {
  }
  readRegister(offset) {
    return this.memoryView.getUint32(offset, true);
  }
  writeRegister(offset, value) {
    this.memoryView.setUint32(offset, value, true);
  }
  readField(field) {
    return this.readRegister(field.reg) >> field.shift & field.mask;
  }
  writeField(field, value) {
    const current = this.readRegister(field.reg);
    const masked = field.mask << field.shift;
    const next = current & ~masked | value << field.shift & masked;
    this.writeRegister(field.reg, next);
  }
  writeFieldOpt(field, value) {
    if (field) this.writeField(field, value);
  }
  setRegisterBits(offset, bits) {
    const next = this.readRegister(offset) | bits;
    this.writeRegister(offset, next);
    return next;
  }
  clearRegisterBits(offset, bits) {
    const next = this.readRegister(offset) & ~bits;
    this.writeRegister(offset, next);
    return next;
  }
  register(offset) {
    return new RegisterField(new Uint32Array(this.memory.buffer, offset, 1));
  }
  dumpState(sink) {
    sink.writeBytes(this.memory);
  }
  restoreState(source) {
    source.readBytes(this.memory);
  }
};
PeripheralBase.debug = false;

// src/peripherals/common/uart.js
function getFieldValue(addr, value) {
  return addr >> value.shift & value.mask;
}
function clearFieldBits(addr, value) {
  return addr & ~(value.mask << value.shift);
}
var FIFO_SIZE = 128;
var REG_FIFO_DATA = 0;
var REG_INT_RAW = 4;
var REG_INT_ST = 8;
var REG_INT_ENA = 12;
var REG_INT_CLR = 16;
var REG_CLKDIV = 20;
var REG_CONF0 = 32;
var REG_CONF1 = 36;
var REG_STATUS = 28;
var BIT_AUTOBAUD_EN = 1;
var SHIFT_BIT_NUM = 2;
var MASK_BIT_NUM = 3;
var BIT_CONF_FLAG_4 = 4;
var BIT_TICK_REF_ON = 134217728;
var MASK_RX_FIFO_CNT = 255;
var SHIFT_RX_FIFO_CNT = 0;
var MASK_TX_FIFO_CNT = 255;
var SHIFT_TX_FIFO_CNT = 16;
var MASK_TX_STATE = 15;
var SHIFT_TX_STATE = 24;
var MASK_AT_CHAR = 255;
var SHIFT_AT_CHAR = 0;
var MASK_AT_NUM = 255;
var SHIFT_AT_NUM = 8;
var BIT_AT_CMD_DET = 262144;
var BIT_TX_DONE = 16384;
var BIT_RX_TIMEOUT = 256;
var BIT_TX_EMPTY_INT = 2;
var BIT_RX_FULL_INT = 1;
var TxStateMachine = {
  TX_IDLE: 0,
  TX_STRT: 1,
  TX_DAT0: 2,
  TX_DAT1: 3,
  TX_DAT2: 4,
  TX_DAT3: 5,
  TX_DAT4: 6,
  TX_DAT5: 7,
  TX_DAT6: 8,
  TX_DAT7: 9,
  TX_PRTY: 10,
  TX_STP1: 11,
  TX_STP2: 12,
  TX_DL0: 13,
  TX_DL1: 14
};
var UartController = class extends PeripheralBase {
  constructor(chip2, base, coreIndex, irq, clk, uartConfig) {
    super(chip2, base, coreIndex), this.index = irq, this.irq = clk, this.config = uartConfig, this.rxFIFO = [], this.txFIFO = [], this.rxFullThreshold = 0, this.txEmptyThreshold = 0, this.rxTimeoutThreshold = 10, this.txState = TxStateMachine.TX_IDLE, this.dataReceived = false, this._baudRate = 0, this.rxTimeout = this.cpu.clocks.cpu.createEvent(() => {
      this.setRegisterBits(REG_INT_RAW, BIT_RX_TIMEOUT), this.checkInterrupt();
    }), this.intCheckEvent = this.cpu.clocks.cpu.createEvent(() => {
      this.checkInterrupt();
    }), this.atChar = 0, this.atNum = 0, this.atCounter = 0, this.loopbackByte = -1, this.autobaudEnabled = false, this.autobaudRate = 115200, this._glitchFilterEnabled = false, this._glitchFilterThreshold = 0, this.onTX = () => {
    }, this.onConfigurationUpdated = () => {
    }, this.onAutoBaudUpdated = () => {
    }, this.onGlitchFilterUpdated = () => {
    };
    uartConfig.clockSource?.addFrequencyListener(() => this.updateBaudRate());
  }
  get baudRate() {
    return this._baudRate;
  }
  get intStatus() {
    return this.readRegister(REG_INT_RAW) & this.readRegister(REG_INT_ENA);
  }
  checkInterrupt() {
    let { rxFullThreshold: addr, txEmptyThreshold: value } = this, regOffset = this.readRegister(REG_INT_RAW) & ~(BIT_RX_FULL_INT | BIT_TX_EMPTY_INT);
    this.rxFIFO.length >= addr && (regOffset |= BIT_RX_FULL_INT), this.txFIFO.length <= value && (regOffset |= BIT_TX_EMPTY_INT), this.writeRegister(REG_INT_RAW, regOffset), this.cpu.interrupt(this.irq, 0 !== this.intStatus);
  }
  get clock() {
    let { clocks: addr } = this.cpu;
    if (this.config.clockSource) return this.config.clockSource;
    if (this.config.F.SCLK_SEL)
      switch (this.readField(this.config.F.SCLK_SEL)) {
        case 1:
        default:
          return addr.apb;
        case 2:
          return addr.rcFast;
        case 3:
          return addr.xtal;
      }
    return addr.ref ? this.readRegister(REG_CONF0) & BIT_TICK_REF_ON ? addr.apb : addr.ref : addr.apb;
  }
  get clockForAutoBaud() {
    let { clocks: addr } = this.cpu;
    return addr.ref ? addr.apb : this.clock;
  }
  get clockDiv() {
    if (this.config.clockSource) return 1;
    let { SCLK_DIV_NUM: addr } = this.config.F;
    return 1 + (addr ? this.readField(addr) : 0);
  }
  get glitchFilterThresholdNs() {
    if (!this._glitchFilterEnabled || 0 === this._glitchFilterThreshold)
      return 0;
    let addr = this.clockForAutoBaud.frequency / this.clockDiv;
    return Math.round(1e9 * this._glitchFilterThreshold / addr);
  }
  get glitchFilterEnabled() {
    return this._glitchFilterEnabled;
  }
  updateGlitchFilter(addr, value) {
    let regOffset = false;
    addr !== this._glitchFilterEnabled && (this._glitchFilterEnabled = addr, regOffset = true), void 0 !== value && value !== this._glitchFilterThreshold && (this._glitchFilterThreshold = value, regOffset = true), regOffset && this.onGlitchFilterUpdated(this);
  }
  updateBaudRate() {
    let addr = this.readRegister(REG_CLKDIV), value = 1048575 & addr, regOffset = addr >> 20 & 15;
    this._baudRate = Math.floor(
      this.clock.frequency / this.clockDiv / (value + regOffset / 16)
    ), this.onConfigurationUpdated(this);
  }
  get txBusy() {
    return this.txState !== TxStateMachine.TX_IDLE;
  }
  get irdaTxSuppressed() {
    let { F: fieldCfg } = this.config;
    return !!this.readField(fieldCfg.IRDA_EN) && !this.readField(fieldCfg.IRDA_TX_EN);
  }
  txUpdated() {
    let { F: fieldCfg } = this.config;
    if (!this.txBusy && this.txFIFO.length) {
      if (this.irdaTxSuppressed) {
        this.txFIFO.length = 0, this.loopbackByte = -1, this.setRegisterBits(REG_INT_RAW, BIT_TX_DONE), this.checkInterrupt();
        return;
      }
      let value = this.txFIFO.shift();
      this.clearRegisterBits(REG_INT_RAW, BIT_TX_DONE), this.readField(fieldCfg.LOOPBACK) ? this.loopbackByte = value : this.loopbackByte = -1, this.txState = TxStateMachine.TX_STRT, this.onTX(value), this.txComplete();
    } else this.txFIFO.length || this.setRegisterBits(REG_INT_RAW, BIT_TX_DONE);
    this.checkInterrupt();
  }
  txComplete() {
    let { F: fieldCfg } = this.config;
    this.readField(fieldCfg.LOOPBACK) && this.loopbackByte >= 0 && (this.feedByte(this.loopbackByte, true), this.loopbackByte = -1), this.txState = TxStateMachine.TX_IDLE, this.txUpdated();
  }
  startDetected() {
    this.rxTimeout.unschedule();
  }
  get irdaRxSuppressed() {
    let { F: fieldCfg } = this.config;
    return !!this.readField(fieldCfg.IRDA_EN) && !!this.readField(fieldCfg.IRDA_TX_EN);
  }
  feedByte(addr, value = false) {
    if (!(!value && (this.readField(this.config.F.LOOPBACK) || this.irdaRxSuppressed)))
      return addr === this.atChar ? (this.atCounter++, this.atCounter === this.atNum && (this.atCounter = 0, this.setRegisterBits(REG_INT_RAW, BIT_AT_CMD_DET), this.checkInterrupt())) : this.atCounter = 0, this.rxFIFO.length !== FIFO_SIZE && (this.rxFIFO.push(addr), this.dataReceived = true, this.checkInterrupt(), this.rxTimeout.schedule(
        this.rxTimeoutThreshold * (1e9 / this.baudRate)
      ), true);
  }
  get bitNum() {
    switch (this.readRegister(REG_CONF0) >> SHIFT_BIT_NUM & MASK_BIT_NUM) {
      case 0:
        return 5;
      case 1:
        return 6;
      case 2:
        return 7;
      default:
        return 8;
    }
  }
  updateAutoBaud(addr) {
    addr !== this.autobaudEnabled && (this.autobaudEnabled = addr, this.onAutoBaudUpdated(this));
  }
  readUint8(addr) {
    return 255 & this.readUint32(addr);
  }
  readUint16(addr) {
    return 65535 & this.readUint32(addr);
  }
  readUint32(addr) {
    let value = addr - this.baseAddr, { RmtChannelRegister: regOffset, F: fieldCfg } = this.config;
    switch (value) {
      case REG_FIFO_DATA: {
        let rxByte = this.rxFIFO.shift();
        if (null == rxByte) return 238;
        return this.checkInterrupt(), rxByte;
      }
      case REG_INT_ST:
        return this.intStatus;
      case regOffset.LOWPULSE:
      case regOffset.HIGHPULSE:
        return Math.min(
          fieldCfg.LOWPULSE_MIN_CNT.mask,
          (2 * this.clockForAutoBaud.frequency / this.autobaudRate - 2) / 2
        );
      case regOffset.NEGPULSE:
      case regOffset.POSPULSE:
        return Math.min(
          fieldCfg.NEGEDGE_MIN_CNT.mask,
          2 * this.clockForAutoBaud.frequency / this.autobaudRate - 1
        );
      case regOffset.RXD_CNT:
        if (this.dataReceived) return 255;
        break;
      case REG_STATUS: {
        let addr2 = (this.txFIFO.length & MASK_TX_FIFO_CNT) << SHIFT_TX_FIFO_CNT | (this.rxFIFO.length & MASK_RX_FIFO_CNT) << SHIFT_RX_FIFO_CNT;
        return this.config.hasTXState && (addr2 |= (this.txState & MASK_TX_STATE) << SHIFT_TX_STATE), addr2;
      }
      case regOffset.MEM_RX_STATUS: {
        let addr2 = this.rxFIFO.length;
        return addr2 < FIFO_SIZE ? addr2 << 13 : 0;
      }
      case regOffset.ID:
        return 2147483647 & this.readRegister(value);
      case regOffset.REG_UPDATE:
        return 0;
    }
    return super.readUint32(addr);
  }
  writeUint8(addr, value) {
    if (addr - this.baseAddr === REG_FIFO_DATA) {
      this.txFIFO.length < FIFO_SIZE && (this.txFIFO.push(value), this.txUpdated());
      return;
    }
    super.writeUint8(addr, value);
  }
  writeUint32(addr, value) {
    let regOffset = addr - this.baseAddr, { RmtChannelRegister: rmtCfg, F: fieldCfg } = this.config;
    if (regOffset === fieldCfg.RX_TOUT_THRHD.reg) {
      let addr2 = value >> fieldCfg.RX_TOUT_THRHD.shift & fieldCfg.RX_TOUT_THRHD.mask;
      if (this.config.toutMultiply) {
        let value2 = this.readRegister(REG_CONF0) & BIT_TICK_REF_ON;
        this.rxTimeoutThreshold = value2 ? 8 * addr2 : addr2 >> 3;
      } else this.rxTimeoutThreshold = addr2;
    }
    regOffset === fieldCfg.RXFIFO_RST.reg && getFieldValue(value, fieldCfg.RXFIFO_RST) && (this.rxFIFO.splice(0, this.rxFIFO.length), this.dataReceived = false, this.checkInterrupt(), value = clearFieldBits(value, fieldCfg.RXFIFO_RST)), regOffset === fieldCfg.TXFIFO_RST.reg && getFieldValue(value, fieldCfg.TXFIFO_RST) && (this.txFIFO.splice(0, this.txFIFO.length), this.txUpdated(), value = clearFieldBits(value, fieldCfg.TXFIFO_RST)), fieldCfg.AUTOBAUD_EN && regOffset === fieldCfg.AUTOBAUD_EN.reg && this.updateAutoBaud(
      !!getFieldValue(value, fieldCfg.AUTOBAUD_EN)
    );
    let sclkReg = fieldCfg.SCLK_SEL ? fieldCfg.SCLK_SEL.reg : -1;
    switch (regOffset) {
      case REG_FIFO_DATA:
        this.txFIFO.length < FIFO_SIZE && (this.txFIFO.push(255 & value), this.txUpdated());
        return;
      case REG_INT_ENA:
        super.writeRegister(REG_INT_ENA, value), this.intCheckEvent.schedule(100);
        return;
      case REG_INT_CLR:
        this.clearRegisterBits(REG_INT_RAW, value), this.checkInterrupt(), this.cpu.interrupt(this.irq, 0 !== this.intStatus);
        return;
      case REG_CONF0:
        super.writeUint32(addr, value), this.updateBaudRate();
        break;
      case REG_CONF1:
        this.rxFullThreshold = getFieldValue(
          value,
          fieldCfg.RXFIFO_FULL_THRHD
        ), this.txEmptyThreshold = getFieldValue(
          value,
          fieldCfg.TXFIFO_EMPTY_THRHD
        );
        break;
      case REG_CLKDIV:
      case sclkReg:
        super.writeUint32(addr, value), this.updateBaudRate();
        return;
      case rmtCfg.RX_FILT:
        if (rmtCfg.RX_FILT >= 0) {
          let regOffset2 = fieldCfg.GLITCH_FILT ? getFieldValue(value, fieldCfg.GLITCH_FILT) : 0, rmtCfg2 = fieldCfg.GLITCH_FILT_EN ? !!getFieldValue(value, fieldCfg.GLITCH_FILT_EN) : regOffset2 > 0;
          this.updateGlitchFilter(rmtCfg2, regOffset2), super.writeUint32(addr, value);
          return;
        }
        break;
      case rmtCfg.AT_CMD_CHAR:
        this.atChar = value >> SHIFT_AT_CHAR & MASK_AT_CHAR, this.atNum = value >> SHIFT_AT_NUM & MASK_AT_NUM, this.atCounter = 0, this.clearRegisterBits(REG_INT_RAW, BIT_AT_CMD_DET), this.checkInterrupt();
        return;
      case rmtCfg.AUTOBAUD:
        if (this.updateAutoBaud(!!(value & BIT_AUTOBAUD_EN)), fieldCfg.GLITCH_FILT) {
          let addr2 = getFieldValue(value, fieldCfg.GLITCH_FILT);
          this.updateGlitchFilter(!!(value & BIT_AUTOBAUD_EN), addr2);
        }
    }
    super.writeUint32(addr, value);
  }
  reset() {
    super.reset(), this.writeRegister(
      REG_CONF0,
      BIT_TICK_REF_ON | 3 << SHIFT_BIT_NUM | 1 << BIT_CONF_FLAG_4
    ), this.writeRegister(REG_CLKDIV, 694);
    let { F: addr } = this.config, value = this.readRegister(REG_CONF1);
    this.rxFullThreshold = getFieldValue(value, addr.RXFIFO_FULL_THRHD), this.txEmptyThreshold = getFieldValue(
      value,
      addr.TXFIFO_EMPTY_THRHD
    ), this.rxFIFO.splice(0, this.rxFIFO.length), this.txFIFO.splice(0, this.txFIFO.length), this.atCounter = 0;
  }
};

// src/peripherals/common/enums.js
function defineEnum2(map) {
  for (const [key, value] of Object.entries(map)) map[value] = key;
  return map;
}
var TimerMode = defineEnum2({
  Increment: 0,
  Decrement: 1,
  ZigZag: 2
});
var PeripheralType = defineEnum2({
  GPIO: 0,
  SPI: 1,
  I2C: 2,
  UART: 3,
  LEDC: 4,
  PCNT: 5,
  TWAI: 6,
  Other: 7,
  None: 8
});
var PinState = defineEnum2({
  Low: 0,
  High: 1,
  Input: 2,
  PullUp: 3,
  PullDown: 4
});
var SignalDirection = defineEnum2({
  Input: 1,
  Output: 2,
  Both: 3
});
var InterruptTrigger = defineEnum2({
  Disable: 0,
  RisingEdge: 1,
  FallingEdge: 2,
  Edge: 3,
  LowLevel: 4,
  HighLevel: 5
});
var I2cCommand = defineEnum2({
  RSTART: 0,
  WRITE: 1,
  READ: 2,
  STOP: 3,
  END: 4,
  RSTART_NEW: 6,
  STOP_NEW: 2,
  READ_NEW: 3
});
var I2cInterruptType = defineEnum2({
  RXFIFO_FULL: 0,
  TXFIFO_EMPTY: 1,
  RXFIFO_OVF: 2,
  END_DETECT: 3,
  SLAVE_TRAN_COMP: 4,
  ARBITRATION_LOST: 5,
  MASTER_TRAN_COMP: 6,
  TRANS_COMPLETE: 7,
  TIME_OUT: 8,
  TRANS_START: 9,
  ACK_ERR: 10,
  RX_REC_FULL: 11,
  TX_SEND_EMPTY: 12
});
var PcntRegister = defineEnum2({
  CONF0: 0,
  CONF1: 1,
  CONF2: 2,
  CNT: 3,
  STATUS: 4
});
var RmtClockSource = defineEnum2({
  APB: 1,
  RC_FAST: 2,
  XTAL: 3
});
var RmtChannelRegister = defineEnum2({
  CONF0: 0,
  CONF1: 1,
  TX_LIM: 2
});
var ShaAlgorithm = defineEnum2({
  SHA1: 0,
  SHA224: 1,
  SHA256: 2,
  SHA384: 3,
  SHA512: 4,
  SHA512_224: 5,
  SHA512_256: 6
});
var ShaPeripheralMode = defineEnum2({
  SHA1: 0,
  SHA256: 1,
  SHA384: 2,
  SHA512: 3
});
var ShaOperation = defineEnum2({
  Start: 0,
  Continue: 1,
  Final: 2
});
var KeyManagerCryptoAlgo = defineEnum2({
  AES: 1,
  ECDH0: 2,
  ECDH1: 3
});
var KeyPurpose = defineEnum2({
  INVALID: 0,
  ECDSA_192: 1,
  ECDSA_256: 2,
  FLASH_256_1: 3,
  FLASH_256_2: 4,
  FLASH_128: 5,
  HMAC: 6,
  DS: 7,
  PSRAM_256_1: 8,
  PSRAM_256_2: 9,
  PSRAM_128: 10,
  ECDSA_384_L: 11,
  ECDSA_384_H: 12
});
var KeyManagerState = defineEnum2({
  IDLE: 0,
  LOAD: 1,
  GAIN: 2,
  BUSY: 3
});
var XtsState = defineEnum2({
  IDLE: 0,
  BUSY: 1,
  DONE: 2,
  VISIBLE: 3
});
var OutputSignalIndex = defineEnum2({
  LEDC_HS_SIG_OUT0: 71,
  LEDC_LS_SIG_OUT0: 79,
  RMT_SIG_OUT0: 87
});

// src/peripherals/common/xts-state.js
var XtsEncryptionState = class {
  constructor() {
    this.state = XtsState.IDLE;
    this.lineSize = 0;
    this.destination = 0;
    this.physicalAddress = 0;
  }
  reset() {
    this.state = XtsState.IDLE;
    this.lineSize = 0;
    this.destination = 0;
    this.physicalAddress = 0;
  }
};

// src/peripherals/common/helpers.js
function randomUint32() {
  return Math.floor(4294967296 * Math.random());
}
function createFieldDescriptor(reg, shift, width) {
  return { reg, shift, mask: (1 << width) - 1 };
}

// src/peripherals/common/gpio.js
var GpioSignalDefs = {
  GPIO: "GPIO",
  U0TXD: {
    name: "U0TXD",
    peripheral: PeripheralType.UART,
    index: 0,
    signal: "rsaReg26"
  },
  U0RXD: {
    name: "U0RXD",
    peripheral: PeripheralType.UART,
    index: 0,
    signal: "rsaReg57"
  },
  U0RTS: {
    name: "U0RTS",
    peripheral: PeripheralType.UART,
    index: 0,
    signal: "RTS"
  },
  U0CTS: {
    name: "U0CTS",
    peripheral: PeripheralType.UART,
    index: 0,
    signal: "CTS"
  },
  U0DSR: {
    name: "U0DSR",
    peripheral: PeripheralType.UART,
    index: 0,
    signal: "DSR"
  },
  U0DTR: {
    name: "U0DTR",
    peripheral: PeripheralType.UART,
    index: 0,
    signal: "DTR"
  },
  U1TXD: {
    name: "U1TXD",
    peripheral: PeripheralType.UART,
    index: 1,
    signal: "rsaReg26"
  },
  U1RXD: {
    name: "U1RXD",
    peripheral: PeripheralType.UART,
    index: 1,
    signal: "rsaReg57"
  },
  U1RTS: {
    name: "U1RTS",
    peripheral: PeripheralType.UART,
    index: 1,
    signal: "RTS"
  },
  U1CTS: {
    name: "U1CTS",
    peripheral: PeripheralType.UART,
    index: 1,
    signal: "CTS"
  },
  U1DSR: {
    name: "U1DSR",
    peripheral: PeripheralType.UART,
    index: 1,
    signal: "DSR"
  },
  U1DTR: {
    name: "U1DTR",
    peripheral: PeripheralType.UART,
    index: 1,
    signal: "DTR"
  },
  U2TXD: {
    name: "U2TXD",
    peripheral: PeripheralType.UART,
    index: 2,
    signal: "rsaReg26"
  },
  U2RXD: {
    name: "U2RXD",
    peripheral: PeripheralType.UART,
    index: 2,
    signal: "rsaReg57"
  },
  U2RTS: {
    name: "U2RTS",
    peripheral: PeripheralType.UART,
    index: 2,
    signal: "RTS"
  },
  U2CTS: {
    name: "U2CTS",
    peripheral: PeripheralType.UART,
    index: 2,
    signal: "CTS"
  },
  U2DSR: {
    name: "U2DSR",
    peripheral: PeripheralType.UART,
    index: 2,
    signal: "DSR"
  },
  U2DTR: {
    name: "U2DTR",
    peripheral: PeripheralType.UART,
    index: 2,
    signal: "DTR"
  },
  U3TXD: {
    name: "U3TXD",
    peripheral: PeripheralType.UART,
    index: 3,
    signal: "rsaReg26"
  },
  U3RXD: {
    name: "U3RXD",
    peripheral: PeripheralType.UART,
    index: 3,
    signal: "rsaReg57"
  },
  U3RTS: {
    name: "U3RTS",
    peripheral: PeripheralType.UART,
    index: 3,
    signal: "RTS"
  },
  U3CTS: {
    name: "U3CTS",
    peripheral: PeripheralType.UART,
    index: 3,
    signal: "CTS"
  },
  U3DSR: {
    name: "U3DSR",
    peripheral: PeripheralType.UART,
    index: 3,
    signal: "DSR"
  },
  U3DTR: {
    name: "U3DTR",
    peripheral: PeripheralType.UART,
    index: 3,
    signal: "DTR"
  },
  U4TXD: {
    name: "U4TXD",
    peripheral: PeripheralType.UART,
    index: 4,
    signal: "rsaReg26"
  },
  U4RXD: {
    name: "U4RXD",
    peripheral: PeripheralType.UART,
    index: 4,
    signal: "rsaReg57"
  },
  U4RTS: {
    name: "U4RTS",
    peripheral: PeripheralType.UART,
    index: 4,
    signal: "RTS"
  },
  U4CTS: {
    name: "U4CTS",
    peripheral: PeripheralType.UART,
    index: 4,
    signal: "CTS"
  },
  U4DSR: {
    name: "U4DSR",
    peripheral: PeripheralType.UART,
    index: 4,
    signal: "DSR"
  },
  U4DTR: {
    name: "U4DTR",
    peripheral: PeripheralType.UART,
    index: 4,
    signal: "DTR"
  },
  I2CEXT0_SCL: {
    name: "I2CEXT0_SCL",
    peripheral: PeripheralType.I2C,
    index: 0,
    signal: "SCL"
  },
  I2CEXT0_SDA: {
    name: "I2CEXT0_SDA",
    peripheral: PeripheralType.I2C,
    index: 0,
    signal: "SDA"
  },
  I2CEXT1_SCL: {
    name: "I2CEXT1_SCL",
    peripheral: PeripheralType.I2C,
    index: 1,
    signal: "SCL"
  },
  I2CEXT1_SDA: {
    name: "I2CEXT1_SDA",
    peripheral: PeripheralType.I2C,
    index: 1,
    signal: "SDA"
  },
  SPICS0: {
    name: "SPICS0",
    peripheral: PeripheralType.SPI,
    index: 1,
    signal: "CS0"
  },
  SPICS1: {
    name: "SPICS1",
    peripheral: PeripheralType.SPI,
    index: 1,
    signal: "CS1"
  },
  SPICS2: {
    name: "SPICS2",
    peripheral: PeripheralType.SPI,
    index: 1,
    signal: "CS2"
  },
  SPID: {
    name: "SPID",
    peripheral: PeripheralType.SPI,
    index: 1,
    signal: "MOSI"
  },
  SPICLK: {
    name: "SPICLK",
    peripheral: PeripheralType.SPI,
    index: 1,
    signal: "CLK"
  },
  SPIQ: {
    name: "SPIQ",
    peripheral: PeripheralType.SPI,
    index: 1,
    signal: "MISO"
  },
  SPIHD: {
    name: "SPIHD",
    peripheral: PeripheralType.SPI,
    index: 1,
    signal: "HD"
  },
  SPIWP: {
    name: "SPIWP",
    peripheral: PeripheralType.SPI,
    index: 1,
    signal: "WP"
  },
  FSPICS0: {
    name: "FSPICS0",
    peripheral: PeripheralType.SPI,
    index: 2,
    signal: "CS0"
  },
  FSPICS1: {
    name: "FSPICS1",
    peripheral: PeripheralType.SPI,
    index: 2,
    signal: "CS1"
  },
  FSPICS2: {
    name: "FSPICS2",
    peripheral: PeripheralType.SPI,
    index: 2,
    signal: "CS2"
  },
  FSPICS3: {
    name: "FSPICS3",
    peripheral: PeripheralType.SPI,
    index: 2,
    signal: "CS3"
  },
  FSPICS4: {
    name: "FSPICS4",
    peripheral: PeripheralType.SPI,
    index: 2,
    signal: "CS4"
  },
  FSPICS5: {
    name: "FSPICS5",
    peripheral: PeripheralType.SPI,
    index: 2,
    signal: "CS5"
  },
  FSPID: {
    name: "FSPID",
    peripheral: PeripheralType.SPI,
    index: 2,
    signal: "MOSI"
  },
  FSPICLK: {
    name: "FSPICLK",
    peripheral: PeripheralType.SPI,
    index: 2,
    signal: "CLK"
  },
  FSPIQ: {
    name: "FSPIQ",
    peripheral: PeripheralType.SPI,
    index: 2,
    signal: "MISO"
  },
  FSPIHD: {
    name: "FSPIHD",
    peripheral: PeripheralType.SPI,
    index: 2,
    signal: "HD"
  },
  FSPIWP: {
    name: "FSPIWP",
    peripheral: PeripheralType.SPI,
    index: 2,
    signal: "WP"
  },
  FSPID4: {
    name: "FSPID4",
    peripheral: PeripheralType.SPI,
    index: 2,
    signal: "D4"
  },
  FSPID5: {
    name: "FSPID5",
    peripheral: PeripheralType.SPI,
    index: 2,
    signal: "D5"
  },
  FSPID6: {
    name: "FSPID6",
    peripheral: PeripheralType.SPI,
    index: 2,
    signal: "D6"
  },
  FSPID7: {
    name: "FSPID7",
    peripheral: PeripheralType.SPI,
    index: 2,
    signal: "D7"
  },
  FSPIDQS: {
    name: "FSPIDQS",
    peripheral: PeripheralType.SPI,
    index: 2,
    signal: "DQS"
  },
  SPI3CS0: {
    name: "SPI3CS0",
    peripheral: PeripheralType.SPI,
    index: 3,
    signal: "CS0"
  },
  SPI3CS1: {
    name: "SPI3CS1",
    peripheral: PeripheralType.SPI,
    index: 3,
    signal: "CS1"
  },
  SPI3CS2: {
    name: "SPI3CS2",
    peripheral: PeripheralType.SPI,
    index: 3,
    signal: "CS2"
  },
  SPI3CS3: {
    name: "SPI3CS3",
    peripheral: PeripheralType.SPI,
    index: 3,
    signal: "CS3"
  },
  SPI3CS4: {
    name: "SPI3CS4",
    peripheral: PeripheralType.SPI,
    index: 3,
    signal: "CS4"
  },
  SPI3CS5: {
    name: "SPI3CS5",
    peripheral: PeripheralType.SPI,
    index: 3,
    signal: "CS5"
  },
  SPI3D: {
    name: "SPI3D",
    peripheral: PeripheralType.SPI,
    index: 3,
    signal: "MOSI"
  },
  SPI3CLK: {
    name: "SPI3CLK",
    peripheral: PeripheralType.SPI,
    index: 3,
    signal: "CLK"
  },
  SPI3Q: {
    name: "SPI3Q",
    peripheral: PeripheralType.SPI,
    index: 3,
    signal: "MISO"
  },
  SPI3HD: {
    name: "SPI3HD",
    peripheral: PeripheralType.SPI,
    index: 3,
    signal: "HD"
  },
  SPI3WP: {
    name: "SPI3WP",
    peripheral: PeripheralType.SPI,
    index: 3,
    signal: "WP"
  },
  HSPICS0: {
    name: "HSPICS0",
    peripheral: PeripheralType.SPI,
    index: 2,
    signal: "CS0"
  },
  HSPICS1: {
    name: "HSPICS1",
    peripheral: PeripheralType.SPI,
    index: 2,
    signal: "CS1"
  },
  HSPICS2: {
    name: "HSPICS2",
    peripheral: PeripheralType.SPI,
    index: 2,
    signal: "CS2"
  },
  HSPID: {
    name: "HSPID",
    peripheral: PeripheralType.SPI,
    index: 2,
    signal: "MOSI"
  },
  HSPICLK: {
    name: "HSPICLK",
    peripheral: PeripheralType.SPI,
    index: 2,
    signal: "CLK"
  },
  HSPIQ: {
    name: "HSPIQ",
    peripheral: PeripheralType.SPI,
    index: 2,
    signal: "MISO"
  },
  HSPIHD: {
    name: "HSPIHD",
    peripheral: PeripheralType.SPI,
    index: 2,
    signal: "HD"
  },
  HSPIWP: {
    name: "HSPIWP",
    peripheral: PeripheralType.SPI,
    index: 2,
    signal: "WP"
  },
  VSPICS0: {
    name: "VSPICS0",
    peripheral: PeripheralType.SPI,
    index: 3,
    signal: "CS0"
  },
  VSPICS1: {
    name: "VSPICS1",
    peripheral: PeripheralType.SPI,
    index: 3,
    signal: "CS1"
  },
  VSPICS2: {
    name: "VSPICS2",
    peripheral: PeripheralType.SPI,
    index: 3,
    signal: "CS2"
  },
  VSPID: {
    name: "VSPID",
    peripheral: PeripheralType.SPI,
    index: 3,
    signal: "MOSI"
  },
  VSPICLK: {
    name: "VSPICLK",
    peripheral: PeripheralType.SPI,
    index: 3,
    signal: "CLK"
  },
  VSPIQ: {
    name: "VSPIQ",
    peripheral: PeripheralType.SPI,
    index: 3,
    signal: "MISO"
  },
  VSPIHD: {
    name: "VSPIHD",
    peripheral: PeripheralType.SPI,
    index: 3,
    signal: "HD"
  },
  VSPIWP: {
    name: "VSPIWP",
    peripheral: PeripheralType.SPI,
    index: 3,
    signal: "WP"
  },
  LP_UART_RXD: {
    name: "LP_UART_RXD",
    peripheral: PeripheralType.UART,
    index: 2,
    signal: "rsaReg57"
  },
  LP_UART_TXD: {
    name: "LP_UART_TXD",
    peripheral: PeripheralType.UART,
    index: 2,
    signal: "rsaReg26"
  },
  LP_UART_RTS: {
    name: "LP_UART_RTS",
    peripheral: PeripheralType.UART,
    index: 2,
    signal: "RTS"
  },
  LP_UART_CTS: {
    name: "LP_UART_CTS",
    peripheral: PeripheralType.UART,
    index: 2,
    signal: "CTS"
  },
  LP_UART_DSR: {
    name: "LP_UART_DSR",
    peripheral: PeripheralType.UART,
    index: 2,
    signal: "DSR"
  },
  LP_UART_DTR: {
    name: "LP_UART_DTR",
    peripheral: PeripheralType.UART,
    index: 2,
    signal: "DTR"
  },
  LP_I2C_SDA: {
    name: "LP_I2C_SDA",
    peripheral: PeripheralType.I2C,
    index: 1,
    signal: "SDA"
  },
  LP_I2C_SCL: {
    name: "LP_I2C_SCL",
    peripheral: PeripheralType.I2C,
    index: 1,
    signal: "SCL"
  },
  LP_SPI_CS0: {
    name: "LP_SPI_CS0",
    peripheral: PeripheralType.SPI,
    index: 3,
    signal: "CS0"
  },
  LP_SPI_D: {
    name: "LP_SPI_D",
    peripheral: PeripheralType.SPI,
    index: 3,
    signal: "MOSI"
  },
  LP_SPI_CLK: {
    name: "LP_SPI_CLK",
    peripheral: PeripheralType.SPI,
    index: 3,
    signal: "CLK"
  },
  LP_SPI_Q: {
    name: "LP_SPI_Q",
    peripheral: PeripheralType.SPI,
    index: 3,
    signal: "MISO"
  },
  PCNT_SIG_CH0_IN0: {
    name: "PCNT_SIG_CH0_IN0",
    peripheral: PeripheralType.PCNT,
    index: 0,
    signal: "SIG_CH0"
  },
  PCNT_SIG_CH1_IN0: {
    name: "PCNT_SIG_CH1_IN0",
    peripheral: PeripheralType.PCNT,
    index: 0,
    signal: "SIG_CH1"
  },
  PCNT_CTRL_CH0_IN0: {
    name: "PCNT_CTRL_CH0_IN0",
    peripheral: PeripheralType.PCNT,
    index: 0,
    signal: "CTRL_CH0"
  },
  PCNT_CTRL_CH1_IN0: {
    name: "PCNT_CTRL_CH1_IN0",
    peripheral: PeripheralType.PCNT,
    index: 0,
    signal: "CTRL_CH1"
  },
  PCNT_SIG_CH0_IN1: {
    name: "PCNT_SIG_CH0_IN1",
    peripheral: PeripheralType.PCNT,
    index: 1,
    signal: "SIG_CH0"
  },
  PCNT_SIG_CH1_IN1: {
    name: "PCNT_SIG_CH1_IN1",
    peripheral: PeripheralType.PCNT,
    index: 1,
    signal: "SIG_CH1"
  },
  PCNT_CTRL_CH0_IN1: {
    name: "PCNT_CTRL_CH0_IN1",
    peripheral: PeripheralType.PCNT,
    index: 1,
    signal: "CTRL_CH0"
  },
  PCNT_CTRL_CH1_IN1: {
    name: "PCNT_CTRL_CH1_IN1",
    peripheral: PeripheralType.PCNT,
    index: 1,
    signal: "CTRL_CH1"
  },
  PCNT_SIG_CH0_IN2: {
    name: "PCNT_SIG_CH0_IN2",
    peripheral: PeripheralType.PCNT,
    index: 2,
    signal: "SIG_CH0"
  },
  PCNT_SIG_CH1_IN2: {
    name: "PCNT_SIG_CH1_IN2",
    peripheral: PeripheralType.PCNT,
    index: 2,
    signal: "SIG_CH1"
  },
  PCNT_CTRL_CH0_IN2: {
    name: "PCNT_CTRL_CH0_IN2",
    peripheral: PeripheralType.PCNT,
    index: 2,
    signal: "CTRL_CH0"
  },
  PCNT_CTRL_CH1_IN2: {
    name: "PCNT_CTRL_CH1_IN2",
    peripheral: PeripheralType.PCNT,
    index: 2,
    signal: "CTRL_CH1"
  },
  PCNT_SIG_CH0_IN3: {
    name: "PCNT_SIG_CH0_IN3",
    peripheral: PeripheralType.PCNT,
    index: 3,
    signal: "SIG_CH0"
  },
  PCNT_SIG_CH1_IN3: {
    name: "PCNT_SIG_CH1_IN3",
    peripheral: PeripheralType.PCNT,
    index: 3,
    signal: "SIG_CH1"
  },
  PCNT_CTRL_CH0_IN3: {
    name: "PCNT_CTRL_CH0_IN3",
    peripheral: PeripheralType.PCNT,
    index: 3,
    signal: "CTRL_CH0"
  },
  PCNT_CTRL_CH1_IN3: {
    name: "PCNT_CTRL_CH1_IN3",
    peripheral: PeripheralType.PCNT,
    index: 3,
    signal: "CTRL_CH1"
  },
  PCNT_SIG_CH0_IN4: {
    name: "PCNT_SIG_CH0_IN4",
    peripheral: PeripheralType.PCNT,
    index: 4,
    signal: "SIG_CH0"
  },
  PCNT_SIG_CH1_IN4: {
    name: "PCNT_SIG_CH1_IN4",
    peripheral: PeripheralType.PCNT,
    index: 4,
    signal: "SIG_CH1"
  },
  PCNT_CTRL_CH0_IN4: {
    name: "PCNT_CTRL_CH0_IN4",
    peripheral: PeripheralType.PCNT,
    index: 4,
    signal: "CTRL_CH0"
  },
  PCNT_CTRL_CH1_IN4: {
    name: "PCNT_CTRL_CH1_IN4",
    peripheral: PeripheralType.PCNT,
    index: 4,
    signal: "CTRL_CH1"
  },
  PCNT_SIG_CH0_IN5: {
    name: "PCNT_SIG_CH0_IN5",
    peripheral: PeripheralType.PCNT,
    index: 5,
    signal: "SIG_CH0"
  },
  PCNT_SIG_CH1_IN5: {
    name: "PCNT_SIG_CH1_IN5",
    peripheral: PeripheralType.PCNT,
    index: 5,
    signal: "SIG_CH1"
  },
  PCNT_CTRL_CH0_IN5: {
    name: "PCNT_CTRL_CH0_IN5",
    peripheral: PeripheralType.PCNT,
    index: 5,
    signal: "CTRL_CH0"
  },
  PCNT_CTRL_CH1_IN5: {
    name: "PCNT_CTRL_CH1_IN5",
    peripheral: PeripheralType.PCNT,
    index: 5,
    signal: "CTRL_CH1"
  },
  PCNT_SIG_CH0_IN6: {
    name: "PCNT_SIG_CH0_IN6",
    peripheral: PeripheralType.PCNT,
    index: 6,
    signal: "SIG_CH0"
  },
  PCNT_SIG_CH1_IN6: {
    name: "PCNT_SIG_CH1_IN6",
    peripheral: PeripheralType.PCNT,
    index: 6,
    signal: "SIG_CH1"
  },
  PCNT_CTRL_CH0_IN6: {
    name: "PCNT_CTRL_CH0_IN6",
    peripheral: PeripheralType.PCNT,
    index: 6,
    signal: "CTRL_CH0"
  },
  PCNT_CTRL_CH1_IN6: {
    name: "PCNT_CTRL_CH1_IN6",
    peripheral: PeripheralType.PCNT,
    index: 6,
    signal: "CTRL_CH1"
  },
  PCNT_SIG_CH0_IN7: {
    name: "PCNT_SIG_CH0_IN7",
    peripheral: PeripheralType.PCNT,
    index: 7,
    signal: "SIG_CH0"
  },
  PCNT_SIG_CH1_IN7: {
    name: "PCNT_SIG_CH1_IN7",
    peripheral: PeripheralType.PCNT,
    index: 7,
    signal: "SIG_CH1"
  },
  PCNT_CTRL_CH0_IN7: {
    name: "PCNT_CTRL_CH0_IN7",
    peripheral: PeripheralType.PCNT,
    index: 7,
    signal: "CTRL_CH0"
  },
  PCNT_CTRL_CH1_IN7: {
    name: "PCNT_CTRL_CH1_IN7",
    peripheral: PeripheralType.PCNT,
    index: 7,
    signal: "CTRL_CH1"
  },
  PCNT_RST_PAD_IN0: {
    name: "PCNT_RST_PAD_IN0",
    peripheral: PeripheralType.PCNT,
    index: 0,
    signal: "RST_PAD"
  },
  PCNT_RST_PAD_IN1: {
    name: "PCNT_RST_PAD_IN1",
    peripheral: PeripheralType.PCNT,
    index: 1,
    signal: "RST_PAD"
  },
  PCNT_RST_PAD_IN2: {
    name: "PCNT_RST_PAD_IN2",
    peripheral: PeripheralType.PCNT,
    index: 2,
    signal: "RST_PAD"
  },
  PCNT_RST_PAD_IN3: {
    name: "PCNT_RST_PAD_IN3",
    peripheral: PeripheralType.PCNT,
    index: 3,
    signal: "RST_PAD"
  },
  TWAI0_RX: {
    name: "TWAI0_RX",
    peripheral: PeripheralType.TWAI,
    index: 0,
    signal: "rsaReg57"
  },
  TWAI0_TX: {
    name: "TWAI0_TX",
    peripheral: PeripheralType.TWAI,
    index: 0,
    signal: "rsaReg26"
  },
  TWAI0_BUS_OFF_ON: {
    name: "TWAI0_BUS_OFF_ON",
    peripheral: PeripheralType.TWAI,
    index: 0,
    signal: "BUS_OFF_ON"
  },
  TWAI0_CLKOUT: {
    name: "TWAI0_CLKOUT",
    peripheral: PeripheralType.TWAI,
    index: 0,
    signal: "CLKOUT"
  },
  TWAI0_STANDBY: {
    name: "TWAI0_STANDBY",
    peripheral: PeripheralType.TWAI,
    index: 0,
    signal: "STANDBY"
  },
  TWAI1_RX: {
    name: "TWAI1_RX",
    peripheral: PeripheralType.TWAI,
    index: 1,
    signal: "rsaReg57"
  },
  TWAI1_TX: {
    name: "TWAI1_TX",
    peripheral: PeripheralType.TWAI,
    index: 1,
    signal: "rsaReg26"
  },
  TWAI1_BUS_OFF_ON: {
    name: "TWAI1_BUS_OFF_ON",
    peripheral: PeripheralType.TWAI,
    index: 1,
    signal: "BUS_OFF_ON"
  },
  TWAI1_CLKOUT: {
    name: "TWAI1_CLKOUT",
    peripheral: PeripheralType.TWAI,
    index: 1,
    signal: "CLKOUT"
  },
  TWAI1_STANDBY: {
    name: "TWAI1_STANDBY",
    peripheral: PeripheralType.TWAI,
    index: 1,
    signal: "STANDBY"
  },
  TWAI2_RX: {
    name: "TWAI2_RX",
    peripheral: PeripheralType.TWAI,
    index: 2,
    signal: "rsaReg57"
  },
  TWAI2_TX: {
    name: "TWAI2_TX",
    peripheral: PeripheralType.TWAI,
    index: 2,
    signal: "rsaReg26"
  },
  TWAI2_BUS_OFF_ON: {
    name: "TWAI2_BUS_OFF_ON",
    peripheral: PeripheralType.TWAI,
    index: 2,
    signal: "BUS_OFF_ON"
  },
  TWAI2_CLKOUT: {
    name: "TWAI2_CLKOUT",
    peripheral: PeripheralType.TWAI,
    index: 2,
    signal: "CLKOUT"
  },
  TWAI2_STANDBY: {
    name: "TWAI2_STANDBY",
    peripheral: PeripheralType.TWAI,
    index: 2,
    signal: "STANDBY"
  }
};

// src/peripherals/common/wifi-analog.js
function applyPeripheralResetValues(chip2, resetSpec) {
  for (const peripheral of chip2.peripherals) peripheral.zeroMemory();
  const core = chip2.cores[0];
  for (let i = 0; i < resetSpec.length; i += 2) {
    const addr = resetSpec[i];
    const resetList = resetSpec[i + 1];
    for (const [off, val, count, stride] of resetList) {
      for (let k = 0; k < count; k++) {
        core.writeUint32(addr + off + k * stride, val);
      }
    }
  }
}
function applySingleResetValues(peripheral, resetSpec) {
  peripheral.zeroMemory();
  for (let i = 0; i < resetSpec.length; i += 2) {
    const addr = resetSpec[i];
    const resetList = resetSpec[i + 1];
    if (addr === peripheral.baseAddr) {
      for (const [off, val, count, stride] of resetList) {
        for (let k = 0; k < count; k++) {
          peripheral.writeUint32(addr + off + k * stride, val);
        }
      }
    }
  }
}

// src/peripherals/esp32/register-data.js
var {
  GPIO: xV,
  U0TXD: xq,
  U0RXD: xY,
  U0RTS: xz,
  U0CTS: xQ,
  U1TXD: x$,
  U1RXD: xJ,
  U1RTS: xZ,
  U1CTS: xj
} = GpioSignalDefs;
var { U2TXD: x0, U2RXD: x1, U2RTS: x2, U2CTS: x6 } = GpioSignalDefs;
var {
  SPICS0: x4,
  SPID: x3,
  SPICLK: x8,
  SPIQ: x5,
  SPIHD: x7,
  SPIWP: x9
} = GpioSignalDefs;
var {
  HSPICS0: Ce,
  HSPID: Ct,
  HSPICLK: Ci,
  HSPIQ: Cs,
  HSPIHD: Cn,
  HSPIWP: Cr
} = GpioSignalDefs;
var {
  VSPICS0: Ca,
  VSPID: C_,
  VSPICLK: Cc,
  VSPIQ: Ch,
  VSPIHD: Co,
  VSPIWP: Cl
} = GpioSignalDefs;
var SdmmcAltBaseAddr = 1072955392;
var UhciBaseAddr = 1073020928;
var UhciAltBaseAddr = 1073143808;
var UartRegisterMap = {
  AUTOBAUD: 24,
  REG_UPDATE: -1,
  ID: 124,
  AT_CMD_PRECNT: 72,
  AT_CMD_POSTCNT: 76,
  AT_CMD_GAPTOUT: 80,
  AT_CMD_CHAR: 84,
  MEM_RX_STATUS: 96,
  RXD_CNT: 48,
  LOWPULSE: 40,
  HIGHPULSE: 44,
  NEGPULSE: 108,
  POSPULSE: 104,
  CLK_CONF: -1,
  RX_FILT: -1
};
var UartFieldMap = {
  RXFIFO_RST: createFieldDescriptor(32, 17, 1),
  TXFIFO_RST: createFieldDescriptor(32, 18, 1),
  LOOPBACK: createFieldDescriptor(32, 14, 1),
  TX_FLOW_EN: createFieldDescriptor(32, 15, 1),
  IRDA_EN: createFieldDescriptor(32, 16, 1),
  IRDA_TX_EN: createFieldDescriptor(32, 10, 1),
  LOWPULSE_MIN_CNT: createFieldDescriptor(40, 0, 20),
  HIGHPULSE_MIN_CNT: createFieldDescriptor(44, 0, 20),
  NEGEDGE_MIN_CNT: createFieldDescriptor(108, 0, 20),
  POSEDGE_MIN_CNT: createFieldDescriptor(104, 0, 20),
  RX_TOUT_THRHD: createFieldDescriptor(36, 24, 7),
  GLITCH_FILT: createFieldDescriptor(24, 8, 8),
  RXFIFO_FULL_THRHD: createFieldDescriptor(36, 0, 7),
  TXFIFO_EMPTY_THRHD: createFieldDescriptor(36, 8, 7)
};
var LedcTimerFieldMap = {
  TIMERn_DUTY_RES: createFieldDescriptor(352, 0, 5),
  TIMERn_CLK_DIV: createFieldDescriptor(352, 5, 18),
  TIMERn_PAUSE: createFieldDescriptor(352, 23, 1),
  TIMERn_RST: createFieldDescriptor(352, 24, 1),
  TIMERn_TICK_SEL: createFieldDescriptor(352, 25, 1),
  TIMERn_PARA_UP: createFieldDescriptor(352, 26, 1)
};
var TimerWdtFieldMap = {
  RTC_CALI_START: createFieldDescriptor(104, 31, 1),
  RTC_CALI_START_CYCLING: createFieldDescriptor(104, 12, 1),
  RTC_CALI_CLK_SEL: createFieldDescriptor(104, 13, 2),
  RTC_CALI_RDY: createFieldDescriptor(104, 15, 1),
  RTC_CALI_MAX: createFieldDescriptor(104, 16, 15),
  RTC_CALI_VALUE: createFieldDescriptor(108, 7, 25),
  WDT_INT_RAW: createFieldDescriptor(156, 2, 1)
};
var GpioInputSelectFieldMap = {
  IN_SEL: createFieldDescriptor(304, 0, 6),
  IN_INV_SEL: createFieldDescriptor(304, 6, 1),
  SEL: createFieldDescriptor(304, 7, 1)
};
var I2cFieldMap = {
  SCL_LOW_PERIOD: createFieldDescriptor(0, 0, 14),
  SCL_HIGH_PERIOD: createFieldDescriptor(56, 0, 14),
  SCL_FILTER_EN: createFieldDescriptor(80, 3, 1),
  SCL_FILTER_THRES: createFieldDescriptor(80, 0, 3)
};
var SpiFieldMap = {
  USR: createFieldDescriptor(0, 18, 1),
  WR_BIT_ORDER: createFieldDescriptor(8, 26, 1),
  MODE: createFieldDescriptor(56, 30, 1),
  DOUTDIN: createFieldDescriptor(28, 0, 1),
  USR_DUMMY_CYCLELEN: createFieldDescriptor(32, 0, 8),
  USR_ADDR_BITLEN: createFieldDescriptor(32, 26, 6),
  USR_COMMAND_VALUE: createFieldDescriptor(36, 0, 16),
  USR_COMMAND_BITLEN: createFieldDescriptor(36, 28, 4),
  SLV_DATA_BITLEN: createFieldDescriptor(100, 0, 24),
  CLKCNT_N: createFieldDescriptor(24, 12, 6),
  CLKDIV_PRE: createFieldDescriptor(24, 18, 13)
};
var I2sFieldMap = {
  TX_RESET: createFieldDescriptor(8, 0, 1),
  RX_RESET: createFieldDescriptor(8, 1, 1),
  TX_FIFO_RESET: createFieldDescriptor(8, 2, 1),
  RX_FIFO_RESET: createFieldDescriptor(8, 3, 1),
  TX_START: createFieldDescriptor(8, 4, 1),
  RX_START: createFieldDescriptor(8, 5, 1),
  TX_SLAVE_MOD: createFieldDescriptor(8, 6, 1),
  RX_SLAVE_MOD: createFieldDescriptor(8, 7, 1),
  SIG_LOOPBACK: createFieldDescriptor(8, 18, 1),
  DSCR_EN: createFieldDescriptor(32, 12, 1),
  TX_FIFO_MOD: createFieldDescriptor(32, 13, 3),
  RX_FIFO_MOD: createFieldDescriptor(32, 16, 3),
  TX_DATA_NUM: createFieldDescriptor(32, 6, 6),
  RX_DATA_NUM: createFieldDescriptor(32, 0, 6),
  TX_CHAN_MOD: createFieldDescriptor(44, 0, 3),
  RX_CHAN_MOD: createFieldDescriptor(44, 3, 2),
  OUT_RST: createFieldDescriptor(96, 1, 1),
  IN_RST: createFieldDescriptor(96, 0, 1),
  OUT_LOOP_TEST: createFieldDescriptor(96, 4, 1),
  IN_LOOP_TEST: createFieldDescriptor(96, 5, 1),
  OUT_AUTO_WRBACK: createFieldDescriptor(96, 6, 1),
  OUT_EOF_MODE: createFieldDescriptor(96, 8, 1),
  OUTDSCR_BURST_EN: createFieldDescriptor(96, 9, 1),
  INDSCR_BURST_EN: createFieldDescriptor(96, 10, 1),
  CHECK_OWNER: createFieldDescriptor(96, 12, 1),
  CLKM_DIV_NUM: createFieldDescriptor(172, 0, 8),
  CLKM_DIV_A: createFieldDescriptor(172, 14, 6),
  CLKM_DIV_B: createFieldDescriptor(172, 8, 6),
  CLK_EN: createFieldDescriptor(172, 20, 1),
  TX_BCK_DIV_NUM: createFieldDescriptor(176, 0, 6),
  RX_BCK_DIV_NUM: createFieldDescriptor(176, 6, 6),
  TX_BITS_MOD: createFieldDescriptor(176, 12, 6),
  RX_BITS_MOD: createFieldDescriptor(176, 18, 6),
  IN_SUC_EOF_INT_RAW: createFieldDescriptor(12, 9, 1),
  OUT_EOF_INT_RAW: createFieldDescriptor(12, 12, 1),
  OUT_DONE_INT_RAW: createFieldDescriptor(12, 11, 1),
  IN_DONE_INT_RAW: createFieldDescriptor(12, 8, 1),
  TX_HUNG_INT_RAW: createFieldDescriptor(12, 7, 1),
  RX_HUNG_INT_RAW: createFieldDescriptor(12, 6, 1),
  OUTLINK_ADDR: createFieldDescriptor(48, 0, 20),
  OUTLINK_STOP: createFieldDescriptor(48, 28, 1),
  OUTLINK_START: createFieldDescriptor(48, 29, 1),
  OUTLINK_RESTART: createFieldDescriptor(48, 30, 1),
  INLINK_ADDR: createFieldDescriptor(52, 0, 20),
  INLINK_STOP: createFieldDescriptor(52, 28, 1),
  INLINK_START: createFieldDescriptor(52, 29, 1),
  INLINK_RESTART: createFieldDescriptor(52, 30, 1),
  CAMERA_EN: createFieldDescriptor(168, 0, 1),
  LCD_TX_WRX2_EN: createFieldDescriptor(168, 1, 1),
  LCD_TX_SDX2_EN: createFieldDescriptor(168, 2, 1),
  DATA_ENABLE_TEST_EN: createFieldDescriptor(168, 3, 1),
  DATA_ENABLE: createFieldDescriptor(168, 4, 1),
  LCD_EN: createFieldDescriptor(168, 5, 1),
  EXT_ADC_START_EN: createFieldDescriptor(168, 6, 1),
  INTER_VALID_EN: createFieldDescriptor(168, 7, 1)
};
var TwaiFieldMap = {
  RX_FILTER_MODE: createFieldDescriptor(0, 3, 1),
  RX_MESSAGE_COUNTER: createFieldDescriptor(116, 0, 7)
};
var EfuseResetValues = [
  [4, 3, 1, 0],
  [24, 22364299, 1, 0],
  [64, 8, 1, 0],
  [68, 8, 1, 0],
  [80, 8, 1, 0],
  [84, 8, 1, 0],
  [248, 369369088, 1, 0]
];
var spiReg1 = EfuseResetValues;
var SpiDmaResetValues = [
  [8, 197376, 1, 0],
  [32, 6176, 1, 0],
  [36, 64, 1, 0],
  [96, 256, 1, 0],
  [116, 2064, 1, 0],
  [128, 2147516415, 1, 0],
  [132, 656640, 1, 0],
  [136, 328356, 1, 0],
  [140, 145228601, 1, 0],
  [144, 2685897221, 1, 0],
  [148, 40, 1, 0],
  [160, 137, 1, 0],
  [164, 10, 1, 0],
  [172, 4, 1, 0],
  [176, 4260230, 1, 0],
  [180, 22347808, 1, 0],
  [184, 983520, 1, 0],
  [188, 7, 1, 0],
  [252, 23085569, 1, 0]
];
var spiReg2 = SpiDmaResetValues;
var LedcResetValues = [
  [4, 65280, 1, 0],
  [20, 65280, 1, 0],
  [36, 65280, 1, 0],
  [76, 32, 1, 0],
  [88, 98304, 1, 0],
  [132, 32, 1, 0],
  [144, 98304, 1, 0],
  [188, 32, 1, 0],
  [200, 98304, 1, 0],
  [268, 85, 1, 0],
  [292, 34632240, 1, 0]
];
var PcntResetValues = LedcResetValues;
var UartResetValues = [
  [8, 2139136, 1, 0],
  [12, 1610547200, 1, 0],
  [20, 17, 1, 0],
  [24, 2147496003, 1, 0],
  [28, 2147483712, 1, 0],
  [32, 1543503879, 1, 0],
  [36, 1879048192, 1, 0],
  [52, 6, 1, 0],
  [56, 32, 1, 0],
  [60, 33554432, 1, 0],
  [84, 364922928, 1, 0],
  [240, 2148139088, 1, 0],
  [244, 2148466688, 1, 0],
  [256, 512, 1, 0],
  [1020, 23085680, 1, 0]
];
var spiReg3 = UartResetValues;
var Spi2ResetValues = UartResetValues;
var Spi3ResetValues = UartResetValues;
var TimerGroup0ResetValues = [
  [0, 1610620928, 1, 0],
  [36, 1610620928, 1, 0],
  [72, 311296, 1, 0],
  [76, 65536, 1, 0],
  [80, 26e6, 1, 0],
  [84, 134217727, 1, 0],
  [88, 1048575, 1, 0],
  [92, 1048575, 1, 0],
  [100, 1356348065, 1, 0],
  [104, 77824, 1, 0],
  [112, 1610621696, 1, 0],
  [248, 23085712, 1, 0]
];
var TimerGroup1ResetValues = TimerGroup0ResetValues;
var SdmmcResetValues = [
  [20, 694, 1, 0],
  [24, 4096, 1, 0],
  [32, 134217756, 1, 0],
  [36, 24672, 1, 0],
  [40, 1048575, 1, 0],
  [44, 1048575, 1, 0],
  [56, 240, 1, 0],
  [60, 319938560, 1, 0],
  [64, 10748160, 1, 0],
  [72, 16e5, 1, 0],
  [76, 16e5, 1, 0],
  [80, 7680, 1, 0],
  [84, 811, 1, 0],
  [88, 136, 1, 0],
  [104, 1048575, 1, 0],
  [108, 1048575, 1, 0],
  [120, 353510656, 1, 0],
  [124, 1280, 1, 0]
];
var Uhci0ResetValues = SdmmcResetValues;
var spiReg4 = SdmmcResetValues;
var UhciResetValues = [
  [0, 3604736, 1, 0],
  [20, 2, 1, 0],
  [28, 2, 1, 0],
  [40, 1048576, 1, 0],
  [44, 51, 1, 0],
  [100, 51, 1, 0],
  [104, 8456208, 1, 0],
  [176, 14474176, 1, 0],
  [180, 14539739, 1, 0],
  [184, 14605073, 1, 0],
  [188, 14670611, 1, 0],
  [192, 128, 1, 0],
  [252, 369364993, 1, 0]
];
var Uhci3ResetValues = UhciResetValues;
var Esp32FullResetValues = [
  1073111040,
  [
    [0, 8192, 1, 0],
    [4, 39, 1, 0],
    [8, 79, 1, 0],
    [12, 11, 1, 0],
    [16, 8356416, 1, 0],
    [20, 510, 1, 0],
    [24, 34144008, 1, 0],
    [28, 252645135, 1, 0],
    [32, 252645135, 1, 0],
    [36, 252645135, 1, 0],
    [40, 252645135, 1, 0],
    [44, 252645135, 1, 0],
    [48, 252645135, 1, 0],
    [52, 252645135, 1, 0],
    [56, 252645135, 1, 0],
    [60, 99, 1, 0],
    [124, 369369088, 1, 0]
  ],
  1072693248,
  [
    [44, 1, 1, 0],
    [64, 16, 1, 0],
    [68, 2303, 1, 0],
    [88, 16, 1, 0],
    [92, 2303, 1, 0],
    [140, 1, 1, 0],
    [148, 3, 1, 0],
    [160, 4294967295, 1, 0],
    [164, 1, 1, 0],
    [172, 257, 1, 0],
    [180, 4294967295, 1, 0],
    [184, 511, 1, 0],
    [192, 4190232687, 1, 0],
    [204, 4294762544, 1, 0],
    [212, 255, 1, 0],
    [216, 33558529, 1, 0],
    [260, 16, 1, 0],
    [264, 16, 1, 0],
    [268, 16, 1, 0],
    [272, 16, 1, 0],
    [276, 16, 1, 0],
    [280, 16, 1, 0],
    [284, 16, 1, 0],
    [288, 16, 1, 0],
    [292, 16, 1, 0],
    [296, 16, 1, 0],
    [300, 16, 1, 0],
    [304, 16, 1, 0],
    [308, 16, 1, 0],
    [312, 16, 1, 0],
    [316, 16, 1, 0],
    [320, 16, 1, 0],
    [324, 16, 1, 0],
    [328, 16, 1, 0],
    [332, 16, 1, 0],
    [336, 16, 1, 0],
    [340, 16, 1, 0],
    [344, 16, 1, 0],
    [348, 16, 1, 0],
    [352, 16, 1, 0],
    [356, 16, 1, 0],
    [360, 16, 1, 0],
    [364, 16, 1, 0],
    [368, 16, 1, 0],
    [372, 16, 1, 0],
    [376, 16, 1, 0],
    [380, 16, 1, 0],
    [384, 16, 1, 0],
    [388, 16, 1, 0],
    [392, 16, 1, 0],
    [396, 16, 1, 0],
    [400, 16, 1, 0],
    [404, 16, 1, 0],
    [408, 16, 1, 0],
    [412, 16, 1, 0],
    [416, 16, 1, 0],
    [420, 16, 1, 0],
    [424, 16, 1, 0],
    [428, 16, 1, 0],
    [432, 16, 1, 0],
    [436, 16, 1, 0],
    [440, 16, 1, 0],
    [444, 16, 1, 0],
    [448, 16, 1, 0],
    [452, 16, 1, 0],
    [456, 16, 1, 0],
    [460, 16, 1, 0],
    [464, 16, 1, 0],
    [468, 16, 1, 0],
    [472, 16, 1, 0],
    [476, 16, 1, 0],
    [480, 16, 1, 0],
    [484, 16, 1, 0],
    [488, 16, 1, 0],
    [492, 16, 1, 0],
    [496, 16, 1, 0],
    [500, 16, 1, 0],
    [504, 16, 1, 0],
    [508, 16, 1, 0],
    [512, 16, 1, 0],
    [516, 16, 1, 0],
    [520, 16, 1, 0],
    [524, 16, 1, 0],
    [528, 16, 1, 0],
    [532, 16, 1, 0],
    [536, 16, 1, 0],
    [540, 16, 1, 0],
    [544, 16, 1, 0],
    [548, 16, 1, 0],
    [552, 16, 1, 0],
    [556, 16, 1, 0],
    [560, 16, 1, 0],
    [564, 16, 1, 0],
    [568, 16, 1, 0],
    [572, 16, 1, 0],
    [576, 16, 1, 0],
    [580, 16, 1, 0],
    [584, 16, 1, 0],
    [588, 16, 1, 0],
    [592, 16, 1, 0],
    [596, 16, 1, 0],
    [600, 16, 1, 0],
    [604, 16, 1, 0],
    [608, 16, 1, 0],
    [612, 16, 1, 0],
    [616, 16, 1, 0],
    [620, 16, 1, 0],
    [624, 16, 1, 0],
    [628, 16, 1, 0],
    [632, 16, 1, 0],
    [636, 16, 1, 0],
    [640, 16, 1, 0],
    [644, 16, 1, 0],
    [648, 16, 1, 0],
    [652, 16, 1, 0],
    [656, 16, 1, 0],
    [660, 16, 1, 0],
    [664, 16, 1, 0],
    [668, 16, 1, 0],
    [672, 16, 1, 0],
    [676, 16, 1, 0],
    [680, 16, 1, 0],
    [684, 16, 1, 0],
    [688, 16, 1, 0],
    [692, 16, 1, 0],
    [696, 16, 1, 0],
    [700, 16, 1, 0],
    [704, 16, 1, 0],
    [708, 16, 1, 0],
    [712, 16, 1, 0],
    [716, 16, 1, 0],
    [720, 16, 1, 0],
    [724, 16, 1, 0],
    [728, 16, 1, 0],
    [732, 16, 1, 0],
    [736, 16, 1, 0],
    [740, 16, 1, 0],
    [744, 16, 1, 0],
    [748, 16, 1, 0],
    [752, 16, 1, 0],
    [756, 16, 1, 0],
    [760, 16, 1, 0],
    [764, 16, 1, 0],
    [768, 16, 1, 0],
    [772, 16, 1, 0],
    [776, 16, 1, 0],
    [780, 16, 1, 0],
    [784, 16, 1, 0],
    [788, 16, 1, 0],
    [792, 16, 1, 0],
    [796, 16, 1, 0],
    [800, 16, 1, 0],
    [804, 16, 1, 0],
    [808, 16, 1, 0],
    [1088, 256, 1, 0],
    [1128, 256, 1, 0],
    [1172, 1, 1, 0],
    [1176, 1, 1, 0],
    [1180, 1, 1, 0],
    [1184, 1, 1, 0],
    [1188, 1, 1, 0],
    [1192, 1, 1, 0],
    [1196, 1, 1, 0],
    [1200, 1, 1, 0],
    [1204, 1, 1, 0],
    [1208, 1, 1, 0],
    [1212, 1, 1, 0],
    [1216, 1, 1, 0],
    [1220, 1, 1, 0],
    [1224, 1, 1, 0],
    [1228, 1, 1, 0],
    [1232, 1, 1, 0],
    [1236, 1, 1, 0],
    [1240, 1, 1, 0],
    [1244, 1, 1, 0],
    [1248, 1, 1, 0],
    [1252, 1, 1, 0],
    [1256, 1, 1, 0],
    [1260, 1, 1, 0],
    [1264, 1, 1, 0],
    [1268, 1, 1, 0],
    [1272, 1, 1, 0],
    [1276, 1, 1, 0],
    [1280, 1, 1, 0],
    [1288, 1, 1, 0],
    [1292, 2, 1, 0],
    [1296, 3, 1, 0],
    [1300, 4, 1, 0],
    [1304, 5, 1, 0],
    [1308, 6, 1, 0],
    [1312, 7, 1, 0],
    [1316, 8, 1, 0],
    [1320, 9, 1, 0],
    [1324, 10, 1, 0],
    [1328, 11, 1, 0],
    [1332, 12, 1, 0],
    [1336, 13, 1, 0],
    [1340, 14, 1, 0],
    [1344, 15, 1, 0],
    [1352, 1, 1, 0],
    [1356, 2, 1, 0],
    [1360, 3, 1, 0],
    [1364, 4, 1, 0],
    [1368, 5, 1, 0],
    [1372, 6, 1, 0],
    [1376, 7, 1, 0],
    [1380, 8, 1, 0],
    [1384, 9, 1, 0],
    [1388, 10, 1, 0],
    [1392, 11, 1, 0],
    [1396, 12, 1, 0],
    [1400, 13, 1, 0],
    [1404, 14, 1, 0],
    [1408, 15, 1, 0],
    [1412, 1, 1, 0],
    [1420, 1, 1, 0],
    [1428, 5, 1, 0],
    [4092, 23089552, 1, 0]
  ],
  1073061888,
  [
    [248, 16466, 1, 0],
    [252, 65536, 1, 0],
    [280, 40, 1, 0],
    [508, 369370624, 1, 0]
  ],
  1072975616,
  [
    [0, 65280, 1, 0],
    [4, 65280, 1, 0],
    [8, 65280, 1, 0],
    [12, 65280, 1, 0],
    [16, 65280, 1, 0],
    [20, 65280, 1, 0],
    [24, 65280, 1, 0],
    [28, 65280, 1, 0],
    [40, 22045072, 1, 0]
  ],
  1073000448,
  [
    [0, 572679782, 1, 0],
    [4, 17891345, 1, 0],
    [28, 131072, 1, 0],
    [32, 4294967295, 1, 0],
    [36, 4294967295, 1, 0],
    [40, 4294967295, 1, 0],
    [44, 4294967295, 1, 0],
    [48, 4294967295, 1, 0],
    [52, 4294967295, 1, 0],
    [56, 4294967295, 1, 0],
    [60, 4294967295, 1, 0],
    [64, 859006566, 1, 0],
    [252, 352518656, 1, 0]
  ],
  1073033216,
  EfuseResetValues,
  1073115136,
  spiReg1,
  1073016832,
  SpiDmaResetValues,
  1073139712,
  spiReg2,
  1073057792,
  [
    [12, 1073741824, 8, 20],
    [172, 1073741824, 8, 20],
    [320, 16777216, 4, 8],
    [352, 16777216, 4, 8],
    [508, 369301248, 1, 0]
  ],
  1073078272,
  LedcResetValues,
  1073135616,
  PcntResetValues,
  1073049600,
  [
    [0, 15376, 8, 12],
    [176, 21845, 1, 0],
    [252, 336733696, 1, 0]
  ],
  1073045504,
  [
    [32, 823132162, 8, 8],
    [36, 3872, 8, 8],
    [176, 4194368, 1, 0],
    [180, 4194368, 1, 0],
    [184, 4194368, 1, 0],
    [188, 4194368, 1, 0],
    [192, 4194368, 1, 0],
    [196, 4194368, 1, 0],
    [200, 4194368, 1, 0],
    [204, 4194368, 1, 0],
    [208, 128, 8, 4],
    [252, 369239552, 1, 0]
  ],
  1072988160,
  [
    [0, 474554368, 1, 0],
    [24, 3145728, 1, 0],
    [28, 672400387, 1, 0],
    [32, 17301504, 1, 0],
    [36, 336988680, 1, 0],
    [40, 270535176, 1, 0],
    [44, 303333377, 1, 0],
    [48, 8388608, 1, 0],
    [52, 12288, 1, 0],
    [56, 24576, 1, 0],
    [112, 8720, 1, 0],
    [116, 44040192, 1, 0],
    [124, 687875072, 1, 0],
    [128, 76069, 1, 0],
    [132, 1398096, 1, 0],
    [136, 2863288320, 1, 0],
    [140, 19584, 1, 0],
    [144, 128e3, 1, 0],
    [148, 8e4, 1, 0],
    [152, 4095, 1, 0],
    [156, 4095, 1, 0],
    [164, 1356348065, 1, 0],
    [212, 335478784, 1, 0],
    [316, 23085696, 1, 0]
  ],
  1072989184,
  [
    [132, 2147483648, 1, 0],
    [136, 2147483648, 1, 0],
    [140, 2215641104, 1, 0],
    [144, 1711276032, 1, 0],
    [148, 1375731712, 1, 0],
    [152, 1241513984, 1, 0],
    [156, 1375731712, 1, 0],
    [160, 1241513984, 1, 0],
    [164, 1375731712, 1, 0],
    [168, 1375731712, 1, 0],
    [172, 1241513984, 1, 0],
    [176, 1107296256, 1, 0],
    [180, 33554432, 1, 0],
    [184, 33554432, 1, 0],
    [200, 23081312, 1, 0]
  ],
  1073119232,
  [
    [20, 4294967104, 1, 0],
    [28, 512, 1, 0],
    [32, 512, 1, 0],
    [44, 536870912, 1, 0],
    [72, 1814, 1, 0],
    [108, 1412572938, 1, 0],
    [112, 54807747, 1, 0],
    [120, 1, 1, 0],
    [2048, 8520192, 1, 0]
  ],
  1072990208,
  [
    [0, 461058, 1, 0],
    [8, 655370, 1, 0],
    [12, 2097162, 1, 0],
    [16, 117912463, 1, 0],
    [24, 200, 1, 0],
    [28, 100, 1, 0],
    [32, 50, 1, 0],
    [36, 40, 1, 0],
    [40, 20, 1, 0],
    [44, 15, 1, 0],
    [48, 1049088, 1, 0],
    [52, 4294967295, 1, 0],
    [56, 4294967295, 1, 0],
    [76, 417794, 1, 0],
    [88, 33820672, 1, 0],
    [132, 4196352, 1, 0],
    [140, 1073741823, 1, 0],
    [144, 461058, 1, 0],
    [156, 50331648, 1, 0],
    [160, 3, 1, 0],
    [252, 23089536, 1, 0]
  ],
  1073053696,
  [
    [0, 4282187568, 1, 0],
    [36, 131074, 1, 0],
    [48, 131074, 1, 0],
    [68, 1048576, 1, 0],
    [96, 3145848, 1, 0],
    [116, 685856, 1, 0],
    [152, 270209050, 1, 0],
    [216, 128, 1, 0],
    [276, 1289, 1, 0],
    [280, 1023, 1, 0],
    [312, 21504, 1, 0],
    [504, 369239296, 1, 0],
    [508, 256, 1, 0]
  ],
  1073041408,
  [
    [32, 1, 1, 0],
    [120, 192, 1, 0],
    [124, 511, 1, 0],
    [268, 245828, 1, 0],
    [272, 246240, 1, 0],
    [376, 369239296, 1, 0],
    [380, 1536, 1, 0]
  ],
  1072967680,
  UartResetValues,
  1072963584,
  spiReg3,
  1073102848,
  Spi2ResetValues,
  1073106944,
  Spi3ResetValues,
  1073082368,
  TimerGroup0ResetValues,
  1073086464,
  TimerGroup1ResetValues,
  1073131520,
  [
    [0, 1, 1, 0],
    [52, 96, 1, 0]
  ],
  1072955392,
  SdmmcResetValues,
  1073020928,
  Uhci0ResetValues,
  1073143808,
  spiReg4,
  1073037312,
  UhciResetValues,
  1073004544,
  Uhci3ResetValues
];
var MmuPageTableConfig = [
  { start: 1061158912, index: 0, pages: 64, sys: true, user: false },
  { start: 1074528256, index: 76, pages: 52, sys: true, user: false },
  { start: 1077936128, index: 128, pages: 64, sys: true, user: true },
  { start: 1082130432, index: 192, pages: 64, sys: true, user: true }
];
var Drom0Size = 1048576;
var RegionDrom0Base = 1065353216;
var RegionDrom1Base = 1069547520;
var GpioBaseAddrAlt = 1072693248;
var RegionDram1Base = 1073283072;
var RegionRtcSlowBase = 1073405952;
var RegionFlashCacheBase = 1074200576;
var RegionIrom0Base = 1074397184;
var RegionCodeBase = 1073741824;
var RegionIram0Base = 1074528256;
var RegionIram1BaseAlt = 1074536448;
var Iram0Size = 458752;
var UsbOtgBaseAddr = 1342177280;
var RtcSlowSize = 8192;
var RegionPeriBusBase = 1610612736;
var RegionUsbBase = 1610874880;
var RegionDrom0MapBase = 537657344;
var Iram1Size = 204800;
var Drom0CacheSize = 131072;
var RegionIrom0BaseAlt = 1074397184;
var RegionIram1Base = 1074528256;
var RegionPeri1Base = 1072758784;
var RegionDromSize = 8192;
var RegionCacheLineSize = 2048;
var RegionPeri2Base = 1072775168;
var RegionCacheAlignSize = 16;

// src/peripherals/esp32/xtensa.js
var RegisterType = {
  PC: 0,
  AR: 16777216,
  Special: 33554432,
  User: 50331648,
  FP: 67108864,
  Mask: 4278190080
};
var LoopBegin = 0;
var LoopEnd = 1;
var LoopCount = 2;
var ExcCause = 3;
var PsRegister = 4;
var SarRegister = 12;
var LbegRegister = 16;
var WindowStart = 17;
var InterruptState = 32;
var InterruptEnable = 33;
var InterruptClear = 34;
var InterruptSet = 35;
var MemFaultInfo = 72;
var CacheControl = 73;
var DdrRegister = 176;
var Eps2Register = 208;
var IntSet = 230;
var IntLevelAlias = 231;
var IntSetAlias = 230;
var IntStatusAlias = 232;
var IntRawAlias = 233;
var CcountAlias = 234;
var CcompareAlias = 235;
var Ccompare3Reg = 236;
var XtensaRegisterTable = [
  RegisterType.PC,
  0 | RegisterType.AR,
  1 | RegisterType.AR,
  2 | RegisterType.AR,
  3 | RegisterType.AR,
  4 | RegisterType.AR,
  5 | RegisterType.AR,
  6 | RegisterType.AR,
  7 | RegisterType.AR,
  8 | RegisterType.AR,
  9 | RegisterType.AR,
  10 | RegisterType.AR,
  11 | RegisterType.AR,
  12 | RegisterType.AR,
  13 | RegisterType.AR,
  14 | RegisterType.AR,
  15 | RegisterType.AR,
  16 | RegisterType.AR,
  17 | RegisterType.AR,
  18 | RegisterType.AR,
  19 | RegisterType.AR,
  20 | RegisterType.AR,
  21 | RegisterType.AR,
  22 | RegisterType.AR,
  23 | RegisterType.AR,
  24 | RegisterType.AR,
  25 | RegisterType.AR,
  26 | RegisterType.AR,
  27 | RegisterType.AR,
  28 | RegisterType.AR,
  29 | RegisterType.AR,
  30 | RegisterType.AR,
  31 | RegisterType.AR,
  32 | RegisterType.AR,
  33 | RegisterType.AR,
  34 | RegisterType.AR,
  35 | RegisterType.AR,
  36 | RegisterType.AR,
  37 | RegisterType.AR,
  38 | RegisterType.AR,
  39 | RegisterType.AR,
  40 | RegisterType.AR,
  41 | RegisterType.AR,
  42 | RegisterType.AR,
  43 | RegisterType.AR,
  44 | RegisterType.AR,
  45 | RegisterType.AR,
  46 | RegisterType.AR,
  47 | RegisterType.AR,
  48 | RegisterType.AR,
  49 | RegisterType.AR,
  50 | RegisterType.AR,
  51 | RegisterType.AR,
  52 | RegisterType.AR,
  53 | RegisterType.AR,
  54 | RegisterType.AR,
  55 | RegisterType.AR,
  56 | RegisterType.AR,
  57 | RegisterType.AR,
  58 | RegisterType.AR,
  59 | RegisterType.AR,
  60 | RegisterType.AR,
  61 | RegisterType.AR,
  62 | RegisterType.AR,
  63 | RegisterType.AR,
  RegisterType.Special | LoopBegin,
  RegisterType.Special | LoopEnd,
  RegisterType.Special | LoopCount,
  RegisterType.Special | ExcCause,
  RegisterType.Special | MemFaultInfo,
  RegisterType.Special | CacheControl,
  RegisterType.Special | DdrRegister,
  RegisterType.Special | Eps2Register,
  RegisterType.Special | IntSet,
  RegisterType.User | IntLevelAlias,
  RegisterType.Special | PsRegister,
  RegisterType.Special | SarRegister,
  RegisterType.Special | LbegRegister,
  RegisterType.Special | WindowStart,
  RegisterType.Special | InterruptState,
  RegisterType.Special | InterruptEnable,
  RegisterType.Special | InterruptClear,
  RegisterType.Special | InterruptSet,
  RegisterType.User | IntSetAlias,
  RegisterType.User | CcountAlias,
  RegisterType.User | CcompareAlias,
  RegisterType.User | Ccompare3Reg,
  0 | RegisterType.FP,
  1 | RegisterType.FP,
  2 | RegisterType.FP,
  3 | RegisterType.FP,
  4 | RegisterType.FP,
  5 | RegisterType.FP,
  6 | RegisterType.FP,
  7 | RegisterType.FP,
  8 | RegisterType.FP,
  9 | RegisterType.FP,
  10 | RegisterType.FP,
  11 | RegisterType.FP,
  12 | RegisterType.FP,
  13 | RegisterType.FP,
  14 | RegisterType.FP,
  15 | RegisterType.FP,
  RegisterType.User | IntStatusAlias,
  RegisterType.User | IntRawAlias
];
var {
  U0RXD,
  U0RTS,
  U0CTS,
  U0TXD,
  U0DTR,
  U0DSR
} = GpioSignalDefs;
var { U1RXD, U1RTS, U1CTS, U1TXD } = GpioSignalDefs;
var { U2RXD, U2RTS, U2CTS, U2TXD } = GpioSignalDefs;
var {
  I2CEXT0_SCL,
  I2CEXT0_SDA,
  I2CEXT1_SCL,
  I2CEXT1_SDA
} = GpioSignalDefs;
var {
  SPIQ,
  SPIHD,
  SPIWP,
  SPICLK,
  SPID,
  SPICS0,
  SPICS1,
  SPICS2
} = GpioSignalDefs;
var {
  HSPIQ,
  HSPIHD,
  HSPIWP,
  HSPICLK,
  HSPID,
  HSPICS0,
  HSPICS1,
  HSPICS2
} = GpioSignalDefs;
var {
  VSPIQ,
  VSPIHD,
  VSPIWP,
  VSPICLK,
  VSPID,
  VSPICS0,
  VSPICS1,
  VSPICS2
} = GpioSignalDefs;
var {
  TWAI0_RX,
  TWAI0_TX,
  TWAI0_BUS_OFF_ON,
  TWAI0_CLKOUT,
  TWAI0_STANDBY
} = GpioSignalDefs;
var Esp32GpioMatrixEntries = [
  { i: 0, in: SPICLK, out: SPICLK },
  { i: 1, in: SPIQ, out: SPIQ },
  { i: 2, in: SPID, out: SPID },
  { i: 3, in: SPIHD, out: SPIHD },
  { i: 4, in: SPIWP, out: SPIWP },
  { i: 5, in: SPICS0, out: SPICS0 },
  { i: 6, in: SPICS1, out: SPICS1 },
  { i: 7, in: SPICS2, out: SPICS2 },
  { i: 8, in: HSPICLK, out: HSPICLK },
  { i: 9, in: HSPIQ, out: HSPIQ },
  { i: 10, in: HSPID, out: HSPID },
  { i: 11, in: HSPICS0, out: HSPICS0 },
  { i: 12, in: HSPIHD, out: HSPIHD },
  { i: 13, in: HSPIWP, out: HSPIWP },
  { i: 14, in: U0RXD, out: U0TXD },
  { i: 15, in: U0CTS, out: U0RTS },
  { i: 16, in: U0DSR, out: U0DTR },
  { i: 17, in: U1RXD, out: U1TXD },
  { i: 18, in: U1CTS, out: U1RTS },
  { i: 23, in: "I2S0O_BCK_in", out: "I2S0O_BCK_out" },
  { i: 24, in: "I2S1O_BCK_in", out: "I2S1O_BCK_out" },
  { i: 25, in: "I2S0O_WS_in", out: "I2S0O_WS_out" },
  { i: 26, in: "I2S1O_WS_in", out: "I2S1O_WS_out" },
  { i: 27, in: "I2S0I_BCK_in", out: "I2S0I_BCK_out" },
  { i: 28, in: "I2S0I_WS_in", out: "I2S0I_WS_out" },
  { i: 29, in: I2CEXT0_SCL, out: I2CEXT0_SCL },
  { i: 30, in: I2CEXT0_SDA, out: I2CEXT0_SDA },
  { i: 31, in: "pwm0_sync0_in", out: "sdio_tohost_int_out" },
  { i: 32, in: "pwm0_sync1_in", out: "pwm0_out0a" },
  { i: 33, in: "pwm0_sync2_in", out: "pwm0_out0b" },
  { i: 34, in: "pwm0_f0_in", out: "pwm0_out1a" },
  { i: 35, in: "pwm0_f1_in", out: "pwm0_out1b" },
  { i: 36, in: "pwm0_f2_in", out: "pwm0_out2a" },
  { i: 37, in: null, out: "pwm0_out2b" },
  { i: 39, in: GpioSignalDefs.PCNT_SIG_CH0_IN0, out: null },
  { i: 40, in: GpioSignalDefs.PCNT_SIG_CH1_IN0, out: null },
  { i: 41, in: GpioSignalDefs.PCNT_CTRL_CH0_IN0, out: null },
  { i: 42, in: GpioSignalDefs.PCNT_CTRL_CH1_IN0, out: null },
  { i: 43, in: GpioSignalDefs.PCNT_SIG_CH0_IN1, out: null },
  { i: 44, in: GpioSignalDefs.PCNT_SIG_CH1_IN1, out: null },
  { i: 45, in: GpioSignalDefs.PCNT_CTRL_CH0_IN1, out: null },
  { i: 46, in: GpioSignalDefs.PCNT_CTRL_CH1_IN1, out: null },
  { i: 47, in: GpioSignalDefs.PCNT_SIG_CH0_IN2, out: null },
  { i: 48, in: GpioSignalDefs.PCNT_SIG_CH1_IN2, out: null },
  { i: 49, in: GpioSignalDefs.PCNT_CTRL_CH0_IN2, out: null },
  { i: 50, in: GpioSignalDefs.PCNT_CTRL_CH1_IN2, out: null },
  { i: 51, in: GpioSignalDefs.PCNT_SIG_CH0_IN3, out: null },
  { i: 52, in: GpioSignalDefs.PCNT_SIG_CH1_IN3, out: null },
  { i: 53, in: GpioSignalDefs.PCNT_CTRL_CH0_IN3, out: null },
  { i: 54, in: GpioSignalDefs.PCNT_CTRL_CH1_IN3, out: null },
  { i: 55, in: GpioSignalDefs.PCNT_SIG_CH0_IN4, out: null },
  { i: 56, in: GpioSignalDefs.PCNT_SIG_CH1_IN4, out: null },
  { i: 57, in: GpioSignalDefs.PCNT_CTRL_CH0_IN4, out: null },
  { i: 58, in: GpioSignalDefs.PCNT_CTRL_CH1_IN4, out: null },
  { i: 61, in: HSPICS1, out: HSPICS1 },
  { i: 62, in: HSPICS2, out: HSPICS2 },
  { i: 63, in: VSPICLK, out: VSPICLK },
  { i: 64, in: VSPIQ, out: VSPIQ },
  { i: 65, in: VSPID, out: VSPID },
  { i: 66, in: VSPIHD, out: VSPIHD },
  { i: 67, in: VSPIWP, out: VSPIWP },
  { i: 68, in: VSPICS0, out: VSPICS0 },
  { i: 69, in: VSPICS1, out: VSPICS1 },
  { i: 70, in: VSPICS2, out: VSPICS2 },
  { i: 71, in: GpioSignalDefs.PCNT_SIG_CH0_IN5, out: "ledc_hs_sig_out0" },
  { i: 72, in: GpioSignalDefs.PCNT_SIG_CH1_IN5, out: "ledc_hs_sig_out1" },
  { i: 73, in: GpioSignalDefs.PCNT_CTRL_CH0_IN5, out: "ledc_hs_sig_out2" },
  { i: 74, in: GpioSignalDefs.PCNT_CTRL_CH1_IN5, out: "ledc_hs_sig_out3" },
  { i: 75, in: GpioSignalDefs.PCNT_SIG_CH0_IN6, out: "ledc_hs_sig_out4" },
  { i: 76, in: GpioSignalDefs.PCNT_SIG_CH1_IN6, out: "ledc_hs_sig_out5" },
  { i: 77, in: GpioSignalDefs.PCNT_CTRL_CH0_IN6, out: "ledc_hs_sig_out6" },
  { i: 78, in: GpioSignalDefs.PCNT_CTRL_CH1_IN6, out: "ledc_hs_sig_out7" },
  { i: 79, in: GpioSignalDefs.PCNT_SIG_CH0_IN7, out: "ledc_ls_sig_out0" },
  { i: 80, in: GpioSignalDefs.PCNT_SIG_CH1_IN7, out: "ledc_ls_sig_out1" },
  { i: 81, in: GpioSignalDefs.PCNT_CTRL_CH0_IN7, out: "ledc_ls_sig_out2" },
  { i: 82, in: GpioSignalDefs.PCNT_CTRL_CH1_IN7, out: "ledc_ls_sig_out3" },
  { i: 83, in: "rmt_sig_in0", out: "ledc_ls_sig_out4" },
  { i: 84, in: "rmt_sig_in1", out: "ledc_ls_sig_out5" },
  { i: 85, in: "rmt_sig_in2", out: "ledc_ls_sig_out6" },
  { i: 86, in: "rmt_sig_in3", out: "ledc_ls_sig_out7" },
  { i: 87, in: "rmt_sig_in4", out: "rmt_sig_out0" },
  { i: 88, in: "rmt_sig_in5", out: "rmt_sig_out1" },
  { i: 89, in: "rmt_sig_in6", out: "rmt_sig_out2" },
  { i: 90, in: "rmt_sig_in7", out: "rmt_sig_out3" },
  { i: 91, in: null, out: "rmt_sig_out4" },
  { i: 92, in: null, out: "rmt_sig_out5" },
  { i: 93, in: null, out: "rmt_sig_out6" },
  { i: 94, in: TWAI0_RX, out: "rmt_sig_out7" },
  { i: 95, in: I2CEXT1_SCL, out: I2CEXT1_SCL },
  { i: 96, in: I2CEXT1_SDA, out: I2CEXT1_SDA },
  { i: 97, in: "host_card_detect_n_1", out: "host_ccmd_od_pullup_en_n" },
  { i: 98, in: "host_card_detect_n_2", out: "host_rst_n_1" },
  { i: 99, in: "host_card_write_prt_1", out: "host_rst_n_2" },
  { i: 100, in: "host_card_write_prt_2", out: "gpio_sd0_out" },
  { i: 101, in: "host_card_int_n_1", out: "gpio_sd1_out" },
  { i: 102, in: "host_card_int_n_2", out: "gpio_sd2_out" },
  { i: 103, in: "pwm1_sync0_in", out: "gpio_sd3_out" },
  { i: 104, in: "pwm1_sync1_in", out: "gpio_sd4_out" },
  { i: 105, in: "pwm1_sync2_in", out: "gpio_sd5_out" },
  { i: 106, in: "pwm1_f0_in", out: "gpio_sd6_out" },
  { i: 107, in: "pwm1_f1_in", out: "gpio_sd7_out" },
  { i: 108, in: "pwm1_f2_in", out: "pwm1_out0a" },
  { i: 109, in: "pwm0_cap0_in", out: "pwm1_out0b" },
  { i: 110, in: "pwm0_cap1_in", out: "pwm1_out1a" },
  { i: 111, in: "pwm0_cap2_in", out: "pwm1_out1b" },
  { i: 112, in: "pwm1_cap0_in", out: "pwm1_out2a" },
  { i: 113, in: "pwm1_cap1_in", out: "pwm1_out2b" },
  { i: 114, in: "pwm1_cap2_in", out: null },
  { i: 123, in: null, out: TWAI0_TX },
  { i: 124, in: null, out: TWAI0_BUS_OFF_ON },
  { i: 125, in: null, out: TWAI0_CLKOUT },
  { i: 140, in: "I2S0I_DATA_in0", out: "I2S0O_DATA_out0" },
  { i: 141, in: "I2S0I_DATA_in1", out: "I2S0O_DATA_out1" },
  { i: 142, in: "I2S0I_DATA_in2", out: "I2S0O_DATA_out2" },
  { i: 143, in: "I2S0I_DATA_in3", out: "I2S0O_DATA_out3" },
  { i: 144, in: "I2S0I_DATA_in4", out: "I2S0O_DATA_out4" },
  { i: 145, in: "I2S0I_DATA_in5", out: "I2S0O_DATA_out5" },
  { i: 146, in: "I2S0I_DATA_in6", out: "I2S0O_DATA_out6" },
  { i: 147, in: "I2S0I_DATA_in7", out: "I2S0O_DATA_out7" },
  { i: 148, in: "I2S0I_DATA_in8", out: "I2S0O_DATA_out8" },
  { i: 149, in: "I2S0I_DATA_in9", out: "I2S0O_DATA_out9" },
  { i: 150, in: "I2S0I_DATA_in10", out: "I2S0O_DATA_out10" },
  { i: 151, in: "I2S0I_DATA_in11", out: "I2S0O_DATA_out11" },
  { i: 152, in: "I2S0I_DATA_in12", out: "I2S0O_DATA_out12" },
  { i: 153, in: "I2S0I_DATA_in13", out: "I2S0O_DATA_out13" },
  { i: 154, in: "I2S0I_DATA_in14", out: "I2S0O_DATA_out14" },
  { i: 155, in: "I2S0I_DATA_in15", out: "I2S0O_DATA_out15" },
  { i: 156, in: null, out: "I2S0O_DATA_out16" },
  { i: 157, in: null, out: "I2S0O_DATA_out17" },
  { i: 158, in: null, out: "I2S0O_DATA_out18" },
  { i: 159, in: null, out: "I2S0O_DATA_out19" },
  { i: 160, in: null, out: "I2S0O_DATA_out20" },
  { i: 161, in: null, out: "I2S0O_DATA_out21" },
  { i: 162, in: null, out: "I2S0O_DATA_out22" },
  { i: 163, in: null, out: "I2S0O_DATA_out23" },
  { i: 164, in: "I2S1I_BCK_in", out: "I2S1I_BCK_out" },
  { i: 165, in: "I2S1I_WS_in", out: "I2S1I_WS_out" },
  { i: 166, in: "I2S1I_DATA_in0", out: "I2S1O_DATA_out0" },
  { i: 167, in: "I2S1I_DATA_in1", out: "I2S1O_DATA_out1" },
  { i: 168, in: "I2S1I_DATA_in2", out: "I2S1O_DATA_out2" },
  { i: 169, in: "I2S1I_DATA_in3", out: "I2S1O_DATA_out3" },
  { i: 170, in: "I2S1I_DATA_in4", out: "I2S1O_DATA_out4" },
  { i: 171, in: "I2S1I_DATA_in5", out: "I2S1O_DATA_out5" },
  { i: 172, in: "I2S1I_DATA_in6", out: "I2S1O_DATA_out6" },
  { i: 173, in: "I2S1I_DATA_in7", out: "I2S1O_DATA_out7" },
  { i: 174, in: "I2S1I_DATA_in8", out: "I2S1O_DATA_out8" },
  { i: 175, in: "I2S1I_DATA_in9", out: "I2S1O_DATA_out9" },
  { i: 176, in: "I2S1I_DATA_in10", out: "I2S1O_DATA_out10" },
  { i: 177, in: "I2S1I_DATA_in11", out: "I2S1O_DATA_out11" },
  { i: 178, in: "I2S1I_DATA_in12", out: "I2S1O_DATA_out12" },
  { i: 179, in: "I2S1I_DATA_in13", out: "I2S1O_DATA_out13" },
  { i: 180, in: "I2S1I_DATA_in14", out: "I2S1O_DATA_out14" },
  { i: 181, in: "I2S1I_DATA_in15", out: "I2S1O_DATA_out15" },
  { i: 182, in: null, out: "I2S1O_DATA_out16" },
  { i: 183, in: null, out: "I2S1O_DATA_out17" },
  { i: 184, in: null, out: "I2S1O_DATA_out18" },
  { i: 185, in: null, out: "I2S1O_DATA_out19" },
  { i: 186, in: null, out: "I2S1O_DATA_out20" },
  { i: 187, in: null, out: "I2S1O_DATA_out21" },
  { i: 188, in: null, out: "I2S1O_DATA_out22" },
  { i: 189, in: null, out: "I2S1O_DATA_out23" },
  { i: 190, in: "I2S0I_H_SYNC", out: null },
  { i: 191, in: "I2S0I_V_SYNC", out: null },
  { i: 192, in: "I2S0I_H_ENABLE", out: null },
  { i: 193, in: "I2S1I_H_SYNC", out: null },
  { i: 194, in: "I2S1I_V_SYNC", out: null },
  { i: 195, in: "I2S1I_H_ENABLE", out: null },
  { i: 198, in: U2RXD, out: U2TXD },
  { i: 199, in: U2CTS, out: U2RTS },
  { i: 200, in: "emac_mdc_i", out: "emac_mdc_o" },
  { i: 201, in: "emac_mdi_i", out: "emac_mdo_o" },
  { i: 202, in: "emac_crs_i", out: "emac_crs_o" },
  { i: 203, in: "emac_col_i", out: "emac_col_o" },
  { i: 204, in: "pcmfsync_in", out: "bt_audio0_irq" },
  { i: 205, in: "pcmclk_in", out: "bt_audio1_irq" },
  { i: 206, in: "pcmdin", out: "bt_audio2_irq" },
  { i: 207, in: null, out: "ble_audio0_irq" },
  { i: 208, in: null, out: "ble_audio1_irq" },
  { i: 209, in: null, out: "ble_audio2_irq" },
  { i: 210, in: null, out: "pcmfsync_out" },
  { i: 211, in: null, out: "pcmclk_out" },
  { i: 212, in: null, out: "pcmdout" },
  { i: 213, in: null, out: "ble_audio_sync0_p" },
  { i: 214, in: null, out: "ble_audio_sync1_p" },
  { i: 215, in: null, out: "ble_audio_sync2_p" },
  { i: 224, in: null, out: "sig_in_func224" },
  { i: 225, in: null, out: "sig_in_func225" },
  { i: 226, in: null, out: "sig_in_func226" },
  { i: 227, in: null, out: "sig_in_func227" },
  { i: 228, in: null, out: "sig_in_func228" }
];

// tools/md5-shim.mjs
var S = [
  7,
  12,
  17,
  22,
  7,
  12,
  17,
  22,
  7,
  12,
  17,
  22,
  7,
  12,
  17,
  22,
  5,
  9,
  14,
  20,
  5,
  9,
  14,
  20,
  5,
  9,
  14,
  20,
  5,
  9,
  14,
  20,
  4,
  11,
  16,
  23,
  4,
  11,
  16,
  23,
  4,
  11,
  16,
  23,
  4,
  11,
  16,
  23,
  6,
  10,
  15,
  21,
  6,
  10,
  15,
  21,
  6,
  10,
  15,
  21,
  6,
  10,
  15,
  21
];
var K = (() => {
  const t = [];
  for (let i = 0; i < 64; i++) t.push(Math.floor(Math.abs(Math.sin(i + 1)) * 4294967296) >>> 0);
  return t;
})();
function rotl(x, c) {
  return (x << c | x >>> 32 - c) >>> 0;
}
function md5bytes(data) {
  const len = data.length;
  const bitLen = len * 8;
  const withOne = len + 1;
  const padLen = (56 - withOne % 64 + 64) % 64;
  const total = withOne + padLen + 8;
  const msg = new Uint8Array(total);
  msg.set(data, 0);
  msg[len] = 128;
  const dv = new DataView(msg.buffer);
  dv.setUint32(total - 8, bitLen >>> 0, true);
  dv.setUint32(total - 4, Math.floor(bitLen / 4294967296) >>> 0, true);
  let a0 = 1732584193, b0 = 4023233417, c0 = 2562383102, d0 = 271733878;
  const M = new Uint32Array(16);
  for (let off = 0; off < total; off += 64) {
    for (let i = 0; i < 16; i++) M[i] = dv.getUint32(off + i * 4, true);
    let A = a0, B = b0, C = c0, D = d0;
    for (let i = 0; i < 64; i++) {
      let F, g;
      if (i < 16) {
        F = B & C | ~B & D;
        g = i;
      } else if (i < 32) {
        F = D & B | ~D & C;
        g = (5 * i + 1) % 16;
      } else if (i < 48) {
        F = B ^ C ^ D;
        g = (3 * i + 5) % 16;
      } else {
        F = C ^ (B | ~D);
        g = 7 * i % 16;
      }
      F = F + A + K[i] + M[g] >>> 0;
      A = D;
      D = C;
      C = B;
      B = B + rotl(F, S[i]) >>> 0;
    }
    a0 = a0 + A >>> 0;
    b0 = b0 + B >>> 0;
    c0 = c0 + C >>> 0;
    d0 = d0 + D >>> 0;
  }
  const out = new Uint8Array(16);
  const odv = new DataView(out.buffer);
  odv.setUint32(0, a0, true);
  odv.setUint32(4, b0, true);
  odv.setUint32(8, c0, true);
  odv.setUint32(12, d0, true);
  return out;
}
function createHash(_algo) {
  const chunks = [];
  return { update(b) {
    chunks.push(Uint8Array.from(b));
    return this;
  }, digest() {
    let n = 0;
    for (const c of chunks) n += c.length;
    const cat = new Uint8Array(n);
    let o = 0;
    for (const c of chunks) {
      cat.set(c, o);
      o += c.length;
    }
    return md5bytes(cat);
  } };
}

// src/peripherals/common/partition-table.js
var PARTITION_MAGIC = 20650;
var PARTITION_ENTRY_SIZE = 32;
var PARTITION_TABLE_OFFSET = 32768;
var PARTITION_MAGIC_MD5 = 60395;
var TYPE_MAP = {
  app: 0,
  data: 1
};
var SUBTYPE_MAP = {
  // app subtypes
  ota_0: 0,
  ota_1: 16,
  ota_2: 32,
  ota_3: 48,
  ota_4: 64,
  ota_5: 80,
  ota_6: 96,
  ota_7: 112,
  ota_8: 128,
  ota_9: 144,
  ota_10: 160,
  ota_11: 176,
  ota_12: 192,
  ota_13: 208,
  ota_14: 224,
  ota_15: 240,
  test: 0,
  // data subtypes
  ota: 0,
  phy: 1,
  nvs: 2,
  coredump: 3,
  nvs_keys: 4,
  efuse: 5,
  undefined: 6,
  esphttpd: 128,
  fat: 129,
  spiffs: 130
};
function parseHex(v) {
  if (typeof v === "number") return v;
  return parseInt(v, 16);
}
function parseCSV(csvText) {
  const lines = csvText.split("\n");
  const entries = [];
  for (const raw of lines) {
    const line = raw.trim();
    if (!line || line.startsWith("#")) continue;
    const parts = line.split(",").map((s) => s.trim());
    if (parts.length < 5) continue;
    const [name, type, subtype, offset, size, flags] = parts;
    const typeVal = TYPE_MAP[type.toLowerCase()];
    if (typeVal === void 0) continue;
    let subtypeVal = SUBTYPE_MAP[subtype.toLowerCase()];
    if (subtypeVal === void 0) subtypeVal = parseHex(subtype);
    entries.push({
      name,
      type: typeVal,
      subtype: subtypeVal,
      offset: parseHex(offset),
      size: parseHex(size),
      flags: flags ? parseHex(flags) : 0
    });
  }
  return entries;
}
function generateBinary(entries) {
  const n = entries.length;
  const totalBytes = (n + 1) * PARTITION_ENTRY_SIZE;
  const buf = new Uint8Array(totalBytes);
  const view = new DataView(buf.buffer);
  const enc = new TextEncoder();
  for (let i = 0; i < n; i++) {
    const e = entries[i];
    const off = i * PARTITION_ENTRY_SIZE;
    view.setUint16(off, PARTITION_MAGIC, true);
    view.setUint8(off + 2, e.type);
    view.setUint8(off + 3, e.subtype);
    view.setUint32(off + 4, e.offset, true);
    view.setUint32(off + 8, e.size, true);
    const label = enc.encode(e.name);
    for (let j = 0; j < 16; j++) {
      view.setUint8(off + 12 + j, j < label.length ? label[j] : 0);
    }
  }
  const endOff = n * PARTITION_ENTRY_SIZE;
  view.setUint16(endOff, PARTITION_MAGIC_MD5, true);
  for (let i = 2; i < 16; i++) view.setUint8(endOff + i, 255);
  const md5 = createHash("md5").update(buf.subarray(0, endOff)).digest();
  buf.set(new Uint8Array(md5), endOff + 16);
  return buf;
}
function writePartitionTable(flash, csvText) {
  const entries = parseCSV(csvText);
  if (entries.length === 0) return;
  const binary = generateBinary(entries);
  const end = PARTITION_TABLE_OFFSET + binary.length;
  if (end > flash.length) {
    console.warn("Partition table exceeds flash size");
    return;
  }
  flash.set(binary, PARTITION_TABLE_OFFSET);
  const sectorEnd = PARTITION_TABLE_OFFSET + 4096;
  const clearStart = PARTITION_TABLE_OFFSET + binary.length;
  const clearEnd = Math.min(sectorEnd, flash.length);
  if (clearEnd > clearStart) {
    flash.fill(255, clearStart, clearEnd);
  }
}
function parseMacAddress(str) {
  if (!str || typeof str !== "string") return null;
  const parts = str.split(":").map((s) => parseInt(s, 16));
  if (parts.length !== 6 || parts.some((p) => isNaN(p) || p < 0 || p > 255)) return null;
  return parts;
}
function parseFirmwareOffset(v) {
  if (v === void 0 || v === null || v === "") return 0;
  if (typeof v === "number") return v;
  if (typeof v === "string") return parseInt(v, 16);
  return 0;
}

// src/boards/esp32-cam.js
var ESP32_CAM_BOARD_ID = "esp32-cam";
var ESP32_CAM_DEFAULTS = {
  board: ESP32_CAM_BOARD_ID,
  flashSizeMB: 4,
  psramSizeMB: 4,
  psramType: "quad"
};
function applyBoardPreset(config = {}) {
  const cfg = config || {};
  const board = cfg.board ?? "esp32";
  if (board === "esp32") return { board, ...cfg };
  if (board === ESP32_CAM_BOARD_ID) return { ...ESP32_CAM_DEFAULTS, ...cfg };
  throw new Error(`unknown board: ${board} (expected 'esp32' or '${ESP32_CAM_BOARD_ID}')`);
}

// src/engine/esp-xtensa/xtensa-constants.js
function defineEnum3(map) {
  for (const [key, value] of Object.entries(map)) map[value] = key;
  return map;
}
var ExcCause2 = 3;
var LbegRegister2 = 16;
var WindowStart2 = 17;
var MemFaultInfo2 = 72;
var AtomCtrl = 97;
var MiscRegister = 177;
var EpsRegister = 192;
var IntEnable = 226;
var IntClear = 227;
var IntSet2 = 230;
var IntLevel = 231;
var IntStatus = 232;
var CcountReg = 234;
var CcompareReg = 235;
var MiscConfig = 238;
var Ccompare0Reg = 240;
var Ccompare1Reg = 241;
var Ccompare2Reg = 242;
var trapCauseMap = defineEnum3({
  IllegalInstruction: 0,
  Syscall: 1,
  InstructionFetchError: 2,
  LoadStoreError: 3,
  Level1Interrupt: 4,
  Alloca: 5,
  IntegerDivideByZero: 6,
  Reserved1: 7,
  Privileged: 8,
  LoadStoreAlignment: 9,
  Reserved2: 10,
  Reserved3: 11,
  InstrPIFDataError: 12,
  LoadStorePIFDataError: 13,
  InstrPIFAddrError: 14,
  LoadStorePIFAddrError: 15,
  InstTLBMiss: 16,
  InstTLBMultiHit: 17,
  InstFetchPrivilege: 18,
  Reserved4: 19,
  InstFetchProhibited: 20,
  Reserved5: 21,
  Reserved6: 22,
  Reserved7: 23,
  LoadStoreTLBMiss: 24,
  LoadStoreTLBMultiHit: 25,
  LoadStorePrivilege: 26,
  Reserved8: 27,
  LoadProhibited: 28,
  StoreProhibited: 29,
  Reserved9: 30,
  Reserved10: 31,
  CoprocessornDisabled0: 32,
  CoprocessornDisabled1: 33,
  CoprocessornDisabled2: 34,
  CoprocessornDisabled3: 35,
  CoprocessornDisabled4: 36,
  CoprocessornDisabled5: 37,
  CoprocessornDisabled6: 38,
  CoprocessornDisabled7: 39
});
var pieEnabledFlag = false;
var regOff768 = 768;
var regOff832 = 832;
var regOff960 = 960;
var RegisterType2 = {
  PC: 0,
  AR: 16777216,
  Special: 33554432,
  User: 50331648,
  FP: 67108864,
  Mask: 4278190080
};
var PhysicalRegCount = 64;
var Ccompare0IntBit = 6;
var Ccompare1IntBit = 15;
var Ccompare2IntBit = 16;
var IntEnableMaskBase = 536871040;
var IntEnableMask2 = 98368;
var IntEnableMask3 = 16384;
var IntEnableMask4 = 1346372608;

// src/engine/esp-xtensa/xtensa-core.js
var XtensaCore = class {
  set sarByte(addr) {
    this.userRegisters[13] = 255 & addr;
  }
  set fftBitWidth(addr) {
    this.userRegisters[14] = 255 & addr;
  }
  constructor(esp32, index, name, processorId, gdbRegisters) {
    this.esp32 = esp32, this.index = index, this.name = name, this.processorId = processorId, this.gdbRegisters = gdbRegisters, this.physicalRegisters = new Uint32Array(PhysicalRegCount), this.floatRegisters = new Float32Array(16), this.floatRegistersUint32 = new Uint32Array(this.floatRegisters.buffer), this.specialRegisters = new Uint32Array(256), this.userRegisters = new Uint32Array(237), this.qRegisters = new Uint8Array(128), this.qRegistersUint32 = new Uint32Array(this.qRegisters.buffer), this.qRegistersInt32 = new Int32Array(this.qRegisters.buffer), this.qRegistersInt16 = new Int16Array(this.qRegisters.buffer), this.qRegistersUint16 = new Uint16Array(this.qRegisters.buffer), this.qRegistersInt8 = new Int8Array(this.qRegisters.buffer), this.qaccHigh = new Uint8Array(20), this.qaccLow = new Uint8Array(20), this.accx = 0n, this.gdbRegisterCount = this.gdbRegisters.length, this.enabled = true, this.idle = false, this.lightSleep = false, this.pendingInterrupts = false, this.opcodeSegment = 0, this._dataPageIdx = -1, this._dataPageType = -1, this._dataPageData = null, this.pageTable = null, this.mmioHandlers = null, this._memRegions = null, this._cbTraceMemWrite = null, this._cbWriteWatchpoint = null, this._cbGetCpuTicks = null, this.ccompare0Callback = () => {
      this.specialRegisters[IntEnable] |= 1 << Ccompare0IntBit, this.pendingInterrupts = true;
    }, this.ccompare1Callback = () => {
      this.specialRegisters[IntEnable] |= 1 << Ccompare1IntBit, this.pendingInterrupts = true;
    }, this.ccompare2Callback = () => {
      this.specialRegisters[IntEnable] |= 1 << Ccompare2IntBit, this.pendingInterrupts = true;
    }, this.ccompare0Event = esp32.clocks.cpu.createEvent(
      this.ccompare0Callback
    ), this.ccompare1Event = esp32.clocks.cpu.createEvent(
      this.ccompare1Callback
    ), this.ccompare2Event = esp32.clocks.cpu.createEvent(
      this.ccompare2Callback
    ), this.reset();
  }
  attachMemorySystem(pageTable, mmioHandlers, memRegions, callbacks) {
    this.pageTable = pageTable;
    this.mmioHandlers = mmioHandlers;
    this._memRegions = memRegions;
    this._dataPageIdx = -1;
    if (callbacks) {
      this._cbTraceEntry = callbacks.traceEntry ?? null;
      this._cbTraceReturn = callbacks.traceReturn ?? null;
      this._cbTraceMemWrite = callbacks.traceMemWrite ?? null;
      this._cbOnBreak = callbacks.onBreak ?? null;
      this._cbOnUnknownInst = callbacks.onUnknownInst ?? null;
      this._cbWriteWatchpoint = callbacks.writeWatchpoint ?? null;
      this._cbGetCpuTicks = callbacks.getCpuTicks ?? null;
    }
  }
  _updatePageCache(pageIdx) {
    const tbl = this.pageTable.table;
    this._dataPageIdx = pageIdx;
    this._dataPageType = tbl[pageIdx * 2];
    this._dataPageData = tbl[pageIdx * 2 + 1];
  }
  reset() {
    this.pendingInterrupts = false, this.enabled = true, this.idle = false, this.lightSleep = false, this.physicalRegisters.fill(0), this.floatRegisters.fill(0), this.specialRegisters.fill(0), this.userRegisters.fill(0), this.PC = 1073742848, this.PS_WOE = 1, this.setAR(1, 1073741823), this.specialRegisters[CcompareReg] = this.processorId, this.specialRegisters[AtomCtrl] = 1, this.specialRegisters[IntLevel] = 1073741824, this.specialRegisters[CcountReg] = 87426864, this.nextPC = this.PC;
  }
  set SAR(addr) {
    this.specialRegisters[ExcCause2] = addr;
  }
  set PS_INTLEVEL(addr) {
    this.specialRegisters[IntSet2] = 4294967280 & this.specialRegisters[IntSet2] | 15 & addr, this.updateInterrupts();
  }
  get PS_EXCM() {
    return this.specialRegisters[IntSet2] >> 4 & 1;
  }
  set PS_EXCM(addr) {
    this.specialRegisters[IntSet2] = 4294967279 & this.specialRegisters[IntSet2] | (1 & addr) << 4, this.pendingInterrupts = true;
  }
  set PS_UM(addr) {
    this.specialRegisters[IntSet2] = 4294967263 & this.specialRegisters[IntSet2] | (1 & addr) << 5;
  }
  set PS_OWB(addr) {
    this.specialRegisters[IntSet2] = 4294963455 & this.specialRegisters[IntSet2] | (15 & addr) << 8;
  }
  set PS_CALLINC(addr) {
    this.specialRegisters[IntSet2] = 4294770687 & this.specialRegisters[IntSet2] | (3 & addr) << 16;
  }
  set PS_WOE(addr) {
    this.specialRegisters[IntSet2] = 4294705151 & this.specialRegisters[IntSet2] | (1 & addr) << 18;
  }
  set ACC(addr) {
    this.specialRegisters[WindowStart2] = 255 & Math.floor(addr / 4294967296), this.specialRegisters[LbegRegister2] = 0 | addr;
  }
  setAR(addr, value) {
    let regIdx = this.specialRegisters[MemFaultInfo2] << 2;
    this.physicalRegisters[(regIdx + addr) % PhysicalRegCount] = value;
  }
  vector(addr) {
    return this.specialRegisters[IntLevel] + addr;
  }
  exception(addr) {
    pieEnabledFlag && console.log(
      `[${this.name}] CPU exception: ${addr} @ 0x${this.PC.toString(16)}`
    ), this.PS_EXCM ? (this.specialRegisters[EpsRegister] = this.PC, this.nextPC = this.vector(regOff960)) : this.PS_UM ? (this.specialRegisters[MiscRegister] = this.PC, this.nextPC = this.vector(regOff832)) : (this.specialRegisters[MiscRegister] = this.PC, this.nextPC = this.vector(regOff768)), this.specialRegisters[IntStatus] = addr, this.PS_EXCM = 1;
  }
  get CCOUNT() {
    return this._cbGetCpuTicks ? this._cbGetCpuTicks() >>> 0 : this.esp32.clocks.cpu.ticks >>> 0;
  }
  ccompareUpdate(addr, value, event) {
    let enabled = this.specialRegisters[IntEnable] & 1 << value;
    this.specialRegisters[IntEnable] &= ~(1 << value), enabled || event.unschedule();
    let delayTicks = addr === this.CCOUNT ? 4294967295 : addr - this.CCOUNT >>> 0;
    event.schedule(delayTicks);
  }
  _readPageTable(addr, size) {
    const pageIdx = addr >>> PAGE_SHIFT;
    if (pageIdx !== this._dataPageIdx) {
      this._updatePageCache(pageIdx);
    }
    if (this._dataPageType === PTE_TYPE_RAM) {
      return this._memRegions[this._dataPageData][`readUint${size}`](addr);
    }
    if (this._dataPageType === PTE_TYPE_MMIO) {
      if (this._dataPageData & 2147483648) {
        return this.esp32.mapAddress(addr, this.index)[`readUint${size}`](addr);
      }
      const handlerId = this._dataPageData;
      return this.mmioHandlers.read(handlerId, addr, size >>> 3);
    }
    return this.esp32.mapAddress(addr, this.index)[`readUint${size}`](addr);
  }
  _writePageTable(addr, val, size) {
    const pageIdx = addr >>> PAGE_SHIFT;
    if (pageIdx !== this._dataPageIdx) {
      this._updatePageCache(pageIdx);
    }
    if (this._dataPageType === PTE_TYPE_RAM) {
      this._memRegions[this._dataPageData][`writeUint${size}`](addr, val);
    } else if (this._dataPageType === PTE_TYPE_MMIO) {
      if (this._dataPageData & 2147483648) {
        this.esp32.mapAddress(addr, this.index)[`writeUint${size}`](addr, val);
        return;
      }
      const handlerId = this._dataPageData;
      this.mmioHandlers.write(handlerId, addr, val, size >>> 3);
    } else {
      this.esp32.mapAddress(addr, this.index)[`writeUint${size}`](addr, val);
    }
  }
  readUint8(addr) {
    if (this.pageTable) return this._readPageTable(addr, 8);
    return this.esp32.mapAddress(addr, this.index).readUint8(addr);
  }
  readUint16(addr) {
    if (this.pageTable) return this._readPageTable(addr, 16);
    return this.esp32.mapAddress(addr, this.index).readUint16(addr);
  }
  readUint32(addr) {
    if (this.pageTable) return this._readPageTable(addr, 32);
    return this.esp32.mapAddress(addr, this.index).readUint32(addr);
  }
  _writeWithTraps(addr, val, size) {
    if (this._cbWriteWatchpoint) {
      this._cbWriteWatchpoint(addr, this);
    } else {
      this.esp32.writeWatchPoints.has(addr) && this.esp32.onBreak?.(this);
    }
    if (this._cbTraceMemWrite) {
      this._cbTraceMemWrite(this.index, this.PC, addr, val, size);
    } else {
      this.esp32.trace.traceMemWrite(this.index, this.PC, addr, val, size);
    }
  }
  writeUint8(addr, value, regIdx = false) {
    if (regIdx && 0 === addr) {
      this.writeSpecialRegister(MiscConfig, addr), this.exception(trapCauseMap.StoreProhibited);
      return;
    }
    this._writeWithTraps(addr, value, 8);
    if (this.pageTable) {
      this._writePageTable(addr, value, 8);
      return;
    }
    this.esp32.mapAddress(addr, this.index).writeUint8(addr, value);
  }
  writeUint16(addr, value, regIdx = false) {
    if (regIdx && 0 === addr) {
      this.writeSpecialRegister(MiscConfig, addr), this.exception(trapCauseMap.StoreProhibited);
      return;
    }
    this._writeWithTraps(addr, value, 16);
    if (this.pageTable) {
      this._writePageTable(addr, value, 16);
      return;
    }
    this.esp32.mapAddress(addr, this.index).writeUint16(addr, value);
  }
  writeUint32(addr, value, regIdx = false) {
    if (regIdx && 0 === addr) {
      this.writeSpecialRegister(MiscConfig, addr), this.exception(trapCauseMap.StoreProhibited), false;
      return false;
    }
    this._writeWithTraps(addr, value, 32);
    if (this.pageTable) {
      this._writePageTable(addr, value, 32);
      return true;
    }
    this.esp32.mapAddress(addr, this.index).writeUint32(addr, value);
    return true;
  }
  writeSpecialRegister(addr, value) {
    switch (addr) {
      case IntSet2:
        this.pendingInterrupts = true;
        break;
      case Ccompare0Reg:
        this.ccompareUpdate(value, Ccompare0IntBit, this.ccompare0Event);
        break;
      case Ccompare1Reg:
        this.ccompareUpdate(value, Ccompare1IntBit, this.ccompare1Event);
        break;
      case Ccompare2Reg:
        this.ccompareUpdate(value, Ccompare2IntBit, this.ccompare2Event);
        break;
      case IntEnable:
        this.specialRegisters[IntEnable] |= value & IntEnableMaskBase, this.pendingInterrupts = true;
        return;
      case IntClear: {
        let regIdx = IntEnableMask4 | IntEnableMask3 | IntEnableMask2 | IntEnableMaskBase;
        this.specialRegisters[IntEnable] &= ~(value & regIdx);
        return;
      }
    }
    this.specialRegisters[addr] = value;
  }
  gdbReadRegister(addr) {
    let value = this.gdbRegisters[addr] & RegisterType2.Mask, regIdx = this.gdbRegisters[addr] & ~RegisterType2.Mask;
    switch (value) {
      case RegisterType2.PC:
        return this.PC;
      case RegisterType2.Special:
        return this.specialRegisters[regIdx];
      case RegisterType2.User:
        return this.userRegisters[regIdx];
      case RegisterType2.AR:
        return this.physicalRegisters[regIdx];
      case RegisterType2.FP:
        return this.floatRegistersUint32[regIdx];
    }
    return console.warn("Unknown register", addr), 0;
  }
  gdbWriteRegister(addr, value) {
    let regIdx = this.gdbRegisters[addr] & RegisterType2.Mask, regType = this.gdbRegisters[addr] & ~RegisterType2.Mask;
    switch (regIdx) {
      case RegisterType2.PC:
        this.PC = value;
        break;
      case RegisterType2.Special:
        this.specialRegisters[regType] = value;
        break;
      case RegisterType2.User:
        this.userRegisters[regType] = value;
        break;
      case RegisterType2.AR:
        this.physicalRegisters[regType] = value;
        break;
      case RegisterType2.FP:
        this.floatRegistersUint32[regType] = value;
        break;
      default:
        console.warn("Unknown register", addr);
    }
  }
};

// src/engine/esp-xtensa/wasm-loader.js
init_wasm_memory_layout();
var CORE_OFF_PHYS_REGS = 16;
var CORE_OFF_SPECIAL_REGS = 336;
var CORE_OFF_ENABLED = 2528;
var CORE_OFF_IDLE = 2532;
var CORE_OFF_LIGHT_SLEEP = 2536;
var CORE_OFF_PENDING_INT = 2540;
var CORE_OFF_OPCODE_SEG = 2544;
var CORE_OFF_PC = 2584;
var CORE_OFF_NEXT_PC = 2588;
var MEM_FAULT_INFO = 72;
var NATIVE_HANDLER_FLAG2 = 2147483648;
var HID_SHA = 0;
var HID_RNG = 1;
var HID_EFUSE = 2;
var HID_IO_MUX = 3;
var HID_SYSCON = 4;
var HID_GPIO = 5;
var HID_AES = 6;
var HID_FRC_TIMER = 7;
var HID_TIMG0 = 8;
var HID_UART = 9;
var HID_I2C = 10;
var HID_SPI = 11;
var HID_TIMG1 = 12;
var HID_TWAI = 13;
var HID_RSA = 14;
var HID_RTC = 15;
var HID_LEDC = 16;
var HID_PCNT = 17;
var HID_RMT = 18;
var HID_I2S = 19;
var HID_SDMMC = 20;
var HID_WIFI_ANALOG = 22;
var HID_WIFI_MAC = 23;
var HID_DPORT = 24;
var HID_SDIO_SLAVE = 25;
var HID_FE = 26;
var HID_MCPWM = 27;
var HID_UHCI = 28;
var HID_EMAC = 29;
var HID_SWEEP = 30;
var HID_INVALID_MEM = 31;
var HID_STUB_ZERO = 32;
var HID_BT_RF = 33;
var WasmLoader = class {
  constructor() {
    this.instance = null;
    this.memory = null;
    this.exports = null;
    this.esp32 = null;
    this._mmioRegistry = null;
    this._callbacks = {};
    this._wasmCores = [];
    this._ccompareEvents = [];
    this._efuseCmdClearEvent = null;
    this._uartRxTimeoutEvents = null;
    this._uartIntCheckEvents = null;
    this._mmioCount = new Float64Array(64);
    this._mmioPageCount = /* @__PURE__ */ new Map();
    this._mapCache = /* @__PURE__ */ new Map();
    this._i2sTxDataHook = null;
    this._wifiMacTx = null;
    this._wifiApRxFrame = null;
    this._wifiApSendEth = null;
    this._wifiApConnected = null;
  }
  async load(wasmBytes, esp32, memoryObj) {
    this.esp32 = esp32;
    this.mmioRegistry = esp32.mmioHandlers;
    this._callbacks = esp32._coreCallbacks || {};
    this.memory = memoryObj;
    this.memoryBuffer = memoryObj.buffer;
    const env = {
      memory: memoryObj,
      mmio_read: (handlerId, addr, size) => {
        if (handlerId & NATIVE_HANDLER_FLAG2) return 0;
        this._mmioCount[handlerId >>> 0]++;
        this._mmioPageCount.set((addr >>> 12).toString(16), (this._mmioPageCount.get((addr >>> 12).toString(16)) || 0) + 1);
        const h = this.mmioRegistry?.handlers[handlerId];
        if (!h) return 0;
        try {
          if (size === 1) return h.readUint8?.(addr) ?? 0;
          if (size === 2) return h.readUint16?.(addr) ?? 0;
          return h.readUint32?.(addr) ?? 0;
        } catch (e) {
          console.warn(`mmio_read error: handler=${handlerId} addr=0x${addr.toString(16)} size=${size}: ${e.message}`);
          return 0;
        }
      },
      mmio_write: (handlerId, addr, val, size) => {
        this._mmioCount[handlerId >>> 0]++;
        this._mmioPageCount.set((addr >>> 12).toString(16), (this._mmioPageCount.get((addr >>> 12).toString(16)) || 0) + 1);
        const h = this.mmioRegistry?.handlers[handlerId];
        if (!h) return;
        try {
          if (size === 1) h.writeUint8?.(addr, val);
          else if (size === 2) h.writeUint16?.(addr, val);
          else h.writeUint32?.(addr, val);
        } catch (e) {
          console.warn(`mmio_write error: handler=${handlerId} addr=0x${addr.toString(16)} val=0x${val.toString(16)} size=${size}: ${e.message}`);
        }
        if (handlerId === 89 && (addr >= 1072955392 && addr <= 1072955424)) {
          console.log(`[UART] write handler=${handlerId} addr=0x${addr.toString(16)} val=0x${(val >>> 0).toString(16)} size=${size}`);
        }
      },
      on_unknown_inst: (coreIdx, pc, opcode) => this._callbacks.onUnknownInst?.(coreIdx, pc, opcode),
      on_break: (coreIdx) => this._callbacks.onBreak?.(coreIdx),
      write_watchpoint: (addr, coreIdx) => {
        this._callbacks.writeWatchpoint?.(addr, coreIdx);
        return 0;
      },
      trace_mem_write: (coreIdx, pc, addr, val, size) => this._callbacks.traceMemWrite?.(coreIdx, pc, addr, val, size),
      trace_entry: (coreIdx, pc, arg) => this._callbacks.traceEntry?.(coreIdx, pc, arg),
      trace_return: (coreIdx, pc, retVal, retAddr) => this._callbacks.traceReturn?.(coreIdx, pc, retVal, retAddr),
      get_cpu_ticks: () => this._callbacks.getCpuTicks?.() ?? 0,
      get_cpu_cycles: () => this._callbacks.getCpuCycles?.() ?? 0,
      get_sim_freq: () => this._callbacks.getSimFreq?.() ?? 0,
      get_cpu_freq: () => this._callbacks.getCpuFreq?.() ?? 0,
      ccompare_schedule: (coreIdx, which, value) => {
        const intBits = [6, 15, 16];
        const intBit = intBits[which];
        if (intBit === void 0) return;
        if (!this._ccompareEvents) this._ccompareEvents = [];
        if (!this._ccompareEvents[coreIdx]) this._ccompareEvents[coreIdx] = [];
        if (!this._ccompareEvents[coreIdx][which]) {
          const coreOff = coreIdx * 4096;
          const intEnableOff = coreOff + 336 + 226 * 4;
          const pendingIntOff = coreOff + 2540;
          const u32 = new Uint32Array(this.memoryBuffer);
          this._ccompareEvents[coreIdx][which] = this.esp32.clocks.cpu.createEvent(() => {
            u32[intEnableOff >>> 2] |= 1 << intBit;
            u32[pendingIntOff >>> 2] = 1;
          });
        }
        const ev = this._ccompareEvents[coreIdx][which];
        if (value === 4294967295) {
          ev.schedule(0);
        } else {
          ev.schedule(value);
        }
      },
      ccompare_unschedule: (coreIdx, which) => {
        const ev = this._ccompareEvents?.[coreIdx]?.[which];
        if (ev) ev.unschedule();
      },
      // Native UART bridge: TX byte emitted by the Rust UARCtrl goes to the JS
      // UartController.onTX callback (the ring-buffer hook wired by the worker).
      js_uart_tx_byte: (idx, byte) => {
        const u = this.esp32?.uart?.[idx];
        if (u?.onTX) u.onTX(byte);
        else console.log(`[UART-BRIDGE] dropped byte idx=${idx} byte=${byte}`);
      },
      is_window_inst: (_corePtr) => 0,
      map_address: (coreIdx, addr) => {
        if (!this.esp32) return 0;
        this.esp32.mapAddress(addr, coreIdx);
        return 1;
      },
      js_flash_write_override: () => ReadonlyMemory.override ? 1 : 0,
      map_read: (coreIdx, addr, size) => {
        if (!this.esp32) return 0;
        this._mmioCount[40]++;
        const page = addr >>> 12;
        let c = this._mapCache.get(page);
        if (c && c.multi) c = null;
        if (!c) {
          c = { region: this.esp32.mapAddress(addr, coreIdx), m: this.esp32.mmuEntryFor(addr, coreIdx), multi: false };
          if (c.m === void 0) {
            const base = page << 12;
            const r0 = this.esp32.mapAddress(base, coreIdx);
            const r1 = this.esp32.mapAddress(base | 2047, coreIdx);
            const r2 = this.esp32.mapAddress(base | 4095, coreIdx);
            if (r0 !== c.region || r1 !== c.region || r2 !== c.region) c.multi = true;
          }
          if (!c.multi) this._mapCache.set(page, c);
        } else if (c.m !== void 0 && c.m !== this.esp32.mmuEntryFor(addr, coreIdx)) {
          c.region = this.esp32.mapAddress(addr, coreIdx);
          c.m = this.esp32.mmuEntryFor(addr, coreIdx);
        }
        const rsize = size <= 8 ? 8 : size <= 16 ? 16 : 32;
        return c.region[`readUint${rsize}`](addr);
      },
      map_write: (coreIdx, addr, val, size) => {
        if (!this.esp32) return;
        this._mmioCount[41]++;
        const page = addr >>> 12;
        let c = this._mapCache.get(page);
        if (c && c.multi) c = null;
        if (!c) {
          c = { region: this.esp32.mapAddress(addr, coreIdx), m: this.esp32.mmuEntryFor(addr, coreIdx), multi: false };
          if (c.m === void 0) {
            const base = page << 12;
            const r0 = this.esp32.mapAddress(base, coreIdx);
            const r1 = this.esp32.mapAddress(base | 2047, coreIdx);
            const r2 = this.esp32.mapAddress(base | 4095, coreIdx);
            if (r0 !== c.region || r1 !== c.region || r2 !== c.region) c.multi = true;
          }
          if (!c.multi) this._mapCache.set(page, c);
        } else if (c.m !== void 0 && c.m !== this.esp32.mmuEntryFor(addr, coreIdx)) {
          c.region = this.esp32.mapAddress(addr, coreIdx);
          c.m = this.esp32.mmuEntryFor(addr, coreIdx);
        }
        const wsize = size <= 8 ? 8 : size <= 16 ? 16 : 32;
        c.region[`writeUint${wsize}`](addr, val);
      },
      has_breakpoint: (coreIdx, pc) => this._callbacks.hasBreakpoint?.(coreIdx, pc) ? 1 : 0,
      abort: (msg, file, line, col) => {
        console.error(`[WASM ABORT] msg=${msg} file=${file} line=${line} col=${col}`);
      },
      js_interrupt: (irq, level) => {
        this._mmioCount[42]++;
        if (this.esp32?.interrupt) {
          this.esp32.interrupt(irq >>> 0, level !== 0);
        }
      },
      js_log_u32: (val) => {
        console.log(`[WASM] 0x${(val >>> 0).toString(16).padStart(8, "0")}`);
      },
      js_log_str: (ptr, len) => {
        this._mmioCount[43]++;
        const buf = new Uint8Array(this.memory.buffer);
        const bytes = buf.slice(ptr, ptr + len);
        console.log(`[WASM] ${new TextDecoder().decode(bytes)}`);
      },
      js_spi_flash_get_byte: (off) => {
        const f = this.esp32?.flash;
        return f && off < f.length ? f[off] : 0;
      },
      js_spi_flash_set_byte: (off, val) => {
        const f = this.esp32?.flash;
        if (f && off < f.length) f[off] = val;
        const m = this.esp32?._flashMirror;
        if (m && off < m.length) m[off] = val;
      },
      // Virtual SD card block storage (JS parity with js_spi_flash_* byte
      // bridges). Rust passes a linear-memory scratch ptr (512B).
      js_sd_num_blocks: () => {
        const sd = this.esp32?.sdData;
        return sd ? sd.length / 512 >>> 0 : 32768;
      },
      js_sd_read_block: (block, ptr) => {
        const sd = this.esp32?.sdData;
        const mem = new Uint8Array(this.memory.buffer);
        if (!sd || block * 512 + 512 > sd.length) {
          mem.fill(0, ptr, ptr + 512);
          return;
        }
        mem.set(sd.subarray(block * 512, block * 512 + 512), ptr);
      },
      js_sd_write_block: (block, ptr) => {
        const sd = this.esp32?.sdData;
        if (!sd || block * 512 + 512 > sd.length) return;
        const mem = new Uint8Array(this.memory.buffer);
        sd.set(mem.subarray(ptr, ptr + 512), block * 512);
      },
      // Native I2S TX data hook — forwards consumed DMA TX words to the JS
      // host (JS parity with I2sPeripheral.onTxData).
      js_i2s_tx_data: (idx, ptr, len) => {
        if (!this._i2sTxDataHook) return;
        const buf = new Uint8Array(this.memory.buffer);
        const words = new Uint32Array(buf.slice(ptr, ptr + len * 4).buffer);
        this._i2sTxDataHook(idx, words);
      },
      // Native RTC bridge: sleep wakeup on the rcSlow clock event queue.
      // Fires native_rtc_fire_sleep_wakeup when rcSlow ticks reach the target.
      js_rtc_schedule_wakeup: (target) => {
        if (this.esp32?.clocks?.rcSlow?.createEvent && this.exports?.native_rtc_fire_sleep_wakeup) {
          if (!this._rtcWakeupEvent) {
            this._rtcWakeupEvent = this.esp32.clocks.rcSlow.createEvent(() => {
              try {
                this.exports.native_rtc_fire_sleep_wakeup();
              } catch (e) {
                console.error("[WASM-RTC-WAKEUP-ERR]", e?.message || e);
              }
            });
          }
          const delta = Math.max(0, (target >>> 0) - (this.esp32.clocks.rcSlow?.ticks ?? 0));
          this._rtcWakeupEvent.schedule(delta);
        }
      },
      js_set_core_enabled: (idx, enabled) => {
        if (this.esp32?._wasmCores?.[idx]) this.esp32._wasmCores[idx].enabled = enabled !== 0;
      },
      js_on_analog_read: (pin, cfg) => this.esp32?.onAnalogRead?.(pin, cfg) ?? 0,
      js_on_touch_read: (pad) => this.esp32?.onTouchRead?.(pad) ?? 1e3,
      js_dac_write: (channel, value) => {
        this.esp32?.onDacWrite?.(channel, value);
      },
      js_core_enter_light_sleep: (idx) => {
        this.esp32?._wasmCores?.[idx]?.enterLightSleep?.();
      },
      js_core_exit_light_sleep: (idx) => {
        this.esp32?._wasmCores?.[idx]?.exitLightSleep?.();
      },
      js_set_reset_reason: (reason) => {
        if (this.esp32) this.esp32.resetReason = reason >>> 0;
      },
      js_get_reset_reason: () => this.esp32?.resetReason ?? 0,
      js_reset_soc: () => {
        try {
          this.esp32?.reset?.();
        } catch (e) {
          console.error("[WASM-RTC-RESET-ERR]", e?.message || e);
        }
      },
      js_reset_core: (idx) => {
        try {
          this.esp32?.cores?.[idx]?.reset?.();
          this.esp32?._wasmCores?.[idx]?.reset?.();
        } catch (e) {
          console.error("[WASM-RTC-CORE-RESET-ERR]", e?.message || e);
        }
      },
      // Native WiFi MAC bridge: TX-complete clock event (JS parity with
      // wifi.txCompleteEvent.schedule(1e3) — fires native_wifi_tx_complete →
      // on_tx_complete → setEvent(0x80)).
      js_wifi_tx_complete: (nanos) => {
        if (this.esp32?.clocks?.cpu?.createEvent && this.exports?.native_wifi_tx_complete) {
          if (!this._wifiTxCompleteEvent) {
            this._wifiTxCompleteEvent = this.esp32.clocks.cpu.createEvent(() => {
              this.exports.native_wifi_tx_complete();
            });
          }
          this._wifiTxCompleteEvent.schedule(nanos);
        }
      },
      // Native WiFi MAC TX frame bridge — JS parity with the JS peripheral's
      // writeUint32 DMA_TXBUF arm calling this.onTX(hVal) synchronously.
      js_wifi_send_frame: (ptr, len) => {
        const buf = new Uint8Array(this.memory.buffer);
        const frame = buf.slice(ptr, ptr + len);
        this._wifiMacTx?.(frame);
      },
      // ESP-NOW action-frame TX bridge — raw 802.11 MPDU bytes for the host
      // medium hook (installed by worker-entry; two-node delivery).
      js_espnow_tx_frame: (ptr, len) => {
        const buf = new Uint8Array(this.memory.buffer);
        const frame = buf.slice(ptr, ptr + len);
        this._espnowTx?.(frame);
      },
      // Native WiFi AP (NativeInternetAP) host bridges — the Rust AP builds
      // frames in its statics and hands pointers to the worker, which owns
      // the gateway WebSocket, pcap records and RX delivery (sendFrame).
      // Hooks (_wifiApRxFrame/_wifiApSendEth/_wifiApConnected) are installed
      // by worker-entry after instantiation.
      js_wifi_ap_rx_frame: (ptr, len) => {
        this._wifiApRxFrame?.(ptr, len);
      },
      js_wifi_ap_send_eth: (ptr, len) => {
        this._wifiApSendEth?.(ptr, len);
      },
      js_wifi_ap_connected: () => {
        this._wifiApConnected?.();
      },
      // Native DPORT shell bridges — behavioral side-effects of the native
      // DPORT handler (clock tree, core1 reset/stall, peripheral clock-gate
      // /reset enables, cross-core IRQs). The interrupt matrix is fully Rust
      // (InterruptMatrixPeripheral) — no JS matrix round-trip.
      js_dport_get_cpu_clock_period: () => this.esp32?.clocks?.cpuClockPeriod ?? 0,
      js_dport_core1_reset: () => {
        try {
          this.esp32?.cores?.[1]?.reset?.();
          this.esp32?._wasmCores?.[1]?.reset?.();
          if (this.esp32?.cores?.[1]?.specialRegisters) {
            this.esp32.cores[1].specialRegisters[22] = 31;
          }
        } catch (e) {
          console.error("[WASM-DPORT-CORE1-RESET-ERR]", e?.message || e);
        }
      },
      js_dport_set_cpu_clock_period: (val) => {
        const clk = this.esp32?.clocks;
        if (!clk) return;
        clk.cpuClockPeriod = 3 & val;
        clk.update?.();
      },
      js_dport_peri_clk_en: (val) => {
        const clk = this.esp32?.clocks;
        if (!clk) return;
        clk.tg0Timer.enable = !!(val & 8192);
        clk.tg0WDT.enable = !!(val & 8192);
        clk.tg1Timer.enable = !!(val & 32768);
        clk.tg1WDT.enable = !!(val & 32768);
        clk.ledc.enable = !!(val & 2048);
        clk.rmt.enable = !!(val & 512);
        clk.uart0.enable = !!(val & 4);
        clk.uart1.enable = !!(val & 32);
        clk.uart2.enable = !!(val & 8388608);
        clk.spi2.enable = !!(val & 64);
        clk.i2c0.enable = !!(val & 128);
      },
      js_dport_peri_rst_en: (val) => {
        const chip2 = this.esp32;
        if (!chip2) return;
        const reset = (base, bit) => chip2.resetPeripheral(base, !!(val & bit));
        reset(1073061888, 16384);
        reset(1073033216, 128);
        reset(1073115136, 262144);
        reset(1073016832, 16);
        reset(1073139712, 2097152);
        reset(1073057792, 2048);
        reset(1073049600, 1024);
        reset(1073045504, 512);
        reset(1072967680, 2);
        reset(1072963584, 2);
        reset(1073102848, 64);
        reset(1073106944, 65536);
        reset(1073082368, 8192);
        reset(1073086464, 32768);
        reset(1073131520, 524288);
        reset(1072955392, 4);
        reset(1073020928, 32);
        reset(1073143808, 8388608);
      },
      js_dport_refresh_core1_enabled: () => {
        if (!this.esp32) return;
        this.esp32.cores[1].enabled = (this.exports?.native_dport_get_core1_enabled?.() ?? 0) !== 0;
      },
      js_rtc_pause_wdts: () => {
        try {
          this.esp32?.clocks?.pauseApbClocks?.();
        } catch {
        }
      },
      js_rtc_resume_wdts: () => {
        try {
          this.esp32?.clocks?.resumeApbClocks?.();
        } catch {
        }
      }
    };
    const importObj = { env };
    if (wasmBytes instanceof WebAssembly.Module) {
      this.instance = new WebAssembly.Instance(wasmBytes, importObj);
    } else {
      const result = await WebAssembly.instantiate(wasmBytes, importObj);
      this.instance = result.instance;
    }
    this.exports = this.instance.exports;
    if (this.exports.native_peripheral_init) {
      this.exports.native_peripheral_init();
    }
    if (this.exports.native_flash_init) {
      this.exports.native_flash_init(FLASH_DATA_OFFSET, MMU_TABLE_REGION_ID);
    }
    if (this.exports.native_flash_seg3_off && this.esp32?.flash) {
      try {
        const f = this.esp32.flash;
        const u32 = (o) => (f[o] | f[o + 1] << 8 | f[o + 2] << 16 | f[o + 3] << 24) >>> 0;
        let seg3 = 0;
        for (const base of [65536, 4096, 0]) {
          if (f[base] !== 233) continue;
          const n = f[base + 1];
          let off = base + 24;
          for (let i = 0; i < n && off + 8 <= f.length; i++) {
            if (u32(off) === 1074593824) {
              seg3 = off + 8;
              break;
            }
            off += 8 + u32(off + 4);
          }
          if (seg3) break;
        }
        if (seg3) this.exports.native_flash_seg3_off(seg3);
      } catch {
      }
    }
    const ptU32 = new Uint32Array(this.memory.buffer, PAGE_TABLE_OFFSET);
    const setPTE = (addr, hid) => {
      const page = addr >>> 12;
      ptU32[page * 2] = 2;
      ptU32[page * 2 + 1] = NATIVE_HANDLER_FLAG2 | hid;
    };
    setPTE(1072705536, HID_SHA);
    setPTE(1072697344, HID_AES);
    setPTE(1073082368, HID_TIMG0);
    setPTE(1073086464, HID_TIMG1);
    setPTE(1072971776, HID_GPIO);
    setPTE(1072992256, HID_IO_MUX);
    setPTE(1072984064, HID_FRC_TIMER);
    setPTE(1072955392, HID_UART);
    setPTE(1073020928, HID_UART);
    setPTE(1073143808, HID_UART);
    setPTE(1073033216, HID_I2C);
    setPTE(1073115136, HID_I2C);
    setPTE(1072963584, HID_SPI);
    setPTE(1072967680, HID_SPI);
    setPTE(1073102848, HID_SPI);
    setPTE(1073106944, HID_SPI);
    setPTE(1073061888, HID_EFUSE);
    setPTE(1073111040, HID_SYSCON);
    setPTE(1073131520, HID_TWAI);
    setPTE(1072701440, HID_RSA);
    setPTE(1072988160, HID_RTC);
    setPTE(1073057792, HID_LEDC);
    setPTE(1073049600, HID_PCNT);
    setPTE(1073045504, HID_RMT);
    setPTE(1073016832, HID_I2S);
    setPTE(1073139712, HID_I2S);
    setPTE(1073119232, HID_SDMMC);
    setPTE(1072693248, HID_DPORT);
    setPTE(1073172480, HID_RNG);
    setPTE(1610829824, HID_RNG);
    setPTE(1073012736, HID_WIFI_ANALOG);
    setPTE(1073164288, HID_WIFI_MAC);
    setPTE(1610670080, HID_WIFI_ANALOG);
    setPTE(1610821632, HID_WIFI_MAC);
    setPTE(1610612736, HID_UART);
    setPTE(1610678272, HID_UART);
    setPTE(1610801152, HID_MCPWM);
    setPTE(1610690560, HID_I2C);
    setPTE(1610772480, HID_I2C);
    setPTE(1073053696, HID_SDIO_SLAVE);
    setPTE(1072979968, HID_FE);
    setPTE(1073078272, HID_MCPWM);
    setPTE(1073135616, HID_MCPWM);
    setPTE(1073037312, HID_UHCI);
    setPTE(1073004544, HID_UHCI);
    setPTE(1073123328, HID_EMAC);
    setPTE(1073127424, HID_EMAC);
    setPTE(1072709632, HID_SWEEP);
    setPTE(1073000448, HID_SWEEP);
    setPTE(1073041408, HID_SWEEP);
    setPTE(1073065984, HID_SWEEP);
    setPTE(1072820224, HID_SWEEP);
    setPTE(1072799744, HID_SWEEP);
    setPTE(1073074176, HID_STUB_ZERO);
    setPTE(1073073152, HID_STUB_ZERO);
    setPTE(1073168384, HID_STUB_ZERO);
    setPTE(1073221632, HID_INVALID_MEM);
    setPTE(1073156096, HID_BT_RF);
    setPTE(1073160192, HID_BT_RF);
    console.log(`[WASM] TIMG0 PTE: type=${ptU32[261983 * 2]} data=0x${(ptU32[261983 * 2 + 1] >>> 0).toString(16)}`);
    const rustPtOff = this.exports.native_get_pt_offset();
    console.log(`[WASM] JS_PTOFS=0x${PAGE_TABLE_OFFSET.toString(16)} RUST_PTOFS=0x${rustPtOff.toString(16)} MATCH=${PAGE_TABLE_OFFSET === rustPtOff}`);
    this.exports.native_print_pt_entry(261983);
    const shaPage = 1072705536 >>> 12;
    console.log(`[WASM] SHA PTE: type=${ptU32[shaPage * 2]} data=0x${(ptU32[shaPage * 2 + 1] >>> 0).toString(16)}`);
    const mmuPage = 1072988160 >>> 12;
    console.log(`[WASM] MMU 0x3FF48 PTE: type=${ptU32[mmuPage * 2]} data=0x${(ptU32[mmuPage * 2 + 1] >>> 0).toString(16)}`);
    const flashPage = 1074536448 >>> 12;
    console.log(`[WASM] FLASHWIN 0x400C2000 PTE: type=${ptU32[flashPage * 2]} data=0x${(ptU32[flashPage * 2 + 1] >>> 0).toString(16)}`);
    const iramPage = 1074266112 >>> 12;
    console.log(`[WASM] IRAM 0x40080000 PTE: type=${ptU32[iramPage * 2]} data=0x${(ptU32[iramPage * 2 + 1] >>> 0).toString(16)}`);
    try {
      this.exports.native_timg0_init();
      console.log(`[WASM] TIMG0 init OK, access count=${this.exports.native_timg0_get_access_count()}`);
    } catch (e) {
      console.error(`[WASM] TIMG0 init FAILED: ${e.message}`);
    }
    if (this.exports.native_timg0_reset_count) this.exports.native_timg0_reset_count();
    try {
      if (this.exports.native_timg1_init) this.exports.native_timg1_init();
      if (this.exports.native_timg1_reset) this.exports.native_timg1_reset();
      console.log("[WASM] TIMG1 init OK");
    } catch (e) {
      console.error(`[WASM] TIMG1 init FAILED: ${e.message}`);
    }
    try {
      if (this.exports.native_frc_timer_init) this.exports.native_frc_timer_init();
    } catch (e) {
      console.error(`[WASM] FRC timer init FAILED: ${e.message}`);
    }
    try {
      if (this.exports.native_gpio_init) this.exports.native_gpio_init();
      this.seedNativeGpio();
      console.log("[WASM] GPIO init OK");
    } catch (e) {
      console.error(`[WASM] GPIO init FAILED: ${e.message}`);
    }
    try {
      if (this.exports.native_uart_init) this.exports.native_uart_init();
      if (this.exports.native_uart_reset) this.exports.native_uart_reset();
      console.log("[WASM] UART init OK");
    } catch (e) {
      console.error(`[WASM] UART init FAILED: ${e.message}`);
    }
    try {
      if (this.exports.native_i2c_init) this.exports.native_i2c_init();
      if (this.exports.native_i2c_reset) this.exports.native_i2c_reset();
      console.log("[WASM] I2C init OK");
    } catch (e) {
      console.error(`[WASM] I2C init FAILED: ${e.message}`);
    }
    try {
      if (this.exports.native_spi_init) this.exports.native_spi_init();
      if (this.exports.native_spi_reset) this.exports.native_spi_reset();
      console.log("[WASM] SPI init OK");
    } catch (e) {
      console.error(`[WASM] SPI init FAILED: ${e.message}`);
    }
    try {
      if (this.exports.native_efuse_reset) this.exports.native_efuse_reset();
      if (this.exports.native_syscon_reset) this.exports.native_syscon_reset();
      console.log("[WASM] EFUSE/SYSCON init OK");
    } catch (e) {
      console.error(`[WASM] EFUSE/SYSCON init FAILED: ${e.message}`);
    }
    try {
      if (this.exports.native_twai_init) this.exports.native_twai_init();
      if (this.exports.native_twai_reset) this.exports.native_twai_reset();
      console.log("[WASM] TWAI init OK");
    } catch (e) {
      console.error(`[WASM] TWAI init FAILED: ${e.message}`);
    }
    try {
      if (this.exports.native_sdmmc_init) this.exports.native_sdmmc_init();
      if (this.exports.native_sdmmc_reset) this.exports.native_sdmmc_reset();
      console.log("[WASM] SDMMC init OK");
    } catch (e) {
      console.error(`[WASM] SDMMC init FAILED: ${e.message}`);
    }
    try {
      if (this.exports.native_rsa_init) this.exports.native_rsa_init();
      if (this.exports.native_rsa_reset) this.exports.native_rsa_reset();
      console.log("[WASM] RSA init OK");
    } catch (e) {
      console.error(`[WASM] RSA init FAILED: ${e.message}`);
    }
    try {
      if (this.exports.native_rtc_init) this.exports.native_rtc_init();
      if (this.exports.native_rtc_reset) this.exports.native_rtc_reset();
      console.log("[WASM] RTC init OK");
    } catch (e) {
      console.error(`[WASM] RTC init FAILED: ${e.message}`);
    }
    try {
      if (this.exports.native_ledc_init) this.exports.native_ledc_init();
      if (this.exports.native_ledc_reset) this.exports.native_ledc_reset();
      console.log("[WASM] LEDC init OK");
    } catch (e) {
      console.error(`[WASM] LEDC init FAILED: ${e.message}`);
    }
    try {
      if (this.exports.native_pcnt_init) this.exports.native_pcnt_init();
      if (this.exports.native_pcnt_reset) this.exports.native_pcnt_reset();
      console.log("[WASM] PCNT init OK");
    } catch (e) {
      console.error(`[WASM] PCNT init FAILED: ${e.message}`);
    }
    try {
      if (this.exports.native_rmt_init) this.exports.native_rmt_init();
      if (this.exports.native_rmt_reset) this.exports.native_rmt_reset();
      console.log("[WASM] RMT init OK");
    } catch (e) {
      console.error(`[WASM] RMT init FAILED: ${e.message}`);
    }
    try {
      const wantShim = !!(this.esp32?.bleShimEnabled || this.esp32?.config?.bleShim);
      if (this.exports.native_bt_diag_disable) this.exports.native_bt_diag_disable(wantShim ? 0 : 1);
      if (wantShim) console.log("[WASM] BT shim ENABLED (bleShim opt-in)");
    } catch (e) {
      console.error(`[WASM] BT diag disable FAILED: ${e.message}`);
    }
    console.log(`[WASM] ESP32 engine (wasm) initialized`);
    return this;
  }
  createCores(count) {
    this._wasmCores = [];
    for (let i = 0; i < count; i++) {
      const wc = new WasmCore(this, i);
      const processorId = this.esp32?.cores[i]?.processorId ?? 0;
      wc.init(processorId, 0);
      wc.reset();
      this._wasmCores.push(wc);
    }
    return this._wasmCores;
  }
  get wasmCores() {
    return this._wasmCores;
  }
  setDebugLog(enabled) {
    const u32 = new Uint32Array(this.memory.buffer);
    for (let i = 0; i < this._wasmCores.length; i++) {
      const baseOff = i * CORE_STATE_SIZE;
      const DEBUG_LOG_OFF = 2584 + 7 * 4;
      u32[baseOff + DEBUG_LOG_OFF >>> 2] = enabled ? 1 : 0;
    }
  }
  // Push the JS GPIO pin input levels + strap into the Rust GpioController
  // (mirrors applyBasicSetup pinInputs; call after chip.reset() too — reset
  // clears native pin state). One-shot bulk export: 41 x u32 LE into the
  // GPIO_SEED_SCRATCH (pins 0..39 input levels, index 40 = strap).
  seedNativeGpio() {
    if (!this.exports?.native_gpio_seed) return;
    const gpio = this.esp32?.gpio;
    if (!gpio?.pins) return;
    const ptr = this.exports.native_gpio_seed_scratch();
    const u32 = new Uint32Array(this.memory.buffer, ptr, 41);
    for (let i = 0; i < gpio.pins.length; i++) {
      const pin = gpio.pins[i];
      u32[i] = pin.inputValue ? 1 : 0;
    }
    u32[40] = (gpio.strapValue ?? 0) >>> 0;
    try {
      this.exports.native_gpio_seed();
    } catch {
    }
  }
  // Native DPORT bridge helper: route an in-page offset to the JS interrupt
  // matrix (matrix0: STATUS0-2 at 236/240/244 + maps 260..536; matrix1:
  // STATUS0-2 at 248/252/256 + maps 536..812 = 536 + 4*MAX_INT(69)).
  // Rust InterruptMatrixPeripheral instances own the matrix state natively —
  // no JS matrix (was DportPeripheral.intMatrix, removed with the FFI round-trip).
};
var WasmCore = class {
  constructor(loader, index) {
    this._loader = loader;
    this._exports = loader.exports;
    this.index = index;
    this.name = `WASM_CORE_${index}`;
    this.debugState = {};
    const sab = loader.memory.buffer;
    const baseOff = index * CORE_STATE_SIZE;
    this._physRegs = new Uint32Array(sab, baseOff + CORE_OFF_PHYS_REGS, 64);
    this._specRegs = new Uint32Array(sab, baseOff + CORE_OFF_SPECIAL_REGS, 256);
    this._pcView = new Uint32Array(sab, baseOff + CORE_OFF_PC, 1);
    this._nextPcView = new Uint32Array(sab, baseOff + CORE_OFF_NEXT_PC, 1);
    this._enabledView = new Uint32Array(sab, baseOff + CORE_OFF_ENABLED, 1);
    this._idleView = new Uint32Array(sab, baseOff + CORE_OFF_IDLE, 1);
    this._lightSleepView = new Uint32Array(sab, baseOff + CORE_OFF_LIGHT_SLEEP, 1);
    this._pendingIntView = new Uint32Array(sab, baseOff + CORE_OFF_PENDING_INT, 1);
    this._opcodeSegmentView = new Uint32Array(sab, baseOff + CORE_OFF_OPCODE_SEG, 1);
  }
  get enabled() {
    return this._enabledView[0] !== 0;
  }
  set enabled(v) {
    this._enabledView[0] = v ? 1 : 0;
  }
  get idle() {
    return this._idleView[0] !== 0;
  }
  set idle(v) {
    this._idleView[0] = v ? 1 : 0;
  }
  get pendingInterrupts() {
    return this._pendingIntView[0];
  }
  set pendingInterrupts(v) {
    this._pendingIntView[0] = v;
  }
  init(processorId, arch) {
    this._exports.core_init(this.index, processorId, arch);
  }
  reset() {
    this._exports.core_reset(this.index);
  }
  runInstruction() {
    if (!this._enabledView[0]) return;
    this._exports.core_step(this.index);
  }
  readUint8(addr) {
    return this._exports.core_read_uint8(this.index, addr);
  }
  readUint16(addr) {
    return this._exports.core_read_uint16(this.index, addr);
  }
  readUint32(addr) {
    return this._exports.core_read_uint32(this.index, addr);
  }
  writeUint8(addr, val) {
    this._exports.core_write_uint8(this.index, addr, val);
  }
  writeUint16(addr, val) {
    this._exports.core_write_uint16(this.index, addr, val);
  }
  writeUint32(addr, val) {
    this._exports.core_write_uint32(this.index, addr, val);
  }
  get PC() {
    return this._pcView[0];
  }
  set PC(v) {
    this._pcView[0] = v;
  }
  get nextPC() {
    return this._nextPcView[0];
  }
  set nextPC(v) {
    this._nextPcView[0] = v;
  }
  // Windowed AR register access (matches Rust CoreState::ar())
  AR(reg) {
    const base = this._specRegs[MEM_FAULT_INFO] << 2;
    return this._physRegs[(base + reg) % 64];
  }
  setAR(reg, val) {
    const base = this._specRegs[MEM_FAULT_INFO] << 2;
    this._physRegs[(base + reg) % 64] = val;
  }
  // BR registers (match state.rs br()/set_br())
  BR(reg) {
    return !!(this._specRegs[4] & 1 << reg);
  }
  setBR(reg, val) {
    if (val) this._specRegs[4] |= 1 << reg;
    else this._specRegs[4] &= ~(1 << reg);
  }
  get specialRegisters() {
    return this._specRegs;
  }
  set specialRegisters(_v) {
  }
  get config() {
    return {};
  }
  unimplemented(opcode) {
    console.warn(`WASM core ${this.index}: unimplemented ${opcode.toString(16)}`);
  }
  exception(cause) {
    console.warn(`WASM core ${this.index}: exception ${cause}`);
  }
  windowCheck(_a, _b, _c) {
    return false;
  }
  setPSExcm(_v) {
  }
  get PS_CRING() {
    return 0;
  }
  setFloatFlags(_flags) {
  }
  get nextInterrupt() {
    return 0;
  }
  set nextInterrupt(_v) {
  }
  get nextInterruptPri() {
    return 0;
  }
  set nextInterruptPri(_v) {
  }
  attachMemorySystem(_pt, _mmio, _regions, _cbs) {
  }
};

// src/peripherals/esp32/esp32.js
init_wasm_memory_layout();
var GpioBothDir = 3;
var InterruptEnum = {
  MAC_INTR: 0,
  MAC_NMI: 1,
  BB_INT: 2,
  BT_MAC_INT: 3,
  BT_BB_INT: 4,
  BT_BB_NMI: 5,
  RWBT_IRQ: 6,
  RWBLE_IRQ: 7,
  RWBT_NMI: 8,
  RWBLE_NMI: 9,
  SLC0_INTR: 10,
  SLC1_INTR: 11,
  UHCI0_INTR: 12,
  UHCI1_INTR: 13,
  TG_T0_LEVEL_INT: 14,
  TG_T1_LEVEL_INT: 15,
  TG_WDT_LEVEL_INT: 16,
  TG_LACT_LEVEL_INT: 17,
  TG1_T0_LEVEL_INT: 18,
  TG1_T1_LEVEL_INT: 19,
  TG1_WDT_LEVEL_INT: 20,
  TG1_LACT_LEVEL_INT: 21,
  GPIO_INTERRUPT_PRO: 22,
  GPIO_INTERRUPT_PRO_NMI: 23,
  CPU_INTR_FROM_CPU_0: 24,
  CPU_INTR_FROM_CPU_1: 25,
  CPU_INTR_FROM_CPU_2: 26,
  CPU_INTR_FROM_CPU_3: 27,
  SPI_INTR_0: 28,
  SPI_INTR_1: 29,
  SPI_INTR_2: 30,
  SPI_INTR_3: 31,
  I2S0_INT: 32,
  I2S1_INT: 33,
  UART_INTR: 34,
  UART1_INTR: 35,
  UART2_INTR: 36,
  SDIO_HOST_INTERRUPT: 37,
  EMAC_INT: 38,
  PWM0_INTR: 39,
  PWM1_INTR: 40,
  PWM2_INTR: 41,
  PWM3_INTR: 42,
  LEDC_INT: 43,
  EFUSE_INT: 44,
  CAN_INT: 45,
  RTC_CORE_INTR: 46,
  RMT_INTR: 47,
  PCNT_INTR: 48,
  I2C_EXT0_INTR: 49,
  I2C_EXT1_INTR: 50,
  RSA_INTR: 51,
  SPI1_DMA_INT: 52,
  SPI2_DMA_INT: 53,
  SPI3_DMA_INT: 54,
  WDG_INT: 55,
  TIMER_INT1: 56,
  TIMER_INT2: 57,
  TG_T0_EDGE_INT: 58,
  TG_T1_EDGE_INT: 59,
  TG_WDT_EDGE_INT: 60,
  TG_LACT_EDGE_INT: 61,
  TG1_T0_EDGE_INT: 62,
  TG1_T1_EDGE_INT: 63,
  TG1_WDT_EDGE_INT: 64,
  TG1_LACT_EDGE_INT: 65,
  MMU_IA_INT: 66,
  MPU_IA_INT: 67,
  CACHE_IA_INT: 68,
  MAX_INT: 69
};
var _debugLog = false;
function _parseSizeMB(cfg, key, alt, def) {
  const v = cfg[key] !== void 0 ? parseInt(cfg[key], 10) : cfg[alt];
  return v !== void 0 && !isNaN(v) ? v : def;
}
var ESP32 = class {
  constructor(config = {}) {
    config = applyBoardPreset(config);
    this.config = config, this.chipName = "esp32", this.chipId = 0, // BT shim opt-in: the BT task/queue intervention (bt_shim_step hook
    // scan) costs ~3s/step on non-BT firmware, so wasm-loader disables it
    // unless the firmware actually uses BT/BLE. BT tests opt in via
    // config.bleShim=true (forwarded through worker init config).
    this.bleShimEnabled = config.bleShim === true, this.board = config.board || "esp32", this._flashSizeMB = _parseSizeMB(config, "flashSize", "flashSizeMB", 4), this._psramSizeMB = _parseSizeMB(config, "psramSize", "psramSizeMB", 4), this._psramType = config.psramType || "quad", this._firmwareOffset = parseFirmwareOffset(config.firmwareOffset), this._macAddress = parseMacAddress(config.macAddress), this.wifiMacState = new Uint8Array(new SharedArrayBuffer(4096)), this.flash = new Uint8Array(
      new SharedArrayBuffer(this._flashSizeMB * Drom0Size)
    ), this.chipROM = new ReadonlyMemory(
      new Uint8Array(new SharedArrayBuffer(Iram0Size)),
      RegionCodeBase
    ), this.rom1 = this.chipROM.createView(RegionDram1Base, 393216, 65536), this.iram = new Memory(
      new Uint8Array(new SharedArrayBuffer(RegionIrom0Base - RegionFlashCacheBase)),
      RegionFlashCacheBase
    ), this.psram = new Uint8Array(
      new SharedArrayBuffer(this._psramSizeMB * Drom0Size)
    ), this.psramMemory = new MMUMemory(this.psram, RegionDrom0Base), this.dataMem = new Memory(
      new Uint8Array(new SharedArrayBuffer(Iram1Size + Drom0CacheSize)),
      RegionRtcSlowBase
    ), this.sram1Reverse = new ReverseMemory(
      this.dataMem.data.subarray(Iram1Size),
      RegionIrom0BaseAlt
    ), this.rtcFastMem = new Memory(
      new Uint8Array(new SharedArrayBuffer(RegionIram1BaseAlt - RegionIram0Base)),
      RegionIram0Base
    ), this.rtcSlowMem = new Memory(
      new Uint8Array(new SharedArrayBuffer(RtcSlowSize)),
      UsbOtgBaseAddr
    ), this.invalidMem = new InvalidMemory(), this.flashMMUMap = /* @__PURE__ */ new Map(), this.mmuTableMemory = new Memory(
      new Uint8Array(new SharedArrayBuffer(2 * RegionDromSize)),
      RegionPeri1Base
    ), this.mmuTablePro = new Uint32Array(
      this.mmuTableMemory.data.buffer,
      0,
      RegionCacheLineSize
    ), this.mmuTableApp = new Uint32Array(
      this.mmuTableMemory.data.buffer,
      RegionDromSize
    ), this.dmaBase = 1072693248, this.peripheralMap = {}, this.gpio = (() => {
      const inputValues = [0, 0];
      const pins = Array.from({ length: 40 }, (_, pinNum) => {
        const bank = pinNum < 32 ? 0 : 1;
        const idx = pinNum % 32;
        return {
          get inputValue() {
            return !!(inputValues[bank] & 1 << idx);
          },
          set inputValue(newLevel) {
            const bit = 1 << idx;
            if (!!(inputValues[bank] & bit) === !!newLevel) return;
            newLevel ? inputValues[bank] |= bit : inputValues[bank] &= ~bit;
          }
        };
      });
      return {
        strapValue: 19,
        inputValues,
        pins,
        zeroMemory: () => {
        },
        reset: () => {
        }
      };
    })(), this.resetReason = 1, this.onReset = () => true, this.onAnalogRead = (pin, cfg) => {
      const v = this.analogVolts?.[pin];
      if (v === void 0) return 0;
      const att = Math.max(0, Math.min(3, (cfg ?? 12) - 9));
      const fullScale = [1.1, 1.34, 2, 3.3][att];
      return Math.max(0, Math.min(4095, Math.round(Math.min(v, fullScale) / fullScale * 4095)));
    }, this.analogVolts = { ...config.analogInputs || {} }, this.setAnalogInput = (pin, volts) => {
      this.analogVolts[pin] = volts;
    }, this.vddMv = config.vddMv ?? 3300, // Touch pad readings (pad 0-9 -> raw count; untouched ~1000+, touched
    // reads low). config.touchInputs overrides per pad; default 1000.
    this.touchCounts = { ...config.touchInputs || {} }, this.setTouchInput = (pad, count) => {
      this.touchCounts[pad] = count;
    }, this.onTouchRead = (pad) => {
      const v = this.touchCounts?.[pad];
      return v === void 0 ? 1e3 : v >>> 0;
    }, // DAC output stage: dacWrite() in firmware lands here via the native
    // SENS DAC_CTRL2 handler (virtual wire DAC ch0/ch1 -> GPIO25/26, which
    // are ADC2 ch8/ch9, so analogRead(25/26) observes the DAC voltage).
    this.onDacWrite = (channel, value) => {
      const pin = channel ? 26 : 25;
      this.analogVolts[pin] = (value & 255) / 255 * 3.3;
    }, // Virtual SD card image (host-owned block storage for the native SDMMC
    // virtual card). config.sdCard: { sizeMB?: number, blocks?: number }.
    // Default 16MB (32768 x 512B) zero-filled; survives chip.reset() like
    // flash (reset only returns the virtual card to idle, storage kept).
    // SAB-backed so the host can save/reload the image between runs
    // (shared via getMemorySABs as "sdcard", like flash).
    this.sdData = (() => {
      const opt = config.sdCard === true ? {} : config.sdCard || null;
      if (!opt && config.sdCard !== void 0 && config.sdCard !== true) return null;
      const blocks = opt?.blocks ?? Math.max(1024, Math.round((opt?.sizeMB ?? 16) * 1024 * 1024 / 512));
      const arr = new Uint8Array(new SharedArrayBuffer(blocks * 512));
      if (opt?.image instanceof Uint8Array) arr.set(opt.image.subarray(0, arr.length));
      return arr;
    })(), this.setSdPresent = (present) => {
      this.sdPresent = !!present;
    }, this.sdPresent = config.sdCard === false ? false : true, // Dev-board buttons (real-device behavior):
    // - RESET (EN pin): momentary reset — pressResetButton() runs a full
    //   chip.reset() (same as the EN pin pulling CHIP_PU low: digital +
    //   RTC state cleared, flash/RTC-memory preserved like real HW).
    // - BOOT (GPIO0 strapping): pressBootButton(held) drives GPIO0's input
    //   level + the BOOT strap bit (bit4 of the 0x13 strap value) so a
    //   subsequent reset samples download mode exactly like holding BOOT
    //   on a real dev board. holdBootAndReset() = BOOT held + RESET tap.
    this.pressResetButton = () => {
      this.reset();
    }, this.pressBootButton = (held) => {
      const level = held ? 1 : 0;
      try {
        this._wasmLoader?.exports?.native_gpio_set_pin_input?.(0, level);
      } catch {
      }
      const holder = this.gpio?.pins?.[0];
      if (holder) {
        try {
          holder.inputValue = level ? 1 : 0;
        } catch {
        }
      }
      if (this.gpio) {
        this.gpio.strapValue = held ? this.gpio.strapValue | 16 : this.gpio.strapValue & ~16;
        try {
          this._wasmLoader?.exports?.native_gpio_set_strap?.(this.gpio.strapValue >>> 0);
        } catch {
        }
      }
    }, this.holdBootAndReset = () => {
      this.pressBootButton(true);
      this.reset();
    }, this.onRandomRead = randomUint32, this._nativeFrequency = 16e7, this.clocks = new ClockTree(new ChipRootClock(this)), this.cycles = 0, this.xts = new XtsEncryptionState(), this.cores = [
      new XtensaCore(this, 0, "PRO_CPU", 52685, XtensaRegisterTable),
      new XtensaCore(this, 1, "APP_CPU", 43947, XtensaRegisterTable)
    ], this.trace = new ESPTrace(), this.gdbTargetXml = "", this.stopped = true, this.writeWatchPoints = /* @__PURE__ */ new Set(), this.lastMappedAddress = 0, this.flash.fill(255), config.partitions && writePartitionTable(this.flash, config.partitions);
    const uartConfig = {
      hasTXState: true,
      toutMultiply: true,
      RmtChannelRegister: UartRegisterMap,
      F: UartFieldMap
    };
    this.uart = [
      new UartController(
        this,
        SdmmcAltBaseAddr,
        "UART0",
        0,
        InterruptEnum.UART_INTR,
        uartConfig
      ),
      new UartController(
        this,
        UhciBaseAddr,
        "UART1",
        1,
        InterruptEnum.UART1_INTR,
        uartConfig
      ),
      new UartController(
        this,
        UhciAltBaseAddr,
        "UART2",
        2,
        InterruptEnum.UART2_INTR,
        uartConfig
      )
    ];
    for (const addr of (this.peripherals = [
      this.gpio,
      ...this.uart
    ], this.peripherals)) {
      this.peripheralMap[addr.baseAddr] = addr;
      const value = addr.baseAddr + RegionDrom0MapBase;
      value >= RegionPeriBusBase && value < RegionUsbBase && (this.peripheralMap[value] = new MemoryTranslator(
        addr,
        RegionDrom0MapBase
      ));
    }
    for (const {
      start: addr,
      pages: value,
      index: regIdx
    } of MmuPageTableConfig)
      for (let i = 0; i < value; i++)
        this.flashMMUMap.set(
          (addr >>> RegionCacheAlignSize) + i,
          regIdx + i
        );
    this.pageTable = new PageTable();
    this.mmioHandlers = new MMIOHandlerRegistry();
    this.buildPageTable();
    {
      const callbacks = {
        traceEntry: (i, pc, op) => this.trace.traceEntry(i, pc, op),
        traceReturn: (i, v, t, idx) => this.trace.traceReturn(i, v, t, idx),
        traceMemWrite: (i, pc, a, v, s) => this.trace.traceMemWrite(i, pc, a, v, s),
        writeWatchpoint: (a, c) => {
          this.writeWatchPoints.has(a) && this.onBreak?.(c);
        },
        onBreak: (c) => this.onBreak?.(c),
        onUnknownInst: (i, pc, op) => this.onUnknownInstruction?.(i, pc, op),
        getCpuTicks: () => this.clocks.cpu.ticks,
        getCpuCycles: () => this.cycles,
        getSimFreq: () => 16e7,
        getCpuFreq: () => this.clocks.cpu.frequency,
        getClockNanos: () => this.cycles / 16e7 * 1e9,
        isValidCodeAddress: (a) => this.isValidCodeAddress?.(a)
      };
      this._coreCallbacks = callbacks;
      for (const core of this.cores) core.attachMemorySystem(this.pageTable, this.mmioHandlers, this._memRegions, callbacks);
    }
    this.reset();
    if (this._macAddress && this.wifiMacState) {
      const mac = this._macAddress;
      this.wifiMacState[64] = mac[0];
      this.wifiMacState[65] = mac[1];
      this.wifiMacState[66] = mac[2];
      this.wifiMacState[67] = mac[3];
      this.wifiMacState[68] = mac[4];
      this.wifiMacState[69] = mac[5];
    }
  }
  loadROM(addr) {
    if (!(addr instanceof Uint8Array) || addr.length === 0) {
      throw new Error(
        `Invalid boot ROM: expected a non-empty Uint8Array (the ESP32 boot ROM, e.g. esp32-v3-rom.bin). Got ${addr === null ? "null" : addr && addr.length !== void 0 ? "an empty array" : typeof addr}.
Provide it via config.bootrom or chip.loadROM(romBytes).`
      );
    }
    this.chipROM.set(addr, RegionCodeBase);
  }
  // Create shared WASM memory and remap JS regions into it
  // All RAM/ROM regions become subarray views of one SAB that WASM imports as linear memory
  _initWasmMemory() {
    const regions = this._memRegions;
    const offsets = [];
    let cum = 0;
    for (const r of regions) {
      offsets.push(cum);
      cum += r.data.length;
    }
    this._memRegionOffsets = offsets;
    const ramBytes = cum;
    const flashBytes = this.flash?.length ?? 0;
    const neededBytes = Math.max(RAM_DATA_OFFSET + ramBytes, FLASH_DATA_OFFSET + flashBytes);
    const pages = Math.ceil(neededBytes / 65536);
    this._wasmMemoryObj = new WebAssembly.Memory({ initial: pages, maximum: pages, shared: true });
    this._wasmMemory = this._wasmMemoryObj.buffer;
    const dst = new Uint8Array(this._wasmMemory);
    for (let i = 0; i < regions.length; i++) {
      const off = RAM_DATA_OFFSET + offsets[i];
      dst.set(regions[i].data, off);
      regions[i].data = new Uint8Array(this._wasmMemory, off, regions[i].data.length);
    }
    if (flashBytes) {
      dst.set(this.flash, FLASH_DATA_OFFSET);
      this._flashMirror = new Uint8Array(this._wasmMemory, FLASH_DATA_OFFSET, flashBytes);
    }
    const u32 = new Uint32Array(this._wasmMemory);
    const rtBase = REGION_TABLE_OFFSET >>> 2;
    for (let i = 0; i < regions.length; i++) {
      u32[rtBase + i * 2] = regions[i].baseAddr;
      u32[rtBase + i * 2 + 1] = offsets[i];
    }
    const oldPt = this.pageTable.table;
    this.pageTable.table = new Uint32Array(this._wasmMemory, PAGE_TABLE_OFFSET, PAGE_ENTRIES2 * 2);
    this.pageTable.table.set(oldPt);
    this.rom1 = this.chipROM.createView(RegionDram1Base, 393216, 65536);
    this.sram1Reverse = new ReverseMemory(this.dataMem.data.subarray(Iram1Size), RegionIrom0BaseAlt);
    this.mmuTablePro = new Uint32Array(this.mmuTableMemory.data.buffer, this.mmuTableMemory.data.byteOffset, RegionCacheLineSize);
    this.mmuTableApp = new Uint32Array(this.mmuTableMemory.data.buffer, this.mmuTableMemory.data.byteOffset + RegionDromSize, RegionCacheLineSize);
  }
  // Load WASM engine. Memory is shared via SAB.
  async loadWasm(wasmBytes, enabled = "wasm") {
    try {
      if (!this._wasmMemory) this._initWasmMemory();
      const loader = new WasmLoader();
      await loader.load(wasmBytes, this, this._wasmMemoryObj);
      this._wasmCores = loader.createCores(this.cores.length);
      for (let i = 0; i < this.cores.length; i++) {
        this.cores[i].specialRegisters = this._wasmCores[i]._specRegs;
        this.cores[i].physicalRegisters = this._wasmCores[i]._physRegs;
        Object.defineProperty(this.cores[i], "pendingInterrupts", {
          get: () => this._wasmCores[i].pendingInterrupts,
          set: (v) => {
            this._wasmCores[i]._pendingIntView[0] = v ? 1 : 0;
          },
          configurable: true
        });
        Object.defineProperty(this.cores[i], "enabled", {
          get: () => this._wasmCores[i].enabled,
          set: (v) => {
            this._wasmCores[i]._enabledView[0] = v ? 1 : 0;
          },
          configurable: true
        });
        Object.defineProperty(this.cores[i], "idle", {
          get: () => this._wasmCores[i].idle,
          set: (v) => {
            this._wasmCores[i]._idleView[0] = v ? 1 : 0;
          },
          configurable: true
        });
        Object.defineProperty(this.cores[i], "PC", {
          get: () => this._wasmCores[i]._pcView[0],
          set: (v) => {
            this._wasmCores[i]._pcView[0] = v >>> 0;
          },
          configurable: true
        });
        Object.defineProperty(this.cores[i], "nextPC", {
          get: () => this._wasmCores[i]._nextPcView[0],
          set: (v) => {
            this._wasmCores[i]._nextPcView[0] = v >>> 0;
          },
          configurable: true
        });
        Object.defineProperty(this.cores[i], "lightSleep", {
          get: () => this._wasmCores[i]._lightSleepView[0] !== 0,
          set: (v) => {
            this._wasmCores[i]._lightSleepView[0] = v ? 1 : 0;
          },
          configurable: true
        });
        Object.defineProperty(this.cores[i], "opcodeSegment", {
          get: () => this._wasmCores[i]._opcodeSegmentView[0],
          set: (v) => {
            this._wasmCores[i]._opcodeSegmentView[0] = v >>> 0;
          },
          configurable: true
        });
      }
      this._wasmLoader = loader;
      this._nativeUartReset = () => {
        try {
          loader.exports?.native_uart_reset?.();
        } catch {
        }
      };
      this.setVddMv = (mv) => {
        this.vddMv = mv >>> 0;
        try {
          loader.exports?.native_bod_set_voltage_mv?.(this.vddMv);
        } catch {
        }
      };
      this._nativeI2cReset = () => {
        try {
          loader.exports?.native_i2c_reset?.();
        } catch {
        }
      };
      this._nativeSpiReset = () => {
        try {
          loader.exports?.native_spi_reset?.();
        } catch {
        }
      };
      this._nativeGpioReset = () => {
        try {
          loader.exports?.native_gpio_reset?.();
        } catch {
        }
      };
      this._nativeFrcReset = () => {
        try {
          loader.exports?.native_frc_timer_reset?.();
        } catch {
        }
      };
      this._nativeTimg1Reset = () => {
        try {
          loader.exports?.native_timg1_reset?.();
        } catch {
        }
      };
      this._nativeEfuseReset = () => {
        try {
          loader.exports?.native_efuse_reset?.();
        } catch {
        }
      };
      this._nativeSysconReset = () => {
        try {
          loader.exports?.native_syscon_reset?.();
        } catch {
        }
      };
      this._nativeTwaiReset = () => {
        try {
          loader.exports?.native_twai_reset?.();
        } catch {
        }
      };
      this._nativeRsaReset = () => {
        try {
          loader.exports?.native_rsa_reset?.();
        } catch {
        }
      };
      this._nativeRtcReset = () => {
        try {
          loader.exports?.native_rtc_reset?.();
        } catch {
        }
      };
      this._nativeLedcReset = () => {
        try {
          loader.exports?.native_ledc_reset?.();
        } catch {
        }
      };
      this._nativePcntReset = () => {
        try {
          loader.exports?.native_pcnt_reset?.();
        } catch {
        }
      };
      this._nativeRmtReset = () => {
        try {
          loader.exports?.native_rmt_reset?.();
        } catch {
        }
      };
      this._nativeI2sReset = () => {
        try {
          loader.exports?.native_i2s_reset?.();
        } catch {
        }
      };
      this._nativeSdmmcReset = () => {
        try {
          loader.exports?.native_sdmmc_reset?.();
        } catch {
        }
      };
      this._nativeWifiAnalogReset = () => {
        try {
          loader.exports?.native_wifi_analog_reset?.();
        } catch {
        }
      };
      this._nativeWifiMacReset = () => {
        try {
          loader.exports?.native_wifi_mac_reset?.();
        } catch {
        }
      };
      this._nativeDportReset = () => {
        try {
          loader.exports?.native_dport_reset?.();
        } catch {
        }
      };
      this._nativeSdioSlaveReset = () => {
        try {
          loader.exports?.native_sdio_slave_reset?.();
        } catch {
        }
      };
      this._nativeFeReset = () => {
        try {
          loader.exports?.native_fe_reset?.();
        } catch {
        }
      };
      this._nativeMcpwmReset = () => {
        try {
          loader.exports?.native_mcpwm_reset?.();
        } catch {
        }
      };
      this._nativeUhciReset = () => {
        try {
          loader.exports?.native_uhci_reset?.();
        } catch {
        }
      };
      this._nativeEmacReset = () => {
        try {
          loader.exports?.native_emac_reset?.();
        } catch {
        }
      };
      this._nativeSweepReset = () => {
        try {
          loader.exports?.native_sweep_reset?.();
        } catch {
        }
      };
      this._nativeBtRfReset = () => {
        try {
          loader.exports?.native_bt_rf_reset?.();
        } catch {
        }
      };
      console.log(`WASM engine loaded (${this._wasmCores.length} cores). Mode: ${enabled}`);
      return true;
    } catch (err) {
      throw new Error(
        `Failed to load the WASM engine. Underlying error: ${err.message}
Check that esp_engine_wasm.wasm is present at src/engine/esp-xtensa/ and is a valid WebAssembly module built for wasm32-unknown-unknown. Rebuild with:
  cd engine-wasm && cargo build --release --target wasm32-unknown-unknown && cp target/wasm32-unknown-unknown/release/esp_engine_wasm.wasm ../src/engine/esp-xtensa/`
      );
    }
  }
  reset() {
    this._stepExp = void 0;
    for (let e of this.cores) e.reset();
    if (this._wasmCores) {
      for (let wc of this._wasmCores) wc.reset();
    }
    for (let e of (this.cores[1].enabled = (this._wasmLoader?.exports?.native_dport_get_core1_enabled?.() ?? 1) !== 0, applyPeripheralResetValues(this, Esp32FullResetValues), this.cores[0].writeUint32(1072992256, 1023), this.mmuTablePro.fill(256), this.mmuTableApp.fill(256), this.peripherals))
      e.reset();
    if (this._nativeUartReset) this._nativeUartReset();
    if (this._nativeI2cReset) this._nativeI2cReset();
    if (this._nativeSpiReset) this._nativeSpiReset();
    if (this._nativeGpioReset) this._nativeGpioReset();
    if (this._nativeFrcReset) this._nativeFrcReset();
    if (this._nativeTimg1Reset) this._nativeTimg1Reset();
    if (this._nativeEfuseReset) this._nativeEfuseReset();
    if (this._nativeSysconReset) this._nativeSysconReset();
    if (this._nativeTwaiReset) this._nativeTwaiReset();
    if (this._nativeRsaReset) this._nativeRsaReset();
    if (this._nativeRtcReset) this._nativeRtcReset();
    if (this._nativeLedcReset) this._nativeLedcReset();
    if (this._nativePcntReset) this._nativePcntReset();
    if (this._nativeRmtReset) this._nativeRmtReset();
    if (this._nativeI2sReset) this._nativeI2sReset();
    if (this._nativeSdmmcReset) this._nativeSdmmcReset();
    if (this._nativeWifiAnalogReset) this._nativeWifiAnalogReset();
    if (this._nativeWifiMacReset) this._nativeWifiMacReset();
    if (this._nativeDportReset) this._nativeDportReset();
    if (this._nativeSdioSlaveReset) this._nativeSdioSlaveReset();
    if (this._nativeFeReset) this._nativeFeReset();
    if (this._nativeMcpwmReset) this._nativeMcpwmReset();
    if (this._nativeUhciReset) this._nativeUhciReset();
    if (this._nativeEmacReset) this._nativeEmacReset();
    if (this._nativeSweepReset) this._nativeSweepReset();
    if (this._nativeBtRfReset) this._nativeBtRfReset();
  }
  resetPeripheral(addr, value) {
    if (!value) return;
    let regIdx = this.peripherals.find((value2) => value2.baseAddr === addr);
    if (regIdx) { applySingleResetValues(regIdx, Esp32FullResetValues); regIdx.reset(); }
  }
  mapAddress(addr, value) {
    if (this.lastMappedAddress = addr, addr >= 1073397760 && addr <= 1073405951 && console.error("SRAM 2 mmu access:", addr.toString(16)), addr >= GpioBaseAddrAlt && addr < RegionDram1Base || addr >= RegionPeriBusBase && addr <= RegionUsbBase) {
      let value2 = this.peripheralMap[4294966272 & addr], regIdx2 = this.peripheralMap[4294963200 & addr];
      if (value2 || regIdx2) return value2 || regIdx2;
    }
    if (addr >= RegionRtcSlowBase && addr < RegionCodeBase)
      return this.dataMem;
    if (addr >= RegionDram1Base && addr < RegionRtcSlowBase)
      return this.rom1;
    if (addr >= RegionCodeBase && addr < RegionCodeBase + Iram0Size)
      return this.chipROM;
    if (addr >= RegionFlashCacheBase && addr < RegionIrom0Base)
      return this.iram;
    if (addr >= RegionIram0Base && addr < RegionIram1BaseAlt)
      return this.rtcFastMem;
    if (addr >= RegionDrom0Base && addr < RegionDrom1Base) {
      let addr2 = value ? this.mmuTableApp : this.mmuTablePro;
      return this.psramMemory.setTable(addr2, 1152), this.psramMemory;
    }
    if (addr >= RegionIrom0BaseAlt && addr <= RegionIram1Base)
      return this.sram1Reverse;
    let regIdx = this.flashMMUMap.get(addr >>> RegionCacheAlignSize);
    if (null != regIdx) {
      let phyAddr = (value ? this.mmuTableApp : this.mmuTablePro)[regIdx];
      if (256 & phyAddr) return this.invalidMem;
      let phyAddrAligned = phyAddr << RegionCacheAlignSize;
      return new ReadonlyMemory(
        this.flash.subarray(phyAddrAligned),
        addr >>> RegionCacheAlignSize << RegionCacheAlignSize
      );
    }
    return addr >= RegionPeri1Base && addr < RegionPeri2Base ? this.mmuTableMemory : addr >= UsbOtgBaseAddr && addr < UsbOtgBaseAddr + RtcSlowSize ? this.rtcSlowMem : (_debugLog && console.log("Invalid memory access", addr.toString(16)), this.invalidMem);
  }
  step() {
    if (this._wasmCores) {
      let exp = this._stepExp;
      if (exp === void 0) {
        exp = this._wasmLoader?.exports || null;
        this._stepExp = exp;
      }
      if (exp?.core_run) {
        exp.core_run(512);
        this.cycles += 512;
      } else {
        this._wasmCores[0].runInstruction();
        if (this.cores[1]?.enabled) this._wasmCores[1].runInstruction();
        this.cycles++;
      }
    } else {
      this.cycles++;
    }
    if (this._wasmCores && !this._sabU32) {
      this._sabU32 = new Uint32Array(this._wasmMemory || this._wasmMemoryObj?.buffer);
    }
    if (this._sabU32) {
      this._sabU32[998] = this.clocks.cpu.ticks >>> 0;
      this._sabU32[999] = this.cycles >>> 0;
    }
  }
  // MMU-table entry governing `addr` (or undefined when not MMU-routed).
  // Used by wasm-loader's map_read/map_write page cache to detect remaps —
  // mirrors the flashMMUMap + psramMemory branches of mapAddress().
  mmuEntryFor(addr, value) {
    if (addr >= RegionDrom0Base && addr < RegionDrom1Base) {
      return (value ? this.mmuTableApp : this.mmuTablePro)[1152 + (addr - RegionDrom0Base >>> 15)];
    }
    let regIdx = this.flashMMUMap.get(addr >>> RegionCacheAlignSize);
    if (regIdx === void 0) return void 0;
    return (value ? this.mmuTableApp : this.mmuTablePro)[regIdx];
  }
  get coresIdle() {
    let { cores: addr } = this;
    return addr[0].idle && !addr[0].pendingInterrupts && addr[1].idle && !addr[1].pendingInterrupts;
  }
  interrupt(addr, value = true, regIdx = GpioBothDir) {
    this._wasmLoader?.exports?.native_interrupt?.(addr >>> 0, value ? 1 : 0, regIdx >>> 0);
  }
  buildPageTable() {
    const pt = this.pageTable;
    const reg = this.mmioHandlers;
    const sharedPages = /* @__PURE__ */ new Set();
    const pagePerifs = {};
    for (const p of this.peripherals) {
      const page = p.baseAddr >>> 12;
      if (!pagePerifs[page]) pagePerifs[page] = [];
      pagePerifs[page].push(p);
    }
    for (const [page, perifs] of Object.entries(pagePerifs)) {
      const hasAligned = perifs.some((p) => (p.baseAddr & 4095) === 0);
      const hasNonAligned = perifs.some((p) => (p.baseAddr & 4095) !== 0);
      if (hasAligned && hasNonAligned && perifs.length > 1) sharedPages.add(parseInt(page));
    }
    for (const p of this.peripherals) {
      if ((p.baseAddr & 4095) !== 0) continue;
      if (sharedPages.has(p.baseAddr >>> 12)) continue;
      const id = reg.register(p);
      pt.setRange(p.baseAddr, 4096, PTE_TYPE_MMIO, id);
      const alias = p.baseAddr + RegionDrom0MapBase;
      if (alias >= RegionPeriBusBase && alias < RegionUsbBase) {
        const aliasId = reg.register(new MemoryTranslator(p, RegionDrom0MapBase));
        pt.setRange(alias, 4096, PTE_TYPE_MMIO, aliasId);
      }
    }
    this._memRegions = [
      this.dataMem,
      this.chipROM,
      this.rtcFastMem,
      this.rtcSlowMem,
      this.rom1,
      this.mmuTableMemory,
      this.iram
    ];
    for (let i = 0; i < this._memRegions.length; i++) {
      const m = this._memRegions[i];
      pt.setRange(m.baseAddr, m.data.length, PTE_TYPE_RAM, i);
    }
    for (const [page16, regIdx] of this.flashMMUMap) {
      if ((page16 < 16192 || page16 >= 16256) && (page16 < 16396 || page16 >= 16448)) continue;
      for (let i = 0; i < 16; i++) {
        pt.setPage((page16 << 16) + i * 4096, PTE_TYPE_FLASH, regIdx);
      }
    }
    console.log(`[PTEDBG] flashMMUMap.size=${this.flashMMUMap.size} pte0x400C2000=${pt.getType(1074536448)} pte0x40080000=${pt.getType(1074266112)} pte0x3F400000=${pt.getType(1061158912)}`);
  }
  get devices() {
    return {
      nonvolatile: { flash: this.flash },
      volatile: {
        iram: this.iram.data,
        dram: this.dataMem.data,
        rtcFast: this.rtcFastMem.data,
        rtcSlow: this.rtcSlowMem.data,
        mmu: this.mmuTableMemory.data,
        psram: this.psram
      }
    };
  }
};

// src/sab/worker-entry.js
var ENGINE = typeof process !== "undefined" && process.env && process.env.ENGINE || "wasm";
var DEBUG_BOOT = true;
var SAB_RUN = 0;
var SAB_NANOS_LO = 2;
var SAB_STATUS = 3;
var SAB_PC = 4;
var SAB_IDLE = 6;
var SAB_NANOS_HI = 8;
var SAB_CMD = 9;
var SAB_CMD_ARG0 = 10;
var SAB_CMD_ARG1 = 11;
var SAB_RESP = 12;
var SAB_ERR_FLAG = 13;
var SAB_CPU_FREQ = 14;
var SAB_CMD_ARG2 = 21;
var SAB_WIFI_STATE = 15;
var SAB_WIFI_TX_FRAMES = 16;
var SAB_WIFI_TX_BYTES = 17;
var SAB_WIFI_RX_FRAMES = 18;
var SAB_WIFI_RX_BYTES = 19;
var SAB_WIFI_PROBES = 20;
var DBG_PC0 = 0;
var DBG_PC1 = 1;
var DBG_CYCLES = 2;
var DBG_NANOS_LO = 3;
var DBG_NANOS_HI = 4;
var DBG_TICKS = 5;
var DBG_ENABLED0 = 6;
var DBG_ENABLED1 = 7;
var DBG_PHYS_START = 16;
var DBG_SPEC_START = 80;
var DBG_ALL_SPEC_START = 96;
var DBG_MMU_PRO_START = 352;
var DBG_MMU_APP_START = 416;
var DBG_MEM_HASH_START = 480;
var DBG_PERIPH_START = 484;
var DBG_CORE1_SPEC_START = 500;
var DBG_WIFI_START = 756;
function hashMem(data, maxBytes) {
  const len = Math.min(data.length, maxBytes);
  let h = 2166136261;
  let i = 0;
  for (; i + 3 < len; i += 4) {
    const w = data[i] | data[i + 1] << 8 | data[i + 2] << 16 | data[i + 3] << 24;
    h = Math.imul(h ^ w, 16777619) >>> 0;
  }
  for (; i < len; i++) {
    h ^= data[i];
    h = Math.imul(h, 16777619) >>> 0;
  }
  return h;
}
var CMD_NONE = 0;
var CMD_RUN = 1;
var CMD_RESET = 2;
var CMD_SEED_MMU = 3;
var CMD_WRITE_UINT32 = 4;
var CMD_READ_MEMORY = 5;
var CMD_GET_PCAP = 6;
var CMD_GET_WIFI_STATS = 7;
var CMD_STEP = 8;
var CMD_DESTROY = 9;
var CMD_GET_FFI_COUNTS = 10;
var CMD_READ_MMIO = 11;
var CMD_UART_RX = 12;
var CMD_SET_PIN_INPUT = 13;
var CMD_SET_TOUCH_INPUT = 14;
var CMD_SET_VOLTAGE = 15;
var CMD_FEED_I2S_RX = 16;
var CMD_SEND_TWAI = 17;
var CMD_PUSH_TWAI = 18;
var CMD_GET_TWAI_TX = 19;
var CMD_WATCHPOINT = 20;
var CMD_PCTRACE = 21;
var CMD_PRESS_RESET = 22;
var CMD_PRESS_BOOT = 23;
var RESP_DONE = 1;
var RESP_ERROR = 2;
var UART_RING_SIZE = 16384;
var msgPort;
if (typeof self !== "undefined") {
  msgPort = self;
} else {
  const { parentPort } = await import("worker_threads");
  msgPort = parentPort;
}
function send(msg) {
  msgPort.postMessage(msg);
}
var chipConstructors = { ESP32 };
var chip = null;
var ctrl = null;
var uartCtrl = null;
var uartRing = null;
var uartRxCtrl = null;
var uartRxRing = null;
var readResp = null;
var errMsg = null;
var debugData = null;
function getMemorySABs(chip2) {
  const regions = {};
  const extract = (key, obj) => {
    if (!obj) return;
    const arr = obj instanceof Uint8Array ? obj : obj.data;
    if (!arr || !(arr.buffer instanceof SharedArrayBuffer)) return;
    regions[key] = { buffer: arr.buffer, byteOffset: arr.byteOffset, byteLength: arr.byteLength };
  };
  extract("flash", chip2.flash);
  extract("chipROM", chip2.chipROM);
  extract("iram", chip2.iram);
  extract("psram", chip2.psram);
  extract("dataMem", chip2.dataMem);
  extract("rtcFastMem", chip2.rtcFastMem);
  extract("mmuTableMemory", chip2.mmuTableMemory);
  extract("iramCache", chip2.iramCache);
  extract("sram", chip2.sram);
  extract("wifiMac", chip2.wifiMacState);
  extract("sdcard", chip2.sdData);
  return regions;
}
function setError(msg) {
  if (errMsg) {
    const enc = new TextEncoder();
    const encoded = enc.encode(msg);
    const len = Math.min(encoded.length, errMsg.length - 1);
    errMsg.set(encoded.subarray(0, len));
    errMsg[len] = 0;
  }
  if (ctrl) ctrl[SAB_ERR_FLAG] = 1;
  send({ type: "error", message: msg });
}
function nanosMultiplier() {
  if (!chip || !ctrl) return 1;
  const targetFreq = ctrl[SAB_CPU_FREQ];
  if (!targetFreq || targetFreq <= 0) return 1;
  const native = chip._nativeFrequency || 16e7;
  return targetFreq >= native ? 1 : native / targetFreq;
}
var _dbgUserCodeNotified = false;
var _writeSABCount = 0;
function writeSABState() {
  if (!ctrl) return;
  const n = chip ? chip.cycles / 16e7 * 1e9 : 0;
  const scaled = n * nanosMultiplier();
  ctrl[SAB_NANOS_LO] = scaled | 0;
  ctrl[SAB_NANOS_HI] = scaled / 4294967296 | 0;
  if (chip._wasmCores?.[0]) {
    ctrl[SAB_PC] = chip._wasmCores[0].PC;
  } else {
    ctrl[SAB_PC] = chip?.cores?.[0]?.PC ?? -1;
  }
  ctrl[SAB_IDLE] = chip?.coresIdle ? 1 : 0;
  if (DEBUG_BOOT && !_dbgUserCodeNotified && chip) {
    const pc = ctrl[SAB_PC];
    if (pc >= 1074593792 && pc < 1074790400) {
      console.log(`[DBG] USER CODE ENTERED at nanos=${n} PC=0x${pc.toString(16).padStart(8, "0")}`);
      _dbgUserCodeNotified = true;
    } else if (pc >= 1074233344 && pc < 1074270208) {
    } else if (pc > 0 && pc < 1074233344 && pc !== (ctrl[SAB_PC] || 0)) {
    }
  }
  if (debugData && chip) {
    debugData[DBG_PC0] = ctrl[SAB_PC] >>> 0;
    debugData[DBG_PC1] = (chip._wasmCores?.[1]?.PC ?? chip.cores?.[1]?.PC ?? 0) >>> 0;
    debugData[DBG_CYCLES] = chip.cycles >>> 0;
    debugData[DBG_NANOS_LO] = ctrl[SAB_NANOS_LO];
    debugData[DBG_NANOS_HI] = ctrl[SAB_NANOS_HI];
    debugData[DBG_TICKS] = (chip.clocks?.cpu?.ticks ?? 0) >>> 0;
    debugData[DBG_ENABLED0] = chip._wasmCores?.[0] ? chip._wasmCores[0].enabled ? 1 : 0 : chip.cores?.[0]?.enabled ? 1 : 0;
    debugData[DBG_ENABLED1] = chip._wasmCores?.[1] ? chip._wasmCores[1].enabled ? 1 : 0 : chip.cores?.[1]?.enabled ? 1 : 0;
    _writeSABCount++;
    if ((_writeSABCount & 3) === 0) {
      const phys = chip._wasmCores?.[0]?._physRegs ?? chip.cores?.[0]?.physicalRegisters;
      if (phys) {
        for (let i = 0; i < 64 && i < phys.length; i++)
          debugData[DBG_PHYS_START + i] = phys[i] >>> 0;
      }
      const spec = chip._wasmCores?.[0]?._specRegs ?? chip.cores?.[0]?.specialRegisters;
      if (spec) {
        const keySpecs = [72, 226, 228, 230, 231, 232, 233, 234, 235, 240, 241, 242];
        for (let i = 0; i < keySpecs.length; i++)
          if (keySpecs[i] < spec.length)
            debugData[DBG_SPEC_START + i] = spec[keySpecs[i]] >>> 0;
        for (let i = 0; i < spec.length && i < 256; i++)
          debugData[DBG_ALL_SPEC_START + i] = spec[i] >>> 0;
        const canonicalCcount = (chip.clocks?.cpu?.ticks ?? 0) >>> 0;
        debugData[DBG_SPEC_START + 7] = canonicalCcount;
        debugData[DBG_ALL_SPEC_START + 234] = canonicalCcount;
      }
      if (chip.mmuTablePro) {
        for (let i = 0; i < 64; i++)
          debugData[DBG_MMU_PRO_START + i] = chip.mmuTablePro[i] >>> 0;
      }
      if (chip.mmuTableApp) {
        for (let i = 0; i < 64; i++)
          debugData[DBG_MMU_APP_START + i] = chip.mmuTableApp[i] >>> 0;
      }
      const spec1 = chip._wasmCores?.[1]?._specRegs ?? chip.cores?.[1]?.specialRegisters;
      if (spec1) {
        for (let i = 0; i < spec1.length && i < 256; i++)
          debugData[DBG_CORE1_SPEC_START + i] = spec1[i] >>> 0;
      }
      if (chip._wasmLoader?.exports?.map_read) {
        const mr = (addr) => {
          try {
            return chip._wasmLoader.exports.map_read(0, addr, 4) >>> 0;
          } catch (_) {
            return 0;
          }
        };
        debugData[DBG_WIFI_START + 0] = mr(1072693508);
        debugData[DBG_WIFI_START + 1] = mr(1072693784);
        debugData[DBG_WIFI_START + 2] = mr(1072693484);
        debugData[DBG_WIFI_START + 3] = mr(1072693496);
        debugData[DBG_WIFI_START + 4] = mr(17170631752);
        debugData[DBG_WIFI_START + 5] = mr(17170631756);
        debugData[DBG_WIFI_START + 6] = mr(1073164420);
        debugData[DBG_WIFI_START + 7] = mr(1073165508);
      }
      if ((_writeSABCount & 15) === 0) {
        const getData = (mem) => mem instanceof Uint8Array ? mem : mem?.data;
        debugData[DBG_MEM_HASH_START] = getData(chip.iram) ? hashMem(getData(chip.iram), 65536) : 0;
        debugData[DBG_MEM_HASH_START + 1] = getData(chip.dataMem) ? hashMem(getData(chip.dataMem), 65536) : 0;
        debugData[DBG_MEM_HASH_START + 2] = getData(chip.rtcFastMem) ? hashMem(getData(chip.rtcFastMem), 16384) : 0;
        debugData[DBG_MEM_HASH_START + 3] = getData(chip.sram) ? hashMem(getData(chip.sram), 65536) : 0;
      }
      const g = chip.gpio;
      if (g) {
        debugData[DBG_PERIPH_START] = (g.out?.[0] ?? 0) >>> 0;
        debugData[DBG_PERIPH_START + 1] = (g.out?.[1] ?? 0) >>> 0;
        debugData[DBG_PERIPH_START + 2] = (g.enable?.[0] ?? 0) >>> 0;
        debugData[DBG_PERIPH_START + 3] = (g.in?.[0] ?? 0) >>> 0;
        debugData[DBG_PERIPH_START + 4] = (g.status?.[0] ?? 0) >>> 0;
      }
      debugData[DBG_PERIPH_START + 5] = (chip.uart?.[0]?.status ?? 0) >>> 0;
      debugData[DBG_PERIPH_START + 6] = (chip.uart?.[1]?.status ?? 0) >>> 0;
      const tg0 = chip.timerGroup?.[0]?.timers?.[0];
      debugData[DBG_PERIPH_START + 7] = tg0 ? Number(tg0.counterLo) >>> 0 : 0;
      debugData[DBG_PERIPH_START + 8] = tg0 ? Number(tg0.counterHi) >>> 0 : 0;
      debugData[DBG_PERIPH_START + 9] = 0;
    }
  }
  chip?._syncWifiStatus?.();
  const ws = chip?._wifiBridge?.ap?.status;
  if (ws) {
    ctrl[SAB_WIFI_STATE] = ws.state;
    ctrl[SAB_WIFI_TX_FRAMES] = ws.txFrames;
    ctrl[SAB_WIFI_TX_BYTES] = ws.txBytes;
    ctrl[SAB_WIFI_RX_FRAMES] = ws.rxFrames;
    ctrl[SAB_WIFI_RX_BYTES] = ws.rxBytes;
    ctrl[SAB_WIFI_PROBES] = ws.probeRequestCount;
  }
}
var _uartCount = 0;
function uartWriteByte(byte) {
  if (!uartCtrl) {
    if (++_uartCount < 10) console.log(`[UART-WARN] uartCtrl null on byte=${byte}`);
    return;
  }
  const idx = uartCtrl[0] % UART_RING_SIZE;
  uartRing[idx] = byte;
  uartCtrl[0]++;
}
var needsEventLoop = true;
var simChunkSize = 5e5;
var twaiStage = [0, 0, 0];
function finishSim() {
  ctrl[SAB_RUN] = 0;
  ctrl[SAB_STATUS] = 1;
  writeSABState();
}
function syncClockState(exp) {
  const e = exp || chip?._wasmLoader?.exports;
  if (!e?.native_set_clock_state) return;
  e.native_set_clock_state(Number(chip.cycles ?? 0) >>> 0);
}
function runSimChunk() {
  if (!chip || !ctrl || !Atomics.load(ctrl, SAB_RUN)) {
    finishSim();
    setTimeout(() => commandLoop(), 0);
    return;
  }
  const CHUNK = needsEventLoop ? simChunkSize : Infinity;
  const wasmExp = chip._wasmLoader?.exports || null;
  const idleAdvance = wasmExp?.native_idle_advance || null;
  const pumpEvents = wasmExp?.native_pump_events || null;
  const clockRoot = chip.clocks?.root || null;
  const fireDue = clockRoot?.fireDueEvents?.bind(clockRoot) || null;
  const wCores = chip._wasmCores;
  const idleView0 = wCores?.[0]?._idleView || null;
  const idleView1 = wCores?.[1]?._idleView || null;
  const pendingView0 = wCores?.[0]?._pendingIntView || null;
  const pendingView1 = wCores?.[1]?._pendingIntView || null;
  syncClockState(wasmExp);
  try {
    let steps = 0;
    let cycles = chip.cycles;
    const nativeUartFeed = wasmExp?.native_uart_feed || null;
    if (cycles === 0 || isNaN(cycles)) console.log(`[RUNSIM] start cycles=${cycles} isNaN=${isNaN(cycles)}`);
    while (steps < CHUNK) {
      if (uartRxCtrl && uartRxRing) {
        const w = Atomics.load(uartRxCtrl, 0);
        const r = Atomics.load(uartRxCtrl, 1);
        const avail = w - r;
        if (avail > 0) {
          const n = Math.min(avail, 32);
          for (let i = 0; i < n; i++) {
            const byte = uartRxRing[(r + i) % UART_RING_SIZE];
            try {
              if (nativeUartFeed) nativeUartFeed(0, byte);
              else chip.uart[0].feedByte(byte);
            } catch {
            }
          }
          Atomics.store(uartRxCtrl, 1, r + n);
        }
      }
      if (Atomics.load(ctrl, SAB_CMD) === CMD_PCTRACE) {
        try {
          chip?._wasmLoader?.exports?.native_pc_trace?.(Atomics.load(ctrl, SAB_CMD_ARG0) >>> 0);
        } catch {
        }
        Atomics.store(ctrl, SAB_RESP, RESP_DONE);
        Atomics.store(ctrl, SAB_CMD, CMD_NONE);
      }
      if (Atomics.load(ctrl, SAB_CMD) === CMD_WATCHPOINT) {
        try {
          const ex = chip?._wasmLoader?.exports;
          const addr = Atomics.load(ctrl, SAB_CMD_ARG0);
          if (addr >>> 0 === 4294967295) ex?.native_watchpoint_clear?.();
          else ex?.native_watchpoint_add?.(addr >>> 0);
        } catch {
        }
        Atomics.store(ctrl, SAB_RESP, RESP_DONE);
        Atomics.store(ctrl, SAB_CMD, CMD_NONE);
      }
      if (Atomics.load(ctrl, SAB_CMD) === CMD_READ_MEMORY) {
        try {
          const addr = Atomics.load(ctrl, SAB_CMD_ARG0);
          const maxBytes = Atomics.load(ctrl, SAB_CMD_ARG1);
          let written = 0;
          if (typeof readResp !== "undefined" && readResp) {
            const mem = chip?.mapAddress ? chip.mapAddress(addr) : null;
            if (mem && mem.data) {
              const offset = addr - mem.baseAddr;
              const available = Math.min(mem.data.length - offset, readResp.length);
              const len = Math.min(maxBytes || available, available);
              if (len > 0) readResp.set(mem.data.subarray(offset, offset + len), 0);
              written = len;
            }
          }
          Atomics.store(ctrl, SAB_CMD_ARG1, written);
        } catch {
        }
        Atomics.store(ctrl, SAB_RESP, RESP_DONE);
        Atomics.store(ctrl, SAB_CMD, CMD_NONE);
      }
      if (Atomics.load(ctrl, SAB_CMD) === CMD_SET_VOLTAGE) {
        try {
          chip?.setVddMv?.(Atomics.load(ctrl, SAB_CMD_ARG0));
        } catch {
        }
        Atomics.store(ctrl, SAB_RESP, RESP_DONE);
        Atomics.store(ctrl, SAB_CMD, CMD_NONE);
      }
      if (Atomics.load(ctrl, SAB_CMD) === CMD_SET_TOUCH_INPUT) {
        const pad = Atomics.load(ctrl, SAB_CMD_ARG0);
        const count = Atomics.load(ctrl, SAB_CMD_ARG1);
        try {
          chip?.setTouchInput?.(pad, count);
        } catch {
        }
        Atomics.store(ctrl, SAB_RESP, RESP_DONE);
        Atomics.store(ctrl, SAB_CMD, CMD_NONE);
      }
      if (Atomics.load(ctrl, SAB_CMD) === CMD_SET_PIN_INPUT) {
        const pin = Atomics.load(ctrl, SAB_CMD_ARG0);
        const level = Atomics.load(ctrl, SAB_CMD_ARG1);
        try {
          wasmExp?.native_gpio_set_pin_input?.(pin, level);
        } catch {
        }
        const holder = chip?.gpio?.pins?.[pin];
        if (holder) {
          try {
            holder.inputValue = level ? 1 : 0;
          } catch {
          }
        }
        Atomics.store(ctrl, SAB_RESP, RESP_DONE);
        Atomics.store(ctrl, SAB_CMD, CMD_NONE);
      }
      if (Atomics.load(ctrl, SAB_CMD) === CMD_FEED_I2S_RX) {
        const sample = Atomics.load(ctrl, SAB_CMD_ARG0) >>> 0;
        try {
          wasmExp?.native_i2s_push_rx_sample?.(0, sample);
        } catch {
        }
        Atomics.store(ctrl, SAB_RESP, RESP_DONE);
        Atomics.store(ctrl, SAB_CMD, CMD_NONE);
      }
      if (Atomics.load(ctrl, SAB_CMD) === CMD_SEND_TWAI) {
        twaiStage[0] = Atomics.load(ctrl, SAB_CMD_ARG0) >>> 0;
        twaiStage[1] = Atomics.load(ctrl, SAB_CMD_ARG1) >>> 0;
        twaiStage[2] = Atomics.load(ctrl, SAB_CMD_ARG2) >>> 0;
        Atomics.store(ctrl, SAB_RESP, RESP_DONE);
        Atomics.store(ctrl, SAB_CMD, CMD_NONE);
      }
      if (Atomics.load(ctrl, SAB_CMD) === CMD_PUSH_TWAI) {
        try {
          wasmExp?.native_twai_push_rx?.(twaiStage[0], twaiStage[1], twaiStage[2], Atomics.load(ctrl, SAB_CMD_ARG0) >>> 0);
        } catch {
        }
        Atomics.store(ctrl, SAB_RESP, RESP_DONE);
        Atomics.store(ctrl, SAB_CMD, CMD_NONE);
      }
      if (Atomics.load(ctrl, SAB_CMD) === CMD_PRESS_RESET) {
        try {
          chip?.pressResetButton?.();
        } catch {
        }
        Atomics.store(ctrl, SAB_RESP, RESP_DONE);
        Atomics.store(ctrl, SAB_CMD, CMD_NONE);
      }
      if (Atomics.load(ctrl, SAB_CMD) === CMD_PRESS_BOOT) {
        try {
          chip?.pressBootButton?.(Atomics.load(ctrl, SAB_CMD_ARG0) !== 0);
        } catch {
        }
        Atomics.store(ctrl, SAB_RESP, RESP_DONE);
        Atomics.store(ctrl, SAB_CMD, CMD_NONE);
      }
      chip.step();
      steps++;
      cycles = chip.cycles;
      if (fireDue) {
        try {
          fireDue();
        } catch (_) {
        }
      }
      const isIdle = idleView0 && idleView1 ? idleView0[0] !== 0 && pendingView0[0] === 0 && idleView1[0] !== 0 && pendingView1[0] === 0 : chip.coresIdle;
      if (isIdle) {
        if (idleAdvance) {
          try {
            chip.cycles += idleAdvance(chip.cycles >>> 0);
            cycles = chip.cycles;
          } catch (_) {
          }
        }
      } else {
        syncClockState(wasmExp);
        if (pumpEvents) {
          try {
            pumpEvents();
          } catch (_) {
          }
        }
      }
      if ((cycles & 524287) === 0) writeSABState();
      if (!Atomics.load(ctrl, SAB_RUN)) break;
    }
  } catch (err) {
    console.error("[SIM-ERROR]", err.message, err.stack);
    setError(err.message);
    finishSim();
    setTimeout(() => commandLoop(), 0);
    return;
  }
  writeSABState();
  if (Atomics.load(ctrl, SAB_RUN)) {
    setTimeout(() => runSimChunk(), 0);
  } else {
    finishSim();
    setTimeout(() => commandLoop(), 0);
  }
}
function processCommand(cmd) {
  switch (cmd) {
    case CMD_RUN:
      ctrl[SAB_STATUS] = 0;
      runSimChunk();
      return true;
    case CMD_RESET:
      if (chip?.reset) chip.reset();
      break;
    case CMD_SEED_MMU: {
      const offset = Atomics.load(ctrl, SAB_CMD_ARG0);
      if (chip?.mmuTablePro) {
        chip.mmuTablePro[0] = 1;
        chip.mmuTablePro[1] = 2;
        chip.mmuTablePro[2] = 3;
        chip.mmuTablePro[3] = 0;
        const pages = Math.ceil((chip.flash?.length || 0) / 65536);
        for (let p = 4; p < pages + 1; p++) chip.mmuTablePro[p] = p + (offset || 0);
      }
      break;
    }
    case CMD_WRITE_UINT32: {
      const addr = Atomics.load(ctrl, SAB_CMD_ARG0);
      const value = Atomics.load(ctrl, SAB_CMD_ARG1);
      try {
        chip?.cores?.[0]?.writeUint32(addr, value);
      } catch {
      }
      break;
    }
    case CMD_READ_MEMORY: {
      const addr = Atomics.load(ctrl, SAB_CMD_ARG0);
      const maxBytes = Atomics.load(ctrl, SAB_CMD_ARG1);
      let written = 0;
      if (readResp) {
        try {
          const mem = chip?.mapAddress ? chip.mapAddress(addr) : null;
          if (mem && mem.data) {
            const offset = addr - mem.baseAddr;
            const available = Math.min(mem.data.length - offset, readResp.length);
            const len = Math.min(maxBytes || available, available);
            if (len > 0) readResp.set(mem.data.subarray(offset, offset + len), 0);
            written = len;
          }
        } catch {
        }
      }
      Atomics.store(ctrl, SAB_CMD_ARG1, written);
      break;
    }
    case CMD_GET_PCAP: {
      let written = 0;
      if (readResp && chip?._wifiBridge?.ap) {
        try {
          const data = chip._wifiBridge.ap.getPcapData();
          const len = Math.min(data.length, readResp.length);
          readResp.set(data.subarray(0, len), 0);
          written = len;
        } catch {
        }
      }
      Atomics.store(ctrl, SAB_CMD_ARG1, written);
      break;
    }
    case CMD_GET_WIFI_STATS: {
      let written = 0;
      chip?._syncWifiStatus?.();
      if (readResp && chip?._wifiBridge?.ap) {
        try {
          const s = chip._wifiBridge.ap.status;
          const dv = new DataView(readResp.buffer, readResp.byteOffset, readResp.length);
          dv.setInt32(0, s.state, true);
          dv.setInt32(4, s.txFrames, true);
          dv.setInt32(8, s.txBytes, true);
          dv.setInt32(12, s.rxFrames, true);
          dv.setInt32(16, s.rxBytes, true);
          dv.setInt32(20, s.probeRequestCount, true);
          dv.setInt32(24, s.connectedClients, true);
          const enc = new TextEncoder();
          let off = 28;
          const writeStr = (str) => {
            const b = enc.encode(str);
            const len = Math.min(b.length, 39);
            readResp.set(b.subarray(0, len), off);
            readResp[off + len] = 0;
            off += 40;
          };
          writeStr(s.ip || "");
          writeStr(s.portForward || "");
          writeStr(s.errorMessage || "");
          writeStr(s.udpForward || "");
          written = off;
        } catch {
        }
      }
      Atomics.store(ctrl, SAB_CMD_ARG1, written);
      break;
    }
    case CMD_GET_FFI_COUNTS: {
      let written = 0;
      if (readResp && chip?._wasmLoader?._mmioCount) {
        const pages = chip._wasmLoader._mmioPageCount;
        let pi = 256 * 8;
        const pdv = new DataView(readResp.buffer, readResp.byteOffset, readResp.length);
        for (const [k, v] of pages) {
          if (pi + 16 > readResp.length) break;
          pdv.setFloat64(pi, parseInt(k, 16), true);
          pdv.setFloat64(pi + 8, v, true);
          pi += 16;
        }
        Atomics.store(ctrl, SAB_CMD_ARG0, pi);
        const src = chip._wasmLoader._mmioCount;
        const n = Math.min(src.length, Math.floor(readResp.length / 8));
        const dv = new DataView(readResp.buffer, readResp.byteOffset, readResp.length);
        for (let i = 0; i < n; i++) dv.setFloat64(i * 8, src[i], true);
        written = n * 8;
      }
      Atomics.store(ctrl, SAB_CMD_ARG1, written);
      break;
    }
    case CMD_READ_MMIO: {
      const hid = Atomics.load(ctrl, SAB_CMD_ARG0);
      const addr = Atomics.load(ctrl, SAB_CMD_ARG1);
      const size = Atomics.load(ctrl, SAB_CMD_ARG2);
      let val = 0;
      try {
        const ex = chip?._wasmLoader?.exports;
        if (hid === 99 && ex?.native_bt_diag) {
          const scratch = ex.native_bt_diag_scratch?.() >>> 0 || 0;
          if (scratch) {
            ex.native_bt_diag(addr >>> 0, scratch);
            const mem = new Uint32Array(chip._wasmLoader.memoryBuffer);
            const base = scratch >>> 2;
            Atomics.store(ctrl, SAB_CMD_ARG0, mem[base] >>> 0);
            Atomics.store(ctrl, SAB_CMD_ARG1, mem[base + 1] >>> 0);
            Atomics.store(ctrl, SAB_CMD_ARG2, mem[base + 2] >>> 0);
            Atomics.store(ctrl, SAB_RESP, mem[base + 3] >>> 0);
            Atomics.store(ctrl, SAB_STATUS, mem[base + 4] >>> 0);
            Atomics.store(ctrl, SAB_PC, mem[base + 5] >>> 0);
            Atomics.store(ctrl, SAB_NANOS_LO, mem[base + 6] >>> 0);
            Atomics.store(ctrl, SAB_NANOS_HI, mem[base + 7] >>> 0);
          }
        } else if (ex?.native_diag_read) val = ex.native_diag_read(hid, addr, size);
      } catch (_e) {
        console.warn(`native_diag_read error: ${_e.message}`);
      }
      if (hid !== 99) Atomics.store(ctrl, SAB_CMD_ARG0, val);
      break;
    }
    case CMD_WATCHPOINT: {
      const addr = Atomics.load(ctrl, SAB_CMD_ARG0);
      try {
        const ex = chip?._wasmLoader?.exports;
        if (addr >>> 0 === 4294967295) ex?.native_watchpoint_clear?.();
        else ex?.native_watchpoint_add?.(addr >>> 0);
      } catch (_e) {
        console.warn(`native_watchpoint error: ${_e.message}`);
      }
      break;
    }
    case CMD_PCTRACE: {
      const n = Atomics.load(ctrl, SAB_CMD_ARG0);
      try {
        chip?._wasmLoader?.exports?.native_pc_trace?.(n >>> 0);
      } catch (_e) {
      }
      break;
    }
    case CMD_STEP: {
      const count = Atomics.load(ctrl, SAB_CMD_ARG0);
      if (count > 0) {
        for (let i = 0; i < count; i++) chip.step();
      }
      writeSABState();
      break;
    }
    case CMD_UART_RX: {
      if (uartRxCtrl && uartRxRing) {
        const w = Atomics.load(uartRxCtrl, 0);
        const r = Atomics.load(uartRxCtrl, 1);
        const count = w - r;
        if (count > 0) {
          const nativeFeed = chip?._wasmLoader?.exports?.native_uart_feed || null;
          const maxRead = Math.min(count, UART_RING_SIZE);
          for (let i = 0; i < maxRead; i++) {
            const byte = uartRxRing[(r + i) % UART_RING_SIZE];
            try {
              if (nativeFeed) nativeFeed(0, byte);
              else chip.uart[0].feedByte(byte);
            } catch {
            }
          }
          Atomics.store(uartRxCtrl, 1, r + maxRead);
        }
      }
      break;
    }
    case CMD_SET_VOLTAGE: {
      try {
        chip?.setVddMv?.(Atomics.load(ctrl, SAB_CMD_ARG0));
      } catch {
      }
      break;
    }
    case CMD_SET_TOUCH_INPUT: {
      try {
        chip?.setTouchInput?.(Atomics.load(ctrl, SAB_CMD_ARG0), Atomics.load(ctrl, SAB_CMD_ARG1));
      } catch {
      }
      break;
    }
    case CMD_SET_PIN_INPUT: {
      const pin = Atomics.load(ctrl, SAB_CMD_ARG0);
      const level = Atomics.load(ctrl, SAB_CMD_ARG1);
      try {
        chip?._wasmLoader?.exports?.native_gpio_set_pin_input?.(pin, level);
      } catch {
      }
      const holder = chip?.gpio?.pins?.[pin];
      if (holder) {
        try {
          holder.inputValue = level ? 1 : 0;
        } catch {
        }
      }
      break;
    }
    case CMD_FEED_I2S_RX: {
      const sample = Atomics.load(ctrl, SAB_CMD_ARG0) >>> 0;
      try {
        chip?._wasmLoader?.exports?.native_i2s_push_rx_sample?.(0, sample);
      } catch {
      }
      break;
    }
    // Dev-board buttons (real-device behavior): RESET (EN) taps chip reset;
    // BOOT (GPIO0 strapping) drives the strap bit + live pin level.
    // Serviced inline here (like CMD_SET_PIN_INPUT) so they work mid-run.
    case CMD_PRESS_RESET: {
      try {
        chip?.pressResetButton?.();
      } catch {
      }
      break;
    }
    case CMD_PRESS_BOOT: {
      try {
        chip?.pressBootButton?.(Atomics.load(ctrl, SAB_CMD_ARG0) !== 0);
      } catch {
      }
      break;
    }
    case CMD_SEND_TWAI: {
      twaiStage[0] = Atomics.load(ctrl, SAB_CMD_ARG0) >>> 0;
      twaiStage[1] = Atomics.load(ctrl, SAB_CMD_ARG1) >>> 0;
      twaiStage[2] = Atomics.load(ctrl, SAB_CMD_ARG2) >>> 0;
      break;
    }
    case CMD_PUSH_TWAI: {
      try {
        chip?._wasmLoader?.exports?.native_twai_push_rx?.(twaiStage[0], twaiStage[1], twaiStage[2], Atomics.load(ctrl, SAB_CMD_ARG0) >>> 0);
      } catch {
      }
      break;
    }
    case CMD_GET_TWAI_TX: {
      let written = 0;
      const exp = chip?._wasmLoader?.exports;
      if (readResp && exp?.native_twai_get_tx) {
        try {
          const dv = new DataView(readResp.buffer, readResp.byteOffset, readResp.length);
          for (let i = 0; i < 5; i++) dv.setUint32(i * 4, exp.native_twai_get_tx(i) >>> 0, true);
          written = 20;
        } catch {
        }
      }
      Atomics.store(ctrl, SAB_CMD_ARG1, written);
      break;
    }
  }
  return false;
}
function commandLoop() {
  while (true) {
    const cmd = Atomics.exchange(ctrl, SAB_CMD, CMD_NONE);
    if (cmd !== CMD_NONE) {
      try {
        const isRun = processCommand(cmd);
        if (isRun) return;
      } catch (err) {
        setError(err.message);
      }
      Atomics.store(ctrl, SAB_RESP, RESP_DONE);
    }
    setTimeout(() => commandLoop(), 0);
    return;
  }
}
function blockingCommandLoop() {
  while (true) {
    Atomics.wait(ctrl, SAB_CMD, CMD_NONE);
    const cmd = Atomics.exchange(ctrl, SAB_CMD, CMD_NONE);
    if (cmd !== CMD_NONE) {
      let ok = true;
      try {
        if (cmd === CMD_STEP || cmd === CMD_RESET || cmd === CMD_SEED_MMU || cmd === CMD_WRITE_UINT32 || cmd === CMD_READ_MEMORY || cmd === CMD_GET_PCAP || cmd === CMD_GET_WIFI_STATS || cmd === CMD_READ_MMIO || cmd === CMD_UART_RX || cmd === CMD_SET_PIN_INPUT || cmd === CMD_SET_TOUCH_INPUT || cmd === CMD_SET_VOLTAGE || cmd === CMD_FEED_I2S_RX || cmd === CMD_SEND_TWAI || cmd === CMD_PUSH_TWAI || cmd === CMD_GET_TWAI_TX || cmd === CMD_WATCHPOINT || cmd === CMD_PCTRACE || cmd === CMD_PRESS_RESET || cmd === CMD_PRESS_BOOT) {
          processCommand(cmd);
        } else if (cmd === CMD_RUN) {
          ctrl[SAB_STATUS] = 0;
          runSimChunk();
        } else if (cmd === CMD_DESTROY) {
          break;
        }
      } catch (err) {
        setError(err.message);
        ok = false;
      }
      Atomics.store(ctrl, SAB_RESP, ok ? RESP_DONE : RESP_ERROR);
      Atomics.notify(ctrl, SAB_RESP, 1);
    }
  }
}
function applyBasicSetup(chip2, config, flash, rom) {
  if (flash && chip2.flash) {
    if (typeof SharedArrayBuffer !== "undefined" && flash.buffer instanceof SharedArrayBuffer && flash.byteLength >= chip2.flash.length) {
      chip2.flash = new Uint8Array(flash.buffer, flash.byteOffset, chip2.flash.length);
      if (DEBUG_BOOT) console.log(`[DBG] Flash SAB-backed: ${chip2.flash.length} bytes`);
    } else {
      const off = chip2._firmwareOffset || 0;
      chip2.flash.set(flash, off);
      if (DEBUG_BOOT) console.log(`[DBG] Flash copied: ${flash.length} bytes to offset ${off}, flash[0]=${chip2.flash[0].toString(16)}`);
    }
  } else if (DEBUG_BOOT) console.log(`[DBG] No flash to load (flash=${!!flash} chip.flash=${!!chip2.flash})`);
  if (config.partitions && chip2.flash) writePartitionTable(chip2.flash, config.partitions);
  if (rom && chip2.loadROM) {
    chip2.loadROM(rom);
    if (DEBUG_BOOT) console.log(`[DBG] ROM loaded: ${rom.length} bytes, ROM[0]=${chip2.chipROM?.[0]?.toString(16)}`);
  }
  let mac = null;
  if (config.macAddress) mac = parseMacAddress(config.macAddress);
  if (chip2.reset) chip2.reset();
  if (chip2.mmuTablePro && config.mmuPages) {
    for (let p = 0; p < config.mmuPages; p++) chip2.mmuTablePro[p] = p;
    if (chip2.mmuTableApp) {
      for (let p = 0; p < config.mmuPages; p++) chip2.mmuTableApp[p] = p;
    }
  }
  if (chip2.gpio) {
    if (config.strapValue !== void 0) chip2.gpio.strapValue = config.strapValue;
    if (config.pinInputs) {
      for (const [pin, val] of Object.entries(config.pinInputs)) {
        if (chip2.gpio.pins?.[pin]) chip2.gpio.pins[pin].inputValue = val;
      }
    }
  }
  if (config.analogInputs) {
    for (const [pin, volts] of Object.entries(config.analogInputs)) {
      chip2.setAnalogInput?.(Number(pin), volts);
    }
  }
  if (config.touchInputs) {
    for (const [pad, count] of Object.entries(config.touchInputs)) {
      chip2.setTouchInput?.(Number(pad), count);
    }
  }
  if (chip2.uart?.[0]) {
    chip2.uart[0].onTX = (byte) => {
      uartWriteByte(byte);
    };
  }
  if (chip2.cores?.[1]) chip2.cores[1].enabled = true;
  try {
    chip2.cores[0].writeUint32(1073062148, 23205);
  } catch (_) {
  }
  if (mac) {
    try {
      chip2.cores[0].writeUint32(1073061892, (mac[2] << 24 | mac[3] << 16 | mac[4] << 8 | mac[5]) >>> 0);
    } catch (_) {
    }
    try {
      let crc = 0;
      for (let i = 0; i < mac.length; i++) {
        crc ^= mac[i];
        for (let j = 0; j < 8; j++) {
          const lsb = crc & 1;
          crc >>= 1;
          if (lsb) crc ^= 140;
        }
      }
      chip2.cores[0].writeUint32(1073061896, (crc << 16 | mac[0] << 8 | mac[1]) >>> 0);
    } catch (_) {
    }
  }
}
function setupNativeWifiBridge(chip2, config) {
  if (config.wifi === false) return;
  const loader = chip2._wasmLoader;
  const mem = chip2._wasmMemory;
  const boardMac = config.macAddress ? parseMacAddress(config.macAddress) : null;
  const wifiOpts = config.wifi === true ? {} : config.wifi || {};
  const ssidB = new TextEncoder().encode(wifiOpts.ssid || "ESP-GUEST");
  const bssidB = wifiOpts.bssid ? parseMacAddress(wifiOpts.bssid) : null;
  const scratch = loader.exports.native_wifi_ap_scratch();
  const sv = new Uint8Array(mem, scratch, 128);
  sv.set(ssidB, 0);
  const bssidOff = 64;
  if (bssidB) sv.set(bssidB, bssidOff);
  loader.exports.native_wifi_ap_init(scratch, ssidB.length, scratch + bssidOff, bssidB ? 6 : 0, wifiOpts.channel || 6, 0);
  const status = { state: 0, errorMessage: "", txFrames: 0, txBytes: 0, rxFrames: 0, rxBytes: 0, probeRequestCount: 0, connectedClients: 0, ip: "", portForward: "", udpForward: "" };
  const pcapBuffer = [];
  const packetBuffer = [];
  let socket = null;
  const roomQuery = wifiOpts.room ? `?sessionId=${encodeURIComponent(wifiOpts.room)}` : "";
  const connectGateway = () => {
    if (socket) return;
    try {
      socket = new WebSocket(`ws://127.0.0.1:5085/api/network-gateway${roomQuery}`);
      socket.binaryType = "arraybuffer";
      socket.onopen = () => {
        while (packetBuffer.length > 0) socket.send(packetBuffer.shift());
      };
      socket.onmessage = (ev) => {
        if (ev.data instanceof ArrayBuffer) {
          const bytes = new Uint8Array(ev.data);
          if (bytes.length > 4 && bytes[0] === 229 && bytes[1] === 80 && bytes[2] === 78 && bytes[3] === 87) {
            const raw = bytes.subarray(4);
            const v2 = new Uint8Array(mem, scratch, raw.length);
            v2.set(raw);
            loader.exports.native_wifi_mac_rx_frame(scratch, raw.length, 0);
            return;
          }
          const eth = bytes;
          pcapBuffer.push({ timeUs: performance.now() * 1e3, data: eth });
          const v = new Uint8Array(mem, scratch, eth.length);
          v.set(eth);
          loader.exports.native_wifi_ap_eth_rx(eth.length);
        } else if (typeof ev.data === "string") {
          const msg = ev.data;
          if (msg.startsWith("BOARD_IP:")) status.ip = msg.substring(9);
          else if (msg.startsWith("PORT_FORWARD:")) status.portForward = msg.substring(13);
          else if (msg.startsWith("UDP_FORWARD:")) status.udpForward = msg.substring(12);
          console.log(msg);
        }
      };
      socket.onerror = () => {
        status.state = 4;
        status.errorMessage = "Gateway connection failed";
      };
    } catch (_) {
    }
  };
  loader._wifiApRxFrame = (ptr, len) => {
    loader.exports.native_wifi_mac_rx_frame(ptr, len, 0);
  };
  loader._wifiApSendEth = (ptr, len) => {
    const eth = new Uint8Array(mem, ptr, len).slice();
    pcapBuffer.push({ timeUs: performance.now() * 1e3, data: eth });
    if (socket && socket.readyState === WebSocket.OPEN) socket.send(eth);
    else packetBuffer.push(eth);
  };
  loader._wifiApConnected = () => {
    connectGateway();
  };
  const ESPNOW_MAGIC = [229, 80, 78, 87];
  loader._espnowTx = (frame) => {
    const marked = new Uint8Array(4 + frame.length);
    marked.set(ESPNOW_MAGIC, 0);
    marked.set(frame, 4);
    if (socket && socket.readyState === WebSocket.OPEN) socket.send(marked);
    else packetBuffer.push(marked);
  };
  const syncWifiStatus = () => {
    const st = new DataView(mem, scratch + 128, 28);
    loader.exports.native_wifi_ap_get_status(scratch + 128);
    status.state = st.getInt32(0, true);
    status.txFrames = st.getInt32(4, true);
    status.txBytes = st.getInt32(8, true);
    status.rxFrames = st.getInt32(12, true);
    status.rxBytes = st.getInt32(16, true);
    status.probeRequestCount = st.getInt32(20, true);
    status.connectedClients = st.getInt32(24, true);
  };
  const beaconEvent = chip2.clocks.cpu.createEvent(() => {
    loader.exports.native_wifi_ap_send_beacon();
    beaconEvent.schedule(102e6);
  });
  beaconEvent.schedule(0);
  loader._wifiMacTx = (frame) => {
    if (frame.length >= 24 && boardMac && frame.slice(10, 16).every((b) => b === 0)) frame.set(boardMac, 10);
    const v = new Uint8Array(mem, scratch, frame.length);
    v.set(frame);
    loader.exports.native_wifi_ap_handle_tx(frame.length, 0);
  };
  loader.exports.native_wifi_mac_rx_force(1, 1);
  connectGateway();
  chip2._wifiBridge = {
    ap: {
      status,
      getPcapData: () => {
        const header = new ArrayBuffer(24);
        const h = new DataView(header);
        h.setUint32(0, 2712847316, false);
        h.setUint16(4, 2, false);
        h.setUint16(6, 4, false);
        h.setInt32(8, 0, false);
        h.setUint32(12, 0, false);
        h.setUint32(16, 65535, false);
        h.setUint32(20, 1, false);
        const parts = [new Uint8Array(header)];
        for (const p of pcapBuffer) {
          const ph = new ArrayBuffer(16);
          const pv = new DataView(ph);
          const sec = Math.floor(p.timeUs / 1e6);
          const usec = Math.floor(p.timeUs % 1e6);
          pv.setUint32(0, sec, false);
          pv.setUint32(4, usec, false);
          pv.setUint32(8, p.data.length, false);
          pv.setUint32(12, p.data.length, false);
          parts.push(new Uint8Array(ph));
          parts.push(p.data);
        }
        const total = parts.reduce((s, a) => s + a.length, 0);
        const out = new Uint8Array(total);
        let off = 0;
        for (const a of parts) {
          out.set(a, off);
          off += a.length;
        }
        return out;
      },
      clearPcapBuffer: () => {
        pcapBuffer.length = 0;
      }
    }
  };
  chip2._syncWifiStatus = syncWifiStatus;
}
var BOOTROM_PATHS = {
  ESP32: "../rom/esp32-v3-rom.bin"
};
var WASM_PATHS = {
  ESP32: "../engine/esp-xtensa/esp_engine_wasm.wasm"
};
async function loadBinary(config, chipType, name) {
  const pathMap = name === "bootrom" ? BOOTROM_PATHS : WASM_PATHS;
  const _fallbackKey = name === "bootrom" ? "bootrom" : "wasmBinary";
  const configKey = name === "bootrom" ? "bootrom" : "wasmBinary";
  if (config[configKey]) {
    return config[configKey];
  }
  const relPath = pathMap[chipType] || pathMap.ESP32;
  try {
    const url = new URL(relPath, import.meta.url);
    if ((url.protocol === "http:" || url.protocol === "https:" || url.protocol === "data:") && typeof fetch === "function") {
      const res = await fetch(url);
      if (res.ok) return new Uint8Array(await res.arrayBuffer());
    }
  } catch (_) {
  }
  try {
    const { readFileSync } = await import("fs");
    const { resolve, dirname } = await import("path");
    const { fileURLToPath } = await import("url");
    const __dirname = dirname(fileURLToPath(import.meta.url));
    return readFileSync(resolve(__dirname, relPath));
  } catch (_e) {
    setError(`Failed to load ${name} binary: ` + _e.message);
    return null;
  }
}
function createDefaultFlash() {
  const size = 4 * 1024 * 1024;
  const flash = new Uint8Array(size);
  flash.fill(255);
  flash[4096] = 233;
  flash[4097] = 1;
  flash[4098] = 32;
  flash[4099] = 64;
  flash[4100] = 0;
  flash[4101] = 0;
  flash[4102] = 0;
  flash[4103] = 64;
  flash[4108] = 0;
  flash[4109] = 0;
  flash[4110] = 0;
  flash[4111] = 0;
  flash[4116] = 0;
  flash[4117] = 0;
  flash[4118] = 0;
  flash[4119] = 64;
  flash[4120] = 4;
  flash[4121] = 0;
  flash[4122] = 0;
  flash[4123] = 0;
  flash[4124] = 0;
  flash[4125] = 0;
  flash[4126] = 0;
  flash[4127] = 0;
  flash[4128] = 239;
  return flash;
}
async function onMessage(type, data) {
  switch (type) {
    case "init": {
      const { chipType, config, flash, rom, sab, uartSab, uartRxSab, readRespSab, errSab, debugSab, msgId } = data;
      ctrl = new Int32Array(sab);
      uartCtrl = new Int32Array(uartSab, 0, 2);
      uartRing = new Uint8Array(uartSab, 8, UART_RING_SIZE);
      if (uartRxSab) {
        uartRxCtrl = new Int32Array(uartRxSab, 0, 2);
        uartRxRing = new Uint8Array(uartRxSab, 8, UART_RING_SIZE);
      }
      if (readRespSab) readResp = new Uint8Array(readRespSab);
      if (errSab) {
        errMsg = new Uint8Array(errSab);
        errMsg.fill(0);
      }
      if (debugSab) debugData = new Uint32Array(debugSab);
      const ChipClass = chipConstructors[chipType];
      if (!ChipClass) {
        setError("Unknown chip: " + chipType);
        send({ type: "ready", memoryRegions: {}, msgId });
        if (config.simMode === "blocking") {
          setTimeout(() => blockingCommandLoop(), 0);
        } else {
          commandLoop();
        }
        return;
      }
      try {
        const engineMode = config.engine || ENGINE;
        const loadFlash = flash || createDefaultFlash();
        const loadRom = rom || await loadBinary(config, chipType, "bootrom");
        chip = new ChipClass(config);
        if (DEBUG_BOOT) console.log(`[DBG] Chip created, ROM=${!!chip.chipROM} flashLen=${chip.flash?.length}`);
        applyBasicSetup(chip, config, loadFlash, loadRom);
        if (DEBUG_BOOT) console.log(`[DBG] after applyBasicSetup: PC=0x${chip.cores?.[0]?.PC?.toString(16).padStart(8, "0")} flash[0]=${chip.flash?.[0]?.toString(16)} ROM[0]=${chip.chipROM?.[0]?.toString(16)}`);
        {
          const wasmBinary = await loadBinary(config, chipType, "wasmBinary");
          if (wasmBinary) {
            if (DEBUG_BOOT) console.log(`[DBG] Loading WASM (${wasmBinary.length} bytes) from ${WASM_PATHS[chipType] || WASM_PATHS.ESP32}...`);
            await chip.loadWasm(wasmBinary, "wasm");
            if (config.pcTrace && chip._wasmLoader?.exports?.native_pc_trace) {
              try {
                chip._wasmLoader.exports.native_pc_trace(config.pcTrace);
              } catch (_) {
              }
            }
            if (DEBUG_BOOT) console.log(`[DBG] WASM loaded, resetting chip...`);
            chip.reset();
            console.log(`[DBG] WASM: about to re-seed MMU, mmuTablePro=${!!chip.mmuTablePro} mmuPages=${config.mmuPages}`);
            if (chip.mmuTablePro && config.mmuPages) {
              for (let p = 0; p < config.mmuPages; p++) chip.mmuTablePro[p] = p;
              if (chip.mmuTableApp) {
                for (let p = 0; p < config.mmuPages; p++) chip.mmuTableApp[p] = p;
              }
              if (DEBUG_BOOT) console.log(`[DBG] WASM: MMU re-seeded (${config.mmuPages} pages)`);
            }
            if (chip.cores?.[1]) chip.cores[1].enabled = true;
            if (chip._wasmCores?.[1]) chip._wasmCores[1].enabled = true;
            if (chip.gpio) {
              if (config.strapValue !== void 0) chip.gpio.strapValue = config.strapValue;
              if (config.pinInputs) {
                for (const [pin, val] of Object.entries(config.pinInputs)) {
                  if (chip.gpio.pins?.[pin]) chip.gpio.pins[pin].inputValue = val;
                }
              }
              chip._wasmLoader?.seedNativeGpio?.();
            }
            if (chip.uart?.[0]) {
              chip.uart[0].onTX = (byte) => {
                uartWriteByte(byte);
              };
            }
            try {
              chip.cores[0].writeUint32(1073062148, 23205);
            } catch (_) {
            }
            if (config.macAddress) {
              const mac = parseMacAddress(config.macAddress);
              const lo = (mac[0] | mac[1] << 8 | mac[2] << 16 | mac[3] << 24) >>> 0;
              const hi = (mac[4] | mac[5] << 8) >>> 0;
              if (config.vddMv !== void 0) chip.setVddMv?.(config.vddMv);
              if (chip._wasmLoader?.exports?.native_wifi_mac_set_mac) {
                chip._wasmLoader.exports.native_wifi_mac_set_mac(lo, hi);
              }
              try {
                chip.cores[0].writeUint32(1073061892, (mac[2] << 24 | mac[3] << 16 | mac[4] << 8 | mac[5]) >>> 0);
              } catch (_) {
              }
              try {
                let crc = 0;
                for (let i = 0; i < mac.length; i++) {
                  crc ^= mac[i];
                  for (let j = 0; j < 8; j++) {
                    const lsb = crc & 1;
                    crc >>= 1;
                    if (lsb) crc ^= 140;
                  }
                }
                chip.cores[0].writeUint32(1073061896, (crc << 16 | mac[0] << 8 | mac[1]) >>> 0);
              } catch (_) {
              }
            }
            if (config.sdCard !== false && !chip.sdData) {
              const opt = config.sdCard === true ? {} : config.sdCard || {};
              const blocks = opt.blocks ?? Math.max(1024, Math.round((opt.sizeMB ?? 16) * 1024 * 1024 / 512));
              chip.sdData = new Uint8Array(new SharedArrayBuffer(blocks * 512));
              if (opt.image instanceof Uint8Array) chip.sdData.set(opt.image.subarray(0, chip.sdData.length));
            }
            if (config.sdCard === false) chip.sdData = null;
            try {
              const sdOpt = config.sdCard === true ? {} : config.sdCard || {};
              const mmc = sdOpt.type === "mmc" ? 1 : 0;
              chip._wasmLoader?.exports?.native_sdmmc_set_card_mmc?.(mmc);
            } catch (_) {
            }
            setupNativeWifiBridge(chip, config);
            if (config.camFrameBytes !== void 0) {
              try {
                chip._wasmLoader?.exports?.native_i2s_cam_frame_bytes?.(0, config.camFrameBytes >>> 0);
              } catch (_) {
              }
            }
            try {
              chip._wasmLoader?.exports?.native_spi_set_psram_size?.(chip._psramSizeMB >>> 0);
            } catch (_) {
            }
            if (DEBUG_BOOT) console.log(`[DBG] After reset: PC=0x${chip._wasmCores?.[0]?.PC?.toString(16).padStart(8, "0")} JS_PC=0x${chip.cores?.[0]?.PC?.toString(16).padStart(8, "0")}`);
            try {
              const SAB = chip._wasmMemory || chip._wasmLoader && chip._wasmLoader.memory && chip._wasmLoader.memory.buffer;
              console.log(`[DBG] SAB=${!!SAB} _wasmMemory=${!!chip._wasmMemory} _wasmLoader=${!!chip._wasmLoader}`);
              if (SAB) {
                const { PAGE_TABLE_OFFSET: PAGE_TABLE_OFFSET2 } = await Promise.resolve().then(() => (init_wasm_memory_layout(), wasm_memory_layout_exports));
                const ptU32Check = new Uint32Array(SAB, PAGE_TABLE_OFFSET2);
                const t0 = ptU32Check[261983 * 2];
                const t0d = ptU32Check[261983 * 2 + 1];
                const s0 = ptU32Check[261891 * 2];
                const s0d = ptU32Check[261891 * 2 + 1];
                console.log(`[DBG] After reset PTE: TIMG0 type=${t0} data=0x${(t0d >>> 0).toString(16)} SHA type=${s0} data=0x${(s0d >>> 0).toString(16)}`);
              }
            } catch (_e) {
              console.log(`[DBG] PTE check error: ${_e.message}`);
            }
          }
        }
        if (config.simMode === "chunked") needsEventLoop = true;
        else if (config.simMode === "tight") needsEventLoop = false;
        else if (config.simMode === "blocking") needsEventLoop = false;
        if (typeof config.chunkSize === "number" && config.chunkSize > 0) simChunkSize = config.chunkSize;
        if (config.nanos !== void 0) {
          chip.cycles = Math.round(Number(config.nanos) / 1e9 * 16e7);
          syncClockState();
        }
        {
          const cf = config.cpuFrequency;
          const native = chip._nativeFrequency || 16e7;
          ctrl[SAB_CPU_FREQ] = !cf || cf === "max" ? native : cf === "auto" ? 8e6 : Number(cf) * 1e6;
        }
        writeSABState();
        send({ type: "ready", memoryRegions: getMemorySABs(chip), engineMode, chipInfo: { board: chip.board || "esp32", flashSizeMB: chip._flashSizeMB, psramSizeMB: chip._psramSizeMB, psramType: chip._psramType, cpuFrequency: 16e7, _nativeFrequency: chip._nativeFrequency }, msgId });
        if (config.simMode === "blocking") {
          setTimeout(() => blockingCommandLoop(), 0);
        } else {
          setTimeout(() => commandLoop(), 0);
        }
      } catch (err) {
        setError(err.message);
        send({ type: "ready", memoryRegions: {}, msgId });
        if (config.simMode === "blocking") {
          setTimeout(() => blockingCommandLoop(), 0);
        } else {
          setTimeout(() => commandLoop(), 0);
        }
      }
      break;
    }
  }
}
if (typeof self !== "undefined") {
  self.onmessage = (e) => onMessage(e.data.type, e.data);
} else {
  msgPort.on("message", (data) => onMessage(data.type, data));
}
