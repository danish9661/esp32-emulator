use crate::peripherals::types::*;
use crate::peripherals::common::peripheral::PeripheralBase;
use crate::crypto::sha::ShaEngine;
use crate::crypto::aes::{AesCore, AesEcb};
use crate::crypto::ecc::{ecc_curve_params, jacobian_from_affine, jacobian_to_affine, ecc_jacobian_scalar_mult, EccPoint};
use crate::crypto::bigint_math::{bytes_to_bigint, bigint_to_bytes, U384};

pub const RSA_REG7: u32 = 128;
pub const RSA_REG8: u32 = 132;
pub const RSA_REG9: u32 = 136;
pub const RSA_REG10: u32 = 140;
pub const RSA_REG11: u32 = 144;
pub const RSA_REG12: u32 = 148;
pub const RSA_REG13: u32 = 152;
pub const RSA_REG14: u32 = 156;
pub const RSA_REG15: u32 = 160;
pub const RSA_REG16: u32 = 164;
pub const RSA_REG17: u32 = 168;
pub const RSA_REG18: u32 = 172;
pub const RSA_REG19: u32 = 176;
pub const RSA_REG20: u32 = 180;
pub const RSA_REG21: u32 = 184;
pub const RSA_REG22: u32 = 188;
pub const RSA_REG23: [u32; 4] = [20, 32, 48, 64];
pub const RSA_REG24: u32 = 4;
pub const RSA_REG25: u32 = 8;
pub const RSA_REG26: u32 = 12;
pub const RSA_REG27: u32 = 16;
pub const RSA_REG28: u32 = 20;
pub const RSA_REG29: u32 = 24;
pub const RSA_REG30: u32 = 28;
pub const RSA_REG31: u32 = 32;
pub const RSA_REG32: u32 = 36;
pub const RSA_REG33: u32 = 40;
pub const RSA_REG34: u32 = 44;
pub const RSA_REG35: u32 = 48;
pub const RSA_REG36: u32 = 52;
pub const RSA_REG37: u32 = 252;
pub const RSA_REG38: u32 = 256;
pub const RSA_REG39: u32 = 320;
pub const RSA_REG40: u32 = 384;
pub const RSA_REG41: u32 = 1;
pub const RSA_REG42: u32 = 2;
pub const RSA_REG43: u32 = 4;
pub const RSA_REG44: u32 = 7;
pub const RSA_REG45: u32 = 5;

pub const SHA_PERIPHERAL_MODE_SHA1: u32 = 0;
pub const SHA_PERIPHERAL_MODE_SHA256: u32 = 1;
pub const SHA_PERIPHERAL_MODE_SHA384: u32 = 2;
pub const SHA_PERIPHERAL_MODE_SHA512: u32 = 3;

pub const SHA_OPERATION_START: u32 = 0;
pub const SHA_OPERATION_CONTINUE: u32 = 1;
pub const SHA_OPERATION_FINAL: u32 = 2;

pub const ECC_REG1: u32 = 31;
pub const ECC_REG2: u32 = 1024;
pub const ECC_REG3: u32 = 2048;
pub const ECC_REG4: u32 = 31;
pub const ECC_REG5: u32 = 32;
pub const ECC_REG6: u32 = 64;
pub const ECC_REG7: u32 = 128;
pub const ECC_REG8: u32 = 1;
pub const ECC_REG9: u32 = 2;
pub const ECC_REG10: u32 = 7;
pub const ECC_REG11: u32 = 3;
pub const ECC_REG12: u32 = 15;
pub const ECC_REG14: u32 = 1;
pub const ECC_REG15: u32 = 2;
pub const ECC_REG16: u32 = 4;
pub const ECC_REG17: u32 = 8;
pub const ECC_REG18: u32 = 16;
pub const ECC_REG19: u32 = 32;
pub const ECC_REG20: u32 = 64;
pub const ECC_REG13: U384 = U384([0, 0, 0x0123456789abcdef, 0x0123456789abcdef, 0x0123456789abcdef, 0x0123456789abcdef]);
pub const ECC_REG21: u32 = 0;
pub const ECC_REG22: u32 = 16448;
pub const ECC_REG23: u32 = 32896;
pub const ECC_REG24: [u32; 5] = [0, 1392879, 1458376, 1523951, 1589448];
pub const ECC_REG25: [u32; 5] = [0, 0, 1, 3, 5];
pub const ECC_REG26: u32 = 12000000;
pub const ECC_REG27: u32 = 16000000;
pub const ECC_REG28: [u32; 5] = [0, 0, 23821, 4218125, 9657613];
pub const ECC_REG29: u32 = 0;
pub const ECC_REG30: u32 = 4;
pub const ECC_REG31: u32 = 8;
pub const ECC_REG32: u32 = 368;
pub const ECC_REG33: u32 = 0x80000000;
pub const ECC_REG34: u32 = 376;
pub const ECC_REG35: u32 = 892;
pub const ECC_REG36: u32 = 896;
pub const ECC_REG37: u32 = 900;
pub const ECC_REG38: u32 = 3;
pub const ECC_REG39: u32 = 3;
pub const ECC_REG40: u32 = 768;
pub const ECC_REG41: u32 = 832;
pub const ECC_REG42: u32 = 836;
pub const ECC_REG43: u32 = 840;
pub const ECC_REG44: u32 = 844;
pub const ECC_REG45: u32 = 848;
pub const ECC_REG46: u32 = 852;
pub const ECC_REG47: u32 = 856;
pub const ECC_REG48: u32 = 860;
pub const ECC_REG49: u32 = 0x80000000;
pub const ECC_REG50: u32 = 32768;
pub const ECC_REG51: u32 = 0x8000000;
pub const ECC_REG52: u32 = 0x10000000;
pub const ECC_REG53: u32 = 0x20000000;
pub const ECC_REG54: u32 = 0x40000000;
pub const ECC_REG55: u32 = 0x80000000;
pub const ECC_REG56: u32 = 1;
pub const ECC_REG57: u32 = 0x20000000;
pub const ECC_REG58: u32 = 0x20000000;
pub const ECC_REG59: u32 = 16;

pub const DS_REG1: u32 = 512;
pub const DS_REG2: u32 = 1024;
pub const DS_REG3: u32 = 2048;
pub const DS_REG4: u32 = 0x1000000;
pub const DS_REG5: u32 = 0x2000000;
pub const DS_REG6: u32 = 0x8000000;
pub const DS_REG7: u32 = 0x10000000;
pub const DS_REG8: u32 = 0x20000000;
pub const DS_REG9: u32 = 0x40000000;
pub const DS_REG10: u32 = 0x80000000;
pub const DS_REG11: u32 = 1;
pub const DS_REG12: u32 = 2;

pub const KEY_MANAGER_STATE_IDLE: u32 = 0;
pub const KEY_MANAGER_STATE_LOAD: u32 = 1;
pub const KEY_MANAGER_STATE_BUSY: u32 = 2;
pub const KEY_MANAGER_STATE_GAIN: u32 = 3;

pub const KEY_MANAGER_CRYPTO_ALGO_AES: u32 = 0;
pub const KEY_MANAGER_CRYPTO_ALGO_ECDH0: u32 = 1;
pub const KEY_MANAGER_CRYPTO_ALGO_ECDH1: u32 = 2;

pub const KEY_PURPOSE_ECDSA_192: u32 = 0;
pub const KEY_PURPOSE_ECDSA_256: u32 = 1;
pub const KEY_PURPOSE_FLASH_256_1: u32 = 2;
pub const KEY_PURPOSE_FLASH_256_2: u32 = 3;
pub const KEY_PURPOSE_FLASH_128: u32 = 4;
pub const KEY_PURPOSE_HMAC: u32 = 5;
pub const KEY_PURPOSE_DS: u32 = 6;
pub const KEY_PURPOSE_PSRAM_256_1: u32 = 7;
pub const KEY_PURPOSE_PSRAM_256_2: u32 = 8;
pub const KEY_PURPOSE_PSRAM_128: u32 = 9;
pub const KEY_PURPOSE_ECDSA_384_L: u32 = 10;
pub const KEY_PURPOSE_ECDSA_384_H: u32 = 11;

pub const XTS_STATE_IDLE: u32 = 0;
pub const XTS_STATE_BUSY: u32 = 1;
pub const XTS_STATE_DONE: u32 = 2;
pub const XTS_STATE_VISIBLE: u32 = 3;

fn read_u32_be(buf: &[u8; 128], offset: usize) -> u32 {
    (buf[offset] as u32) << 24 | (buf[offset + 1] as u32) << 16 | (buf[offset + 2] as u32) << 8 | buf[offset + 3] as u32
}

fn write_u32_be(buf: &mut [u8; 128], offset: usize, val: u32) {
    buf[offset] = (val >> 24) as u8;
    buf[offset + 1] = (val >> 16) as u8;
    buf[offset + 2] = (val >> 8) as u8;
    buf[offset + 3] = val as u8;
}

pub fn sha_mode_to_engine(cpu_val: u32) -> u32 {
    match cpu_val {
        SHA_PERIPHERAL_MODE_SHA1 => crate::crypto::sha::SHA1,
        SHA_PERIPHERAL_MODE_SHA256 => crate::crypto::sha::SHA256,
        SHA_PERIPHERAL_MODE_SHA384 => crate::crypto::sha::SHA384,
        SHA_PERIPHERAL_MODE_SHA512 => crate::crypto::sha::SHA512,
        _ => 0,
    }
}

pub struct ShaPeripheral {
    pub base: PeripheralBase,
    pub sha_buffer: [u8; 128],
    pub sha_busy: [u32; 4],
    pub sha_enabled: [u32; 4],
    pub sha1: ShaEngine,
    pub sha256: ShaEngine,
    pub sha512: ShaEngine,
}

impl ShaPeripheral {
    pub fn new(base_addr: u32, name: &'static str) -> Self {
        ShaPeripheral {
            base: PeripheralBase::new(base_addr, name),
            sha_buffer: [0u8; 128],
            sha_busy: [0u32; 4],
            sha_enabled: [0u32; 4],
            sha1: ShaEngine::new(),
            sha256: ShaEngine::new(),
            sha512: ShaEngine::new(),
        }
    }

    fn sha_complete(&mut self, cpu_val: u32) {
        let tmp_val = if cpu_val == SHA_PERIPHERAL_MODE_SHA384 { SHA_PERIPHERAL_MODE_SHA512 } else { cpu_val };
        self.sha_busy[tmp_val as usize] = 0;
    }

    fn sha_write(&mut self, cpu_val: u32, tmp_val: u32, idx_val: u32) {
        let clock_event = if tmp_val == SHA_PERIPHERAL_MODE_SHA384 { SHA_PERIPHERAL_MODE_SHA512 } else { tmp_val };
        if cpu_val != 0 && self.sha_busy[tmp_val as usize] == 0 {
            if idx_val == SHA_OPERATION_START {
                self.sha_enabled[clock_event as usize] = 1;
            }
            if self.sha_enabled[clock_event as usize] == 0 {
                let len = RSA_REG23[tmp_val as usize] as usize;
                self.sha_buffer[..len].fill(0);
                return;
            }
            self.sha_busy[clock_event as usize] = 1;
            let sha_engine = if tmp_val == SHA_PERIPHERAL_MODE_SHA1 {
                &mut self.sha1
            } else if tmp_val == SHA_PERIPHERAL_MODE_SHA256 {
                &mut self.sha256
            } else {
                &mut self.sha512
            };
            let simulation_clock = sha_mode_to_engine(tmp_val);
            if idx_val == SHA_OPERATION_START {
                sha_engine.initialize(simulation_clock);
            }
            if idx_val == SHA_OPERATION_FINAL {
                sha_engine.digest(simulation_clock, &mut self.sha_buffer);
            } else {
                sha_engine.update(simulation_clock, &self.sha_buffer);
            }
            self.sha_complete(tmp_val);
        }
    }

    pub fn read_u32(&mut self, _ctx: &mut CpuContext, addr: u32) -> u32 {
        let tmp_val = addr & 0xFFF;
        if tmp_val < RSA_REG7 {
            return read_u32_be(&self.sha_buffer, tmp_val as usize);
        }
        match tmp_val {
            RSA_REG10 => self.sha_busy[SHA_PERIPHERAL_MODE_SHA1 as usize],
            RSA_REG14 => self.sha_busy[SHA_PERIPHERAL_MODE_SHA256 as usize],
            RSA_REG18 => self.sha_busy[SHA_PERIPHERAL_MODE_SHA384 as usize],
            RSA_REG22 => self.sha_busy[SHA_PERIPHERAL_MODE_SHA512 as usize],
            _ => self.base.read_uint32(addr),
        }
    }

    pub fn write_u32(&mut self, _ctx: &mut CpuContext, addr: u32, val: u32) {
        self.base.write_uint32(addr, val);
        let idx_val = addr & 0xFFF;
        if idx_val < RSA_REG7 {
            write_u32_be(&mut self.sha_buffer, idx_val as usize, val);
            return;
        }
        match idx_val {
            RSA_REG7 => { self.sha_write(1 & val, SHA_PERIPHERAL_MODE_SHA1, SHA_OPERATION_START); }
            RSA_REG8 => { self.sha_write(1 & val, SHA_PERIPHERAL_MODE_SHA1, SHA_OPERATION_CONTINUE); }
            RSA_REG9 => { self.sha_write(1 & val, SHA_PERIPHERAL_MODE_SHA1, SHA_OPERATION_FINAL); }
            RSA_REG11 => { self.sha_write(1 & val, SHA_PERIPHERAL_MODE_SHA256, SHA_OPERATION_START); }
            RSA_REG12 => { self.sha_write(1 & val, SHA_PERIPHERAL_MODE_SHA256, SHA_OPERATION_CONTINUE); }
            RSA_REG13 => { self.sha_write(1 & val, SHA_PERIPHERAL_MODE_SHA256, SHA_OPERATION_FINAL); }
            RSA_REG15 => { self.sha_write(1 & val, SHA_PERIPHERAL_MODE_SHA384, SHA_OPERATION_START); }
            RSA_REG16 => { self.sha_write(1 & val, SHA_PERIPHERAL_MODE_SHA384, SHA_OPERATION_CONTINUE); }
            RSA_REG17 => { self.sha_write(1 & val, SHA_PERIPHERAL_MODE_SHA384, SHA_OPERATION_FINAL); }
            RSA_REG19 => { self.sha_write(1 & val, SHA_PERIPHERAL_MODE_SHA512, SHA_OPERATION_START); }
            RSA_REG20 => { self.sha_write(1 & val, SHA_PERIPHERAL_MODE_SHA512, SHA_OPERATION_CONTINUE); }
            RSA_REG21 => { self.sha_write(1 & val, SHA_PERIPHERAL_MODE_SHA512, SHA_OPERATION_FINAL); }
            _ => {}
        }
    }

    pub fn reset(&mut self) {
        self.base.zero_memory();
        self.sha_busy = [0u32; 4];
        self.sha_enabled = [0u32; 4];
    }
}

pub struct BitWriter<'a> {
    buffer: &'a mut [u8],
    msb_first: bool,
    index: u32,
}

impl<'a> BitWriter<'a> {
    pub fn new(buffer: &'a mut [u8], msb_first: bool) -> Self {
        BitWriter { buffer, msb_first, index: 0 }
    }

    pub fn write_value(&mut self, mut cpu_val: u32, tmp_val: u32) {
        let mut idx_val = self.index;
        let mut clock_event = tmp_val;
        if self.msb_first {
            let mut tmp_val2 = 255 & cpu_val;
            let mut simulation_clock = 0u32;
            while clock_event > 0 {
                let reg_val = 128 & tmp_val2;
                self.buffer[(idx_val >> 3) as usize] |= (reg_val >> (7 & idx_val)) as u8;
                tmp_val2 <<= 1;
                idx_val += 1;
                simulation_clock += 1;
                if simulation_clock == 7 {
                    simulation_clock = 0;
                    cpu_val >>= 8;
                    tmp_val2 = 255 & cpu_val;
                }
                clock_event -= 1;
            }
        } else {
            while clock_event > 0 {
                let tmp_val2 = 1 & cpu_val;
                self.buffer[(idx_val >> 3) as usize] |= (tmp_val2 << (7 - (7 & idx_val))) as u8;
                cpu_val >>= 1;
                idx_val += 1;
                clock_event -= 1;
            }
        }
        self.index = idx_val;
    }

    pub fn write_u8(&mut self, cpu_val: u8) {
        if self.index % 8 == 0 {
            self.buffer[(self.index >> 3) as usize] = cpu_val;
            self.index += 8;
        } else {
            self.write_value(cpu_val as u32, 8);
        }
    }

    pub fn byte_index(&self) -> u32 {
        self.index >> 3
    }
}

pub struct XtsAes {
    data_aes: AesCore,
    tweak_ecb: AesEcb,
}

impl XtsAes {
    pub fn new(key: &[u8]) -> Self {
        let half = key.len() / 2;
        let key1 = &key[..half];
        let key2 = &key[half..];
        let data_aes = AesCore::new(key1);
        let tweak_ecb = AesEcb::new(key2);
        XtsAes { data_aes, tweak_ecb }
    }

    pub fn encrypt(&self, input: &[u8], sector_offset: u32, output: &mut [u8]) {
        self.process(input, sector_offset, false, output);
    }

    pub fn decrypt(&self, input: &[u8], sector_offset: u32, output: &mut [u8]) {
        self.process(input, sector_offset, true, output);
    }

    fn process(&self, input: &[u8], sector_offset: u32, decrypt: bool, output: &mut [u8]) {
        let block_size = 128u32;
        let simulation_clock_offset = sector_offset % 128;
        let reg_val = simulation_clock_offset + input.len() as u32;
        let arg_val = (block_size - (reg_val % block_size)) % block_size;
        let register_type = (reg_val + arg_val) as usize;

        let mut cfg_val = [0u8; 2048];
        let mut h_val = [0u8; 2048];

        if register_type > 2048 {
            return;
        }

        cfg_val[simulation_clock_offset as usize..simulation_clock_offset as usize + input.len()].copy_from_slice(input);

        let mut off_val = sector_offset & !127;
        for chunk_start in (0..register_type).step_by(128) {
            let chunk_end = (chunk_start + 128).min(register_type);
            let mut tmp = [0u8; 128];
            let len = chunk_end - chunk_start;
            for i in 0..len {
                tmp[i] = cfg_val[chunk_start + len - 1 - i];
            }
            let tweak_val = Self::generate_tweak(off_val);
            let reg_val_block = self.process_block(&tmp, &tweak_val, decrypt);
            let mut reg_val_rev = [0u8; 128];
            for i in 0..128 {
                reg_val_rev[i] = reg_val_block[127 - i];
            }
            let copy_len = if chunk_start + 128 <= register_type { 128 } else { register_type - chunk_start };
            h_val[chunk_start..chunk_start + copy_len].copy_from_slice(&reg_val_rev[..copy_len]);
            off_val = off_val.wrapping_add(128);
        }

        let out_start = simulation_clock_offset as usize;
        let out_len = input.len();
        if out_start + out_len <= output.len() {
            output[..out_len].copy_from_slice(&h_val[out_start..out_start + out_len]);
        }
    }

    fn generate_tweak(sector_offset: u32) -> [u8; 16] {
        let mut tmp = [0u8; 16];
        let val = sector_offset & !127;
        tmp[..4].copy_from_slice(&val.to_le_bytes());
        tmp
    }

    fn process_block(&self, block: &[u8; 128], tweak: &[u8; 16], decrypt: bool) -> [u8; 128] {
        let mut clock_event = [0u8; 128];
        let mut simulation_clock = self.tweak_ecb.encrypt(tweak);
        for tmp_idx in (0..128).step_by(16) {
            let arg = &block[tmp_idx..tmp_idx + 16];
            let arg_array: &[u8; 16] = arg.try_into().unwrap();
            let register_type = Self::xor_bytes(arg_array, &simulation_clock);
            let cfg = if decrypt {
                self.data_aes.decrypt(&register_type)
            } else {
                self.data_aes.encrypt(&register_type)
            };
            let peripheral_type = Self::xor_bytes(&cfg, &simulation_clock);
            clock_event[tmp_idx..tmp_idx + 16].copy_from_slice(&peripheral_type);
            simulation_clock = Self::multiply_by_alpha(&simulation_clock);
        }
        clock_event
    }

    fn xor_bytes(a: &[u8; 16], b: &[u8; 16]) -> [u8; 16] {
        let mut out = [0u8; 16];
        for i in 0..16 {
            out[i] = a[i] ^ b[i];
        }
        out
    }

    fn multiply_by_alpha(input: &[u8; 16]) -> [u8; 16] {
        let mut tmp = [0u8; 16];
        let mut idx = 0u32;
        for clock_event in 0..16 {
            let simulation_clock = (input[clock_event] >> 7) & 1;
            tmp[clock_event] = ((input[clock_event] << 1) | idx as u8) & 255;
            idx = simulation_clock as u32;
        }
        if idx != 0 {
            tmp[0] ^= 135;
        }
        tmp
    }
}

const MAX_DERIVED_KEYS: usize = 16;

#[derive(Clone, Copy)]
struct KeyEntry {
    purpose: u32,
    key: [u8; 32],
    valid: bool,
}

pub trait KeyManager {
    fn get_derived_key(&self, purpose: u32, out: &mut [u8; 32]) -> bool;
}

pub struct KeyManagerPeripheral {
    pub base: PeripheralBase,
    pub irq: u32,
    pub clk: u32,
    pub int_raw: u32,
    pub int_enable: u32,
    pub static_reg: u32,
    pub lock: u32,
    pub conf: u32,
    pub state: u32,
    pub result: u32,
    pub key_vld: u32,
    pub huk_vld: u32,
    derived_keys: [KeyEntry; MAX_DERIVED_KEYS],
    derived_key_count: u32,
}

impl KeyManagerPeripheral {
    pub fn new(base_addr: u32, name: &'static str, irq: u32) -> Self {
        KeyManagerPeripheral {
            base: PeripheralBase::new(base_addr, name),
            irq,
            clk: 0,
            int_raw: 0,
            int_enable: 0,
            static_reg: 0,
            lock: 0,
            conf: 0,
            state: KEY_MANAGER_STATE_IDLE,
            result: 0,
            key_vld: 0,
            huk_vld: 1,
            derived_keys: [KeyEntry { purpose: 0, key: [0u8; 32], valid: false }; MAX_DERIVED_KEYS],
            derived_key_count: 0,
        }
    }

    fn int_status(&self) -> u32 {
        self.int_raw & self.int_enable
    }

    fn update_interrupts(&mut self, ctx: &mut CpuContext) {
        ctx.interrupt(self.irq, (self.int_raw & self.int_enable) != 0);
    }

    pub fn read_u32(&mut self, _ctx: &mut CpuContext, addr: u32) -> u32 {
        let offset = addr & 0xFFF;
        match offset {
            RSA_REG24 => self.clk,
            RSA_REG25 => self.int_raw,
            RSA_REG26 => self.int_status(),
            RSA_REG27 => self.int_enable,
            RSA_REG29 => self.static_reg,
            RSA_REG30 => self.lock,
            RSA_REG31 => self.conf,
            RSA_REG33 => {
                let cpu_val = self.state;
                if self.state == KEY_MANAGER_STATE_GAIN {
                    self.state = KEY_MANAGER_STATE_IDLE;
                }
                cpu_val
            }
            RSA_REG34 => self.result,
            RSA_REG35 => {
                if self.state == KEY_MANAGER_STATE_GAIN {
                    self.state = KEY_MANAGER_STATE_IDLE;
                }
                self.key_vld
            }
            RSA_REG36 => self.huk_vld,
            RSA_REG37 => 0x20230621,
            _ => self.base.read_uint32(addr),
        }
    }

    pub fn write_u32(&mut self, ctx: &mut CpuContext, addr: u32, val: u32) {
        self.base.write_uint32(addr, val);
        let offset = addr & 0xFFF;
        match offset {
            RSA_REG24 => {
                self.clk = val;
            }
            RSA_REG27 => {
                self.int_enable = val & RSA_REG44;
                self.update_interrupts(ctx);
            }
            RSA_REG28 => {
                self.int_raw &= !(val & RSA_REG44);
                self.update_interrupts(ctx);
            }
            RSA_REG29 => {
                let mut tmp_val = val;
                if self.lock & ECC_REG4 != 0 {
                    let cpu_val = self.lock & ECC_REG4;
                    tmp_val = (tmp_val & !cpu_val) | (self.static_reg & cpu_val);
                }
                if self.lock & ECC_REG5 != 0 {
                    let cpu_val = ECC_REG1 << RSA_REG45;
                    tmp_val = (tmp_val & !cpu_val) | (self.static_reg & cpu_val);
                }
                if self.lock & ECC_REG6 != 0 {
                    tmp_val = (tmp_val & !ECC_REG2) | (self.static_reg & ECC_REG2);
                }
                if self.lock & ECC_REG7 != 0 {
                    tmp_val = (tmp_val & !ECC_REG3) | (self.static_reg & ECC_REG3);
                }
                self.static_reg = tmp_val;
            }
            RSA_REG30 => {
                self.lock |= val;
            }
            RSA_REG31 => {
                self.conf = val;
            }
            RSA_REG32 => {
                if val & ECC_REG8 != 0 {
                    self.start_key_generation(ctx);
                }
                if val & ECC_REG9 != 0 {
                    self.continue_key_generation(ctx);
                }
            }
            _ => {
                self.base.write_uint32(addr, val);
            }
        }
    }

    fn start_key_generation(&mut self, ctx: &mut CpuContext) {
        self.state = KEY_MANAGER_STATE_LOAD;
        self.key_vld = 0;
        self.result = 0;
        self.int_raw |= RSA_REG41;
        self.update_interrupts(ctx);
    }

    fn continue_key_generation(&mut self, ctx: &mut CpuContext) {
        if self.state == KEY_MANAGER_STATE_LOAD {
            self.state = KEY_MANAGER_STATE_BUSY;
            self.int_raw |= RSA_REG42;
            self.update_interrupts(ctx);
            let cpu_val = (self.conf >> ECC_REG11) & ECC_REG12;
            self.derive_key(cpu_val);
            self.key_vld = self.key_vld_bit_for_purpose(cpu_val);
            self.result = 1;
            self.state = KEY_MANAGER_STATE_GAIN;
            self.int_raw |= RSA_REG43;
            self.update_interrupts(ctx);
        } else if self.state == KEY_MANAGER_STATE_GAIN {
            self.state = KEY_MANAGER_STATE_IDLE;
        }
    }

    fn derive_key(&mut self, cpu_val: u32) {
        let tmp_val = self.conf & ECC_REG10;
        if tmp_val == KEY_MANAGER_CRYPTO_ALGO_AES {
            self.derive_key_aes(cpu_val);
        } else if tmp_val == KEY_MANAGER_CRYPTO_ALGO_ECDH0 {
            self.derive_key_ecdh0(cpu_val);
        } else if tmp_val == KEY_MANAGER_CRYPTO_ALGO_ECDH1 {
            self.derive_key_ecdh1(cpu_val);
        }
    }

    fn recover_k2(&self) -> [u8; 32] {
        let cpu_aes = AesEcb::new(&self.base.memory[RSA_REG40 as usize..RSA_REG40 as usize + 32]);
        let mut tmp_val = [0u8; 64];
        for idx_val in (0..64).step_by(16) {
            let block: &[u8; 16] = self.base.memory[RSA_REG38 as usize + idx_val..RSA_REG38 as usize + idx_val + 16].try_into().unwrap();
            tmp_val[idx_val..idx_val + 16].copy_from_slice(&cpu_aes.decrypt(block));
        }
        let clock_event = AesEcb::new(&tmp_val[32..64]);
        let mut simulation_clock = [0u8; 32];
        for cpu_idx in (0..32).step_by(16) {
            let block: &[u8; 16] = tmp_val[cpu_idx..cpu_idx + 16].try_into().unwrap();
            simulation_clock[cpu_idx..cpu_idx + 16].copy_from_slice(&clock_event.decrypt(block));
        }
        simulation_clock
    }

    fn derive_key_aes(&mut self, cpu_val: u32) {
        if self.static_reg & ECC_REG2 == 0 {
            return;
        }
        let tmp_val = self.recover_k2();
        let idx_val = &self.base.memory[RSA_REG39 as usize..RSA_REG39 as usize + 32];
        let clock_event = AesEcb::new(&tmp_val);
        let mut simulation_clock = [0u8; 32];
        for cpu_idx in (0..32).step_by(16) {
            let block: &[u8; 16] = idx_val[cpu_idx..cpu_idx + 16].try_into().unwrap();
            simulation_clock[cpu_idx..cpu_idx + 16].copy_from_slice(&clock_event.decrypt(block));
        }
        if cpu_val == KEY_PURPOSE_FLASH_256_1
            || cpu_val == KEY_PURPOSE_FLASH_256_2
            || cpu_val == KEY_PURPOSE_FLASH_128
            || cpu_val == KEY_PURPOSE_PSRAM_256_1
            || cpu_val == KEY_PURPOSE_PSRAM_256_2
            || cpu_val == KEY_PURPOSE_PSRAM_128
            || cpu_val == KEY_PURPOSE_DS
        {
            simulation_clock.reverse();
        }
        self.set_derived_key(cpu_val, &simulation_clock);
    }

    fn derive_key_ecdh0(&mut self, cpu_val: u32) {
        let tmp_val = ECC_REG13;
        let curve = ecc_curve_params();
        let point = EccPoint { x: curve.p256.gx, y: curve.p256.gy };
        let idx_val = self.mul_p256(&point, tmp_val);
        bigint_to_bytes(&idx_val.x, &mut self.base.memory[RSA_REG38 as usize..RSA_REG38 as usize + 32], 32);
        bigint_to_bytes(&idx_val.y, &mut self.base.memory[RSA_REG38 as usize + 32..RSA_REG38 as usize + 64], 32);
        self.derive_ecdh_shared_key(cpu_val, tmp_val);
    }

    fn derive_key_ecdh1(&mut self, cpu_val: u32) {
        let tmp_val = bytes_to_bigint(&self.recover_k2());
        self.derive_ecdh_shared_key(cpu_val, tmp_val);
    }

    fn derive_ecdh_shared_key(&mut self, cpu_val: u32, tmp_val: U384) {
        let idx_val = EccPoint {
            x: bytes_to_bigint(&self.base.memory[RSA_REG39 as usize..RSA_REG39 as usize + 32]),
            y: bytes_to_bigint(&self.base.memory[RSA_REG39 as usize + 32..RSA_REG39 as usize + 64]),
        };
        let clock_event = self.mul_p256(&idx_val, tmp_val);
        let simulation_clock = cpu_val == KEY_PURPOSE_ECDSA_192
            || cpu_val == KEY_PURPOSE_ECDSA_256
            || cpu_val == KEY_PURPOSE_ECDSA_384_L
            || cpu_val == KEY_PURPOSE_ECDSA_384_H
            || cpu_val == KEY_PURPOSE_HMAC;
        let mut reg_val = [0u8; 32];
        bigint_to_bytes(&clock_event.x, &mut reg_val, 32);
        if !simulation_clock {
            reg_val.reverse();
        }
        self.set_derived_key(cpu_val, &reg_val);
    }

    fn mul_p256(&self, point: &EccPoint, scalar: U384) -> EccPoint {
        let curve = ecc_curve_params();
        jacobian_to_affine(
            &ecc_jacobian_scalar_mult(&jacobian_from_affine(point), scalar, &curve.p256),
            &curve.p256,
        )
    }

    fn set_derived_key(&mut self, purpose: u32, key: &[u8; 32]) {
        for i in 0..self.derived_key_count as usize {
            if self.derived_keys[i].purpose == purpose {
                self.derived_keys[i].key = *key;
                self.derived_keys[i].valid = true;
                return;
            }
        }
        if (self.derived_key_count as usize) < MAX_DERIVED_KEYS {
            let idx = self.derived_key_count as usize;
            self.derived_keys[idx] = KeyEntry { purpose, key: *key, valid: true };
            self.derived_key_count += 1;
        }
    }

    pub fn get_derived_key(&self, purpose: u32, out: &mut [u8; 32]) -> bool {
        for i in 0..self.derived_key_count as usize {
            if self.derived_keys[i].purpose == purpose && self.derived_keys[i].valid {
                *out = self.derived_keys[i].key;
                return true;
            }
        }
        false
    }

    pub fn read_key_block(&self, purpose: u32, len: usize) -> [u8; 48] {
        let mut key = [0u8; 48];
        if len <= 32 {
            let mut buf = [0u8; 32];
            if self.get_derived_key(purpose, &mut buf) {
                key[..len].copy_from_slice(&buf[..len]);
            }
        }
        key
    }

    fn key_vld_bit_for_purpose(&self, cpu_val: u32) -> u32 {
        match cpu_val {
            KEY_PURPOSE_ECDSA_192 => ECC_REG14,
            KEY_PURPOSE_ECDSA_256 => ECC_REG15,
            KEY_PURPOSE_FLASH_256_1 | KEY_PURPOSE_FLASH_256_2 | KEY_PURPOSE_FLASH_128 => ECC_REG16,
            KEY_PURPOSE_HMAC => ECC_REG17,
            KEY_PURPOSE_DS => ECC_REG18,
            KEY_PURPOSE_PSRAM_256_1 | KEY_PURPOSE_PSRAM_256_2 | KEY_PURPOSE_PSRAM_128 => ECC_REG19,
            KEY_PURPOSE_ECDSA_384_L | KEY_PURPOSE_ECDSA_384_H => ECC_REG20,
            _ => ECC_REG16,
        }
    }

    pub fn reset(&mut self) {
        self.base.reset();
        self.clk = 0;
        self.int_raw = 0;
        self.int_enable = 0;
        self.static_reg = 0;
        self.lock = 0;
        self.conf = 0;
        self.state = KEY_MANAGER_STATE_IDLE;
        self.result = 0;
        self.key_vld = 0;
        self.huk_vld = 1;
        self.derived_key_count = 0;
        for i in 0..MAX_DERIVED_KEYS {
            self.derived_keys[i] = KeyEntry { purpose: 0, key: [0u8; 32], valid: false };
        }
    }
}

pub struct DmaReader {
    input_is_buffer: bool,
    input_ptr: *const u8,
    input_len: u32,
    index: u32,
    active: bool,
}

impl DmaReader {
    pub fn new() -> Self {
        DmaReader {
            input_is_buffer: false,
            input_ptr: core::ptr::null(),
            input_len: 0,
            index: 0,
            active: false,
        }
    }

    pub fn start(&mut self, input: &[u8]) {
        self.index = 0;
        self.input_is_buffer = true;
        self.input_ptr = input.as_ptr();
        self.input_len = input.len() as u32;
        self.active = true;
    }

    pub fn reset(&mut self) {
        self.active = false;
        self.index = 0;
    }

    pub fn read_u8(&mut self) -> u8 {
        if !self.active {
            return 0;
        }
        if self.input_is_buffer {
            if self.index >= self.input_len {
                return 0;
            }
            let val = unsafe { *self.input_ptr.add(self.index as usize) };
            if self.index != 63 {
                self.index += 1;
            }
            val
        } else {
            0
        }
    }

    pub fn read_bytes(&mut self, buf: &mut [u8], tmp_val: u32) {
        if !self.active {
            for i in tmp_val as usize..buf.len() {
                buf[i] = 0;
            }
            return;
        }
        if self.input_is_buffer {
            for i in tmp_val as usize..buf.len() {
                buf[i] = self.read_u8();
            }
        }
    }
}

pub struct DmaWriter {
    target_is_buffer: bool,
    target_ptr: *mut u8,
    target_len: u32,
    length: u32,
    active: bool,
    index: u32,
}

impl DmaWriter {
    pub fn new() -> Self {
        DmaWriter {
            target_is_buffer: false,
            target_ptr: core::ptr::null_mut(),
            target_len: 0,
            length: 0,
            active: false,
            index: 0,
        }
    }

    pub fn start(&mut self, target: &mut [u8]) {
        self.index = 0;
        self.target_is_buffer = true;
        self.target_ptr = target.as_mut_ptr();
        self.target_len = target.len() as u32;
        self.length = 0;
        self.active = true;
    }

    pub fn reset(&mut self) {
        self.active = false;
        self.index = 0;
        self.length = 0;
    }

    pub fn write_u8(&mut self, cpu_val: u8) {
        if self.active {
            if self.target_is_buffer {
                if self.index < self.target_len {
                    unsafe { *self.target_ptr.add(self.index as usize) = cpu_val; }
                }
                if self.index < 63 {
                    self.index += 1;
                }
            }
            self.length += 1;
        }
    }

    pub fn write_bytes(&mut self, buf: &[u8], tmp_val: u32) {
        if self.active {
            if self.target_is_buffer {
                for i in tmp_val as usize..buf.len() {
                    if self.index < self.target_len {
                        unsafe { *self.target_ptr.add(self.index as usize) = buf[i]; }
                    }
                    if self.index < 63 {
                        self.index += 1;
                    }
                }
            }
            self.length += buf.len() as u32;
        }
    }
}

pub struct XtsEncryptionState {
    pub state: u32,
    pub line_size: u32,
    pub destination: u32,
    pub physical_address: u32,
}

impl XtsEncryptionState {
    pub fn new() -> Self {
        XtsEncryptionState {
            state: XTS_STATE_IDLE,
            line_size: 0,
            destination: 0,
            physical_address: 0,
        }
    }

    pub fn reset(&mut self) {
        self.state = XTS_STATE_IDLE;
        self.line_size = 0;
        self.destination = 0;
        self.physical_address = 0;
    }
}
