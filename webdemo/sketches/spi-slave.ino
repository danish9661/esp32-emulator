
#include <SPI.h>
#include <driver/spi_slave.h>
#include <string.h>

volatile bool cbRan = false;
void IRAM_ATTR trans_cb(spi_slave_transaction_t *t) { cbRan = true; }

void setup() {
  Serial.begin(115200);
  Serial.println("=== SPISLAVE TEST ===");
  bool pass = true;

  spi_bus_config_t buscfg = {};
  buscfg.mosi_io_num = 13;
  buscfg.miso_io_num = 12;
  buscfg.sclk_io_num = 14;
  buscfg.quadwp_io_num = -1;
  buscfg.quadhd_io_num = -1;
  spi_slave_interface_config_t slvcfg = {};
  slvcfg.spics_io_num = 15;
  slvcfg.queue_size = 2;
  slvcfg.mode = 0;
  slvcfg.post_trans_cb = trans_cb;
  esp_err_t ierr = spi_slave_initialize(HSPI_HOST, &buscfg, &slvcfg, 0);
  Serial.printf("SLAVE_INIT=%d", (int)ierr);
  if (ierr != ESP_OK) pass = false;

  SPI.begin();

  // Transaction 1.
  static uint8_t tx1[4] = {'W','X','Y','Z'};
  static uint8_t rx1[4] = {0};
  static uint8_t mosi1[4] = {'a','b','c','d'};
  static uint8_t miso1[4] = {0};
  spi_slave_transaction_t t1 = {};
  t1.length = 32; t1.tx_buffer = tx1; t1.rx_buffer = rx1;
  spi_slave_queue_trans(HSPI_HOST, &t1, portMAX_DELAY);
  SPI.beginTransaction(SPISettings(1000000, MSBFIRST, SPI_MODE0));
  SPI.transferBytes(mosi1, miso1, 4);
  SPI.endTransaction();
  spi_slave_transaction_t *ret = NULL;
  esp_err_t g1 = spi_slave_get_trans_result(HSPI_HOST, &ret, 5000 / portTICK_PERIOD_MS);
  Serial.printf("GET1=%d CB=%d LEN1=%lu", (int)g1, (int)cbRan, ret ? (unsigned long)ret->trans_len : 999UL);
  uint32_t sw0 = REG_READ(0x3FF64000 + 0x80);
  Serial.printf("SLAVEW0=0x%x", sw0);
  Serial.printf("RX1=%c%c%c%c MISO1=%c%c%c%c", rx1[0], rx1[1], rx1[2], rx1[3], miso1[0], miso1[1], miso1[2], miso1[3]);
  if (g1 != ESP_OK || memcmp(rx1, "abcd", 4) != 0) { Serial.println("SLV_RX=FAIL"); pass = false; }
  else Serial.println("SLV_RX=PASS");
  if (memcmp(miso1, "WXYZ", 4) != 0) { Serial.println("MST_RX=FAIL"); pass = false; }
  else Serial.println("MST_RX=PASS");
  if (!cbRan) { Serial.println("POSTCB=FAIL"); pass = false; }
  else Serial.println("POSTCB=PASS");

  // Transaction 2 (re-queued): proves repeat transactions + remount.
  cbRan = false;
  static uint8_t tx2[4] = {'1','2','3','4'};
  static uint8_t rx2[4] = {0};
  static uint8_t mosi2[4] = {'e','f','g','h'};
  static uint8_t miso2[4] = {0};
  spi_slave_transaction_t t2 = {};
  t2.length = 32; t2.tx_buffer = tx2; t2.rx_buffer = rx2;
  spi_slave_queue_trans(HSPI_HOST, &t2, portMAX_DELAY);
  SPI.beginTransaction(SPISettings(1000000, MSBFIRST, SPI_MODE0));
  SPI.transferBytes(mosi2, miso2, 4);
  SPI.endTransaction();
  esp_err_t g2 = spi_slave_get_trans_result(HSPI_HOST, &ret, 5000 / portTICK_PERIOD_MS);
  Serial.printf("GET2=%d CB=%d", (int)g2, (int)cbRan);
  Serial.printf("RX2=%c%c%c%c MISO2=%c%c%c%c", rx2[0], rx2[1], rx2[2], rx2[3], miso2[0], miso2[1], miso2[2], miso2[3]);
  if (g2 != ESP_OK || memcmp(rx2, "efgh", 4) != 0) { Serial.println("SLV_RX2=FAIL"); pass = false; }
  else Serial.println("SLV_RX2=PASS");
  if (memcmp(miso2, "1234", 4) != 0) { Serial.println("MST_RX2=FAIL"); pass = false; }
  else Serial.println("MST_RX2=PASS");

  Serial.print("RESULT="); Serial.println(pass ? "PASS" : "FAIL");
  Serial.println("=== ALL TESTS PASSED ===");
}
void loop() { delay(1000); }
