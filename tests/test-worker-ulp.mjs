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
#include "esp32/ulp.h"
enum { ULP_MAGIC_ADDR = 16 };
const ulp_insn_t prog[] = {
  I_MOVI(R1, ULP_MAGIC_ADDR),
  I_MOVI(R0, 0xBEEF),
  I_ST(R0, R1, 0),
  I_MOVI(R0, 100),
  I_MOVI(R2, 23),
  I_ADDR(R0, R0, R2),
  I_ST(R0, R1, 1),
  I_LD(R3, R1, 0),
  I_ST(R3, R1, 2),
  I_BXI(11),
  I_MOVI(R0, 0xDEAD),
  I_ST(R0, R1, 3),
  I_HALT(),
};
void setup() {
  Serial.begin(115200);
  Serial.println("=== ULP TEST ===");
  bool pass = true;
  volatile uint32_t *slow = (volatile uint32_t *)0x50000000;
  size_t psize = sizeof(prog) / sizeof(ulp_insn_t);
  esp_err_t err = ulp_process_macros_and_load(0, prog, &psize);
  Serial.print("ULP_LOAD="); Serial.println(err == ESP_OK ? "PASS" : "FAIL");
  if (err != ESP_OK) pass = false;
  err = ulp_run(0);
  Serial.print("ULP_RUN="); Serial.println(err == ESP_OK ? "PASS" : "FAIL");
  if (err != ESP_OK) pass = false;
  uint32_t m0 = slow[ULP_MAGIC_ADDR];
  uint32_t m1 = slow[ULP_MAGIC_ADDR + 1];
  uint32_t m2 = slow[ULP_MAGIC_ADDR + 2];
  uint32_t m3 = slow[ULP_MAGIC_ADDR + 3];
  Serial.printf("ULP_MEM=%x %x %x %x\\n", (unsigned)m0, (unsigned)m1, (unsigned)m2, (unsigned)m3);
  if (m0 != 0xBEEF || m1 != 123 || m2 != 0xBEEF || m3 != 123) { Serial.println("ULP_DATA=FAIL"); pass = false; }
  else Serial.println("ULP_DATA=PASS");
  Serial.print("RESULT="); Serial.println(pass ? "PASS" : "FAIL");
  Serial.println("\\n=== ALL TESTS PASSED ===");
}
void loop() { delay(1000); }
`;async function run() {
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
  if (!out.includes('RESULT=PASS') || !out.includes('ULP_LOAD=PASS') || !out.includes('ULP_RUN=PASS') || !out.includes('ULP_DATA=PASS') || !out.includes('ALL TESTS PASSED')) {
    console.log('[test] FAILED'); process.exit(1);
  }
  console.log('[test] PASSED');
}

run().catch(e => { console.error(e); process.exit(1); });
