import { SimulatorWorker } from '/home/danish1075/Documents/esp32 emu/src/index.js';
import { readFileSync } from 'fs';
import axios from 'axios';

async function compile(code) {
  const r = await axios.post('http://localhost:5525/api/compile/start', { code, target: 'esp32', targetEngine: 'frontend', fqbn: 'esp32:esp32:esp32' });
  while (true) {
    const s = await axios.get(`http://localhost:5525/api/compile/status/${r.data.buildId}`);
    if (s.data.status === 'success') return s.data.binary_content;
    if (s.data.status === 'failed') throw new Error('Compile failed: ' + s.data.error);
    await new Promise(r => setTimeout(r, 1000));
  }
}

const fs = readFileSync('/tmp/opencode/diag-rmt-fw.bin', 'base64');
const flash = new Uint8Array(new SharedArrayBuffer(4 * 1024 * 1024));
flash.set(new Uint8Array(Buffer.from(fs, 'base64')));
const rom = readFileSync('/home/danish1075/Documents/esp32 emu/rom/esp32-v3-rom.bin');

const proxy = new SimulatorWorker();
await proxy.init('ESP32', { engine: 'wasm', flashSizeMB: 4, mmuPages: 64, strapValue: 0x13, budget: 10000000 }, flash, rom);
proxy.run();

let lastCyc = 0, lastTicks = 0;
for (let i = 0; i < 30; i++) {
  await new Promise(r => setTimeout(r, 250));
  proxy.stop();
  await new Promise(r => setTimeout(r, 50));
  proxy.pollUart();
  const cyc = proxy.debug[2];
  const ticks = proxy.debug[5];
  console.log(`[diag] i=${i} pc0=0x${proxy.pc.toString(16)} cyc=${cyc} dcyc=${cyc - lastCyc} ticks=${ticks} dticks=${ticks - lastTicks}`);
  lastCyc = cyc; lastTicks = ticks;
  proxy.run();
}
process.exit(0);