use super::constants::*;
use super::state::CoreState;
use super::exports::{read_uint8, read_uint16, read_uint32, write_uint8, write_uint16, write_uint32, write_special_register, read_special_register};

/// Resolved-instruction function pointer (50mips decode cache).
pub type DecodeFn = fn(&mut CoreState, u32);

// Helper: sign-extend utilities (nHandler0-4)
fn n_handler0(cpu_val: u32) -> u32 { cpu_val | if (cpu_val & 8) != 0 { 0xfffffff0 } else { 0 } }
fn n_handler1(cpu_val: u32) -> u32 { cpu_val | if (cpu_val & 128) != 0 { 0xffffff00 } else { 0 } }
fn n_handler2(cpu_val: u32) -> u32 { cpu_val | if (cpu_val & 2048) != 0 { 0xfffff000 } else { 0 } }
fn n_handler3(cpu_val: u32) -> u32 { cpu_val | if (cpu_val & 32768) != 0 { 0xffff0000 } else { 0 } }
fn n_handler4(cpu_val: u32) -> u32 { cpu_val | if (cpu_val & 131072) != 0 { 0xfffc0000 } else { 0 } }

const NF_SHIFT_TABLE: [i32; 16] = [-1, 1, 2, 3, 4, 5, 6, 7, 8, 10, 12, 16, 32, 64, 128, 256];
const NF_TABLE2: [u32; 16] = [32768, 65536, 2, 3, 4, 5, 6, 7, 8, 10, 12, 16, 32, 64, 128, 256];

fn f32_from_bits(bits: u32) -> f32 { f32::from_bits(bits) }
fn f32_to_bits(val: f32) -> u32 { val.to_bits() }

fn f32_powi(f: f32, n: i32) -> f32 {
    if n == 0 { return 1.0; }
    let mut result = 1.0f32;
    if n > 0 {
        for _ in 0..n { result *= f; }
    } else {
        for _ in 0..(-n) { result /= f; }
    }
    result
}
fn f32_sqrt(f: f32) -> f32 {
    if f < 0.0 { return f32::from_bits(0x7fc00000); } // NaN
    if f == 0.0 { return f; }
    let mut x = f;
    for _ in 0..12 { x = (x + f / x) * 0.5; }
    x
}
fn f32_ceil(f: f32) -> f32 {
    let bits = f.to_bits();
    let exp = ((bits >> 23) & 0xff) as i32 - 127;
    if exp >= 23 { return f; }
    if exp < 0 { return if (bits >> 31) == 0 { 1.0 } else { -0.0 }; }
    let frac_mask = (1u32 << (23 - exp)) - 1;
    if (bits & frac_mask) == 0 { return f; }
    f32::from_bits((bits & !frac_mask) + (1 << (23 - exp)))
}
fn f32_floor(f: f32) -> f32 {
    let bits = f.to_bits();
    let exp = ((bits >> 23) & 0xff) as i32 - 127;
    if exp >= 23 { return f; }
    if exp < 0 { return if (bits >> 31) == 0 { 0.0 } else { -1.0 }; }
    let frac_mask = (1u32 << (23 - exp)) - 1;
    if (bits & frac_mask) == 0 { return f; }
    if (bits >> 31) != 0 {
        f32::from_bits((bits & !frac_mask) + (1 << (23 - exp)))
    } else {
        f32::from_bits(bits & !frac_mask)
    }
}
fn f32_round(f: f32) -> f32 {
    let int_part = f32_floor(f);
    let frac = f - int_part;
    if frac >= 0.5 { int_part + 1.0 } else if frac <= -0.5 { int_part - 1.0 } else { int_part }
}
fn f32_trunc(f: f32) -> f32 {
    if f >= 0.0 { f32_floor(f) } else { f32_ceil(f) }
}



pub fn n_handler5(core: &mut CoreState, tmp_val: u32) {
    let idx_val = (tmp_val >> 12) & 15;
    let clock_event = (tmp_val >> 4) & 15;
    if core.window_check(idx_val >> 2, 0, clock_event >> 2) { return; }
    let sim = core.ar(clock_event) as i32;
    core.set_ar(idx_val, sim.unsigned_abs());
}
pub fn n_handler6(core: &mut CoreState, tmp_val: u32) {
    let idx_val = (tmp_val >> 12) & 15;
    let clock_event = (tmp_val >> 8) & 15;
    core.float_registers[idx_val as usize] = f32_to_bits(f32_from_bits(core.float_registers[clock_event as usize]).abs());
}
pub fn n_handler7(core: &mut CoreState, tmp_val: u32) {
    let idx_val = (tmp_val >> 12) & 15;
    let clock_event = (tmp_val >> 8) & 15;
    let sim = (tmp_val >> 4) & 15;
    if !core.window_check(idx_val >> 2, clock_event >> 2, sim >> 2) {
        core.set_ar(idx_val, core.ar(clock_event).wrapping_add(core.ar(sim)));
    }
}
pub fn n_handler8(core: &mut CoreState, tmp_val: u32) {
    let idx_val = (tmp_val >> 12) & 15;
    let clock_event = (tmp_val >> 8) & 15;
    core.float_registers[idx_val as usize] = core.float_registers[clock_event as usize];
}
pub fn n_handler9(core: &mut CoreState, tmp_val: u32) {
    let idx_val = (tmp_val >> 12) & 15;
    let clock_event = (tmp_val >> 8) & 15;
    let sim = (tmp_val >> 4) & 15;
    if !core.window_check(idx_val >> 2, clock_event >> 2, sim >> 2) {
        core.set_ar(idx_val, core.ar(clock_event).wrapping_add(core.ar(sim)));
    }
}
pub fn n_handler10(core: &mut CoreState, tmp_val: u32) {
    let idx_val = (tmp_val >> 12) & 15;
    let clock_event = (tmp_val >> 8) & 15;
    let sim = (tmp_val >> 4) & 15;
    core.float_registers[idx_val as usize] = f32_to_bits(
        f32_from_bits(core.float_registers[clock_event as usize]) +
        f32_from_bits(core.float_registers[sim as usize])
    );
}
pub fn n_handler11(core: &mut CoreState, tmp_val: u32) {
    let idx_val = (tmp_val >> 16) & 255;
    let clock_event = (tmp_val >> 8) & 15;
    let sim = (tmp_val >> 4) & 15;
    if !core.window_check(0, clock_event >> 2, sim >> 2) {
        core.set_ar(sim, core.ar(clock_event).wrapping_add(n_handler1(idx_val)));
    }
}
pub fn n_handler12(core: &mut CoreState, tmp_val: u32) {
    let idx_val = (tmp_val >> 12) & 15;
    let clock_event = (tmp_val >> 8) & 15;
    let sim = (tmp_val >> 4) & 15;
    if !core.window_check(idx_val >> 2, clock_event >> 2, 0) {
        core.set_ar(idx_val, core.ar(clock_event).wrapping_add(if sim != 0 { sim } else { 0xffffffff }));
    }
}
pub fn n_handler13(core: &mut CoreState, tmp_val: u32) {
    let idx_val = (tmp_val >> 16) & 255;
    let clock_event = (tmp_val >> 8) & 15;
    let sim = (tmp_val >> 4) & 15;
    if !core.window_check(0, clock_event >> 2, sim >> 2) {
        core.set_ar(sim, core.ar(clock_event).wrapping_add(n_handler1(idx_val) << 8));
    }
}
pub fn n_handler14(core: &mut CoreState, tmp_val: u32) {
    let idx_val = (tmp_val >> 12) & 15;
    let clock_event = (tmp_val >> 8) & 15;
    let sim = (tmp_val >> 4) & 15;
    if !core.window_check(idx_val >> 2, clock_event >> 2, sim >> 2) {
        core.set_ar(idx_val, (core.ar(clock_event) << 1).wrapping_add(core.ar(sim)));
    }
}
pub fn n_handler15(core: &mut CoreState, tmp_val: u32) {
    let idx_val = (tmp_val >> 12) & 15;
    let clock_event = (tmp_val >> 8) & 15;
    let sim = (tmp_val >> 4) & 15;
    if !core.window_check(idx_val >> 2, clock_event >> 2, sim >> 2) {
        core.set_ar(idx_val, (core.ar(clock_event) << 2).wrapping_add(core.ar(sim)));
    }
}
pub fn n_handler16(core: &mut CoreState, tmp_val: u32) {
    let idx_val = (tmp_val >> 12) & 15;
    let clock_event = (tmp_val >> 8) & 15;
    let sim = (tmp_val >> 4) & 15;
    if !core.window_check(idx_val >> 2, clock_event >> 2, sim >> 2) {
        core.set_ar(idx_val, (core.ar(clock_event) << 3).wrapping_add(core.ar(sim)));
    }
}
pub fn n_handler17(core: &mut CoreState, tmp_val: u32) {
    let idx_val = (tmp_val >> 8) & 15;
    let clock_event = (tmp_val >> 4) & 15;
    core.set_br(clock_event,
        core.br(idx_val + 3) && core.br(idx_val + 2) && core.br(idx_val + 1) && core.br(idx_val));
}
pub fn n_handler18(core: &mut CoreState, tmp_val: u32) {
    let idx_val = (tmp_val >> 8) & 15;
    let clock_event = (tmp_val >> 4) & 15;
    core.set_br(clock_event,
        core.br(idx_val + 7) && core.br(idx_val + 6) && core.br(idx_val + 5) && core.br(idx_val + 4)
        && core.br(idx_val + 3) && core.br(idx_val + 2) && core.br(idx_val + 1) && core.br(idx_val));
}
pub fn n_handler19(core: &mut CoreState, tmp_val: u32) {
    let idx_val = (tmp_val >> 12) & 15;
    let clock_event = (tmp_val >> 8) & 15;
    let sim = (tmp_val >> 4) & 15;
    if !core.window_check(idx_val >> 2, clock_event >> 2, sim >> 2) {
        core.set_ar(idx_val, core.ar(clock_event) & core.ar(sim));
    }
}
pub fn n_handler20(core: &mut CoreState, tmp_val: u32) {
    let idx_val = (tmp_val >> 12) & 15;
    let clock_event = (tmp_val >> 8) & 15;
    let sim = (tmp_val >> 4) & 15;
    core.set_br(idx_val, core.br(clock_event) && core.br(sim));
}
pub fn n_handler21(core: &mut CoreState, tmp_val: u32) {
    let idx_val = (tmp_val >> 12) & 15;
    let clock_event = (tmp_val >> 8) & 15;
    let sim = (tmp_val >> 4) & 15;
    core.set_br(idx_val, core.br(clock_event) && !core.br(sim));
}
pub fn n_handler22(core: &mut CoreState, tmp_val: u32) {
    let idx_val = (tmp_val >> 8) & 15;
    let clock_event = (tmp_val >> 4) & 15;
    core.set_br(clock_event,
        core.br(idx_val + 3) || core.br(idx_val + 2) || core.br(idx_val + 1) || core.br(idx_val));
}
pub fn n_handler23(core: &mut CoreState, tmp_val: u32) {
    let idx_val = (tmp_val >> 8) & 15;
    let clock_event = (tmp_val >> 4) & 15;
    core.set_br(clock_event,
        core.br(idx_val + 7) || core.br(idx_val + 6) || core.br(idx_val + 5) || core.br(idx_val + 4)
        || core.br(idx_val + 3) || core.br(idx_val + 2) || core.br(idx_val + 1) || core.br(idx_val));
}
pub fn n_handler24(core: &mut CoreState, tmp_val: u32) {
    let idx_val = (tmp_val >> 16) & 255;
    let clock_event = (tmp_val >> 8) & 15;
    let sim = (tmp_val >> 4) & 15;
    if !core.window_check(0, clock_event >> 2, sim >> 2) {
        if (!core.ar(clock_event) & core.ar(sim)) == 0 {
            core.next_pc = core.pc.wrapping_add(n_handler1(idx_val)).wrapping_add(4);
        }
    }
}
pub fn n_handler25(core: &mut CoreState, tmp_val: u32) {
    let idx_val = (tmp_val >> 16) & 255;
    let clock_event = (tmp_val >> 8) & 15;
    let sim = (tmp_val >> 4) & 15;
    if !core.window_check(0, clock_event >> 2, sim >> 2) {
        if (core.ar(clock_event) & core.ar(sim)) != 0 {
            core.next_pc = core.pc.wrapping_add(n_handler1(idx_val)).wrapping_add(4);
        }
    }
}
pub fn n_handler26(core: &mut CoreState, tmp_val: u32) {
    let idx_val = (tmp_val >> 16) & 255;
    let clock_event = (tmp_val >> 8) & 15;
    let sim = (tmp_val >> 4) & 15;
    if core.window_check(0, clock_event >> 2, sim >> 2) { return; }
    let rr = 31 & core.ar(sim);
    if (core.ar(clock_event) & (1 << rr)) == 0 {
        core.next_pc = core.pc.wrapping_add(n_handler1(idx_val)).wrapping_add(4);
    }
}
pub fn n_handler27(core: &mut CoreState, tmp_val: u32) {
    let idx_val = (tmp_val >> 16) & 255;
    let clock_event = (tmp_val >> 12) & 1;
    let sim = (tmp_val >> 8) & 15;
    let rr = (clock_event << 4) | ((tmp_val >> 4) & 15);
    if !core.window_check(0, sim >> 2, 0) {
        if (core.ar(sim) & (1 << rr)) == 0 {
            core.next_pc = core.pc.wrapping_add(n_handler1(idx_val)).wrapping_add(4);
        }
    }
}
pub fn n_handler28(core: &mut CoreState, tmp_val: u32) {
    let idx_val = (tmp_val >> 16) & 255;
    let clock_event = (tmp_val >> 8) & 15;
    let sim = (tmp_val >> 4) & 15;
    if core.window_check(0, clock_event >> 2, sim >> 2) { return; }
    let rr = 31 & core.ar(sim);
    if (core.ar(clock_event) & (1 << rr)) != 0 {
        core.next_pc = core.pc.wrapping_add(n_handler1(idx_val)).wrapping_add(4);
    }
}
pub fn n_handler29(core: &mut CoreState, tmp_val: u32) {
    let idx_val = (tmp_val >> 16) & 255;
    let clock_event = (tmp_val >> 12) & 1;
    let sim = (tmp_val >> 8) & 15;
    let rr = (clock_event << 4) | ((tmp_val >> 4) & 15);
    if !core.window_check(0, sim >> 2, 0) {
        if (core.ar(sim) & (1 << rr)) != 0 {
            core.next_pc = core.pc.wrapping_add(n_handler1(idx_val)).wrapping_add(4);
        }
    }
}
pub fn n_handler30(core: &mut CoreState, tmp_val: u32) {
    let idx_val = (tmp_val >> 16) & 255;
    let clock_event = (tmp_val >> 8) & 15;
    let sim = (tmp_val >> 4) & 15;
    if !core.window_check(0, clock_event >> 2, sim >> 2) {
        if core.ar(clock_event) == core.ar(sim) {
            core.next_pc = core.pc.wrapping_add(n_handler1(idx_val)).wrapping_add(4);
        }
    }
}
pub fn n_handler31(core: &mut CoreState, tmp_val: u32) {
    let idx_val = (tmp_val >> 16) & 255;
    let clock_event = (tmp_val >> 12) & 15;
    let sim = (tmp_val >> 8) & 15;
    if !core.window_check(0, sim >> 2, 0) {
        if (core.ar(sim) as i32) == NF_SHIFT_TABLE[clock_event as usize] {
            core.next_pc = core.pc.wrapping_add(n_handler1(idx_val)).wrapping_add(4);
        }
    }
}
pub fn n_handler32(core: &mut CoreState, tmp_val: u32) {
    let idx_val = (tmp_val >> 12) & 4095;
    let clock_event = (tmp_val >> 8) & 15;
    if !core.window_check(0, clock_event >> 2, 0) {
        if core.ar(clock_event) == 0 {
            core.next_pc = core.pc.wrapping_add(n_handler2(idx_val)).wrapping_add(4);
        }
    }
}
pub fn n_handler33(core: &mut CoreState, tmp_val: u32) {
    let idx_val = (tmp_val >> 12) & 15;
    let clock_event = (tmp_val >> 8) & 15;
    let sim = (((tmp_val >> 4) & 3) << 4) | idx_val;
    if !core.window_check(0, clock_event >> 2, 0) {
        if core.ar(clock_event) == 0 {
            core.next_pc = core.pc.wrapping_add(sim).wrapping_add(4);
        }
    }
}
pub fn n_handler34(core: &mut CoreState, tmp_val: u32) {
    let idx_val = (tmp_val >> 16) & 255;
    let clock_event = (tmp_val >> 8) & 15;
    if !core.br(clock_event) {
        core.next_pc = core.pc.wrapping_add(n_handler1(idx_val)).wrapping_add(4);
    }
}
pub fn n_handler35(core: &mut CoreState, tmp_val: u32) {
    let idx_val = (tmp_val >> 16) & 255;
    let clock_event = (tmp_val >> 8) & 15;
    let sim = (tmp_val >> 4) & 15;
    if !core.window_check(0, clock_event >> 2, sim >> 2) {
        if (core.ar(clock_event) as i32) >= (core.ar(sim) as i32) {
            core.next_pc = core.pc.wrapping_add(n_handler1(idx_val)).wrapping_add(4);
        }
    }
}
pub fn n_handler36(core: &mut CoreState, tmp_val: u32) {
    let idx_val = (tmp_val >> 16) & 255;
    let clock_event = (tmp_val >> 12) & 15;
    let sim = (tmp_val >> 8) & 15;
    if !core.window_check(0, sim >> 2, 0) {
        if (core.ar(sim) as i32) >= NF_SHIFT_TABLE[clock_event as usize] {
            core.next_pc = core.pc.wrapping_add(n_handler1(idx_val)).wrapping_add(4);
        }
    }
}
pub fn n_handler37(core: &mut CoreState, tmp_val: u32) {
    let idx_val = (tmp_val >> 16) & 255;
    let clock_event = (tmp_val >> 8) & 15;
    let sim = (tmp_val >> 4) & 15;
    if !core.window_check(0, clock_event >> 2, sim >> 2) {
        if core.ar(clock_event) >= core.ar(sim) {
            core.next_pc = core.pc.wrapping_add(n_handler1(idx_val)).wrapping_add(4);
        }
    }
}
pub fn n_handler38(core: &mut CoreState, tmp_val: u32) {
    let idx_val = (tmp_val >> 16) & 255;
    let clock_event = (tmp_val >> 12) & 15;
    let sim = (tmp_val >> 8) & 15;
    if !core.window_check(0, sim >> 2, 0) {
        if core.ar(sim) >= NF_TABLE2[clock_event as usize] {
            core.next_pc = core.pc.wrapping_add(n_handler1(idx_val)).wrapping_add(4);
        }
    }
}
pub fn n_handler39(core: &mut CoreState, tmp_val: u32) {
    let idx_val = (tmp_val >> 12) & 4095;
    let clock_event = (tmp_val >> 8) & 15;
    if !core.window_check(0, clock_event >> 2, 0) {
        if (0x80000000 & core.ar(clock_event)) == 0 {
            core.next_pc = core.pc.wrapping_add(n_handler2(idx_val)).wrapping_add(4);
        }
    }
}
pub fn n_handler40(core: &mut CoreState, tmp_val: u32) {
    let idx_val = (tmp_val >> 16) & 255;
    let clock_event = (tmp_val >> 8) & 15;
    let sim = (tmp_val >> 4) & 15;
    if !core.window_check(0, clock_event >> 2, sim >> 2) {
        if (core.ar(clock_event) as i32) < (core.ar(sim) as i32) {
            core.next_pc = core.pc.wrapping_add(n_handler1(idx_val)).wrapping_add(4);
        }
    }
}
pub fn n_handler41(core: &mut CoreState, tmp_val: u32) {
    let idx_val = (tmp_val >> 16) & 255;
    let clock_event = (tmp_val >> 12) & 15;
    let sim = (tmp_val >> 8) & 15;
    if !core.window_check(0, sim >> 2, 0) {
        if (core.ar(sim) as i32) < NF_SHIFT_TABLE[clock_event as usize] {
            core.next_pc = core.pc.wrapping_add(n_handler1(idx_val)).wrapping_add(4);
        }
    }
}
pub fn n_handler42(core: &mut CoreState, tmp_val: u32) {
    let idx_val = (tmp_val >> 16) & 255;
    let clock_event = (tmp_val >> 8) & 15;
    let sim = (tmp_val >> 4) & 15;
    if !core.window_check(0, clock_event >> 2, sim >> 2) {
        if core.ar(clock_event) < core.ar(sim) {
            core.next_pc = core.pc.wrapping_add(n_handler1(idx_val)).wrapping_add(4);
        }
    }
}
pub fn n_handler43(core: &mut CoreState, tmp_val: u32) {
    let idx_val = (tmp_val >> 16) & 255;
    let clock_event = (tmp_val >> 12) & 15;
    let sim = (tmp_val >> 8) & 15;
    if !core.window_check(0, sim >> 2, 0) {
        if core.ar(sim) < NF_TABLE2[clock_event as usize] {
            core.next_pc = core.pc.wrapping_add(n_handler1(idx_val)).wrapping_add(4);
        }
    }
}
pub fn n_handler44(core: &mut CoreState, tmp_val: u32) {
    let idx_val = (tmp_val >> 12) & 4095;
    let clock_event = (tmp_val >> 8) & 15;
    if !core.window_check(0, clock_event >> 2, 0) {
        if (0x80000000 & core.ar(clock_event)) != 0 {
            core.next_pc = core.pc.wrapping_add(n_handler2(idx_val)).wrapping_add(4);
        }
    }
}
pub fn n_handler45(core: &mut CoreState, tmp_val: u32) {
    let idx_val = (tmp_val >> 16) & 255;
    let clock_event = (tmp_val >> 8) & 15;
    let sim = (tmp_val >> 4) & 15;
    if !core.window_check(0, clock_event >> 2, sim >> 2) {
        if (!core.ar(clock_event) & core.ar(sim)) != 0 {
            core.next_pc = core.pc.wrapping_add(n_handler1(idx_val)).wrapping_add(4);
        }
    }
}
pub fn n_handler46(core: &mut CoreState, tmp_val: u32) {
    let idx_val = (tmp_val >> 16) & 255;
    let clock_event = (tmp_val >> 8) & 15;
    let sim = (tmp_val >> 4) & 15;
    if !core.window_check(0, clock_event >> 2, sim >> 2) {
        if core.ar(clock_event) != core.ar(sim) {
            core.next_pc = core.pc.wrapping_add(n_handler1(idx_val)).wrapping_add(4);
        }
    }
}
pub fn n_handler47(core: &mut CoreState, tmp_val: u32) {
    let idx_val = (tmp_val >> 16) & 255;
    let clock_event = (tmp_val >> 12) & 15;
    let sim = (tmp_val >> 8) & 15;
    if !core.window_check(0, sim >> 2, 0) {
        if (core.ar(sim) as i32) != NF_SHIFT_TABLE[clock_event as usize] {
            core.next_pc = core.pc.wrapping_add(n_handler1(idx_val)).wrapping_add(4);
        }
    }
}
pub fn n_handler48(core: &mut CoreState, tmp_val: u32) {
    let idx_val = (tmp_val >> 12) & 4095;
    let clock_event = (tmp_val >> 8) & 15;
    if !core.window_check(0, clock_event >> 2, 0) {
        if 0 != core.ar(clock_event) {
            core.next_pc = core.pc.wrapping_add(n_handler2(idx_val)).wrapping_add(4);
        }
    }
}
pub fn n_handler49(core: &mut CoreState, tmp_val: u32) {
    let idx_val = (tmp_val >> 12) & 15;
    let clock_event = (tmp_val >> 8) & 15;
    let sim = (((tmp_val >> 4) & 3) << 4) | idx_val;
    if !core.window_check(0, clock_event >> 2, 0) {
        if 0 != core.ar(clock_event) {
            core.next_pc = core.pc.wrapping_add(sim).wrapping_add(4);
        }
    }
}
pub fn n_handler50(core: &mut CoreState, tmp_val: u32) {
    let idx_val = (tmp_val >> 16) & 255;
    let clock_event = (tmp_val >> 8) & 15;
    let sim = (tmp_val >> 4) & 15;
    if !core.window_check(0, clock_event >> 2, sim >> 2) {
        if (core.ar(clock_event) & core.ar(sim)) == 0 {
            core.next_pc = core.pc.wrapping_add(n_handler1(idx_val)).wrapping_add(4);
        }
    }
}
pub fn n_handler51(core: &mut CoreState, _tmp_val: u32) {
    core.special_registers[INT_RAW] = 8;
    unsafe { super::exports::on_break(core.index); }
}
pub fn n_handler52(core: &mut CoreState, _tmp_val: u32) {
    core.special_registers[INT_RAW] = 16;
    unsafe { super::exports::on_break(core.index); }
}
pub fn n_handler53(core: &mut CoreState, tmp_val: u32) {
    let idx_val = (tmp_val >> 16) & 255;
    let clock_event = (tmp_val >> 8) & 15;
    if core.br(clock_event) {
        core.next_pc = core.pc.wrapping_add(n_handler1(idx_val)).wrapping_add(4);
    }
}

// rHandler functions
pub fn r_handler0(core: &mut CoreState, tmp_val: u32) {
    let idx_val = (tmp_val >> 6) & 262143;
    let clock_event = ((core.pc >> 2).wrapping_add(n_handler4(idx_val)).wrapping_add(1)) << 2;
    core.set_ar(0, core.next_pc);
    core.next_pc = clock_event;
}
pub fn r_handler1(core: &mut CoreState, tmp_val: u32) {
    let idx_val = (tmp_val >> 6) & 262143;
    let clock_event = ((core.pc >> 2).wrapping_add(n_handler4(idx_val)).wrapping_add(1)) << 2;
    if !core.window_check(0, 0, 1) {
        core.set_ps_callinc(1);
        core.set_ar(4, 0x40000000 | (0x3fffffff & core.next_pc));
        core.next_pc = clock_event;
    }
}
pub fn r_handler2(core: &mut CoreState, tmp_val: u32) {
    let idx_val = (tmp_val >> 6) & 262143;
    let clock_event = ((core.pc >> 2).wrapping_add(n_handler4(idx_val)).wrapping_add(1)) << 2;
    if !core.window_check(0, 0, 2) {
        core.set_ps_callinc(2);
        core.set_ar(8, 0x80000000 | (0x3fffffff & core.next_pc));
        core.next_pc = clock_event;
    }
}
pub fn r_handler3(core: &mut CoreState, tmp_val: u32) {
    let idx_val = (tmp_val >> 6) & 262143;
    let clock_event = ((core.pc >> 2).wrapping_add(n_handler4(idx_val)).wrapping_add(1)) << 2;
    if !core.window_check(0, 0, 3) {
        core.set_ps_callinc(3);
        core.set_ar(12, 0xc0000000 | (0x3fffffff & core.next_pc));
        core.next_pc = clock_event;
    }
}
pub fn r_handler4(core: &mut CoreState, tmp_val: u32) {
    let idx_val = (tmp_val >> 8) & 15;
    if core.window_check(0, idx_val >> 2, 0) { return; }
    let clock_event = core.next_pc;
    core.next_pc = core.ar(idx_val);
    core.set_ar(0, clock_event);
}
pub fn r_handler5(core: &mut CoreState, tmp_val: u32) {
    let idx_val = (tmp_val >> 8) & 15;
    if core.window_check(0, 0, 1) { return; }
    core.set_ps_callinc(1);
    let clock_event = core.next_pc;
    core.next_pc = core.ar(idx_val);
    core.set_ar(4, 0x40000000 | (0x3fffffff & clock_event));
}
pub fn r_handler6(core: &mut CoreState, tmp_val: u32) {
    let idx_val = (tmp_val >> 8) & 15;
    if core.window_check(0, 0, 2) { return; }
    core.set_ps_callinc(2);
    let clock_event = core.next_pc;
    core.next_pc = core.ar(idx_val);
    core.set_ar(8, 0x80000000 | (0x3fffffff & clock_event));
}
pub fn r_handler7(core: &mut CoreState, tmp_val: u32) {
    let idx_val = (tmp_val >> 8) & 15;
    if core.window_check(0, 0, 3) { return; }
    core.set_ps_callinc(3);
    let clock_event = core.next_pc;
    core.next_pc = core.ar(idx_val);
    core.set_ar(12, 0xc0000000 | (0x3fffffff & clock_event));
}
pub fn r_handler8(core: &mut CoreState, tmp_val: u32) {
    let idx_val = (tmp_val >> 12) & 15;
    let clock_event = (tmp_val >> 8) & 15;
    let sim = (tmp_val >> 4) & 15;
    if !core.window_check(idx_val >> 2, 0, 0) {
        let f = f32_from_bits(core.float_registers[clock_event as usize]);
        let scale = f32_powi(2.0f32, sim as i32);
        core.set_ar(idx_val, f32_ceil(f * scale) as u32);
    }
}
pub fn r_handler9(core: &mut CoreState, tmp_val: u32) {
    let idx_val = (tmp_val >> 12) & 15;
    let clock_event = (tmp_val >> 8) & 15;
    let sim = (tmp_val >> 4) & 15;
    if core.window_check(idx_val >> 2, clock_event >> 2, 0) { return; }
    let rr = sim + 7;
    let wa = core.ar(clock_event) as i32;
    let rt = (1u32 << rr).wrapping_sub(1);
    let cfg = -(1i32 << rr);
    let hv = if wa > (rt as i32) { rt as i32 } else if wa < cfg { cfg } else { wa };
    core.set_ar(idx_val, hv as u32);
}
pub fn r_handler10(core: &mut CoreState, tmp_val: u32) {
    let idx_val = (tmp_val >> 12) & 15;
    let clock_event = (tmp_val >> 16) & 15;
    const SIM: [f32; 4] = [0.0, 1.0, 2.0, 0.5];
    core.float_registers[idx_val as usize] = f32_to_bits(SIM[(clock_event as usize) % 4]);
}
pub fn r_handler11(core: &mut CoreState, tmp_val: u32) { core.unimplemented(tmp_val); }
pub fn r_handler12(core: &mut CoreState, tmp_val: u32) { core.unimplemented(tmp_val); }
pub fn r_handler13(core: &mut CoreState, tmp_val: u32) { core.unimplemented(tmp_val); }
pub fn r_handler14(core: &mut CoreState, tmp_val: u32) { core.unimplemented(tmp_val); }
pub fn r_handler15(core: &mut CoreState, tmp_val: u32) { core.unimplemented(tmp_val); }
pub fn r_handler16(core: &mut CoreState, tmp_val: u32) { core.unimplemented(tmp_val); }
pub fn r_handler17(core: &mut CoreState, tmp_val: u32) { core.unimplemented(tmp_val); }
pub fn r_handler18(core: &mut CoreState, tmp_val: u32) { core.unimplemented(tmp_val); }
pub fn r_handler19(core: &mut CoreState, tmp_val: u32) { core.unimplemented(tmp_val); }
pub fn r_handler20(core: &mut CoreState, tmp_val: u32) { core.unimplemented(tmp_val); }
pub fn r_handler21(core: &mut CoreState, tmp_val: u32) { core.unimplemented(tmp_val); }
pub fn r_handler22(core: &mut CoreState, tmp_val: u32) { core.unimplemented(tmp_val); }
pub fn r_handler23(core: &mut CoreState, tmp_val: u32) { core.unimplemented(tmp_val); }
pub fn r_handler24(core: &mut CoreState, tmp_val: u32) {
    let idx_val = (tmp_val >> 12) & 4095;
    let clock_event = (tmp_val >> 8) & 15;
    if core.window_check(0, core.ps_callinc(), 0) { return; }
    if clock_event > 3 || core.ps_woe() == 0 || core.ps_excm() != 0 {
        core.exception(TRAP_ILLEGAL_INSTRUCTION);
    } else {
        // traceEntry call back to JS
        core.set_ar((core.ps_callinc() << 2) | clock_event, core.ar(clock_event).wrapping_sub(idx_val << 3));
        let tmp = (core.special_registers[MEM_FAULT_INFO].wrapping_add(core.ps_callinc())) & 15;
        core.special_registers[MEM_FAULT_INFO] = tmp;
        core.special_registers[CACHE_CONTROL] |= 1 << tmp;
    }
}
pub fn r_handler25(core: &mut CoreState, tmp_val: u32) { core.unimplemented(tmp_val); }
pub fn r_handler26(core: &mut CoreState, tmp_val: u32) {
    let idx_val = (tmp_val >> 20) & 15;
    let clock_event = (tmp_val >> 16) & 1;
    let sim = (tmp_val >> 12) & 15;
    let rr = (tmp_val >> 8) & 15;
    let wa = (tmp_val >> 4) & 15;
    let rt = (clock_event << 4) | rr;
    let cfg = (1u32 << (idx_val + 1)).wrapping_sub(1);
    if !core.window_check(sim >> 2, 0, wa >> 2) {
        core.set_ar(sim, (core.ar(wa) >> rt) & cfg);
    }
}
pub fn r_handler27(core: &mut CoreState, tmp_val: u32) {
    let idx_val = (tmp_val >> 12) & 15;
    let clock_event = (tmp_val >> 8) & 15;
    let sim = (tmp_val >> 4) & 15;
    if !core.window_check(0, clock_event >> 2, 0) {
        let val = (core.ar(clock_event) as f32) * f32_powi(2.0f32, -(sim as i32));
        core.float_registers[idx_val as usize] = f32_to_bits(val);
    }
}
pub fn r_handler28(core: &mut CoreState, tmp_val: u32) {
    let idx_val = (tmp_val >> 12) & 15;
    let clock_event = (tmp_val >> 8) & 15;
    let sim = (tmp_val >> 4) & 15;
    if !core.window_check(idx_val >> 2, 0, 0) {
        let f = f32_from_bits(core.float_registers[clock_event as usize]);
        core.set_ar(idx_val, f32_floor(f * f32_powi(2.0f32, sim as i32)) as u32);
    }
}
pub fn r_handler29(core: &mut CoreState, tmp_val: u32) { core.unimplemented(tmp_val); }
pub fn r_handler30(core: &mut CoreState, tmp_val: u32) { core.unimplemented(tmp_val); }
pub fn r_handler31(core: &mut CoreState, tmp_val: u32) { core.unimplemented(tmp_val); }
pub fn r_handler32(core: &mut CoreState, tmp_val: u32) { core.unimplemented(tmp_val); }
pub fn r_handler33(core: &mut CoreState, tmp_val: u32) { core.unimplemented(tmp_val); }
pub fn r_handler34(core: &mut CoreState, tmp_val: u32) { core.unimplemented(tmp_val); }
pub fn r_handler35(core: &mut CoreState, _tmp_val: u32) { core.exception(TRAP_ILLEGAL_INSTRUCTION); }
pub fn r_handler36(core: &mut CoreState, _tmp_val: u32) { core.exception(TRAP_ILLEGAL_INSTRUCTION); }
pub fn r_handler37(core: &mut CoreState, tmp_val: u32) { core.unimplemented(tmp_val); }
pub fn r_handler38(core: &mut CoreState, tmp_val: u32) { core.unimplemented(tmp_val); }
pub fn r_handler39(core: &mut CoreState, tmp_val: u32) {
    let idx_val = (tmp_val >> 6) & 262143;
    core.next_pc = core.pc.wrapping_add(n_handler4(idx_val)).wrapping_add(4);
}
pub fn r_handler40(core: &mut CoreState, tmp_val: u32) {
    let idx_val = (tmp_val >> 8) & 15;
    if !core.window_check(0, idx_val >> 2, 0) {
        core.next_pc = core.ar(idx_val);
    }
}
pub fn r_handler41(core: &mut CoreState, tmp_val: u32) {
    let idx_val = (tmp_val >> 16) & 255;
    let clock_event = (tmp_val >> 8) & 15;
    let sim = (tmp_val >> 4) & 15;
    if !core.window_check(0, clock_event >> 2, sim >> 2) {
        let addr = core.ar(clock_event).wrapping_add(idx_val);
        let val = read_uint8(core, addr);
        core.set_ar(sim, val);
    }
}
pub fn r_handler42(core: &mut CoreState, tmp_val: u32) {
    let idx_val = (tmp_val >> 16) & 255;
    let clock_event = (tmp_val >> 8) & 15;
    let sim = (tmp_val >> 4) & 15;
    if !core.window_check(0, clock_event >> 2, sim >> 2) {
        let addr = core.ar(clock_event).wrapping_add(idx_val << 1);
        let val = read_uint16(core, addr);
        core.set_ar(sim, n_handler3(val));
    }
}
pub fn r_handler43(core: &mut CoreState, tmp_val: u32) {
    let idx_val = (tmp_val >> 16) & 255;
    let clock_event = (tmp_val >> 8) & 15;
    let sim = (tmp_val >> 4) & 15;
    if !core.window_check(0, clock_event >> 2, sim >> 2) {
        let addr = core.ar(clock_event).wrapping_add(idx_val << 1);
        let val = read_uint16(core, addr);
        core.set_ar(sim, val);
    }
}
pub fn r_handler44(core: &mut CoreState, tmp_val: u32) {
    let idx_val = (tmp_val >> 16) & 255;
    let clock_event = (tmp_val >> 8) & 15;
    let sim = (tmp_val >> 4) & 15;
    if core.window_check(0, clock_event >> 2, sim >> 2) { return; }
    let rr = core.ar(clock_event).wrapping_add(idx_val << 2);
    let val = read_uint32(core, rr);
    core.set_ar(sim, val);
}
pub fn r_handler45(core: &mut CoreState, tmp_val: u32) {
    let idx_val = (tmp_val >> 12) & 15;
    let clock_event = (tmp_val >> 8) & 15;
    let sim = (tmp_val >> 4) & 15;
    if core.ps_owing() != 0 { core.exception(TRAP_PRIVILEGED); }
    else {
        let tmp = core.ar(clock_event).wrapping_add(0xffffffc0 | (idx_val << 2));
        let val = read_uint32(core, tmp);
        core.set_ar(sim, val);
    }
}
pub fn r_handler46(core: &mut CoreState, tmp_val: u32) {
    let idx_val = (tmp_val >> 16) & 255;
    let clock_event = (tmp_val >> 8) & 15;
    let sim = (tmp_val >> 4) & 15;
    if !core.window_check(0, clock_event >> 2, sim >> 2) {
        let addr = core.ar(clock_event).wrapping_add(idx_val << 2);
        let val = read_uint32(core, addr);
        core.set_ar(sim, val);
    }
}
pub fn r_handler47(core: &mut CoreState, tmp_val: u32) {
    let idx_val = (tmp_val >> 12) & 15;
    let clock_event = (tmp_val >> 8) & 15;
    let sim = (tmp_val >> 4) & 15;
    if !core.window_check(0, clock_event >> 2, sim >> 2) {
        let addr = core.ar(clock_event).wrapping_add(idx_val << 2);
        let val = read_uint32(core, addr);
        core.set_ar(sim, val);
    }
}
pub fn r_handler48(core: &mut CoreState, tmp_val: u32) {
    let clock_event = (tmp_val >> 4) & 15;
    if !core.window_check(0, 0, clock_event >> 2) {
        let idx_val = (tmp_val >> 8) & 65535;
        let addr = ((core.next_pc >> 2).wrapping_add(0xffff0000 | idx_val)) << 2;
        let val = read_uint32(core, addr);
        core.set_ar(clock_event, val);
    }
}
pub fn r_handler49(core: &mut CoreState, tmp_val: u32) { core.unimplemented(tmp_val); }
pub fn r_handler50(core: &mut CoreState, tmp_val: u32) {
    let idx_val = (tmp_val >> 12) & 3;
    let clock_event = (tmp_val >> 8) & 15;
    if core.window_check(0, clock_event >> 2, 0) { return; }
    let sim = core.ar(clock_event).wrapping_sub(4);
    core.special_registers[INTERRUPT_STATE + idx_val as usize] = read_uint32(core, sim);
    core.set_ar(clock_event, sim);
}
pub fn r_handler51(core: &mut CoreState, tmp_val: u32) {
    let idx_val = (tmp_val >> 12) & 3;
    let clock_event = (tmp_val >> 8) & 15;
    if core.window_check(0, clock_event >> 2, 0) { return; }
    let sim = core.ar(clock_event).wrapping_add(4);
    core.special_registers[INTERRUPT_STATE + idx_val as usize] = read_uint32(core, sim);
    core.set_ar(clock_event, sim);
}
pub fn r_handler52(core: &mut CoreState, tmp_val: u32) { core.unimplemented(tmp_val); }
pub fn r_handler53(core: &mut CoreState, tmp_val: u32) { core.unimplemented(tmp_val); }
pub fn r_handler54(core: &mut CoreState, tmp_val: u32) {
    let idx_val = (tmp_val >> 16) & 255;
    let clock_event = (tmp_val >> 8) & 15;
    let sim = core.pc.wrapping_add(idx_val).wrapping_add(4);
    if !core.window_check(0, clock_event >> 2, 0) {
        core.special_registers[LOOP_COUNT] = core.ar(clock_event).wrapping_sub(1);
        core.special_registers[LOOP_BEGIN] = core.next_pc;
        core.special_registers[LOOP_END] = sim;
    }
}
pub fn r_handler55(core: &mut CoreState, tmp_val: u32) {
    let idx_val = (tmp_val >> 16) & 255;
    let clock_event = (tmp_val >> 8) & 15;
    let sim = core.pc.wrapping_add(idx_val).wrapping_add(4);
    if !core.window_check(0, clock_event >> 2, 0) {
        core.special_registers[LOOP_COUNT] = core.ar(clock_event).wrapping_sub(1);
        core.special_registers[LOOP_BEGIN] = core.next_pc;
        core.special_registers[LOOP_END] = sim;
        if (core.ar(clock_event) as i32) <= 0 {
            core.next_pc = sim;
        }
    }
}
pub fn r_handler56(core: &mut CoreState, tmp_val: u32) {
    let idx_val = (tmp_val >> 16) & 255;
    let clock_event = (tmp_val >> 8) & 15;
    let sim = core.pc.wrapping_add(idx_val).wrapping_add(4);
    if !core.window_check(0, clock_event >> 2, 0) {
        core.special_registers[LOOP_COUNT] = core.ar(clock_event).wrapping_sub(1);
        core.special_registers[LOOP_BEGIN] = core.next_pc;
        core.special_registers[LOOP_END] = sim;
        if core.ar(clock_event) == 0 {
            core.next_pc = sim;
        }
    }
}
pub fn r_handler57(core: &mut CoreState, tmp_val: u32) {
    let idx_val = (tmp_val >> 16) & 255;
    let clock_event = (tmp_val >> 8) & 15;
    let sim = (tmp_val >> 4) & 15;
    if core.window_check(0, clock_event >> 2, 0) { return; }
    let rr = core.ar(clock_event).wrapping_add(idx_val << 2);
    core.float_registers[sim as usize] = read_uint32(core, rr);
}
pub fn r_handler58(core: &mut CoreState, tmp_val: u32) {
    let idx_val = (tmp_val >> 16) & 255;
    let clock_event = (tmp_val >> 8) & 15;
    let sim = (tmp_val >> 4) & 15;
    if core.window_check(0, clock_event >> 2, 0) { return; }
    let rr = core.ar(clock_event).wrapping_add(idx_val << 2);
    core.float_registers[sim as usize] = read_uint32(core, rr);
    core.set_ar(clock_event, rr);
}
pub fn r_handler59(core: &mut CoreState, tmp_val: u32) {
    let idx_val = (tmp_val >> 12) & 15;
    let clock_event = (tmp_val >> 8) & 15;
    let sim = (tmp_val >> 4) & 15;
    if core.window_check(0, clock_event >> 2, sim >> 2) { return; }
    let rr = core.ar(clock_event).wrapping_add(core.ar(sim));
    core.float_registers[idx_val as usize] = read_uint32(core, rr);
}
pub fn r_handler60(core: &mut CoreState, tmp_val: u32) {
    let idx_val = (tmp_val >> 12) & 15;
    let clock_event = (tmp_val >> 8) & 15;
    let sim = (tmp_val >> 4) & 15;
    if core.window_check(0, clock_event >> 2, sim >> 2) { return; }
    let rr = core.ar(clock_event).wrapping_add(core.ar(sim));
    core.float_registers[idx_val as usize] = read_uint32(core, rr);
    core.set_ar(clock_event, rr);
}
pub fn r_handler61(core: &mut CoreState, tmp_val: u32) {
    let idx_val = (tmp_val >> 12) & 15;
    let clock_event = (tmp_val >> 8) & 15;
    let sim = (tmp_val >> 4) & 15;
    core.float_registers[idx_val as usize] = f32_to_bits(
        f32_from_bits(core.float_registers[idx_val as usize]) +
        f32_from_bits(core.float_registers[clock_event as usize]) * f32_from_bits(core.float_registers[sim as usize])
    );
}
pub fn r_handler62(core: &mut CoreState, tmp_val: u32) {
    let idx_val = (tmp_val >> 12) & 15;
    let clock_event = (tmp_val >> 8) & 15;
    let sim = (tmp_val >> 4) & 15;
    if core.window_check(idx_val >> 2, clock_event >> 2, sim >> 2) { return; }
    let rr = core.ar(clock_event) as i32;
    let wa = core.ar(sim) as i32;
    core.set_ar(idx_val, if rr > wa { rr as u32 } else { wa as u32 });
}
pub fn r_handler63(core: &mut CoreState, tmp_val: u32) {
    let idx_val = (tmp_val >> 12) & 15;
    let clock_event = (tmp_val >> 8) & 15;
    let sim = (tmp_val >> 4) & 15;
    if core.window_check(idx_val >> 2, clock_event >> 2, sim >> 2) { return; }
    let rr = core.ar(clock_event);
    let wa = core.ar(sim);
    core.set_ar(idx_val, if rr > wa { rr } else { wa });
}

// aHandler functions (partial — will continue with all of them)
pub fn a_handler0(core: &mut CoreState, tmp_val: u32) {
    let idx_val = (tmp_val >> 12) & 15;
    let clock_event = (tmp_val >> 8) & 15;
    let sim = (tmp_val >> 4) & 15;
    if core.window_check(idx_val >> 2, clock_event >> 2, sim >> 2) { return; }
    let rr = core.ar(clock_event) as i32;
    let wa = core.ar(sim) as i32;
    core.set_ar(idx_val, if rr < wa { rr as u32 } else { wa as u32 });
}
pub fn a_handler1(core: &mut CoreState, tmp_val: u32) {
    let idx_val = (tmp_val >> 12) & 15;
    let clock_event = (tmp_val >> 8) & 15;
    let sim = (tmp_val >> 4) & 15;
    if core.window_check(idx_val >> 2, clock_event >> 2, sim >> 2) { return; }
    let rr = core.ar(clock_event);
    let wa = core.ar(sim);
    core.set_ar(idx_val, if rr < wa { rr } else { wa });
}
pub fn a_handler2(core: &mut CoreState, tmp_val: u32) {
    let idx_val = (tmp_val >> 12) & 15;
    let clock_event = (tmp_val >> 8) & 15;
    core.float_registers[idx_val as usize] = f32_to_bits(
        f32_from_bits(core.float_registers[clock_event as usize]) / f32_from_bits(core.float_registers[idx_val as usize])
    );
}
pub fn a_handler3(core: &mut CoreState, tmp_val: u32) {
    let idx_val = (tmp_val >> 12) & 15;
    let clock_event = (tmp_val >> 8) & 15;
    core.float_registers[idx_val as usize] = f32_to_bits(f32_sqrt(f32_from_bits(core.float_registers[clock_event as usize])));
}
pub fn a_handler4(core: &mut CoreState, tmp_val: u32) {
    let idx_val = (tmp_val >> 8) & 15;
    let clock_event = (tmp_val >> 4) & 15;
    if !core.window_check(0, idx_val >> 2, clock_event >> 2) {
        core.set_ar(clock_event, core.ar(idx_val));
    }
}
pub fn a_handler5(core: &mut CoreState, tmp_val: u32) {
    let idx_val = (tmp_val >> 12) & 15;
    let clock_event = (tmp_val >> 8) & 15;
    core.float_registers[idx_val as usize] = core.float_registers[clock_event as usize];
}
pub fn a_handler6(core: &mut CoreState, tmp_val: u32) {
    let idx_val = (tmp_val >> 12) & 15;
    let clock_event = (tmp_val >> 8) & 15;
    let sim = (tmp_val >> 4) & 15;
    if !core.window_check(idx_val >> 2, clock_event >> 2, sim >> 2) {
        if core.ar(sim) == 0 {
            core.set_ar(idx_val, core.ar(clock_event));
        }
    }
}
pub fn a_handler7(core: &mut CoreState, tmp_val: u32) {
    let idx_val = (tmp_val >> 12) & 15;
    let clock_event = (tmp_val >> 8) & 15;
    let sim = (tmp_val >> 4) & 15;
    if !core.window_check(0, 0, sim >> 2) {
        if core.ar(sim) == 0 {
            core.float_registers[idx_val as usize] = core.float_registers[clock_event as usize];
        }
    }
}
pub fn a_handler8(core: &mut CoreState, tmp_val: u32) {
    let idx_val = (tmp_val >> 12) & 15;
    let clock_event = (tmp_val >> 8) & 15;
    let sim = (tmp_val >> 4) & 15;
    if !core.window_check(idx_val >> 2, clock_event >> 2, 0) {
        if !core.br(sim) { core.set_ar(idx_val, core.ar(clock_event)); }
    }
}
pub fn a_handler9(core: &mut CoreState, tmp_val: u32) {
    let idx_val = (tmp_val >> 12) & 15;
    let clock_event = (tmp_val >> 8) & 15;
    let sim = (tmp_val >> 4) & 15;
    if !core.br(sim) { core.float_registers[idx_val as usize] = core.float_registers[clock_event as usize]; }
}
pub fn a_handler10(core: &mut CoreState, tmp_val: u32) {
    let idx_val = (tmp_val >> 12) & 15;
    let clock_event = (tmp_val >> 8) & 15;
    let sim = (tmp_val >> 4) & 15;
    if !core.window_check(idx_val >> 2, clock_event >> 2, sim >> 2) {
        if (0x80000000 & core.ar(sim)) == 0 {
            core.set_ar(idx_val, core.ar(clock_event));
        }
    }
}
pub fn a_handler11(core: &mut CoreState, tmp_val: u32) {
    let idx_val = (tmp_val >> 12) & 15;
    let clock_event = (tmp_val >> 8) & 15;
    let sim = (tmp_val >> 4) & 15;
    if !core.window_check(0, 0, sim >> 2) {
        if (0x80000000 & core.ar(sim)) == 0 {
            core.float_registers[idx_val as usize] = core.float_registers[clock_event as usize];
        }
    }
}
pub fn a_handler12(core: &mut CoreState, tmp_val: u32) {
    let idx_val = (tmp_val >> 16) & 255;
    let clock_event = (tmp_val >> 8) & 15;
    let sim = (tmp_val >> 4) & 15;
    if !core.window_check(0, 0, sim >> 2) {
        core.set_ar(sim, n_handler2((clock_event << 8) | idx_val));
    }
}
pub fn a_handler13(core: &mut CoreState, tmp_val: u32) {
    let idx_val = (tmp_val >> 12) & 15;
    let clock_event = (tmp_val >> 8) & 15;
    let sim = (((tmp_val >> 4) & 7) << 4) | idx_val;
    if !core.window_check(0, clock_event >> 2, 0) {
        let extended = if (sim & 96) == 96 { sim | 0xffffff80 } else { sim };
        core.set_ar(clock_event, extended);
    }
}
// ... CONTINUED WITH MORE HANDLERS

pub fn c_handler7(core: &mut CoreState, tmp_val: u32) -> Option<DecodeFn> {
    match tmp_val & 15 {
        0 => {
            match tmp_val & 917504 {
                0 => {
                    match tmp_val & 0xe10000 {
                        0 => {
                            match tmp_val & 1048576 {
                                0 => {
                                    match tmp_val & 61440 {
                                        0 => {
                                            match tmp_val & 240 {
                                                0 => return Some(r_handler35),
                                                128 => return Some(a_handler61),
                                                144 => return Some(a_handler62),
                                                160 => return Some(r_handler40),
                                                192 => return Some(r_handler4),
                                                208 => return Some(r_handler5),
                                                224 => return Some(r_handler6),
                                                240 => return Some(r_handler7),
                                                _ => {}
                                            }
                                        }
                                        4096 => return Some(a_handler18),
                                        8192 => {
                                            match tmp_val & 4080 {
                                                0 | 16 | 32 | 48 | 192 | 208 | 240 => return None,
                                                128 => return Some(r_handler25),
                                                _ => {}
                                            }
                                        }
                                        12288 => {
                                            match tmp_val & 240 {
                                                0 => {
                                                    match tmp_val & 3840 {
                                                        0 => return Some(_handler2),
                                                        256 => return Some(_handler6),
                                                        512 => return Some(_handler0),
                                                        1024 => return Some(_handler7),
                                                        1280 => return Some(_handler8),
                                                        _ => {}
                                                    }
                                                }
                                                16 => return Some(_handler3),
                                                32 => return Some(_handler4),
                                                _ => {}
                                            }
                                        }
                                        16384 => return Some(n_handler51),
                                        20480 => {
                                            if (tmp_val & 4080) == 256 { return Some(_handler29); }
                                            if (tmp_val & 4080) == 0 { return Some(_handler54); }
                                        }
                                        24576 => return Some(_handler13),
                                        28672 => return Some(_handler63),
                                        32768 => return Some(n_handler22),
                                        36864 => return Some(n_handler17),
                                        40960 => return Some(n_handler23),
                                        45056 => return Some(n_handler18),
                                        _ => {}
                                    }
                                }
                                1048576 => return Some(n_handler19),
                                _ => {}
                            }
                        }
                        65536 => return Some(_handler31),
                        2097152 => {
                            if (tmp_val & 1048576) == 0 { return Some(a_handler51); }
                            if (tmp_val & 1048576) == 1048576 { return Some(c_handler4); }
                        }
                        2162688 => return Some(_handler36),
                        4194304 => {
                            match tmp_val & 1110016 {
                                0 => return Some(_handler46),
                                4096 => return Some(_handler45),
                                8192 => return Some(_handler41),
                                12288 => return Some(_handler40),
                                16384 => return Some(_handler42),
                                24576 => return Some(a_handler60),
                                28672 => return Some(c_handler0),
                                32768 => return Some(_handler11),
                                57344 => return Some(a_handler46),
                                61440 => return Some(a_handler47),
                                1060864 => return Some(_handler9),
                                1064960 => return Some(r_handler33),
                                1069056 => return Some(a_handler55),
                                1073152 | 1093632 | 1105920 | 1110016 => return None,
                                1077248 => return Some(_handler10),
                                1097728 => return Some(r_handler29),
                                1101824 => return Some(a_handler54),
                                _ => {}
                            }
                        }
                        4259840 => return Some(_handler39),
                        6291456 => {
                            if (tmp_val & 1052416) == 256 { return Some(n_handler5); }
                            if (tmp_val & 1052416) == 0 { return Some(a_handler44); }
                        }
                        6356992 => return Some(c_handler6),
                        8388608 => {
                            if (tmp_val & 1048576) == 0 { return Some(n_handler7); }
                            if (tmp_val & 1048576) == 1048576 { return Some(n_handler14); }
                        }
                        8454144 => {
                            if (tmp_val & 1048576) == 0 { return Some(_handler37); }
                            if (tmp_val & 1052416) == 1048576 { return Some(_handler38); }
                        }
                        0xa00000 => {
                            if (tmp_val & 1048576) == 0 { return Some(n_handler15); }
                            if (tmp_val & 1048576) == 1048576 { return Some(n_handler16); }
                        }
                        0xa10000 => {
                            if (tmp_val & 1048816) == 0 { return Some(_handler30); }
                            if (tmp_val & 1052416) == 1048576 { return Some(_handler35); }
                        }
                        0xc00000 => {
                            if (tmp_val & 1048576) == 0 { return Some(_handler49); }
                            if (tmp_val & 1048576) == 1048576 { return Some(_handler51); }
                        }
                        0xc10000 => {
                            if (tmp_val & 1048576) == 1048576 { return Some(a_handler27); }
                            if (tmp_val & 1048576) == 0 { return Some(a_handler28); }
                        }
                        0xe00000 => {
                            if (tmp_val & 1048576) == 0 { return Some(_handler52); }
                            if (tmp_val & 1048576) == 1048576 { return Some(_handler53); }
                        }
                        0xe10000 => {
                            match tmp_val & 1110016 {
                                1048576 => return Some(r_handler52),
                                1052672 => return Some(_handler27),
                                1056768 => return Some(r_handler53),
                                1060864 => return Some(_handler28),
                                1081344 => return Some(r_handler49),
                                1085440 => return Some(_handler25),
                                1105920 => {
                                    if (tmp_val & 3824) == 16 { return Some(a_handler63); }
                                    if (tmp_val & 4080) == 0 { return Some(_handler1); }
                                }
                                _ => {}
                            }
                        }
                        _ => {}
                    }
                }
                131072 => {
                    match tmp_val & 0xf10000 {
                        0 => return Some(n_handler20),
                        65536 => return Some(_handler14),
                        1048576 => return Some(n_handler21),
                        1114112 => return Some(c_handler2),
                        2097152 => return Some(a_handler52),
                        2162688 => return Some(_handler26),
                        3145728 => return Some(a_handler53),
                        3211264 => return Some(r_handler9),
                        4194304 => return Some(c_handler5),
                        4259840 => return Some(a_handler0),
                        5308416 => return Some(r_handler62),
                        6291456 => return Some(_handler24),
                        6356992 => return Some(a_handler1),
                        7340032 => return Some(_handler23),
                        7405568 => return Some(r_handler63),
                        8388608 => return Some(a_handler37),
                        8454144 => return Some(a_handler6),
                        9502720 => return Some(a_handler16),
                        0xa00000 => return Some(a_handler43),
                        0xa10000 => return Some(a_handler14),
                        0xb00000 => return Some(a_handler42),
                        0xb10000 => return Some(a_handler10),
                        0xc00000 => return Some(a_handler57),
                        0xc10000 => return Some(a_handler8),
                        0xd00000 => return Some(a_handler56),
                        0xd10000 => return Some(a_handler19),
                        0xe00000 => return Some(a_handler59),
                        0xe10000 => return Some(_handler15),
                        0xf00000 => return Some(a_handler58),
                        0xf10000 => return Some(c_handler3),
                        _ => {}
                    }
                }
                262144 => return Some(r_handler26),
                524288 => {
                    match tmp_val & 0xf10000 {
                        0 => return Some(r_handler59),
                        65536 => return Some(r_handler45),
                        1048576 => return Some(r_handler60),
                        4194304 => return Some(_handler47),
                        4259840 => return Some(_handler19),
                        5242880 => return Some(_handler48),
                        _ => {}
                    }
                }
                655360 => {
                    match tmp_val & 0xf10000 {
                        0 => return Some(n_handler10),
                        1048576 => return Some(_handler50),
                        1114112 => return Some(_handler61),
                        2097152 => return Some(a_handler26),
                        2162688 => return Some(a_handler48),
                        3211264 => return Some(_handler56),
                        4194304 => return Some(r_handler61),
                        4259840 => return Some(a_handler50),
                        5242880 => return Some(a_handler21),
                        5308416 => return Some(_handler59),
                        6291456 | 7340032 => return None,
                        6356992 => return Some(a_handler49),
                        7405568 => return Some(_handler58),
                        8388608 => return Some(_handler12),
                        8454144 => return Some(a_handler7),
                        9437184 => return Some(_handler55),
                        9502720 => return Some(a_handler17),
                        0xa00000 => return Some(r_handler28),
                        0xa10000 => return Some(a_handler15),
                        0xb00000 => return Some(r_handler8),
                        0xb10000 => return Some(a_handler11),
                        0xc00000 => return Some(r_handler27),
                        0xc10000 => return Some(a_handler9),
                        0xd00000 => return Some(_handler57),
                        0xd10000 => return Some(a_handler20),
                        0xe00000 => return Some(_handler62),
                        0xf00000 => {
                            match tmp_val & 240 {
                                0 => return Some(a_handler5),
                                16 => return Some(n_handler6),
                                48 => return Some(r_handler10),
                                64 => return Some(_handler5),
                                80 => return Some(c_handler1),
                                96 => return Some(a_handler45),
                                112 | 176 | 224 => return None,
                                128 => return Some(_handler33),
                                144 => return Some(_handler32),
                                160 => return Some(_handler34),
                                192 => return Some(a_handler3),
                                208 => return Some(a_handler2),
                                240 => return Some(n_handler8),
                                _ => {}
                            }
                        }
                        _ => {}
                    }
                }
                _ => {}
            }
        }
        1 => return Some(r_handler48),
        2 => {
            match tmp_val & 61440 {
                0 => return Some(r_handler41),
                4096 => return Some(r_handler43),
                8192 => return Some(r_handler46),
                16384 => return Some(_handler16),
                20480 => return Some(_handler17),
                24576 => return Some(_handler20),
                28672 => {
                    match tmp_val & 240 {
                        0 => return Some(r_handler20),
                        16 => return Some(r_handler22),
                        32 => return Some(r_handler21),
                        48 => return Some(r_handler23),
                        64 => return Some(r_handler13),
                        80 => return Some(r_handler14),
                        96 => return Some(r_handler11),
                        112 => return Some(r_handler15),
                        128 => {
                            match tmp_val & 983040 {
                                0 => return Some(r_handler19),
                                131072 => return Some(r_handler12),
                                196608 => return Some(r_handler16),
                                262144 => return Some(r_handler17),
                                327680 => return Some(r_handler18),
                                _ => {}
                            }
                        }
                        192 => return Some(r_handler37),
                        208 => {
                            match tmp_val & 983040 {
                                0 => return Some(r_handler38),
                                131072 => return Some(r_handler31),
                                196608 => return Some(r_handler34),
                                _ => {}
                            }
                        }
                        224 => return Some(r_handler30),
                        240 => return Some(r_handler32),
                        _ => {}
                    }
                }
                36864 => return Some(r_handler42),
                40960 => return Some(a_handler12),
                45056 => return Some(r_handler44),
                49152 => return Some(n_handler11),
                53248 => return Some(n_handler13),
                57344 => return Some(_handler18),
                61440 => return Some(_handler22),
                _ => {}
            }
        }
        3 => {
            match tmp_val & 61440 {
                0 => return Some(r_handler57),
                16384 => return Some(_handler43),
                32768 => return Some(r_handler58),
                49152 => return Some(_handler44),
                _ => {}
            }
        }
        4 => {
            match tmp_val & 0xfc8000 {
                524288 => return Some(a_handler36),
                1572864 => return Some(a_handler35),
                2359296 => return Some(a_handler25),
                2621440 => return Some(a_handler34),
                2883584 => return Some(a_handler41),
                3407872 => return Some(a_handler23),
                3670016 => return Some(a_handler30),
                3932160 => return Some(a_handler39),
                4718592 => return Some(a_handler33),
                5767168 => return Some(a_handler32),
                6553600 => return Some(a_handler24),
                6815744 => return Some(a_handler31),
                7077888 => return Some(a_handler40),
                7340032 => return Some(_handler60),
                7602176 => return Some(a_handler22),
                7864320 => return Some(a_handler29),
                8126464 => return Some(a_handler38),
                8388608 => return Some(r_handler51),
                9437184 => return Some(r_handler50),
                _ => {}
            }
        }
        5 => {
            match tmp_val & 48 {
                0 => return Some(r_handler0),
                16 => return Some(r_handler1),
                32 => return Some(r_handler2),
                48 => return Some(r_handler3),
                _ => {}
            }
        }
        6 => {
            match tmp_val & 48 {
                0 => return Some(r_handler39),
                16 => {
                    match tmp_val & 192 {
                        0 => return Some(n_handler32),
                        64 => return Some(n_handler48),
                        128 => return Some(n_handler44),
                        192 => return Some(n_handler39),
                        _ => {}
                    }
                }
                32 => {
                    match tmp_val & 192 {
                        0 => return Some(n_handler31),
                        64 => return Some(n_handler47),
                        128 => return Some(n_handler41),
                        192 => return Some(n_handler36),
                        _ => {}
                    }
                }
                48 => {
                    match tmp_val & 192 {
                        0 => return Some(r_handler24),
                        64 => {
                            match tmp_val & 61440 {
                                0 => return Some(n_handler34),
                                4096 => return Some(n_handler53),
                                32768 => return Some(r_handler54),
                                36864 => return Some(r_handler56),
                                40960 => return Some(r_handler55),
                                _ => {}
                            }
                        }
                        128 => return Some(n_handler43),
                        192 => return Some(n_handler38),
                        _ => {}
                    }
                }
                _ => {}
            }
        }
        7 => {
            match tmp_val & 57344 {
                0 => {
                    if (tmp_val & 4096) == 4096 { return Some(n_handler30); }
                    if (tmp_val & 4096) == 0 { return Some(n_handler50); }
                }
                8192 => {
                    if (tmp_val & 4096) == 0 { return Some(n_handler40); }
                    if (tmp_val & 4096) == 4096 { return Some(n_handler42); }
                }
                16384 => {
                    if (tmp_val & 4096) == 0 { return Some(n_handler24); }
                    if (tmp_val & 4096) == 4096 { return Some(n_handler26); }
                }
                24576 => return Some(n_handler27),
                32768 => {
                    if (tmp_val & 4096) == 0 { return Some(n_handler25); }
                    if (tmp_val & 4096) == 4096 { return Some(n_handler46); }
                }
                40960 => {
                    if (tmp_val & 4096) == 0 { return Some(n_handler35); }
                    if (tmp_val & 4096) == 4096 { return Some(n_handler37); }
                }
                49152 => {
                    if (tmp_val & 4096) == 4096 { return Some(n_handler28); }
                    if (tmp_val & 4096) == 0 { return Some(n_handler45); }
                }
                57344 => return Some(n_handler29),
                _ => {}
            }
        }
        8 => return Some(r_handler47),
        9 => return Some(_handler21),
        10 => return Some(n_handler9),
        11 => return Some(n_handler12),
        12 => {
            match tmp_val & 128 {
                0 => return Some(a_handler13),
                128 => {
                    if (tmp_val & 64) == 0 { return Some(n_handler33); }
                    if (tmp_val & 64) == 64 { return Some(n_handler49); }
                }
                _ => {}
            }
        }
        13 => {
            match tmp_val & 61440 {
                0 => return Some(a_handler4),
                61440 => {
                    match tmp_val & 240 {
                        0 => return Some(a_handler61),
                        16 => return Some(a_handler62),
                        32 => return Some(n_handler52),
                        48 => return None,
                        96 => return Some(r_handler36),
                        _ => {}
                    }
                }
                _ => {}
            }
        }
        _ => {}
    }
    // unknownInstruction — call back to JS (miss fall-through preserved:
    // the caller treats None as miss and continues exactly as before).
    core.exception(TRAP_ILLEGAL_INSTRUCTION);
    None
}

// Stubs for remaining handlers (will be filled in from JS source)
pub fn a_handler14(core: &mut CoreState, tmp_val: u32) {
    let idx_val = (tmp_val >> 12) & 15;
    let clock_event = (tmp_val >> 8) & 15;
    let sim = (tmp_val >> 4) & 15;
    if !core.window_check(idx_val >> 2, clock_event >> 2, sim >> 2) {
        if (0x80000000 & core.ar(sim)) != 0 {
            core.set_ar(idx_val, core.ar(clock_event));
        }
    }
}
pub fn a_handler15(core: &mut CoreState, tmp_val: u32) {
    let idx_val = (tmp_val >> 12) & 15;
    let clock_event = (tmp_val >> 8) & 15;
    let sim = (tmp_val >> 4) & 15;
    if !core.window_check(0, 0, sim >> 2) {
        if (0x80000000 & core.ar(sim)) != 0 {
            core.float_registers[idx_val as usize] = core.float_registers[clock_event as usize];
        }
    }
}
pub fn a_handler16(core: &mut CoreState, tmp_val: u32) {
    let idx_val = (tmp_val >> 12) & 15;
    let clock_event = (tmp_val >> 8) & 15;
    let sim = (tmp_val >> 4) & 15;
    if !core.window_check(idx_val >> 2, clock_event >> 2, sim >> 2) {
        if core.ar(sim) != 0 {
            core.set_ar(idx_val, core.ar(clock_event));
        }
    }
}
pub fn a_handler17(core: &mut CoreState, tmp_val: u32) {
    let idx_val = (tmp_val >> 12) & 15;
    let clock_event = (tmp_val >> 8) & 15;
    let sim = (tmp_val >> 4) & 15;
    if !core.window_check(0, 0, sim >> 2) {
        if core.ar(sim) != 0 {
            core.float_registers[idx_val as usize] = core.float_registers[clock_event as usize];
        }
    }
}
pub fn a_handler18(core: &mut CoreState, tmp_val: u32) {
    let idx_val = (tmp_val >> 8) & 15;
    let clock_event = (tmp_val >> 4) & 15;
    if core.window_check(0, idx_val >> 2, clock_event >> 2) { return; }
    let sim = core.special_registers[CACHE_CONTROL];
    let rr = core.special_registers[MEM_FAULT_INFO];
    if (sim & (1 << ((rr.wrapping_sub(1)) & 15))) != 0
        || (sim & (1 << ((rr.wrapping_sub(2)) & 15))) != 0
        || (sim & (1 << ((rr.wrapping_sub(3)) & 15))) != 0 {
        core.set_ar(clock_event, core.ar(idx_val));
    } else {
        core.exception(TRAP_ALLOCA);
    }
}
pub fn a_handler19(core: &mut CoreState, tmp_val: u32) {
    let idx_val = (tmp_val >> 12) & 15;
    let clock_event = (tmp_val >> 8) & 15;
    let sim = (tmp_val >> 4) & 15;
    if !core.window_check(idx_val >> 2, clock_event >> 2, 0) {
        if core.br(sim) { core.set_ar(idx_val, core.ar(clock_event)); }
    }
}
pub fn a_handler20(core: &mut CoreState, tmp_val: u32) {
    let idx_val = (tmp_val >> 12) & 15;
    let clock_event = (tmp_val >> 8) & 15;
    let sim = (tmp_val >> 4) & 15;
    if core.br(sim) { core.float_registers[idx_val as usize] = core.float_registers[clock_event as usize]; }
}
pub fn a_handler21(core: &mut CoreState, tmp_val: u32) {
    let idx_val = (tmp_val >> 12) & 15;
    let clock_event = (tmp_val >> 8) & 15;
    let sim = (tmp_val >> 4) & 15;
    core.float_registers[idx_val as usize] = f32_to_bits(
        f32_from_bits(core.float_registers[idx_val as usize]) -
        f32_from_bits(core.float_registers[clock_event as usize]) * f32_from_bits(core.float_registers[sim as usize])
    );
}
pub fn a_handler22(core: &mut CoreState, tmp_val: u32) {
    let idx_val = (tmp_val >> 16) & 3;
    let clock_event = (tmp_val >> 8) & 15;
    let sim = (tmp_val >> 4) & 15;
    if core.window_check(0, clock_event >> 2, sim >> 2) { return; }
    let rr = if (idx_val & 1) != 0 { core.ar(clock_event) >> 16 } else { 65535 & core.ar(clock_event) };
    let wa = if (idx_val & 2) != 0 { core.ar(sim) >> 16 } else { 65535 & core.ar(sim) };
    core.set_acc((n_handler3(rr) as u64).wrapping_mul(n_handler3(wa) as u64));
}
pub fn a_handler23(core: &mut CoreState, tmp_val: u32) {
    let idx_val = (tmp_val >> 16) & 3;
    let clock_event = (tmp_val >> 8) & 15;
    let sim = (tmp_val >> 6) & 1;
    if core.window_check(0, clock_event >> 2, 0) { return; }
    let rr = if sim != 0 { core.special_registers[INTERRUPT_SET] } else { core.special_registers[INTERRUPT_CLEAR] };
    let wa = if (idx_val & 1) != 0 { core.ar(clock_event) >> 16 } else { 65535 & core.ar(clock_event) };
    let rt = if (idx_val & 2) != 0 { rr >> 16 } else { 65535 & rr };
    core.set_acc((n_handler3(wa) as u64).wrapping_mul(n_handler3(rt) as u64));
}
pub fn a_handler24(core: &mut CoreState, tmp_val: u32) {
    let idx_val = (tmp_val >> 16) & 3;
    let clock_event = (tmp_val >> 14) & 1;
    let sim = (tmp_val >> 4) & 15;
    if core.window_check(0, 0, sim >> 2) { return; }
    let rr = if clock_event != 0 { core.special_registers[INTERRUPT_ENABLE] } else { core.special_registers[INTERRUPT_STATE] };
    let wa = if (idx_val & 1) != 0 { rr >> 16 } else { 65535 & rr };
    let rt = if (idx_val & 2) != 0 { core.ar(sim) >> 16 } else { 65535 & core.ar(sim) };
    core.set_acc((n_handler3(wa) as u64).wrapping_mul(n_handler3(rt) as u64));
}
pub fn a_handler25(core: &mut CoreState, tmp_val: u32) {
    let idx_val = (tmp_val >> 16) & 3;
    let clock_event = (tmp_val >> 14) & 1;
    let sim = (tmp_val >> 6) & 1;
    let rr = if clock_event != 0 { core.special_registers[INTERRUPT_ENABLE] } else { core.special_registers[INTERRUPT_STATE] };
    let wa = if sim != 0 { core.special_registers[INTERRUPT_SET] } else { core.special_registers[INTERRUPT_CLEAR] };
    let rt = if (idx_val & 1) != 0 { rr >> 16 } else { 65535 & rr };
    let cfg = if (idx_val & 2) != 0 { wa >> 16 } else { 65535 & wa };
    core.set_acc((n_handler3(rt) as u64).wrapping_mul(n_handler3(cfg) as u64));
}
pub fn a_handler26(core: &mut CoreState, tmp_val: u32) {
    let idx_val = (tmp_val >> 12) & 15;
    let clock_event = (tmp_val >> 8) & 15;
    let sim = (tmp_val >> 4) & 15;
    core.float_registers[idx_val as usize] = f32_to_bits(
        f32_from_bits(core.float_registers[clock_event as usize]) *
        f32_from_bits(core.float_registers[sim as usize])
    );
}
pub fn a_handler27(core: &mut CoreState, tmp_val: u32) {
    let idx_val = (tmp_val >> 12) & 15;
    let clock_event = (tmp_val >> 8) & 15;
    let sim = (tmp_val >> 4) & 15;
    if !core.window_check(idx_val >> 2, clock_event >> 2, sim >> 2) {
        core.set_ar(idx_val, n_handler3(65535 & core.ar(clock_event)).wrapping_mul(n_handler3(65535 & core.ar(sim))));
    }
}
pub fn a_handler28(core: &mut CoreState, tmp_val: u32) {
    let idx_val = (tmp_val >> 12) & 15;
    let clock_event = (tmp_val >> 8) & 15;
    let sim = (tmp_val >> 4) & 15;
    if !core.window_check(idx_val >> 2, clock_event >> 2, sim >> 2) {
        core.set_ar(idx_val, (65535 & core.ar(clock_event)).wrapping_mul(65535 & core.ar(sim)));
    }
}
pub fn a_handler29(core: &mut CoreState, tmp_val: u32) {
    let idx_val = (tmp_val >> 16) & 3;
    let clock_event = (tmp_val >> 8) & 15;
    let sim = (tmp_val >> 4) & 15;
    if core.window_check(0, clock_event >> 2, sim >> 2) { return; }
    let rr = if (idx_val & 1) != 0 { core.ar(clock_event) >> 16 } else { 65535 & core.ar(clock_event) };
    let wa = if (idx_val & 2) != 0 { core.ar(sim) >> 16 } else { 65535 & core.ar(sim) };
    core.set_acc(core.acc().wrapping_add((n_handler3(rr) as u64).wrapping_mul(n_handler3(wa) as u64)));
}
pub fn a_handler30(core: &mut CoreState, tmp_val: u32) {
    let idx_val = (tmp_val >> 16) & 3;
    let clock_event = (tmp_val >> 8) & 15;
    let sim = (tmp_val >> 6) & 1;
    if core.window_check(0, clock_event >> 2, 0) { return; }
    let rr = if sim != 0 { core.special_registers[INTERRUPT_SET] } else { core.special_registers[INTERRUPT_CLEAR] };
    let wa = if (idx_val & 1) != 0 { core.ar(clock_event) >> 16 } else { 65535 & core.ar(clock_event) };
    let rt = if (idx_val & 2) != 0 { rr >> 16 } else { 65535 & rr };
    core.set_acc(core.acc().wrapping_add((n_handler3(wa) as u64).wrapping_mul(n_handler3(rt) as u64)));
}
pub fn a_handler31(core: &mut CoreState, tmp_val: u32) {
    let idx_val = (tmp_val >> 16) & 3;
    let clock_event = (tmp_val >> 14) & 1;
    let sim = (tmp_val >> 4) & 15;
    if core.window_check(0, 0, sim >> 2) { return; }
    let rr = if clock_event != 0 { core.special_registers[INTERRUPT_ENABLE] } else { core.special_registers[INTERRUPT_STATE] };
    let wa = if (idx_val & 1) != 0 { rr >> 16 } else { 65535 & rr };
    let rt = if (idx_val & 2) != 0 { core.ar(sim) >> 16 } else { 65535 & core.ar(sim) };
    core.set_acc(core.acc().wrapping_add((n_handler3(wa) as u64).wrapping_mul(n_handler3(rt) as u64)));
}
pub fn a_handler32(core: &mut CoreState, tmp_val: u32) {
    let idx_val = (tmp_val >> 16) & 3;
    let clock_event = (tmp_val >> 14) & 1;
    let sim = (tmp_val >> 12) & 3;
    let rr = (tmp_val >> 8) & 15;
    let wa = (tmp_val >> 4) & 15;
    if core.window_check(0, rr >> 2, wa >> 2) { return; }
    let rt = core.ar(rr).wrapping_sub(4);
    let cfg = if clock_event != 0 { core.special_registers[INTERRUPT_ENABLE] } else { core.special_registers[INTERRUPT_STATE] };
    let hv = if (idx_val & 1) != 0 { cfg >> 16 } else { 65535 & cfg };
    let ov = if (idx_val & 2) != 0 { core.ar(wa) >> 16 } else { 65535 & core.ar(wa) };
    core.set_acc(core.acc().wrapping_add((n_handler3(hv) as u64).wrapping_mul(n_handler3(ov) as u64)));
    core.set_ar(rr, rt);
    let mem_val = read_uint32(core, rt);
    write_special_register(core, INTERRUPT_STATE as u32 + sim, mem_val);
}
pub fn a_handler33(core: &mut CoreState, tmp_val: u32) {
    let idx_val = (tmp_val >> 16) & 3;
    let clock_event = (tmp_val >> 14) & 1;
    let sim = (tmp_val >> 12) & 3;
    let rr = (tmp_val >> 8) & 15;
    let wa = (tmp_val >> 4) & 15;
    if core.window_check(0, rr >> 2, wa >> 2) { return; }
    let rt = core.ar(rr).wrapping_add(4);
    let cfg = if clock_event != 0 { core.special_registers[INTERRUPT_ENABLE] } else { core.special_registers[INTERRUPT_STATE] };
    let hv = if (idx_val & 1) != 0 { cfg >> 16 } else { 65535 & cfg };
    let ov = if (idx_val & 2) != 0 { core.ar(wa) >> 16 } else { 65535 & core.ar(wa) };
    core.set_acc(core.acc().wrapping_add((n_handler3(hv) as u64).wrapping_mul(n_handler3(ov) as u64)));
    core.set_ar(rr, rt);
    let mem_val = read_uint32(core, rt);
    write_special_register(core, INTERRUPT_STATE as u32 + sim, mem_val);
}
pub fn a_handler34(core: &mut CoreState, tmp_val: u32) {
    let idx_val = (tmp_val >> 16) & 3;
    let clock_event = (tmp_val >> 14) & 1;
    let sim = (tmp_val >> 6) & 1;
    let rr = if clock_event != 0 { core.special_registers[INTERRUPT_ENABLE] } else { core.special_registers[INTERRUPT_STATE] };
    let wa = if sim != 0 { core.special_registers[INTERRUPT_SET] } else { core.special_registers[INTERRUPT_CLEAR] };
    let rt = if (idx_val & 1) != 0 { rr >> 16 } else { 65535 & rr };
    let cfg = if (idx_val & 2) != 0 { wa >> 16 } else { 65535 & wa };
    core.set_acc(core.acc().wrapping_add((n_handler3(rt) as u64).wrapping_mul(n_handler3(cfg) as u64)));
}
pub fn a_handler35(core: &mut CoreState, tmp_val: u32) {
    let idx_val = (tmp_val >> 16) & 3;
    let clock_event = (tmp_val >> 14) & 1;
    let sim = (tmp_val >> 12) & 3;
    let rr = (tmp_val >> 8) & 15;
    let wa = (tmp_val >> 6) & 1;
    if core.window_check(0, rr >> 2, 0) { return; }
    let rt = core.ar(rr).wrapping_sub(4);
    let cfg = if clock_event != 0 { core.special_registers[INTERRUPT_ENABLE] } else { core.special_registers[INTERRUPT_STATE] };
    let hv = if wa != 0 { core.special_registers[INTERRUPT_SET] } else { core.special_registers[INTERRUPT_CLEAR] };
    let ov = if (idx_val & 1) != 0 { cfg >> 16 } else { 65535 & cfg };
    let sd = if (idx_val & 2) != 0 { hv >> 16 } else { 65535 & hv };
    core.set_acc(core.acc().wrapping_add((n_handler3(ov) as u64).wrapping_mul(n_handler3(sd) as u64)));
    core.set_ar(rr, rt);
    let mem_val = read_uint32(core, rt);
    write_special_register(core, INTERRUPT_STATE as u32 + sim, mem_val);
}
pub fn a_handler36(core: &mut CoreState, tmp_val: u32) {
    let idx_val = (tmp_val >> 16) & 3;
    let clock_event = (tmp_val >> 14) & 1;
    let sim = (tmp_val >> 12) & 3;
    let rr = (tmp_val >> 8) & 15;
    let wa = (tmp_val >> 6) & 1;
    if core.window_check(0, rr >> 2, 0) { return; }
    let rt = core.ar(rr).wrapping_add(4);
    let cfg = if clock_event != 0 { core.special_registers[INTERRUPT_ENABLE] } else { core.special_registers[INTERRUPT_STATE] };
    let hv = if wa != 0 { core.special_registers[INTERRUPT_SET] } else { core.special_registers[INTERRUPT_CLEAR] };
    let ov = if (idx_val & 1) != 0 { cfg >> 16 } else { 65535 & cfg };
    let sd = if (idx_val & 2) != 0 { hv >> 16 } else { 65535 & hv };
    core.set_acc(core.acc().wrapping_add((n_handler3(ov) as u64).wrapping_mul(n_handler3(sd) as u64)));
    core.set_ar(rr, rt);
    let mem_val = read_uint32(core, rt);
    write_special_register(core, INTERRUPT_STATE as u32 + sim, mem_val);
}
pub fn a_handler37(core: &mut CoreState, tmp_val: u32) {
    let idx_val = (tmp_val >> 12) & 15;
    let clock_event = (tmp_val >> 8) & 15;
    let sim = (tmp_val >> 4) & 15;
    if !core.window_check(idx_val >> 2, clock_event >> 2, sim >> 2) {
        let a = core.ar(clock_event);
        let b = core.ar(sim);
        core.set_ar(idx_val, (a as i32).wrapping_mul(b as i32) as u32);
    }
}
pub fn a_handler38(core: &mut CoreState, tmp_val: u32) {
    let idx_val = (tmp_val >> 16) & 3;
    let clock_event = (tmp_val >> 8) & 15;
    let sim = (tmp_val >> 4) & 15;
    if core.window_check(0, clock_event >> 2, sim >> 2) { return; }
    let rr = if (idx_val & 1) != 0 { core.ar(clock_event) >> 16 } else { 65535 & core.ar(clock_event) };
    let wa = if (idx_val & 2) != 0 { core.ar(sim) >> 16 } else { 65535 & core.ar(sim) };
    core.set_acc(core.acc().wrapping_sub((n_handler3(rr) as u64).wrapping_mul(n_handler3(wa) as u64)));
}
pub fn a_handler39(core: &mut CoreState, tmp_val: u32) {
    let idx_val = (tmp_val >> 16) & 3;
    let clock_event = (tmp_val >> 8) & 15;
    let sim = (tmp_val >> 6) & 1;
    if core.window_check(0, clock_event >> 2, 0) { return; }
    let rr = if sim != 0 { core.special_registers[INTERRUPT_SET] } else { core.special_registers[INTERRUPT_CLEAR] };
    let wa = if (idx_val & 1) != 0 { core.ar(clock_event) >> 16 } else { 65535 & core.ar(clock_event) };
    let rt = if (idx_val & 2) != 0 { rr >> 16 } else { 65535 & rr };
    core.set_acc(core.acc().wrapping_sub((n_handler3(wa) as u64).wrapping_mul(n_handler3(rt) as u64)));
}
pub fn a_handler40(core: &mut CoreState, tmp_val: u32) {
    let idx_val = (tmp_val >> 16) & 3;
    let clock_event = (tmp_val >> 14) & 1;
    let sim = (tmp_val >> 4) & 15;
    if core.window_check(0, 0, sim >> 2) { return; }
    let rr = if clock_event != 0 { core.special_registers[INTERRUPT_ENABLE] } else { core.special_registers[INTERRUPT_STATE] };
    let wa = if (idx_val & 1) != 0 { rr >> 16 } else { 65535 & rr };
    let rt = if (idx_val & 2) != 0 { core.ar(sim) >> 16 } else { 65535 & core.ar(sim) };
    core.set_acc(core.acc().wrapping_sub((n_handler3(wa) as u64).wrapping_mul(n_handler3(rt) as u64)));
}
pub fn a_handler41(core: &mut CoreState, tmp_val: u32) {
    let idx_val = (tmp_val >> 16) & 3;
    let clock_event = (tmp_val >> 14) & 1;
    let sim = (tmp_val >> 6) & 1;
    let rr = if clock_event != 0 { core.special_registers[INTERRUPT_ENABLE] } else { core.special_registers[INTERRUPT_STATE] };
    let wa = if sim != 0 { core.special_registers[INTERRUPT_SET] } else { core.special_registers[INTERRUPT_CLEAR] };
    let rt = if (idx_val & 1) != 0 { rr >> 16 } else { 65535 & rr };
    let cfg = if (idx_val & 2) != 0 { wa >> 16 } else { 65535 & wa };
    core.set_acc(core.acc().wrapping_sub((n_handler3(rt) as u64).wrapping_mul(n_handler3(cfg) as u64)));
}
pub fn a_handler42(core: &mut CoreState, tmp_val: u32) {
    let idx_val = (tmp_val >> 12) & 15;
    let clock_event = (tmp_val >> 8) & 15;
    let sim = (tmp_val >> 4) & 15;
    if core.window_check(idx_val >> 2, clock_event >> 2, sim >> 2) { return; }
    let rr = core.ar(clock_event) as i32;
    let wa = core.ar(sim) as i32;
    let rt = if rr >= 0 { rr as u32 } else { rr.wrapping_neg() as u32 };
    let cfg = if wa >= 0 { wa as u32 } else { wa.wrapping_neg() as u32 };
    let hv = rt >> 16;
    let ov = cfg >> 16;
    if hv != 0 || ov != 0 {
        let t = (65535 & rt) as u64;
        let ce = (65535 & cfg) as u64;
        let sc = (hv as u64) * ce + t * (ov as u64) + ((t * ce) >> 16);
        let lv = if sc > 0xffffffff { 65536 } else { 0 };
        let vv = ((hv as u64) * (ov as u64) + (sc >> 16) + lv) as u32;
        let negate = (rr < 0 && wa > 0) || (rr > 0 && wa < 0);
        core.set_ar(idx_val, if negate { !vv } else { vv });
    } else {
        core.set_ar(idx_val, if (rr as i64).wrapping_mul(wa as i64) < 0 { 0xffffffff } else { 0 });
    }
}
pub fn a_handler43(core: &mut CoreState, tmp_val: u32) {
    let idx_val = (tmp_val >> 12) & 15;
    let clock_event = (tmp_val >> 8) & 15;
    let sim = (tmp_val >> 4) & 15;
    if core.window_check(idx_val >> 2, clock_event >> 2, sim >> 2) { return; }
    let rr = core.ar(clock_event);
    let wa = core.ar(sim);
    let rt = rr >> 16;
    let cfg = wa >> 16;
    let hv = 65535 & rr;
    let ov = 65535 & wa;
    let sd = (rt as u64) * (ov as u64) + (hv as u64) * (cfg as u64) + (((hv as u64) * (ov as u64)) >> 16);
    let it = if sd > 0xffffffff { 65536 } else { 0 };
    core.set_ar(idx_val, ((rt as u64) * (cfg as u64) + (sd >> 16) + it) as u32);
}
pub fn a_handler44(core: &mut CoreState, tmp_val: u32) {
    let idx_val = (tmp_val >> 12) & 15;
    let clock_event = (tmp_val >> 4) & 15;
    if !core.window_check(idx_val >> 2, 0, clock_event >> 2) {
        core.set_ar(idx_val, (!core.ar(clock_event)).wrapping_add(1));
    }
}
pub fn a_handler45(core: &mut CoreState, tmp_val: u32) {
    let idx_val = (tmp_val >> 12) & 15;
    let clock_event = (tmp_val >> 8) & 15;
    core.float_registers[idx_val as usize] = f32_to_bits(-f32_from_bits(core.float_registers[clock_event as usize]));
}
pub fn a_handler46(core: &mut CoreState, tmp_val: u32) {
    let idx_val = (tmp_val >> 8) & 15;
    let clock_event = (tmp_val >> 4) & 15;
    if core.window_check(0, idx_val >> 2, clock_event >> 2) { return; }
    let sim = core.ar(idx_val) as i32;
    let rr = if sim < 0 { (!(sim as u32)).wrapping_add(1) } else { sim as u32 };
    let wa = if rr == 0 { 32 } else { rr.leading_zeros() };
    core.set_ar(clock_event, wa.wrapping_sub(1));
}
pub fn a_handler47(core: &mut CoreState, tmp_val: u32) {
    let idx_val = (tmp_val >> 8) & 15;
    let clock_event = (tmp_val >> 4) & 15;
    if core.window_check(0, idx_val >> 2, clock_event >> 2) { return; }
    let sim = core.ar(idx_val);
    if sim != 0 {
        let t = if (0xffff0000 & sim) == 0 { 16 } else { 0 };
        let i = if t != 0 { 65535 & sim } else { (sim >> 16) & 65535 };
        let rr = if (65280 & i) == 0 { 8 } else { 0 };
        let wa = if rr != 0 { 255 & i } else { (i >> 8) & 255 };
        let rt = if (240 & wa) == 0 { 4 } else { 0 };
        let tm = if rt != 0 { 15 & wa } else { (wa >> 4) & 15 };
        let hv = if (12 & tm) == 0 { 2 } else { 0 };
        let ov = if hv != 0 { if (2 & tm) == 0 { 1 } else { 0 } } else { if (8 & tm) == 0 { 1 } else { 0 } };
        core.set_ar(clock_event, t | rr | rt | hv | ov);
    } else {
        core.set_ar(clock_event, 32);
    }
}
pub fn a_handler48(core: &mut CoreState, tmp_val: u32) {
    let idx_val = (tmp_val >> 12) & 15;
    let clock_event = (tmp_val >> 8) & 15;
    let sim = (tmp_val >> 4) & 15;
    let rr = f32_from_bits(core.float_registers[clock_event as usize]);
    let wa = f32_from_bits(core.float_registers[sim as usize]);
    core.set_br(idx_val, !rr.is_nan() && !wa.is_nan() && rr == wa);
}
pub fn a_handler49(core: &mut CoreState, tmp_val: u32) {
    let idx_val = (tmp_val >> 12) & 15;
    let clock_event = (tmp_val >> 8) & 15;
    let sim = (tmp_val >> 4) & 15;
    let rr = f32_from_bits(core.float_registers[clock_event as usize]);
    let wa = f32_from_bits(core.float_registers[sim as usize]);
    core.set_br(idx_val, !rr.is_nan() && !wa.is_nan() && rr <= wa);
}
pub fn a_handler50(core: &mut CoreState, tmp_val: u32) {
    let idx_val = (tmp_val >> 12) & 15;
    let clock_event = (tmp_val >> 8) & 15;
    let sim = (tmp_val >> 4) & 15;
    let rr = f32_from_bits(core.float_registers[clock_event as usize]);
    let wa = f32_from_bits(core.float_registers[sim as usize]);
    core.set_br(idx_val, !rr.is_nan() && !wa.is_nan() && rr < wa);
}
pub fn a_handler51(core: &mut CoreState, tmp_val: u32) {
    let idx_val = (tmp_val >> 12) & 15;
    let clock_event = (tmp_val >> 8) & 15;
    let sim = (tmp_val >> 4) & 15;
    if !core.window_check(idx_val >> 2, clock_event >> 2, sim >> 2) {
        core.set_ar(idx_val, core.ar(clock_event) | core.ar(sim));
    }
}
pub fn a_handler52(core: &mut CoreState, tmp_val: u32) {
    let idx_val = (tmp_val >> 12) & 15;
    let clock_event = (tmp_val >> 8) & 15;
    let sim = (tmp_val >> 4) & 15;
    core.set_br(idx_val, core.br(clock_event) || core.br(sim));
}
pub fn a_handler53(core: &mut CoreState, tmp_val: u32) {
    let idx_val = (tmp_val >> 12) & 15;
    let clock_event = (tmp_val >> 8) & 15;
    let sim = (tmp_val >> 4) & 15;
    core.set_br(idx_val, core.br(clock_event) || !core.br(sim));
}
pub fn a_handler54(core: &mut CoreState, tmp_val: u32) { core.unimplemented(tmp_val); }
pub fn a_handler55(core: &mut CoreState, tmp_val: u32) { core.unimplemented(tmp_val); }
pub fn a_handler56(core: &mut CoreState, tmp_val: u32) {
    let idx_val = (tmp_val >> 12) & 15;
    let clock_event = (tmp_val >> 8) & 15;
    let sim = (tmp_val >> 4) & 15;
    if core.window_check(idx_val >> 2, clock_event >> 2, sim >> 2) { return; }
    let rr = core.ar(sim) as i32;
    if rr != 0 { core.set_ar(idx_val, (core.ar(clock_event) as i32 / rr) as u32); }
    else { core.exception(TRAP_INTEGER_DIVIDE_BY_ZERO); }
}
pub fn a_handler57(core: &mut CoreState, tmp_val: u32) {
    let idx_val = (tmp_val >> 12) & 15;
    let clock_event = (tmp_val >> 8) & 15;
    let sim = (tmp_val >> 4) & 15;
    if core.window_check(idx_val >> 2, clock_event >> 2, sim >> 2) { return; }
    let rr = core.ar(sim);
    if rr != 0 { core.set_ar(idx_val, core.ar(clock_event) / rr); }
    else { core.exception(TRAP_INTEGER_DIVIDE_BY_ZERO); }
}
pub fn a_handler58(core: &mut CoreState, tmp_val: u32) {
    let idx_val = (tmp_val >> 12) & 15;
    let clock_event = (tmp_val >> 8) & 15;
    let sim = (tmp_val >> 4) & 15;
    if core.window_check(idx_val >> 2, clock_event >> 2, sim >> 2) { return; }
    let rr = core.ar(sim) as i32;
    if rr != 0 { core.set_ar(idx_val, (core.ar(clock_event) as i32 % rr) as u32); }
    else { core.exception(TRAP_INTEGER_DIVIDE_BY_ZERO); }
}
pub fn a_handler59(core: &mut CoreState, tmp_val: u32) {
    let idx_val = (tmp_val >> 12) & 15;
    let clock_event = (tmp_val >> 8) & 15;
    let sim = (tmp_val >> 4) & 15;
    if core.window_check(idx_val >> 2, clock_event >> 2, sim >> 2) { return; }
    let rr = core.ar(sim);
    if rr != 0 { core.set_ar(idx_val, core.ar(clock_event) % rr); }
    else { core.exception(TRAP_INTEGER_DIVIDE_BY_ZERO); }
}
pub fn a_handler60(core: &mut CoreState, tmp_val: u32) {
    let idx_val = (tmp_val >> 8) & 15;
    let clock_event = (tmp_val >> 4) & 15;
    if !core.window_check(0, idx_val >> 2, clock_event >> 2) {
        if core.ps_owing() != 0 { core.exception(TRAP_PRIVILEGED); }
        else { core.set_ar(clock_event, 0); }
    }
}
pub fn a_handler61(core: &mut CoreState, _tmp_val: u32) { core.next_pc = core.ar(0); }
pub fn a_handler62(core: &mut CoreState, _tmp_val: u32) {
    let idx_val = core.ar(0);
    let clock_event = idx_val >> 30;
    core.next_pc = (0xc0000000 & core.pc) | (0x3fffffff & idx_val);
    let sim = core.special_registers[MEM_FAULT_INFO];
    let rr = core.special_registers[CACHE_CONTROL];
    let rt = if (rr & (1 << ((sim.wrapping_sub(1)) & 15))) != 0 { 1 }
             else if (rr & (1 << ((sim.wrapping_sub(2)) & 15))) != 0 { 2 }
             else if (rr & (1 << ((sim.wrapping_sub(3)) & 15))) != 0 { 3 }
             else { 0 };
    if super::exports::trace_return_active() {
        unsafe { super::exports::trace_return(core.index, core.pc, core.next_pc, core.ar(2)); }
    }
    if clock_event == 0 || (rt != 0 && rt != clock_event) || core.ps_woe() == 0 || core.ps_excm() != 0 {
        unsafe {
            static mut RWIL_N: u32 = 0;
            if RWIL_N < 4 {
                RWIL_N += 1;
                let hx = |mut v: u32, o: &mut [u8]| {
                    for i in 0..8 { o[7 - i] = b"0123456789abcdef"[(v & 0xF) as usize] as u8; v >>= 4; }
                };
                let mut m = [0u8; 256];
                let mut n = 0;
                for &b in b"[RWIL] pc=" { m[n] = b; n += 1; }
                let mut h = [0u8; 8]; hx(core.pc, &mut h); for &b in &h { m[n] = b; n += 1; }
                for &b in b" a0=" { m[n] = b; n += 1; } hx(idx_val, &mut h); for &b in &h { m[n] = b; n += 1; }
                for &b in b" ce=" { m[n] = b; n += 1; } hx(clock_event, &mut h); for &b in &h { m[n] = b; n += 1; }
                for &b in b" rt=" { m[n] = b; n += 1; } hx(rt, &mut h); for &b in &h { m[n] = b; n += 1; }
                for &b in b" woe=" { m[n] = b; n += 1; } hx(core.ps_woe(), &mut h); for &b in &h { m[n] = b; n += 1; }
                for &b in b" excm=" { m[n] = b; n += 1; } hx(core.ps_excm(), &mut h); for &b in &h { m[n] = b; n += 1; }
                for &b in b" il=" { m[n] = b; n += 1; } hx(core.ps_intlevel(), &mut h); for &b in &h { m[n] = b; n += 1; }
                for &b in b" sim=" { m[n] = b; n += 1; } hx(sim, &mut h); for &b in &h { m[n] = b; n += 1; }
                for &b in b" rr=" { m[n] = b; n += 1; } hx(rr, &mut h); for &b in &h { m[n] = b; n += 1; }
                for &b in b" xc=" { m[n] = b; n += 1; } hx(core.special_registers[crate::xtensa::constants::EXC_CAUSE], &mut h); for &b in &h { m[n] = b; n += 1; }
                for &b in b" ar5=" { m[n] = b; n += 1; } hx(core.ar(5), &mut h); for &b in &h { m[n] = b; n += 1; }
                for &b in b" ar8=" { m[n] = b; n += 1; } hx(core.ar(8), &mut h); for &b in &h { m[n] = b; n += 1; }
                crate::js_log_str(m.as_ptr() as u32, n as u32);
            }
        }
        core.exception(TRAP_ILLEGAL_INSTRUCTION);
    } else {
        let tmp = (sim.wrapping_sub(clock_event)) & 15;
        core.special_registers[MEM_FAULT_INFO] = tmp;
        if (rr & (1 << tmp)) != 0 {
            core.special_registers[CACHE_CONTROL] = rr & !(1 << sim);
        } else {
            core.set_ps_excm(1);
            core.special_registers[MISC_REGISTER] = core.pc;
            core.set_ps_owb(sim);
            core.next_pc = core.vector(
                if 1 == clock_event { REG_OFF_64 }
                else if 2 == clock_event { REG_OFF_192 }
                else { REG_OFF_320 }
            );
        }
    }
}
pub fn a_handler63(core: &mut CoreState, tmp_val: u32) { core.unimplemented(tmp_val); }

// _Handler functions
pub fn _handler0(core: &mut CoreState, tmp_val: u32) { core.unimplemented(tmp_val); }
pub fn _handler1(core: &mut CoreState, tmp_val: u32) { core.unimplemented(tmp_val); }
pub fn _handler2(core: &mut CoreState, _tmp_val: u32) {
    if core.ps_owing() != 0 { core.exception(TRAP_PRIVILEGED); }
    else {
        core.set_ps_excm(0);
        core.next_pc = core.special_registers[MISC_REGISTER];
        if super::exports::trace_return_active() {
            unsafe { super::exports::trace_return(core.index, core.pc, core.next_pc, 0xFFFFFFFF); }
        }
    }
}
pub fn _handler3(core: &mut CoreState, tmp_val: u32) {
    let idx_val = (tmp_val >> 8) & 15;
    if core.ps_owing() != 0 { core.exception(TRAP_PRIVILEGED); }
    else {
        core.next_pc = core.special_registers[MISC1_REGISTER + idx_val as usize - 2];
        // Self-loop canary (a +3 skip here crashed into linker data, see
        // 0x40083bcd IllegalInstruction reboot loop): an rfi whose EPC
        // equals its own address loops forever. There is no safe skip
        // target, so log only (counter-gated).
        if core.next_pc == core.pc {
            static mut RFISELF_LOGGED: u32 = 0;
            unsafe {
                if RFISELF_LOGGED < 4 {
                    RFISELF_LOGGED += 1;
                    let mut db = [0u8; 24];
                    let hx = |mut v: u32| -> [u8; 8] {
                        let mut o = [0u8; 8];
                        for i in (0..8).rev() { o[i] = b"0123456789abcdef"[(v & 0xF) as usize]; v >>= 4; }
                        o
                    };
                    let mut n = 0;
                    for &b in b"[RFISELF] " { db[n] = b; n += 1; }
                    for &b in &hx(core.pc) { if n < 24 { db[n] = b; n += 1; } }
                    crate::js_log_str(db.as_ptr() as u32, n as u32);
                }
            }
        }
        core.special_registers[INT_SET] = core.special_registers[DEPC_REGISTER + idx_val as usize - 2];
        // Diag: an rfi restore that clears WOE precedes the retw-illegal
        // crash signature; keep while BT/BLE injection is under test.
        if core.ps_woe() == 0 {
            unsafe {
                static mut RFI0_N: u32 = 0;
                if RFI0_N < 8 {
                    RFI0_N += 1;
                    let mut m = [0u8; 48];
                    let hx = |mut v: u32, o: &mut [u8]| {
                        for i in 0..8 { o[7 - i] = b"0123456789abcdef"[(v & 0xF) as usize] as u8; v >>= 4; }
                    };
                    let mut h = [0u8; 8];
                    let mut n = 0;
                    for &b in b"[RFI0] pc=" { m[n] = b; n += 1; }
                    hx(core.pc, &mut h); for &b in &h { m[n] = b; n += 1; }
                    for &b in b" idx=" { m[n] = b; n += 1; } hx(idx_val, &mut h); for &b in &h { m[n] = b; n += 1; }
                    crate::js_log_str(m.as_ptr() as u32, n as u32);
                }
            }
        }
        // HW-exact PS restore (PS <- EPS_level saved by take_interrupt /
        // exception). Without this, ISR levels whose stubs lack an explicit
        // wsr.ps (e.g. level-4) leave INTLEVEL stuck and wedge all
        // level<=INTLEVEL interrupts forever.
        core.special_registers[PS_REGISTER] = core.special_registers[184 + idx_val as usize - 1];
        // Advance pc past the rfi BEFORE update_interrupts(): on HW the
        // PS-restore + jump are atomic, so a nested vector saves the
        // post-rfi address as EPC. Without this, a still-pending level
        // (e.g. BT RW sources mid-batch, before the pump clears the pulse)
        // vectors with pc still at the rfi, clobbering that level's EPC
        // slot to the rfi address itself -> permanent rfi-to-self loop
        // (observed at 0x40083bca). Safe: if no vector fires, the main
        // loop's pc = next_pc assignment is a no-op.
        core.pc = core.next_pc;
        core.update_interrupts();
        if super::exports::trace_return_active() {
            unsafe { super::exports::trace_return(core.index, core.pc, core.next_pc, idx_val); }
        }
    }
}
pub fn _handler4(core: &mut CoreState, tmp_val: u32) { core.unimplemented(tmp_val); }
pub fn _handler5(core: &mut CoreState, tmp_val: u32) {
    let idx_val = (tmp_val >> 12) & 15;
    let clock_event = (tmp_val >> 8) & 15;
    if !core.window_check(idx_val >> 2, 0, 0) {
        core.set_ar(idx_val, core.float_registers[clock_event as usize]);
    }
}
pub fn _handler6(core: &mut CoreState, tmp_val: u32) { core.unimplemented(tmp_val); }
pub fn _handler7(core: &mut CoreState, _tmp_val: u32) {
    if core.ps_owing() != 0 { core.exception(TRAP_PRIVILEGED); }
    else {
        let tmp = core.special_registers[MEM_FAULT_INFO];
        core.set_ps_excm(0);
        core.next_pc = core.special_registers[MISC_REGISTER];
        core.special_registers[CACHE_CONTROL] &= !(1 << tmp);
        core.special_registers[MEM_FAULT_INFO] = core.ps_owb();
    }
}
pub fn _handler8(core: &mut CoreState, _tmp_val: u32) {
    if core.ps_owing() != 0 { core.exception(TRAP_PRIVILEGED); }
    else {
        let tmp = core.special_registers[MEM_FAULT_INFO];
        core.set_ps_excm(0);
        core.next_pc = core.special_registers[MISC_REGISTER];
        core.special_registers[CACHE_CONTROL] |= 1 << tmp;
        core.special_registers[MEM_FAULT_INFO] = core.ps_owb();
    }
}
pub fn _handler9(core: &mut CoreState, tmp_val: u32) { core.unimplemented(tmp_val); }
pub fn _handler10(core: &mut CoreState, tmp_val: u32) { core.unimplemented(tmp_val); }
pub fn _handler11(core: &mut CoreState, tmp_val: u32) {
    let idx_val = (tmp_val >> 4) & 15;
    if core.ps_owing() != 0 { core.exception(TRAP_PRIVILEGED); }
    else {
        core.special_registers[MEM_FAULT_INFO] = core.special_registers[MEM_FAULT_INFO].wrapping_add(n_handler0(idx_val)) & 15;
    }
}
pub fn _handler12(core: &mut CoreState, tmp_val: u32) {
    let idx_val = (tmp_val >> 12) & 15;
    let clock_event = (tmp_val >> 8) & 15;
    let sim = (tmp_val >> 4) & 15;
    if !core.window_check(idx_val >> 2, 0, 0) {
        let f = f32_from_bits(core.float_registers[clock_event as usize]);
        core.set_ar(idx_val, f32_round(f * f32_powi(2.0f32, sim as i32)) as u32);
    }
}
pub fn _handler13(core: &mut CoreState, tmp_val: u32) {
    let idx_val = (tmp_val >> 8) & 15;
    let clock_event = (tmp_val >> 4) & 15;
    if !core.window_check(0, 0, clock_event >> 2) {
        if core.ps_owing() != 0 { core.exception(TRAP_PRIVILEGED); }
        else {
            core.set_ar(clock_event, core.special_registers[INT_SET]);
            core.set_ps_intlevel(idx_val);
        }
    }
}
pub fn _handler14(core: &mut CoreState, tmp_val: u32) {
    let idx_val = (tmp_val >> 8) & 255;
    let clock_event = (tmp_val >> 4) & 15;
    if !core.window_check(0, 0, clock_event >> 2) {
        if idx_val >= 64 && core.ps_owing() != 0 { core.exception(TRAP_PRIVILEGED); }
        else { let val = read_special_register(core, idx_val); core.set_ar(clock_event, val); }
    }
}
pub fn _handler15(core: &mut CoreState, tmp_val: u32) {
    let idx_val = (tmp_val >> 12) & 15;
    let clock_event = (tmp_val >> 8) & 15;
    let sim = (tmp_val >> 4) & 15;
    if !core.window_check(idx_val >> 2, 0, 0) {
        core.set_ar(idx_val, core.user_registers[((clock_event << 4) | sim) as usize]);
    }
}
pub fn _handler16(core: &mut CoreState, tmp_val: u32) {
    let idx_val = (tmp_val >> 16) & 255;
    let clock_event = (tmp_val >> 8) & 15;
    let sim = (tmp_val >> 4) & 15;
    if core.window_check(0, clock_event >> 2, sim >> 2) { return; }
    let rr = core.ar(clock_event).wrapping_add(idx_val);
    write_uint8(core, rr, core.ar(sim), 1);
}
pub fn _handler17(core: &mut CoreState, tmp_val: u32) {
    let idx_val = (tmp_val >> 16) & 255;
    let clock_event = (tmp_val >> 8) & 15;
    let sim = (tmp_val >> 4) & 15;
    if core.window_check(0, clock_event >> 2, sim >> 2) { return; }
    let rr = core.ar(clock_event).wrapping_add(idx_val << 1);
    write_uint16(core, rr, core.ar(sim), 1);
}
pub fn _handler18(core: &mut CoreState, tmp_val: u32) {
    let idx_val = (tmp_val >> 16) & 255;
    let clock_event = (tmp_val >> 8) & 15;
    let sim = (tmp_val >> 4) & 15;
    if core.window_check(0, clock_event >> 2, sim >> 2) { return; }
    let rr = core.ar(clock_event).wrapping_add(idx_val << 2);
    let wa = read_uint32(core, rr);
    if wa != core.special_registers[SAR_REGISTER] || write_uint32(core, rr, core.ar(sim), 1) != 0 {
        core.set_ar(sim, wa);
    }
}
pub fn _handler19(core: &mut CoreState, tmp_val: u32) {
    let idx_val = (tmp_val >> 12) & 15;
    let clock_event = (tmp_val >> 8) & 15;
    let sim = (tmp_val >> 4) & 15;
    if core.ps_owing() != 0 { core.exception(TRAP_PRIVILEGED); }
    else {
        let tmp = core.ar(clock_event).wrapping_add(0xffffffc0 | (idx_val << 2));
        write_uint32(core, tmp, core.ar(sim), 1);
    }
}
pub fn _handler20(core: &mut CoreState, tmp_val: u32) {
    let idx_val = (tmp_val >> 16) & 255;
    let clock_event = (tmp_val >> 8) & 15;
    let sim = (tmp_val >> 4) & 15;
    if core.window_check(0, clock_event >> 2, sim >> 2) { return; }
    write_uint32(core, core.ar(clock_event).wrapping_add(idx_val << 2), core.ar(sim), 1);
}
pub fn _handler21(core: &mut CoreState, tmp_val: u32) {
    let idx_val = (tmp_val >> 12) & 15;
    let clock_event = (tmp_val >> 8) & 15;
    let sim = (tmp_val >> 4) & 15;
    if !core.window_check(0, clock_event >> 2, sim >> 2) {
        write_uint32(core, core.ar(clock_event).wrapping_add(idx_val << 2), core.ar(sim), 1);
    }
}
pub fn _handler22(core: &mut CoreState, tmp_val: u32) {
    let idx_val = (tmp_val >> 16) & 255;
    let clock_event = (tmp_val >> 8) & 15;
    let sim = (tmp_val >> 4) & 15;
    if core.window_check(0, clock_event >> 2, sim >> 2) { return; }
    write_uint32(core, core.ar(clock_event).wrapping_add(idx_val << 2), core.ar(sim), 1);
}
pub fn _handler23(core: &mut CoreState, tmp_val: u32) {
    let idx_val = (tmp_val >> 12) & 15;
    let clock_event = (tmp_val >> 8) & 15;
    let sim = (tmp_val >> 4) & 15;
    if !core.window_check(idx_val >> 2, clock_event >> 2, sim >> 2) {
        core.set_ar(idx_val, if (core.ar(clock_event) as i32) < (core.ar(sim) as i32) { 1 } else { 0 });
    }
}
pub fn _handler24(core: &mut CoreState, tmp_val: u32) {
    let idx_val = (tmp_val >> 12) & 15;
    let clock_event = (tmp_val >> 8) & 15;
    let sim = (tmp_val >> 4) & 15;
    if !core.window_check(idx_val >> 2, clock_event >> 2, sim >> 2) {
        core.set_ar(idx_val, if core.ar(clock_event) < core.ar(sim) { 1 } else { 0 });
    }
}
pub fn _handler25(core: &mut CoreState, tmp_val: u32) { core.unimplemented(tmp_val); }
pub fn _handler26(core: &mut CoreState, tmp_val: u32) {
    let idx_val = (tmp_val >> 12) & 15;
    let clock_event = (tmp_val >> 8) & 15;
    let sim = (tmp_val >> 4) & 15;
    if core.window_check(idx_val >> 2, clock_event >> 2, 0) { return; }
    let rr = sim + 7;
    let wa = core.ar(clock_event);
    if (wa & (1 << rr)) != 0 {
        core.set_ar(idx_val, wa | (0xffffffff << rr));
    } else {
        core.set_ar(idx_val, wa & !(0xffffffff << rr));
    }
}
pub fn _handler27(core: &mut CoreState, tmp_val: u32) { core.unimplemented(tmp_val); }
pub fn _handler28(core: &mut CoreState, tmp_val: u32) { core.unimplemented(tmp_val); }
pub fn _handler29(core: &mut CoreState, tmp_val: u32) { core.unimplemented(tmp_val); }
pub fn _handler30(core: &mut CoreState, tmp_val: u32) {
    let idx_val = (tmp_val >> 12) & 15;
    let clock_event = (tmp_val >> 8) & 15;
    if !core.window_check(idx_val >> 2, clock_event >> 2, 0) {
        core.set_ar(idx_val, core.ar(clock_event) << (32 - (63 & core.special_registers[EXC_CAUSE])));
    }
}
pub fn _handler31(core: &mut CoreState, tmp_val: u32) {
    let idx_val = (tmp_val >> 20) & 1;
    let clock_event = (tmp_val >> 12) & 15;
    let sim = (tmp_val >> 8) & 15;
    let rr = (idx_val << 4) | ((tmp_val >> 4) & 15);
    if !core.window_check(clock_event >> 2, sim >> 2, 0) {
        core.set_ar(clock_event, core.ar(sim) << (32 - rr));
    }
}
pub fn _handler32(core: &mut CoreState, tmp_val: u32) {
    let idx_val = (tmp_val >> 12) & 15;
    let clock_event = (tmp_val >> 8) & 15;
    core.float_registers[idx_val as usize] = f32_to_bits(f32_sqrt(f32_from_bits(core.float_registers[clock_event as usize])));
}
pub fn _handler33(core: &mut CoreState, tmp_val: u32) {
    let idx_val = (tmp_val >> 12) & 15;
    let clock_event = (tmp_val >> 8) & 15;
    core.float_registers[idx_val as usize] = f32_to_bits(1.0 / f32_from_bits(core.float_registers[clock_event as usize]));
}
pub fn _handler34(core: &mut CoreState, tmp_val: u32) {
    let idx_val = (tmp_val >> 12) & 15;
    let clock_event = (tmp_val >> 8) & 15;
    core.float_registers[idx_val as usize] = f32_to_bits(1.0 / f32_sqrt(f32_from_bits(core.float_registers[clock_event as usize])));
}
pub fn _handler35(core: &mut CoreState, tmp_val: u32) {
    let idx_val = (tmp_val >> 12) & 15;
    let clock_event = (tmp_val >> 4) & 15;
    if !core.window_check(idx_val >> 2, 0, clock_event >> 2) {
        core.set_ar(idx_val, (core.ar(clock_event) as i32 >> (63 & core.special_registers[EXC_CAUSE])) as u32);
    }
}
pub fn _handler36(core: &mut CoreState, tmp_val: u32) {
    let idx_val = (tmp_val >> 20) & 1;
    let clock_event = (tmp_val >> 12) & 15;
    let sim = (tmp_val >> 8) & 15;
    let rr = (tmp_val >> 4) & 15;
    let wa = (idx_val << 4) | sim;
    if !core.window_check(clock_event >> 2, 0, rr >> 2) {
        core.set_ar(clock_event, (core.ar(rr) as i32 >> wa) as u32);
    }
}
pub fn _handler37(core: &mut CoreState, tmp_val: u32) {
    let idx_val = (tmp_val >> 12) & 15;
    let clock_event = (tmp_val >> 8) & 15;
    let sim = (tmp_val >> 4) & 15;
    if core.window_check(idx_val >> 2, clock_event >> 2, sim >> 2) { return; }
    let rr = core.special_registers[EXC_CAUSE] & 63;
    let wa = core.ar(clock_event);
    let rt = core.ar(sim);
    core.set_ar(idx_val, if rr == 0 { rt }
        else if rr == 32 { wa }
        else { ((wa << (32 - rr)) | (rt >> rr)) & 0xffffffff });
}
pub fn _handler38(core: &mut CoreState, tmp_val: u32) {
    let idx_val = (tmp_val >> 12) & 15;
    let clock_event = (tmp_val >> 4) & 15;
    if !core.window_check(idx_val >> 2, 0, clock_event >> 2) {
        core.set_ar(idx_val, core.ar(clock_event) >> (63 & core.special_registers[EXC_CAUSE]));
    }
}
pub fn _handler39(core: &mut CoreState, tmp_val: u32) {
    let idx_val = (tmp_val >> 12) & 15;
    let clock_event = (tmp_val >> 8) & 15;
    let sim = (tmp_val >> 4) & 15;
    if !core.window_check(idx_val >> 2, 0, sim >> 2) {
        core.set_ar(idx_val, core.ar(sim) >> clock_event);
    }
}
pub fn _handler40(core: &mut CoreState, tmp_val: u32) {
    let idx_val = (tmp_val >> 8) & 15;
    if !core.window_check(0, idx_val >> 2, 0) {
        core.special_registers[EXC_CAUSE] = 32 - (((3 & core.ar(idx_val)) << 3) as i32) as u32;
    }
}
pub fn _handler41(core: &mut CoreState, tmp_val: u32) {
    let idx_val = (tmp_val >> 8) & 15;
    if !core.window_check(0, idx_val >> 2, 0) {
        core.special_registers[EXC_CAUSE] = (3 & core.ar(idx_val)) << 3;
    }
}
pub fn _handler42(core: &mut CoreState, tmp_val: u32) {
    let idx_val = (tmp_val >> 8) & 15;
    core.special_registers[EXC_CAUSE] = (((tmp_val >> 4) & 1) << 4) | idx_val;
    core.sar_m32_pending = 0;
}
pub fn _handler43(core: &mut CoreState, tmp_val: u32) {
    let idx_val = (tmp_val >> 16) & 255;
    let clock_event = (tmp_val >> 8) & 15;
    let sim = (tmp_val >> 4) & 15;
    if core.window_check(0, clock_event >> 2, 0) { return; }
    let rr = core.ar(clock_event).wrapping_add(idx_val << 2);
    write_uint32(core, rr, core.float_registers[sim as usize], 1);
}
pub fn _handler44(core: &mut CoreState, tmp_val: u32) {
    let idx_val = (tmp_val >> 16) & 255;
    let clock_event = (tmp_val >> 8) & 15;
    let sim = (tmp_val >> 4) & 15;
    if core.window_check(0, clock_event >> 2, 0) { return; }
    let rr = core.ar(clock_event).wrapping_add(idx_val << 2);
    if write_uint32(core, rr, core.float_registers[sim as usize], 1) != 0 {
        core.set_ar(clock_event, rr);
    }
}
pub fn _handler45(core: &mut CoreState, tmp_val: u32) {
    let idx_val = (tmp_val >> 8) & 15;
    if core.window_check(0, idx_val >> 2, 0) { return; }
    let ce = 31 & core.ar(idx_val);
    core.special_registers[EXC_CAUSE] = 32 - ce;
    core.sar_m32 = ce as i32;
    core.sar_m32_pending = 1;
}
pub fn _handler46(core: &mut CoreState, tmp_val: u32) {
    let idx_val = (tmp_val >> 8) & 15;
    if !core.window_check(0, idx_val >> 2, 0) {
        core.special_registers[EXC_CAUSE] = 31 & core.ar(idx_val);
        core.sar_m32_pending = 0;
    }
}
pub fn _handler47(core: &mut CoreState, tmp_val: u32) {
    let idx_val = (tmp_val >> 12) & 15;
    let clock_event = (tmp_val >> 8) & 15;
    let sim = (tmp_val >> 4) & 15;
    if core.window_check(0, clock_event >> 2, sim >> 2) { return; }
    let rr = core.ar(clock_event).wrapping_add(core.ar(sim));
    write_uint32(core, rr, core.float_registers[idx_val as usize], 1);
}
pub fn _handler48(core: &mut CoreState, tmp_val: u32) {
    let idx_val = (tmp_val >> 12) & 15;
    let clock_event = (tmp_val >> 8) & 15;
    let sim = (tmp_val >> 4) & 15;
    if core.window_check(0, clock_event >> 2, sim >> 2) { return; }
    let rr = core.ar(clock_event).wrapping_add(core.ar(sim));
    if write_uint32(core, rr, core.float_registers[idx_val as usize], 1) != 0 {
        core.set_ar(clock_event, rr);
    }
}
pub fn _handler49(core: &mut CoreState, tmp_val: u32) {
    let idx_val = (tmp_val >> 12) & 15;
    let clock_event = (tmp_val >> 8) & 15;
    let sim = (tmp_val >> 4) & 15;
    if !core.window_check(idx_val >> 2, clock_event >> 2, sim >> 2) {
        core.set_ar(idx_val, core.ar(clock_event).wrapping_sub(core.ar(sim)));
    }
}
pub fn _handler50(core: &mut CoreState, tmp_val: u32) {
    let idx_val = (tmp_val >> 12) & 15;
    let clock_event = (tmp_val >> 8) & 15;
    let sim = (tmp_val >> 4) & 15;
    core.float_registers[idx_val as usize] = f32_to_bits(
        f32_from_bits(core.float_registers[clock_event as usize]) -
        f32_from_bits(core.float_registers[sim as usize])
    );
}
pub fn _handler51(core: &mut CoreState, tmp_val: u32) {
    let idx_val = (tmp_val >> 12) & 15;
    let clock_event = (tmp_val >> 8) & 15;
    let sim = (tmp_val >> 4) & 15;
    if !core.window_check(idx_val >> 2, clock_event >> 2, sim >> 2) {
        core.set_ar(idx_val, (core.ar(clock_event) << 1).wrapping_sub(core.ar(sim)));
    }
}
pub fn _handler52(core: &mut CoreState, tmp_val: u32) {
    let idx_val = (tmp_val >> 12) & 15;
    let clock_event = (tmp_val >> 8) & 15;
    let sim = (tmp_val >> 4) & 15;
    if !core.window_check(idx_val >> 2, clock_event >> 2, sim >> 2) {
        core.set_ar(idx_val, (core.ar(clock_event) << 2).wrapping_sub(core.ar(sim)));
    }
}
pub fn _handler53(core: &mut CoreState, tmp_val: u32) {
    let idx_val = (tmp_val >> 12) & 15;
    let clock_event = (tmp_val >> 8) & 15;
    let sim = (tmp_val >> 4) & 15;
    if !core.window_check(idx_val >> 2, clock_event >> 2, sim >> 2) {
        core.set_ar(idx_val, (core.ar(clock_event) << 3).wrapping_sub(core.ar(sim)));
    }
}
pub fn _handler54(core: &mut CoreState, _tmp_val: u32) { core.exception(TRAP_SYSCALL); }
pub fn _handler55(core: &mut CoreState, tmp_val: u32) {
    let idx_val = (tmp_val >> 12) & 15;
    let clock_event = (tmp_val >> 8) & 15;
    let sim = (tmp_val >> 4) & 15;
    if !core.window_check(idx_val >> 2, 0, 0) {
        let f = f32_from_bits(core.float_registers[clock_event as usize]);
        core.set_ar(idx_val, f32_trunc(f * f32_powi(2.0f32, sim as i32)) as u32);
    }
}
pub fn _handler56(core: &mut CoreState, tmp_val: u32) {
    let idx_val = (tmp_val >> 12) & 15;
    let clock_event = (tmp_val >> 8) & 15;
    let sim = (tmp_val >> 4) & 15;
    let rr = f32_from_bits(core.float_registers[clock_event as usize]);
    let wa = f32_from_bits(core.float_registers[sim as usize]);
    core.set_br(idx_val, rr.is_nan() || wa.is_nan() || rr == wa);
}
pub fn _handler57(core: &mut CoreState, tmp_val: u32) {
    let idx_val = (tmp_val >> 12) & 15;
    let clock_event = (tmp_val >> 8) & 15;
    let sim = (tmp_val >> 4) & 15;
    if !core.window_check(0, clock_event >> 2, 0) {
        let val = (core.ar(clock_event) as f32) * f32_powi(2.0f32, -(sim as i32));
        core.float_registers[idx_val as usize] = f32_to_bits(val);
    }
}
pub fn _handler58(core: &mut CoreState, tmp_val: u32) {
    let idx_val = (tmp_val >> 12) & 15;
    let clock_event = (tmp_val >> 8) & 15;
    let sim = (tmp_val >> 4) & 15;
    let rr = f32_from_bits(core.float_registers[clock_event as usize]);
    let wa = f32_from_bits(core.float_registers[sim as usize]);
    core.set_br(idx_val, rr.is_nan() || wa.is_nan() || rr <= wa);
}
pub fn _handler59(core: &mut CoreState, tmp_val: u32) {
    let idx_val = (tmp_val >> 12) & 15;
    let clock_event = (tmp_val >> 8) & 15;
    let sim = (tmp_val >> 4) & 15;
    // unimplemented - noop
    let rr = f32_from_bits(core.float_registers[clock_event as usize]);
    let wa = f32_from_bits(core.float_registers[sim as usize]);
    core.set_br(idx_val, rr.is_nan() || wa.is_nan() || rr < wa);
}
pub fn _handler60(core: &mut CoreState, tmp_val: u32) {
    let idx_val = (tmp_val >> 16) & 3;
    let clock_event = (tmp_val >> 8) & 15;
    let sim = (tmp_val >> 4) & 15;
    if core.window_check(0, clock_event >> 2, sim >> 2) { return; }
    let rr = if (idx_val & 1) != 0 { core.ar(clock_event) >> 16 } else { 65535 & core.ar(clock_event) };
    let wa = if (idx_val & 2) != 0 { core.ar(sim) >> 16 } else { 65535 & core.ar(sim) };
    core.set_acc((rr as u64).wrapping_mul(wa as u64));
}
pub fn _handler61(core: &mut CoreState, tmp_val: u32) {
    let idx_val = (tmp_val >> 12) & 15;
    let clock_event = (tmp_val >> 8) & 15;
    let sim = (tmp_val >> 4) & 15;
    let rr = f32_from_bits(core.float_registers[clock_event as usize]);
    let wa = f32_from_bits(core.float_registers[sim as usize]);
    core.set_br(idx_val, rr.is_nan() || wa.is_nan());
}
pub fn _handler62(core: &mut CoreState, tmp_val: u32) {
    let idx_val = (tmp_val >> 12) & 15;
    let clock_event = (tmp_val >> 8) & 15;
    let sim = (tmp_val >> 4) & 15;
    let f = f32_from_bits(core.float_registers[clock_event as usize]);
    core.set_ar(idx_val, f32_trunc(f * f32_powi(2.0f32, sim as i32)) as u32);
}
pub fn _handler63(core: &mut CoreState, tmp_val: u32) {
    let idx_val = (tmp_val >> 8) & 15;
    if core.ps_owing() != 0 { core.exception(TRAP_PRIVILEGED); }
    else {
        core.next_pc = core.pc;
        core.idle = 1;
        core.set_ps_intlevel(idx_val);
    }
}
// cHandler functions
pub fn c_handler0(core: &mut CoreState, tmp_val: u32) { core.unimplemented(tmp_val); }
pub fn c_handler1(core: &mut CoreState, tmp_val: u32) {
    let idx_val = (tmp_val >> 12) & 15;
    let clock_event = (tmp_val >> 8) & 15;
    if !core.window_check(0, clock_event >> 2, 0) {
        core.float_registers[idx_val as usize] = core.ar(clock_event);
    }
}
pub fn c_handler2(core: &mut CoreState, tmp_val: u32) {
    let idx_val = (tmp_val >> 8) & 255;
    let clock_event = (tmp_val >> 4) & 15;
    if !core.window_check(0, 0, clock_event >> 2) {
        if idx_val >= 64 && core.ps_owing() != 0 { core.exception(TRAP_PRIVILEGED); }
        else { write_special_register(core, idx_val, core.ar(clock_event)); }
    }
}
pub fn c_handler3(core: &mut CoreState, tmp_val: u32) {
    let idx_val = (tmp_val >> 8) & 255;
    let clock_event = (tmp_val >> 4) & 15;
    if !core.window_check(0, 0, clock_event >> 2) {
        core.user_registers[idx_val as usize] = core.ar(clock_event);
    }
}
pub fn c_handler4(core: &mut CoreState, tmp_val: u32) {
    let idx_val = (tmp_val >> 12) & 15;
    let clock_event = (tmp_val >> 8) & 15;
    let sim = (tmp_val >> 4) & 15;
    if !core.window_check(idx_val >> 2, clock_event >> 2, sim >> 2) {
        core.set_ar(idx_val, core.ar(clock_event) ^ core.ar(sim));
    }
}
pub fn c_handler5(core: &mut CoreState, tmp_val: u32) {
    let idx_val = (tmp_val >> 12) & 15;
    let clock_event = (tmp_val >> 8) & 15;
    let sim = (tmp_val >> 4) & 15;
    core.set_br(idx_val, if core.br(clock_event) { !core.br(sim) } else { core.br(sim) });
}
pub fn c_handler6(core: &mut CoreState, tmp_val: u32) {
    let idx_val = (tmp_val >> 8) & 255;
    let clock_event = (tmp_val >> 4) & 15;
    if !core.window_check(0, 0, clock_event >> 2) {
        if idx_val >= 64 && core.ps_owing() != 0 { core.exception(TRAP_PRIVILEGED); }
        else {
            let tmp = core.ar(clock_event);
            let sim = read_special_register(core, idx_val);
            write_special_register(core, idx_val, tmp);
            core.set_ar(clock_event, sim);
        }
    }
}
