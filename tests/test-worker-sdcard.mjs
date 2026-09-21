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

// Full-stack SD test through the Arduino SD_MMC API: mount (format-if-needed
// on the blank virtual card), file write/read-back, multi-block file, stat.
const firmware = `
#include "SD_MMC.h"

void setup() {
  Serial.begin(115200);
  Serial.println("=== SDCARD TEST ===");
  bool pass = true;

  bool ok = SD_MMC.begin("/sdcard", true, true);
  Serial.print("SD_BEGIN="); Serial.println(ok ? "PASS" : "FAIL");
  if (!ok) pass = false;

  uint64_t sectors = SD_MMC.numSectors();
  uint64_t cardSize = SD_MMC.cardSize();
  Serial.printf("SECTORS=%llu CARDSIZE=%llu\\n", sectors, cardSize);
  if (sectors == 0 || cardSize == 0) pass = false;

  if (ok) {
    File f = SD_MMC.open("/hello.txt", FILE_WRITE);
    if (!f) { Serial.println("OPEN_W=FAIL"); pass = false; }
    else {
      f.println("esp32emu-sd-test-12345");
      f.close();
      Serial.println("OPEN_W=PASS");
    }

    File r = SD_MMC.open("/hello.txt", FILE_READ);
    if (!r) { Serial.println("OPEN_R=FAIL"); pass = false; }
    else {
      String s = r.readString();
      r.close();
      bool hit = s.indexOf("esp32emu-sd-test-12345") >= 0;
      Serial.print("READBACK="); Serial.println(hit ? "PASS" : "FAIL");
      if (!hit) pass = false;
    }

    // Multi-block file (4KB + 100B spans 9 blocks).
    File w = SD_MMC.open("/big.bin", FILE_WRITE);
    if (!w) { Serial.println("BIG_W=FAIL"); pass = false; }
    else {
      uint32_t cksum = 0;
      for (int i = 0; i < 4196; i++) { uint8_t b = (i * 37 + 11) & 0xFF; w.write(b); cksum += b; }
      w.close();
      Serial.printf("BIG_W=PASS cksum=%u\\n", cksum);
    }
    File br = SD_MMC.open("/big.bin", FILE_READ);
    if (!br) { Serial.println("BIG_R=FAIL"); pass = false; }
    else {
      uint32_t cksum = 0, n = 0;
      while (br.available()) { cksum += br.read(); n++; }
      br.close();
      bool hit = (n == 4196 && cksum == 534714);
      Serial.printf("BIG_R n=%u cksum=%u %s\\n", n, cksum, hit ? "PASS" : "FAIL");
      if (!hit) pass = false;
    }

    File root = SD_MMC.open("/");
    int entries = 0;
    if (root) {
      while (true) { File e = root.openNextFile(); if (!e) break; entries++; e.close(); }
      root.close();
    }
    Serial.printf("LS entries=%d %s\\n", entries, entries >= 2 ? "PASS" : "FAIL");
    if (entries < 2) pass = false;
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

  for (let i = 0; i < 240; i++) {
    await new Promise(r => setTimeout(r, 500));
    proxy.pollUart();
    if (out.includes('ALL TESTS PASSED')) break;
  }
  proxy.stop();
  await new Promise(r => setTimeout(r, 100));
  proxy.terminate();

  console.log(out);
  if (!out.includes('SD_BEGIN=PASS') || !out.includes('READBACK=PASS') || !out.includes('RESULT=PASS') || !out.includes('ALL TESTS PASSED')) {
    console.log('[test] FAILED'); process.exit(1);
  }
  console.log('[test] PASSED');
}

run().catch(e => { console.error(e); process.exit(1); });
