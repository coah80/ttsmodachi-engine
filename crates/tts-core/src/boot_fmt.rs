//! RVCT byte formatter. Guest pointers refer to caller-owned `boot::Memory`.
//! No host varargs, libc, allocation, or Rust formatting are used.
use crate::boot::Memory;

/// Source of promoted arguments. `wide` consumes an eight-byte, eight-aligned
/// ARM argument; ordinary arguments consume four bytes. The source owns its cursor.
pub(crate) trait Arguments {
    fn word(&mut self, memory: &Memory<'_>) -> u32;
    fn wide(&mut self, memory: &Memory<'_>) -> u64;
    /// Pointer reads a word before any ll/j alignment, but consumes a wide slot.
    fn pointer(&mut self, memory: &Memory<'_>, wide: bool) -> u32;
}

/// ARM AAPCS argument cursor, including alignment holes before doubles and ll/j.
pub struct VaList {
    pub address: u32,
}
impl Arguments for VaList {
    fn pointer(&mut self, memory: &Memory<'_>, wide: bool) -> u32 {
        let result = memory.word(self.address);
        if wide {
            self.wide(memory);
        } else {
            self.word(memory);
        }
        result
    }
    fn word(&mut self, memory: &Memory<'_>) -> u32 {
        let result = memory.word(self.address);
        self.address = self.address.wrapping_add(4);
        result
    }
    fn wide(&mut self, memory: &Memory<'_>) -> u64 {
        self.address = self.address.wrapping_add(7) & !7;
        let low = self.word(memory) as u64;
        low | ((self.word(memory) as u64) << 32)
    }
}

/// Locale and runtime rounding are platform state, not host libc calls.
pub(crate) trait Host {
    /// Convert one UTF-16 code unit as RVCT wcrtomb does. None means EILSEQ.
    fn encode(&mut self, unit: u16, output: &mut [u8; 8], state: &mut [u8; 8]) -> Option<usize>;
    fn decimal_point(&mut self) -> u8 {
        b'.'
    }
}
pub struct CLocale;
impl Host for CLocale {
    fn encode(&mut self, unit: u16, output: &mut [u8; 8], _state: &mut [u8; 8]) -> Option<usize> {
        if unit <= 255 {
            output[0] = unit as u8;
            Some(1)
        } else {
            None
        }
    }
}
const LEFT: u32 = 1;
const PLUS: u32 = 2;
const SPACE: u32 = 4;
const ALT: u32 = 8;
const ZERO: u32 = 16;
const PREC: u32 = 32;
const LONG: u32 = 64;
const LL: u32 = 128;
const SHORT: u32 = 256;
const BYTE: u32 = 1024;
const UPPER: u32 = 2048;
#[derive(Clone, Copy)]
struct Field {
    flags: u32,
    width: i32,
    precision: i32,
}
struct Output {
    cursor: u32,
    end: Option<u32>,
    count: u32,
}
impl Output {
    fn byte(&mut self, m: &mut Memory<'_>, v: u8) {
        if self.end.map_or(true, |end| self.cursor < end) {
            let at = m.offset(self.cursor);
            m.bytes[at] = v;
            self.cursor = self.cursor.wrapping_add(1);
        }
        self.count = self.count.wrapping_add(1);
    }
    fn repeat(&mut self, m: &mut Memory<'_>, v: u8, n: i32) {
        for _ in 0..n.max(0) {
            self.byte(m, v);
        }
    }
    fn pad(&mut self, m: &mut Memory<'_>, f: Field, remaining: i32, before: bool) {
        if (f.flags & LEFT == 0) == before {
            self.repeat(
                m,
                if before && f.flags & ZERO != 0 {
                    b'0'
                } else {
                    b' '
                },
                remaining,
            );
        }
    }
    fn field(&mut self, m: &mut Memory<'_>, f: Field, bytes: &[u8], prefix: usize) {
        let pad = f.width.wrapping_sub(bytes.len() as i32);
        if f.flags & ZERO == 0 {
            self.pad(m, f, pad, true);
        }
        for &b in &bytes[..prefix] {
            self.byte(m, b);
        }
        if f.flags & ZERO != 0 {
            self.pad(m, f, pad, true);
        }
        for &b in &bytes[prefix..] {
            self.byte(m, b);
        }
        self.pad(m, f, pad, false);
    }
}
fn take(m: &Memory<'_>, fmt: &mut u32) -> u8 {
    let v = m.byte(*fmt);
    *fmt = fmt.wrapping_add(1);
    v
}
fn sign(f: Field, negative: bool) -> Option<u8> {
    if negative {
        Some(b'-')
    } else if f.flags & PLUS != 0 {
        Some(b'+')
    } else if f.flags & SPACE != 0 {
        Some(b' ')
    } else {
        None
    }
}
fn integer(m: &mut Memory<'_>, out: &mut Output, mut f: Field, c: u8, raw: u64) {
    let signed = c == b'd' || c == b'i';
    let raw = if c == b'p' {
        raw as u32 as u64
    } else if f.flags & LL != 0 {
        raw
    } else if f.flags & BYTE != 0 {
        if signed {
            (raw as i8 as i64) as u64
        } else {
            raw as u8 as u64
        }
    } else if f.flags & SHORT != 0 {
        if signed {
            (raw as i16 as i64) as u64
        } else {
            raw as u16 as u64
        }
    } else if signed {
        (raw as i32 as i64) as u64
    } else {
        raw as u32 as u64
    };
    let negative = signed && (raw as i64) < 0;
    let mut v = if negative { raw.wrapping_neg() } else { raw };
    let radix = if c == b'o' {
        8
    } else if c == b'x' || c == b'p' {
        16
    } else {
        10
    };
    let alphabet = if f.flags & UPPER != 0 {
        b"0123456789ABCDEF"
    } else {
        b"0123456789abcdef"
    };
    let mut digits = [0u8; 64];
    let mut n = 0;
    while v != 0 {
        digits[n] = alphabet[(v % radix) as usize];
        n += 1;
        v /= radix;
    }
    let mut prefix = [0u8; 2];
    let mut pn = 0;
    if signed {
        if let Some(s) = sign(f, negative) {
            prefix[0] = s;
            pn = 1;
        }
    } else if c == b'p' {
        f.flags |= PREC;
        f.precision = 8;
        if f.flags & ALT != 0 {
            prefix[0] = b'@';
            pn = 1;
        }
    } else if f.flags & ALT != 0 {
        if c == b'o' && (n != 0 || f.flags & PREC != 0) {
            prefix[0] = b'0';
            pn = 1;
            f.precision = f.precision.wrapping_sub(1);
        } else if c == b'x' && n != 0 {
            prefix[0] = b'0';
            prefix[1] = if f.flags & UPPER != 0 { b'X' } else { b'x' };
            pn = 2;
        }
    }
    let min = if f.flags & PREC != 0 {
        f.flags &= !ZERO;
        f.precision
    } else {
        1
    };
    let zeros = if min > n as i32 { min - n as i32 } else { 0 };
    let pad = f.width.wrapping_sub((n + pn) as i32).wrapping_sub(zeros);
    if f.flags & ZERO == 0 {
        out.pad(m, f, pad, true);
    }
    for &b in &prefix[..pn] {
        out.byte(m, b);
    }
    if f.flags & ZERO != 0 {
        out.pad(m, f, pad, true);
    }
    out.repeat(m, b'0', zeros);
    for i in (0..n).rev() {
        out.byte(m, digits[i]);
    }
    out.pad(m, f, pad, false);
}
fn text(
    m: &mut Memory<'_>,
    out: &mut Output,
    f: Field,
    p: u32,
    character: bool,
    wide: bool,
    host: &mut impl Host,
) {
    let mut n = 0u32;
    let mut bytes = 0i32;
    let mut tmp = [0u8; 8];
    let mut conversion_state = [0u8; 8];
    let limit = if character && !wide {
        1
    } else if f.flags & PREC != 0 {
        f.precision as u32
    } else {
        u32::MAX
    };
    while if wide {
        bytes < (limit as i32) || limit == u32::MAX
    } else {
        n < limit
    } {
        if character && n == 1 {
            break;
        }
        let unit = if character {
            p as u16
        } else if wide {
            m.byte(p + n * 2) as u16 | ((m.byte(p + n * 2 + 1) as u16) << 8)
        } else {
            m.byte(p + n) as u16
        };
        if !character && unit == 0 {
            break;
        }
        if wide {
            if let Some(len) = host.encode(unit, &mut tmp, &mut conversion_state) {
                if bytes.wrapping_add(len as i32) > limit as i32 && limit != u32::MAX {
                    break;
                }
                bytes = bytes.wrapping_add(len as i32);
            }
        } else {
            bytes = bytes.wrapping_add(1);
        }
        n += 1;
    }
    conversion_state.fill(0);
    let pad = f.width.wrapping_sub(bytes);
    out.pad(m, f, pad, true);
    for i in 0..n {
        let unit = if character {
            p as u16
        } else if wide {
            m.byte(p + i * 2) as u16 | ((m.byte(p + i * 2 + 1) as u16) << 8)
        } else {
            m.byte(p + i) as u16
        };
        if wide {
            if let Some(len) = host.encode(unit, &mut tmp, &mut conversion_state) {
                for &b in &tmp[..len] {
                    out.byte(m, b);
                }
            }
        } else {
            out.byte(m, unit as u8);
        }
    }
    out.pad(m, f, pad, false);
}

/// Exact finite binary64 value represented in decimal, before RVCT's 17 digit cap.
/// Digits are little endian; `scale` is the number after the decimal point.
pub(crate) struct Decimal {
    digits: [u8; 1100],
    len: usize,
    scale: i32,
}
impl Decimal {
    fn new(bits: u64) -> Self {
        let exp = ((bits >> 52) & 2047) as i32;
        let mut value = bits & 0x000f_ffff_ffff_ffff;
        if exp != 0 {
            value |= 1 << 52;
        }
        let mut d = Self {
            digits: [0; 1100],
            len: 0,
            scale: 0,
        };
        while value != 0 {
            d.digits[d.len] = (value % 10) as u8;
            d.len += 1;
            value /= 10;
        }
        if d.len == 0 {
            d.len = 1;
            return d;
        }
        let binary = if exp == 0 { -1074 } else { exp - 1075 };
        let factor = if binary < 0 { 5 } else { 2 };
        for _ in 0..binary.unsigned_abs() {
            let mut carry = 0;
            for digit in &mut d.digits[..d.len] {
                let v = *digit as u16 * factor + carry;
                *digit = (v % 10) as u8;
                carry = v / 10;
            }
            while carry != 0 {
                d.digits[d.len] = (carry % 10) as u8;
                d.len += 1;
                carry /= 10;
            }
        }
        d.scale = (-binary).max(0);
        d
    }
    pub(crate) fn exponent(&self) -> i32 {
        self.len as i32 - self.scale - 1
    }
    pub(crate) fn zero(&self) -> bool {
        self.len == 1 && self.digits[0] == 0
    }
    pub(crate) fn at(&self, power: i64) -> u8 {
        let i = power + self.scale as i64;
        if i >= 0 && (i as usize) < self.len {
            b'0' + self.digits[i as usize]
        } else {
            b'0'
        }
    }
}
// RVCT uses a 64-bit significand, not exact rational decimal rounding.
// Keeping each intermediate rounding is observable at decimal half-way cases.
#[derive(Clone, Copy)]
struct Extended {
    sig: u64,
    exponent: i32,
}
fn rounded(n: u128, d: u128, mode: i32) -> u128 {
    let q = n / d;
    let r = n % d;
    q + u128::from(r != 0 && (mode > 0 || mode == 0 && (r > d - r || r == d - r && q & 1 != 0)))
}
impl Extended {
    fn one() -> Self {
        Self {
            sig: 1 << 63,
            exponent: 0,
        }
    }
    fn product(self, other: Self, mode: i32) -> Self {
        let product = self.sig as u128 * other.sig as u128;
        let shift = if product >> 127 != 0 { 64 } else { 63 };
        let mut sig = rounded(product, 1u128 << shift, mode);
        let mut exponent = self.exponent + other.exponent + (shift - 63);
        if sig == 1u128 << 64 {
            sig >>= 1;
            exponent += 1;
        }
        Self {
            sig: sig as u64,
            exponent,
        }
    }
    fn quotient(self, other: Self, mode: i32) -> Self {
        let shift = if self.sig < other.sig { 64 } else { 63 };
        let mut sig = rounded((self.sig as u128) << shift, other.sig as u128, mode);
        let mut exponent = self.exponent - other.exponent + 63 - shift;
        if sig == 1u128 << 64 {
            sig >>= 1;
            exponent += 1;
        }
        Self {
            sig: sig as u64,
            exponent,
        }
    }
    fn exact_power10(n: usize, mode: i32) -> Self {
        // Generate the RVCT power table mathematically; no ROM table is embedded.
        let mut limbs = [0u64; 24];
        limbs[0] = 1;
        let mut len = 1;
        for _ in 0..n {
            let mut carry = 0u128;
            for limb in &mut limbs[..len] {
                let v = *limb as u128 * 10 + carry;
                *limb = v as u64;
                carry = v >> 64;
            }
            if carry != 0 {
                limbs[len] = carry as u64;
                len += 1;
            }
        }
        let bits = (len - 1) * 64 + 64 - limbs[len - 1].leading_zeros() as usize;
        if bits <= 64 {
            return Self {
                sig: limbs[0] << (64 - bits),
                exponent: bits as i32 - 1,
            };
        }
        let cut = bits - 64;
        let word = cut / 64;
        let shift = cut % 64;
        let mut sig = (limbs[word] >> shift) as u128;
        if shift != 0 {
            sig |= (limbs[word + 1] as u128) << (64 - shift);
        }
        let guard = (limbs[(cut - 1) / 64] >> ((cut - 1) % 64)) & 1 != 0;
        let sticky = limbs[..(cut - 1) / 64].iter().any(|&v| v != 0)
            || limbs[(cut - 1) / 64] & ((1u64 << ((cut - 1) % 64)) - 1) != 0;
        let up = if mode > 0 {
            guard || sticky
        } else if mode == 0 {
            guard && (sticky || sig & 1 != 0)
        } else {
            false
        };
        sig += u128::from(up);
        let mut exponent = bits as i32 - 1;
        if sig == 1u128 << 64 {
            sig >>= 1;
            exponent += 1;
        }
        Self {
            sig: sig as u64,
            exponent,
        }
    }
    fn power10(n: i32, mode: i32) -> Self {
        let q = (n + 27) / 55;
        let r = n - q * 55;
        let mut a = Self::one();
        let mut b = Self::one();
        for i in 0..5 {
            if r.unsigned_abs() >> i & 1 != 0 {
                a = a.product(Self::exact_power10(1 << i, mode), mode);
            }
        }
        for i in 0..4 {
            if q >> i & 1 != 0 {
                b = b.product(Self::exact_power10(55 << i, mode), mode);
            }
        }
        if r < 0 {
            b.quotient(a, mode)
        } else {
            b.product(a, mode)
        }
    }
    fn binary64(bits: u64) -> Self {
        let raw = (bits >> 52) & 2047;
        let fraction = bits & ((1 << 52) - 1);
        if raw != 0 {
            Self {
                sig: (fraction | (1 << 52)) << 11,
                exponent: raw as i32 - 1023,
            }
        } else if fraction != 0 {
            let shift = fraction.leading_zeros();
            Self {
                sig: fraction << shift,
                exponent: -1011 - shift as i32,
            }
        } else {
            Self {
                sig: 0,
                exponent: 0,
            }
        }
    }
    /// Scale and round directly to an integer. This deliberately does not round
    /// once to normalized extended and then a second time to integer.
    fn scaled_integer(self, power: i32, mode: i32) -> Option<u64> {
        if self.sig == 0 {
            return Some(0);
        }
        let factor = Self::power10(power.abs(), if power > 0 { -mode } else { mode });
        let (numerator, denominator, shift) = if power > 0 {
            (
                self.sig as u128,
                factor.sig as u128,
                self.exponent - factor.exponent,
            )
        } else {
            (
                self.sig as u128 * factor.sig as u128,
                1u128,
                self.exponent + factor.exponent - 126,
            )
        };
        let value = if shift >= 0 {
            if numerator.leading_zeros() < (shift as u32) {
                return None;
            }
            rounded(numerator << shift, denominator, mode)
        } else {
            let shift = shift.unsigned_abs();
            if denominator.leading_zeros() < shift {
                // Numerator is at most 128 bits. A denominator beyond that range
                // yields zero except upward rounding, or a nearest half threshold.
                if mode > 0
                    || mode == 0
                        && shift > 0
                        && denominator.leading_zeros() >= shift - 1
                        && numerator > (denominator << (shift - 1))
                {
                    1
                } else {
                    0
                }
            } else {
                rounded(numerator, denominator << shift, mode)
            }
        };
        u64::try_from(value).ok()
    }
}
pub(crate) fn rvct_decimal(bits: u64, c: u8, precision: i32, rounding: u32) -> Decimal {
    let negative = bits >> 63 != 0;
    let mode = match rounding {
        1 => {
            if negative {
                -1
            } else {
                1
            }
        }
        2 => {
            if negative {
                1
            } else {
                -1
            }
        }
        3 => -1,
        _ => 0,
    };
    let original = Decimal::new(bits);
    if original.zero() {
        return original;
    }
    let ext = Extended::binary64(bits);
    let mut exponent = original.exponent();
    let mut fixed = c == b'f' && original.exponent().saturating_add(precision) < 20;
    let digits = if c == b'e' {
        precision.saturating_add(1).min(17)
    } else if c == b'g' {
        precision.max(1).min(17)
    } else {
        17
    };
    loop {
        let power = if fixed {
            -precision
        } else {
            exponent - digits + 1
        };
        if let Some(mut integer) = ext.scaled_integer(power, mode) {
            let mut result = Decimal {
                digits: [0; 1100],
                len: 0,
                scale: -power,
            };
            while integer != 0 {
                result.digits[result.len] = (integer % 10) as u8;
                result.len += 1;
                integer /= 10;
            }
            if result.len == 0 {
                result.len = 1;
            }
            if fixed {
                if result.len <= 17 {
                    return result;
                }
            } else if result.len == digits as usize {
                return result;
            } else if result.len > digits as usize {
                exponent += 1;
                continue;
            } else {
                exponent -= 1;
                continue;
            }
        }
        fixed = false;
    }
}

fn append(buf: &mut [u8; 32], n: &mut usize, b: u8) {
    buf[*n] = b;
    *n += 1;
}
pub(crate) fn exponent(buf: &mut [u8; 32], n: &mut usize, e: i32, letter: u8, min: usize) {
    append(buf, n, letter);
    append(buf, n, if e < 0 { b'-' } else { b'+' });
    let mut v = e.unsigned_abs();
    let mut b = [0; 10];
    let mut count = 0;
    while v != 0 || count < min {
        b[count] = b'0' + (v % 10) as u8;
        v /= 10;
        count += 1;
    }
    for i in (0..count).rev() {
        append(buf, n, b[i]);
    }
}
fn floating(
    m: &mut Memory<'_>,
    out: &mut Output,
    mut f: Field,
    c: u8,
    bits: u64,
    host: &mut impl Host,
) {
    let negative = bits >> 63 != 0;
    let exp = ((bits >> 52) & 2047) as i32;
    let frac = bits & 0x000f_ffff_ffff_ffff;
    let s = sign(f, negative);
    if exp == 2047 {
        f.flags &= !ZERO;
        let mut b = [0u8; 32];
        let mut n = 0;
        if let Some(s) = s {
            append(&mut b, &mut n, s);
        }
        let word = match (frac == 0, f.flags & UPPER != 0) {
            (true, true) => b"INF",
            (true, false) => b"inf",
            (false, true) => b"NAN",
            (false, false) => b"nan",
        };
        for &v in word {
            append(&mut b, &mut n, v);
        }
        out.field(m, f, &b[..n], 0);
        return;
    }
    if c == b'a' {
        let explicit = f.flags & PREC != 0;
        let p = if explicit { f.precision.max(0) } else { 13 };
        let mut value = frac | if exp != 0 { 1 << 52 } else { 0 };
        let mut e = if exp != 0 {
            exp - 1023
        } else if frac != 0 {
            -1022
        } else {
            0
        };
        if p < 13 {
            let shift = 52 - 4 * p;
            let mask = (1u64 << shift) - 1;
            // RVCT inspects only this shifted word, as in the Thumb runtime.
            let rem = if p < 5 {
                ((bits >> 32) as u32).wrapping_shl((4 * p + 12) as u32)
            } else {
                (bits as u32).wrapping_shl((4 * p - 20) as u32)
            };
            let half = 1u32 << 31;
            let odd = value & (1 << shift) != 0;
            // fn_2ffde4 returns zero in this ROM: nearest/even.
            let up = rem > half || (rem == half && odd);
            value &= !mask;
            if up {
                value += 1 << shift;
            }
            if value >= 1 << 53 {
                value >>= 1;
                e += 1;
            }
        }
        let mut prefix = [0u8; 32];
        let mut pn = 0;
        if let Some(s) = s {
            append(&mut prefix, &mut pn, s);
        }
        append(&mut prefix, &mut pn, b'0');
        append(
            &mut prefix,
            &mut pn,
            if f.flags & UPPER != 0 { b'X' } else { b'x' },
        );
        let mut mantissa = [0u8; 32];
        let mut n = 0;
        append(&mut mantissa, &mut n, b'0' + (value >> 52) as u8);
        if p > 0 || f.flags & ALT != 0 {
            append(&mut mantissa, &mut n, b'.');
        }
        let alphabet = if f.flags & UPPER != 0 {
            b"0123456789ABCDEF"
        } else {
            b"0123456789abcdef"
        };
        for i in 0..p.min(13) {
            append(
                &mut mantissa,
                &mut n,
                alphabet[((value >> (48 - i * 4)) & 15) as usize],
            );
        }
        if !explicit {
            while mantissa[n - 1] == b'0' {
                n -= 1;
            }
            if mantissa[n - 1] == b'.' {
                n -= 1;
            }
        }
        // RVCT's sentinel consumes one precision position beyond the 13 nibbles,
        // and also reserves one extra field position that is never emitted.
        let zeros = (p - 14).max(0);
        let mut tail = [0u8; 32];
        let mut tn = 0;
        exponent(
            &mut tail,
            &mut tn,
            e,
            if f.flags & UPPER != 0 { b'P' } else { b'p' },
            1,
        );
        let pad = f
            .width
            .wrapping_sub((pn + n + tn) as i32)
            .wrapping_sub(zeros)
            .wrapping_sub(i32::from(p > 13));
        if f.flags & ZERO == 0 {
            out.pad(m, f, pad, true);
        }
        for &v in &prefix[..pn] {
            out.byte(m, v);
        }
        if f.flags & ZERO != 0 {
            out.pad(m, f, pad, true);
        }
        for &v in &mantissa[..n] {
            out.byte(m, v);
        }
        out.repeat(m, b'0', zeros);
        for &v in &tail[..tn] {
            out.byte(m, v);
        }
        out.pad(m, f, pad, false);
        return;
    }
    let p = if f.flags & PREC != 0 {
        f.precision.max(0)
    } else {
        6
    };
    let d = rvct_decimal(bits, c, p, 0);
    let e = if d.zero() { 0 } else { d.exponent() };
    let scientific = c == b'e' || c == b'g' && (e < -4 || e >= p.max(1));
    let mut places = if c == b'g' {
        if scientific {
            p.max(1) - 1
        } else {
            p.max(1).saturating_sub(1 + e).max(0)
        }
    } else {
        p
    };
    if c == b'g' && f.flags & ALT == 0 {
        // All digits outside the 17 retained positions are zero. Skip that
        // potentially enormous zero suffix before inspecting the retained digits.
        let effective = if scientific { e + d.scale } else { d.scale };
        places = places.min(effective.max(0));
        while places > 0 {
            let at = if scientific { e - places } else { -places };
            if d.at(at as i64) != b'0' {
                break;
            }
            places -= 1;
        }
    }
    let first = if scientific {
        e as i64
    } else {
        e.max(0) as i64
    };
    let last = if scientific {
        e as i64 - places as i64
    } else {
        -(places as i64)
    };
    let point = places > 0 || f.flags & ALT != 0;
    let mut tail = [0u8; 32];
    let mut tn = 0;
    if scientific {
        exponent(
            &mut tail,
            &mut tn,
            e,
            if f.flags & UPPER != 0 { b'E' } else { b'e' },
            2,
        );
    }
    let len = (first - last + 1) as i32;
    let len = len
        .wrapping_add(i32::from(point))
        .wrapping_add(i32::from(s.is_some()))
        .wrapping_add(tn as i32);
    let pad = f.width.wrapping_sub(len);
    if f.flags & ZERO == 0 {
        out.pad(m, f, pad, true);
    }
    if let Some(s) = s {
        out.byte(m, s);
    }
    if f.flags & ZERO != 0 {
        out.pad(m, f, pad, true);
    }
    for power in (last..=first).rev() {
        out.byte(m, d.at(power));
        if power == (if scientific { e as i64 } else { 0 }) && point {
            out.byte(m, host.decimal_point());
        }
    }
    for &v in &tail[..tn] {
        out.byte(m, v);
    }
    out.pad(m, f, pad, false);
}
fn format(
    m: &mut Memory<'_>,
    out: &mut Output,
    mut fmt: u32,
    args: &mut impl Arguments,
    host: &mut impl Host,
) {
    loop {
        let mut c = take(m, &mut fmt);
        if c == 0 {
            break;
        }
        if c != b'%' {
            out.byte(m, c);
            continue;
        }
        let mut f = Field {
            flags: 0,
            width: 0,
            precision: 0,
        };
        c = take(m, &mut fmt);
        loop {
            let flag = match c {
                b'-' => LEFT,
                b'+' => PLUS,
                b' ' => SPACE,
                b'#' => ALT,
                b'0' => ZERO,
                _ => 0,
            };
            if flag == 0 {
                break;
            }
            f.flags |= flag;
            c = take(m, &mut fmt);
        }
        if f.flags & PLUS != 0 {
            f.flags &= !SPACE;
        }
        for part in 0..2 {
            let mut value = 0i32;
            let star = c == b'*';
            if star {
                value = args.word(m) as i32;
                c = take(m, &mut fmt);
            } else {
                while c.is_ascii_digit() {
                    value = value.wrapping_mul(10).wrapping_add((c - b'0') as i32);
                    c = take(m, &mut fmt);
                }
            }
            if part == 0 {
                f.width = value;
            } else {
                f.precision = value;
                if star && value < 0 {
                    f.flags &= !PREC;
                }
            }
            if part != 0 || c != b'.' {
                break;
            }
            f.flags |= PREC;
            c = take(m, &mut fmt);
        }
        if f.width < 0 {
            f.width = f.width.wrapping_neg();
            f.flags |= LEFT;
        }
        if f.flags & LEFT != 0 {
            f.flags &= !ZERO;
        }
        if c == b'h' || c == b'l' {
            let prev = c;
            c = take(m, &mut fmt);
            if c == prev {
                f.flags |= if prev == b'h' { BYTE } else { LL };
                c = take(m, &mut fmt);
            } else {
                f.flags |= if prev == b'h' { SHORT } else { LONG };
            }
        } else if matches!(c, b'j' | b'L' | b't' | b'z') {
            if c == b'j' {
                f.flags |= LL;
            }
            c = take(m, &mut fmt);
        }
        if c == 0 {
            break;
        }
        if c.is_ascii_uppercase() {
            f.flags |= UPPER;
            c += 32;
        }
        match c {
            b'd' | b'i' | b'u' | b'o' | b'x' | b'p' => {
                let raw = if c == b'p' {
                    args.pointer(m, f.flags & LL != 0) as u64
                } else if f.flags & LL != 0 {
                    args.wide(m)
                } else {
                    args.word(m) as u64
                };
                integer(m, out, f, c, raw);
            }
            b'n' => {
                let p = args.word(m);
                let count = out.count;
                if f.flags & BYTE != 0 {
                    let at = m.offset(p);
                    m.bytes[at] = count as u8;
                } else if f.flags & SHORT != 0 {
                    m.put_half(p, count as u16);
                } else {
                    m.put_word(p, count);
                    if f.flags & LL != 0 {
                        m.put_word(p + 4, (count as i32 >> 31) as u32);
                    }
                }
            }
            b'c' | b's' => {
                let p = args.word(m);
                text(m, out, f, p, c == b'c', f.flags & (LONG | LL) != 0, host);
            }
            b'a' | b'e' | b'f' | b'g' => {
                let bits = args.wide(m);
                floating(m, out, f, c, bits, host);
            }
            _ => out.byte(m, c),
        }
    }
}

/// fn_10012c, Thumb vsnprintf. Returns the wrapping would-have-written byte count.
/// `dst`, NUL-terminated `fmt`, arguments, and %s/%n pointees are guest addresses.
/// Capacity zero performs no destination access. A nonzero capacity always writes NUL.
pub fn fn_10012c(
    m: &mut Memory<'_>,
    dst: u32,
    capacity: u32,
    fmt: u32,
    args: &mut impl Arguments,
    host: &mut impl Host,
) -> u32 {
    let mut out = Output {
        cursor: dst,
        end: Some(if capacity == 0 {
            dst
        } else {
            dst.wrapping_add(capacity - 1)
        }),
        count: 0,
    };
    format(m, &mut out, fmt, args, host);
    if capacity != 0 {
        let at = m.offset(out.cursor);
        m.bytes[at] = 0;
    }
    out.count
}

/// fn_100b50, Thumb sprintf. Same argument contract as fn_10012c; caller guarantees
/// writable space for the complete result and its terminating NUL.
pub fn fn_100b50(
    m: &mut Memory<'_>,
    dst: u32,
    fmt: u32,
    args: &mut impl Arguments,
    host: &mut impl Host,
) -> u32 {
    let mut out = Output {
        cursor: dst,
        end: None,
        count: 0,
    };
    format(m, &mut out, fmt, args, host);
    let at = m.offset(out.cursor);
    m.bytes[at] = 0;
    out.count
}
