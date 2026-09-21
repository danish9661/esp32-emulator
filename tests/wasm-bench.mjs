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
flash[0x1000] = 0xe9; flash[0x1001] = 1; flash[0x1002] = 0x20; flash[0x1003] = 0x40;
flash[0x1004] = 0x00; flash[0x1005] = 0x00; flash[0x1006] = 0x00; flash[0x1007] = 0x40;
flash[0x100c] = 0x00; flash[0x100d] = 0x00; flash[0x100e] = 0x00; flash[0x100f] = 0x00;
flash[0x1014] = 0x00; flash[0x1015] = 0x00; flash[0x1016] = 0x00; flash[0x1017] = 0x40;
flash[0x1018] = 0x04; flash[0x1019] = 0x00; flash[0x101a] = 0x00; flash[0x101b] = 0x00;
flash[0x101c] = 0x00; flash[0x101d] = 0x00; flash[0x101e] = 0x00; flash[0x101f] = 0x00;
flash[0x1020] = 0xef;

const esp32 = new ESP32({ flashSizeMB: 4, flash });
esp32.loadROM(rom);
if (esp32.gpio?.pins) {
  if (esp32.gpio.pins[0])  esp32.gpio.pins[0].inputValue = true;
  if (esp32.gpio.pins[2])  esp32.gpio.pins[2].inputValue = false;
  if (esp32.gpio.pins[12]) esp32.gpio.pins[12].inputValue = false;
  if (esp32.gpio.pins[15]) esp32.gpio.pins[15].inputValue = false;
}
esp32.reset();
esp32.flash.set(flash);
if (esp32.gpio) esp32.gpio.strapValue = 0x13;
if (esp32.mmuTablePro) {
  for (let p = 0; p < 64; p++) { esp32.mmuTablePro[p] = p; esp32.mmuTableApp[p] = p; }
}
try { esp32.cores[0].writeUint32(0x3ff5a104, 0x5aa5); } catch {}
if (esp32.uart?.[0]) {
  const u = esp32.uart[0];
  const orig = u.txUpdated?.bind?.(u);
  if (orig) { u.txUpdated = function() { orig(); if (this.txState !== 0) this.txComplete(); }; }
}

// Load WASM in 'wasm' mode (batch)
const loaded = await esp32.loadWasm(wasmBytes, 'wasm');
if (!loaded) { console.log('FAILED to load WASM'); process.exit(1); }

const chip = esp32;
let PC = chip.cores[0].PC;
console.log(`Initial PC=0x${PC.toString(16)}`);

// Measure throughput
const BATCHES = 5;
let totalInsts = 0;
for (let b = 0; b < BATCHES; b++) {
  const t0 = process.hrtime.bigint();
  chip.step();
  const t1 = process.hrtime.bigint();
  const us = Number(t1 - t0) / 1000;
  totalInsts = chip.cycles;
  console.log(`batch ${b}: ${us.toFixed(0)}us = ${(us/4096).toFixed(2)}us/inst`);
}

PC = chip.cores[0].PC;
console.log(`Final PC=0x${PC.toString(16)}`);

  // Dump MMIO counts
  if (chip._wasmLoader?._mmioCount) {
    const counts = chip._wasmLoader._mmioCount;
    const handlers = chip._wasmLoader?.mmioRegistry?.handlers;
    console.log('\nMMIO counts per handler:');
    let totalMmio = 0;
    for (let i = 0; i < counts.length; i++) {
      if (counts[i] > 0) {
        const name = handlers?.[i]?.constructor?.name || 'unknown';
        const native = ((chip._wasmLoader?.mmioRegistry?.handlerType?.[i] ?? 0) & 0x80000000) ? ' [NATIVE]' : '';
        const label = i === 40 ? 'map_read' : i === 41 ? 'map_write' : `handler ${i} (${name})`;
        console.log(`  ${label}: ${counts[i]}${native}`);
        totalMmio += counts[i];
      }
    }
    console.log(`Total MMIO calls: ${totalMmio}`);
  }

