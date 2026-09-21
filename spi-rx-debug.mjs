import { SimulatorWorker } from './src/index.js';
import axios from 'axios';
import { readFileSync } from 'fs';

async function compile(code) {
  const r = await axios.post('http://localhost:5525/api/compile/start', { code, target: 'esp32', targetEngine: 'frontend', fqbn: 'esp32:esp32:esp32' });
  while (true) {
    const s = await axios.get(`http://localhost:5525/api/compile/status/${r.data.buildId}`);
    if (s.data.status === 'success') return s.data.binary_content;
    if (s.data.status === 'failed') throw new Error('Compile failed: ' + s.data.error);
    await new Promise(r => setTimeout(r, 1000));
  }
}

const firmware = `
#include <SPI.h>
#include <driver/spi_slave.h>
#include <soc/soc.h>
#define SPI2_BASE 0x3ff64000
void dumpW(const char *tag) {
  uint32_t w0 = REG_READ(SPI2_BASE + 0x80);
  Serial.printf("%s W0=0x%08x\\n", tag, w0);
}
void setup() {
  Serial.begin(115200);
  Serial.println("=== SPIRX ===");
  spi_bus_config_t buscfg = {};
  buscfg.mosi_io_num = 13; buscfg.miso_io_num = 12; buscfg.sclk_io_num = 14;
  buscfg.quadwp_io_num = -1; buscfg.quadhd_io_num = -1;
  spi_slave_interface_config_t slvcfg = {};
  slvcfg.spics_io_num = 15; slvcfg.queue_size = 2; slvcfg.mode = 0;
  slvcfg.post_trans_cb = NULL;
  spi_slave_initialize(HSPI_HOST, &buscfg, &slvcfg, 0);
  static uint8_t tx1[4] = {'W','X','Y','Z'};
  static uint8_t rx1[4] = {0};
  spi_slave_transaction_t t1 = {};
  t1.length = 32; t1.tx_buffer = tx1; t1.rx_buffer = rx1;
  spi_slave_queue_trans(HSPI_HOST, &t1, portMAX_DELAY);
  dumpW("AFTER_QUEUE");
  SPI.begin();
  static uint8_t mosi[4] = {'a','b','c','d'};
  static uint8_t miso[4] = {0};
  SPI.beginTransaction(SPISettings(1000000, MSBFIRST, SPI_MODE0));
  SPI.transferBytes(mosi, miso, 4);
  SPI.endTransaction();
  dumpW("AFTER_MASTER");
  spi_slave_transaction_t *ret = NULL;
  esp_err_t g = spi_slave_get_trans_result(HSPI_HOST, &ret, 5000 / portTICK_PERIOD_MS);
  dumpW("AFTER_GET");
  Serial.printf("GET=%d RX=%c%c%c%c MISO=%c%c%c%c\\n", (int)g, rx1[0], rx1[1], rx1[2], rx1[3], miso[0], miso[1], miso[2], miso[3]);
  Serial.println("=== DONE ===");
}
void loop() { delay(1000); }
`;

const b64 = await compile(firmware);
const flash = new Uint8Array(new SharedArrayBuffer(4 * 1024 * 1024));
flash.set(new Uint8Array(Buffer.from(b64, 'base64').toString('binary').split('').map(c => c.charCodeAt(0))));
const rom = readFileSync('./rom/esp32-v3-rom.bin');
const proxy = new SimulatorWorker();
let out = '';
proxy._onUART = (b) => { out += String.fromCharCode(b); };
await proxy.init('ESP32', { flashSizeMB: 4, mmuPages: 64, strapValue: 0x13, budget: 5000000 }, flash, rom);
proxy.run();
for (let i = 0; i < 30; i++) {
  await new Promise(r => setTimeout(r, 500));
  proxy.pollUart();
  if (out.includes('=== DONE ===')) break;
}
proxy.terminate();
await new Promise(r => setTimeout(r, 3000));
console.log(out);
process.exit(0);
