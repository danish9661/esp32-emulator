import { ESP32 } from '../src/peripherals/esp32/esp32.js';
import { readFileSync } from 'fs';
import { resolve, dirname } from 'path';
import { fileURLToPath } from 'url';

const __dirname = dirname(fileURLToPath(import.meta.url));
const flashBytes = readFileSync(resolve(__dirname, 'compiled-wifitest.bin'));
const romBytes = readFileSync(resolve(__dirname, '../rom/esp32-v3-rom.bin'));
const wasmBytes = readFileSync(resolve(__dirname, '../src/engine/esp-xtensa/esp_engine_wasm.wasm'));

const DIV_STEP = 10105000; // step BEFORE divergence (both at PC=0x40000480)

async function main() {
  // JS chip - run to step BEFORE divergence to dump C1 state
  const cJS = new ESP32({ flashSizeMB: 4, mmuPages: 128, strapValue: 0x13 });
  cJS.flash.fill(0xff);
  cJS.flash.set(flashBytes);
  cJS.loadROM(romBytes);
  cJS.cores[1].enabled = true;
  cJS.reset();
  for (let i = 0; i < DIV_STEP; i++) { cJS.step(); }

  // WASM chip
  const cWM = new ESP32({ flashSizeMB: 4, mmuPages: 128, strapValue: 0x13 });
  cWM.flash.fill(0xff);
  cWM.flash.set(flashBytes);
  cWM.loadROM(romBytes);
  await cWM.loadWasm(wasmBytes, 'wasm');
  cWM.cores[1].enabled = true;
  cWM.reset();
  for (let i = 0; i < DIV_STEP; i++) { cWM.step(); }

  // Core 1 special register comparison
  const js1SR = cJS.cores[1].specialRegisters;
  const wm1SR = cWM._wasmCores[1]._specRegs;

  console.log(`=== Core 1 state at step ${DIV_STEP} (after divergence) ===`);
  console.log(`JS C1: PC=0x${cJS.cores[1].PC.toString(16)}`);
  console.log(`WM C1: PC=0x${cWM._wasmCores[1].PC.toString(16)}`);

  const keys = [0,1,2,3,4,16,17,18,20,33,34,35,72,73,
    226,227,228,229,230,231,232,233,234,
    235,236,237,238,239,240,241,242,243,
    244,245,246,247,248,249,250,251,252,253,254,255,
    264,265,266];
  console.log('\n=== C1 Special registers ===');
  let mismatches = [];
  for (const k of keys) {
    const jv = (js1SR[k] >>> 0).toString(16).padStart(8, '0');
    const wv = (wm1SR[k] >>> 0).toString(16).padStart(8, '0');
    const match = jv === wv;
    if (!match) mismatches.push(`  [${k}] JS=0x${jv} WM=0x${wv} ***`);
  }
  if (mismatches.length === 0) {
    console.log('  All checked registers match');
  } else {
    for (const m of mismatches) console.log(m);
  }

  // Core 0 comparison at this step too
  const js0SR = cJS.cores[0].specialRegisters;
  const wm0SR = cWM._wasmCores[0]._specRegs;
  console.log('\n=== C0 Special registers ===');
  mismatches = [];
  for (const k of keys) {
    const jv = (js0SR[k] >>> 0).toString(16).padStart(8, '0');
    const wv = (wm0SR[k] >>> 0).toString(16).padStart(8, '0');
    const match = jv === wv;
    if (!match) mismatches.push(`  [${k}] JS=0x${jv} WM=0x${wv} ***`);
  }
  if (mismatches.length === 0) {
    console.log('  All checked registers match');
  } else {
    for (const m of mismatches) console.log(m);
  }

  // pendingInterrupts and enabled
  console.log(`\nJS C0: pendingInt=${cJS.cores[0].pendingInterrupts} enabled=${cJS.cores[0].enabled}`);
  console.log(`WM C0: pendingInt=${cWM._wasmCores[0].pendingInterrupts} enabled=${cWM._wasmCores[0].enabled}`);
  console.log(`JS C1: pendingInt=${cJS.cores[1].pendingInterrupts} enabled=${cJS.cores[1].enabled}`);
  console.log(`WM C1: pendingInt=${cWM._wasmCores[1].pendingInterrupts} enabled=${cWM._wasmCores[1].enabled}`);
}
main().catch(e => console.error(e));
