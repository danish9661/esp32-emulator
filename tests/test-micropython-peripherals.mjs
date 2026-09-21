import { SimulatorWorker } from '../src/index.js';
import { readFileSync, existsSync } from 'fs';
import { resolve, dirname } from 'path';
import { fileURLToPath } from 'url';

const __dirname = dirname(fileURLToPath(import.meta.url));
const CANDIDATES = ['/tmp/micropython-esp32.bin', resolve(__dirname, 'micropython-esp32.bin')];
let binPath = CANDIDATES.find(p => existsSync(p));
if (!binPath) {
  console.log('[test] SKIP — MicroPython bin not found (download to /tmp/micropython-esp32.bin to enable)');
  console.log('  curl -L -o /tmp/micropython-esp32.bin https://micropython.org/resources/firmware/ESP32_GENERIC-20260824-v1.29.0.bin');
  process.exit(0);
}
const bin = readFileSync(binPath);
console.log(`[test] MicroPython bin: ${bin.length} bytes`);

const flash = new Uint8Array(new SharedArrayBuffer(4*1024*1024));
flash.fill(0xff);
flash.set(bin, 0x1000);

const sim = new SimulatorWorker();
let uart = '';
sim._onUART = b => { uart += String.fromCharCode(b); };

await sim.init('ESP32', { flashSizeMB: 4, mmuPages: 30, strapValue: 0x13, budget: 5_000_000_000 }, flash);
sim.run();
const sleep = ms => new Promise(r => setTimeout(r, ms));
for (let i = 0; i < 30; i++) {
  await sleep(250);
  sim.pollUart();
  if (uart.includes('>>>')) break;
}
if (!uart.includes('MicroPython')) {
  console.error('[test] FAILED: MicroPython did not boot');
  console.error(uart.slice(-2000));
  process.exit(1);
}
console.log('[test] Booted to REPL');

// Helper to send command and wait for prompt
async function send(cmd, expect, timeout = 5000) {
  uart = '';
  sim.sendUart(cmd + '\r\n');
  const start = Date.now();
  while (Date.now() - start < timeout) {
    await sleep(100);
    sim.pollUart();
    if (uart.includes('>>>') && uart.includes(cmd.split(';')[0].split(' ')[0])) break;
  }
  const ok = expect ? uart.includes(expect) : !uart.includes('Traceback');
  const result = uart.slice(-1000).replace(/\r/g, '');
  console.log(`[test] ${cmd.slice(0,60)} => ${ok ? 'PASS' : 'FAIL'}: ${JSON.stringify(result.slice(-200))}`);
  return { ok, uart: result };
}

let passed = 0, failed = 0;
function check(name, cond) {
  if (cond) { passed++; console.log(`[test] PASS: ${name}`); }
  else { failed++; console.error(`[test] FAIL: ${name}`); }
}

// Basic REPL
let r = await send('print(1+2)', '3'); check('REPL print', r.ok);

// machine module
r = await send('import machine; print("machine OK")', 'machine OK'); check('import machine', r.ok);

// GPIO
r = await send('from machine import Pin; p=Pin(2, Pin.OUT); p.value(1); print("GPIO", p.value())', 'GPIO'); check('GPIO Pin', r.ok);

// ADC
r = await send('from machine import ADC; a=ADC(Pin(36)); print("ADC", a.read())', 'ADC'); check('ADC', r.ok);

// PWM
r = await send('from machine import PWM; pwm=PWM(Pin(2), freq=1000, duty=512); print("PWM", pwm.freq()); pwm.deinit(); print("PWM OK")', 'PWM OK'); check('PWM', r.ok);

// I2C
r = await send('from machine import I2C, Pin; i=I2C(0, scl=Pin(22), sda=Pin(21), freq=100000); print("I2C", i.scan())', 'I2C'); check('I2C', r.ok);

// SPI
r = await send('from machine import SPI, Pin; spi=SPI(2, baudrate=1000000, sck=Pin(18), mosi=Pin(23), miso=Pin(19)); print("SPI OK"); spi.deinit()', 'SPI OK'); check('SPI', r.ok);

// Timer
r = await send('from machine import Timer; t=Timer(0); t.init(period=1000, mode=Timer.PERIODIC, callback=lambda x: None); print("Timer OK"); t.deinit()', 'Timer OK'); check('Timer', r.ok);

// UART
r = await send('from machine import UART; u=UART(1, baudrate=115200, tx=1, rx=3); print("UART", u.any()); u.deinit()', 'UART'); check('UART', r.ok);

// Time
r = await send('import time; print("ticks", time.ticks_ms())', 'ticks'); check('time.ticks_ms', r.ok);

// esp32
r = await send('import esp32; print("esp32", esp32.raw_temperature())', 'esp32'); check('esp32.raw_temperature', r.ok);

// Final summary
console.log(`\n[test] ${passed} passed, ${failed} failed`);
if (failed === 0) console.log('[test] === ALL MICROPYTHON PERIPHERAL TESTS PASSED ===');
sim.terminate?.();
process.exit(failed > 0 ? 1 : 0);
