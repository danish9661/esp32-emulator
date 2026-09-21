import { SimulatorWorker } from '/home/danish1075/Documents/esp32 emu/src/index.js';
import { readFileSync } from 'fs';
const fs = readFileSync('/tmp/opencode/diag-rmt-fw.bin', 'base64');
const flash = new Uint8Array(new SharedArrayBuffer(4 * 1024 * 1024));
flash.set(new Uint8Array(Buffer.from(fs, 'base64')));
const rom = readFileSync('/home/danish1075/Documents/esp32 emu/rom/esp32-v3-rom.bin');
const proxy = new SimulatorWorker();
await proxy.init('ESP32', { engine: 'wasm', flashSizeMB: 4, mmuPages: 64, strapValue: 0x13, budget: 10000000 }, flash, rom);
proxy.run();
for (let i = 0; i < 6; i++) await new Promise(r => setTimeout(r, 250));
proxy.stop();
await new Promise(r => setTimeout(r, 50));
const bin = readFileSync('/tmp/opencode/bl_seg.bin');
const base = 0x3fff0030 - 0x40078000;
for (const a of [0x3fff0338, 0x3fff07ef, 0x3fff0897, 0x3fff08c4, 0x3fff0768, 0x3fff077b, 0x3fff07ad, 0x3fff0630, 0x3fff063a, 0x3fff0672, 0x3fff0714, 0x3fff06cc, 0x3fff0004]) {
  const off = a - 0x3fff0030;
  const bytes = bin.subarray(base + off, base + off + 64);
  const s = Buffer.from(bytes).toString('latin1').split('\0')[0];
  console.log(`0x${a.toString(16)}: "${s}"`);
}
process.exit(0);
