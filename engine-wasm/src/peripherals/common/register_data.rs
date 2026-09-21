// Translated from: src/peripherals/esp32/register-data.js
// Do NOT modify the logic — match the JS line-for-line

use crate::peripherals::types::*;
use crate::peripherals::common::peripheral::FieldDesc;

// ============================================================
// GpioSignalDef — matches JS GpioSignalDefs object entries
// ============================================================

#[derive(Clone, Copy)]
pub struct GpioSignalDef {
    pub name: &'static str,
    pub peripheral: u32,
    pub index: u32,
    pub signal: &'static str,
}

// ============================================================
// PinFnEntry — matches JS mixed-type fn array entries
// ============================================================

#[derive(Clone, Copy)]
pub enum PinFnEntry {
    Gpio,
    Named(u32),
    Signal(GpioSignalDef),
    Null,
}

// ============================================================
// GpioPinConfig — matches JS Esp32GpioPinConfigs entries
// ============================================================

#[derive(Clone, Copy)]
pub struct GpioPinConfig {
    pub simulation_clock: u32,
    pub fn_entries: [PinFnEntry; 6],
    pub reset: u32,
    pub flags: Option<&'static str>,
}

// ============================================================
// ResetEntry — matches JS [offset, value, repeat, stride]
// ============================================================

pub type ResetEntry = [u32; 4];

// ============================================================
// FullResetEntry — matches JS [base_addr, [...reset_entries]]
// ============================================================

#[derive(Clone, Copy)]
pub struct FullResetEntry {
    pub base_addr: u32,
    pub entries: &'static [ResetEntry],
}

// ============================================================
// Memory region constants (JS PeripheralType value for SPI)
// ============================================================

pub const SPI_PERIPHERAL_TYPE: u32 = 1;
pub const I2C_PERIPHERAL_TYPE: u32 = 2;
pub const UART_PERIPHERAL_TYPE: u32 = 3;
pub const PCNT_PERIPHERAL_TYPE: u32 = 5;

// ============================================================
// GpioSignalDefs constants: destructured signals
// ============================================================

pub const GPIO_DEF: &str = "GPIO";

pub const U0TXD_DEF: GpioSignalDef = GpioSignalDef {
    name: "U0TXD",
    peripheral: UART_PERIPHERAL_TYPE,
    index: 0,
    signal: "rsaReg26",
};

pub const U0RXD_DEF: GpioSignalDef = GpioSignalDef {
    name: "U0RXD",
    peripheral: UART_PERIPHERAL_TYPE,
    index: 0,
    signal: "rsaReg57",
};

pub const U0RTS_DEF: GpioSignalDef = GpioSignalDef {
    name: "U0RTS",
    peripheral: UART_PERIPHERAL_TYPE,
    index: 0,
    signal: "RTS",
};

pub const U0CTS_DEF: GpioSignalDef = GpioSignalDef {
    name: "U0CTS",
    peripheral: UART_PERIPHERAL_TYPE,
    index: 0,
    signal: "CTS",
};

pub const U1TXD_DEF: GpioSignalDef = GpioSignalDef {
    name: "U1TXD",
    peripheral: UART_PERIPHERAL_TYPE,
    index: 1,
    signal: "rsaReg26",
};

pub const U1RXD_DEF: GpioSignalDef = GpioSignalDef {
    name: "U1RXD",
    peripheral: UART_PERIPHERAL_TYPE,
    index: 1,
    signal: "rsaReg57",
};

pub const U1RTS_DEF: GpioSignalDef = GpioSignalDef {
    name: "U1RTS",
    peripheral: UART_PERIPHERAL_TYPE,
    index: 1,
    signal: "RTS",
};

pub const U1CTS_DEF: GpioSignalDef = GpioSignalDef {
    name: "U1CTS",
    peripheral: UART_PERIPHERAL_TYPE,
    index: 1,
    signal: "CTS",
};

pub const U2TXD_DEF: GpioSignalDef = GpioSignalDef {
    name: "U2TXD",
    peripheral: UART_PERIPHERAL_TYPE,
    index: 2,
    signal: "rsaReg26",
};

pub const U2RXD_DEF: GpioSignalDef = GpioSignalDef {
    name: "U2RXD",
    peripheral: UART_PERIPHERAL_TYPE,
    index: 2,
    signal: "rsaReg57",
};

pub const U2RTS_DEF: GpioSignalDef = GpioSignalDef {
    name: "U2RTS",
    peripheral: UART_PERIPHERAL_TYPE,
    index: 2,
    signal: "RTS",
};

pub const U2CTS_DEF: GpioSignalDef = GpioSignalDef {
    name: "U2CTS",
    peripheral: UART_PERIPHERAL_TYPE,
    index: 2,
    signal: "CTS",
};

pub const SPICS0_DEF: GpioSignalDef = GpioSignalDef {
    name: "SPICS0",
    peripheral: SPI_PERIPHERAL_TYPE,
    index: 1,
    signal: "CS0",
};

pub const SPID_DEF: GpioSignalDef = GpioSignalDef {
    name: "SPID",
    peripheral: SPI_PERIPHERAL_TYPE,
    index: 1,
    signal: "MOSI",
};

pub const SPICLK_DEF: GpioSignalDef = GpioSignalDef {
    name: "SPICLK",
    peripheral: SPI_PERIPHERAL_TYPE,
    index: 1,
    signal: "CLK",
};

pub const SPIQ_DEF: GpioSignalDef = GpioSignalDef {
    name: "SPIQ",
    peripheral: SPI_PERIPHERAL_TYPE,
    index: 1,
    signal: "MISO",
};

pub const SPIHD_DEF: GpioSignalDef = GpioSignalDef {
    name: "SPIHD",
    peripheral: SPI_PERIPHERAL_TYPE,
    index: 1,
    signal: "HD",
};

pub const SPIWP_DEF: GpioSignalDef = GpioSignalDef {
    name: "SPIWP",
    peripheral: SPI_PERIPHERAL_TYPE,
    index: 1,
    signal: "WP",
};

pub const HSPICS0_DEF: GpioSignalDef = GpioSignalDef {
    name: "HSPICS0",
    peripheral: SPI_PERIPHERAL_TYPE,
    index: 2,
    signal: "CS0",
};

pub const HSPID_DEF: GpioSignalDef = GpioSignalDef {
    name: "HSPID",
    peripheral: SPI_PERIPHERAL_TYPE,
    index: 2,
    signal: "MOSI",
};

pub const HSPICLK_DEF: GpioSignalDef = GpioSignalDef {
    name: "HSPICLK",
    peripheral: SPI_PERIPHERAL_TYPE,
    index: 2,
    signal: "CLK",
};

pub const HSPIQ_DEF: GpioSignalDef = GpioSignalDef {
    name: "HSPIQ",
    peripheral: SPI_PERIPHERAL_TYPE,
    index: 2,
    signal: "MISO",
};

pub const HSPIHD_DEF: GpioSignalDef = GpioSignalDef {
    name: "HSPIHD",
    peripheral: SPI_PERIPHERAL_TYPE,
    index: 2,
    signal: "HD",
};

pub const HSPIWP_DEF: GpioSignalDef = GpioSignalDef {
    name: "HSPIWP",
    peripheral: SPI_PERIPHERAL_TYPE,
    index: 2,
    signal: "WP",
};

pub const VSPICS0_DEF: GpioSignalDef = GpioSignalDef {
    name: "VSPICS0",
    peripheral: SPI_PERIPHERAL_TYPE,
    index: 3,
    signal: "CS0",
};

pub const VSPID_DEF: GpioSignalDef = GpioSignalDef {
    name: "VSPID",
    peripheral: SPI_PERIPHERAL_TYPE,
    index: 3,
    signal: "MOSI",
};

pub const VSPICLK_DEF: GpioSignalDef = GpioSignalDef {
    name: "VSPICLK",
    peripheral: SPI_PERIPHERAL_TYPE,
    index: 3,
    signal: "CLK",
};

pub const VSPIQ_DEF: GpioSignalDef = GpioSignalDef {
    name: "VSPIQ",
    peripheral: SPI_PERIPHERAL_TYPE,
    index: 3,
    signal: "MISO",
};

pub const VSPIHD_DEF: GpioSignalDef = GpioSignalDef {
    name: "VSPIHD",
    peripheral: SPI_PERIPHERAL_TYPE,
    index: 3,
    signal: "HD",
};

pub const VSPIWP_DEF: GpioSignalDef = GpioSignalDef {
    name: "VSPIWP",
    peripheral: SPI_PERIPHERAL_TYPE,
    index: 3,
    signal: "WP",
};

// ============================================================
// Esp32GpioPinConfigs — pin function tables
// ============================================================

pub static ESP32_GPIO_PIN_CONFIGS: &[GpioPinConfig] = &[
    GpioPinConfig {
        simulation_clock: 0,
        fn_entries: [
            PinFnEntry::Gpio,
            PinFnEntry::Named(100),
            PinFnEntry::Gpio,
            PinFnEntry::Null,
            PinFnEntry::Null,
            PinFnEntry::Named(101),
        ],
        reset: 3,
        flags: Some("R"),
    },
    GpioPinConfig {
        simulation_clock: 1,
        fn_entries: [
            PinFnEntry::Signal(U0TXD_DEF),
            PinFnEntry::Named(102),
            PinFnEntry::Gpio,
            PinFnEntry::Null,
            PinFnEntry::Null,
            PinFnEntry::Named(103),
        ],
        reset: 3,
        flags: None,
    },
    GpioPinConfig {
        simulation_clock: 2,
        fn_entries: [
            PinFnEntry::Gpio,
            PinFnEntry::Signal(HSPIWP_DEF),
            PinFnEntry::Gpio,
            PinFnEntry::Named(104),
            PinFnEntry::Named(105),
            PinFnEntry::Null,
        ],
        reset: 2,
        flags: Some("R"),
    },
    GpioPinConfig {
        simulation_clock: 3,
        fn_entries: [
            PinFnEntry::Signal(U0RXD_DEF),
            PinFnEntry::Named(106),
            PinFnEntry::Gpio,
            PinFnEntry::Null,
            PinFnEntry::Null,
            PinFnEntry::Null,
        ],
        reset: 3,
        flags: None,
    },
    GpioPinConfig {
        simulation_clock: 4,
        fn_entries: [
            PinFnEntry::Gpio,
            PinFnEntry::Signal(HSPIHD_DEF),
            PinFnEntry::Gpio,
            PinFnEntry::Named(107),
            PinFnEntry::Named(108),
            PinFnEntry::Named(109),
        ],
        reset: 2,
        flags: Some("R"),
    },
    GpioPinConfig {
        simulation_clock: 5,
        fn_entries: [
            PinFnEntry::Gpio,
            PinFnEntry::Signal(VSPICS0_DEF),
            PinFnEntry::Gpio,
            PinFnEntry::Named(110),
            PinFnEntry::Null,
            PinFnEntry::Named(111),
        ],
        reset: 3,
        flags: None,
    },
    GpioPinConfig {
        simulation_clock: 6,
        fn_entries: [
            PinFnEntry::Named(112),
            PinFnEntry::Signal(SPICLK_DEF),
            PinFnEntry::Gpio,
            PinFnEntry::Named(113),
            PinFnEntry::Signal(U1CTS_DEF),
            PinFnEntry::Null,
        ],
        reset: 3,
        flags: None,
    },
    GpioPinConfig {
        simulation_clock: 7,
        fn_entries: [
            PinFnEntry::Named(114),
            PinFnEntry::Signal(SPIQ_DEF),
            PinFnEntry::Gpio,
            PinFnEntry::Named(115),
            PinFnEntry::Signal(U2RTS_DEF),
            PinFnEntry::Null,
        ],
        reset: 3,
        flags: None,
    },
    GpioPinConfig {
        simulation_clock: 8,
        fn_entries: [
            PinFnEntry::Named(116),
            PinFnEntry::Signal(SPID_DEF),
            PinFnEntry::Gpio,
            PinFnEntry::Named(117),
            PinFnEntry::Signal(U2CTS_DEF),
            PinFnEntry::Null,
        ],
        reset: 3,
        flags: None,
    },
    GpioPinConfig {
        simulation_clock: 9,
        fn_entries: [
            PinFnEntry::Named(118),
            PinFnEntry::Signal(SPIHD_DEF),
            PinFnEntry::Gpio,
            PinFnEntry::Named(119),
            PinFnEntry::Signal(U1RXD_DEF),
            PinFnEntry::Null,
        ],
        reset: 3,
        flags: None,
    },
    GpioPinConfig {
        simulation_clock: 10,
        fn_entries: [
            PinFnEntry::Named(120),
            PinFnEntry::Signal(SPIWP_DEF),
            PinFnEntry::Gpio,
            PinFnEntry::Named(121),
            PinFnEntry::Signal(U1TXD_DEF),
            PinFnEntry::Null,
        ],
        reset: 3,
        flags: None,
    },
    GpioPinConfig {
        simulation_clock: 11,
        fn_entries: [
            PinFnEntry::Named(122),
            PinFnEntry::Signal(SPICS0_DEF),
            PinFnEntry::Gpio,
            PinFnEntry::Named(123),
            PinFnEntry::Signal(U1RTS_DEF),
            PinFnEntry::Null,
        ],
        reset: 3,
        flags: None,
    },
    GpioPinConfig {
        simulation_clock: 12,
        fn_entries: [
            PinFnEntry::Named(124),
            PinFnEntry::Signal(HSPIQ_DEF),
            PinFnEntry::Gpio,
            PinFnEntry::Named(125),
            PinFnEntry::Named(126),
            PinFnEntry::Named(127),
        ],
        reset: 2,
        flags: Some("R"),
    },
    GpioPinConfig {
        simulation_clock: 13,
        fn_entries: [
            PinFnEntry::Named(128),
            PinFnEntry::Signal(HSPID_DEF),
            PinFnEntry::Gpio,
            PinFnEntry::Named(129),
            PinFnEntry::Named(130),
            PinFnEntry::Named(131),
        ],
        reset: 2,
        flags: Some("R"),
    },
    GpioPinConfig {
        simulation_clock: 14,
        fn_entries: [
            PinFnEntry::Named(132),
            PinFnEntry::Signal(HSPICLK_DEF),
            PinFnEntry::Gpio,
            PinFnEntry::Named(133),
            PinFnEntry::Named(134),
            PinFnEntry::Named(135),
        ],
        reset: 3,
        flags: Some("R"),
    },
    GpioPinConfig {
        simulation_clock: 15,
        fn_entries: [
            PinFnEntry::Named(136),
            PinFnEntry::Signal(HSPICS0_DEF),
            PinFnEntry::Gpio,
            PinFnEntry::Named(137),
            PinFnEntry::Named(138),
            PinFnEntry::Named(139),
        ],
        reset: 3,
        flags: Some("R"),
    },
    GpioPinConfig {
        simulation_clock: 16,
        fn_entries: [
            PinFnEntry::Gpio,
            PinFnEntry::Null,
            PinFnEntry::Gpio,
            PinFnEntry::Named(140),
            PinFnEntry::Signal(U2RXD_DEF),
            PinFnEntry::Named(141),
        ],
        reset: 1,
        flags: None,
    },
    GpioPinConfig {
        simulation_clock: 17,
        fn_entries: [
            PinFnEntry::Gpio,
            PinFnEntry::Null,
            PinFnEntry::Gpio,
            PinFnEntry::Named(142),
            PinFnEntry::Signal(U2TXD_DEF),
            PinFnEntry::Named(143),
        ],
        reset: 1,
        flags: None,
    },
    GpioPinConfig {
        simulation_clock: 18,
        fn_entries: [
            PinFnEntry::Gpio,
            PinFnEntry::Signal(VSPICLK_DEF),
            PinFnEntry::Gpio,
            PinFnEntry::Named(144),
            PinFnEntry::Null,
            PinFnEntry::Null,
        ],
        reset: 1,
        flags: None,
    },
    GpioPinConfig {
        simulation_clock: 19,
        fn_entries: [
            PinFnEntry::Gpio,
            PinFnEntry::Signal(VSPIQ_DEF),
            PinFnEntry::Gpio,
            PinFnEntry::Signal(U0CTS_DEF),
            PinFnEntry::Null,
            PinFnEntry::Named(145),
        ],
        reset: 1,
        flags: None,
    },
    GpioPinConfig {
        simulation_clock: 21,
        fn_entries: [
            PinFnEntry::Gpio,
            PinFnEntry::Signal(VSPIHD_DEF),
            PinFnEntry::Gpio,
            PinFnEntry::Null,
            PinFnEntry::Null,
            PinFnEntry::Named(146),
        ],
        reset: 1,
        flags: None,
    },
    GpioPinConfig {
        simulation_clock: 22,
        fn_entries: [
            PinFnEntry::Gpio,
            PinFnEntry::Signal(VSPIWP_DEF),
            PinFnEntry::Gpio,
            PinFnEntry::Signal(U0RTS_DEF),
            PinFnEntry::Null,
            PinFnEntry::Named(147),
        ],
        reset: 1,
        flags: None,
    },
    GpioPinConfig {
        simulation_clock: 23,
        fn_entries: [
            PinFnEntry::Gpio,
            PinFnEntry::Signal(VSPID_DEF),
            PinFnEntry::Gpio,
            PinFnEntry::Named(148),
            PinFnEntry::Null,
            PinFnEntry::Null,
        ],
        reset: 1,
        flags: None,
    },
    GpioPinConfig {
        simulation_clock: 25,
        fn_entries: [
            PinFnEntry::Gpio,
            PinFnEntry::Null,
            PinFnEntry::Gpio,
            PinFnEntry::Null,
            PinFnEntry::Null,
            PinFnEntry::Named(149),
        ],
        reset: 0,
        flags: Some("R"),
    },
    GpioPinConfig {
        simulation_clock: 26,
        fn_entries: [
            PinFnEntry::Gpio,
            PinFnEntry::Null,
            PinFnEntry::Gpio,
            PinFnEntry::Null,
            PinFnEntry::Null,
            PinFnEntry::Named(150),
        ],
        reset: 0,
        flags: Some("R"),
    },
    GpioPinConfig {
        simulation_clock: 27,
        fn_entries: [
            PinFnEntry::Gpio,
            PinFnEntry::Null,
            PinFnEntry::Gpio,
            PinFnEntry::Null,
            PinFnEntry::Null,
            PinFnEntry::Named(151),
        ],
        reset: 0,
        flags: Some("R"),
    },
    GpioPinConfig {
        simulation_clock: 32,
        fn_entries: [
            PinFnEntry::Gpio,
            PinFnEntry::Null,
            PinFnEntry::Gpio,
            PinFnEntry::Null,
            PinFnEntry::Null,
            PinFnEntry::Null,
        ],
        reset: 0,
        flags: Some("R"),
    },
    GpioPinConfig {
        simulation_clock: 33,
        fn_entries: [
            PinFnEntry::Gpio,
            PinFnEntry::Null,
            PinFnEntry::Gpio,
            PinFnEntry::Null,
            PinFnEntry::Null,
            PinFnEntry::Null,
        ],
        reset: 0,
        flags: Some("R"),
    },
    GpioPinConfig {
        simulation_clock: 34,
        fn_entries: [
            PinFnEntry::Gpio,
            PinFnEntry::Null,
            PinFnEntry::Gpio,
            PinFnEntry::Null,
            PinFnEntry::Null,
            PinFnEntry::Null,
        ],
        reset: 0,
        flags: Some("RI"),
    },
    GpioPinConfig {
        simulation_clock: 35,
        fn_entries: [
            PinFnEntry::Gpio,
            PinFnEntry::Null,
            PinFnEntry::Gpio,
            PinFnEntry::Null,
            PinFnEntry::Null,
            PinFnEntry::Null,
        ],
        reset: 0,
        flags: Some("RI"),
    },
    GpioPinConfig {
        simulation_clock: 36,
        fn_entries: [
            PinFnEntry::Gpio,
            PinFnEntry::Null,
            PinFnEntry::Gpio,
            PinFnEntry::Null,
            PinFnEntry::Null,
            PinFnEntry::Null,
        ],
        reset: 0,
        flags: Some("RI"),
    },
    GpioPinConfig {
        simulation_clock: 37,
        fn_entries: [
            PinFnEntry::Gpio,
            PinFnEntry::Null,
            PinFnEntry::Gpio,
            PinFnEntry::Null,
            PinFnEntry::Null,
            PinFnEntry::Null,
        ],
        reset: 0,
        flags: Some("RI"),
    },
    GpioPinConfig {
        simulation_clock: 38,
        fn_entries: [
            PinFnEntry::Gpio,
            PinFnEntry::Null,
            PinFnEntry::Gpio,
            PinFnEntry::Null,
            PinFnEntry::Null,
            PinFnEntry::Null,
        ],
        reset: 0,
        flags: Some("RI"),
    },
    GpioPinConfig {
        simulation_clock: 39,
        fn_entries: [
            PinFnEntry::Gpio,
            PinFnEntry::Null,
            PinFnEntry::Gpio,
            PinFnEntry::Null,
            PinFnEntry::Null,
            PinFnEntry::Null,
        ],
        reset: 0,
        flags: Some("RI"),
    },
];

// ============================================================
// GpioPinDriveCapabilities
// ============================================================

pub static GPIO_PIN_DRIVE_CAPABILITIES: [u32; 64] = [
    2048, 2048, 2048, 2048, 2048, 2048, 2048, 2048, 2048, 2048, 2560, 2560,
    2688, 2560, 2816, 2688, 2816, 2688, 2560, 2560, 2816, 2816, 2816, 2816,
    2816, 2816, 2816, 2560, 2560, 2560, 2560, 2560, 2816, 2816, 2560, 2048,
    2048, 2048, 2048, 2048, 2048, 2048, 2048, 2048, 2048, 2048, 2048, 2048,
    2048, 2048, 2048, 2048, 2048, 2048, 2048, 2048, 2048, 2048, 2048, 2048,
    2048, 2048, 2048, 1023,
];

// ============================================================
// Base addresses
// ============================================================

pub const DPORT_BASE_ADDR: u32 = 0x3ff01000;
pub const SYSCON_BASE_ADDR: u32 = 0x3ff66000;
pub const RMT_ALT_BASE_ADDR: u32 = 0x3ff5d000;
pub const GPIO_BASE_ADDR: u32 = 0x3ff00000;
pub const EFUSE_BASE_ADDR: u32 = 0x3ff5a000;
pub const FE_BASE_ADDR: u32 = 0x3ff46000;
pub const FRC_TIMER_BASE_ADDR: u32 = 0x3ff47000;
pub const UART_BASE_ADDR: u32 = 0x3ff44000;
pub const UART1_MEM_BASE_ADDR: u32 = 0x3ff44f00;
pub const I2C_CONFIG_BASE_ADDR: u32 = 0x3ff4b000;
pub const I2C0_BASE_ADDR: u32 = 0x3ff53000;
pub const I2C1_BASE_ADDR: u32 = 0x3ff67000;
pub const I2S0_BASE_ADDR: u32 = 0x3ff4f000;
pub const I2S1_BASE_ADDR: u32 = 0x3ff6d000;
pub const IO_MUX_BASE_ADDR: u32 = 0x3ff49000;
pub const LEDC_BASE_ADDR: u32 = 0x3ff59000;
pub const PID_CONTROLLER_BASE_ADDR: u32 = 0x3ff5e000;
pub const PCNT_BASE_ADDR: u32 = 0x3ff6c000;
pub const PCNT_REGS_BASE_ADDR: u32 = 0x3ff5cc00;
pub const PCNT_ALT_BASE_ADDR: u32 = 0x3ff57000;
pub const RMT_BASE_ADDR: u32 = 0x3ff56000;
pub const RSA_BASE_ADDR: u32 = 0x3ff02000;
pub const RTC_CNTL_BASE_ADDR: u32 = 0x3ff48000;
pub const RTC_IO_BASE_ADDR: u32 = 0x3ff48400;
pub const RTC_I2C_BASE_ADDR: u32 = 0x3ff48c00;
pub const SDMMC_BASE_ADDR: u32 = 0x3ff68000;
pub const SENS_ADC_BASE_ADDR: u32 = 0x3ff48800;
pub const SHA_BASE_ADDR: u32 = 0x3ff03000;
pub const SDIO_SLAVE_BASE_ADDR: u32 = 0x3ff58000;
pub const UART3_BASE_ADDR: u32 = 0x3ff55000;
pub const SPI0_BASE_ADDR: u32 = 0x3ff43000;
pub const SPI1_BASE_ADDR: u32 = 0x3ff42000;
pub const SPI2_BASE_ADDR: u32 = 0x3ff64000;
pub const SPI3_BASE_ADDR: u32 = 0x3ff65000;
pub const TIMER_GROUP0_BASE_ADDR: u32 = 0x3ff5f000;
pub const TIMER_GROUP1_BASE_ADDR: u32 = 0x3ff60000;
pub const TWAI_BASE_ADDR: u32 = 0x3ff6b000;
pub const SDMMC_ALT_BASE_ADDR: u32 = 0x3ff40000;
pub const UHCI_BASE_ADDR: u32 = 0x3ff50000;
pub const UHCI_ALT_BASE_ADDR: u32 = 0x3ff6e000;
pub const UHCI_ALT_BASE_ADDR2: u32 = 0x3ff54000;
pub const UHCI_ALT_BASE_ADDR3: u32 = 0x3ff4c000;

// ============================================================
// UartRegisterMap (JS UartRegisterMap)
// ============================================================

pub const UART_REG_AUTOBAUD: i32 = 24;
pub const UART_REG_UPDATE: i32 = -1;
pub const UART_REG_ID: i32 = 124;
pub const UART_REG_AT_CMD_PRECNT: i32 = 72;
pub const UART_REG_AT_CMD_POSTCNT: i32 = 76;
pub const UART_REG_AT_CMD_GAPTOUT: i32 = 80;
pub const UART_REG_AT_CMD_CHAR: i32 = 84;
pub const UART_REG_MEM_RX_STATUS: i32 = 96;
pub const UART_REG_RXD_CNT: i32 = 48;
pub const UART_REG_LOWPULSE: i32 = 40;
pub const UART_REG_HIGHPULSE: i32 = 44;
pub const UART_REG_NEGPULSE: i32 = 108;
pub const UART_REG_POSPULSE: i32 = 104;
pub const UART_REG_CLK_CONF: i32 = -1;
pub const UART_REG_RX_FILT: i32 = -1;

// ============================================================
// UartFieldMap (JS UartFieldMap)
// ============================================================

pub const UART_FIELD_RXFIFO_RST: FieldDesc = FieldDesc::new(32, 17, 1);
pub const UART_FIELD_TXFIFO_RST: FieldDesc = FieldDesc::new(32, 18, 1);
pub const UART_FIELD_LOOPBACK: FieldDesc = FieldDesc::new(32, 14, 1);
pub const UART_FIELD_TX_FLOW_EN: FieldDesc = FieldDesc::new(32, 15, 1);
pub const UART_FIELD_IRDA_EN: FieldDesc = FieldDesc::new(32, 16, 1);
pub const UART_FIELD_IRDA_TX_EN: FieldDesc = FieldDesc::new(32, 10, 1);
pub const UART_FIELD_LOWPULSE_MIN_CNT: FieldDesc = FieldDesc::new(40, 0, 20);
pub const UART_FIELD_HIGHPULSE_MIN_CNT: FieldDesc = FieldDesc::new(44, 0, 20);
pub const UART_FIELD_NEGEDGE_MIN_CNT: FieldDesc = FieldDesc::new(108, 0, 20);
pub const UART_FIELD_POSEDGE_MIN_CNT: FieldDesc = FieldDesc::new(104, 0, 20);
pub const UART_FIELD_RX_TOUT_THRHD: FieldDesc = FieldDesc::new(36, 24, 7);
pub const UART_FIELD_GLITCH_FILT: FieldDesc = FieldDesc::new(24, 8, 8);
pub const UART_FIELD_RXFIFO_FULL_THRHD: FieldDesc = FieldDesc::new(36, 0, 7);
pub const UART_FIELD_TXFIFO_EMPTY_THRHD: FieldDesc = FieldDesc::new(36, 8, 7);

// ============================================================
// RmtRegisterMap (JS RmtRegisterMap)
// ============================================================

pub const RMT_REG_CH0CONF0: i32 = 32;
pub const RMT_REG_CH0CONF1: i32 = 36;
pub const RMT_REG_CH0_TX_CONF0: i32 = -1;
pub const RMT_REG_INT_RAW: i32 = 160;
pub const RMT_REG_INT_ST: i32 = 164;
pub const RMT_REG_INT_ENA: i32 = 168;
pub const RMT_REG_INT_CLR: i32 = 172;
pub const RMT_REG_CH0CARRIER_DUTY: i32 = 176;
pub const RMT_REG_CH0_TX_LIM: i32 = 208;
pub const RMT_REG_APB_CONF: i32 = 240;
pub const RMT_REG_SYS_CONF: i32 = -1;

// ============================================================
// LedcRegisterMap (JS LedcRegisterMap)
// ============================================================

pub const LEDC_REG_HSTIMER0_CONF: i32 = 320;
pub const LEDC_REG_HSTIMER1_CONF: i32 = 328;
pub const LEDC_REG_HSTIMER0_VALUE: i32 = 324;
pub const LEDC_REG_TIMER0_CONF: i32 = 352;
pub const LEDC_REG_TIMER1_CONF: i32 = 360;
pub const LEDC_REG_TIMER0_VALUE: i32 = 356;
pub const LEDC_REG_CH0_CONF0: i32 = 0;
pub const LEDC_REG_INT_RAW: i32 = 384;
pub const LEDC_REG_INT_ST: i32 = 388;
pub const LEDC_REG_INT_ENA: i32 = 392;
pub const LEDC_REG_INT_CLR: i32 = 396;
pub const LEDC_REG_CONF: i32 = 400;

// ============================================================
// LedcTimerFieldMap (JS LedcTimerFieldMap)
// ============================================================

pub const LEDC_FIELD_TIMERN_DUTY_RES: FieldDesc = FieldDesc::new(352, 0, 5);
pub const LEDC_FIELD_TIMERN_CLK_DIV: FieldDesc = FieldDesc::new(352, 5, 18);
pub const LEDC_FIELD_TIMERN_PAUSE: FieldDesc = FieldDesc::new(352, 23, 1);
pub const LEDC_FIELD_TIMERN_RST: FieldDesc = FieldDesc::new(352, 24, 1);
pub const LEDC_FIELD_TIMERN_TICK_SEL: FieldDesc = FieldDesc::new(352, 25, 1);
pub const LEDC_FIELD_TIMERN_PARA_UP: FieldDesc = FieldDesc::new(352, 26, 1);

// ============================================================
// TimerWdtRegisterMap (JS TimerWdtRegisterMap)
// ============================================================

pub const TIMER_WDT_REG_WDTCONFIG0: i32 = 72;
pub const TIMER_WDT_REG_WDTCONFIG1: i32 = 76;
pub const TIMER_WDT_REG_WDTCONFIG2: i32 = 80;
pub const TIMER_WDT_REG_WDTCONFIG3: i32 = 84;
pub const TIMER_WDT_REG_WDTCONFIG4: i32 = 88;
pub const TIMER_WDT_REG_WDTCONFIG5: i32 = 92;
pub const TIMER_WDT_REG_WDTFEED: i32 = 96;
pub const TIMER_WDT_REG_WDTWPROTECT: i32 = 100;
pub const TIMER_WDT_REG_RTCCALICFG: i32 = 104;
pub const TIMER_WDT_REG_RTCCALICFG1: i32 = 108;
pub const TIMER_WDT_REG_LACTCONFIG: i32 = 112;
pub const TIMER_WDT_REG_LACTLO: i32 = 120;
pub const TIMER_WDT_REG_LACTHI: i32 = 124;
pub const TIMER_WDT_REG_LACTUPDATE: i32 = 128;
pub const TIMER_WDT_REG_LACTALARMLO: i32 = 132;
pub const TIMER_WDT_REG_LACTALARMHI: i32 = 136;
pub const TIMER_WDT_REG_INT_ENA_TIMERS: i32 = 152;
pub const TIMER_WDT_REG_INT_RAW_TIMERS: i32 = 156;
pub const TIMER_WDT_REG_INT_ST_TIMERS: i32 = 160;
pub const TIMER_WDT_REG_INT_CLR_TIMERS: i32 = 164;

// ============================================================
// TimerWdtFieldMap (JS TimerWdtFieldMap)
// ============================================================

pub const TIMER_WDT_FIELD_RTC_CALI_START: FieldDesc = FieldDesc::new(104, 31, 1);
pub const TIMER_WDT_FIELD_RTC_CALI_START_CYCLING: FieldDesc = FieldDesc::new(104, 12, 1);
pub const TIMER_WDT_FIELD_RTC_CALI_CLK_SEL: FieldDesc = FieldDesc::new(104, 13, 2);
pub const TIMER_WDT_FIELD_RTC_CALI_RDY: FieldDesc = FieldDesc::new(104, 15, 1);
pub const TIMER_WDT_FIELD_RTC_CALI_MAX: FieldDesc = FieldDesc::new(104, 16, 15);
pub const TIMER_WDT_FIELD_RTC_CALI_VALUE: FieldDesc = FieldDesc::new(108, 7, 25);
pub const TIMER_WDT_FIELD_WDT_INT_RAW: FieldDesc = FieldDesc::new(156, 2, 1);

// ============================================================
// GpioRegisterMap (JS GpioRegisterMap)
// ============================================================

pub const GPIO_REG_STRAP: i32 = 56;
pub const GPIO_REG_OUT: i32 = 4;
pub const GPIO_REG_OUT_W1TS: i32 = 8;
pub const GPIO_REG_OUT_W1TC: i32 = 12;
pub const GPIO_REG_ENABLE: i32 = 32;
pub const GPIO_REG_ENABLE_W1TS: i32 = 36;
pub const GPIO_REG_ENABLE_W1TC: i32 = 40;
pub const GPIO_REG_SYSCON_TICK_COUNT_MASK: i32 = 60;
pub const GPIO_REG_STATUS: i32 = 68;
pub const GPIO_REG_STATUS_W1TS: i32 = 72;
pub const GPIO_REG_STATUS_W1TC: i32 = 76;
pub const GPIO_REG_OUT1: i32 = 16;
pub const GPIO_REG_OUT1_W1TS: i32 = 20;
pub const GPIO_REG_OUT1_W1TC: i32 = 24;
pub const GPIO_REG_ENABLE1: i32 = 44;
pub const GPIO_REG_ENABLE1_W1TS: i32 = 48;
pub const GPIO_REG_ENABLE1_W1TC: i32 = 52;
pub const GPIO_REG_IN1: i32 = 64;
pub const GPIO_REG_STATUS1: i32 = 80;
pub const GPIO_REG_STATUS1_W1TS: i32 = 84;
pub const GPIO_REG_STATUS1_W1TC: i32 = 88;
pub const GPIO_REG_ACPU_INT: i32 = 96;
pub const GPIO_REG_ACPU_NMI_INT: i32 = 100;
pub const GPIO_REG_PCPU_INT: i32 = 104;
pub const GPIO_REG_PCPU_NMI_INT: i32 = 108;
pub const GPIO_REG_ACPU_INT1: i32 = 116;
pub const GPIO_REG_ACPU_NMI_INT1: i32 = 120;
pub const GPIO_REG_PCPU_INT1: i32 = 124;
pub const GPIO_REG_PCPU_NMI_INT1: i32 = 128;
pub const GPIO_REG_STATUS_NEXT: i32 = -1;
pub const GPIO_REG_STATUS_NEXT1: i32 = -1;
pub const GPIO_REG_INTR_0: i32 = -1;
pub const GPIO_REG_INTR1_0: i32 = -1;
pub const GPIO_REG_INTR_1: i32 = -1;
pub const GPIO_REG_INTR1_1: i32 = -1;
pub const GPIO_REG_PIN0: i32 = 136;
pub const GPIO_REG_FUNC0_OUT_SEL_CFG: i32 = 1328;

pub const GPIO_REG_FUNCN_IN_SEL_CFG_FIRST: i32 = 304;
pub const GPIO_REG_FUNCN_IN_SEL_CFG_STRIDE: i32 = 4;
pub const GPIO_REG_FUNCN_IN_SEL_CFG_COUNT: u32 = 256;
pub const GPIO_REG_FUNCN_IN_SEL_CFG_START_INDEX: u32 = 0;

// ============================================================
// GpioInputSelectFieldMap (JS GpioInputSelectFieldMap)
// ============================================================

pub const GPIO_FIELD_IN_SEL: FieldDesc = FieldDesc::new(304, 0, 6);
pub const GPIO_FIELD_IN_INV_SEL: FieldDesc = FieldDesc::new(304, 6, 1);
pub const GPIO_FIELD_SEL: FieldDesc = FieldDesc::new(304, 7, 1);

// ============================================================
// I2cRegisterMap (JS I2cRegisterMap)
// ============================================================

pub const I2C_REG_COMD8: i32 = 120;
pub const I2C_REG_CLK_CONF: i32 = -1;
pub const I2C_REG_FIFO_ST: i32 = 20;
pub const I2C_REG_SLAVE_ADDR: i32 = 16;
pub const I2C_REG_SCL_SP_CONF: i32 = -1;

// ============================================================
// I2cFieldMap (JS I2cFieldMap)
// ============================================================

pub const I2C_FIELD_SCL_LOW_PERIOD: FieldDesc = FieldDesc::new(0, 0, 14);
pub const I2C_FIELD_SCL_HIGH_PERIOD: FieldDesc = FieldDesc::new(56, 0, 14);
pub const I2C_FIELD_SCL_FILTER_EN: FieldDesc = FieldDesc::new(80, 3, 1);
pub const I2C_FIELD_SCL_FILTER_THRES: FieldDesc = FieldDesc::new(80, 0, 3);

// ============================================================
// RtcCntlRegisterMap (JS RtcCntlRegisterMap)
// ============================================================

pub const RTC_CNTL_REG_CRC_CTRL: i32 = -1;
pub const RTC_CNTL_REG_CRC_VALUE: i32 = -1;
pub const RTC_CNTL_REG_RESET_STATE: i32 = 52;
pub const RTC_CNTL_REG_CLK_CONF: i32 = 112;
pub const RTC_CNTL_REG_SW_CPU_STALL: i32 = 172;
pub const RTC_CNTL_REG_INT_RAW_RTC: i32 = 64;
pub const RTC_CNTL_REG_INT_ST_RTC: i32 = 68;
pub const RTC_CNTL_REG_INT_ENA_RTC: i32 = 60;
pub const RTC_CNTL_REG_INT_CLR_RTC: i32 = 72;
pub const RTC_CNTL_REG_DIG_PWC: i32 = 132;
pub const RTC_CNTL_REG_STORE0: i32 = 76;
pub const RTC_CNTL_REG_STORE1: i32 = 80;
pub const RTC_CNTL_REG_STORE2: i32 = 84;
pub const RTC_CNTL_REG_STORE3: i32 = 88;
pub const RTC_CNTL_REG_STORE4: i32 = 176;
pub const RTC_CNTL_REG_STORE5: i32 = 180;
pub const RTC_CNTL_REG_STORE6: i32 = 184;
pub const RTC_CNTL_REG_STORE7: i32 = 188;
pub const RTC_CNTL_REG_ANA_CONF: i32 = 48;
pub const RTC_CNTL_REG_SLP_WAKEUP_CAUSE: i32 = 56;

// ============================================================
// RtcIoRegisterMap (JS RtcIoRegisterMap)
// ============================================================

pub const RTC_IO_REG_PAD_DAC1: i32 = 132;
pub const RTC_IO_REG_PAD_DAC2: i32 = 136;
pub const RTC_IO_REG_XTAL_32K_PAD: i32 = 140;
pub const RTC_IO_REG_TOUCH_PAD0: i32 = 148;
pub const RTC_IO_REG_TOUCH_PAD1: i32 = 152;
pub const RTC_IO_REG_TOUCH_PAD2: i32 = 156;
pub const RTC_IO_REG_TOUCH_PAD3: i32 = 160;
pub const RTC_IO_REG_TOUCH_PAD4: i32 = 164;
pub const RTC_IO_REG_TOUCH_PAD5: i32 = 168;
pub const RTC_IO_REG_TOUCH_PAD6: i32 = 172;
pub const RTC_IO_REG_TOUCH_PAD7: i32 = 176;

// ============================================================
// PcntRegisterMap (JS PcntRegisterMap)
// ============================================================

pub const PCNT_REG_UN_CONF0_FIRST: i32 = 0;
pub const PCNT_REG_UN_CONF0_STRIDE: i32 = 12;
pub const PCNT_REG_UN_CONF0_COUNT: u32 = 8;
pub const PCNT_REG_UN_CONF0_START_INDEX: u32 = 0;
pub const PCNT_REG_UN_CONF1_FIRST: i32 = 4;
pub const PCNT_REG_UN_CONF1_STRIDE: i32 = 12;
pub const PCNT_REG_UN_CONF1_COUNT: u32 = 8;
pub const PCNT_REG_UN_CONF1_START_INDEX: u32 = 0;
pub const PCNT_REG_UN_CONF2_FIRST: i32 = 8;
pub const PCNT_REG_UN_CONF2_STRIDE: i32 = 12;
pub const PCNT_REG_UN_CONF2_COUNT: u32 = 8;
pub const PCNT_REG_UN_CONF2_START_INDEX: u32 = 0;
pub const PCNT_REG_UN_CNT_FIRST: i32 = 96;
pub const PCNT_REG_UN_CNT_STRIDE: i32 = 4;
pub const PCNT_REG_UN_CNT_COUNT: u32 = 8;
pub const PCNT_REG_UN_CNT_START_INDEX: u32 = 0;
pub const PCNT_REG_UN_STATUS_FIRST: i32 = 144;
pub const PCNT_REG_UN_STATUS_STRIDE: i32 = 4;
pub const PCNT_REG_UN_STATUS_COUNT: u32 = 8;
pub const PCNT_REG_UN_STATUS_START_INDEX: u32 = 0;
pub const PCNT_REG_CTRL: i32 = 176;
pub const PCNT_REG_INT_RAW: i32 = 128;
pub const PCNT_REG_INT_ST: i32 = 132;
pub const PCNT_REG_INT_ENA: i32 = 136;
pub const PCNT_REG_INT_CLR: i32 = 140;

// ============================================================
// SpiRegisterMap (JS SpiRegisterMap)
// ============================================================

pub const SPI_REG_SLV_WR_STATUS: i32 = 48;
pub const SPI_REG_CTRL1: i32 = 12;
pub const SPI_REG_CTRL2: i32 = 20;
pub const SPI_REG_CLOCK: i32 = 24;
pub const SPI_REG_CLK_GATE: i32 = -1;
pub const SPI_REG_USER: i32 = 28;
pub const SPI_REG_USER1: i32 = 32;
pub const SPI_REG_USER2: i32 = 36;
pub const SPI_REG_MOSI_DLEN: i32 = 40;
pub const SPI_REG_MISO_DLEN: i32 = 44;
pub const SPI_REG_MS_DLEN: i32 = -1;
pub const SPI_REG_RD_STATUS: i32 = 16;
pub const SPI_REG_DIN_MODE: i32 = -1;
pub const SPI_REG_DIN_NUM: i32 = -1;
pub const SPI_REG_DOUT_MODE: i32 = -1;
pub const SPI_REG_PIN: i32 = 52;
pub const SPI_REG_MISC: i32 = -1;
pub const SPI_REG_SLAVE: i32 = 56;
pub const SPI_REG_SLV_RDBUF_DLEN: i32 = 76;
pub const SPI_REG_W0: i32 = 128;
pub const SPI_REG_DMA_CONF: i32 = 256;
pub const SPI_REG_DMA_IN_LINK: i32 = 264;
pub const SPI_REG_DMA_OUT_LINK: i32 = 260;
pub const SPI_REG_DMA_INT_ENA: i32 = 272;
pub const SPI_REG_DMA_INT_CLR: i32 = 284;
pub const SPI_REG_DMA_INT_RAW: i32 = 276;
pub const SPI_REG_DMA_INT_ST: i32 = 280;

// ============================================================
// SpiFieldMap (JS SpiFieldMap)
// ============================================================

pub const SPI_FIELD_USR: FieldDesc = FieldDesc::new(0, 18, 1);
pub const SPI_FIELD_WR_BIT_ORDER: FieldDesc = FieldDesc::new(8, 26, 1);
pub const SPI_FIELD_MODE: FieldDesc = FieldDesc::new(56, 29, 1);
pub const SPI_FIELD_DOUTDIN: FieldDesc = FieldDesc::new(28, 0, 1);
pub const SPI_FIELD_USR_DUMMY_CYCLELEN: FieldDesc = FieldDesc::new(32, 0, 8);
pub const SPI_FIELD_USR_ADDR_BITLEN: FieldDesc = FieldDesc::new(32, 26, 6);
pub const SPI_FIELD_USR_COMMAND_VALUE: FieldDesc = FieldDesc::new(36, 0, 16);
pub const SPI_FIELD_USR_COMMAND_BITLEN: FieldDesc = FieldDesc::new(36, 28, 4);
pub const SPI_FIELD_SLV_DATA_BITLEN: FieldDesc = FieldDesc::new(100, 0, 24);
pub const SPI_FIELD_CLKCNT_N: FieldDesc = FieldDesc::new(24, 12, 6);
pub const SPI_FIELD_CLKDIV_PRE: FieldDesc = FieldDesc::new(24, 18, 13);

// ============================================================
// I2sRegisterMap (JS I2sRegisterMap)
// ============================================================

pub const I2S_REG_CONF: i32 = 8;
pub const I2S_REG_CONF1: i32 = 160;
pub const I2S_REG_CONF2: i32 = 168;
pub const I2S_REG_INT_RAW: i32 = 12;
pub const I2S_REG_INT_ST: i32 = 16;
pub const I2S_REG_INT_ENA: i32 = 20;
pub const I2S_REG_INT_CLR: i32 = 24;
pub const I2S_REG_TIMING: i32 = 28;
pub const I2S_REG_FIFO_CONF: i32 = 32;
pub const I2S_REG_RXEOF_NUM: i32 = 36;
pub const I2S_REG_CONF_SIGLE_DATA: i32 = 40;
pub const I2S_REG_CONF_CHAN: i32 = 44;
pub const I2S_REG_OUT_LINK: i32 = 48;
pub const I2S_REG_IN_LINK: i32 = 52;
pub const I2S_REG_OUT_EOF_DES_ADDR: i32 = 56;
pub const I2S_REG_IN_EOF_DES_ADDR: i32 = 60;
pub const I2S_REG_OUT_EOF_BFR_DES_ADDR: i32 = 64;
pub const I2S_REG_INLINK_DSCR: i32 = 72;
pub const I2S_REG_INLINK_DSCR_BF0: i32 = 76;
pub const I2S_REG_INLINK_DSCR_BF1: i32 = 80;
pub const I2S_REG_OUTLINK_DSCR: i32 = 84;
pub const I2S_REG_OUTLINK_DSCR_BF0: i32 = 88;
pub const I2S_REG_OUTLINK_DSCR_BF1: i32 = 92;
pub const I2S_REG_LC_CONF: i32 = 96;
pub const I2S_REG_OUTFIFO_PUSH: i32 = 100;
pub const I2S_REG_INFIFO_POP: i32 = 104;
pub const I2S_REG_LC_STATE0: i32 = 108;
pub const I2S_REG_LC_STATE1: i32 = 112;
pub const I2S_REG_LC_HUNG_CONF: i32 = 116;
pub const I2S_REG_CLKM_CONF: i32 = 172;
pub const I2S_REG_SAMPLE_RATE_CONF: i32 = 176;
pub const I2S_REG_PD_CONF: i32 = 164;
pub const I2S_REG_STATE: i32 = 188;
pub const I2S_REG_DATE: i32 = 252;
pub const I2S_REG_TX_CONF: i32 = -1;
pub const I2S_REG_TX_CONF1: i32 = -1;
pub const I2S_REG_RX_CONF: i32 = -1;
pub const I2S_REG_RX_CONF1: i32 = -1;
pub const I2S_REG_TX_CLKM_CONF: i32 = -1;
pub const I2S_REG_TX_CLKM_DIV_CONF: i32 = -1;
pub const I2S_REG_RX_CLKM_CONF: i32 = -1;
pub const I2S_REG_RX_CLKM_DIV_CONF: i32 = -1;

// ============================================================
// I2sFieldMap (JS I2sFieldMap)
// ============================================================

pub const I2S_FIELD_TX_RESET: FieldDesc = FieldDesc::new(8, 0, 1);
pub const I2S_FIELD_RX_RESET: FieldDesc = FieldDesc::new(8, 1, 1);
pub const I2S_FIELD_TX_FIFO_RESET: FieldDesc = FieldDesc::new(8, 2, 1);
pub const I2S_FIELD_RX_FIFO_RESET: FieldDesc = FieldDesc::new(8, 3, 1);
pub const I2S_FIELD_TX_START: FieldDesc = FieldDesc::new(8, 4, 1);
pub const I2S_FIELD_RX_START: FieldDesc = FieldDesc::new(8, 5, 1);
pub const I2S_FIELD_TX_SLAVE_MOD: FieldDesc = FieldDesc::new(8, 6, 1);
pub const I2S_FIELD_RX_SLAVE_MOD: FieldDesc = FieldDesc::new(8, 7, 1);
pub const I2S_FIELD_SIG_LOOPBACK: FieldDesc = FieldDesc::new(8, 18, 1);
pub const I2S_FIELD_DSCR_EN: FieldDesc = FieldDesc::new(32, 12, 1);
pub const I2S_FIELD_TX_FIFO_MOD: FieldDesc = FieldDesc::new(32, 13, 3);
pub const I2S_FIELD_RX_FIFO_MOD: FieldDesc = FieldDesc::new(32, 16, 3);
pub const I2S_FIELD_TX_DATA_NUM: FieldDesc = FieldDesc::new(32, 6, 6);
pub const I2S_FIELD_RX_DATA_NUM: FieldDesc = FieldDesc::new(32, 0, 6);
pub const I2S_FIELD_TX_CHAN_MOD: FieldDesc = FieldDesc::new(44, 0, 3);
pub const I2S_FIELD_RX_CHAN_MOD: FieldDesc = FieldDesc::new(44, 3, 2);
pub const I2S_FIELD_OUT_RST: FieldDesc = FieldDesc::new(96, 1, 1);
pub const I2S_FIELD_IN_RST: FieldDesc = FieldDesc::new(96, 0, 1);
pub const I2S_FIELD_OUT_LOOP_TEST: FieldDesc = FieldDesc::new(96, 4, 1);
pub const I2S_FIELD_IN_LOOP_TEST: FieldDesc = FieldDesc::new(96, 5, 1);
pub const I2S_FIELD_OUT_AUTO_WRBACK: FieldDesc = FieldDesc::new(96, 6, 1);
pub const I2S_FIELD_OUT_EOF_MODE: FieldDesc = FieldDesc::new(96, 8, 1);
pub const I2S_FIELD_OUTDSCR_BURST_EN: FieldDesc = FieldDesc::new(96, 9, 1);
pub const I2S_FIELD_INDSCR_BURST_EN: FieldDesc = FieldDesc::new(96, 10, 1);
pub const I2S_FIELD_CHECK_OWNER: FieldDesc = FieldDesc::new(96, 12, 1);
pub const I2S_FIELD_CLKM_DIV_NUM: FieldDesc = FieldDesc::new(172, 0, 8);
pub const I2S_FIELD_CLKM_DIV_A: FieldDesc = FieldDesc::new(172, 14, 6);
pub const I2S_FIELD_CLKM_DIV_B: FieldDesc = FieldDesc::new(172, 8, 6);
pub const I2S_FIELD_CLK_EN: FieldDesc = FieldDesc::new(172, 20, 1);
pub const I2S_FIELD_TX_BCK_DIV_NUM: FieldDesc = FieldDesc::new(176, 0, 6);
pub const I2S_FIELD_RX_BCK_DIV_NUM: FieldDesc = FieldDesc::new(176, 6, 6);
pub const I2S_FIELD_TX_BITS_MOD: FieldDesc = FieldDesc::new(176, 12, 6);
pub const I2S_FIELD_RX_BITS_MOD: FieldDesc = FieldDesc::new(176, 18, 6);
pub const I2S_FIELD_IN_SUC_EOF_INT_RAW: FieldDesc = FieldDesc::new(12, 9, 1);
pub const I2S_FIELD_OUT_EOF_INT_RAW: FieldDesc = FieldDesc::new(12, 12, 1);
pub const I2S_FIELD_OUT_DONE_INT_RAW: FieldDesc = FieldDesc::new(12, 11, 1);
pub const I2S_FIELD_IN_DONE_INT_RAW: FieldDesc = FieldDesc::new(12, 8, 1);
pub const I2S_FIELD_TX_HUNG_INT_RAW: FieldDesc = FieldDesc::new(12, 7, 1);
pub const I2S_FIELD_RX_HUNG_INT_RAW: FieldDesc = FieldDesc::new(12, 6, 1);
pub const I2S_FIELD_OUTLINK_ADDR: FieldDesc = FieldDesc::new(48, 0, 20);
pub const I2S_FIELD_OUTLINK_STOP: FieldDesc = FieldDesc::new(48, 28, 1);
pub const I2S_FIELD_OUTLINK_START: FieldDesc = FieldDesc::new(48, 29, 1);
pub const I2S_FIELD_OUTLINK_RESTART: FieldDesc = FieldDesc::new(48, 30, 1);
pub const I2S_FIELD_INLINK_ADDR: FieldDesc = FieldDesc::new(52, 0, 20);
pub const I2S_FIELD_INLINK_STOP: FieldDesc = FieldDesc::new(52, 28, 1);
pub const I2S_FIELD_INLINK_START: FieldDesc = FieldDesc::new(52, 29, 1);
pub const I2S_FIELD_INLINK_RESTART: FieldDesc = FieldDesc::new(52, 30, 1);
pub const I2S_FIELD_CAMERA_EN: FieldDesc = FieldDesc::new(168, 0, 1);
pub const I2S_FIELD_LCD_TX_WRX2_EN: FieldDesc = FieldDesc::new(168, 1, 1);
pub const I2S_FIELD_LCD_TX_SDX2_EN: FieldDesc = FieldDesc::new(168, 2, 1);
pub const I2S_FIELD_DATA_ENABLE_TEST_EN: FieldDesc = FieldDesc::new(168, 3, 1);
pub const I2S_FIELD_DATA_ENABLE: FieldDesc = FieldDesc::new(168, 4, 1);
pub const I2S_FIELD_LCD_EN: FieldDesc = FieldDesc::new(168, 5, 1);
pub const I2S_FIELD_EXT_ADC_START_EN: FieldDesc = FieldDesc::new(168, 6, 1);
pub const I2S_FIELD_INTER_VALID_EN: FieldDesc = FieldDesc::new(168, 7, 1);

// ============================================================
// TwaiRegisterMap (JS TwaiRegisterMap)
// ============================================================

pub const TWAI_REG_MODE: i32 = 0;
pub const TWAI_REG_CMD: i32 = 4;
pub const TWAI_REG_STATUS: i32 = 8;
pub const TWAI_REG_DATA_0: i32 = 64;
pub const TWAI_REG_DATA_12: i32 = 112;
pub const TWAI_REG_INT_RAW: i32 = 12;
pub const TWAI_REG_INT_ENA: i32 = 16;

// ============================================================
// TwaiFieldMap (JS TwaiFieldMap)
// ============================================================

pub const TWAI_FIELD_RX_FILTER_MODE: FieldDesc = FieldDesc::new(0, 3, 1);
pub const TWAI_FIELD_RX_MESSAGE_COUNTER: FieldDesc = FieldDesc::new(116, 0, 7);

// ============================================================
// EfuseRegisterMap (JS EfuseRegisterMap)
// ============================================================

pub const EFUSE_REG_PGM_DATA6: i32 = 24;
pub const EFUSE_REG_RD_WR_DIS: i32 = -1;
pub const EFUSE_REG_RD_KEY0_DATA0: i32 = -1;
pub const EFUSE_REG_RD_SYS_PART1_DATA4: i32 = -1;
pub const EFUSE_REG_RD_SYS_PART2_DATA7: i32 = -1;
pub const EFUSE_REG_CONF: i32 = 252;
pub const EFUSE_REG_STATUS: i32 = 256;
pub const EFUSE_REG_CMD: i32 = 260;

// ============================================================
// EfuseFieldMap (JS EfuseFieldMap)
// ============================================================

pub static EFUSE_FIELD_MAP: &[FieldDesc] = &[];

pub static EFUSE_FIELD_MAP_PLACEHOLDER: &[FieldDesc] = &[];

// ============================================================
// Reset Values
// ============================================================

pub static EFUSE_RESET_VALUES: &[ResetEntry] = &[
    [4, 3, 1, 0],
    [24, 0x155408b, 1, 0],
    [64, 8, 1, 0],
    [68, 8, 1, 0],
    [80, 8, 1, 0],
    [84, 8, 1, 0],
    [248, 0x16042000, 1, 0],
];

pub static SPI_REG1: &[ResetEntry] = EFUSE_RESET_VALUES;

pub static SPI_DMA_RESET_VALUES: &[ResetEntry] = &[
    [8, 197376, 1, 0],
    [32, 6176, 1, 0],
    [36, 64, 1, 0],
    [96, 256, 1, 0],
    [116, 2064, 1, 0],
    [128, 0x80007fff, 1, 0],
    [132, 656640, 1, 0],
    [136, 328356, 1, 0],
    [140, 0x8a80339, 1, 0],
    [144, 0xa0178a05, 1, 0],
    [148, 40, 1, 0],
    [160, 137, 1, 0],
    [164, 10, 1, 0],
    [172, 4, 1, 0],
    [176, 4260230, 1, 0],
    [180, 0x1550020, 1, 0],
    [184, 983520, 1, 0],
    [188, 7, 1, 0],
    [252, 0x1604201, 1, 0],
];

pub static SPI_REG2: &[ResetEntry] = SPI_DMA_RESET_VALUES;

pub static LEDC_RESET_VALUES: &[ResetEntry] = &[
    [4, 65280, 1, 0],
    [20, 65280, 1, 0],
    [36, 65280, 1, 0],
    [76, 32, 1, 0],
    [88, 98304, 1, 0],
    [132, 32, 1, 0],
    [144, 98304, 1, 0],
    [188, 32, 1, 0],
    [200, 98304, 1, 0],
    [268, 85, 1, 0],
    [292, 0x2107230, 1, 0],
];

pub static PCNT_RESET_VALUES: &[ResetEntry] = LEDC_RESET_VALUES;

pub static UART_RESET_VALUES: &[ResetEntry] = &[
    [8, 2139136, 1, 0],
    [12, 0x5fff0000, 1, 0],
    [20, 17, 1, 0],
    [24, 0x80003043, 1, 0],
    [28, 0x80000040, 1, 0],
    [32, 0x5c000007, 1, 0],
    [36, 0x70000000, 1, 0],
    [52, 6, 1, 0],
    [56, 32, 1, 0],
    [60, 0x2000000, 1, 0],
    [84, 0x15c04830, 1, 0],
    [240, 0x800a0050, 1, 0],
    [244, 0x800f0000, 1, 0],
    [256, 512, 1, 0],
    [1020, 0x1604270, 1, 0],
];

pub static SPI_REG3: &[ResetEntry] = UART_RESET_VALUES;
pub static SPI2_RESET_VALUES: &[ResetEntry] = UART_RESET_VALUES;
pub static SPI3_RESET_VALUES: &[ResetEntry] = UART_RESET_VALUES;

pub static TIMER_GROUP0_RESET_VALUES: &[ResetEntry] = &[
    [0, 0x60002000, 1, 0],
    [36, 0x60002000, 1, 0],
    [72, 311296, 1, 0],
    [76, 65536, 1, 0],
    [80, 26e6 as u32, 1, 0],
    [84, 0x7ffffff, 1, 0],
    [88, 1048575, 1, 0],
    [92, 1048575, 1, 0],
    [100, 0x50d83aa1, 1, 0],
    [104, 77824, 1, 0],
    [112, 0x60002300, 1, 0],
    [248, 0x1604290, 1, 0],
];

pub static TIMER_GROUP1_RESET_VALUES: &[ResetEntry] = TIMER_GROUP0_RESET_VALUES;

pub static SDMMC_RESET_VALUES: &[ResetEntry] = &[
    [20, 694, 1, 0],
    [24, 4096, 1, 0],
    [32, 0x800001c, 1, 0],
    [36, 24672, 1, 0],
    [40, 1048575, 1, 0],
    [44, 1048575, 1, 0],
    [56, 240, 1, 0],
    [60, 0x1311e000, 1, 0],
    [64, 0xa40100, 1, 0],
    [72, 16e5 as u32, 1, 0],
    [76, 16e5 as u32, 1, 0],
    [80, 7680, 1, 0],
    [84, 811, 1, 0],
    [88, 136, 1, 0],
    [104, 1048575, 1, 0],
    [108, 1048575, 1, 0],
    [120, 0x15122500, 1, 0],
    [124, 1280, 1, 0],
];

pub static UHCI0_RESET_VALUES: &[ResetEntry] = SDMMC_RESET_VALUES;
pub static SPI_REG4: &[ResetEntry] = SDMMC_RESET_VALUES;

pub static UHCI_RESET_VALUES: &[ResetEntry] = &[
    [0, 3604736, 1, 0],
    [20, 2, 1, 0],
    [28, 2, 1, 0],
    [40, 1048576, 1, 0],
    [44, 51, 1, 0],
    [100, 51, 1, 0],
    [104, 8456208, 1, 0],
    [176, 0xdcdbc0, 1, 0],
    [180, 0xdddbdb, 1, 0],
    [184, 0xdedb11, 1, 0],
    [188, 0xdfdb13, 1, 0],
    [192, 128, 1, 0],
    [252, 0x16041001, 1, 0],
];

pub static UHCI3_RESET_VALUES: &[ResetEntry] = UHCI_RESET_VALUES;

// ============================================================
// Esp32FullResetValues — alternating (base_addr, [entries])
// ============================================================

pub static ESP32_FULL_RESET_VALUES: &[FullResetEntry] = &[
    FullResetEntry {
        base_addr: 0x3ff66000,
        entries: &[
            [0, 8192, 1, 0],
            [4, 39, 1, 0],
            [8, 79, 1, 0],
            [12, 11, 1, 0],
            [16, 8356416, 1, 0],
            [20, 510, 1, 0],
            [24, 0x208ff08, 1, 0],
            [28, 0xf0f0f0f, 1, 0],
            [32, 0xf0f0f0f, 1, 0],
            [36, 0xf0f0f0f, 1, 0],
            [40, 0xf0f0f0f, 1, 0],
            [44, 0xf0f0f0f, 1, 0],
            [48, 0xf0f0f0f, 1, 0],
            [52, 0xf0f0f0f, 1, 0],
            [56, 0xf0f0f0f, 1, 0],
            [60, 99, 1, 0],
            [124, 0x16042000, 1, 0],
        ],
    },
    FullResetEntry {
        base_addr: 0x3ff00000,
        entries: &[
            [44, 1, 1, 0],
            [64, 16, 1, 0],
            [68, 2303, 1, 0],
            [88, 16, 1, 0],
            [92, 2303, 1, 0],
            [140, 1, 1, 0],
            [148, 3, 1, 0],
            [160, 0xffffffff, 1, 0],
            [164, 1, 1, 0],
            [172, 257, 1, 0],
            [180, 0xffffffff, 1, 0],
            [184, 511, 1, 0],
            [192, 0xf9c1e06f, 1, 0],
            [204, 0xfffce030, 1, 0],
            [212, 255, 1, 0],
            [216, 0x2001001, 1, 0],
            [260, 16, 1, 0],
            [264, 16, 1, 0],
            [268, 16, 1, 0],
            [272, 16, 1, 0],
            [276, 16, 1, 0],
            [280, 16, 1, 0],
            [284, 16, 1, 0],
            [288, 16, 1, 0],
            [292, 16, 1, 0],
            [296, 16, 1, 0],
            [300, 16, 1, 0],
            [304, 16, 1, 0],
            [308, 16, 1, 0],
            [312, 16, 1, 0],
            [316, 16, 1, 0],
            [320, 16, 1, 0],
            [324, 16, 1, 0],
            [328, 16, 1, 0],
            [332, 16, 1, 0],
            [336, 16, 1, 0],
            [340, 16, 1, 0],
            [344, 16, 1, 0],
            [348, 16, 1, 0],
            [352, 16, 1, 0],
            [356, 16, 1, 0],
            [360, 16, 1, 0],
            [364, 16, 1, 0],
            [368, 16, 1, 0],
            [372, 16, 1, 0],
            [376, 16, 1, 0],
            [380, 16, 1, 0],
            [384, 16, 1, 0],
            [388, 16, 1, 0],
            [392, 16, 1, 0],
            [396, 16, 1, 0],
            [400, 16, 1, 0],
            [404, 16, 1, 0],
            [408, 16, 1, 0],
            [412, 16, 1, 0],
            [416, 16, 1, 0],
            [420, 16, 1, 0],
            [424, 16, 1, 0],
            [428, 16, 1, 0],
            [432, 16, 1, 0],
            [436, 16, 1, 0],
            [440, 16, 1, 0],
            [444, 16, 1, 0],
            [448, 16, 1, 0],
            [452, 16, 1, 0],
            [456, 16, 1, 0],
            [460, 16, 1, 0],
            [464, 16, 1, 0],
            [468, 16, 1, 0],
            [472, 16, 1, 0],
            [476, 16, 1, 0],
            [480, 16, 1, 0],
            [484, 16, 1, 0],
            [488, 16, 1, 0],
            [492, 16, 1, 0],
            [496, 16, 1, 0],
            [500, 16, 1, 0],
            [504, 16, 1, 0],
            [508, 16, 1, 0],
            [512, 16, 1, 0],
            [516, 16, 1, 0],
            [520, 16, 1, 0],
            [524, 16, 1, 0],
            [528, 16, 1, 0],
            [532, 16, 1, 0],
            [536, 16, 1, 0],
            [540, 16, 1, 0],
            [544, 16, 1, 0],
            [548, 16, 1, 0],
            [552, 16, 1, 0],
            [556, 16, 1, 0],
            [560, 16, 1, 0],
            [564, 16, 1, 0],
            [568, 16, 1, 0],
            [572, 16, 1, 0],
            [576, 16, 1, 0],
            [580, 16, 1, 0],
            [584, 16, 1, 0],
            [588, 16, 1, 0],
            [592, 16, 1, 0],
            [596, 16, 1, 0],
            [600, 16, 1, 0],
            [604, 16, 1, 0],
            [608, 16, 1, 0],
            [612, 16, 1, 0],
            [616, 16, 1, 0],
            [620, 16, 1, 0],
            [624, 16, 1, 0],
            [628, 16, 1, 0],
            [632, 16, 1, 0],
            [636, 16, 1, 0],
            [640, 16, 1, 0],
            [644, 16, 1, 0],
            [648, 16, 1, 0],
            [652, 16, 1, 0],
            [656, 16, 1, 0],
            [660, 16, 1, 0],
            [664, 16, 1, 0],
            [668, 16, 1, 0],
            [672, 16, 1, 0],
            [676, 16, 1, 0],
            [680, 16, 1, 0],
            [684, 16, 1, 0],
            [688, 16, 1, 0],
            [692, 16, 1, 0],
            [696, 16, 1, 0],
            [700, 16, 1, 0],
            [704, 16, 1, 0],
            [708, 16, 1, 0],
            [712, 16, 1, 0],
            [716, 16, 1, 0],
            [720, 16, 1, 0],
            [724, 16, 1, 0],
            [728, 16, 1, 0],
            [732, 16, 1, 0],
            [736, 16, 1, 0],
            [740, 16, 1, 0],
            [744, 16, 1, 0],
            [748, 16, 1, 0],
            [752, 16, 1, 0],
            [756, 16, 1, 0],
            [760, 16, 1, 0],
            [764, 16, 1, 0],
            [768, 16, 1, 0],
            [772, 16, 1, 0],
            [776, 16, 1, 0],
            [780, 16, 1, 0],
            [784, 16, 1, 0],
            [788, 16, 1, 0],
            [792, 16, 1, 0],
            [796, 16, 1, 0],
            [800, 16, 1, 0],
            [804, 16, 1, 0],
            [808, 16, 1, 0],
            [1088, 256, 1, 0],
            [1128, 256, 1, 0],
            [1172, 1, 1, 0],
            [1176, 1, 1, 0],
            [1180, 1, 1, 0],
            [1184, 1, 1, 0],
            [1188, 1, 1, 0],
            [1192, 1, 1, 0],
            [1196, 1, 1, 0],
            [1200, 1, 1, 0],
            [1204, 1, 1, 0],
            [1208, 1, 1, 0],
            [1212, 1, 1, 0],
            [1216, 1, 1, 0],
            [1220, 1, 1, 0],
            [1224, 1, 1, 0],
            [1228, 1, 1, 0],
            [1232, 1, 1, 0],
            [1236, 1, 1, 0],
            [1240, 1, 1, 0],
            [1244, 1, 1, 0],
            [1248, 1, 1, 0],
            [1252, 1, 1, 0],
            [1256, 1, 1, 0],
            [1260, 1, 1, 0],
            [1264, 1, 1, 0],
            [1268, 1, 1, 0],
            [1272, 1, 1, 0],
            [1276, 1, 1, 0],
            [1280, 1, 1, 0],
            [1288, 1, 1, 0],
            [1292, 2, 1, 0],
            [1296, 3, 1, 0],
            [1300, 4, 1, 0],
            [1304, 5, 1, 0],
            [1308, 6, 1, 0],
            [1312, 7, 1, 0],
            [1316, 8, 1, 0],
            [1320, 9, 1, 0],
            [1324, 10, 1, 0],
            [1328, 11, 1, 0],
            [1332, 12, 1, 0],
            [1336, 13, 1, 0],
            [1340, 14, 1, 0],
            [1344, 15, 1, 0],
            [1352, 1, 1, 0],
            [1356, 2, 1, 0],
            [1360, 3, 1, 0],
            [1364, 4, 1, 0],
            [1368, 5, 1, 0],
            [1372, 6, 1, 0],
            [1376, 7, 1, 0],
            [1380, 8, 1, 0],
            [1384, 9, 1, 0],
            [1388, 10, 1, 0],
            [1392, 11, 1, 0],
            [1396, 12, 1, 0],
            [1400, 13, 1, 0],
            [1404, 14, 1, 0],
            [1408, 15, 1, 0],
            [1412, 1, 1, 0],
            [1420, 1, 1, 0],
            [1428, 5, 1, 0],
            [4092, 0x1605190, 1, 0],
        ],
    },
    FullResetEntry {
        base_addr: 0x3ff5a000,
        entries: &[
            [248, 16466, 1, 0],
            [252, 65536, 1, 0],
            [280, 40, 1, 0],
            [508, 0x16042600, 1, 0],
        ],
    },
    FullResetEntry {
        base_addr: 0x3ff44f00,
        entries: &[
            [0, 65280, 1, 0],
            [4, 65280, 1, 0],
            [8, 65280, 1, 0],
            [12, 65280, 1, 0],
            [16, 65280, 1, 0],
            [20, 65280, 1, 0],
            [24, 65280, 1, 0],
            [28, 65280, 1, 0],
            [40, 0x1506190, 1, 0],
        ],
    },
    FullResetEntry {
        base_addr: 0x3ff4b000,
        entries: &[
            [0, 0x22226666, 1, 0],
            [4, 0x1110011, 1, 0],
            [28, 131072, 1, 0],
            [32, 0xffffffff, 1, 0],
            [36, 0xffffffff, 1, 0],
            [40, 0xffffffff, 1, 0],
            [44, 0xffffffff, 1, 0],
            [48, 0xffffffff, 1, 0],
            [52, 0xffffffff, 1, 0],
            [56, 0xffffffff, 1, 0],
            [60, 0xffffffff, 1, 0],
            [64, 0x33336666, 1, 0],
            [252, 0x15030200, 1, 0],
        ],
    },
    FullResetEntry {
        base_addr: 0x3ff53000,
        entries: EFUSE_RESET_VALUES,
    },
    FullResetEntry {
        base_addr: 0x3ff67000,
        entries: SPI_REG1,
    },
    FullResetEntry {
        base_addr: 0x3ff4f000,
        entries: SPI_DMA_RESET_VALUES,
    },
    FullResetEntry {
        base_addr: 0x3ff6d000,
        entries: SPI_REG2,
    },
    FullResetEntry {
        base_addr: 0x3ff59000,
        entries: &[
            [12, 0x40000000, 8, 20],
            [172, 0x40000000, 8, 20],
            [320, 0x1000000, 4, 8],
            [352, 0x1000000, 4, 8],
            [508, 0x16031700, 1, 0],
        ],
    },
    FullResetEntry {
        base_addr: 0x3ff5e000,
        entries: LEDC_RESET_VALUES,
    },
    FullResetEntry {
        base_addr: 0x3ff6c000,
        entries: PCNT_RESET_VALUES,
    },
    FullResetEntry {
        base_addr: 0x3ff57000,
        entries: &[
            [0, 15376, 8, 12],
            [176, 21845, 1, 0],
            [252, 0x14122600, 1, 0],
        ],
    },
    FullResetEntry {
        base_addr: 0x3ff56000,
        entries: &[
            [32, 0x31100002, 8, 8],
            [36, 3872, 8, 8],
            [176, 4194368, 1, 0],
            [180, 4194368, 1, 0],
            [184, 4194368, 1, 0],
            [188, 4194368, 1, 0],
            [192, 4194368, 1, 0],
            [196, 4194368, 1, 0],
            [200, 4194368, 1, 0],
            [204, 4194368, 1, 0],
            [208, 128, 8, 4],
            [252, 0x16022600, 1, 0],
        ],
    },
    FullResetEntry {
        base_addr: 0x3ff48000,
        entries: &[
            [0, 0x1c492000, 1, 0],
            [24, 3145728, 1, 0],
            [28, 0x28140403, 1, 0],
            [32, 0x1080000, 1, 0],
            [36, 0x14160a08, 1, 0],
            [40, 0x10200a08, 1, 0],
            [44, 0x12148001, 1, 0],
            [48, 8388608, 1, 0],
            [52, 12288, 1, 0],
            [56, 24576, 1, 0],
            [112, 8720, 1, 0],
            [116, 0x2a00000, 1, 0],
            [124, 0x29002400, 1, 0],
            [128, 76069, 1, 0],
            [132, 1398096, 1, 0],
            [136, 0xaaaa5000, 1, 0],
            [140, 19584, 1, 0],
            [144, 128e3 as u32, 1, 0],
            [148, 8e4 as u32, 1, 0],
            [152, 4095, 1, 0],
            [156, 4095, 1, 0],
            [164, 0x50d83aa1, 1, 0],
            [212, 0x13ff0000, 1, 0],
            [316, 0x1604280, 1, 0],
        ],
    },
    FullResetEntry {
        base_addr: 0x3ff48400,
        entries: &[
            [132, 0x80000000, 1, 0],
            [136, 0x80000000, 1, 0],
            [140, 0x84100010, 1, 0],
            [144, 0x66000000, 1, 0],
            [148, 0x52000000, 1, 0],
            [152, 0x4a000000, 1, 0],
            [156, 0x52000000, 1, 0],
            [160, 0x4a000000, 1, 0],
            [164, 0x52000000, 1, 0],
            [168, 0x52000000, 1, 0],
            [172, 0x4a000000, 1, 0],
            [176, 0x42000000, 1, 0],
            [180, 0x2000000, 1, 0],
            [184, 0x2000000, 1, 0],
            [200, 0x1603160, 1, 0],
        ],
    },
    FullResetEntry {
        base_addr: 0x3ff68000,
        entries: &[
            [20, 0xffffff40, 1, 0],
            [28, 512, 1, 0],
            [32, 512, 1, 0],
            [44, 0x20000000, 1, 0],
            [72, 1814, 1, 0],
            [108, 0x5432270a, 1, 0],
            [112, 0x3444cc3, 1, 0],
            [120, 1, 1, 0],
            [2048, 8520192, 1, 0],
        ],
    },
    FullResetEntry {
        base_addr: 0x3ff48800,
        entries: &[
            [0, 461058, 1, 0],
            [8, 655370, 1, 0],
            [12, 2097162, 1, 0],
            [16, 0x707338f, 1, 0],
            [24, 200, 1, 0],
            [28, 100, 1, 0],
            [32, 50, 1, 0],
            [36, 40, 1, 0],
            [40, 20, 1, 0],
            [44, 15, 1, 0],
            [48, 1049088, 1, 0],
            [52, 0xffffffff, 1, 0],
            [56, 0xffffffff, 1, 0],
            [76, 417794, 1, 0],
            [88, 0x2041000, 1, 0],
            [132, 4196352, 1, 0],
            [140, 0x3fffffff, 1, 0],
            [144, 461058, 1, 0],
            [156, 0x3000000, 1, 0],
            [160, 3, 1, 0],
            [252, 0x1605180, 1, 0],
        ],
    },
    FullResetEntry {
        base_addr: 0x3ff58000,
        entries: &[
            [0, 0xff3cff30, 1, 0],
            [36, 131074, 1, 0],
            [48, 131074, 1, 0],
            [68, 1048576, 1, 0],
            [96, 3145848, 1, 0],
            [116, 685856, 1, 0],
            [152, 0x101b101a, 1, 0],
            [216, 128, 1, 0],
            [276, 1289, 1, 0],
            [280, 1023, 1, 0],
            [312, 21504, 1, 0],
            [504, 0x16022500, 1, 0],
            [508, 256, 1, 0],
        ],
    },
    FullResetEntry {
        base_addr: 0x3ff55000,
        entries: &[
            [32, 1, 1, 0],
            [120, 192, 1, 0],
            [124, 511, 1, 0],
            [268, 245828, 1, 0],
            [272, 246240, 1, 0],
            [376, 0x16022500, 1, 0],
            [380, 1536, 1, 0],
        ],
    },
    FullResetEntry {
        base_addr: 0x3ff43000,
        entries: UART_RESET_VALUES,
    },
    FullResetEntry {
        base_addr: 0x3ff42000,
        entries: SPI_REG3,
    },
    FullResetEntry {
        base_addr: 0x3ff64000,
        entries: SPI2_RESET_VALUES,
    },
    FullResetEntry {
        base_addr: 0x3ff65000,
        entries: SPI3_RESET_VALUES,
    },
    FullResetEntry {
        base_addr: 0x3ff5f000,
        entries: TIMER_GROUP0_RESET_VALUES,
    },
    FullResetEntry {
        base_addr: 0x3ff60000,
        entries: TIMER_GROUP1_RESET_VALUES,
    },
    FullResetEntry {
        base_addr: 0x3ff6b000,
        entries: &[
            [0, 1, 1, 0],
            [52, 96, 1, 0],
        ],
    },
    FullResetEntry {
        base_addr: 0x3ff40000,
        entries: SDMMC_RESET_VALUES,
    },
    FullResetEntry {
        base_addr: 0x3ff50000,
        entries: UHCI0_RESET_VALUES,
    },
    FullResetEntry {
        base_addr: 0x3ff6e000,
        entries: SPI_REG4,
    },
    FullResetEntry {
        base_addr: 0x3ff54000,
        entries: UHCI_RESET_VALUES,
    },
    FullResetEntry {
        base_addr: 0x3ff4c000,
        entries: UHCI3_RESET_VALUES,
    },
];

// ============================================================
// MmuPageTableConfig (JS MmuPageTableConfig)
// ============================================================

#[derive(Clone, Copy)]
pub struct MmuPageConfig {
    pub start: u32,
    pub index: u32,
    pub pages: u32,
    pub sys: bool,
    pub user: bool,
}

pub static MMU_PAGE_TABLE_CONFIG: &[MmuPageConfig] = &[
    MmuPageConfig { start: 0x3f400000, index: 0, pages: 64, sys: true, user: false },
    MmuPageConfig { start: 0x400c0000, index: 76, pages: 52, sys: true, user: false },
    MmuPageConfig { start: 0x40400000, index: 128, pages: 64, sys: true, user: true },
    MmuPageConfig { start: 0x40800000, index: 192, pages: 64, sys: true, user: true },
];

// ============================================================
// Memory region constants
// ============================================================

pub const DROM0_SIZE: u32 = 1048576;
pub const REGION_DROM0_BASE: u32 = 0x3f800000;
pub const REGION_DROM1_BASE: u32 = 0x3fc00000;
pub const GPIO_BASE_ADDR_ALT: u32 = 0x3ff00000;
pub const REGION_DRAM1_BASE: u32 = 0x3ff90000;
pub const REGION_RTC_SLOW_BASE: u32 = 0x3ffae000;
pub const REGION_FLASH_CACHE_BASE: u32 = 0x40070000;
pub const REGION_IROM0_BASE: u32 = 0x400a0000;
pub const REGION_CODE_BASE: u32 = 0x40000000;
pub const REGION_IRAM0_BASE: u32 = 0x400c0000;
pub const REGION_IRAM1_BASE_ALT: u32 = 0x400c2000;
pub const IRAM0_SIZE: u32 = 458752;
pub const USB_OTG_BASE_ADDR: u32 = 0x50000000;
pub const RTC_SLOW_SIZE: u32 = 8192;
pub const REGION_PERI_BUS_BASE: u32 = 0x60000000;
pub const REGION_USB_BASE: u32 = 0x60040000;
pub const REGION_DROM0_MAP_BASE: u32 = 0x200c0000;
pub const IRAM1_SIZE: u32 = 204800;
pub const DROM0_CACHE_SIZE: u32 = 131072;
pub const REGION_IROM0_BASE_ALT: u32 = 0x400a0000;
pub const REGION_IRAM1_BASE: u32 = 0x400c0000;
pub const REGION_PERI1_BASE: u32 = 0x3ff10000;
pub const REGION_DROM_SIZE: u32 = 8192;
pub const REGION_CACHE_LINE_SIZE: u32 = 2048;
pub const REGION_PERI2_BASE: u32 = 0x3ff14000;
pub const REGION_CACHE_ALIGN_SIZE: u32 = 16;
