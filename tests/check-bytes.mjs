import { ESP32 } from '../src/peripherals/esp32/esp32.js';
import { readFileSync } from 'fs';
import { resolve, dirname } from 'path';
import { fileURLToPath } from 'url';

const __dirname = dirname(fileURLToPath(import.meta.url));

async function main() {
  const flashBytes = readFileSync(resolve(__dirname, 'compiled-wifitest.bin'));
  const romBytes = readFileSync(resolve(__dirname, '../rom/esp32-v3-rom.bin'));

  // JS-only chip
  const cJS = new ESP32({ flashSizeMB: 4, mmuPages: 128, strapValue: 0x13 });
  cJS.flash.fill(0xff);
  cJS.flash.set(flashBytes);
  cJS.loadROM(romBytes);
  if (cJS.cores[1]) cJS.cores[1].enabled = true;
  cJS.reset();

  // WASM chip
  const wasmBytes = readFileSync(resolve(__dirname, '../src/engine/esp-xtensa/esp_engine_wasm.wasm'));
  const cW = new ESP32({ flashSizeMB: 4, mmuPages: 128, strapValue: 0x13 });
  cW.flash.fill(0xff);
  cW.flash.set(flashBytes);
  cW.loadROM(romBytes);
  await cW.loadWasm(wasmBytes, 'wasm');
  if (cW.cores[1]) cW.cores[1].enabled = true;
  cW.reset();

  function check(addr, label) {
    const jb0 = cJS.cores[0].readUint8(addr);
    const jb1 = cJS.cores[0].readUint8(addr + 1);
    const jb2 = cJS.cores[0].readUint8(addr + 2);
    const wb0 = cW.cores[0].readUint8(addr);
    const wb1 = cW.cores[0].readUint8(addr + 1);
    const wb2 = cW.cores[0].readUint8(addr + 2);
    const ok = jb0 === wb0 && jb1 === wb1 && jb2 === wb2;
    console.log(`${label} JS=[${jb0.toString(16)},${jb1.toString(16)},${jb2.toString(16)}] WASM=[${wb0.toString(16)},${wb1.toString(16)},${wb2.toString(16)}] ${ok ? 'OK' : 'DIFF'}`);
  }

  check(0x4000bff0, '0x4000bff0:');
  check(0x4000bfed, '0x4000bfed:');
  check(0x4000bfea, '0x4000bfea:');
  check(0x4000bfe7, '0x4000bfe7:');
  check(0x4000bfe4, '0x4000bfe4:');
  check(0x40070000, '0x40070000:');  // start of iram0
  check(0x40080000, '0x40080000:');  // start of iram1/cache
}
main().catch(e => { console.error(e); process.exit(1); });
