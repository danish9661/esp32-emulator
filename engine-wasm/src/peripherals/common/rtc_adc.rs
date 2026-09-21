// Translated from: src/peripherals/common/rtc-adc.js
// Do NOT modify the logic — match the JS line-for-line

use crate::peripherals::types::*;
use crate::peripherals::common::peripheral::PeripheralBase;

// ============================================================
// Constants (match JS let/const declarations)
// ============================================================

// AdcPeripheral / shared constants
const UART_REG12: u32 = 44;
const UART_REG13: u32 = 84;
const UART_REG14: u32 = 148;
const UART_REG15: u32 = 4095;
const UART_REG16: u32 = 19;
const UART_REG17: u32 = 65535;
const UART_REG18: u32 = 0;
const UART_REG19: u32 = 65536;
const UART_REG20: u32 = 131072;
const UART_REG21: u32 = 262144;
const UART_REG22: u32 = 0xFFFC0000; // -262144 as u32

// SENS_SAR_ATTEN1/2_REG = DR_REG_SENS_BASE + 0x34 / + 0x38 (real HW per-channel
// attenuation; the IDF adc_oneshot driver writes these via adc_ll_set_atten).
// Not part of the JS parity (the JS emulator only ever saw the start-force
// bit-width field) — added for attenuation-accurate analog reads.
const SAR_ATTEN1_OFF: u32 = 52;
const SAR_ATTEN2_OFF: u32 = 56;

// SENS touch_meas[5] (SENS_SAR_TOUCH_OUT1..5_REG, +0x70..+0x80): each u32 holds
// two pads, low 16 = l_val, high 16 = h_val. Pad order per register follows
// touch_ll_read_raw_data with the 8<->9 channel swap:
//   meas[0..3] = pads (0,1),(2,3),(4,5),(6,7) as (h,l); meas[4] = (pad9,pad8).
const TOUCH_MEAS_OFFS: [u32; 5] = [112, 116, 120, 124, 128];
const TOUCH_MEAS_PADS: [(u32, u32); 5] = [(0, 1), (2, 3), (4, 5), (6, 7), (9, 8)];

// SENS_SAR_DAC_CTRL2_REG (+0x9C): dac_dc1 [7:0] = ch0 (GPIO25),
// dac_dc2 [15:8] = ch1 (GPIO26). Written by dac_oneshot_output_voltage.
const SAR_DAC_CTRL2_OFF: u32 = 156;

// SENS_SAR_TOUCH_CTRL2_REG (+0x84): touch_meas_en [9:0],
// touch_meas_done [10], trigger/fsm control above.
const TOUCH_CTRL2_OFF: u32 = 132;

// RtcCntlPeripheral constants
const UART_REG23: u32 = 0;
const UART_REG24: u32 = 4;
const UART_REG25: u32 = 8;
const UART_REG26: u32 = 0;
const UART_REG27: u32 = 2;
const UART_REG28: u32 = 20;
const UART_REG29: u32 = 26;
const UART_REG30: u32 = 134;
const UART_REG31: u32 = 12;
const UART_REG32: u32 = 0x80000000; // -0x80000000 as u32
const UART_REG33: u32 = 16;
const UART_REG34: u32 = 20;
const UART_REG35: u32 = 24;
const UART_REG36: u32 = 0x80000000; // -0x80000000 as u32
const UART_REG37: u32 = 0x80000000; // -0x80000000 as u32
const UART_REG38: u32 = 256;
const UART_REG39: u32 = 1;
const UART_REG40: u32 = 65536;
const UART_REG41: u32 = 8;
const UART_REG42: u32 = 16384;
const UART_REG43: u32 = 16;
const UART_REG44: u32 = 32;
const UART_REG45: u32 = 0x80000000; // -0x80000000 as u32
const UART_REG46: u32 = 3;
const UART_REG47: u32 = 5;
const UART_REG48: u32 = 12;
const UART_REG49: u32 = 0;
const UART_REG50: u32 = 0;
const UART_REG51: u32 = 0x20000000;
const UART_REG52: u32 = 27;
const RTC_CLK_SRC_FIELD_MASK: u32 = 3;

const FLASH_REG1: u32 = 64;

// ============================================================
// Helper: findFirstSetBit (from bit-helpers.js)
// Returns Some(idx) for the least-significant set bit (0-based),
// or None if val is 0.
// ============================================================
fn find_first_set_bit(val: u32) -> Option<u32> {
    if val == 0 {
        None
    } else {
        Some(val.trailing_zeros())
    }
}

// ============================================================
// ADC config struct (matches JS Adc1Config / Adc2Config objects)
// ============================================================
#[derive(Clone, Copy)]
pub struct AdcConfig {
    pub pins: &'static [u32],
    pub sample_nanos: u32,
    pub reg: u32,
    pub bits_shift: u32,
    pub bits_mask: u32,
}

pub const ADC1_CONFIG: AdcConfig = AdcConfig {
    pins: &[36, 37, 38, 39, 32, 33, 34, 35],
    sample_nanos: 9500,
    reg: 0,
    bits_shift: 0,
    bits_mask: 3,
};

pub const ADC2_CONFIG: AdcConfig = AdcConfig {
    pins: &[4, 0, 2, 15, 13, 12, 14, 27, 25, 26],
    sample_nanos: 11500,
    reg: 1,
    bits_shift: 2,
    bits_mask: 3,
};

// ============================================================
// RTC IO config: register offsets used by RtcIoPeripheral
// ============================================================
pub struct RtcIoChannelRegister {
    pub pad_dac1: u32,
    pub pad_dac2: u32,
    pub xtal_32k_pad: u32,
    pub touch_pad0: u32,
    pub touch_pad1: u32,
    pub touch_pad2: u32,
    pub touch_pad3: u32,
    pub touch_pad4: u32,
    pub touch_pad5: u32,
    pub touch_pad6: u32,
    pub touch_pad7: u32,
}

pub struct RtcIoConfig {
    pub rmt_channel_register: RtcIoChannelRegister,
    pub touch_pad_gpio: &'static [u32],
    pub dac_gpio: &'static [u32],
    pub xtal_gpio: &'static [u32],
}

// ============================================================
// RTC CNTL config: register offsets used by RtcCntlPeripheral
// ============================================================
pub struct RmtChannelRegisterOffsets {
    pub clk_conf: u32,
    pub reset_state: u32,
    pub ana_conf: u32,
    pub sw_cpu_stall: u32,
    pub store0: u32,
    pub store1: u32,
    pub store2: u32,
    pub store3: u32,
    pub store4: u32,
    pub store5: u32,
    pub store6: u32,
    pub store7: u32,
    pub dig_pwc: u32,
    pub int_raw_rtc: u32,
    pub int_clr_rtc: u32,
    pub slp_wakeup_cause: i32,
}

pub struct LightSleepClocks {
    pub resume_apb_clocks: fn(&mut CpuContext),
    pub pause_apb_clocks: fn(&mut CpuContext),
}

pub struct RtcCntlConfig {
    pub rmt_channel_register: RmtChannelRegisterOffsets,
    pub light_sleep_clocks: LightSleepClocks,
    pub irq: i32,
    pub strap_read_offset: i32,
}

// ============================================================
// RtcIoPeripheral
// JS: class RtcIoPeripheral extends PeripheralBase { ... }
// ============================================================
pub struct RtcIoPeripheral {
    pub base: PeripheralBase,
    pub config: RtcIoConfig,
}

impl RtcIoPeripheral {
    pub fn new(base_addr: u32, name: &'static str, config: RtcIoConfig) -> Self {
        RtcIoPeripheral {
            base: PeripheralBase::new(base_addr, name),
            config,
        }
    }

    // JS: updateResistors(cpuVal, tmpVal, idxVal)
    // this.cpu.gpio.pins[pin] → ctx.gpio_pins[pin]
    pub fn update_resistors(&mut self, ctx: &mut CpuContext, pin: u32, val: u32, shift: u32) {
        let gpio_pin = &mut ctx.gpio_pins[pin as usize];
        // ClockEvent.rtcPullUp = !!(tmpVal & (1 << idxVal))
        let rtc_pull_up = (val & (1u32 << shift)) != 0;
        // ClockEvent.rtcPullDown = !!(tmpVal & (1 << (idxVal + 1)))
        let rtc_pull_down = (val & (1u32 << (shift + 1))) != 0;
        // ClockEvent.update()
        // NOTE: GpioPinState is missing rtc_pull_up, rtc_pull_down, update().
        // Approximate: pull-up drives high, pull-down drives low.
        if rtc_pull_up {
            gpio_pin.input_value = 1;
        } else if rtc_pull_down {
            gpio_pin.input_value = 0;
        }
    }
}

impl MmioPeripheral for RtcIoPeripheral {
    fn read_u32(&mut self, _ctx: &mut CpuContext, addr: u32) -> u32 {
        // RtcIoPeripheral has no custom readUint32 — uses super.readUint32
        self.base.read_uint32(addr)
    }

    fn write_u32(&mut self, ctx: &mut CpuContext, addr: u32, val: u32) {
        let offset = addr - self.base.base_addr;
        let ch = &self.config.rmt_channel_register;
        let touch_pad = self.config.touch_pad_gpio;
        let dac = self.config.dac_gpio;
        let xtal = self.config.xtal_gpio;

        match offset {
            _ if offset == ch.pad_dac1 => {
                self.update_resistors(ctx, dac[0], val, 27);
                // dac_oneshot_output_voltage writes the 8-bit DAC code here
                // (RTCIO pad_dac.dac field, bits 26:19). Virtual wire to ADC.
                unsafe {
                    crate::peripherals::common::ffi::js_dac_write(0, (val >> 19) & 0xFF);
                }
            }
            _ if offset == ch.pad_dac2 => {
                self.update_resistors(ctx, dac[1], val, 27);
                unsafe {
                    crate::peripherals::common::ffi::js_dac_write(1, (val >> 19) & 0xFF);
                }
            }
            _ if offset == ch.xtal_32k_pad => {
                // JS: argVal && (updateResistors(argVal[0]...), updateResistors(argVal[1]...))
                if xtal.len() >= 2 {
                    self.update_resistors(ctx, xtal[0], val, 22);
                    self.update_resistors(ctx, xtal[1], val, 27);
                }
            }
            _ if offset == ch.touch_pad0 => {
                self.update_resistors(ctx, touch_pad[0], val, 27);
            }
            _ if offset == ch.touch_pad1 => {
                self.update_resistors(ctx, touch_pad[1], val, 27);
            }
            _ if offset == ch.touch_pad2 => {
                self.update_resistors(ctx, touch_pad[2], val, 27);
            }
            _ if offset == ch.touch_pad3 => {
                self.update_resistors(ctx, touch_pad[3], val, 27);
            }
            _ if offset == ch.touch_pad4 => {
                self.update_resistors(ctx, touch_pad[4], val, 27);
            }
            _ if offset == ch.touch_pad5 => {
                self.update_resistors(ctx, touch_pad[5], val, 27);
            }
            _ if offset == ch.touch_pad6 => {
                self.update_resistors(ctx, touch_pad[6], val, 27);
            }
            _ if offset == ch.touch_pad7 => {
                self.update_resistors(ctx, touch_pad[7], val, 27);
            }
            _ => {}
        }
        // super.writeUint32(cpuVal, tmpVal) — write to memory after switch
        self.base.write_uint32(addr, val);
    }

    fn reset(&mut self) {
        self.base.reset();
    }
}

// ============================================================
// RtcI2cPeripheral (0x3FF48C00) + virtual RTC-I2C slave table
// ============================================================
// The ESP32 RTC_I2C controller has no IDF driver (ULP/ROM-only); the model
// covers what firmware + ULP programs actually touch:
// - Register file with TRM offsets (CTRL/SLAVE_ADDR/DATA/INT_*_REG...).
// - Firmware master WRITE: SLAVE_ADDR + DATA, then CTRL MS_MODE|TRANS_START
//   performs a single-byte write to the virtual slave (pointer model: the
//   first byte after programming selects the sub-address, following bytes
//   are data, like I2C EEPROMs), sets MASTER_TRANS_COMPLETE +
//   TRANS_COMPLETE, and auto-clears TRANS_START (pollable completion).
//   Firmware-driven READ has no direction bit on this controller (reads are
//   ULP-only on real HW) — DATA reads return the last-result mirror.
// - ULP I2C instruction (opcode 3, in ulp.rs) uses the same slave table
//   with full R/W (slave address from SENS_I2C_SLAVE_ADDRx).
// Slave table entries are EXTERNAL devices: they survive SoC reset (never
// cleared by reset — a fresh worker/WASM instance starts empty).

const RTC_I2C_CTRL_OFF: u32 = 0x04;
const RTC_I2C_SLAVE_ADDR_OFF: u32 = 0x10;
const RTC_I2C_DATA_OFF: u32 = 0x1C;
const RTC_I2C_INT_RAW_OFF: u32 = 0x20;
const RTC_I2C_INT_CLR_OFF: u32 = 0x24;
const RTC_I2C_MS_MODE_BIT: u32 = 1 << 4;
const RTC_I2C_TRANS_START_BIT: u32 = 1 << 5;
const RTC_I2C_MASTER_DONE_BIT: u32 = 1 << 5;
const RTC_I2C_TRANS_DONE_BIT: u32 = 1 << 6;

struct RtcI2cSlave {
    used: bool,
    addr: u8,
    file: [u8; 256],
    ptr: u8,
    ptr_set: bool,
}

static mut RTC_I2C_SLAVES: [RtcI2cSlave; 4] = [
    RtcI2cSlave { used: false, addr: 0, file: [0; 256], ptr: 0, ptr_set: false },
    RtcI2cSlave { used: false, addr: 0, file: [0; 256], ptr: 0, ptr_set: false },
    RtcI2cSlave { used: false, addr: 0, file: [0; 256], ptr: 0, ptr_set: false },
    RtcI2cSlave { used: false, addr: 0, file: [0; 256], ptr: 0, ptr_set: false },
];

fn rtc_i2c_slot(slave7: u8) -> Option<usize> {
    if slave7 == 0 {
        return None; // address 0 = unprogrammed/general-call: no match
    }
    unsafe {
        for i in 0..4 {
            if RTC_I2C_SLAVES[i].used && RTC_I2C_SLAVES[i].addr == slave7 {
                return Some(i);
            }
        }
        for i in 0..4 {
            if !RTC_I2C_SLAVES[i].used {
                RTC_I2C_SLAVES[i].used = true;
                RTC_I2C_SLAVES[i].addr = slave7;
                RTC_I2C_SLAVES[i].ptr_set = false;
                return Some(i);
            }
        }
        None // table full: drop (documented)
    }
}

/// ULP/controller slave read. 0xFF when no device answers (NACK parity).
pub(crate) fn rtc_i2c_slave_read(slave7: u8, sub: u8) -> u8 {
    match rtc_i2c_slot(slave7) {
        Some(i) => unsafe { RTC_I2C_SLAVES[i].file[sub as usize] },
        None => 0xFF,
    }
}

/// ULP slave write (explicit sub-address).
pub(crate) fn rtc_i2c_slave_write(slave7: u8, sub: u8, val: u8) {
    if let Some(i) = rtc_i2c_slot(slave7) {
        unsafe {
            RTC_I2C_SLAVES[i].file[sub as usize] = val;
        }
    }
}

/// Firmware-controller write (pointer model: first byte selects sub-address).
pub(crate) fn rtc_i2c_controller_write(slave7: u8, byte: u8) {
    if let Some(i) = rtc_i2c_slot(slave7) {
        unsafe {
            if !RTC_I2C_SLAVES[i].ptr_set {
                RTC_I2C_SLAVES[i].ptr = byte;
                RTC_I2C_SLAVES[i].ptr_set = true;
            } else {
                let p = RTC_I2C_SLAVES[i].ptr;
                RTC_I2C_SLAVES[i].file[p as usize] = byte;
                RTC_I2C_SLAVES[i].ptr = p.wrapping_add(1);
            }
        }
    }
}

pub struct RtcI2cPeripheral {
    pub base: PeripheralBase,
}

impl RtcI2cPeripheral {
    pub fn new(base_addr: u32, name: &'static str) -> Self {
        RtcI2cPeripheral {
            base: PeripheralBase::new(base_addr, name),
        }
    }
}

impl MmioPeripheral for RtcI2cPeripheral {
    fn read_u32(&mut self, _ctx: &mut CpuContext, addr: u32) -> u32 {
        self.base.read_uint32(addr)
    }

    fn write_u32(&mut self, _ctx: &mut CpuContext, addr: u32, val: u32) {
        // super.writeUint32 first (register file is the source of truth;
        // DATA reads return the last-result mirror written below).
        self.base.write_uint32(addr, val);
        let offset = addr - self.base.base_addr;
        // INT_CLR (W/O): CLR bit N clears RAW bit N-1 (8->7 ... 4->3).
        if offset == RTC_I2C_INT_CLR_OFF {
            let cur = self.base.read_register(RTC_I2C_INT_RAW_OFF);
            self.base.write_register(
                RTC_I2C_INT_RAW_OFF,
                cur & !((val >> 1) & 0xF8),
            );
            return;
        }
        // CTRL with MS_MODE|TRANS_START: single-byte master write.
        if offset == RTC_I2C_CTRL_OFF
            && (val & (RTC_I2C_MS_MODE_BIT | RTC_I2C_TRANS_START_BIT))
                == (RTC_I2C_MS_MODE_BIT | RTC_I2C_TRANS_START_BIT)
        {
            let slave7 = (self.base.read_register(RTC_I2C_SLAVE_ADDR_OFF) & 0x7F) as u8;
            let byte = (self.base.read_register(RTC_I2C_DATA_OFF) & 0xFF) as u8;
            rtc_i2c_controller_write(slave7, byte);
            self.base.set_register_bits(
                RTC_I2C_INT_RAW_OFF,
                RTC_I2C_MASTER_DONE_BIT | RTC_I2C_TRANS_DONE_BIT,
            );
            // Completion: HW auto-clears TRANS_START (firmware polls it).
            self.base
                .clear_register_bits(RTC_I2C_CTRL_OFF, RTC_I2C_TRANS_START_BIT);
        }
    }

    fn reset(&mut self) {
        self.base.reset();
    }
}

// ============================================================
// AdcPeripheral
// JS: class AdcPeripheral extends PeripheralBase { ... }
// ============================================================
pub struct AdcPeripheral {
    pub base: PeripheralBase,
    // JS: sarReg = new Uint32Array(2) → [u32; 2]
    pub sar_reg: [u32; 2],
    // JS: startForceReg = 0
    pub start_force_reg: u32,
    // JS: pendingResult = [0, 0]
    pub pending_result: [u32; 2],
    // SENS_SAR_ATTEN1/2 (2-bit attenuation per channel). HW reset = 0xFFFFFFFF
    // (11dB default on every channel); the IDF oneshot driver writes per-pin
    // attenuation here via adc_ll_set_atten.
    pub sar_atten: [u32; 2],
}

impl AdcPeripheral {
    pub fn new(base_addr: u32, name: &'static str) -> Self {
        AdcPeripheral {
            base: PeripheralBase::new(base_addr, name),
            sar_reg: [0; 2],
            start_force_reg: 0,
            pending_result: [0; 2],
            sar_atten: [0xFFFFFFFF; 2],
        }
    }

    // JS: completeAdcMeasurement(cpuVal) — cpuVal is the ADC unit index (0 or 1)
    // Called when the ADC measurement event fires.
    pub fn complete_adc_measurement(&mut self, unit: u32) {
        let mut tmp_val = self.sar_reg[unit as usize];
        // tmpVal &= ~(uartReg17 << uartReg18)
        tmp_val &= !(UART_REG17 << UART_REG18);
        // tmpVal |= ((this.pendingResult[unit] & uartReg17) << uartReg18) | uartReg19
        tmp_val |= ((self.pending_result[unit as usize] & UART_REG17) << UART_REG18) | UART_REG19;
        // this.sarReg[unit] = tmpVal
        self.sar_reg[unit as usize] = tmp_val;
    }

    // JS: adcMeasurement(config, regVal)
    // config is destructured: { pins, bitsMask, bitsShift, sampleNanos, reg }
    pub fn adc_measurement(&mut self, _ctx: &mut CpuContext, config: &AdcConfig, reg_val: u32) {
        let reg = config.reg as usize;
        // (this.sarReg[SimulationClock] = regVal & uartReg22)
        self.sar_reg[reg] = reg_val & UART_REG22;
        // if (!(regVal & uartReg20)) return;
        if (reg_val & UART_REG20) == 0 {
            return;
        }
        // if (!(regVal & uartReg21)) { this.sarReg[SimulationClock] |= uartReg19; return; }
        if (reg_val & UART_REG21) == 0 {
            self.sar_reg[reg] |= UART_REG19;
            return;
        }
        // let argVal = findFirstSetBit((regVal >> uartReg16) & uartReg15)
        let arg_val = find_first_set_bit((reg_val >> UART_REG16) & UART_REG15);
        // RegisterType = pins[argVal]
        let pin = arg_val.and_then(|i| config.pins.get(i as usize).copied()).unwrap_or(0);
        // Effective attenuation: the real per-channel SENS_SAR_ATTEN1/2 field
        // (2 bits per channel, HW default 11dB = 3). Fall back to the JS-parity
        // start-force bit-width field only if the driver never wrote SAR_ATTEN
        // (all-zero — older drivers / bootloader).
        let atten = {
            let ch = arg_val.unwrap_or(0) as usize;
            let per_channel = (self.sar_atten[reg] >> (ch * 2)) & 3;
            if per_channel != 0 {
                per_channel
            } else {
                (self.start_force_reg >> config.bits_shift) & config.bits_mask
            }
        };
        // cfgVal = 9 + attenuation
        let cfg_val = 9u32 + atten;
        // this.pendingResult[SimulationClock] = null != argVal ? this.cpu.onAnalogRead(pin, cfgVal) : 0
        let analog_val = if arg_val.is_some() {
            unsafe { crate::peripherals::common::ffi::js_on_analog_read(pin, cfg_val) }
        } else {
            0u32
        };
        self.pending_result[reg] = analog_val;
        // this.adcDoneEvents[SimulationClock].schedule(sampleNanos) — native clock
        // event fires complete_adc_measurement(reg) (sample_nanos → 80MHz APB ticks).
        crate::peripherals::common::spi_syscon::schedule_global(
            (config.sample_nanos as u64 * 80) / 1000,
            EventTag::AdcDone { unit: reg as u32 },
        );
    }

    // JS: readUint32
    pub fn read_u32_inner(&mut self, _ctx: &mut CpuContext, addr: u32) -> u32 {
        let offset = addr - self.base.base_addr;
        // SENS_SAR_SLAVE_ADDR3_REG (+0x44): TSENS_OUT lives in bits[29:22]
        // (temprature_sens_read returns (reg >> 22) & 0xFF as Fahrenheit).
        // Serve a plausible room temperature: 77F = 25C. The low 22 bits
        // are the live register file (I2C_SLAVE_ADDR4 [21:11] + ADDR5
        // [10:0] for the ULP I2C instruction) — a whole-register constant
        // would wipe firmware-programmed slave addresses on readback.
        if offset == 0x44 {
            return (77u32 << 22) | (self.base.read_register(offset) & 0x3FFFFF);
        }
        match offset {
            UART_REG12 => self.start_force_reg,
            UART_REG13 => self.sar_reg[0],
            UART_REG14 => self.sar_reg[1],
            SAR_ATTEN1_OFF => self.sar_atten[0],
            SAR_ATTEN2_OFF => self.sar_atten[1],
            _ => {
                // Touch measurement output: compose both pads from the host
                // (config.touchInputs, default = untouched count).
                for i in 0..5 {
                    if offset == TOUCH_MEAS_OFFS[i] {
                        let (hp, lp) = TOUCH_MEAS_PADS[i];
                        let h = unsafe {
                            crate::peripherals::common::ffi::js_on_touch_read(hp)
                        } & 0xFFFF;
                        let l = unsafe {
                            crate::peripherals::common::ffi::js_on_touch_read(lp)
                        } & 0xFFFF;
                        return (h << 16) | l;
                    }
                }
                // All other SENS registers (thresholds, CTRL1/CTRL2 incl.
                // touch_meas_done, DAC_CTRL1, ...) read back the register
                // file — the old `0` broke read-back (CTRL2 DONE, FSM mode).
                self.base.read_register(offset)
            }
        }
    }

    // JS: writeUint32
    pub fn write_u32_inner(&mut self, ctx: &mut CpuContext, addr: u32, val: u32) {
        // super.writeUint32(cpuVal, tmpVal)
        self.base.write_uint32(addr, val);
        let offset = addr - self.base.base_addr;
        match offset {
            UART_REG12 => {
                self.start_force_reg = val;
            }
            UART_REG13 => {
                self.adc_measurement(ctx, &ADC1_CONFIG, val);
            }
            UART_REG14 => {
                self.adc_measurement(ctx, &ADC2_CONFIG, val);
            }
            SAR_ATTEN1_OFF => {
                self.sar_atten[0] = val;
            }
            SAR_ATTEN2_OFF => {
                self.sar_atten[1] = val;
            }
            SAR_DAC_CTRL2_OFF => {
                // DAC cosine-wave path (dac_dc1/dc2); the oneshot path lands
                // in RTC_IO PAD_DAC (see RtcIoPeripheral) and runs after this.
                unsafe {
                    crate::peripherals::common::ffi::js_dac_write(0, val & 0xFF);
                    crate::peripherals::common::ffi::js_dac_write(1, (val >> 8) & 0xFF);
                }
            }
            TOUCH_CTRL2_OFF => {
                // Touch FSM completes instantly (BT-RF-bit31 pattern): the
                // driver polls touch_meas_done after triggering one-shot
                // measurement; real HW sets it when the FSM finishes.
                self.base.set_register_bits(TOUCH_CTRL2_OFF, 1 << 10);
            }
            _ => {}
        }
    }

    // JS: reset
    pub fn reset_inner(&mut self) {
        // super.reset()
        self.base.reset();
        // this.sarReg.fill(0)
        self.sar_reg = [0; 2];
        // HW reset: attenuation defaults to 11dB on every channel
        self.sar_atten = [0xFFFFFFFF; 2];
    }
}

impl MmioPeripheral for AdcPeripheral {
    fn read_u32(&mut self, ctx: &mut CpuContext, addr: u32) -> u32 {
        self.read_u32_inner(ctx, addr)
    }

    fn write_u32(&mut self, ctx: &mut CpuContext, addr: u32, val: u32) {
        self.write_u32_inner(ctx, addr, val);
    }

    fn reset(&mut self) {
        self.reset_inner();
    }
}

// ============================================================
// RtcCntlPeripheral
// JS: class RtcCntlPeripheral extends PeripheralBase { ... }
// ============================================================
pub struct RtcCntlPeripheral {
    pub base: PeripheralBase,
    pub config: RtcCntlConfig,
    // JS: rtcOptions0 = 0
    pub rtc_options0: u32,
    // JS: rtcSwCpuStall = 0
    pub rtc_sw_cpu_stall: u32,
    // JS: rtcClkConf = uartReg50 | uartReg51 | uartReg49
    pub rtc_clk_conf: u32,
    // JS: strapReadOffset = ClockEvent.strapReadOffset ?? -1
    pub strap_read_offset: i32,
    // JS: storeRegisters = new Uint32Array(8)
    pub store_registers: [u32; 8],
    // JS: slpTimer0 = 0
    pub slp_timer0: u32,
    // JS: slpTimer1 = 0
    pub slp_timer1: u32,
    // JS: deepSleepPending = false
    pub deep_sleep_pending: bool,
    // JS: sleepWakeupEvent — event handle (replaced by flags + deferred scheduling)
    // Flag to cancel wakeup during reset (since MmioPeripheral::reset has no ctx)
    pub sleep_wakeup_cancelled: bool,
    // Brownout detector: host-driven VDD in mV (default nominal 3.3V) +
    // last evaluated DET state (edge detect). Reset restores nominal so a
    // brownout reboot doesn't immediately re-fire before the host acts.
    pub vdd_mv: u32,
    pub bod_det: bool,
    // Previous ENA+RST_ENA arming (transition detect for the reset path).
    pub bod_armed: bool,
    // WAKEUP_ENA[10:0] snapshot taken at deep-sleep entry. SoC reset
    // re-seeds SLP_WAKEUP_CAUSE (wiping firmware's enable bitmap), so the
    // wake path cannot read it post-reset — snapshot pre-reset instead for
    // wake-cause attribution (ULP vs timer).
    pub sleep_entry_wakeup_ena: u32,
}

// Brownout threshold select -> mV (approximate ESP32 datasheet levels).
pub const BOD_THRES_MV: [u32; 8] = [2430, 2560, 2690, 2810, 2940, 3060, 3190, 3320];

// RTC_CNTL register offsets (rown from DR_REG_RTCCNTL_BASE).
pub const BOD_REG_OFF: u32 = 0xD4;
const BOD_DET_BIT: u32 = 1 << 31;
const BOD_ENA_BIT: u32 = 1 << 30;
const BOD_RST_ENA_BIT: u32 = 1 << 26;
const BOD_INT_BIT: u32 = 1 << 7;
const INT_ST_OFF: u32 = 0x44;
const INT_ENA_OFF: u32 = 0x3C;

impl RtcCntlPeripheral {
    pub fn new(base_addr: u32, name: &'static str, config: RtcCntlConfig) -> Self {
        let strap_read_offset = config.strap_read_offset;
        RtcCntlPeripheral {
            base: PeripheralBase::new(base_addr, name),
            config,
            rtc_options0: 0,
            rtc_sw_cpu_stall: 0,
            rtc_clk_conf: UART_REG50 | UART_REG51 | UART_REG49,
            strap_read_offset,
            store_registers: [0; 8],
            slp_timer0: 0,
            slp_timer1: 0,
            deep_sleep_pending: false,
            sleep_wakeup_cancelled: false,
            vdd_mv: 3300,
            bod_det: false,
            bod_armed: false,
            sleep_entry_wakeup_ena: 0,
        }
    }

    // JS: onSleepWakeup callback (from constructor sleepWakeupEvent)
    // Called when the sleep wakeup timer fires.
    pub fn on_sleep_wakeup(&mut self, ctx: &mut CpuContext) {
        if self.sleep_wakeup_cancelled {
            self.sleep_wakeup_cancelled = false;
            return;
        }
        if self.deep_sleep_pending {
            // this.deepSleepPending = false
            self.deep_sleep_pending = false;
            // this.cpu.resetReason = uartReg47
            ctx.set_reset_reason(UART_REG47);
            // Read the ULP WAKE latch BEFORE reset_soc(): SoC reset clears
            // it (ulp_reset via native_rtc_reset). Cleared here either way
            // so a stale WAKE can't leak into a later sleep cycle.
            let ulp_woke = crate::peripherals::common::ulp::ulp_wake_latched();
            crate::peripherals::common::ulp::ulp_wake_clear();
            // this.cpu.onReset() && this.cpu.reset()
            if ctx.on_reset() {
                ctx.reset_soc();
            }
            // this.config.RmtChannelRegister.SLP_WAKEUP_CAUSE >= 0 &&
            //   this.setRegisterBits(this.config.RmtChannelRegister.SLP_WAKEUP_CAUSE, uartReg41)
            if self.config.rmt_channel_register.slp_wakeup_cause >= 0 {
                let cause_off = self.config.rmt_channel_register.slp_wakeup_cause as u32;
                // ULP WAKE latch (I_WAKE executed by the ULP program, read
                // above before reset_soc) wins over the timer bit when ULP
                // wakeup was enabled at sleep entry (snapshot — reset wipes
                // the live register) — first-trigger race parity (the latch
                // means the ULP already fired).
                // ULP trigger = RTC_ULP_TRIG_EN = BIT(9); TIMER = BIT(3)
                // (= uartReg41, the historical unconditional set).
                if ulp_woke && (self.sleep_entry_wakeup_ena & (1 << 9)) != 0 {
                    self.base.set_register_bits(cause_off, 1 << 9);
                } else {
                    self.base.set_register_bits(cause_off, UART_REG41);
                }
            }
        } else {
            // this.config.lightSleepClocks.resumeApbClocks()
            (self.config.light_sleep_clocks.resume_apb_clocks)(ctx);
            // this.config.RmtChannelRegister.SLP_WAKEUP_CAUSE >= 0 &&
            //   this.setRegisterBits(this.config.RmtChannelRegister.SLP_WAKEUP_CAUSE, uartReg41)
            if self.config.rmt_channel_register.slp_wakeup_cause >= 0 {
                self.base.set_register_bits(
                    self.config.rmt_channel_register.slp_wakeup_cause as u32,
                    UART_REG41,
                );
            }
            // this.setRegisterBits(this.config.RmtChannelRegister.INT_RAW_RTC, uartReg39)
            self.base.set_register_bits(self.config.rmt_channel_register.int_raw_rtc, UART_REG39);
            // this.cpu.cores → for each core: cpuVal.exitLightSleep()
            ctx.exit_light_sleep(0);
            ctx.exit_light_sleep(1);
            // this.config.irq >= 0 && this.cpu.interrupt(this.config.irq, true)
            if self.config.irq >= 0 {
                ctx.interrupt(self.config.irq as u32, true);
            }
        }
    }

    // JS: getStoreIndex(cpuVal)
    pub fn get_store_index(&self, offset: u32) -> i32 {
        let stores = [
            self.config.rmt_channel_register.store0,
            self.config.rmt_channel_register.store1,
            self.config.rmt_channel_register.store2,
            self.config.rmt_channel_register.store3,
            self.config.rmt_channel_register.store4,
            self.config.rmt_channel_register.store5,
            self.config.rmt_channel_register.store6,
            self.config.rmt_channel_register.store7,
        ];
        for i in 0..stores.len() {
            if stores[i] == offset {
                return i as i32;
            }
        }
        -1
    }

    // JS: getSleepTimerTicks()
    // (this.slpTimer0 >>> 0) + 0x100000000 * ((65535 & this.slpTimer1) >>> 0)
    pub fn get_sleep_timer_ticks(&self) -> u64 {
        (self.slp_timer0 as u64) + 0x100000000u64 * ((self.slp_timer1 & 65535) as u64)
    }

    // JS: scheduleSleepWakeup()
    pub fn schedule_sleep_wakeup(&mut self, _ctx: &mut CpuContext) {
        // JS: let cpuVal = Math.max(0, this.getSleepTimerTicks() - (rcSlow?.ticks ?? 0));
        //     this.sleepWakeupEvent?.schedule(cpuVal); (fires native_rtc_fire_sleep_wakeup)
        // Native: rcSlow (32.768kHz) target converted to APB ticks on the shared
        // queue (80e6 / 32768 ≈ 2441.40625 APB ticks per rcSlow tick).
        let target = self.get_sleep_timer_ticks();
        let current = crate::native_mmio::clk_rtc_slow();
        let delta = target.saturating_sub(current);
        crate::peripherals::common::spi_syscon::schedule_global(delta * 2441, EventTag::RtcSlowWakeup);
        self.sleep_wakeup_cancelled = false;
    }

    // JS: isTimerWakeupEnabled()
    pub fn is_timer_wakeup_enabled(&mut self) -> bool {
        // !!(this.slpTimer1 & uartReg40)
        if (self.slp_timer1 & UART_REG40) != 0 {
            return true;
        }
        // this.config.RmtChannelRegister.SLP_WAKEUP_CAUSE >= 0 &&
        //   !!(this.readRegister(this.config.RmtChannelRegister.SLP_WAKEUP_CAUSE) & uartReg42)
        if self.config.rmt_channel_register.slp_wakeup_cause >= 0 {
            let wakeup_cause_val = self.base.read_register(self.config.rmt_channel_register.slp_wakeup_cause as u32);
            if (wakeup_cause_val & UART_REG42) != 0 {
                return true;
            }
        }
        false
    }

    // JS: isCpuStalled(cpuVal)
    pub fn is_cpu_stalled_inner(&self, core_idx: u32) -> bool {
        let tmp_val = if core_idx == 0 { UART_REG27 } else { UART_REG26 };
        let idx_val = if core_idx == 0 { UART_REG29 } else { UART_REG28 };
        // ClockEvent = (this.rtcOptions0 >> tmpVal) & 3
        let clock_event = (self.rtc_options0 >> tmp_val) & 3;
        // ((((this.rtcSwCpuStall >> idxVal) & 63) << 2) | ClockEvent) === uartReg30
        ((((self.rtc_sw_cpu_stall >> idx_val) & 63) << 2) | clock_event) == UART_REG30
    }

    // JS: updateCpuStallState()
    pub fn update_cpu_stall_state(&mut self, ctx: &mut CpuContext) {
        // if ("esp32" !== this.cpu.chipName) return;
        if ctx.chip_name != "esp32" {
            return;
        }
        // let cpuVal = this.cpu;
        // cpuVal.cores[0].enabled = !this.isCpuStalled(0)
        ctx.set_core_enabled(0, !self.is_cpu_stalled_inner(0));
        // cpuVal.cores[1].enabled = cpuVal.dport.enableCore1
        ctx.set_core_enabled(1, crate::native_mmio::dport_core1_enabled());
    }

    // JS: updateResistors(cpuVal, tmpVal, idxVal)
    // Same implementation as RtcIoPeripheral.updateResistors
    pub fn update_resistors(&mut self, ctx: &mut CpuContext, pin: u32, val: u32, shift: u32) {
        let gpio_pin = &mut ctx.gpio_pins[pin as usize];
        let rtc_pull_up = (val & (1u32 << shift)) != 0;
        let rtc_pull_down = (val & (1u32 << (shift + 1))) != 0;
        if rtc_pull_up {
            gpio_pin.input_value = 1;
        } else if rtc_pull_down {
            gpio_pin.input_value = 0;
        }
    }

    // JS: readUint32(cpuVal)
    pub fn read_u32_inner(&mut self, ctx: &mut CpuContext, addr: u32) -> u32 {
        let offset = addr - self.base.base_addr;
        let ch = &self.config.rmt_channel_register;

        // let argVal = this.getStoreIndex(tmpVal)
        let store_idx = self.get_store_index(offset);
        // if (argVal >= 0) return this.storeRegisters[argVal]
        if store_idx >= 0 {
            return self.store_registers[store_idx as usize];
        }
        // if (tmpVal === this.config.strapReadOffset && this.cpu.gpio)
        //   return this.cpu.gpio.strapValue
        if offset as i32 == self.strap_read_offset {
            // NOTE: gpio_strap_value() not yet on CpuContext
            return 0;
        }

        match offset {
            UART_REG23 => self.rtc_options0,
            UART_REG24 => self.slp_timer0,
            UART_REG25 => self.slp_timer1,
            _ if offset == ch.sw_cpu_stall => self.rtc_sw_cpu_stall,
            UART_REG31 => 0x40000000,
            _ if offset == ch.reset_state => {
                // JS: return this.cpu.resetReason
                ctx.reset_reason()
            }
            _ if offset == ch.ana_conf => 12353,
            UART_REG38 => 0x80000000,
            _ if offset == ch.clk_conf => self.rtc_clk_conf,
            // BROWN_OUT_REG: DET (bit31) is live comparator state.
            _ if offset == BOD_REG_OFF => {
                let cfg = self.base.read_register(BOD_REG_OFF);
                let det = (cfg & BOD_ENA_BIT) != 0
                    && self.vdd_mv < BOD_THRES_MV[((cfg >> 27) & 7) as usize];
                (cfg & !BOD_DET_BIT) | ((det as u32) << 31)
            }
            // INT_ST: OR in the latched BOD bit (RAW & ENA); other bits
            // keep plain-file behavior.
            _ if offset == INT_ST_OFF => {
                let raw = self.base.read_register(ch.int_raw_rtc);
                let ena = self.base.read_register(INT_ENA_OFF);
                self.base.read_uint32(addr) | (raw & ena & BOD_INT_BIT)
            }
            _ => self.base.read_uint32(addr),
        }
    }

    // JS: writeUint32(cpuVal, tmpVal)
    pub fn write_u32_inner(&mut self, ctx: &mut CpuContext, addr: u32, val: u32) {
        // super.writeUint32(cpuVal, tmpVal) — always write to memory first
        self.base.write_uint32(addr, val);

        let offset = addr - self.base.base_addr;
        // let ClockEvent = this.getStoreIndex(idxVal)
        let store_idx = self.get_store_index(offset);
        // if (ClockEvent >= 0) { this.storeRegisters[ClockEvent] = tmpVal; return; }
        if store_idx >= 0 {
            self.store_registers[store_idx as usize] = val;
            return;
        }

        // Copy fields before match to avoid borrow conflict
        let sw_cpu_stall = self.config.rmt_channel_register.sw_cpu_stall;
        let clk_conf = self.config.rmt_channel_register.clk_conf;
        let dig_pwc = self.config.rmt_channel_register.dig_pwc;
        let int_raw_rtc = self.config.rmt_channel_register.int_raw_rtc;
        let int_clr_rtc = self.config.rmt_channel_register.int_clr_rtc;

        match offset {
            UART_REG24 => {
                self.slp_timer0 = val;
                return;
            }
            UART_REG25 => {
                self.slp_timer1 = val;
                return;
            }
            UART_REG31 => {
                // if (tmpVal & uartReg32) { ... }
                if (val & UART_REG32) != 0 {
                    // let cpuVal = this.cpu.clocks.rcSlow?.ticks ?? 0
                    // NOTE: rc_slow.ticks not available on Clocks in types.rs
                    let rc_slow_ticks: u64 = 0;
                    // this.writeRegister(uartReg33, cpuVal >>> 0)
                    self.base.write_register(UART_REG33, rc_slow_ticks as u32);
                    // this.writeRegister(uartReg34, Math.floor(cpuVal / 0x100000000))
                    self.base.write_register(UART_REG34, (rc_slow_ticks >> 32) as u32);
                }
                // break → falls through to final super.writeUint32
            }
            UART_REG23 => {
                // tmpVal & uartReg45 && this.cpu.onReset() &&
                //   ((this.cpu.resetReason = uartReg46), this.cpu.reset())
                if (val & UART_REG45) != 0 && ctx.on_reset() {
                    ctx.set_reset_reason(UART_REG46);
                    ctx.reset_soc();
                }
                // tmpVal & uartReg44 &&
                //   ((this.cpu.resetReason = uartReg48),
                //    this.cpu.sensitiveReg?.zeroMemory(),
                //    this.cpu.cores[0].reset())
                if (val & UART_REG44) != 0 {
                    ctx.set_reset_reason(UART_REG48);
                    ctx.zero_sensitive_memory();
                    ctx.reset_core(0);
                }
                // tmpVal & uartReg43 && this.cpu.cores[1].reset()
                if (val & UART_REG43) != 0 {
                    ctx.reset_core(1);
                }
                // this.rtcOptions0 = tmpVal & ~(uartReg45 | uartReg43 | uartReg44)
                self.rtc_options0 = val & !(UART_REG45 | UART_REG43 | UART_REG44);
                // this.updateCpuStallState()
                self.update_cpu_stall_state(ctx);
                return;
            }
            _ if offset == sw_cpu_stall => {
                // ((this.rtcSwCpuStall = tmpVal), this.updateCpuStallState())
                self.rtc_sw_cpu_stall = val;
                self.update_cpu_stall_state(ctx);
                return;
            }
            _ if offset == clk_conf => {
                // ((this.rtcClkConf = tmpVal),
                self.rtc_clk_conf = val;
                // "esp32" === this.cpu.chipName &&
                if ctx.chip_name == "esp32" {
                    // (this.cpu.clocks.cpuClockSource = (tmpVal >> uartReg52) & RtcClkSrcFieldMask)
                    // NOTE: set_cpu_clock_source() not yet on CpuContext/Clocks
                    // this.cpu.clocks.update()
                    ctx.update_clocks();
                }
                return;
            }
            UART_REG35 => {
                // if (tmpVal & uartReg36) { ... }
                if (val & UART_REG36) != 0 {
                    // if (this.readRegister(regVal) & uartReg37) { ... }
                    if (self.base.read_register(dig_pwc) & UART_REG37) != 0 {
                        if self.is_timer_wakeup_enabled() {
                            // for (let cpuVal of ((this.deepSleepPending = true),
                            //   this.scheduleSleepWakeup(), this.cpu.cores))
                            //   cpuVal.enabled = false;
                            self.deep_sleep_pending = true;
                            // Snapshot WAKEUP_ENA now: reset_soc re-seeds
                            // SLP_WAKEUP_CAUSE, so post-reset reads see only
                            // defaults. Used for ULP-vs-timer attribution.
                            if self.config.rmt_channel_register.slp_wakeup_cause >= 0 {
                                let wstate = self.base.read_register(
                                    self.config.rmt_channel_register.slp_wakeup_cause as u32,
                                );
                                self.sleep_entry_wakeup_ena = (wstate >> 11) & 0x7FF;
                            }
                            self.schedule_sleep_wakeup(ctx);
                            ctx.set_core_enabled(0, false);
                            ctx.set_core_enabled(1, false);
                        } else if ctx.on_reset() {
                            // for (let cpuVal of this.cpu.cores) cpuVal.reset()
                            ctx.reset_core(0);
                            ctx.reset_core(1);
                        }
                    } else if self.is_timer_wakeup_enabled() {
                        // for (let cpuVal of (this.config.lightSleepClocks.pauseApbClocks(),
                        //   this.scheduleSleepWakeup(), this.cpu.cores))
                        //   cpuVal.enterLightSleep();
                        (self.config.light_sleep_clocks.pause_apb_clocks)(ctx);
                        self.schedule_sleep_wakeup(ctx);
                        ctx.enter_light_sleep(0);
                        ctx.enter_light_sleep(1);
                    } else {
                        // this.setRegisterBits(argVal, uartReg39)
                        self.base.set_register_bits(int_raw_rtc, UART_REG39);
                    }
                    return;
                }
                // break → falls through to final super.writeUint32
            }
            _ if offset == int_clr_rtc => {
                // tmpVal & uartReg39 &&
                if (val & UART_REG39) != 0 {
                    // this.clearRegisterBits(argVal, uartReg39)
                    self.base.clear_register_bits(int_raw_rtc, UART_REG39);
                    // this.config.irq >= 0 && this.cpu.interrupt(this.config.irq, false)
                    if self.config.irq >= 0 {
                        ctx.interrupt(self.config.irq as u32, false);
                    }
                }
                // Brownout INT_CLR bit7: clear the latched BOD bit; drop
                // the irq line when nothing remains (RAW & ENA).
                if (val & BOD_INT_BIT) != 0 {
                    self.base.clear_register_bits(int_raw_rtc, BOD_INT_BIT);
                    let raw = self.base.read_register(int_raw_rtc);
                    let ena = self.base.read_register(INT_ENA_OFF);
                    if (raw & ena) == 0 && self.config.irq >= 0 {
                        ctx.interrupt(self.config.irq as u32, false);
                    }
                }
                return;
            }
            // BROWN_OUT_REG writes re-evaluate the detector (plain store
            // already applied above).
            _ if offset == BOD_REG_OFF => {
                self.evaluate_bod(ctx);
            }
            // ULP force-start (RTCCNTL+0x2C bit15, set by ulp_run; entry
            // word address in SENS+0x2C[21:11]): execute the loaded program
            // synchronously to HALT/END.
            _ if offset == 0x2C => {
                if (val & (1 << 15)) != 0 {
                    let sens = crate::xtensa::memory::dma_read_u32(0x3FF4882C);
                    let entry = (sens >> 11) & 0x7FF;
                    crate::peripherals::common::ulp::ulp_start(entry);
                }
            }
            _ => {
                // fall through to final super.writeUint32
            }
        }
        // super.writeUint32(cpuVal, tmpVal) — for unhandled or break cases
        self.base.write_uint32(addr, val);
    }

    // JS: reset()
    pub fn reset_inner(&mut self) {
        // super.reset()
        self.base.reset();
        // this.rtcOptions0 = 0
        self.rtc_options0 = 0;
        // this.rtcSwCpuStall = 0
        self.rtc_sw_cpu_stall = 0;
        // this.rtcClkConf = uartReg50 | uartReg51 | uartReg49
        self.rtc_clk_conf = UART_REG50 | UART_REG51 | UART_REG49;
        // this.slpTimer0 = 0
        self.slp_timer0 = 0;
        // this.slpTimer1 = 0
        self.slp_timer1 = 0;
        // this.deepSleepPending || this.sleepWakeupEvent?.unschedule()
        // Without ctx, we set a cancelled flag checked by on_sleep_wakeup.
        if !self.deep_sleep_pending {
            self.sleep_wakeup_cancelled = true;
        }
        // Brownout: supply restored to nominal (a brownout reboot must not
        // immediately re-fire before the host drives the rail again).
        self.vdd_mv = 3300;
        self.bod_det = false;
        self.bod_armed = false;
    }

    // Brownout detector evaluation. Runs after BROWN_OUT_REG writes and
    // after host voltage changes: DET = ENA && VDD < threshold(select).
    // INT_RAW latches on the DET rising edge. The reset fires on DET
    // rising OR on RST_ENA arming while already browned-out — but never
    // repeatedly without a transition (reset-during-reset would wedge
    // the boot ROM in a WDT loop). The irq line recomputes from RAW & ENA.
    pub fn evaluate_bod(&mut self, ctx: &mut CpuContext) {
        let cfg = self.base.read_register(BOD_REG_OFF);
        let det = (cfg & BOD_ENA_BIT) != 0
            && self.vdd_mv < BOD_THRES_MV[((cfg >> 27) & 7) as usize];
        let armed = (cfg & (BOD_ENA_BIT | BOD_RST_ENA_BIT)) == (BOD_ENA_BIT | BOD_RST_ENA_BIT);
        let rising = det && !self.bod_det;
        let rearm = det && armed && !self.bod_armed;
        self.bod_det = det;
        self.bod_armed = armed;
        if rising {
            self.base
                .set_register_bits(self.config.rmt_channel_register.int_raw_rtc, BOD_INT_BIT);
        }
        if (rising || rearm) && armed {
            if ctx.on_reset() {
                // NOTE: the ROM banner renders this as RTCWDT_SYS_RESET
                // (reset_soc doesn't set the SW_SYS_RST cause bit the way
                // esp_restart does); esp_reset_reason() == 9 (BROWNOUT) is
                // authoritative and DRAM/RTC mem survive like SW reset.
                ctx.set_reset_reason(9); // ESP_RST_BROWNOUT
                ctx.reset_soc();
            }
            return;
        }
        // Assert-only: never disturb other RTC interrupt users (sleep paths
        // manage the shared line themselves; INT_CLR handles release).
        let raw = self.base.read_register(self.config.rmt_channel_register.int_raw_rtc);
        let ena = self.base.read_register(INT_ENA_OFF);
        if (raw & ena & BOD_INT_BIT) != 0 && self.config.irq >= 0 {
            ctx.interrupt(self.config.irq as u32, true);
        }
    }
}

impl MmioPeripheral for RtcCntlPeripheral {
    fn read_u32(&mut self, ctx: &mut CpuContext, addr: u32) -> u32 {
        self.read_u32_inner(ctx, addr)
    }

    fn write_u32(&mut self, ctx: &mut CpuContext, addr: u32, val: u32) {
        self.write_u32_inner(ctx, addr, val);
    }

    fn reset(&mut self) {
        self.reset_inner();
    }
}
