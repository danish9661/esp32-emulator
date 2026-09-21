import { SimulatorWorker } from '../src/index.js';
import { readFileSync, existsSync } from 'fs';
import { resolve, dirname } from 'path';
import { fileURLToPath } from 'url';

const __dirname = dirname(fileURLToPath(import.meta.url));

// Prefer cached download, fallback to repo-local copy
const CANDIDATES = [
  '/tmp/micropython-esp32.bin',
  resolve(__dirname, 'micropython-esp32.bin'),
  resolve(__dirname, '../micropython-esp32.bin'),
];
let binPath = CANDIDATES.find(p => existsSync(p));
if (!binPath) {
  console.log('[test] SKIP — MicroPython bin not found (download to /tmp/micropython-esp32.bin to enable)');
  console.log('  curl -L -o /tmp/micropython-esp32.bin https://micropython.org/resources/firmware/ESP32_GENERIC-20260824-v1.29.0.bin');
  process.exit(0);
}

const bin = readFileSync(binPath);
console.log(`[test] MicroPython bin: ${bin.length} bytes from ${binPath}`);

// Flash: 4MB, copy micropython at offset 0x1000 (as esptool write_flash 0x1000)
const flashSizeMB = 4;
const flash = new Uint8Array(new SharedArrayBuffer(flashSizeMB * 1024 * 1024));
flash.fill(0xff);
flash.set(bin, 0x1000);

const sim = new SimulatorWorker();
let uart = '';
let uartLines = [];
sim._onUART = (b) => {
  const c = String.fromCharCode(b);
  uart += c;
  if (c === '\n') {
    const line = uart.split('\n').slice(-2)[0];
    if (line && line.trim()) uartLines.push(line.trim());
  }
};
sim._onError = (e) => console.error('[ERR]', e.message || e);

await sim.init('ESP32', { flashSizeMB, mmuPages: Math.ceil(bin.length / 65536) + 2, strapValue: 0x13, budget: 5_000_000_000 }, flash);
sim.run();
const sleep = (ms) => new Promise(r => setTimeout(r, ms));
let booted = false;
for (let i = 0; i < 40; i++) {
  await sleep(250);
  sim.pollUart();
  if (uart.includes('MicroPython') && uart.includes('>>>')) { booted = true; break; }
}
sim.pollUart();

console.log(`[test] UART: ${uart.length} bytes, ${uartLines.length} lines`);
for (const l of uartLines.slice(0, 20)) console.log(`  ${l}`);

if (booted) {
  console.log('\n[test] PASSED — MicroPython v1.29.0 booted to REPL (>>> prompt)');
} else {
  console.error('\n[test] FAILED — no MicroPython banner/REPL');
  console.error(uart.slice(-2000));
  sim.terminate?.();
  process.exit(1);
}

const errFlag = Atomics.load(sim.ctrl, 13);
if (errFlag) {
  console.error(`[test] Error flag: ${errFlag}`);
  sim.terminate?.();
  process.exit(1);
}

sim.terminate?.();
process.exit(0);
