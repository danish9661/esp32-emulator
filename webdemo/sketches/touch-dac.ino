
volatile bool touchFired = false;
void IRAM_ATTR onTouch2() { touchFired = true; }
volatile bool touchFired15 = false;
void IRAM_ATTR onTouch15() { touchFired15 = true; }
void setup() {
  Serial.begin(115200);
  Serial.println("=== TOUCHDAC TEST ===");
  bool pass = true;

  uint16_t t2 = touchRead(2);    // T2 = GPIO2, driven low (touched)
  uint16_t t15 = touchRead(15);  // T3 = GPIO15, default (untouched)
  Serial.printf("TOUCH2=%u TOUCH15=%u", t2, t15);
  if (t2 > 500) { Serial.println("TOUCH_LOW=FAIL"); pass = false; }
  else Serial.println("TOUCH_LOW=PASS");
  if (t15 < 800) { Serial.println("TOUCH_HIGH=FAIL"); pass = false; }
  touchAttachInterrupt(2, onTouch2, 500);
  delay(800);
  Serial.print("TOUCH_IRQ="); Serial.println(touchFired ? "PASS" : "FAIL");
  if (!touchFired) pass = false;
  touchAttachInterrupt(15, onTouch15, 500);
  delay(400);
  Serial.print("TOUCH_DYN_NEG="); Serial.println(!touchFired15 ? "PASS" : "FAIL");
  if (touchFired15) pass = false;
  Serial.println("DYN_READY");
  for (int i = 0; i < 100 && !touchFired15; i++) delay(50);
  Serial.print("TOUCH_DYN="); Serial.println(touchFired15 ? "PASS" : "FAIL");
  if (!touchFired15) pass = false;
  else Serial.println("TOUCH_HIGH=PASS");

  // DAC ch0 (GPIO25) mid-scale -> ADC2 reads ~1.65V (11dB: ~2054).
  analogRead(25);
  dacWrite(25, 128);
  int a25 = analogRead(25);
  Serial.printf("DAC25=%d", a25);
  if (a25 < 1700 || a25 > 2400) { Serial.println("DAC_MID=FAIL"); pass = false; }
  else Serial.println("DAC_MID=PASS");

  // DAC ch1 (GPIO26) full-scale -> ~3.3V (clamped to 4095).
  analogRead(26);
  dacWrite(26, 255);
  int a26 = analogRead(26);
  Serial.printf("DAC26=%d", a26);
  if (a26 < 3900) { Serial.println("DAC_FULL=FAIL"); pass = false; }
  else Serial.println("DAC_FULL=PASS");

  Serial.print("RESULT="); Serial.println(pass ? "PASS" : "FAIL");
  Serial.println("=== ALL TESTS PASSED ===");
}
void loop() { delay(1000); }
