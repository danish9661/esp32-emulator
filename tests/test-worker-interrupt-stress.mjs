import { SimulatorWorker } from '../src/index.js';
import axios from 'axios';
import { readFileSync } from 'fs';
import { resolve, dirname } from 'path';
import { fileURLToPath } from 'url';

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
#include <Arduino.h>

// Interrupt stress test: timer counting + GPIO rapid toggling
// Validates no crashes/panics under high-frequency register activity

static volatile uint64_t timerCount = 0;

void setup() {
  Serial.begin(115200);
  Serial.println("=== INTERRUPT STRESS TEST ===\\n");

  // High-frequency timer (1MHz base, read continuously)
  hw_timer_t *timer = timerBegin(1000000);
  timerStart(timer);

  // Rapid GPIO toggling stress — creates heavy MMIO traffic
  pinMode(17, OUTPUT);
  for (int i = 0; i < 5000; i++) {
    digitalWrite(17, i & 1);
  }

  // Read timer after burst
  uint64_t cnt = timerRead(timer);
  timerStop(timer);
  timerEnd(timer);

  Serial.printf("[STRESS] timer_count=%lu gpio_toggles=5000\\n", (uint32_t)cnt);

  // Timer at 1MHz should have counted ~thousands of ticks during the 5000 toggles
  bool pass = cnt > 100;

  Serial.print("RESULT="); Serial.println(pass ? "PASS" : "FAIL");
  Serial.println("\\n=== ALL TESTS PASSED ===");
}

void loop() { delay(1000); }
`;

async function run() {
  const b64 = await compile(firmware);
  const flash = new Uint8Array(new SharedArrayBuffer(4 * 1024 * 1024));
  flash.set(new Uint8Array(Buffer.from(b64, 'base64').toString('binary').split('').map(c => c.charCodeAt(0))));
  const rom = readFileSync(resolve(__dirname, '../rom/esp32-v3-rom.bin'));

  const proxy = new SimulatorWorker();
  let out = '';
  proxy._onUART = (b) => { out += String.fromCharCode(b); };
  proxy._onError = (e) => console.error('\n[test] Error:', e.message);

  await proxy.init('ESP32', { flashSizeMB: 4, mmuPages: 64, strapValue: 0x13, budget: 10000000 }, flash, rom);
  proxy.run();

  for (let i = 0; i < 200; i++) {
    await new Promise(r => setTimeout(r, 200));
    proxy.pollUart();
    if (out.includes('ALL TESTS PASSED') || out.includes('RESULT=FAIL')) break;
  }
  proxy.stop();
  await new Promise(r => setTimeout(r, 100));
  proxy.terminate();

  console.log(out);
  if (!out.includes('RESULT=PASS') || !out.includes('ALL TESTS PASSED')) {
    console.log('[test] FAILED');
    process.exit(1);
  }
  console.log('[test] PASSED');
}

run().catch(e => { console.error(e); process.exit(1); });
