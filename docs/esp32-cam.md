# ESP32-CAM (AI-Thinker) board support

Select the module with the `board` config key (default `'esp32'`):

```js
await sim.init('ESP32', {
  board: 'esp32-cam', mmuPages: 64, strapValue: 0x13,
  camFrameBytes: 160 * 120 * 2,   // QQVGA RGB565 sensor frame
}, flashData);
```

`board: 'esp32-cam'` applies the AI-Thinker module preset — 4MB flash +
4MB quad PSRAM (explicit `flashSizeMB`/`psramSizeMB` always win; unknown
board ids throw). The board id is visible as `chip.board`, `proxy.board`
and `chipInfo.board`. The module pinout is exported as `ESP32_CAM_PINS`
(camera XI pins, on-board LED on GPIO 33, flash LED on GPIO 4).

## Protocol support

The module is the same ESP32 silicon, so **every protocol the emulator
supports on ESP32 works on ESP32-CAM unchanged** — verified by running the
stock protocol tests against the board preset (via
`tests/board-hook.mjs`, which forces `board: 'esp32-cam'` into
`SimulatorWorker.init`):

```
node --import ./tests/board-hook.mjs tests/test-worker-gpio.mjs
```

| Protocol / block | Status on ESP32-CAM |
|---|---|
| GPIO, UART, SPI (+slave), I2C (+slave, RTC_I2C), I2S (+RX), timers, LEDC/PWM, MCPWM, RMT, PCNT, TWAI (+dual-node), ADC cont., DAC, touch, TSENS, BOD, SD/MMC/eMMC, SDIO slave, EMAC loopback, ULP (+wake), deep sleep, NVS/flash persist, OTA, WiFi scan/web | Same as ESP32 — PASS (cross-section gpio/uart/spi/i2c/timer/pwm/analog re-run on-board, all PASS; full suite PASS=53) |
| Camera (OV2640, DVP via I2S) | PASS — `test-worker-esp32-cam`: SCCB probe + init over the module pinout, full QQVGA RGB565 frame byte-exact (`CAM_FB=38400 CAM_DATA=PASS`) |
| PSRAM | PASS — `psramFound()=1`, 4KB SPIRAM heap write/read-back (`PSRAM_HEAP=PASS`); SPI1 JEDEC ID reports the configured chip |
| BT | Controller-init only (full BTDM advertising unsupported — same as ESP32) |

## Firmware build requirements

Like the real module default, camera/PSRAM sketches must be compiled with
PSRAM enabled — the generic `esp32:esp32:esp32` build disables it and
`psramInit()` never runs (`psramFound()` stays false). Either:

- `esp32:esp32:esp32:PSRAM=enabled` (same flash layout as other tests), or
- `esp32:esp32:esp32cam` (the real AI-Thinker board definition)

The compile server (`tests/compile-server.mjs`) honors a validated
client fqbn and caches by fqbn + sketch hash.

## Driving the camera

The virtual OV2640 serves SCCB + a ramp frame; the frame still needs VSYNC
edges from the host while `esp_camera_fb_get()` waits (the driver
configures VSYNC NEGEDGE — pulse GPIO 25 high→low every ~150ms, see
`test-worker-esp32-cam.mjs`). Frame size is host-known
(`config.camFrameBytes`, sensor bytes, e.g. 38400 for QQVGA RGB565).
