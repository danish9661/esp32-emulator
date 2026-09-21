import { readFileSync } from 'fs';
import { resolve, dirname } from 'path';
import { fileURLToPath } from 'url';
import { SimulatorWorker } from '../src/sab/worker-proxy.js';

const __dirname = dirname(fileURLToPath(import.meta.url));

// ── Load firmware ──
const flashPath = process.argv[2] || resolve(__dirname, 'fw-proxy32.bin');
const flashBytes = readFileSync(flashPath);
const romBytes = readFileSync(resolve(__dirname, '../rom/esp32-v3-rom.bin'));

const flashSizeMB = Math.max(4, Math.ceil(flashBytes.length / (1024 * 1024)));
const flash = new Uint8Array(new SharedArrayBuffer(flashSizeMB * 1024 * 1024));
flash.fill(0xff);
flash.set(flashBytes);

console.log("=== ESP32 Performance Profiler ===\n");
console.log(`Firmware: ${flashPath.split('/').pop()} (${flashBytes.length} bytes)`);
console.log(`Flash: ${flashSizeMB}MB\n`);

// ── Phase 1: Boot profile ──
const BUDGET = 5_000_000;
const POLL_INTERVAL = 50; // ms
const MAX_POLLS = 600; // 30s max

const proxy = new SimulatorWorker();
let uartOutput = '';
proxy._onUART = (b) => { uartOutput += String.fromCharCode(b); };
proxy._onError = (e) => console.error('[error]', e.message);

await proxy.init('ESP32', {
  flashSizeMB,
  mmuPages: Math.ceil(flashBytes.length / 65536),
  strapValue: 0x13,
  budget: BUDGET,
  progressInterval: 10_000_000,
}, flash, romBytes);

const t0 = performance.now();
proxy.run();

// Poll until stable (PC unchanged for 3 polls) or timeout
let prevPCs = [0, 0, 0, 0, 0];
let stablePolls = 0;
let totalPolls = 0;
let lastCycleCount = 0;
let lastTime = t0;
const cycleSamples = [];

for (let i = 0; i < MAX_POLLS; i++) {
  await new Promise(r => setTimeout(r, POLL_INTERVAL));
  proxy.pollUart();
  totalPolls++;

  const pc = proxy.pc;
  const cyc = proxy.debug[2];
  const now = performance.now();
  const dt = now - lastTime;
  const dc = cyc - lastCycleCount;

  if (dt > 0 && dc > 0) {
    cycleSamples.push({ wallMs: dt, cycles: dc, mips: dc / dt / 1000 });
  }

  prevPCs.shift();
  prevPCs.push(pc);
  const allSame = prevPCs.every(p => p === prevPCs[0]);
  if (allSame && totalPolls > 5) {
    stablePolls++;
    if (stablePolls >= 3) break;
  } else {
    stablePolls = 0;
  }

  lastCycleCount = cyc;
  lastTime = now;
}

proxy.stop();
const wallMs = performance.now() - t0;
const totalCycles = proxy.debug[2];
const halted = proxy.halted;

console.log("── Boot Profile ──");
console.log(`  Wall time:     ${wallMs.toFixed(0)}ms`);
console.log(`  Total cycles:  ${(totalCycles / 1e6).toFixed(2)}M`);
console.log(`  Final PC:      0x${proxy.pc.toString(16)}`);
console.log(`  Halted:        ${halted}`);
console.log(`  Polls:         ${totalPolls}`);
console.log(`  UART bytes:    ${uartOutput.length}`);

// ── Phase 2: Throughput measurement ──
let avgMIPS = 0;
if (cycleSamples.length > 2) {
  // Skip first 2 samples (warmup)
  const steady = cycleSamples.slice(2);
  avgMIPS = steady.reduce((s, c) => s + c.mips, 0) / steady.length;
  const maxMIPS = Math.max(...steady.map(c => c.mips));
  const minMIPS = Math.min(...steady.map(c => c.mips));

  console.log("\n── Throughput (steady-state) ──");
  console.log(`  Avg MIPS:      ${avgMIPS.toFixed(2)}`);
  console.log(`  Peak MIPS:     ${maxMIPS.toFixed(2)}`);
  console.log(`  Min MIPS:      ${minMIPS.toFixed(2)}`);
  console.log(`  Samples:       ${steady.length}`);
}

// ── Phase 3: Step granularity ──
proxy.run();
await new Promise(r => setTimeout(r, 200));
proxy.stop();
const cycBefore = proxy.debug[2];
const tStep0 = performance.now();
proxy.step(100000);
const tStep1 = performance.now();
const cycAfter = proxy.debug[2];
const stepMs = tStep1 - tStep0;
const stepCycles = cycAfter - cycBefore;

console.log("\n── Step Granularity (chip.step(100000)) ──");
console.log(`  Wall time:     ${stepMs.toFixed(1)}ms`);
console.log(`  Cycles:        ${(stepCycles / 1e3).toFixed(1)}K`);
if (stepMs > 0) {
  console.log(`  Throughput:    ${(stepCycles / stepMs / 1e3).toFixed(1)} Kcycles/ms = ${(stepCycles / stepMs / 1e6).toFixed(2)} MIPS`);
}

// ── Phase 4: JS overhead estimate ──
// writeSABState() runs every 262K cycles. With debug SAB, it copies ~500
// registers + 8 MMIO reads + memory hashes (every 16th call). Without debug
// SAB, it's just 10 typed-array writes. The optimization gated the heavy
// debug block and reduced hash frequency 4x.
console.log("\n── JS Overhead Estimate ──");
{
  const avgCyclesPerSec = avgMIPS * 1e6;
  const callsPerSec = avgCyclesPerSec / 524288;

  console.log(`  Steady-state:  ${avgMIPS.toFixed(0)} MIPS (run/stop hot loop)`);
  console.log(`  Step(100K):    ${(stepCycles / stepMs / 1e6).toFixed(1)} MIPS (single-call overhead dominates)`);
  console.log(`  writeSABState: ~${Math.round(callsPerSec)} calls/sec (every 512K cycles)`);
  console.log(`  Debug block:   ~${Math.round(callsPerSec / 4)} heavy calls/sec (every 4th = 2M cycles)`);
  console.log(`  Hash block:    ~${Math.round(callsPerSec / 16)} hash calls/sec (every 16th = 8M cycles)`);
}

// ── Phase 5: UART output analysis ──
const lines = uartOutput.split('\n').filter(l => l.trim());
console.log("\n── UART Output ──");
console.log(`  Total bytes:   ${uartOutput.length}`);
console.log(`  Total lines:   ${lines.length}`);
if (lines.length > 0) {
  console.log(`  First line:    ${lines[0].substring(0, 80)}`);
  console.log(`  Last line:     ${lines[lines.length - 1].substring(0, 80)}`);
}

// ── Summary ──
console.log("\n── Bottleneck Assessment ──");
if (totalCycles < 10_000_000) {
  console.log("  Status: LOW (boot completed quickly)");
} else if (totalCycles < 100_000_000) {
  console.log("  Status: MODERATE (firmware is doing work)");
} else {
  console.log("  Status: HIGH (heavy firmware activity)");
}

const uartRatio = uartOutput.length / Math.max(1, totalCycles);
console.log(`  UART/cycle:    ${uartRatio.toFixed(6)} bytes/cycle`);
console.log(`  Boot efficiency: ${(totalCycles / wallMs / 1000).toFixed(1)} Kcycles/ms wall-clock`);

console.log("\n=== DONE ===");
await proxy.terminate();
process.exit(0);
