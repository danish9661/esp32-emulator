import axios from 'axios';
import { writeFileSync } from 'fs';

const firmware = `
void setup() {
  Serial.begin(115200);
  Serial.println("HELLO FROM MINIMAL SETUP");
}
void loop() { delay(1000); }
`;

const r = await axios.post('http://localhost:5525/api/compile/start', { code: firmware, target: 'esp32', targetEngine: 'frontend', fqbn: 'esp32:esp32:esp32' });
while (true) {
  const s = await axios.get(`http://localhost:5525/api/compile/status/${r.data.buildId}`);
  if (s.data.status === 'success') {
    const buf = Buffer.from(s.data.binary_content, 'base64');
    writeFileSync('minimal-test.bin', buf);
    console.log('saved', buf.length, 'bytes');
    break;
  }
  if (s.data.status === 'failed') throw new Error('Compile failed: ' + s.data.error);
  await new Promise(r => setTimeout(r, 1000));
}
