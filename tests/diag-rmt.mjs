import { SimulatorWorker } from '/home/danish1075/Documents/esp32 emu/src/index.js';
import { readFileSync } from 'fs';
import { resolve, dirname } from 'path';
import { fileURLToPath } from 'url';
import axios from 'axios';

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
#include <driver/rmt.h>
void setup() {
  Serial.begin(115200);
  Serial.println("=== RMT TEST ===\\n");
  rmt_config_t cfg = RMT_DEFAULT_CONFIG_TX(GPIO_NUM_2, RMT_CHANNEL_0);
  cfg.clk_div = 80;
  esp_err_t err = rmt_config(&cfg);
  Serial.print("RMT_CONFIG="); Serial.println(err == ESP_OK ? "PASS" : "FAIL");
  if (err == ESP_OK) {
    err = rmt_driver_install(RMT_CHANNEL_0, 0, 0);
    Serial.print("RMT_INSTALL="); Serial.println(err == ESP_OK ? "PASS" : "FAIL");
    if (err == ESP_OK) { rmt_driver_uninstall(RMT_CHANNEL_0); Serial.println("[RMT] uninstall OK"); }
  }
  Serial.print("RESULT="); Serial.println(err == ESP_OK ? "PASS" : "FAIL");
  Serial.println("\\n=== ALL TESTS PASSED ===");
}
void loop() { delay(1000); }
`;

const b64 = await compile(firmware);
const flash = new Uint8Array(new SharedArrayBuffer(4 * 1024 * 1024));
flash.set(new Uint8Array(Buffer.from(b64, 'base64').toString('binary').split('').map(c => c.charCodeAt(0))));
const rom = readFileSync('/home/danish1075/Documents/esp32 emu/rom/esp32-v3-rom.bin');

const proxy = new SimulatorWorker();
let out = '';
proxy._onUART = (b) => { out += String.fromCharCode(b); };
proxy._onError = (e) => console.error('\n[diag] Error:', e.message);

await proxy.init('ESP32', { engine: 'wasm', flashSizeMB: 4, mmuPages: 64, strapValue: 0x13, budget: 10000000 }, flash, rom);
proxy.run();

for (let i = 0; i < 120; i++) {
  await new Promise(r => setTimeout(r, 250));
  proxy.stop();
  await new Promise(r => setTimeout(r, 50));
  proxy.pollUart();
  if (out.includes('ALL TESTS PASSED') || out.includes('FAIL')) break;
  if (i % 2 === 0) { const loop = await proxy.readMemory(0x40079558, 40); console.log('[diag] loop bytes:', Buffer.from(loop).toString('hex'));
    const c040 = await proxy.readMemory(0x4000c040, 64); console.log('[diag] c040 bytes:', Buffer.from(c040).toString('hex'));
    console.log(`[diag] i=${i} pc0=0x${proxy.pc.toString(16)} pc1=0x${proxy.pc1.toString(16)} uart=${out.length} tail=${JSON.stringify(out.slice(-50).replace(/\n/g, '|'))}`);
  }
  proxy.run();
}
console.log('[diag] final uart tail:', JSON.stringify(out.slice(-200)));
process.exit(0);