
void setup() {
  Serial.begin(115200);
  Serial.println("=== ANALOG TEST ===");
  analogReadResolution(12);
  int a = analogRead(36);
  int b = analogRead(32);
  int c = analogRead(33);
  int d0 = analogRead(34);
  analogSetPinAttenuation(34, ADC_6db);
  int d = analogRead(34);
  Serial.printf("[ANALOG] a=%d b=%d c=%d d0=%d d=%d", a, b, c, d0, d);
  bool ok = (a >= 1900 && a <= 2200) && (b < 100) && (c >= 3990) && (d0 >= 1900 && d0 <= 2200) && (d >= 3280 && d <= 3480);
  Serial.print("RESULT="); Serial.println(ok ? "PASS" : "FAIL");
  if (ok) Serial.println("=== ALL TESTS PASSED ===");
  else Serial.println("=== TEST FAILED ===");
}
void loop() { delay(1000); }
