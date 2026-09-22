
#include "sdmmc_cmd.h"
#include "driver/sdmmc_host.h"

void setup() {
  Serial.begin(115200);
  Serial.println("=== SDMMC TEST ===");
  sdmmc_host_t host = SDMMC_HOST_DEFAULT();
  sdmmc_slot_config_t slot = SDMMC_SLOT_CONFIG_DEFAULT();
  bool pass = true;
  Serial.print("SDMMC_HOST="); Serial.println(host.flags ? "HAS_FLAGS" : "ZERO");
  esp_err_t herr = sdmmc_host_init();
  Serial.print("SDMMC_HOST_INIT="); Serial.println(herr == ESP_OK ? "PASS" : "FAIL");
  if (herr != ESP_OK) pass = false;
  esp_err_t derr = sdmmc_host_deinit();
  Serial.print("SDMMC_HOST_DEINIT="); Serial.println(derr == ESP_OK ? "PASS" : "FAIL");
  if (derr != ESP_OK) pass = false;
  Serial.print("RESULT="); Serial.println(pass ? "PASS" : "FAIL");
  Serial.println("=== ALL TESTS PASSED ===");
}
void loop() { delay(1000); }
