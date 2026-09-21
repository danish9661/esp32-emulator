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
  // JS-only chip
  const cJS = new ESP32({ flashSizeMB: 4, mmuPages: 128, strapValue: 0x13 });
  cJS.flash.fill(0xff);
  cJS.flash.set(flashBytes);
  cJS.loadROM(romBytes);
  if (cJS.cores[1]) cJS.cores[1].enabled = true;
  cJS.reset();

  // WASM chip
  const cW = new ESP32({ flashSizeMB: 4, mmuPages: 128, strapValue: 0x13 });
  cW.flash.fill(0xff);
  cW.flash.set(flashBytes);
  cW.loadROM(romBytes);
  await cW.loadWasm(wasmBytes, 'wasm');
  if (cW.cores[1]) cW.cores[1].enabled = true;
  cW.reset();

  // Run up to step 10547640, then trace step-by-step
  const TRACE_START = 10547640;
  const TRACE_END = 10547650;

  for (let step = 0; step < TRACE_END; step++) {
    const jsPC_before = cJS.cores[0].PC >>> 0;
    const wmPC_before = cW._wasmCores[0].PC >>> 0;

    cJS.step();
    cW.step();

    const jsPC = cJS.cores[0].PC >>> 0;
    const wmPC = cW._wasmCores[0].PC >>> 0;

    if (step >= TRACE_START) {
      // Read instruction bytes at pre-step PC
      const jb0 = cJS.cores[0].readUint8(jsPC_before);
      const jb1 = cJS.cores[0].readUint8(jsPC_before + 1);
      const jb2 = cJS.cores[0].readUint8(jsPC_before + 2);
      const wb0 = cW.cores[0].readUint8(wmPC_before);
      const wb1 = cW.cores[0].readUint8(wmPC_before + 1);
      const wb2 = cW.cores[0].readUint8(wmPC_before + 2);

      const jInst = fmt(jsPC_before) + ':[b0=' + jb0.toString(16) + ',b1=' + jb1.toString(16) + ',b2=' + jb2.toString(16) + ']';
      const wInst = fmt(wmPC_before) + ':[b0=' + wb0.toString(16) + ',b1=' + wb1.toString(16) + ',b2=' + wb2.toString(16) + ']';
      const eq = jsPC === wmPC ? '=' : 'X';
      const marker = step === 10547643 ? ' <<< DIV' : '';
      console.log(`[${step}] ${eq} JS: ${jInst} -> ${fmt(jsPC)};  WASM: ${wInst} -> ${fmt(wmPC)}${marker}`);
    }

    if (step === TRACE_START - 1) {
      console.log('--- TRACE START ---');
    }
  }
}
main().catch(e => { console.error(e); process.exit(1); });
