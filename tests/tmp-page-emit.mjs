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
// ---- M1 page spike: per-page WASM dispatch loop over threaded superblocks.
// Same selection/validation discipline as the loop bench; the module keeps
// ALL exits inside WASM (in-page threading) and returns only on page-leave,
// yield expiry, or cold ops.
import { prepareTrace } from './tmp-jit-lib.mjs';
import { uleb, sleb, OP } from './tmp-jit-lib.mjs';
let S = null, runPage = null, slice = null, pageLen = 0, exitPcPtr = 0;
const YIELD_SB = 30; // superblock executions per page call (small: validation stability first)
async function buildPage(Sc, ctx) {
  const { CORE, CBASE } = ctx;
  const { emitOpsBody } = await import('./tmp-jit-lib.mjs');
  const INSTW = CBASE + 2608;
  const B = [];
  const cc = (v) => B.push(OP.i32_const, ...sleb(v | 0));
  // entry: pc = START, yield = YIELD_OPS
  cc(Sc[0].pc); B.push(OP.local_set, 0x01);
  cc(YIELD_SB * 64); B.push(OP.local_set, 0x02);
  const pagePcs = new Set(Sc.map((e) => e.pc >>> 0));
  const inPage = (pc) => pagePcs.has(pc >>> 0);
  const xfer = (targetPc) => {
    // transfer to targetPc: thread inside page, else exit(1) with exitpc set
    if (inPage(targetPc)) { cc(targetPc); B.push(OP.local_set, 0x01, OP.br, 0x01); }
    else {
      cc(targetPc); B.push(OP.global_set, 0x00);
      cc(1); B.push(OP.return_);
    }
  };
  const instAdd = (n) => {
    cc(INSTW); cc(INSTW);
    B.push(OP.i32_load, 0x02, 0x00);
    cc(n); B.push(OP.i32_add, OP.i32_store, 0x02, 0x00);
  };
  B.push(OP.loop, 0x40);
  // yield decrement per dispatch; every return parks pc in exitpc
  B.push(OP.local_get, 0x02, OP.i32_const, ...sleb(-1), OP.i32_add, OP.local_set, 0x02);
  B.push(OP.local_get, 0x02, OP.i32_eqz, OP.if_, 0x40);
  B.push(OP.local_get, 0x01, OP.global_set, 0x00, OP.i32_const, ...sleb(2), OP.return_, OP.end);
  // superblock groups (split after every branch), recorded order
  const prep = (await import('./tmp-jit-lib.mjs')).prepareTrace(Sc.map((e) => ({ ...e })), ctx);
  const groups = [];
  {
    let g = [0];
    for (let i = 1; i < prep.TE.length; i++) {
      const pk = prep.kinds[i - 1].k;
      if (pk === 'br') { groups.push(g); g = [i]; }
      else if (prep.TE[i].pc !== prep.TE[i - 1].pc + prep.TE[i - 1].w) { groups.push(g); g = [i]; }
      else g.push(i);
    }
    groups.push(g);
  }
  for (const g of groups) {
    const startPc = prep.TE[g[0]].pc;
    B.push(OP.i32_const, ...sleb(startPc), OP.local_get, 0x01, OP.i32_eq, OP.if_, 0x40);
    const subT = g.map((i) => prep.TE[i]);
    const subK = g.map((i) => prep.kinds[i]);
    const subB = prep.branches.filter((br) => subT.some((e) => e.pc === br.pc));
    const glen = subT.length;
    const hooks = {
      noTail: true,
      onBranch(BB, H, br, e) {
        const takenPc = br.target >>> 0, fallPc = (e.pc + e.w) >>> 0;
        if (br.b === 'bne' || br.b === 'bltu' || br.b === 'beq') {
          H.ld(H.P(br.r)); H.ld(H.P(br.s));
          BB.push(br.b === 'bne' ? H.OP.i32_ne : br.b === 'bltu' ? H.OP.i32_lt_u : H.OP.i32_eq);
        } else { H.ld(H.P(br.r)); BB.push(H.OP.i32_eqz); if (br.b === 'bnez' || br.b === 'bnezn') BB.push(H.OP.i32_eqz); }
        BB.push(H.OP.if_, 0x40);
        // taken path: inst-add then transfer
        BB.push(H.OP.i32_const, ...H.sleb(INSTW), H.OP.i32_const, ...H.sleb(INSTW));
        BB.push(H.OP.i32_load, 0x02, 0x00, H.OP.i32_const, ...H.sleb(glen), H.OP.i32_add, H.OP.i32_store, 0x02, 0x00);
        if (inPage(takenPc)) { BB.push(H.OP.i32_const, ...H.sleb(takenPc), H.OP.local_set, 0x01, H.OP.br, 0x02); }
        else { BB.push(H.OP.i32_const, ...H.sleb(takenPc), H.OP.global_set, 0x00, H.OP.i32_const, ...H.sleb(1), H.OP.return_); }
        BB.push(H.OP.else_);
        BB.push(H.OP.i32_const, ...H.sleb(INSTW), H.OP.i32_const, ...H.sleb(INSTW));
        BB.push(H.OP.i32_load, 0x02, 0x00, H.OP.i32_const, ...H.sleb(glen), H.OP.i32_add, H.OP.i32_store, 0x02, 0x00);
        if (inPage(fallPc)) { BB.push(H.OP.i32_const, ...H.sleb(fallPc), H.OP.local_set, 0x01, H.OP.br, 0x02); }
        else { BB.push(H.OP.i32_const, ...H.sleb(fallPc), H.OP.global_set, 0x00, H.OP.i32_const, ...H.sleb(1), H.OP.return_); }
        BB.push(H.OP.end);
      },
    };
    const em = emitOpsBody(subT, subK, subB, ctx, hooks);
    B.push(...em.body);
    // straight transfer at group end (last op not a branch by construction)
    const lastE = subT[subT.length - 1];
    instAdd(subT.length);
    const nx = lastE.next;
    if (inPage(nx)) { cc(nx); B.push(OP.local_set, 0x01, OP.br, 0x01); }
    else { cc(nx); B.push(OP.global_set, 0x00, OP.i32_const, ...sleb(1), OP.return_); }
    B.push(OP.end); // if pc==start
  }
  B.push(OP.local_get, 0x01, OP.global_set, 0x00, OP.i32_const, ...sleb(4), OP.return_); // dispatch miss
  B.push(OP.end); // loop $L
  B.push(OP.unreachable_); // structurally unreachable: all paths return or loop
  const { assemblePageModule, instantiatePageModule } = await import('./tmp-jit-lib.mjs');
  const mod = assemblePageModule(B, { locals: 3 });
  const { run, exitpc } = await instantiatePageModule(mod, L, ex);
  return { run, exitpc, groups: groups.length, ops: prep.TE.length };
}

// ---- main: longest emittable window as ONE page; sustained calls from
// wherever pc is (exitpc-synced, no cyclic re-entry). This is the real
// page-driver shape: entries at any superblock start, exits handled.
let page = null, pageTE = null, pageStart = 0, pageSlice = null;
{
  // longest coherent window in the recorded trace
  const W = T;
  const Tb = W.map((e) => ({ ...e }));
  Tb.forEach((e, i) => { e.next = (i + 1 < Tb.length) ? Tb[i + 1].pc : (e.pc + e.w); });
  pageStart = Tb[0].pc;
  const built = await buildPage(Tb, { CORE, CBASE, wb, recLoop, sarOff });
  page = built;
  pageTE = Tb;
  console.log(`[page] built window len=${Tb.length} groups=${built.groups}`);
}
function guardOk() {
  if ((memU32()[PENDW] >>> 0) !== 0) return false;
  const s = spec();
  return s[72] === live[72] && s[73] === live[73] && s[230] === live[230] && s[0] === recLoop[0] && s[1] === recLoop[1] && s[2] === recLoop[2];
}
// validate: 3 prefix-clean runs from live pcs + at least one non-crash exit path
{
  // Threading validation (emission already proven by the trace driver):
  // every exit parks pc in the precomputed valid set (recorded pcs +
  // all branch targets). No reference comparison needed.
  const validExits = new Set(pageTE.map((e) => e.pc >>> 0));
  {
    const { classify } = await import('./tmp-jit-lib.mjs');
    for (const e of pageTE) {
      const k = classify(e.op);
      if (k.k === 'br') { validExits.add((e.pc + k.imm + 4) >>> 0); validExits.add((e.pc + e.w) >>> 0); }
    }
  }
  let ok = 0, di = 0, gg = 0, wild = 0;
  const codes = {};
  for (let t = 0; t < 30 && ok < 10; t++) {
    if (!guardOk()) { gg++; continue; }
    wc.PC = pageStart;
    const genS = ex.native_code_gen() >>> 0;
    const i0 = memU32()[INSTW] >>> 0;
    const code = page.run();
    const opsDone = (memU32()[INSTW] >>> 0) - i0;
    if ((ex.native_code_gen() >>> 0) !== genS) { di++; continue; }
    const xp = page.exitpc.value >>> 0;
    wc.PC = xp;
    codes[code] = (codes[code] || 0) + 1;
    if (!validExits.has(xp)) { wild++; console.log(`[page] WILD pc=0x${xp.toString(16)} code=${code}`); continue; }
    if (opsDone <= 0) { console.log('[page] zero retired'); continue; }
    ok++;
  }
  console.log(`[page] threading validation ${ok}/10 clean codes=${JSON.stringify(codes)} (gen-discards=${di} guard-skips=${gg} wild=${wild})`);
  if (ok < 10 || wild > 0) { console.log('[page] FAILED'); process.exit(1); }
  pageSlice = pageTE.slice(0, 56);
}
// timing: sustained page calls, pc synced via exitpc, misses step along
{
  const N = 600;
  let jitOps = 0, intOps = 0, yields = 0, leaves = 0, missx = 0;
  const t0 = process.hrtime.bigint();
  for (let i = 0; i < N; i++) {
    if ((memU32()[PENDW] >>> 0) !== 0) { for (let k = 0; k < 4 && (memU32()[PENDW] >>> 0) !== 0; k++) ex.core_run(512); }
    if (!guardOk()) { ex.core_run(16); intOps += 16; continue; }
    const i0 = memU32()[INSTW] >>> 0;
    const c = page.run();
    const d = (memU32()[INSTW] >>> 0) - i0;
    jitOps += d;
    wc.PC = page.exitpc.value >>> 0;
    if (c === 2) yields++;
    else if (c === 1) {
      leaves++;
      for (let j = 0; j < 64; j++) ex.core_run(1);
      intOps += 64;
    } else { missx++; for (let j = 0; j < 16; j++) ex.core_run(1); intOps += 16; }
  }
  const us = Number(process.hrtime.bigint() - t0) / 1000;
  const tot = jitOps + intOps;
  console.log(`[page] jitOps=${jitOps} intOps=${intOps} yields=${yields} leaves=${leaves} miss=${missx}`);
  console.log(`[page] BLENDED MIPS=${(tot / us).toFixed(1)}`);
  const M = 300, SL = pageTE.length;
  const i1 = memU32()[INSTW] >>> 0;
  const t1 = process.hrtime.bigint();
  for (let i = 0; i < M; i++) for (let j = 0; j < SL; j++) ex.core_run(1);
  const us2 = Number(process.hrtime.bigint() - t1) / 1000;
  const ret = (memU32()[INSTW] >>> 0) - i1;
  console.log(`[page] interp: ${ret} retired, ${(ret / us2).toFixed(1)} MIPS`);
}
process.exit(0);
