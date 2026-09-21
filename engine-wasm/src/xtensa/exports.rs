use super::constants::*;
use super::state::{CoreState, sab_ticks, sab_cycles};
use super::memory::*;
use super::handlers::c_handler7;
use super::vecinst::{decode_pie0, decode_pie31};

extern "C" {
    fn mmio_read(handler_id: u32, addr: u32, size: u32) -> u32;
    fn mmio_write(handler_id: u32, addr: u32, val: u32, size: u32);
    fn on_unknown_inst(core_idx: u32, pc: u32, opcode: u32);
    pub fn on_break(core_idx: u32);
    fn write_watchpoint(addr: u32, core_idx: u32) -> u32;
    fn trace_mem_write(core_idx: u32, pc: u32, addr: u32, val: u32, size: u32);
    fn trace_entry(core_idx: u32, pc: u32, arg: u32);
    pub     fn trace_return(core_idx: u32, pc: u32, ret_val: u32, ret_addr: u32);
    fn is_window_inst(core_ptr: u32) -> u32;
    fn map_address(core_idx: u32, addr: u32) -> u32;
    fn has_breakpoint(core_idx: u32, pc: u32) -> u32;
    fn js_log_u32(val: u32);
    fn js_log_str(ptr: u32, len: u32);
    fn get_sim_freq() -> u32;
    fn get_cpu_freq() -> u32;
    fn ccompare_schedule(core_idx: u32, which: u32, value: u32);
    fn ccompare_unschedule(core_idx: u32, which: u32);
    fn efuse_cmd_schedule(nanos: u32);
}

// Public wrappers for JS logging (used by sha_peripheral.rs)
pub static mut PC_TRACE_LEFT: u32 = 0;
pub static mut PC_TRACE_CORE: u32 = 2;
// run152 bisect gate (RETIRED run153 with the run151 intercept; the gate
// and shadow now stand by for future CAS forensics, default OFF).
pub static mut S32C1I_ENABLE: u32 = 0;
// SCOMPARE1 shadow register (RETIRED run153 — SAR-as-shadow is coherent).
pub static mut SCOMPARE1_SHADOW: u32 = 0;
pub static mut GARBAGE_PC_LOGGED: u32 = 0;
pub static mut RING_IDX: u32 = 0;
pub static mut RING: [[u32; 6]; 512] = [[0; 6]; 512];
pub static mut TRIGGERED: u32 = 0;
pub static mut WR_IDX: u32 = 0;
pub static mut WR_RING: [[u32; 3]; 512] = [[0; 3]; 512];
pub static mut DBG_BUF: [u8; 26000] = [0; 26000];
pub static mut LAST_PC: u32 = 0;
pub static mut SPIN_LOGGED: u32 = 0;
pub static mut LOOP_LOGGED: u32 = 0;
pub static mut LOOP979F_DONE: u32 = 0;
pub static mut LOOP979F_N: u32 = 0;
pub static mut FREEZE_LOGGED: u32 = 0;
pub static mut DECODE_TRACE: u32 = 0;
pub static mut DECODE_TRACE_LEFT: u32 = 0;
pub static mut DEFER_PC: u32 = 0;

pub fn ring_write(pc: u32, addr: u32, val: u32) {
    unsafe {
        let wi = (WR_IDX % 512) as usize;
        WR_RING[wi][0] = pc;
        WR_RING[wi][1] = addr;
        WR_RING[wi][2] = val;
        WR_IDX += 1;
    }
}

pub static mut RD_LOG_LEFT: u32 = 4000;

pub fn ring_read(pc: u32, addr: u32, val: u32) {
    unsafe {
        if RD_LOG_LEFT == 0 {
            return;
        }
        RD_LOG_LEFT -= 1;
        let mut db = [0u8; 88];
        let hx = |mut v: u32| -> [u8; 8] {
            let mut o = [0u8; 8];
            for i in (0..8).rev() {
                o[i] = b"0123456789abcdef"[(v & 0xF) as usize];
                v >>= 4;
            }
            o
        };
        let mut n = 0;
        for &b in b"[RD] pc=" { db[n] = b; n += 1; }
        for &b in &hx(pc) { db[n] = b; n += 1; }
        for &b in b" a=" { db[n] = b; n += 1; }
        for &b in &hx(addr) { db[n] = b; n += 1; }
        for &b in b" v=" { db[n] = b; n += 1; }
        for &b in &hx(val) { db[n] = b; n += 1; }
        crate::js_log_str(db.as_ptr() as u32, n as u32);
    }
}

// Trace FFI gating — the JS ESPTrace sink only receives events when a host
// debugger enables tracing (native_trace_set_flags); default off, so the
// per-memory-write / per-return FFI round-trips are eliminated.
pub static mut TRACE_RETURNS: bool = false;
pub static mut TRACE_MEM_WRITES: bool = false;

#[no_mangle]
pub extern "C" fn native_trace_set_flags(returns: u32, mem_writes: u32) {
    unsafe {
        TRACE_RETURNS = returns != 0;
        TRACE_MEM_WRITES = mem_writes != 0;
    }
}

/// Enable the PC-range decode/execute trace for `left` instructions (debug
/// only). It is OFF by default so normal runs are not flooded when the PC
/// enters the WiFi/BT firmware ranges. Pass 0 to disable.
#[no_mangle]
pub extern "C" fn native_set_decode_trace(left: u32) {
    unsafe {
        DECODE_TRACE_LEFT = left;
    }
}

pub fn trace_return_active() -> bool {
    unsafe { TRACE_RETURNS }
}

pub fn trace_mem_write_active() -> bool {
    unsafe { TRACE_MEM_WRITES }
}
// run266: reset ALL per-boot one-shot diag budgets. Called from
// native_bt_rf_reset() (which runs on every chip.reset()). Function-static
// counters in run_instruction + native_mmio shim legs survive WASM-instance
// reuse across boots in the same worker process; without reset, boot 2+
// logs NOTHING and looks hung even when healthy.
#[no_mangle]
pub extern "C" fn reset_boot_diag_pub() {
    reset_boot_diag();
}

pub fn reset_boot_diag() {
    unsafe {
        SPIN_LOGGED = 0;
        LOOP_LOGGED = 0;
        LOOP979F_DONE = 0;
        LOOP979F_N = 0;
        FREEZE_LOGGED = 0;
    }
    crate::native_mmio::reset_shim_diag();
}

pub fn js_log(val: u32) {
    unsafe { js_log_u32(val); }
}
pub fn js_log_msg(ptr: u32, len: u32) {
    unsafe { js_log_str(ptr, len); }
}

// Ungated trace of LL VHCI ISR DRAM accesses (set via BT_VHCI_TRACE_LEFT in
// native_mmio; consumed in memory.rs read/write hooks). Used to find which
// address the LL recv reads HCI from.
pub fn bt_vhci_log(pc: u32, addr: u32, val: u32, is_write: u32) {
    unsafe {
        let mut db = [0u8; 80];
        let hx = |mut v: u32| -> [u8; 8] {
            let mut o = [0u8; 8];
            for i in (0..8).rev() { o[i] = b"0123456789abcdef"[(v & 0xF) as usize]; v >>= 4; }
            o
        };
        let mut n = 0;
        for &b in b"[VHCI] " { db[n] = b; n += 1; }
        for &b in if is_write != 0 { b"w " } else { b"r " } { db[n] = b; n += 1; }
        for &b in &hx(pc) { db[n] = b; n += 1; }
        for &b in b" a=" { db[n] = b; n += 1; }
        for &b in &hx(addr) { db[n] = b; n += 1; }
        for &b in b" v=" { db[n] = b; n += 1; }
        for &b in &hx(val) { db[n] = b; n += 1; }
        crate::js_log_str(db.as_ptr() as u32, n as u32);
    }
}

fn log_msg(msg: &str) {
    unsafe { js_log_str(msg.as_ptr() as u32, msg.len() as u32) }
}

fn get_core(idx: u32) -> &'static mut CoreState {
    unsafe { CoreState::from_index(idx) }
}

#[no_mangle]
pub extern "C" fn native_pc_trace(n: u32) {
    unsafe { PC_TRACE_LEFT = n; PC_TRACE_CORE = 2; }
}

// Which core(s) the PC trace covers: 0/1 = that core only, 2 = both.
// Set alongside PC_TRACE_LEFT (default both; the btc_init entry arm narrows
// to c1). JS CMD_PCTRACE passes count only -> traces both cores.

#[no_mangle]
pub extern "C" fn native_get_pt_offset() -> u32 {
    super::state::PAGE_TABLE_OFFSET
}

#[no_mangle]
pub extern "C" fn native_print_pt_entry(page_idx: u32) {
    unsafe {
        let off = (super::state::PAGE_TABLE_OFFSET + page_idx * 8) as usize;
        let typ = *(off as *const u32);
        let data = *((off + 4) as *const u32);
        js_log_u32(typ);
        js_log_u32(data);
    }
}

#[no_mangle]
pub extern "C" fn native_peripheral_init() -> u32 {
    crate::native_mmio::init()
}

#[no_mangle]
pub extern "C" fn native_gpio_init() {
    crate::native_mmio::init_gpio();
}

#[no_mangle]
pub extern "C" fn native_gpio_reset() {
    crate::native_mmio::native_gpio_reset();
}

#[no_mangle]
pub extern "C" fn native_gpio_set_pin_input(pin: u32, level: u32) {
    crate::native_mmio::native_gpio_set_pin_input(pin, level);
}

#[no_mangle]
pub extern "C" fn native_gpio_seed() {
    crate::native_mmio::native_gpio_seed();
}

#[no_mangle]
pub extern "C" fn native_gpio_seed_scratch() -> u32 {
    crate::native_mmio::native_gpio_seed_scratch()
}

#[no_mangle]
pub extern "C" fn native_gpio_set_strap(val: u32) {
    crate::native_mmio::native_gpio_set_strap(val);
}

#[no_mangle]
pub extern "C" fn native_frc_timer_reset() {
    crate::native_mmio::native_frc_timer_reset();
}

#[no_mangle]
pub extern "C" fn native_aes_init() {
    crate::native_mmio::init_aes();
}

#[no_mangle]
pub extern "C" fn native_frc_timer_init() {
    crate::native_mmio::init_frc_timer();
}

#[no_mangle]
pub extern "C" fn native_timg0_init() {
    crate::native_mmio::init_timg0();
}

#[no_mangle]
pub extern "C" fn native_flash_init(flash_off: u32, mmu_region_id: u32) {
    crate::xtensa::memory::init_flash(flash_off, mmu_region_id);
}

// run265r: app .flash.text seg3 file offset (VMA 0x400D0020 is NOT at file
// offset 0 — the image is a bootloader container; seg3 lives at ~0x40020).
// JS parses the image headers once per boot and pushes the offset here so
// flash_mirror_read_u32 (HOOK scanner) reads the true .flash.text bytes.
#[no_mangle]
pub extern "C" fn native_flash_seg3_off(seg3_file_off: u32) {
    crate::xtensa::memory::init_flash_seg3(seg3_file_off);
}

#[no_mangle]
pub extern "C" fn core_init(core_idx: u32, processor_id: u32, _arch: u32) {
    let core = get_core(core_idx);
    core.index = core_idx;
    core.processor_id = processor_id;
    core.reset();
}

#[no_mangle]
pub extern "C" fn core_reset(core_idx: u32) {
    get_core(core_idx).reset();
}

#[no_mangle]
pub extern "C" fn core_step(core_idx: u32) -> u32 {
    let core = get_core(core_idx);
    core.sync_ccount();
    run_instruction(core)
}

/// Batched instruction runner. Executes up to `max_steps` *iterations* with a
/// single FFI call, where each iteration steps core 0 then core 1 (1:1
/// interleaving, identical to `chip.step()` calling `core_step` for both cores).
/// `sync_ccount` is applied per instruction exactly like `core_step`, so
/// CCOUNT/CCOMPARE and all per-instruction interrupt handling are preserved.
///
/// This is a pure speed optimization: the per-instruction JS↔WASM boundary
/// dominates `chip.step()`'s cost, and running both cores' instructions inside
/// one Rust call removes it. Event delivery cadence is unchanged because the
/// host pumps native timer events once per `chip.step()` call (which now covers
/// `max_steps * 2` instructions, the same instruction count the original pumped
/// every 512 single-instruction steps).
#[no_mangle]
pub extern "C" fn core_run(max_steps: u32) -> u32 {
    let mut n: u32 = 0;
    while n < max_steps {
        let mut ran: u32 = 0;
        {
            let core = get_core(0);
            if core.enabled != 0 {
                core.sync_ccount();
                run_instruction(core);
                ran = ran.wrapping_add(1);
            }
        }
        {
            let core = get_core(1);
            if core.enabled != 0 {
                core.sync_ccount();
                run_instruction(core);
                ran = ran.wrapping_add(1);
            }
        }
        n = n.wrapping_add(1);
        if ran == 0 {
            break;
        }
        let c0_idle = { let c = get_core(0); c.enabled == 0 || c.idle != 0 };
        let c1_idle = { let c = get_core(1); c.enabled == 0 || c.idle != 0 };
        if c0_idle && c1_idle {
            break;
        }
    }
    n
}

#[no_mangle]
pub extern "C" fn core_get_pc(core_idx: u32) -> u32 {
    get_core(core_idx).pc
}

#[no_mangle]
pub extern "C" fn core_get_debug_opcode(core_idx: u32) -> u32 {
    get_core(core_idx).debug_opcode
}

#[no_mangle]
pub extern "C" fn core_get_idle(core_idx: u32) -> u32 {
    get_core(core_idx).idle
}

#[no_mangle]
pub extern "C" fn core_set_idle(core_idx: u32, val: u32) {
    get_core(core_idx).idle = val;
}

#[no_mangle]
pub extern "C" fn core_get_special(core_idx: u32, reg: u32) -> u32 {
    get_core(core_idx).special_registers[reg as usize]
}

#[no_mangle]
pub extern "C" fn core_write_special(core_idx: u32, reg: u32, val: u32) {
    write_special_register(get_core(core_idx), reg, val);
}

// run176 diag: single-call snapshot of core interrupt state + BT progress.
// Uses the SAME get_core() path as core_get_pc (proven live by the JS SAB
// mirror + writeSABState PC tracking). selector: 0 = core regs (out0..7),
// 1 = matrix+BT (out0..7).
// | out ptr (8 x u32 LE written) |
#[no_mangle]
pub extern "C" fn native_bt_diag_scratch() -> u32 {
    unsafe {
        static mut SCRATCH: [u32; 8] = [0; 8];
        SCRATCH.as_mut_ptr() as u32
    }
}

#[no_mangle]
pub extern "C" fn native_bt_diag(core_sel: u32, out: u32) {
    let w = |i: usize, v: u32| unsafe {
        core::ptr::write_volatile((out as *mut u32).add(i), v);
    };
    if core_sel == 0 {
        for c in 0..2u32 {
            let core = get_core(c);
            let b = (c * 4) as usize;
            w(b, core.special_registers[INT_ENABLE]);
            w(b + 1, core.special_registers[CLOCK_CONFIG]);
            w(b + 2, core.special_registers[INT_SET] & 15);
            w(b + 3, core.pc);
        }
    } else {
        unsafe {
            let mut ctx = crate::native_mmio::make_ctx_pub();
            let s0 = crate::native_mmio::int_matrix_status_pub(&mut ctx, 0);
            let s1 = crate::native_mmio::int_matrix_status_pub(&mut ctx, 1);
            let m06 = crate::native_mmio::int_matrix_map_pub(0, 6);
            let m07 = crate::native_mmio::int_matrix_map_pub(0, 7);
            w(0, s0);
            w(1, s1);
            w(2, m06);
            w(3, m07);
            w(4, crate::native_mmio::bt_diag_state_pub());
            w(5, crate::xtensa::memory::dma_read_u32(0x3ffbff68));
            w(6, crate::xtensa::memory::dma_read_u32(0x3ffc3a60));
            w(7, crate::xtensa::memory::dma_read_u32(0x3ffc3a4c));
        }
    }
}

#[no_mangle]
pub extern "C" fn core_read_uint8(core_idx: u32, addr: u32) -> u32 {
    read_uint8(get_core(core_idx), addr)
}

#[no_mangle]
pub extern "C" fn core_read_uint16(core_idx: u32, addr: u32) -> u32 {
    read_uint16(get_core(core_idx), addr)
}

#[no_mangle]
pub extern "C" fn core_read_uint32(core_idx: u32, addr: u32) -> u32 {
    read_uint32(get_core(core_idx), addr)
}

#[no_mangle]
pub extern "C" fn core_write_uint8(core_idx: u32, addr: u32, val: u32) {
    write_uint8(get_core(core_idx), addr, val, 0);
}

#[no_mangle]
pub extern "C" fn core_write_uint16(core_idx: u32, addr: u32, val: u32) {
    write_uint16(get_core(core_idx), addr, val, 0);
}

#[no_mangle]
pub extern "C" fn core_write_uint32(core_idx: u32, addr: u32, val: u32) -> u32 {
    write_uint32(get_core(core_idx), addr, val, 0)
}

pub(crate) fn run_instruction(core: &mut CoreState) -> u32 {
    if core.enabled == 0 {
        return 0;
    }
    if core.pending_interrupts != 0 {
        core.next_pc = core.pc;
        core.update_interrupts();
        core.pc = core.next_pc;
    }
    if core.idle != 0 {
        check_ccompare(core);
        core.update_interrupts();
        if core.idle != 0 {
            return 0;
        }
    }

    let breakpoints = unsafe { has_breakpoint(core.index, core.pc) };
    if breakpoints != 0 {
        return 0;
    }

    // Native shims for BT hlevel queue wrappers (bypass broken double-deref;
    // returns true when the step was emulated and the original skipped).
    if crate::native_mmio::bt_shim_step(core) {
        return 0;
    }

    core.inst_count = core.inst_count.wrapping_add(1);
    unsafe {
        // run138: trace BOTH cores (was c1-only). Core-0 visibility is
        // required: the BTC thread is pinned to core 0 and never runs, so
        // the stall starver (if any) executes on c0. Mid-stall windows are
        // armed from JS via native_pc_trace (CMD_PCTRACE); T lines carry a
        // trailing core digit (len 73->74). PC_TRACE_CORE selects the core
        // (0/1/both=2): the btc_init entry arm narrows to c1, JS arms both.
        if PC_TRACE_LEFT > 0 && (PC_TRACE_CORE == 2 || PC_TRACE_CORE == core.index) {
            PC_TRACE_LEFT -= 1;
            let mut db = [0u8; 80];
            db[0] = b'T';
            for sh in [28, 24, 20, 16, 12, 8, 4, 0].iter() {
                db[(7 - sh / 4 + 1) as usize] = b"0123456789abcdef"[((core.pc >> sh) & 15) as usize];
            }
            let mut off = 9;
            // run137: a0/a2 added (call-return + trap-load regs). Layout:
            // a1 a13 a15 a3 a10 a14 a0 a2 (8 regs x 8 chars = 64, len 73).
            for r in [1u32, 13, 15, 3, 10, 14, 0, 2] {
                let v = core.ar(r);
                db[off] = b"QRSTUVWXYZabcdef"[(v >> 28 & 15) as usize];
                db[off + 1] = b"QRSTUVWXYZabcdef"[(v >> 24 & 15) as usize];
                db[off + 2] = b"QRSTUVWXYZabcdef"[(v >> 20 & 15) as usize];
                db[off + 3] = b"QRSTUVWXYZabcdef"[(v >> 16 & 15) as usize];
                db[off + 4] = b"QRSTUVWXYZabcdef"[(v >> 12 & 15) as usize];
                db[off + 5] = b"QRSTUVWXYZabcdef"[(v >> 8 & 15) as usize];
                db[off + 6] = b"QRSTUVWXYZabcdef"[(v >> 4 & 15) as usize];
                db[off + 7] = b"QRSTUVWXYZabcdef"[(v & 15) as usize];
                off += 8;
            }
            db[off] = b'0' + core.index as u8;
            crate::js_log_str(db.as_ptr() as u32, 74);
        }
    }
    if core.debug_log != 0 && (core.inst_count & 0xFFFF) == 0 {
        unsafe { js_log_u32(core.pc); }
    }

    // Log when entering flash cache region repeatedly
    if core.debug_log != 0 && core.pc >= 0x40080000 && core.pc < 0x40090000 && (core.inst_count & 0x7FFF) == 0 {
        unsafe { js_log_u32(core.pc); }
    }

    let pc = core.pc;
    unsafe { LAST_PC = pc; }
    if pc == 0x4005590f && unsafe { SPIN_LOGGED } == 0 {
        unsafe { SPIN_LOGGED = 1; }
        let mut db = [0u8; 220];
        let hx = |mut v: u32| -> [u8; 8] {
            let mut o = [0u8; 8];
            for i in (0..8).rev() { o[i] = b"0123456789abcdef"[(v & 0xF) as usize]; v >>= 4; }
            o
        };
        let mut n = 0;
        for &b in b"[SPIN] " { db[n] = b; n += 1; }
        let mut i = 0;
        for r in [2u32, 3, 8, 10, 12, 13, 14, 15] {
            for &b in b" a" { db[n] = b; n += 1; }
            db[n] = b'0' + (r / 10) as u8; if r >= 10 { n += 1; }
            db[n] = b'0' + (r % 10) as u8; n += 1;
            for &b in b"=" { db[n] = b; n += 1; }
            for &b in &hx(core.ar(r)) { db[n] = b; n += 1; }
            i += 1;
        }
        for &b in b" br=" { db[n] = b; n += 1; }
        for &b in &hx(core.br(8) as u32) { db[n] = b; n += 1; }
        unsafe { crate::js_log_str(db.as_ptr() as u32, n as u32); }
    }
    if pc >= 0x40091200 && pc <= 0x40091400 && unsafe { FREEZE_LOGGED } == 0 {
        unsafe { FREEZE_LOGGED = 1; }
        let mut db = [0u8; 200];
        let hx = |mut v: u32| -> [u8; 8] {
            let mut o = [0u8; 8];
            for i in (0..8).rev() { o[i] = b"0123456789abcdef"[(v & 0xF) as usize]; v >>= 4; }
            o
        };
        let mut n = 0;
        for &b in b"[FREEZE] pc=" { db[n] = b; n += 1; }
        for &b in &hx(pc) { db[n] = b; n += 1; }
        for &b in b" op=" { db[n] = b; n += 1; }
        for &b in &hx(core.last_opcode) { db[n] = b; n += 1; }
        for r in [0u32, 1, 2, 3, 8, 10, 12, 13, 14, 15] {
            for &b in b" a" { db[n] = b; n += 1; }
            db[n] = b'0' + (r / 10) as u8; if r >= 10 { n += 1; }
            db[n] = b'0' + (r % 10) as u8; n += 1;
            for &b in b"=" { db[n] = b; n += 1; }
            for &b in &hx(core.ar(r)) { db[n] = b; n += 1; }
        }
        unsafe { crate::js_log_str(db.as_ptr() as u32, n as u32); }
    }
    // run189 (2026-09-15): RETW tracer — proves whether future_ready
    // (0x40107dfc) ever RETURNS. Hook is the TRUE retw.n slot 0x40107e3b
    // (toolchain: e38 = memw, e3b = retw.n). Logs pc + a2 + live future
    // word [a2+16] (NOT the hardcoded 0x3ffd0d6c — the future MOVED in
    // later runs). Budget 6, both cores.
    if pc == 0x40107e3b {
        static mut RETW_N: u32 = 0;
        unsafe {
            if RETW_N < 6 {
                RETW_N += 1;
                let mut db = [0u8; 128];
                let hx = |mut v: u32| -> [u8; 8] {
                    let mut o = [0u8; 8];
                    for i in (0..8).rev() { o[i] = b"0123456789abcdef"[(v & 0xF) as usize]; v >>= 4; }
                    o
                };
                let fut = core.ar(2);
                let mut n = 0;
                for &b in b"[RETW] pc=" { db[n] = b; n += 1; }
                for &b in &hx(pc) { db[n] = b; n += 1; }
                for &b in b" c=" { db[n] = b; n += 1; }
                db[n] = b'0' + core.index as u8; n += 1;
                for &b in b" a2=" { db[n] = b; n += 1; }
                for &b in &hx(fut) { db[n] = b; n += 1; }
                for &b in b" fut16=" { db[n] = b; n += 1; }
                let w16 = if (0x3ffb0000..0x40000000).contains(&fut) { crate::xtensa::memory::dma_read_u32(fut.wrapping_add(16)) } else { 0xDEADDEAD };
                for &b in &hx(w16) { db[n] = b; n += 1; }
                crate::js_log_str(db.as_ptr() as u32, n as u32);
            }
        }
    }
    // run190 (2026-09-15): future_await SPIN-LOOP decoder — log c1's full
    // a-reg frame + engine-decoded op/w/nxt the first 8 times it spins
    // INSIDE future_await's wait loop (0x40107e8c-0x40107f00). The op words
    // disassemble the loop without toolchain objdump (raw-words problem);
    // the load addr + compare value identify the exact polled word + wake
    // value. (FUTSPIN history: original run180f watch was retired with the
    // DEC window; this restores it WITH opcode decode.)
    if pc >= 0x40107e8c && pc < 0x40107f00 && core.index == 1 {
        static mut FUTSPIN_N: u32 = 0;
        unsafe {
            if FUTSPIN_N < 8 {
                FUTSPIN_N += 1;
                let mut db = [0u8; 512];
                let hx = |mut v: u32| -> [u8; 8] {
                    let mut o = [0u8; 8];
                    for i in (0..8).rev() { o[i] = b"0123456789abcdef"[(v & 0xF) as usize]; v >>= 4; }
                    o
                };
                let mut n = 0;
                for &b in b"[FUTSPIN] pc=" { db[n] = b; n += 1; }
                for &b in &hx(pc) { db[n] = b; n += 1; }
                {
                    let op16 = read_opcode_u16(core, pc);
                    let w = INST_WIDTH_TABLE[(op16 & 15) as usize];
                    let op = if w == 4 { read_opcode_u32(core, pc) } else if w == 3 { op16 | (read_opcode_u8(core, pc + 2) << 16) } else { op16 };
                    for &b in b" op=" { db[n] = b; n += 1; }
                    for &b in &hx(op) { db[n] = b; n += 1; }
                    for &b in b" w=" { db[n] = b; n += 1; }
                    db[n] = b'0' + w as u8; n += 1;
                    for &b in b" nxt=" { db[n] = b; n += 1; }
                    for &b in &hx(core.pc.wrapping_add(w as u32)) { db[n] = b; n += 1; }
                }
                for r in 0..16u32 {
                    for &b in b" a" { db[n] = b; n += 1; }
                    if r >= 10 { db[n] = b'1'; n += 1; db[n] = b'0' + (r - 10) as u8; n += 1; }
                    else { db[n] = b'0' + r as u8; n += 1; }
                    for &b in b"=" { db[n] = b; n += 1; }
                    for &b in &hx(core.ar(r)) { db[n] = b; n += 1; }
                }
                crate::js_log_str(db.as_ptr() as u32, n as u32);
            }
        }
    }
    // run112: RE-ADD per-PC LOOP watch for the NULL-check site 0x4009379f
    // (one-shot: first hit wins — healthy or NULL, either maps the slot).
    // The RING never contains 979f (no trigger covers it) and the nulltake
    // hook's word-verify may be failing blind. Full a0-a15 dump.
    // run116: LOOP shows HEALTHY (a2=3ffc3d70) — one-shot spent pre-wedge.
    // Make it REPEATABLE (budget 4, all hits logged): the NULL-hit frame
    // (a2==0) will then appear verbatim in some run.
    if pc == 0x4009379f && unsafe { LOOP979F_N } < 4 {
        unsafe { LOOP979F_N += 1; }
        let mut db = [0u8; 448];
        let hx = |mut v: u32| -> [u8; 8] {
            let mut o = [0u8; 8];
            for i in (0..8).rev() { o[i] = b"0123456789abcdef"[(v & 0xF) as usize]; v >>= 4; }
            o
        };
        let mut n = 0;
        for &b in b"[LOOP] pc=" { db[n] = b; n += 1; }
        for &b in &hx(pc) { db[n] = b; n += 1; }
        for r in 0..16u32 {
            for &b in b" a" { db[n] = b; n += 1; }
            if r >= 10 { db[n] = b'1'; n += 1; db[n] = b'0' + (r - 10) as u8; n += 1; }
            else { db[n] = b'0' + r as u8; n += 1; }
            for &b in b"=" { db[n] = b; n += 1; }
            for &b in &hx(core.ar(r)) { db[n] = b; n += 1; }
        }
        unsafe { crate::js_log_str(db.as_ptr() as u32, n as u32); }
    }
    if [0x400076dd, 0x400076e0, 0x400076e2, 0x4000fca9, 0x4000fcac, 0x4000fcae].contains(&pc) && unsafe { LOOP_LOGGED } == 0 {
        unsafe { LOOP_LOGGED = 1; }
        let mut db = [0u8; 200];
        let hx = |mut v: u32| -> [u8; 8] {
            let mut o = [0u8; 8];
            for i in (0..8).rev() { o[i] = b"0123456789abcdef"[(v & 0xF) as usize]; v >>= 4; }
            o
        };
        let mut n = 0;
        for &b in b"[LOOP] pc=" { db[n] = b; n += 1; }
        for &b in &hx(pc) { db[n] = b; n += 1; }
        for &b in b" a1=" { db[n] = b; n += 1; }
        for &b in &hx(core.ar(1)) { db[n] = b; n += 1; }
        for &b in b" a2=" { db[n] = b; n += 1; }
        for &b in &hx(core.ar(2)) { db[n] = b; n += 1; }
        for &b in b" a3=" { db[n] = b; n += 1; }
        for &b in &hx(core.ar(3)) { db[n] = b; n += 1; }
        for &b in b" a4=" { db[n] = b; n += 1; }
        for &b in &hx(core.ar(4)) { db[n] = b; n += 1; }
        for &b in b" a8=" { db[n] = b; n += 1; }
        for &b in &hx(core.ar(8)) { db[n] = b; n += 1; }
        for &b in b" a10=" { db[n] = b; n += 1; }
        for &b in &hx(core.ar(10)) { db[n] = b; n += 1; }
        for &b in b" a15=" { db[n] = b; n += 1; }
        for &b in &hx(core.ar(15)) { db[n] = b; n += 1; }
        unsafe { crate::js_log_str(db.as_ptr() as u32, n as u32); }
    }
    if pc >= 0x40083800 && pc <= 0x40083a00 && unsafe { GARBAGE_PC_LOGGED } == 0 {
        unsafe { GARBAGE_PC_LOGGED = 1; }
        let mut db = [0u8; 320];
        let hx = |mut v: u32| -> [u8; 8] {
            let mut o = [0u8; 8];
            for i in (0..8).rev() { o[i] = b"0123456789abcdef"[(v & 0xF) as usize]; v >>= 4; }
            o
        };
        let mut n = 0;
        for &b in b"[GARBAGE] pc=" { db[n] = b; n += 1; }
        for &b in &hx(pc) { db[n] = b; n += 1; }
        for &b in b" a0=" { db[n] = b; n += 1; }
        for &b in &hx(core.ar(0)) { db[n] = b; n += 1; }
        for &b in b" a1=" { db[n] = b; n += 1; }
        for &b in &hx(core.ar(1)) { db[n] = b; n += 1; }
        for &b in b" a2=" { db[n] = b; n += 1; }
        for &b in &hx(core.ar(2)) { db[n] = b; n += 1; }
        let sp = core.ar(1);
        for k in 0..20 {
            for &b in b" stk=" { db[n] = b; n += 1; }
            let w = read_page_table(core, sp + k * 4, 32);
            for &b in &hx(w) { db[n] = b; n += 1; }
        }
        unsafe { crate::js_log_str(db.as_ptr() as u32, n as u32); }
    }
    unsafe {
        let ri = (RING_IDX % 512) as usize;
        RING[ri][0] = pc;
        RING[ri][1] = core.ar(0);
        RING[ri][2] = core.ar(1);
        RING[ri][3] = core.ar(8);
        RING[ri][4] = core.ar(10);
        RING[ri][5] = core.ar(15);
        RING_IDX += 1;
        // run115: old trigger (0x40083801 boot park) dumps boot noise only.
        // Retarget to the LIVE wedge: r_rwbtdm_isr entry (0x4008f96c,
        // newest RING pcs prove it runs at stall time) + the future_await
        // spin (0x4010f308, where bluedroid_init blocks). Throttle 50k,
        // budget 3 (same bounds as before).
        // run117: NO dump fired (neither pc hit in 120s!). The stall is NOT
        // at the ISR nor future_await — and TCBs show BOTH tasks parked
        // with evC==0 (NOT event-blocked: stC=delayed-list?). stC=0x3ffc3cb8
        // (btCtl) / 0x3ffc3c68 (BTC) vs evC==0x00000000. evC==0 means NOT
        // blocked on an event — the tasks never even waited?! BTC created
        // but bluedroid_init's future_await never armed? Trigger INSTEAD on
        // the top RD spinners: tlsf_create loop (0x4018ae82) is boot noise;
        // the heap-mutex pair (0x40093eba/fc2/fc7) is the wedge. Watch
        // 0x40093eba (repeatable budget 2): LOOP dump shows mux+owner words
        // at the ACTUAL hot spin.
        static mut RING_DUMPS: u32 = 0;
        static mut RING_LAST: u32 = 0;
        if (pc == 0x4008f96c || pc == 0x4010f308 || pc == 0x40093eba)
            && RING_DUMPS < 4
            && RING_IDX.wrapping_sub(RING_LAST) >= 50000
        {
            RING_DUMPS += 1;
            RING_LAST = RING_IDX;
            let hx = |mut v: u32| -> [u8; 8] {
                let mut o = [0u8; 8];
                for i in (0..8).rev() { o[i] = b"0123456789abcdef"[(v & 0xF) as usize]; v >>= 4; }
                o
            };
            let mut n = 0;
            for &b in b"[RING] pc=" { DBG_BUF[n] = b; n += 1; }
            for &b in &hx(pc) { DBG_BUF[n] = b; n += 1; }
            for &b in b" a0=" { DBG_BUF[n] = b; n += 1; }
            for &b in &hx(core.ar(0)) { DBG_BUF[n] = b; n += 1; }
            for &b in b" a1=" { DBG_BUF[n] = b; n += 1; }
            for &b in &hx(core.ar(1)) { DBG_BUF[n] = b; n += 1; }
            for &b in b" a8=" { DBG_BUF[n] = b; n += 1; }
            for &b in &hx(core.ar(8)) { DBG_BUF[n] = b; n += 1; }
            for &b in b" a10=" { DBG_BUF[n] = b; n += 1; }
            for &b in &hx(core.ar(10)) { DBG_BUF[n] = b; n += 1; }
            for &b in b" sp14=" { DBG_BUF[n] = b; n += 1; }
            for &b in &hx(read_page_table(core, core.ar(1) + 56, 32)) { DBG_BUF[n] = b; n += 1; }
            let sp = core.ar(1);
            for k in 0..20 {
                for &b in b" stk=" { DBG_BUF[n] = b; n += 1; }
                let w = read_page_table(core, sp + k * 4, 32);
                for &b in &hx(w) { DBG_BUF[n] = b; n += 1; }
            }
            for i in 0..512 {
                let r = ((RING_IDX.wrapping_add(i)) as usize) % 512;
                for &b in b" t=" { DBG_BUF[n] = b; n += 1; }
                for &b in &hx(RING[r][0]) { DBG_BUF[n] = b; n += 1; }
                for &b in b":" { DBG_BUF[n] = b; n += 1; }
                for &b in &hx(RING[r][3]) { DBG_BUF[n] = b; n += 1; }
            }
            for i in 0..512 {
                let r = ((WR_IDX.wrapping_add(i)) as usize) % 512;
                for &b in b" w=" { DBG_BUF[n] = b; n += 1; }
                for &b in &hx(WR_RING[r][0]) { DBG_BUF[n] = b; n += 1; }
                for &b in b":" { DBG_BUF[n] = b; n += 1; }
                for &b in &hx(WR_RING[r][1]) { DBG_BUF[n] = b; n += 1; }
                for &b in b"=" { DBG_BUF[n] = b; n += 1; }
                for &b in &hx(WR_RING[r][2]) { DBG_BUF[n] = b; n += 1; }
            }
            crate::js_log_str(DBG_BUF.as_ptr() as u32, n as u32);
        }
    }
    let seg = core.opcode_segment;
    if seg != (-8192i32 as u32) & pc {
        get_opcode_memory(core, pc);
        core.opcode_segment = (-8192i32 as u32) & pc;
    }

    // run118b: extend decode-trace window to bluedroid_init_with_cfg
    // (0x400e28b8-0x400e2930, ADV build 96e3c02c) — bundled objdump emits
    // raw words there; the engine decoder is ground truth. Traced via
    // native_set_decode_trace + [DEC] lines.
    // run118c: ARMED ALWAYS (no export call needed): trace first 40000
    // insns in the window per boot (one-shot budget, zero host wiring).
    // run119: WRONG BUILD — 96e3c02c != ADV f13066 (init at 0x400e28b8 vs
    // f13066 0x400e2a9c/0x400e2b14/0x400e2a40). Retarget to the ADV build:
    // esp_bluedroid_init_with_cfg 0x400e2a9c, esp_bluedroid_init 0x400e2b14,
    // esp_bluedroid_enable 0x400e2a40, btu_init_core 0x400f3828, btc_init
    // 0x4010bba8, future_new 0x4010f2d4, future_await 0x4010f308.
    // run120: 24 DEC lines then execution leaves window at 0x400e2abf
    // (call8 far target). FOLLOW: also trace btu_init_core (0x400f3828),
    // btc_init (0x4010bba8), future_new/await (0x4010f2d4/0x4010f308),
    // btc_init_callback path + the 0x400e2abf call target (resolved below).
    // Keep per-boot budget (raise to 120k: init chain is long).
    // run121: reached btc_init+0x1E (4010bbc6) then silence — btc_init's
    // callees live OUTSIDE the window (osi/future/btu at other addresses).
    // Widen: whole-bluedroid range 0x400e2000-0x400e3000 is cheap (only hit
    // during BT bring-up), plus btu (0x400f3800-0x400f4000), btc task
    // (0x40106000-0x40118000), osi/future (0x4010f200-0x40110000).
    // Budget stays 120k (init chain bounded; stall loops elsewhere).
    // run124: init map COMPLETE through controller_init entry (0x400e2813)
    // and beyond (0x400e287f+). Stall is DEEPER — past with_cfg, past
    // controller_init prologue. Keep window (already covers 0x400e2xxx);
    // raise budget to 400k so the trace reaches the true stall (bluedroid
    // callback / VHCI / task-wait). Log volume ~400k lines worst case —
    // acceptable for one diag run (filtered by grep).
    // run125: 400k always-on trace is TOO HEAVY (log floods, run slows,
    // budget burns before the stall). Replace with targeted low-volume
    // hooks: function-ENTRY log (one line per entry, budget 2k) for the
    // init chain + a STALL detector (same pc 1M steps -> dump). The DEC
    // machinery stays for manual use via native_set_decode_trace.
    // run180k (2026-09-15, e7f2): DEC-window retarget — decode btc_init_callback
    // body (0x400e2b10-0x400e2b24, 5 insns + entry). The callback's future*
    // register + queue-post sequence are unknown (raw-words objdump); the
    // engine decoder is ground truth. One-shot 200-insn budget, c0 only
    // (callback runs on c0 per FENT), zero host wiring.
    // run181 (2026-09-15): budget 200 -> 2000. The 200-line window expired
    // mid-setter (deep in xQueueGenericSend/lock at 40093ea4, never showing
    // the return). 2000 covers setter return + future write + callback retw
    // + btc_init resume (or the exact wedge pc if it never returns).
    // run182: S32C1I-failure watch on the setter tail. The tail spins on
    // CAS (S32C1I via _handler18: mem[SAR] vs newval): log the first 8
    // failures with addr+mem+SAR — a stuck kernel lock (xKernelLock @
    // 0x3ffbdde0) shows as repeated failures on the SAME addr, which is the
    // wedge signature (lock never released because the holder never runs).
    // run184 (2026-09-15): REPLACED by the episodic sampler below (per-step
    // dma_read_u32 in this hot range cost ~30% wall time and fired on every
    // boot's benign lock traffic). Kept compiled-out for reference.
    if false && pc >= 0x40093800 && pc < 0x40094000 && core.index == 0 {
        static mut S32CFAIL_N: u32 = 0;
        unsafe {
            if S32CFAIL_N < 8 {
                // Detect S32C1I failure AFTER execution: the handler already
                // ran by the time we see the next pc. Instead sample the
                // lock word directly: if xKernelLock != 0 someone holds it.
                let lv = crate::xtensa::memory::dma_read_u32(0x3ffbdde0);
                if lv != 0 {
                    S32CFAIL_N += 1;
                    let mut db = [0u8; 96];
                    let hx = |mut v: u32| -> [u8; 8] {
                        let mut o = [0u8; 8];
                        for i in (0..8).rev() { o[i] = b"0123456789abcdef"[(v & 0xF) as usize]; v >>= 4; }
                        o
                    };
                    let mut n = 0;
                    for &b in b"[S32C] pc=" { db[n] = b; n += 1; }
                    for &b in &hx(pc) { db[n] = b; n += 1; }
                    for &b in b" lock=" { db[n] = b; n += 1; }
                    for &b in &hx(lv) { db[n] = b; n += 1; }
                    for &b in b" a2=" { db[n] = b; n += 1; }
                    for &b in &hx(core.ar(2)) { db[n] = b; n += 1; }
                    crate::js_log_str(db.as_ptr() as u32, n as u32);
                }
            }
        }
    }
    // run184 (2026-09-15): episodic ISR-epoch sampler. Fires once every 2M
    // c0 steps while inside the BT ISR vector page (0x40080000-0x40084000):
    // one compact line with IE25(mask-gated line state), matrix STATUS0,
    // MISC1/EPC slot (take_interrupt saves pre-vector pc there), VEC25
    // census, and ISR-active flag. Proves per-epoch whether the ISR drains
    // work (EPC varies, STATUS consumed) or re-vectors sterilely (same EPC,
    // STATUS stuck). ~10 lines per 20s probe — zero flood risk.
    if (pc & 0xFFFFC000) == 0x40080000 && core.index == 0 {
        static mut EPOCH_N: u32 = 0;
        static mut EPOCH_LAST: u32 = 0;
        unsafe {
            EPOCH_N = EPOCH_N.wrapping_add(1);
            if EPOCH_N.wrapping_sub(EPOCH_LAST) >= 2000000 {
                EPOCH_LAST = EPOCH_N;
                let mut db = [0u8; 160];
                let hx = |mut v: u32| -> [u8; 8] {
                    let mut o = [0u8; 8];
                    for i in (0..8).rev() { o[i] = b"0123456789abcdef"[(v & 0xF) as usize]; v >>= 4; }
                    o
                };
                let mut n = 0;
                for &b in b"[EPOCH] pc=" { db[n] = b; n += 1; }
                for &b in &hx(pc) { db[n] = b; n += 1; }
                for &b in b" ie=" { db[n] = b; n += 1; }
                for &b in &hx(core.special_registers[crate::xtensa::constants::INT_ENABLE]) { db[n] = b; n += 1; }
                for &b in b" epc=" { db[n] = b; n += 1; }
                for &b in &hx(core.special_registers[crate::xtensa::constants::MISC1_REGISTER + 2]) { db[n] = b; n += 1; }
                for &b in b" vec=" { db[n] = b; n += 1; }
                let mut v = crate::native_mmio::bt_vec_count();
                let mut digits = [0u8; 10];
                let mut nd = 0;
                if v == 0 { digits[0] = b'0'; nd = 1; }
                while v > 0 { digits[nd] = b'0' + (v % 10) as u8; v /= 10; nd += 1; }
                while nd > 0 { nd -= 1; db[n] = digits[nd]; n += 1; }
                crate::js_log_str(db.as_ptr() as u32, n as u32);
            }
        }
    }
    // run125: targeted ENTRY trace — one short line per init-chain function
    // entry (adv-build addrs from nm), budget 2k shared. Cheap: a compare
    // chain per step, one js_log per ENTRY (not per insn).
    // run126: init->with_cfg->btc_init reached, then STOPS. btc_init's
    // callees next: osi_thread_create 0x4010fce8, xTaskCreatePinnedToCore
    // 0x40096844, vTaskDelay 0x40095eb4, xQueueGenericSend 0x40093798
    // (already LOOP-watched), btu path later. Add them here.
    // run127: btc_init calls osi_thread_create (a0=4010bbc6 inside btc_init)
    // which calls xTaskCreatePinnedToCore. Stall is INSIDE task creation.
    // Add: pvPortMalloc 0x40094388, pxPortInitialiseStack 0x40093df8,
    // vListInitialise 0x40094a48, xTaskCreateUniversal 0x400d2794.
    // run128: chain reaches pxPortInitialiseStack then STOPS — the stall is
    // AFTER stack-init inside xTaskCreatePinnedToCore: next callees are
    // vListInitialiseItem 0x40094a70, vListInsertEnd 0x40094a80, vListInsert
    // 0x40094abc, prvTakeKernelLock 0x400967b4, prvReleaseKernelLock
    // 0x400967c4. Add them.
    // run129: vListInsertEnd fires ONCE (c0, early boot task), then the
    // c1 chain stops AT vListInsert (a0=80093c2c). Next inside vListInsert:
    // uxListRemove 0x40094b1c, memcpy 0x40090518, memset 0x400906ac. Add them.
    // run130: c1 tail = FLASH driver (memcpy/memset from esp_flash_write),
    // NOT BT — the BT chain ended earlier (last BT pc 400e2813 =
    // controller_init entry, then heap/list traffic). The STALL is between
    // controller_init entry and ITS first callee trace. controller_init's
    // callees (from the f13066 map): btdm_controller_init 0x40162571 is
    // CALLED (FENT a0 shows it) — trace INSIDE it: r_rw_pre_main 0x40161b68,
    // r_rw_schedule 0x40161c24, ke_task_init?, btdm_task_create?. Add
    // 0x40162571 (btdm_controller_init) + 0x40161b68 + 0x40161c24.
    // run131: btdm_controller_init ENTERS but pre_main/schedule NEVER fire.
    // Its prologue calls sdk_config_set_mask 0x40177b14, sdk_config_set_opts
    // 0x40177b28, sdk_config_get_opts 0x4008f204 (objdump-verified). Add them.
    // run132: set_mask/opts NEVER fire either, and memcpy/memset (0x40090518/
    // 0x400906ac) eat 1732/2000 of the budget — DROP them from the whitelist
    // (they fire per-byte during flash/NVS writes, drowning the BT chain).
    // Keep xTaskCreateUniversxal (0x400d2794) as the anchor. Add the
    // btdm return-path: esp_bluedroid_init_with_cfg is called with
    // a0=0x800d1c69 (loop/setup); bluedroid's OWN chain after btc_init and
    // btu_init_core is what we need past 40162571.
    // run146: post-btc_init drain. NEW: future_new 0x4010f2d4 (did with_cfg
    // arm it?), osi_thread_run?? (inlined into osi_thread_create — the T
    // trace shows create returns straight to btc_init+176), osi_sem_take
    // 0x401100a0 (start_sem rendezvous: create BLOCKS until the new task's
    // osi_thread_run gives start_sem — if the task never runs, create
    // never returns!), xQueueSemaphoreTake 0x40093b5c (sem body),
    // bte_main_boot_entry 0x400e58ac, osi_init 0x4011004c,
    // hci_start_up 0x4011f388, BTU_StartUp 0x400f38b8,
    // btu_init_core 0x400f3828, btc_init_callback 0x400e3c74.
    // xQueueGenericCreate 0x40093694, xQueueCreateMutex 0x4009391c,
    // future_ready 0x4010f278, btu_task_start_up 0x400f3b64,
    // btc_init_callback 0x400e3c74, btc_main_call_handler 0x400e3c94.
    // DROP task-create internals (proven) + sdk_config_get_opts noise
    // (42 hits of bt-mode reads, no signal).
    // run134: FULL btc_init body dump via PC-trace-on-entry. When c1 hits
    // btc_init entry (0x4010bba8), arm native_pc_trace for 600 insns: the
    // [T] lines (pc + a1/a13/a15/a3/a10/a14 regs) show the body including
    // the faulting load (a2=0 -> l32i trap pc) without manual objdump.
    // Budget: one-shot (first entry only), zero cost otherwise.
    // run135: RE-ARM every entry (budget 4): run134's 600-line window was
    // consumed by the FIRST btc_init call (early boot, before BT); the BT
    // call (2nd, from esp_bluedroid_init_with_cfg) got nothing. 600 lines
    // is plenty (body is ~30 insns + allocator calls); 4 arms cover early
    // + BT calls with margin. ALSO re-arm native FENT (static FENT_N is
    // one-shot per boot already — no change needed).
    // run137: arm 50000 c1-insns (covers btc_init body + everything after
    // until the true stall; the window is consumed ONLY by c1 now).
    // Budget: still 4 arms (first arm fires at the BT btc_init call since
    // earlier boot calls... actually btc_init is called ONCE — run134/135
    // prove a single BT-time call; arms 2-4 never fire. 50k >> 600 needed
    // because osi_thread_create + allocator descent is thousands of insns.
    // run138: RETIRE the btc_init-entry auto-arm (mid-stall windows are now
    // armed from JS via native_pc_trace; entry-arming only captures boot).
    // Keep the block compiled-out (not deleted) so the FENT whitelist below
    // keeps its run-history comments intact.
    // run160: WB rotation audit (CAS newval chain). Log WB (spec[72]) +
    // a4/a12 at Timeout entry (0x40093ea4) and CAS entry (0x40091220).
    // run162: STALL-PHASE ONLY (c1 + xTickCount>0x1000): early-boot calls
    // consumed the 4-budget before the wedge formed. The wedge-path
    // rotation question (CAS-entry a4 vs Timeout a12) needs STALL hits.
    if (pc == 0x40093ea4 || pc == 0x40091220) && core.index == 1 {
        static mut WB_N: u32 = 0;
        unsafe {
            let tick = crate::xtensa::memory::dma_read_u32(0x3ffc3a60);
            if tick > 0x1000 && WB_N < 6 {
                WB_N += 1;
                let mut db = [0u8; 96];
                let hx = |mut v: u32| -> [u8; 8] {
                    let mut o = [0u8; 8];
                    for i in (0..8).rev() { o[i] = b"0123456789abcdef"[(v & 0xF) as usize]; v >>= 4; }
                    o
                };
                let mut n = 0;
                for &b in b"[WB] pc=" { db[n] = b; n += 1; }
                for &b in &hx(pc) { db[n] = b; n += 1; }
                for &b in b" c=" { db[n] = b; n += 1; }
                db[n] = b'0' + core.index as u8; n += 1;
                for &b in b" wb=" { db[n] = b; n += 1; }
                for &b in &hx(core.special_registers[72]) { db[n] = b; n += 1; }
                for &b in b" a4=" { db[n] = b; n += 1; }
                for &b in &hx(core.ar(4)) { db[n] = b; n += 1; }
                for &b in b" a12=" { db[n] = b; n += 1; }
                for &b in &hx(core.ar(12)) { db[n] = b; n += 1; }
                crate::js_log_str(db.as_ptr() as u32, n as u32);
            }
        }
    }
    // run188 (2026-09-15): ISR-body-entry probe. take_interrupt vectors to
    // the ROM level-4 vector page, but EPOCH shows c0 parked in _xt_lowint1
    // (0x40083fxx) — the ROM dispatcher never reaches the LL bodies. Count
    // executions of the three LL entry pcs directly: r_rwble_isr 0x4008e084,
    // r_rwbt_isr 0x4008ea2c, r_rwbtdm_isr_wrapper 0x4008f700. Zero-cost
    // unless hit (3 compares/step); budget 16 logs so a late first-entry
    // still shows. Format matches FENT for greppability.
    if pc == 0x4008e084 || pc == 0x4008ea2c || pc == 0x4008f700 {
        static mut ISRE_N: u32 = 0;
        unsafe {
            if ISRE_N < 16 {
                ISRE_N += 1;
                let mut db = [0u8; 64];
                let hx = |mut v: u32| -> [u8; 8] {
                    let mut o = [0u8; 8];
                    for i in (0..8).rev() { o[i] = b"0123456789abcdef"[(v & 0xF) as usize]; v >>= 4; }
                    o
                };
                let mut n = 0;
                for &b in b"[ISRE] c" { db[n] = b; n += 1; }
                db[n] = b'0' + core.index as u8;
                n += 1;
                for &b in b" " { db[n] = b; n += 1; }
                for &b in &hx(pc) { db[n] = b; n += 1; }
                for &b in b" n=" { db[n] = b; n += 1; }
                let mut v = ISRE_N;
                let mut digits = [0u8; 10];
                let mut nd = 0;
                if v == 0 { digits[0] = b'0'; nd = 1; }
                while v > 0 { digits[nd] = b'0' + (v % 10) as u8; v /= 10; nd += 1; }
                while nd > 0 { nd -= 1; db[n] = digits[nd]; n += 1; }
                crate::js_log_str(db.as_ptr() as u32, n as u32);
            }
        }
    }
    // run179 (2026-09-15, e7f2 build): enable-path ENTRY trace. White-list =
    // e7f2 nm truth (NEVER reuse stale f13066 addrs): esp_bluedroid_enable
    // 0x400e2a1c, btc_init_callback 0x400e2b10, btc_main_call_handler
    // 0x400e2b30, btc_transfer_context 0x40106870, btc_init 0x4010692c,
    // future_new 0x40107e58, future_await 0x40107e8c, osi_thread_create
    // 0x4010886c, btu_init_core 0x400ee740, btu_task_start_up 0x400eea7c,
    // btu_task_post 0x400eeaf8, btdm_controller_task 0x40161af4,
    // r_rw_schedule 0x401619dc, btdm_task_post 0x40177970,
    // API_vhci_host_send_packet 0x40177d30, btdm_controller_enable
    // 0x40162008, esp_bt_controller_enable 0x400e284c (+ the two observed
    // stall pcs 0x400e2870/0x40095eb4). Logs pc + a0 (return addr) so the
    // caller chain is visible without a backtrace.
    // run195: + queue-create path (the future-sem poisoner): 0x400e2bd8
    // (init dispatch: s32i [g_var]=future, calls osi_mutex_new 0x40108bd0
    // + queue-create 0x4011c1f4), fixed_pkt_queue_new 0x40136060,
    // queue-process 0x401360a8, 0x40135fd4, osi_xxx 0x40108af4.
    // run212: + future signal path + queue kernel: future_ready 0x40107dfc,
    // osi_sem_give 0x40108c10, xQueueGenericSend 0x40093798,
    // xQueueGenericCreate 0x40093694, prvAddNewTaskToReadyList 0x4009351c,
    // + the bte handler callees 0x40107718/0x40107750/0x4010cf0c/0x4010d5f8/
    // 0x40109bec. Logs pc + a0 (return addr) + a2 (first arg).
    // run226: + btc_init tail (the TRUE waiter-park site): 0x40117e30,
    // 0x40117018, 0x40117034, 0x401067c4 (fail path), osi_thread_create's
    // post cortex 0x40108981/0x401089d2.
    // run179 (2026-09-15, e7f2 build): enable-path ENTRY trace. White-list =
    // e7f2 nm truth (NEVER reuse stale f13066 addrs): esp_bluedroid_enable
    // 0x400e2a1c, btc_init_callback 0x400e2b10, btc_main_call_handler
    // 0x400e2b30, btc_transfer_context 0x40106870, btc_init 0x4010692c,
    // future_new 0x40107e58, future_await 0x40107e8c, osi_thread_create
    // 0x4010886c, btu_init_core 0x400ee740, btu_task_start_up 0x400eea7c,
    // btu_task_post 0x400eeaf8, btdm_controller_task 0x40161af4,
    // r_rw_schedule 0x401619dc, btdm_task_post 0x40177970,
    // API_vhci_host_send_packet 0x40177d30, btdm_controller_enable
    // 0x40162008, esp_bt_controller_enable 0x400e284c (+ the two observed
    // stall pcs 0x400e2870/0x40095eb4). Logs pc + a0 (return addr) so the
    // caller chain is visible without a backtrace.
    // run195: + queue-create path (the future-sem poisoner): 0x400e2bd8
    // (init dispatch: s32i [g_var]=future, calls osi_mutex_new 0x40108bd0
    // + queue-create 0x4011c1f4), fixed_pkt_queue_new 0x40136060,
    // queue-process 0x401360a8, 0x40135fd4, osi_xxx 0x40108af4.
    // run212: + future signal path + queue kernel: future_ready 0x40107dfc,
    // osi_sem_give 0x40108c10, xQueueGenericSend 0x40093798,
    // xQueueGenericCreate 0x40093694, prvAddNewTaskToReadyList 0x4009351c,
    // + the bte handler callees 0x40107718/0x40107750/0x4010cf0c/0x4010d5f8/
    // 0x40109bec. Logs pc + a0 (return addr) + a2 (first arg).
    // run226: + btc_init tail (the TRUE waiter-park site): 0x40117e30,
    // 0x40117018, 0x40117034, 0x401067c4 (fail path), osi_thread_create's
    // post cortex 0x40108981/0x401089d2.
    // run264 (2026-09-17): MULTI-BUILD whitelist. e7f2 literals below are now
    // OR-ed with the 62ea (ADV, build 62eae06cf35ccac1) + f130 (ADV-1055,
    // build f13066b2600b189f) nm truth — all three builds' pcs fire FENT.
    // Kernel pcs (40093xxx/40095eb4) are STABLE across builds (single set).
    // NOTE: 0x400e2a1c vs 62ea 0x400e2a2c vs f130 0x400e2a40 (enable) etc.
    // differ per build — keep ALL literals; unknown future builds just won't
    // fire FENT (diag-only loss, zero functional effect).
    if pc == 0x400e2a1c || pc == 0x400e2a2c || pc == 0x400e2a40 // enable (e7f2/62ea/f130)
        || pc == 0x400e2b10 || pc == 0x400e3c60 || pc == 0x400e3c74 // btc_init_callback
        || pc == 0x400e2b1c // btc_init_callback pre-call (run293: prove the call8 body runs)
        || pc == 0x400e2b30 || pc == 0x400e3c80 || pc == 0x400e3c94 // btc_main_call_handler
        || pc == 0x400e2bd8
        || pc == 0x40108bd0 || pc == 0x4010fb08 || pc == 0x4010fb34 // osi_mutex_new
        || pc == 0x4011c1f4 || pc == 0x4011f35c || pc == 0x4011f388 // hci_start_up (queue-create)
        || pc == 0x40136060 || pc == 0x40136254 || pc == 0x40136280 // fixed_pkt_queue_new
        || pc == 0x401360a8 || pc == 0x4013629c || pc == 0x401362c8 // queue-process
        || pc == 0x40135fd4
        || pc == 0x40108af4
        || pc == 0x40107dfc || pc == 0x4010f24c || pc == 0x4010f278 // future_ready
        || pc == 0x40108c10 || pc == 0x40110060 || pc == 0x4011008c // osi_sem_give
        || pc == 0x40108a7c // osi_sem_take wrapper (btc_transfer's take path; own frame a1,48)
        || pc == 0x4010b758 // take-wrapper inner tail (7d1e668a; a2 live here — same frame as wrapper entry)
        || pc == 0x4010fcf8 // take site called by btc_transfer_context (7d1e668a; callee frame a1,4 — slot load below)
        || pc == 0x40110d2c // osi_sem_free (7d1e668a; take-family free path marker)
        || pc == 0x40093798
        || pc == 0x40093694
        || pc == 0x4009351c
        || pc == 0x40107718
        || pc == 0x40107750
        || pc == 0x4010cf0c
        || pc == 0x4010d5f8
        || pc == 0x40109bec
        || pc == 0x40117e30
        || pc == 0x40117018
        || pc == 0x40117034
        || pc == 0x401067c4
        || pc == 0x40108981
        || pc == 0x401089d2
        || pc == 0x40106870 || pc == 0x4010bad8 || pc == 0x4010baec // btc_transfer_context
        || pc == 0x4010c6f4 // btc_transfer_context (7d1e668a; scanner HOOK_BTRANSFER is authoritative — FENT literal is backup)
        || pc == 0x4010692c || pc == 0x4010bb94 || pc == 0x4010bba8 // btc_init
        || pc == 0x40107e58 || pc == 0x4010f2a8 || pc == 0x4010f2d4 // future_new
        || pc == 0x4010ff34 // future_new (7d1e668a; HOOK_NEW authoritative — FENT literal is backup)
        || pc == 0x40107e8c || pc == 0x4010f2dc || pc == 0x4010f308 // future_await
        || pc == 0x4010ff68 // future_await (7d1e668a; HOOK_AWAIT_EB2 authoritative — FENT literal is backup)
        || pc == 0x4010886c || pc == 0x4010fcbc || pc == 0x4010fce8 // osi_thread_create
        || pc == 0x400e2af0 || pc == 0x400e2b5c || pc == 0x400e2bd8 // esp_ble_gatts_app_register/create/start_service (7d1e668a)
        || pc == 0x40110d00 // osi_sem_take (7d1e668a; HOOK_TAKE authoritative — FENT literal is backup)
        || pc == 0x400ee740 || pc == 0x400f3814 || pc == 0x400f3828 // btu_init_core
        || pc == 0x400eea7c || pc == 0x400f3b50 || pc == 0x400f3b64 // btu_task_start_up
        || pc == 0x400eeaf8 || pc == 0x400f3bcc || pc == 0x400f3be0 // btu_task_post
        || pc == 0x40161af4 || pc == 0x40161cf4 || pc == 0x40161d3c // btdm_controller_task
        || pc == 0x401619dc || pc == 0x40161bdc || pc == 0x40161c24 // r_rw_schedule
        || pc == 0x40177970 || pc == 0x40177b70 || pc == 0x40177bb8 // btdm_task_post
        || pc == 0x40177d30 || pc == 0x40177f30 || pc == 0x40177f78 // API_vhci_host_send_packet
        || pc == 0x40162008 || pc == 0x40162208 || pc == 0x40162250 // btdm_controller_enable
        || pc == 0x400e284c || pc == 0x400e285c || pc == 0x400e2870 // esp_bt_controller_enable
        || pc == 0x40095eb4
    {
        static mut FENT_N: u32 = 8000;
        unsafe {
            if FENT_N != 0 {
                FENT_N -= 1;
                let mut db = [0u8; 96];
                let hx = |mut v: u32| -> [u8; 8] {
                    let mut o = [0u8; 8];
                    for i in (0..8).rev() {
                        o[i] = b"0123456789abcdef"[(v & 0xF) as usize];
                        v >>= 4;
                    }
                    o
                };
                let mut n = 0;
                for &b in b"[FENT] c" { db[n] = b; n += 1; }
                db[n] = b'0' + core.index as u8;
                n += 1;
                for &b in b" " { db[n] = b; n += 1; }
                for &b in &hx(pc) { db[n] = b; n += 1; }
                for &b in b" a0=" { db[n] = b; n += 1; }
                for &b in &hx(core.ar(0)) { db[n] = b; n += 1; }
                // run212: a2 = first arg (queue ptr for give/send/take,
                // future* for ready) — the testsuite ROADMAP: which queue
                // does the give actually post to?
                for &b in b" a2=" { db[n] = b; n += 1; }
                for &b in &hx(core.ar(2)) { db[n] = b; n += 1; }
                crate::js_log_str(db.as_ptr() as u32, n as u32);
            }
        }
    }
    // run180k: DECTOT-armed decode trace (RETIRED run184 — one-shot CBDEC
    // arming block removed; the episodic sampler above replaces it. Kept
    // compiled-out for manual use via native_set_decode_trace).
    static mut DECTOT: u32 = 0;
    if false && unsafe { DECTOT } != 0
    {
        unsafe { DECTOT -= 1; }
        unsafe { DEFER_PC = pc; }
        {
            let op16 = read_opcode_u16(core, pc);
            let w = INST_WIDTH_TABLE[(op16 & 15) as usize];
            let op = if w == 4 { read_opcode_u32(core, pc) } else if w == 3 { op16 | (read_opcode_u8(core, pc + 2) << 16) } else { op16 };
            let mut db = [0u8; 320];
            let hx = |mut v: u32| -> [u8; 8] {
                let mut o = [0u8; 8];
                for i in (0..8).rev() { o[i] = b"0123456789abcdef"[(v & 0xF) as usize]; v >>= 4; }
                o
            };
            let mut n = 0;
            for &b in b"[DEC] c" { db[n] = b; n += 1; }
            db[n] = b'0' + core.index as u8; n += 1;
            for &b in b" " { db[n] = b; n += 1; }
            for &b in &hx(pc) { db[n] = b; n += 1; }
            for &b in b" op=" { db[n] = b; n += 1; }
            for &b in &hx(op) { db[n] = b; n += 1; }
            for &b in b" w=" { db[n] = b; n += 1; }
            db[n] = b'0' + w as u8; n += 1;
            for &b in b" nxt=" { db[n] = b; n += 1; }
            for &b in &hx(core.pc.wrapping_add(w as u32)) { db[n] = b; n += 1; }
            for &b in b" a0=" { db[n] = b; n += 1; }
            for &b in &hx(core.ar(0)) { db[n] = b; n += 1; }
            for &b in b" a1=" { db[n] = b; n += 1; }
            for &b in &hx(core.ar(1)) { db[n] = b; n += 1; }
            for &b in b" a2=" { db[n] = b; n += 1; }
            for &b in &hx(core.ar(2)) { db[n] = b; n += 1; }
            for &b in b" a3=" { db[n] = b; n += 1; }
            for &b in &hx(core.ar(3)) { db[n] = b; n += 1; }
            for &b in b" a4=" { db[n] = b; n += 1; }
            for &b in &hx(core.ar(4)) { db[n] = b; n += 1; }
            for &b in b" a5=" { db[n] = b; n += 1; }
            for &b in &hx(core.ar(5)) { db[n] = b; n += 1; }
            for &b in b" a6=" { db[n] = b; n += 1; }
            for &b in &hx(core.ar(6)) { db[n] = b; n += 1; }
            for &b in b" a7=" { db[n] = b; n += 1; }
            for &b in &hx(core.ar(7)) { db[n] = b; n += 1; }
            for &b in b" a8=" { db[n] = b; n += 1; }
            for &b in &hx(core.ar(8)) { db[n] = b; n += 1; }
            for &b in b" a9=" { db[n] = b; n += 1; }
            for &b in &hx(core.ar(9)) { db[n] = b; n += 1; }
            for &b in b" a10=" { db[n] = b; n += 1; }
            for &b in &hx(core.ar(10)) { db[n] = b; n += 1; }
            for &b in b" a11=" { db[n] = b; n += 1; }
            for &b in &hx(core.ar(11)) { db[n] = b; n += 1; }
            for &b in b" a12=" { db[n] = b; n += 1; }
            for &b in &hx(core.ar(12)) { db[n] = b; n += 1; }
            for &b in b" a13=" { db[n] = b; n += 1; }
            for &b in &hx(core.ar(13)) { db[n] = b; n += 1; }
            for &b in b" a14=" { db[n] = b; n += 1; }
            for &b in &hx(core.ar(14)) { db[n] = b; n += 1; }
            for &b in b" a15=" { db[n] = b; n += 1; }
            for &b in &hx(core.ar(15)) { db[n] = b; n += 1; }
            unsafe { crate::js_log_str(db.as_ptr() as u32, n as u32); }
        }
    }

    let mut opcode = read_opcode_u16(core, pc);
    let width = INST_WIDTH_TABLE[(opcode & 15) as usize];
    let is_wide = 4 == width;

    if is_wide {
        opcode = read_opcode_u32(core, pc);
    } else if 3 == width {
        let extra_byte = read_opcode_u8(core, pc + 2);
        opcode |= (extra_byte as u32) << 16;
    }

    core.last_opcode = opcode;
    core.debug_opcode = opcode;
    core.next_pc = core.pc + width;

    // WAITI (0x1DF03x) — wait for interrupt with level x: hold PC and enter idle.
    // The core wakes via update_interrupts() when a pending interrupt matches PS_INTLEVEL.
    if (opcode & 0xFF) == 0x1D && ((opcode >> 8) & 0xFF) == 0xF0 && ((opcode >> 16) & 0xF0) == 0x30 {
        core.next_pc = core.pc;
        core.idle = 1;
        core.pc = core.next_pc;
        return 0;
    }

    // run151/run152 REVERTED (run153): the engine ALREADY emulates the CAS
    // pair coherently WITHOUT any intercept — c_handler7 routes S32C1I
    // (0x00E002) to _handler18 (conditional store with SAR as comparand)
    // and WSR.SCOMPARE1 (sr=12) via c_handler2 into spec[12]==SAR. The two
    // halves agree (SAR-as-shadow) and must stay together. My run151
    // shadow-static split them (_handler18 still compared SAR while WSR
    // wrote the static) and broke coherence -> early-boot assert. The
    // remaining c1 CAS-loop question is NOT missing emulation but why the
    // coherent _handler18 store never succeeds (window_check? trap?).
    // KEPT: RSIL only (PROVEN SAFE: boot reaches BDINIT_CALL; RSIL has no
    // table route — without it rsil decodes as ILLEGAL and every critical
    // section wedges).
    // RSIL (read-and-set-interrupt-level): at=(op>>4)&F gets old INTLEVEL,
    // PS.INTLEVEL=level=(op>>8)&F. Encoding: word & 0xFFFF0F == 0x0060C0.
    // REVERTED run166: the gate `if false` disabled RSIL vs the tables and
    // early boot hung in a loop (run166: 41 lines, LOOP pc=4000fca9). RSIL
    // has no table route (falls to ILLEGAL), so every critical section
    // wedges without this intercept. Always on.
    if width == 3 {
        if (opcode & 0xFFFF0F) == 0x0060C0 {
            let level = ((opcode >> 8) & 0xF) as u32;
            let at = ((opcode >> 4) & 0xF) as u32;
            core.set_ar(at, core.ps_intlevel());
            core.set_ps_intlevel(level);
            core.pc = core.next_pc;
            return 0;
        }
    }

    if core.special_registers[LOOP_COUNT] != 0
        && core.next_pc == core.special_registers[LOOP_END]
    {
        core.special_registers[LOOP_COUNT] = core.special_registers[LOOP_COUNT].wrapping_sub(1);
        core.next_pc = core.special_registers[LOOP_BEGIN];
    }

    if is_wide {
        if core.pie_enabled != 0 {
            decode_pie0(core, opcode);
        } else {
            unsafe { on_unknown_inst(core.index, core.pc, opcode); }
            core.exception(TRAP_ILLEGAL_INSTRUCTION);
        }
    } else {
        if core.pie_enabled != 0 {
            let lo = opcode & 15;
            let hi = (opcode >> 16) & 255;
            if (4 == lo || (0 == lo && (22 == hi || 23 == hi))) && decode_pie31(core, opcode) != 0 {
                core.pc = core.next_pc;
                return 0;
            }
        }
        c_handler7(core, opcode);
    }

    if unsafe { core.pc == DEFER_PC } {
        unsafe { DEFER_PC = 0; }
        let mut db = [0u8; 320];
        let hx = |mut v: u32| -> [u8; 8] {
            let mut o = [0u8; 8];
            for i in (0..8).rev() { o[i] = b"0123456789abcdef"[(v & 0xF) as usize]; v >>= 4; }
            o
        };
        let mut n = 0;
        for &b in b"[DAF] c" { db[n] = b; n += 1; }
        db[n] = b'0' + core.index as u8; n += 1;
        for &b in b" " { db[n] = b; n += 1; }
        for &b in &hx(core.pc) { db[n] = b; n += 1; }
        for &b in b" a0=" { db[n] = b; n += 1; }
        for &b in &hx(core.ar(0)) { db[n] = b; n += 1; }
        for &b in b" a1=" { db[n] = b; n += 1; }
        for &b in &hx(core.ar(1)) { db[n] = b; n += 1; }
        for &b in b" a2=" { db[n] = b; n += 1; }
        for &b in &hx(core.ar(2)) { db[n] = b; n += 1; }
        for &b in b" a3=" { db[n] = b; n += 1; }
        for &b in &hx(core.ar(3)) { db[n] = b; n += 1; }
        for &b in b" a4=" { db[n] = b; n += 1; }
        for &b in &hx(core.ar(4)) { db[n] = b; n += 1; }
        for &b in b" a5=" { db[n] = b; n += 1; }
        for &b in &hx(core.ar(5)) { db[n] = b; n += 1; }
        for &b in b" a6=" { db[n] = b; n += 1; }
        for &b in &hx(core.ar(6)) { db[n] = b; n += 1; }
        for &b in b" a7=" { db[n] = b; n += 1; }
        for &b in &hx(core.ar(7)) { db[n] = b; n += 1; }
        for &b in b" a8=" { db[n] = b; n += 1; }
        for &b in &hx(core.ar(8)) { db[n] = b; n += 1; }
        for &b in b" a9=" { db[n] = b; n += 1; }
        for &b in &hx(core.ar(9)) { db[n] = b; n += 1; }
        for &b in b" a10=" { db[n] = b; n += 1; }
        for &b in &hx(core.ar(10)) { db[n] = b; n += 1; }
        for &b in b" a11=" { db[n] = b; n += 1; }
        for &b in &hx(core.ar(11)) { db[n] = b; n += 1; }
        for &b in b" a12=" { db[n] = b; n += 1; }
        for &b in &hx(core.ar(12)) { db[n] = b; n += 1; }
        for &b in b" a13=" { db[n] = b; n += 1; }
        for &b in &hx(core.ar(13)) { db[n] = b; n += 1; }
        for &b in b" a14=" { db[n] = b; n += 1; }
        for &b in &hx(core.ar(14)) { db[n] = b; n += 1; }
        for &b in b" a15=" { db[n] = b; n += 1; }
        for &b in &hx(core.ar(15)) { db[n] = b; n += 1; }
        unsafe { crate::js_log_str(db.as_ptr() as u32, n as u32); }
    }

    core.pc = core.next_pc;
    check_ccompare(core);
    0
}

pub(crate) fn read_uint8(core: &mut CoreState, addr: u32) -> u32 {
    let val = read_page_table(core, addr, 8);
    val
}

pub(crate) fn read_uint16(core: &mut CoreState, addr: u32) -> u32 {
    read_page_table(core, addr, 16)
}

pub(crate) fn read_uint32(core: &mut CoreState, addr: u32) -> u32 {
    let val = read_page_table(core, addr, 32);
    val
}

pub(crate) fn write_uint8(core: &mut CoreState, addr: u32, val: u32, trap_check: u32) {
    if trap_check != 0 && 0 == addr {
        write_special_register(core, MISC_CONFIG as u32, addr);
        core.exception(TRAP_STORE_PROHIBITED);
        return;
    }
    write_with_traps(core, addr, val, 8);
    write_page_table(core, addr, val, 8);
}

pub(crate) fn write_uint16(core: &mut CoreState, addr: u32, val: u32, trap_check: u32) {
    if trap_check != 0 && 0 == addr {
        write_special_register(core, MISC_CONFIG as u32, addr);
        core.exception(TRAP_STORE_PROHIBITED);
        return;
    }
    write_with_traps(core, addr, val, 16);
    write_page_table(core, addr, val, 16);
}

pub(crate) fn write_uint32(core: &mut CoreState, addr: u32, val: u32, trap_check: u32) -> u32 {
    if trap_check != 0 && 0 == addr {
        write_special_register(core, MISC_CONFIG as u32, addr);
        core.exception(TRAP_STORE_PROHIBITED);
        return 0;
    }
    write_with_traps(core, addr, val, 32);
    write_page_table(core, addr, val, 32);
    1
}

fn write_with_traps(core: &mut CoreState, addr: u32, val: u32, size: u32) {
    unsafe {
        if write_watchpoint(addr, core.index) != 0 {
        }
        // Rust-side watchpoints (diag): log pc+val for armed addresses.
        // Armed via native_watchpoint_add; cleared via native_watchpoint_clear.
        // Used to catch wild writers (e.g. the BT list-corruption hunt).
        for i in 0..8 {
            if WP_ADDRS[i] == addr {
                let mut db = [0u8; 96];
                let hx = |mut v: u32| -> [u8; 8] {
                    let mut o = [0u8; 8];
                    for i in (0..8).rev() { o[i] = b"0123456789abcdef"[(v & 0xF) as usize]; v >>= 4; }
                    o
                };
                let mut n = 0;
                for &b in b"[WP] pc=" { db[n] = b; n += 1; }
                for &b in &hx(core.pc) { db[n] = b; n += 1; }
                for &b in b" core=" { db[n] = b; n += 1; }
                db[n] = b'0' + (core.index as u8); n += 1;
                for &b in b" a=" { db[n] = b; n += 1; }
                for &b in &hx(addr) { db[n] = b; n += 1; }
                for &b in b" v=" { db[n] = b; n += 1; }
                for &b in &hx(val) { db[n] = b; n += 1; }
                for &b in b" sz=" { db[n] = b; n += 1; }
                db[n] = b'0' + (size as u8); n += 1;
                crate::js_log_str(db.as_ptr() as u32, n as u32);
                break;
            }
        }
        // Gated in Rust — the JS ESPTrace sink is a no-op unless enabled
        // (host-side debug tool; default off).
        if TRACE_MEM_WRITES { trace_mem_write(core.index, core.pc, addr, val, size); }
    }
}

static mut WP_ADDRS: [u32; 8] = [0; 8];

#[no_mangle]
pub extern "C" fn native_watchpoint_add(addr: u32) {
    unsafe {
        for i in 0..8 {
            if WP_ADDRS[i] == 0 {
                WP_ADDRS[i] = addr;
                return;
            }
        }
    }
}

#[no_mangle]
pub extern "C" fn native_watchpoint_clear() {
    unsafe {
        WP_ADDRS = [0; 8];
    }
}

pub(crate) fn read_special_register(core: &CoreState, reg: u32) -> u32 {
    match reg as usize {
        CCOUNT_REG => core.ccount(),
        WINDOW_START => {
            let v = core.special_registers[WINDOW_START];
            (((v & 255) << 24) as i32 >> 24) as u32
        }
        _ => core.special_registers[reg as usize],
    }
}

pub(crate) fn write_special_register(core: &mut CoreState, reg: u32, val: u32) {
    match reg as usize {
        INT_SET => {
            core.special_registers[INT_SET] = val;
            core.pending_interrupts = 1;
        }
        CCOMPARE0_REG => {
            ccompare_update(core, val, CCOMPARE0_INT_BIT, 0);
        }
        CCOMPARE1_REG => {
            ccompare_update(core, val, CCOMPARE1_INT_BIT, 1);
        }
        CCOMPARE2_REG => {
            ccompare_update(core, val, CCOMPARE2_INT_BIT, 2);
        }
        INT_ENABLE => {
            // JS parity (xtensa-core.js writeSpecialRegister): firmware WSR
            // to INTENABLE can only set self-managed bits (IntEnableMaskBase
            // = 0x20000080: CCOMPARE0/1/2 + NMI). Matrix-owned lines (25 =
            // RWBT/RWBLE, held by DPORT int_set_clear) must NOT be ORed in —
            // the old blanket OR let a firmware WSR re-assert IE25 behind
            // the matrix's back, defeating the take_interrupt edge-ack and
            // feeding the run176o level storm (1.1M+ vectors). The CLOCK_CONFIG
            // mask write keeps legacy boot behavior (ROM UART hangs without
            // it); pending slots still go through int_set_clear only.
            // NOTE: our own BT unmask (bt_shim_step, matrix-authorized) sets
            // INT_ENABLE/CLOCK_CONFIG directly, bypassing this gate.
            core.special_registers[CLOCK_CONFIG] |= val;
            core.special_registers[INT_ENABLE] |= val & INT_ENABLE_MASK_BASE;
            core.pending_interrupts = 1;
            return;
        }
        INT_CLEAR => {
            // HW-exact: INTCLEAR clears any specified pending bits.
            core.special_registers[INT_ENABLE] &= !val;
            return;
        }
        _ => {}
    }
    core.special_registers[reg as usize] = val;
}

fn check_ccompare(_core: &mut CoreState) {
    // No-op: the JS clock event (via ccompare_schedule callback) handles
    // CCOMPARE interrupts. The native dual mechanism caused timing
    // mismatches (run176: timer reads 25/107 instead of ~2000/~12000 —
    // early CCOMPARE wake compressed delay()).
}

fn ccompare_update(core: &mut CoreState, val: u32, int_bit: u32, which: u32) {
    core.special_registers[INT_ENABLE] &= !(1 << int_bit);
    let ccount = core.ccount();
    let sim_clock = if val == ccount { 0xffffffff } else { val.wrapping_sub(ccount) };
    unsafe { ccompare_schedule(core.index, which, sim_clock); }
}
