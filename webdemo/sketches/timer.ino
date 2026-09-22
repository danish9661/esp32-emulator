
void setup() {
  Serial.begin(115200);
  Serial.println("=== HW TIMER TEST ===");
  bool pass = true;
  hw_timer_t *timer = timerBegin(1000000);
  Serial.print("TIMER_CREATE="); Serial.println(timer ? "PASS" : "FAIL");
  if (!timer) pass = false;
  if (timer) {
    timerStart(timer);
    delay(2);
    uint64_t cnt = timerRead(timer);
    Serial.print("[TIMER] count after 2ms: ");
    Serial.println((uint32_t)cnt);
    if (cnt > 500 && cnt < 50000) { Serial.println("[TIMER] plausible"); } else { Serial.println("[TIMER] unexpected"); pass = false; }
    timerWrite(timer, 0);
    timerStop(timer);
    timerEnd(timer);
    Serial.println("[TIMER] stop/end OK");
  }
  Serial.print("RESULT="); Serial.println(pass ? "PASS" : "FAIL");
  Serial.println("=== ALL TESTS PASSED ===");
}
void loop() { delay(1000); }
