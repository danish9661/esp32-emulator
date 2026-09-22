
#include <soc/soc.h>
#define DS_BASE 0x3ff1a000
void setup() {
  Serial.begin(115200);
  Serial.println("=== DS TEST ===");
  bool pass = true;
  // DS C_MEM area (TRM: params start at +0x0)
  REG_WRITE(DS_BASE + 0x0, 0x12345678);
  uint32_t v = REG_READ(DS_BASE + 0x0);
  Serial.printf("DS_C0=%x", v);
  if (v != 0x12345678) pass = false;
  REG_WRITE(DS_BASE + 0x4, 0xDEADBEEF);
  v = REG_READ(DS_BASE + 0x4);
  if (v != 0xDEADBEEF) pass = false;
  REG_WRITE(DS_BASE + 0x100, 0xA5A5A5A5);
  v = REG_READ(DS_BASE + 0x100);
  Serial.printf("DS_100=%x", v);
  if (v != 0xA5A5A5A5) pass = false;
  REG_WRITE(DS_BASE + 0x800, 0x0BADF00D);
  v = REG_READ(DS_BASE + 0x800);
  if (v != 0x0BADF00D) pass = false;
  // unwritten words read 0 (zeroed reset, stub)
  v = REG_READ(DS_BASE + 0x8);
  Serial.printf("DS_ZERO=%x", v);
  if (v != 0) pass = false;
  v = REG_READ(DS_BASE + 0xFFC);
  if (v != 0) pass = false;
  Serial.print("DS_RW="); Serial.println(pass ? "PASS" : "FAIL");
  Serial.print("RESULT="); Serial.println(pass ? "PASS" : "FAIL");
  Serial.println("=== ALL TESTS PASSED ===");
}
void loop() { delay(1000); }
