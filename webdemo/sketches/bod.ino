
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
  Serial.printf("BOOT #%d reason=%d", bootCount, reason);
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
      Serial.println("=== ALL TESTS PASSED ===");
    }
    return;
  }
  Serial.println("LOW_READY");
  for (int i = 0; i < 80; i++) {
    uint32_t st = REG_READ(BOD_INT_ST);
    uint32_t det = REG_READ(BOD_REG);
    if ((st & 0x80) || (det & 0x80000000u)) { Serial.printf("TRIP st=0x%x det=0x%x", (unsigned)st, (unsigned)det); break; }
    delay(50);
  }
  delay(4000);
  Serial.println("BODINT=FAIL");
}
void loop() { delay(1000); }
