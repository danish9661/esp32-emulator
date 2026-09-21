# Audit — ESP32 WASM Emulator (`esp32emu`)

Operational and security audit of the fully-WASM ESP32 (Xtensa LX6, dual-core)
emulator. Measurements below were taken on this machine (Linux, Node 22,
release-built WASM engine); absolute numbers vary by CPU, but the ratios and
orders of magnitude are representative.

> Source of numbers:
> - **Measured (this run)**: `tests/wasm-bench.mjs`, `tests/test-wasm-standalone.mjs`,
>   on-disk sizes.
> - **Documented**: `AGENTS.md` tracker + prior `MultiSimulator`/`test-real-firmware`
>   runs.

---

## 1. Scope

| Item | State |
|---|---|
| CPU engine | Rust → WASM (the only engine; JS interpreter removed) |
| Peripherals | Native Rust ports reached via MMIO FFI (PTE overrides) |
| Host glue | JS (Worker host, SharedArrayBuffer IPC, GDB, clock driver, WiFi bridge) |
| Tests | 31 passing (28 non-WiFi + 3 WiFi) |
| BLE full-stack | **Parked** — needs Xtensa disassembler for LL HCI handler |

---

## 2. Performance

| Benchmark | Result | Notes |
|---|---|---|
| ROM boot (20K inst) | ~0.63 ms total (5 batches, 551/16/2/37/22 µs) | Final PC `0x4000fdac` → boot ROM completion |
| Per-instruction (boot) | ~0.03 µs/inst avg; warm batches 0.00 µs | Page cache + flash fast-path |
| `test-wasm-standalone` (1M steps) | sub-second (~0 s wall) | WASM-only, no FFI stalls |
| `test-real-firmware` | 332 M cycles in 1.0 s | Full peripheral firmware, WASM engine |
| `MultiSimulator` scaling | 16.5 M steps/s (1 node) → 54.5 M total (4 nodes, ~3.3×) | Per-node linear speedup |

**Why it's fast:**
- **Flash fast-path** (PTE_TYPE_FLASH): opcode/data fetches from the flash
  cache window resolve entirely inside the WASM linear memory (4 MB flash
  mirror) — no JS round-trip.
- **Page cache** (`_mapCache`, keyed by `addr >>> 12`) avoids per-access region
  lookups and allocations.
- **Self-timed core**: clock values computed in-WASM; idle fast-forward
  (`native_fast_forward`) jumps `CLK_CYCLES` straight to the next timer alarm.
- **Native timer events in the busy loop** fire every 512 steps, so alarm
  interrupts occur while the CPU runs (hardware-like, no busy-wait amplification).

---

## 3. Memory footprint

### WASM linear memory layout (per `AGENTS.md`)
| Region | Offset | Size |
|---|---|---|
| Static zone | `0x400000` | holds Rust statics (register files, buffers) |
| Page table | `0x400000` (4,194,304) | MMU page table |
| Region table | `0xC00000` (12,582,912) | memory region descriptors |
| RAM data | `0xC00100` (12,583,168) | DRAM/IRAM mirror |
| Flash data | `0x1200000` (18,874,624) | **4 MB flash mirror** |

The flash mirror + region tables put the minimum useful linear-memory size in
the ~20–24 MB range; the WASM module allocates this up front. The engine does
**not** grow memory per step.

### SharedArrayBuffers (host↔worker IPC)
- UART ring buffer (streaming TX output)
- Debug/SAB state (polled every 2 s, time-based)
- `rtcSlowMem` / `rtcFastMem` (survive deep-sleep reset)
- Flash SAB (host-owned, kept in sync via `js_spi_flash_set_byte`)

### On-disk
| Artifact | Size |
|---|---|
| `esp_engine_wasm.wasm` | 1.5 MB |
| `dist/` (bundled package) | 3.2 MB |
| `src/` (unbundled) | 2.3 MB |
| `node_modules` (incl. esbuild, axios) | 15 MB (dev only) |

---

## 4. Memory leaks & lifecycle

**Engine (hot path): no leaks.** Stepping allocates nothing per instruction
(the old per-access `new ReadonlyMemory(...)` was removed by the page cache).
WASM linear memory is fixed-size.

**Lifecycle risks (host/JS side):**
- **Worker threads** — each `SimulatorWorker` spawns a Node `worker_threads`
  instance holding the WASM module + SABs. If `worker.stop()` is not called,
  the worker and its memory persist for the process lifetime. Consumers must
  stop workers when done (the `webserver` test self-terminates after 20 s to
  avoid this).
- **WiFi gateway socket / pcap buffers** — the gateway bridge keeps a WebSocket
  and an accumulating pcap; only active during WiFi tests. Call `getPcapData()`
  / close the socket to release.
- **Static-layout fragility (design note, not a leak)**: adding new `static mut`
  register files in Rust shifts the linker's static-data layout and can break
  boot (the DROM window falls back to JS). Mitigation: `STATIC_ZONE` move put
  new statics below the 4 MB line so layout changes are safe. Storage-free
  handlers (e.g. `HID_STUB_ZERO`, `HID_INVALID_MEM`) carry no statics.

---

## 5. Overhead

- **FFI call rate**: ~0.00004 FFI calls/cycle (`diag-ffi-counts`, full
  `fw-proxy32` run ≈ 1395 JS `mmio_read`/`mmio_write` over 58 M cycles;
  `map_read` 0, `map_write` 3 = GDB flash). Near-zero host round-trips.
- **JS surface minimized** to design-only:
  - `SimulationClock` host time source (advances `chip.cycles`)
  - loader FFI glue
  - zero-traffic `map_read`/`map_write` fallback (GDB flash writes + never
    touched DROM0 window)
- **Boot MMIO**: `Total MMIO calls: 0` — the entire boot ROM runs against
  native Rust peripheral ports.

---

## 6. Security

**Threat model:** the emulator loads and executes *untrusted* code — the real
ESP32 boot ROM and arbitrary user firmware. Safety rests on the WASM sandbox.

| Control | Status |
|---|---|
| Untrusted code execution | Runs inside WASM; **no syscalls, no `fs`/network from wasm** |
| Host I/O | Only via explicitly-defined JS FFI exports (MMIO, clock events, WiFi bridge) — auditable surface |
| Loader | `fetch` (http/https/data) **or** `readFileSync`; **no `eval`/`Function`** (the `trace.js` `eval("require")` dead code was removed) |
| GDB FFI | `gdb-session.js` uses an **in-process transport** (no network server by default), so untrusted-input exposure only arises if *you* attach a network transport. When attached, it can write flash (`js_flash_write_override`) and read/write memory — treat that transport as privileged |
| Supply chain | Boot ROM is Espressif `esp-rom-elfs`, **Apache-2.0**; attribution in `THIRD-PARTY-NOTICES.md` |
| Worker isolation | Each simulator runs in its own `worker_threads` context (separate WASM instance + SABs) |

**Recommended hardening (not yet enforced):**
- Run the WASM instance with a bounded memory and disabled `table.grow`
  beyond need.
- Expose the GDB server only on localhost / behind auth.
- In the browser, serve with COOP/COEP headers so `SharedArrayBuffer` works
  and the worker is fully isolated.

---

## 7. Correctness & test coverage

- **31 tests pass**: `PASS=28 FAIL=0 SKIP=3` (non-WiFi) + `PASS=31 FAIL=0 SKIP=0`
  (with WiFi gateway up).
- Boot path verified: ROM boot completes with **0 MMIO calls** (fully native).
- UART output round-trips through the SAB ring and the published `dist` package
  boots correctly as an installed consumer.

---

## 8. Known limitations / future work

- **BLE full-stack advertising** — unsupported. `test-worker-bt` (controller-init)
  passes; the full BTDM `HCI_RESET` response requires the ESP32 LL/baseband (in
  ROM), which is not emulated. The Xtensa disassembler was used to confirm the
  VHCI delivery path (btrom `cb108`/`cb100`/`cb4c`) needs LL controller state.
- **Firmware tests need an external compile server** (`:5000`, arduino-cli) and
  (for WiFi) the gateway (`:5085`). No-compile-server tests (`wasm-bench`,
  `test-wasm-standalone`) run standalone.
- **Browser use** requires a bundler to collapse `dist/` into one file and a
  COOP/COEP context for `SharedArrayBuffer`; the fetch-based loader already
  avoids `fs`.

---

## 9. Reproduce the measurements

```bash
# perf + MMIO (no server needed)
node tests/wasm-bench.mjs
node tests/test-wasm-standalone.mjs

# requires compile server on :5000
node ./tests/compile-server.mjs &
./tests/run-worker-tests.sh --timeout=900

# requires gateway on :5085 (WiFi tests only)
./tests/run-worker-tests.sh --wifi --timeout=900

# bundle size
npm run build && du -sh dist
```
