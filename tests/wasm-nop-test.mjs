import { ESP32 } from '../src/index.js';
import { readFileSync } from 'fs';
import { fileURLToPath } from 'url';
import { dirname, join } from 'path';

const __dirname = dirname(fileURLToPath(import.meta.url));
const BOOTROM_PATH = join(__dirname, '..', 'rom', 'esp32-v3-rom.bin');
const WASM_PATH = join(__dirname, '..', 'src', 'engine', 'esp-xtensa', 'esp_engine_wasm.wasm');

const rom = readFileSync(BOOTROM_PATH);
const wasmBytes = readFileSync(WASM_PATH);
const flash = new Uint8Array(4 * 1024 * 1024);
flash.fill(0xff);

const esp32 = new ESP32({ flashSizeMB: 4, flash });
esp32.loadROM(rom);
esp32.reset();
esp32.flash.set(flash);
try { esp32.cores[0].writeUint32(0x3ff5a104, 0x5aa5); } catch {}

const loaded = await esp32.loadWasm(wasmBytes, 'wasm');
if (!loaded) { console.log('FAILED'); process.exit(1); }

const chip = esp32;
console.log(`Initial PC=0x${chip.cores[0].PC.toString(16)} idle=${chip.cores[0].idle}`);

// Single step to watch
for (let i = 0; i < 10; i++) {
  chip.step();
  const c0 = chip.cycles;
  console.log(`step ${i}: cycles=${chip.cycles} PC=0x${chip.cores[0].PC.toString(16)} idle=${chip.cores[0].idle} enabled=${chip.cores[0].enabled}`);
}

// Now run 100 single steps and see PC progression
const t0 = Date.now();
let lastPC = chip.cores[0].PC;
for (let i = 0; i < 10000; i++) {
  chip.cores[0].runInstruction();
  const pc = chip.cores[0].PC;
  if (pc !== lastPC && i < 50) {
    // too noisy
  }
  lastPC = pc;
}
console.log(`10000 JS single steps in ${Date.now()-t0}ms, final PC=0x${chip.cores[0].PC.toString(16)}`);
