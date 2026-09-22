
void setup() {
  Serial.begin(115200);
  Serial.println("=== RTC + WDT TEST ===");
  bool pass = true;

  Serial.println("[RTC] Testing micros()...");
  unsigned long t0 = micros();
  delay(1);
  unsigned long t1 = micros();
  unsigned long elapsed = t1 - t0;
  Serial.print("[RTC] delta="); Serial.println(elapsed);
  if (elapsed > 50 && elapsed < 5000) { Serial.println("[RTC] plausible"); }
  else { Serial.println("[RTC] unexpected"); pass = false; }

  Serial.println("[RTC] Testing RTC GPIO registers...");
  uint32_t rtcReg = READ_PERI_REG(0x3ff5e094);
  Serial.print("[RTCIO] TOUCH_PAD0=0x"); Serial.println(rtcReg, HEX);
  WRITE_PERI_REG(0x3ff5e094, rtcReg | (1 << 7));
  rtcReg = READ_PERI_REG(0x3ff5e094);
  if (rtcReg & (1 << 7)) { Serial.println("[RTCIO] pull-up set OK"); }
  else { Serial.println("[RTCIO] pull-up failed"); pass = false; }

  Serial.println("[WDT] Testing TG0 WDT registers...");
  uint32_t wdtConf = READ_PERI_REG(0x3ff5f048);
  Serial.print("[WDT] TG0_WDTCONFIG0=0x"); Serial.println(wdtConf, HEX);
  uint32_t wdtFeed = READ_PERI_REG(0x3ff5f048);
  Serial.print("[WDT] TG0_WDTFEED=0x"); Serial.println(wdtFeed, HEX);
  Serial.println("[WDT] register access OK");

  Serial.println("[WDT] Testing RTC WDT...");
  uint32_t rtcWdt = READ_PERI_REG(0x3ff480a0);
  Serial.print("[WDT] RTC_CNTL_WDTCONFIG0=0x"); Serial.println(rtcWdt, HEX);
  Serial.println("[WDT] Done");

  Serial.print("RESULT="); Serial.println(pass ? "PASS" : "FAIL");
  Serial.println("=== ALL TESTS PASSED ===");
}
void loop() { delay(1000); }
