
#include <driver/mcpwm.h>
#include <soc/soc.h>

static volatile uint32_t capVal = 0, capEdge = 0, capCount = 0;
static bool IRAM_ATTR capCb(mcpwm_unit_t m, mcpwm_capture_channel_id_t ch, const cap_event_data_t *e, void *u) {
  capVal = e->cap_value; capEdge = e->cap_edge; capCount++; return false;
}
void setup() {
  Serial.begin(115200);
  Serial.println("=== MCPWM TEST ===");
  bool pass = true;

  esp_err_t err = mcpwm_gpio_init(MCPWM_UNIT_0, MCPWM0A, 2);
  Serial.printf("[MCPWM] gpio_init err=%d", err);
  if (err != ESP_OK) pass = false;

  mcpwm_config_t cfg = {
    .frequency = 1000,
    .cmpr_a = 50.0,
    .cmpr_b = 50.0,
    .duty_mode = MCPWM_DUTY_MODE_0,
    .counter_mode = MCPWM_UP_COUNTER,
  };
  err = mcpwm_init(MCPWM_UNIT_0, MCPWM_TIMER_0, &cfg);
  Serial.printf("[MCPWM] init err=%d", err);
  if (err != ESP_OK) pass = false;

  err = mcpwm_set_duty(MCPWM_UNIT_0, MCPWM_TIMER_0, MCPWM_OPR_A, 25.0);
  Serial.printf("[MCPWM] set_duty A err=%d", err);
  if (err != ESP_OK) pass = false;
  err = mcpwm_set_duty(MCPWM_UNIT_0, MCPWM_TIMER_0, MCPWM_OPR_B, 75.0);
  if (err != ESP_OK) pass = false;

  mcpwm_start(MCPWM_UNIT_0, MCPWM_TIMER_0);
  Serial.println("[MCPWM] start OK");
  mcpwm_stop(MCPWM_UNIT_0, MCPWM_TIMER_0);
  Serial.println("[MCPWM] stop OK");

  float duty = mcpwm_get_duty(MCPWM_UNIT_0, MCPWM_TIMER_0, MCPWM_OPR_A);
  Serial.printf("[MCPWM] duty=%f", duty);
  if (duty < 24.0f || duty > 26.0f) pass = false;

  err = mcpwm_set_frequency(MCPWM_UNIT_0, MCPWM_TIMER_0, 2000);
  Serial.printf("[MCPWM] set_frequency err=%d", err);
  if (err != ESP_OK) pass = false;

  // Unit 1 (0x3FF6C000)
  err = mcpwm_init(MCPWM_UNIT_1, MCPWM_TIMER_0, &cfg);
  Serial.printf("[MCPWM] init(1) err=%d", err);
  if (err != ESP_OK) pass = false;

  err = mcpwm_set_duty(MCPWM_UNIT_1, MCPWM_TIMER_0, MCPWM_OPR_A, 33.0);
  Serial.printf("[MCPWM] set_duty A(1) err=%d", err);
  if (err != ESP_OK) pass = false;

  mcpwm_start(MCPWM_UNIT_1, MCPWM_TIMER_0);
  Serial.println("[MCPWM] start(1) OK");
  mcpwm_stop(MCPWM_UNIT_1, MCPWM_TIMER_0);

  float duty1 = mcpwm_get_duty(MCPWM_UNIT_1, MCPWM_TIMER_0, MCPWM_OPR_A);
  Serial.printf("[MCPWM] duty(1)=%f", duty1);
  if (duty1 < 32.0f || duty1 > 34.0f) pass = false;

  // ---- capture UNIT_0 CAP0 on gpio 4 (host-driven edges) ----
  err = mcpwm_gpio_init(MCPWM_UNIT_0, MCPWM_CAP_0, 4);
  Serial.print("CAP_GPIO="); Serial.println(err == ESP_OK ? "PASS" : "FAIL");
  if (err != ESP_OK) pass = false;
  mcpwm_capture_config_t ccfg = {};
  ccfg.cap_edge = MCPWM_POS_EDGE;
  ccfg.cap_prescale = 1;
  ccfg.capture_cb = capCb;
  ccfg.user_data = NULL;
  err = mcpwm_capture_enable_channel(MCPWM_UNIT_0, MCPWM_SELECT_CAP0, &ccfg);
  Serial.print("CAP_EN="); Serial.println(err == ESP_OK ? "PASS" : "FAIL");
  if (err != ESP_OK) pass = false;
  Serial.println("CAP_READY");
  for (int i = 0; i < 6000 && !Serial.available(); i++) delay(10);
  if (Serial.available()) Serial.read();
  uint32_t v1 = mcpwm_capture_signal_get_value(MCPWM_UNIT_0, MCPWM_SELECT_CAP0);
  uint32_t e1 = mcpwm_capture_signal_get_edge(MCPWM_UNIT_0, MCPWM_SELECT_CAP0);
  Serial.printf("CAP1 val=%u edge=%u", (unsigned)v1, (unsigned)e1);
  Serial.println("CAP_READY2");
  for (int i = 0; i < 6000 && !Serial.available(); i++) delay(10);
  if (Serial.available()) Serial.read();
  uint32_t v2 = mcpwm_capture_signal_get_value(MCPWM_UNIT_0, MCPWM_SELECT_CAP0);
  uint32_t e2 = mcpwm_capture_signal_get_edge(MCPWM_UNIT_0, MCPWM_SELECT_CAP0);
  Serial.printf("CAP2 val=%u edge=%u cb=%u", (unsigned)v2, (unsigned)e2, (unsigned)capCount);
  if (!(v2 > v1 && e1 == 1 && e2 == 1 && capCount >= 1)) { Serial.println("CAPTURE=FAIL"); pass = false; }
  else Serial.println("CAPTURE=PASS");
  // ---- fault F0 on gpio 5 (host drives high) ----
  err = mcpwm_gpio_init(MCPWM_UNIT_0, MCPWM_FAULT_0, 5);
  Serial.print("FAULT_GPIO="); Serial.println(err == ESP_OK ? "PASS" : "FAIL");
  if (err != ESP_OK) pass = false;
  err = mcpwm_fault_init(MCPWM_UNIT_0, MCPWM_HIGH_LEVEL_TGR, MCPWM_SELECT_F0);
  Serial.print("FAULT_INIT="); Serial.println(err == ESP_OK ? "PASS" : "FAIL");
  if (err != ESP_OK) pass = false;
  Serial.println("FAULT_READY");
  for (int i = 0; i < 6000 && !Serial.available(); i++) delay(10);
  if (Serial.available()) Serial.read();
  uint32_t fdet = REG_READ(0x3FF5E000 + 0xE4);
  Serial.printf("FAULTDET=0x%x", (unsigned)fdet);
  if ((fdet & 0x40) == 0) { Serial.println("FAULT=FAIL"); pass = false; }
  else Serial.println("FAULT=PASS");
  Serial.print("RESULT="); Serial.println(pass ? "PASS" : "FAIL");
  Serial.println("=== ALL TESTS PASSED ===");
}
void loop() { delay(1000); }
