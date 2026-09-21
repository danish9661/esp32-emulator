import { resolve, dirname } from 'path';
import { fileURLToPath } from 'url';
import { readFileSync } from 'fs';
import { SimulatorWorker } from '../src/sab/worker-proxy.js';

const __dirname = dirname(fileURLToPath(import.meta.url));
const flashBytes = readFileSync(resolve(__dirname, 'fw-proxy32.bin'));
const romBytes = readFileSync(resolve(__dirname, '../rom/esp32-v3-rom.bin'));

const flashSizeMB = Math.max(4, Math.ceil(flashBytes.length / (1024 * 1024)));
const flash = new Uint8Array(new SharedArrayBuffer(flashSizeMB * 1024 * 1024));
flash.fill(0xff);
flash.set(flashBytes);

const proxy = new SimulatorWorker();
let uartOutput = '';
proxy._onUART = (byte) => { uartOutput += String.fromCharCode(byte); };
proxy._onError = (err) => console.error('\n[ffi] Worker error:', err.message);

await proxy.init('ESP32', {
  engine: 'wasm',
  flashSizeMB,
  mmuPages: Math.ceil(flashBytes.length / 65536),
  strapValue: 0x13,
  pinInputs: { 0: true, 2: false, 12: false, 15: false },
  budget: 5000000,
  progressInterval: 10000000,
}, flash, romBytes);

proxy.run();

for (let i = 0; i < 2000; i++) {
  await new Promise((r) => setTimeout(r, 200));
  proxy.pollUart();
  if (uartOutput.includes('ALL TESTS PASSED')) break;
  if (i % 20 === 0 && i > 0) {
    console.log(`[ffi ${i * 0.2}s] nanos=${proxy.nanos} PC=0x${(proxy.pc || 0).toString(16).padStart(8, '0')} idle=${proxy.idle}`);
  }
}
const counts = await (async () => {
  proxy.stop();
  await new Promise((r) => setTimeout(r, 300));
  return proxy.getFfiCounts();
})();
const totals = { mmio: 0, mapRead: 0, mapWrite: 0, jsInt: 0, jsLog: 0 };
for (let i = 0; i < 40; i++) totals.mmio += counts[i] || 0;
totals.mapRead = counts[40] || 0;
totals.mapWrite = counts[41] || 0;
totals.jsInt = counts[42] || 0;
totals.jsLog = counts[43] || 0;
console.log('\n=== FFI COUNTS (fw-proxy32 full run) ===');
console.log(`mmio_read/write total:  ${totals.mmio}`);
const nonzero = [];
for (let i = 0; i < 40; i++) if (counts[i]) nonzero.push(`${i}=${counts[i]}`);
console.log(`mmio by handler: ${nonzero.join(', ')}`);
const pageOff = 256 * 8;
const pdv = new DataView(proxy.readResp.buffer, proxy.readResp.byteOffset, proxy.readResp.length);
const pages = [];
for (let i = pageOff; i + 16 <= proxy.readResp.length; i += 16) {
  const p = pdv.getFloat64(i, true);
  const c = pdv.getFloat64(i + 8, true);
  if (p === 0 && c === 0) break;
  pages.push(`0x${p.toString(16).padStart(8, '0')}=${c}`);
}
console.log(`mmio by page: ${pages.join(', ')}`);
console.log(`map_read:               ${totals.mapRead}`);
console.log(`map_write:              ${totals.mapWrite}`);
console.log(`js_interrupt:           ${totals.jsInt}`);
console.log(`js_log_str:             ${totals.jsLog}`);
console.log(`nanos: ${proxy.nanos}, uart: ${uartOutput.includes('ALL TESTS PASSED') ? 'PASSED' : 'NOT-DONE'}`);
await new Promise((r) => setTimeout(r, 500));
process.exit(0);