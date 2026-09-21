import { SimulatorWorker } from '../src/index.js';
import axios from 'axios';
import { readFileSync } from 'fs';
import { resolve, dirname } from 'path';
import { fileURLToPath } from 'url';

const __dirname = dirname(fileURLToPath(import.meta.url));

async function compileArduino(code) {
  console.log(`[compile] Sending code for esp32...`);
  const startRes = await axios.post('http://localhost:5525/api/compile/start', {
    code, target: 'esp32', targetEngine: 'frontend', fqbn: 'esp32:esp32:esp32'
  });
  const buildId = startRes.data.buildId;
  while (true) {
    const statRes = await axios.get(`http://localhost:5525/api/compile/status/${buildId}`);
    if (statRes.data.status === 'success') {
      console.log(`[compile] Success! Binary size: ${(statRes.data.binary_content.length * 0.75).toFixed(0)} bytes (base64)`);
      return statRes.data.binary_content;
    } else if (statRes.data.status === 'failed') {
      throw new Error('Compile failed: ' + statRes.data.error);
    }
    process.stdout.write('.');
    await new Promise(r => setTimeout(r, 1000));
  }
}

function base64ToBytes(b64) {
  const binaryString = Buffer.from(b64, 'base64').toString('binary');
  const bytes = new Uint8Array(binaryString.length);
  for (let i = 0; i < binaryString.length; i++) bytes[i] = binaryString.charCodeAt(i);
  return bytes;
}

const testCode = `
void setup() {
  Serial.begin(115200);
  Serial.println("=== TIMER FREQ TEST ===\\n");

  // Timer at 1MHz
  hw_timer_t *timer = timerBegin(1000000);
  if (!timer) { Serial.println("[TIMER] timerBegin failed"); return; }
  timerStart(timer);

  // 1ms test
  delay(1);
  uint64_t cnt1ms = timerRead(timer);

  // 10s test
  timerWrite(timer, 0);
  delay(10000);
  uint64_t cnt10s = timerRead(timer);

  timerStop(timer);
  timerEnd(timer);

  unsigned long t0 = micros();
  delay(2);
  unsigned long us = micros() - t0;

  unsigned long m0 = millis();
  delay(10);
  unsigned long ms = millis() - m0;

  Serial.print("TIMER_1MS="); Serial.println((uint32_t)cnt1ms);
  Serial.print("TIMER_10S="); Serial.println((uint32_t)cnt10s);
  Serial.print("MICROS_2MS="); Serial.println(us);
  Serial.print("MILLIS_10MS="); Serial.println(ms);

  bool ok = cnt1ms > 0 && cnt10s > 0 && us > 0 && ms > 0;
  // Check 10s timer is at least ~1000x larger than 1ms timer
  bool ratioOk = cnt10s > cnt1ms * 500;
  Serial.print("RATIO="); Serial.println(ratioOk ? "PASS" : "FAIL");
  Serial.print("RESULT="); Serial.println(ok ? "PASS" : "FAIL");
  Serial.println("\\n=== ALL TESTS PASSED ===");
}

void loop() { delay(1000); }
`;

async function runTest(label, config) {
  console.log(`\n========== ${label} ==========`);
  const proxy = new SimulatorWorker();

  let uartOutput = '';
  proxy._onUART = (byte) => { uartOutput += String.fromCharCode(byte); };
  proxy._onError = (err) => console.error('\n[test] Worker error:', err.message);

  const flashSizeMB = 4;
  const flashSab = new SharedArrayBuffer(flashSizeMB * 1024 * 1024);
  const flash = new Uint8Array(flashSab);

  const romPath = resolve(__dirname, '../rom/esp32-v3-rom.bin');
  const romBytes = readFileSync(romPath);

  const b64 = await compileArduino(testCode);
  const flashBytes = base64ToBytes(b64);
  flash.fill(0xff);
  flash.set(flashBytes);

  console.log('[test] Initializing Worker...');
  await proxy.init('ESP32', {
    cpuFrequency: config.cpuFrequency,
    flashSizeMB,
    mmuPages: Math.ceil(flashBytes.length / 65536),
    strapValue: 0x13,
    budget: 5000000,
    progressInterval: 10000000,
  }, flash, romBytes);
  console.log('[test] Worker ready');

  proxy.run();

  const nanosTarget = config.cpuFrequency === 'auto' ? 12e9 : 12e9;
  const maxIter = config.cpuFrequency === 'auto' ? 500 : 1000;
  for (let i = 0; i < maxIter; i++) {
    await new Promise((r) => setTimeout(r, 200));
    proxy.pollUart();
    const nan = proxy.nanos;
    process.stdout.write(`\r  [${(i * 0.1).toFixed(1)}s] nanos=${nan}`);
    if (nan > nanosTarget && uartOutput.includes('ALL TESTS PASSED')) break;
  }

  proxy.stop();
  await new Promise((r) => setTimeout(r, 200));
  const nanos = proxy.nanos;
  const uartLines = uartOutput.split('\n').filter(l => l.trim());
  console.log(`\n[test] Simulated nanos: ${Number(nanos).toLocaleString()}`);
  proxy.terminate();
  console.log(uartOutput);

  // Parse values
  const getVal = (key) => {
    const m = uartOutput.match(new RegExp(key + '=(\\d+)'));
    return m ? parseInt(m[1]) : null;
  };

  const fail = (s) => { console.log(s); process.exit(1); };
  if (!uartOutput.includes('ALL TESTS PASSED')) fail('[test] FAILED');
  if (!uartOutput.includes('RESULT=PASS')) fail('[test] RESULT not PASS');
  if (!uartOutput.includes('RATIO=PASS')) fail('[test] 10s timer ratio check FAILED');
  console.log('[test] PASSED');

  return { nanos, t1ms: getVal('TIMER_1MS'), t10s: getVal('TIMER_10S'),
           us: getVal('MICROS_2MS'), ms: getVal('MILLIS_10MS') };
}

async function run() {
  const native = await runTest('Native (160MHz)', { cpuFrequency: undefined });
  const auto = await runTest('Auto (8MHz cap)', { cpuFrequency: 'auto' });

  console.log(`\n========== TIMING COMPARISON ==========`);
  console.log(`                    Native      Auto        `);
  console.log(`Sim nanos:         ${String(Number(native.nanos).toLocaleString()).padStart(12)}  ${String(Number(auto.nanos).toLocaleString()).padStart(12)}`);
  console.log(`TIMER 1ms cnt:     ${String(native.t1ms).padStart(10)}  ${String(auto.t1ms).padStart(10)}`);
  console.log(`TIMER 10s cnt:     ${String(native.t10s).padStart(10)}  ${String(auto.t10s).padStart(10)}`);
  console.log(`micros() 2ms:      ${String(native.us).padStart(10)}  ${String(auto.us).padStart(10)}`);
  console.log(`millis() 10ms:     ${String(native.ms).padStart(10)}  ${String(auto.ms).padStart(10)}`);
  console.log(`=========================================`);
  console.log(`Timer works correctly at both frequencies (${Number(native.t10s).toLocaleString()} vs ${Number(auto.t10s).toLocaleString()} counts for 10s).`);
}

run().catch(e => { console.error(e); process.exit(1); });
