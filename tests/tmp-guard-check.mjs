// Milestone B guard validation: JS window_check port vs Rust probe on live
// state (all 20 spin-loop op arg patterns), TRUE-path + CODE_GEN funnel.
import { ESP32 } from '../src/index.js';
import { readFileSync, existsSync } from 'fs';
import { resolve, dirname } from 'path';
import { fileURLToPath } from 'url';
const __dirname = dirname(fileURLToPath(import.meta.url));

// JS port of CoreState::window_check PREDICATE (state.rs:188-200).
// Returns true iff the Rust fn would take the window path (mutate + vector).
function windowCheckPredict(spec, cpu, tmp, idx) {
  const clock_event = spec[72] >>> 0, sim = spec[73] >>> 0;
  const rr = ((cpu | tmp | idx) >>> 0);
  let wa = 0;
  if (rr !== 0 && (sim & (1 << ((1 + clock_event) & 15))) !== 0) wa = 1;
  else if ((2 & rr) !== 0 && (sim & (1 << ((2 + clock_event) & 15))) !== 0) wa = 2;
  else if ((3 === cpu || 3 === tmp || 3 === idx) && (sim & (1 << ((3 + clock_event) & 15))) !== 0) wa = 3;
  if ((((spec[230] >>> 4) & 1) !== 0) || (((spec[230] >>> 18) & 1) === 0) || wa === 0) return false;
  return true;
}
// Per-op arg derivation (handler source-verified; ssai has no call -> null).
function windowArgs(op) {
  const lo = op & 15;
  const f12 = (op >> 12) & 15, f8 = (op >> 8) & 15, f4 = (op >> 4) & 15;
  switch (op) {
    case 0x661b: case 0x551b: return [f12 >> 2, f8 >> 2, 0];           // addi.n n_handler12
    case 0x778a: return [f12 >> 2, f8 >> 2, f4 >> 2];                  // add.n n_handler9
    case 0x827750: case 0x827740: case 0x828680: case 0x828580:        // mull a_handler37
      return [f12 >> 2, f8 >> 2, f4 >> 2];
    case 0x983c: return [0, f8 >> 2, 0];                              // movi.n a_handler13
    case 0xc6a092: return [0, 0, f4 >> 2];                            // movi a_handler12
    case 0xfa5381: case 0xfa8081: case 0xfa5981: case 0xfa7b81:        // l32r r_handler48
      return [0, 0, ((op >> 4) & 15) >> 2];
    case 0xd39687: case 0xd39587: return [0, f8 >> 2, f4 >> 2];        // bne n_handler46
    case 0x43987: return [0, f8 >> 2, f4 >> 2];                        // bltu n_handler42
    case 0x10d992: case 0x30d882: return [0, f8 >> 2, f4 >> 2];        // addmi n_handler13
    case 0x818880: // src _handler37 (same arg shape)
      return [f12 >> 2, f8 >> 2, f4 >> 2];
    case 0x404600: return null;                                       // ssai _handler42: no call
    default:
      if (lo === 0 && op !== 0x404600 && op !== 0x818880) return 'unknown-lo0';
      return 'unknown-op-0x' + op.toString(16);
  }
}

const ROM = readFileSync(resolve(__dirname, 'rom/esp32-v3-rom.bin'));
const WASM = readFileSync(resolve(__dirname, '../src/engine/esp-xtensa/esp_engine_wasm.wasm'));
const bin = readFileSync(resolve(__dirname, 'tmp-speed-fw.bin'));
if (!existsSync(resolve(__dirname, 'tmp-speed-fw.bin'))) throw new Error('no cached firmware');
const flash = new Uint8Array(4 * 1024 * 1024);
flash.fill(0xff);
flash.set(new Uint8Array(bin.buffer, bin.byteOffset, bin.byteLength));
const chip = new ESP32({ flashSizeMB: 4, flash });
chip.loadROM(ROM);
if (chip.gpio?.pins) {
  if (chip.gpio.pins[0]) chip.gpio.pins[0].inputValue = true;
  if (chip.gpio.pins[2]) chip.gpio.pins[2].inputValue = false;
  if (chip.gpio.pins[12]) chip.gpio.pins[12].inputValue = false;
  if (chip.gpio.pins[15]) chip.gpio.pins[15].inputValue = false;
}
chip.reset();
chip.flash.set(flash);
if (chip.gpio) chip.gpio.strapValue = 0x13;
if (chip.mmuTablePro) for (let p = 0; p < 64; p++) { chip.mmuTablePro[p] = p; chip.mmuTableApp[p] = p; }
try { chip.cores[0].writeUint32(0x3ff5a104, 0x5aa5); } catch {}
await chip.loadWasm(WASM, 'wasm');
const ex = chip._wasmLoader.exports;
const L = chip._wasmLoader;
for (const fn of ['native_window_check_probe', 'native_code_gen', 'native_trace_len', 'native_trace_scratch_ptr', 'native_trace_scratch_len', 'native_trace_threshold']) {
  if (typeof ex[fn] !== 'function') throw new Error('missing export ' + fn);
}
let fireDueFn = null;
let uart = '';
chip.uart[0].onTX = (b) => { uart += String.fromCharCode(b); };
function step() {
  ex.core_run(512);
  chip.cycles += 512;
  try {
    if (!chip._sabU32) chip._sabU32 = new Uint32Array(chip._wasmMemory || chip._wasmMemoryObj?.buffer);
    chip._sabU32[998] = chip.clocks.cpu.ticks >>> 0;
    chip._sabU32[999] = chip.cycles >>> 0;
  } catch {}
  try {
    if (!fireDueFn) { const r = chip.clocks?.root; fireDueFn = r?.fireDueEvents?.bind(r) || null; }
    if (fireDueFn) fireDueFn();
  } catch {}
  let idle = false;
  try { idle = !!chip.coresIdle; } catch {}
  if (idle && ex?.native_idle_advance) {
    try { chip.cycles += ex.native_idle_advance(chip.cycles >>> 0); } catch {}
  } else {
    try { ex?.native_set_clock_state?.(chip.cycles >>> 0); } catch {}
    try { ex?.native_pump_events?.(); } catch {}
  }
}
const tB0 = Date.now();
while (!uart.includes('SPEEDGO')) { step(); if (Date.now() - tB0 > 600000) throw new Error('boot timeout'); }
console.log('[guard] SPEEDGO');
const idle1 = ex.core_get_idle(1) >>> 0;
const CORE = (ex.core_get_idle(0) >>> 0) === 0 ? 0 : 1;
const CBASE = CORE * 4096;
const memU32 = () => new Uint32Array(L.memory.buffer);
const spec = () => Array.from(memU32().slice((CBASE + 336) >>> 2, ((CBASE + 336) >>> 2) + 256));
// fill the ring fast, then grab a spin trace
ex.native_trace_threshold(5000);
for (let i = 0; i < 600; i++) step();
const n = ex.native_trace_len() >>> 0;
console.log('[guard] traces=' + n + ' CORE=' + CORE);
const ptr = ex.native_trace_scratch_ptr() >>> 0;
const ring = memU32().slice(ptr >>> 2, (ptr >>> 2) + (1 + 16 * 195));
// pick the longest trace rooted in the spin area
let best = -1, bestLen = 0;
for (let s = 0; s < 16; s++) {
  const cc = ring[1 + s * 195] >>> 0, st = ring[1 + s * 195 + 1] >>> 0, ln = ring[1 + s * 195 + 2] >>> 0;
  if (cc === CORE && ln > bestLen && st >= 0x400d1000 && st <= 0x400d2000) { best = s; bestLen = ln; }
}
if (best < 0) throw new Error('no spin trace');
console.log('[guard] using slot' + best + ' len=' + bestLen);
const live = spec();
console.log('[guard] spec72=' + live[72] + ' spec73=0x' + live[73].toString(16) + ' spec230=0x' + live[230].toString(16) + ' veccbase=0x' + live[231].toString(16));
// false-path: every entry must predict false AND probe 0, spec untouched
let checked = 0, skipped = 0;
const specBefore = live.join(',');
for (let i = 0; i < bestLen; i++) {
  const op = ring[1 + best * 195 + 3 + i * 3 + 1] >>> 0;
  const args = windowArgs(op);
  if (args === null) { skipped++; continue; }
  if (typeof args === 'string') throw new Error('no derivation for ' + args);
  const pred = windowCheckPredict(live, args[0], args[1], args[2]);
  const probe = ex.native_window_check_probe(CORE, args[0], args[1], args[2]) >>> 0;
  if (pred !== false || probe !== 0) throw new Error(`FIRE at entry ${i} op=0x${op.toString(16)} pred=${pred} probe=${probe}`);
  checked++;
}
const specAfter = spec().join(',');
if (specBefore !== specAfter) throw new Error('probe mutated spec on false path!');
console.log(`[guard] false-path: ${checked} entries agree (pred=false, probe=0), ${skipped} ssai skipped, spec untouched`);
// true-path: force write_addr with live CACHE_CONTROL bits
const wb = live[72] >>> 0, cc = live[73] >>> 0;
let forceArgs = null;
for (const [c, t, d] of [[1, 0, 0], [2, 0, 0], [3, 3, 3]]) {
  const rr = (c | t | d) >>> 0;
  let wa = 0;
  if (rr !== 0 && (cc & (1 << ((1 + wb) & 15))) !== 0) wa = 1;
  else if ((2 & rr) !== 0 && (cc & (1 << ((2 + wb) & 15))) !== 0) wa = 2;
  else if (3 === c && (cc & (1 << ((3 + wb) & 15))) !== 0) wa = 3;
  if (wa !== 0) { forceArgs = [c, t, d]; break; }
}
const woe = (live[230] >>> 18) & 1, excm = (live[230] >>> 4) & 1;
console.log('[guard] woe=' + woe + ' excm=' + excm + ' forceArgs=' + JSON.stringify(forceArgs));
if (woe === 1 && excm === 0 && forceArgs) {
  const stateFull = memU32().slice(CBASE, CBASE + 4096);
  const pred = windowCheckPredict(live, ...forceArgs);
  const probe = ex.native_window_check_probe(CORE, ...forceArgs) >>> 0;
  memU32().set(stateFull, CBASE); // probe fires -> mutated: restore
  console.log('[guard] true-path: pred=' + pred + ' probe=' + probe);
  if (pred !== true || probe !== 1) throw new Error('true-path disagreement');
} else console.log('[guard] true-path(natural): SKIPPED (live PS/CACHE bits admit no firing case)');
// true-path (synthetic): set the exact CACHE_CONTROL bit the live window
// base consults, so write_addr=1 becomes reachable; probe MUST fire.
{
  const stateFull = memU32().slice(CBASE, CBASE + 4096);
  const w73 = ((CBASE + 336) >>> 2) + 73;
  const synth = live.slice();
  synth[73] = (synth[73] | (1 << ((1 + wb) & 15))) >>> 0;
  memU32()[w73] = synth[73];
  const pred = windowCheckPredict(synth, 1, 0, 0);
  const probe = ex.native_window_check_probe(CORE, 1, 0, 0) >>> 0;
  const mutated = spec()[72] !== live[72]; // probe sets MEM_FAULT_INFO=(wb+wa)&15
  memU32().set(stateFull, CBASE);
  console.log('[guard] true-path(synthetic): pred=' + pred + ' probe=' + probe + ' mutated=' + mutated);
  if (pred !== true || probe !== 1 || !mutated) throw new Error('synthetic true-path disagreement');
}
// CODE_GEN funnel: must advance across spinning + jump on a forced DRAM store
const g0 = ex.native_code_gen() >>> 0;
for (let i = 0; i < 50; i++) step();
const g1 = ex.native_code_gen() >>> 0;
ex.core_write_uint8(CORE, 0x3ffe7ff0, 0xab);
ex.core_write_uint8(CORE, 0x3ffe7ff1, 0xcd);
const g2 = ex.native_code_gen() >>> 0;
console.log(`[guard] codegen: boot=${g0} +50steps=${g1} +2stores=${g2}`);
if (!(g1 >= g0 && g2 >= g1 + 2)) throw new Error('CODE_GEN funnel dead');
console.log('[guard] PASSED');
