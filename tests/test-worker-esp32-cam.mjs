import { SimulatorWorker, ESP32_CAM_PINS } from '../src/index.js';
import axios from 'axios';
import { readFileSync } from 'fs';
import { resolve, dirname } from 'path';
import { fileURLToPath } from 'url';

const __dirname = dirname(fileURLToPath(import.meta.url));

async function compile(code) {
  // Like the AI-Thinker board default: PSRAM enabled (generic ESP32 disables
  // it, so psramInit() would never run and psramFound() stays false).
  const r = await axios.post('http://localhost:5525/api/compile/start', { code, target: 'esp32', targetEngine: 'frontend', fqbn: 'esp32:esp32:esp32:PSRAM=enabled' });
  while (true) {
    const s = await axios.get(`http://localhost:5525/api/compile/status/${r.data.buildId}`);
    if (s.data.status === 'success') return s.data.binary_content;
    if (s.data.status === 'failed') throw new Error('Compile failed: ' + s.data.error);
    await new Promise(r => setTimeout(r, 1000));
  }
}

// ESP32-CAM module test: board preset (4MB flash + 4MB PSRAM), psramFound(),
// SPIRAM heap write/read-back, then the real esp-camera driver against the
// virtual OV2640 over the AI-Thinker pinout (QQVGA RGB565, DRAM frame).
const firmware = `
#include "esp_camera.h"
#include "esp_heap_caps.h"
#define CAM_PIN_PWDN ${ESP32_CAM_PINS.pinPwdn}
#define CAM_PIN_RESET ${ESP32_CAM_PINS.pinReset}
#define CAM_PIN_XCLK ${ESP32_CAM_PINS.pinXclk}
#define CAM_PIN_SIOD ${ESP32_CAM_PINS.pinSiod}
#define CAM_PIN_SIOC ${ESP32_CAM_PINS.pinSioc}
#define CAM_PIN_D7 ${ESP32_CAM_PINS.pinD7}
#define CAM_PIN_D6 ${ESP32_CAM_PINS.pinD6}
#define CAM_PIN_D5 ${ESP32_CAM_PINS.pinD5}
#define CAM_PIN_D4 ${ESP32_CAM_PINS.pinD4}
#define CAM_PIN_D3 ${ESP32_CAM_PINS.pinD3}
#define CAM_PIN_D2 ${ESP32_CAM_PINS.pinD2}
#define CAM_PIN_D1 ${ESP32_CAM_PINS.pinD1}
#define CAM_PIN_D0 ${ESP32_CAM_PINS.pinD0}
#define CAM_PIN_VSYNC ${ESP32_CAM_PINS.pinVsync}
#define CAM_PIN_HREF ${ESP32_CAM_PINS.pinHref}
#define CAM_PIN_PCLK ${ESP32_CAM_PINS.pinPclk}
void setup() {
  Serial.begin(115200);
  Serial.println("=== ESP32-CAM TEST ===");
  bool pass = true;
  // PSRAM: found + SPIRAM heap write/read-back.
  Serial.printf("PSRAM_FOUND=%d\\n", psramFound() ? 1 : 0);
  if (!psramFound()) pass = false;
  else {
    uint8_t *p = (uint8_t *)heap_caps_malloc(4096, MALLOC_CAP_SPIRAM);
    bool ok = (p != NULL);
    for (uint32_t i = 0; ok && i < 4096; i++) p[i] = (uint8_t)(i ^ 0xA5);
    for (uint32_t i = 0; ok && i < 4096; i++) ok = (p[i] == (uint8_t)(i ^ 0xA5));
    heap_caps_free(p);
    Serial.print("PSRAM_HEAP="); Serial.println(ok ? "PASS" : "FAIL");
    if (!ok) pass = false;
  }
  // Camera over the module pinout.
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
  Serial.printf("CAM_INIT=%d\\n", (int)err);
  if (err != ESP_OK) { Serial.println("RESULT=FAIL"); Serial.println("\\n=== ALL TESTS PASSED ==="); return; }
  else Serial.println("CAM_INIT=PASS");
  camera_fb_t *fb = NULL;
  for (int attempt = 0; attempt < 8 && !fb; attempt++) {
    if (attempt > 0) Serial.printf("CAM_RETRY=%d\\n", attempt);
    fb = esp_camera_fb_get();
  }
  if (!fb) { Serial.println("CAM_FB=FAIL"); pass = false; }
  else {
    Serial.printf("CAM_FB=%u %u %u\\n", (unsigned)fb->len, (unsigned)fb->width, (unsigned)fb->height);
    bool ok = (fb->len == 160 * 120 * 2) && (fb->width == 160) && (fb->height == 120);
    for (uint32_t i = 0; ok && i < fb->len; i += 4096) {
      if (fb->buf[i] != (uint8_t)(i & 0xFF)) ok = false;
    }
    if (ok && fb->buf[fb->len - 1] != (uint8_t)((fb->len - 1) & 0xFF)) ok = false;
    Serial.print("CAM_DATA="); Serial.println(ok ? "PASS" : "FAIL");
    if (!ok) pass = false;
    esp_camera_fb_return(fb);
  }
  Serial.print("RESULT="); Serial.println(pass ? "PASS" : "FAIL");
  Serial.println("\\n=== ALL TESTS PASSED ===");
}
void loop() { delay(1000); }
`;

async function run() {
  const b64 = await compile(firmware);
  const flash = new Uint8Array(new SharedArrayBuffer(4 * 1024 * 1024));
  flash.set(new Uint8Array(Buffer.from(b64, 'base64').toString('binary').split('').map(c => c.charCodeAt(0))));
  const rom = readFileSync(resolve(__dirname, '../rom/esp32-v3-rom.bin'));

  const proxy = new SimulatorWorker();
  let out = '';
  proxy._onUART = (b) => { out += String.fromCharCode(b); };
  proxy._onError = (e) => console.error('\n[test] Error:', e.message);

  // Board preset only — flash/PSRAM sizes come from the esp32-cam module.
  await proxy.init('ESP32', { board: 'esp32-cam', mmuPages: 64, strapValue: 0x13, budget: 500000000, camFrameBytes: 38400 }, flash, rom);
  console.log('BOARD=' + proxy.board + ' FLASH=' + proxy.flashSizeMB + 'MB PSRAM=' + proxy.psramSizeMB + 'MB');
  if (proxy.board !== 'esp32-cam' || proxy.flashSizeMB !== 4 || proxy.psramSizeMB !== 4) {
    console.error('[test] FAILED (board preset not applied)');
    process.exit(1);
  }
  proxy.run();

  // VSYNC frame boundaries (driver configures NEGEDGE): rapid pulses so
  // every fb_get window contains several complete frame opportunities.
  let phase = 0;
  let closes = 0;
  const vsyncFall = async () => {
    try { await proxy.setPinInput(ESP32_CAM_PINS.pinVsync, 1); } catch {}
    await new Promise(r => setTimeout(r, 30));
    try { await proxy.setPinInput(ESP32_CAM_PINS.pinVsync, 0); } catch {}
  };
  for (let i = 0; i < 800; i++) {
    await new Promise(r => setTimeout(r, 100));
    proxy.pollUart();
    if (out.includes('CAM_INIT=') && phase === 0) {
      phase = 1;
      console.log('[test] VSYNC pulsing started');
    }
    if (phase >= 1 && closes < 240 && !out.includes('CAM_FB=')) {
      await new Promise(r => setTimeout(r, 50));
      await vsyncFall();
      closes++;
    }
    if (out.includes('ALL TESTS PASSED')) break;
  }
  proxy.stop();
  await new Promise(r => setTimeout(r, 100));
  proxy.terminate();

  const get = (re) => { const m = out.match(re); return m ? m[1] : 'MISSING'; };
  console.log('PSRAM_FOUND=' + get(/PSRAM_FOUND=(\d+)/));
  console.log('PSRAM_HEAP=' + get(/PSRAM_HEAP=(\w+)/));
  console.log('CAM_INIT=' + get(/CAM_INIT=([-\d]+|PASS)/));
  console.log('CAM_FB=' + get(/CAM_FB=(.*)/), 'CAM_DATA=' + get(/CAM_DATA=(\w+)/));
  const result = get(/RESULT=(\w+)/);
  console.log('RESULT=' + result);
  if (process.env.CAMDUMP) console.log('---UART---\n' + out.slice(-3000));
  if (result !== 'PASS') { console.error('[test] FAILED'); process.exit(1); }
  console.log('[test] PASSED');
}

run().catch(e => { console.error(e); process.exit(1); });
