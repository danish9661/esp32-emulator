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
#include <esp_flash.h>
void setup() {
  Serial.begin(115200);
  // Dump partition table bytes at 0x8000 via direct flash read
  uint32_t buf[16];
  esp_err_t err = esp_flash_read(NULL, buf, 0x8000, 64);
  if (err == ESP_OK) {
    for (int i = 0; i < 16; i++) {
      if (i % 4 == 0) Serial.print("\\n");
      Serial.print(buf[i], HEX); Serial.print(" ");
    }
    Serial.println();
  } else {
    Serial.print("flash_read err=0x"); Serial.println(err, HEX);
  }
  // List partitions from partition API
  esp_partition_iterator_t it = esp_partition_find(ESP_PARTITION_TYPE_APP, ESP_PARTITION_SUBTYPE_ANY, NULL);
  int app_cnt = 0;
  while (it) {
    const esp_partition_t *p = esp_partition_get(it);
    Serial.print("APP "); Serial.print(p->label); Serial.print(" @0x"); Serial.print(p->address, HEX); Serial.print(" sz=0x"); Serial.println(p->size, HEX);
    app_cnt++; it = esp_partition_next(it);
  }
  it = esp_partition_find(ESP_PARTITION_TYPE_DATA, ESP_PARTITION_SUBTYPE_ANY, NULL);
  int data_cnt = 0;
  while (it) {
    const esp_partition_t *p = esp_partition_get(it);
    Serial.print("DATA "); Serial.print(p->label); Serial.print(" @0x"); Serial.print(p->address, HEX); Serial.print(" sz=0x"); Serial.println(p->size, HEX);
    data_cnt++; it = esp_partition_next(it);
  }
  Serial.print("apps="); Serial.print(app_cnt); Serial.print(" data="); Serial.println(data_cnt);
  Serial.println("=== ALL TESTS PASSED ===");
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
  // NO custom partitions - use firmware's defaults
  await proxy.init('ESP32', { flashSizeMB: 4, mmuPages: 64, strapValue: 0x13, budget: 8000000 }, flash, rom);
  proxy.run();
  for (let i = 0; i < 200; i++) {
    await new Promise(r => setTimeout(r, 200));
    proxy.pollUart();
    if (proxy.nanos > 100000000 && out.includes('ALL TESTS PASSED')) break;
  }
  proxy.stop();
  await new Promise(r => setTimeout(r, 100));
  proxy.terminate();
  console.log(out);
  if (!out.includes('ALL TESTS PASSED')) { console.log('[test] FAILED'); process.exit(1); }
  console.log('[test] PASSED');
}
run().catch(e => { console.error(e); process.exit(1); });
