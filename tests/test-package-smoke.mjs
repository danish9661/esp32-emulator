// Consumer smoke test for the `esp32emu` package.
// Validates two things:
//   A) The package entry (src/index.js) boots the ESP32 directly via the
//      ESP32 class — proving the WASM engine + bundled boot ROM work end to
//      end (no compile server needed; ROM-only boot).
//   B) The high-level SimulatorWorker auto-loads the bundled WASM + boot ROM
//      (no rom/wasm passed in) — proving the published file layout resolves
//      from the package root, i.e. `npm install esp32emu` works out of the box.
import { ESP32, SimulatorWorker } from '../src/index.js';
import { readFileSync } from 'fs';
import { join, dirname } from 'path';
import { fileURLToPath } from 'url';

const __dirname = dirname(fileURLToPath(import.meta.url));

const WASM_PATH = join(__dirname, '..', 'src', 'engine', 'esp-xtensa', 'esp_engine_wasm.wasm');
const ROM_PATH = join(__dirname, '..', 'src', 'rom', 'esp32-v3-rom.bin');

const sleep = (ms) => new Promise((r) => setTimeout(r, ms));

function makeBootHeaderFlash() {
  const flash = new Uint8Array(4 * 1024 * 1024);
  flash.fill(0xff);
  flash[0x1000] = 0xe9; flash[0x1001] = 1; flash[0x1002] = 0x20; flash[0x1003] = 0x40;
  flash[0x1004] = 0x00; flash[0x1005] = 0x00; flash[0x1006] = 0x00; flash[0x1007] = 0x40;
  flash[0x100c] = 0x00; flash[0x100d] = 0x00; flash[0x100e] = 0x00; flash[0x100f] = 0x00;
  flash[0x1014] = 0x00; flash[0x1015] = 0x00; flash[0x1016] = 0x00; flash[0x1017] = 0x40;
  flash[0x1018] = 0x04; flash[0x1019] = 0x00; flash[0x101a] = 0x00; flash[0x101b] = 0x00;
  flash[0x101c] = 0x00; flash[0x101d] = 0x00; flash[0x101e] = 0x00; flash[0x101f] = 0x00;
  flash[0x1020] = 0xef;
  return flash;
}

function assert(cond, msg) {
  if (!cond) { console.error('[test] FAILED: ' + msg); process.exit(1); }
  console.log('[test] ok: ' + msg);
}

async function partA_directBoot() {
  console.log('\n=== Part A: direct ESP32 boot (package entry) ===');
  const rom = readFileSync(ROM_PATH);
  const wasmBytes = readFileSync(WASM_PATH);
  const flash = makeBootHeaderFlash();

  const esp32 = new ESP32({ flashSizeMB: 4, flash });
  esp32.loadROM(rom);
  if (esp32.gpio?.pins) {
    if (esp32.gpio.pins[0]) esp32.gpio.pins[0].inputValue = true;
    if (esp32.gpio.pins[2]) esp32.gpio.pins[2].inputValue = false;
    if (esp32.gpio.pins[12]) esp32.gpio.pins[12].inputValue = false;
    if (esp32.gpio.pins[15]) esp32.gpio.pins[15].inputValue = false;
  }
  esp32.reset();
  esp32.flash.set(flash);
  if (esp32.gpio) esp32.gpio.strapValue = 0x13;
  if (esp32.mmuTablePro) {
    for (let p = 0; p < 64; p++) { esp32.mmuTablePro[p] = p; esp32.mmuTableApp[p] = p; }
  }

  const loaded = await esp32.loadWasm(wasmBytes, 'wasm');
  assert(loaded, 'ESP32.loadWasm() loaded the bundled wasm');

  const startPC = esp32.cores[0].PC >>> 0;
  for (let b = 0; b < 5; b++) esp32.step();
  const endPC = esp32.cores[0].PC >>> 0;

  assert(startPC === 0x40000400 || startPC >= 0x40000000, `core0 started in ROM (PC=0x${startPC.toString(16)})`);
  assert(endPC !== startPC, `core0 executed instructions (PC 0x${startPC.toString(16)} -> 0x${endPC.toString(16)})`);
  console.log(`[test] Part A PASSED (booted ROM, PC advanced to 0x${endPC.toString(16)})`);
}

async function partB_workerAutoLoad() {
  console.log('\n=== Part B: SimulatorWorker auto-loads bundled wasm+rom ===');
  const flash = makeBootHeaderFlash();
  const proxy = new SimulatorWorker();
  let errored = null;
  proxy._onError = (e) => { errored = e; };
  let uart = '';
  proxy._onUART = (b) => { uart += String.fromCharCode(b); };

  // No rom / no wasm passed -> worker must resolve them from the package bundle.
  await proxy.init('ESP32', { flashSizeMB: 4, mmuPages: 64, strapValue: 0x13, budget: 2000000 }, flash);
  assert(proxy._ready === true, 'SimulatorWorker.init() resolved (bundled files found)');

  proxy.run();
  for (let i = 0; i < 25; i++) {
    await sleep(100);
    proxy.pollUart();
    if (errored) break;
  }

  assert(!errored, 'SimulatorWorker produced no error during boot' + (errored ? (': ' + errored.message) : ''));
  // SAB_SLOT_ERR_FLAG = 13
  const errSlot = Atomics.load(proxy.ctrl, 13);
  assert(errSlot === 0, 'worker error flag is clear (no load/runtime failure)');

  const pc0 = proxy.debug[0] >>> 0;
  assert(pc0 >= 0x40000000, `core0 PC is in ROM region (PC=0x${pc0.toString(16)})`);
  console.log(`[test] Part B PASSED (auto-loaded bundle, core0 PC=0x${pc0.toString(16)})`);
  proxy.terminate?.();
}

(async () => {
  try {
    await partA_directBoot();
    await partB_workerAutoLoad();
    console.log('\n[test] ALL PACKAGE SMOKE TESTS PASSED');
    process.exit(0);
  } catch (e) {
    console.error('\n[test] FAILED:', e && e.stack ? e.stack : e);
    process.exit(1);
  }
})();
