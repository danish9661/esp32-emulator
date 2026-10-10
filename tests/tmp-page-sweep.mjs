// Loop-slice throughput: find a loop-closed slice in a recorded trace,
// prove it bit-exact over several iterations, then measure JIT MIPS vs
// interpreter MIPS on the same dynamic code.
import { ESP32 } from '../src/index.js';
import { readFileSync, existsSync } from 'fs';
import { resolve, dirname } from 'path';
import { fileURLToPath } from 'url';
import { buildTraceModule, instantiateTrace, findLoopSlices, classify } from './tmp-jit-lib.mjs';
const __dirname = dirname(fileURLToPath(import.meta.url));

const ROM = readFileSync(resolve(__dirname, 'rom/esp32-v3-rom.bin'));
const WASM = readFileSync(resolve(__dirname, '../src/engine/esp-xtensa/esp_engine_wasm.wasm'));
const FWBIN = process.env.FW_BIN || resolve(__dirname, 'tmp-speed-fw.bin');
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
const L = chip._wasmLoader;
for (const fn of ['native_jit_l32r', 'native_jit_ssai', 'native_code_gen', 'native_trace_len', 'native_trace_scratch_ptr', 'native_trace_scratch_len', 'native_trace_threshold', 'core_get_idle']) {
  if (typeof ex[fn] !== 'function') throw new Error('missing export ' + fn + ' (rebuild with jit call-outs?)');
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
while (!uart.includes('SPEEDGO')) { step(); if (Date.now() - tB0 > 600000) throw new Error('boot timeout'); }
console.log('[loop] SPEEDGO');
const CORE = (ex.core_get_idle(0) >>> 0) === 0 ? 0 : 1;
const CBASE = CORE * 4096;
const memU32 = () => new Uint32Array(L.memory.buffer);
const wc = L._wasmCores[CORE];
const SPEC = (CBASE + 336) >>> 2, INSTW = (CBASE + 2608) >>> 2, PENDW = (CBASE + 2540) >>> 2;
const spec = () => Array.from(memU32().slice(SPEC, SPEC + 256));
import { buildPage } from './tmp-jit-lib.mjs';
console.log('[sweep] SPEEDGO (booted above)');
ex.native_trace_enable(0);
// ---- M2: linear sweep of the hot 4KB page, thread ALL of it ----
// Widths authoritative: INST_WIDTH_TABLE[lo] (engine constants.rs).
const WIDTH = [3, 3, 3, 3, 3, 3, 3, 3, 2, 2, 2, 2, 2, 2, 4, 4];
// pages from the recorder's hot traces (what actually burns cycles)
ex.native_trace_threshold(5000);
for (let i = 0; i < 600; i++) step();
const n = ex.native_trace_len() >>> 0;
const ptr = ex.native_trace_scratch_ptr() >>> 0;
const ring = memU32().slice(ptr >>> 2, (ptr >>> 2) + (ex.native_trace_scratch_len() >>> 0));
const STR = (ring.length - 1) / 16;
const pages = new Set();
for (let sl = 0; sl < 16; sl++) {
  const st = ring[1 + sl * STR + 1] >>> 0, ln = ring[1 + sl * STR + 2] & 0xFFFF;
  if (ln < 8) continue;
  pages.add(((st >>> 12) << 12) >>> 0);
}
console.log('[sweep] hot pages: ' + [...pages].map((p) => '0x' + p.toString(16)).join(' '));
const GH = {};
let Gtot = 0, Gunk = 0;
for (const pageBase of pages) {
  const bytes = new Uint8Array(4096);
  for (let i = 0; i < 4096; i++) bytes[i] = ex.core_read_uint8(CORE, pageBase + i) >>> 0;
// linear sweep: decode every instruction top-down
const ops = [];
{
  let a = 0;
  while (a < 4096) {
    const b0 = bytes[a] | (bytes[a + 1] << 8);
    const w = WIDTH[b0 & 15];
    if (w === 4) { a += 4; continue; } // wide: skip (interpreter covers)
    let op = b0;
    if (w === 3) op |= bytes[a + 2] << 16;
    ops.push({ pc: pageBase + a, op: op >>> 0, w });
    a += w;
  }
}
console.log('[sweep] page 0x' + pageBase.toString(16) + ': ' + ops.length + ' narrow ops');
// classify coverage census
const hist = {};
for (const e of ops) {
  const k = classify(e.op).k;
  hist[k] = (hist[k] || 0) + 1;
}
const top = Object.entries(hist).sort((x, y) => y[1] - x[1]).slice(0, 12);
console.log('[sweep] shape census: ' + top.map(([k, n]) => `${k}=${n}`).join(' '));
  globalThis.__lastOps = ops;
  for (const [k, nn] of Object.entries(hist)) GH[k] = (GH[k] || 0) + nn;
  Gtot += ops.length; Gunk += hist['unknown'] || 0;
}
const top = Object.entries(GH).sort((x, y) => y[1] - x[1]).slice(0, 14);
console.log('[sweep] UNION shapes: ' + top.map(([k, nn]) => `${k}=${nn}`).join(' '));
console.log(`[sweep] UNION unknown=${Gunk}/${Gtot} (${(100 * Gunk / Gtot).toFixed(1)}%)`);
process.exit(0);
