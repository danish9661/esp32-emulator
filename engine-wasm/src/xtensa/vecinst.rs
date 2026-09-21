use super::state::CoreState;
use super::exports::{read_uint8, read_uint32, write_uint8, write_uint32};


const SAR_REGISTER: usize = 12;

fn clamp_int8(val: i32) -> i32 {
    if val > 127 { 127 } else if val < -128 { -128 } else { val }
}

fn clamp_int16(val: i32) -> i32 {
    if val > 32767 { 32767 } else if val < -32768 { -32768 } else { val }
}

fn clamp_int32(val: i64) -> i32 {
    if val > 2147483647 { 2147483647 } else if val < -2147483648 { -2147483648 } else { val as i32 }
}

pub struct VQFormat {
    pub qu: u32,
    pub a_handler3: u32,
    pub imm: i32,
    pub qx: u32,
    pub qy: u32,
}

pub struct VQFormatXp {
    pub qu: u32,
    pub a_handler3: u32,
    pub a_handler15: u32,
    pub qx: u32,
    pub qy: u32,
}

pub struct FloatLoadFormat {
    pub fu0: u32,
    pub fu1: u32,
    pub a_handler3: u32,
    pub offset: i32,
}

pub struct VSMulasFormat {
    pub qu: u32,
    pub a_handler3: u32,
    pub qx: u32,
    pub qy: u32,
    pub qs0: u32,
    pub qs1: u32,
}

pub struct BasicVecFormat {
    pub qa: u32,
    pub qz: u32,
    pub qx: u32,
    pub qy: u32,
    pub a_handler3: u32,
}

pub struct VecStoreFormat {
    pub qv: u32,
    pub qa: u32,
    pub qx: u32,
    pub qy: u32,
    pub a_handler3: u32,
}

pub struct CmplxVecFormat {
    pub qu: u32,
    pub qz: u32,
    pub qx: u32,
    pub qy: u32,
    pub sel: u32,
    pub a_handler3: u32,
}

pub struct FFTr2bf {
    pub qa0: u32,
    pub qx: u32,
    pub qy: u32,
    pub a_handler3: u32,
    pub sar4: u32,
}

pub struct FFTscFormat {
    pub qu: u32,
    pub a_handler3: u32,
    pub a_handler15: u32,
    pub qz: u32,
    pub qx: u32,
    pub qy: u32,
    pub sel8: u32,
}

pub struct FFTcmulFormat {
    pub qx: u32,
    pub qy: u32,
    pub qv: u32,
    pub a_handler3: u32,
    pub a_handler15: u32,
    pub sel8: u32,
    pub upd4: u32,
    pub sar4: u32,
}

pub struct FFTamsFormat {
    pub qu: u32,
    pub a_handler3: u32,
    pub qz: u32,
    pub qz1: u32,
    pub qx: u32,
    pub qy: u32,
    pub qm: u32,
    pub sel2: u32,
}

pub struct FFTldstFormat {
    pub qv: u32,
    pub qz1: u32,
    pub as0: u32,
    pub a_handler3: u32,
    pub qx: u32,
    pub qy: u32,
    pub qm: u32,
    pub sel2: u32,
}

pub struct CmulPairResult {
    pub lo: i32,
    pub dport_peripheral: i32,
    pub pair: u32,
}

pub struct VecShift {
    pub qu: u32,
    pub qs0: u32,
    pub qs1: u32,
    pub a_handler3: u32,
}

pub fn decode_vq_format(cpu_val: u32) -> VQFormat {
    let tmp_val = ((cpu_val >> 19) & 1) | (((cpu_val >> 24) & 3) << 1);
    let idx_val = (cpu_val >> 4) & 15;
    let t = ((cpu_val >> 8) & 15) | (((cpu_val >> 26) & 3) << 4);
    let r = (((t << 26) as i32) >> 22);
    VQFormat {
        qu: tmp_val,
        a_handler3: idx_val,
        imm: r,
        qx: ((cpu_val >> 14) & 1) | (((cpu_val >> 15) & 1) << 1) | ((cpu_val & 1) << 2),
        qy: ((cpu_val >> 23) & 1) | (((cpu_val >> 12) & 1) << 1) | (((cpu_val >> 13) & 1) << 2),
    }
}

pub fn decode_vq_format_xp(cpu_val: u32) -> VQFormatXp {
    let tmp_val = ((cpu_val >> 19) & 1) | (((cpu_val >> 24) & 3) << 1);
    let idx_val = (cpu_val >> 4) & 15;
    let r = (cpu_val >> 8) & 15;
    VQFormatXp {
        qu: tmp_val,
        a_handler3: idx_val,
        a_handler15: r,
        qx: ((cpu_val >> 14) & 1) | (((cpu_val >> 15) & 1) << 1) | ((cpu_val & 1) << 2),
        qy: ((cpu_val >> 23) & 1) | (((cpu_val >> 12) & 1) << 1) | (((cpu_val >> 13) & 1) << 2),
    }
}

pub fn load_q_registers(core: &mut CoreState, tmp_val: u32, idx_val: u32) {
    let aligned = idx_val & !15;
    for i in 0..16 {
        core.q_registers[(16 * tmp_val + i) as usize] =
            read_uint8(core, aligned + i as u32) as u8;
    }
}

pub fn read_qacc_signed(core: &CoreState, tmp_val: u32) -> i32 {
    let qacc = if tmp_val < 8 { &core.qacc_low } else { &core.qacc_high };
    let slot = tmp_val % 8;
    let read_result = (5 * (slot >> 1)) as usize;
    let val = if (1 & slot) == 0 {
        (qacc[read_result] as u32)
            | ((qacc[read_result + 1] as u32) << 8)
            | (((qacc[read_result + 2] & 15) as u32) << 16)
    } else {
        (((qacc[read_result + 2] >> 4) & 15) as u32)
            | ((qacc[read_result + 3] as u32) << 4)
            | ((qacc[read_result + 4] as u32) << 12)
    };
    ((val << 12) as i32) >> 12
}

pub fn write_qacc_value(core: &mut CoreState, tmp_val: u32, idx_val: i32) {
    let mut val = idx_val;
    if val > 524287 { val = 524287; }
    else if val < -524288 { val = -524288; }
    let val = (val as u32) & 1048575;
    let read_result = tmp_val % 8;
    let write_addr = (5 * (read_result >> 1)) as usize;
    if (1 & read_result) == 0 {
        core.qacc_low[write_addr] = (val & 255) as u8;
        core.qacc_low[write_addr + 1] = ((val >> 8) & 255) as u8;
        core.qacc_low[write_addr + 2] = (core.qacc_low[write_addr + 2] & 240) | ((val >> 16) as u8 & 15);
    } else {
        core.qacc_low[write_addr + 2] = (core.qacc_low[write_addr + 2] & 15) | (((val & 15) as u8) << 4);
        core.qacc_low[write_addr + 3] = ((val >> 4) & 255) as u8;
        core.qacc_low[write_addr + 4] = ((val >> 12) & 255) as u8;
    }
}

pub fn read_qacc_big_int(core: &CoreState, tmp_val: u32) -> i128 {
    let qacc = if tmp_val < 4 { &core.qacc_low } else { &core.qacc_high };
    let byte_off = ((tmp_val % 4) * 5) as usize;
    let mut val = (qacc[byte_off] as i128)
        | ((qacc[byte_off + 1] as i128) << 8)
        | ((qacc[byte_off + 2] as i128) << 16)
        | ((qacc[byte_off + 3] as i128) << 24)
        | ((qacc[byte_off + 4] as i128) << 32);
    if val & (1i128 << 39) != 0 {
        val |= !((1i128 << 40) - 1);
    }
    val
}

pub fn write_qacc_big_int(core: &mut CoreState, tmp_val: u32, idx_val: i128, signed: bool) {
    let mut val = idx_val;
    if signed {
        let max_val = (1i128 << 40) - 1;
        if val < 0 { val = 0; }
        if val > max_val { val = max_val; }
    } else {
        let max_val = (1i128 << 39) - 1;
        let min_val = -(1i128 << 39);
        if val > max_val { val = max_val; }
        if val < min_val { val = min_val; }
    }
    let masked = val & ((1i128 << 40) - 1);
    let byte_off = ((tmp_val % 4) * 5) as usize;
    if tmp_val < 4 {
        core.qacc_low[byte_off] = (masked & 255) as u8;
        core.qacc_low[byte_off + 1] = ((masked >> 8) & 255) as u8;
        core.qacc_low[byte_off + 2] = ((masked >> 16) & 255) as u8;
        core.qacc_low[byte_off + 3] = ((masked >> 24) & 255) as u8;
        core.qacc_low[byte_off + 4] = ((masked >> 32) & 255) as u8;
    } else {
        core.qacc_high[byte_off] = (masked & 255) as u8;
        core.qacc_high[byte_off + 1] = ((masked >> 8) & 255) as u8;
        core.qacc_high[byte_off + 2] = ((masked >> 16) & 255) as u8;
        core.qacc_high[byte_off + 3] = ((masked >> 24) & 255) as u8;
        core.qacc_high[byte_off + 4] = ((masked >> 32) & 255) as u8;
    }
}

pub fn clamp_signed_24(cpu_val: i32) -> i32 {
    if cpu_val > 524287 { 524287 } else if cpu_val < -524287 { -524287 } else { cpu_val }
}

pub fn clamp_signed_40(cpu_val: i128) -> i128 {
    let tmp_val: i128 = 549755813887;
    if cpu_val > tmp_val { tmp_val } else if cpu_val < -tmp_val { -tmp_val } else { cpu_val }
}

pub fn clamp_unsigned_40(cpu_val: i128) -> i128 {
    let tmp_val: i128 = 1099511627775;
    if cpu_val < 0 { 0 } else if cpu_val > tmp_val { tmp_val } else { cpu_val }
}

pub fn qacc_mul_acc_s8(core: &mut CoreState, tmp_val: u32, idx_val: u32) {
    for i in 0..16 {
        let a_val = (core.q_registers[(16 * tmp_val + i) as usize] as i8) as i32;
        let b_val = (core.q_registers[(16 * idx_val + i) as usize] as i8) as i32;
        write_qacc_value(
            core,
            i,
            clamp_signed_24(read_qacc_signed(core, i) + a_val * b_val),
        );
    }
}

pub fn qacc_mul_acc_u8(core: &mut CoreState, tmp_val: u32, idx_val: u32) {
    for i in 0..16 {
        let a_val = 255 & core.q_registers[(16 * tmp_val + i) as usize] as u32;
        let b_val = 255 & core.q_registers[(16 * idx_val + i) as usize] as u32;
        let mut qacc_val = read_qacc_signed(core, i);
        if qacc_val < 0 { qacc_val += 1048576; }
        let mut new_val = qacc_val + (a_val * b_val) as i32;
        if new_val > 1048575 { new_val = 1048575; }
        if new_val < 0 { new_val = 0; }
        let final_val = if new_val >= 524288 { new_val - 1048576 } else { new_val };
        write_qacc_value(core, i, final_val);
    }
}

pub fn qacc_mul_acc_s16(core: &mut CoreState, tmp_val: u32, idx_val: u32) {
    for i in 0..8 {
        let a = core.q_reg_int16((8 * tmp_val + i) as usize) as i32;
        let b = core.q_reg_int16((8 * idx_val + i) as usize) as i32;
        let prod = (a as i128) * (b as i128);
        let mut acc = read_qacc_big_int(core, i) + prod;
        let max_val: i128 = 549755813887;
        if acc > max_val { acc = max_val; }
        if acc < -max_val { acc = -max_val; }
        write_qacc_big_int(core, i as u32, acc, false);
    }
}

pub fn qacc_mul_acc_u16(core: &mut CoreState, tmp_val: u32, idx_val: u32) {
    for i in 0..8 {
        let a_val = core.q_reg_uint16((8 * tmp_val + i) as usize) as i128;
        let b_val = core.q_reg_uint16((8 * idx_val + i) as usize) as i128;
        let mut qacc_val = read_qacc_big_int(core, i);
        if qacc_val < 0 { qacc_val += 1i128 << 40; }
        let mut new_val = qacc_val + a_val * b_val;
        let max_val: i128 = 1099511627775;
        if new_val > max_val { new_val = max_val; }
        if new_val >= 549755813888 { new_val -= 1i128 << 40; }
        write_qacc_big_int(core, i as u32, new_val, false);
    }
}

pub fn accx_mul_acc_s8(core: &mut CoreState, tmp_val: u32, idx_val: u32) {
    let mut acc = core.get_accx();
    for i in 0..16 {
        let a = (core.q_registers[(16 * tmp_val + i) as usize] as i8) as i32;
        let b = (core.q_registers[(16 * idx_val + i) as usize] as i8) as i32;
        acc += (a * b) as i128;
    }
    core.set_accx(clamp_signed_40(acc));
}

pub fn accx_mul_acc_u8(core: &mut CoreState, tmp_val: u32, idx_val: u32) {
    let mut acc = core.get_accx();
    if acc < 0 { acc += 1i128 << 40; }
    for i in 0..16 {
        let a = (255 & core.q_registers[(16 * tmp_val + i) as usize] as u32) as i128;
        let b = (255 & core.q_registers[(16 * idx_val + i) as usize] as u32) as i128;
        acc += a * b;
    }
    acc = clamp_unsigned_40(acc);
    if acc >= 549755813888 { acc -= 1i128 << 40; }
    core.set_accx(acc);
}

pub fn accx_mul_acc_s16(core: &mut CoreState, tmp_val: u32, idx_val: u32) {
    let mut acc = core.get_accx();
    for i in 0..8 {
        let a = core.q_reg_int16((8 * tmp_val + i) as usize) as i128;
        let b = core.q_reg_int16((8 * idx_val + i) as usize) as i128;
        acc += a * b;
    }
    core.set_accx(clamp_signed_40(acc));
}

pub fn accx_mul_acc_u16(core: &mut CoreState, tmp_val: u32, idx_val: u32) {
    let mut acc = core.get_accx();
    if acc < 0 { acc += 1i128 << 40; }
    for i in 0..8 {
        let a = core.q_reg_uint16((8 * tmp_val + i) as usize) as i128;
        let b = core.q_reg_uint16((8 * idx_val + i) as usize) as i128;
        acc += a * b;
    }
    acc = clamp_unsigned_40(acc);
    if acc >= 549755813888 { acc -= 1i128 << 40; }
    core.set_accx(acc);
}

pub fn shift_q_registers(core: &mut CoreState, tmp_val: u32, idx_val: u32) {
    let shift = (core.sar_byte() & 15) as usize;
    let mut temp = [0u8; 16];
    for i in 0..(16 - shift) {
        temp[i] = core.q_registers[(16 * tmp_val + i as u32 + shift as u32) as usize];
    }
    for i in (16 - shift)..16 {
        temp[i] = core.q_registers[(16 * idx_val + i as u32 - (16 - shift) as u32) as usize];
    }
    for i in 0..16 {
        core.q_registers[(16 * tmp_val + i as u32) as usize] = temp[i];
    }
}

pub fn decode_float_load_format(cpu_val: u32) -> FloatLoadFormat {
    let tmp_val = (cpu_val >> 12) & 15;
    let off = ((cpu_val >> 8) & 1)
        | ((cpu_val & 1) << 1)
        | (((cpu_val >> 24) & 1) << 2)
        | (((cpu_val >> 25) & 1) << 3);
    let offset = ((((off << 28) as i32) >> 28) * 8);
    FloatLoadFormat {
        fu0: tmp_val,
        fu1: (cpu_val >> 20) & 15,
        a_handler3: (cpu_val >> 4) & 15,
        offset,
    }
}

pub fn vmulas_ip(core: &mut CoreState, tmp_val: u32, idx_val: &str, clock_event: &str) {
    let fmt = decode_vq_format(tmp_val);
    if !core.window_check(0, fmt.a_handler3 >> 2, 0) {
        vmulas_dispatch(core, fmt.qx, fmt.qy, idx_val, clock_event);
        load_q_registers(core, fmt.qu, core.ar(fmt.a_handler3));
        core.set_ar(fmt.a_handler3, core.ar(fmt.a_handler3).wrapping_add(fmt.imm as u32));
    }
}

pub fn vmulas_xp(core: &mut CoreState, tmp_val: u32, idx_val: &str, clock_event: &str) {
    let fmt = decode_vq_format_xp(tmp_val);
    if !core.window_check(0, fmt.a_handler3 >> 2, 0) {
        vmulas_dispatch(core, fmt.qx, fmt.qy, idx_val, clock_event);
        load_q_registers(core, fmt.qu, core.ar(fmt.a_handler3));
        core.set_ar(fmt.a_handler3, core.ar(fmt.a_handler3).wrapping_add(core.ar(fmt.a_handler15)));
    }
}

pub fn decode_vs_mulas_format(cpu_val: u32) -> VSMulasFormat {
    let tmp_val = ((cpu_val >> 19) & 1) | (((cpu_val >> 24) & 3) << 1);
    let idx_val = (cpu_val >> 4) & 15;
    let clock_event = ((cpu_val >> 14) & 1)
        | (((cpu_val >> 15) & 1) << 1)
        | (((cpu_val & 1) << 2));
    let simulation_clock = ((cpu_val >> 23) & 1)
        | (((cpu_val >> 12) & 1) << 1)
        | (((cpu_val >> 13) & 1) << 2);
    VSMulasFormat {
        qu: tmp_val,
        a_handler3: idx_val,
        qx: clock_event,
        qy: simulation_clock,
        qs0: ((cpu_val >> 20) & 1)
            | (((cpu_val >> 21) & 1) << 1)
            | (((cpu_val >> 22) & 1) << 2),
        qs1: ((cpu_val >> 16) & 1)
            | (((cpu_val >> 17) & 1) << 1)
            | (((cpu_val >> 18) & 1) << 2),
    }
}

pub fn vmulas_dispatch(core: &mut CoreState, tmp_val: u32, idx_val: u32, clock_event: &str, simulation_clock: &str) {
    match simulation_clock {
        "qacc" => match clock_event {
            "regOff64" => qacc_mul_acc_s8(core, tmp_val, idx_val),
            "u8" => qacc_mul_acc_u8(core, tmp_val, idx_val),
            "s16" => qacc_mul_acc_s16(core, tmp_val, idx_val),
            _ => qacc_mul_acc_u16(core, tmp_val, idx_val),
        },
        _ => match clock_event {
            "regOff64" => accx_mul_acc_s8(core, tmp_val, idx_val),
            "u8" => accx_mul_acc_u8(core, tmp_val, idx_val),
            "s16" => accx_mul_acc_s16(core, tmp_val, idx_val),
            _ => accx_mul_acc_u16(core, tmp_val, idx_val),
        },
    }
}

pub fn vmulas_qup(core: &mut CoreState, tmp_val: u32, idx_val: &str, clock_event: &str, simulation_clock: &str) {
    let fmt = decode_vs_mulas_format(tmp_val);
    if !core.window_check(0, fmt.a_handler3 >> 2, 0) {
        if simulation_clock == "ldbc" {
            let addr = core.ar(fmt.a_handler3);
            vmulas_dispatch(core, fmt.qx, fmt.qy, idx_val, clock_event);
            shift_q_registers(core, fmt.qs0, fmt.qs1);
            if idx_val == "regOff64" || idx_val == "u8" {
                let val = read_uint8(core, addr);
                for i in 0..16 {
                    core.q_registers[(16 * fmt.qu + i) as usize] = val as u8;
                }
                core.set_ar(fmt.a_handler3, addr.wrapping_add(1));
            } else {
                let val = read_uint8(core, addr) as u32
                    | ((read_uint8(core, addr + 1) as u32) << 8);
                for i in 0..8 {
                    core.q_registers[(16 * fmt.qu + 2 * i) as usize] = (val & 255) as u8;
                    core.q_registers[(16 * fmt.qu + 2 * i + 1) as usize] = ((val >> 8) & 255) as u8;
                }
                core.set_ar(fmt.a_handler3, addr.wrapping_add(2));
            }
            return;
        }
        vmulas_dispatch(core, fmt.qx, fmt.qy, idx_val, clock_event);
        shift_q_registers(core, fmt.qs0, fmt.qs1);
        load_q_registers(core, fmt.qu, core.ar(fmt.a_handler3));
        if simulation_clock == "decodeVQFormatXp" {
            let t = ((tmp_val >> 8) & 15) | (((tmp_val >> 26) & 3) << 4);
            let idx_val = (((t << 26) as i32) >> 22);
            core.set_ar(fmt.a_handler3, core.ar(fmt.a_handler3).wrapping_add(idx_val as u32));
        } else {
            let idx_val = (tmp_val >> 8) & 15;
            core.set_ar(fmt.a_handler3, core.ar(fmt.a_handler3).wrapping_add(core.ar(idx_val)));
        }
    }
}

pub fn vsmulas_dispatch(core: &mut CoreState, tmp_val: u32) {
    let idx_val = ((tmp_val >> 19) & 1) | (((tmp_val >> 24) & 3) << 1);
    let clock_event = (tmp_val >> 4) & 15;
    let simulation_clock = ((tmp_val >> 14) & 1)
        | (((tmp_val >> 15) & 1) << 1)
        | (((tmp_val & 1) << 2));
    let read_result = ((tmp_val >> 23) & 1)
        | (((tmp_val >> 12) & 1) << 1)
        | (((tmp_val >> 13) & 1) << 2);
    let write_addr = (4194304 & tmp_val) != 0;
    let register_type = if write_addr {
        ((tmp_val >> 16) & 1)
            | (((tmp_val >> 17) & 1) << 1)
            | (((tmp_val >> 18) & 1) << 2)
    } else {
        ((tmp_val >> 20) & 1)
            | (((tmp_val >> 16) & 1) << 1)
            | (((tmp_val >> 17) & 1) << 2)
            | (((tmp_val >> 18) & 1) << 3)
    };
    if !core.window_check(0, clock_event >> 2, 0) {
        if write_addr {
            let tmp = core.q_reg_int16((8 * read_result + (7 & register_type)) as usize) as i32;
            for i in 0..8 {
                let prod = (core.q_reg_int16((8 * simulation_clock + i) as usize) as i32)
                    .wrapping_mul(tmp);
                let mut result = read_qacc_big_int(core, i) + prod as i128;
                let max_val: i128 = 549755813887;
                if result > max_val { result = max_val; }
                if result < -max_val { result = -max_val; }
                write_qacc_big_int(core, i as u32, result, false);
            }
        } else {
            let tmp = (core.q_registers[(16 * read_result + (15 & register_type)) as usize] as i8) as i32;
            for i in 0..16 {
                let prod = (core.q_registers[(16 * simulation_clock + i) as usize] as i8) as i32;
                write_qacc_value(
                    core,
                    i,
                    clamp_signed_24(read_qacc_signed(core, i) + prod * tmp),
                );
            }
        }
        load_q_registers(core, idx_val, core.ar(clock_event));
        core.set_ar(clock_event, core.ar(clock_event).wrapping_add(16));
    }
}

pub fn decode_basic_vec_format(cpu_val: u32) -> BasicVecFormat {
    let tmp_val = ((cpu_val >> 19) & 1) | (((cpu_val >> 24) & 3) << 1);
    let idx_val = (cpu_val >> 16) & 7;
    let clock_event = ((cpu_val >> 14) & 1)
        | (((cpu_val >> 15) & 1) << 1)
        | ((1 & cpu_val) << 2);
    BasicVecFormat {
        qa: tmp_val,
        qz: idx_val,
        qx: clock_event,
        qy: ((cpu_val >> 23) & 1)
            | (((cpu_val >> 12) & 1) << 1)
            | (((cpu_val >> 13) & 1) << 2),
        a_handler3: (cpu_val >> 4) & 15,
    }
}

pub fn vadds8_vec_op(core: &mut CoreState, tmp_val: u32, idx_val: u32, clock_event: u32) {
    for i in 0..16 {
        let a = (core.q_registers[(16 * idx_val + i) as usize] as i8) as i32;
        let b = (core.q_registers[(16 * clock_event + i) as usize] as i8) as i32;
        core.q_registers[(16 * tmp_val + i) as usize] = (clamp_int8(a + b) & 255) as u8;
    }
}

pub fn vadds16_vec_op(core: &mut CoreState, tmp_val: u32, idx_val: u32, clock_event: u32) {
    for i in 0..8 {
        let a = core.q_reg_int16((8 * idx_val + i) as usize) as i32;
        let b = core.q_reg_int16((8 * clock_event + i) as usize) as i32;
        core.set_q_reg_int16((8 * tmp_val + i) as usize, clamp_int16(a + b) as i16);
    }
}

pub fn vmax8_vec_op(core: &mut CoreState, tmp_val: u32, idx_val: u32, clock_event: u32) {
    for i in 0..16 {
        let a = (core.q_registers[(16 * idx_val + i) as usize] as i8) as i32;
        let b = (core.q_registers[(16 * clock_event + i) as usize] as i8) as i32;
        core.q_registers[(16 * tmp_val + i) as usize] = (if a > b { a } else { b }) as u8;
    }
}

pub fn vmax16_vec_op(core: &mut CoreState, tmp_val: u32, idx_val: u32, clock_event: u32) {
    for i in 0..8 {
        let a = core.q_reg_int16((8 * idx_val + i) as usize);
        let b = core.q_reg_int16((8 * clock_event + i) as usize);
        core.set_q_reg_int16((8 * tmp_val + i) as usize, if a > b { a } else { b });
    }
}

pub fn vsubs8_vec_op(core: &mut CoreState, tmp_val: u32, idx_val: u32, clock_event: u32) {
    for i in 0..16 {
        let a = (core.q_registers[(16 * idx_val + i) as usize] as i8) as i32;
        let b = (core.q_registers[(16 * clock_event + i) as usize] as i8) as i32;
        core.q_registers[(16 * tmp_val + i) as usize] = (clamp_int8(a - b) & 255) as u8;
    }
}

pub fn vsubs16_vec_op(core: &mut CoreState, tmp_val: u32, idx_val: u32, clock_event: u32) {
    for i in 0..8 {
        let a = core.q_reg_int16((8 * idx_val + i) as usize) as i32;
        let b = core.q_reg_int16((8 * clock_event + i) as usize) as i32;
        core.set_q_reg_int16((8 * tmp_val + i) as usize, clamp_int16(a - b) as i16);
    }
}

pub fn vmin8_vec_op(core: &mut CoreState, tmp_val: u32, idx_val: u32, clock_event: u32) {
    for i in 0..16 {
        let a = (core.q_registers[(16 * idx_val + i) as usize] as i8) as i32;
        let b = (core.q_registers[(16 * clock_event + i) as usize] as i8) as i32;
        core.q_registers[(16 * tmp_val + i) as usize] = (if a < b { a } else { b }) as u8;
    }
}

pub fn vmin16_vec_op(core: &mut CoreState, tmp_val: u32, idx_val: u32, clock_event: u32) {
    for i in 0..8 {
        let a = core.q_reg_int16((8 * idx_val + i) as usize);
        let b = core.q_reg_int16((8 * clock_event + i) as usize);
        core.set_q_reg_int16((8 * tmp_val + i) as usize, if a < b { a } else { b });
    }
}

pub fn clamp_int32_full(cpu_val: i32) -> i32 {
    if cpu_val > 0x7fffffff { 0x7fffffff }
    else if cpu_val < -0x80000000 { -0x80000000 }
    else { cpu_val }
}

pub fn vadds32_vec_op(core: &mut CoreState, tmp_val: u32, idx_val: u32, clock_event: u32) {
    for i in 0..4 {
        let a = core.q_reg_int32((4 * idx_val + i) as usize);
        let b = core.q_reg_int32((4 * clock_event + i) as usize);
        core.set_q_reg_int32((4 * tmp_val + i) as usize, clamp_int32_full(a.wrapping_add(b)));
    }
}

pub fn vsubs32_vec_op(core: &mut CoreState, tmp_val: u32, idx_val: u32, clock_event: u32) {
    for i in 0..4 {
        let a = core.q_reg_int32((4 * idx_val + i) as usize);
        let b = core.q_reg_int32((4 * clock_event + i) as usize);
        core.set_q_reg_int32((4 * tmp_val + i) as usize, clamp_int32_full(a.wrapping_sub(b)));
    }
}

pub fn vmax32_vec_op(core: &mut CoreState, tmp_val: u32, idx_val: u32, clock_event: u32) {
    for i in 0..4 {
        let a = core.q_reg_int32((4 * idx_val + i) as usize);
        let b = core.q_reg_int32((4 * clock_event + i) as usize);
        core.set_q_reg_int32((4 * tmp_val + i) as usize, if a > b { a } else { b });
    }
}

pub fn vmin32_vec_op(core: &mut CoreState, tmp_val: u32, idx_val: u32, clock_event: u32) {
    for i in 0..4 {
        let a = core.q_reg_int32((4 * idx_val + i) as usize);
        let b = core.q_reg_int32((4 * clock_event + i) as usize);
        core.set_q_reg_int32((4 * tmp_val + i) as usize, if a < b { a } else { b });
    }
}

pub fn vmul_s8_shift_vec_op(core: &mut CoreState, tmp_val: u32, idx_val: u32, clock_event: u32) {
    let sim_clock = if core.sar_m32_pending != 0 { core.sar_m32 as u32 } else { 63 & core.special_registers[SAR_REGISTER] };
    for r in 0..16 {
        let a = (core.q_registers[(16 * idx_val + r) as usize] as i8) as i32;
        let b = (core.q_registers[(16 * clock_event + r) as usize] as i8) as i32;
        core.q_registers[(16 * tmp_val + r) as usize] = ((a * b >> (sim_clock & 31)) & 255) as u8;
    }
}

pub fn vmul_s16_shift_vec_op(core: &mut CoreState, tmp_val: u32, idx_val: u32, clock_event: u32) {
    let sim_clock = if core.sar_m32_pending != 0 { core.sar_m32 as u32 } else { 63 & core.special_registers[SAR_REGISTER] };
    for r in 0..8 {
        let a = core.q_reg_int16((8 * idx_val + r) as usize) as i32;
        let b = core.q_reg_int16((8 * clock_event + r) as usize) as i32;
        let result = (((a.wrapping_mul(b)) >> (sim_clock & 31)) & 65535) as i16;
        core.set_q_reg_int16((8 * tmp_val + r) as usize, result);
    }
}

pub fn vmul_u8_shift_vec_op(core: &mut CoreState, tmp_val: u32, idx_val: u32, clock_event: u32) {
    let sim_clock = if core.sar_m32_pending != 0 { core.sar_m32 as u32 } else { 63 & core.special_registers[SAR_REGISTER] };
    for r in 0..16 {
        let a = (255 & core.q_registers[(16 * idx_val + r) as usize] as u32) as u32;
        let b = (255 & core.q_registers[(16 * clock_event + r) as usize] as u32) as u32;
        core.q_registers[(16 * tmp_val + r) as usize] = (((a * b) >> sim_clock) & 255) as u8;
    }
}

pub fn vmul_u16_shift_vec_op(core: &mut CoreState, tmp_val: u32, idx_val: u32, clock_event: u32) {
    let sim_clock = if core.sar_m32_pending != 0 { core.sar_m32 as u32 } else { 63 & core.special_registers[SAR_REGISTER] };
    for r in 0..8 {
        let a = core.q_reg_uint16((8 * idx_val + r) as usize) as u32;
        let b = core.q_reg_uint16((8 * clock_event + r) as usize) as u32;
        let result = (((a * b) >> sim_clock) & 65535) as u16;
        core.set_q_reg_uint16((8 * tmp_val + r) as usize, result);
    }
}

pub fn exec_vec_op_incp(core: &mut CoreState, tmp_val: u32, idx_val: fn(&mut CoreState, u32, u32, u32)) {
    let fmt = decode_basic_vec_format(tmp_val);
    if !core.window_check(0, fmt.a_handler3 >> 2, 0) {
        idx_val(core, fmt.qz, fmt.qx, fmt.qy);
        load_q_registers(core, fmt.qa, core.ar(fmt.a_handler3));
        core.set_ar(fmt.a_handler3, core.ar(fmt.a_handler3).wrapping_add(16));
    }
}

pub fn decode_vec_store_format(cpu_val: u32) -> VecStoreFormat {
    let tmp_val = ((cpu_val >> 20) & 1)
        | (((cpu_val >> 21) & 1) << 1)
        | (((cpu_val >> 22) & 1) << 2);
    let idx_val = (cpu_val >> 16) & 7;
    let clock_event = ((cpu_val >> 14) & 1)
        | (((cpu_val >> 15) & 1) << 1)
        | ((1 & cpu_val) << 2);
    VecStoreFormat {
        qv: tmp_val,
        qa: idx_val,
        qx: clock_event,
        qy: ((cpu_val >> 23) & 1)
            | (((cpu_val >> 12) & 1) << 1)
            | (((cpu_val >> 13) & 1) << 2),
        a_handler3: (cpu_val >> 4) & 15,
    }
}

pub fn store_q_registers(core: &mut CoreState, tmp_val: u32, idx_val: u32) {
    let aligned = idx_val & !15;
    for i in 0..16 {
        write_uint8(core, aligned + i, core.q_registers[(16 * tmp_val + i) as usize] as u32, 0);
    }
}

pub fn exec_vec_store_incp(core: &mut CoreState, tmp_val: u32, idx_val: fn(&mut CoreState, u32, u32, u32)) {
    let fmt = decode_vec_store_format(tmp_val);
    if !core.window_check(0, fmt.a_handler3 >> 2, 0) {
        idx_val(core, fmt.qa, fmt.qx, fmt.qy);
        store_q_registers(core, fmt.qv, core.ar(fmt.a_handler3));
        core.set_ar(fmt.a_handler3, core.ar(fmt.a_handler3).wrapping_add(16));
    }
}

pub fn decode_cmplx_vec_format(cpu_val: u32) -> CmplxVecFormat {
    let tmp_val = ((cpu_val >> 19) & 1)
        | (((cpu_val >> 24) & 1) << 1)
        | (((cpu_val >> 25) & 1) << 2);
    let idx_val = ((cpu_val >> 16) & 1)
        | (((cpu_val >> 17) & 1) << 1)
        | (((cpu_val >> 18) & 1) << 2);
    let clock_event = ((cpu_val >> 14) & 1)
        | (((cpu_val >> 15) & 1) << 1)
        | (((cpu_val & 1) << 2));
    let simulation_clock = ((cpu_val >> 23) & 1)
        | (((cpu_val >> 12) & 1) << 1)
        | (((cpu_val >> 13) & 1) << 2);
    CmplxVecFormat {
        qu: tmp_val,
        qz: idx_val,
        qx: clock_event,
        qy: simulation_clock,
        sel: ((cpu_val >> 8) & 1) | (((cpu_val >> 9) & 1) << 1),
        a_handler3: (cpu_val >> 4) & 15,
    }
}

pub fn cmul_pair(core: &mut CoreState, tmp_val: u32, idx_val: u32, clock_event: u32, simulation_clock: u32) {
    let read_result = if core.sar_m32_pending != 0 { core.sar_m32 as u32 } else { 31 & core.special_registers[SAR_REGISTER] };
    let write_addr = (1 & simulation_clock) == 1;
    let register_type = (2 & simulation_clock) == 2;
    let cfg_val = 4 * write_addr as u32;
    for i in 0..2 {
        let off_val = core.q_reg_int16((8 * idx_val + cfg_val + 2 * i) as usize) as i128;
        let len_val = core.q_reg_int16((8 * idx_val + cfg_val + 2 * i + 1) as usize) as i128;
        let val_val = core.q_reg_int16((8 * clock_event + cfg_val + 2 * i) as usize) as i128;
        let i2c_command = core.q_reg_int16((8 * clock_event + cfg_val + 2 * i + 1) as usize) as i128;
        let (prod_a, prod_b) = if register_type {
            (off_val * val_val + len_val * i2c_command,
             off_val * i2c_command - len_val * val_val)
        } else {
            (off_val * val_val - len_val * i2c_command,
             off_val * i2c_command + len_val * val_val)
        };
        core.set_q_reg_int16(
            (8 * tmp_val + cfg_val + 2 * i) as usize,
            clamp_s16_value((prod_a >> read_result) as i32) as i16,
        );
        core.set_q_reg_int16(
            (8 * tmp_val + cfg_val + 2 * i + 1) as usize,
            clamp_s16_value((prod_b >> read_result) as i32) as i16,
        );
    }
}

pub fn clamp_s16_value(cpu_val: i32) -> i32 {
    if cpu_val > 32767 { 32767 } else if cpu_val < -32768 { -32768 } else { cpu_val }
}

pub fn decode_fft_r2bf(cpu_val: u32) -> FFTr2bf {
    let tmp_val = ((cpu_val >> 16) & 1)
        | (((cpu_val >> 17) & 1) << 1)
        | (((cpu_val >> 18) & 1) << 2);
    let idx_val = ((cpu_val >> 14) & 1)
        | (((cpu_val >> 15) & 1) << 1)
        | (((cpu_val & 1) << 2));
    let clock_event = ((cpu_val >> 20) & 1)
        | (((cpu_val >> 21) & 1) << 1)
        | (((cpu_val >> 22) & 1) << 2);
    FFTr2bf {
        qa0: tmp_val,
        qx: idx_val,
        qy: clock_event,
        a_handler3: (cpu_val >> 4) & 15,
        sar4: ((cpu_val >> 23) & 1) | (((cpu_val >> 12) & 1) << 1),
    }
}

pub fn decode_fft_sc_format(cpu_val: u32) -> FFTscFormat {
    let tmp_val = ((cpu_val >> 23) & 1)
        | (((cpu_val >> 12) & 1) << 1)
        | (((cpu_val >> 13) & 1) << 2);
    let idx_val = (cpu_val >> 4) & 15;
    let clock_event = (cpu_val >> 8) & 15;
    let simulation_clock = ((cpu_val >> 16) & 1)
        | (((cpu_val >> 17) & 1) << 1)
        | (((cpu_val >> 18) & 1) << 2);
    let read_result = ((cpu_val >> 14) & 1)
        | (((cpu_val >> 15) & 1) << 1)
        | (((cpu_val & 1) << 2));
    FFTscFormat {
        qu: tmp_val,
        a_handler3: idx_val,
        a_handler15: clock_event,
        qz: simulation_clock,
        qx: read_result,
        qy: ((cpu_val >> 20) & 1)
            | (((cpu_val >> 21) & 1) << 1)
            | (((cpu_val >> 22) & 1) << 2),
        sel8: ((cpu_val >> 19) & 1)
            | (((cpu_val >> 24) & 1) << 1)
            | (((cpu_val >> 25) & 1) << 2),
    }
}

pub fn decode_fft_cmul_format(cpu_val: u32) -> FFTcmulFormat {
    let tmp_val = ((cpu_val >> 14) & 1)
        | (((cpu_val >> 15) & 1) << 1)
        | (((cpu_val & 1) << 2));
    let idx_val = ((cpu_val >> 20) & 1)
        | (((cpu_val >> 21) & 1) << 1)
        | (((cpu_val >> 22) & 1) << 2);
    let clock_event = ((cpu_val >> 16) & 1)
        | (((cpu_val >> 17) & 1) << 1)
        | (((cpu_val >> 18) & 1) << 2);
    let simulation_clock = (cpu_val >> 4) & 15;
    let read_result = (cpu_val >> 8) & 15;
    let write_addr = ((cpu_val >> 23) & 1)
        | (((cpu_val >> 12) & 1) << 1)
        | (((cpu_val >> 13) & 1) << 2);
    FFTcmulFormat {
        qx: tmp_val,
        qy: idx_val,
        qv: clock_event,
        a_handler3: simulation_clock,
        a_handler15: read_result,
        sel8: write_addr,
        upd4: ((cpu_val >> 19) & 1) | (((cpu_val >> 24) & 1) << 1),
        sar4: ((cpu_val >> 25) & 1) | (((cpu_val >> 26) & 1) << 1),
    }
}

pub fn decode_fft_ams_format(cpu_val: u32) -> FFTamsFormat {
    let tmp_val = ((cpu_val >> 8) & 1)
        | (((cpu_val >> 9) & 1) << 1)
        | (((cpu_val >> 10) & 1) << 2);
    let idx_val = (cpu_val >> 4) & 15;
    let clock_event = ((cpu_val >> 23) & 1)
        | (((cpu_val >> 12) & 1) << 1)
        | (((cpu_val >> 13) & 1) << 2);
    let simulation_clock = ((cpu_val >> 11) & 1)
        | (((cpu_val >> 19) & 1) << 1)
        | (((cpu_val >> 24) & 1) << 2);
    let read_result = ((cpu_val >> 14) & 1)
        | (((cpu_val >> 15) & 1) << 1)
        | (((cpu_val & 1) << 2));
    let write_addr = ((cpu_val >> 20) & 1)
        | (((cpu_val >> 21) & 1) << 1)
        | (((cpu_val >> 22) & 1) << 2);
    FFTamsFormat {
        qu: tmp_val,
        a_handler3: idx_val,
        qz: clock_event,
        qz1: simulation_clock,
        qx: read_result,
        qy: write_addr,
        qm: ((cpu_val >> 16) & 1)
            | (((cpu_val >> 17) & 1) << 1)
            | (((cpu_val >> 18) & 1) << 2),
        sel2: (cpu_val >> 25) & 1,
    }
}

pub fn decode_fft_ldst_format(cpu_val: u32) -> FFTldstFormat {
    let tmp_val = ((cpu_val >> 20) & 1)
        | (((cpu_val >> 21) & 1) << 1)
        | (((cpu_val >> 22) & 1) << 2);
    let idx_val = ((cpu_val >> 19) & 1)
        | (((cpu_val >> 24) & 1) << 1)
        | (((cpu_val >> 25) & 1) << 2);
    let clock_event = (cpu_val >> 4) & 15;
    let simulation_clock = (cpu_val >> 8) & 15;
    let read_result = ((cpu_val >> 16) & 1)
        | (((cpu_val >> 17) & 1) << 1)
        | (((cpu_val >> 18) & 1) << 2);
    let write_addr = ((cpu_val >> 10) & 1)
        | (((cpu_val >> 11) & 1) << 1)
        | (((cpu_val & 1) << 2));
    FFTldstFormat {
        qv: tmp_val,
        qz1: idx_val,
        as0: clock_event,
        a_handler3: simulation_clock,
        qx: read_result,
        qy: write_addr,
        qm: ((cpu_val >> 23) & 1)
            | (((cpu_val >> 12) & 1) << 1)
            | (((cpu_val >> 13) & 1) << 2),
        sel2: (cpu_val >> 26) & 1,
    }
}

pub fn cmul_pair_result(core: &CoreState, tmp_val: u32, idx_val: u32, clock_event: u32) -> CmulPairResult {
    let write_addr = if core.sar_m32_pending != 0 { core.sar_m32 as u32 } else { 31 & core.special_registers[SAR_REGISTER] };
    let register_type = (clock_event >> 1) as usize;
    let cfg_val = 1 & clock_event;
    let h_val = 2 * register_type;
    let off_val = 2 * register_type + 1;
    let signal_direction = core.q_reg_int16((8 * tmp_val + h_val as u32) as usize) as i128;
    let interrupt_trigger = core.q_reg_int16((8 * tmp_val + off_val as u32) as usize) as i128;
    let i2c_command = core.q_reg_int16((8 * idx_val + h_val as u32) as usize) as i128;
    let i2c_interrupt_type = core.q_reg_int16((8 * idx_val + off_val as u32) as usize) as i128;
    let (simulation_clock, read_result) = if cfg_val == 0 {
        (signal_direction * i2c_command + interrupt_trigger * i2c_interrupt_type,
         interrupt_trigger * i2c_command - signal_direction * i2c_interrupt_type)
    } else {
        (signal_direction * i2c_command - interrupt_trigger * i2c_interrupt_type,
         interrupt_trigger * i2c_command + signal_direction * i2c_interrupt_type)
    };
    CmulPairResult {
        lo: ((simulation_clock >> write_addr) as i32) & 65535,
        dport_peripheral: ((read_result >> write_addr) as i32) & 65535,
        pair: register_type as u32,
    }
}

pub fn exec_fft_cmul(core: &mut CoreState, tmp_val: u32) {
    if (0xf0000000 & tmp_val) == 0xa0000000 {
        let fmt = decode_fft_cmul_format(tmp_val);
        if core.window_check(0, fmt.a_handler3 >> 2, 0) { return; }
        let off_val = core.ar(fmt.a_handler3) & !15;
        let signal_direction = if core.sar_m32_pending != 0 { core.sar_m32 as u32 } else { 63 & core.special_registers[SAR_REGISTER] };
        let interrupt_trigger = |idx: u32| -> i32 {
            core.q_reg_int16((8 * fmt.qv + idx) as usize) as i32
        };
        let i2c_command = |idx: u32| -> i32 {
            core.q_reg_int16((8 * fmt.qx + idx) as usize) as i32
        };
        let i2c_interrupt_type = |idx: u32| -> i32 {
            core.q_reg_int16((8 * fmt.qy + idx) as usize) as i32
        };
        let pcnt_register = |v: u32| -> u32 {
            (v >> fmt.sar4) & 65535
        };
        let mut i_val = [0u32; 4];
        if fmt.upd4 == 0 {
            i_val[0] = (interrupt_trigger(0) as u32) & 65535;
            i_val[1] = (interrupt_trigger(1) as u32) & 65535;
            i_val[2] = (interrupt_trigger(2) as u32) & 65535;
            i_val[3] = (interrupt_trigger(3) as u32) & 65535;
        } else if fmt.upd4 == 1 {
            i_val[0] = pcnt_register(i2c_command(0) as u32);
            i_val[1] = pcnt_register(i2c_command(1) as u32);
            i_val[2] = pcnt_register(i2c_command(2) as u32);
            i_val[3] = pcnt_register(i2c_command(3) as u32);
        } else {
            i_val[0] = pcnt_register(i2c_command(0) as u32);
            i_val[1] = pcnt_register(i2c_command(1) as u32);
            i_val[2] = (interrupt_trigger(4) as u32) & 65535;
            i_val[3] = (interrupt_trigger(5) as u32) & 65535;
        }
        let r_val = ((fmt.sel8 >> 1) & 3) as u32;
        let sha_algorithm = (fmt.sel8 & 1) != 0;
        let sha_peripheral_mode = i2c_command(2 * r_val) as i128;
        let s_val = i2c_command(2 * r_val + 1) as i128;
        let op_param = i2c_interrupt_type(2 * r_val) as i128;
        let key_purpose = i2c_interrupt_type(2 * r_val + 1) as i128;
        let mut key_manager_state = sha_peripheral_mode * op_param;
        key_manager_state = if sha_algorithm {
            key_manager_state - s_val * key_purpose
        } else {
            key_manager_state + s_val * key_purpose
        };
        let mut e_val = s_val * op_param;
        e_val = if sha_algorithm {
            e_val + sha_peripheral_mode * key_purpose
        } else {
            e_val - sha_peripheral_mode * key_purpose
        };
        let gpio = ((key_manager_state >> signal_direction) & 65535) as u16;
        let cpu_clock_source = ((e_val >> signal_direction) & 65535) as u16;
        let mut output_signal_index = [0u32; 4];
        if fmt.upd4 == 0 || fmt.upd4 == 1 {
            output_signal_index[0] = (interrupt_trigger(4) as u32) & 65535;
            output_signal_index[1] = (interrupt_trigger(5) as u32) & 65535;
        } else {
            output_signal_index[0] = pcnt_register(i2c_command(2) as u32);
            output_signal_index[1] = pcnt_register(i2c_command(3) as u32);
        }
        output_signal_index[2] = gpio as u32;
        output_signal_index[3] = cpu_clock_source as u32;
        for i in 0..4 {
            write_uint8(core, off_val + 2 * i, (i_val[i as usize] & 255) as u32, 0);
            write_uint8(core, off_val + 2 * i + 1, ((i_val[i as usize] >> 8) & 255) as u32, 0);
        }
        for i in 0..4 {
            write_uint8(core, off_val + 8 + 2 * i, (output_signal_index[i as usize] & 255) as u32, 0);
            write_uint8(core, off_val + 8 + 2 * i + 1, ((output_signal_index[i as usize] >> 8) & 255) as u32, 0);
        }
        core.set_ar(fmt.a_handler3, core.ar(fmt.a_handler3).wrapping_add(core.ar(fmt.a_handler15)));
    } else {
        let fmt = decode_fft_sc_format(tmp_val);
        if core.window_check(0, fmt.a_handler3 >> 2, 0) { return; }
        let result = cmul_pair_result(core, fmt.qx, fmt.qy, fmt.sel8);
        core.set_q_reg_int16(
            (8 * fmt.qz + 2 * result.pair) as usize,
            ((result.lo << 16) >> 16) as i16,
        );
        core.set_q_reg_int16(
            (8 * fmt.qz + 2 * result.pair + 1) as usize,
            ((result.dport_peripheral << 16) >> 16) as i16,
        );
        load_q_registers(core, fmt.qu, core.ar(fmt.a_handler3));
        core.set_ar(fmt.a_handler3, core.ar(fmt.a_handler3).wrapping_add(core.ar(fmt.a_handler15)));
    }
}

pub fn fft_bf_step(
    core: &mut CoreState,
    tmp_val: u32,
    idx_val: u32,
    clock_event: u32,
    simulation_clock: u32,
    read_result: u32,
    write_addr: u32,
    register_type: u32,
) {
    let cfg_val = 31 & core.special_registers[SAR_REGISTER];
    let h_val = core.q_reg_int16((8 * clock_event + 2 * register_type) as usize) as i32;
    let off_val = core.q_reg_int16((8 * clock_event + 2 * register_type + 1) as usize) as i32;
    let signal_direction = core.q_reg_int16((8 * simulation_clock + 2 * register_type) as usize) as i32;
    let interrupt_trigger = core.q_reg_int16((8 * simulation_clock + 2 * register_type + 1) as usize) as i32;
    let i2c_command = core.q_reg_int16((8 * read_result + 2 * register_type) as usize) as i32;
    let i2c_interrupt_type = core.q_reg_int16((8 * read_result + 2 * register_type + 1) as usize) as i32;
    let pcnt_register = clamp_s16_value(h_val + signal_direction);
    let i_val = clamp_s16_value(off_val - interrupt_trigger);
    let (r_val, sha_algorithm) = if write_addr == 0 {
        let prod = (h_val - signal_direction) as i128 * i2c_command as i128
            - (off_val + interrupt_trigger) as i128 * i2c_interrupt_type as i128;
        let r_val = (prod >> cfg_val) as i32;
        let sha = ((h_val - signal_direction) as i128 * i2c_interrupt_type as i128
            + (off_val + interrupt_trigger) as i128 * i2c_command as i128) >> cfg_val;
        (r_val, sha as i32)
    } else {
        let prod = (off_val + interrupt_trigger) as i128 * i2c_interrupt_type as i128
            + (h_val - signal_direction) as i128 * i2c_command as i128;
        let r_val = (prod >> cfg_val) as i32;
        let sha = ((off_val + interrupt_trigger) as i128 * i2c_command as i128
            - (h_val - signal_direction) as i128 * i2c_interrupt_type as i128) >> cfg_val;
        (r_val, sha as i32)
    };
    let r_val = ((r_val << 16) >> 16) as i32;
    let sha_algorithm = ((sha_algorithm << 16) >> 16) as i32;
    core.set_q_reg_int16(
        (8 * tmp_val + 2 * register_type) as usize,
        ((pcnt_register + r_val) << 16 >> 16) as i16,
    );
    core.set_q_reg_int16(
        (8 * tmp_val + 2 * register_type + 1) as usize,
        ((i_val + sha_algorithm) << 16 >> 16) as i16,
    );
    core.set_q_reg_int16(
        (8 * idx_val + 2 * register_type) as usize,
        ((pcnt_register - r_val) << 16 >> 16) as i16,
    );
    core.set_q_reg_int16(
        (8 * idx_val + 2 * register_type + 1) as usize,
        ((sha_algorithm - i_val) << 16 >> 16) as i16,
    );
}

pub fn exec_fft_ams(core: &mut CoreState, tmp_val: u32) {
    let idx_val = (tmp_val >> 24) & 252;
    if idx_val >= 160 && idx_val < 168 {
        let fmt = decode_fft_ldst_format(tmp_val);
        if core.window_check(0, fmt.a_handler3 >> 2, 0)
            || core.window_check(0, fmt.as0 >> 2, 0)
        {
            return;
        }
        let off_val = core.ar(fmt.a_handler3) & !15;
        let len_val = core.ar(fmt.as0);
        let val_val = if fmt.sel2 == 0 { 1u32 } else { 0u32 };
        let i2c_command = |v: u32| -> u32 {
            (((v as i16) as i32) >> val_val) as u32 & 65535
        };
        let i2c_interrupt_type = i2c_command(len_val & 65535);
        let pcnt_register = i2c_command((len_val >> 16) & 65535);
        write_uint8(core, off_val, (i2c_interrupt_type & 255) as u32, 0);
        write_uint8(core, off_val + 1, ((i2c_interrupt_type >> 8) & 255) as u32, 0);
        write_uint8(core, off_val + 2, (pcnt_register & 255) as u32, 0);
        write_uint8(core, off_val + 3, ((pcnt_register >> 8) & 255) as u32, 0);
        for i in 0..6 {
            let val = i2c_command(core.q_reg_int16((8 * fmt.qv + i) as usize) as u32);
            write_uint8(core, off_val + 4 + 2 * i, (val & 255) as u32, 0);
            write_uint8(core, off_val + 4 + 2 * i + 1, ((val >> 8) & 255) as u32, 0);
        }
        let i_val = 31 & core.special_registers[SAR_REGISTER];
        let rmt_channel_register = clamp_s16_value(
            core.q_reg_int16((8 * fmt.qx + 6) as usize) as i32
                + core.q_reg_int16((8 * fmt.qy + 6) as usize) as i32,
        );
        let sha_algorithm = clamp_s16_value(
            core.q_reg_int16((8 * fmt.qx + 7) as usize) as i32
                - core.q_reg_int16((8 * fmt.qy + 7) as usize) as i32,
        );
        let mut sha_peripheral_mode = (core.q_reg_int16((8 * fmt.qx + 6) as usize) as i128
            - core.q_reg_int16((8 * fmt.qy + 6) as usize) as i128)
            * core.q_reg_int16((8 * fmt.qm + 6) as usize) as i128;
        if fmt.sel2 == 0 {
            sha_peripheral_mode -= (core.q_reg_int16((8 * fmt.qx + 7) as usize) as i128
                + core.q_reg_int16((8 * fmt.qy + 7) as usize) as i128)
                * core.q_reg_int16((8 * fmt.qm + 7) as usize) as i128;
        } else {
            sha_peripheral_mode += (core.q_reg_int16((8 * fmt.qx + 7) as usize) as i128
                + core.q_reg_int16((8 * fmt.qy + 7) as usize) as i128)
                * core.q_reg_int16((8 * fmt.qm + 7) as usize) as i128;
        }
        let s_val = (sha_peripheral_mode >> i_val) as i32;
        sha_peripheral_mode = (core.q_reg_int16((8 * fmt.qx + 6) as usize) as i128
            - core.q_reg_int16((8 * fmt.qy + 6) as usize) as i128)
            * core.q_reg_int16((8 * fmt.qm + 7) as usize) as i128;
        if fmt.sel2 == 0 {
            sha_peripheral_mode += (core.q_reg_int16((8 * fmt.qx + 7) as usize) as i128
                + core.q_reg_int16((8 * fmt.qy + 7) as usize) as i128)
                * core.q_reg_int16((8 * fmt.qm + 6) as usize) as i128;
        } else {
            sha_peripheral_mode -= (core.q_reg_int16((8 * fmt.qx + 7) as usize) as i128
                + core.q_reg_int16((8 * fmt.qy + 7) as usize) as i128)
                * core.q_reg_int16((8 * fmt.qm + 6) as usize) as i128;
        }
        let op_param = (sha_peripheral_mode >> i_val) as i32;
        let key_purpose = ((rmt_channel_register + s_val) & 65535) as i16;
        let key_manager_state = ((sha_algorithm + op_param) & 65535) as i16;
        core.set_q_reg_int16(
            (8 * fmt.qz1 + 6) as usize,
            (((rmt_channel_register - s_val) << 16) >> 16) as i16,
        );
        core.set_q_reg_int16(
            (8 * fmt.qz1 + 7) as usize,
            (((op_param - sha_algorithm) << 16) >> 16) as i16,
        );
        core.set_ar(
            fmt.as0,
            ((key_manager_state as u32) << 16) | (key_purpose as u32),
        );
        core.set_ar(fmt.a_handler3, core.ar(fmt.a_handler3).wrapping_add(16));
    } else {
        let fmt = decode_fft_ams_format(tmp_val);
        if core.window_check(0, fmt.a_handler3 >> 2, 0) { return; }
        let len_val = idx_val;
        let pair_idx = if len_val == 212 { 0 } else if len_val == 216 { 2 } else { 1 };
        fft_bf_step(
            core,
            fmt.qx,
            fmt.qy,
            fmt.qz,
            fmt.qz1,
            fmt.qm,
            fmt.sel2,
            pair_idx,
        );
        if len_val == 212 {
            let tmp = core.ar(fmt.a_handler3) & !15;
            let mut idx_buf = [0u8; 32];
            let mut ua_bytes = [0u8; 16];
            for i in 0..4 {
                let le = core.ua_state[i].to_le_bytes();
                ua_bytes[i * 4..(i + 1) * 4].copy_from_slice(&le);
            }
            idx_buf[..16].copy_from_slice(&ua_bytes);
            for i in 0..16 {
                idx_buf[16 + i] = read_uint8(core, tmp + i as u32) as u8;
            }
            let write_addr = (core.sar_byte() & 15) as usize;
            for i in 0..16 {
                core.q_registers[(16 * fmt.qu + i as u32) as usize] = idx_buf[i + write_addr];
            }
            for i in 0..16 {
                ua_bytes[i] = idx_buf[16 + i];
            }
            for i in 0..4 {
                core.ua_state[i] = u32::from_le_bytes(
                    ua_bytes[i * 4..(i + 1) * 4].try_into().unwrap(),
                );
            }
            core.set_ar(fmt.a_handler3, core.ar(fmt.a_handler3).wrapping_add(16));
        } else if len_val == 216 {
            load_q_registers(core, fmt.qu, core.ar(fmt.a_handler3));
            let mut tmp_buf = [0u32; 4];
            for i in 0..4 {
            tmp_buf[i] = core.q_reg_uint32((4 * fmt.qu + i as u32) as usize);
        }
        for i in 0..4 {
            core.set_q_reg_uint32((4 * fmt.qu + (3 - i as u32)) as usize, tmp_buf[i]);
            }
            core.set_ar(fmt.a_handler3, core.ar(fmt.a_handler3).wrapping_sub(16));
        } else {
            load_q_registers(core, fmt.qu, core.ar(fmt.a_handler3));
            core.set_ar(fmt.a_handler3, core.ar(fmt.a_handler3).wrapping_add(16));
        }
    }
}

pub fn decode_vec_shift(cpu_val: u32) -> VecShift {
    let tmp_val = (cpu_val >> 16) & 7;
    let idx_val = ((cpu_val >> 14) & 1)
        | (((cpu_val >> 15) & 1) << 1)
        | ((1 & cpu_val) << 2);
    VecShift {
        qu: tmp_val,
        qs0: idx_val,
        qs1: (cpu_val >> 20) & 7,
        a_handler3: (cpu_val >> 4) & 15,
    }
}

pub fn decode_vec_shift_amount(cpu_val: u32) -> i32 {
    let val = ((cpu_val >> 8) & 1)
        | (((cpu_val >> 9) & 1) << 1)
        | (((cpu_val >> 23) & 1) << 2)
        | (((cpu_val >> 12) & 1) << 3)
        | (((cpu_val >> 13) & 1) << 4)
        | (((cpu_val >> 19) & 1) << 5)
        | (((cpu_val >> 24) & 1) << 6)
        | (((cpu_val >> 25) & 1) << 7);
    ((val as i8) as i32) * 16
}

pub struct SxInstEntry {
    pub name: &'static str,
    pub mask: u32,
    pub opcode: u32,
    pub func: fn(&mut CoreState, u32),
}

fn sx_ee_ldf_128_decode_vq_format_xp(core: &mut CoreState, tmp_val: u32) {
    let idx_val = (tmp_val >> 20) & 15;
    let clock_event = (tmp_val >> 12) & 15;
    let sim_clock = (((tmp_val >> 16) & 7) << 1) | (1 & tmp_val);
    let read_result = (((tmp_val >> 24) & 7) << 1) | ((tmp_val >> 19) & 1);
    let write_addr = (tmp_val >> 4) & 15;
    let register_type = (tmp_val >> 8) & 15;
    if core.window_check(0, write_addr >> 2, 0) { return; }
    let cfg_val = (-16i32 as u32) & core.ar(write_addr);
    core.float_registers[idx_val as usize] = read_uint32(core, cfg_val);
    core.float_registers[clock_event as usize] = read_uint32(core, cfg_val + 4);
    core.float_registers[sim_clock as usize] = read_uint32(core, cfg_val + 8);
    core.float_registers[read_result as usize] = read_uint32(core, cfg_val + 12);
    let h_val = (((register_type << 28) as i32) >> 28) << 4;
    core.set_ar(write_addr, core.ar(write_addr).wrapping_add(h_val as u32));
}

fn sx_ee_ldf_128_xp(core: &mut CoreState, tmp_val: u32) {
    let idx_val = (tmp_val >> 20) & 15;
    let clock_event = (tmp_val >> 12) & 15;
    let sim_clock = (((tmp_val >> 16) & 7) << 1) | (1 & tmp_val);
    let read_result = (((tmp_val >> 24) & 7) << 1) | ((tmp_val >> 19) & 1);
    let write_addr = (tmp_val >> 4) & 15;
    let register_type = (tmp_val >> 8) & 15;
    if core.window_check(0, write_addr >> 2, 0) { return; }
    let cfg_val = (-16i32 as u32) & core.ar(write_addr);
    core.float_registers[idx_val as usize] = read_uint32(core, cfg_val);
    core.float_registers[clock_event as usize] = read_uint32(core, cfg_val + 4);
    core.float_registers[sim_clock as usize] = read_uint32(core, cfg_val + 8);
    core.float_registers[read_result as usize] = read_uint32(core, cfg_val + 12);
    core.set_ar(write_addr, core.ar(write_addr).wrapping_add(core.ar(register_type)));
}

fn sx_ee_stf_128_decode_vq_format_xp(core: &mut CoreState, tmp_val: u32) {
    let idx_val = (tmp_val >> 20) & 15;
    let clock_event = (tmp_val >> 12) & 15;
    let sim_clock = (((tmp_val >> 16) & 7) << 1) | (1 & tmp_val);
    let read_result = (((tmp_val >> 24) & 7) << 1) | ((tmp_val >> 19) & 1);
    let write_addr = (tmp_val >> 4) & 15;
    let register_type = (tmp_val >> 8) & 15;
    if core.window_check(0, write_addr >> 2, 0) { return; }
    let cfg_val = (-16i32 as u32) & core.ar(write_addr);
    write_uint32(core, cfg_val, core.float_registers[idx_val as usize], 0);
    write_uint32(core, cfg_val + 4, core.float_registers[clock_event as usize], 0);
    write_uint32(core, cfg_val + 8, core.float_registers[sim_clock as usize], 0);
    write_uint32(core, cfg_val + 12, core.float_registers[read_result as usize], 0);
    let h_val = (((register_type << 28) as i32) >> 28) << 4;
    core.set_ar(write_addr, core.ar(write_addr).wrapping_add(h_val as u32));
}

fn sx_ee_stf_128_xp(core: &mut CoreState, tmp_val: u32) {
    let idx_val = (tmp_val >> 20) & 15;
    let clock_event = (tmp_val >> 12) & 15;
    let sim_clock = (((tmp_val >> 16) & 7) << 1) | (1 & tmp_val);
    let read_result = (((tmp_val >> 24) & 7) << 1) | ((tmp_val >> 19) & 1);
    let write_addr = (tmp_val >> 4) & 15;
    let register_type = (tmp_val >> 8) & 15;
    if core.window_check(0, write_addr >> 2, 0) { return; }
    let cfg_val = (-16i32 as u32) & core.ar(write_addr);
    write_uint32(core, cfg_val, core.float_registers[idx_val as usize], 0);
    write_uint32(core, cfg_val + 4, core.float_registers[clock_event as usize], 0);
    write_uint32(core, cfg_val + 8, core.float_registers[sim_clock as usize], 0);
    write_uint32(core, cfg_val + 12, core.float_registers[read_result as usize], 0);
    core.set_ar(write_addr, core.ar(write_addr).wrapping_add(core.ar(register_type)));
}

fn sx_ee_ldf_64_decode_vq_format_xp(core: &mut CoreState, tmp_val: u32) {
    let fmt = decode_float_load_format(tmp_val);
    if core.window_check(0, fmt.a_handler3 >> 2, 0) { return; }
    let write_addr = (-8i32 as u32) & core.ar(fmt.a_handler3);
    core.float_registers[fmt.fu0 as usize] = read_uint32(core, write_addr);
    core.float_registers[fmt.fu1 as usize] = read_uint32(core, write_addr + 4);
    core.set_ar(fmt.a_handler3, core.ar(fmt.a_handler3).wrapping_add(fmt.offset as u32));
}

fn sx_ee_stf_64_decode_vq_format_xp(core: &mut CoreState, tmp_val: u32) {
    let fmt = decode_float_load_format(tmp_val);
    if core.window_check(0, fmt.a_handler3 >> 2, 0) { return; }
    let write_addr = (-8i32 as u32) & core.ar(fmt.a_handler3);
    write_uint32(core, write_addr, core.float_registers[fmt.fu0 as usize], 0);
    write_uint32(core, write_addr + 4, core.float_registers[fmt.fu1 as usize], 0);
    core.set_ar(fmt.a_handler3, core.ar(fmt.a_handler3).wrapping_add(fmt.offset as u32));
}

fn sx_ee_vmulas_reg_off64_qacc_ledc_channel_decode_vq_format_xp(core: &mut CoreState, tmp_val: u32) {
    vmulas_ip(core, tmp_val, "regOff64", "qacc");
}

fn sx_ee_vmulas_u8_qacc_ledc_channel_decode_vq_format_xp(core: &mut CoreState, tmp_val: u32) {
    vmulas_ip(core, tmp_val, "u8", "qacc");
}

fn sx_ee_vmulas_s16_qacc_ledc_channel_decode_vq_format_xp(core: &mut CoreState, tmp_val: u32) {
    vmulas_ip(core, tmp_val, "s16", "qacc");
}

fn sx_ee_vmulas_u16_qacc_ledc_channel_decode_vq_format_xp(core: &mut CoreState, tmp_val: u32) {
    vmulas_ip(core, tmp_val, "u16", "qacc");
}

fn sx_ee_vmulas_reg_off64_accx_ledc_channel_decode_vq_format_xp(core: &mut CoreState, tmp_val: u32) {
    vmulas_ip(core, tmp_val, "regOff64", "accx");
}

fn sx_ee_vmulas_u8_accx_ledc_channel_decode_vq_format_xp(core: &mut CoreState, tmp_val: u32) {
    vmulas_ip(core, tmp_val, "u8", "accx");
}

fn sx_ee_vmulas_s16_accx_ledc_channel_decode_vq_format_xp(core: &mut CoreState, tmp_val: u32) {
    vmulas_ip(core, tmp_val, "s16", "accx");
}

fn sx_ee_vmulas_u16_accx_ledc_channel_decode_vq_format_xp(core: &mut CoreState, tmp_val: u32) {
    vmulas_ip(core, tmp_val, "u16", "accx");
}

fn sx_ee_vmulas_reg_off64_qacc_ledc_channel_xp(core: &mut CoreState, tmp_val: u32) {
    vmulas_xp(core, tmp_val, "regOff64", "qacc");
}

fn sx_ee_vmulas_u8_qacc_ledc_channel_xp(core: &mut CoreState, tmp_val: u32) {
    vmulas_xp(core, tmp_val, "u8", "qacc");
}

fn sx_ee_vmulas_s16_qacc_ledc_channel_xp(core: &mut CoreState, tmp_val: u32) {
    vmulas_xp(core, tmp_val, "s16", "qacc");
}

fn sx_ee_vmulas_u16_qacc_ledc_channel_xp(core: &mut CoreState, tmp_val: u32) {
    vmulas_xp(core, tmp_val, "u16", "qacc");
}

fn sx_ee_vmulas_reg_off64_accx_ledc_channel_xp(core: &mut CoreState, tmp_val: u32) {
    vmulas_xp(core, tmp_val, "regOff64", "accx");
}

fn sx_ee_vmulas_u8_accx_ledc_channel_xp(core: &mut CoreState, tmp_val: u32) {
    vmulas_xp(core, tmp_val, "u8", "accx");
}

fn sx_ee_vmulas_s16_accx_ledc_channel_xp(core: &mut CoreState, tmp_val: u32) {
    vmulas_xp(core, tmp_val, "s16", "accx");
}

fn sx_ee_vmulas_u16_accx_ledc_channel_xp(core: &mut CoreState, tmp_val: u32) {
    vmulas_xp(core, tmp_val, "u16", "accx");
}

fn sx_ee_vmulas_reg_off64_qacc_ledc_channel_decode_vq_format_xp_qup(core: &mut CoreState, tmp_val: u32) {
    vmulas_qup(core, tmp_val, "regOff64", "qacc", "decodeVQFormatXp");
}

fn sx_ee_vmulas_s16_qacc_ledc_channel_decode_vq_format_xp_qup(core: &mut CoreState, tmp_val: u32) {
    vmulas_qup(core, tmp_val, "s16", "qacc", "decodeVQFormatXp");
}

fn sx_ee_vmulas_u8_qacc_ledc_channel_decode_vq_format_xp_qup(core: &mut CoreState, tmp_val: u32) {
    vmulas_qup(core, tmp_val, "u8", "qacc", "decodeVQFormatXp");
}

fn sx_ee_vmulas_u16_qacc_ledc_channel_decode_vq_format_xp_qup(core: &mut CoreState, tmp_val: u32) {
    vmulas_qup(core, tmp_val, "u16", "qacc", "decodeVQFormatXp");
}

fn sx_ee_vmulas_reg_off64_accx_ledc_channel_decode_vq_format_xp_qup(core: &mut CoreState, tmp_val: u32) {
    vmulas_qup(core, tmp_val, "regOff64", "accx", "decodeVQFormatXp");
}

fn sx_ee_vmulas_s16_accx_ledc_channel_decode_vq_format_xp_qup(core: &mut CoreState, tmp_val: u32) {
    vmulas_qup(core, tmp_val, "s16", "accx", "decodeVQFormatXp");
}

fn sx_ee_vmulas_u8_accx_ledc_channel_decode_vq_format_xp_qup(core: &mut CoreState, tmp_val: u32) {
    vmulas_qup(core, tmp_val, "u8", "accx", "decodeVQFormatXp");
}

fn sx_ee_vmulas_u16_accx_ledc_channel_decode_vq_format_xp_qup(core: &mut CoreState, tmp_val: u32) {
    vmulas_qup(core, tmp_val, "u16", "accx", "decodeVQFormatXp");
}

fn sx_ee_vmulas_reg_off64_qacc_ledc_channel_xp_qup(core: &mut CoreState, tmp_val: u32) {
    vmulas_qup(core, tmp_val, "regOff64", "qacc", "xp");
}

fn sx_ee_vmulas_s16_qacc_ledc_channel_xp_qup(core: &mut CoreState, tmp_val: u32) {
    vmulas_qup(core, tmp_val, "s16", "qacc", "xp");
}

fn sx_ee_vmulas_u8_qacc_ledc_channel_xp_qup(core: &mut CoreState, tmp_val: u32) {
    vmulas_qup(core, tmp_val, "u8", "qacc", "xp");
}

fn sx_ee_vmulas_u16_qacc_ledc_channel_xp_qup(core: &mut CoreState, tmp_val: u32) {
    vmulas_qup(core, tmp_val, "u16", "qacc", "xp");
}

fn sx_ee_vmulas_reg_off64_accx_ledc_channel_xp_qup(core: &mut CoreState, tmp_val: u32) {
    vmulas_qup(core, tmp_val, "regOff64", "accx", "xp");
}

fn sx_ee_vmulas_s16_accx_ledc_channel_xp_qup(core: &mut CoreState, tmp_val: u32) {
    vmulas_qup(core, tmp_val, "s16", "accx", "xp");
}

fn sx_ee_vmulas_u8_accx_ledc_channel_xp_qup(core: &mut CoreState, tmp_val: u32) {
    vmulas_qup(core, tmp_val, "u8", "accx", "xp");
}

fn sx_ee_vmulas_u16_accx_ledc_channel_xp_qup(core: &mut CoreState, tmp_val: u32) {
    vmulas_qup(core, tmp_val, "u16", "accx", "xp");
}

fn sx_ee_vmulas_reg_off64_qacc_ldbc_incp_qup(core: &mut CoreState, tmp_val: u32) {
    vmulas_qup(core, tmp_val, "regOff64", "qacc", "ldbc");
}

fn sx_ee_vmulas_s16_qacc_ldbc_incp_qup(core: &mut CoreState, tmp_val: u32) {
    vmulas_qup(core, tmp_val, "s16", "qacc", "ldbc");
}

fn sx_ee_vmulas_u8_qacc_ldbc_incp_qup(core: &mut CoreState, tmp_val: u32) {
    vmulas_qup(core, tmp_val, "u8", "qacc", "ldbc");
}

fn sx_ee_vmulas_u16_qacc_ldbc_incp_qup(core: &mut CoreState, tmp_val: u32) {
    vmulas_qup(core, tmp_val, "u16", "qacc", "ldbc");
}

fn sx_ee_vadds_reg_off64_ledc_channel_incp(core: &mut CoreState, tmp_val: u32) {
    let fmt = decode_basic_vec_format(tmp_val);
    if !core.window_check(0, fmt.a_handler3 >> 2, 0) {
        vadds8_vec_op(core, fmt.qz, fmt.qx, fmt.qy);
        load_q_registers(core, fmt.qa, core.ar(fmt.a_handler3));
        core.set_ar(fmt.a_handler3, core.ar(fmt.a_handler3).wrapping_add(16));
    }
}

fn sx_ee_vadds_s16_ledc_channel_incp(core: &mut CoreState, tmp_val: u32) {
    let fmt = decode_basic_vec_format(tmp_val);
    if !core.window_check(0, fmt.a_handler3 >> 2, 0) {
        vadds16_vec_op(core, fmt.qz, fmt.qx, fmt.qy);
        load_q_registers(core, fmt.qa, core.ar(fmt.a_handler3));
        core.set_ar(fmt.a_handler3, core.ar(fmt.a_handler3).wrapping_add(16));
    }
}

fn sx_ee_vmax_reg_off64_ledc_channel_incp(core: &mut CoreState, tmp_val: u32) {
    let fmt = decode_basic_vec_format(tmp_val);
    if !core.window_check(0, fmt.a_handler3 >> 2, 0) {
        vmax8_vec_op(core, fmt.qz, fmt.qx, fmt.qy);
        load_q_registers(core, fmt.qa, core.ar(fmt.a_handler3));
        core.set_ar(fmt.a_handler3, core.ar(fmt.a_handler3).wrapping_add(16));
    }
}

fn sx_ee_vmax_s16_ledc_channel_incp(core: &mut CoreState, tmp_val: u32) {
    let fmt = decode_basic_vec_format(tmp_val);
    if !core.window_check(0, fmt.a_handler3 >> 2, 0) {
        vmax16_vec_op(core, fmt.qz, fmt.qx, fmt.qy);
        load_q_registers(core, fmt.qa, core.ar(fmt.a_handler3));
        core.set_ar(fmt.a_handler3, core.ar(fmt.a_handler3).wrapping_add(16));
    }
}

fn sx_ee_vadds_s32_ledc_channel_incp(core: &mut CoreState, tmp_val: u32) {
    exec_vec_op_incp(core, tmp_val, vadds32_vec_op);
}

fn sx_ee_vsubs_reg_off64_ledc_channel_incp(core: &mut CoreState, tmp_val: u32) {
    exec_vec_op_incp(core, tmp_val, vsubs8_vec_op);
}

fn sx_ee_vsubs_s16_ledc_channel_incp(core: &mut CoreState, tmp_val: u32) {
    exec_vec_op_incp(core, tmp_val, vsubs16_vec_op);
}

fn sx_ee_vsubs_s32_ledc_channel_incp(core: &mut CoreState, tmp_val: u32) {
    exec_vec_op_incp(core, tmp_val, vsubs32_vec_op);
}

fn sx_ee_vmax_s32_ledc_channel_incp(core: &mut CoreState, tmp_val: u32) {
    exec_vec_op_incp(core, tmp_val, vmax32_vec_op);
}

fn sx_ee_vmin_reg_off64_ledc_channel_incp(core: &mut CoreState, tmp_val: u32) {
    exec_vec_op_incp(core, tmp_val, vmin8_vec_op);
}

fn sx_ee_vmin_s16_ledc_channel_incp(core: &mut CoreState, tmp_val: u32) {
    exec_vec_op_incp(core, tmp_val, vmin16_vec_op);
}

fn sx_ee_vmin_s32_ledc_channel_incp(core: &mut CoreState, tmp_val: u32) {
    exec_vec_op_incp(core, tmp_val, vmin32_vec_op);
}

fn sx_ee_vmul_reg_off64_ledc_channel_incp(core: &mut CoreState, tmp_val: u32) {
    exec_vec_op_incp(core, tmp_val, vmul_s8_shift_vec_op);
}

fn sx_ee_vmul_s16_ledc_channel_incp(core: &mut CoreState, tmp_val: u32) {
    exec_vec_op_incp(core, tmp_val, vmul_s16_shift_vec_op);
}

fn sx_ee_vmul_u8_ledc_channel_incp(core: &mut CoreState, tmp_val: u32) {
    exec_vec_op_incp(core, tmp_val, vmul_u8_shift_vec_op);
}

fn sx_ee_vmul_u16_ledc_channel_incp(core: &mut CoreState, tmp_val: u32) {
    exec_vec_op_incp(core, tmp_val, vmul_u16_shift_vec_op);
}

fn sx_ee_vadds_reg_off64_decode_vec_store_format_incp(core: &mut CoreState, tmp_val: u32) {
    exec_vec_store_incp(core, tmp_val, vadds8_vec_op);
}

fn sx_ee_vadds_s16_decode_vec_store_format_incp(core: &mut CoreState, tmp_val: u32) {
    exec_vec_store_incp(core, tmp_val, vadds16_vec_op);
}

fn sx_ee_vadds_s32_decode_vec_store_format_incp(core: &mut CoreState, tmp_val: u32) {
    exec_vec_store_incp(core, tmp_val, vadds32_vec_op);
}

fn sx_ee_vsubs_reg_off64_decode_vec_store_format_incp(core: &mut CoreState, tmp_val: u32) {
    exec_vec_store_incp(core, tmp_val, vsubs8_vec_op);
}

fn sx_ee_vsubs_s16_decode_vec_store_format_incp(core: &mut CoreState, tmp_val: u32) {
    exec_vec_store_incp(core, tmp_val, vsubs16_vec_op);
}

fn sx_ee_vsubs_s32_decode_vec_store_format_incp(core: &mut CoreState, tmp_val: u32) {
    exec_vec_store_incp(core, tmp_val, vsubs32_vec_op);
}

fn sx_ee_vmax_reg_off64_decode_vec_store_format_incp(core: &mut CoreState, tmp_val: u32) {
    exec_vec_store_incp(core, tmp_val, vmax8_vec_op);
}

fn sx_ee_vmax_s16_decode_vec_store_format_incp(core: &mut CoreState, tmp_val: u32) {
    exec_vec_store_incp(core, tmp_val, vmax16_vec_op);
}

fn sx_ee_vmax_s32_decode_vec_store_format_incp(core: &mut CoreState, tmp_val: u32) {
    exec_vec_store_incp(core, tmp_val, vmax32_vec_op);
}

fn sx_ee_vmin_reg_off64_decode_vec_store_format_incp(core: &mut CoreState, tmp_val: u32) {
    exec_vec_store_incp(core, tmp_val, vmin8_vec_op);
}

fn sx_ee_vmin_s16_decode_vec_store_format_incp(core: &mut CoreState, tmp_val: u32) {
    exec_vec_store_incp(core, tmp_val, vmin16_vec_op);
}

fn sx_ee_vmin_s32_decode_vec_store_format_incp(core: &mut CoreState, tmp_val: u32) {
    exec_vec_store_incp(core, tmp_val, vmin32_vec_op);
}

fn sx_ee_vmul_reg_off64_decode_vec_store_format_incp(core: &mut CoreState, tmp_val: u32) {
    exec_vec_store_incp(core, tmp_val, vmul_s8_shift_vec_op);
}

fn sx_ee_vmul_s16_decode_vec_store_format_incp(core: &mut CoreState, tmp_val: u32) {
    exec_vec_store_incp(core, tmp_val, vmul_s16_shift_vec_op);
}

fn sx_ee_vmul_u8_decode_vec_store_format_incp(core: &mut CoreState, tmp_val: u32) {
    exec_vec_store_incp(core, tmp_val, vmul_u8_shift_vec_op);
}

fn sx_ee_vmul_u16_decode_vec_store_format_incp(core: &mut CoreState, tmp_val: u32) {
    exec_vec_store_incp(core, tmp_val, vmul_u16_shift_vec_op);
}

fn sx_ee_cmul_s16_ledc_channel_incp(core: &mut CoreState, tmp_val: u32) {
    let fmt = decode_cmplx_vec_format(tmp_val);
    if !core.window_check(0, fmt.a_handler3 >> 2, 0) {
        cmul_pair(core, fmt.qz, fmt.qx, fmt.qy, fmt.sel);
        load_q_registers(core, fmt.qu, core.ar(fmt.a_handler3));
        core.set_ar(fmt.a_handler3, core.ar(fmt.a_handler3).wrapping_add(16));
    }
}

fn sx_ee_cmul_s16_decode_vec_store_format_incp(core: &mut CoreState, tmp_val: u32) {
    let fmt = decode_cmplx_vec_format(tmp_val);
    let register_type = ((tmp_val >> 20) & 1)
        | (((tmp_val >> 21) & 1) << 1)
        | (((tmp_val >> 22) & 1) << 2);
    if core.window_check(0, fmt.a_handler3 >> 2, 0) { return; }
    cmul_pair(core, fmt.qz, fmt.qx, fmt.qy, fmt.sel);
    let cfg_val = (-16i32 as u32) & core.ar(fmt.a_handler3);
    for r in 0..16 {
        write_uint8(core, cfg_val + r, core.q_registers[(16 * register_type + r) as usize] as u32, 0);
    }
    core.set_ar(fmt.a_handler3, core.ar(fmt.a_handler3).wrapping_add(16));
}

fn sx_ee_fft_r2bf_s16_decode_vec_store_format_incp(core: &mut CoreState, tmp_val: u32) {
    let fmt = decode_fft_r2bf(tmp_val);
    if core.window_check(0, fmt.a_handler3 >> 2, 0) { return; }
    let register_type = (-16i32 as u32) & core.ar(fmt.a_handler3);
    for r in 0..8 {
        let val = (core.q_reg_int16((8 * fmt.qx + r) as usize) as i32)
            - (core.q_reg_int16((8 * fmt.qy + r) as usize) as i32);
        core.set_q_reg_int16((8 * fmt.qa0 + r) as usize, clamp_s16_value(val) as i16);
    }
    for r in 0..8 {
        let mut idx_val = (core.q_reg_int16((8 * fmt.qx + r) as usize) as i32)
            + (core.q_reg_int16((8 * fmt.qy + r) as usize) as i32);
        idx_val >>= fmt.sar4;
        write_uint8(core, register_type + 2 * r, (255 & idx_val) as u32, 0);
        write_uint8(core, register_type + 2 * r + 1, ((idx_val >> 8) & 255) as u32, 0);
    }
    core.set_ar(fmt.a_handler3, core.ar(fmt.a_handler3).wrapping_add(16));
}

fn sx_ee_src_q_ledc_channel_decode_vq_format_xp(core: &mut CoreState, tmp_val: u32) {
    let fmt = decode_vec_shift(tmp_val);
    let write_addr = decode_vec_shift_amount(tmp_val) as u32;
    if !core.window_check(0, fmt.a_handler3 >> 2, 0) {
        shift_q_registers(core, fmt.qs0, fmt.qs1);
        load_q_registers(core, fmt.qu, core.ar(fmt.a_handler3));
        core.set_ar(fmt.a_handler3, core.ar(fmt.a_handler3).wrapping_add(write_addr));
    }
}

fn sx_ee_src_q_ledc_channel_xp(core: &mut CoreState, tmp_val: u32) {
    let fmt = decode_vec_shift(tmp_val);
    let write_addr = (tmp_val >> 8) & 15;
    if !core.window_check(0, fmt.a_handler3 >> 2, 0) {
        shift_q_registers(core, fmt.qs0, fmt.qs1);
        load_q_registers(core, fmt.qu, core.ar(fmt.a_handler3));
        core.set_ar(fmt.a_handler3, core.ar(fmt.a_handler3).wrapping_add(core.ar(write_addr)));
    }
}
pub static SX_INST_TABLE: &[SxInstEntry] = &[
    SxInstEntry { name: "ee.ldf.128.decodeVQFormatXp", mask: 0xf800000e, opcode: 0x8000000e, func: sx_ee_ldf_128_decode_vq_format_xp },
    SxInstEntry { name: "ee.ldf.128.xp", mask: 0xf800000e, opcode: 0x8800000e, func: sx_ee_ldf_128_xp },
    SxInstEntry { name: "ee.stf.128.decodeVQFormatXp", mask: 0xf800000e, opcode: 0x9000000e, func: sx_ee_stf_128_decode_vq_format_xp },
    SxInstEntry { name: "ee.stf.128.xp", mask: 0xf800000e, opcode: 0x9800000e, func: sx_ee_stf_128_xp },
    SxInstEntry { name: "ee.ldf.64.decodeVQFormatXp", mask: 0xfc000e0e, opcode: 0xe000040e, func: sx_ee_ldf_64_decode_vq_format_xp },
    SxInstEntry { name: "ee.stf.64.decodeVQFormatXp", mask: 0xfc000e0e, opcode: 0xe000060e, func: sx_ee_stf_64_decode_vq_format_xp },
    SxInstEntry { name: "ee.vmulas.regOff64.qacc.LedcChannel.decodeVQFormatXp", mask: 0xf077000e, opcode: 0xf003000e, func: sx_ee_vmulas_reg_off64_qacc_ledc_channel_decode_vq_format_xp },
    SxInstEntry { name: "ee.vmulas.u8.qacc.LedcChannel.decodeVQFormatXp", mask: 0xf077000e, opcode: 0xf007000e, func: sx_ee_vmulas_u8_qacc_ledc_channel_decode_vq_format_xp },
    SxInstEntry { name: "ee.vmulas.s16.qacc.LedcChannel.decodeVQFormatXp", mask: 0xf077000e, opcode: 0xf001000e, func: sx_ee_vmulas_s16_qacc_ledc_channel_decode_vq_format_xp },
    SxInstEntry { name: "ee.vmulas.u16.qacc.LedcChannel.decodeVQFormatXp", mask: 0xf077000e, opcode: 0xf005000e, func: sx_ee_vmulas_u16_qacc_ledc_channel_decode_vq_format_xp },
    SxInstEntry { name: "ee.vmulas.regOff64.accx.LedcChannel.decodeVQFormatXp", mask: 0xf077000e, opcode: 0xf002000e, func: sx_ee_vmulas_reg_off64_accx_ledc_channel_decode_vq_format_xp },
    SxInstEntry { name: "ee.vmulas.u8.accx.LedcChannel.decodeVQFormatXp", mask: 0xf077000e, opcode: 0xf006000e, func: sx_ee_vmulas_u8_accx_ledc_channel_decode_vq_format_xp },
    SxInstEntry { name: "ee.vmulas.s16.accx.LedcChannel.decodeVQFormatXp", mask: 0xf077000e, opcode: 0xf000000e, func: sx_ee_vmulas_s16_accx_ledc_channel_decode_vq_format_xp },
    SxInstEntry { name: "ee.vmulas.u16.accx.LedcChannel.decodeVQFormatXp", mask: 0xf077000e, opcode: 0xf004000e, func: sx_ee_vmulas_u16_accx_ledc_channel_decode_vq_format_xp },
    SxInstEntry { name: "ee.vmulas.regOff64.qacc.LedcChannel.xp", mask: 0xf077000e, opcode: 0xf013000e, func: sx_ee_vmulas_reg_off64_qacc_ledc_channel_xp },
    SxInstEntry { name: "ee.vmulas.u8.qacc.LedcChannel.xp", mask: 0xf077000e, opcode: 0xf017000e, func: sx_ee_vmulas_u8_qacc_ledc_channel_xp },
    SxInstEntry { name: "ee.vmulas.s16.qacc.LedcChannel.xp", mask: 0xf077000e, opcode: 0xf011000e, func: sx_ee_vmulas_s16_qacc_ledc_channel_xp },
    SxInstEntry { name: "ee.vmulas.u16.qacc.LedcChannel.xp", mask: 0xf077000e, opcode: 0xf015000e, func: sx_ee_vmulas_u16_qacc_ledc_channel_xp },
    SxInstEntry { name: "ee.vmulas.regOff64.accx.LedcChannel.xp", mask: 0xf077000e, opcode: 0xf012000e, func: sx_ee_vmulas_reg_off64_accx_ledc_channel_xp },
    SxInstEntry { name: "ee.vmulas.u8.accx.LedcChannel.xp", mask: 0xf077000e, opcode: 0xf016000e, func: sx_ee_vmulas_u8_accx_ledc_channel_xp },
    SxInstEntry { name: "ee.vmulas.s16.accx.LedcChannel.xp", mask: 0xf077000e, opcode: 0xf010000e, func: sx_ee_vmulas_s16_accx_ledc_channel_xp },
    SxInstEntry { name: "ee.vmulas.u16.accx.LedcChannel.xp", mask: 0xf077000e, opcode: 0xf014000e, func: sx_ee_vmulas_u16_accx_ledc_channel_xp },
    SxInstEntry { name: "ee.vmulas.regOff64.qacc.LedcChannel.decodeVQFormatXp.qup", mask: 0xf000000e, opcode: 0x3034500e, func: sx_ee_vmulas_reg_off64_qacc_ledc_channel_decode_vq_format_xp_qup },
    SxInstEntry { name: "ee.vmulas.s16.qacc.LedcChannel.decodeVQFormatXp.qup", mask: 0xf000000e, opcode: 0x1034500e, func: sx_ee_vmulas_s16_qacc_ledc_channel_decode_vq_format_xp_qup },
    SxInstEntry { name: "ee.vmulas.u8.qacc.LedcChannel.decodeVQFormatXp.qup", mask: 0xf000000e, opcode: 0x7034500e, func: sx_ee_vmulas_u8_qacc_ledc_channel_decode_vq_format_xp_qup },
    SxInstEntry { name: "ee.vmulas.u16.qacc.LedcChannel.decodeVQFormatXp.qup", mask: 0xf000000e, opcode: 0x5034500e, func: sx_ee_vmulas_u16_qacc_ledc_channel_decode_vq_format_xp_qup },
    SxInstEntry { name: "ee.vmulas.regOff64.accx.LedcChannel.decodeVQFormatXp.qup", mask: 0xf000000e, opcode: 0x2034500e, func: sx_ee_vmulas_reg_off64_accx_ledc_channel_decode_vq_format_xp_qup },
    SxInstEntry { name: "ee.vmulas.s16.accx.LedcChannel.decodeVQFormatXp.qup", mask: 0xf000000e, opcode: 3428366, func: sx_ee_vmulas_s16_accx_ledc_channel_decode_vq_format_xp_qup },
    SxInstEntry { name: "ee.vmulas.u8.accx.LedcChannel.decodeVQFormatXp.qup", mask: 0xf000000e, opcode: 0x6034500e, func: sx_ee_vmulas_u8_accx_ledc_channel_decode_vq_format_xp_qup },
    SxInstEntry { name: "ee.vmulas.u16.accx.LedcChannel.decodeVQFormatXp.qup", mask: 0xf000000e, opcode: 0x4034500e, func: sx_ee_vmulas_u16_accx_ledc_channel_decode_vq_format_xp_qup },
    SxInstEntry { name: "ee.vmulas.regOff64.qacc.LedcChannel.xp.qup", mask: 0xfc00000e, opcode: 0xbc00010e, func: sx_ee_vmulas_reg_off64_qacc_ledc_channel_xp_qup },
    SxInstEntry { name: "ee.vmulas.s16.qacc.LedcChannel.xp.qup", mask: 0xfc00000e, opcode: 0xb400010e, func: sx_ee_vmulas_s16_qacc_ledc_channel_xp_qup },
    SxInstEntry { name: "ee.vmulas.u8.qacc.LedcChannel.xp.qup", mask: 0xfc00000e, opcode: 0xcc00010e, func: sx_ee_vmulas_u8_qacc_ledc_channel_xp_qup },
    SxInstEntry { name: "ee.vmulas.u16.qacc.LedcChannel.xp.qup", mask: 0xfc00000e, opcode: 0xc400010e, func: sx_ee_vmulas_u16_qacc_ledc_channel_xp_qup },
    SxInstEntry { name: "ee.vmulas.regOff64.accx.LedcChannel.xp.qup", mask: 0xfc00000e, opcode: 0xb800010e, func: sx_ee_vmulas_reg_off64_accx_ledc_channel_xp_qup },
    SxInstEntry { name: "ee.vmulas.s16.accx.LedcChannel.xp.qup", mask: 0xfc00000e, opcode: 0xb000010e, func: sx_ee_vmulas_s16_accx_ledc_channel_xp_qup },
    SxInstEntry { name: "ee.vmulas.u8.accx.LedcChannel.xp.qup", mask: 0xfc00000e, opcode: 0xc800010e, func: sx_ee_vmulas_u8_accx_ledc_channel_xp_qup },
    SxInstEntry { name: "ee.vmulas.u16.accx.LedcChannel.xp.qup", mask: 0xfc00000e, opcode: 0xc000010e, func: sx_ee_vmulas_u16_accx_ledc_channel_xp_qup },
    SxInstEntry { name: "ee.vmulas.regOff64.qacc.ldbc.incp.qup", mask: 0xfc000f0e, opcode: 0xe000090e, func: sx_ee_vmulas_reg_off64_qacc_ldbc_incp_qup },
    SxInstEntry { name: "ee.vmulas.s16.qacc.ldbc.incp.qup", mask: 0xfc000f0e, opcode: 0xe000080e, func: sx_ee_vmulas_s16_qacc_ldbc_incp_qup },
    SxInstEntry { name: "ee.vmulas.u8.qacc.ldbc.incp.qup", mask: 0xfc000f0e, opcode: 0xe0000b0e, func: sx_ee_vmulas_u8_qacc_ldbc_incp_qup },
    SxInstEntry { name: "ee.vmulas.u16.qacc.ldbc.incp.qup", mask: 0xfc000f0e, opcode: 0xe0000a0e, func: sx_ee_vmulas_u16_qacc_ldbc_incp_qup },
    SxInstEntry { name: "ee.vsmulas.regOff64.qacc.LedcChannel.incp", mask: 0xfc600f0e, opcode: 0xe0200c0e, func: vsmulas_dispatch },
    SxInstEntry { name: "ee.vsmulas.s16.qacc.LedcChannel.incp", mask: 0xfc700f0e, opcode: 0xe0700c0e, func: vsmulas_dispatch },
    SxInstEntry { name: "ee.vadds.regOff64.LedcChannel.incp", mask: 0xe0700f0e, opcode: 0xe0100c0e, func: sx_ee_vadds_reg_off64_ledc_channel_incp },
    SxInstEntry { name: "ee.vadds.s16.LedcChannel.incp", mask: 0xe0700f0e, opcode: 0xe0200d0e, func: sx_ee_vadds_s16_ledc_channel_incp },
    SxInstEntry { name: "ee.vmax.regOff64.LedcChannel.incp", mask: 0xe0700f0e, opcode: 0xe0100f0e, func: sx_ee_vmax_reg_off64_ledc_channel_incp },
    SxInstEntry { name: "ee.vmax.s16.LedcChannel.incp", mask: 0xe0700f0e, opcode: 0xe0100d0e, func: sx_ee_vmax_s16_ledc_channel_incp },
    SxInstEntry { name: "ee.vadds.s32.LedcChannel.incp", mask: 0xe0700f0e, opcode: 0xe0300d0e, func: sx_ee_vadds_s32_ledc_channel_incp },
    SxInstEntry { name: "ee.vsubs.regOff64.LedcChannel.incp", mask: 0xe0700f0e, opcode: 0xe0600d0e, func: sx_ee_vsubs_reg_off64_ledc_channel_incp },
    SxInstEntry { name: "ee.vsubs.s16.LedcChannel.incp", mask: 0xe0700f0e, opcode: 0xe0400d0e, func: sx_ee_vsubs_s16_ledc_channel_incp },
    SxInstEntry { name: "ee.vsubs.s32.LedcChannel.incp", mask: 0xe0700f0e, opcode: 0xe0500d0e, func: sx_ee_vsubs_s32_ledc_channel_incp },
    SxInstEntry { name: "ee.vmax.s32.LedcChannel.incp", mask: 0xe0700f0e, opcode: 0xe0100e0e, func: sx_ee_vmax_s32_ledc_channel_incp },
    SxInstEntry { name: "ee.vmin.regOff64.LedcChannel.incp", mask: 0xe0700f0e, opcode: 0xe0200f0e, func: sx_ee_vmin_reg_off64_ledc_channel_incp },
    SxInstEntry { name: "ee.vmin.s16.LedcChannel.incp", mask: 0xe0700f0e, opcode: 0xe0200e0e, func: sx_ee_vmin_s16_ledc_channel_incp },
    SxInstEntry { name: "ee.vmin.s32.LedcChannel.incp", mask: 0xe0700f0e, opcode: 0xe0300e0e, func: sx_ee_vmin_s32_ledc_channel_incp },
    SxInstEntry { name: "ee.vmul.regOff64.LedcChannel.incp", mask: 0xe0700f0e, opcode: 0xe0400c0e, func: sx_ee_vmul_reg_off64_ledc_channel_incp },
    SxInstEntry { name: "ee.vmul.s16.LedcChannel.incp", mask: 0xe0700f0e, opcode: 0xe0300f0e, func: sx_ee_vmul_s16_ledc_channel_incp },
    SxInstEntry { name: "ee.vmul.u8.LedcChannel.incp", mask: 0xe0700f0e, opcode: 0xe0600c0e, func: sx_ee_vmul_u8_ledc_channel_incp },
    SxInstEntry { name: "ee.vmul.u16.LedcChannel.incp", mask: 0xe0700f0e, opcode: 0xe0500c0e, func: sx_ee_vmul_u16_ledc_channel_incp },
    SxInstEntry { name: "ee.vadds.regOff64.decodeVecStoreFormat.incp", mask: 0xff08070e, opcode: 0xe408020e, func: sx_ee_vadds_reg_off64_decode_vec_store_format_incp },
    SxInstEntry { name: "ee.vadds.s16.decodeVecStoreFormat.incp", mask: 0xff08070e, opcode: 0xe408000e, func: sx_ee_vadds_s16_decode_vec_store_format_incp },
    SxInstEntry { name: "ee.vadds.s32.decodeVecStoreFormat.incp", mask: 0xff08070e, opcode: 0xe408010e, func: sx_ee_vadds_s32_decode_vec_store_format_incp },
    SxInstEntry { name: "ee.vsubs.regOff64.decodeVecStoreFormat.incp", mask: 0xff08070e, opcode: 0xe808030e, func: sx_ee_vsubs_reg_off64_decode_vec_store_format_incp },
    SxInstEntry { name: "ee.vsubs.s16.decodeVecStoreFormat.incp", mask: 0xff08070e, opcode: 0xe808010e, func: sx_ee_vsubs_s16_decode_vec_store_format_incp },
    SxInstEntry { name: "ee.vsubs.s32.decodeVecStoreFormat.incp", mask: 0xff08070e, opcode: 0xe808020e, func: sx_ee_vsubs_s32_decode_vec_store_format_incp },
    SxInstEntry { name: "ee.vmax.regOff64.decodeVecStoreFormat.incp", mask: 0xff08070e, opcode: 0xe508000e, func: sx_ee_vmax_reg_off64_decode_vec_store_format_incp },
    SxInstEntry { name: "ee.vmax.s16.decodeVecStoreFormat.incp", mask: 0xff08070e, opcode: 0xe408030e, func: sx_ee_vmax_s16_decode_vec_store_format_incp },
    SxInstEntry { name: "ee.vmax.s32.decodeVecStoreFormat.incp", mask: 0xff08070e, opcode: 0xe500000e, func: sx_ee_vmax_s32_decode_vec_store_format_incp },
    SxInstEntry { name: "ee.vmin.regOff64.decodeVecStoreFormat.incp", mask: 0xff08070e, opcode: 0xe500020e, func: sx_ee_vmin_reg_off64_decode_vec_store_format_incp },
    SxInstEntry { name: "ee.vmin.s16.decodeVecStoreFormat.incp", mask: 0xff08070e, opcode: 0xe500010e, func: sx_ee_vmin_s16_decode_vec_store_format_incp },
    SxInstEntry { name: "ee.vmin.s32.decodeVecStoreFormat.incp", mask: 0xff08070e, opcode: 0xe508010e, func: sx_ee_vmin_s32_decode_vec_store_format_incp },
    SxInstEntry { name: "ee.vmul.regOff64.decodeVecStoreFormat.incp", mask: 0xff08070e, opcode: 0xe500030e, func: sx_ee_vmul_reg_off64_decode_vec_store_format_incp },
    SxInstEntry { name: "ee.vmul.s16.decodeVecStoreFormat.incp", mask: 0xff08070e, opcode: 0xe508020e, func: sx_ee_vmul_s16_decode_vec_store_format_incp },
    SxInstEntry { name: "ee.vmul.u8.decodeVecStoreFormat.incp", mask: 0xff08070e, opcode: 0xe808000e, func: sx_ee_vmul_u8_decode_vec_store_format_incp },
    SxInstEntry { name: "ee.vmul.u16.decodeVecStoreFormat.incp", mask: 0xff08070e, opcode: 0xe508030e, func: sx_ee_vmul_u16_decode_vec_store_format_incp },
    SxInstEntry { name: "ee.cmul.s16.LedcChannel.incp", mask: 0xfc700c0e, opcode: 0xe0000c0e, func: sx_ee_cmul_s16_ledc_channel_incp },
    SxInstEntry { name: "ee.cmul.s16.decodeVecStoreFormat.incp", mask: 0xfc000c0e, opcode: 0xe400000e, func: sx_ee_cmul_s16_decode_vec_store_format_incp },
    SxInstEntry { name: "ee.fft.r2bf.s16.decodeVecStoreFormat.incp", mask: 0xff082f0e, opcode: 0xe808040e, func: sx_ee_fft_r2bf_s16_decode_vec_store_format_incp },
    SxInstEntry { name: "ee.fft.cmul.s16.LedcChannel.xp", mask: 0xfc00000e, opcode: 0xdc00000e, func: exec_fft_cmul },
    SxInstEntry { name: "ee.fft.cmul.s16.decodeVecStoreFormat.xp", mask: 0xf800000e, opcode: 0xa800000e, func: exec_fft_cmul },
    SxInstEntry { name: "ee.fft.ams.s16.LedcChannel.incp", mask: 0xfc00000e, opcode: 0xd000000e, func: exec_fft_ams },
    SxInstEntry { name: "ee.fft.ams.s16.LedcChannel.incp.uaup", mask: 0xfc00000e, opcode: 0xd400000e, func: exec_fft_ams },
    SxInstEntry { name: "ee.fft.ams.s16.LedcChannel.r32.decp", mask: 0xfc00000e, opcode: 0xd800000e, func: exec_fft_ams },
    SxInstEntry { name: "ee.fft.ams.s16.decodeVecStoreFormat.incp", mask: 0xf800000e, opcode: 0xa000000e, func: exec_fft_ams },
    SxInstEntry { name: "ee.src.q.LedcChannel.decodeVQFormatXp", mask: 0xfc000c0e, opcode: 0xe000000e, func: sx_ee_src_q_ledc_channel_decode_vq_format_xp },
    SxInstEntry { name: "ee.src.q.LedcChannel.xp", mask: 0xff88300e, opcode: 0xe800000e, func: sx_ee_src_q_ledc_channel_xp },
];
pub fn decode_pie0(core: &mut CoreState, tmp_val: u32) -> u32 {
    for entry in SX_INST_TABLE {
        if (tmp_val & entry.mask) == (entry.opcode & entry.mask) {
            (entry.func)(core, tmp_val);
            return 1;
        }
    }
    0
}

pub fn decode_pie_slot_select(cpu_val: u32) -> u32 {
    ((cpu_val >> 15) & 1) | (((cpu_val >> 20) & 3) << 1)
}

pub fn decode_pie1(cpu_val: u32) -> u32 {
    (cpu_val >> 4) & 15
}

pub fn decode_pie_ar_dest(cpu_val: u32) -> u32 {
    (cpu_val >> 8) & 15
}

pub fn decode_imm_signed(cpu_val: u32, tmp_val: u32) -> i32 {
    let idx_val = (cpu_val >> 8) & 127;
    let val = (((cpu_val >> 22) & 1) << 7) | idx_val;
    (val as i8 as i32) * (tmp_val as i32)
}

pub fn decode_imm_unsigned(cpu_val: u32, tmp_val: u32) -> u32 {
    ((cpu_val >> 8) & 127) * tmp_val
}

pub fn decode_pie2(core: &mut CoreState, tmp_val: u32, idx_val: u32, clock_event: u32, sim_clock: u32) {
    for r in 0..sim_clock {
        core.q_registers[(16 * tmp_val + idx_val + r) as usize] =
            read_uint8(core, clock_event + r) as u8;
    }
}

pub fn decode_pie3(core: &mut CoreState, tmp_val: u32, idx_val: u32, clock_event: u32, sim_clock: u32) {
    for r in 0..sim_clock {
        write_uint8(core, clock_event + r, core.q_registers[(16 * tmp_val + idx_val + r) as usize] as u32, 0);
    }
}

pub fn deposit_vec_reg(core: &mut CoreState, tmp_val: u32, idx_val: u32, clock_event: u32) {
    let sim_clock = 16 / idx_val;
    for r in 0..sim_clock {
        for j in 0..idx_val {
            core.q_registers[(16 * tmp_val + r * idx_val + j) as usize] =
                ((clock_event >> (8 * j)) & 255) as u8;
        }
    }
}

pub fn decode_pie4(cpu_val: u32) -> u32 {
    (cpu_val >> 10) & 3
}

pub fn decode_pie5(core: &mut CoreState, tmp_val: u32) {
    let idx_val = (tmp_val >> 4) & 7;
    let clock_event = (tmp_val >> 12) & 7;
    let sim_clock = decode_pie_slot_select(tmp_val);
    let read_result = 15 & core.sar_byte();
    for r in 0..16 {
        let write_addr = r + read_result;
        core.q_registers[(16 * idx_val + r) as usize] = if write_addr < 16 {
            core.q_registers[(16 * clock_event + write_addr) as usize]
        } else {
            core.q_registers[(16 * sim_clock + (write_addr - 16)) as usize]
        };
    }
}

pub fn decode_pie6(core: &mut CoreState, tmp_val: u32) {
    decode_pie5(core, tmp_val);
}

pub fn decode_pie7(core: &CoreState, tmp_val: u32) -> i128 {
    let qacc = if tmp_val < 4 { &core.qacc_low } else { &core.qacc_high };
    let byte_off = ((tmp_val % 4) * 5) as usize;
    let mut val = (qacc[byte_off] as i128)
        | ((qacc[byte_off + 1] as i128) << 8)
        | ((qacc[byte_off + 2] as i128) << 16)
        | ((qacc[byte_off + 3] as i128) << 24)
        | ((qacc[byte_off + 4] as i128) << 32);
    if val & (1i128 << 39) != 0 {
        val |= !((1i128 << 40) - 1);
    }
    val
}

pub fn decode_pie8(core: &mut CoreState, tmp_val: u32, idx_val: i128) {
    let mut clock_event = idx_val;
    let sim_clock: i128 = (1i128 << 39) - 1;
    let read_result = -(1i128 << 39);
    if clock_event > sim_clock { clock_event = sim_clock; }
    if clock_event < read_result { clock_event = read_result; }
    let write_addr: u128 = (clock_event & ((1i128 << 40) - 1)) as u128;
    let cfg_val = ((tmp_val % 4) * 5) as usize;
    if tmp_val < 4 {
        core.qacc_low[cfg_val] = (write_addr & 255) as u8;
        core.qacc_low[cfg_val + 1] = ((write_addr >> 8) & 255) as u8;
        core.qacc_low[cfg_val + 2] = ((write_addr >> 16) & 255) as u8;
        core.qacc_low[cfg_val + 3] = ((write_addr >> 24) & 255) as u8;
        core.qacc_low[cfg_val + 4] = ((write_addr >> 32) & 255) as u8;
    } else {
        core.qacc_high[cfg_val] = (write_addr & 255) as u8;
        core.qacc_high[cfg_val + 1] = ((write_addr >> 8) & 255) as u8;
        core.qacc_high[cfg_val + 2] = ((write_addr >> 16) & 255) as u8;
        core.qacc_high[cfg_val + 3] = ((write_addr >> 24) & 255) as u8;
        core.qacc_high[cfg_val + 4] = ((write_addr >> 32) & 255) as u8;
    }
}

pub fn decode_pie9(core: &mut CoreState, tmp_val: u32, idx_val: u32) {
    let clock_event = decode_pie1(tmp_val);
    let sim_clock = decode_imm_signed(tmp_val, 16);
    if core.window_check(0, clock_event >> 2, 0) { return; }
    let read_result = (-16i32 as u32) & core.ar(clock_event);
    for r in 0..16 {
        let val = read_uint8(core, read_result + r);
        decode_pie17(core, r, if idx_val != 0 { ((val << 24) as i32) >> 24 } else { val as i32 });
    }
    core.set_ar(clock_event, core.ar(clock_event).wrapping_add(sim_clock as u32));
}

pub fn decode_pie10(core: &mut CoreState, tmp_val: u32, idx_val: u32) {
    let clock_event = decode_pie1(tmp_val);
    let sim_clock = decode_imm_signed(tmp_val, 16);
    if core.window_check(0, clock_event >> 2, 0) { return; }
    let read_result = (-16i32 as u32) & core.ar(clock_event);
    for r in 0..8 {
        let val = read_uint8(core, read_result + 2 * r) as u32
            | ((read_uint8(core, read_result + 2 * r + 1) as u32) << 8);
        decode_pie8(core, r, if idx_val != 0 { (((val << 16) as i32) >> 16) as i128 } else { (65535 & val) as i128 });
    }
    core.set_ar(clock_event, core.ar(clock_event).wrapping_add(sim_clock as u32));
}

pub fn decode_pie_qx_select(cpu_val: u32) -> u32 {
    ((cpu_val >> 4) & 1) | (((cpu_val >> 6) & 1) << 1) | (((cpu_val >> 7) & 1) << 2)
}

pub fn decode_pie11(cpu_val: u32) -> u32 {
    ((cpu_val >> 5) & 1) | (((cpu_val >> 10) & 1) << 1) | (((cpu_val >> 11) & 1) << 2)
}

pub struct DecodePie12 {
    pub qu: u32,
    pub qx: u32,
    pub qy: u32,
}

pub fn decode_pie12(cpu_val: u32) -> DecodePie12 {
    DecodePie12 {
        qu: decode_pie_slot_select(cpu_val),
        qx: decode_pie_qx_select(cpu_val),
        qy: decode_pie11(cpu_val),
    }
}

pub fn decode_pie13(cpu_val: u32) -> u32 {
    (cpu_val >> 8) & 7
}

pub fn decode_pie14(cpu_val: u32) -> u32 {
    ((cpu_val >> 11) & 1) | (((cpu_val >> 12) & 1) << 1) | (((cpu_val >> 14) & 1) << 2)
}

pub struct DecodePie15 {
    pub qu: u32,
    pub qx: u32,
    pub qy: u32,
}

pub fn decode_pie15(cpu_val: u32) -> DecodePie15 {
    DecodePie15 {
        qu: decode_pie_slot_select(cpu_val),
        qx: decode_pie13(cpu_val),
        qy: decode_pie14(cpu_val),
    }
}

pub fn decode_pie16(core: &CoreState, tmp_val: u32) -> i32 {
    read_qacc_signed(core, tmp_val)
}
pub fn decode_pie17(core: &mut CoreState, tmp_val: u32, idx_val: i32) {
    let mut clock_event = idx_val;
    if clock_event > 524287 { clock_event = 524287; }
    else if clock_event < -524288 { clock_event = -524288; }
    let clock_event = (clock_event as u32) & 1048575;
    let sim_clock = tmp_val % 8;
    let read_result = (5 * (sim_clock >> 1)) as usize;
    if (1 & sim_clock) == 0 {
        if tmp_val < 8 {
            core.qacc_low[read_result] = (clock_event & 255) as u8;
            core.qacc_low[read_result + 1] = ((clock_event >> 8) & 255) as u8;
            core.qacc_low[read_result + 2] = (core.qacc_low[read_result + 2] & 240) | ((clock_event >> 16) as u8 & 15);
        } else {
            core.qacc_high[read_result] = (clock_event & 255) as u8;
            core.qacc_high[read_result + 1] = ((clock_event >> 8) & 255) as u8;
            core.qacc_high[read_result + 2] = (core.qacc_high[read_result + 2] & 240) | ((clock_event >> 16) as u8 & 15);
        }
    } else {
        if tmp_val < 8 {
            core.qacc_low[read_result + 2] = (core.qacc_low[read_result + 2] & 15) | (((clock_event & 15) as u8) << 4);
            core.qacc_low[read_result + 3] = ((clock_event >> 4) & 255) as u8;
            core.qacc_low[read_result + 4] = ((clock_event >> 12) & 255) as u8;
        } else {
            core.qacc_high[read_result + 2] = (core.qacc_high[read_result + 2] & 15) | (((clock_event & 15) as u8) << 4);
            core.qacc_high[read_result + 3] = ((clock_event >> 4) & 255) as u8;
            core.qacc_high[read_result + 4] = ((clock_event >> 12) & 255) as u8;
        }
    }
}

pub struct DecodePie18 {
    pub qu: u32,
    pub qx: u32,
}

pub fn decode_pie18(cpu_val: u32) -> DecodePie18 {
    DecodePie18 {
        qu: (cpu_val >> 4) & 7,
        qx: ((cpu_val >> 15) & 1) | (((cpu_val >> 20) & 1) << 1) | (((cpu_val >> 21) & 1) << 2),
    }
}

pub struct DecodePie19 {
    pub qu: u32,
    pub qx: u32,
}

pub fn decode_pie19(cpu_val: u32) -> DecodePie19 {
    DecodePie19 {
        qu: ((cpu_val >> 12) & 1) | (((cpu_val >> 13) & 1) << 1) | (((cpu_val >> 14) & 1) << 2),
        qx: ((cpu_val >> 15) & 1) | (((cpu_val >> 20) & 1) << 1) | (((cpu_val >> 21) & 1) << 2),
    }
}

pub fn decode_pie20(core: &mut CoreState, tmp_val: u32, idx_val: u32) {
    let clock_event = decode_pie1(tmp_val);
    let sim_clock = decode_pie_ar_dest(tmp_val);
    if core.window_check(0, clock_event >> 2, 0) { return; }
    let read_result = (-16i32 as u32) & core.ar(clock_event);
    for r in 0..16 {
        let val = read_uint8(core, read_result + r);
        decode_pie17(core, r, if idx_val != 0 { ((val << 24) as i32) >> 24 } else { val as i32 });
    }
    core.set_ar(clock_event, core.ar(clock_event).wrapping_add(core.ar(sim_clock)));
}

pub fn decode_pie21(core: &mut CoreState, tmp_val: u32, idx_val: u32) {
    let clock_event = decode_pie1(tmp_val);
    let sim_clock = decode_pie_ar_dest(tmp_val);
    if core.window_check(0, clock_event >> 2, 0) { return; }
    let read_result = (-16i32 as u32) & core.ar(clock_event);
    for r in 0..8 {
        let val = read_uint8(core, read_result + 2 * r) as u32
            | ((read_uint8(core, read_result + 2 * r + 1) as u32) << 8);
        decode_pie8(core, r, if idx_val != 0 { (((val << 16) as i32) >> 16) as i128 } else { (65535 & val) as i128 });
    }
    core.set_ar(clock_event, core.ar(clock_event).wrapping_add(core.ar(sim_clock)));
}

pub struct DecodePie22 {
    pub qu: u32,
    pub qx: u32,
    pub a_handler15: u32,
    pub a_handler3: u32,
}

pub fn decode_pie22(cpu_val: u32) -> DecodePie22 {
    let tmp_val = decode_pie_slot_select(cpu_val);
    let idx_val = (cpu_val >> 12) & 7;
    DecodePie22 {
        qu: tmp_val,
        qx: idx_val,
        a_handler15: decode_pie_ar_dest(cpu_val),
        a_handler3: decode_pie1(cpu_val),
    }
}

pub fn decode_pie23(cpu_val: u32, tmp_val: u32) -> u32 {
    let mut idx_val = 0;
    for clock_event in 0..tmp_val {
        if (cpu_val & (1 << clock_event)) != 0 {
            idx_val |= 1 << (tmp_val - clock_event - 1);
        }
    }
    idx_val
}

pub fn decode_pie24(_core: &mut CoreState, _tmp_val: u32) {}

pub struct DecodePie25 {
    pub qu: u32,
    pub a_handler3: u32,
    pub qx: u32,
    pub qy: u32,
}

pub fn decode_pie25(cpu_val: u32) -> DecodePie25 {
    let tmp_val = ((cpu_val >> 13) & 1) | (((cpu_val >> 15) & 1) << 1) | (((cpu_val >> 20) & 1) << 2);
    let idx_val = decode_pie1(cpu_val);
    DecodePie25 {
        qu: tmp_val,
        a_handler3: idx_val,
        qx: (cpu_val >> 8) & 7,
        qy: ((cpu_val >> 11) & 1) | (((cpu_val >> 12) & 1) << 1) | (((cpu_val >> 14) & 1) << 2),
    }
}

pub fn decode_pie26(core: &mut CoreState, tmp_val: u32, idx_val: u32) {
    for clock_event in 0..16 {
        let sim_clock = ((core.q_registers[(16 * tmp_val + clock_event) as usize] as i8) as i32);
        let read_result = ((core.q_registers[(16 * idx_val + clock_event) as usize] as i8) as i32);
        let mut write_addr = decode_pie16(core, clock_event) + sim_clock * read_result;
        if write_addr > 524287 { write_addr = 524287; }
        if write_addr < -524287 { write_addr = -524287; }
        decode_pie17(core, clock_event, write_addr);
    }
}

pub fn decode_pie27(core: &mut CoreState, tmp_val: u32, idx_val: u32) {
    for clock_event in 0..16 {
        let sim_clock = 255 & core.q_registers[(16 * tmp_val + clock_event) as usize] as u32;
        let read_result = 255 & core.q_registers[(16 * idx_val + clock_event) as usize] as u32;
        let mut write_addr = decode_pie16(core, clock_event);
        if write_addr < 0 { write_addr += 1048576; }
        let mut register_type = write_addr + (sim_clock * read_result) as i32;
        if register_type > 1048575 { register_type = 1048575; }
        if register_type < 0 { register_type = 0; }
        decode_pie17(core, clock_event, if register_type >= 524288 { register_type - 1048576 } else { register_type });
    }
}

pub fn decode_pie28(core: &mut CoreState, tmp_val: u32, idx_val: u32) {
    let clock_event: i128 = 549755813887;
    for sim_clock in 0..8 {
        let read_result = core.q_reg_int16((8 * tmp_val + sim_clock) as usize) as i32;
        let write_addr = core.q_reg_int16((8 * idx_val + sim_clock) as usize) as i32;
        let mut register_type = decode_pie7(core, sim_clock) + (read_result.wrapping_mul(write_addr) as i128);
        if register_type > clock_event { register_type = clock_event; }
        if register_type < -clock_event { register_type = -clock_event; }
        decode_pie8(core, sim_clock, register_type);
    }
}

pub fn decode_pie29(core: &mut CoreState, tmp_val: u32, idx_val: u32) {
    let clock_event: i128 = 1099511627775;
    for sim_clock in 0..8 {
        let read_result = core.q_reg_uint16((8 * tmp_val + sim_clock) as usize) as i128;
        let write_addr = core.q_reg_uint16((8 * idx_val + sim_clock) as usize) as i128;
        let mut register_type = decode_pie7(core, sim_clock);
        if register_type < 0 { register_type += 1i128 << 40; }
        let mut cfg_val = register_type + read_result * write_addr;
        if cfg_val > clock_event { cfg_val = clock_event; }
        if cfg_val >= 549755813888 { cfg_val -= 1i128 << 40; }
        decode_pie8(core, sim_clock, cfg_val);
    }
}

pub struct DecodePie30 {
    pub qu: u32,
    pub qx: u32,
    pub qy: u32,
    pub a_handler3: u32,
}

pub fn decode_pie30(cpu_val: u32) -> DecodePie30 {
    let tmp_val = decode_pie_slot_select(cpu_val);
    let idx_val = (cpu_val >> 8) & 7;
    DecodePie30 {
        qu: tmp_val,
        qx: idx_val,
        qy: ((cpu_val >> 11) & 1) | (((cpu_val >> 12) & 1) << 1) | (((cpu_val >> 14) & 1) << 2),
        a_handler3: decode_pie1(cpu_val),
    }
}

pub struct S2InstEntry {
    pub name: &'static str,
    pub mask: u32,
    pub opcode: u32,
    pub func: fn(&mut CoreState, u32),
}

fn s2_ee_vld_128_decode_vq_format_xp(core: &mut CoreState, tmp_val: u32) {
    let idx_val = decode_pie_slot_select(tmp_val);
    let clock_event = decode_pie1(tmp_val);
    let simulation_clock = decode_imm_signed(tmp_val, 16);
    if !core.window_check(0, clock_event >> 2, 0) {
        decode_pie2(core, idx_val, 0, 0xfffffff0 & core.ar(clock_event), 16);
        core.set_ar(clock_event, core.ar(clock_event).wrapping_add_signed(simulation_clock));
    }
}

fn s2_ee_vld_128_xp(core: &mut CoreState, tmp_val: u32) {
    let idx_val = decode_pie_slot_select(tmp_val);
    let clock_event = decode_pie1(tmp_val);
    let simulation_clock = decode_pie_ar_dest(tmp_val);
    if !core.window_check(0, clock_event >> 2, 0) {
        decode_pie2(core, idx_val, 0, 0xfffffff0 & core.ar(clock_event), 16);
        core.set_ar(clock_event, core.ar(clock_event).wrapping_add(core.ar(simulation_clock)));
    }
}

fn s2_ee_vld_peripheral_type_64_decode_vq_format_xp(core: &mut CoreState, tmp_val: u32) {
    let idx_val = decode_pie_slot_select(tmp_val);
    let clock_event = decode_pie1(tmp_val);
    let simulation_clock = decode_imm_signed(tmp_val, 8);
    if !core.window_check(0, clock_event >> 2, 0) {
        decode_pie2(core, idx_val, 8, 0xfffffff8 & core.ar(clock_event), 8);
        core.set_ar(clock_event, core.ar(clock_event).wrapping_add_signed(simulation_clock));
    }
}

fn s2_ee_vld_signal_direction_64_decode_vq_format_xp(core: &mut CoreState, tmp_val: u32) {
    let idx_val = decode_pie_slot_select(tmp_val);
    let clock_event = decode_pie1(tmp_val);
    let simulation_clock = decode_imm_signed(tmp_val, 8);
    if !core.window_check(0, clock_event >> 2, 0) {
        decode_pie2(core, idx_val, 0, 0xfffffff8 & core.ar(clock_event), 8);
        core.set_ar(clock_event, core.ar(clock_event).wrapping_add_signed(simulation_clock));
    }
}

fn s2_ee_vld_signal_direction_64_xp(core: &mut CoreState, tmp_val: u32) {
    let idx_val = decode_pie_slot_select(tmp_val);
    let clock_event = decode_pie1(tmp_val);
    let simulation_clock = decode_pie_ar_dest(tmp_val);
    if !core.window_check(0, clock_event >> 2, 0) {
        decode_pie2(core, idx_val, 0, 0xfffffff8 & core.ar(clock_event), 8);
        core.set_ar(clock_event, core.ar(clock_event).wrapping_add(core.ar(simulation_clock)));
    }
}

fn s2_ee_vld_peripheral_type_64_xp(core: &mut CoreState, tmp_val: u32) {
    let idx_val = decode_pie_slot_select(tmp_val);
    let clock_event = decode_pie1(tmp_val);
    let simulation_clock = decode_pie_ar_dest(tmp_val);
    if !core.window_check(0, clock_event >> 2, 0) {
        decode_pie2(core, idx_val, 8, 0xfffffff8 & core.ar(clock_event), 8);
        core.set_ar(clock_event, core.ar(clock_event).wrapping_add(core.ar(simulation_clock)));
    }
}

fn s2_ee_vldbc_8_decode_vq_format_xp(core: &mut CoreState, tmp_val: u32) {
    let idx_val = decode_pie_slot_select(tmp_val);
    let clock_event = decode_pie1(tmp_val);
    let simulation_clock = decode_imm_unsigned(tmp_val, 1);
    if !core.window_check(0, clock_event >> 2, 0) {
        let __tmp = read_uint8(core, core.ar(clock_event)); deposit_vec_reg(core, idx_val, 1, __tmp);
        core.set_ar(clock_event, core.ar(clock_event).wrapping_add(simulation_clock));
    }
}

fn s2_ee_vldbc_16_decode_vq_format_xp(core: &mut CoreState, tmp_val: u32) {
    let idx_val = decode_pie_slot_select(tmp_val);
    let clock_event = decode_pie1(tmp_val);
    let simulation_clock = decode_imm_unsigned(tmp_val, 2);
    if core.window_check(0, clock_event >> 2, 0) { return; }
    let read_result = 0xfffffffe & core.ar(clock_event);
    let __val = read_uint8(core, read_result) | (read_uint8(core, read_result + 1) << 8);
    deposit_vec_reg(core, idx_val, 2, __val);
    core.set_ar(clock_event, core.ar(clock_event).wrapping_add(simulation_clock));
}

fn s2_ee_vldbc_32_decode_vq_format_xp(core: &mut CoreState, tmp_val: u32) {
    let idx_val = decode_pie_slot_select(tmp_val);
    let clock_event = decode_pie1(tmp_val);
    let simulation_clock = decode_imm_signed(tmp_val, 4);
    if !core.window_check(0, clock_event >> 2, 0) {
        let __tmp = read_uint32(core, 0xfffffffc & core.ar(clock_event));
        deposit_vec_reg(core, idx_val, 4, __tmp);
        core.set_ar(clock_event, core.ar(clock_event).wrapping_add_signed(simulation_clock));
    }
}

fn s2_ee_vldbc_8_xp(core: &mut CoreState, tmp_val: u32) {
    let idx_val = decode_pie_slot_select(tmp_val);
    let clock_event = decode_pie1(tmp_val);
    let simulation_clock = decode_pie_ar_dest(tmp_val);
    if !core.window_check(0, clock_event >> 2, 0) {
        let __tmp = read_uint8(core, core.ar(clock_event));
        deposit_vec_reg(core, idx_val, 1, __tmp);
        core.set_ar(clock_event, core.ar(clock_event).wrapping_add(core.ar(simulation_clock)));
    }
}

fn s2_ee_vldbc_16_xp(core: &mut CoreState, tmp_val: u32) {
    let idx_val = decode_pie_slot_select(tmp_val);
    let clock_event = decode_pie1(tmp_val);
    let simulation_clock = decode_pie_ar_dest(tmp_val);
    if core.window_check(0, clock_event >> 2, 0) { return; }
    let read_result = 0xfffffffe & core.ar(clock_event);
    let __val = read_uint8(core, read_result) | (read_uint8(core, read_result + 1) << 8);
    deposit_vec_reg(core, idx_val, 2, __val);
    core.set_ar(clock_event, core.ar(clock_event).wrapping_add(core.ar(simulation_clock)));
}

fn s2_ee_vldbc_32_xp(core: &mut CoreState, tmp_val: u32) {
    let idx_val = decode_pie_slot_select(tmp_val);
    let clock_event = decode_pie1(tmp_val);
    let simulation_clock = decode_pie_ar_dest(tmp_val);
    if !core.window_check(0, clock_event >> 2, 0) {
        let __tmp = read_uint32(core, 0xfffffffc & core.ar(clock_event));
        deposit_vec_reg(core, idx_val, 4, __tmp);
        core.set_ar(clock_event, core.ar(clock_event).wrapping_add(core.ar(simulation_clock)));
    }
}

fn s2_ee_vldhbc_16_incp(core: &mut CoreState, tmp_val: u32) {
    let idx_val = decode_pie_slot_select(tmp_val);
    let clock_event = ((tmp_val >> 12) & 7) as usize;
    let simulation_clock = decode_pie1(tmp_val);
    if core.window_check(0, simulation_clock >> 2, 0) { return; }
    let read_result = 0xfffffff0 & core.ar(simulation_clock);
    for t in 0..4 {
        let sim_val =
            read_uint8(core, read_result + 2 * t)
                | (read_uint8(core, read_result + 2 * t + 1) << 8);
        core.set_q_reg_uint16(8 * idx_val as usize + 2 * t as usize, sim_val as u16);
        core.set_q_reg_uint16(8 * idx_val as usize + 2 * t as usize + 1, sim_val as u16);
        let write_addr =
            read_uint8(core, read_result + 8 + 2 * t)
                | (read_uint8(core, read_result + 8 + 2 * t + 1) << 8);
        core.set_q_reg_uint16(8 * clock_event + 2 * t as usize, write_addr as u16);
        core.set_q_reg_uint16(8 * clock_event + 2 * t as usize + 1, write_addr as u16);
    }
    core.set_ar(simulation_clock, core.ar(simulation_clock) + 16);
}

fn s2_ee_vst_128_decode_vq_format_xp(core: &mut CoreState, tmp_val: u32) {
    let idx_val = decode_pie_slot_select(tmp_val);
    let clock_event = decode_pie1(tmp_val);
    let simulation_clock = decode_imm_signed(tmp_val, 16);
    if !core.window_check(0, clock_event >> 2, 0) {
        decode_pie3(core, idx_val, 0, 0xfffffff0 & core.ar(clock_event), 16);
        core.set_ar(clock_event, core.ar(clock_event).wrapping_add_signed(simulation_clock));
    }
}

fn s2_ee_vst_128_xp(core: &mut CoreState, tmp_val: u32) {
    let idx_val = decode_pie_slot_select(tmp_val);
    let clock_event = decode_pie1(tmp_val);
    let simulation_clock = decode_pie_ar_dest(tmp_val);
    if !core.window_check(0, clock_event >> 2, 0) {
        decode_pie3(core, idx_val, 0, 0xfffffff0 & core.ar(clock_event), 16);
        core.set_ar(clock_event, core.ar(clock_event).wrapping_add(core.ar(simulation_clock)));
    }
}

fn s2_ee_vst_peripheral_type_64_decode_vq_format_xp(core: &mut CoreState, tmp_val: u32) {
    let idx_val = decode_pie_slot_select(tmp_val);
    let clock_event = decode_pie1(tmp_val);
    let simulation_clock = decode_imm_signed(tmp_val, 8);
    if !core.window_check(0, clock_event >> 2, 0) {
        decode_pie3(core, idx_val, 8, 0xfffffff8 & core.ar(clock_event), 8);
        core.set_ar(clock_event, core.ar(clock_event).wrapping_add_signed(simulation_clock));
    }
}

fn s2_ee_vst_signal_direction_64_decode_vq_format_xp(core: &mut CoreState, tmp_val: u32) {
    let idx_val = decode_pie_slot_select(tmp_val);
    let clock_event = decode_pie1(tmp_val);
    let simulation_clock = decode_imm_signed(tmp_val, 8);
    if !core.window_check(0, clock_event >> 2, 0) {
        decode_pie3(core, idx_val, 0, 0xfffffff8 & core.ar(clock_event), 8);
        core.set_ar(clock_event, core.ar(clock_event).wrapping_add_signed(simulation_clock));
    }
}

fn s2_ee_vst_peripheral_type_64_xp(core: &mut CoreState, tmp_val: u32) {
    let idx_val = decode_pie_slot_select(tmp_val);
    let clock_event = decode_pie1(tmp_val);
    let simulation_clock = decode_pie_ar_dest(tmp_val);
    if !core.window_check(0, clock_event >> 2, 0) {
        decode_pie3(core, idx_val, 8, 0xfffffff8 & core.ar(clock_event), 8);
        core.set_ar(clock_event, core.ar(clock_event).wrapping_add(core.ar(simulation_clock)));
    }
}

fn s2_ee_vst_signal_direction_64_xp(core: &mut CoreState, tmp_val: u32) {
    let idx_val = decode_pie_slot_select(tmp_val);
    let clock_event = decode_pie1(tmp_val);
    let simulation_clock = decode_pie_ar_dest(tmp_val);
    if !core.window_check(0, clock_event >> 2, 0) {
        decode_pie3(core, idx_val, 0, 0xfffffff8 & core.ar(clock_event), 8);
        core.set_ar(clock_event, core.ar(clock_event).wrapping_add(core.ar(simulation_clock)));
    }
}

fn s2_ee_movi_32_q(core: &mut CoreState, tmp_val: u32) {
    let idx_val = decode_pie_slot_select(tmp_val);
    let clock_event = decode_pie1(tmp_val);
    let simulation_clock = decode_pie4(tmp_val);
    if core.window_check(0, clock_event >> 2, 0) { return; }
    let read_result = core.ar(clock_event);
    for t in 0..4 {
        core.q_registers[16 * idx_val as usize + 4 * simulation_clock as usize + t as usize] = ((read_result >> (8 * t)) & 255) as u8;
    }
}

fn s2_ee_movi_32_trap_cause_map(core: &mut CoreState, tmp_val: u32) {
    let idx_val = decode_pie_slot_select(tmp_val);
    let clock_event = decode_pie1(tmp_val);
    let simulation_clock = decode_pie4(tmp_val);
    if core.window_check(0, clock_event >> 2, 0) { return; }
    let mut read_result = 0u32;
    for t in 0..4 {
        read_result |= (core.q_registers[16 * idx_val as usize + 4 * simulation_clock as usize + t as usize] as u32) << (8 * t);
    }
    core.set_ar(clock_event, read_result);
}

fn s2_ee_zero_q(core: &mut CoreState, tmp_val: u32) {
    let idx_val = decode_pie_slot_select(tmp_val) as usize;
    core.q_registers[16 * idx_val..16 * idx_val + 16].fill(0);
}

fn s2_ee_zero_qacc(core: &mut CoreState, tmp_val: u32) {
    core.qacc_high.fill(0);
    core.qacc_low.fill(0);
}

fn s2_ee_zero_accx(core: &mut CoreState, tmp_val: u32) {
    core.set_accx(0);
}

fn s2_ee_ledc_channel_128_usar_decode_vq_format_xp(core: &mut CoreState, tmp_val: u32) {
    let idx_val = decode_pie_slot_select(tmp_val);
    let clock_event = decode_pie1(tmp_val);
    let simulation_clock = decode_imm_signed(tmp_val, 16);
    if core.window_check(0, clock_event >> 2, 0) { return; }
    let read_result = core.ar(clock_event);
    decode_pie2(core, idx_val, 0, 0xfffffff0 & read_result, 16);
    core.set_sar_byte(15 & read_result);
    core.set_ar(clock_event, read_result.wrapping_add_signed(simulation_clock));
}

fn s2_ee_ledc_channel_128_usar_xp(core: &mut CoreState, tmp_val: u32) {
    let idx_val = decode_pie_slot_select(tmp_val);
    let clock_event = decode_pie1(tmp_val);
    let simulation_clock = decode_pie_ar_dest(tmp_val);
    if core.window_check(0, clock_event >> 2, 0) { return; }
    let read_result = core.ar(clock_event);
    decode_pie2(core, idx_val, 0, 0xfffffff0 & read_result, 16);
    core.set_sar_byte(15 & read_result);
    core.set_ar(clock_event, read_result.wrapping_add(core.ar(simulation_clock)));
}

fn s2_ee_srcmb_reg_off64_qacc(core: &mut CoreState, tmp_val: u32) {
    let idx_val = decode_pie_slot_select(tmp_val);
    let clock_event = decode_pie1(tmp_val);
    if core.window_check(0, clock_event >> 2, 0) { return; }
    let simulation_clock = 63 & core.ar(clock_event);
    for t in 0..8 {
        let write_addr = decode_pie16(core, t) >> simulation_clock;
        let register_type = decode_pie16(core, t + 8) >> simulation_clock;
        decode_pie17(core, t, write_addr);
        decode_pie17(core, t + 8, register_type);
        let wa = write_addr as i32;
        let rt = register_type as i32;
        let ce = if wa > 127 { 127 } else if wa < -127 { 128 } else { (255 & write_addr) as u32 };
        let rr = if rt > 127 { 127 } else if rt < -127 { 128 } else { (255 & register_type) as u32 };
        core.q_registers[16 * idx_val as usize + t as usize] = ce as u8;
        core.q_registers[16 * idx_val as usize + t as usize + 8] = rr as u8;
    }
}

fn s2_ee_srcmb_s16_qacc(core: &mut CoreState, tmp_val: u32) {
    let idx_val = decode_pie_slot_select(tmp_val);
    let clock_event = decode_pie1(tmp_val);
    if core.window_check(0, clock_event >> 2, 0) { return; }
    let simulation_clock = (63 & core.ar(clock_event)) as i128;
    for t in 0..4 {
        let write_addr = decode_pie7(core, t) >> simulation_clock;
        let register_type = decode_pie7(core, t + 4) >> simulation_clock;
        decode_pie8(core, t, write_addr);
        decode_pie8(core, t + 4, register_type);
        let ce = if write_addr > 32767 { 32767 } else if write_addr < -32767 { -32768 } else { write_addr as i32 };
        let rr = if register_type > 32767 { 32767 } else if register_type < -32767 { -32768 } else { register_type as i32 };
        core.set_q_reg_int16(8 * idx_val as usize + t as usize, ce as i16);
        core.set_q_reg_int16(8 * idx_val as usize + t as usize + 4, rr as i16);
    }
}

fn s2_ee_mov_reg_off64_qacc(core: &mut CoreState, tmp_val: u32) {
    let idx_val = decode_pie_slot_select(tmp_val) as usize;
    for t in 0..16 {
        let clock_event = core.q_registers[16 * idx_val + t];
        decode_pie17(core, t as u32, (clock_event as i8) as i32);
    }
}

fn s2_ee_mov_s16_qacc(core: &mut CoreState, tmp_val: u32) {
    let idx_val = decode_pie_slot_select(tmp_val) as usize;
    for t in 0..8 {
        let clock_event = core.q_reg_int16(8 * idx_val + t);
        decode_pie8(core, t as u32, clock_event as i128);
    }
}

fn s2_ee_mov_u8_qacc(core: &mut CoreState, tmp_val: u32) {
    let idx_val = decode_pie_slot_select(tmp_val) as usize;
    for t in 0..16 {
        decode_pie17(core, t as u32, (255 & core.q_registers[16 * idx_val + t as usize]) as i32);
    }
}

fn s2_ee_mov_u16_qacc(core: &mut CoreState, tmp_val: u32) {
    let idx_val = decode_pie_slot_select(tmp_val) as usize;
    for t in 0..8 {
        decode_pie8(core, t as u32, core.q_reg_uint16(8 * idx_val + t as usize) as i128);
    }
}

fn s2_ee_ledc_channel_accx_decode_vq_format_xp(core: &mut CoreState, tmp_val: u32) {
    let idx_val = decode_pie1(tmp_val);
    let clock_event = decode_imm_signed(tmp_val, 8);
    if core.window_check(0, idx_val >> 2, 0) { return; }
    let simulation_clock = core.ar(idx_val);
    let mut read_result = 0i128;
    for t in (0..=4).rev() {
        read_result = (read_result << 8) | read_uint8(core, simulation_clock + t) as i128;
    }
    if read_result & (1i128 << 39) != 0 {
        read_result |= !((1i128 << 40) - 1);
    }
    core.set_accx(read_result);
    core.set_ar(idx_val, simulation_clock.wrapping_add_signed(clock_event));
}

fn s2_ee_decode_vec_store_format_accx_decode_vq_format_xp(core: &mut CoreState, tmp_val: u32) {
    let idx_val = decode_pie1(tmp_val);
    let clock_event = decode_imm_signed(tmp_val, 8);
    if core.window_check(0, idx_val >> 2, 0) { return; }
    let simulation_clock = core.ar(idx_val);
    let mut read_result = core.get_accx() & ((1i128 << 40) - 1);
    for t in 0..5 {
        write_uint8(core, simulation_clock + t as u32, (255 & read_result as u32) as u32, 0);
        read_result >>= 8;
    }
    write_uint8(core, simulation_clock + 5, 0, 0);
    write_uint8(core, simulation_clock + 6, 0, 0);
    write_uint8(core, simulation_clock + 7, 0, 0);
    core.set_ar(idx_val, simulation_clock.wrapping_add_signed(clock_event));
}

fn s2_ee_srs_accx(core: &mut CoreState, tmp_val: u32) {
    let idx_val = (tmp_val >> 8) & 15;
    let clock_event = (tmp_val >> 4) & 15;
    if core.window_check(0, idx_val >> 2, 0) { return; }
    let simulation_clock = (63 & core.ar(clock_event)) as u32;
    let shifted = core.get_accx() >> simulation_clock;
    core.set_accx(shifted);
    let read_result = (shifted as u64) as u32;
    core.set_ar(idx_val, read_result);
}

fn s2_ee_ldqa_reg_off64_128_decode_vq_format_xp(core: &mut CoreState, tmp_val: u32) {
    decode_pie9(core, tmp_val, 1);
}

fn s2_ee_ldqa_s16_128_decode_vq_format_xp(core: &mut CoreState, tmp_val: u32) {
    decode_pie10(core, tmp_val, 1);
}

fn s2_ee_ldqa_u8_128_decode_vq_format_xp(core: &mut CoreState, tmp_val: u32) {
    decode_pie9(core, tmp_val, 0);
}

fn s2_ee_ldqa_u16_128_decode_vq_format_xp(core: &mut CoreState, tmp_val: u32) {
    decode_pie10(core, tmp_val, 0);
}

fn s2_ee_andq(core: &mut CoreState, tmp_val: u32) {
    let r = decode_pie12(tmp_val);
    for t in 0..16 {
        core.q_registers[16 * r.qu as usize + t] = core.q_registers[16 * r.qx as usize + t] & core.q_registers[16 * r.qy as usize + t];
    }
}

fn s2_ee_orq(core: &mut CoreState, tmp_val: u32) {
    let r = decode_pie12(tmp_val);
    for t in 0..16 {
        core.q_registers[16 * r.qu as usize + t] = core.q_registers[16 * r.qx as usize + t] | core.q_registers[16 * r.qy as usize + t];
    }
}

fn s2_ee_xorq(core: &mut CoreState, tmp_val: u32) {
    let r = decode_pie12(tmp_val);
    for t in 0..16 {
        core.q_registers[16 * r.qu as usize + t] = core.q_registers[16 * r.qx as usize + t] ^ core.q_registers[16 * r.qy as usize + t];
    }
}

fn s2_ee_notq(core: &mut CoreState, tmp_val: u32) {
    let idx_val = ((tmp_val >> 15) & 1) | (((tmp_val >> 20) & 1) << 1) | (((tmp_val >> 21) & 1) << 2);
    let clock_event = ((tmp_val >> 4) & 1) | (((tmp_val >> 6) & 1) << 1) | (((tmp_val >> 7) & 1) << 2);
    for t in 0..16 {
        core.q_registers[16 * idx_val as usize + t] = 255 & !core.q_registers[16 * clock_event as usize + t];
    }
}

fn s2_ee_vsl_32(core: &mut CoreState, tmp_val: u32) {
    let r = decode_pie18(tmp_val);
    let simulation_clock = if core.sar_m32_pending != 0 { core.sar_m32 as u32 } else { 31 & core.special_registers[12] };
    for t in 0..4 {
        core.set_q_reg_uint32(4 * r.qu as usize + t, core.q_reg_uint32(4 * r.qx as usize + t) << simulation_clock);
    }
}

fn s2_ee_vsr_32(core: &mut CoreState, tmp_val: u32) {
    let r = decode_pie18(tmp_val);
    let simulation_clock = if core.sar_m32_pending != 0 { core.sar_m32 as u32 } else { 31 & core.special_registers[12] };
    for t in 0..4 {
        core.set_q_reg_int32(4 * r.qu as usize + t, core.q_reg_int32(4 * r.qx as usize + t) >> simulation_clock);
    }
}

fn s2_ee_vzip_8(core: &mut CoreState, tmp_val: u32) {
    let r = decode_pie19(tmp_val);
    let mut simulation_clock = [0u8; 16];
    let mut read_result = [0u8; 16];
    for t in 0..8 {
        simulation_clock[2 * t] = core.q_registers[16 * r.qu as usize + t];
        simulation_clock[2 * t + 1] = core.q_registers[16 * r.qx as usize + t];
        read_result[2 * t] = core.q_registers[16 * r.qu as usize + t + 8];
        read_result[2 * t + 1] = core.q_registers[16 * r.qx as usize + t + 8];
    }
    core.q_registers[16 * r.qu as usize..16 * r.qu as usize + 16].copy_from_slice(&simulation_clock);
    core.q_registers[16 * r.qx as usize..16 * r.qx as usize + 16].copy_from_slice(&read_result);
}

fn s2_ee_vunzip_8(core: &mut CoreState, tmp_val: u32) {
    let r = decode_pie19(tmp_val);
    let mut simulation_clock = [0u8; 16];
    let mut read_result = [0u8; 16];
    for t in 0..8 {
        simulation_clock[t] = core.q_registers[16 * r.qu as usize + 2 * t];
        simulation_clock[t + 8] = core.q_registers[16 * r.qx as usize + 2 * t];
        read_result[t] = core.q_registers[16 * r.qu as usize + 2 * t + 1];
        read_result[t + 8] = core.q_registers[16 * r.qx as usize + 2 * t + 1];
    }
    core.q_registers[16 * r.qu as usize..16 * r.qu as usize + 16].copy_from_slice(&simulation_clock);
    core.q_registers[16 * r.qx as usize..16 * r.qx as usize + 16].copy_from_slice(&read_result);
}

fn s2_ee_vmax_reg_off64(core: &mut CoreState, tmp_val: u32) {
    let r = decode_pie15(tmp_val);
    for t in 0..16 {
        let read_result = (core.q_registers[16 * r.qx as usize + t] as i8) as i32;
        let write_addr = (core.q_registers[16 * r.qy as usize + t] as i8) as i32;
        core.q_registers[16 * r.qu as usize + t] = (if read_result > write_addr { read_result } else { write_addr }) as u8;
    }
}

fn s2_ee_vmax_s16(core: &mut CoreState, tmp_val: u32) {
    let r = decode_pie15(tmp_val);
    for t in 0..8 {
        let read_result = core.q_reg_int16(8 * r.qx as usize + t);
        let write_addr = core.q_reg_int16(8 * r.qy as usize + t);
        core.set_q_reg_int16(8 * r.qu as usize + t, if read_result > write_addr { read_result } else { write_addr });
    }
}

fn s2_ee_vmin_reg_off64(core: &mut CoreState, tmp_val: u32) {
    let r = decode_pie15(tmp_val);
    for t in 0..16 {
        let read_result = (core.q_registers[16 * r.qx as usize + t] as i8) as i32;
        let write_addr = (core.q_registers[16 * r.qy as usize + t] as i8) as i32;
        core.q_registers[16 * r.qu as usize + t] = (if read_result < write_addr { read_result } else { write_addr }) as u8;
    }
}

fn s2_ee_vmin_s16(core: &mut CoreState, tmp_val: u32) {
    let r = decode_pie15(tmp_val);
    for t in 0..8 {
        let read_result = core.q_reg_int16(8 * r.qx as usize + t);
        let write_addr = core.q_reg_int16(8 * r.qy as usize + t);
        core.set_q_reg_int16(8 * r.qu as usize + t, if read_result < write_addr { read_result } else { write_addr });
    }
}

fn s2_ee_vadds_reg_off64(core: &mut CoreState, tmp_val: u32) {
    let r = decode_pie15(tmp_val);
    for t in 0..16 {
        let read_result = (core.q_registers[16 * r.qx as usize + t] as i8) as i32;
        let write_addr = (core.q_registers[16 * r.qy as usize + t] as i8) as i32;
        core.q_registers[16 * r.qu as usize + t] = (255 & clamp_int8(read_result + write_addr)) as u8;
    }
}

fn s2_ee_vadds_s16(core: &mut CoreState, tmp_val: u32) {
    let r = decode_pie15(tmp_val);
    for t in 0..8 {
        core.set_q_reg_int16(8 * r.qu as usize + t, clamp_int16(core.q_reg_int16(8 * r.qx as usize + t) as i32 + core.q_reg_int16(8 * r.qy as usize + t) as i32) as i16);
    }
}

fn s2_ee_vsubs_reg_off64(core: &mut CoreState, tmp_val: u32) {
    let r = decode_pie15(tmp_val);
    for t in 0..16 {
        let read_result = (core.q_registers[16 * r.qx as usize + t] as i8) as i32;
        let write_addr = (core.q_registers[16 * r.qy as usize + t] as i8) as i32;
        core.q_registers[16 * r.qu as usize + t] = (255 & clamp_int8(read_result - write_addr)) as u8;
    }
}

fn s2_ee_vsubs_s16(core: &mut CoreState, tmp_val: u32) {
    let r = decode_pie15(tmp_val);
    for t in 0..8 {
        core.set_q_reg_int16(8 * r.qu as usize + t, clamp_int16(core.q_reg_int16(8 * r.qx as usize + t) as i32 - core.q_reg_int16(8 * r.qy as usize + t) as i32) as i16);
    }
}

fn s2_ee_vmul_reg_off64(core: &mut CoreState, tmp_val: u32) {
    let r = decode_pie15(tmp_val);
    let read_result = if core.sar_m32_pending != 0 { core.sar_m32 as u32 } else { 63 & core.special_registers[12] };
    for t in 0..16 {
        let write_addr = (core.q_registers[16 * r.qx as usize + t] as i8) as i32;
        let register_type = (core.q_registers[16 * r.qy as usize + t] as i8) as i32;
        core.q_registers[16 * r.qu as usize + t] = ((write_addr * register_type) >> read_result) as u8;
    }
}

fn s2_ee_vmul_s16(core: &mut CoreState, tmp_val: u32) {
    let r = decode_pie15(tmp_val);
    let read_result = if core.sar_m32_pending != 0 { core.sar_m32 as u32 } else { 63 & core.special_registers[12] };
    for t in 0..8 {
        let val = ((core.q_reg_int16(8 * r.qx as usize + t) as i32).wrapping_mul(core.q_reg_int16(8 * r.qy as usize + t) as i32) >> read_result) as i16;
        core.set_q_reg_int16(8 * r.qu as usize + t, val);
    }
}

fn s2_ee_vcmp_eq_reg_off64(core: &mut CoreState, tmp_val: u32) {
    let r = decode_pie15(tmp_val);
    for t in 0..16 {
        core.q_registers[16 * r.qu as usize + t] = if core.q_registers[16 * r.qx as usize + t] == core.q_registers[16 * r.qy as usize + t] { 255 } else { 0 };
    }
}

fn s2_ee_vcmp_lt_reg_off64(core: &mut CoreState, tmp_val: u32) {
    let r = decode_pie15(tmp_val);
    for t in 0..16 {
        let read_result = (core.q_registers[16 * r.qx as usize + t] as i8) as i32;
        let write_addr = (core.q_registers[16 * r.qy as usize + t] as i8) as i32;
        core.q_registers[16 * r.qu as usize + t] = if read_result < write_addr { 255 } else { 0 };
    }
}

fn s2_ee_vcmp_gt_reg_off64(core: &mut CoreState, tmp_val: u32) {
    let r = decode_pie15(tmp_val);
    for t in 0..16 {
        let read_result = (core.q_registers[16 * r.qx as usize + t] as i8) as i32;
        let write_addr = (core.q_registers[16 * r.qy as usize + t] as i8) as i32;
        core.q_registers[16 * r.qu as usize + t] = if read_result > write_addr { 255 } else { 0 };
    }
}

fn s2_ee_vcmp_eq_s16(core: &mut CoreState, tmp_val: u32) {
    let r = decode_pie15(tmp_val);
    for t in 0..8 {
        core.set_q_reg_int16(8 * r.qu as usize + t, if core.q_reg_int16(8 * r.qx as usize + t) == core.q_reg_int16(8 * r.qy as usize + t) { -1 } else { 0 });
    }
}

fn s2_ee_vcmp_lt_s16(core: &mut CoreState, tmp_val: u32) {
    let r = decode_pie15(tmp_val);
    for t in 0..8 {
        core.set_q_reg_int16(8 * r.qu as usize + t, if core.q_reg_int16(8 * r.qx as usize + t) < core.q_reg_int16(8 * r.qy as usize + t) { -1 } else { 0 });
    }
}

fn s2_ee_vcmp_gt_s16(core: &mut CoreState, tmp_val: u32) {
    let r = decode_pie15(tmp_val);
    for t in 0..8 {
        core.set_q_reg_int16(8 * r.qu as usize + t, if core.q_reg_int16(8 * r.qx as usize + t) > core.q_reg_int16(8 * r.qy as usize + t) { -1 } else { 0 });
    }
}

fn s2_ee_vmul_u8(core: &mut CoreState, tmp_val: u32) {
    let r = decode_pie15(tmp_val);
    let read_result = if core.sar_m32_pending != 0 { core.sar_m32 as u32 } else { 63 & core.special_registers[12] };
    for t in 0..16 {
        let write_addr = core.q_registers[16 * r.qx as usize + t] as u32;
        let register_type = core.q_registers[16 * r.qy as usize + t] as u32;
        core.q_registers[16 * r.qu as usize + t] = ((write_addr * register_type).wrapping_shr(read_result) & 255) as u8;
    }
}

fn s2_ee_vmul_u16(core: &mut CoreState, tmp_val: u32) {
    let r = decode_pie15(tmp_val);
    let read_result = if core.sar_m32_pending != 0 { core.sar_m32 as u32 } else { 63 & core.special_registers[12] };
    for t in 0..8 {
        let val = (core.q_reg_uint16(8 * r.qx as usize + t) as u32).wrapping_mul(core.q_reg_uint16(8 * r.qy as usize + t) as u32).wrapping_shr(read_result) as u16;
        core.set_q_reg_uint16(8 * r.qu as usize + t, val);
    }
}

fn s2_ee_vadds_s32(core: &mut CoreState, tmp_val: u32) {
    let r = decode_pie15(tmp_val);
    for t in 0..4 {
        core.set_q_reg_int32(4 * r.qu as usize + t, clamp_int32(core.q_reg_int32(4 * r.qx as usize + t).wrapping_add(core.q_reg_int32(4 * r.qy as usize + t)) as i64));
    }
}

fn s2_ee_vsubs_s32(core: &mut CoreState, tmp_val: u32) {
    let r = decode_pie15(tmp_val);
    for t in 0..4 {
        core.set_q_reg_int32(4 * r.qu as usize + t, clamp_int32(core.q_reg_int32(4 * r.qx as usize + t).wrapping_sub(core.q_reg_int32(4 * r.qy as usize + t)) as i64));
    }
}

fn s2_ee_vmax_s32(core: &mut CoreState, tmp_val: u32) {
    let r = decode_pie15(tmp_val);
    for t in 0..4 {
        let read_result = core.q_reg_int32(4 * r.qx as usize + t);
        let write_addr = core.q_reg_int32(4 * r.qy as usize + t);
        core.set_q_reg_int32(4 * r.qu as usize + t, if read_result > write_addr { read_result } else { write_addr });
    }
}

fn s2_ee_vmin_s32(core: &mut CoreState, tmp_val: u32) {
    let r = decode_pie15(tmp_val);
    for t in 0..4 {
        let read_result = core.q_reg_int32(4 * r.qx as usize + t);
        let write_addr = core.q_reg_int32(4 * r.qy as usize + t);
        core.set_q_reg_int32(4 * r.qu as usize + t, if read_result < write_addr { read_result } else { write_addr });
    }
}

fn s2_ee_vcmp_eq_s32(core: &mut CoreState, tmp_val: u32) {
    let r = decode_pie15(tmp_val);
    for t in 0..4 {
        core.set_q_reg_int32(4 * r.qu as usize + t, if core.q_reg_int32(4 * r.qx as usize + t) == core.q_reg_int32(4 * r.qy as usize + t) { -1 } else { 0 });
    }
}

fn s2_ee_vcmp_lt_s32(core: &mut CoreState, tmp_val: u32) {
    let r = decode_pie15(tmp_val);
    for t in 0..4 {
        core.set_q_reg_int32(4 * r.qu as usize + t, if core.q_reg_int32(4 * r.qx as usize + t) < core.q_reg_int32(4 * r.qy as usize + t) { -1 } else { 0 });
    }
}

fn s2_ee_vcmp_gt_s32(core: &mut CoreState, tmp_val: u32) {
    let r = decode_pie15(tmp_val);
    for t in 0..4 {
        core.set_q_reg_int32(4 * r.qu as usize + t, if core.q_reg_int32(4 * r.qx as usize + t) > core.q_reg_int32(4 * r.qy as usize + t) { -1 } else { 0 });
    }
}

fn s2_ee_vmulas_reg_off64_qacc(core: &mut CoreState, tmp_val: u32) {
    let idx_val = decode_pie13(tmp_val);
    let clock_event = decode_pie14(tmp_val);
    for t in 0..16 {
        let simulation_clock = (core.q_registers[16 * idx_val as usize + t as usize] as i8) as i32;
        let read_result = (core.q_registers[16 * clock_event as usize + t as usize] as i8) as i32;
        decode_pie17(core, t as u32, decode_pie16(core, t as u32) + simulation_clock * read_result);
    }
}

fn s2_ee_vmulas_s16_qacc(core: &mut CoreState, tmp_val: u32) {
    let idx_val = decode_pie13(tmp_val);
    let clock_event = decode_pie14(tmp_val);
    for t in 0..8 {
        let simulation_clock = core.q_reg_int16(8 * idx_val as usize + t) as i32;
        let read_result = core.q_reg_int16(8 * clock_event as usize + t) as i32;
        let write_addr = if t < 4 { &mut core.qacc_low } else { &mut core.qacc_high };
        let register_type = if t < 4 { (t % 4) * 5 } else { (t % 4) * 5 };
        let timer_mode =
            (write_addr[register_type] as i128)
                | ((write_addr[register_type + 1] as i128) << 8)
                | ((write_addr[register_type + 2] as i128) << 16)
                | ((write_addr[register_type + 3] as i128) << 24)
                | ((write_addr[register_type + 4] as i128) << 32);
        let timer_mode = if timer_mode & (1i128 << 39) != 0 { timer_mode | !((1i128 << 40) - 1) } else { timer_mode };
        let h_val = timer_mode + (simulation_clock.wrapping_mul(read_result) as i128);
        let off_val = (1i128 << 39) - 1;
        let len_val = -(1i128 << 39);
        let h_val = if h_val > off_val { off_val } else if h_val < len_val { len_val } else { h_val };
        let val_val = h_val & ((1i128 << 40) - 1);
        write_addr[register_type] = (val_val & 255) as u8;
        write_addr[register_type + 1] = ((val_val >> 8) & 255) as u8;
        write_addr[register_type + 2] = ((val_val >> 16) & 255) as u8;
        write_addr[register_type + 3] = ((val_val >> 24) & 255) as u8;
        write_addr[register_type + 4] = ((val_val >> 32) & 255) as u8;
    }
}

fn s2_ee_vmulas_reg_off64_accx(core: &mut CoreState, tmp_val: u32) {
    let idx_val = decode_pie13(tmp_val);
    let clock_event = decode_pie14(tmp_val);
    let mut simulation_clock = 0i32;
    for t in 0..16 {
        simulation_clock +=
            (core.q_registers[16 * idx_val as usize + t] as i8) as i32
                * (core.q_registers[16 * clock_event as usize + t] as i8) as i32;
    }
    let mut accx = core.get_accx() + simulation_clock as i128;
    let read_result = (1i128 << 40) - 1;
    accx &= read_result;
    if accx & (1i128 << 39) != 0 { accx |= !read_result; }
    core.set_accx(accx);
}

fn s2_ee_vmulas_s16_accx(core: &mut CoreState, tmp_val: u32) {
    let idx_val = decode_pie13(tmp_val);
    let clock_event = decode_pie14(tmp_val);
    let mut simulation_clock = 0i128;
    for t in 0..8 {
        simulation_clock +=
            (core.q_reg_int16(8 * idx_val as usize + t) as i128)
                * (core.q_reg_int16(8 * clock_event as usize + t) as i128);
    }
    let mut accx = core.get_accx() + simulation_clock;
    let read_result = (1i128 << 40) - 1;
    accx &= read_result;
    if accx & (1i128 << 39) != 0 { accx |= !read_result; }
    core.set_accx(accx);
}

fn s2_ee_vmulas_u8_qacc(core: &mut CoreState, tmp_val: u32) {
    let idx_val = decode_pie13(tmp_val);
    let clock_event = decode_pie14(tmp_val);
    for t in 0..16 {
        let simulation_clock = core.q_registers[16 * idx_val as usize + t as usize] as u32;
        let read_result = core.q_registers[16 * clock_event as usize + t as usize] as u32;
        decode_pie17(core, t as u32, (decode_pie16(core, t as u32) as u32 + simulation_clock * read_result) as i32);
    }
}

fn s2_ee_vmulas_u16_qacc(core: &mut CoreState, tmp_val: u32) {
    let idx_val = decode_pie13(tmp_val);
    let clock_event = decode_pie14(tmp_val);
    for t in 0..8 {
        let simulation_clock = core.q_reg_uint16(8 * idx_val as usize + t) as u128;
        let read_result = core.q_reg_uint16(8 * clock_event as usize + t) as u128;
        let write_addr = if t < 4 { &mut core.qacc_low } else { &mut core.qacc_high };
        let register_type = (t % 4) * 5;
        let timer_mode =
            (write_addr[register_type] as u128)
                | ((write_addr[register_type + 1] as u128) << 8)
                | ((write_addr[register_type + 2] as u128) << 16)
                | ((write_addr[register_type + 3] as u128) << 24)
                | ((write_addr[register_type + 4] as u128) << 32)
                + simulation_clock * read_result;
        let timer_mode = if timer_mode > 1099511627775 { 1099511627775 } else { timer_mode };
        write_addr[register_type] = (timer_mode & 255) as u8;
        write_addr[register_type + 1] = ((timer_mode >> 8) & 255) as u8;
        write_addr[register_type + 2] = ((timer_mode >> 16) & 255) as u8;
        write_addr[register_type + 3] = ((timer_mode >> 24) & 255) as u8;
        write_addr[register_type + 4] = ((timer_mode >> 32) & 255) as u8;
    }
}

fn s2_ee_vmulas_u8_accx(core: &mut CoreState, tmp_val: u32) {
    let idx_val = decode_pie13(tmp_val);
    let clock_event = decode_pie14(tmp_val);
    let mut simulation_clock = 0u32;
    for t in 0..16 {
        simulation_clock +=
            (core.q_registers[16 * idx_val as usize + t] as u32)
                * (core.q_registers[16 * clock_event as usize + t] as u32);
    }
    let mut accx = core.get_accx() + simulation_clock as i128;
    let read_result = (1i128 << 40) - 1;
    accx &= read_result;
    if accx & (1i128 << 39) != 0 { accx |= !read_result; }
    core.set_accx(accx);
}

fn s2_ee_vmulas_u16_accx(core: &mut CoreState, tmp_val: u32) {
    let idx_val = decode_pie13(tmp_val);
    let clock_event = decode_pie14(tmp_val);
    let mut simulation_clock = 0i128;
    for t in 0..8 {
        simulation_clock +=
            (core.q_reg_uint16(8 * idx_val as usize + t) as i128)
                * (core.q_reg_uint16(8 * clock_event as usize + t) as i128);
    }
    let mut accx = core.get_accx() + simulation_clock;
    let read_result = (1i128 << 40) - 1;
    accx &= read_result;
    if accx & (1i128 << 39) != 0 { accx |= !read_result; }
    core.set_accx(accx);
}

fn s2_ee_vsmulas_s16_qacc(core: &mut CoreState, tmp_val: u32) {
    let idx_val = decode_pie13(tmp_val);
    let clock_event = decode_pie14(tmp_val);
    let simulation_clock = ((tmp_val >> 15) & 1) | (((tmp_val >> 20) & 1) << 1) | (((tmp_val >> 21) & 1) << 2);
    let read_result = core.q_reg_int16(8 * clock_event as usize + (7 & simulation_clock) as usize);
    let write_addr = 549755813887i128;
    for t in 0..8 {
        let ce = (core.q_reg_int16(8 * idx_val as usize + t as usize) as i32).wrapping_mul(read_result as i32) as i128;
        let mut sim = decode_pie7(core, t as u32) + ce;
        if sim > write_addr { sim = write_addr; }
        if sim < -write_addr { sim = -write_addr; }
        decode_pie8(core, t as u32, sim);
    }
}

fn s2_ee_vsmulas_reg_off64_qacc(core: &mut CoreState, tmp_val: u32) {
    let idx_val = decode_pie13(tmp_val);
    let clock_event = decode_pie14(tmp_val);
    let simulation_clock = ((tmp_val >> 4) & 1) | (((tmp_val >> 15) & 1) << 1) | (((tmp_val >> 20) & 1) << 2) | (((tmp_val >> 21) & 1) << 3);
    let read_result = (core.q_registers[16 * clock_event as usize + (15 & simulation_clock) as usize] as i8) as i32;
    for t in 0..16 {
        let ce = (core.q_registers[16 * idx_val as usize + t as usize] as i8) as i32;
        let mut sim = decode_pie16(core, t as u32) as i32 + ce * read_result;
        if sim > 524287 { sim = 524287; }
        if sim < -524287 { sim = -524287; }
        decode_pie17(core, t as u32, sim);
    }
}

fn s2_ee_ledc_channel_qacc_l_signal_direction_128_decode_vq_format_xp(core: &mut CoreState, tmp_val: u32) {
    let idx_val = decode_pie1(tmp_val);
    let clock_event = decode_imm_signed(tmp_val, 16);
    if core.window_check(0, idx_val >> 2, 0) { return; }
    let simulation_clock = 0xfffffff0 & core.ar(idx_val);
    for t in 0..16 {
        core.qacc_low[t] = read_uint8(core, simulation_clock + t as u32) as u8;
    }
    core.set_ar(idx_val, core.ar(idx_val).wrapping_add_signed(clock_event));
}

fn s2_ee_ledc_channel_qacc_h_signal_direction_128_decode_vq_format_xp(core: &mut CoreState, tmp_val: u32) {
    let idx_val = decode_pie1(tmp_val);
    let clock_event = decode_imm_signed(tmp_val, 16);
    if core.window_check(0, idx_val >> 2, 0) { return; }
    let simulation_clock = 0xfffffff0 & core.ar(idx_val);
    for t in 0..16 {
        core.qacc_high[t] = read_uint8(core, simulation_clock + t as u32) as u8;
    }
    core.set_ar(idx_val, core.ar(idx_val).wrapping_add_signed(clock_event));
}

fn s2_ee_ledc_channel_qacc_l_peripheral_type_32_decode_vq_format_xp(core: &mut CoreState, tmp_val: u32) {
    let idx_val = decode_pie1(tmp_val);
    let clock_event = decode_imm_signed(tmp_val, 4);
    if core.window_check(0, idx_val >> 2, 0) { return; }
    let simulation_clock = 0xfffffffc & core.ar(idx_val);
    for t in 0..4 {
        core.qacc_low[16 + t] = read_uint8(core, simulation_clock + t as u32) as u8;
    }
    core.set_ar(idx_val, core.ar(idx_val).wrapping_add_signed(clock_event));
}

fn s2_ee_ledc_channel_qacc_h_peripheral_type_32_decode_vq_format_xp(core: &mut CoreState, tmp_val: u32) {
    let idx_val = decode_pie1(tmp_val);
    let clock_event = decode_imm_signed(tmp_val, 4);
    if core.window_check(0, idx_val >> 2, 0) { return; }
    let simulation_clock = 0xfffffffc & core.ar(idx_val);
    for t in 0..4 {
        core.qacc_high[16 + t] = read_uint8(core, simulation_clock + t as u32) as u8;
    }
    core.set_ar(idx_val, core.ar(idx_val).wrapping_add_signed(clock_event));
}

fn s2_ee_decode_vec_store_format_qacc_l_signal_direction_128_decode_vq_format_xp(core: &mut CoreState, tmp_val: u32) {
    let idx_val = decode_pie1(tmp_val);
    let clock_event = decode_imm_signed(tmp_val, 16);
    if core.window_check(0, idx_val >> 2, 0) { return; }
    let simulation_clock = 0xfffffff0 & core.ar(idx_val);
    for t in 0..16 {
        write_uint8(core, simulation_clock + t as u32, core.qacc_low[t] as u32, 0);
    }
    core.set_ar(idx_val, core.ar(idx_val).wrapping_add_signed(clock_event));
}

fn s2_ee_decode_vec_store_format_qacc_h_signal_direction_128_decode_vq_format_xp(core: &mut CoreState, tmp_val: u32) {
    let idx_val = decode_pie1(tmp_val);
    let clock_event = decode_imm_signed(tmp_val, 16);
    if core.window_check(0, idx_val >> 2, 0) { return; }
    let simulation_clock = 0xfffffff0 & core.ar(idx_val);
    for t in 0..16 {
        write_uint8(core, simulation_clock + t as u32, core.qacc_high[t] as u32, 0);
    }
    core.set_ar(idx_val, core.ar(idx_val).wrapping_add_signed(clock_event));
}

fn s2_ee_decode_vec_store_format_qacc_l_peripheral_type_32_decode_vq_format_xp(core: &mut CoreState, tmp_val: u32) {
    let idx_val = decode_pie1(tmp_val);
    let clock_event = decode_imm_signed(tmp_val, 4);
    if core.window_check(0, idx_val >> 2, 0) { return; }
    let simulation_clock = 0xfffffffc & core.ar(idx_val);
    for t in 0..4 {
        write_uint8(core, simulation_clock + t as u32, core.qacc_low[16 + t] as u32, 0);
    }
    core.set_ar(idx_val, core.ar(idx_val).wrapping_add_signed(clock_event));
}

fn s2_ee_decode_vec_store_format_qacc_h_peripheral_type_32_decode_vq_format_xp(core: &mut CoreState, tmp_val: u32) {
    let idx_val = decode_pie1(tmp_val);
    let clock_event = decode_imm_signed(tmp_val, 4);
    if core.window_check(0, idx_val >> 2, 0) { return; }
    let simulation_clock = 0xfffffffc & core.ar(idx_val);
    for t in 0..4 {
        write_uint8(core, simulation_clock + t as u32, core.qacc_high[16 + t] as u32, 0);
    }
    core.set_ar(idx_val, core.ar(idx_val).wrapping_add_signed(clock_event));
}

fn s2_ee_ldqa_reg_off64_128_xp(core: &mut CoreState, tmp_val: u32) {
    decode_pie20(core, tmp_val, 1);
}

fn s2_ee_ldqa_s16_128_xp(core: &mut CoreState, tmp_val: u32) {
    decode_pie21(core, tmp_val, 1);
}

fn s2_ee_ldqa_u8_128_xp(core: &mut CoreState, tmp_val: u32) {
    decode_pie20(core, tmp_val, 0);
}

fn s2_ee_ldqa_u16_128_xp(core: &mut CoreState, tmp_val: u32) {
    decode_pie21(core, tmp_val, 0);
}

fn s2_ee_slcxxp_2q(core: &mut CoreState, tmp_val: u32) {
    let r = decode_pie22(tmp_val);
    if core.window_check(0, r.a_handler3 >> 2, 0) { return; }
    let write_addr = ((core.ar(r.a_handler3) + 1) & 31) as usize;
    let mut register_type = [0u8; 32];
    for t in 0..16 {
        register_type[t] = core.q_registers[16 * r.qx as usize + t];
        register_type[t + 16] = core.q_registers[16 * r.qu as usize + t];
    }
    for t in 0..write_addr.min(16) {
        core.q_registers[16 * r.qx as usize + t] = 0;
    }
    for t in write_addr..16 {
        core.q_registers[16 * r.qx as usize + t] = register_type[t - write_addr];
    }
    for t in 0..16 {
        core.q_registers[16 * r.qu as usize + t] = register_type[16 - write_addr + t];
    }
    core.set_ar(r.a_handler3, core.ar(r.a_handler3).wrapping_add(core.ar(r.a_handler15)));
}

fn s2_ee_slci_2q(core: &mut CoreState, tmp_val: u32) {
    let idx_val = decode_pie_slot_select(tmp_val) as usize;
    let clock_event = ((tmp_val >> 12) & 7) as usize;
    let simulation_clock = ((((tmp_val >> 4) & 15) + 1) & 31) as usize;
    let mut read_result = [0u8; 32];
    for t in 0..16 {
        read_result[t] = core.q_registers[16 * clock_event + t];
        read_result[t + 16] = core.q_registers[16 * idx_val + t];
    }
    for t in 0..simulation_clock.min(16) {
        core.q_registers[16 * clock_event + t] = 0;
    }
    for t in simulation_clock..16 {
        core.q_registers[16 * clock_event + t] = read_result[t - simulation_clock];
    }
    for t in 0..16 {
        core.q_registers[16 * idx_val + t] = read_result[16 - simulation_clock + t];
    }
}

fn s2_ee_srci_2q(core: &mut CoreState, tmp_val: u32) {
    let idx_val = decode_pie_slot_select(tmp_val) as usize;
    let clock_event = ((tmp_val >> 12) & 7) as usize;
    let simulation_clock = ((((tmp_val >> 4) & 15) + 1) & 31) as usize;
    let mut read_result = [0u8; 32];
    for t in 0..16 {
        read_result[t] = core.q_registers[16 * clock_event + t];
        read_result[t + 16] = core.q_registers[16 * idx_val + t];
    }
    for t in 0..16 {
        core.q_registers[16 * clock_event + t] = read_result[t + simulation_clock];
    }
    for t in 0..(16 - simulation_clock) {
        core.q_registers[16 * idx_val + t] = read_result[t + simulation_clock + 16];
    }
    for t in (16 - simulation_clock)..16 {
        core.q_registers[16 * idx_val + t] = 0;
    }
}

fn s2_ee_srcq_128_decode_vec_store_format_incp(core: &mut CoreState, tmp_val: u32) {
    let idx_val = ((tmp_val >> 12) & 7) as usize;
    let clock_event = decode_pie_slot_select(tmp_val) as usize;
    let simulation_clock = decode_pie1(tmp_val);
    if core.window_check(0, simulation_clock >> 2, 0) { return; }
    let read_result = (31 & core.sar_byte()) as usize;
    let mut write_addr = [0u8; 32];
    for t in 0..16 {
        write_addr[t] = core.q_registers[16 * idx_val + t];
        write_addr[t + 16] = core.q_registers[16 * clock_event + t];
    }
    let register_type = 0xfffffff0 & core.ar(simulation_clock);
    for t in 0..16 {
        write_uint8(core, register_type + t as u32, write_addr[t + read_result] as u32, 0);
    }
    core.set_ar(simulation_clock, core.ar(simulation_clock) + 16);
}

fn s2_ee_fft_vst_r32_decp(core: &mut CoreState, tmp_val: u32) {
    let idx_val = decode_pie_slot_select(tmp_val) as usize;
    let clock_event = decode_pie1(tmp_val);
    let simulation_clock = ((tmp_val >> 10) & 1) as u32;
    if core.window_check(0, clock_event >> 2, 0) { return; }
    let read_result = 0xfffffff0 & core.ar(clock_event);
    for t in 0..4 {
        let ce = 8 * idx_val + (3 - t) * 2;
        let write_addr = (core.q_reg_int16(ce) as i32) >> simulation_clock;
        let register_type = (core.q_reg_int16(ce + 1) as i32) >> simulation_clock;
        write_uint8(core, read_result + 4 * t as u32 + 0, (255 & write_addr) as u32, 0);
        write_uint8(core, read_result + 4 * t as u32 + 1, ((write_addr >> 8) & 255) as u32, 0);
        write_uint8(core, read_result + 4 * t as u32 + 2, (255 & register_type) as u32, 0);
        write_uint8(core, read_result + 4 * t as u32 + 3, ((register_type >> 8) & 255) as u32, 0);
    }
    core.set_ar(clock_event, core.ar(clock_event).wrapping_sub(16));
}

fn s2_ee_srcxxp_2q(core: &mut CoreState, tmp_val: u32) {
    let r = decode_pie22(tmp_val);
    if core.window_check(0, r.a_handler3 >> 2, 0) { return; }
    let write_addr = ((core.ar(r.a_handler3) + 1) & 31) as usize;
    let mut register_type = [0u8; 32];
    for t in 0..16 {
        register_type[t] = core.q_registers[16 * r.qx as usize + t];
        register_type[t + 16] = core.q_registers[16 * r.qu as usize + t];
    }
    for t in 0..16 {
        core.q_registers[16 * r.qx as usize + t] = register_type[t + write_addr];
    }
    for t in 0..(16 - write_addr) {
        core.q_registers[16 * r.qu as usize + t] = register_type[t + write_addr + 16];
    }
    for t in (16 - write_addr)..16 {
        core.q_registers[16 * r.qu as usize + t] = 0;
    }
    core.set_ar(r.a_handler3, core.ar(r.a_handler3).wrapping_add(core.ar(r.a_handler15)));
}

fn s2_ee_bitrev(core: &mut CoreState, tmp_val: u32) {
    let idx_val = decode_pie_slot_select(tmp_val) as usize;
    let clock_event = decode_pie1(tmp_val);
    if core.window_check(0, clock_event >> 2, 0) { return; }
    let simulation_clock = core.ar(clock_event);
    let read_result = core.fft_bit_width();
    for t in 0..8 {
        let ce;
        if read_result == 1 || read_result == 2 {
            ce = decode_pie23((simulation_clock + t) & ((1 << (8 + read_result)) - 1), 8 + read_result);
        } else if read_result == 3 {
            if t >= 6 { ce = 0; } else {
                let cv = (simulation_clock + t) & 7;
                let iv = decode_pie23(cv, 3);
                ce = if cv > iv { cv } else { iv };
            }
        } else {
            let cv = (simulation_clock + t) & ((1 << read_result) - 1);
            let iv = decode_pie23(cv, read_result);
            ce = if cv > iv { cv } else { iv };
        }
        core.q_registers[16 * idx_val + 2 * t as usize] = (ce & 255) as u8;
        core.q_registers[16 * idx_val + 2 * t as usize + 1] = ((ce >> 8) & 255) as u8;
    }
    core.set_ar(clock_event, core.ar(clock_event) + 8);
}

fn s2_ee_vrelu_reg_off64(core: &mut CoreState, tmp_val: u32) {
    let idx_val = decode_pie_slot_select(tmp_val) as usize;
    let clock_event = (tmp_val >> 8) & 15;
    let simulation_clock = (tmp_val >> 4) & 15;
    if core.window_check(0, clock_event >> 2, 0) { return; }
    let read_result = (core.ar(clock_event) as i32) as i16 as i32;
    let write_addr = 63 & core.ar(simulation_clock);
    for t in 0..16 {
        let mut ce = (core.q_registers[16 * idx_val + t] as i8) as i32;
        if ce <= 0 {
            ce = (ce.wrapping_mul(read_result) >> write_addr);
        }
        core.q_registers[16 * idx_val + t] = (255 & ce) as u8;
    }
}

fn s2_ee_vrelu_s16(core: &mut CoreState, tmp_val: u32) {
    let idx_val = decode_pie_slot_select(tmp_val) as usize;
    let clock_event = (tmp_val >> 8) & 15;
    let simulation_clock = (tmp_val >> 4) & 15;
    if core.window_check(0, clock_event >> 2, 0) { return; }
    let read_result = core.ar(clock_event) as i32;
    let write_addr = 63 & core.ar(simulation_clock);
    for t in 0..8 {
        let mut ce = core.q_reg_int16(8 * idx_val + t) as i32;
        if ce <= 0 {
            ce = ce.wrapping_mul(read_result) >> write_addr;
        }
        core.set_q_reg_int16(8 * idx_val + t, ce as i16);
    }
}

fn s2_ee_vzip_16(core: &mut CoreState, tmp_val: u32) {
    let r = decode_pie19(tmp_val);
    let mut simulation_clock = [0u16; 8];
    let mut read_result = [0u16; 8];
    for t in 0..4 {
        simulation_clock[2 * t] = core.q_reg_int16(8 * r.qu as usize + t) as u16;
        simulation_clock[2 * t + 1] = core.q_reg_int16(8 * r.qx as usize + t) as u16;
        read_result[2 * t] = core.q_reg_int16(8 * r.qu as usize + t + 4) as u16;
        read_result[2 * t + 1] = core.q_reg_int16(8 * r.qx as usize + t + 4) as u16;
    }
    for t in 0..8 {
        core.set_q_reg_int16(8 * r.qu as usize + t, simulation_clock[t] as i16);
        core.set_q_reg_int16(8 * r.qx as usize + t, read_result[t] as i16);
    }
}

fn s2_ee_vunzip_16(core: &mut CoreState, tmp_val: u32) {
    let r = decode_pie19(tmp_val);
    let mut simulation_clock = [0u16; 8];
    let mut read_result = [0u16; 8];
    for t in 0..4 {
        simulation_clock[t] = core.q_reg_int16(8 * r.qu as usize + 2 * t) as u16;
        read_result[t] = core.q_reg_int16(8 * r.qu as usize + 2 * t + 1) as u16;
        simulation_clock[t + 4] = core.q_reg_int16(8 * r.qx as usize + 2 * t) as u16;
        read_result[t + 4] = core.q_reg_int16(8 * r.qx as usize + 2 * t + 1) as u16;
    }
    for t in 0..8 {
        core.set_q_reg_int16(8 * r.qu as usize + t, simulation_clock[t] as i16);
        core.set_q_reg_int16(8 * r.qx as usize + t, read_result[t] as i16);
    }
}

fn s2_ee_vunzip_32(core: &mut CoreState, tmp_val: u32) {
    let r = decode_pie19(tmp_val);
    let mut simulation_clock = [0u32; 4];
    let mut read_result = [0u32; 4];
    for t in 0..2 {
        simulation_clock[t] = core.q_reg_uint32(4 * r.qu as usize + 2 * t);
        read_result[t] = core.q_reg_uint32(4 * r.qu as usize + 2 * t + 1);
        simulation_clock[t + 2] = core.q_reg_uint32(4 * r.qx as usize + 2 * t);
        read_result[t + 2] = core.q_reg_uint32(4 * r.qx as usize + 2 * t + 1);
    }
    for t in 0..4 {
        core.set_q_reg_uint32(4 * r.qu as usize + t, simulation_clock[t]);
        core.set_q_reg_uint32(4 * r.qx as usize + t, read_result[t]);
    }
}

fn s2_ee_vzip_32(core: &mut CoreState, tmp_val: u32) {
    let r = decode_pie19(tmp_val);
    let mut simulation_clock = [0u32; 4];
    let mut read_result = [0u32; 4];
    for t in 0..2 {
        simulation_clock[2 * t] = core.q_reg_uint32(4 * r.qu as usize + t);
        simulation_clock[2 * t + 1] = core.q_reg_uint32(4 * r.qx as usize + t);
        read_result[2 * t] = core.q_reg_uint32(4 * r.qu as usize + t + 2);
        read_result[2 * t + 1] = core.q_reg_uint32(4 * r.qx as usize + t + 2);
    }
    for t in 0..4 {
        core.set_q_reg_uint32(4 * r.qu as usize + t, simulation_clock[t]);
        core.set_q_reg_uint32(4 * r.qx as usize + t, read_result[t]);
    }
}

fn s2_ee_ledc_channel_ua_state_decode_vq_format_xp(core: &mut CoreState, tmp_val: u32) {
    let idx_val = decode_pie1(tmp_val);
    if core.window_check(0, idx_val >> 2, 0) { return; }
    let clock_event = core.ar(idx_val);
    let simulation_clock = 0xfffffff0 & clock_event;
    for t in 0..4 {
        core.ua_state[t as usize] = read_uint32(core, simulation_clock + 4 * t as u32);
    }
    core.set_ar(idx_val, clock_event.wrapping_add_signed(decode_imm_signed(tmp_val, 16)));
}

fn s2_ee_decode_vec_store_format_ua_state_decode_vq_format_xp(core: &mut CoreState, tmp_val: u32) {
    let idx_val = decode_pie1(tmp_val);
    if core.window_check(0, idx_val >> 2, 0) { return; }
    let clock_event = core.ar(idx_val);
    let simulation_clock = 0xfffffff0 & clock_event;
    for t in 0..4 {
        write_uint32(core, simulation_clock + 4 * t, core.ua_state[t as usize], 0);
    }
    core.set_ar(idx_val, clock_event.wrapping_add_signed(decode_imm_signed(tmp_val, 16)));
}

fn s2_ee_set_bit_gpio_out(core: &mut CoreState, tmp_val: u32) {
    decode_pie24(core, tmp_val);
}

fn s2_ee_clr_bit_gpio_out(core: &mut CoreState, tmp_val: u32) {
    decode_pie24(core, tmp_val);
}

fn s2_ee_get_gpio_in(core: &mut CoreState, tmp_val: u32) {
    decode_pie24(core, tmp_val);
}

fn s2_ee_wr_mask_gpio_out(core: &mut CoreState, tmp_val: u32) {
    decode_pie24(core, tmp_val);
}

fn s2_ee_vmulas_reg_off64_qacc_ldbc_incp(core: &mut CoreState, tmp_val: u32) {
    let r = decode_pie25(tmp_val);
    if !core.window_check(0, r.a_handler3 >> 2, 0) {
        decode_pie26(core, r.qx, r.qy);
        let __tmp = read_uint8(core, core.ar(r.a_handler3));
        deposit_vec_reg(core, r.qu, 1, __tmp);
        core.set_ar(r.a_handler3, core.ar(r.a_handler3) + 1);
    }
}

fn s2_ee_vmulas_s16_qacc_ldbc_incp(core: &mut CoreState, tmp_val: u32) {
    let r = decode_pie25(tmp_val);
    if core.window_check(0, r.a_handler3 >> 2, 0) { return; }
    decode_pie28(core, r.qx, r.qy);
    let write_addr = core.ar(r.a_handler3);
    let register_type = read_uint8(core, write_addr) | (read_uint8(core, write_addr + 1) << 8);
    deposit_vec_reg(core, r.qu, 2, register_type);
    core.set_ar(r.a_handler3, write_addr + 2);
}

fn s2_ee_vmulas_u8_qacc_ldbc_incp(core: &mut CoreState, tmp_val: u32) {
    let r = decode_pie25(tmp_val);
    if !core.window_check(0, r.a_handler3 >> 2, 0) {
        decode_pie27(core, r.qx, r.qy);
        let __tmp = read_uint8(core, core.ar(r.a_handler3));
        deposit_vec_reg(core, r.qu, 1, __tmp);
        core.set_ar(r.a_handler3, core.ar(r.a_handler3) + 1);
    }
}

fn s2_ee_vmulas_u16_qacc_ldbc_incp(core: &mut CoreState, tmp_val: u32) {
    let r = decode_pie25(tmp_val);
    if core.window_check(0, r.a_handler3 >> 2, 0) { return; }
    decode_pie29(core, r.qx, r.qy);
    let write_addr = core.ar(r.a_handler3);
    let register_type = read_uint8(core, write_addr) | (read_uint8(core, write_addr + 1) << 8);
    deposit_vec_reg(core, r.qu, 2, register_type);
    core.set_ar(r.a_handler3, write_addr + 2);
}

fn s2_ee_vprelu_reg_off64(core: &mut CoreState, tmp_val: u32) {
    let r = decode_pie30(tmp_val);
    if core.window_check(0, r.a_handler3 >> 2, 0) { return; }
    let write_addr = 31 & core.ar(r.a_handler3);
    for t in 0..16 {
        let register_type = (core.q_registers[16 * r.qx as usize + t] as i8) as i32;
        let timer_mode = (core.q_registers[16 * r.qy as usize + t] as i8) as i32;
        let read_result = if register_type <= 0 {
            255 & (((register_type.wrapping_mul(timer_mode) << 16) as i32 >> 16) >> write_addr)
        } else {
            255 & register_type
        };
        core.q_registers[16 * r.qu as usize + t] = read_result as u8;
    }
}

fn s2_ee_vprelu_s16(core: &mut CoreState, tmp_val: u32) {
    let r = decode_pie30(tmp_val);
    if core.window_check(0, r.a_handler3 >> 2, 0) { return; }
    let write_addr = 31 & core.ar(r.a_handler3);
    for t in 0..8 {
        let register_type = core.q_reg_int16(8 * r.qx as usize + t) as i32;
        let timer_mode = core.q_reg_int16(8 * r.qy as usize + t) as i32;
        let read_result = if register_type <= 0 {
            let rr = (register_type.wrapping_mul(timer_mode) >> write_addr) as i16;
            rr as i32
        } else {
            register_type
        };
        core.set_q_reg_int16(8 * r.qu as usize + t, read_result as i16);
    }
}

fn s2_ee_vldbc_8(core: &mut CoreState, tmp_val: u32) {
    let idx_val = decode_pie_slot_select(tmp_val);
    let clock_event = decode_pie1(tmp_val);
    if !core.window_check(0, clock_event >> 2, 0) {
        let __tmp = read_uint8(core, core.ar(clock_event));
        deposit_vec_reg(core, idx_val, 1, __tmp);
    }
}

fn s2_ee_vldbc_16(core: &mut CoreState, tmp_val: u32) {
    let idx_val = decode_pie_slot_select(tmp_val);
    let clock_event = decode_pie1(tmp_val);
    if core.window_check(0, clock_event >> 2, 0) { return; }
    let simulation_clock = read_uint8(core, core.ar(clock_event))
        | (read_uint8(core, core.ar(clock_event) + 1) << 8);
    deposit_vec_reg(core, idx_val, 2, simulation_clock);
}

fn s2_ee_vldbc_32(core: &mut CoreState, tmp_val: u32) {
    let idx_val = decode_pie_slot_select(tmp_val);
    let clock_event = decode_pie1(tmp_val);
    if !core.window_check(0, clock_event >> 2, 0) {
        let __tmp = read_uint32(core, core.ar(clock_event));
        deposit_vec_reg(core, idx_val, 4, __tmp);
    }
}

fn s2_ee_cmul_s16(core: &mut CoreState, tmp_val: u32) {
    let idx_val = decode_pie_slot_select(tmp_val) as usize;
    let clock_event = ((tmp_val >> 8) & 7) as usize;
    let simulation_clock = (((tmp_val >> 11) & 1) | (((tmp_val >> 12) & 1) << 1) | (((tmp_val >> 14) & 1) << 2)) as usize;
    let read_result = (tmp_val >> 4) & 3;
    let write_addr = if core.sar_m32_pending != 0 { core.sar_m32 as u32 } else { 31 & core.special_registers[12] };
    let register_type = if read_result & 1 != 0 { 4 } else { 0 };
    let cfg_val = (read_result & 2) != 0;
    for t in 0..2 {
        let off_val = core.q_reg_int16(8 * clock_event + register_type + 2 * t) as i32;
        let len_val = core.q_reg_int16(8 * clock_event + register_type + 2 * t + 1) as i32;
        let val_val = core.q_reg_int16(8 * simulation_clock + register_type + 2 * t) as i32;
        let i2c_cmd = core.q_reg_int16(8 * simulation_clock + register_type + 2 * t + 1) as i32;
        let (rr, hv) = if cfg_val {
            ((off_val.wrapping_mul(val_val) + len_val.wrapping_mul(i2c_cmd)) >> write_addr,
             (off_val.wrapping_mul(i2c_cmd) - len_val.wrapping_mul(val_val)) >> write_addr)
        } else {
            ((off_val.wrapping_mul(val_val) - len_val.wrapping_mul(i2c_cmd)) >> write_addr,
             (off_val.wrapping_mul(i2c_cmd) + len_val.wrapping_mul(val_val)) >> write_addr)
        };
        core.set_q_reg_int16(8 * idx_val + register_type + 2 * t, rr as i16);
        core.set_q_reg_int16(8 * idx_val + register_type + 2 * t + 1, hv as i16);
    }
}

fn s2_ee_fft_r2bf_s16(core: &mut CoreState, tmp_val: u32) {
    let idx_val = (((tmp_val >> 15) & 1) | (((tmp_val >> 20) & 1) << 1) | (((tmp_val >> 21) & 1) << 2)) as usize;
    let clock_event = (((tmp_val >> 12) & 1) | (((tmp_val >> 13) & 1) << 1) | (((tmp_val >> 14) & 1) << 2)) as usize;
    let simulation_clock = (((tmp_val >> 4) & 1) | (((tmp_val >> 6) & 1) << 1) | (((tmp_val >> 7) & 1) << 2)) as usize;
    let read_result = (((tmp_val >> 5) & 1) | (((tmp_val >> 10) & 1) << 1) | (((tmp_val >> 11) & 1) << 2)) as usize;
    let write_addr = (tmp_val >> 8) & 1;
    let mut register_type = [0i16; 8];
    let mut cfg_val = [0i16; 8];
    if write_addr == 0 {
        for t in 0..4 {
            register_type[t] = core.q_reg_int16(8 * simulation_clock + t);
            register_type[t + 4] = core.q_reg_int16(8 * read_result + t);
            cfg_val[t] = core.q_reg_int16(8 * simulation_clock + t + 4);
            cfg_val[t + 4] = core.q_reg_int16(8 * read_result + t + 4);
        }
    } else {
        register_type[0] = core.q_reg_int16(8 * simulation_clock + 0);
        register_type[1] = core.q_reg_int16(8 * simulation_clock + 1);
        register_type[2] = core.q_reg_int16(8 * simulation_clock + 4);
        register_type[3] = core.q_reg_int16(8 * simulation_clock + 5);
        register_type[4] = core.q_reg_int16(8 * read_result + 0);
        register_type[5] = core.q_reg_int16(8 * read_result + 1);
        register_type[6] = core.q_reg_int16(8 * read_result + 4);
        register_type[7] = core.q_reg_int16(8 * read_result + 5);
        cfg_val[0] = core.q_reg_int16(8 * simulation_clock + 2);
        cfg_val[1] = core.q_reg_int16(8 * simulation_clock + 3);
        cfg_val[2] = core.q_reg_int16(8 * simulation_clock + 6);
        cfg_val[3] = core.q_reg_int16(8 * simulation_clock + 7);
        cfg_val[4] = core.q_reg_int16(8 * read_result + 2);
        cfg_val[5] = core.q_reg_int16(8 * read_result + 3);
        cfg_val[6] = core.q_reg_int16(8 * read_result + 6);
        cfg_val[7] = core.q_reg_int16(8 * read_result + 7);
    }
    let h_val = |v: i32| -> i16 { if v > 32767 { 32767 } else if v < -32768 { -32768 } else { v as i16 } };
    for t in 0..4 {
        core.set_q_reg_int16(8 * idx_val + t, h_val(register_type[t] as i32 + cfg_val[t] as i32));
        core.set_q_reg_int16(8 * idx_val + t + 4, h_val(register_type[t] as i32 - cfg_val[t] as i32));
        core.set_q_reg_int16(8 * clock_event + t, h_val(register_type[t + 4] as i32 + cfg_val[t + 4] as i32));
        core.set_q_reg_int16(8 * clock_event + t + 4, h_val(register_type[t + 4] as i32 - cfg_val[t + 4] as i32));
    }
}

fn s2_ee_ldf_64_xp(core: &mut CoreState, tmp_val: u32) {
    let idx_val = ((tmp_val >> 12) & 15) as usize;
    let clock_event = ((tmp_val >> 20) & 15) as usize;
    let simulation_clock = (tmp_val >> 4) & 15;
    let read_result = (tmp_val >> 8) & 15;
    if core.window_check(0, simulation_clock >> 2, 0) { return; }
    let write_addr = 0xfffffff8 & core.ar(simulation_clock);
    core.float_registers[idx_val as usize] = read_uint32(core, write_addr);
    core.float_registers[clock_event as usize] = read_uint32(core, write_addr + 4);
    core.set_ar(simulation_clock, core.ar(simulation_clock).wrapping_add(core.ar(read_result)));
}

fn s2_ee_stf_64_xp(core: &mut CoreState, tmp_val: u32) {
    let idx_val = ((tmp_val >> 12) & 15) as usize;
    let clock_event = ((tmp_val >> 20) & 15) as usize;
    let simulation_clock = (tmp_val >> 4) & 15;
    let read_result = (tmp_val >> 8) & 15;
    if core.window_check(0, simulation_clock >> 2, 0) { return; }
    let write_addr = 0xfffffff8 & core.ar(simulation_clock);
    write_uint32(core, write_addr, core.float_registers[idx_val as usize], 0);
    write_uint32(core, write_addr + 4, core.float_registers[clock_event as usize], 0);
    core.set_ar(simulation_clock, core.ar(simulation_clock).wrapping_add(core.ar(read_result)));
}

pub static S2_INST_TABLE: &[S2InstEntry] = &[
    S2InstEntry { name: "ee.vld.128.decodeVQFormatXp", mask: 9371663, opcode: 8585220, func: s2_ee_vld_128_decode_vq_format_xp },
    S2InstEntry { name: "ee.vld.128.xp", mask: 0xcf700f, opcode: 9248772, func: s2_ee_vld_128_xp },
    S2InstEntry { name: "ee.vld.PeripheralType.64.decodeVQFormatXp", mask: 9371663, opcode: 8912900, func: s2_ee_vld_peripheral_type_64_decode_vq_format_xp },
    S2InstEntry { name: "ee.vld.SignalDirection.64.decodeVQFormatXp", mask: 9371663, opcode: 8978436, func: s2_ee_vld_signal_direction_64_decode_vq_format_xp },
    S2InstEntry { name: "ee.vld.SignalDirection.64.xp", mask: 0xcf700f, opcode: 9252868, func: s2_ee_vld_signal_direction_64_xp },
    S2InstEntry { name: "ee.vld.PeripheralType.64.xp", mask: 0xcf700f, opcode: 9265156, func: s2_ee_vld_peripheral_type_64_xp },
    S2InstEntry { name: "ee.vldbc.8.decodeVQFormatXp", mask: 0xcf000f, opcode: 0xc50004, func: s2_ee_vldbc_8_decode_vq_format_xp },
    S2InstEntry { name: "ee.vldbc.16.decodeVQFormatXp", mask: 0xcf000f, opcode: 8716292, func: s2_ee_vldbc_16_decode_vq_format_xp },
    S2InstEntry { name: "ee.vldbc.32.decodeVQFormatXp", mask: 9371663, opcode: 8519684, func: s2_ee_vldbc_32_decode_vq_format_xp },
    S2InstEntry { name: "ee.vldbc.8.xp", mask: 0xcf700f, opcode: 9261060, func: s2_ee_vldbc_8_xp },
    S2InstEntry { name: "ee.vldbc.16.xp", mask: 0xcf700f, opcode: 9256964, func: s2_ee_vldbc_16_xp },
    S2InstEntry { name: "ee.vldbc.32.xp", mask: 0xcf700f, opcode: 9244676, func: s2_ee_vldbc_32_xp },
    S2InstEntry { name: "ee.vldhbc.16.incp", mask: 0xcf0f0f, opcode: 0xcc0204, func: s2_ee_vldhbc_16_incp },
    S2InstEntry { name: "ee.vst.128.decodeVQFormatXp", mask: 9371663, opcode: 9043972, func: s2_ee_vst_128_decode_vq_format_xp },
    S2InstEntry { name: "ee.vst.128.xp", mask: 0xcf700f, opcode: 9269252, func: s2_ee_vst_128_xp },
    S2InstEntry { name: "ee.vst.PeripheralType.64.decodeVQFormatXp", mask: 9371663, opcode: 9109508, func: s2_ee_vst_peripheral_type_64_decode_vq_format_xp },
    S2InstEntry { name: "ee.vst.SignalDirection.64.decodeVQFormatXp", mask: 9371663, opcode: 8650756, func: s2_ee_vst_signal_direction_64_decode_vq_format_xp },
    S2InstEntry { name: "ee.vst.PeripheralType.64.xp", mask: 0xcf700f, opcode: 0xcd0004, func: s2_ee_vst_peripheral_type_64_xp },
    S2InstEntry { name: "ee.vst.SignalDirection.64.xp", mask: 0xcf700f, opcode: 0xcd4004, func: s2_ee_vst_signal_direction_64_xp },
    S2InstEntry { name: "ee.movi.32.q", mask: 0xcf730f, opcode: 0xcd3204, func: s2_ee_movi_32_q },
    S2InstEntry { name: "ee.movi.32.trapCauseMap", mask: 0xcf730f, opcode: 0xcd7104, func: s2_ee_movi_32_trap_cause_map },
    S2InstEntry { name: "ee.zero.q", mask: 0xcf7fff, opcode: 0xcd7fa4, func: s2_ee_zero_q },
    S2InstEntry { name: "ee.zero.qacc", mask: 0xffffff, opcode: 2426948, func: s2_ee_zero_qacc },
    S2InstEntry { name: "ee.zero.accx", mask: 0xffffff, opcode: 2426884, func: s2_ee_zero_accx },
    S2InstEntry { name: "ee.LedcChannel.128.usar.decodeVQFormatXp", mask: 9371663, opcode: 8454148, func: s2_ee_ledc_channel_128_usar_decode_vq_format_xp },
    S2InstEntry { name: "ee.LedcChannel.128.usar.xp", mask: 0xcf700f, opcode: 9240580, func: s2_ee_ledc_channel_128_usar_xp },
    S2InstEntry { name: "ee.src.q", mask: 0xcf0f8f, opcode: 0xcc0304, func: decode_pie5 },
    S2InstEntry { name: "ee.src.q.qup", mask: 0xcf0f8f, opcode: 0xcc0704, func: decode_pie6 },
    S2InstEntry { name: "ee.srcmb.regOff64.qacc", mask: 0xcf7b0f, opcode: 0xcd7a04, func: s2_ee_srcmb_reg_off64_qacc },
    S2InstEntry { name: "ee.srcmb.s16.qacc", mask: 0xcf7b0f, opcode: 0xcd7204, func: s2_ee_srcmb_s16_qacc },
    S2InstEntry { name: "ee.mov.regOff64.qacc", mask: 0xcf7fff, opcode: 0xcd7f34, func: s2_ee_mov_reg_off64_qacc },
    S2InstEntry { name: "ee.mov.s16.qacc", mask: 0xcf7fff, opcode: 0xcd7f24, func: s2_ee_mov_s16_qacc },
    S2InstEntry { name: "ee.mov.u8.qacc", mask: 0xcf7fff, opcode: 0xcd7f74, func: s2_ee_mov_u8_qacc },
    S2InstEntry { name: "ee.mov.u16.qacc", mask: 0xcf7fff, opcode: 0xcd7f64, func: s2_ee_mov_u16_qacc },
    S2InstEntry { name: "ee.LedcChannel.accx.decodeVQFormatXp", mask: 0xbf000f, opcode: 917508, func: s2_ee_ledc_channel_accx_decode_vq_format_xp },
    S2InstEntry { name: "ee.decodeVecStoreFormat.accx.decodeVQFormatXp", mask: 0xbf000f, opcode: 131076, func: s2_ee_decode_vec_store_format_accx_decode_vq_format_xp },
    S2InstEntry { name: "ee.srs.accx", mask: 0xff100f, opcode: 8261636, func: s2_ee_srs_accx },
    S2InstEntry { name: "ee.ldqa.regOff64.128.decodeVQFormatXp", mask: 0xbf000f, opcode: 1114116, func: s2_ee_ldqa_reg_off64_128_decode_vq_format_xp },
    S2InstEntry { name: "ee.ldqa.s16.128.decodeVQFormatXp", mask: 0xbf000f, opcode: 65540, func: s2_ee_ldqa_s16_128_decode_vq_format_xp },
    S2InstEntry { name: "ee.ldqa.u8.128.decodeVQFormatXp", mask: 0xbf000f, opcode: 1376260, func: s2_ee_ldqa_u8_128_decode_vq_format_xp },
    S2InstEntry { name: "ee.ldqa.u16.128.decodeVQFormatXp", mask: 0xbf000f, opcode: 327684, func: s2_ee_ldqa_u16_128_decode_vq_format_xp },
    S2InstEntry { name: "ee.andq", mask: 0xcf730f, opcode: 0xcd3004, func: s2_ee_andq },
    S2InstEntry { name: "ee.orq", mask: 0xcf730f, opcode: 0xcd7004, func: s2_ee_orq },
    S2InstEntry { name: "ee.xorq", mask: 0xcf730f, opcode: 0xcd3104, func: s2_ee_xorq },
    S2InstEntry { name: "ee.notq", mask: 0xcf7f0f, opcode: 0xcd7f04, func: s2_ee_notq },
    S2InstEntry { name: "ee.vsl.32", mask: 0xcf7f8f, opcode: 0xcd3f04, func: s2_ee_vsl_32 },
    S2InstEntry { name: "ee.vsr.32", mask: 0xcf7f8f, opcode: 0xcd3f84, func: s2_ee_vsr_32 },
    S2InstEntry { name: "ee.vzip.8", mask: 0xcf0fff, opcode: 0xcc03d4, func: s2_ee_vzip_8 },
    S2InstEntry { name: "ee.vunzip.8", mask: 0xcf0fff, opcode: 0xcc03a4, func: s2_ee_vunzip_8 },
    S2InstEntry { name: "ee.vmax.regOff64", mask: 0xcf20ff, opcode: 9314372, func: s2_ee_vmax_reg_off64 },
    S2InstEntry { name: "ee.vmax.s16", mask: 0xcf20ff, opcode: 9314340, func: s2_ee_vmax_s16 },
    S2InstEntry { name: "ee.vmin.regOff64", mask: 0xcf20ff, opcode: 9314420, func: s2_ee_vmin_reg_off64 },
    S2InstEntry { name: "ee.vmin.s16", mask: 0xcf20ff, opcode: 9314388, func: s2_ee_vmin_s16 },
    S2InstEntry { name: "ee.vadds.regOff64", mask: 0xcf20ff, opcode: 9306244, func: s2_ee_vadds_reg_off64 },
    S2InstEntry { name: "ee.vadds.s16", mask: 0xcf20ff, opcode: 9306212, func: s2_ee_vadds_s16 },
    S2InstEntry { name: "ee.vsubs.regOff64", mask: 0xcf20ff, opcode: 9314548, func: s2_ee_vsubs_reg_off64 },
    S2InstEntry { name: "ee.vsubs.s16", mask: 0xcf20ff, opcode: 9314516, func: s2_ee_vsubs_s16 },
    S2InstEntry { name: "ee.vmul.regOff64", mask: 0xcf20ff, opcode: 9314452, func: s2_ee_vmul_reg_off64 },
    S2InstEntry { name: "ee.vmul.s16", mask: 0xcf20ff, opcode: 9314436, func: s2_ee_vmul_s16 },
    S2InstEntry { name: "ee.vcmp.eq.regOff64", mask: 0xcf20ff, opcode: 9306292, func: s2_ee_vcmp_eq_reg_off64 },
    S2InstEntry { name: "ee.vcmp.lt.regOff64", mask: 0xcf20ff, opcode: 9314324, func: s2_ee_vcmp_lt_reg_off64 },
    S2InstEntry { name: "ee.vcmp.gt.regOff64", mask: 0xcf20ff, opcode: 9306340, func: s2_ee_vcmp_gt_reg_off64 },
    S2InstEntry { name: "ee.vcmp.eq.s16", mask: 0xcf20ff, opcode: 9306260, func: s2_ee_vcmp_eq_s16 },
    S2InstEntry { name: "ee.vcmp.lt.s16", mask: 0xcf20ff, opcode: 9306356, func: s2_ee_vcmp_lt_s16 },
    S2InstEntry { name: "ee.vcmp.gt.s16", mask: 0xcf20ff, opcode: 9306308, func: s2_ee_vcmp_gt_s16 },
    S2InstEntry { name: "ee.vmul.u8", mask: 0xcf20ff, opcode: 9314484, func: s2_ee_vmul_u8 },
    S2InstEntry { name: "ee.vmul.u16", mask: 0xcf20ff, opcode: 9314468, func: s2_ee_vmul_u16 },
    S2InstEntry { name: "ee.vadds.s32", mask: 0xcf20ff, opcode: 9306228, func: s2_ee_vadds_s32 },
    S2InstEntry { name: "ee.vsubs.s32", mask: 0xcf20ff, opcode: 9314532, func: s2_ee_vsubs_s32 },
    S2InstEntry { name: "ee.vmax.s32", mask: 0xcf20ff, opcode: 9314356, func: s2_ee_vmax_s32 },
    S2InstEntry { name: "ee.vmin.s32", mask: 0xcf20ff, opcode: 9314404, func: s2_ee_vmin_s32 },
    S2InstEntry { name: "ee.vcmp.eq.s32", mask: 0xcf20ff, opcode: 9306276, func: s2_ee_vcmp_eq_s32 },
    S2InstEntry { name: "ee.vcmp.lt.s32", mask: 0xcf20ff, opcode: 9314308, func: s2_ee_vcmp_lt_s32 },
    S2InstEntry { name: "ee.vcmp.gt.s32", mask: 0xcf20ff, opcode: 9306324, func: s2_ee_vcmp_gt_s32 },
    S2InstEntry { name: "ee.vmulas.regOff64.qacc", mask: 0xffa0ff, opcode: 1712324, func: s2_ee_vmulas_reg_off64_qacc },
    S2InstEntry { name: "ee.vmulas.s16.qacc", mask: 0xffa0ff, opcode: 1712260, func: s2_ee_vmulas_s16_qacc },
    S2InstEntry { name: "ee.vmulas.regOff64.accx", mask: 0xffa0ff, opcode: 1704132, func: s2_ee_vmulas_reg_off64_accx },
    S2InstEntry { name: "ee.vmulas.s16.accx", mask: 0xffa0ff, opcode: 1704068, func: s2_ee_vmulas_s16_accx },
    S2InstEntry { name: "ee.vmulas.u8.qacc", mask: 0xffa0ff, opcode: 663748, func: s2_ee_vmulas_u8_qacc },
    S2InstEntry { name: "ee.vmulas.u16.qacc", mask: 0xffa0ff, opcode: 663684, func: s2_ee_vmulas_u16_qacc },
    S2InstEntry { name: "ee.vmulas.u8.accx", mask: 0xffa0ff, opcode: 655556, func: s2_ee_vmulas_u8_accx },
    S2InstEntry { name: "ee.vmulas.u16.accx", mask: 0xffa0ff, opcode: 655492, func: s2_ee_vmulas_u16_accx },
    S2InstEntry { name: "ee.vsmulas.s16.qacc", mask: 0xcf20c4, opcode: 9314500, func: s2_ee_vsmulas_s16_qacc },
    S2InstEntry { name: "ee.vsmulas.regOff64.qacc", mask: 0xcf20e4, opcode: 9306180, func: s2_ee_vsmulas_reg_off64_qacc },
    S2InstEntry { name: "ee.LedcChannel.qacc_l.SignalDirection.128.decodeVQFormatXp", mask: 0xbf000f, opcode: 4, func: s2_ee_ledc_channel_qacc_l_signal_direction_128_decode_vq_format_xp },
    S2InstEntry { name: "ee.LedcChannel.qacc_h.SignalDirection.128.decodeVQFormatXp", mask: 0xbf000f, opcode: 393220, func: s2_ee_ledc_channel_qacc_h_signal_direction_128_decode_vq_format_xp },
    S2InstEntry { name: "ee.LedcChannel.qacc_l.PeripheralType.32.decodeVQFormatXp", mask: 0xbf000f, opcode: 1441796, func: s2_ee_ledc_channel_qacc_l_peripheral_type_32_decode_vq_format_xp },
    S2InstEntry { name: "ee.LedcChannel.qacc_h.PeripheralType.32.decodeVQFormatXp", mask: 0xbf000f, opcode: 1966084, func: s2_ee_ledc_channel_qacc_h_peripheral_type_32_decode_vq_format_xp },
    S2InstEntry { name: "ee.decodeVecStoreFormat.qacc_l.SignalDirection.128.decodeVQFormatXp", mask: 0xbf000f, opcode: 786436, func: s2_ee_decode_vec_store_format_qacc_l_signal_direction_128_decode_vq_format_xp },
    S2InstEntry { name: "ee.decodeVecStoreFormat.qacc_h.SignalDirection.128.decodeVQFormatXp", mask: 0xbf000f, opcode: 851972, func: s2_ee_decode_vec_store_format_qacc_h_signal_direction_128_decode_vq_format_xp },
    S2InstEntry { name: "ee.decodeVecStoreFormat.qacc_l.PeripheralType.32.decodeVQFormatXp", mask: 0xbf000f, opcode: 1900548, func: s2_ee_decode_vec_store_format_qacc_l_peripheral_type_32_decode_vq_format_xp },
    S2InstEntry { name: "ee.decodeVecStoreFormat.qacc_h.PeripheralType.32.decodeVQFormatXp", mask: 0xbf000f, opcode: 1179652, func: s2_ee_decode_vec_store_format_qacc_h_peripheral_type_32_decode_vq_format_xp },
    S2InstEntry { name: "ee.ldqa.regOff64.128.xp", mask: 0xfff00f, opcode: 7421956, func: s2_ee_ldqa_reg_off64_128_xp },
    S2InstEntry { name: "ee.ldqa.s16.128.xp", mask: 0xfff00f, opcode: 8273924, func: s2_ee_ldqa_s16_128_xp },
    S2InstEntry { name: "ee.ldqa.u8.128.xp", mask: 0xfff00f, opcode: 7356420, func: s2_ee_ldqa_u8_128_xp },
    S2InstEntry { name: "ee.ldqa.u16.128.xp", mask: 0xfff00f, opcode: 8011780, func: s2_ee_ldqa_u16_128_xp },
    S2InstEntry { name: "ee.slcxxp.2q", mask: 0xcf000f, opcode: 8781828, func: s2_ee_slcxxp_2q },
    S2InstEntry { name: "ee.slci.2q", mask: 0xcf0f0f, opcode: 0xcc0604, func: s2_ee_slci_2q },
    S2InstEntry { name: "ee.srci.2q", mask: 0xcf0f0f, opcode: 0xcc0a04, func: s2_ee_srci_2q },
    S2InstEntry { name: "ee.srcq.128.decodeVecStoreFormat.incp", mask: 0xcf0f0f, opcode: 0xcc0e04, func: s2_ee_srcq_128_decode_vec_store_format_incp },
    S2InstEntry { name: "ee.fft.vst.r32.decp", mask: 0xcf7b0f, opcode: 0xcd3304, func: s2_ee_fft_vst_r32_decp },
    S2InstEntry { name: "ee.srcxxp.2q", mask: 0xcf000f, opcode: 0xc60004, func: s2_ee_srcxxp_2q },
    S2InstEntry { name: "ee.bitrev", mask: 0xcf7f0f, opcode: 0xcd7b04, func: s2_ee_bitrev },
    S2InstEntry { name: "ee.vrelu.regOff64", mask: 0xcf600f, opcode: 0xcd5004, func: s2_ee_vrelu_reg_off64 },
    S2InstEntry { name: "ee.vrelu.s16", mask: 0xcf600f, opcode: 0xcd1004, func: s2_ee_vrelu_s16 },
    S2InstEntry { name: "ee.vzip.16", mask: 0xcf0fff, opcode: 0xcc03b4, func: s2_ee_vzip_16 },
    S2InstEntry { name: "ee.vunzip.16", mask: 0xcf0fff, opcode: 0xcc0384, func: s2_ee_vunzip_16 },
    S2InstEntry { name: "ee.vunzip.32", mask: 0xcf0fff, opcode: 0xcc0394, func: s2_ee_vunzip_32 },
    S2InstEntry { name: "ee.vzip.32", mask: 0xcf0fff, opcode: 0xcc03c4, func: s2_ee_vzip_32 },
    S2InstEntry { name: "ee.LedcChannel.ua_state.decodeVQFormatXp", mask: 0xbf000f, opcode: 1048580, func: s2_ee_ledc_channel_ua_state_decode_vq_format_xp },
    S2InstEntry { name: "ee.decodeVecStoreFormat.ua_state.decodeVQFormatXp", mask: 0xbf000f, opcode: 1835012, func: s2_ee_decode_vec_store_format_ua_state_decode_vq_format_xp },
    S2InstEntry { name: "ee.set_bit_gpio_out", mask: 0xffe00f, opcode: 7684100, func: s2_ee_set_bit_gpio_out },
    S2InstEntry { name: "ee.clr_bit_gpio_out", mask: 0xffe00f, opcode: 7749636, func: s2_ee_clr_bit_gpio_out },
    S2InstEntry { name: "ee.get_gpio_in", mask: 0xffff0f, opcode: 6621188, func: s2_ee_get_gpio_in },
    S2InstEntry { name: "ee.wr_mask_gpio_out", mask: 0xff000f, opcode: 7487492, func: s2_ee_wr_mask_gpio_out },
    S2InstEntry { name: "ee.vmulas.regOff64.qacc.ldbc.incp", mask: 0xef000f, opcode: 0xa70004, func: s2_ee_vmulas_reg_off64_qacc_ldbc_incp },
    S2InstEntry { name: "ee.vmulas.s16.qacc.ldbc.incp", mask: 0xef000f, opcode: 8847364, func: s2_ee_vmulas_s16_qacc_ldbc_incp },
    S2InstEntry { name: "ee.vmulas.u8.qacc.ldbc.incp", mask: 0xef000f, opcode: 0xe70004, func: s2_ee_vmulas_u8_qacc_ldbc_incp },
    S2InstEntry { name: "ee.vmulas.u16.qacc.ldbc.incp", mask: 0xef000f, opcode: 0xc70004, func: s2_ee_vmulas_u16_qacc_ldbc_incp },
    S2InstEntry { name: "ee.vprelu.regOff64", mask: 0xcf200f, opcode: 9183236, func: s2_ee_vprelu_reg_off64 },
    S2InstEntry { name: "ee.vprelu.s16", mask: 0xcf200f, opcode: 9175044, func: s2_ee_vprelu_s16 },
    S2InstEntry { name: "ee.vldbc.8", mask: 0xcf7f0f, opcode: 0xcd3b04, func: s2_ee_vldbc_8 },
    S2InstEntry { name: "ee.vldbc.16", mask: 0xcf7f0f, opcode: 0xcd7304, func: s2_ee_vldbc_16 },
    S2InstEntry { name: "ee.vldbc.32", mask: 0xcf7f0f, opcode: 0xcd7704, func: s2_ee_vldbc_32 },
    S2InstEntry { name: "ee.cmul.s16", mask: 0xcf20cf, opcode: 9306116, func: s2_ee_cmul_s16 },
    S2InstEntry { name: "ee.fft.r2bf.s16", mask: 0xcf020f, opcode: 0xcc0004, func: s2_ee_fft_r2bf_s16 },
    S2InstEntry { name: "ee.ldf.64.xp", mask: 458767, opcode: 393216, func: s2_ee_ldf_64_xp },
    S2InstEntry { name: "ee.stf.64.xp", mask: 458767, opcode: 458752, func: s2_ee_stf_64_xp },
];

pub fn decode_pie31(core: &mut CoreState, tmp_val: u32) -> u32 {
    // JS builds a 256-slot lookup table; we iterate S2_INST_TABLE directly (identical behavior)
    for entry in S2_INST_TABLE {
        if (tmp_val & entry.mask) == entry.opcode {
            (entry.func)(core, tmp_val);
            return 1;
        }
    }
    0
}
