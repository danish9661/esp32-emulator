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

  const START = 10105000;
  const END = 10105020;

  for (let step = 0; step < END; step++) {
    const js_c1_before = cJS.cores[1] ? cJS.cores[1].PC >>> 0 : 0;
    const wm_c1_before = cW._wasmCores?.[1] ? cW._wasmCores[1].PC >>> 0 : 0;

    cJS.step();
    cW.step();

    const js_c0 = cJS.cores[0].PC >>> 0;
    const wm_c0 = cW._wasmCores[0].PC >>> 0;
    const js_c1 = cJS.cores[1] ? cJS.cores[1].PC >>> 0 : 0;
    const wm_c1 = cW._wasmCores?.[1] ? cW._wasmCores[1].PC >>> 0 : 0;
    const js_c1pc_before = cJS.cores[1] ? cJS.cores[1].PC >>> 0 : 0;
    const wm_c1pc_before = cW._wasmCores?.[1] ? cW._wasmCores[1].PC >>> 0 : 0;

    if (step >= START) {
      const c1_eq = js_c1 === wm_c1 ? '=' : 'X';
      const c0_eq = js_c0 === wm_c0 ? '=' : 'X';
      // Also show registers if divergence
      if (c1_eq === 'X') {
        const jr = cJS.cores[1].physicalRegisters;
        const wr = cW._wasmCores[1]._physRegs;
        const regDiffs = [];
        for (let r = 0; r < 64; r++) {
          if (jr[r] !== wr[r]) regDiffs.push(`PHYS[${r}]:JS=${fmt(jr[r])} WM=${fmt(wr[r])}`);
        }
        console.log(`[${step}] C1:${c1_eq} C0:${c0_eq} JS1=${fmt(js_c1pc_before)}->${fmt(js_c1)} WM1=${fmt(wm_c1pc_before)}->${fmt(wm_c1)} ${regDiffs.join(' ')}`);
      } else {
        console.log(`[${step}] C1:${c1_eq} C0:${c0_eq} JS1=${fmt(js_c1)} WM1=${fmt(wm_c1)}`);
      }
    }
  }
}
main().catch(e => { console.error(e); process.exit(1); });
