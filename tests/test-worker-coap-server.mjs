import { SimulatorWorker } from '../src/index.js';
import axios from 'axios';
import dgram from 'dgram';
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

// CoAP SERVER role on the board: firmware binds UDP 5683 and answers CON
// GET with ACK 2.05 (mirrored msgid/token). The host (Node dgram) plays the
// client through the gateway UDP forward (127.0.0.1:5683 -> board:5683) —
// needs the gateway up.
const firmware = `
#include <WiFi.h>
#include <WiFiUdp.h>
void setup() {
  Serial.begin(115200);
  Serial.println("=== COAP SERVER TEST ===");
  bool pass = true;
  WiFi.mode(WIFI_STA);
  WiFi.begin("TEST-AP", "");
  for (int i = 0; i < 100 && WiFi.status() != WL_CONNECTED; i++) delay(500);
  if (WiFi.status() != WL_CONNECTED) { Serial.println("RESULT=FAIL"); Serial.println("\\n=== ALL TESTS PASSED ==="); return; }
  WiFiUDP udp;
  if (!udp.begin(5683)) { Serial.println("RESULT=FAIL"); Serial.println("\\n=== ALL TESTS PASSED ==="); return; }
  Serial.println("COAP_SRV_READY");
  int served = 0;
  // Serve up to 6: the first 1-2 exchanges of a session are routinely lost
  // to gVisor neighbor-resolution warmup (replies need the stack to ARP the
  // board first); the host needs any 3 ACKs.
  for (int i = 0; i < 900 && served < 6; i++) {
    delay(100);
    int n = udp.parsePacket();
    if (n >= 4) {
      uint8_t req[64];
      int m = udp.read(req, sizeof(req));
      // CON GET -> ACK 2.05, same msgid/token, link-format payload.
      if (m >= 4 && (req[0] >> 6) == 1 && ((req[0] >> 4) & 3) == 0 && req[1] == 0x01) {
        uint8_t tkl = req[0] & 0x0F;
        uint8_t resp[64];
        resp[0] = 0x60 | tkl; resp[1] = 0x45; resp[2] = req[2]; resp[3] = req[3];
        int o = 4;
        for (int t = 0; t < tkl && o < 60; t++) resp[o++] = req[4 + t];
        resp[o++] = 0xFF;
        const char *pl = "</test>";
        for (int t = 0; pl[t] && o < 63; t++) resp[o++] = pl[t];
        udp.beginPacket(udp.remoteIP(), udp.remotePort());
        udp.write(resp, o);
        int sr = udp.endPacket();
        served++;
        Serial.printf("COAP_SERVED=%d sr=%d rip=%s rport=%u\\n", served, sr,
          udp.remoteIP().toString().c_str(), udp.remotePort());
      }
    }
  }
  // Let the final ACK drain through the forward before closing: the host
  // asserts on all 3 and may still be polling when we finish serving.
  delay(2000);
  udp.stop();
  Serial.printf("COAP_RX=%d\\n", served);
  if (served < 3) pass = false;
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
  await proxy.init('ESP32', { flashSizeMB: 4, mmuPages: 64, strapValue: 0x13, budget: 500000000, chunkSize: 20000, wifi: { ssid: 'TEST-AP', channel: 6 }, macAddress: '24:0a:c4:12:34:5A' }, flash, rom);
  proxy.chunkSize = 20000;
  proxy.run();

  // Host CoAP client: 3x CON GET, expect ACK 2.05 with our msgid.
  // Goes through the gateway UDP forward (per-board UDP_FORWARD port ->
  // board:5683) — the board address itself is unroutable from the host,
  // and 127.0.0.1:5683 belongs to whichever board bound it first on a
  // shared gateway. OUR forward port is announced on UART (BOARD_IP: /
  // PORT_FORWARD: / UDP_FORWARD: lines are console.log'd by the worker,
  // but the guest UART stream the test polls only carries guest bytes) —
  // so parse it from the worker stdout is impossible here; instead poll
  // getWifiStats() ONLY while the sim is paused (CMD protocol: the
  // command loop never runs while runSimChunk self-reschedules, so a
  // blocking getWifiStats() mid-run deadlocks both sides).
  let udpPort = 5683;
  let udpPortKnown = false;
  const cli = dgram.createSocket('udp4');
  const acks = [];
  cli.on('message', (msg) => {
    if (msg.length >= 4 && (msg[0] >> 6) === 1 && ((msg[0] >> 4) & 3) === 2) acks.push(msg[1]);
  });
  let sent = 0;
  // Run until the host has 3 ACKs AND the firmware has finished serving —
  // stopping early (on either alone) kills the other side mid-exchange.
  // NOTE: no getWifiStats() in this loop (see above) — instead, resolve
  // the UDP forward port BEFORE run() via a stop/poll/start cycle once
  // the gateway has issued DHCP (BOARD_IP known => forward allocated).
  for (let i = 0; i < 900 && (acks.length < 3 || !out.includes('ALL TESTS PASSED')); i++) {
    await new Promise(r => setTimeout(r, 150));
    proxy.pollUart();
    if (!udpPortKnown && i % 32 === 8) {
      proxy.stop();
      await new Promise(r => setTimeout(r, 300));
      try { const st = await proxy.getWifiStats(); if (st?.udpForward) { const m = st.udpForward.match(/:(\d+)$/); if (m) { udpPort = Number(m[1]); udpPortKnown = true; } } } catch {}
      proxy.run();
    }
    if (out.includes('COAP_SRV_READY') && sent < 16 && i % 8 === 0) {
      const mid = [0xA0 + sent, 0x0B + sent];
      const req = Buffer.concat([Buffer.from([0x40, 0x01, ...mid, 0xBC]), Buffer.from('.well-known'), Buffer.from([0x04]), Buffer.from('core')]);
      try { cli.send(req, udpPort, '127.0.0.1'); sent++; } catch {}
    }
    if (out.includes('ALL TESTS PASSED')) break;
  }
  // Drain: the last ACK is often still in flight when the firmware prints
  // RESULT (it served 3/3) — give it up to 5s wall before asserting.
  for (let i = 0; i < 50 && acks.length < 3; i++) {
    await new Promise(r => setTimeout(r, 100));
    proxy.pollUart();
  }
  cli.close();
  proxy.stop();
  await new Promise(r => setTimeout(r, 100));
  proxy.terminate();
  const get = (re) => { const m = out.match(re); return m ? m[1] : 'MISSING'; };
  console.log('COAP_RX=' + get(/COAP_RX=(\d+)/), 'HOST_ACKS=' + acks.length);
  const result = get(/RESULT=(\w+)/);
  console.log('RESULT=' + result);
  if (process.env.CAMDUMP) console.log('---UART---\n' + out.slice(-2000));
  if (result !== 'PASS' || acks.length < 3) { console.error('[test] FAILED'); process.exit(1); }
  console.log('[test] PASSED');
}
run().catch(e => { console.error(e); process.exit(1); });
