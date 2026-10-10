// JIT trace driver (production, 50mips Phase 2). Per-core independent
// trace execution with interpreter continuation.
//
// Model: each busy step, per core, try a cached compiled trace (pc + guard).
// Hit (completed or side-exit): exact accounting (pc, inst_count), then the
// step's normal core_run continues BOTH cores from the advanced state.
// Miss/uncompiled: plain interpreter step (zero behavior change).
// Interleaving skew from independent per-core traces is bounded by trace
// length (<=256 ops), same order as the existing 512-batch skew; UART-trace
// equivalence across the worker battery is the standing gate.
// Kill switch: enabled=false (or JIT=0) disables everything; the
// interpreter covers all code paths by construction.
import { classify, buildTraceModule, instantiateTrace } from './jit-trace-emit.js';

export class JitDriver {
  constructor(chip, loader, opts = {}) {
    this.chip = chip;
    this.loader = loader;
    this.ex = loader.exports;
    this.enabled = opts.enabled !== false && !process.env.JIT?.startsWith?.('0');
    this.cache = new Map(); // `${core}:${pc}` -> entry
    this.uncompilable = new Set();
    this.maxEntries = opts.maxEntries || 128;
    this.minLen = opts.minLen || 24;
    this.stats = { hits: 0, completed: 0, exits: 0, misses: 0, compiles: 0, evictions: 0, gGen: 0, gPend: 0, gSpec: 0, invalidated: 0, findNull: 0, buildFail: 0, instFail: 0, firstErr: '', badOps: {} };
    const L = loader;
    // Memory never grows (maximum == initial), so the buffer is stable;
    // still re-check identity defensively (loader documents detach risk).
    this._buf = null;
    this._u32 = null;
    this.memU32 = () => {
      const b = L.memory.buffer;
      if (b !== this._buf) { this._buf = b; this._u32 = new Uint32Array(b); }
      return this._u32;
    };
    this.CORE_SZ = 4096;
    this.sarOff = null;
    try {
      if (typeof this.ex.native_sar_pending_off === 'function') {
        this.sarOff = this.ex.native_sar_pending_off() >>> 0;
      }
    } catch {}
  }

  spec(core) {
    const base = ((core * this.CORE_SZ + 336) >>> 2);
    const m = this.memU32();
    return { w72: m[base + 72] >>> 0, w73: m[base + 73] >>> 0, w230: m[base + 230] >>> 0, l0: m[base] >>> 0, l1: m[base + 1] >>> 0, l2: m[base + 2] >>> 0 };
  }

  pcOf(core) { return this.loader._wasmCores[core].PC >>> 0; }
  setPc(core, pc) { this.loader._wasmCores[core].PC = pc >>> 0; }
  pending(core) { return this.memU32()[((core * this.CORE_SZ + 2540) >>> 2)] >>> 0; }
  addInst(core, n) {
    const w = ((core * this.CORE_SZ + 2608) >>> 2);
    const m = this.memU32();
    m[w] = (m[w] + n) >>> 0;
  }

  guardPass(e, key) {
    let genOk = true;
    for (let r = e.rmin, i = 0; r <= e.rmax; r++, i++) {
      if ((this.ex.native_code_gen_region(r) >>> 0) !== e.rgens[i]) { genOk = false; break; }
    }
    if (!genOk) {
      // Live system moved on (usually an ISR stack store elsewhere — the
      // global gen is conservative). Drop the stale entry so the next kick
      // recompiles from a fresh ring trace; never run possibly-stale code.
      this.stats.gGen++;
      this.stats.invalidated++;
      try { this.cache.delete(key); } catch {}
      return false;
    }
    if (this.pending(e.core) !== 0) { this.stats.gPend++; return false; }
    const s = this.spec(e.core);
    const ok = s.w72 === e.s72 && s.w73 === e.s73 && s.w230 === e.s230
      && s.l0 === e.l0 && s.l1 === e.l1 && s.l2 === e.l2;
    if (!ok) this.stats.gSpec++;
    return ok;
  }

  // Try one JIT trace execution for core. Returns ops executed (0 = no hit).
  // Never throws: any failure falls back to the interpreter (returns 0 with
  // pc untouched... note partial prefix stays executed on exits — exact).
  tryCore(core) {
    if (!this.enabled) return 0;
    let pc;
    try { pc = this.pcOf(core); } catch { return 0; }
    const key = core + ':' + pc;
    let e = this.cache.get(key);
    if (e) {
      let ok = false;
      try { ok = this.guardPass(e, key); } catch { return 0; }
      if (!ok) return 0;
      let code;
      try { code = e.runFn() | 0; } catch { return 0; }
      this.stats.hits++;
      if (code === 0) {
        this.stats.completed++;
        try {
          this.setPc(core, e.endPC);
          this.addInst(core, e.len);
        } catch { return 0; }
        return e.len;
      }
      const br = e.branches[code - 100];
      if (!br) return 0;
      this.stats.exits++;
      try {
        this.addInst(core, br.idx);
        this.setPc(core, br.pc);
      } catch { return 0; }
      // One interpreter step executes the missed branch from its pc.
      try { this.ex.core_run(1); } catch { return 0; }
      return br.idx + 1;
    }
    this.stats.misses++;
    return 0;
  }

  // Scan the recorder ring for a fresh trace starting at (core, pc).
  // Returns entry material (T with nexts, recLoop, gen, spec) or null.
  findTrace(core, pc) {
    const ex = this.ex;
    let ptr, slen;
    try {
      ptr = ex.native_trace_scratch_ptr() >>> 0;
      slen = ex.native_trace_scratch_len() >>> 0;
    } catch { return null; }
    const ring = this.memU32().slice(ptr >>> 2, (ptr >>> 2) + slen);
    const STR = (ring.length - 1) / 16;
    if (!Number.isInteger(STR) || STR < 8) return null;
    const n = ring[0] >>> 0;
    for (let s = 0; s < Math.min(n, 16); s++) {
      const cc = ring[1 + s * STR] >>> 0;
      if (cc !== core) continue;
      const st = ring[1 + s * STR + 1] >>> 0;
      if (st !== pc) continue;
      const packed = ring[1 + s * STR + 2] >>> 0;
      const ln = packed & 0xFFFF;
      if (ln < this.minLen || ln > 256) continue;
      const rmin = (packed >>> 16) & 63, rmax = (packed >>> 24) & 63;
      const rgens = [];
      for (let r = rmin; r <= rmax; r++) rgens.push(this.ex.native_code_gen_region(r) >>> 0);
      const T = [];
      for (let i = 0; i < ln; i++) {
        T.push({
          pc: ring[1 + s * STR + 3 + i * 3] >>> 0,
          op: ring[1 + s * STR + 3 + i * 3 + 1] >>> 0,
          w: ring[1 + s * STR + 3 + i * 3 + 2] >>> 0,
        });
      }
      // Last pc bounds the window (emit all but last).
      const endPC = T[T.length - 1].pc;
      const TE = T.slice(0, T.length - 1);
      TE.forEach((e, i) => { e.next = (i + 1 < TE.length) ? TE[i + 1].pc : endPC; });
      const sp = this.spec(core);
      return {
        TE, endPC,
        rmin, rmax, rgens,
        s72: sp.w72, s73: sp.w73, s230: sp.w230,
        loop: [sp.l0, sp.l1, sp.l2],
        wb: sp.w72,
      };
    }
    return null;
  }

  // Compile (async: ~1ms instantiate) and cache a trace for (core, pc).
  // Returns true when a runnable entry was installed.
  async compile(core, pc) {
    const key = core + ':' + pc;
    if (this.cache.has(key) || this.uncompilable.has(key)) return this.cache.has(key);
    const found = this.findTrace(core, pc);
    if (!found) { this.stats.findNull++; return false; }
    let built;
    try {
      built = buildTraceModule(found.TE, {
        CORE: core, CBASE: core * this.CORE_SZ, wb: found.wb,
        recLoop: found.loop, sarOff: this.sarOff,
      });
    } catch (err) {
      this.stats.buildFail++;
      try {
        const m = String(err && err.message || err);
        const o = m.match(/unemittable op (0x[0-9a-f]+)/);
        if (o && Object.keys(this.stats.badOps).length < 24) {
          this.stats.badOps[o[1]] = (this.stats.badOps[o[1]] || 0) + 1;
        }
      } catch {}
      if (!this.stats.firstErr) { try { this.stats.firstErr = 'build:' + String(err && err.message || err).slice(0, 120); } catch {} }
      this.uncompilable.add(key);
      return false;
    }
    let runFn;
    try {
      runFn = await instantiateTrace(built.mod, this.loader, this.ex);
    } catch (err) {
      this.stats.instFail++;
      if (!this.stats.firstErr) { try { this.stats.firstErr = 'inst:' + String(err && err.message || err).slice(0, 120); } catch {} }
      this.uncompilable.add(key);
      return false;
    }
    if (this.cache.size >= this.maxEntries) {
      // Evict oldest quarter (Map preserves insertion order).
      let drop = Math.max(1, this.maxEntries >> 2);
      for (const k of this.cache.keys()) {
        if (drop-- <= 0) break;
        this.cache.delete(k);
        this.stats.evictions++;
      }
    }
    this.cache.set(key, {
      runFn, endPC: built.endPC !== null && built.endPC !== undefined ? built.endPC : found.endPC,
      len: built.len, branches: built.branches,
      core, rmin: found.rmin, rmax: found.rmax, rgens: found.rgens,
      s72: found.s72, s73: found.s73, s230: found.s230,
      l0: found.loop[0], l1: found.loop[1], l2: found.loop[2],
    });
    this.stats.compiles++;
    if (this.stats.compiles === 1) {
      try { console.log(`[JIT] first trace compiled: core=${core} pc=0x${pc.toString(16)} len=${found.TE.length}`); } catch {}
    }
    return true;
  }
}
