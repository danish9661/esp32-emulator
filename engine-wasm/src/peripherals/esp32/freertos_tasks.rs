// Translated from: src/peripherals/esp32/freertos-tasks.js
// Do NOT modify the logic -- match the JS line-for-line
// Verified 2026-06-26: all methods, constants, and logic match JS

use crate::peripherals::types::*;

// Xtensa register constants (from xtensa-constants.js)
const LOOP_BEGIN: u32 = 0;
const LOOP_END: u32 = 1;
const LOOP_COUNT: u32 = 2;
const EXC_CAUSE: u32 = 3;
const INT_SET: u32 = 230;
const INT_STATUS: u32 = 232;
const MISC_CONFIG: u32 = 238;
const REGISTER_TYPE_PC: u32 = 0;
const REGISTER_TYPE_AR: u32 = 0x1000000;
const REGISTER_TYPE_SPECIAL: u32 = 0x2000000;
const REGISTER_TYPE_USER: u32 = 0x3000000;
const REGISTER_TYPE_FP: u32 = 0x4000000;

// Precomputed register map keys matching JS regMap computed property keys
const REG_KEY_PC: u32 = REGISTER_TYPE_PC;                                      // 0
const REG_KEY_INT_SET: u32 = REGISTER_TYPE_SPECIAL | INT_SET;                  // 0x20000E6
const REG_KEY_AR0: u32 = REGISTER_TYPE_AR | 0;                                 // 0x1000000
const REG_KEY_AR1: u32 = REGISTER_TYPE_AR | 1;                                 // 0x1000001
const REG_KEY_AR2: u32 = REGISTER_TYPE_AR | 2;                                 // 0x1000002
const REG_KEY_AR3: u32 = REGISTER_TYPE_AR | 3;                                 // 0x1000003
const REG_KEY_AR4: u32 = REGISTER_TYPE_AR | 4;                                 // 0x1000004
const REG_KEY_AR5: u32 = REGISTER_TYPE_AR | 5;                                 // 0x1000005
const REG_KEY_AR6: u32 = REGISTER_TYPE_AR | 6;                                 // 0x1000006
const REG_KEY_AR7: u32 = REGISTER_TYPE_AR | 7;                                 // 0x1000007
const REG_KEY_AR8: u32 = REGISTER_TYPE_AR | 8;                                 // 0x1000008
const REG_KEY_AR9: u32 = REGISTER_TYPE_AR | 9;                                 // 0x1000009
const REG_KEY_AR10: u32 = REGISTER_TYPE_AR | 10;                               // 0x100000A
const REG_KEY_AR11: u32 = REGISTER_TYPE_AR | 11;                               // 0x100000B
const REG_KEY_AR12: u32 = REGISTER_TYPE_AR | 12;                               // 0x100000C
const REG_KEY_AR13: u32 = REGISTER_TYPE_AR | 13;                               // 0x100000D
const REG_KEY_AR14: u32 = REGISTER_TYPE_AR | 14;                               // 0x100000E
const REG_KEY_AR15: u32 = REGISTER_TYPE_AR | 15;                               // 0x100000F
const REG_KEY_EXC_CAUSE: u32 = REGISTER_TYPE_SPECIAL | EXC_CAUSE;              // 0x2000003
const REG_KEY_INT_STATUS: u32 = REGISTER_TYPE_SPECIAL | INT_STATUS;            // 0x20000E8
const REG_KEY_MISC_CONFIG: u32 = REGISTER_TYPE_SPECIAL | MISC_CONFIG;          // 0x20000EE
const REG_KEY_LOOP_BEGIN: u32 = REGISTER_TYPE_SPECIAL | LOOP_BEGIN;            // 0x2000000
const REG_KEY_LOOP_END: u32 = REGISTER_TYPE_SPECIAL | LOOP_END;                // 0x2000001
const REG_KEY_LOOP_COUNT: u32 = REGISTER_TYPE_SPECIAL | LOOP_COUNT;            // 0x2000002

pub const MAX_THREADS: usize = 64;
pub const MAX_NAME_LEN: usize = 64;
pub const MAX_LIST_NAME_LEN: usize = 32;

pub const MAX_VERSION_LEN: usize = 16;

pub struct FreeRtOsTaskScheduler {
    pub openocd_params: u32,
    pub openocd_params_size: u32,
    pub px_ready_tasks_lists: u32,
    pub x_delayed_task_list1: u32,
    pub x_delayed_task_list2: u32,
    pub x_pending_ready_list: u32,
    pub x_tasks_waiting_termination: u32,
    pub x_suspended_task_list: u32,
    pub ux_current_number_of_tasks: u32,
    pub ux_top_used_priority: u32,
    pub x_scheduler_running: u32,
    pub ux_task_number: u32,
    pub px_current_tcbs: u32,
    pub px_current_tcbs_size: u32,
    rtos_version_buf: [u8; MAX_VERSION_LEN],
    rtos_version_len: u8,
    rtos_version_valid: bool,
}

impl FreeRtOsTaskScheduler {
    pub fn new() -> Self {
        FreeRtOsTaskScheduler {
            openocd_params: 0,
            openocd_params_size: 0,
            px_ready_tasks_lists: 0,
            x_delayed_task_list1: 0,
            x_delayed_task_list2: 0,
            x_pending_ready_list: 0,
            x_tasks_waiting_termination: 0,
            x_suspended_task_list: 0,
            ux_current_number_of_tasks: 0,
            ux_top_used_priority: 0,
            x_scheduler_running: 0,
            ux_task_number: 0,
            px_current_tcbs: 0,
            px_current_tcbs_size: 0,
            rtos_version_buf: [0u8; MAX_VERSION_LEN],
            rtos_version_len: 0,
            rtos_version_valid: false,
        }
    }

    pub fn init(&mut self, openocd: u32, openocd_size: u32, ready_lists: u32, delayed1: u32, delayed2: u32, pending_ready: u32, waiting_term: u32, suspended: u32, num_tasks: u32, top_priority: u32, sched_running: u32, task_num: u32, current_tcbs: u32, current_tcbs_size: u32) {
        self.openocd_params = openocd;
        self.openocd_params_size = openocd_size;
        self.px_ready_tasks_lists = ready_lists;
        self.x_delayed_task_list1 = delayed1;
        self.x_delayed_task_list2 = delayed2;
        self.x_pending_ready_list = pending_ready;
        self.x_tasks_waiting_termination = waiting_term;
        self.x_suspended_task_list = suspended;
        self.ux_current_number_of_tasks = num_tasks;
        self.ux_top_used_priority = top_priority;
        self.x_scheduler_running = sched_running;
        self.ux_task_number = task_num;
        self.px_current_tcbs = current_tcbs;
        self.px_current_tcbs_size = current_tcbs_size;
    }

    fn mem_read_u32(&self, ctx: &mut CpuContext, addr: u32) -> u32 {
        ctx.mem_read_u32(addr)
    }

    fn mem_read_u8(&self, ctx: &mut CpuContext, addr: u32) -> u8 {
        ctx.mem_read_u8(addr)
    }

    fn read_u32(&self, ctx: &mut CpuContext, addr: u32, offset: u32) -> u32 {
        ctx.mem_read_u32(addr + 4 * offset)
    }

    fn read_string(&self, ctx: &mut CpuContext, addr: u32, max_len: u32) -> ([u8; MAX_NAME_LEN], u32) {
        let mut chars = [0u8; MAX_NAME_LEN];
        let mut len = 0;
        for i in 0..max_len {
            let byte = ctx.mem_read_u8(addr + i);
            if byte == 0 { break; }
            if (i as usize) < MAX_NAME_LEN {
                chars[i as usize] = byte;
            }
            len = i + 1;
        }
        (chars, len)
    }

    pub fn rtos_params(&self, ctx: &mut CpuContext) -> Option<RtosParams> {
        if self.openocd_params == 0 { return None; }
        Some(RtosParams {
            size: self.mem_read_u8(ctx, self.openocd_params + 0),
            version: self.mem_read_u8(ctx, self.openocd_params + 1),
            kernel_major: self.mem_read_u8(ctx, self.openocd_params + 2),
            kernel_minor: self.mem_read_u8(ctx, self.openocd_params + 3),
            kernel_build: self.mem_read_u8(ctx, self.openocd_params + 4),
            top_used_priority: self.mem_read_u8(ctx, self.openocd_params + 5),
            top_of_stack: self.mem_read_u8(ctx, self.openocd_params + 6),
            task_name_offset: self.mem_read_u8(ctx, self.openocd_params + 7),
        })
    }

    pub fn rtos_version(&mut self, ctx: &mut CpuContext) -> Option<&str> {
        if !self.rtos_version_valid {
            let params = self.rtos_params(ctx)?;
            let mut pos = 0usize;
            pos = self.write_u32_digits(params.kernel_major as u32, pos);
            if pos < MAX_VERSION_LEN { self.rtos_version_buf[pos] = b'.'; pos += 1; }
            pos = self.write_u32_digits(params.kernel_minor as u32, pos);
            if pos < MAX_VERSION_LEN { self.rtos_version_buf[pos] = b'.'; pos += 1; }
            pos = self.write_u32_digits(params.kernel_build as u32, pos);
            self.rtos_version_len = pos.min(MAX_VERSION_LEN - 1) as u8;
            self.rtos_version_valid = true;
        }
        if self.rtos_version_len == 0 { return None; }
        Some(core::str::from_utf8(&self.rtos_version_buf[..self.rtos_version_len as usize]).unwrap_or(""))
    }

    fn write_u32_digits(&mut self, val: u32, start: usize) -> usize {
        if start >= MAX_VERSION_LEN { return start; }
        if val >= 100 {
            self.rtos_version_buf[start] = b'0' + (val / 100) as u8;
            if start + 1 < MAX_VERSION_LEN {
                self.rtos_version_buf[start + 1] = b'0' + ((val / 10) % 10) as u8;
                if start + 2 < MAX_VERSION_LEN {
                    self.rtos_version_buf[start + 2] = b'0' + (val % 10) as u8;
                    return start + 3;
                }
            }
            start + 2
        } else if val >= 10 {
            self.rtos_version_buf[start] = b'0' + (val / 10) as u8;
            if start + 1 < MAX_VERSION_LEN {
                self.rtos_version_buf[start + 1] = b'0' + (val % 10) as u8;
                return start + 2;
            }
            start + 1
        } else {
            self.rtos_version_buf[start] = b'0' + val as u8;
            start + 1
        }
    }

    pub fn rtos_ready(&self, ctx: &mut CpuContext) -> bool {
        if self.x_scheduler_running == 0 { return false; }
        if 1 != self.read_u32(ctx, self.x_scheduler_running, 0) { return false; }
        if self.openocd_params_size != 8 { return false; }
        true
    }

    pub fn current_tcbs(&self, ctx: &mut CpuContext, out: &mut [u32]) -> u32 {
        if self.px_current_tcbs == 0 { return 0; }
        let max_count = self.px_current_tcbs_size / 4;
        let count = if max_count < 32 { max_count } else { 32 };
        for i in 0..count as usize {
            if i < out.len() {
                out[i] = self.read_u32(ctx, self.px_current_tcbs, i as u32);
            }
        }
        count
    }

    pub fn threads(&self, ctx: &mut CpuContext, out: &mut ThreadList) -> u32 {
        let e = match self.rtos_params(ctx) {
            Some(p) => p,
            None => return 0,
        };
        let t = self.current_tcbs(ctx, &mut out.tcb_buf);
        if t == 0 { return 0; }
        let i = self.ux_current_number_of_tasks;
        if i == 0 { return 0; }
        let task_num_addr = self.ux_task_number;
        if task_num_addr == 0 { return 0; }
        let top_used_addr = self.ux_top_used_priority;
        if top_used_addr == 0 { return 0; }
        let r = self.px_ready_tasks_lists;
        if r == 0 { return 0; }

        let a = self.read_u32(ctx, i, 0);
        self.read_u32(ctx, task_num_addr, 0);
        let list_size = self.read_u32(ctx, top_used_addr, 0);

        if !self.rtos_ready(ctx) { return 0; }
        if a == 0 { return 0; }
        if list_size == 0 { return 0; }

        let mut list_entries: [[u8; MAX_LIST_NAME_LEN]; 48] = [[0u8; MAX_LIST_NAME_LEN]; 48];
        let mut list_addrs: [u32; 48] = [0u32; 48];
        let mut list_count: u32 = 0;

        // delayed1
        if self.x_delayed_task_list1 != 0 {
            let mut name_buf = [0u8; MAX_LIST_NAME_LEN];
            let name_bytes = b"delayed1";
            let mut j = 0;
            while j < name_bytes.len() && j < MAX_LIST_NAME_LEN {
                name_buf[j] = name_bytes[j];
                j += 1;
            }
            list_entries[list_count as usize] = name_buf;
            list_addrs[list_count as usize] = self.x_delayed_task_list1;
            list_count += 1;
        }
        // delayed2
        if self.x_delayed_task_list2 != 0 {
            let mut name_buf = [0u8; MAX_LIST_NAME_LEN];
            let name_bytes = b"delayed2";
            let mut j = 0;
            while j < name_bytes.len() && j < MAX_LIST_NAME_LEN {
                name_buf[j] = name_bytes[j];
                j += 1;
            }
            list_entries[list_count as usize] = name_buf;
            list_addrs[list_count as usize] = self.x_delayed_task_list2;
            list_count += 1;
        }
        // pendingReady
        if self.x_pending_ready_list != 0 {
            let mut name_buf = [0u8; MAX_LIST_NAME_LEN];
            let name_bytes = b"pendingReady";
            let mut j = 0;
            while j < name_bytes.len() && j < MAX_LIST_NAME_LEN {
                name_buf[j] = name_bytes[j];
                j += 1;
            }
            list_entries[list_count as usize] = name_buf;
            list_addrs[list_count as usize] = self.x_pending_ready_list;
            list_count += 1;
        }
        // suspended
        if self.x_suspended_task_list != 0 {
            let mut name_buf = [0u8; MAX_LIST_NAME_LEN];
            let name_bytes = b"suspended";
            let mut j = 0;
            while j < name_bytes.len() && j < MAX_LIST_NAME_LEN {
                name_buf[j] = name_bytes[j];
                j += 1;
            }
            list_entries[list_count as usize] = name_buf;
            list_addrs[list_count as usize] = self.x_suspended_task_list;
            list_count += 1;
        }
        // waitingTermination
        if self.x_tasks_waiting_termination != 0 {
            let mut name_buf = [0u8; MAX_LIST_NAME_LEN];
            let name_bytes = b"waitingTermination";
            let mut j = 0;
            while j < name_bytes.len() && j < MAX_LIST_NAME_LEN {
                name_buf[j] = name_bytes[j];
                j += 1;
            }
            list_entries[list_count as usize] = name_buf;
            list_addrs[list_count as usize] = self.x_tasks_waiting_termination;
            list_count += 1;
        }

        let peripheral_type = list_size + 1;
        let pin_state: u32 = 20;
        let signal_direction: u32 = 8;
        let interrupt_trigger: u32 = 16;
        let i2c_command: u32 = 12;
        let i2c_interrupt_type: u32 = 8;

        for pri in 0..peripheral_type {
            let addr = r + pri * pin_state;
            if addr != 0 {
                let mut name_buf = [0u8; MAX_LIST_NAME_LEN];
                let prefix = b"priority";
                let mut j = 0;
                while j < prefix.len() && j < MAX_LIST_NAME_LEN {
                    name_buf[j] = prefix[j];
                    j += 1;
                }
                if j < MAX_LIST_NAME_LEN {
                    let d = pri;
                    if d >= 100 {
                        name_buf[j] = b'0' + (d / 100) as u8;
                        j += 1;
                    }
                    if d >= 10 {
                        name_buf[j] = b'0' + ((d / 10) % 10) as u8;
                        j += 1;
                    }
                    if j < MAX_LIST_NAME_LEN {
                        name_buf[j] = b'0' + (d % 10) as u8;
                        j += 1;
                    }
                }
                if (list_count as usize) < 48 {
                    list_entries[list_count as usize] = name_buf;
                    list_addrs[list_count as usize] = addr;
                    list_count += 1;
                }
            }
        }

        let mut thread_count: u32 = 0;

        for idx in 0..list_count as usize {
            let tcb_addr = list_addrs[idx];
            if tcb_addr == 0 { continue; }
            let list_len = self.mem_read_u32(ctx, tcb_addr);
            let mut r_next = self.mem_read_u32(ctx, tcb_addr + interrupt_trigger);
            let a_end = self.mem_read_u32(ctx, tcb_addr + signal_direction);
            let mut entry_count = 0u32;

            while r_next != 0 && r_next != a_end && entry_count < list_len && (thread_count as usize) < MAX_THREADS {
                let task_tcb_addr = self.mem_read_u32(ctx, r_next + i2c_command);

                let (task_name, task_name_len) = self.read_string(ctx, task_tcb_addr + e.task_name_offset as u32, 64);

                out.threads[thread_count as usize].tcb_handle = task_tcb_addr;
                out.threads[thread_count as usize].name = task_name;
                out.threads[thread_count as usize].name_len = task_name_len;
                out.threads[thread_count as usize].list = list_entries[idx];
                out.threads[thread_count as usize].sp = self.mem_read_u32(ctx, task_tcb_addr + e.top_of_stack as u32);

                let core_idx = find_tcb_index(&out.tcb_buf, t, task_tcb_addr);
                out.threads[thread_count as usize].core = core_idx;

                thread_count += 1;
                entry_count += 1;
                r_next = self.mem_read_u32(ctx, r_next + i2c_interrupt_type);
            }
        }

        // Sort: tasks with core first, then by tcbHandle ascending
        for i in 0..thread_count as usize {
            for j in i + 1..thread_count as usize {
                let ei = &out.threads[i];
                let ej = &out.threads[j];
                let cmp = if ei.core >= 0 && ej.core < 0 {
                    -1i32
                } else if ei.core < 0 && ej.core >= 0 {
                    1i32
                } else if ei.tcb_handle < ej.tcb_handle {
                    -1i32
                } else if ei.tcb_handle > ej.tcb_handle {
                    1i32
                } else {
                    0i32
                };
                if cmp > 0 {
                    out.threads.swap(i, j);
                }
            }
        }

        thread_count
    }

    pub fn read_register(&self, ctx: &mut CpuContext, addr: u32, reg: u32) -> u32 {
        let e = match self.rtos_params(ctx) {
            Some(p) => p,
            None => return 0,
        };
        let mut tcb_buf = [0u32; 32];
        let tcb_list = self.current_tcbs(ctx, &mut tcb_buf);
        if tcb_list == 0 { return 0; }

        let core_idx = find_tcb_index(&tcb_buf, tcb_list, addr);
        if core_idx >= 0 {
            return ctx.gdb_read_register(core_idx as u32, reg);
        }
        let a = self.mem_read_u32(ctx, addr + e.top_of_stack as u32);
        if ctx.is_xtensa_core(0) {
            return self.read_xtensa_register(ctx, reg, a);
        }
        match reg {
            0 => 0,
            32 => self.mem_read_u32(ctx, a),
            _ => self.mem_read_u32(ctx, a + 4 * reg),
        }
    }

    pub fn read_xtensa_register(&self, ctx: &mut CpuContext, reg: u32, stack_ptr: u32) -> u32 {
        let gdb_reg_type = ctx.gdb_register_type(0, reg);
        self.mem_read_u32(ctx, stack_ptr);
        let a = match gdb_reg_type {
            x if x == REG_KEY_PC => 4,
            x if x == REG_KEY_INT_SET => 8,
            x if x == REG_KEY_AR0 => 12,
            x if x == REG_KEY_AR1 => 16,
            x if x == REG_KEY_AR2 => 20,
            x if x == REG_KEY_AR3 => 24,
            x if x == REG_KEY_AR4 => 28,
            x if x == REG_KEY_AR5 => 32,
            x if x == REG_KEY_AR6 => 36,
            x if x == REG_KEY_AR7 => 40,
            x if x == REG_KEY_AR8 => 44,
            x if x == REG_KEY_AR9 => 48,
            x if x == REG_KEY_AR10 => 52,
            x if x == REG_KEY_AR11 => 56,
            x if x == REG_KEY_AR12 => 60,
            x if x == REG_KEY_AR13 => 64,
            x if x == REG_KEY_AR14 => 68,
            x if x == REG_KEY_AR15 => 72,
            x if x == REG_KEY_EXC_CAUSE => 76,
            x if x == REG_KEY_INT_STATUS => 80,
            x if x == REG_KEY_MISC_CONFIG => 84,
            x if x == REG_KEY_LOOP_BEGIN => 88,
            x if x == REG_KEY_LOOP_END => 92,
            x if x == REG_KEY_LOOP_COUNT => 96,
            _ => 0,
        };
        if a == 0 { return 0; }
        let mut val = self.mem_read_u32(ctx, stack_ptr + a);
        if gdb_reg_type == REG_KEY_INT_SET {
            val &= !16;
        }
        val
    }
}

fn find_tcb_index(tcb_list: &[u32], count: u32, addr: u32) -> i32 {
    for i in 0..count as usize {
        if i < tcb_list.len() && tcb_list[i] == addr {
            return i as i32;
        }
    }
    -1
}

pub struct RtosParams {
    pub size: u8,
    pub version: u8,
    pub kernel_major: u8,
    pub kernel_minor: u8,
    pub kernel_build: u8,
    pub top_used_priority: u8,
    pub top_of_stack: u8,
    pub task_name_offset: u8,
}

pub struct ThreadInfo {
    pub tcb_handle: u32,
    pub name: [u8; MAX_NAME_LEN],
    pub name_len: u32,
    pub list: [u8; MAX_LIST_NAME_LEN],
    pub sp: u32,
    pub core: i32,
}

pub struct ThreadList {
    pub threads: [ThreadInfo; MAX_THREADS],
    pub tcb_buf: [u32; 32],
}
