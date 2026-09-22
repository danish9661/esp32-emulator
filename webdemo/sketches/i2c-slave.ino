
#include <Wire.h>
#include <driver/i2c_master.h>

volatile bool gotMsg = false;
char smsg[16] = {0};

void onRecv(int n) {
  int i = 0;
  while (Wire1.available() && i < 15) smsg[i++] = (char)Wire1.read();
  smsg[i] = 0;
  gotMsg = true;
}

void onReq() {
  Wire1.write((const uint8_t*)"Hi!", 3);
}

void waitGo() {
  for (int i = 0; i < 6000 && !Serial.available(); i++) delay(10);
  if (Serial.available()) Serial.read();
}

void setup() {
  Serial.begin(115200);
  Serial.println("=== I2CSLAVE TEST ===");
  bool pass = true;

  // Master bus FIRST (order test: master intr_alloc before slave intr_alloc).
  i2c_master_bus_config_t bus_cfg = {};
  bus_cfg.i2c_port = 0;
  bus_cfg.sda_io_num = (gpio_num_t)21;
  bus_cfg.scl_io_num = (gpio_num_t)22;
  bus_cfg.clk_source = I2C_CLK_SRC_DEFAULT;
  bus_cfg.glitch_ignore_cnt = 7;
  bus_cfg.flags.enable_internal_pullup = 1;
  i2c_master_bus_handle_t bus;
  esp_err_t e1 = i2c_new_master_bus(&bus_cfg, &bus);
  Serial.printf("NEWBUS=0x%x", e1);
  i2c_device_config_t dev_cfg = {};
  dev_cfg.dev_addr_length = I2C_ADDR_BIT_LEN_7;
  dev_cfg.device_address = 0x42;
  dev_cfg.scl_speed_hz = 100000;
  i2c_master_dev_handle_t dev;
  esp_err_t e2 = i2c_master_bus_add_device(bus, &dev_cfg, &dev);
  Serial.printf("ADDDEV=0x%x", e2);

  bool slv = Wire1.begin((uint8_t)0x42, 18, 19, 100000);
  Serial.printf("SLAVE_BEGIN=%d", (int)slv);
  Wire1.write((const uint8_t*)"OK", 2);  // preload slave TX for the first master read
  Wire1.onReceive(onRecv);
  Wire1.onRequest(onReq);

  // 1. Prime read: slave TX FIFO is empty (Arduino buffers preload in RAM
  // until onRequest flushes), so this returns filler — but its completion
  // fires the slave TX event, running onRequest which flushes "Hi!".
  uint8_t prime[2] = {0};
  esp_err_t r0 = i2c_master_receive(dev, prime, 2, 1000);
  Serial.printf("PRIME=0x%x", r0);
  if (r0 != ESP_OK) { Serial.println("PRIME_OK=FAIL"); pass = false; }
  else Serial.println("PRIME_OK=PASS");

  // 2. The prime read's onRequest flushed "Hi!" — read it back.
  // (No master writes before this: RX delivery shares FIFO slots with TX.)
  uint8_t r2[4] = {0};
  esp_err_t e3 = i2c_master_receive(dev, r2, 3, 1000);
  Serial.printf("READ2=%c%c%c", r2[0], r2[1], r2[2]);
  if (e3 != ESP_OK || r2[0] != 'H' || r2[1] != 'i' || r2[2] != '!') { Serial.println("REFILL=FAIL"); pass = false; }
  else Serial.println("REFILL=PASS");

  // 3. Master write -> slave onReceive.
  uint8_t wbuf[3] = {0x41, 0x42, 0x43};
  esp_err_t e = i2c_master_transmit(dev, wbuf, 3, 1000);
  Serial.printf("TXERR=0x%x", e);
  uint32_t mraw = REG_READ(0x3FF53000 + 0x20);
  uint32_t mena = REG_READ(0x3FF53000 + 0x28);
  uint32_t sraw = REG_READ(0x3FF67000 + 0x20);
  uint32_t sena = REG_READ(0x3FF67000 + 0x28);
  Serial.printf("MRAW=0x%x MENA=0x%x SRAW=0x%x SENA=0x%x", mraw, mena, sraw, sena);
  if (e != ESP_OK) { Serial.println("WR_OK=FAIL"); pass = false; }
  else Serial.println("WR_OK=PASS");

  Serial.println("SLAVE_READY");
  // Wait for the host 'go' (slave ISR + onReceive have run by then).
  waitGo();

  Serial.printf("GOT=%d MSG=%s", (int)gotMsg, smsg);
  if (!gotMsg || String(smsg) != "ABC") { Serial.println("ONRECV=FAIL"); pass = false; }
  else Serial.println("ONRECV=PASS");

  Serial.print("RESULT="); Serial.println(pass ? "PASS" : "FAIL");
  Serial.println("=== ALL TESTS PASSED ===");
}
void loop() { delay(1000); }
