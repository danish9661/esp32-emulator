import { ESP32 } from '../src/peripherals/esp32/esp32.js';
import { readFileSync } from 'fs';
import { resolve, dirname } from 'path';
import { fileURLToPath } from 'url';

const __dirname = dirname(fileURLToPath(import.meta.url));
const flashBytes = readFileSync(resolve(__dirname, 'compiled-wifitest.bin'));
const romBytes = readFileSync(resolve(__dirname, '../rom/esp32-v3-rom.bin'));
const wasmBytes = readFileSync(resolve(__dirname, '../src/engine/esp-xtensa/esp_engine_wasm.wasm'));

const MAX = 10547643;

async function main() {
  // JS chip
  const cJS = new ESP32({ flashSizeMB: 4, mmuPages: 128, strapValue: 0x13 });
  cJS.flash.fill(0xff);
  cJS.flash.set(flashBytes);
  cJS.loadROM(romBytes);
  cJS.cores[1].enabled = true;
  cJS.reset();
  for (let i = 0; i < MAX; i++) { cJS.step(); }

  // WASM chip
  const cWM = new ESP32({ flashSizeMB: 4, mmuPages: 128, strapValue: 0x13 });
  cWM.flash.fill(0xff);
  cWM.flash.set(flashBytes);
  cWM.loadROM(romBytes);
  await cWM.loadWasm(wasmBytes, 'wasm');
  cWM.cores[1].enabled = true;
  cWM.reset();
  for (let i = 0; i < MAX; i++) { cWM.step(); }

  // Dump JS core 0 special regs at important indices
  const jsSR = cJS.cores[0].specialRegisters;
  console.log('=== JS C0 special registers ===');
  const js_keys = [226, 228, 230, 35, 17, 72, 73, 231, 234, 264, 265, 266, 3, 4];
  for (const k of js_keys) {
    console.log(`  [${k}] = 0x${(jsSR[k] >>> 0).toString(16).padStart(8,'0')}`);
  }

  // Dump WASM core 0 special regs
  const wmSR = cWM._wasmCores[0]._specRegs;
  console.log('\n=== WASM C0 special registers ===');
  const wm_keys = [226 /* INT_ENABLE */, 228 /* CLOCK_CONFIG */, 230 /* PS */, 35 /* INT_SET */, 17 /* WINDOW_START */, 72 /* MEM_FAULT_INFO */, 73 /* CACHE_CONTROL */, 231 /* INT_LEVEL */, 234 /* CCOUNT */, 264, 265, 266 /* CCOMPARE0/1/2 */, 3 /* EXCCAUSE */, 4 /* PS - yes, there are TWO PS locations */];
  for (const k of wm_keys) {
    console.log(`  [${k}] = 0x${(wmSR[k] >>> 0).toString(16).padStart(8,'0')}`);
  }

  // Now check what compute_clocks_after_powerup does - clock_config might be set early
  // Compare the clock_event computation
  const js_clock_event = jsSR[226] & (jsSR[228] | 0x4000);
  const wm_clock_event = wmSR[226] & (wmSR[228] | 0x4000);
  console.log('\n=== Clock event comparison ===');
  console.log(`JS: IntEnable=0x${jsSR[226].toString(16)} ClockConfig=0x${jsSR[228].toString(16)} ClockEvent=0x${js_clock_event.toString(16)}`);
  console.log(`WM: IntEnable=0x${wmSR[226].toString(16)} ClockConfig=0x${wmSR[228].toString(16)} ClockEvent=0x${wm_clock_event.toString(16)}`);

  // Level masks
  const levelMask = {1: 407551, 2: 3670016, 3: 0x28c08800, 4: 0x53000000, 5: 0x84010000, 7: 16384};
  console.log('\n=== Level check for JS ===');
  for (let lvl = 7; lvl >= 1; lvl--) {
    if (lvl === 6) continue;
    const mask = levelMask[lvl] || 0;
    console.log(`  Level ${lvl}: event=0x${js_clock_event.toString(16)} mask=0x${mask.toString(16)} match=${(js_clock_event & mask) !== 0}`);
  }
  console.log('\n=== Level check for WASM ===');
  for (let lvl = 7; lvl >= 1; lvl--) {
    if (lvl === 6) continue;
    const mask = levelMask[lvl] || 0;
    console.log(`  Level ${lvl}: event=0x${wm_clock_event.toString(16)} mask=0x${mask.toString(16)} match=${(wm_clock_event & mask) !== 0}`);
  }

  // Also check pendingInterrupts
  console.log('\n=== Pending interrupts ===');
  console.log(`JS: pendingInterrupts=${cJS.cores[0].pendingInterrupts}`);
  console.log(`WM: pendingInterrupts=${cWM._wasmCores[0].pendingInterrupts}`);
}

main().catch(e => console.error(e));
