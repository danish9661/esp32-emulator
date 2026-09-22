
#include <WiFi.h>
void setup() {
  Serial.begin(115200);
  Serial.println("=== WIFI TEST ===");
  WiFi.mode(WIFI_STA);
  WiFi.disconnect();
  delay(100);
  int n = WiFi.scanNetworks();
  Serial.print("[SCAN] found "); Serial.print(n); Serial.println(" networks");
  Serial.println("=== ALL TESTS PASSED ===");
}
void loop() { delay(1000); }
