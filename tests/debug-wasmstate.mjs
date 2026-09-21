import { ESP32 } from '../src/peripherals/esp32/esp32.js';
import { readFileSync } from 'fs';
import { resolve, dirname } from 'path';
import { fileURLToPath } from 'url';

const __dirname = dirname(fileURLToPath(import.meta.url));
function fmt(v) { return '0x' + (v >>> 0).toString(16).padStart(8, '0'); }

function dumpWasmCore(label, wc, jsCore) {
  console.log(`=== ${label} Core 0 ===`);
  console.log(`PC=0x${(wc.PC >>> 0).toString(16)} nextPC=0x${(wc.nextPC >>> 0).toString(16)}`);
  console.log(`enabled=${wc.enabled} idle=${wc.idle} pendingInterrupts=${wc.pendingInterrupts}`);
  
  // PS and fields
  const ps = wc.PS;
  const intlevel = ps & 0xF;
  const excm = (ps >> 4) & 1;
  const um = (ps >> 5) & 1;
  const woe = (ps >> 6) & 1;
  const owb = (ps >> 12) & 0xF;
  console.log(`PS=0x${ps.toString(16)} (INTLEVEL=${intlevel} EXCM=${excm} UM=${um} WOE=${woe} OWB=${owb})`);
  
  // Interrupt state
  console.log(`IntEnable=0x${(wc.intEnable >>> 0).toString(16)}`);
  console.log(`clockConfig=0x${(wc.clockConfig >>> 0).toString(16)}`);
  console.log(`lastOpcode=0x${(wc.lastOpcode >>> 0).toString(16)}`);
  console.log(`exccause=${wc.exccause}`);
}

const flashBytes = readFileSync(resolve(__dirname, 'compiled-wifitest.bin'));
const romBytes = readFileSync(resolve(__dirname, '../rom/esp32-v3-rom.bin'));
const wasmBytes = readFileSync(resolve(__dirname, '../src/engine/esp-xtensa/esp_engine_wasm.wasm'));

async function main() {
  const cWM = new ESP32({ flashSizeMB: 4, mmuPages: 128, strapValue: 0x13 });
  cWM.flash.fill(0xff);
  cWM.flash.set(flashBytes);
  cWM.loadROM(romBytes);
  await cWM.loadWasm(wasmBytes, 'wasm');
  cWM.cores[1].enabled = true;
  cWM.reset();

  const MAX = 10547643;
  for (let i = 0; i < MAX; i++) { cWM.step(); }
  
  const wc0 = cWM._wasmCores[0];
  console.log(`\n=== WASM at step ${MAX} (BEFORE divergence) ===`);
  dumpWasmCore('WASM', wc0, cWM.cores[0]);
  
  cWM.step();
  console.log(`\n=== WASM after divergence step ===`);
  dumpWasmCore('WASM', wc0, cWM.cores[0]);
  console.log(`PC=0x${(wc0.PC >>> 0).toString(16)}`);
}

main().catch(e => console.error(e));
