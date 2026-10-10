// JIT trace emitter (production, 50mips Phase 2). Pure JS: classify + emit
// a recorded linear trace to a WASM module over shared engine memory.
// Routing verified exhaustively: full 24-bit narrow-op fuzz vs the real
// c_handler7 tree, 0 misroutes (tests/tmp-jit-lib.mjs is the dev copy).
// Kill switch: callers simply never call (interpreter covers everything).
function uleb(n) { const o = []; do { let b = n & 0x7f; n >>>= 7; if (n) b |= 0x80; o.push(b); } while (n); return o; }
function sleb(n) { const o = []; let more = true; while (more) { let b = n & 0x7f; n >>= 7; if ((n === 0 && (b & 0x40) === 0) || (n === -1 && (b & 0x40) !== 0)) more = false; else b |= 0x80; o.push(b); } return o; }
const OP = {
  end: 0x0b, if_: 0x04, else_: 0x05, return_: 0x0f, call: 0x10,
  local_get: 0x20, local_set: 0x21,
  i32_const: 0x41, i32_eqz: 0x45, i32_eq: 0x46, i32_ne: 0x47, i32_lt_u: 0x48,
  i32_add: 0x6a, i32_sub: 0x6b, i32_mul: 0x6c, i32_shl: 0x74, i32_shr_u: 0x76,
  i32_load: 0x28, i32_store: 0x36,
};
const sext8 = (v) => (v & 128) ? (v | 0xffffff00) : v;

// ---- op classification (handler-verified) ----
export function classify(op) {
  // run_instruction intercepts (before c_handler7): RSIL, WAITI. The fuzz
  // only models c_handler7, so these are excluded explicitly (else a waiti
  // level or rsil encoding colliding with a shape below would misroute).
  if (((op & 0xffff0f) >>> 0) === 0x60c0) return { k: 'unknown', op }; // RSIL (w3)
  if ((op & 0xff) === 0x1d && ((op >> 8) & 0xff) === 0xf0 && ((op >> 16) & 0xf0) === 0x30)
    return { k: 'unknown', op }; // WAITI
  const lo = op & 15;
  const f12 = (op >> 12) & 15, f8 = (op >> 8) & 15, f4 = (op >> 4) & 15;
  // Narrow single-arm shapes (no sub-tests in c_handler7 — stable).
  if (lo === 8) { const imm = (op >> 12) & 15; return { k: 'l32in', t: f4, s: f8, off: imm << 2 }; } // r_handler47
  if (lo === 9) { const imm = (op >> 12) & 15; return { k: 's32in', t: f4, s: f8, off: imm << 2 }; } // _handler21
  if (lo === 10) return { k: 'addN', t: f12, r: f8, s: f4 }; // n_handler9
  if (lo === 11) { const s4 = f4; return { k: 'addiN', t: f12, r: f8, imm: (s4 !== 0 ? s4 : 0xffffffff) >>> 0 }; } // n_handler12
  if (lo === 12 && !(op & 128)) { // movi.n a_handler13 (bit7 clear)
    const idx = f12, clk = f8, sim = ((((op >> 4) & 7) << 4) | idx);
    return { k: 'moviN', t: clk, imm: ((sim & 96) === 96 ? (sim | 0xffffff80) : sim) >>> 0 };
  }
  if (lo === 1) return { k: 'l32r' }; // r_handler48 (fields at emit)
  if (lo === 7 && (op & 57344) === 32768 && (op & 4096) === 4096) // bne n_handler46
    return { k: 'bne', r: f8, s: f4, imm: (op >> 16) & 255 };
  if (lo === 7 && (op & 57344) === 8192 && (op & 4096) === 4096) // bltu n_handler42
    return { k: 'bltu', r: f8, s: f4, imm: (op >> 16) & 255 };
  if (lo === 2 && (op & 61440) === 40960) { // movi a_handler12
    const idx = (op >> 16) & 255, clk = f8, dst = f4, raw = (clk << 8) | idx;
    return { k: 'movi', t: dst, imm: ((raw & 2048) ? (raw | 0xfffff000) : raw) >>> 0 };
  }
  if (lo === 2 && (op & 61440) === 53248) { // addmi n_handler13
    const idx = (op >> 16) & 255;
    return { k: 'addmi', t: f4, r: f8, imm: ((sext8(idx) << 8) >>> 0) };
  }
  if (lo === 5) return { k: 'callEnd' }; // r_handler0-3 J/CALL*: trace boundary (all change next_pc)
  if (lo === 6 && (op & 48) === 48 && (op & 192) === 64) { // LOOP setters: trace boundary
    const v = op & 61440;
    if (v === 32768 || v === 36864 || v === 40960) return { k: 'loopEnd' };
  }
  if (lo === 0) {
    const e = op & 0xE0000, f = op & 0xE10000, b20 = op & 0x100000;
    if (e === 0x20000 && f === 0x800000 && b20 === 0) return { k: 'mull', t: f12, r: f8, s: f4 }; // a_handler37 full mul
    if (e === 0 && f === 0x810000 && b20 === 0) return { k: 'src', t: f12, r: f8, s: f4 }; // _handler37 (formula as coded)
    if (e === 0x40000) { // r_handler26 (formula as coded, NOT objdump name)
      const idx = (op >> 20) & 15, clk = (op >> 16) & 1, rr = (op >> 8) & 15;
      return { k: 'extui', t: f12, s: f4, shift: ((clk << 4) | rr) >>> 0, mask: ((1 << (idx + 1)) - 1) >>> 0 };
    }
    if (e === 0 && (op & 0xE10000) === 0 && b20 !== 0) return { k: 'and', t: f12, r: f8, s: f4 }; // n_handler19
    if (e === 0 && f === 0x200000 && b20 === 0) return { k: 'or', t: f12, r: f8, s: f4 }; // a_handler51
    if (e === 0 && f === 0xC00000 && b20 === 0) return { k: 'sub', t: f12, r: f8, s: f4 }; // _handler49
    if (e === 0x20000 && (op & 0xF10000) === 65536) { // _handler14 RSR: inline ccount only
      const sr = (op >> 8) & 255;
      if (sr === 234) return { k: 'rsrcc', t: f4 };
      return { k: 'unknown', op };
    }
    if (e === 0 && f === 0x400000 && (op & 1110016) === 16384) return { k: 'ssai' }; // _handler42 (no window call)
  }
  return { k: 'unknown', op };
}

export function buildTraceModule(T, ctx) {
  const { CORE, CBASE, wb, recLoop, sarOff } = ctx;
  // callEnd (r_handler2 CALL family): trace boundary. Emit the prefix only;
  // endPC is the call's own pc; the interpreter executes it on continuation.
  // (A call at index 0 leaves nothing to emit -> unemittable, correctly.)
  let endPC = null;
  {
    const j = T.findIndex((e, i) => { const k = classify(e.op).k; return k === 'callEnd' || k === 'loopEnd'; });
    if (j >= 0) {
      if (j === 0) throw new Error('boundary at trace start (nothing to emit)');
      endPC = T[j].pc;
      T = T.slice(0, j);
      T.forEach((e, i) => { e.next = (i + 1 < T.length) ? T[i + 1].pc : endPC; });
    } else {
      endPC = T[T.length - 1].next;
      if (endPC === null || endPC === undefined) throw new Error('unbounded trace (no end pc)');
    }
  }
  const kinds = T.map((e) => classify(e.op));
  kinds.forEach((cc, i) => { if (cc.k === 'unknown') throw new Error('unemittable op 0x' + T[i].op.toString(16) + ' at ' + i); });
  for (const e of T) {
    const lo = e.op & 15;
    if (e.w === 4) throw new Error('wide op needs pie guard (not implemented v1)');
    if (lo === 4 || (lo === 0 && (((e.op >> 16) & 255) === 22 || ((e.op >> 16) & 255) === 23))) throw new Error('pie31-routable narrow op (not implemented v1)');
  }
  const branches = [];
  kinds.forEach((cc, i) => {
    if (cc.k === 'bne' || cc.k === 'bltu') {
      const target = (T[i].pc + sext8(cc.imm) + 4) >>> 0;
      const takenRec = T[i].next !== ((T[i].pc + T[i].w) >>> 0);
      if (takenRec && T[i].next !== target) throw new Error(`branch target mismatch at ${i}`);
      branches.push({ idx: i, pc: T[i].pc, w: T[i].w, kind: cc.k, r: cc.r, s: cc.s, target, takenRec });
    }
  });
  const P = (reg) => CBASE + 16 + ((((wb << 2) + reg) & 63)) * 4;
  const SPEC3 = CBASE + 336 + 3 * 4, LOOPC = CBASE + 336 + 2 * 4;
  const B = [];
  const cc2 = (v) => B.push(OP.i32_const, ...sleb(v | 0));
  const ld = (a) => { cc2(a); B.push(OP.i32_load, 0x02, 0x00); };
  let bj = 0;
  const loopOn = recLoop[2] !== 0;
  const hits = new Set();
  if (loopOn) T.forEach((e, i) => { if (e.next === recLoop[1]) hits.add(i); });
  T.forEach((e, i) => {
    const k = kinds[i];
    const loopDec = loopOn && hits.has(i);
    const afterLoop = () => { if (loopDec) { cc2(LOOPC); ld(LOOPC); cc2(1); B.push(OP.i32_sub); B.push(OP.i32_store, 0x02, 0x00); } };
    if (k.k === 'addiN') { cc2(P(k.t)); ld(P(k.r)); cc2(k.imm); B.push(OP.i32_add, OP.i32_store, 0x02, 0x00); }
    else if (k.k === 'addN') { cc2(P(k.t)); ld(P(k.r)); ld(P(k.s)); B.push(OP.i32_add, OP.i32_store, 0x02, 0x00); }
    else if (k.k === 'mull') { cc2(P(k.t)); ld(P(k.r)); ld(P(k.s)); B.push(OP.i32_mul, OP.i32_store, 0x02, 0x00); }
    else if (k.k === 'moviN' || k.k === 'movi') { cc2(P(k.t)); cc2(k.imm); B.push(OP.i32_store, 0x02, 0x00); }
    else if (k.k === 'addmi') { cc2(P(k.t)); ld(P(k.r)); cc2(k.imm); B.push(OP.i32_add, OP.i32_store, 0x02, 0x00); }
    else if (k.k === 'src') {
      cc2(SPEC3); B.push(OP.i32_load, 0x02, 0x00); cc2(63); B.push(0x71); B.push(OP.local_set, 0x00);
      B.push(OP.local_get, 0x00); B.push(OP.i32_eqz);
      B.push(OP.if_, 0x40); cc2(P(k.t)); ld(P(k.s)); B.push(OP.i32_store, 0x02, 0x00);
      B.push(OP.else_);
      B.push(OP.local_get, 0x00); cc2(32); B.push(OP.i32_eq);
      B.push(OP.if_, 0x40); cc2(P(k.t)); ld(P(k.r)); B.push(OP.i32_store, 0x02, 0x00);
      B.push(OP.else_);
      cc2(P(k.t)); ld(P(k.r)); cc2(32); B.push(OP.local_get, 0x00); B.push(OP.i32_sub); B.push(OP.i32_shl);
      ld(P(k.s)); B.push(OP.local_get, 0x00); B.push(OP.i32_shr_u); B.push(0x72);
      B.push(OP.i32_store, 0x02, 0x00);
      B.push(OP.end, OP.end);
    }
    else if (k.k === 'l32in') { // r_handler47: dst = MR32(ar[s]+off)
      c(P(k.t)); c(CORE); ld(P(k.s)); c(k.off); B.push(OP.i32_add, OP.call, 0x02, OP.i32_store, 0x02, 0x00);
    }
    else if (k.k === 's32in') { // _handler21: MW32(ar[s]+off, ar[t]); addr 0 traps -> side exit
      ld(P(k.s)); c(k.off); B.push(OP.i32_add, OP.local_tee, 0x00, OP.i32_eqz, OP.if_, 0x40, OP.else_);
      branches.push({ idx: i, pc: e.pc }); c(100 + branches.length - 1); B.push(OP.return_, OP.end);
      c(CORE); B.push(OP.local_get, 0x00); ld(P(k.t)); B.push(OP.call, 0x05);
    }
    else if (k.k === 'extui') { // r_handler26 formula as coded
      c(P(k.t)); ld(P(k.s)); c(k.shift); B.push(OP.i32_shr_u); c(k.mask); B.push(OP.i32_and, OP.i32_store, 0x02, 0x00);
    }
    else if (k.k === 'and') { c(P(k.t)); ld(P(k.r)); ld(P(k.s)); B.push(OP.i32_and, OP.i32_store, 0x02, 0x00); }
    else if (k.k === 'or') { c(P(k.t)); ld(P(k.r)); ld(P(k.s)); B.push(0x72, OP.i32_store, 0x02, 0x00); }
    else if (k.k === 'sub') { c(P(k.t)); ld(P(k.r)); ld(P(k.s)); B.push(OP.i32_sub, OP.i32_store, 0x02, 0x00); }
    else if (k.k === 'rsrcc') { c(P(k.t)); c(CBASE + 336 + 234 * 4); B.push(OP.i32_load, 0x02, 0x00, OP.i32_store, 0x02, 0x00); }
    else if (k.k === 'l32r') { if (e.next === null) throw new Error('l32r as last trace op (no next pc)'); cc2(P(k.t)); cc2(CORE); cc2(e.op); cc2(e.next); B.push(OP.call, 0x06, OP.i32_store, 0x02, 0x00); }
    else if (k.k === 'ssai') {
      if (sarOff == null) { cc2(CORE); cc2(e.op); B.push(OP.call, 0x07); }
      else {
        // inline _handler42 (window-check-free): EXC_CAUSE word + pending=0.
        // sarOff is Rust-measured (native_sar_pending_off), never guessed.
        const sarVal = ((((e.op >> 4) & 1) << 4) | ((e.op >> 8) & 15)) >>> 0;
        cc2(CBASE + 336 + 12); cc2(sarVal); B.push(OP.i32_store, 0x02, 0x00);
        cc2(CBASE + sarOff); cc2(0); B.push(OP.i32_store, 0x02, 0x00);
      }
    }
    else if (k.k === 'bne' || k.k === 'bltu') {
      const br = branches[bj++];
      ld(P(br.r)); ld(P(br.s)); B.push(k.k === 'bne' ? OP.i32_ne : OP.i32_lt_u);
      cc2(br.takenRec ? 1 : 0); B.push(OP.i32_eq);
      B.push(OP.if_, 0x40, OP.else_); cc2(100 + branches.indexOf(br)); B.push(OP.return_); B.push(OP.end);
    }
    afterLoop();
  });
  cc2(0);
  const encStr = (s) => [s.length, ...[...s].map((ch) => ch.charCodeAt(0))];
  const typePay = [0x05,
    0x60, 0x02, 0x7f, 0x7f, 0x01, 0x7f, // t0: (i32,i32)->i32 mr8/16/32
    0x60, 0x03, 0x7f, 0x7f, 0x7f, 0x01, 0x7f, // t1: (i32,i32,i32)->i32 l32r
    0x60, 0x03, 0x7f, 0x7f, 0x7f, 0x00, // t2: (i32,i32,i32)->void mw8/16/32
    0x60, 0x02, 0x7f, 0x7f, 0x00, // t3: (i32,i32)->void ssai
    0x60, 0x00, 0x01, 0x7f]; // t4: ()->i32 run
  const typeSec = [0x01, ...uleb(typePay.length), ...typePay];
  const impR = (nm) => [...encStr('env'), ...encStr(nm), 0x00, 0x00]; // (i32,i32)->i32
  const impW = (nm) => [...encStr('env'), ...encStr(nm), 0x00, 0x02]; // (i32,i32,i32)->void
  const impPay = [0x09,
    ...impR('mr8'), ...impR('mr16'), ...impR('mr32'),
    ...impW('mw8'), ...impW('mw16'), ...impW('mw32'),
    ...[...encStr('env'), ...encStr('jit_l32r'), 0x00, 0x01],
    ...[...encStr('env'), ...encStr('jit_ssai'), 0x00, 0x03],
    ...[...encStr('env'), ...encStr('mem'), 0x02, 0x03, ...uleb(1), ...uleb(65536)]];
  const impSec = [0x02, ...uleb(impPay.length), ...impPay];
  const funcSec = [0x03, ...uleb(2), 0x01, 0x04];
  const expPay = [0x01, ...encStr('run'), 0x00, 0x08];
  const expSec = [0x07, ...uleb(expPay.length), ...expPay];
  const codeBody = [0x01, 0x01, 0x7f, ...B, OP.end];
  const codeContent = [0x01, ...uleb(codeBody.length), ...codeBody];
  const codeSec = [0x0a, ...uleb(codeContent.length), ...codeContent];
  const mod = new Uint8Array([0x00, 0x61, 0x73, 0x6d, 0x01, 0x00, 0x00, 0x00, ...typeSec, ...impSec, ...funcSec, ...expSec, ...codeSec]);
  return { mod, branches, endPC, len: T.length };
}
export async function instantiateTrace(mod, L, ex) {
  const inst = await WebAssembly.instantiate(mod, {
    env: {
      mem: L.memory,
      mr8: (core, addr) => ex.core_read_uint8(core, addr),
      mr16: (core, addr) => ex.core_read_uint16(core, addr),
      mr32: (core, addr) => ex.core_read_uint32(core, addr),
      mw8: (core, addr, val) => ex.core_write_uint8(core, addr, val),
      mw16: (core, addr, val) => ex.core_write_uint16(core, addr, val),
      mw32: (core, addr, val) => ex.core_write_uint32(core, addr, val),
      jit_l32r: (core, op, nextpc) => ex.native_jit_l32r(core, op, nextpc),
      jit_ssai: (core, op) => ex.native_jit_ssai(core, op),
    },
  });
  return inst.instance.exports.run;
}
// All loop-closed slices [s,e) with pc[s]==pc[e], longest first.
// Shortest repeats are inner counted loops (back-edges flip when counts
// exhaust); longer periods are more likely direction-stable, but nothing
// beats live validation, so callers try candidates in this order.
export function findLoopSlices(T) {
  const first = new Map();
  const out = [];
  for (let i = 0; i < T.length; i++) {
    if (first.has(T[i].pc)) {
      const s = first.get(T[i].pc);
      if (i - s >= 8) out.push({ start: s, end: i });
    } else first.set(T[i].pc, i);
  }
  out.sort((a, b) => (b.end - b.start) - (a.end - a.start));
  return out;
}
export function findLoopSlice(T) {
  const all = findLoopSlices(T);
  return all.length ? all[0] : null;
}
