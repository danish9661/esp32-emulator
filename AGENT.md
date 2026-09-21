# Agent Handoff — ESP32 WASM Emulator BTDM

**Date**: 2026-09-18
**HEAD**: `051ddae` (dirty: native_mmio big, exports +diag/reset/FENT, memory +seg3, wasm-loader +seg3, worker-entry +cmds)
**Mission**: full BLE/BT + GATT: `BLU_ENABLE_DONE` → `ADV_START_DONE` → `GATT_SVC_STARTED`
**WASM**: `dac3910e43206b7f89729b479916d239`, md5(src)==md5(target) — ALWAYS re-verify after cp

## Status: BDINIT=0 + ENABLE=0 on e7f2 (run38) — first REAL enable

`tests/tmp-qlong2.mjs` (build e7f2dac8): ADVBOOT → BDINIT_CALL → **BDINIT=0** → M_BLUENABLE → **0** → BLU_ENABLE_DONE. No assert. (enable()=0 = ESP_OK — the stack is truly up.)

## Root causes fixed (all measured, in tree)

1. **run265r — flash_mirror seg3 offset** (~0x40020): JS parses image headers → `native_flash_seg3_off`. Without it the HOOK scanner reads wrong bytes.
2. **run265s/u — sig gates**: take skeleton `(w2&0xFFFF)==0x1103 && (w2>>24)!=0xc4`; ofree mask `(w1&0xFFFFFF)==0x8120a2` + tail; ofree retw `HOOK_OFREE+0xC`.
3. **run265q — bench perf**: `p0==0xFFFFFFFF` → DONE immediately.
4. **run266 — per-boot diag reset** (`reset_boot_diag` + `reset_shim_diag`, DG_* counters).
5. **run267-269/278-285 — send-entry plants**: 93764 entry (or/beqz/bltu/call8/msgs++ ISR-shim), 93798 worker-entry (M98 dma `w0=0x49008136`), 9379f bnez (verified `bnez a2→0x400937b1`), sendplant nulls; heal-in-place; ALL gated on `bt_bluedroid_started()` (ungated heal killed early boot with :822). NEVER trust file view for 0x4009xxxx (bootloader MMU remaps) — dma diag is truth.
6. **run286 — true-give hook** (HOOK_GIVE sig = mutex_unlock twin 0x40108700; firmware runs 0x40108c10): +9 slot plant + GV/G9 diag on the true give.
7. **run287 — free-invalidate** FUTSEM_TAB pairs at HOOK_FREE (use-after-free address aliasing).
8. **run289 — :3a9 a9emu** at 0x400937b8: NULL-item send to isz-8 queue → `[q+56]+=1`, jump 0x400937c4, native unblock runs.
9. **run290 — await-xref take**: true take = the one future_await call8-targets (7ffd: 40110074, not twin 4010fb24); call8 decode `(word>>6)&0x3FFFF` sign-extended.
10. **run290b/c — tretw range + half-align**: scan from take+0x10; upper-half match → a+2 (applies to take/await/new retw).
11. **run291 — take returns 0 on success** (take-body ground truth: kernel-ret 1 → moveqz zeroes → neg → 0). take-emu + waiter-emu set a2=0 (were 1 → transfer `bnez → 0` → BDINIT=-1).
12. **run294 — CB-value emulation**: CB/set_value never run (no BTC task; all CB FENTs silent) → [fut+8] stays 0 → await-ret 0 → beqz → -1. take-emu stores completion value 1 when zero (self-gating).
13. **run295 — prepost per-(slot,sem)**: same slot recycled with new sem (3ffd0d70: 3ffd3814→3ffd3894) must re-arm (was one-shot per boot → future #2 starved).

## How the enable path actually works (e7f2 objdump ground truth)

- `esp_bluedroid_init` (0x400e2a1c): a2=0x103 preset; btc_init-stub; future_new; beqz-null→0x101; **btc_transfer_context** (0x40106870); beqz-ret→-1; future_await; beqz-ret→-1; else 0.
- `btc_transfer_context` (0x40106870): msg checks; malloc; memcpy; post; take(sem,-1); **bnez take-ret → return 0**; else free → return 4. (Take convention: 0=success.)
- `osi_sem_take` (0x40108c24): infinite path kernel-ret 1 → a2=0; timed path beqi==1 → a2=0, else -2.
- BDINIT=-1 ⟺ transfer==0 OR await==0. Enable=259 (INVALID_STATE) ⟺ init never set INITED. GAPREG/ADVCFG=259 follow. "BLU_ENABLE_DONE" marker is unconditional — check VALUES (0 needed).

## Known gaps

- Fresh builds (62ea/ec31/2ca0/7ffd-ADV/fceab): HOOK resolves per-boot; 7ffd reached DONE once tretw fixed. ADV (`ADV_DATA_DONE`) never yet observed on any build — next.
- `SHIM_RECV/SHIM_SEND` hardcoded candidates + dma-verify (ROM-mapped, fine).
- `ofree` resolves to 0x400e1ef8 on e7f2 (not default 0x40107a20) — works; confirm it's really osi_free_func.
- CB path (btc_init_callback → set_value) never executes — fully emulated (take-emu + value-store). Fine unless a build checks more CB side effects.

## Next

1. e7f2 ADV: `tests/tmp-advdata.mjs` (fresh) → ADV_DATA_DONE (gap_cb → BTC task → HCI). The BTC TASK still doesn't exist — ADV likely needs task-creation or transfer-emulation work.
2. e7f2 GATT → GATT_SVC_STARTED.
3. Re-test fresh builds with current wasm.
4. Trim diag + full regression + single commit (per AGENTS.md).

## Commands

```bash
node tests/compile-server.mjs > tests/tmp-server.log 2>&1 &   # dies often; restart first
cd engine-wasm && cargo build --release --target wasm32-unknown-unknown
cp target/wasm32-unknown-unknown/release/esp_engine_wasm.wasm ../src/engine/esp-xtensa/ && md5sum target/wasm32-unknown-unknown/release/esp_engine_wasm.wasm ../src/engine/esp-xtensa/esp_engine_wasm.wasm
node tests/tmp-qlong2.mjs > tests/tmp-verify.log 2>&1 &
timeout 120 node tests/wasm-bench.mjs 2>&1 | grep -E "HOOK|LOOP|batch|Total"
```

## Notes

- AGENTS.md tracker stays `[ACTIVE]` until ADV+GATT both PASS.
- objdump: `~/.arduino15/packages/esp32/tools/esp-x32/2601/bin/xtensa-esp32-elf-objdump -b binary -m xtensa --adjust-vma=<vma> -D <bin>`; FILE offsets valid ONLY for 0x400Dxxxx (seg3 fileoff 0x40020); for 0x4009xxxx use dma-view diag.
