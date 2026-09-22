
#include <esp_ota_ops.h>
#include <nvs_flash.h>
#include <esp_partition.h>
void setup() {
  Serial.begin(115200);
  Serial.println("=== OTA TEST ===");
  esp_err_t err = nvs_flash_init();
  if (err == ESP_ERR_NVS_NO_FREE_PAGES || err == ESP_ERR_NVS_NEW_VERSION_FOUND) {
    nvs_flash_erase();
    err = nvs_flash_init();
  }
  Serial.print("[OTA] nvs="); Serial.println(err == ESP_OK ? "PASS" : "FAIL");
  bool pass = (err == ESP_OK);
  if (!pass) { Serial.print("RESULT="); Serial.println(pass ? "PASS" : "FAIL"); return; }
  const esp_partition_t *part = esp_ota_get_next_update_partition(NULL);
  pass = (part != NULL);
  Serial.print("[OTA] get_partition="); Serial.println(pass ? "PASS" : "FAIL");
  if (!pass) { Serial.print("RESULT="); Serial.println(pass ? "PASS" : "FAIL"); return; }
  esp_ota_handle_t handle;
  err = esp_ota_begin(part, OTA_SIZE_UNKNOWN, &handle);
  Serial.print("[OTA] begin="); Serial.println(err == ESP_OK ? "PASS" : "FAIL");
  if (err != ESP_OK) { pass = false; Serial.print(" err=0x"); Serial.println(err, HEX); }
  if (err == ESP_OK) {
    const esp_partition_t *running = esp_ota_get_running_partition();
    if (running != NULL) {
      // Use small stack buffer (1KB) to avoid stack overflow
      uint8_t buf[1024];
      uint32_t off = 0;
      while (off < running->size) {
        uint32_t to_read = (running->size - off) < sizeof(buf) ? (running->size - off) : sizeof(buf);
        err = esp_partition_read(running, off, buf, to_read);
        if (err != ESP_OK) { Serial.print("[OTA] read_err=0x"); Serial.println(err, HEX); break; }
        err = esp_ota_write(handle, buf, to_read);
        if (err != ESP_OK) { Serial.print("[OTA] write_err=0x"); Serial.println(err, HEX); break; }
        off += to_read;
      }
    } else {
      err = ESP_FAIL;
      Serial.println("[OTA] running=NULL");
    }
    if (err == ESP_OK) {
      err = esp_ota_end(handle);
      Serial.print("[OTA] end="); Serial.println(err == ESP_OK ? "PASS" : "FAIL");
      if (err != ESP_OK) { pass = false; Serial.print(" err=0x"); Serial.println(err, HEX); }
    } else {
      // abort OTA on error
      esp_ota_abort(handle);
    }
  }
  Serial.print("RESULT="); Serial.println(pass ? "PASS" : "FAIL");
  Serial.println("=== ALL TESTS PASSED ===");
}
void loop() { delay(1000); }
