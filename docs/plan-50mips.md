# Plan: 50+ MIPS in the browser (WASM)

Goal: sustained **>50 MIPS** on CPU-bound guest code in the browser build,
with firmware behavior unchanged (UART-trace equivalence) and timing
fidelity no worse than today. Anything that trades either is listed under
rejected, not postponed.

## Baseline (measured, same dual-core busy firmware)

| Path | Throughput |
|---|---|
| Raw, no SAB | ~30 MIPS end-to-end (26–31 across box states) |
| 1 worker, SAB on (production) | ~26 MIPS |
| 2 / 4 workers | ~53 / ~96 total |

Method: retired `inst_count`, end-to-end incl. boot, same-run ratios only
(DVFS/contention move absolutes ±2× across days; see README Performance).

## Why the interpreter caps near ~35 (all measured)

- V8 profile: `run_instruction` self 75%, fetch helpers 9%, handler
  bodies 4%, shim 2%, `core_run` 5%. Execution is tiny; dispatch is all.
- The narrow dispatch tree is already compiler-optimal: a generation-
  validated bypass cache measured **−14%** interleaved A/B (probe ≥ tree),
  a reread-validated one −20%. Fast lanes cannot beat a predicted tree.
- Boundary costs are zero (batch sweep +6%-at-noise, pump sweep exactly 0).
- Banked micro-wins (kept): breakpoint-FFI gate +9%, inst batching,
  unchecked AR, hot-handler inlines. Each ≤10%, all accuracy-neutral.

Conclusion: no further interpreter tweak crosses 50. The remaining lever
is executing fewer interpreter steps per guest instruction — i.e., a
second execution tier. Precedent: v86 (x86-to-WASM JIT in the browser).

## Phase 1 — squeeze the interpreter (days, mostly done)

Cold outlining (BT/SPIN/FREEZE/DEFER/trace formatting out of the hot
function), hot-handler force-inlines, unchecked register access. Each
measured or provably identical; anything unmeasurable reverts. Realistic
ceiling after this phase: ~33–36 MIPS. tracked on this branch.

## Phase 2 — trace-based WASM JIT (weeks; THE 50-crosser)

Keep the interpreter as tier 0 (fallback, diag, single-step — untouched).
Add a tier that compiles hot *linear traces* to WASM at runtime.

- **Unit: traces, not pages.** Record from loop back-edges (backward
  branches + the LOOP instruction) until the next backward edge, capped
  length. No joins inside a trace → no stackifier, no br_table problem
  (the two hard parts of v86's page compiler simply don't occur).
- **Emission: hybrid.** Top ~20 integer/load/store/branch ops emitted
  inline as WASM (new code, bounded scope); everything else emitted as a
  call-out to the EXISTING handler functions (same semantics, zero
  re-implementation for the long tail).
- **State:** guest regs in WASM locals within the trace, synced to the
  SAB-backed CoreState at exits (exits: MMIO, interrupt-pending check per
  N ops, branch-direction miss, cold-op call-out, trace end).
- **Guards:** entry guard = (pc, page generations, MMU generation, pie) —
  the generation machinery designed for this (store funnels audited:
  mem_write8/16/32, flash_set_byte, GDB-override hammer). Any code write
  or remap fails the guard → fall back to the interpreter. Invalidation
  is free and exact.
- **Compile:** async `WebAssembly.Module` + instantiate off the hot path;
  key compiled traces by op-sequence hash to reuse across boots; cap live
  count with cold eviction (v86's 900-slot lesson); interpreter covers
  everything meanwhile (no startup cliff).
- **Timing fidelity, preserved:** retired counts added in bulk per trace
  (exact `inst_count`); virtual time charged per instruction; traces yield
  at least as often as today's 512-batches, so event latency is no worse
  than production. Cycle-counting precision was never offered (CCOUNT is
  already batch-maintained) and is not lost.
- **Validation (non-negotiable):** UART-trace equivalence interpreter-vs-JIT
  on the battery (deterministic emulator ⇒ byte-identical output), the ACC
  bit-exact oracle, full worker suite in both modes, plus a JIT on/off
  kill-switch that stays forever (any divergence report reproduces with
  one flag).
- **Estimate:** compute loops 5–10× (near-native for straight-line integer
  code), mixed firmware 2–3× → 60–90 loops, 50–70 end-to-end. First
  measurable milestone: loop-only micro-JIT (no MMIO, no branches except
  the back-edge) beating the interpreter 3×+ on the busy firmware.

## Phase 3 — only if Phase 2 lands short

Superoperators *inside* traces (free once emitting: fuse top pairs from
the measured nibble histogram), wider superblock shapes with guards,
fetch TLB for MMIO-heavy code. Each small, each measured.

## Explicitly rejected (do not reopen without new evidence)

- **Dual-core threads:** kills determinism (wall-clock interleaving breaks
  every diag diff, race repro, and calibration in the tree). Fundamental.
- **CCOUNT/timing granularity changes:** directly guest-observable
  (delayMicroseconds/DHT/OneWire read CCOUNT at µs scale). Fundamental.
- **Classic JIT (method/page, LLVM at runtime):** no LLVM in the browser;
  man-months for gains capped by exit density. v86-style WASM emission
  (Phase 2) dominates it on every axis here.
- **Native addon:** same logic, better codegen — but forks the platform
  matrix and breaks the web story that motivates this whole plan.
- **Bigger batches / thinner pumps:** measured +6%-at-noise and exactly
  zero. Not levers.

## Standing rules for all phases

1. Same-run ratios only (taskset-pinned P-cores, interleaved A/B).
2. No speed change merges without battery green + (for execution-tier
   changes) UART-trace equivalence.
3. Kill-switches stay (JIT on/off, probe gates); unmeasurable cuts revert.
4. `main` receives only measured-green work; experiments live here.
