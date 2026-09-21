// @ts-check
/** Base memory class — flat byte array with an absolute base address.
 * Provides little-endian read/write at 8/16/32-bit widths. */
export class Memory {
  /** @param {Uint8Array} data @param {number} base */
  constructor(data, base) {
    ((this.data = data), (this.base = base));
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
    return tmpVal[addr - idxVal] | (tmpVal[addr - idxVal + 1] << 8);
  }
  /** @param {number} addr @returns {number} */
  readUint32(addr) {
    let { data: tmpVal, base: idxVal } = this,
      off = addr - idxVal;
    return (
      (tmpVal[off] |
        (tmpVal[off + 1] << 8) |
        (tmpVal[off + 2] << 16) |
        (tmpVal[off + 3] << 24)) >>>
      0
    );
  }
  /** @param {number} addr @param {number} val */
  writeUint8(addr, val) {
    let { data: idxVal, base: ClockEvent } = this;
    idxVal[addr - ClockEvent] = val;
  }
  /** @param {number} addr @param {number} val */
  writeUint16(addr, val) {
    let { data: idxVal, base: ClockEvent } = this;
    ((idxVal[addr - ClockEvent] = 255 & val),
      (idxVal[addr - ClockEvent + 1] = (val >> 8) & 255));
  }
  /** @param {number} addr @param {number} val */
  writeUint32(addr, val) {
    let { data: idxVal, base: ClockEvent } = this,
      off = addr - ClockEvent;
    ((idxVal[off] = 255 & val),
      (idxVal[off + 1] = (val >> 8) & 255),
      (idxVal[off + 2] = (val >> 16) & 255),
      (idxVal[off + 3] = (val >> 24) & 255));
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
        srcAddr - src.base + len,
      ),
      dstAddr - this.base,
    );
  }
  /** @param {number} base @param {number} len @returns {Memory} */
  createView(base, len) {
    return new Memory(this.data.subarray(len, len + base), base);
  }
  /** @param {number} base @returns {Memory} */
  remap(base) {
    return new Memory(this.data, base);
  }
}

/**
 * Read-only memory region. Writes are blocked unless
 * `ReadonlyMemory.override` is true (used by GDB flash writes).
 */
export class ReadonlyMemory extends Memory {
  /** @param {number} addr @param {number} val */
  writeUint8(addr, val) {
    if (ReadonlyMemory.override) return super.writeUint8(addr, val);
    console.error("Invalid write to read-only memory at", addr.toString(16));
  }
  /** @param {number} addr @param {number} val */
  writeUint16(addr, val) {
    if (ReadonlyMemory.override) return super.writeUint16(addr, val);
    console.error("Invalid write to read-only memory at", addr.toString(16));
  }
  /** @param {number} addr @param {number} val */
  writeUint32(addr, val) {
    if (ReadonlyMemory.override) return super.writeUint32(addr, val);
    console.error("Invalid write to read-only memory at", addr.toString(16));
  }
  /** @param {number} base @param {number} len @returns {ReadonlyMemory} */
  createView(base, len) {
    return new ReadonlyMemory(
      this.data.subarray(len, len + base),
      base,
    );
  }
}
/** @type {boolean} Global override flag — set by GDB session during flash writes */
ReadonlyMemory.override = false;

/** Stub memory: reads return a fixed default, writes are dropped. */
export class InvalidMemory {
  /** @param {number} [defaultValue=0xFFFFFFFF] */
  constructor(defaultValue = 0xffffffff) {
    this.defaultValue = defaultValue;
  }
  /** @returns {number} */
  readUint8() { return 255 & this.defaultValue; }
  /** @returns {number} */
  readUint16() { return 65535 & this.defaultValue; }
  /** @returns {number} */
  readUint32() { return 0 | this.defaultValue; }
  writeUint8() {}
  writeUint16() {}
  writeUint32() {}
}

/** Address-shifting wrapper: translates addresses by a fixed delta before forwarding. */
export class MemoryTranslator {
  /** @param {Memory} base @param {number} delta */
  constructor(base, delta) {
    ((this.base = base), (this.delta = delta));
  }
  /** @param {number} addr @returns {number} */
  readUint8(addr) { return this.base.readUint8(addr - this.delta); }
  /** @param {number} addr @returns {number} */
  readUint16(addr) { return this.base.readUint16(addr - this.delta); }
  /** @param {number} addr @returns {number} */
  readUint32(addr) { return this.base.readUint32(addr - this.delta); }
  /** @param {number} addr @param {number} val */
  writeUint8(addr, val) { this.base.writeUint8(addr - this.delta, val); }
  /** @param {number} addr @param {number} val */
  writeUint16(addr, val) { this.base.writeUint16(addr - this.delta, val); }
  /** @param {number} addr @param {number} val */
  writeUint32(addr, val) { this.base.writeUint32(addr - this.delta, val); }
}

/** Byte-reversed addressing (for SRAM1 reverse window). */
export class ReverseMemory {
  /** @param {Uint8Array} data @param {number} base */
  constructor(data, base) {
    ((this.data = data), (this.base = base));
  }
  /** @param {number} addr @returns {number} byte-reversed offset */
  translateAddress(addr) {
    let off = addr - this.base;
    return this.data.length - 4 - (0xfffffffc & off) + (3 & off);
  }
  /** @param {number} addr @returns {number} */
  readUint8(addr) { return this.data[this.translateAddress(addr)]; }
  /** @param {number} addr @returns {number} */
  readUint16(addr) {
    let lo = this.translateAddress(addr),
      hi = this.translateAddress(addr + 1);
    return this.data[lo] | (this.data[hi] << 8);
  }
  /** @param {number} addr @returns {number} */
  readUint32(addr) {
    let off = this.translateAddress(addr);
    return (
      (this.data[off] |
        (this.data[off + 1] << 8) |
        (this.data[off + 2] << 16) |
        (this.data[off + 3] << 24)) >>> 0
    );
  }
  /** @param {number} addr @param {number} val */
  writeUint8(addr, val) { this.data[this.translateAddress(addr)] = val; }
  /** @param {number} addr @param {number} val */
  writeUint16(addr, val) {
    let lo = this.translateAddress(addr),
      hi = this.translateAddress(addr + 1);
    ((this.data[lo] = 255 & val),
      (this.data[hi] = (val >> 8) & 255));
  }
  /** @param {number} addr @param {number} val */
  writeUint32(addr, val) {
    let off = this.translateAddress(addr);
    ((this.data[off] = 255 & val),
      (this.data[off + 1] = (val >> 8) & 255),
      (this.data[off + 2] = (val >> 16) & 255),
      (this.data[off + 3] = (val >> 24) & 255));
  }
}

/** MMU-table-driven address translation (used for PSRAM region). */
export class MMUMemory extends Memory {
  /** @param {Uint8Array} data @param {number} base */
  constructor(data, base) {
    (super(data, base),
      (this.table = new Uint32Array(0)),
      (this.tableOffset = 0));
  }
  /** @type {Uint32Array} */  table;
  /** @type {number} */  tableOffset;
  /** @param {Uint32Array} table @param {number} offset */
  setTable(table, offset) {
    this.table = table;
    this.tableOffset = offset;
  }
  /** @param {number} addr @returns {number} physical address */
  mapAddress(addr) {
    let off = addr - this.baseAddr,
      idx = off >>> 15,
      entry = this.table[this.tableOffset + idx] ?? idx;
    return this.baseAddr + (entry << 15) + (off & 32767);
  }
  /** @param {number} addr @returns {number} */
  readUint8(addr) { return ((addr = this.mapAddress(addr)), super.readUint8(addr)); }
  /** @param {number} addr @returns {number} */
  readUint16(addr) { return ((addr = this.mapAddress(addr)), super.readUint16(addr)); }
  /** @param {number} addr @returns {number} */
  readUint32(addr) { return ((addr = this.mapAddress(addr)), super.readUint32(addr)); }
  /** @param {number} addr @param {number} val */
  writeUint8(addr, val) { ((addr = this.mapAddress(addr)), super.writeUint8(addr, val)); }
  /** @param {number} addr @param {number} val */
  writeUint16(addr, val) { ((addr = this.mapAddress(addr)), super.writeUint16(addr, val)); }
  /** @param {number} addr @param {number} val */
  writeUint32(addr, val) { ((addr = this.mapAddress(addr)), super.writeUint32(addr, val)); }
}

// ─── Page Table & MMIO Handler Registry ───────────────────────────────────

export const PAGE_SHIFT = 12;
export const PAGE_SIZE = 1 << PAGE_SHIFT;       // 4096
export const PAGE_ENTRIES = 0x100000;            // 1,048,576 (covers 4GB)
export const PT_SIZE = PAGE_ENTRIES * 2;         // 2,097,152 u32 entries

export const PTE_TYPE_INVALID = 0;
export const PTE_TYPE_RAM = 1;
export const PTE_TYPE_MMIO = 2;
export const PTE_TYPE_FLASH = 3;

/** 4GB page table — 1M entries × 2 u32 each (type + data). */
export class PageTable {
  constructor() {
    this.table = new Uint32Array(PT_SIZE);
  }
  /** @param {number} addr @returns {number} */
  pageIndex(addr) { return addr >>> PAGE_SHIFT; }
  /** @param {number} addr @returns {number} PTE type */
  getType(addr) { return this.table[this.pageIndex(addr) * 2]; }
  /** @param {number} addr @returns {number} PTE data (RAM offset or MMIO handler ID) */
  getData(addr) { return this.table[this.pageIndex(addr) * 2 + 1]; }
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
}

/** Registry of MMIO handler objects, indexed by handler ID. */
export class MMIOHandlerRegistry {
  constructor() { this.handlers = []; }
  /** @param {object} handler @returns {number} handler ID */
  register(handler) {
    const id = this.handlers.length;
    this.handlers.push(handler);
    return id;
  }
  /** @param {number} id @returns {object|undefined} */
  get(id) { return this.handlers[id]; }
  /** @param {number} handlerId @param {number} addr @param {number} size @returns {number} */
  read(handlerId, addr, size) {
    const h = this.handlers[handlerId];
    if (!h) return 0;
    switch (size) {
      case 1: return h.mmio_read?.(addr, 1) ?? h.readUint8?.(addr) ?? 0;
      case 2: return h.mmio_read?.(addr, 2) ?? h.readUint16?.(addr) ?? 0;
      case 4: return h.mmio_read?.(addr, 4) ?? h.readUint32?.(addr) ?? 0;
      default: return 0;
    }
  }
  /** @param {number} handlerId @param {number} addr @param {number} val @param {number} size */
  write(handlerId, addr, val, size) {
    const h = this.handlers[handlerId];
    if (!h) return;
    switch (size) {
      case 1: h.mmio_write?.(addr, val, 1) ?? h.writeUint8?.(addr, val); break;
      case 2: h.mmio_write?.(addr, val, 2) ?? h.writeUint16?.(addr, val); break;
      case 4: h.mmio_write?.(addr, val, 4) ?? h.writeUint32?.(addr, val); break;
    }
  }
}

