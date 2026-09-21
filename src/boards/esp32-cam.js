// ESP32-CAM (AI-Thinker module) board preset.
//
// Same ESP32 silicon as the base board, plus an OV2640 camera, 4MB flash,
// 4MB PSRAM and an on-board LED + flash LED. The camera data path needs no
// board plumbing (the virtual OV2640 serves SCCB + DVP through the native
// I2S-camera DMA on whatever pins the firmware configures), but firmware
// written for the module assumes the memory sizes below — most notably
// PSRAM (`psramFound()`, SPIRAM heap, `CAMERA_FB_IN_PSRAM` frame buffers).

export const ESP32_CAM_BOARD_ID = "esp32-cam";

/** AI-Thinker ESP32-CAM pinout (camera + LEDs). pinReset -1 = not wired. */
export const ESP32_CAM_PINS = {
  pinPwdn: 32,
  pinReset: -1,
  pinXclk: 0,
  pinSiod: 26,
  pinSioc: 27,
  pinD7: 35,
  pinD6: 34,
  pinD5: 39,
  pinD4: 36,
  pinD3: 21,
  pinD2: 19,
  pinD1: 18,
  pinD0: 5,
  pinVsync: 25,
  pinHref: 23,
  pinPclk: 22,
  pinLed: 33,
  pinFlashLed: 4,
};

/** Memory sizes the module ships (explicit config always wins). */
export const ESP32_CAM_DEFAULTS = {
  board: ESP32_CAM_BOARD_ID,
  flashSizeMB: 4,
  psramSizeMB: 4,
  psramType: "quad",
};

/**
 * Merge a board preset into a chip config. `board` defaults to 'esp32'
 * (no overrides); unknown board ids throw. Explicit config keys always
 * win over preset defaults. Returns a fresh object.
 */
export function applyBoardPreset(config = {}) {
  const cfg = config || {};
  const board = cfg.board ?? "esp32";
  if (board === "esp32") return { board, ...cfg };
  if (board === ESP32_CAM_BOARD_ID) return { ...ESP32_CAM_DEFAULTS, ...cfg };
  throw new Error(`unknown board: ${board} (expected 'esp32' or '${ESP32_CAM_BOARD_ID}')`);
}
