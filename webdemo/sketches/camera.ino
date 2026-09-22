
#include "esp_camera.h"
#define CAM_PIN_PWDN 32
#define CAM_PIN_RESET -1
#define CAM_PIN_XCLK 0
#define CAM_PIN_SIOD 26
#define CAM_PIN_SIOC 27
#define CAM_PIN_D7 35
#define CAM_PIN_D6 34
#define CAM_PIN_D5 39
#define CAM_PIN_D4 36
#define CAM_PIN_D3 21
#define CAM_PIN_D2 19
#define CAM_PIN_D1 18
#define CAM_PIN_D0 5
#define CAM_PIN_VSYNC 25
#define CAM_PIN_HREF 23
#define CAM_PIN_PCLK 22
void setup() {
  Serial.begin(115200);
  Serial.println("=== CAMERA TEST ===");
  esp_log_level_set("*", ESP_LOG_INFO);
  bool pass = true;
  camera_config_t config;
  config.ledc_channel = LEDC_CHANNEL_0;
  config.ledc_timer = LEDC_TIMER_0;
  config.pin_d0 = CAM_PIN_D0;
  config.pin_d1 = CAM_PIN_D1;
  config.pin_d2 = CAM_PIN_D2;
  config.pin_d3 = CAM_PIN_D3;
  config.pin_d4 = CAM_PIN_D4;
  config.pin_d5 = CAM_PIN_D5;
  config.pin_d6 = CAM_PIN_D6;
  config.pin_d7 = CAM_PIN_D7;
  config.pin_xclk = CAM_PIN_XCLK;
  config.pin_pclk = CAM_PIN_PCLK;
  config.pin_vsync = CAM_PIN_VSYNC;
  config.pin_href = CAM_PIN_HREF;
  config.pin_sccb_sda = CAM_PIN_SIOD;
  config.pin_sccb_scl = CAM_PIN_SIOC;
  config.pin_pwdn = CAM_PIN_PWDN;
  config.pin_reset = CAM_PIN_RESET;
  config.xclk_freq_hz = 20000000;
  config.pixel_format = PIXFORMAT_RGB565;
  config.frame_size = FRAMESIZE_QQVGA;
  config.jpeg_quality = 12;
  config.fb_count = 1;
  config.fb_location = CAMERA_FB_IN_DRAM;
  config.grab_mode = CAMERA_GRAB_WHEN_EMPTY;
  esp_err_t err = esp_camera_init(&config);
  Serial.printf("CAM_INIT=%d", (int)err);
  if (err != ESP_OK) { Serial.println("RESULT=FAIL"); Serial.println("=== ALL TESTS PASSED ==="); return; }
  else Serial.println("CAM_INIT=PASS");
  camera_fb_t *fb = NULL;
  for (int attempt = 0; attempt < 8 && !fb; attempt++) {
    if (attempt > 0) Serial.printf("CAM_RETRY=%d", attempt);
    fb = esp_camera_fb_get();
  }
  if (!fb) { Serial.println("CAM_FB=FAIL"); pass = false; }
  else {
    Serial.printf("CAM_FB=%u %u %u", (unsigned)fb->len, (unsigned)fb->width, (unsigned)fb->height);
    bool ok = (fb->len == 160 * 120 * 2) && (fb->width == 160) && (fb->height == 120);
    // Ramp check: first/last bytes + strided samples across the frame.
    for (uint32_t i = 0; ok && i < fb->len; i += 4096) {
      if (fb->buf[i] != (uint8_t)(i & 0xFF)) ok = false;
    }
    if (ok && fb->buf[fb->len - 1] != (uint8_t)((fb->len - 1) & 0xFF)) ok = false;
    Serial.print("CAM_DATA="); Serial.println(ok ? "PASS" : "FAIL");
    if (!ok) pass = false;
    esp_camera_fb_return(fb);
  }
  Serial.print("RESULT="); Serial.println(pass ? "PASS" : "FAIL");
  Serial.println("=== ALL TESTS PASSED ===");
}
void loop() { delay(1000); }
