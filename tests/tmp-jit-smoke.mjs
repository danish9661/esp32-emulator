// JIT smoke: worker boots cached speed firmware; grep process output for [JIT].
import { SimulatorWorker } from '../src/index.js';
import { readFileSync } from 'fs';
import { resolve, dirname } from 'path';
import { fileURLToPath } from 'url';
const __dirname = dirname(fileURLToPath(import.meta.url));
const ROM = readFileSync(resolve(__dirname, 'rom/esp32-v3-rom.bin'));
const FW = readFileSync(resolve(__dirname, 'tmp-speed-fw.bin'));
const sleep = (ms) => new Promise((r) => setTimeout(r, ms));
const flash = new Uint8Array(new SharedArrayBuffer(4 * 1024 * 1024));
flash.fill(0xff);
flash.set(new Uint8Array(FW.buffer, FW.byteOffset, FW.byteLength));
const proxy = new SimulatorWorker();
let out = '';
proxy._onUART = (b) => { out += String.fromCharCode(b); };
proxy._onError = (e) => console.error('[worker] Error:', e.message);
await proxy.init('ESP32', { flashSizeMB: 4, mmuPages: 64, strapValue: 0x13, budget: 5000000 }, flash, ROM);
proxy.run();
const t0 = Date.now();
while (!out.includes('SPEEDGO')) {
  await sleep(500);
  proxy.pollUart();
  if (Date.now() - t0 > 420000) throw new Error('no SPEEDGO; tail=' + out.slice(-400));
}
console.log('[smoke] SPEEDGO; spinning 90s...');
await sleep(90000);
proxy.stop();
await sleep(50);
proxy.terminate();
console.log('[smoke] done');
process.exit(0);
