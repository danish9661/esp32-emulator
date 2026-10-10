// Speed knobs: batch-size and pump-cadence sensitivity on live firmware.
// Boots once, runs to SPEEDGO, then times fixed instruction windows.
// All cells keep JS-clock firing per batch (tick delivery unchanged);
// only the WASM batch size and native pump cadence vary.
import { ESP32 } from '../src/index.js';
import axios from 'axios';
import { readFileSync, existsSync, writeFileSync } from 'fs';
import { resolve, dirname } from 'path';
import { fileURLToPath } from 'url';

const __dirname = dirname(fileURLToPath(import.meta.url));
const ROM = readFileSync(resolve(__dirname, 'rom/esp32-v3-rom.bin'));
const WASM = readFileSync(process.env.WASM_BIN || resolve(__dirname, '../src/engine/esp-xtensa/esp_engine_wasm.wasm'));
const FWBIN = resolve(__dirname, 'tmp-speed-fw.bin');

async function compile(code) {
  const r = await axios.post('http://localhost:5525/api/compile/start', { code, target: 'esp32', targetEngine: 'frontend', fqbn: 'esp32:esp32:esp32' });
  while (true) {
    const s = await axios.get(`http://localhost:5525/api/compile/status/${r.data.buildId}`);
    if (s.data.status === 'success') return Buffer.from(s.data.binary_content, 'base64');
    if (s.data.status === 'failed') throw new Error('Compile failed: ' + s.data.error);
    await new Promise(r => setTimeout(r, 1000));
  }
}

const FW = `
volatile uint32_t acc0 = 0, acc1 = 0;
volatile bool c0done = false;
void spin0(void*) {
  uint32_t a = 0x12345678;
  for (int i = 0; i < 100000000; i++) {
    a = a * 1103515245u + 12345u;
    if ((i % 1000000) == 0) delay(1);
  }
  acc0 = a; c0done = true;
  vTaskDelete(NULL);
}
void setup() {
  Serial.begin(115200);
  Serial.println("SPEEDGO");
  xTaskCreatePinnedToCore(spin0, "spin0", 4096, NULL, 1, NULL, 0);
  uint32_t a = 0x87654321;
  for (int i = 0; i < 100000000; i++) {
    a = a * 1103515245u + 12345u;
    if ((i % 1000000) == 0) delay(1);
  }
  acc1 = a;
  while (!c0done) { delay(1); }
  Serial.println("SPEEDDONE");
}
void loop() { delay(1000); }
`;

let fireDueFn = null;
function fullStep(chip, ex, batch) {
  ex.core_run(batch);
  chip.cycles += batch;
  // chip.step() bookkeeping that the engine reads back: Rust sab_ticks()/
  // sab_cycles() read these SAB slots for CCOUNT. Skipping them freezes
  // CCOUNT at 0 and the scheduler tick never fires (wedge pre-SPEEDGO).
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

async function main() {
  let bin;
  if (existsSync(FWBIN)) { bin = readFileSync(FWBIN); console.log('[fw] cached'); }
  else {
    console.log('[compile] building...');
    bin = await compile(FW);
    writeFileSync(FWBIN, bin);
    console.log('[compile] ok, cached');
  }
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
  let uart = '';
  chip.uart[0].onTX = (b) => { uart += String.fromCharCode(b); };
  console.log('[boot] waiting for SPEEDGO...');
  const tB0 = Date.now();
  while (!uart.includes('SPEEDGO')) {
    fullStep(chip, ex, 512);
    if (Date.now() - tB0 > 900000) throw new Error('boot timeout');
  }
  console.log(`[boot] SPEEDGO in ${((Date.now() - tB0) / 1000).toFixed(0)}s, both cores spinning`);
  const inst0 = () => (chip._wasmCores[0].instCount >>> 0) + (chip._wasmCores[1].instCount >>> 0);

  async function window(label, batches, batch, pumpEvery) {
    const i0 = inst0();
    const t0 = process.hrtime.bigint();
    for (let i = 0; i < batches; i++) {
      ex.core_run(batch);
      chip.cycles += batch;
      try {
        if (!chip._sabU32) chip._sabU32 = new Uint32Array(chip._wasmMemory || chip._wasmMemoryObj?.buffer);
        chip._sabU32[998] = chip.clocks.cpu.ticks >>> 0;
        chip._sabU32[999] = chip.cycles >>> 0;
      } catch {}
      try { if (fireDueFn) fireDueFn(); } catch {}
      if ((i % pumpEvery) === 0) {
        let idle = false;
        try { idle = !!chip.coresIdle; } catch {}
        if (idle && ex?.native_idle_advance) {
          try { chip.cycles += ex.native_idle_advance(chip.cycles >>> 0); } catch {}
        } else {
          try { ex?.native_set_clock_state?.(chip.cycles >>> 0); } catch {}
          try { ex?.native_pump_events?.(); } catch {}
        }
      }
      if (uart.includes('SPEEDDONE')) break;
    }
    const us = Number(process.hrtime.bigint() - t0) / 1000;
    const di = inst0() - i0;
    console.log(`[${label}] batch=${batch} pumpEvery=${pumpEvery}: ${(di / 1e6).toFixed(1)}M inst in ${(us / 1e6).toFixed(1)}s = ${(di / us).toFixed(1)} MIPS`);
  }

  await window(process.env.WINTAG || 'A1', 1500, 512, 1);
  console.log('[done] uart DONE=' + uart.includes('SPEEDDONE'));
}

main().catch(e => { console.error(e); process.exit(1); });
