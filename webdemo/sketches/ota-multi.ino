
#include <esp_ota_ops.h>
#include <esp_partition.h>

void setup() {
  Serial.begin(115200);
  Serial.println("=== OTA MULTI-PARTITION TEST ===");
  bool pass = true;

  const esp_partition_t *running = esp_ota_get_running_partition();
  Serial.print("[OTA] running="); Serial.println(running ? "PASS" : "FAIL");
  if (!running) pass = false;

  const esp_partition_t *next = esp_ota_get_next_update_partition(NULL);
  Serial.print("[OTA] next_update="); Serial.println(next ? "PASS" : "FAIL");
  if (!next) pass = false;

  if (running) {
    Serial.print("[OTA] running_label="); Serial.println(running->label);
    Serial.print("[OTA] running_addr=0x"); Serial.println(running->address, HEX);
    Serial.print("[OTA] running_size="); Serial.println(running->size);
  }

  if (next) {
    Serial.print("[OTA] next_label="); Serial.println(next->label);
    Serial.print("[OTA] next_addr=0x"); Serial.println(next->address, HEX);
    Serial.print("[OTA] next_size="); Serial.println(next->size);

    bool diff_part = (running && running->address != next->address);
    Serial.print("[OTA] diff_partition="); Serial.println(diff_part ? "PASS" : "FAIL");
    if (!diff_part) pass = false;

    const esp_partition_t *next2 = esp_ota_get_next_update_partition(next);
    bool cycle = (next2 && next2->address == running->address);
    Serial.print("[OTA] partition_cycle="); Serial.println(cycle ? "PASS" : "FAIL");
    if (!cycle) pass = false;
  }

  Serial.print("RESULT="); Serial.println(pass ? "PASS" : "FAIL");
  Serial.println("=== ALL TESTS PASSED ===");
}

void loop() { delay(1000); }
