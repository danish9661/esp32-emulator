// Translated from: src/peripherals/common/gpio.js
// Do NOT modify the logic — match the JS line-for-line

use crate::peripherals::types::*;

use super::gpio_core::{GpioSignalInfo, PeripheralType};

// ── Constants ──
pub const TIM_REG19: u32 = 256;

// ── Signal ID constants (matching JS GpioSignalDefs) ──
pub const SIG_GPIO: u32 = 0xFFFF_FFFF;
pub const SIG_RSA_REG26: u32 = 0;
pub const SIG_RSA_REG57: u32 = 1;
pub const SIG_RTS: u32 = 2;
pub const SIG_CTS: u32 = 3;
pub const SIG_DSR: u32 = 4;
pub const SIG_DTR: u32 = 5;
pub const SIG_SCL: u32 = 6;
pub const SIG_SDA: u32 = 7;
pub const SIG_CS0: u32 = 8;
pub const SIG_CS1: u32 = 9;
pub const SIG_CS2: u32 = 10;
pub const SIG_CS3: u32 = 11;
pub const SIG_CS4: u32 = 12;
pub const SIG_CS5: u32 = 13;
pub const SIG_MOSI: u32 = 14;
pub const SIG_CLK: u32 = 15;
pub const SIG_MISO: u32 = 16;
pub const SIG_HD: u32 = 17;
pub const SIG_WP: u32 = 18;
pub const SIG_D4: u32 = 19;
pub const SIG_D5: u32 = 20;
pub const SIG_D6: u32 = 21;
pub const SIG_D7: u32 = 22;
pub const SIG_DQS: u32 = 23;
pub const SIG_SIG_CH0: u32 = 24;
pub const SIG_SIG_CH1: u32 = 25;
pub const SIG_CTRL_CH0: u32 = 26;
pub const SIG_CTRL_CH1: u32 = 27;
pub const SIG_RST_PAD: u32 = 28;
pub const SIG_BUS_OFF_ON: u32 = 29;
pub const SIG_CLKOUT: u32 = 30;
pub const SIG_STANDBY: u32 = 31;

// ── GpioSignalDefs entries ──
macro_rules! sig_def {
    ($periph:expr, $idx:expr, $sig:expr) => {
        GpioSignalInfo {
            name_tag: 1,
            name_id: 0,
            peripheral: $periph,
            index: $idx,
            signal_id: $sig,
        }
    };
}

pub const GPIO_SIGNAL_DEFS: &[GpioSignalInfo] = &[
    // UART 0
    sig_def!(PeripheralType::Uart, 0, SIG_RSA_REG26),
    sig_def!(PeripheralType::Uart, 0, SIG_RSA_REG57),
    sig_def!(PeripheralType::Uart, 0, SIG_RTS),
    sig_def!(PeripheralType::Uart, 0, SIG_CTS),
    sig_def!(PeripheralType::Uart, 0, SIG_DSR),
    sig_def!(PeripheralType::Uart, 0, SIG_DTR),
    // UART 1
    sig_def!(PeripheralType::Uart, 1, SIG_RSA_REG26),
    sig_def!(PeripheralType::Uart, 1, SIG_RSA_REG57),
    sig_def!(PeripheralType::Uart, 1, SIG_RTS),
    sig_def!(PeripheralType::Uart, 1, SIG_CTS),
    sig_def!(PeripheralType::Uart, 1, SIG_DSR),
    sig_def!(PeripheralType::Uart, 1, SIG_DTR),
    // UART 2
    sig_def!(PeripheralType::Uart, 2, SIG_RSA_REG26),
    sig_def!(PeripheralType::Uart, 2, SIG_RSA_REG57),
    sig_def!(PeripheralType::Uart, 2, SIG_RTS),
    sig_def!(PeripheralType::Uart, 2, SIG_CTS),
    sig_def!(PeripheralType::Uart, 2, SIG_DSR),
    sig_def!(PeripheralType::Uart, 2, SIG_DTR),
    // UART 3
    sig_def!(PeripheralType::Uart, 3, SIG_RSA_REG26),
    sig_def!(PeripheralType::Uart, 3, SIG_RSA_REG57),
    sig_def!(PeripheralType::Uart, 3, SIG_RTS),
    sig_def!(PeripheralType::Uart, 3, SIG_CTS),
    sig_def!(PeripheralType::Uart, 3, SIG_DSR),
    sig_def!(PeripheralType::Uart, 3, SIG_DTR),
    // UART 4
    sig_def!(PeripheralType::Uart, 4, SIG_RSA_REG26),
    sig_def!(PeripheralType::Uart, 4, SIG_RSA_REG57),
    sig_def!(PeripheralType::Uart, 4, SIG_RTS),
    sig_def!(PeripheralType::Uart, 4, SIG_CTS),
    sig_def!(PeripheralType::Uart, 4, SIG_DSR),
    sig_def!(PeripheralType::Uart, 4, SIG_DTR),
    // I2C 0
    sig_def!(PeripheralType::I2c, 0, SIG_SCL),
    sig_def!(PeripheralType::I2c, 0, SIG_SDA),
    // I2C 1
    sig_def!(PeripheralType::I2c, 1, SIG_SCL),
    sig_def!(PeripheralType::I2c, 1, SIG_SDA),
    // SPI 1
    sig_def!(PeripheralType::Spi, 1, SIG_CS0),
    sig_def!(PeripheralType::Spi, 1, SIG_CS1),
    sig_def!(PeripheralType::Spi, 1, SIG_CS2),
    sig_def!(PeripheralType::Spi, 1, SIG_MOSI),
    sig_def!(PeripheralType::Spi, 1, SIG_CLK),
    sig_def!(PeripheralType::Spi, 1, SIG_MISO),
    sig_def!(PeripheralType::Spi, 1, SIG_HD),
    sig_def!(PeripheralType::Spi, 1, SIG_WP),
    // SPI 2 (FSPI)
    sig_def!(PeripheralType::Spi, 2, SIG_CS0),
    sig_def!(PeripheralType::Spi, 2, SIG_CS1),
    sig_def!(PeripheralType::Spi, 2, SIG_CS2),
    sig_def!(PeripheralType::Spi, 2, SIG_CS3),
    sig_def!(PeripheralType::Spi, 2, SIG_CS4),
    sig_def!(PeripheralType::Spi, 2, SIG_CS5),
    sig_def!(PeripheralType::Spi, 2, SIG_MOSI),
    sig_def!(PeripheralType::Spi, 2, SIG_CLK),
    sig_def!(PeripheralType::Spi, 2, SIG_MISO),
    sig_def!(PeripheralType::Spi, 2, SIG_HD),
    sig_def!(PeripheralType::Spi, 2, SIG_WP),
    sig_def!(PeripheralType::Spi, 2, SIG_D4),
    sig_def!(PeripheralType::Spi, 2, SIG_D5),
    sig_def!(PeripheralType::Spi, 2, SIG_D6),
    sig_def!(PeripheralType::Spi, 2, SIG_D7),
    sig_def!(PeripheralType::Spi, 2, SIG_DQS),
    // SPI 3
    sig_def!(PeripheralType::Spi, 3, SIG_CS0),
    sig_def!(PeripheralType::Spi, 3, SIG_CS1),
    sig_def!(PeripheralType::Spi, 3, SIG_CS2),
    sig_def!(PeripheralType::Spi, 3, SIG_CS3),
    sig_def!(PeripheralType::Spi, 3, SIG_CS4),
    sig_def!(PeripheralType::Spi, 3, SIG_CS5),
    sig_def!(PeripheralType::Spi, 3, SIG_MOSI),
    sig_def!(PeripheralType::Spi, 3, SIG_CLK),
    sig_def!(PeripheralType::Spi, 3, SIG_MISO),
    sig_def!(PeripheralType::Spi, 3, SIG_HD),
    sig_def!(PeripheralType::Spi, 3, SIG_WP),
    // SPI 2 (HSPI — same periph/index as FSPI, different name)
    sig_def!(PeripheralType::Spi, 2, SIG_CS0),
    sig_def!(PeripheralType::Spi, 2, SIG_CS1),
    sig_def!(PeripheralType::Spi, 2, SIG_CS2),
    sig_def!(PeripheralType::Spi, 2, SIG_MOSI),
    sig_def!(PeripheralType::Spi, 2, SIG_CLK),
    sig_def!(PeripheralType::Spi, 2, SIG_MISO),
    sig_def!(PeripheralType::Spi, 2, SIG_HD),
    sig_def!(PeripheralType::Spi, 2, SIG_WP),
    // SPI 3 (VSPI — same periph/index as SPI3, different name)
    sig_def!(PeripheralType::Spi, 3, SIG_CS0),
    sig_def!(PeripheralType::Spi, 3, SIG_CS1),
    sig_def!(PeripheralType::Spi, 3, SIG_CS2),
    sig_def!(PeripheralType::Spi, 3, SIG_MOSI),
    sig_def!(PeripheralType::Spi, 3, SIG_CLK),
    sig_def!(PeripheralType::Spi, 3, SIG_MISO),
    sig_def!(PeripheralType::Spi, 3, SIG_HD),
    sig_def!(PeripheralType::Spi, 3, SIG_WP),
    // LP_UART (UART 2)
    sig_def!(PeripheralType::Uart, 2, SIG_RSA_REG57),
    sig_def!(PeripheralType::Uart, 2, SIG_RSA_REG26),
    sig_def!(PeripheralType::Uart, 2, SIG_RTS),
    sig_def!(PeripheralType::Uart, 2, SIG_CTS),
    sig_def!(PeripheralType::Uart, 2, SIG_DSR),
    sig_def!(PeripheralType::Uart, 2, SIG_DTR),
    // LP_I2C (I2C 1)
    sig_def!(PeripheralType::I2c, 1, SIG_SDA),
    sig_def!(PeripheralType::I2c, 1, SIG_SCL),
    // LP_SPI (SPI 3)
    sig_def!(PeripheralType::Spi, 3, SIG_CS0),
    sig_def!(PeripheralType::Spi, 3, SIG_MOSI),
    sig_def!(PeripheralType::Spi, 3, SIG_CLK),
    sig_def!(PeripheralType::Spi, 3, SIG_MISO),
    // PCNT unit 0
    sig_def!(PeripheralType::Pcnt, 0, SIG_SIG_CH0),
    sig_def!(PeripheralType::Pcnt, 0, SIG_SIG_CH1),
    sig_def!(PeripheralType::Pcnt, 0, SIG_CTRL_CH0),
    sig_def!(PeripheralType::Pcnt, 0, SIG_CTRL_CH1),
    // PCNT unit 1
    sig_def!(PeripheralType::Pcnt, 1, SIG_SIG_CH0),
    sig_def!(PeripheralType::Pcnt, 1, SIG_SIG_CH1),
    sig_def!(PeripheralType::Pcnt, 1, SIG_CTRL_CH0),
    sig_def!(PeripheralType::Pcnt, 1, SIG_CTRL_CH1),
    // PCNT unit 2
    sig_def!(PeripheralType::Pcnt, 2, SIG_SIG_CH0),
    sig_def!(PeripheralType::Pcnt, 2, SIG_SIG_CH1),
    sig_def!(PeripheralType::Pcnt, 2, SIG_CTRL_CH0),
    sig_def!(PeripheralType::Pcnt, 2, SIG_CTRL_CH1),
    // PCNT unit 3
    sig_def!(PeripheralType::Pcnt, 3, SIG_SIG_CH0),
    sig_def!(PeripheralType::Pcnt, 3, SIG_SIG_CH1),
    sig_def!(PeripheralType::Pcnt, 3, SIG_CTRL_CH0),
    sig_def!(PeripheralType::Pcnt, 3, SIG_CTRL_CH1),
    // PCNT unit 4
    sig_def!(PeripheralType::Pcnt, 4, SIG_SIG_CH0),
    sig_def!(PeripheralType::Pcnt, 4, SIG_SIG_CH1),
    sig_def!(PeripheralType::Pcnt, 4, SIG_CTRL_CH0),
    sig_def!(PeripheralType::Pcnt, 4, SIG_CTRL_CH1),
    // PCNT unit 5
    sig_def!(PeripheralType::Pcnt, 5, SIG_SIG_CH0),
    sig_def!(PeripheralType::Pcnt, 5, SIG_SIG_CH1),
    sig_def!(PeripheralType::Pcnt, 5, SIG_CTRL_CH0),
    sig_def!(PeripheralType::Pcnt, 5, SIG_CTRL_CH1),
    // PCNT unit 6
    sig_def!(PeripheralType::Pcnt, 6, SIG_SIG_CH0),
    sig_def!(PeripheralType::Pcnt, 6, SIG_SIG_CH1),
    sig_def!(PeripheralType::Pcnt, 6, SIG_CTRL_CH0),
    sig_def!(PeripheralType::Pcnt, 6, SIG_CTRL_CH1),
    // PCNT unit 7
    sig_def!(PeripheralType::Pcnt, 7, SIG_SIG_CH0),
    sig_def!(PeripheralType::Pcnt, 7, SIG_SIG_CH1),
    sig_def!(PeripheralType::Pcnt, 7, SIG_CTRL_CH0),
    sig_def!(PeripheralType::Pcnt, 7, SIG_CTRL_CH1),
    // PCNT reset pads
    sig_def!(PeripheralType::Pcnt, 0, SIG_RST_PAD),
    sig_def!(PeripheralType::Pcnt, 1, SIG_RST_PAD),
    sig_def!(PeripheralType::Pcnt, 2, SIG_RST_PAD),
    sig_def!(PeripheralType::Pcnt, 3, SIG_RST_PAD),
    // TWAI 0
    sig_def!(PeripheralType::Twai, 0, SIG_RSA_REG57),
    sig_def!(PeripheralType::Twai, 0, SIG_RSA_REG26),
    sig_def!(PeripheralType::Twai, 0, SIG_BUS_OFF_ON),
    sig_def!(PeripheralType::Twai, 0, SIG_CLKOUT),
    sig_def!(PeripheralType::Twai, 0, SIG_STANDBY),
    // TWAI 1
    sig_def!(PeripheralType::Twai, 1, SIG_RSA_REG57),
    sig_def!(PeripheralType::Twai, 1, SIG_RSA_REG26),
    sig_def!(PeripheralType::Twai, 1, SIG_BUS_OFF_ON),
    sig_def!(PeripheralType::Twai, 1, SIG_CLKOUT),
    sig_def!(PeripheralType::Twai, 1, SIG_STANDBY),
    // TWAI 2
    sig_def!(PeripheralType::Twai, 2, SIG_RSA_REG57),
    sig_def!(PeripheralType::Twai, 2, SIG_RSA_REG26),
    sig_def!(PeripheralType::Twai, 2, SIG_BUS_OFF_ON),
    sig_def!(PeripheralType::Twai, 2, SIG_CLKOUT),
    sig_def!(PeripheralType::Twai, 2, SIG_STANDBY),
];

// ── findGpioSignal ──
pub fn find_gpio_signal(peripheral: PeripheralType, index: u32, signal_id: u32) -> Option<&'static GpioSignalInfo> {
    for entry in GPIO_SIGNAL_DEFS {
        if entry.peripheral == peripheral && entry.index == index && entry.signal_id == signal_id {
            return Some(entry);
        }
    }
    None
}
