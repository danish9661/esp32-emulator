// Acceptance §3: firmware-driven GPIO-out LED cell (esp32-emu.md).
// Firmware drives GPIO2 HIGH; the host reads the level back through the SAB
// (proxy.getGpioOut) and observes the edge through pollGpioChanges.
// Requires the compile server (:5525).
//
// The firmware PARKS on a UART byte while each level is asserted
// ("LED_ON" -> wait -> "LED_OFF" -> wait). Without that gate the whole
// HIGH/LOW sequence can retire between two host polls — the emulator is much
// faster than the 200 ms sampling cadence — and the level readback then
// samples whichever state happens to be current (historically flaky: it
// passed on slow machines and failed once the JIT made the guest faster).
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
static void waitHost() {
  unsigned long t0 = millis();
  while (Serial.available() <= 0 && millis() - t0 < 20000) delay(5);
  while (Serial.available() > 0) (void)Serial.read();
}
void setup() {
  Serial.begin(115200);
  Serial.println("=== GPIOLED TEST ===");
  pinMode(2, OUTPUT);
  digitalWrite(2, HIGH);
  Serial.println("LED_ON");
  waitHost();
  digitalWrite(2, LOW);
  Serial.println("LED_OFF");
  waitHost();
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

  let sawHigh = null, sawLow = null, sawHighEdge = false, sawLowEdge = false;
  let releasedOn = false, releasedOff = false;
  const t0 = Date.now();
  while (Date.now() - t0 < 120000) {
    await new Promise(r => setTimeout(r, 100));
    proxy.pollUart();
    for (const e of await proxy.pollGpioChanges()) {
      if (e.pin === 2 && e.level === 1) sawHighEdge = true;
      if (e.pin === 2 && e.level === 0) sawLowEdge = true;
    }
    // The guest parks in waitHost() with the level asserted, so the readback
    // is deterministic instead of racing the LED_OFF write.
    if (!releasedOn && out.includes('LED_ON')) {
      const g = await proxy.getGpioOut(2);
      if (g.level === null || g.level === undefined) throw new Error('getGpioOut returned null');
      sawHigh = g.level;
      releasedOn = true;
      proxy.sendUart('A');
    } else if (releasedOn && !releasedOff && out.includes('LED_OFF')) {
      const g = await proxy.getGpioOut(2);
      if (g.level === null || g.level === undefined) throw new Error('getGpioOut returned null');
      sawLow = g.level;
      releasedOff = true;
      proxy.sendUart('B');
    }
    if (out.includes('ALL TESTS PASSED') && sawHigh !== null && sawLow !== null) break;
  }
  proxy.stop();
  await new Promise(r => setTimeout(r, 100));
  proxy.terminate();

  console.log(out);
  console.log(`[test] level@LED_ON=${sawHigh} level@LED_OFF=${sawLow} highEdge=${sawHighEdge} lowEdge=${sawLowEdge}`);
  if (sawHigh !== 1 || sawLow !== 0 || !sawHighEdge || !sawLowEdge) {
    console.log('[test] FAILED'); process.exit(1);
  }
  console.log('[test] PASSED');
}

run().catch(e => { console.error(e); process.exit(1); });
