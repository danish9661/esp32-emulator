import { ESP32 } from '../src/peripherals/esp32/esp32.js';
import { readFileSync } from 'fs';
import { resolve, dirname } from 'path';
import { fileURLToPath } from 'url';

const __dirname = dirname(fileURLToPath(import.meta.url));
const flashBytes = readFileSync(resolve(__dirname, 'compiled-wifitest.bin'));
const romBytes = readFileSync(resolve(__dirname, '../rom/esp32-v3-rom.bin'));
const wasmBytes = readFileSync(resolve(__dirname, '../src/engine/esp-xtensa/esp_engine_wasm.wasm'));

// Trace around C1 divergence point (10105001)
const START = 10105000 - 5;
const END = 10105005;

async function main() {
  // JS chip - track pendingInterrupts per step
  const cJS = new ESP32({ flashSizeMB: 4, mmuPages: 128, strapValue: 0x13 });
  cJS.flash.fill(0xff);
  cJS.flash.set(flashBytes);
  cJS.loadROM(romBytes);
  cJS.cores[1].enabled = true;
  cJS.reset();
  for (let i = 0; i < START; i++) { cJS.step(); }

  console.log('=== JS chip trace ===');
  for (let step = START; step < END; step++) {
    const c0pc = cJS.cores[0].PC;
    const c1pc = cJS.cores[1].PC;
    const c0pi = cJS.cores[0].pendingInterrupts;
    const c1pi = cJS.cores[1].pendingInterrupts;
    const c0ie = cJS.cores[0].specialRegisters[226];
    const c1ie = cJS.cores[1].specialRegisters[226];
    console.log(`[${step}] C0: PC=0x${c0pc.toString(16)} pi=${c0pi} ie=0x${(c0ie>>>0).toString(16)} | C1: PC=0x${c1pc.toString(16)} pi=${c1pi} ie=0x${(c1ie>>>0).toString(16)}`);
    cJS.step();
  }

  // WASM chip
  const cWM = new ESP32({ flashSizeMB: 4, mmuPages: 128, strapValue: 0x13 });
  cWM.flash.fill(0xff);
  cWM.flash.set(flashBytes);
  cWM.loadROM(romBytes);
  await cWM.loadWasm(wasmBytes, 'wasm');
  cWM.cores[1].enabled = true;
  cWM.reset();
  for (let i = 0; i < START; i++) { cWM.step(); }

  console.log('\n=== WASM chip trace ===');
  for (let step = START; step < END; step++) {
    const c0pc = cWM._wasmCores[0].PC;
    const c1pc = cWM._wasmCores[1].PC;
    const c0pi = cWM._wasmCores[0].pendingInterrupts;
    const c1pi = cWM._wasmCores[1].pendingInterrupts;
    const c0ie = cWM._wasmCores[0]._specRegs[226];
    const c1ie = cWM._wasmCores[1]._specRegs[226];
    console.log(`[${step}] C0: PC=0x${c0pc.toString(16)} pi=${c0pi} ie=0x${(c0ie>>>0).toString(16)} | C1: PC=0x${c1pc.toString(16)} pi=${c1pi} ie=0x${(c1ie>>>0).toString(16)}`);
    cWM.step();
  }
}
main().catch(e => console.error(e));
