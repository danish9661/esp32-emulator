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

// I2C slave via the single-chip virtual bus. The slave side is the Arduino
// Wire1 stack (addr 0x42, onReceive/onRequest callbacks); the master side
// drives the new IDF master driver directly (bus 0). Covers: slave RX
// delivery + onReceive, preloaded TX read, and TXEMPTY-triggered onRequest
// refill for the following read. UART 'go' handshakes order host/firmware.
// NOTE: the shared 32B FIFO aliases RX/TX slots, so the preloaded read comes
// before the first master write (real-HW FIFO behavior).
// NOTE: Arduino's legacy Wire master path (endTransmission) returns error 4
// even with no slave attached (pre-existing, ISR-fed sequencing) — the old
// master-only test tolerates it; the bus itself is proven here.
const firmware = `
#include <Wire.h>
#include <driver/i2c_master.h>

volatile bool gotMsg = false;
char smsg[16] = {0};

void onRecv(int n) {
  int i = 0;
  while (Wire1.available() && i < 15) smsg[i++] = (char)Wire1.read();
  smsg[i] = 0;
  gotMsg = true;
}

void onReq() {
  Wire1.write((const uint8_t*)"Hi!", 3);
}

void waitGo() {
  for (int i = 0; i < 6000 && !Serial.available(); i++) delay(10);
  if (Serial.available()) Serial.read();
}

void setup() {
  Serial.begin(115200);
  Serial.println("=== I2CSLAVE TEST ===");
  bool pass = true;

  // Master bus FIRST (order test: master intr_alloc before slave intr_alloc).
  i2c_master_bus_config_t bus_cfg = {};
  bus_cfg.i2c_port = 0;
  bus_cfg.sda_io_num = (gpio_num_t)21;
  bus_cfg.scl_io_num = (gpio_num_t)22;
  bus_cfg.clk_source = I2C_CLK_SRC_DEFAULT;
  bus_cfg.glitch_ignore_cnt = 7;
  bus_cfg.flags.enable_internal_pullup = 1;
  i2c_master_bus_handle_t bus;
  esp_err_t e1 = i2c_new_master_bus(&bus_cfg, &bus);
  Serial.printf("NEWBUS=0x%x\\n", e1);
  i2c_device_config_t dev_cfg = {};
  dev_cfg.dev_addr_length = I2C_ADDR_BIT_LEN_7;
  dev_cfg.device_address = 0x42;
  dev_cfg.scl_speed_hz = 100000;
  i2c_master_dev_handle_t dev;
  esp_err_t e2 = i2c_master_bus_add_device(bus, &dev_cfg, &dev);
  Serial.printf("ADDDEV=0x%x\\n", e2);

  bool slv = Wire1.begin((uint8_t)0x42, 18, 19, 100000);
  Serial.printf("SLAVE_BEGIN=%d\\n", (int)slv);
  Wire1.write((const uint8_t*)"OK", 2);  // preload slave TX for the first master read
  Wire1.onReceive(onRecv);
  Wire1.onRequest(onReq);

  // 1. Prime read: slave TX FIFO is empty (Arduino buffers preload in RAM
  // until onRequest flushes), so this returns filler — but its completion
  // fires the slave TX event, running onRequest which flushes "Hi!".
  uint8_t prime[2] = {0};
  esp_err_t r0 = i2c_master_receive(dev, prime, 2, 1000);
  Serial.printf("PRIME=0x%x\\n", r0);
  if (r0 != ESP_OK) { Serial.println("PRIME_OK=FAIL"); pass = false; }
  else Serial.println("PRIME_OK=PASS");

  // 2. The prime read's onRequest flushed "Hi!" — read it back.
  // (No master writes before this: RX delivery shares FIFO slots with TX.)
  uint8_t r2[4] = {0};
  esp_err_t e3 = i2c_master_receive(dev, r2, 3, 1000);
  Serial.printf("READ2=%c%c%c\\n", r2[0], r2[1], r2[2]);
  if (e3 != ESP_OK || r2[0] != 'H' || r2[1] != 'i' || r2[2] != '!') { Serial.println("REFILL=FAIL"); pass = false; }
  else Serial.println("REFILL=PASS");

  // 3. Master write -> slave onReceive.
  uint8_t wbuf[3] = {0x41, 0x42, 0x43};
  esp_err_t e = i2c_master_transmit(dev, wbuf, 3, 1000);
  Serial.printf("TXERR=0x%x\\n", e);
  uint32_t mraw = REG_READ(0x3FF53000 + 0x20);
  uint32_t mena = REG_READ(0x3FF53000 + 0x28);
  uint32_t sraw = REG_READ(0x3FF67000 + 0x20);
  uint32_t sena = REG_READ(0x3FF67000 + 0x28);
  Serial.printf("MRAW=0x%x MENA=0x%x SRAW=0x%x SENA=0x%x\\n", mraw, mena, sraw, sena);
  if (e != ESP_OK) { Serial.println("WR_OK=FAIL"); pass = false; }
  else Serial.println("WR_OK=PASS");

  Serial.println("SLAVE_READY");
  // Wait for the host 'go' (slave ISR + onReceive have run by then).
  waitGo();

  Serial.printf("GOT=%d MSG=%s\\n", (int)gotMsg, smsg);
  if (!gotMsg || String(smsg) != "ABC") { Serial.println("ONRECV=FAIL"); pass = false; }
  else Serial.println("ONRECV=PASS");

  Serial.print("RESULT="); Serial.println(pass ? "PASS" : "FAIL");
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
  let ready1 = false;
  proxy._onUART = (b) => { out += String.fromCharCode(b); };
  proxy._onError = (e) => console.error('\n[test] Error:', e.message);

  await proxy.init('ESP32', { flashSizeMB: 4, mmuPages: 64, strapValue: 0x13, budget: 5000000, pinInputs: { 18: 1, 19: 1 } }, flash, rom);
  proxy.run();

  const t0 = Date.now();
  while (Date.now() - t0 < 120000) {
    await new Promise(r => setTimeout(r, 200));
    proxy.pollUart();
    if (!ready1 && out.includes('SLAVE_READY')) {
      ready1 = true;
      await new Promise(r => setTimeout(r, 800));
      proxy.sendUart('g');
    }
    if (out.includes('ALL TESTS PASSED')) break;
  }
  proxy.stop();
  await new Promise(r => setTimeout(r, 100));
  proxy.terminate();
  await new Promise(r => setTimeout(r, 4000));

  console.log(out);
  if (!out.includes('PRIME_OK=PASS') || !out.includes('REFILL=PASS') || !out.includes('WR_OK=PASS') || !out.includes('ONRECV=PASS') || !out.includes('RESULT=PASS')) {
    console.log('[test] FAILED'); process.exit(1);
  }
  console.log('[test] PASSED');
}

run().catch(e => { console.error(e); process.exit(1); });
