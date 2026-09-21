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

// Host-side analog inputs: pin -> volts.
// 36 = ADC1_CH0 (SENSOR_VP), 32 = ADC1_CH4, 33 = ADC1_CH5, 34 = ADC1_CH6.
// Expected (12-bit, 11dB default full-scale 3.3V, 6dB full-scale 2.0V):
//   a = 1.65V / 3.3V * 4095 ~ 2048
//   b = 0V                      ~ 0
//   c = 3.3V / 3.3V * 4095      ~ 4095
//   d = 1.65V / 2.0V * 4095     ~ 3382  (attenuation math)
// NOTE: analogSetPinAttenuation before the first read is a no-op — the
// Arduino core only reconfigures channels already claimed by a prior
// analogRead (real-HW parity). So: read 34 once (11dB, ~2048), set 6dB,
// read again (~3382).
const firmware = `
void setup() {
  Serial.begin(115200);
  Serial.println("=== ANALOG TEST ===\\n");
  analogReadResolution(12);
  int a = analogRead(36);
  int b = analogRead(32);
  int c = analogRead(33);
  int d0 = analogRead(34);
  analogSetPinAttenuation(34, ADC_6db);
  int d = analogRead(34);
  Serial.printf("[ANALOG] a=%d b=%d c=%d d0=%d d=%d\\n", a, b, c, d0, d);
  bool ok = (a >= 1900 && a <= 2200) && (b < 100) && (c >= 3990) && (d0 >= 1900 && d0 <= 2200) && (d >= 3280 && d <= 3480);
  Serial.print("RESULT="); Serial.println(ok ? "PASS" : "FAIL");
  if (ok) Serial.println("\\n=== ALL TESTS PASSED ===");
  else Serial.println("\\n=== TEST FAILED ===");
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

  await proxy.init('ESP32', {
    flashSizeMB: 4, mmuPages: 64, strapValue: 0x13, budget: 5000000,
    analogInputs: { 36: 1.65, 32: 0.0, 33: 3.3, 34: 1.65 },
  }, flash, rom);
  proxy.run();

  for (let i = 0; i < 100; i++) {
    await new Promise(r => setTimeout(r, 200));
    proxy.pollUart();
    if (proxy.nanos > 50000000 && out.includes('ALL TESTS PASSED')) break;
  }
  proxy.stop();
  await new Promise(r => setTimeout(r, 100));
  proxy.terminate();

  console.log(out);
  if (!out.includes('RESULT=PASS') || !out.includes('ALL TESTS PASSED')) {
    console.log('[test] FAILED'); process.exit(1);
  }
  console.log('[test] PASSED');
}

run().catch(e => { console.error(e); process.exit(1); });