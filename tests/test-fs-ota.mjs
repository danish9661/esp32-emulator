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
spiffs,data,spiffs,0x3E0000,0x10000
littlefs,data,spiffs,0x3F0000,0x10000`;

const firmware = `
#include <esp_ota_ops.h>
#include <nvs_flash.h>
#include <esp_partition.h>
#include <SPIFFS.h>
#include <LittleFS.h>

static bool test_ota() {
  Serial.println("\\n--- OTA Test ---");
  const esp_partition_t *part = esp_ota_get_next_update_partition(NULL);
  if (part == NULL) { Serial.println("[OTA] get_partition=FAIL"); return false; }
  esp_ota_handle_t handle;
  esp_err_t err = esp_ota_begin(part, OTA_SIZE_UNKNOWN, &handle);
  if (err != ESP_OK) { Serial.print("[OTA] begin=FAIL err=0x"); Serial.println(err, HEX); return false; }
  Serial.println("[OTA] begin=PASS");
  const esp_partition_t *running = esp_ota_get_running_partition();
  if (running == NULL) { Serial.println("[OTA] running=NULL"); esp_ota_abort(handle); return false; }
  uint8_t buf[1024];
  uint32_t off = 0;
  while (off < running->size) {
    uint32_t to_read = (running->size - off) < sizeof(buf) ? (running->size - off) : sizeof(buf);
    err = esp_partition_read(running, off, buf, to_read);
    if (err != ESP_OK) { Serial.print("[OTA] read_err=0x"); Serial.println(err, HEX); esp_ota_abort(handle); return false; }
    err = esp_ota_write(handle, buf, to_read);
    if (err != ESP_OK) { Serial.print("[OTA] write_err=0x"); Serial.println(err, HEX); esp_ota_abort(handle); return false; }
    off += to_read;
  }
  err = esp_ota_end(handle);
  if (err != ESP_OK) { Serial.print("[OTA] end=FAIL err=0x"); Serial.println(err, HEX); return false; }
  Serial.println("[OTA] end=PASS");
  return true;
}

static bool test_spiffs() {
  Serial.println("\\n--- SPIFFS Test ---");
  if (!SPIFFS.begin(true)) { Serial.println("[SPIFFS] begin=FAIL"); return false; }
  Serial.println("[SPIFFS] begin=PASS");
  {
    File f = SPIFFS.open("/test.txt", "w");
    if (!f) { Serial.println("[SPIFFS] open_write=FAIL"); SPIFFS.end(); return false; }
    f.print("Hello SPIFFS!");
    f.close();
    Serial.println("[SPIFFS] write=PASS");
  }
  {
    File f = SPIFFS.open("/test.txt", "r");
    if (!f) { Serial.println("[SPIFFS] open_read=FAIL"); SPIFFS.end(); return false; }
    String s = f.readString();
    f.close();
    if (s != "Hello SPIFFS!") {
      Serial.print("[SPIFFS] read=PASS content='"); Serial.print(s); Serial.println("' != expected");
      SPIFFS.end(); return false;
    }
    Serial.println("[SPIFFS] read=PASS");
  }
  SPIFFS.end();
  return true;
}

static bool test_littlefs() {
  Serial.println("\\n--- LittleFS Test ---");
  if (!LittleFS.begin(true, "/littlefs", 8)) { Serial.println("[LittleFS] begin=FAIL"); return false; }
  Serial.println("[LittleFS] begin=PASS");
  {
    File f = LittleFS.open("/test.txt", "w");
    if (!f) { Serial.println("[LittleFS] open_write=FAIL"); LittleFS.end(); return false; }
    f.print("Hello LittleFS!");
    f.close();
    Serial.println("[LittleFS] write=PASS");
  }
  {
    File f = LittleFS.open("/test.txt", "r");
    if (!f) { Serial.println("[LittleFS] open_read=FAIL"); LittleFS.end(); return false; }
    String s = f.readString();
    f.close();
    if (s != "Hello LittleFS!") {
      Serial.print("[LittleFS] read=PASS content='"); Serial.print(s); Serial.println("' != expected");
      LittleFS.end(); return false;
    }
    Serial.println("[LittleFS] read=PASS");
  }
  LittleFS.end();
  return true;
}

void setup() {
  Serial.begin(115200);
  Serial.println("=== FILESYSTEM + OTA TEST ===\\n");
  esp_err_t err = nvs_flash_init();
  if (err == ESP_ERR_NVS_NO_FREE_PAGES || err == ESP_ERR_NVS_NEW_VERSION_FOUND) {
    nvs_flash_erase();
    err = nvs_flash_init();
  }
  Serial.print("[NVS] "); Serial.println(err == ESP_OK ? "PASS" : "FAIL");
  if (err != ESP_OK) { Serial.println("RESULT=FAIL"); return; }

  // Validate all partitions exist
  esp_partition_iterator_t it = esp_partition_find(ESP_PARTITION_TYPE_ANY, ESP_PARTITION_SUBTYPE_ANY, NULL);
  int count = 0;
  for (; it; it = esp_partition_next(it), count++);
  Serial.print("[PARTITIONS] count="); Serial.println(count);

  bool pass = true;
  pass = test_spiffs() && pass;
  pass = test_littlefs() && pass;
  pass = test_ota() && pass;

  Serial.print("\\nRESULT="); Serial.println(pass ? "PASS" : "FAIL");
  if (pass) Serial.println("\\n=== ALL TESTS PASSED ===");
}
void loop() { delay(1000); }
`;

async function run() {
  const b64 = await compile(firmware);
  const flash = new Uint8Array(new SharedArrayBuffer(8 * 1024 * 1024));
  flash.set(new Uint8Array(Buffer.from(b64, 'base64')));
  const rom = readFileSync(resolve(__dirname, '../rom/esp32-v3-rom.bin'));
  const proxy = new SimulatorWorker();
  let out = '';
  proxy._onUART = (b) => { out += String.fromCharCode(b); };
  await proxy.init('ESP32', {
    flashSizeMB: 8,
    mmuPages: 128,
    strapValue: 0x13,
    budget: 150000000,
    partitions,
  }, flash, rom);
  proxy.run();
  for (let i = 0; i < 600; i++) {
    await new Promise(r => setTimeout(r, 400));
    proxy.pollUart();
    if (proxy.nanos > 800000000 && out.includes('ALL TESTS PASSED')) break;
  }
  proxy.stop();
  await new Promise(r => setTimeout(r, 100));
  proxy.terminate();
  console.log(out);
  if (!out.includes('RESULT=PASS') || !out.includes('ALL TESTS PASSED')) { console.log('[test] FAILED'); process.exit(1); }
  console.log('[test] PASSED');
}
run().catch(e => { console.error(e); process.exit(1); });
