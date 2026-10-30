//! ROM string subset used by the JP engine (strcpy/strcat/strlen) plus the
//! SDK int/log formatters (`fn_6b2694`, `fn_6ae534`). All over borrowed
//! slices; no allocation. Extra ROM libc helpers (strcmp, strtol, ...) get
//! added back with direct-ROM rows when a ported caller needs them.

/// strcpy: copies through the NUL; returns length excluding the NUL.
/// Matches ROM `fn_303cd0` (2 args, dst return ignored here).
pub fn strcpy0(dst: &mut [u8], src: &[u8]) -> usize {
    let mut w = 0usize;
    for &b in src.iter() {
        dst[w] = b;
        w += 1;
        if b == 0 {
            break;
        }
    }
    w - 1
}

/// strcat at known length `dlen`; returns the new length (excl. NUL).
/// Matches ROM `fn_101228`.
pub fn strcat0(dst: &mut [u8], dlen: usize, src: &[u8]) -> usize {
    let mut w = dlen;
    for &b in src.iter() {
        dst[w] = b;
        w += 1;
        if b == 0 {
            break;
        }
    }
    w - 1
}

/// strlen: bytes before the first NUL (or slice end).
/// Matches ROM `fn_3034a4`.
pub fn strlen0(s: &[u8]) -> usize {
    s.iter().position(|&b| b == 0).unwrap_or(s.len())
}

/// ROM `fn_16b53c`: zero `n` bytes.
pub fn bzero0(dst: &mut [u8], n: usize) {
    dst[..n].fill(0);
}

/// strcmp with unsigned-char difference. Matches ROM `fn_303f78`.
pub fn strcmp0(a: &[u8], b: &[u8]) -> i32 {
    let mut i = 0usize;
    loop {
        let ca = *a.get(i).unwrap_or(&0);
        let cb = *b.get(i).unwrap_or(&0);
        if ca != cb {
            return ca as i32 - cb as i32;
        }
        if ca == 0 {
            return 0;
        }
        i += 1;
    }
}

/// strncpy: exactly `n` bytes, NUL-padded past the source terminator.
/// Matches ROM `fn_1935c0`.
pub fn strncpy0(dst: &mut [u8], src: &[u8], n: usize) {
    for i in 0..n {
        dst[i] = *src.get(i).unwrap_or(&0);
    }
}

/// strncat: at most `n` non-NUL source bytes plus the terminator.
/// Matches ROM `fn_1009ac`. Returns the new length (excl. NUL).
pub fn strncat0(dst: &mut [u8], dlen: usize, src: &[u8], n: usize) -> usize {
    let mut w = dlen;
    let mut k = 0usize;
    for &b in src.iter() {
        if b == 0 || k >= n {
            break;
        }
        dst[w] = b;
        w += 1;
        k += 1;
    }
    dst[w] = 0;
    w
}

/// strtol, base 10, 32-bit long, endptr NULL. Matches ROM `fn_100d9c`.
/// Skips ASCII whitespace, takes an optional sign and digits, clamps to
/// LONG_MAX/MIN on overflow, yields 0 when no digits follow.
pub fn strtol10(s: &[u8]) -> i32 {
    let mut i = 0usize;
    while i < s.len() && matches!(s[i], b' ' | b'\t' | b'\n' | 0x0b | 0x0c | b'\r') {
        i += 1;
    }
    let mut neg = false;
    if i < s.len() && (s[i] == b'+' || s[i] == b'-') {
        neg = s[i] == b'-';
        i += 1;
    }
    let mut acc: u32 = 0;
    let mut any = false;
    let mut overflow = false;
    while i < s.len() && s[i].is_ascii_digit() {
        any = true;
        let d = (s[i] - b'0') as u32;
        let lim = if neg { 0x80000000u32 } else { 0x7FFFFFFFu32 };
        if acc > lim / 10 || (acc == lim / 10 && d > lim % 10) {
            overflow = true;
        }
        acc = acc.wrapping_mul(10).wrapping_add(d);
        i += 1;
    }
    if !any {
        return 0;
    }
    if overflow {
        return if neg { i32::MIN } else { i32::MAX };
    }
    if neg {
        (acc as i32).wrapping_neg()
    } else {
        acc as i32
    }
}

/// memcmp over exactly `n` bytes; unsigned-char difference.
/// Matches ROM `fn_1009d0` (caller of `fn_19caa8`).
pub fn memcmp0(a: &[u8], b: &[u8], n: usize) -> i32 {
    for i in 0..n {
        let ca = a[i];
        let cb = b[i];
        if ca != cb {
            return ca as i32 - cb as i32;
        }
    }
    0
}

/// fn_6b2694: SDK int formatter with the `(s16)(0 - t)` negative quirk.
/// Returns `dst` itself so call sites chain like the C.
pub fn fmt_int(dst: &mut [u8], v: i32) -> &mut [u8] {
    let mut p = 0usize;
    let mut t = v;
    if t < 0 {
        dst[p] = b'-';
        p += 1;
        t = (0i32.wrapping_sub(t)) as i16 as i32;
    }
    let mut div = 1i32;
    while t / div >= 10 {
        div *= 10;
    }
    let mut d = div;
    let mut r = t;
    loop {
        let q = r / d;
        dst[p] = (q + 48) as u8;
        p += 1;
        r -= q * d;
        d /= 10;
        if d < 1 {
            break;
        }
    }
    dst[p] = 0;
    dst
}

/// fn_6ae534: SDK log formatter (`buf + mode/a/b/c` words, newline).
/// Returns `(s16)len` on success, -1 when `size <= len_out + len_sp`.
/// Total stack use stays under 212 bytes for caller-sized inputs.
pub fn log_format(
    out: &mut [u8],
    size: i32,
    buf: &[u8],
    mode: i32,
    a: i32,
    b: i32,
    c: i32,
) -> i32 {
    if buf.first().copied().unwrap_or(0) == 0 {
        return 0;
    }
    let mut stack = [0u8; 212];
    let mut tmp = [0u8; 20];
    let mut slen = strcpy0(&mut stack, buf);
    let mut app = |s: &[u8]| {
        slen = strcat0(&mut stack, slen, s);
    };
    app(b" \0");
    app(fmt_int(&mut tmp, mode));
    app(b" \0");
    app(fmt_int(&mut tmp, a));
    app(b" \0");
    app(fmt_int(&mut tmp, b));
    app(b" \0");
    app(fmt_int(&mut tmp, c));
    app(b"\n\0");
    let len_sp = slen as i32;
    let len_out = strlen0(out) as i32;
    if (size as u32) <= (len_out.wrapping_add(len_sp)) as u32 {
        return -1;
    }
    let olen = len_out as usize;
    strcat0(out, olen, &stack);
    len_sp as i16 as i32
}

/// strchr: index of the first `c` in `s`, stopping at NUL like ROM
/// `fn_101144` (native maps it to libc `strchr`). `None` if absent.
pub fn strchr0(s: &[u8], c: u8) -> Option<usize> {
    for (i, &b) in s.iter().enumerate() {
        if b == c {
            return Some(i);
        }
        if b == 0 {
            return None;
        }
    }
    None
}
