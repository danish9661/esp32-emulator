import { ESP32 } from './src/index.js';
import axios from 'axios';
import { readFileSync } from 'fs';
import { resolve, dirname } from 'path';
import { fileURLToPath } from 'url';

const __dirname = dirname(fileURLToPath(import.meta.url));

async function compileArduino(code, target = 'esp32') {
  console.log(`[compile] Sending code for ${target}...`);
  const startRes = await axios.post('http://localhost:5525/api/compile/start', { code, target, targetEngine: 'frontend', fqbn: `esp32:esp32:${target}` });
  const buildId = startRes.data.buildId;
  while (true) {
    const statRes = await axios.get(`http://localhost:5525/api/compile/status/${buildId}`);
    if (statRes.data.status === 'success') { console.log(`[compile] Success!`); return statRes.data.binary_content; }
    else if (statRes.data.status === 'failed') { throw new Error('Compile failed: ' + statRes.data.error); }
    process.stdout.write('.');
    await new Promise(r => setTimeout(r, 1000));
  }
}

function base64ToBytes(b64) {
  const binaryString = Buffer.from(b64, 'base64').toString('binary');
  const bytes = new Uint8Array(binaryString.length);
  for (let i = 0; i < binaryString.length; i++) bytes[i] = binaryString.charCodeAt(i);
  return bytes;
}

async function runTest(sketch, label, maxCycles = 80_000_000) {
  console.log(`\n=== ${label} ===`);
  const b64 = await compileArduino(sketch, 'esp32');
  const flashBytes = base64ToBytes(b64);
  const romBytes = readFileSync(resolve(__dirname, '../rom/esp32-v3-rom.bin'));
  const flashSizeMB = Math.max(4, Math.ceil(flashBytes.length / (1024 * 1024)));
  const flash = new Uint8Array(flashSizeMB * 1024 * 1024);
  flash.fill(0xff); flash.set(flashBytes);
  const esp32 = new ESP32({ flashSizeMB, flash });
  esp32.loadROM(romBytes);
  if (esp32.gpio?.pins) {
    if (esp32.gpio.pins[0]) esp32.gpio.pins[0].inputValue = true;
    if (esp32.gpio.pins[2]) esp32.gpio.pins[2].inputValue = false;
    if (esp32.gpio.pins[12]) esp32.gpio.pins[12].inputValue = false;
    if (esp32.gpio.pins[15]) esp32.gpio.pins[15].inputValue = false;
  }
  esp32.reset();
  esp32.flash.set(flashBytes);
  if (esp32.gpio) esp32.gpio.strapValue = 0x13;
  if (esp32.mmuTablePro) {
    const pages = Math.ceil(flashBytes.length / 65536);
    for (let p = 0; p < pages; p++) { esp32.mmuTablePro[p] = p; esp32.mmuTableApp[p] = p; }
  }
  try { esp32.cores[0].writeUint32(0x3ff5a104, 0x5aa5); } catch {}
  esp32.stopped = false;
  if (esp32.uart?.[0]) {
    const u = esp32.uart[0];
    const orig = u.txUpdated?.bind?.(u);
    if (orig) u.txUpdated = function() { orig(); if (this.txState !== 0) this.txComplete(); };
  }
  let uartLog = '', uartLines = [];
  if (esp32.uart?.[0]) {
    esp32.uart[0].onTX = (byte) => {
      uartLog += String.fromCharCode(byte);
      if (uartLog.endsWith('\n')) { const line = uartLog.trim(); if (line) uartLines.push(line); uartLog = ''; }
    };
  }
  let lastPC = -1, stuck = 0, maxPC = 0;
  for (let i = 0; i < maxCycles; i++) {
    esp32.step();
    if (esp32.clock) esp32.clock.tick?.();
    const pc = esp32.cores?.[0]?.PC;
    if (pc !== undefined) { if (pc === lastPC) stuck++; else stuck = 0; lastPC = pc; if (pc > maxPC) maxPC = pc; }
    if (i % 10_000_000 === 0 && i > 0) process.stdout.write(`\r  cycle ${(i/1e6).toFixed(0)}M  PC=0x${lastPC?.toString(16)}`);
    if (stuck > 500_000) break;
  }
  console.log(`\n  Stuck: ${stuck}, MaxPC: 0x${maxPC.toString(16)}, Lines: ${uartLines.length}`);
  uartLines.slice(-20).forEach(l => console.log(`  ${l}`));
  const appOk = maxPC >= 0x40080000;
  const hasStr = (s) => uartLines.some(l => l.includes(s));
  console.log(`  User code: ${appOk ? 'YES' : 'NO'}`);
  return { uartLines, appOk, hasStr };
}

async function run() {
  // Test 1: Just ADC
  await runTest(`
void setup() {
  Serial.begin(115200); delay(100);
  Serial.println("[ADC] Testing analogRead...");
  analogReadResolution(12);
  int v = analogRead(36);
  Serial.print("[ADC] SENSOR_VP=");
  Serial.println(v);
  v = analogRead(39);
  Serial.print("[ADC] SENSOR_VN=");
  Serial.println(v);
  Serial.println("[ADC] Done");
}
void loop() { delay(10000); }
`, "ADC Only");

  // Test 2: Just Serial2
  await runTest(`
void setup() {
  Serial.begin(115200); delay(100);
  Serial.println("[SER2] Testing Serial2...");
  Serial2.begin(115200);
  Serial2.println("Hello from Serial2!");
  Serial2.flush();
  Serial2.end();
  Serial.println("[SER2] Done");
}
void loop() { delay(10000); }
`, "Serial2 Only");

  // Test 3: Just Serial1
  await runTest(`
void setup() {
  Serial.begin(115200); delay(100);
  Serial.println("[SER1] Testing Serial1...");
  Serial1.begin(115200);
  Serial1.println("Hello from Serial1!");
  Serial1.flush();
  Serial1.end();
  Serial.println("[SER1] Done");
}
void loop() { delay(10000); }
`, "Serial1 Only");
}

run().catch(e => { console.error(e); process.exit(1); });
