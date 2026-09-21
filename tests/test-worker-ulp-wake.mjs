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

// ULP WR_REG/RD_REG live routing + I_WAKE deep-sleep cause. The ULP program
// writes RTC_CNTL_STORE0[15:0] via WR_REG, reads it back via RD_REG into R0
// (stored to RTC slow mem), then I_WAKE + HALT. Boot 1 verifies the routing
// in both directions (slow-mem copy AND direct REG_READ — proving the write
// hit the real register file, not a shadow), enables ULP + timer wakeup and
// deep-sleeps. Boot 2 requires wake cause ESP_SLEEP_WAKEUP_ULP (6): the WAKE
// latch wins over the timer bit when ULP wakeup is enabled (first-trigger
// race parity). Wake TIMING still follows the sleep timer (synchronous ULP
// model runs the program at ulp_run time, not during sleep) — documented.
const firmware = `
#include "esp32/ulp.h"
#include "esp_sleep.h"
#include "soc/rtc_cntl_reg.h"
#include "soc/soc.h"
RTC_DATA_ATTR int bootCount = 0;
enum { ULP_MAGIC_ADDR = 16 };
const ulp_insn_t prog[] = {
  I_WR_REG(RTC_CNTL_STORE0_REG, 0, 15, 0xAB),
  I_RD_REG(RTC_CNTL_STORE0_REG, 0, 15),
  I_MOVI(R1, ULP_MAGIC_ADDR),
  I_ST(R0, R1, 0),
  I_WAKE(),
  I_HALT(),
};
void setup() {
  Serial.begin(115200);
  Serial.println("=== ULPWAKE TEST ===");
  bootCount++;
  esp_sleep_wakeup_cause_t cause = esp_sleep_get_wakeup_cause();
  esp_reset_reason_t reason = esp_reset_reason();
  Serial.printf("[ULPWAKE] boot=%d cause=%d reason=%d\\n", bootCount, (int)cause, (int)reason);
  Serial.printf("[ULPWAKE] wstate=%x\\n", (unsigned)REG_READ(RTC_CNTL_WAKEUP_STATE_REG));
  bool pass = true;
  if (bootCount == 1) {
    volatile uint32_t *slow = (volatile uint32_t *)0x50000000;
    size_t psize = sizeof(prog) / sizeof(ulp_insn_t);
    esp_err_t err = ulp_process_macros_and_load(0, prog, &psize);
    Serial.print("ULP_LOAD="); Serial.println(err == ESP_OK ? "PASS" : "FAIL");
    if (err != ESP_OK) pass = false;
    err = ulp_run(0);
    Serial.print("ULP_RUN="); Serial.println(err == ESP_OK ? "PASS" : "FAIL");
    if (err != ESP_OK) pass = false;
    uint32_t m0 = slow[ULP_MAGIC_ADDR];
    uint32_t s0 = REG_READ(RTC_CNTL_STORE0_REG);
    Serial.printf("ULP_RW=%x %x\\n", (unsigned)m0, (unsigned)s0);
    if (m0 != 0xAB || (s0 & 0xFFFF) != 0xAB) { Serial.println("ULP_WRREG=FAIL"); pass = false; }
    else Serial.println("ULP_WRREG=PASS");
    if (!pass) { Serial.println("RESULT=FAIL"); Serial.println("\\n=== ALL TESTS PASSED ==="); return; }
    err = esp_sleep_enable_ulp_wakeup();
    Serial.printf("ULP_WUEN=%d\\n", (int)err);
    esp_sleep_enable_timer_wakeup(2000000);
    esp_deep_sleep_start();
  } else {
    if (cause != ESP_SLEEP_WAKEUP_ULP) { Serial.println("ULP_CAUSE=FAIL"); pass = false; }
    else Serial.println("ULP_CAUSE=PASS");
    if (reason != ESP_RST_DEEPSLEEP) { Serial.println("ULP_REASON=FAIL"); pass = false; }
    else Serial.println("ULP_REASON=PASS");
    Serial.print("RESULT="); Serial.println(pass ? "PASS" : "FAIL");
    Serial.println("\\n=== ALL TESTS PASSED ===");
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

  const get = (re) => { const m = out.match(re); return m ? m[1] : 'MISSING'; };
  for (const m of out.matchAll(/\[ULPWAKE\] boot=\d+ cause=\d+ reason=\d+/g)) console.log('[ULPWAKE]', m[0]);
  for (const m of out.matchAll(/\[ULPWAKE\] wstate=[0-9a-f]+/g)) console.log('[ULPWAKE]', m[0]);
  console.log('ULP_LOAD=' + get(/ULP_LOAD=(\w+)/), 'ULP_RUN=' + get(/ULP_RUN=(\w+)/));
  console.log('ULP_RW=' + get(/ULP_RW=([0-9a-f ]+)/), 'ULP_WRREG=' + get(/ULP_WRREG=(\w+)/));
  console.log('ULP_WUEN=' + get(/ULP_WUEN=(-?\d+)/), 'ULP_CAUSE=' + get(/ULP_CAUSE=(\w+)/), 'ULP_REASON=' + get(/ULP_REASON=(\w+)/));
  const result = get(/RESULT=(\w+)/);
  console.log('RESULT=' + result);
  if (result !== 'PASS') { console.error('[test] FAILED'); process.exit(1); }
  console.log('[test] PASSED');
}

run().catch(e => { console.error(e); process.exit(1); });
