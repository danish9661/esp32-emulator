import { SimulatorWorker } from '../src/index.js';
import axios from 'axios';
import { readFileSync, writeFileSync, existsSync, unlinkSync } from 'fs';
import { resolve, dirname } from 'path';
import { fileURLToPath } from 'url';

const __dirname = dirname(fileURLToPath(import.meta.url));
const FLASH_FILE = resolve(__dirname, 'tmp-flash-persist.bin');

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
#include "nvs_flash.h"
#include "nvs.h"
#include <esp_flash.h>

void setup() {
  Serial.begin(115200);
  Serial.println("=== FLASH PERSIST TEST ===\\n");
  esp_err_t err = nvs_flash_init();
  if (err != ESP_OK) { nvs_flash_erase(); nvs_flash_init(); }
  nvs_handle_t h;
  err = nvs_open("store", NVS_READWRITE, &h);
  if (err != ESP_OK) { Serial.print("[PERSIST] nvs_open FAIL 0x"); Serial.println(err, HEX); }
  int32_t boots = 0;
  err = nvs_get_i32(h, "boots", &boots);
  if (err == ESP_ERR_NVS_NOT_FOUND) boots = 0;
  boots++;
  nvs_set_i32(h, "boots", boots);
  nvs_commit(h);
  nvs_close(h);
  Serial.print("[PERSIST] run boots="); Serial.println(boots);
  uint32_t magic = 0;
  err = esp_flash_read(NULL, &magic, 0x3F0000, 4);
  if (magic != 0xDEADBEEF) {
    esp_flash_erase_region(NULL, 0x3F0000, 0x1000);
    uint32_t w = 0xDEADBEEF;
    esp_flash_write(NULL, &w, 0x3F0000, 4);
    Serial.println("[PERSIST] magic written");
  } else {
    Serial.println("[PERSIST] magic persisted");
  }
  if (boots >= 3 && magic == 0xDEADBEEF) {
    Serial.print("RESULT="); Serial.println("PASS");
    Serial.println("\\n=== ALL TESTS PASSED ===");
  }
  Serial.println("[PERSIST] done");
}
void loop() { delay(1000); }
`;

async function runOnce(flashBytes) {
  const flash = new Uint8Array(new SharedArrayBuffer(4 * 1024 * 1024));
  flash.set(flashBytes);
  const rom = readFileSync(resolve(__dirname, '../rom/esp32-v3-rom.bin'));

  const proxy = new SimulatorWorker();
  let out = '';
  proxy._onUART = (b) => { out += String.fromCharCode(b); };
  proxy._onError = (e) => console.error('\n[test] Error:', e.message);

  await proxy.init('ESP32', { flashSizeMB: 4, mmuPages: 64, strapValue: 0x13, budget: 8000000 }, flash, rom);
  proxy.run();

  for (let i = 0; i < 150; i++) {
    await new Promise(r => setTimeout(r, 200));
    proxy.pollUart();
    if (out.includes('ALL TESTS PASSED')) break;
  }
  proxy.stop();
  await new Promise(r => setTimeout(r, 100));
  proxy.terminate();

  console.log(out);
  return { out, flashSaved: new Uint8Array(flash.buffer).slice() };
}

async function run() {
  const b64 = await compile(firmware);
  const base = new Uint8Array(Buffer.from(b64, 'base64').toString('binary').split('').map(c => c.charCodeAt(0)));

  if (existsSync(FLASH_FILE)) unlinkSync(FLASH_FILE);

  let finalOut = '';
  const boots = [];
  for (let runIdx = 0; runIdx < 3; runIdx++) {
    let bytes;
    if (existsSync(FLASH_FILE)) {
      bytes = new Uint8Array(readFileSync(FLASH_FILE));
      if (bytes.length < 4 * 1024 * 1024) {
        const full = new Uint8Array(4 * 1024 * 1024);
        full.set(bytes);
        bytes = full;
      }
      console.log(`[test] --- run ${runIdx + 1} (saved flash) ---`);
    } else {
      bytes = base;
      console.log(`[test] --- run ${runIdx + 1} (fresh flash) ---`);
    }
    const { out, flashSaved } = await runOnce(bytes);
    finalOut = out;
    const bm = out.match(/run boots=(\d+)/);
    if (bm) boots.push(Number(bm[1]));
    writeFileSync(FLASH_FILE, Buffer.from(flashSaved.buffer));
  }
  unlinkSync(FLASH_FILE);

  console.log(`[test] boots per run: ${boots.join(',')}`);
  if (!finalOut.includes('RESULT=PASS') || !finalOut.includes('ALL TESTS PASSED') || boots.length !== 3) {
    console.log('[test] FAILED');
    process.exit(1);
  }
  console.log('[test] PASSED');
}

run().catch(e => { console.error(e); process.exit(1); });