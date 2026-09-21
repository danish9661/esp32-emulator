import { ESP32 } from '../src/index.js';
import { readFileSync } from 'fs';
import { resolve, dirname } from 'path';
import { fileURLToPath } from 'url';
import axios from 'axios';

const __dirname = dirname(fileURLToPath(import.meta.url));

async function compileArduino(code) {
  const startRes = await axios.post('http://localhost:5525/api/compile/start', { code, target: 'esp32', targetEngine: 'frontend', fqbn: 'esp32:esp32:esp32' });
  const buildId = startRes.data.buildId;
  while (true) {
    const statRes = await axios.get(`http://localhost:5525/api/compile/status/${buildId}`);
    if (statRes.data.status === 'success') return statRes.data.binary_content;
    else if (statRes.data.status === 'failed') throw new Error('Compile failed: ' + statRes.data.error);
    await new Promise(r => setTimeout(r, 1000));
  }
}

const code = `void setup() { Serial.begin(115200); Serial.println("Hello from WASM!"); } void loop() { delay(1000); }`;
const b64 = await compileArduino(code);
const firmwareBytes = new Uint8Array(Buffer.from(b64, 'base64'));
console.log(`Firmware: ${firmwareBytes.length} bytes`);
console.log(`Flash at 0x1000:`, [...firmwareBytes.slice(0x1000,0x1010)].map(b=>'0x'+b.toString(16).padStart(2,'0')).join(' '));

const flashSizeMB = Math.max(4, Math.ceil(firmwareBytes.length / (1024 * 1024)));
const flash = new Uint8Array(flashSizeMB * 1024 * 1024);
flash.fill(0xff);
flash.set(firmwareBytes);

const romBytes = readFileSync(resolve(__dirname, '../rom/esp32-v3-rom.bin'));
const wasmBytes = readFileSync(resolve(__dirname, '../src/engine/esp-xtensa/esp_engine_wasm.wasm'));

const esp32 = new ESP32({ flashSizeMB, flash, strapValue: 0x13 });
esp32.loadROM(romBytes);
esp32.reset();
console.log(`Initial PC=0x${esp32.cores[0].PC.toString(16)}`);
console.log(`flash[0]=0x${esp32.flash[0].toString(16)} flash[0x1000]=0x${esp32.flash[0x1000].toString(16)}`);

const loaded = await esp32.loadWasm(wasmBytes, 'wasm');
if (!loaded) { console.log('FAILED'); process.exit(1); }

for (let i = 0; i < 10000; i++) {
  esp32.step();
  const pc = esp32.cores[0].PC;
  const uart = esp32._uartOutput?.[0] || '';
  if (pc >= 0x40080000 && pc < 0x40100000) {
    console.log(`step ${i}: PC=0x${pc.toString(16)} UART=${uart.length > 0 ? uart.slice(0,80) : '(empty)'} BOOTLOADER ENTERED`);
  }
  if (i % 500 === 0) {
    console.log(`step ${i}: PC=0x${pc.toString(16)} cycles=${esp32.cycles} uartLen=${uart.length}`);
  }
  if (uart.includes('Hello')) {
    console.log(`SUCCESS at step ${i}: ${uart}`);
    process.exit(0);
  }
}
console.log('FAILED: no UART output after 10000 steps');
process.exit(1);
