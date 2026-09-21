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

// 4-bit SD mode: mount (format-if-needed), single file write/read-back.
// Exercises ACMD6 4-bit switch + wide-bus block transfers.
const firmware = `
#include "SD_MMC.h"

void setup() {
  Serial.begin(115200);
  Serial.println("=== SD4BIT TEST ===");
  bool pass = true;

  bool ok = SD_MMC.begin("/sdcard", false, true);
  Serial.print("SD_BEGIN4="); Serial.println(ok ? "PASS" : "FAIL");
  if (!ok) pass = false;

  if (ok) {
    File f = SD_MMC.open("/wide.txt", FILE_WRITE);
    if (!f) { Serial.println("OPEN_W=FAIL"); pass = false; }
    else { f.println("wide-bus-4bit-ok"); f.close(); Serial.println("OPEN_W=PASS"); }

    File r = SD_MMC.open("/wide.txt", FILE_READ);
    if (!r) { Serial.println("OPEN_R=FAIL"); pass = false; }
    else {
      String s = r.readString();
      r.close();
      bool hit = s.indexOf("wide-bus-4bit-ok") >= 0;
      Serial.print("READBACK="); Serial.println(hit ? "PASS" : "FAIL");
      if (!hit) pass = false;
    }
  }

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

  await proxy.init('ESP32', { flashSizeMB: 4, mmuPages: 64, strapValue: 0x13, budget: 5000000, sdCard: { sizeMB: 16 } }, flash, rom);
  proxy.run();

  for (let i = 0; i < 120; i++) {
    await new Promise(r => setTimeout(r, 500));
    proxy.pollUart();
    if (out.includes('ALL TESTS PASSED')) break;
  }
  proxy.stop();
  await new Promise(r => setTimeout(r, 100));
  proxy.terminate();

  console.log(out);
  if (!out.includes('SD_BEGIN4=PASS') || !out.includes('READBACK=PASS') || !out.includes('RESULT=PASS')) {
    console.log('[test] FAILED'); process.exit(1);
  }
  console.log('[test] PASSED');
}

run().catch(e => { console.error(e); process.exit(1); });
