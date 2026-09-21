# Changelog

All notable changes to the `esp32emu` package are documented here.
This project follows semantic versioning (`MAJOR.MINOR.PATCH`).

## [Unreleased]

### Added
- IPv6 board↔gateway: SLAAC global ULA (`fd00::/64` via unicast gateway
  RAs with finite lifetimes + 30s periodic re-advertise), ICMPv6 echo and
  UDP echo, all crafted by hand in the gateway (`handleIPv6.go`; the gVisor
  stack is v4-NAT only). Board lessons: `sin6_scope_id` is the LWIP netif
  ifindex (`num+1`, station = 2, not 1); the Arduino `globalIPv6()`
  wrapper lags LWIP ground truth; never walk past
  `LWIP_IPV6_NUM_ADDRESSES`. New test: `test-worker-ipv6`
  (`SLAAC=OK PING6=OK UDP6=PASS`, gateway group).
### Fixed
- BT LL interrupt delivery (BTDM bring-up, advertising still blocked):
  the engine raised matrix source 3 (`ETS_BT_MAC_INTR_SOURCE`, marked
  "will be cancelled" in `soc/interrupts.h`) which routes to a dead CPU-6
  line — live FIRST_INTR_MAP dumps show the firmware maps RivieraWaves
  RWBT(src 6)→CPU 25 and RWBLE(src 7)→CPU 25. New `bt_raise_ll_irq()`
  raises 6+7 at all BT sites. The unmodeled RW status-clear registers made
  sticky raises storm the CPU-25 ISR (3.9M runs, LL task starved) and
  same-level nested vectors clobbered EPC4 with the `rfi` address
  (rfi-to-self park at `0x40083bca`); raises are now pulses
  (`BT_IRQ_PULSE`, cleared each pump) plus edge-ack in
  `CoreState::take_interrupt` when CPU-25 is pending. Result:
  `esp_bluedroid_init()` returns 0 (was stuck in BTC `future_await`);
  `esp_bluedroid_enable` still waits for the HCI_RESET Command Complete
  (doorbell rung, no reset/CC bytes in DRAM — host/LL mailbox mismatch,
  under investigation). No behavior change for non-BT tests (hook is
  inert unless CPU 25 is pending).
- RFI restores PS (HW-exact): `take_interrupt` now saves PS to EPS_level
  (free slots 184+idx-1) and `_handler3` (rfi) restores it; `exception()`
  saves the level-1 slot. Without this, ISR levels whose stubs lack an
  explicit `wsr.ps` (e.g. the level-4 BT dispatcher, unlike level-1) left
  INTLEVEL stuck high and wedged every level<=4 interrupt (ticks included)
  forever. Full suite green (55/0/7).
- Virtual SD card: the native SDMMC host now backs a full SD stack —
  init (CMD0/8/55/41/2/3/7/9/16), R1/R2/R3/R6/R7 responses, SDHC CSD sized
  from the image, SCR (ACMD51), and FIFO + DMA block transfers into a
  host-owned image (`config.sdCard: { sizeMB?, blocks?, image? }`, default
  16MB zero-filled, SAB-backed and shared as the `sdcard` memory region).
  `SD_MMC.begin()` (1-bit and 4-bit + format-if-needed), file
  write/read-back, multi-block files and FAT persistence across simulator
  restarts all pass. New tests: `test-worker-sdcard`,
  `test-worker-sdcard-4bit`, `test-worker-sdcard-persist`.
  Debug aid: `native_sd_trace()` gates per-command `[SD ...]` logging.
- Touch + DAC engines: `SENS.touch_meas` returns host `config.touchInputs`
  counts (default 1000 = untouched, 8↔9 swap per `touch_ll`), one-shot
  measurement completes instantly (`touch_meas_done`), and `dacWrite()`
  (RTCIO `pad_dac.dac` code) feeds a virtual wire to ADC2 so
  `analogRead(25/26)` observes the DAC voltage. New test:
  `test-worker-touch-dac`. Also fixed SENS reads returning 0 for all
  non-ADC offsets (broke CTRL2 read-back); they now return the register file.
- EMAC function: MDIO PHY model (LAN8720 `0007/C0F1`, link-up BSR,
  instant busy-clear) and a TX→RX DMA descriptor loopback pump (TI/RI +
  NIS summary + irq 38, STATUS write-1-to-clear). New test:
  `test-worker-emac-loopback` (PHY ID/link + 64B loopback, byte-exact).
- SPI slave virtual bus: single-chip master↔slave exchange through the
  slave W-regs (slave TX preload → MISO, master MOSI → slave RX), with the
  slave RX bit counter (offset 100) and SLAVE TRANS_DONE maintained by the
  bus so `spi_slave_get_trans_result` reports the real `trans_len` and
  fills `rx_buffer`, plus `post_trans_cb`. Slave CMD[USR] (`user_start`)
  is a silent arm — completing there fired a phantom TRANS_DONE whose
  store ran before any MOSI arrived (`trans_len=0`, empty RX). New test:
  `test-worker-spi-slave` (2 re-queued full-duplex transactions).
- TWAI TX/RX: SJA1000 STATUS parity (RBS/TBS-always/TCS-latch/DOS-latch,
  TCS clears on next TX_REQ, overrun clears on CMD bit3) so the driver's
  TCS poll loop exits; plain TX_REQ self-receives in SELF_TEST HW mode
  (what IDF NO_ACK mode uses). `test-worker-twai` now transmits a std
  frame (ID 0x123) and receives it back byte-exact.
- RMT TX/RX virtual wire: FrcTimerAlarm channels 200+i (RMT TX pacing)
  are now dispatched to `RmtChannel::handle_event` (previously dropped,
  so TX could never complete); fixed the channel `rmt` back-pointer
  (pointed at `new()`'s dead local — TX deref was UB); TX completion
  streams items into RX_EN-armed siblings (RAM copy + MEM_OWNER flip +
  RX_END + CHnSTATUS WADDR so the RX ISR's length math works).
  `test-worker-rmt` now TXs 2 NEC-like items on ch0 and receives them
  byte-exact on ch2 via ringbuffer.
- LEDC hardware fade: DUTY_START now runs the fade engine (was: instant
  FADE_END with no ramp) — target from DUTY_NUM step count, duration
  from num/cycle/scale over the channel timer, DUTY_R interpolates
  (clamped to [start, target]; reported << 4 per DUTY_RD layout), and a
  new `LedcFadeEnd` event latches the target + raises FADE_END so
  `ledc_fade_start(WAIT_DONE)` returns. Also fixed LS timer clock
  polarity (TICK_SEL=1 is APB per HAL/TRM, was inverted to REF →
  ~80x slow). `test-worker-pwm` now fades 0→255 over 1000ms and reads
  back exactly 255.
- MCPWM capture/fault: GPIO-matrix-routed pin edges now drive MCPWM
  inputs (CAP channels latch a warp-tolerant monotonic APB timestamp
  into CAP_CHN + edge type into CAP_STATUS + CAP interrupt; fault
  inputs latch EVENT_F bits + fault interrupt on entering the active
  pole level). Added INT_ENA/ST/CLR plumbing (irqs 39/40) and fixed
  wrong companion-table rows that ate INT_ENA writes. `test-worker-
  mcpwm` now captures two host-driven rising edges (values increase,
  pos edge, ISR callback fires) and detects a driven fault (EVENT_F0).
- Touch IRQ: the attach/fire path works end-to-end through the
  Arduino v2 driver (threshold-gated per-pad dispatch, verified with
  negative control); added `setTouchInput(pad, count)` host API
  (worker command 14, mirroring `setPinInput`) for live threshold
  crossing. `test-worker-touch-dac` now also verifies no-fire while
  untouched plus fire after driving the pad below threshold.
- TSENS: `SENS_SAR_SLAVE_ADDR3` TSENS_OUT field now reads 77F, so
  Arduino `temperatureRead()` returns 25.0C (was 0F/-17.8C from a
  zeroed register). New test: `test-worker-tsens` (range + stability).
- BOD: brownout detector modeled (ENA + threshold select + DET live
  bit + INT_RAW latch + INT_ST/CLR plumbing on RTC irq 46 + RST_ENA
  reboot with `ESP_RST_BROWNOUT`). Host `setVoltageMv()` API (worker
  command 15) drives the rail; reset restores nominal to avoid boot
  loops. New test: `test-worker-bod` (IDF brownout reboot + direct
  reset-reboot with reason 9 across 3 boots).
- ADC continuous/DMA: the DIG controller's I2S0 RX-DMA path now delivers
  real samples — pattern tables (SYSCON SAR1/SAR2 tabs) + host analog
  inputs are synthesized into TYPE1 frames on the I2sRx pump while
  `data_to_i2s` is set, filling the driver's own DMA descriptors so its
  RX_EOF ISR feeds the read ringbuffer. Also fixed a load-bearing I2S
  bug: `old_val` was read AFTER the register-file store, so CONF
  TX/RX_START edges could never fire and I2S RX/TX could never start
  (root-caused via temporary native js_log_str tracing — the "phantom
  writer" was the stale read itself). Side effect: `test-worker-i2s`
  now reports WRITTEN=64 (TX actually completes). New test:
  `test-worker-adc-cont` (128/128 TYPE1 frames, ch0, plausible data).
- DS (digital signature): native register-file stub (6th SWEEP page at
  0x3FF1A000, plain RW + zeroed reset). Full signing is deliberately
  out of scope: the DS flow needs the HMAC peripheral and Arduino esp32
  3.3.10 refuses to compile any DS driver code (`esp_hmac.h: #error
  "HMAC peripheral is not supported for the selected target"`), so no
  consumer can reach it. New test: `test-worker-ds` (raw MMIO
  write-readback + zeroed-reset reads).
- eMMC virtual card (`config.sdCard.type: 'mmc'`): the SDMMC host now
  answers the full MMC probe path — CMD1 SEND_OP_COND with OCR
  busy-then-ready (exercises the driver retry loop), host-assigned RCA
  on CMD3 (arg RCA != 0; SD assign path unchanged), MMC CID/CSD,
  CMD8 SEND_EXT_CSD (R1 + staged 512B: EXT_CSD_REV=8, CARD_TYPE=26MHz,
  SEC_COUNT=virtual blocks) selected by arg==0 (0x1AA keeps the R7 echo),
  CMD6 SWITCH (BUS_WIDTH recorded, instant R1) — while CMD8/0x1AA +
  CMD55 get no response in MMC mode so the SD probe times out and the
  driver falls back to MMC. Sector R/W reuses the shared data path.
  New test: `test-worker-emmc` (is_mmc/RCA/capacity via negotiation,
  manual EXT_CSD readback, sector magic round-trip).
  - Known upstream quirk (documented in the test): `sdmmc_card_init`
    returns 262, verified by RET-trace to come solely from
    `sdmmc_init_mmc_read_ext_csd`'s `blti mmc_ver, 4` skip (returns the
    preset 0x106 without sending CMD8). `csd.mmc_ver` has no producer
    anywhere in this IDF build's init flow (no store to csd+4 in cid/csd
    decode — stays 0 from memset), so the gate is unsatisfiable on real
    HW too. Also fixed en route: MMC CSD TRAN_SPEED belongs in r3[7:0]
    (was OR'd into r2's C_SIZE), and CLKSRC now resets sourced (both
    slots source 1) to match the always-on virtual clock.
  - Second upstream quirk: manual no-data commands via
    `sdmmc_host_do_transaction` (e.g. SWITCH, CMD13) never complete
    (host waits for a data event nodata never posts; verified: commands
    are received+answered, host times out). The driver itself never sends
    SWITCH in this flow, so nothing observable is lost.
- SDIO slave: `sdio_slave_initialize` + recv-register/load-buf + start
  now work (new test `test-worker-sdio-slave`: init/start PASS, recv
  correctly times out after 500ms with no host, stop/deinit clean).
  Two fixes: (1) the SLC RX engine never signaled reset-complete —
  CONF0 RX_RST now raises INT_RAW RX_DONE (without it `start()` spins
  on RX_DONE forever); data-path DONE bits stay clear so recv still
  times out honestly. (2) A real addressing bug: the SLC region writer
  passed `addr & 0xFFC` (page-stripped) while the peripheral computed
  offsets via subtraction, silently misrouting every write (found via
  PC histogram on core 1 pinning the spin to send_start's RX_DONE poll,
  then observing forced values never land); offsets are now mask-based
  and the region passes the full aligned address.
- I2S RX (virtual microphone): host-fed RX audio via new
  `proxy.feedI2SRX(sample)` (worker CMD 16 → `native_i2s_push_rx_sample`,
  staged into pairs for feed_rx_data's packed channel modes). New test
   `test-worker-i2s-rx` (legacy RX install, 512 fed words, 256B read back
   byte-exact). Note: legacy DMA descriptors exceed 64 words, so short
   feeds never complete one — feed generously. Camera (DVP via
   I2S-camera DMA) has since been implemented — see below.
- ULP (FSM coprocessor) interpreter: new `ulp.rs` executes programs from
  RTC_SLOW_MEM synchronously to HALT/END on the RTCCNTL+0x2C force-start
  bit (entry from SENS+0x2C[21:11]); full ALU/ST/LD/BRANCH plus ADC
  (host analog inputs), TSENS (=77F), DELAY-nop; WR/RD_REG/I2C stubbed,
  WAKE/sleep-timer effects not modeled. RTC_SLOW sharing rides the
  existing native page-fallback (same SAB the CPU uses). New test
  `test-worker-ulp` (macro program: MOVI/ST/ADD/LD/direct-BX/HALT,
   magic + sum + readback verified through real `ulp_run`).
- I2C slave ACK fix (latent `static mut` aliasing UB): `write_next` read
  the ACK from `self.bus_slave` AFTER the `i2c_bus_write_byte` call that
  mutates the same I2CS slot through the static — the compiler cached the
  stale None and every addressed byte NACKed (master got ACK_ERR → 0x103,
  slave saw nothing). It passed at introduction and broke after an
  unrelated rebuild (pure codegen fragility, no logic change). The ACK is
   now the bus call's return value. SPI's `spi_bus_exchange` already
   returns by value (clean). Full suite: PASS=48 FAIL=0 SKIP=4.
- Deeper ULP: WR_REG/RD_REG routed to the live native RTC_CNTL/RTC_IO/
  SENS register files (periph_sel 0/1/2, addr=(reg/4)&0xff, R0 =
  reg[high:low] right-aligned per the I_RD_REG doc; >8-bit data / >16-bit
  R0 truncations documented; RTC_I2C sel 3 + I2C insn still stubbed);
  END regrouped to documented semantics (I_WAKE latches + keeps running
  to HALT, I_SLEEP_CYCLE_SEL records + keeps running, plain END stops);
  I_WAKE sets a latch consumed by the deep-sleep wake path (ULP cause
  BIT(9)=RTC_ULP_TRIG_EN wins over timer when ULP wakeup was enabled —
  first-trigger race parity; latch read pre-reset_soc + WAKEUP_ENA
  snapshotted at sleep entry since reset re-seeds the register; stale
  latches cleared on wake + SoC reset). Reentrancy guard for WR_REG back
  into the force-start register. New `test-worker-ulp-wake` (WR/RD_REG
  round-trip via STORE0 in both directions + 2-boot deep sleep asserting
  cause 6/reason 8, wstate 0x6200). Wake TIMING still follows the sleep
  timer (sync model runs ULP at ulp_run, not during sleep); deep sleep
  still requires the timer enabled (pre-existing gate). Suite: PASS=49.
- `static mut` aliasing audit (same-slot re-access after a mutating call
  — the I2C ACK class): I2C fixed; SPI (sibling-only + return value),
  RMT (sub-structs, single borrow), RTC/ULP (locals + ignored return),
  WiFi (single-borrow chains, private accessor), I2S/UART/TWAI/SDMMC/
  GPIO/ADC (no reborrow in methods; Copy indices, pend-only interrupts)
  all clean. No changes needed beyond the I2C fix.
- TWAI dual-node virtual bus: NORMAL-mode TX captures to a TX mailbox
  (no self-reception — SELF_TEST only); `native_twai_push_rx` delivers a
  peer frame through the real acceptance filter + FIFO + RX interrupt, and
  `native_twai_get_tx` exposes the capture. Host API: `sendTwaiFrame(id,
  data, {ext, rtr})` (two-phase CMDs 17/18 over the 3-arg channel) +
  `getTwaiTx()` (CMD 19). Also fixed a native-only ISR storm: reading
  INT_RAW consumed the flag without re-evaluating the interrupt line, so
  500K+ empty ISRs starved NORMAL-mode TX. New test:
  `test-worker-twai-normal` (TX capture + peer RX `0x123/CAN!`).
- RTC_I2C engine + ULP I2C: new `RtcI2cPeripheral` at 0x3FF48C00 (CTRL /
  SLAVE_ADDR / DATA / INT_RAW / INT_CLR, pointer-model single-byte
  controller writes, 4-slot virtual slave table, DONE bits, table-full
  drop, addr-0 no-match) plus the ULP I2C instruction via SENS slave
  addresses (unprogrammed = 0xFF NACK). Also fixed `PeripheralBase`
  sub-page addressing (`addr & 0xFFF` broke RTC_I2C/SENS sharing one 4KB
  page — now `wrapping_sub(base) & 0xFFF`) and a TSENS/SENS+0x44 overlap
  (high bits read 77F, low 22 bits live). New test:
  `test-worker-rtc-i2c` (`ULP_I2C=ab ff 5a`).
- Virtual OV2640 camera (DVP via I2S): banked SCCB register file at 0x30
  (PID 0x26, sticky sub-address pointer) served through the virtual I2C
  bus; the sensor packs the ramp per SM_0A00_0B00 (one byte per DMA
  element in sample1, matching the driver's yuyv_highspeed filter) with a
  finite frame (`config.camFrameBytes`, default QQVGA RGB565 38400);
  in_suc_eof fires per rx_eof_num ELEMENTS (one driver ping-pong half —
  the register counts 4-byte elements, not bytes); copy_in completes +
  advances eagerly so the frame-final descriptor never strands its EOF
  (exactly one short per frame before the fix). New test:
  `test-worker-camera` (SCCB probe + init + full frame `CAM_FB=38400 160
  120 CAM_DATA=PASS`). Suite: PASS=52 FAIL=0 SKIP=4.
- Compile-server retry fix: failed/errored job ids are re-enqueued
  instead of poisoning the id forever (transient PyInstaller exit-255
  failures).
- ESP32-CAM board support: new `board` config (`'esp32'` default,
  `'esp32-cam'` AI-Thinker preset: 4MB flash + 4MB PSRAM, explicit sizes
  win, unknown ids throw) threaded through `ESP32`/`SimulatorWorker`
  (`proxy.board`, chipInfo) with `ESP32_CAM_PINS` (camera + LED pinout)
  exported from the package index. Firmware PSRAM detection works via a
  `native_spi_set_psram_size` seed bridge (SPI1 JEDEC ID reports the
  configured chip; validity/size decode verified against the IDF
  KGD/EID macros). New test: `test-worker-esp32-cam` (board preset
  asserts + `psramFound()=1` + 4KB SPIRAM heap write/read-back + full
  camera frame `CAM_FB=38400 CAM_DATA=PASS`; compile with
  `esp32:esp32:esp32:PSRAM=enabled`, like the real module default).
- Compile server honors the client fqbn (validated ESP32-only) and keys
  the build cache by fqbn + sketch hash, so menu options like
  `PSRAM=enabled` actually take effect.
- ESP32-CAM protocol parity: new `tests/board-hook.mjs` forces
  `board: 'esp32-cam'` into any stock worker test
  (`node --import ./tests/board-hook.mjs tests/test-worker-gpio.mjs`);
  cross-section gpio/uart/spi/i2c/timer/pwm/analog all PASS on the module
  (same silicon + same default memory map, so the whole suite applies).
  Protocol matrix + module notes: `docs/esp32-cam.md`.
- WiFi protocol battery: new `test-worker-net-protocols` (gateway group)
  verifies DNS, outbound HTTP (200), NTP/UDP, ICMP ping to gateway AND
  internet, and an MQTT CONNECT→CONNACK handshake; pcap writer fixed to
  uniform big-endian (records were LE — Wireshark misparsed everything);
  network tests need small `proxy.chunkSize` (default 500K-step chunks
  stall gateway delivery ~1s while sim time races ahead). Matrix +
  advisory: `docs/networking.md`.
- CoAP verified: hand-rolled CON GET → ACK 2.05 client in the battery,
  ×3 legs (early/late/local-responder + public internet server), all
  PASS. Two test-code bugs found by the investigation (both in the test,
  not the emulator): CoAP header bit-shifts (`(b>>4)==1`, `(b>>2)&3`
  reject every valid ACK — version is bits[7:6], type bits[5:4]) and a
  `0x80` response byte (ver=2/CON) instead of `0x60` (ver=1/ACK).
  UDP waits busy-spin (delay()-polling burns 40 sim-sec before a
  40ms-wall reply arrives); NTP refactored to a retrying `doNtp`
  helper; NTP2 server switched to `time.windows.com` (Cloudflare
  blackholes NTP from here).
- CoAP SERVER role on the board: firmware binds UDP 5683 and answers CON
  GET with ACK 2.05; new `test-worker-coap-server` plays client from the
  host (3 served + 3 ACKs asserted). Required a gateway UDP forward
  (`127.0.0.1:5683 → board:5683`, raw-frame injection + reply snoop —
  the VN Dial API is TCP-only) with per-DHCP room rebinding, plus a
  gateway ARP responder for 192.168.4.1 (gVisor's cold stack ignores the
  first ARP ~1s, killing the session's first reply).
- Soft-AP mode: `WiFi.softAP()` init/IP/station-count are live driver
  paths; fixed the softAP MAC (eFuse area now seeded natively with CRC8
  — zeros gave `00:00:00:00:00:01`, a CRC-less partial seed aborts
  `ESP_ERR_INVALID_CRC`). New `test-worker-softap` (gateway-free).
  Association is untestable (no virtual station exists).
- Two-node ESP-NOW delivery: TXDMA snoop for action frames (type 0 /
  subtype 13 / `0x7F` / OUI `18:FE:34`; offsets relative to `buf[0]` —
  this config has `tx_header: false`), `js_espnow_tx_frame` FFI, room
  broadcast with an `E5 50 4E 57` marker the gateway keeps out of
  gVisor, peer injection via the normal RX DMA path. New
  `test-worker-espnow`: A↔B broadcast `RXCB=68656c6c6f` both ways.
  Unencrypted only; encrypted/broadcast-stress untested.
- IPv6 verdict: link-local comes up (`enableIPv6()=1`, EUI-64, no crash)
  but there is no global route (gateway is v4-NAT only) — documented,
  unsupported.
- Fixed a latent JS-fallback size-convention bug: the Rust engine passes
  `map_read`/`map_write` sizes in BITS (8/16/32) but the loader mapped
  only 1/2 → every fallback access ran as 32-bit, zeroing neighbor bytes
  on sub-word PSRAM stores (heap canary corruption:
  `Bad tail ... got 0xbafffff0`). Slept until PSRAM came alive — the
  fallback previously served only GDB writes + the untouched DROM0
  window. Accepts both conventions now.

### Notes
- Hall sensor: no driver path exists (IDF 5.x removed `hall_sensor_read`,
  Arduino 3.x has no `hallRead`) — SENS hall control bits stay plain RW.
- MCPWM/UHCI/SDIO-slave/FE/SWEEP/BT-RF remain register-file stubs (IDF
  caches state in RAM); existing tests verify readback behavior.

## [0.1.6] - 2026-09-02

### Performance
- `writeSABState()` split into fast path (always, ~10 typed-array writes) and
  heavy debug path (every 4th call: register dumps, MMIO reads, peripheral
  snapshots). Memory hashes reduced from every call to every 16th call
  (~8M cycles). Net effect: ~27K → ~5K debug SAB update calls/sec,
  ~1.7K → ~300 memory hash calls/sec.
- `writeSABState()` interval doubled: 262K → 512K cycles, halving the update
  frequency. Firmware boots and passes all tests identically.

## [0.1.5] - 2026-09-01

### Performance
- Optimized JS hot loop in `worker-entry.js`:
  - Cached WASM export references (`native_idle_advance`, `native_pump_events`)
    and `clocks.root.fireDueEvents` as locals before the while loop.
  - Inlined `coresIdle` as 4 direct SAB typed-array reads (bypasses getter
    chains on `WasmCore`).
  - Skipped `syncClockState()` on idle path — `native_idle_advance` already
    syncs `CLK_CYCLES` internally (saves 1 FFI call per step when idle).
  - Cached `step()` exports as `_stepExp` (invalidated on `reset()`).
  - Removed first-5-step debug logging from hot path.

### Type Checking
- Fixed all 22 `tsc --checkJs` errors (22 → 0):
  - `memory.js`: field-level type declarations for `table`/`tableOffset`.
  - `worker-proxy.js`: fixed function types, added `@types/node`, `any` casts
    for `_chipInfo`/`memory`, Worker `.on()` cast for Node.js compatibility.
- Added `@types/node` devDependency and `types: ["node"]` to `tsconfig.json`.

### CI
- New `typecheck` job: runs `npx tsc --checkJs --noEmit` on every push/PR.
- Enhanced `worker-summary` job: starts compile server + gateway server when
  `run_wifi=true` via workflow_dispatch; per-test timeout increased to 600s.

## [0.1.4] - 2026-09-01

### Performance
- FFI call reduction in the worker hot loop. Two new combined WASM exports replace
  multiple per-step FFI calls:
  - `native_pump_events()`: merges all 5 native timer pumps (TIMG0, TIMG1, FRC,
    event queue, BT RF) into a single FFI call. Busy path: 7 → 2 FFI per step.
  - `native_idle_advance(js_cycles)`: merges clock sync + idle fast-forward + all
    pumps into a single FFI call. Idle path: 9 → 2 FFI per step.
  Combined, this reduces per-step FFI calls by 4-7× depending on CPU state.

### Dead Code Removal
- Removed 16 inert `EmptyPeripheral` instances from `esp32.js` — all pages have
  native WASM PTE handlers (RNG, EMAC, MCPWM, WiFi stubs, etc.).
- Removed 8 dead re-exports from `index.js` (`IOPinState`, `SignalDirection`,
  `PinPeripheral`, `Memory`, `MemoryTranslator`, `ReadonlyMemory`, `XtensaCore`,
  `GDBSession`). Only `SimulatorWorker`, `MultiSimulator`, `ESP32` are kept.
- Removed dead functions from `helpers.js` (`prescalerToDivider`, `crc8`,
  `filterOutChips`, `differenceOf`, `readFieldValue`, `clearField`, `uint16ToHex`).
- Removed dead crypto barrel re-exports from `common/index.js` (`sha`, `ecc`,
  `bigint-math`).
- Removed empty `import {}` from `xtensa-constants.js`.
- Net: 8 files changed, −206/+62 lines.

### Code Quality
- ESLint: zero errors, zero warnings across all `src/` files. Fixed real bug in
  uart.js (wrong variable in irdaTxSuppressed/txUpdated getters), removed 83+
  unused imports, added `caughtErrorsIgnorePattern`, suppressed intentional
  spin-wait empty blocks.
- JSDoc type annotations on core modules: `memory.js` (Memory, ReadonlyMemory,
  InvalidMemory, MemoryTranslator, ReverseMemory, MMUMemory, PageTable,
  MMIOHandlerRegistry), `worker-proxy.js` (SimulatorWorker class + all key
  methods), `wasm-loader.js` (property declarations).
- TypeScript `tsconfig.json` configured for `tsc --checkJs` on annotated modules.

### Tests
- `test-worker-ota-multi.mjs`: multi-partition OTA validation (partition labels,
  begin/write/end on next OTA partition).
- `test-worker-twdt.mjs`: Task WDT init/feed/delete + HW timer create/read/delete.
- `test-worker-ble-init.mjs`: BLE controller init + enable (mem_release + enable).
- `bench-trace.mjs`: trace overhead profiler (measures FFI cost of
  `native_trace_set_flags` for trace_return vs trace_mem_write vs both).

## [0.1.3] - 2026-08-28

### Documentation
- Record the measured end-to-end speedup of the batched Xtensa core (`core_run`,
  shipped in 0.1.2). For sustained active (CPU-bound) execution the worker path
  runs **~1.9× faster**: 30 → 58 M instructions/sec at a fixed 30 M virtual-cycle
  target (busy-loop firmware, loop WDT disabled so the core stays active). This is
  on top of the ~1.55× core-level gain measured at the `chip.step()` boundary
  (37 → 57 M instr/sec). Idle/yielding firmware shows little wall-clock change
  because the cost is dominated by idle fast-forward + native peripheral FFI, not
  the per-instruction core loop. No code changes in this release.

## [0.1.2] - 2026-08-27

### Performance
- Batched Xtensa core execution. The new `core_run` WASM export runs up to 512
  interleaved iterations (both cores, 1:1) per FFI call instead of one FFI call
  per instruction. `chip.step()` now dispatches this single call and the worker
  hot path (`runSimChunk`) invokes `chip.step()` ~1024× less often, cutting the
  JS↔WASM boundary and per-step JS overhead. Per-instruction throughput rises
  from ~37 M to ~57 M instructions/sec at the `chip.step()` level; the real
  win is in the worker loop, where stepping overhead scales with the number of
  `chip.step()` calls rather than instructions executed.
- `core_run` applies `sync_ccount` per instruction (matching `core_step`), so
  CCOUNT/CCOMPARE (the FreeRTOS tick) and all per-instruction interrupt handling
  are byte-for-byte identical to the per-instruction path. Verified against the
  full worker suite: **PASS=27 FAIL=0 SKIP=4** (unchanged from the prior
  release), including the dual-core proxy firmware and the heavy peripheral
  suite (`test-real-firmware` → `=== ALL TESTS PASSED ===`).

## [0.1.1] - 2026-08-27

### Fixed
- CLI `esp32emu run <firmware.bin>`: load the firmware image at flash offset
  `0` instead of `0x1000`. The bundled firmware images are *combined*
  (bootloader @ `0x1000`, partition table @ `0x8000`, app @ `0x10000`), so
  placing them at `0x1000` shifted every segment one sector too high and the
  ROM could not find the second-stage bootloader — it then scanned empty flash
  and printed `invalid header: 0xffffffff` in a loop without ever launching the
  app. With offset `0` the firmware now boots and emits its serial output.

## [0.1.0] - 2026-08-22

Initial public package of the ESP32 (Xtensa LX6, dual-core) WASM emulator.

### Engine
- Fully WASM-compiled Xtensa core; no JS interpreter. All peripherals are
  native Rust (UART, GPIO, IO_MUX, SPI, I2C, I2S, LEDC, PCNT, RMT, MCPWM, UHCI,
  SDMMC, SDIO slave, TWAI, TIMG, FRC, DPORT, RNG, AES, SHA, RSA, EFUSE,
  SYSCON, RTC, EMAC, WiFi MAC/AP, BT RF, and stub pages) reached from WASM via
  MMIO PTE overrides. Native clock (values + idle fast-forward) — no
  `SimulationClock` FFI round-trips.
- Bundled boot ROM (`esp32-v3-rom.bin`) and WASM engine
  (`esp_engine_wasm.wasm`) so the package works out of the box.

### API
- `ESP32` (single-threaded, direct core stepping) and `SimulatorWorker`
  (worker-thread, auto-loads the bundled ROM/WASM; stream UART via
  `_onUART`).
- `bin/esp32emu` CLI: `esp32emu run <firmware.bin>` boots firmware and streams
  UART; `--help` for usage.

### Tests / packaging
- `npm test` runs the no-server battery (boot ROM + 1M-step + worker-init +
  package smoke test), all green.
- `tests/test-package-smoke.mjs` boots the bundled ROM via the package entry
  and via `SimulatorWorker` with no explicit rom/wasm (proves the published
  file layout resolves).
- `tests/run-worker-tests.sh`: WiFi-gateway tests (wifi/web/webserver) are
  skipped by default (opt-in with `--wifi`); they get a longer per-test
  timeout because wifi-association time is environmentally variable.
- Release workflow (`.github/workflows/release.yml`) builds the WASM engine
  from Rust, runs the no-server tests, publishes to npm, and creates a GitHub
  release.

### Known limitations
- Full BTDM advertising (beyond controller-init) is unsupported — it requires the
  ESP32 LL/baseband, which is not emulated. `test-worker-bt` covers controller-init only.
- Running real firmware requires compiling it on an external Arduino/ESP-IDF
  compile server (not bundled).
