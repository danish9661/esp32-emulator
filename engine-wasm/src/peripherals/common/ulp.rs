// ESP32 ULP (Ultra Low Power, FSM) coprocessor interpreter.
//
// Executes ULP programs loaded into RTC_SLOW_MEM (0x50000000) via
// ulp_process_macros_and_load + ulp_run. Trigger: RTC_CNTL+0x2C bit15
// rising (the force-start bit ulp_run sets after programming the entry
// address into SENS+0x2C[21:11]); execution is synchronous run-to-HALT
// (bounded), matching what the test observes (no deep sleep needed).
//
// ISA reference: components/hal/esp32/include/hal/adc_ll.h-style header
// `ulp/ulp_fsm/include/esp32/ulp.h` (union ulp_insn layouts + I_* macros).
// Registers R0-R3 are 16-bit with wraparound; PC/addresses are word indexes
// into RTC slow memory.
//
// Implemented: DELAY(nop), HALT, END (sub 0 = stop, or latch WAKE + keep
// running per the I_WAKE doc; sub 1 = sleep-cycle select, keep running per
// the I_SLEEP_CYCLE_SEL doc), ST/LD (16-bit via word RMW), ALU (all REG/IMM/
// CNT ops + zero/overflow flags), BRANCH (bx/b/bs), ADC (via
// js_on_analog_read, 11dB, ADC1/ADC2 pad maps), TSENS (=77F, matching the
// TSENS stub), WR_REG/RD_REG (routed to the live native RTC_CNTL/RTC_IO/
// SENS register files through rtc_read/write_region, so firmware observes
// ULP writes via REG_READ and vice versa; R0 = reg[high:low] right-aligned;
// fields wider than the 8-bit data / 16-bit R0 are truncated, documented).
// I2C (slave address from SENS_I2C_SLAVE_ADDRx, R0 result on read) runs
// against the virtual RTC-I2C slave table in rtc_adc.rs (shared with the
// firmware-driven controller path); unprogrammed slave = 0xFF NACK parity.
// WAKE latches ULP_WAKE_LATCH, consumed by the deep-sleep wake path (ULP
// cause wins over timer when ULP wakeup is enabled — first-trigger race
// parity). I2C + periph_sel 3 (RTC_I2C, no engine) stay stubbed; MACRO
// halts defensively (never in loaded binaries).
//
// Reentrancy: ULP runs synchronously INSIDE the RTCCNTL+0x2C write hook, so
// a WR_REG targeting that same register would recurse — ULP_BUSY guards
// nested ulp_start. The interpreter state itself is a stack local (no
// static aliasing): region calls return values by value, never re-read
// through a live borrow (the I2C slave ACK lesson).
//
// RTC_SLOW_MEM access goes through dma_read/write_u32 (native page-table
// resolution falls back to the JS rtcSlowMem SAB — the same store the main
// CPU uses, so ULP writes are immediately visible to firmware reads).

use crate::xtensa::memory::{dma_read_u32, dma_write_u32};

pub const RTC_SLOW_BASE: u32 = 0x50000000;
const MAX_STEPS: u32 = 65536;

// WR_REG/RD_REG peripheral bases (RD_REG_PERIPH_* from esp32/ulp.h:
// 0 = RTC_CNTL, 1 = RTC_IO, 2 = SENS/SARADC, 3 = RTC_I2C).
// addr field = word index within the 1KB page (I_WR_REG: (reg/4) & 0xff).
const ULP_PERIPH_BASES: [u32; 4] = [0x3FF48000, 0x3FF48400, 0x3FF48800, 0x3FF48C00];

// ULP wakeup-cause trigger bit in RTC_CNTL_WAKEUP_STATE_REG (offset 56):
// RTC_ULP_TRIG_EN = BIT(9) (TIMER = BIT(3), TOUCH = BIT(8), BT = BIT(10)).
pub const ULP_WAKEUP_TRIGGER_BIT: u32 = 1 << 9;

static mut ULP_WAKE_LATCH: bool = false;
static mut ULP_BUSY: bool = false;

/// Latched by I_WAKE; consumed (and cleared) by the deep-sleep wake path.
pub fn ulp_wake_latched() -> bool {
    unsafe { ULP_WAKE_LATCH }
}

pub fn ulp_wake_clear() {
    unsafe {
        ULP_WAKE_LATCH = false;
    }
}

/// SoC reset parity: a stale WAKE must not leak into a later sleep cycle.
pub fn ulp_reset() {
    unsafe {
        ULP_WAKE_LATCH = false;
        ULP_BUSY = false;
    }
}

// Start handshake (see RtcCntl write hook): ulp_run programs the entry
// word address into SENS+0x2C[21:11] and then sets RTCCNTL+0x2C bit15.
// The hook reads the entry fresh from the SENS register file, so no
// cached state is needed. Reentrancy-guarded: a WR_REG back into
// RTCCNTL+0x2C bit15 would otherwise recurse unboundedly.
pub fn ulp_start(entry: u32) -> u32 {
    unsafe {
        if ULP_BUSY {
            return 0;
        }
        ULP_BUSY = true;
    }
    let n = ulp_run_sync(entry & 0x7FF);
    unsafe {
        ULP_BUSY = false;
    }
    n
}

// ADC1 channels 0-7 -> GPIO, ADC2 channels 0-7 -> GPIO (IDF pad mapping).
const ADC1_GPIO_MAP: [u32; 8] = [36, 37, 38, 39, 32, 33, 34, 35];
const ADC2_GPIO_MAP: [u32; 8] = [4, 0, 2, 15, 13, 12, 14, 27];

fn rtc_word(addr_word: u32) -> u32 {
    dma_read_u32(RTC_SLOW_BASE.wrapping_add(addr_word.wrapping_mul(4)))
}

fn rtc_write_word(addr_word: u32, val: u32) {
    dma_write_u32(RTC_SLOW_BASE.wrapping_add(addr_word.wrapping_mul(4)), val);
}

fn rtc_read16(addr_word: u32) -> u16 {
    (rtc_word(addr_word) & 0xFFFF) as u16
}

fn rtc_write16(addr_word: u32, val: u16) {
    let w = rtc_word(addr_word);
    rtc_write_word(addr_word, (w & 0xFFFF0000) | (val as u32));
}

pub struct UlpState {
    pub r: [u16; 4],
    pub pc: u32,
    pub stage: u16,
    pub zero: bool,
    pub ovf: bool,
    pub sleep_cycle_sel: u8,
}

impl UlpState {
    pub fn new(entry: u32) -> Self {
        UlpState {
            r: [0u16; 4],
            pc: entry,
            stage: 0,
            zero: false,
            ovf: false,
            sleep_cycle_sel: 0,
        }
    }
}

fn alu_op(sel: u32, a: u16, b: u16) -> (u16, bool) {
    match sel {
        0 => {
            // ADD, ovf = carry out of bit 15
            let r = (a as u32) + (b as u32);
            ((r & 0xFFFF) as u16, r > 0xFFFF)
        }
        1 => {
            // SUB, ovf = borrow
            ((a.wrapping_sub(b)) & 0xFFFF, (a as u32) < (b as u32))
        }
        2 => (a & b, false),
        3 => (a | b, false),
        4 => (b, false), // MOV: dest = operand B (imm) or src (reg form uses b=src)
        5 => (((a as u32) << (b & 15)) as u16, false),
        6 => (a >> (b & 15), false),
        _ => (0, false),
    }
}

/// Execute from `entry` (word address) until HALT/END or step bound.
/// Returns the number of instructions executed.
pub fn ulp_run_sync(entry: u32) -> u32 {
    let mut st = UlpState::new(entry & 0x7FF);
    let mut steps = 0u32;
    loop {
        if steps >= MAX_STEPS {
            break;
        }
        steps += 1;
        let w = rtc_word(st.pc);
        let opcode = w >> 28;
        match opcode {
            4 => {
                // DELAY: nop (cycles ignored — synchronous execution).
            }
            11 => break, // HALT
            9 => {
                // END. sub 0 (SUB_OPCODE_END): wakeup bit latches the ULP
                // wake trigger but execution CONTINUES until HALT (I_WAKE
                // doc: "ULP program will still keep running until the
                // I_HALT instruction"); wakeup=0 stops. sub 1
                // (SUB_OPCODE_SLEEP / I_SLEEP_CYCLE_SEL): record the
                // timer-select, keep running (timer re-run not modeled).
                let sub = (w >> 25) & 7;
                if sub == 0 {
                    if (w & 1) != 0 {
                        unsafe {
                            ULP_WAKE_LATCH = true;
                        }
                    } else {
                        break;
                    }
                } else if sub == 1 {
                    st.sleep_cycle_sel = (w & 0xF) as u8;
                } else {
                    break;
                }
            }
            15 => break, // MACRO token: never in loaded binaries; stop safely.
            6 => {
                // ST: Mem[R[sreg]+offset] = R[dreg] (16-bit).
                let dreg = (w & 3) as usize;
                let sreg = ((w >> 2) & 3) as usize;
                let off = (w >> 10) & 0x7FF;
                let addr = st.r[sreg] as u32 + off;
                rtc_write16(addr, st.r[dreg]);
            }
            13 => {
                // LD: R[dreg] = Mem[R[sreg]+offset] (16-bit, zero-extended).
                let dreg = (w & 3) as usize;
                let sreg = ((w >> 2) & 3) as usize;
                let off = (w >> 10) & 0x7FF;
                let addr = st.r[sreg] as u32 + off;
                st.r[dreg] = rtc_read16(addr);
            }
            7 => {
                // ALU.
                let sub = (w >> 25) & 7;
                if sub == 0 {
                    // REG form.
                    let dreg = (w & 3) as usize;
                    let sreg = ((w >> 2) & 3) as usize;
                    let treg = ((w >> 4) & 3) as usize;
                    let sel = (w >> 21) & 0xF;
                    let (r, ovf) = if sel == 4 {
                        // MOV reg: dest = src (operand A).
                        (st.r[sreg], false)
                    } else {
                        alu_op(sel, st.r[sreg], st.r[treg])
                    };
                    st.r[dreg] = r;
                    st.zero = r == 0;
                    st.ovf = ovf;
                } else if sub == 1 {
                    // IMM form.
                    let dreg = (w & 3) as usize;
                    let sreg = ((w >> 2) & 3) as usize;
                    let imm = ((w >> 4) & 0xFFFF) as u16;
                    let sel = (w >> 21) & 0xF;
                    let (r, ovf) = if sel == 4 {
                        // MOVI: dest = imm.
                        (imm, false)
                    } else {
                        alu_op(sel, st.r[sreg], imm)
                    };
                    st.r[dreg] = r;
                    st.zero = r == 0;
                    st.ovf = ovf;
                } else {
                    // CNT form: stage counter ops (imm unused).
                    let sel = (w >> 21) & 0xF;
                    match sel {
                        0 => st.stage = st.stage.wrapping_add(1),
                        1 => st.stage = st.stage.wrapping_sub(1),
                        _ => st.stage = 0,
                    }
                }
            }
            8 => {
                // BRANCH.
                let sub = (w >> 25) & 7;
                if sub == 0 {
                    // BX absolute.
                    let typ = (w >> 14) & 7;
                    let take = match typ {
                        0 => true,
                        1 => st.zero,
                        _ => st.ovf,
                    };
                    if take {
                        if ((w >> 13) & 1) != 0 {
                            st.pc = st.r[(w & 3) as usize] as u32;
                        } else {
                            st.pc = (w >> 2) & 0x7FF;
                        }
                        continue;
                    }
                } else if sub == 1 {
                    // B relative on R0.
                    let imm = (w & 0xFFFF) as u16;
                    let cmp = (w >> 16) & 1;
                    let off = (w >> 17) & 0x7F;
                    let sign = (w >> 24) & 1;
                    let take = if cmp == 0 {
                        st.r[0] < imm
                    } else {
                        st.r[0] >= imm
                    };
                    if take {
                        if sign != 0 {
                            st.pc = st.pc.wrapping_add(1).wrapping_sub(off);
                        } else {
                            st.pc = st.pc.wrapping_add(1).wrapping_add(off);
                        }
                        continue;
                    }
                } else {
                    // BS relative on stage counter.
                    let imm = (w & 0xFF) as u16;
                    let cmp = (w >> 15) & 3;
                    let off = (w >> 17) & 0x7F;
                    let sign = (w >> 24) & 1;
                    let take = match cmp {
                        0 => st.stage < imm,
                        1 => st.stage >= imm,
                        _ => st.stage <= imm,
                    };
                    if take {
                        if sign != 0 {
                            st.pc = st.pc.wrapping_add(1).wrapping_sub(off);
                        } else {
                            st.pc = st.pc.wrapping_add(1).wrapping_add(off);
                        }
                        continue;
                    }
                }
            }
            5 => {
                // ADC: SAR measurement via host analog inputs (11dB).
                let dreg = (w & 3) as usize;
                let mux = ((w >> 2) & 0xF) as usize;
                let sar_sel = (w >> 6) & 1;
                let pin = if sar_sel == 0 {
                    if mux < 8 { ADC1_GPIO_MAP[mux] } else { 36 }
                } else if mux < 8 {
                    ADC2_GPIO_MAP[mux]
                } else {
                    4
                };
                let v = unsafe { crate::peripherals::common::ffi::js_on_analog_read(pin, 12) };
                st.r[dreg] = (v & 0xFFF) as u16;
            }
            10 => {
                // TSENS: Fahrenheit, matching the TSENS stub (77F = 25C).
                let dreg = (w & 3) as usize;
                st.r[dreg] = 77;
            }
            1 | 2 => {
                // WR_REG / RD_REG: live RTC register-file access (routing
                // through the same region handlers the CPU uses, so all
                // side-effect hooks fire and firmware observes the writes).
                // addr = word index in page (I_WR_REG: (reg/4) & 0xff);
                // R0 = reg[high:low] right-aligned (I_RD_REG doc).
                let addr8 = w & 0xFF;
                let periph = ((w >> 8) & 3) as usize;
                let low = (w >> 18) & 0x1F;
                let high = (w >> 23) & 0x1F;
                if periph < 4 && high >= low {
                    let reg = ULP_PERIPH_BASES[periph] + (addr8 << 2);
                    let width = high - low + 1;
                    let fmask = if width >= 32 {
                        0xFFFF_FFFF
                    } else {
                        (1u32 << width) - 1
                    };
                    if opcode == 1 {
                        // reg[high:low] = data (8-bit, low-aligned; wider
                        // fields zero-extend — documented assumption).
                        let data = (w >> 10) & 0xFF;
                        let mask = fmask << low;
                        let cur = crate::native_mmio::rtc_read_region(reg, 4);
                        let new = (cur & !mask) | ((data << low) & mask);
                        crate::native_mmio::rtc_write_region(reg, new, 4);
                    } else {
                        // R0 truncates to 16 bits (documented).
                        let val = crate::native_mmio::rtc_read_region(reg, 4);
                        st.r[0] = ((val >> low) & fmask & 0xFFFF) as u16;
                    }
                }
                // (no RTC-I2C engine yet — dropped; high < low: no-op).
            }
            3 => {
                // I2C: RTC-I2C transaction against the virtual slave table.
                // Slave 7-bit address comes from SENS_I2C_SLAVE_ADDRx
                // (x = i2c_sel & 7; regs 0x3C/0x40/0x44/0x48, even sel =
                // bits[21:11], odd sel = bits[10:0]; low 7 bits are the
                // address — documented assumption). i2c_addr field = slave
                // sub-address. READ (rw=0, SUB_OPCODE_I2C_RD) loads R0;
                // WRITE (rw=1) stores val[high:low] (I_I2C_WRITE: full
                // byte). Unprogrammed sel (addr 0) reads 0xFF (NACK parity).
                let sub = (w & 0xFF) as u8;
                let data = ((w >> 8) & 0xFF) as u8;
                let low = (w >> 16) & 7;
                let high = (w >> 19) & 7;
                let sel = ((w >> 22) & 0xF) as usize;
                let rw = (w >> 27) & 1;
                if sel < 8 {
                    let sreg = crate::native_mmio::rtc_read_region(
                        0x3FF48800 + 0x3C + 4 * ((sel / 2) as u32),
                        4,
                    );
                    let field = if sel % 2 == 0 {
                        (sreg >> 11) & 0x7FF
                    } else {
                        sreg & 0x7FF
                    };
                    let slave7 = (field & 0x7F) as u8;
                    if rw == 0 {
                        let b = crate::peripherals::common::rtc_adc::rtc_i2c_slave_read(
                            slave7, sub,
                        );
                        st.r[0] = b as u16;
                        // TRM debug visibility: last read result mirrors
                        // into RTC_I2C DATA_REG + completion bits.
                        crate::native_mmio::rtc_write_region(0x3FF48C1C, b as u32, 4);
                        let raw = crate::native_mmio::rtc_read_region(0x3FF48C20, 4);
                        crate::native_mmio::rtc_write_region(
                            0x3FF48C20,
                            raw | (1 << 5) | (1 << 6),
                            4,
                        );
                    } else if high >= low {
                        let width = high - low + 1;
                        let fmask = if width >= 8 {
                            0xFF
                        } else {
                            (1u32 << width) - 1
                        };
                        let cur = crate::peripherals::common::rtc_adc::rtc_i2c_slave_read(
                            slave7, sub,
                        ) as u32;
                        let mask = fmask << low;
                        let new = ((cur & !mask) | ((data as u32) << low) & mask) as u8;
                        crate::peripherals::common::rtc_adc::rtc_i2c_slave_write(
                            slave7, sub, new,
                        );
                    }
                }
            }
            _ => {}
        }
        st.pc = st.pc.wrapping_add(1) & 0x7FF;
    }
    steps
}
