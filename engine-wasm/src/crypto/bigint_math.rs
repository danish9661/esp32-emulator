use core::cmp::Ordering;

#[derive(Clone, Copy, PartialEq, Eq)]
pub struct U384(pub [u64; 6]);

impl U384 {
    pub fn from_hex_be(s: &str) -> Self {
        let s = s.strip_prefix("0x").unwrap_or(s);
        let hex_len = s.len();
        let bytes_len = hex_len / 2;
        let mut bytes = [0u8; 48];
        let start = 48 - bytes_len;
        for i in 0..bytes_len {
            bytes[start + i] = u8::from_str_radix(&s[i * 2..i * 2 + 2], 16).unwrap();
        }
        let mut out = [0u64; 6];
        for i in 0..6 {
            out[i] = u64::from_be_bytes([
                bytes[i * 8], bytes[i * 8 + 1], bytes[i * 8 + 2], bytes[i * 8 + 3],
                bytes[i * 8 + 4], bytes[i * 8 + 5], bytes[i * 8 + 6], bytes[i * 8 + 7],
            ]);
        }
        U384(out)
    }

    pub fn is_zero(&self) -> bool {
        self.0 == [0; 6]
    }

    pub fn is_one(&self) -> bool {
        self.0 == [0, 0, 0, 0, 0, 1]
    }

    pub fn leading_bit_len(&self) -> u32 {
        for (i, &v) in self.0.iter().enumerate() {
            if v != 0 {
                return (5 - i as u32) * 64 + (64 - v.leading_zeros());
            }
        }
        0
    }
}

impl From<u64> for U384 {
    fn from(v: u64) -> Self {
        U384([0, 0, 0, 0, 0, v])
    }
}

impl From<u32> for U384 {
    fn from(v: u32) -> Self {
        U384([0, 0, 0, 0, 0, v as u64])
    }
}

impl core::ops::Add for U384 {
    type Output = U384;
    fn add(self, rhs: U384) -> U384 {
        let mut res = [0u64; 6];
        let mut carry = 0u64;
        for i in (0..6).rev() {
            let (sum, c1) = self.0[i].overflowing_add(rhs.0[i]);
            let (sum2, c2) = sum.overflowing_add(carry);
            res[i] = sum2;
            carry = (c1 as u64) + (c2 as u64);
        }
        U384(res)
    }
}

impl core::ops::Sub for U384 {
    type Output = U384;
    fn sub(self, rhs: U384) -> U384 {
        let mut res = [0u64; 6];
        let mut borrow = 0u64;
        for i in (0..6).rev() {
            let (diff, b1) = self.0[i].overflowing_sub(rhs.0[i]);
            let (diff2, b2) = diff.overflowing_sub(borrow);
            res[i] = diff2;
            borrow = (b1 as u64) | (b2 as u64);
        }
        U384(res)
    }
}

impl core::ops::Mul for U384 {
    type Output = U384;
    fn mul(self, rhs: U384) -> U384 {
        let mut res = [0u64; 12];
        for i in 0..6 {
            let mut carry = 0u128;
            for j in 0..6 {
                let prod = (self.0[5 - i] as u128) * (rhs.0[5 - j] as u128)
                    + (res[11 - (i + j)] as u128)
                    + carry;
                res[11 - (i + j)] = prod as u64;
                carry = prod >> 64;
            }
            res[11 - (i + 6)] = carry as u64;
        }
        U384([res[6], res[7], res[8], res[9], res[10], res[11]])
    }
}

impl core::ops::Rem for U384 {
    type Output = U384;
    fn rem(self, rhs: U384) -> U384 {
        mod_rem(&self, &rhs)
    }
}

impl core::ops::Div for U384 {
    type Output = U384;
    fn div(self, rhs: U384) -> U384 {
        if rhs.is_zero() { return U384([0; 6]); }
        if self < rhs { return U384([0; 6]); }
        let mut quotient = U384([0; 6]);
        let mut remainder = self;
        let m_bits = rhs.leading_bit_len();
        while remainder >= rhs {
            let mut shift = remainder.leading_bit_len() - m_bits;
            let mut shifted = rhs << shift;
            while shifted > remainder && shift > 0 {
                shift -= 1;
                shifted = rhs << shift;
            }
            let q_bit = U384::from(1u64) << shift;
            quotient = quotient + q_bit;
            remainder = remainder - shifted;
        }
        quotient
    }
}

impl core::ops::Shl<u32> for U384 {
    type Output = U384;
    fn shl(self, rhs: u32) -> U384 {
        if rhs == 0 {
            return self;
        }
        let mut res = [0u64; 6];
        let word_shift = (rhs / 64) as usize;
        let bit_shift = rhs % 64;
        for i in 0..6 {
            if i + word_shift < 6 {
                res[i] = self.0[i + word_shift] << bit_shift;
                if bit_shift != 0 && i + word_shift + 1 < 6 {
                    res[i] |= self.0[i + word_shift + 1] >> (64 - bit_shift);
                }
            }
        }
        U384(res)
    }
}

impl core::ops::BitAnd for U384 {
    type Output = U384;
    fn bitand(self, rhs: U384) -> U384 {
        let mut res = [0u64; 6];
        for i in 0..6 {
            res[i] = self.0[i] & rhs.0[i];
        }
        U384(res)
    }
}

impl PartialOrd for U384 {
    fn partial_cmp(&self, other: &U384) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for U384 {
    fn cmp(&self, other: &U384) -> Ordering {
        for i in 0..6 {
            match self.0[i].cmp(&other.0[i]) {
                Ordering::Less => return Ordering::Less,
                Ordering::Greater => return Ordering::Greater,
                Ordering::Equal => {}
            }
        }
        Ordering::Equal
    }
}

pub fn mod_rem(a: &U384, m: &U384) -> U384 {
    if a.0 == [0; 6] || *a < *m {
        return *a;
    }
    let mut r = *a;
    let m_bits = m.leading_bit_len();
    while r >= *m {
        let mut shift = r.leading_bit_len() - m_bits;
        let mut shifted = *m << shift;
        while shifted > r && shift > 0 {
            shift -= 1;
            shifted = *m << shift;
        }
        r = r - shifted;
    }
    r
}

pub fn mod_inverse(a: U384, m: U384) -> U384 {
    mod_inverse_alt(a, m)
}

pub fn mod_inverse_alt(a: U384, m: U384) -> U384 {
    let a_norm = mod_rem(&a, &m);
    if a_norm.is_zero() || m < U384::from(2u64) {
        return U384::from(0u64);
    }
    let mut old_r = m;
    let mut r = a_norm;
    let mut old_s = U384::from(0u64);
    let mut s = U384::from(1u64);
    while !r.is_zero() {
        let quotient = if old_r >= r {
            let shift = old_r.leading_bit_len() - r.leading_bit_len();
            if shift > 0 {
                m << 1
            } else {
                let q_val = if r.0[5] != 0 {
                    old_r.0[5] / r.0[5]
                } else if r.0[4] != 0 {
                    old_r.0[5] / r.0[4]
                } else {
                    0
                };
                if q_val > 0 { U384::from(q_val.min(0xFFFFFFFFu64)) } else { U384::from(1u64) }
            }
        } else {
            U384::from(0u64)
        };
        let new_r = if quotient.is_zero() {
            if r.0[5] != 0 && old_r.0[5] != 0 {
                let q = old_r.0[5] / r.0[5];
                let mut qb = U384::from(q);
                if qb * r > old_r && q > 0 {
                    qb = U384::from(q - 1);
                }
                old_r - r * qb
            } else {
                old_r - r
            }
        } else {
            old_r - r * quotient
        };
        old_r = r;
        r = mod_rem(&new_r, &m);
        let new_s = if quotient.is_zero() {
            if r.0[5] != 0 && old_r.0[5] != 0 {
                let q = old_r.0[5] / r.0[5];
                old_s - s * U384::from(q)
            } else {
                old_s - s
            }
        } else {
            old_s - s * quotient
        };
        old_s = s;
        s = mod_rem(&new_s, &m);
    }
    if old_r.is_one() {
        mod_rem(&old_s, &m)
    } else {
        U384::from(0u64)
    }
}

pub fn bytes_to_bigint(bytes: &[u8]) -> U384 {
    let mut buf = [0u8; 48];
    let start = 48 - bytes.len().min(48);
    for i in 0..bytes.len().min(48) {
        buf[start + i] = bytes[bytes.len() - 1 - i];
    }
    let mut out = [0u64; 6];
    for i in 0..6 {
        out[i] = u64::from_be_bytes([
            buf[i * 8], buf[i * 8 + 1], buf[i * 8 + 2], buf[i * 8 + 3],
            buf[i * 8 + 4], buf[i * 8 + 5], buf[i * 8 + 6], buf[i * 8 + 7],
        ]);
    }
    U384(out)
}

pub fn bigint_to_bytes(val: &U384, out: &mut [u8], len: usize) {
    let mut bytes = [0u8; 48];
    for i in 0..6 {
        bytes[i * 8..(i + 1) * 8].copy_from_slice(&val.0[i].to_be_bytes());
    }
    let start = 48 - len.min(48);
    let copy_len = len.min(48);
    for i in 0..copy_len {
        out[i] = bytes[start + i];
    }
}

pub fn mod_pow(base: U384, exp: U384, modulus: U384, _factorization: &[(U384, u32)]) -> U384 {
    if modulus.is_one() {
        return U384::from(0u64);
    }
    let mut result = U384::from(1u64);
    let mut b = mod_rem(&base, &modulus);
    let mut e = exp;
    while !e.is_zero() {
        if !(e & U384::from(1u64)).is_zero() {
            result = mod_rem(&(result * b), &modulus);
        }
        b = mod_rem(&(b * b), &modulus);
        let mut shifted = [0u64; 6];
        for i in 0..5 {
            shifted[i] = (e.0[i] >> 1) | (e.0[i + 1] << 63);
        }
        shifted[5] = e.0[5] >> 1;
        e = U384(shifted);
    }
    result
}

pub fn mod_product(factors: &[U384], modulus: &U384) -> U384 {
    let mut result = U384::from(1u64);
    for &f in factors {
        let f_mod = mod_rem(&f, modulus);
        result = mod_rem(&(result * f_mod), modulus);
    }
    result
}
