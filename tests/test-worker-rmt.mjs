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

const firmware = `
#include <driver/rmt.h>

void setup() {
  Serial.begin(115200);
  Serial.println("=== RMT TEST ===\\n");
  rmt_config_t cfg = RMT_DEFAULT_CONFIG_TX(GPIO_NUM_2, RMT_CHANNEL_0);
  cfg.clk_div = 80;
  esp_err_t err = rmt_config(&cfg);
  Serial.print("RMT_CONFIG="); Serial.println(err == ESP_OK ? "PASS" : "FAIL");
  if (err == ESP_OK) {
    err = rmt_driver_install(RMT_CHANNEL_0, 0, 0);
    Serial.print("RMT_INSTALL="); Serial.println(err == ESP_OK ? "PASS" : "FAIL");
    if (err == ESP_OK) {
      // RX channel 2 (virtual wire: TX ch0 items loop into armed RX).
      rmt_config_t rxcfg = RMT_DEFAULT_CONFIG_RX(GPIO_NUM_4, RMT_CHANNEL_2);
      rxcfg.clk_div = 80;
      err = rmt_config(&rxcfg);
      Serial.print("RMT_RXCONFIG="); Serial.println(err == ESP_OK ? "PASS" : "FAIL");
      if (err == ESP_OK) {
        err = rmt_driver_install(RMT_CHANNEL_2, 1000, 0);
        Serial.print("RMT_RXINSTALL="); Serial.println(err == ESP_OK ? "PASS" : "FAIL");
      }
      if (err == ESP_OK) {
        RingbufHandle_t rb = NULL;
        rmt_get_ringbuf_handle(RMT_CHANNEL_2, &rb);
        Serial.printf("GOTRB=%d\\n", rb ? 1 : 0);
        Serial.printf("RXSTART=%d\\n", (int)rmt_rx_start(RMT_CHANNEL_2, true));
        rmt_item32_t items[3];
        items[0].duration0 = 100; items[0].level0 = 1;
        items[0].duration1 = 100; items[0].level1 = 0;
        items[1].duration0 = 200; items[1].level0 = 1;
        items[1].duration1 = 200; items[1].level1 = 0;
        items[2].val = 0;
        err = rmt_write_items(RMT_CHANNEL_0, items, 3, true);
        Serial.print("RMT_TX="); Serial.println(err == ESP_OK ? "PASS" : "FAIL");
        if (err == ESP_OK) {
          size_t rxlen = 0;
          rmt_item32_t *rx = (rmt_item32_t *)xRingbufferReceive(rb, &rxlen, 3000 / portTICK_PERIOD_MS);
          Serial.print("RMT_RX="); Serial.println(rx ? "PASS" : "FAIL");
          if (rx) {
            Serial.printf("RX_LEN=%u D0=%u,%u D1=%u,%u\\n", (unsigned)rxlen,
              (unsigned)rx[0].duration0, (unsigned)rx[0].duration1,
              (unsigned)rx[1].duration0, (unsigned)rx[1].duration1);
            if (rxlen < 8 || rx[0].duration0 != 100 || rx[0].duration1 != 100 ||
                rx[1].duration0 != 200 || rx[1].duration1 != 200)
              { Serial.println("RMT_DATA=FAIL"); err = ESP_FAIL; }
            else Serial.println("RMT_DATA=PASS");
            vRingbufferReturnItem(rb, rx);
          } else err = ESP_FAIL;
        }
        rmt_rx_stop(RMT_CHANNEL_2);
        rmt_driver_uninstall(RMT_CHANNEL_2);
      }
      rmt_driver_uninstall(RMT_CHANNEL_0);
      Serial.println("[RMT] uninstall OK");
    }
  }
  bool pass = (err == ESP_OK);
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
  proxy._onUART = (b) => { out += String.fromCharCode(b); };
  proxy._onError = (e) => console.error('\n[test] Error:', e.message);

  await proxy.init('ESP32', { flashSizeMB: 4, mmuPages: 64, strapValue: 0x13, budget: 5000000 }, flash, rom);
  proxy.run();

  for (let i = 0; i < 100; i++) {
    await new Promise(r => setTimeout(r, 200));
    proxy.pollUart();
    if (proxy.nanos > 50000000 && out.includes('ALL TESTS PASSED')) break;
  }
  proxy.stop();
  await new Promise(r => setTimeout(r, 100));
  proxy.terminate();

  console.log(out);
  if (!out.includes('RESULT=PASS') || !out.includes('RMT_TX=PASS') || !out.includes('RMT_RX=PASS') || !out.includes('RMT_DATA=PASS') || !out.includes('ALL TESTS PASSED')) {
    console.log('[test] FAILED'); process.exit(1);
  }
  console.log('[test] PASSED');
}

run().catch(e => { console.error(e); process.exit(1); });
