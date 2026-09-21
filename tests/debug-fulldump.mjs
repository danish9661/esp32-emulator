import { ESP32 } from '../src/peripherals/esp32/esp32.js';
import { readFileSync } from 'fs';
import { resolve, dirname } from 'path';
import { fileURLToPath } from 'url';

const __dirname = dirname(fileURLToPath(import.meta.url));
const flashBytes = readFileSync(resolve(__dirname, 'compiled-wifitest.bin'));
const romBytes = readFileSync(resolve(__dirname, '../rom/esp32-v3-rom.bin'));
const wasmBytes = readFileSync(resolve(__dirname, '../src/engine/esp-xtensa/esp_engine_wasm.wasm'));

const DIV_STEP = 10599425;

async function main() {
  // JS chip
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

  // Full physical registers comparison
  console.log('=== Physical Registers (all 64) ===');
  const jsPhys = cJS.cores[0].physicalRegisters;
  const wmPhys = cWM._wasmCores[0]._physRegs;
  let physMismatches = [];
  for (let i = 0; i < 64; i++) {
    if (jsPhys[i] !== wmPhys[i]) {
      physMismatches.push(i);
    }
  }
  if (physMismatches.length === 0) {
    console.log('All match');
  } else {
    for (const i of physMismatches) {
      console.log(`  [${i}] JS=0x${(jsPhys[i]>>>0).toString(16).padStart(8,'0')} WM=0x${(wmPhys[i]>>>0).toString(16).padStart(8,'0')}`);
    }
  }

  // Full user registers comparison
  console.log('\n=== User Registers (all 237) ===');
  const jsUser = cJS.cores[0].userRegisters;
  const wmUser = cWM._wasmCores[0]; // WasmCore doesn't have userRegisters - they're in SAB too
  const wmUserArr = new Uint32Array(cWM._wasmCores[0]._loader.memory.buffer,
    cWM._wasmCores[0].index * 4096 + 336 + 256*4, 237); // after special_registers
  let userMismatches = [];
  for (let i = 0; i < 237; i++) {
    if (jsUser[i] !== wmUserArr[i]) {
      userMismatches.push(i);
    }
  }
  if (userMismatches.length === 0) {
    console.log('All match');
  } else {
    for (const i of userMismatches) {
      console.log(`  [${i}] JS=0x${(jsUser[i]>>>0).toString(16).padStart(8,'0')} WM=0x${(wmUserArr[i]>>>0).toString(16).padStart(8,'0')}`);
    }
  }

  // Special registers (all 256)
  console.log('\n=== Special Registers (all 256) ===');
  const jsSR = cJS.cores[0].specialRegisters;
  const wmSR = cWM._wasmCores[0]._specRegs;
  let specMismatches = [];
  for (let i = 0; i < 256; i++) {
    if (jsSR[i] !== wmSR[i]) {
      specMismatches.push(i);
    }
  }
  if (specMismatches.length === 0) {
    console.log('All match');
  } else {
    for (const i of specMismatches) {
      console.log(`  [${i}] JS=0x${(jsSR[i]>>>0).toString(16).padStart(8,'0')} WM=0x${(wmSR[i]>>>0).toString(16).padStart(8,'0')}`);
    }
  }

  // Compare some key memory regions
  console.log('\n=== Memory comparison (DRAM0 SRAM) ===');
  const jsDRAM = cJS.dataMem.data;
  const wmDRAM = cWM.dataMem.data;
  let memDiffs = [];
  // Check first 256 bytes of DRAM
  for (let i = 0; i < 256 && memDiffs.length < 20; i++) {
    if (jsDRAM[cWM.dataMem.baseAddr + i] !== wmDRAM[cWM.dataMem.baseAddr + i]) {
      memDiffs.push({off: cWM.dataMem.baseAddr + i, js: jsDRAM[cWM.dataMem.baseAddr + i], wm: wmDRAM[cWM.dataMem.baseAddr + i]});
    }
  }
  if (memDiffs.length === 0) {
    console.log('First 256 bytes of DRAM match');
  } else {
    for (const d of memDiffs) {
      console.log(`  @0x${d.off.toString(16)} JS=0x${d.js.toString(16)} WM=0x${d.wm.toString(16)}`);
    }
  }

  // Also check a few pages of IRAM
  console.log('\n=== Memory comparison (IRAM 0x40080000-0x40082000) ===');
  const jsIRAM = cJS.iram?.data || cJS.sram1Reverse; // Flash cache for IRAM
  const wmIRAM = cWM.iram?.data || cWM.sram1Reverse;
  // IRAM is mapped through flash MMU - let's read directly
  let iramDiffs = [];
  for (let addr = 0x40080000; addr < 0x40082000 && iramDiffs.length < 20; addr += 4) {
    let jv, wv;
    try {
      const rJS = cJS.mapAddress(addr, 0);
      jv = rJS.readUint32(addr);
    } catch { jv = 0xffffffff; }
    try {
      const rWM = cWM.mapAddress(addr, 0);
      wv = rWM.readUint32(addr);
    } catch { wv = 0xffffffff; }
    if (jv !== wv) {
      iramDiffs.push({addr, js: jv, wm: wv});
    }
  }
  if (iramDiffs.length === 0) {
    console.log('All IRAM (0x40080000-0x40082000) matches');
  } else {
    for (const d of iramDiffs) {
      console.log(`  @0x${d.addr.toString(16)} JS=0x${(d.js>>>0).toString(16).padStart(8,'0')} WM=0x${(d.wm>>>0).toString(16).padStart(8,'0')}`);
    }
  }

  // CORE 1 state
  console.log('\n=== Core 1 comparison ===');
  const js1PC = cJS.cores[1].PC;
  const wm1PC = cWM._wasmCores[1].PC;
  console.log(`PC: JS=0x${js1PC.toString(16)} WM=0x${wm1PC.toString(16)} ${js1PC !== wm1PC ? '*** MISMATCH' : ''}`);
  console.log(`pendingInterrupts: JS=${cJS.cores[1].pendingInterrupts} WM=${cWM._wasmCores[1].pendingInterrupts}`);
  const js1SR = cJS.cores[1].specialRegisters;
  const wm1SR = cWM._wasmCores[1]._specRegs;
  for (const k of [226, 228, 230, 35, 234, 264, 265, 266]) {
    const jv = (js1SR[k] >>> 0).toString(16).padStart(8,'0');
    const wv = (wm1SR[k] >>> 0).toString(16).padStart(8,'0');
    if (jv !== wv) console.log(`  C1 [${k}] JS=0x${jv} WM=0x${wv} *** MISMATCH`);
  }
}

main().catch(e => console.error(e));
