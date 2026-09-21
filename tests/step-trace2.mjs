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

  const TRACE_START = 10547640;
  const TRACE_END = 10547650;
  let core1DivAt = -1;

  for (let step = 0; step < TRACE_END; step++) {
    // Track core 1 PC BEFORE step
    const js_c1_before = cJS.cores[1] ? cJS.cores[1].PC >>> 0 : 0;
    const wm_c1_before = cW._wasmCores?.[1] ? cW._wasmCores[1].PC >>> 0 : 0;

    cJS.step();
    cW.step();

    const js_c0 = cJS.cores[0].PC >>> 0;
    const wm_c0 = cW._wasmCores[0].PC >>> 0;
    const js_c1 = cJS.cores[1] ? cJS.cores[1].PC >>> 0 : 0;
    const wm_c1 = cW._wasmCores?.[1] ? cW._wasmCores[1].PC >>> 0 : 0;

    // Check for core 1 divergence
    if (js_c1 !== wm_c1 && core1DivAt === -1) core1DivAt = step;

    if (step >= TRACE_START) {
      const c0eq = js_c0 === wm_c0 ? '=' : 'X';
      const c1eq = js_c1 === wm_c1 ? '=' : 'X';
      const marker = step === 10547643 ? ' <<<' : '';
      console.log(`[${step}] C0:${c0eq} JS=${fmt(js_c0)} WM=${fmt(wm_c0)}  C1:${c1eq} JS=${fmt(js_c1)} WM=${fmt(wm_c1)}${marker}`);
    }
  }
  if (core1DivAt >= 0) {
    console.log(`Core 1 first diverged at step ${core1DivAt}`);
  } else {
    console.log('Core 1 never diverged in this range');
  }
}
main().catch(e => { console.error(e); process.exit(1); });
