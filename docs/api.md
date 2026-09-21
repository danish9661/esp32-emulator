# API reference

`esp32emu` exposes 15 named exports (from `src/index.js`):

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

## `SimulatorWorker`

```js
import { SimulatorWorker } from 'esp32emu';

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
import { ESP32 } from 'esp32emu';
const chip = new ESP32({ flash });
chip.loadROM(rom);
await chip.loadWasm(wasm, 'wasm');
chip.reset();
chip.step();   // advance the core
```

Use when you do not want a worker (tight loop, or GDB-style inspection without IPC).

## `GDBSession`

`new GDBSession(transport)`, where `transport` is any object with `send(data)` /
`onData(cb)`. **No network server is started for you** — it uses an in-process
transport by default. It can read/write emulated memory and registers through the
`SimulatorWorker` / `ESP32` memory views.
