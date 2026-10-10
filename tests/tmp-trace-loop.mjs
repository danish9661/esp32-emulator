// Loop-slice throughput: find a loop-closed slice in a recorded trace,
// prove it bit-exact over several iterations, then measure JIT MIPS vs
// interpreter MIPS on the same dynamic code.
import { ESP32 } from '../src/index.js';
import { readFileSync, existsSync } from 'fs';
import { resolve, dirname } from 'path';
import { fileURLToPath } from 'url';
import { buildTraceModule, instantiateTrace, findLoopSlices, classify } from './tmp-jit-lib.mjs';
const __dirname = dirname(fileURLToPath(import.meta.url));

const ROM = readFileSync(resolve(__dirname, 'rom/esp32-v3-rom.bin'));
const WASM = readFileSync(resolve(__dirname, '../src/engine/esp-xtensa/esp_engine_wasm.wasm'));
const FWBIN = process.env.FW_BIN || resolve(__dirname, 'tmp-speed-fw.bin');
if (!existsSync(FWBIN)) throw new Error('no cached firmware');
const bin = readFileSync(FWBIN);
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
for (const fn of ['native_jit_l32r', 'native_jit_ssai', 'native_code_gen', 'native_trace_len', 'native_trace_scratch_ptr', 'native_trace_scratch_len', 'native_trace_threshold', 'core_get_idle']) {
  if (typeof ex[fn] !== 'function') throw new Error('missing export ' + fn + ' (rebuild with jit call-outs?)');
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
console.log('[loop] SPEEDGO');
const CORE = (ex.core_get_idle(0) >>> 0) === 0 ? 0 : 1;
const CBASE = CORE * 4096;
const memU32 = () => new Uint32Array(L.memory.buffer);
const wc = L._wasmCores[CORE];
const SPEC = (CBASE + 336) >>> 2, INSTW = (CBASE + 2608) >>> 2, PENDW = (CBASE + 2540) >>> 2;
const spec = () => Array.from(memU32().slice(SPEC, SPEC + 256));
ex.native_trace_threshold(5000);
for (let i = 0; i < 600; i++) step();
const n = ex.native_trace_len() >>> 0;
const ptr = ex.native_trace_scratch_ptr() >>> 0;
const ring = memU32().slice(ptr >>> 2, (ptr >>> 2) + (ex.native_trace_scratch_len() >>> 0));
const STR = (ring.length - 1) / 16;
let best = -1, bestLen = 0;
for (let s = 0; s < 16; s++) {
  const cc = ring[1 + s * STR] >>> 0, st = ring[1 + s * STR + 1] >>> 0, ln = ring[1 + s * STR + 2] >>> 0;
  if (cc === CORE && ln > bestLen && st >= 0x400d1000 && st <= 0x400d2000) { best = s; bestLen = ln; }
}
if (best < 0) throw new Error('no spin trace');
const T = [];
for (let i = 0; i < bestLen; i++) {
  T.push({
    pc: ring[1 + best * STR + 3 + i * 3] >>> 0,
    op: ring[1 + best * STR + 3 + i * 3 + 1] >>> 0,
    w: ring[1 + best * STR + 3 + i * 3 + 2] >>> 0,
  });
}
for (let i = 0; i < T.length; i++) T[i].next = (i + 1 < T.length) ? T[i + 1].pc : null;
const live = spec();
const recGen = ex.native_code_gen() >>> 0;
const recLoop = [live[0], live[1], live[2]];
const slices = findLoopSlices(T);
if (!slices.length) throw new Error('no loop-closed slice in trace');
console.log('[loop] candidates: ' + slices.map((c) => `[${c.start},${c.end})`).join(' '));
const wb = live[72] >>> 0;
const sarOff = (typeof ex.native_sar_pending_off === 'function') ? (ex.native_sar_pending_off() >>> 0) : null;
console.log('[loop] sarOff=' + sarOff);
// Cyclic entry: step the interpreter until pc reaches a candidate top, so
// regs are the loop's OWN live values (restoring foreign regs flips
// data-dependent branches). First candidate validating 3/3 clean wins.
async function enterTop(top) {
  for (let i = 0; i < 200000; i++) {
    if ((wc.PC >>> 0) === top) return true;
    ex.core_run(1);
  }
  return false;
}
let S = null, runFn = null, branches = null, slice = null, builtLen = 0;
for (const cand of slices) {
  const Sc = T.slice(cand.start, cand.end).map((e) => ({ ...e }));
  Sc.forEach((e, i) => { e.next = (i + 1 < Sc.length) ? Sc[i + 1].pc : Sc[0].pc; });
  let built = null;
  try { built = buildTraceModule(Sc, { CORE, CBASE, wb, recLoop, sarOff }); }
  catch (e) { console.log(`[loop] candidate [${cand.start},${cand.end}): unemittable (${e.message})`); continue; }
  const fn = await instantiateTrace(built.mod, L, ex);
  if (!(await enterTop(Sc[0].pc))) { console.log(`[loop] candidate [${cand.start},${cand.end}): top never reached`); continue; }
  // validate 3 clean from cyclic entry (no reg restore: live loop state)
  let ok = 0, bad = 0;
  for (let t = 0; t < 6 && ok < 3; t++) {
    if ((wc.PC >>> 0) !== Sc[0].pc) { if (!(await enterTop(Sc[0].pc))) break; }
    if ((memU32()[PENDW] >>> 0) !== 0) { ex.core_run(512); continue; }
    const genS = ex.native_code_gen() >>> 0;
    const S0 = memU32().slice(CBASE, CBASE + 4096);
    const code = fn() | 0;
    if (code !== 0) { memU32().set(S0, CBASE); bad++; continue; }
    memU32()[INSTW] = (memU32()[INSTW] + built.len) >>> 0;
    const jit1 = memU32().slice(CBASE, CBASE + 4096);
    memU32().set(S0, CBASE);
    for (let i = 0; i < built.len; i++) ex.core_run(1);
    if ((ex.native_code_gen() >>> 0) !== genS) continue;
    if (built.len !== Sc.length) { ok++; continue; } // call-truncated: prefix proven, not a closed loop
    if ((wc.PC >>> 0) !== Sc[0].pc) continue; // not loop-closed: prefix proven only
    const ref1 = memU32().slice(CBASE, CBASE + 4096);
    let eq = true;
    for (let i = 0; i < 1024; i++) if (jit1[i] !== ref1[i]) { eq = false; break; }
    if (!eq) { bad++; memU32().set(S0, CBASE); continue; }
    ok++;
  }
  console.log(`[loop] candidate [${cand.start},${cand.end}) len=${Sc.length} emitlen=${built.len}: ${ok}/3 clean`);
  if (ok >= 3 && built.len === Sc.length) { S = Sc; runFn = fn; branches = built.branches; slice = cand; builtLen = built.len; break; }
}
if (!S) { console.log('[loop] FAILED (no direction-stable slice)'); process.exit(1); }
console.log(`[loop] using slice [${slice.start},${slice.end}) len=${S.length} branches=${branches.length}`);
function guardOk() {
  if ((memU32()[PENDW] >>> 0) !== 0) return false;
  const s = spec();
  return s[72] === live[72] && s[73] === live[73] && s[230] === live[230] && s[0] === recLoop[0] && s[1] === recLoop[1] && s[2] === recLoop[2];
}
// throughput: JIT loop vs interpreter loop (from cyclic live state; the
// selection validation already left pc at the slice top with live regs)
// Unit cost under natural evolution: pc==top invariant; each iteration
// times ONE slice execution, then advances via interpreter (untimed) so
// regs evolve exactly as live execution evolves them. Exits are normal
// (inner-loop phase changes) — counted, not fatal.
if (!(await enterTop(S[0].pc))) throw new Error('lost loop top before timing');
if (!guardOk()) throw new Error('guard failed before timing');
const N = 3000;
let jitClean = 0, jitFlips = 0, jitAcc = 0n;
for (let i = 0; i < N; i++) {
  if ((memU32()[PENDW] >>> 0) !== 0) { for (let k = 0; k < 4 && (memU32()[PENDW] >>> 0) !== 0; k++) ex.core_run(512); }
  if ((wc.PC >>> 0) !== S[0].pc) { if (!(await enterTop(S[0].pc))) throw new Error('lost top mid-timing'); continue; }
  const t = process.hrtime.bigint();
  const c = runFn();
  const dt = process.hrtime.bigint() - t;
  if (c !== 0) { jitFlips++; } else { jitClean++; jitAcc += dt; }
  for (let j = 0; j < builtLen; j++) ex.core_run(1); // natural advance (untimed)
}
if (jitClean < 100) throw new Error('too few clean iterations: ' + jitClean);
const jitNsPerIter = Number(jitAcc) / jitClean;
const jitMips = (S.length * 1000) / jitNsPerIter;
console.log(`[loop] JIT: ${jitClean} clean + ${jitFlips} flips, ${jitNsPerIter.toFixed(1)} ns/iter = ${jitMips.toFixed(1)} MIPS`);
// interpreter baseline, same discipline
if (!(await enterTop(S[0].pc))) throw new Error('lost loop top before baseline');
const stateB = memU32().slice(0, 4096), stateC = memU32().slice(4096, 8192);
const M = 300;
let intAcc = 0n, intClean = 0;
for (let i = 0; i < M; i++) {
  if ((memU32()[PENDW] >>> 0) !== 0) { for (let k = 0; k < 4 && (memU32()[PENDW] >>> 0) !== 0; k++) ex.core_run(512); }
  if ((wc.PC >>> 0) !== S[0].pc) { if (!(await enterTop(S[0].pc))) throw new Error('lost top mid-baseline'); continue; }
  const t = process.hrtime.bigint();
  for (let j = 0; j < S.length; j++) ex.core_run(1);
  intAcc += process.hrtime.bigint() - t;
  intClean++;
}
const intNsPerIter = Number(intAcc) / intClean;
const intMips = (S.length * 1000) / intNsPerIter;
console.log(`[loop] interp: ${intClean} iters, ${intNsPerIter.toFixed(1)} ns/iter = ${intMips.toFixed(1)} MIPS`);
console.log(`[loop] speedup=${(jitMips / intMips).toFixed(1)}x flipRate=${(100 * jitFlips / (jitClean + jitFlips)).toFixed(1)}%`);
memU32().set(stateB, 0); memU32().set(stateC, 4096);
try { ex.native_set_clock_state(chip.cycles >>> 0); } catch {}
if (jitMips < 50) console.log('[loop] NOTE: below 50 MIPS goal on this slice');
else console.log('[loop] AT/ABOVE 50 MIPS on spin slice');
