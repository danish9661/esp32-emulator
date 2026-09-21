import { SimulatorWorker } from '/home/danish1075/Documents/esp32 emu/src/index.js';
import { readFileSync } from 'fs';
const fs = readFileSync('/tmp/opencode/diag-rmt-fw.bin', 'base64');
const flash = new Uint8Array(new SharedArrayBuffer(4 * 1024 * 1024));
flash.set(new Uint8Array(Buffer.from(fs, 'base64')));
const rom = readFileSync('/home/danish1075/Documents/esp32 emu/rom/esp32-v3-rom.bin');
const proxy = new SimulatorWorker();
await proxy.init('ESP32', { engine: 'wasm', flashSizeMB: 4, mmuPages: 64, strapValue: 0x13, budget: 10000000 }, flash, rom);
proxy.run();
for (let i = 0; i < 16; i++) {
  await new Promise(r => setTimeout(r, 250));
  proxy.stop();
  await new Promise(r => setTimeout(r, 50));
  proxy.pollUart();
  const m = await proxy.readMemory(0x3fff0000, 16);
  const dwords = [];
  for (let j = 0; j < 16; j += 4) dwords.push((m[j] | (m[j+1]<<8) | (m[j+2]<<16) | (m[j+3]<<24)) >>> 0);
  console.log(`[diag] i=${i} pc0=0x${proxy.pc.toString(16)} flag[0]=0x${dwords[0].toString(16)} dwords=${dwords.map(x=>'0x'+x.toString(16)).join(' ')}`);
  proxy.run();
}
process.exit(0);
