import { ESP32 } from '../src/peripherals/esp32/esp32.js';
import { readFileSync } from 'fs';
import { resolve, dirname } from 'path';
import { fileURLToPath } from 'url';

const __dirname = dirname(fileURLToPath(import.meta.url));
function fmt(v) { return '0x' + (v >>> 0).toString(16).padStart(8, '0'); }

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

  // Save core 0 and core 1 PC histories
  const N = 10105010;
  const c0_js = new Uint32Array(N);
  const c0_wm = new Uint32Array(N);
  const c1_js = new Uint32Array(N);
  const c1_wm = new Uint32Array(N);

  for (let step = 0; step < N; step++) {
    const js_c1_before = cJS.cores[1] ? cJS.cores[1].PC >>> 0 : 0;

    cJS.step();
    cW.step();

    c0_js[step] = cJS.cores[0].PC >>> 0;
    c0_wm[step] = cW._wasmCores[0].PC >>> 0;
    c1_js[step] = js_c1_before; // before step, so it's the PC of the instruction ABOUT to execute
    c1_wm[step] = cW._wasmCores[1].PC >>> 0;

    // Actually, wm_c1_before is not captured, but the WASM core 1 after step is ok for comparison
    // Let's just use after-step values
  }

  // Now analyze: find first step where c1 diverges
  // c1_js[i] is PC before step i on core 1 (JS), c1_wm[i] is... hmm
  // Let me redo this properly

  console.log('Done collecting');
}
main().catch(e => { console.error(e); process.exit(1); });
