import { ESP32 } from '../src/peripherals/esp32/esp32.js';
import { readFileSync } from 'fs';
import { resolve, dirname } from 'path';
import { fileURLToPath } from 'url';

const __dirname = dirname(fileURLToPath(import.meta.url));

const flashBytes = readFileSync(resolve(__dirname, 'compiled-wifitest.bin'));
const romBytes = readFileSync(resolve(__dirname, '../rom/esp32-v3-rom.bin'));
const wasmBytes = readFileSync(resolve(__dirname, '../src/engine/esp-xtensa/esp_engine_wasm.wasm'));

async function main() {
  const cJS = new ESP32({ flashSizeMB: 4, mmuPages: 128, strapValue: 0x13 });
  cJS.flash.fill(0xff);
  cJS.flash.set(flashBytes);
  cJS.loadROM(romBytes);
  if (cJS.cores[1]) cJS.cores[1].enabled = true;
  cJS.reset();

  const cW = new ESP32({ flashSizeMB: 4, mmuPages: 128, strapValue: 0x13 });
  cW.flash.fill(0xff);
  cW.flash.set(flashBytes);
  cW.loadROM(romBytes);
  await cW.loadWasm(wasmBytes, 'wasm');
  if (cW.cores[1]) cW.cores[1].enabled = true;
  cW.reset();

  const TARGET = 10105000;

  for (let step = 0; step <= TARGET; step++) {
    // Capture BEFORE step
    const js_c1_before = cJS.cores[1] ? cJS.cores[1].PC >>> 0 : 0;
    const wm_c1_before = cW._wasmCores?.[1] ? cW._wasmCores[1].PC >>> 0 : 0;

    cJS.step();
    cW.step();

    const js_c1 = cJS.cores[1] ? cJS.cores[1].PC >>> 0 : 0;
    const wm_c1 = cW._wasmCores?.[1] ? cW._wasmCores[1].PC >>> 0 : 0;

    if (step === TARGET) {
      // Dump instruction at the divergence PC
      const addr = js_c1_before; // 0x40000480
      const b0 = cJS.cores[0].readUint8(addr);
      const b1 = cJS.cores[0].readUint8(addr + 1);
      const b2 = cJS.cores[0].readUint8(addr + 2);
      const op16 = (b0 | (b1 << 8)) >>> 0;
      const op24 = (b0 | (b1 << 8) | (b2 << 16)) >>> 0;
      const width = [3,3,3,3,3,3,3,3,2,2,2,2,2,2,4,4][op16 & 15];

      console.log(`[${step}] CORE 1 DIVERGENCE`);
      console.log(`  PC: 0x${addr.toString(16).padStart(8,'0')}`);
      console.log(`  bytes: [${b0.toString(16)}, ${b1.toString(16)}, ${b2.toString(16)}]`);
      console.log(`  op16: 0x${op16.toString(16).padStart(4,'0')}, op24: 0x${op24.toString(16).padStart(6,'0')}, width: ${width}`);
      console.log(`  JS:  ${'0x' + js_c1_before.toString(16)} -> ${'0x' + js_c1.toString(16)}`);
      console.log(`  WASM: ${'0x' + wm_c1_before.toString(16)} -> ${'0x' + wm_c1.toString(16)}`);

      // Check registers
      const jr = cJS.cores[1].physicalRegisters;
      const wr = cW._wasmCores[1]._physRegs;
      for (let r = 0; r < 64; r++) {
        if (jr[r] !== wr[r]) {
          console.log(`  PHYS[${r}]: JS=${'0x' + (jr[r]>>>0).toString(16).padStart(8,'0')} WASM=${'0x' + (wr[r]>>>0).toString(16).padStart(8,'0')}`);
        }
      }
    }
  }
}
main().catch(e => { console.error(e); process.exit(1); });
