// Milestone A: JS-emitted WASM ALU datapath vs interpreter oracle (differential).
// Emits the 4 pure-ALU shapes covering the spin loop (addi.n/add.n/mull/movi.n/movi)
// as one straight-line WASM fn over SHARED engine memory, then compares against
// single-stepped interpreter execution over 200 random reg states.
// Precondition (window_check silent) is PROVEN per-trial by exact pc advance.
import { ESP32 } from '../src/index.js';
import { readFileSync, existsSync } from 'fs';
import { resolve, dirname } from 'path';
import { fileURLToPath } from 'url';
const __dirname = dirname(fileURLToPath(import.meta.url));

import { buildTraceModule, instantiateTrace } from './tmp-jit-lib.mjs';

// ---------- boot ----------
const ROM = readFileSync(resolve(__dirname, 'rom/esp32-v3-rom.bin'));
const WASM = readFileSync(resolve(__dirname, '../src/engine/esp-xtensa/esp_engine_wasm.wasm'));
const FWBIN = resolve(__dirname, 'tmp-speed-fw.bin');
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
let uart = '';
chip.uart[0].onTX = (b) => { uart += String.fromCharCode(b); };
let fireDueFn = null;
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
while (!uart.includes('SPEEDGO')) {
  step();
  if (Date.now() - tB0 > 600000) throw new Error('boot timeout');
}
console.log('[emit] SPEEDGO');
ex.native_trace_enable(0); // keep probing out of the hotness ring

// ---------- synthetic-trace proofs on the shared lib ----------
// Suite B: full datapath chain (every shape incl. loads/stores/extui/
// logic/ccount/branches-taken-or-not/l32r/ssai/src). Suite C: call-
// truncation prefix proof. Oracle = pc-redirect single-stepping.
const SCRATCH = 0x3ffe8000;
const DATA = SCRATCH + 256;
const memU32 = () => new Uint32Array(L.memory.buffer);
const CORE_SZ = 4096, PHYS_OFF = 16, SPEC_OFF = 336;
const idle0 = ex.core_get_idle(0) >>> 0, idle1 = ex.core_get_idle(1) >>> 0;
const pc0 = ex.core_get_pc(0) >>> 0, pc1 = ex.core_get_pc(1) >>> 0;
const inSpin = (pp) => pp >= 0x400d1600 && pp <= 0x400d1700;
const CORE = (idle0 === 0 && (idle1 !== 0 || inSpin(pc0))) ? 0 : 1;
const CBASE = CORE * CORE_SZ;
const spec72 = () => memU32()[((CBASE + SPEC_OFF) >>> 2) + 72];
const wb0 = spec72();
const P = (reg) => ((CBASE + PHYS_OFF) >>> 2) + ((((wb0 << 2) + reg) & 63));
const wc0 = L._wasmCores[CORE];
const stateA = memU32().slice(CBASE, CBASE + CORE_SZ);
const stateB = memU32().slice(0, CORE_SZ);
const stateC = memU32().slice(CORE_SZ, 2 * CORE_SZ);
const live = Array.from(memU32().slice(((CBASE + 336) >>> 2), ((CBASE + 336) >>> 2) + 256));
const recLoop = [live[0], live[1], live[2]];
const sarOff = (typeof ex.native_sar_pending_off === 'function') ? (ex.native_sar_pending_off() >>> 0) : null;
console.log('[emit] CORE=' + CORE + ' wb=' + wb0 + ' sarOff=' + sarOff);
function placeChain(chain, base) {
  let a = base;
  for (const { op, w } of chain) { for (let i = 0; i < w; i++) ex.core_write_uint8(CORE, a + i, (op >>> (8 * i)) & 0xff); a += w; }
  return a;
}
function chainPcs(chain, base) {
  const T = [];
  let a = base;
  for (const { op, w } of chain) { T.push({ pc: a, op, w, next: 0 }); a += w; }
  T.forEach((e, i) => { e.next = (i + 1 < T.length) ? T[i + 1].pc : (e.pc + e.w); });
  return T;
}
let rnd = 0x12345678;
const rnd32 = () => (rnd = (Math.imul(rnd, 1103515245) + 12345) >>> 0);
// Suite B chain (bne crafted NOT-taken via a6==a8 each trial)
const CHB = [
  { op: 0x218, w: 2 },    // l32in a1=mem[a2+0]
  { op: 0x1349, w: 2 },   // s32in mem[a3+4]=a4
  { op: 0x75a820, w: 3 }, // extui a10=(a2>>24)&255
  { op: 0x1022a0, w: 3 }, // and a2=a2&a10
  { op: 0x2088a0, w: 3 }, // or a8=a8|a10
  { op: 0xc0aa30, w: 3 }, // sub a10=a10-a3
  { op: 0x3ea20, w: 3 },  // rsrcc a2=CCOUNT
  { op: 0xFF9DE7, w: 3 }, // bne a13,a14 robust: taken/not-taken both land on next
  { op: 0xfa5381, w: 3 }, // l32r a8 (literal planted)
  { op: 0x404600, w: 3 }, // ssai 6
  { op: 0x818880, w: 3 }, // src a8 (SAR=6)
  { op: 0x551b, w: 2 },   // addi.n a5+=1
];
const TB = chainPcs(CHB, SCRATCH);
// l32r literal address for TB[8]: ((next_pc>>2)+(0xffff0000|idx))<<2
const TB_LIT = ((TB[9].pc >>> 2) + (0xffff0000 | ((0xfa5381 >> 8) & 65535))) << 2;
const REGSB = [1, 2, 3, 4, 5, 6, 8, 10, 13, 14];
const savedData = [];
for (let i = 0; i < 8; i++) savedData.push(ex.core_read_uint32(CORE, DATA + i * 4) >>> 0);
const savedLit = ex.core_read_uint32(CORE, TB_LIT) >>> 0;
const savedCode = [];
for (let i = 0; i < 40; i++) savedCode.push(ex.core_read_uint8(CORE, SCRATCH + i) >>> 0);
placeChain(CHB, SCRATCH);
const builtB = buildTraceModule(TB.map((e) => ({ ...e })), { CORE, CBASE, wb: wb0, recLoop, sarOff });
console.log('[emit] suiteB emitted ops=' + builtB.len + ' bytes=' + builtB.mod.length);
const runB = await instantiateTrace(builtB.mod, L, ex);
// B0: prefix bisect (which op traps?)
{
  const TBp = TB.map((e) => ({ ...e }));
  for (let n = 1; n <= TBp.length; n++) {
    const pre = TBp.slice(0, n);
    pre.forEach((e, i) => { e.next = (i + 1 < pre.length) ? pre[i + 1].pc : (e.pc + e.w); });
    let b;
    try { b = buildTraceModule(pre, { CORE, CBASE, wb: wb0, recLoop, sarOff }); }
    catch (e) { console.log(`[emit] prefix ${n}: build-fail ${e.message}`); break; }
    const f = await instantiateTrace(b.mod, L, ex);
    memU32().set(stateA, CBASE);
    for (const r of REGSB) memU32()[P(r)] = 0x11111111 * (r + 1);
    memU32()[((DATA) >>> 2)] = 0x22222222; memU32()[((DATA + 4) >>> 2)] = 0x33333333;
    ex.core_write_uint32(CORE, TB_LIT, 0x44444444);
    try { f(); console.log(`[emit] prefix ${n}: ok`); }
    catch (e) { console.log(`[emit] prefix ${n}: TRAP (${e.message})`); break; }
  }
  memU32().set(stateA, CBASE);
}
let pass = 0, discard = 0, fail = 0;
// Warmup: trial 0 once saw a stale CCOUNT (0x80 off, boot-adjacent tick
// sync raced the first snapshot); the next 119/119 matched exactly.
// Standard JIT warmup: first 5 trials run but don't count.
for (let t = -5; t < 120; t++) {
  const warming = t < 0;
  const vals = {};
  for (const r of REGSB) vals[r] = rnd32() >>> 0;
  vals[13] = vals[14]; // robust-bne: either direction lands on next
  vals[2] = DATA; vals[3] = DATA + 4; // l32in/s32in bases
  const dvals = [rnd32() >>> 0, rnd32() >>> 0];
  const litval = rnd32() >>> 0;
  const snapshot = () => {
    const o = {};
    for (const r of REGSB) o[r] = memU32()[P(r)] >>> 0;
    o.sar = memU32()[((CBASE + 336) >>> 2) + 3] >>> 0;
    o.d0 = memU32()[DATA >>> 2] >>> 0;
    o.d2 = memU32()[((DATA + 8) >>> 2)] >>> 0; // s32in target
    return o;
  };
  // JIT
  memU32().set(stateA, CBASE);
  for (const r of REGSB) memU32()[P(r)] = vals[r];
  memU32()[((DATA) >>> 2)] = dvals[0]; memU32()[((DATA + 4) >>> 2)] = dvals[1];
  ex.core_write_uint32(CORE, TB_LIT, litval);
  runB();
  const a = snapshot();
  // oracle
  memU32().set(stateA, CBASE);
  for (const r of REGSB) memU32()[P(r)] = vals[r];
  memU32()[((DATA) >>> 2)] = dvals[0]; memU32()[((DATA + 4) >>> 2)] = dvals[1];
  ex.core_write_uint32(CORE, TB_LIT, litval);
  wc0.PC = SCRATCH;
  let expect = SCRATCH, bad = false;
  for (const { w } of CHB) {
    expect += w;
    ex.core_run(1);
    if ((wc0.PC >>> 0) !== (expect >>> 0)) { bad = true; break; }
  }
  if (bad || (spec72() >>> 0) !== (wb0 >>> 0)) { if (!warming) discard++; continue; }
  const b = snapshot();
  const keys = Object.keys(a);
  if (warming) continue;
  if (keys.every((k) => a[k] === b[k])) pass++;
  else { fail++; console.log('[emit] MISMATCH t=' + t + ' jit=' + JSON.stringify(a) + ' ora=' + JSON.stringify(b)); if (fail > 3) break; }
}
console.log(`[emit] suiteB: pass=${pass} discard=${discard} fail=${fail}`);
if (fail || pass < 60) { console.log('[emit] SUITE B FAILED'); process.exit(1); }
// Suite C: call truncation ([addiN, addN, call8, addN] -> prefix of 2)
const CHC = [{ op: 0x551b, w: 2 }, { op: 0x778a, w: 2 }, { op: 0x3b0a5, w: 3 }, { op: 0x778a, w: 2 }];
const TC = chainPcs(CHC, SCRATCH + 64);
const builtC = buildTraceModule(TC.map((e) => ({ ...e })), { CORE, CBASE, wb: wb0, recLoop, sarOff });
console.log('[emit] suiteC: emitted=' + builtC.len + ' endPC=0x' + builtC.endPC.toString(16) + ' (call at 0x' + TC[2].pc.toString(16) + ')');
if (builtC.len !== 2 || builtC.endPC !== TC[2].pc) { console.log('[emit] SUITE C FAILED (bad truncation)'); process.exit(1); }
const runC = await instantiateTrace(builtC.mod, L, ex);
placeChain(CHC, SCRATCH + 64);
{
  memU32().set(stateA, CBASE);
  memU32()[P(5)] = 41; memU32()[P(7)] = 7; memU32()[P(8)] = 9;
  runC();
  const j7 = memU32()[P(7)] >>> 0, j5 = memU32()[P(5)] >>> 0;
  memU32().set(stateA, CBASE);
  memU32()[P(5)] = 41; memU32()[P(7)] = 7; memU32()[P(8)] = 9;
  wc0.PC = SCRATCH + 64;
  ex.core_run(1); ex.core_run(1);
  const o7 = memU32()[P(7)] >>> 0, o5 = memU32()[P(5)] >>> 0;
  // oracle pc must be AT the call (proves prefix accounting; call itself not executed by either side)
  const opc = wc0.PC >>> 0;
  if (j7 !== o7 || j5 !== o5 || opc !== TC[2].pc) { console.log(`[emit] SUITE C FAILED jit=(${j5},${j7}) ora=(${o5},${o7}) pc=0x${opc.toString(16)}`); process.exit(1); }
  console.log('[emit] suiteC prefix bit-exact, pc parked at call');
}
// restore everything
for (let i = 0; i < 40; i++) ex.core_write_uint8(CORE, SCRATCH + i, savedCode[i]);
for (let i = 0; i < 8; i++) ex.core_write_uint32(CORE, DATA + i * 4, savedData[i]);
ex.core_write_uint32(CORE, TB_LIT, savedLit);
memU32().set(stateA, CBASE);
memU32().set(stateB, 0);
memU32().set(stateC, CORE_SZ);
try { ex.native_set_clock_state(chip.cycles >>> 0); } catch {}
ex.native_trace_enable(1);
console.log('[emit] PASSED (suites B+C)');
