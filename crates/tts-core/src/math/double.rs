use super::{abs, nan64, sqrt, INVALID64};
use crate::tables::NumericTables;

fn high(x: f64) -> i32 {
    (x.to_bits() >> 32) as i32
}
fn low(x: f64) -> u32 {
    x.to_bits() as u32
}
fn with_high(x: f64, word: i32) -> f64 {
    f64::from_bits(((word as u64) << 32) | low(x) as u64)
}
fn truncate_low(x: f64) -> f64 {
    f64::from_bits(x.to_bits() & 0xffffffff00000000)
}
fn signed(x: f64, sign: f64) -> f64 {
    f64::from_bits((x.to_bits() & 0x7fffffffffffffff) | (sign.to_bits() & 0x8000000000000000))
}

/// fn_7fe900: Horner evaluation, preserving separate multiply/add rounding.
pub fn polynomial(x: f64, coefficients: &[f64]) -> f64 {
    let (&last, rest) = coefficients.split_last().expect("nonempty polynomial");
    rest.iter()
        .rev()
        .fold(last, |acc, &coefficient| acc * x + coefficient)
}

fn table_polynomial(x: f64, tables: &NumericTables<'_>, address: usize, count: usize) -> f64 {
    let mut acc = tables.double(address, count - 1);
    for index in (0..count - 1).rev() {
        acc = acc * x + tables.double(address, index);
    }
    acc
}

/// fn_84f2e4: exponent adjustment with the ARM library's flush-to-zero rules.
/// Subnormal input and underflow return positive zero; NaNs become default NaN.
pub fn scale(x: f64, n: i32) -> f64 {
    let bits = x.to_bits();
    let exponent = ((bits >> 52) & 0x7ff) as i32;
    if exponent == 0 {
        return if bits << 1 == 0 { x } else { 0.0 };
    }
    if exponent == 0x7ff {
        return if x.is_nan() { INVALID64 } else { x };
    }
    if n < 0x7ff - exponent && n > -exponent {
        return f64::from_bits((bits & 0x800fffffffffffff) | (((exponent + n) as u64) << 52));
    }
    if n >= 0 {
        signed(f64::INFINITY, x)
    } else {
        0.0
    }
}

fn parity(hx: i32, iy: i32, ly: u32) -> i32 {
    if hx >= 0 || iy >= 0x7ff00000 {
        return 0;
    }
    if iy >= 0x43400000 {
        return 2;
    }
    if iy < 0x3ff00000 {
        return 0;
    }
    let exponent = (iy >> 20) - 0x3ff;
    if exponent > 20 {
        let integer = ly >> (52 - exponent);
        if integer << (52 - exponent) == ly {
            return 2 - (integer & 1) as i32;
        }
    } else if ly == 0 {
        let integer = iy >> (20 - exponent);
        if integer << (20 - exponent) == iy {
            return 2 - (integer & 1);
        }
    }
    0
}

/// fn_7fd578: double power using the engine's split logarithm/exponential.
pub fn pow(x: f64, y: f64, tables: &NumericTables<'_>) -> f64 {
    let hx = high(x);
    let hy = high(y);
    let mut ix = hx & 0x7fffffff;
    let iy = hy & 0x7fffffff;
    let lx = low(x);
    let ly = low(y);
    if (hx as u32)
        .wrapping_mul(2)
        .wrapping_add(u32::from(lx != 0))
        .wrapping_add(0x1fffff)
        < 0xfffff
        || (hy as u32)
            .wrapping_mul(2)
            .wrapping_add(u32::from(ly != 0))
            .wrapping_add(0x1fffff)
            < 0xfffff
    {
        return nan64(x, y);
    }
    if (iy as u32 | ly) == 0 || (hx == 0x3ff00000 && lx == 0) {
        return 1.0;
    }
    if x.is_nan() || y.is_nan() {
        return nan64(x, y);
    }
    let parity = parity(hx, iy, ly);
    if (ix as u32 | lx) == 0 && hy < 0 {
        return if hx != 0 && parity == 1 {
            f64::NEG_INFINITY
        } else {
            f64::INFINITY
        };
    }
    if ly == 0 {
        if iy == 0x7ff00000 {
            if hx == 0xbff00000u32 as i32 && lx == 0 {
                return 1.0;
            }
            if ix >= 0x3ff00000 {
                if hy >= 0 {
                    return y;
                }
            } else if hy < 0 {
                return -y;
            }
            return tables.double(0xa224b8, 0);
        }
        if iy == 0x3ff00000 {
            if hy >= 0 {
                return x;
            }
            if ix == 0 {
                return signed(f64::INFINITY, x);
            }
            if ix == 0x7ff00000 && lx == 0 {
                return signed(0.0, x);
            }
            if ix < 0x7ff00000 {
                return 1.0 / x;
            }
            return x;
        }
        if ix < 0x7ff00000 {
            if hy == 0x40000000 {
                return x * x;
            }
            if hy == 0x3fe00000 && hx >= 0 {
                return sqrt(x);
            }
        }
    }
    let mut ax = abs(x);
    if lx == 0 {
        if ix == 0 {
            return if hx < 0 && parity == 1 { -0.0 } else { 0.0 };
        }
        if ix == 0x7ff00000 {
            return match (hy < 0, hx < 0 && parity == 1) {
                (true, true) => -0.0,
                (true, false) => 0.0,
                (false, true) => f64::NEG_INFINITY,
                (false, false) => f64::INFINITY,
            };
        }
    }
    let nonnegative = 1 + (hx >> 31);
    if nonnegative | parity == 0 {
        return INVALID64;
    }
    let (mut leading, trailing) = if iy > 0x41e00000 {
        if iy > 0x43f00000 {
            if ix <= 0x3fefffff {
                return if hy < 0 { f64::INFINITY } else { 0.0 };
            }
            if ix >= 0x3ff00000 {
                return if hy > 0 { f64::INFINITY } else { 0.0 };
            }
        }
        if ix < 0x3fefffff {
            return if hy < 0 { f64::INFINITY } else { 0.0 };
        }
        if ix > 0x3ff00000 {
            return if hy <= 0 { 0.0 } else { f64::INFINITY };
        }
        let t = x - 1.0;
        let w = t * t * (0.5 - t * (0.33333333333333331 - t * 0.25));
        let u = t * 1.4426950216293335;
        let v = t * 1.9259629911266175e-8 - w * 1.4426950408889634;
        let leading = truncate_low(u + v);
        (leading, v - (leading - u))
    } else {
        let mut n = 0;
        if ix < 0x00100000 {
            ax *= 9007199254740992.0;
            n -= 53;
            ix = high(ax);
        }
        n += (ix >> 20) - 0x3ff;
        let mantissa = ix & 0xfffff;
        ix = mantissa | 0x3ff00000;
        let index = if mantissa <= 0x3988e {
            0
        } else if mantissa < 0xbb67a {
            1
        } else {
            n += 1;
            ix -= 0x00100000;
            0
        };
        ax = with_high(ax, ix);
        let center = tables.double(0xa22020, index);
        let u = ax - center;
        let v = 1.0 / (ax + center);
        let s = u * v;
        let t = truncate_low(s);
        let center_high = with_high(
            tables.double(0xa224b8, 0),
            ((ix >> 1) | 0x20000000) + 0x80000 + ((index as i32) << 18),
        );
        let center_low = ax - (center_high - center);
        let sl = v * ((u - t * center_high) - t * center_low);
        let s2 = s * s;
        let r = s2 * s2 * table_polynomial(s2, tables, 0xa22050, 6);
        let r = r + sl * (t + s);
        let s2 = t * t;
        let leading = truncate_low(3.0 + s2 + r);
        let tail = r - ((leading - 3.0) - s2);
        let u = t * leading;
        let v = sl * leading + tail * s;
        let leading = truncate_low(u + v);
        let pl = v - (leading - u);
        let z = 0.96179670095443726 * leading;
        let w = -7.0284616509527583e-9 * leading
            + 0.96179669392597558 * pl
            + tables.double(0xa22040, index);
        let t = n as f64;
        let offset = tables.double(0xa22030, index);
        let leading = truncate_low(((z + w) + offset) + t);
        (leading, w - (((leading - t) - offset) - z))
    };
    let sign = if nonnegative | (parity - 1) == 0 {
        -1.0
    } else {
        1.0
    };
    let t = truncate_low(y);
    let pl = (y - t) * leading + y * trailing;
    leading *= t;
    let z = pl + leading;
    let mut j = high(z);
    let low_word = low(z);
    if j >= 0x40900000 {
        if low_word | (j - 0x40900000) as u32 != 0 || pl + 8.0085662595372944e-17 > z - leading {
            return signed(f64::INFINITY, sign);
        }
    } else if (j & 0x7fffffff) as u32 >= 0x4090cc00
        && (low_word | j.wrapping_add(0x3f6f3400) as u32 != 0 || z - leading >= pl)
    {
        return 0.0;
    }
    let mut k = j & 0x7fffffff;
    let mut n = 0;
    let exponent = (k >> 20) - 0x3ff;
    if k > 0x3fe00000 {
        k = exponent;
        n = j.wrapping_add(0x00100000 >> (k + 1));
        k = ((n & 0x7fffffff) >> 20) - 0x3ff;
        let t = with_high(tables.double(0xa224b8, 0), n & !(0x000fffff >> k));
        n = ((n & 0xfffff) | 0x00100000) >> (20 - k);
        if j < 0 {
            n = -n;
        }
        leading -= t;
    }
    let t = truncate_low(pl + leading);
    let u = t * 0.69314718246459961;
    let v = (pl - (t - leading)) * 0.69314718055994529 + t * -1.904654299957768e-9;
    let z = u + v;
    let w = v - (z - u);
    let t = z * z;
    let leading = z - t * table_polynomial(t, tables, 0xa22080, 5);
    let r = (z * leading) / (leading - 2.0) - (w + z * w);
    let z = 1.0 - (r - z);
    j = high(z).wrapping_add(n.wrapping_shl(20));
    let z = if j >> 20 > 0 {
        with_high(z, j)
    } else {
        scale(z, n)
    };
    sign * z
}
