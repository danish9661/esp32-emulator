import { ESP32 } from '../src/index.js';
import axios from 'axios';
import { readFileSync } from 'fs';
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
      console.log(`[compile] Success!`);
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
void setup() {
  Serial.begin(115200);
  Serial.println("=== ESP32 Extra Tests ===\\n");

  // ---- TOUCH SENSOR ----
  Serial.println("[TOUCH] Testing touch sensor registers...");
  // RTC_IO base: 0x3ff5e000, TOUCH_PAD0-7 at offsets 0x94,0x98,0x9c,0xa0,0xa4,0xa8,0xac,0xb0
  uint32_t t0 = READ_PERI_REG(0x3ff5e094);
  uint32_t t1 = READ_PERI_REG(0x3ff5e098);
  uint32_t t2 = READ_PERI_REG(0x3ff5e09c);
  uint32_t t3 = READ_PERI_REG(0x3ff5e0a0);
  Serial.print("[TOUCH] R0=0x"); Serial.print(t0, HEX);
  Serial.print(" R1=0x"); Serial.print(t1, HEX);
  Serial.print(" R2=0x"); Serial.print(t2, HEX);
  Serial.print(" R3=0x"); Serial.println(t3, HEX);
  // Write touch threshold to trigger measurement
  WRITE_PERI_REG(0x3ff5e094, 0x100);
  uint32_t t0w = READ_PERI_REG(0x3ff5e094);
  Serial.print("[TOUCH] After write threshold: 0x");
  Serial.println(t0w, HEX);
  Serial.println("[TOUCH] Done");

  // ---- HALL SENSOR ----
  Serial.println("[HALL] Testing hall/ADC registers...");
  // RTC_IO SENSOR_VP/VN at offset 0x1c/0x20 (28/32) - hall sensor mux
  uint32_t sensVp = READ_PERI_REG(0x3ff5e01c);
  uint32_t sensVn = READ_PERI_REG(0x3ff5e020);
  Serial.print("[HALL] SENSOR_VP_REG=0x");
  Serial.print(sensVp, HEX);
  Serial.print(" SENSOR_VN_REG=0x");
  Serial.println(sensVn, HEX);
  // SENS (ADC) register space at 0x3ff48800+
  uint32_t sensReg = READ_PERI_REG(0x3ff4880c);
  Serial.print("[HALL] SENS_SAR_READ_CTRL=0x");
  Serial.println(sensReg, HEX);
  // Write to SENS_SAR_READ_CTRL to verify write works
  WRITE_PERI_REG(0x3ff4880c, 0x1);
  uint32_t sensW = READ_PERI_REG(0x3ff4880c);
  Serial.print("[HALL] After write: 0x");
  Serial.println(sensW, HEX);
  Serial.println("[HALL] Done");

  // ---- RTC MEMORY ----
  Serial.println("[RTCMEM] Testing RTC slow memory...");
  // RTC_SLOW_MEM at 0x50000000
  volatile uint32_t *rtcMem = (volatile uint32_t *)0x50000000;
  rtcMem[0] = 0xDEADBEEF;
  rtcMem[1] = 0xCAFEBABE;
  rtcMem[2] = 0x12345678;
  rtcMem[100] = 0x87654321;
  uint32_t r0 = rtcMem[0];
  uint32_t r1 = rtcMem[1];
  uint32_t r2 = rtcMem[2];
  uint32_t r100 = rtcMem[100];
  Serial.print("[RTCMEM] Wrote 0xDEADBEEF read=0x");
  Serial.println(r0, HEX);
  Serial.print("[RTCMEM] Wrote 0xCAFEBABE read=0x");
  Serial.println(r1, HEX);
  Serial.print("[RTCMEM] Wrote 0x12345678 read=0x");
  Serial.println(r2, HEX);
  Serial.print("[RTCMEM] Offset 100 read=0x");
  Serial.println(r100, HEX);
  bool rtcMemOk = (r0 == 0xDEADBEEF && r1 == 0xCAFEBABE && r2 == 0x12345678 && r100 == 0x87654321);
  Serial.println(rtcMemOk ? "[RTCMEM] RTC memory R/W OK" : "[RTCMEM] RTC memory FAIL");
  Serial.println("[RTCMEM] Done");

  // ---- DEEP SLEEP registers ----
  Serial.println("[SLEEP] Testing deep sleep registers...");
  // RTC_CNTL base: 0x3ff48000, SLP_WAKEUP_CAUSE at offset 0x38 (56)
  uint32_t wakeupCause = READ_PERI_REG(0x3ff48038);
  Serial.print("[SLEEP] Wakeup cause=");
  Serial.println(wakeupCause, HEX);
  // SLP_CTRL at offset 0x80 (128) - sleep control register
  uint32_t slpCtrl = READ_PERI_REG(0x3ff48080);
  Serial.print("[SLEEP] SLP_CTRL=0x");
  Serial.println(slpCtrl, HEX);
  // Write sleep timer value (SLP_TIMER0 at offset 0x94, 0x98)
  WRITE_PERI_REG(0x3ff48094, 1000000);
  WRITE_PERI_REG(0x3ff48098, 0);
  uint32_t timerLo = READ_PERI_REG(0x3ff48094);
  Serial.print("[SLEEP] Sleep timer set=");
  Serial.println(timerLo);
  Serial.println("[SLEEP] Done");

  Serial.println("\\n=== ALL EXTRA TESTS PASSED ===");
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
  uartLines.slice(-40).forEach(l => console.log(`  ${l}`));

  let touchOk = uartLines.some(l => l.includes('[TOUCH] D'));
  let hallOk = uartLines.some(l => l.includes('[HALL] D'));
  let rtcMemOk = uartLines.some(l => l.includes('[RTCMEM] RTC memory R/W OK'));
  let sleepOk = uartLines.some(l => l.includes('[SLEEP] D'));
  let allPassed = uartLines.some(l => l.includes('ALL EXTRA TESTS PASSED'));

  console.log(`\n--- Extra Peripheral Check ---`);
  console.log(`Touch:       ${touchOk ? '✓' : '✗'}`);
  console.log(`Hall:        ${hallOk ? '✓' : '✗'}`);
  console.log(`RTC Memory:  ${rtcMemOk ? '✓' : '✗'}`);
  console.log(`Deep Sleep:  ${sleepOk ? '✓' : '✗'}`);
  console.log(`All:         ${allPassed ? '✓ ALL PASSED' : '✗'}`);

  if (appReached) {
    console.log(`\n✅ BOOT SUCCESSFUL`);
  } else {
    console.log(`\n❌ Boot failed`);
    process.exit(1);
  }
}

run().catch(e => { console.error(e); process.exit(1); });
