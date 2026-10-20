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
import { classify, buildTraceModule, instantiateTrace, findLoopSlices, buildPage } from './jit-trace-emit.js';

export class JitDriver {
  constructor(chip, loader, opts = {}) {
    this.chip = chip;
    this.loader = loader;
    this.ex = loader.exports;
    this.enabled = opts.enabled !== false && (typeof process === 'undefined' || !process.env.JIT?.startsWith?.('0'));
    this.dryRun = !!opts.dryRun;
    this.instGate = !!opts.instGate; // DIAG Exp D: no JIT until APP core retires (dual-core handshake safety)
    // Hotness gate for compiles: a pc must be missed this many times before a
    // window is compiled (0 = compile on first miss, the default).
    // DEFAULT 0 (off) — measured Oct 2026 on the 2.81G-instruction heavy
    // firmware: gating changes WHEN a hot loop is recorded, and that window set
    // diverges (8/20 wrong ACC at hot=100 before the phase fix in findTrace,
    // 4/20 after it) while hot=0 is 20/20 canonical. It also buys no speed —
    // the kick cadence below already throttles compiles (hot=100: 3.5-10.0s,
    // hot=0: 4.8-11.3s on the same runs). Keep the knob for probing.
    // Hot-gate probe sweep on the 268M-instruction firmware (no kick cadence):
    // 2000 -> 4.6-5.4s, 500 -> 2.8s, 100/50/25 -> 2.2s, 0 -> never finishes.
    this.hotMin = opts.hot !== undefined ? (opts.hot | 0)
      : (typeof process !== 'undefined' && process.env.JITHOT !== undefined ? (parseInt(process.env.JITHOT, 10) || 0) : 0);
    // Compile-kick cadence in retired instructions (0 = unthrottled).
    this.kickInst = opts.kick !== undefined ? (opts.kick | 0)
      : (typeof process !== 'undefined' && process.env.JITKICK !== undefined ? (parseInt(process.env.JITKICK, 10) || 0) : 4096);
    // Warmup gate: never compile before this core has retired N guest
    // instructions. Boot-phase app code (early init, one-shot list walks) is
    // where the remaining page-tier miscompiles live — a boot window wedges
    // the browser inside the boot loader (measured: page tier compiles
    // 0x400d82ba/0x400d8dxx during boot and never reaches "SPEEDGO"). Unlike
    // the park-count gate this is phase-neutral: it only delays the FIRST
    // compile, so a hot loop is recorded at the same loop phase either way.
    // The reference firmware prints its first line after ~6.7M instructions.
    this.bootInst = opts.boot !== undefined ? (opts.boot | 0)
      : (typeof process !== 'undefined' && process.env.JITBOOT !== undefined ? (parseInt(process.env.JITBOOT, 10) || 0) : 12000000);
    // Diagnostics (env-only): JITLIST dumps every compiled window; JITDELAY
    // simulates async-availability latency; JITSYNC (worker-entry) instantiates
    // synchronously so a window set reproduces identically across runs.
    this.listDump = !!(opts.list || (typeof process !== 'undefined' && !!process.env.JITLIST));
    this.listOps = !!(typeof process !== 'undefined' && !!process.env.JITLISTOPS);
    this.installDelayMs = (typeof process !== 'undefined' && process.env.JITDELAY) ? (parseInt(process.env.JITDELAY, 10) || 0) : 0;
    // Page tier is the default (see worker-entry.js); JITPAGE=0 opts back out
    // to the trace tier.
    this.pageEnabled = opts.page !== undefined ? !!opts.page
      : !(typeof process !== 'undefined' && process.env.JITPAGE === '0');
    this.cache = new Map(); // `${core}:${pc}` -> entry
    this.uncompilable = new Set();
    this.maxEntries = opts.maxEntries || 128;
    this._cooldown = new Map();
    this.minLen = opts.minLen || 24;
    this.stats = { hits: 0, completed: 0, exits: 0, misses: 0, compiles: 0, evictions: 0, gGen: 0, gPend: 0, gSpec: 0, invalidated: 0, findNull: 0, buildFail: 0, instFail: 0, firstErr: '', badOps: {}, exitSum: 0, exitN: 0, compLen: 0, pcExit: {}, pcComp: {} };
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

  // Chain loop-closed modules within one step (up to maxTraces): after a
  // completion the pc usually returns to a cached start, so keep executing.
  // Every chained entry re-checks the guard (window base / LOOP triple /
  // code-region generations / pending IRQs) instead of trusting the entry
  // guard taken before the previous module ran. Any exit or miss breaks to
  // the interpreter batch, which continues from the exact parked pc.
  // Returns total ops executed.
  instOf(core) { try { return this.memU32()[((core * this.CORE_SZ + 2608) >>> 2)] >>> 0; } catch { return 0; } }

  chainCore(core, maxTraces = 8) {
    if (this.dryRun) return 0;
    if (this.instGate && this.instOf(1) < 10000000) return 0;
    try {
      if ((this.pcOf(core) >>> 0) < 0x400d0020) return 0;
    } catch { return 0; }
    let total = 0;    for (let n = 0; n < maxTraces; n++) {
      let pc;
      try { pc = this.pcOf(core); } catch { break; }
      const key = core + ':' + pc;
      const e = this.cache.get(key);
      if (!e) {
        if (n === 0) this.stats.misses++;
        break;
      }
      // Guard EVERY entry, not just the first of a chain. A chained entry
      // follows a module that already retired guest code (page modules run
      // thousands of ops), so the compile-time window base / LOOP triple may
      // no longer describe it; entering with stale baked window offsets
      // mis-addresses the register file and can produce a wild jump.
      // guardPass is memory reads + one region-gen FFI, negligible next to a
      // page module's run.
      {
        let ok = false;
        try { ok = this.guardPass(e, key); } catch { break; }
        if (!ok) break;
      }
      if (e.page) {
        // Phase 2 page entry: module self-accounts INSTW; park pc via exitpc.
        let code;
        try {
          const i0 = this.instCount(e.core);
          code = e.run() | 0;
          const ops = ((this.instCount(e.core) - i0) >>> 0);
          this.setPc(e.core, e.exitpc.value >>> 0);
          this.stats.hits++;
          if (code === 2) {
            this.stats.completed++;
            e.compCount = (e.compCount || 0) + 1;
            total += ops;
            if (total >= 2048) break; // page runs retire more per call; wider batch
            continue;
          }
          this.stats.exits++;
          e.exitCount = (e.exitCount || 0) + 1;
          if (e.exitCount >= 64 && (e.compCount || 0) === 0) {
            try { this.cache.delete(key); } catch {}
            try { this._cooldown.set(key, Date.now() + 30000); } catch {}
            this.stats.evictedPoison = (this.stats.evictedPoison || 0) + 1;
          }
          total += ops;
          break;
        } catch { break; }
      }
      let code;
      try { code = e.runFn() | 0; } catch { break; }
      this.stats.hits++;
      if (code === 0) {
        this.stats.completed++;
        try {
          this.setPc(core, e.endPC);
          this.addInst(core, e.len);
        } catch { break; }
        total += e.len;
        if (total >= 512) break; // keep per-step totals near batch scale
        continue;
      }
      const br = e.branches[code - 100];
      if (!br) break;
      this.stats.exits++;
      this.stats.exitSum += br.idx; this.stats.exitN++;
      e.exitCount = (e.exitCount || 0) + 1;
      // Poison-trace eviction: entries that only ever exit (compiled from
      // an anomalous phase/recording) are pure overhead — drop them so a
      // fresh phase trace can take the slot. Completions reset the count.
      if (e.exitCount >= 64 && (e.compCount || 0) === 0) {
        try { this.cache.delete(key); } catch {}
        try { this._cooldown.set(key, Date.now() + 30000); } catch {}
        this.stats.evictedPoison = (this.stats.evictedPoison || 0) + 1;
      }
      try {
        const px = '0x' + e.startPc.toString(16);
        this.stats.pcExit[px] = (this.stats.pcExit[px] || 0) + 1;
      } catch {}
      try {
        this.addInst(core, br.idx);
        this.setPc(core, br.pc);
      } catch { break; }
      total += br.idx;
      break;
    }
    return total;
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
      if (e.page) {
        let code;
        try {
          const i0 = this.instCount(core);
          code = e.run() | 0;
          const ops = ((this.instCount(core) - i0) >>> 0);
          this.setPc(core, e.exitpc.value >>> 0);
          this.stats.hits++;
          if (code === 2) {
            this.stats.completed++;
            e.compCount = (e.compCount || 0) + 1;
            return ops;
          }
          this.stats.exits++;
          e.exitCount = (e.exitCount || 0) + 1;
          return ops;
        } catch { return 0; }
      }
      let code;
      try {
        if (typeof process !== 'undefined' && process.env.JITTRACE) {
          if ((this._callLog = (this._callLog || 0) + 1) <= 12) {
            try { console.log(`[JITCALL] #${this._callLog} len=${e.len} branches=${e.branches.length}`); } catch {}
          }
        }
        code = e.runFn() | 0;
        if (typeof process !== 'undefined' && process.env.JITTRACE) {
          if ((this._callLog || 0) <= 12) { try { console.log(`[JITCALL] #${this._callLog} returned ${code}`); } catch {} }
        }
      } catch (err) {
        if (typeof process !== 'undefined' && process.env.JITTRACE) {
          try { console.log(`[JITCALL] threw: ${(err && err.message) || err}`); } catch {}
        }
        return 0;
      }
      this.stats.hits++;
      if (code === 0) {
        this.stats.completed++;
        this.stats.compLen = e.len;
        e.compCount = (e.compCount || 0) + 1;
        try {
          const px = '0x' + e.startPc.toString(16);
          this.stats.pcComp[px] = (this.stats.pcComp[px] || 0) + 1;
        } catch {}
        try {
          this.setPc(core, e.endPC);
          this.addInst(core, e.len);
        } catch { return 0; }
        return e.len;
      }
      const br = e.branches[code - 100];
      if (!br) return 0;
      this.stats.exits++;
      this.stats.exitSum += br.idx; this.stats.exitN++;
      // Prefix executed (br.idx ops); pc parks AT the missed branch/op and
      // the step's interpreter batch executes it and everything after.
      // (Never single-step the suffix: one FFI per op is ~50x batch cost
      // and advancing both cores per single-step double-counts.)
      try {
        this.addInst(core, br.idx);
        this.setPc(core, br.pc);
      } catch { return 0; }
      return br.idx;
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
      let endPC = T[T.length - 1].pc;
      let TE = T.slice(0, T.length - 1);
      TE.forEach((e, i) => { e.next = (i + 1 < TE.length) ? TE[i + 1].pc : endPC; });
      // PHASE CONSISTENCY (required): a closed slice must repeat with a
      // uniform period. A compile kick fires at an arbitrary park, so the
      // ring's newest pass can begin MID-ITERATION; the first period is then
      // shorter than the steady-state period and buildPage emits two dispatch
      // groups with the same start pc -- only the first (truncated) one is
      // reachable, so the loop body that updates the induction registers is
      // dead code and the guest gets a wrong trip count (measured: 8/20 wrong
      // ACC on the heavy firmware, and the boot list-walk wedge). Drop the
      // truncated prefix: TE[i0] has the SAME pc as TE[0], so the module's
      // anchor/entry pc and guard context are unchanged.
      {
        const p0 = TE[0].pc >>> 0;
        const occ = [];
        for (let i = 0; i < TE.length; i++) if ((TE[i].pc >>> 0) === p0) occ.push(i);
        if (occ.length >= 3) {
          const per = occ[2] - occ[1];
          if (occ[1] - occ[0] !== per && per >= this.minLen) {
            let end = occ[1] + per;
            while (end + per <= TE.length) end += per;
            TE = TE.slice(occ[1], end);
            endPC = p0; // the period repeats: the successor of the last op is p0
            TE.forEach((e, i) => { e.next = (i + 1 < TE.length) ? TE[i + 1].pc : endPC; });
            this.stats.phaseShift = (this.stats.phaseShift || 0) + 1;
          }
        }
      }
      const sp = this.spec(core);
      // Prefer a loop-closed slice in the completable band (24..64 ops):
      // the bench shows 200+ op windows never validate (phase flips) while
      // ~56-op closed slices go 3/3 clean. Falls back to the full window.
      let use = TE, useEnd = endPC, closed = false;
      try {
        const cands = findLoopSlices(TE);
        for (const cand of cands) {
          const L = cand.end - cand.start;
          // ANCHOR REQUIRED (see compile()): the slice must start at the
          // window start, otherwise the module's hardcoded entry pc and the
          // entry guard context would not match the pc it is entered at.
          if (cand.start !== 0) continue;
          if (L >= this.minLen && L <= 64) {
            const S = TE.slice(cand.start, cand.end);
            use = S;
            useEnd = S[0].pc;
            closed = true;
            break;
          }
        }
      } catch {}
      use.forEach((e, i) => { e.next = (i + 1 < use.length) ? use[i + 1].pc : useEnd; });
      return {
        TE: use, endPC: useEnd, closed,
        rmin, rmax, rgens,
        s72: sp.w72, s73: sp.w73, s230: sp.w230,
        loop: [sp.l0, sp.l1, sp.l2],
        wb: sp.w72,
      };
    }
    return null;
  }

  // One-time self-test: a 1-op module must round-trip shared memory
  // exactly (guards against detached/stale memory views, which would
  // silently diverge the guest). Failure disables the driver permanently.
  // Memoized promise: the flag used to be set before the awaits, so every
  // compile kicked while the first self-test was in flight saw `undefined`
  // and silently bailed (measured: compiles started, none completed).
  selfTest() {
    if (!this._selfTestP) this._selfTestP = this._selfTestRun();
    return this._selfTestP;
  }
  async _selfTestRun() {
    try {
      const CBASE = 0, wb = 0;
      const TE = [{ pc: 0x1000, op: 0x551b, w: 2, next: 0x1002 }];
      const built = buildTraceModule(TE, { CORE: 0, CBASE, wb, recLoop: [0, 0, 0], sarOff: 4096 });
      const runFn = await instantiateTrace(built.mod, this.loader, this.ex);
      const m = this.memU32();
      const phys = ((0 + 16) >>> 2) + 5; // logical a5, window base 0
      const saved = m[phys] >>> 0;
      const before = 0x12345678;
      m[phys] = before;
      const code = runFn() | 0;
      const after = m[phys] >>> 0;
      m[phys] = saved;
      this._selfTestOk = (code === 0 && after === ((before + 1) >>> 0));
      if (!this._selfTestOk) {
        try { console.log(`[JIT] self-test FAILED code=${code} before=0x${before.toString(16)} after=0x${after.toString(16)}; driver disabled`); } catch {}
        this.enabled = false;
      } else {
        try { console.log('[JIT] self-test passed (shared memory live)'); } catch {}
        // Phase 2: call-out round trip (mr32/mw32 -> engine exports ->
        // MMU path). Untested-in-browser WASM->JS->WASM reentrancy lives
        // here; failure disables the driver instead of wedging the tab.
        try {
          const SCR = 0x3ffe7f00;
          const TE2 = [
            { pc: 0x2000, op: 0x218, w: 2, next: 0x2002 }, // l32in a1=[a2]
            { pc: 0x2002, op: 0x1349, w: 2, next: 0x2004 }, // s32in [a3+4]=a4
          ];
          const b2 = buildTraceModule(TE2, { CORE: 0, CBASE: 0, wb: 0, recLoop: [0, 0, 0], sarOff: 4096 });
          const f2 = await instantiateTrace(b2.mod, this.loader, this.ex);
          const w0 = ((0 + 16) >>> 2);
          const s2 = m[w0 + 2] >>> 0, s3 = m[w0 + 3] >>> 0, s4 = m[w0 + 4] >>> 0;
          const smem = this.ex.core_read_uint32(0, SCR) >>> 0;
          this.ex.core_write_uint32(0, SCR, 0xA5A50000);
          m[w0 + 2] = SCR; m[w0 + 3] = SCR + 8; m[w0 + 4] = 0x51ED0000;
          const c2 = f2() | 0;
          const r1 = m[w0 + 1] >>> 0, rw = this.ex.core_read_uint32(0, SCR + 12) >>> 0;
          m[w0 + 2] = s2; m[w0 + 3] = s3; m[w0 + 4] = s4;
          this.ex.core_write_uint32(0, SCR, smem);
          this.ex.core_write_uint32(0, SCR + 12, 0);
          if (!(c2 === 0 && r1 === 0xA5A50000 && rw === 0x51ED0000)) {
            try { console.log(`[JIT] self-test phase 2 FAILED code=${c2} r1=0x${r1.toString(16)} mem=0x${rw.toString(16)}; driver disabled`); } catch {}
            this._selfTestOk = false;
            this.enabled = false;
          } else {
            try { console.log('[JIT] self-test phase 2 passed (call-outs live)'); } catch {}
          }
        } catch (e) {
          try { console.log('[JIT] self-test phase 2 error, driver disabled: ' + (e && e.message)); } catch {}
          this._selfTestOk = false;
          this.enabled = false;
        }
      }
    } catch (e) {
      try { console.log('[JIT] self-test error, driver disabled: ' + (e && e.message)); } catch {}
      this._selfTestOk = false;
      this.enabled = false;
    }
    return this._selfTestOk;
  }

  // Compile (async: ~1ms instantiate) and cache a trace for (core, pc).
  // Returns true when a runnable entry was installed.
  async compile(core, pc) {
    if ((pc >>> 0) < 0x400d0020) return false; // boot code: never worth compiling
    const key = core + ':' + pc;
    if (this.cache.has(key) || this.uncompilable.has(key)) return this.cache.has(key);
    if (!(await this.selfTest())) return false;
    // Cooldown for poison-evicted pcs (avoids immediate recompile churn).
    const cool = this._cooldown && this._cooldown.get(key);
    if (cool && Date.now() < cool) return false;
    const found = this.findTrace(core, pc);
    if (!found) { this.stats.findNull++; return false; }
    // ANCHOR INVARIANT: an emitted module hardcodes its entry pc to the
    // recorded window start (buildPage: local pc = Sc[0].pc; a trace runs
    // its ops from TE[0]) and the driver enters a cached module whenever the
    // guest pc equals its cache key. The entry guard context (window
    // base/start + register generations) equally belongs to that window
    // start, so a module is only valid at the pc it was recorded at. A
    // window anchored elsewhere (mid-window loop slice) would execute a
    // foreign code region and park the guest at that region's exit pc
    // (observed: wild jump -> exception storm in IRAM). Refuse: interpreter
    // covers the code, and findTrace never returns unanchored windows now.
    if ((found.TE[0].pc >>> 0) !== (pc >>> 0)) {
      this.stats.anchorRej = (this.stats.anchorRej || 0) + 1;
      return false;
    }
    if (this.pageEnabled) return this.compilePage(core, pc, found);
    let built;
    const foldedAddrs = [];
    try {
      const ex2 = this.ex;
      built = buildTraceModule(found.TE, {
        CORE: core, CBASE: core * this.CORE_SZ, wb: found.wb,
        recLoop: found.loop, sarOff: this.sarOff,
        noCallout: !!this.chip._jitNoCallout,
        noFold: !!this.chip._jitNoFold,
        folded: foldedAddrs,
        readMem: (addr) => ex2.core_read_uint32(core, addr) >>> 0,
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
    // Diag: simulate async module-availability latency deterministically.
    if (this.installDelayMs > 0) {
      await new Promise((r) => setTimeout(r, this.installDelayMs));
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
    if (this._cooldown) { try { this._cooldown.delete(key); } catch {} }
    try {
      if (typeof process !== 'undefined' && process.env.JITDUMP && !this._dumped) {
        this._dumped = true;
        const fs = await import('fs');
        fs.writeFileSync('/home/danish1075/Documents/esp32 emu/tests/tmp-boottrace.bin', Buffer.from(built.mod));
        try { console.log('[JITDUMP] wrote ' + built.mod.length + ' bytes, ops=' + built.len); } catch {}
      }
    } catch {}
    this.cache.set(key, {
      runFn, endPC: built.endPC !== null && built.endPC !== undefined ? built.endPC : found.endPC,
      len: built.len, branches: built.branches, startPc: pc,
      core, rmin: found.rmin, rmax: found.rmax, rgens: found.rgens,
      s72: found.s72, s73: found.s73, s230: found.s230,
      l0: found.loop[0], l1: found.loop[1], l2: found.loop[2],
    });
    this.stats.compiles++;
    this.stats.folds = (this.stats.folds || 0) + foldedAddrs.length;
    if (typeof process !== 'undefined' && process.env.JITTRACE) {
      try {
        const shapes = found.TE.map((e) => '0x' + e.op.toString(16)).join(',');
        console.log(`[JITTRACE] core=${core} pc=0x${pc.toString(16)} len=${found.TE.length} ops=${shapes}`);
      } catch {}
    } else if (this.stats.compiles === 1) {
      try { console.log(`[JIT] first trace compiled: core=${core} pc=0x${pc.toString(16)} len=${found.TE.length}`); } catch {}
    }
    return true;
  }

  // Phase 2 page tier: compile the recorded window as an in-WASM threaded
  // region (groups + dispatch loop) instead of a single linear trace.
  // Loops inside the recorded set stay in-WASM (no per-iteration exits);
  // page-leave/yield/miss return with exitpc parked. Same cache, guards,
  // poison, cooldown and uncompilable discipline as the trace tier.
  async compilePage(core, pc, found) {
    const key = core + ':' + pc;
    // SELF-CONTAINED LOOP SLICES ONLY. A straight-line fallback window (the
    // full 255-op recording) ends at the ring tail (endPC), so its module
    // depends on where the recording happened to stop and can span phase or
    // call boundaries -- those windows are where the remaining miscompiles
    // live (boot-phase quiet stalls and, rarely, wrong loop-trip counts).
    // A loop-closed slice starts and ends at the same pc and is therefore
    // self-contained; the hot speed loops are exactly those, so nothing is
    // lost where it matters. Escape hatch for experiments: _jitPageWindow.
    if (!found.closed && !this.chip._jitPageWindow) {
      this.stats.openRej = (this.stats.openRej || 0) + 1;
      return false;
    }
    let handle;
    const foldedAddrs = [];
    try {
      const ex2 = this.ex;
      handle = await buildPage(found.TE.map((e) => ({ ...e })), {
        CORE: core, CBASE: core * this.CORE_SZ, L: this.loader, ex: this.ex,
        yieldOps: 2048,
        wb: found.wb, recLoop: found.loop, sarOff: this.sarOff,
        noCallout: !!this.chip._jitNoCallout,
        noFold: !!this.chip._jitNoFold,
        folded: foldedAddrs,
        readMem: (addr) => ex2.core_read_uint32(core, addr) >>> 0,
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
      if (!this.stats.firstErr) { try { this.stats.firstErr = 'page-build:' + String(err && err.message || err).slice(0, 120); } catch {} }
      this.uncompilable.add(key);
      return false;
    }
    if (!handle || typeof handle.run !== 'function') {
      this.stats.instFail++;
      if (!this.stats.firstErr) { try { this.stats.firstErr = 'page-inst: no run handle'; } catch {} }
      this.uncompilable.add(key);
      return false;
    }
    if (this.cache.size >= this.maxEntries) {
      let drop = Math.max(1, this.maxEntries >> 2);
      for (const k of this.cache.keys()) {
        if (drop-- <= 0) break;
        this.cache.delete(k);
        this.stats.evictions++;
      }
    }
    if (this._cooldown) { try { this._cooldown.delete(key); } catch {} }
    this.cache.set(key, {
      page: true, run: handle.run, exitpc: handle.exitpc, startPc: pc,
      core, rmin: found.rmin, rmax: found.rmax, rgens: found.rgens,
      s72: found.s72, s73: found.s73, s230: found.s230,
      l0: found.loop[0], l1: found.loop[1], l2: found.loop[2],
      len: found.TE.length,
    });
    this.stats.compiles++;
    this.stats.pageCompiles = (this.stats.pageCompiles || 0) + 1;
    if (this.listDump) {
      try {
        console.log(`[JP] core=${core} pc=0x${(pc >>> 0).toString(16)} len=${found.TE.length} ops=${handle.ops} groups=${handle.groups} wb=${found.wb} loop=${JSON.stringify(found.loop)} regs=${found.rmin}..${found.rmax}`);
        if (this.listOps) console.log(`[JPO] core=${core} pc=0x${(pc >>> 0).toString(16)} len=${found.TE.length} pcs=${found.TE.map((e) => (e.pc >>> 0).toString(16)).join(',')}`);
      } catch {}
    }
    this.stats.folds = (this.stats.folds || 0) + foldedAddrs.length;
    return true;
  }

  instCount(core) {
    try { return this.memU32()[((core * this.CORE_SZ + 2608) >>> 2)] >>> 0; }
    catch { return 0; }
  }
}
