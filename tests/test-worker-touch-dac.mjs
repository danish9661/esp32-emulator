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

// Touch + DAC through the Arduino APIs. Host drives touch pad counts via
// config.touchInputs (pad 2 = touched/low, pad 3 = untouched/high);
// dacWrite() feeds the virtual wire DAC->ADC2 so analogRead observes it.
const firmware = `
volatile bool touchFired = false;
void IRAM_ATTR onTouch2() { touchFired = true; }
volatile bool touchFired15 = false;
void IRAM_ATTR onTouch15() { touchFired15 = true; }
void setup() {
  Serial.begin(115200);
  Serial.println("=== TOUCHDAC TEST ===");
  bool pass = true;

  uint16_t t2 = touchRead(2);    // T2 = GPIO2, driven low (touched)
  uint16_t t15 = touchRead(15);  // T3 = GPIO15, default (untouched)
  Serial.printf("TOUCH2=%u TOUCH15=%u\\n", t2, t15);
  if (t2 > 500) { Serial.println("TOUCH_LOW=FAIL"); pass = false; }
  else Serial.println("TOUCH_LOW=PASS");
  if (t15 < 800) { Serial.println("TOUCH_HIGH=FAIL"); pass = false; }
  touchAttachInterrupt(2, onTouch2, 500);
  delay(800);
  Serial.print("TOUCH_IRQ="); Serial.println(touchFired ? "PASS" : "FAIL");
  if (!touchFired) pass = false;
  touchAttachInterrupt(15, onTouch15, 500);
  delay(400);
  Serial.print("TOUCH_DYN_NEG="); Serial.println(!touchFired15 ? "PASS" : "FAIL");
  if (touchFired15) pass = false;
  Serial.println("DYN_READY");
  for (int i = 0; i < 100 && !touchFired15; i++) delay(50);
  Serial.print("TOUCH_DYN="); Serial.println(touchFired15 ? "PASS" : "FAIL");
  if (!touchFired15) pass = false;
  else Serial.println("TOUCH_HIGH=PASS");

  // DAC ch0 (GPIO25) mid-scale -> ADC2 reads ~1.65V (11dB: ~2054).
  analogRead(25);
  dacWrite(25, 128);
  int a25 = analogRead(25);
  Serial.printf("DAC25=%d\\n", a25);
  if (a25 < 1700 || a25 > 2400) { Serial.println("DAC_MID=FAIL"); pass = false; }
  else Serial.println("DAC_MID=PASS");

  // DAC ch1 (GPIO26) full-scale -> ~3.3V (clamped to 4095).
  analogRead(26);
  dacWrite(26, 255);
  int a26 = analogRead(26);
  Serial.printf("DAC26=%d\\n", a26);
  if (a26 < 3900) { Serial.println("DAC_FULL=FAIL"); pass = false; }
  else Serial.println("DAC_FULL=PASS");

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

  await proxy.init('ESP32', { flashSizeMB: 4, mmuPages: 64, strapValue: 0x13, budget: 5000000, touchInputs: { 2: 320 } }, flash, rom);
  proxy.run();

  for (let i = 0; i < 120; i++) {
    await new Promise(r => setTimeout(r, 500));
    proxy.pollUart();
    if (!globalThis.__dyn && out.includes('DYN_READY')) { globalThis.__dyn = true; await proxy.setTouchInput(3, 300); }
    if (out.includes('ALL TESTS PASSED')) break;
  }
  proxy.stop();
  await new Promise(r => setTimeout(r, 100));
  proxy.terminate();

  console.log(out);
  if (!out.includes('TOUCH_LOW=PASS') || !out.includes('TOUCH_IRQ=PASS') || !out.includes('TOUCH_DYN_NEG=PASS') || !out.includes('TOUCH_DYN=PASS') || !out.includes('TOUCH_HIGH=PASS') || !out.includes('DAC_MID=PASS') || !out.includes('DAC_FULL=PASS') || !out.includes('RESULT=PASS')) {
    console.log('[test] FAILED'); process.exit(1);
  }
  console.log('[test] PASSED');
}

run().catch(e => { console.error(e); process.exit(1); });
