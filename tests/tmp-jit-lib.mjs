// Shared JIT-trace emitter (milestone C). Pure JS: classify + emit.
// Context: { CORE, CBASE, wb, recLoop:[begin,end,count] }.
export function uleb(n) { const o = []; do { let b = n & 0x7f; n >>>= 7; if (n) b |= 0x80; o.push(b); } while (n); return o; }
export function sleb(n) { const o = []; let more = true; while (more) { let b = n & 0x7f; n >>= 7; if ((n === 0 && (b & 0x40) === 0) || (n === -1 && (b & 0x40) !== 0)) more = false; else b |= 0x80; o.push(b); } return o; }
export const OP = {
  end: 0x0b, if_: 0x04, else_: 0x05, return_: 0x0f, call: 0x10, br: 0x0c, br_if: 0x0d,
  loop: 0x03, block: 0x02,
  local_get: 0x20, local_set: 0x21,
  i32_const: 0x41, i32_eqz: 0x45, i32_eq: 0x46, i32_ne: 0x47, i32_lt_u: 0x48, i32_and: 0x71, i32_or: 0x72, local_tee: 0x22,
  i32_add: 0x6a, i32_sub: 0x6b, i32_mul: 0x6c, i32_shl: 0x74, i32_shr_u: 0x76, i32_shr_s: 0x75,
  i32_load: 0x28, i32_store: 0x36,
  global_get: 0x23, global_set: 0x24, unreachable_: 0x00, nop_: 0x01, drop: 0x1a,
};
export const sext8 = (v) => (v & 128) ? (v | 0xffffff00) : v;

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
  if (lo === 1) return { k: 'l32r', t: (op >> 4) & 15 }; // r_handler48 dest=(>>4)&15
  if (lo === 7 && (op & 57344) === 32768 && (op & 4096) === 4096) // bne n_handler46
    return { k: 'br', b: 'bne', r: f8, s: f4, imm: sext8((op >> 16) & 255) };
  if (lo === 7 && (op & 57344) === 8192 && (op & 4096) === 4096) // bltu n_handler42
    return { k: 'br', b: 'bltu', r: f8, s: f4, imm: sext8((op >> 16) & 255) };
  if (lo === 7 && (op & 57344) === 0 && (op & 4096) === 4096) { // beq n_handler30
    const r = f8, ss = f4;
    return { k: 'br', b: 'beq', r, s: ss, imm: sext8((op >> 16) & 255) };
  }
  if (lo === 6 && (op & 48) === 16 && (op & 192) === 64) { // bnez n_handler48 (12-bit offset)
    const imm12 = (op >> 12) & 4095, se = (imm12 & 2048) ? (imm12 | 0xfffff000) : imm12;
    return { k: 'br', b: 'bnez', r: (op >> 8) & 15, s: -1, imm: se };
  }
  if (lo === 6 && (op & 48) === 16 && (op & 192) === 0) { // beqz n_handler32 (12-bit offset)
    const imm12 = (op >> 12) & 4095, se = (imm12 & 2048) ? (imm12 | 0xfffff000) : imm12;
    return { k: 'br', b: 'beqz', r: (op >> 8) & 15, s: -1, imm: se };
  }
  if (lo === 6 && (op & 48) === 48 && (op & 192) === 0) return { k: 'callEnd' }; // r_handler24 RETW: trace boundary
  if (lo === 12 && (op & 128) && (op & 64)) { // bnez.n n_handler49 (taken iff !=0, small unsigned offset)
    const sim = ((((op >> 4) & 3) << 4) | ((op >> 12) & 15)) >>> 0;
    return { k: 'br', b: 'bnezn', r: (op >> 8) & 15, s: -1, imm: sim };
  }
  if (lo === 13 && (op & 61440) === 61440 && (op & 240) === 16) return { k: 'callEnd' }; // retw.n a_handler62
  if (lo === 2 && (op & 61440) === 0) { // l8ui r_handler41
    const t = (op >> 4) & 15, ss = (op >> 8) & 15, off = (op >> 16) & 255;
    return { k: 'l8ui', t, s: ss, off };
  }
  if (lo === 12 && (op & 128) && !(op & 64)) { // beqz.n n_handler33 (small unsigned offset)
    const sim = ((((op >> 4) & 3) << 4) | ((op >> 12) & 15)) >>> 0;
    return { k: 'br', b: 'beqz', r: f8, s: -1, imm: sim };
  }

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
    if (e === 0 && f === 65536) { // slli _handler31
      const sh = 32 - ((((op >> 20) & 1) << 4) | ((op >> 4) & 15));
      return { k: 'slli', t: (op >> 12) & 15, s: (op >> 8) & 15, sh };
    }
    if (e === 0 && f === 2162688) { // srai _handler36
      const wa = ((((op >> 20) & 1) << 4) | ((op >> 8) & 15)) >>> 0;
      return { k: 'srai', t: (op >> 12) & 15, s: (op >> 4) & 15, sh: wa };
    }
    if (e === 0 && f === 0xA00000 && b20 === 0) return { k: 'addx2', t: (op >> 12) & 15, r: (op >> 8) & 15, s: (op >> 4) & 15 }; // n_handler15
    if (e === 0 && f === 0 && (op & 1048576) === 0 && (op & 61440) === 8192) { // NOP family (bare return)
      const sub = op & 4080;
      if (sub === 0 || sub === 16 || sub === 32 || sub === 48 || sub === 192 || sub === 208 || sub === 240)
        return { k: 'nop' };
    }
    if (e === 0 && (op & 0xE10000) === 0 && (op & 1048576) === 0 && (op & 61440) === 0 && (op & 240) === 224)
      return { k: 'callEnd' }; // r_handler6 RET: trace boundary
  }
  return { k: 'unknown', op };
}

export function prepareTrace(T, ctx) {
  // Boundary truncation + classification + branch resolution.
  // Returns { TE, endPC, kinds, branches }.
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
    if (cc.k === 'br') {
      const target = (T[i].pc + cc.imm + 4) >>> 0;
      const takenRec = T[i].next !== ((T[i].pc + T[i].w) >>> 0);
      if (takenRec && T[i].next !== target) throw new Error(`branch target mismatch at ${i}`);
      branches.push({ idx: i, pc: T[i].pc, w: T[i].w, kind: cc.k, b: cc.b, r: cc.r, s: cc.s, target, takenRec });
    }
  });
  return { TE: T, endPC, kinds, branches };
}

export function emitOpsBody(TE, kinds, branches, ctx, hooks = null) {
  // Emits the straight-line op bodies. Branch handling depends on hooks:
  // null (trace mode) = direction-checked side exits via return codes;
  // { onBranch(br, takenPc, fallPc) } (page mode) = custom emission that
  // must consume the taken-flag and route both targets (in-page threading
  // or exits) WITHOUT returning, unless it returns an exit code itself.
  // Returns { body, locals } (locals = number of i32 locals used: 1).
  const { CORE, CBASE, wb, recLoop, sarOff } = ctx;
  const T = TE;
  const P = (reg) => CBASE + 16 + ((((wb << 2) + reg) & 63)) * 4;
  const SPEC3 = CBASE + 336 + 3 * 4, LOOPC = CBASE + 336 + 2 * 4;
  const B = [];
  const cc2 = (v) => B.push(OP.i32_const, ...sleb(v | 0));
  const c = cc2;
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
      // NOTE: exit goes in the THEN (eqz=1 means addr==0); an exit in ELSE
      // would fire on every nonzero address (inverted once, caught by micro).
      ld(P(k.s)); c(k.off); B.push(OP.i32_add, OP.local_tee, 0x00, OP.i32_eqz, OP.if_, 0x40);
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
    else if (k.k === 'l8ui') { // r_handler41: dst = MR8(ar[s]+off)
      c(P(k.t)); c(CORE); ld(P(k.s)); c(k.off); B.push(OP.i32_add, OP.call, 0x00, OP.i32_store, 0x02, 0x00);
    }
    else if (k.k === 'nop') { /* no state change; pc advances by w */ }
    else if (k.k === 'slli') { c(P(k.t)); ld(P(k.s)); c(k.sh); B.push(OP.i32_shl, OP.i32_store, 0x02, 0x00); }
    else if (k.k === 'srai') {
      c(P(k.t)); ld(P(k.s)); c(k.sh); B.push(OP.i32_shr_s, OP.i32_store, 0x02, 0x00);
    }
    else if (k.k === 'addx2') {
      c(P(k.t)); ld(P(k.r)); c(2); B.push(OP.i32_shl); ld(P(k.s)); B.push(OP.i32_add, OP.i32_store, 0x02, 0x00);
    }
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
    else if (k.k === 'br') {
      const br = branches[bj++];
      if (hooks && hooks.onBranch) { hooks.onBranch(B, { cc2, ld, P, OP, sleb }, br, e); return; }
      if (br.b === 'bne' || br.b === 'bltu' || br.b === 'beq') {
        ld(P(br.r)); ld(P(br.s));
        B.push(br.b === 'bne' ? OP.i32_ne : br.b === 'bltu' ? OP.i32_lt_u : OP.i32_eq);
      } else { ld(P(br.r)); B.push(OP.i32_eqz); if (br.b === 'bnez' || br.b === 'bnezn') B.push(OP.i32_eqz); }
      cc2(br.takenRec ? 1 : 0); B.push(OP.i32_eq);
      B.push(OP.if_, 0x40, OP.else_); cc2(100 + branches.indexOf(br)); B.push(OP.return_); B.push(OP.end);
    }
    afterLoop();
  });
  if (!hooks || !hooks.noTail) cc2(0);
  for (const v of B) if (!Number.isInteger(v) || v < 0 || v > 255) throw new Error('bad emitted byte ' + v);
  return { body: B, locals: 1, branches };
}

export function buildTraceModule(T, ctx) {
  const prep = prepareTrace(T, ctx);
  const em = emitOpsBody(prep.TE, prep.kinds, prep.branches, ctx, null);
  const B = em.body;
  const branches = em.branches;
  const endPC = prep.endPC;
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
  for (const v of B) if (!Number.isInteger(v) || v < 0 || v > 255) throw new Error('bad emitted byte ' + v);
  const mod = new Uint8Array([0x00, 0x61, 0x73, 0x6d, 0x01, 0x00, 0x00, 0x00, ...typeSec, ...impSec, ...funcSec, ...expSec, ...codeSec]);
  return { mod, branches, endPC, len: prep.TE.length };
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
export function assemblePageModule(body, { locals = 3, exitGlobal = true } = {}) {
  // Module with the standard 8 mem/call-out imports + shared memory, one
  // global $exitpc (mut i32, exported as 'exitpc'), one export run()->i32.
  const encStr = (s) => [s.length, ...[...s].map((ch) => ch.charCodeAt(0))];
  const typePay = [0x05,
    0x60, 0x02, 0x7f, 0x7f, 0x01, 0x7f,
    0x60, 0x03, 0x7f, 0x7f, 0x7f, 0x01, 0x7f,
    0x60, 0x03, 0x7f, 0x7f, 0x7f, 0x00,
    0x60, 0x02, 0x7f, 0x7f, 0x00,
    0x60, 0x00, 0x01, 0x7f];
  const typeSec = [0x01, ...uleb(typePay.length), ...typePay];
  const impR = (nm) => [...encStr('env'), ...encStr(nm), 0x00, 0x00];
  const impW = (nm) => [...encStr('env'), ...encStr(nm), 0x00, 0x02];
  const impPay = [0x09,
    ...impR('mr8'), ...impR('mr16'), ...impR('mr32'),
    ...impW('mw8'), ...impW('mw16'), ...impW('mw32'),
    ...[...encStr('env'), ...encStr('jit_l32r'), 0x00, 0x01],
    ...[...encStr('env'), ...encStr('jit_ssai'), 0x00, 0x03],
    ...[...encStr('env'), ...encStr('mem'), 0x02, 0x03, ...uleb(1), ...uleb(65536)]];
  const impSec = [0x02, ...uleb(impPay.length), ...impPay];
  const funcSec = [0x03, ...uleb(2), 0x01, 0x04];
  const globSec = [0x06, ...uleb(6), 0x01, 0x7f, 0x01, OP.i32_const, ...sleb(0), OP.end];
  const expPay = [0x02, ...encStr('run'), 0x00, 0x08, ...encStr('exitpc'), 0x03, 0x00];
  const expSec = [0x07, ...uleb(expPay.length), ...expPay];
  const codeBody = [0x01, locals, 0x7f, ...body, OP.end];
  const codeContent = [0x01, ...uleb(codeBody.length), ...codeBody];
  const codeSec = [0x0a, ...uleb(codeContent.length), ...codeContent];
  return new Uint8Array([0x00, 0x61, 0x73, 0x6d, 0x01, 0x00, 0x00, 0x00,
    ...typeSec, ...impSec, ...funcSec, ...globSec, ...expSec, ...codeSec]);
}

export async function instantiatePageModule(mod, L, ex) {
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
  return { run: inst.instance.exports.run, exitpc: inst.instance.exports.exitpc };
}

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

export async function buildPage(Sc, ctx) {
  const { CORE, CBASE, L, ex, yieldOps } = ctx;
  const INSTW = CBASE + 2608;
  const B = [];
  const cc = (v) => B.push(OP.i32_const, ...sleb(v | 0));
  // entry: pc = START, yield = YIELD_OPS
  cc(Sc[0].pc); B.push(OP.local_set, 0x01);
  cc(yieldOps || 19200); B.push(OP.local_set, 0x02);
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
  const prep = prepareTrace(Sc.map((e) => ({ ...e })), ctx);
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
  // (assemblePageModule/instantiatePageModule are module-local below)
  const mod = assemblePageModule(B, { locals: 3 });
  const { run, exitpc } = await instantiatePageModule(mod, L, ex);
  return { run, exitpc, groups: groups.length, ops: prep.TE.length };
}
