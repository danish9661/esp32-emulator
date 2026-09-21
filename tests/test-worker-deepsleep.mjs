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
#include "esp_sleep.h"
#include <string.h>
RTC_DATA_ATTR int bootCount = 0;
RTC_DATA_ATTR char rtcData[16] = "rtc-boot-marker";

void setup() {
  Serial.begin(115200);
  Serial.println("=== DEEP SLEEP TEST ===\\n");
  bootCount++;
  esp_sleep_wakeup_cause_t cause = esp_sleep_get_wakeup_cause();
  esp_reset_reason_t reason = esp_reset_reason();
  Serial.printf("[SLEEP] boot=%d cause=%d reason=%d rtc=%s\\n", bootCount, cause, reason, rtcData);
  esp_sleep_enable_timer_wakeup(1000000);
  if (bootCount >= 3) {
    bool pass = (cause == ESP_SLEEP_WAKEUP_TIMER) &&
                (reason == ESP_RST_DEEPSLEEP) &&
                (strcmp(rtcData, "rtc-boot-marker") == 0) &&
                (bootCount == 3);
    Serial.print("RESULT="); Serial.println(pass ? "PASS" : "FAIL");
    Serial.println("\\n=== ALL TESTS PASSED ===");
  } else {
    esp_deep_sleep_start();
  }
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

  await proxy.init('ESP32', { flashSizeMB: 4, mmuPages: 64, strapValue: 0x13, budget: 8000000 }, flash, rom);
  proxy.run();

  for (let i = 0; i < 350; i++) {
    await new Promise(r => setTimeout(r, 200));
    proxy.pollUart();
    if (out.includes('ALL TESTS PASSED')) break;
  }
  proxy.stop();
  await new Promise(r => setTimeout(r, 100));
  proxy.terminate();

  console.log(out);
  const boots = (out.match(/boot=(\d)/g) || []).map(m => Number(m.slice(5)));
  if (!out.includes('RESULT=PASS') || !out.includes('ALL TESTS PASSED') || boots.length < 3) {
    console.log('[test] FAILED');
    process.exit(1);
  }
  console.log(`[test] boots per cycle: ${boots.join(',')} (causes 0/4/4, reason=8 DEEPSLEEP)`);
  console.log('[test] PASSED');
}

run().catch(e => { console.error(e); process.exit(1); });