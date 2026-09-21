import axios from 'axios';
import { writeFileSync } from 'fs';
async function compile(code) {
  const r = await axios.post('http://localhost:5525/api/compile/start', { code, target: 'esp32', targetEngine: 'frontend', fqbn: 'esp32:esp32:esp32' });
  while (true) {
    const s = await axios.get(`http://localhost:5525/api/compile/status/${r.data.buildId}`);
    if (s.data.status === 'success') return s.data.binary_content;
    if (s.data.status === 'failed') throw new Error('Compile failed: ' + s.data.error);
    await new Promise(r => setTimeout(r, 1000));
  }
}
const firmware = `void setup() { Serial.begin(115200); Serial.println("hello"); } void loop() { delay(1000); }`;
async function run() {
  const b64 = await compile(firmware);
  const bin = Buffer.from(b64, 'base64');
  writeFileSync('firmware.bin', bin);
  console.log('Firmware size:', bin.length);
  console.log('Partition table at 0x8000:');
  const pt = bin.subarray(0x8000, 0x8100);
  for (let i = 0; i < pt.length; i += 16) {
    const hex = Array.from(pt.subarray(i, i + 16)).map(b => b.toString(16).padStart(2, '0')).join(' ');
    console.log(`${(0x8000 + i).toString(16)}: ${hex}`);
  }
}
run().catch(e => { console.error(e); process.exit(1); });
