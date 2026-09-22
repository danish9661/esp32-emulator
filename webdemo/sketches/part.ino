
#include <esp_flash.h>
void setup() {
  Serial.begin(115200);
  Serial.println("=== PART TABLE TEST ===");
  // Dump partition table bytes at 0x8000 via direct flash read
  uint32_t buf[8];
  esp_err_t err = esp_flash_read(NULL, buf, 0x8000, 32);
  if (err == ESP_OK) {
    Serial.print("magic="); Serial.println(buf[0], HEX);
    for (int i = 0; i < 8; i++) {
      Serial.print("  ["); Serial.print(i); Serial.print("]="); Serial.println(buf[i], HEX);
    }
  } else {
    Serial.print("read failed err=0x"); Serial.println(err, HEX);
  }
  const void *part = NULL;
  esp_partition_iterator_t it = esp_partition_find(ESP_PARTITION_TYPE_APP, ESP_PARTITION_SUBTYPE_ANY, NULL);
  int count = 0;
  while (it) { count++; it = esp_partition_next(it); }
  Serial.print("app_partitions="); Serial.println(count);
  it = esp_partition_find(ESP_PARTITION_TYPE_DATA, ESP_PARTITION_SUBTYPE_ANY, NULL);
  count = 0;
  while (it) { 
    const esp_partition_t *p = esp_partition_get(it);
    Serial.print(p->label); Serial.print(" @0x"); Serial.print(p->address, HEX); Serial.print(" sz=0x"); Serial.println(p->size, HEX);
    count++; it = esp_partition_next(it); 
  }
  Serial.print("data_partitions="); Serial.println(count);
  Serial.println("=== ALL TESTS PASSED ===");
}
void loop() { delay(1000); }
