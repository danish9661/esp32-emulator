import { readFileSync } from 'fs';
import { resolve, dirname } from 'path';
import { fileURLToPath } from 'url';

const __dirname = dirname(fileURLToPath(import.meta.url));

// Load WASM engine directly (no worker thread) for precise measurement
const { ESP32 } = await import('../src/peripherals/esp32/esp32.js');

const flashPath = process.argv[2] || resolve(__dirname, 'fw-proxy32.bin');
const flashBytes = readFileSync(flashPath);
const romBytes = readFileSync(resolve(__dirname, '../rom/esp32-v3-rom.bin'));

const flashSizeMB = Math.max(4, Math.ceil(flashBytes.length / (1024 * 1024)));

function createChip() {
  const chip = new ESP32({ flashSizeMB });
  chip.flash.fill(0xff);
  chip.flash.set(flashBytes);
  chip.loadROM(romBytes);
  const wasmPath = resolve(__dirname, '../src/engine/esp-xtensa/esp_engine_wasm.wasm');
  const wasmBytes = readFileSync(wasmPath);
  chip.loadWasm(wasmBytes);
  chip.reset();
  return chip;
}

function benchStep(label, chip, steps, opts = {}) {
  const { traceReturns = false, traceMemWrites = false } = opts;

  // Set trace flags
  const exp = chip._wasmLoader?.exports;
  if (exp?.native_trace_set_flags) {
    exp.native_trace_set_flags(traceReturns ? 1 : 0, traceMemWrites ? 1 : 0);
  }

  // Warm up
  for (let i = 0; i < 10; i++) chip.step();

  // Benchmark
  const t0 = performance.now();
  for (let i = 0; i < steps; i++) {
    chip.step();
  }
  const elapsed = performance.now() - t0;
  const mips = (steps * 1024) / (elapsed * 1000); // 1024 instructions per step
  console.log(`  ${label.padEnd(35)} ${elapsed.toFixed(1)}ms  ${mips.toFixed(1)} MIPS  (${steps} steps)`);
  return { elapsed, mips };
}

console.log('=== Debug Trace Overhead Profiler ===\n');

// Phase 1: Boot the firmware to get past ROM
console.log('Booting firmware...');
const chip = createChip();
const bootSteps = 200000;
for (let i = 0; i < bootSteps; i++) chip.step();
console.log(`Booted: PC=0x${chip.cores[0].PC.toString(16)} cycles=${chip.cycles}\n`);

// Phase 2: Benchmark with tracing OFF (baseline)
const STEPS = 50000;
console.log(`--- Baseline (tracing OFF) ---`);
const baseline = benchStep('tracing OFF', chip, STEPS);

// Phase 3: Benchmark with trace_return ON
console.log(`\n--- Trace RETURN only ---`);
const traceReturn = benchStep('trace_return=ON', chip, STEPS, { traceReturns: true });

// Phase 4: Benchmark with trace_mem_write ON
console.log(`\n--- Trace MEM_WRITE only ---`);
const traceMem = benchStep('trace_mem_write=ON', chip, STEPS, { traceMemWrites: true });

// Phase 5: Benchmark with both ON
console.log(`\n--- Both traces ON ---`);
const traceBoth = benchStep('both=ON', chip, STEPS, { traceReturns: true, traceMemWrites: true });

// Turn traces off
const expCleanup = chip._wasmLoader?.exports;
if (expCleanup?.native_trace_set_flags) {
  expCleanup.native_trace_set_flags(0, 0);
}

// Summary
console.log('\n=== Summary ===');
console.log(`  Baseline:          ${baseline.mips.toFixed(1)} MIPS`);
console.log(`  +trace_return:     ${traceReturn.mips.toFixed(1)} MIPS  (${((baseline.elapsed / traceReturn.elapsed - 1) * 100).toFixed(1)}% overhead)`);
console.log(`  +trace_mem_write:  ${traceMem.mips.toFixed(1)} MIPS  (${((baseline.elapsed / traceMem.elapsed - 1) * 100).toFixed(1)}% overhead)`);
console.log(`  +both:             ${traceBoth.mips.toFixed(1)} MIPS  (${((baseline.elapsed / traceBoth.elapsed - 1) * 100).toFixed(1)}% overhead)`);
console.log(`\nNote: Tracing OFF = 0 FFI calls from trace paths.`);
console.log(`Tracing ON = FFI round-trip per store/return instruction.`);
