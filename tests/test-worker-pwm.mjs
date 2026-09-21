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
#include <driver/ledc.h>
void setup() {
  Serial.begin(115200);
  Serial.println("=== PWM TEST ===\\n");
  bool pass = true;
  esp_err_t err = ESP_OK;
  ledcAttach(2, 5000, 8);
  ledcWrite(2, 128);
  Serial.println("[PWM] write 128 OK");
  ledcWrite(2, 64);
  Serial.println("[PWM] write 64 OK");
  ledcDetach(2);
  Serial.println("[PWM] detach OK");
  ledc_timer_config_t ft = {};
  ft.speed_mode = LEDC_LOW_SPEED_MODE;
  ft.duty_resolution = LEDC_TIMER_8_BIT;
  ft.timer_num = LEDC_TIMER_1;
  ft.freq_hz = 5000;
  ft.clk_cfg = LEDC_AUTO_CLK;
  err = ledc_timer_config(&ft);
  Serial.print("FADE_TIMER="); Serial.println(err == ESP_OK ? "PASS" : "FAIL");
  if (err != ESP_OK) pass = false;
  ledc_channel_config_t fc = {};
  fc.gpio_num = 4;
  fc.speed_mode = LEDC_LOW_SPEED_MODE;
  fc.channel = LEDC_CHANNEL_1;
  fc.timer_sel = LEDC_TIMER_1;
  fc.duty = 0;
  fc.hpoint = 0;
  err = ledc_channel_config(&fc);
  Serial.print("FADE_CHAN="); Serial.println(err == ESP_OK ? "PASS" : "FAIL");
  if (err != ESP_OK) pass = false;
  err = ledc_fade_func_install(0);
  Serial.print("FADE_INSTALL="); Serial.println(err == ESP_OK ? "PASS" : "FAIL");
  if (err != ESP_OK) pass = false;
  err = ledc_set_fade_with_time(LEDC_LOW_SPEED_MODE, LEDC_CHANNEL_1, 255, 1000);
  Serial.print("FADE_SET="); Serial.println(err == ESP_OK ? "PASS" : "FAIL");
  if (err != ESP_OK) pass = false;
  err = ledc_fade_start(LEDC_LOW_SPEED_MODE, LEDC_CHANNEL_1, LEDC_FADE_WAIT_DONE);
  Serial.print("FADE_RUN="); Serial.println(err == ESP_OK ? "PASS" : "FAIL");
  if (err != ESP_OK) pass = false;
  uint32_t fduty = ledc_get_duty(LEDC_LOW_SPEED_MODE, LEDC_CHANNEL_1);
  Serial.printf("FADE_DUTY=%u\\n", (unsigned)fduty);
  if (fduty != 255) { Serial.println("FADE_DATA=FAIL"); pass = false; }
  else Serial.println("FADE_DATA=PASS");
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
  if (!out.includes('RESULT=PASS') || !out.includes('FADE_RUN=PASS') || !out.includes('FADE_DATA=PASS') || !out.includes('ALL TESTS PASSED')) {
    console.log('[test] FAILED'); process.exit(1);
  }
  console.log('[test] PASSED');
}

run().catch(e => { console.error(e); process.exit(1); });
