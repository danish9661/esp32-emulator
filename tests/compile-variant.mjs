import axios from 'axios';
import { readFileSync, writeFileSync } from 'fs';
const testSrc = readFileSync('/home/danish1075/Documents/esp32 emu/tests/run-wasm-test.mjs', 'utf8');
const m = testSrc.match(/const code = `([\s\S]*?)`;/);
let code = m[1].replace(/\\n/g, '\n').replace(/\\"/g, '"').replace(/\\`/g, '`');
const i2cStart = code.indexOf('Serial.println("[I2C] Testing...");');
const i2cEnd = code.indexOf('Serial.println("[I2C] Done");') + '[I2C] Done".length'.length - 2;
const end = code.indexOf('Serial.println("[I2C] Done");');
const afterLen = 'Serial.println("[I2C] Done");'.length;
const i2cBlockEnd = end + afterLen;
const replacement = '  Serial.println("[I2C] Skipped (variant)");\n  delay(1);';
code = code.slice(0, i2cStart) + replacement + code.slice(i2cBlockEnd);
const startRes = await axios.post('http://localhost:5525/api/compile/start', {
  code, target: 'esp32', targetEngine: 'frontend', fqbn: 'esp32:esp32:esp32'
});
const id = startRes.data.buildId || startRes.data.id;
console.log('build id', id);
for (let i = 0; i < 180; i++) {
  await new Promise((r) => setTimeout(r, 2000));
  const st = await axios.get(`http://localhost:5525/api/compile/status/${id}`);
  if (st.data.status === 'success') {
    const bin = Buffer.from(st.data.binary_content, 'base64');
    writeFileSync('/tmp/opencode/fw-noi2c.bin', bin);
    console.log('saved', bin.length, 'bytes');
    process.exit(0);
  }
  if (st.data.status === 'error' || st.data.error) { console.error('compile error', JSON.stringify(st.data).slice(0, 400)); process.exit(1); }
}
console.error('timeout'); process.exit(1);
