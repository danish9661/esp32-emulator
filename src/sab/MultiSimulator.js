// MultiSimulator — runs N independent chip instances, each with its own
// memory and peripherals. No SAB aliasing, no engine conflicts.
//
// Two modes:
//   sync (default) — all nodes step in-process, one at a time
//   parallel — each node runs in its own worker_thread for full speed

import { fileURLToPath } from 'url';
import { dirname, resolve as resolvePath } from 'path';

const __dirname = dirname(fileURLToPath(import.meta.url));

export class MultiSimulator {
  // Parallel sync granularity — tradeoff: lower = tighter sync, higher = faster
  //   1000  = ~10M steps/sec, sync every 6μs at 160MHz
  //   5000  = ~15M steps/sec, sync every 31μs (default — best balance)
  //   10000 = ~15M steps/sec, sync every 62μs
  //   50000 = ~16M steps/sec, sync every 312μs
  constructor() {
    this.nodes = [];
    this._workers = [];
    this._msgCounter = 0;
    this.batchSize = 5000;
    this._UART_RING_SIZE = 16384;
  }

  // ── Synchronous mode (in-process) ──

  addNode(ChipClass, config = {}) {
    const idx = this.nodes.length;
    this.nodes.push({ chip: new ChipClass(config), wasmLoaded: false });
    return idx;
  }

  async loadWasm(nodeIdx, wasmBytes, mode = 'wasm') {
    const node = this.nodes[nodeIdx];
    await node.chip.loadWasm(wasmBytes, mode);
    node.wasmLoaded = true;
  }

  step() {
    for (const node of this.nodes) {
      if (node.chip) node.chip.step();
    }
  }

  run(steps) {
    for (let i = 0; i < steps; i++) this.step();
  }

  chip(nodeIdx) { return this.nodes[nodeIdx]?.chip; }
  cycles(nodeIdx) { return this.nodes[nodeIdx]?.chip?.cycles ?? 0; }

  // ── Parallel mode (worker threads with SAB command channel) ──
  // Single-node optimization: if only 1 node, runs in-process with no worker

  async addNodeParallel(chipType, config = {}, flash = null, rom = null, wasmBinary = null) {
    const idx = this.nodes.length;

    const { Worker } = await import('worker_threads');
    const worker = new Worker(resolvePath(__dirname, 'worker-entry.js'), { type: 'module' });

    this._workers.push(worker);
    this.nodes.push({ chip: null, wasmLoaded: false, _parallel: true });

    // Allocate SABs matching worker-entry.js layout
    const ctrlSab = new SharedArrayBuffer(24 * 4);
    const ctrlView = new Int32Array(ctrlSab);
    ctrlView[9] = 0; ctrlView[12] = 0;
    this._workers[this._workers.length - 1]._ctrlSab = ctrlSab;
    this._workers[this._workers.length - 1]._ctrlView = ctrlView;

    const uartSabSize = 8 + this._UART_RING_SIZE;
    const uartSab = new SharedArrayBuffer(uartSabSize);
    const uartView = new Int32Array(uartSab, 0, 2);
    uartView[0] = 0; uartView[1] = this._UART_RING_SIZE;
    this._workers[this._workers.length - 1]._uartSab = uartSab;

    const errSab = new SharedArrayBuffer(256);
    this._workers[this._workers.length - 1]._errSab = errSab;

    const debugSab = new SharedArrayBuffer(2000);
    this._workers[this._workers.length - 1]._debugSab = debugSab;

    const readRespSab = new SharedArrayBuffer(65536);
    this._workers[this._workers.length - 1]._readRespSab = readRespSab;

    const msgId = this._msgCounter++;
    const result = await new Promise((resolve, reject) => {
      worker.on('message', (msg) => {
        if (msg.msgId === msgId) resolve(msg);
      });
      worker.on('error', reject);
      worker.postMessage({ type: 'init', msgId, chipType,
        config: { ...config, simMode: 'blocking' },
        flash, rom, wasmBinary,
        sab: ctrlSab, uartSab, errSab, readRespSab, debugSab
      });
    });

    if (result.error) throw new Error(result.error);
    return idx;
  }

  // Run all workers in CMD_RUN mode (idle-aware, like the proxy test)
  // onPoll(sim) is called every pollInterval ms; return true to stop
  async runParallel({ onPoll = null, pollInterval = 50 } = {}) {
    if (this._workers.length === 0) return;
    const SAB_CMD = 9, SAB_RESP = 12;
    const CMD_RUN = 1, RESP_IDLE = 0;

    for (const w of this._workers) {
      const ctrl = w._ctrlView;
      Atomics.store(ctrl, 0, 1); // SAB_RUN
      Atomics.store(ctrl, SAB_RESP, RESP_IDLE);
      Atomics.store(ctrl, SAB_CMD, CMD_RUN);
      Atomics.notify(ctrl, SAB_CMD, 1);
    }

    while (true) {
      await new Promise(r => setTimeout(r, pollInterval));

      let allStopped = true;
      for (const w of this._workers) {
        if (Atomics.load(w._ctrlView, 0) !== 0) { allStopped = false; break; }
      }
      if (allStopped) break;

      if (onPoll && onPoll(this)) break;
    }

    for (const w of this._workers) {
      Atomics.store(w._ctrlView, 0, 0);
      Atomics.wait(w._ctrlView, SAB_RESP, RESP_IDLE);
    }
  }

  stepParallel(batchSize) {
    if (batchSize === undefined) batchSize = this.batchSize;
    if (this._workers.length === 0) return;
    const SAB_CMD = 9, SAB_CMD_ARG0 = 10, SAB_RESP = 12;
    const CMD_STEP = 8, RESP_IDLE = 0;
    for (const w of this._workers) {
      const ctrl = w._ctrlView;
      Atomics.store(ctrl, SAB_CMD_ARG0, batchSize);
      Atomics.store(ctrl, SAB_RESP, RESP_IDLE);
      Atomics.store(ctrl, SAB_CMD, CMD_STEP);
      Atomics.notify(ctrl, SAB_CMD, 1);
    }
    for (const w of this._workers) {
      const ctrl = w._ctrlView;
      Atomics.wait(ctrl, SAB_RESP, RESP_IDLE);
    }
  }

  // ── UART output (parallel workers) ──

  getUART(nodeIdx) {
    const w = this._workers[nodeIdx];
    if (!w || !w._uartSab) return '';
    const ctrl = new Int32Array(w._uartSab, 0, 2);
    const writeIdx = ctrl[0];
    const ringSize = ctrl[1] || this._UART_RING_SIZE;
    const ring = new Uint8Array(w._uartSab, 8, ringSize);
    const buf = writeIdx > ringSize ? ringSize : writeIdx;
    const start = writeIdx >= ringSize ? writeIdx % ringSize : 0;
    const out = [];
    for (let i = 0; i < buf; i++) {
      const b = ring[(start + i) % ringSize];
      if (b === 0 && i === buf - 1) continue;
      if (b >= 32 && b <= 126) out.push(String.fromCharCode(b));
      else if (b === 10) out.push('\n');
    }
    return out.join('');
  }

  // ── Comparison (sync + parallel via debug SAB) ──

  comparePC(nodeIdxA, nodeIdxB, coreIdx = 0) {
    const a = this.nodes[nodeIdxA]?.chip?.cores?.[coreIdx]?.PC >>> 0;
    const b = this.nodes[nodeIdxB]?.chip?._wasmCores?.[coreIdx]?.PC ?? this.nodes[nodeIdxB]?.chip?.cores?.[coreIdx]?.PC >>> 0;
    return a === b ? null : { a, b };
  }

  comparePhysRegs(nodeIdxA, nodeIdxB, coreIdx = 0, count = 64) {
    const a = this.nodes[nodeIdxA]?.chip?.cores?.[coreIdx]?.physicalRegisters;
    const b = this.nodes[nodeIdxB]?.chip?._wasmCores?.[coreIdx]?._physRegs ?? a;
    if (!a || !b) return [];
    const diff = [];
    for (let i = 0; i < count; i++) {
      if (a[i] !== b[i]) diff.push({ index: i, a: a[i], b: b[i] });
    }
    return diff;
  }

  compareAll(nodeIdxA, nodeIdxB, coreIdx = 0) {
    const pc = this.comparePC(nodeIdxA, nodeIdxB, coreIdx);
    const phys = this.comparePhysRegs(nodeIdxA, nodeIdxB, coreIdx);
    return { pc, phys, match: !pc && phys.length === 0 };
  }

  // Debug SAB access — raw Uint32Array
  debugView(nodeIdx) {
    const w = this._workers[nodeIdx];
    if (!w?._debugSab) return null;
    return new Uint32Array(w._debugSab);
  }

  // Structured snapshot of one node's full debug state
  debugState(nodeIdx) {
    const dv = this.debugView(nodeIdx);
    if (!dv) return null;
    const DBG_PC0 = 0, DBG_PC1 = 1, DBG_CYCLES = 2, DBG_NANOS_LO = 3, DBG_NANOS_HI = 4;
    const DBG_TICKS = 5, DBG_ENABLED0 = 6, DBG_ENABLED1 = 7;
    const DBG_PHYS_START = 16, DBG_ALL_SPEC_START = 96;
    const DBG_MMU_PRO_START = 352, DBG_MMU_APP_START = 416;
    const nanos = BigInt(dv[DBG_NANOS_HI]) * 4294967296n + BigInt(dv[DBG_NANOS_LO]);
    const phys = [];
    for (let i = 0; i < 64; i++) phys.push(dv[DBG_PHYS_START + i] >>> 0);
    const spec = [];
    for (let i = 0; i < 256; i++) spec.push(dv[DBG_ALL_SPEC_START + i] >>> 0);
    const mmuPro = [];
    for (let i = 0; i < 64; i++) mmuPro.push(dv[DBG_MMU_PRO_START + i] >>> 0);
    const mmuApp = [];
    for (let i = 0; i < 64; i++) mmuApp.push(dv[DBG_MMU_APP_START + i] >>> 0);
    const DBG_MEM_HASH_START = 480, DBG_PERIPH_START = 484;
    const memHashes = {};
    const memHashLabels = ['iram', 'dram', 'rtcFast', 'sram'];
    for (let i = 0; i < 4; i++) memHashes[memHashLabels[i]] = dv[DBG_MEM_HASH_START + i] >>> 0;
    const periph = [];
    for (let i = 0; i < 16; i++) periph.push(dv[DBG_PERIPH_START + i] >>> 0);
    return {
      pc0: dv[DBG_PC0] >>> 0, pc1: dv[DBG_PC1] >>> 0,
      cycles: dv[DBG_CYCLES] >>> 0, nanos: nanos.toString(),
      ticks: dv[DBG_TICKS] >>> 0,
      enabled0: !!dv[DBG_ENABLED0], enabled1: !!dv[DBG_ENABLED1],
      physicalRegisters: phys,
      specialRegisters: spec,
      mmuPro, mmuApp,
      memHashes, periph
    };
  }

  // Register name lookup for rich reporting
  static _specRegNames = {
    0: 'LBEG', 1: 'LEND', 2: 'LCOUNT',
    3: 'SAR', 32: 'BR', 72: 'PS', 73: 'IP',
    74: 'CONFIGID', 75: 'BOOT', 76: 'CACHEATTR',
    77: 'INTERRUPT', 80: 'THREADPTR',
    81: 'THREADCTRL', 82: 'DATAR', 83: 'MISC',
    90: 'EXCCAUSE', 91: 'DEBUGCAUSE',
    92: 'ICOUNT', 93: 'ICOUNTLEVEL',
    96: 'DBREAKA0', 97: 'DBREAKA1',
    98: 'DREAKC0', 99: 'DREAKC1',
    104: 'IBREAKA0', 105: 'IBREAKA1',
    112: 'CACHEATTR0', 113: 'CACHEATTR1',
    114: 'CACHEATTR2', 115: 'CACHEATTR3',
    116: 'CACHEATTR4', 117: 'CACHEATTR5',
    118: 'CACHEATTR6', 119: 'CACHEATTR7',
    128: 'ME0', 129: 'ME1', 130: 'ME2', 131: 'ME3',
    132: 'ME4', 133: 'ME5', 134: 'ME6', 135: 'ME7',
    136: 'ME8', 137: 'ME9', 138: 'ME10', 139: 'ME11',
    140: 'ME12', 141: 'ME13', 142: 'ME14', 143: 'ME15',
    144: 'ME16', 145: 'ME17', 146: 'ME18', 147: 'ME19',
    148: 'ME20', 149: 'ME21',
    160: 'M0', 161: 'M1', 162: 'M2', 163: 'M3',
    164: 'M4', 165: 'M5', 166: 'M6', 167: 'M7',
    168: 'M8', 169: 'M9', 170: 'M10', 171: 'M11',
    172: 'M12', 173: 'M13', 174: 'M14', 175: 'M15',
    176: 'M16', 177: 'M17', 178: 'M18', 179: 'M19',
    180: 'M20', 181: 'M21', 182: 'M22', 183: 'M23',
    184: 'M24', 185: 'M25', 186: 'M26', 187: 'M27',
    188: 'M28',
    192: 'EXPSTATE',
    224: 'VECBASE', 225: 'EBREAKCAUSE',
    226: 'EPC1', 227: 'EPC2',
    228: 'EPC3', 229: 'EPC4',
    230: 'EPC5', 231: 'EPC6',
    232: 'EPC7',
    233: 'DEPC',
    234: 'CCOUNT', 235: 'CCOMPARE',
    236: 'MECCOUNT', 237: 'MEPC',
    238: 'MEPS', 239: 'MESAVE',
    240: 'EXCSAVE1', 241: 'EXCSAVE2', 242: 'EXCSAVE3',
    243: 'EXCSAVE4', 244: 'EXCSAVE5', 245: 'EXCSAVE6',
    246: 'EXCSAVE7',
    247: 'EPS1', 248: 'EPS2', 249: 'EPS3',
    250: 'EPS4', 251: 'EPS5', 252: 'EPS6',
    253: 'EPS7',
    254: 'CPENABLE',
  };

  static _physRegName(i) {
    if (i < 16) return `a${i}`;
    return `a${i}`;
  }

  // Gather raw differences between two debug SABs
  debugDiffs(nodeIdxA, nodeIdxB) {
    const da = this.debugView(nodeIdxA);
    const db = this.debugView(nodeIdxB);
    if (!da || !db) return { error: 'debug SAB not available' };
    const DBG_PC0 = 0, DBG_PC1 = 1, DBG_CYCLES = 2, DBG_TICKS = 5;
    const DBG_PHYS_START = 16, DBG_ALL_SPEC_START = 96;
    const DBG_MMU_PRO_START = 352, DBG_MMU_APP_START = 416;
    const DBG_MEM_HASH_START = 480, DBG_PERIPH_START = 484;

    const diffs = {};
    const collect = (label, start, count) => {
      const items = [];
      for (let i = 0; i < count; i++) {
        if (da[start + i] !== db[start + i])
          items.push({ index: i, a: da[start + i], b: db[start + i] });
      }
      if (items.length) diffs[label] = items;
    };
    if (da[DBG_PC0] !== db[DBG_PC0]) diffs.pc0 = { a: da[DBG_PC0], b: db[DBG_PC0] };
    if (da[DBG_PC1] !== db[DBG_PC1]) diffs.pc1 = { a: da[DBG_PC1], b: db[DBG_PC1] };
    if (da[DBG_CYCLES] !== db[DBG_CYCLES]) diffs.cycles = { a: da[DBG_CYCLES], b: db[DBG_CYCLES] };
    if (da[DBG_TICKS] !== db[DBG_TICKS]) diffs.ticks = { a: da[DBG_TICKS], b: db[DBG_TICKS] };
    collect('phys', DBG_PHYS_START, 64);
    collect('spec', DBG_ALL_SPEC_START, 256);
    collect('mmuPro', DBG_MMU_PRO_START, 64);
    collect('mmuApp', DBG_MMU_APP_START, 64);
    collect('memHash', DBG_MEM_HASH_START, 4);
    collect('periph', DBG_PERIPH_START, 16);
    return diffs;
  }

  // Rich default report formatter
  formatReport(diffs) {
    if (!diffs || Object.keys(diffs).length === 0) return 'No differences.';
    const lines = [];
    lines.push(''); // blank line before
    const addScalar = (label, val) => {
      if (!val) return;
      lines.push(`  ${label.padEnd(10)} JS=0x${val.a.toString(16).padStart(8, '0')}  WM=0x${val.b.toString(16).padStart(8, '0')}`);
    };
    addScalar('PC0', diffs.pc0);
    addScalar('PC1', diffs.pc1);
    addScalar('CYCLES', diffs.cycles);
    addScalar('TICKS', diffs.ticks);

    const periphLabels = ['GPIO_OUT0','GPIO_OUT1','GPIO_ENABLE','GPIO_IN','GPIO_STATUS',
      'UART0_STATUS','UART1_STATUS','TIMG0_T0LO','TIMG0_T0HI','reserved',
      'reserved','reserved','reserved','reserved','reserved','reserved'];
    const memHashLabels = ['IRAM','DRAM','RTC_FAST','SRAM'];

    for (const [label, items] of Object.entries(diffs)) {
      if (label === 'pc0' || label === 'pc1' || label === 'cycles' || label === 'ticks') continue;
      if (!items || !items.length) continue;
      const nameMap = label === 'spec' ? MultiSimulator._specRegNames : null;
      const itemLabels = label === 'periph' ? periphLabels : label === 'memHash' ? memHashLabels : null;
      const header = label === 'phys' ? 'Physical Registers' :
        label === 'spec' ? 'Special Registers' :
        label === 'mmuPro' ? 'MMU PRO Table' :
        label === 'mmuApp' ? 'MMU APP Table' :
        label === 'memHash' ? 'Memory Hashes' :
        label === 'periph' ? 'Peripheral Registers' : label;
      lines.push(`  ${header} (${items.length} diff${items.length > 1 ? 's' : ''}):`);
      const show = items.length > 12 ? items.slice(0, 12) : items;
      for (const item of show) {
        const idxLabel = itemLabels?.[item.index] ?? item.index;
        const name = nameMap?.[item.index] ? `${nameMap[item.index]}(${item.index})` : `${idxLabel}`;
        lines.push(`    [${name}]  0x${item.a.toString(16).padStart(8, '0')}  vs  0x${item.b.toString(16).padStart(8, '0')}`);
        if ((label === 'phys' || label === 'spec') && show.length <= 6) {
          lines.push(`             ${Number(item.a).toLocaleString()}  vs  ${Number(item.b).toLocaleString()}`);
        }
      }
      if (items.length > 12) lines.push(`    ... and ${items.length - 12} more`);
    }
    return lines.join('\n');
  }

  // Compare and return structured result with rich report
  // Pass customFormatter(reportStr, diffs) to override default formatting
  compareDebug(nodeIdxA, nodeIdxB, customFormatter) {
    const da = this.debugView(nodeIdxA);
    const db = this.debugView(nodeIdxB);
    if (!da || !db) return { match: false, error: 'debug SAB not available', report: '[ERROR] debug SAB not available' };
    const diffs = this.debugDiffs(nodeIdxA, nodeIdxB);
    const match = Object.keys(diffs).length === 0;
    const report = match ? '' : (customFormatter ? customFormatter(diffs) : this.formatReport(diffs));
    return { match, diffs, report };
  }
}
