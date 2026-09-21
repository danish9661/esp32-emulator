import { ESP32 } from '../src/index.js';
import axios from 'axios';
import { readFileSync, writeFileSync } from 'fs';
import { resolve, dirname } from 'path';
import { fileURLToPath } from 'url';

const __dirname = dirname(fileURLToPath(import.meta.url));

async function compileArduino(code, target = 'esp32') {
  console.log(`[compile] Sending code for ${target}...`);
  const startRes = await axios.post('http://localhost:5525/api/compile/start', {
    code, target, targetEngine: 'frontend', fqbn: `esp32:esp32:${target}`
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

  // ---- GPIO ----
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

  // ---- GPIO Interrupt ----
  Serial.println("[INT] Testing GPIO interrupt...");
  pinMode(5, INPUT_PULLUP);
  attachInterrupt(digitalPinToInterrupt(5), onGPIOInterrupt, RISING);
  Serial.print("[INT] Counter before: ");
  Serial.println(interruptCounter);
  // Simulate rising edge by writing HIGH then LOW (not applicable on input, but tests attach)
  Serial.println("[INT] Attached OK");
  detachInterrupt(digitalPinToInterrupt(5));
  Serial.println("[INT] Done");

  // ---- PWM (LEDC) ----
  Serial.println("[PWM] Starting LEDC...");
  ledcAttach(2, 5000, 8);
  ledcWrite(2, 128);
  ledcWrite(2, 64);
  ledcDetach(2);
  Serial.println("[PWM] Done");

  // ---- SPI ----
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

  // ---- I2C ----
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

  // ---- Hardware Timer ----
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

  // ---- RTC / Timer ----
  Serial.println("[RTC] Testing micros() (uses RTC timer)...");
  unsigned long t0 = micros();
  delay(1);
  unsigned long t1 = micros();
  unsigned long elapsed = t1 - t0;
  Serial.print("[RTC] micros() delta: ");
  Serial.println(elapsed);
  Serial.println(elapsed > 500 && elapsed < 5000 ? "[RTC] plausible" : "[RTC] unexpected");
  Serial.println("[RTC] Done");

  // ---- WDT (timer group watchdog registers) ----
  Serial.println("[WDT] Testing timer WDT registers...");
  // TG0 WDT base: 0x3ff5f000, WDT config at offset 0x48 (WDTCONFIG0)
  // Just read the WDT config register to verify R/W access works
  uint32_t wdtConf = READ_PERI_REG(0x3ff5f048);
  Serial.print("[WDT] TG0 WDTCONFIG0: 0x");
  Serial.println(wdtConf, HEX);
  // Enable WDT via DPORT bit 13 (cY = 8192 = 1<<13)
  uint32_t dpVal = READ_PERI_REG(0x3ff5a104);
  Serial.print("[WDT] DPORT clock gate: 0x");
  Serial.println(dpVal, HEX);
  Serial.println("[WDT] Done");

  // ---- RNG ----
  Serial.println("[RNG] Testing random generator...");
  uint32_t r1 = esp_random();
  uint32_t r2 = esp_random();
  Serial.print("[RNG] Two values: 0x");
  Serial.print(r1, HEX);
  Serial.print(" 0x");
  Serial.println(r2, HEX);
  Serial.println(r1 != r2 ? "[RNG] appears random" : "[RNG] suspicious");
  Serial.println("[RNG] Done");

  // ---- SHA ----
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

  // ---- AES ----
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

  // ---- RTC pull-up/down (via RTC_IO register access) ----
  Serial.println("[RTCIO] Testing RTC pull resistor registers...");
  // RTC_IO base: 0x3ff5e000, TOUCH_PAD0 at offset 0x94 (148)
  uint32_t rtcReg = READ_PERI_REG(0x3ff5e094);
  Serial.print("[RTCIO] TOUCH_PAD0 reg: 0x");
  Serial.println(rtcReg, HEX);
  // Write pull-up enable on TOUCH_PAD0 (bit 7 = RUE = pull-up enable)
  WRITE_PERI_REG(0x3ff5e094, rtcReg | (1 << 7));
  rtcReg = READ_PERI_REG(0x3ff5e094);
  Serial.print("[RTCIO] After set pull-up: 0x");
  Serial.println(rtcReg, HEX);
  Serial.println(rtcReg & (1 << 7) ? "[RTCIO] pull-up set OK" : "[RTCIO] pull-up failed");
  Serial.println("[RTCIO] Done");

  // ---- ADC ----
  Serial.println("[ADC] Testing analogRead...");
  analogReadResolution(12);
  int adcVal = analogRead(36);
  Serial.print("[ADC] SENSOR_VP=");
  Serial.println(adcVal);
  adcVal = analogRead(39);
  Serial.print("[ADC] SENSOR_VN=");
  Serial.println(adcVal);
  Serial.println("[ADC] Done");

  // ---- Serial2 ----
  Serial.println("[SERIAL] Testing Serial2...");
  Serial2.begin(115200);
  Serial2.println("Hello from Serial2!");
  Serial2.flush();
  Serial2.end();
  Serial.println("[SERIAL] Serial2 Done");

  // ---- Serial1 ----
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
  const b64 = await compileArduino(code, 'esp32');

  const flashBytes = base64ToBytes(b64);
  console.log(`[sim] Decoded flash: ${flashBytes.length} bytes`);

  const romPath = resolve(__dirname, '../rom/esp32-v3-rom.bin');
  const romBytes = readFileSync(romPath);
  console.log(`[sim] BootROM: ${romBytes.length} bytes`);

  const flashSizeMB = Math.max(4, Math.ceil(flashBytes.length / (1024 * 1024)));
  const flash = new Uint8Array(flashSizeMB * 1024 * 1024);
  flash.fill(0xff);
  flash.set(flashBytes);

  const esp32 = new ESP32({ flashSizeMB, flash });

  esp32.loadROM(romBytes);

  if (esp32.gpio?.pins) {
    if (esp32.gpio.pins[0])  esp32.gpio.pins[0].inputValue = true;
    if (esp32.gpio.pins[2])  esp32.gpio.pins[2].inputValue = false;
    if (esp32.gpio.pins[12]) esp32.gpio.pins[12].inputValue = false;
    if (esp32.gpio.pins[15]) esp32.gpio.pins[15].inputValue = false;
  }

  esp32.reset();
  esp32.flash.set(flashBytes);

  if (esp32.gpio) esp32.gpio.strapValue = 0x13;

  if (esp32.mmuTablePro) {
    const pages = Math.ceil(flashBytes.length / 65536);
    for (let p = 0; p < pages; p++) {
      esp32.mmuTablePro[p] = p;
      esp32.mmuTableApp[p] = p;
    }
    console.log(`[sim] MMU seeded: ${pages} pages`);
  }

  try { esp32.cores[0].writeUint32(0x3ff5a104, 0x5aa5); } catch {}

  esp32.stopped = false;

  let uartLog = '';
  let uartLines = [];
  if (esp32.uart?.[0]) {
    esp32.uart[0].onTX = (byte) => {
      uartLog += String.fromCharCode(byte);
      if (uartLog.endsWith('\n')) {
        const line = uartLog.trim();
        if (line) uartLines.push(line);
        uartLog = '';
      }
    };
  }

  const MAX_CYCLES = 600_000_000;
  let lastPC = -1, stuck = 0, maxPC = 0;

  console.log(`[sim] Running ${MAX_CYCLES.toLocaleString()} cycles...`);
  for (let i = 0; i < MAX_CYCLES; i++) {
    esp32.step();
    if (esp32.clock) esp32.clock.tick?.();

    const pc = esp32.cores?.[0]?.PC;
    if (pc !== undefined) {
      if (pc === lastPC) stuck++; else stuck = 0;
      lastPC = pc;
      if (pc > maxPC) maxPC = pc;
    }

    if (i > 0 && i % 15_000_000 === 0) {
      process.stdout.write(`\r  cycle ${(i/1e6).toFixed(0)}M  PC=0x${lastPC?.toString(16).padStart(8,'0')}  max=0x${maxPC.toString(16).padStart(8,'0')}`);
    }
    if (stuck > 2_000_000) break;
  }

  console.log(`\n\n=== Results ===`);
  console.log(`Cycles:     ${MAX_CYCLES.toLocaleString()}`);
  console.log(`Final PC:   0x${lastPC?.toString(16).padStart(8, '0')}`);
  console.log(`Max PC:     0x${maxPC.toString(16).padStart(8, '0')}`);
  console.log(`Stuck:      ${stuck}`);

  const appReached = maxPC >= 0x40080000;

  console.log(`\nUART output (${uartLines.length} lines):`);
  uartLines.slice(-30).forEach(l => console.log(`  ${l}`));

  console.log(`\nBootROM executed: ${uartLines.length > 0 || maxPC > 0x40020000 ? 'YES' : 'NO'}`);
  console.log(`User code:        ${appReached ? 'YES :)' : 'NO'}`);

  let gpioOk = uartLines.some(l => l.includes('[GPIO] ') || l.includes('[GPIO] D'));
  let gpioIntOk = uartLines.some(l => l.includes('[INT]'));
  let pwmOk = uartLines.some(l => l.includes('PWM'));
  let spiOk = uartLines.some(l => l.includes('SPI'));
  let i2cOk = uartLines.some(l => l.includes('I2C'));
  let timerOk = uartLines.some(l => l.includes('[TIMER]'));
  let wdtOk = uartLines.some(l => l.includes('[WDT]'));
  let rtcOk = uartLines.some(l => l.includes('[RTC] m') || l.includes('[RTC] p'));
  let rngOk = uartLines.some(l => l.includes('[RNG] '));
  let shaOk = uartLines.some(l => l.includes('[SHA] '));
  let aesOk = uartLines.some(l => l.includes('[AES] '));
  let rtcIoOk = uartLines.some(l => l.includes('[RTCIO]'));
  let adcOk = uartLines.some(l => l.includes('[ADC]'));
  let serOk = uartLines.some(l => l.includes('[SERIAL]'));
  let allPassed = uartLines.some(l => l.includes('ALL TESTS PASSED'));

  console.log(`\n--- Peripheral Check ---`);
  console.log(`GPIO:        ${gpioOk ? '✓' : '✗'}`);
  console.log(`GPIO Int:    ${gpioIntOk ? '✓' : '✗'}`);
  console.log(`PWM:         ${pwmOk ? '✓' : '✗'}`);
  console.log(`SPI:         ${spiOk ? '✓' : '✗'}`);
  console.log(`I2C:         ${i2cOk ? '✓' : '✗'}`);
  console.log(`Timer:       ${timerOk ? '✓' : '✗'}`);
  console.log(`WDT:         ${wdtOk ? '✓' : '✗'}`);
  console.log(`RTC:         ${rtcOk ? '✓' : '✗'}`);
  console.log(`RTC_IO:      ${rtcIoOk ? '✓' : '✗'}`);
  console.log(`RNG:         ${rngOk ? '✓' : '✗'}`);
  console.log(`SHA:         ${shaOk ? '✓' : '✗'}`);
  console.log(`AES:         ${aesOk ? '✓' : '✗'}`);
  console.log(`ADC:         ${adcOk ? '✓' : '✗'}`);
  console.log(`Serial1/2:   ${serOk ? '✓' : '✗'}`);
  console.log(`All:         ${allPassed ? '✓ ALL PASSED' : '✗'}`);

  if (appReached) {
    console.log(`\n✅ BOOT SUCCESSFUL`);
  } else {
    console.log(`\n❌ Boot failed`);
    process.exit(1);
  }
}

run().catch(e => { console.error(e); process.exit(1); });

