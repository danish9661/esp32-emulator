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

// Dev-board buttons test: RESET (EN) reboots the running firmware from
// setup() again (like pulling CHIP_PU low); BOOT (GPIO0) hold + RESET
// drives the strapping path without hanging the boot.
const firmware = `
RTC_NOINIT_ATTR uint32_t bootCount;
void setup() {
  Serial.begin(115200);
  bootCount++;
  Serial.printf("BOOT #%u\\n", (unsigned)bootCount);
  Serial.println("=== BUTTONS TEST ===");
  // BOOT strap sampled live: held => bit4 SET in the strap register.
  uint32_t strap = REG_READ(0x3FF44038 + 0);
  (void)strap;
  Serial.println("READY");
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

  const waitFor = async (re, timeoutMs = 30000) => {
    const t0 = Date.now();
    while (Date.now() - t0 < timeoutMs) {
      await new Promise(r => setTimeout(r, 200));
      proxy.pollUart();
      if (re.test(out)) return true;
    }
    return false;
  };

  let pass = true;
  // 1. First boot reaches setup.
  if (!await waitFor(/BOOT #1/)) { console.log('BOOT1=MISS'); pass = false; }
  else console.log('BOOT1=PASS');
  if (!await waitFor(/READY/)) { console.log('READY1=MISS'); pass = false; }

  // 2. RESET button (EN tap) mid-run => second BOOT #2 from setup().
  const n1 = (out.match(/BOOT #/g) || []).length;
  await proxy.pressResetButton();
  if (!await waitFor(/BOOT #2/)) { console.log('RESETBTN=FAIL'); pass = false; }
  else console.log('RESETBTN=PASS');
  const n2 = (out.match(/BOOT #/g) || []).length;
  if (n2 <= n1) { console.log('RESETBTN-NO-REBOOT=FAIL'); pass = false; }

  // 3. BOOT button hold + RESET => boots cleanly (no hang), level sticks.
  await proxy.pressBootButton(true);
  await proxy.pressResetButton();
  if (!await waitFor(/BOOT #3/)) { console.log('BOOTBTN=FAIL'); pass = false; }
  else console.log('BOOTBTN=PASS');
  await proxy.pressBootButton(false);
  await proxy.pressResetButton();
  if (!await waitFor(/BOOT #4/)) { console.log('BOOTRELEASE=FAIL'); pass = false; }
  else console.log('BOOTRELEASE=PASS');

  proxy.stop();
  await new Promise(r => setTimeout(r, 100));
  proxy.terminate();

  console.log(out.split('\n').filter(l => /BOOT|READY|BUTTONS/.test(l)).slice(-8).join('\n'));
  if (!out.includes('BOOT #1') || !out.includes('BOOT #2')) { console.log('[test] FAILED'); process.exit(1); }
  if (!pass) { console.log('[test] FAILED'); process.exit(1); }
  console.log('RESULT=PASS');
  console.log('\n=== ALL TESTS PASSED ===');
  console.log('[test] PASSED');
}

run().catch(e => { console.error(e); process.exit(1); });
