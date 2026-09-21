import { SimulatorWorker } from '../src/index.js';
import axios from 'axios';
import { readFileSync, writeFileSync } from 'fs';
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
#include <WiFi.h>
void setup() {
  Serial.begin(115200);
  Serial.println("\\n=== WIFI TEST ===\\n");
  WiFi.mode(WIFI_STA);
  WiFi.disconnect();
  delay(100);
  int n = WiFi.scanNetworks();
  Serial.print("[SCAN] found "); Serial.print(n); Serial.println(" networks");
  Serial.println("\\n=== ALL TESTS PASSED ===");
}
void loop() { delay(1000); }
`;
async function run() {
  const b64 = await compile(firmware);
  console.log('[test] compiled ' + Buffer.from(b64, 'base64').length + ' bytes');
  const flash = new Uint8Array(new SharedArrayBuffer(4 * 1024 * 1024));
  flash.set(new Uint8Array(Buffer.from(b64, 'base64').toString('binary').split('').map(c => c.charCodeAt(0))));
  const rom = readFileSync(resolve(__dirname, '../rom/esp32-v3-rom.bin'));
  const proxy = new SimulatorWorker();
  let out = '';
  proxy._onUART = (b) => { out += String.fromCharCode(b); };
  proxy._onError = (e) => console.error('\n[test] Error:', e.message);
  await proxy.init('ESP32', {
    flashSizeMB: 4, mmuPages: 64, strapValue: 0x13, budget: 80000000,
    wifi: { ssid: 'TEST-AP', channel: 6 },
    macAddress: '24:0a:c4:12:34:56'
  }, flash, rom);
  let prevOutLen = 0;
  proxy.run();
  for (let i = 0; i < 600; i++) {
    await new Promise(r => setTimeout(r, 400));
    proxy.pollUart();
    if (out.length > prevOutLen) {
      console.log('[UART+' + (out.length - prevOutLen) + '] ns=' + proxy.nanos + ' total=' + out.length);
      prevOutLen = out.length;
    }
    if (proxy.nanos > 600000000 && out.includes('ALL TESTS PASSED')) break;
  }
  proxy.stop();
  const pcap = await proxy.getPcapData();
  if (pcap.length > 0) {
    writeFileSync('wifi-capture.pcap', Buffer.from(pcap));
    console.log('[PCAP] saved wifi-capture.pcap (' + pcap.length + ' bytes)');
  } else {
    console.log('[PCAP] no pcap data');
  }
  await new Promise(r => setTimeout(r, 100));
  proxy.terminate();
  console.log(out);
  if (!out.includes('ALL TESTS PASSED')) { console.log('[test] FAILED'); process.exit(1); }
  console.log('[test] PASSED');
}
run().catch(e => { console.error(e); process.exit(1); });
