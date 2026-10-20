// JIT trace emitter (production, 50mips Phase 2). 84 shapes, routing proven
// by full 24-bit fuzz vs c_handler7 (9.06M/0 misroutes), emission proven per
// shape (73/73 micro suites). Kill switch: callers simply never call.
// RSR/RSRV are engine call-outs (native_jit_rsr/native_jit_rsrv, exact
// _handler14/c_handler6 bodies) — imported at func index 8/9, so the defined
// function (exported as `run`) sits at index 10.
// Context: { CORE, CBASE, wb, recLoop:[begin,end,count] }.
export function uleb(n) { const o = []; do { let b = n & 0x7f; n >>>= 7; if (n) b |= 0x80; o.push(b); } while (n); return o; }
export function sleb(n) { const o = []; let more = true; while (more) { let b = n & 0x7f; n >>= 7; if ((n === 0 && (b & 0x40) === 0) || (n === -1 && (b & 0x40) !== 0)) more = false; else b |= 0x80; o.push(b); } return o; }
export const OP = {
  end: 0x0b, if_: 0x04, else_: 0x05, return_: 0x0f, call: 0x10, br: 0x0c, br_if: 0x0d,
  loop: 0x03, block: 0x02,
  local_get: 0x20, local_set: 0x21,
  i32_const: 0x41, i32_eqz: 0x45, i32_eq: 0x46, i32_ne: 0x47, i32_lt_u: 0x49, i32_lt_s: 0x48, i32_and: 0x71, i32_or: 0x72, i32_xor: 0x73, local_tee: 0x22, i32_extend16: 0xC1, i32_clz: 0x67, i64_const: 0x42, i64_mul: 0x7E, i64_shr_u: 0x88, i32_wrap: 0xA7, i64_extend_u: 0xAD, i64_add: 0x7C, i64_shl: 0x86,
  i32_add: 0x6a, i32_sub: 0x6b, i32_mul: 0x6c, i32_shl: 0x74, i32_shr_u: 0x76, i32_shr_s: 0x75,
  i32_load: 0x28, i32_store: 0x36, f32_load: 0x2A, f32_store: 0x38, f32_add: 0x92, f32_const: 0x43, f32_trunc: 0x8F, i32_rem_s: 0x6F, f32_mul: 0x94, f32_reinterpret: 0xBE, f32_ne: 0x5C, f32_lt: 0x5D, i32_div_u: 0x6E, i32_gt_u: 0x4B, i32_gt_s: 0x4A,
  global_get: 0x23, global_set: 0x24, unreachable_: 0x00, nop_: 0x01, drop: 0x1a,
};
export const sext8 = (v) => (v & 128) ? (v | 0xffffff00) : v;

// ---- op classification (handler-verified) ----
export const NF_SHIFT_TABLE = [-1, 1, 2, 3, 4, 5, 6, 7, 8, 10, 12, 16, 32, 64, 128, 256];
export const NF_TABLE2 = [32768, 65536, 2, 3, 4, 5, 6, 7, 8, 10, 12, 16, 32, 64, 128, 256];
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
  if (lo === 7 && (op & 57344) === 32768 && (op & 4096) === 4096)
    return { k: 'br', b: 'bne', r: f8, s: f4, imm: sext8((op >> 16) & 255) }; // n_handler46 (both encodings)
  if (lo === 7 && (op & 57344) === 40960) { // bge/bgeu n_handler35/37
    const r = f8, ss = f4;
    return { k: 'br', b: ((op & 4096) ? 'bgeu' : 'bge'), r, s: ss, imm: sext8((op >> 16) & 255) };
  }
  if (lo === 7 && (op & 57344) === 8192 && !(op & 4096)) // blt n_handler40
    return { k: 'br', b: 'blt', r: f8, s: f4, imm: sext8((op >> 16) & 255) };
  if (lo === 7 && (op & 57344) === 24576) // bbci n_handler27 (taken iff bit clear)
    return { k: 'br', b: 'bbci', r: f8, s: -1, rr: ((((op >> 12) & 1) << 4) | ((op >> 4) & 15)) >>> 0, imm: sext8((op >> 16) & 255) };
  if (lo === 7 && (op & 57344) === 57344) // bbsi n_handler29 (taken iff bit set)
    return { k: 'br', b: 'bbsi', r: f8, s: -1, rr: ((((op >> 12) & 1) << 4) | ((op >> 4) & 15)) >>> 0, imm: sext8((op >> 16) & 255) };
  if (lo === 7 && (op & 61440) === 49152) // bbr n_handler45 (taken iff (~ar[r])&ar[s] != 0; full-nibble: 0xD sibling is n28)
    return { k: 'br', b: 'bbr', r: f8, s: f4, imm: sext8((op >> 16) & 255) };
  if (lo === 7 && (op & 61440) === 16384) // bbrn n_handler24 (taken iff ((~ar[r])&ar[s])==0; full-nibble 0x4)
    return { k: 'br', b: 'bbrn', r: f8, s: f4, imm: sext8((op >> 16) & 255) };
  if (lo === 7 && (op & 61440) === 20480) // bban n_handler26 (taken iff (ar[r]&(1<<(ar[s]&31)))==0; full-nibble: 0x4 sibling is n24)
    return { k: 'br', b: 'bban', r: f8, s: f4, imm: sext8((op >> 16) & 255) };
  if (lo === 2 && (op & 61440) === 4096) { // l16ui-zero r_handler43 (MR16 zero-extended)
    const t = (op >> 4) & 15, ss = (op >> 8) & 15, off = (((op >> 16) & 255) << 1) >>> 0;
    return { k: 'l16u', t, s: ss, off };
  }
  if (lo === 2 && (op & 61440) === 36864) { // l16ui r_handler42 (sext16 load)
    const t = (op >> 4) & 15, ss = (op >> 8) & 15, off = (((op >> 16) & 255) << 1) >>> 0;
    return { k: 'l16i', t, s: ss, off };
  }
  if (lo === 2 && (op & 61440) === 8192) { // l32i r_handler46 (8-bit offset load)
    const t = (op >> 4) & 15, ss = (op >> 8) & 15, off = (((op >> 16) & 255) << 2) >>> 0;
    return { k: 'l32i8', t, s: ss, off };
  }
  if (lo === 2 && (op & 61440) === 24576) { // s32i _handler20 (8-bit offset store)
    const t = (op >> 4) & 15, ss = (op >> 8) & 15, off = (((op >> 16) & 255) << 2) >>> 0;
    return { k: 's32i8', t, s: ss, off };
  }
  if (lo === 2 && (op & 61440) === 57344) { // s32c1i _handler18 (cond store iff mem==SAR)
    const t = (op >> 4) & 15, ss = (op >> 8) & 15, off = (((op >> 16) & 255) << 2) >>> 0;
    return { k: 's32c1i', t, s: ss, off };
  }
  if (lo === 3 && (op & 61440) === 32768) { // lsiu r_handler58 (f[s]=MR32(ar[r]+off); ar[r]+=off)
    const t = (op >> 4) & 15, ss = (op >> 8) & 15, off = (((op >> 16) & 255) << 2) >>> 0;
    return { k: 'lsiu', t, s: ss, off };
  }
  if (lo === 2 && (op & 61440) === 20480) { // s16i _handler17 (half store, off<<1)
    const t = (op >> 4) & 15, ss = (op >> 8) & 15, off = (((op >> 16) & 255) << 1) >>> 0;
    return { k: 's16i', t, s: ss, off };
  }
  if (lo === 2 && (op & 61440) === 16384) { // s8i _handler16 (byte store, unshifted off)
    const t = (op >> 4) & 15, ss = (op >> 8) & 15, off = ((op >> 16) & 255) >>> 0;
    return { k: 's8i', t, s: ss, off };
  }
  if (lo === 2 && (op & 61440) === 49152) { // addi8 n_handler11
    const idx = (op >> 16) & 255;
    return { k: 'addi8', t: f4, r: f8, imm: sext8(idx) };
  }
  if (lo === 13 && (op & 61440) === 0) // mov a_handler4
    return { k: 'mov', t: f4, s: f8 };
  if (lo === 6 && (op & 48) === 0) { // j r_handler39 (unconditional)
    const idx = (op >> 6) & 262143;
    const off = (idx & 131072) ? (idx | 0xfffc0000) : idx;
    return { k: 'j', off: off | 0 };
  }
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
  if (lo === 6 && (op & 48) === 32 && (op & 192) === 64) // bnetbl n_handler47 (taken iff ar!=TABLE[t], signed)
    return { k: 'br', b: 'bnetbl', r: (op >> 8) & 15, s: -1, t: (op >> 12) & 15, imm: sext8((op >> 16) & 255) };
  if (lo === 6 && (op & 48) === 16 && (op & 192) === 192) { // bgez12 n_handler39 (taken iff ar>=0, 12-bit offset)
    const imm12 = (op >> 12) & 4095, se = (imm12 & 2048) ? (imm12 | 0xfffff000) : imm12;
    return { k: 'br', b: 'bgez12', r: (op >> 8) & 15, s: -1, imm: se };
  }
  if (lo === 6 && (op & 48) === 16 && (op & 192) === 128) { // bltz n_handler44 (taken iff ar<0, 12-bit offset)
    const imm12 = (op >> 12) & 4095, se = (imm12 & 2048) ? (imm12 | 0xfffff000) : imm12;
    return { k: 'br', b: 'bltz', r: (op >> 8) & 15, s: -1, imm: se };
  }
  if (lo === 6 && (op & 48) === 48 && (op & 192) === 192) // bgetbl n_handler38 (taken iff ar>=TABLE2[t] unsigned)
    return { k: 'br', b: 'bgetbl', r: (op >> 8) & 15, s: -1, t: (op >> 12) & 15, imm: sext8((op >> 16) & 255) };
  if (lo === 6 && (op & 48) === 32 && (op & 192) === 192) // bgetbls n_handler36 (taken iff ar>=TABLE[t] SIGNED)
    return { k: 'br', b: 'bgetbls', r: (op >> 8) & 15, s: -1, t: (op >> 12) & 15, imm: sext8((op >> 16) & 255) };
  if (lo === 6 && (op & 48) === 32 && (op & 192) === 0) // beqtbl n_handler31 (taken iff ar==TABLE[t], signed)
    return { k: 'br', b: 'beqtbl', r: (op >> 8) & 15, s: -1, t: (op >> 12) & 15, imm: sext8((op >> 16) & 255) };
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
    // RSR (hi 0x03, lo 0) and RSRV (hi 0x61|0x71, lo 0): exact router masks,
    // both proven by the full-24-bit fuzz. RSR reads an SR into an AR, RSRV
    // swaps them; neither is a pure register move (read_special_register /
    // write_special_register side effects), so both emit call-outs.
    if ((op & 0xFF000F) === 0x030000) {
      const sr = (op >> 8) & 255;
      if (sr === 234) return { k: 'rsrcc', t: f4 }; // CCOUNT: register-file read is its value
      return { k: 'rsr', t: f4, sr };
    }
    if ((op & 0xEF000F) === 0x610000) return { k: 'rsrv', t: f4, sr: (op >> 8) & 255 };
    if (e === 0x20000 && f === 0x800000 && b20 === 0) return { k: 'mull', t: f12, r: f8, s: f4 }; // a_handler37 full mul
    if (e === 0 && f === 0x810000 && b20 === 0) return { k: 'src', t: f12, r: f8, s: f4 }; // _handler37 (formula as coded)
    if (e === 0x40000) { // r_handler26 (formula as coded, NOT objdump name)
      const idx = (op >> 20) & 15, clk = (op >> 16) & 1, rr = (op >> 8) & 15;
      return { k: 'extui', t: f12, s: f4, shift: ((clk << 4) | rr) >>> 0, mask: ((1 << (idx + 1)) - 1) >>> 0 };
    }
    if (e === 0 && (op & 0xE10000) === 0 && b20 !== 0) return { k: 'and', t: f12, r: f8, s: f4 }; // n_handler19
    if (e === 0 && f === 0x200000 && b20 === 0) return { k: 'or', t: f12, r: f8, s: f4 }; // a_handler51
    if (e === 0 && f === 0x410000) return { k: 'srl', t: f12, s: f4, imm: (op >> 8) & 15 }; // _handler39 SRLI: ar[t]=ar[s]>>imm4 (NOT register shift!)
    if (e === 0 && f === 0x200000 && b20 !== 0) return { k: 'xor', t: f12, r: f8, s: f4 }; // c_handler4 (or-split on b20)
    if (e === 0 && f === 0 && b20 === 0 && (op & 61440) === 0 && (op & 240) === 240) return { k: 'callEnd' }; // r_handler7 JX: trace boundary
    if (e === 0 && f === 0 && b20 === 0 && (op & 61440) === 0 && (op & 240) === 144) return { k: 'callEnd' }; // a_handler62 RETQ: trace boundary
    if (lo === 0 && e === 0xA0000 && (op & 0xF10000) === 0) return { k: 'fadd', t: (op >> 12) & 15, r: (op >> 8) & 15, s: (op >> 4) & 15 }; // n_handler10 (f[t] = f[r]+f[s])
    if (lo === 0 && e === 0x20000 && (op & 0xF10000) === 0) return { k: 'band', t: (op >> 12) & 15, r: (op >> 8) & 15, s: (op >> 4) & 15 }; // n_handler20 (br[t] = br[r]&br[s])
    if (lo === 0 && e === 0x20000 && (op & 0xF10000) === 0xC10000) return { k: 'movbr', t: (op >> 12) & 15, r: (op >> 8) & 15, s: (op >> 4) & 15 }; // a_handler8 (if (!br[s]) ar[t]=ar[r])
    if (lo === 0 && (op >> 16) === 0xB0) return { k: 'addx8', t: (op >> 12) & 15, r: (op >> 8) & 15, s: (op >> 4) & 15 }; // n_handler16: (r<<3)+s
    if (lo === 0 && e === 0x20000 && (op & 0xF10000) === 0x710000) return { k: 'maxu', t: (op >> 12) & 15, r: (op >> 8) & 15, s: (op >> 4) & 15 }; // r_handler63: max(r,s) unsigned
    if (lo === 0 && e === 0x20000 && (op & 0xF10000) === 0x510000) return { k: 'maxs', t: (op >> 12) & 15, r: (op >> 8) & 15, s: (op >> 4) & 15 }; // r_handler62: max(r,s) signed
    if (lo === 0 && e === 0x20000 && (op & 0xF10000) === 0xC00000) return { k: 'divu', t: (op >> 12) & 15, r: (op >> 8) & 15, s: (op >> 4) & 15 }; // a_handler57: unsigned div (div0 exits; no overflow case)
    if (lo === 0 && e === 0xA0000 && (op & 0xF10000) === 0x510000) return { k: 'fcmp', t: (op >> 12) & 15, r: (op >> 8) & 15, s: (op >> 4) & 15 }; // _handler59: br[t] = isnan(r)|isnan(s)|(r<s)
    if (lo === 0 && e === 0x20000 && (op & 0xF10000) === 0xF00000) return { k: 'rems', t: (op >> 12) & 15, r: (op >> 8) & 15, s: (op >> 4) & 15 }; // a_handler58: signed rem (div0/overflow exit)
    if (lo === 0 && e === 0x20000 && (op & 0xF10000) === 0x200000) return { k: 'bor', t: (op >> 12) & 15, r: (op >> 8) & 15, s: (op >> 4) & 15 }; // a_handler52: br[t] = br[r]|br[s]
    if (lo === 0 && e === 0x20000 && (op & 0xF10000) === 0x210000) return { k: 'sexpn', t: (op >> 12) & 15, r: (op >> 8) & 15, rr: (((op >> 4) & 15) + 7) >>> 0 }; // _handler26: if bit set, sign-expand
    if (lo === 0 && e === 0xA0000 && (op & 0xF10000) === 0x900000) return { k: 'ftrunc', t: (op >> 12) & 15, r: (op >> 8) & 15, sc: (((127 + ((op >> 4) & 15)) << 23) >>> 0) }; // _handler55: ar[t] = trunc_sat(f[r]*2^s)
    if (lo === 0 && (op >> 16) === 0x60 && (op & 0xF00) === 0) return { k: 'neg', t: (op >> 12) & 15, r: (op >> 4) & 15 }; // a_handler44 (tight: hi+bits8-11)
    if (lo === 0 && (op >> 16) === 0xE0) return { k: 'subx4', t: (op >> 12) & 15, r: (op >> 8) & 15, s: (op >> 4) & 15 }; // _handler52: (r<<2)-s
    if (lo === 0 && (op >> 16) === 0xF0) return { k: 'subx8', t: (op >> 12) & 15, r: (op >> 8) & 15, s: (op >> 4) & 15 }; // _handler53: (r<<3)-s
    if (lo === 0 && e === 0 && (op & 0xE10000) === 0x400000 && (op & 0x10F000) === 0xF000) return { k: 'nsa', t: (op >> 8) & 15, s: (op >> 4) & 15 }; // a_handler47 (== clz, incl. 0->32)
    if (lo === 0 && e === 0 && (op & 0xE10000) === 0x400000 && (op & 0x10F000) === 0x1000) return { k: 'wsar', t: (op >> 8) & 15 }; // _handler45 (SAR <= ar[t]&31)
    if (lo === 0 && (op >> 16) === 0xA1 && (op & 0xF0) === 0) return { k: 'ssl', t: (op >> 12) & 15, r: (op >> 8) & 15 }; // _handler30 (ar[t] = ar[r] << (32-(63&EXC_CAUSE)))
    if (lo === 0 && e === 0x20000 && (op & 0xF10000) === 0x610000) return { k: 'minu', t: (op >> 12) & 15, r: (op >> 8) & 15, s: (op >> 4) & 15 }; // a_handler1: min(r,s) unsigned
    if (lo === 0 && e === 0x20000 && (op & 0xF10000) === 0x910000) return { k: 'movnez', t: (op >> 12) & 15, r: (op >> 8) & 15, s: (op >> 4) & 15 }; // a_handler16
    if (lo === 0 && e === 0x20000 && (op & 0xF10000) === 0xA00000) return { k: 'muluhf', t: (op >> 12) & 15, r: (op >> 8) & 15, s: (op >> 4) & 15 }; // a_handler43 (formula as coded: NOT exact muluh, +0x10000 in ~7%; verified vs bigint)
    if (lo === 0 && e === 0x20000 && (op & 0xF10000) === 0x810000) return { k: 'moveqz', t: (op >> 12) & 15, r: (op >> 8) & 15, s: (op >> 4) & 15 }; // a_handler6
    if (lo === 0 && e === 0 && f === 0x800000 && b20 === 0) return { k: 'addN', t: f12, r: f8, s: f4 }; // n_handler7 (reuse addN emission)
    if (e === 0x80000 && (op & 0xF10000) === 0) return { k: 'fload', t: (op >> 12) & 15, s: (op >> 8) & 15, u: (op >> 4) & 15 }; // r_handler59 float load
    if (e === 0 && f === 0xC00000 && b20 === 0) return { k: 'sub', t: f12, r: f8, s: f4 }; // _handler49
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
  const CALLOUT = ctx.noCallout ? new Set(['l32in', 'l32i8', 's32in', 'l32r', 'l8ui']) : null;
  kinds.forEach((cc, i) => {
    if (cc.k === 'unknown') throw new Error('unemittable op 0x' + T[i].op.toString(16) + ' at ' + i);
    if (CALLOUT && CALLOUT.has(cc.k)) throw new Error('call-out shape gated: ' + cc.k);
  });
  for (const e of T) {
    const lo = e.op & 15;
    if (e.w === 4) throw new Error('wide op needs pie guard (not implemented v1)');
    if (lo === 4 || (lo === 0 && (((e.op >> 16) & 255) === 22 || ((e.op >> 16) & 255) === 23))) throw new Error('pie31-routable narrow op (not implemented v1)');
  }
  const branches = [];
  kinds.forEach((cc, i) => {
    if (cc.k === 'j') {
      const target = (T[i].pc + cc.off + 4) >>> 0;
      if (T[i].next !== target) throw new Error(`jump target mismatch at ${i}`);
    }
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
    else if (k.k === 'neg') { cc2(P(k.t)); ld(P(k.r)); c(-1); B.push(OP.i32_xor); c(1); B.push(OP.i32_add, OP.i32_store, 0x02, 0x00); } // a_handler44: ~r+1
    else if (k.k === 'fadd') { // n_handler10: f[t] = f[r]+f[s] (float file at CBASE+272, NOT windowed)
      cc2(CBASE + 272 + k.t * 4); cc2(CBASE + 272 + k.r * 4); B.push(OP.f32_load, 0x02, 0x00);
      cc2(CBASE + 272 + k.s * 4); B.push(OP.f32_load, 0x02, 0x00);
      B.push(OP.f32_add, OP.f32_store, 0x02, 0x00);
    }
    else if (k.k === 'band') { // n_handler20: PS.bit[t] = PS.bit[r]&PS.bit[s]
      // NOTE (micro33): the final or/and MUST combine the masked VALUE, not
      // the address: [A,A,nb]+or stores A|nb (address leak!). Recompute the
      // masked value as its own stack item: [A, masked, nb]+or.
      cc2(CBASE + 336 + 16);
      cc2(CBASE + 336 + 16); B.push(OP.i32_load, 0x02, 0x00);
      c(1 << k.t); c(-1); B.push(OP.i32_xor, OP.i32_and);
      cc2(CBASE + 336 + 16); B.push(OP.i32_load, 0x02, 0x00);
      c(k.r); B.push(OP.i32_shr_u); c(1); B.push(OP.i32_and);
      cc2(CBASE + 336 + 16); B.push(OP.i32_load, 0x02, 0x00);
      c(k.s); B.push(OP.i32_shr_u); c(1); B.push(OP.i32_and, OP.i32_and);
      c(k.t); B.push(OP.i32_shl, OP.i32_or, OP.i32_store, 0x02, 0x00);
    }
    else if (k.k === 'movbr') { // a_handler8: if (!br[s]) ar[t]=ar[r]
      cc2(CBASE + 336 + 16); B.push(OP.i32_load, 0x02, 0x00);
      c(k.s); B.push(OP.i32_shr_u); c(1); B.push(OP.i32_and, OP.i32_eqz, OP.if_, 0x40);
      cc2(P(k.t)); ld(P(k.r)); B.push(OP.i32_store, 0x02, 0x00, OP.end);
    }
    else if (k.k === 'addx8') { // n_handler16: ar[t] = (ar[r]<<3)+ar[s]
      cc2(P(k.t)); ld(P(k.r)); c(3); B.push(OP.i32_shl); ld(P(k.s)); B.push(OP.i32_add, OP.i32_store, 0x02, 0x00);
    }
    else if (k.k === 'lsiu') { // r_handler58: f[t] = MR32(ar[s]+off); ar[s] += off (float dst k.t, base k.s)
      ld(P(k.s)); c(k.off); B.push(OP.i32_add, OP.local_tee, 0x00, OP.i32_eqz, OP.if_, 0x40);
      branches.push({ idx: i, pc: e.pc }); c(100 + branches.length - 1);
      if (hooks) { cc(e.pc); B.push(OP.global_set, 0x00); } // page mode: driver parks via exitpc
      B.push(OP.return_, OP.end);
      cc2(CBASE + 272 + k.t * 4); c(CORE); B.push(OP.local_get, 0x00, OP.call, 0x02, OP.f32_reinterpret, OP.f32_store, 0x02, 0x00);
      cc2(P(k.s)); B.push(OP.local_get, 0x00, OP.i32_store, 0x02, 0x00);
    }
    else if (k.k === 'rems') { // a_handler58: ar[t] = ar[r] % ar[s] signed; div0 AND INT_MIN%-1 exit (WASM rem_s traps, Rust wraps)
      ld(P(k.s)); B.push(OP.i32_eqz);
      ld(P(k.r)); c(-2147483648); B.push(OP.i32_eq);
      ld(P(k.s)); c(-1); B.push(OP.i32_eq, OP.i32_and, OP.i32_or, OP.if_, 0x40);
      branches.push({ idx: i, pc: e.pc }); c(100 + branches.length - 1);
      if (hooks) { cc(e.pc); B.push(OP.global_set, 0x00); }
      B.push(OP.return_, OP.end);
      cc2(P(k.t)); ld(P(k.r)); ld(P(k.s)); B.push(OP.i32_rem_s, OP.i32_store, 0x02, 0x00);
    }
    else if (k.k === 'bor') { // a_handler52: PS.bit[t] = PS.bit[r]|PS.bit[s] (same addr/value discipline as band)
      cc2(CBASE + 336 + 16);
      cc2(CBASE + 336 + 16); B.push(OP.i32_load, 0x02, 0x00);
      c(1 << k.t); c(-1); B.push(OP.i32_xor, OP.i32_and);
      cc2(CBASE + 336 + 16); B.push(OP.i32_load, 0x02, 0x00);
      c(k.r); B.push(OP.i32_shr_u); c(1); B.push(OP.i32_and);
      cc2(CBASE + 336 + 16); B.push(OP.i32_load, 0x02, 0x00);
      c(k.s); B.push(OP.i32_shr_u); c(1); B.push(OP.i32_and, OP.i32_or);
      c(k.t); B.push(OP.i32_shl, OP.i32_or, OP.i32_store, 0x02, 0x00);
    }
    else if (k.k === 'sexpn') { // _handler26: ar[t] = bit? wa|mask : wa&~mask (ALWAYS writes; ~mask=(1<<rr)-1 at classify time)
      ld(P(k.r)); B.push(OP.local_tee, 0x00); c(k.rr); B.push(OP.i32_shr_u); c(1); B.push(OP.i32_and, OP.if_, 0x40);
      cc2(P(k.t)); B.push(OP.local_get, 0x00); c(-1); c(k.rr); B.push(OP.i32_shl, OP.i32_or, OP.i32_store, 0x02, 0x00, OP.else_);
      cc2(P(k.t)); B.push(OP.local_get, 0x00); c((1 << k.rr) - 1); B.push(OP.i32_and, OP.i32_store, 0x02, 0x00, OP.end);
    }
    else if (k.k === 'ftrunc') { // _handler55: ar[t] = trunc_sat_u(f[r]*2^s) (Rust `as` saturates, like 0xFC ops)
      cc2(P(k.t)); cc2(CBASE + 272 + k.r * 4); B.push(OP.f32_load, 0x02, 0x00);
      B.push(OP.f32_const); for (const byte of new Uint8Array(new Uint32Array([k.sc]).buffer)) B.push(byte);
      B.push(OP.f32_mul, OP.f32_trunc, 0xFC, 0x01, OP.i32_store, 0x02, 0x00); // sat_u subop is 0x01 (0x02 is f64 - caught by micro36)
    }
    else if (k.k === 'maxu' || k.k === 'maxs') { // r_handler63/62: ar[t] = max(ar[r],ar[s]) (un)signed
      ld(P(k.r)); ld(P(k.s)); B.push(k.k === 'maxu' ? OP.i32_gt_u : OP.i32_gt_s, OP.if_, 0x40);
      cc2(P(k.t)); ld(P(k.r)); B.push(OP.i32_store, 0x02, 0x00, OP.else_);
      cc2(P(k.t)); ld(P(k.s)); B.push(OP.i32_store, 0x02, 0x00, OP.end);
    }
    else if (k.k === 'divu') { // a_handler57: ar[t] = ar[r]/ar[s] unsigned; div0 exits (WASM div_u traps like rem_s)
      ld(P(k.s)); B.push(OP.i32_eqz, OP.if_, 0x40);
      branches.push({ idx: i, pc: e.pc }); c(100 + branches.length - 1);
      if (hooks) { cc(e.pc); B.push(OP.global_set, 0x00); }
      B.push(OP.return_, OP.end);
      cc2(P(k.t)); ld(P(k.r)); ld(P(k.s)); B.push(OP.i32_div_u, OP.i32_store, 0x02, 0x00);
    }
    else if (k.k === 'fcmp') { // _handler59: br[t] = isnan(r)|isnan(s)|(r<s); newbit is i32 so local0 tee is legal
      cc2(CBASE + 336 + 16); cc2(CBASE + 336 + 16); B.push(OP.i32_load, 0x02, 0x00);
      c(1 << k.t); c(-1); B.push(OP.i32_xor, OP.i32_and, OP.i32_store, 0x02, 0x00);
      cc2(CBASE + 272 + k.r * 4); B.push(OP.f32_load, 0x02, 0x00);
      cc2(CBASE + 272 + k.r * 4); B.push(OP.f32_load, 0x02, 0x00);
      B.push(OP.f32_ne);
      cc2(CBASE + 272 + k.s * 4); B.push(OP.f32_load, 0x02, 0x00);
      cc2(CBASE + 272 + k.s * 4); B.push(OP.f32_load, 0x02, 0x00);
      B.push(OP.f32_ne);
      cc2(CBASE + 272 + k.r * 4); B.push(OP.f32_load, 0x02, 0x00);
      cc2(CBASE + 272 + k.s * 4); B.push(OP.f32_load, 0x02, 0x00);
      B.push(OP.f32_lt, OP.i32_or, OP.i32_or, OP.local_tee, 0x00, OP.drop);
      cc2(CBASE + 336 + 16); cc2(CBASE + 336 + 16); B.push(OP.i32_load, 0x02, 0x00);
      B.push(OP.local_get, 0x00); c(k.t); B.push(OP.i32_shl, OP.i32_or, OP.i32_store, 0x02, 0x00);
    }
    else if (k.k === 'minu') { // a_handler1: ar[t] = min(ar[r],ar[s]) unsigned
      ld(P(k.r)); ld(P(k.s)); B.push(OP.i32_lt_u, OP.if_, 0x40);
      cc2(P(k.t)); ld(P(k.r)); B.push(OP.i32_store, 0x02, 0x00, OP.else_);
      cc2(P(k.t)); ld(P(k.s)); B.push(OP.i32_store, 0x02, 0x00, OP.end);
    }
    else if (k.k === 'subx4' || k.k === 'subx8') { // _handler52/53: ar[t] = (ar[r]<<N)-ar[s] (sub order: deeper-top, natural here)
      cc2(P(k.t)); ld(P(k.r)); c(k.k === 'subx4' ? 2 : 3); B.push(OP.i32_shl); ld(P(k.s)); B.push(OP.i32_sub, OP.i32_store, 0x02, 0x00);
    }
    else if (k.k === 'nsa') { cc2(P(k.t)); ld(P(k.s)); B.push(OP.i32_clz, OP.i32_store, 0x02, 0x00); } // a_handler47: NSA == clz
    else if (k.k === 'wsar') { // _handler45: ce=31&ar[t]; EXC_CAUSE=32-ce; sar_m32=ce; pending=1
      if (sarOff == null) throw new Error('unemittable op 0x' + e.op.toString(16) + ' (wsar needs sarOff)');
      cc2(CBASE + 336 + 12); c(32); ld(P(k.t)); c(31); B.push(OP.i32_and, OP.i32_sub, OP.i32_store, 0x02, 0x00);
      cc2(CBASE + sarOff - 4); ld(P(k.t)); c(31); B.push(OP.i32_and, OP.i32_store, 0x02, 0x00);
      cc2(CBASE + sarOff); c(1); B.push(OP.i32_store, 0x02, 0x00);
    }
    else if (k.k === 'ssl') { // _handler30: ar[t] = ar[r] << (32-(63&EXC_CAUSE))
      // NOTE: i32.sub takes (deeper)-(top), and local.tee KEEPS its value, so
      // tee-then-get leaves a stale extra copy (micro20: stack-height 2 at
      // end + store to a garbage address). Push 32 BEFORE loading m instead.
      cc2(P(k.t)); ld(P(k.r)); c(32); cc2(CBASE + 336 + 12); B.push(OP.i32_load, 0x02, 0x00);
      c(63); B.push(OP.i32_and, OP.i32_sub, OP.i32_shl, OP.i32_store, 0x02, 0x00);
    }
    else if (k.k === 'muluhf') { // a_handler43, formula as coded: rt*cfg + (sd>>16) + (sd>0xFFFFFFFF ? 65536 : 0)
      // No i64 locals exist (all locals i32), so sd is computed twice from
      // reloads rather than tee'd (loads are plain shared-mem, no call-out).
      const sdSeq = () => {
        ld(P(k.r)); c(16); B.push(OP.i32_shr_u, OP.i64_extend_u);
        ld(P(k.s)); c(0xFFFF); B.push(OP.i32_and, OP.i64_extend_u);
        B.push(OP.i64_mul);
        ld(P(k.r)); c(0xFFFF); B.push(OP.i32_and, OP.i64_extend_u);
        ld(P(k.s)); c(16); B.push(OP.i32_shr_u, OP.i64_extend_u);
        B.push(OP.i64_mul, OP.i64_add);
        ld(P(k.r)); c(0xFFFF); B.push(OP.i32_and, OP.i64_extend_u);
        ld(P(k.s)); c(0xFFFF); B.push(OP.i32_and, OP.i64_extend_u);
        B.push(OP.i64_mul, OP.i64_const, ...sleb(16), OP.i64_shr_u, OP.i64_add);
      };
      cc2(P(k.t));
      ld(P(k.r)); c(16); B.push(OP.i32_shr_u, OP.i64_extend_u);
      ld(P(k.s)); c(16); B.push(OP.i32_shr_u, OP.i64_extend_u);
      B.push(OP.i64_mul); // A = rt*cfg
      sdSeq(); B.push(OP.i64_const, ...sleb(16), OP.i64_shr_u, OP.i64_add); // A + (sd>>16)
      sdSeq(); B.push(OP.i64_const, ...sleb(32), OP.i64_shr_u, OP.i64_const, ...sleb(16), OP.i64_shl, OP.i64_add); // + it
      B.push(OP.i32_wrap, OP.i32_store, 0x02, 0x00);
    }
    else if (k.k === 'moviN' || k.k === 'movi') { cc2(P(k.t)); cc2(k.imm); B.push(OP.i32_store, 0x02, 0x00); }
    else if (k.k === 'addmi') { cc2(P(k.t)); ld(P(k.r)); cc2(k.imm); B.push(OP.i32_add, OP.i32_store, 0x02, 0x00); }
    else if (k.k === 'addi8') { cc2(P(k.t)); ld(P(k.r)); cc2(k.imm); B.push(OP.i32_add, OP.i32_store, 0x02, 0x00); }
    else if (k.k === 'mov') { cc2(P(k.t)); ld(P(k.s)); B.push(OP.i32_store, 0x02, 0x00); }
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
    else if (k.k === 'fload') { // r_handler59: f[t] = MR32(ar[s]+ar[u]) (float dest NOT windowed)
      c(CBASE + 272 + k.t * 4); c(CORE); ld(P(k.s)); ld(P(k.u)); B.push(OP.i32_add, OP.call, 0x02, OP.i32_store, 0x02, 0x00);
    }
    else if (k.k === 'l16u') { // r_handler43: dst = MR16(ar[s]+off) (WASM i32 return already zero-extends)
      c(P(k.t)); c(CORE); ld(P(k.s)); c(k.off); B.push(OP.i32_add, OP.call, 0x01, OP.i32_store, 0x02, 0x00);
    }
    else if (k.k === 'l16i') { // r_handler42: dst = SEXT16(MR16(ar[s]+off))
      c(P(k.t)); c(CORE); ld(P(k.s)); c(k.off); B.push(OP.i32_add, OP.call, 0x01, OP.i32_extend16, OP.i32_store, 0x02, 0x00);
    }
    else if (k.k === 'l32in' || k.k === 'l32i8') { // r_handler47/46: dst = MR32(ar[s]+off)
      c(P(k.t)); c(CORE); ld(P(k.s)); c(k.off); B.push(OP.i32_add, OP.call, 0x02, OP.i32_store, 0x02, 0x00);
    }
    else if (k.k === 's32in') { // _handler21: MW32(ar[s]+off, ar[t]); addr 0 traps -> side exit
      // NOTE: exit goes in the THEN (eqz=1 means addr==0); an exit in ELSE
      // would fire on every nonzero address (inverted once, caught by micro).
      ld(P(k.s)); c(k.off); B.push(OP.i32_add, OP.local_tee, 0x00, OP.i32_eqz, OP.if_, 0x40);
      branches.push({ idx: i, pc: e.pc }); c(100 + branches.length - 1);
      if (hooks) { cc(e.pc); B.push(OP.global_set, 0x00); } // page mode: driver parks via exitpc
      B.push(OP.return_, OP.end);
      c(CORE); B.push(OP.local_get, 0x00); ld(P(k.t)); B.push(OP.call, 0x05);
    }
    else if (k.k === 's32i8') { // _handler20: MW32(ar[s]+off8, ar[t]); addr 0 traps -> side exit
      ld(P(k.s)); c(k.off); B.push(OP.i32_add, OP.local_tee, 0x00, OP.i32_eqz, OP.if_, 0x40);
      branches.push({ idx: i, pc: e.pc }); c(100 + branches.length - 1);
      if (hooks) { cc(e.pc); B.push(OP.global_set, 0x00); } // page mode: driver parks via exitpc
      B.push(OP.return_, OP.end);
      c(CORE); B.push(OP.local_get, 0x00); ld(P(k.t)); B.push(OP.call, 0x05);
    }
    else if (k.k === 'movnez') { // a_handler16: if (ar[s]!=0) ar[t]=ar[r]
      // NOTE: addr+val pushes go INSIDE the if: a void if-block must end
      // with the same stack height it entered with (caught by micro10).
      ld(P(k.s)); B.push(OP.i32_eqz, OP.i32_eqz, OP.if_, 0x40);
      c(P(k.t)); ld(P(k.r)); B.push(OP.i32_store, 0x02, 0x00); B.push(OP.end);
    }
    else if (k.k === 'moveqz') { // a_handler6: if (ar[s]==0) ar[t]=ar[r]
      ld(P(k.s)); B.push(OP.i32_eqz, OP.if_, 0x40);
      c(P(k.t)); ld(P(k.r)); B.push(OP.i32_store, 0x02, 0x00); B.push(OP.end);
    }
    else if (k.k === 's32c1i') { // _handler18: wa=MR32(ar[s]+off); if (wa!=SAR) ar[t]=wa; else MW32(ar[s]+off, ar[t])
      // Write-fault sub-branch assumed unreachable (same assumption as all
      // store shapes: only addr-0 traps, handled by the side exit below).
      ld(P(k.s)); c(k.off); B.push(OP.i32_add, OP.local_tee, 0x00, OP.i32_eqz, OP.if_, 0x40);
      branches.push({ idx: i, pc: e.pc }); c(100 + branches.length - 1);
      if (hooks) { cc(e.pc); B.push(OP.global_set, 0x00); } // page mode: driver parks via exitpc
      B.push(OP.return_, OP.end);
      c(CORE); B.push(OP.local_get, 0x00, OP.call, 0x02, OP.local_tee, 0x00);
      cc2(CBASE + 336 + 48); B.push(OP.i32_load, 0x02, 0x00, OP.i32_ne, OP.if_, 0x40);
      c(P(k.t)); B.push(OP.local_get, 0x00, OP.i32_store, 0x02, 0x00, OP.else_);
      c(CORE); ld(P(k.s)); c(k.off); B.push(OP.i32_add); ld(P(k.t)); B.push(OP.call, 0x05, OP.end);
    }
    else if (k.k === 's16i') { // _handler17: MW16(ar[s]+off, ar[t]); addr 0 traps -> side exit
      ld(P(k.s)); c(k.off); B.push(OP.i32_add, OP.local_tee, 0x00, OP.i32_eqz, OP.if_, 0x40);
      branches.push({ idx: i, pc: e.pc }); c(100 + branches.length - 1);
      if (hooks) { cc(e.pc); B.push(OP.global_set, 0x00); } // page mode: driver parks via exitpc
      B.push(OP.return_, OP.end);
      c(CORE); B.push(OP.local_get, 0x00); ld(P(k.t)); B.push(OP.call, 0x04);
    }
    else if (k.k === 's8i') { // _handler16: MW8(ar[s]+off, ar[t]); addr 0 traps -> side exit
      ld(P(k.s)); c(k.off); B.push(OP.i32_add, OP.local_tee, 0x00, OP.i32_eqz, OP.if_, 0x40);
      branches.push({ idx: i, pc: e.pc }); c(100 + branches.length - 1);
      if (hooks) { cc(e.pc); B.push(OP.global_set, 0x00); } // page mode: driver parks via exitpc
      B.push(OP.return_, OP.end);
      c(CORE); B.push(OP.local_get, 0x00); ld(P(k.t)); B.push(OP.call, 0x03);
    }
    else if (k.k === 'extui') { // r_handler26 formula as coded
      c(P(k.t)); ld(P(k.s)); c(k.shift); B.push(OP.i32_shr_u); c(k.mask); B.push(OP.i32_and, OP.i32_store, 0x02, 0x00);
    }
    else if (k.k === 'and') { c(P(k.t)); ld(P(k.r)); ld(P(k.s)); B.push(OP.i32_and, OP.i32_store, 0x02, 0x00); }
    else if (k.k === 'or') { c(P(k.t)); ld(P(k.r)); ld(P(k.s)); B.push(0x72, OP.i32_store, 0x02, 0x00); }
    else if (k.k === 'sub') { c(P(k.t)); ld(P(k.r)); ld(P(k.s)); B.push(OP.i32_sub, OP.i32_store, 0x02, 0x00); }
    else if (k.k === 'srl') { c(P(k.t)); ld(P(k.s)); c(k.imm); B.push(OP.i32_shr_u, OP.i32_store, 0x02, 0x00); }
    else if (k.k === 'xor') { c(P(k.t)); ld(P(k.r)); ld(P(k.s)); B.push(0x73, OP.i32_store, 0x02, 0x00); }
    else if (k.k === 'rsrcc') { c(P(k.t)); c(CBASE + 336 + 234 * 4); B.push(OP.i32_load, 0x02, 0x00, OP.i32_store, 0x02, 0x00); }
    else if (k.k === 'rsr') { cc2(CORE); cc2(e.op); B.push(OP.call, 0x08); } // _handler14 call-out
    else if (k.k === 'rsrv') { cc2(CORE); cc2(e.op); B.push(OP.call, 0x09); } // c_handler6 call-out
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
    else if (k.k === 'l32r') {
      if (e.next === null) throw new Error('l32r as last trace op (no next pc)');
      // Constant-fold when the literal sits in flash (code windows
      // 0x400C0000-0x40400000): value can't change without a flash write,
      // which bumps the already-guarded region). RAM/DROM literals keep
      // the call-out (RAM is mutable under the trace; DROM has its own
      // window semantics).
      const idx = (e.op >> 8) & 65535;
      const litAddr = (((e.next >>> 2) + (0xffff0000 | idx)) << 2) >>> 0;
      const inFlash = litAddr >= 0x400c0000 && litAddr < 0x40400000;
      if (ctx.readMem && !ctx.noFold && inFlash) {
        const val = ctx.readMem(litAddr) >>> 0;
        c(P(k.t)); c(val); B.push(OP.i32_store, 0x02, 0x00);
        if (ctx.folded) ctx.folded.push(litAddr >>> 0);
      } else {
        cc2(P(k.t)); cc2(CORE); cc2(e.op); cc2(e.next); B.push(OP.call, 0x06, OP.i32_store, 0x02, 0x00);
      }
    }
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
      } else if (br.b === 'bge') {
        ld(P(br.r)); ld(P(br.s)); B.push(OP.i32_lt_s);
        B.push(OP.i32_eqz);
      } else if (br.b === 'bgeu') {
        ld(P(br.r)); ld(P(br.s)); B.push(OP.i32_lt_u);
        B.push(OP.i32_eqz);
      } else if (br.b === 'blt') {
        ld(P(br.r)); ld(P(br.s)); B.push(OP.i32_lt_s);
      } else if (br.b === 'bbci') {
        ld(P(br.r)); c(br.rr); B.push(OP.i32_shr_u); c(1); B.push(OP.i32_and, OP.i32_eqz);
      } else if (br.b === 'beqtbl') {
        ld(P(br.r)); c(NF_SHIFT_TABLE[br.t]); B.push(OP.i32_eq);
      } else if (br.b === 'bbr') {
        ld(P(br.r)); c(-1); B.push(OP.i32_xor); ld(P(br.s)); B.push(OP.i32_and, OP.i32_eqz, OP.i32_eqz);
      } else if (br.b === 'bbrn') {
        ld(P(br.r)); c(-1); B.push(OP.i32_xor); ld(P(br.s)); B.push(OP.i32_and, OP.i32_eqz);
      } else if (br.b === 'bltz') {
        ld(P(br.r)); c(0); B.push(OP.i32_lt_s);
      } else if (br.b === 'bgez12') {
        ld(P(br.r)); c(0); B.push(OP.i32_lt_s, OP.i32_eqz);
      } else if (br.b === 'bnetbl') {
        ld(P(br.r)); c(NF_SHIFT_TABLE[br.t]); B.push(OP.i32_ne);
      } else if (br.b === 'bbsi') {
        ld(P(br.r)); c(br.rr); B.push(OP.i32_shr_u); c(1); B.push(OP.i32_and);
      } else if (br.b === 'bgetbl') {
        ld(P(br.r)); c(NF_TABLE2[br.t]); B.push(OP.i32_lt_u, OP.i32_eqz);
      } else if (br.b === 'bgetbls') {
        ld(P(br.r)); c(NF_SHIFT_TABLE[br.t]); B.push(OP.i32_lt_s, OP.i32_eqz);
      } else if (br.b === 'bban') {
        c(1); ld(P(br.s)); c(31); B.push(OP.i32_and, OP.i32_shl); ld(P(br.r)); B.push(OP.i32_and, OP.i32_eqz);
      } else { ld(P(br.r)); B.push(OP.i32_eqz); if (br.b === 'bnez' || br.b === 'bnezn') B.push(OP.i32_eqz); }
      cc2(br.takenRec ? 1 : 0); B.push(OP.i32_eq);
      B.push(OP.if_, 0x40, OP.else_); cc2(100 + branches.indexOf(br)); B.push(OP.return_); B.push(OP.end);
    }
    else if (k.k === 'j') {
      // trace mode: nothing (pc is driver fiction; endPC lands correctly).
      // page mode: must thread explicitly (not yet implemented -> fail loud).
      if (hooks) {
        const t = (e.pc + k.off + 4) >>> 0;
        if (t !== e.next) throw new Error('page-mode j needs threading (target 0x' + t.toString(16) + ')');
      }
    }
    else { throw new Error('no emission for shape ' + k.k + ' (op 0x' + e.op.toString(16) + ')'); }
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
  const impPay = [0x0b,
    ...impR('mr8'), ...impR('mr16'), ...impR('mr32'),
    ...impW('mw8'), ...impW('mw16'), ...impW('mw32'),
    ...[...encStr('env'), ...encStr('jit_l32r'), 0x00, 0x01],
    ...[...encStr('env'), ...encStr('jit_ssai'), 0x00, 0x03],
    ...[...encStr('env'), ...encStr('mem'), 0x02, 0x03, ...uleb(1), ...uleb(65536)],
    ...[...encStr('env'), ...encStr('jit_rsr'), 0x00, 0x03],
    ...[...encStr('env'), ...encStr('jit_rsrv'), 0x00, 0x03]];
  const impSec = [0x02, ...uleb(impPay.length), ...impPay];
  const funcSec = [0x03, ...uleb(2), 0x01, 0x04];
  const expPay = [0x01, ...encStr('run'), 0x00, 0x0a]; // 10 func imports (mem is a memory)
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
      jit_rsr: (core, op) => ex.native_jit_rsr(core, op),
      jit_rsrv: (core, op) => ex.native_jit_rsrv(core, op),
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
  const impPay = [0x0b,
    ...impR('mr8'), ...impR('mr16'), ...impR('mr32'),
    ...impW('mw8'), ...impW('mw16'), ...impW('mw32'),
    ...[...encStr('env'), ...encStr('jit_l32r'), 0x00, 0x01],
    ...[...encStr('env'), ...encStr('jit_ssai'), 0x00, 0x03],
    ...[...encStr('env'), ...encStr('mem'), 0x02, 0x03, ...uleb(1), ...uleb(65536)],
    ...[...encStr('env'), ...encStr('jit_rsr'), 0x00, 0x03],
    ...[...encStr('env'), ...encStr('jit_rsrv'), 0x00, 0x03]];
  const impSec = [0x02, ...uleb(impPay.length), ...impPay];
  const funcSec = [0x03, ...uleb(2), 0x01, 0x04];
  const globSec = [0x06, ...uleb(6), 0x01, 0x7f, 0x01, OP.i32_const, ...sleb(0), OP.end];
  const expPay = [0x02, ...encStr('run'), 0x00, 0x0a, ...encStr('exitpc'), 0x03, 0x00]; // 10 func imports
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
      jit_rsr: (core, op) => ex.native_jit_rsr(core, op),
      jit_rsrv: (core, op) => ex.native_jit_rsrv(core, op),
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
        // Exact accounting: only the executed prefix (through this branch),
        // NOT the full group (unexecuted suffix would inflate instTotal).
        const prefixLen = subT.indexOf(e) + 1;
        if (br.b === 'bne' || br.b === 'bltu' || br.b === 'beq') {
          H.ld(H.P(br.r)); H.ld(H.P(br.s));
          BB.push(br.b === 'bne' ? H.OP.i32_ne : br.b === 'bltu' ? H.OP.i32_lt_u : H.OP.i32_eq);
        } else if (br.b === 'bge') {
          H.ld(H.P(br.r)); H.ld(H.P(br.s)); BB.push(H.OP.i32_lt_s);
          BB.push(H.OP.i32_eqz);
        } else if (br.b === 'bgeu') {
          H.ld(H.P(br.r)); H.ld(H.P(br.s)); BB.push(H.OP.i32_lt_u);
          BB.push(H.OP.i32_eqz);
        } else if (br.b === 'blt') {
          H.ld(H.P(br.r)); H.ld(H.P(br.s)); BB.push(H.OP.i32_lt_s);
        } else if (br.b === 'bbci') {
          H.ld(H.P(br.r)); H.cc2(br.rr); BB.push(H.OP.i32_shr_u); H.cc2(1); BB.push(H.OP.i32_and, H.OP.i32_eqz);
        } else if (br.b === 'beqtbl') {
          H.ld(H.P(br.r)); H.cc2(NF_SHIFT_TABLE[br.t]); BB.push(H.OP.i32_eq);
        } else if (br.b === 'bnetbl') {
          H.ld(H.P(br.r)); H.cc2(NF_SHIFT_TABLE[br.t]); BB.push(H.OP.i32_ne);
        } else if (br.b === 'bbsi') {
          H.ld(H.P(br.r)); H.cc2(br.rr); BB.push(H.OP.i32_shr_u); H.cc2(1); BB.push(H.OP.i32_and);
        } else if (br.b === 'bbr') {
          H.ld(H.P(br.r)); H.cc2(-1); BB.push(H.OP.i32_xor); H.ld(H.P(br.s)); BB.push(H.OP.i32_and, H.OP.i32_eqz, H.OP.i32_eqz);
        } else if (br.b === 'bbrn') {
          H.ld(H.P(br.r)); H.cc2(-1); BB.push(H.OP.i32_xor); H.ld(H.P(br.s)); BB.push(H.OP.i32_and, H.OP.i32_eqz);
        } else if (br.b === 'bltz') {
          H.ld(H.P(br.r)); H.cc2(0); BB.push(H.OP.i32_lt_s);
        } else if (br.b === 'bgez12') {
          H.ld(H.P(br.r)); H.cc2(0); BB.push(H.OP.i32_lt_s, H.OP.i32_eqz);
        } else if (br.b === 'bgetbl') {
          H.ld(H.P(br.r)); H.cc2(NF_TABLE2[br.t]); BB.push(H.OP.i32_lt_u);
          BB.push(H.OP.i32_eqz);
        } else if (br.b === 'bgetbls') {
          H.ld(H.P(br.r)); H.cc2(NF_SHIFT_TABLE[br.t]); BB.push(H.OP.i32_lt_s);
          BB.push(H.OP.i32_eqz);
        } else if (br.b === 'bban') {
          H.cc2(1); H.ld(H.P(br.s)); H.cc2(31); BB.push(H.OP.i32_and, H.OP.i32_shl); H.ld(H.P(br.r)); BB.push(H.OP.i32_and, H.OP.i32_eqz);
        } else { H.ld(H.P(br.r)); BB.push(H.OP.i32_eqz); if (br.b === 'bnez' || br.b === 'bnezn') BB.push(H.OP.i32_eqz); }
        BB.push(H.OP.if_, 0x40);
        // taken path: inst-add then transfer
        BB.push(H.OP.i32_const, ...H.sleb(INSTW), H.OP.i32_const, ...H.sleb(INSTW));
        BB.push(H.OP.i32_load, 0x02, 0x00, H.OP.i32_const, ...H.sleb(prefixLen), H.OP.i32_add, H.OP.i32_store, 0x02, 0x00);
        if (inPage(takenPc)) { BB.push(H.OP.i32_const, ...H.sleb(takenPc), H.OP.local_set, 0x01, H.OP.br, 0x02); }
        else { BB.push(H.OP.i32_const, ...H.sleb(takenPc), H.OP.global_set, 0x00, H.OP.i32_const, ...H.sleb(1), H.OP.return_); }
        BB.push(H.OP.else_);
        BB.push(H.OP.i32_const, ...H.sleb(INSTW), H.OP.i32_const, ...H.sleb(INSTW));
        BB.push(H.OP.i32_load, 0x02, 0x00, H.OP.i32_const, ...H.sleb(prefixLen), H.OP.i32_add, H.OP.i32_store, 0x02, 0x00);
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
