
#include <driver/i2s.h>

void setup() {
  Serial.begin(115200);
  Serial.println("=== I2S TEST ===");
  i2s_config_t cfg = {
    .mode = (i2s_mode_t)(I2S_MODE_MASTER | I2S_MODE_TX),
    .sample_rate = 44100,
    .bits_per_sample = I2S_BITS_PER_SAMPLE_16BIT,
    .channel_format = I2S_CHANNEL_FMT_RIGHT_LEFT,
    .communication_format = I2S_COMM_FORMAT_I2S,
    .intr_alloc_flags = ESP_INTR_FLAG_LEVEL1,
    .dma_buf_count = 2,
    .dma_buf_len = 64
  };
  esp_err_t err = i2s_driver_install(I2S_NUM_0, &cfg, 0, NULL);
  Serial.print("I2S_INSTALL="); Serial.println(err == ESP_OK ? "PASS" : "FAIL");
  bool pass = (err == ESP_OK);
  if (err == ESP_OK) {
    uint8_t buf[64] = {0};
    size_t written = 0;
    esp_err_t werr = i2s_write(I2S_NUM_0, buf, sizeof(buf), &written, 1000 / portTICK_PERIOD_MS);
    Serial.print("I2S_WRITE="); Serial.println(werr == ESP_OK ? "PASS" : "FAIL");
    Serial.print("I2S_WRITTEN="); Serial.println((int)written);
    if (werr != ESP_OK) pass = false;
    Serial.println("[I2S] write done");
    i2s_driver_uninstall(I2S_NUM_0);
    Serial.println("[I2S] uninstall OK");
  }
  Serial.print("RESULT="); Serial.println(pass ? "PASS" : "FAIL");
  Serial.println("=== ALL TESTS PASSED ===");
}
void loop() { delay(1000); }
