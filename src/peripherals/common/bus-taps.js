// Host peripheral taps (esp32-emu.md §1-4, §9, §11) — JS models for I2C slaves,
// SPI transfers, I2S TX capture, OneWire slaves, RMT single-wire sensors
// (DHT22) and OV2640 camera sensors, plus the registry sync that arms the
// Rust FFI hooks.
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

/**
 * RMT tap — host single-wire sensor models (DHT22 et al).
 *
 * Transport only (the sensor model lives host-side): the engine reports
 * completed guest TX item streams (the start pulse a DHT22 answers) and
 * the host stages response item streams for RX-armed channels.
 *
 * Item word format (ESP32 RMT item = 32 bits, channel-RAM order):
 *   bits[14:0] duration0 (ticks), [15] level0, [30:16] duration1, [31] level1.
 * A zero word terminates the stream. Ticks are channel-clock ticks — use
 * getRmtTickHz(ch) (APB/ref ÷ divider) for µs↔tick conversion (DHT22 slots
 * are specified in µs: start ≥1ms low, response 80µs+80µs, bit = 50µs low
 * + 26µs (0) / 70µs (1) high).
 *
 * Two cooperating directions (EITHER works):
 *  A. Capture: onRmtTx(ch, cb) + pollRmtTx(ch?) drains guest→host TX items
 *     (incl. terminator). Arming is per-channel (native_rmt_set_tx_hook);
 *     unarmed channels keep the loopback path bit-for-bit (zero FFI).
 *  B. Inject: injectRmtRx(ch, words[]) stages host→guest RX items. Injected
 *     while the channel is RX-armed → delivers immediately (RAM +
 *     MEM_OWNER + RX_END, like loopback); injected early → stays pending
 *     and delivers on the next RX_EN arm (sensor-answers-start parity).
 *     Pending = preload: survives reset.
 */
export class RmtTap {
  constructor(chip) {
    this.chip = chip;
    /** Per-channel capture queues (ch 0..7 -> u32 item words). Cleared on reset. */
    this.txQueue = [[], [], [], [], [], [], [], []];
    /** Per-channel listeners: [{cb}] — wiring survives reset. */
    this.txListeners = [[], [], [], [], [], [], [], []];
    /** Pre-WASM inject staging (ch -> u32[]), flushed at _syncHooks. */
    this._localPending = new Map();
    this._syncedCh = new Set();
  }

  get _exports() { return this.chip?._wasmLoader?.exports ?? null; }

  /** Push per-channel hook flags to Rust (no-op before loadWasm). */
  _syncHooks() {
    const ex = this._exports;
    if (!ex) return;
    try {
      for (let ch = 0; ch < 8; ch++) {
        if (this.txListeners[ch].length && !this._syncedCh.has(ch)) {
          try { ex.native_rmt_set_tx_hook?.(ch, 1); } catch {}
          this._syncedCh.add(ch);
        }
      }
    } catch {}
    // Flush pre-WASM inject staging through the real path (failure-atomic:
    // unstaged tails stay queued for the next sync).
    for (const [ch, words] of [...this._localPending]) {
      this._localPending.delete(ch);
      let staged = 0;
      try { staged = this._injectLive(ch, words); } catch { staged = 0; }
      if (staged < words.length) {
        const prev = this._localPending.get(ch) ?? [];
        this._localPending.set(ch, words.slice(staged).concat(prev).slice(-64));
      }
    }
  }

  /**
   * Subscribe to guest→host TX items on a channel. Returns an unsubscribe
   * fn. Listener MUST NOT re-enter the engine (record-only). cb(ch, items, tickHz).
   */
  onRmtTx(ch, cb) {
    ch >>>= 0;
    if (ch >= 8 || typeof cb !== 'function') return () => {};
    this.txListeners[ch].push(cb);
    this._syncHooks();
    return () => {
      const i = this.txListeners[ch].indexOf(cb);
      if (i >= 0) this.txListeners[ch].splice(i, 1);
    };
  }

  /** Drain staged TX items: pollRmtTx() → all channels, pollRmtTx(ch) → one. */
  pollRmtTx(ch = null) {
    const out = [];
    const chs = ch === null ? [0, 1, 2, 3, 4, 5, 6, 7] : [ch >>> 0];
    for (const c of chs) {
      if (c >= 8) continue;
      const q = this.txQueue[c];
      if (q?.length) {
        const items = q.splice(0);
        const tickHz = this.getRmtTickHz(c);
        out.push({ ch: c, items, tickHz });
        for (const cb of [...this.txListeners[c]]) {
          try { cb(c, items, tickHz); } catch (e) { console.warn(`[RMT-TAP] listener threw: ${e?.message || e}`); }
        }
      }
    }
    return out;
  }

  /** FFI sink for js_rmt_tx_items (never throws; bounded staging). */
  _onRmtTxData(ch, words) {
    try {
      const q = this.txQueue[ch >>> 0];
      if (!q) return;
      for (const w of words) {
        q.push(w >>> 0);
        if (q.length > 4096) q.shift();
      }
    } catch {}
  }

  /**
   * Stage host→guest RX items (raw u32 item words, ≤ 64) for a channel.
   * @returns words staged.
   */
  injectRmtRx(ch, words) {
    ch >>>= 0;
    const arr = Array.from(words ?? []).map((w) => w >>> 0).slice(0, 64);
    if (ch >= 8 || arr.length === 0) return 0;
    if (!this._exports) {
      const prev = this._localPending.get(ch) ?? [];
      this._localPending.set(ch, prev.concat(arr).slice(-64));
      return arr.length;
    }
    this._syncHooks();
    return this._injectLive(ch, arr);
  }

  _injectLive(ch, arr) {
    try {
      const ex = this._exports;
      const scratch = ex.native_rmt_rx_scratch_ptr?.() >>> 0;
      if (!scratch || !ex.native_rmt_rx_inject) return 0;
      const mem = this.chip._wasmLoader.memoryBuffer;
      const dv = new DataView(mem);
      for (let i = 0; i < arr.length; i++) dv.setUint32(scratch + i * 4, arr[i] >>> 0, true);
      return ex.native_rmt_rx_inject(ch >>> 0, scratch, arr.length) >>> 0;
    } catch { return 0; }
  }

  /** Channel tick rate in Hz (0 when WASM absent). */
  getRmtTickHz(ch) {
    try { return this._exports?.native_rmt_tick_hz?.(ch >>> 0) >>> 0 || 0; }
    catch { return 0; }
  }

  /** Clear captured (in-flight) state (chip.reset(); wiring + preloads survive). */
  _resetFlight() {
    for (const q of this.txQueue) q.length = 0;
  }

  /** Encode one RMT item word from two (duration-ticks, level) halves. */
  static item(d0, l0, d1, l1) {
    return (((d1 & 0x7fff) << 16) | ((l1 ? 1 : 0) << 31) | ((d0 & 0x7fff)) | ((l0 ? 1 : 0) << 15)) >>> 0;
  }

  /** Decode one RMT item word into {d0, l0, d1, l1}. */
  static splitItem(w) {
    w >>>= 0;
    return { d0: w & 0x7fff, l0: (w >>> 15) & 1, d1: (w >>> 16) & 0x7fff, l1: (w >>> 31) & 1 };
  }

  /** Convert microseconds to channel ticks at hz (rounded, ≥ 1 when us > 0). */
  static usToTicks(us, hz) {
    if (!(us > 0) || !(hz > 0)) return 0;
    return Math.max(1, Math.round((us * hz) / 1e6));
  }
}

/**
 * OV2640 camera sensor model (SCCB control + DVP frame expectation).
 *
 * Two halves, matching the engine split:
 *  - SCCB (control): the sensor is an I2C-compatible slave at 0x30.
 *    attachToI2c(i2cTap) wires the register-pointer protocol (first write
 *    byte after START = register address, following bytes = data with
 *    auto-increment; repeated-START reads serve regs[ptr++]).
 *    Commit points are STOPs; drainSccbLog() exposes the init sequence
 *    the firmware programmed for host verification.
 *  - DVP (pixels): the model does NOT push pixels — the engine's virtual
 *    sensor emits the deterministic ramp (byte[i] = i & 0xFF, one byte per
 *    DMA element in SM_0A00_0B00 sample1). expectedFrame() reproduces that
 *    ramp for host-side frame verification; frameBytes() sizes captures.
 *
 * Register file: PID reset values 0x0A=0x26/0x0B=0x42 (what esp-camera
 * checks); everything else resets 0x00. Device memory survives reset;
 * transaction pointer state + SCCB log clear in _resetFlight().
 */
export class Ov2640 {
  constructor() {
    /** 7-bit SCCB address (0x30 write / 0x31 read). */
    this.addr = 0x30;
    this.regs = new Uint8Array(256);
    this.regs[0x0a] = 0x26;
    this.regs[0x0b] = 0x42;
    this.width = 160;
    this.height = 120;
    this.format = 'RGB565';
    this._ptr = 0;
    this._havePtr = false;
    this._curReg = -1;
    this._curBytes = [];
    /** Committed SCCB writes since last drain: [{reg, bytes:[...]}]. */
    this.sccbLog = [];
    this._tap = null;
    this._entry = null;
  }

  /** Attach to an I2C tap's bus as an SCCB slave (post-create safe). */
  attachToI2c(i2cTap, addr7 = 0x30) {
    this.addr = addr7 & 0x7f;
    this._tap = i2cTap;
    this._entry = i2cTap.attachSlave(this.addr, {
      onWrite: (b) => this._sccbWrite(b & 0xff),
      onRead: () => this._sccbRead(),
      onStop: (wasRead) => this._sccbStop(!!wasRead),
    });
    return this._entry;
  }

  detachFromI2c() {
    try { this._tap?.detachSlave?.(this.addr); } catch {}
    this._tap = null;
    this._entry = null;
  }

  _sccbWrite(b) {
    if (!this._havePtr) {
      this._ptr = b & 0xff;
      this._havePtr = true;
      this._curReg = this._ptr;
      this._curBytes = [];
    } else {
      this.regs[this._ptr] = b & 0xff;
      this._curBytes.push(b & 0xff);
      this._ptr = (this._ptr + 1) & 0xff;
    }
    return true; // ACK
  }

  _sccbRead() {
    const b = this.regs[this._ptr] & 0xff;
    this._ptr = (this._ptr + 1) & 0xff;
    return b;
  }

  _sccbStop() {
    // Commit data-bearing writes (pure pointer writes select, not program).
    if (this._havePtr && this._curBytes.length) {
      this.sccbLog.push({ reg: this._curReg & 0xff, bytes: this._curBytes.splice(0) });
      if (this.sccbLog.length > 1024) this.sccbLog.splice(0, this.sccbLog.length - 1024);
    }
    this._havePtr = false;
    this._curReg = -1;
    this._curBytes = [];
  }

  /** Drain committed SCCB register writes for init-sequence verification. */
  drainSccbLog() {
    return this.sccbLog.splice(0);
  }

  /** Set the capture resolution (sensor window the cell asserts). */
  setResolution(w, h) {
    this.width = w >>> 0;
    this.height = h >>> 0;
  }

  /** Set the pixel format ('RGB565' | 'YUV422' | 'GRAYSCALE'). */
  setFormat(f) {
    if (f !== 'RGB565' && f !== 'YUV422' && f !== 'GRAYSCALE') {
      throw new Error(`Ov2640: unsupported format '${f}' (use RGB565, YUV422, GRAYSCALE)`);
    }
    this.format = f;
  }

  /** Expected frame size in sensor bytes (one byte per DMA element). */
  frameBytes() {
    if (this.format === 'GRAYSCALE') return this.width * this.height;
    return this.width * this.height * 2; // RGB565 / YUV422
  }

  /**
   * Expected frame bytes: the deterministic virtual-sensor ramp
   * (byte[i] = i & 0xFF) — engine parity (i2c_i2s.rs camera feed), so a
   * firmware cell can assert fb contents byte-exact.
   */
  expectedFrame() {
    const n = this.frameBytes();
    const out = new Uint8Array(n);
    for (let i = 0; i < n; i++) out[i] = i & 0xff;
    return out;
  }

  /** Clear transaction state + SCCB log (chip.reset(); regs/resolution survive). */
  _resetFlight() {
    this._ptr = 0;
    this._havePtr = false;
    this._curReg = -1;
    this._curBytes = [];
    this.sccbLog.length = 0;
  }
}
