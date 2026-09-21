use super::constants::*;
use super::state::CoreState;
use super::exports::{read_uint8, read_uint16, read_uint32, write_uint8, write_uint16, write_uint32, write_special_register, read_special_register};

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

pub fn c_handler7(core: &mut CoreState, tmp_val: u32) {
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
                                                0 => { r_handler35(core, tmp_val); return; }
                                                128 => { a_handler61(core, tmp_val); return; }
                                                144 => { a_handler62(core, tmp_val); return; }
                                                160 => { r_handler40(core, tmp_val); return; }
                                                192 => { r_handler4(core, tmp_val); return; }
                                                208 => { r_handler5(core, tmp_val); return; }
                                                224 => { r_handler6(core, tmp_val); return; }
                                                240 => { r_handler7(core, tmp_val); return; }
                                                _ => {}
                                            }
                                        }
                                        4096 => { a_handler18(core, tmp_val); return; }
                                        8192 => {
                                            match tmp_val & 4080 {
                                                0 | 16 | 32 | 48 | 192 | 208 | 240 => { return; }
                                                128 => { r_handler25(core, tmp_val); return; }
                                                _ => {}
                                            }
                                        }
                                        12288 => {
                                            match tmp_val & 240 {
                                                0 => {
                                                    match tmp_val & 3840 {
                                                        0 => { _handler2(core, tmp_val); return; }
                                                        256 => { _handler6(core, tmp_val); return; }
                                                        512 => { _handler0(core, tmp_val); return; }
                                                        1024 => { _handler7(core, tmp_val); return; }
                                                        1280 => { _handler8(core, tmp_val); return; }
                                                        _ => {}
                                                    }
                                                }
                                                16 => { _handler3(core, tmp_val); return; }
                                                32 => { _handler4(core, tmp_val); return; }
                                                _ => {}
                                            }
                                        }
                                        16384 => { n_handler51(core, tmp_val); return; }
                                        20480 => {
                                            if (tmp_val & 4080) == 256 { _handler29(core, tmp_val); return; }
                                            if (tmp_val & 4080) == 0 { _handler54(core, tmp_val); return; }
                                        }
                                        24576 => { _handler13(core, tmp_val); return; }
                                        28672 => { _handler63(core, tmp_val); return; }
                                        32768 => { n_handler22(core, tmp_val); return; }
                                        36864 => { n_handler17(core, tmp_val); return; }
                                        40960 => { n_handler23(core, tmp_val); return; }
                                        45056 => { n_handler18(core, tmp_val); return; }
                                        _ => {}
                                    }
                                }
                                1048576 => { n_handler19(core, tmp_val); return; }
                                _ => {}
                            }
                        }
                        65536 => { _handler31(core, tmp_val); return; }
                        2097152 => {
                            if (tmp_val & 1048576) == 0 { a_handler51(core, tmp_val); return; }
                            if (tmp_val & 1048576) == 1048576 { c_handler4(core, tmp_val); return; }
                        }
                        2162688 => { _handler36(core, tmp_val); return; }
                        4194304 => {
                            match tmp_val & 1110016 {
                                0 => { _handler46(core, tmp_val); return; }
                                4096 => { _handler45(core, tmp_val); return; }
                                8192 => { _handler41(core, tmp_val); return; }
                                12288 => { _handler40(core, tmp_val); return; }
                                16384 => { _handler42(core, tmp_val); return; }
                                24576 => { a_handler60(core, tmp_val); return; }
                                28672 => { c_handler0(core, tmp_val); return; }
                                32768 => { _handler11(core, tmp_val); return; }
                                57344 => { a_handler46(core, tmp_val); return; }
                                61440 => { a_handler47(core, tmp_val); return; }
                                1060864 => { _handler9(core, tmp_val); return; }
                                1064960 => { r_handler33(core, tmp_val); return; }
                                1069056 => { a_handler55(core, tmp_val); return; }
                                1073152 | 1093632 | 1105920 | 1110016 => { return; }
                                1077248 => { _handler10(core, tmp_val); return; }
                                1097728 => { r_handler29(core, tmp_val); return; }
                                1101824 => { a_handler54(core, tmp_val); return; }
                                _ => {}
                            }
                        }
                        4259840 => { _handler39(core, tmp_val); return; }
                        6291456 => {
                            if (tmp_val & 1052416) == 256 { n_handler5(core, tmp_val); return; }
                            if (tmp_val & 1052416) == 0 { a_handler44(core, tmp_val); return; }
                        }
                        6356992 => { c_handler6(core, tmp_val); return; }
                        8388608 => {
                            if (tmp_val & 1048576) == 0 { n_handler7(core, tmp_val); return; }
                            if (tmp_val & 1048576) == 1048576 { n_handler14(core, tmp_val); return; }
                        }
                        8454144 => {
                            if (tmp_val & 1048576) == 0 { _handler37(core, tmp_val); return; }
                            if (tmp_val & 1052416) == 1048576 { _handler38(core, tmp_val); return; }
                        }
                        0xa00000 => {
                            if (tmp_val & 1048576) == 0 { n_handler15(core, tmp_val); return; }
                            if (tmp_val & 1048576) == 1048576 { n_handler16(core, tmp_val); return; }
                        }
                        0xa10000 => {
                            if (tmp_val & 1048816) == 0 { _handler30(core, tmp_val); return; }
                            if (tmp_val & 1052416) == 1048576 { _handler35(core, tmp_val); return; }
                        }
                        0xc00000 => {
                            if (tmp_val & 1048576) == 0 { _handler49(core, tmp_val); return; }
                            if (tmp_val & 1048576) == 1048576 { _handler51(core, tmp_val); return; }
                        }
                        0xc10000 => {
                            if (tmp_val & 1048576) == 1048576 { a_handler27(core, tmp_val); return; }
                            if (tmp_val & 1048576) == 0 { a_handler28(core, tmp_val); return; }
                        }
                        0xe00000 => {
                            if (tmp_val & 1048576) == 0 { _handler52(core, tmp_val); return; }
                            if (tmp_val & 1048576) == 1048576 { _handler53(core, tmp_val); return; }
                        }
                        0xe10000 => {
                            match tmp_val & 1110016 {
                                1048576 => { r_handler52(core, tmp_val); return; }
                                1052672 => { _handler27(core, tmp_val); return; }
                                1056768 => { r_handler53(core, tmp_val); return; }
                                1060864 => { _handler28(core, tmp_val); return; }
                                1081344 => { r_handler49(core, tmp_val); return; }
                                1085440 => { _handler25(core, tmp_val); return; }
                                1105920 => {
                                    if (tmp_val & 3824) == 16 { a_handler63(core, tmp_val); return; }
                                    if (tmp_val & 4080) == 0 { _handler1(core, tmp_val); return; }
                                }
                                _ => {}
                            }
                        }
                        _ => {}
                    }
                }
                131072 => {
                    match tmp_val & 0xf10000 {
                        0 => { n_handler20(core, tmp_val); return; }
                        65536 => { _handler14(core, tmp_val); return; }
                        1048576 => { n_handler21(core, tmp_val); return; }
                        1114112 => { c_handler2(core, tmp_val); return; }
                        2097152 => { a_handler52(core, tmp_val); return; }
                        2162688 => { _handler26(core, tmp_val); return; }
                        3145728 => { a_handler53(core, tmp_val); return; }
                        3211264 => { r_handler9(core, tmp_val); return; }
                        4194304 => { c_handler5(core, tmp_val); return; }
                        4259840 => { a_handler0(core, tmp_val); return; }
                        5308416 => { r_handler62(core, tmp_val); return; }
                        6291456 => { _handler24(core, tmp_val); return; }
                        6356992 => { a_handler1(core, tmp_val); return; }
                        7340032 => { _handler23(core, tmp_val); return; }
                        7405568 => { r_handler63(core, tmp_val); return; }
                        8388608 => { a_handler37(core, tmp_val); return; }
                        8454144 => { a_handler6(core, tmp_val); return; }
                        9502720 => { a_handler16(core, tmp_val); return; }
                        0xa00000 => { a_handler43(core, tmp_val); return; }
                        0xa10000 => { a_handler14(core, tmp_val); return; }
                        0xb00000 => { a_handler42(core, tmp_val); return; }
                        0xb10000 => { a_handler10(core, tmp_val); return; }
                        0xc00000 => { a_handler57(core, tmp_val); return; }
                        0xc10000 => { a_handler8(core, tmp_val); return; }
                        0xd00000 => { a_handler56(core, tmp_val); return; }
                        0xd10000 => { a_handler19(core, tmp_val); return; }
                        0xe00000 => { a_handler59(core, tmp_val); return; }
                        0xe10000 => { _handler15(core, tmp_val); return; }
                        0xf00000 => { a_handler58(core, tmp_val); return; }
                        0xf10000 => { c_handler3(core, tmp_val); return; }
                        _ => {}
                    }
                }
                262144 => { r_handler26(core, tmp_val); return; }
                524288 => {
                    match tmp_val & 0xf10000 {
                        0 => { r_handler59(core, tmp_val); return; }
                        65536 => { r_handler45(core, tmp_val); return; }
                        1048576 => { r_handler60(core, tmp_val); return; }
                        4194304 => { _handler47(core, tmp_val); return; }
                        4259840 => { _handler19(core, tmp_val); return; }
                        5242880 => { _handler48(core, tmp_val); return; }
                        _ => {}
                    }
                }
                655360 => {
                    match tmp_val & 0xf10000 {
                        0 => { n_handler10(core, tmp_val); return; }
                        1048576 => { _handler50(core, tmp_val); return; }
                        1114112 => { _handler61(core, tmp_val); return; }
                        2097152 => { a_handler26(core, tmp_val); return; }
                        2162688 => { a_handler48(core, tmp_val); return; }
                        3211264 => { _handler56(core, tmp_val); return; }
                        4194304 => { r_handler61(core, tmp_val); return; }
                        4259840 => { a_handler50(core, tmp_val); return; }
                        5242880 => { a_handler21(core, tmp_val); return; }
                        5308416 => { _handler59(core, tmp_val); return; }
                        6291456 | 7340032 => { return; }
                        6356992 => { a_handler49(core, tmp_val); return; }
                        7405568 => { _handler58(core, tmp_val); return; }
                        8388608 => { _handler12(core, tmp_val); return; }
                        8454144 => { a_handler7(core, tmp_val); return; }
                        9437184 => { _handler55(core, tmp_val); return; }
                        9502720 => { a_handler17(core, tmp_val); return; }
                        0xa00000 => { r_handler28(core, tmp_val); return; }
                        0xa10000 => { a_handler15(core, tmp_val); return; }
                        0xb00000 => { r_handler8(core, tmp_val); return; }
                        0xb10000 => { a_handler11(core, tmp_val); return; }
                        0xc00000 => { r_handler27(core, tmp_val); return; }
                        0xc10000 => { a_handler9(core, tmp_val); return; }
                        0xd00000 => { _handler57(core, tmp_val); return; }
                        0xd10000 => { a_handler20(core, tmp_val); return; }
                        0xe00000 => { _handler62(core, tmp_val); return; }
                        0xf00000 => {
                            match tmp_val & 240 {
                                0 => { a_handler5(core, tmp_val); return; }
                                16 => { n_handler6(core, tmp_val); return; }
                                48 => { r_handler10(core, tmp_val); return; }
                                64 => { _handler5(core, tmp_val); return; }
                                80 => { c_handler1(core, tmp_val); return; }
                                96 => { a_handler45(core, tmp_val); return; }
                                112 | 176 | 224 => { return; }
                                128 => { _handler33(core, tmp_val); return; }
                                144 => { _handler32(core, tmp_val); return; }
                                160 => { _handler34(core, tmp_val); return; }
                                192 => { a_handler3(core, tmp_val); return; }
                                208 => { a_handler2(core, tmp_val); return; }
                                240 => { n_handler8(core, tmp_val); return; }
                                _ => {}
                            }
                        }
                        _ => {}
                    }
                }
                _ => {}
            }
        }
        1 => { r_handler48(core, tmp_val); return; }
        2 => {
            match tmp_val & 61440 {
                0 => { r_handler41(core, tmp_val); return; }
                4096 => { r_handler43(core, tmp_val); return; }
                8192 => { r_handler46(core, tmp_val); return; }
                16384 => { _handler16(core, tmp_val); return; }
                20480 => { _handler17(core, tmp_val); return; }
                24576 => { _handler20(core, tmp_val); return; }
                28672 => {
                    match tmp_val & 240 {
                        0 => { r_handler20(core, tmp_val); return; }
                        16 => { r_handler22(core, tmp_val); return; }
                        32 => { r_handler21(core, tmp_val); return; }
                        48 => { r_handler23(core, tmp_val); return; }
                        64 => { r_handler13(core, tmp_val); return; }
                        80 => { r_handler14(core, tmp_val); return; }
                        96 => { r_handler11(core, tmp_val); return; }
                        112 => { r_handler15(core, tmp_val); return; }
                        128 => {
                            match tmp_val & 983040 {
                                0 => { r_handler19(core, tmp_val); return; }
                                131072 => { r_handler12(core, tmp_val); return; }
                                196608 => { r_handler16(core, tmp_val); return; }
                                262144 => { r_handler17(core, tmp_val); return; }
                                327680 => { r_handler18(core, tmp_val); return; }
                                _ => {}
                            }
                        }
                        192 => { r_handler37(core, tmp_val); return; }
                        208 => {
                            match tmp_val & 983040 {
                                0 => { r_handler38(core, tmp_val); return; }
                                131072 => { r_handler31(core, tmp_val); return; }
                                196608 => { r_handler34(core, tmp_val); return; }
                                _ => {}
                            }
                        }
                        224 => { r_handler30(core, tmp_val); return; }
                        240 => { r_handler32(core, tmp_val); return; }
                        _ => {}
                    }
                }
                36864 => { r_handler42(core, tmp_val); return; }
                40960 => { a_handler12(core, tmp_val); return; }
                45056 => { r_handler44(core, tmp_val); return; }
                49152 => { n_handler11(core, tmp_val); return; }
                53248 => { n_handler13(core, tmp_val); return; }
                57344 => { _handler18(core, tmp_val); return; }
                61440 => { _handler22(core, tmp_val); return; }
                _ => {}
            }
        }
        3 => {
            match tmp_val & 61440 {
                0 => { r_handler57(core, tmp_val); return; }
                16384 => { _handler43(core, tmp_val); return; }
                32768 => { r_handler58(core, tmp_val); return; }
                49152 => { _handler44(core, tmp_val); return; }
                _ => {}
            }
        }
        4 => {
            match tmp_val & 0xfc8000 {
                524288 => { a_handler36(core, tmp_val); return; }
                1572864 => { a_handler35(core, tmp_val); return; }
                2359296 => { a_handler25(core, tmp_val); return; }
                2621440 => { a_handler34(core, tmp_val); return; }
                2883584 => { a_handler41(core, tmp_val); return; }
                3407872 => { a_handler23(core, tmp_val); return; }
                3670016 => { a_handler30(core, tmp_val); return; }
                3932160 => { a_handler39(core, tmp_val); return; }
                4718592 => { a_handler33(core, tmp_val); return; }
                5767168 => { a_handler32(core, tmp_val); return; }
                6553600 => { a_handler24(core, tmp_val); return; }
                6815744 => { a_handler31(core, tmp_val); return; }
                7077888 => { a_handler40(core, tmp_val); return; }
                7340032 => { _handler60(core, tmp_val); return; }
                7602176 => { a_handler22(core, tmp_val); return; }
                7864320 => { a_handler29(core, tmp_val); return; }
                8126464 => { a_handler38(core, tmp_val); return; }
                8388608 => { r_handler51(core, tmp_val); return; }
                9437184 => { r_handler50(core, tmp_val); return; }
                _ => {}
            }
        }
        5 => {
            match tmp_val & 48 {
                0 => { r_handler0(core, tmp_val); return; }
                16 => { r_handler1(core, tmp_val); return; }
                32 => { r_handler2(core, tmp_val); return; }
                48 => { r_handler3(core, tmp_val); return; }
                _ => {}
            }
        }
        6 => {
            match tmp_val & 48 {
                0 => { r_handler39(core, tmp_val); return; }
                16 => {
                    match tmp_val & 192 {
                        0 => { n_handler32(core, tmp_val); return; }
                        64 => { n_handler48(core, tmp_val); return; }
                        128 => { n_handler44(core, tmp_val); return; }
                        192 => { n_handler39(core, tmp_val); return; }
                        _ => {}
                    }
                }
                32 => {
                    match tmp_val & 192 {
                        0 => { n_handler31(core, tmp_val); return; }
                        64 => { n_handler47(core, tmp_val); return; }
                        128 => { n_handler41(core, tmp_val); return; }
                        192 => { n_handler36(core, tmp_val); return; }
                        _ => {}
                    }
                }
                48 => {
                    match tmp_val & 192 {
                        0 => { r_handler24(core, tmp_val); return; }
                        64 => {
                            match tmp_val & 61440 {
                                0 => { n_handler34(core, tmp_val); return; }
                                4096 => { n_handler53(core, tmp_val); return; }
                                32768 => { r_handler54(core, tmp_val); return; }
                                36864 => { r_handler56(core, tmp_val); return; }
                                40960 => { r_handler55(core, tmp_val); return; }
                                _ => {}
                            }
                        }
                        128 => { n_handler43(core, tmp_val); return; }
                        192 => { n_handler38(core, tmp_val); return; }
                        _ => {}
                    }
                }
                _ => {}
            }
        }
        7 => {
            match tmp_val & 57344 {
                0 => {
                    if (tmp_val & 4096) == 4096 { n_handler30(core, tmp_val); return; }
                    if (tmp_val & 4096) == 0 { n_handler50(core, tmp_val); return; }
                }
                8192 => {
                    if (tmp_val & 4096) == 0 { n_handler40(core, tmp_val); return; }
                    if (tmp_val & 4096) == 4096 { n_handler42(core, tmp_val); return; }
                }
                16384 => {
                    if (tmp_val & 4096) == 0 { n_handler24(core, tmp_val); return; }
                    if (tmp_val & 4096) == 4096 { n_handler26(core, tmp_val); return; }
                }
                24576 => { n_handler27(core, tmp_val); return; }
                32768 => {
                    if (tmp_val & 4096) == 0 { n_handler25(core, tmp_val); return; }
                    if (tmp_val & 4096) == 4096 { n_handler46(core, tmp_val); return; }
                }
                40960 => {
                    if (tmp_val & 4096) == 0 { n_handler35(core, tmp_val); return; }
                    if (tmp_val & 4096) == 4096 { n_handler37(core, tmp_val); return; }
                }
                49152 => {
                    if (tmp_val & 4096) == 4096 { n_handler28(core, tmp_val); return; }
                    if (tmp_val & 4096) == 0 { n_handler45(core, tmp_val); return; }
                }
                57344 => { n_handler29(core, tmp_val); return; }
                _ => {}
            }
        }
        8 => { r_handler47(core, tmp_val); return; }
        9 => { _handler21(core, tmp_val); return; }
        10 => { n_handler9(core, tmp_val); return; }
        11 => { n_handler12(core, tmp_val); return; }
        12 => {
            match tmp_val & 128 {
                0 => { a_handler13(core, tmp_val); return; }
                128 => {
                    if (tmp_val & 64) == 0 { n_handler33(core, tmp_val); return; }
                    if (tmp_val & 64) == 64 { n_handler49(core, tmp_val); return; }
                }
                _ => {}
            }
        }
        13 => {
            match tmp_val & 61440 {
                0 => { a_handler4(core, tmp_val); return; }
                61440 => {
                    match tmp_val & 240 {
                        0 => { a_handler61(core, tmp_val); return; }
                        16 => { a_handler62(core, tmp_val); return; }
                        32 => { n_handler52(core, tmp_val); return; }
                        48 => { return; }
                        96 => { r_handler36(core, tmp_val); return; }
                        _ => {}
                    }
                }
                _ => {}
            }
        }
        _ => {}
    }
    // unknownInstruction — call back to JS
    core.exception(TRAP_ILLEGAL_INSTRUCTION);
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
