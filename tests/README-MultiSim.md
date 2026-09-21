# MultiSimulator — ESP32 Parallel Nodes

> **STALE-NOTE (2026-08-18):** the JS engine and `'both'`/lockstep mode were
> removed — MultiSimulator now runs N parallel WASM nodes only. The dual-engine
> and lockstep sections below are historical; the SAB/worker flow and
> `addNodeParallel` plumbing remain accurate.

## What it does

Two ESP32 emulator instances (JS-only vs WASM-only) run in parallel workers. Two execution modes:

| Mode | Method | Use case |
|------|--------|----------|
| **Lockstep** | `stepParallel(N)` | Step both by N, compare after each batch. Good for cycle-exact comparison on small code. |
| **Free-run** | `runParallel({onPoll})` | Both run continuously. Poll debug SABs every ~100ms to stream UART or detect when done. Good for running real firmware to completion. |

## Architecture

```
test script                         MultiSimulator                Worker (worker-entry.js)
────────────                        ──────────────                ─────────────────────
                                     alloc ctrlSab,
  await sim.addNodeParallel(           uartSab, errSab,
    'ESP32', {engine:'js',...}         debugSab, readRespSab
  ) ─────────────────────────►      spawn Worker ──────────►   init handler:
                                     postMessage({init})         createDefaultFlash()
                                                                 loadBinary('bootrom')
                                                                 new ESP32(config)
                                                                 applyBasicSetup()
                                                                 → blockingCommandLoop()

  await sim.addNodeParallel(         spawn Worker ──────────►   same flow for WASM node
    'ESP32', {engine:'wasm',...}       + loadBinary('wasmBinary')
  ) ─────────────────────────►        + chip.loadWasm()
                                       + chip.reset()
                                       + re-apply setup

  ─── Lockstep mode ───
  for (;;) {
    sim.stepParallel(500) ──────►   Atomics.store(CMD_STEP)   Atomics.wait(CMD)
                                     to ALL workers             step() × 500
                                     Atomics.wait(RESP) ←──    writeSABState()
                                     for ALL workers            Atomics.store(RESP)

    sim.compareDebug(0,1) ──────►   reads both debug SABs
                                     returns {match, diffs, report}
  }

  ─── Free-run mode ───
  await sim.runParallel({
    onPoll: (s) => {                  polling loop
      sim.compareDebug(0,1)           reads debug SABs
      s.getUART(0)                    reads UART ring buffer
      return true → stop all           break, signal stop to all workers
    },
    pollInterval: 100
  });
```

## Creating a test

### 1. Boilerplate

```javascript
import { MultiSimulator } from '../src/sab/MultiSimulator.js';

const sim = new MultiSimulator();
```

### 2. Add nodes (one JS, one WASM)

```javascript
await sim.addNodeParallel('ESP32', {
  engine: 'js',                        // JS engine only
  strapValue: 0x13,
  pinInputs: { 0: true, 2: false, 12: false, 15: false },
  mmuPages: 64
});

await sim.addNodeParallel('ESP32', {
  engine: 'wasm',                      // WASM engine only
  strapValue: 0x13,
  pinInputs: { 0: true, 2: false, 12: false, 15: false },
  mmuPages: 64
});
```

**worker-entry.js auto-loads** ROM and WASM from hardcoded default paths. No file imports needed in your test.

### 3. Step in lockstep

```javascript
sim.stepParallel(500);  // both ESPs step exactly 500 times
```

### 3b. Free-run (run to completion)

```javascript
await sim.runParallel({
  onPoll: (s) => {
    // Called every ~100ms. s = MultiSimulator instance
    const uart = s.getUART(0);
    if (uart.includes('ALL TESTS PASSED')) return true; // stop all
    const cmp = s.compareDebug(0, 1);
    if (cmp && !cmp.match) console.log(cmp.report);
    return false;
  },
  pollInterval: 100,
});
```

`runParallel` sends `CMD_RUN` to all workers (continuous execution). The polling loop reads debug SABs and UART buffers. When `onPoll` returns `true`, all workers are signalled to stop via `SAB_RUN = 0`.

### 4. Get data

| Method | Returns | Use |
|--------|---------|-----|
| `debugView(idx)` | `Uint32Array(480)` | Raw SAB data |
| `debugState(idx)` | `{ pc0, pc1, cycles, nanos, ticks, enabled0, enabled1, physicalRegisters[], specialRegisters[], mmuPro[], mmuApp[] }` | Structured single-node snapshot |
| `debugDiffs(0,1)` | `{ pc0?, pc1?, cycles?, ticks?, phys[], spec[], mmuPro[], mmuApp[] }` | Raw differences between nodes |
| `compareDebug(0,1)` | `{ match, diffs, report }` | Differences + rich formatted report |

### 5. Compare

```javascript
// Full comparison with rich report:
const cmp = sim.compareDebug(0, 1);
if (!cmp.match) {
  console.log('Divergence found!');
  console.log(cmp.report);
}

// Custom formatter:
const cmp = sim.compareDebug(0, 1, (diffs) => {
  let out = '';
  if (diffs.pc0) out += `PC0: ${diffs.pc0.a.toString(16)} vs ${diffs.pc0.b.toString(16)}\n`;
  if (diffs.spec) {
    for (const d of diffs.spec) {
      const name = MultiSimulator._specRegNames[d.index] || `sr${d.index}`;
      out += `${name}: 0x${d.a.toString(16)} vs 0x${d.b.toString(16)}\n`;
    }
  }
  return out;
});

// Raw diffs only (inspect programmatically):
const diffs = sim.debugDiffs(0, 1);
if (diffs.spec) {
  for (const d of diffs.spec) {
    if (d.index === 234) console.log('CCOUNT differs!');   // CCOUNT = special reg 234
  }
}
```

### 6. Single-node inspection

```javascript
const state = sim.debugState(0);
console.log(`JS PC = 0x${state.pc0.toString(16)}`);
console.log(`CCOUNT = ${state.specialRegisters[234]}`);
console.log(`MMU[0] = ${state.mmuPro[0]}`);

const wasmState = sim.debugState(1);
console.log(`WASM PC = 0x${wasmState.pc0.toString(16)}`);

// Memory hashes (FNV-1a of first 64KB of each region)
console.log(`IRAM hash: 0x${state.memHashes.iram.toString(16)}`);
console.log(`DRAM hash: 0x${state.memHashes.dram.toString(16)}`);

// Peripheral register values
console.log(`GPIO_OUT0 = 0x${state.periph[0].toString(16)}`);
console.log(`UART0_STATUS = 0x${state.periph[5].toString(16)}`);
```

## Debug SAB Layout

| Offset | u32s | Content |
|--------|------|---------|
| 0 | 1 | PC core 0 |
| 1 | 1 | PC core 1 |
| 2 | 1 | cycles |
| 3-4 | 2 | nanos (lo/hi) |
| 5 | 1 | ticks |
| 6-7 | 2 | core0/1 enabled flags |
| 8-15 | 8 | (unused) |
| 16-79 | 64 | physical registers a0..a15 (core 0) |
| 80-95 | 16 | key special registers (subset: PS, EPC1-7, CCOUNT, CCOMPARE, EXCSAVE1-3, etc.) |
| 96-351 | 256 | ALL special registers (0..255) |
| 352-415 | 64 | MMU PRO table |
| 416-479 | 64 | MMU APP table |
| 480-483 | 4 | Memory region hashes (IRAM, DRAM, RTC_FAST, SRAM — FNV-1a of first 64KB) |
| 484-499 | 16 | Peripheral register snapshots (GPIO_OUT0/1, GPIO_ENABLE, GPIO_IN, GPIO_STATUS, UART0_STATUS, UART1_STATUS, TIMG0_T0LO/HI, reserved) |

**Total: 500 u32s = 2000 bytes**

## Register Names (for report and inspection)

From `MultiSimulator._specRegNames`:

| Index | Name | Description |
|-------|------|-------------|
| 0 | LBEG | Loop begin |
| 1 | LEND | Loop end |
| 2 | LCOUNT | Loop count |
| 3 | SAR | Shift amount |
| 72 | PS | Processor state |
| 90 | EXCCAUSE | Exception cause |
| 91 | DEBUGCAUSE | Debug cause |
| 224 | VECBASE | Vector base |
| 226-232 | EPC1..EPC7 | Exception PCs |
| 233 | DEPC | Debug exception PC |
| 234 | CCOUNT | Cycle count |
| 235 | CCOMPARE | Cycle compare |
| 240-246 | EXCSAVE1..EXCSAVE7 | Exception save regs |
| 247-253 | EPS1..EPS7 | Exception PS |
| 254 | CPENABLE | Coprocessor enable |

## UART Output

Workers write UART bytes into a shared ring buffer (SAB). Read with:

```javascript
const uart0 = sim.getUART(0);  // engine 0 (JS)
const uart1 = sim.getUART(1);  // engine 1 (WASM)
```

Returns a string with `\n` line separators.

## Full Working Test: `big-test-10m.mjs` (lockstep)

```javascript
import { MultiSimulator } from '../src/sab/MultiSimulator.js';

const sim = new MultiSimulator();

await sim.addNodeParallel('ESP32', { engine: 'js', strapValue: 0x13, pinInputs: { 0: true, 2: false, 12: false, 15: false }, mmuPages: 64 });
await sim.addNodeParallel('ESP32', { engine: 'wasm', strapValue: 0x13, pinInputs: { 0: true, 2: false, 12: false, 15: false }, mmuPages: 64 });

const TOTAL = 10_000_000;
const BATCH = 500;
console.log(`Running ${TOTAL.toLocaleString()} steps (batch=${BATCH})...`);

for (let s = 0; s < TOTAL; s += BATCH) {
  const batch = Math.min(BATCH, TOTAL - s);
  sim.stepParallel(batch);

  const cmp = sim.compareDebug(0, 1);
  if (cmp && !cmp.match) {
    console.log(`\n*** DIVERGENCE at step ${s + batch} ***`);
    console.log(cmp.report);
    process.exit(1);
  }

  if ((s + batch) % 100000 === 0) console.log(`  step ${(s + batch).toLocaleString()}: OK`);
}

console.log(`\n*** PASSED ${TOTAL.toLocaleString()} steps ***`);
```

## Full Working Test: `test-real-firmware.mjs` (free-run, real compiled firmware)

```javascript
import { MultiSimulator } from '../src/sab/MultiSimulator.js';
import axios from 'axios';
import { readFileSync } from 'fs';

const SIM_FREQ = 40e6; // 40 MHz simulation clock

// 1. Compile .ino firmware via compile server
const code = readFileSync('path/to/firmware.ino', 'utf-8');
const startRes = await axios.post('http://localhost:5000/api/compile/start', { code, target: 'esp32' });
const buildId = startRes.data.buildId;
let b64;
while (true) {
  const stat = await axios.get(`http://localhost:5000/api/compile/status/${buildId}`);
  if (stat.data.status === 'success') { b64 = stat.data.binary_content; break; }
  await new Promise(r => setTimeout(r, 1000));
}
const firmwareBytes = Buffer.from(b64, 'base64');

// 2. Create SharedArrayBuffer for flash and ROM
const flashSab = new SharedArrayBuffer(4 * 1024 * 1024);
new Uint8Array(flashSab).set(firmwareBytes);
const romSab = new SharedArrayBuffer(512 * 1024);
new Uint8Array(romSab).set(readFileSync('rom/bootrom.bin'));

// 3. Setup dual-engine simulation
const sim = new MultiSimulator();
await sim.addNodeParallel('ESP32', { engine: 'js', strapValue: 0x13 }, flashSab, romSab);
await sim.addNodeParallel('ESP32', { engine: 'wasm', strapValue: 0x13 }, flashSab, romSab);

// 4. Run to completion
let output = '';
await sim.runParallel({
  onPoll: (s) => {
    const uart = s.getUART(0);
    if (uart.length > output.length) process.stdout.write(uart.slice(output.length));
    output = uart;
    if (uart.includes('ALL TESTS PASSED') && s.getUART(1).includes('ALL TESTS PASSED')) return true;
    return false;
  },
  pollInterval: 100,
});

// 5. Verify
const uart0 = sim.getUART(0);
const uart1 = sim.getUART(1);
console.log(`JS engine: ${uart0.includes('ALL TESTS PASSED') ? 'PASS' : 'FAIL'}`);
console.log(`WASM engine: ${uart1.includes('ALL TESTS PASSED') ? 'PASS' : 'FAIL'}`);
```

## Creating custom test scripts

You can use the same infrastructure to write different kinds of tests:

```javascript
// Example: check every 1000 steps, stop after first 100K
const sim = new MultiSimulator();
await sim.addNodeParallel('ESP32', { engine: 'js', mmuPages: 64 });
await sim.addNodeParallel('ESP32', { engine: 'wasm', mmuPages: 64 });

for (let s = 0; s < 100_000; s += 1000) {
  sim.stepParallel(1000);
  const { match, diffs } = sim.compareDebug(0, 1);
  if (!match) {
    // Binary-search to find exact step
    for (let sub = 0; sub < 1000; sub += 1) {
      sim.stepParallel(1);
      const { match: m } = sim.compareDebug(0, 1);
      if (!m) { console.log(`Exact divergence at step ${s + sub + 1}`); break; }
    }
    break;
  }
}
```
