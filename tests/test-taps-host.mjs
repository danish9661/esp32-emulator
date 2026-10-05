// Host peripheral taps (esp32-emu.md §1-4) — no compile server needed.
// Part A (pure JS, no WASM): tap routing/NACK policy, preloads, queues,
//   GPIO queue/listeners, reset semantics.
// Part B (WASM+ROM, direct ESP32, MMIO-driven masters — no firmware):
//   native export presence, registry sync, GPIO drive → readback + events,
//   I2C master WRITE/READ/NACK via COMD programming, SPI transfer echo.
// Part C (Worker, ROM-only): SAB plumbing for all 11 tap CMDs (no deadlock,
//   sane values).
import { ESP32, SimulatorWorker, I2cTap, SpiTap } from '../src/index.js';
import { readFileSync } from 'fs';
import { resolve, dirname } from 'path';
import { fileURLToPath } from 'url';

const __dirname = dirname(fileURLToPath(import.meta.url));
let pass = 0, fail = 0;
function assert(cond, msg) {
  if (cond) { pass++; console.log(`  ok: ${msg}`); }
  else { fail++; console.error(`  FAIL: ${msg}`); }
}

// I2C0 MMIO map (offsets from i2c_i2s.rs / native_mmio.rs)
const I2C0 = 0x3ff53000;
const I2C_OF = 28, I2C_CTR = 4, I2C_COMD = 88, I2C_STATUS = 8, I2C_INT_RAW = 32;
const I2C_CTR_EXEC = 0x20; // bit5
const I2C_OP_RSTART = 0, I2C_OP_WRITE = 1, I2C_OP_READ = 2, I2C_OP_STOP = 3;
const I2C_INT_TRANS_COMPLETE = 7, I2C_INT_ACK_ERR = 10;
// SPI2 (HSPI, user bus) MMIO map
const SPI2 = 0x3ff64000;
const SPI_CMD = 0, SPI_USER = 28, SPI_MOSI_DLEN = 40, SPI_MISO_DLEN = 44, SPI_W0 = 128;
const SPI_USR_BIT = 1 << 18, SPI_USER_MOSI = 0x08000000, SPI_USER_MISO = 0x10000000;
// GPIO MMIO map
const GPIO_BASE = 0x3ff44000;
const GPIO_OUT = 4, GPIO_ENABLE = 32;

async function partA() {
  console.log('\n=== Part A: tap objects (no WASM) ===');
  const fakeChip = {}; // taps must work pre-load (sync deferred)
  const i2c = new I2cTap(fakeChip, 0);
  assert(i2c.bus === 0 && i2c.slaves.size === 0, 'I2cTap constructs empty');

  // Unregistered → NACK at every phase
  assert(i2c._ffiStart(0x3c, false) === 0, 'unregistered addr NACKs on start');
  assert(i2c._ffiWrite(0x41) === 0, 'unregistered write NACKs');
  assert(i2c._ffiRead() === 0xff, 'unregistered read returns 0xFF');

  // Attach → ACK, write lands, read served from preload
  i2c.attachSlave(0x3c);
  assert(i2c._ffiStart(0x3c, false) === 1, 'registered addr ACKs');
  assert(i2c._ffiWrite(0x41) === 1 && i2c._ffiWrite(0x42) === 1, 'registered writes ACK');
  assert(JSON.stringify(i2c.drainWriteLog(0x3c)) === '[65,66]', 'writeLog captures bytes');
  assert(i2c.drainWriteLog(0x3c).length === 0, 'drain clears the log');
  i2c.preloadRead(0x3c, [0xab, 0xcd]);
  assert(i2c._ffiStart(0x3c, true) === 1, 'registered read addr ACKs');
  assert(i2c._ffiRead() === 0xab && i2c._ffiRead() === 0xcd, 'preloaded reads served in order');
  assert(i2c._ffiRead() === 0xff, 'exhausted preload underruns 0xFF');
  i2c._ffiStop(true);

  // Per-slave model functions
  let sawStop = null;
  i2c.attachSlave(0x42, {
    onWrite: (b) => b !== 0x00, // NACK zero bytes
    onRead: () => 0x55,
    onStop: (r) => { sawStop = r; },
  });
  assert(i2c._ffiStart(0x42, false) === 1, 'model slave ACKs start');
  assert(i2c._ffiWrite(0x01) === 1, 'model ACKs nonzero byte');
  assert(i2c._ffiWrite(0x00) === 0, 'model NACKs zero byte');
  i2c._ffiStop(false);
  assert(sawStop === false, 'model onStop fires with wasRead=false');
  assert(i2c._ffiStart(0x42, true) === 1 && i2c._ffiRead() === 0x55, 'model onRead serves 0x55');

  // Bus-level override (F1 style): explicit false forces NACK on registered
  i2c.onStart = () => false;
  assert(i2c._ffiStart(0x3c, false) === 0, 'bus onStart=false forces NACK');
  // Bus-level claim of unregistered address
  i2c.onStart = (addr) => addr === 0x3d;
  assert(i2c._ffiStart(0x3d, false) === 1, 'bus onStart can claim unregistered addr');
  i2c.onStart = null;
  i2c.detachSlave(0x3c);
  i2c.detachSlave(0x42);
  assert(i2c._ffiStart(0x3c, false) === 0, 'detached addr NACKs again');

  // Events staged for async draining (drain stale ones from the checks above first)
  i2c.pollEvents();
  i2c.attachSlave(0x3c);
  i2c._ffiStart(0x3c, false); i2c._ffiWrite(0x99); i2c._ffiStop(false);
  const evs = i2c.pollEvents();
  assert(evs.length === 3 && evs[0].t === 'start' && evs[1].t === 'write' && evs[2].t === 'stop', 'events staged in order');
  assert(i2c.pollEvents().length === 0, 'pollEvents drains');

  // Reset: wiring + preloads survive, flight clears
  i2c.preloadRead(0x3c, [0x11]);
  i2c._ffiStart(0x3c, false);
  i2c._resetFlight();
  assert(i2c.slaves.has(0x3c), 'reset keeps registrations');
  assert(i2c.events.length === 0, 'reset clears staged events');
  assert(i2c._ffiStart(0x3c, true) === 1 && i2c._ffiRead() === 0x11, 'reset keeps preloads');
  i2c.pollEvents();

  // SPI tap
  const spi = new SpiTap(fakeChip, 2);
  assert(spi.bus === 2, 'SpiTap constructs');
  assert(JSON.stringify(spi._ffiTransfer([1, 2, 3], 3)) === '[]', 'unclaimed transfer returns []');
  spi.onTransfer = (tx) => tx.map((b) => b ^ 0xff);
  assert(JSON.stringify(spi._ffiTransfer([0x00, 0xff], 2)) === '[255,0]', 'onTransfer echo works');
  spi.onTransfer = null;
  spi.injectMiso([0xde, 0xad]);
  assert(JSON.stringify(spi._ffiTransfer([9, 9, 9], 2)) === '[222,173]', 'injectMiso preload served');
  assert(JSON.stringify(spi.drainMosi()) === '[1,2,3,0,255,9,9,9]', 'mosiLog captures all MOSI');
  assert(spi.drainMosi().length === 0, 'drainMosi clears');
  spi.injectMiso([0x11]);
  spi._resetFlight();
  assert(JSON.stringify(spi._ffiTransfer([0], 1)) === '[17]', 'reset keeps MISO preload');
}

function makeChip() {
  const chip = new ESP32({ flashSizeMB: 4 });
  assert(chip.i2c0 instanceof I2cTap && chip.i2c1 instanceof I2cTap, 'chip exposes i2c taps');
  assert(Array.isArray(chip.i2c) && chip.i2c.length === 2, 'chip exposes i2c array');
  assert(chip.spi2 instanceof SpiTap && Array.isArray(chip.spi) && chip.spi.length === 4, 'chip exposes spi taps');
  return chip;
}

async function bootChip(chip) {
  const rom = readFileSync(resolve(__dirname, '../src/rom/esp32-v3-rom.bin'));
  const wasmBytes = readFileSync(resolve(__dirname, '../src/engine/esp-xtensa/esp_engine_wasm.wasm'));
  chip.loadROM(rom);
  chip.reset();
  const loaded = await chip.loadWasm(wasmBytes, 'wasm');
  assert(loaded, 'loadWasm ok');
  return chip._wasmLoader.exports;
}

async function partB() {
  console.log('\n=== Part B: engine hooks via MMIO (WASM+ROM, no firmware) ===');
  const chip = makeChip();
  const ex = await bootChip(chip);

  // Export presence (14 tap symbols + diag write half of the MMIO backdoor)
  for (const name of [
    'native_i2c_set_host_slave', 'native_i2c_set_promisc', 'native_i2c_host_claimed',
    'native_spi_set_transfer_hook', 'native_spi_xfer_mosi_ptr', 'native_spi_xfer_miso_ptr',
    'native_gpio_get_output', 'native_gpio_get_direction', 'native_gpio_get_pull',
    'native_gpio_set_change_hook',
    'native_ledc_channel_count', 'native_ledc_get_duty', 'native_ledc_get_timer', 'native_ledc_get_freq',
    'native_diag_write',
  ]) {
    assert(typeof ex[name] === 'function', `export ${name} present`);
  }

  // Registry sync (JS attach → Rust query), incl. pre-load registration:
  // attach BEFORE boot and verify _applyTapHooks synced it at loadWasm.
  {
    const c0 = makeChip();
    c0.i2c1.attachSlave(0x22); // pre-load registration (no WASM yet)
    const ex0 = await bootChip(c0);
    assert(ex0.native_i2c_host_claimed(1, 0x22) === 1, 'pre-load attach synced at loadWasm');
  }
  chip.i2c0.attachSlave(0x3c);
  assert(ex.native_i2c_host_claimed(0, 0x3c) === 1, 'registry syncs attach to Rust');
  assert(ex.native_i2c_host_claimed(0, 0x3d) === 0, 'unregistered reads 0');
  assert(ex.native_i2c_host_claimed(1, 0x3c) === 0, 'registry is per-bus');
  chip.i2c0.detachSlave(0x3c);
  assert(ex.native_i2c_host_claimed(0, 0x3c) === 0, 'detach syncs to Rust');

  // GPIO drive → readback + events via the MMIO backdoor (native_diag_write:
  // the JS facade writeUint32 cannot reach native pages — real guest path).
  const HID_GPIO = 5, HID_I2C = 10, HID_SPI = 11;
  const dw = (hid, a, v) => ex.native_diag_write(hid, a, v >>> 0, 32);
  const dr = (hid, a) => ex.native_diag_read(hid, a, 32) >>> 0;
  const edges = [];
  const unsub = chip.onGpioOutChange((changes) => edges.push(...changes));
  dw(HID_GPIO, GPIO_BASE + GPIO_ENABLE, 1 << 2);
  dw(HID_GPIO, GPIO_BASE + GPIO_OUT, 1 << 2);
  assert(chip.getGpioOut(2) === 1, 'getGpioOut(2) reads driven HIGH');
  assert(chip.getGpioDir(2) === 1, 'getGpioDir(2) reads out');
  assert(chip.getGpioPull(2) === 0, 'getGpioPull(2) reads none');
  assert(chip.getGpioOut(4) === 0 && chip.getGpioDir(4) === 0, 'untouched pin reads in/low');
  const samp = chip.sampleGpioOut();
  assert(samp.levels.length === 40 && samp.levels[2] === 1, 'sampleGpioOut snapshot shape+value');
  dw(HID_GPIO, GPIO_BASE + GPIO_OUT, 0);
  assert(chip.getGpioOut(2) === 0, 'getGpioOut(2) follows drive LOW');
  const polled = chip.pollGpioChanges();
  assert(polled.some((e) => e.pin === 2 && e.level === 1), 'change queue saw pin2 rising edge');
  assert(polled.some((e) => e.pin === 2 && e.level === 0), 'change queue saw pin2 falling edge');
  assert(edges.some((e) => e.pin === 2 && e.level === 1), 'onGpioOutChange listener fired');
  unsub();
  chip.reset();
  assert(chip.pollGpioChanges().length === 0, 'reset clears gpio change queue');
  assert(chip.getGpioOut(2) === 0 && chip.getGpioDir(2) === 0, 'reset clears output state');
  assert(chip.cycles === 0, 'reset clears cycle counter');

  // LEDC zero-state readback (duty 0; freq is derived from timer config and
  // reads nonzero garbage until firmware programs the timer — assert shape)
  assert(chip.ledcChannelCount() === 16, 'ledcChannelCount() === 16');
  assert(chip.getLedcDuty(0) === 0, 'unconfigured LEDC duty reads 0');
  assert(Number.isFinite(chip.getLedcFreq(0)), 'unconfigured LEDC freq reads a number');
  assert(chip.getLedcPin(0) === -1, 'unrouted LEDC channel maps to no pin');

  // I2C master WRITE to a host slave (MMIO-programmed, fully synchronous)
  {
    const c2 = makeChip();
    const ex2 = await bootChip(c2);
    c2.i2c0.attachSlave(0x3c);
    const w = (a, v) => ex2.native_diag_write(HID_I2C, a, v >>> 0, 32);
    const r = (a) => ex2.native_diag_read(HID_I2C, a, 32) >>> 0;
    w(I2C0 + I2C_OF, 0x78); // addr 0x3c + W
    w(I2C0 + I2C_OF, 0x41);
    w(I2C0 + I2C_OF, 0x42);
    w(I2C0 + I2C_COMD + 0, (I2C_OP_RSTART << 11) | 0);
    w(I2C0 + I2C_COMD + 4, (I2C_OP_WRITE << 11) | (1 << 8) | 3);
    w(I2C0 + I2C_COMD + 8, (I2C_OP_STOP << 11) | 0);
    w(I2C0 + I2C_CTR, I2C_CTR_EXEC);
    assert(JSON.stringify(c2.i2c0.drainWriteLog(0x3c)) === '[65,66]', 'host slave received AB');
    const raw = r(I2C0 + I2C_INT_RAW);
    assert((raw & (1 << I2C_INT_TRANS_COMPLETE)) !== 0, 'TRANS_COMPLETE set');
    assert((raw & (1 << I2C_INT_ACK_ERR)) === 0, 'no ACK_ERR on claimed addr');
    const evts = c2.i2c0.pollEvents().map((e) => e.t).join(',');
    assert(evts === 'start,write,write,stop', `event order start,write,write,stop (got ${evts})`);
  }

  // I2C NACK on unregistered address
  {
    const c3 = makeChip();
    const ex3 = await bootChip(c3);
    const w = (a, v) => ex3.native_diag_write(HID_I2C, a, v >>> 0, 32);
    const r = (a) => ex3.native_diag_read(HID_I2C, a, 32) >>> 0;
    w(I2C0 + I2C_OF, 0x7a); // addr 0x3d + W (nothing attached)
    w(I2C0 + I2C_COMD + 0, (I2C_OP_RSTART << 11) | 0);
    w(I2C0 + I2C_COMD + 4, (I2C_OP_WRITE << 11) | (1 << 8) | 1);
    w(I2C0 + I2C_COMD + 8, (I2C_OP_STOP << 11) | 0);
    w(I2C0 + I2C_CTR, I2C_CTR_EXEC);
    const raw = r(I2C0 + I2C_INT_RAW);
    assert((raw & (1 << I2C_INT_ACK_ERR)) !== 0, 'ACK_ERR set on unregistered addr (NACK)');
  }

  // I2C master READ from preloaded host slave
  {
    const c4 = makeChip();
    const ex4 = await bootChip(c4);
    c4.i2c0.preloadRead(0x3c, [0xab, 0xcd]);
    const w = (a, v) => ex4.native_diag_write(HID_I2C, a, v >>> 0, 32);
    const r = (a) => ex4.native_diag_read(HID_I2C, a, 32) >>> 0;
    w(I2C0 + I2C_OF, 0x79); // addr 0x3c + R
    w(I2C0 + I2C_COMD + 0, (I2C_OP_RSTART << 11) | 0);
    w(I2C0 + I2C_COMD + 4, (I2C_OP_WRITE << 11) | (1 << 8) | 1);
    w(I2C0 + I2C_COMD + 8, (I2C_OP_READ << 11) | 2);
    w(I2C0 + I2C_COMD + 12, (I2C_OP_STOP << 11) | 0);
    w(I2C0 + I2C_CTR, I2C_CTR_EXEC);
    const b0 = r(I2C0 + I2C_OF) & 0xff, b1 = r(I2C0 + I2C_OF) & 0xff;
    assert(b0 === 0xab && b1 === 0xcd, `preloaded read bytes served (got ${b0.toString(16)},${b1.toString(16)})`);
  }

  // SPI transfer with onTransfer echo (unit 2 = SPI2/HSPI)
  {
    const c5 = makeChip();
    const ex5 = await bootChip(c5);
    c5.spi2.onTransfer = () => [0x57, 0x58, 0x59, 0x5a];
    const w = (a, v) => ex5.native_diag_write(HID_SPI, a, v >>> 0, 32);
    const r = (a) => ex5.native_diag_read(HID_SPI, a, 32) >>> 0;
    w(SPI2 + SPI_USER, SPI_USER_MOSI | SPI_USER_MISO);
    w(SPI2 + SPI_MOSI_DLEN, 4 * 8 - 1);
    w(SPI2 + SPI_MISO_DLEN, 4 * 8 - 1);
    w(SPI2 + SPI_W0, 0x64636261); // 'abcd' LE
    w(SPI2 + SPI_CMD, SPI_USR_BIT);
    assert((r(SPI2 + SPI_W0) >>> 0) === 0x5a595857, `MISO from onTransfer (got 0x${r(SPI2 + SPI_W0).toString(16)})`);
    assert(JSON.stringify(c5.spi2.drainMosi()) === '[97,98,99,100]', 'MOSI captured for host');
    // Split path: injectMiso preload serves when onTransfer absent
    c5.spi2.onTransfer = null;
    c5.spi2.injectMiso([1, 2, 3, 4]);
    w(SPI2 + SPI_W0, 0x68676665); // 'efgh'
    w(SPI2 + SPI_CMD, SPI_USR_BIT);
    assert((r(SPI2 + SPI_W0) >>> 0) === 0x04030201, 'injectMiso preload served');
  }
}

async function partC() {
  console.log('\n=== Part C: SAB plumbing (Worker, ROM-only) ===');
  const flash = new Uint8Array(new SharedArrayBuffer(4 * 1024 * 1024));
  flash.fill(0xff);
  flash[0x1000] = 0xe9;
  const rom = readFileSync(resolve(__dirname, '../src/rom/esp32-v3-rom.bin'));
  const proxy = new SimulatorWorker();
  let errored = null;
  proxy._onError = (e) => { errored = e; };
  await proxy.init('ESP32', { flashSizeMB: 4, mmuPages: 64, strapValue: 0x13, budget: 2000000 }, flash, rom);
  assert(proxy._ready === true, 'worker init ok (new FFI imports resolve)');
  assert(!errored, 'no worker error during boot');

  // I2C split path over SAB (no deadlock while RUN=1 not needed here; stopped is fine too)
  await proxy.attachI2cSlave(0, 0x3c);
  await proxy.pushI2cTx(0, 0x3c, [0x11, 0x22]);
  assert(JSON.stringify(await proxy.popI2cRx(0, 0x3c)) === '[]', 'empty RX drains []');
  assert(JSON.stringify(await proxy.pollI2c(0)) === '[]', 'no events yet');
  await proxy.detachI2cSlave(0, 0x3c);

  // SPI split path
  await proxy.injectSpiMiso(2, [5, 6]);
  assert(JSON.stringify(await proxy.pollSpiTx(2)) === '[]', 'no MOSI captured without transactions');

  // GPIO readback + snapshot + changes
  const g = await proxy.getGpioOut(2);
  assert(g.level === 0 && g.dir === 0 && g.pull === 0, 'reset-state pin reads in/low/none');
  const samp = await proxy.sampleGpioOut();
  assert(samp.levels.length === 40 && samp.dirs.length === 40 && samp.pulls.length === 40, 'sample shape 40x3');
  assert(JSON.stringify(await proxy.pollGpioChanges()) === '[]', 'no gpio changes pending');

  // ADC + LEDC (freq is derived from timer config — nonzero garbage until
  // firmware programs the timer; only duty is asserted zero pre-config)
  await proxy.setAnalogInput(36, 1.1);
  const ledc = await proxy.getLedc(0);
  assert(ledc.duty === 0 && ledc.timer === 0 && Number.isFinite(ledc.freq), 'unconfigured LEDC reads duty 0');
  assert((await proxy.getLedcPin(0)) === -1, 'worker: unrouted LEDC channel maps to no pin');

  // Mid-run inline servicing: run() starves commandLoop, so these prove the
  // runSimChunk inline blocks (same bodies as processCommand) serve tap CMDs
  // while RUN=1 (a missing block would spin-wait forever here).
  proxy.run();
  await new Promise((r) => setTimeout(r, 300));
  await proxy.attachI2cSlave(0, 0x3c);
  await proxy.pushI2cTx(0, 0x3c, [0xaa]);
  const gmid = await proxy.getGpioOut(2);
  assert(gmid.level === 0 && gmid.dir === 0, 'mid-run getGpioOut works');
  assert(JSON.stringify(await proxy.pollGpioChanges()) === '[]', 'mid-run gpio poll works');
  assert(JSON.stringify(await proxy.pollI2c(0)) === '[]', 'mid-run i2c poll works (no transactions)');
  proxy.stop();
  await new Promise((r) => setTimeout(r, 100));

  proxy.terminate?.();
  assert(!errored, 'still no worker error after tap CMDs');
}

(async () => {
  try {
    await partA();
    await partB();
    await partC();
    console.log(`\n[test] taps-host: PASS=${pass} FAIL=${fail}`);
    if (fail) { console.log('[test] FAILED'); process.exit(1); }
    console.log('[test] PASSED');
  } catch (e) {
    console.error('\n[test] FAILED:', e?.stack ?? e);
    process.exit(1);
  }
})();
