# Architecture

## Overview

A fast ESP32 (Xtensa LX6, dual-core) emulator. The CPU is executed by a
Rust-compiled **WASM engine**; peripherals run natively in Rust (inside the same
WASM module) and reach JavaScript only through a thin FFI glue layer. JavaScript
hosts the remaining subsystems that cannot live in WASM: the Worker /
SharedArrayBuffer IPC, the clock-event driver, the WiFi gateway bridge, the GDB
transport, and loader glue.

## Data flow

```
tests/*.mjs ─► src/sab/MultiSimulator.js ─► src/sab/worker-entry.js (worker thread)
                  │ allocates SharedArrayBuffers   │ creates ESP32 chip
                  │ spawns N workers               │ loads boot ROM + WASM engine
                   └── polls debug SAB + UART ring  └── blockingCommandLoop()
```

## Components

| File | Purpose |
|---|---|
| `engine-wasm/` | Rust crate → `src/engine/esp-xtensa/esp_engine_wasm.wasm` |
| `src/engine/esp-xtensa/wasm-loader.js` | Loads WASM, wires MMIO FFI, PTE overrides for every native peripheral, `mapAddress` page cache |
| `src/peripherals/esp32/esp32.js` | ESP32 chip class: memory regions, MMU, `step()`, `loadWasm()` |
| `src/engine/esp-xtensa/xtensa-core.js` | Facade only — exposes WASM core state to JS, never executes instructions |
| `src/sab/worker-entry.js` | Worker entry: chip setup, WASM load + reset, `blockingCommandLoop`, native event pumps |
| `src/sab/worker-proxy.js` | Main-thread proxy; spawns the worker, owns the SharedArrayBuffer command/UART ring |
| `src/sab/MultiSimulator.js` | N parallel WASM nodes, UART ring splitting, debug-state compare |

## Native peripherals

All peripherals (UART, GPIO, IO_MUX, SPI, I2C, I2S, LEDC, PCNT, RMT, MCPWM,
UHCI, SDMMC, SDIO slave, TWAI, TIMG, FRC, DPORT, RNG, AES, SHA, RSA, EFUSE,
SYSCON, RTC, EMAC, WiFi MAC/AP, BT RF, stub pages) are routed through Rust PTE
overrides. Measured fallback FFI traffic over a 58M-cycle firmware run: 0
`mmio_read`/`mmio_write`, 0 `map_read`, 3 `map_write` (GDB flash writes). Boot
ROM completes with **0 MMIO calls**.

## Memory model & IPC (SharedArrayBuffer)

The worker owns the WASM engine and the emulated RAM. It shares the actual
memory regions (flash, IRAM, DRAM, RTC, MMU table, WiFi MAC) with the main thread
as `SharedArrayBuffer`s, so GDB, `readMemory`, and host flash writes
(`js_spi_flash_set_byte`) work **zero-copy**. Control commands
(run/reset/step/read/write) use an `Atomics` command channel; UART output streams
through a shared ring buffer. See [Browser usage](./browser.md) for the
cross-origin-isolation requirement and [audit.md](./audit.md) for why the SAB is
kept.

## Clock

The self-timed core advances `CLK_CYCLES` inside WASM; idle fast-forward jumps
straight to the next timer alarm (`native_fast_forward`). The JS `SimulationClock`
only supplies host time.

## Limitations

- ESP32 only (ESP32-C3/RV32 support was removed).
- WASM is the only engine (the JS CPU interpreter was removed).
- BLE full-stack advertising is unsupported (requires the ESP32 LL/baseband, which is not emulated; `test-worker-bt` covers controller-init only).
