import { ESP32 } from '../src/peripherals/esp32/esp32.js';
import { readFileSync } from 'fs';
import { resolve, dirname } from 'path';
import { fileURLToPath } from 'url';
import axios from 'axios';
const __dirname = dirname(fileURLToPath(import.meta.url));

async function compile(code) {
  const r = await axios.post('http://localhost:5525/api/compile/start', { code, target: 'esp32', targetEngine: 'frontend', fqbn: 'esp32:esp32:esp32' });
  while (true) {
    const s = await axios.get(`http://localhost:5525/api/compile/status/${r.data.buildId}`);
    if (s.data.status === 'success') return s.data.binary_content;
    if (s.data.status === 'failed') throw new Error('Compile failed: ' + s.data.error);
    await new Promise(r => setTimeout(r, 1000));
  }
}

const firmware = `
#include <WiFi.h>
void setup() {
  Serial.begin(115200);
  Serial.println("\\n=== WIFI TEST ===\\n");
  WiFi.mode(WIFI_STA);
  WiFi.disconnect();
  delay(100);
  int n = WiFi.scanNetworks();
  Serial.print("[SCAN] found "); Serial.print(n); Serial.println(" networks");
  Serial.println("\\n=== ALL TESTS PASSED ===");
}
void loop() { delay(1000); }
`;

const b64 = await compile(firmware);
const fw = Buffer.from(b64, 'base64');
console.log('[diag] compiled', fw.length, 'bytes');

const rom = readFileSync(resolve(__dirname, '../rom/esp32-v3-rom.bin'));

function runTest(label, initFn) {
  const chip = new ESP32({flashSizeMB:4});
  initFn(chip, fw);
  if (chip.wifi.enabled === undefined) chip.wifi.enabled=true;
  if (chip.wifi.rxEnabled === undefined) chip.wifi.rxEnabled=true;
  chip.cores[1].enabled=true;
  let out='';
  chip.uart[0].onTX=b=>out+=String.fromCharCode(b);
  for(let i=0;i<2000000;i++)chip.step();
  const has = out.includes('WIFI TEST');
  console.log(`[${label}] ${out.length} chars, has WIFI TEST: ${has}`);
  if (!has && out.length > 0) console.log('  output:', out.substring(0, 500).replace(/\n/g, '\\n'));
}

// Method A: test-wifi.mjs style (flash.set, MMU after reset)
runTest('A: flash.set, MMU after reset', (chip, fw) => {
  chip.loadROM(rom);
  chip.reset();
  chip.flash.set(fw);
  chip.gpio.strapValue = 0x13;
  const pg = Math.ceil(fw.length/65536);
  for(let p=0;p<pg;p++){chip.mmuTablePro[p]=p;chip.mmuTableApp[p]=p;}
  chip.cores[0].writeUint32(0x3ff5a104,0x5aa5);
});

// Method B: SAB flash replacement, MMU before reset
runTest('B: SAB flash, MMU before reset', (chip, fw) => {
  const flash = new Uint8Array(new SharedArrayBuffer(4*1024*1024));
  flash.set(fw);
  chip.flash = new Uint8Array(flash.buffer, flash.byteOffset, chip.flash.length);
  chip.loadROM(rom);
  for(let p=0;p<64;p++){chip.mmuTablePro[p]=p;chip.mmuTableApp[p]=p;}
  chip.gpio.strapValue=0x13;
  chip.reset();
  chip.cores[0].writeUint32(0x3ff5a104,0x5aa5);
});

// Method C: SAB flash replacement, MMU after reset
runTest('C: SAB flash, MMU after reset', (chip, fw) => {
  const flash = new Uint8Array(new SharedArrayBuffer(4*1024*1024));
  flash.set(fw);
  chip.flash = new Uint8Array(flash.buffer, flash.byteOffset, chip.flash.length);
  chip.loadROM(rom);
  chip.gpio.strapValue=0x13;
  chip.reset();
  for(let p=0;p<64;p++){chip.mmuTablePro[p]=p;chip.mmuTableApp[p]=p;}
  chip.cores[0].writeUint32(0x3ff5a104,0x5aa5);
});

// Method D: flash.set (not SAB), MMU before reset
runTest('D: flash.set, MMU before reset', (chip, fw) => {
  chip.loadROM(rom);
  for(let p=0;p<64;p++){chip.mmuTablePro[p]=p;chip.mmuTableApp[p]=p;}
  chip.gpio.strapValue=0x13;
  chip.reset();
  chip.flash.set(fw);
  chip.cores[0].writeUint32(0x3ff5a104,0x5aa5);
});

// Method E: No wifi at all (to test if wifi causes the hang)
runTest('E: no wifi.enabled', (chip, fw) => {
  chip.loadROM(rom);
  chip.gpio.strapValue=0x13;
  chip.reset();
  chip.flash.set(fw);
  for(let p=0;p<64;p++){chip.mmuTablePro[p]=p;chip.mmuTableApp[p]=p;}
  chip.cores[0].writeUint32(0x3ff5a104,0x5aa5);
  chip.wifi.enabled=false; chip.wifi.rxEnabled=false;
});

process.exit(0);
