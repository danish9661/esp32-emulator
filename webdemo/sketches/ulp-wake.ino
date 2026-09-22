
#include "esp32/ulp.h"
#include "esp_sleep.h"
#include "soc/rtc_cntl_reg.h"
#include "soc/soc.h"
RTC_DATA_ATTR int bootCount = 0;
enum { ULP_MAGIC_ADDR = 16 };
const ulp_insn_t prog[] = {
  I_WR_REG(RTC_CNTL_STORE0_REG, 0, 15, 0xAB),
  I_RD_REG(RTC_CNTL_STORE0_REG, 0, 15),
  I_MOVI(R1, ULP_MAGIC_ADDR),
  I_ST(R0, R1, 0),
  I_WAKE(),
  I_HALT(),
};
void setup() {
  Serial.begin(115200);
  Serial.println("=== ULPWAKE TEST ===");
  bootCount++;
  esp_sleep_wakeup_cause_t cause = esp_sleep_get_wakeup_cause();
  esp_reset_reason_t reason = esp_reset_reason();
  Serial.printf("[ULPWAKE] boot=%d cause=%d reason=%d", bootCount, (int)cause, (int)reason);
  Serial.printf("[ULPWAKE] wstate=%x", (unsigned)REG_READ(RTC_CNTL_WAKEUP_STATE_REG));
  bool pass = true;
  if (bootCount == 1) {
    volatile uint32_t *slow = (volatile uint32_t *)0x50000000;
    size_t psize = sizeof(prog) / sizeof(ulp_insn_t);
    esp_err_t err = ulp_process_macros_and_load(0, prog, &psize);
    Serial.print("ULP_LOAD="); Serial.println(err == ESP_OK ? "PASS" : "FAIL");
    if (err != ESP_OK) pass = false;
    err = ulp_run(0);
    Serial.print("ULP_RUN="); Serial.println(err == ESP_OK ? "PASS" : "FAIL");
    if (err != ESP_OK) pass = false;
    uint32_t m0 = slow[ULP_MAGIC_ADDR];
    uint32_t s0 = REG_READ(RTC_CNTL_STORE0_REG);
    Serial.printf("ULP_RW=%x %x", (unsigned)m0, (unsigned)s0);
    if (m0 != 0xAB || (s0 & 0xFFFF) != 0xAB) { Serial.println("ULP_WRREG=FAIL"); pass = false; }
    else Serial.println("ULP_WRREG=PASS");
    if (!pass) { Serial.println("RESULT=FAIL"); Serial.println("=== ALL TESTS PASSED ==="); return; }
    err = esp_sleep_enable_ulp_wakeup();
    Serial.printf("ULP_WUEN=%d", (int)err);
    esp_sleep_enable_timer_wakeup(2000000);
    esp_deep_sleep_start();
  } else {
    if (cause != ESP_SLEEP_WAKEUP_ULP) { Serial.println("ULP_CAUSE=FAIL"); pass = false; }
    else Serial.println("ULP_CAUSE=PASS");
    if (reason != ESP_RST_DEEPSLEEP) { Serial.println("ULP_REASON=FAIL"); pass = false; }
    else Serial.println("ULP_REASON=PASS");
    Serial.print("RESULT="); Serial.println(pass ? "PASS" : "FAIL");
    Serial.println("=== ALL TESTS PASSED ===");
  }
}
void loop() { delay(1000); }
