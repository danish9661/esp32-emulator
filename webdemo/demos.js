// Demo catalogue: 53 prebuilt firmware images + the sim config each needs.
export const DEMOS = [
  { id: 'gpio', name: 'GPIO', file: 'fw-gpio.bin.gz', desc: 'Output, pull-up, interrupt attach', expect: 'ALL TESTS PASSED' },
  { id: 'uart', name: 'UART', file: 'fw-uart.bin.gz', desc: 'Serial0/1/2 loopback', expect: 'ALL TESTS PASSED' },
  { id: 'uart-echo', name: 'UART echo', file: 'fw-uart-echo.bin.gz', desc: 'Interactive: type, chip echoes', interactive: 'uart-echo', expect: 'ECHO_READY' },
  { id: 'spi', name: 'SPI', file: 'fw-spi.bin.gz', desc: 'Loopback transfer 0xAA', expect: 'LOOPBACK=PASS' },
  { id: 'spi-slave', name: 'SPI slave', file: 'fw-spi-slave.bin.gz', desc: 'Master-slave full-duplex x2', expect: 'RESULT=PASS' },
  { id: 'i2c', name: 'I2C', file: 'fw-i2c.bin.gz', desc: 'Master write probe', expect: 'I2C=PASS' },
  { id: 'i2c-slave', name: 'I2C slave', file: 'fw-i2c-slave.bin.gz', desc: 'Slave recv + master write', config: { pinInputs: { 18: 1, 19: 1 } }, script: 'i2c-slave-go', expect: 'RESULT=PASS' },
  { id: 'rtc-i2c', name: 'RTC I2C', file: 'fw-rtc-i2c.bin.gz', desc: 'RTC I2C controller path', budget: 8000000, expect: 'RESULT=PASS' },
  { id: 'timer', name: 'HW timer', file: 'fw-timer.bin.gz', desc: '1 MHz timer create/read/stop', expect: 'ALL TESTS PASSED' },
  { id: 'timer-freq', name: 'Timer freq', file: 'fw-timer-freq.bin.gz', desc: '1 ms vs 10 s ratio + micros/millis', expect: 'RATIO=PASS' },
  { id: 'rtc-wdt', name: 'RTC + WDT', file: 'fw-rtc-wdt.bin.gz', desc: 'RTC watchdog feed + reset', expect: 'RESULT=PASS' },
  { id: 'twdt', name: 'Task WDT', file: 'fw-twdt.bin.gz', desc: 'Task watchdog via gptimer', budget: 15000000, expect: 'RESULT=PASS' },
  { id: 'pwm', name: 'PWM + fade', file: 'fw-pwm.bin.gz', desc: 'LEDC write + 0-255 hardware fade', expect: 'FADE_DATA=PASS' },
  { id: 'mcpwm', name: 'MCPWM', file: 'fw-mcpwm.bin.gz', desc: 'Duty/freq + capture + fault', script: 'mcpwm-drive', expect: 'RESULT=PASS' },
  { id: 'rmt', name: 'RMT', file: 'fw-rmt.bin.gz', desc: 'TX two NEC-like items, RX loopback', expect: 'ALL TESTS PASSED' },
  { id: 'i2s', name: 'I2S audio', file: 'fw-i2s.bin.gz', desc: 'DMA TX completes 64 B', expect: 'RESULT=PASS' },
  { id: 'i2s-rx', name: 'I2S mic', file: 'fw-i2s-rx.bin.gz', desc: 'Host-fed RX, 256 B exact', script: 'i2s-rx-feed', expect: 'RESULT=PASS' },
  { id: 'twai', name: 'TWAI / CAN', file: 'fw-twai.bin.gz', desc: 'Self-transmit std frame 0x123', expect: 'ALL TESTS PASSED' },
  { id: 'twai-normal', name: 'TWAI normal', file: 'fw-twai-normal.bin.gz', desc: 'Peer frame inject + TX capture', budget: 8000000, script: 'twai-peer', expect: 'RESULT=PASS' },
  { id: 'analog', name: 'ADC one-shot', file: 'fw-analog.bin.gz', desc: '4 channels, attenuation table', config: { analogInputs: { 36: 1.65, 32: 0.0, 33: 3.3, 34: 1.65 } }, expect: 'RESULT=PASS' },
  { id: 'adc-cont', name: 'ADC continuous', file: 'fw-adc-cont.bin.gz', desc: 'DMA frames via I2S path', config: { analogInputs: { 36: 2.0 } }, expect: 'ALL TESTS PASSED' },
  { id: 'tsens', name: 'Temp sensor', file: 'fw-tsens.bin.gz', desc: 'temperatureRead 25 C range', expect: 'RESULT=PASS' },
  { id: 'touch-dac', name: 'Touch + DAC', file: 'fw-touch-dac.bin.gz', desc: 'Touch IRQ, DAC to ADC loopback', config: { touchInputs: { 2: 320 } }, script: 'touch-dyn', expect: 'ALL TESTS PASSED' },
  { id: 'multicore', name: 'Dual core', file: 'fw-multicore.bin.gz', desc: 'Core 0 + core 1 shared counters', expect: 'RESULT=PASS' },
  { id: 'interrupt-stress', name: 'IRQ stress', file: 'fw-interrupt-stress.bin.gz', desc: 'Timer + GPIO IRQ storm', budget: 10000000, expect: 'RESULT=PASS' },
  { id: 'pcnt', name: 'Pulse counter', file: 'fw-pcnt.bin.gz', desc: '10 host-driven edges, threshold IRQ', script: 'pcnt-edges', expect: 'COUNT10=PASS' },
  { id: 'bod', name: 'Brownout', file: 'fw-bod.bin.gz', desc: 'Rail dip to 2.0 V, BOD IRQ + reset', script: 'bod-rail', expect: 'BODRESET=PASS' },
  { id: 'ulp', name: 'ULP coproc', file: 'fw-ulp.bin.gz', desc: 'FSM program load + run + data', expect: 'RESULT=PASS' },
  { id: 'ulp-wake', name: 'ULP wake', file: 'fw-ulp-wake.bin.gz', desc: 'ULP wake from deep sleep', budget: 8000000, expect: 'RESULT=PASS' },
  { id: 'flash', name: 'Flash R/W', file: 'fw-flash.bin.gz', desc: 'Raw flash erase/write/read', budget: 8000000, expect: 'RESULT=PASS' },
  { id: 'part', name: 'Partitions', file: 'fw-part.bin.gz', desc: 'OTA table enumerate', budget: 8000000, partitions: 'nvs,data,nvs,0x9000,0x5000\notadata,data,ota,0xe000,0x2000\nota_0,app,ota_0,0x10000,0x1E0000\nota_1,app,ota_1,0x200000,0x1E0000', expect: 'ALL TESTS PASSED' },
  { id: 'ota', name: 'OTA update', file: 'fw-ota.bin.gz', desc: 'OTA begin/write/end cycle', budget: 120000000, partitions: 'nvs,data,nvs,0x9000,0x5000\notadata,data,ota,0xe000,0x2000\nota_0,app,ota_0,0x10000,0x1E0000\nota_1,app,ota_1,0x200000,0x1E0000\nspiffs,data,spiffs,0x3E0000,0x20000', expect: 'RESULT=PASS' },
  { id: 'ota-multi', name: 'OTA slots', file: 'fw-ota-multi.bin.gz', desc: 'Running/next partition cycle', budget: 10000000, expect: 'RESULT=PASS' },
  { id: 'default-pt', name: 'Default part.', file: 'fw-default-pt.bin.gz', desc: 'Factory partition table dump', budget: 8000000, expect: 'ALL TESTS PASSED' },
  { id: 'spiffs', name: 'SPIFFS', file: 'fw-spiffs.bin.gz', desc: 'Mount + file write/read', budget: 12000000, expect: 'RESULT=PASS' },
  { id: 'sdmmc', name: 'SDMMC', file: 'fw-sdmmc.bin.gz', desc: 'SD host init/deinit', expect: 'RESULT=PASS' },
  { id: 'sdcard', name: 'SD card', file: 'fw-sdcard.bin.gz', desc: 'FAT mount, 4 KB file R/W', config: { sdCard: { sizeMB: 16 } }, expect: 'RESULT=PASS' },
  { id: 'emmc', name: 'eMMC', file: 'fw-emmc.bin.gz', desc: 'MMC probe + sector R/W', config: { sdCard: { sizeMB: 16, type: 'mmc' } }, expect: 'RESULT=PASS' },
  { id: 'sdio-slave', name: 'SDIO slave', file: 'fw-sdio-slave.bin.gz', desc: 'Slave init/start/recv timeout', expect: 'RESULT=PASS' },
  { id: 'uhci', name: 'UHCI', file: 'fw-uhci.bin.gz', desc: 'Raw UART DMA register R/W', expect: 'RESULT=PASS' },
  { id: 'sweep', name: 'Reg sweep', file: 'fw-sweep.bin.gz', desc: 'Secure-boot/I2C/SLC/FE pages', expect: 'RESULT=PASS' },
  { id: 'rsa', name: 'RSA accel', file: 'fw-rsa.bin.gz', desc: 'Hardware RSA operation', expect: 'RESULT=PASS' },
  { id: 'ds', name: 'Dig. signature', file: 'fw-ds.bin.gz', desc: 'DS register file stub', expect: 'RESULT=PASS' },
  { id: 'bt', name: 'BT controller', file: 'fw-bt.bin.gz', desc: 'Controller init (no baseband)', config: { bleShim: true }, budget: 8000000, expect: 'RESULT=PASS' },
  { id: 'ble-init', name: 'BLE init', file: 'fw-ble-init.bin.gz', desc: 'Bluedroid init, HCI wait', config: { bleShim: true }, budget: 10000000, expect: 'RESULT=PASS' },
  { id: 'wifi', name: 'WiFi scan', file: 'fw-wifi.bin.gz', desc: 'Station scan finds TEST-AP', budget: 80000000, config: { wifi: { ssid: 'TEST-AP', channel: 6 }, macAddress: '24:0a:c4:12:34:56' }, expect: 'ALL TESTS PASSED' },
  { id: 'softap', name: 'Soft-AP', file: 'fw-softap.bin.gz', desc: 'AP init, IP, MAC, station count', budget: 500000000, config: { wifi: false, macAddress: '24:0a:c4:12:34:59' }, expect: 'RESULT=PASS' },
  { id: 'emac', name: 'Ethernet reg', file: 'fw-emac.bin.gz', desc: 'EMAC register readback', expect: 'RESULT=PASS' },
  { id: 'emac-loopback', name: 'Ethernet MAC', file: 'fw-emac-loopback.bin.gz', desc: 'PHY ID + 64 B DMA loopback', expect: 'LOOPBACK=PASS' },
  { id: 'camera', name: 'Camera', file: 'fw-camera.bin.gz', desc: 'OV2640 frame 38400 B via VSYNC drive', config: { camFrameBytes: 38400 }, budget: 500000000, script: 'camera-vsync', expect: 'CAM_DATA=PASS' },
  { id: 'esp32-cam', name: 'ESP32-CAM', file: 'fw-esp32-cam.bin.gz', desc: 'Module preset: PSRAM + camera', board: 'esp32-cam', budget: 500000000, config: { camFrameBytes: 38400 }, script: 'camera-vsync', expect: 'CAM_DATA=PASS' },
  { id: 'deepsleep', name: 'Deep sleep', file: 'fw-deepsleep.bin.gz', desc: '3 boots, RTC memory survives', expect: 'RESULT=PASS' },
  { id: 'buttons', name: 'Buttons', file: 'fw-buttons.bin.gz', desc: 'RESET + BOOT pins, 4 boots', script: 'buttons-press', expect: 'BOOT #4' },
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
  } else if (kind === 'mcpwm-drive') {
    // mcpwm waits on UART keys at CAP_READY / CAP_READY2 / FAULT_READY.
    const t0 = Date.now();
    let cap1 = false, cap2 = false, flt = false;
    while (Date.now() - t0 < 90000) {
      await new Promise((r) => setTimeout(r, 200));
      sim.pollUart();
      const text = log();
      if (!cap1 && text.includes('CAP_READY')) {
        cap1 = true;
        await sim.setPinInput(4, 0);
        await new Promise((r) => setTimeout(r, 300));
        await sim.setPinInput(4, 1);
        await new Promise((r) => setTimeout(r, 300));
        sim.sendUart('a');
      }
      if (!cap2 && text.includes('CAP_READY2')) {
        cap2 = true;
        await sim.setPinInput(4, 0);
        await new Promise((r) => setTimeout(r, 300));
        await sim.setPinInput(4, 1);
        await new Promise((r) => setTimeout(r, 300));
        sim.sendUart('b');
      }
      if (!flt && text.includes('FAULT_READY')) {
        flt = true;
        await sim.setPinInput(5, 1);
        await new Promise((r) => setTimeout(r, 300));
        sim.sendUart('c');
      }
      if (text.includes('ALL TESTS PASSED')) return;
    }
  } else if (kind === 'i2c-slave-go') {
    const t0 = Date.now();
    let sent = false;
    while (Date.now() - t0 < 90000) {
      await new Promise((r) => setTimeout(r, 200));
      sim.pollUart();
      if (!sent && log().includes('SLAVE_READY')) {
        sent = true;
        await new Promise((r) => setTimeout(r, 800));
        sim.sendUart('g');
      }
      if (log().includes('ALL TESTS PASSED')) return;
    }
  } else if (kind === 'twai-peer') {
    const t0 = Date.now();
    let sent = false;
    while (Date.now() - t0 < 90000) {
      await new Promise((r) => setTimeout(r, 200));
      sim.pollUart();
      if (!sent && log().includes('READY')) {
        sent = true;
        await sim.sendTwaiFrame(0x123, [0x43, 0x41, 0x4e, 0x21]);
      }
      if (log().includes('ALL TESTS PASSED')) return;
    }
  } else if (kind === 'i2s-rx-feed') {
    // i2s-rx installs the RX driver, then blocks reading until 512 host-fed
    // words arrive (values 0x1000+i, verified word-exact by firmware).
    const t0 = Date.now();
    let fed = false;
    while (Date.now() - t0 < 90000) {
      await new Promise((r) => setTimeout(r, 100));
      sim.pollUart();
      if (!fed && log().includes('RX_INSTALL=')) {
        fed = true;
        for (let i = 0; i < 512; i++) await sim.feedI2SRX(0x1000 + i);
      }
      if (log().includes('ALL TESTS PASSED')) return;
    }
  } else if (kind === 'buttons-press') {
    // buttons: RESET tap reboots to BOOT #2, BOOT hold + RESET to #3,
    // release + RESET to #4 (dev-board EN + GPIO0 behavior).
    const t0 = Date.now();
    const waitFor = async (re, ms = 30000) => {
      const s = Date.now();
      while (Date.now() - s < ms) {
        await new Promise((r) => setTimeout(r, 200));
        sim.pollUart();
        if (re.test(log())) return true;
      }
      return false;
    };
    if (!await waitFor(/BOOT #1/)) return;
    await waitFor(/READY/);
    await sim.pressResetButton();
    if (!await waitFor(/BOOT #2/)) return;
    await sim.pressBootButton(true);
    await sim.pressResetButton();
    if (!await waitFor(/BOOT #3/)) return;
    await sim.pressBootButton(false);
    await sim.pressResetButton();
    await waitFor(/BOOT #4/);
  }
}
