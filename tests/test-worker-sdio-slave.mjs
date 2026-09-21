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
#include "driver/sdio_slave.h"
static uint8_t rxbuf[2048];
void setup() {
  Serial.begin(115200);
  Serial.println("=== SDIOSLAVE TEST ===");
  bool pass = true;
  sdio_slave_config_t cfg = {};
  cfg.timing = SDIO_SLAVE_TIMING_NSEND_PSAMPLE;
  cfg.sending_mode = SDIO_SLAVE_SEND_PACKET;
  cfg.send_queue_size = 4;
  cfg.recv_buffer_size = 1024;
  esp_err_t err = sdio_slave_initialize(&cfg);
  Serial.printf("SLV_INIT=%d\\n", (int)err);
  if (err != ESP_OK) pass = false;
  else Serial.println("SLV_INIT=PASS");
  sdio_slave_buf_handle_t h = sdio_slave_recv_register_buf(rxbuf);
  Serial.print("SLV_REGBUF="); Serial.println(h ? "PASS" : "FAIL");
  if (!h) pass = false;
  err = sdio_slave_recv_load_buf(h);
  Serial.print("SLV_LOADBUF="); Serial.println(err == ESP_OK ? "PASS" : "FAIL");
  if (err != ESP_OK) pass = false;
  err = sdio_slave_start();
  Serial.print("SLV_START="); Serial.println(err == ESP_OK ? "PASS" : "FAIL");
  if (err != ESP_OK) pass = false;
  sdio_slave_buf_handle_t rh = NULL;
  uint8_t *got = NULL;
  size_t len = 0;
  TickType_t t0 = xTaskGetTickCount();
  err = sdio_slave_recv(&rh, &got, &len, 500 / portTICK_PERIOD_MS);
  TickType_t dt = xTaskGetTickCount() - t0;
  Serial.printf("SLV_RECV=%d dt=%u\\n", (int)err, (unsigned)dt);
  if (err != ESP_ERR_TIMEOUT) { Serial.println("SLV_RECV=FAIL"); pass = false; }
  else Serial.println("SLV_RECV=PASS");
  sdio_slave_stop();
  sdio_slave_deinit();
  Serial.println("[SLV] stop/deinit OK");
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
  if (!out.includes('RESULT=PASS') || !out.includes('SLV_INIT=PASS') || !out.includes('SLV_REGBUF=PASS') || !out.includes('SLV_LOADBUF=PASS') || !out.includes('SLV_START=PASS') || !out.includes('SLV_RECV=PASS') || !out.includes('ALL TESTS PASSED')) {
    console.log('[test] FAILED'); process.exit(1);
  }
  console.log('[test] PASSED');
}

run().catch(e => { console.error(e); process.exit(1); });
