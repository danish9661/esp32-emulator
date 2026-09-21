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

// RTC_I2C controller (firmware-driven WRITE) + ULP I2C instruction against
// the shared virtual slave table. Firmware writes pointer+value to slave
// 0x48 via CTRL/TRANS_START (polled completion), programs SENS sel0, then a
// ULP program round-trips its own byte, cross-reads the firmware-written
// byte, and probes an unprogrammed slave (NACK = 0xFF).
const firmware = `
#include "esp32/ulp.h"
#include "soc/rtc_i2c_reg.h"
#include "soc/sens_reg.h"
#include "soc/soc.h"
enum { ULP_MAGIC_ADDR = 16 };
const ulp_insn_t prog[] = {
  I_I2C_WRITE(0, 0x10, 0xAB),
  I_I2C_READ(0, 0x10),
  I_MOVI(R1, ULP_MAGIC_ADDR),
  I_ST(R0, R1, 0),
  I_I2C_READ(0, 0x00),
  I_ST(R0, R1, 2),
  I_I2C_READ(1, 0x10),
  I_ST(R0, R1, 1),
  I_HALT(),
};
static bool rtc_i2c_write_byte(uint8_t b) {
  REG_WRITE(RTC_I2C_DATA_REG, b);
  REG_WRITE(RTC_I2C_CTRL_REG, RTC_I2C_MS_MODE | RTC_I2C_TRANS_START);
  for (int i = 0; i < 10000; i++) {
    if (REG_READ(RTC_I2C_INT_RAW_REG) & RTC_I2C_MASTER_TRANS_COMPLETE_INT_RAW) {
      REG_WRITE(RTC_I2C_INT_CLR_REG, RTC_I2C_MASTER_TRANS_COMPLETE_INT_CLR);
      return true;
    }
  }
  return false;
}
void setup() {
  Serial.begin(115200);
  Serial.println("=== RTCI2C TEST ===");
  bool pass = true;
  REG_WRITE(RTC_I2C_SLAVE_ADDR_REG, 0x48);
  bool w0 = rtc_i2c_write_byte(0x00);  // pointer = sub 0
  bool w1 = rtc_i2c_write_byte(0x5A);  // slave[0] = 0x5A
  Serial.print("RTCI2C_WR="); Serial.println(w0 && w1 ? "PASS" : "FAIL");
  if (!w0 || !w1) pass = false;
  // ULP slave sel0 = 0x48 (SENS_I2C_SLAVE_ADDR0 lives in [21:11]).
  REG_WRITE(SENS_SAR_SLAVE_ADDR1_REG, (0x48 << 11));
  volatile uint32_t *slow = (volatile uint32_t *)0x50000000;
  size_t psize = sizeof(prog) / sizeof(ulp_insn_t);
  esp_err_t err = ulp_process_macros_and_load(0, prog, &psize);
  Serial.print("ULP_LOAD="); Serial.println(err == ESP_OK ? "PASS" : "FAIL");
  if (err != ESP_OK) pass = false;
  err = ulp_run(0);
  Serial.print("ULP_RUN="); Serial.println(err == ESP_OK ? "PASS" : "FAIL");
  if (err != ESP_OK) pass = false;
  uint32_t m0 = slow[ULP_MAGIC_ADDR] & 0xFFFF;
  uint32_t m1 = slow[ULP_MAGIC_ADDR + 1] & 0xFFFF;
  uint32_t m2 = slow[ULP_MAGIC_ADDR + 2] & 0xFFFF;
  Serial.printf("ULP_I2C=%x %x %x\\n", (unsigned)m0, (unsigned)m1, (unsigned)m2);
  if (m0 != 0xAB) { Serial.println("ULP_RW=FAIL"); pass = false; }
  else Serial.println("ULP_RW=PASS");
  if (m2 != 0x5A) { Serial.println("ULP_XRW=FAIL"); pass = false; }
  else Serial.println("ULP_XRW=PASS");
  if (m1 != 0xFF) { Serial.println("ULP_NACK=FAIL"); pass = false; }
  else Serial.println("ULP_NACK=PASS");
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

  for (let i = 0; i < 200; i++) {
    await new Promise(r => setTimeout(r, 200));
    proxy.pollUart();
    if (out.includes('ALL TESTS PASSED')) break;
  }
  proxy.stop();
  await new Promise(r => setTimeout(r, 100));
  proxy.terminate();

  const get = (re) => { const m = out.match(re); return m ? m[1] : 'MISSING'; };
  console.log('RTCI2C_WR=' + get(/RTCI2C_WR=(\w+)/), 'ULP_LOAD=' + get(/ULP_LOAD=(\w+)/), 'ULP_RUN=' + get(/ULP_RUN=(\w+)/));
  console.log('ULP_I2C=' + get(/ULP_I2C=([0-9a-f ]+)/));
  console.log('ULP_RW=' + get(/ULP_RW=(\w+)/), 'ULP_XRW=' + get(/ULP_XRW=(\w+)/), 'ULP_NACK=' + get(/ULP_NACK=(\w+)/));
  const result = get(/RESULT=(\w+)/);
  console.log('RESULT=' + result);
  if (result !== 'PASS') { console.error('[test] FAILED'); process.exit(1); }
  console.log('[test] PASSED');
}

run().catch(e => { console.error(e); process.exit(1); });
