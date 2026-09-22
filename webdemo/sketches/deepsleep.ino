
#include "esp_sleep.h"
#include <string.h>
RTC_DATA_ATTR int bootCount = 0;
RTC_DATA_ATTR char rtcData[16] = "rtc-boot-marker";

void setup() {
  Serial.begin(115200);
  Serial.println("=== DEEP SLEEP TEST ===");
  bootCount++;
  esp_sleep_wakeup_cause_t cause = esp_sleep_get_wakeup_cause();
  esp_reset_reason_t reason = esp_reset_reason();
  Serial.printf("[SLEEP] boot=%d cause=%d reason=%d rtc=%s", bootCount, cause, reason, rtcData);
  esp_sleep_enable_timer_wakeup(1000000);
  if (bootCount >= 3) {
    bool pass = (cause == ESP_SLEEP_WAKEUP_TIMER) &&
                (reason == ESP_RST_DEEPSLEEP) &&
                (strcmp(rtcData, "rtc-boot-marker") == 0) &&
                (bootCount == 3);
    Serial.print("RESULT="); Serial.println(pass ? "PASS" : "FAIL");
    Serial.println("=== ALL TESTS PASSED ===");
  } else {
    esp_deep_sleep_start();
  }
}
void loop() { delay(1000); }
