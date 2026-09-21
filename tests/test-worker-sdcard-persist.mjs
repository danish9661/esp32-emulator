import { SimulatorWorker } from '../src/index.js';
import axios from 'axios';
import { readFileSync, writeFileSync, existsSync, unlinkSync } from 'fs';
import { resolve, dirname } from 'path';
import { fileURLToPath } from 'url';

const __dirname = dirname(fileURLToPath(import.meta.url));
const SD_FILE = resolve(__dirname, 'tmp-sd-persist.bin');

async function compile(code) {
  const r = await axios.post('http://localhost:5525/api/compile/start', { code, target: 'esp32', targetEngine: 'frontend', fqbn: 'esp32:esp32:esp32' });
  while (true) {
    const s = await axios.get(`http://localhost:5525/api/compile/status/${r.data.buildId}`);
    if (s.data.status === 'success') return s.data.binary_content;
    if (s.data.status === 'failed') throw new Error('Compile failed: ' + s.data.error);
    await new Promise(r => setTimeout(r, 1000));
  }
}

// SD image persistence across simulator restarts: the host owns the card
// image (shared "sdcard" SAB, like flash). Run 1 formats + writes count=1;
// runs 2-3 reload the saved image and increment. Proves FAT data survives.
const firmware = `
#include "SD_MMC.h"

void setup() {
  Serial.begin(115200);
  Serial.println("=== SDPERSIST TEST ===");
  bool ok = SD_MMC.begin("/sdcard", true, true);
  if (!ok) { Serial.println("SD_BEGIN=FAIL"); return; }
  int count = 0;
  File r = SD_MMC.open("/count.txt", FILE_READ);
  if (r) { String s = r.readString(); r.close(); count = s.toInt(); }
  count++;
  SD_MMC.remove("/count.txt");
  File w = SD_MMC.open("/count.txt", FILE_WRITE);
  if (w) { w.println(count); w.close(); }
  Serial.printf("[PERSIST] run count=%d\\n", count);
  if (count >= 3) {
    Serial.print("RESULT="); Serial.println("PASS");
    Serial.println("\\n=== ALL TESTS PASSED ===");
  }
  Serial.println("[PERSIST] done");
}
void loop() { delay(1000); }
`;

async function runOnce(flashBytes, sdImage) {
  const flash = new Uint8Array(new SharedArrayBuffer(4 * 1024 * 1024));
  flash.set(flashBytes);
  const rom = readFileSync(resolve(__dirname, '../rom/esp32-v3-rom.bin'));

  const proxy = new SimulatorWorker();
  let out = '';
  proxy._onUART = (b) => { out += String.fromCharCode(b); };
  proxy._onError = (e) => console.error('\n[test] Error:', e.message);

  const sdCard = sdImage ? { sizeMB: 16, image: sdImage } : { sizeMB: 16 };
  await proxy.init('ESP32', { flashSizeMB: 4, mmuPages: 64, strapValue: 0x13, budget: 8000000, sdCard }, flash, rom);
  proxy.run();

  for (let i = 0; i < 150; i++) {
    await new Promise(r => setTimeout(r, 200));
    proxy.pollUart();
    if (out.includes('ALL TESTS PASSED')) break;
  }
  proxy.stop();
  const sdSaved = proxy.memory.sdcard ? new Uint8Array(proxy.memory.sdcard).slice() : null;
  await new Promise(r => setTimeout(r, 100));
  proxy.terminate();

  console.log(out);
  return { out, sdSaved };
}

async function run() {
  const b64 = await compile(firmware);
  const base = new Uint8Array(Buffer.from(b64, 'base64').toString('binary').split('').map(c => c.charCodeAt(0)));

  if (existsSync(SD_FILE)) unlinkSync(SD_FILE);

  let finalOut = '';
  const counts = [];
  for (let runIdx = 0; runIdx < 3; runIdx++) {
    let image = null;
    if (existsSync(SD_FILE)) {
      image = new Uint8Array(readFileSync(SD_FILE));
      console.log(`[test] --- run ${runIdx + 1} (saved image) ---`);
    } else {
      console.log(`[test] --- run ${runIdx + 1} (fresh card) ---`);
    }
    const { out, sdSaved } = await runOnce(base, image);
    finalOut = out;
    const m = out.match(/run count=(\d+)/);
    if (m) counts.push(Number(m[1]));
    if (sdSaved) writeFileSync(SD_FILE, Buffer.from(sdSaved.buffer));
  }
  unlinkSync(SD_FILE);

  console.log(`[test] counts per run: ${counts.join(',')}`);
  if (!finalOut.includes('RESULT=PASS') || !finalOut.includes('ALL TESTS PASSED') || counts.join(',') !== '1,2,3') {
    console.log('[test] FAILED');
    process.exit(1);
  }
  console.log('[test] PASSED');
}

run().catch(e => { console.error(e); process.exit(1); });
