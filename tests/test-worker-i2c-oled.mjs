// Acceptance §1: ESP32 SSD1306 OLED vramFill > 0 via I2C (esp32-emu.md).
// Firmware (IDF i2c_master driver, no libs) inits an SSD1306 at 0x3C and
// fills the framebuffer; the host model (control 0x00 = command, 0x40 =
// data, GDDRAM 8 pages x 128 cols) is fed from tapped I2C transactions and
// must report vramFill > 0 with the display on. Requires :5525.
import { SimulatorWorker } from '../src/index.js';
import axios from 'axios';
import { readFileSync } from 'fs';
import { resolve, dirname } from 'path';
import { fileURLToPath } from 'url';

const __dirname = dirname(fileURLToPath(import.meta.url));

async function compile(code) {
  const r = await axios.post('http://localhost:5525/api/compile/start', { code, target: 'esp32', targetEngine: 'frontend', fqbn: 'esp32:esp32:esp32' });
  while (true) {
    const s = await axios.get(`http://localhost:5525/api/compile/status/${r.data.buildId}`);
    if (s.data.status === 'success') return s.data.binary_content;
    if (s.data.status === 'failed') throw new Error('Compile failed: ' + s.data.error);
    await new Promise(r => setTimeout(r, 1000));
  }
}

// Minimal SSD1306 host model: command/data framing + GDDRAM + vramFill.
function makeSsd1306() {
  const gddram = new Uint8Array(8 * 128);
  let colStart = 0, colEnd = 127, pageStart = 0, pageEnd = 7;
  let col = 0, page = 0, displayOn = false;
  let pendingCmd = -1, pendingArg = 0;
  function command(b) {
    if (pendingCmd === 0x21) { // column address: start, then end
      if (pendingArg === 0) { colStart = b & 0x7f; pendingArg = 1; }
      else { colEnd = b & 0x7f; col = colStart; pendingCmd = -1; pendingArg = 0; }
      return;
    }
    if (pendingCmd === 0x22) { // page address: start, then end
      if (pendingArg === 0) { pageStart = b & 0x07; pendingArg = 1; }
      else { pageEnd = b & 0x07; page = pageStart; pendingCmd = -1; pendingArg = 0; }
      return;
    }
    if (b === 0x21 || b === 0x22) { pendingCmd = b; pendingArg = 0; return; }
    if (b === 0xaf) displayOn = true;
    else if (b === 0xae) displayOn = false;
  }
  return {
    // One I2C write transaction (bytes AFTER the 7-bit address): first byte
    // is the control byte (0x00 = commands follow, 0x40 = GDDRAM data).
    transact(bytes) {
      if (!bytes.length) return;
      if (bytes[0] === 0x40) {
        for (let i = 1; i < bytes.length; i++) {
          gddram[page * 128 + col] = bytes[i];
          col++;
          if (col > colEnd) { col = colStart; page = page + 1 > pageEnd ? pageStart : page + 1; }
        }
      } else if (bytes[0] === 0x00) {
        for (let i = 1; i < bytes.length; i++) command(bytes[i]);
      }
    },
    get vramFill() { let n = 0; for (const b of gddram) if (b) n++; return n; },
    get isOn() { return displayOn; },
  };
}

const firmware = `
#include <driver/i2c_master.h>
#define OLED 0x3c

static i2c_master_dev_handle_t dev;
static esp_err_t tx(const uint8_t *b, size_t n) {
  return i2c_master_transmit(dev, b, n, 1000);
}
static void cmd1(uint8_t c) { uint8_t b[2] = {0x00, c}; tx(b, 2); }
static void cmd2(uint8_t c, uint8_t a) { uint8_t b[3] = {0x00, c, a}; tx(b, 3); }

void setup() {
  Serial.begin(115200);
  Serial.println("=== OLED TEST ===");
  i2c_master_bus_config_t bus_cfg = {};
  bus_cfg.i2c_port = 0;
  bus_cfg.sda_io_num = (gpio_num_t)21;
  bus_cfg.scl_io_num = (gpio_num_t)22;
  bus_cfg.clk_source = I2C_CLK_SRC_DEFAULT;
  bus_cfg.glitch_ignore_cnt = 7;
  bus_cfg.flags.enable_internal_pullup = 1;
  i2c_master_bus_handle_t bus;
  if (i2c_new_master_bus(&bus_cfg, &bus) != ESP_OK) { Serial.println("BUS=FAIL"); return; }
  i2c_device_config_t dev_cfg = {};
  dev_cfg.dev_addr_length = I2C_ADDR_BIT_LEN_7;
  dev_cfg.device_address = OLED;
  dev_cfg.scl_speed_hz = 400000;
  if (i2c_master_bus_add_device(bus, &dev_cfg, &dev) != ESP_OK) { Serial.println("ADDDEV=FAIL"); return; }
  Serial.println("BUSDEV=PASS");
  cmd1(0xAE);
  cmd2(0xD5, 0x80); cmd2(0xA8, 0x3F); cmd2(0xD3, 0x00);
  cmd1(0x40); cmd2(0x8D, 0x14); cmd2(0x20, 0x00);
  cmd1(0xA1); cmd1(0xC8); cmd2(0xDA, 0x12);
  cmd2(0x81, 0xCF); cmd2(0xD9, 0xF1); cmd2(0xDB, 0x40);
  cmd1(0xA4); cmd1(0xA6); cmd1(0xAF);
  Serial.println("INIT=PASS");
  { uint8_t b[4] = {0x00, 0x21, 0, 127}; tx(b, 4); }
  { uint8_t b[4] = {0x00, 0x22, 0, 7}; tx(b, 4); }
  static uint8_t fill[129];
  fill[0] = 0x40;
  for (int i = 1; i < 129; i++) fill[i] = 0xFF;
  for (int p = 0; p < 8; p++) tx(fill, sizeof(fill));
  Serial.println("FILLDONE");
  Serial.println("\\n=== ALL TESTS PASSED ===");
}
void loop() { delay(1000); }
`;

async function run() {
  const b64 = await compile(firmware);
  const flash = new Uint8Array(new SharedArrayBuffer(4 * 1024 * 1024));
  flash.set(new Uint8Array(Buffer.from(b64, 'base64').toString('binary').split('').map(c => c.charCodeAt(0))));
  const rom = readFileSync(resolve(__dirname, '../rom/esp32-v3-rom.bin'));

  const proxy = new SimulatorWorker();
  let out = '';
  proxy._onUART = (b) => { out += String.fromCharCode(b); };
  proxy._onError = (e) => console.error('\n[test] Error:', e.message);

  await proxy.init('ESP32', { flashSizeMB: 4, mmuPages: 64, strapValue: 0x13, budget: 5000000 }, flash, rom);
  await proxy.attachI2cSlave(0, 0x3c);
  proxy.run();

  const oled = makeSsd1306();
  let pending = [];
  const t0 = Date.now();
  while (Date.now() - t0 < 150000) {
    await new Promise(r => setTimeout(r, 200));
    proxy.pollUart();
    for (const e of await proxy.pollI2c(0)) {
      if (e.t === 'start') { pending = []; }
      else if (e.t === 'write') { pending.push(e.byte); }
      else if (e.t === 'stop' && pending.length) { oled.transact(pending); pending = []; }
    }
    if (out.includes('FILLDONE')) break;
  }
  // Drain anything left in flight, then stop.
  for (const e of await proxy.pollI2c(0)) {
    if (e.t === 'write') pending.push(e.byte);
    else if (e.t === 'stop' && pending.length) { oled.transact(pending); pending = []; }
  }
  proxy.stop();
  await new Promise(r => setTimeout(r, 100));
  proxy.terminate();

  console.log(out);
  console.log(`[test] vramFill=${oled.vramFill} displayOn=${oled.isOn}`);
  if (oled.vramFill <= 0 || !oled.isOn) { console.log('[test] FAILED'); process.exit(1); }
  console.log('[test] PASSED');
}

run().catch(e => { console.error(e); process.exit(1); });
