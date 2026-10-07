# API reference

`esp32-emu` exposes 21 named exports (from `src/index.js`):

| Export | Kind | Purpose |
|---|---|---|
| `SimulatorWorker` | class | Runs the ESP32 in a **worker thread** (auto-loads bundled ROM/WASM). Main entry point. |
| `MultiSimulator` | class | Runs **N parallel WASM nodes**; splits UART ring, compares debug state. |
| `ESP32` | class | Single-threaded chip (no worker): `loadROM`, `loadWasm`, `reset`, `step`. |
| `XtensaCore` | class | Facade exposing WASM core state (PC, regs, ccompare) to JS. Does **not** execute. |
| `Memory` | class | Memory-model primitive (emulated address space). |
| `MemoryTranslator` | class | MMU window translation primitive. |
| `ReadonlyMemory` | class | Read-only memory view (e.g. boot ROM). |
| `GDBSession` | class | GDB remote-protocol session over an in-process transport. |
| `IOPinState` | enum | GPIO pin state constants (`PinState`). |
| `SignalDirection` | enum | Signal direction constants. |
| `PinPeripheral` | enum | Pin peripheral-type constants (`PeripheralType`). |
| `ESP32_CAM_BOARD_ID` | const | `'esp32-cam'` board id for the `board` config key. |
| `ESP32_CAM_PINS` | const | AI-Thinker ESP32-CAM pinout (camera XI pins, LED + flash LED). |
| `ESP32_CAM_DEFAULTS` | const | ESP32-CAM memory defaults (4MB flash + 4MB PSRAM). |
| `applyBoardPreset` | fn | Merge a board preset into a chip config (explicit keys win). |
| `I2cTap` | class | Host I2C slave model (`chip.i2c0`/`i2c1`): attach/preload/drain. |
| `SpiTap` | class | Host SPI transfer model (`chip.spi2`/`spi3`): onTransfer/injectMiso/drainMosi. |
| `OneWireDevice` | class | DS18B20-compatible slave (`chip.attachOneWire(pin)`). |
| `RmtTap` | class | RMT single-wire tap (`chip.rmt`): TX capture + RX inject + tick rate. |
| `Ov2640` | class | OV2640 SCCB/DVP sensor model (`chip.attachOv2640`). |
| `dallasCrc8` | fn | Dallas CRC8 (OneWire ROM/scratchpad). |

## `SimulatorWorker`

```js
import { SimulatorWorker } from 'esp32-emu';

const flash = new Uint8Array(4 * 1024 * 1024).fill(0xff);
const sim = new SimulatorWorker();
sim._onUART  = (b) => process.stdout.write(String.fromCharCode(b));
sim._onError = (e) => console.error(e);
sim._onReady = () => {};

await sim.init('ESP32', { flashSizeMB: 4, mmuPages: 64, strapValue: 0x13, budget: 2_000_000 }, flash);
sim.run();
sim.pollUart();                        // drain UART ring (call on a timer)
await sim.writeUint32(0x3FF44004, 1);  // poke a register (zero-copy, SAB-backed)
const rom = sim.readMemory(0x40000000, 64);
await sim.seedMMU();
await sim.reset();
await sim.step(1000);                  // advance 1000 instructions
const pcap  = sim.getPcapData();       // WiFi pcap bytes
const wifi  = sim.getWifiStats();      // { state, txFrames, txBytes, rxFrames, ... }
sim.terminate();                        // release worker + its memory
```

**Methods:** `init`, `run`, `stop`, `terminate`, `seedMMU`, `reset`,
`writeUint32`, `readMemory`, `readMmio`, `step`, `getPcapData`, `getFfiCounts`,
`getWifiStats`, `pollUart`.

**Getters:** `running`, `pc`, `pc1`, `nanos`, `stuck`, `idle`, `ready`,
`macAddress`, `board`, `cpuFrequency`, `psramType`, `flashSizeMB`, `psramSizeMB`,
`wifiState`, `wifiTxFrames`, `wifiRxFrames`, `wifiTxBytes`, `wifiRxBytes`,
`wifiProbes`, `budget`, and `memory.*` (direct `flash`/`iram`/`dataMem`/`rtcFastMem`/… views).

## `ESP32` (single-threaded, no worker)

```js
import { ESP32 } from 'esp32-emu';
const chip = new ESP32({ flash });
chip.loadROM(rom);
await chip.loadWasm(wasm, 'wasm');
chip.reset();
chip.step();   // advance the core
```

Use when you do not want a worker (tight loop, or GDB-style inspection without IPC).

## Host peripheral taps

Sensor/peer models that run on the host and talk to guest firmware through
the real controllers — no firmware stubs. Contract (all taps): wiring
(registrations, listeners, hook flags) survives `chip.reset()`; in-flight
bytes/events clear; with nothing attached the engine never calls into JS
(zero behavior change). FFI handlers are synchronous pure-JS (no WASM
re-entry). In the worker build the models live in the worker thread and the
`SimulatorWorker` bridge pre-stages reads and drains writes/events.

### I2C slaves — `chip.i2c0` / `chip.i2c1` (`I2cTap`)

`attachSlave(addr, {onWrite, onRead, onStop, onStart})`, `detachSlave`,
`preloadRead(addr, bytes)`, `drainWriteLog(addr)`, `pollEvents()`; bus-level
`onStart/onWrite/onRead/onStop` (promiscuous claim). Worker:
`attachI2cSlave/detachI2cSlave/pushI2cTx/popI2cRx/pollI2c`.

### SPI transfers — `chip.spi[0..3]` (`SpiTap`; user buses 2/HSPI, 3/VSPI)

`onTransfer(tx, recvLen) → rx` or split `injectMiso` + `drainMosi`/`pollTx`.
Worker: `injectSpiMiso/pollSpiTx`.

### GPIO / ADC / LEDC readback

`getGpioOut/getGpioDir/getGpioPull`, `sampleGpioOut`, `onGpioOutChange` +
`pollGpioChanges`; `setAnalogInput`/`getAnalogInput`;
`getLedcDuty/getLedcFreq/getLedcTimer/getLedcPin` (+ `ledcChannelCount`).
Worker: `getGpioOut/sampleGpioOut/pollGpioChanges`, `setAnalogInput`,
`getLedc/getLedcPin`.

### I2S TX + OneWire

`onI2sTx`/`pollI2sTx` (worker `armI2sTx`/`pollI2sTx`); `feedI2sRxSample` feeds
one RX sample word (worker `feedI2SRX`). `attachOneWire(pin[, device])`/
`detachOneWire` (`OneWireDevice`: `setTemperature`, `preloadScratch`,
`pollLog`; worker `attachOneWire/setOneWireTemp/preloadOneWireScratch/
pollOneWire`).

### RMT single-wire sensors — `chip.rmt` (`RmtTap`)

`onRmtTx(ch, cb)` + `pollRmtTx(ch?)` → `[{ch, items, tickHz}]` (channel-RAM
item words + zero terminator); `injectRmtRx(ch, words)` (immediate delivery
when RX-armed, else pending for the next RX_EN arm — sensor-answers-start
parity); `getRmtTickHz(ch)` for host µs↔tick conversion;
`RmtTap.item/splitItem/usToTicks` word helpers. Worker:
`armRmtTx/pollRmtTx/injectRmtRx/getRmtTickHz`.

### Camera — OV2640 + frame tap + feed

`attachOv2640(bus?, addr?, model?)`/`detachOv2640` (`Ov2640`: SCCB slave
0x30, PID 0x26/0x42, reg-pointer protocol, `frameBytes`/`expectedFrame` =
the engine ramp, `drainSccbLog`); `onCameraFrame`/`pollCameraFrame`
(guest→host sensor bytes consumed by RX DMA);
`setCameraFrameBytes`/`getCameraFrameBytes` (virtual-sensor frame size);
`feedCameraFrame`/`cameraFeedLength` (host→sensor scripted scenes: one full
staged frame replaces the ramp for exactly one capture). Worker:
`attachCameraSccb/armCamera/pollCameraFrame/setCameraFrameBytes/
getCameraFrameBytes/feedCameraFrame/cameraFeedLength`.

Covered by `tests/test-taps-host.mjs` (302 checks, no compile server);
firmware cells `test-worker-rmt-dht`, `test-worker-camera`,
`test-worker-camera-fed` (needs the compile server — see
[testing](./testing.md)).

## `GDBSession`

`new GDBSession(transport)`, where `transport` is any object with `send(data)` /
`onData(cb)`. **No network server is started for you** — it uses an in-process
transport by default. It can read/write emulated memory and registers through the
`SimulatorWorker` / `ESP32` memory views.
