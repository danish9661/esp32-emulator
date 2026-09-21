# BLE Handover — resume ADV/GATT on this branch (2026-09-19)

Audience: any agent picking up BLE work. Read this file first, then
`ble.md` (44-line checkpoint summary), then the reference log.

## 0. Branch contract

- You are on `ble-checkpoint` (commit `a99de84`, parent `051ddae` on `main`).
- Do NOT merge this branch into `main`. `main` stays clean at `051ddae`.
- Everything here is committed as-is, including diag scripts and logs.
- User decision (2026-09-19): timebox ADV only. One trigger, one mark:
  drive the GAP config-adv-data envelope to a real `ADV_DATA_DONE` UART
  mark. No new shims without a mark moving. GATT is parked.
- Before any final commit: delete stale `tests/tmp-adv-run*.log` (AGENTS.md).

## 1. Resume in 5 commands

```
node tests/compile-server.mjs            # :5525, log tests/compile-server.log
node tests/tmp-gatt.mjs                  # repro -> tests/tmp-follow-run2.log pattern
node tests/wasm-bench.mjs                # sanity: boot MMIO ~0/1
cargo build --release --target wasm32-unknown-unknown   # in engine-wasm/
cp target/wasm32-unknown-unknown/release/esp_engine_wasm.wasm ../src/engine/esp-xtensa/esp_engine_wasm.wasm
```

Gotcha: diag scripts use `http://localhost:5525`, not `:5000`.
Check `curl :5525` first; the server dies silently. Keep its log in the
repo, not `/tmp` (wiped aggressively). Write all new diag scripts to
`tests/tmp-*.mjs`, all new logs to `tests/tmp-*.log`.

## 2. Repro and the true wall

Script `tests/tmp-gatt.mjs` (90 lines): compiles the GATT+ADV firmware
(`// gatt1` marker, build `7d1e668ae5a30acb`), boots it on one worker,
polls UART marks for 180s. Reference output is `tests/tmp-follow-run2.log`
(8030 lines, committed).

UART marks reached (with timings):

```
ADVBOOT 2s, BDINIT=0 4s, ENABLE=0 6s, BLU_ENABLE_DONE 6s,
GAPREG=0 6s, GATTREG=0 6s, ADVCFG=0 6s, GATTAPPREG= 6s (EMPTY value, hang)
```

Never printed: `ADV_DATA_DONE`, `SETUP_END`, `GATTAPPREG=0`, `LOOP_ALIVE`,
`Guru`, `assert`. Tail of run: `wedgenolock 40083d74` x2203
(`lv=00006246 c0=3ffbd0d0`). Both cores parked, no completions delivered.

Pattern after 3 days: the future/take/give shim stack releases individual
waits, but the BTC task that produces GAP/GATT completion callbacks never
runs, so each fix only moves the park point. No ADV/GATT completion has
ever been observed on this branch. Do not add UART-spoof prints (run305
removed the pumps for exactly this reason: marks must come from firmware).

## 3. Reference-log census (tmp-follow-run2.log)

- `[HOOK]` x1: `new=4010ff34 take=40110d00 give=401107dc free=4010ff1c
  ofree=400e1dac eb2=40107eb2 cb=400e2b1c rretw=4010ff17 btr=4010c6f4
  bdtr=400e2adc bdaw=400e2ae4 cbset=4010c5e2 done=1 miss=0x80`.
  `done=1` = all mandatory hooks resolved on build `7d1e668a`.
- `[TR]` x4 (btc_transfer envelopes, `HOOK_BTRANSFER+3`, own frame):
  TR1 `e0=0 e1=1` (init path), TR2 `e0=0x2000000 e1=0xa` (enable path),
  TR3 `e0=0x50000 e1=0x10100 a4=0x2c` = GAP config-adv-data (YOUR trigger),
  TR4 `e0=0x20000 a4=0x20` = GATT app_register (parked, GATT is out of scope).
- `[FENT]` 469 total, 18 pcs. Top: `40093798` x357 (xQueueGenericSend),
  `4009351c` x51, `40093694` x36. BT-relevant: `4010c6f4` x4 (transfer,
  a0 = 800e29bc/800d1c62/800d1cd4/800d1ceb = four distinct callers),
  `40110d00` x3 (take), `4010ff34` x2 (future_new), `4010ff68` x2
  (future_await), `400e2b1c` x1 + `400e2b10` x1 (btc_init_callback entry +
  pre-call), `400e2af0` x1 (app_register), `40110d2c` x1 (osi_sem_free).
  Zero hits: `4010fcf8`, `40108a7c`, `4010b758` (FENT literals added but
  never fired — see §7), `RETW`/`EB2`/`FRE`/`EBB` x0, `PZ` x0.
- `[SHIM]`: `midplant 3ffceee4` x14, `prepost` x2, `take-emu` x2 (both
  consumed, no EB2 after), `a9emu`/`bnezplant`/`sendnull2` x2 each, `qheal2`
  x4 (3ffd03a4/3ffc4748/3ffd0450/3ffd0d64), `entryplant` x1, `btcb_call` x1.
- Other: `[RING]` x4, `[FREEZE]` x1, `[R] qgate` x2 (no-BTC gate),
  `Peripheral to reset not found` x14 (harmless, missing EmptyPeripherals).

## 4. The ONE trigger (ADV timebox)

TR3 is the GAP config-adv-data envelope: `e0==0x50000 && e1==0x10100 &&
a4==0x2c` (ae1decdd-measured; IDF BTC enums are build-stable). The TR leg
in `native_mmio.rs` (~line 6119) already detects it and sets
`ADV_SPOOF_STAGE=1, ADV_SPOOF_TICK=50` — staging only, pumps print REMOVED.
Your job: replace the staging with a REAL completion. The completion must
invoke the recorded `gap_cb` (see §5) with
`ESP_GAP_BLE_ADV_DATA_SET_COMPLETE_EVT`, so firmware prints `ADV_DATA_DONE`
itself and proceeds to `esp_ble_gap_start_advertising`.

Constraints: (a) no UART prints from the engine — the mark must come from
firmware; (b) no new shims without the mark moving — one trigger, one
mark; (c) GATT (`TR4`, `GATT_SPOOF_*`, `400e2af0`) is parked, do not touch.

Suggested order: (1) confirm `gap_cb` value is captured in `CB_SLOT_TAB`
at `HOOK_CBSET` during this run (log-only check); (2) find where the BTC
task should dispatch the completion (disassemble around the transfer
return `0x4010c6f4` + callers `800e29bc/800d1c62/800d1cd4/800d1ceb`);
(3) implement exactly one completion leg, gated on `ADV_SPOOF_STAGE==1`
plus tick expiry plus `bt_bluedroid_started()`; (4) re-run `tmp-gatt.mjs`
and grep `ADV_DATA_DONE`.

## 5. gap_cb capture (already wired, verify only)

`btc_profile_cb_set` store site is resolved per-build by the hook scanner
(`HOOK_CBSET`, `addr+3`, ~line 2834) and every `(slot, value)` pair is
recorded into `CB_SLOT_TAB[16]` (~line 6213). App-code values
(`0x400dxxxx`) are the `gap_cb` candidates. Nothing to build here — just
confirm the table holds a `0x400dxxxx` value in your run before writing
the completion leg. If the table is empty, the cb_set signature missed
this build and THAT is your first (and only) fix.

## 6. Engine map (what changed vs 051ddae, +4455/-194)

- `engine-wasm/src/native_mmio.rs` (+4164): everything BT. Entry point
  `bt_shim_step()` (~line 3202, called per step, early-outs on
  `BT_DIAG_DISABLE`). `bt_hook_scan()` (~line 2268) runs at fn top:
  resolve-once per boot, idempotent, self-verifying (re-scans when cached
  TAKE bytes mismatch = new build). All legs read the `HOOK_*` cache;
  e7f2 defaults are the zero-cost fast path.
- `engine-wasm/src/xtensa/exports.rs` (+379): FENT whitelist (~line 1100,
  multi-build literals), `reset_boot_diag_pub` (~line 128, called from
  the Rust reset path — per-boot one-shot budgets, else boot 2+ logs
  nothing), RETW tracer (~line 571, budget 6), `native_flash_seg3_off`
  (~line 271), `native_pc_trace` (~line 182, CMD_PCTRACE), watchpoints
  (~line 1472, CMD_WATCHPOINT).
- `engine-wasm/src/xtensa/memory.rs` (+29): `FLASH_SEG3_OFF` +
  `flash_mirror_read_u32` (~line 111): the flash FILE is a bootloader
  container, not a 1:1 VMA window — seg3 (VMA `0x400D0020`) lives at file
  offset `~0x40020`. Scanner must use the SEG3 offset, not VMA math.
- `src/engine/esp-xtensa/wasm-loader.js` (+21): `native_flash_seg3_off`
  push (~line 484, parses image headers per boot, three base guesses
  `0x10000/0x1000/0x0`).
- `src/sab/worker-entry.js` (+56): inline CMD_PCTRACE/CMD_WATCHPOINT/
  CMD_READ_MEMORY servicing inside runSimChunk (~line 303, else mid-stall
  diag deadlocks) + `PAGE_TABLE_OFFSET` fix (~line 1110, old hardcoded
  82048 predates the static-zone move).
- `src/engine/esp-xtensa/esp_engine_wasm.wasm`: rebuilt binary (1599214 B).
  Stale-wasm trap (known): always verify mtime/hash after rebuild before
  testing; a skipped `cp` ships yesterday's engine silently.

## 7. Current-build literals (build 7d1e668a — re-verify per build)

Scanner `HOOK_*` values (authoritative, see `[HOOK]` line): `NEW 4010ff34`
(future_new), `TAKE 40110d00` (osi_sem_take), `GIVE 401107dc`,
`FREE 4010ff1c`, `OFREE 400e1dac`, `EB2 40107eb2` (await take-return),
`CB 400e2b1c` (btc_init_callback pre-call), `RRETW 4010ff17`,
`BTR 4010c6f4` (btc_transfer_context), `BDTR 400e2adc` / `BDAW 400e2ae4`
(bluedroid_init transfer/await-ret), `CBSET 4010c5e2` (cb_set store).
FENT backup literals: transfer callers `800e29bc/800d1c62/800d1cd4/800d1ceb`,
take site `4010fcf8`, wrapper `40108a7c/4010b758`, `osi_sem_free 40110d2c`,
app_register `400e2af0`, `btc_init_callback 400e2b10/400e2b1c`, enable
`400e2a40`. Kernel pcs (`40093xxx/40095eb4`) are build-stable; `400exxxx`
and `4010xxxx` differ per build — keep ALL literals, never reuse stale
ones. ELF ground truth: `tests/build/local-compile/7d1e668ae5a30acb/out/
sketch.ino.elf` + toolchain `~/.arduino15/packages/esp32/tools/esp-x32/
2601/bin/` (`xtensa-esp32-elf-nm`, objdump needs `-b binary -m xtensa
--adjust-vma=<addr>` for flash).

## 8. Shim inventory (do not extend — consume)

- `FUTSEM_TAB[8]` (4 pairs, ~line 2246): pinned (future,sem) records, never
  evicted. Snapshot sites: future_new retw + `400e2b1c` (a10=future*) +
  future_ready post-entry. Invalidated at `HOOK_FREE` (run287: freed blocks
  recycle, stale pairs poison new futures -> `:3a9` assert).
- `PREPOST_SLOT/PREPOST_SEM` (~line 2256): pre-park give. At take ENTRY,
  first lap only, if `q56==0`, pre-post `msgs=1` so the current lap
  succeeds natively. Keyed (slot,sem) so recycled slots re-arm (run295).
- `HOOK_TAKE_POST` legs (~lines 3296/3503/3598): TKO lap logging, TKG gate
  diag, take-emu (consume `[q+56]`, neutralize `sem_free` via `[slot]=0`,
  preset take-frame `a2=0` (run291: native take returns 0 on success, NOT
  the future value — the old `[fut+8]` value turned every emulated take
  into `BDINIT=-1` via btc_transfer's `bnez`), emulate the CB effect
  `[fut+8]=1` when still zero (run294: the setter never runs, no BTC task),
  `pc=HOOK_TAKE_RETW` for a REAL retw (never fabricate windows — run261
  h32 double-fault: fabricated WB fails the rr-valid check -> user-vector
  fault).
- `HOOK_READY_RETW` leg (~line 4035): give-path emulation, OUR-future
  gated. `osi_free` swallow at `HOOK_OFREE` (run261: leak 12B, heap
  untouched — free on recycled blocks asserts).
- `midplant/entryplant/bnezplant/a9emu/qheal2/sendnull2` (14/1/2/2/4/4
  hits): queue-heal and send-path repairs around `3ffceee4` — backstops,
  leave alone unless your run shows them missing.
- `ADV_SPOOF_*/GATT_SPOOF_*` (~lines 2234/2236, TR staging ~6176/6188,
  pumps ~7644): trigger state only, prints REMOVED (run305). GATT parked.
- Per-boot resets (~line 2100 + `reset_boot_diag()`): ALL one-shot statics
  (`HOOK_*_N`, `DG_*_N`, `FENT_N`, `RETW_N`, tables, stages) zero per boot
  — WASM-instance reuse across boots otherwise makes boot 2+ log nothing
  (run265p/run266 lesson).

## 9. Dead ends (do not retry)

- UART-spoof pumps (run302/303 printing marks from the engine): REMOVED
  run305. Marks must come from firmware or they prove nothing.
- Hooking `host_recv` / synthesizing HCI Command-Complete: LL C2H state
  genuinely absent (2022 verdict holds for that path; guest-call crashes
  into a reboot loop).
- New take/give/waiter shims without a mark moving: 3 days proved each fix
  only moves the park point while the BTC task never runs.
- Reusing stale per-build addresses (f13066/e7f2 literals on a new build):
  silent zero-fire, looks like a hang. Re-run `nm` per build.
- `/tmp` for scripts/logs: wiped aggressively. Repo `tests/tmp-*` only.
- Committing partial/forensics-only checkpoints to `main`: prohibited by
  AGENTS.md phase order (this branch is the exception, by user order).

## 10. Diagnostics cheat sheet

- `grep -c wedgenolock <log>`: park depth (2203 = fully parked).
- `grep '\[TR\]' <log>`: transfer envelopes (expect 4: init/enable/ADV/GATT).
- `grep 'FENT] c1 4010c6f4' <log>`: transfer entries + caller a0s (expect 4).
- `grep '\[HOOK\]' <log>`: `done=1 miss=<mask>` — miss bit 0x80 is the
  known-accepted gap on this build, not a failure.
- `grep '\[SHIM\] take-emu' <log>`: emulated take returns (x2 consumed,
  expect no EB2 after — that absence is itself the signal the waiter path
  ends inside take, motivating the completion-leg approach in §4).
- Mid-stall live reads: CMD_PCTRACE (`native_pc_trace(n)`, T-lines, both
  cores, trailing core digit) + CMD_WATCHPOINT + CMD_READ_MEMORY all work
  mid-run via the runSimChunk inline servicing (§6) — use them instead of
  stop/restart cycles.
- Battery after any engine change: `wasm-bench.mjs` (MMIO ~0/1) +
  `test-wasm-standalone.mjs` (PASSED) + RMT boot (`ALL TESTS PASSED`)
  before the BLE repro. Full suite only at the end (AGENTS.md phase 4).

## 11. Files on this branch (vs 051ddae)

Core (5 source files + binary): `engine-wasm/src/native_mmio.rs`,
`engine-wasm/src/xtensa/exports.rs`, `engine-wasm/src/xtensa/memory.rs`,
`src/engine/esp-xtensa/wasm-loader.js`, `src/sab/worker-entry.js`,
`src/engine/esp-xtensa/esp_engine_wasm.wasm`. Docs: `ble.md` (checkpoint)
+ this file (handover; read first). Repro: `tests/tmp-gatt.mjs`.
Reference: `tests/tmp-follow-run2.log` (8030 lines). Diag corpus: 65x
`tests/tmp-*.mjs` scripts + run logs (committed as reference; normally
untracked — keep them here, delete stale `tmp-adv-run*.log` before any
final commit). Build cache: `tests/build/local-compile/7d1e668ae5a30acb/`
(elf/map/bin for disassembly). `tests/compile-server.log` (server log).
AGENTS.md mission section (2026-09-15) + BTDM tracker `[ACTIVE]` entry
remain the standing orders; remove `[ACTIVE]` only when ADV+GATT both
PASS.

## 12. Repro script anatomy (tests/tmp-gatt.mjs, 90 lines, READ in full)

- Compile path: `POST http://localhost:5525/api/compile/start` with
  `{code, target:'esp32', targetEngine:'frontend',
  fqbn:'esp32:esp32:esp32'}`, then polls
  `/api/compile/status/<buildId>` every 1s until `success` (takes
  `binary_content` base64 + `buildId`) or `failed` (throws with `error`).
  Firmware marker `// gatt1` is appended to force a fresh build.
- Firmware source (Arduino sketch for ESP32): includes `esp_bt.h`,
  `esp_bt_main.h`, `esp_gap_ble_api.h`, `esp_gatts_api.h`, `nvs_flash.h`.
  Strong overrides `btClassicInUse()/bleInUse()` return `true` — the
  documented weak-function mechanism. Without these, `initArduino()`
  releases BT memory and controller init fails with 0x103 (lesson from the
  DONE controller-init work in AGENTS.md). `SVC_UUID 0x00FF`, one static
  `svc_handle`.
- `gatts_cb`: on `ESP_GATTS_REG_EVT` prints `GATT_REG_DONE` and calls
  `esp_ble_gatts_create_service(g, NULL, 4)`; on `ESP_GATTS_CREATE_EVT`
  prints `GATT_CREATE_DONE h=<handle>`, stores the handle, calls
  `esp_ble_gatts_start_service`; on `ESP_GATTS_START_EVT` prints
  `GATT_SVC_STARTED`.
- `gap_cb`: on `ESP_GAP_BLE_ADV_DATA_SET_COMPLETE_EVT` prints
  `ADV_DATA_DONE`, builds `esp_ble_adv_params_t` (`0x20/0x40`,
  `ADV_TYPE_IND`, `ADV_CHNL_ALL`,
  `ADV_FILTER_ALLOW_SCAN_ANY_CON_ANY`), calls
  `esp_ble_gap_start_advertising`; on
  `ESP_GAP_BLE_ADV_START_COMPLETE_EVT` prints `ADV_START_DONE`.
- `setup()`: `Serial.begin(115200)`, `ADVBOOT`, `nvs_flash_init()`,
  `BT_CONTROLLER_INIT_CONFIG_DEFAULT()` with `cfg.mode=ESP_BT_MODE_BLE`
  (explicit mode — sdkconfig-default BTDM(3) mismatch returns 258
  `ESP_ERR_NO_MEM`; AGENTS.md), `esp_bt_controller_init`,
  `esp_bt_controller_enable`, then `BDINIT=` (0), `ENABLE=` (0),
  `BLU_ENABLE_DONE`, `GAPREG=` (0), `GATTREG=` (0), `ADVCFG=` (0; adv data
  with `include_name`, 4-byte `ADV1` manufacturer data, `include_txpower`),
  `GATTAPPREG=` (`app_register(0)`), `SETUP_END`. `loop()` prints
  `LOOP_ALIVE` every 2s (never observed — the wall hits first).
- Harness: 4MB SharedArrayBuffer flash, ROM from
  `rom/esp32-v3-rom.bin`, one `SimulatorWorker`, `chunkSize=20000`,
  `init('ESP32', {flashSizeMB:4, mmuPages:64, strapValue:0x13,
  budget:10000000}, flash, rom)`, `run()`, 18-string mark watch
  (`ADVBOOT/BDINIT=/ENABLE=/BLU_ENABLE_DONE/GAPREG=/GATTREG=/ADVCFG=/
  GATTAPPREG=/SETUP_END/GAP_EVT=/ADV_DATA_DONE/ADV_START_DONE/GATT_EVT=/
  GATT_REG_DONE/GATT_CREATE_DONE/GATT_SVC_STARTED/assert/Guru`), 500ms
  poll for 180s, early break on `GATT_SVC_STARTED`, then `stop()`, final
  `pollUart()`, UART slice from `ADVBOOT`, `MARKS=` JSON, terminate.

## 13. Boot-sequence walk (reference log head, observed lines 1–40)

```
[gatt] build=7d1e668ae5a30acb
[PTEDBG] flashMMUMap.size=244 pte0x400C2000=3 pte0x40080000=1 pte0x3F400000=3
[DBG] Chip created, ROM=true flashLen=4194304
[DBG] Flash SAB-backed: 4194304 bytes
[DBG] ROM loaded: 455722 bytes, ROM[0]=undefined
[DBG] after applyBasicSetup: PC=0x40000400 flash[0]=ff ROM[0]=undefined
[DBG] Loading WASM (1599214 bytes) from ../engine/esp-xtensa/esp_engine_wasm.wasm...
[WASM] [HOOK] new=4010ff34 ... done=00000001 miss=00000080   (full line: §3)
[WASM] [LOOP] pc=4000fca9 ...                              (boot ROM loop, NORMAL)
```

Notes: the `[HOOK]` census prints at engine init — the scan runs on the
first `bt_shim_step`, before any BT firmware executes (resolve-once per
boot). `ROM[0]=undefined` / `flash[0]=ff` are logger artifacts of the SAB
views at that point, not faults: boot proceeds normally through `[LOOP]`
at `0x4000fca9`, the known-normal boot path (`0x4000FC90–0x4000FDE9` per
AGENTS.md — do not re-diagnose it). The `1599214` byte size must match the
committed wasm or you are testing a stale engine (§1 gotchas).

## 14. Commit history on this branch (git log, newest first)

- `a99de84` BLE checkpoint: preserve ADV/GATT work as-is (THIS branch
  head; 243 files, +1030645/-194 — the bulk is committed `tests/tmp-*`
  logs).
- `051ddae` BTDM run178: force-clear ack kills the 1.1M-vector storm (CC
  still open) — `main` HEAD, branch point, tree is clean there.
- `b92b94a` run177: LEVEL + ISR-exit re-arm + WSR-INTENABLE mask.
- `481e99d` run176: timer fix + LEVEL IRQ + native_bt_diag + storm
  guards.
- `a18b41a` run175: LEVEL IRQ semantics + CLOCK_CONFIG unmask.
- `51f139f` phase 7: HCI transport + LL RESET CC responder (transport
  corrected).
- `34a3f19` phase 6: per-build addrs, modem 2nd page, ADV/GATT probes.
- `5f0946b` phase 5: modem status gates + ACK-consume + WEDGE
  observe-only.
- `68933db` exp E1: CPU25 unmask + vec census + ISR trace capture,
  repoint-only queue fix, drop hli emulate.
- `01a3943` phase 4: fix RFI breaker + stale-queue TLSF corruption,
  mailbox repoint, pacemaker-only epochs.
- `04e72c4` phase 3: scheduler-queue plumbing + hlevel shims + rfi-self
  break (AGENTS.md phase-3 entry).
- Older: `06c57fb` HW-exact RFI PS save/restore (fixes stuck INTLEVEL),
  `d4aa4b1` IPv6 board-gateway + BT LL interrupt delivery, plus the
  pre-existing `ble/synthetic-cc` branch (`83effeb` WIP HCI CC synthesis
  scaffold — does not deliver; archaeology only).
Every BTDM commit message ends `(CC still open)` — that tag is the honest
trail: the Command Complete never arrived in any phase.

## 15. Exact-code atlas (line numbers verified on this tree)

`engine-wasm/src/native_mmio.rs` (11369 lines):

- Per-boot reset block ~2100–2145: zeroes `FUTSEM_TAB`,
  `FUTWT_TAB/RING/POS/PIN`, `PREPOST_SLOT/SEM`, `HOOK_SCAN_DONE`,
  `HOOK_MISS_N/FAIL_N/LOG_DONE/SKIP_N`, restores
  `HOOK_BD_TR=0x400e2adc` / `HOOK_BD_AW=0x400e2ae4`, clears `HOOK_CBSET` /
  `CB_SLOT_TAB` / `ADV_SPOOF_STAGE/TICK` / `GATT_SPOOF_STAGE/TICK` /
  `HOOK_BTRANSFER`, then calls `reset_boot_diag()`.
- Hook statics ~2219–2257: `HOOK_NEW 0x40107e58`,
  `HOOK_READY_RETW 0x40107e3b`, `HOOK_TAKE 0x40108c24`,
  `HOOK_TAKE_POST 0x40108c27` (= entry+3), `HOOK_GIVE 0x40108c10`,
  `HOOK_FREE 0x40107e40`, `HOOK_AWAIT_EB2 0x40107eb2`,
  `HOOK_AWAIT_RETW 0x40107ebb`, `HOOK_OFREE 0x40107a20`,
  `HOOK_TAKE_RETW 0x40108c3e`, `HOOK_CB 0x400e2b1c`,
  `HOOK_BD_TR 0x400e2adc`, `HOOK_BTRANSFER 0` (scanner-filled),
  `HOOK_CBSET 0` (scanner-filled), `CB_SLOT_TAB[16]`,
  `ADV_SPOOF_STAGE/TICK`, `GATT_SPOOF_STAGE/TICK`,
  `HOOK_BD_AW 0x400e2ae4`,
  `HOOK_SCAN_DONE/MISS_N/FAIL_N/LOG_DONE/SKIP_N`, `FUTSEM_TAB[8]`,
  `FUTWT_TAB[8]/RING[32]/POS/PIN[8]`, `PREPOST_SLOT/PREPOST_SEM`.
- `bt_hook_scan()` ~2268: resolve-once per boot; writes `HOOK_NEW`
  (~2475), `HOOK_TAKE_POST = take+3` (~2517), `HOOK_TAKE_RETW`
  (~2532/2535), `HOOK_FREE/HOOK_OFREE` (~2541/2542), `HOOK_BTRANSFER`
  (~2544), `HOOK_AWAIT_EB2` + retw scan loop (~2565/2576),
  `HOOK_READY_RETW` (~2624/2629/2633), `HOOK_CBSET = addr+3` (~2834),
  `HOOK_BD_TR` / `HOOK_BD_AW = addr / addr+8` (~2870/2871).
- LL IRQ: `bt_raise_ll_irq()` ~3015 (raises sources 6+7 = RWBT/RWBLE),
  `bt_ack_ll_vector()` ~3059 (edge-ack on vectoring, called from
  `take_interrupt` in state.rs when CPU-25 pends).
- `bt_shim_step()` ~3202 (per-step entry, `BT_DIAG_DISABLE` early-out,
  `bt_hook_scan()` at top). Snapshot/restore comment + TKO hook
  ~3226–3300. TKG diag + prepost + take-emu ~3503–3808 (§16).
  Consume-trace (EB2/FRE/EBB) + poison restore + TKR ~3820–4001 (§17).
  Give-path emulation at `HOOK_READY_RETW` ~4035 (§17). `:3a9`
  NULL-item emulation (`pc==0x400937b8 && a3==0`, shape check,
  `msgs<=5 → msgs+1`, `pc=0x400937c4`) ~6079–6118. TR/cb_set legs
  ~6119–6234 (§4/§5). Wedge observe-only (`WEDGE_N>20000 &&
  WEDGE_DONE==0 && bt_bluedroid_started()` → log `wedgenolock`,
  `WEDGE_DONE=2`, never writes — the old `qb+104` seed was wrong for the
  hli ring, corrupts the heap-lock protocol) ~6500–6540.
  `bt_find_tcb()` ~6759 (+ `BTC_TCB` static ~6740),
  `bt_bluedroid_started()` ~6858, `bt_queue_for_tcb()` ~7406.
  `qgate-nobtc/noqvar/notcb/noqueue` gate-trip tags
  ~7259/7278/7318/7333. Spoof pumps `bt_adv_spoof_pump()` ~7648 /
  `bt_gatt_spoof_pump()` ~7678 (prints REMOVED). `reset_shim_diag()`
  ~11355.

`engine-wasm/src/xtensa/exports.rs` (1555 lines):

- `PC_TRACE_LEFT/CORE` statics ~29–30; `reset_boot_diag_pub()` ~128,
  `reset_boot_diag()` ~132; `native_pc_trace(n)` ~182
  (`PC_TRACE_LEFT=n, CORE=2`); seg3 export `native_flash_seg3_off()`
  ~271; T-line block ~483 (trailing core digit); `[FREEZE]` ~552
  (pc/op/a0-a3/a8/a10/a12-a15); RETW tracer ~571 (budget 6, both cores,
  `a2` + `[a2+16]`); `[RING]` ~756 (pc/a0/a1/a8 + regs); btc_init entry
  T-arm ~967 (600 insns); FENT whitelist ~1100–1160 (multi-build
  literals, §7); `FENT_N` budget 8000 ~1161; watchpoints ~1472/1484.
  New FENT literals this work: `4010fcf8` + `40110d2c` (~1124/1125),
  `4010b758` (~1123), `4010c6f4` (~1141) — scanner values stay
  authoritative.

`engine-wasm/src/xtensa/memory.rs`: `init_flash()` ~93;
`FLASH_SEG3_OFF` + `init_flash_seg3()` ~112–114;
`flash_mirror_read_u32()` ~116 (`off = SEG3 + (vma-0x400D0020)`).

JS side: `wasm-loader.js` seg3 push ~484–499; `worker-entry.js` inline
CMD_PCTRACE/CMD_WATCHPOINT/CMD_READ_MEMORY ~303–355,
`PAGE_TABLE_OFFSET` fix ~1103–1111. Layout truth
`src/engine/wasm-memory-layout.js`: `STATIC_ZONE` = 4MB (~20),
`PAGE_TABLE_OFFSET = STATIC_ZONE` (~22), `REGION_TABLE_OFFSET` (~26),
`RAM_DATA_OFFSET` (~28), `FLASH_DATA_OFFSET` (~33),
`MMU_TABLE_REGION_ID = 5` (~36). Compile server
`tests/compile-server.mjs`: `PORT = 5525` (~20), listen (~168–169).

## 16. Take path step-by-step (entry → post → retw; read, not inferred)

- At `HOOK_TAKE` (take ENTRY, caller frame, ~3564): `slot = a2+4`
  (caller a2 = future*). Match all 4 `FUTSEM_TAB` pairs. If the slot's
  queue is DRAM-valid with `q60==1 && q64==0 && q56==0` (len 1, no
  storage, zero msgs = give hasn't landed) and the `(slot,sem)` key is
  new: `dma_write(q+56, 1)` (pre-post), record `PREPOST_SLOT/SEM`,
  one-shot log `[SHIM] prepost` (`DG_PRE_N<2`). The CURRENT lap then
  succeeds natively. Reference run: x2.
- At `HOOK_TAKE_POST` (post-entry, own frame, three legs):
  TKO lap log (~3296, all known slots, `[TKO] slot=` attributable);
  TKG gate diag (~3503: match pair → read `q/[q+56]/[q+60]` → `[TKG]`
  log, `DG_TKG_N<8`; reference run x2, both `msgs=1 len=1`); take-emu
  (~3598, reference run x2, both consumed): restore `[slot]` from snap
  if poisoned → if `1<=msgs<=qlen`, consume (`[q+56]-=1`), log
  `[SHIM] take-emu` (`DG_TAKE_EMU_N<2`), then the exact exit sequence
  (~3767): `dma_write(slot, 0)` (neutralize sem_free), arm
  `PC_TRACE_LEFT=400, CORE=1` (post-emu trace), `set_ar(2, 0)`
  (native take returns 0 on success — run291), `[fut+8]=1` if still
  zero (run294 CB-effect), `pc = next_pc = HOOK_TAKE_RETW` (REAL retw
  in the real frame — run261: never fabricate windows), `return true`.
- Poison path (~3872): at `HOOK_TAKE`/`HOOK_TAKE_POST`, candidates `a2`
  (post, exact) / `a2+4` (entry, future*→slot). `[PZ]` log on
  `0xbaad5678` hits (`DG_PZ_N<8`; reference run x0 — waiter never
  re-laps post-poison), `[TKR]` slot/value log (`DG_TKR_N<60`; x6 in
  reference, none on our slot), osirestore from snap (`DG_OSIR_N<2`;
  x0 — nothing poisoned at lap time in the reference run).

## 17. Give path + consume-trace + :3a9 backstop (read, not inferred)

- run247 leg at `HOOK_READY_RETW` (~4035): the real give path never runs
  (give = entry + `l32i` + `callx8` into `xQueueGenericSend`, which parks
  in the heap-lock CAS spin — both cores observed in kernel lock/list
  spins, never in `prvCopyDataToQueue`). So the leg emulates kernel
  effects at the ready-return, gated on OUR future: slot restore +
  `xQueueGenericSend` msgs bump + suspended-list removal / ready-insert /
  TopReady / yield. `osi_free` swallow at `HOOK_OFREE` (leak 12B, heap
  untouched — free on recycled blocks asserts, run261/h30).
- Consume-trace (~3820): `pc == HOOK_AWAIT_EB2 || HOOK_FREE ||
  HOOK_AWAIT_RETW`, `_chk` = a7 (eb2) else a2, known-future check →
  `[EB2]/[FRE]/[EBB]` log (`DG_EBX_N<4`). Reference run: x0/x0/x0 —
  the emulated take return is never consumed downstream. At `HOOK_FREE`
  also invalidates any `FUTSEM_TAB` pair with this fut (run287).
- `:3a9` NULL-item emulation (~6090): `pc==0x400937b8 && a3==0` + queue
  shape OK + `msgs<=5` → `[q+56]+=1`, `pc=0x400937c4`, `[SHIM] a9emu`
  (`A9_N<4`). Covers semaphore-signals misrouted to isz-8 queues by
  recycled-block slot aliasing. Reference run: `a9emu` x2.

## 18. Wedge / qgate / RING / FREEZE instrumentation (read, not inferred)

- `wedgenolock` (~6520): fires when `WEDGE_N>20000 && WEDGE_DONE==0 &&
  bt_bluedroid_started()`. OBSERVE-ONLY by design: logs
  `mux/lv/c0=3ffc3ce0`, sets `WEDGE_DONE=2`, never writes (the old
  `qb+104` seed was wrong for the hli custom ring — no xLock word
  exists there — and corrupted the heap-lock holder protocol).
  Reference run x2203 = fully parked tail. `grep -c wedgenolock` =
  park depth.
- `qgate-*` (~7259–7333): `nobtc/noqvar/notcb/noqueue` = which gate
  tripped before queue repair. Reference run: x2 total.
- `[RING]` (exports ~756): ring-buffer trace at select pcs
  (pc/a0/a1/a8 + regs). Reference run x4, all early-boot.
- `[FREEZE]` (exports ~552): one-shot full-reg dump
  (pc/op/a0-a3/a8/a10/a12-a15). Reference run x1, early-boot.
- `[LOOP] pc=4000fca9` = normal boot-ROM loop, not a stall (AGENTS.md).

## 19. Diag corpus (65 scripts) + build cache + verify checklist

Scripts grouped by filename prefix (only `tmp-gatt.mjs` was read in
full this session plus `tmp-gatt-trace.mjs` header; CHECK THE HEADER of
any other script before running — some are stale-build probes):

- `adv*` (7): adv62, advdata, adv-e7f2, adve7, advloop, advval,
  advval2 — ADV-stage probes across builds e7f2/62ea/f130.
- `ble*` (3): ble-adv, ble-probe, ble-resume — earlier BTDM probes.
- `btc*` (4): btc180, btcscan, btctask, btctrace180 — BTC-task
  forensics.
- `stall*` (8): stall170/171/172/173/174/176, stall-probe —
  stall-park forensics. `takeprobe`, `tcb`, `tabptr`, `cbslot`,
  `cbdec180` — take/TCB/cb-slot forensics. `future*` (4):
  future180/180b/193, futurewr — future forensics.
- `q*` (4): qdrain180, qdump, qlong, qlong2 — queue-drain forensics.
- `wtest*` (6), `direct/direct2`, `ctl-probe`, `cur`, `dec-probe`,
  `enable180`, `env180`, `evt`, `gattval`, `nvs`, `pc`, `pctrace`,
  `pcworker`, `sched-probe`, `semdump`, `shim180`, `sus`, `swctx`,
  `timer-dbg`, `timg-probe`, `waiter`, `wpwrite`, `anatest` — mixed
  one-off probes (sems, scheduler, waiter, timer, NVS, analog).
- `gatt-trace.mjs`: PCTRACE-armed variant of the repro (header only).

Build cache: 74 dirs under `tests/build/local-compile/`; current build
`7d1e668ae5a30acb/out/` holds
`sketch.ino.{elf,map,bin,bootloader.bin,merged.bin,partitions.bin}` —
the ELF is the disassembly ground truth (§7). Map buildId from the
repro header `[gatt] build=...`.

Verify checklist (AGENTS.md order): `wasm-bench.mjs` (MMIO ~0/1) →
`test-wasm-standalone.mjs` (PASSED) → RMT boot (ALL TESTS PASSED) →
BLE repro (`ADV_DATA_DONE` grep) → full suite at the very end
(`run-worker-tests.sh --skip-bt --skip-wifi` + gateway tests
separately). `tests/compile-server.log` is the local server log;
`ble.md` is the 44-line checkpoint;
`tests/tmp-handover-btdm-2026-09-15.md` is SUPERSEDED for resume
(archaeology only).
