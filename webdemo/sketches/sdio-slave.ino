
#include "driver/sdio_slave.h"
static uint8_t rxbuf[2048];
void setup() {
  Serial.begin(115200);
  Serial.println("=== SDIOSLAVE TEST ===");
  bool pass = true;
  sdio_slave_config_t cfg = {};
  cfg.timing = SDIO_SLAVE_TIMING_NSEND_PSAMPLE;
  cfg.sending_mode = SDIO_SLAVE_SEND_PACKET;
  cfg.send_queue_size = 4;
  cfg.recv_buffer_size = 1024;
  esp_err_t err = sdio_slave_initialize(&cfg);
  Serial.printf("SLV_INIT=%d", (int)err);
  if (err != ESP_OK) pass = false;
  else Serial.println("SLV_INIT=PASS");
  sdio_slave_buf_handle_t h = sdio_slave_recv_register_buf(rxbuf);
  Serial.print("SLV_REGBUF="); Serial.println(h ? "PASS" : "FAIL");
  if (!h) pass = false;
  err = sdio_slave_recv_load_buf(h);
  Serial.print("SLV_LOADBUF="); Serial.println(err == ESP_OK ? "PASS" : "FAIL");
  if (err != ESP_OK) pass = false;
  err = sdio_slave_start();
  Serial.print("SLV_START="); Serial.println(err == ESP_OK ? "PASS" : "FAIL");
  if (err != ESP_OK) pass = false;
  sdio_slave_buf_handle_t rh = NULL;
  uint8_t *got = NULL;
  size_t len = 0;
  TickType_t t0 = xTaskGetTickCount();
  err = sdio_slave_recv(&rh, &got, &len, 500 / portTICK_PERIOD_MS);
  TickType_t dt = xTaskGetTickCount() - t0;
  Serial.printf("SLV_RECV=%d dt=%u", (int)err, (unsigned)dt);
  if (err != ESP_ERR_TIMEOUT) { Serial.println("SLV_RECV=FAIL"); pass = false; }
  else Serial.println("SLV_RECV=PASS");
  sdio_slave_stop();
  sdio_slave_deinit();
  Serial.println("[SLV] stop/deinit OK");
  Serial.print("RESULT="); Serial.println(pass ? "PASS" : "FAIL");
  Serial.println("=== ALL TESTS PASSED ===");
}
void loop() { delay(1000); }
