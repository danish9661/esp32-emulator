# BLE branch — checkpoint (2026-09-19)

Purpose: preserve the in-progress BLE/BT + GATT work exactly as it is.
Do NOT merge this branch into `main`. `main` stays clean at `051ddae`.
Future work resumes here.

## Baseline

- `main` HEAD at branch point: `051ddae BTDM run178: force-clear ack kills the 1.1M-vector storm (CC still open)`
- This commit contains that HEAD plus all working-tree changes as of 2026-09-19 (no code changes made during checkpointing).

## What works (measured, `tests/tmp-follow-run2.log`, build `7d1e668a`)

- UART marks reached: `ADVBOOT`, `BDINIT=0`, `ENABLE=0`, `BLU_ENABLE_DONE`, `GAPREG=0`, `GATTREG=0`, `ADVCFG=0`, then `GATTAPPREG=` with EMPTY value (hang).
- Hook scan resolves: `[HOOK] new=4010ff34 take=40110d00 give=401107dc free=4010ff1c ofree=400e1dac eb2=40107eb2 cb=400e2b1c rretw=4010ff17 btr=4010c6f4 bdtr=400e2adc bdaw=400e2ae4 cbset=4010c5e2 done=1 miss=0x80`.
- 4x `btc_transfer_context` envelopes logged (`[TR]`), 4x `4010c6f4` FENTs, `take-emu` x2, `prepost` x2, `btcb_call` x1.

## True wall (no UART-spoof)

- App never prints `ADV_DATA_DONE`, `SETUP_END`, or `GATTAPPREG=0`.
- Tail of run: `wedgenolock 40083d74` x2203 (`lv=00006246 c0=3ffbd0d0`). Both cores parked, no completions delivered.
- Root pattern after 3 days: the future/take/give shim stack releases individual waits, but the BTC task that produces GAP/GATT completion callbacks never runs, so each fix only moves the park point. No ADV/GATT completion has ever been observed on this branch.

## How to reproduce

- Compile server must be up (this tree's diag script uses `http://localhost:5525`, not `:5000`): `node tests/compile-server.mjs` (log: `tests/compile-server.log`).
- Repro script: `node tests/tmp-gatt.mjs` → log `tests/tmp-follow-run2.log` (checked in here as reference).
- WASM rebuild: `cargo build --release --target wasm32-unknown-unknown` in `engine-wasm/`, then copy to `src/engine/esp-xtensa/esp_engine_wasm.wasm`. Sanity: `node tests/wasm-bench.mjs` (boot MMIO ~0/1).
- Current-build literals (build `7d1e668a`, see `engine-wasm/src/xtensa/exports.rs`): `btc_transfer 0x4010c6f4`, take site `0x4010fcf8`, take wrapper `0x40108a7c/0x4010b758`, `osi_sem_free 0x40110d2c`, app_register `0x400e2af0`, `btc_init_callback 0x400e2b10/0x400e2b1c`.

## Engine files touched (vs `051ddae`)

- `engine-wasm/src/native_mmio.rs` (+~4164): future/take/give/await/ready-retw shim legs, TR/cb_set logging, ADV/GATT spoof staging (log-only, pumps print REMOVED per run305).
- `engine-wasm/src/xtensa/exports.rs` (+~379): FENT whitelist incl. `0x4010fcf8`/`0x40110d2c`, `reset_boot_diag_pub`, RETW tracer, TR diag.
- `engine-wasm/src/xtensa/memory.rs` (+~29): `FLASH_SEG3_OFF` + `flash_mirror_read_u32` for hook scanner.
- `src/engine/esp-xtensa/wasm-loader.js` (+21): `native_flash_seg3_off` seg3 push.
- `src/engine/esp-xtensa/wasm-loader.js` + `src/sab/worker-entry.js` (+56): `PAGE_TABLE_OFFSET` fix, inline CMD_PCTRACE/CMD_WATCHPOINT/CMD_READ_MEMORY servicing.
- `src/engine/esp-xtensa/esp_engine_wasm.wasm`: rebuilt binary.
- `tests/tmp-*`: all diag scripts + run logs committed as reference (normally untracked; kept here only for resume).

## Agreed direction (2026-09-19)

- User decision: timebox ADV only (`ADV_DATA_DONE`), no new shims without a mark moving. Park GATT.
- Next session: resume on this branch, pick ONE trigger (GAP config-adv-data envelope `e0==0x50000 && e1==0x10100 && a4==0x2c`) and drive it to a real `ADV_DATA_DONE` UART mark. Delete stale `tests/tmp-adv-run*.log` before any final commit per AGENTS.md.
