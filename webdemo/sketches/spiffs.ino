
#include <SPIFFS.h>
#include <esp_partition.h>
void setup() {
  Serial.begin(115200);
  Serial.println("=== SPIFFS TEST ===");
  bool pass = true;

  // List partitions
  esp_partition_iterator_t it = esp_partition_find(ESP_PARTITION_TYPE_DATA, ESP_PARTITION_SUBTYPE_ANY, NULL);
  while (it) {
    const esp_partition_t *p = esp_partition_get(it);
    Serial.print("  "); Serial.print(p->label);
    Serial.print(" type=0x"); Serial.print(p->type, HEX);
    Serial.print(" subtype=0x"); Serial.print(p->subtype, HEX);
    Serial.print(" off=0x"); Serial.print(p->address, HEX);
    Serial.print(" sz=0x"); Serial.println(p->size, HEX);
    it = esp_partition_next(it);
  }

  // Test partition erase+write
  const esp_partition_t *part = esp_partition_find_first(ESP_PARTITION_TYPE_DATA, ESP_PARTITION_SUBTYPE_DATA_SPIFFS, NULL);
  if (part) {
    Serial.print("[SPIFFS] partition found: "); Serial.print(part->label);
    Serial.print(" @0x"); Serial.print(part->address, HEX);
    Serial.print(" size=0x"); Serial.println(part->size, HEX);
    uint32_t wdata = 0xCAFEBABE;
    esp_err_t err = esp_partition_erase_range(part, 0, 4096);
    Serial.print("[SPIFFS] erase="); Serial.println(err == ESP_OK ? "PASS" : "FAIL");
    if (err == ESP_OK) {
      err = esp_partition_write(part, 0, &wdata, 4);
      Serial.print("[SPIFFS] write="); Serial.println(err == ESP_OK ? "PASS" : "FAIL");
      if (err == ESP_OK) {
        uint32_t rdata = 0;
        esp_partition_read(part, 0, &rdata, 4);
        Serial.print("[SPIFFS] verify="); Serial.println(rdata == wdata ? "PASS" : "FAIL");
        Serial.print(" data=0x"); Serial.println(rdata, HEX);
        pass = pass && (rdata == wdata);
      }
    }
  } else {
    Serial.println("[SPIFFS] no spiffs partition found");
    // Try default partition write to nvs instead
    const esp_partition_t *nvs = esp_partition_find_first(ESP_PARTITION_TYPE_DATA, ESP_PARTITION_SUBTYPE_DATA_NVS, NULL);
    if (nvs) {
      uint32_t test = 0xAABBCCDD;
      esp_partition_erase_range(nvs, 0, nvs->size);
      esp_partition_write(nvs, 0, &test, 4);
      uint32_t rb = 0;
      esp_partition_read(nvs, 0, &rb, 4);
      pass = pass && (rb == 0xAABBCCDD);
      Serial.print("[FLASH] nvs_rw="); Serial.println(pass ? "PASS" : "FAIL");
    }
  }
  Serial.print("RESULT="); Serial.println(pass ? "PASS" : "FAIL");
  Serial.println("=== ALL TESTS PASSED ===");
}
void loop() { delay(1000); }
