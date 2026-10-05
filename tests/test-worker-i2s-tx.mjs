// I2S TX tap (§9): guest→host DMA capture through the full stack.
// Firmware (legacy driver) writes a 64-byte incrementing pattern via
// i2s_write on I2S0; the host arms the TX hook, polls staged u32 words and
// asserts the byte stream matches. Requires the compile server (:5525).
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
  Serial.println("=== I2STX TEST ===");
  i2s_config_t cfg = {
    .mode = (i2s_mode_t)(I2S_MODE_MASTER | I2S_MODE_TX),
    .sample_rate = 44100,
    .bits_per_sample = I2S_BITS_PER_SAMPLE_16BIT,
    .channel_format = I2S_CHANNEL_FMT_RIGHT_LEFT,
    .communication_format = I2S_COMM_FORMAT_I2S,
    .intr_alloc_flags = ESP_INTR_FLAG_LEVEL1,
    .dma_buf_count = 2,
    .dma_buf_len = 64
  };
  esp_err_t err = i2s_driver_install(I2S_NUM_0, &cfg, 0, NULL);
  Serial.print("INSTALL="); Serial.println(err == ESP_OK ? "PASS" : "FAIL");
  bool pass = (err == ESP_OK);
  if (err == ESP_OK) {
    uint8_t buf[64];
    for (int i = 0; i < 64; i++) buf[i] = (uint8_t)(i * 3 + 1);
    size_t written = 0;
    esp_err_t werr = i2s_write(I2S_NUM_0, buf, sizeof(buf), &written, 1000 / portTICK_PERIOD_MS);
    Serial.print("WRITE="); Serial.println(werr == ESP_OK ? "PASS" : "FAIL");
    if (werr != ESP_OK) pass = false;
    delay(500);
    Serial.println("TXDONE");
    i2s_driver_uninstall(I2S_NUM_0);
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
  await proxy.armI2sTx();
  proxy.run();

  const t0 = Date.now();
  let txBytes = [];
  while (Date.now() - t0 < 150000) {
    await new Promise(r => setTimeout(r, 200));
    proxy.pollUart();
    for (const w of await proxy.pollI2sTx(0)) {
      txBytes.push(w & 0xff, (w >> 8) & 0xff, (w >> 16) & 0xff, (w >> 24) & 0xff);
    }
    if (out.includes('RESULT=') && txBytes.length >= 64) break;
  }
  proxy.stop();
  await new Promise(r => setTimeout(r, 100));
  proxy.terminate();

  console.log(out);
  console.log(`[test] captured ${txBytes.length} TX bytes`);
  let pass = out.includes('RESULT=PASS');
  const expect = [];
  for (let i = 0; i < 64; i++) expect.push((i * 3 + 1) & 0xff);
  // DMA ships LE words; the byte stream must contain the pattern in order
  // (chunked across FFI calls — search, don't assume offset 0).
  const hay = txBytes.join(',');
  const needle = expect.join(',');
  if (!hay.includes(needle)) {
    console.log(`[test] pattern missing (head: ${txBytes.slice(0, 16).join(',')})`);
    pass = false;
  } else console.log('[test] TX pattern verified in captured stream');
  if (!pass || txBytes.length < 64) { console.log('[test] FAILED'); process.exit(1); }
  console.log('[test] PASSED');
}

run().catch(e => { console.error(e); process.exit(1); });
