import { SimulatorWorker } from '../src/index.js';
import axios from 'axios';
import { readFileSync } from 'fs';
import { resolve, dirname } from 'path';
import { fileURLToPath } from 'url';

const __dirname = dirname(fileURLToPath(import.meta.url));
const rom = readFileSync(resolve(__dirname, '../rom/esp32-v3-rom.bin'));

async function compile(code) {
  const r = await axios.post('http://localhost:5525/api/compile/start', { code, target: 'esp32', targetEngine: 'frontend', fqbn: 'esp32:esp32:esp32' });
  while (true) {
    const s = await axios.get(`http://localhost:5525/api/compile/status/${r.data.buildId}`);
    if (s.data.status === 'success') return s.data.binary_content;
    if (s.data.status === 'failed') throw new Error('Compile failed: ' + s.data.error);
    await new Promise(r => setTimeout(r, 1000));
  }
}

const firmwareCode = `
void setup() {
  Serial.begin(115200);
  Serial.println("HELLO");
  delay(10);
  unsigned long t0 = millis();
  hw_timer_t *timer = timerBegin(1000000);
  timerStart(timer);
  for (int i = 0; i < 5; i++) {
    delay(100);
    uint64_t cnt = timerRead(timer);
    Serial.print("T");
    Serial.print(millis());
    Serial.print(" cnt=");
    Serial.print((uint32_t)cnt);
    Serial.print(" diff=");
    Serial.print((uint32_t)(cnt - (i * 100000)));
    Serial.println();
  }
  timerStop(timer);
  timerEnd(timer);
  Serial.println("RESULT=PASS");
}
void loop() {}
`;

async function runTest(label, cpuFreq) {
  console.log(`\n=== ${label} ===`);
  const b64 = await compile(firmwareCode);
  const buf = Buffer.from(b64, 'base64');
  const flash = new Uint8Array(new SharedArrayBuffer(4 * 1024 * 1024));
  flash.set(new Uint8Array(buf));

  const proxy = new SimulatorWorker();
  let allUart = '';
  proxy._onUART = (b) => { allUart += String.fromCharCode(b); };

  await proxy.init('ESP32', {
    flashSizeMB: 4, mmuPages: 64, strapValue: 0x13, budget: 80000000,
    wifi: false, cpuFrequency: cpuFreq,
  }, flash, rom);

  const t0 = Date.now();
  proxy.run();

  for (let i = 0; i < 600; i++) {
    await new Promise(r => setTimeout(r, 100));
    proxy.pollUart();
    if (allUart.includes('RESULT=PASS')) break;
  }
  const dt = Date.now() - t0;
  proxy.stop();
  proxy.terminate();

  const lines = allUart.split('\n').filter(l => l.trim());
  const timerLines = lines.filter(l => l.startsWith('T'));
  console.log(`  wall=${dt}ms  nanos=${proxy.nanos}`);
  for (const l of timerLines) console.log(`  ${l}`);

  // Check timer scaling: extract timestamps
  const ts = timerLines.map(l => parseInt(l.match(/T(\d+)/)?.[1] || '0'));
  if (ts.length >= 2) {
    const intervals = [];
    for (let i = 1; i < ts.length; i++) intervals.push(ts[i] - ts[i-1]);
    console.log(`  intervals(ms): ${intervals.join(', ')} (expected ~100)`);
  }
  return { wall: dt, nanos: proxy.nanos, timerLines: ts };
}

console.log('=== CPU Frequency vs Timer Timing (Worker) ===\n');
console.log('Timer at 1MHz tick rate, delay(100) between prints.\n');
console.log('If frequency affects timer: intervals would scale inversely with freq.\n');
console.log('If frequency only scales nanos: intervals stay ~100ms regardless.\n');

const r0 = await runTest('160MHz (max) ', 'max');
const r1 = await runTest('80MHz         ', 80);
const r2 = await runTest('8MHz (auto)   ', 'auto');

console.log('\n--- Summary ---');
const all = [r0, r1, r2];
for (const r of all) {
  const avg = r.timerLines.length > 1 ? (r.timerLines[r.timerLines.length-1] - r.timerLines[0]) / (r.timerLines.length - 1) : 0;
  console.log(`  wall=${r.wall}ms nanos=${r.nanos} avg_interval=${avg.toFixed(0)}ms`);
}
process.exit(0);
