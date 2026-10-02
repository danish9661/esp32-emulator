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
#include <soc/soc.h>
#define BOD_REG (0x3FF48000 + 0xD4)
#define BOD_INT_ENA (0x3FF48000 + 0x3C)
#define BOD_INT_ST (0x3FF48000 + 0x44)
RTC_NOINIT_ATTR uint32_t bootCount;
void setup() {
  Serial.begin(115200);
  int reason = (int)esp_reset_reason();
  if (reason != 9) bootCount = 0;
  bootCount++;
  Serial.printf("BOOT #%d reason=%d\\n", bootCount, reason);
  if (reason == 9) {
    if (bootCount == 2) {
      Serial.println("BODINT=PASS");
      uint32_t ena = REG_READ(BOD_INT_ENA);
      REG_WRITE(BOD_INT_ENA, ena & ~0x80u);
      Serial.println("RST_READY");
      delay(1500);
      REG_WRITE(BOD_REG, (1u << 30) | (2u << 27) | (1u << 26));
      delay(3000);
      Serial.println("BODRESET=FAIL");
    } else {
      Serial.println("BODRESET=PASS");
      Serial.println("\\n=== ALL TESTS PASSED ===");
    }
    return;
  }
  Serial.println("LOW_READY");
  for (int i = 0; i < 80; i++) {
    uint32_t st = REG_READ(BOD_INT_ST);
    uint32_t det = REG_READ(BOD_REG);
    if ((st & 0x80) || (det & 0x80000000u)) { Serial.printf("TRIP st=0x%x det=0x%x\\n", (unsigned)st, (unsigned)det); break; }
    delay(50);
  }
  delay(4000);
  Serial.println("BODINT=FAIL");
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

  // Host poll loop runs on WALL time (50ms) while the guest runs on SIM
  // time (~1000x in idle): after a reboot the guest can print LOW_READY and
  // finish its whole trip poll before the next host poll even sees it. So
  // voltage commands must be EDGE-triggered on first sighting of each phase
  // marker, and the low rail must be asserted SYNCHRONOUSLY in the same
  // poll that first sees LOW_READY (not one poll later — the guest may
  // already be past its trip window).
  //
  // RACE NOTE (observed): between the guest printing LOW_READY and the host
  // asserting 2000, the guest's 80x50ms trip poll can complete entirely at
  // nominal voltage (no trip latched), then delay(4000) expires and it
  // prints BODINT=FAIL — even though the rail IS low one poll later. The
  // fix is ordering, not timing: assert 2000 in the SAME poll iteration
  // that first observes LOW_READY (below), and additionally pre-arm the
  // rail low as soon as the first boot's setup runs (BOOT #1 seen, before
  // LOW_READY prints) so the dip is already present when the poll starts.
  let lo = false, boots = 0, sawRstReady = false, railLow = false;
  let lastOutLen = 0;
  for (let i = 0; i < 250; i++) {
    await new Promise(r => setTimeout(r, 50));
    proxy.pollUart();
    // NOTE: do NOT blindly restore 3300 on every BOOT — the BODRESET phase
    // NEEDS the rail low (2000) across the reboot: the firmware re-arms
    // RST_ENA while still browned-out and the reset must fire again
    // (level-sensitive BOD). Restoring 3300 here would clear DET and the
    // second reset would never come (observed: reboot loop, reason=7).
    // Only the very first boot (power-on, before LOW_READY) gets nominal.
    const nboots = (out.match(/BOOT #/g) || []).length;
    if (nboots === 1 && !out.includes('LOW_READY')) { await proxy.setVoltageMv(3300).catch(()=>{}); }
    if (!lo && out.includes('LOW_READY')) { lo = true; railLow = true; await proxy.setVoltageMv(2000).catch(()=>{}); }
    if (!sawRstReady && out.includes('RST_READY')) { sawRstReady = true; }
    // Re-assert the active rail level every poll (cheap, idempotent): this
    // closes the mid-reset command race — whatever the worker observed,
    // the next eval sees the intended level.
    if (railLow && !out.includes('BODRESET=PASS')) { await proxy.setVoltageMv(2000); }
    if (out !== undefined && out.length !== lastOutLen) { lastOutLen = out.length; }
    if (out.includes('ALL TESTS PASSED')) break;
  }
  proxy.stop();
  await new Promise(r => setTimeout(r, 100));
  proxy.terminate();

  console.log(out);
  if (!out.includes('BODINT=PASS') || !out.includes('BODRESET=PASS') || !out.includes('ALL TESTS PASSED')) {
    console.log('[test] FAILED'); process.exit(1);
  }
  console.log('[test] PASSED');
}

run().catch(e => { console.error(e); process.exit(1); });
