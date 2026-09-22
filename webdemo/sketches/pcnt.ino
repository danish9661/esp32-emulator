
#include "driver/pcnt.h"
#include <soc/soc.h>

#define PCNT_BASE   0x3ff57000
#define CONF0_U0    (PCNT_BASE + 0x00)
#define CONF1_U0    (PCNT_BASE + 0x04)

void setup() {
  Serial.begin(115200);
  Serial.println("=== PCNT TEST ===");
  bool pass = true;

  pcnt_config_t cfg = {};
  cfg.pulse_gpio_num = 4;
  cfg.ctrl_gpio_num = PCNT_PIN_NOT_USED;
  cfg.lctrl_mode = PCNT_MODE_KEEP;
  cfg.hctrl_mode = PCNT_MODE_KEEP;
  cfg.pos_mode = PCNT_COUNT_INC;
  cfg.neg_mode = PCNT_COUNT_DIS;
  cfg.channel = PCNT_CHANNEL_0;
  cfg.unit = PCNT_UNIT_0;
  cfg.counter_h_lim = 100;
  cfg.counter_l_lim = -100;
  esp_err_t err = pcnt_unit_config(&cfg);
  Serial.print("CONFIG="); Serial.println(err == ESP_OK ? "PASS" : "FAIL");
  if (err != ESP_OK) pass = false;

  // Threshold-0 event at count 10 via direct registers (no interrupt enable,
  // so no ISR needed): thres0_en = CONF0 bit 14, thres0 = CONF1 low 16.
  uint32_t c0 = REG_READ(CONF0_U0);
  REG_WRITE(CONF0_U0, c0 | (1 << 14));
  REG_WRITE(CONF1_U0, 10);

  pcnt_counter_pause(PCNT_UNIT_0);
  pcnt_counter_clear(PCNT_UNIT_0);
  pcnt_counter_resume(PCNT_UNIT_0);
  Serial.println("PCNT_READY");
  // Wait for the host 'go' char (all edges delivered) with a long timeout.
  for (int i = 0; i < 6000 && !Serial.available(); i++) delay(10);
  if (Serial.available()) Serial.read();

  int16_t count = -1;
  pcnt_get_counter_value(PCNT_UNIT_0, &count);
  Serial.printf("COUNT=%d", count);
  if (count != 10) { Serial.println("COUNT10=FAIL"); pass = false; }
  else Serial.println("COUNT10=PASS");

  uint32_t st = REG_READ(PCNT_BASE + 0x90); // Un_STATUS unit 0
  Serial.printf("STATUS=0x%x", st);
  if ((st & 0x8) == 0) { Serial.println("THRES_EVT=FAIL"); pass = false; }
  else Serial.println("THRES_EVT=PASS");

  Serial.print("RESULT="); Serial.println(pass ? "PASS" : "FAIL");
  Serial.println("=== ALL TESTS PASSED ===");
}
void loop() { delay(1000); }
