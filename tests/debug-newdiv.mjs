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

  const jsSR = cJS.cores[0].specialRegisters;
  const wmSR = cWM._wasmCores[0]._specRegs;

  console.log(`JS C0: PC=0x${cJS.cores[0].PC.toString(16)} nextPC=0x${cJS.cores[0].nextPC.toString(16)}`);
  console.log(`WM C0: PC=0x${cWM._wasmCores[0].PC.toString(16)} nextPC=0x${cWM._wasmCores[0].nextPC.toString(16)}`);

  // Compare key special registers
  const keys = [/*INT_ENABLE*/226, /*CLOCK_CONFIG*/228, /*PS*/230, /*INT_SET*/35,
    /*WINDOW_START*/17, /*MEM_FAULT_INFO*/72, /*CACHE_CONTROL*/73, /*INT_LEVEL*/231,
    /*CCOUNT*/234, /*CCOMPARE0*/264, /*CCOMPARE1*/265, /*CCOMPARE2*/266,
    /*EXCCAUSE*/3, /*PS_REGISTER*/4, /*SAR*/16, /*LBEG*/0, /*LEND*/1, /*LCOUNT*/2,
    /*ATOMCTRL*/14, /*CPENABLE*/18, /*INTERRUPT_STATE*/20,
    48,49,50,51,52,53,54,55,56,57,58,59,60,61,62,63];
  console.log('\n=== Register comparison ===');
  for (const k of keys) {
    const jv = (jsSR[k] >>> 0).toString(16).padStart(8, '0');
    const wv = (wmSR[k] >>> 0).toString(16).padStart(8, '0');
    const match = jv === wv ? '' : ' *** MISMATCH';
    console.log(`  [${k}] JS=0x${jv} WM=0x${wv}${match}`);
  }

  // Compare AR registers
  const jsBase = jsSR[72] << 2;
  const wmBase = wmSR[72] << 2;
  console.log('\n=== AR registers (first 16) ===');
  for (let i = 0; i < 16; i++) {
    const jv = (cJS.cores[0].physicalRegisters[(jsBase + i) % 64] >>> 0).toString(16).padStart(8, '0');
    const wv = (cWM._wasmCores[0]._physRegs[(wmBase + i) % 64] >>> 0).toString(16).padStart(8, '0');
    const match = jv === wv ? '' : ' *** MISMATCH';
    console.log(`  AR[${i}] JS=0x${jv} WM=0x${wv}${match}`);
  }

  // Compare pendingInterrupts
  console.log(`\n=== Pending interrupts ===`);
  console.log(`JS: pendingInterrupts=${cJS.cores[0].pendingInterrupts}`);
  console.log(`WM: pendingInterrupts=${cWM._wasmCores[0].pendingInterrupts}`);

  // Read instruction at PC
  const readInst = (chip, addr) => {
    try {
      const region = chip.mapAddress(addr, 0);
      const inst24 = region.readUint8(addr) | (region.readUint8(addr+1) << 8) | (region.readUint8(addr+2) << 16);
      return '0x' + inst24.toString(16).padStart(6, '0');
    } catch { return '???'; }
  };
  console.log(`\n=== Opcodes at PC ===`);
  console.log(`JS @0x${cJS.cores[0].PC.toString(16)}: ${readInst(cJS, cJS.cores[0].PC)}`);
  console.log(`WM @0x${cWM._wasmCores[0].PC.toString(16)}: ${readInst(cWM, cWM._wasmCores[0].PC)}`);

  // CCOUNT and clock ticks
  console.log(`\n=== CCOUNT / Clock ===`);
  console.log(`JS CCOUNT=${jsSR[234]} clock.ticks=${cJS.clocks.cpu.ticks}`);
  console.log(`WM CCOUNT=${wmSR[234]} clock.ticks=${cWM.clocks.cpu.ticks}`);
}

main().catch(e => console.error(e));
