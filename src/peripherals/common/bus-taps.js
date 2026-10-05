// Host peripheral taps (esp32-emu.md §1-4, §9, §11) — JS models for I2C slaves,
// SPI transfers, I2S TX capture and OneWire slaves, plus the registry sync
// that arms the Rust FFI hooks.
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
    /** Capture-only arming (injectMiso called, even empty — the host is
     * observing a write-only device like MAX7219 and stages no MISO). */
    this._captureArmed = false;
  }

  get _exports() { return this.chip?._wasmLoader?.exports ?? null; }

  _syncHooks() {
    const ex = this._exports;
    if (!ex) return;
    try {
      if (this.onTransfer || this.misoQueue.length || this._captureArmed) {
        ex.native_spi_set_transfer_hook?.(this.bus, 1);
      }
    } catch {}
  }

  /** Pre-stage MISO bytes for future master reads (split mode). Survives reset.
   * Calling with an empty array arms capture-only observation (no MISO). */
  injectMiso(bytes) {
    this._captureArmed = true;
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

/**
 * Dallas CRC8 (poly 0x8C reversed, init 0) — the DS18B20 scratchpad/ROM CRC.
 * Firmware (DallasTemperature) rejects reads with a bad CRC, so the model
 * must serve authentic bytes.
 */
export function dallasCrc8(bytes) {
  let crc = 0;
  for (const byte of bytes ?? []) {
    let b = byte & 0xff;
    for (let i = 0; i < 8; i++) {
      const mix = (crc ^ b) & 1;
      crc >>>= 1;
      if (mix) crc ^= 0x8c;
      b >>>= 1;
    }
  }
  return crc & 0xff;
}

/**
 * OneWire slave tap — DS18B20-compatible subset (esp32-emu.md §11, P1).
 *
 * Why a synchronous edge responder instead of host polling: OneWire slots
 * are 60–120us and the host SAB poll runs at 50ms wall — polling can NEVER
 * react in-slot. Instead the model runs synchronously inside the guest's
 * own MMIO writes (Rust js_gpio_edge / js_gpio_read_override FFI on
 * sim-time APB ticks): edges are observed with exact tick stamps, and IN
 * reads are answered as a pure function of (edge history, now). Wall speed
 * is irrelevant; busy-spun firmware keeps sim:wall ≈ 1 anyway.
 *
 * Supported: reset + presence pulse, SKIP ROM (0xCC), READ ROM (0x33),
 * CONVERT T (0x44), READ SCRATCHPAD (0xBE, 9 bytes incl. CRC). Single-drop
 * only (no SEARCH ROM — document the limit; enough for DallasTemperature
 * cells with one sensor). LSB-first, like the wire.
 *
 * Timing (us): reset low ≥ 480 → presence 0 for ≤ 250 after release;
 * slot release < 30 (after a >0 fall) → 1-bit / read-slot start;
 * release ≥ 30 (and < 480) → 0-bit. Read sampling is the master's business
 * (it reads IN at +15us); the model just serves the next TX bit.
 */
export class OneWireDevice {
  constructor(pin) {
    this.pin = pin >>> 0;
    this.rom = [0x28, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x00];
    this.rom[7] = dallasCrc8(this.rom.slice(0, 7));
    this.scratchpad = [0x50, 0x05, 0x4b, 0x46, 0x7f, 0xff, 0x0c, 0x10, 0x00];
    this._fixScratchCrc();
    this._resetState();
    /** Decoded master traffic for host verification: [{t:'presence'} | {t:'byte', byte}] */
    this.log = [];
    this.presenceCount = 0;
  }

  _fixScratchCrc() {
    this.scratchpad[8] = dallasCrc8(this.scratchpad.slice(0, 8));
  }

  /** Set temperature in °C (12-bit, 1/16th units, two's complement). */
  setTemperature(celsius) {
    const raw = Math.max(-880, Math.min(880, Math.round(Number(celsius) * 16)));
    const u = raw & 0xffff;
    this.scratchpad[0] = u & 0xff;
    this.scratchpad[1] = (u >> 8) & 0xff;
    this._fixScratchCrc();
    return raw / 16;
  }

  /** Replace the scratchpad (8 bytes → CRC recomputed; 9 bytes used as-is). */
  preloadScratch(bytes) {
    const arr = Array.from(bytes ?? []).slice(0, 9).map((b) => b & 0xff);
    for (let i = 0; i < Math.min(8, arr.length); i++) this.scratchpad[i] = arr[i];
    if (arr.length >= 9) this.scratchpad[8] = arr[8];
    else this._fixScratchCrc();
  }

  _resetState() {
    this.state = 'idle'; // idle | presence | cmd | serving
    this.lastFallTick = -1;
    this.cmdByte = 0;
    this.cmdBits = 0;
    this.txBytes = [];
    this.txBit = 0;
  }

  /** Clear transient parse state (chip.reset(); ROM/scratchpad survive). */
  _resetFlight() {
    this._resetState();
    this.log.length = 0;
    this.presenceCount = 0;
  }

  _usSince(tick, ref) {
    return (((tick >>> 0) - (ref >>> 0)) >>> 0) / 80;
  }

  // ---- FFI-facing (synchronous, never throws) ----
  /** Edge notify: level 0 = fall, 1 = drive-high, 2 = release-to-float. tick = APB. */
  _ffiEdge(level, tick) {
    tick >>>= 0;
    if (level === 0) {
      this.lastFallTick = tick;
      return;
    }
    // Release (or driven high — the master only ever releases high here).
    if (this.lastFallTick < 0) return;
    const lowUs = this._usSince(tick, this.lastFallTick);
    this.lastFallTick = -1;
    if (lowUs >= 480) {
      // Reset pulse → presence armed.
      this.state = 'presence';
      this.resetTick = tick;
      this.presenceCount++;
      this.log.push({ t: 'presence' });
      this.cmdByte = 0;
      this.cmdBits = 0;
      return;
    }
    if (lowUs < 30) this._slotBit(1);
    else this._slotBit(0);
  }

  _slotBit(bit) {
    if (this.state === 'serving') {
      // Read slot: the value is ours (served on the following IN read);
      // nothing to accumulate — but a stray write-slot here would corrupt
      // framing, so ignore short releases while serving (the read consumes).
      return;
    }
    if (this.state !== 'cmd' && this.state !== 'presence') return;
    this.state = 'cmd';
    this.cmdByte |= (bit & 1) << this.cmdBits;
    this.cmdBits++;
    if (this.cmdBits >= 8) {
      const byte = this.cmdByte & 0xff;
      this.log.push({ t: 'byte', byte });
      this.cmdByte = 0;
      this.cmdBits = 0;
      this._dispatch(byte);
    }
  }

  _dispatch(byte) {
    if (byte === 0xcc) { this.state = 'cmd'; } // SKIP ROM → next byte is a function
    else if (byte === 0x44) { this.state = 'idle'; } // CONVERT T (instant in emulation)
    else if (byte === 0xbe) { this._serve(this.scratchpad); } // READ SCRATCHPAD
    else if (byte === 0x33) { this._serve(this.rom); } // READ ROM
    else { this.state = 'idle'; } // unknown → idle (real slaves ignore)
  }

  _serve(bytes) {
    this.txBytes = Array.from(bytes);
    this.txBit = 0;
    this.state = 'serving';
  }

  /** IN-read override: pure function of (state, now). 2 = no override. */
  _ffiRead(tick) {
    tick >>>= 0;
    if (this.state === 'presence') {
      // Presence pulse: 0 while within the window, bus-high after — but the
      // session STAYS armed until command bits arrive or a new reset comes.
      // (Disarming here would drop the ROM/function bytes the master sends
      // right after sampling presence — the normal Dallas flow.)
      if (this.resetTick !== undefined && this._usSince(tick, this.resetTick) <= 250) return 0;
      return 1;
    }
    if (this.state === 'serving') {
      if (this.txBit >= this.txBytes.length * 8) {
        this.state = 'idle';
        return 1;
      }
      const bit = (this.txBytes[this.txBit >> 3] >> (this.txBit & 7)) & 1;
      this.txBit++;
      return bit;
    }
    return 2; // no override — use latched input
  }

  /** Drain decoded master traffic + presence count for host verification. */
  pollLog() {
    const log = this.log.splice(0);
    const presence = this.presenceCount;
    this.presenceCount = 0;
    return { log, presence };
  }
}
