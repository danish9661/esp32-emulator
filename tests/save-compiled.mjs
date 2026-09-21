import axios from 'axios';
import { writeFileSync } from 'fs';

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

const r = await axios.post('http://localhost:5525/api/compile/start', { code: firmware, target: 'esp32', targetEngine: 'frontend', fqbn: 'esp32:esp32:esp32' });
while (true) {
  const s = await axios.get(`http://localhost:5525/api/compile/status/${r.data.buildId}`);
  if (s.data.status === 'success') {
    const b64 = s.data.binary_content;
    const buf = Buffer.from(b64, 'base64');
    writeFileSync('compiled-wifitest.bin', buf);
    console.log('saved', buf.length, 'bytes to compiled-wifitest.bin');
    break;
  }
  if (s.data.status === 'failed') throw new Error('Compile failed: ' + s.data.error);
  await new Promise(r => setTimeout(r, 1000));
}
