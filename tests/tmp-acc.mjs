// Bit-exact equivalence gate for the decode cache: boot the busy firmware
// raw and compare its printed ACC against the independent CPython reference
// f0d809784e93ba21 (200M cached loop iterations must match bit-for-bit).
// Exit 0 on match, 1 otherwise. No compile server needed (cached binary).
import { ESP32 } from '../src/index.js';
import { readFileSync, existsSync } from 'fs';
import { resolve, dirname } from 'path';
import { fileURLToPath } from 'url';

const __dirname = dirname(fileURLToPath(import.meta.url));
const ROM = readFileSync(resolve(__dirname, 'rom/esp32-v3-rom.bin'));
const WASM = readFileSync(process.env.WASM_BIN || resolve(__dirname, '../src/engine/esp-xtensa/esp_engine_wasm.wasm'));
const FWBIN = resolve(__dirname, 'tmp-speed-fw.bin');
if (!existsSync(FWBIN)) throw new Error('no cached firmware');
const bin = readFileSync(FWBIN);
const EXPECT = 'f0d809784e93ba21';

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
let fireDueFn = null;
let uart = '';
chip.uart[0].onTX = (b) => { uart += String.fromCharCode(b); };
const t0 = Date.now();
while (!uart.includes('SPEEDDONE')) {
  chip.step();
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
  if (Date.now() - t0 > 600000) throw new Error('timeout; uart tail:\n' + uart.slice(-800));
}
const m = uart.match(/ACC=([0-9a-fA-F]+)/);
console.log(`[acc] wall ${((Date.now() - t0) / 1000).toFixed(0)}s uart=${uart.length}B hasDONE=${uart.includes('SPEEDDONE')} ACC=${m?.[1] ?? 'MISSING'} expect ${EXPECT}`);
console.log('[acc] tail: ' + JSON.stringify(uart.slice(-400)));
if ((m?.[1] ?? '').toLowerCase() !== EXPECT) { console.log('[acc] FAILED'); process.exit(1); }
console.log('[acc] PASSED');
