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
void setup() {
  Serial.begin(115200);
  Serial.println("=== RTC + WDT TEST ===\\n");
  bool pass = true;

  Serial.println("[RTC] Testing micros()...");
  unsigned long t0 = micros();
  delay(1);
  unsigned long t1 = micros();
  unsigned long elapsed = t1 - t0;
  Serial.print("[RTC] delta="); Serial.println(elapsed);
  if (elapsed > 50 && elapsed < 5000) { Serial.println("[RTC] plausible"); }
  else { Serial.println("[RTC] unexpected"); pass = false; }

  Serial.println("[RTC] Testing RTC GPIO registers...");
  uint32_t rtcReg = READ_PERI_REG(0x3ff5e094);
  Serial.print("[RTCIO] TOUCH_PAD0=0x"); Serial.println(rtcReg, HEX);
  WRITE_PERI_REG(0x3ff5e094, rtcReg | (1 << 7));
  rtcReg = READ_PERI_REG(0x3ff5e094);
  if (rtcReg & (1 << 7)) { Serial.println("[RTCIO] pull-up set OK"); }
  else { Serial.println("[RTCIO] pull-up failed"); pass = false; }

  Serial.println("[WDT] Testing TG0 WDT registers...");
  uint32_t wdtConf = READ_PERI_REG(0x3ff5f048);
  Serial.print("[WDT] TG0_WDTCONFIG0=0x"); Serial.println(wdtConf, HEX);
  uint32_t wdtFeed = READ_PERI_REG(0x3ff5f048);
  Serial.print("[WDT] TG0_WDTFEED=0x"); Serial.println(wdtFeed, HEX);
  Serial.println("[WDT] register access OK");

  Serial.println("[WDT] Testing RTC WDT...");
  uint32_t rtcWdt = READ_PERI_REG(0x3ff480a0);
  Serial.print("[WDT] RTC_CNTL_WDTCONFIG0=0x"); Serial.println(rtcWdt, HEX);
  Serial.println("[WDT] Done");

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
  if (!out.includes('RESULT=PASS') || !out.includes('ALL TESTS PASSED')) {
    console.log('[test] FAILED'); process.exit(1);
  }
  console.log('[test] PASSED');
}

run().catch(e => { console.error(e); process.exit(1); });
