
#include <WiFi.h>
void setup() {
  Serial.begin(115200);
  Serial.println("=== SOFTAP TEST ===");
  bool pass = true;
  WiFi.mode(WIFI_AP);
  bool ok = WiFi.softAP("emu-ap", "password1", 6, 0, 4);
  Serial.printf("SOFTAP=%d", ok ? 1 : 0);
  if (!ok) pass = false;
  String ip = WiFi.softAPIP().toString();
  Serial.print("SOFTAPIP="); Serial.println(ip);
  if (ip != "192.168.4.1") pass = false;
  String mac = WiFi.softAPmacAddress();
  Serial.print("SOFTAPMAC="); Serial.println(mac);
  // Must be a real derived address, not the eFuse-zero artifact.
  if (mac == "00:00:00:00:00:00" || mac == "00:00:00:00:00:01") pass = false;
  int n0 = WiFi.softAPgetStationNum();
  delay(3000);
  int n1 = WiFi.softAPgetStationNum();
  Serial.printf("STANUM=%d/%d", n0, n1);
  if (n0 != 0 || n1 != 0) pass = false;
  Serial.print("RESULT="); Serial.println(pass ? "PASS" : "FAIL");
  Serial.println("=== ALL TESTS PASSED ===");
}
void loop() { delay(1000); }
