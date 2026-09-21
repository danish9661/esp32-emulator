import { ESP32 } from '../src/index.js';
import { readFileSync } from 'fs';

const BOOTROM_PATH = 'rom/esp32-v3-rom.bin';
const WASM_PATH = 'src/engine/esp-xtensa/esp_engine_wasm.wasm';
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
  if (esp32.gpio.pins[0]) esp32.gpio.pins[0].inputValue = true;
  if (esp32.gpio.pins[2]) esp32.gpio.pins[2].inputValue = false;
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

const loaded = await esp32.loadWasm(wasmBytes, 'wasm');
if (!loaded) { console.log('FATAL: WASM did not load'); process.exit(1); }
console.log('WASM engine loaded (WASM-only mode).');

const TOTAL = 1_000_000;
const CHECK_INTERVAL = 100_000;
console.log(`Running WASM-only engine for ${TOTAL} steps...`);
const start = Date.now();

for (let s = 1; s <= TOTAL; s++) {
  esp32.step();
  if (s % CHECK_INTERVAL === 0) {
    const pc = (esp32._wasmCores[0].PC >>> 0).toString(16);
    const elapsed = ((Date.now() - start) / 1000).toFixed(0);
    console.log(`  step ${s.toLocaleString()}: PC=0x${pc} (${elapsed}s)`);
  }
}

console.log(`\n*** PASSED ${TOTAL.toLocaleString()} WASM-only steps in ${((Date.now() - start) / 1000).toFixed(0)}s ***`);
