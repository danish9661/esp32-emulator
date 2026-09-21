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

// PCNT: count 10 host-driven rising edges on GPIO4 (unit 0, channel 0,
// count-up on positive edge) + threshold-0 event status at the final count.
// Host toggles the pin via the new setPinInput() worker command, which routes
// through the native GPIO matrix into the PCNT pulse input.
const firmware = `
#include "driver/pcnt.h"
#include <soc/soc.h>

#define PCNT_BASE   0x3ff57000
#define CONF0_U0    (PCNT_BASE + 0x00)
#define CONF1_U0    (PCNT_BASE + 0x04)

void setup() {
  Serial.begin(115200);
  Serial.println("=== PCNT TEST ===");
  bool pass = true;

  pcnt_config_t cfg = {};
  cfg.pulse_gpio_num = 4;
  cfg.ctrl_gpio_num = PCNT_PIN_NOT_USED;
  cfg.lctrl_mode = PCNT_MODE_KEEP;
  cfg.hctrl_mode = PCNT_MODE_KEEP;
  cfg.pos_mode = PCNT_COUNT_INC;
  cfg.neg_mode = PCNT_COUNT_DIS;
  cfg.channel = PCNT_CHANNEL_0;
  cfg.unit = PCNT_UNIT_0;
  cfg.counter_h_lim = 100;
  cfg.counter_l_lim = -100;
  esp_err_t err = pcnt_unit_config(&cfg);
  Serial.print("CONFIG="); Serial.println(err == ESP_OK ? "PASS" : "FAIL");
  if (err != ESP_OK) pass = false;

  // Threshold-0 event at count 10 via direct registers (no interrupt enable,
  // so no ISR needed): thres0_en = CONF0 bit 14, thres0 = CONF1 low 16.
  uint32_t c0 = REG_READ(CONF0_U0);
  REG_WRITE(CONF0_U0, c0 | (1 << 14));
  REG_WRITE(CONF1_U0, 10);

  pcnt_counter_pause(PCNT_UNIT_0);
  pcnt_counter_clear(PCNT_UNIT_0);
  pcnt_counter_resume(PCNT_UNIT_0);
  Serial.println("PCNT_READY");
  // Wait for the host 'go' char (all edges delivered) with a long timeout.
  for (int i = 0; i < 6000 && !Serial.available(); i++) delay(10);
  if (Serial.available()) Serial.read();

  int16_t count = -1;
  pcnt_get_counter_value(PCNT_UNIT_0, &count);
  Serial.printf("COUNT=%d\\n", count);
  if (count != 10) { Serial.println("COUNT10=FAIL"); pass = false; }
  else Serial.println("COUNT10=PASS");

  uint32_t st = REG_READ(PCNT_BASE + 0x90); // Un_STATUS unit 0
  Serial.printf("STATUS=0x%x\\n", st);
  if ((st & 0x8) == 0) { Serial.println("THRES_EVT=FAIL"); pass = false; }
  else Serial.println("THRES_EVT=PASS");

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
  let ready = false;
  proxy._onUART = (b) => { out += String.fromCharCode(b); };
  proxy._onError = (e) => console.error('\n[test] Error:', e.message);

  await proxy.init('ESP32', { flashSizeMB: 4, mmuPages: 64, strapValue: 0x13, budget: 5000000 }, flash, rom);
  proxy.run();

  // Wait for READY, drive 10 rising edges, then wait for completion.
  const t0 = Date.now();
  while (Date.now() - t0 < 60000) {
    await new Promise(r => setTimeout(r, 200));
    proxy.pollUart();
    if (!ready && out.includes('PCNT_READY')) {
      ready = true;
      for (let i = 0; i < 10; i++) {
        await proxy.setPinInput(4, 1);
        await new Promise(r => setTimeout(r, 60));
        await proxy.setPinInput(4, 0);
        await new Promise(r => setTimeout(r, 60));
      }
      proxy.sendUart('g');
    }
    if (out.includes('ALL TESTS PASSED')) break;
  }
  proxy.stop();
  await new Promise(r => setTimeout(r, 100));
  proxy.terminate();

  console.log(out);
  if (!out.includes('CONFIG=PASS') || !out.includes('COUNT10=PASS') || !out.includes('THRES_EVT=PASS') || !out.includes('RESULT=PASS')) {
    console.log('[test] FAILED'); process.exit(1);
  }
  console.log('[test] PASSED');
}

run().catch(e => { console.error(e); process.exit(1); });
