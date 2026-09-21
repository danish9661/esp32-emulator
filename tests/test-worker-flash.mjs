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
#include <esp_partition.h>
#include <esp_flash.h>
void setup() {
  Serial.begin(115200);
  Serial.println("=== FLASH TEST ===\\n");
  bool pass = true;
  const esp_partition_t *part = esp_partition_find_first(ESP_PARTITION_TYPE_DATA, ESP_PARTITION_SUBTYPE_ANY, NULL);
  if (part) {
    Serial.print("[FLASH] partition="); Serial.println(part->label);
    Serial.print("[FLASH] addr=0x"); Serial.println(part->address, HEX);
    Serial.print("[FLASH] size=0x"); Serial.println(part->size, HEX);
  } else {
    Serial.println("[FLASH] no partition found");
    pass = false;
  }
  uint32_t buf[4] = {0};
  esp_err_t err = esp_flash_read(NULL, buf, 0, 16);
  Serial.print("[FLASH] read="); Serial.println(err == ESP_OK ? "PASS" : "FAIL");
  if (err != ESP_OK) { Serial.print(" err=0x"); Serial.println(err, HEX); pass = false; }
  else {
    Serial.print("[FLASH] data=");
    for (int i = 0; i < 4; i++) { Serial.print(buf[i], HEX); Serial.print(" "); }
    Serial.println();
  }
  // Test erase/write
  err = esp_flash_erase_region(NULL, 0x3F0000, 0x1000);
  Serial.print("[FLASH] erase="); Serial.println(err == ESP_OK ? "PASS" : "FAIL");
  if (err != ESP_OK) { Serial.print(" err=0x"); Serial.println(err, HEX); pass = false; }
  if (err == ESP_OK) {
    uint32_t wdata = 0xDEADBEEF;
    err = esp_flash_write(NULL, &wdata, 0x3F0000, 4);
    Serial.print("[FLASH] write="); Serial.println(err == ESP_OK ? "PASS" : "FAIL");
    if (err != ESP_OK) { Serial.print(" err=0x"); Serial.println(err, HEX); pass = false; }
    if (err == ESP_OK) {
      uint32_t rdata = 0;
      esp_flash_read(NULL, &rdata, 0x3F0000, 4);
      pass = pass && (rdata == 0xDEADBEEF);
      Serial.print("[FLASH] verify="); Serial.println(rdata == 0xDEADBEEF ? "PASS" : "FAIL");
      Serial.print(" readback=0x"); Serial.println(rdata, HEX);
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
  await proxy.init('ESP32', { flashSizeMB: 4, mmuPages: 64, strapValue: 0x13, budget: 8000000 }, flash, rom);
  proxy.run();
  for (let i = 0; i < 150; i++) {
    await new Promise(r => setTimeout(r, 200));
    proxy.pollUart();
    if (proxy.nanos > 80000000 && out.includes('ALL TESTS PASSED')) break;
  }
  proxy.stop();
  await new Promise(r => setTimeout(r, 100));
  proxy.terminate();
  console.log(out);
  if (!out.includes('RESULT=PASS') || !out.includes('ALL TESTS PASSED')) { console.log('[test] FAILED'); process.exit(1); }
  console.log('[test] PASSED');
}
run().catch(e => { console.error(e); process.exit(1); });
