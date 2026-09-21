import { SimulatorWorker } from '/home/danish1075/Documents/esp32 emu/src/index.js';
import { readFileSync } from 'fs';
const fs = readFileSync('/tmp/opencode/diag-rmt-fw.bin', 'base64');
const flash = new Uint8Array(new SharedArrayBuffer(4 * 1024 * 1024));
flash.set(new Uint8Array(Buffer.from(fs, 'base64')));
const rom = readFileSync('/home/danish1075/Documents/esp32 emu/rom/esp32-v3-rom.bin');
const proxy = new SimulatorWorker();
await proxy.init('ESP32', { engine: 'wasm', flashSizeMB: 4, mmuPages: 64, strapValue: 0x13, budget: 10000000 }, flash, rom);
proxy.run();
for (let i = 0; i < 8; i++) await new Promise(r => setTimeout(r, 250));
proxy.stop();
await new Promise(r => setTimeout(r, 50));
for (const addr of [0x3ff75144, 0x60035144, 0x3ff75000, 0x60035000]) {
  const a = await proxy.readMemory(addr, 8);
  const a2 = await proxy.readMemory(addr, 8);
  const d = (m) => (m[0] | (m[1]<<8) | (m[2]<<16) | (m[3]<<24)) >>> 0;
  console.log(`0x${addr.toString(16)}: read1=0x${d(a).toString(16)} read2=0x${d(a2).toString(16)}`);
}
process.exit(0);
