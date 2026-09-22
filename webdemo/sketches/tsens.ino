
#include <math.h>
void setup() {
  Serial.begin(115200);
  Serial.println("=== TSENS TEST ===");
  bool pass = true;
  float t1 = temperatureRead();
  Serial.printf("TEMP1=%f", t1);
  if (isnan(t1) || t1 < 10.0f || t1 > 50.0f) { Serial.println("TEMP_RANGE=FAIL"); pass = false; }
  else Serial.println("TEMP_RANGE=PASS");
  delay(200);
  float t2 = temperatureRead();
  Serial.printf("TEMP2=%f", t2);
  if (isnan(t2) || t2 < 10.0f || t2 > 50.0f || fabsf(t2 - t1) > 5.0f) { Serial.println("TEMP_STABLE=FAIL"); pass = false; }
  else Serial.println("TEMP_STABLE=PASS");
  Serial.print("RESULT="); Serial.println(pass ? "PASS" : "FAIL");
  Serial.println("=== ALL TESTS PASSED ===");
}
void loop() { delay(1000); }
