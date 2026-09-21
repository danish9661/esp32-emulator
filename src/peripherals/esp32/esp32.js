// ESP32 chip class — extracted from index.js

const GpioBothDir = 3;

import { ESPTrace } from "../common/trace.js";
import { ChipRootClock } from "../common/clocks.js";
import {
  Memory,
  ReadonlyMemory,
  InvalidMemory,
  MemoryTranslator,
  ReverseMemory,
  MMUMemory,
  PageTable,
  MMIOHandlerRegistry,
  PTE_TYPE_RAM,
  PTE_TYPE_MMIO,
  PTE_TYPE_FLASH,
} from "../common/memory.js";


import { UartController } from "../common/uart.js";
import { XtsEncryptionState } from "../common/xts-state.js";
import {
  applyPeripheralResetValues, applySingleResetValues, Drom0Size, RegionDrom0Base,
  RegionDrom1Base, GpioBaseAddrAlt, Iram0Size, RegionPeriBusBase,
  RegionUsbBase, RegionDrom0MapBase, RegionIrom0BaseAlt,
  UartRegisterMap, UartFieldMap,
  randomUint32, XtensaRegisterTable, ClockTree,
  RegionIram1Base, RegionPeri1Base, RegionDromSize,
  RegionCacheAlignSize, RegionPeri2Base, RegionCacheLineSize,
  RegionDram1Base, RegionRtcSlowBase, RegionIrom0Base,
  Iram1Size, RegionIram0Base, Esp32FullResetValues, MmuPageTableConfig,
  UsbOtgBaseAddr, RegionCodeBase, RegionFlashCacheBase,
  RegionIram1BaseAlt, RtcSlowSize, Drom0CacheSize, UhciBaseAddr,
  UhciAltBaseAddr, SdmmcAltBaseAddr,
} from "../common/index.js";
import { writePartitionTable, parseMacAddress, parseFirmwareOffset } from "../common/partition-table.js";
import { applyBoardPreset } from "../../boards/esp32-cam.js";
import { XtensaCore } from "../../engine/esp-xtensa/xtensa-core.js";
import { WasmLoader } from "../../engine/esp-xtensa/wasm-loader.js";
import * as WM from "../../engine/wasm-memory-layout.js";

// Interrupt enum (local copy, matches factory)
const InterruptEnum = {
  MAC_INTR: 0, MAC_NMI: 1, BB_INT: 2, BT_MAC_INT: 3, BT_BB_INT: 4, BT_BB_NMI: 5,
  RWBT_IRQ: 6, RWBLE_IRQ: 7, RWBT_NMI: 8, RWBLE_NMI: 9,
  SLC0_INTR: 10, SLC1_INTR: 11, UHCI0_INTR: 12, UHCI1_INTR: 13,
  TG_T0_LEVEL_INT: 14, TG_T1_LEVEL_INT: 15, TG_WDT_LEVEL_INT: 16, TG_LACT_LEVEL_INT: 17,
  TG1_T0_LEVEL_INT: 18, TG1_T1_LEVEL_INT: 19, TG1_WDT_LEVEL_INT: 20, TG1_LACT_LEVEL_INT: 21,
  GPIO_INTERRUPT_PRO: 22, GPIO_INTERRUPT_PRO_NMI: 23,
  CPU_INTR_FROM_CPU_0: 24, CPU_INTR_FROM_CPU_1: 25, CPU_INTR_FROM_CPU_2: 26, CPU_INTR_FROM_CPU_3: 27,
  SPI_INTR_0: 28, SPI_INTR_1: 29, SPI_INTR_2: 30, SPI_INTR_3: 31,
  I2S0_INT: 32, I2S1_INT: 33, UART_INTR: 34, UART1_INTR: 35, UART2_INTR: 36,
  SDIO_HOST_INTERRUPT: 37, EMAC_INT: 38, PWM0_INTR: 39, PWM1_INTR: 40,
  PWM2_INTR: 41, PWM3_INTR: 42, LEDC_INT: 43, EFUSE_INT: 44, CAN_INT: 45,
  RTC_CORE_INTR: 46, RMT_INTR: 47, PCNT_INTR: 48,
  I2C_EXT0_INTR: 49, I2C_EXT1_INTR: 50, RSA_INTR: 51,
  SPI1_DMA_INT: 52, SPI2_DMA_INT: 53, SPI3_DMA_INT: 54, WDG_INT: 55,
  TIMER_INT1: 56, TIMER_INT2: 57,
  TG_T0_EDGE_INT: 58, TG_T1_EDGE_INT: 59, TG_WDT_EDGE_INT: 60, TG_LACT_EDGE_INT: 61,
  TG1_T0_EDGE_INT: 62, TG1_T1_EDGE_INT: 63, TG1_WDT_EDGE_INT: 64, TG1_LACT_EDGE_INT: 65,
  MMU_IA_INT: 66, MPU_IA_INT: 67, CACHE_IA_INT: 68, MAX_INT: 69,
};

let _debugLog = false;

function _parseSizeMB(cfg, key, alt, def) {
  const v = cfg[key] !== undefined ? parseInt(cfg[key], 10) : cfg[alt];
  return v !== undefined && !isNaN(v) ? v : def;
}

class ESP32 {
  constructor(config = {}) {
    config = applyBoardPreset(config);
    ((this.config = config),
      (this.chipName = "esp32"),
      (this.chipId = 0),
      // BT shim opt-in: the BT task/queue intervention (bt_shim_step hook
      // scan) costs ~3s/step on non-BT firmware, so wasm-loader disables it
      // unless the firmware actually uses BT/BLE. BT tests opt in via
      // config.bleShim=true (forwarded through worker init config).
      (this.bleShimEnabled = config.bleShim === true),
      (this.board = config.board || "esp32"),
      (this._flashSizeMB = _parseSizeMB(config, "flashSize", "flashSizeMB", 4)),
      (this._psramSizeMB = _parseSizeMB(config, "psramSize", "psramSizeMB", 4)),
      (this._psramType = config.psramType || "quad"),
      (this._firmwareOffset = parseFirmwareOffset(config.firmwareOffset)),
      (this._macAddress = parseMacAddress(config.macAddress)),
      (this.wifiMacState = new Uint8Array(new SharedArrayBuffer(4096))),
      (this.flash = new Uint8Array(
        new SharedArrayBuffer(this._flashSizeMB * Drom0Size),
      )),
      (this.chipROM = new ReadonlyMemory(
        new Uint8Array(new SharedArrayBuffer(Iram0Size)),
        RegionCodeBase,
      )),
      (this.rom1 = this.chipROM.createView(RegionDram1Base, 393216, 65536)),
      (this.iram = new Memory(
        new Uint8Array(new SharedArrayBuffer(RegionIrom0Base - RegionFlashCacheBase)),
        RegionFlashCacheBase,
      )),
      (this.psram = new Uint8Array(
        new SharedArrayBuffer(this._psramSizeMB * Drom0Size),
      )),
      (this.psramMemory = new MMUMemory(this.psram, RegionDrom0Base)),
      (this.dataMem = new Memory(
        new Uint8Array(new SharedArrayBuffer(Iram1Size + Drom0CacheSize)),
        RegionRtcSlowBase,
      )),
      (this.sram1Reverse = new ReverseMemory(
        this.dataMem.data.subarray(Iram1Size),
        RegionIrom0BaseAlt,
      )),
      (this.rtcFastMem = new Memory(
        new Uint8Array(new SharedArrayBuffer(RegionIram1BaseAlt - RegionIram0Base)),
        RegionIram0Base,
      )),
      (this.rtcSlowMem = new Memory(
        new Uint8Array(new SharedArrayBuffer(RtcSlowSize)),
        UsbOtgBaseAddr,
      )),
      (this.invalidMem = new InvalidMemory()),
      (this.flashMMUMap = new Map()),
      (this.mmuTableMemory = new Memory(
        new Uint8Array(new SharedArrayBuffer(2 * RegionDromSize)),
        RegionPeri1Base,
      )),
      (this.mmuTablePro = new Uint32Array(
        this.mmuTableMemory.data.buffer,
        0,
        RegionCacheLineSize,
      )),
      (this.mmuTableApp = new Uint32Array(
        this.mmuTableMemory.data.buffer,
        RegionDromSize,
      )),
      (this.dmaBase = 0x3ff00000),
      (this.peripheralMap = {}),
      (this.gpio = (() => {
        const inputValues = [0, 0];
        const pins = Array.from({ length: 40 }, (_, pinNum) => {
          const bank = pinNum < 32 ? 0 : 1;
          const idx = pinNum % 32;
          return {
            get inputValue() {
              return !!(inputValues[bank] & (1 << idx));
            },
            set inputValue(newLevel) {
              const bit = 1 << idx;
              if (!!(inputValues[bank] & bit) === !!newLevel) return;
              newLevel ? (inputValues[bank] |= bit) : (inputValues[bank] &= ~bit);
            },
          };
        });
        return {
          strapValue: 19,
          inputValues,
          pins,
          zeroMemory: () => {},
          reset: () => {},
        };
      })()),
      (this.resetReason = 1),
      (this.onReset = () => true),
      (this.onAnalogRead = (pin, cfg) => {
        const v = this.analogVolts?.[pin];
        if (v === undefined) return 0;
        const att = Math.max(0, Math.min(3, (cfg ?? 12) - 9));
        const fullScale = [1.1, 1.34, 2.0, 3.3][att];
        return Math.max(0, Math.min(4095, Math.round((Math.min(v, fullScale) / fullScale) * 4095)));
      }),
      (this.analogVolts = { ...(config.analogInputs || {}) }),
      (this.setAnalogInput = (pin, volts) => { this.analogVolts[pin] = volts; }),
      (this.vddMv = config.vddMv ?? 3300),
      // Touch pad readings (pad 0-9 -> raw count; untouched ~1000+, touched
      // reads low). config.touchInputs overrides per pad; default 1000.
      (this.touchCounts = { ...(config.touchInputs || {}) }),
      (this.setTouchInput = (pad, count) => { this.touchCounts[pad] = count; }),
      (this.onTouchRead = (pad) => {
        const v = this.touchCounts?.[pad];
        return v === undefined ? 1000 : (v >>> 0);
      }),
      // DAC output stage: dacWrite() in firmware lands here via the native
      // SENS DAC_CTRL2 handler (virtual wire DAC ch0/ch1 -> GPIO25/26, which
      // are ADC2 ch8/ch9, so analogRead(25/26) observes the DAC voltage).
      (this.onDacWrite = (channel, value) => {
        const pin = channel ? 26 : 25;
        this.analogVolts[pin] = ((value & 0xFF) / 255) * 3.3;
      }),
      // Virtual SD card image (host-owned block storage for the native SDMMC
      // virtual card). config.sdCard: { sizeMB?: number, blocks?: number }.
      // Default 16MB (32768 x 512B) zero-filled; survives chip.reset() like
      // flash (reset only returns the virtual card to idle, storage kept).
      // SAB-backed so the host can save/reload the image between runs
      // (shared via getMemorySABs as "sdcard", like flash).
      (this.sdData = (() => {
        const opt = config.sdCard === true ? {} : (config.sdCard || null);
        if (!opt && config.sdCard !== undefined && config.sdCard !== true) return null;
        const blocks = opt?.blocks ?? Math.max(1024, Math.round(((opt?.sizeMB ?? 16) * 1024 * 1024) / 512));
        const arr = new Uint8Array(new SharedArrayBuffer(blocks * 512));
        if (opt?.image instanceof Uint8Array) arr.set(opt.image.subarray(0, arr.length));
        return arr;
      })()),
      (this.setSdPresent = (present) => { this.sdPresent = !!present; }),
      (this.sdPresent = config.sdCard === false ? false : true),
      // Dev-board buttons (real-device behavior):
      // - RESET (EN pin): momentary reset — pressResetButton() runs a full
      //   chip.reset() (same as the EN pin pulling CHIP_PU low: digital +
      //   RTC state cleared, flash/RTC-memory preserved like real HW).
      // - BOOT (GPIO0 strapping): pressBootButton(held) drives GPIO0's input
      //   level + the BOOT strap bit (bit4 of the 0x13 strap value) so a
      //   subsequent reset samples download mode exactly like holding BOOT
      //   on a real dev board. holdBootAndReset() = BOOT held + RESET tap.
      (this.pressResetButton = () => { this.reset(); }),
      (this.pressBootButton = (held) => {
        const level = held ? 1 : 0;
        // Drive GPIO0's live input level (native matrix + JS holder, same
        // path as the setPinInput host API) AND the sampled strap bit.
        // GPIO0 is strap bit4 (0x10 inside the 0x13 default): holding BOOT
        // (GPIO0 LOW on real HW... here held=true asserts the strap bit)
        // makes the next reset sample download mode like a real dev board.
        try { this._wasmLoader?.exports?.native_gpio_set_pin_input?.(0, level); } catch {}
        const holder = this.gpio?.pins?.[0];
        if (holder) { try { holder.inputValue = level ? 1 : 0; } catch {} }
        if (this.gpio) {
          this.gpio.strapValue = held
            ? (this.gpio.strapValue | 0x10)
            : (this.gpio.strapValue & ~0x10);
          try { this._wasmLoader?.exports?.native_gpio_set_strap?.(this.gpio.strapValue >>> 0); } catch {}
        }
      }),
      (this.holdBootAndReset = () => { this.pressBootButton(true); this.reset(); }),
      (this.onRandomRead = randomUint32),
      (this._nativeFrequency = 16e7),
      (this.clocks = new ClockTree(new ChipRootClock(this))),
      (this.cycles = 0),
      (this.xts = new XtsEncryptionState()),
      (this.cores = [
        new XtensaCore(this, 0, "PRO_CPU", 52685, XtensaRegisterTable),
        new XtensaCore(this, 1, "APP_CPU", 43947, XtensaRegisterTable),
      ]),
      (this.trace = new ESPTrace()),
      (this.gdbTargetXml = ""),
      (this.stopped = true),
      (this.writeWatchPoints = new Set()),
      (this.lastMappedAddress = 0),
      this.flash.fill(255),
      config.partitions && writePartitionTable(this.flash, config.partitions));
    const uartConfig = {
        hasTXState: true,
        toutMultiply: true,
        RmtChannelRegister: UartRegisterMap,
        F: UartFieldMap,
      };
    (this.uart = [
      new UartController(
        this,
        SdmmcAltBaseAddr,
        "UART0",
        0,
        InterruptEnum.UART_INTR,
        uartConfig,
      ),
      new UartController(
        this,
        UhciBaseAddr,
        "UART1",
        1,
        InterruptEnum.UART1_INTR,
        uartConfig,
      ),
      new UartController(
        this,
        UhciAltBaseAddr,
        "UART2",
        2,
        InterruptEnum.UART2_INTR,
        uartConfig,
      ),
    ]);
    for (const addr of ((this.peripherals = [
      this.gpio,
      ...this.uart,
    ]),
    this.peripherals)) {
      this.peripheralMap[addr.baseAddr] = addr;
      const value = addr.baseAddr + RegionDrom0MapBase;
      value >= RegionPeriBusBase &&
        value < RegionUsbBase &&
        (this.peripheralMap[value] = new MemoryTranslator(
          addr,
          RegionDrom0MapBase,
        ));
    }
    for (const {
      start: addr,
      pages: value,
      index: regIdx,
    } of MmuPageTableConfig)
      for (let i = 0; i < value; i++)
        this.flashMMUMap.set(
          (addr >>> RegionCacheAlignSize) + i,
          regIdx + i,
        );
    this.pageTable = new PageTable();
    this.mmioHandlers = new MMIOHandlerRegistry();
    this.buildPageTable();
    {
      const callbacks = {
        traceEntry: (i, pc, op) => this.trace.traceEntry(i, pc, op),
        traceReturn: (i, v, t, idx) => this.trace.traceReturn(i, v, t, idx),
        traceMemWrite: (i, pc, a, v, s) => this.trace.traceMemWrite(i, pc, a, v, s),
        writeWatchpoint: (a, c) => { this.writeWatchPoints.has(a) && this.onBreak?.(c); },
        onBreak: (c) => this.onBreak?.(c),
        onUnknownInst: (i, pc, op) => this.onUnknownInstruction?.(i, pc, op),
        getCpuTicks: () => this.clocks.cpu.ticks,
        getCpuCycles: () => this.cycles,
        getSimFreq: () => 160e6,
        getCpuFreq: () => this.clocks.cpu.frequency,
        getClockNanos: () => (this.cycles / 160e6) * 1e9,
        isValidCodeAddress: (a) => this.isValidCodeAddress?.(a),
      };
      this._coreCallbacks = callbacks;
      for (const core of this.cores) core.attachMemorySystem(this.pageTable, this.mmioHandlers, this._memRegions, callbacks);
    }
    this.reset();
    if (this._macAddress && this.wifiMacState) {
      const mac = this._macAddress;
      this.wifiMacState[64] = mac[0];
      this.wifiMacState[65] = mac[1];
      this.wifiMacState[66] = mac[2];
      this.wifiMacState[67] = mac[3];
      this.wifiMacState[68] = mac[4];
      this.wifiMacState[69] = mac[5];
    }
  }
  loadROM(addr) {
    if (!(addr instanceof Uint8Array) || addr.length === 0) {
      throw new Error(
        `Invalid boot ROM: expected a non-empty Uint8Array (the ESP32 boot ROM, ` +
        `e.g. esp32-v3-rom.bin). Got ${addr === null ? 'null' : (addr && addr.length !== undefined ? 'an empty array' : typeof addr)}.\n` +
        `Provide it via config.bootrom or chip.loadROM(romBytes).`
      );
    }
    this.chipROM.set(addr, RegionCodeBase);
  }

  // Create shared WASM memory and remap JS regions into it
  // All RAM/ROM regions become subarray views of one SAB that WASM imports as linear memory
  _initWasmMemory() {
    const regions = this._memRegions;
    const offsets = [];
    let cum = 0;
    for (const r of regions) {
      offsets.push(cum);
      cum += r.data.length;
    }
    this._memRegionOffsets = offsets;

    const ramBytes = cum;
    // Flash mirror for the Rust flash fast-path (PTE_TYPE_FLASH)
    const flashBytes = this.flash?.length ?? 0;
    const neededBytes = Math.max(WM.RAM_DATA_OFFSET + ramBytes, WM.FLASH_DATA_OFFSET + flashBytes);
    const pages = Math.ceil(neededBytes / 65536);

    this._wasmMemoryObj = new WebAssembly.Memory({ initial: pages, maximum: pages, shared: true });
    this._wasmMemory = this._wasmMemoryObj.buffer;

    const dst = new Uint8Array(this._wasmMemory);
    for (let i = 0; i < regions.length; i++) {
      const off = WM.RAM_DATA_OFFSET + offsets[i];
      dst.set(regions[i].data, off);
      regions[i].data = new Uint8Array(this._wasmMemory, off, regions[i].data.length);
    }
    if (flashBytes) {
      dst.set(this.flash, WM.FLASH_DATA_OFFSET);
      this._flashMirror = new Uint8Array(this._wasmMemory, WM.FLASH_DATA_OFFSET, flashBytes);
    }

    // Region table
    const u32 = new Uint32Array(this._wasmMemory);
    const rtBase = WM.REGION_TABLE_OFFSET >>> 2;
    for (let i = 0; i < regions.length; i++) {
      u32[rtBase + i * 2] = regions[i].baseAddr;
      u32[rtBase + i * 2 + 1] = offsets[i];
    }

    // Page table — replace JS table with direct view into the shared SAB
    const oldPt = this.pageTable.table;
    this.pageTable.table = new Uint32Array(this._wasmMemory, WM.PAGE_TABLE_OFFSET, WM.PAGE_ENTRIES * 2);
    this.pageTable.table.set(oldPt);

    // Recreate derived views that depended on old buffer objects
    this.rom1 = this.chipROM.createView(RegionDram1Base, 393216, 65536);
    this.sram1Reverse = new ReverseMemory(this.dataMem.data.subarray(Iram1Size), RegionIrom0BaseAlt);
    this.mmuTablePro = new Uint32Array(this.mmuTableMemory.data.buffer, this.mmuTableMemory.data.byteOffset, RegionCacheLineSize);
    this.mmuTableApp = new Uint32Array(this.mmuTableMemory.data.buffer, this.mmuTableMemory.data.byteOffset + RegionDromSize, RegionCacheLineSize);
  }

  // Load WASM engine. Memory is shared via SAB.
  async loadWasm(wasmBytes, enabled = 'wasm') {
    try {
      if (!this._wasmMemory) this._initWasmMemory();
      const loader = new WasmLoader();
      await loader.load(wasmBytes, this, this._wasmMemoryObj);
      this._wasmCores = loader.createCores(this.cores.length);
      // Redirect JS core's specialRegisters & key properties to WASM core's SAB-backed state
      // so that peripheral callbacks (e.g. interrupt matrix, DPORT reset) update the correct memory
      for (let i = 0; i < this.cores.length; i++) {
        this.cores[i].specialRegisters = this._wasmCores[i]._specRegs;
        this.cores[i].physicalRegisters = this._wasmCores[i]._physRegs;
        Object.defineProperty(this.cores[i], 'pendingInterrupts', {
          get: () => this._wasmCores[i].pendingInterrupts,
          set: (v) => { this._wasmCores[i]._pendingIntView[0] = v ? 1 : 0; },
          configurable: true,
        });
        Object.defineProperty(this.cores[i], 'enabled', {
          get: () => this._wasmCores[i].enabled,
          set: (v) => { this._wasmCores[i]._enabledView[0] = v ? 1 : 0; },
          configurable: true,
        });
        Object.defineProperty(this.cores[i], 'idle', {
          get: () => this._wasmCores[i].idle,
          set: (v) => { this._wasmCores[i]._idleView[0] = v ? 1 : 0; },
          configurable: true,
        });
        Object.defineProperty(this.cores[i], 'PC', {
          get: () => this._wasmCores[i]._pcView[0],
          set: (v) => { this._wasmCores[i]._pcView[0] = v >>> 0; },
          configurable: true,
        });
        Object.defineProperty(this.cores[i], 'nextPC', {
          get: () => this._wasmCores[i]._nextPcView[0],
          set: (v) => { this._wasmCores[i]._nextPcView[0] = v >>> 0; },
          configurable: true,
        });
        Object.defineProperty(this.cores[i], 'lightSleep', {
          get: () => this._wasmCores[i]._lightSleepView[0] !== 0,
          set: (v) => { this._wasmCores[i]._lightSleepView[0] = v ? 1 : 0; },
          configurable: true,
        });
        Object.defineProperty(this.cores[i], 'opcodeSegment', {
          get: () => this._wasmCores[i]._opcodeSegmentView[0],
          set: (v) => { this._wasmCores[i]._opcodeSegmentView[0] = v >>> 0; },
          configurable: true,
        });
      }
      this._wasmLoader = loader;
      this._nativeUartReset = () => { try { loader.exports?.native_uart_reset?.(); } catch {} };
      this.setVddMv = (mv) => { this.vddMv = mv >>> 0; try { loader.exports?.native_bod_set_voltage_mv?.(this.vddMv); } catch {} };
      this._nativeI2cReset = () => { try { loader.exports?.native_i2c_reset?.(); } catch {} };
      this._nativeSpiReset = () => { try { loader.exports?.native_spi_reset?.(); } catch {} };
      this._nativeGpioReset = () => { try { loader.exports?.native_gpio_reset?.(); } catch {} };
      this._nativeFrcReset = () => { try { loader.exports?.native_frc_timer_reset?.(); } catch {} };
      this._nativeTimg1Reset = () => { try { loader.exports?.native_timg1_reset?.(); } catch {} };
      this._nativeEfuseReset = () => { try { loader.exports?.native_efuse_reset?.(); } catch {} };
      this._nativeSysconReset = () => { try { loader.exports?.native_syscon_reset?.(); } catch {} };
      this._nativeTwaiReset = () => { try { loader.exports?.native_twai_reset?.(); } catch {} };
      this._nativeRsaReset = () => { try { loader.exports?.native_rsa_reset?.(); } catch {} };
      this._nativeRtcReset = () => { try { loader.exports?.native_rtc_reset?.(); } catch {} };
      this._nativeLedcReset = () => { try { loader.exports?.native_ledc_reset?.(); } catch {} };
      this._nativePcntReset = () => { try { loader.exports?.native_pcnt_reset?.(); } catch {} };
      this._nativeRmtReset = () => { try { loader.exports?.native_rmt_reset?.(); } catch {} };
      this._nativeI2sReset = () => { try { loader.exports?.native_i2s_reset?.(); } catch {} };
      this._nativeSdmmcReset = () => { try { loader.exports?.native_sdmmc_reset?.(); } catch {} };
      this._nativeWifiAnalogReset = () => { try { loader.exports?.native_wifi_analog_reset?.(); } catch {} };
      this._nativeWifiMacReset = () => { try { loader.exports?.native_wifi_mac_reset?.(); } catch {} };
      this._nativeDportReset = () => { try { loader.exports?.native_dport_reset?.(); } catch {} };
      this._nativeSdioSlaveReset = () => { try { loader.exports?.native_sdio_slave_reset?.(); } catch {} };
      this._nativeFeReset = () => { try { loader.exports?.native_fe_reset?.(); } catch {} };
      this._nativeMcpwmReset = () => { try { loader.exports?.native_mcpwm_reset?.(); } catch {} };
      this._nativeUhciReset = () => { try { loader.exports?.native_uhci_reset?.(); } catch {} };
      this._nativeEmacReset = () => { try { loader.exports?.native_emac_reset?.(); } catch {} };
      this._nativeSweepReset = () => { try { loader.exports?.native_sweep_reset?.(); } catch {} };
      this._nativeBtRfReset = () => { try { loader.exports?.native_bt_rf_reset?.(); } catch {} };
      console.log(`WASM engine loaded (${this._wasmCores.length} cores). Mode: ${enabled}`);
      return true;
    } catch (err) {
      throw new Error(
        `Failed to load the WASM engine. Underlying error: ${err.message}\n` +
        `Check that esp_engine_wasm.wasm is present at src/engine/esp-xtensa/ and is a ` +
        `valid WebAssembly module built for wasm32-unknown-unknown. Rebuild with:\n` +
        `  cd engine-wasm && cargo build --release --target wasm32-unknown-unknown && ` +
        `cp target/wasm32-unknown-unknown/release/esp_engine_wasm.wasm ../src/engine/esp-xtensa/`
      );
    }
  }

  reset() {
    // Invalidate cached step exports (WASM may be reloaded).
    this._stepExp = undefined;
    for (let e of this.cores) e.reset();
    if (this._wasmCores) { for (let wc of this._wasmCores) wc.reset(); }
    for (let e of ((this.cores[1].enabled =
      (this._wasmLoader?.exports?.native_dport_get_core1_enabled?.() ?? 1) !==
      0),
    applyPeripheralResetValues(this, Esp32FullResetValues),
    this.cores[0].writeUint32(0x3ff49000, 1023),
    this.mmuTablePro.fill(256),
    this.mmuTableApp.fill(256),
    this.peripherals))
      e.reset();
    if (this._nativeUartReset) this._nativeUartReset();
    if (this._nativeI2cReset) this._nativeI2cReset();
    if (this._nativeSpiReset) this._nativeSpiReset();
    if (this._nativeGpioReset) this._nativeGpioReset();
    if (this._nativeFrcReset) this._nativeFrcReset();
    if (this._nativeTimg1Reset) this._nativeTimg1Reset();
    if (this._nativeEfuseReset) this._nativeEfuseReset();
    if (this._nativeSysconReset) this._nativeSysconReset();
    if (this._nativeTwaiReset) this._nativeTwaiReset();
    if (this._nativeRsaReset) this._nativeRsaReset();
    if (this._nativeRtcReset) this._nativeRtcReset();
    if (this._nativeLedcReset) this._nativeLedcReset();
    if (this._nativePcntReset) this._nativePcntReset();
    if (this._nativeRmtReset) this._nativeRmtReset();
    if (this._nativeI2sReset) this._nativeI2sReset();
    if (this._nativeSdmmcReset) this._nativeSdmmcReset();
    if (this._nativeWifiAnalogReset) this._nativeWifiAnalogReset();
    if (this._nativeWifiMacReset) this._nativeWifiMacReset();
    if (this._nativeDportReset) this._nativeDportReset();
    if (this._nativeSdioSlaveReset) this._nativeSdioSlaveReset();
    if (this._nativeFeReset) this._nativeFeReset();
    if (this._nativeMcpwmReset) this._nativeMcpwmReset();
    if (this._nativeUhciReset) this._nativeUhciReset();
    if (this._nativeEmacReset) this._nativeEmacReset();
    if (this._nativeSweepReset) this._nativeSweepReset();
    if (this._nativeBtRfReset) this._nativeBtRfReset();
  }
  resetPeripheral(addr, value) {
    if (!value) return;
    let regIdx = this.peripherals.find((value) => value.baseAddr === addr);
    regIdx
      ? (applySingleResetValues(regIdx, Esp32FullResetValues), regIdx.reset())
      : console.error("Peripheral to reset not found", addr.toString(16));
  }
  mapAddress(addr, value) {
    if (
      ((this.lastMappedAddress = addr),
      addr >= 0x3ffac000 &&
        addr <= 0x3ffadfff &&
        console.error("SRAM 2 mmu access:", addr.toString(16)),
      (addr >= GpioBaseAddrAlt && addr < RegionDram1Base) ||
        (addr >= RegionPeriBusBase && addr <= RegionUsbBase))
    ) {
      let value = this.peripheralMap[0xfffffc00 & addr],
        regIdx = this.peripheralMap[0xfffff000 & addr];
      if (value || regIdx) return value || regIdx;
    }
    if (addr >= RegionRtcSlowBase && addr < RegionCodeBase)
      return this.dataMem;
    if (addr >= RegionDram1Base && addr < RegionRtcSlowBase)
      return this.rom1;
    if (addr >= RegionCodeBase && addr < RegionCodeBase + Iram0Size)
      return this.chipROM;
    if (addr >= RegionFlashCacheBase && addr < RegionIrom0Base)
      return this.iram;
    if (addr >= RegionIram0Base && addr < RegionIram1BaseAlt)
      return this.rtcFastMem;
    if (addr >= RegionDrom0Base && addr < RegionDrom1Base) {
      let addr = value ? this.mmuTableApp : this.mmuTablePro;
      return (this.psramMemory.setTable(addr, 1152), this.psramMemory);
    }
    if (addr >= RegionIrom0BaseAlt && addr <= RegionIram1Base)
      return this.sram1Reverse;
    let regIdx = this.flashMMUMap.get(addr >>> RegionCacheAlignSize);
    if (null != regIdx) {
      let phyAddr = (value ? this.mmuTableApp : this.mmuTablePro)[regIdx];
      if (256 & phyAddr) return this.invalidMem;
      let phyAddrAligned = phyAddr << RegionCacheAlignSize;
      return new ReadonlyMemory(
        this.flash.subarray(phyAddrAligned),
        (addr >>> RegionCacheAlignSize) << RegionCacheAlignSize,
      );
    }
    return addr >= RegionPeri1Base && addr < RegionPeri2Base
      ? this.mmuTableMemory
      : addr >= UsbOtgBaseAddr && addr < UsbOtgBaseAddr + RtcSlowSize
        ? this.rtcSlowMem
        : (_debugLog && console.log("Invalid memory access", addr.toString(16)),
          this.invalidMem);
  }
  step() {
    if (this._wasmCores) {
      // Cache exports reference to avoid repeated optional-chain per step.
      let exp = this._stepExp;
      if (exp === undefined) {
        exp = this._wasmLoader?.exports || null;
        this._stepExp = exp;
      }
      if (exp?.core_run) {
        exp.core_run(512);
        this.cycles += 512;
      } else {
        this._wasmCores[0].runInstruction();
        if (this.cores[1]?.enabled) this._wasmCores[1].runInstruction();
        this.cycles++;
      }
    } else {
      this.cycles++;
    }
    // Write CPU tick/cycle counts to SAB for Rust to read (avoids FFI per step).
    // Stored in core0's _pad[0] (offset 3992) and _pad[1] (offset 3996).
    if (this._wasmCores && !this._sabU32) {
      this._sabU32 = new Uint32Array(this._wasmMemory || this._wasmMemoryObj?.buffer);
    }
    if (this._sabU32) {
      this._sabU32[998] = this.clocks.cpu.ticks >>> 0;
      this._sabU32[999] = this.cycles >>> 0;
    }
  }
  // MMU-table entry governing `addr` (or undefined when not MMU-routed).
  // Used by wasm-loader's map_read/map_write page cache to detect remaps —
  // mirrors the flashMMUMap + psramMemory branches of mapAddress().
  mmuEntryFor(addr, value) {
    if (addr >= RegionDrom0Base && addr < RegionDrom1Base) {
      return (value ? this.mmuTableApp : this.mmuTablePro)[1152 + ((addr - RegionDrom0Base) >>> 15)];
    }
    let regIdx = this.flashMMUMap.get(addr >>> RegionCacheAlignSize);
    if (regIdx === undefined) return undefined;
    return (value ? this.mmuTableApp : this.mmuTablePro)[regIdx];
  }
  get coresIdle() {
    let { cores: addr } = this;
    return (
      addr[0].idle &&
      !addr[0].pendingInterrupts &&
      addr[1].idle &&
      !addr[1].pendingInterrupts
    );
  }
  interrupt(addr, value = true, regIdx = GpioBothDir) {
    // The interrupt matrix runs fully in Rust (native_interrupt →
    // Rust InterruptMatrixPeripheral → core int_set_clear). No JS matrix.
    this._wasmLoader?.exports?.native_interrupt?.(addr >>> 0, value ? 1 : 0, regIdx >>> 0);
  }
  buildPageTable() {
    const pt = this.pageTable;
    const reg = this.mmioHandlers;

    // Find pages shared by aligned + non-aligned peripherals — skip these entirely
    const sharedPages = new Set();
    const pagePerifs = {};
    for (const p of this.peripherals) {
      const page = p.baseAddr >>> 12;
      if (!pagePerifs[page]) pagePerifs[page] = [];
      pagePerifs[page].push(p);
    }
    for (const [page, perifs] of Object.entries(pagePerifs)) {
      const hasAligned = perifs.some(p => (p.baseAddr & 0xFFF) === 0);
      const hasNonAligned = perifs.some(p => (p.baseAddr & 0xFFF) !== 0);
      if (hasAligned && hasNonAligned && perifs.length > 1) sharedPages.add(parseInt(page));
    }

    for (const p of this.peripherals) {
      // Skip non-page-aligned peripherals — handled by mapAddress fallback
      if ((p.baseAddr & 0xFFF) !== 0) continue;
      // Skip pages shared with non-aligned peripherals — mapAddress handles routing
      if (sharedPages.has(p.baseAddr >>> 12)) continue;
      const id = reg.register(p);
      pt.setRange(p.baseAddr, 0x1000, PTE_TYPE_MMIO, id);
      // Aliased peripheral mappings — use MemoryTranslator for address delta
      const alias = p.baseAddr + RegionDrom0MapBase;
      if (alias >= RegionPeriBusBase && alias < RegionUsbBase) {
        const aliasId = reg.register(new MemoryTranslator(p, RegionDrom0MapBase));
        pt.setRange(alias, 0x1000, PTE_TYPE_MMIO, aliasId);
      }
    }

    // Fixed RAM/ROM regions — store Memory object for Phase 3 CPU lookup
    // iram (0x40070000-0x4009FFFF) is a static buffer (not MMU-routed), so it
    // is safe to expose as PTE_RAM to the Rust engine.
    this._memRegions = [
      this.dataMem,
      this.chipROM,
      this.rtcFastMem,
      this.rtcSlowMem,
      this.rom1,
      this.mmuTableMemory,
      this.iram,
    ];
    for (let i = 0; i < this._memRegions.length; i++) {
      const m = this._memRegions[i];
      pt.setRange(m.baseAddr, m.data.length, PTE_TYPE_RAM, i);
    }

    // Flash-cache windows → PTE_TYPE_FLASH with the static MMU-table index in
    // `data`. The Rust engine reads the MMU table directly from linear memory
    // each access (so runtime remaps are picked up) and resolves the flash
    // byte from the linear-memory flash mirror. DROM0 (0x3F800000, psram-
    // backed) stays on the JS fallback path (never touched by firmware —
    // measured 0 reads over the full peripheral suite).
    for (const [page16, regIdx] of this.flashMMUMap) {
      // DROM low window (0x3F400000-0x3F800000, table idx 0-63, keys 0x3F40-0x3F7F)
      // + flash-cache windows 0x400C..0x4040 native. 0x4040+ stays isolated
      // (entries 128+ exceed a 4MB flash mirror).
      if ((page16 < 0x3f40 || page16 >= 0x3f80) && (page16 < 0x400c || page16 >= 0x4040)) continue;
      for (let i = 0; i < 16; i++) {
        pt.setPage((page16 << 16) + i * 0x1000, PTE_TYPE_FLASH, regIdx);
      }
    }
    console.log(`[PTEDBG] flashMMUMap.size=${this.flashMMUMap.size} pte0x400C2000=${pt.getType(0x400C2000)} pte0x40080000=${pt.getType(0x40080000)} pte0x3F400000=${pt.getType(0x3F400000)}`);
  }
  get devices() {
    return {
      nonvolatile: { flash: this.flash },
      volatile: {
        iram: this.iram.data,
        dram: this.dataMem.data,
        rtcFast: this.rtcFastMem.data,
        rtcSlow: this.rtcSlowMem.data,
        mmu: this.mmuTableMemory.data,
        psram: this.psram,
      },
    };
  }
}

export { ESP32 };



