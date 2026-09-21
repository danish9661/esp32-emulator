import { ESP32 } from '../src/index.js';
import { readFileSync } from 'fs';
import { fileURLToPath } from 'url';
import { dirname, join } from 'path';

const __dirname = dirname(fileURLToPath(import.meta.url));
const BOOTROM_PATH = join(__dirname, '..', 'rom', 'esp32-v3-rom.bin');
const WASM_PATH = join(__dirname, '..', 'src', 'engine', 'esp-xtensa', 'esp_engine_wasm.wasm');

const rom = readFileSync(BOOTROM_PATH);
const wasmBytes = readFileSync(WASM_PATH);

// Create flash with valid boot header at 0x1000
const flash = new Uint8Array(4 * 1024 * 1024);
flash.fill(0xff);
flash[0x1000] = 0xe9; flash[0x1001] = 1; flash[0x1002] = 0x20; flash[0x1003] = 0x40;
flash[0x1004] = 0x00; flash[0x1005] = 0x00; flash[0x1006] = 0x00; flash[0x1007] = 0x40;
flash[0x100c] = 0x00; flash[0x100d] = 0x00; flash[0x100e] = 0x00; flash[0x100f] = 0x00;
flash[0x1014] = 0x00; flash[0x1015] = 0x00; flash[0x1016] = 0x00; flash[0x1017] = 0x40;
flash[0x1018] = 0x04; flash[0x1019] = 0x00; flash[0x101a] = 0x00; flash[0x101b] = 0x00;
flash[0x101c] = 0x00; flash[0x101d] = 0x00; flash[0x101e] = 0x00; flash[0x101f] = 0x00;
flash[0x1020] = 0xef;

function applySetup(chip, strapValue, mmuPages) {
  if (chip.gpio?.pins) {
    if (chip.gpio.pins[0]) chip.gpio.pins[0].inputValue = true;
    if (chip.gpio.pins[2]) chip.gpio.pins[2].inputValue = false;
    if (chip.gpio.pins[12]) chip.gpio.pins[12].inputValue = false;
    if (chip.gpio.pins[15]) chip.gpio.pins[15].inputValue = false;
  }
  chip.reset();
  chip.flash.set(flash);
  if (chip.gpio) chip.gpio.strapValue = strapValue;
  if (chip.mmuTablePro) {
    for (let p = 0; p < mmuPages; p++) { chip.mmuTablePro[p] = p; chip.mmuTableApp[p] = p; }
  }
  try { chip.cores[0].writeUint32(0x3ff5a104, 0x5aa5); } catch {}
  if (chip.uart?.[0]) {
    chip.uart[0].onTX = (b) => {
      const c = String.fromCharCode(b);
      process.stdout.write(c === '\n' ? '\n' : c);
    };
  }
  if (chip.cores[1]) chip.cores[1].enabled = true;
}

console.log('\n=== Test 1: wasm-bench.mjs style (no second reset) ===');
{
  const chip = new ESP32({ flashSizeMB: 4, flash });
  chip.loadROM(rom);
  applySetup(chip, 0x13, 64);
  await chip.loadWasm(wasmBytes, 'wasm');
  console.log(`Initial PC=0x${chip._wasmCores[0].PC.toString(16)}`);
  for (let b = 0; b < 200; b++) {
    chip.step();
    const pc = chip._wasmCores[0].PC;
    if ((b % 20) === 0) console.log(`batch ${b}: PC=0x${pc.toString(16)} cycles=${chip.cycles}`);
    if (pc === 0x4000fddf || pc >= 0x4000fe00) {
      console.log(`Boot ROM completed at batch ${b}! PC=0x${pc.toString(16)}`);
      break;
    }
    if (pc >= 0x4000fc90 && pc <= 0x4000fde9) {
      // Check if stuck
      const uartLen = chip.uart?.[0]?.txBuffer?.length || 0;
      console.log(`WARNING: In error loop PC=0x${pc.toString(16)} batch=${b} uartLen=${uartLen}`);
      break;
    }
  }
  console.log(`Final: PC=0x${chip._wasmCores[0].PC.toString(16)} cycles=${chip.cycles}`);
}

console.log('\n=== Test 2: Worker style (second reset after loadWasm) ===');
{
  const chip = new ESP32({ flashSizeMB: 4, flash });
  chip.loadROM(rom);
  applySetup(chip, 0x13, 64);
  await chip.loadWasm(wasmBytes, 'wasm');
  
  // SECOND reset like worker-entry.js line 589
  console.log('Calling chip.reset() again...');
  chip.reset();
  console.log(`After reset: PC=0x${chip._wasmCores[0].PC.toString(16)}`);
  
  // Re-apply like worker-entry.js lines 594-623
  if (chip.mmuTablePro) {
    for (let p = 0; p < 64; p++) { chip.mmuTablePro[p] = p; chip.mmuTableApp[p] = p; }
  }
  if (chip.cores[1]) chip.cores[1].enabled = true;
  if (chip._wasmCores[1]) chip._wasmCores[1].enabled = true;
  if (chip.gpio) {
    chip.gpio.strapValue = 0x13;
    if (chip.gpio.pins[0]) chip.gpio.pins[0].inputValue = true;
    if (chip.gpio.pins[2]) chip.gpio.pins[2].inputValue = false;
    if (chip.gpio.pins[12]) chip.gpio.pins[12].inputValue = false;
    if (chip.gpio.pins[15]) chip.gpio.pins[15].inputValue = false;
  }
  if (chip.uart[0]) {
    chip.uart[0].onTX = (b) => {
      const c = String.fromCharCode(b);
      process.stdout.write(c === '\n' ? '\n' : c);
    };
  }
  try { chip.cores[0].writeUint32(0x3ff5a104, 0x5aa5); } catch {}

  console.log(`Re-applied. PC=0x${chip._wasmCores[0].PC.toString(16)}`);
  for (let b = 0; b < 200; b++) {
    chip.step();
    const pc = chip._wasmCores[0].PC;
    if ((b % 20) === 0) console.log(`batch ${b}: PC=0x${pc.toString(16)} cycles=${chip.cycles}`);
    if (pc === 0x4000fddf || pc >= 0x4000fe00) {
      console.log(`Boot ROM completed at batch ${b}! PC=0x${pc.toString(16)}`);
      break;
    }
    if (pc >= 0x4000fc90 && pc <= 0x4000fde9) {
      const uartLen = chip.uart?.[0]?.txBuffer?.length || 0;
      console.log(`WARNING: In error loop PC=0x${pc.toString(16)} batch=${b} uartLen=${uartLen}`);
      break;
    }
  }
  console.log(`Final: PC=0x${chip._wasmCores[0].PC.toString(16)} cycles=${chip.cycles}`);
}

console.log('\n=== Done ===');
