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
#include <driver/mcpwm.h>
#include <soc/soc.h>

static volatile uint32_t capVal = 0, capEdge = 0, capCount = 0;
static bool IRAM_ATTR capCb(mcpwm_unit_t m, mcpwm_capture_channel_id_t ch, const cap_event_data_t *e, void *u) {
  capVal = e->cap_value; capEdge = e->cap_edge; capCount++; return false;
}
void setup() {
  Serial.begin(115200);
  Serial.println("=== MCPWM TEST ===\\n");
  bool pass = true;

  esp_err_t err = mcpwm_gpio_init(MCPWM_UNIT_0, MCPWM0A, 2);
  Serial.printf("[MCPWM] gpio_init err=%d\\n", err);
  if (err != ESP_OK) pass = false;

  mcpwm_config_t cfg = {
    .frequency = 1000,
    .cmpr_a = 50.0,
    .cmpr_b = 50.0,
    .duty_mode = MCPWM_DUTY_MODE_0,
    .counter_mode = MCPWM_UP_COUNTER,
  };
  err = mcpwm_init(MCPWM_UNIT_0, MCPWM_TIMER_0, &cfg);
  Serial.printf("[MCPWM] init err=%d\\n", err);
  if (err != ESP_OK) pass = false;

  err = mcpwm_set_duty(MCPWM_UNIT_0, MCPWM_TIMER_0, MCPWM_OPR_A, 25.0);
  Serial.printf("[MCPWM] set_duty A err=%d\\n", err);
  if (err != ESP_OK) pass = false;
  err = mcpwm_set_duty(MCPWM_UNIT_0, MCPWM_TIMER_0, MCPWM_OPR_B, 75.0);
  if (err != ESP_OK) pass = false;

  mcpwm_start(MCPWM_UNIT_0, MCPWM_TIMER_0);
  Serial.println("[MCPWM] start OK");
  mcpwm_stop(MCPWM_UNIT_0, MCPWM_TIMER_0);
  Serial.println("[MCPWM] stop OK");

  float duty = mcpwm_get_duty(MCPWM_UNIT_0, MCPWM_TIMER_0, MCPWM_OPR_A);
  Serial.printf("[MCPWM] duty=%f\\n", duty);
  if (duty < 24.0f || duty > 26.0f) pass = false;

  err = mcpwm_set_frequency(MCPWM_UNIT_0, MCPWM_TIMER_0, 2000);
  Serial.printf("[MCPWM] set_frequency err=%d\\n", err);
  if (err != ESP_OK) pass = false;

  // Unit 1 (0x3FF6C000)
  err = mcpwm_init(MCPWM_UNIT_1, MCPWM_TIMER_0, &cfg);
  Serial.printf("[MCPWM] init(1) err=%d\\n", err);
  if (err != ESP_OK) pass = false;

  err = mcpwm_set_duty(MCPWM_UNIT_1, MCPWM_TIMER_0, MCPWM_OPR_A, 33.0);
  Serial.printf("[MCPWM] set_duty A(1) err=%d\\n", err);
  if (err != ESP_OK) pass = false;

  mcpwm_start(MCPWM_UNIT_1, MCPWM_TIMER_0);
  Serial.println("[MCPWM] start(1) OK");
  mcpwm_stop(MCPWM_UNIT_1, MCPWM_TIMER_0);

  float duty1 = mcpwm_get_duty(MCPWM_UNIT_1, MCPWM_TIMER_0, MCPWM_OPR_A);
  Serial.printf("[MCPWM] duty(1)=%f\\n", duty1);
  if (duty1 < 32.0f || duty1 > 34.0f) pass = false;

  // ---- capture UNIT_0 CAP0 on gpio 4 (host-driven edges) ----
  err = mcpwm_gpio_init(MCPWM_UNIT_0, MCPWM_CAP_0, 4);
  Serial.print("CAP_GPIO="); Serial.println(err == ESP_OK ? "PASS" : "FAIL");
  if (err != ESP_OK) pass = false;
  mcpwm_capture_config_t ccfg = {};
  ccfg.cap_edge = MCPWM_POS_EDGE;
  ccfg.cap_prescale = 1;
  ccfg.capture_cb = capCb;
  ccfg.user_data = NULL;
  err = mcpwm_capture_enable_channel(MCPWM_UNIT_0, MCPWM_SELECT_CAP0, &ccfg);
  Serial.print("CAP_EN="); Serial.println(err == ESP_OK ? "PASS" : "FAIL");
  if (err != ESP_OK) pass = false;
  Serial.println("CAP_READY");
  for (int i = 0; i < 6000 && !Serial.available(); i++) delay(10);
  if (Serial.available()) Serial.read();
  uint32_t v1 = mcpwm_capture_signal_get_value(MCPWM_UNIT_0, MCPWM_SELECT_CAP0);
  uint32_t e1 = mcpwm_capture_signal_get_edge(MCPWM_UNIT_0, MCPWM_SELECT_CAP0);
  Serial.printf("CAP1 val=%u edge=%u\\n", (unsigned)v1, (unsigned)e1);
  Serial.println("CAP_READY2");
  for (int i = 0; i < 6000 && !Serial.available(); i++) delay(10);
  if (Serial.available()) Serial.read();
  uint32_t v2 = mcpwm_capture_signal_get_value(MCPWM_UNIT_0, MCPWM_SELECT_CAP0);
  uint32_t e2 = mcpwm_capture_signal_get_edge(MCPWM_UNIT_0, MCPWM_SELECT_CAP0);
  Serial.printf("CAP2 val=%u edge=%u cb=%u\\n", (unsigned)v2, (unsigned)e2, (unsigned)capCount);
  if (!(v2 > v1 && e1 == 1 && e2 == 1 && capCount >= 1)) { Serial.println("CAPTURE=FAIL"); pass = false; }
  else Serial.println("CAPTURE=PASS");
  // ---- fault F0 on gpio 5 (host drives high) ----
  err = mcpwm_gpio_init(MCPWM_UNIT_0, MCPWM_FAULT_0, 5);
  Serial.print("FAULT_GPIO="); Serial.println(err == ESP_OK ? "PASS" : "FAIL");
  if (err != ESP_OK) pass = false;
  err = mcpwm_fault_init(MCPWM_UNIT_0, MCPWM_HIGH_LEVEL_TGR, MCPWM_SELECT_F0);
  Serial.print("FAULT_INIT="); Serial.println(err == ESP_OK ? "PASS" : "FAIL");
  if (err != ESP_OK) pass = false;
  Serial.println("FAULT_READY");
  for (int i = 0; i < 6000 && !Serial.available(); i++) delay(10);
  if (Serial.available()) Serial.read();
  uint32_t fdet = REG_READ(0x3FF5E000 + 0xE4);
  Serial.printf("FAULTDET=0x%x\\n", (unsigned)fdet);
  if ((fdet & 0x40) == 0) { Serial.println("FAULT=FAIL"); pass = false; }
  else Serial.println("FAULT=PASS");
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

  let cap1 = false, cap2 = false, flt = false;
  for (let i = 0; i < 200; i++) {
    await new Promise(r => setTimeout(r, 200));
    proxy.pollUart();
    if (!cap1 && out.includes('CAP_READY') && !out.includes('CAP_READY2')) {
      cap1 = true;
      await proxy.setPinInput(4, 0);
      await new Promise(r => setTimeout(r, 300));
      await proxy.setPinInput(4, 1);
      await new Promise(r => setTimeout(r, 300));
      proxy.sendUart('a');
    }
    if (!cap2 && out.includes('CAP_READY2')) {
      cap2 = true;
      await proxy.setPinInput(4, 0);
      await new Promise(r => setTimeout(r, 300));
      await proxy.setPinInput(4, 1);
      await new Promise(r => setTimeout(r, 300));
      proxy.sendUart('b');
    }
    if (!flt && out.includes('FAULT_READY')) {
      flt = true;
      await proxy.setPinInput(5, 1);
      await new Promise(r => setTimeout(r, 300));
      proxy.sendUart('c');
    }
    if (out.includes('ALL TESTS PASSED')) break;
  }
  proxy.stop();
  await new Promise(r => setTimeout(r, 100));
    proxy.terminate();

  console.log(out);
  if (!out.includes('RESULT=PASS') || !out.includes('CAPTURE=PASS') || !out.includes('FAULT=PASS') || !out.includes('ALL TESTS PASSED')) {
    console.log('[test] FAILED'); process.exit(1);
  }
  console.log('[test] PASSED');
}

run().catch(e => { console.error(e); process.exit(1); });
