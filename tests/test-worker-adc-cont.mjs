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
#include "esp_adc/adc_continuous.h"
void setup() {
  Serial.begin(115200);
  Serial.println("=== ADCCONT TEST ===");
  bool pass = true;
  adc_continuous_handle_t handle = NULL;
  adc_continuous_handle_cfg_t hcfg = {};
  hcfg.max_store_buf_size = 1024;
  hcfg.conv_frame_size = 256;
  esp_err_t err = adc_continuous_new_handle(&hcfg, &handle);
  Serial.print("ADC_NEW="); Serial.println(err == ESP_OK ? "PASS" : "FAIL");
  if (err != ESP_OK) pass = false;
  adc_digi_pattern_config_t pat = {};
  pat.atten = ADC_ATTEN_DB_11;
  pat.channel = 0;
  pat.unit = ADC_UNIT_1;
  pat.bit_width = SOC_ADC_DIGI_MAX_BITWIDTH;
  adc_continuous_config_t dcfg = {};
  dcfg.sample_freq_hz = 20000;
  dcfg.conv_mode = ADC_CONV_SINGLE_UNIT_1;
  dcfg.format = ADC_DIGI_OUTPUT_FORMAT_TYPE1;
  dcfg.pattern_num = 1;
  dcfg.adc_pattern = &pat;
  err = adc_continuous_config(handle, &dcfg);
  Serial.print("ADC_CFG="); Serial.println(err == ESP_OK ? "PASS" : "FAIL");
  if (err != ESP_OK) pass = false;
  err = adc_continuous_start(handle);
  Serial.print("ADC_START="); Serial.println(err == ESP_OK ? "PASS" : "FAIL");
  if (err != ESP_OK) pass = false;
  uint8_t buf[256] = {0};
  uint32_t rlen = 0;
  err = adc_continuous_read(handle, buf, sizeof(buf), &rlen, 3000);
  Serial.printf("ADC_READ=%d LEN=%u\\n", (int)err, (unsigned)rlen);
  uint32_t good = 0, badch = 0;
  for (uint32_t i = 0; i + 1 < rlen; i += 2) {
    uint16_t v = buf[i] | ((uint16_t)buf[i+1] << 8);
    uint16_t ch = v >> 12, data = v & 0xFFF;
    if (ch == 0 && data > 1000) good++;
    else if (ch != 0) badch++;
  }
  Serial.printf("GOOD=%u BADCH=%u\\n", (unsigned)good, (unsigned)badch);
  if (err != ESP_OK || good == 0 || badch != 0) { Serial.println("ADC_DATA=FAIL"); pass = false; }
  else Serial.println("ADC_DATA=PASS");
  adc_continuous_stop(handle);
  adc_continuous_deinit(handle);
  Serial.println("[ADC] stop/deinit OK");
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

  await proxy.init('ESP32', { flashSizeMB: 4, mmuPages: 64, strapValue: 0x13, budget: 5000000, analogInputs: { 36: 2.0 } }, flash, rom);
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
  if (!out.includes('RESULT=PASS') || !out.includes('ADC_DATA=PASS') || !out.includes('ALL TESTS PASSED')) {
    console.log('[test] FAILED'); process.exit(1);
  }
  console.log('[test] PASSED');
}

run().catch(e => { console.error(e); process.exit(1); });
