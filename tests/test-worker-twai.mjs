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
#include <driver/twai.h>
#include <string.h>

void setup() {
  Serial.begin(115200);
  Serial.println("=== TWAI TEST ===\\n");
  bool pass = true;
  twai_general_config_t g = TWAI_GENERAL_CONFIG_DEFAULT(GPIO_NUM_21, GPIO_NUM_22, TWAI_MODE_NO_ACK);
  twai_timing_config_t t = TWAI_TIMING_CONFIG_125KBITS();
  twai_filter_config_t f = TWAI_FILTER_CONFIG_ACCEPT_ALL();
  esp_err_t err = twai_driver_install(&g, &t, &f);
  Serial.print("TWAI_INSTALL="); Serial.println(err == ESP_OK ? "PASS" : "FAIL");
  if (err != ESP_OK) pass = false;
  if (err == ESP_OK) {
    err = twai_start();
    Serial.print("TWAI_START="); Serial.println(err == ESP_OK ? "PASS" : "FAIL");
    if (err != ESP_OK) pass = false;
    // NO_ACK self-transmission: TX a std frame, expect self-reception.
    twai_message_t txmsg = {};
    txmsg.identifier = 0x123;
    txmsg.data_length_code = 4;
    txmsg.data[0] = 'C'; txmsg.data[1] = 'A'; txmsg.data[2] = 'N'; txmsg.data[3] = '!';
    err = twai_transmit(&txmsg, 1000 / portTICK_PERIOD_MS);
    Serial.print("TWAI_TX="); Serial.println(err == ESP_OK ? "PASS" : "FAIL");
    if (err != ESP_OK) pass = false;
    twai_message_t rxmsg = {};
    err = twai_receive(&rxmsg, 2000 / portTICK_PERIOD_MS);
    Serial.print("TWAI_RX="); Serial.println(err == ESP_OK ? "PASS" : "FAIL");
    if (err != ESP_OK) pass = false;
    else {
      Serial.printf("RX_ID=0x%x RX_DLC=%d RX_DATA=%c%c%c%c\\n", (unsigned)rxmsg.identifier,
        (int)rxmsg.data_length_code, rxmsg.data[0], rxmsg.data[1], rxmsg.data[2], rxmsg.data[3]);
      if (rxmsg.identifier != 0x123 || rxmsg.data_length_code != 4 ||
          memcmp(rxmsg.data, "CAN!", 4) != 0) { Serial.println("TWAI_DATA=FAIL"); pass = false; }
      else Serial.println("TWAI_DATA=PASS");
    }
    twai_stop();
    Serial.println("[TWAI] stop OK");
    twai_driver_uninstall();
    Serial.println("[TWAI] uninstall OK");
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

  await proxy.init('ESP32', { flashSizeMB: 4, mmuPages: 64, strapValue: 0x13, budget: 5000000 }, flash, rom);
  proxy.run();

  for (let i = 0; i < 100; i++) {
    await new Promise(r => setTimeout(r, 200));
    proxy.pollUart();
    if (proxy.nanos > 50000000 && out.includes('ALL TESTS PASSED')) break;
  }
  proxy.stop();
  await new Promise(r => setTimeout(r, 100));
  proxy.terminate();

  console.log(out);
  if (!out.includes('RESULT=PASS') || !out.includes('TWAI_TX=PASS') || !out.includes('TWAI_RX=PASS') || !out.includes('TWAI_DATA=PASS') || !out.includes('ALL TESTS PASSED')) {
    console.log('[test] FAILED'); process.exit(1);
  }
  console.log('[test] PASSED');
}

run().catch(e => { console.error(e); process.exit(1); });
