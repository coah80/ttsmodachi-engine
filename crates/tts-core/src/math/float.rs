use super::{float_to_u32, nan32, INVALID32};
use crate::tables::NumericTables;

fn rounded_high(x: f32) -> f32 {
    f32::from_bits(x.to_bits().wrapping_add(0x800) & 0xfffff000)
}

/// fn_1b90d0: table-reduced single-precision exponential.
pub fn expf(x: f32, tables: &NumericTables<'_>) -> f32 {
    let bits = x.to_bits();
    let mut special = false;
    if bits.wrapping_mul(2).wrapping_add(0x99000000) > 0x1e500000 {
        if ((!bits << 1) >> 24) == 0 {
            return if bits == 0xff800000 {
                0.0
            } else if x.is_nan() {
                nan32(x, x)
            } else {
                x + x
            };
        }
        if bits.wrapping_mul(2) < 0x67000000 {
            return 1.0;
        }
        if bits.wrapping_mul(2) > 0x85a00000 {
            return if bits >> 31 == 0 { f32::INFINITY } else { 0.0 };
        }
        special = true;
    }
    let bias = if bits >> 31 == 0 { 0.5 } else { -0.5 };
    let n = (bias + x * 5.770780086517334) as i32;
    let index = (n & 3) as usize;
    let nf = n as f32;
    let r = (x - nf * 0.17327880859375) - nf * 7.986545824678615e-6;
    let p = 0.16672924160957336 + r * 0.04168231040239334;
    let p = 0.4999999701976776 + r * p;
    let p = 0.9999999403953552 + r * p;
    let p = p * r;
    let p = tables.float(0xa21fc0, index)
        + tables.float(0xa21fd0, index) * p
        + tables.float(0xa21fb0, index);
    let exponent = n >> 2;
    if special {
        let half = exponent / 2;
        let first = f32::from_bits(0x3f800000u32.wrapping_add((half as u32) << 23));
        let second = f32::from_bits(0x3f800000u32.wrapping_add(((exponent - half) as u32) << 23));
        return second * (first * p);
    }
    f32::from_bits((1065353216.0 + exponent as f32 * 8388608.0) as u32) * p
}

/// fn_1b9300: table-reduced single-precision natural logarithm.
pub fn logf(mut x: f32, tables: &NumericTables<'_>) -> f32 {
    let mut bits = x.to_bits();
    let mut exponent = 0;
    if bits.wrapping_sub(0x800000) >= 0x7f000000 {
        if bits << 1 > 0xff000000 {
            return nan32(x, x);
        }
        if bits == 0x7f800000 {
            return x;
        }
        if x < 0.0 {
            return INVALID32;
        }
        if x == 0.0 {
            return f32::NEG_INFINITY;
        }
        exponent = -23;
        x *= 8388608.0;
        bits = x.to_bits();
    }
    let adjustment = ((bits as i32).wrapping_add(0x80000) >> 23) - 127;
    let index = ((bits << 9).wrapping_add(0x10000000) >> 29) as usize;
    bits = bits.wrapping_sub((adjustment as u32) << 23);
    x = f32::from_bits(bits);
    exponent += adjustment;
    let center = f32::from_bits(0x3f800000 + ((index as u32) << 20));
    if exponent != 0 || index != 0 {
        x = (x - center) / (x + center);
        let square = x * x;
        let cube = x * square;
        let polynomial = 0.666666626930236816 + square * 0.400279223918914795;
        let correction = tables.float(0xa22000, index) + exponent as f32 * 3.19461832987144589e-5;
        let leading = tables.float(0xa21fe0, index) + exponent as f32 * 0.693115234375;
        let tail = correction + cube * polynomial;
        return leading + (tail + x * 2.0);
    }
    x -= 1.0;
    let polynomial = -0.250129997730255127 + x * 0.190357699990272522;
    let polynomial = 0.333341628313064575 + x * polynomial;
    let polynomial = -0.499999970197677612 + x * polynomial;
    x + (x * x) * polynomial
}

fn exponent_parity(bits: u32) -> i32 {
    let exponent = (bits << 1) >> 24;
    if exponent < 127 {
        return 0;
    }
    if exponent >= 151 {
        return 2;
    }
    let mask = 1 << (150 - exponent);
    if (mask - 1) & bits != 0 {
        0
    } else if mask & bits != 0 {
        1
    } else {
        2
    }
}

fn special_pow(x: f32, y: f32) -> f32 {
    let ix = x.to_bits();
    let iy = y.to_bits();
    let mx = ix & 0x7fffffff;
    let my = iy & 0x7fffffff;
    if mx.wrapping_sub(0x7f800001) < 0x3fffff || my.wrapping_sub(0x7f800001) < 0x3fffff {
        return nan32(x, y);
    }
    if ix == 0x3f800000 || iy << 1 == 0 {
        return 1.0;
    }
    if x.is_nan() || y.is_nan() {
        return nan32(x, y);
    }
    if ix == 0x7f800000 {
        return if iy >> 31 != 0 { 0.0 } else { f32::INFINITY };
    }
    let parity = exponent_parity(iy);
    if ix == 0xff800000 {
        return match (iy >> 31 != 0, parity == 1) {
            (true, true) => -0.0,
            (true, false) => 0.0,
            (false, true) => f32::NEG_INFINITY,
            (false, false) => f32::INFINITY,
        };
    }
    if ix == 0 {
        return if iy >> 31 == 0 { 0.0 } else { f32::INFINITY };
    }
    if ix == 0x80000000 {
        if iy == 0x7f800000 {
            return 0.0;
        }
        if iy == 0xff800000 {
            return f32::INFINITY;
        }
        if iy >> 31 != 0 {
            return if parity == 1 {
                f32::NEG_INFINITY
            } else {
                f32::INFINITY
            };
        }
        return if parity == 1 { -0.0 } else { 0.0 };
    }
    if ix == 0xbf800000 {
        return 1.0;
    }
    if i32::from(ix << 1 >= 0x7f000000) != 1 + ((iy as i32) >> 31) {
        0.0
    } else {
        f32::INFINITY
    }
}

/// fn_1b94c0: table-reduced single-precision power with ARM saturating casts.
pub fn powf(x: f32, y: f32, tables: &NumericTables<'_>) -> f32 {
    let result = power_inner(x, y, tables);
    if result.is_nan() && !x.is_nan() && !y.is_nan() {
        INVALID32
    } else {
        result
    }
}

fn power_inner(mut x: f32, mut y: f32, tables: &NumericTables<'_>) -> f32 {
    let mut ix = x.to_bits();
    let mut iy = y.to_bits();
    let mut adjustment = 0;
    let mut sign = 1.0;
    let direct_log = ix.wrapping_sub(0x800000) < 0x7f000000
        && iy.wrapping_mul(2).wrapping_add(0x2000000) >= 0x40000000;
    if !direct_log {
        if iy.wrapping_mul(2).wrapping_add(0x2000000) < 0x40000000 {
            let magnitude = iy.wrapping_mul(2).wrapping_add(0x2000000);
            if magnitude >= 0x2000000 {
                if y == 0.0 {
                    if ix << 1 != 0 && ix << 1 < 0xff000000 {
                        return 1.0;
                    }
                } else {
                    iy = (iy & 0x80000000) | 0x1f800000;
                    y = f32::from_bits(iy);
                }
            } else if magnitude < 0x1000000 {
                y = f32::from_bits(y.to_bits().wrapping_sub(0x800000));
            } else {
                return special_pow(x, y);
            }
        }
        let negative = if ix.wrapping_add(0x7f800000) < 0x7f000000 {
            true
        } else if ix << 1 != 0 && ix << 1 < 0x1000000 {
            adjustment = -27;
            x *= 134217728.0;
            ix = x.to_bits();
            ix >> 31 != 0
        } else {
            if ix.wrapping_sub(0x800000) >= 0x7f000000 {
                return special_pow(x, y);
            }
            false
        };
        if negative {
            x = f32::from_bits(x.to_bits() & 0x7fffffff);
            ix = x.to_bits();
            match exponent_parity(iy) {
                0 => return INVALID32,
                1 => sign = -1.0,
                _ => {}
            }
        }
    }
    let (high, low) = if ix.wrapping_add(0xc0801000) <= 0x2000 {
        let a = (x as f64 - 1.0) as f32;
        let b = 0.3333333432674408 - a * 0.25;
        let c = 0.5 - a * b;
        let d = -(a * a * c);
        let e = a * 23.0859375;
        let f = a * -0.0028168456628918648 + d * 23.083120346069336;
        let high = rounded_high(e + f);
        (high, (e - high) + f)
    } else {
        let n = ix.wrapping_add(0x40000) >> 19;
        let index = (n & 15) as usize;
        let exponent = ((n << 20) >> 24) as i32 - 127;
        let a = f32::from_bits(ix.wrapping_sub((exponent as u32) << 23));
        let b = 1.0 + index as f32 * 0.0625;
        let c = rounded_high(a + b);
        let d = (b - c) + a;
        let e = 1.0 / (a + b);
        let f = a - b;
        let g = rounded_high(f * e);
        let h = ((f - g * c) - g * d) * e;
        let a = (g + h) * (g + h);
        let b = (0.3333333432674408 + a * 0.20008274912834167) * a;
        let c = g;
        let d = h + b * (g + h);
        let e = rounded_high(c + d);
        let f = (c - e) + d;
        let h = e * -0.0056336913257837296 + f * 46.16624069213867;
        let g = e * 46.171875;
        let a = rounded_high(g + h);
        let b = (g - a) + h;
        let c = tables.float(0xa220a8, index * 2);
        let d = tables.float(0xa220a8, index * 2 + 1);
        let e = ((exponent + adjustment) * 16) as f32;
        let high = rounded_high(b + d + a + c + e);
        (high, b - ((((high - e) - c) - a) - d))
    };
    let a = rounded_high(y);
    let b = y - a;
    let c = high * a;
    let d = high * b + low * (a + b);
    let high = rounded_high(c + d);
    let low = (c - high) + d;
    let a = high + low;
    let bias = if a >= 0.0 { 0.5 } else { -0.5 };
    let n = (bias + a) as i32;
    let exponent = (n >> 4) as f32;
    let index = (n & 15) as usize;
    let b = (high - n as f32) + low;
    let c = 0.0009384123259223998 + b * 1.3551205483963713e-5;
    let d = (0.043321698904037476 + b * c) * b;
    let e = tables.float(0xa22168, index);
    let f = tables.float(0xa221a8, index);
    let g = tables.float(0xa22128, index);
    x = e + f * d + g;
    if !(exponent.abs() >= 126.0) {
        return f32::from_bits(float_to_u32((exponent + 127.0) * 8388608.0)) * (sign * x);
    }
    if !(exponent.abs() >= 192.0) {
        let n = exponent as i32;
        let half = n / 2;
        let first = f32::from_bits(((half as u32) << 23).wrapping_add(0x3f800000));
        let second = f32::from_bits((((n - half) as u32) << 23).wrapping_add(0x3f800000));
        x = (first * (sign * x)) * second;
        if x.to_bits() << 1 != 0xff000000 && x.to_bits() << 1 != 0 {
            return x;
        }
    }
    let magnitude = if exponent >= 0.0 { f32::INFINITY } else { 0.0 };
    f32::from_bits(magnitude.to_bits() | (sign.to_bits() & 0x80000000))
}
