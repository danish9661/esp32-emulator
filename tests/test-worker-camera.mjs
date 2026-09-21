import { SimulatorWorker } from '../src/index.js';
import axios from 'axios';
import { readFileSync } from 'fs';
import { resolve, dirname } from 'path';
import { fileURLToPath } from 'url';

const __dirname = dirname(fileURLToPath(import.meta.url));

async function compile(code) {
  const r = await axios.post('http://localhost:5525/api/compile/start', { code, target: 'esp32', targetEngine: 'frontend', fqbn: 'esp32:esp32:esp32' });
  while (true) {
    const s = await axios.get(`http://localhost:5525/api/compile/status/${r.data.buildId}`);
    if (s.data.status === 'success') return s.data.binary_content;
    if (s.data.status === 'failed') throw new Error('Compile failed: ' + s.data.error);
    await new Promise(r => setTimeout(r, 1000));
  }
}

// Real esp-camera driver against the virtual OV2640: SCCB probe + full
// register init over the native I2C master, then one RGB565 QQVGA frame via
// I2S0 camera DMA. The sensor serves PID (0x26) + file-backed init regs;
// DVP bytes are the virtual-camera ramp (byte[i] = i & 0xFF). The host
// pulses VSYNC (GPIO 25) while the frame is awaited — capture itself is
// synchronous once VSYNC arrives (no tight timing).
const firmware = `
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

  await proxy.init('ESP32', { flashSizeMB: 4, mmuPages: 64, strapValue: 0x13, budget: 500000000, camFrameBytes: 38400 }, flash, rom);
  proxy.run();

  // VSYNC: falling edges (driver configures NEGEDGE). The sim runs much
  // faster than wall-clock, so firmware fb_get windows elapse in a fraction
  // of a second of wall-time: pulse a frame boundary every ~150ms from
  // CAM_INIT until CAM_FB so every attempt window contains several complete
  // frame opportunities. A boundary ending a partial frame just discards it
  // (FB-SIZE error, harmless); the next boundary starts fresh.
  let phase = 0;
  let closes = 0;
  const vsyncFall = async () => {
    try { await proxy.setPinInput(25, 1); } catch {}
    await new Promise(r => setTimeout(r, 30));
    try { await proxy.setPinInput(25, 0); } catch {}
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
      if (closes % 40 === 0) console.log(`[test] VSYNC pulses=${closes}`);
    }
    if (out.includes('ALL TESTS PASSED')) break;
  }
  proxy.stop();
  await new Promise(r => setTimeout(r, 100));
  proxy.terminate();

  const get = (re) => { const m = out.match(re); return m ? m[1] : 'MISSING'; };
  console.log('CAM_INIT=' + get(/CAM_INIT=([-\d]+|PASS)/));
  console.log('CAM_FB=' + get(/CAM_FB=(.*)/), 'CAM_DATA=' + get(/CAM_DATA=(\w+)/));
  const result = get(/RESULT=(\w+)/);
  console.log('RESULT=' + result);
  if (process.env.CAMDUMP) console.log('---UART---\n' + out.slice(-3000));
  if (result !== 'PASS') { console.error('[test] FAILED'); process.exit(1); }
  console.log('[test] PASSED');
}

run().catch(e => { console.error(e); process.exit(1); });
