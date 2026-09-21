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
#include <soc/soc.h>
#define DS_BASE 0x3ff1a000
void setup() {
  Serial.begin(115200);
  Serial.println("=== DS TEST ===");
  bool pass = true;
  // DS C_MEM area (TRM: params start at +0x0)
  REG_WRITE(DS_BASE + 0x0, 0x12345678);
  uint32_t v = REG_READ(DS_BASE + 0x0);
  Serial.printf("DS_C0=%x\\n", v);
  if (v != 0x12345678) pass = false;
  REG_WRITE(DS_BASE + 0x4, 0xDEADBEEF);
  v = REG_READ(DS_BASE + 0x4);
  if (v != 0xDEADBEEF) pass = false;
  REG_WRITE(DS_BASE + 0x100, 0xA5A5A5A5);
  v = REG_READ(DS_BASE + 0x100);
  Serial.printf("DS_100=%x\\n", v);
  if (v != 0xA5A5A5A5) pass = false;
  REG_WRITE(DS_BASE + 0x800, 0x0BADF00D);
  v = REG_READ(DS_BASE + 0x800);
  if (v != 0x0BADF00D) pass = false;
  // unwritten words read 0 (zeroed reset, stub)
  v = REG_READ(DS_BASE + 0x8);
  Serial.printf("DS_ZERO=%x\\n", v);
  if (v != 0) pass = false;
  v = REG_READ(DS_BASE + 0xFFC);
  if (v != 0) pass = false;
  Serial.print("DS_RW="); Serial.println(pass ? "PASS" : "FAIL");
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

  await proxy.init('ESP32', { flashSizeMB: 4, mmuPages: 64, strapValue: 0x13, budget: 5000000 }, flash, rom);
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
  if (!out.includes('RESULT=PASS') || !out.includes('DS_RW=PASS') || !out.includes('ALL TESTS PASSED')) {
    console.log('[test] FAILED'); process.exit(1);
  }
  console.log('[test] PASSED');
}

run().catch(e => { console.error(e); process.exit(1); });
