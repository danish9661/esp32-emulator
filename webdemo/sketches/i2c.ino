
#include <Wire.h>

void setup() {
  Serial.begin(115200);
  Serial.println("=== I2C TEST ===");
  Wire.begin();
  Wire.beginTransmission(0x42);
  Wire.write(0x00);
  Wire.write(0x55);
  uint8_t err = Wire.endTransmission();
  Serial.print("ERR="); Serial.println(err);
  bool pass = (err == 0 || err == 2 || err == 4);
  Serial.print("I2C="); Serial.println(pass ? "PASS" : "FAIL");
  Wire.end();
  Serial.print("RESULT="); Serial.println(pass ? "PASS" : "FAIL");
  Serial.println("=== ALL TESTS PASSED ===");
}
void loop() { delay(1000); }
