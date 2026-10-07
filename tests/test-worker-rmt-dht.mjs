// RMT single-wire sensor tap: DHT22 end-to-end through the published API.
// Firmware sends the DHT22 start pulse with RMT TX (ch0, div 80 = 1MHz so
// 1 tick = 1us) and captures the sensor reply with RMT RX (ch1); the host
// plays the sensor: it verifies the captured start pulse, builds the 41
// -item response (80us+80us header + 40 bits: 50us low + 27us/70us high)
// encoding 55.5% RH / 23.5C, and injects it via the RX tap. Requires the
// compile server (:5525). No gateway needed.
import { SimulatorWorker, RmtTap } from '../src/index.js';
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
  Serial.println("=== RMT-DHT TEST ===");
  bool pass = true;
  rmt_config_t txcfg = RMT_DEFAULT_CONFIG_TX(GPIO_NUM_2, RMT_CHANNEL_0);
  txcfg.clk_div = 80; // 1MHz: 1 tick = 1us
  if (rmt_config(&txcfg) != ESP_OK) { Serial.println("TXCONFIG=FAIL"); pass = false; }
  if (rmt_driver_install(RMT_CHANNEL_0, 0, 0) != ESP_OK) { Serial.println("TXINSTALL=FAIL"); pass = false; }
  rmt_config_t rxcfg = RMT_DEFAULT_CONFIG_RX(GPIO_NUM_4, RMT_CHANNEL_1);
  rxcfg.clk_div = 80;
  if (rmt_config(&rxcfg) != ESP_OK) { Serial.println("RXCONFIG=FAIL"); pass = false; }
  if (rmt_driver_install(RMT_CHANNEL_1, 1000, 0) != ESP_OK) { Serial.println("RXINSTALL=FAIL"); pass = false; }
  RingbufHandle_t rb = NULL;
  rmt_get_ringbuf_handle(RMT_CHANNEL_1, &rb);
  if (!rb) { Serial.println("GOTRB=FAIL"); pass = false; }
  // DHT22 start: >=1ms low, then release (~30us high).
  rmt_item32_t start[2];
  start[0].duration0 = 1100; start[0].level0 = 0;
  start[0].duration1 = 30; start[0].level1 = 1;
  start[1].val = 0;
  if (rmt_rx_start(RMT_CHANNEL_1, true) != ESP_OK) { Serial.println("RXSTART=FAIL"); pass = false; }
  if (rmt_write_items(RMT_CHANNEL_0, start, 2, true) != ESP_OK) { Serial.println("TXSTART=FAIL"); pass = false; }
  // Two frames arrive, in order (virtual-wire determinism: the TX-done
  // loopback lands before the host can react to the TX capture, so the echo
  // is always first):
  //   1. echo of our own start pulse via the single-chip loopback (no wire
  //      on real HW either — the sensor is silent until it sees the start).
  //      Verified + discarded.
  //   2. the host sensor answer injected through the RX tap. The blocking
  //      take retries so wall-clock host round-trips (SAB polls) never
  //      starve the window.
  rmt_item32_t *rx = NULL;
  size_t rxlen = 0;
  for (int attempt = 0; attempt < 30 && !rx; attempt++) {
    rx = (rmt_item32_t *)xRingbufferReceive(rb, &rxlen, 2000 / portTICK_PERIOD_MS);
  }
  if (!rx || rxlen != 8) { Serial.println("DHT_ECHO=FAIL"); pass = false; }
  else {
    rmt_item32_t e0 = rx[0];
    if (e0.duration0 != 1100 || e0.level0 != 0 || e0.duration1 != 30 || e0.level1 != 1)
      { Serial.println("DHT_ECHO=FAIL"); pass = false; }
    else Serial.println("DHT_ECHO=PASS");
    vRingbufferReturnItem(rb, rx);
    rx = NULL;
  }
  rmt_rx_start(RMT_CHANNEL_1, true); // re-arm for the sensor frame
  rxlen = 0;
  for (int attempt = 0; attempt < 30 && !rx; attempt++) {
    rx = (rmt_item32_t *)xRingbufferReceive(rb, &rxlen, 2000 / portTICK_PERIOD_MS);
  }
  if (!rx) { Serial.println("DHT_RX=FAIL"); pass = false; }
  else {
    size_t n = rxlen / 4;
    Serial.printf("DHT_N=%u\\n", (unsigned)n);
    if (n < 42) { Serial.println("DHT_LEN=FAIL"); pass = false; }
    else {
      // items[0] = 80us low + 80us high header; items[1..40] = bits MSB-first.
      bool ok = (rx[0].duration0 == 80 && rx[0].level0 == 0 && rx[0].duration1 == 80 && rx[0].level1 == 1);
      uint8_t data[5] = {0, 0, 0, 0, 0};
      for (int i = 0; ok && i < 40; i++) {
        if (rx[1 + i].duration0 != 50 || rx[1 + i].level0 != 0 || rx[1 + i].level1 != 1) ok = false;
        else if (rx[1 + i].duration1 >= 50) data[i / 8] |= (uint8_t)(1 << (7 - (i % 8)));
        else if (rx[1 + i].duration1 < 20) ok = false;
      }
      uint8_t ck = (uint8_t)(data[0] + data[1] + data[2] + data[3]);
      Serial.printf("DHT_RAW=%02x%02x%02x%02x%02x\\n", data[0], data[1], data[2], data[3], data[4]);
      if (!ok) { Serial.println("DHT_SHAPE=FAIL"); pass = false; }
      else if (ck != data[4]) { Serial.println("DHT_CKSUM=FAIL"); pass = false; }
      else {
        Serial.println("DHT_CKSUM=PASS");
        int hum = (data[0] << 8) | data[1];
        int tmp = (data[2] << 8) | data[3];
        if (tmp & 0x8000) tmp = -(tmp & 0x7fff);
        Serial.printf("DHT_HUM=%d.%d\\n", hum / 10, hum % 10);
        Serial.printf("DHT_TEMP=%d.%d\\n", tmp / 10, abs(tmp % 10));
        if (hum != 555 || tmp != 235) { Serial.println("DHT_VAL=FAIL"); pass = false; }
        else Serial.println("DHT_VAL=PASS");
      }
    }
    vRingbufferReturnItem(rb, rx);
  }
  rmt_rx_stop(RMT_CHANNEL_1);
  rmt_driver_uninstall(RMT_CHANNEL_1);
  rmt_driver_uninstall(RMT_CHANNEL_0);
  Serial.print("RESULT="); Serial.println(pass ? "PASS" : "FAIL");
  Serial.println("\\n=== ALL TESTS PASSED ===");
}
void loop() { delay(1000); }
`;

// 55.5% RH (0x022B) + 23.5C (0x00EB): DHT22 bit stream, MSB-first.
const PAYLOAD = [0x02, 0x2b, 0x00, 0xeb];
PAYLOAD.push((PAYLOAD[0] + PAYLOAD[1] + PAYLOAD[2] + PAYLOAD[3]) & 0xff);

function buildResponse(tickHz) {
  const t = (us) => RmtTap.usToTicks(us, tickHz);
  const words = [RmtTap.item(t(80), 0, t(80), 1)]; // sensor header
  for (const byte of PAYLOAD) {
    for (let i = 7; i >= 0; i--) {
      const bit = (byte >> i) & 1;
      words.push(RmtTap.item(t(50), 0, bit ? t(70) : t(27), 1));
    }
  }
  words.push(0); // terminator
  return words;
}

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
  await proxy.armRmtTx(0);
  proxy.run();

  // 1) Await the guest start pulse on the TX tap and verify its shape.
  let start = [];
  const t0 = Date.now();
  while (Date.now() - t0 < 120000) {
    await new Promise(r => setTimeout(r, 100));
    proxy.pollUart();
    start = await proxy.pollRmtTx(0);
    if (start.length) break;
  }
  if (!start.length) throw new Error('no TX start pulse captured');
  const s0 = RmtTap.splitItem(start[0]);
  console.log(`[test] start pulse: d0=${s0.d0} l0=${s0.l0} d1=${s0.d1} l1=${s0.l1}`);
  if (s0.d0 !== 1100 || s0.l0 !== 0 || s0.d1 !== 30 || s0.l1 !== 1) {
    throw new Error('start pulse shape mismatch');
  }
  console.log('[test] TXSTART=PASS');

  // 2) Play the sensor: divider comes from the firmware config (div 80).
  const hz = await proxy.getRmtTickHz(0);
  console.log(`[test] tickHz=${hz}`);
  if (hz !== 1000000) throw new Error(`tickHz mismatch (got ${hz})`);
  const staged = await proxy.injectRmtRx(1, buildResponse(hz));
  if (staged !== 42) throw new Error(`inject staged ${staged}, want 42`);

  // 3) Await the decoded verdict.
  while (Date.now() - t0 < 180000) {
    await new Promise(r => setTimeout(r, 200));
    proxy.pollUart();
    if (out.includes('RESULT=')) break;
  }
  proxy.stop();
  await new Promise(r => setTimeout(r, 100));
  proxy.terminate();

  console.log(out);
  const ok = out.includes('DHT_ECHO=PASS') && out.includes('DHT_CKSUM=PASS') && out.includes('DHT_HUM=55.5')
    && out.includes('DHT_TEMP=23.5') && out.includes('DHT_VAL=PASS') && out.includes('RESULT=PASS');
  if (!ok) { console.log('[test] FAILED'); process.exit(1); }
  console.log('[test] PASSED');
}

run().catch(e => { console.error(e); process.exit(1); });
