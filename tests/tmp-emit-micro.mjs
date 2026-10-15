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
// Micro-proofs: one shape family per synthetic chain, 20 random trials each.
// Compares JIT vs single-stepped interpreter on regs + touched memory.
const MICROS = [
  { name: 'l32in', ops: [{ op: 0x218, w: 2 }], regs: [1, 2], setup: (V) => { V[2] = DATA; }, cmp: [1] },
  { name: 's32in', ops: [{ op: 0x1349, w: 2 }], regs: [3, 4], setup: (V) => { V[3] = DATA + 8; }, cmp: [3, 4], mem: [DATA + 12] },
  { name: 'extui', ops: [{ op: 0x75a820, w: 3 }], regs: [2, 10], setup: null, cmp: [10] },
  { name: 'alu3', ops: [{ op: 0x1022a0, w: 3 }, { op: 0x2088a0, w: 3 }, { op: 0xc0aa30, w: 3 }], regs: [2, 3, 8, 10], setup: null, cmp: [2, 8, 10] },
  { name: 'rsrcc', ops: [{ op: 0x3ea20, w: 3 }], regs: [2], setup: null, cmp: [2] },
  { name: 'bne-robust', ops: [{ op: 0xFF9DE7, w: 3 }], regs: [13, 14], setup: (V) => { V[13] = V[14]; }, cmp: [13, 14] },
  { name: 'ssai-src', ops: [{ op: 0x404600, w: 3 }, { op: 0x818880, w: 3 }], regs: [7, 8], setup: null, cmp: [7, 8], spec3: true },
  { name: 'l32r', ops: [{ op: 0xfa5381, w: 3 }], regs: [8], setup: null, cmp: [8], lit: 0xfa5381 },
  { name: 'slli', ops: [{ op: 0x18280, w: 3 }], regs: [2, 8], setup: null, cmp: [8] },
  { name: 'srai', ops: [{ op: 0x219890, w: 3 }], regs: [9], setup: null, cmp: [9] },
  { name: 'addx2', ops: [{ op: 0xa02320, w: 3 }], regs: [2, 3], setup: null, cmp: [2] },
  { name: 'l8ui', ops: [{ op: 0x982, w: 3 }], regs: [8, 9], setup: (V) => { V[9] = DATA; }, cmp: [8] },
  { name: 'nop', ops: [{ op: 0x20c0, w: 3 }], regs: [1, 2], setup: null, cmp: [1, 2] },
  { name: 'bnezn', ops: [{ op: 0xDCC, w: 2 }], regs: [13], setup: (V) => { V[13] = 0; }, cmp: [13] },
  { name: 'l32i8', ops: [{ op: 0x2212, w: 3 }], regs: [1, 2], setup: (V) => { V[2] = DATA; }, cmp: [1] },
  { name: 'bge', ops: [{ op: 0xFFA687, w: 3 }], regs: [6, 8], setup: (V) => { V[6] = 0; V[8] = 1; }, cmp: [6, 8] },
  { name: 'bgeu', ops: [{ op: 0xFFB687, w: 3 }], regs: [6, 8], setup: (V) => { V[6] = 0; V[8] = 1; }, cmp: [6, 8] },
  { name: 'blt', ops: [{ op: 0xFF2687, w: 3 }], regs: [6, 8], setup: (V) => { V[6] = 5; V[8] = 5; }, cmp: [6, 8] },
  { name: 'j', ops: [{ op: 0xFFFFC6, w: 3 }], regs: [1, 2], setup: null, cmp: [1, 2] },
  { name: 'mov', ops: [{ op: 0x65D, w: 2 }], regs: [5, 6], setup: null, cmp: [5] },
  { name: 'addi8', ops: [{ op: 0x10C652, w: 3 }], regs: [5, 6], setup: null, cmp: [5] },
  { name: 'extui-2', ops: [{ op: 0x5a080, w: 3 }], regs: [8, 10], setup: null, cmp: [10] },
  { name: 'extui-3', ops: [{ op: 0x458180, w: 3 }], regs: [8], setup: null, cmp: [8] },
  { name: 'l32in-2', ops: [{ op: 0x288, w: 2 }], regs: [2, 8], setup: (V) => { V[2] = DATA; }, cmp: [8] },
  { name: 'l32in-3', ops: [{ op: 0x3228, w: 2 }], regs: [2], setup: (V) => { V[2] = DATA; }, cmp: [2] },
  { name: 's32i8', ops: [{ op: 0x16682, w: 3 }], regs: [6, 8], setup: (V) => { V[6] = DATA; }, cmp: [6, 8], mem: [DATA + 4] },
  { name: 'bbci', ops: [{ op: 0x66397, w: 3 }], regs: [3], setup: (V) => { V[3] = 0x200; }, cmp: [3] },
  { name: 'srl', ops: [{ op: 0x418290, w: 3 }], regs: [2, 8, 9], setup: null, cmp: [8] },
  { name: 'xor', ops: [{ op: 0x308290, w: 3 }], regs: [2, 8, 9], setup: null, cmp: [8] },
  { name: 'beqtbl', ops: [{ op: 0xf71826, w: 3 }], regs: [8], setup: (V) => { V[8] = 7; }, cmp: [8] },
  { name: 'fload', ops: [{ op: 0x82800, w: 3 }], regs: [0, 8], setup: (V) => { V[8] = DATA; V[0] = 0; }, cmp: [], fcmp: [2] },
  { name: 'l16i', ops: [{ op: 0x9682, w: 3 }], regs: [6, 8], setup: (V) => { V[6] = DATA; }, cmp: [8] },
  { name: 'bnetbl', ops: [{ op: 0x20466, w: 3 }], regs: [4], setup: (V) => { V[4] = 0xFFFFFFFF; }, cmp: [4] },
  { name: 'bbsi', ops: [{ op: 0xf1e807, w: 3 }], regs: [8], setup: (V) => { V[8] = 0xFFFFFFFE; }, cmp: [8] },
  { name: 's8i', ops: [{ op: 0x4382, w: 3 }], regs: [3, 8], setup: (V) => { V[3] = DATA + 8; }, cmp: [3, 8], mem: [DATA + 8] },
  { name: 'movnez-t', ops: [{ op: 0x932930, w: 3 }], regs: [2, 3, 9], setup: (V) => { if (V[3] === 0) V[3] = 1; }, cmp: [2] },
  { name: 'movnez-n', ops: [{ op: 0x932930, w: 3 }], regs: [2, 3, 9], setup: (V) => { V[3] = 0; }, cmp: [2] },
  { name: 'moveqz-t', ops: [{ op: 0x838660, w: 3 }], regs: [6, 8], setup: (V) => { V[6] = 0; }, cmp: [8] },
  { name: 'moveqz-n', ops: [{ op: 0x838660, w: 3 }], regs: [6, 8], setup: (V) => { V[6] = 1; }, cmp: [8] },
  { name: 'add', ops: [{ op: 0x803390, w: 3 }], regs: [3, 9], setup: null, cmp: [3] },
  { name: 'bbr', ops: [{ op: 0xec897, w: 3 }], regs: [8, 9], setup: (V) => { V[8] = 0xFFFFFFFF; }, cmp: [8, 9] },
  { name: 'bltz', ops: [{ op: 0xfe7296, w: 3 }], regs: [2], setup: (V) => { V[2] = 5; }, cmp: [2] },
  { name: 'l16u', ops: [{ op: 0x1ea2, w: 3 }], regs: [10, 14], setup: (V) => { V[14] = DATA; }, cmp: [10] },
  { name: 's16i', ops: [{ op: 0x5592, w: 3 }], regs: [5, 9], setup: (V) => { V[5] = DATA + 8; }, cmp: [5, 9], mem: [DATA + 8] },
  { name: 'neg', ops: [{ op: 0x602020, w: 3 }], regs: [2], setup: null, cmp: [2] },
  { name: 'bgetbl', ops: [{ op: 0x3087f6, w: 3 }], regs: [7], setup: (V) => { V[7] = V[7] % 8; }, cmp: [7] },
  { name: 'bban', ops: [{ op: 0x65387, w: 3 }], regs: [3, 8], setup: (V) => { V[3] = 0xFFFFFFFF; }, cmp: [3, 8] },
  { name: 'nsa', ops: [{ op: 0x40f550, w: 3 }], regs: [5], setup: null, cmp: [5] },
  { name: 'muluhf', ops: [{ op: 0xa23360, w: 3 }], regs: [3, 6], setup: null, cmp: [3] },
  { name: 'wsar', ops: [{ op: 0x401400, w: 3 }], regs: [4], setup: null, cmp: [4] },
  { name: 'ssl', ops: [{ op: 0xa1aa00, w: 3 }], regs: [10], setup: null, cmp: [10] },
  { name: 'subx4', ops: [{ op: 0xe0a990, w: 3 }], regs: [9, 10], setup: null, cmp: [10] },
  { name: 'subx8', ops: [{ op: 0xf0a990, w: 3 }], regs: [9, 10], setup: null, cmp: [10] },
  { name: 'bgetbls', ops: [{ op: 0x518e6, w: 3 }], regs: [8], setup: (V) => { V[8] = 0; }, cmp: [8] },
  { name: 's32c1i', ops: [{ op: 0x3fe502, w: 3 }], regs: [0, 5], setup: (V) => { V[5] = DATA + 8; }, cmp: [0, 5], mem: [DATA + 260] },
  { name: 'bbrn', ops: [{ op: 0xb4387, w: 3 }], regs: [3, 8], setup: (V) => { V[3] = 0; }, cmp: [3, 8] },
  { name: 'minu', ops: [{ op: 0x638200, w: 3 }], regs: [0, 2, 8], setup: null, cmp: [8] },
];
globalThis.__l32log = 6;
const savedD = [];
for (let i = 0; i < 8; i++) savedD.push(ex.core_read_uint32(CORE, DATA + i * 4) >>> 0);
const savedCode = [];
for (let i = 0; i < 32; i++) savedCode.push(ex.core_read_uint8(CORE, SCRATCH + i) >>> 0);
let allOk = true;
for (const m of MICROS) {
  const T = chainPcs(m.ops, SCRATCH);
  placeChain(m.ops, SCRATCH);
  let litAddr = 0, savedLit = 0;
  if (m.lit !== undefined) {
    litAddr = ((T[0].next >>> 2) + (0xffff0000 | ((m.lit >> 8) & 65535))) << 2;
    savedLit = ex.core_read_uint32(CORE, litAddr) >>> 0;
  }
  if (m.lit !== undefined) ex.core_write_uint32(CORE, litAddr, 0xA5A5A5A5); // stable before fold bakes it
  const built = buildTraceModule(T.map((e) => ({ ...e })), { CORE, CBASE, wb: wb0, recLoop, sarOff, readMem: (a) => ex.core_read_uint32(CORE, a) >>> 0 });
  const fn = await instantiateTrace(built.mod, L, ex);
  let pass = 0, discard = 0, fail = 0;
  for (let t = 0; t < 20; t++) {
    const V = {};
    for (const r of m.regs) V[r] = rnd32() >>> 0;
    if (m.setup) m.setup(V);
    const snap = () => {
      const o = {};
      for (const r of m.cmp) o['r' + r] = memU32()[P(r)] >>> 0;
      if (m.fcmp) for (const r of m.fcmp) o['f' + r] = memU32()[((CBASE + 272) >>> 2) + r] >>> 0;
      if (m.mem) for (const a of m.mem) o['m' + a.toString(16)] = ex.core_read_uint32(CORE, a) >>> 0;
      if (m.spec3) o.sar = memU32()[((CBASE + 336) >>> 2) + 3] >>> 0;
      return o;
    };
    memU32().set(stateA, CBASE);
    for (const r of m.regs) memU32()[P(r)] = V[r];
    if (m.lit !== undefined) ex.core_write_uint32(CORE, litAddr, 0xA5A5A5A5);
    let a;
    try { fn(); a = snap(); } catch (e) { fail++; console.log(`[micro] ${m.name} t=${t} JIT threw: ${e.message}`); break; }
    memU32().set(stateA, CBASE);
    for (const r of m.regs) memU32()[P(r)] = V[r];
    if (m.lit !== undefined) ex.core_write_uint32(CORE, litAddr, 0xA5A5A5A5);
    wc0.PC = SCRATCH;
    let expect = SCRATCH, bad = false;
    for (const { w } of m.ops) {
      expect += w;
      ex.core_run(1);
      if ((wc0.PC >>> 0) !== (expect >>> 0)) { bad = true; break; }
    }
    if (bad || (spec72() >>> 0) !== (wb0 >>> 0)) { discard++; continue; }
    const b = snap();
    const ka = Object.keys(a);
    if (ka.every((k) => a[k] === b[k])) pass++;
    else { fail++; console.log(`[micro] ${m.name} t=${t} MISMATCH jit=${JSON.stringify(a)} ora=${JSON.stringify(b)}`); if (fail > 2) break; }
  }
  console.log(`[micro] ${m.name}: pass=${pass} discard=${discard} fail=${fail}`);
  if (fail || pass < 10) allOk = false;
  if (m.lit !== undefined) ex.core_write_uint32(CORE, litAddr, savedLit);
}
for (let i = 0; i < 32; i++) ex.core_write_uint8(CORE, SCRATCH + i, savedCode[i]);
for (let i = 0; i < 8; i++) ex.core_write_uint32(CORE, DATA + i * 4, savedD[i]);
memU32().set(stateA, CBASE);
memU32().set(stateB, 0);
memU32().set(stateC, CORE_SZ);
try { ex.native_set_clock_state(chip.cycles >>> 0); } catch {}
ex.native_trace_enable(1);
console.log(allOk ? '[micro] ALL PASSED' : '[micro] FAILED');
process.exit(allOk ? 0 : 1);
