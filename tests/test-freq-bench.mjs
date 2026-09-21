import { SimulatorWorker } from '../src/index.js';
import { readFileSync } from 'fs';
import { resolve, dirname } from 'path';
import { fileURLToPath } from 'url';
const __dirname = dirname(fileURLToPath(import.meta.url));
const flashData = readFileSync(resolve(__dirname, 'build/webserver.bin'));
const flash = new Uint8Array(new SharedArrayBuffer(flashData.length));
flash.set(flashData);
const rom = readFileSync(resolve(__dirname, '../rom/esp32-v3-rom.bin'));

async function bench(label, cpuFreq) {
  console.log(`\n=== ${label}: cpuFreq=${JSON.stringify(cpuFreq)} ===`);
  const proxy = new SimulatorWorker();
  let out = '';
  proxy._onUART = (b) => { out += String.fromCharCode(b); };
  await proxy.init('ESP32', {
    flashSizeMB: 4, mmuPages: 64, strapValue: 0x13, budget: 80000000,
    wifi: false, cpuFrequency: cpuFreq,
  }, flash, rom);
  
  const target = 63000000;
  const actualFreq = proxy.cpuFrequency;
  proxy.run();
  const t0 = Date.now();
  while (proxy.nanos < target) {
    await new Promise(r => setTimeout(r, 5));
    if (Date.now() - t0 > 30000) break;
  }
  const dt = Date.now() - t0;
  const finalNanos = proxy.nanos;
  proxy.stop();
  proxy.terminate();
  console.log(`  freq=${(actualFreq/1e6).toFixed(0)}MHz  wall=${dt}ms  nanos=${finalNanos}`);
}

console.log('--- CPU Frequency vs Wall Time (same 63M nanos target) ---\n');
console.log('Lower freq => larger nanos multiplier => reach target with fewer actual cycles => less wall time\n');
await bench('max  ', 'max');
await bench('80MHz', 80);
await bench('8MHz ', 'auto');
process.exit(0);
