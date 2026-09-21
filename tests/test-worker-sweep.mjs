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
#include <soc/dport_reg.h>

#define SECURE_BOOT_BASE 0x3ff04000
#define I2C_CFG_BASE     0x3ff4b000
#define SLCHOST_BASE     0x3ff55000
#define FLASH_CRYPT_BASE 0x3ff5b000
#define PID_CTRL_BASE    0x3ff1f000

void setup() {
  Serial.begin(115200);
  Serial.println("=== SWEEP TEST ===\\n");
  bool pass = true;

  // Secure Boot (0x3FF04000) — inside the DPORT window per IDF, use DPORT_REG_*
  DPORT_REG_WRITE(SECURE_BOOT_BASE + 0x0, 0x12345678);
  uint32_t v = DPORT_REG_READ(SECURE_BOOT_BASE + 0x0);
  Serial.printf("[SWEEP] secure_boot0 readback=%x\\n", v);
  if (v != 0x12345678) pass = false;
  DPORT_REG_WRITE(SECURE_BOOT_BASE + 0x4, 0xDEADBEEF);
  v = DPORT_REG_READ(SECURE_BOOT_BASE + 0x4);
  if (v != 0xDEADBEEF) pass = false;
  DPORT_REG_WRITE(SECURE_BOOT_BASE + 0x400, 0xA5);
  v = DPORT_REG_READ(SECURE_BOOT_BASE + 0x400);
  if (v != 0xA5) pass = false;

  // I2C config (0x3FF4B000)
  REG_WRITE(I2C_CFG_BASE + 0x0, 0x00000001);
  v = REG_READ(I2C_CFG_BASE + 0x0);
  Serial.printf("[SWEEP] i2c_cfg0 readback=%x\\n", v);
  if (v != 0x00000001) pass = false;
  REG_WRITE(I2C_CFG_BASE + 0x200, 0x80000000);
  v = REG_READ(I2C_CFG_BASE + 0x200);
  if (v != 0x80000000) pass = false;

  // SLCHOST (0x3FF55000)
  REG_WRITE(SLCHOST_BASE + 0x0, 0x00000003);
  v = REG_READ(SLCHOST_BASE + 0x0);
  Serial.printf("[SWEEP] slchost0 readback=%x\\n", v);
  if (v != 0x00000003) pass = false;
  REG_WRITE(SLCHOST_BASE + 0x44, 0x3FFE0000);
  v = REG_READ(SLCHOST_BASE + 0x44);
  if (v != 0x3FFE0000) pass = false;
  REG_WRITE(SLCHOST_BASE + 0x200, 0x11112222);
  v = REG_READ(SLCHOST_BASE + 0x200);
  if (v != 0x11112222) pass = false;

  // Flash Encryption (0x3FF5B000)
  REG_WRITE(FLASH_CRYPT_BASE + 0x0, 0x00000001);
  v = REG_READ(FLASH_CRYPT_BASE + 0x0);
  Serial.printf("[SWEEP] flash_crypt0 readback=%x\\n", v);
  if (v != 0x00000001) pass = false;
  REG_WRITE(FLASH_CRYPT_BASE + 0x100, 0xCAFEBABE);
  v = REG_READ(FLASH_CRYPT_BASE + 0x100);
  if (v != 0xCAFEBABE) pass = false;

  // PID Controller per-CPU (0x3FF1F000)
  REG_WRITE(PID_CTRL_BASE + 0x0, 0x00000007);
  v = REG_READ(PID_CTRL_BASE + 0x0);
  Serial.printf("[SWEEP] pid_ctrl0 readback=%x\\n", v);
  if (v != 0x00000007) pass = false;
  REG_WRITE(PID_CTRL_BASE + 0x200, 0x1234ABCD);
  v = REG_READ(PID_CTRL_BASE + 0x200);
  if (v != 0x1234ABCD) pass = false;

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
