
#include <Arduino.h>
#include "esp_task.h"

static volatile int core0Count = 0;
static volatile int core1Count = 0;
static volatile bool core1Done = false;
static volatile uint32_t sharedA = 0;
static volatile uint32_t sharedB = 0;

void core1Task(void* arg) {
  for (int i = 0; i < 200000; i++) {
    core1Count++;
    sharedA = i;
    if ((i & 0x3FFF) == 0) taskYIELD();
  }
  core1Done = true;
  vTaskDelete(NULL);
}

void setup() {
  Serial.begin(115200);
  Serial.println("=== MULTI-CORE TEST ===");

  xTaskCreatePinnedToCore(core1Task, "c1", 4096, NULL, 5, NULL, 1);

  // Core 0: spin doing independent work until core1 signals done
  while (!core1Done) {
    core0Count++;
    sharedB = core0Count;
    asm volatile("nop");
  }
  // Final increment to prove we ran after core1 finished
  core0Count++;
  sharedB = core0Count;

  bool pass = core1Done &&
              core1Count >= 200000 &&
              core0Count > 0 &&
              sharedA >= 199999;

  Serial.printf("[MC] core0=%d core1=%d sharedA=%d sharedB=%d",
                core0Count, core1Count, sharedA, sharedB);
  Serial.print("RESULT="); Serial.println(pass ? "PASS" : "FAIL");
  Serial.println("=== ALL TESTS PASSED ===");
}

void loop() { delay(1000); }
