// Counter dump: boot to SPEEDGO, fixed 4000-batch window, read PROF[0..25].
import { ESP32 } from '../src/index.js';
import { readFileSync, existsSync } from 'fs';
import { resolve, dirname } from 'path';
import { fileURLToPath } from 'url';
const __dirname = dirname(fileURLToPath(import.meta.url));
const ROM = readFileSync(resolve(__dirname, 'rom/esp32-v3-rom.bin'));
const WASM = readFileSync(resolve(__dirname, '../src/engine/esp-xtensa/esp_engine_wasm.wasm'));
const FWBIN = resolve(__dirname, 'tmp-speed-fw.bin');
if (!existsSync(FWBIN)) throw new Error('no cached firmware');
const bin = readFileSync(FWBIN);
const flash = new Uint8Array(4 * 1024 * 1024);
flash.fill(0xff);
flash.set(new Uint8Array(bin.buffer, bin.byteOffset, bin.byteLength));
const chip = new ESP32({ flashSizeMB: 4, flash });
chip.loadROM(ROM);
if (chip.gpio?.pins) {
  if (chip.gpio.pins[0]) chip.gpio.pins[0].inputValue = true;
  if (chip.gpio.pins[2]) chip.gpio.pins[2].inputValue = false;
  if (chip.gpio.pins[12]) chip.gpio.pins[12].inputValue = false;
  if (chip.gpio.pins[15]) chip.gpio.pins[15].inputValue = false;
}
chip.reset();
chip.flash.set(flash);
if (chip.gpio) chip.gpio.strapValue = 0x13;
if (chip.mmuTablePro) for (let p = 0; p < 64; p++) { chip.mmuTablePro[p] = p; chip.mmuTableApp[p] = p; }
try { chip.cores[0].writeUint32(0x3ff5a104, 0x5aa5); } catch {}
await chip.loadWasm(WASM, 'wasm');
const ex = chip._wasmLoader.exports;
if (typeof ex.native_prof_read !== 'function') throw new Error('no prof export');
let uart = '';
chip.uart[0].onTX = (b) => { uart += String.fromCharCode(b); };
let fireDueFn = null;
function step() {
  ex.core_run(512);
  chip.cycles += 512;
  try {
    if (!chip._sabU32) chip._sabU32 = new Uint32Array(chip._wasmMemory || chip._wasmMemoryObj?.buffer);
    chip._sabU32[998] = chip.clocks.cpu.ticks >>> 0;
    chip._sabU32[999] = chip.cycles >>> 0;
  } catch {}
  try {
    if (!fireDueFn) { const r = chip.clocks?.root; fireDueFn = r?.fireDueEvents?.bind(r) || null; }
    if (fireDueFn) fireDueFn();
  } catch {}
  let idle = false;
  try { idle = !!chip.coresIdle; } catch {}
  if (idle && ex?.native_idle_advance) {
    try { chip.cycles += ex.native_idle_advance(chip.cycles >>> 0); } catch {}
  } else {
    try { ex?.native_set_clock_state?.(chip.cycles >>> 0); } catch {}
    try { ex?.native_pump_events?.(); } catch {}
  }
}
const tB0 = Date.now();
while (!uart.includes('SPEEDGO')) {
  step();
  if (Date.now() - tB0 > 600000) throw new Error('boot timeout');
}
console.log('[profcnt] at spins, draining 4000 batches...');
for (let i = 0; i < 4000; i++) step();
const v = [];
for (let i = 0; i < 26; i++) { try { v.push(ex.native_prof_read(i) >>> 0); } catch { v.push(-1); } }
const tot = v.slice(0, 16).reduce((a, b) => a + b, 0);
console.log('nibble-hist total=' + tot);
v.slice(0, 16).forEach((c, i) => console.log(`  nib${i}: ${(100 * c / Math.max(1, tot)).toFixed(1)}% (${c})`));
console.log(`width: 2B=${v[16]} 3B=${v[17]} 4B=${v[18]}`);
console.log(`dispatch: wide=${v[19]} narrow=${v[20]} pie31=${v[24]}`);
console.log(`intercepts: rsil=${v[21]} waiti=${v[22]} loop=${v[23]} idle-exit=${v[25]}`);
process.exit(0);
