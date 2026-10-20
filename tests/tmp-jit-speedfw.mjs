// JIT on the tmp-speed firmware (fresh compile): boot, spin 60s, stats.
import { SimulatorWorker } from '../src/index.js';
import axios from 'axios';
import { readFileSync } from 'fs';
import { resolve, dirname } from 'path';
import { fileURLToPath } from 'url';
const __dirname = dirname(fileURLToPath(import.meta.url));
const ROM = readFileSync(resolve(__dirname, 'rom/esp32-v3-rom.bin'));
const FW = `
volatile uint32_t acc0 = 0, acc1 = 0;
volatile bool c0done = false;
void spin0(void*) {
  uint32_t a = 0x12345678;
  for (int i = 0; i < 100000000; i++) {
    a = a * 1103515245u + 12345u;
    if ((i % 1000000) == 0) delay(1);
  }
  acc0 = a; c0done = true;
  vTaskDelete(NULL);
}
void setup() {
  Serial.begin(115200);
  Serial.println("SPEEDGO");
  xTaskCreatePinnedToCore(spin0, "spin0", 4096, NULL, 1, NULL, 0);
  uint32_t a = 0x87654321;
  for (int i = 0; i < 100000000; i++) {
    a = a * 1103515245u + 12345u;
    if ((i % 1000000) == 0) delay(1);
  }
  acc1 = a;
  while (!c0done) { delay(1); }
  Serial.printf("ACC=%08x%08x\\n", (unsigned)acc0, (unsigned)acc1);
  Serial.println("SPEEDDONE");
}
void loop() { delay(1000); }
`;
async function compile(code) {
  const r = await axios.post('http://localhost:5525/api/compile/start', { code, target: 'esp32', targetEngine: 'frontend', fqbn: 'esp32:esp32:esp32' });
  while (true) {
    const s = await axios.get(`http://localhost:5525/api/compile/status/${r.data.buildId}`);
    if (s.data.status === 'success') return Buffer.from(s.data.binary_content, 'base64');
    if (s.data.status === 'failed') throw new Error('Compile failed: ' + s.data.error);
    await new Promise((r) => setTimeout(r, 1000));
  }
}
const sleep = (ms) => new Promise((r) => setTimeout(r, ms));
console.log('[jfw] compiling...');
const bin = await compile(FW);
import('fs').then(m => m.writeFileSync('/tmp/opencode/speedfw-fresh.bin', Buffer.from(bin.buffer, bin.byteOffset, bin.byteLength)));
const flash = new Uint8Array(new SharedArrayBuffer(4 * 1024 * 1024));
flash.fill(0xff);
flash.set(new Uint8Array(bin.buffer, bin.byteOffset, bin.byteLength));
const proxy = new SimulatorWorker();
let out = '';
proxy._onUART = (b) => { out += String.fromCharCode(b); };
proxy._onError = (e) => console.error('[worker] Error:', e.message);
await proxy.init('ESP32', { flashSizeMB: 4, mmuPages: 64, strapValue: 0x13, budget: 5000000 }, flash, ROM);
proxy.run();
const t0 = Date.now();
while (!out.includes('SPEEDGO')) {
  await sleep(500);
  proxy.pollUart();
  if (Date.now() - t0 > 420000) throw new Error('no SPEEDGO; tail=' + out.slice(-400));
}
console.log('[jfw] SPEEDGO; spinning 75s...');
await sleep(75000);
proxy.stop();
await sleep(50);
proxy.terminate();
console.log('[jfw] done');
process.exit(0);
