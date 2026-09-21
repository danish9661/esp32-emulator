use crate::peripherals::types::*;

pub fn gcm_multiply(cpu_val: &mut [u32; 4], tmp_val: &[u32; 4]) {
    let mut idx_val = [0u32; 4];
    let mut clock_event = *tmp_val;
    for tmp_val in 0..128 {
        if (cpu_val[tmp_val >> 5] & (1 << (31 - (tmp_val % 32)))) != 0 {
            idx_val[0] ^= clock_event[0];
            idx_val[1] ^= clock_event[1];
            idx_val[2] ^= clock_event[2];
            idx_val[3] ^= clock_event[3];
        }
        let simulation_clock = if (1 & clock_event[3]) != 0 { 0xe1000000 } else { 0 };
        clock_event[3] = (clock_event[3] >> 1) | ((1 & clock_event[2]) << 31);
        clock_event[2] = (clock_event[2] >> 1) | ((1 & clock_event[1]) << 31);
        clock_event[1] = (clock_event[1] >> 1) | ((1 & clock_event[0]) << 31);
        clock_event[0] = (clock_event[0] >> 1) ^ simulation_clock;
    }
    *cpu_val = idx_val;
}

pub fn find_first_set_bit(cpu_val: u32) -> u32 {
    for t in 0..32 {
        if (cpu_val & (1 << t)) != 0 {
            return t;
        }
    }
    32
}

pub fn byte_swap32(cpu_val: u32) -> u32 {
    ((255 & cpu_val) << 24) |
    ((65280 & cpu_val) << 8) |
    ((cpu_val >> 8) & 65280) |
    ((cpu_val >> 24) & 255)
}

pub fn mask_low_bits(cpu_val: u32, tmp_val: u32) -> u32 {
    if tmp_val >= 32 { cpu_val } else { cpu_val & ((1 << tmp_val) - 1) }
}

pub fn sign_extend(cpu_val: u32, tmp_val: u32) -> u32 {
    let idx_val = 32 - tmp_val;
    (cpu_val.wrapping_shl(idx_val) as i32).wrapping_shr(idx_val) as u32
}

pub fn clamp_int8(cpu_val: i32) -> i32 {
    if cpu_val > 127 { 127 } else if cpu_val < -128 { -128 } else { cpu_val }
}

pub fn clamp_int16(cpu_val: i32) -> i32 {
    if cpu_val > 32767 { 32767 } else if cpu_val < -32768 { -32768 } else { cpu_val }
}

pub fn clamp_int32(cpu_val: i64) -> i32 {
    if cpu_val > 0x7fffffff { 0x7fffffff } else if cpu_val < (-0x80000000i64) { -0x80000000i32 } else { cpu_val as i32 }
}
