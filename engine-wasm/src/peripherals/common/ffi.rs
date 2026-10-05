// FFI bridge to browser APIs via extern "C"
// The JS host provides these functions when the WASM module is instantiated.

// Timer APIs (JS: setTimeout/clearTimeout)
extern "C" {
    pub fn js_set_timeout(ms: u32, callback_id: u32) -> u32;
    pub fn js_clear_timeout(timer_id: u32);
}

// WASM memory creation (JS: new WebAssembly.Memory({initial, maximum, shared}))
extern "C" {
    pub fn js_create_wasm_memory(initial_pages: u32, max_pages: u32) -> u32;
    pub fn js_get_memory_buffer(handle: u32) -> *mut u8;
    pub fn js_get_memory_length(handle: u32) -> u32;
}

// WASM loading (JS: WebAssembly.instantiate)
extern "C" {
    pub fn js_load_wasm(wasm_bytes: *const u8, len: usize, core_idx: u32) -> i32;
    pub fn js_step_core(core_idx: u32, cycles: u32);
    pub fn js_get_wasm_memory(core_idx: u32) -> u32;
}

// Debug/info callbacks (host provides these)
extern "C" {
    pub fn js_on_reset();
    pub fn js_on_analog_read(pin: u32, cfg: u32) -> u32;
    pub fn js_on_touch_read(pad: u32) -> u32;
    pub fn js_dac_write(channel: u32, value: u32);
    pub fn js_on_break(core: u32, reason: u32);
}

// SharedArrayBuffer creation
extern "C" {
    pub fn js_create_sab(byte_length: u32) -> *mut u8;
}

// Partition table / flash
extern "C" {
    pub fn js_write_flash(offset: u32, data: *const u8, len: u32);
}

// Interrupt matrix
extern "C" {
    pub fn js_interrupt(irq: u32, level: u32);
}

// UART native bridge
extern "C" {
    // TX byte emitted by a native UART → JS UartController.onTX (ring buffer hook)
    pub fn js_uart_tx_byte(idx: u32, byte: u32);
}

// SPI flash bridge — JS chip.flash SAB is not addressable from WASM linear memory,
// so SPI flash commands go through these byte accessors (matches JS flashCmdWrite/flashCommand).
extern "C" {
    pub fn js_spi_flash_get_byte(offset: u32) -> u32;
    pub fn js_spi_flash_set_byte(offset: u32, val: u32);
    // Erase-done clock event (JS createEvent) — fires native_spi_flash_erase_done(idx)
    pub fn js_spi_flash_erase_done(idx: u32, nanos: u32);
}

// I2S native bridge — DMA TX/RX processing passes (JS parity with
// i2s.txClockEvent.schedule(1e4) / i2s.rxClockEvent.schedule(1e4)).
extern "C" {
    // Schedule another DMA TX processing pass (fires native_i2s_clock_tx(idx))
    pub fn js_i2s_schedule_tx(idx: u32, nanos: u32);
    // Schedule another DMA RX processing pass (fires native_i2s_clock_rx(idx))
    pub fn js_i2s_schedule_rx(idx: u32, nanos: u32);
    // TX data consumed by the I2S peripheral (JS onTxData hook, 32-bit words)
    pub fn js_i2s_tx_data(idx: u32, data_ptr: u32, len: u32);
}

// RTC bridge — sleep/wakeup semantics cross the JS/WASM boundary because the
// rcSlow clock event queue and the CPU core enabled flags live in JS.
extern "C" {
    // Schedule the RTC sleep wakeup on the JS rcSlow clock (fires
    // native_rtc_fire_sleep_wakeup when rcSlow ticks reach `target_ticks`).
    pub fn js_rtc_schedule_wakeup(target_ticks: u32);
    // Set core enabled flag on the JS WASM core facades.
    pub fn js_set_core_enabled(idx: u32, enabled: u32);
    pub fn js_core_enter_light_sleep(idx: u32);
    pub fn js_core_exit_light_sleep(idx: u32);
    // Chip reset reason (JS chip.resetReason).
    pub fn js_set_reset_reason(reason: u32);
    pub fn js_get_reset_reason() -> u32;
    // Full SoC reset (JS chip.reset()) and single-core reset.
    pub fn js_reset_soc();
    pub fn js_reset_core(idx: u32);
    // Light-sleep WDT pause/resume (JS clocks.pauseApbClocks/resumeApbClocks).
    pub fn js_rtc_pause_wdts();
    pub fn js_rtc_resume_wdts();
}

// SDMMC native bridge — command-complete delivery (JS parity with
// sdmmc.cmdCompleteEvent.schedule(1e3) → onCmdComplete()).
extern "C" {
    // Schedule command-complete interrupt delivery (fires native_sdmmc_cmd_complete())
    pub fn js_sdmmc_schedule(nanos: u32);
}

// Virtual SD card block storage — the JS host owns the card image
// (chip.sdData, allocated from config.sdCard, default 16MB zero-filled).
// Rust passes a linear-memory scratch pointer (1 FFI call per 512B block).
extern "C" {
    // Number of 512-byte blocks on the virtual card (default 32768 = 16MB).
    pub fn js_sd_num_blocks() -> u32;
    // Copy one 512B block FROM the card image TO wasm linear memory at `ptr`.
    pub fn js_sd_read_block(block: u32, ptr: u32);
    // Copy one 512B block FROM wasm linear memory at `ptr` TO the card image.
    pub fn js_sd_write_block(block: u32, ptr: u32);
}

// WiFi native bridges — MAC TX-complete clock event (JS parity with
// wifi.txCompleteEvent.schedule(1e3) → txComplete()).
extern "C" {
    pub fn js_wifi_tx_complete(nanos: u32);
    pub fn js_wifi_send_frame(data_ptr: u32, len: u32);
    // ESP-NOW action-frame TX: raw 802.11 MPDU bytes for the host medium
    // hook (two-node delivery via the shared gateway room).
    pub fn js_espnow_tx_frame(data_ptr: u32, len: u32);
}

// WiFi AP (NativeInternetAP) host bridges — the native AP builds 802.11
// frames in its own statics and hands pointers to the worker, which owns
// the gateway WebSocket, pcap records and the RX delivery (sendFrame).
extern "C" {
    pub fn js_wifi_ap_rx_frame(data_ptr: u32, len: u32);
    pub fn js_wifi_ap_send_eth(data_ptr: u32, len: u32);
    pub fn js_wifi_ap_connected();
}

// BT HCI proxy bridges (Bumble virtual controller over TCP) — scaffold,
// default OFF. The engine stages a raw H2C HCI packet (type byte + payload)
// in a linear-memory scratch and hands the pointer to the JS worker, which
// owns the TCP socket to tests/test-bumble-hci.py. C2H bytes come back via
// native_bt_hci_push_c2h (length-prefixed scratch write + poll flag), and
// the VHCI C2H inject path consumes them exactly like the canned CC.
extern "C" {
    // H2C packet staged at (ptr,len) for the JS TCP proxy to forward.
    pub fn js_bt_hci_send_packet(data_ptr: u32, len: u32);
}

// DPORT native bridge — behavioral side effects stay in JS (clock tree, core
// state, peripheral resets). The interrupt matrix runs fully in Rust
// (InterruptMatrixPeripheral instances wired into the native DPORT handler —
// commit 934943c); the js_dport_matrix_* round-trip was removed.
extern "C" {
    pub fn js_dport_get_cpu_clock_period() -> u32;
    pub fn js_dport_set_cpu_clock_period(val: u32);
    pub fn js_dport_peri_clk_en(val: u32);
    pub fn js_dport_peri_rst_en(val: u32);
    pub fn js_dport_core1_reset();
    pub fn js_dport_refresh_core1_enabled();
}

// Host I2C slave taps (esp32-emu.md §1) — synchronous, additive-only.
// The engine calls these ONLY when the per-bus hook is enabled via
// native_i2c_set_slave_hook (set when the first host slave attaches or a
// bus-level onStart handler is assigned; cleared never — wiring survives
// reset, in-flight transaction state does not). With no hook the legacy
// sibling/virt routing is untouched (zero behavior change, one bool check).
//
// Contract (mirrors the F1 runner tap shape):
// - js_i2c_slave_start(bus, addr7, read): host claims the address?
//   Return 1 = ACK (host owns this transaction), 0 = not claimed (fall
//   through to sibling/virt routing, then historical NACK-ish path).
// - js_i2c_slave_write(bus, byte): data byte on a host-claimed WRITE.
//   Return 1 = ACK, 0 = NACK (propagates to ACK_ERR + abort, real-HW parity).
// - js_i2c_slave_read(bus): next byte for a host-claimed READ (0..255).
//   Called only while claimed; host must answer synchronously (pre-staged
//   queue or model function). Default when unimplemented: 0xFF.
// - js_i2c_slave_stop(bus, was_read): transaction close (model commit point).
// Handlers MUST be synchronous pure-JS (no WASM re-entry, no await) — they
// run inside the guest's MMIO write.
extern "C" {
    pub fn js_i2c_slave_start(bus: u32, addr: u32, read: u32) -> u32;
    pub fn js_i2c_slave_write(bus: u32, byte: u32) -> u32;
    pub fn js_i2c_slave_read(bus: u32) -> u32;
    pub fn js_i2c_slave_stop(bus: u32, was_read: u32);
}

// Host SPI transfer tap (esp32-emu.md §2) — synchronous, additive-only.
// Called ONLY when the per-unit hook is enabled via
// native_spi_set_transfer_hook (set on onTransfer assignment or injectMiso
// preload; wiring survives reset). MOSI bytes are staged at the scratch
// returned by native_spi_xfer_mosi_ptr(); the handler writes MISO bytes at
// native_spi_xfer_miso_ptr() and returns the MISO length (0 = not claimed,
// fall through to the sibling virtual-bus exchange, then loopback).
// Fires once per CPU-mode (W-reg) transaction at CMD USR time — the ESP32
// equivalent of F1's per-DR-write onTransfer. DMA and flash-controller
// transfers never call it (documented limitation).
extern "C" {
    pub fn js_spi_transfer(bus: u32, send_len: u32, recv_len: u32) -> u32;
}

// GPIO output change events (esp32-emu.md §3) — level, additive-only.
// Fired synchronously from the output-update path when the driven level of
// a pin changes AND the hook is enabled via native_gpio_set_change_hook.
// Handlers MUST be non-reentrant (record, never MMIO back into WASM).
extern "C" {
    pub fn js_gpio_changed(pin: u32, level: u32);
}
