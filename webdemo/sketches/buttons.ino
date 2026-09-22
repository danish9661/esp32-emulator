
RTC_NOINIT_ATTR uint32_t bootCount;
void setup() {
  Serial.begin(115200);
  bootCount++;
  Serial.printf("BOOT #%u", (unsigned)bootCount);
  Serial.println("=== BUTTONS TEST ===");
  // BOOT strap sampled live: held => bit4 SET in the strap register.
  uint32_t strap = REG_READ(0x3FF44038 + 0);
  (void)strap;
  Serial.println("READY");
}
void loop() { delay(1000); }
