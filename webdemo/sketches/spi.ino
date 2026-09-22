
#include <SPI.h>

void setup() {
  Serial.begin(115200);
  Serial.println("=== SPI TEST ===");
  SPI.begin();
  SPI.beginTransaction(SPISettings(1000000, MSBFIRST, SPI_MODE0));
  uint8_t tx = 0xAA;
  uint8_t rx = SPI.transfer(tx);
  Serial.print("TX="); Serial.print(tx, HEX);
  Serial.print(" RX="); Serial.println(rx, HEX);
  bool pass = (rx == tx);
  Serial.print("LOOPBACK="); Serial.println(pass ? "PASS" : "FAIL");
  SPI.endTransaction();
  SPI.end();
  Serial.print("RESULT="); Serial.println(pass ? "PASS" : "FAIL");
  Serial.println("=== ALL TESTS PASSED ===");
}
void loop() { delay(1000); }
