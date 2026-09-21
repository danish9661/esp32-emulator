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
#include <Arduino.h>
#include "esp_task.h"

static volatile int core0Count = 0;
static volatile int core1Count = 0;
static volatile bool core1Done = false;
static volatile uint32_t sharedA = 0;
static volatile uint32_t sharedB = 0;

void core1Task(void* arg) {
  for (int i = 0; i < 200000; i++) {
    core1Count++;
    sharedA = i;
    if ((i & 0x3FFF) == 0) taskYIELD();
  }
  core1Done = true;
  vTaskDelete(NULL);
}

void setup() {
  Serial.begin(115200);
  Serial.println("=== MULTI-CORE TEST ===\\n");

  xTaskCreatePinnedToCore(core1Task, "c1", 4096, NULL, 5, NULL, 1);

  // Core 0: spin doing independent work until core1 signals done
  while (!core1Done) {
    core0Count++;
    sharedB = core0Count;
    asm volatile("nop");
  }
  // Final increment to prove we ran after core1 finished
  core0Count++;
  sharedB = core0Count;

  bool pass = core1Done &&
              core1Count >= 200000 &&
              core0Count > 0 &&
              sharedA >= 199999;

  Serial.printf("[MC] core0=%d core1=%d sharedA=%d sharedB=%d\\n",
                core0Count, core1Count, sharedA, sharedB);
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

  await proxy.init('ESP32', { flashSizeMB: 4, mmuPages: 64, strapValue: 0x13, budget: 10000000 }, flash, rom);
  proxy.run();

  for (let i = 0; i < 200; i++) {
    await new Promise(r => setTimeout(r, 200));
    proxy.pollUart();
    if (out.includes('ALL TESTS PASSED') || out.includes('RESULT=FAIL')) break;
  }
  proxy.stop();
  await new Promise(r => setTimeout(r, 100));
  proxy.terminate();

  console.log(out);
  if (!out.includes('RESULT=PASS') || !out.includes('ALL TESTS PASSED')) {
    console.log('[test] FAILED');
    process.exit(1);
  }
  console.log('[test] PASSED');
}

run().catch(e => { console.error(e); process.exit(1); });
