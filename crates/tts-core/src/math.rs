mod double;
mod float;
pub use double::*;
pub use float::*;

pub(crate) const INVALID32: f32 = f32::from_bits(0x7fc00000);
pub(crate) const INVALID64: f64 = f64::from_bits(0x7ff8000000000000);

pub(crate) fn nan32(a: f32, b: f32) -> f32 {
    let signaling = |x: f32| x.is_nan() && x.to_bits() & 0x00400000 == 0;
    let x = if signaling(a) {
        a
    } else if signaling(b) {
        b
    } else if a.is_nan() {
        a
    } else {
        b
    };
    f32::from_bits(x.to_bits() | 0x00400000)
}

pub(crate) fn nan64(a: f64, b: f64) -> f64 {
    let signaling = |x: f64| x.is_nan() && x.to_bits() & 0x0008000000000000 == 0;
    let x = if signaling(a) {
        a
    } else if signaling(b) {
        b
    } else if a.is_nan() {
        a
    } else {
        b
    };
    f64::from_bits(x.to_bits() | 0x0008000000000000)
}

fn rounded_sqrt(significand: u128, shift: u32) -> u64 {
    let radicand = significand << shift;
    let mut root = 0u128;
    let mut bit = 1u128 << ((128 - radicand.leading_zeros() - 1) & !1);
    let mut remainder = radicand;
    while bit != 0 {
        if remainder >= root + bit {
            remainder -= root + bit;
            root = (root >> 1) + bit;
        } else {
            root >>= 1;
        }
        bit >>= 2;
    }
    (root + u128::from(remainder > root)) as u64
}

pub fn sqrtf(x: f32) -> f32 {
    if x.is_nan() {
        return nan32(x, x);
    }
    if x == 0.0 || x == f32::INFINITY {
        return x;
    }
    if x < 0.0 {
        return INVALID32;
    }
    let mut bits = x.to_bits();
    let mut exponent = ((bits >> 23) & 255) as i32 - 127;
    if exponent == -127 {
        let normalized = x * 16777216.0;
        bits = normalized.to_bits();
        exponent = ((bits >> 23) & 255) as i32 - 127 - 24;
    }
    let mut mantissa = ((bits & 0x7fffff) | 0x800000) as u128;
    if exponent & 1 != 0 {
        mantissa <<= 1;
        exponent -= 1;
    }
    let root = rounded_sqrt(mantissa, 23) as u32;
    f32::from_bits((((exponent / 2 + 127) as u32) << 23) + root - 0x800000)
}

pub fn sqrt(x: f64) -> f64 {
    if x.is_nan() {
        return nan64(x, x);
    }
    if x == 0.0 || x == f64::INFINITY {
        return x;
    }
    if x < 0.0 {
        return INVALID64;
    }
    let mut bits = x.to_bits();
    let mut exponent = ((bits >> 52) & 2047) as i32 - 1023;
    if exponent == -1023 {
        bits = (x * 18014398509481984.0).to_bits();
        exponent = ((bits >> 52) & 2047) as i32 - 1023 - 54;
    }
    let mut mantissa = ((bits & 0xfffffffffffff) | 0x10000000000000) as u128;
    if exponent & 1 != 0 {
        mantissa <<= 1;
        exponent -= 1;
    }
    let root = rounded_sqrt(mantissa, 52);
    f64::from_bits((((exponent / 2 + 1023) as u64) << 52) + root - 0x10000000000000)
}

pub fn float_to_u32(x: f32) -> u32 {
    x as u32
}

pub fn floorf(x: f32) -> f32 {
    let mut bits = x.to_bits();
    let exponent = ((bits >> 23) & 255) as i32 - 127;
    if exponent < 0 {
        if bits << 1 == 0 {
            return x;
        }
        return if bits >> 31 == 0 { 0.0 } else { -1.0 };
    }
    if exponent >= 23 {
        return x;
    }
    let mask = 0x7fffff >> exponent;
    if bits & mask == 0 {
        return x;
    }
    if bits >> 31 != 0 {
        bits = bits.wrapping_add(0x800000 >> exponent);
    }
    f32::from_bits(bits & !mask)
}

pub fn classifyf(x: f32) -> i32 {
    let bits = x.to_bits() << 1;
    let mut class = if bits << 8 != 0 { 4 } else { 0 };
    if bits >> 24 != 0 {
        class |= 1;
    }
    if bits & 0xff000000 == 0xff000000 {
        class |= 2;
    }
    if class == 1 {
        5
    } else {
        class
    }
}

pub fn classify(x: f64) -> i32 {
    let bits = x.to_bits();
    let mut class = if bits & 0xfffffffffffff != 0 { 4 } else { 0 };
    let exponent = (bits >> 52) & 2047;
    if exponent != 0 {
        class |= 1;
    }
    if exponent == 2047 {
        class |= 2;
    }
    if class == 1 {
        5
    } else {
        class
    }
}

pub fn abs(x: f64) -> f64 {
    f64::from_bits(x.to_bits() & 0x7fffffffffffffff)
}

pub fn tinyf() -> f32 {
    0.0
}

pub fn tiny() -> f64 {
    0.0
}

pub fn remainderf(x: f32, y: f32) -> f32 {
    let magnitude_y = f32::from_bits(y.to_bits() & 0x7fffffff);
    if x.is_nan() || y.is_nan() || x.is_infinite() || magnitude_y < f32::MIN_POSITIVE {
        return INVALID32;
    }
    let remainder = truncated_remainder(x, y);
    let magnitude = remainder.abs() as f64;
    let tiny = (x != 0.0 && x.abs() < f32::MIN_POSITIVE)
        || (magnitude != 0.0
            && magnitude.min(magnitude_y as f64 - magnitude) < f32::MIN_POSITIVE as f64);
    if tiny {
        if x.is_sign_negative() {
            -magnitude_y
        } else {
            0.0
        }
    } else {
        remainder
    }
}

fn truncated_remainder(x: f32, y: f32) -> f32 {
    let sign = x.to_bits() & 0x80000000;
    let mut a = x.to_bits() & 0x7fffffff;
    let mut b = y.to_bits() & 0x7fffffff;
    if a < b {
        return x;
    }
    if a == b {
        return f32::from_bits(sign);
    }
    let mut ea = (a >> 23) as i32;
    let mut eb = (b >> 23) as i32;
    if ea == 0 {
        ea = 1;
        while a & 0x800000 == 0 {
            a <<= 1;
            ea -= 1;
        }
    } else {
        a = (a & 0x7fffff) | 0x800000;
    }
    if eb == 0 {
        eb = 1;
        while b & 0x800000 == 0 {
            b <<= 1;
            eb -= 1;
        }
    } else {
        b = (b & 0x7fffff) | 0x800000;
    }
    while ea > eb {
        if a >= b {
            a -= b;
        }
        a <<= 1;
        ea -= 1;
    }
    if a >= b {
        a -= b;
    }
    if a == 0 {
        return f32::from_bits(sign);
    }
    while a & 0x800000 == 0 {
        a <<= 1;
        ea -= 1;
    }
    if ea > 0 {
        a = (a - 0x800000) | ((ea as u32) << 23);
    } else {
        a >>= 1 - ea;
    }
    f32::from_bits(a | sign)
}

pub fn cosine(x: f32, tables: &crate::tables::NumericTables<'_>) -> f32 {
    let mut angle = f32::from_bits(x.to_bits() & 0x7fffffff);
    if angle.to_bits() as i32 >= 0x47800000 {
        angle = remainderf(angle, 65536.0);
    }
    let whole = (angle as u32) & 0xffff;
    let fraction = angle - whole as f32;
    let index = (whole & 255) as usize * 4;
    tables.float(0x8cbfe0, index + 1) + fraction * tables.float(0x8cbfe0, index + 3)
}

pub fn sine(x: f32, tables: &crate::tables::NumericTables<'_>) -> f32 {
    let mut angle = f32::from_bits(x.to_bits() & 0x7fffffff);
    if angle.to_bits() as i32 >= 0x47800000 {
        angle = remainderf(angle, 65536.0);
    }
    let whole = (angle as u32) & 0xffff;
    let fraction = angle - whole as f32;
    let index = (whole & 255) as usize * 4;
    let result = tables.float(0x8cbfe0, index) + fraction * tables.float(0x8cbfe0, index + 2);
    if x < 0.0 {
        -result
    } else {
        result
    }
}
