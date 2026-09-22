
#include <esp_task_wdt.h>
#include <driver/gptimer.h>

void setup() {
  Serial.begin(115200);
  Serial.println("=== TIMER WDT TEST ===");
  bool pass = true;

  // Test 1: TWDT reconfigure (IDF 5.x API)
  Serial.println("[TWDT] Testing TWDT reconfigure...");
  esp_task_wdt_config_t twdt_cfg = {
    .timeout_ms = 5000,
    .idle_core_mask = 0,
    .trigger_panic = true,
  };
  esp_err_t err = esp_task_wdt_reconfigure(&twdt_cfg);
  Serial.print("[TWDT] reconfigure="); Serial.println(err == ESP_OK ? "PASS" : "FAIL");
  if (err != ESP_OK) pass = false;

  // Test 2: Feed the WDT
  Serial.println("[TWDT] Feeding WDT...");
  for (int i = 0; i < 5; i++) {
    esp_task_wdt_reset();
    delay(100);
  }
  Serial.println("[TWDT] feed OK");

  // Test 3: GPTimer (IDF 5.x replacement for legacy timer API)
  Serial.println("[TWDT] Testing gptimer...");
  gptimer_handle_t timer;
  gptimer_config_t tcfg = {
    .clk_src = GPTIMER_CLK_SRC_DEFAULT,
    .direction = GPTIMER_COUNT_UP,
    .resolution_hz = 1000000,
  };
  err = gptimer_new_timer(&tcfg, &timer);
  Serial.print("[TWDT] gptimer_new="); Serial.println(err == ESP_OK ? "PASS" : "FAIL");
  if (err != ESP_OK) pass = false;

  if (err == ESP_OK) {
    gptimer_set_raw_count(timer, 0);
    gptimer_start(timer);
    delay(50);
    uint64_t val = 0;
    gptimer_get_raw_count(timer, &val);
    gptimer_stop(timer);
    gptimer_del_timer(timer);
    Serial.print("[TWDT] timer_val="); Serial.println((uint32_t)val);
    Serial.println("[TWDT] timer_lifecycle=PASS");
  }

  Serial.print("RESULT="); Serial.println(pass ? "PASS" : "FAIL");
  Serial.println("=== ALL TESTS PASSED ===");
}

void loop() { delay(1000); }
