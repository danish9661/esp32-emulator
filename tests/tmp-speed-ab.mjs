// Same-run A/B: two workers side by side (JIT on vs off), identical
// firmware, thermal-identical. Compares per-chip MIPS + UART equality.
import { SimulatorWorker } from '../src/index.js';
import axios from 'axios';
import { createHash } from 'crypto';
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
console.log('[AB] compiling...');
const bin = await compile(FW);
const INST = 2813.8e6; // same sketch as tmp-speed (retired-count parity)
async function launch(jitEnabled, tag) {
  const flash = new Uint8Array(new SharedArrayBuffer(4 * 1024 * 1024));
  flash.fill(0xff);
  flash.set(new Uint8Array(bin.buffer, bin.byteOffset, bin.byteLength));
  const proxy = new SimulatorWorker();
  let out = '';
  proxy._onUART = (b) => { out += String.fromCharCode(b); };
  proxy._onError = (e) => console.error(`[worker:${tag}] Error:`, e.message);
  await proxy.init('ESP32', { flashSizeMB: 4, mmuPages: 64, strapValue: 0x13, budget: 5000000, jitEnabled }, flash, ROM);
  return { proxy, tag, getOut: () => out, done: false, tDone: 0 };
}
const a = await launch(true, 'jitON');
const b = await launch(false, 'jitOFF');
const t0 = Date.now();
a.proxy.run();
b.proxy.run();
while (true) {
  await sleep(500);
  for (const p of [a, b]) {
    p.proxy.pollUart();
    if (!p.done && p.getOut().includes('SPEEDDONE')) { p.done = true; p.tDone = (Date.now() - t0) / 1000; }
  }
  if (a.done && b.done) break;
  if (Date.now() - t0 > 900000) {
    console.log('[AB] TIMEOUT');
    console.log('jitON tail:', a.getOut().slice(-300));
    console.log('jitOFF tail:', b.getOut().slice(-300));
    throw new Error('AB timeout');
  }
}
const mipsA = INST / a.tDone / 1e6, mipsB = INST / b.tDone / 1e6;
const hA = createHash('sha256').update(a.getOut()).digest('hex').slice(0, 16);
const hB = createHash('sha256').update(b.getOut()).digest('hex').slice(0, 16);
console.log(`[AB] jitON : ${a.tDone.toFixed(1)}s = ${mipsA.toFixed(1)} MIPS uart=${hA}`);
console.log(`[AB] jitOFF: ${b.tDone.toFixed(1)}s = ${mipsB.toFixed(1)} MIPS uart=${hB}`);
console.log(`[AB] ratio=${(mipsA / mipsB).toFixed(2)}x uartEqual=${hA === hB}`);
for (const p of [a, b]) { p.proxy.stop(); await sleep(50); p.proxy.terminate(); }
if (hA !== hB) { console.log('[AB] UART DIVERGENCE — JIT UNSOUND'); process.exit(1); }
process.exit(0);
