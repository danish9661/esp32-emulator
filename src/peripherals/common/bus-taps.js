// Host peripheral taps (esp32-emu.md §1-4) — JS models for I2C slaves and SPI
// transfers, plus the registry sync that arms the Rust FFI hooks.
//
// Design (documented contract):
// - Registration is WIRING (survives chip.reset(), like soldered devices).
//   In-flight transaction state (_curAddr, mosiLog, events, gpio change
//   queue) clears on reset (pending events must not leak across reboot).
// - Preloaded read data (I2C readQueue, SPI misoQueue) is DEVICE memory and
//   survives reset (sensor ID ROM, SSD1306 state, ...).
// - With NO tap attached, firmware behavior is frozen: the Rust engine never
//   calls the FFI (registry/promisc gating) and keeps sibling/virt/loopback
//   routing bit-for-bit.
// - FFI handlers (_ffiStart/_ffiWrite/_ffiRead/_ffiStop/_ffiTransfer) run
//   synchronously INSIDE the guest's MMIO write. They MUST be synchronous
//   pure-JS (no WASM re-entry, no await). In the Worker build the same
//   objects live in the worker thread; the main-thread bridge pre-stages
//   reads (preloadRead/injectMiso) and drains writes/events (poll*), so no
//   round-trip is ever needed mid-transaction.

/**
 * I2C slave tap for one controller (I2C0 = bus 0, I2C1 = bus 1).
 *
 * Two registration styles (both post-create, F1 parity):
 *  A. Bus-level handlers (F1: `mcu.i2c0.onStart = …`):
 *     - onStart(addr7, read) → truthy = ACK/claim, falsy/undefined = NACK…
 *       NOTE: returning undefined falls through to the per-address registry
 *       below; return explicit false/0 to force NACK.
 *     - onWrite(byte) → truthy = ACK (default ACK when undefined).
 *     - onRead() → byte 0..255 (undefined → registry/0xFF fallback).
 *     - onStop(wasRead).
 *     Assigning ANY of onStart/onWrite/onRead arms promiscuous FFI for the
 *     whole bus (every address consults onStart; unclaimed → NACK, so
 *     scan() enumerates only what the handler claims).
 *  B. Per-address slaves: attachSlave(addr7, {onWrite?, onRead?, onStop?,
 *     onStart?}) + detachSlave(addr7). Only registered addresses ACK;
 *     everything else NACKs (scan-clean). The FFI is consulted ONLY for
 *     registered addresses (zero cost otherwise).
 */
export class I2cTap {
  constructor(chip, bus) {
    this.chip = chip;
    this.bus = bus >>> 0;
    /** addr7 -> {onWrite,onRead,onStop,onStart,readQueue:[],writeLog:[]} */
    this.slaves = new Map();
    // Bus-level handlers are accessor pairs (F1 style: `mcu.i2c0.onStart =
    // …` post-create): assigning ANY of onStart/onWrite/onRead arms the
    // promiscuous Rust hook for the bus (wiring survives reset; assigning
    // null never disarms — physical probes stay soldered).
    let _onStart = null, _onWrite = null, _onRead = null, _onStop = null;
    Object.defineProperties(this, {
      onStart: {
        get: () => _onStart,
        set: (fn) => { _onStart = (typeof fn === 'function') ? fn : null; this._syncHooks(); },
        enumerable: true, configurable: true,
      },
      onWrite: {
        get: () => _onWrite,
        set: (fn) => { _onWrite = (typeof fn === 'function') ? fn : null; this._syncHooks(); },
        enumerable: true, configurable: true,
      },
      onRead: {
        get: () => _onRead,
        set: (fn) => { _onRead = (typeof fn === 'function') ? fn : null; this._syncHooks(); },
        enumerable: true, configurable: true,
      },
      onStop: {
        get: () => _onStop,
        set: (fn) => { _onStop = (typeof fn === 'function') ? fn : null; },
        enumerable: true, configurable: true,
      },
    });
    this._curAddr = null;
    this._curRead = false;
    /** Staged transaction events for async draining (worker split mode). */
    this.events = [];
    this._syncedAddrs = new Set();
    this._promiscSynced = false;
  }

  get _exports() { return this.chip?._wasmLoader?.exports ?? null; }

  /** Push registry + promisc flag to Rust (no-op before loadWasm). */
  _syncHooks() {
    const ex = this._exports;
    if (!ex) return;
    try {
      const promisc = !!(this.onStart || this.onWrite || this.onRead);
      if (promisc !== this._promiscSynced) {
        ex.native_i2c_set_promisc?.(this.bus, promisc ? 1 : 0);
        this._promiscSynced = promisc;
      }
      for (const addr of this.slaves.keys()) {
        if (!this._syncedAddrs.has(addr)) {
          ex.native_i2c_set_host_slave?.(this.bus, addr, 1);
          this._syncedAddrs.add(addr);
        }
      }
      for (const addr of [...this._syncedAddrs]) {
        if (!this.slaves.has(addr)) {
          try { ex.native_i2c_set_host_slave?.(this.bus, addr, 0); } catch {}
          this._syncedAddrs.delete(addr);
        }
      }
    } catch {}
  }

  /**
   * Attach a host slave model at a 7-bit address (post-create safe).
   * handlers: {onWrite?(byte), onRead?()→byte, onStop?(wasRead), onStart?(addr,read)}.
   */
  attachSlave(addr7, handlers = {}) {
    const addr = addr7 & 0x7f;
    let s = this.slaves.get(addr);
    if (!s) {
      s = { onWrite: null, onRead: null, onStop: null, onStart: null, readQueue: [], writeLog: [] };
      this.slaves.set(addr, s);
    }
    for (const k of ['onWrite', 'onRead', 'onStop', 'onStart']) {
      if (typeof handlers[k] === 'function') s[k] = handlers[k];
    }
    this._syncHooks();
    return s;
  }

  detachSlave(addr7) {
    const addr = addr7 & 0x7f;
    this.slaves.delete(addr);
    this._syncHooks();
  }

  clearSlaves() {
    this.slaves.clear();
    this._syncHooks();
  }

  /** Pre-stage bytes a future master READ will consume (sensor ROM, split mode). Survives reset. */
  preloadRead(addr7, bytes) {
    const s = this.attachSlave(addr7);
    for (const b of bytes ?? []) s.readQueue.push(b & 0xff);
    return s.readQueue.length;
  }

  /** Drain bytes the master WROTE to this slave since the last drain. */
  drainWriteLog(addr7) {
    const s = this.slaves.get(addr7 & 0x7f);
    if (!s) return [];
    return s.writeLog.splice(0);
  }

  /** Drain staged transaction events [{t:'start'|'write'|'read'|'stop',...}]. */
  pollEvents() {
    return this.events.splice(0);
  }

  /** Clear in-flight state (called on chip.reset(); wiring + preloads survive). */
  _resetFlight() {
    this._curAddr = null;
    this._curRead = false;
    this.events.length = 0;
  }

  // ---- FFI-facing (called synchronously from wasm-loader; never throws) ----
  _ffiStart(addr, read) {
    this._curAddr = addr & 0x7f;
    this._curRead = !!read;
    this.events.push({ t: 'start', addr: this._curAddr, read: this._curRead });
    if (typeof this.onStart === 'function') {
      try {
        const r = this.onStart(this._curAddr, this._curRead);
        if (r !== undefined) return r ? 1 : 0;
      } catch { return 0; }
    }
    const s = this.slaves.get(this._curAddr);
    if (!s) return 0; // NACK — scan() skips unregistered addresses
    try { s.onStart?.(this._curAddr, this._curRead); } catch {}
    return 1;
  }

  _ffiWrite(byte) {
    const b = byte & 0xff;
    this.events.push({ t: 'write', byte: b });
    if (typeof this.onWrite === 'function') {
      try {
        const r = this.onWrite(b);
        return r === undefined ? 1 : (r ? 1 : 0);
      } catch { return 0; }
    }
    const s = this.slaves.get(this._curAddr);
    if (!s) return 0;
    s.writeLog.push(b);
    if (s.writeLog.length > 4096) s.writeLog.splice(0, s.writeLog.length - 4096);
    try {
      const r = s.onWrite?.(b);
      return r === undefined ? 1 : (r ? 1 : 0);
    } catch { return 0; }
  }

  _ffiRead() {
    if (typeof this.onRead === 'function') {
      try {
        const r = this.onRead();
        if (r !== undefined) {
          const b = r & 0xff;
          this.events.push({ t: 'read', byte: b });
          return b;
        }
      } catch {}
    }
    const s = this.slaves.get(this._curAddr);
    if (s) {
      if (s.readQueue.length) {
        const b = s.readQueue.shift() & 0xff;
        this.events.push({ t: 'read', byte: b });
        return b;
      }
      if (typeof s.onRead === 'function') {
        try {
          const r = s.onRead();
          if (r !== undefined) {
            const b = r & 0xff;
            this.events.push({ t: 'read', byte: b });
            return b;
          }
        } catch {}
      }
    }
    return 0xff;
  }

  _ffiStop(wasRead) {
    this.events.push({ t: 'stop', wasRead: !!wasRead });
    try { this.onStop?.(!!wasRead); } catch {}
    try { this.slaves.get(this._curAddr)?.onStop?.(!!wasRead); } catch {}
    this._curAddr = null;
    this._curRead = false;
  }
}

/**
 * SPI transfer tap for one controller (bus 0..3 = SPI1, SPI0, SPI2, SPI3).
 *
 * Two cooperating styles (documented per esp32-emu.md §2 — EITHER works):
 *  A. onTransfer(txBytes[], recvLen) → rxBytes[] (synchronous, main-thread;
 *     mirrors F1 `mcu.spi1.onTransfer`). Return []/undefined = not claimed
 *     (falls through to the sibling virtual bus, then loopback). Return
 *     recvLen bytes for exact MISO (short reads pack what exists).
 *  B. Split: injectMiso(bytes[]) pre-stages MISO (like F1 injectMiso) and
 *     pollTx()/drainMosi() drains captured MOSI — the Worker-split path
 *     (no synchronous round-trip needed; onTransfer may be absent).
 * Assigning onTransfer OR preloading via injectMiso arms the Rust hook for
 * the unit (wiring survives reset). Fires once per CPU-mode (W-reg)
 * transaction at CMD USR time; DMA and flash-controller transfers excluded.
 */
export class SpiTap {
  constructor(chip, bus) {
    this.chip = chip;
    this.bus = bus >>> 0;
    // Accessor (F1 style: `mcu.spi1.onTransfer = …` post-create): assigning
    // arms the Rust hook for the unit (assigning null never disarms).
    let _onTransfer = null;
    Object.defineProperty(this, 'onTransfer', {
      get: () => _onTransfer,
      set: (fn) => { _onTransfer = (typeof fn === 'function') ? fn : null; this._syncHooks(); },
      enumerable: true, configurable: true,
    });
    /** Pre-staged MISO bytes (device TX memory; survives reset). */
    this.misoQueue = [];
    /** Captured MOSI bytes since last drain (host observability). */
    this.mosiLog = [];
  }

  get _exports() { return this.chip?._wasmLoader?.exports ?? null; }

  _syncHooks() {
    const ex = this._exports;
    if (!ex) return;
    try {
      if (this.onTransfer || this.misoQueue.length) {
        ex.native_spi_set_transfer_hook?.(this.bus, 1);
      }
    } catch {}
  }

  /** Pre-stage MISO bytes for future master reads (split mode). Survives reset. */
  injectMiso(bytes) {
    for (const b of bytes ?? []) {
      this.misoQueue.push(b & 0xff);
      if (this.misoQueue.length > 4096) this.misoQueue.shift();
    }
    this._syncHooks();
    return this.misoQueue.length;
  }

  /** Drain captured MOSI bytes (guest→host) since the last drain. */
  drainMosi(maxBytes = 4096) {
    return this.mosiLog.splice(0, maxBytes);
  }

  /** Alias for drainMosi (bridge-friendly name). */
  pollTx(maxBytes) { return this.drainMosi(maxBytes); }

  /** Clear in-flight/captured state (called on chip.reset(); preloads survive). */
  _resetFlight() {
    this.mosiLog.length = 0;
  }

  // ---- FFI-facing (called synchronously from wasm-loader; never throws) ----
  _ffiTransfer(mosiBytes, recvLen) {
    for (const b of mosiBytes) {
      this.mosiLog.push(b & 0xff);
      if (this.mosiLog.length > 4096) this.mosiLog.shift();
    }
    if (typeof this.onTransfer === 'function') {
      try {
        const r = this.onTransfer(Array.from(mosiBytes), recvLen >>> 0);
        if (Array.isArray(r) && r.length) return r.map((b) => b & 0xff).slice(0, 128);
      } catch {}
    }
    if (this.misoQueue.length) {
      const n = Math.min(recvLen >>> 0, this.misoQueue.length, 128);
      return this.misoQueue.splice(0, n);
    }
    return [];
  }
}
