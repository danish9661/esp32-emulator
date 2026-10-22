// Shared firmware + build cache for the web/webserver worker tests.
//
// Both tests used to depend on a hardcoded Windows path
// (C:\Users\Danish\Downloads\esp32_1.ino) / a pre-built
// tests/build/webserver.bin that never existed in a fresh checkout, so they
// could not run anywhere but the original author's machine. The firmware now
// lives here and is compiled on demand through the compile server.
import axios from 'axios';
import { readFileSync, existsSync, mkdirSync, writeFileSync } from 'fs';
import { resolve, dirname } from 'path';
import { fileURLToPath } from 'url';

const __dirname = dirname(fileURLToPath(import.meta.url));

// Marker grepped by both tests to prove the served body is really ours.
export const WEB_MARKER = 'OPENHW-WEB-OK';

export const WEB_FIRMWARE = `
#include <WiFi.h>
#include <WebServer.h>

static const char *AP_SSID = "TEST-AP";
static const char *AP_PASS = "";  // virtual AP is open (same as the other wifi tests)
WebServer server(80);
static int hits = 0;

static void handleRoot() {
  hits++;
  String body = String("<!DOCTYPE html><html><head><title>ESP32</title></head><body>")
    + "<h1>ESP32 Web!</h1><p>" + "` + WEB_MARKER + `" + "</p>"
    + "<p>hits=" + String(hits) + "</p></body></html>";
  server.send(200, "text/html", body);
  Serial.printf("WEB! GET / hits=%d len=%u\\n", hits, (unsigned)body.length());
}

static void handleLed() {
  Serial.println("WEB! GET /led");
  server.send(200, "text/plain", "led=1");
}

void setup() {
  Serial.begin(115200);
  Serial.println("\\n=== WEB TEST ===");
  WiFi.mode(WIFI_STA);
  WiFi.begin(AP_SSID, AP_PASS);
  unsigned long t0 = millis();
  while (WiFi.status() != WL_CONNECTED && millis() - t0 < 20000) delay(50);
  bool up = WiFi.status() == WL_CONNECTED;
  Serial.printf("WIFI %s ip=%s mac=%s\\n", up ? "OK" : "FAIL",
                WiFi.localIP().toString().c_str(), WiFi.macAddress().c_str());
  if (!up) { Serial.println("WEB=FAIL"); return; }
  server.on("/", handleRoot);
  server.on("/led", handleLed);
  server.onNotFound([]() { server.send(404, "text/plain", "not found"); });
  server.begin();
  Serial.println("ready on port 80");
}

void loop() { server.handleClient(); delay(1); }
`;

// Compile once per checkout; the result is cached in the (gitignored)
// tests/build/ so the worker battery stays fast on re-runs.
export async function buildWebFirmware() {
  const cachePath = resolve(__dirname, 'build/webserver.bin');
  if (existsSync(cachePath)) return readFileSync(cachePath);
  const r = await axios.post('http://localhost:5525/api/compile/start', {
    code: WEB_FIRMWARE, target: 'esp32', targetEngine: 'frontend', fqbn: 'esp32:esp32:esp32',
  });
  for (;;) {
    const s = await axios.get(`http://localhost:5525/api/compile/status/${r.data.buildId}`);
    if (s.data.status === 'success') {
      const buf = Buffer.from(s.data.binary_content, 'base64');
      try { mkdirSync(resolve(__dirname, 'build'), { recursive: true }); } catch (e) { }
      writeFileSync(cachePath, buf);
      return buf;
    }
    if (s.data.status === 'failed') throw new Error('Compile failed: ' + s.data.error);
    await new Promise((res) => setTimeout(res, 1000));
  }
}

// Fetch through the gateway's static forward (TCP :80 on the board). The
// forward port is allocated per board IP by binding upward from 8080 at first
// DHCP and is never released, so a long-lived gateway hands out 8080, 8081,
// 8082 ... for successive boards — probe a range in parallel instead of
// hardcoding a port.
export async function fetchBoard(path = '/', timeoutMs = 4000, first = 8080, last = 8089) {
  const attempts = [];
  for (let port = first; port <= last; port++) {
    attempts.push((async () => {
      const res = await fetch(`http://127.0.0.1:${port}${path}`, { signal: AbortSignal.timeout(timeoutMs) });
      const body = await res.text();
      if (res.status !== 200) throw new Error(`:${port} status ${res.status}`);
      return { ok: true, status: res.status, body, bytes: body.length, port };
    })());
  }
  try {
    return await Promise.any(attempts);
  } catch (e) {
    const tried = (e && e.errors) ? e.errors.length : 0;
    const firstErr = (e && e.errors && e.errors[0]) ? (e.errors[0].message || String(e.errors[0])) : String(e);
    return { ok: false, error: `no forward port in ${first}:${first + 1}..${last} answered 200 (${tried} tried, first error: ${firstErr})` };
  }
}
