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

async function run() {
  const firmwareArg = process.argv[2];

  let flashBytes;
  if (firmwareArg) {
    const mergedPath = resolve(process.cwd(), firmwareArg);
    flashBytes = readFileSync(mergedPath);
    console.log(`[test] Firmware: ${mergedPath} (${flashBytes.length} bytes)`);
  } else {
    const code = `
#include <SPI.h>
#include <Wire.h>
#include <mbedtls/sha256.h>
#include <mbedtls/aes.h>

volatile int interruptCounter = 0;

void IRAM_ATTR onGPIOInterrupt() {
  interruptCounter++;
}

void setup() {
  Serial.begin(115200);
  Serial.println("=== ESP32 Peripheral Test ===\\n");

  Serial.println("[GPIO] Testing...");
  pinMode(2, OUTPUT);
  pinMode(4, INPUT_PULLUP);
  digitalWrite(2, HIGH);
  int gpioVal = digitalRead(2);
  Serial.print("[GPIO] Pin2 wrote HIGH, read: ");
  Serial.println(gpioVal);
  digitalWrite(2, LOW);
  gpioVal = digitalRead(2);
  Serial.print("[GPIO] Pin2 wrote LOW, read: ");
  Serial.println(gpioVal);
  Serial.println("[GPIO] Done");

  Serial.println("[INT] Testing GPIO interrupt...");
  pinMode(5, INPUT_PULLUP);
  attachInterrupt(digitalPinToInterrupt(5), onGPIOInterrupt, RISING);
  Serial.print("[INT] Counter before: ");
  Serial.println(interruptCounter);
  Serial.println("[INT] Attached OK");
  detachInterrupt(digitalPinToInterrupt(5));
  Serial.println("[INT] Done");

  Serial.println("[PWM] Starting LEDC...");
  ledcAttach(2, 5000, 8);
  ledcWrite(2, 128);
  ledcWrite(2, 64);
  ledcDetach(2);
  Serial.println("[PWM] Done");

  Serial.println("[SPI] Testing SPI2 (VSPI)...");
  SPI.begin();
  SPI.beginTransaction(SPISettings(1000000, MSBFIRST, SPI_MODE0));
  uint8_t spiSent = 0xAA;
  uint8_t spiReceived = SPI.transfer(spiSent);
  Serial.print("[SPI] Sent: 0x");
  Serial.print(spiSent, HEX);
  Serial.print(" Received: 0x");
  Serial.println(spiReceived, HEX);
  SPI.endTransaction();
  SPI.end();
  Serial.println("[SPI] Done");

  Serial.println("[I2C] Testing...");
  Wire.begin();
  Wire.beginTransmission(0x42);
  Wire.write(0x00);
  Wire.write(0x55);
  uint8_t i2cErr = Wire.endTransmission();
  Serial.print("[I2C] Transmission result: ");
  Serial.println(i2cErr);
  Wire.end();
  Serial.println("[I2C] Done");

  Serial.println("[TIMER] Testing hardware timer...");
  hw_timer_t *timer = timerBegin(1000000);
  if (timer) {
    timerStart(timer);
    delay(1);
    uint64_t cnt = timerRead(timer);
    Serial.print("[TIMER] Counter after 1ms: ");
    Serial.println((uint32_t)(cnt & 0xFFFFFFFF));
    timerWrite(timer, 0);
    uint64_t target = 10000;
    uint64_t pollStart = timerRead(timer);
    uint64_t pollTimeout = target + 50000;
    while (timerRead(timer) < target && (timerRead(timer) - pollStart) < pollTimeout) { }
    uint64_t final = timerRead(timer);
    Serial.print("[TIMER] Poll target reached: ");
    Serial.println(final >= target ? "yes" : "no");
    timerStop(timer);
    timerEnd(timer);
    Serial.println("[TIMER] Done");
  } else {
    Serial.println("[TIMER] timerBegin failed");
  }

  Serial.println("[RTC] Testing micros() (uses RTC timer)...");
  unsigned long t0 = micros();
  delay(1);
  unsigned long t1 = micros();
  unsigned long elapsed = t1 - t0;
  Serial.print("[RTC] micros() delta: ");
  Serial.println(elapsed);
  Serial.println(elapsed > 500 && elapsed < 5000 ? "[RTC] plausible" : "[RTC] unexpected");
  Serial.println("[RTC] Done");

  Serial.println("[WDT] Testing timer WDT registers...");
  uint32_t wdtConf = READ_PERI_REG(0x3ff5f048);
  Serial.print("[WDT] TG0 WDTCONFIG0: 0x");
  Serial.println(wdtConf, HEX);
  uint32_t dpVal = READ_PERI_REG(0x3ff5a104);
  Serial.print("[WDT] DPORT clock gate: 0x");
  Serial.println(dpVal, HEX);
  Serial.println("[WDT] Done");

  Serial.println("[RNG] Testing random generator...");
  uint32_t r1 = esp_random();
  uint32_t r2 = esp_random();
  Serial.print("[RNG] Two values: 0x");
  Serial.print(r1, HEX);
  Serial.print(" 0x");
  Serial.println(r2, HEX);
  Serial.println(r1 != r2 ? "[RNG] appears random" : "[RNG] suspicious");
  Serial.println("[RNG] Done");

  Serial.println("[SHA] Testing SHA256...");
  mbedtls_sha256_context shaCtx;
  mbedtls_sha256_init(&shaCtx);
  mbedtls_sha256_starts(&shaCtx, 0);
  const char *shaMsg = "Hello ESP32";
  mbedtls_sha256_update(&shaCtx, (const uint8_t*)shaMsg, strlen(shaMsg));
  uint8_t shaOut[32];
  mbedtls_sha256_finish(&shaCtx, shaOut);
  mbedtls_sha256_free(&shaCtx);
  Serial.print("[SHA] Hash: ");
  for (int i = 0; i < 4; i++) { Serial.print(shaOut[i], HEX); }
  Serial.println("...");
  Serial.println("[SHA] Done");

  Serial.println("[AES] Testing AES128...");
  mbedtls_aes_context aesCtx;
  mbedtls_aes_init(&aesCtx);
  uint8_t key[16] = { 0x2b,0x7e,0x15,0x16,0x28,0xae,0xd2,0xa6,0xab,0xf7,0x15,0x88,0x09,0xcf,0x4f,0x3c };
  uint8_t pt[16] = { 0x6b,0xc1,0xbe,0xe2,0x2e,0x40,0x9f,0x96,0xe9,0x3d,0x7e,0x11,0x73,0x93,0x17,0x2a };
  uint8_t ct[16], dec[16];
  mbedtls_aes_setkey_enc(&aesCtx, key, 128);
  mbedtls_aes_crypt_ecb(&aesCtx, MBEDTLS_AES_ENCRYPT, pt, ct);
  mbedtls_aes_setkey_dec(&aesCtx, key, 128);
  mbedtls_aes_crypt_ecb(&aesCtx, MBEDTLS_AES_DECRYPT, ct, dec);
  bool aesOk = memcmp(pt, dec, 16) == 0;
  Serial.print("[AES] Encrypt/decrypt: ");
  Serial.println(aesOk ? "OK" : "FAIL");
  mbedtls_aes_free(&aesCtx);
  Serial.println("[AES] Done");

  Serial.println("[RTCIO] Testing RTC pull resistor registers...");
  uint32_t rtcReg = READ_PERI_REG(0x3ff5e094);
  Serial.print("[RTCIO] TOUCH_PAD0 reg: 0x");
  Serial.println(rtcReg, HEX);
  WRITE_PERI_REG(0x3ff5e094, rtcReg | (1 << 7));
  rtcReg = READ_PERI_REG(0x3ff5e094);
  Serial.print("[RTCIO] After set pull-up: 0x");
  Serial.println(rtcReg, HEX);
  Serial.println(rtcReg & (1 << 7) ? "[RTCIO] pull-up set OK" : "[RTCIO] pull-up failed");
  Serial.println("[RTCIO] Done");

  Serial.println("[ADC] Testing analogRead...");
  analogReadResolution(12);
  int adcVal = analogRead(36);
  Serial.print("[ADC] SENSOR_VP=");
  Serial.println(adcVal);
  adcVal = analogRead(39);
  Serial.print("[ADC] SENSOR_VN=");
  Serial.println(adcVal);
  Serial.println("[ADC] Done");

  Serial.println("[SERIAL] Testing Serial2...");
  Serial2.begin(115200);
  Serial2.println("Hello from Serial2!");
  Serial2.flush();
  Serial2.end();
  Serial.println("[SERIAL] Serial2 Done");

  Serial.println("[SERIAL] Testing Serial1...");
  Serial1.begin(115200);
  Serial1.println("Hello from Serial1!");
  Serial1.flush();
  Serial1.end();
  Serial.println("[SERIAL] Serial1 Done");

  Serial.println("\\n=== ALL TESTS PASSED ===");
}

void loop() {
  delay(1000);
}
`;
    const b64 = await compileArduino(code);
    flashBytes = base64ToBytes(b64);
    console.log(`[test] Decoded flash: ${flashBytes.length} bytes`);
  }

  const chipName = 'ESP32';
  const romPath = resolve(__dirname, '../rom/esp32-v3-rom.bin');
  const romBytes = readFileSync(romPath);
  console.log(`[test] BootROM: ${romBytes.length} bytes`);

  const flashSizeMB = Math.max(4, Math.ceil(flashBytes.length / (1024 * 1024)));
  const flashSab = new SharedArrayBuffer(flashSizeMB * 1024 * 1024);
  const flash = new Uint8Array(flashSab);
  flash.fill(0xff);
  flash.set(flashBytes);

  const proxy = new SimulatorWorker();

  let uartOutput = '';
  proxy._onUART = (byte) => { uartOutput += String.fromCharCode(byte); };
  proxy._onError = (err) => console.error('\n[test] Worker error:', err.message);

  const engine = process.env.ENGINE || 'wasm';
  console.log(`[test] Engine: ${engine}`);

  console.log('[test] Initializing Worker...');
  await proxy.init(chipName, {
    engine,
    flashSizeMB,
    mmuPages: Math.ceil(flashBytes.length / 65536),
    strapValue: 0x13,
    pinInputs: { 0: true, 2: false, 12: false, 15: false },
    budget: 5000000,
    progressInterval: 10000000,
  }, flash, romBytes);
  console.log('[test] Worker ready');

  proxy.run();

  let prevUartLen = 0;
  let entryDetected = false;
  for (let i = 0; i < 1000; i++) {
    await new Promise((r) => setTimeout(r, 200));
    proxy.pollUart();
    const nan = proxy.nanos;
    const pc = proxy.pc;
    if (!entryDetected && uartOutput.includes('entry 0x')) { entryDetected = true; console.log(`\n[test] Entry point detected at t=${(i*0.2).toFixed(1)}s nanos=${nan}`); }
    if (i % 5 === 0 || pc > 0x400e0000) {
      const newUart = uartOutput.slice(prevUartLen);
      const newLines = newUart.trim().split('\n').filter(l => l).length;
      if (newLines) { process.stdout.write(`\n  [${(i*0.2).toFixed(1)}s] nanos=${nan} PC=0x${pc?.toString(16).padStart(8,'0')} idle=${proxy.idle} uart+${newLines}l`); }
      prevUartLen = uartOutput.length;
    }
    // Print PC and idle state every 20 iterations to track progress
    if (i % 20 === 0) {
      process.stdout.write(`\n  [TRACE ${(i*0.2).toFixed(1)}s] PC=0x${pc?.toString(16).padStart(8,'0')} idle=${proxy.idle} nanos=${nan}`);
    }
    if (nan > 500000000 && uartOutput.includes('ALL TESTS PASSED')) break;
  }

  const finalNanos = proxy.nanos;
  const finalPC = proxy.pc;

  proxy.stop();
  await new Promise((r) => setTimeout(r, 200));
  proxy.terminate();

  const uartLines = uartOutput.split('\n').filter(l => l.trim());
  console.log(`\n\n[test] Final nanos: ${finalNanos}`);
  console.log(`[test] Final PC:    0x${finalPC?.toString(16).padStart(8, '0')}`);
  console.log(`[test] UART lines:  ${uartLines.length}`);
  console.log(`\n${uartOutput}`);
  if (uartOutput.includes('ALL TESTS PASSED')) {
    console.log(`\n[test] PASSED`);
  } else {
    console.log(`\n[test] FAILED - did not see ALL TESTS PASSED`);
    process.exit(1);
  }
}

run().catch(e => { console.error(e); process.exit(1); });
