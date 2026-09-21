use super::bigint_math::{U384, mod_rem, mod_inverse_alt, mod_product};

#[derive(Clone, Copy, PartialEq, Eq)]
pub struct EccPoint {
    pub x: U384,
    pub y: U384,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub struct EccJacobianPoint {
    pub x: U384,
    pub y: U384,
    pub z: U384,
}

#[derive(Clone, Copy)]
pub struct EccCurve {
    pub prime: U384,
    pub a: U384,
    pub b: U384,
    pub order: U384,
    pub gx: U384,
    pub gy: U384,
}

#[derive(Clone, Copy)]
pub struct EccCurveParams {
    pub p192: EccCurve,
    pub p256: EccCurve,
    pub p384: EccCurve,
}

fn mod_mul(a: U384, b: U384, m: U384) -> U384 {
    mod_product(&[a, b], &m)
}

pub fn mod_add(a: U384, b: U384, m: U384) -> U384 {
    if a >= m - b { a - (m - b) } else { a + b }
}

fn mod_sub(a: U384, b: U384, m: U384) -> U384 {
    if a >= b { a - b } else { m - (b - a) }
}

pub fn ecc_curve_params() -> EccCurveParams {
    EccCurveParams {
        p192: EccCurve {
            prime: U384::from_hex_be("0xfffffffffffffffffffffffffffffffeffffffffffffffff"),
            a: U384::from_hex_be("0xfffffffffffffffffffffffffffffffefffffffffffffffc"),
            b: U384::from_hex_be("0x64210519e59c80e70fa7e9ab72243049feb8deecc146b9b1"),
            order: U384::from_hex_be("0xffffffffffffffffffffffff99def836146bc9b1b4d22831"),
            gx: U384::from_hex_be("0x188da80eb03090f67cbf20eb43a18800f4ff0afd82ff1012"),
            gy: U384::from_hex_be("0x07192b95ffc8da78631011ed6b24cdd573f977a11e794811"),
        },
        p256: EccCurve {
            prime: U384::from_hex_be("0xffffffff00000001000000000000000000000000ffffffffffffffffffffffff"),
            a: U384::from_hex_be("0xffffffff00000001000000000000000000000000fffffffffffffffffffffffc"),
            b: U384::from_hex_be("0x5ac635d8aa3a93e7b3ebbd55769886bc651d06b0cc53b0f63bce3c3e27d2604b"),
            order: U384::from_hex_be("0xffffffff00000000ffffffffffffffffbce6faada7179e84f3b9cac2fc632551"),
            gx: U384::from_hex_be("0x6b17d1f2e12c4247f8bce6e563a440f277037d812deb33a0f4a13945d898c296"),
            gy: U384::from_hex_be("0x4fe342e2fe1a7f9b8ee7eb4a7c0f9e162bce33576b315ececbb6406837bf51f5"),
        },
        p384: EccCurve {
            prime: U384::from_hex_be("0xfffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffeffffffff0000000000000000ffffffff"),
            a: U384::from_hex_be("0xfffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffeffffffff0000000000000000fffffffc"),
            b: U384::from_hex_be("0xb3312fa7e23ee7e4988e056be3f82d19181d9c6efe8141120314088f5013875ac656398d8a2ed19d2a85c8edd3ec2aef"),
            order: U384::from_hex_be("0xffffffffffffffffffffffffffffffffffffffffffffffffc7634d81f4372ddf581a0db248b0a77aecec196accc52973"),
            gx: U384::from_hex_be("0xaa87ca22be8b05378eb1c71ef320ad746e1d3b628ba79b9859f741e082542a385502f25dbf55296c3a545e3872760ab7"),
            gy: U384::from_hex_be("0x3617de4a96262c6f5d9e98bf9292dc29f8f41dbd289a147ce9da3113b5f0b8c00a60b1ce1d7e819d7a431d7c90ea0e5f"),
        },
    }
}

pub fn jacobian_from_affine(point: &EccPoint) -> EccJacobianPoint {
    EccJacobianPoint { x: point.x, y: point.y, z: U384::from(1u64) }
}

pub fn jacobian_to_affine(point: &EccJacobianPoint, curve: &EccCurve) -> EccPoint {
    if point.z.is_zero() {
        return EccPoint { x: U384::from(0u64), y: U384::from(0u64) };
    }
    let z_inv = mod_inverse_alt(point.z, curve.prime);
    let z_inv_sq = mod_mul(z_inv, z_inv, curve.prime);
    let z_inv_cu = mod_mul(z_inv_sq, z_inv, curve.prime);
    EccPoint {
        x: mod_mul(point.x, z_inv_sq, curve.prime),
        y: mod_mul(point.y, z_inv_cu, curve.prime),
    }
}

pub fn jacobian_equals(p1: &EccJacobianPoint, p2: &EccJacobianPoint) -> bool {
    p1.x == p2.x && p1.y == p2.y && p1.z == p2.z
}

pub fn ecc_point_add(p1: &EccPoint, p2: &EccPoint, curve: &EccCurve) -> EccPoint {
    if p1.x == p2.x && p1.y == p2.y {
        return ecc_point_double(p1, curve);
    }
    let s = mod_mul(
        mod_sub(p2.y, p1.y, curve.prime),
        mod_inverse_alt(mod_sub(p2.x, p1.x, curve.prime), curve.prime),
        curve.prime,
    );
    let x3 = mod_sub(mod_sub(mod_mul(s, s, curve.prime), p1.x, curve.prime), p2.x, curve.prime);
    let y3 = mod_sub(mod_mul(s, mod_sub(p1.x, x3, curve.prime), curve.prime), p1.y, curve.prime);
    EccPoint { x: x3, y: y3 }
}

pub fn ecc_point_double(point: &EccPoint, curve: &EccCurve) -> EccPoint {
    let p = curve.prime;
    let three_x2 = mod_mul(U384::from(3u64), mod_mul(point.x, point.x, p), p);
    let num = mod_add(three_x2, curve.a, p);
    let den = mod_mul(U384::from(2u64), point.y, p);
    let s = mod_mul(num, mod_inverse_alt(den, p), p);
    let x3 = mod_sub(mod_mul(s, s, p), mod_mul(U384::from(2u64), point.x, p), p);
    let y3 = mod_sub(mod_mul(s, mod_sub(point.x, x3, p), p), point.y, p);
    EccPoint { x: x3, y: y3 }
}

pub fn ecc_scalar_mult(point: &EccPoint, scalar: U384, curve: &EccCurve) -> EccPoint {
    let mut result: Option<EccPoint> = None;
    let mut current = EccPoint { x: point.x, y: point.y };
    let bit_len = scalar.leading_bit_len();
    let one = U384::from(1u64);
    for i in 0..bit_len {
        if !(scalar & (one << (i as u32))).is_zero() {
            result = Some(match result {
                None => current,
                Some(r) => ecc_point_add(&r, &current, curve),
            });
        }
        current = ecc_point_double(&current, curve);
    }
    result.unwrap_or(EccPoint { x: U384::from(0u64), y: U384::from(0u64) })
}

pub fn ecc_verify_point(point: &EccPoint, curve: &EccCurve) -> bool {
    let p = curve.prime;
    let y2 = mod_mul(point.y, point.y, p);
    let x3 = mod_mul(point.x, mod_mul(point.x, point.x, p), p);
    let ax = mod_mul(curve.a, point.x, p);
    let rhs = mod_add(mod_add(x3, ax, p), curve.b, p);
    y2 == rhs
}

pub fn ecc_jacobian_add(p1: &EccJacobianPoint, p2: &EccJacobianPoint, curve: &EccCurve) -> EccJacobianPoint {
    let p = curve.prime;
    if p1.z.is_zero() { return *p2; }
    if p2.z.is_zero() { return *p1; }
    let z2_sq = mod_mul(p2.z, p2.z, p);
    let z1_sq = mod_mul(p1.z, p1.z, p);
    let u1 = mod_mul(p1.x, z2_sq, p);
    let u2 = mod_mul(p2.x, z1_sq, p);
    let s1 = mod_mul(p1.y, mod_mul(p2.z, z2_sq, p), p);
    let s2 = mod_mul(p2.y, mod_mul(p1.z, z1_sq, p), p);
    if u1 == u2 {
        if s1 != s2 {
            return EccJacobianPoint { x: U384::from(0u64), y: U384::from(0u64), z: U384::from(1u64) };
        }
        return ecc_jacobian_double(p1, curve);
    }
    let h = mod_sub(u2, u1, p);
    let r = mod_sub(s2, s1, p);
    let h2 = mod_mul(h, h, p);
    let h3 = mod_mul(h2, h, p);
    let u1h2 = mod_mul(u1, h2, p);
    let x3 = mod_sub(mod_sub(mod_mul(r, r, p), h3, p), mod_mul(U384::from(2u64), u1h2, p), p);
    let y3 = mod_sub(mod_mul(r, mod_sub(u1h2, x3, p), p), mod_mul(s1, h3, p), p);
    let z3 = mod_mul(h, mod_mul(p1.z, p2.z, p), p);
    EccJacobianPoint { x: x3, y: y3, z: z3 }
}

pub fn ecc_jacobian_double(point: &EccJacobianPoint, curve: &EccCurve) -> EccJacobianPoint {
    let p = curve.prime;
    let y2 = mod_mul(point.y, point.y, p);
    let y4 = mod_mul(y2, y2, p);
    let h = mod_mul(U384::from(4u64), mod_mul(point.x, y2, p), p);
    let three_x2 = mod_mul(U384::from(3u64), mod_mul(point.x, point.x, p), p);
    let z2 = mod_mul(point.z, point.z, p);
    let z4 = mod_mul(z2, z2, p);
    let a_z4 = mod_mul(curve.a, z4, p);
    let off = mod_add(three_x2, a_z4, p);
    let x3 = mod_sub(mod_mul(off, off, p), mod_mul(U384::from(2u64), h, p), p);
    let y3 = mod_sub(mod_mul(off, mod_sub(h, x3, p), p), mod_mul(U384::from(8u64), y4, p), p);
    let z3 = mod_mul(U384::from(2u64), mod_mul(point.y, point.z, p), p);
    EccJacobianPoint { x: x3, y: y3, z: z3 }
}

pub fn ecc_jacobian_scalar_mult(point: &EccJacobianPoint, scalar: U384, curve: &EccCurve) -> EccJacobianPoint {
    let mut result: Option<EccJacobianPoint> = None;
    let mut current = EccJacobianPoint { x: point.x, y: point.y, z: point.z };
    let bit_len = scalar.leading_bit_len();
    let one = U384::from(1u64);
    for i in 0..bit_len {
        if !(scalar & (one << (i as u32))).is_zero() {
            result = Some(match result {
                None => current,
                Some(r) => ecc_jacobian_add(&r, &current, curve),
            });
        }
        current = ecc_jacobian_double(&current, curve);
    }
    result.unwrap_or(EccJacobianPoint { x: U384::from(0u64), y: U384::from(0u64), z: U384::from(0u64) })
}

pub fn ecc_jacobian_verify_point(point: &EccJacobianPoint, curve: &EccCurve) -> bool {
    let p = curve.prime;
    let y2 = mod_mul(point.y, point.y, p);
    let x3 = mod_mul(point.x, mod_mul(point.x, point.x, p), p);
    let z2 = mod_mul(point.z, point.z, p);
    let z4 = mod_mul(z2, z2, p);
    let a_x_z4 = mod_mul(curve.a, mod_mul(point.x, z4, p), p);
    let z6 = mod_mul(z4, z2, p);
    let b_z6 = mod_mul(curve.b, z6, p);
    let rhs = mod_add(mod_add(x3, a_x_z4, p), b_z6, p);
    y2 == rhs
}
