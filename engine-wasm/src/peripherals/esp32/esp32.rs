// Translated from: src/peripherals/esp32/esp32.js
// Do NOT modify the logic — match the JS line-for-line

use crate::peripherals::types::*;
use crate::peripherals::common::memory::*;
use crate::peripherals::common::register_data::*;
use crate::peripherals::common::clocks::*;
use crate::peripherals::esp32::interrupt_efuse::*;
use crate::peripherals::common::gpio_core::*;
use crate::peripherals::common::spi_syscon::*;
use crate::peripherals::common::i2c_i2s::*;
use crate::peripherals::common::twai_fifo::*;
use crate::peripherals::common::wifi_analog::*;
use crate::peripherals::common::sdmmc::*;
use crate::peripherals::common::rmt_rng_math::*;
use crate::peripherals::common::timers::*;
use crate::peripherals::common::rtc_adc::*;
use crate::peripherals::common::ledc_pcnt::*;
use crate::peripherals::common::aes::AesPeripheral;
use crate::peripherals::common::sha_ecc_key::*;
use crate::peripherals::common::uart::{UartConfig, UartRegisterMap, UartFieldMap, UartController};
use crate::peripherals::common::ffi;
use crate::native_mmio;
use core::ptr;

// ==============================
// Interrupt enum (JS lines 233-305)
// ==============================
pub const MAC_INTR: u32 = 0; pub const MAC_NMI: u32 = 1; pub const BB_INT: u32 = 2;
pub const BT_MAC_INT: u32 = 3; pub const BT_BB_INT: u32 = 4; pub const BT_BB_NMI: u32 = 5;
pub const RWBT_IRQ: u32 = 6; pub const RWBLE_IRQ: u32 = 7; pub const RWBT_NMI: u32 = 8;
pub const RWBLE_NMI: u32 = 9; pub const SLC0_INTR: u32 = 10; pub const SLC1_INTR: u32 = 11;
pub const UHCI0_INTR: u32 = 12; pub const UHCI1_INTR: u32 = 13;
pub const TG_T0_LEVEL_INT: u32 = 14; pub const TG_T1_LEVEL_INT: u32 = 15;
pub const TG_WDT_LEVEL_INT: u32 = 16; pub const TG_LACT_LEVEL_INT: u32 = 17;
pub const TG1_T0_LEVEL_INT: u32 = 18; pub const TG1_T1_LEVEL_INT: u32 = 19;
pub const TG1_WDT_LEVEL_INT: u32 = 20; pub const TG1_LACT_LEVEL_INT: u32 = 21;
pub const GPIO_INTERRUPT_PRO: u32 = 22; pub const GPIO_INTERRUPT_PRO_NMI: u32 = 23;
pub const CPU_INTR_FROM_CPU_0: u32 = 24; pub const CPU_INTR_FROM_CPU_1: u32 = 25;
pub const CPU_INTR_FROM_CPU_2: u32 = 26; pub const CPU_INTR_FROM_CPU_3: u32 = 27;
pub const SPI_INTR_0: u32 = 28; pub const SPI_INTR_1: u32 = 29; pub const SPI_INTR_2: u32 = 30;
pub const SPI_INTR_3: u32 = 31; pub const I2S0_INT: u32 = 32; pub const I2S1_INT: u32 = 33;
pub const UART_INTR: u32 = 34; pub const UART1_INTR: u32 = 35; pub const UART2_INTR: u32 = 36;
pub const SDIO_HOST_INTERRUPT: u32 = 37; pub const EMAC_INT: u32 = 38;
pub const PWM0_INTR: u32 = 39; pub const PWM1_INTR: u32 = 40; pub const PWM2_INTR: u32 = 41;
pub const PWM3_INTR: u32 = 42; pub const LEDC_INT: u32 = 43; pub const EFUSE_INT: u32 = 44;
pub const CAN_INT: u32 = 45; pub const RTC_CORE_INTR: u32 = 46; pub const RMT_INTR: u32 = 47;
pub const PCNT_INTR: u32 = 48; pub const I2C_EXT0_INTR: u32 = 49;
pub const I2C_EXT1_INTR: u32 = 50; pub const RSA_INTR: u32 = 51;
pub const SPI1_DMA_INT: u32 = 52; pub const SPI2_DMA_INT: u32 = 53;
pub const SPI3_DMA_INT: u32 = 54; pub const WDG_INT: u32 = 55;
pub const TIMER_INT1: u32 = 56; pub const TIMER_INT2: u32 = 57;
pub const TG_T0_EDGE_INT: u32 = 58; pub const TG_T1_EDGE_INT: u32 = 59;
pub const TG_WDT_EDGE_INT: u32 = 60; pub const TG_LACT_EDGE_INT: u32 = 61;
pub const TG1_T0_EDGE_INT: u32 = 62; pub const TG1_T1_EDGE_INT: u32 = 63;
pub const TG1_WDT_EDGE_INT: u32 = 64; pub const TG1_LACT_EDGE_INT: u32 = 65;
pub const MMU_IA_INT: u32 = 66; pub const MPU_IA_INT: u32 = 67;
pub const CACHE_IA_INT: u32 = 68; pub const MAX_INT: u32 = 69;

pub const ESP_KEY: bool = false;
pub const ESP32_YN: bool = false;
pub const GPIO_BOTH_DIR: u32 = 3;

// WASM memory layout offsets (matches xtensa::state for ESP32)
pub const WASM_CORE_STATE_SIZE: u32 = 4096;
pub const WASM_PAGE_TABLE_OFFSET: u32 = 4194304;
pub const WASM_PAGE_TABLE_BYTE_SIZE: u32 = 8388608;
pub const WASM_REGION_TABLE_OFFSET: u32 = 12582912;
pub const WASM_RAM_DATA_OFFSET: u32 = 12583168;

pub struct Devices<'a> {
    pub flash: &'a [u8],
    pub iram: &'a [u8],
    pub dram: &'a [u8],
    pub rtc_fast: &'a [u8],
    pub rtc_slow: &'a [u8],
    pub mmu: &'a [u8],
    pub psram: &'a [u8],
}

pub struct CpuCore {
    pub enabled: bool,
    pub idle: bool,
    pub pending_interrupts: bool,
}

impl CpuCore {
    pub fn new(enabled: bool) -> Self {
        CpuCore { enabled, idle: true, pending_interrupts: false }
    }
    pub fn reset(&mut self) {
    }
    pub fn run_instruction(&mut self) {
    }
    pub fn write_register(&mut self, _addr: u32, _val: u32) {
    }
}

pub const TIMG0_INT_CACHE_CONTROL: u32 = TG_T0_LEVEL_INT;
pub const TIMG0_INT_ATOM_CTRL: u32 = TG_T1_LEVEL_INT;
pub const TIMG0_INT_WDT: u32 = TG_WDT_LEVEL_INT;
pub const TIMG0_INT_LACT: u32 = TG_LACT_LEVEL_INT;
pub const TIMG1_INT_CACHE_CONTROL: u32 = TG1_T0_LEVEL_INT;
pub const TIMG1_INT_ATOM_CTRL: u32 = TG1_T1_LEVEL_INT;
pub const TIMG1_INT_WDT: u32 = TG1_WDT_LEVEL_INT;
pub const TIMG1_INT_LACT: u32 = TG1_LACT_LEVEL_INT;

extern "C" fn default_on_reset() { unsafe { ffi::js_on_reset(); } }
extern "C" fn default_on_analog_read(pin: u32, cfg: u32) -> u32 { unsafe { ffi::js_on_analog_read(pin, cfg) } }

fn parse_size_mb(cfg: &[u32; 8], key: u32, alt: u32, def: u32) -> u32 {
    let v = if cfg[key as usize] != 0 { cfg[key as usize] } else { cfg[alt as usize] };
    if v != 0 { v } else { def }
}

// ==============================
// Peripheral address dispatch table
// ==============================

const PERIPHERAL_TABLE_SIZE: usize = 64;

#[derive(Clone, Copy)]
pub struct PeripheralEntry {
    pub base_addr: u32,
    pub name: &'static str,
    pub irq: u32,
    pub flags: u32,
}

pub struct Esp32Peripheral {
    pub config: [u32; 8],
    pub chip_name: &'static str,
    pub chip_id: u32,
    pub flash_size_mb: u32,
    pub psram_size_mb: u32,
    pub psram_type: u32,
    pub firmware_offset: u32,
    pub _mac_address: [u8; 6],
    pub flash: *mut u8,
    pub flash_len: u32,
    pub chip_rom: Memory,
    pub rom1: Memory,
    pub iram: Memory,
    pub psram_buf: *mut u8,
    pub psram_len: u32,
    pub psram_memory: Memory,
    pub data_mem: Memory,
    pub sram1_reverse: Memory,
    pub rtc_fast_mem: Memory,
    pub rtc_slow_mem: Memory,
    pub invalid_mem: InvalidMemory,
    pub mmu_table_memory: Memory,
    pub mmu_table_pro: *mut u32,
    pub mmu_table_app: *mut u32,
    pub dma_base: u32,
    pub peripheral_entries: [PeripheralEntry; PERIPHERAL_TABLE_SIZE],
    pub peripheral_count: u32,
    pub gpio_matrix: GpioMatrix,
    pub reset_reason: u32,
    pub native_frequency: u32,
    pub cycles: u64,
    pub cores: [CpuCore; 2],
    pub stopped: bool,
    pub write_watch_points: [u32; 32],
    pub write_wp_count: u32,
    pub last_mapped_address: u32,
    pub twai_count: u32,
    pub xts_state: u32,
    pub clock_tree: ClockTree,
    pub dport: DportPeripheral,
    pub gpio: GpioController<'static>,
    pub uart0: Option<UartController>,
    pub uart1: Option<UartController>,
    pub uart2: Option<UartController>,
    pub spi: [SpiPeripheral; 4],
    pub i2c: [I2cPeripheral; 2],
    pub i2s: [I2sPeripheral; 2],
    pub twai: [TwaiPeripheral; 1],
    pub aes: AesPeripheral,
    pub sha: ShaPeripheral,
    pub rsa: RsaPeripheral,
    pub wifi: WifiMacPeripheral,
    pub sdmmc: SdmmcPeripheral,
    pub fe_stub: StubPeripheral,
    pub frc_timer: FrcTimerPeripheral,
    pub rtc: RtcCntlPeripheral,
    pub rtc_io_0: RtcIoPeripheral,
    pub rtc_io_1: RtcIoPeripheral,
    pub adc: AdcPeripheral,
    pub io_mux: IoMuxPeripheral,
    pub analog_rf: AnalogRfPeripheral,
    pub rmt: RmtPeripheral,
    pub pcnt: PcntPeripheral,
    pub sdio_slave: SdioSlavePeripheral,
    pub ledc: LedcPeripheral,
    pub efuse: EfuseControllerPeripheral,
    pub timer_group0: TimerGroupPeripheral,
    pub timer_group1: TimerGroupPeripheral,
    pub syscon: SysconPeripheral,
    pub rng: RngPeripheral,
    pub is_initialized: bool,
    pub flash_mmu_map: [i32; 512],
    pub peripheral_index_map: [i32; 512],
    pub gdb_target_xml_present: u32,
    pub trace_available: u32,
    pub page_table: PageTable,
    pub mmio_handlers: MMIOHandlerRegistry,
    // WASM engine state (JS: _wasmMemory, _wasmCores, _memRegions, etc.)
    pub wasm_memory: *mut u8,
    pub wasm_memory_len: u32,
    pub wasm_cores_loaded: bool,
    pub engine_mode: u8,
    pub mem_region_offsets: [u32; 6],
    pub sab_u32: *mut u32,
    pub sab_u32_valid: bool,
    pub cycles_per_step: u32,
    pub mem_pages: u32,
    pub on_reset: Option<extern "C" fn()>,
    pub on_analog_read: Option<extern "C" fn(u32, u32) -> u32>,
}

static GPIO_STRAP_PINS: [i32; 6] = [5, 15, 4, 2, 0, 12];
static GPIO_CONFIG: GpioConfig<'static> = GpioConfig {
    gpio_count: 40,
    out_function_max: 256,
    strap_value: 19,
    strap_boot: 16,
    strap_pins: &GPIO_STRAP_PINS,
    irq: GPIO_INTERRUPT_PRO,
    nmi_irq: GPIO_INTERRUPT_PRO_NMI,
    reg_enable: GPIO_REG_ENABLE,
    reg_out: GPIO_REG_OUT,
    reg_out1: GPIO_REG_OUT1,
    reg_enable1: GPIO_REG_ENABLE1,
    reg_out1_w1ts: GPIO_REG_OUT1_W1TS,
    reg_out1_w1tc: GPIO_REG_OUT1_W1TC,
    reg_enable_w1ts: GPIO_REG_ENABLE_W1TS,
    reg_enable_w1tc: GPIO_REG_ENABLE_W1TC,
    reg_enable1_w1ts: GPIO_REG_ENABLE1_W1TS,
    reg_enable1_w1tc: GPIO_REG_ENABLE1_W1TC,
    reg_status_w1ts: GPIO_REG_STATUS_W1TS,
    reg_status_w1tc: GPIO_REG_STATUS_W1TC,
    reg_status1_w1ts: GPIO_REG_STATUS1_W1TS,
    reg_status1_w1tc: GPIO_REG_STATUS1_W1TC,
    reg_strap: GPIO_REG_STRAP,
    reg_syscon_tick_count_mask: 60,
    reg_in1: GPIO_REG_IN1,
    reg_status: GPIO_REG_STATUS,
    reg_status1: GPIO_REG_STATUS1,
    reg_acpu_int: GPIO_REG_ACPU_INT,
    reg_acpu_nmi_int: GPIO_REG_ACPU_NMI_INT,
    reg_pcpu_int: GPIO_REG_PCPU_INT,
    reg_pcpu_nmi_int: GPIO_REG_PCPU_NMI_INT,
    reg_acpu_int1: GPIO_REG_ACPU_INT1,
    reg_acpu_nmi_int1: GPIO_REG_ACPU_NMI_INT1,
    reg_pcpu_int1: GPIO_REG_PCPU_INT1,
    reg_pcpu_nmi_int1: GPIO_REG_PCPU_NMI_INT1,
    reg_intr_0: GPIO_REG_INTR_0,
    reg_intr1_0: GPIO_REG_INTR1_0,
    reg_intr_1: GPIO_REG_INTR_1,
    reg_intr1_1: GPIO_REG_INTR1_1,
    reg_pin0: GPIO_REG_PIN0,
    reg_func0_out_sel_cfg: GPIO_REG_FUNC0_OUT_SEL_CFG,
    reg_funcn_in_sel_cfg: GPIO_REG_FUNCN_IN_SEL_CFG_FIRST,
    funcn_in_sel_cfg_first: GPIO_REG_FUNCN_IN_SEL_CFG_FIRST,
    funcn_in_sel_cfg_count: GPIO_REG_FUNCN_IN_SEL_CFG_COUNT,
    funcn_in_sel_cfg_start_index: GPIO_REG_FUNCN_IN_SEL_CFG_START_INDEX,
    f_in_sel: FieldDef { shift: 0, mask: 0x3F },
    f_in_inv_sel: FieldDef { shift: 6, mask: 1 },
    f_sel: FieldDef { shift: 7, mask: 1 },
    iomux_table: &[],
};

impl Esp32Peripheral {
    pub fn new(cfg: [u32; 8]) -> Self {
        let flash_size_mb_val = parse_size_mb(&cfg, 0, 1, 4);
        let psram_size_mb_val = parse_size_mb(&cfg, 2, 3, 4);
        let psram_type_val = cfg[4];
        let firmware_offset_val = cfg[5];
        let mut mac = [0u8; 6];
        mac[0] = (cfg[6] & 0xFF) as u8;
        mac[1] = ((cfg[6] >> 8) & 0xFF) as u8;
        mac[2] = ((cfg[6] >> 16) & 0xFF) as u8;
        mac[3] = ((cfg[6] >> 24) & 0xFF) as u8;
        mac[4] = (cfg[7] & 0xFF) as u8;
        mac[5] = ((cfg[7] >> 8) & 0xFF) as u8;
        let dport_config = DportConfig {
            cross_core_irqs: [CPU_INTR_FROM_CPU_0, CPU_INTR_FROM_CPU_1, CPU_INTR_FROM_CPU_2, CPU_INTR_FROM_CPU_3],
            clocks: DportClocks {
                tg0_timer_enable: true, tg0_wdt_enable: true, tg1_timer_enable: true, tg1_wdt_enable: true,
                ledc_enable: true, rmt_enable: true, uart0_enable: true, uart1_enable: true,
                uart2_enable: true, spi2_enable: true, i2c0_enable: true,
            },
            reset_efuse: None, reset_i2c0: None, reset_i2c1: None,
            reset_i2s0: None, reset_i2s1: None, reset_ledc: None,
            reset_pcnt: None, reset_rmt: None, reset_spi0: None,
            reset_spi1: None, reset_spi2: None, reset_spi3: None,
            reset_timg0: None, reset_timg1: None, reset_twai0: None,
            reset_uart0: None, reset_uart1: None, reset_uart2: None,
        };
        let gpio = GpioController::new(UART_BASE_ADDR, &GPIO_CONFIG);

        let spi1 = SpiPeripheral::new(SPI1_BASE_ADDR, "SPI1", SpiConfig { cs_bits: 3, addr_left_aligned: true, flash: true, irq: SPI_INTR_1, ..SpiConfig::new() }, core::ptr::null_mut(), 0, 0);
        let spi0 = SpiPeripheral::new(SPI0_BASE_ADDR, "SPI0", SpiConfig { cs_bits: 3, addr_left_aligned: true, irq: SPI_INTR_0, ..SpiConfig::new() }, core::ptr::null_mut(), 0, 0);
        let spi2 = SpiPeripheral::new(SPI2_BASE_ADDR, "SPI2", SpiConfig { cs_bits: 3, addr_left_aligned: true, irq: SPI_INTR_2, ..SpiConfig::new() }, core::ptr::null_mut(), 0, 0);
        let spi3 = SpiPeripheral::new(SPI3_BASE_ADDR, "SPI3", SpiConfig { cs_bits: 3, addr_left_aligned: true, irq: SPI_INTR_3, ..SpiConfig::new() }, core::ptr::null_mut(), 0, 0);

        let i2c_cfg: I2cConfig = unsafe { core::mem::zeroed() };
        let i2c_xtal = ClockRef::new(40_000_000);
        let i2c0 = I2cPeripheral::new(I2C0_BASE_ADDR, "I2C0", i2c_cfg, I2C_EXT0_INTR, i2c_xtal);
        let i2c1 = I2cPeripheral::new(I2C1_BASE_ADDR, "I2C1", i2c_cfg, I2C_EXT1_INTR, i2c_xtal);

        let i2s_cfg: I2sConfig = unsafe { core::mem::zeroed() };
        let i2s0 = I2sPeripheral::new(I2S0_BASE_ADDR, "I2S0", i2s_cfg, I2S0_INT, 0);
        let i2s1 = I2sPeripheral::new(I2S1_BASE_ADDR, "I2S1", i2s_cfg, I2S1_INT, 1);

        let twai_cfg: TwaiConfig = unsafe { core::mem::zeroed() };
        let twai0 = TwaiPeripheral::new(TWAI_BASE_ADDR, 0, twai_cfg, CAN_INT);

        let aes = AesPeripheral::new(DPORT_BASE_ADDR, "AES Accelerator");
        let sha = ShaPeripheral::new(SHA_BASE_ADDR, "SHA Accelerator");
        let rsa = RsaPeripheral::new(RSA_BASE_ADDR, "RSA Accelerator");

        let wifi_cfg: WifiMacConfig = unsafe { core::mem::zeroed() };
        let wifi = WifiMacPeripheral::new(0x3ff73000, wifi_cfg);

        let sdmmc_cfg: SdmmcConfig = unsafe { core::mem::zeroed() };
        let sdmmc = SdmmcPeripheral::new(SDMMC_BASE_ADDR, "SDMMC", sdmmc_cfg);

        let fe_stub = StubPeripheral::new(FE_BASE_ADDR, StubRmtChannelRegister { iq_est: 124 });

        let frc_timer = FrcTimerPeripheral::new(FRC_TIMER_BASE_ADDR, "FRC", TIMER_INT1, TIMER_INT2);

        let rtc_cfg: RtcCntlConfig = unsafe { core::mem::zeroed() };
        let rtc = RtcCntlPeripheral::new(RTC_CNTL_BASE_ADDR, "RTC_CNTL", rtc_cfg);

        let rtc_io_0 = RtcIoPeripheral::new(RTC_IO_BASE_ADDR, "RTC_IO", unsafe { core::mem::zeroed() });
        let rtc_io_1 = RtcIoPeripheral::new(RTC_I2C_BASE_ADDR, "RTC_I2C", unsafe { core::mem::zeroed() });

        let adc = AdcPeripheral::new(SENS_ADC_BASE_ADDR, "SENS");

        let iomux_cfg: IoMuxConfig = unsafe { core::mem::zeroed() };
        let io_mux = IoMuxPeripheral::new(IO_MUX_BASE_ADDR, "TimerLoadLoOffset MUX", iomux_cfg);

        let analog_rf_cfg: AnalogRfConfig = unsafe { core::mem::zeroed() };
        let analog_rf = AnalogRfPeripheral::new(0x3ff4e000, analog_rf_cfg);

        let rmt_cfg: RmtPeripheralConfig = unsafe { core::mem::zeroed() };
        let rmt = RmtPeripheral::new(RMT_BASE_ADDR, "RMT", rmt_cfg);

        let pcnt_cfg: PcntConfig = unsafe { core::mem::zeroed() };
        let pcnt = PcntPeripheral::new(PCNT_ALT_BASE_ADDR, "PCNT", pcnt_cfg, PCNT_INTR);

        let sdio_slave = SdioSlavePeripheral::new(SDIO_SLAVE_BASE_ADDR, "SDIO Slave 3/3");

        let ledc_cfg: LedcConfig = unsafe { core::mem::zeroed() };
        let ledc = LedcPeripheral::new(LEDC_BASE_ADDR, "LED PWM", ledc_cfg);

        let efuse_cfg: EfuseConfig = unsafe { core::mem::zeroed() };
        let efuse = EfuseControllerPeripheral::new(EFUSE_BASE_ADDR, efuse_cfg);

        let timer_group0 = TimerGroupPeripheral::new(0, TIMER_GROUP0_BASE_ADDR, "TIMG0", unsafe { core::mem::zeroed() }, TG_T0_LEVEL_INT, TG_T1_LEVEL_INT as i32, TG_WDT_LEVEL_INT, TG_LACT_LEVEL_INT as i32);
        let timer_group1 = TimerGroupPeripheral::new(1, TIMER_GROUP1_BASE_ADDR, "TIMG1", unsafe { core::mem::zeroed() }, TG1_T0_LEVEL_INT, TG1_T1_LEVEL_INT as i32, TG1_WDT_LEVEL_INT, TG1_LACT_LEVEL_INT as i32);

        let syscon = SysconPeripheral::new(SYSCON_BASE_ADDR, "SYSCON");
        let rng = RngPeripheral::new(0x3ff75000, "RNG", 324);

        let mut esp = Esp32Peripheral {
            config: cfg,
            chip_name: "esp32",
            chip_id: 0,
            flash_size_mb: flash_size_mb_val,
            psram_size_mb: psram_size_mb_val,
            psram_type: psram_type_val,
            firmware_offset: firmware_offset_val,
            _mac_address: mac,
            flash: ptr::null_mut(),
            flash_len: flash_size_mb_val * DROM0_SIZE,
            chip_rom: Memory::new(ptr::null_mut(), IRAM0_SIZE, REGION_CODE_BASE),
            rom1: Memory::new(ptr::null_mut(), 65536, REGION_DRAM1_BASE),
            iram: Memory::new(ptr::null_mut(), REGION_IROM0_BASE - REGION_FLASH_CACHE_BASE, REGION_FLASH_CACHE_BASE),
            psram_buf: ptr::null_mut(),
            psram_len: psram_size_mb_val * DROM0_SIZE,
            psram_memory: Memory::new(ptr::null_mut(), psram_size_mb_val * DROM0_SIZE, REGION_DROM0_BASE),
            data_mem: Memory::new(ptr::null_mut(), IRAM1_SIZE + DROM0_CACHE_SIZE, REGION_RTC_SLOW_BASE),
            sram1_reverse: Memory::new(ptr::null_mut(), REGION_IROM0_BASE_ALT.wrapping_sub(REGION_IRAM1_BASE_ALT), REGION_IROM0_BASE_ALT),
            rtc_fast_mem: Memory::new(ptr::null_mut(), REGION_IRAM1_BASE_ALT - REGION_IRAM0_BASE, REGION_IRAM0_BASE),
            rtc_slow_mem: Memory::new(ptr::null_mut(), RTC_SLOW_SIZE, USB_OTG_BASE_ADDR),
            invalid_mem: InvalidMemory::new(0),
            mmu_table_memory: Memory::new(ptr::null_mut(), 2 * REGION_DROM_SIZE, REGION_PERI1_BASE),
            mmu_table_pro: ptr::null_mut(),
            mmu_table_app: ptr::null_mut(),
            dma_base: 0x3ff00000,
            peripheral_entries: [PeripheralEntry { base_addr: 0, name: "", irq: 0, flags: 0 }; PERIPHERAL_TABLE_SIZE],
            peripheral_count: 0,
            gpio_matrix: GpioMatrix::new(),
            reset_reason: 1,
            native_frequency: 160_000_000,
            cycles: 0,
            cores: [CpuCore::new(true), CpuCore::new(false)],
            stopped: true,
            write_watch_points: [0u32; 32],
            write_wp_count: 0,
            last_mapped_address: 0,
            twai_count: 0,
            xts_state: 0,
            clock_tree: ClockTree::new(160_000_000),
            dport: DportPeripheral::new(GPIO_BASE_ADDR, dport_config),
            gpio,
            uart0: None,
            uart1: None,
            uart2: None,
            spi: [spi1, spi0, spi2, spi3],
            i2c: [i2c0, i2c1],
            i2s: [i2s0, i2s1],
            twai: [twai0],
            aes,
            sha,
            rsa,
            wifi,
            sdmmc,
            fe_stub,
            frc_timer,
            rtc,
            rtc_io_0,
            rtc_io_1,
            adc,
            io_mux,
            analog_rf,
            rmt,
            pcnt,
            sdio_slave,
            ledc,
            efuse,
            timer_group0,
            timer_group1,
            syscon,
            rng,
            is_initialized: false,
            flash_mmu_map: [-1i32; 512],
            peripheral_index_map: [-1i32; 512],
            gdb_target_xml_present: 0,
            trace_available: 0,
            page_table: PageTable::new(core::ptr::null_mut(), 0),
            mmio_handlers: MMIOHandlerRegistry::new(),
            wasm_memory: core::ptr::null_mut(),
            wasm_memory_len: 0,
            wasm_cores_loaded: false,
            engine_mode: 0,
            mem_region_offsets: [0u32; 6],
            sab_u32: core::ptr::null_mut(),
            sab_u32_valid: false,
            cycles_per_step: 1,
            mem_pages: 0,
            on_reset: Some(default_on_reset as extern "C" fn()),
            on_analog_read: Some(default_on_analog_read as extern "C" fn(u32, u32) -> u32),
        };
        // Register all peripherals matching JS constructor (lines 413-760)
        esp.register_peripheral(UART_BASE_ADDR, "GPIO", GPIO_INTERRUPT_PRO, 0);
        esp.register_peripheral(SDMMC_ALT_BASE_ADDR, "UART0", UART_INTR, 0);
        esp.register_peripheral(UHCI_BASE_ADDR, "UART1", UART1_INTR, 0);
        esp.register_peripheral(UHCI_ALT_BASE_ADDR, "UART2", UART2_INTR, 0);
        esp.register_peripheral(SPI1_BASE_ADDR, "SPI1", SPI_INTR_1, 0);
        esp.register_peripheral(SPI0_BASE_ADDR, "SPI0", SPI_INTR_0, 0);
        esp.register_peripheral(SPI2_BASE_ADDR, "SPI2", SPI_INTR_2, 0);
        esp.register_peripheral(SPI3_BASE_ADDR, "SPI3", SPI_INTR_3, 0);
        esp.register_peripheral(I2C0_BASE_ADDR, "I2C0", I2C_EXT0_INTR, 0);
        esp.register_peripheral(I2C1_BASE_ADDR, "I2C1", I2C_EXT1_INTR, 0);
        esp.register_peripheral(I2S0_BASE_ADDR, "I2S0", I2S0_INT, 0);
        esp.register_peripheral(I2S1_BASE_ADDR, "I2S1", I2S1_INT, 0);
        esp.register_peripheral(TWAI_BASE_ADDR, "TWAI", CAN_INT, 0);
        esp.register_peripheral(DPORT_BASE_ADDR, "AES Accelerator", 0, 0);
        esp.register_peripheral(SHA_BASE_ADDR, "SHA Accelerator", 0, 0);
        esp.register_peripheral(0x3ff73000, "WiMac", MAC_INTR, 0);
        esp.register_peripheral(SDMMC_BASE_ADDR, "SDMMC", SDIO_HOST_INTERRUPT, 0);
        esp.register_peripheral(GPIO_BASE_ADDR, "DPort Register", 0, 0);
        esp.register_peripheral(UART1_MEM_BASE_ADDR, "GPIO_SD", 0, 0);
        esp.register_peripheral(RSA_BASE_ADDR, "RSA Accelerator", 0, 0);
        esp.register_peripheral(0x3ff04000, "Secure Boot", 0, 0);
        esp.register_peripheral(0x3ff1f000, "PID Controller Per-CPU peripheral", 0, 0);
        esp.register_peripheral(FE_BASE_ADDR, "FE", 0, 0);
        esp.register_peripheral(FRC_TIMER_BASE_ADDR, "FRC", 0, 0);
        esp.register_peripheral(RTC_CNTL_BASE_ADDR, "RTC_CNTL", RTC_CORE_INTR, 0);
        esp.register_peripheral(RTC_IO_BASE_ADDR, "RTC_IO", 0, 0);
        esp.register_peripheral(RTC_I2C_BASE_ADDR, "RTC_I2C", 0, 0);
        esp.register_peripheral(SENS_ADC_BASE_ADDR, "SENS", 0, 0);
        esp.register_peripheral(IO_MUX_BASE_ADDR, "TimerLoadLoOffset MUX", 0, 0);
        esp.register_peripheral(I2C_CONFIG_BASE_ADDR, "SDIO Slave 1/3", 0, 0);
        esp.register_peripheral(UHCI_ALT_BASE_ADDR3, "UHCI1", 0, 0);
        esp.register_peripheral(0x3ff4e000, "Unknown", 0, 0);
        esp.register_peripheral(UHCI_ALT_BASE_ADDR2, "UHCI0", 0, 0);
        esp.register_peripheral(UART3_BASE_ADDR, "SLCHOST", 0, 0);
        esp.register_peripheral(RMT_BASE_ADDR, "RMT", RMT_INTR, 0);
        esp.register_peripheral(PCNT_ALT_BASE_ADDR, "PCNT", PCNT_INTR, 0);
        esp.register_peripheral(SDIO_SLAVE_BASE_ADDR, "SDIO Slave 3/3", 0, 0);
        esp.register_peripheral(LEDC_BASE_ADDR, "LED PWM", LEDC_INT, 0);
        esp.register_peripheral(EFUSE_BASE_ADDR, "Efuse Controller", EFUSE_INT, 0);
        esp.register_peripheral(0x3ff5b000, "Flash Encryption", 0, 0);
        esp.register_peripheral(PID_CONTROLLER_BASE_ADDR, "MCPWM0", 0, 0);
        esp.register_peripheral(TIMER_GROUP0_BASE_ADDR, "TIMG0", TG_T0_LEVEL_INT, 0);
        esp.register_peripheral(TIMER_GROUP1_BASE_ADDR, "TIMG1", TG1_T0_LEVEL_INT, 0);
        esp.register_peripheral(SYSCON_BASE_ADDR, "SYSCON", 0, 0);
        esp.register_peripheral(0x3ff69000, "EMAC", 0, 0);
        esp.register_peripheral(0x3ff6a000, "EMAC", 0, 0);
        esp.register_peripheral(PCNT_BASE_ADDR, "PWM1", 0, 0);
        esp.register_peripheral(0x3ff74000, "WiMac/2", 0, 0);
        esp.register_peripheral(0x3ff75000, "RNG", 0, 0);
        esp.register_peripheral(RMT_ALT_BASE_ADDR, "BB", 0, 0);
        esp.register_peripheral(PCNT_REGS_BASE_ADDR, "NRX", 0, 0);
        esp.build_flash_mmu_map();
        esp.build_peripheral_index_map();
        esp.flash_fill(255);
        esp.write_partition_table();
        esp.set_mac_address_on_wifi();
        esp.build_page_table_full();
        esp.reset_full();
        esp.stopped = true;
        esp
    }

    pub fn write_partition_table(&mut self) {
        if self.flash.is_null() { return; }
        let off = if self.firmware_offset != 0 { self.firmware_offset } else { 0x10000 };
        let size = self.flash_len.saturating_sub(off);
        if size < 0x2000 { return; }
        let mut buf = [0u8; 64];
        buf[0] = 0xAA; buf[1] = 0x50;
        buf[2] = 0x00; buf[3] = 0x00;
        buf[4] = off as u8; buf[5] = (off >> 8) as u8; buf[6] = (off >> 16) as u8; buf[7] = (off >> 24) as u8;
        buf[8] = size as u8; buf[9] = (size >> 8) as u8; buf[10] = (size >> 16) as u8; buf[11] = (size >> 24) as u8;
        let label = b"app\0";
        buf[12..16].copy_from_slice(label);
        buf[32] = 0xEB; buf[33] = 0xEB;
        for i in 34..64 { buf[i] = 0xFF; }
        unsafe {
            ffi::js_write_flash(0x8000, buf.as_ptr(), buf.len() as u32);
            core::ptr::copy_nonoverlapping(buf.as_ptr(), self.flash.add(0x8000usize), buf.len());
        }
        let sector_end = 0x9000u32;
        let clear_start = 0x8040u32;
        if sector_end > clear_start && sector_end <= self.flash_len {
            let clear_len = (sector_end - clear_start) as usize;
            if self.flash.is_null() { return; }
            unsafe { ptr::write_bytes(self.flash.add(clear_start as usize), 0xFF, clear_len); }
        }
    }

    pub fn set_mac_address_on_wifi(&mut self) {
        let mac = self._mac_address;
        self.wifi.set_mac_address(&mac);
        self.wifi.registers[16] = mac[0] as u32 | (mac[1] as u32) << 8 | (mac[2] as u32) << 16 | (mac[3] as u32) << 24;
        self.wifi.registers[17] = mac[4] as u32 | (mac[5] as u32) << 8;
    }

    pub fn register_peripheral(&mut self, base_addr: u32, name: &'static str, irq: u32, flags: u32) {
        let idx = self.peripheral_count as usize;
        if idx >= PERIPHERAL_TABLE_SIZE { return; }
        self.peripheral_entries[idx] = PeripheralEntry { base_addr, name, irq, flags };
        self.peripheral_count += 1;
    }

    pub fn find_peripheral_by_addr(&self, addr: u32) -> u32 {
        for i in 0..self.peripheral_count as usize {
            let entry = &self.peripheral_entries[i];
            let masked = addr & 0xFFFFF000;
            let base_masked = entry.base_addr & 0xFFFFF000;
            if base_masked == masked || (addr & 0xFFFFFC00) == (entry.base_addr & 0xFFFFFC00) {
                return i as u32;
            }
        }
        0xFFFFFFFF
    }

    pub fn init_buffers(&mut self, flash_buf: *mut u8, chip_rom_buf: *mut u8, iram_buf: *mut u8, psram_buf: *mut u8, data_buf: *mut u8, rtc_fast_buf: *mut u8, rtc_slow_buf: *mut u8, mmu_table_buf: *mut u8, mmu_pro: *mut u32, mmu_app: *mut u32) {
        if self.is_initialized { return; }
        self.flash = flash_buf;
        self.flash_len = self.flash_size_mb * DROM0_SIZE;
        if !chip_rom_buf.is_null() {
            self.chip_rom = Memory::new(chip_rom_buf, IRAM0_SIZE, REGION_CODE_BASE);
            self.rom1 = Memory::new(unsafe { chip_rom_buf.add(REGION_DRAM1_BASE.wrapping_sub(REGION_CODE_BASE) as usize) }, 65536, REGION_DRAM1_BASE);
        }
        if !iram_buf.is_null() {
            self.iram = Memory::new(iram_buf, REGION_IROM0_BASE - REGION_FLASH_CACHE_BASE, REGION_FLASH_CACHE_BASE);
        }
        if !psram_buf.is_null() {
            self.psram_buf = psram_buf;
            self.psram_memory = Memory::new(psram_buf, self.psram_len, REGION_DROM0_BASE);
        }
        if !data_buf.is_null() {
            self.data_mem = Memory::new(data_buf, IRAM1_SIZE + DROM0_CACHE_SIZE, REGION_RTC_SLOW_BASE);
            let rev_offset = IRAM1_SIZE as usize;
            if data_buf as usize + rev_offset + REGION_IROM0_BASE_ALT.wrapping_sub(REGION_IRAM1_BASE_ALT) as usize > 0 {
                self.sram1_reverse = Memory::new(unsafe { data_buf.add(rev_offset) }, DROM0_CACHE_SIZE, REGION_IROM0_BASE_ALT);
            }
        }
        if !rtc_fast_buf.is_null() {
            self.rtc_fast_mem = Memory::new(rtc_fast_buf, REGION_IRAM1_BASE_ALT - REGION_IRAM0_BASE, REGION_IRAM0_BASE);
        }
        if !rtc_slow_buf.is_null() {
            self.rtc_slow_mem = Memory::new(rtc_slow_buf, RTC_SLOW_SIZE, USB_OTG_BASE_ADDR);
        }
        if !mmu_table_buf.is_null() {
            self.mmu_table_memory = Memory::new(mmu_table_buf, 2 * REGION_DROM_SIZE, REGION_PERI1_BASE);
        }
        self.mmu_table_pro = mmu_pro;
        self.mmu_table_app = mmu_app;
        self.is_initialized = true;
    }

    pub fn flash_fill(&mut self, val: u8) {
        if self.flash.is_null() { return; }
        unsafe { ptr::write_bytes(self.flash, val, self.flash_len as usize); }
    }

    pub fn load_rom_data(&mut self, data: &[u8], base: u32) {
        let offset = (base - REGION_CODE_BASE) as usize;
        if self.chip_rom.len == 0 || offset + data.len() > self.chip_rom.len as usize { return; }
        let dest = unsafe { core::slice::from_raw_parts_mut(self.chip_rom.data.add(offset), data.len()) };
        dest.copy_from_slice(data);
        let rom1_offset = REGION_DRAM1_BASE.wrapping_sub(REGION_CODE_BASE) as usize;
        if offset <= rom1_offset && offset + data.len() > rom1_offset {
            let rom1_data_offset = if rom1_offset > offset { rom1_offset - offset } else { 0 };
            let rom1_len = core::cmp::min(65536_usize, data.len().saturating_sub(rom1_data_offset));
            self.rom1 = Memory::new(unsafe { self.chip_rom.data.add(rom1_offset) }, rom1_len as u32, REGION_DRAM1_BASE);
        }
    }

    pub fn write_flash(&mut self, data: &[u8], offset: u32) {
        if self.flash.is_null() || offset as usize + data.len() > self.flash_len as usize { return; }
        unsafe { ptr::copy_nonoverlapping(data.as_ptr(), self.flash.add(offset as usize), data.len()); }
    }

    pub fn flash_subarray(&self, start: u32, end: u32) -> &[u8] {
        if self.flash.is_null() { return &[]; }
        let s = core::cmp::min(start, self.flash_len) as usize;
        let e = core::cmp::min(end, self.flash_len) as usize;
        if e <= s { return &[]; }
        unsafe { core::slice::from_raw_parts(self.flash.add(s), e - s) }
    }

    pub fn flash_subarray_mut(&mut self, start: u32, end: u32) -> &mut [u8] {
        if self.flash.is_null() { return &mut []; }
        let s = core::cmp::min(start, self.flash_len) as usize;
        let e = core::cmp::min(end, self.flash_len) as usize;
        if e <= s { return &mut []; }
        unsafe { core::slice::from_raw_parts_mut(self.flash.add(s), e - s) }
    }

    // JS: mapAddress(cpuVal, tmpVal) — lines 937-982
    pub fn map_address(&self, addr: u32, is_app: bool) -> u32 {
        let _ = is_app;
        if addr >= GPIO_BASE_ADDR_ALT && addr < REGION_DRAM1_BASE {
            if self.find_peripheral_by_addr(addr) != 0xFFFFFFFF { return 1; }
        }
        if addr >= REGION_PERI_BUS_BASE && addr <= REGION_USB_BASE {
            if self.find_peripheral_by_addr(addr) != 0xFFFFFFFF { return 1; }
        }
        if addr >= REGION_RTC_SLOW_BASE && addr < REGION_CODE_BASE { return 2; }
        if addr >= REGION_DRAM1_BASE && addr < REGION_RTC_SLOW_BASE { return 3; }
        if addr >= REGION_CODE_BASE && addr < REGION_CODE_BASE + IRAM0_SIZE { return 4; }
        if addr >= REGION_FLASH_CACHE_BASE && addr < REGION_IROM0_BASE { return 5; }
        if addr >= REGION_IRAM0_BASE && addr < REGION_IRAM1_BASE_ALT { return 6; }
        if addr >= REGION_DROM0_BASE && addr < REGION_DROM1_BASE { return 7; }
        if addr >= REGION_IROM0_BASE_ALT && addr <= REGION_IRAM1_BASE { return 8; }
        let _page = addr >> REGION_CACHE_ALIGN_SIZE;
        if addr >= REGION_PERI1_BASE && addr < REGION_PERI2_BASE { return 10; }
        if addr >= USB_OTG_BASE_ADDR && addr < USB_OTG_BASE_ADDR + RTC_SLOW_SIZE { return 11; }
        if ESP32_YN { /* console.log equivalent skipped */ }
        0xFFFFFFFF
    }

    pub fn read_memory_u32(&self, addr: u32, is_app: bool) -> u32 {
        match self.map_address(addr, is_app) {
            2 => self.data_mem.read_u32(addr),
            3 => self.rom1.read_u32(addr),
            4 => self.chip_rom.read_u32(addr),
            5 => self.iram.read_u32(addr),
            6 => self.rtc_fast_mem.read_u32(addr),
            7 => self.psram_memory.read_u32(addr),
            8 => self.sram1_reverse.read_u32(addr),
            10 => self.mmu_table_memory.read_u32(addr),
            11 => self.rtc_slow_mem.read_u32(addr),
            _ => 0,
        }
    }

    pub fn write_memory_u32(&mut self, addr: u32, val: u32, is_app: bool) {
        match self.map_address(addr, is_app) {
            2 => self.data_mem.write_u32(addr, val),
            3 => self.rom1.write_u32(addr, val),
            5 => self.iram.write_u32(addr, val),
            6 => self.rtc_fast_mem.write_u32(addr, val),
            7 => { let _ = self.psram_memory.write_u32(addr, val); }
            8 => self.sram1_reverse.write_u32(addr, val),
            10 => self.mmu_table_memory.write_u32(addr, val),
            11 => self.rtc_slow_mem.write_u32(addr, val),
            _ => {}
        }
    }

    // JS: reset() — lines 919-929
    pub fn reset_all(&mut self) {
        self.mmu_table_pro_fill(256);
        self.mmu_table_app_fill(256);
    }

    fn mmu_table_pro_fill(&self, val: u32) {
        if self.mmu_table_pro.is_null() { return; }
        for i in 0..(REGION_CACHE_LINE_SIZE as usize) {
            unsafe { *self.mmu_table_pro.add(i) = val; }
        }
    }

    fn mmu_table_app_fill(&self, val: u32) {
        if self.mmu_table_app.is_null() { return; }
        for i in 0..(REGION_DROM_SIZE as usize) {
            unsafe { *self.mmu_table_app.add(i) = val; }
        }
    }

    // JS: resetPeripheral(cpuVal, tmpVal) — lines 930-936
    pub fn reset_peripheral(&mut self, addr: u32, do_reset: bool) {
        if !do_reset { return; }
        let _found = self.find_peripheral_by_addr(addr);
    }

    // JS: _initWasmMemory() — lines 817-859
    pub fn init_wasm_memory(&mut self) {
        let region_lens = [
            self.data_mem.len,
            self.chip_rom.len,
            self.rtc_fast_mem.len,
            self.rtc_slow_mem.len,
            self.rom1.len,
            self.mmu_table_memory.len,
        ];
        let region_bases = [
            self.data_mem.base,
            self.chip_rom.base,
            self.rtc_fast_mem.base,
            self.rtc_slow_mem.base,
            self.rom1.base,
            self.mmu_table_memory.base,
        ];
        let region_datas = [
            self.data_mem.data,
            self.chip_rom.data,
            self.rtc_fast_mem.data,
            self.rtc_slow_mem.data,
            self.rom1.data,
            self.mmu_table_memory.data,
        ];
        let mut cum = 0u32;
        for i in 0..6 {
            self.mem_region_offsets[i] = cum;
            cum = cum.wrapping_add(region_lens[i]);
        }
        let ram_bytes = cum;
        let needed_bytes = WASM_RAM_DATA_OFFSET + ram_bytes;
        let pages = (needed_bytes + 65535) / 65536;
        let handle = unsafe { ffi::js_create_wasm_memory(pages, pages) };
        let wasm_mem_ptr = unsafe { ffi::js_get_memory_buffer(handle) };
        let wasm_mem_len = unsafe { ffi::js_get_memory_length(handle) };
        self.wasm_memory = wasm_mem_ptr;
        self.wasm_memory_len = wasm_mem_len;
        if wasm_mem_ptr.is_null() { return; }
        for i in 0..6 {
            let off = WASM_RAM_DATA_OFFSET + self.mem_region_offsets[i];
            if region_lens[i] > 0 {
                unsafe {
                    core::ptr::copy_nonoverlapping(
                        region_datas[i],
                        wasm_mem_ptr.add(off as usize),
                        region_lens[i] as usize,
                    );
                }
            }
        }
        // Reassign region data pointers into WASM linear memory (JS: regions[i].data = new Uint8Array(this._wasmMemory, off, ...))
        self.data_mem.data = unsafe { wasm_mem_ptr.add((WASM_RAM_DATA_OFFSET + self.mem_region_offsets[0]) as usize) };
        self.chip_rom.data = unsafe { wasm_mem_ptr.add((WASM_RAM_DATA_OFFSET + self.mem_region_offsets[1]) as usize) };
        self.rtc_fast_mem.data = unsafe { wasm_mem_ptr.add((WASM_RAM_DATA_OFFSET + self.mem_region_offsets[2]) as usize) };
        self.rtc_slow_mem.data = unsafe { wasm_mem_ptr.add((WASM_RAM_DATA_OFFSET + self.mem_region_offsets[3]) as usize) };
        self.rom1.data = unsafe { wasm_mem_ptr.add((WASM_RAM_DATA_OFFSET + self.mem_region_offsets[4]) as usize) };
        self.mmu_table_memory.data = unsafe { wasm_mem_ptr.add((WASM_RAM_DATA_OFFSET + self.mem_region_offsets[5]) as usize) };
        let rt_base = (WASM_REGION_TABLE_OFFSET >> 2) as usize;
        let u32_ptr = wasm_mem_ptr as *mut u32;
        for i in 0..6 {
            unsafe {
                *u32_ptr.add(rt_base + i * 2) = region_bases[i];
                *u32_ptr.add(rt_base + i * 2 + 1) = self.mem_region_offsets[i];
            }
        }
        let pt_src = self.page_table.table;
        let pt_offset = WASM_PAGE_TABLE_OFFSET as usize;
        let pt_entries = (WASM_PAGE_TABLE_BYTE_SIZE / 4) as usize;
        if !pt_src.is_null() {
            unsafe {
                core::ptr::copy_nonoverlapping(pt_src, u32_ptr.add(pt_offset / 4), pt_entries);
            }
        }
        self.page_table = PageTable::new(
            unsafe { u32_ptr.add(pt_offset / 4) },
            WASM_PAGE_TABLE_BYTE_SIZE / 4 / 2,
        );
        self.rom1 = self.chip_rom.create_view(
            REGION_DRAM1_BASE,
            REGION_DRAM1_BASE.wrapping_sub(REGION_CODE_BASE),
            65536,
        );
        if !self.data_mem.data.is_null() {
            let rev_data = unsafe { self.data_mem.data.add(IRAM1_SIZE as usize) };
            self.sram1_reverse = Memory::new(
                rev_data,
                DROM0_CACHE_SIZE,
                REGION_IROM0_BASE_ALT,
            );
        }
        if !self.mmu_table_memory.data.is_null() {
            let drom_size = REGION_DROM_SIZE as usize;
            let mmu_data = self.mmu_table_memory.data;
            self.mmu_table_pro = mmu_data as *mut u32;
            self.mmu_table_app = unsafe { mmu_data.add(drom_size * 4) as *mut u32 };
        }
        let sab_ptr = unsafe { ffi::js_create_sab(4096) };
        self.sab_u32 = sab_ptr as *mut u32;
        self.sab_u32_valid = true;
    }

    // JS: loadWasm() — lines 862-917
    pub fn load_wasm(&mut self, wasm_bytes: &[u8], enabled: &str) -> bool {
        if self.wasm_memory.is_null() {
            self.init_wasm_memory();
        }
        if self.wasm_memory.is_null() {
            return false;
        }
        let ok0 = unsafe { ffi::js_load_wasm(wasm_bytes.as_ptr(), wasm_bytes.len(), 0) };
        if ok0 < 0 { return false; }
        if self.cores.len() > 1 {
            let ok1 = unsafe { ffi::js_load_wasm(wasm_bytes.as_ptr(), wasm_bytes.len(), 1) };
            if ok1 < 0 { return false; }
        }
        self.wasm_cores_loaded = true;
        self.engine_mode = 1;
        true
    }

    // JS: step() — lines 983-1011
    pub fn step(&mut self) {
        if self.wasm_cores_loaded {
            unsafe { ffi::js_step_core(0, self.cycles_per_step); }
            if self.cores[1].enabled {
                unsafe { ffi::js_step_core(1, self.cycles_per_step); }
            }
        } else {
            self.cores[0].run_instruction();
            if self.cores[1].enabled {
                self.cores[1].run_instruction();
            }
        }
        self.cycles = self.cycles.wrapping_add(1);
        crate::native_mmio::advance_clock(1);
        if self.sab_u32_valid {
            let sab = self.sab_u32;
            unsafe {
                let cpu_ticks = self.clock_tree.clocks[CLK_CPU].base_ticks as u32;
                *sab.add(998) = cpu_ticks;
                *sab.add(999) = self.cycles as u32;
            }
        }
    }

    // JS: get coresIdle — lines 1012-1020
    pub fn cores_idle(&self) -> bool {
        self.cores[0].idle && !self.cores[0].pending_interrupts
            && self.cores[1].idle && !self.cores[1].pending_interrupts
    }

    // JS: interrupt(cpuVal, tmpVal, idxVal) — lines 1021-1025
    pub fn interrupt_cpu(&mut self, irq: u32, level: bool, dir: u32) {
        if ESP_KEY { /* console.log equivalent */ }
        if dir & 1 != 0 { let _ = irq; let _ = level; } // intMatrix[0].interrupt
        if dir & 2 != 0 { let _ = irq; let _ = level; } // intMatrix[1].interrupt
    }

    // JS: buildPageTable() — lines 1026-1073
    pub fn build_page_table(&mut self) {
        if self.page_table.table.is_null() { return; }
        self.build_page_table_full();
    }

    // JS: get camera — lines 1074-1076
    pub fn camera(&self) -> u32 { self.camera_full() }

    // JS: get devices — lines 1077-1089
    pub fn get_devices(&self) -> Devices<'_> { self.get_devices_full() }

    // JS: lines 772-781 — MmuPageTableConfig loop to build flashMMUMap
    pub fn build_flash_mmu_map(&mut self) {
        for config in MMU_PAGE_TABLE_CONFIG {
            let start_page = config.start >> REGION_CACHE_ALIGN_SIZE;
            for i in 0..config.pages {
                let key = ((start_page + i) as usize) & 0x1FF;
                self.flash_mmu_map[key] = (config.index + i) as i32;
            }
        }
    }

    // JS: lines 762-771 — peripheralMap build with MemoryTranslator
    pub fn build_peripheral_index_map(&mut self) {
        for i in 0..self.peripheral_count as usize {
            let base = self.peripheral_entries[i].base_addr;
            let key = (base >> 12) as usize;
            if key < 512 {
                self.peripheral_index_map[key] = i as i32;
            }
            let alias = base.wrapping_add(REGION_DROM0_MAP_BASE);
            if alias >= REGION_PERI_BUS_BASE && alias < REGION_USB_BASE {
                let alias_key = (alias >> 12) as usize;
                if alias_key < 512 {
                    self.peripheral_index_map[alias_key] = i as i32;
                }
            }
        }
    }

    // JS: lines 966-975 — flashMMUMap.get + mmu table lookup
    pub fn mmu_page_lookup(&self, addr: u32, is_app: bool) -> i32 {
        let key = ((addr >> REGION_CACHE_ALIGN_SIZE) as usize) & 0x1FF;
        let idx = self.flash_mmu_map[key];
        if idx < 0 { return -1; }
        let table = if is_app { self.mmu_table_app } else { self.mmu_table_pro };
        if table.is_null() { return -2; }
        let phy_addr = unsafe { *table.add(idx as usize) };
        if phy_addr & 256 != 0 { return -3; }
        (phy_addr << REGION_CACHE_ALIGN_SIZE) as i32
    }

    // JS: lines 960-963 — PSRAM with MMU table
    pub fn psram_page_lookup(&self, addr: u32, is_app: bool) -> i32 {
        let offset = (addr - REGION_DROM0_BASE) >> 15;
        let table = if is_app { self.mmu_table_app } else { self.mmu_table_pro };
        if table.is_null() { return -2; }
        let idx = (1152 + offset) as usize;
        let max = if is_app { REGION_DROM_SIZE as usize } else { REGION_CACHE_LINE_SIZE as usize };
        if idx >= max { return -3; }
        let phy_addr = unsafe { *table.add(idx) };
        if phy_addr & 256 != 0 { return -4; }
        (phy_addr << 15) as i32
    }

    // JS: lines 937-982 — full mapAddress with flash MMU, PSRAM, MMU table
    pub fn map_address_full(&self, addr: u32, is_app: bool) -> i32 {
        let key = (addr >> 12) as usize;
        if key < 512 && self.peripheral_index_map[key] >= 0 { return 1; }
        if addr >= REGION_RTC_SLOW_BASE && addr < REGION_CODE_BASE { return 2; }
        if addr >= REGION_DRAM1_BASE && addr < REGION_RTC_SLOW_BASE { return 3; }
        if addr >= REGION_CODE_BASE && addr < REGION_CODE_BASE + IRAM0_SIZE { return 4; }
        if addr >= REGION_FLASH_CACHE_BASE && addr < REGION_IROM0_BASE { return 5; }
        if addr >= REGION_IRAM0_BASE && addr < REGION_IRAM1_BASE_ALT { return 6; }
        if addr >= REGION_DROM0_BASE && addr < REGION_DROM1_BASE {
            let _phy = self.psram_page_lookup(addr, is_app);
            if _phy >= 0 { return 7; }
        }
        if addr >= REGION_IROM0_BASE_ALT && addr <= REGION_IRAM1_BASE { return 8; }
        let _phy = self.mmu_page_lookup(addr, is_app);
        if _phy >= 0 { return 9; }
        if addr >= REGION_PERI1_BASE && addr < REGION_PERI2_BASE { return 10; }
        if addr >= USB_OTG_BASE_ADDR && addr < USB_OTG_BASE_ADDR + RTC_SLOW_SIZE { return 11; }
        -1
    }

    // JS: reset() — lines 919-929
    pub fn reset_full(&mut self) {
        for core in self.cores.iter_mut() {
            core.reset();
        }
        self.cores[1].enabled = true;
        for entry in ESP32_FULL_RESET_VALUES {
            let base = entry.base_addr;
            for quad in entry.entries {
                let offset = quad[0];
                let val = quad[1];
                let count = quad[2];
                let stride = quad[3];
                for j in 0..count {
                    self.cores[0].write_register(base + offset + j * stride, val);
                }
            }
        }
        self.cores[0].write_register(IO_MUX_BASE_ADDR, 1023);
        self.mmu_table_pro_fill(256);
        self.mmu_table_app_fill(256);
        self.cycles = 0;
        self.reset_reason = 1;
    }

    // JS: reset peripheral loop — lines 919-929 (for (let e of this.peripherals) e.reset())
    pub fn reset_peripheral_loop(&mut self) {
        self.gpio.reset();
        self.dport.reset();
        self.aes.reset();
        self.sha.reset();
        self.rsa.reset();
        self.wifi.reset();
        self.sdmmc.reset();
        // StubPeripheral has no reset — skip
        self.frc_timer.reset();
        self.rtc.reset();
        self.rtc_io_0.reset();
        self.rtc_io_1.reset();
        self.adc.reset();
        self.io_mux.reset();
        self.analog_rf.reset();
        self.rmt.reset();
        self.pcnt.reset();
        self.sdio_slave.reset();
        self.ledc.reset();
        self.efuse.reset();
        // Timer groups: use qualified trait call to avoid inherent method
        <TimerGroupPeripheral as MmioPeripheral>::reset(&mut self.timer_group0);
        <TimerGroupPeripheral as MmioPeripheral>::reset(&mut self.timer_group1);
        self.syscon.reset();
        self.rng.reset();
        if let Some(ref mut u) = self.uart0 { u.reset(); }
        if let Some(ref mut u) = self.uart1 { u.reset(); }
        if let Some(ref mut u) = self.uart2 { u.reset(); }
        for i in 0..2 { self.spi[i].reset(); }
        for i in 0..2 { self.i2c[i].reset(); }
        for i in 0..2 { self.i2s[i].reset(); }
        for i in 0..1 { self.twai[i].reset(); }
    }

    // JS: resetPeripheral(cpuVal, tmpVal) — lines 930-936
    pub fn reset_peripheral_full(&mut self, addr: u32, do_reset: bool) {
        if !do_reset { return; }
        let mut found = false;
        for entry in ESP32_FULL_RESET_VALUES {
            if entry.base_addr == addr {
                found = true;
                for quad in entry.entries {
                    let offset = quad[0];
                    let val = quad[1];
                    let count = quad[2];
                    let stride = quad[3];
                    for j in 0..count {
                        self.cores[0].write_register(addr + offset + j * stride, val);
                    }
                }
                break;
            }
        }
        if !found {
            let _ = self.find_peripheral_by_addr(addr);
        }
    }

    // JS: step() — lines 983-1011
    pub fn step_full(&mut self) {
        // Run instructions on both cores (WASM or JS — same cores in Rust)
        self.cores[0].run_instruction();
        if self.cores[1].enabled {
            self.cores[1].run_instruction();
        }
        self.cycles = self.cycles.wrapping_add(1);
        // Write CPU tick/cycle counts to SAB (avoids FFI per step)
        // Stored in core0 _pad[0] (offset 3992) and _pad[1] (offset 3996)
        if self.sab_u32_valid {
            let sab = self.sab_u32;
            unsafe {
                let cpu_ticks = self.clock_tree.clocks[CLK_CPU].base_ticks as u32;
                *sab.add(998) = cpu_ticks;
                *sab.add(999) = self.cycles as u32;
            }
        }
    }

    // JS: get coresIdle — lines 1012-1020
    pub fn cores_idle_full(&self) -> bool {
        self.cores[0].idle && !self.cores[0].pending_interrupts
            && self.cores[1].idle && !self.cores[1].pending_interrupts
    }

    // JS: interrupt(cpuVal, tmpVal, idxVal) — lines 1021-1025
    pub fn interrupt_cpu_full(&mut self, ctx: &mut CpuContext, irq: u32, level: bool, dir: u32) {
        if dir & 1 != 0 {
            self.dport.int_matrix[0].interrupt(ctx, irq, level);
        }
        if dir & 2 != 0 {
            self.dport.int_matrix[1].interrupt(ctx, irq, level);
        }
    }

    // JS: buildPageTable() — lines 1026-1073
    pub fn build_page_table_full(&mut self) {
        if self.page_table.table.is_null() { return; }
        let mut shared_pages = [false; 512];
        let mut page_perif_count = [0u32; 512];
        let mut page_has_aligned = [false; 512];
        let mut page_has_non_aligned = [false; 512];
        for i in 0..self.peripheral_count as usize {
            let page = (self.peripheral_entries[i].base_addr >> 12) as usize;
            if page < 512 {
                page_perif_count[page] += 1;
                if self.peripheral_entries[i].base_addr & 0xFFF == 0 {
                    page_has_aligned[page] = true;
                } else {
                    page_has_non_aligned[page] = true;
                }
            }
        }
        for page in 0..512 {
            if page_has_aligned[page] && page_has_non_aligned[page] && page_perif_count[page] > 1 {
                shared_pages[page] = true;
            }
        }
        for i in 0..self.peripheral_count as usize {
            let entry = &self.peripheral_entries[i];
            if entry.base_addr & 0xFFF != 0 { continue; }
            let page = (entry.base_addr >> 12) as usize;
            if page < 512 && shared_pages[page] { continue; }
            let handler_id = self.mmio_handlers.register(MMIOHandler::empty());
            self.page_table.set_range(entry.base_addr, 0x1000, PTE_TYPE_MMIO, handler_id);
            let alias = entry.base_addr.wrapping_add(REGION_DROM0_MAP_BASE);
            if alias >= REGION_PERI_BUS_BASE && alias < REGION_USB_BASE {
                let alias_handler_id = self.mmio_handlers.register(MMIOHandler::empty());
                self.page_table.set_range(alias, 0x1000, PTE_TYPE_MMIO, alias_handler_id);
            }
        }
        let regions = [&self.data_mem, &self.chip_rom, &self.rtc_fast_mem, &self.rtc_slow_mem, &self.rom1, &self.mmu_table_memory];
        for (i, m) in regions.iter().enumerate() {
            if m.len > 0 {
                self.page_table.set_range(m.base, m.len, PTE_TYPE_RAM, i as u32);
            }
        }
    }

    // JS: get camera — lines 1074-1076
    pub fn camera_full(&self) -> u32 { 0 }

    // JS: get devices — lines 1077-1089
    pub fn get_devices_full(&self) -> Devices<'_> {
        let flash = if self.flash.is_null() { &[] } else { unsafe { core::slice::from_raw_parts(self.flash, self.flash_len as usize) } };
        let iram = if self.iram.data.is_null() { &[] } else { unsafe { core::slice::from_raw_parts(self.iram.data, self.iram.len as usize) } };
        let dram = if self.data_mem.data.is_null() { &[] } else { unsafe { core::slice::from_raw_parts(self.data_mem.data, self.data_mem.len as usize) } };
        let rtc_fast = if self.rtc_fast_mem.data.is_null() { &[] } else { unsafe { core::slice::from_raw_parts(self.rtc_fast_mem.data, self.rtc_fast_mem.len as usize) } };
        let rtc_slow = if self.rtc_slow_mem.data.is_null() { &[] } else { unsafe { core::slice::from_raw_parts(self.rtc_slow_mem.data, self.rtc_slow_mem.len as usize) } };
        let mmu = if self.mmu_table_memory.data.is_null() { &[] } else { unsafe { core::slice::from_raw_parts(self.mmu_table_memory.data, self.mmu_table_memory.len as usize) } };
        let psram = if self.psram_buf.is_null() { &[] } else { unsafe { core::slice::from_raw_parts(self.psram_buf, self.psram_len as usize) } };
        Devices { flash, iram, dram, rtc_fast, rtc_slow, mmu, psram }
    }

    // JS: loadWasm() — lines 830-917
    pub fn load_wasm_full(&mut self, wasm_bytes: &[u8], enabled: &str) -> bool {
        self.load_wasm(wasm_bytes, enabled)
    }

    pub fn init_uarts(&mut self, ctx: &mut CpuContext) {
        let uart_cfg = UartConfig {
            has_tx_state: false,
            tout_multiply: false,
            reg_map: UartRegisterMap {
                autobaud: UART_REG_AUTOBAUD,
                reg_update: UART_REG_UPDATE,
                id: UART_REG_ID,
                at_cmd_char: UART_REG_AT_CMD_CHAR,
                mem_rx_status: UART_REG_MEM_RX_STATUS,
                rxd_cnt: UART_REG_RXD_CNT,
                lowpulse: UART_REG_LOWPULSE,
                highpulse: UART_REG_HIGHPULSE,
                negpulse: UART_REG_NEGPULSE,
                pospulse: UART_REG_POSPULSE,
                rx_filt: UART_REG_RX_FILT,
                clk_conf: UART_REG_CLK_CONF,
            },
            fields: UartFieldMap {
                rxfifo_rst: UART_FIELD_RXFIFO_RST,
                txfifo_rst: UART_FIELD_TXFIFO_RST,
                loopback: UART_FIELD_LOOPBACK,
                tx_flow_en: UART_FIELD_TX_FLOW_EN,
                irda_en: UART_FIELD_IRDA_EN,
                irda_tx_en: UART_FIELD_IRDA_TX_EN,
                lowpulse_min_cnt: UART_FIELD_LOWPULSE_MIN_CNT,
                highpulse_min_cnt: UART_FIELD_HIGHPULSE_MIN_CNT,
                negedge_min_cnt: UART_FIELD_NEGEDGE_MIN_CNT,
                posedge_min_cnt: UART_FIELD_POSEDGE_MIN_CNT,
                rx_tout_thrhd: UART_FIELD_RX_TOUT_THRHD,
                rxfifo_full_thrhd: UART_FIELD_RXFIFO_FULL_THRHD,
                txfifo_empty_thrhd: UART_FIELD_TXFIFO_EMPTY_THRHD,
                autobaud_en: None,
                sclk_sel: None,
                sclk_div_num: None,
                glitch_filt: None,
                glitch_filt_en: None,
            },
            clock_source_frequency: None,
        };
        self.uart0 = Some(UartController::new(ctx, SDMMC_ALT_BASE_ADDR, "UART0", 0, UART_INTR, uart_cfg));
        self.uart1 = Some(UartController::new(ctx, UHCI_BASE_ADDR, "UART1", 1, UART1_INTR, uart_cfg));
        self.uart2 = Some(UartController::new(ctx, UHCI_ALT_BASE_ADDR, "UART2", 2, UART2_INTR, uart_cfg));
    }

    // JS line 811-813
    pub fn load_rom(&mut self, data: &[u8]) {
        let offset = (REGION_CODE_BASE - REGION_CODE_BASE) as usize;
        if self.chip_rom.len == 0 || data.len() > self.chip_rom.len as usize { return; }
        let dest = unsafe { core::slice::from_raw_parts_mut(self.chip_rom.data.add(offset), data.len()) };
        dest.copy_from_slice(data);
        let rom1_offset = REGION_DRAM1_BASE.wrapping_sub(REGION_CODE_BASE) as usize;
        if data.len() > rom1_offset {
            let rom1_len = core::cmp::min(65536_usize, data.len().saturating_sub(rom1_offset));
            self.rom1 = Memory::new(unsafe { self.chip_rom.data.add(rom1_offset) }, rom1_len as u32, REGION_DRAM1_BASE);
        }
    }

}

impl MmioPeripheral for Esp32Peripheral {
    fn read_u32(&mut self, ctx: &mut CpuContext, addr: u32) -> u32 {
        match addr & 0xFFFFF000 {
            SDMMC_ALT_BASE_ADDR => self.uart0.as_mut().map_or(0, |u| u.read_u32(ctx, addr)),
            UHCI_BASE_ADDR => self.uart1.as_mut().map_or(0, |u| u.read_u32(ctx, addr)),
            UHCI_ALT_BASE_ADDR => self.uart2.as_mut().map_or(0, |u| u.read_u32(ctx, addr)),
            _ => self.read_memory_u32(addr, false),
        }
    }

    fn write_u32(&mut self, ctx: &mut CpuContext, addr: u32, val: u32) {
        match addr & 0xFFFFF000 {
            SDMMC_ALT_BASE_ADDR => if let Some(u) = self.uart0.as_mut() { u.write_u32(ctx, addr, val); },
            UHCI_BASE_ADDR => if let Some(u) = self.uart1.as_mut() { u.write_u32(ctx, addr, val); },
            UHCI_ALT_BASE_ADDR => if let Some(u) = self.uart2.as_mut() { u.write_u32(ctx, addr, val); },
            _ => self.write_memory_u32(addr, val, false),
        }
    }

    fn reset(&mut self) {
        self.reset_all();
    }
}
