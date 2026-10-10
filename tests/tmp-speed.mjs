// Speed audit: current engine throughput + threading-model comparison.
// Diag only (untracked). Measures, on identical dual-core busy firmware:
//   T1 raw main-thread, no SAB (engine ceiling)
//   T2 single worker, SAB on (production path: both cores, one thread)
//   T3/T4 2 and 4 workers in parallel (per-chip threading/scaling)
//   T5 micro: core_run(512) vs 512 core_step pairs (FFI batching cost —
//      the cost a per-core-thread split would pay per batch)
// Retired instructions come from the SAB-backed instCount views (real
// executed instructions, both cores). Worker-path MIPS reuse the raw-path
// retired count for the identical binary (deterministic emulator).
import { ESP32, SimulatorWorker } from '../src/index.js';
import axios from 'axios';
import { readFileSync } from 'fs';
import { resolve, dirname } from 'path';
import { fileURLToPath } from 'url';

const __dirname = dirname(fileURLToPath(import.meta.url));
const ROM = readFileSync(resolve(__dirname, 'rom/esp32-v3-rom.bin'));
const WASM = readFileSync(resolve(__dirname, '../src/engine/esp-xtensa/esp_engine_wasm.wasm'));

async function compile(code) {
  const r = await axios.post('http://localhost:5525/api/compile/start', { code, target: 'esp32', targetEngine: 'frontend', fqbn: 'esp32:esp32:esp32' });
  while (true) {
    const s = await axios.get(`http://localhost:5525/api/compile/status/${r.data.buildId}`);
    if (s.data.status === 'success') return Buffer.from(s.data.binary_content, 'base64');
    if (s.data.status === 'failed') throw new Error('Compile failed: ' + s.data.error);
    await new Promise(r => setTimeout(r, 1000));
  }
}

// Dual-core busy firmware: 100M dependent-imul iterations on core 1 (loop)
// + 100M on core 0 (pinned task), then DONE. delay(1) every 1M iterations
// keeps IDLE scheduled (INT WDT fed) and the task WDT happy — no WDT API
// calls needed. loop() idles after.
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
  Serial.printf("ACC=%08x%08x\\n", (unsigned)acc0, (unsigned)acc1);
  Serial.println("SPEEDDONE");
}
void loop() { delay(1000); }
`;

function makeFlash(binBytes) {
  const flash = new Uint8Array(4 * 1024 * 1024);
  flash.fill(0xff);
  flash.set(binBytes);
  return flash;
}

async function bootRaw(binBytes) {
  const flash = makeFlash(binBytes);
  const chip = new ESP32({ flashSizeMB: 4, flash });
  chip.loadROM(ROM);
  // SPI-boot strapping (copied from wasm-bench — without it the ROM sits
  // in the UART-download spin loop forever: PC 0x4000fca9, 0 UART).
  if (chip.gpio?.pins) {
    if (chip.gpio.pins[0]) chip.gpio.pins[0].inputValue = true;
    if (chip.gpio.pins[2]) chip.gpio.pins[2].inputValue = false;
    if (chip.gpio.pins[12]) chip.gpio.pins[12].inputValue = false;
    if (chip.gpio.pins[15]) chip.gpio.pins[15].inputValue = false;
  }
  chip.reset();
  chip.flash.set(flash);
  if (chip.gpio) chip.gpio.strapValue = 0x13;
  if (chip.mmuTablePro) {
    for (let p = 0; p < 64; p++) { chip.mmuTablePro[p] = p; chip.mmuTableApp[p] = p; }
  }
  try { chip.cores[0].writeUint32(0x3ff5a104, 0x5aa5); } catch {}
  const loaded = await chip.loadWasm(WASM, 'wasm');
  if (!loaded) throw new Error('loadWasm failed');
  let uart = '';
  chip.uart[0].onTX = (b) => { uart += String.fromCharCode(b); };
  const getUart = () => uart;
  return { chip, ex: chip._wasmLoader.exports, getUart };
}

// Mirror of the worker loop body (runSimChunk): step, fire the JS clock
// tree (ccompare/FreeRTOS tick), then idle-advance or sync+pump. A bare
// chip.step() loop wedges twice over: first on unpumped timer/UART events
// (PC 0x4000fca9, 0 UART), then parked pre-tick with 0 retired instructions.
let fireDueFn = null;
function rawStep(chip, ex) {
  chip.step();
  try {
    if (!fireDueFn) {
      const root = chip.clocks?.root;
      fireDueFn = root?.fireDueEvents?.bind(root) || null;
    }
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

const instSum = (chip) => (chip._wasmCores[0].instCount >>> 0) + (chip._wasmCores[1].instCount >>> 0);
const sleep = (ms) => new Promise(r => setTimeout(r, ms));

async function main() {
  console.log('[compile] building dual-core busy firmware...');
  const tC0 = Date.now();
  const bin = await compile(FW);
  console.log(`[compile] ok ${(bin.length / 1024).toFixed(0)}KB in ${((Date.now() - tC0) / 1000).toFixed(0)}s`);

  // ---- T5 micro: batching cost (fresh boot, dense ROM code) ----
  {
    const { chip } = await bootRaw(bin);
    const N = 300;
    let t0 = process.hrtime.bigint();
    for (let i = 0; i < N; i++) chip.step(); // core_run(512): both cores, 1 FFI
    let t1 = process.hrtime.bigint();
    const runUs = Number(t1 - t0) / 1000;
    const { chip: c2 } = await bootRaw(bin);
    t0 = process.hrtime.bigint();
    for (let i = 0; i < N; i++) {
      for (let j = 0; j < 512; j++) {
        c2._wasmCores[0].runInstruction();
        if (c2.cores[1]?.enabled) c2._wasmCores[1].runInstruction();
      }
      c2.cycles += 512;
    }
    t1 = process.hrtime.bigint();
    const pairUs = Number(t1 - t0) / 1000;
    console.log(`[T5] core_run(512)x${N}: ${(runUs / 1000).toFixed(1)}ms | 512x core_step pairs x${N}: ${(pairUs / 1000).toFixed(1)}ms | ratio ${(pairUs / runUs).toFixed(1)}x`);
  }

  // ---- T1 raw, no SAB ----
  let INST = 0;
  {
    const { chip, ex, getUart } = await bootRaw(bin);
    const t0 = Date.now();
    let lastPrint = 0;
    while (true) {
      rawStep(chip, ex);
      if (getUart().includes('SPEEDDONE')) break;
      const now = Date.now();
      if (now - lastPrint > 30000) {
        lastPrint = now;
        console.log(`[T1] ... ${(instSum(chip) / 1e6).toFixed(0)}M inst, uart ${getUart().length}B, pc0=0x${(chip._wasmCores[0].PC >>> 0).toString(16)}`);
      }
      if (instSum(chip) > 3e9 || now - t0 > 480000) {
        console.log('[T1] UART tail:\n' + getUart().slice(-1500));
        throw new Error('T1 timeout');
      }
    }
    const wall = (Date.now() - t0) / 1000;
    INST = instSum(chip);
    console.log(`[T1] raw main-thread: ${(INST / 1e6).toFixed(1)}M inst in ${wall.toFixed(1)}s = ${(INST / wall / 1e6).toFixed(1)} MIPS`);
  }

  // ---- T2/T3/T4 workers ----
  async function runWorkers(n) {
    const proxies = [];
    for (let i = 0; i < n; i++) {
      const flash = new Uint8Array(new SharedArrayBuffer(4 * 1024 * 1024));
      flash.set(bin);
      const proxy = new SimulatorWorker();
      let out = '';
      proxy._onUART = (b) => { out += String.fromCharCode(b); };
      proxy._onError = (e) => console.error('[worker] Error:', e.message);
      await proxy.init('ESP32', { flashSizeMB: 4, mmuPages: 64, strapValue: 0x13, budget: 5000000 }, flash, ROM);
      proxies.push({ proxy, has: () => out.includes('SPEEDDONE'), getOut: () => out });
    }
    const t0 = Date.now();
    for (const p of proxies) p.proxy.run();
    while (true) {
      await sleep(200);
      for (const p of proxies) p.proxy.pollUart();
      if (proxies.every(p => p.has())) break;
      if (Date.now() - t0 > 480000) {
        console.log(`[T${n + 1}] UART tail:\n` + proxies[0].getOut().slice(-1500));
        throw new Error(`T${n} timeout`);
      }
    }
    const wall = (Date.now() - t0) / 1000;
    const perChip = INST / wall / 1e6;
    console.log(`[T${n + 1}] ${n} worker(s) SAB-on: ${wall.toFixed(1)}s wall to DONE | per-chip ${perChip.toFixed(1)} MIPS | total ${(perChip * n).toFixed(1)} MIPS | virtual ${(proxies[0].proxy.nanos / 1e9 / wall).toFixed(1)}x wall`);
    for (const p of proxies) { p.proxy.stop(); await sleep(50); p.proxy.terminate(); }
  }
  await runWorkers(1); // T2
  await runWorkers(2); // T3
  await runWorkers(4); // T4
  console.log('[done]');
}

main().catch(e => { console.error(e); process.exit(1); });
