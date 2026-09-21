// Test helper (NOT a test — run-worker-tests.sh ignores it): forces
// board:'esp32-cam' into every SimulatorWorker.init config so the stock
// protocol tests run against the ESP32-CAM module preset. Usage:
//   node --import ./tests/board-hook.mjs tests/test-worker-gpio.mjs
import { SimulatorWorker } from '../src/index.js';

const origInit = SimulatorWorker.prototype.init;
SimulatorWorker.prototype.init = function (chipType, config = {}, ...rest) {
  config = { board: 'esp32-cam', ...config };
  return origInit.call(this, chipType, config, ...rest);
};
console.log('[board-hook] forcing board=esp32-cam');
