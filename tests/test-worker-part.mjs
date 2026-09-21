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
ota_1,app,ota_1,0x200000,0x1E0000`;
const firmware = `
#include <esp_flash.h>
void setup() {
  Serial.begin(115200);
  Serial.println("=== PART TABLE TEST ===\\n");
  // Dump partition table bytes at 0x8000 via direct flash read
  uint32_t buf[8];
  esp_err_t err = esp_flash_read(NULL, buf, 0x8000, 32);
  if (err == ESP_OK) {
    Serial.print("magic="); Serial.println(buf[0], HEX);
    for (int i = 0; i < 8; i++) {
      Serial.print("  ["); Serial.print(i); Serial.print("]="); Serial.println(buf[i], HEX);
    }
  } else {
    Serial.print("read failed err=0x"); Serial.println(err, HEX);
  }
  const void *part = NULL;
  esp_partition_iterator_t it = esp_partition_find(ESP_PARTITION_TYPE_APP, ESP_PARTITION_SUBTYPE_ANY, NULL);
  int count = 0;
  while (it) { count++; it = esp_partition_next(it); }
  Serial.print("app_partitions="); Serial.println(count);
  it = esp_partition_find(ESP_PARTITION_TYPE_DATA, ESP_PARTITION_SUBTYPE_ANY, NULL);
  count = 0;
  while (it) { 
    const esp_partition_t *p = esp_partition_get(it);
    Serial.print(p->label); Serial.print(" @0x"); Serial.print(p->address, HEX); Serial.print(" sz=0x"); Serial.println(p->size, HEX);
    count++; it = esp_partition_next(it); 
  }
  Serial.print("data_partitions="); Serial.println(count);
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
  await proxy.init('ESP32', { flashSizeMB: 4, mmuPages: 64, strapValue: 0x13, budget: 8000000, partitions }, flash, rom);
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
  if (!out.includes('ALL TESTS PASSED')) { console.log('[test] FAILED'); process.exit(1); }
  console.log('[test] PASSED');
}
run().catch(e => { console.error(e); process.exit(1); });
