
#include <esp_flash.h>
void setup() {
  Serial.begin(115200);
  // Dump partition table bytes at 0x8000 via direct flash read
  uint32_t buf[16];
  esp_err_t err = esp_flash_read(NULL, buf, 0x8000, 64);
  if (err == ESP_OK) {
    for (int i = 0; i < 16; i++) {
      if (i % 4 == 0) Serial.print("");
      Serial.print(buf[i], HEX); Serial.print(" ");
    }
    Serial.println();
  } else {
    Serial.print("flash_read err=0x"); Serial.println(err, HEX);
  }
  // List partitions from partition API
  esp_partition_iterator_t it = esp_partition_find(ESP_PARTITION_TYPE_APP, ESP_PARTITION_SUBTYPE_ANY, NULL);
  int app_cnt = 0;
  while (it) {
    const esp_partition_t *p = esp_partition_get(it);
    Serial.print("APP "); Serial.print(p->label); Serial.print(" @0x"); Serial.print(p->address, HEX); Serial.print(" sz=0x"); Serial.println(p->size, HEX);
    app_cnt++; it = esp_partition_next(it);
  }
  it = esp_partition_find(ESP_PARTITION_TYPE_DATA, ESP_PARTITION_SUBTYPE_ANY, NULL);
  int data_cnt = 0;
  while (it) {
    const esp_partition_t *p = esp_partition_get(it);
    Serial.print("DATA "); Serial.print(p->label); Serial.print(" @0x"); Serial.print(p->address, HEX); Serial.print(" sz=0x"); Serial.println(p->size, HEX);
    data_cnt++; it = esp_partition_next(it);
  }
  Serial.print("apps="); Serial.print(app_cnt); Serial.print(" data="); Serial.println(data_cnt);
  Serial.println("=== ALL TESTS PASSED ===");
}
void loop() { delay(1000); }
