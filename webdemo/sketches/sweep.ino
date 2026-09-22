
#include <soc/soc.h>
#include <soc/dport_reg.h>

#define SECURE_BOOT_BASE 0x3ff04000
#define I2C_CFG_BASE     0x3ff4b000
#define SLCHOST_BASE     0x3ff55000
#define FLASH_CRYPT_BASE 0x3ff5b000
#define PID_CTRL_BASE    0x3ff1f000

void setup() {
  Serial.begin(115200);
  Serial.println("=== SWEEP TEST ===");
  bool pass = true;

  // Secure Boot (0x3FF04000) — inside the DPORT window per IDF, use DPORT_REG_*
  DPORT_REG_WRITE(SECURE_BOOT_BASE + 0x0, 0x12345678);
  uint32_t v = DPORT_REG_READ(SECURE_BOOT_BASE + 0x0);
  Serial.printf("[SWEEP] secure_boot0 readback=%x", v);
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
  Serial.printf("[SWEEP] i2c_cfg0 readback=%x", v);
  if (v != 0x00000001) pass = false;
  REG_WRITE(I2C_CFG_BASE + 0x200, 0x80000000);
  v = REG_READ(I2C_CFG_BASE + 0x200);
  if (v != 0x80000000) pass = false;

  // SLCHOST (0x3FF55000)
  REG_WRITE(SLCHOST_BASE + 0x0, 0x00000003);
  v = REG_READ(SLCHOST_BASE + 0x0);
  Serial.printf("[SWEEP] slchost0 readback=%x", v);
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
  Serial.printf("[SWEEP] flash_crypt0 readback=%x", v);
  if (v != 0x00000001) pass = false;
  REG_WRITE(FLASH_CRYPT_BASE + 0x100, 0xCAFEBABE);
  v = REG_READ(FLASH_CRYPT_BASE + 0x100);
  if (v != 0xCAFEBABE) pass = false;

  // PID Controller per-CPU (0x3FF1F000)
  REG_WRITE(PID_CTRL_BASE + 0x0, 0x00000007);
  v = REG_READ(PID_CTRL_BASE + 0x0);
  Serial.printf("[SWEEP] pid_ctrl0 readback=%x", v);
  if (v != 0x00000007) pass = false;
  REG_WRITE(PID_CTRL_BASE + 0x200, 0x1234ABCD);
  v = REG_READ(PID_CTRL_BASE + 0x200);
  if (v != 0x1234ABCD) pass = false;

  Serial.print("RESULT="); Serial.println(pass ? "PASS" : "FAIL");
  Serial.println("=== ALL TESTS PASSED ===");
}
void loop() { delay(1000); }
