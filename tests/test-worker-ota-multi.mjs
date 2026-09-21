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
#include <esp_partition.h>

void setup() {
  Serial.begin(115200);
  Serial.println("=== OTA MULTI-PARTITION TEST ===\\n");
  bool pass = true;

  const esp_partition_t *running = esp_ota_get_running_partition();
  Serial.print("[OTA] running="); Serial.println(running ? "PASS" : "FAIL");
  if (!running) pass = false;

  const esp_partition_t *next = esp_ota_get_next_update_partition(NULL);
  Serial.print("[OTA] next_update="); Serial.println(next ? "PASS" : "FAIL");
  if (!next) pass = false;

  if (running) {
    Serial.print("[OTA] running_label="); Serial.println(running->label);
    Serial.print("[OTA] running_addr=0x"); Serial.println(running->address, HEX);
    Serial.print("[OTA] running_size="); Serial.println(running->size);
  }

  if (next) {
    Serial.print("[OTA] next_label="); Serial.println(next->label);
    Serial.print("[OTA] next_addr=0x"); Serial.println(next->address, HEX);
    Serial.print("[OTA] next_size="); Serial.println(next->size);

    bool diff_part = (running && running->address != next->address);
    Serial.print("[OTA] diff_partition="); Serial.println(diff_part ? "PASS" : "FAIL");
    if (!diff_part) pass = false;

    const esp_partition_t *next2 = esp_ota_get_next_update_partition(next);
    bool cycle = (next2 && next2->address == running->address);
    Serial.print("[OTA] partition_cycle="); Serial.println(cycle ? "PASS" : "FAIL");
    if (!cycle) pass = false;
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
  await proxy.init('ESP32', { flashSizeMB: 4, mmuPages: 64, strapValue: 0x13, budget: 10000000 }, flash, rom);
  proxy.run();
  for (let i = 0; i < 250; i++) {
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
