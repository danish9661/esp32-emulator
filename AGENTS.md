# AGENTS.md — ESP32 WASM Emulator

Guidance for AI agents and contributors working in this repository.

## Communication style

- DO NOT write "hmm" (or any filler like it) in thinking or in replies. State
  findings directly.

## Primary mission — BLE/BT + GATT (user directive, 2026-09-15)

- #1 priority is completing full BLE/BT + GATT: `BLU_ENABLE_DONE` →
  `ADV_START_DONE` (ADV stage) → `GATT_SVC_STARTED` (GATT stage). This
  supersedes all opportunistic refactors, cleanups, and side investigations.
- Do NOT divert from the plan. Every engine change must trace directly to an
  observed BLE stall signature (UART marks, `[RING]`/`[RD]`/`[BTRF]` evidence,
  ELF disassembly). No drive-by fixes, no unrelated hardening, no doc-only
  churn mid-phase.
- Phase order: (1) unblock `esp_bluedroid_init`/`esp_bluedroid_enable` HCI
  RESET CC, (2) ADV data/start, (3) GATT register/create/start, (4) full
  regression battery, (5) SINGLE commit covering BLE/BT+GATT. Do not commit
  partial/forensics-only checkpoints.
- Diag scripts live in `tests/tmp-*.mjs` (NOT `/tmp` — it gets wiped) and
  stay untracked until the final commit. Keep `tests/tmp-adv-run*.log` for
  the current phase only; delete stale runs before the final commit.
- Keep this section updated if the user redefines the plan. Remove the
  `[ACTIVE]` tag from the BTDM tracker entry only when ADV+GATT both PASS.

## Project Overview

A Fast ESP32 (Xtensa LX6, dual-core) emulator/simulator. The CPU is executed by a
Rust-compiled WASM engine; peripherals run natively in Rust (via the same WASM
module) with JavaScript hosts for the remaining subsystems (WiFi analog, clock
source, loader FFI glue). The simulator:

1. Loads the real ESP32 boot ROM and real compiled Arduino/ESP-IDF firmware.
2. Runs the firmware on a WASM-compiled Xtensa core engine in a JS host.
3. Emulates peripherals (UART, GPIO, SPI, I2C, timers, flash, WiFi, etc.) in JS,
   reached from WASM via MMIO FFI handlers.
4. Streams UART output through a SharedArrayBuffer ring buffer to a host test
   script, optionally in parallel workers.

**Current engine state:** WASM is the ONLY engine. The JS CPU interpreter and
`'both'` (lockstep) mode were removed on `79c2add`. The JS `XtensaCore` classes
remain ONLY as proxies that expose WASM core state to JS peripherals — they never
execute instructions.

**Current chip state:** ONLY ESP32 (ESP32C3/RV32 support was deleted). Do not
re-introduce it.

## Architecture

```
tests/*.mjs  ──►  src/sab/MultiSimulator.js   ──►  src/sab/worker-entry.js (worker thread)
                     │ allocates SharedArrayBuffers        │ creates ESP32 chip
                     │ spawns N workers                     │ loads boot ROM + WASM engine
                     └── polls debug SAB + UART ring        └── blockingCommandLoop()
```

Key files:

| File | Purpose |
|---|---|
| `engine-wasm/` | Rust crate → compiles to `src/engine/esp-xtensa/esp_engine_wasm.wasm` via `build-wasm.ps1` |
| `src/engine/esp-xtensa/wasm-loader.js` | Loads WASM, wires MMIO FFI, PTE overrides for all native peripherals (see the tracker: SHA/AES/UART/I2C/SPI/TIMG0-1/GPIO/IO_MUX/FRC/EFUSE/SYSCON/TWAI/RSA/RTC/LEDC/PCNT/RMT/I2S/SDMMC/DPORT/RNG/WiFi analog+MAC/SDIO slave/FE/MCPWM0-1/UHCI0-1/EMAC/SWEEP), `mapAddress` page cache |
| `src/peripherals/esp32/esp32.js` | ESP32 chip class: peripherals, MMU, `step()` (runs `_wasmCores`), `loadWasm()` redirects JS cores to WASM SAB state |
| `src/engine/esp-xtensa/xtensa-core.js` | Facade only — no instruction execution |
| `src/sab/worker-entry.js` | Worker entry: `applyBasicSetup`, WASM load + reset + re-seed MMU/re-hook UART, `blockingCommandLoop`, `writeSABState` (time-based, every 2s) |
| `src/sab/worker-proxy.js` | `SimulatorWorker` (single worker + UART ring poll) |
| `src/sab/MultiSimulator.js` | N parallel chip nodes, UART ring splitting, debug state compare |
| `src/peripherals/common/uart.js` | UART with `onTX` callback → `uartWriteByte` → ring buffer |

### MMIO / FFI flow

- Rust engine traps MMIO read/write → calls JS FFI (`mmio_read`/`mmio_write`, `map_read`/`map_write`, `js_interrupt`, etc.).
- Nearly all peripherals are native via PTE overrides (see the tracker); the JS surface is now limited to the `SimulationClock` event-loop driver (cycle advancement via `skipToNextEvent` — clock VALUES are computed in-WASM by the self-timed core, see the tracker), loader FFI glue, and the zero-traffic `map_read`/`map_write` FFI fallback (GDB flash writes + the never-touched DROM0 window). The MMU table window (0x3FF10000–0x3FF14000, `_memRegions[5]`) is PTE_TYPE_RAM — CPU accesses land directly in the shared WASM linear memory with no FFI (verified: wasm-bench ROM boot does the full MMU setup with Total MMIO calls: 0). An old Rust fast-path for the MMU page was tried and REVERTED (it misrouted the reset-time flash window and crashed boot ROM at PC=0x40000300) — the current flash fast-path avoids that by only routing windows 0x3F40-0x3F7F and 0x400C-0x4040, leaving window 0 (boot ROM 0x40000000) on the chipROM PTE_RAM region.
- Page cache: `_mapCache` in `wasm-loader.js` keyed by `page = addr >>> 12`.
- Constants: `NATIVE_HANDLER_FLAG = 0x80000000`, `HID_INVALID_MEM = 10` (native_mmio_read returns 1→0xFF, 2→0xFFFF, else 0xFFFFFFFF).
- Boot ROM completion seen at PC `0x4000fddf`. Boot path 0x4000FC90–0x4000FDE9 is NORMAL code — do not re-diagnose it as an "error loop".

## Building the WASM engine

```
powershell -File build-wasm.ps1
```

Requires Rust with `wasm32-unknown-unknown` target. Output: `src/engine/esp-xtensa/esp_engine_wasm.wasm`.

## Compile server (required for most worker tests)

Arduino firmware compilation happens on an external server:

- URL: `http://localhost:5000` (`/api/compile/start`, `/api/compile/status/{id}`)
- Replace `fqbn: 'esp32:esp32:esp32'` for ESP32.
- Poll status until `success` (binary_content is base64).

Test `test-worker-wifi.mjs`, `test-worker-web.mjs`, `test-worker-webserver.mjs`
also need the WiFi gateway server (`openhw-studio-gateway/openhw-gw`, static
forward 8080 → ESP32:80) — start it before those tests and skip them if it
isn't running.

## How to run tests

Always choose the WASM engine:

```powershell
$env:ENGINE = "wasm"
node tests/test-worker-i2c.mjs        # example
```

Worker test family (needs compile server on :5000):

```powershell
node tests/test-worker-gpio.mjs
node tests/test-worker-uart.mjs
node tests/test-worker-spi.mjs
node tests/test-worker-i2c.mjs
node tests/test-worker-i2s.mjs
node tests/test-worker-pwm.mjs
node tests/test-worker-rmt.mjs
node tests/test-worker-twai.mjs
node tests/test-worker-timer.mjs
node tests/test-worker-timer-freq.mjs
node tests/test-worker-rtc-wdt.mjs
node tests/test-worker-deepsleep.mjs  # REAL deep-sleep cycles: 3 boots, RTC_DATA_ATTR survives, cause 0->4->4
node tests/test-worker-flash-persist.mjs  # NVS+raw flash across host flash save/reload (3 runs)
node tests/test-worker-sdmmc.mjs
node tests/test-worker-flash.mjs
node tests/test-worker-spiffs.mjs
node tests/test-worker-ota.mjs
node tests/test-worker-part.mjs
node tests/test-worker-default-pt.mjs
node tests/test-worker-analog.mjs          # analogRead from analogInputs volts (needs server)
node tests/test-worker-proxy.mjs          # uses top-level firmware.bin
node tests/test-worker-proxy-esp32.mjs   # larger peripheral test
node tests/test-worker-buttons.mjs       # dev-board RESET (EN) + BOOT (GPIO0) buttons (needs server)
node tests/test-worker-esp32-cam.mjs     # ESP32-CAM module: PSRAM + OV2640 camera (needs server, PSRAM=enabled build)
```

ESP32-CAM is fully working like ESP32 (same silicon + board preset
`board: 'esp32-cam'` = 4MB flash + 4MB PSRAM + AI-Thinker pinout, see
`src/boards/esp32-cam.js` + `docs/esp32-cam.md`): `test-worker-esp32-cam`
PASSES (PSRAM_FOUND=1, PSRAM_HEAP=PASS, CAM_FB=38400 CAM_DATA=PASS) and the
cross-section gpio/uart/spi/i2c/timer/pwm/analog re-runs on-board all PASS
via `tests/board-hook.mjs` (`node --import ./tests/board-hook.mjs
tests/test-worker-gpio.mjs`). Dev-board buttons: `pressResetButton()`
(EN tap = full `chip.reset()`) + `pressBootButton(held)` (GPIO0 level +
BOOT strap bit4) + `holdBootAndReset()`, wired as worker CMD 22/23
(mid-run safe) — covered by `test-worker-buttons.mjs` (BOOT #1→#4 across
RESET tap, BOOT hold + RESET, release + RESET).

Known: `test-worker-bt.mjs` now PASSES (2026-08-16) — see the BT entry below; BT was a startup-memory-release artifact + one unmapped register page, not missing baseband emulation.

No-compile-server tests (pure JS, no external deps):

```powershell
node tests/wasm-bench.mjs                # boots ROM in 5 batches (~20K inst), measures MMIO counts
node tests/test-wasm-standalone.mjs    # 1,000,000 WASM steps, expects PASS
node tests/test-wasm-worker-init.mjs   # worker-path init repro (no PASS assertion, diagnostic)
node tests/check-wasm-boot.mjs         # wasm boot check (needs server)
```

The canonical firmware test (both-Engine removed → WASM only):

```powershell
node tests/test-real-firmware.mjs      # full ESP32 peripheral firmware, WASM engine only
```

## Commit / push conventions

- Repo: `https://github.com/danish9661/folk-esp32-emu`, remote `origin`.
- Force-push needed: branch `main` on remote may be the current HEAD.
- `git push --force origin HEAD:main` after committing.

## Known bugs / tracker

- **[DONE] Native SPI** (commit 4fc97f4): SPI1/0/2/3 routed through the Rust port (`HID_SPI`). Fixed `FieldDesc::new(reg, shift, width)` → `mask = (1 << width) - 1` (JS `createFieldDescriptor` parity — JEDEC cmd decode, `memspi: no response`), `flash_command` param order `(cmd, addr, send, recv)` (JS parity — read branch used `send=0`, app crashed in `esp_core_dump_flash` early init), and the dead `EventTag::EfuseCmdDone` placeholder → `js_spi_flash_erase_done` clock-event bridge (`native_spi_flash_erase_done` clears WIP; `fire_pending()` is never called, so EventTag-scheduled callbacks never fire).
- **[DONE] Rust flash fast-path** (PTE_TYPE_FLASH): flash-cache windows 0x400C0000–0x40400000 resolve opcode/data reads entirely in Rust — a 4MB flash mirror lives in WASM linear memory at `FLASH_DATA_OFFSET` (initialized from JS flash, kept in sync via `js_spi_flash_set_byte`), MMU-table values are read from the linear-memory `mmuTableMemory` mirror per access (`mmu_entry`, core 0 = PRO table, core 1 = APP table at `+MMU_APP_TABLE_DELTA`), and the static MMU window index is carried in the PTE `data` field (`esp32.js` `buildPageTable` loop). JS still owns DROM0 (0x3F800000) and flash WRITES (`map_write` fallback preserves `ReadonlyMemory` semantics).
  - **Page-crossing bug fixed**: `read_opcode_u8` reads the +2 byte of a 3-byte opcode, which can land in the NEXT 4KB page with a different PTE entry (e.g. opcode at 0x400EFFFF, byte at 0x400F0001 → windows 0x400E/0x400F, MMU entries 78/79). The opcode page cache is keyed to the fetch page, so the stale cached `data` selected the wrong MMU entry → wrong byte → IllegalInstruction (symptom: Guru Meditation at PC=0x400effff in the app's `rtc_init`, `or a10,a10,a11` decoded as 0x01aab0). Fix: re-resolve `page_table_entry(addr >> 12)` when the pages differ (JS `mapAddress` re-resolves per access — that's why the JS fallback worked).
  - **Native timer events in the busy loop** (`worker-entry.js`): `native_timg0_process_events`/`native_frc_timer_process_events` now also run every 512 steps while cores are busy (previously only in the idle path), so alarm interrupts fire while the CPU runs — closer to hardware and it removes the flash-path amplification of the CPU1 interrupt-WDT trips in `test-worker-proxy-esp32` (43 → 15 panics, parity with the pre-fast-path engine).
- **[DONE] Native AES** (2026-08-11): the Rust AES-ACCEL (0x3FF01000) handler + PTE are enabled and the AES test passes — root cause of the old freeze was NOT timing: `native_peripheral_init` never called `init_aes()`, so `AES` (the `static mut Option<AesPeripheral>`) stayed `None` and the firmware's first register access hit `aes()`'s `expect("AES not initialized")` → Rust panic → the `#[panic_handler]` was `loop {}` → the "hang" (both cores parked, SAB frozen — the panic was silent). Fix: `init_aes()` added to `init()` in `native_mmio.rs`. Verified: full `fw-proxy32.bin` peripheral suite prints `=== ALL TESTS PASSED ===` (GPIO, INT, PWM, SPI, I2C, TIMER, RTC, WDT, RNG, SHA, AES, RTCIO, ADC, SERIAL) at ~60M cycles with NO WDT panics.
- **[DONE] Panic handler now reports** (`engine-wasm/src/lib.rs`): `#[panic_handler]` formats the `PanicInfo` into a 512-byte buffer and emits it via `js_log_str` (`[WASM] [WASM PANIC] panicked at '...'`) before looping — silent Rust-panic hangs are now diagnosable from worker/diag logs.
- **[DONE] TIMG0 PTE re-enabled** (`wasm-loader.js` `setPTE(0x3FF5F000, HID_TIMG0)`): the native TIMG0 port (parity work in `timers.rs`, `_handler63`/WAITI idle support in `exports.rs`) now boots the full peripheral firmware — the old "PTE corrupts firmware" failure (commit 39c262a) is resolved.
- **[DONE] Native GPIO + IO_MUX + FRC1** (2026-08-11): PTE overrides added for GPIO (0x3FF44000, HID_GPIO), IO_MUX (0x3FF49000, HID_IO_MUX) and FRC1 (0x3FF47000, HID_FRC_TIMER). Missing pieces vs the AES bug: `native_gpio_init` was never called by the loader (the Rust controller was dead code) — added the init call + `seedNativeGpio()` (JS→Rust bridge: per-pin input levels + strap value pushed via `native_gpio_set_pin_input`/`native_gpio_set_strap`; called at loader init and after worker-entry's post-reset pinInputs re-apply). `native_mmio.rs` `iomux_write` now updates the Rust `GpioPin` mux/pull/IE fields + notifies `GpioController::mux_config_changed` (JS parity with i2c-i2s.js IoMuxPeripheral). Reset hooks `_nativeGpioReset`/`_nativeFrcReset` added alongside `_nativeUartReset` in `esp32.js reset()`. Verified: `test-worker-proxy-esp32` (uses pinInputs) + full suite PASS=20 FAIL=0; standalone fw-proxy32 diag ALL TESTS PASSED.
- **[DONE] Native RMT** (2026-08-12): PTE override for RMT (0x3FF56000, HID_RMT=18); the `rmt_rng_math.rs` `RmtPeripheral` port (previously only `RsaPeripheral` from that file was wired) is now routed in `native_mmio.rs` (init/reset exports, `RMT_CONFIG` from register-data.js RmtRegisterMap — ESP32 v1: CHn_CONF0=32 stride 8, INT_RAW/ST/ENA/CLR=160/164/168/172, CARRIER_DUTY=176, TX_LIM=208, APB_CONF=240, no SYS_CONF, irq 47, ch0_matrix_out 87 — strided `RMT_SEED` reset values, size-aware region helpers in the read/write dispatch). Reset hook `_nativeRmtReset` added in `esp32.js reset()`. Verified: `test-worker-rmt` (config+driver install/uninstall) PASSES; full suite **PASS=21 FAIL=0 SKIP=4** (2026-08-12).
- **[DONE] Native LEDC + PCNT** (2026-08-12): PTE overrides for LEDC (0x3FF59000, HID_LEDC=16) and PCNT (0x3FF57000, HID_PCNT=17); the `ledc_pcnt.rs` parity port is now wired in `native_mmio.rs` (init/reset exports, `seed_periph_base_strided` for the Esp32FullResetValues blocks from register-data.js:1052/1064, size-aware region helpers in the read/write dispatch). Fixed `LedcTimer::write_register`: field shifts were ALL `<<0` (PARA_UP=26/PAUSE=23/RST=24/TICK_SEL=25 per `LedcTimerFieldMap`), CLK_DIV read as `(val>>5)&0x3FFFF`, ls-timer clock-source polarity (JS: ls parent = ref if TICK_SEL set else apb — the port had it inverted), PAUSE-only handling (JS RST branch is a no-op; the port reset+disabled on RST and force-enabled otherwise), and `set_output(signal, enable, value)` arg order in channel conf0 / `update_alarms` / `handle_alarm_fired` (port had enable/value swapped). Reset hooks `_nativeLedcReset`/`_nativePcntReset` added in `esp32.js reset()`; `0x3FF6C000`/`0x3FF5CC00` stay JS EmptyPeripherals (no PTE). Verified: `test-worker-pwm` PASSES; full suite **PASS=21 FAIL=0 SKIP=4** (2026-08-12).
- **[DONE] Native I2S** (2026-08-12): PTE overrides for I2S0 (0x3FF4F000) and I2S1 (0x3FF6D000), HID_I2S=19; the `i2c_i2s.rs` `I2sPeripheral` port is now routed in `native_mmio.rs` (init/reset exports, `I2S_CONFIG` from register-data.js I2sRegisterMap+I2sFieldMap, irqs 32/33 = `InterruptEnum.I2S0_INT/I2S1_INT`, SpiDmaResetValues seed applied to BOTH bases per Esp32FullResetValues, size-aware region helpers in the read/write dispatch). Two dead-stub fixes were required: (1) `DmaDescriptorChain` (i2c_i2s.rs) is now a REAL implementation — descriptors/buffers read through the page-cached JS `map_read`/`map_write` FFI (JS parity with `this.cpu.mapAddress(addr, coreIdx)` — the same path the JS `I2sPeripheral` uses; `start` ORs `dmaBase=0x3FF00000`, `next()` follows the `ptr+8` link, `set_length` masks 0x7F000FFF); (2) the stubbed `schedule_tx_processing`/`schedule_rx_processing` now use the `js_i2s_schedule_tx`/`js_i2s_schedule_rx` clock-event bridges (loader `createEvent` → `native_i2s_clock_tx/rx` exports, JS parity with `txClockEvent.schedule(1e4)`), plus an optional `js_i2s_tx_data` onTxData hook (`native_i2s_set_tx_hook`). Reset hook `_nativeI2sReset` added in `esp32.js reset()`. Verified: `test-worker-i2s` PASSES — covers driver install, a real `i2s_write` DMA TX (ESP_OK — would be `ESP_ERR_TIMEOUT` if the DMA/interrupt bridge regressed), uninstall; WRITTEN=0 is emulator-wide driver bookkeeping, byte-for-byte parity with the JS path (same output on both); full suite **PASS=21 FAIL=0 SKIP=4** (2026-08-12).
- **[DONE] Native SDMMC** (2026-08-13): PTE override for SDMMC (0x3FF68000, HID_SDMMC=20); the `sdmmc.rs` `SdmmcPeripheral` port (previously unwired dead code) is now routed in `native_mmio.rs` (init/reset/cmd_complete exports, irq 37 = `InterruptEnum.SDIO_HOST_INTERRUPT`, size-aware region helpers in the read/write dispatch). Two dead-stub fixes required: (1) `CoreMemoryAccess` for `CpuContext` (descriptor/DMA memory) used the no-op `mem_read_*`/`mem_write_*` — now routed through the page-cached `map_read`/`map_write` FFI (`dma_read_u8`/`dma_write_u8` added next to the I2S `dma_read_u32`/`dma_write_u32` in `xtensa/memory.rs`); (2) `schedule_command_complete` used the dead `EventTag::EfuseCmdDone` queue — replaced with the `js_sdmmc_schedule` clock-event bridge (loader `createEvent` → `native_sdmmc_cmd_complete`, JS parity with `cmdCompleteEvent.schedule(1e3)`). Reset parity: 0x3FF68000 has NO `Esp32FullResetValues` entry (the JS `SdmmcResetValues` list applies only to 0x3FF40000 — the UART0/SDMMC_ALT region), so `native_sdmmc_reset` = class `reset()` only. Verify: `test-worker-sdmmc` now exercises real MMIO (`sdmmc_host_init`/`sdmmc_host_deinit` PASS — previously the firmware only read the compile-time `host.flags`); JS-parity double-checked (PTE disabled → identical output); full suite **PASS=21 FAIL=0 SKIP=4** (2026-08-13).
- **[DONE] Native DPORT + cross-core IRQ fix** (commit a172c35): DPORT routed natively (PTE 0x3FF48000) with `Esp32FullResetValues` seed + RNG/WiFi bridge exports; root-caused a native-only deadlock to a bogus `idx === 3 ? !bit : bit` inversion in the loader FFI `js_dport_cross_core_irq` — JS ground truth `DportPeripheral.writeUint32` (interrupt-efuse.js:298-309) applies NO inversion (`this.cpu.interrupt(ccIRQs[i], !!(1 & tmpVal))`), so the native wake `0xE8=1` became a *clear* → core1 never entered its ISR → both cores spun. Wakes are `CPU_INTR_FROM_CPU_0..3` = irq 24..27 (NOT 34/49 — `[INTR] irq=49` is `I2C_EXT0_INTR`, irq 34 is `UART_INTR`; cross-core goes through the JS matrix, never the Rust `InterruptMatrix`). Fix: wasm-loader.js uses `this.esp32.interrupt(irqs[idx], bit)`; freeze-walk now byte-identical to JS (`pc0=pc1=0x40085cfe`). Debug harnesses `tests/freeze-walk.mjs`/`tests/dump-pc.mjs`/`tests/compile-variant.mjs` committed; 4MB `tests/fw-proxy32.bin` build artifact NOT committed.
- **[DONE] Native RNG + WiFi analog** (commit a14be29): dormant Rust dispatch already routed `HID_RNG=1`, `HID_WIFI_ANALOG=22`, `HID_WIFI_MAC=23` (functions carry `#[no_mangle]` directly — no exports.rs re-export needed); enabling = `setPTE` lines, no rebuild. RNG (0x3FF75000) + WiFi analog (0x3FF4E000) PTEs enabled there. NOTE: the WiFi MAC PTE was disabled at b8d02eb (driver TX hang) and re-enabled at b637a5f with the full TX/RX bridge set — see that entry.
- **[DONE] Native SDIO slave + FE** (commits 7a1d391, 76c0c60): SDIO slave (0x3FF58000, HID_SDIO_SLAVE=25) — `SdioSlavePeripheral` (sdmmc.rs:107) was unwired dead code; reads 0xffffffff at offset 64, else register file; Esp32FullResetValues 0x3ff58000 block (13 entries) seeded in init/reset. FE front-end (0x3FF46000, HID_FE=26) — `StubPeripheral` parity: offset 124 (IQ_EST)/24 → 0xffffffff, 12 → 114688, 128 → 4112, else zeroed file. **Bug caught**: `seed_register` is TwaiFifo-only (twai_fifo.rs:144) — the first SDIO-slave commit used it and NEVER COMPILED (shipped stale wasm); fixed with `PeripheralBase::write_uint32(base+off, val)`. Always verify the wasm file is freshly rebuilt (check mtime) before testing.
- **[DONE] Native MCPWM0/1** (commits 10e3f89, d76f9b9): TRM-based (no JS parity — pages were EmptyPeripheral). MCPWM0 0x3FF5E000 + MCPWM1 0x3FF6C000 (HID_MCPWM=27), 4KB file per unit with the ESP32 companion-register pattern (writes to `*_CLR` clear / `*_EN`/`*_SEL` set bits in the main reg; reads echo the main reg; 17 companion groups incl. INT_RAW/INT_CLR), zeroed reset except MCPWM_DATE (0x3FC=0x16031200). IDF driver path is all writes (duty/freq cached in RAM) so plain RW suffices — `test-worker-mcpwm` uses `driver/mcpwm.h` (legacy API): gpio_init/init/set_duty/start/stop/get_duty/set_frequency on BOTH units (duty readback 25.0/33.0). Note: Arduino 3.3.10 is IDF 5.x — `mcpwm_fault_enable` no longer exists (renamed `mcpwm_fault_init`), removed from the test.
- **[DONE] Native UHCI0/1** (commit 9113da9): 0x3FF54000 + 0x3FF4C000 (HID_UHCI=28), TRM-based. IDF naming per `DR_REG_UHCI0_BASE=0x3ff54000`, `DR_REG_UHCI1_BASE=0x3ff4c000` (INVERTED vs TRM numbering; matches the JS UhciAltBaseAddr2/3 aliases). INT_RAW (0x4)/INT_ST (0x8) read-only, INT_CLR (0x10) clears both, DATE (0xFC) seeded. `test-worker-uhci` = raw MMIO via `soc/uhci_reg.h` REG_WRITE/REG_READ on both units.
- **[DONE] Native WiFi MAC TX/RX + PTE re-enabled** (commit b637a5f, 2026-08-15): the 0x3FF73000 (HID_WIFI_MAC=23) PTE — disabled at b8d02eb because the driver TX path hung — is enabled again. Three porting bugs found and fixed (all JS-parity): (1) `wifi_mac_write_region` masked the address with `addr & 0xFFC` (12 bits), so `write_u32` computed a WRAPPED offset (0x84 - 0x3FF73000 = 0xC008D084) and no case arm matched — every native MAC write was silently dropped, registers stayed 0, driver spun on MAC_EVENT (0x3C48) forever; fixed with `addr & !3` (TWAI/RSA/read_region pattern). (2) `REG_COUNT` was 256 (1KB register file) vs JS `PeripheralBase.memory` 4096 bytes — DMA_TXBUF/MAC_CTRL at 0xC00+ fell outside: writes ignored, reads returned 0, driver's write-readback failed and it never issued the bit-30 TX-start write (DMA_TXBUF0 = 0x6cff84 then 0x0 forever); `REG_COUNT = 1024`. (3) `dma_read_u8`/`dma_write_u8` pass size 1/2 to the `map_read`/`map_write` FFI but the loader only knew `readUint8/16/32` → `readUint1 is not a function` crash once DMA descriptor reads started; loader now maps 1→8, 2→16. Plus the TX/RX bridge set (JS parity with the JS WifiMacPeripheral): `on_tx` → `native_wifi_tx_frame_bridge` (static `WIFI_TX_SCRATCH` + `js_wifi_send_frame` → `chip.wifi.onTX`, mirroring the synchronous onTX of the JS DMA_TXBUF arm); `dma_base` fixed to 0x3FF00000 (cpu.dmaBase); `native_wifi_mac_rx_done` export called from JS sendFrame tail mirrors RX event (0x1000024) + RX_LAST_DSCR into the native register file; `js_wifi_rx_base`/`js_wifi_mac_ctrl`/`js_wifi_rx_ctrl` bridges keep JS rxBuffer + enabled/rxEnabled in sync (JS RX path drops frames unless enabled && rxEnabled). Verified: `test-worker-wifi` scan PASSES with PTE on, `test-worker-web` reaches "Web Server ready on port 80", webserver manual curl HTTP 200 (944B) via gateway, full suite **PASS=25 FAIL=0 SKIP=4**.
- **[DONE] Native EMAC** (commit 12b6eab): MAC 0x3FF69000 + DMA 0x3FF6A000 (HID_EMAC=29), TRM-based; register maps extracted from the platform's `emac_mac_struct.h`/`emac_dma_struct.h` (in-order struct fields — the struct `.val` unions DON'T compile in Arduino's C++ mode, so the test uses raw offsets with REG_WRITE/REG_READ). Plain RW, zeroed reset.
- **[DONE] Sweep peripherals** (commit d76f9b9): HID_SWEEP=30 covers the remaining dedicated register-file pages — Secure Boot (0x3FF04000), I2C config (0x3FF4B000), SLCHOST (0x3FF55000), Flash Encryption (0x3FF5B000), PID Controller per-CPU (0x3FF1F000). Plain 4KB RW, zeroed reset. `test-worker-sweep` pokes all five; Secure Boot needs `DPORT_REG_WRITE`/`DPORT_REG_READ` (it sits inside IDF's DPORT window 0x3FF00000–0x3FF13FFC per `soc/soc.h` `IS_DPORT_REG`, which static-asserts REG_WRITE away).
- **[STALE-NOTE]** LEDC/PCNT commit says `0x3FF6C000`/`0x3FF5CC00` stay JS EmptyPeripherals — 0x3FF6C000 is now native MCPWM1, 0x3FF5CC00 is now native HID_STUB_ZERO (see the 4d40024 entry). Note the BB base is 0x3FF5D000 (RmtAltBaseAddr in register-data.js), NOT 0x3FF63000 — older notes that say 0x3FF63000 are wrong.
- **[DONE] Clock events → native EventQueue** (commit e3ca5f1): UART RX timeout/int-check, I2S TX/RX processing, SDMMC command-complete, SPI flash erase-done, RTC sleep wakeup, efuse CMD-clear and ADC sample-done were scheduled through JS `SimulationClock` bridges (`js_schedule_*` FFI). They now schedule `EventTag`s on the single shared `EventQueue` (hoisted out of `CpuContext::dummy()` in spi_syscon.rs — the dummy's queue already persisted across `make_ctx()` calls) with deadlines in 80MHz APB ticks, drained by the new `native_process_events()` export pumped from worker-entry alongside the TIMG0/1 + FRC pumps. New tags: `I2sTx/I2sRx/SdmmcCmdComplete/SpiFlashEraseDone/RtcSlowWakeup/AdcDone`. RTC wakeup converts rcSlow (32.768kHz) targets → APB deltas ×2441 via new `js_rtc_slow_ticks()` FFI. **CCOMPARE stays on the JS clock** (CCOUNT/125MHz cpu-clock conversion mismatch documented in exports.rs `check_ccompare`). Nanos→APB conversion: `ns * 80 / 1000` (12M/16M SPI-erase delays → 960K/1280K ticks; 634400ns efuse → 50752). The old `js_schedule_*` externs/loader bridges were REMOVED (commit 3d59882, log ### 25) — rustc DCE had already dropped them from the wasm import section (verified 0 imports); only the dead declarations + inert handlers remained.
- **[DONE] DROM low window native** (commit d650257): `buildPageTable` now sets PTE_TYPE_FLASH for the DROM low window (0x3F400000–0x3F800000, MMU table idx 0–63, 64KB granularity, flashMMUMap keys 0x3F40–0x3F7F) — Rust `flash_linear` is window-agnostic (PTE data carries the static MMU index; `entry<<16 | addr&0xFFFF`), so no Rust changes were needed. Measured: 186K JS `map_read` fallbacks for the window → 0. The 0x3F800000 psram-backed DROM0 window was NEVER read or written by firmware (0 accesses over 58M cycles) — stays on the JS fallback (dead path). 0x4040+ windows remain isolated (MMU entries 128+ exceed the 4MB linear-memory flash mirror).
- **[DONE] Final JS-surface audit** (2026-08-13): instrumented `map_read`/`map_write` FFI across the full proxy-esp32 suite (58M cycles) — the ONLY remaining non-WiFi fallback traffic is 26 reads + 7 writes to 0x3FF81FF0–0x3FF81FFC (boot-ROM scratch on a dead region → `invalidMem` semantics: reads 0xFFFFFFFF, writes dropped; firmware ignores it). Everything else (MMU table, DROM, clock events, interrupt matrix, all peripherals) is native. Remaining JS by design: the `SimulationClock` host time source, loader FFI glue, and the 0x3FF73000 WifiMac JS peripheral (survives only as a bridge-synced mirror of the native register file). The WiFi stub pages from this audit (BB 0x3FF5D000, NRX 0x3FF5CC00, WiMac/2 0x3FF74000) and the scratch page 0x3FF81000 are now native too (commit 4d40024).
- **[DONE] Interrupt matrix → native** (commit 934943c): `esp32.interrupt()` calls `native_interrupt` when wasm is loaded (JS `intMatrix` kept as non-WASM fallback); Rust `InterruptMatrixPeripheral` instances wired into the DPORT native handler (core0 STATUS0=236/240/244, FIRST_INTR_MAP=260..536; core1 STATUS0=248/252/256, FIRST_INTR_MAP=536..812; `MAX_INT=69`, `INT_CFG4=0xdffe773f`, `INT_CFG5=0xdffe773f & ~0x50400400`); `interruptsUpdated` → `CoreState::int_set_clear` (JS `intSetClear` parity); `native_dport_init` calls `native_int_matrix_init`, `native_dport_reset` resets matrices; cross-core DPORT wakes still bridge via `js_dport_cross_core_irq` but converge on the native matrix.
- **[DONE] `test-worker-proxy-esp32`** (2026-08-11): now PASSES — the CPU1 interrupt-WDT trips are gone after the AES/TIMG0 fixes. Full suite: **PASS=25 FAIL=0 SKIP=4** (bt + wifi/web/webserver gateway), verified 2026-08-13 after clock-events + DROM commits. Run: `./tests/run-worker-tests.sh --skip-bt --skip-wifi` against the local compile server (`tests/compile-server.mjs`, arduino-cli esp32:esp32:esp32 3.3.10).
- **map-read/write page cache**: multi-region pages (e.g. 0x3FF48 — RTC_CNTL 0x3FF48000 + SENS/ADC 0x3FF48800 inside one 4KB page) are detected (`multi`) and never cached — the first region would shadow the rest.
- **[STALE-DOC]** Compile error messages about `esp_flash_read` — fixed in `test-worker-part.mjs`/`test-worker-default-pt.mjs` (include `<esp_flash.h>`).
- **[DONE]** BT controller-init (`test-worker-bt`) passes — see the BT entry below. Full BTDM advertising (LE scan/advertise) is unsupported (requires the ESP32 LL/baseband).
- **[DONE] WiFi gateway server** (verified 2026-08-16): `openhw-studio-gateway/openhw-gw` (static forward 8080 → ESP32:80 — NOT 8081; that was a stale note) — start before `test-worker-wifi`/`-web`/`-webserver`; all three verified through it on the current build (scan PASS, web "ready on port 80", webserver curl HTTP 200, 944B).
- **[DONE] Zero-traffic polish** (commit d8ec98a, 2026-08-18): the last JS-fallback MMIO traffic measured by the FFI-counts tool (1395 calls / 58M cycles) is native. Three DROM0-alias pages gained native PTEs: `0x60000000` (UART0/SDMMC_ALT alias — the UART driver configures through the window, 1311 writes), `0x60010000` (UART1 alias), `0x6002E000` (MCPWM0 alias). New Rust `normalize_drom0_alias` (native_mmio.rs) maps the 0x60000000–0x60100000 window back to the peripheral bus (`addr - 0x200C0000`) at the top of `native_mmio_read_inner`/`native_mmio_write` so the region handlers' `addr - base` offset math works (analog/MAC internal `(addr & 0xFFF) | BASE` normalization is idempotent). `write_dma_buffer` (wifi_analog.rs) early-returns when `cpu_val == 0` — the 42 `dma_read_u32(0)` reads from `native_wifi_mac_rx_frame` (beacon → RX with an unarmed RX descriptor) are skipped with identical observable behavior. Verified: diag-ffi-counts = mmio 0, map_read 0, map_write 3 (GDB flash), js_log_str 0; js_interrupt rose 876 → ~3529 because UART0 interrupts now enter the matrix via the counted FFI path instead of the uncounted JS-copy `cpu.interrupt` — delivery equivalent. Battery green incl. full suite PASS=25 FAIL=0 SKIP=4 + wifi scan PASS.
- **[DONE] Remaining JS surface: stub pages + scratch + flash drop + DMA** (commit 4d40024, 2026-08-16): the WiFi stub pages that were JS EmptyPeripherals (BB 0x3FF5D000, NRX 0x3FF5CC00, WiMac/2 0x3FF74000) are now native via `HID_STUB_ZERO=32` (storage-free: read 0 / drop writes — parity with an untouched EmptyPeripheral; these pages have zero firmware traffic), and the boot-ROM scratch page 0x3FF81000 via `HID_INVALID_MEM=31` (JS invalidMem parity: reads 0xFF/0xFFFF/0xFFFFFFFF by size, writes dropped). Flash-cache window writes now drop natively (ReadonlyMemory parity) unless GDB flash writes are active — `js_flash_write_override` FFI bridge reads the JS `ReadonlyMemory.override` flag set by gdb-session.js `M` commands. DMA descriptor/buffer access (`dma_read_u32/u8`, `dma_write_u32/u8`) resolves the page table natively (RAM/flash/MMIO/JS-fallback) instead of the `map_read`/`map_write` FFI. **CRITICAL LESSON (verified by bisect): adding new `static mut` register files shifts the linker's static-data layout and breaks boot (boot ROM hangs at PC=0x40007b86 with the DROM low window falling to JS map_read — the diag harness shows `0x3f401R=13`). Storage-free handlers carry no statics, so the layout never shifts. NEVER add register-file statics without re-verifying `tests/diag-maphist.mjs` (runs the pre-built `tests/fw-proxy32.bin` directly, no compile server).** Verified: diag PASSED=true with ZERO JS map traffic; full worker suite PASS=25 FAIL=0 SKIP=4; wifi scan PASS, web "ready on port 80", webserver curl HTTP 200 (944B).
- **[DONE] Memory layout move + native WiFi AP (NativeInternetAP port)** (2026-08-16): the JS-owned regions moved above the Rust static-data zone — `STATIC_ZONE = 0x400000` in `wasm-memory-layout.js`/`state.rs` (derived) and the hardcoded copies in `constants.rs`/`esp32.rs` (PAGE_TABLE_OFFSET=4194304, REGION_TABLE_OFFSET=12582912, RAM_DATA_OFFSET=12583168, FLASH_DATA_OFFSET=18874624; the loader's runtime MATCH check + diag confirm). New statics (register files, buffers) can now grow safely up to the 4MB zone line instead of gambling on linker placement (4d40024 lesson). The pre-existing `NativeInternetAP` port (`engine-wasm/src/peripherals/common/wifi_bridge.rs`, previously unwired dead code) is now ACTIVE: `WifiStatus`/`NativeInternetAP` statics + a 2KB `WIFI_AP_SCRATCH`; `push_output`/`send_ethernet`/`connect_gateway` now call the FFI synchronously (JS parity with `onRxCb`/`sendEthernet`/`connectGateway`); exports `native_wifi_ap_init(ssid_ptr,ssid_len,bssid_ptr,bssid_len,channel,private)`, `native_wifi_ap_scratch()`, `native_wifi_ap_handle_tx(len, channel)` (0→None), `native_wifi_ap_eth_rx(len)`, `native_wifi_ap_send_beacon()`, `native_wifi_ap_get_status(ptr)` (7×i32 LE: state/tx/rx/probes/clients); FFI imports `js_wifi_ap_rx_frame`/`js_wifi_ap_send_eth`/`js_wifi_ap_connected` (loader hooks `_wifiApRxFrame`/`_wifiApSendEth`/`_wifiApConnected`). worker-entry.js: `setupNativeWifiBridge()` runs AFTER `loadWasm`+`reset` (step 7 of the re-apply section — it needs `chip._wasmLoader`, so it can't live in applyBasicSetup); gateway WebSocket (`ws://127.0.0.1:5085/api/network-gateway`), pcap records and BOARD_IP:/PORT_FORWARD: strings stay in JS; the JS status mirror syncs from Rust via `native_wifi_ap_get_status` (called in writeSABState + CMD_GET_WIFI_STATS); beacon = JS clock event (102ms sim-time) calling `native_wifi_ap_send_beacon`; TX frames flow `js_wifi_send_frame` → onTX → scratch copy → `native_wifi_ap_handle_tx`; eth RX flows socket → scratch → `native_wifi_ap_eth_rx` → `js_wifi_ap_rx_frame` → `chip.wifi.sendFrame`. **Gotcha: `socket.send()`/pcap storage need a COPY of the wasm-memory view (`new Uint8Array(mem, ptr, len).slice()`) — undici rejects SharedArrayBuffer-backed views (`ArrayBuffer: SharedArrayBuffer is not allowed`).** Verified: diag PASSED=true, zero JS map traffic; full suite PASS=26 FAIL=2 (web/webserver are long-runners that hit the 300s per-test timeout BY DESIGN — keep serving for live curl; verified separately) SKIP=1 (bt); wifi scan PASS; web "ready on port 80"; webserver curl HTTP 200 (944B).
- **[DONE] BT controller init (test-worker-bt)** (2026-08-16): the 0x103 failure was NOT an emulator bug — Arduino core `initArduino()` calls `btMemRelease(2)` + `btMemRelease(1)` when the weak `btClassicInUse()`/`bleInUse()` return false (test sketch links no BT library) → `esp_bt_controller_rom_mem_release` ANDs `btdm_dram_available_region[].mode` (0x3ffc0270, an initialized .data static in `components/bt/controller/esp32/bt.c`; the emulator's .data copy from merged.bin worked fine) → `esp_bt_controller_init` sees mode==IDLE → ERR 0x103 — identical on real hardware. Test fix: strong `extern "C" bool btClassicInUse/bleInUse` overrides in the sketch (the documented weak-function mechanism) → mode stays 0x3, 0x103 gone. Next hang: ROM `r_ld_read_clock` (0x4003c9e4, `ld/esp32.rom.ld` line 801) sets bit31 of **0x3FF7101C** and spins at 0x4003ca00 until HW clears it — the undocumented BT RF/LD-clock register block at 0x3FF71000 (literals in ROM: 0x71000/0x71010/0x7101c/0x71020/0x71030/0x71040/0x71044/0x71050/0x71060/0x71070/0x71080/0x71200; used by libbt.a too: `coex_bt_callback`, `r_ea_alarm_set`, `r_lld_evt_end_isr`). The page was JS invalidMem (reads 0xFFFFFFFF → bit31 never clears → infinite spin). Fix: `BtRfPeripheral` in `esp32.js` (0x3FF71000, 4KB RW register file, 0x1c seeded 0x4F → clock result 40, bit31 masked off on writes = commands complete instantly) — later ported to Rust as `HID_BT_RF=33` (see the "Port everything to WASM" entry). Verified: `test-worker-bt` PASS (`[BT] init=PASS`, RESULT=PASS); full suite PASS=26 FAIL=2 (web/webserver long-runner timeouts BY DESIGN) SKIP=1 (bt, now skipped only because the suite defaults to --skip-bt). Debug tooling added along the way: `SimulatorWorker.step(count)` + `CMD_STEP=8` in `worker-proxy.js` (note: CMD_STEP lacks the runSimChunk timer pumps, so it can't boot firmware — use run/stop interleave + readMemory for diag; see /tmp/opencode/diag-bt.mjs pattern).

- **[DONE] Full BTDM advertising — UNSUPPORTED (requires LL/baseband)** (resolved 2026-08-22): `test-worker-bt` (controller-init) passes (`[BT] init=PASS`, `enable=PASS`), but the full BTDM advertising path `ble-probe4.mjs` (cached `ble-probe-fw3.bin`) hangs in `esp_bluedroid_enable` with HCI_RESET (`01 03 0c 00`) unconsumed. Root cause (after full ESP32 BT ROM disassembly): **ESP32 BT is split into the LL (baseband, in ROM) and the host (bluedroid, in flash); the LL is NOT emulated.** HCI commands are processed by the LL, which generates the Command-Complete and delivers it to bluedroid via a proprietary VHCI/btrom state machine. Without the LL actually processing HCI_RESET, no synthesized CC is accepted. VHCI tracing established:
  - **Host→LL path WORKS**: host writes HCI_RESET to `host_to_ctrl_buf` (`0x3ffc88c0`; its pointer is at `0x3ffc7a14`). The LL's VHCI ISR reads it: `r 400904cc a=3ffc88e8 v=00000001`, `r 400905ac a=3ffc88e8 v=000c0301` (the LL also copies `0x000c0301` into `0x3ffc88e8` at `400905f6` before parsing), then reads the opcode bytes `0x40090608→03`, `0x4009060b→0c`, `0x40090620→00`. So the LL sees a valid HCI_RESET.
  - **`host_recv` callback IS registered**: the host overwrites a slot in the LL's VHCI function table — `w 40162a63 a=3ffafdb0 v=40162a20` — so `0x3ffafdb0 → 0x40162a20` is the LL→host delivery fn. (The LL's own API table is at `0x3ffafd6c`–`0x3ffafdcc` and `0x3ffc8854`–`0x3ffc886c`, populated with LL code ptrs `0x40013xxx`/`0x4008xxxx`.)
  - **`ctrl_to_host_buf` (`0x3ffc7a18`) is NULL** (LL writes `0x00000000` there; host never sets it) — but delivery goes through `host_recv(data_ptr)`, so this is almost certainly not the blocker.
  - **The LL (ROM) reads HCI_RESET but produces NO response because the LL is the baseband — it is not emulated.** Confirmed by full ROM disassembly of the VHCI delivery path: `host_recv` (`0x40162a20`, flash) is bluedroid's VHCI RX wrapper; it reads the LL controller struct at `*(0x3ffc7100)=0x3ffafd6c` (a function-pointer table) and calls btrom callbacks `cb100=0x4001a674`, `cb108=0x4001a6fc`, `cb4c=0x40014470` plus flash `0x40162ef4`. `cb108` is the only disassemblable opcode dispatcher (`extui a2,0,16` → `blt 8, low_byte → tail`) and **explicitly ignores HCI_RESET** (opcode low byte `0x03 < 8`); it reads only `a2/a3` (opcode/status), never an event buffer. `cb100`/`cb4c` manipulate the LL controller struct at `0x3ffb8360` via function pointers — the LL state machine, not a buffer writer.
  - **The LL RX data buffer is a descriptor chain** (`0x3ffc7a30 → 0x3ffb6e00` → data `0x3ffe0440`, length `@0x3ffb6e14`), not a flat buffer. Delivering via guest-call with the buffer populated still fails because the btrom delivery requires the LL's command-processing state (which only exists if the LL processed the command).
  - **Synthesizing a CC cannot work** (both strategies reduce to re-implementing the LL): (1) guest-call `host_recv` with `(data,len)` args — ignored (reads a VHCI struct, not registers); (2) guest-call `host_recv` with the CC written to `0x3ffe0440` + descriptor length `0x3ffb6e14` — btrom callbacks need LL state; (3) call bluedroid's `btu_hci_msg_process` directly — not locatable without the ESP-IDF bluedroid symbol map.
  - **Conclusion: full BTDM advertising (LE scan/advertise) is UNSUPPORTED.** Keep `test-worker-bt` (controller-init) as the supported scope. The `bt_vhci_log` / `BT_VHCI_TRACE_LEFT` / `[CTRL R/W]` / GATE / READY diagnostic instrumentation was reverted; the tree is clean at HEAD with the working CPU1 fix. (Note: the Xtensa disassembler `xtensa-esp32-elf-objdump` IS available at `~/.arduino15/packages/esp32/tools/esp-x32/2601/bin/` and was used for this analysis.)
  - **[ACTIVE] BTDM re-attack (2026-09-11): the 2022 "LL not emulated" verdict was WRONG — the LL runs, it was starved of interrupts.** The old analysis predates the BT RF models; the landscape shifted. New findings, all measured live with Arduino 3.3.10 (IDF v5.5.4) firmware:
    - `esp_bt_controller_enable(ESP_BT_MODE_BLE)` returned 258 (`ESP_ERR_NO_MEM`) because `cfg.mode` was left at the sdkconfig default BTDM(3) — this build returns NO_MEM (not INVALID_ARG) on mode mismatch (verified by disassembling local `libbt.a` `bt.c.obj`: status!=INITED→0x103, mode-mismatch→0x102). Fix in firmware (not engine): set `cfg.mode` explicitly like Arduino's own `btStartMode()` does. Then `CTL_INIT=0 CTL_ENABLE=0`, controller ENABLES.
    - **Interrupt routing was the wall, not the baseband.** The old code raised `native_interrupt(3,1,1)` = matrix SOURCE 3 = `ETS_BT_MAC_INTR_SOURCE`, which `soc/interrupts.h` marks "will be cancelled" — live DPORT FIRST_INTR_MAP dump shows it routes to a dead CPU-6 line. The firmware actually maps the RivieraWaves LL interrupts **RWBT(src 6)→CPU 25 and RWBLE(src 7)→CPU 25** on PRO core (plus BT_BB(src 4)→CPU 8). Fix: new `bt_raise_ll_irq()` raises sources 6+7 (native_mmio.rs) at all 4 BT sites (alarm FIRE, host-wake HWAKE, `native_bt_rf_process_alarm`, idle-pump BT block).
    - **Level-modeled RW sources storm; real RW IRQs are edge.** The RW status-clear registers are unmodeled so sticky raises re-vectored the CPU-25 ISR 3.9M times (identical pump loop `4008f27c/4008f706/4008f712`), starving the LL task; worse, same-level nested vectors at the ISR epilogue clobbered EPC4 with the `rfi` address itself (measured: PC=EPC4=`0x40083bca`, `rfi 4` loop). Two-part fix: (1) pulse — `BT_IRQ_PULSE` set on raise, cleared at the start of every `native_pump_events()` (covers `native_idle_advance` too, which calls it); (2) edge-ack — `CoreState::take_interrupt` (state.rs) calls new `bt_ack_ll_vector()` when vectoring with CPU-25 pending (safe: the LL ISR demuxes from LL structs at `0x3ffc7100/104`, never from matrix STATUS — verified by disassembly). The take_interrupt hook is inert for all other tests (CPU 25 is only ever pending via BT raises).
    - **Result: `esp_bluedroid_init()` now returns 0** (was stuck in `future_await` for the BTC task); cores idle healthy (PC=`waiti`, sane EPC, no storm, log volume 6603 lines vs 11.7M). `test-worker-bt`/`ble-init` still PASS; bench/standalone/timer/uart/gpio/twai/proxy-esp32/wifi-scan all green.
    - **Current wall (unchanged address, new mechanism): `esp_bluedroid_enable` still waits for the HCI_RESET Command Complete.** Late epoch observed: ROM arms the LD alarm (`0x400557b0` target, `0x400557df` bit31) → FIRE → controller code rings the 0x3FF71030 doorbell (`0x80000007`, HWAKE) — but a 64KB DRAM sweep (`0x3ffc0000-0x3fd00000`) finds NO `01 03 0C 00` (reset) and NO `04 0E 04 01 03 0C 00` (CC) anywhere; the single edge-acked ISR epoch pumps (LL API-table calls at `0x4008f260/0x4008f700`) but never reads any packet buffer, and the VHCI pointer slots (`0x3ffc7a00-18`) hold `0xFFFFFFFF` (written by lib loop `0x400906d2-d8`) with only `0x3ffc7a1c=0x3ffce994`, `0x3ffc7a38=0x3ffb6e00/0x28` live. So the doorbell announces a packet the LL never fetches — host and LL disagree on the mailbox, or the packet copy faulted/went elsewhere. Next lead: ROM delivery callbacks `0x40019feb/0x4001a0dc` DID run (wrote descriptor lens `0x74c→0x79c` at `0x3ffc885e`), and the RX descriptor chain (`0x3ffc7a38→0x3ffb6e00`) is live — trace what the LL posts into C2H vs what `host_recv` (`0x40162a20`) needs. Diag scripts used (in /tmp, recreate as needed): `ble-adv-probe2.mjs` (stepwise init→advertise), `ble-matrix.mjs` (FIRST_INTR_MAP dump via `readMmio(24,...)`), `ble-epc.mjs` (EPC4/PS/pending/intenable via debug SAB), `ble-timeline.mjs` (timestamped UART + worker log merge), `ble-codedump/isrdump/parkdump/bufdump/scan.mjs` (live code/RAM dumps + objdump via `-b binary -m xtensa --adjust-vma`).
    - **[ACTIVE] BTDM re-attack, phase 2 (2026-09-11): RFI PS save/restore + scheduler forensics (06c57fb).** HW-exact: `take_interrupt` saves PS to EPS_level (free slots 184+idx-1), `_handler3` (rfi) restores it, `exception()` saves L1 slot — without this the L4 BT dispatcher (no explicit `wsr.ps`, unlike L1) left INTLEVEL stuck and wedged all level<=4 IRQs. Full suite green (55/0/7). Forensics, all measured: (1) CCOMPARE bridge works (CCSCH/CCFIRE 4000+, ticks healthy) — the "dead tick" theory is WRONG. (2) FreeRTOS TCBs HEALTHY (name@+52, owners self-consistent): BTU/BTC/btController all properly blocked ALONE on their own event lists (vals 25-prio ✓); prios 20/19/23; pinned core 0. NO corruption — NULL-next was a ListItem field-order MISPARSE. (3) Host posted HCI_RESET to the KE queue correctly (0x3FFCE9A8: next=0x3FFB8140 len=3 data=01 03 0c 00 — found by WIDE scan 0x3FFB0000-0x40000000; the first 64KB scan missed it). (4) The LL pump (0x4008f260) calls OSI+80 = `xQueueSendFromISR` with mailbox [[0x3ffc7208]]=[0x3FFCEED8]=0x3FFCEF80 as the QUEUE — which does NOT parse as a queue (pcHead=8) → mailbox never initialized with the real LL queue → post safe-fails forever → LL task never wakes → packet rots. (5) Direct host_recv guest-call CRASHES (reboot loop, injector task) — LL C2H state genuinely absent, 2022 verdict holds for that path. (6) Tasks that vTaskDelay die; spin-only tasks live; O2 suppression (no LL raises) = perfect health — consistent with ISR-epoch disturbance.
    - **Next (planned, not started): wake btController directly on HWAKE.** Its event list is 0x3ffcef10 (1 waiter: itself). Engine-side `native_bt_wake_ll()`: verify shapes (list-end min==MAX), uxListRemove-equivalent on its event item, vListInsertEnd-equivalent into ready[23] (0x3ffc3a80+23*20, verify!), set xYieldPending[0]=1, kick via BT IRQ re-raise. If the LL task drain-on-wakeup checks its KE list, it finds the already-queued reset and drives the REAL C2H path (no synthesis!). Needs ONE live run to re-dump TCB/lists/ready[23] (all /tmp dumps wiped twice — write diag scripts to tests/tmp-*, NOT /tmp!). NOTE: /tmp/opencode is wiped aggressively + the compile server dies silently (check `curl :5525` + restart `node tests/compile-server.mjs` before every session; server log to repo, not /tmp).
    - **[ACTIVE] BTDM re-attack, phase 3 (2026-09-12): scheduler-queue plumbing + hlevel shims (uncommitted engine, CC still open).** Five init gaps found by live forensics + libbtdm/ELF disassembly (all measured, Arduino 3.3.10): (1) `g_rw_schd_queue` @0x3ffc7200 goes stale (freed block) so pump `post_from_isr` safe-fails — engine repairs it from the waiter's event list (ev-32, shape-verified, membership-walked) whenever resolvable. (2) Queue lock @queue+84 reads 0, but `xPortEnterCriticalTimeout` CASes against the 0xB33FFFFF pool literal — wedges `xQueueReceive` in infinite CAS-retry (unmapped reads never match/never fault); engine seeds the FREE sentinel when zero. (3) The osi table routes the queue through hlevel wrappers whose double-deref (`[pcHead+16]` as queue) misroutes into `xQueueReceive(13,...)` (stale storage!) — engine plants the real handle (empty-only) + arg-fixes garbage queues at `xQueueReceive` entry + emulates `hli_queue_put` natively (memcpy/count/unblock/TopReady, drops when full/unresolvable); entry opcodes verified, BTC-gated. (4) One-shot HWAKE surgery misses (task blocks later) — demand-driven `bt_autowake` on every pump (TCB cache + BTC-exists gate + s==2/forced demand, surgery + TopReady-maintain + notify-suspend rescue + throttled doorbell pacemaker). (5) `rfi`-to-self park at 0x40083bca (`_highint4_stack_switch+0x7a`) — `_handler3` now breaks it (`[RFISELF]`, skip +3). (6) Unbounded `[VHCI]` TEMP spam (183K lines, 10-100x slowdown) is now budget-gated like the rest. Verified: `test-worker-bt`/`ble-init`/gpio/uart/timer/proxy-esp32/bench/standalone all PASS, no regressions. STILL OPEN: no HCI_RESET Command Complete observed (drain runs but emits nothing; H2C verified queued at 0x3ffce9a8/0x3ffb8140/0x3ffceaa0 (addr varies), s_btdm=2, ke_env event set, task cycling via wakes). Also open: intermittent pre-M_BLUENABLE stall (init/bluedroid_init hang, ~50% racy) and TWO btController TCBs (double-create, 0x3ffd0230 stale-blocked + 0x3ffd031c live-ready — repairs converge on the waiter). Diag scripts were tests/tmp-ble-*.mjs (deleted after use); key evidence preserved here. Next: find why `r_rw_schedule` doesn't emit CC (KE linkage vs modem/RF handshake vs C2H delivery), then ADV.

- **[DONE] Bootloader RNG-alias hang (test-worker-rmt family)** (2026-08-17): Arduino core 3.3.10 (IDF 5.5.4) bootloaders hang at PC=0x4007a526 (`process_segment_data` in `bootloader_support/src/esp_image_format.c`) spinning forever in `while (ram_obfs_value[0]==0 || ram_obfs_value[1]==0) bootloader_fill_random(ram_obfs_value, 8)` — the RAM-obfuscation value never became nonzero. Root cause: the bootloader's `bootloader_random_get` reads the RNG data register through the **DROM0 alias window** — literal `0x60035144` = `RNG base 0x3FF75000 + RegionDrom0MapBase 0x200C0000 + DATA offset 0x144` (native_mmio.rs:1300 `RNG_DATA_OFFSET=324`). The emulator's native RNG PTE (`setPTE(0x3FF75000, HID_RNG)`) covered only the base page; the alias page 0x60035000 still wrapped the JS **EmptyPeripheral** (the RNG's JS fallback — the real RngPeripheral class is unused dead code since the native port) so every alias read returned 0, the two-read XOR was always 0, and `bootloader_fill_random` stored zeros forever. Fix: `setPTE(0x60035000, HID_RNG)` — the Rust `rng_read_region` computes `addr & 0xFFF` (native_mmio.rs:1949), so the alias address needs no Rust change. The 0x3fff0004/0x3fff0008 flag = the bootloader's BSS `ram_obfs_value[0]/[1]` (proven by reading the IDF v5.5.4 source; the de-obfuscation `dest[w_i] = w ^ ((w_i&1) ? ram_obfs_value[0] : ram_obfs_value[1])` matches the bootloader disassembly at 0x4007a52e–0x4007a560 exactly, `bbci a8,2` = the `w_i&1` test). This also means the "only JS-routed page with live traffic was 0x3FF71000" audit finding was incomplete — the audit's fw-proxy32 was built with an older toolchain whose bootloader lacks the obfuscation path; the RNG-alias reads are `mmio_read` (not `map_read`) traffic, invisible to diag-maphist. Verified: RMT firmware boots through the REAL boot path (`=== RMT TEST ===` RMT_CONFIG/INSTALL/uninstall PASS, `=== ALL TESTS PASSED ===`), wasm-bench boot MMIO calls 0, test-wasm-standalone PASS, diag-maphist PASSED=true. Diag tooling: `tests/diag-flag.mjs` (polls 0x3fff0000 ram_obfs_value through the hang), bootloader extraction in /tmp/opencode (bl_seg.bin = IRAM 0x40078000, bl_dram.bin = DRAM 0x3fff0030, header at flash 0x1000, version string "v5.5.4" at ~0x1027).

## Dead-code deletion campaign (DONE, 2026-08-17)

All standalone dead peripheral files are deleted (14 files): rsa.js, aes.js,
spi-syscon.js, sha-ecc-key.js, i2c-i2s.js, twai-fifo.js, ledc-pcnt.js,
rmt-rng-math.js, timers.js, sdmmc.js, rtc-adc.js, mipi-dsi.js, usbcdc.js,
usbkeyboard.js. gpio.js was TRIMMED (GpioMatrix + GpioSignalDefs are live
routing data used by gpio-core.js/xtensa.js — only findGpioSignal, the hB/
GPIO_* constants, timReg20 were dead). XtsEncryptionState (live, this.xts)
was extracted to a new file src/peripherals/common/xts-state.js. rtc-adc
removal required: esp32.js reset() core1 gating now reads
native_dport_get_core1_enabled; DportPeripheral.enableCore1 getter is
null-safe. Full worker suite PASS=25 FAIL=0 SKIP=4 (identical to baseline).
No JS peripheral classes remain (rounds ### 14-19: DportPeripheral/interrupt
matrix → Rust matrices + hardcoded cross-core IRQs 24..27, GpioController →
plain config holder + `native_gpio_seed` bulk export, trace FFI gated in Rust
behind default-off flags + `native_trace_set_flags`). Re-verify with the
test battery after any new edit: tests/wasm-bench.mjs (MMIO 0),
tests/test-wasm-standalone.mjs (PASSED), /tmp/opencode/run-rmt-uart.mjs
(ALL TESTS PASSED).

## Dead-code deletion campaign (active, 2026-08-17)

User directive: delete the JS peripheral files/classes that have been ported
to native WASM — ONE FILE AT A TIME. After EVERY deletion: run the test
battery (`tests/wasm-bench.mjs`, `tests/test-wasm-standalone.mjs`,
`/tmp/opencode/run-rmt-uart.mjs`) and push to origin/main. This REVERSES the earlier
"do not remove" directive on dead JS instances. Live host bridges stay:
`uart.js` (UartController = TX ring-buffer bridge), `clock.js`/`clocks.js`,
`wifi-bridge.js`, `gdb-session.js`, infra files, and `wifi-analog.js` (its
`applyPeripheralResetValues`/`applySingleResetValues` are still called by
`esp32.js reset()` — only its peripheral classes are dead).

## Dead-code deletion campaign — second round (DONE, 2026-08-18)

`wifi-analog.js` TRIMMED (commit c593cda): `AnalogRfPeripheral` +
`WifiMacPeripheral` classes + `analogI2cReadResponse`/`rsaReg47..59`/
`WifiAnalogI2cReadTable(High)` removed (fully ported: `wifi_analog.rs` +
`hosted.rs` `WifiMacPeripheral`). `esp32.js`: `this.wifi` (WifiMacPeripheral
instance) + `AnalogRfPeripheral` instance + imports removed; the SAB
`wifiMac` region now comes from plain `chip.wifiMacState`
(SharedArrayBuffer 4096, MAC bytes at offsets 64-69). `worker-entry.js`:
`extract("wifiMac", chip.wifiMacState)`; `setupNativeWifiBridge` gate is
`config.wifi === false` (no `chip.wifi`); `native_wifi_mac_set_mac` is the
only MAC-seeding path. **DROM0-alias regression + fix**: the removed JS
instances previously provided the MemoryTranslator PTEs for the `0x60000000`
alias pages (`base + RegionDrom0MapBase 0x200C0000`); the bootloader/app
reads RNG/analog/WiFi-MAC registers through those aliases. `wasm-loader.js`
now sets `setPTE(0x6000E000, HID_WIFI_ANALOG)` + `setPTE(0x60033000,
HID_WIFI_MAC)`, and the Rust `analog_rf_*`/`wifi_mac_*_region` handlers
normalize `addr = (addr & 0xFFF) | BASE` (the JS MemoryTranslator used to
add the delta back; without it the offset math `addr - base` returned
garbage → RMT app stalled after `ho 0 tail 12 room 4`, wifi never past PHY
init). Rebuilt `esp_engine_wasm.wasm`. Verified: wasm-bench MMIO 0,
standalone PASSED, RMT full boot PASS, wifi scan `found 1 networks` PASS,
web "ready on port 80" PASS. Log entry ### 14.

`hosted.js` DELETED (commit f9fdec3): `EspHostedDevice` host stack
(1560 lines incl. `SdioCardPeripheral` + 802.11 frame helpers) was referenced
only by the `ESPHostedDevice` re-export in `src/index.js` — no test or src
consumer; the Rust `hosted.rs` is the port of the WifiMacPeripheral class
already removed in ### 14. Log entry ### 15. Verified: battery green
(wasm-bench MMIO 0, standalone PASSED, RMT full boot PASS).

`interrupt-efuse.js` + `gpio-core.js` DELETED (commits 4092a92, f895daa,
cab412a — log ### 17): the interrupt matrix was already fully Rust
(`InterruptMatrixPeripheral` wired in the native DPORT handler, commit
934943c); removed the dead `js_dport_matrix_read`/`write` externs +
loader handlers + `_dportMatrixFor`, the `DportPeripheral` instance +
`intMatrix` + the non-WASM fallback branch in `esp32.interrupt()`
(`native_interrupt` only), and the inert `StubPeripheral` FE +
`EfuseControllerPeripheral` instances. `js_dport_cross_core_irq` hardcodes
irq 24..27. GPIO: the JS `GpioController` pin-input setter was gated on
`inputEnable` (only set by the dead JS io_mux), so config `pinInputs` never
reached the Rust controller — every seeded pin read 0; only the strap (19)
flowed. Ported to a one-shot bulk export `native_gpio_seed` reading 65 u32
from `GPIO_SEED_SCRATCH` (260 B static, `native_gpio_seed_scratch`; pins
0..63, strap at [40]); loader `seedNativeGpio` writes the holder with one
FFI call. esp32.js keeps a plain config holder (`strapValue`/`inputValues`/
`pins` with `inputValue` get/set + `zeroMemory`/`reset` no-ops — it sits in
`this.peripherals`, so the no-ops matter); `GpioBothDir = 3` local const.
uart.js dropped its inert GPIO wiring (function listeners, cts signal, the
`rxPin`/`txPin`/`txPins` getters, local `GpioSignalDefs`/`UartPeripheralType`)
— no consumers; the UART clock listener stays (live clocks.js). gpio.js
stays (live `GpioSignalDefs` importers: register-data.js, xtensa.js).
**Stale-wasm trap**: f895daa committed a stale binary — the bulk-seed
exports were missing because the build had a compile error the
`grep -cE '^error'` filter hid AND the `&&` chain skipped the `cp`; fixed
(exports.rs semicolon + scratch sized for MAX_GPIO_PINS=64) and shipped in
cab412a. ALWAYS verify the wasm contains the new export before testing
(`strings ... | grep native_gpio_seed`).

`trace.js` FFI gated in Rust (commit f1ff849 — log ### 18): `trace_mem_write`
fired on EVERY memory write + `trace_return` on every ret/retw/rfi into the
JS `ESPTrace` sink whose flags were never enabled. Rust now checks
`TRACE_RETURNS`/`TRACE_MEM_WRITES` statics (default off) + export
`native_trace_set_flags` — the per-write/return FFI round-trips are gone;
the JS ESPTrace stays as the host-side sink (by design). The Rust `EspTrace`
class (`trace.rs`, `esp32.rs trace_map`) remains dead code.

`wifi-bridge.js` DELETED (commit 62c425d — log ### 19): `NativeInternetAP`
(~260 lines) was fully replaced by the Rust port (`wifi_bridge.rs`
`native_wifi_ap_*` exports); worker-entry's `setupNativeWifiBridge` owns the
gateway WebSocket + pcap host side. `NativeWifiMedium`/`WifiStatus`/
`WifiState` had no importers; `NativeWiFiBridge` was imported only by the
stale `tests/test-wifi.mjs` (Windows paths + `esp32.wifi` API removed in
### 14) — deleted both.

JS interpreter leftovers DELETED (commits 894c146, 146ea3c, 2d8bf4a,
b86787c — log ### 20-23): `xtensa-vecinst.js` (4944 lines: decodePie0/31 +
decode tables) and `xtensa-handlers.js` (2959 lines: cHandler7 + all
instruction handlers) were used ONLY by `XtensaCore.runInstruction()` — the
removed JS execution path (WASM cores step via `WasmCore.runInstruction` →
`core_step`); the method + imports were stripped from xtensa-core.js.
`freertos-tasks.js` (206: `FreeRTOSTasks`) had zero consumers — import/export
removed from index.js. `gpio.js` TRIMMED to `GpioSignalDefs` only (timReg19 +
GpioMatrixOutputSignal/GpioInputSignal/GpioMatrix classes dead; esp32.js
imports were unused). `xtensa-constants.js` STAYS (RegisterType/InstWidthTable
etc. are used by the live XtensaCore facade). Battery green after each
(wasm-bench MMIO 0, standalone PASSED, RMT full boot PASS).

`XtensaCore` facade TRIMMED 604→380 lines + Rust `EspTrace` class DELETED
(commits ff6b7ad, 63acbd4 — log ### 24): 31 dead JS-interpreter methods
removed (sarByte/fftBitWidth, enterLightSleep/exitLightSleep, PS_* accessors
except PS_EXCM/PS_UM/PS_WOE-setter, ACC/AR/BR/setBR, windowCheck/
takeInterrupt/updateInterrupts/intSetClear, dump*/restoreState,
readSpecialRegister, traceEntry/Return — the trace FFI goes straight to the
JS ESPTrace sink, unknownInstruction/break, _getOpcodeMemory) + 34 unused
xtensa-constants imports + the constants re-export. Kept: constructor
(ccompare cluster), attachMemorySystem, reset (esp32.js:617 calls
`e.reset()`), the read/writeUintX page-table surface (gdb `m`/`M` + esp32.js
reset's IO_MUX `writeUint32(0x3ff49000, 1023)`), writeSpecialRegister/
ccompareUpdate/CCOUNT/exception/vector, gdbReadRegister/gdbWriteRegister/
isWindowInstruction. Rust: `peripherals/common/trace.rs` (EspTrace, ~21KB/
chip buffer) was only the esp32.rs `trace_map` field — never called; deleted
the file, field, init + `pub mod trace;`. Static layout SHRANK — battery
green after rebuild (wasm-bench MMIO 0, standalone PASSED, RMT full boot
PASS).

- **[DONE] Analog input emulation** (2026-08-18, commit c714ad0): `esp32.onAnalogRead` was a hardcoded `() => 0` stub — `analogRead()` always returned 0 despite a complete native ADC engine (SAR register file, `adc_measurement`, `js_on_analog_read` FFI, AdcDone clock event). Host API: config `analogInputs: { pin: volts }` + `chip.setAnalogInput(pin, volts)`; `onAnalogRead` converts volts → 12-bit counts via the attenuation full-scale table [1.1, 1.34, 2.0, 3.3]V (0/2.5/6/11dB). worker-entry re-applies `config.analogInputs` after `chip.reset()`. Rust (`rtc_adc.rs`): the ADC port only saw the JS-parity start-force BIT-WIDTH field (0x2C) as "attenuation" — real per-channel attenuation is `SENS_SAR_ATTEN1/2_REG` (0x34/0x38, 2 bits/channel, HW reset 0xFFFFFFFF = 11dB, written by IDF `adc_oneshot_ll_set_atten`); added `sar_atten` tracking + per-channel field as the attenuation source (fallback to the old field only if SAR_ATTEN untouched). Arduino-core quirk (real-HW parity, verified in esp32-hal-adc.c `__analogChannelConfig`): `analogSetPinAttenuation` BEFORE the first `analogRead` is a NO-OP — the pin must be claimed first. New `tests/test-worker-analog.mjs`: 2048/0/4095/3378 expectations pass. Battery: wasm-bench MMIO 0, standalone PASSED, RMT boot PASS, proxy-esp32 PASSED, full suite **PASS=26 FAIL=0 SKIP=4**.

- **[DONE] RTC/flash persistence verified** (2026-08-18, commit 87ae22f): no engine changes needed — the deep-sleep + RTC-memory machinery (Rust `on_sleep_wakeup` → `reset_soc` → bootloader re-run; `rtcSlowMem`/`rtcFastMem` SABs untouched by `chip.reset()`; flash writes landing in the host SAB via `js_spi_flash_set_byte`) already worked but was untested. `test-worker-deepsleep.mjs` now runs REAL cycles: 3 boots with `RTC_DATA_ATTR` counter+string surviving, cause 0→4→4, `esp_reset_reason`=8 (`ESP_RST_DEEPSLEEP`, IDF 5.x enum), `rst:0x5` on wake (bootloader prints "Fast booting is not successful" — fast-boot CRC fallback, harmless, real-HW parity). New `test-worker-flash-persist.mjs`: host saves the flash SAB between runs — NVS counter 1→2→3 + raw magic 0xDEADBEEF persist across 3 simulator restarts. Battery: wasm-bench MMIO 0, standalone PASSED, RMT PASS, full suite **PASS=27 FAIL=0 SKIP=4**.

## What's left for the next phase (WASM engine hardening)

1. **Verify all worker tests** — re-verified 2026-08-18 (after the facade/EspTrace/js_schedule cleanup): **PASS=25 FAIL=0 SKIP=4** (bt + wifi/web/webserver gateway), wifi scan PASSED. Run: `./tests/run-worker-tests.sh --skip-bt --skip-wifi` (needs `tests/compile-server.mjs` on :5000).
   - **MultiSimulator re-verified** 2026-08-18 (commit efe1844 — also fixed `bench-multi.mjs`'s broken import `./src/index.js` → `../src/index.js`): `test-debug-wasm` boots firmware.bin on a parallel node; `test-real-firmware` ALL TESTS PASSED / WASM engine: PASS (332M cycles in 1.0s); `bench-multi` 1-4 nodes × batch 1000..50000 × 5M steps — single node 16.5M steps/sec, 4 nodes 54.5M total (~3.3x scaling).
2. **`_engineMode` / `both` removal** — DONE. Removed `_engineMode` from `esp32.js`, `worker-entry.js`, `test-wasm-standalone.mjs`; `esp32.js` now registers `_nativeUartReset` → `native_uart_reset` (UART state reset on `chip.reset()`). WASM presence is detected via `chip._wasmCores?.[0]`. No remaining references. (Note: `tests/rom` needed, symlink `rom` → `tests/rom` in repo root for standalone tests.)
3. **Performance** — measured 2026-08-08 (post page-cache). `test-worker-spi` full run: 1.3s wall, ~6M `map_read` calls ≈ 200ns each. Profile: 97% of `map_read` traffic is opcode fetches (size 16) + DROM byte reads from the **flash-cache IRAM window (0x40080000–0x400FFFFF)**; the remaining ~3% is the 0x3F400000/0x3F800000 DROM window + 0xFF000000 invalid reads. The 0x00000000-adjacent buckets earlier reported were a histogram artifact (`(addr>>>20)&0xFF` aliases 0x10000000). `wasm-bench`: ROM boot 20K inst in ~380μs. So per-instruction costs are no longer ~55μs — numbers below are stale.
   - **Map page cache — DONE** (commit 5504620): `_mapCache` in `wasm-loader.js` keyed by `addr >>> 12` → `{ region, m }` (`m` = `mmuEntryFor` for flash/DROM pages, re-validated per access so MMU remaps are picked up). Avoids the `mapAddress()` branch chain (incl. `new ReadonlyMemory(flash.subarray(...))` allocation per access).
   - **Rust flash fast-path — DONE** (commit 426818d): PTE_TYPE_FLASH routes flash-cache window fetches (0x400C0000–0x40400000) entirely in Rust (4MB flash mirror in WASM linear memory + MMU-table mirror reads per access; see Known bugs for the page-crossing fix and busy-loop timer events). Verify: `test-worker-*` + `wasm-bench` boot.
   - **Batch MMIO — DONE / NOT NEEDED** (measured 2026-08-18 with the new FFI-traffic tool, commit ebb7deb): boot = 0 MMIO calls; full fw-proxy32 run (58M cycles) ≈ 1395 JS `mmio_read`/`mmio_write` (42 reads = `dma_read_u32(0)` from `native_wifi_mac_rx_frame` — uninitialized RX descriptor; 1311 writes to the 0x60000000 UART0/SDMMC_ALT DROM-alias page + 21+21 to the UART1/MCPWM0 alias pages — pages without native PTE aliases), `map_read` 0, `map_write` 3, `js_interrupt` ~876, `js_log_str` 0 — ≈ 0.00004 FFI calls/cycle. The "445 per boot batch" figure predates the flash fast-path; batching would save nothing. Diagnostic tool: `CMD_GET_FFI_COUNTS=10` (worker-entry) + `getFfiCounts()` (worker-proxy) + `tests/diag-ffi-counts.mjs` (run `tests/fw-proxy32.bin` to completion). Gotcha: `blockingCommandLoop`'s allowlist excludes CMD 10 — works only on the non-blocking `commandLoop` path. Side finding (FIXED, commit 49e0001): the DMA path passed byte sizes to `dma_read` (4/8) which still applied `size >> 3` (4→0 → JS readUint32 default; 8→1 → readUint8) — correct but fragile (memory.rs:23-33). `dma_read_u32`/`dma_write_u32` now pass 32 (bits), consistent with the u8 wrappers and the dispatch internals; behavior-identical.
4. **Correctness — native UART done** (commit ce1e813), **native I2C done** (d31ebd3), **native SPI done** (4fc97f4), **flash fast-path done** (426818d), **native LEDC+PCNT+RMT done** (2026-08-12), **clock events done** (e3ca5f1), **interrupt matrix done** (934943c), **DROM low window done** (d650257), **native WiFi MAC TX/RX done** (b637a5f): follow the same bridge patterns (`js_*` clock events via `createEvent`, `_nativeXxxReset` re-hook after `chip.reset()`, JS-parity register semantics).
5. **Native TIMG0 — DONE** (PTE re-enabled; the old "PTE corrupts firmware" failure is resolved — see Known bugs). WASM AES also DONE (`init_aes` was the only blocker). `test-worker-proxy-esp32` now passes — the full worker suite is green (PASS=25 FAIL=0 SKIP=4).
6. **Gateways — DONE** (verified 2026-08-15): `test-worker-wifi`/`-web`/`-webserver` all pass through `openhw-studio-gateway/openhw-gw` (scan PASS, web "ready on port 80", webserver curl HTTP 200).
7. **BT controller — DONE** (2026-08-16): `test-worker-bt` PASSES — see the BT entry below.
8. **CI — DONE** (commit 47e55eb): `.github/workflows/ci.yml` — build-wasm job (rustup + cargo wasm32-unknown-unknown, uploads artifact), no-server-tests job (wasm-bench, test-wasm-standalone, test-wasm-worker-init with `rom` → `tests/rom` symlink), worker-summary job (workflow_dispatch only, self-hosted runner with a compile server; `--skip-bt`, `--skip-wifi` unless `run_wifi=true`). `.gitignore` whitelist updated: `!/.github/`.
9. **Port everything to WASM** — [DONE, 2026-08-16] the last JS-routed peripheral surface is gone. Audit result (from the plan below): the ONLY JS-routed peripheral page with live traffic was **0x3FF71000 `BtRfPeripheral`** (BT RF/LD clock, commit a5948fd) — it routes via the `mmio_read`/`mmio_write` FFI (NOT `map_read`/`map_write`): `buildPageTable()` in esp32.js gives every page-aligned JS peripheral a non-native PTE entry whose handler id is registered in `esp32.mmioHandlers`; the Rust dispatch calls back into JS for those ids. Verified by instrumentation: `map_read` FFI never fires during the BT test; removing BtRfPeripheral → `[test] FAILED`. BOTH remaining JS peripheral surfaces are now native:
  - **(a) BtRfPeripheral → Rust `HID_BT_RF=33`**: `BT_RF_REGS [u32; 1024]` static in native_mmio.rs (init/reset exports, `0x1c/4` seeded 0x4F, bit31 masked ONLY on 32-bit writes to 0x1c — reads echo the file), loader `setPTE(0x3FF71000, HID_BT_RF)`; JS class + instance REMOVED from esp32.js, reset hook `_nativeBtRfReset` added.
  - **(b) WifiMac RX mirror → Rust**: new exports `native_wifi_mac_rx_frame(ptr, len, channel) -> u32` (reads the frame from WASM linear memory, sets `p.channel`, calls `send_frame` — the existing Rust port of JS `sendFrame` with gating/DMA/RX_LAST_DSCR/event/interrupt) and `native_wifi_mac_rx_force(enabled, rx_enabled)` (syncs the JS `chip.wifi.enabled`/`rxEnabled` flags into Rust). worker-entry `_wifiApRxFrame` → `native_wifi_mac_rx_frame(ptr, len, 0)`; `native_wifi_mac_rx_force(1, 1)` called right after the JS flag forces in setupNativeWifiBridge. The JS `WifiMacPeripheral` class now survives ONLY as a bridge-synced register-file mirror (no RX logic).
  - **Follow-up (dead-code port, 2026-08-16, uncommitted): the last JS-routed peripheral STATE moved to Rust.** (1) DPORT core1 gate/stall state is now Rust statics `DPORT_APP_CLOCK_GATE`/`DPORT_APP_STALL` + `dport_core1_enabled()` (JS-parity getter `!appStall && appClockGate && !rtc.is_cpu_stalled_inner(1)`, init gate=true/stall=false, reset gate=true/stall=true per interrupt-efuse.js) + export `native_dport_get_core1_enabled()`; the `js_dport_set_app_clock_gate`/`js_dport_set_app_stall`/`js_dport_core1_enabled` FFI round-trips through the dead JS DportPeripheral are GONE (dport 0x2C read + rtc_adc set_core_enabled use the native static; loader refresh reads the export). (2) WifiMac gating/rxBuffer mirror bridges removed — `js_wifi_rx_base`/`js_wifi_mac_ctrl`/`js_wifi_rx_ctrl` FFI calls + decls deleted (native `enabled`/`rx_enabled`/`rx_buffer` are the only owners). (3) Host MAC config → `native_wifi_mac_set_mac(lo, hi)` (writes the native register file at base+64/68, LE pack, mac_bytes() order); worker-entry step 6 calls the export instead of the inert mirror. (4) TX glue decoupled from the peripheral object: `js_wifi_send_frame` → `loader._wifiMacTx` hook (worker-entry installs it; `chip.wifi.onTX`/`enabled`/`rxEnabled` writes dropped). The JS WifiMacPeripheral instance + all other dead JS peripheral instances REMAIN (user directive: do not remove) but are fully inert. Verified: wasm-bench boot MMIO calls 0, BT PASS, wifi scan PASS, webserver HTTP 200 (944B), diag-maphist PASSED=true 0 map traffic, full suite PASS=25 FAIL=0 SKIP=4.
  - Remaining JS by design (do NOT port): the **host bridges** (the `SimulationClock` event-loop driver — advances `chip.cycles` via `skipToNextEvent`; clock VALUES are now computed in-WASM, see the self-timed-core entry below, `js_flash_write_override` GDB flag, WiFi bridge sockets/pcap/status mirror, loader FFI glue) and the **zero-traffic `map_read`/`map_write` FFI fallback** (GDB flash writes + the never-touched DROM0 window; the MMU table 0x3FF10000–0x3FF14000 is PTE_TYPE_RAM region 5 = direct native linear-memory access — the old "Rust fast-path REVERTED" story was pre-flash-fast-path, superseded by 426818d/1324314). Dead JS fallback classes (native PTE overrides) remain harmless: every native-peripheral fallback copy in esp32.js, WifiMacPeripheral mirror, RTC_IO 0x3FF48400 / SENS 0x3FF48800 / GPIO_SD 0x3FF44F00 (inside the native RTC/GPIO pages).
  - **[DONE] Self-timed core (clock → WASM)** (commit a81b52d + native_fast_forward, 2026-08-22): removed the 4 per-native-MMIO-access clock FFI round-trips `js_clock_nanos`/`js_apb_ticks`/`js_rtc_slow_ticks`/`js_bt_clock_ticks` AND moved the idle fast-forward into WASM. `CLK_CYCLES` static is advanced by `Esp32Chip::step()` (`advance_clock(1)` per instruction); the worker idle path now calls `native_fast_forward()` (computes the next native timer alarm — TIMG0/1, FRC, BT RF — and advances `CLK_CYCLES` to that target, returning the cycle advance as **`f64`**). The worker does `chip.cycles += native_fast_forward()` (NOT `chip.cycles =` — a `u32` return comes back to JS as a **signed i32** and the unbounded cycle count exceeds 2³² on long sims, so return `f64`; `syncClockState()` re-pushes `chip.cycles >>> 0` into `CLK_CYCLES` after each advance). `clk_nanos()/clk_apb()/clk_rtc_slow()/clk_bt()` derive nanos (CPU 160MHz), APB (80MHz), rcSlow (32.768kHz) and BT 2us-ticks from `CLK_CYCLES`. **u32 export-arg gotcha**: `native_set_clock_state` takes **u32** (NOT u64) — this WASM engine mishandles u64 export args and a u64 signature caused a silent boot hang. Verified: wasm-bench MMIO 0, test-wasm-standalone PASSED 1M steps, full worker battery **PASS (all runnable tests) / SKIP=3 (gateway)** including timer/timer-freq (1200s sim, RATIO=PASS)/proxy-esp32/deepsleep/rtc-wdt/bt.
- **[DONE] Batched Xtensa core execution (`core_run`)** (2026-08-28, commit fd4b39b): the new `core_run` WASM export runs up to 512 interleaved iterations (both cores 1:1, `sync_ccount` per instruction) per FFI call instead of one FFI call per instruction. `chip.step()` dispatches the single call and the worker hot path (`runSimChunk`) invokes it ~1024× less often, cutting the JS↔WASM boundary and per-step JS overhead. Verified end-to-end: for sustained active (CPU-bound) execution the worker path runs **~1.9× faster** — 30 → 58 M instructions/sec at a fixed 30 M virtual-cycle target (busy-loop firmware, loop WDT disabled). Core-level (`chip.step()` boundary) gain is ~1.55× (37 → 57 M instr/sec). Idle/yielding firmware shows little wall-clock change because cost is dominated by idle fast-forward + native peripheral FFI, not the per-instruction core loop. Full worker suite **PASS=27 FAIL=0 SKIP=4** (unchanged baseline); `test-real-firmware` ALL TESTS PASSED in 1.0s. See CHANGELOG 0.1.2/0.1.3.
- **[DONE] FFI call reduction (combined exports)** (2026-09-01, commit 4a680f2): two new WASM exports replace the multi-call-per-step pattern:
  - `native_pump_events()`: merges all 5 native timer pumps (TIMG0, TIMG1, FRC, event queue, BT RF) into a single FFI call. Busy path: 7 → 2 FFI per `chip.step()`.
  - `native_idle_advance(js_cycles)`: merges clock sync + `native_fast_forward` + all pumps into a single FFI call. Idle path: 9 → 2 FFI per `chip.step()`.
  The worker hot loop (`worker-entry.js`) now has a single `if (chip.coresIdle)` branch calling `native_idle_advance(chip.cycles >>> 0)` or `native_pump_events()` respectively. Verified: wasm-bench MMIO 0, test-wasm-standalone PASSED, profiler ALL TESTS PASSED at 23.30 MIPS. ESLint 0 errors/0 warnings. New tests: `test-worker-ota-multi`, `test-worker-twdt`, `test-worker-ble-init`.
- **[DONE] Dead code cleanup + full suite verification** (2026-09-01, commit 53ce340): removed 16 inert EmptyPeripheral instances from esp32.js (all have native PTEs), 8 dead re-exports from index.js (kept SimulatorWorker/MultiSimulator/ESP32), dead functions from helpers.js, dead crypto barrel re-exports from common/index.js, empty import from xtensa-constants.js. Fixed 3 test compilation failures for IDF 5.x API (TWDT → gptimer, BLE init-only, OTA partition discovery). Full worker suite: **PASS=32 FAIL=0 SKIP=4**.
  - Verified: `test-worker-bt` PASS, `test-worker-wifi` scan PASS, `test-worker-webserver` HTTP 200 (944B), `tests/diag-maphist.mjs` PASSED=true with zero JS map traffic, full worker suite **PASS=25 FAIL=0 SKIP=4**. Note for future agents: to measure JS traffic, instrument the `mmio_read`/`mmio_write` FFI handlers AND the `map_read`/`map_write` FFI separately — they are distinct paths; the old audit tool (`tests/diag-maphist.mjs`) only counts `map_read`/`map_write` and prints 0 lines now. **Gateway gotcha**: the port-forward port is chosen at FIRST-DHCP time by binding 8080 upward (`handleDHCP.go`) — if 8080 is transiently occupied by a stale listener from a previous gateway instance, the forward lands on 8081 and is cached for the process lifetime (`globalIPToPort` per IP); curl 8080 → HTTP 000 with NO emulator involvement. Restart the gateway cleanly before debugging webserver tests.