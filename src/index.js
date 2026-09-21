import { ESP32 } from "./peripherals/esp32/esp32.js";
import { SimulatorWorker } from "./sab/worker-proxy.js";
import { MultiSimulator } from "./sab/MultiSimulator.js";
import { ESP32_CAM_BOARD_ID, ESP32_CAM_PINS, ESP32_CAM_DEFAULTS, applyBoardPreset } from "./boards/esp32-cam.js";

export { SimulatorWorker, MultiSimulator, ESP32, ESP32_CAM_BOARD_ID, ESP32_CAM_PINS, ESP32_CAM_DEFAULTS, applyBoardPreset };
