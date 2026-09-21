# ESP32 WASM Emulator

[![npm version](https://img.shields.io/npm/v/esp32emu)](https://www.npmjs.com/package/esp32emu)
[![license](https://img.shields.io/npm/l/esp32emu)](LICENSE)
[![CI](https://github.com/danish9661/esp32-emu/actions/workflows/ci.yml/badge.svg)](https://github.com/danish9661/esp32-emu/actions)

A fast ESP32 (Xtensa LX6, dual-core) emulator. The CPU is executed by a
Rust-compiled WASM engine; peripherals run natively in Rust (inside the same
WASM module) with JavaScript hosts for the remaining subsystems (WiFi analog,
clock source, loader FFI glue).

## Architecture

```
┌──────────────────────────────────────────────────────────────────┐
│                        JavaScript Host                           │
│                                                                  │
│  ┌──────────────┐   ┌───────────────┐   ┌───────────────────┐   │
│  │  Worker SAB  │   │ Clock Driver  │   │  WiFi Gateway     │   │
│  │  UART ring   │◄──│ (Simulation   │◄──│  WebSocket+pcap   │   │
│  │  debug state │   │  Clock)       │   │                   │   │
│  └──────┬───────┘   └───────┬───────┘   └────────┬──────────┘   │
│         │                   │                     │              │
│  ┌──────▼───────────────────▼─────────────────────▼──────────┐   │
│  │              WASM Boundary (FFI glue)                     │   │
│  └──────────────────────────┬────────────────────────────────┘   │
└─────────────────────────────┼────────────────────────────────────┘
                              │ JS ↔ WASM calls
┌─────────────────────────────▼────────────────────────────────────┐
│                    WASM Module (Rust)                             │
│                                                                  │
│  ┌───────────────────────────────────────────────────────────┐   │
│  │              Xtensa LX6 Core Engine (batched)             │   │
│  │         core_run: 512 interleaved iterations/call         │   │
│  │                    × 2 cores                              │   │
│  └───────────────────────┬───────────────────────────────────┘   │
│                          │ MMIO traps                            │
│  ┌───────────────────────▼───────────────────────────────────┐   │
│  │              Native Peripheral Dispatch                    │   │
│  │                                                           │   │
│  │  UART  GPIO  SPI  I2C  Timer  Flash  RNG  SHA  AES      │   │
│  │  LEDC  PCNT  RMT  I2S  SDMMC  WiFi  BT RF  EMAC  ...   │   │
│  │                                                           │   │
│  │  ┌─────────────┐  ┌──────────────┐  ┌─────────────────┐  │   │
│  │  │ Page Table  │  │ Flash Mirror │  │ Event Queue     │  │   │
│  │  │ (PTE-based  │  │ (4MB linear  │  │ (clock events,  │  │   │
│  │  │  routing)   │  │  in memory)  │  │  timer alarms)  │  │   │
│  │  └─────────────┘  └──────────────┘  └─────────────────┘  │   │
│  └───────────────────────────────────────────────────────────┘   │
│                                                                  │
│  ┌───────────────────────────────────────────────────────────┐   │
│  │           Shared Memory (SAB)                              │   │
│  │  Flash (4MB) │ ROM (64KB) │ RTC mem │ RAM data             │   │
│  └───────────────────────────────────────────────────────────┘   │
└──────────────────────────────────────────────────────────────────┘
```

### Boot flow

1. **ROM loads** — real ESP32 boot ROM executes, sets up MMU/flash cache
2. **Firmware loads** — bootloader finds your compiled Arduino/ESP-IDF firmware
3. **Core runs** — `core_run` executes both Xtensa cores (512 iterations/call)
4. **MMIO traps** — register access dispatched to native Rust peripheral handlers
5. **UART streams** — output flows through SharedArrayBuffer ring buffer to host

### Peripheral coverage

All major ESP32 peripherals are native Rust (no JS fallback traffic):

| Peripheral | Status | Notes |
|------------|--------|-------|
| UART (0/1/2) | Native | TX/RX FIFO, baud autodetect, IRDA |
| GPIO + IO_MUX | Native | Matrix routing, 64 pins, strap values |
| SPI (1/2/3) | Native | Full flash-cache fast-path (4MB mirror) |
| I2C (0/1) | Native | Master/slave, clock stretching |
| Timers (TIMG0/1, FRC1) | Native | Watchdog, alarm, clock events |
| Flash + MMU | Native | Read/write, page table, DROM windows |
| RNG | Native | True RNG register emulation |
| SHA + AES | Native | Hardware accelerator |
| LEDC + PCNT | Native | PWM capture, pulse counting |
| RMT | Native | Remote control transceiver |
| Touch + DAC | Native | `touchInputs` counts, `dacWrite`→ADC loopback |
| I2S (0/1) | Native | DMA descriptor chain, TX/RX processing |
| SDMMC | Native | SD host + virtual card (mount, FAT R/W, `sdCard` image) |
| WiFi (analog + MAC) | Native | Full TX/RX bridge, AP mode |
| EMAC | Native | MDIO PHY (LAN8720 ID/link) + DMA TX→RX loopback |
| BT RF | Native | Controller init (no LL/baseband) |
| DPORT + IRQ matrix | Native | Cross-core, interrupt routing |
| MCPWM, UHCI, SDIO slave | Native | Peripheral register files |

## Quick start

### Install

```bash
npm install esp32emu
```

Node 18+ required. No native dependencies — the WASM engine is bundled.

### Run your first firmware

```js
import { SimulatorWorker } from 'esp32emu';

// 4 MB flash, erase-filled. Write your compiled .bin here:
const flash = new Uint8Array(4 * 1024 * 1024).fill(0xff);

const sim = new SimulatorWorker();
let uart = '';
sim._onUART = (b) => { uart += String.fromCharCode(b); };

await sim.init('ESP32', {
  flashSizeMB: 4,
  mmuPages: 64,
  strapValue: 0x13,
  budget: 2_000_000
}, flash);
sim.run();

// Wait for firmware to boot and produce output
for (let i = 0; i < 25; i++) {
  await new Promise(r => setTimeout(r, 100));
  sim.pollUart();
}
console.log(uart);
sim.terminate();
```

### Compile an Arduino sketch

The package includes a compile server that compiles Arduino sketches to ESP32
firmware binaries:

```bash
# Start the compile server (needs arduino-cli with esp32:esp32 core)
# Default port is 5525; override with PORT=XXXX
node tests/compile-server.mjs &
# or: PORT=5000 node tests/compile-server.mjs &

# Compile your sketch
curl -X POST http://localhost:5525/api/compile/start \
  -F "sketch=@MySketch.ino" \
  -F "fqbn=esp32:esp32:esp32"
# If you changed PORT, use that port instead
```

### Single-threaded mode

For lower-level control without a worker thread:

```js
import { ESP32 } from 'esp32emu';

const chip = new ESP32({ flashSizeMB: 4, strapValue: 0x13 });
chip.loadROM(romBytes);
await chip.loadWasm(wasmBytes, 'wasm');
chip.reset();

// Step one instruction at a time
chip.step(1000); // advance 1000 instructions
console.log('PC:', chip.cores[0].pc.toString(16));
```

### MicroPython

Boot the official MicroPython ESP32 firmware and interact via the REPL:

```bash
# Download once (1.8 MB, ESP32_GENERIC v1.29.0)
curl -L -o /tmp/micropython-esp32.bin \
  https://micropython.org/resources/firmware/ESP32_GENERIC-20260824-v1.29.0.bin
```

```js
import { SimulatorWorker } from 'esp32emu';
import { readFileSync } from 'fs';

const mpyBin = readFileSync('/tmp/micropython-esp32.bin');
const flash = new Uint8Array(new SharedArrayBuffer(4 * 1024 * 1024));
flash.fill(0xff);
flash.set(mpyBin, 0x1000); // as `esptool write_flash 0x1000`

const sim = new SimulatorWorker();
let uart = '';
sim._onUART = (b) => { uart += String.fromCharCode(b); };
await sim.init('ESP32', { flashSizeMB: 4, mmuPages: 30, strapValue: 0x13 }, flash);
sim.run();

// Wait for REPL prompt
for (let i = 0; i < 30; i++) {
  await new Promise(r => setTimeout(r, 250));
  sim.pollUart();
  if (uart.includes('>>>')) break;
}
console.log(uart); // MicroPython v1.29.0 ... >>>

// Send Python code to the REPL
sim.sendUart('print(1+2)\r\n');
await new Promise(r => setTimeout(r, 500));
sim.pollUart();
console.log(uart); // ... 3 ...

// Peripherals via machine module
sim.sendUart('from machine import Pin; p=Pin(2, Pin.OUT); p.value(1); print(p.value())\r\n');
sim.sendUart('from machine import I2C, Pin; i=I2C(0, scl=Pin(22), sda=Pin(21)); print(i.scan())\r\n');
sim.sendUart('from machine import ADC; print(ADC(Pin(36)).read())\r\n');
```

All `machine` peripherals (GPIO, ADC, PWM, I2C, SPI, Timer, UART) use the same
native MMIO registers as the Arduino tests (`PASS=32`), so they work out of the box.

## API reference

### Main exports

| Export | Description |
|--------|-------------|
| `SimulatorWorker` | Worker-thread simulator (recommended entry point) |
| `MultiSimulator` | N parallel chip nodes with UART splitting |
| `ESP32` | Single-threaded chip (no worker) |
| `XtensaCore` | Facade exposing WASM core state to JS |
| `Memory` / `MemoryTranslator` / `ReadonlyMemory` | Memory model primitives |
| `GDBSession` | GDB remote-protocol session |
| `IOPinState` / `SignalDirection` / `PinPeripheral` | GPIO enums |

### `SimulatorWorker`

```js
const sim = new SimulatorWorker();

// Initialize with config and flash data
await sim.init('ESP32', {
  board: 'esp32',       // board preset: 'esp32' (default) or 'esp32-cam'
  flashSizeMB: 4,        // flash size in MB
  psramSizeMB: 0,        // PSRAM size (0 = none)
  mmuPages: 64,          // MMU page count
  strapValue: 0x13,      // boot strap (GPIO strapping)
  budget: 2_000_000,     // instructions per step budget
  macAddress: 'AA:BB:CC:DD:EE:FF',
  analogInputs: { 36: 3.3 },  // analog pin voltages
}, flashData);

sim.run();                // start execution in worker
sim.stop();               // pause
sim.pollUart();           // flush UART buffer
sim.terminate();          // kill worker

// Callbacks
sim._onUART = (byte) => {};    // UART byte received
sim._onError = (err) => {};    // worker error
```

### Boards (`board` config)

`board: 'esp32'` (default) is bare silicon. `board: 'esp32-cam'` selects the
AI-Thinker ESP32-CAM module preset (4MB flash + 4MB PSRAM — explicit sizes
always win) and enables firmware PSRAM detection (`psramFound()`, SPIRAM
heap) plus the virtual OV2640 over the module pinout (`ESP32_CAM_PINS`):

```js
import { SimulatorWorker, ESP32_CAM_PINS } from 'esp32emu';

const sim = new SimulatorWorker();
await sim.init('ESP32', {
  board: 'esp32-cam', mmuPages: 64, strapValue: 0x13,
  camFrameBytes: 160 * 120 * 2,   // QQVGA RGB565 sensor frame
}, flashData);
// ESP32_CAM_PINS.pinVsync / .pinSiod / ... — AI-Thinker camera + LED pins
```

See [ESP32-CAM board](./docs/esp32-cam.md) for the module preset, the full
protocol-support matrix, and camera/PSRAM firmware notes.

### `ESP32` (single-threaded)

```js
const chip = new ESP32({
  flashSizeMB: 4,
  strapValue: 0x13,
  partitions: [           // optional partition table
    { name: 'nvs',    type: 'data', subtype: 'nvs',    offset: 0x9000,  size: 0x6000 },
    { name: 'app',    type: 'app',  subtype: 'factory', offset: 0x10000, size: 0x300000 },
  ]
});

chip.loadROM(romBytes);
await chip.loadWasm(wasmBytes, 'wasm');
chip.reset();

chip.step(n);            // run n instructions
chip.halt();             // stop execution
chip.readMemory(addr, len);  // read address space
chip.writeMemory(addr, data); // write address space

// Interrupt delivery
chip.interrupt(irqNumber, level);
```

## Performance

The Xtensa core runs as a Rust-compiled WASM engine. Execution is batched: a
single JS↔WASM call (`core_run`) runs up to 512 interleaved core iterations
(both cores) instead of one call per instruction.

| Metric | Per-instruction | Batched (v0.1.2+) |
|--------|----------------|-------------------|
| Core throughput | ~37 M instr/sec | ~57 M instr/sec |
| Worker throughput | ~30 M instr/sec | ~58 M instr/sec |
| Speedup | — | **~1.9×** |

Idle/yielding firmware is dominated by idle fast-forward + native peripheral
FFI and sees little wall-clock change. All 32 worker tests pass with zero
JS fallback traffic (0 `map_read`/`map_write` FFI calls).

## Limitations

- **ESP32 only** — ESP32-C3/RV32 support was removed.
- **WASM is the only engine** — the JS CPU interpreter was removed.
- **BLE full-stack advertising unsupported** — requires the ESP32 LL/baseband
  which is not emulated; `test-worker-bt` covers controller-init only.

## Testing

```bash
# Quick sanity (no compile server needed)
npm test

# Full worker suite (needs compile server on :5525)
node tests/compile-server.mjs &   # PORT=5525 by default; PORT=XXXX to override
./tests/run-worker-tests.sh --skip-bt --skip-wifi

# Lint
npm run lint
```

See [docs/testing.md](docs/testing.md) for the full test battery.

## Documentation

- [Architecture](docs/architecture.md) — engine, native peripherals, SAB IPC
- [API reference](docs/api.md) — all 11 public exports and key methods
- [Browser usage](docs/browser.md) — bundling + cross-origin isolation
- [Building](docs/building.md) — Rust→WASM engine and `dist/` bundle
- [Security audit](docs/audit.md) — memory, overhead, security

## Contributing

1. Fork the repo
2. Create a feature branch
3. Make changes + run `npm run lint` and `npm test`
4. Open a PR

## License

MIT (see [LICENSE](LICENSE)). The bundled ESP32 boot ROM is Espressif
proprietary code redistributed under Apache License 2.0 — see
[THIRD-PARTY-NOTICES.md](THIRD-PARTY-NOTICES.md).
