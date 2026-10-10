// Trace-recorder proof: boot cached firmware to spins, dump the ring,
// assert a trace covers the known spin-loop body (0x400d1620-0x400d1649
// per objdump) with sane op words. No compile server needed (cached bin).
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
for (const fn of ['native_trace_len', 'native_trace_scratch_ptr', 'native_trace_scratch_len', 'native_trace_enable', 'native_trace_threshold']) {
  if (typeof ex[fn] !== 'function') throw new Error('missing export ' + fn);
}
let fireDueFn = null;
let uart = '';
chip.uart[0].onTX = (b) => { uart += String.fromCharCode(b); };
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
console.log('[trace] SPEEDGO, spinning 3000 batches...');
for (let i = 0; i < 3000; i++) step();
const n = ex.native_trace_len() >>> 0;
console.log('[trace] completed traces: ' + n);
const ptr = ex.native_trace_scratch_ptr() >>> 0;
const len = ex.native_trace_scratch_len() >>> 0;
const mem = new Uint32Array(chip._wasmLoader.memoryBuffer);
const ring = mem.slice(ptr >>> 2, (ptr >>> 2) + len);
const STR = (ring.length - 1) / 16;
console.log('[trace] scratch[0]=' + ring[0]);
let hits = 0;
for (let s = 0; s < 16; s++) {
  const start = ring[1 + s * STR + 1] >>> 0;
  const tlen = ring[1 + s * STR + 2] & 0xFFFF;
  if (!tlen) continue;
  const pcs = [];
  for (let i = 0; i < Math.min(tlen, 6); i++) pcs.push('0x' + (ring[1 + s * STR + 3 + i * 3] >>> 0).toString(16));
  const core = ring[1 + s * STR] >>> 0;
  console.log(`[trace] slot${s}: core=${core} start=0x${start.toString(16)} len=${tlen} pcs=${pcs.join(' ')}${tlen > 6 ? ' ...' : ''}`);
  if (process.env.OPS && s < 2) {
    const ops = [];
    for (let i = 0; i < tlen; i++) ops.push('0x' + (ring[1 + s * STR + 3 + i * 3] >>> 0).toString(16) + ':0x' + (ring[1 + s * STR + 3 + i * 3 + 1] >>> 0).toString(16) + ':w' + (ring[1 + s * STR + 3 + i * 3 + 2] >>> 0));
    console.log('[ops] slot' + s + ' ' + ops.join(' '));
  }
  if (start >= 0x400d1600 && start <= 0x400d1660 && tlen >= 8) {
    // every pc in range + sane widths + nonzero ops?
    let ok = true;
    for (let i = 0; i < tlen; i++) {
      const p = ring[1 + s * STR + 3 + i * 3] >>> 0;
      const o = ring[1 + s * STR + 3 + i * 3 + 1] >>> 0;
      const w = ring[1 + s * STR + 3 + i * 3 + 2] >>> 0;
      if (p < 0x400d1000 || p > 0x400d2000 || !o || (w !== 2 && w !== 3 && w !== 4)) { ok = false; break; }
    }
    if (ok) { hits++; console.log(`[trace] slot${s} COVERS spin loop`); }
  }
}
if (!hits) { console.log('[trace] FAILED: no spin-loop trace'); process.exit(1); }
console.log('[trace] PASSED');
