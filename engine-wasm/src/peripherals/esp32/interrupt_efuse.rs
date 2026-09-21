// Translated from: src/peripherals/esp32/interrupt-efuse.js
// Do NOT modify the logic -- match the JS line-for-line

use crate::peripherals::types::*;

// ============================================================
// Register/Mask Constants (match JS let/const declarations)
// ============================================================

pub const INT_ENABLE_MASK_4: u32 = 0x50400400;
pub const MAX_INT: u32 = 69;

pub const INT_CFG1: u32 = 31;
pub const INT_CFG2: u32 = 392;
pub const INT_CFG3: u32 = 4092;
pub const INT_CFG4: u32 = 0xdffe_773f;
pub const INT_CFG5: u32 = 0xdffe_773f & !0x5040_0400;

pub const INT_CFG6: u32 = 64;
pub const INT_CFG7: u32 = 88;
pub const INT_CFG8: u32 = 44;
pub const INT_CFG9: u32 = 48;
pub const INT_CFG10: u32 = 52;
pub const INT_CFG11: u32 = 60;
pub const INT_CFG12: u32 = 192;
pub const INT_CFG13: u32 = 196;
pub const INT_CFG14: u32 = 220;
pub const INT_CFG15: u32 = 224;
pub const INT_CFG16: u32 = 228;
pub const INT_CFG17: u32 = 232;
pub const INT_CFG18: u32 = 1008;
pub const INT_CFG19: u32 = 1048;
pub const INT_CFG20: u32 = 7;
pub const INT_CFG21: u32 = 8_388_608;
pub const INT_CFG22: u32 = 2_097_152;
pub const INT_CFG23: u32 = 524_288;
pub const INT_CFG24: u32 = 262_144;
pub const INT_CFG25: u32 = 65_536;
pub const INT_CFG26: u32 = 32_768;
pub const INT_CFG27: u32 = 16_384;
pub const INT_CFG28: u32 = 8192;
pub const INT_CFG29: u32 = 2048;
pub const INT_CFG30: u32 = 1024;
pub const INT_CFG31: u32 = 512;
pub const INT_CFG32: u32 = 128;
pub const INT_CFG33: u32 = 64;
pub const INT_CFG34: u32 = 32;
pub const INT_CFG35: u32 = 16;
pub const INT_CFG36: u32 = 4;
pub const INT_CFG37: u32 = 2;
pub const INT_CFG38: u32 = 236;
pub const INT_CFG39: u32 = 240;
pub const INT_CFG40: u32 = 244;
pub const INT_CFG41: u32 = 248;
pub const INT_CFG42: u32 = 252;
pub const INT_CFG43: u32 = 256;
pub const INT_CFG44: u32 = 260;
pub const TIM_REG1: u32 = 536;
pub const TIM_REG2: u32 = TIM_REG1 + 4 * MAX_INT; // 812

// Efuse-related constants
pub const TIM_REG6: u32 = 0;
pub const TIM_REG7: u32 = 12;
pub const TIM_REG8: u32 = 16;
pub const TIM_REG9: u32 = 20;
pub const TIM_REG10: u32 = 2;

// ============================================================
// Fixed-capacity register map (no_std compatible)
// ============================================================

const REG_MAP_CAPACITY: usize = 256;

struct RegEntry {
    offset: u32,
    value: u32,
}

pub struct RegisterMap {
    entries: [RegEntry; REG_MAP_CAPACITY],
    count: usize,
}

impl RegisterMap {
    pub const fn new() -> Self {
        const ZERO: RegEntry = RegEntry { offset: 0, value: 0 };
        RegisterMap {
            entries: [ZERO; REG_MAP_CAPACITY],
            count: 0,
        }
    }

    pub fn read(&self, offset: u32) -> u32 {
        for i in 0..self.count {
            if self.entries[i].offset == offset {
                return self.entries[i].value;
            }
        }
        0
    }

    pub fn write(&mut self, offset: u32, val: u32) {
        for i in 0..self.count {
            if self.entries[i].offset == offset {
                self.entries[i].value = val;
                return;
            }
        }
        if self.count < REG_MAP_CAPACITY {
            self.entries[self.count] = RegEntry { offset, value: val };
            self.count += 1;
        }
    }

    pub fn clear(&mut self) {
        self.count = 0;
    }

    pub fn iter(&self) -> RegisterMapIter {
        RegisterMapIter { map: self, index: 0 }
    }
}

pub struct RegisterMapIter<'a> {
    map: &'a RegisterMap,
    index: usize,
}

impl<'a> Iterator for RegisterMapIter<'a> {
    type Item = (u32, u32);
    fn next(&mut self) -> Option<(u32, u32)> {
        if self.index < self.map.count {
            let e = &self.map.entries[self.index];
            self.index += 1;
            Some((e.offset, e.value))
        } else {
            None
        }
    }
}

// ============================================================
// InterruptMatrixPeripheral (JS lines 18–114)
// ============================================================

pub struct InterruptMatrixConfig {
    pub irqs: u32,
    pub first_intr_map: u32,
    pub status0: u32,
    pub status1: u32,
    pub status2: u32,
    pub status3: i32,
}

pub struct InterruptMatrixPeripheral {
    pub base_addr: u32,
    pub core_idx: u32,
    pub config: InterruptMatrixConfig,
    pub registers: RegisterMap,
}

impl InterruptMatrixPeripheral {
    pub fn new(base_addr: u32, core_idx: u32, config: InterruptMatrixConfig) -> Self {
        InterruptMatrixPeripheral {
            base_addr,
            core_idx,
            config,
            registers: RegisterMap::new(),
        }
    }

    fn read_register(&self, offset: u32) -> u32 {
        self.registers.read(offset)
    }

    fn write_register(&mut self, offset: u32, val: u32) {
        self.registers.write(offset, val);
    }

    fn set_register_bits(&mut self, offset: u32, mask: u32) {
        let v = self.read_register(offset) | mask;
        self.write_register(offset, v);
    }

    fn clear_register_bits(&mut self, offset: u32, mask: u32) {
        let v = self.read_register(offset) & !mask;
        self.write_register(offset, v);
    }

    // JS: lines 24–48  interrupt(cpuVal, tmpVal)
    pub fn interrupt(&mut self, ctx: &mut CpuContext, irq: u32, level: bool) {
        let idx_val = self.config.status0;
        let off1 = self.config.status1;
        let off2 = self.config.status2;
        let reg_val = self.config.status3;

        let arg_val = if irq < 32 {
            idx_val
        } else if irq < 64 {
            off1
        } else if irq < 96 {
            off2
        } else {
            reg_val as u32
        };

        let register_type = 1u32 << (31 & irq);
        let cfg_val = self.read_register(arg_val);

        if level {
            if cfg_val & register_type != 0 {
                return;
            }
            self.set_register_bits(arg_val, register_type);
        } else {
            if cfg_val & register_type == 0 {
                return;
            }
            self.clear_register_bits(arg_val, register_type);
        }
        self.interrupts_updated(ctx);
    }

    // JS: lines 50–81  interruptsUpdated()
    fn interrupts_updated(&mut self, ctx: &mut CpuContext) {
        let irq_count = self.config.irqs;
        let st0_off = self.config.status0;
        let st1_off = self.config.status1;
        let st2_off = self.config.status2;
        let st3_off = self.config.status3;
        let reg_val = self.config.first_intr_map;

        let arg_val = self.read_register(st0_off);
        let register_type = self.read_register(st1_off);
        let cfg_val = self.read_register(st2_off);
        let h_val = if st3_off >= 0 {
            self.read_register(st3_off as u32)
        } else {
            0
        };

        let mut off_val: u32 = 0;

        let start = if arg_val != 0 {
            0
        } else if register_type != 0 {
            32
        } else if cfg_val != 0 {
            64
        } else {
            96
        };

        let mut tmp_val = start;
        while tmp_val < irq_count {
            let src = if tmp_val < 32 {
                arg_val
            } else if tmp_val < 64 {
                register_type
            } else if tmp_val < 96 {
                cfg_val
            } else {
                h_val
            };
            if src & (1u32 << (31 & tmp_val)) != 0 {
                off_val |= 1u32 << (self.read_register(reg_val + (tmp_val << 2)) & INT_CFG1);
            }
            tmp_val += 1;
        }

        let set_bits = off_val & INT_CFG4;
        let clear_bits = !off_val & INT_CFG5;

        // Write directly to CPU core's INT_ENABLE, matching JS this.core.intSetClear(lenVal, valVal)
        let core = unsafe { &mut *crate::xtensa::state::CoreState::from_index(self.core_idx) };
        core.int_set_clear(set_bits, clear_bits);
    }

    // JS: lines 82–86  readUint32(cpuVal)
    pub fn read_u32(&mut self, _ctx: &mut CpuContext, addr: u32) -> u32 {
        let offset = addr.wrapping_sub(self.base_addr);
        if offset == INT_CFG3 {
            return 0x1904180;
        }
        self.read_register(offset)
    }

    // JS: lines 87–102  writeUint32(cpuVal, tmpVal)
    pub fn write_u32(&mut self, ctx: &mut CpuContext, addr: u32, val: u32) {
        let rel_addr = addr.wrapping_sub(self.base_addr);

        self.write_register(rel_addr, val);

        match rel_addr {
            s if s == self.config.status0 => return,
            s if s == self.config.status1 => return,
            s if s == self.config.status2 => return,
            s if s == self.config.status3 as u32 => return,
            _ => {}
        }

        self.write_register(rel_addr, val);

        if rel_addr >= self.config.first_intr_map
            && rel_addr <= self.config.first_intr_map + 4 * self.config.irqs
        {
            self.interrupts_updated(ctx);
        }
    }

    // JS: lines 103–113  reset()
    pub fn reset(&mut self) {
        self.registers.clear();
        self.write_register(self.config.status0, 0);
        self.write_register(self.config.status1, 0);
        self.write_register(self.config.status2, 0);
        if self.config.status3 >= 0 {
            self.write_register(self.config.status3 as u32, 0);
        }
        self.write_register(INT_CFG2, 1);

        for i in 0..self.config.irqs {
            self.write_register(self.config.first_intr_map + 4 * i, 16);
        }
    }
}

// ============================================================
// DportPeripheral (JS lines 156–321)
// ============================================================

pub struct DportClocks {
    pub tg0_timer_enable: bool,
    pub tg0_wdt_enable: bool,
    pub tg1_timer_enable: bool,
    pub tg1_wdt_enable: bool,
    pub ledc_enable: bool,
    pub rmt_enable: bool,
    pub uart0_enable: bool,
    pub uart1_enable: bool,
    pub uart2_enable: bool,
    pub spi2_enable: bool,
    pub i2c0_enable: bool,
}

pub struct DportConfig {
    pub cross_core_irqs: [u32; 4],
    pub clocks: DportClocks,
    pub reset_efuse: Option<fn(&mut CpuContext, bool)>,
    pub reset_i2c0: Option<fn(&mut CpuContext, bool)>,
    pub reset_i2c1: Option<fn(&mut CpuContext, bool)>,
    pub reset_i2s0: Option<fn(&mut CpuContext, bool)>,
    pub reset_i2s1: Option<fn(&mut CpuContext, bool)>,
    pub reset_ledc: Option<fn(&mut CpuContext, bool)>,
    pub reset_pcnt: Option<fn(&mut CpuContext, bool)>,
    pub reset_rmt: Option<fn(&mut CpuContext, bool)>,
    pub reset_spi0: Option<fn(&mut CpuContext, bool)>,
    pub reset_spi1: Option<fn(&mut CpuContext, bool)>,
    pub reset_spi2: Option<fn(&mut CpuContext, bool)>,
    pub reset_spi3: Option<fn(&mut CpuContext, bool)>,
    pub reset_timg0: Option<fn(&mut CpuContext, bool)>,
    pub reset_timg1: Option<fn(&mut CpuContext, bool)>,
    pub reset_twai0: Option<fn(&mut CpuContext, bool)>,
    pub reset_uart0: Option<fn(&mut CpuContext, bool)>,
    pub reset_uart1: Option<fn(&mut CpuContext, bool)>,
    pub reset_uart2: Option<fn(&mut CpuContext, bool)>,
}

pub struct DportPeripheral {
    pub base_addr: u32,
    pub cores_enabled: [bool; 2],
    pub config: DportConfig,
    pub app_clock_gate: bool,
    pub app_stall: bool,
    pub int_matrix: [InterruptMatrixPeripheral; 2],
    pub registers: RegisterMap,
}

impl DportPeripheral {
    pub fn new(
        base_addr: u32,
        config: DportConfig,
    ) -> Self {
        let int_matrix_0_config = InterruptMatrixConfig {
            irqs: MAX_INT,
            first_intr_map: INT_CFG44,
            status0: INT_CFG38,
            status1: INT_CFG39,
            status2: INT_CFG40,
            status3: -1,
        };
        let int_matrix_1_config = InterruptMatrixConfig {
            irqs: MAX_INT,
            first_intr_map: TIM_REG1,
            status0: INT_CFG41,
            status1: INT_CFG42,
            status2: INT_CFG43,
            status3: -1,
        };

        DportPeripheral {
            base_addr,
            cores_enabled: [true, false],
            config,
            app_clock_gate: true,
            app_stall: false,
            int_matrix: [
                InterruptMatrixPeripheral::new(base_addr, 0, int_matrix_0_config),
                InterruptMatrixPeripheral::new(base_addr, 1, int_matrix_1_config),
            ],
            registers: RegisterMap::new(),
        }
    }

    fn read_register(&self, offset: u32) -> u32 {
        self.registers.read(offset)
    }

    fn write_register(&mut self, offset: u32, val: u32) {
        self.registers.write(offset, val);
    }

    // JS: line 230–232  get enableCore1
    fn enable_core1(&self, ctx: &CpuContext) -> bool {
        let stalled = ctx.is_cpu_stalled(1);
        !self.app_stall && self.app_clock_gate && !stalled
    }

    // JS: lines 199–229  readUint32(cpuVal)
    pub fn read_u32(&mut self, ctx: &mut CpuContext, addr: u32) -> u32 {
        let offset = addr.wrapping_sub(self.base_addr);
        match offset {
            INT_CFG38 | INT_CFG39 | INT_CFG40 => self.int_matrix[0].read_u32(ctx, addr),
            INT_CFG41 | INT_CFG42 | INT_CFG43 => self.int_matrix[1].read_u32(ctx, addr),
            INT_CFG6 | INT_CFG7 => 32,
            INT_CFG8 => {
                if !self.cores_enabled[1] { 1 } else { 0 }
            }
            INT_CFG11 => ctx.cpu_clock_period(),
            INT_CFG18 | INT_CFG19 => 1u32 << INT_CFG20,
            68 => 6,
            _ => {
                if offset >= INT_CFG44 && offset < TIM_REG1 {
                    self.int_matrix[0].read_u32(ctx, addr)
                } else if offset >= TIM_REG1 && offset < TIM_REG2 {
                    self.int_matrix[1].read_u32(ctx, addr)
                } else {
                    self.read_register(offset)
                }
            }
        }
    }

    // JS: lines 234–313  writeUint32(cpuVal, tmpVal)
    pub fn write_u32(&mut self, ctx: &mut CpuContext, addr: u32, val: u32) {
        let reg_val = addr.wrapping_sub(self.base_addr);

        if reg_val >= INT_CFG44 && reg_val < TIM_REG1 {
            self.int_matrix[0].write_u32(ctx, addr, val);
            return;
        }
        if reg_val >= TIM_REG1 && reg_val < TIM_REG2 {
            self.int_matrix[1].write_u32(ctx, addr, val);
            return;
        }

        let cc_irqs = &self.config.cross_core_irqs;

        match reg_val {
            INT_CFG8 => {
                if val & 1 == 0 {
                    ctx.reset_core(1);
                }
                // JS also sets: argVal.specialRegisters[IntSet] = 31 (core-internal, skipped)
                self.write_register(reg_val, val);
            }
            INT_CFG9 => {
                self.app_clock_gate = (1 & val) != 0;
                self.write_register(reg_val, val);
            }
            INT_CFG10 => {
                self.app_stall = (1 & val) != 0;
                self.write_register(reg_val, val);
            }
            INT_CFG11 => {
                ctx.set_cpu_clock_period(3 & val);
                ctx.update_clocks();
                self.write_register(reg_val, val);
            }
            INT_CFG12 => {
                self.config.clocks.tg0_timer_enable = (val & INT_CFG28) != 0;
                self.config.clocks.tg0_wdt_enable = (val & INT_CFG28) != 0;
                self.config.clocks.tg1_timer_enable = (val & INT_CFG26) != 0;
                self.config.clocks.tg1_wdt_enable = (val & INT_CFG26) != 0;
                self.config.clocks.ledc_enable = (val & INT_CFG29) != 0;
                self.config.clocks.rmt_enable = (val & INT_CFG31) != 0;
                self.config.clocks.uart0_enable = (val & INT_CFG36) != 0;
                self.config.clocks.uart1_enable = (val & INT_CFG34) != 0;
                self.config.clocks.uart2_enable = (val & INT_CFG21) != 0;
                self.config.clocks.spi2_enable = (val & INT_CFG33) != 0;
                self.config.clocks.i2c0_enable = (val & INT_CFG32) != 0;
                self.write_register(reg_val, val);
                return;
            }
            INT_CFG13 => {
                if let Some(f) = self.config.reset_efuse {
                    f(ctx, (val & INT_CFG27) != 0);
                }
                if let Some(f) = self.config.reset_i2c0 {
                    f(ctx, (val & INT_CFG32) != 0);
                }
                if let Some(f) = self.config.reset_i2c1 {
                    f(ctx, (val & INT_CFG24) != 0);
                }
                if let Some(f) = self.config.reset_i2s0 {
                    f(ctx, (val & INT_CFG35) != 0);
                }
                if let Some(f) = self.config.reset_i2s1 {
                    f(ctx, (val & INT_CFG22) != 0);
                }
                if let Some(f) = self.config.reset_ledc {
                    f(ctx, (val & INT_CFG29) != 0);
                }
                if let Some(f) = self.config.reset_pcnt {
                    f(ctx, (val & INT_CFG30) != 0);
                }
                if let Some(f) = self.config.reset_rmt {
                    f(ctx, (val & INT_CFG31) != 0);
                }
                if let Some(f) = self.config.reset_spi0 {
                    f(ctx, (val & INT_CFG37) != 0);
                }
                if let Some(f) = self.config.reset_spi1 {
                    f(ctx, (val & INT_CFG37) != 0);
                }
                if let Some(f) = self.config.reset_spi2 {
                    f(ctx, (val & INT_CFG33) != 0);
                }
                if let Some(f) = self.config.reset_spi3 {
                    f(ctx, (val & INT_CFG25) != 0);
                }
                if let Some(f) = self.config.reset_timg0 {
                    f(ctx, (val & INT_CFG28) != 0);
                }
                if let Some(f) = self.config.reset_timg1 {
                    f(ctx, (val & INT_CFG26) != 0);
                }
                if let Some(f) = self.config.reset_twai0 {
                    f(ctx, (val & INT_CFG23) != 0);
                }
                if let Some(f) = self.config.reset_uart0 {
                    f(ctx, (val & INT_CFG36) != 0);
                }
                if let Some(f) = self.config.reset_uart1 {
                    f(ctx, (val & INT_CFG34) != 0);
                }
                if let Some(f) = self.config.reset_uart2 {
                    f(ctx, (val & INT_CFG21) != 0);
                }
                self.write_register(reg_val, val);
                return;
            }
            INT_CFG14 => {
                ctx.interrupt(cc_irqs[0], (1 & val) != 0);
                self.write_register(reg_val, val);
            }
            INT_CFG15 => {
                ctx.interrupt(cc_irqs[1], (1 & val) != 0);
                self.write_register(reg_val, val);
            }
            INT_CFG16 => {
                ctx.interrupt(cc_irqs[2], (1 & val) != 0);
                self.write_register(reg_val, val);
            }
            INT_CFG17 => {
                ctx.interrupt(cc_irqs[3], (1 & val) != 0);
                self.write_register(reg_val, val);
            }
            _ => {
                self.write_register(reg_val, val);
            }
        }

        self.cores_enabled[1] = self.enable_core1(ctx);
    }

    // JS: lines 314–320  reset()
    pub fn reset(&mut self) {
        self.registers.clear();
        self.app_clock_gate = true;
        self.app_stall = true;
        self.int_matrix[0].reset();
        self.int_matrix[1].reset();
    }
}

// ============================================================
// EfuseControllerPeripheral (JS lines 327–411)
// ============================================================

pub struct EfuseRmtChannelRegister {
    pub pgm_data6: u32,
    pub rd_sys_part1_data4: u32,
    pub status: u32,
    pub cmd: u32,
    pub rd_key0_data0: i32,
    pub rd_sys_part2_data7: u32,
    pub rd_wr_dis: u32,
}

pub struct EfuseFields {
    pub blk_num_offset: u32,
    pub blk_num_width: u32,
}

pub struct EfuseConfig {
    pub rmt_channel_register: EfuseRmtChannelRegister,
    pub fields: EfuseFields,
    pub initial: RegisterMap,
}

pub struct EfuseControllerPeripheral {
    pub base_addr: u32,
    pub config: EfuseConfig,
    pub cmd: u32,
    pub registers: RegisterMap,
}

impl EfuseControllerPeripheral {
    pub fn new(base_addr: u32, config: EfuseConfig) -> Self {
        EfuseControllerPeripheral {
            base_addr,
            config,
            cmd: 0,
            registers: RegisterMap::new(),
        }
    }

    fn read_register(&self, offset: u32) -> u32 {
        self.registers.read(offset)
    }

    fn write_register(&mut self, offset: u32, val: u32) {
        self.registers.write(offset, val);
    }

    fn set_register_bits(&mut self, offset: u32, mask: u32) {
        let v = self.read_register(offset) | mask;
        self.write_register(offset, v);
    }

    // JS: lines 336–357  readUint32(cpuVal)
    pub fn read_u32(&mut self, _ctx: &mut CpuContext, addr: u32) -> u32 {
        let offset = addr.wrapping_sub(self.base_addr);
        let rcr = &self.config.rmt_channel_register;
        match offset {
            TIM_REG6 => 0,
            TIM_REG7 => 40960,
            TIM_REG8 => 1844,
            TIM_REG9 => 1_048_576,
            o if o == rcr.pgm_data6 => 4,
            o if o == rcr.rd_sys_part1_data4 => 17,
            o if o == rcr.status => 1,
            o if o == rcr.cmd => self.cmd,
            _ => self.read_register(offset),
        }
    }

    // JS: lines 358–395  writeUint32(cpuVal, tmpVal)
    pub fn write_u32(&mut self, ctx: &mut CpuContext, addr: u32, val: u32) {
        let rel_addr = addr.wrapping_sub(self.base_addr);
        let rcr_rd_key0_data0 = self.config.rmt_channel_register.rd_key0_data0;
        let rcr_rd_sys_part2_data7 = self.config.rmt_channel_register.rd_sys_part2_data7;
        let rcr_cmd = self.config.rmt_channel_register.cmd;

        if rel_addr >= rcr_rd_key0_data0 as u32 && rel_addr <= rcr_rd_sys_part2_data7 {
            return;
        }

        if rel_addr == rcr_cmd {
            self.write_register(rel_addr, val);
            self.cmd = val;
            if val & TIM_REG10 != 0 {
                let blk_num_val = if self.config.fields.blk_num_width != 0 {
                    read_field_value(val, self.config.fields.blk_num_offset, self.config.fields.blk_num_width)
                } else {
                    u32::MAX
                };
                let off_addr: i32 = match blk_num_val {
                    0 => self.config.rmt_channel_register.rd_wr_dis as i32,
                    4 => self.config.rmt_channel_register.rd_key0_data0,
                    _ => -1,
                };
                if off_addr >= 0 {
                    for i in 0..8 {
                        let src = self.read_register(4 * i);
                        self.set_register_bits(off_addr as u32 + 4 * i, src);
                    }
                }
            }
            ctx.schedule_event(634400, EventTag::EfuseCmdDone);
        } else {
            self.write_register(rel_addr, val);
        }
    }

    // JS: lines 396–410  reset()
    pub fn reset(&mut self) {
        self.registers.clear();
        self.cmd = 0;

        // Collect initial values before mutating self
        let mut init_buf = [(0u32, 0u32); 64];
        let mut init_len = 0;
        for (k, v) in self.config.initial.iter() {
            if init_len < 64 {
                init_buf[init_len] = (k, v);
                init_len += 1;
            }
        }
        for i in 0..init_len {
            self.write_register(init_buf[i].0, init_buf[i].1);
        }

        let rcr = &self.config.rmt_channel_register;
        if rcr.rd_key0_data0 >= 0 {
            let base = rcr.rd_key0_data0 as u32;
            self.write_register(base, 0x12345678);
            self.write_register(base + 4, 0x87654321);
            self.write_register(base + 8, 0xabcdef01);
            self.write_register(base + 12, 0x10abcdef);
            self.write_register(base + 16, 0x12345678);
            self.write_register(base + 20, 0x87654321);
            self.write_register(base + 24, 0xabcdef01);
            self.write_register(base + 28, 0x10abcdef);
        }
    }
}

// ============================================================
// StubPeripheral (JS lines 412–429)
// ============================================================

pub struct StubRmtChannelRegister {
    pub iq_est: u32,
}

pub struct StubPeripheral {
    pub base_addr: u32,
    pub rmt_channel_register: StubRmtChannelRegister,
    pub registers: RegisterMap,
}

impl StubPeripheral {
    pub fn new(base_addr: u32, rmt: StubRmtChannelRegister) -> Self {
        StubPeripheral {
            base_addr,
            rmt_channel_register: rmt,
            registers: RegisterMap::new(),
        }
    }

    fn read_register(&self, offset: u32) -> u32 {
        self.registers.read(offset)
    }

    // JS: lines 416–428  readUint32(cpuVal)
    pub fn read_u32(&mut self, _ctx: &mut CpuContext, addr: u32) -> u32 {
        let offset = addr.wrapping_sub(self.base_addr);
        match offset {
            o if o == self.rmt_channel_register.iq_est => 0xffffffff,
            24 => 0xffffffff,
            12 => 114688,
            128 => 4112,
            _ => self.read_register(offset),
        }
    }
}

// ============================================================
// Helper: readFieldValue (JS global helper used in EfuseControllerPeripheral)
// ============================================================

fn read_field_value(val: u32, offset: u32, width: u32) -> u32 {
    if width == 0 {
        return 0;
    }
    (val >> offset) & ((1u32 << width) - 1)
}
