// Shared JIT-trace emitter (milestone C). Pure JS: classify + emit.
// Context: { CORE, CBASE, wb, recLoop:[begin,end,count] }.
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
  switch (op) {
    case 0x661b: case 0x551b: { const s4 = (op >> 4) & 15; return { k: 'addiN', t: (op >> 12) & 15, r: (op >> 8) & 15, imm: (s4 !== 0 ? s4 : 0xffffffff) >>> 0 }; }
    case 0x778a: return { k: 'addN', t: (op >> 12) & 15, r: (op >> 8) & 15, s: (op >> 4) & 15 };
    case 0x827750: case 0x827740: case 0x828680: case 0x828580:
      return { k: 'mull', t: (op >> 12) & 15, r: (op >> 8) & 15, s: (op >> 4) & 15 };
    case 0x983c: { const idx = (op >> 12) & 15, clk = (op >> 8) & 15, s = (((op >> 4) & 7) << 4) | idx; return { k: 'moviN', t: clk, imm: ((s & 96) === 96 ? (s | 0xffffff80) : s) >>> 0 }; }
    case 0xc6a092: { const idx = (op >> 16) & 255, clk = (op >> 8) & 15, s = (op >> 4) & 15, raw = (clk << 8) | idx; return { k: 'movi', t: s, imm: ((raw & 2048) ? (raw | 0xfffff000) : raw) >>> 0 }; }
    case 0xfa5381: case 0xfa8081: case 0xfa5981: case 0xfa7b81:
      return { k: 'l32r', t: (op >> 4) & 15 };
    case 0xd39687: case 0xd39587: return { k: 'bne', r: (op >> 8) & 15, s: (op >> 4) & 15, imm: (op >> 16) & 255 };
    case 0x43987: return { k: 'bltu', r: (op >> 8) & 15, s: (op >> 4) & 15, imm: (op >> 16) & 255 };
    case 0x10d992: case 0x30d882: { const idx = (op >> 16) & 255, clk = (op >> 8) & 15, s = (op >> 4) & 15; return { k: 'addmi', t: s, r: clk, imm: ((sext8(idx) << 8) >>> 0) }; }
    case 0x818880: return { k: 'src', t: (op >> 12) & 15, r: (op >> 8) & 15, s: (op >> 4) & 15 };
    case 0x404600: return { k: 'ssai' };
    default: return { k: 'unknown', op };
  }
}

export function buildTraceModule(T, ctx) {
  const { CORE, CBASE, wb, recLoop, sarOff } = ctx;
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
    else if (k.k === 'l32r') { if (e.next === null) throw new Error('l32r as last trace op (no next pc)'); cc2(P(k.t)); cc2(CORE); cc2(e.op); cc2(e.next); B.push(OP.call, 0x00, OP.i32_store, 0x02, 0x00); }
    else if (k.k === 'ssai') {
      if (sarOff == null) { cc2(CORE); cc2(e.op); B.push(OP.call, 0x01); }
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
  const typePay = [0x03, 0x60, 0x03, 0x7f, 0x7f, 0x7f, 0x01, 0x7f, 0x60, 0x02, 0x7f, 0x7f, 0x00, 0x60, 0x00, 0x01, 0x7f];
  const typeSec = [0x01, ...uleb(typePay.length), ...typePay];
  const imp1 = [...encStr('env'), ...encStr('jit_l32r'), 0x00, 0x00];
  const imp2 = [...encStr('env'), ...encStr('jit_ssai'), 0x00, 0x01];
  const impMem = [...encStr('env'), ...encStr('mem'), 0x02, 0x03, ...uleb(1), ...uleb(65536)];
  const impPay = [0x03, ...imp1, ...imp2, ...impMem];
  const impSec = [0x02, ...uleb(impPay.length), ...impPay];
  const funcSec = [0x03, ...uleb(2), 0x01, 0x02];
  const expPay = [0x01, ...encStr('run'), 0x00, 0x02];
  const expSec = [0x07, ...uleb(expPay.length), ...expPay];
  const codeBody = [0x01, 0x01, 0x7f, ...B, OP.end];
  const codeContent = [0x01, ...uleb(codeBody.length), ...codeBody];
  const codeSec = [0x0a, ...uleb(codeContent.length), ...codeContent];
  const mod = new Uint8Array([0x00, 0x61, 0x73, 0x6d, 0x01, 0x00, 0x00, 0x00, ...typeSec, ...impSec, ...funcSec, ...expSec, ...codeSec]);
  return { mod, branches };
}
export async function instantiateTrace(mod, L, ex) {
  const inst = await WebAssembly.instantiate(mod, {
    env: {
      mem: L.memory,
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
