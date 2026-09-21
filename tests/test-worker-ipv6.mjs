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

// IPv6 board<->gateway: SLAAC ULA via the gateway RA, ICMPv6 echo and a
// UDP echo exchange with the gateway's fd00::1. Needs the gateway up.
// (No internet IPv6 — the gateway NAT is v4-only.)
const firmware = `
#include <WiFi.h>
#include <WiFiUdp.h>
#include "lwip/sockets.h"
#include "lwip/inet.h"
#include "lwip/netif.h"
#include "esp_netif.h"
#include "esp_netif_types.h"

// EUI-64 interface ID for MAC 24:0A:C4:12:34:59 (U/L bit flipped).
static const uint8_t IID[8] = {0x26, 0x0A, 0xC4, 0xFF, 0xFE, 0x12, 0x34, 0x59};

static uint16_t cksum6(const uint8_t *src, const uint8_t *dst, const uint8_t *msg, int n) {
  uint32_t s = 0;
  for (int i = 0; i < 16; i += 2) s += ((uint16_t)src[i] << 8) | src[i + 1];
  for (int i = 0; i < 16; i += 2) s += ((uint16_t)dst[i] << 8) | dst[i + 1];
  s += (uint32_t)n;
  s += 58;
  for (int i = 0; i + 1 < n; i += 2) s += ((uint16_t)msg[i] << 8) | msg[i + 1];
  if (n & 1) s += ((uint16_t)msg[n - 1] << 8);
  s = (s >> 16) + (s & 0xFFFF);
  s += (s >> 16);
  return (uint16_t)~s;
}

static bool ping6once(const uint8_t *src, const uint8_t *dst) {
  int s = lwip_socket(AF_INET6, SOCK_RAW, IPPROTO_ICMPV6);
  if (s < 0) { Serial.println("PING6 sockerr"); return false; }
  struct sockaddr_in6 to; memset(&to, 0, sizeof(to));
  to.sin6_family = AF_INET6;
  memcpy(&to.sin6_addr, dst, 16);
  // Link-local scope = LWIP netif IFINDEX, which is num+1 (station num=1
  // per the NETIF dump above, so scope 2 — NOT 1! scope 1 routes into the
  // loopback and frames vanish silently). Ignored for global destinations.
  to.sin6_scope_id = 2;
  uint8_t pkt[72];
  memset(pkt, 0, sizeof(pkt));
  pkt[0] = 128; pkt[1] = 0;
  pkt[4] = 0x12; pkt[5] = 0x34; pkt[6] = 0x00; pkt[7] = 0x01;
  for (int i = 8; i < 72; i++) pkt[i] = (uint8_t)i;
  uint16_t c = cksum6(src, dst, pkt, sizeof(pkt));
  pkt[2] = c >> 8; pkt[3] = c & 0xFF;
  if (lwip_sendto(s, pkt, sizeof(pkt), 0, (struct sockaddr *)&to, sizeof(to)) < 0) {
    Serial.println("PING6 senderr"); lwip_close(s); return false;
  }
  // Busy-spin poll (NOT blocking recv): blocking sleeps idle the CPU so
  // sim time fast-forwards past the reply; spinning keeps sim:wall <= ~1.
  bool ok = false;
  for (int i = 0; i < 30 && !ok; i++) {
    for (volatile uint32_t k = 0; k < 3000000u; k++) {}
    uint8_t rx[128];
    struct sockaddr_in6 from; socklen_t fl = sizeof(from);
    int n = lwip_recvfrom(s, rx, sizeof(rx), MSG_DONTWAIT, (struct sockaddr *)&from, &fl);
    // 40-byte IPv6 header + echo reply?
    ok = (n >= 48 && rx[40] == 129 && rx[44] == 0x12 && rx[45] == 0x34);
  }
  lwip_close(s);
  Serial.printf("PING6 %s\\n", ok ? "OK" : "noreply");
  return ok;
}

void setup() {
  Serial.begin(115200);
  Serial.println("=== IPV6 TEST ===");
  bool pass = true;
  WiFi.mode(WIFI_STA);
  WiFi.begin("TEST-AP", "");
  for (int i = 0; i < 100 && WiFi.status() != WL_CONNECTED; i++) delay(500);
  if (WiFi.status() != WL_CONNECTED) { Serial.println("RESULT=FAIL"); Serial.println("\\n=== ALL TESTS PASSED ==="); return; }
  WiFi.enableIPv6();
  // Read the netif flags: bit 8 = IPV6_AUTOCONFIG_ENABLED (SLAAC). If the
  // Arduino libs were built without CONFIG_LWIP_IPV6_AUTOCONFIG, no RA
  // will ever create a global address, no matter how correct it is.
  {
    esp_netif_t *e = esp_netif_get_handle_from_ifkey("WIFI_STA_DEF");
    struct netif *n = NULL;
    // esp_netif handle -> lwip netif via the (public) impl-name lookup is
    // unavailable; walk netif_list for the station interface instead.
    for (struct netif *t = netif_list; t != NULL; t = t->next) {
      if (t->name[0] == 's' && t->name[1] == 't') { n = t; break; }
    }
    (void)e;
    if (n) {
      Serial.printf("NETIF st flags=%08x mtu=%u autocfg=%d num=%d\\n",
        (unsigned)n->flags, (unsigned)n->mtu, (int)n->ip6_autoconfig_enabled, (int)n->num);
      for (int i = 0; i < LWIP_IPV6_NUM_ADDRESSES; i++) {
        const uint8_t *a = (const uint8_t *)&n->ip6_addr[i];
        Serial.printf("IP6ADDR[%d] st=%02x %02x%02x:%02x%02x:%02x%02x:%02x%02x:%02x%02x:%02x%02x:%02x%02x:%02x%02x\\n",
          i, (unsigned)n->ip6_addr_state[i],
          a[0],a[1],a[2],a[3],a[4],a[5],a[6],a[7],a[8],a[9],a[10],a[11],a[12],a[13],a[14],a[15]);
      }
    } else Serial.println("NETIF st NOT FOUND");
  }
  // Wait for the SLAAC global ULA (gateway RA). Busy-spin paced (NOT
  // delay()): delay() idles the CPU so sim time fast-forwards ~1000x and a
  // sim-time wait elapses in wall-ms, before the RA round trip completes.
  // Ground truth is the LWIP table itself (the Arduino globalIPv6()
  // wrapper lags it); the discovered address is reused below so the test
  // never assumes SLAAC output.
  uint8_t gua[16];
  auto slaacPresent = [&]() -> bool {
    struct netif *n = NULL;
    for (struct netif *t = netif_list; t != NULL; t = t->next) {
      if (t->name[0] == 's' && t->name[1] == 't') { n = t; break; }
    }
    if (!n) return false;
    for (int i = 0; i < LWIP_IPV6_NUM_ADDRESSES; i++) {
      const uint8_t *a = (const uint8_t *)&n->ip6_addr[i];
      bool global = !(a[0] == 0xFE && (a[1] & 0xC0) == 0x80);
      if (global && n->ip6_addr_state[i] >= 0x10) {
        memcpy(gua, a, 16);
        return true;
      }
    }
    return false;
  };
  bool got = false;
  // Generous budget: on a loaded host, boot+DHCP+assoc can take ~90s wall
  // before the first RS/RA round even happens; the gateway re-advertises
  // every 30s so any window eventually overlaps one.
  for (int i = 0; i < 400 && !got; i++) {
    for (volatile uint32_t k = 0; k < 3000000u; k++) {}
    got = slaacPresent();
  }
  Serial.print("IPV6_GUA="); Serial.println(WiFi.globalIPv6().toString());
  Serial.printf("SLAAC=%s\\n", got ? "OK" : "FAIL");
  if (!got) {
    // One-shot diagnostic dump (states + remaining valid lifetimes).
    struct netif *nd = NULL;
    for (struct netif *t = netif_list; t != NULL; t = t->next) {
      if (t->name[0] == 's' && t->name[1] == 't') { nd = t; break; }
    }
    if (nd) {
      for (int i = 0; i < LWIP_IPV6_NUM_ADDRESSES; i++) {
        const uint8_t *a = (const uint8_t *)&nd->ip6_addr[i];
        Serial.printf("IP6D[%d] st=%02x vlife=%lu %02x%02x%02x%02x\\n",
          i, (unsigned)nd->ip6_addr_state[i], (unsigned long)nd->ip6_addr_valid_life[i],
          a[0], a[1], a[8], a[9]);
      }
    }
    Serial.println("RESULT=FAIL"); Serial.println("\\n=== ALL TESTS PASSED ==="); return;
  }
  // Our own ULA = fd00:: + EUI-64 (matches what SLAAC assigns).
  uint8_t src[16] = {0xFD,0,0,0,0,0,0,0, 0,0,0,0,0,0,0,0};
  memcpy(src + 8, IID, 8);
  // Link-local source (always assigned).
  uint8_t llsrc[16] = {0xFE,0x80,0,0,0,0,0,0, 0,0,0,0,0,0,0,0};
  memcpy(llsrc + 8, IID, 8);
  const uint8_t gw[16] = {0xFD,0,0,0,0,0,0,0, 0,0,0,0,0,0,0,1};
  bool pok = false;
  for (int r = 0; r < 3 && !pok; r++) {
    if (r > 0) delay(1000);
    pok = ping6once(gua, gw);
  }
  if (!pok) {
    Serial.println("PING6-GUA miss, trying link-local source");
    for (int r = 0; r < 3 && !pok; r++) {
      if (r > 0) delay(1000);
      pok = ping6once(llsrc, gw);
    }
  }
  if (!pok) pass = false;
  // UDP echo with the gateway [fd00::1]:5683.
  {
    // NOTE: Arduino UDP has no v6 API surface here; raw socket carries it.
    int s = lwip_socket(AF_INET6, SOCK_DGRAM, 0);
    bool ok = false;
    if (s >= 0) {
      struct timeval tv; tv.tv_sec = 8; tv.tv_usec = 0;
      lwip_setsockopt(s, SOL_SOCKET, SO_RCVTIMEO, &tv, sizeof(tv));
      struct sockaddr_in6 srcaddr; memset(&srcaddr, 0, sizeof(srcaddr));
      srcaddr.sin6_family = AF_INET6;
      memcpy(&srcaddr.sin6_addr, llsrc, 16);
      srcaddr.sin6_port = htons(5684);
      bool bound = (lwip_bind(s, (struct sockaddr *)&srcaddr, sizeof(srcaddr)) == 0);
      Serial.printf("UDP6BIND=%d\\n", bound ? 1 : 0);
      struct sockaddr_in6 to; memset(&to, 0, sizeof(to));
      to.sin6_family = AF_INET6;
      memcpy(&to.sin6_addr, gw, 16);
      to.sin6_port = htons(5683);
      const char *msg = "hello6";
      for (int r = 0; r < 3 && !ok; r++) {
        if (r > 0) delay(1000);
        if (!bound) break;
        if (lwip_sendto(s, msg, 6, 0, (struct sockaddr *)&to, sizeof(to)) != 6) continue;
        for (int i = 0; i < 30 && !ok; i++) {
          for (volatile uint32_t k = 0; k < 3000000u; k++) {}
          uint8_t rx[64];
          struct sockaddr_in6 from; socklen_t fl = sizeof(from);
          int n = lwip_recvfrom(s, rx, sizeof(rx), MSG_DONTWAIT, (struct sockaddr *)&from, &fl);
          if (n == 6 && memcmp(rx, "hello6", 6) == 0) ok = true;
        }
      }
      lwip_close(s);
    }
    Serial.print("UDP6="); Serial.println(ok ? "PASS" : "FAIL");
    if (!ok) pass = false;
  }
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
  proxy.chunkSize = 20000;
  await proxy.init('ESP32', { flashSizeMB: 4, mmuPages: 64, strapValue: 0x13, budget: 500000000, wifi: { ssid: 'TEST-AP', channel: 6 }, macAddress: '24:0a:c4:12:34:59' }, flash, rom);
  proxy.run();
  for (let i = 0; i < 1200 && !out.includes('ALL TESTS PASSED'); i++) {
    await new Promise(r => setTimeout(r, 500));
    proxy.pollUart();
  }
  proxy.stop();
  try {
    const pcap = await proxy.getPcapData();
    if (pcap.length > 24) { writeFileSync('/tmp/opencode/ipv6.pcap', Buffer.from(pcap)); console.log('[PCAP] saved (' + pcap.length + ' bytes)'); }
  } catch {}
  await new Promise(r => setTimeout(r, 100));
  proxy.terminate();
  const get = (re) => { const m = out.match(re); return m ? m[1] : 'MISSING'; };
  console.log('IPV6_GUA=' + get(/IPV6_GUA=(.*)/));
  console.log('SLAAC=' + get(/SLAAC=(\w+)/));
  console.log('PING6=' + get(/PING6 (\w+)/), 'UDP6=' + get(/UDP6=(\w+)/));
  const result = get(/RESULT=(\w+)/);
  console.log('RESULT=' + result);
  if (process.env.CAMDUMP) console.log('---UART---\n' + out.slice(-3000));
  if (result !== 'PASS') { console.error('[test] FAILED'); process.exit(1); }
  console.log('[test] PASSED');
}
run().catch(e => { console.error(e); process.exit(1); });
