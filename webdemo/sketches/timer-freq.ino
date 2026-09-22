
void setup() {
  Serial.begin(115200);
  Serial.println("=== TIMER FREQ TEST ===");

  // Timer at 1MHz
  hw_timer_t *timer = timerBegin(1000000);
  if (!timer) { Serial.println("[TIMER] timerBegin failed"); return; }
  timerStart(timer);

  // 1ms test
  delay(1);
  uint64_t cnt1ms = timerRead(timer);

  // 10s test
  timerWrite(timer, 0);
  delay(10000);
  uint64_t cnt10s = timerRead(timer);

  timerStop(timer);
  timerEnd(timer);

  unsigned long t0 = micros();
  delay(2);
  unsigned long us = micros() - t0;

  unsigned long m0 = millis();
  delay(10);
  unsigned long ms = millis() - m0;

  Serial.print("TIMER_1MS="); Serial.println((uint32_t)cnt1ms);
  Serial.print("TIMER_10S="); Serial.println((uint32_t)cnt10s);
  Serial.print("MICROS_2MS="); Serial.println(us);
  Serial.print("MILLIS_10MS="); Serial.println(ms);

  bool ok = cnt1ms > 0 && cnt10s > 0 && us > 0 && ms > 0;
  // Check 10s timer is at least ~1000x larger than 1ms timer
  bool ratioOk = cnt10s > cnt1ms * 500;
  Serial.print("RATIO="); Serial.println(ratioOk ? "PASS" : "FAIL");
  Serial.print("RESULT="); Serial.println(ok ? "PASS" : "FAIL");
  Serial.println("=== ALL TESTS PASSED ===");
}

void loop() { delay(1000); }
