# Testing

## No-compile-server battery (pure JS)

```
node tests/wasm-bench.mjs              # boots ROM in batches (~20K inst), MMIO counts must be 0
node tests/test-wasm-standalone.mjs    # 1,000,000 WASM steps, expects PASS
node tests/test-wasm-worker-init.mjs   # worker-path init repro (diagnostic)
node tests/diag-ffi-counts.mjs         # FFI traffic audit
```

Canonical firmware test (WASM only):

```
node tests/test-real-firmware.mjs
```

## Worker test family (needs the compile server)

Arduino firmware is compiled on an external server (`tests/compile-server.mjs`,
`http://localhost:5525`, `PORT` env var to change; arduino-cli `esp32:esp32:esp32`
3.3.10). Start it, then run individual tests:

```
node tests/test-worker-gpio.mjs        # ...and the other test-worker-*.mjs
```

or the full battery:

```
./tests/run-worker-tests.sh --skip-bt --skip-wifi
```

WiFi tests (`test-worker-wifi`, `-web`, `-webserver`, `-net-protocols`) also need the WiFi gateway
(`openhw-studio-gateway/openhw-gw`, forwards 8080 → ESP32:80) and are skipped by
default; run with `--wifi`. `test-worker-net-protocols` covers the full L3–L7
battery (DNS/HTTP/NTP/ICMP/MQTT + pcap) — see [networking](./networking.md).

`test-worker-analog.mjs` sets host `analogInputs` (pin → volts) and asserts
`analogRead` returns attenuation-accurate 12-bit counts. `test-worker-deepsleep`
and `test-worker-flash-persist` exercise RTC memory and host flash persistence.
`test-worker-bt.mjs` passes when the sketch overrides `btClassicInUse`/`bleInUse`.
`test-worker-esp32-cam.mjs` selects `board: 'esp32-cam'` and verifies the
module preset, `psramFound()` + SPIRAM heap, and a full camera frame
(compiled with `PSRAM=enabled`, like the real module default).

To run any stock protocol test against the ESP32-CAM preset instead of the
default board (proves module protocol parity without editing tests):

```
node --import ./tests/board-hook.mjs tests/test-worker-gpio.mjs
```

## MultiSimulator

`tests/README-MultiSim.md` documents the SAB/worker flow. MultiSimulator runs N
parallel WASM nodes (the JS engine and lockstep dual-engine mode are gone).

## Status

Full worker suite: `PASS=31 FAIL=0` (incl. WiFi) when the compile server and
gateway are up; `PASS=28 FAIL=0 SKIP=3` with WiFi skipped.
