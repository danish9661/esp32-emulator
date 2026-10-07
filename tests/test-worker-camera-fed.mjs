// Camera host-feed cell: scripted sensor scenes through the published API.
// Firmware captures 6 QQVGA RGB565 frames with the real esp-camera driver;
// after the first capture lands, the host feeds one scripted frame
// (F[i] = (i*13+7)&0xFF, never equal to the ramp at any index) via
// feedCameraFrame. The frame-tap stream is searched post-hoc for F's
// signature — deterministic regardless of wall/sim pacing (consumption is
// latched at capture start; the tap reports every consumed byte).
// Requires the compile server (:5525). No gateway needed.
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
  Serial.println("=== CAMERA-FED TEST ===");
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
  for (int k = 0; k < 6; k++) {
    camera_fb_t *fb = NULL;
    for (int attempt = 0; attempt < 8 && !fb; attempt++) fb = esp_camera_fb_get();
    if (!fb) { Serial.printf("CAM_FB%d=FAIL\\n", k); pass = false; break; }
    Serial.printf("CAM_FB%d=%u\\n", k, (unsigned)fb->len);
    if (fb->len != 160 * 120 * 2) pass = false;
    esp_camera_fb_return(fb);
  }
  Serial.print("RESULT="); Serial.println(pass ? "PASS" : "FAIL");
  Serial.println("\\n=== ALL TESTS PASSED ===");
}
void loop() { delay(1000); }
`;

// Scripted scene: F[i] = (i*13+7) & 0xFF. Distinct from the ramp at every
// index (13i+7 = i mod 256 has no solution) and no 64-run of F can match a
// ramp run (steps of 13 vs 1 mod 256), so a contiguous 64-word tap run is
// proof the fed frame was captured.
const FRAME = 160 * 120 * 2;
const FED = new Uint8Array(FRAME);
for (let i = 0; i < FRAME; i++) FED[i] = (i * 13 + 7) & 0xff;
const SIG = Array.from(FED.slice(0, 64));

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
  await proxy.attachCameraSccb(0, 0x30);
  await proxy.setCameraFrameBytes(0, 38400);
  await proxy.armCamera(0);
  proxy.run();

  const tapBytes = [];
  let fed = false;
  let closes = 0;
  const vsyncFall = async () => {
    try { await proxy.setPinInput(25, 1); } catch {}
    await new Promise(r => setTimeout(r, 30));
    try { await proxy.setPinInput(25, 0); } catch {}
  };
  const t0 = Date.now();
  while (Date.now() - t0 < 240000) {
    await new Promise(r => setTimeout(r, 100));
    proxy.pollUart();
    // Drain the tap every iteration (staging caps at 65536 words ≈ 1.7
    // frames; draining at loop cadence loses nothing).
    try {
      for (const w of await proxy.pollCameraFrame(0)) tapBytes.push((w >>> 16) & 0xff);
    } catch {}
    // Feed the scripted scene once the first capture proves the pipeline.
    if (!fed && out.includes('CAM_FB0=')) {
      const staged = await proxy.feedCameraFrame(0, FED);
      const queued = await proxy.cameraFeedLength(0);
      console.log(`[test] fed frame: staged=${staged} queued=${queued}`);
      if (staged !== FRAME || queued !== FRAME) throw new Error('feed shortfall');
      fed = true;
    }
    if (!out.includes('RESULT=') && closes < 400) {
      await new Promise(r => setTimeout(r, 50));
      await vsyncFall();
      closes++;
    }
    if (out.includes('ALL TESTS PASSED')) break;
  }
  try {
    for (const w of await proxy.pollCameraFrame(0)) tapBytes.push((w >>> 16) & 0xff);
  } catch {}
  proxy.stop();
  await new Promise(r => setTimeout(r, 100));
  proxy.terminate();

  console.log(out.slice(-1200));
  // Search the tap stream for F's 64-byte signature as a contiguous run.
  let foundAt = -1;
  outer: for (let i = 0; i + SIG.length <= tapBytes.length; i++) {
    for (let j = 0; j < SIG.length; j++) {
      if (tapBytes[i + j] !== SIG[j]) continue outer;
    }
    foundAt = i;
    break;
  }
  console.log(`[test] tapBytes=${tapBytes.length} fedSigAt=${foundAt}`);
  const lensOk = [0, 1, 2, 3, 4, 5].every((k) => out.includes(`CAM_FB${k}=38400`));
  const ok = out.includes('CAM_INIT=0') && lensOk && out.includes('RESULT=PASS') && fed && foundAt >= 0;
  if (!ok) { console.log('[test] FAILED'); process.exit(1); }
  console.log('[test] PASSED');
}

run().catch(e => { console.error(e); process.exit(1); });
