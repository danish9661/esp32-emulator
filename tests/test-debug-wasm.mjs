import { MultiSimulator } from '../src/sab/MultiSimulator.js';
import { readFileSync } from 'fs';
import { resolve, dirname } from 'path';
import { fileURLToPath } from 'url';

const __dirname = dirname(fileURLToPath(import.meta.url));

// Load pre-compiled firmware binary
const firmwarePath = resolve(__dirname, '../firmware.bin');
let firmwareBytes;
try {
  firmwareBytes = readFileSync(firmwarePath);
} catch {
  console.log("No firmware.bin found, using minimal flash");
  firmwareBytes = new Uint8Array(4096);
}
console.log(`Firmware: ${firmwareBytes.length} bytes`);

const flashSizeMB = Math.max(4, Math.ceil(firmwareBytes.length / (1024 * 1024)));
const flashSab = new SharedArrayBuffer(Math.max(flashSizeMB, 4) * 1024 * 1024);
const flash = new Uint8Array(flashSab);
flash.fill(0xff);
flash.set(firmwareBytes);

const romBytes = readFileSync(resolve(__dirname, '../rom/esp32-v3-rom.bin'));
const romSab = new SharedArrayBuffer(romBytes.length);
const rom = new Uint8Array(romSab);
rom.set(romBytes);
console.log(`BootROM: ${rom.length} bytes`);

const sim = new MultiSimulator();
await sim.addNodeParallel('ESP32', { engine: 'wasm', strapValue: 0x13, pinInputs: { 0: true, 2: false, 12: false, 15: false }, mmuPages: Math.ceil(firmwareBytes.length / 65536) }, flash, rom);

console.log('WASM engine initialized at node 0');
console.log('Running for 5 seconds...');

const startTime = Date.now();
await sim.runParallel({
  onPoll: (s) => {
    const ds = sim.debugState(0);
    if (ds) {
      // debugData layout:
      // [0] pc0          = 0xBEEF if blockingCommandLoop started
      // [1] cmdVal       = value read from SAB_CMD before exchange
      // [2] runVal       = value read from SAB_RUN
      // [3] cmd          = value returned by exchange
      // [4] = 0xCAFE if CMD_RUN branch is taken
      // [5] = 0x5CAFE if runSimChunk entered, 0xBAD if early-exit
      // [6] = chip? 1:0
      // [7] = Atomics.load(ctrl, SAB_RUN)
      console.log(`[DEBUG] ds[0..7]=${ds.pc0.toString(16)} ${ds.physicalRegisters[1].toString(16)} ${ds.physicalRegisters[2]} ${ds.physicalRegisters[3].toString(16)} ${ds.physicalRegisters[4].toString(16)} ${ds.physicalRegisters[5].toString(16)} ${ds.physicalRegisters[6]} ${ds.physicalRegisters[7]}`);
    }
    const uart = s.getUART(0);
    if (uart && uart.length > 0) {
      console.log(`[UART] len=${uart.length} tail=${uart.slice(-40)}`);
    }
    if (Date.now() - startTime > 5000) return true;
    return false;
  },
  pollInterval: 1000,
});

const ds = sim.debugState(0);
console.log(`\nFinal debug state:`);
console.log(`  pc0=0x${ds.pc0.toString(16)}`);
console.log(`  cycles=${ds.cycles}`);
console.log(`  phys[0..7]=${ds.physicalRegisters.slice(0,8).map(v => '0x' + v.toString(16)).join(' ')}`);
console.log(`  enabled=${ds.enabled0}`);

const uart = sim.getUART(0);
console.log(`\nUART (${uart.length} bytes):`);
console.log(uart.slice(0, 200) || '(empty)');

process.exit(0);
