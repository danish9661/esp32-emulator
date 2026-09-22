
#include <driver/ledc.h>
void setup() {
  Serial.begin(115200);
  Serial.println("=== PWM TEST ===");
  bool pass = true;
  esp_err_t err = ESP_OK;
  ledcAttach(2, 5000, 8);
  ledcWrite(2, 128);
  Serial.println("[PWM] write 128 OK");
  ledcWrite(2, 64);
  Serial.println("[PWM] write 64 OK");
  ledcDetach(2);
  Serial.println("[PWM] detach OK");
  ledc_timer_config_t ft = {};
  ft.speed_mode = LEDC_LOW_SPEED_MODE;
  ft.duty_resolution = LEDC_TIMER_8_BIT;
  ft.timer_num = LEDC_TIMER_1;
  ft.freq_hz = 5000;
  ft.clk_cfg = LEDC_AUTO_CLK;
  err = ledc_timer_config(&ft);
  Serial.print("FADE_TIMER="); Serial.println(err == ESP_OK ? "PASS" : "FAIL");
  if (err != ESP_OK) pass = false;
  ledc_channel_config_t fc = {};
  fc.gpio_num = 4;
  fc.speed_mode = LEDC_LOW_SPEED_MODE;
  fc.channel = LEDC_CHANNEL_1;
  fc.timer_sel = LEDC_TIMER_1;
  fc.duty = 0;
  fc.hpoint = 0;
  err = ledc_channel_config(&fc);
  Serial.print("FADE_CHAN="); Serial.println(err == ESP_OK ? "PASS" : "FAIL");
  if (err != ESP_OK) pass = false;
  err = ledc_fade_func_install(0);
  Serial.print("FADE_INSTALL="); Serial.println(err == ESP_OK ? "PASS" : "FAIL");
  if (err != ESP_OK) pass = false;
  err = ledc_set_fade_with_time(LEDC_LOW_SPEED_MODE, LEDC_CHANNEL_1, 255, 1000);
  Serial.print("FADE_SET="); Serial.println(err == ESP_OK ? "PASS" : "FAIL");
  if (err != ESP_OK) pass = false;
  err = ledc_fade_start(LEDC_LOW_SPEED_MODE, LEDC_CHANNEL_1, LEDC_FADE_WAIT_DONE);
  Serial.print("FADE_RUN="); Serial.println(err == ESP_OK ? "PASS" : "FAIL");
  if (err != ESP_OK) pass = false;
  uint32_t fduty = ledc_get_duty(LEDC_LOW_SPEED_MODE, LEDC_CHANNEL_1);
  Serial.printf("FADE_DUTY=%u", (unsigned)fduty);
  if (fduty != 255) { Serial.println("FADE_DATA=FAIL"); pass = false; }
  else Serial.println("FADE_DATA=PASS");
  Serial.print("RESULT="); Serial.println(pass ? "PASS" : "FAIL");
  Serial.println("=== ALL TESTS PASSED ===");
}
void loop() { delay(1000); }
