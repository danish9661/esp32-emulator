
#include <driver/i2s.h>
void setup() {
  Serial.begin(115200);
  Serial.println("=== I2SRX TEST ===");
  bool pass = true;
  i2s_config_t cfg = {};
  cfg.mode = (i2s_mode_t)(I2S_MODE_MASTER | I2S_MODE_RX);
  cfg.sample_rate = 16000;
  cfg.bits_per_sample = I2S_BITS_PER_SAMPLE_16BIT;
  cfg.channel_format = I2S_CHANNEL_FMT_RIGHT_LEFT;
  cfg.communication_format = I2S_COMM_FORMAT_STAND_I2S;
  cfg.intr_alloc_flags = ESP_INTR_FLAG_LEVEL1;
  cfg.dma_buf_count = 4;
  cfg.dma_buf_len = 64;
  esp_err_t err = i2s_driver_install(I2S_NUM_0, &cfg, 0, NULL);
  Serial.print("RX_INSTALL="); Serial.println(err == ESP_OK ? "PASS" : "FAIL");
  if (err != ESP_OK) pass = false;
  uint8_t buf[256];
  memset(buf, 0, sizeof(buf));
  size_t got = 0;
  err = i2s_read(I2S_NUM_0, buf, sizeof(buf), &got, 3000 / portTICK_PERIOD_MS);
  Serial.printf("RX_READ=%d GOT=%u", (int)err, (unsigned)got);
  if (err != ESP_OK || got != sizeof(buf)) { Serial.println("RX_READ=FAIL"); pass = false; }
  else Serial.println("RX_READ=PASS");
  uint32_t bad = 0;
  for (int i = 0; i < 64; i++) {
    uint32_t v = buf[4*i] | ((uint32_t)buf[4*i+1] << 8) | ((uint32_t)buf[4*i+2] << 16) | ((uint32_t)buf[4*i+3] << 24);
    if (v != (uint32_t)(0x1000 + i)) bad++;
  }
  Serial.printf("RX_BAD=%u", (unsigned)bad);
  if (bad != 0) { Serial.println("RX_DATA=FAIL"); pass = false; }
  else Serial.println("RX_DATA=PASS");
  i2s_driver_uninstall(I2S_NUM_0);
  Serial.println("[I2S] uninstall OK");
  Serial.print("RESULT="); Serial.println(pass ? "PASS" : "FAIL");
  Serial.println("=== ALL TESTS PASSED ===");
}
void loop() { delay(1000); }
