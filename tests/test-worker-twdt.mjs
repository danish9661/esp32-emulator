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
#include <esp_task_wdt.h>
#include <driver/gptimer.h>

void setup() {
  Serial.begin(115200);
  Serial.println("=== TIMER WDT TEST ===\\n");
  bool pass = true;

  // Test 1: TWDT reconfigure (IDF 5.x API)
  Serial.println("[TWDT] Testing TWDT reconfigure...");
  esp_task_wdt_config_t twdt_cfg = {
    .timeout_ms = 5000,
    .idle_core_mask = 0,
    .trigger_panic = true,
  };
  esp_err_t err = esp_task_wdt_reconfigure(&twdt_cfg);
  Serial.print("[TWDT] reconfigure="); Serial.println(err == ESP_OK ? "PASS" : "FAIL");
  if (err != ESP_OK) pass = false;

  // Test 2: Feed the WDT
  Serial.println("[TWDT] Feeding WDT...");
  for (int i = 0; i < 5; i++) {
    esp_task_wdt_reset();
    delay(100);
  }
  Serial.println("[TWDT] feed OK");

  // Test 3: GPTimer (IDF 5.x replacement for legacy timer API)
  Serial.println("[TWDT] Testing gptimer...");
  gptimer_handle_t timer;
  gptimer_config_t tcfg = {
    .clk_src = GPTIMER_CLK_SRC_DEFAULT,
    .direction = GPTIMER_COUNT_UP,
    .resolution_hz = 1000000,
  };
  err = gptimer_new_timer(&tcfg, &timer);
  Serial.print("[TWDT] gptimer_new="); Serial.println(err == ESP_OK ? "PASS" : "FAIL");
  if (err != ESP_OK) pass = false;

  if (err == ESP_OK) {
    gptimer_set_raw_count(timer, 0);
    gptimer_start(timer);
    delay(50);
    uint64_t val = 0;
    gptimer_get_raw_count(timer, &val);
    gptimer_stop(timer);
    gptimer_del_timer(timer);
    Serial.print("[TWDT] timer_val="); Serial.println((uint32_t)val);
    Serial.println("[TWDT] timer_lifecycle=PASS");
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
  await proxy.init('ESP32', { flashSizeMB: 4, mmuPages: 64, strapValue: 0x13, budget: 15000000 }, flash, rom);
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
