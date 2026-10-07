// Host peripheral taps (esp32-emu.md §1-4) — no compile server needed.
// Part A (pure JS, no WASM): tap routing/NACK policy, preloads, queues,
//   GPIO queue/listeners, reset semantics.
// Part B (WASM+ROM, direct ESP32, MMIO-driven masters — no firmware):
//   native export presence, registry sync, GPIO drive → readback + events,
//   I2C master WRITE/READ/NACK via COMD programming, SPI transfer echo.
// Part C (Worker, ROM-only): SAB plumbing for all 11 tap CMDs (no deadlock,
//   sane values).
import { ESP32, SimulatorWorker, I2cTap, SpiTap, OneWireDevice, RmtTap, Ov2640, dallasCrc8 } from '../src/index.js';
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

  // OneWire device model (pure timing simulation on synthetic APB ticks)
  assert(dallasCrc8([0x28, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06]) === new OneWireDevice(4).rom[7], 'dallas CRC8 matches ROM CRC');
  const ow = new OneWireDevice(4);
  ow.setTemperature(23.5);
  assert(ow.scratchpad[0] === 0x78 && ow.scratchpad[1] === 0x01, 'setTemperature encodes 23.5°C (0x0178)');
  assert(ow.scratchpad[8] === dallasCrc8(ow.scratchpad.slice(0, 8)), 'scratchpad CRC authentic');
  const T = (us) => Math.round(us * 80); // APB ticks
  let now = 1000000;
  const fall = () => ow._ffiEdge(0, now);
  const releaseAfter = (lowUs) => { now += T(lowUs); ow._ffiEdge(2, now); };
  const writeBit = (b) => { fall(); releaseAfter(b ? 5 : 70); };
  const writeByte = (v) => { for (let i = 0; i < 8; i++) writeBit((v >> i) & 1); now += T(70); };
  const readBit = () => { fall(); releaseAfter(5); now += T(10); return ow._ffiRead(now); };
  const readByte = () => { let v = 0; for (let i = 0; i < 8; i++) v |= readBit() << i; now += T(70); return v; };
  fall(); releaseAfter(500); // reset pulse
  assert(ow.presenceCount === 1, 'reset pulse arms presence');
  now += T(70);
  assert(ow._ffiRead(now) === 0, 'presence pulse reads 0 in-window');
  now += T(300);
  assert(ow._ffiRead(now) === 1, 'bus idles high after presence window');
  writeByte(0xcc); writeByte(0xbe); // SKIP ROM + READ SCRATCHPAD
  const got = [];
  for (let i = 0; i < 9; i++) got.push(readByte());
  assert(JSON.stringify(got) === JSON.stringify(ow.scratchpad), 'scratchpad reads back byte-exact');
  assert(dallasCrc8(got.slice(0, 8)) === got[8], 'served scratchpad CRC verifies');
  const logged = ow.pollLog();
  assert(logged.presence === 1, 'presence logged');
  assert(JSON.stringify(logged.log.filter((e) => e.t === 'byte').map((e) => e.byte)) === '[204,190]', 'command bytes logged');
  ow._resetFlight();
  assert(ow.pollLog().log.length === 0, 'onewire reset clears log');

  // I2S TX staging (no WASM: direct _onI2sTxData)
  {
    const c0 = makeChip();
    const seen = [];
    const unsub = c0.onI2sTx((idx, words) => seen.push([idx, Array.from(words)]));
    c0._onI2sTxData(0, [1, 2, 3]);
    c0._onI2sTxData(1, [9]);
    const drained = c0.pollI2sTx();
    assert(drained.length === 2 && JSON.stringify(drained[0].words) === '[1,2,3]', 'pollI2sTx drains per-idx queues');
    assert(seen.length === 2, 'onI2sTx listener fired');
    unsub();
    c0._onI2sTxData(0, [7]);
    c0.reset();
    assert(c0.pollI2sTx(0).length === 0, 'reset clears i2s tx queues');
  }

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

  // RMT tap (pure model math + staging, no WASM)
  assert(RmtTap.item(10, 1, 20, 0) === ((((20 & 0x7fff) << 16) | (0 << 31) | 10 | (1 << 15)) >>> 0), 'RmtTap.item encodes halves');
  assert(RmtTap.item(0, 0, 0, 0) === 0, 'zero word is the terminator');
  {
    const sp = RmtTap.splitItem(RmtTap.item(10, 1, 20, 0));
    assert(sp.d0 === 10 && sp.l0 === 1 && sp.d1 === 20 && sp.l1 === 0, 'RmtTap.splitItem decodes halves');
  }
  assert(RmtTap.usToTicks(80, 1000000) === 80, 'usToTicks 80us @1MHz');
  assert(RmtTap.usToTicks(0, 1000000) === 0 && RmtTap.usToTicks(10, 0) === 0, 'usToTicks guards');
  {
    const cR = makeChip();
    assert(cR.rmt instanceof RmtTap, 'chip exposes rmt tap');
    const seenR = [];
    const unsubR = cR.rmt.onRmtTx(0, (ch, items) => seenR.push([ch, Array.from(items)]));
    cR.rmt._onRmtTxData(0, [0xaabbccdd, 0]);
    const drainedR = cR.rmt.pollRmtTx(0);
    assert(drainedR.length === 1 && drainedR[0].ch === 0
      && JSON.stringify(drainedR[0].items) === '[2864434397,0]', 'pollRmtTx drains staged items');
    assert(seenR.length === 1 && seenR[0][0] === 0, 'onRmtTx listener fired');
    assert(cR.rmt.getRmtTickHz(0) === 0, 'tickHz 0 pre-WASM');
    unsubR();
    cR.rmt.onRmtTx(9, () => {});
    assert(cR.rmt.pollRmtTx(9).length === 0, 'invalid ch polls empty');
    assert(cR.rmt.injectRmtRx(9, [1]) === 0 && cR.rmt.injectRmtRx(0, []) === 0, 'inject validates ch/len');
    assert(cR.rmt.injectRmtRx(2, [5, 6]) === 2, 'pre-WASM inject stages locally');
    cR.rmt._onRmtTxData(0, [7]);
    cR.reset();
    assert(cR.rmt.pollRmtTx(0).length === 0, 'reset clears captures');
  }

  // Camera frame staging (no WASM: direct _onCamFrameData, next to the I2S tap)
  {
    const cC = makeChip();
    const seenC = [];
    const unsubC = cC.onCameraFrame(0, (idx, words) => seenC.push([idx, Array.from(words)]));
    cC._onCamFrameData(0, [0x10000, 0x20101]);
    const drainedC = cC.pollCameraFrame(0);
    assert(drainedC.length === 1 && JSON.stringify(drainedC[0].words) === '[65536,131329]', 'pollCameraFrame drains staged words');
    assert(seenC.length === 1, 'onCameraFrame listener fired');
    unsubC();
    cC._onCamFrameData(1, [1]);
    cC.reset();
    assert(cC.pollCameraFrame(1).length === 0, 'reset clears camera queues');
    assert(cC.getCameraFrameBytes(0) === 0, 'frame len 0 pre-WASM');
    assert(cC.feedCameraFrame(0, [1, 2, 3]) === 0, 'feed stages 0 pre-WASM');
    assert(cC.cameraFeedLength(0) === 0, 'feed len 0 pre-WASM');
    cC.feedI2sRxSample(0, 0x12345678); // no-throw pre-WASM (worker path covers delivery)
  }

  // OV2640 sensor model (pure SCCB protocol, no WASM)
  {
    const ov = new Ov2640();
    assert(ov.regs[0x0a] === 0x26 && ov.regs[0x0b] === 0x42, 'OV2640 PID reset values');
    assert(ov.frameBytes() === 38400, 'QQVGA RGB565 = 38400B');
    ov.setResolution(640, 480);
    assert(ov.frameBytes() === 614400, 'VGA RGB565 = 614400B');
    ov.setFormat('GRAYSCALE');
    assert(ov.frameBytes() === 307200, 'VGA GRAYSCALE = 307200B');
    ov.setFormat('RGB565');
    ov.setResolution(160, 120);
    const ef = ov.expectedFrame();
    assert(ef.length === 38400 && ef[0] === 0 && ef[255] === 255 && ef[256] === 0
      && ef[38399] === (38399 & 0xff), 'expectedFrame is the engine ramp');
    let threw = false;
    try { ov.setFormat('JPEG'); } catch { threw = true; }
    assert(threw, 'JPEG format rejected (unbounded)');
    // SCCB through the I2C tap object (fake chip — sync deferred, pure JS)
    const i2cA = new I2cTap(fakeChip, 0);
    ov.attachToI2c(i2cA);
    assert(i2cA._ffiStart(0x30, false) === 1, 'SCCB addr ACKs');
    assert(i2cA._ffiWrite(0x0a) === 1, 'reg-pointer write ACKs');
    i2cA._ffiStop(false);
    assert(i2cA._ffiStart(0x30, true) === 1, 'SCCB read ACKs');
    assert(i2cA._ffiRead() === 0x26 && i2cA._ffiRead() === 0x42, 'PID reads serve 0x26/0x42');
    i2cA._ffiStop(true);
    assert(JSON.stringify(ov.drainSccbLog()) === '[]', 'pointer-only selects do not commit');
    i2cA._ffiStart(0x30, false);
    i2cA._ffiWrite(0x12);
    i2cA._ffiWrite(0x80);
    i2cA._ffiStop(false);
    assert(JSON.stringify(ov.drainSccbLog()) === '[{"reg":18,"bytes":[128]}]', 'register write commits at STOP');
    i2cA._ffiStart(0x30, false);
    i2cA._ffiWrite(0x12);
    i2cA._ffiStop(false);
    i2cA._ffiStart(0x30, true);
    assert(i2cA._ffiRead() === 0x80, 'written reg reads back');
    i2cA._ffiStop(true);
    ov.detachFromI2c();
    assert(i2cA._ffiStart(0x30, false) === 0, 'detached SCCB NACKs');
    ov._resetFlight();
    assert(ov.regs[0x12] === 0x80, 'reset keeps device regs');
  }
}

function makeChip() {
  const chip = new ESP32({ flashSizeMB: 4 });
  assert(chip.i2c0 instanceof I2cTap && chip.i2c1 instanceof I2cTap, 'chip exposes i2c taps');
  assert(Array.isArray(chip.i2c) && chip.i2c.length === 2, 'chip exposes i2c array');
  assert(chip.spi2 instanceof SpiTap && Array.isArray(chip.spi) && chip.spi.length === 4, 'chip exposes spi taps');
  assert(chip.rmt instanceof RmtTap, 'chip exposes rmt tap');
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
    'native_rmt_set_tx_hook', 'native_rmt_rx_scratch_ptr', 'native_rmt_rx_inject', 'native_rmt_tick_hz',
    'native_i2s_set_cam_hook', 'native_i2s_cam_frame_bytes', 'native_i2s_cam_frame_len',
    'native_i2s_cam_feed', 'native_i2s_cam_feed_scratch_ptr', 'native_i2s_cam_host_len',
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

  // ---- Programmed-state LEDC readback (esp32-emu.md §7 robustness) ----
  // Driver-sequence programming (timer + CONF0 + DUTY<<4 + CONF1 START):
  // duty must latch at INTEGER scale (DUTY holds driver <<4 format;
  // current_duty/DUTY_RD>>4/getLedcDuty are integer — 16x was wrong).
  {
    const LEDC = 0x3ff59000, HID_LEDC = 16;
    const lw = (a, v) => ex.native_diag_write(HID_LEDC, a, v >>> 0, 32);
    const lr = (a) => ex.native_diag_read(HID_LEDC, a, 32) >>> 0;
    lw(LEDC + 352, 13 | (500 << 5) | (1 << 25) | (1 << 26)); // LS timer0: res13, div500, APB, PARA_UP
    lw(LEDC + 160, 0x10); // CH8 CONF0: timer0 + SIG_OUT_EN
    lw(LEDC + 168, 2048 << 4); // CH8 DUTY (driver <<4 format)
    lw(LEDC + 172, 0x80000000); // CH8 CONF1 DUTY_START (plain latch)
    assert(chip.getLedcDuty(8) === 2048, `programmed LEDC duty latches integer 2048 (got ${chip.getLedcDuty(8)})`);
    assert((lr(LEDC + 176) >>> 0) === (2048 << 4), 'DUTY_RD echoes driver <<4 format (guest ledc_get_duty path)');
    const f8 = chip.getLedcFreq(8);
    assert(f8 > 4900 && f8 < 5100, `programmed LEDC freq ~5kHz (got ${f8})`);
    assert(chip.getLedcTimer(8) === 0, 'programmed LEDC timer sel reads 0');
    // Channel→pin resolution through the real GPIO matrix register file
    ex.native_diag_write(HID_GPIO, GPIO_BASE + 1328 + 5 * 4, 71 + 8);
    assert(chip.getLedcPin(8) === 5, `getLedcPin(8) resolves routed GPIO5 (got ${chip.getLedcPin(8)})`);
    // HS channel mirrors the LS latch path
    lw(LEDC + 320, 13 | (500 << 5) | (1 << 25) | (1 << 26)); // HS timer0
    lw(LEDC + 0, 0x10);
    lw(LEDC + 8, 1024 << 4);
    lw(LEDC + 12, 0x80000000);
    assert(chip.getLedcDuty(0) === 1024, `HS duty latches integer 1024 (got ${chip.getLedcDuty(0)})`);
  }

  // ---- v3 pin-API LEDC replay: ledcAttach(pin, 5000, 8) + ledcWrite(pin, 192) ----
  // The START-only pattern above is synthetic; real Arduino-3.x/IDF firmware
  // emits INC/NUM=1/CYCLE=1/SCALE=0 filler in CONF1 (ledc_duty_config) with
  // DUTY_START set by a separate ledc_update_duty write. That 1-step filler
  // used to mistrigger the fade ramp (duty latched 1 instead of 192).
  {
    const LEDC = 0x3ff59000, HID_LEDC = 16;
    const lw = (a, v) => ex.native_diag_write(HID_LEDC, a, v >>> 0, 32);
    const lr = (a) => ex.native_diag_read(HID_LEDC, a, 32) >>> 0;
    lw(LEDC + 320, 8 | (16000 << 5) | (1 << 25) | (1 << 26)); // HS timer0: res8, div16000 (5kHz), APB, PARA_UP
    lw(LEDC + 0, 0x00); // CH0 CONF0 as left by ledc_channel_config (timer0, out not yet en)
    lw(LEDC + 4, 0); // CH0 HPOINT 0 (ledc_channel_config hpoint)
    lw(LEDC + 8, 0 << 4); // attach-time DUTY 0 (channel_config duty=current 0)
    lw(LEDC + 12, 0x40100400); // attach-time CONF1 filler (INC/NUM=1/CYCLE=1, no START)
    lw(LEDC + 12, 0xc0100400); // attach-time DUTY_START (ledc_update_duty)
    assert(chip.getLedcDuty(0) === 0, `attach leaves duty 0 (got ${chip.getLedcDuty(0)})`);
    lw(LEDC + 0, 0x08); // ledc_update_duty sets SIG_OUT_EN
    lw(LEDC + 8, 192 << 4); // ledcWrite(pin,192): ledc_set_duty DUTY
    lw(LEDC + 12, 0x40100400); // ledc_set_duty stages filler (no START yet)
    lw(LEDC + 12, 0xc0100400); // ledc_update_duty DUTY_START
    assert(chip.getLedcDuty(0) === 192, `v3 ledcWrite(192) reads back 192 (got ${chip.getLedcDuty(0)})`);
    assert((lr(LEDC + 16) >>> 0) === (192 << 4), 'DUTY_RD echoes 192<<4 (guest ledcRead path)');
    const f0 = chip.getLedcFreq(0);
    assert(f0 > 4900 && f0 < 5100, `v3 timer freq ~5kHz (got ${f0})`);
    assert(chip.getLedcTimer(0) === 0, 'v3 channel timer sel reads 0');
    // Repeat write to a new value latches too (no start+1 drift)
    lw(LEDC + 8, 64 << 4);
    lw(LEDC + 12, 0x40100400);
    lw(LEDC + 12, 0xc0100400);
    assert(chip.getLedcDuty(0) === 64, `second v3 write reads 64 (got ${chip.getLedcDuty(0)})`);
  }

  // ---- v3 pin-API LEDC, SLOW cadence: same register sequence with ~3.8M
  // retired instructions interleaved between writes (browser/SAB-worker
  // cadence: coarse stepping, boots over minutes). The 1-step filler used
  // to arm a real 16000-APB-tick fade (start+1); with coarse stepping the
  // guest DUTY_RD sampled mid-fade read 0 while the host tap sampled
  // post-fade read 1. The latch fix makes both read 192 at any cadence.
  // Fails pre-fix (DUTY_RD>>4 reads 0 mid-fade, host current_duty stale 0).
  {
    const c8 = makeChip();
    const ex8 = await bootChip(c8);
    const LEDC = 0x3ff59000, HID_LEDC = 16;
    const lw8 = (a, v) => ex8.native_diag_write(HID_LEDC, a, v >>> 0, 32);
    const lr8 = (a) => ex8.native_diag_read(HID_LEDC, a, 32) >>> 0;
    const stepMs = (n) => { for (let i = 0; i < n; i++) c8.step(); }; // ~512 instr each
    const cyc0 = c8.cycles;
    lw8(LEDC + 320, 8 | (16000 << 5) | (1 << 25) | (1 << 26)); // HS timer0 5kHz
    stepMs(500);
    lw8(LEDC + 0, 0x00);
    stepMs(500);
    lw8(LEDC + 4, 0);
    stepMs(500);
    lw8(LEDC + 8, 0 << 4);
    stepMs(500);
    lw8(LEDC + 12, 0x40100400);
    stepMs(500);
    lw8(LEDC + 12, 0xc0100400); // attach START
    stepMs(500);
    assert(c8.getLedcDuty(0) === 0, `slow attach leaves duty 0 (got ${c8.getLedcDuty(0)})`);
    lw8(LEDC + 0, 0x08);
    stepMs(500);
    lw8(LEDC + 8, 192 << 4); // ledcWrite(pin,192)
    stepMs(500);
    lw8(LEDC + 12, 0x40100400);
    stepMs(500);
    lw8(LEDC + 12, 0xc0100400); // update START
    // NOTE: the direct harness clock is frozen (CLK_CYCLES only advances via
    // the worker pumps), so the guard here is retired-instruction count
    // between MMIO writes — the browser-cadence interleave. True sim-time
    // elapse is covered by the Part C SAB block below (worker-driven clock).
    stepMs(3000);
    assert(c8.cycles - cyc0 >= 3000 * 512, `instructions elapsed between writes (${c8.cycles - cyc0})`);
    assert((lr8(LEDC + 16) >>> 4) === 192, `slow guest DUTY_RD reads 192 (got ${lr8(LEDC + 16) >>> 4})`);
    assert(c8.getLedcDuty(0) === 192, `slow host duty reads 192 (got ${c8.getLedcDuty(0)})`);
    const f8 = c8.getLedcFreq(0);
    assert(f8 > 4900 && f8 < 5100, `slow timer freq ~5kHz (got ${f8})`);
  }

  // ---- I2C sensor pattern: write-pointer then repeated-START read + 0xFF over-read fill ----
  {
    const c6 = makeChip();
    const ex6 = await bootChip(c6);
    c6.i2c0.preloadRead(0x3c, [0xab]); // 1 staged byte, firmware reads 2
    const w6 = (a, v) => ex6.native_diag_write(HID_I2C, a, v >>> 0, 32);
    const r6 = (a) => ex6.native_diag_read(HID_I2C, a, 32) >>> 0;
    w6(I2C0 + I2C_OF, 0x78); w6(I2C0 + I2C_OF, 0x00); // addr+W, register pointer
    w6(I2C0 + I2C_COMD + 0, (I2C_OP_RSTART << 11) | 0);
    w6(I2C0 + I2C_COMD + 4, (I2C_OP_WRITE << 11) | (1 << 8) | 2);
    w6(I2C0 + I2C_COMD + 8, (I2C_OP_STOP << 11) | 0);
    w6(I2C0 + I2C_CTR, I2C_CTR_EXEC);
    w6(I2C0 + I2C_OF, 0x79); // addr+R
    w6(I2C0 + I2C_COMD + 0, (I2C_OP_RSTART << 11) | 0);
    w6(I2C0 + I2C_COMD + 4, (I2C_OP_WRITE << 11) | (1 << 8) | 1);
    w6(I2C0 + I2C_COMD + 8, (I2C_OP_READ << 11) | 2);
    w6(I2C0 + I2C_COMD + 12, (I2C_OP_STOP << 11) | 0);
    w6(I2C0 + I2C_CTR, I2C_CTR_EXEC);
    const q0 = r6(I2C0 + I2C_OF) & 0xff, q1 = r6(I2C0 + I2C_OF) & 0xff;
    assert(q0 === 0xab && q1 === 0xff, `write-then-read serves staged + 0xFF fill (got ${q0.toString(16)},${q1.toString(16)})`);
    assert(JSON.stringify(c6.i2c0.drainWriteLog(0x3c)) === '[0]', 'pointer byte captured in write phase');
  }

  // ---- SPI capture-only arm: empty injectMiso still captures MOSI ----
  {
    const c7 = makeChip();
    const ex7 = await bootChip(c7);
    c7.spi2.injectMiso([]); // observe a write-only device, stage no MISO
    const w7 = (a, v) => ex7.native_diag_write(HID_SPI, a, v >>> 0, 32);
    w7(SPI2 + SPI_USER, SPI_USER_MOSI | SPI_USER_MISO);
    w7(SPI2 + SPI_MOSI_DLEN, 2 * 8 - 1);
    w7(SPI2 + SPI_MISO_DLEN, 2 * 8 - 1);
    w7(SPI2 + SPI_W0, 0x0201);
    w7(SPI2 + SPI_CMD, SPI_USR_BIT);
    assert(JSON.stringify(c7.spi2.drainMosi()) === '[1,2]', 'empty-arm captures MOSI for host models');
  }

  // ---- RMT tap: guest TX capture + host RX inject (MMIO-driven, no firmware) ----
  // RMT v1 map: CHn_CONF0 = 32+8n, CHn_CONF1 = 36+8n, RAM = 2048+256n,
  // INT_RAW = 160, INT_CLR = 172, CHnSTATUS = 0x60+4n. CONF1 bit0 =
  // TX_START, bit1 = RX_EN (this port); bit17 selects the APB parent (REF
  // otherwise); bit5 clear = MEM_OWNER TX. TX_END = bit 3n, RX_END =
  // bit 3n+1 (port numbering, LED_REG40 = 0).
  const RMT = 0x3ff56000, HID_RMT = 18;
  const RMT_CONF0 = 32, RMT_CONF1 = 36, RMT_RAM = 2048;
  const RMT_INT_RAW = 160, RMT_INT_CLR = 172, RMT_STATUS = 0x60;
  const RMT_CONF_TX = 0x20f01; // APB parent + owner TX + TX_START
  const RMT_CONF_RX = 0x20f22; // APB parent + owner RX + RX_EN
  {
    const cR = makeChip();
    const exR = await bootChip(cR);
    const wR = (a, v) => exR.native_diag_write(HID_RMT, a, v >>> 0, 32);
    const rR = (a) => exR.native_diag_read(HID_RMT, a, 32) >>> 0;
    const seenR = [];
    cR.rmt.onRmtTx(0, (ch, items) => seenR.push([ch, Array.from(items)]));
    const w0 = RmtTap.item(10, 1, 20, 0);
    const w1 = RmtTap.item(30, 0, 40, 1);
    wR(RMT + RMT_RAM + 0, w0);
    wR(RMT + RMT_RAM + 4, w1);
    wR(RMT + RMT_RAM + 8, 0); // zero terminator
    wR(RMT + RMT_CONF0, 0x31100002); // div 2 (40 MHz), 1 block
    wR(RMT + RMT_CONF1, RMT_CONF_TX);
    // Pump like the worker busy path: syncClockState (chip.cycles lives in
    // JS; the native event queue reads CLK_CYCLES) then native_pump_events.
    const pumpR = () => {
      cR.step();
      try { exR.native_set_clock_state?.(cR.cycles >>> 0); } catch {}
      try { exR.native_pump_events?.(); } catch {}
    };
    let cap = [];
    for (let i = 0; i < 500 && cap.length === 0; i++) {
      pumpR();
      cap = cR.rmt.pollRmtTx(0);
    }
    assert(cap.length === 1 && JSON.stringify(cap[0].items) === JSON.stringify([w0, w1, 0]),
      'RMT TX capture reports items + terminator');
    assert(cap[0].ch === 0 && cap[0].tickHz === 40000000, `capture carries ch + 40MHz tick (got ${cap[0]?.tickHz})`);
    assert(seenR.length === 1, 'onRmtTx listener fired once');
    assert((rR(RMT + RMT_INT_RAW) & (1 << 0)) !== 0, 'TX_END (ch0) raised');
    assert(cR.rmt.getRmtTickHz(0) === 40000000, 'getRmtTickHz reads 40MHz');
    // Unarmed channel stays silent (zero FFI): TX still completes in HW.
    wR(RMT + 3 * 256 + RMT_RAM, w0);
    wR(RMT + 3 * 256 + RMT_RAM + 4, 0);
    wR(RMT + 3 * 8 + RMT_CONF0, 0x31100002);
    wR(RMT + 3 * 8 + RMT_CONF1, RMT_CONF_TX);
    for (let i = 0; i < 500; i++) pumpR();
    assert(cR.rmt.pollRmtTx(3).length === 0, 'unarmed channel stages nothing');
  }

  // ---- RMT RX inject-while-armed: immediate delivery (ch1) ----
  {
    const cR2 = makeChip();
    const exR2 = await bootChip(cR2);
    const wR2 = (a, v) => exR2.native_diag_write(HID_RMT, a, v >>> 0, 32);
    const rR2 = (a) => exR2.native_diag_read(HID_RMT, a, 32) >>> 0;
    wR2(RMT + 8 + RMT_CONF1, RMT_CONF_RX); // arm RX
    wR2(RMT + RMT_INT_CLR, 0xffffffff); // clear the blip
    const rxw = [0x11111111, 0x22222222, 0x33333333];
    assert(cR2.rmt.injectRmtRx(1, rxw) === 3, 'inject-while-armed stages 3');
    assert((rR2(RMT + RMT_INT_RAW) & (1 << 4)) !== 0, 'RX_END (ch1) raised');
    assert(rR2(RMT + 256 + RMT_RAM) === 0x11111111
      && rR2(RMT + 256 + RMT_RAM + 4) === 0x22222222
      && rR2(RMT + 256 + RMT_RAM + 8) === 0x33333333, 'RX RAM holds injected words');
    assert((rR2(RMT + 8 + RMT_CONF1) & 32) !== 0, 'MEM_OWNER flipped to SW');
    assert(rR2(RMT + RMT_STATUS + 4) === 67, `WADDR reports 64+3 (got ${rR2(RMT + RMT_STATUS + 4)})`);
  }

  // ---- RMT RX inject-before-arm: pending delivers on RX_EN (ch2) ----
  // Pre-WASM local staging flushes through the real path at loadWasm, so
  // inject BEFORE boot and arm after: proves the preload survives boot.
  {
    const cR3 = makeChip();
    assert(cR3.rmt.injectRmtRx(2, [0xaaaaaaaa, 0xbbbbbbbb]) === 2, 'pre-boot inject stages locally');
    const exR3 = await bootChip(cR3);
    const wR3 = (a, v) => exR3.native_diag_write(HID_RMT, a, v >>> 0, 32);
    const rR3 = (a) => exR3.native_diag_read(HID_RMT, a, 32) >>> 0;
    assert(rR3(RMT + RMT_INT_RAW) === 0, 'no RX_END before arming');
    wR3(RMT + 16 + RMT_CONF1, RMT_CONF_RX); // RX_EN arm
    assert((rR3(RMT + RMT_INT_RAW) & (1 << 7)) !== 0, 'pending delivers on RX_EN arm');
    assert(rR3(RMT + 512 + RMT_RAM) === 0xaaaaaaaa
      && rR3(RMT + 512 + RMT_RAM + 4) === 0xbbbbbbbb, 'pending words land in RX RAM');
    assert(rR3(RMT + RMT_STATUS + 8) === 130, `WADDR reports 128+2 (got ${rR3(RMT + RMT_STATUS + 8)})`);
  }

  // ---- Camera tap: frame-size config + OV2640 SCCB over the I2C master path ----
  {
    assert(chip.getCameraFrameBytes(0) === 38400, 'default camera frame 38400B (QQVGA RGB565)');
    chip.setCameraFrameBytes(0, 64);
    assert(chip.getCameraFrameBytes(0) === 64, 'frame-size config roundtrips');
    chip.setCameraFrameBytes(0, 38400);
    assert(JSON.stringify(chip.pollCameraFrame(0)) === '[]', 'no staged frame bytes pre-capture');
    // Host→sensor feed: stage, append, clear, cap (one QQVGA frame), reset.
    const feed64 = [];
    for (let i = 0; i < 64; i++) feed64.push((i * 13 + 7) & 0xff);
    assert(chip.feedCameraFrame(0, feed64) === 64, 'feed stages 64B');
    assert(chip.cameraFeedLength(0) === 64, 'feed len reads 64');
    assert(chip.feedCameraFrame(0, feed64) === 64, 'feed appends');
    assert(chip.cameraFeedLength(0) === 128, 'feed len reads 128 after append');
    assert(chip.feedCameraFrame(0, []) === 0, 'empty feed clears');
    assert(chip.cameraFeedLength(0) === 0, 'feed len 0 after clear');
    const big = new Array(40000).fill(0xab);
    assert(chip.feedCameraFrame(0, big) === 38400, 'feed caps at one QQVGA frame');
    assert(chip.cameraFeedLength(0) === 38400, 'feed len reads cap');
    assert(chip.feedCameraFrame(5, [1]) === 0 && chip.cameraFeedLength(5) === 0, 'invalid idx feeds nothing');
    chip.feedCameraFrame(0, []);
    chip.reset();
    assert(chip.cameraFeedLength(0) === 0, 'reset clears feed staging');
    chip.feedI2sRxSample(0, 0x12345678);
    chip.feedI2sRxSample(0, 0x9abcdef0); // pair completes silently (no camera RX active)
  }
  {
    // esp-camera-style SCCB PID probe: pointer-write then repeated-START
    // read serves 0x26/0x42 from the OV2640 model through the I2C tap.
    const cC = makeChip();
    const exC = await bootChip(cC);
    const cam = cC.attachOv2640(0);
    assert(cam.regs[0x0a] === 0x26 && cam.regs[0x0b] === 0x42, 'attached OV2640 serves PID');
    assert(cC._ov2640Models.get('0:48') === cam, 'model registered by bus:addr');
    const wC = (a, v) => exC.native_diag_write(HID_I2C, a, v >>> 0, 32);
    const rC = (a) => exC.native_diag_read(HID_I2C, a, 32) >>> 0;
    wC(I2C0 + I2C_OF, 0x60); wC(I2C0 + I2C_OF, 0x0a); // SCCB addr+W, reg pointer
    wC(I2C0 + I2C_COMD + 0, (I2C_OP_RSTART << 11) | 0);
    wC(I2C0 + I2C_COMD + 4, (I2C_OP_WRITE << 11) | (1 << 8) | 2);
    wC(I2C0 + I2C_COMD + 8, (I2C_OP_STOP << 11) | 0);
    wC(I2C0 + I2C_CTR, I2C_CTR_EXEC);
    wC(I2C0 + I2C_OF, 0x61); // SCCB addr+R
    wC(I2C0 + I2C_COMD + 0, (I2C_OP_RSTART << 11) | 0);
    wC(I2C0 + I2C_COMD + 4, (I2C_OP_WRITE << 11) | (1 << 8) | 1);
    wC(I2C0 + I2C_COMD + 8, (I2C_OP_READ << 11) | 2);
    wC(I2C0 + I2C_COMD + 12, (I2C_OP_STOP << 11) | 0);
    wC(I2C0 + I2C_CTR, I2C_CTR_EXEC);
    const p0 = rC(I2C0 + I2C_OF) & 0xff, p1 = rC(I2C0 + I2C_OF) & 0xff;
    assert(p0 === 0x26 && p1 === 0x42, `SCCB PID probe reads 0x26/0x42 (got ${p0.toString(16)},${p1.toString(16)})`);
    assert(JSON.stringify(cam.drainSccbLog()) === '[]', 'probe selects commit nothing');
    // Register program + readback through the same path.
    wC(I2C0 + I2C_OF, 0x60); wC(I2C0 + I2C_OF, 0x12); wC(I2C0 + I2C_OF, 0x80);
    wC(I2C0 + I2C_COMD + 0, (I2C_OP_RSTART << 11) | 0);
    wC(I2C0 + I2C_COMD + 4, (I2C_OP_WRITE << 11) | (1 << 8) | 3);
    wC(I2C0 + I2C_COMD + 8, (I2C_OP_STOP << 11) | 0);
    wC(I2C0 + I2C_CTR, I2C_CTR_EXEC);
    assert(JSON.stringify(cam.drainSccbLog()) === '[{"reg":18,"bytes":[128]}]', 'SCCB program commits at STOP');
    cC.detachOv2640(0);
    assert(!cC._ov2640Models.has('0:48'), 'detach removes the model');
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

  // RMT tap split path (stopped is fine; inject-while-unarmed pends)
  await proxy.armRmtTx(0);
  assert(JSON.stringify(await proxy.pollRmtTx(0)) === '[]', 'no RMT TX staged pre-capture');
  assert((await proxy.injectRmtRx(1, [0x11111111, 0x22222222])) === 2, 'worker inject stages 2');
  {
    // Program ch1 APB-parented + arm RX via the backdoor: the pending
    // delivery lands, then read back purely through SAB.
    const RMT = 0x3ff56000, HID_RMT = 18;
    await proxy.writeMmio(HID_RMT, RMT + 40, 0x31100002);
    await proxy.writeMmio(HID_RMT, RMT + 44, 0x20f02);
    assert((await proxy.getRmtTickHz(1)) === 40000000, 'worker tickHz reads 40MHz');
    const raw = await proxy.readMmio(HID_RMT, RMT + 160, 4);
    assert((raw & (1 << 4)) !== 0, 'worker RX_END (ch1) raised after arm');
  }

  // Camera split path
  await proxy.armCamera(0);
  assert(JSON.stringify(await proxy.pollCameraFrame(0)) === '[]', 'no staged frame bytes pre-capture');
  await proxy.setCameraFrameBytes(0, 128);
  assert((await proxy.getCameraFrameBytes(0)) === 128, 'worker frame-size config roundtrips');
  await proxy.setCameraFrameBytes(0, 38400);
  await proxy.attachCameraSccb(0, 0x30);
  assert(JSON.stringify(await proxy.pollI2c(0)) === '[]', 'no SCCB events without transactions');
  {
    const fbytes = [];
    for (let i = 0; i < 128; i++) fbytes.push((i * 13 + 7) & 0xff);
    assert((await proxy.feedCameraFrame(0, fbytes)) === 128, 'worker feed stages 128B');
    assert((await proxy.cameraFeedLength(0)) === 128, 'worker feed len reads 128');
    assert((await proxy.feedCameraFrame(0, [])) === 0, 'worker empty feed clears');
    assert((await proxy.cameraFeedLength(0)) === 0, 'worker feed len 0 after clear');
  }

  // Mid-run inline servicing: run() starves commandLoop, so these prove the
  // runSimChunk inline blocks (same bodies as processCommand) serve tap CMDs
  // while RUN=1 (a missing block would spin-wait forever here).
  proxy.run();  await new Promise((r) => setTimeout(r, 300));
  await proxy.attachI2cSlave(0, 0x3c);
  await proxy.pushI2cTx(0, 0x3c, [0xaa]);
  const gmid = await proxy.getGpioOut(2);
  assert(gmid.level === 0 && gmid.dir === 0, 'mid-run getGpioOut works');
  assert(JSON.stringify(await proxy.pollGpioChanges()) === '[]', 'mid-run gpio poll works');
  assert(JSON.stringify(await proxy.pollI2c(0)) === '[]', 'mid-run i2c poll works (no transactions)');
  assert(JSON.stringify(await proxy.pollRmtTx(0)) === '[]', 'mid-run RMT poll works');
  assert((await proxy.getRmtTickHz(1)) === 40000000, 'mid-run tickHz works');
  assert(JSON.stringify(await proxy.pollCameraFrame(0)) === '[]', 'mid-run camera poll works');
  assert((await proxy.getCameraFrameBytes(0)) === 38400, 'mid-run frame-len works');
  assert((await proxy.feedCameraFrame(0, [1, 2, 3, 4])) === 4, 'mid-run feed works');
  assert((await proxy.cameraFeedLength(0)) === 4, 'mid-run feed-len works');
  assert((await proxy.feedCameraFrame(0, [])) === 0, 'mid-run feed clear works');
  // SAB-path slow-cadence v3 LEDC: program ch0 via the MMIO backdoor while
  // the worker runs, let wall/sim time elapse (the browser cadence that
  // used to race the 1-step filler fade), then read back purely through
  // SAB (CMD_GET_LEDC + CMD_READ_MMIO) — guest DUTY_RD and host tap agree.
  {
    const LEDC = 0x3ff59000, HID_LEDC = 16;
    await proxy.writeMmio(HID_LEDC, LEDC + 320, 8 | (16000 << 5) | (1 << 25) | (1 << 26));
    const t0 = await proxy.readMmio(HID_LEDC, LEDC + 324, 4);
    await proxy.writeMmio(HID_LEDC, LEDC + 0, 0x00);
    await proxy.writeMmio(HID_LEDC, LEDC + 4, 0);
    await proxy.writeMmio(HID_LEDC, LEDC + 8, 0 << 4);
    await proxy.writeMmio(HID_LEDC, LEDC + 12, 0x40100400);
    await proxy.writeMmio(HID_LEDC, LEDC + 12, 0xc0100400);
    await proxy.writeMmio(HID_LEDC, LEDC + 0, 0x08);
    await proxy.writeMmio(HID_LEDC, LEDC + 8, 192 << 4);
    await proxy.writeMmio(HID_LEDC, LEDC + 12, 0x40100400);
    await proxy.writeMmio(HID_LEDC, LEDC + 12, 0xc0100400);
    await new Promise((r) => setTimeout(r, 1500)); // coarse SAB cadence
    const t1 = await proxy.readMmio(HID_LEDC, LEDC + 324, 4);
    assert(t1 !== t0, `worker sim time elapsed over SAB cadence (timer ${t0} -> ${t1})`);
    const slow = await proxy.getLedc(0);
    const slowRd = await proxy.readMmio(HID_LEDC, LEDC + 16, 4);
    assert(slow.duty === 192, `slow SAB host duty reads 192 (got ${slow.duty})`);
    assert((slowRd >>> 4) === 192, `slow SAB guest DUTY_RD reads 192 (got ${slowRd >>> 4})`);
    assert(slow.freq > 4900 && slow.freq < 5100, `slow SAB timer freq ~5kHz (got ${slow.freq})`);
  }
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
