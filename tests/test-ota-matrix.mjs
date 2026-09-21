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
      esp_ota_abort(handle);
    }
  }
  Serial.print("RESULT="); Serial.println(pass ? "PASS" : "FAIL");
  Serial.println("\\n=== ALL TESTS PASSED ===");
}
void loop() { delay(1000); }
`;

const partitions = `\
nvs,data,nvs,0x9000,0x5000
otadata,data,ota,0xe000,0x2000
ota_0,app,ota_0,0x10000,0x1E0000
ota_1,app,ota_1,0x200000,0x1E0000
spiffs,data,spiffs,0x3E0000,0x20000`;

const matrix = [];
for (const flashMB of [4, 8, 16]) {
  for (const psramMB of [0, 4, 8]) {
    const types = psramMB === 0 ? ['n/a'] : ['quad', 'octal'];
    for (const psramType of types) {
      matrix.push({ flashMB, psramMB, psramType });
    }
  }
}

console.log(`Testing ${matrix.length} configurations...\n`);

const b64 = await compile(firmware);
const bin = Buffer.from(b64, 'base64');
const rom = readFileSync(resolve(__dirname, '../rom/esp32-v3-rom.bin'));

let passed = 0, failed = 0;
for (const cfg of matrix) {
  const flashBytes = cfg.flashMB * 1024 * 1024;
  const mmuPages = Math.ceil(flashBytes / 65536);
  const psramLabel = cfg.psramMB === 0 ? 'no' : `${cfg.psramMB}MB`;
  const label = `flash=${cfg.flashMB}MB psram=${psramLabel} type=${cfg.psramType}`;
  process.stdout.write(`  ${label} ... `);

  const flash = new Uint8Array(new SharedArrayBuffer(flashBytes));
  flash.set(new Uint8Array(bin));
  const proxy = new SimulatorWorker();
  let out = '';
  proxy._onUART = (b) => { out += String.fromCharCode(b); };
  try {
    await proxy.init('ESP32', {
      flashSizeMB: cfg.flashMB,
      psramSizeMB: cfg.psramMB,
      psramType: cfg.psramType,
      mmuPages,
      strapValue: 0x13,
      budget: 120000000,
      partitions,
    }, flash, rom);
    proxy.run();
    for (let i = 0; i < 500; i++) {
      await new Promise(r => setTimeout(r, 400));
      proxy.pollUart();
      if (proxy.nanos > 600000000 && out.includes('ALL TESTS PASSED')) break;
    }
    proxy.stop();
    await new Promise(r => setTimeout(r, 100));
    proxy.terminate();
    if (out.includes('RESULT=PASS') && out.includes('ALL TESTS PASSED')) {
      console.log('PASS');
      passed++;
    } else {
      console.log('FAIL');
      console.log(`  output: ${out.slice(0, 500).replace(/\n/g, '\\n')}`);
      failed++;
    }
  } catch (e) {
    console.log(`ERROR: ${e.message}`);
    failed++;
    proxy.terminate?.();
  }
}

console.log(`\nResults: ${passed} passed, ${failed} failed out of ${matrix.length}`);
if (failed > 0) process.exit(1);
