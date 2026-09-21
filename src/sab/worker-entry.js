import { ESP32 } from "../peripherals/esp32/esp32.js";
import { parseMacAddress, writePartitionTable } from "../peripherals/common/partition-table.js";


// ===== Engine selection =====
// 'wasm' = WASM engine (loaded automatically from default path)
// Can be overridden by config.engine from the host
const ENGINE = (typeof process !== 'undefined' && process.env && process.env.ENGINE) || 'wasm';
const DEBUG_BOOT = true;
// ============================

const SAB_RUN = 0, _SAB_BUDGET = 1, SAB_NANOS_LO = 2, SAB_STATUS = 3;
const SAB_PC = 4, _SAB_STUCK = 5, SAB_IDLE = 6, _SAB_PROGRESS_INTERVAL = 7, SAB_NANOS_HI = 8;
const SAB_CMD = 9, SAB_CMD_ARG0 = 10, SAB_CMD_ARG1 = 11, SAB_RESP = 12, SAB_ERR_FLAG = 13, SAB_CPU_FREQ = 14, SAB_CMD_ARG2 = 21;
const SAB_WIFI_STATE = 15, SAB_WIFI_TX_FRAMES = 16, SAB_WIFI_TX_BYTES = 17, SAB_WIFI_RX_FRAMES = 18, SAB_WIFI_RX_BYTES = 19, SAB_WIFI_PROBES = 20;

// Debug SAB layout (u32 indices) — populated when debugSab is provided in init
const DBG_PC0 = 0, DBG_PC1 = 1, DBG_CYCLES = 2, DBG_NANOS_LO = 3, DBG_NANOS_HI = 4;
const DBG_TICKS = 5, DBG_ENABLED0 = 6, DBG_ENABLED1 = 7;
const DBG_PHYS_START = 16;   // 64 u32s for physical registers (core 0)
const DBG_SPEC_START = 80;   // 16 u32s for key special registers (12 used)
const DBG_ALL_SPEC_START = 96;  // 256 u32s for ALL special registers
const DBG_MMU_PRO_START = 352;  // 64 u32s for MMU pro table
const DBG_MMU_APP_START = 416;  // 64 u32s for MMU app table
const DBG_MEM_HASH_START = 480;  // 4 u32s: iram, dram, rtc_fast, sram hashes
const DBG_PERIPH_START = 484;    // 16 u32s for peripheral register snapshots
const DBG_CORE1_SPEC_START = 500; // 256 u32s for ALL special registers (core 1)
const DBG_WIFI_START = 756;      // 16 u32s: wifi/matrix debug
// Total: 772 u32s = 3088 bytes

// Quick 32-bit FNV-1a hash of a Uint8Array region (max N bytes)
function hashMem(data, maxBytes) {
  const len = Math.min(data.length, maxBytes);
  let h = 0x811c9dc5;
  let i = 0;
  for (; i + 3 < len; i += 4) {
    const w = data[i] | (data[i + 1] << 8) | (data[i + 2] << 16) | (data[i + 3] << 24);
    h = Math.imul(h ^ w, 0x01000193) >>> 0;
  }
  for (; i < len; i++) { h ^= data[i]; h = Math.imul(h, 0x01000193) >>> 0; }
  return h;
}

const CMD_NONE = 0, CMD_RUN = 1, CMD_RESET = 2, CMD_SEED_MMU = 3, CMD_WRITE_UINT32 = 4, CMD_READ_MEMORY = 5, CMD_GET_PCAP = 6, CMD_GET_WIFI_STATS = 7, CMD_STEP = 8, CMD_DESTROY = 9, CMD_GET_FFI_COUNTS = 10, CMD_READ_MMIO = 11, CMD_UART_RX = 12, CMD_SET_PIN_INPUT = 13, CMD_SET_TOUCH_INPUT = 14, CMD_SET_VOLTAGE = 15, CMD_FEED_I2S_RX = 16, CMD_SEND_TWAI = 17, CMD_PUSH_TWAI = 18, CMD_GET_TWAI_TX = 19, CMD_WATCHPOINT = 20, CMD_PCTRACE = 21, CMD_PRESS_RESET = 22, CMD_PRESS_BOOT = 23;
const _RESP_IDLE = 0, RESP_DONE = 1, RESP_ERROR = 2;

const UART_RING_SIZE = 16384;

let msgPort;
if (typeof self !== "undefined") {
  msgPort = self;
} else {
  const { parentPort } = await import("worker_threads");
  msgPort = parentPort;
}

function send(msg) { msgPort.postMessage(msg); }

const chipConstructors = { ESP32 };
let chip = null;
let ctrl = null;
let uartCtrl = null;
let uartRing = null;
let uartRxCtrl = null;
let uartRxRing = null;
let readResp = null;
let errMsg = null;
let debugData = null; // Uint32Array for debug SAB

function getMemorySABs(chip) {
  const regions = {};
  const extract = (key, obj) => {
    if (!obj) return;
    const arr = obj instanceof Uint8Array ? obj : obj.data;
    if (!arr || !(arr.buffer instanceof SharedArrayBuffer)) return;
    regions[key] = { buffer: arr.buffer, byteOffset: arr.byteOffset, byteLength: arr.byteLength };
  };
  extract("flash", chip.flash);
  extract("chipROM", chip.chipROM);
  extract("iram", chip.iram);
  extract("psram", chip.psram);
  extract("dataMem", chip.dataMem);
  extract("rtcFastMem", chip.rtcFastMem);
  extract("mmuTableMemory", chip.mmuTableMemory);
  extract("iramCache", chip.iramCache);
  extract("sram", chip.sram);
  extract("wifiMac", chip.wifiMacState);
  extract("sdcard", chip.sdData);
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
  const native = chip._nativeFrequency || 160e6;
  return targetFreq >= native ? 1 : native / targetFreq;
}

let _dbgUserCodeNotified = false;
let _writeSABCount = 0;
function writeSABState() {
  if (!ctrl) return;
  const n = chip ? (chip.cycles / 160e6) * 1e9 : 0;
  const scaled = n * nanosMultiplier();
  ctrl[SAB_NANOS_LO] = scaled | 0;
  ctrl[SAB_NANOS_HI] = (scaled / 4294967296) | 0;
  // Use WASM core PC when WASM is loaded (JS core PC is stale)
  if (chip._wasmCores?.[0]) {
    ctrl[SAB_PC] = chip._wasmCores[0].PC;
  } else {
    ctrl[SAB_PC] = chip?.cores?.[0]?.PC ?? -1;
  }
  ctrl[SAB_IDLE] = chip?.coresIdle ? 1 : 0;
  if (DEBUG_BOOT && !_dbgUserCodeNotified && chip) {
    const pc = ctrl[SAB_PC];
    if (pc >= 0x400d0000 && pc < 0x40100000) { console.log(`[DBG] USER CODE ENTERED at nanos=${n} PC=0x${pc.toString(16).padStart(8,'0')}`); _dbgUserCodeNotified = true; }
    else if (pc >= 0x40078000 && pc < 0x40081000) { /* bootloader region */ }
    else if (pc > 0 && pc < 0x40078000 && pc !== (ctrl[SAB_PC] || 0)) { /* boot ROM */ }
  }

  // Fast path: always write core state (cheap — ~10 typed-array writes).
  if (debugData && chip) {
    debugData[DBG_PC0] = ctrl[SAB_PC] >>> 0;
    debugData[DBG_PC1] = (chip._wasmCores?.[1]?.PC ?? chip.cores?.[1]?.PC ?? 0) >>> 0;
    debugData[DBG_CYCLES] = chip.cycles >>> 0;
    debugData[DBG_NANOS_LO] = ctrl[SAB_NANOS_LO];
    debugData[DBG_NANOS_HI] = ctrl[SAB_NANOS_HI];
    debugData[DBG_TICKS] = (chip.clocks?.cpu?.ticks ?? 0) >>> 0;
    debugData[DBG_ENABLED0] = chip._wasmCores?.[0] ? (chip._wasmCores[0].enabled ? 1 : 0) : (chip.cores?.[0]?.enabled ? 1 : 0);
    debugData[DBG_ENABLED1] = chip._wasmCores?.[1] ? (chip._wasmCores[1].enabled ? 1 : 0) : (chip.cores?.[1]?.enabled ? 1 : 0);

    // Heavy debug: register dumps, MMIO reads, memory hashes — every 4th call.
    _writeSABCount++;
    if ((_writeSABCount & 3) === 0) {
      // First 64 physical registers (core 0)
      const phys = chip._wasmCores?.[0]?._physRegs ?? chip.cores?.[0]?.physicalRegisters;
      if (phys) {
        for (let i = 0; i < 64 && i < phys.length; i++)
          debugData[DBG_PHYS_START + i] = phys[i] >>> 0;
      }
      // Key special registers
      const spec = chip._wasmCores?.[0]?._specRegs ?? chip.cores?.[0]?.specialRegisters;
      if (spec) {
        const keySpecs = [72, 226, 228, 230, 231, 232, 233, 234, 235, 240, 241, 242];
        for (let i = 0; i < keySpecs.length; i++)
          if (keySpecs[i] < spec.length)
            debugData[DBG_SPEC_START + i] = spec[keySpecs[i]] >>> 0;
        // All special registers
        for (let i = 0; i < spec.length && i < 256; i++)
          debugData[DBG_ALL_SPEC_START + i] = spec[i] >>> 0;
        const canonicalCcount = (chip.clocks?.cpu?.ticks ?? 0) >>> 0;
        debugData[DBG_SPEC_START + 7] = canonicalCcount;
        debugData[DBG_ALL_SPEC_START + 234] = canonicalCcount;
      }
      // MMU tables
      if (chip.mmuTablePro) {
        for (let i = 0; i < 64; i++)
          debugData[DBG_MMU_PRO_START + i] = chip.mmuTablePro[i] >>> 0;
      }
      if (chip.mmuTableApp) {
        for (let i = 0; i < 64; i++)
          debugData[DBG_MMU_APP_START + i] = chip.mmuTableApp[i] >>> 0;
      }
      // All special registers (core 1)
      const spec1 = chip._wasmCores?.[1]?._specRegs ?? chip.cores?.[1]?.specialRegisters;
      if (spec1) {
        for (let i = 0; i < spec1.length && i < 256; i++)
          debugData[DBG_CORE1_SPEC_START + i] = spec1[i] >>> 0;
      }
      // WiFi / interrupt-matrix state via native MMIO read path
      if (chip._wasmLoader?.exports?.map_read) {
        const mr = (addr) => { try { return chip._wasmLoader.exports.map_read(0, addr, 4) >>> 0; } catch(_) { return 0; } };
        debugData[DBG_WIFI_START + 0] = mr(0x3ff00104);
        debugData[DBG_WIFI_START + 1] = mr(0x3ff00218);
        debugData[DBG_WIFI_START + 2] = mr(0x3ff000ec);
        debugData[DBG_WIFI_START + 3] = mr(0x3ff000f8);
        debugData[DBG_WIFI_START + 4] = mr(0x3ff730c48);
        debugData[DBG_WIFI_START + 5] = mr(0x3ff730c4c);
        debugData[DBG_WIFI_START + 6] = mr(0x3ff73084);
        debugData[DBG_WIFI_START + 7] = mr(0x3ff734c4);
      }
      // Memory hashes — every 16th call (~1M cycles) to avoid 256KB hashing overhead.
      if ((_writeSABCount & 15) === 0) {
        const getData = (mem) => (mem instanceof Uint8Array ? mem : mem?.data);
        debugData[DBG_MEM_HASH_START] = getData(chip.iram) ? hashMem(getData(chip.iram), 65536) : 0;
        debugData[DBG_MEM_HASH_START + 1] = getData(chip.dataMem) ? hashMem(getData(chip.dataMem), 65536) : 0;
        debugData[DBG_MEM_HASH_START + 2] = getData(chip.rtcFastMem) ? hashMem(getData(chip.rtcFastMem), 16384) : 0;
        debugData[DBG_MEM_HASH_START + 3] = getData(chip.sram) ? hashMem(getData(chip.sram), 65536) : 0;
      }
      // Peripheral register snapshots
      const g = chip.gpio;
      if (g) {
        debugData[DBG_PERIPH_START]     = (g.out?.[0] ?? 0) >>> 0;
        debugData[DBG_PERIPH_START + 1] = (g.out?.[1] ?? 0) >>> 0;
        debugData[DBG_PERIPH_START + 2] = (g.enable?.[0] ?? 0) >>> 0;
        debugData[DBG_PERIPH_START + 3] = (g.in?.[0] ?? 0) >>> 0;
        debugData[DBG_PERIPH_START + 4] = (g.status?.[0] ?? 0) >>> 0;
      }
      debugData[DBG_PERIPH_START + 5] = (chip.uart?.[0]?.status ?? 0) >>> 0;
      debugData[DBG_PERIPH_START + 6] = (chip.uart?.[1]?.status ?? 0) >>> 0;
      const tg0 = chip.timerGroup?.[0]?.timers?.[0];
      debugData[DBG_PERIPH_START + 7] = tg0 ? (Number(tg0.counterLo) >>> 0) : 0;
      debugData[DBG_PERIPH_START + 8] = tg0 ? (Number(tg0.counterHi) >>> 0) : 0;
      debugData[DBG_PERIPH_START + 9] = 0;
    }
  }

  // Live wifi stats — piggyback on existing state sync (zero extra cost)
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

let _uartCount = 0;
function uartWriteByte(byte) {
  if (!uartCtrl) { if (++_uartCount < 10) console.log(`[UART-WARN] uartCtrl null on byte=${byte}`); return; }
  const idx = uartCtrl[0] % UART_RING_SIZE;
  uartRing[idx] = byte;
  uartCtrl[0]++;
}

let needsEventLoop = true;
let simChunkSize = 500000;
// TWAI peer-frame staging (SEND_TWAI stages id/flags/data0-3, PUSH_TWAI
// delivers with data4-7) — the 3-arg CMD channel can't fit a full frame.
let twaiStage = [0, 0, 0];

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
  // --- Hot-loop prefetched references (avoids optional-chain + getter overhead per step) ---
  const wasmExp = chip._wasmLoader?.exports || null;
  const idleAdvance = wasmExp?.native_idle_advance || null;
  const pumpEvents  = wasmExp?.native_pump_events  || null;
  const clockRoot   = chip.clocks?.root || null;
  const fireDue     = clockRoot?.fireDueEvents?.bind(clockRoot) || null;
  // Direct SAB views for coresIdle check (bypasses getter chains)
  const wCores = chip._wasmCores;
  const idleView0     = wCores?.[0]?._idleView || null;
  const idleView1     = wCores?.[1]?._idleView || null;
  const pendingView0  = wCores?.[0]?._pendingIntView || null;
  const pendingView1  = wCores?.[1]?._pendingIntView || null;

  // Sync clock once before entering the loop.
  syncClockState(wasmExp);
  try {
    let steps = 0;
    let cycles = chip.cycles;
    // Cache native UART feed for hot loop
    const nativeUartFeed = wasmExp?.native_uart_feed || null;
    if (cycles === 0 || isNaN(cycles)) console.log(`[RUNSIM] start cycles=${cycles} isNaN=${isNaN(cycles)}`);
    while (steps < CHUNK) {
      // Poll UART RX ring (host -> guest) and feed bytes
      if (uartRxCtrl && uartRxRing) {
        const w = Atomics.load(uartRxCtrl, 0);
        const r = Atomics.load(uartRxCtrl, 1);
        const avail = w - r;
        if (avail > 0) {
          const n = Math.min(avail, 32); // feed at most 32 bytes per step to avoid starvation
          for (let i = 0; i < n; i++) {
            const byte = uartRxRing[(r + i) % UART_RING_SIZE];
            try {
              if (nativeUartFeed) nativeUartFeed(0, byte);
              else chip.uart[0].feedByte(byte);
            } catch {}
          }
          Atomics.store(uartRxCtrl, 1, r + n);
        }
      }
      // Runtime command servicing during run (UART-RX precedent): the main
      // command loop never runs while runSimChunk self-reschedules, so
      // service one-shot commands inline here instead of deadlocking the
      // host. CMD_PCTRACE/CMD_WATCHPOINT arm Rust-side diag (needed
      // mid-stall: stop() only clears SAB_RUN, the worker still spins in
      // runSimChunk until the chunk ends, so commandLoop never runs).
      if (Atomics.load(ctrl, SAB_CMD) === CMD_PCTRACE) {
        try { chip?._wasmLoader?.exports?.native_pc_trace?.(Atomics.load(ctrl, SAB_CMD_ARG0) >>> 0); } catch {}
        Atomics.store(ctrl, SAB_RESP, RESP_DONE);
        Atomics.store(ctrl, SAB_CMD, CMD_NONE);
      }
      if (Atomics.load(ctrl, SAB_CMD) === CMD_WATCHPOINT) {
        try {
          const ex = chip?._wasmLoader?.exports;
          const addr = Atomics.load(ctrl, SAB_CMD_ARG0);
          if (addr >>> 0 === 0xffffffff) ex?.native_watchpoint_clear?.();
          else ex?.native_watchpoint_add?.(addr >>> 0);
        } catch {}
        Atomics.store(ctrl, SAB_RESP, RESP_DONE);
        Atomics.store(ctrl, SAB_CMD, CMD_NONE);
      }
      // run196: readMemory must not deadlock mid-run (read while RUN=1).
      // Service it inline like the other one-shot commands: the DRAM/flash
      // views are live in WASM memory, safe to copy from the hot loop.
      if (Atomics.load(ctrl, SAB_CMD) === CMD_READ_MEMORY) {
        try {
          const addr = Atomics.load(ctrl, SAB_CMD_ARG0);
          const maxBytes = Atomics.load(ctrl, SAB_CMD_ARG1);
          let written = 0;
          if (typeof readResp !== 'undefined' && readResp) {
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
        } catch {}
        Atomics.store(ctrl, SAB_RESP, RESP_DONE);
        Atomics.store(ctrl, SAB_CMD, CMD_NONE);
      }
      if (Atomics.load(ctrl, SAB_CMD) === CMD_SET_VOLTAGE) {
        try { chip?.setVddMv?.(Atomics.load(ctrl, SAB_CMD_ARG0)); } catch {}
        Atomics.store(ctrl, SAB_RESP, RESP_DONE);
        Atomics.store(ctrl, SAB_CMD, CMD_NONE);
      }
      if (Atomics.load(ctrl, SAB_CMD) === CMD_SET_TOUCH_INPUT) {
        const pad = Atomics.load(ctrl, SAB_CMD_ARG0);
        const count = Atomics.load(ctrl, SAB_CMD_ARG1);
        try { chip?.setTouchInput?.(pad, count); } catch {}
        Atomics.store(ctrl, SAB_RESP, RESP_DONE);
        Atomics.store(ctrl, SAB_CMD, CMD_NONE);
      }
      if (Atomics.load(ctrl, SAB_CMD) === CMD_SET_PIN_INPUT) {
        const pin = Atomics.load(ctrl, SAB_CMD_ARG0);
        const level = Atomics.load(ctrl, SAB_CMD_ARG1);
        try { wasmExp?.native_gpio_set_pin_input?.(pin, level); } catch {}
        const holder = chip?.gpio?.pins?.[pin];
        if (holder) { try { holder.inputValue = level ? 1 : 0; } catch {} }
        Atomics.store(ctrl, SAB_RESP, RESP_DONE);
        Atomics.store(ctrl, SAB_CMD, CMD_NONE);
      }
      if (Atomics.load(ctrl, SAB_CMD) === CMD_FEED_I2S_RX) {
        const sample = Atomics.load(ctrl, SAB_CMD_ARG0) >>> 0;
        try { wasmExp?.native_i2s_push_rx_sample?.(0, sample); } catch {}
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
        try { wasmExp?.native_twai_push_rx?.(twaiStage[0], twaiStage[1], twaiStage[2], Atomics.load(ctrl, SAB_CMD_ARG0) >>> 0); } catch {}
        Atomics.store(ctrl, SAB_RESP, RESP_DONE);
        Atomics.store(ctrl, SAB_CMD, CMD_NONE);
      }
      if (Atomics.load(ctrl, SAB_CMD) === CMD_PRESS_RESET) {
        try { chip?.pressResetButton?.(); } catch {}
        Atomics.store(ctrl, SAB_RESP, RESP_DONE);
        Atomics.store(ctrl, SAB_CMD, CMD_NONE);
      }
      if (Atomics.load(ctrl, SAB_CMD) === CMD_PRESS_BOOT) {
        try { chip?.pressBootButton?.(Atomics.load(ctrl, SAB_CMD_ARG0) !== 0); } catch {}
        Atomics.store(ctrl, SAB_RESP, RESP_DONE);
        Atomics.store(ctrl, SAB_CMD, CMD_NONE);
      }
      chip.step();
      steps++;
      cycles = chip.cycles;
      // Fire due ClockTree events (ccompare / beacon / wifi-tx) on the JS
      // clock tree. Pumped at the per-step (1024-instruction) cadence so the
      // JS clock stays out of the execution hot path.
      if (fireDue) { try { fireDue(); } catch(_) {} }
      // Inline coresIdle: 4 direct typed-array reads (bypasses getter chains).
      const isIdle = idleView0 && idleView1
        ? (idleView0[0] !== 0 && pendingView0[0] === 0 && idleView1[0] !== 0 && pendingView1[0] === 0)
        : chip.coresIdle;
      if (isIdle) {
        // Idle path: native_idle_advance already syncs CLK_CYCLES internally,
        // so no separate syncClockState() needed (saves 1 FFI call per step).
        if (idleAdvance) {
          try { chip.cycles += idleAdvance(chip.cycles >>> 0); cycles = chip.cycles; } catch(_) {}
        }
      } else {
        // Busy path: sync clock before pumping (native_pump_events reads CLK_CYCLES).
        syncClockState(wasmExp);
        if (pumpEvents) { try { pumpEvents(); } catch(_) {} }
      }
      if ((cycles & 524287) === 0) writeSABState();
      if (!Atomics.load(ctrl, SAB_RUN)) break;
    }
  } catch (err) {
    console.error('[SIM-ERROR]', err.message, err.stack);
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
        chip.mmuTablePro[0] = 1; chip.mmuTablePro[1] = 2;
        chip.mmuTablePro[2] = 3; chip.mmuTablePro[3] = 0;
        const pages = Math.ceil((chip.flash?.length || 0) / 65536);
        for (let p = 4; p < pages + 1; p++) chip.mmuTablePro[p] = p + (offset || 0);
      }
      break;
    }
    case CMD_WRITE_UINT32: {
      const addr = Atomics.load(ctrl, SAB_CMD_ARG0);
      const value = Atomics.load(ctrl, SAB_CMD_ARG1);
      try { chip?.cores?.[0]?.writeUint32(addr, value); } catch {}
      break;
    }
    case CMD_READ_MEMORY: {
      const addr = Atomics.load(ctrl, SAB_CMD_ARG0);
      const maxBytes = Atomics.load(ctrl, SAB_CMD_ARG1);
      let written = 0;
      if (readResp) {
        try {
          // run196: readMemory must work mid-run AND mid-stall. chip.step()
          // advances cores; the DRAM/flash views are live in WASM memory, so
          // read via the loader's mapAddress (same path as GDB `m`).
          const mem = chip?.mapAddress ? chip.mapAddress(addr) : null;
          if (mem && mem.data) {
            const offset = addr - mem.baseAddr;
            const available = Math.min(mem.data.length - offset, readResp.length);
            const len = Math.min(maxBytes || available, available);
            if (len > 0) readResp.set(mem.data.subarray(offset, offset + len), 0);
            written = len;
          }
        } catch {}
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
        } catch {}
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
          writeStr(s.ip || '');
          writeStr(s.portForward || '');
          writeStr(s.errorMessage || '');
          writeStr(s.udpForward || '');
          written = off;
        } catch {}
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
        // run176: hid 99 = native_bt_diag bulk snapshot. CAUTION: words 3..7
        // alias live ctrl slots (STATUS/PC/NANOS) — the caller must snapshot
        // them BEFORE issuing another command (any command overwrites them).
        // Words: 0..2 = ARG0/1/2, 3 = RESP, 4 = STATUS, 5 = PC,
        // 6 = NANOS_LO, 7 = NANOS_HI.
        if (hid === 99 && ex?.native_bt_diag) {
          const scratch = (ex.native_bt_diag_scratch?.() >>> 0) || 0;
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
      } catch(_e) { console.warn(`native_diag_read error: ${_e.message}`); }
      if (hid !== 99) Atomics.store(ctrl, SAB_CMD_ARG0, val);
      break;
    }
    case CMD_WATCHPOINT: {
      const addr = Atomics.load(ctrl, SAB_CMD_ARG0);
      try {
        const ex = chip?._wasmLoader?.exports;
        if (addr >>> 0 === 0xffffffff) ex?.native_watchpoint_clear?.();
        else ex?.native_watchpoint_add?.(addr >>> 0);
      } catch(_e) { console.warn(`native_watchpoint error: ${_e.message}`); }
      break;
    }
    case CMD_PCTRACE: {
      const n = Atomics.load(ctrl, SAB_CMD_ARG0);
      try { chip?._wasmLoader?.exports?.native_pc_trace?.(n >>> 0); } catch(_e) {}
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
            } catch {}
          }
          Atomics.store(uartRxCtrl, 1, r + maxRead);
        }
      }
      break;
    }
    case CMD_SET_VOLTAGE: {
      try { chip?.setVddMv?.(Atomics.load(ctrl, SAB_CMD_ARG0)); } catch {}
      break;
    }
    case CMD_SET_TOUCH_INPUT: {
      try { chip?.setTouchInput?.(Atomics.load(ctrl, SAB_CMD_ARG0), Atomics.load(ctrl, SAB_CMD_ARG1)); } catch {}
      break;
    }
    case CMD_SET_PIN_INPUT: {
      const pin = Atomics.load(ctrl, SAB_CMD_ARG0);
      const level = Atomics.load(ctrl, SAB_CMD_ARG1);
      try { chip?._wasmLoader?.exports?.native_gpio_set_pin_input?.(pin, level); } catch {}
      const holder = chip?.gpio?.pins?.[pin];
      if (holder) { try { holder.inputValue = level ? 1 : 0; } catch {} }
      break;
    }
    case CMD_FEED_I2S_RX: {
      const sample = Atomics.load(ctrl, SAB_CMD_ARG0) >>> 0;
      try { chip?._wasmLoader?.exports?.native_i2s_push_rx_sample?.(0, sample); } catch {}
      break;
    }
    // Dev-board buttons (real-device behavior): RESET (EN) taps chip reset;
    // BOOT (GPIO0 strapping) drives the strap bit + live pin level.
    // Serviced inline here (like CMD_SET_PIN_INPUT) so they work mid-run.
    case CMD_PRESS_RESET: {
      try { chip?.pressResetButton?.(); } catch {}
      break;
    }
    case CMD_PRESS_BOOT: {
      try { chip?.pressBootButton?.(Atomics.load(ctrl, SAB_CMD_ARG0) !== 0); } catch {}
      break;
    }
    case CMD_SEND_TWAI: {
      twaiStage[0] = Atomics.load(ctrl, SAB_CMD_ARG0) >>> 0;
      twaiStage[1] = Atomics.load(ctrl, SAB_CMD_ARG1) >>> 0;
      twaiStage[2] = Atomics.load(ctrl, SAB_CMD_ARG2) >>> 0;
      break;
    }
    case CMD_PUSH_TWAI: {
      try { chip?._wasmLoader?.exports?.native_twai_push_rx?.(twaiStage[0], twaiStage[1], twaiStage[2], Atomics.load(ctrl, SAB_CMD_ARG0) >>> 0); } catch {}
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
        } catch {}
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

// Blocking command loop for MultiSimulator workers (no event loop yield)
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

// Helper: apply common chip setup (flash, ROM, MMU, GPIO, UART, MAC, reset)
function applyBasicSetup(chip, config, flash, rom) {
  if (flash && chip.flash) {
    if (typeof SharedArrayBuffer !== "undefined" && flash.buffer instanceof SharedArrayBuffer && flash.byteLength >= chip.flash.length) {
      chip.flash = new Uint8Array(flash.buffer, flash.byteOffset, chip.flash.length);
      if (DEBUG_BOOT) console.log(`[DBG] Flash SAB-backed: ${chip.flash.length} bytes`);
    } else {
      const off = chip._firmwareOffset || 0;
      chip.flash.set(flash, off);
      if (DEBUG_BOOT) console.log(`[DBG] Flash copied: ${flash.length} bytes to offset ${off}, flash[0]=${chip.flash[0].toString(16)}`);
    }
  } else if (DEBUG_BOOT) console.log(`[DBG] No flash to load (flash=${!!flash} chip.flash=${!!chip.flash})`);
  if (config.partitions && chip.flash) writePartitionTable(chip.flash, config.partitions);
  if (rom && chip.loadROM) {
    chip.loadROM(rom);
    if (DEBUG_BOOT) console.log(`[DBG] ROM loaded: ${rom.length} bytes, ROM[0]=${chip.chipROM?.[0]?.toString(16)}`);
  }
  let mac = null;
  if (config.macAddress) mac = parseMacAddress(config.macAddress);
  if (chip.reset) chip.reset();
  // ── Re-apply post-reset (reset wipes MMU, GPIO, UART, core state) ──
  if (chip.mmuTablePro && config.mmuPages) {
    for (let p = 0; p < config.mmuPages; p++) chip.mmuTablePro[p] = p;
    if (chip.mmuTableApp) {
      for (let p = 0; p < config.mmuPages; p++) chip.mmuTableApp[p] = p;
    }
  }
  if (chip.gpio) {
    if (config.strapValue !== undefined) chip.gpio.strapValue = config.strapValue;
    if (config.pinInputs) {
      for (const [pin, val] of Object.entries(config.pinInputs)) {
        if (chip.gpio.pins?.[pin]) chip.gpio.pins[pin].inputValue = val;
      }
    }
  }
  if (config.analogInputs) {
    for (const [pin, volts] of Object.entries(config.analogInputs)) {
      chip.setAnalogInput?.(Number(pin), volts);
    }
  }
  if (config.touchInputs) {
    for (const [pad, count] of Object.entries(config.touchInputs)) {
      chip.setTouchInput?.(Number(pad), count);
    }
  }
  if (chip.uart?.[0]) {
    chip.uart[0].onTX = (byte) => { uartWriteByte(byte); };
  }
  if (chip.cores?.[1]) chip.cores[1].enabled = true;
  try { chip.cores[0].writeUint32(0x3ff5a104, 0x5aa5); } catch(_) {}
  if (mac) {
    try { chip.cores[0].writeUint32(0x3ff5a004, ((mac[2]<<24)|(mac[3]<<16)|(mac[4]<<8)|mac[5])>>>0); } catch(_) {}
    try {
      let crc = 0;
      for (let i = 0; i < mac.length; i++) {
        crc ^= mac[i];
        for (let j = 0; j < 8; j++) { const lsb = crc & 1; crc >>= 1; if (lsb) crc ^= 140; }
      }
      chip.cores[0].writeUint32(0x3ff5a008, ((crc << 16) | (mac[0] << 8) | mac[1]) >>> 0);
    } catch(_) {}
  }
}

// Native WiFi AP wiring — Rust NativeInternetAP port (wifi_bridge.rs) with
// JS-side gateway WebSocket + pcap records. Called after loadWasm + reset.
function setupNativeWifiBridge(chip, config) {
  if (config.wifi === false) return;
  const loader = chip._wasmLoader;
  const mem = chip._wasmMemory;
  const boardMac = config.macAddress ? parseMacAddress(config.macAddress) : null;
  const wifiOpts = config.wifi === true ? {} : (config.wifi || {});
  const ssidB = new TextEncoder().encode(wifiOpts.ssid || 'ESP-GUEST');
  const bssidB = wifiOpts.bssid ? parseMacAddress(wifiOpts.bssid) : null;
  const scratch = loader.exports.native_wifi_ap_scratch();
  const sv = new Uint8Array(mem, scratch, 128);
  sv.set(ssidB, 0);
  const bssidOff = 64;
  if (bssidB) sv.set(bssidB, bssidOff);
  loader.exports.native_wifi_ap_init(scratch, ssidB.length, scratch + bssidOff, bssidB ? 6 : 0, wifiOpts.channel || 6, 0);
  // Gateway WebSocket + pcap records (host I/O stays in JS; the Rust AP
  // owns the 802.11 state machine and the status counters).
  const status = { state: 0, errorMessage: '', txFrames: 0, txBytes: 0, rxFrames: 0, rxBytes: 0, probeRequestCount: 0, connectedClients: 0, ip: '', portForward: '', udpForward: '' };
  const pcapBuffer = [];
  const packetBuffer = [];
  let socket = null;
  // Shared gateway room for ESP-NOW delivery: boards with the same
  // wifi.room join one L2 hub room so action frames reach each other.
  const roomQuery = wifiOpts.room ? `?sessionId=${encodeURIComponent(wifiOpts.room)}` : '';
  const connectGateway = () => {
    if (socket) return;
    try {
      socket = new WebSocket(`ws://127.0.0.1:5085/api/network-gateway${roomQuery}`);
      socket.binaryType = 'arraybuffer';
      socket.onopen = () => { while (packetBuffer.length > 0) socket.send(packetBuffer.shift()); };
      socket.onmessage = (ev) => {
        if (ev.data instanceof ArrayBuffer) {
          const bytes = new Uint8Array(ev.data);
          // ESP-NOW medium frames (magic-prefixed raw 802.11): strip and
          // inject into the wifi MAC RX path; never touch pcap/gVisor.
          if (bytes.length > 4 && bytes[0] === 0xE5 && bytes[1] === 0x50 && bytes[2] === 0x4E && bytes[3] === 0x57) {
            const raw = bytes.subarray(4);
            const v = new Uint8Array(mem, scratch, raw.length);
            v.set(raw);
            loader.exports.native_wifi_mac_rx_frame(scratch, raw.length, 0);
            return;
          }
          const eth = bytes;
          pcapBuffer.push({ timeUs: performance.now() * 1000, data: eth });
          const v = new Uint8Array(mem, scratch, eth.length);
          v.set(eth);
          loader.exports.native_wifi_ap_eth_rx(eth.length);
        } else if (typeof ev.data === 'string') {
          const msg = ev.data;
          if (msg.startsWith('BOARD_IP:')) status.ip = msg.substring(9);
          else if (msg.startsWith('PORT_FORWARD:')) status.portForward = msg.substring(13);
          else if (msg.startsWith('UDP_FORWARD:')) status.udpForward = msg.substring(12);
          console.log(msg);
        }
      };
      socket.onerror = () => { status.state = 4; status.errorMessage = 'Gateway connection failed'; };
    } catch(_) {}
  };
  loader._wifiApRxFrame = (ptr, len) => { loader.exports.native_wifi_mac_rx_frame(ptr, len, 0); };
  loader._wifiApSendEth = (ptr, len) => {
    const eth = new Uint8Array(mem, ptr, len).slice();
    pcapBuffer.push({ timeUs: performance.now() * 1000, data: eth });
    if (socket && socket.readyState === WebSocket.OPEN) socket.send(eth);
    else packetBuffer.push(eth);
  };
  loader._wifiApConnected = () => { connectGateway(); };
  // ESP-NOW medium TX: magic-prefix the raw 802.11 MPDU and send it to the
  // shared room; the gateway broadcasts room frames to peers (skipping
  // gVisor for marked frames) and the peer injects above.
  const ESPNOW_MAGIC = [0xE5, 0x50, 0x4E, 0x57];
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
  // Beacon every 102ms — sim-time (clock event), parity with the JS bridge.
  const beaconEvent = chip.clocks.cpu.createEvent(() => {
    loader.exports.native_wifi_ap_send_beacon();
    beaconEvent.schedule(102e6);
  });
  beaconEvent.schedule(0);
  loader._wifiMacTx = (frame) => {
    if (frame.length >= 24 && boardMac && frame.slice(10, 16).every(b => b === 0)) frame.set(boardMac, 10);
    const v = new Uint8Array(mem, scratch, frame.length);
    v.set(frame);
    loader.exports.native_wifi_ap_handle_tx(frame.length, 0);
  };
  loader.exports.native_wifi_mac_rx_force(1, 1);
  connectGateway();
  chip._wifiBridge = {
    ap: {
      status,
      getPcapData: () => {
        const header = new ArrayBuffer(24);
        const h = new DataView(header);
        h.setUint32(0, 0xa1b2c3d4, false);
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
          const sec = Math.floor(p.timeUs / 1000000);
          const usec = Math.floor(p.timeUs % 1000000);
          // Big-endian throughout to match the magic above — mixed
          // endianness made standard tools (Wireshark/tshark) misparse
          // every record (pcap was header-BE + records-LE).
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
        for (const a of parts) { out.set(a, off); off += a.length; }
        return out;
      },
      clearPcapBuffer: () => { pcapBuffer.length = 0; },
    },
  };
  chip._syncWifiStatus = syncWifiStatus;
}

const BOOTROM_PATHS = {
  ESP32: '../rom/esp32-v3-rom.bin',
};

const WASM_PATHS = {
  ESP32: '../engine/esp-xtensa/esp_engine_wasm.wasm',
};

async function loadBinary(config, chipType, name) {
  const pathMap = name === 'bootrom' ? BOOTROM_PATHS : WASM_PATHS;
  const _fallbackKey = name === 'bootrom' ? 'bootrom' : 'wasmBinary';
  const configKey = name === 'bootrom' ? 'bootrom' : 'wasmBinary';
  if (config[configKey]) {
    return config[configKey];
  }
  const relPath = pathMap[chipType] || pathMap.ESP32;
  // Browser / worker-over-http context: fetch the binary (single-file friendly).
  try {
    const url = new URL(relPath, import.meta.url);
    if ((url.protocol === 'http:' || url.protocol === 'https:' || url.protocol === 'data:') && typeof fetch === 'function') {
      const res = await fetch(url);
      if (res.ok) return new Uint8Array(await res.arrayBuffer());
    }
  } catch (_) { /* fall through to filesystem */ }
  // Node context: read the binary from disk.
  try {
    const { readFileSync } = await import('fs');
    const { resolve, dirname } = await import('path');
    const { fileURLToPath } = await import('url');
    const __dirname = dirname(fileURLToPath(import.meta.url));
    return readFileSync(resolve(__dirname, relPath));
  } catch(_e) {
    setError(`Failed to load ${name} binary: ` + _e.message);
    return null;
  }
}

function createDefaultFlash() {
  const size = 4 * 1024 * 1024;
  const flash = new Uint8Array(size);
  flash.fill(0xff);
  flash[0x1000] = 0xe9; flash[0x1001] = 0x01; flash[0x1002] = 0x20; flash[0x1003] = 0x40;
  flash[0x1004] = 0x00; flash[0x1005] = 0x00; flash[0x1006] = 0x00; flash[0x1007] = 0x40;
  flash[0x100c] = 0x00; flash[0x100d] = 0x00; flash[0x100e] = 0x00; flash[0x100f] = 0x00;
  flash[0x1014] = 0x00; flash[0x1015] = 0x00; flash[0x1016] = 0x00; flash[0x1017] = 0x40;
  flash[0x1018] = 0x04; flash[0x1019] = 0x00; flash[0x101a] = 0x00; flash[0x101b] = 0x00;
  flash[0x101c] = 0x00; flash[0x101d] = 0x00; flash[0x101e] = 0x00; flash[0x101f] = 0x00;
  flash[0x1020] = 0xef;
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
      if (errSab) { errMsg = new Uint8Array(errSab); errMsg.fill(0); }
      if (debugSab) debugData = new Uint32Array(debugSab);
      const ChipClass = chipConstructors[chipType];
      if (!ChipClass) {
        setError("Unknown chip: " + chipType);
        send({ type: "ready", memoryRegions: {}, msgId });
        if (config.simMode === 'blocking') { setTimeout(() => blockingCommandLoop(), 0); }
        else { commandLoop(); }
        return;
      }
      try {
        const engineMode = config.engine || ENGINE;

        // Use defaults when not provided by host (don't reassign const destructured vars)
        const loadFlash = flash || createDefaultFlash();
        const loadRom = rom || await loadBinary(config, chipType, 'bootrom');

        chip = new ChipClass(config);
        if (DEBUG_BOOT) console.log(`[DBG] Chip created, ROM=${!!chip.chipROM} flashLen=${chip.flash?.length}`);
        applyBasicSetup(chip, config, loadFlash, loadRom);
        if (DEBUG_BOOT) console.log(`[DBG] after applyBasicSetup: PC=0x${chip.cores?.[0]?.PC?.toString(16).padStart(8,'0')} flash[0]=${chip.flash?.[0]?.toString(16)} ROM[0]=${chip.chipROM?.[0]?.toString(16)}`);

        {
          const wasmBinary = await loadBinary(config, chipType, 'wasmBinary');
          if (wasmBinary) {
            if (DEBUG_BOOT) console.log(`[DBG] Loading WASM (${wasmBinary.length} bytes) from ${WASM_PATHS[chipType] || WASM_PATHS.ESP32}...`);
            await chip.loadWasm(wasmBinary, 'wasm');
            if (config.pcTrace && chip._wasmLoader?.exports?.native_pc_trace) {
              try { chip._wasmLoader.exports.native_pc_trace(config.pcTrace); } catch(_) {}
            }
            if (DEBUG_BOOT) console.log(`[DBG] WASM loaded, resetting chip...`);
            chip.reset();
            // chip.reset() fills mmuTablePro/App with 0x100 (invalid pages) and clears all
            // peripheral state — re-apply everything applyBasicSetup had configured:

            console.log(`[DBG] WASM: about to re-seed MMU, mmuTablePro=${!!chip.mmuTablePro} mmuPages=${config.mmuPages}`);
            // 1. Re-seed MMU so flash pages are valid (matches applyBasicSetup lines 327-331)
            if (chip.mmuTablePro && config.mmuPages) {
              for (let p = 0; p < config.mmuPages; p++) chip.mmuTablePro[p] = p;
              if (chip.mmuTableApp) {
                for (let p = 0; p < config.mmuPages; p++) chip.mmuTableApp[p] = p;
              }
              if (DEBUG_BOOT) console.log(`[DBG] WASM: MMU re-seeded (${config.mmuPages} pages)`);
            }

            // 2. Re-enable core 1 (reset disables it; applyBasicSetup re-enables at line 347)
            if (chip.cores?.[1]) chip.cores[1].enabled = true;
            if (chip._wasmCores?.[1]) chip._wasmCores[1].enabled = true;

            // 3. Re-apply GPIO strap value and pin inputs (reset clears GPIO state)
            if (chip.gpio) {
              if (config.strapValue !== undefined) chip.gpio.strapValue = config.strapValue;
              if (config.pinInputs) {
                for (const [pin, val] of Object.entries(config.pinInputs)) {
                  if (chip.gpio.pins?.[pin]) chip.gpio.pins[pin].inputValue = val;
                }
              }
              // Push JS pin input levels + strap into the native GpioController
              // (one-shot bulk seed via GPIO_SEED_SCRATCH)
              chip._wasmLoader?.seedNativeGpio?.();
            }

            // 4. Re-hook UART TX callback (reset clears peripheral state)
            if (chip.uart?.[0]) {
              chip.uart[0].onTX = (byte) => { uartWriteByte(byte); };
            }

            // 5. Re-apply DPORT clock gate write (applyBasicSetup line 348)
            try { chip.cores[0].writeUint32(0x3ff5a104, 0x5aa5); } catch(_) {}

            // 6. Re-apply MAC (reset zeroes all peripheral memory — wipes WifiMac regs 64/68 + eFuse)
            if (config.macAddress) {
              const mac = parseMacAddress(config.macAddress);
              const lo = (mac[0] | (mac[1] << 8) | (mac[2] << 16) | (mac[3] << 24)) >>> 0;
              const hi = (mac[4] | (mac[5] << 8)) >>> 0;
              // WifiMac MAC regs (base+64/68) — now owned by the native register
              // file; the JS mirror is inert (host-MAC writes via native export).
              if (config.vddMv !== undefined) chip.setVddMv?.(config.vddMv);
            if (chip._wasmLoader?.exports?.native_wifi_mac_set_mac) {
                chip._wasmLoader.exports.native_wifi_mac_set_mac(lo, hi);
              }
              try { chip.cores[0].writeUint32(0x3ff5a004, ((mac[2]<<24)|(mac[3]<<16)|(mac[4]<<8)|mac[5])>>>0); } catch(_) {}
              try {
                let crc = 0;
                for (let i = 0; i < mac.length; i++) {
                  crc ^= mac[i];
                  for (let j = 0; j < 8; j++) { const lsb = crc & 1; crc >>= 1; if (lsb) crc ^= 140; }
                }
                chip.cores[0].writeUint32(0x3ff5a008, ((crc << 16) | (mac[0] << 8) | mac[1]) >>> 0);
              } catch(_) {}
            }

            // 7. Virtual SD card image (constructor allocates from config.sdCard;
            // re-allocate here if missing so post-reset state always matches).
            if (config.sdCard !== false && !chip.sdData) {
              const opt = config.sdCard === true ? {} : (config.sdCard || {});
              const blocks = opt.blocks ?? Math.max(1024, Math.round(((opt.sizeMB ?? 16) * 1024 * 1024) / 512));
              chip.sdData = new Uint8Array(new SharedArrayBuffer(blocks * 512));
              if (opt.image instanceof Uint8Array) chip.sdData.set(opt.image.subarray(0, chip.sdData.length));
            }
            if (config.sdCard === false) chip.sdData = null;
            // eMMC card type (SD probe starved -> MMC path). Re-applied
            // after every reset alongside the image above.
            try {
              const sdOpt = config.sdCard === true ? {} : (config.sdCard || {});
              const mmc = (sdOpt.type === 'mmc') ? 1 : 0;
              chip._wasmLoader?.exports?.native_sdmmc_set_card_mmc?.(mmc);
            } catch(_) {}

            // 8. Native WiFi AP (NativeInternetAP port) — must run AFTER
            // loadWasm + reset (needs chip._wasmLoader exports).
            setupNativeWifiBridge(chip, config);

            // 9. Virtual-camera frame size (finite sensor frame per capture;
            // host-known, e.g. 160*120*2 for QQVGA RGB565).
            if (config.camFrameBytes !== undefined) {
              try { chip._wasmLoader?.exports?.native_i2s_cam_frame_bytes?.(0, config.camFrameBytes >>> 0); } catch(_) {}
            }

            // 10. Host PSRAM size (board config) for the SPI1 JEDEC ID path
            // so firmware PSRAM detection (psramFound()) sees the chip.
            try { chip._wasmLoader?.exports?.native_spi_set_psram_size?.(chip._psramSizeMB >>> 0); } catch(_) {}

            if (DEBUG_BOOT) console.log(`[DBG] After reset: PC=0x${chip._wasmCores?.[0]?.PC?.toString(16).padStart(8,'0')} JS_PC=0x${chip.cores?.[0]?.PC?.toString(16).padStart(8,'0')}`);
            // Check PTEs after reset (after chip.reset() which might corrupt them)
            // run265e: use the REAL page-table base (WM.PAGE_TABLE_OFFSET =
            // STATIC_ZONE = 4MB). The old hardcoded 82048 predates the
            // static-zone move and reads an unrelated struct (always 0).
            try {
              const SAB = chip._wasmMemory || (chip._wasmLoader && chip._wasmLoader.memory && chip._wasmLoader.memory.buffer);
              console.log(`[DBG] SAB=${!!SAB} _wasmMemory=${!!chip._wasmMemory} _wasmLoader=${!!chip._wasmLoader}`);
              if (SAB) {
                const { PAGE_TABLE_OFFSET } = await import('../engine/wasm-memory-layout.js');
                const ptU32Check = new Uint32Array(SAB, PAGE_TABLE_OFFSET);
                const t0 = ptU32Check[0x3FF5F * 2];
                const t0d = ptU32Check[0x3FF5F * 2 + 1];
                const s0 = ptU32Check[0x3FF03 * 2];
                const s0d = ptU32Check[0x3FF03 * 2 + 1];
                console.log(`[DBG] After reset PTE: TIMG0 type=${t0} data=0x${(t0d>>>0).toString(16)} SHA type=${s0} data=0x${(s0d>>>0).toString(16)}`);
              }
            } catch(_e) { console.log(`[DBG] PTE check error: ${_e.message}`); }
          }
        }

        if (config.simMode === 'chunked') needsEventLoop = true;
        else if (config.simMode === 'tight') needsEventLoop = false;
        else if (config.simMode === 'blocking') needsEventLoop = false;
        if (typeof config.chunkSize === 'number' && config.chunkSize > 0) simChunkSize = config.chunkSize;
        if (config.nanos !== undefined) { chip.cycles = Math.round(Number(config.nanos) / 1e9 * 160e6); syncClockState(); }
        {
          const cf = config.cpuFrequency;
          const native = chip._nativeFrequency || 160e6;
          ctrl[SAB_CPU_FREQ] = !cf || cf === "max" ? native : (cf === "auto" ? 8e6 : Number(cf) * 1e6);
        }
        writeSABState();
        send({ type: "ready", memoryRegions: getMemorySABs(chip), engineMode, chipInfo: { board: chip.board || 'esp32', flashSizeMB: chip._flashSizeMB, psramSizeMB: chip._psramSizeMB, psramType: chip._psramType, cpuFrequency: 160e6, _nativeFrequency: chip._nativeFrequency }, msgId });
        if (config.simMode === 'blocking') { setTimeout(() => blockingCommandLoop(), 0); }
        else { setTimeout(() => commandLoop(), 0); }
      } catch (err) {
        setError(err.message);
        send({ type: "ready", memoryRegions: {}, msgId });
        if (config.simMode === 'blocking') { setTimeout(() => blockingCommandLoop(), 0); }
        else { setTimeout(() => commandLoop(), 0); }
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
