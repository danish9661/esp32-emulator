
#include <esp_partition.h>
#include <esp_flash.h>
void setup() {
  Serial.begin(115200);
  Serial.println("=== FLASH TEST ===");
  bool pass = true;
  const esp_partition_t *part = esp_partition_find_first(ESP_PARTITION_TYPE_DATA, ESP_PARTITION_SUBTYPE_ANY, NULL);
  if (part) {
    Serial.print("[FLASH] partition="); Serial.println(part->label);
    Serial.print("[FLASH] addr=0x"); Serial.println(part->address, HEX);
    Serial.print("[FLASH] size=0x"); Serial.println(part->size, HEX);
  } else {
    Serial.println("[FLASH] no partition found");
    pass = false;
  }
  uint32_t buf[4] = {0};
  esp_err_t err = esp_flash_read(NULL, buf, 0, 16);
  Serial.print("[FLASH] read="); Serial.println(err == ESP_OK ? "PASS" : "FAIL");
  if (err != ESP_OK) { Serial.print(" err=0x"); Serial.println(err, HEX); pass = false; }
  else {
    Serial.print("[FLASH] data=");
    for (int i = 0; i < 4; i++) { Serial.print(buf[i], HEX); Serial.print(" "); }
    Serial.println();
  }
  // Test erase/write
  err = esp_flash_erase_region(NULL, 0x3F0000, 0x1000);
  Serial.print("[FLASH] erase="); Serial.println(err == ESP_OK ? "PASS" : "FAIL");
  if (err != ESP_OK) { Serial.print(" err=0x"); Serial.println(err, HEX); pass = false; }
  if (err == ESP_OK) {
    uint32_t wdata = 0xDEADBEEF;
    err = esp_flash_write(NULL, &wdata, 0x3F0000, 4);
    Serial.print("[FLASH] write="); Serial.println(err == ESP_OK ? "PASS" : "FAIL");
    if (err != ESP_OK) { Serial.print(" err=0x"); Serial.println(err, HEX); pass = false; }
    if (err == ESP_OK) {
      uint32_t rdata = 0;
      esp_flash_read(NULL, &rdata, 0x3F0000, 4);
      pass = pass && (rdata == 0xDEADBEEF);
      Serial.print("[FLASH] verify="); Serial.println(rdata == 0xDEADBEEF ? "PASS" : "FAIL");
      Serial.print(" readback=0x"); Serial.println(rdata, HEX);
    }
  }
  Serial.print("RESULT="); Serial.println(pass ? "PASS" : "FAIL");
  Serial.println("=== ALL TESTS PASSED ===");
}
void loop() { delay(1000); }
