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

// Two-node ESP-NOW delivery through a shared gateway room: both boards run
// this firmware, add the BROADCAST peer, and send "hello" once. Each board
// must receive the other's frame (RXCB with the payload) — the gateway
// room is the 802.11 medium, the emulator the radios. Unencrypted, ch 1.
const firmware = `
#include <WiFi.h>
#include <esp_now.h>
void onSent(const wifi_tx_info_t *info, esp_now_send_status_t status) {
  Serial.printf("TXCB=%d\\n", (int)status);
}
void onRecv(const esp_now_recv_info_t *info, const uint8_t *data, int len) {
  Serial.print("RXCB=");
  for (int i = 0; i < len; i++) Serial.printf("%02x", data[i]);
  Serial.printf(" len=%d\\n", len);
}
void setup() {
  Serial.begin(115200);
  Serial.println("=== ESPNOW 2-NODE ===");
  WiFi.mode(WIFI_STA);
  WiFi.disconnect();
  Serial.printf("INIT=%d\\n", (int)esp_now_init());
  esp_now_register_send_cb(onSent);
  esp_now_register_recv_cb(onRecv);
  esp_now_peer_info_t p;
  memset(&p, 0, sizeof(p));
  memset(p.peer_addr, 0xFF, 6);
  p.channel = 1;
  p.encrypt = false;
  Serial.printf("ADDPEER=%d\\n", (int)esp_now_add_peer(&p));
  uint8_t bcast[6];
  memset(bcast, 0xFF, 6);
  delay(5000);
  const char *msg = "hello";
  Serial.printf("SEND=%d\\n", (int)esp_now_send(bcast, (const uint8_t *)msg, 5));
  Serial.println("READY");
}
void loop() { delay(1000); }
`;

async function makeNode(mac, name, flash, rom, room) {
  const proxy = new SimulatorWorker();
  let out = '';
  proxy._onUART = (b) => { out += String.fromCharCode(b); };
  proxy._onError = (e) => console.error(`\n[${name}] Error:`, e.message);
  proxy.chunkSize = 20000;
  await proxy.init('ESP32', { flashSizeMB: 4, mmuPages: 64, strapValue: 0x13, budget: 500000000, wifi: { ssid: 'TEST-AP', channel: 1, room }, macAddress: mac }, flash, rom);
  proxy.run();
  return { proxy, name, get out() { return out; } };
}

async function run() {
  const b64 = await compile(firmware);
  const mkFlash = () => {
    const flash = new Uint8Array(new SharedArrayBuffer(4 * 1024 * 1024));
    flash.set(new Uint8Array(Buffer.from(b64, 'base64').toString('binary').split('').map(c => c.charCodeAt(0))));
    return flash;
  };
  const rom = readFileSync(resolve(__dirname, '../rom/esp32-v3-rom.bin'));
  const room = 'espnow-room';

  const a = await makeNode('24:0a:c4:12:34:A1', 'A', mkFlash(), rom, room);
  const b = await makeNode('24:0a:c4:12:34:B2', 'B', mkFlash(), rom, room);

  let okA = false, okB = false;
  for (let i = 0; i < 600 && !(okA && okB); i++) {
    await new Promise(r => setTimeout(r, 200));
    a.proxy.pollUart();
    b.proxy.pollUart();
    // Each board's RXCB must show the peer's "hello" (68656c6c6f, len 5).
    // The gateway never echoes to the sender, so any RXCB here is remote.
    if (!okA && /RXCB=68656c6c6f len=5/.test(a.out)) { okA = true; console.log('[test] A received hello'); }
    if (!okB && /RXCB=68656c6c6f len=5/.test(b.out)) { okB = true; console.log('[test] B received hello'); }
  }
  const summarize = (n) => {
    const get = (re) => { const m = n.out.match(re); return m ? m[1] : 'MISSING'; };
    console.log(`[${n.name}] INIT=${get(/INIT=([-\d]+)/)} ADDPEER=${get(/ADDPEER=([-\d]+)/)} SEND=${get(/SEND=([-\d]+)/)} TXCB=${get(/TXCB=(\d+)/)}`);
  };
  summarize(a);
  summarize(b);
  a.proxy.terminate();
  b.proxy.terminate();
  if (okA && okB) console.log('[test] PASSED');
  else { console.error('[test] FAILED'); process.exit(1); }
}
run().catch(e => { console.error(e); process.exit(1); });
