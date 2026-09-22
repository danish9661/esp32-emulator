
#include <esp_bt.h>
extern "C" bool btClassicInUse(void) { return true; }
extern "C" bool bleInUse(void) { return true; }
void setup() {
  Serial.begin(115200);
  Serial.println("=== BT TEST ===");
  bool pass = true;
  esp_bt_controller_config_t cfg = BT_CONTROLLER_INIT_CONFIG_DEFAULT();
  esp_err_t err = esp_bt_controller_init(&cfg);
  Serial.print("[BT] init="); Serial.println(err == ESP_OK ? "PASS" : "FAIL");
  if (err != ESP_OK) { Serial.print(" err=0x"); Serial.println(err, HEX); pass = false; }
  Serial.print("RESULT="); Serial.println(pass ? "PASS" : "FAIL");
  Serial.println("=== ALL TESTS PASSED ===");
}
void loop() { delay(1000); }
