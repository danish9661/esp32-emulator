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
const partitions = `\
nvs,data,nvs,0x9000,0x5000
otadata,data,ota,0xe000,0x2000
ota_0,app,ota_0,0x10000,0x1E0000
ota_1,app,ota_1,0x200000,0x1E0000
spiffs,data,spiffs,0x3E0000,0x20000`;
const firmware = `
#include <esp_ota_ops.h>
#include <nvs_flash.h>
#include <esp_partition.h>
void setup() {
  Serial.begin(115200);
  Serial.println("=== OTA TEST ===\\n");
  esp_err_t err = nvs_flash_init();
  if (err == ESP_ERR_NVS_NO_FREE_PAGES || err == ESP_ERR_NVS_NEW_VERSION_FOUND) {
    nvs_flash_erase();
    err = nvs_flash_init();
  }
  Serial.print("[OTA] nvs="); Serial.println(err == ESP_OK ? "PASS" : "FAIL");
  bool pass = (err == ESP_OK);
  if (!pass) { Serial.print("RESULT="); Serial.println(pass ? "PASS" : "FAIL"); return; }
  const esp_partition_t *part = esp_ota_get_next_update_partition(NULL);
  pass = (part != NULL);
  Serial.print("[OTA] get_partition="); Serial.println(pass ? "PASS" : "FAIL");
  if (!pass) { Serial.print("RESULT="); Serial.println(pass ? "PASS" : "FAIL"); return; }
  esp_ota_handle_t handle;
  err = esp_ota_begin(part, OTA_SIZE_UNKNOWN, &handle);
  Serial.print("[OTA] begin="); Serial.println(err == ESP_OK ? "PASS" : "FAIL");
  if (err != ESP_OK) { pass = false; Serial.print(" err=0x"); Serial.println(err, HEX); }
  if (err == ESP_OK) {
    const esp_partition_t *running = esp_ota_get_running_partition();
    if (running != NULL) {
      // Use small stack buffer (1KB) to avoid stack overflow
      uint8_t buf[1024];
      uint32_t off = 0;
      while (off < running->size) {
        uint32_t to_read = (running->size - off) < sizeof(buf) ? (running->size - off) : sizeof(buf);
        err = esp_partition_read(running, off, buf, to_read);
        if (err != ESP_OK) { Serial.print("[OTA] read_err=0x"); Serial.println(err, HEX); break; }
        err = esp_ota_write(handle, buf, to_read);
        if (err != ESP_OK) { Serial.print("[OTA] write_err=0x"); Serial.println(err, HEX); break; }
        off += to_read;
      }
    } else {
      err = ESP_FAIL;
      Serial.println("[OTA] running=NULL");
    }
    if (err == ESP_OK) {
      err = esp_ota_end(handle);
      Serial.print("[OTA] end="); Serial.println(err == ESP_OK ? "PASS" : "FAIL");
      if (err != ESP_OK) { pass = false; Serial.print(" err=0x"); Serial.println(err, HEX); }
    } else {
      // abort OTA on error
      esp_ota_abort(handle);
    }
  }
  Serial.print("RESULT="); Serial.println(pass ? "PASS" : "FAIL");
  Serial.println("\\n=== ALL TESTS PASSED ===");
}
void loop() { delay(1000); }
`;
async function run() {
  const b64 = await compile(firmware);
  console.log('[test] compiled, flash size:', Buffer.from(b64, 'base64').length);
  const flash = new Uint8Array(new SharedArrayBuffer(4 * 1024 * 1024));
  flash.set(new Uint8Array(Buffer.from(b64, 'base64')));
  const rom = readFileSync(resolve(__dirname, '../rom/esp32-v3-rom.bin'));
  const proxy = new SimulatorWorker();
  let out = '';
  proxy._onUART = (b) => { out += String.fromCharCode(b); };
  proxy._onError = (e) => console.error('\n[test] Error:', e.message);
  await proxy.init('ESP32', { flashSizeMB: 4, mmuPages: 64, strapValue: 0x13, budget: 120000000, partitions }, flash, rom);
  proxy.run();
  for (let i = 0; i < 500; i++) {
    await new Promise(r => setTimeout(r, 400));
    proxy.pollUart();
    if (proxy.nanos > 600000000 && out.includes('ALL TESTS PASSED')) break;
  }
  proxy.stop();
  await new Promise(r => setTimeout(r, 100));
  proxy.terminate();
  console.log(out);
  if (!out.includes('RESULT=PASS') || !out.includes('ALL TESTS PASSED')) { console.log('[test] FAILED'); process.exit(1); }
  console.log('[test] PASSED');
}
run().catch(e => { console.error(e); process.exit(1); });
