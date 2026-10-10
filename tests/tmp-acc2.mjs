// Bit-exact equivalence gate v2 (printf-free): same 100M dual-core loops,
// ACC printed via println + manual nibbles (the lone printf %08x emits
// nothing on ANY engine version — pre-existing quirk, unrelated to cache).
import { ESP32 } from '../src/index.js';
import axios from 'axios';
import { readFileSync, existsSync, writeFileSync } from 'fs';
import { resolve, dirname } from 'path';
import { fileURLToPath } from 'url';

const __dirname = dirname(fileURLToPath(import.meta.url));
const ROM = readFileSync(resolve(__dirname, 'rom/esp32-v3-rom.bin'));
const WASM = readFileSync(process.env.WASM_BIN || resolve(__dirname, '../src/engine/esp-xtensa/esp_engine_wasm.wasm'));
const FWBIN = resolve(__dirname, 'tmp-acc-fw.bin');
const EXPECT = 'f0d809784e93ba21';

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
  char buf[17];
  const char *hex = "0123456789abcdef";
  for (int i = 0; i < 8; i++) buf[i] = hex[(acc0 >> (28 - i * 4)) & 15];
  for (int i = 0; i < 8; i++) buf[8 + i] = hex[(acc1 >> (28 - i * 4)) & 15];
  buf[16] = 0;
  Serial.print("ACC=");
  Serial.println(buf);
  Serial.println("SPEEDDONE");
}
void loop() { delay(1000); }
`;

let bin;
if (existsSync(FWBIN) && !process.env.RECOMPILE) { bin = readFileSync(FWBIN); console.log('[fw] cached'); }
else { console.log('[compile] building oracle...'); bin = await compile(FW); writeFileSync(FWBIN, bin); }
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
  if (Date.now() - t0 > 600000) throw new Error('timeout; tail:\n' + uart.slice(-500));
}
const m = uart.match(/ACC=([0-9a-fA-F]+)/);
console.log(`[acc2] wall ${((Date.now() - t0) / 1000).toFixed(0)}s ACC=${m?.[1] ?? 'MISSING'} expect ${EXPECT}`);
if ((m?.[1] ?? '').toLowerCase() !== EXPECT) { console.log('[acc2] FAILED'); process.exit(1); }
console.log('[acc2] PASSED');
