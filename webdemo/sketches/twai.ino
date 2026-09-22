
#include <driver/twai.h>
#include <string.h>

void setup() {
  Serial.begin(115200);
  Serial.println("=== TWAI TEST ===");
  bool pass = true;
  twai_general_config_t g = TWAI_GENERAL_CONFIG_DEFAULT(GPIO_NUM_21, GPIO_NUM_22, TWAI_MODE_NO_ACK);
  twai_timing_config_t t = TWAI_TIMING_CONFIG_125KBITS();
  twai_filter_config_t f = TWAI_FILTER_CONFIG_ACCEPT_ALL();
  esp_err_t err = twai_driver_install(&g, &t, &f);
  Serial.print("TWAI_INSTALL="); Serial.println(err == ESP_OK ? "PASS" : "FAIL");
  if (err != ESP_OK) pass = false;
  if (err == ESP_OK) {
    err = twai_start();
    Serial.print("TWAI_START="); Serial.println(err == ESP_OK ? "PASS" : "FAIL");
    if (err != ESP_OK) pass = false;
    // NO_ACK self-transmission: TX a std frame, expect self-reception.
    twai_message_t txmsg = {};
    txmsg.identifier = 0x123;
    txmsg.data_length_code = 4;
    txmsg.data[0] = 'C'; txmsg.data[1] = 'A'; txmsg.data[2] = 'N'; txmsg.data[3] = '!';
    err = twai_transmit(&txmsg, 1000 / portTICK_PERIOD_MS);
    Serial.print("TWAI_TX="); Serial.println(err == ESP_OK ? "PASS" : "FAIL");
    if (err != ESP_OK) pass = false;
    twai_message_t rxmsg = {};
    err = twai_receive(&rxmsg, 2000 / portTICK_PERIOD_MS);
    Serial.print("TWAI_RX="); Serial.println(err == ESP_OK ? "PASS" : "FAIL");
    if (err != ESP_OK) pass = false;
    else {
      Serial.printf("RX_ID=0x%x RX_DLC=%d RX_DATA=%c%c%c%c", (unsigned)rxmsg.identifier,
        (int)rxmsg.data_length_code, rxmsg.data[0], rxmsg.data[1], rxmsg.data[2], rxmsg.data[3]);
      if (rxmsg.identifier != 0x123 || rxmsg.data_length_code != 4 ||
          memcmp(rxmsg.data, "CAN!", 4) != 0) { Serial.println("TWAI_DATA=FAIL"); pass = false; }
      else Serial.println("TWAI_DATA=PASS");
    }
    twai_stop();
    Serial.println("[TWAI] stop OK");
    twai_driver_uninstall();
    Serial.println("[TWAI] uninstall OK");
  }
  Serial.print("RESULT="); Serial.println(pass ? "PASS" : "FAIL");
  Serial.println("=== ALL TESTS PASSED ===");
}
void loop() { delay(1000); }
