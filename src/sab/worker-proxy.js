// Main-thread proxy for the Worker-bound simulator
// Communication via SharedArrayBuffer only (no postMessage for runtime data)

const SAB_SLOT_RUN = 0;
const SAB_SLOT_BUDGET = 1;
const SAB_SLOT_NANOS_LO = 2;
const _SAB_SLOT_STATUS = 3;
const SAB_SLOT_PC = 4;
const SAB_SLOT_STUCK = 5;
const SAB_SLOT_IDLE = 6;
const SAB_SLOT_PROGRESS_INTERVAL = 7;
const SAB_SLOT_NANOS_HI = 8;
const SAB_SLOT_CMD = 9;
const SAB_SLOT_CMD_ARG0 = 10;
const SAB_SLOT_CMD_ARG1 = 11;
const SAB_SLOT_RESP = 12;
const SAB_SLOT_ERR_FLAG = 13;
const SAB_SLOT_CPU_FREQ = 14;
const SAB_SLOT_WIFI_STATE = 15;
const SAB_SLOT_WIFI_TX_FRAMES = 16;
const SAB_SLOT_WIFI_TX_BYTES = 17;
const SAB_SLOT_WIFI_RX_FRAMES = 18;
const SAB_SLOT_WIFI_RX_BYTES = 19;
const SAB_SLOT_WIFI_PROBES = 20;
const SAB_SLOT_CMD_ARG2 = 21;

const CMD_NONE = 0;
const CMD_RUN = 1;
const CMD_RESET = 2;
const CMD_SEED_MMU = 3;
const CMD_WRITE_UINT32 = 4;
const CMD_READ_MEMORY = 5;
const CMD_GET_PCAP = 6;
const CMD_GET_WIFI_STATS = 7;
const CMD_STEP = 8;
const CMD_GET_FFI_COUNTS = 10;
const CMD_READ_MMIO = 11;
const CMD_UART_RX = 12;
const CMD_SET_PIN_INPUT = 13;
const CMD_SET_TOUCH_INPUT = 14;
const CMD_SET_VOLTAGE = 15;
const CMD_FEED_I2S_RX = 16;
const CMD_SEND_TWAI = 17;
const CMD_PUSH_TWAI = 18;
const CMD_GET_TWAI_TX = 19;
const CMD_WATCHPOINT = 20;
const CMD_PCTRACE = 21;
const CMD_PRESS_RESET = 22;
const CMD_PRESS_BOOT = 23;

const RESP_IDLE = 0;
const _RESP_DONE = 1;
const _RESP_ERROR = 2;

const SAB_NUM_SLOTS = 22;
const UART_RING_SIZE = 16384;
const UART_CTRL_INTS = 2;
const READ_RESP_SIZE = 65536;
const ERR_SAB_SIZE = 256;

// Track live worker threads so we can terminate them on process shutdown.
// Without this, a SimulatorWorker that is never .stop()'d keeps its WASM
// instance + SharedArrayBuffers alive for the whole process lifetime (leak).
const _liveWorkers = new Set();
let _shutdownHandlersInstalled = false;

function _installShutdownHandlers() {
  if (_shutdownHandlersInstalled || typeof process === "undefined" || !process.on) return;
  _shutdownHandlersInstalled = true;
  const cleanup = () => {
    for (const w of _liveWorkers) { try { w.terminate(); } catch (_) {} }
    _liveWorkers.clear();
  };
  process.on("SIGINT", cleanup);
  process.on("SIGTERM", cleanup);
}

/**
 * Main-thread proxy for the Worker-bound ESP32 simulator.
 * All runtime data exchange uses SharedArrayBuffers (no postMessage for data).
 */
class SimulatorWorker {
  constructor() {
    /** @type {Worker|null} */ this.worker = null;
    /** @type {SharedArrayBuffer|null} */ this.ctrlSab = null;
    /** @type {Int32Array|null} */ this.ctrl = null;
    /** @type {SharedArrayBuffer|null} */ this.uartSab = null;
    /** @type {Int32Array|null} */ this.uartCtrl = null;
    /** @type {Uint8Array|null} */ this.uartRing = null;
    /** @type {SharedArrayBuffer|null} */ this.readRespSab = null;
    /** @type {Uint8Array|null} */ this.readResp = null;
    /** @type {SharedArrayBuffer|null} */ this.errSab = null;
    /** @type {Uint8Array|null} */ this.errMsg = null;
    /** @type {boolean} */ this._ready = false;
    /** @type {any} */ this._chipInfo = {};
    /** @type {(value: number) => void|null} */ this._onUART = null;
    /** @type {((value?: any) => void)|null} */ this._onReady = null;
    /** @type {((err: Error) => void)|null} */ this._onError = null;
    /** @type {any} */ this.memory = {};
    /** @type {'auto'|'tight'|'chunked'} */ this._simMode = 'auto';
    /** @type {number} */ this._chunkSize = 500000;
  }

  /** @returns {'auto'|'tight'|'chunked'} */
  get simMode() { return this._simMode; }
  /** @param {'auto'|'tight'|'chunked'} mode */
  set simMode(mode) {
    if (mode === 'tight' || mode === 'chunked' || mode === 'auto') this._simMode = mode;
  }

  /** @returns {number} */
  get chunkSize() { return this._chunkSize; }
  /** @param {number} n */
  set chunkSize(n) {
    if (typeof n === 'number' && n > 0 && Number.isFinite(n)) this._chunkSize = Math.floor(n);
  }

  /**
   * Initialize the simulator worker.
   * @param {string} chipType - Chip type (e.g. 'ESP32')
   * @param {any} config - Configuration (flashSizeMB, mmuPages, strapValue, budget, partitions, etc.)
   * @param {Uint8Array|null} flash - Flash image (SharedArrayBuffer-backed preferred)
   * @param {Uint8Array|null} rom - Boot ROM bytes
   * @returns {Promise<void>} Resolves when worker is ready
   */
  async init(chipType, config = {}, flash = null, rom = null) {
    if (this.worker) this.terminate();

    this.ctrlSab = new SharedArrayBuffer(SAB_NUM_SLOTS * 4);
    this.ctrl = new Int32Array(this.ctrlSab);
    Atomics.store(this.ctrl, SAB_SLOT_RUN, 0);
    Atomics.store(this.ctrl, SAB_SLOT_BUDGET, config.budget || 50000);
    Atomics.store(this.ctrl, SAB_SLOT_PROGRESS_INTERVAL, config.progressInterval || 5000000);
    Atomics.store(this.ctrl, SAB_SLOT_CMD, CMD_NONE);
    Atomics.store(this.ctrl, SAB_SLOT_RESP, RESP_IDLE);
    Atomics.store(this.ctrl, SAB_SLOT_ERR_FLAG, 0);

    this.uartSab = new SharedArrayBuffer(UART_CTRL_INTS * 4 + UART_RING_SIZE);
    this.uartCtrl = new Int32Array(this.uartSab, 0, 2);
    this.uartCtrl[0] = 0;
    this.uartCtrl[1] = 0;
    this.uartRing = new Uint8Array(this.uartSab, UART_CTRL_INTS * 4, UART_RING_SIZE);

    this.uartRxSab = new SharedArrayBuffer(UART_CTRL_INTS * 4 + UART_RING_SIZE);
    this.uartRxCtrl = new Int32Array(this.uartRxSab, 0, 2);
    this.uartRxCtrl[0] = 0;
    this.uartRxCtrl[1] = 0;
    this.uartRxRing = new Uint8Array(this.uartRxSab, UART_CTRL_INTS * 4, UART_RING_SIZE);

    this.readRespSab = new SharedArrayBuffer(READ_RESP_SIZE);
    this.readResp = new Uint8Array(this.readRespSab);

    this.errSab = new SharedArrayBuffer(ERR_SAB_SIZE);
    this.errMsg = new Uint8Array(this.errSab);
    this.debugSab = new SharedArrayBuffer(772 * 4);
    this.debug = new Uint32Array(this.debugSab);

    let WorkerCtor;
    if (typeof Worker !== "undefined") {
      WorkerCtor = Worker;
    } else {
      WorkerCtor = (await import("worker_threads")).Worker;
    }
    const url = new URL("./worker-entry.js", import.meta.url);
    this.worker = new WorkerCtor(url, { type: "module" });
    _liveWorkers.add(this.worker);
    _installShutdownHandlers();

    if (typeof this.worker.onmessage !== "undefined") {
      this.worker.onmessage = this._onMessage.bind(this);
    } else {
      // Node.js worker_threads.Worker — has .on() but TS only sees browser Worker type.
      const w = /** @type {any} */ (this.worker);
      w.on("message", this._onMessage.bind(this));
      w.on("error", (err) => {
        if (this._onError) this._onError(err);
      });
    }

    if (flash && flash.buffer instanceof SharedArrayBuffer) {
      /** @type {any} */ (this.memory).flash = new Uint8Array(flash.buffer, flash.byteOffset, flash.byteLength);
    }
    if (rom && rom.buffer instanceof SharedArrayBuffer) {
      /** @type {any} */ (this.memory).chipROM = new Uint8Array(rom.buffer, rom.byteOffset, rom.byteLength);
    }

    await new Promise((resolve, reject) => {
      this._onReady = resolve;
      this._onError = reject;
      this.worker.postMessage({
        type: "init", chipType,
        config: { ...config, simMode: this._simMode, chunkSize: this._chunkSize },
        flash, rom,
        sab: this.ctrlSab, uartSab: this.uartSab, uartRxSab: this.uartRxSab,
        readRespSab: this.readRespSab, errSab: this.errSab,
        debugSab: this.debugSab,
      });
    });
    this._ready = true;
  }

  // loadROM is superseded by direct SAB write to proxy.memory.chipROM
  //   proxy.memory.chipROM.set(romBytes, 0) — instantly visible to worker

  /** @param {number} [offset=0] Seed MMU page table from flash data */
  async seedMMU(offset = 0) {
    this._checkReady();
    Atomics.store(this.ctrl, SAB_SLOT_CMD_ARG0, offset);
    Atomics.store(this.ctrl, SAB_SLOT_RESP, RESP_IDLE);
    Atomics.store(this.ctrl, SAB_SLOT_CMD, CMD_SEED_MMU);
    Atomics.notify(this.ctrl, SAB_SLOT_CMD, 1);
    while (Atomics.load(this.ctrl, SAB_SLOT_RESP) === RESP_IDLE) {} // eslint-disable-line no-empty
  }

  /** Reset the chip to initial state */
  async reset() {
    this._checkReady();
    Atomics.store(this.ctrl, SAB_SLOT_RESP, RESP_IDLE);
    Atomics.store(this.ctrl, SAB_SLOT_CMD, CMD_RESET);
    Atomics.notify(this.ctrl, SAB_SLOT_CMD, 1);
    while (Atomics.load(this.ctrl, SAB_SLOT_RESP) === RESP_IDLE) {} // eslint-disable-line no-empty
  }

  /**
   * Tap the RESET (EN) button: full chip reset, like pulling CHIP_PU low
   * on a real dev board (digital + RTC state cleared, flash/RTC-memory
   * preserved). Works mid-run (serviced inline in the sim loop).
   */
  async pressResetButton() {
    this._checkReady();
    Atomics.store(this.ctrl, SAB_SLOT_RESP, RESP_IDLE);
    Atomics.store(this.ctrl, SAB_SLOT_CMD, CMD_PRESS_RESET);
    Atomics.notify(this.ctrl, SAB_SLOT_CMD, 1);
    while (Atomics.load(this.ctrl, SAB_SLOT_RESP) === RESP_IDLE) {} // eslint-disable-line no-empty
  }

  /**
   * Hold/release the BOOT button (GPIO0 strapping): drives GPIO0's live
   * input level + the BOOT strap bit (bit4 of the strap value), so a
   * subsequent reset samples download mode like holding BOOT on a real
   * dev board. Works mid-run (serviced inline in the sim loop).
   * @param {boolean} held - true = button held down, false = released
   */
  async pressBootButton(held) {
    this._checkReady();
    Atomics.store(this.ctrl, SAB_SLOT_CMD_ARG0, held ? 1 : 0);
    Atomics.store(this.ctrl, SAB_SLOT_RESP, RESP_IDLE);
    Atomics.store(this.ctrl, SAB_SLOT_CMD, CMD_PRESS_BOOT);
    Atomics.notify(this.ctrl, SAB_SLOT_CMD, 1);
    while (Atomics.load(this.ctrl, SAB_SLOT_RESP) === RESP_IDLE) {} // eslint-disable-line no-empty
  }

  /** Write a 32-bit value to core 0's address space */
  async writeUint32(addr, value) {
    this._checkReady();
    Atomics.store(this.ctrl, SAB_SLOT_CMD_ARG0, addr);
    Atomics.store(this.ctrl, SAB_SLOT_CMD_ARG1, value);
    Atomics.store(this.ctrl, SAB_SLOT_RESP, RESP_IDLE);
    Atomics.store(this.ctrl, SAB_SLOT_CMD, CMD_WRITE_UINT32);
    Atomics.notify(this.ctrl, SAB_SLOT_CMD, 1);
    while (Atomics.load(this.ctrl, SAB_SLOT_RESP) === RESP_IDLE) {} // eslint-disable-line no-empty
  }

  /** Step N instructions on core 0 */
  async step(count = 1) {
    this._checkReady();
    Atomics.store(this.ctrl, SAB_SLOT_CMD_ARG0, count);
    Atomics.store(this.ctrl, SAB_SLOT_RESP, RESP_IDLE);
    Atomics.store(this.ctrl, SAB_SLOT_CMD, CMD_STEP);
    Atomics.notify(this.ctrl, SAB_SLOT_CMD, 1);
    while (Atomics.load(this.ctrl, SAB_SLOT_RESP) === RESP_IDLE) {} // eslint-disable-line no-empty
  }

  /**
   * Drive a GPIO input level at runtime (host-driven pin toggle).
   * Routes through the native GPIO matrix to PCNT/RMT/etc. inputs.
   */
  async setPinInput(pin, level) {
    this._checkReady();
    Atomics.store(this.ctrl, SAB_SLOT_CMD_ARG0, pin);
    Atomics.store(this.ctrl, SAB_SLOT_CMD_ARG1, level ? 1 : 0);
    Atomics.store(this.ctrl, SAB_SLOT_RESP, RESP_IDLE);
    Atomics.store(this.ctrl, SAB_SLOT_CMD, CMD_SET_PIN_INPUT);
    Atomics.notify(this.ctrl, SAB_SLOT_CMD, 1);
    while (Atomics.load(this.ctrl, SAB_SLOT_RESP) === RESP_IDLE) {} // eslint-disable-line no-empty
  }

  /**
   * Drive a touch pad count at runtime (host-driven touch).
   * Updates the JS touchCounts read by the native SENS Touch engine.
   */
  async setTouchInput(pad, count) {
    this._checkReady();
    Atomics.store(this.ctrl, SAB_SLOT_CMD_ARG0, pad);
    Atomics.store(this.ctrl, SAB_SLOT_CMD_ARG1, count);
    Atomics.store(this.ctrl, SAB_SLOT_RESP, RESP_IDLE);
    Atomics.store(this.ctrl, SAB_SLOT_CMD, CMD_SET_TOUCH_INPUT);
    Atomics.notify(this.ctrl, SAB_SLOT_CMD, 1);
    while (Atomics.load(this.ctrl, SAB_SLOT_RESP) === RESP_IDLE) {} // eslint-disable-line no-empty
  }

  /**
   * Drive the supply rail in millivolts at runtime (brownout detector).
   */
  async setVoltageMv(mv) {
    this._checkReady();
    Atomics.store(this.ctrl, SAB_SLOT_CMD_ARG0, mv);
    Atomics.store(this.ctrl, SAB_SLOT_RESP, RESP_IDLE);
    Atomics.store(this.ctrl, SAB_SLOT_CMD, CMD_SET_VOLTAGE);
    Atomics.notify(this.ctrl, SAB_SLOT_CMD, 1);
    while (Atomics.load(this.ctrl, SAB_SLOT_RESP) === RESP_IDLE) {} // eslint-disable-line no-empty
  }

  /**
   * Push one u32 audio sample word into I2S0's RX buffer (virtual microphone).
   * Serviced inline during run; call repeatedly to stream, or bulk-feed
   * before run() (buffer persists until the RX DMA drains it).
   */
  async feedI2SRX(sample) {
    this._checkReady();
    Atomics.store(this.ctrl, SAB_SLOT_CMD_ARG0, sample >>> 0);
    Atomics.store(this.ctrl, SAB_SLOT_RESP, RESP_IDLE);
    Atomics.store(this.ctrl, SAB_SLOT_CMD, CMD_FEED_I2S_RX);
    Atomics.notify(this.ctrl, SAB_SLOT_CMD, 1);
    while (Atomics.load(this.ctrl, SAB_SLOT_RESP) === RESP_IDLE) {} // eslint-disable-line no-empty
  }

  /**
   * Inject a virtual-CAN peer frame (peer -> ESP32). The frame lands in the
   * TWAI RX path through the real acceptance filter + RX interrupt.
   * Two-phase over the 3-arg CMD channel: STAGE (id/flags/data0-3) then
   * PUSH (data4-7 + deliver).
   * @param {number} id - 11-bit standard or 29-bit extended identifier
   * @param {number[]|Uint8Array} data - up to 8 payload bytes
   * @param {object} [opts] - { ext: boolean, rtr: boolean }
   */
  async sendTwaiFrame(id, data, opts = {}) {
    this._checkReady();
    const bytes = Array.from(data || []).slice(0, 8);
    const flags = bytes.length | ((opts.ext ? 1 : 0) << 8) | ((opts.rtr ? 1 : 0) << 9);
    let w2 = 0, w3 = 0;
    for (let i = 0; i < bytes.length; i++) {
      if (i < 4) w2 |= (bytes[i] & 0xFF) << (8 * i);
      else w3 |= (bytes[i] & 0xFF) << (8 * (i - 4));
    }
    Atomics.store(this.ctrl, SAB_SLOT_CMD_ARG0, id >>> 0);
    Atomics.store(this.ctrl, SAB_SLOT_CMD_ARG1, flags >>> 0);
    Atomics.store(this.ctrl, SAB_SLOT_CMD_ARG2, w2 >>> 0);
    Atomics.store(this.ctrl, SAB_SLOT_RESP, RESP_IDLE);
    Atomics.store(this.ctrl, SAB_SLOT_CMD, CMD_SEND_TWAI);
    Atomics.notify(this.ctrl, SAB_SLOT_CMD, 1);
    while (Atomics.load(this.ctrl, SAB_SLOT_RESP) === RESP_IDLE) {} // eslint-disable-line no-empty
    Atomics.store(this.ctrl, SAB_SLOT_CMD_ARG0, w3 >>> 0);
    Atomics.store(this.ctrl, SAB_SLOT_RESP, RESP_IDLE);
    Atomics.store(this.ctrl, SAB_SLOT_CMD, CMD_PUSH_TWAI);
    Atomics.notify(this.ctrl, SAB_SLOT_CMD, 1);
    while (Atomics.load(this.ctrl, SAB_SLOT_RESP) === RESP_IDLE) {} // eslint-disable-line no-empty
  }

  /**
   * Read back the last NORMAL-mode TX frame captured by the virtual peer.
   * @returns {Promise<{count:number,id:number,ext:boolean,rtr:boolean,dlc:number,data:number[]}|null>}
   */
  async getTwaiTx() {
    this._checkReady();
    Atomics.store(this.ctrl, SAB_SLOT_RESP, RESP_IDLE);
    Atomics.store(this.ctrl, SAB_SLOT_CMD, CMD_GET_TWAI_TX);
    Atomics.notify(this.ctrl, SAB_SLOT_CMD, 1);
    while (Atomics.load(this.ctrl, SAB_SLOT_RESP) === RESP_IDLE) {} // eslint-disable-line no-empty
    const written = Atomics.load(this.ctrl, SAB_SLOT_CMD_ARG1);
    if (written < 20) return null;
    const dv = new DataView(this.readResp.buffer, this.readResp.byteOffset, written);
    const flags = dv.getUint32(8, true);
    const d0 = dv.getUint32(12, true), d1 = dv.getUint32(16, true);
    const dlc = flags & 0xF, data = [];
    for (let i = 0; i < dlc && i < 8; i++) data.push(i < 4 ? (d0 >>> (8 * i)) & 0xFF : (d1 >>> (8 * (i - 4))) & 0xFF);
    return { count: dv.getUint32(0, true), id: dv.getUint32(4, true), ext: !!(flags & 0x100), rtr: !!(flags & 0x200), dlc, data };
  }

  /**
   * Read memory from the chip address space.
   * @param {number} addr - Start address
   * @param {number} bytes - Number of bytes to read
   * @returns {Promise<Uint8Array>} Read data
   */
  async readMemory(addr, bytes = 0) {
    this._checkReady();
    Atomics.store(this.ctrl, SAB_SLOT_CMD_ARG0, addr);
    Atomics.store(this.ctrl, SAB_SLOT_CMD_ARG1, bytes);
    Atomics.store(this.ctrl, SAB_SLOT_RESP, RESP_IDLE);
    Atomics.store(this.ctrl, SAB_SLOT_CMD, CMD_READ_MEMORY);
    Atomics.notify(this.ctrl, SAB_SLOT_CMD, 1);
    while (Atomics.load(this.ctrl, SAB_SLOT_RESP) === RESP_IDLE) {} // eslint-disable-line no-empty
    const written = Atomics.load(this.ctrl, SAB_SLOT_CMD_ARG1);
    return this.readResp.subarray(0, written);
  }

  /**
   * Read a native MMIO register.
   * @param {number} hid - Native handler ID
   * @param {number} addr - Register address
   * @param {number} [size=4] - Access size (1/2/4 bytes)
   * @returns {Promise<number>} Register value
   */
  /**
   * Arm a Rust-side write watchpoint (logs [WP] pc/addr/val on match).
   * Pass 0xFFFFFFFF to clear all. Fire-and-forget (no response data).
   * @param {number} addr - DRAM address to watch (u32 word)
   */
  /**
   * Arm Rust PC tracing for the next n instructions ([T] pc=... lines).
   * @param {number} n - instruction budget
   */
  async pcTrace(n) {
    this._checkReady();
    Atomics.store(this.ctrl, SAB_SLOT_CMD_ARG0, n);
    Atomics.store(this.ctrl, SAB_SLOT_RESP, RESP_IDLE);
    Atomics.store(this.ctrl, SAB_SLOT_CMD, CMD_PCTRACE);
    Atomics.notify(this.ctrl, SAB_SLOT_CMD, 1);
    while (Atomics.load(this.ctrl, SAB_SLOT_RESP) === RESP_IDLE) {} // eslint-disable-line no-empty
  }

  async watchpoint(addr) {
    this._checkReady();
    Atomics.store(this.ctrl, SAB_SLOT_CMD_ARG0, addr);
    Atomics.store(this.ctrl, SAB_SLOT_RESP, RESP_IDLE);
    Atomics.store(this.ctrl, SAB_SLOT_CMD, CMD_WATCHPOINT);
    Atomics.notify(this.ctrl, SAB_SLOT_CMD, 1);
    while (Atomics.load(this.ctrl, SAB_SLOT_RESP) === RESP_IDLE) {} // eslint-disable-line no-empty
  }

  async readMmio(hid, addr, size = 4) {
    this._checkReady();
    Atomics.store(this.ctrl, SAB_SLOT_CMD_ARG0, hid);
    Atomics.store(this.ctrl, SAB_SLOT_CMD_ARG1, addr);
    Atomics.store(this.ctrl, SAB_SLOT_CMD_ARG2, size);
    Atomics.store(this.ctrl, SAB_SLOT_RESP, RESP_IDLE);
    Atomics.store(this.ctrl, SAB_SLOT_CMD, CMD_READ_MMIO);
    Atomics.notify(this.ctrl, SAB_SLOT_CMD, 1);
    while (Atomics.load(this.ctrl, SAB_SLOT_RESP) === RESP_IDLE) {} // eslint-disable-line no-empty
    return Atomics.load(this.ctrl, SAB_SLOT_CMD_ARG0) >>> 0;
  }

  async getPcapData() {
    this._checkReady();
    Atomics.store(this.ctrl, SAB_SLOT_RESP, RESP_IDLE);
    Atomics.store(this.ctrl, SAB_SLOT_CMD, CMD_GET_PCAP);
    Atomics.notify(this.ctrl, SAB_SLOT_CMD, 1);
    while (Atomics.load(this.ctrl, SAB_SLOT_RESP) === RESP_IDLE) {} // eslint-disable-line no-empty
    const written = Atomics.load(this.ctrl, SAB_SLOT_CMD_ARG1);
    return this.readResp.subarray(0, written);
  }

  async getFfiCounts() {
    this._checkReady();
    Atomics.store(this.ctrl, SAB_SLOT_RESP, RESP_IDLE);
    Atomics.store(this.ctrl, SAB_SLOT_CMD, CMD_GET_FFI_COUNTS);
    Atomics.notify(this.ctrl, SAB_SLOT_CMD, 1);
    while (Atomics.load(this.ctrl, SAB_SLOT_RESP) === RESP_IDLE) {} // eslint-disable-line no-empty
    const written = Atomics.load(this.ctrl, SAB_SLOT_CMD_ARG1);
    const out = [];
    const dv = new DataView(this.readResp.buffer, this.readResp.byteOffset, written);
    for (let i = 0; i < written / 8; i++) out.push(dv.getFloat64(i * 8, true));
    return out;
  }

  /** Get WiFi AP statistics (state, tx/rx, clients, IP) */
  async getWifiStats() {
    this._checkReady();
    Atomics.store(this.ctrl, SAB_SLOT_RESP, RESP_IDLE);
    Atomics.store(this.ctrl, SAB_SLOT_CMD, CMD_GET_WIFI_STATS);
    Atomics.notify(this.ctrl, SAB_SLOT_CMD, 1);
    while (Atomics.load(this.ctrl, SAB_SLOT_RESP) === RESP_IDLE) {} // eslint-disable-line no-empty
    const written = Atomics.load(this.ctrl, SAB_SLOT_CMD_ARG1);
    if (written < 28) return null;
    const dv = new DataView(this.readResp.buffer, this.readResp.byteOffset, written);
    const decode = (off) => {
      let end = off;
      while (end < written && this.readResp[end] !== 0) end++;
      return new TextDecoder().decode(this.readResp.subarray(off, end));
    };
    return {
      state: dv.getInt32(0, true),
      txFrames: dv.getInt32(4, true),
      txBytes: dv.getInt32(8, true),
      rxFrames: dv.getInt32(12, true),
      rxBytes: dv.getInt32(16, true),
      probeRequestCount: dv.getInt32(20, true),
      connectedClients: dv.getInt32(24, true),
      ip: decode(28),
      portForward: decode(68),
      errorMessage: decode(108),
      udpForward: decode(148),
    };
  }

  /** Start continuous simulation (non-blocking) */
  run() {
    this._checkReady();
    Atomics.store(this.ctrl, SAB_SLOT_RUN, 1);
    Atomics.store(this.ctrl, SAB_SLOT_RESP, RESP_IDLE);
    Atomics.store(this.ctrl, SAB_SLOT_CMD, CMD_RUN);
    Atomics.notify(this.ctrl, SAB_SLOT_CMD, 1);
  }

  /** Stop the simulation loop (non-blocking, finishes current chunk) */
  stop() {
    if (!this._ready) return;
    Atomics.store(this.ctrl, SAB_SLOT_RUN, 0);
  }

  /** Terminate the worker thread and release resources */
  terminate() {
    if (this.worker) {
      _liveWorkers.delete(this.worker);
      this.worker.terminate();
      this.worker = null;
    }
    this._ready = false;
    this.ctrl = null;
    this.ctrlSab = null;
    this.uartCtrl = null;
    this.uartRing = null;
    this.uartSab = null;
    this.readResp = null;
    this.readRespSab = null;
    this.errMsg = null;
    this.errSab = null;
    this.memory = {};
  }

  /** Poll the UART ring buffer and deliver bytes via _onUART callback */
  pollUart() {
    if (!this.uartCtrl || !this._onUART) return;
    const w = Atomics.load(this.uartCtrl, 0);
    const r = Atomics.load(this.uartCtrl, 1);
    const count = w - r;
    if (count <= 0) return;
    const maxRead = Math.min(count, UART_RING_SIZE);
    for (let i = 0; i < maxRead; i++) {
      this._onUART(this.uartRing[(r + i) % UART_RING_SIZE]);
    }
    Atomics.store(this.uartCtrl, 1, r + maxRead);
  }

  /** Send bytes to the emulated UART RX (guest input) */
  sendUart(data) {
    if (!this.uartRxCtrl || !this.uartRxRing) return;
    const bytes = typeof data === 'string' ? Buffer.from(data, 'utf-8') : data;
    for (let i = 0; i < bytes.length; i++) {
      const w = Atomics.load(this.uartRxCtrl, 0);
      this.uartRxRing[w % UART_RING_SIZE] = bytes[i];
      Atomics.store(this.uartRxCtrl, 0, w + 1);
    }
    // If not running, wake worker via CMD_UART_RX to feed immediately
    if (this.ctrl && Atomics.load(this.ctrl, SAB_SLOT_RUN) === 0) {
      Atomics.store(this.ctrl, SAB_SLOT_RESP, RESP_IDLE);
      Atomics.store(this.ctrl, SAB_SLOT_CMD, CMD_UART_RX);
      Atomics.notify(this.ctrl, SAB_SLOT_CMD, 1);
      while (Atomics.load(this.ctrl, SAB_SLOT_RESP) === RESP_IDLE) {} // eslint-disable-line no-empty
    }
    // If running, runSimChunk polls uartRxRing directly — no CMD needed
  }

  get running() {
    return this.ctrl ? Atomics.load(this.ctrl, SAB_SLOT_RUN) === 1 : false;
  }
  get nanos() {
    if (!this.ctrl) return 0;
    const lo = Atomics.load(this.ctrl, SAB_SLOT_NANOS_LO);
    const hi = Atomics.load(this.ctrl, SAB_SLOT_NANOS_HI);
    return hi * 4294967296 + (lo >>> 0);
  }
  get pc1() {
    return this.debug ? this.debug[1] : -1;
  }
  get pc() {
    return this.ctrl ? Atomics.load(this.ctrl, SAB_SLOT_PC) : -1;
  }
  get stuck() {
    return this.ctrl ? Atomics.load(this.ctrl, SAB_SLOT_STUCK) : 0;
  }
  get idle() {
    return this.ctrl ? Atomics.load(this.ctrl, SAB_SLOT_IDLE) === 1 : false;
  }
  get ready() { return this._ready; }
  get macAddress() {
    if (!this.memory.wifiMac) return null;
    const buf = this.memory.wifiMac;
    const hi = new DataView(buf.buffer, buf.byteOffset + 64, 4).getUint32(0, true);
    const lo = new DataView(buf.buffer, buf.byteOffset + 68, 4).getUint32(0, true);
    return [
      hi & 0xff, (hi >> 8) & 0xff, (hi >> 16) & 0xff, (hi >> 24) & 0xff,
      lo & 0xff, (lo >> 8) & 0xff,
    ].map(b => b.toString(16).padStart(2, "0")).join(":");
  }
  get psramType() { return this._chipInfo.psramType || null; }
  get flashSizeMB() { return this._chipInfo.flashSizeMB || null; }
  get psramSizeMB() { return this._chipInfo.psramSizeMB || null; }
  get board() { return this._chipInfo.board || 'esp32'; }
  get cpuFrequency() { return this.ctrl ? Atomics.load(this.ctrl, SAB_SLOT_CPU_FREQ) : (this._chipInfo.cpuFrequency || null); }
  set cpuFrequency(freq) {
    if (!this.ctrl) return;
    const native = this._chipInfo._nativeFrequency || 160000000;
    const v = freq === "auto" ? 8000000 : freq === "max" ? native : Number(freq) * 1e6;
    Atomics.store(this.ctrl, SAB_SLOT_CPU_FREQ, v);
  }

  // Live WiFi stats — read directly from SAB, zero-latency
  get wifiState() { return this.ctrl ? Atomics.load(this.ctrl, SAB_SLOT_WIFI_STATE) : -1; }
  get wifiTxFrames() { return this.ctrl ? Atomics.load(this.ctrl, SAB_SLOT_WIFI_TX_FRAMES) : 0; }
  get wifiTxBytes() { return this.ctrl ? Atomics.load(this.ctrl, SAB_SLOT_WIFI_TX_BYTES) : 0; }
  get wifiRxFrames() { return this.ctrl ? Atomics.load(this.ctrl, SAB_SLOT_WIFI_RX_FRAMES) : 0; }
  get wifiRxBytes() { return this.ctrl ? Atomics.load(this.ctrl, SAB_SLOT_WIFI_RX_BYTES) : 0; }
  get wifiProbes() { return this.ctrl ? Atomics.load(this.ctrl, SAB_SLOT_WIFI_PROBES) : 0; }

  set budget(nanos) {
    if (this.ctrl) Atomics.store(this.ctrl, SAB_SLOT_BUDGET, nanos);
  }
  get budget() {
    return this.ctrl ? Atomics.load(this.ctrl, SAB_SLOT_BUDGET) : 50000;
  }

  _checkReady() {
    if (!this._ready) throw new Error("SimulatorWorker not initialized");
  }

  _onMessage(e) {
    const msg = (e && e.data && typeof e.data === "object" && e.data.type) ? e.data : e;
    const { type } = msg;
    switch (type) {
      case "ready":
        this._ready = true;
        if (msg.memoryRegions) {
          for (const [key, { buffer, byteOffset, byteLength }] of Object.entries(msg.memoryRegions)) {
            this.memory[key] = new Uint8Array(buffer, byteOffset, byteLength);
          }
        }
        if (msg.chipInfo) Object.assign(this._chipInfo, msg.chipInfo);
        if (Atomics.load(this.ctrl, SAB_SLOT_ERR_FLAG) && this.errMsg) {
          const end = this.errMsg.indexOf(0);
          const errText = new TextDecoder().decode(this.errMsg.subarray(0, end >= 0 ? end : this.errMsg.length));
          if (this._onError) { this._onError(new Error(errText)); this._onError = null; }
        }
        if (this._onReady) { this._onReady(); this._onReady = null; }
        break;
      case "error":
        if (this._onError) { this._onError(new Error(msg.message)); this._onError = null; }
        break;
    }
  }
}

export { SimulatorWorker };
