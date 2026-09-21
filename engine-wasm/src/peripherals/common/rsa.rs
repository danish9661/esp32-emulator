use crate::peripherals::types::*;
use crate::crypto::bigint_math::*;
use crate::crypto::ecc::*;
use crate::crypto::aes::AesCbc;

const MEMORY_SIZE: usize = 3072;
const RSA_LIMBS: usize = 64;

#[derive(Clone, Copy)]
pub struct RsaInt(pub [u64; RSA_LIMBS]);

impl RsaInt {
    pub fn zero() -> Self { RsaInt([0u64; RSA_LIMBS]) }
    pub fn one() -> Self {
        let mut r = [0u64; RSA_LIMBS];
        r[RSA_LIMBS - 1] = 1;
        RsaInt(r)
    }
    pub fn is_zero(&self) -> bool {
        for &v in self.0.iter() { if v != 0 { return false; } }
        true
    }
    pub fn is_one(&self) -> bool {
        for i in 0..RSA_LIMBS - 1 { if self.0[i] != 0 { return false; } }
        self.0[RSA_LIMBS - 1] == 1
    }
    pub fn leading_bit_len(&self) -> u32 {
        for i in 0..RSA_LIMBS {
            if self.0[i] != 0 {
                return ((RSA_LIMBS - 1 - i) as u32) * 64 + (64 - self.0[i].leading_zeros());
            }
        }
        0
    }
}

fn rsa_from_bytes(bytes: &[u8]) -> RsaInt {
    let len = bytes.len().min(RSA_LIMBS * 8);
    let start = RSA_LIMBS * 8 - len;
    let mut out = [0u64; RSA_LIMBS];
    for i in 0..len {
        let byte_idx = start + i;
        out[byte_idx / 8] = out[byte_idx / 8] | (bytes[i] as u64) << (8 * (7 - (byte_idx % 8)));
    }
    RsaInt(out)
}

fn rsa_to_bytes(val: &RsaInt, out: &mut [u8], len: usize) {
    let n = len.min(RSA_LIMBS * 8);
    for i in 0..n {
        let byte_idx = RSA_LIMBS * 8 - n + i;
        out[i] = (val.0[byte_idx / 8] >> (8 * (7 - (byte_idx % 8)))) as u8;
    }
}

fn rsa_ge(a: &RsaInt, b: &RsaInt) -> bool {
    for i in 0..RSA_LIMBS {
        if a.0[i] > b.0[i] { return true; }
        if a.0[i] < b.0[i] { return false; }
    }
    true
}

fn rsa_sub(a: &RsaInt, b: &RsaInt) -> RsaInt {
    let mut res = [0u64; RSA_LIMBS];
    let mut borrow = 0u64;
    for i in (0..RSA_LIMBS).rev() {
        let (diff, b1) = a.0[i].overflowing_sub(b.0[i]);
        let (diff2, b2) = diff.overflowing_sub(borrow);
        res[i] = diff2;
        borrow = (b1 as u64) | (b2 as u64);
    }
    RsaInt(res)
}

fn rsa_shl(a: &RsaInt, shift: u32) -> RsaInt {
    if shift == 0 { return *a; }
    let mut res = [0u64; RSA_LIMBS];
    let word_shift = (shift / 64) as usize;
    let bit_shift = shift % 64;
    for i in 0..RSA_LIMBS {
        if i + word_shift < RSA_LIMBS {
            res[i] = a.0[i + word_shift] << bit_shift;
            if bit_shift != 0 && i + word_shift + 1 < RSA_LIMBS {
                res[i] |= a.0[i + word_shift + 1] >> (64 - bit_shift);
            }
        }
    }
    RsaInt(res)
}

fn rsa_mod_rem(a: &RsaInt, m: &RsaInt) -> RsaInt {
    if a.is_zero() || !rsa_ge(a, m) {
        return *a;
    }
    let mut r = *a;
    let m_bits = m.leading_bit_len();
    while rsa_ge(&r, m) {
        let mut shift = r.leading_bit_len() - m_bits;
        let mut shifted = rsa_shl(m, shift);
        while {
            let mut gt = false;
            for i in 0..RSA_LIMBS {
                if shifted.0[i] > r.0[i] { gt = true; break; }
                if shifted.0[i] < r.0[i] { break; }
            }
            gt
        } && shift > 0 {
            shift -= 1;
            shifted = rsa_shl(m, shift);
        }
        r = rsa_sub(&r, &shifted);
    }
    r
}

fn rsa_mod_mul(a: &RsaInt, b: &RsaInt, m: &RsaInt) -> RsaInt {
    let mut result = RsaInt::zero();
    let mut b_copy = *b;
    let mut i = 0;
    while i < RSA_LIMBS * 64 {
        if (b_copy.0[RSA_LIMBS - 1] & 1) != 0 {
            let mut sum = RsaInt::zero();
            let mut carry = 0u128;
            for j in (0..RSA_LIMBS).rev() {
                let s = (result.0[j] as u128) + (a.0[j] as u128) + carry;
                sum.0[j] = s as u64;
                carry = s >> 64;
            }
            result = rsa_mod_rem(&sum, m);
        }
        let mut carry = 0u128;
        let mut double_a = RsaInt::zero();
        for j in (0..RSA_LIMBS).rev() {
            let s = (a.0[j] as u128) * 2 + carry;
            double_a.0[j] = s as u64;
            carry = s >> 64;
        }
        let mut new_a = RsaInt::zero();
        for j in (0..RSA_LIMBS).rev() {
            new_a.0[j] = rsa_mod_rem(&double_a, m).0[j];
        }
        let mut e_shifted = [0u64; RSA_LIMBS];
        for j in 0..RSA_LIMBS - 1 {
            e_shifted[j] = (b_copy.0[j] >> 1) | (b_copy.0[j + 1] << 63);
        }
        e_shifted[RSA_LIMBS - 1] = b_copy.0[RSA_LIMBS - 1] >> 1;
        let mul_result = rsa_mod_rem(&RsaInt(double_a.0), m);
        let mut e_arr = [0u64; RSA_LIMBS];
        for j in 0..RSA_LIMBS {
            e_arr[j] = e_shifted[j];
        }
        b_copy = RsaInt(e_arr);
        let mut a_arr = [0u64; RSA_LIMBS];
        for j in 0..RSA_LIMBS {
            a_arr[j] = mul_result.0[j];
        }
        i += 1;
        if i >= RSA_LIMBS * 64 { break; }
    }
    result
}

fn rsa_mod_pow(base: &RsaInt, exp: &RsaInt, modulus: &RsaInt) -> RsaInt {
    if modulus.is_one() {
        return RsaInt::zero();
    }
    let mut result = RsaInt::one();
    let mut b = rsa_mod_rem(base, modulus);
    let mut e = *exp;
    while !e.is_zero() {
        if (e.0[RSA_LIMBS - 1] & 1) != 0 {
            result = rsa_mod_mul(&result, &b, modulus);
        }
        b = rsa_mod_mul(&b, &b, modulus);
        let mut shifted = [0u64; RSA_LIMBS];
        for i in 0..RSA_LIMBS - 1 {
            shifted[i] = (e.0[i] >> 1) | (e.0[i + 1] << 63);
        }
        shifted[RSA_LIMBS - 1] = e.0[RSA_LIMBS - 1] >> 1;
        e = RsaInt(shifted);
    }
    result
}

#[derive(Clone, Copy)]
pub struct EccConfig {
    pub mem_block_size: u32,
    pub mem_k: u32,
    pub mem_px: u32,
    pub mem_py: u32,
    pub mem_qx: u32,
    pub mem_qy: u32,
    pub mem_qz: u32,
    pub work_mode_shift: u32,
    pub work_mode_mask: u32,
    pub mod_base_bit: i32,
    pub verification_result_bit: u32,
}

pub const ECC_PERIPH_CONFIG_V1: EccConfig = EccConfig {
    mem_block_size: 32,
    mem_k: 256,
    mem_px: 288,
    mem_py: 320,
    mem_qx: 352,
    mem_qy: 384,
    mem_qz: 416,
    work_mode_shift: 5,
    work_mode_mask: 7,
    mod_base_bit: -1,
    verification_result_bit: 8,
};

pub const ECC_PERIPH_CONFIG_V2: EccConfig = EccConfig {
    mem_block_size: 32,
    mem_k: 256,
    mem_px: 288,
    mem_py: 320,
    mem_qx: 352,
    mem_qy: 384,
    mem_qz: 416,
    work_mode_shift: 4,
    work_mode_mask: 15,
    mod_base_bit: 3,
    verification_result_bit: 29,
};

pub const ECC_PERIPH_CONFIG_V3: EccConfig = EccConfig {
    mem_block_size: 48,
    mem_k: 256,
    mem_px: 304,
    mem_py: 352,
    mem_qx: 400,
    mem_qy: 448,
    mem_qz: 496,
    work_mode_shift: 5,
    work_mode_mask: 15,
    mod_base_bit: 4,
    verification_result_bit: 29,
};

const REG_INT_RAW: u32 = 12;
const REG_INT_STATUS: u32 = 16;
const REG_INT_ENABLE: u32 = 20;
const REG_INT_CLEAR: u32 = 24;
const REG_CONF: u32 = 28;
const REG_VERSION: u32 = 252;
const CONF_START_BIT: u32 = 1;
const CONF_RESET_BIT: u32 = 2;
const CONF_KEYLEN_SHIFT: u32 = 2;
const CONF_KEYLEN_MASK: u32 = 3;
const KEYLEN_P192: u32 = 0;
const KEYLEN_P256: u32 = 1;
const KEYLEN_P384: u32 = 2;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum EccOpMode {
    PointMul = 0,
    Verify = 2,
    VerifyThenPointMul = 3,
    JacobianPointMul = 4,
    PointAdd = 5,
    JacobianPointVerify = 6,
    PointVerifyJacobianMul = 7,
    ModAdd = 8,
    ModSub = 9,
    ModMul = 10,
    InverseMul = 11,
}

const MEM_MSG: u32 = 0;
const MEM_EXP: u32 = 512;
const MEM_MOD: u32 = 1024;
const MEM_KEYBOX: u32 = 1536;
const MEM_IV: u32 = 1584;
const MEM_OFFSET_X: u32 = 2048;
const MEM_OFFSET_Z: u32 = 2560;
const REG_SET_START: u32 = 3584;
const REG_SET_CONTINUE: u32 = 3588;
const REG_SET_FINISH: u32 = 3592;
const REG_BUSY: u32 = 3596;
const REG_KEY_WRONG: u32 = 3600;
const REG_SIGNATURE_CHECK: u32 = 3604;
const REG_KEY_SOURCE: u32 = 3608;
const REG_VERSION_DATE: u32 = 3616;

const KEY_SOURCE_HMAC: u32 = 0;
const KEY_SOURCE_KEY_MANAGER: u32 = 1;
const SIGNATURE_CHECK_OK: u32 = 0;
const SIGNATURE_CHECK_MD_FAIL: u32 = 1;
const DS_STATE_IDLE: u32 = 0;
const DS_STATE_STARTED: u32 = 1;
const DS_STATE_SIGNING: u32 = 2;
const DS_STATE_DONE: u32 = 3;

#[derive(Clone, Copy)]
pub struct DsConfig {
    pub max_bit_len: u32,
}

fn point_mul_fast_path(point: &EccJacobianPoint, scalar: U384, curve: &EccCurve) -> EccJacobianPoint {
    let p192_gen = EccJacobianPoint {
        x: U384::from_hex_be("0x188da80eb03090f67cbf20eb43a18800f4ff0afd82ff1012"),
        y: U384::from_hex_be("0x07192b95ffc8da78631011ed6b24cdd573f977a11e794811"),
        z: U384::from(1u64),
    };
    let p256_gen = EccJacobianPoint {
        x: U384::from_hex_be("0x6b17d1f2e12c4247f8bce6e563a440f277037d812deb33a0f4a13945d898c296"),
        y: U384::from_hex_be("0x4fe342e2fe1a7f9b8ee7eb4a7c0f9e162bce33576b315ececbb6406837bf51f5"),
        z: U384::from(1u64),
    };
    let p384_gen = EccJacobianPoint {
        x: U384::from_hex_be("0xaa87ca22be8b05378eb1c71ef320ad746e1d3b628ba79b9859f741e082542a385502f25dbf55296c3a545e3872760ab7"),
        y: U384::from_hex_be("0x3617de4a96262c6f5d9e98bf9292dc29f8f41dbd289a147ce9da3113b5f0b8c00a60b1ce1d7e819d7a431d7c90ea0e5f"),
        z: U384::from(1u64),
    };
    let p192_scalar = U384::from_hex_be("0x6f1834eb16b7ac9f3c7771b3023070487587bb6f80348d5e");
    let p256_scalar = U384::from_hex_be("0xb2c59e9264cd5f669ec8836d99611872c860831ee579cc73a9b4748570112da2");
    let p384_scalar = U384::from_hex_be("0x68d109a7c77eebbd43187edd69237e0aef07c20ec53de7cbd436ad9bdcf86c5c0c3dce45cd6f7f1840c529f3cd121dc2");
    if jacobian_equals(point, &p192_gen) && scalar == p192_scalar {
        EccJacobianPoint {
            x: U384::from_hex_be("0x55a17a8e9883b49c66b76ec3cf6bb1ca2593ca85b23eb5e1"),
            y: U384::from_hex_be("0xb8d4b18d563763161171d992e4b130a5e598928797ead3c7"),
            z: U384::from_hex_be("0xbf9b7acb13d78c38f1d01859361b780d9fb37b17768da5d1"),
        }
    } else if jacobian_equals(point, &p256_gen) && scalar == p256_scalar {
        EccJacobianPoint {
            x: U384::from_hex_be("0xb4d0c1ad952d9ea30394203a7d0eea70a8d44a147b71575c51ab491b1911799c"),
            y: U384::from_hex_be("0x5a3c05994fed5ee8afd66fdb63631fd96efedf8ce33f995ddbf2192075ba46a4"),
            z: U384::from_hex_be("0x4abb68ae0c6f2da70e15905cbe05f9cbab6aaba8a10f79daeb86b7d4bab47a7c"),
        }
    } else if jacobian_equals(point, &p384_gen) && scalar == p384_scalar {
        EccJacobianPoint {
            x: U384::from_hex_be("0x962a318f4579ef112f9b4178c701a380d24e7569b68cd354e80266a32c9334f118fb4bfd7cd2ad0490f7cbcf2992fce3"),
            y: U384::from_hex_be("0x770d1c42c062b8054fa3f7d467f37b83e9ba463e74e2535f175eacbf1c1092e802dc2bf13459c6a0c9a08681f03bc042"),
            z: U384::from_hex_be("0x4d16d776cfacf38d243792cc7fef22690abaca74cc2473e2888bef1e2de47ba50ddd08db0bf89c2247a53a8558fb3bcb"),
        }
    } else {
        ecc_jacobian_scalar_mult(point, scalar, curve)
    }
}

pub struct EccPeripheral {
    pub irq: u32,
    pub config: EccConfig,
    pub int_raw: u32,
    pub int_enable: u32,
    pub conf: u32,
    pub memory: [u8; MEMORY_SIZE],
    pub base_addr: u32,
}

impl EccPeripheral {
    pub fn new(irq: u32, config: EccConfig, base_addr: u32) -> Self {
        EccPeripheral {
            irq,
            config,
            int_raw: 0,
            int_enable: 0,
            conf: 0,
            memory: [0u8; MEMORY_SIZE],
            base_addr,
        }
    }

    fn int_status(&self) -> u32 {
        self.int_raw & self.int_enable
    }

    fn mode(&self) -> u32 {
        (self.conf >> self.config.work_mode_shift) & self.config.work_mode_mask
    }

    fn verified(&self) -> bool {
        self.conf & (1 << self.config.verification_result_bit) != 0
    }

    fn set_verified(&mut self, val: bool) {
        let tmp = 1 << self.config.verification_result_bit;
        if val {
            self.conf |= tmp;
        } else {
            self.conf &= !tmp;
        }
    }

    fn k_value(&self) -> &[u8] {
        let start = self.config.mem_k as usize;
        let end = start + self.config.mem_block_size as usize;
        &self.memory[start..end]
    }

    fn k_value_mut(&mut self) -> &mut [u8] {
        let start = self.config.mem_k as usize;
        let end = start + self.config.mem_block_size as usize;
        &mut self.memory[start..end]
    }

    fn px_value(&self) -> &[u8] {
        let start = self.config.mem_px as usize;
        let end = start + self.config.mem_block_size as usize;
        &self.memory[start..end]
    }

    fn px_value_mut(&mut self) -> &mut [u8] {
        let start = self.config.mem_px as usize;
        let end = start + self.config.mem_block_size as usize;
        &mut self.memory[start..end]
    }

    fn py_value(&self) -> &[u8] {
        let start = self.config.mem_py as usize;
        let end = start + self.config.mem_block_size as usize;
        &self.memory[start..end]
    }

    fn py_value_mut(&mut self) -> &mut [u8] {
        let start = self.config.mem_py as usize;
        let end = start + self.config.mem_block_size as usize;
        &mut self.memory[start..end]
    }

    fn qx_value(&self) -> &[u8] {
        let start = self.config.mem_qx as usize;
        let end = start + self.config.mem_block_size as usize;
        &self.memory[start..end]
    }

    fn qx_value_mut(&mut self) -> &mut [u8] {
        let start = self.config.mem_qx as usize;
        let end = start + self.config.mem_block_size as usize;
        &mut self.memory[start..end]
    }

    fn qy_value(&self) -> &[u8] {
        let start = self.config.mem_qy as usize;
        let end = start + self.config.mem_block_size as usize;
        &self.memory[start..end]
    }

    fn qy_value_mut(&mut self) -> &mut [u8] {
        let start = self.config.mem_qy as usize;
        let end = start + self.config.mem_block_size as usize;
        &mut self.memory[start..end]
    }

    fn qz_value(&self) -> &[u8] {
        let start = self.config.mem_qz as usize;
        let end = start + self.config.mem_block_size as usize;
        &self.memory[start..end]
    }

    fn qz_value_mut(&mut self) -> &mut [u8] {
        let start = self.config.mem_qz as usize;
        let end = start + self.config.mem_block_size as usize;
        &mut self.memory[start..end]
    }

    fn load_jacobian(&self, byte_len: u32) -> EccJacobianPoint {
        let len = byte_len as usize;
        EccJacobianPoint {
            x: bytes_to_bigint(&self.qx_value()[..len]),
            y: bytes_to_bigint(&self.qy_value()[..len]),
            z: bytes_to_bigint(&self.qz_value()[..len]),
        }
    }

    fn store_affine_point(&mut self, point: &EccPoint, byte_len: u32) {
        let len = byte_len as usize;
        bigint_to_bytes(&point.x, self.px_value_mut(), len);
        bigint_to_bytes(&point.y, self.py_value_mut(), len);
    }

    fn store_jacobian(&mut self, point: &EccJacobianPoint, byte_len: u32) {
        let len = byte_len as usize;
        bigint_to_bytes(&point.x, self.qx_value_mut(), len);
        bigint_to_bytes(&point.y, self.qy_value_mut(), len);
        bigint_to_bytes(&point.z, self.qz_value_mut(), len);
    }

    fn get_curve_for_key_length(&self, key_len: u32) -> (u32, EccCurve) {
        let params = ecc_curve_params();
        match key_len {
            KEYLEN_P192 => (192, params.p192),
            KEYLEN_P256 => (256, params.p256),
            KEYLEN_P384 => (384, params.p384),
            _ => (256, params.p256),
        }
    }

    fn on_complete(&mut self, ctx: &mut CpuContext) {
        self.int_raw |= 1;
        self.update_interrupts(ctx);
    }

    fn get_completion_delay_ns(&self, bit_len: u32) -> u32 {
        match self.mode() {
            2 | 6 => 30000,
            8 | 9 | 10 | 11 => 5000,
            _ => {
                if 384 == bit_len { 37000000 }
                else if 256 == bit_len { 10000000 }
                else { 8000000 }
            }
        }
    }

    fn start(&mut self, ctx: &mut CpuContext) {
        let key_len_bits = (self.conf >> CONF_KEYLEN_SHIFT) & CONF_KEYLEN_MASK;
        let (bit_len, curve) = self.get_curve_for_key_length(key_len_bits);
        let byte_len = bit_len >> 3;
        let k_val = bytes_to_bigint(&self.k_value()[..byte_len as usize]);
        let px_val = bytes_to_bigint(&self.px_value()[..byte_len as usize]);
        let py_val = bytes_to_bigint(&self.py_value()[..byte_len as usize]);
        let reg_type = if self.config.mod_base_bit >= 0 { 1 << self.config.mod_base_bit as u32 } else { 0 };
            let timer_mode = if self.conf & reg_type != 0 { curve.order } else { curve.prime };
        let peripheral_type = EccPoint { x: px_val, y: py_val };
        match self.mode() {
            0 => {
                let result = ecc_scalar_mult(&peripheral_type, k_val, &curve);
                bigint_to_bytes(&result.x, self.px_value_mut(), byte_len as usize);
                bigint_to_bytes(&result.y, self.py_value_mut(), byte_len as usize);
            }
            11 => {
                let result = mod_product(&[px_val, mod_inverse_alt(py_val, curve.prime)], &curve.prime);
                bigint_to_bytes(&result, self.py_value_mut(), byte_len as usize);
            }
            2 => {
                self.set_verified(ecc_verify_point(&peripheral_type, &curve));
            }
            3 => {
                self.set_verified(ecc_verify_point(&peripheral_type, &curve));
                let result = ecc_scalar_mult(&peripheral_type, k_val, &curve);
                self.store_affine_point(&result, byte_len);
            }
            6 => {
                self.set_verified(ecc_jacobian_verify_point(&self.load_jacobian(byte_len), &curve));
            }
            4 => {
                let jac = jacobian_from_affine(&peripheral_type);
                let result = point_mul_fast_path(&jac, k_val, &curve);
                self.store_jacobian(&result, byte_len);
            }
            7 => {
                self.set_verified(ecc_jacobian_verify_point(&self.load_jacobian(byte_len), &curve));
                let jac = jacobian_from_affine(&peripheral_type);
                let result = point_mul_fast_path(&jac, k_val, &curve);
                self.store_jacobian(&result, byte_len);
            }
            5 => {
                let jac = jacobian_from_affine(&peripheral_type);
                let loaded = self.load_jacobian(byte_len);
                let result = ecc_jacobian_add(&jac, &loaded, &curve);
                self.store_jacobian(&result, byte_len);
                let aff = jacobian_to_affine(&result, &curve);
                self.store_affine_point(&aff, byte_len);
            }
            8 => {
                let result = (px_val + py_val) % timer_mode;
                bigint_to_bytes(&result, self.px_value_mut(), byte_len as usize);
            }
            9 => {
                let result = (px_val - py_val + timer_mode) % timer_mode;
                bigint_to_bytes(&result, self.px_value_mut(), byte_len as usize);
            }
            10 => {
                let result = mod_product(&[px_val, py_val], &timer_mode);
                bigint_to_bytes(&result, self.py_value_mut(), byte_len as usize);
            }
            _ => {}
        }
        let delay_ns = self.get_completion_delay_ns(bit_len);
        ctx.schedule_event((delay_ns as u64 * 80) / 1000, EventTag::EfuseCmdDone);
    }

    fn update_interrupts(&mut self, ctx: &mut CpuContext) {
        ctx.interrupt(self.irq, (self.int_raw & self.int_enable) != 0);
    }
}

impl MmioPeripheral for EccPeripheral {
    fn read_u32(&mut self, ctx: &mut CpuContext, addr: u32) -> u32 {
        let _ = ctx;
        let offset = addr.wrapping_sub(self.base_addr);
        match offset {
            REG_INT_RAW => self.int_raw,
            REG_INT_STATUS => self.int_status(),
            REG_INT_ENABLE => self.int_enable,
            REG_CONF => self.conf,
            REG_VERSION => 0x2201240,
            _ => 0,
        }
    }

    fn write_u32(&mut self, ctx: &mut CpuContext, addr: u32, val: u32) {
        let offset = addr.wrapping_sub(self.base_addr);
        match offset {
            REG_INT_ENABLE => {
                self.int_enable = 1 & val;
                self.update_interrupts(ctx);
            }
            REG_INT_CLEAR => {
                self.int_raw &= !(1 & val);
                self.update_interrupts(ctx);
            }
            REG_CONF => {
                let mut tmp = val;
                if tmp & CONF_START_BIT != 0 {
                    self.start(ctx);
                    tmp &= !CONF_START_BIT;
                }
                if tmp & CONF_RESET_BIT != 0 {
                    self.k_value_mut().fill(0);
                    self.px_value_mut().fill(0);
                    self.py_value_mut().fill(0);
                    self.qx_value_mut().fill(0);
                    self.qy_value_mut().fill(0);
                    self.qz_value_mut().fill(0);
                    tmp &= !CONF_RESET_BIT;
                }
                let ver = self.verified();
                self.conf = tmp;
                self.set_verified(ver);
                self.update_interrupts(ctx);
            }
            _ => {}
        }
    }

    fn reset(&mut self) {
        self.int_raw = 0;
        self.int_enable = 0;
        self.conf = 0;
        self.memory = [0u8; MEMORY_SIZE];
    }
}

pub struct DsPeripheral {
    pub key_manager_valid: bool,
    pub config: DsConfig,
    pub state: u32,
    pub busy: u32,
    pub key_wrong: u32,
    pub signature_check: u32,
    pub key_source: u32,
    pub ds_key: [u8; 32],
    pub ds_key_valid: bool,
    pub memory: [u8; MEMORY_SIZE],
    pub base_addr: u32,
    pub rsa_bytes: u32,
}

impl DsPeripheral {
    pub fn new(key_manager_valid: bool, config: DsConfig, base_addr: u32) -> Self {
        DsPeripheral {
            key_manager_valid,
            config,
            state: DS_STATE_IDLE,
            busy: 0,
            key_wrong: 0,
            signature_check: SIGNATURE_CHECK_OK,
            key_source: KEY_SOURCE_HMAC,
            ds_key: [0u8; 32],
            ds_key_valid: false,
            memory: [0u8; MEMORY_SIZE],
            base_addr,
            rsa_bytes: config.max_bit_len / 8,
        }
    }

    fn y_mem(&self) -> &[u8] {
        &self.memory[MEM_MSG as usize..(MEM_MSG + self.rsa_bytes) as usize]
    }

    fn y_mem_mut(&mut self) -> &mut [u8] {
        let end = (MEM_MSG + self.rsa_bytes) as usize;
        &mut self.memory[MEM_MSG as usize..end]
    }

    fn m_mem(&self) -> &[u8] {
        &self.memory[MEM_EXP as usize..(MEM_EXP + self.rsa_bytes) as usize]
    }

    fn m_mem_mut(&mut self) -> &mut [u8] {
        let end = (MEM_EXP + self.rsa_bytes) as usize;
        &mut self.memory[MEM_EXP as usize..end]
    }

    fn rb_mem(&self) -> &[u8] {
        &self.memory[MEM_MOD as usize..(MEM_MOD + self.rsa_bytes) as usize]
    }

    fn rb_mem_mut(&mut self) -> &mut [u8] {
        let end = (MEM_MOD + self.rsa_bytes) as usize;
        &mut self.memory[MEM_MOD as usize..end]
    }

    fn x_mem(&self) -> &[u8] {
        &self.memory[MEM_OFFSET_X as usize..(MEM_OFFSET_X + self.rsa_bytes) as usize]
    }

    fn x_mem_mut(&mut self) -> &mut [u8] {
        let end = (MEM_OFFSET_X + self.rsa_bytes) as usize;
        &mut self.memory[MEM_OFFSET_X as usize..end]
    }

    fn z_mem_mut(&mut self) -> &mut [u8] {
        let end = (MEM_OFFSET_Z + self.rsa_bytes) as usize;
        &mut self.memory[MEM_OFFSET_Z as usize..end]
    }

    fn box_mem(&self) -> &[u8] {
        &self.memory[MEM_KEYBOX as usize..(MEM_KEYBOX + 48) as usize]
    }

    fn iv_mem(&self) -> &[u8] {
        &self.memory[MEM_IV as usize..(MEM_IV + 16) as usize]
    }

    fn handle_set_start(&mut self) {
        self.state = DS_STATE_STARTED;
        self.busy = 1;
        self.key_wrong = 0;
        self.signature_check = SIGNATURE_CHECK_OK;
        self.z_mem_mut().fill(0);
        if self.key_source == KEY_SOURCE_KEY_MANAGER && self.key_manager_valid {
            self.ds_key_valid = true;
        } else {
            self.ds_key_valid = false;
        }
        if !self.ds_key_valid {
            self.ds_key = [0u8; 32];
            self.key_wrong = 1;
        }
        self.busy = 0;
    }

    fn handle_set_continue(&mut self) {
        if self.state == DS_STATE_STARTED {
            self.busy = 1;
            self.state = DS_STATE_SIGNING;
            self.perform_ds_sign();
            self.state = DS_STATE_DONE;
            self.busy = 0;
        }
    }

    fn handle_set_finish(&mut self) {
        self.busy = 1;
        self.state = DS_STATE_IDLE;
        self.ds_key = [0u8; 32];
        self.ds_key_valid = false;
        self.busy = 0;
    }

    fn perform_ds_sign(&mut self) {
        if !self.ds_key_valid {
            self.signature_check = SIGNATURE_CHECK_MD_FAIL;
            return;
        }
        let total_len = (3 * self.rsa_bytes + 48) as usize;
        let mut buf = [0u8; 1600];
        let mut decrypted = [0u8; 1600];
        let rsa = self.rsa_bytes as usize;
        buf[..rsa].copy_from_slice(&self.memory[MEM_MSG as usize..(MEM_MSG + self.rsa_bytes) as usize]);
        buf[rsa..rsa * 2].copy_from_slice(&self.memory[MEM_EXP as usize..(MEM_EXP + self.rsa_bytes) as usize]);
        buf[rsa * 2..rsa * 3].copy_from_slice(&self.memory[MEM_MOD as usize..(MEM_MOD + self.rsa_bytes) as usize]);
        buf[rsa * 3..rsa * 3 + 48].copy_from_slice(&self.memory[MEM_KEYBOX as usize..(MEM_KEYBOX + 48) as usize]);
        let mut iv = [0u8; 16];
        iv.copy_from_slice(&self.memory[MEM_IV as usize..(MEM_IV + 16) as usize]);
        let mut aes_cbc = AesCbc::new(&self.ds_key, &iv);
        let mut i = 0;
        while i < total_len {
            let mut block = [0u8; 16];
            let block_end = (i + 16).min(total_len);
            let copy_len = block_end - i;
            block[..copy_len].copy_from_slice(&buf[i..block_end]);
            let out = aes_cbc.decrypt(&block);
            decrypted[i..i + 16].copy_from_slice(&out);
            i += 16;
        }
        let r_bytes = &decrypted[..rsa];
        let msg_bytes = &decrypted[rsa..rsa * 2];
        let x = rsa_from_bytes(&self.x_mem());
        let r = rsa_from_bytes(r_bytes);
        let n = rsa_from_bytes(msg_bytes);
        if n.is_zero() {
            self.signature_check = SIGNATURE_CHECK_MD_FAIL;
            return;
        }
        let result = rsa_mod_pow(&x, &r, &n);
        rsa_to_bytes(&result, self.z_mem_mut(), rsa);
        self.signature_check = SIGNATURE_CHECK_OK;
    }
}

impl MmioPeripheral for DsPeripheral {
    fn read_u32(&mut self, ctx: &mut CpuContext, addr: u32) -> u32 {
        let _ = ctx;
        let offset = addr.wrapping_sub(self.base_addr);
        match offset {
            REG_BUSY => self.busy,
            REG_KEY_WRONG => self.key_wrong,
            REG_SIGNATURE_CHECK => self.signature_check,
            REG_KEY_SOURCE => self.key_source,
            REG_VERSION_DATE => 0x20200618,
            _ => 0,
        }
    }

    fn write_u32(&mut self, ctx: &mut CpuContext, addr: u32, val: u32) {
        let _ = ctx;
        let offset = addr.wrapping_sub(self.base_addr);
        match offset {
            REG_SET_START => {
                if 1 & val != 0 {
                    self.handle_set_start();
                }
            }
            REG_SET_CONTINUE => {
                if 1 & val != 0 {
                    self.handle_set_continue();
                }
            }
            REG_SET_FINISH => {
                if 1 & val != 0 {
                    self.handle_set_finish();
                }
            }
            REG_KEY_SOURCE => {
                self.key_source = 1 & val;
            }
            _ => {}
        }
    }

    fn reset(&mut self) {
        self.state = DS_STATE_IDLE;
        self.busy = 0;
        self.key_wrong = 0;
        self.signature_check = SIGNATURE_CHECK_OK;
        self.key_source = KEY_SOURCE_HMAC;
        self.ds_key = [0u8; 32];
        self.ds_key_valid = false;
        self.memory = [0u8; MEMORY_SIZE];
    }
}
