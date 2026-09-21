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
proxy._onError = (err) => console.error('\n[diag] Worker error:', err.message);

console.log('[diag] Initializing Worker...');
await proxy.init('ESP32', {
  engine: 'wasm',
  flashSizeMB,
  mmuPages: Math.ceil(flashBytes.length / 65536),
  strapValue: 0x13,
  pinInputs: { 0: true, 2: false, 12: false, 15: false },
  budget: 5000000,
  progressInterval: 10000000,
}, flash, romBytes);
console.log('[diag] Worker ready');

proxy.run();

for (let i = 0; i < 2000; i++) {
  await new Promise((r) => setTimeout(r, 200));
  proxy.pollUart();
  if (uartOutput.includes('ALL TESTS PASSED')) break;
  if (i % 20 === 0 && i > 0) {
    console.log(`[diag ${i * 0.2}s] nanos=${proxy.nanos} PC=0x${(proxy.pc || 0).toString(16).padStart(8, '0')} idle=${proxy.idle}`);
  }
}

const finalNanos = proxy.nanos;
proxy.stop();
await new Promise((r) => setTimeout(r, 200));
proxy.terminate();
console.log(`\n[diag] Final nanos: ${finalNanos}`);
console.log(`[diag] PASSED=${uartOutput.includes('ALL TESTS PASSED')}`);
console.log(`[diag] uart lines: ${uartOutput.split('\n').filter(l => l).length}`);
const histLines = uartOutput.split('\n').filter(l => l.includes('[MAPHIST]'));
console.log(`\n[MAPHIST-LINES] ${histLines.length}`);
for (const hl of histLines.slice(-40)) console.log(hl);