use super::constants::*;

// SAB offsets for shared CPU tick/cycle counts (stored in core0's _pad slots, written by JS after each clock.tick)
// _pad[0] at struct offset 3992, _pad[1] at 3996
pub const SAB_OFFSET_TICKS: u32 = 3992;
pub const SAB_OFFSET_CYCLES: u32 = 3996;

pub fn sab_ticks() -> u32 { unsafe { *(SAB_OFFSET_TICKS as *const u32) } }
pub fn sab_cycles() -> u32 { unsafe { *(SAB_OFFSET_CYCLES as *const u32) } }

// WASM linear memory layout offsets (used by JS to find/initialize data)
// The page table and RAM/flash mirrors live ABOVE the Rust static-data
// zone (statics occupy [~0x100000, ~0x210400] in this build). STATIC_ZONE
// reserves 4MB so new `static mut` peripherals never collide with the
// JS-owned regions — adding statics below the zone used to shift the
// linker's data layout and break boot (see AGENTS.md 4d40024 entry).
pub const CORES: u32 = 20;
pub const CORE_STATE_SIZE: u32 = 4096;
pub const SHA_DATA_SIZE: u32 = 128;
pub const SHA_DATA_OFFSET: u32 = CORE_STATE_SIZE * CORES;
pub const STATIC_ZONE: u32 = 0x400000;
pub const PAGE_TABLE_OFFSET: u32 = STATIC_ZONE;
pub const PAGE_TABLE_BYTE_SIZE: u32 = 8388608;
pub const REGION_TABLE_OFFSET: u32 = PAGE_TABLE_OFFSET + PAGE_TABLE_BYTE_SIZE;
pub const REGION_TABLE_SIZE: u32 = 256;
pub const RAM_DATA_OFFSET: u32 = REGION_TABLE_OFFSET + REGION_TABLE_SIZE;

#[repr(C)]
pub struct CoreState {
    pub esp32: u32,  // unused in WASM, stored for compatibility
    pub index: u32,
    pub name: u32,   // unused in WASM
    pub processor_id: u32,
    // Arrays
    pub physical_registers: [u32; 64],
    pub float_registers: [u32; 16],  // stored as bits
    pub special_registers: [u32; 256],
    pub user_registers: [u32; 237],
    // Q registers (PIE)
    pub q_registers: [u8; 128],
    pub qacc_high: [u8; 20],
    pub qacc_low: [u8; 20],
    // Individual fields
    pub accx_lo: u64,  // bigint low
    pub accx_hi: u64,  // bigint high
    pub sar_m32: i32,
    pub sar_m32_pending: u32,
    pub ua_state: [u32; 4],
    pub pie_enabled: u32,
    pub last_opcode: u32,
    pub enabled: u32,
    pub idle: u32,
    pub light_sleep: u32,
    pub pending_interrupts: u32,
    pub opcode_segment: u32,
    pub data_page_idx: i32,
    pub data_page_type: i32,
    pub data_page_data: i32,
    pub opcode_page_idx: i32,
    pub opcode_page_type: i32,
    pub opcode_page_data: i32,
    pub page_table_ptr: u32,   // unused in WASM (page table at fixed offset)
    pub mmio_handlers_ptr: u32, // unused in WASM
    pub mem_regions_ptr: u32,   // unused in WASM
    pub pc: u32,
    pub next_pc: u32,
    pub ccompare0_event: u32,  // CCOUNT value at which CCOMPARE0 fires (0 = not scheduled)
    pub ccompare1_event: u32,
    pub ccompare2_event: u32,
    pub debug_opcode: u32,  // last decoded opcode (debug)
    pub inst_count: u32,    // instruction counter for debug logging
    pub debug_log: u32,     // enable debug logging
    pub _pad: [u32; 26],  // padding to reach 4096 bytes
}

impl CoreState {
    pub unsafe fn from_index(idx: u32) -> &'static mut CoreState {
        let off = idx * CORE_STATE_SIZE;
        &mut *(off as *mut CoreState)
    }

    // byte/short/int access helpers
    pub fn sar_byte(&self) -> u32 {
        self.user_registers[13]
    }
    pub fn set_sar_byte(&mut self, val: u32) {
        self.user_registers[13] = val & 255;
    }
    pub fn fft_bit_width(&self) -> u32 {
        self.user_registers[14]
    }
    pub fn set_fft_bit_width(&mut self, val: u32) {
        self.user_registers[14] = val & 255;
    }

    // PS bitfield accessors
    pub fn ps_intlevel(&self) -> u32 {
        self.special_registers[INT_SET] & 15
    }
    pub fn set_ps_intlevel(&mut self, val: u32) {
        self.special_registers[INT_SET] =
            (self.special_registers[INT_SET] & 0xfffffff0) | (val & 15);
        self.update_interrupts();
    }
    pub fn ps_excm(&self) -> u32 {
        (self.special_registers[INT_SET] >> 4) & 1
    }
    pub fn set_ps_excm(&mut self, val: u32) {
        self.special_registers[INT_SET] =
            (self.special_registers[INT_SET] & 0xffffffef) | ((val & 1) << 4);
        self.pending_interrupts = 1;
    }
    pub fn ps_um(&self) -> u32 {
        (self.special_registers[INT_SET] >> 5) & 1
    }
    pub fn set_ps_um(&mut self, val: u32) {
        self.special_registers[INT_SET] =
            (self.special_registers[INT_SET] & 0xffffffdf) | ((val & 1) << 5);
    }
    pub fn ps_owing(&self) -> u32 {
        0
    }
    pub fn ps_owb(&self) -> u32 {
        (self.special_registers[INT_SET] >> 8) & 15
    }
    pub fn set_ps_owb(&mut self, val: u32) {
        self.special_registers[INT_SET] =
            (self.special_registers[INT_SET] & 0xfffff0ff) | ((val & 15) << 8);
    }
    pub fn ps_callinc(&self) -> u32 {
        (self.special_registers[INT_SET] >> 16) & 3
    }
    pub fn set_ps_callinc(&mut self, val: u32) {
        self.special_registers[INT_SET] =
            (self.special_registers[INT_SET] & 0xfffcffff) | ((val & 3) << 16);
    }
    pub fn ps_woe(&self) -> u32 {
        (self.special_registers[INT_SET] >> 18) & 1
    }
    pub fn set_ps_woe(&mut self, val: u32) {
        self.special_registers[INT_SET] =
            (self.special_registers[INT_SET] & 0xfffbffff) | ((val & 1) << 18);
    }
    pub fn acc(&self) -> u64 {
        ((self.special_registers[WINDOW_START] as u64 & 255) << 32)
            | self.special_registers[LBEG_REGISTER] as u64
    }
    pub fn set_acc(&mut self, val: u64) {
        self.special_registers[WINDOW_START] = ((val >> 32) & 255) as u32;
        self.special_registers[LBEG_REGISTER] = val as u32;
    }

    // AR register access (windowed)
    pub fn ar(&self, reg: u32) -> u32 {
        let base = self.special_registers[MEM_FAULT_INFO] << 2;
        self.physical_registers[((base + reg) % PHYSICAL_REG_COUNT) as usize]
    }
    pub fn set_ar(&mut self, reg: u32, val: u32) {
        let base = self.special_registers[MEM_FAULT_INFO] << 2;
        self.physical_registers[((base + reg) % PHYSICAL_REG_COUNT) as usize] = val;
    }

    // BR register access
    pub fn br(&self, reg: u32) -> bool {
        (self.special_registers[PS_REGISTER] & (1 << (reg & 15))) != 0
    }
    pub fn set_br(&mut self, reg: u32, val: bool) {
        let mask = 1 << reg;
        if val {
            self.special_registers[PS_REGISTER] |= mask;
        } else {
            self.special_registers[PS_REGISTER] &= !mask;
        }
    }

    // Vector
    pub fn vector(&self, offset: u32) -> u32 {
        self.special_registers[INT_LEVEL] + offset
    }

    // Window check (faithful translation)
    pub fn window_check(&mut self, cpu_val: u32, tmp_val: u32, idx_val: u32) -> bool {
        let clock_event = self.special_registers[MEM_FAULT_INFO];
        let simulation_clock = self.special_registers[CACHE_CONTROL];
        let read_result = cpu_val | tmp_val | idx_val;
        let write_addr = if read_result != 0 && (simulation_clock & (1 << ((1 + clock_event) & 15))) != 0 {
            1
        } else if (2 & read_result) != 0 && (simulation_clock & (1 << ((2 + clock_event) & 15))) != 0 {
            2
        } else if (3 == cpu_val || 3 == tmp_val || 3 == idx_val) && (simulation_clock & (1 << ((3 + clock_event) & 15))) != 0 {
            3
        } else {
            0
        };
        if self.ps_excm() != 0 || self.ps_woe() == 0 || write_addr == 0 {
            return false;
        }
        self.set_ps_owb(clock_event);
        self.set_ps_excm(1);
        let cpu_val = (clock_event + write_addr) & 15;
        self.special_registers[MISC_REGISTER] = self.pc;
        self.next_pc = self.vector(
            if (simulation_clock & (1 << ((cpu_val + 1) & 15))) != 0 {
                STACK_ALIGN_SHIFT
            } else if (simulation_clock & (1 << ((cpu_val + 2) & 15))) != 0 {
                REG_OFF_128
            } else {
                REG_OFF_256
            }
        );
        self.special_registers[MEM_FAULT_INFO] = cpu_val;
        true
    }

    // Exception (faithful translation)
    pub fn exception(&mut self, cause: u32) {
        // HW-exact PS save for the level-1 RFI restore (slot 184; see
        // take_interrupt/_handler3). Saved before EXCM is set.
        self.special_registers[184] = self.special_registers[PS_REGISTER];
        if self.ps_excm() != 0 {
            self.special_registers[EPS_REGISTER] = self.pc;
            self.next_pc = self.vector(REG_OFF_960);
        } else if self.ps_um() != 0 {
            self.special_registers[MISC_REGISTER] = self.pc;
            self.next_pc = self.vector(REG_OFF_832);
        } else {
            self.special_registers[MISC_REGISTER] = self.pc;
            self.next_pc = self.vector(REG_OFF_768);
        }
        self.special_registers[INT_STATUS] = cause;
        self.set_ps_excm(1);
    }

    // Take interrupt (faithful translation)
    pub fn take_interrupt(&mut self, level: u32) {
        // run178 (2026-09-15): the ack needs the vectoring core's &mut to
        // force-clear the cpu line (STATUS-clear alone is a no-op when STATUS
        // is already clear — the 1.1M-vector storm's root cause). Pass self
        // through so no from_index re-borrow is needed (run177j UB).
        if level == 4 && (self.special_registers[INT_ENABLE] & (1 << 25)) != 0 {
            crate::native_mmio::bt_ack_ll_vector(self);
        }
        // HW-exact PS save: silicon copies PS to EPS_level on vector and
        // RFI restores it. Without this, ISRs whose stubs rely on RFI for
        // PS restore (e.g. the level-4 BT dispatcher, unlike the level-1
        // stub which does an explicit wsr.ps) leave INTLEVEL stuck high and
        // wedge every level<=4 interrupt forever. Slots 184+idx-1 are free.
        self.special_registers[184 + level as usize - 1] = self.special_registers[PS_REGISTER];
        self.special_registers[MISC1_REGISTER + level as usize - 2] = self.pc;
        self.special_registers[DEPC_REGISTER + level as usize - 2] = self.special_registers[INT_SET];
        self.next_pc = self.vector(
            if 2 == level { REG_OFF_384 }
            else if 3 == level { REG_OFF_448 }
            else if 4 == level { REG_OFF_512 }
            else { REG_OFF_576 }
        );
        self.set_ps_intlevel(level);
        self.set_ps_excm(1);
    }

    // Update interrupts (faithful translation)
    // Q-register typed view helpers (1:1 with JS TypedArray views)
    pub fn q_reg_int16(&self, idx: usize) -> i16 {
        i16::from_le_bytes([self.q_registers[2 * idx], self.q_registers[2 * idx + 1]])
    }
    pub fn set_q_reg_int16(&mut self, idx: usize, val: i16) {
        let bytes = val.to_le_bytes();
        self.q_registers[2 * idx] = bytes[0];
        self.q_registers[2 * idx + 1] = bytes[1];
    }
    pub fn q_reg_uint16(&self, idx: usize) -> u16 {
        u16::from_le_bytes([self.q_registers[2 * idx], self.q_registers[2 * idx + 1]])
    }
    pub fn set_q_reg_uint16(&mut self, idx: usize, val: u16) {
        let bytes = val.to_le_bytes();
        self.q_registers[2 * idx] = bytes[0];
        self.q_registers[2 * idx + 1] = bytes[1];
    }
    pub fn q_reg_int32(&self, idx: usize) -> i32 {
        i32::from_le_bytes([
            self.q_registers[4 * idx],
            self.q_registers[4 * idx + 1],
            self.q_registers[4 * idx + 2],
            self.q_registers[4 * idx + 3],
        ])
    }
    pub fn set_q_reg_int32(&mut self, idx: usize, val: i32) {
        let bytes = val.to_le_bytes();
        self.q_registers[4 * idx] = bytes[0];
        self.q_registers[4 * idx + 1] = bytes[1];
        self.q_registers[4 * idx + 2] = bytes[2];
        self.q_registers[4 * idx + 3] = bytes[3];
    }
    pub fn q_reg_uint32(&self, idx: usize) -> u32 {
        u32::from_le_bytes([
            self.q_registers[4 * idx],
            self.q_registers[4 * idx + 1],
            self.q_registers[4 * idx + 2],
            self.q_registers[4 * idx + 3],
        ])
    }
    pub fn set_q_reg_uint32(&mut self, idx: usize, val: u32) {
        let bytes = val.to_le_bytes();
        self.q_registers[4 * idx] = bytes[0];
        self.q_registers[4 * idx + 1] = bytes[1];
        self.q_registers[4 * idx + 2] = bytes[2];
        self.q_registers[4 * idx + 3] = bytes[3];
    }

    // accx (40-bit signed BigInt in JS, stored as two u64)
    pub fn get_accx(&self) -> i128 {
        (self.accx_hi as i128) << 64 | self.accx_lo as i128
    }
    pub fn set_accx(&mut self, val: i128) {
        self.accx_lo = val as u64;
        self.accx_hi = (val >> 64) as u64;
    }

    pub fn update_interrupts(&mut self) {
        let cpu_val = if self.ps_intlevel() < 6 { self.ps_intlevel() } else { 6 };
        let cpu_val = if self.ps_excm() != 0 {
            if cpu_val > 3 { cpu_val } else { 3 }
        } else {
            cpu_val
        };
        let tmp_val: u32 = 16384;
        let idx_val = self.special_registers[CLOCK_CONFIG] | tmp_val;
        let clock_event = self.special_registers[INT_ENABLE] & idx_val;
        let mut level = 7i32;
        while level > cpu_val as i32 {
            let mask = match level {
                1 => 407551u32,
                2 => 3670016u32,
                3 => 0x28c08800,
                4 => 0x53000000,
                5 => 0x84010000,
                7 => 16384,
                _ => 0,
            };
            if (clock_event & mask) != 0 {
                if self.light_sleep == 0 {
                    self.idle = 0;
                }
                if level >= 2 {
                    self.take_interrupt(level as u32);
                } else {
                    self.exception(TRAP_LEVEL1_INTERRUPT);
                }
                self.pc = self.next_pc;
                self.pending_interrupts = 1;
                return;
            }
            level -= 1;
        }
        self.pending_interrupts = 0;
    }

    // Reset (faithful translation)
    pub fn reset(&mut self) {
        self.pending_interrupts = 0;
        self.enabled = 1;
        self.idle = 0;
        self.light_sleep = 0;
        self.physical_registers.fill(0);
        self.float_registers.fill(0);
        self.special_registers.fill(0);
        self.user_registers.fill(0);
        if self.pie_enabled != 0 {
            self.q_registers.fill(0);
            self.qacc_high.fill(0);
            self.qacc_low.fill(0);
            self.accx_lo = 0;
            self.accx_hi = 0;
            self.ua_state.fill(0);
            self.set_sar_byte(0);
            self.set_fft_bit_width(0);
        }
        self.pc = 0x40000400;
        self.set_ps_woe(1);
        self.set_ar(1, 0x3fffffff);
        self.special_registers[CCOMPARE_REG] = self.processor_id;
        self.special_registers[ATOM_CTRL] = 1;
        self.special_registers[INT_LEVEL] = 0x40000000;
        self.special_registers[CCOUNT_REG] = 0x5360730;
        self.next_pc = self.pc;
        self.opcode_segment = 0xFFFFFFFF;
        self.opcode_page_idx = -1;
        self.data_page_idx = -1;
    }

    // Get CCOUNT from cached value (synced once per step)
    pub fn ccount(&self) -> u32 {
        self.special_registers[CCOUNT_REG]
    }
    // Sync CCOUNT from SAB (written by JS after each clock.tick)
    pub fn sync_ccount(&mut self) {
        self.special_registers[CCOUNT_REG] = sab_ticks();
    }

    // unimplemented (faithful translation from JS cpuVal.unimplemented)
    pub fn unimplemented(&self, _tmp_val: u32) {
        // In JS this logs a message. In WASM this is a no-op.
    }

    // intSetClear (faithful translation)
    pub fn int_set_clear(&mut self, set_bits: u32, clear_bits: u32) {
        let idx_val = (self.special_registers[INT_ENABLE] & !clear_bits) | set_bits;
        self.special_registers[INT_ENABLE] = idx_val;
        self.pending_interrupts = if idx_val != 0 { 1 } else { 0 };
    }

    // isWindowInstruction (faithful translation)
    // Note: this calls readUint8/readUint16 which needs memory access
    // This function is rarely called (only during window exception handling in cHandler7)
    // We'll call back to JS for this
}
