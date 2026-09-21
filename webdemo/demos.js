// Demo catalogue: 19 prebuilt firmware images + the sim config each needs.
export const DEMOS = [
  { id: 'gpio', name: 'GPIO', file: 'fw-gpio.bin.gz', desc: 'Output, pull-up, interrupt attach', expect: 'ALL TESTS PASSED' },
  { id: 'uart', name: 'UART', file: 'fw-uart.bin.gz', desc: 'Serial0/1/2 loopback', expect: 'ALL TESTS PASSED' },
  { id: 'spi', name: 'SPI', file: 'fw-spi.bin.gz', desc: 'Loopback transfer 0xAA', expect: 'LOOPBACK=PASS' },
  { id: 'i2c', name: 'I2C', file: 'fw-i2c.bin.gz', desc: 'Master write probe', expect: 'I2C=PASS' },
  { id: 'timer', name: 'HW timer', file: 'fw-timer.bin.gz', desc: '1 MHz timer create/read/stop', expect: 'ALL TESTS PASSED' },
  { id: 'pwm', name: 'PWM + fade', file: 'fw-pwm.bin.gz', desc: 'LEDC write + 0-255 hardware fade', expect: 'FADE_DATA=PASS' },
  { id: 'rmt', name: 'RMT', file: 'fw-rmt.bin.gz', desc: 'TX two NEC-like items, RX loopback', expect: 'ALL TESTS PASSED' },
  { id: 'twai', name: 'TWAI / CAN', file: 'fw-twai.bin.gz', desc: 'Self-transmit std frame 0x123', expect: 'ALL TESTS PASSED' },
  { id: 'analog', name: 'ADC one-shot', file: 'fw-analog.bin.gz', desc: '4 channels, attenuation table', config: { analogInputs: { 36: 1.65, 32: 0.0, 33: 3.3, 34: 1.65 } }, expect: 'RESULT=PASS' },
  { id: 'adc-cont', name: 'ADC continuous', file: 'fw-adc-cont.bin.gz', desc: 'DMA frames via I2S path', config: { analogInputs: { 36: 2.0 } }, expect: 'ALL TESTS PASSED' },
  { id: 'touch-dac', name: 'Touch + DAC', file: 'fw-touch-dac.bin.gz', desc: 'Touch IRQ, DAC to ADC loopback', config: { touchInputs: { 2: 320 } }, script: 'touch-dyn', expect: 'ALL TESTS PASSED' },
  { id: 'multicore', name: 'Dual core', file: 'fw-multicore.bin.gz', desc: 'Core 0 + core 1 shared counters', expect: 'RESULT=PASS' },
  { id: 'pcnt', name: 'Pulse counter', file: 'fw-pcnt.bin.gz', desc: '10 host-driven edges, threshold IRQ', script: 'pcnt-edges', expect: 'COUNT10=PASS' },
  { id: 'bod', name: 'Brownout', file: 'fw-bod.bin.gz', desc: 'Rail dip to 2.0 V, BOD IRQ + reset', script: 'bod-rail', expect: 'BODRESET=PASS' },
  { id: 'bt', name: 'BT controller', file: 'fw-bt.bin.gz', desc: 'Controller init (no baseband)', config: { bleShim: true }, budget: 8000000, expect: 'RESULT=PASS' },
  { id: 'ble-init', name: 'BLE init', file: 'fw-ble-init.bin.gz', desc: 'Bluedroid init, HCI wait', config: { bleShim: true }, budget: 10000000, expect: 'RESULT=PASS' },
  { id: 'emac-loopback', name: 'Ethernet MAC', file: 'fw-emac-loopback.bin.gz', desc: 'PHY ID + 64 B DMA loopback', expect: 'LOOPBACK=PASS' },
  { id: 'camera', name: 'Camera', file: 'fw-camera.bin.gz', desc: 'OV2640 frame 38400 B via VSYNC drive', config: { camFrameBytes: 38400 }, budget: 500000000, script: 'camera-vsync', expect: 'CAM_DATA=PASS' },
  { id: 'deepsleep', name: 'Deep sleep', file: 'fw-deepsleep.bin.gz', desc: '3 boots, RTC memory survives', expect: 'RESULT=PASS' },
];

// Measured on this machine (Node, gpio image, run/stop hot loop): ~709 MIPS
// steady-state. Per-demo page shows a live MIPS readout instead.
export const MEASURED_MIPS = 709;

// Interactive scripts: host actions the demo needs while it runs.
export async function runDemoScript(kind, sim, log) {
  if (kind === 'touch-dyn') {
    // touch-dac waits at DYN_READY for pad 3 to be driven low.
    const t0 = Date.now();
    while (Date.now() - t0 < 60000) {
      await new Promise((r) => setTimeout(r, 300));
      if (log().includes('DYN_READY')) { await sim.setTouchInput(3, 300); return; }
      if (log().includes('ALL TESTS PASSED')) return;
    }
  } else if (kind === 'pcnt-edges') {
    const t0 = Date.now();
    let driven = false;
    while (Date.now() - t0 < 60000) {
      await new Promise((r) => setTimeout(r, 200));
      sim.pollUart();
      if (!driven && log().includes('PCNT_READY')) {
        driven = true;
        for (let i = 0; i < 10; i++) {
          await sim.setPinInput(4, 1);
          await new Promise((r) => setTimeout(r, 60));
          await sim.setPinInput(4, 0);
          await new Promise((r) => setTimeout(r, 60));
        }
        sim.sendUart('g');
      }
      if (log().includes('ALL TESTS PASSED')) return;
    }
  } else if (kind === 'bod-rail') {
    const t0 = Date.now();
    let lo = false;
    while (Date.now() - t0 < 90000) {
      await new Promise((r) => setTimeout(r, 200));
      sim.pollUart();
      const text = log();
      if ((text.match(/BOOT #/g) || []).length > 0) { try { await sim.setVoltageMv(3300); } catch {} }
      if (!lo && text.includes('LOW_READY')) { lo = true; try { await sim.setVoltageMv(2000); } catch {} }
      if (text.includes('RST_READY')) { try { await sim.setVoltageMv(2000); } catch {} }
      if (text.includes('ALL TESTS PASSED')) return;
    }
  } else if (kind === 'camera-vsync') {
    const t0 = Date.now();
    let phase = 0;
    while (Date.now() - t0 < 120000) {
      await new Promise((r) => setTimeout(r, 100));
      sim.pollUart();
      if (log().includes('CAM_INIT=') && phase === 0) phase = 1;
      if (phase === 1) {
        try { await sim.setPinInput(25, 1); } catch {}
        await new Promise((r) => setTimeout(r, 30));
        try { await sim.setPinInput(25, 0); } catch {}
      }
      if (log().includes('ALL TESTS PASSED')) return;
    }
  }
}
