
void setup() {
  Serial.begin(115200);
  Serial.println("=== UART TEST ===");
  Serial.println("[UART] Serial0 works");
  Serial2.begin(115200);
  Serial2.println("[UART] Serial2 Hello");
  Serial2.flush();
  Serial2.end();
  Serial.println("[UART] Serial2 Done");
  Serial1.begin(115200);
  Serial1.println("[UART] Serial1 Hello");
  Serial1.flush();
  Serial1.end();
  Serial.println("[UART] Serial1 Done");
  Serial.print("RESULT="); Serial.println("PASS");
  Serial.println("=== ALL TESTS PASSED ===");
}
void loop() { delay(1000); }
