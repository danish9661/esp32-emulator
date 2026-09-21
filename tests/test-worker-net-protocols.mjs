import { SimulatorWorker } from '../src/index.js';
import axios from 'axios';
import dgram from 'dgram';
import os from 'os';
import { readFileSync, writeFileSync } from 'fs';
import { resolve, dirname } from 'path';
import { fileURLToPath } from 'url';

const __dirname = dirname(fileURLToPath(import.meta.url));

function hostLanIp() {
  for (const ifs of Object.values(os.networkInterfaces())) {
    for (const a of ifs || []) {
      if (a.family === 'IPv4' && !a.internal) return a.address;
    }
  }
  return '127.0.0.1';
}

async function compile(code) {
  const r = await axios.post('http://localhost:5525/api/compile/start', { code, target: 'esp32', targetEngine: 'frontend', fqbn: 'esp32:esp32:esp32' });
  while (true) {
    const s = await axios.get(`http://localhost:5525/api/compile/status/${r.data.buildId}`);
    if (s.data.status === 'success') return s.data.binary_content;
    if (s.data.status === 'failed') throw new Error('Compile failed: ' + s.data.error);
    await new Promise(r => setTimeout(r, 1000));
  }
}

// Full L3-L7 battery through the gateway NAT: DNS, outbound HTTP (TCP),
// NTP (UDP), raw-socket ICMP ping, and a minimal MQTT CONNECT handshake.
const firmware = `
#include <WiFi.h>
#include <HTTPClient.h>
#include <WiFiUdp.h>
#include "lwip/sockets.h"
#include "lwip/inet.h"

static uint16_t icmp_cksum(const uint8_t *p, int n) {
  uint32_t s = 0;
  for (int i = 0; i + 1 < n; i += 2) s += ((uint16_t)p[i] << 8) | p[i + 1];
  if (n & 1) s += ((uint16_t)p[n - 1] << 8);
  s = (s >> 16) + (s & 0xFFFF);
  s += (s >> 16);
  return (uint16_t)~s;
}

static bool pingOnce(const char *dstStr) {
  int s = lwip_socket(AF_INET, SOCK_RAW, IPPROTO_ICMP);
  if (s < 0) { Serial.printf("PING %s sockerr\\n", dstStr); return false; }
  struct timeval tv; tv.tv_sec = 8; tv.tv_usec = 0;
  lwip_setsockopt(s, SOL_SOCKET, SO_RCVTIMEO, &tv, sizeof(tv));
  struct sockaddr_in to; memset(&to, 0, sizeof(to));
  to.sin_family = AF_INET;
  to.sin_addr.s_addr = inet_addr(dstStr);
  uint8_t pkt[64]; memset(pkt, 0, sizeof(pkt));
  pkt[0] = 8; pkt[1] = 0;
  pkt[4] = 0x12; pkt[5] = 0x34; pkt[6] = 0x00; pkt[7] = 0x01;
  for (int i = 8; i < 64; i++) pkt[i] = (uint8_t)i;
  uint16_t c = icmp_cksum(pkt, sizeof(pkt));
  pkt[2] = c >> 8; pkt[3] = c & 0xFF;
  if (lwip_sendto(s, pkt, sizeof(pkt), 0, (struct sockaddr *)&to, sizeof(to)) < 0) {
    Serial.printf("PING %s senderr\\n", dstStr); lwip_close(s); return false;
  }
  uint8_t rx[128];
  struct sockaddr_in from; socklen_t fl = sizeof(from);
  int n = lwip_recvfrom(s, rx, sizeof(rx), 0, (struct sockaddr *)&from, &fl);
  lwip_close(s);
  if (n < 28) { Serial.printf("PING %s noreply\\n", dstStr); return false; }
  // skip 20-byte IP header (no options expected)
  bool ok = (rx[20] == 0 && rx[24] == 0x12 && rx[25] == 0x34);
  Serial.printf("PING %s %s\\n", dstStr, ok ? "OK" : "BAD");
  return ok;
}

// CoAP CON GET /.well-known/core over UDP (hand-rolled client) against a
// local responder (no public CoAP server is reachable from here). The
// responder runs in this test process; the board reaches it at the host
// LAN IP through the gateway NAT, proving the full CoAP exchange.
// Runs twice (EARLY + LATE) to catch state-dependent UDP-RX stalls.
static bool doCoap(const char *tag, const char *host, uint16_t port) {
  bool ok = false;
  IPAddress cip;
  for (int r = 0; r < 5 && cip.toString() == "0.0.0.0"; r++) {
    if (r > 0) delay(1000);
    if (WiFi.hostByName(host, cip) != 1) cip = IPAddress((uint32_t)0);
  }
  WiFiUDP udp;
  udp.begin(5684);
  const uint8_t mid_hi = 0x5A, mid_lo = 0xA5;
  // CON(0) GET(0.01) mid=0x5AA5, Uri-Path ".well-known" (11,12) + "core" (0,4)
  const uint8_t req[] = {0x40,0x01,mid_hi,mid_lo, 0xBC,
    '.','w','e','l','l','-','k','n','o','w','n', 0x04,'c','o','r','e'};
  for (int r = 0; r < 3 && !ok; r++) {
    if (r > 0) delay(1000);
    udp.beginPacket(cip, port);
    udp.write(req, sizeof(req));
    if (udp.endPacket() != 1) continue;
      // Busy-spin waits (NOT delay()): delay() idles the CPU so sim time
      // fast-forwards ~1000x vs wall-clock and the whole poll window burns
      // before the real-world reply (~40ms wall) can arrive. Spinning keeps
      // sim:wall <= ~1 so every reply lands inside the window.
      for (int i = 0; i < 30 && !ok; i++) {
        for (volatile uint32_t k = 0; k < 3000000u; k++) {}
        int n = udp.parsePacket();
        if (n > 0) {
          uint8_t resp[128];
          int m = udp.read(resp, sizeof(resp));
          Serial.printf("COAP_%s_PKT n=%d m=%d %02x%02x%02x%02x%02x%02x%02x%02x\\n",
            tag, n, m, resp[0], resp[1], resp[2], resp[3], resp[4], resp[5], resp[6], resp[7]);
          // Any well-formed CoAP response with our msgid (2.05 content from
          // the local responder, 4.xx errors from public servers — either
          // proves the full request/response round trip).
          // NOTE: version is bits[7:6] and type is bits[5:4] — an earlier
          // revision shifted both wrong ((b>>4)==1, (b>>2)&3) and rejected
          // every valid ACK, which was chased through the whole RX path
          // before the test's own bit-math was rechecked.
          ok = (m > 4 && ((resp[0] >> 6) & 3) == 1 && ((resp[0] >> 4) & 3) == 2 &&
                resp[2] == mid_hi && resp[3] == mid_lo);
        }
    }
  }
  udp.stop();
  Serial.printf("COAP_%s=%s\\n", tag, ok ? "PASS" : "FAIL");
  return ok;
}

void setup() {
  Serial.begin(115200);
  Serial.println("\\n=== NET PROTOCOLS TEST ===\\n");
  bool pass = true;
  WiFi.mode(WIFI_STA);
  WiFi.begin("TEST-AP", "");
  for (int i = 0; i < 100 && WiFi.status() != WL_CONNECTED; i++) delay(500);
  Serial.print("WIFI="); Serial.println(WiFi.status() == WL_CONNECTED ? "OK" : "FAIL");
  if (WiFi.status() != WL_CONNECTED) { Serial.println("RESULT=FAIL"); Serial.println("\\n=== ALL TESTS PASSED ==="); return; }
  Serial.print("IP="); Serial.println(WiFi.localIP());
  if (!doCoap("EARLY", COAP_SRV, 5683)) pass = false;

  // DNS (retried: the sim runs faster than wall-clock, so upstream
  // answers can arrive after one lookup's timeout — like real WiFi).
  IPAddress ip;
  bool dns = false;
  for (int r = 0; r < 5 && !dns; r++) {
    if (r > 0) { Serial.printf("DNS_RETRY=%d\\n", r); delay(1000); }
    dns = (WiFi.hostByName("example.com", ip) == 1);
  }
  Serial.print("DNS="); Serial.println(dns ? ip.toString() : "FAIL");
  if (!dns) pass = false;

  // HTTP over TCP (retried: handshakes can lose the sim-vs-wall race —
  // the SYN-ACK sometimes lands just after LWIP's RTO/user timeout).
  {
    HTTPClient http;
    http.setTimeout(30000);
    int code = -1;
    String body;
    for (int r = 0; r < 3 && code != 200; r++) {
      if (r > 0) { Serial.printf("HTTP_RETRY=%d\\n", r); delay(1000); }
      http.begin("http://example.com/");
      code = http.GET();
      if (code > 0) body = http.getString();
      http.end();
    }
    bool ok = (code == 200) && (body.indexOf("Example Domain") >= 0);
    Serial.printf("HTTP=%d body=%d %s\\n", code, body.length(), ok ? "OK" : "FAIL");
    if (!ok) pass = false;
  }

  // NTP over UDP (retried exchange like CoAP: same sim-vs-wall race).
  auto doNtp = [&](const char *tag, const char *host, uint16_t lport) -> bool {
    bool ok = false;
    IPAddress nip;
    for (int r = 0; r < 5; r++) { if (WiFi.hostByName(host, nip) == 1) break; delay(1000); }
    if (nip.toString() == "0.0.0.0") { Serial.printf("NTP_%s=dns FAIL\\n", tag); return false; }
    WiFiUDP udp;
    udp.begin(lport);
    for (int r = 0; r < 3 && !ok; r++) {
      if (r > 0) delay(1000);
      uint8_t req[48]; memset(req, 0, sizeof(req));
      req[0] = 0x1B;
      udp.beginPacket(nip, 123);
      udp.write(req, sizeof(req));
      if (udp.endPacket() != 1) continue;
      for (int i = 0; i < 80 && !ok; i++) {
        delay(500);
        if (udp.parsePacket() < 48) continue;
        uint8_t resp[48];
        if (udp.read(resp, sizeof(resp)) == 48) {
          uint32_t secs = ((uint32_t)resp[40] << 24) | ((uint32_t)resp[41] << 16) | ((uint32_t)resp[42] << 8) | resp[43];
          ok = (secs > 3900000000u); // past 2023
        }
      }
    }
    udp.stop();
    Serial.printf("NTP%s=%s\\n", tag, ok ? "OK" : "FAIL");
    return ok;
  };
  if (!doNtp("", "time.google.com", 2390)) pass = false;

  // Second NTP exchange late in the session (different server/port):
  // guards the late-test UDP RX path independently of CoAP.
  if (!doNtp("2", "time.windows.com", 2391)) pass = false;

  // CoAP, late phase (see EARLY call after WiFi connect).
  if (!doCoap("LATE", COAP_SRV, 5683)) pass = false;

  // ICMP ping: gateway first, then internet
  if (!pingOnce("192.168.4.1")) pass = false;
  if (!pingOnce("8.8.8.8")) pass = false;

  // Minimal MQTT CONNECT handshake (retried for the same race reasons).
  {
    WiFiClient c;
    bool ok = false;
    IPAddress mip;
    for (int r = 0; r < 5; r++) { if (WiFi.hostByName("test.mosquitto.org", mip) == 1) break; delay(1000); }
    Serial.print("MQTTDNS="); Serial.println(mip.toString());
    uint8_t ack[4] = {0};
    int n = 0;
    for (int r = 0; r < 2 && !ok; r++) {
      if (r > 0) { Serial.printf("MQTT_RETRY=%d\\n", r); delay(1000); }
      if (c.connect(mip, 1883, 30000)) {
        const uint8_t conn[] = {0x10,0x10, 0x00,0x04,'M','Q','T','T',0x04,0x02,0x00,0x3C, 0x00,0x04,'e','m','u','1'};
        c.write(conn, sizeof(conn));
        memset(ack, 0, sizeof(ack));
        n = 0;
        unsigned long t0 = millis();
        while (millis() - t0 < 30000 && n < 4) { int rr = c.readBytes(ack + n, 4 - n); if (rr > 0) n += rr; }
        ok = (n == 4 && ack[0] == 0x20 && ack[1] == 0x02 && ack[2] == 0x00 && ack[3] == 0x00);
        c.stop();
      } else Serial.println("MQTT=conn FAIL");
    }
    Serial.printf("MQTT=%02x%02x%02x%02x %s\\n", ack[0], ack[1], ack[2], ack[3], ok ? "OK" : "FAIL");
    if (!ok) pass = false;
  }

  // CoAP against a public internet server (any well-formed response with
  // our msgid proves the internet-CoAP round trip; the server answers
  // errors to unknown resources, which is fine).
  if (!doCoap("INET", "californium.eclipseprojects.io", 5683)) pass = false;

  Serial.print("RESULT="); Serial.println(pass ? "PASS" : "FAIL");
  Serial.println("\\n=== ALL TESTS PASSED ===");
}
void loop() { delay(1000); }
`;

async function run() {
  // Local CoAP responder (deterministic: no public CoAP server is reachable
  // from here). Replies ACK 2.05 + link-format payload to any CON GET.
  const coapHits = [];
  const coapSrv = dgram.createSocket('udp4');
  coapSrv.on('message', (msg, rinfo) => {
    if (msg.length < 4 || msg[0] >> 6 !== 1 || msg[1] !== 1) return;
    const tkl = msg[0] & 0x0F;
    // ACK ver=1/type=2/tkl: byte = 01_10_tkl = 0x60|tkl (0x80 would be
    // ver=2/CON — wrong; the firmware check is strict and correct).
    const resp = Buffer.concat([Buffer.from([(0x60 | tkl), 0x45]), msg.subarray(2, 4), msg.subarray(4, 4 + tkl), Buffer.from([0xFF]), Buffer.from('</test>;rt="test"')]);
    coapSrv.send(resp, rinfo.port, rinfo.address);
    coapHits.push(Date.now());
  });
  await new Promise((res, rej) => coapSrv.bind(5683, '0.0.0.0', (e) => e ? rej(e) : res()));
  const hostIp = hostLanIp();
  console.log('[test] CoAP responder on :5683, board target ' + hostIp);
  const b64 = await compile(firmware.replaceAll('COAP_SRV', `"${hostIp}"`));
  const flash = new Uint8Array(new SharedArrayBuffer(4 * 1024 * 1024));
  flash.set(new Uint8Array(Buffer.from(b64, 'base64').toString('binary').split('').map(c => c.charCodeAt(0))));
  const rom = readFileSync(resolve(__dirname, '../rom/esp32-v3-rom.bin'));
  const proxy = new SimulatorWorker();
  let out = '';
  proxy._onUART = (b) => { out += String.fromCharCode(b); };
  proxy._onError = (e) => console.error('\n[test] Error:', e.message);
  // Small sim chunks: runSimChunk() blocks the worker event loop (and the
  // gateway WebSocket with it) for a whole chunk, so big chunks quantize
  // inbound network delivery to ~1s and firmware timeouts expire first.
  // (Set via the proxy setter — init() forwards proxy._chunkSize, so a
  // config key alone would be overwritten by the 500000 default.)
  proxy.chunkSize = 20000;
  await proxy.init('ESP32', { flashSizeMB: 4, mmuPages: 64, strapValue: 0x13, budget: 80000000, wifi: { ssid: 'TEST-AP', channel: 6 }, macAddress: '24:0a:c4:12:34:57' }, flash, rom);
  proxy.run();
  for (let i = 0; i < 1200 && !out.includes('ALL TESTS PASSED'); i++) {
    await new Promise(r => setTimeout(r, 500));
    proxy.pollUart();
  }
  proxy.stop();
  try {
    const pcap = await proxy.getPcapData();
    if (pcap.length > 24) { writeFileSync('net-protocols.pcap', Buffer.from(pcap)); console.log('[PCAP] saved net-protocols.pcap (' + pcap.length + ' bytes)'); }
  } catch {}
  await new Promise(r => setTimeout(r, 100));
  proxy.terminate();
  const get = (re) => { const m = out.match(re); return m ? m[1] : 'MISSING'; };
  for (const k of ['WIFI', 'IP', 'DNS', 'HTTP', 'NTP', 'MQTTDNS', 'MQTT', 'NTP2']) console.log(k + '=' + get(new RegExp(k + '=(.*)')));
  console.log('COAP_EARLY=' + get(/COAP_EARLY=(\w+)/), 'COAP_LATE=' + get(/COAP_LATE=(\w+)/), 'COAP_INET=' + get(/COAP_INET=(\w+)/));
  console.log('RESULT=' + get(/RESULT=(\w+)/));
  console.log('COAP_HITS=' + coapHits.length);
  coapSrv.close();
  console.log('PING-GW=' + (out.match(/PING 192\.168\.4\.1 (\w+)/) || [])[1]);
  console.log('PING-NET=' + (out.match(/PING 8\.8\.8\.8 (\w+)/) || [])[1]);
  if (process.env.CAMDUMP) console.log('---UART---\n' + out.slice(-4000));
  if (get(/RESULT=(\w+)/) !== 'PASS' || coapHits.length === 0) { console.error('[test] FAILED'); process.exit(1); }
  console.log('[test] PASSED');
}
run().catch(e => { console.error(e); process.exit(1); });
