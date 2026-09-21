import { ESP32 } from '../src/index.js';
import { readFileSync } from 'fs';
import { fileURLToPath } from 'url';
import { dirname, join } from 'path';

const __dirname = dirname(fileURLToPath(import.meta.url));
const BOOTROM_PATH = join(__dirname, '..', 'rom', 'esp32-v3-rom.bin');
const WASM_PATH = join(__dirname, '..', 'src', 'engine', 'esp-xtensa', 'esp_engine_wasm.wasm');

const rom = readFileSync(BOOTROM_PATH);
const wasmBytes = readFileSync(WASM_PATH);

const flash = new Uint8Array(4 * 1024 * 1024);
flash.fill(0xff);
flash[0x1000] = 0xe9; flash[0x1001] = 1; flash[0x1002] = 0x20; flash[0x1003] = 0x40;
flash[0x1004] = 0x00; flash[0x1005] = 0x00; flash[0x1006] = 0x00; flash[0x1007] = 0x40;
flash[0x100c] = 0x00; flash[0x100d] = 0x00; flash[0x100e] = 0x00; flash[0x100f] = 0x00;
flash[0x1014] = 0x00; flash[0x1015] = 0x00; flash[0x1016] = 0x00; flash[0x1017] = 0x40;
flash[0x1018] = 0x04; flash[0x1019] = 0x00; flash[0x101a] = 0x00; flash[0x101b] = 0x00;
flash[0x101c] = 0x00; flash[0x101d] = 0x00; flash[0x101e] = 0x00; flash[0x101f] = 0x00;
flash[0x1020] = 0xef;

// Method 1: exact wasm-bench setup
console.log('=== Method 1: exact wasm-bench setup ===');
{
  const esp32 = new ESP32({ flashSizeMB: 4, flash });
  esp32.loadROM(rom);
  if (esp32.gpio?.pins) {
    esp32.gpio.pins[0].inputValue = true;
    esp32.gpio.pins[2].inputValue = false;
    esp32.gpio.pins[12].inputValue = false;
    esp32.gpio.pins[15].inputValue = false;
  }
  esp32.reset();
  esp32.flash.set(flash);
  if (esp32.gpio) esp32.gpio.strapValue = 0x13;
  if (esp32.mmuTablePro) {
    for (let p = 0; p < 64; p++) { esp32.mmuTablePro[p] = p; esp32.mmuTableApp[p] = p; }
  }
  try { esp32.cores[0].writeUint32(0x3ff5a104, 0x5aa5); } catch {}
  if (esp32.uart[0]) {
    esp32.uart[0].onTX = (b) => { process.stdout.write(String.fromCharCode(b)); };
  }
  const loaded = await esp32.loadWasm(wasmBytes, 'wasm');
  
  console.log(`Initial PC=0x${esp32._wasmCores[0].PC.toString(16)}`);
  console.log(`flash[0]=${esp32.flash[0].toString(16)} flash[0x1000]=${esp32.flash[0x1000].toString(16)}`);
  console.log(`ROM[0]=${esp32.chipROM.data[0].toString(16)} PC_offset=${esp32._wasmCores[0].PC - 0x40000000}`);
  
  // Check what's at the reset vector
  const romOffset = esp32._wasmCores[0].PC - 0x40000000;
  console.log(`ROM @ PC: ${new Uint8Array(esp32.chipROM.data.buffer, romOffset, 16).join(',')}`);
  
  for (let b = 0; b < 6; b++) {
    esp32.step();
    const pc = esp32._wasmCores[0].PC;
    console.log(`batch ${b}: PC=0x${pc.toString(8).padStart(8,'0')} cycles=${esp32.cycles}`);
    if (pc >= 0x4000fddf) { console.log('BOOT COMPLETE!'); break; }
  }
  console.log(`Final: PC=0x${esp32._wasmCores[0].PC.toString(16)}`);
}

// Method 2: same but with mmuPages = Math.ceil(flash.length / 65536)
console.log('\n=== Method 2: mmuPages=64 ===');
{
  const esp32 = new ESP32({ flashSizeMB: 4, flash });
  esp32.loadROM(rom);
  if (esp32.gpio?.pins) {
    esp32.gpio.pins[0].inputValue = true;
    esp32.gpio.pins[2].inputValue = false;
    esp32.gpio.pins[12].inputValue = false;
    esp32.gpio.pins[15].inputValue = false;
  }
  esp32.reset();
  esp32.flash.set(flash);
  if (esp32.gpio) esp32.gpio.strapValue = 0x13;
  const mmuPages = Math.ceil(4 * 1024 * 1024 / 65536);
  if (esp32.mmuTablePro) {
    for (let p = 0; p < mmuPages; p++) { esp32.mmuTablePro[p] = p; esp32.mmuTableApp[p] = p; }
  }
  try { esp32.cores[0].writeUint32(0x3ff5a104, 0x5aa5); } catch {}
  if (esp32.uart[0]) {
    esp32.uart[0].onTX = (b) => { process.stdout.write(String.fromCharCode(b)); };
  }
  const loaded = await esp32.loadWasm(wasmBytes, 'wasm');
  
  console.log(`mmuPages=${mmuPages} Initial PC=0x${esp32._wasmCores[0].PC.toString(16)}`);
  
  for (let b = 0; b < 6; b++) {
    esp32.step();
    const pc = esp32._wasmCores[0].PC;
    console.log(`batch ${b}: PC=0x${pc.toString(16)} cycles=${esp32.cycles}`);
    if (pc >= 0x4000fddf) { console.log('BOOT COMPLETE!'); break; }
  }
  console.log(`Final: PC=0x${esp32._wasmCores[0].PC.toString(16)}`);
}

// Method 3: change config — DON'T pass flash in config
console.log('\n=== Method 3: no flash in config ===');
{
  const esp32 = new ESP32({ flashSizeMB: 4 });
  esp32.loadROM(rom);
  if (esp32.gpio?.pins) {
    esp32.gpio.pins[0].inputValue = true;
    esp32.gpio.pins[2].inputValue = false;
    esp32.gpio.pins[12].inputValue = false;
    esp32.gpio.pins[15].inputValue = false;
  }
  esp32.reset();
  esp32.flash.set(flash);
  if (esp32.gpio) esp32.gpio.strapValue = 0x13;
  const mmuPages = Math.ceil(4 * 1024 * 1024 / 65536);
  if (esp32.mmuTablePro) {
    for (let p = 0; p < mmuPages; p++) { esp32.mmuTablePro[p] = p; esp32.mmuTableApp[p] = p; }
  }
  try { esp32.cores[0].writeUint32(0x3ff5a104, 0x5aa5); } catch {}
  if (esp32.uart[0]) {
    esp32.uart[0].onTX = (b) => { process.stdout.write(String.fromCharCode(b)); };
  }
  const loaded = await esp32.loadWasm(wasmBytes, 'wasm');
  
  console.log(`Initial PC=0x${esp32._wasmCores[0].PC.toString(16)}`);
  
  for (let b = 0; b < 6; b++) {
    esp32.step();
    const pc = esp32._wasmCores[0].PC;
    console.log(`batch ${b}: PC=0x${pc.toString(16)} cycles=${esp32.cycles}`);
    if (pc >= 0x4000fddf) { console.log('BOOT COMPLETE!'); break; }
  }
  console.log(`Final: PC=0x${esp32._wasmCores[0].PC.toString(16)}`);
}
