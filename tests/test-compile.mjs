import axios from 'axios';
const code = `void setup() { Serial.begin(115200); Serial.println("Hello"); } void loop() { delay(1000); }`;
const r = await axios.post('http://localhost:5525/api/compile/start', {
  code, target: 'esp32', targetEngine: 'frontend', fqbn: 'esp32:esp32:esp32'
});
const id = r.data.buildId;
console.log('Build ID:', id);
let b64 = null;
while (true) {
  const s = await axios.get(`http://localhost:5525/api/compile/status/${id}`);
  if (s.data.status === 'success') { b64 = s.data.binary_content; break; }
  else if (s.data.status === 'failed') throw new Error('Failed: ' + s.data.error);
  process.stdout.write('.');
  await new Promise(r => setTimeout(r, 1000));
}
const buf = Buffer.from(b64, 'base64');
console.log('Total bytes:', buf.length);
console.log('Byte 0 (flash magic): 0x' + buf[0].toString(16));
console.log('Byte 1: 0x' + buf[1].toString(16));
console.log('First 64 hex:', buf.subarray(0, 64).toString('hex'));
// Look for ELF magic
console.log('First 4 as ELF magic:', buf.subarray(0, 4).toString('hex'));
// Check for the word "ESP32" in binary
const idx = buf.indexOf('ESP32', 0, 'utf8');
console.log('ESP32 string at offset:', idx);
const idxc3 = buf.indexOf('ESP32-C3', 0, 'utf8');
console.log('ESP32-C3 string at offset:', idxc3);
