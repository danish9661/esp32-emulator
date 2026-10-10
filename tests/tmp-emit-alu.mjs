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

// ---------- WASM emitter (from tmp-jit-spike.mjs) ----------
function uleb(n) { const o = []; do { let b = n & 0x7f; n >>>= 7; if (n) b |= 0x80; o.push(b); } while (n); return o; }
function sleb(n) { const o = []; let more = true; while (more) { let b = n & 0x7f; n >>= 7; if ((n === 0 && (b & 0x40) === 0) || (n === -1 && (b & 0x40) !== 0)) more = false; else b |= 0x80; o.push(b); } return o; }
const OP = { end: 0x0b, i32_const: 0x41, i32_add: 0x6a, i32_mul: 0x6c, i32_and: 0x71, i32_load: 0x28, i32_store: 0x36 };
// Module with IMPORTED shared memory (env.mem), one void->void export 'f'.
function buildImportedMemModule(body) {
  const magic = [0x00, 0x61, 0x73, 0x6d, 0x01, 0x00, 0x00, 0x00];
  const typeSec = [0x01, ...uleb(4), 0x01, 0x60, 0x00, 0x00];
  const encStr = (s) => [s.length, ...[...s].map((c) => c.charCodeAt(0))];
  // import "env"."mem", kind=mem(2), limits flag 0x03 (shared+max), init 1, max 65536
  const impPayload = [0x01, ...encStr('env'), ...encStr('mem'), 0x02, 0x03, ...uleb(1), ...uleb(65536)];
  const impSec = [0x02, ...uleb(impPayload.length), ...impPayload];
  const funcSec = [0x03, ...uleb(2), 0x01, 0x00];
  const expPayload = [0x01, ...encStr('f'), 0x00, 0x00];
  const expSec = [0x07, ...uleb(expPayload.length), ...expPayload];
  const codeBody = [0x00, ...body, OP.end];
  const codeContent = [0x01, ...uleb(codeBody.length), ...codeBody];
  const codeSec = [0x0a, ...uleb(codeContent.length), ...codeContent];
  return new Uint8Array([...magic, ...typeSec, ...impSec, ...funcSec, ...expSec, ...codeSec]);
}
// Field extraction mirrors handlers.rs exactly.
const F = {
  addN: (op) => ({ t: (op >> 12) & 15, r: (op >> 8) & 15, s: (op >> 4) & 15 }),       // n_handler9
  addiN: (op) => ({ t: (op >> 12) & 15, r: (op >> 8) & 15, sim: (op >> 4) & 15 }),     // n_handler12
  mull: (op) => ({ t: (op >> 12) & 15, r: (op >> 8) & 15, s: (op >> 4) & 15 }),        // a_handler37: FULL 32-bit (a as i32).wrapping_mul(b as i32)
  moviN: (op) => { const idx = (op >> 12) & 15, clk = (op >> 8) & 15; const sim = (((op >> 4) & 7) << 4) | idx; return { t: clk, imm: (sim & 96) === 96 ? (sim | 0xffffff80) >>> 0 : sim }; }, // a_handler13
  movi: (op) => { const idx = (op >> 16) & 255, clk = (op >> 8) & 15, s = (op >> 4) & 15; const raw = (clk << 8) | idx; return { t: s, imm: (raw & 2048) ? (raw | 0xfffff000) >>> 0 : raw }; }, // a_handler12+n_handler2
};
function emitChain(ops, physAddr) {
  const B = [];
  const c = (v) => B.push(OP.i32_const, ...sleb(v | 0));
  const ld = (a) => { c(a); B.push(OP.i32_load, 0x02, 0x00); };
  const st = (a) => { B.push(OP.i32_const, ...sleb(a), OP.i32_store, 0x02, 0x00); };
  // stack discipline: push addr first, then value, then store. Reorder: emit value-seq with addr saved:
  // WTO: addr,value -> store. We emit: <addr> <value> store. So compute value first into... no locals;
  // instead: emit value ops that leave value on stack, but addr must be UNDER it. Order: c(addr) THEN value THEN store.
  for (const { kind, op } of ops) {
    if (kind === 'addiN') {
      const { t, r, sim } = F.addiN(op); const imm = sim !== 0 ? sim : 0xffffffff;
      c(physAddr(t)); ld(physAddr(r)); c(imm); B.push(OP.i32_add); B.push(OP.i32_store, 0x02, 0x00);
    } else if (kind === 'addN') {
      const { t, r, s } = F.addN(op);
      c(physAddr(t)); ld(physAddr(r)); ld(physAddr(s)); B.push(OP.i32_add); B.push(OP.i32_store, 0x02, 0x00);
    } else if (kind === 'mull') {
      const { t, r, s } = F.mull(op); // a_handler37: full 32-bit wrapping mul, NO masking
      c(physAddr(t)); ld(physAddr(r)); ld(physAddr(s)); B.push(OP.i32_mul); B.push(OP.i32_store, 0x02, 0x00);
    } else if (kind === 'moviN' || kind === 'movi') {
      const { t, imm } = kind === 'moviN' ? F.moviN(op) : F.movi(op);
      c(physAddr(t)); c(imm); B.push(OP.i32_store, 0x02, 0x00);
    } else throw new Error('kind ' + kind);
  }
  return B;
}

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

// ---------- chain + oracle ----------
const CHAIN = [
  { kind: 'addiN', op: 0x551b, w: 2 },  // addi.n a5,a5,1
  { kind: 'addN', op: 0x778a, w: 2 },   // add.n a7,a7,a8
  { kind: 'mull', op: 0x827740, w: 3 }, // mull a7,a7,a4
  { kind: 'movi', op: 0xc6a092, w: 3 }, // movi a9,198
  { kind: 'moviN', op: 0x983c, w: 2 },  // movi.n a8,57
  { kind: 'addiN', op: 0x661b, w: 2 },  // addi.n a6,a6,1
  { kind: 'mull', op: 0x828680, w: 3 }, // mull a8,a6,a8
  { kind: 'mull', op: 0x828580, w: 3 }, // mull a8,a5,a8
];
const TOTW = CHAIN.reduce((a, o) => a + o.w, 0);
const memU32 = () => new Uint32Array(L.memory.buffer);
const CORE_SZ = 4096, PHYS_OFF = 16, SPEC_OFF = 336, PC_OFF = 2584;
// pick the SPINNING core (the other may sit in WAITI: idle==1 steps nothing)
const idle0 = ex.core_get_idle(0) >>> 0, idle1 = ex.core_get_idle(1) >>> 0;
const pc0 = ex.core_get_pc(0) >>> 0, pc1 = ex.core_get_pc(1) >>> 0;
const inSpin = (p) => p >= 0x400d1600 && p <= 0x400d1700;
const CORE = (idle0 === 0 && (idle1 !== 0 || inSpin(pc0))) ? 0 : 1;
console.log('[emit] idle0=' + idle0 + ' idle1=' + idle1 + ' pc0=0x' + pc0.toString(16) + ' pc1=' + pc1.toString(16) + ' => CORE=' + CORE);
const CBASE = CORE * CORE_SZ;
const spec72 = () => memU32()[((CBASE + SPEC_OFF) >>> 2) + 72];
const physIdx = (reg) => (((spec72() << 2) + reg) & 63);
const physByte = (reg) => CBASE + PHYS_OFF + physIdx(reg) * 4;
const physWord = (reg) => ((CBASE + PHYS_OFF) >>> 2) + ((((wb0 << 2) + reg) & 63));
const wb0 = spec72();
console.log('[emit] windowbase(spec72)=' + wb0);
const mod = buildImportedMemModule(emitChain(CHAIN, (reg) => physByte(reg)));
console.log('[emit] module bytes=' + mod.length + ' shared-mem pages=' + (L.memory.buffer.byteLength / 65536));
const inst = await WebAssembly.instantiate(mod, { env: { mem: L.memory } });
const jitFn = inst.instance.exports.f;
if (typeof jitFn !== 'function') throw new Error('no export f');

const SCRATCH = 0x3ffe8000;
const savedCode = [];
for (let i = 0; i < TOTW; i++) savedCode.push(ex.core_read_uint8(CORE, SCRATCH + i) >>> 0);
// place chain bytes
{
  let a = SCRATCH;
  for (const { op, w } of CHAIN) { for (let i = 0; i < w; i++) ex.core_write_uint8(CORE, a + i, (op >>> (8 * i)) & 0xff); a += w; }
}
const wc0 = L._wasmCores[CORE];
const stateA = memU32().slice(CBASE, CBASE + CORE_SZ);
const stateB = memU32().slice(0, CORE_SZ);
const stateC = memU32().slice(CORE_SZ, 2 * CORE_SZ);
// diag: verify placement + single step once with full visibility
{
  let ok = true;
  let a = SCRATCH;
  for (const { op, w } of CHAIN) { for (let i = 0; i < w; i++) { if ((ex.core_read_uint8(CORE, a + i) >>> 0) !== ((op >>> (8 * i)) & 0xff)) ok = false; } a += w; }
  console.log('[emit] placement-readback=' + (ok ? 'OK' : 'MISMATCH'));
  wc0.PC = SCRATCH;
  console.log('[emit] pc-before=0x' + (ex.core_get_pc(CORE) >>> 0).toString(16) + ' idle=' + (ex.core_get_idle(CORE) >>> 0));
  ex.core_run(1);
  console.log('[emit] pc-after=0x' + (ex.core_get_pc(CORE) >>> 0).toString(16) + ' expect=0x' + (SCRATCH + CHAIN[0].w).toString(16) + ' opc=0x' + (ex.core_get_debug_opcode(CORE) >>> 0).toString(16));
  console.log('[emit] spec72-after=' + (spec72() >>> 0));
  memU32().set(stateA, CBASE); // restore (diag step dirtied regs)
}
let pass = 0, discard = 0, fail = 0;
let rnd = 0x12345678;
const rnd32 = () => (rnd = (Math.imul(rnd, 1103515245) + 12345) >>> 0);
const REGS = [4, 5, 6, 7, 8, 9];
for (let t = 0; t < 200; t++) {
  const inputs = REGS.map(() => rnd32() >>> 0);
  const runJit = () => {
    memU32().set(stateA, CBASE);
    REGS.forEach((r, i) => { memU32()[((CBASE + PHYS_OFF) >>> 2) + ((((wb0 << 2) + r) & 63))] = inputs[i]; });
    jitFn();
    return REGS.map((r) => memU32()[((CBASE + PHYS_OFF) >>> 2) + ((((wb0 << 2) + r) & 63))] >>> 0);
  };
  const runOracle = () => {
    memU32().set(stateA, CBASE);
    REGS.forEach((r, i) => { memU32()[((CBASE + PHYS_OFF) >>> 2) + ((((wb0 << 2) + r) & 63))] = inputs[i]; });
    wc0.PC = SCRATCH;
    let expect = SCRATCH;
    for (const { w } of CHAIN) {
      expect += w;
      ex.core_run(1);
      if ((wc0.PC >>> 0) !== (expect >>> 0)) return null; // interrupt diverted / exception: discard
    }
    if ((spec72() >>> 0) !== (wb0 >>> 0)) return null; // window moved: discard
    return REGS.map((r) => memU32()[((CBASE + PHYS_OFF) >>> 2) + ((((wb0 << 2) + r) & 63))] >>> 0);
  };
  const a = runJit();
  const b = runOracle();
  if (!b) { discard++; continue; }
  const eq = a.every((v, i) => v === b[i]);
  if (eq) pass++;
  else { fail++; console.log('[emit] MISMATCH t=' + t + ' in=' + inputs.map((v) => v.toString(16)) + ' jit=' + a.map((v) => v.toString(16)) + ' ora=' + b.map((v) => v.toString(16))); if (fail > 3) break; }
}
// restore everything
for (let i = 0; i < TOTW; i++) ex.core_write_uint8(CORE, SCRATCH + i, savedCode[i]);
memU32().set(stateA, CBASE);
memU32().set(stateB, 0);
memU32().set(stateC, CORE_SZ);
try { ex.native_set_clock_state(chip.cycles >>> 0); } catch {}
ex.native_trace_enable(1);
console.log(`[emit] pass=${pass} discard=${discard} fail=${fail}`);
if (fail || pass < 100) { console.log('[emit] FAILED'); process.exit(1); }
console.log('[emit] PASSED (ALU datapath bit-exact, window silent proven by pc advance)');
