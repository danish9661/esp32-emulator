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
#include <driver/i2s.h>
void setup() {
  Serial.begin(115200);
  Serial.println("=== I2SRX TEST ===");
  bool pass = true;
  i2s_config_t cfg = {};
  cfg.mode = (i2s_mode_t)(I2S_MODE_MASTER | I2S_MODE_RX);
  cfg.sample_rate = 16000;
  cfg.bits_per_sample = I2S_BITS_PER_SAMPLE_16BIT;
  cfg.channel_format = I2S_CHANNEL_FMT_RIGHT_LEFT;
  cfg.communication_format = I2S_COMM_FORMAT_STAND_I2S;
  cfg.intr_alloc_flags = ESP_INTR_FLAG_LEVEL1;
  cfg.dma_buf_count = 4;
  cfg.dma_buf_len = 64;
  esp_err_t err = i2s_driver_install(I2S_NUM_0, &cfg, 0, NULL);
  Serial.print("RX_INSTALL="); Serial.println(err == ESP_OK ? "PASS" : "FAIL");
  if (err != ESP_OK) pass = false;
  uint8_t buf[256];
  memset(buf, 0, sizeof(buf));
  size_t got = 0;
  err = i2s_read(I2S_NUM_0, buf, sizeof(buf), &got, 3000 / portTICK_PERIOD_MS);
  Serial.printf("RX_READ=%d GOT=%u\\n", (int)err, (unsigned)got);
  if (err != ESP_OK || got != sizeof(buf)) { Serial.println("RX_READ=FAIL"); pass = false; }
  else Serial.println("RX_READ=PASS");
  uint32_t bad = 0;
  for (int i = 0; i < 64; i++) {
    uint32_t v = buf[4*i] | ((uint32_t)buf[4*i+1] << 8) | ((uint32_t)buf[4*i+2] << 16) | ((uint32_t)buf[4*i+3] << 24);
    if (v != (uint32_t)(0x1000 + i)) bad++;
  }
  Serial.printf("RX_BAD=%u\\n", (unsigned)bad);
  if (bad != 0) { Serial.println("RX_DATA=FAIL"); pass = false; }
  else Serial.println("RX_DATA=PASS");
  i2s_driver_uninstall(I2S_NUM_0);
  Serial.println("[I2S] uninstall OK");
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
  // Feed after driver init (init resets RX and would wipe a pre-run feed).
  (async () => {
    for (let i = 0; i < 200 && !out.includes("RX_INSTALL="); i++) { await new Promise(r => setTimeout(r, 100)); proxy.pollUart(); }
    for (let i = 0; i < 512; i++) await proxy.feedI2SRX(0x1000 + i);
  })();

  for (let i = 0; i < 100; i++) {
    await new Promise(r => setTimeout(r, 200));
    proxy.pollUart();
    if (proxy.nanos > 50000000 && out.includes('ALL TESTS PASSED')) break;
  }
  proxy.stop();
  await new Promise(r => setTimeout(r, 100));
  proxy.terminate();

  console.log(out);
  if (!out.includes('RESULT=PASS') || !out.includes('RX_INSTALL=PASS') || !out.includes('RX_READ=PASS') || !out.includes('RX_DATA=PASS') || !out.includes('ALL TESTS PASSED')) {
    console.log('[test] FAILED'); process.exit(1);
  }
  console.log('[test] PASSED');
}

run().catch(e => { console.error(e); process.exit(1); });
