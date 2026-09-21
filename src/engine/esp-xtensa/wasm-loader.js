// WASM loader for Rust ESP32 engine — shared memory mode
// WASM imports a SAB as linear memory; JS regions are subarray views of the same SAB.
// No syncing, no copying — both engines read/write the same bytes.

import * as WM from "../wasm-memory-layout.js";
import { ReadonlyMemory } from "../../peripherals/common/memory.js";

// Byte offsets within CoreState #[repr(C)] struct (matches state.rs field order)
// CoreState layout (byte offsets computed from field types):
//   esp32:4 + index:4 + name:4 + processor_id:4 = 16
// + physical_registers[64]:256 -> 272
// + float_registers[16]:64 -> 336
// + special_registers[256]:1024 -> 1360
// + user_registers[237]:948 -> 2308
// + q_registers[128]:128 -> 2436
// + qacc_high[20]:20 -> 2456
// + qacc_low[20]:20 -> 2476
// + pad4 (for u64 align):4 -> 2480
// + accx_lo:8 + accx_hi:8 -> 2496
// + sar_m32:4 + sar_m32_pending:4 -> 2504
// + ua_state[4]:16 -> 2520
// + pie_enabled:4 + last_opcode:4 + enabled:4 + idle:4 + light_sleep:4 + pending_interrupts:4 -> 2544
// + opcode_segment:4 + data_page_idx:4 + data_page_type:4 + data_page_data:4 -> 2560
// + page_table_ptr:4 + mmio_handlers_ptr:4 + mem_regions_ptr:4 -> 2572
// + pc:4 + next_pc:4 + ccompare0_event:4 + ccompare1_event:4 + ccompare2_event:4 + debug_opcode:4 -> 2596
// + inst_count:4 + debug_log:4 + _pad[26]:104 = 2708 total
const CORE_OFF_PHYS_REGS    = 16;    // physical_registers: [u32; 64]
const _CORE_OFF_FLOAT_REGS = 272;   // float_registers: [u32; 16]
const CORE_OFF_SPECIAL_REGS = 336;   // special_registers: [u32; 256]
const CORE_OFF_ENABLED      = 2528;  // enabled: u32
const CORE_OFF_IDLE         = 2532;  // idle: u32
const CORE_OFF_LIGHT_SLEEP  = 2536;  // light_sleep: u32
const CORE_OFF_PENDING_INT  = 2540;  // pending_interrupts: u32
const CORE_OFF_OPCODE_SEG   = 2544;  // opcode_segment: u32
const CORE_OFF_PC           = 2584;  // pc: u32
const CORE_OFF_NEXT_PC      = 2588;  // next_pc: u32

const MEM_FAULT_INFO = 72; // index in special_registers
const NATIVE_HANDLER_FLAG = 0x80000000;

// Handler IDs matching Rust native_mmio.rs
const HID_SHA = 0;
const HID_RNG = 1;
const HID_EFUSE = 2;
const HID_IO_MUX = 3;
const HID_SYSCON = 4;
const HID_GPIO = 5;
const HID_AES = 6;
const HID_FRC_TIMER = 7;
const HID_TIMG0 = 8;
const HID_UART = 9;
const HID_I2C = 10;
const HID_SPI = 11;
const HID_TIMG1 = 12;
const HID_TWAI = 13;
const HID_RSA = 14;
const HID_RTC = 15;
const HID_LEDC = 16;
const HID_PCNT = 17;
const HID_RMT = 18;
const HID_I2S = 19;
const HID_SDMMC = 20;
const HID_WIFI_ANALOG = 22;
const HID_WIFI_MAC = 23;
const HID_DPORT = 24;
const HID_SDIO_SLAVE = 25;
const HID_FE = 26;
const HID_MCPWM = 27;
const HID_UHCI = 28;
const HID_EMAC = 29;
const HID_SWEEP = 30;
const HID_INVALID_MEM = 31;
const HID_STUB_ZERO = 32;
const HID_BT_RF = 33;

export class WasmLoader {
  constructor() {
    /** @type {WebAssembly.Instance|null} */ this.instance = null;
    /** @type {WebAssembly.Memory|null} */ this.memory = null;
    /** @type {object|null} */ this.exports = null;
    /** @type {object|null} */ this.esp32 = null;
    /** @type {object|null} */ this._mmioRegistry = null;
    /** @type {object} */ this._callbacks = {};
    /** @type {Array} */ this._wasmCores = [];
    /** @type {Array} */ this._ccompareEvents = [];
    /** @type {object|null} */ this._efuseCmdClearEvent = null;
    /** @type {object|null} */ this._uartRxTimeoutEvents = null;
    /** @type {object|null} */ this._uartIntCheckEvents = null;
    /** @type {Float64Array} */ this._mmioCount = new Float64Array(64);
    /** @type {Map} */ this._mmioPageCount = new Map();
    /** @type {Map} */ this._mapCache = new Map();
    /** @type {function|null} */ this._i2sTxDataHook = null;
    /** @type {function|null} */ this._wifiMacTx = null;
    /** @type {function|null} */ this._wifiApRxFrame = null;
    /** @type {function|null} */ this._wifiApSendEth = null;
    /** @type {function|null} */ this._wifiApConnected = null;
  }

  async load(wasmBytes, esp32, memoryObj) {
    this.esp32 = esp32;
    this.mmioRegistry = esp32.mmioHandlers;
    this._callbacks = esp32._coreCallbacks || {};
    this.memory = memoryObj;
    this.memoryBuffer = memoryObj.buffer;

    const env = {
      memory: memoryObj,

      mmio_read: (handlerId, addr, size) => {
        if (handlerId & NATIVE_HANDLER_FLAG) return 0;
        this._mmioCount[handlerId >>> 0]++;
        this._mmioPageCount.set((addr >>> 12).toString(16), (this._mmioPageCount.get((addr >>> 12).toString(16)) || 0) + 1);
        const h = this.mmioRegistry?.handlers[handlerId];
        if (!h) return 0;
        try {
          if (size === 1) return h.readUint8?.(addr) ?? 0;
          if (size === 2) return h.readUint16?.(addr) ?? 0;
          return h.readUint32?.(addr) ?? 0;
        } catch (e) {
          console.warn(`mmio_read error: handler=${handlerId} addr=0x${addr.toString(16)} size=${size}: ${e.message}`);
          return 0;
        }
      },

      mmio_write: (handlerId, addr, val, size) => {
        this._mmioCount[handlerId >>> 0]++;
        this._mmioPageCount.set((addr >>> 12).toString(16), (this._mmioPageCount.get((addr >>> 12).toString(16)) || 0) + 1);
        const h = this.mmioRegistry?.handlers[handlerId];
        if (!h) return;
        try {
          if (size === 1) h.writeUint8?.(addr, val);
          else if (size === 2) h.writeUint16?.(addr, val);
          else h.writeUint32?.(addr, val);
        } catch (e) {
          console.warn(`mmio_write error: handler=${handlerId} addr=0x${addr.toString(16)} val=0x${val.toString(16)} size=${size}: ${e.message}`);
        }
        // Debug: log first UART byte written
        if (handlerId === 89 && (addr >= 0x3ff40000 && addr <= 0x3ff40020)) {
          console.log(`[UART] write handler=${handlerId} addr=0x${addr.toString(16)} val=0x${(val>>>0).toString(16)} size=${size}`);
        }
      },

      on_unknown_inst: (coreIdx, pc, opcode) => this._callbacks.onUnknownInst?.(coreIdx, pc, opcode),
      on_break: (coreIdx) => this._callbacks.onBreak?.(coreIdx),
      write_watchpoint: (addr, coreIdx) => { this._callbacks.writeWatchpoint?.(addr, coreIdx); return 0; },
      trace_mem_write: (coreIdx, pc, addr, val, size) => this._callbacks.traceMemWrite?.(coreIdx, pc, addr, val, size),
      trace_entry: (coreIdx, pc, arg) => this._callbacks.traceEntry?.(coreIdx, pc, arg),
      trace_return: (coreIdx, pc, retVal, retAddr) => this._callbacks.traceReturn?.(coreIdx, pc, retVal, retAddr),
      get_cpu_ticks: () => this._callbacks.getCpuTicks?.() ?? 0,
      get_cpu_cycles: () => this._callbacks.getCpuCycles?.() ?? 0,
      get_sim_freq: () => this._callbacks.getSimFreq?.() ?? 0,
      get_cpu_freq: () => this._callbacks.getCpuFreq?.() ?? 0,
      ccompare_schedule: (coreIdx, which, value) => {
        const intBits = [6, 15, 16];
        const intBit = intBits[which];
        if (intBit === undefined) return;
        if (!this._ccompareEvents) this._ccompareEvents = [];
        if (!this._ccompareEvents[coreIdx]) this._ccompareEvents[coreIdx] = [];
        if (!this._ccompareEvents[coreIdx][which]) {
          const coreOff = coreIdx * 4096;
          const intEnableOff = coreOff + 336 + 226 * 4;
          const pendingIntOff = coreOff + 2540;
          const u32 = new Uint32Array(this.memoryBuffer);
          this._ccompareEvents[coreIdx][which] = this.esp32.clocks.cpu.createEvent(() => {
            u32[(intEnableOff >>> 2)] |= (1 << intBit);
            u32[(pendingIntOff >>> 2)] = 1;
          });
        }
        const ev = this._ccompareEvents[coreIdx][which];
        if (value === 0xffffffff) {
          ev.schedule(0);
        } else {
          ev.schedule(value);
        }
      },
      ccompare_unschedule: (coreIdx, which) => {
        const ev = this._ccompareEvents?.[coreIdx]?.[which];
        if (ev) ev.unschedule();
      },



      // Native UART bridge: TX byte emitted by the Rust UARCtrl goes to the JS
      // UartController.onTX callback (the ring-buffer hook wired by the worker).
      js_uart_tx_byte: (idx, byte) => {
        const u = this.esp32?.uart?.[idx];
        if (u?.onTX) u.onTX(byte);
        else console.log(`[UART-BRIDGE] dropped byte idx=${idx} byte=${byte}`);
      },
      is_window_inst: (_corePtr) => 0,

      map_address: (coreIdx, addr) => {
        if (!this.esp32) return 0;
        this.esp32.mapAddress(addr, coreIdx);
        return 1;
      },

      js_flash_write_override: () => (ReadonlyMemory.override ? 1 : 0),

      map_read: (coreIdx, addr, size) => {
        if (!this.esp32) return 0;
        this._mmioCount[40]++;
        const page = addr >>> 12;
        let c = this._mapCache.get(page);
        if (c && c.multi) c = null;
        if (!c) {
          c = { region: this.esp32.mapAddress(addr, coreIdx), m: this.esp32.mmuEntryFor(addr, coreIdx), multi: false };
          // Pages whose region varies within the 4KB page (e.g. 0x3FF48:
          // RTC_CNTL 0x3FF48000 + SENS/ADC 0x3FF48800) must never be cached —
          // the first region would shadow the rest.
          if (c.m === undefined) {
            const base = page << 12;
            const r0 = this.esp32.mapAddress(base, coreIdx);
            const r1 = this.esp32.mapAddress(base | 0x7ff, coreIdx);
            const r2 = this.esp32.mapAddress(base | 0xfff, coreIdx);
            if (r0 !== c.region || r1 !== c.region || r2 !== c.region) c.multi = true;
          }
          if (!c.multi) this._mapCache.set(page, c);
        } else if (c.m !== undefined && c.m !== this.esp32.mmuEntryFor(addr, coreIdx)) {
          c.region = this.esp32.mapAddress(addr, coreIdx);
          c.m = this.esp32.mmuEntryFor(addr, coreIdx);
        }
        // Size convention: BITS (8/16/32) from the Rust engine (memory.rs);
        // accept byte sizes (1/2/4, e.g. GDB) too. The old `size === 1`
        // mapping sent every fallback access as 32-bit, zeroing neighbors
        // on sub-word PSRAM stores (heap canary corruption).
        const rsize = size <= 8 ? 8 : size <= 16 ? 16 : 32;
        return c.region[`readUint${rsize}`](addr);
      },

      map_write: (coreIdx, addr, val, size) => {
        if (!this.esp32) return;
        this._mmioCount[41]++;
        const page = addr >>> 12;
        let c = this._mapCache.get(page);
        if (c && c.multi) c = null;
        if (!c) {
          c = { region: this.esp32.mapAddress(addr, coreIdx), m: this.esp32.mmuEntryFor(addr, coreIdx), multi: false };
          if (c.m === undefined) {
            const base = page << 12;
            const r0 = this.esp32.mapAddress(base, coreIdx);
            const r1 = this.esp32.mapAddress(base | 0x7ff, coreIdx);
            const r2 = this.esp32.mapAddress(base | 0xfff, coreIdx);
            if (r0 !== c.region || r1 !== c.region || r2 !== c.region) c.multi = true;
          }
          if (!c.multi) this._mapCache.set(page, c);
        } else if (c.m !== undefined && c.m !== this.esp32.mmuEntryFor(addr, coreIdx)) {
          c.region = this.esp32.mapAddress(addr, coreIdx);
          c.m = this.esp32.mmuEntryFor(addr, coreIdx);
        }
        // Size convention: BITS (8/16/32) from the Rust engine — see map_read.
        const wsize = size <= 8 ? 8 : size <= 16 ? 16 : 32;
        c.region[`writeUint${wsize}`](addr, val);
      },

      has_breakpoint: (coreIdx, pc) => this._callbacks.hasBreakpoint?.(coreIdx, pc) ? 1 : 0,

      abort: (msg, file, line, col) => {
        console.error(`[WASM ABORT] msg=${msg} file=${file} line=${line} col=${col}`);
      },
      js_interrupt: (irq, level) => {
        this._mmioCount[42]++;
        if (this.esp32?.interrupt) {
          this.esp32.interrupt(irq >>> 0, level !== 0);
        }
      },
js_log_u32: (val) => {
        console.log(`[WASM] 0x${(val >>> 0).toString(16).padStart(8,'0')}`);
      },
      js_log_str: (ptr, len) => {
        this._mmioCount[43]++;
        const buf = new Uint8Array(this.memory.buffer);
        const bytes = buf.slice(ptr, ptr + len);
        console.log(`[WASM] ${new TextDecoder().decode(bytes)}`);
      },
      js_spi_flash_get_byte: (off) => {
        const f = this.esp32?.flash;
        return (f && off < f.length) ? f[off] : 0;
      },
      js_spi_flash_set_byte: (off, val) => {
        const f = this.esp32?.flash;
        if (f && off < f.length) f[off] = val;
        // Keep the linear-memory flash mirror in sync (Rust flash fast-path)
        const m = this.esp32?._flashMirror;
        if (m && off < m.length) m[off] = val;
      },
      // Virtual SD card block storage (JS parity with js_spi_flash_* byte
      // bridges). Rust passes a linear-memory scratch ptr (512B).
      js_sd_num_blocks: () => {
        const sd = this.esp32?.sdData;
        return sd ? (sd.length / 512) >>> 0 : 32768;
      },
      js_sd_read_block: (block, ptr) => {
        const sd = this.esp32?.sdData;
        const mem = new Uint8Array(this.memory.buffer);
        if (!sd || block * 512 + 512 > sd.length) {
          mem.fill(0, ptr, ptr + 512);
          return;
        }
        mem.set(sd.subarray(block * 512, block * 512 + 512), ptr);
      },
      js_sd_write_block: (block, ptr) => {
        const sd = this.esp32?.sdData;
        if (!sd || block * 512 + 512 > sd.length) return;
        const mem = new Uint8Array(this.memory.buffer);
        sd.set(mem.subarray(ptr, ptr + 512), block * 512);
      },


      // Native I2S TX data hook — forwards consumed DMA TX words to the JS
      // host (JS parity with I2sPeripheral.onTxData).
      js_i2s_tx_data: (idx, ptr, len) => {
        if (!this._i2sTxDataHook) return;
        const buf = new Uint8Array(this.memory.buffer);
        const words = new Uint32Array(buf.slice(ptr, ptr + len * 4).buffer);
        this._i2sTxDataHook(idx, words);
      },
      // Native RTC bridge: sleep wakeup on the rcSlow clock event queue.
      // Fires native_rtc_fire_sleep_wakeup when rcSlow ticks reach the target.
      js_rtc_schedule_wakeup: (target) => {
        if (this.esp32?.clocks?.rcSlow?.createEvent && this.exports?.native_rtc_fire_sleep_wakeup) {
          if (!this._rtcWakeupEvent) {
            this._rtcWakeupEvent = this.esp32.clocks.rcSlow.createEvent(() => {
              try { this.exports.native_rtc_fire_sleep_wakeup(); }
              catch (e) { console.error('[WASM-RTC-WAKEUP-ERR]', e?.message || e); }
            });
          }
          const delta = Math.max(0, (target >>> 0) - (this.esp32.clocks.rcSlow?.ticks ?? 0));
          this._rtcWakeupEvent.schedule(delta);
        }
      },
      js_set_core_enabled: (idx, enabled) => {
        if (this.esp32?._wasmCores?.[idx]) this.esp32._wasmCores[idx].enabled = enabled !== 0;
      },
      js_on_analog_read: (pin, cfg) => this.esp32?.onAnalogRead?.(pin, cfg) ?? 0,
      js_on_touch_read: (pad) => this.esp32?.onTouchRead?.(pad) ?? 1000,
      js_dac_write: (channel, value) => { this.esp32?.onDacWrite?.(channel, value); },
      js_core_enter_light_sleep: (idx) => {
        this.esp32?._wasmCores?.[idx]?.enterLightSleep?.();
      },
      js_core_exit_light_sleep: (idx) => {
        this.esp32?._wasmCores?.[idx]?.exitLightSleep?.();
      },
      js_set_reset_reason: (reason) => {
        if (this.esp32) this.esp32.resetReason = reason >>> 0;
      },
      js_get_reset_reason: () => this.esp32?.resetReason ?? 0,
      js_reset_soc: () => {
        try { this.esp32?.reset?.(); } catch (e) { console.error('[WASM-RTC-RESET-ERR]', e?.message || e); }
      },
      js_reset_core: (idx) => {
        try {
          this.esp32?.cores?.[idx]?.reset?.();
          this.esp32?._wasmCores?.[idx]?.reset?.();
        } catch (e) { console.error('[WASM-RTC-CORE-RESET-ERR]', e?.message || e); }
      },
      // Native WiFi MAC bridge: TX-complete clock event (JS parity with
      // wifi.txCompleteEvent.schedule(1e3) — fires native_wifi_tx_complete →
      // on_tx_complete → setEvent(0x80)).
      js_wifi_tx_complete: (nanos) => {
        if (this.esp32?.clocks?.cpu?.createEvent && this.exports?.native_wifi_tx_complete) {
          if (!this._wifiTxCompleteEvent) {
            this._wifiTxCompleteEvent = this.esp32.clocks.cpu.createEvent(() => {
              this.exports.native_wifi_tx_complete();
            });
          }
          this._wifiTxCompleteEvent.schedule(nanos);
        }
      },
      // Native WiFi MAC TX frame bridge — JS parity with the JS peripheral's
      // writeUint32 DMA_TXBUF arm calling this.onTX(hVal) synchronously.
      js_wifi_send_frame: (ptr, len) => {
        const buf = new Uint8Array(this.memory.buffer);
        const frame = buf.slice(ptr, ptr + len);
        this._wifiMacTx?.(frame);
      },
      // ESP-NOW action-frame TX bridge — raw 802.11 MPDU bytes for the host
      // medium hook (installed by worker-entry; two-node delivery).
      js_espnow_tx_frame: (ptr, len) => {
        const buf = new Uint8Array(this.memory.buffer);
        const frame = buf.slice(ptr, ptr + len);
        this._espnowTx?.(frame);
      },
      // Native WiFi AP (NativeInternetAP) host bridges — the Rust AP builds
      // frames in its statics and hands pointers to the worker, which owns
      // the gateway WebSocket, pcap records and RX delivery (sendFrame).
      // Hooks (_wifiApRxFrame/_wifiApSendEth/_wifiApConnected) are installed
      // by worker-entry after instantiation.
      js_wifi_ap_rx_frame: (ptr, len) => {
        this._wifiApRxFrame?.(ptr, len);
      },
      js_wifi_ap_send_eth: (ptr, len) => {
        this._wifiApSendEth?.(ptr, len);
      },
      js_wifi_ap_connected: () => {
        this._wifiApConnected?.();
      },
      // Native DPORT shell bridges — behavioral side-effects of the native
      // DPORT handler (clock tree, core1 reset/stall, peripheral clock-gate
      // /reset enables, cross-core IRQs). The interrupt matrix is fully Rust
      // (InterruptMatrixPeripheral) — no JS matrix round-trip.
      js_dport_get_cpu_clock_period: () => (this.esp32?.clocks?.cpuClockPeriod ?? 0),
      js_dport_core1_reset: () => {
        try {
          this.esp32?.cores?.[1]?.reset?.();
          this.esp32?._wasmCores?.[1]?.reset?.();
          if (this.esp32?.cores?.[1]?.specialRegisters) {
            // XtensaCore specialRegisters[IntSet] = 31 (JS parity, IntSet=22)
            this.esp32.cores[1].specialRegisters[22] = 31;
          }
        } catch (e) { console.error('[WASM-DPORT-CORE1-RESET-ERR]', e?.message || e); }
      },
      js_dport_set_cpu_clock_period: (val) => {
        const clk = this.esp32?.clocks; if (!clk) return;
        clk.cpuClockPeriod = 3 & val;
        clk.update?.();
      },
      js_dport_peri_clk_en: (val) => {
        const clk = this.esp32?.clocks; if (!clk) return;
        clk.tg0Timer.enable = !!(val & 8192);
        clk.tg0WDT.enable = !!(val & 8192);
        clk.tg1Timer.enable = !!(val & 32768);
        clk.tg1WDT.enable = !!(val & 32768);
        clk.ledc.enable = !!(val & 2048);
        clk.rmt.enable = !!(val & 512);
        clk.uart0.enable = !!(val & 4);
        clk.uart1.enable = !!(val & 32);
        clk.uart2.enable = !!(val & 8388608);
        clk.spi2.enable = !!(val & 64);
        clk.i2c0.enable = !!(val & 128);
      },
      js_dport_peri_rst_en: (val) => {
        const chip = this.esp32; if (!chip) return;
        const reset = (base, bit) => chip.resetPeripheral(base, !!(val & bit));
        reset(0x3ff5a000, 16384); // EFUSE
        reset(0x3ff53000, 128); // I2C0
        reset(0x3ff67000, 262144); // I2C1
        reset(0x3ff4f000, 16); // I2S0
        reset(0x3ff6d000, 2097152); // I2S1
        reset(0x3ff59000, 2048); // LEDC
        reset(0x3ff57000, 1024); // PCNT
        reset(0x3ff56000, 512); // RMT
        reset(0x3ff43000, 2); // SPI0
        reset(0x3ff42000, 2); // SPI1
        reset(0x3ff64000, 64); // SPI2
        reset(0x3ff65000, 65536); // SPI3
        reset(0x3ff5f000, 8192); // TIMG0
        reset(0x3ff60000, 32768); // TIMG1
        reset(0x3ff6b000, 524288); // TWAI0
        reset(0x3ff40000, 4); // UART0
        reset(0x3ff50000, 32); // UART1
        reset(0x3ff6e000, 8388608); // UART2
      },
      js_dport_refresh_core1_enabled: () => {
        if (!this.esp32) return;
        this.esp32.cores[1].enabled = (this.exports?.native_dport_get_core1_enabled?.() ?? 0) !== 0;
      },
      js_rtc_pause_wdts: () => {
        try { this.esp32?.clocks?.pauseApbClocks?.(); } catch {}
      },
      js_rtc_resume_wdts: () => {
        try { this.esp32?.clocks?.resumeApbClocks?.(); } catch {}
      },
    };

    const importObj = { env };
    if (wasmBytes instanceof WebAssembly.Module) {
      this.instance = new WebAssembly.Instance(wasmBytes, importObj);
    } else {
      const result = await WebAssembly.instantiate(wasmBytes, importObj);
      this.instance = result.instance;
    }

    this.exports = this.instance.exports;
    if (this.exports.native_peripheral_init) {
      this.exports.native_peripheral_init();
    }
    if (this.exports.native_flash_init) {
      this.exports.native_flash_init(WM.FLASH_DATA_OFFSET, WM.MMU_TABLE_REGION_ID);
    }
    // run265r: push the app .flash.text seg3 file offset so the Rust HOOK
    // scanner reads true .flash.text bytes. The image is a bootloader
    // container — VMA 0x400D0020 is NOT at file offset 0 (seg3 ~0x40020).
    if (this.exports.native_flash_seg3_off && this.esp32?.flash) {
      try {
        const f = this.esp32.flash;
        const u32 = (o) => (f[o] | (f[o+1] << 8) | (f[o+2] << 16) | (f[o+3] << 24)) >>> 0;
        let seg3 = 0;
        for (const base of [0x10000, 0x1000, 0x0]) {
          if (f[base] !== 0xe9) continue;
          const n = f[base + 1];
          let off = base + 0x18;
          for (let i = 0; i < n && off + 8 <= f.length; i++) {
            if (u32(off) === 0x400d0020) { seg3 = off + 8; break; }
            off += 8 + u32(off + 4);
          }
          if (seg3) break;
        }
        if (seg3) this.exports.native_flash_seg3_off(seg3);
      } catch {}
    }
    const ptU32 = new Uint32Array(this.memory.buffer, WM.PAGE_TABLE_OFFSET);

    // Override pages with Rust native MMIO handlers
    const setPTE = (addr, hid) => {
      const page = addr >>> 12;
      ptU32[page * 2]     = 2; // PTE_TYPE_MMIO
      ptU32[page * 2 + 1] = NATIVE_HANDLER_FLAG | hid;
    };
    setPTE(0x3FF03000, HID_SHA);
    // Native AES (0x3FF01000) — compute-only, polling protocol, no interrupts
    setPTE(0x3FF01000, HID_AES);
    // TIMG0 native port re-enabled 2026-08 — parity port complete in timers.rs; monitor CPU1-WDT window
    setPTE(0x3FF5F000, HID_TIMG0);
    // Native TIMG1 (0x3FF60000) — same parity port, group=1 event tags, IRQs 18-21
    setPTE(0x3FF60000, HID_TIMG1);
    // Native GPIO (0x3FF44000) + IO_MUX (0x3FF49000) — mux writes notify the
    // Rust GpioController (JS parity: i2c-i2s.js IoMuxPeripheral → gpio.updateGPIO)
    setPTE(0x3FF44000, HID_GPIO);
    setPTE(0x3FF49000, HID_IO_MUX);
    // Native FRC1 timer (0x3FF47000) — alarm processing via the WASM clock statics (self-timed core)
    setPTE(0x3FF47000, HID_FRC_TIMER);
    // Native UART (UART0 0x3FF40000, UART1 0x3FF50000, UART2 0x3FF6E000)
    setPTE(0x3FF40000, HID_UART);
    setPTE(0x3FF50000, HID_UART);
    setPTE(0x3FF6E000, HID_UART);
    // Native I2C (I2C0 0x3FF53000, I2C1 0x3FF67000)
    setPTE(0x3FF53000, HID_I2C);
    setPTE(0x3FF67000, HID_I2C);
    // Native SPI (SPI1 0x3FF42000, SPI0 0x3FF43000, SPI2 0x3FF64000, SPI3 0x3FF65000)
    setPTE(0x3FF42000, HID_SPI);
    setPTE(0x3FF43000, HID_SPI);
    setPTE(0x3FF64000, HID_SPI);
    setPTE(0x3FF65000, HID_SPI);
    // Native EFUSE (0x3FF5A000) + SYSCON (0x3FF66000) — register-file parity ports
    // with native reset re-seeding (native_efuse_reset / native_syscon_reset).
    setPTE(0x3FF5A000, HID_EFUSE);
    setPTE(0x3FF66000, HID_SYSCON);
    // Native TWAI (0x3FF6B000) — loopback-only parity port, synchronous interrupts
    setPTE(0x3FF6B000, HID_TWAI);
    // Native RSA (0x3FF02000) — synchronous accelerator, parity with JS RsaPeripheral
    setPTE(0x3FF02000, HID_RSA);
    // Native RTC (0x3FF48000) — one 4KB page holding RTC_CNTL + RTC_IO + SENS + RTC_I2C
    setPTE(0x3FF48000, HID_RTC); // Native RTC (0x3FF48000) — one 4KB page holding RTC_CNTL + RTC_IO + SENS + RTC_I2C
    setPTE(0x3FF59000, HID_LEDC); // Native LEDC (0x3FF59000)
    setPTE(0x3FF57000, HID_PCNT); // Native PCNT (0x3FF57000)
    setPTE(0x3FF56000, HID_RMT); // Native RMT (0x3FF56000)
    setPTE(0x3FF4F000, HID_I2S); // Native I2S0 (0x3FF4F000)
    setPTE(0x3FF6D000, HID_I2S); // Native I2S1 (0x3FF6D000)
    setPTE(0x3FF68000, HID_SDMMC); // Native SDMMC (0x3FF68000)
    setPTE(0x3FF00000, HID_DPORT); // Native DPORT (0x3FF00000) — full Esp32FullResetValues seed in Rust
    setPTE(0x3FF75000, HID_RNG); // Native RNG (0x3FF75000) — js_rng_random_u32 host source
    setPTE(0x60035000, HID_RNG); // Native RNG DROM0 alias (0x3FF75000+0x200C0000) — the bootloader reads the RNG data register through this window (0x60035144); the JS alias wrapped the dead EmptyPeripheral and returned 0, so bootloader_fill_random's XOR always produced 0 and process_segment_data spun forever on ram_obfs_value
    setPTE(0x3FF4E000, HID_WIFI_ANALOG); // Native WiFi analog (0x3FF4E000)
    setPTE(0x3FF73000, HID_WIFI_MAC); // Native WiFi MAC (0x3FF73000)
    setPTE(0x6000E000, HID_WIFI_ANALOG); // Native WiFi analog DROM0 alias (0x3FF4E000+0x200C0000)
    setPTE(0x60033000, HID_WIFI_MAC); // Native WiFi MAC DROM0 alias (0x3FF73000+0x200C0000)
    setPTE(0x60000000, HID_UART); // UART0/SDMMC_ALT DROM0 alias (0x3FF40000+0x200C0000) — the UART driver configures through this window; Rust normalize_drom0_alias maps it back
    setPTE(0x60010000, HID_UART); // UART1 DROM0 alias (0x3FF50000+0x200C0000)
    setPTE(0x6002E000, HID_MCPWM); // MCPWM0 DROM0 alias (0x3FF5E000+0x200C0000)
    setPTE(0x60013000, HID_I2C); // Native I2C0 DROM0 alias (0x3FF53000+0x200C0000) — the HAL moves ALL FIFO bytes through this window (i2c_ll_write_txfifo); without it addr/data bytes vanish (reads see 0x00) and every transaction NACKs
    setPTE(0x60027000, HID_I2C); // Native I2C1 DROM0 alias (0x3FF67000+0x200C0000)
    setPTE(0x3FF58000, HID_SDIO_SLAVE); // Native SDIO slave (0x3FF58000)
    setPTE(0x3FF46000, HID_FE); // Native FE front-end (0x3FF46000)
    setPTE(0x3FF5E000, HID_MCPWM); // Native MCPWM0 (0x3FF5E000)
    setPTE(0x3FF6C000, HID_MCPWM); // Native MCPWM1 (0x3FF6C000)
    setPTE(0x3FF54000, HID_UHCI); // Native UHCI0 (0x3FF54000)
    setPTE(0x3FF4C000, HID_UHCI); // Native UHCI1 (0x3FF4C000)
    setPTE(0x3FF69000, HID_EMAC); // Native EMAC MAC (0x3FF69000)
    setPTE(0x3FF6A000, HID_EMAC); // Native EMAC DMA (0x3FF6A000)
    setPTE(0x3FF04000, HID_SWEEP); // Secure Boot (0x3FF04000)
    setPTE(0x3FF4B000, HID_SWEEP); // I2C config (0x3FF4B000)
    setPTE(0x3FF55000, HID_SWEEP); // SLCHOST (0x3FF55000)
    setPTE(0x3FF5B000, HID_SWEEP); // Flash Encryption (0x3FF5B000)
    setPTE(0x3FF1F000, HID_SWEEP); // PID Controller per-CPU (0x3FF1F000)
    setPTE(0x3FF1A000, HID_SWEEP); // Digital Signature (0x3FF1A000, stub: HMAC-gated, no driver)
    setPTE(0x3FF5D000, HID_STUB_ZERO); // WiFi BB stub (JS EmptyPeripheral, no traffic)
    setPTE(0x3FF5CC00, HID_STUB_ZERO); // WiFi NRX stub (JS EmptyPeripheral, no traffic)
    setPTE(0x3FF74000, HID_STUB_ZERO); // WiFi WiMac/2 stub (JS EmptyPeripheral, no traffic)
    setPTE(0x3FF81000, HID_INVALID_MEM); // Boot-ROM scratch (reads 0xFF..F, writes dropped)
    setPTE(0x3FF71000, HID_BT_RF); // BT RF / LD-clock block (BtRfPeripheral port — bit31 self-clearing)
    setPTE(0x3FF72000, HID_BT_RF); // BT modem-status block 0x3FF712xx alias (2026-09-14: r_rwble_isr gates on [0x3FF71210]; single 4KB PTE left it on the invalid path = eternal beqz-bail). Handler folds by page offset.

    console.log(`[WASM] TIMG0 PTE: type=${ptU32[0x3FF5F * 2]} data=0x${(ptU32[0x3FF5F * 2 + 1] >>> 0).toString(16)}`);
    // Check Rust PAGE_TABLE_OFFSET vs JS
    const rustPtOff = this.exports.native_get_pt_offset();
    console.log(`[WASM] JS_PTOFS=0x${WM.PAGE_TABLE_OFFSET.toString(16)} RUST_PTOFS=0x${rustPtOff.toString(16)} MATCH=${WM.PAGE_TABLE_OFFSET === rustPtOff}`);
    // Dump Rust's view of the TIMG0 PTE
    this.exports.native_print_pt_entry(0x3FF5F);
    const shaPage = 0x3FF03000 >>> 12;
    console.log(`[WASM] SHA PTE: type=${ptU32[shaPage * 2]} data=0x${(ptU32[shaPage * 2 + 1] >>> 0).toString(16)}`);
    const mmuPage = 0x3FF48000 >>> 12;
    console.log(`[WASM] MMU 0x3FF48 PTE: type=${ptU32[mmuPage * 2]} data=0x${(ptU32[mmuPage * 2 + 1] >>> 0).toString(16)}`);
    const flashPage = 0x400C2000 >>> 12;
    console.log(`[WASM] FLASHWIN 0x400C2000 PTE: type=${ptU32[flashPage * 2]} data=0x${(ptU32[flashPage * 2 + 1] >>> 0).toString(16)}`);
    const iramPage = 0x40080000 >>> 12;
    console.log(`[WASM] IRAM 0x40080000 PTE: type=${ptU32[iramPage * 2]} data=0x${(ptU32[iramPage * 2 + 1] >>> 0).toString(16)}`);

    // Initialize Rust native peripheral state
    try {
      this.exports.native_timg0_init();
      console.log(`[WASM] TIMG0 init OK, access count=${this.exports.native_timg0_get_access_count()}`);
    } catch(e) {
      console.error(`[WASM] TIMG0 init FAILED: ${e.message}`);
    }
    if (this.exports.native_timg0_reset_count) this.exports.native_timg0_reset_count();

    try {
      if (this.exports.native_timg1_init) this.exports.native_timg1_init();
      if (this.exports.native_timg1_reset) this.exports.native_timg1_reset();
      console.log('[WASM] TIMG1 init OK');
    } catch(e) {
      console.error(`[WASM] TIMG1 init FAILED: ${e.message}`);
    }

    try {
      if (this.exports.native_frc_timer_init) this.exports.native_frc_timer_init();
    } catch(e) {
      console.error(`[WASM] FRC timer init FAILED: ${e.message}`);
    }

    // Native GPIO init + seed JS pin input levels (strap pins, test inputs)
    try {
      if (this.exports.native_gpio_init) this.exports.native_gpio_init();
      this.seedNativeGpio();
      console.log('[WASM] GPIO init OK');
    } catch(e) {
      console.error(`[WASM] GPIO init FAILED: ${e.message}`);
    }

    // Native UART init + initial reset
    try {
      if (this.exports.native_uart_init) this.exports.native_uart_init();
      if (this.exports.native_uart_reset) this.exports.native_uart_reset();
      console.log('[WASM] UART init OK');
    } catch(e) {
      console.error(`[WASM] UART init FAILED: ${e.message}`);
    }

    // Native I2C init + initial reset
    try {
      if (this.exports.native_i2c_init) this.exports.native_i2c_init();
      if (this.exports.native_i2c_reset) this.exports.native_i2c_reset();
      console.log('[WASM] I2C init OK');
    } catch(e) {
      console.error(`[WASM] I2C init FAILED: ${e.message}`);
    }

    // Native SPI init + initial reset
    try {
      if (this.exports.native_spi_init) this.exports.native_spi_init();
      if (this.exports.native_spi_reset) this.exports.native_spi_reset();
      console.log('[WASM] SPI init OK');
    } catch(e) {
      console.error(`[WASM] SPI init FAILED: ${e.message}`);
    }

    // Native EFUSE + SYSCON init + initial reset (register-file seeds)
    try {
      if (this.exports.native_efuse_reset) this.exports.native_efuse_reset();
      if (this.exports.native_syscon_reset) this.exports.native_syscon_reset();
      console.log('[WASM] EFUSE/SYSCON init OK');
    } catch(e) {
      console.error(`[WASM] EFUSE/SYSCON init FAILED: ${e.message}`);
    }

    // Native TWAI init + initial reset
    try {
      if (this.exports.native_twai_init) this.exports.native_twai_init();
      if (this.exports.native_twai_reset) this.exports.native_twai_reset();
      console.log('[WASM] TWAI init OK');
    } catch(e) {
      console.error(`[WASM] TWAI init FAILED: ${e.message}`);
    }

    // Native SDMMC init + initial reset
    try {
      if (this.exports.native_sdmmc_init) this.exports.native_sdmmc_init();
      if (this.exports.native_sdmmc_reset) this.exports.native_sdmmc_reset();
      console.log('[WASM] SDMMC init OK');
    } catch(e) {
      console.error(`[WASM] SDMMC init FAILED: ${e.message}`);
    }

    // INIT-BLOCKS-DISABLED

// Native RSA init + initial reset
    try {
      if (this.exports.native_rsa_init) this.exports.native_rsa_init();
      if (this.exports.native_rsa_reset) this.exports.native_rsa_reset();
      console.log('[WASM] RSA init OK');
    } catch(e) {
      console.error(`[WASM] RSA init FAILED: ${e.message}`);
    }

    // Native RTC init + initial reset
    try {
      if (this.exports.native_rtc_init) this.exports.native_rtc_init();
      if (this.exports.native_rtc_reset) this.exports.native_rtc_reset();
      console.log('[WASM] RTC init OK');
    } catch(e) {
      console.error(`[WASM] RTC init FAILED: ${e.message}`);
    }

    // Native LEDC init + initial reset
    try {
      if (this.exports.native_ledc_init) this.exports.native_ledc_init();
      if (this.exports.native_ledc_reset) this.exports.native_ledc_reset();
      console.log('[WASM] LEDC init OK');
    } catch(e) {
      console.error(`[WASM] LEDC init FAILED: ${e.message}`);
    }

    // Native PCNT init + initial reset
    try {
      if (this.exports.native_pcnt_init) this.exports.native_pcnt_init();
      if (this.exports.native_pcnt_reset) this.exports.native_pcnt_reset();
      console.log('[WASM] PCNT init OK');
    } catch(e) {
      console.error(`[WASM] PCNT init FAILED: ${e.message}`);
    }

    // Native RMT init + initial reset
    try {
      if (this.exports.native_rmt_init) this.exports.native_rmt_init();
      if (this.exports.native_rmt_reset) this.exports.native_rmt_reset();
      console.log('[WASM] RMT init OK');
    } catch(e) {
      console.error(`[WASM] RMT init FAILED: ${e.message}`);
    }

    // BT task/queue intervention kill-switch: the BT shim's per-step
    // hook-scan (byte-stepped flash sweep) costs ~3s/step on non-BT
    // firmware with zero benefit (no BT libraries linked = no hooks can
    // ever match). Disable unless the firmware actually uses BT/BLE.
    // Opt-in is via config.bleShim=true (worker init config -> chip
    // config -> esp32.bleShimEnabled, set in the ESP32 constructor before
    // loadWasm; test-worker-bt / ble-init do this). Without this, every
    // worker test (gpio/uart/...) wedges: 500k-step chunks × ~3s/step
    // never finish.
    try {
      const wantShim = !!(this.esp32?.bleShimEnabled || this.esp32?.config?.bleShim);
      if (this.exports.native_bt_diag_disable) this.exports.native_bt_diag_disable(wantShim ? 0 : 1);
      if (wantShim) console.log('[WASM] BT shim ENABLED (bleShim opt-in)');
    } catch(e) {
      console.error(`[WASM] BT diag disable FAILED: ${e.message}`);
    }

    console.log(`[WASM] ESP32 engine (wasm) initialized`);
    return this;
  }

  createCores(count) {
    this._wasmCores = [];
    for (let i = 0; i < count; i++) {
      const wc = new WasmCore(this, i);
      const processorId = this.esp32?.cores[i]?.processorId ?? 0;
      wc.init(processorId, 0);
      wc.reset();
      this._wasmCores.push(wc);
    }
    return this._wasmCores;
  }

  get wasmCores() { return this._wasmCores; }

  setDebugLog(enabled) {
    const u32 = new Uint32Array(this.memory.buffer);
    for (let i = 0; i < this._wasmCores.length; i++) {
      const baseOff = i * WM.CORE_STATE_SIZE;
      const DEBUG_LOG_OFF = 2584 + 7 * 4; // pc(2584) next_pc ccompare0 ccompare1 ccompare2 debug_opcode inst_count debug_log
      u32[(baseOff + DEBUG_LOG_OFF) >>> 2] = enabled ? 1 : 0;
    }
  }

  // Push the JS GPIO pin input levels + strap into the Rust GpioController
  // (mirrors applyBasicSetup pinInputs; call after chip.reset() too — reset
  // clears native pin state). One-shot bulk export: 41 x u32 LE into the
  // GPIO_SEED_SCRATCH (pins 0..39 input levels, index 40 = strap).
  seedNativeGpio() {
    if (!this.exports?.native_gpio_seed) return;
    const gpio = this.esp32?.gpio;
    if (!gpio?.pins) return;
    const ptr = this.exports.native_gpio_seed_scratch();
    const u32 = new Uint32Array(this.memory.buffer, ptr, 41);
    for (let i = 0; i < gpio.pins.length; i++) {
      const pin = gpio.pins[i];
      u32[i] = pin.inputValue ? 1 : 0;
    }
    u32[40] = (gpio.strapValue ?? 0) >>> 0;
    try { this.exports.native_gpio_seed(); } catch {}
  }

  // Native DPORT bridge helper: route an in-page offset to the JS interrupt
  // matrix (matrix0: STATUS0-2 at 236/240/244 + maps 260..536; matrix1:
  // STATUS0-2 at 248/252/256 + maps 536..812 = 536 + 4*MAX_INT(69)).
  // Rust InterruptMatrixPeripheral instances own the matrix state natively —
  // no JS matrix (was DportPeripheral.intMatrix, removed with the FFI round-trip).
}

export class WasmCore {
  constructor(loader, index) {
    this._loader = loader;
    this._exports = loader.exports;
    this.index = index;
    this.name = `WASM_CORE_${index}`;
    this.debugState = {};

    // SAB-backed views for core state
    const sab = loader.memory.buffer;
    const baseOff = index * WM.CORE_STATE_SIZE;

    this._physRegs   = new Uint32Array(sab, baseOff + CORE_OFF_PHYS_REGS, 64);
    this._specRegs   = new Uint32Array(sab, baseOff + CORE_OFF_SPECIAL_REGS, 256);
    this._pcView          = new Uint32Array(sab, baseOff + CORE_OFF_PC, 1);
    this._nextPcView      = new Uint32Array(sab, baseOff + CORE_OFF_NEXT_PC, 1);
    this._enabledView     = new Uint32Array(sab, baseOff + CORE_OFF_ENABLED, 1);
    this._idleView        = new Uint32Array(sab, baseOff + CORE_OFF_IDLE, 1);
    this._lightSleepView  = new Uint32Array(sab, baseOff + CORE_OFF_LIGHT_SLEEP, 1);
    this._pendingIntView  = new Uint32Array(sab, baseOff + CORE_OFF_PENDING_INT, 1);
    this._opcodeSegmentView = new Uint32Array(sab, baseOff + CORE_OFF_OPCODE_SEG, 1);
  }

  get enabled()   { return this._enabledView[0] !== 0; }
  set enabled(v)  { this._enabledView[0] = v ? 1 : 0; }
  get idle()      { return this._idleView[0] !== 0; }
  set idle(v)     { this._idleView[0] = v ? 1 : 0; }
  get pendingInterrupts() { return this._pendingIntView[0]; }
  set pendingInterrupts(v) { this._pendingIntView[0] = v; }

  init(processorId, arch) { this._exports.core_init(this.index, processorId, arch); }
  reset() { this._exports.core_reset(this.index); }

  runInstruction() {
    if (!this._enabledView[0]) return;
    this._exports.core_step(this.index);
  }

  readUint8(addr) { return this._exports.core_read_uint8(this.index, addr); }
  readUint16(addr) { return this._exports.core_read_uint16(this.index, addr); }
  readUint32(addr) { return this._exports.core_read_uint32(this.index, addr); }
  writeUint8(addr, val) { this._exports.core_write_uint8(this.index, addr, val); }
  writeUint16(addr, val) { this._exports.core_write_uint16(this.index, addr, val); }
  writeUint32(addr, val) { this._exports.core_write_uint32(this.index, addr, val); }

  get PC() { return this._pcView[0]; }
  set PC(v) { this._pcView[0] = v; }
  get nextPC() { return this._nextPcView[0]; }
  set nextPC(v) { this._nextPcView[0] = v; }

  // Windowed AR register access (matches Rust CoreState::ar())
  AR(reg) {
    const base = this._specRegs[MEM_FAULT_INFO] << 2;
    return this._physRegs[(base + reg) % 64];
  }
  setAR(reg, val) {
    const base = this._specRegs[MEM_FAULT_INFO] << 2;
    this._physRegs[(base + reg) % 64] = val;
  }

  // BR registers (match state.rs br()/set_br())
  BR(reg) {
    return !!(this._specRegs[4] & (1 << reg)); // PS_REGISTER = 4
  }
  setBR(reg, val) {
    if (val) this._specRegs[4] |= (1 << reg);
    else this._specRegs[4] &= ~(1 << reg);
  }

  get specialRegisters() { return this._specRegs; }
  set specialRegisters(_v) {}

  get config() { return {}; }

  unimplemented(opcode) { console.warn(`WASM core ${this.index}: unimplemented ${opcode.toString(16)}`); }
  exception(cause) { console.warn(`WASM core ${this.index}: exception ${cause}`); }
  windowCheck(_a, _b, _c) { return false; }
  setPSExcm(_v) {}
  get PS_CRING() { return 0; }
  setFloatFlags(_flags) {}
  get nextInterrupt() { return 0; }
  set nextInterrupt(_v) {}
  get nextInterruptPri() { return 0; }
  set nextInterruptPri(_v) {}
  attachMemorySystem(_pt, _mmio, _regions, _cbs) {}
}
