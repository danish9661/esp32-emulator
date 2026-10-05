// Acceptance §2: ESP32 MAX7219 digits via SPI (esp32-emu.md).
// Firmware (Arduino SPI, no libs) inits a MAX7219 (shutdown off, scan all,
// 8 digit registers) on the default VSPI bus; the host model is fed from
// tapped MOSI bytes (16-bit addr+data pairs) and must show the digits.
// Requires the compile server (:5525).
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

// Minimal MAX7219 host model: 16-bit (addr, data) pairs → registers.
function makeMax7219() {
  const regs = new Array(16).fill(0);
  return {
    write(addr, data) { if (addr >= 0 && addr < 16) regs[addr] = data & 0xff; },
    get digits() { return regs.slice(1, 9); },
    get enabled() { return regs[0x0c] === 0x01; },
    get regs() { return regs.slice(); },
  };
}

const firmware = `
#include <SPI.h>

static void maxWrite(uint8_t addr, uint8_t data) {
  SPI.beginTransaction(SPISettings(1000000, MSBFIRST, SPI_MODE0));
  digitalWrite(SS, LOW);
  SPI.transfer(addr);
  SPI.transfer(data);
  digitalWrite(SS, HIGH);
  SPI.endTransaction();
}

void setup() {
  Serial.begin(115200);
  Serial.println("=== MAX7219 TEST ===");
  SPI.begin();
  maxWrite(0x0C, 0x00); // shutdown: off
  maxWrite(0x09, 0x00); // decode: none
  maxWrite(0x0A, 0x08); // intensity
  maxWrite(0x0B, 0x07); // scan limit: all
  maxWrite(0x0C, 0x01); // shutdown: on
  static const uint8_t digits[8] = {0x3F, 0x06, 0x5B, 0x4F, 0x66, 0x6D, 0x7D, 0x07};
  for (int i = 0; i < 8; i++) maxWrite(0x01 + i, digits[i]);
  Serial.println("DIGITS_SENT");
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

  await proxy.init('ESP32', { flashSizeMB: 4, mmuPages: 64, strapValue: 0x13, budget: 5000000 }, flash, rom);
  // Capture-only arm on both user buses (firmware uses one of them; the
  // empty inject arms the hook with no MISO side-effects).
  await proxy.injectSpiMiso(2, []);
  await proxy.injectSpiMiso(3, []);
  proxy.run();

  const max = makeMax7219();
  const pending = { 2: [], 3: [] };
  const t0 = Date.now();
  while (Date.now() - t0 < 150000) {
    await new Promise(r => setTimeout(r, 200));
    proxy.pollUart();
    for (const bus of [2, 3]) {
      for (const b of await proxy.pollSpiTx(bus)) pending[bus].push(b);
      // 16-bit addr+data pairs per CS cycle (firmware sends transfer() x2
      // back-to-back per register; pair them in order).
      while (pending[bus].length >= 2) {
        const addr = pending[bus].shift(), data = pending[bus].shift();
        max.write(addr, data);
      }
    }
    if (out.includes('DIGITS_SENT')) break;
  }
  proxy.stop();
  await new Promise(r => setTimeout(r, 100));
  proxy.terminate();

  console.log(out);
  console.log(`[test] digits=${max.digits.map((d) => d.toString(16)).join(',')} enabled=${max.enabled}`);
  const want = [0x3f, 0x06, 0x5b, 0x4f, 0x66, 0x6d, 0x7d, 0x07];
  const ok = max.enabled && JSON.stringify(max.digits) === JSON.stringify(want);
  if (!ok) { console.log('[test] FAILED'); process.exit(1); }
  console.log('[test] PASSED');
}

run().catch(e => { console.error(e); process.exit(1); });
