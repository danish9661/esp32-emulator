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

// Soft-AP mode: the ESP32 itself is the access point. No station can
// associate (no virtual station exists), but init, IP, MAC derivation and
// station counting are all live driver paths.
const firmware = `
#include <WiFi.h>
void setup() {
  Serial.begin(115200);
  Serial.println("=== SOFTAP TEST ===");
  bool pass = true;
  WiFi.mode(WIFI_AP);
  bool ok = WiFi.softAP("emu-ap", "password1", 6, 0, 4);
  Serial.printf("SOFTAP=%d\\n", ok ? 1 : 0);
  if (!ok) pass = false;
  String ip = WiFi.softAPIP().toString();
  Serial.print("SOFTAPIP="); Serial.println(ip);
  if (ip != "192.168.4.1") pass = false;
  String mac = WiFi.softAPmacAddress();
  Serial.print("SOFTAPMAC="); Serial.println(mac);
  // Must be a real derived address, not the eFuse-zero artifact.
  if (mac == "00:00:00:00:00:00" || mac == "00:00:00:00:00:01") pass = false;
  int n0 = WiFi.softAPgetStationNum();
  delay(3000);
  int n1 = WiFi.softAPgetStationNum();
  Serial.printf("STANUM=%d/%d\\n", n0, n1);
  if (n0 != 0 || n1 != 0) pass = false;
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
  // No gateway needed: soft-AP init/IP/MAC/station-count are all local.
  await proxy.init('ESP32', { flashSizeMB: 4, mmuPages: 64, strapValue: 0x13, budget: 500000000, wifi: false, macAddress: '24:0a:c4:12:34:59' }, flash, rom);
  proxy.run();
  for (let i = 0; i < 800 && !out.includes('ALL TESTS PASSED'); i++) {
    await new Promise(r => setTimeout(r, 100));
    proxy.pollUart();
  }
  proxy.stop();
  await new Promise(r => setTimeout(r, 100));
  proxy.terminate();
  const get = (re) => { const m = out.match(re); return m ? m[1] : 'MISSING'; };
  console.log('SOFTAP=' + get(/SOFTAP=(\d+)/), 'SOFTAPIP=' + get(/SOFTAPIP=(.*)/));
  console.log('SOFTAPMAC=' + get(/SOFTAPMAC=(.*)/), 'STANUM=' + get(/STANUM=(.*)/));
  const result = get(/RESULT=(\w+)/);
  console.log('RESULT=' + result);
  if (process.env.CAMDUMP) console.log('---UART---\n' + out.slice(-2000));
  if (result !== 'PASS') { console.error('[test] FAILED'); process.exit(1); }
  console.log('[test] PASSED');
}
run().catch(e => { console.error(e); process.exit(1); });
