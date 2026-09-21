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

// TWAI NORMAL mode against the virtual CAN peer (the NO_ACK/self-test path
// is covered by test-worker-twai). The peer ACKs every TX (real-HW parity:
// with a node on the bus, transmission completes with TCS) and the host
// injects one peer frame mid-run. UART READY handshakes host/firmware so
// the injection can't land before driver install (install resets the fifo).
const firmware = `
#include <driver/twai.h>
#include <string.h>

void setup() {
  Serial.begin(115200);
  Serial.println("=== TWAINORMAL TEST ===");
  bool pass = true;
  twai_general_config_t g = TWAI_GENERAL_CONFIG_DEFAULT(GPIO_NUM_21, GPIO_NUM_22, TWAI_MODE_NORMAL);
  twai_timing_config_t t = TWAI_TIMING_CONFIG_125KBITS();
  twai_filter_config_t f = TWAI_FILTER_CONFIG_ACCEPT_ALL();
  esp_err_t err = twai_driver_install(&g, &t, &f);
  Serial.print("TWAI_INSTALL="); Serial.println(err == ESP_OK ? "PASS" : "FAIL");
  if (err != ESP_OK) pass = false;
  if (err == ESP_OK) {
    err = twai_start();
    Serial.print("TWAI_START="); Serial.println(err == ESP_OK ? "PASS" : "FAIL");
    if (err != ESP_OK) pass = false;
    // TX in NORMAL mode: the peer ACKs (no self-reception on a real bus).
    twai_message_t txmsg = {};
    txmsg.identifier = 0x456;
    txmsg.data_length_code = 4;
    txmsg.data[0] = 'P'; txmsg.data[1] = 'E'; txmsg.data[2] = 'E'; txmsg.data[3] = 'R';
    err = twai_transmit(&txmsg, 1000 / portTICK_PERIOD_MS);
    Serial.print("TWAI_TX="); Serial.println(err == ESP_OK ? "PASS" : "FAIL");
    if (err != ESP_OK) pass = false;
    Serial.println("READY");
    // RX: the host-injected peer frame.
    twai_message_t rxmsg = {};
    err = twai_receive(&rxmsg, 3000 / portTICK_PERIOD_MS);
    Serial.print("TWAI_RX="); Serial.println(err == ESP_OK ? "PASS" : "FAIL");
    if (err != ESP_OK) pass = false;
    else {
      Serial.printf("RX_ID=0x%x RX_DLC=%d RX_DATA=%c%c%c%c\\n", (unsigned)rxmsg.identifier,
        (int)rxmsg.data_length_code, rxmsg.data[0], rxmsg.data[1], rxmsg.data[2], rxmsg.data[3]);
      if (rxmsg.identifier != 0x123 || rxmsg.data_length_code != 4 ||
          memcmp(rxmsg.data, "CAN!", 4) != 0) { Serial.println("TWAI_DATA=FAIL"); pass = false; }
      else Serial.println("TWAI_DATA=PASS");
    }
    twai_stop();
    twai_driver_uninstall();
  }
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

  await proxy.init('ESP32', { flashSizeMB: 4, mmuPages: 64, strapValue: 0x13, budget: 8000000 }, flash, rom);
  proxy.run();

  let sent = false;
  for (let i = 0; i < 200; i++) {
    await new Promise(r => setTimeout(r, 200));
    proxy.pollUart();
    if (!sent && out.includes('READY')) {
      sent = true;
      await proxy.sendTwaiFrame(0x123, [0x43, 0x41, 0x4E, 0x21]); // "CAN!"
      console.log('[test] peer frame injected');
    }
    if (out.includes('ALL TESTS PASSED')) break;
  }
  proxy.stop();
  await new Promise(r => setTimeout(r, 100));

  // The peer must have captured the firmware's TX (proof the frame went
  // out on the virtual bus — not just locally completed).
  const tx = await proxy.getTwaiTx();
  console.log('[test] peer captured TX:', JSON.stringify(tx));
  proxy.terminate();

  const get = (re) => { const m = out.match(re); return m ? m[1] : 'MISSING'; };
  console.log('TWAI_INSTALL=' + get(/TWAI_INSTALL=(\w+)/), 'TWAI_START=' + get(/TWAI_START=(\w+)/));
  console.log('TWAI_TX=' + get(/TWAI_TX=(\w+)/), 'TWAI_RX=' + get(/TWAI_RX=(\w+)/), 'TWAI_DATA=' + get(/TWAI_DATA=(\w+)/));
  console.log('RX line:', get(/(RX_ID=.*)/));
  const result = get(/RESULT=(\w+)/);
  const txOk = tx && tx.count >= 1 && tx.id === 0x456 && !tx.ext && !tx.rtr && tx.dlc === 4 &&
    tx.data.length === 4 && tx.data[0] === 0x50 && tx.data[1] === 0x45 && tx.data[2] === 0x45 && tx.data[3] === 0x52;
  console.log('RESULT=' + result, 'TXCAP=' + (txOk ? 'PASS' : 'FAIL'));
  if (result !== 'PASS' || !txOk) { console.error('[test] FAILED'); process.exit(1); }
  console.log('[test] PASSED');
}

run().catch(e => { console.error(e); process.exit(1); });
