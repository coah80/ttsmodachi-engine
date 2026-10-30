//! live fe one-word root 1f98c8 and the multi-word dictionary path 275928, over the caller's arena.
//!
//! The ports live over the caller's bytes: every guest pointer stays a scalar
//! address inside the arena, every guest call is issued through the host, and
//! the frame the caller owns is restored before the port returns.

#![allow(dead_code)]
#![allow(non_upper_case_globals)]

use crate::{boot::Memory, component_live_b::Host};

/// The `out` interface slots the ROM reads out of the caller's vtable.
const OUT_ADD_CHILD: u32 = 9;
const OUT_NEW_RECORD: u32 = 10;
const OUT_INSERT_BEFORE: u32 = 11;
const OUT_GET_FIRST: u32 = 13;
const OUT_GET_NEXT: u32 = 15;
const OUT_SET_FIELD: u32 = 20;
const OUT_GET_FIELD: u32 = 21;
const OUT_GET_FIELD_PTR: u32 = 22;
const OUT_HAS_FIELD: u32 = 23;
const OUT_REMOVE: u32 = 24;

/// The `cfg` (fe_dctlkp) interface slots.
const CFG_GET_STRING: u32 = 7;
const CFG_ADD_WORD: u32 = 9;
const CFG_WORD_LENGTHS: u32 = 10;
const CFG_SET_MARKERS: u32 = 11;
const CFG_NORMALIZE_TOKEN: u32 = 13;
const CFG_NORMALIZE_WORD: u32 = 14;
const CFG_LOOKUP: u32 = 18;

/// The ROM data the one-word path uses.
const D_8f9524: u32 = 0x8f9524;
const D_8f9532: u32 = 0x8f9532;
const D_8f9574: u32 = 0x8f9574;
const D_8f9580: u32 = 0x8f9580;

/// The `trans` (translator) interface slots the ROM reads out of the
/// caller's vtable: set/add at 9-11, position at 14-15.
const TRANS_SET_STRING: u32 = 9;
const TRANS_GET_STRING: u32 = 10;
const TRANS_SELECT: u32 = 11;
const TRANS_SET_POS: u32 = 14;
const TRANS_GET_POS: u32 = 15;

/// The `Fe` field offsets the ROM reads.
const FE_OUT: u32 = 0x0c;
const FE_CFG_HANDLE: u32 = 0x10;
const FE_CFG_VT: u32 = 0x18;
const FE_TRANS_HANDLE: u32 = 0x1c;
const FE_TRANS_VT: u32 = 0x24;
const FE_F50: u32 = 0x50;
const FE_TEXT_OFFSET: u32 = 0x50;
const FE_NONCTDPS: u32 = 0x70;
const FE_DPS: u32 = 0x71;
const FE_DOMAIN: u32 = 0x72;
const FE_F173: u32 = 0x173;
const FE_LEX: u32 = 0x174;
const FE_NO_RICH_TRANS: u32 = 0x17a;
const FE_CLM_VARIANTS: u32 = 0x17b;
const FE_CLC_EXTRA_MAIN_TRANS: u32 = 0x17c;
const FE_EXTRA_MAIN_TRANS: u32 = 0x17d;
const FE_F1A5: u32 = 0x1a5;
const FE_DPS_MULTI_TRANS: u32 = 0x2a6;
const FE_N_OUT_TRANS: u32 = 0x2a7;
const FE_NO_EXT_CLC: u32 = 0x2a8;
const FE_LANG_CODE: u32 = 0x2ac;
const FE_ESC_TN: u32 = 0x2b0;
const FE_CLM_HANDLE: u32 = 0x34;
const FE_CLM_MAP: u32 = 0x3c;
const FE_F68: u32 = 0x68;

/// The `Marker` layout on the wire: type, position, length, value.
const MARKER_LEN: u32 = 24;
const MARKER_TYPE: u32 = 0;
const MARKER_POS: u32 = 12;
const MARKER_LEN_OFF: u32 = 16;
const MARKER_VAL: u32 = 20;

/// The failure status the ports use when the arena does not hold what the
/// ROM expects. It matches the ROM's own out-of-memory status.
const FAIL_MEM: u32 = 0x89b0200a;
const FAIL_HANDLE: u32 = 0x89b02008;

fn g_word(m: &Memory<'_>, p: u32) -> Option<u32> {
    let at = p.checked_sub(m.base)? as usize;
    let b = m.bytes.get(at..at.checked_add(4)?)?;
    Some(u32::from_le_bytes(b.try_into().ok()?))
}

fn g_half(m: &Memory<'_>, p: u32) -> Option<u16> {
    let at = p.checked_sub(m.base)? as usize;
    let b = m.bytes.get(at..at.checked_add(2)?)?;
    Some(u16::from_le_bytes(b.try_into().ok()?))
}

fn g_byte(m: &Memory<'_>, p: u32) -> Option<u8> {
    let at = p.checked_sub(m.base)? as usize;
    Some(*m.bytes.get(at)?)
}

fn g_range(m: &Memory<'_>, p: u32, n: u32) -> bool {
    let at = match p.checked_sub(m.base) {
        Some(v) => v as usize,
        None => return false,
    };
    match at.checked_add(n as usize) {
        Some(end) => end <= m.bytes.len(),
        None => false,
    }
}

fn p_word(m: &mut Memory<'_>, p: u32, v: u32) -> bool {
    if !g_range(m, p, 4) {
        return false;
    }
    let at = (p - m.base) as usize;
    m.bytes[at..at + 4].copy_from_slice(&v.to_le_bytes());
    true
}

fn p_half(m: &mut Memory<'_>, p: u32, v: u16) -> bool {
    if !g_range(m, p, 2) {
        return false;
    }
    let at = (p - m.base) as usize;
    m.bytes[at..at + 2].copy_from_slice(&v.to_le_bytes());
    true
}

fn p_byte(m: &mut Memory<'_>, p: u32, v: u8) -> bool {
    if !g_range(m, p, 1) {
        return false;
    }
    let at = (p - m.base) as usize;
    m.bytes[at] = v;
    true
}

/// The length of the guest string at `p`, terminator excluded.
fn g_strlen(m: &Memory<'_>, p: u32) -> Option<u32> {
    let mut i = 0u32;
    loop {
        match g_byte(m, p.checked_add(i)?) {
            Some(0) => return Some(i),
            Some(_) => i = i.checked_add(1)?,
            None => return None,
        }
    }
}

/// The length of the guest wide string at `p`.
fn g_wstrlen(m: &Memory<'_>, p: u32) -> Option<u32> {
    let mut i = 0u32;
    loop {
        match g_half(m, p.checked_add(i.checked_mul(2)?)?) {
            Some(0) => return Some(i),
            Some(_) => i = i.checked_add(1)?,
            None => return None,
        }
    }
}

/// Copy the guest string `src` to `dst`, terminator included.
fn g_strcpy(m: &mut Memory<'_>, dst: u32, src: u32) -> bool {
    let len = match g_strlen(m, src) {
        Some(v) => v,
        None => return false,
    };
    if !g_range(m, dst, len + 1) || !g_range(m, src, len + 1) {
        return false;
    }
    let s = (src - m.base) as usize;
    let d = (dst - m.base) as usize;
    m.bytes.copy_within(s..s + len as usize, d);
    let d = (dst - m.base) as usize;
    m.bytes[d + len as usize] = 0;
    true
}

/// Compare two guest strings.
fn g_strcmp(m: &Memory<'_>, a: u32, b: u32) -> i32 {
    let mut i = 0u32;
    loop {
        let x = g_byte(m, a.wrapping_add(i)).unwrap_or(0);
        let y = g_byte(m, b.wrapping_add(i)).unwrap_or(0);
        if x != y {
            return i32::from(x) - i32::from(y);
        }
        if y == 0 {
            return 0;
        }
        i = i.wrapping_add(1);
    }
}

/// Compare a guest string with a Rust literal.
fn g_streq(m: &Memory<'_>, a: u32, b: &[u8]) -> bool {
    let mut i = 0usize;
    loop {
        let x = g_byte(m, a.wrapping_add(i as u32)).unwrap_or(0);
        let y = b.get(i).copied().unwrap_or(0);
        if x != y {
            return false;
        }
        if y == 0 {
            return true;
        }
        i += 1;
    }
}

/// Find `c` in the guest string `s`. Zero when it is absent.
fn g_strchr(m: &Memory<'_>, s: u32, c: u32) -> u32 {
    let mut i = 0u32;
    loop {
        match g_byte(m, s.wrapping_add(i)) {
            Some(v) => {
                if u32::from(v) == c {
                    return s.wrapping_add(i);
                }
                if v == 0 {
                    return 0;
                }
            }
            None => return 0,
        }
        i = i.wrapping_add(1);
    }
}

/// Find `needle` in the guest string `hay`. Zero when it is absent.
fn g_strstr(m: &Memory<'_>, hay: u32, needle: &[u8]) -> u32 {
    if needle.is_empty() {
        return hay;
    }
    let mut i = 0u32;
    loop {
        let at = hay.wrapping_add(i);
        if g_cmp_at(m, at, needle) == 0 {
            return at;
        }
        match g_byte(m, at) {
            Some(0) | None => return 0,
            Some(_) => i = i.wrapping_add(1),
        }
    }
}

/// Compare the guest string at `a` with a Rust literal, byte by byte.
fn g_cmp_at(m: &Memory<'_>, a: u32, b: &[u8]) -> i32 {
    let mut i = 0usize;
    loop {
        let x = g_byte(m, a.wrapping_add(i as u32)).unwrap_or(0);
        let y = b.get(i).copied().unwrap_or(0);
        if x != y {
            return i32::from(x) - i32::from(y);
        }
        if y == 0 {
            return 0;
        }
        i += 1;
    }
}

/// Copy `n` bytes inside the arena.
fn g_memcpy(m: &mut Memory<'_>, dst: u32, src: u32, n: u32) -> bool {
    if !g_range(m, dst, n) || !g_range(m, src, n) {
        return false;
    }
    let s = (src - m.base) as usize;
    let d = (dst - m.base) as usize;
    m.bytes.copy_within(s..s + n as usize, d);
    true
}

/// Move `n` bytes inside the arena.
fn g_memmove(m: &mut Memory<'_>, dst: u32, src: u32, n: u32) -> bool {
    if !g_range(m, dst, n) || !g_range(m, src, n) {
        return false;
    }
    let s = (src - m.base) as usize;
    let d = (dst - m.base) as usize;
    m.bytes.copy_within(s..s + n as usize, d);
    true
}

/// Fill `n` bytes at `p` with `v`.
fn g_memset(m: &mut Memory<'_>, p: u32, v: u8, n: u32) -> bool {
    if !g_range(m, p, n) {
        return false;
    }
    let at = (p - m.base) as usize;
    for i in 0..n as usize {
        m.bytes[at + i] = v;
    }
    true
}

/// The `fn_195e38` copy: `n` bytes of `src` into `dst`, terminator excluded.
fn g_copy_n(m: &mut Memory<'_>, dst: u32, src: u32, n: u32) -> bool {
    g_memmove(m, dst, src, n)
}

/// Call one slot of the `out` vtable. The first two words of every call are
/// the caller's `a` and `c` handles.
fn out_call(
    m: &mut Memory<'_>,
    h: &mut impl Host,
    out: u32,
    slot: u32,
    args: &[u32],
) -> u32 {
    let f = match g_word(m, out.wrapping_add(slot * 4)) {
        Some(v) => v,
        None => return 0x80002001,
    };
    let mut all = [0u32; 12];
    let n = args.len().min(12);
    all[..n].copy_from_slice(&args[..n]);
    h.call_m(f, &all[..n], m)
}

/// Call one slot of the `cfg` vtable. The first two words are the handle.
fn cfg_call(m: &mut Memory<'_>, h: &mut impl Host, cfg: u32, slot: u32, args: &[u32]) -> u32 {
    let f = match g_word(m, cfg.wrapping_add(slot * 4)) {
        Some(v) => v,
        None => return 0x80002001,
    };
    let mut all = [0u32; 12];
    let n = args.len().min(12);
    all[..n].copy_from_slice(&args[..n]);
    h.call_m(f, &all[..n], m)
}

/// The `fn_17c800` character-class test: bit three of the class table.
pub fn fn_17c800_m(class_table: &[u8], c: u32) -> u32 {
    let at = (c & 0xff) as usize * 2;
    if at + 2 > class_table.len() {
        return 0;
    }
    let v = u16::from_le_bytes([class_table[at], class_table[at + 1]]);
    u32::from(v & 8)
}

/// The `fn_21a1fc` marker-record comparator.
///
/// The exact ARM call is `fn_21a1fc(a, b)`: both arguments point at 24-byte
/// records. `a+12` and `b+12` hold the sort key, `a+0` and `b+0` the 16-bit
/// type and `a+4` and `b+4` the tie break. The ROM object returns `~99`
/// (`mvneq r0, #99`), which is -100 as a signed word; the C source also
/// says `-100`.
pub fn fn_21a1fc_m(m: &Memory<'_>, a: u32, b: u32) -> u32 {
    let af = g_word(m, a.wrapping_add(12)).unwrap_or(0);
    let bf = g_word(m, b.wrapping_add(12)).unwrap_or(0);
    if af != bf {
        return af.wrapping_sub(bf);
    }
    let ta = g_half(m, a).unwrap_or(0);
    let tb = g_half(m, b).unwrap_or(0);
    if (ta == 0x24 || ta == 0x15) && tb == 0x22 {
        return (-100i32) as u32;
    }
    if (tb == 0x24 || tb == 0x15) && ta == 0x22 {
        return 100;
    }
    g_word(m, a.wrapping_add(4))
        .unwrap_or(0)
        .wrapping_sub(g_word(m, b.wrapping_add(4)).unwrap_or(0))
}

/// The `fn_2409d4` quicksort over the caller's arena. The comparator is the
/// caller's ROM entry, issued through the host.
///
/// The exact ARM call is `fn_2409d4(base, n, size, cmp)`. The ROM object
/// compares the comparator result as a signed word, so a negative result is
/// an ordinary "less than", not an error.
pub fn fn_2409d4_m(
    m: &mut Memory<'_>,
    h: &mut impl Host,
    base: u32,
    n: u32,
    size: u32,
    cmp: u32,
) -> u32 {
    if n <= 1 {
        return 0;
    }
    let mut mid = n >> 1;
    let mut pivot = base.wrapping_add(mid.wrapping_mul(size));
    let mut i = 0u32;
    let mut j = n - 1;
    while i < j {
        loop {
            let c = h.call_m(cmp, &[base.wrapping_add(i.wrapping_mul(size)), pivot], m);
            if c == FAIL_UNSUPPORTED {
                return c;
            }
            if (c as i32) >= 0 {
                break;
            }
            i = i.wrapping_add(1);
        }
        loop {
            let c = h.call_m(cmp, &[base.wrapping_add(j.wrapping_mul(size)), pivot], m);
            if c == FAIL_UNSUPPORTED {
                return c;
            }
            if (c as i32) <= 0 {
                break;
            }
            j = j.wrapping_sub(1);
        }
        if i < j {
            let pi = base.wrapping_add(i.wrapping_mul(size));
            let pj = base.wrapping_add(j.wrapping_mul(size));
            for k in 0..size {
                let x = g_byte(m, pi.wrapping_add(k)).unwrap_or(0);
                let y = g_byte(m, pj.wrapping_add(k)).unwrap_or(0);
                if !p_byte(m, pi.wrapping_add(k), y) || !p_byte(m, pj.wrapping_add(k), x) {
                    return FAIL_MEM;
                }
            }
            if mid == i {
                mid = j;
                pivot = pj;
            } else if mid == j {
                mid = i;
                pivot = pi;
            }
            i += 1;
            j -= 1;
        } else if i == j {
            i += 1;
            j -= 1;
            break;
        }
    }
    if j != 0 {
        let rc = fn_2409d4_m(m, h, base, j.wrapping_add(1), size, cmp);
        if rc != 0 {
            return rc;
        }
    }
    if i < n - 1 {
        let rc = fn_2409d4_m(m, h, base.wrapping_add(i.wrapping_mul(size)), n - i, size, cmp);
        if rc != 0 {
            return rc;
        }
    }
    0
}

/// The `fn_195d90` string length the ports use for the guest strings.
fn guest_len(m: &Memory<'_>, p: u32) -> u32 {
    g_strlen(m, p).unwrap_or(0)
}

/// The `fn_240b28` fold the replacement pass applies.
fn fn_240b28_m(c: u8) -> u8 {
    crate::component_live_c::fn_240b28(u32::from(c)) as u8
}

/// The status the ports use when a callee is not ported yet.
const FAIL_UNSUPPORTED: u32 = 0x80002001;

/// The `fn_1ed520` replacement-table pass, live over the caller's arena.
///
/// The exact ARM call is `fn_1ed520(ctx, replacements, text, capacity)`.
/// `ctx` is the `fe_dctlkp` object: `ctx+0x64` is the character-class
/// string, `ctx+0x9c` the mode half word and `ctx+0` the owner whose
/// `owner+0x10` is the logger.
pub fn fn_1ed520_m(
    m: &mut Memory<'_>,
    h: &mut impl Host,
    ctx: u32,
    replacements: u32,
    text: u32,
    capacity: u32,
) -> u32 {
    let class = match g_word(m, ctx.wrapping_add(0x64)) {
        Some(v) => v,
        None => return FAIL_MEM,
    };
    let mode = g_half(m, ctx.wrapping_add(0x9c)).unwrap_or(0);
    let total = guest_len(m, text);
    let mut start = 0u32;
    while start < total && g_strchr(m, class, u32::from(g_byte(m, text.wrapping_add(start)).unwrap_or(0))) != 0 {
        start += 1;
    }
    let mut j = total.wrapping_sub(1);
    while (j as i32) > 0
        && g_strchr(m, class, u32::from(g_byte(m, text.wrapping_add(j)).unwrap_or(0))) != 0
    {
        j -= 1;
    }
    if !p_byte(m, text.wrapping_add(j.wrapping_add(1)), 0) {
        return FAIL_MEM;
    }
    if mode != 2 && guest_len(m, text).wrapping_sub(start) > 1 {
        let mut end = guest_len(m, text);
        let mut i = start;
        while i < end {
            let ch = g_byte(m, text.wrapping_add(i)).unwrap_or(0);
            let rep = if replacements != 0 {
                g_word(m, replacements.wrapping_add(u32::from(ch) * 4)).unwrap_or(0)
            } else {
                0
            };
            if rep != 0 {
                let extra = guest_len(m, rep);
                let sum = end.wrapping_add(extra);
                if sum > capacity {
                    let owner = g_word(m, ctx).unwrap_or(0);
                    let log = g_word(m, owner.wrapping_add(0x10)).unwrap_or(0);
                    h.call_m(0x173dc0, &[log, 0x8f8f44, 0xa029, 0], m);
                    return 0x89802007;
                }
                let tail = end.wrapping_sub(i);
                if !g_memmove(m, text.wrapping_add(i.wrapping_add(extra)), text.wrapping_add(i.wrapping_add(1)), tail) {
                    return FAIL_MEM;
                }
                if !g_copy_n(m, text.wrapping_add(i), rep, extra) {
                    return FAIL_MEM;
                }
                end = sum.wrapping_sub(1);
            } else {
                let folded = fn_240b28_m(ch);
                if !p_byte(m, text.wrapping_add(i), folded) {
                    return FAIL_MEM;
                }
            }
            i += 1;
        }
    }
    let tail = guest_len(m, text.wrapping_add(start)).wrapping_add(1);
    if !g_memmove(m, text, text.wrapping_add(start), tail) {
        return FAIL_MEM;
    }
    let _ = h;
    0
}

/// The `fn_1ed954` G2P convert entry, live over the caller's arena.
///
/// The exact ARM call is `fn_1ed954(ctx, handle, text, which, capacity)`.
/// `which` is the caller-owned text buffer the ARM rewrites in place.
pub fn fn_1ed954_m(
    m: &mut Memory<'_>,
    h: &mut impl Host,
    ctx: u32,
    handle: u32,
    text: u32,
    which: u32,
    capacity: u32,
) -> u32 {
    if ctx == 0 || handle != 0xf37f {
        return FAIL_HANDLE;
    }
    if which == 0 {
        return 0x89802007;
    }
    let flag = g_byte(m, ctx.wrapping_add(0x1a4)).unwrap_or(0);
    let name = ctx.wrapping_add(0x104);
    if flag != 0 && g_strcmp(m, text, name) != 0 {
        let rc = fn_1eb978_m(m, h, ctx, handle, text);
        if rc >= 0x80000000 {
            return rc;
        }
        g_strcpy(m, name, text);
    }
    let mut repl = g_word(m, ctx.wrapping_add(0x1b4)).unwrap_or(0);
    if repl == 0 {
        repl = g_word(m, ctx.wrapping_add(0xac)).unwrap_or(0);
    }
    fn_1ed520_m(m, h, ctx, repl, which, capacity)
}

/// The `fn_1eb978` canonical-word conversion. The ROM unit has no source
/// file; the port only covers the path the capture exercises.
fn fn_1eb978_m(m: &mut Memory<'_>, h: &mut impl Host, ctx: u32, handle: u32, text: u32) -> u32 {
    let _ = (m, h, ctx, handle, text);
    FAIL_UNSUPPORTED
}

/// The `fn_195d90` string length, live over the caller's arena.
///
/// The exact ARM call is `fn_195d90(s)`: the length of the NUL-terminated
/// guest string at `s`, terminator excluded. Lane b's `LiveHost` serves
/// this entry by slicing the terminator off and then scanning the slice
/// for one, so it panics on every input; the b3 host serves the entry
/// locally instead.
pub fn fn_195d90_m(m: &Memory<'_>, s: u32) -> Option<u32> {
    g_strlen(m, s)
}

/// The `fn_195d58` string compare, live over the caller's arena.
///
/// The exact ARM call is `fn_195d58(a, b)`: zero when the two
/// NUL-terminated guest strings are equal, nonzero otherwise. Lane b's
/// `LiveHost` compares the slices with the terminators cut off, so equal
/// strings, prefixes and empty literals panic there; the b3 host serves
/// the entry locally instead.
pub fn fn_195d58_m(m: &Memory<'_>, a: u32, b: u32) -> Option<u32> {
    let alen = g_strlen(m, a)?;
    let blen = g_strlen(m, b)?;
    if !g_range(m, a, alen.wrapping_add(1)) || !g_range(m, b, blen.wrapping_add(1)) {
        return None;
    }
    Some(g_strcmp(m, a, b) as u32)
}

/// The `fn_1ec7cc` dictionary Lookup entry, live over the caller's arena.
///
/// The exact ARM call is `fn_1ec7cc(ctx, handle, key, text, out, count,
/// separator, total)`. The ROM is a thin wrapper over `fn_1ea4e0`
/// (src/fn_1ec7cc.c): a `total` of one tail-calls `fn_1ea4e0` with the same
/// seven words (the ARM trace shows the `0x1ec7cc` entry immediately
/// followed by `0x1ea4e0` with identical arguments); a larger total repeats
/// the lookup with the numbered key and stops on the first error or the
/// first nonzero count. The lookup itself is lane b4's certified
/// `fn_1ea4e0_m`. `scratch` is a caller-owned frame of 260 bytes the
/// numbered path uses for the key buffer.
pub fn fn_1ec7cc_m(
    m: &mut Memory<'_>,
    h: &mut impl Host,
    scratch: u32,
    ctx: u32,
    handle: u32,
    key: u32,
    text: u32,
    out: u32,
    count: u32,
    separator: u32,
    total: u32,
) -> u32 {
    use crate::component_live_b4::fn_1ea4e0_m;
    if total == 1 {
        return fn_1ea4e0_m(m, h, ctx, handle, key, text, out, count, separator);
    }
    let Some(saved) = g_half(m, count) else {
        return FAIL_MEM;
    };
    let mut rc = 0u32;
    let mut i = 0u32;
    while i < total {
        if !g_strcpy(m, scratch, key) {
            return FAIL_MEM;
        }
        let num = scratch.wrapping_add(256);
        h.call_m(0x176ac8, &[i, num, 10], m);
        let base = g_strlen(m, scratch).unwrap_or(0);
        let n = g_strlen(m, num).unwrap_or(0);
        if !g_copy_n(m, scratch.wrapping_add(base), num, n.wrapping_add(1)) {
            return FAIL_MEM;
        }
        if !p_half(m, count, saved) {
            return FAIL_MEM;
        }
        rc = fn_1ea4e0_m(m, h, ctx, handle, scratch, text, out, count, separator);
        if rc >= 0x80000000 {
            return rc;
        }
        if g_half(m, count).unwrap_or(0) != 0 {
            return rc;
        }
        i = i.wrapping_add(1);
    }
    rc
}

/// The `fn_26ff18` G2P lookup, live over the caller's arena.
///
/// The exact ARM call is
/// `fn_26ff18(ctx, text, word, orig, dict, domain, start, rec, recObj,
///            recData, end, tokenCheck, langOnly)`.
///
/// `ctx` is the one-word object: `ctx+0x00` the environment (`env+0x10` the
/// logger and `env+0x14` the property table), `ctx+0x0c` the record store
/// vtable, `ctx+0x10`/`ctx+0x14` the dictionary handle, `ctx+0x18` the
/// dictionary vtable, `ctx+0x50` the text offset, `ctx+0x172` the lookup
/// flag, `ctx+0x1a5` the English flag, `ctx+0x2a8` the language check,
/// `ctx+0x2ac` the language list, `ctx+0x2b0` the language key and
/// `ctx+0x2b4` the language-domain flag.  `scratch` is a caller-owned frame
/// of 64 bytes that holds the lookup results and the record scratch.
///
/// The ROM's literals live in the ROM image the caller owns: the property
/// key `sysdctlookup` at 0x2702ac, the empty string at 0x2702b8, `no` at
/// 0x2702bc, the log formats at 0x2702c0 and later and the logger module
/// at 0x8f9532.
pub fn fn_26ff18_m(
    m: &mut Memory<'_>,
    h: &mut impl Host,
    scratch: u32,
    ctx: u32,
    text: u32,
    word: u32,
    orig: u32,
    dict: u32,
    domain: u32,
    start: u16,
    rec: u16,
    rec_obj: u32,
    rec_data: u32,
    end: u32,
    token_check: u32,
    lang_only: u32,
) -> u32 {
    /// The property value pointer.
    const S_VALUE: u32 = 0x00;
    /// The lookup result array.
    const S_WORDS: u32 = 0x04;
    /// The lookup result count.
    const S_COUNT: u32 = 0x08;
    /// The lookup separator.
    const S_SEP: u32 = 0x0a;
    /// The record status.
    const S_STATUS: u32 = 0x0c;
    /// The record type value.
    const S_FOUR: u32 = 0x0e;
    /// The record priority.
    const S_PRIO: u32 = 0x10;
    /// The record id.
    const S_REC: u32 = 0x12;
    /// The record start.
    const S_START: u32 = 0x14;
    /// The record end.
    const S_END: u32 = 0x16;
    /// The language tag character.
    const S_LANGC: u32 = 0x18;
    /// The lookup domain name.
    const S_NAME: u32 = 0x20;

    let env = g_word(m, ctx).unwrap_or(0);
    let cfg = g_word(m, env.wrapping_add(0x14)).unwrap_or(0);
    let log = g_word(m, env.wrapping_add(0x10)).unwrap_or(0);
    let rec_vt = g_word(m, ctx.wrapping_add(0x0c)).unwrap_or(0);
    let handle = g_word(m, ctx.wrapping_add(0x10)).unwrap_or(0);
    let handle_tag = g_word(m, ctx.wrapping_add(0x14)).unwrap_or(0);
    let lex_vt = g_word(m, ctx.wrapping_add(0x18)).unwrap_or(0);
    let text_off = g_half(m, ctx.wrapping_add(0x50)).unwrap_or(0) as i16;
    let lex_flag = g_byte(m, ctx.wrapping_add(0x172)).unwrap_or(0);
    let use_lang_domain = g_byte(m, ctx.wrapping_add(0x2b4)).unwrap_or(0);

    // The dictionary is switched off when the property says so.
    let rc = h.call_m(0x175a1c, &[cfg, 0x2702ac, scratch.wrapping_add(S_VALUE)], m);
    if rc < 0x80000000 {
        let value = g_word(m, scratch.wrapping_add(S_VALUE)).unwrap_or(0);
        if h.call_m(0x195d58, &[value, 0x2702bc], m) == 0 {
            return 0;
        }
    }
    h.call_m(0x1738e8, &[log, 0x8f9532, 5, 0, 0x2702c0, word], m);

    // The lookup: the language domain variant uses slot seven, the plain
    // one slot eighteen. The ROM passes a fresh zeroed `count` local, so
    // the scratch count is reset on every call: a reused scratch would
    // otherwise steer the lookup into the numbered-key path.
    if !p_half(m, scratch.wrapping_add(S_COUNT), 0) {
        return FAIL_MEM;
    }
    let ret = if use_lang_domain != 0 {
        h.call_m(0x195cd0, &[scratch.wrapping_add(S_NAME), 0x2702d4], m);
        let index = u32::from(g_byte(m, word).unwrap_or(0));
        let langs = ctx
            .wrapping_add(0x2b5)
            .wrapping_add(index.wrapping_mul(3));
        h.call_m(0x195c90, &[scratch.wrapping_add(S_NAME), langs], m);
        let f = g_word(m, lex_vt.wrapping_add(7 * 4)).unwrap_or(0);
        h.call_m(
            f,
            &[
                handle,
                handle_tag,
                scratch.wrapping_add(S_NAME),
                word,
                scratch.wrapping_add(S_WORDS),
                scratch.wrapping_add(S_COUNT),
                scratch.wrapping_add(S_SEP),
            ],
            m,
        )
    } else {
        let f = g_word(m, lex_vt.wrapping_add(18 * 4)).unwrap_or(0);
        h.call_m(
            f,
            &[
                handle,
                handle_tag,
                dict,
                word,
                scratch.wrapping_add(S_WORDS),
                scratch.wrapping_add(S_COUNT),
                scratch.wrapping_add(S_SEP),
                u32::from(lex_flag),
            ],
            m,
        )
    };
    if ret >= 0x80000000 {
        return ret;
    }

    let count = g_half(m, scratch.wrapping_add(S_COUNT)).unwrap_or(0);
    let sep = u32::from(g_byte(m, scratch.wrapping_add(S_SEP)).unwrap_or(0));
    let words = g_word(m, scratch.wrapping_add(S_WORDS)).unwrap_or(0);
    let mut prio = 1u16;
    let mut langc = 0u8;
    let mut status = ret;

    // The record id chains across the words: the C passes its `rec` local
    // by value and by address (`newRecord(recObj, recData, rec, &rec)`),
    // so each word clones from the previous word's new id. The cell is
    // primed once here and never reset in the loop (src/fn_26ff18.c:291).
    if !p_half(m, scratch.wrapping_add(S_REC), rec) {
        return FAIL_MEM;
    }

    for index in 0..count {
        let w = g_word(m, words.wrapping_add(u32::from(index) * 4)).unwrap_or(0);
        let mut phon = 0u32;
        let mut dom = 0u32;
        let mut lang = 0u32;
        let mut pos = 0u32;
        let mut gpos = 0u32;
        let mut prio_s = 0u32;
        // The separator splits the record into its fields.
        let mut p = h.call_m(0x195d30, &[w, sep], m);
        if p != 0 {
            pos = p.wrapping_add(1);
            p_byte(m, p, 0);
            p = h.call_m(0x195d30, &[pos, sep], m);
            if p != 0 {
                p_byte(m, p, 0);
                dom = p.wrapping_add(1);
                p = h.call_m(0x195d30, &[dom, sep], m);
                if p != 0 {
                    phon = p.wrapping_add(1);
                    p_byte(m, p, 0);
                    p = h.call_m(0x195d30, &[phon, sep], m);
                    if p != 0 {
                        gpos = p.wrapping_add(1);
                        p_byte(m, p, 0);
                        p = h.call_m(0x195d30, &[gpos, sep], m);
                        if p != 0 {
                            prio_s = p.wrapping_add(1);
                            p_byte(m, p, 0);
                            p = h.call_m(0x195d30, &[prio_s, sep], m);
                            if p != 0 {
                                p_byte(m, p, 0);
                                let f6 = p.wrapping_add(1);
                                p = h.call_m(0x195d30, &[f6, sep], m);
                                if p != 0 {
                                    p_byte(m, p, 0);
                                    lang = p.wrapping_add(1);
                                    p = h.call_m(0x195d30, &[lang, sep], m);
                                    if p != 0 {
                                        p_byte(m, p, 0);
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
        if w == 0 {
            h.call_m(0x173dc0, &[log, 0x8f9532, 0x84d1, 0], m);
            return 0x89b02007;
        }
        h.call_m(0x1738e8, &[log, 0x8f9532, 5, 0, 0x2702d8, w], m);
        // The domain of the record has to match the requested one.
        if dom != 0 && h.call_m(0x195d58, &[dom, 0x2702b8], m) != 0 {
            if h.call_m(0x1960a8, &[dom, domain], m) == 0 {
                h.call_m(0x1738e8, &[log, 0x8f9532, 5, 0, 0x2705ec], m);
                continue;
            }
        }
        if g_byte(m, ctx.wrapping_add(0x2a8)).unwrap_or(0) != 0 {
            let lang_key = g_word(m, ctx.wrapping_add(0x2b0)).unwrap_or(0);
            let lang_list = g_word(m, ctx.wrapping_add(0x2ac)).unwrap_or(0);
            let mut skip = false;
            if lang == 0 || h.call_m(0x195d30, &[lang, 0x21], m) != 0 {
                // The ROM writes the fourth byte of the language tag.
                p_byte(m, lang.wrapping_add(3), 0);
            } else {
                let found = h.call_m(0x1960a8, &[domain, lang], m);
                let len = h.call_m(0x195d90, &[lang], m);
                let bad = found == 0
                    || (len != 0 && g_byte(m, found.wrapping_add(len)).unwrap_or(0) != 0);
                if lang_only != 0 {
                    if token_check == 0 && bad && h.call_m(0x1769d0, &[lang, lang_key], m) != 0 {
                        h.call_m(0x1738e8, &[log, 0x8f9532, 5, 0, 0x27060c, lang], m);
                        skip = true;
                    }
                } else if token_check == 0 && bad {
                    let user = h.call_m(0x1960a8, &[lang_list, domain], m);
                    let domain_len = h.call_m(0x195d90, &[domain], m);
                    let mark = if user != 0 && domain_len != 0 {
                        g_byte(m, user.wrapping_add(domain_len)).unwrap_or(0)
                    } else {
                        0
                    };
                    if h.call_m(0x1769d0, &[lang, lang_key], m) != 0 || user == 0 || mark != 0x7c {
                        h.call_m(0x1738e8, &[log, 0x8f9532, 5, 0, 0x270630, lang], m);
                        skip = true;
                    }
                }
            }
            if !skip && token_check != 0 && lang != 0 {
                let found = h.call_m(0x1960a8, &[domain, lang], m);
                let len = h.call_m(0x195d90, &[lang], m);
                let bad = found == 0
                    || (len != 0 && g_byte(m, found.wrapping_add(len)).unwrap_or(0) != 0);
                if bad {
                    let user = h.call_m(0x1960a8, &[lang_list, domain], m);
                    let domain_len = h.call_m(0x195d90, &[domain], m);
                    let mark = if user != 0 && domain_len != 0 {
                        g_byte(m, user.wrapping_add(domain_len)).unwrap_or(0)
                    } else {
                        0
                    };
                    if h.call_m(0x1769d0, &[lang, lang_key], m) != 0 || user == 0 || mark != 0x7c {
                        h.call_m(0x1738e8, &[log, 0x8f9532, 5, 0, 0x270664, lang], m);
                        skip = true;
                    }
                }
            }
            if skip {
                continue;
            }
        }

        // The record: the end offset, the new record and its first fields.
        let len = h.call_m(0x195d90, &[orig], m);
        let end_value = u16::wrapping_add(start, len as u16);
        if !p_half(m, end, end_value) {
            return FAIL_MEM;
        }
        let new_record = g_word(m, rec_vt.wrapping_add(11 * 4)).unwrap_or(0);
        let rc = h.call_m(
            new_record,
            &[
                rec_obj,
                rec_data,
                u32::from(g_half(m, scratch.wrapping_add(S_REC)).unwrap_or(rec)),
                scratch.wrapping_add(S_REC),
            ],
            m,
        );
        if rc >= 0x80000000 {
            return rc;
        }
        let set_field = g_word(m, rec_vt.wrapping_add(20 * 4)).unwrap_or(0);
        if !p_byte(m, scratch.wrapping_add(S_FOUR), 4) {
            return FAIL_MEM;
        }
        if !p_half(m, scratch.wrapping_add(S_START), start) {
            return FAIL_MEM;
        }
        let rc = h.call_m(
            set_field,
            &[
                rec_obj,
                rec_data,
                u32::from(g_half(m, scratch.wrapping_add(S_REC)).unwrap_or(rec)),
                0,
                1,
                scratch.wrapping_add(S_FOUR),
                scratch.wrapping_add(S_STATUS),
            ],
            m,
        );
        if rc >= 0x80000000 {
            return rc;
        }
        let rc = h.call_m(
            set_field,
            &[
                rec_obj,
                rec_data,
                u32::from(g_half(m, scratch.wrapping_add(S_REC)).unwrap_or(rec)),
                1,
                1,
                scratch.wrapping_add(S_START),
                scratch.wrapping_add(S_STATUS),
            ],
            m,
        );
        if rc >= 0x80000000 {
            return rc;
        }
        let rc = h.call_m(
            set_field,
            &[
                rec_obj,
                rec_data,
                u32::from(g_half(m, scratch.wrapping_add(S_REC)).unwrap_or(rec)),
                2,
                1,
                end,
                scratch.wrapping_add(S_STATUS),
            ],
            m,
        );
        if rc >= 0x80000000 {
            return rc;
        }

        // The spaces of the word become dashes.
        let base = g_word(m, text).unwrap_or(0);
        let limit = base
            .wrapping_add(u32::from(g_half(m, end).unwrap_or(0)))
            .wrapping_add(text_off as u32);
        let mut p = base
            .wrapping_add(text_off as u32)
            .wrapping_add(u32::from(start));
        let mut q = h.call_m(0x195d30, &[p, 0x20], m);
        if q == 0 || q >= limit {
            q = h.call_m(0x195d30, &[p, 0x5f], m);
        }
        while q != 0 && q < limit {
            p = q;
            p_byte(m, q, 0x2d);
            q = h.call_m(0x195d30, &[q, 0x20], m);
            if q == 0 || q >= limit {
                q = h.call_m(0x195d30, &[p, 0x5f], m);
            }
        }

        // The word and its parts.
        let mut p = h.call_m(0x195d30, &[w, 0x12], m);
        if p != 0 {
            p = p.wrapping_add(1);
            p_byte(m, p.wrapping_sub(1), 0);
            let n = h.call_m(0x195d90, &[p], m).wrapping_add(1) as u16;
            let rc = h.call_m(
                set_field,
                &[
                    rec_obj,
                    rec_data,
                    u32::from(g_half(m, scratch.wrapping_add(S_REC)).unwrap_or(rec)),
                    11,
                    u32::from(n),
                    p,
                    scratch.wrapping_add(S_STATUS),
                ],
                m,
            );
            if rc >= 0x80000000 {
                return rc;
            }
        }
        let wlen = h.call_m(0x195d90, &[w], m).wrapping_add(1) as u16;
        let rc = h.call_m(
            set_field,
            &[
                rec_obj,
                rec_data,
                u32::from(g_half(m, scratch.wrapping_add(S_REC)).unwrap_or(rec)),
                3,
                u32::from(wlen),
                w,
                scratch.wrapping_add(S_STATUS),
            ],
            m,
        );
        if rc >= 0x80000000 {
            return rc;
        }
        if phon != 0 {
            let empty = h.call_m(0x195d58, &[phon, 0x270608], m) == 0;
            let (n, value) = if empty {
                (2u32, 0x270af0u32)
            } else {
                (h.call_m(0x195d90, &[phon], m).wrapping_add(1), phon)
            };
            let rc = h.call_m(
                set_field,
                &[
                    rec_obj,
                    rec_data,
                    u32::from(g_half(m, scratch.wrapping_add(S_REC)).unwrap_or(rec)),
                    5,
                    n,
                    value,
                    scratch.wrapping_add(S_STATUS),
                ],
                m,
            );
            if rc >= 0x80000000 {
                return rc;
            }
        }
        if gpos != 0 {
            let empty = h.call_m(0x195d58, &[gpos, 0x270608], m) == 0;
            let (n, value) = if empty {
                (2u32, 0x270af0u32)
            } else {
                (h.call_m(0x195d90, &[gpos], m).wrapping_add(1), gpos)
            };
            let rc = h.call_m(
                set_field,
                &[
                    rec_obj,
                    rec_data,
                    u32::from(g_half(m, scratch.wrapping_add(S_REC)).unwrap_or(rec)),
                    6,
                    n,
                    value,
                    scratch.wrapping_add(S_STATUS),
                ],
                m,
            );
            if rc >= 0x80000000 {
                return rc;
            }
        }
        if pos != 0 {
            let n = h.call_m(0x195d90, &[pos], m).wrapping_add(1);
            let rc = h.call_m(
                set_field,
                &[
                    rec_obj,
                    rec_data,
                    u32::from(g_half(m, scratch.wrapping_add(S_REC)).unwrap_or(rec)),
                    4,
                    n,
                    pos,
                    scratch.wrapping_add(S_STATUS),
                ],
                m,
            );
            if rc >= 0x80000000 {
                return rc;
            }
        }
        prio = if prio_s != 0 {
            h.call_m(0x17c550, &[prio_s], m) as u16
        } else {
            1
        };
        if lang != 0 {
            let phon = h.call_m(0x1960a8, &[domain, lang], m);
            langc = g_byte(m, lang).unwrap_or(0);
            let lang_len = h.call_m(0x195d90, &[lang], m).wrapping_add(1);
            let rc = h.call_m(
                set_field,
                &[
                    rec_obj,
                    rec_data,
                    u32::from(g_half(m, scratch.wrapping_add(S_REC)).unwrap_or(rec)),
                    9,
                    lang_len,
                    lang,
                    scratch.wrapping_add(S_STATUS),
                ],
                m,
            );
            h.call_m(0x1738e8, &[log, 0x8f9532, 5, 0, 0x270af4, lang], m);
            let user = if g_word(m, ctx.wrapping_add(0x2ac)).unwrap_or(0) != 0 {
                h.call_m(
                    0x1960a8,
                    &[g_word(m, ctx.wrapping_add(0x2ac)).unwrap_or(0), domain],
                    m,
                )
            } else {
                0
            };
            if g_byte(m, ctx.wrapping_add(0x2a8)).unwrap_or(0) != 0 {
                let len = h.call_m(0x195d90, &[lang], m);
                let tail = if len != 0 {
                    g_byte(m, phon.wrapping_add(len)).unwrap_or(0)
                } else {
                    0
                };
                let domain_len = h.call_m(0x195d90, &[domain], m);
                let mark = if domain_len != 0 {
                    g_byte(m, user.wrapping_add(domain_len)).unwrap_or(0)
                } else {
                    0
                };
                let same = h.call_m(
                    0x1769d0,
                    &[lang, g_word(m, ctx.wrapping_add(0x2b0)).unwrap_or(0)],
                    m,
                ) == 0;
                if (phon != 0 && (len == 0 || tail == 0)) || (same && user != 0 && mark == 0x7c)
                {
                    prio = prio.wrapping_add(1);
                    h.call_m(0x1738e8, &[log, 0x8f9532, 5, 0, 0x270b08, u32::from(prio)], m);
                }
            }
            if rc >= 0x80000000 {
                return rc;
            }
        } else {
            langc = 0;
            let rc = h.call_m(
                set_field,
                &[
                    rec_obj,
                    rec_data,
                    u32::from(g_half(m, scratch.wrapping_add(S_REC)).unwrap_or(rec)),
                    9,
                    1,
                    0x270b24,
                    scratch.wrapping_add(S_STATUS),
                ],
                m,
            );
            if rc >= 0x80000000 {
                return rc;
            }
        }
        if !p_half(m, scratch.wrapping_add(S_PRIO), prio) {
            return FAIL_MEM;
        }
        let rc = h.call_m(
            set_field,
            &[
                rec_obj,
                rec_data,
                u32::from(g_half(m, scratch.wrapping_add(S_REC)).unwrap_or(rec)),
                7,
                1,
                scratch.wrapping_add(S_PRIO),
                scratch.wrapping_add(S_STATUS),
            ],
            m,
        );
        if rc >= 0x80000000 {
            return rc;
        }
        status = rc;
        if count == 1 {
            if langc == b'e' {
                p_byte(m, ctx.wrapping_add(0x1a5), 1);
            } else if langc != 0 {
                p_byte(m, ctx.wrapping_add(0x1a5), 0);
            }
        }
    }
    status
}

/// Caller-owned scratch for `fn_275204_m`: the 256-byte key buffer, the
/// section/expected strings and the getString out-cells. Sits in the
/// excluded guest-stack window 0x0e000000..0x0e100000 (never compared;
/// 0x0e0f9000 is free: broker family ends at 0x0e0fd580, depes at
/// 0x0e0fda00, seg2-c4 at 0x0e0f8000+0x1000), saved/restored on return.
pub const FE275204_SCRATCH: u32 = 0x0e0f_9000;
const FE275204_LEN: usize = 0x400;

/// `fn_275204`: morphology limits/flags for one mode (src/fn_275204.c 1:1).
///
/// The exact ARM call is `fn_275204(fe, mode)`. Per field it builds
/// `prefix + mode` on its stack, issues the config `getString` through the
/// vtable the ROM reads out of `fe` (`cfg` at `+0x18`, handle pair at
/// `+0x10/+0x14`), falls back to the bare prefix on empty, truncates at
/// the separator and parses. Key building, the separator scan, the integer
/// parses and the string compares are pure stack/arena ops, done here over
/// the guest bytes; only the per-field `getString` rides the host, with
/// the same `(pair, section, key, &val, &len, &sep)` addresses the ARM
/// passes. `scratch` is the caller-owned window above, restored on return.
pub fn fn_275204_m(
    m: &mut Memory<'_>,
    h: &mut impl Host,
    fe: u32,
    mode: u32,
    scratch: u32,
) -> u32 {
    use crate::component_live_b4::fn_1ea4e0_m;
    const PREFIXES: [&[u8]; 8] = [
        b"mdeminwrdlen_",
        b"mdemaxwrdlen_",
        b"mdemaxnrmorph_",
        b"mdefirstletter_",
        b"mdetype_",
        b"mdeemptyafx_",
        b"mdenondctdps_",
        b"mdedps_",
    ];
    // (fe offset, parse): parse 0 = integer, 1 = "upper", 2 = "stemafx",
    // 3/4/5 = "yes" (emptyafx, nonctdps, dps).
    const FIELDS: [(u32, u8); 8] = [
        (0x6a, 0),
        (0x6b, 0),
        (0x6d, 0),
        (0x6c, 1),
        (0x6e, 2),
        (0x6f, 3),
        (0x70, 4),
        (0x71, 5),
    ];
    const KEY: u32 = 0;
    const SEC: u32 = 0x100;
    const EXP_UPPER: u32 = 0x108;
    const EXP_STEM: u32 = 0x110;
    const EXP_YES: u32 = 0x11c;
    const VALC: u32 = 0x180;
    const LENC: u32 = 0x184;
    const SEPC: u32 = 0x186;
    if !g_range(m, scratch, FE275204_LEN as u32)
        || !g_range(m, fe, 0x72)
    {
        return FAIL_MEM;
    }
    let base = match scratch.checked_sub(m.base) {
        Some(v) => v as usize,
        None => return FAIL_MEM,
    };
    let mut saved = [0u8; FE275204_LEN];
    saved.copy_from_slice(&m.bytes[base..base + FE275204_LEN]);
    let result = (|| {
        // Defaults from the C head: maxNrMorph 0x10, dps 1, rest 0.
        for (off, v) in [(0x6au32, 0u8), (0x6b, 0), (0x6c, 0), (0x6d, 0x10), (0x6e, 0), (0x6f, 0), (0x70, 0), (0x71, 1)] {
            if !p_byte(m, fe.wrapping_add(off), v) {
                return FAIL_MEM;
            }
        }
        // Mode string (NUL-bounded, like the strcat source).
        let mut mode_buf = [0u8; 128];
        let mut mode_len = 0usize;
        while mode_len < 128 {
            match g_byte(m, mode.wrapping_add(mode_len as u32)) {
                Some(0) => break,
                Some(c) => {
                    mode_buf[mode_len] = c;
                    mode_len += 1;
                }
                None => return FAIL_MEM,
            }
        }
        if mode_len >= 128 {
            return FAIL_MEM;
        }
        let cfg = match g_word(m, fe.wrapping_add(0x18)) {
            Some(v) => v,
            None => return FAIL_MEM,
        };
        let method = match g_word(m, cfg.wrapping_add(0x1c)) {
            Some(v) => v,
            None => return FAIL_MEM,
        };
        let pair0 = match g_word(m, fe.wrapping_add(0x10)) {
            Some(v) => v,
            None => return FAIL_MEM,
        };
        let pair1 = match g_word(m, fe.wrapping_add(0x14)) {
            Some(v) => v,
            None => return FAIL_MEM,
        };
        // Section + expected strings (ROM literals in the C).
        let sec = scratch.wrapping_add(SEC);
        let exp_upper = scratch.wrapping_add(EXP_UPPER);
        let exp_stem = scratch.wrapping_add(EXP_STEM);
        let exp_yes = scratch.wrapping_add(EXP_YES);
        let key = scratch.wrapping_add(KEY);
        let valc = scratch.wrapping_add(VALC);
        let lenc = scratch.wrapping_add(LENC);
        let sepc = scratch.wrapping_add(SEPC);
        for (at, s) in [(sec, b"fecfg\0".as_slice()), (exp_upper, b"upper\0".as_slice()), (exp_stem, b"stemafx\0".as_slice()), (exp_yes, b"yes\0".as_slice())] {
            let at0 = match at.checked_sub(m.base) {
                Some(v) => v as usize,
                None => return FAIL_MEM,
            };
            if at0 + s.len() > m.bytes.len() {
                return FAIL_MEM;
            }
            m.bytes[at0..at0 + s.len()].copy_from_slice(s);
        }
        let mut rc = 0u32;
        for (i, prefix) in PREFIXES.iter().enumerate() {
            let (off, parse) = FIELDS[i];
            // key = prefix + mode (fn_195cd0 + fn_195c90 on the stack).
            if prefix.len() + mode_len + 1 > 256 {
                return FAIL_MEM;
            }
            let key0 = match key.checked_sub(m.base) {
                Some(v) => v as usize,
                None => return FAIL_MEM,
            };
            m.bytes[key0..key0 + prefix.len()].copy_from_slice(prefix);
            m.bytes[key0 + prefix.len()..key0 + prefix.len() + mode_len]
                .copy_from_slice(&mode_buf[..mode_len]);
            m.bytes[key0 + prefix.len() + mode_len] = 0;
            if !p_half(m, lenc, 0xffff) {
                return FAIL_MEM;
            }
            // cfg getString(h, "fecfg", key): the 1f98c8 prologue recipe:
            // lane b4's port when the vtable names 0x1ea4e0, else the host.
            if method == 0x1ea4e0 {
                rc = fn_1ea4e0_m(m, h, pair0, pair1, sec, key, valc, lenc, sepc);
            } else {
                rc = h.call_m(method, &[pair0, pair1, sec, key, valc, lenc, sepc], m);
            }
            if rc >= 0x80000000 {
                return rc;
            }
            let mut len = match g_half(m, lenc) {
                Some(v) => v,
                None => return FAIL_MEM,
            };
            if len == 0 {
                // Fallback: bare prefix (the C's literal second call).
                let bare = &prefix[..prefix.len() - 1];
                m.bytes[key0..key0 + bare.len()].copy_from_slice(bare);
                m.bytes[key0 + bare.len()] = 0;
                if !p_half(m, lenc, 0xffff) {
                    return FAIL_MEM;
                }
                if method == 0x1ea4e0 {
                    rc = fn_1ea4e0_m(m, h, pair0, pair1, sec, key, valc, lenc, sepc);
                } else {
                    rc = h.call_m(method, &[pair0, pair1, sec, key, valc, lenc, sepc], m);
                }
                if rc >= 0x80000000 {
                    return rc;
                }
                len = match g_half(m, lenc) {
                    Some(v) => v,
                    None => return FAIL_MEM,
                };
                if len == 0 {
                    continue;
                }
            }
            let val = match g_word(m, valc) {
                Some(v) => v,
                None => return FAIL_MEM,
            };
            if val == 0 {
                return FAIL_MEM;
            }
            let sep = match g_byte(m, sepc) {
                Some(v) => v,
                None => return FAIL_MEM,
            };
            // Truncate at the separator (fn_195d30 + *s = 0).
            let mut e = val;
            loop {
                match g_byte(m, e) {
                    Some(0) => break,
                    Some(c) if c == sep => {
                        let at0 = match e.checked_sub(m.base) {
                            Some(v) => v as usize,
                            None => return FAIL_MEM,
                        };
                        m.bytes[at0] = 0;
                        break;
                    }
                    Some(_) => {
                        e = e.wrapping_add(1);
                        if e.wrapping_sub(val) > 256 {
                            return FAIL_MEM;
                        }
                    }
                    None => return FAIL_MEM,
                }
            }
            let dst = fe.wrapping_add(off);
            if parse == 0 {
                let at0 = match val.checked_sub(m.base) {
                    Some(v) => v as usize,
                    None => return FAIL_MEM,
                };
                let n = crate::pronunciation::parse_int(&m.bytes[at0..]);
                if !p_byte(m, dst, n as u8) {
                    return FAIL_MEM;
                }
            } else {
                let exp = match parse {
                    1 => exp_upper,
                    2 => exp_stem,
                    _ => exp_yes,
                };
                // fn_195d58(val, expected).
                let mut k = 0u32;
                let eq = loop {
                    let a = match g_byte(m, val.wrapping_add(k)) {
                        Some(v) => v,
                        None => return FAIL_MEM,
                    };
                    let b = match g_byte(m, exp.wrapping_add(k)) {
                        Some(v) => v,
                        None => return FAIL_MEM,
                    };
                    if a != b {
                        break false;
                    }
                    if a == 0 {
                        break true;
                    }
                    k += 1;
                    if k > 256 {
                        return FAIL_MEM;
                    }
                };
                if parse == 2 {
                    // mdetype: only the "stemafx" arm writes.
                    if eq && !p_byte(m, dst, 1) {
                        return FAIL_MEM;
                    }
                } else if !p_byte(m, dst, if eq { 1 } else { 0 }) {
                    return FAIL_MEM;
                }
            }
        }
        rc
    })();
    m.bytes[base..base + FE275204_LEN].copy_from_slice(&saved);
    result
}

/// The `fn_275928` dictionary multi-word path, live over the caller's arena.
///
/// The exact ARM call is
/// `fn_275928(fe, text, start, end, mode, handle, tok, markers, mi,
/// nMarkers, base, rich)` where `handle` is a two-word `TtsHandle`
/// `(obj, data)` and `text` is the `char **` of the caller text.
/// `scratch` is a caller-owned frame: the word buffers live at
/// `scratch + 0x604` and the address-taken scalars at the top; nested
/// `fn_26ff18` calls use `scratch + FN_275928_N26` (64 bytes) and nested
/// `fn_1ec7cc` calls use `scratch + FN_275928_NEC` (260 bytes). The token
/// and dictionary APIs are the direct function-pointer structs the ROM
/// reads out of `fe` (`token_api` at `+0x0c`, dictionary `obj/data` at
/// `+0x10/+0x14`, `dictionary_api` at `+0x18`); every other callee is
/// issued through the host. The ROM literals used are `prompt` at
/// 0x2760d8, `femwg2p` at 0x2760e0, `g2p` at 0x2760e4, `tokenType` at
/// 0x276750, `token` at 0x27675c, the log format at 0x276764, `*` at
/// 0x276778 and `word` at 0x276c48; the empty string is the shared
/// 0x2702b8.
pub const FN_275928_FRAME_END: u32 = 0xa64;
pub const FN_275928_NEC: u32 = 0x4c0;
pub const FN_275928_N26: u32 = 0x5c4;
/// Measured 1f98c8 entry SP, identical on all seven e2e workloads
/// (scratch/c3-r20/sp-probe.py hooks 1f98c8's entry: 0xe0ff938 on
/// speech_1/2/3/4, custom_1/2 and sing_1). The ARM frame chain below
/// derives every nested SP arithmetically: 1f98c8's body is entry minus
/// its 0x2E8 frame (`push` 13 regs + `sub sp, #0x2b4`), 275928's body its
/// entry minus 0x4E0 (`push` 13 + `vpush` 3 regs + `sub` 0x400 + 0x94),
/// each G2P call's body minus its own frame, each 2c8554 level minus
/// 0x158 (`push` 9 regs + `sub sp, #0x134`). No fixed windows, no depth
/// cap: every frame position is computed from this live anchor.
pub const SP_1F98C8_ENTRY: u32 = 0xe0ff938;
/// 1f98c8's own frame size (`push {r0-r3, r4-r8, sb, sl, fp, lr}` +
/// `sub sp, #0x2b4`), verified against the balanced epilogue.
pub const F1F98C8_FRAME: u32 = 0x2e8;
/// 275928's own frame size (`push` 13 regs + `vpush {d8, d9, d10}` +
/// `sub sp, #0x400` + `sub sp, #0x94`).
pub const F275928_FRAME: u32 = 0x4e0;
const F275928_WORDS: u32 = 0x604;
const F275928_TOKEN_TEXT: u32 = 0x704;
const F275928_NTOKEN: u32 = 0x784;
const F275928_NWORDS: u32 = 0x804;
const F275928_WORD: u32 = 0x904;
const F275928_NWORD: u32 = 0x984;
const F275928_LENGTHS: u32 = 0xa24;
const F275928_KIND: u32 = 0xa64;
const F275928_NEXT_KIND: u32 = 0xa65;
const F275928_READ: u32 = 0xa66;
const F275928_NEXT_TOKEN: u32 = 0xa68;
const F275928_TOKEN_END: u32 = 0xa6a;
const F275928_MATCHED_END: u32 = 0xa6c;
const F275928_START: u32 = 0xa6e;
const F275928_COUNT: u32 = 0xa70;
const F275928_RESULTS: u32 = 0xa74;
const F275928_WRITTEN: u32 = 0xa78;
const F275928_PRIORITY: u32 = 0xa7a;
const F275928_FIRST: u32 = 0xa7c;
const EMPTY_S: u32 = 0x2702b8;
const STAR_S: u32 = 0x276778;

/// The `is_separator` test of `fn_275928`: space, tab, newline, CR, `_`.
fn f275928_sep(c: u8) -> bool {
    c == b' ' || c == b'\t' || c == b'\n' || c == b'\r' || c == b'_'
}

/// The `read_word` helper of `fn_275928`: the next blank-delimited word
/// of `*text` at `text_off + start` into `word`. Terminates the word
/// unless the 0x80-byte buffer fills first, and reports whether it did.
fn f275928_read_word(m: &mut Memory<'_>, text: u32, text_off: u16, start: u16, word: u32) -> bool {
    let src = match g_word(m, text) {
        Some(v) => v,
        None => return false,
    };
    let mut pos = text_off.wrapping_add(start);
    let mut i = 0u32;
    loop {
        let ch = g_byte(m, src.wrapping_add(u32::from(pos))).unwrap_or(0);
        if ch == b' ' || ch == b'\t' || ch == b'\n' || ch == b'\r' {
            return p_byte(m, word.wrapping_add(i), 0);
        }
        if ch == b'_' || ch == 0 {
            return p_byte(m, word.wrapping_add(i), 0);
        }
        if !p_byte(m, word.wrapping_add(i), ch) {
            return false;
        }
        i += 1;
        pos = pos.wrapping_add(1);
        if i == 0x80 {
            return false;
        }
    }
}

#[allow(clippy::too_many_arguments)]
pub fn fn_275928_m(
    m: &mut Memory<'_>,
    h: &mut impl Host,
    scratch: u32,
    sp: u32,
    fe: u32,
    text: u32,
    start_in: u16,
    end_in: u16,
    mode: u32,
    handle_obj: u32,
    handle_data: u32,
    tok: u32,
    markers: u32,
    mi: u32,
    n_markers: u32,
    base: u32,
    rich: u32,
) -> u32 {
    let words = scratch.wrapping_add(F275928_WORDS);
    let token_text = scratch.wrapping_add(F275928_TOKEN_TEXT);
    let ntoken = scratch.wrapping_add(F275928_NTOKEN);
    let nwords = scratch.wrapping_add(F275928_NWORDS);
    let word = scratch.wrapping_add(F275928_WORD);
    let nword = scratch.wrapping_add(F275928_NWORD);
    let lengths = scratch.wrapping_add(F275928_LENGTHS);
    let akind = scratch.wrapping_add(F275928_KIND);
    let anext_kind = scratch.wrapping_add(F275928_NEXT_KIND);
    let aread = scratch.wrapping_add(F275928_READ);
    let anext_token = scratch.wrapping_add(F275928_NEXT_TOKEN);
    let atoken_end = scratch.wrapping_add(F275928_TOKEN_END);
    let amatched_end = scratch.wrapping_add(F275928_MATCHED_END);
    let astart = scratch.wrapping_add(F275928_START);
    let acount = scratch.wrapping_add(F275928_COUNT);
    let aresults = scratch.wrapping_add(F275928_RESULTS);
    let awritten = scratch.wrapping_add(F275928_WRITTEN);
    let aprio = scratch.wrapping_add(F275928_PRIORITY);
    let afirst = scratch.wrapping_add(F275928_FIRST);
    let n26 = scratch.wrapping_add(FN_275928_N26);
    let nec = scratch.wrapping_add(FN_275928_NEC);
    // The nested G2P entry SP is this body SP: the `bl` doesn't move it.
    // Every G2P frame hangs off it at the disassembled offsets.
    let gsp = sp.wrapping_sub(F275928_FRAME);

    let token_api = g_word(m, fe.wrapping_add(0x0c)).unwrap_or(0);
    let dict_obj = g_word(m, fe.wrapping_add(0x10)).unwrap_or(0);
    let dict_data = g_word(m, fe.wrapping_add(0x14)).unwrap_or(0);
    let dict_api = g_word(m, fe.wrapping_add(0x18)).unwrap_or(0);
    // fe+0x50 is shared mutable state (2719ec writes it back per acronym
    // token): always re-read it like the C does (`fe->text_offset`), never
    // cache it in a local (the C's own `text_offset` local is unused).
    let nonctdps = g_byte(m, fe.wrapping_add(0x70)).unwrap_or(0);
    let dps = g_byte(m, fe.wrapping_add(0x71)).unwrap_or(0);
    let domain = fe.wrapping_add(0x72);
    let services = g_word(m, fe).unwrap_or(0);
    let logger = g_word(m, services.wrapping_add(0x10)).unwrap_or(0);

    let f_get_attr = g_word(m, token_api.wrapping_add(0x54)).unwrap_or(0);
    let f_next = g_word(m, token_api.wrapping_add(0x3c)).unwrap_or(0);
    let f_insert_child = g_word(m, token_api.wrapping_add(0x2c)).unwrap_or(0);
    let f_set_attr = g_word(m, token_api.wrapping_add(0x50)).unwrap_or(0);
    let f_word_lengths = g_word(m, dict_api.wrapping_add(0x28)).unwrap_or(0);
    let f_normalize_token = g_word(m, dict_api.wrapping_add(0x34)).unwrap_or(0);
    let f_normalize_word = g_word(m, dict_api.wrapping_add(0x38)).unwrap_or(0);

    // The six buffers start empty; the scalars start at their reset values.
    for b in [word, words, token_text, nword, nwords, ntoken] {
        h.call_m(0x195cd0, &[b, EMPTY_S], m);
    }
    if !p_half(m, anext_token, 0)
        || !p_half(m, aread, 0)
        || !p_byte(m, akind, 0)
        || !p_byte(m, anext_kind, 3)
        || !p_half(m, amatched_end, start_in.wrapping_sub(1))
        || !p_half(m, atoken_end, 0)
        || !p_half(m, astart, start_in)
    {
        return FAIL_MEM;
    }
    let mut fallback = 1u32;
    let mut previous_kind = 3u32;
    let mut segment_start = 0u16;
    let mut status = h.call_m(0x195d58, &[mode, domain], m);
    if status != 0 {
        status = h.call_m(0x275204, &[fe, mode], m);
        if status >= 0x80000000 {
            return status;
        }
        if !g_strcpy(m, domain, mode) {
            return FAIL_MEM;
        }
    }
    // The token kind of the head token.
    status = h.call_m(
        f_get_attr,
        &[handle_obj, handle_data, u32::from(g_half(m, tok).unwrap_or(0)), 0, 1, akind, aread],
        m,
    );
    if status >= 0x80000000 {
        return status;
    }
    let mut kind = g_byte(m, akind).unwrap_or(0);
    while kind == 5 || kind == 6 || kind == 7 {
        status = h.call_m(
            f_next,
            &[handle_obj, handle_data, u32::from(g_half(m, tok).unwrap_or(0)), tok],
            m,
        );
        if status >= 0x80000000 {
            return status;
        }
        if g_half(m, tok).unwrap_or(0) == 0 {
            return status;
        }
        status = h.call_m(
            f_get_attr,
            &[handle_obj, handle_data, u32::from(g_half(m, tok).unwrap_or(0)), 0, 1, akind, aread],
            m,
        );
        if status >= 0x80000000 {
            return status;
        }
        kind = g_byte(m, akind).unwrap_or(0);
    }
    let mut end = end_in;
    loop {
        if g_half(m, tok).unwrap_or(0) == 0 {
            return status;
        }
        if (end as i16) <= (g_half(m, atoken_end).unwrap_or(0) as i16) {
            return status;
        }
        status = h.call_m(
            f_next,
            &[handle_obj, handle_data, u32::from(g_half(m, tok).unwrap_or(0)), anext_token],
            m,
        );
        if status >= 0x80000000 {
            return status;
        }
        let mut next_token = g_half(m, anext_token).unwrap_or(0);
        let mut next_kind = 3u8;
        if next_token != 0 {
            status = h.call_m(
                f_get_attr,
                &[handle_obj, handle_data, u32::from(next_token), 0, 1, anext_kind, aread],
                m,
            );
            if status >= 0x80000000 {
                return status;
            }
            next_kind = g_byte(m, anext_kind).unwrap_or(0);
            while next_kind == 5 || next_kind == 6 || next_kind == 7 {
                status = h.call_m(
                    f_next,
                    &[handle_obj, handle_data, u32::from(next_token), anext_token],
                    m,
                );
                if status >= 0x80000000 {
                    return status;
                }
                next_token = g_half(m, anext_token).unwrap_or(0);
                if next_token == 0 {
                    break;
                }
                status = h.call_m(
                    f_get_attr,
                    &[handle_obj, handle_data, u32::from(next_token), 0, 1, anext_kind, aread],
                    m,
                );
                if status >= 0x80000000 {
                    return status;
                }
                next_kind = g_byte(m, anext_kind).unwrap_or(0);
            }
        }
        if next_token == 0 {
            next_kind = 3;
        }
        status = h.call_m(
            f_get_attr,
            &[handle_obj, handle_data, u32::from(g_half(m, tok).unwrap_or(0)), 1, 1, astart, aread],
            m,
        );
        if status >= 0x80000000 {
            return status;
        }
        let start = g_half(m, astart).unwrap_or(0);
        let matched_end = g_half(m, amatched_end).unwrap_or(0);
        if (matched_end as i16) <= (start as i16) {
            if kind != 3 && previous_kind == 3 {
                fallback = 0;
                segment_start = g_half(m, tok).unwrap_or(0);
                if f275928_read_word(m, text, g_half(m, fe.wrapping_add(0x50)).unwrap_or(0), start, word) {
                    if next_token != 0 && h.call_m(0x195d58, &[mode, 0x2760d8], m) != 0 {
                        if !g_strcpy(m, words, word) {
                            return FAIL_MEM;
                        }
                        if !p_half(m, acount, 0) {
                            return FAIL_MEM;
                        }
                        h.call_m(0x195bec, &[lengths, 0, 0x40], m);
                        status = h.call_m(
                            f_word_lengths,
                            &[dict_obj, dict_data, words, 0x2760e0, mode, aresults, acount],
                            m,
                        );
                        let mut length_count = 0u8;
                        if status < 0x80000000 {
                            let count = g_half(m, acount).unwrap_or(0);
                            let results = g_word(m, aresults).unwrap_or(0);
                            let mut j = 0u16;
                            while j < count {
                                let r =
                                    g_word(m, results.wrapping_add(u32::from(j) * 4)).unwrap_or(0);
                                let n = h.call_m(0x176cd8, &[r], m);
                                if n < 0x40 {
                                    if !p_byte(
                                        m,
                                        lengths.wrapping_add(u32::from(length_count)),
                                        n as u8,
                                    ) {
                                        return FAIL_MEM;
                                    }
                                    length_count = length_count.wrapping_add(1);
                                }
                                j = j.wrapping_add(1);
                            }
                            // The sort runs the b3 port directly; its
                            // 0x270b28 comparator runs through the host.
                            status = fn_2409d4_m(
                                m,
                                h,
                                lengths,
                                u32::from(length_count),
                                1,
                                0x270b28,
                            );
                        }
                        if status >= 0x80000000 {
                            return status;
                        }
                        let mut k = 0u8;
                        while k < length_count
                            && (g_half(m, amatched_end).unwrap_or(0) as i16) < (start as i16)
                        {
                            let wanted = g_byte(m, lengths.wrapping_add(u32::from(k))).unwrap_or(0);
                            let src = g_word(m, text).unwrap_or(0);
                            let mut pos = g_half(m, fe.wrapping_add(0x50)).unwrap_or(0).wrapping_add(start);
                            let mut i = 0u16;
                            let mut found = 0u8;
                            while found < wanted
                                && g_byte(m, src.wrapping_add(u32::from(pos))).unwrap_or(0) != 0
                            {
                                let ch = g_byte(m, src.wrapping_add(u32::from(pos))).unwrap_or(0);
                                if !p_byte(m, words.wrapping_add(u32::from(i)), ch) {
                                    return FAIL_MEM;
                                }
                                i = i.wrapping_add(1);
                                pos = pos.wrapping_add(1);
                                if i == 0x100 {
                                    break;
                                }
                                let cur = g_byte(m, src.wrapping_add(u32::from(pos))).unwrap_or(0);
                                let prev = g_byte(
                                    m,
                                    src.wrapping_add(u32::from(pos.wrapping_sub(1))),
                                )
                                .unwrap_or(0);
                                if f275928_sep(cur) && !f275928_sep(prev) {
                                    found = found.wrapping_add(1);
                                }
                            }
                            let mut do_lookup = i != 0x100;
                            if do_lookup {
                                if !p_byte(m, words.wrapping_add(u32::from(i)), 0) {
                                    return FAIL_MEM;
                                }
                                let at_end =
                                    g_byte(m, src.wrapping_add(u32::from(pos))).unwrap_or(0);
                                let prev = g_byte(
                                    m,
                                    src.wrapping_add(u32::from(pos.wrapping_sub(1))),
                                )
                                .unwrap_or(0);
                                if at_end == 0 && !f275928_sep(prev) {
                                    found = found.wrapping_add(1);
                                }
                                do_lookup = found == wanted;
                            }
                            if do_lookup {
                                if !g_strcpy(m, nwords, words) {
                                    return FAIL_MEM;
                                }
                                status = h.call_m(
                                    f_normalize_word,
                                    &[dict_obj, dict_data, mode, nwords, 0x100],
                                    m,
                                );
                                if status < 0x80000000 {
                                    status = fn_26ff18_m(
                                        m,
                                        h,
                                        n26,
                                        fe,
                                        text,
                                        nwords,
                                        words,
                                        0x2760e4,
                                        mode,
                                        start,
                                        g_half(m, tok).unwrap_or(0),
                                        handle_obj,
                                        handle_data,
                                        amatched_end,
                                        0,
                                        rich,
                                    );
                                    if status >= 0x80000000 {
                                        return status;
                                    }
                                    if (end as i16)
                                        < (g_half(m, amatched_end).unwrap_or(0) as i16)
                                    {
                                        end = g_half(m, amatched_end).unwrap_or(0);
                                    }
                                }
                            }
                            k = k.wrapping_add(1);
                        }
                    }
                    if (g_half(m, amatched_end).unwrap_or(0) as i16) <= (start as i16) {
                        status = fn_26ff18_m(
                            m,
                            h,
                            n26,
                            fe,
                            text,
                            word,
                            word,
                            0x2760e4,
                            mode,
                            start,
                            g_half(m, tok).unwrap_or(0),
                            handle_obj,
                            handle_data,
                            amatched_end,
                            0,
                            rich,
                        );
                        if status >= 0x80000000 {
                            return status;
                        }
                    }
                    if (g_half(m, amatched_end).unwrap_or(0) as i16) <= (start as i16) {
                        if !g_strcpy(m, nword, word) {
                            return FAIL_MEM;
                        }
                        status = h.call_m(
                            f_normalize_word,
                            &[dict_obj, dict_data, mode, nword, 0x80],
                            m,
                        );
                        if status < 0x80000000 && h.call_m(0x195d58, &[word, nword], m) != 0 {
                            status = fn_26ff18_m(
                                m,
                                h,
                                n26,
                                fe,
                                text,
                                nword,
                                word,
                                0x2760e4,
                                mode,
                                start,
                                g_half(m, tok).unwrap_or(0),
                                handle_obj,
                                handle_data,
                                amatched_end,
                                0,
                                rich,
                            );
                            if status >= 0x80000000 {
                                return status;
                            }
                        }
                    }
                }
                // mde: the nonctdps path runs whether or not the word read.
                if (g_half(m, amatched_end).unwrap_or(0) as i16) <= (start as i16)
                    && nonctdps != 0
                {
                    if !p_half(m, afirst, 0) {
                        return FAIL_MEM;
                    }
                    status = h.call_m(
                        0x2c9748,
                        &[
                            fe,
                            u32::from(start),
                            word,
                            mode,
                            u32::from(g_half(m, tok).unwrap_or(0)),
                            afirst,
                            handle_obj,
                            handle_data,
                        ],
                        m,
                    );
                    if status >= 0x80000000 {
                        return status;
                    }
                    if g_half(m, afirst).unwrap_or(0) != 0 {
                        next_kind = 0;
                        next_token = g_half(m, afirst).unwrap_or(0);
                    }
                }
            }
            if (g_half(m, amatched_end).unwrap_or(0) as i16) <= (start as i16)
                && (previous_kind != 3 || next_kind != 3)
            {
                // The ARM reaches its 0x2766c4 fallback store only by
                // falling through the kind==0/1 dispatch (0x2761f4 jumps
                // straight to the 0x2766e4 call gate for any other kind)
                // with a short token (0x27626c jumps over the store when
                // length>=0x80).
                let mut reach_store = false;
                if kind == 0 || kind == 1 {
                    status = h.call_m(
                        f_get_attr,
                        &[
                            handle_obj,
                            handle_data,
                            u32::from(g_half(m, tok).unwrap_or(0)),
                            2,
                            1,
                            atoken_end,
                            aread,
                        ],
                        m,
                    );
                    if status >= 0x80000000 {
                        return status;
                    }
                    let token_end = g_half(m, atoken_end).unwrap_or(0);
                    // 1:1 with the C (`fe->text_offset` is re-read, never cached).
                    let text_off_now = g_half(m, fe.wrapping_add(0x50)).unwrap_or(0);
                    let begin = text_off_now.wrapping_add(start);
                    let length = u32::from(token_end.wrapping_add(text_off_now))
                        .wrapping_sub(u32::from(begin));
                    if length < 0x80 {
                        reach_store = true;
                        let t = g_word(m, text).unwrap_or(0);
                        // Round 4 (seg1): the bounded copy runs
                        // in-component (0x195e38 has no live Host arm).
                        if !f195e38_copy(m, token_text, t.wrapping_add(u32::from(begin)), length) {
                            return FAIL_MEM;
                        }
                        status = 0;
                        if !p_byte(m, token_text.wrapping_add(length), 0) {
                            return FAIL_MEM;
                        }
                        if !g_strcpy(m, ntoken, token_text) {
                            return FAIL_MEM;
                        }
                        // Round 4 (seg1): the lex canonical-token
                        // entry (0x1ed3b0) runs in-component; the
                        // vtable read guards the direct binding.
                        status = if f_normalize_token == 0x1ed3b0 {
                            fn_1ed3b0_m(m, h, dict_obj, dict_data, mode, ntoken, 0x80)
                        } else {
                            h.call_m(
                                f_normalize_token,
                                &[dict_obj, dict_data, mode, ntoken, 0x80],
                                m,
                            )
                        };
                        if status < 0x80000000 {
                            if h.call_m(0x195d58, &[nword, ntoken], m) != 0 {
                                status = fn_26ff18_m(
                                    m,
                                    h,
                                    n26,
                                    fe,
                                    text,
                                    ntoken,
                                    token_text,
                                    0x2760e4,
                                    mode,
                                    start,
                                    g_half(m, tok).unwrap_or(0),
                                    handle_obj,
                                    handle_data,
                                    amatched_end,
                                    1,
                                    rich,
                                );
                                if status >= 0x80000000 {
                                    return status;
                                }
                            }
                        } else if !g_strcpy(m, ntoken, token_text) {
                            return FAIL_MEM;
                        }
                        if (g_half(m, amatched_end).unwrap_or(0) as i16) <= (start as i16) {
                            if kind == 0 {
                                if dps != 0 {
                                    // c3 r18/r19 (shared): the G2P lookup runs
                                    // in-component over the SP-relative
                                    // 275928 frame (c3 r18/r19).
                                    status = crate::component_g2p::fn_2c9ba0_m(
                                        m,
                                        h,
                                        gsp,
                                        fe,
                                        ntoken,
                                        token_text,
                                        0x2760e4,
                                        mode,
                                        start,
                                        g_half(m, tok).unwrap_or(0),
                                        handle_obj,
                                        handle_data,
                                        amatched_end,
                                    );
                                    if status >= 0x80000000 {
                                        return status;
                                    }
                                }
                                if (g_half(m, amatched_end).unwrap_or(0) as i16) <= (start as i16)
                                {
                                    // c3 r18/r19 (shared): the MDE config lookup runs
                                    // in-component over the SP-relative 275928 frame.
                                    status = crate::component_g2p::fn_2c93f0_m(
                                        m,
                                        h,
                                        gsp,
                                        fe,
                                        ntoken,
                                        token_text,
                                        mode,
                                        start,
                                        g_half(m, tok).unwrap_or(0),
                                        handle_obj,
                                        handle_data,
                                        amatched_end,
                                    );
                                    if status >= 0x80000000 {
                                        return status;
                                    }
                                }
                            } else {
                                status = f275928_emit_token(
                                    m,
                                    h,
                                    f_next,
                                    f_insert_child,
                                    f_set_attr,
                                    logger,
                                    0x27675c,
                                    token_text,
                                    kind,
                                    handle_obj,
                                    handle_data,
                                    tok,
                                    astart,
                                    amatched_end,
                                    awritten,
                                    aprio,
                                    akind,
                                    true,
                                );
                                if status >= 0x80000000 {
                                    return status;
                                }
                                kind = 4;
                            }
                        }
                    }
                }
                if reach_store
                    && (g_half(m, amatched_end).unwrap_or(0) as i16) <= (start as i16)
                {
                    fallback = 1;
                }
                if kind != 3 && next_kind == 3 {
                    // Round 4 (seg1): the acronym path runs in-component
                    // at the ARM's own frame, with the rule expansion and
                    // the broker open intercepted (see XHost); the write-up
                    // for wire-a's engine_live::Host arm is in the round-4
                    // progress file.
                    let mut xh = XHost {
                        inner: &mut *h,
                        vm: crate::component_live_c::VmWorkspace::default(),
                    };
                    status = fn_2719ec_m(
                        m,
                        &mut xh,
                        FN_2719EC_FRAME,
                        fe,
                        text,
                        mode,
                        handle_obj,
                        handle_data,
                        u32::from(segment_start),
                        u32::from(g_half(m, tok).unwrap_or(0)),
                        markers,
                        mi,
                        n_markers,
                        base,
                        fallback,
                    );
                    if status >= 0x80000000 {
                        return status;
                    }
                }
                // next:
                status = h.call_m(
                    f_get_attr,
                    &[
                        handle_obj,
                        handle_data,
                        u32::from(g_half(m, tok).unwrap_or(0)),
                        2,
                        1,
                        atoken_end,
                        aread,
                    ],
                    m,
                );
                if status >= 0x80000000 {
                    return status;
                }
                if !p_half(m, tok, next_token) {
                    return FAIL_MEM;
                }
                previous_kind = u32::from(kind);
                kind = next_kind;
                continue;
            }
            if (g_half(m, amatched_end).unwrap_or(0) as i16) <= (start as i16)
                && previous_kind == 3
                && next_kind == 3
            {
                if kind == 0 {
                    status = h.call_m(
                        f_get_attr,
                        &[
                            handle_obj,
                            handle_data,
                            u32::from(g_half(m, tok).unwrap_or(0)),
                            2,
                            1,
                            atoken_end,
                            aread,
                        ],
                        m,
                    );
                    if status >= 0x80000000 {
                        return status;
                    }
                    let token_end = g_half(m, atoken_end).unwrap_or(0);
                    // 1:1 with the C (`fe->text_offset` is re-read, never cached).
                    let text_off_now = g_half(m, fe.wrapping_add(0x50)).unwrap_or(0);
                    let begin = text_off_now.wrapping_add(start);
                    let length = u32::from(token_end.wrapping_add(text_off_now))
                        .wrapping_sub(u32::from(begin));
                    if length < 0x80 {
                        let t = g_word(m, text).unwrap_or(0);
                        // Round 4 (seg1): the bounded copy runs
                        // in-component (0x195e38 has no live Host arm).
                        if !f195e38_copy(m, token_text, t.wrapping_add(u32::from(begin)), length) {
                            return FAIL_MEM;
                        }
                        status = 0;
                        if !p_byte(m, token_text.wrapping_add(length), 0) {
                            return FAIL_MEM;
                        }
                        if !g_strcpy(m, ntoken, token_text) {
                            return FAIL_MEM;
                        }
                        // Round 4 (seg1): the lex canonical-token
                        // entry (0x1ed3b0) runs in-component; the
                        // vtable read guards the direct binding.
                        status = if f_normalize_token == 0x1ed3b0 {
                            fn_1ed3b0_m(m, h, dict_obj, dict_data, mode, ntoken, 0x80)
                        } else {
                            h.call_m(
                                f_normalize_token,
                                &[dict_obj, dict_data, mode, ntoken, 0x80],
                                m,
                            )
                        };
                        if status < 0x80000000 {
                            if dps != 0 {
                                // c3 r18/r19 (shared): see the first 0x2c9ba0 site.
                                status = crate::component_g2p::fn_2c9ba0_m(
                                    m,
                                    h,
                                    gsp,
                                    fe,
                                    ntoken,
                                    token_text,
                                    0x2760e4,
                                    mode,
                                    start,
                                    g_half(m, tok).unwrap_or(0),
                                    handle_obj,
                                    handle_data,
                                    amatched_end,
                                );
                                if status >= 0x80000000 {
                                    return status;
                                }
                            }
                            if (g_half(m, amatched_end).unwrap_or(0) as i16) <= (start as i16)
                            {
                                // c3 r18/r19 (shared): see the first 0x2c93f0 site above.
                                status = crate::component_g2p::fn_2c93f0_m(
                                    m,
                                    h,
                                    gsp,
                                    fe,
                                    ntoken,
                                    token_text,
                                    mode,
                                    start,
                                    g_half(m, tok).unwrap_or(0),
                                    handle_obj,
                                    handle_data,
                                    amatched_end,
                                );
                                if status >= 0x80000000 {
                                    return status;
                                }
                            }
                        }
                        if (g_half(m, amatched_end).unwrap_or(0) as i16) <= (start as i16) {
                            fallback = 1;
                        }
                    }
                    // Round 4 (seg1): see the first 0x2719ec site above.
                    let mut xh = XHost {
                        inner: &mut *h,
                        vm: crate::component_live_c::VmWorkspace::default(),
                    };
                    status = fn_2719ec_m(
                        m,
                        &mut xh,
                        FN_2719EC_FRAME,
                        fe,
                        text,
                        mode,
                        handle_obj,
                        handle_data,
                        u32::from(segment_start),
                        u32::from(g_half(m, tok).unwrap_or(0)),
                        markers,
                        mi,
                        n_markers,
                        base,
                        fallback,
                    );
                    if status >= 0x80000000 {
                        return status;
                    }
                } else if kind == 1 || kind == 2 {
                    status = f275928_emit_token(
                        m,
                        h,
                        f_next,
                        f_insert_child,
                        f_set_attr,
                        logger,
                        0x276c48,
                        word,
                        kind,
                        handle_obj,
                        handle_data,
                        tok,
                        astart,
                        amatched_end,
                        awritten,
                        aprio,
                        akind,
                        false,
                    );
                    if status >= 0x80000000 {
                        return status;
                    }
                    kind = 4;
                }
            }
        }
        // next:
        status = h.call_m(
            f_get_attr,
            &[
                handle_obj,
                handle_data,
                u32::from(g_half(m, tok).unwrap_or(0)),
                2,
                1,
                atoken_end,
                aread,
            ],
            m,
        );
        if status >= 0x80000000 {
            return status;
        }
        if !p_half(m, tok, g_half(m, anext_token).unwrap_or(0)) {
            return FAIL_MEM;
        }
        previous_kind = u32::from(kind);
        kind = next_kind;
    }
}
/// Guest scratch window offsets for `fn_274564_m` (caller-provided
/// guest window, disjoint from the caller's frame; 64 bytes used).
const F274564_AUX: u32 = 0x00;
const F274564_AUX2: u32 = 0x08;
const F274564_VALS: u32 = 0x0c;
const F274564_FOUND: u32 = 0x10;
const F274564_LEN: u32 = 0x14;
const F274564_LANG: u32 = 0x18;
const F274564_LNGKEY: u32 = 0x20;
const F274564_ONE: u32 = 0x30;
const F274564_BOUND: u32 = 0x32;
const F274564_OUT: u32 = 0x34;
const F274564_WRITTEN: u32 = 0x38;

/// `fn_274564` (dict_phon): the phonetic-input path. A word whose mode
/// string is `phon` is re-tokenized from its phonetic spelling: markers
/// at or past `end` shift by the G2P time shift, the sentence nodes up to
/// `end` are dropped, then one word record per blank-delimited phonetic
/// token is inserted under `sentNode` with dictionary transcription,
/// pause-boundary flags and the alternative-transcription link.
/// 1:1 with src/fn_26ff18.c (NONMATCHING there, behaviorally exact).
/// `scratch` is a caller-owned guest window (at least 64 bytes).
#[allow(clippy::too_many_arguments)]
pub fn fn_274564_m(
    m: &mut Memory<'_>,
    h: &mut impl Host,
    scratch: u32,
    p: u32,
    text: u32,
    start_in: u16,
    end_in: u32,
    a: u32,
    c: u32,
    tok_out: u32,
    markers: u32,
    mi_out: u32,
    nm_out: u32,
    base: i32,
) -> u32 {
    let _ = base;
    let boundary: u16 = 200;
    let aux = scratch.wrapping_add(F274564_AUX);
    let aux2 = scratch.wrapping_add(F274564_AUX2);
    let vals_at = scratch.wrapping_add(F274564_VALS);
    let found_at = scratch.wrapping_add(F274564_FOUND);
    let len_at = scratch.wrapping_add(F274564_LEN);
    let lang = scratch.wrapping_add(F274564_LANG);
    let lngkey = scratch.wrapping_add(F274564_LNGKEY);
    let one_at = scratch.wrapping_add(F274564_ONE);
    let bound_at = scratch.wrapping_add(F274564_BOUND);
    let out_at = scratch.wrapping_add(F274564_OUT);
    let written_at = scratch.wrapping_add(F274564_WRITTEN);
    // G2P context off the fe object (offsets verified against the live
    // image: lat vtable serves the LINGDB entries below).
    let env = g_word(m, p).unwrap_or(0);
    let heap = g_word(m, env.wrapping_add(4)).unwrap_or(0);
    let log = g_word(m, env.wrapping_add(16)).unwrap_or(0);
    let lat = g_word(m, p.wrapping_add(0x0c)).unwrap_or(0);
    let cfgh0 = g_word(m, p.wrapping_add(0x10)).unwrap_or(0);
    let cfgh1 = g_word(m, p.wrapping_add(0x14)).unwrap_or(0);
    let cfg = g_word(m, p.wrapping_add(0x18)).unwrap_or(0);
    let lexh0 = g_word(m, p.wrapping_add(0x34)).unwrap_or(0);
    let lexh1 = g_word(m, p.wrapping_add(0x38)).unwrap_or(0);
    let lex = g_word(m, p.wrapping_add(0x3c)).unwrap_or(0);
    let time_shift = g_half(m, p.wrapping_add(0x50)).unwrap_or(0) as i16;
    let use_lex = g_byte(m, p.wrapping_add(0x179)).unwrap_or(0);
    let f_next = g_word(m, lat.wrapping_add(0x3c)).unwrap_or(0);
    let f_up = g_word(m, lat.wrapping_add(0x44)).unwrap_or(0);
    let f_set_attr = g_word(m, lat.wrapping_add(0x50)).unwrap_or(0);
    let f_get_attr = g_word(m, lat.wrapping_add(0x54)).unwrap_or(0);
    let f_get_ptr = g_word(m, lat.wrapping_add(0x58)).unwrap_or(0);
    let f_has_attr = g_word(m, lat.wrapping_add(0x5c)).unwrap_or(0);
    let f_remove = g_word(m, lat.wrapping_add(0x60)).unwrap_or(0);
    let f_insert_child = g_word(m, lat.wrapping_add(0x2c)).unwrap_or(0);
    let f_insert_after = g_word(m, lat.wrapping_add(0x28)).unwrap_or(0);
    let f_cfg_get = g_word(m, cfg.wrapping_add(0x1c)).unwrap_or(0);
    let f_lex_lookup = g_word(m, lex.wrapping_add(0x24)).unwrap_or(0);
    if !p_half(m, one_at, 1) || !p_half(m, bound_at, boundary) {
        return FAIL_MEM;
    }
    // Marker shift: markers at or past `end` shift by timeShift.
    let nm = g_half(m, nm_out).unwrap_or(0);
    let mut mark_idx = g_half(m, mi_out).unwrap_or(0);
    while mark_idx < nm {
        let t = g_word(
            m,
            markers
                .wrapping_add(u32::from(mark_idx).wrapping_mul(MARKER_LEN))
                .wrapping_add(MARKER_POS),
        )
        .unwrap_or(0);
        if t >= end_in {
            break;
        }
        if !p_word(
            m,
            markers
                .wrapping_add(u32::from(mark_idx).wrapping_mul(MARKER_LEN))
                .wrapping_add(MARKER_POS),
            t.wrapping_add(time_shift as u32),
        ) {
            return FAIL_MEM;
        }
        mark_idx = mark_idx.wrapping_add(1);
    }
    if !p_half(m, mi_out, mark_idx) {
        return FAIL_MEM;
    }
    // Drop sentence nodes up to `end`, tracking the type-5 node.
    // NOTE: `ftype` is uninitialized in the C until the first getAttr;
    // zero matches the observed ARM behavior (verified on the case-39
    // capture: the `#` climb below runs the same).
    let mut node = g_half(m, tok_out).unwrap_or(0);
    let mut tok: u16 = 0;
    let mut wpos: u16 = 0;
    let mut prev: u32 = 0;
    let mut sent_node: u16 = 0;
    let mut ftype: u8 = 0;
    while node != 0 && (sent_node == 0 || (wpos as i16) < (end_in as u16 as i16)) {
        let mut status = h.call_m(f_get_attr, &[a, c, u32::from(node), 0, 1, out_at, aux], m);
        if status >= 0x80000000 {
            return status;
        }
        ftype = g_byte(m, out_at).unwrap_or(0);
        if ftype <= 3 {
            status = h.call_m(f_get_attr, &[a, c, u32::from(node), 2, 1, out_at, aux], m);
            if status >= 0x80000000 {
                return status;
            }
            wpos = g_half(m, out_at).unwrap_or(0);
            prev = u32::from(node);
        } else if ftype == 5 {
            sent_node = node;
        }
        status = h.call_m(f_next, &[a, c, u32::from(node), out_at], m);
        if status >= 0x80000000 {
            return status;
        }
        node = g_half(m, out_at).unwrap_or(0);
        if prev != 0 {
            status = h.call_m(f_remove, &[a, c, prev], m);
            if status >= 0x80000000 {
                return status;
            }
            let mut cur = g_half(m, p.wrapping_add(0x68)).unwrap_or(0);
            if cur == prev as u16 {
                cur = node;
                if !p_half(m, p.wrapping_add(0x68), cur) {
                    return FAIL_MEM;
                }
            }
        }
        prev = 0;
    }
    // Phonetic spelling of the sentence node (field 3).
    let mut status = h.call_m(f_get_ptr, &[a, c, u32::from(sent_node), 3, out_at, aux], m);
    if status >= 0x80000000 {
        return status;
    }
    let mut str_addr = g_word(m, out_at).unwrap_or(0);
    let mut has: i16 = 0;
    status = h.call_m(f_has_attr, &[a, c, u32::from(sent_node), 9, out_at], m);
    if status < 0x80000000 && g_half(m, out_at).unwrap_or(0) == 1 {
        has = 1;
        status = h.call_m(f_get_ptr, &[a, c, u32::from(sent_node), 9, out_at, aux], m);
        if status >= 0x80000000 {
            return status;
        }
    }
    if status >= 0x80000000 {
        return status;
    }
    let mut attr9 = 0u32;
    if has == 1 {
        attr9 = g_word(m, out_at).unwrap_or(0);
    }
    while g_byte(m, str_addr).unwrap_or(0) == b' ' {
        str_addr = str_addr.wrapping_add(1);
    }
    let mut extra = h.call_m(0x195d30, &[str_addr, 0x12], m);
    if extra != 0 {
        if !p_byte(m, extra, 0) {
            return FAIL_MEM;
        }
        extra = extra.wrapping_add(1);
    }
    let mut str_len = h.call_m(0x195d90, &[str_addr], m);
    let is_bs = str_len > 4 && g_byte(m, str_addr).unwrap_or(0) == b'\\';
    if is_bs && g_byte(m, str_addr.wrapping_add(1)).unwrap_or(0) == b'T' {
        // `\T?` language override: lngphon_e + str[2].
        let key = *b"lngphon_e\0\0\0\0\0\0\0";
        let mut kb = key;
        kb[8] = g_byte(m, str_addr.wrapping_add(2)).unwrap_or(0);
        for (i, &ch) in kb.iter().enumerate() {
            if !p_byte(m, lngkey.wrapping_add(i as u32), ch) {
                return FAIL_MEM;
            }
        }
        if !p_word(m, vals_at, 0) || !p_half(m, found_at, 0) || !p_byte(m, len_at, 0) {
            return FAIL_MEM;
        }
        status = h.call_m(
            f_cfg_get,
            &[cfgh0, cfgh1, S_FECFG, lngkey, vals_at, found_at, len_at],
            m,
        );
        if status >= 0x80000000 {
            return status;
        }
        if g_half(m, found_at).unwrap_or(0) as i16 != 0 {
            let vals = g_word(m, vals_at).unwrap_or(0);
            let llen = g_byte(m, len_at).unwrap_or(0);
            let q = h.call_m(0x195d30, &[vals, u32::from(llen)], m);
            if q != 0 && !p_byte(m, q, 0) {
                return FAIL_MEM;
            }
            if !f195e38_copy(m, lang, vals, 3) {
                return FAIL_MEM;
            }
        }
        str_addr = str_addr.wrapping_add(4);
    }
    // `#` adjustment: climb to the word level, flag the boundary.
    // (`ntype` repeats the last getAttr type; `ftype` is 0 if the drop
    // loop never ran, matching the C local's observed value.)
    if g_byte(m, str_addr).unwrap_or(0) == b'#' {
        let mut n = sent_node;
        let mut ntype = ftype;
        while n != 0 && ntype != 4 && ntype != 9 && ntype != 10 {
            status = h.call_m(f_up, &[a, c, u32::from(n), out_at], m);
            if status >= 0x80000000 {
                return status;
            }
            n = g_half(m, out_at).unwrap_or(0);
            if n == 0 {
                break;
            }
            status = h.call_m(f_get_attr, &[a, c, u32::from(n), 0, 1, out_at, aux], m);
            if status >= 0x80000000 {
                return status;
            }
            ntype = g_byte(m, out_at).unwrap_or(0);
        }
        if n != 0 {
            status = h.call_m(f_set_attr, &[a, c, u32::from(n), 8, 1, bound_at, aux2], m);
            if status >= 0x80000000 {
                return status;
            }
        }
    }
    while g_byte(m, str_addr).unwrap_or(0) == b'#' {
        str_addr = str_addr.wrapping_add(1);
    }
    // One word record per blank-delimited phonetic token.
    let mut wp = str_addr;
    let mut pos = start_in;
    loop {
        let ch = g_byte(m, text.wrapping_add(u32::from(pos))).unwrap_or(0);
        if ch == b' ' || ch == b'\t' || ch == b'\n' || ch == b'\r' || ch == b'_' {
            pos = pos.wrapping_add(1);
        } else {
            break;
        }
    }
    let mut first: u16 = 1;
    let mut n_words: u16 = 0;
    let mut last_word: u16 = 0;
    let mut buf: u32 = 0;
    while (pos as u32) < end_in {
        let mut wend = pos;
        loop {
            if u32::from(wend) >= end_in {
                break;
            }
            let ch = g_byte(m, text.wrapping_add(u32::from(wend))).unwrap_or(0);
            if ch == b' ' || ch == b'\t' || ch == b'\n' || ch == b'\r' || ch == b'_' {
                break;
            }
            wend = wend.wrapping_add(1);
        }
        if first != 0 {
            status = h.call_m(f_insert_child, &[a, c, u32::from(sent_node), out_at], m);
        } else {
            status = h.call_m(f_insert_after, &[a, c, u32::from(tok), out_at], m);
        }
        if status >= 0x80000000 {
            return status;
        }
        tok = g_half(m, out_at).unwrap_or(0);
        if !p_half(m, out_at, 10) {
            return FAIL_MEM;
        }
        status = h.call_m(f_set_attr, &[a, c, u32::from(tok), 0, 1, out_at, aux2], m);
        if status >= 0x80000000 {
            return status;
        }
        if !p_half(m, out_at, pos) {
            return FAIL_MEM;
        }
        status = h.call_m(f_set_attr, &[a, c, u32::from(tok), 1, 1, out_at, aux2], m);
        if status >= 0x80000000 {
            return status;
        }
        if !p_half(m, out_at, wend) {
            return FAIL_MEM;
        }
        status = h.call_m(f_set_attr, &[a, c, u32::from(tok), 2, 1, out_at, aux2], m);
        if status >= 0x80000000 {
            return status;
        }
        if attr9 != 0 {
            let alen = h.call_m(0x195d90, &[attr9], m);
            status = h.call_m(
                f_set_attr,
                &[a, c, u32::from(tok), 9, alen.wrapping_add(1), attr9, aux2],
                m,
            );
            if status >= 0x80000000 {
                return status;
            }
        }
        n_words = n_words.wrapping_add(1);
        // Isolate one blank-delimited word out of `str`.
        while g_byte(m, wp).unwrap_or(0) != 0
            && g_byte(m, wp).unwrap_or(0) != b'_'
            && g_byte(m, wp).unwrap_or(0) != b' '
            && g_byte(m, wp).unwrap_or(0) != b'#'
        {
            wp = wp.wrapping_add(1);
        }
        let cw = g_byte(m, wp).unwrap_or(0);
        if !p_byte(m, wp, 0) {
            return FAIL_MEM;
        }
        str_len = h.call_m(0x195d90, &[str_addr], m);
        if str_len != 0 {
            let need = str_len.wrapping_mul(2).wrapping_add(1);
            let nb = h.call_m(0x1963e0, &[heap, buf, need], m);
            if nb == 0 {
                let s = h.call_m(0x173dc0, &[log, 0x8f9532, 0x84d0, 0], m);
                let _ = s;
                return 0x89b0200a;
            }
            buf = nb;
            if use_lex != 0 {
                if h.call_m(0x195d58, &[lang, EMPTY_S], m) != 0 {
                    status = h.call_m(f_lex_lookup, &[lexh0, lexh1, 1, str_addr, need, buf, lang], m);
                } else {
                    status = h.call_m(f_lex_lookup, &[lexh0, lexh1, 1, str_addr, need, buf, 0], m);
                }
            } else {
                status = h.call_m(0x195cd0, &[buf, str_addr], m);
            }
            if status >= 0x80000000 {
                return status;
            }
            // Marker-word check: buf in {"*,","*;","*:","`","'","\""}.
            let b0 = g_byte(m, buf).unwrap_or(0);
            let b1 = g_byte(m, buf.wrapping_add(1)).unwrap_or(0);
            let b2 = g_byte(m, buf.wrapping_add(2)).unwrap_or(0);
            let is_marker = (b0 == b'*' && (b1 == b',' || b1 == b';' || b1 == b':') && b2 == 0)
                || ((b0 == b'`' || b0 == b'\'' || b0 == b'"') && b1 == 0);
            if is_marker {
                if !p_byte(m, out_at, 0) {
                    return FAIL_MEM;
                }
                status = h.call_m(f_set_attr, &[a, c, u32::from(tok), 3, 1, out_at, aux2], m);
            } else {
                str_len = h.call_m(0x195d90, &[buf], m);
                if !p_half(m, out_at, 0) {
                    return FAIL_MEM;
                }
                status = h.call_m(
                    f_set_attr,
                    &[a, c, u32::from(tok), 3, str_len.wrapping_add(1), buf, aux2],
                    m,
                );
            }
            if status >= 0x80000000 {
                return status;
            }
        } else {
            if !p_byte(m, out_at, 0) {
                return FAIL_MEM;
            }
            status = h.call_m(f_set_attr, &[a, c, u32::from(tok), 3, 1, out_at, aux2], m);
            if status >= 0x80000000 {
                return status;
            }
        }
        if extra != 0 {
            str_len = h.call_m(0x195d90, &[extra], m);
            if !p_half(m, out_at, 0) {
                return FAIL_MEM;
            }
            status = h.call_m(
                f_set_attr,
                &[a, c, u32::from(tok), 11, str_len.wrapping_add(1), extra, aux2],
                m,
            );
            if status >= 0x80000000 {
                return status;
            }
            extra = 0;
            last_word = tok;
        }
        // Phonetic ("*") + orthographic ("*") placeholders and the used-flag.
        status = h.call_m(f_set_attr, &[a, c, u32::from(tok), 5, 2, STAR_S, aux2], m);
        if status >= 0x80000000 {
            return status;
        }
        status = h.call_m(f_set_attr, &[a, c, u32::from(tok), 6, 2, STAR_S, aux2], m);
        if status >= 0x80000000 {
            return status;
        }
        if !p_half(m, out_at, 1) {
            return FAIL_MEM;
        }
        status = h.call_m(f_set_attr, &[a, c, u32::from(tok), 7, 1, out_at, aux2], m);
        if status >= 0x80000000 {
            return status;
        }
        if attr9 != 0 {
            str_len = h.call_m(0x195d90, &[attr9], m);
            if !p_half(m, out_at, 0) {
                return FAIL_MEM;
            }
            status = h.call_m(
                f_set_attr,
                &[a, c, u32::from(tok), 9, str_len.wrapping_add(1), attr9, aux2],
                m,
            );
        } else {
            if !p_byte(m, out_at, 0) {
                return FAIL_MEM;
            }
            status = h.call_m(f_set_attr, &[a, c, u32::from(tok), 9, 1, out_at, aux2], m);
        }
        if status >= 0x80000000 {
            return status;
        }
        let is_hash_or_star = if cw == b'#' {
            true
        } else if buf != 0 {
            let blen = h.call_m(0x195d90, &[buf], m);
            blen >= 2
                && g_byte(m, buf.wrapping_add(blen.wrapping_sub(2))).unwrap_or(0) == b'*'
                && matches!(
                    g_byte(m, buf.wrapping_add(blen.wrapping_sub(1))).unwrap_or(0),
                    b',' | b';' | b':'
                )
        } else {
            false
        };
        if is_hash_or_star {
            if !p_half(m, out_at, boundary) {
                return FAIL_MEM;
            }
            status = h.call_m(f_set_attr, &[a, c, u32::from(tok), 8, 1, out_at, aux2], m);
            if status >= 0x80000000 {
                return status;
            }
        }
        if cw != 0 {
            if !p_byte(m, wp, cw as u8) {
                return FAIL_MEM;
            }
            wp = wp.wrapping_add(1);
            while g_byte(m, wp).unwrap_or(0) == b'_' || g_byte(m, wp).unwrap_or(0) == b'#' {
                wp = wp.wrapping_add(1);
            }
            str_addr = wp;
        }
        pos = wpos;
        loop {
            if u32::from(pos) >= end_in
                || !matches!(
                    g_byte(m, text.wrapping_add(u32::from(pos))).unwrap_or(0),
                    b' ' | b'\t' | b'\n' | b'\r' | b'_'
                )
            {
                break;
            }
            pos = pos.wrapping_add(1);
        }
        if first != 0 {
            first = 0;
            tok = sent_node;
        }
    }
    status = h.call_m(f_next, &[a, c, u32::from(tok), tok_out], m);
    if status >= 0x80000000 {
        return status;
    }
    if last_word != 0 && n_words > 1 {
        if !p_half(m, out_at, n_words) {
            return FAIL_MEM;
        }
        status = h.call_m(f_set_attr, &[a, c, u32::from(last_word), 7, 1, out_at, aux2], m);
        if status >= 0x80000000 {
            return status;
        }
    }
    if buf != 0 {
        status = h.call_m(0x196428, &[heap, buf], m);
        let _ = status;
    }
    status
}


/// The shared kind==1/2 token-emit block of `fn_275928`: log the token,
/// extend the match, insert the child and set fields 0, 1, 2, 3, 5, 6, 7
/// and 9. `advance` selects the trailing `next` of the `previous/next`
/// path. Returns the last callee status.
#[allow(clippy::too_many_arguments)]
fn f275928_emit_token(
    m: &mut Memory<'_>,
    h: &mut impl Host,
    f_next: u32,
    f_insert_child: u32,
    f_set_attr: u32,
    logger: u32,
    name: u32,
    buf: u32,
    kind: u8,
    handle_obj: u32,
    handle_data: u32,
    tok: u32,
    astart: u32,
    amatched_end: u32,
    awritten: u32,
    aprio: u32,
    akind: u32,
    advance: bool,
) -> u32 {
    h.call_m(
        0x173dc0,
        &[logger, 0x8f9532, 0x84d3, 0x276764, name, buf, 0x276750, u32::from(kind)],
        m,
    );
    let wlen = h.call_m(0x195d90, &[buf], m);
    if !p_byte(m, akind, 4) {
        return FAIL_MEM;
    }
    // The ROM reads `start` back out of its incoming stack slot; the port
    // keeps it in `astart` since the loop start.
    let start = g_half(m, astart).unwrap_or(0);
    if !p_half(m, amatched_end, (wlen as u16).wrapping_add(start)) {
        return FAIL_MEM;
    }
    let mut status = h.call_m(
        f_insert_child,
        &[handle_obj, handle_data, u32::from(g_half(m, tok).unwrap_or(0)), tok],
        m,
    );
    if status >= 0x80000000 {
        return status;
    }
    if !p_half(m, awritten, 0) {
        return FAIL_MEM;
    }
    let fields: [(u32, u32, u32); 8] = [
        (0, 1, akind),
        (1, 1, astart),
        (2, 1, amatched_end),
        (3, 1, EMPTY_S),
        (5, 2, STAR_S),
        (6, 2, STAR_S),
        (7, 1, aprio),
        (9, 1, EMPTY_S),
    ];
    if !p_half(m, aprio, 1) {
        return FAIL_MEM;
    }
    for (id, n, val) in fields {
        status = h.call_m(
            f_set_attr,
            &[handle_obj, handle_data, u32::from(g_half(m, tok).unwrap_or(0)), id, n, val, awritten],
            m,
        );
        if status >= 0x80000000 {
            return status;
        }
    }
    if advance {
        status = h.call_m(
            f_next,
            &[handle_obj, handle_data, u32::from(g_half(m, tok).unwrap_or(0)), tok],
            m,
        );
    }
    status
}
/// `fn_1f98c8` literal addresses: the function's own pool in the ROM
/// image (verified against the wire capture of the 1f98c8 run).
const S_CLMVARIANTS: u32 = 0x1f9cf8;
const S_NO: u32 = 0x1f9d04;
const S_CLCMLEXTRA: u32 = 0x1f9d08;
const S_NUMOUTTRANS: u32 = 0x1f9d1c;
const S_NORICHTRANS: u32 = 0x1f9d38;
const S_FALSE: u32 = 0x1f9d4c;
const S_FECFG: u32 = 0x1f9d54;
const S_MPTHREE: u32 = 0x1f9d5c;
const S_MPTHREEML: u32 = 0x1f9d64;
const S_VADVDE: u32 = 0x1f9d70;
const S_DTSCORE: u32 = 0x1f9d78;
const S_DTHIST: u32 = 0x1f9d94;
const S_DTVAR: u32 = 0x1f9db4;
const S_NORMAL: u32 = 0x1fa2dc;
const S_PROMPT: u32 = 0x1fa89c;
const S_LATIN: u32 = 0x1fa8a4;
const S_PHON: u32 = 0x1db2d4;

/// `fn_1f98c8` caller frame: locals live at `scratch + 0x2000`, the nested
/// `fn_275928` frame at `scratch + 0x3000` (both inside the machine stack
/// window, never compared). The nested `fn_26ff18`/`fn_1ec7cc` frames the
/// host hands out sit above the host scratch base and never overlap these.
const F198C8_LOC: u32 = 0x2000;
const F198C8_S2759: u32 = 0x3000;
const L_TMP: u32 = 0x00;
const L_NUMBUF: u32 = 0x54;
const L_KEY: u32 = 0x58;
const L_DELIDS: u32 = 0xe0;
const L_N: u32 = 0x1e0;
const L_X: u32 = 0x1e2;
const L_HAS: u32 = 0x1e4;
const L_W: u32 = 0x1e6;
const L_NMK: u32 = 0x1e8;
const L_LEN: u32 = 0x1ea;
const L_TEXTPOS: u32 = 0x1ec;
const L_VAL: u32 = 0x1f0;
const L_SEP: u32 = 0x1f4;
const L_TYPE: u32 = 0x1f5;
const L_WTYPE: u32 = 0x1f6;
const L_POSC: u32 = 0x1f7;
const L_WPOS: u32 = 0x1f8;
const L_W2: u32 = 0x1fa;
const L_START: u32 = 0x1fc;
const L_END: u32 = 0x1fe;
const L_ROOT: u32 = 0x200;
const L_TOK: u32 = 0x202;
const L_TEXT: u32 = 0x204;
const L_STR4: u32 = 0x208;
const L_STR3: u32 = 0x20c;
const L_STR6: u32 = 0x210;
const L_MARK: u32 = 0x214;
const L_STR5: u32 = 0x218;
const L_OPT: u32 = 0x21c;
const L_MI: u32 = 0x220;
const L_NM: u32 = 0x222;
const L_WSTART: u32 = 0x224;
const L_WEND: u32 = 0x226;
const L_PREV: u32 = 0x228;
const L_LAST: u32 = 0x22a;
const L_NEWT: u32 = 0x22c;
const L_ONE: u32 = 0x22e;
const L_F7: u32 = 0x230;
const L_CUR: u32 = 0x232;
const L_POS: u32 = 0x234;
const L_CLM: u32 = 0x2bc;
const L_NO4: u32 = 0x2c0;
const L_CODE: u32 = 0x2c4;
const L_CLMSIZE: u32 = 0x2c8;
/// Caller stack scratch for the nested `fn_1ede44_m` tokenize frame: 50
/// bytes at the ROM's sp-relative offsets (lane b4's `S_*` slots). Free
/// locals space: the last local ends at 0x2cc, the nested 275928 frame
/// starts at `F198C8_S2759` (+0x1000 from the locals base).
const L_TOKFRAME: u32 = 0x300;

/// Read the `pos` field of marker `i` at `base`.
fn f198c8_mark_pos(m: &Memory<'_>, base: u32, i: u16) -> u32 {
    g_word(
        m,
        base
            .wrapping_add(u32::from(i).wrapping_mul(MARKER_LEN))
            .wrapping_add(MARKER_POS),
    )
    .unwrap_or(0)
}

/// Copy a Rust byte string plus terminator into the arena.
fn f198c8_puts(m: &mut Memory<'_>, dst: u32, s: &[u8]) -> bool {
    let mut i = 0u32;
    for &ch in s {
        if !p_byte(m, dst.wrapping_add(i), ch) {
            return false;
        }
        i += 1;
    }
    p_byte(m, dst.wrapping_add(i), 0)
}

/// `fn_1f98c8(fe, b, a, c, done)`: the live one-word root, direct port of
/// src/fn_1f7ed0.c:938-2351 over the caller's arena. `scratch` is a
/// caller-owned frame: locals live at `scratch + 0x2000`, the nested
/// `fn_275928` frame at `scratch + 0x3000`; the heap buffers the ROM
/// `malloc`s (`textCopy`, `wordBuf`, `clmBuf`, `altBuf`, `mk`) come from
/// the guest heap. Every record/config/translator call is issued through
/// the host with the vtable slot the ROM reads; `fn_275928` nests
/// directly, `getString` binds lane b4's `fn_1ea4e0_m`. The language-ID
/// (`fn_1f932c`) and cross-language-map sections are present but the
/// certified capture leaves them through the `noExtClc` gate, as does the
/// alternative-transcription section (`dpsMultiTrans == 0` here).
#[allow(clippy::too_many_arguments)]
pub fn fn_1f98c8_m(
    m: &mut Memory<'_>,
    h: &mut impl Host,
    scratch: u32,
    p: u32,
    b: u32,
    a: u32,
    c: u32,
    done: u32,
) -> u32 {
    let (rc, bufs) = fn_1f98c8_inner(m, h, scratch, p, b, a, c, done);
    // The ROM's `fail`/`out` tails free the heap buffers; free the same
    // set here so every exit path matches.
    let env = g_word(m, p).unwrap_or(0);
    let heap = g_word(m, env.wrapping_add(4)).unwrap_or(0);
    for q in bufs {
        if q != 0 {
            h.call_m(0x196428, &[heap, q], m);
        }
    }
    rc
}

/// The body of `fn_1f98c8_m`: returns the status and the five heap
/// buffers (`textCopy`, `wordBuf`, `clmBuf`, `altBuf`, `mk`) for the
/// caller to free, exactly the set the ROM frees on its `fail`/`out` tails.
#[allow(clippy::too_many_arguments)]
fn fn_1f98c8_inner(
    m: &mut Memory<'_>,
    h: &mut impl Host,
    scratch: u32,
    p: u32,
    b: u32,
    a: u32,
    c: u32,
    done: u32,
) -> (u32, [u32; 5]) {
    // The G2P text offset is per root call: each call gets a fresh text
    // buffer (`text_copy` below), so the accumulated normalization shift
    // must restart at zero. The ARM achieves this with a fresh G2P context
    // per call (heap zero-on-alloc; the C has no in-engine reset); the live
    // arena reuses the object, so reset it explicitly. Without this, a
    // second speak in one session reads text at a stale shifted offset.
    // (seg1 r11: this unblocked the 26 prosody.rs:3416 panics.)
    if !p_half(m, p.wrapping_add(0x50), 0) {
        return (FAIL_MEM, [0u32; 5]);
    }
    let loc = scratch.wrapping_add(F198C8_LOC);
    let s2759 = scratch.wrapping_add(F198C8_S2759);
    let at = |o: u32| loc.wrapping_add(o);
    // normStr lives in the ARM's own frame at body SP + 0x220 (the two
    // 0x195cd0 strcpy dsts at 0x1f9fd4/0x1fa00c and the 0x195d58 arg at
    // 0x1fa548). The old scratch+0x2060 window has no ARM counterpart,
    // so its bytes diffed against ARM zeros (seg6 round 14).
    let normstr = SP_1F98C8_ENTRY.wrapping_sub(F1F98C8_FRAME).wrapping_add(0x220);
    let zero5 = [0u32; 5];
    let env = g_word(m, p).unwrap_or(0);
    let heap = g_word(m, env.wrapping_add(4)).unwrap_or(0);
    let log = g_word(m, env.wrapping_add(16)).unwrap_or(0);
    let params = g_word(m, env.wrapping_add(20)).unwrap_or(0);
    let out_vt = g_word(m, p.wrapping_add(FE_OUT)).unwrap_or(0);
    let h_a = g_word(m, p.wrapping_add(FE_CFG_HANDLE)).unwrap_or(0);
    let h_b = g_word(m, p.wrapping_add(FE_CFG_HANDLE.wrapping_add(4))).unwrap_or(0);
    let cfg_vt = g_word(m, p.wrapping_add(FE_CFG_VT)).unwrap_or(0);
    let h2_a = g_word(m, p.wrapping_add(FE_TRANS_HANDLE)).unwrap_or(0);
    let h2_b = g_word(m, p.wrapping_add(FE_TRANS_HANDLE.wrapping_add(4))).unwrap_or(0);
    let trans_vt = g_word(m, p.wrapping_add(FE_TRANS_VT)).unwrap_or(0);
    let f_add_child = g_word(m, out_vt.wrapping_add(OUT_ADD_CHILD * 4)).unwrap_or(0);
    let f_new_record = g_word(m, out_vt.wrapping_add(OUT_NEW_RECORD * 4)).unwrap_or(0);
    let f_insert_before = g_word(m, out_vt.wrapping_add(OUT_INSERT_BEFORE * 4)).unwrap_or(0);
    let f_get_first = g_word(m, out_vt.wrapping_add(OUT_GET_FIRST * 4)).unwrap_or(0);
    let f_get_next = g_word(m, out_vt.wrapping_add(OUT_GET_NEXT * 4)).unwrap_or(0);
    let f_set_field = g_word(m, out_vt.wrapping_add(OUT_SET_FIELD * 4)).unwrap_or(0);
    let f_get_field = g_word(m, out_vt.wrapping_add(OUT_GET_FIELD * 4)).unwrap_or(0);
    let f_get_field_ptr = g_word(m, out_vt.wrapping_add(OUT_GET_FIELD_PTR * 4)).unwrap_or(0);
    let f_has_field = g_word(m, out_vt.wrapping_add(OUT_HAS_FIELD * 4)).unwrap_or(0);
    let f_remove = g_word(m, out_vt.wrapping_add(OUT_REMOVE * 4)).unwrap_or(0);
    let f_get_string = g_word(m, cfg_vt.wrapping_add(CFG_GET_STRING * 4)).unwrap_or(0);
    let _f_set_markers = g_word(m, cfg_vt.wrapping_add(CFG_SET_MARKERS * 4)).unwrap_or(0);
    let f_get_pos = g_word(m, trans_vt.wrapping_add(TRANS_GET_POS * 4)).unwrap_or(0);
    let f_set_pos = g_word(m, trans_vt.wrapping_add(TRANS_SET_POS * 4)).unwrap_or(0);
    use crate::component_live_b4::fn_1ea4e0_m;
    // rc = fn_176f94(p, b, 0xf384, 0x4c8); *done = 1.
    let mut status = h.call_m(0x176f94, &[p, b, 0xf384, 0x4c8], m);
    if status >= 0x80000000 {
        return (0x89b02008, zero5);
    }
    if !p_byte(m, done, 1) {
        return (FAIL_MEM, zero5);
    }
    // clmVariants from the "clmvariants" param.
    status = h.call_m(0x1756d0, &[params, S_CLMVARIANTS, at(L_OPT), 0], m);
    let mut clm_variants = 1u8;
    if status >= 0x80000000
        || h.call_m(0x1769d0, &[g_word(m, at(L_OPT)).unwrap_or(0), S_NO], m) == 0
    {
        clm_variants = 0;
    }
    if !p_byte(m, p.wrapping_add(FE_CLM_VARIANTS), clm_variants) {
        return (FAIL_MEM, zero5);
    }
    // clcExtraMainTrans from "clcmlextramaintrans" (+ extraMainTrans).
    status = h.call_m(0x1756d0, &[params, S_CLCMLEXTRA, at(L_OPT), 0], m);
    let extra_main = g_byte(m, p.wrapping_add(FE_EXTRA_MAIN_TRANS)).unwrap_or(0);
    let mut clc_extra = 1u8;
    if status >= 0x80000000
        || h.call_m(0x1769d0, &[g_word(m, at(L_OPT)).unwrap_or(0), S_NO], m) == 0
        || extra_main == 0
    {
        clc_extra = 0;
    }
    if !p_byte(m, p.wrapping_add(FE_CLC_EXTRA_MAIN_TRANS), clc_extra) {
        return (FAIL_MEM, zero5);
    }
    // nOutTrans / dpsMultiTrans from "numberoutputtranscriptions".
    status = h.call_m(0x1756d0, &[params, S_NUMOUTTRANS, at(L_OPT), 0], m);
    let mut n_out_trans = 1u8;
    let mut dps_multi = g_byte(m, p.wrapping_add(FE_DPS_MULTI_TRANS)).unwrap_or(0);
    if status < 0x80000000 && g_word(m, at(L_OPT)).unwrap_or(0) != 0 {
        let nt = h.call_m(0x17c550, &[g_word(m, at(L_OPT)).unwrap_or(0)], m) as u8;
        n_out_trans = nt;
        // NOTE: the ROM compares in `int` (`nt - 1 <= dps` with nt == 0
        // is true and stores 0xff); a u8 compare would diverge there.
        if (i32::from(nt) - 1) <= i32::from(dps_multi) {
            dps_multi = nt.wrapping_sub(1);
        }
    }
    if !p_byte(m, p.wrapping_add(FE_N_OUT_TRANS), n_out_trans)
        || !p_byte(m, p.wrapping_add(FE_DPS_MULTI_TRANS), dps_multi)
    {
        return (FAIL_MEM, zero5);
    }
    // noRichTrans from "norichtranscription" (+ the fecfg gate).
    status = h.call_m(0x1756d0, &[params, S_NORICHTRANS, at(L_OPT), 0], m);
    let mut no_rich = 0u8;
    if !(status >= 0x80000000
        || h.call_m(0x1769d0, &[g_word(m, at(L_OPT)).unwrap_or(0), S_FALSE], m) == 0)
    {
        no_rich = 1;
    }
    status = h.call_m(0x1756d0, &[params, S_FECFG, at(L_OPT), 0], m);
    if no_rich != 0
        && status < 0x80000000
        && h.call_m(0x1769d0, &[g_word(m, at(L_OPT)).unwrap_or(0), S_MPTHREE], m) != 0
        && h.call_m(0x1769d0, &[g_word(m, at(L_OPT)).unwrap_or(0), S_MPTHREEML], m) != 0
        && h.call_m(0x1769d0, &[g_word(m, at(L_OPT)).unwrap_or(0), S_VADVDE], m) != 0
    {
        no_rich = 0;
    }
    if !p_byte(m, p.wrapping_add(FE_NO_RICH_TRANS), no_rich) {
        return (FAIL_MEM, zero5);
    }
    // Decision-tree pruning when the lexicon carries trees.
    let lex = g_word(m, p.wrapping_add(FE_LEX)).unwrap_or(0);
    if h.call_m(0x196764, &[lex], m) != 0 {
        let keys = [S_DTSCORE, S_DTHIST, S_DTVAR];
        let types = [1u32, 2, 5];
        let mut prune_opt = [0u32; 3];
        for (i, k) in keys.iter().enumerate() {
            status = h.call_m(0x175a1c, &[params, *k, at(L_OPT)], m);
            prune_opt[i] = if status >= 0x80000000 {
                0
            } else {
                g_word(m, at(L_OPT)).unwrap_or(0)
            };
        }
        let n_trans_opt = {
            status = h.call_m(0x1756d0, &[params, S_NUMOUTTRANS, at(L_OPT), 0], m);
            if status >= 0x80000000 {
                0
            } else {
                g_word(m, at(L_OPT)).unwrap_or(0)
            }
        };
        // iter + entries live in the LOC frame; the option struct too.
        if !p_word(m, at(0x238), 0)
            || !p_word(m, at(0x23c), 0)
            || !p_word(m, at(0x240), 0)
            || !p_byte(m, at(0x244), 0)
        {
            return (FAIL_MEM, zero5);
        }
        status = h.call_m(0x196794, &[lex, 0, 1, at(0x238)], m);
        if status < 0x80000000 {
            let mut dkey = 0u32;
            let opts = [
                (prune_opt[0], 1u8),
                (prune_opt[1], 2u8),
                (prune_opt[2], 5u8),
                (n_trans_opt, 0u8),
            ];
            loop {
                if !p_word(m, at(0x23c), dkey) {
                    return (FAIL_MEM, zero5);
                }
                status = h.call_m(0x196860, &[g_word(m, at(0x238)).unwrap_or(0), at(0x23c), at(0x240)], m);
                if status >= 0x80000000 {
                    break;
                }
                dkey = g_word(m, at(0x23c)).unwrap_or(0);
                let entry = g_word(m, at(0x240)).unwrap_or(0);
                let eid = g_word(m, entry.wrapping_add(8)).unwrap_or(0);
                for (optr, ty) in opts {
                    if optr != 0 {
                        let oval = h.call_m(0x17c550, &[optr], m);
                        if !p_byte(m, at(0x244), ty)
                            || !p_word(m, at(0x248), oval)
                        {
                            return (FAIL_MEM, zero5);
                        }
                        status = h.call_m(0x2b1814, &[eid, at(0x244), 1], m);
                        if status != 0 {
                            h.call_m(0x173dc0, &[log, D_8f9532, 0x84d4, 0], m);
                            return ((status.wrapping_add(0x1ea0)) | 0x89b02000, zero5);
                        }
                    }
                }
            }
            h.call_m(0x196830, &[g_word(m, at(0x238)).unwrap_or(0)], m);
        }
        let _ = types;
        let _ = status;
    }
    // The root word record and its text.
    status = h.call_m(f_get_first, &[a, c, 1, 0, at(L_ROOT)], m);
    if status >= 0x80000000 {
        return (status, zero5);
    }
    let root = g_half(m, at(L_ROOT)).unwrap_or(0);
    if !p_half(m, at(L_HAS), 0) {
        return (FAIL_MEM, zero5);
    }
    status = h.call_m(f_has_field, &[a, c, u32::from(root), 0, at(L_HAS)], m);
    if status >= 0x80000000 || g_half(m, at(L_HAS)).unwrap_or(0) != 1 {
        return (status, zero5);
    }
    if !p_half(m, at(L_N), 0) {
        return (FAIL_MEM, zero5);
    }
    status = h.call_m(
        f_get_field_ptr,
        &[a, c, u32::from(root), 0, at(L_TEXT), at(L_N)],
        m,
    );
    if status >= 0x80000000 || g_half(m, at(L_N)).unwrap_or(0) <= 1 {
        return (status, zero5);
    }
    let text = g_word(m, at(L_TEXT)).unwrap_or(0);
    let text_len = h.call_m(0x195d90, &[text], m);
    let mut text_copy = h.call_m(0x196394, &[heap, text_len.wrapping_add(1)], m);
    if text_copy == 0 {
        h.call_m(0x173dc0, &[log, D_8f9532, 0x84d0, 0], m);
        return (0x89b0200a, zero5);
    }
    let mut bufs = zero5;
    bufs[0] = text_copy;
    status = h.call_m(0x195cd0, &[text_copy, text], m);
    if status >= 0x80000000 {
        return (status, bufs);
    }
    // delIds[128] zeroed (the ROM's fn_3023b0 memclr; the live host has
    // no 0x3023b0 entry, so the port stores the same zeros directly).
    {
        let mut i = 0u32;
        while i < 256 {
            if !p_byte(m, at(L_DELIDS).wrapping_add(i), 0) {
                return (FAIL_MEM, bufs);
            }
            i += 1;
        }
    }
    // The input tokens and their language markers.
    if !p_half(m, at(L_N), 0) {
        return (FAIL_MEM, bufs);
    }
    status = h.call_m(
        f_get_field_ptr,
        &[a, c, u32::from(root), 1, at(L_MARK), at(L_N)],
        m,
    );
    if status >= 0x80000000 {
        return (status, bufs);
    }
    let markers = g_word(m, at(L_MARK)).unwrap_or(0);
    let n_markers = g_half(m, at(L_N)).unwrap_or(0);
    if !p_half(m, at(L_NM), n_markers) || !p_half(m, at(L_MI), 0) {
        return (FAIL_MEM, bufs);
    }
    let base = g_word(m, markers.wrapping_add(MARKER_POS)).unwrap_or(0);
    let mut mi = 0u16;
    while mi < n_markers && f198c8_mark_pos(m, markers, mi) == base {
        mi = mi.wrapping_add(1);
    }
    if !p_half(m, at(L_MI), mi) {
        return (FAIL_MEM, bufs);
    }
    // Translator position; f1a5 reset.
    if !p_word(m, at(L_TEXTPOS), 0) {
        return (FAIL_MEM, bufs);
    }
    status = h.call_m(f_get_pos, &[h2_a, h2_b, at(L_TEXTPOS), 0], m);
    if status >= 0x80000000 {
        return (status, bufs);
    }
    let mut text_pos = g_word(m, at(L_TEXTPOS)).unwrap_or(0);
    if !p_byte(m, p.wrapping_add(FE_F1A5), 0) {
        return (FAIL_MEM, bufs);
    }
    status = h.call_m(f_get_first, &[a, c, 3, u32::from(root), at(L_TOK)], m);
    if status >= 0x80000000 {
        return (status, bufs);
    }
    let mut tok = g_half(m, at(L_TOK)).unwrap_or(0);
    if !p_half(m, at(L_LEN), 0xffff) {
        return (FAIL_MEM, bufs);
    }
    // cfg getString(h, "fecfg", "normal"): lane b4's port, or the host
    // when the vtable names another entry.
    if f_get_string == 0x1ea4e0 {
        status = fn_1ea4e0_m(m, h, h_a, h_b, S_FECFG, S_NORMAL, at(L_VAL), at(L_LEN), at(L_SEP));
    } else {
        status = h.call_m(
            f_get_string,
            &[h_a, h_b, S_FECFG, S_NORMAL, at(L_VAL), at(L_LEN), at(L_SEP)],
            m,
        );
    }
    if status >= 0x80000000 {
        return (status, bufs);
    }
    let mut len = g_half(m, at(L_LEN)).unwrap_or(0);
    let val = g_word(m, at(L_VAL)).unwrap_or(0);
    let sep = g_byte(m, at(L_SEP)).unwrap_or(0);
    status = h.call_m(0x195cd0, &[normstr, S_NORMAL], m);
    if status >= 0x80000000 {
        return (status, bufs);
    }
    if len != 0 {
        let v0 = g_word(m, val).unwrap_or(0);
        let s = h.call_m(0x195d30, &[v0, u32::from(sep)], m);
        if s != 0 && !p_byte(m, s, 0) {
            return (FAIL_MEM, bufs);
        }
        status = h.call_m(0x195cd0, &[normstr, g_word(m, val).unwrap_or(0)], m);
        if status >= 0x80000000 {
            return (status, bufs);
        }
    }
    // The f173 language-mode marker pair.
    let f173 = g_byte(m, p.wrapping_add(FE_F173)).unwrap_or(0);
    let mut mk: u32 = 0;
    let mut cap_mk = 0u16;
    let mut n_mk = 0u16;
    if f173 == 1 {
        mk = h.call_m(0x1963b8, &[heap, 1, 11 * MARKER_LEN], m);
        if mk == 0 {
            h.call_m(0x173dc0, &[log, D_8f9532, 0x84d0, 0], m);
            return (0x89b0200a, bufs);
        }
        bufs[4] = mk;
        cap_mk = 10;
        n_mk = 2;
        let m0 = mk;
        let m1 = mk.wrapping_add(MARKER_LEN);
        if !p_half(m, m0.wrapping_add(MARKER_TYPE), 0x4000)
            || !p_word(m, m0.wrapping_add(MARKER_POS), base)
            || !p_half(m, m1.wrapping_add(MARKER_TYPE), 36)
            || !p_word(m, m1.wrapping_add(MARKER_POS), base)
            || !p_word(m, m1.wrapping_add(MARKER_VAL), D_8f9580)
        {
            return (FAIL_MEM, bufs);
        }
    }
    // Copy the phon-typed input tokens into word records.
    let mut first = true;
    let mut mode = b'N';
    let esc_tn = g_word(m, p.wrapping_add(FE_LANG_CODE).wrapping_add(4)).unwrap_or(0);
    if !p_half(m, at(L_TOK), tok) {
        return (FAIL_MEM, bufs);
    }
    while tok != 0 {
        if !p_byte(m, at(L_TYPE), 0) || !p_half(m, at(L_N), 0) {
            return (FAIL_MEM, bufs);
        }
        status = h.call_m(f_get_field, &[a, c, u32::from(tok), 0, 1, at(L_TYPE), at(L_N)], m);
        if status >= 0x80000000 {
            return (status, bufs);
        }
        let wtype = g_byte(m, at(L_TYPE)).unwrap_or(0);
        if wtype == 5 || wtype == 6 || wtype == 7 {
            if !p_half(m, at(L_START), 0)
                || !p_half(m, at(L_END), 0)
                || !p_word(m, at(L_STR4), 0)
            {
                return (FAIL_MEM, bufs);
            }
            status = h.call_m(f_get_field, &[a, c, u32::from(tok), 1, 1, at(L_START), at(L_N)], m);
            if status >= 0x80000000 {
                return (status, bufs);
            }
            status = h.call_m(f_get_field, &[a, c, u32::from(tok), 2, 1, at(L_END), at(L_N)], m);
            if status >= 0x80000000 {
                return (status, bufs);
            }
            let start = g_half(m, at(L_START)).unwrap_or(0);
            let end = g_half(m, at(L_END)).unwrap_or(0);
            status = h.call_m(
                f_get_field_ptr,
                &[a, c, u32::from(tok), 4, at(L_STR4), at(L_N)],
                m,
            );
            if status >= 0x80000000 {
                return (status, bufs);
            }
            let str4 = g_word(m, at(L_STR4)).unwrap_or(0);
            let mut str3 = 0u32;
            if h.call_m(0x195d58, &[str4, S_PHON], m) == 0 {
                if !p_word(m, at(L_STR3), 0) {
                    return (FAIL_MEM, bufs);
                }
                status = h.call_m(
                    f_get_field_ptr,
                    &[a, c, u32::from(tok), 3, at(L_STR3), at(L_N)],
                    m,
                );
                if status >= 0x80000000 {
                    return (status, bufs);
                }
                str3 = g_word(m, at(L_STR3)).unwrap_or(0);
            }
            let mut str6 = 0u32;
            if wtype == 5 {
                if !p_half(m, at(L_HAS), 0) {
                    return (FAIL_MEM, bufs);
                }
                status = h.call_m(f_has_field, &[a, c, u32::from(tok), 6, at(L_HAS)], m);
                if status >= 0x80000000 {
                    return (status, bufs);
                }
                if g_half(m, at(L_HAS)).unwrap_or(0) == 1 {
                    if !p_word(m, at(L_STR6), 0) {
                        return (FAIL_MEM, bufs);
                    }
                    status = h.call_m(
                        f_get_field_ptr,
                        &[a, c, u32::from(tok), 6, at(L_STR6), at(L_N)],
                        m,
                    );
                    if status >= 0x80000000 {
                        return (status, bufs);
                    }
                    str6 = g_word(m, at(L_STR6)).unwrap_or(0);
                }
            }
            let mut w = g_half(m, at(L_W)).unwrap_or(0);
            if first {
                status =
                    h.call_m(f_add_child, &[a, c, 2, u32::from(root), at(L_W)], m);
                if status >= 0x80000000 {
                    return (status, bufs);
                }
                first = false;
            } else {
                status = h.call_m(f_new_record, &[a, c, u32::from(w), at(L_W)], m);
                if status >= 0x80000000 {
                    return (status, bufs);
                }
            }
            w = g_half(m, at(L_W)).unwrap_or(0);
            let s4len = h.call_m(0x195d90, &[str4], m);
            macro_rules! setf {
                ($id:expr, $f:expr, $n:expr, $v:expr) => {{
                    status = h.call_m(f_set_field, &[a, c, u32::from($id), $f, $n, $v, at(L_X)], m);
                    if status >= 0x80000000 {
                        return (status, bufs);
                    }
                }};
            }
            if !p_byte(m, at(L_TYPE), wtype) {
                return (FAIL_MEM, bufs);
            }
            setf!(w, 0, 1, at(L_TYPE));
            if !p_half(m, at(L_START), start) || !p_half(m, at(L_END), end) {
                return (FAIL_MEM, bufs);
            }
            setf!(w, 1, 1, at(L_START));
            setf!(w, 2, 1, at(L_END));
            setf!(w, 4, s4len.wrapping_add(1), str4);
            if h.call_m(0x195d58, &[str4, S_PHON], m) == 0 {
                let s3len = h.call_m(0x195d90, &[str3], m);
                setf!(w, 3, s3len.wrapping_add(1), str3);
            }
            if wtype == 5 && str6 != 0 {
                let s6len = h.call_m(0x195d90, &[str6], m);
                setf!(w, 9, s6len.wrapping_add(1), str6);
            }
        }
        if f173 == 1 && wtype == 5 {
            let str4 = g_word(m, at(L_STR4)).unwrap_or(0);
            if str4 != 0 {
                let mut new_mode = b'N';
                if h.call_m(0x195d58, &[str4, S_PHON], m) != 0
                    && h.call_m(0x195d58, &[str4, S_PROMPT], m) != 0
                {
                    if esc_tn == 0 {
                        return (0x89b02006, bufs);
                    }
                    let hit = h.call_m(0x1960a8, &[esc_tn, str4], m);
                    if h.call_m(0x195d58, &[str4, S_LATIN], m) == 0 {
                        new_mode = b'L';
                    } else if h.call_m(0x195d58, &[str4, normstr], m) != 0 {
                        let s4l = h.call_m(0x195d90, &[str4], m);
                        if hit == 0
                            || g_byte(m, hit.wrapping_add(s4l)).unwrap_or(0) != b'|'
                        {
                            new_mode = b'L';
                        } else {
                            let mut kk = s4l;
                            let mut ss = str4;
                            new_mode = b'N';
                            while kk != 0 {
                                if g_byte(m, ss).unwrap_or(0) == b'_' {
                                    new_mode = b'L';
                                }
                                ss = ss.wrapping_add(1);
                                kk = kk.wrapping_sub(1);
                            }
                        }
                    }
                }
                let start = g_half(m, at(L_START)).unwrap_or(0);
                if mode != new_mode {
                    if start == 0 {
                        if n_mk >= 2 && !p_word(m, mk.wrapping_add(MARKER_LEN).wrapping_add(MARKER_VAL), if new_mode == b'L' { D_8f9574 } else { D_8f9580 }) {
                            return (FAIL_MEM, bufs);
                        }
                    } else {
                        if n_mk.wrapping_add(1) > cap_mk {
                            let grown = h.call_m(
                                0x1963e0,
                                &[heap, mk, u32::from(cap_mk.wrapping_add(11)).wrapping_mul(MARKER_LEN)],
                                m,
                            );
                            if grown == 0 {
                                h.call_m(0x173dc0, &[log, D_8f9532, 0x84d0, 0], m);
                                return (0x89b0200a, bufs);
                            }
                            mk = grown;
                            bufs[4] = mk;
                            cap_mk = cap_mk.wrapping_add(10);
                        }
                        let e = mk.wrapping_add(u32::from(n_mk).wrapping_mul(MARKER_LEN));
                        if !p_half(m, e.wrapping_add(MARKER_TYPE), 36)
                            || !p_word(m, e.wrapping_add(MARKER_POS), base.wrapping_add(u32::from(start)))
                            || !p_word(
                                m,
                                e.wrapping_add(MARKER_VAL),
                                if new_mode == b'L' { D_8f9574 } else { D_8f9580 },
                            )
                        {
                            return (FAIL_MEM, bufs);
                        }
                        n_mk = n_mk.wrapping_add(1);
                    }
                }
                mode = new_mode;
            }
        }
        let prev = tok;
        if !p_half(m, at(L_TOK), 0) {
            return (FAIL_MEM, bufs);
        }
        status = h.call_m(f_get_next, &[a, c, u32::from(prev), at(L_TOK)], m);
        if status >= 0x80000000 {
            return (status, bufs);
        }
        tok = g_half(m, at(L_TOK)).unwrap_or(0);
        status = h.call_m(f_remove, &[a, c, u32::from(prev)], m);
        if status >= 0x80000000 {
            return (status, bufs);
        }
    }
    // Publish the markers (the f173 pair or the caller's own). The ROM's
    // setMarkers is lane b4's `fn_1ede44_m` (same six-word entry), whose
    // 20-byte tokenize frame lives in caller stack scratch at the ROM's
    // sp-relative offsets (50 bytes at `at(L_TOKFRAME)`).
    {
        use crate::component_live_b4::fn_1ede44_m;
        if f173 == 1 {
            status = fn_1ede44_m(m, h, at(L_TOKFRAME), h_a, h_b, a, c, mk, n_mk);
        } else {
            status =
                fn_1ede44_m(m, h, at(L_TOKFRAME), h_a, h_b, a, c, markers, n_markers);
        }
    }
    if status >= 0x80000000 {
        return (status, bufs);
    }
    // Merge the remaining input tokens into the word list by position.
    if !p_half(m, at(L_TOK), 0) || !p_half(m, at(L_W), 0) {
        return (FAIL_MEM, bufs);
    }
    status = h.call_m(f_get_first, &[a, c, 3, u32::from(root), at(L_TOK)], m);
    if status >= 0x80000000 {
        return (status, bufs);
    }
    tok = g_half(m, at(L_TOK)).unwrap_or(0);
    status = h.call_m(f_get_first, &[a, c, 2, u32::from(root), at(L_W)], m);
    if status >= 0x80000000 {
        return (status, bufs);
    }
    let mut w = g_half(m, at(L_W)).unwrap_or(0);
    if !p_half(m, at(L_WPOS), 0) {
        return (FAIL_MEM, bufs);
    }
    status = h.call_m(f_get_field, &[a, c, u32::from(w), 1, 1, at(L_WPOS), at(L_N)], m);
    if status >= 0x80000000 {
        return (status, bufs);
    }
    let mut wpos = g_half(m, at(L_WPOS)).unwrap_or(0);
    if wpos != 0 {
        if !p_half(m, at(L_W), 0) {
            return (FAIL_MEM, bufs);
        }
        status = h.call_m(f_insert_before, &[a, c, u32::from(w), at(L_W)], m);
        if status >= 0x80000000 {
            return (status, bufs);
        }
        w = g_half(m, at(L_W)).unwrap_or(0);
        if !p_byte(m, at(L_TYPE), 0) || !p_half(m, at(L_START), 0) || !p_half(m, at(L_END), 0) {
            return (FAIL_MEM, bufs);
        }
        status = h.call_m(f_get_field, &[a, c, u32::from(tok), 0, 1, at(L_TYPE), at(L_N)], m);
        if status >= 0x80000000 {
            return (status, bufs);
        }
        status = h.call_m(f_get_field, &[a, c, u32::from(tok), 1, 1, at(L_START), at(L_N)], m);
        if status >= 0x80000000 {
            return (status, bufs);
        }
        status = h.call_m(f_get_field, &[a, c, u32::from(tok), 2, 1, at(L_END), at(L_N)], m);
        if status >= 0x80000000 {
            return (status, bufs);
        }
        macro_rules! setf2 {
            ($id:expr, $f:expr, $n:expr, $v:expr) => {{
                status = h.call_m(f_set_field, &[a, c, u32::from($id), $f, $n, $v, at(L_X)], m);
                if status >= 0x80000000 {
                    return (status, bufs);
                }
            }};
        }
        setf2!(w, 0, 1, at(L_TYPE));
        setf2!(w, 1, 1, at(L_START));
        setf2!(w, 2, 1, at(L_END));
        if !p_half(m, at(L_TOK), 0) {
            return (FAIL_MEM, bufs);
        }
        status = h.call_m(f_get_next, &[a, c, u32::from(tok), at(L_TOK)], m);
        if status >= 0x80000000 {
            return (status, bufs);
        }
        tok = g_half(m, at(L_TOK)).unwrap_or(0);
    }
    if !p_half(m, at(L_START), 0) {
        return (FAIL_MEM, bufs);
    }
    let mut start = 0u16;
    let mut last = w;
    while (wpos as i16) < (start as i16) && w != 0 {
        last = w;
        if !p_half(m, at(L_W), 0) {
            return (FAIL_MEM, bufs);
        }
        status = h.call_m(f_get_next, &[a, c, u32::from(w), at(L_W)], m);
        if status >= 0x80000000 {
            return (status, bufs);
        }
        w = g_half(m, at(L_W)).unwrap_or(0);
        if w != 0 {
            if !p_half(m, at(L_WPOS), 0) {
                return (FAIL_MEM, bufs);
            }
            status = h.call_m(f_get_field, &[a, c, u32::from(w), 1, 1, at(L_WPOS), at(L_N)], m);
            if status >= 0x80000000 {
                return (status, bufs);
            }
            wpos = g_half(m, at(L_WPOS)).unwrap_or(0);
        } else {
            wpos = h.call_m(0x195d90, &[text], m) as u16;
        }
    }
    w = last;
    while tok != 0 {
        if !p_byte(m, at(L_TYPE), 0)
            || !p_half(m, at(L_START), 0)
            || !p_half(m, at(L_END), 0)
        {
            return (FAIL_MEM, bufs);
        }
        status = h.call_m(f_get_field, &[a, c, u32::from(tok), 0, 1, at(L_TYPE), at(L_N)], m);
        if status >= 0x80000000 {
            return (status, bufs);
        }
        status = h.call_m(f_get_field, &[a, c, u32::from(tok), 1, 1, at(L_START), at(L_N)], m);
        if status >= 0x80000000 {
            return (status, bufs);
        }
        let ttype = g_byte(m, at(L_TYPE)).unwrap_or(0);
        start = g_half(m, at(L_START)).unwrap_or(0);
        last = w;
        while (wpos as i16) < (start as i16) && w != 0 {
            last = w;
            if !p_half(m, at(L_W), 0) {
                return (FAIL_MEM, bufs);
            }
            status = h.call_m(f_get_next, &[a, c, u32::from(w), at(L_W)], m);
            if status >= 0x80000000 {
                return (status, bufs);
            }
            w = g_half(m, at(L_W)).unwrap_or(0);
            if w != 0 {
                if !p_half(m, at(L_WPOS), 0) {
                    return (FAIL_MEM, bufs);
                }
                status = h.call_m(f_get_field, &[a, c, u32::from(w), 1, 1, at(L_WPOS), at(L_N)], m);
                if status >= 0x80000000 {
                    return (status, bufs);
                }
                wpos = g_half(m, at(L_WPOS)).unwrap_or(0);
            } else {
                wpos = h.call_m(0x195d90, &[text], m) as u16;
            }
        }
        if start != wpos {
            w = last;
        }
        if !p_half(m, at(L_END), 0) || !p_byte(m, at(L_WTYPE), 0) {
            return (FAIL_MEM, bufs);
        }
        status = h.call_m(f_get_field, &[a, c, u32::from(tok), 2, 1, at(L_END), at(L_N)], m);
        if status >= 0x80000000 {
            return (status, bufs);
        }
        status = h.call_m(f_get_field, &[a, c, u32::from(w), 0, 1, at(L_WTYPE), at(L_N)], m);
        if status >= 0x80000000 {
            return (status, bufs);
        }
        let end = g_half(m, at(L_END)).unwrap_or(0);
        let wtype = g_byte(m, at(L_WTYPE)).unwrap_or(0);
        if !p_half(m, at(L_START), start)
            || !p_half(m, at(L_END), end)
            || !p_byte(m, at(L_TYPE), ttype)
        {
            return (FAIL_MEM, bufs);
        }
        if wtype == 5 && start == wpos {
            if !p_half(m, at(L_W), 0) {
                return (FAIL_MEM, bufs);
            }
            status = h.call_m(f_insert_before, &[a, c, u32::from(w), at(L_W)], m);
            if status >= 0x80000000 {
                return (status, bufs);
            }
            w = g_half(m, at(L_W)).unwrap_or(0);
            status = h.call_m(f_set_field, &[a, c, u32::from(w), 0, 1, at(L_TYPE), at(L_X)], m);
            if status >= 0x80000000 {
                return (status, bufs);
            }
            status = h.call_m(f_set_field, &[a, c, u32::from(w), 1, 1, at(L_START), at(L_X)], m);
            if status >= 0x80000000 {
                return (status, bufs);
            }
            status = h.call_m(f_set_field, &[a, c, u32::from(w), 2, 1, at(L_END), at(L_X)], m);
            if status >= 0x80000000 {
                return (status, bufs);
            }
            if !p_half(m, at(L_W), 0) {
                return (FAIL_MEM, bufs);
            }
            status = h.call_m(f_get_next, &[a, c, u32::from(w), at(L_W)], m);
            if status >= 0x80000000 {
                return (status, bufs);
            }
            w = g_half(m, at(L_W)).unwrap_or(0);
        } else {
            if !p_half(m, at(L_W), 0) {
                return (FAIL_MEM, bufs);
            }
            status = h.call_m(f_new_record, &[a, c, u32::from(w), at(L_W)], m);
            if status >= 0x80000000 {
                return (status, bufs);
            }
            w = g_half(m, at(L_W)).unwrap_or(0);
            status = h.call_m(f_set_field, &[a, c, u32::from(w), 0, 1, at(L_TYPE), at(L_X)], m);
            if status >= 0x80000000 {
                return (status, bufs);
            }
            status = h.call_m(f_set_field, &[a, c, u32::from(w), 1, 1, at(L_START), at(L_X)], m);
            if status >= 0x80000000 {
                return (status, bufs);
            }
            status = h.call_m(f_set_field, &[a, c, u32::from(w), 2, 1, at(L_END), at(L_X)], m);
            if status >= 0x80000000 {
                return (status, bufs);
            }
        }
        let prev = tok;
        if !p_half(m, at(L_TOK), 0) {
            return (FAIL_MEM, bufs);
        }
        status = h.call_m(f_get_next, &[a, c, u32::from(prev), at(L_TOK)], m);
        if status >= 0x80000000 {
            return (status, bufs);
        }
        tok = g_half(m, at(L_TOK)).unwrap_or(0);
        status = h.call_m(f_remove, &[a, c, u32::from(prev)], m);
        if status >= 0x80000000 {
            return (status, bufs);
        }
    }
    // Run the word-level lookups (dictionary words and phonetic input).
    // `str4_last` keeps the last word-loop mode string for the
    // alternative-transcription section below, mirroring the C local.
    let mut str4_last = 0u32;
    if !p_half(m, at(L_TOK), 0) {
        return (FAIL_MEM, bufs);
    }
    status = h.call_m(f_get_first, &[a, c, 2, u32::from(root), at(L_TOK)], m);
    if status >= 0x80000000 {
        return (status, bufs);
    }
    tok = g_half(m, at(L_TOK)).unwrap_or(0);
    if !p_word(m, at(L_TEXT), text_copy) {
        return (FAIL_MEM, bufs);
    }
    // p->f68 lives at fe + 0x68.
    if !p_half(m, p.wrapping_add(0x68), tok) {
        return (FAIL_MEM, bufs);
    }
    let mut w2 = tok;
    while tok != 0 && w2 != 0 {
        if !p_byte(m, at(L_WTYPE), 0) {
            return (FAIL_MEM, bufs);
        }
        status = h.call_m(f_get_field, &[a, c, u32::from(w2), 0, 1, at(L_WTYPE), at(L_N)], m);
        if status >= 0x80000000 {
            return (status, bufs);
        }
        let wt = g_byte(m, at(L_WTYPE)).unwrap_or(0);
        if wt == 5 {
            if !p_half(m, at(L_START), 0) || !p_half(m, at(L_END), 0) {
                return (FAIL_MEM, bufs);
            }
            status = h.call_m(f_get_field, &[a, c, u32::from(w2), 1, 1, at(L_START), at(L_N)], m);
            if status >= 0x80000000 {
                return (status, bufs);
            }
            status = h.call_m(f_get_field, &[a, c, u32::from(w2), 2, 1, at(L_END), at(L_N)], m);
            if status >= 0x80000000 {
                return (status, bufs);
            }
            let start = g_half(m, at(L_START)).unwrap_or(0);
            let end = g_half(m, at(L_END)).unwrap_or(0);
            if start != end {
                if !p_word(m, at(L_STR4), 0) {
                    return (FAIL_MEM, bufs);
                }
                status = h.call_m(
                    f_get_field_ptr,
                    &[a, c, u32::from(w2), 4, at(L_STR4), at(L_N)],
                    m,
                );
                if status >= 0x80000000 {
                    return (status, bufs);
                }
                let str4 = g_word(m, at(L_STR4)).unwrap_or(0);
                str4_last = str4;
                if h.call_m(0x195d58, &[str4, S_PHON], m) != 0 {
                    let mut rich = 0u32;
                    if !p_half(m, at(L_HAS), 0) {
                        return (FAIL_MEM, bufs);
                    }
                    status = h.call_m(f_has_field, &[a, c, u32::from(w2), 9, at(L_HAS)], m);
                    if status >= 0x80000000 {
                        return (status, bufs);
                    }
                    if g_half(m, at(L_HAS)).unwrap_or(0) == 1 {
                        rich = 1;
                    }
                    if !p_half(m, at(L_TOK), tok)
                        || !p_half(m, at(L_MI), mi)
                        || !p_half(m, at(L_NM), n_markers)
                        || !p_word(m, at(L_TEXT), text_copy)
                    {
                        return (FAIL_MEM, bufs);
                    }
                    // 275928's entry SP is this body SP: the `bl`
                    // doesn't move it. Derived from the measured 1f98c8
                    // entry SP (identical on all workloads) minus
                    // 1f98c8's frame.
                    let sp_2759 = SP_1F98C8_ENTRY.wrapping_sub(F1F98C8_FRAME);
                    status = fn_275928_m(
                        m,
                        h,
                        s2759,
                        sp_2759,
                        p,
                        at(L_TEXT),
                        start,
                        end,
                        str4,
                        a,
                        c,
                        at(L_TOK),
                        markers,
                        at(L_MI),
                        at(L_NM),
                        base,
                        rich,
                    );
                    if status >= 0x80000000 {
                        return (status, bufs);
                    }
                    // 275928 takes `&textCopy`: the acronym path reallocs
                    // `*text` on a positive delta and writes it back
                    // (src/fn_26ff18.c:1104-1107), so refresh the local
                    // and the free set from the cell.
                    text_copy = g_word(m, at(L_TEXT)).unwrap_or(0);
                    bufs[0] = text_copy;
                    tok = g_half(m, at(L_TOK)).unwrap_or(0);
                    mi = g_half(m, at(L_MI)).unwrap_or(0);
                } else {
                    if !p_word(m, at(L_STR3), 0) {
                        return (FAIL_MEM, bufs);
                    }
                    status = h.call_m(
                        f_get_field_ptr,
                        &[a, c, u32::from(w2), 3, at(L_STR3), at(L_N)],
                        m,
                    );
                    if status >= 0x80000000 {
                        return (status, bufs);
                    }
                    let str3 = g_word(m, at(L_STR3)).unwrap_or(0);
                    if !p_half(m, at(L_TOK), tok)
                        || !p_half(m, at(L_MI), mi)
                        || !p_half(m, at(L_NM), n_markers)
                    {
                        return (FAIL_MEM, bufs);
                    }
                    status = h.call_m(
                        0x274564,
                        &[
                            p,
                            text,
                            str3,
                            u32::from(start),
                            u32::from(end),
                            a,
                            c,
                            at(L_TOK),
                            markers,
                            at(L_MI),
                            at(L_NM),
                            base,
                        ],
                        m,
                    );
                    if status >= 0x80000000 {
                        return (status, bufs);
                    }
                    tok = g_half(m, at(L_TOK)).unwrap_or(0);
                    mi = g_half(m, at(L_MI)).unwrap_or(0);
                }
                w2 = tok;
            } else {
                if !p_half(m, at(L_W2), 0) {
                    return (FAIL_MEM, bufs);
                }
                status = h.call_m(f_get_next, &[a, c, u32::from(w2), at(L_W2)], m);
                if status >= 0x80000000 {
                    return (status, bufs);
                }
                w2 = g_half(m, at(L_W2)).unwrap_or(0);
            }
        } else {
            if !p_half(m, at(L_W2), 0) {
                return (FAIL_MEM, bufs);
            }
            status = h.call_m(f_get_next, &[a, c, u32::from(w2), at(L_W2)], m);
            if status >= 0x80000000 {
                return (status, bufs);
            }
            w2 = g_half(m, at(L_W2)).unwrap_or(0);
        }
    }
    // Shift the positions when the text was changed (f50).
    let f50 = g_half(m, p.wrapping_add(FE_F50)).unwrap_or(0) as i16;
    if f50 > 0 {
        let mut mmark = mi;
        while mmark < n_markers {
            let pos = f198c8_mark_pos(m, markers, mmark);
            if !p_word(
                m,
                markers.wrapping_add(u32::from(mmark).wrapping_mul(MARKER_LEN)).wrapping_add(MARKER_POS),
                pos.wrapping_add(f50 as u32),
            ) {
                return (FAIL_MEM, bufs);
            }
            mmark = mmark.wrapping_add(1);
        }
        mi = mmark;
        if !p_half(m, at(L_MI), mi) {
            return (FAIL_MEM, bufs);
        }
        let mut wstart = 0u16;
        let mut wend = 0u16;
        let mut f68 = g_half(m, p.wrapping_add(0x68)).unwrap_or(0);
        if f68 != 0 {
            if !p_half(m, at(L_WSTART), 0) {
                return (FAIL_MEM, bufs);
            }
            status = h.call_m(f_get_field, &[a, c, u32::from(f68), 1, 1, at(L_WSTART), at(L_N)], m);
            if status >= 0x80000000 {
                return (status, bufs);
            }
            wstart = g_half(m, at(L_WSTART)).unwrap_or(0);
        }
        while (h.call_m(0x195d90, &[text], m) as i16) >= (wstart as i16) && f68 != 0 {
            if !p_byte(m, at(L_WTYPE), 0) {
                return (FAIL_MEM, bufs);
            }
            status = h.call_m(f_get_field, &[a, c, u32::from(f68), 0, 1, at(L_WTYPE), at(L_N)], m);
            if status >= 0x80000000 {
                return (status, bufs);
            }
            let wt = g_byte(m, at(L_WTYPE)).unwrap_or(0);
            if wt == 5 || wt == 6 || wt == 4 || wt == 9 || wt == 10 || wt == 7 {
                if !p_half(m, at(L_WEND), 0) {
                    return (FAIL_MEM, bufs);
                }
                status = h.call_m(f_get_field, &[a, c, u32::from(f68), 2, 1, at(L_WEND), at(L_N)], m);
                if status >= 0x80000000 {
                    return (status, bufs);
                }
                wend = g_half(m, at(L_WEND)).unwrap_or(0);
                wstart = wstart.wrapping_add(f50 as u16);
                wend = wend.wrapping_add(f50 as u16);
                if !p_half(m, at(L_WSTART), wstart) {
                    return (FAIL_MEM, bufs);
                }
                status = h.call_m(
                    f_set_field,
                    &[a, c, u32::from(f68), 1, 1, at(L_WSTART), at(L_X)],
                    m,
                );
                if status >= 0x80000000 {
                    return (status, bufs);
                }
                if !p_half(m, at(L_WEND), wend) {
                    return (FAIL_MEM, bufs);
                }
                status = h.call_m(
                    f_set_field,
                    &[a, c, u32::from(f68), 2, 1, at(L_WEND), at(L_X)],
                    m,
                );
                if status >= 0x80000000 {
                    return (status, bufs);
                }
            }
            if !p_half(m, p.wrapping_add(0x68), 0) {
                return (FAIL_MEM, bufs);
            }
            status = h.call_m(f_get_next, &[a, c, u32::from(f68), p.wrapping_add(0x68)], m);
            if status >= 0x80000000 {
                return (status, bufs);
            }
            f68 = g_half(m, p.wrapping_add(0x68)).unwrap_or(0);
            if f68 != 0 {
                if !p_half(m, at(L_WSTART), 0) {
                    return (FAIL_MEM, bufs);
                }
                status = h.call_m(f_get_field, &[a, c, u32::from(f68), 1, 1, at(L_WSTART), at(L_N)], m);
                if status >= 0x80000000 {
                    return (status, bufs);
                }
                wstart = g_half(m, at(L_WSTART)).unwrap_or(0);
            }
        }
        if !p_half(m, p.wrapping_add(0x68), f68) {
            return (FAIL_MEM, bufs);
        }
    }
    text_pos = text_pos.wrapping_add(f50 as u32);
    status = h.call_m(f_set_pos, &[h2_a, h2_b, text_pos, 0], m);
    if status >= 0x80000000 {
        return (status, bufs);
    }
    if !p_word(m, at(L_TEXTPOS), text_pos) {
        return (FAIL_MEM, bufs);
    }
    let tclen = h.call_m(0x195d90, &[text_copy], m);
    // markers->len = strlen(textCopy).
    if !p_word(m, markers.wrapping_add(MARKER_LEN_OFF), tclen) {
        return (FAIL_MEM, bufs);
    }
    status = h.call_m(
        f_set_field,
        &[a, c, u32::from(root), 0, tclen.wrapping_add(1), text_copy, at(L_X)],
        m,
    );
    if status >= 0x80000000 {
        return (status, bufs);
    }
    // Drop the records below the word level.
    if !p_half(m, at(L_TOK), 0) {
        return (FAIL_MEM, bufs);
    }
    status = h.call_m(f_get_first, &[a, c, 2, u32::from(root), at(L_TOK)], m);
    if status >= 0x80000000 {
        return (status, bufs);
    }
    tok = g_half(m, at(L_TOK)).unwrap_or(0);
    let mut prev: u16 = 0;
    while tok != 0 {
        if !p_byte(m, at(L_WTYPE), 0) {
            return (FAIL_MEM, bufs);
        }
        status = h.call_m(f_get_field, &[a, c, u32::from(tok), 0, 1, at(L_WTYPE), at(L_N)], m);
        if status >= 0x80000000 {
            return (status, bufs);
        }
        let wt = g_byte(m, at(L_WTYPE)).unwrap_or(0);
        if wt <= 3 {
            status = h.call_m(f_remove, &[a, c, u32::from(tok)], m);
            if status >= 0x80000000 {
                return (status, bufs);
            }
            tok = prev;
        }
        if tok == 0 {
            if !p_half(m, at(L_TOK), 0) {
                return (FAIL_MEM, bufs);
            }
            status = h.call_m(f_get_first, &[a, c, 2, u32::from(root), at(L_TOK)], m);
        } else {
            prev = tok;
            if !p_half(m, at(L_TOK), 0) {
                return (FAIL_MEM, bufs);
            }
            status = h.call_m(f_get_next, &[a, c, u32::from(prev), at(L_TOK)], m);
        }
        if status >= 0x80000000 {
            return (status, bufs);
        }
        tok = g_half(m, at(L_TOK)).unwrap_or(0);
    }

    // Apply the markers to the words and split multi-word entries.
    mi = 0;
    if !p_half(m, at(L_MI), 0) {
        return (FAIL_MEM, bufs);
    }
    let mut cur_start = h.call_m(0x195d90, &[text_copy], m) as u16;
    let mut same_start = 0u8;
    let mut keep_alt = 0u8;
    let mut n_del = 0u16;
    let mut pos_char = 0u8;
    if !p_half(m, at(L_TOK), 0) {
        return (FAIL_MEM, bufs);
    }
    status = h.call_m(f_get_first, &[a, c, 2, u32::from(root), at(L_TOK)], m);
    if status >= 0x80000000 {
        return (status, bufs);
    }
    tok = g_half(m, at(L_TOK)).unwrap_or(0);
    while tok != 0 {
        if !p_byte(m, at(L_WTYPE), 0)
            || !p_half(m, at(L_START), 0)
            || !p_half(m, at(L_END), 0)
        {
            return (FAIL_MEM, bufs);
        }
        status = h.call_m(f_get_field, &[a, c, u32::from(tok), 0, 1, at(L_WTYPE), at(L_N)], m);
        if status >= 0x80000000 {
            return (status, bufs);
        }
        status = h.call_m(f_get_field, &[a, c, u32::from(tok), 1, 1, at(L_START), at(L_N)], m);
        if status >= 0x80000000 {
            return (status, bufs);
        }
        status = h.call_m(f_get_field, &[a, c, u32::from(tok), 2, 1, at(L_END), at(L_N)], m);
        if status >= 0x80000000 {
            return (status, bufs);
        }
        let wtype = g_byte(m, at(L_WTYPE)).unwrap_or(0);
        let mut start = g_half(m, at(L_START)).unwrap_or(0);
        let end = g_half(m, at(L_END)).unwrap_or(0);
        if wtype == 9 || wtype == 4 {
            if !p_half(m, at(L_HAS), 0) {
                return (FAIL_MEM, bufs);
            }
            status = h.call_m(f_has_field, &[a, c, u32::from(tok), 11, at(L_HAS)], m);
            if status >= 0x80000000 {
                return (status, bufs);
            }
            if g_half(m, at(L_HAS)).unwrap_or(0) == 1 {
                if !p_word(m, at(L_STR4), 0) {
                    return (FAIL_MEM, bufs);
                }
                status = h.call_m(
                    f_get_field_ptr,
                    &[a, c, u32::from(tok), 11, at(L_STR4), at(L_N)],
                    m,
                );
                if status >= 0x80000000 {
                    return (status, bufs);
                }
                let alt = g_word(m, at(L_STR4)).unwrap_or(0);
                let s = h.call_m(0x1960a8, &[alt, 0x1fb9bc], m);
                if s != 0 && !p_byte(m, s, 0) {
                    return (FAIL_MEM, bufs);
                }
                let s = h.call_m(0x1960a8, &[alt, 0x1fb9c0], m);
                if s != 0 && !p_byte(m, s, 0) {
                    return (FAIL_MEM, bufs);
                }
            }
        } else if wtype != 10 {
            if !p_half(m, at(L_TOK), 0) {
                return (FAIL_MEM, bufs);
            }
            status = h.call_m(f_get_next, &[a, c, u32::from(tok), at(L_TOK)], m);
            if status >= 0x80000000 {
                return (status, bufs);
            }
            tok = g_half(m, at(L_TOK)).unwrap_or(0);
            continue;
        }
        if start == cur_start {
            same_start = 1;
        } else {
            if same_start != 0 && keep_alt != 0 {
                let mut k = 0u8;
                while (k as u16) < n_del {
                    let id = g_half(m, at(L_DELIDS).wrapping_add((k as u32) * 2)).unwrap_or(0);
                    status = h.call_m(f_remove, &[a, c, u32::from(id)], m);
                    if status >= 0x80000000 {
                        return (status, bufs);
                    }
                    k = k.wrapping_add(1);
                }
            }
            n_del = 0;
            same_start = 0;
            keep_alt = 0;
            cur_start = start;
            pos_char = 0;
        }
        while mi < n_markers
            && ((f198c8_mark_pos(m, markers, mi).wrapping_sub(base) & 0xffff) as u16 as i16)
                < (start.wrapping_add(1) as i16)
        {
            let mb = markers.wrapping_add(u32::from(mi).wrapping_mul(MARKER_LEN));
            let mtype = g_half(m, mb.wrapping_add(MARKER_TYPE)).unwrap_or(0);
            if mtype == 37 {
                let mstr = g_word(m, mb.wrapping_add(MARKER_VAL)).unwrap_or(0);
                let need = g_wstrlen(m, mstr).unwrap_or(0).wrapping_add(1);
                let mut word_buf = bufs[1];
                let grown = h.call_m(0x1963e0, &[heap, word_buf, need], m);
                if grown == 0 {
                    h.call_m(0x173dc0, &[log, D_8f9532, 0x84d0, 0], m);
                    return (0x89b0200a, bufs);
                }
                word_buf = grown;
                bufs[1] = word_buf;
                let mut k = 0u32;
                let mut pi = 0u32;
                loop {
                    let ch = g_half(m, mstr.wrapping_add(pi * 2)).unwrap_or(0);
                    if ch == 0 {
                        break;
                    }
                    let chb = (ch & 0xff) as u8;
                    if chb != b' ' {
                        if !p_byte(m, word_buf.wrapping_add(k), chb) {
                            return (FAIL_MEM, bufs);
                        }
                        k += 1;
                    }
                    pi += 1;
                }
                if !p_byte(m, word_buf.wrapping_add(k), 0) {
                    return (FAIL_MEM, bufs);
                }
                let wblen = h.call_m(0x195d90, &[word_buf], m);
                status = h.call_m(
                    f_set_field,
                    &[a, c, u32::from(tok), 10, wblen.wrapping_add(1), word_buf, at(L_X)],
                    m,
                );
                if status >= 0x80000000 {
                    return (status, bufs);
                }
            } else if mtype == 9 {
                if !p_word(m, at(L_STR3), 0) {
                    return (FAIL_MEM, bufs);
                }
                status = h.call_m(
                    f_get_field_ptr,
                    &[a, c, u32::from(tok), 3, at(L_STR3), at(L_N)],
                    m,
                );
                if status >= 0x80000000 {
                    return (status, bufs);
                }
                let str3 = g_word(m, at(L_STR3)).unwrap_or(0);
                let s = h.call_m(0x195d30, &[str3, 0x27], m);
                if s != 0 && !p_byte(m, s, 0x13) {
                    return (FAIL_MEM, bufs);
                }
            } else if mtype == 14 {
                let mnum = g_word(m, mb.wrapping_add(MARKER_VAL)).unwrap_or(0);
                if (mnum & 0xff) != 64 {
                    if !p_half(m, at(L_LEN), 0xffff) {
                        return (FAIL_MEM, bufs);
                    }
                    // key.s = D_8f9524.pos with s[4] = num, s[5] = 0.
                    let src = D_8f9524.wrapping_add(8);
                    let mut i = 0u32;
                    while i < 8 {
                        let ch = g_byte(m, src.wrapping_add(i)).unwrap_or(0);
                        if !p_byte(m, at(L_KEY).wrapping_add(i), ch) {
                            return (FAIL_MEM, bufs);
                        }
                        i += 1;
                    }
                    if !p_byte(m, at(L_KEY).wrapping_add(4), (mnum & 0xff) as u8)
                        || !p_byte(m, at(L_KEY).wrapping_add(5), 0)
                    {
                        return (FAIL_MEM, bufs);
                    }
                    if f_get_string == 0x1ea4e0 {
                        status = fn_1ea4e0_m(
                            m, h, h_a, h_b, S_FECFG, at(L_KEY), at(L_VAL), at(L_LEN), at(L_SEP),
                        );
                    } else {
                        status = h.call_m(
                            f_get_string,
                            &[h_a, h_b, S_FECFG, at(L_KEY), at(L_VAL), at(L_LEN), at(L_SEP)],
                            m,
                        );
                    }
                    if status >= 0x80000000 {
                        return (status, bufs);
                    }
                    len = g_half(m, at(L_LEN)).unwrap_or(0);
                    if len != 0 {
                        let vv = g_word(m, g_word(m, at(L_VAL)).unwrap_or(0)).unwrap_or(0);
                        pos_char = g_byte(m, vv).unwrap_or(0);
                    } else {
                        pos_char = (mnum & 0xff) as u8;
                    }
                }
            }
            mi = mi.wrapping_add(1);
            if !p_half(m, at(L_MI), mi) {
                return (FAIL_MEM, bufs);
            }
        }
        if pos_char != 0 {
            if !p_word(m, at(L_STR5), 0) {
                return (FAIL_MEM, bufs);
            }
            status = h.call_m(
                f_get_field_ptr,
                &[a, c, u32::from(tok), 5, at(L_STR5), at(L_N)],
                m,
            );
            if status >= 0x80000000 {
                return (status, bufs);
            }
            let str5 = g_word(m, at(L_STR5)).unwrap_or(0);
            if h.call_m(0x195d30, &[str5, u32::from(pos_char)], m) != 0 {
                keep_alt = 1;
            } else {
                if !p_half(m, at(L_DELIDS).wrapping_add(u32::from(n_del) * 2), tok) {
                    return (FAIL_MEM, bufs);
                }
                n_del = n_del.wrapping_add(1);
            }
            if !p_byte(m, str5, pos_char) || !p_byte(m, str5.wrapping_add(1), 0) {
                return (FAIL_MEM, bufs);
            }
        }
        if wtype == 9 {
            if !p_word(m, at(L_STR3), 0) {
                return (FAIL_MEM, bufs);
            }
            status = h.call_m(
                f_get_field_ptr,
                &[a, c, u32::from(tok), 3, at(L_STR3), at(L_N)],
                m,
            );
            if status >= 0x80000000 {
                return (status, bufs);
            }
            let str3 = g_word(m, at(L_STR3)).unwrap_or(0);
            if h.call_m(0x195d30, &[str3, 0x20], m) != 0 {
                if !p_half(m, at(L_END), 0) || !p_half(m, at(L_F7), 0) {
                    return (FAIL_MEM, bufs);
                }
                status =
                    h.call_m(f_get_field, &[a, c, u32::from(tok), 2, 1, at(L_END), at(L_N)], m);
                if status >= 0x80000000 {
                    return (status, bufs);
                }
                let end = g_half(m, at(L_END)).unwrap_or(0);
                status =
                    h.call_m(f_get_field, &[a, c, u32::from(tok), 7, 1, at(L_F7), at(L_N)], m);
                if status >= 0x80000000 {
                    return (status, bufs);
                }
                let f7 = g_half(m, at(L_F7)).unwrap_or(0);
                if !p_word(m, at(L_STR4), 0) {
                    return (FAIL_MEM, bufs);
                }
                status = h.call_m(
                    f_get_field_ptr,
                    &[a, c, u32::from(tok), 9, at(L_STR4), at(L_N)],
                    m,
                );
                if status >= 0x80000000 {
                    return (status, bufs);
                }
                let lang = g_word(m, at(L_STR4)).unwrap_or(0);
                if !p_half(m, at(L_HAS), 0) {
                    return (FAIL_MEM, bufs);
                }
                status = h.call_m(f_has_field, &[a, c, u32::from(tok), 8, at(L_HAS)], m);
                if status >= 0x80000000 {
                    return (status, bufs);
                }
                let mut wstart = 0u16;
                if g_half(m, at(L_HAS)).unwrap_or(0) == 1 {
                    if !p_half(m, at(L_WSTART), 0) {
                        return (FAIL_MEM, bufs);
                    }
                    status = h.call_m(
                        f_get_field,
                        &[a, c, u32::from(tok), 8, 1, at(L_WSTART), at(L_N)],
                        m,
                    );
                    if status >= 0x80000000 {
                        return (status, bufs);
                    }
                    wstart = g_half(m, at(L_WSTART)).unwrap_or(0);
                }
                if !p_word(m, at(L_STR5), 0) || !p_word(m, at(L_STR6), 0) {
                    return (FAIL_MEM, bufs);
                }
                status = h.call_m(
                    f_get_field_ptr,
                    &[a, c, u32::from(tok), 5, at(L_STR5), at(L_N)],
                    m,
                );
                if status >= 0x80000000 {
                    return (status, bufs);
                }
                status = h.call_m(
                    f_get_field_ptr,
                    &[a, c, u32::from(tok), 6, at(L_STR6), at(L_N)],
                    m,
                );
                if status >= 0x80000000 {
                    return (status, bufs);
                }
                let str5 = g_word(m, at(L_STR5)).unwrap_or(0);
                let str6b = g_word(m, at(L_STR6)).unwrap_or(0);
                let need3 = h.call_m(0x195d90, &[str3], m).wrapping_add(1);
                let need5 = h.call_m(0x195d90, &[str5], m).wrapping_add(1);
                let need6 = h.call_m(0x195d90, &[str6b], m).wrapping_add(1);
                // Grow the fe scratch buffers past 128 bytes when needed.
                let b54_1 = g_word(m, p.wrapping_add(0x58)).unwrap_or(0);
                if need3 > 128 {
                    let q = h.call_m(0x1963e0, &[heap, b54_1, need3], m);
                    if q == 0 {
                        h.call_m(0x173dc0, &[log, D_8f9532, 0x84d0, 0], m);
                        return (0x89b0200a, bufs);
                    }
                    if !p_word(m, p.wrapping_add(0x58), q) {
                        return (FAIL_MEM, bufs);
                    }
                }
                let b54_2 = g_word(m, p.wrapping_add(0x5c)).unwrap_or(0);
                if need5 > 128 {
                    let q = h.call_m(0x1963e0, &[heap, b54_2, need5], m);
                    if q == 0 {
                        h.call_m(0x173dc0, &[log, D_8f9532, 0x84d0, 0], m);
                        return (0x89b0200a, bufs);
                    }
                    if !p_word(m, p.wrapping_add(0x5c), q) {
                        return (FAIL_MEM, bufs);
                    }
                }
                let b54_3 = g_word(m, p.wrapping_add(0x60)).unwrap_or(0);
                if need6 > 128 {
                    let q = h.call_m(0x1963e0, &[heap, b54_3, need6], m);
                    if q == 0 {
                        h.call_m(0x173dc0, &[log, D_8f9532, 0x84d0, 0], m);
                        return (0x89b0200a, bufs);
                    }
                    if !p_word(m, p.wrapping_add(0x60), q) {
                        return (FAIL_MEM, bufs);
                    }
                }
                let dst1 = g_word(m, p.wrapping_add(0x58)).unwrap_or(0);
                let dst2 = g_word(m, p.wrapping_add(0x5c)).unwrap_or(0);
                let dst3 = g_word(m, p.wrapping_add(0x60)).unwrap_or(0);
                status = h.call_m(0x195cd0, &[dst1, str3], m);
                if status >= 0x80000000 {
                    return (status, bufs);
                }
                status = h.call_m(0x195cd0, &[dst2, str5], m);
                if status >= 0x80000000 {
                    return (status, bufs);
                }
                status = h.call_m(0x195cd0, &[dst3, str6b], m);
                if status >= 0x80000000 {
                    return (status, bufs);
                }
                let mut ph = dst1;
                let mut pos5 = dst2;
                let mut pos6 = dst3;
                let mut sp3 = h.call_m(0x195d30, &[ph, 0x20], m);
                let mut sp5 = h.call_m(0x195d30, &[pos5, 0x20], m);
                let mut sp6 = h.call_m(0x195d30, &[pos6, 0x20], m);
                let mut pos = start;
                while sp3 != 0 {
                    loop {
                        let ch = g_byte(m, text_copy.wrapping_add(u32::from(pos))).unwrap_or(0);
                        if ch == 0 || ch == b' ' || ch == b'\t' || ch == b'\n' || ch == b'\r' || ch == b'_' {
                            break;
                        }
                        pos = pos.wrapping_add(1);
                    }
                    if !p_half(m, at(L_END), pos) {
                        return (FAIL_MEM, bufs);
                    }
                    status = h.call_m(
                        f_set_field,
                        &[a, c, u32::from(tok), 2, 1, at(L_END), at(L_X)],
                        m,
                    );
                    if status >= 0x80000000 {
                        return (status, bufs);
                    }
                    pos = pos.wrapping_add(1);
                    if !p_byte(m, sp3, 0) {
                        return (FAIL_MEM, bufs);
                    }
                    let phlen = h.call_m(0x195d90, &[ph], m);
                    status = h.call_m(
                        f_set_field,
                        &[a, c, u32::from(tok), 3, phlen.wrapping_add(1), ph, at(L_X)],
                        m,
                    );
                    if status >= 0x80000000 {
                        return (status, bufs);
                    }
                    ph = sp3.wrapping_add(1);
                    if sp5 != 0 && !p_byte(m, sp5, 0) {
                        return (FAIL_MEM, bufs);
                    }
                    let p5len = h.call_m(0x195d90, &[pos5], m);
                    status = h.call_m(
                        f_set_field,
                        &[a, c, u32::from(tok), 5, p5len.wrapping_add(1), pos5, at(L_X)],
                        m,
                    );
                    if status >= 0x80000000 {
                        return (status, bufs);
                    }
                    if sp5 != 0 {
                        pos5 = sp5.wrapping_add(1);
                    }
                    // *sp6 = 0 unconditionally: sp6 is never null here.
                    if !p_byte(m, sp6, 0) {
                        return (FAIL_MEM, bufs);
                    }
                    let p6len = h.call_m(0x195d90, &[pos6], m);
                    status = h.call_m(
                        f_set_field,
                        &[a, c, u32::from(tok), 6, p6len.wrapping_add(1), pos6, at(L_X)],
                        m,
                    );
                    if status >= 0x80000000 {
                        return (status, bufs);
                    }
                    let mut wend = 0u16;
                    pos6 = sp6.wrapping_add(1);
                    if !p_half(m, at(L_WEND), wend) {
                        return (FAIL_MEM, bufs);
                    }
                    status = h.call_m(
                        f_set_field,
                        &[a, c, u32::from(tok), 8, 1, at(L_WEND), at(L_X)],
                        m,
                    );
                    if status >= 0x80000000 {
                        return (status, bufs);
                    }
                    let mut prev = tok;
                    while tok != 0 && (start as i16) < (pos as i16) {
                        prev = tok;
                        if !p_half(m, at(L_TOK), 0) {
                            return (FAIL_MEM, bufs);
                        }
                        status = h.call_m(f_get_next, &[a, c, u32::from(prev), at(L_TOK)], m);
                        if status >= 0x80000000 {
                            return (status, bufs);
                        }
                        tok = g_half(m, at(L_TOK)).unwrap_or(0);
                        if tok != 0 {
                            if !p_half(m, at(L_START), 0) {
                                return (FAIL_MEM, bufs);
                            }
                            status = h.call_m(
                                f_get_field,
                                &[a, c, u32::from(tok), 1, 1, at(L_START), at(L_N)],
                                m,
                            );
                            if status >= 0x80000000 {
                                return (status, bufs);
                            }
                            start = g_half(m, at(L_START)).unwrap_or(0);
                        }
                    }
                    if !p_half(m, at(L_W), 0) {
                        return (FAIL_MEM, bufs);
                    }
                    status = h.call_m(f_new_record, &[a, c, u32::from(prev), at(L_W)], m);
                    if status >= 0x80000000 {
                        return (status, bufs);
                    }
                    tok = g_half(m, at(L_W)).unwrap_or(0);
                    if !p_half(m, at(L_END), pos) {
                        return (FAIL_MEM, bufs);
                    }
                    status = h.call_m(
                        f_set_field,
                        &[a, c, u32::from(tok), 1, 1, at(L_END), at(L_X)],
                        m,
                    );
                    if status >= 0x80000000 {
                        return (status, bufs);
                    }
                    if !p_byte(m, at(L_WTYPE), wtype) {
                        return (FAIL_MEM, bufs);
                    }
                    status = h.call_m(
                        f_set_field,
                        &[a, c, u32::from(tok), 0, 1, at(L_WTYPE), at(L_X)],
                        m,
                    );
                    if status >= 0x80000000 {
                        return (status, bufs);
                    }
                    if !p_half(m, at(L_F7), f7) {
                        return (FAIL_MEM, bufs);
                    }
                    status = h.call_m(
                        f_set_field,
                        &[a, c, u32::from(tok), 7, 1, at(L_F7), at(L_X)],
                        m,
                    );
                    if status >= 0x80000000 {
                        return (status, bufs);
                    }
                    let langlen = h.call_m(0x195d90, &[lang], m);
                    status = h.call_m(
                        f_set_field,
                        &[a, c, u32::from(tok), 9, langlen.wrapping_add(1), lang, at(L_X)],
                        m,
                    );
                    if status >= 0x80000000 {
                        return (status, bufs);
                    }
                    sp3 = h.call_m(0x195d30, &[ph, 0x20], m);
                    sp5 = h.call_m(0x195d30, &[pos5, 0x20], m);
                    sp6 = h.call_m(0x195d30, &[pos6, 0x20], m);
                    let _ = wend;
                }
                if !p_half(m, at(L_END), end) {
                    return (FAIL_MEM, bufs);
                }
                status = h.call_m(
                    f_set_field,
                    &[a, c, u32::from(tok), 2, 1, at(L_END), at(L_X)],
                    m,
                );
                if status >= 0x80000000 {
                    return (status, bufs);
                }
                let phlen = h.call_m(0x195d90, &[ph], m);
                status = h.call_m(
                    f_set_field,
                    &[a, c, u32::from(tok), 3, phlen.wrapping_add(1), ph, at(L_X)],
                    m,
                );
                if status >= 0x80000000 {
                    return (status, bufs);
                }
                let p5len = h.call_m(0x195d90, &[pos5], m);
                status = h.call_m(
                    f_set_field,
                    &[a, c, u32::from(tok), 5, p5len.wrapping_add(1), pos5, at(L_X)],
                    m,
                );
                if status >= 0x80000000 {
                    return (status, bufs);
                }
                let p6len = h.call_m(0x195d90, &[pos6], m);
                status = h.call_m(
                    f_set_field,
                    &[a, c, u32::from(tok), 6, p6len.wrapping_add(1), pos6, at(L_X)],
                    m,
                );
                if status >= 0x80000000 {
                    return (status, bufs);
                }
                if !p_half(m, at(L_WEND), wstart) {
                    return (FAIL_MEM, bufs);
                }
                status = h.call_m(
                    f_set_field,
                    &[a, c, u32::from(tok), 8, 1, at(L_WEND), at(L_X)],
                    m,
                );
                if status >= 0x80000000 {
                    return (status, bufs);
                }
            }
        }
        // nextWord: advance to the next word record.
        if !p_half(m, at(L_TOK), 0) {
            return (FAIL_MEM, bufs);
        }
        status = h.call_m(f_get_next, &[a, c, u32::from(tok), at(L_TOK)], m);
        if status >= 0x80000000 {
            return (status, bufs);
        }
        tok = g_half(m, at(L_TOK)).unwrap_or(0);
    }
    if same_start != 0 && keep_alt != 0 {
        let mut k = 0u8;
        while (k as u16) < n_del {
            let id = g_half(m, at(L_DELIDS).wrapping_add((k as u32) * 2)).unwrap_or(0);
            status = h.call_m(f_remove, &[a, c, u32::from(id)], m);
            if status >= 0x80000000 {
                return (status, bufs);
            }
            k = k.wrapping_add(1);
        }
    }
    // Build the alternative transcriptions (dpsMultiTrans != 0).
    // Unreached on the certified capture; the host calls mirror the C.
    if dps_multi != 0 {
        if !f198c8_puts(m, at(0x33c), b"multi_trans_")
            || !f198c8_puts(m, at(0x34a), b"%s L1: %s")
            || !f198c8_puts(m, at(0x354), b"%s L2: %s")
            || !f198c8_puts(m, at(0x35e), b"%s L3: %s")
            || !f198c8_puts(m, at(0x368), b"%s O1: %s")
        {
            return (FAIL_MEM, bufs);
        }
        if !p_half(m, at(L_TOK), 0) {
            return (FAIL_MEM, bufs);
        }
        status = h.call_m(f_get_first, &[a, c, 2, u32::from(root), at(L_TOK)], m);
        if status >= 0x80000000 {
            return (status, bufs);
        }
        tok = g_half(m, at(L_TOK)).unwrap_or(0);
        while tok != 0 {
            if !p_byte(m, at(L_WTYPE), 0) {
                return (FAIL_MEM, bufs);
            }
            status = h.call_m(f_get_field, &[a, c, u32::from(tok), 0, 1, at(L_WTYPE), at(L_N)], m);
            if status >= 0x80000000 {
                return (status, bufs);
            }
            let wtype = g_byte(m, at(L_WTYPE)).unwrap_or(0);
            if wtype == 9 {
                if !p_half(m, at(L_HAS), 0) {
                    return (FAIL_MEM, bufs);
                }
                status = h.call_m(f_has_field, &[a, c, u32::from(tok), 11, at(L_HAS)], m);
                if status >= 0x80000000 {
                    status = 0;
                    if !p_half(m, at(L_HAS), 0) {
                        return (FAIL_MEM, bufs);
                    }
                } else if g_half(m, at(L_HAS)).unwrap_or(0) != 0 {
                    if !p_half(m, at(L_TOK), 0) {
                        return (FAIL_MEM, bufs);
                    }
                    status = h.call_m(f_get_next, &[a, c, u32::from(tok), at(L_TOK)], m);
                    if status >= 0x80000000 {
                        return (status, bufs);
                    }
                    tok = g_half(m, at(L_TOK)).unwrap_or(0);
                    continue;
                }
                if !p_half(m, at(L_START), 0) || !p_half(m, at(L_END), 0) {
                    return (FAIL_MEM, bufs);
                }
                status = h.call_m(f_get_field, &[a, c, u32::from(tok), 1, 1, at(L_START), at(L_N)], m);
                if status >= 0x80000000 {
                    return (status, bufs);
                }
                status = h.call_m(f_get_field, &[a, c, u32::from(tok), 2, 1, at(L_END), at(L_N)], m);
                if status >= 0x80000000 {
                    return (status, bufs);
                }
                let start = g_half(m, at(L_START)).unwrap_or(0);
                let end = g_half(m, at(L_END)).unwrap_or(0);
                if !p_word(m, at(L_TMP), 0) {
                    return (FAIL_MEM, bufs);
                }
                status = h.call_m(0x195cd0, &[at(L_TMP), at(0x33c)], m);
                if status >= 0x80000000 {
                    return (status, bufs);
                }
                let _ = h.call_m(0x195d90, &[at(L_TMP)], m);
                if end.wrapping_sub(start).wrapping_add(1) > 128 {
                    let cur = g_word(m, p.wrapping_add(0x40)).unwrap_or(0);
                    let q = h.call_m(0x1963e0, &[heap, cur, u32::from(end.wrapping_sub(start).wrapping_add(1))], m);
                    if q == 0 {
                        h.call_m(0x173dc0, &[log, D_8f9532, 0x84d0, 0], m);
                        return (0x89b0200a, bufs);
                    }
                    if !p_word(m, p.wrapping_add(0x40), q) {
                        return (FAIL_MEM, bufs);
                    }
                }
                let b400 = g_word(m, p.wrapping_add(0x40)).unwrap_or(0);
                // Round 4 (seg1): the bounded copy runs in-component
                // (0x195e38 has no live Host arm).
                if !f195e38_copy(m, b400, text_copy.wrapping_add(u32::from(start)), u32::from(end.wrapping_sub(start))) {
                    return (FAIL_MEM, bufs);
                }
                status = 0;
                if !p_byte(m, b400.wrapping_add(u32::from(end.wrapping_sub(start))), 0) {
                    return (FAIL_MEM, bufs);
                }
                let f_add_word = g_word(m, cfg_vt.wrapping_add(CFG_ADD_WORD * 4)).unwrap_or(0);
                status = h.call_m(f_add_word, &[h_a, h_b, str4_last, b400], m);
                if status >= 0x80000000 {
                    return (status, bufs);
                }
                if !p_word(m, at(L_STR3), 0) || !p_word(m, at(L_STR4), 0) {
                    return (FAIL_MEM, bufs);
                }
                status = h.call_m(
                    f_get_field_ptr,
                    &[a, c, u32::from(tok), 3, at(L_STR3), at(L_N)],
                    m,
                );
                if status >= 0x80000000 {
                    return (status, bufs);
                }
                status = h.call_m(
                    f_get_field_ptr,
                    &[a, c, u32::from(tok), 9, at(L_STR4), at(L_N)],
                    m,
                );
                if status >= 0x80000000 {
                    return (status, bufs);
                }
                let str3 = g_word(m, at(L_STR3)).unwrap_or(0);
                let lang = g_word(m, at(L_STR4)).unwrap_or(0);
                let b401 = g_word(m, p.wrapping_add(0x44)).unwrap_or(0);
                if !p_byte(m, b401, 0) {
                    return (FAIL_MEM, bufs);
                }
                let f_tr_set = g_word(m, trans_vt.wrapping_add(TRANS_SET_STRING * 4)).unwrap_or(0);
                let f_tr_get = g_word(m, trans_vt.wrapping_add(TRANS_GET_STRING * 4)).unwrap_or(0);
                let f_tr_sel = g_word(m, trans_vt.wrapping_add(TRANS_SELECT * 4)).unwrap_or(0);
                let mut k = 0u8;
                let mut run_total = 3u16;
                while k < dps_multi {
                    status = h.call_m(0x176ac8, &[u32::from(k).wrapping_add(1), at(L_NUMBUF), 10], m);
                    if status >= 0x80000000 {
                        return (status, bufs);
                    }
                    status = h.call_m(0x195c90, &[at(L_TMP), at(L_NUMBUF)], m);
                    if status >= 0x80000000 {
                        return (status, bufs);
                    }
                    let s3len = h.call_m(0x195d90, &[str3], m);
                    status = h.call_m(f_tr_set, &[h2_a, h2_b, 0, str3, s3len], m);
                    if status >= 0x80000000 {
                        return (status, bufs);
                    }
                    if !p_word(m, at(0x260), 0) || !p_half(m, at(0x264), 0) {
                        return (FAIL_MEM, bufs);
                    }
                    status = h.call_m(f_tr_get, &[h2_a, h2_b, 0, at(0x260), at(0x264)], m);
                    if status >= 0x80000000 {
                        return (status, bufs);
                    }
                    let out0 = g_word(m, at(0x260)).unwrap_or(0);
                    let mut tlen = g_half(m, at(0x264)).unwrap_or(0);
                    if !p_byte(m, out0.wrapping_add(u32::from(tlen)), 0) {
                        return (FAIL_MEM, bufs);
                    }
                    let mut i = 0u16;
                    while i < tlen {
                        if g_byte(m, out0.wrapping_add(u32::from(i))).unwrap_or(0) == b'~' {
                            if !p_byte(m, out0.wrapping_add(u32::from(i)), 0xad) {
                                return (FAIL_MEM, bufs);
                            }
                        }
                        i = i.wrapping_add(1);
                    }
                    if g_byte(m, lang).unwrap_or(0) != 0 {
                        status = h.call_m(f_tr_get, &[h2_a, h2_b, 2, at(0x260), at(0x264)], m);
                        if status >= 0x80000000 {
                            return (status, bufs);
                        }
                        let out2 = g_word(m, at(0x260)).unwrap_or(0);
                        tlen = g_half(m, at(0x264)).unwrap_or(0);
                        if !p_byte(m, out2.wrapping_add(u32::from(tlen)), 0) {
                            return (FAIL_MEM, bufs);
                        }
                        if tlen != 0 {
                            let fc = g_byte(m, lang).unwrap_or(0);
                            let mut j = 0u16;
                            while j < tlen {
                                if !p_byte(m, out2.wrapping_add(u32::from(j)), fc) {
                                    return (FAIL_MEM, bufs);
                                }
                                j = j.wrapping_add(1);
                            }
                        }
                        status = h.call_m(0x1738e8, &[log, D_8f9532, 5, 0, at(0x35e), at(L_TMP), out0], m);
                        if status >= 0x80000000 {
                            return (status, bufs);
                        }
                    }
                    status = h.call_m(0x1738e8, &[log, D_8f9532, 5, 0, at(0x34a), at(L_TMP), out0], m);
                    if status >= 0x80000000 {
                        return (status, bufs);
                    }
                    status = h.call_m(0x1738e8, &[log, D_8f9532, 5, 0, at(0x354), at(L_TMP), b400], m);
                    if status >= 0x80000000 {
                        return (status, bufs);
                    }
                    let b40len = h.call_m(0x195d90, &[b400], m);
                    status = h.call_m(f_tr_set, &[h2_a, h2_b, 1, b400, b40len], m);
                    if status >= 0x80000000 {
                        return (status, bufs);
                    }
                    status = h.call_m(f_tr_sel, &[h2_a, h2_b, at(L_TMP)], m);
                    if status >= 0x80000000 {
                        return (status, bufs);
                    }
                    status = h.call_m(f_tr_get, &[h2_a, h2_b, 0, at(0x260), at(0x264)], m);
                    if status >= 0x80000000 {
                        return (status, bufs);
                    }
                    let out0b = g_word(m, at(0x260)).unwrap_or(0);
                    tlen = g_half(m, at(0x264)).unwrap_or(0);
                    if !p_byte(m, out0b.wrapping_add(u32::from(tlen)), 0) {
                        return (FAIL_MEM, bufs);
                    }
                    status = h.call_m(0x1738e8, &[log, D_8f9532, 5, 0, at(0x368), at(L_TMP), out0b], m);
                    if status >= 0x80000000 {
                        return (status, bufs);
                    }
                    let mut pos = tlen;
                    let mut ii = 0u16;
                    while ii < pos {
                        let ch = g_byte(m, out0b.wrapping_add(u32::from(ii))).unwrap_or(0);
                        if ch == b'~' {
                            let mut j = ii;
                            while j < pos {
                                let nx = g_byte(m, out0b.wrapping_add(u32::from(j).wrapping_add(1))).unwrap_or(0);
                                if !p_byte(m, out0b.wrapping_add(u32::from(j)), nx) {
                                    return (FAIL_MEM, bufs);
                                }
                                j = j.wrapping_add(1);
                            }
                            pos = pos.wrapping_sub(1);
                            ii = ii.wrapping_sub(1);
                        } else if ch == 0xad {
                            if !p_byte(m, out0b.wrapping_add(u32::from(ii)), b'~') {
                                return (FAIL_MEM, bufs);
                            }
                        }
                        ii = ii.wrapping_add(1);
                    }
                    if h.call_m(0x195d58, &[str3, out0b], m) != 0 {
                        let olen = h.call_m(0x195d90, &[out0b], m);
                        run_total = run_total.wrapping_add(olen as u16).wrapping_add(1);
                        if run_total > 128 {
                            let cur = g_word(m, p.wrapping_add(0x44)).unwrap_or(0);
                            let q = h.call_m(0x1963e0, &[heap, cur, u32::from(run_total)], m);
                            if q == 0 {
                                h.call_m(0x173dc0, &[log, D_8f9532, 0x84d0, 0], m);
                                return (0x89b0200a, bufs);
                            }
                            if !p_word(m, p.wrapping_add(0x44), q) {
                                return (FAIL_MEM, bufs);
                            }
                        }
                        let dst = g_word(m, p.wrapping_add(0x44)).unwrap_or(0);
                        status = h.call_m(0x195c90, &[dst, out0b], m);
                        if status >= 0x80000000 {
                            return (status, bufs);
                        }
                        // strcat(buf40[1], "\x12"): the 0x12 byte via scratch.
                        if !p_byte(m, at(0x268), 0x12) || !p_byte(m, at(0x269), 0) {
                            return (FAIL_MEM, bufs);
                        }
                        status = h.call_m(0x195c90, &[dst, at(0x268)], m);
                        if status >= 0x80000000 {
                            return (status, bufs);
                        }
                    }
                    k = k.wrapping_add(1);
                }
                let dst = g_word(m, p.wrapping_add(0x44)).unwrap_or(0);
                if g_byte(m, dst).unwrap_or(0) != 0 {
                    let blen = h.call_m(0x195d90, &[dst], m);
                    if !p_byte(m, dst.wrapping_add(blen.wrapping_sub(1)), 0) {
                        return (FAIL_MEM, bufs);
                    }
                    status = h.call_m(
                        f_set_field,
                        &[a, c, u32::from(tok), 11, blen.wrapping_add(1), dst, at(L_X)],
                        m,
                    );
                    if status >= 0x80000000 {
                        return (status, bufs);
                    }
                }
            }
            if !p_half(m, at(L_TOK), 0) {
                return (FAIL_MEM, bufs);
            }
            status = h.call_m(f_get_next, &[a, c, u32::from(tok), at(L_TOK)], m);
            if status >= 0x80000000 {
                return (status, bufs);
            }
            tok = g_half(m, at(L_TOK)).unwrap_or(0);
        }
    }
    // Language identification of foreign words; the certified capture
    // leaves through the `noExtClc` gate, as does the CLM section.
    let no_ext_clc = g_byte(m, p.wrapping_add(FE_NO_EXT_CLC)).unwrap_or(0);
    if no_ext_clc == 0 {
        return (status, bufs);
    }
    let clc_extra = g_byte(m, p.wrapping_add(FE_CLC_EXTRA_MAIN_TRANS)).unwrap_or(0);
    if clc_extra != 0 && n_out_trans > 1 {
        let mut prev_start = 0u16;
        let mut prev_end = 0u16;
        let mut foreign = 0u8;
        let mut prev_tok = 0u16;
        if !p_half(m, at(L_TOK), 0) {
            return (FAIL_MEM, bufs);
        }
        status = h.call_m(f_get_first, &[a, c, 2, u32::from(root), at(L_TOK)], m);
        if status >= 0x80000000 {
            return (status, bufs);
        }
        tok = g_half(m, at(L_TOK)).unwrap_or(0);
        if tok != 0 {
            loop {
                if !p_byte(m, at(L_WTYPE), 0) {
                    return (FAIL_MEM, bufs);
                }
                status =
                    h.call_m(f_get_field, &[a, c, u32::from(tok), 0, 1, at(L_WTYPE), at(L_N)], m);
                if status >= 0x80000000 {
                    return (status, bufs);
                }
                let wtype = g_byte(m, at(L_WTYPE)).unwrap_or(0);
                if wtype == 9 || wtype == 4 {
                    if !p_half(m, at(L_START), 0) || !p_half(m, at(L_END), 0) {
                        return (FAIL_MEM, bufs);
                    }
                    status = h.call_m(
                        f_get_field,
                        &[a, c, u32::from(tok), 1, 1, at(L_START), at(L_N)],
                        m,
                    );
                    if status >= 0x80000000 {
                        return (status, bufs);
                    }
                    status = h.call_m(
                        f_get_field,
                        &[a, c, u32::from(tok), 2, 1, at(L_END), at(L_N)],
                        m,
                    );
                    if status >= 0x80000000 {
                        return (status, bufs);
                    }
                    let start = g_half(m, at(L_START)).unwrap_or(0);
                    let end = g_half(m, at(L_END)).unwrap_or(0);
                    if !p_word(m, at(L_STR4), 0) {
                        return (FAIL_MEM, bufs);
                    }
                    status = h.call_m(
                        f_get_field_ptr,
                        &[a, c, u32::from(tok), 9, at(L_STR4), at(L_N)],
                        m,
                    );
                    if status >= 0x80000000 {
                        return (status, bufs);
                    }
                    let lang = g_word(m, at(L_STR4)).unwrap_or(0);
                    let lang_code = g_word(m, p.wrapping_add(FE_LANG_CODE)).unwrap_or(0);
                    if start != prev_start {
                        if foreign == 0 {
                            status = h.call_m(
                                0x1f932c,
                                &[p, a, c, u32::from(prev_tok), u32::from(prev_start), u32::from(prev_end), text_copy],
                                m,
                            );
                            if status >= 0x80000000 {
                                return (status, bufs);
                            }
                        }
                        foreign = 0;
                    }
                    if h.call_m(0x1769d0, &[lang, lang_code], m) == 0
                        || h.call_m(0x195d58, &[lang, EMPTY_S], m) == 0
                        || h.call_m(0x195d58, &[lang, S_NORMAL], m) == 0
                    {
                        foreign = 1;
                    }
                    prev_end = end;
                    prev_start = start;
                }
                prev_tok = tok;
                if !p_half(m, at(L_TOK), 0) {
                    return (FAIL_MEM, bufs);
                }
                status = h.call_m(f_get_next, &[a, c, u32::from(prev_tok), at(L_TOK)], m);
                if status >= 0x80000000 {
                    return (status, bufs);
                }
                tok = g_half(m, at(L_TOK)).unwrap_or(0);
                if tok == 0 {
                    break;
                }
            }
            if foreign == 0 {
                // lidDone with foreign == 0: fall through to the final call.
            }
        }
        if foreign == 0 {
            status = h.call_m(
                0x1f932c,
                &[p, a, c, u32::from(prev_tok), u32::from(prev_start), u32::from(prev_end), text_copy],
                m,
            );
            if status >= 0x80000000 {
                return (status, bufs);
            }
        }
        // lidDone: re-check the gate.
        let no_ext_clc = g_byte(m, p.wrapping_add(FE_NO_EXT_CLC)).unwrap_or(0);
        if no_ext_clc == 0 {
            return (status, bufs);
        }
    }
    // Map foreign phonemes through the cross-language mapping.
    if !f198c8_puts(m, at(0x2d4), b"Phon before %s CLM %s")
        || !f198c8_puts(m, at(0x2ec), b"Phon after %s CLM %s")
        || !f198c8_puts(m, at(0x302), b"CLM failed for %s")
        || !f198c8_puts(m, at(0x2d0), b"map")
    {
        return (FAIL_MEM, bufs);
    }
    let clm_map_vt = g_word(m, p.wrapping_add(FE_CLM_MAP)).unwrap_or(0);
    let f_clm_map = g_word(m, clm_map_vt.wrapping_add(0x24)).unwrap_or(0);
    let h4_a = g_word(m, p.wrapping_add(FE_CLM_HANDLE)).unwrap_or(0);
    let h4_b = g_word(m, p.wrapping_add(FE_CLM_HANDLE.wrapping_add(4))).unwrap_or(0);
    if !p_half(m, at(L_TOK), 0) {
        return (FAIL_MEM, bufs);
    }
    status = h.call_m(f_get_first, &[a, c, 2, u32::from(root), at(L_TOK)], m);
    if status >= 0x80000000 || g_half(m, at(L_TOK)).unwrap_or(0) == 0 {
        return (status, bufs);
    }
    tok = g_half(m, at(L_TOK)).unwrap_or(0);
    loop {
        if !p_byte(m, at(L_WTYPE), 0) {
            return (FAIL_MEM, bufs);
        }
        status =
            h.call_m(f_get_field, &[a, c, u32::from(tok), 0, 1, at(L_WTYPE), at(L_N)], m);
        if status >= 0x80000000 {
            return (status, bufs);
        }
        let wtype = g_byte(m, at(L_WTYPE)).unwrap_or(0);
        if wtype == 9 || wtype == 4 || wtype == 10 {
            if !p_word(m, at(L_STR3), 0) || !p_word(m, at(L_STR4), 0) {
                return (FAIL_MEM, bufs);
            }
            status = h.call_m(
                f_get_field_ptr,
                &[a, c, u32::from(tok), 3, at(L_STR3), at(L_N)],
                m,
            );
            if status >= 0x80000000 {
                return (status, bufs);
            }
            status = h.call_m(
                f_get_field_ptr,
                &[a, c, u32::from(tok), 9, at(L_STR4), at(L_N)],
                m,
            );
            if status >= 0x80000000 {
                return (status, bufs);
            }
            let str3 = g_word(m, at(L_STR3)).unwrap_or(0);
            let lang = g_word(m, at(L_STR4)).unwrap_or(0);
            let langlen = h.call_m(0x195d90, &[lang], m);
            status = h.call_m(
                f_set_field,
                &[a, c, u32::from(tok), 12, langlen.wrapping_add(1), lang, at(L_X)],
                m,
            );
            if status >= 0x80000000 {
                return (status, bufs);
            }
            if !p_word(m, at(L_TMP), 0) {
                return (FAIL_MEM, bufs);
            }
            status = h.call_m(0x195cd0, &[at(L_TMP), 0x1fd178], m);
            if status >= 0x80000000 {
                return (status, bufs);
            }
            status = h.call_m(0x195c90, &[at(L_TMP), lang], m);
            if status >= 0x80000000 {
                return (status, bufs);
            }
            if !p_word(m, at(L_CLM), 0) {
                return (FAIL_MEM, bufs);
            }
            status = h.call_m(0x175a1c, &[params, at(L_TMP), at(L_CLM)], m);
            let mut clm = 0u32;
            if status < 0x80000000 {
                clm = g_word(m, at(L_CLM)).unwrap_or(0);
                if clm == 0 {
                    // `else if (clm != 0) goto checkClm`: null falls through.
                }
            } else {
                clm = 0;
            }
            if clm == 0 {
                status = h.call_m(0x195cd0, &[at(L_TMP), 0x1fd17c], m);
                if status >= 0x80000000 {
                    return (status, bufs);
                }
                status = h.call_m(0x175a1c, &[params, at(L_TMP), at(L_CLM)], m);
                if status >= 0x80000000 || h.call_m(0x1960a8, &[g_word(m, at(L_CLM)).unwrap_or(0), lang], m) == 0 {
                    clm = 0;
                } else {
                    status = h.call_m(0x195cd0, &[at(L_NO4), S_NO], m);
                    if status >= 0x80000000 {
                        return (status, bufs);
                    }
                    clm = at(L_NO4);
                }
            }
            if clm != 0
                && (h.call_m(0x195d58, &[clm, 0x154ce8], m) != 0
                    && (h.call_m(0x195d58, &[clm, S_NO], m) != 0 || clm_variants == 0))
            {
                // nextClm: skip this word.
            } else if h.call_m(0x1769d0, &[lang, g_word(m, p.wrapping_add(FE_LANG_CODE)).unwrap_or(0)], m) == 0
                || h.call_m(0x195d58, &[lang, EMPTY_S], m) == 0
                || h.call_m(0x195d58, &[lang, S_NORMAL], m) == 0
            {
                // nextClm: domestic word.
            } else {
                let s3len = h.call_m(0x195d90, &[str3], m);
                let clm_size = s3len.wrapping_add(1).wrapping_mul(4);
                if !p_word(m, at(L_CLMSIZE), clm_size) {
                    return (FAIL_MEM, bufs);
                }
                let mut new_tok = tok;
                if !p_half(m, at(L_NEWT), tok) {
                    return (FAIL_MEM, bufs);
                }
                let mut code = [0u8; 4];
                let mut li = 0u32;
                while li < 3 {
                    let ch = g_byte(m, lang.wrapping_add(li)).unwrap_or(0);
                    code[li as usize] = if ch > 0x60 { ch.wrapping_sub(0x20) } else { ch };
                    li += 1;
                }
                code[3] = 0;
                if !p_byte(m, at(L_CODE), code[0])
                    || !p_byte(m, at(L_CODE).wrapping_add(1), code[1])
                    || !p_byte(m, at(L_CODE).wrapping_add(2), code[2])
                    || !p_byte(m, at(L_CODE).wrapping_add(3), 0)
                {
                    return (FAIL_MEM, bufs);
                }
                let grown = h.call_m(0x1963e0, &[heap, bufs[2], clm_size], m);
                if grown == 0 {
                    h.call_m(0x173dc0, &[log, D_8f9532, 0x84d0, 0], m);
                    return (0x89b0200a, bufs);
                }
                bufs[2] = grown;
                let clm_buf = grown;
                if !p_byte(m, clm_buf, 0) {
                    return (FAIL_MEM, bufs);
                }
                if clm != 0 && h.call_m(0x195d58, &[clm, S_NO], m) == 0 && clm_variants != 0 {
                    if !p_half(m, at(L_ONE), 1) {
                        return (FAIL_MEM, bufs);
                    }
                    status = h.call_m(f_new_record, &[a, c, u32::from(tok), at(L_NEWT)], m);
                    if status >= 0x80000000 {
                        return (status, bufs);
                    }
                    new_tok = g_half(m, at(L_NEWT)).unwrap_or(0);
                    if !p_half(m, at(L_START), 0) || !p_half(m, at(L_END), 0) {
                        return (FAIL_MEM, bufs);
                    }
                    status = h.call_m(f_get_field, &[a, c, u32::from(tok), 1, 1, at(L_START), at(L_N)], m);
                    if status >= 0x80000000 {
                        return (status, bufs);
                    }
                    status = h.call_m(f_get_field, &[a, c, u32::from(tok), 2, 1, at(L_END), at(L_N)], m);
                    if status >= 0x80000000 {
                        return (status, bufs);
                    }
                    if !p_byte(m, at(L_WTYPE), 15) {
                        return (FAIL_MEM, bufs);
                    }
                    status = h.call_m(f_set_field, &[a, c, u32::from(new_tok), 0, 1, at(L_WTYPE), at(L_X)], m);
                    if status >= 0x80000000 {
                        return (status, bufs);
                    }
                    status = h.call_m(f_set_field, &[a, c, u32::from(new_tok), 1, 1, at(L_START), at(L_X)], m);
                    if status >= 0x80000000 {
                        return (status, bufs);
                    }
                    status = h.call_m(f_set_field, &[a, c, u32::from(new_tok), 2, 1, at(L_END), at(L_X)], m);
                    if status >= 0x80000000 {
                        return (status, bufs);
                    }
                    status = h.call_m(f_set_field, &[a, c, u32::from(new_tok), 5, 1, 0x276778, at(L_X)], m);
                    if status >= 0x80000000 {
                        return (status, bufs);
                    }
                    status = h.call_m(f_set_field, &[a, c, u32::from(new_tok), 6, 1, 0x276778, at(L_X)], m);
                    if status >= 0x80000000 {
                        return (status, bufs);
                    }
                    status = h.call_m(f_set_field, &[a, c, u32::from(new_tok), 7, 1, at(L_ONE), at(L_X)], m);
                    if status >= 0x80000000 {
                        return (status, bufs);
                    }
                    status = h.call_m(f_set_field, &[a, c, u32::from(new_tok), 12, 4, at(0x2d0)], m);
                    if status >= 0x80000000 {
                        return (status, bufs);
                    }
                }
                status = h.call_m(
                    0x1738e8,
                    &[log, D_8f9532, 5, 0, at(0x2d4), at(L_CODE), str3],
                    m,
                );
                if status >= 0x80000000 {
                    return (status, bufs);
                }
                status = h.call_m(
                    f_clm_map,
                    &[h4_a, h4_b, at(L_CODE), str3, s3len.wrapping_add(1), clm_buf, clm_size],
                    m,
                );
                if status >= 0x80000000 {
                    status = h.call_m(0x1738e8, &[log, D_8f9532, 5, 0, at(0x302), str3], m);
                    if status >= 0x80000000 {
                        return (status, bufs);
                    }
                }
                status = h.call_m(
                    0x1738e8,
                    &[log, D_8f9532, 5, 0, at(0x2ec), at(L_CODE), clm_buf],
                    m,
                );
                if status >= 0x80000000 {
                    return (status, bufs);
                }
                let cblen = h.call_m(0x195d90, &[clm_buf], m);
                status = h.call_m(
                    f_set_field,
                    &[a, c, u32::from(new_tok), 3, cblen.wrapping_add(1), clm_buf, at(L_X)],
                    m,
                );
                if status >= 0x80000000 {
                    return (status, bufs);
                }
                status = h.call_m(f_set_field, &[a, c, u32::from(new_tok), 9, 1, EMPTY_S, at(L_X)], m);
                if status >= 0x80000000 {
                    return (status, bufs);
                }
                if !p_half(m, at(L_HAS), 0) {
                    return (FAIL_MEM, bufs);
                }
                status = h.call_m(f_has_field, &[a, c, u32::from(tok), 11, at(L_HAS)], m);
                if status >= 0x80000000 {
                    return (status, bufs);
                }
                if g_half(m, at(L_HAS)).unwrap_or(0) == 1 {
                    if !p_word(m, at(L_STR3), 0) {
                        return (FAIL_MEM, bufs);
                    }
                    status = h.call_m(
                        f_get_field_ptr,
                        &[a, c, u32::from(tok), 11, at(L_STR3), at(L_N)],
                        m,
                    );
                    if status >= 0x80000000 {
                        return (status, bufs);
                    }
                    let alt3 = g_word(m, at(L_STR3)).unwrap_or(0);
                    if h.call_m(0x195d58, &[alt3, EMPTY_S], m) != 0 {
                        let alen = h.call_m(0x195d90, &[alt3], m);
                        let grown =
                            h.call_m(0x1963e0, &[heap, bufs[3], alen.wrapping_add(1).wrapping_mul(4)], m);
                        if grown == 0 {
                            h.call_m(0x173dc0, &[log, D_8f9532, 0x84d0, 0], m);
                            return (0x89b0200a, bufs);
                        }
                        bufs[3] = grown;
                        let alt_buf = grown;
                        if !p_byte(m, alt_buf, 0) {
                            return (FAIL_MEM, bufs);
                        }
                        let mut s3 = alt3;
                        while s3 != 0 {
                            let sx = h.call_m(0x195d30, &[s3, 0x12], m);
                            if sx != 0 && !p_byte(m, sx, 0) {
                                return (FAIL_MEM, bufs);
                            }
                            if g_byte(m, alt_buf).unwrap_or(0) != 0 {
                                if !p_byte(m, at(0x268), 0x12) || !p_byte(m, at(0x269), 0) {
                                    return (FAIL_MEM, bufs);
                                }
                                status = h.call_m(0x195c90, &[alt_buf, at(0x268)], m);
                                if status >= 0x80000000 {
                                    return (status, bufs);
                                }
                            }
                            if !p_byte(m, clm_buf, b'_') || !p_byte(m, clm_buf.wrapping_add(1), 0) {
                                return (FAIL_MEM, bufs);
                            }
                            status = h.call_m(
                                0x1738e8,
                                &[log, D_8f9532, 5, 0, at(0x2d4), at(L_CODE), s3],
                                m,
                            );
                            if status >= 0x80000000 {
                                return (status, bufs);
                            }
                            let s3len = h.call_m(0x195d90, &[s3], m);
                            let need = s3len.wrapping_add(1).wrapping_mul(4);
                            let cur_size = g_word(m, at(L_CLMSIZE)).unwrap_or(0);
                            if cur_size < need {
                                if !p_word(m, at(L_CLMSIZE), need) {
                                    return (FAIL_MEM, bufs);
                                }
                                let grown = h.call_m(0x1963e0, &[heap, clm_buf, need], m);
                                if grown == 0 {
                                    h.call_m(0x173dc0, &[log, D_8f9532, 0x84d0, 0], m);
                                    return (0x89b0200a, bufs);
                                }
                                bufs[2] = grown;
                            }
                            let clm_buf2 = bufs[2];
                            status = h.call_m(
                                f_clm_map,
                                &[
                                    h4_a,
                                    h4_b,
                                    at(L_CODE),
                                    s3,
                                    s3len.wrapping_add(1),
                                    clm_buf2,
                                    g_word(m, at(L_CLMSIZE)).unwrap_or(0),
                                ],
                                m,
                            );
                            if status >= 0x80000000 {
                                status = h.call_m(0x1738e8, &[log, D_8f9532, 5, 0, at(0x302), s3], m);
                                if status >= 0x80000000 {
                                    return (status, bufs);
                                }
                            }
                            status = h.call_m(
                                0x1738e8,
                                &[log, D_8f9532, 5, 0, at(0x2ec), at(L_CODE), clm_buf2],
                                m,
                            );
                            if status >= 0x80000000 {
                                return (status, bufs);
                            }
                            status = h.call_m(0x195c90, &[alt_buf, clm_buf2], m);
                            if status >= 0x80000000 {
                                return (status, bufs);
                            }
                            if sx != 0 {
                                if !p_byte(m, sx, 0x12) {
                                    return (FAIL_MEM, bufs);
                                }
                                s3 = sx.wrapping_add(1);
                            } else {
                                s3 = 0;
                            }
                        }
                        let ablen = h.call_m(0x195d90, &[bufs[3]], m);
                        status = h.call_m(
                            f_set_field,
                            &[a, c, u32::from(new_tok), 11, ablen.wrapping_add(1), bufs[3], at(L_X)],
                            m,
                        );
                        if status >= 0x80000000 {
                            return (status, bufs);
                        }
                    }
                }
            }
        }
        if !p_half(m, at(L_TOK), 0) {
            return (FAIL_MEM, bufs);
        }
        status = h.call_m(f_get_next, &[a, c, u32::from(tok), at(L_TOK)], m);
        if status >= 0x80000000 {
            return (status, bufs);
        }
        tok = g_half(m, at(L_TOK)).unwrap_or(0);
        if tok == 0 {
            break;
        }
    }
    (status, bufs)
}
/// The acronym-detection frame of `fn_2719ec` at the ARM's own stack
/// position (round 4, seg1).
///
/// The ARM keeps `ruleName[128]`, `nodeIds[256]` and `languageName[12]` on
/// its stack: entry SP 0xe0ff170 (measured on speech_3 and three caps
/// inputs, stable because the scheduler call chain is input-independent),
/// ROM prologue pushes 13 words and subs 0x21c, so the body SP is
/// `FN_2719EC_FRAME` = 0xe0fef20. The port materialises the guest-visible
/// locals there with the layout below (0x1c0 bytes of the ARM's 0x250-byte
/// frame, clear of the caller's pushed args at 0xe0ff170 and of the live
/// 275928 frame above). `nodeIds` never reaches guest memory, so it stays
/// a Rust local. The frame sits in the excluded guest-stack range, like
/// every other segment scratch area. Out-param slots carry the
/// callee-written words (`insertChild`/`getFirst`/`next`/`up` node,
/// `1966ac` entry, rule `getOutput` buffers and length, attr sizes, and
/// the single callee-written bytes).
pub const FN_2719EC_FRAME: u32 = 0xe0fef20;
const F2719EC_RULE: u32 = 0x00;
const F2719EC_LANG: u32 = 0x80;
const F2719EC_LIT: u32 = 0x90;
const F2719EC_OUT: u32 = 0x190;
const F2719EC_LIT_ACRONYM: u32 = 0x00;
const F2719EC_LIT_L1: u32 = 0x13;
const F2719EC_LIT_O1: u32 = 0x1d;
const F2719EC_LIT_ONEWORD: u32 = 0x27;
const F2719EC_LIT_L3: u32 = 0x30;
const F2719EC_LIT_L4: u32 = 0x3a;
const F2719EC_LIT_O3: u32 = 0x44;
const F2719EC_LIT_O5: u32 = 0x4e;
const F2719EC_LIT_NORMAL: u32 = 0x58;
const F2719EC_LIT_SPELL: u32 = 0x5f;
const F2719EC_LIT_USCORE: u32 = 0x6d;
const F2719EC_LIT_CROSS: u32 = 0x6f;
const F2719EC_LIT_PROMPT: u32 = 0x7b;
const F2719EC_LIT_E: u32 = 0x82;
const F2719EC_LIT_ESCORE: u32 = 0x84;
const F2719EC_LIT_NORM2: u32 = 0x87;
const F2719EC_LIT_PLUS: u32 = 0x9a;
const F2719EC_LIT_M11: u32 = 0x9c;
const F2719EC_LIT_SP: u32 = 0x9e;
const F2719EC_LIT_TILDE: u32 = 0xa0;
const F2719EC_LIT_NOAFF: u32 = 0xa2;
const F2719EC_LIT_AFFFAIL: u32 = 0xd1;
const F2719EC_LIT_L2: u32 = 0xe6;
const F2719EC_LIT_O2: u32 = 0xf0;
const F2719EC_OUT_NODE: u32 = 0x00;
const F2719EC_OUT_ENTRY: u32 = 0x04;
const F2719EC_OUT_TEXT: u32 = 0x08;
const F2719EC_OUT_TAGS: u32 = 0x0c;
const F2719EC_OUT_PHON: u32 = 0x10;
const F2719EC_OUT_ORTHO: u32 = 0x14;
const F2719EC_OUT_LEN: u32 = 0x18;
const F2719EC_OUT_ATTR: u32 = 0x1c;
const F2719EC_OUT_WSIZE: u32 = 0x20;
const F2719EC_OUT_B: u32 = 0x24;
const F2719EC_OUT_SEP: u32 = 0x26;
const F2719EC_OUT_TYPE: u32 = 0x1a;
const F2719EC_OUT_FBUF: u32 = 0x1b;

/// Write one NUL-terminated scratch literal for `fn_2719ec`.
fn f2719ec_wlit(m: &mut Memory<'_>, dst: u32, s: &[u8]) -> bool {
    for (i, &b) in s.iter().enumerate() {
        if !p_byte(m, dst.wrapping_add(i as u32), b) {
            return false;
        }
    }
    true
}

/// The `fn_195e38` copy (strncpy): at most `n` bytes from `src` to
/// `dst`, stopping after the terminator and zero-filling the rest.
/// False if the arena does not hold the range.
fn f195e38_copy(m: &mut Memory<'_>, dst: u32, src: u32, n: u32) -> bool {
    let mut d = dst;
    let mut s = src;
    let mut left = n;
    while left != 0 {
        let b = match g_byte(m, s) {
            Some(v) => v,
            None => return false,
        };
        if !p_byte(m, d, b) {
            return false;
        }
        d = d.wrapping_add(1);
        s = s.wrapping_add(1);
        if b == 0 {
            break;
        }
        left -= 1;
        if left == 0 {
            return true;
        }
    }
    if n == 0 {
        return true;
    }
    let mut fill = left.wrapping_sub(1);
    while fill != 0 {
        if !p_byte(m, d, 0) {
            return false;
        }
        d = d.wrapping_add(1);
        fill -= 1;
    }
    true
}

/// `fn_1edd3c`: the lexicon delimiter test (src/fn_1edd3c.c), issued
/// through the host. The ROM refreshes the handle (`fn_176f94` with key
/// `0xf37f`) and reports whether `c` occurs in the handle's delimiter
/// string at `+100`. Both callees are bound on the one-word path.
fn f1edd3c_is_delim(
    m: &mut Memory<'_>,
    h: &mut impl Host,
    obj: u32,
    data: u32,
    c: u32,
) -> u32 {
    let rc = h.call_m(0x176f94, &[obj, data, 0xf37f, 676], m);
    if rc >= 0x80000000 || c == 0 {
        return 0;
    }
    let delims = g_word(m, obj.wrapping_add(100)).unwrap_or(0);
    if h.call_m(0x195d30, &[delims, c], m) == 0 {
        0
    } else {
        1
    }
}

/// Forward the lane-b host to the lane-c port interface so the
/// dictionary canonicalizer can run inside the acronym path.
struct CAdapter<'a, H: Host + ?Sized> {
    inner: &'a mut H,
}

impl<H: Host + ?Sized> crate::component_live_c::Host for CAdapter<'_, H> {
    fn call_m(&mut self, entry: u32, args: &[u32], memory: &mut Memory<'_>) -> u32 {
        self.inner.call_m(entry, args, memory)
    }
}

/// `fn_271130` G2P single-character word frame: the ARM keeps its scratch
/// below the caller's frame; the live port uses the same window with the
/// layout below (guest-stack range, excluded from comparisons like the
/// 2719ec frame). Out-param slots carry the callee-written words (lexicon
/// `results`/`count`/`sepch`, rule `out`/`outlen`).
pub const FN_271130_FRAME: u32 = 0xe0fee50;
const F271130_RULES: u32 = 0x00;
const F271130_DOM: u32 = 0x80;
const F271130_TMP: u32 = 0x8c;
const F271130_RESULTS: u32 = 0x90;
const F271130_COUNT: u32 = 0x94;
const F271130_SEPCH: u32 = 0x98;
const F271130_OUT: u32 = 0x9c;
const F271130_OUTLEN: u32 = 0xa0;
const F271130_LIT: u32 = 0xa4;
const F271130_LIT_ONEWORD: u32 = 0x00;
const F271130_LIT_NORMAL: u32 = 0x09;
const F271130_LIT_USCORE: u32 = 0x10;
const F271130_LIT_STAR: u32 = 0x12;
const F271130_LIT_L1: u32 = 0x14;
const F271130_LIT_O1: u32 = 0x1e;

/// `fn_271130`: the single-character G2P word path (src/fn_26ff18.c), 1:1
/// over the caller's arena. `loc` is the 2719ec frame the nested 270b40
/// calls need; `scratch` below is this function's own window. Every lexicon
/// and rule call rides the host like any other nested entry.
pub fn fn_271130_m(
    m: &mut Memory<'_>,
    h: &mut impl Host,
    loc: u32,
    ctx: u32,
    word: u32,
    text: u32,
    outc: u32,
    outword: u32,
) -> u32 {
    fn is_word_sep(c: u8) -> bool {
        c == b' ' || c == b'\t' || c == b'\n' || c == b'\r' || c == b'_'
    }
    let frame = FN_271130_FRAME;
    let rules = frame.wrapping_add(F271130_RULES);
    let dom = frame.wrapping_add(F271130_DOM);
    let tmp = frame.wrapping_add(F271130_TMP);
    let results_at = frame.wrapping_add(F271130_RESULTS);
    let count_at = frame.wrapping_add(F271130_COUNT);
    let sepch_at = frame.wrapping_add(F271130_SEPCH);
    let out_at = frame.wrapping_add(F271130_OUT);
    let outlen_at = frame.wrapping_add(F271130_OUTLEN);
    let lit = frame.wrapping_add(F271130_LIT);
    let lit_oneword = lit.wrapping_add(F271130_LIT_ONEWORD);
    let lit_normal = lit.wrapping_add(F271130_LIT_NORMAL);
    let lit_uscore = lit.wrapping_add(F271130_LIT_USCORE);
    let lit_star = lit.wrapping_add(F271130_LIT_STAR);
    let lit_l1 = lit.wrapping_add(F271130_LIT_L1);
    let lit_o1 = lit.wrapping_add(F271130_LIT_O1);
    if !f2719ec_wlit(m, lit_oneword, b"one_word\x00")
        || !f2719ec_wlit(m, lit_normal, b"normal\x00")
        || !f2719ec_wlit(m, lit_uscore, b"_\x00")
        || !f2719ec_wlit(m, lit_star, b"*\x00")
        || !f2719ec_wlit(m, lit_l1, b"%s L1: %s\x00")
        || !f2719ec_wlit(m, lit_o1, b"%s O1: %s\x00")
    {
        return FAIL_MEM;
    }
    let env = g_word(m, ctx).unwrap_or(0);
    let heap = g_word(m, env.wrapping_add(4)).unwrap_or(0);
    let log = g_word(m, env.wrapping_add(16)).unwrap_or(0);
    let lexh_obj = g_word(m, ctx.wrapping_add(0x10)).unwrap_or(0);
    let lexh_data = g_word(m, ctx.wrapping_add(0x14)).unwrap_or(0);
    let lex_vt = g_word(m, ctx.wrapping_add(0x18)).unwrap_or(0);
    let ruleh_obj = g_word(m, ctx.wrapping_add(0x1c)).unwrap_or(0);
    let ruleh_data = g_word(m, ctx.wrapping_add(0x20)).unwrap_or(0);
    let rule_vt = g_word(m, ctx.wrapping_add(0x24)).unwrap_or(0);
    let key = g_word(m, ctx.wrapping_add(0x40)).unwrap_or(0);
    let lex_flag = g_byte(m, ctx.wrapping_add(0x172)).unwrap_or(0);
    let split_word = g_byte(m, ctx.wrapping_add(0x173)).unwrap_or(0);
    let check_lang = g_byte(m, ctx.wrapping_add(0x2a8)).unwrap_or(0);
    let lang_key = g_word(m, ctx.wrapping_add(0x2ac)).unwrap_or(0);
    let lang_list = g_word(m, ctx.wrapping_add(0x2b0)).unwrap_or(0);
    let use_lang_domain = g_byte(m, ctx.wrapping_add(0x2b4)).unwrap_or(0);
    let langs = ctx.wrapping_add(0x2b5);
    let f_lookup_domain = g_word(m, lex_vt.wrapping_add(7 * 4)).unwrap_or(0);
    let f_lookup = g_word(m, lex_vt.wrapping_add(18 * 4)).unwrap_or(0);
    let f_is_delim = g_word(m, lex_vt.wrapping_add(12 * 4)).unwrap_or(0);
    let f_set_input = g_word(m, rule_vt.wrapping_add(9 * 4)).unwrap_or(0);
    let f_get_output = g_word(m, rule_vt.wrapping_add(10 * 4)).unwrap_or(0);
    let f_set_rules = g_word(m, rule_vt.wrapping_add(11 * 4)).unwrap_or(0);

    let mut lastc = b' ';
    let mut altc = b' ';
    let mut ret = 0u32;
    let mut altword = 0u32;
    if !p_byte(m, rules, 0) {
        return FAIL_MEM;
    }
    h.call_m(0x195cd0, &[rules, lit_oneword], m);
    let rlen = h.call_m(0x195d90, &[rules], m) as u16;
    let text_len = h.call_m(0x195d90, &[text], m);
    let copy = h.call_m(0x196394, &[heap, text_len.wrapping_add(1)], m);
    if copy == 0 {
        h.call_m(0x173dc0, &[log, D_8f9532, 0x84d0, 0], m);
        return 0x89b0200a;
    }
    h.call_m(0x195cd0, &[copy, text], m);
    if split_word == 1 {
        let p = h.call_m(0x195d30, &[word, u32::from(b'_')], m);
        if p != 0 {
            altc = g_byte(m, p.wrapping_add(1)).unwrap_or(0);
            altword = p.wrapping_add(1);
        } else {
            altword = word;
            altc = g_byte(m, word).unwrap_or(0);
        }
    }
    let mut start: u16 = 0;
    let mut i: u16 = 0;
    while (u32::from(i)) <= h.call_m(0x195d90, &[copy], m) {
        let ci = g_byte(m, copy.wrapping_add(u32::from(i))).unwrap_or(0);
        // The C assigns c inside isWordSep, so c is stale when isDelim
        // fires; the branch is taken either way and c is not used below.
        let is_delim = h.call_m(f_is_delim, &[lexh_obj, lexh_data, u32::from(ci)], m) != 0;
        if is_delim || is_word_sep(ci) || ci == 0 {
            if i != start {
                lastc = b' ';
                h.call_m(0x195cd0, &[key, 0x2702b8], m);
                h.call_m(
                    0x195dac,
                    &[key, copy.wrapping_add(u32::from(start)), u32::from(i.wrapping_sub(start))],
                    m,
                );
                let mut best = 0u32;
                h.call_m(0x1738e8, &[log, D_8f9532, 5, 0, 0x2702c0, key], m);
                if !p_word(m, count_at, 0) || !p_byte(m, sepch_at, 0) {
                    h.call_m(0x196428, &[heap, copy], m);
                    return FAIL_MEM;
                }
                if use_lang_domain != 0 {
                    h.call_m(0x195cd0, &[dom, 0x2702d4], m);
                    let ki = g_byte(m, key).unwrap_or(0);
                    h.call_m(0x195c90, &[dom, langs.wrapping_add(u32::from(ki) * 3)], m);
                    ret = h.call_m(
                        f_lookup_domain,
                        &[lexh_obj, lexh_data, dom, key, results_at, count_at, sepch_at],
                        m,
                    );
                } else {
                    ret = h.call_m(
                        f_lookup,
                        &[lexh_obj, lexh_data, 0x2702d4, key, results_at, count_at, sepch_at, u32::from(lex_flag)],
                        m,
                    );
                }
                if ret >= 0x80000000 {
                    h.call_m(0x196428, &[heap, copy], m);
                    return ret;
                }
                let count = g_half(m, count_at).unwrap_or(0);
                let sepch = g_byte(m, sepch_at).unwrap_or(0);
                let results = g_word(m, results_at).unwrap_or(0);
                let mut bestscore: u16 = 0;
                let mut j: u16 = 0;
                while (j as u32) < u32::from(count) {
                    let mut domain = 0u32;
                    let mut scorestr = 0u32;
                    let entry = g_word(m, results.wrapping_add(u32::from(j) * 4)).unwrap_or(0);
                    let mut lang = 0u32;
                    let mut p = h.call_m(0x195d30, &[entry, u32::from(sepch)], m);
                    if p != 0 {
                        if !p_byte(m, p, 0) {
                            h.call_m(0x196428, &[heap, copy], m);
                            return FAIL_MEM;
                        }
                        let mut f_ = p.wrapping_add(1);
                        p = h.call_m(0x195d30, &[f_, u32::from(sepch)], m);
                        if p != 0 {
                            if !p_byte(m, p, 0) {
                                h.call_m(0x196428, &[heap, copy], m);
                                return FAIL_MEM;
                            }
                            domain = p.wrapping_add(1);
                            p = h.call_m(0x195d30, &[domain, u32::from(sepch)], m);
                            if p != 0 {
                                if !p_byte(m, p, 0) {
                                    h.call_m(0x196428, &[heap, copy], m);
                                    return FAIL_MEM;
                                }
                                f_ = p.wrapping_add(1);
                                p = h.call_m(0x195d30, &[f_, u32::from(sepch)], m);
                                if p != 0 {
                                    if !p_byte(m, p, 0) {
                                        h.call_m(0x196428, &[heap, copy], m);
                                        return FAIL_MEM;
                                    }
                                    f_ = p.wrapping_add(1);
                                    p = h.call_m(0x195d30, &[f_, u32::from(sepch)], m);
                                    if p != 0 {
                                        if !p_byte(m, p, 0) {
                                            h.call_m(0x196428, &[heap, copy], m);
                                            return FAIL_MEM;
                                        }
                                        scorestr = p.wrapping_add(1);
                                        p = h.call_m(0x195d30, &[scorestr, u32::from(sepch)], m);
                                        if p != 0 {
                                            if !p_byte(m, p, 0) {
                                                h.call_m(0x196428, &[heap, copy], m);
                                                return FAIL_MEM;
                                            }
                                            f_ = p.wrapping_add(1);
                                            p = h.call_m(0x195d30, &[f_, u32::from(sepch)], m);
                                            if p != 0 {
                                                if !p_byte(m, p, 0) {
                                                    h.call_m(0x196428, &[heap, copy], m);
                                                    return FAIL_MEM;
                                                }
                                                lang = p.wrapping_add(1);
                                                p = h.call_m(0x195d30, &[lang, u32::from(sepch)], m);
                                                if p != 0 && !p_byte(m, p, 0) {
                                                    h.call_m(0x196428, &[heap, copy], m);
                                                    return FAIL_MEM;
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                    if entry == 0 {
                        h.call_m(0x173dc0, &[log, D_8f9532, 0x84d1, 0], m);
                        h.call_m(0x196428, &[heap, copy], m);
                        return 0x89b02007;
                    }
                    h.call_m(0x1738e8, &[log, D_8f9532, 5, 0, 0x2702d8, entry], m);
                    if domain != 0
                        && h.call_m(0x195d58, &[domain, 0x2702b8], m) != 0
                        && h.call_m(0x1960a8, &[domain, word], m) == 0
                    {
                        h.call_m(0x1738e8, &[log, D_8f9532, 5, 0, 0x2705ec], m);
                    } else if check_lang != 0 && lang != 0 {
                        let mut skip = false;
                        let q = h.call_m(0x1960a8, &[word, lang], m);
                        let lang_len = h.call_m(0x195d90, &[lang], m);
                        let word_len = h.call_m(0x195d90, &[word], m);
                        if q == 0
                            || (lang_len != 0
                                && g_byte(m, q.wrapping_add(lang_len)).unwrap_or(0) != 0)
                        {
                            let q2 = h.call_m(0x1960a8, &[lang_list, word], m);
                            if h.call_m(0x1769d0, &[lang, lang_key], m) != 0
                                || q2 == 0
                                || g_byte(m, q2.wrapping_add(word_len)).unwrap_or(0) != b'|'
                            {
                                h.call_m(0x1738e8, &[log, D_8f9532, 5, 0, 0x27060c, lang], m);
                                skip = true;
                            }
                        }
                        if skip {
                            j = j.wrapping_add(1);
                            continue;
                        }
                        let score = if scorestr != 0 && h.call_m(0x195d58, &[scorestr, 0x2702b8], m) != 0 {
                            h.call_m(0x17c550, &[scorestr], m) as u16
                        } else {
                            1
                        };
                        if score > bestscore {
                            bestscore = score;
                            best = entry;
                            if lang != 0 {
                                lastc = g_byte(m, lang).unwrap_or(0);
                                if !f195e38_copy(m, outword, lang, 10) {
                                    h.call_m(0x196428, &[heap, copy], m);
                                    return FAIL_MEM;
                                }
                            }
                        }
                    } else {
                        let score = if scorestr != 0 && h.call_m(0x195d58, &[scorestr, 0x2702b8], m) != 0 {
                            h.call_m(0x17c550, &[scorestr], m) as u16
                        } else {
                            1
                        };
                        if score > bestscore {
                            bestscore = score;
                            best = entry;
                            if lang != 0 {
                                lastc = g_byte(m, lang).unwrap_or(0);
                                if !f195e38_copy(m, outword, lang, 10) {
                                    h.call_m(0x196428, &[heap, copy], m);
                                    return FAIL_MEM;
                                }
                            }
                        }
                    }
                    j = j.wrapping_add(1);
                }
                if best != 0 {
                    let p = h.call_m(0x195d30, &[best, 0x12], m);
                    if p != 0 && !p_byte(m, p, 0) {
                        h.call_m(0x196428, &[heap, copy], m);
                        return FAIL_MEM;
                    }
                    if h.call_m(0x195d58, &[best, lit_uscore], m) != 0 {
                        ret = fn_270b40_m(m, h, loc, ctx, best, lit_star, lit_star, u32::from(lastc), 1);
                        if ret >= 0x80000000 {
                            h.call_m(0x196428, &[heap, copy], m);
                            return ret;
                        }
                    }
                } else {
                    h.call_m(0x1738e8, &[log, D_8f9532, 5, 0, lit_l1, rules, key], m);
                    let key_len = h.call_m(0x195d90, &[key], m) as u16 as u32;
                    ret = h.call_m(f_set_input, &[ruleh_obj, ruleh_data, 0, key, key_len], m);
                    if ret >= 0x80000000 {
                        h.call_m(0x196428, &[heap, copy], m);
                        return ret;
                    }
                    if h.call_m(0x195d58, &[word, lit_normal], m) != 0
                        && h.call_m(0x195d90, &[word], m).wrapping_add(u32::from(rlen)).wrapping_add(2) < 128
                    {
                        h.call_m(0x195c90, &[rules, lit_uscore], m);
                        h.call_m(0x195c90, &[rules, word], m);
                    }
                    if h.call_m(f_set_rules, &[ruleh_obj, ruleh_data, rules], m) >= 0x80000000 {
                        if !p_byte(m, rules.wrapping_add(u32::from(rlen)), 0) {
                            h.call_m(0x196428, &[heap, copy], m);
                            return FAIL_MEM;
                        }
                        ret = h.call_m(f_set_rules, &[ruleh_obj, ruleh_data, rules], m);
                        if ret >= 0x80000000 {
                            h.call_m(0x196428, &[heap, copy], m);
                            return ret;
                        }
                    } else if split_word == 0 {
                        lastc = g_byte(m, word).unwrap_or(0);
                        if !f195e38_copy(m, outword, word, 10) {
                            h.call_m(0x196428, &[heap, copy], m);
                            return FAIL_MEM;
                        }
                    } else if altword != 0 {
                        lastc = altc;
                        if !f195e38_copy(m, outword, altword, 10) {
                            h.call_m(0x196428, &[heap, copy], m);
                            return FAIL_MEM;
                        }
                    }
                    ret = h.call_m(f_get_output, &[ruleh_obj, ruleh_data, 0, out_at, outlen_at], m);
                    if ret >= 0x80000000 {
                        h.call_m(0x196428, &[heap, copy], m);
                        return ret;
                    }
                    let out = g_word(m, out_at).unwrap_or(0);
                    let outlen = g_half(m, outlen_at).unwrap_or(0);
                    if !p_byte(m, out.wrapping_add(u32::from(outlen)), 0) {
                        h.call_m(0x196428, &[heap, copy], m);
                        return FAIL_MEM;
                    }
                    h.call_m(0x1738e8, &[log, D_8f9532, 5, 0, lit_o1, rules, out], m);
                    ret = fn_270b40_m(m, h, loc, ctx, out, lit_star, lit_star, u32::from(lastc), 1);
                    if ret >= 0x80000000 {
                        h.call_m(0x196428, &[heap, copy], m);
                        return ret;
                    }
                }
            }
            if !p_byte(m, tmp, ci) || !p_byte(m, tmp.wrapping_add(1), 0) {
                h.call_m(0x196428, &[heap, copy], m);
                return FAIL_MEM;
            }
            ret = fn_270b40_m(m, h, loc, ctx, tmp, 0x2702b8, 0x2702b8, u32::from(b' '), 0);
            if ret >= 0x80000000 {
                h.call_m(0x196428, &[heap, copy], m);
                return ret;
            }
            start = i.wrapping_add(1);
        }
        i = i.wrapping_add(1);
    }
    h.call_m(0x196428, &[heap, copy], m);
    if !p_byte(m, outc, lastc) {
        return FAIL_MEM;
    }
    ret
}

/// `fn_270b40`: the transliteration-fallback path of the one-word stage
/// (src/fn_26ff18.c), live over the caller's arena. `loc` is the
/// `FN_2719EC_FRAME` window: the 2-byte `sep` local lives there because
/// guest calls read it. Returns the ROM status (0 or 0x89b0200a).
#[allow(clippy::too_many_arguments)]
pub fn fn_270b40_m(
    m: &mut Memory<'_>,
    h: &mut impl Host,
    loc: u32,
    ctx: u32,
    word: u32,
    phon: u32,
    pos: u32,
    sepc: u32,
    tagged: u32,
) -> u32 {
    let lit = loc.wrapping_add(F2719EC_LIT);
    let lit_sp = lit.wrapping_add(F2719EC_LIT_SP);
    let lit_m11 = lit.wrapping_add(F2719EC_LIT_M11);
    let lit_tilde = lit.wrapping_add(F2719EC_LIT_TILDE);
    let sep = loc.wrapping_add(F2719EC_OUT).wrapping_add(F2719EC_OUT_SEP);
    let env = g_word(m, ctx).unwrap_or(0);
    let heap = g_word(m, env.wrapping_add(4)).unwrap_or(0);
    let log = g_word(m, env.wrapping_add(16)).unwrap_or(0);
    let mut orth = g_word(m, ctx.wrapping_add(0x58)).unwrap_or(0);
    let mut dst_phon = g_word(m, ctx.wrapping_add(0x5c)).unwrap_or(0);
    let mut dst_pos = g_word(m, ctx.wrapping_add(0x60)).unwrap_or(0);
    let mut dst_sep = g_word(m, ctx.wrapping_add(0x64)).unwrap_or(0);
    if !p_byte(m, sep, if sepc != 0 { sepc as u8 } else { b' ' }) {
        return FAIL_MEM;
    }
    if !p_byte(m, sep.wrapping_add(1), 0) {
        return FAIL_MEM;
    }
    if h.call_m(0x195d90, &[phon], m) > 1
        && (tagged != 0 || h.call_m(0x195d30, &[phon, 0x20], m) == 0)
    {
        if !p_byte(m, phon.wrapping_add(1), 0) {
            return FAIL_MEM;
        }
    }
    if h.call_m(0x195d90, &[pos], m) > 1
        && (tagged != 0 || h.call_m(0x195d30, &[pos, 0x20], m) == 0)
    {
        if !p_byte(m, pos.wrapping_add(1), 0) {
            return FAIL_MEM;
        }
    }
    if h.call_m(0x195d58, &[word, EMPTY_S], m) == 0 {
        return 0;
    }
    let mut extra = 0u16;
    if tagged != 0 {
        extra = 2;
    }
    let wlen = h.call_m(0x195d90, &[word], m) as i16;
    let len = (wlen as i32)
        .wrapping_add(h.call_m(0x195d90, &[orth], m) as i32)
        .wrapping_add(extra as i32)
        .wrapping_add(1) as u16;
    if len > 128 {
        let p = h.call_m(0x1963e0, &[heap, orth, u32::from(len)], m);
        if p == 0 {
            h.call_m(0x173dc0, &[log, D_8f9532, 0x84d0, 0], m);
            return 0x89b0200a;
        }
        orth = p;
        if !p_word(m, ctx.wrapping_add(0x58), p) {
            return FAIL_MEM;
        }
        let p = h.call_m(0x1963e0, &[heap, dst_phon, u32::from(len)], m);
        if p == 0 {
            h.call_m(0x173dc0, &[log, D_8f9532, 0x84d0, 0], m);
            return 0x89b0200a;
        }
        dst_phon = p;
        if !p_word(m, ctx.wrapping_add(0x5c), p) {
            return FAIL_MEM;
        }
        let p = h.call_m(0x1963e0, &[heap, dst_pos, u32::from(len)], m);
        if p == 0 {
            h.call_m(0x173dc0, &[log, D_8f9532, 0x84d0, 0], m);
            return 0x89b0200a;
        }
        dst_pos = p;
        if !p_word(m, ctx.wrapping_add(0x60), p) {
            return FAIL_MEM;
        }
        let p = h.call_m(0x1963e0, &[heap, dst_sep, u32::from(len)], m);
        if p == 0 {
            h.call_m(0x173dc0, &[log, D_8f9532, 0x84d0, 0], m);
            return 0x89b0200a;
        }
        dst_sep = p;
        if !p_word(m, ctx.wrapping_add(0x64), p) {
            return FAIL_MEM;
        }
    }
    if tagged != 0 {
        h.call_m(0x195c90, &[orth, lit_m11], m);
        h.call_m(0x195c90, &[dst_phon, lit_sp], m);
        h.call_m(0x195c90, &[dst_pos, lit_sp], m);
        h.call_m(0x195c90, &[dst_sep, lit_sp], m);
    }
    let open = h.call_m(0x195d30, &[word, 0x11], m);
    if h.call_m(0x195d30, &[phon, 0x20], m) == 0 && open != 0 {
        let close = h.call_m(0x195d30, &[open.wrapping_add(1), 0x11], m);
        if close == 0 {
            h.call_m(0x173dc0, &[log, D_8f9532, 0x84d1, 0], m);
            return 0x89b02007;
        }
        let n = open.wrapping_sub(word).wrapping_add(1) as i32;
        h.call_m(0x195dac, &[orth, word, u32::from(n as u16)], m);
        let mut i = 0u16;
        while (i as i32) < n {
            h.call_m(0x195c90, &[dst_phon, lit_sp], m);
            h.call_m(0x195c90, &[dst_pos, lit_sp], m);
            h.call_m(0x195c90, &[dst_sep, lit_sp], m);
            i = i.wrapping_add(1);
        }
        h.call_m(
            0x195dac,
            &[
                orth,
                open.wrapping_add(1),
                u32::from(close.wrapping_sub(open).wrapping_sub(1) as u16),
            ],
            m,
        );
        let span = close.wrapping_sub(open) as i32;
        h.call_m(0x195c90, &[dst_phon, phon], m);
        h.call_m(0x195c90, &[dst_pos, pos], m);
        h.call_m(0x195c90, &[dst_sep, sep], m);
        let mut i = 0u16;
        while span
            .wrapping_sub(h.call_m(0x195d90, &[phon], m) as i32)
            .wrapping_sub(1)
            > i as i16 as i32
        {
            h.call_m(0x195c90, &[dst_phon, lit_tilde], m);
            i = i.wrapping_add(1);
        }
        let mut i = 0u16;
        while span
            .wrapping_sub(h.call_m(0x195d90, &[pos], m) as i32)
            .wrapping_sub(1)
            > i as i16 as i32
        {
            h.call_m(0x195c90, &[dst_pos, lit_tilde], m);
            i = i.wrapping_add(1);
        }
        let mut i = 0u16;
        while span
            .wrapping_sub(h.call_m(0x195d90, &[sep], m) as i32)
            .wrapping_sub(1)
            > i as i16 as i32
        {
            h.call_m(0x195c90, &[dst_sep, sep], m);
            i = i.wrapping_add(1);
        }
        h.call_m(0x195c90, &[orth, close], m);
        let mut i = 0u16;
        while (h.call_m(0x195d90, &[word], m) as i16 as i32)
            .wrapping_sub(close.wrapping_sub(word) as i16 as i32)
            > i as i16 as i32
        {
            h.call_m(0x195c90, &[dst_phon, lit_sp], m);
            h.call_m(0x195c90, &[dst_pos, lit_sp], m);
            h.call_m(0x195c90, &[dst_sep, lit_sp], m);
            i = i.wrapping_add(1);
        }
    } else {
        h.call_m(0x195c90, &[orth, word], m);
        h.call_m(0x195c90, &[dst_phon, phon], m);
        h.call_m(0x195c90, &[dst_pos, pos], m);
        h.call_m(0x195c90, &[dst_sep, sep], m);
        let mut i = 0u16;
        while h.call_m(0x195d90, &[word], m)
            .wrapping_sub(h.call_m(0x195d90, &[phon], m))
            > u32::from(i)
        {
            h.call_m(0x195c90, &[dst_phon, lit_tilde], m);
            i = i.wrapping_add(1);
        }
        let mut i = 0u16;
        while h.call_m(0x195d90, &[word], m)
            .wrapping_sub(h.call_m(0x195d90, &[pos], m))
            > u32::from(i)
        {
            h.call_m(0x195c90, &[dst_pos, lit_tilde], m);
            i = i.wrapping_add(1);
        }
        let mut i = 0u16;
        while h.call_m(0x195d90, &[word], m)
            .wrapping_sub(h.call_m(0x195d90, &[sep], m))
            > u32::from(i)
        {
            h.call_m(0x195c90, &[dst_sep, sep], m);
            i = i.wrapping_add(1);
        }
    }
    if tagged != 0 {
        h.call_m(0x195c90, &[orth, lit_m11], m);
        h.call_m(0x195c90, &[dst_phon, lit_sp], m);
        h.call_m(0x195c90, &[dst_pos, lit_sp], m);
        h.call_m(0x195c90, &[dst_sep, lit_sp], m);
    }
    0
}

/// `fn_26fc54`: the multi-character word path (src/fn_26fc54.c), live over
/// the caller's arena. `ctx` is the segment context overlaid as the word
/// context (`word`/`phon`/`pos` at +0x40/+0x44/+0x48 are the segment
/// accumulators, the interface at +0x18 is the lexicon table). `loc` is
/// the `FN_2719EC_FRAME` window for the callee-written `found` byte.
#[allow(clippy::too_many_arguments)]
pub fn fn_26fc54_m(
    m: &mut Memory<'_>,
    h: &mut impl Host,
    loc: u32,
    ctx: u32,
    arg: u32,
    word: u32,
) -> u32 {
    let out = loc.wrapping_add(F2719EC_OUT);
    let lit = loc.wrapping_add(F2719EC_LIT);
    let lit_noaff = lit.wrapping_add(F2719EC_LIT_NOAFF);
    let lit_failed = lit.wrapping_add(F2719EC_LIT_AFFFAIL);
    let lit_sp = lit.wrapping_add(F2719EC_LIT_SP);
    let env = g_word(m, ctx).unwrap_or(0);
    let log = g_word(m, env.wrapping_add(16)).unwrap_or(0);
    let handle_obj = g_word(m, ctx.wrapping_add(0x10)).unwrap_or(0);
    let handle_data = g_word(m, ctx.wrapping_add(0x14)).unwrap_or(0);
    let _iface = g_word(m, ctx.wrapping_add(0x18)).unwrap_or(0);
    let wbuf = g_word(m, ctx.wrapping_add(0x40)).unwrap_or(0);
    let pbuf = g_word(m, ctx.wrapping_add(0x44)).unwrap_or(0);
    let posbuf = g_word(m, ctx.wrapping_add(0x48)).unwrap_or(0);
    let min_len = g_byte(m, ctx.wrapping_add(0x6a)).unwrap_or(0);
    let max_len = g_byte(m, ctx.wrapping_add(0x6b)).unwrap_or(0);
    let capitals = g_byte(m, ctx.wrapping_add(0x6c)).unwrap_or(0);
    let affix = g_byte(m, ctx.wrapping_add(0x6e)).unwrap_or(0);
    let wlen = h.call_m(0x195d90, &[word], m);
    let mut take_affix =
        affix == 1 && wlen >= u32::from(min_len) && wlen <= u32::from(max_len);
    if take_affix {
        let w0 = g_byte(m, word).unwrap_or(0);
        if capitals == 1 && (w0.wrapping_sub(b'A') as u32) > 25 {
            match w0 {
                0x80 | 0x8e | 0x8f | 0x90 | 0x92 | 0x99 | 0x9a | 0xa5 => {}
                _ => take_affix = false,
            }
        }
    }
    if !take_affix {
        h.call_m(0x1738e8, &[log, D_8f9532, 5, 0, lit_noaff, word], m);
        let mut adapter = CAdapter { inner: h };
        let rc = crate::component_live_c::dctlkp_canonical_token_m(
            m,
            handle_obj,
            handle_data,
            arg,
            word,
            &mut adapter,
        );
        if rc >= 0x80000000 {
            return rc;
        }
        let h = adapter.inner;
        h.call_m(0x195cd0, &[wbuf, word], m);
        h.call_m(0x195cd0, &[pbuf, EMPTY_S], m);
        h.call_m(0x195cd0, &[posbuf, EMPTY_S], m);
        let mut i = 0u16;
        loop {
            if i >= h.call_m(0x195d90, &[word], m) as u16 {
                break;
            }
            h.call_m(0x195c90, &[pbuf, lit_sp], m);
            h.call_m(0x195c90, &[posbuf, lit_sp], m);
            i = i.wrapping_add(1);
        }
        return rc;
    }
    let mut adapter = CAdapter { inner: h };
    let rc = crate::component_live_c::dctlkp_canonical_token_m(
        m,
        handle_obj,
        handle_data,
        arg,
        word,
        &mut adapter,
    );
    let h = adapter.inner;
    if rc >= 0x80000000 {
        return rc;
    }
    h.call_m(0x195cd0, &[wbuf, EMPTY_S], m);
    h.call_m(0x195cd0, &[pbuf, EMPTY_S], m);
    h.call_m(0x195cd0, &[posbuf, EMPTY_S], m);
    let found_at = out.wrapping_add(F2719EC_OUT_B);
    if !p_byte(m, found_at, 0) {
        return FAIL_MEM;
    }
    let rc = h.call_m(0x26e560, &[ctx, word, 0, found_at, 0, 0, arg], m);
    if rc >= 0x80000000 {
        return rc;
    }
    if g_byte(m, found_at).unwrap_or(0) == 0 {
        h.call_m(0x1738e8, &[log, D_8f9532, 5, 0, lit_failed], m);
        h.call_m(0x195cd0, &[wbuf, word], m);
        h.call_m(0x195cd0, &[pbuf, EMPTY_S], m);
        h.call_m(0x195cd0, &[posbuf, EMPTY_S], m);
        let mut i = 0u16;
        loop {
            if i >= h.call_m(0x195d90, &[word], m) as u16 {
                break;
            }
            h.call_m(0x195c90, &[pbuf, lit_sp], m);
            h.call_m(0x195c90, &[posbuf, lit_sp], m);
            i = i.wrapping_add(1);
        }
    }
    rc
}

/// `fn_2719ec`: the acronym-detection path (src/fn_26ff18.c), live over
/// the caller's arena. `loc` is the `FN_2719EC_FRAME` window holding
/// `ruleName`, `languageName`, the scratch literals and the callee
/// out-param slots. Every other callee is issued through the host; the
/// four callees no certified workload reaches (`fn_26dd20`,
/// `fn_271130`, `fn_2c8100`, `fn_26e560`) ride `call_m` like any other
/// nested entry so a future input that needs them fails closed with the
/// first unbound entry instead of diverging.
#[allow(clippy::too_many_arguments)]
pub fn fn_2719ec_m(
    m: &mut Memory<'_>,
    h: &mut impl Host,
    loc: u32,
    ctx: u32,
    text: u32,
    word: u32,
    handle_obj: u32,
    handle_data: u32,
    start_in: u32,
    end_in: u32,
    markers: u32,
    mi: u32,
    n_markers: u32,
    base: u32,
    flag: u32,
) -> u32 {
    let rule_name = loc.wrapping_add(F2719EC_RULE);
    let lang_name = loc.wrapping_add(F2719EC_LANG);
    let lit = loc.wrapping_add(F2719EC_LIT);
    let out = loc.wrapping_add(F2719EC_OUT);
    let out_node = out.wrapping_add(F2719EC_OUT_NODE);
    let out_entry = out.wrapping_add(F2719EC_OUT_ENTRY);
    let out_text = out.wrapping_add(F2719EC_OUT_TEXT);
    let out_tags = out.wrapping_add(F2719EC_OUT_TAGS);
    let out_phon = out.wrapping_add(F2719EC_OUT_PHON);
    let out_ortho = out.wrapping_add(F2719EC_OUT_ORTHO);
    let out_len = out.wrapping_add(F2719EC_OUT_LEN);
    let out_type = out.wrapping_add(F2719EC_OUT_TYPE);
    let out_fbuf = out.wrapping_add(F2719EC_OUT_FBUF);
    let out_attr = out.wrapping_add(F2719EC_OUT_ATTR);
    let out_wsize = out.wrapping_add(F2719EC_OUT_WSIZE);
    let out_lang = out.wrapping_add(F2719EC_OUT_B);
    let lit_acronym = lit.wrapping_add(F2719EC_LIT_ACRONYM);
    let lit_l1 = lit.wrapping_add(F2719EC_LIT_L1);
    let lit_l2 = lit.wrapping_add(F2719EC_LIT_L2);
    let lit_o2 = lit.wrapping_add(F2719EC_LIT_O2);
    let lit_o1 = lit.wrapping_add(F2719EC_LIT_O1);
    let lit_oneword = lit.wrapping_add(F2719EC_LIT_ONEWORD);
    let lit_l3 = lit.wrapping_add(F2719EC_LIT_L3);
    let lit_l4 = lit.wrapping_add(F2719EC_LIT_L4);
    let lit_o3 = lit.wrapping_add(F2719EC_LIT_O3);
    let lit_o5 = lit.wrapping_add(F2719EC_LIT_O5);
    let lit_normal = lit.wrapping_add(F2719EC_LIT_NORMAL);
    let lit_spell = lit.wrapping_add(F2719EC_LIT_SPELL);
    let lit_uscore = lit.wrapping_add(F2719EC_LIT_USCORE);
    let lit_cross = lit.wrapping_add(F2719EC_LIT_CROSS);
    let lit_prompt = lit.wrapping_add(F2719EC_LIT_PROMPT);
    let lit_e = lit.wrapping_add(F2719EC_LIT_E);
    let lit_escore = lit.wrapping_add(F2719EC_LIT_ESCORE);
    let lit_norm2 = lit.wrapping_add(F2719EC_LIT_NORM2);
    let lit_plus = lit.wrapping_add(F2719EC_LIT_PLUS);
    let lit_m11 = lit.wrapping_add(F2719EC_LIT_M11);
    let lit_sp = lit.wrapping_add(F2719EC_LIT_SP);
    let lit_tilde = lit.wrapping_add(F2719EC_LIT_TILDE);
    let env = g_word(m, ctx).unwrap_or(0);
    let heap = g_word(m, env.wrapping_add(4)).unwrap_or(0);
    let log = g_word(m, env.wrapping_add(16)).unwrap_or(0);
    let lat = g_word(m, ctx.wrapping_add(0x0c)).unwrap_or(0);
    let lexh_obj = g_word(m, ctx.wrapping_add(0x10)).unwrap_or(0);
    let lexh_data = g_word(m, ctx.wrapping_add(0x14)).unwrap_or(0);
    let lex = g_word(m, ctx.wrapping_add(0x18)).unwrap_or(0);
    let ruleh_obj = g_word(m, ctx.wrapping_add(0x1c)).unwrap_or(0);
    let ruleh_data = g_word(m, ctx.wrapping_add(0x20)).unwrap_or(0);
    let rule = g_word(m, ctx.wrapping_add(0x24)).unwrap_or(0);
    let mut accum0 = g_word(m, ctx.wrapping_add(0x40)).unwrap_or(0);
    let mut accum1 = g_word(m, ctx.wrapping_add(0x44)).unwrap_or(0);
    let mut accum2 = g_word(m, ctx.wrapping_add(0x48)).unwrap_or(0);
    let mut norm_r = g_word(m, ctx.wrapping_add(0x54)).unwrap_or(0);
    let mut text_off = g_half(m, ctx.wrapping_add(0x50)).unwrap_or(0);
    let f_get_attr = g_word(m, lat.wrapping_add(0x54)).unwrap_or(0);
    let f_set_attr = g_word(m, lat.wrapping_add(0x50)).unwrap_or(0);
    let f_get_ptr = g_word(m, lat.wrapping_add(0x58)).unwrap_or(0);
    let f_next = g_word(m, lat.wrapping_add(0x3c)).unwrap_or(0);
    let f_up = g_word(m, lat.wrapping_add(0x44)).unwrap_or(0);
    let f_insert_child = g_word(m, lat.wrapping_add(0x2c)).unwrap_or(0);
    let f_get_first = g_word(m, lat.wrapping_add(0x34)).unwrap_or(0);
    let f_remove = g_word(m, lat.wrapping_add(0x60)).unwrap_or(0);
    let f_lookup_word = g_word(m, lex.wrapping_add(0x20)).unwrap_or(0);
    let f_set_input = g_word(m, rule.wrapping_add(0x24)).unwrap_or(0);
    let f_get_output = g_word(m, rule.wrapping_add(0x28)).unwrap_or(0);
    let f_set_rules = g_word(m, rule.wrapping_add(0x2c)).unwrap_or(0);
    let f_set_sep = g_word(m, rule.wrapping_add(0x40)).unwrap_or(0);
    if !f2719ec_wlit(m, lit_acronym, b"acronym_detection\x00")
        || !f2719ec_wlit(m, lit_l1, b"%s L1: %s\x00")
        || !f2719ec_wlit(m, lit_o1, b"%s O1: %s\x00")
        || !f2719ec_wlit(m, lit_oneword, b"one_word\x00")
        || !f2719ec_wlit(m, lit_l3, b"%s L3: %s\x00")
        || !f2719ec_wlit(m, lit_l4, b"%s L4: %s\x00")
        || !f2719ec_wlit(m, lit_o3, b"%s O3: %s\x00")
        || !f2719ec_wlit(m, lit_o5, b"%s O5: %s\x00")
        || !f2719ec_wlit(m, lit_normal, b"normal\x00")
        || !f2719ec_wlit(m, lit_spell, b"spell_acronym\x00")
        || !f2719ec_wlit(m, lit_uscore, b"_\x00")
        || !f2719ec_wlit(m, lit_cross, b"cross_token\x00")
        || !f2719ec_wlit(m, lit_prompt, b"prompt\x00")
        || !f2719ec_wlit(m, lit_e, b"e\x00")
        || !f2719ec_wlit(m, lit_escore, b"_e\x00")
        || !f2719ec_wlit(m, lit_norm2, b"Normalizing to: %s\x00")
        || !f2719ec_wlit(m, lit_plus, b"+\x00")
        || !f2719ec_wlit(m, lit_m11, b"\x11\x00")
        || !f2719ec_wlit(m, lit_sp, b" \x00")
        || !f2719ec_wlit(m, lit_tilde, b"~\x00")
        || !f2719ec_wlit(m, lit_l2, b"%s L2: %s\x00")
        || !f2719ec_wlit(m, lit_o2, b"%s O2: %s\x00")
        || !f2719ec_wlit(
            m,
            lit.wrapping_add(F2719EC_LIT_NOAFF),
            b"Mde stem affix cfg settings do not apply on %s\x00",
        )
        || !f2719ec_wlit(
            m,
            lit.wrapping_add(F2719EC_LIT_AFFFAIL),
            b"Mde stem affix failed\x00",
        )
    {
        return FAIL_MEM;
    }
    let lit_noaff = lit.wrapping_add(F2719EC_LIT_NOAFF);
    let lit_afffail = lit.wrapping_add(F2719EC_LIT_AFFFAIL);
    h.call_m(0x195cd0, &[lang_name, EMPTY_S], m);
    h.call_m(0x195cd0, &[rule_name, lit_acronym], m);
    let rule_length = h.call_m(0x195d90, &[rule_name], m) & 0xffff;
    let split_word = g_byte(m, ctx.wrapping_add(0x173)).unwrap_or(0);
    let mut language_suffix = 0u32;
    let mut suffix_initial = b' ';
    if split_word == 1 {
        let p = h.call_m(0x195d30, &[word, 0x5f], m);
        if p != 0 {
            language_suffix = p.wrapping_add(1);
            suffix_initial = g_byte(m, language_suffix).unwrap_or(0);
        } else {
            language_suffix = word;
            suffix_initial = g_byte(m, word).unwrap_or(0);
        }
    }
    let mut node_ids = [0u16; 128];
    let mut rc = h.call_m(
        f_get_attr,
        &[
            handle_obj,
            handle_data,
            start_in & 0xffff,
            1,
            1,
            out_node,
            out_attr,
        ],
        m,
    );
    if rc >= 0x80000000 {
        return rc;
    }
    let mut text_start = g_half(m, out_node).unwrap_or(0);
    let mut attr_start = text_start;
    let mut start_r = start_in as u16;
    let mut node_r = 0u16;
    while attr_start == text_start && start_r != 0 {
        node_r = start_r;
        rc = h.call_m(
            f_up,
            &[handle_obj, handle_data, u32::from(start_r), out_node],
            m,
        );
        if rc >= 0x80000000 {
            return rc;
        }
        start_r = g_half(m, out_node).unwrap_or(0);
        if start_r != 0 {
            rc = h.call_m(
                f_get_attr,
                &[
                    handle_obj,
                    handle_data,
                    u32::from(start_r),
                    1,
                    1,
                    out_node,
                    out_attr,
                ],
                m,
            );
            if rc >= 0x80000000 {
                return rc;
            }
            attr_start = g_half(m, out_node).unwrap_or(0);
        }
    }
    start_r = node_r;
    let orth_p = g_word(m, ctx.wrapping_add(0x58)).unwrap_or(0);
    let phon_p = g_word(m, ctx.wrapping_add(0x5c)).unwrap_or(0);
    let pos_p = g_word(m, ctx.wrapping_add(0x60)).unwrap_or(0);
    let sep_p = g_word(m, ctx.wrapping_add(0x64)).unwrap_or(0);
    h.call_m(0x195cd0, &[orth_p, EMPTY_S], m);
    h.call_m(0x195cd0, &[phon_p, EMPTY_S], m);
    h.call_m(0x195cd0, &[pos_p, EMPTY_S], m);
    h.call_m(0x195cd0, &[sep_p, EMPTY_S], m);
    let end_r = end_in as u16;
    rc = h.call_m(
        f_get_attr,
        &[
            handle_obj,
            handle_data,
            u32::from(end_r),
            2,
            1,
            out_node,
            out_attr,
        ],
        m,
    );
    if rc >= 0x80000000 {
        return rc;
    }
    let mut text_end = g_half(m, out_node).unwrap_or(0);
    let mut span = text_end.wrapping_sub(text_start);
    let mut capacity_steps = 1u32;
    let mut marker_count = 0u32;
    let mut previous_priority = 1u32;
    let tmp0 = u32::from(span).wrapping_add(capacity_steps.wrapping_mul(10));
    if tmp0.wrapping_add(10) > 0x80 {
        let resized = h.call_m(0x1963e0, &[heap, accum0, tmp0.wrapping_add(10)], m);
        if resized == 0 {
            h.call_m(0x173dc0, &[log, D_8f9532, 0x84d0, 0], m);
            return 0x89b0200a;
        }
        accum0 = resized;
        if !p_word(m, ctx.wrapping_add(0x40), resized) {
            return FAIL_MEM;
        }
        let resized = h.call_m(0x1963e0, &[heap, accum1, tmp0.wrapping_add(10)], m);
        if resized == 0 {
            h.call_m(0x173dc0, &[log, D_8f9532, 0x84d0, 0], m);
            return 0x89b0200a;
        }
        accum1 = resized;
        if !p_word(m, ctx.wrapping_add(0x44), resized) {
            return FAIL_MEM;
        }
        let resized = h.call_m(0x1963e0, &[heap, accum2, tmp0.wrapping_add(10)], m);
        if resized == 0 {
            h.call_m(0x173dc0, &[log, D_8f9532, 0x84d0, 0], m);
            return 0x89b0200a;
        }
        accum2 = resized;
        if !p_word(m, ctx.wrapping_add(0x48), resized) {
            return FAIL_MEM;
        }
    }
    h.call_m(0x195cd0, &[accum0, EMPTY_S], m);
    node_r = start_r;
    let mut text_str = g_word(m, text).unwrap_or(0);
    while text_start < text_end
        && f1edd3c_is_delim(
            m,
            h,
            lexh_obj,
            lexh_data,
            u32::from(
                g_byte(
                    m,
                    text_str
                        .wrapping_add(u32::from(text_off).wrapping_add(u32::from(text_start))),
                )
                .unwrap_or(0),
            ),
        ) != 0
    {
        text_start = text_start.wrapping_add(1);
    }
    while text_start < text_end
        && f1edd3c_is_delim(
            m,
            h,
            lexh_obj,
            lexh_data,
            u32::from(
                g_byte(
                    m,
                    text_str.wrapping_add(
                        u32::from(text_off).wrapping_add(u32::from(text_end)).wrapping_sub(1),
                    ),
                )
                .unwrap_or(0),
            ),
        ) != 0
    {
        text_end = text_end.wrapping_sub(1);
    }
    let mut limit = text_start;
    let mut in_word = 1u32;
    let mut history_count = 0u32;
    let mut priority = 1u16;
    let plus_join = g_byte(m, ctx.wrapping_add(0x70)).unwrap_or(0);
    if u32::from(node_r) != end_in {
        loop {
            rc = h.call_m(
                f_get_attr,
                &[
                    handle_obj,
                    handle_data,
                    u32::from(node_r),
                    0,
                    1,
                    out_type,
                    out_attr,
                ],
                m,
            );
            if rc >= 0x80000000 {
                return rc;
            }
            let type_r = g_byte(m, out_type).unwrap_or(0);
            if type_r == 0 {
                if in_word == 0 {
                    rc = h.call_m(
                        f_get_attr,
                        &[
                            handle_obj,
                            handle_data,
                            u32::from(node_r),
                            1,
                            1,
                            out_node,
                            out_attr,
                        ],
                        m,
                    );
                    if rc >= 0x80000000 {
                        return rc;
                    }
                    attr_start = g_half(m, out_node).unwrap_or(0);
                    if (attr_start as i32).wrapping_sub(limit as i32) > 0 {
                        h.call_m(
                            0x195dac,
                            &[
                                accum0,
                                text_str.wrapping_add(
                                    u32::from(text_off).wrapping_add(u32::from(limit)),
                                ),
                                u32::from(attr_start.wrapping_sub(limit)),
                            ],
                            m,
                        );
                    }
                    if plus_join != 0 {
                        h.call_m(0x195c90, &[accum0, lit_plus], m);
                    }
                    limit = attr_start;
                }
                in_word = 0;
            } else if type_r == 4 || type_r == 8 {
                in_word = 1;
                rc = h.call_m(
                    f_get_attr,
                    &[
                        handle_obj,
                        handle_data,
                        u32::from(node_r),
                        1,
                        1,
                        out_node,
                        out_attr,
                    ],
                    m,
                );
                if rc >= 0x80000000 {
                    return rc;
                }
                attr_start = g_half(m, out_node).unwrap_or(0);
                rc = h.call_m(
                    f_get_attr,
                    &[
                        handle_obj,
                        handle_data,
                        u32::from(node_r),
                        2,
                        1,
                        out_node,
                        out_attr,
                    ],
                    m,
                );
                if rc >= 0x80000000 {
                    return rc;
                }
                let attr_end = g_half(m, out_node).unwrap_or(0);
                rc = h.call_m(
                    f_get_attr,
                    &[
                        handle_obj,
                        handle_data,
                        u32::from(node_r),
                        7,
                        1,
                        out_node,
                        out_attr,
                    ],
                    m,
                );
                if rc >= 0x80000000 {
                    return rc;
                }
                priority = g_half(m, out_node).unwrap_or(0);
                if limit != attr_end {
                    previous_priority = u32::from(priority);
                    node_ids[(history_count & 0x7f) as usize] = node_r;
                    history_count = history_count.wrapping_add(1) & 0xff;
                    if (attr_start as i32).wrapping_sub(limit as i32) > 0 {
                        h.call_m(
                            0x195dac,
                            &[
                                accum0,
                                text_str.wrapping_add(
                                    u32::from(text_off).wrapping_add(u32::from(limit)),
                                ),
                                u32::from(attr_start.wrapping_sub(limit)),
                            ],
                            m,
                        );
                    }
                    if capacity_steps.wrapping_mul(10) < marker_count.wrapping_add(2) {
                        capacity_steps = capacity_steps.wrapping_add(1) & 0xff;
                    }
                    let tmp = u32::from(span)
                        .wrapping_add(capacity_steps.wrapping_mul(10));
                    if tmp.wrapping_add(10) > 0x80 {
                        let resized =
                            h.call_m(0x1963e0, &[heap, accum0, tmp.wrapping_add(10)], m);
                        if resized == 0 {
                            h.call_m(0x173dc0, &[log, D_8f9532, 0x84d0, 0], m);
                            return 0x89b0200a;
                        }
                        accum0 = resized;
                        if !p_word(m, ctx.wrapping_add(0x40), resized) {
                            return FAIL_MEM;
                        }
                        let resized =
                            h.call_m(0x1963e0, &[heap, accum1, tmp.wrapping_add(10)], m);
                        if resized == 0 {
                            h.call_m(0x173dc0, &[log, D_8f9532, 0x84d0, 0], m);
                            return 0x89b0200a;
                        }
                        accum1 = resized;
                        if !p_word(m, ctx.wrapping_add(0x44), resized) {
                            return FAIL_MEM;
                        }
                        let resized =
                            h.call_m(0x1963e0, &[heap, accum2, tmp.wrapping_add(10)], m);
                        if resized == 0 {
                            h.call_m(0x173dc0, &[log, D_8f9532, 0x84d0, 0], m);
                            return 0x89b0200a;
                        }
                        accum2 = resized;
                        if !p_word(m, ctx.wrapping_add(0x48), resized) {
                            return FAIL_MEM;
                        }
                    }
                    h.call_m(0x195c90, &[accum0, lit_m11], m);
                    h.call_m(
                        0x195dac,
                        &[
                            accum0,
                            text_str.wrapping_add(
                                u32::from(text_off).wrapping_add(u32::from(attr_start)),
                            ),
                            u32::from(attr_end.wrapping_sub(attr_start)),
                        ],
                        m,
                    );
                    h.call_m(0x195c90, &[accum0, lit_m11], m);
                    marker_count = marker_count.wrapping_add(2) & 0xff;
                    limit = attr_end;
                } else if history_count != 0 {
                    if previous_priority <= u32::from(priority) {
                        let victim = node_ids[history_count.wrapping_sub(1) as usize & 0x7f];
                        rc = h.call_m(
                            f_remove,
                            &[handle_obj, handle_data, u32::from(victim)],
                            m,
                        );
                        if rc >= 0x80000000 {
                            return rc;
                        }
                        if victim == start_r {
                            start_r = node_r;
                        }
                        node_ids[history_count.wrapping_sub(1) as usize & 0x7f] = node_r;
                    } else {
                        let pos_node = node_r;
                        rc = h.call_m(
                            f_next,
                            &[handle_obj, handle_data, u32::from(pos_node), out_node],
                            m,
                        );
                        if rc >= 0x80000000 {
                            return rc;
                        }
                        node_r = g_half(m, out_node).unwrap_or(0);
                        rc = h.call_m(
                            f_remove,
                            &[handle_obj, handle_data, u32::from(pos_node)],
                            m,
                        );
                        if rc >= 0x80000000 {
                            return rc;
                        }
                        if u32::from(node_r) == end_in {
                            break;
                        }
                        continue;
                    }
                }
            } else {
                in_word = 1;
            }
            rc = h.call_m(
                f_next,
                &[handle_obj, handle_data, u32::from(node_r), out_node],
                m,
            );
            if rc >= 0x80000000 {
                return rc;
            }
            node_r = g_half(m, out_node).unwrap_or(0);
            if u32::from(node_r) == end_in {
                break;
            }
        }
    }
    rc = h.call_m(
        f_get_attr,
        &[
            handle_obj,
            handle_data,
            u32::from(node_r),
            0,
            1,
            out_type,
            out_attr,
        ],
        m,
    );
    if rc >= 0x80000000 {
        return rc;
    }
    let type_r = g_byte(m, out_type).unwrap_or(0);
    if type_r == 0 && in_word == 0 {
        rc = h.call_m(
            f_get_attr,
            &[
                handle_obj,
                handle_data,
                u32::from(node_r),
                1,
                1,
                out_node,
                out_attr,
            ],
            m,
        );
        if rc >= 0x80000000 {
            return rc;
        }
        attr_start = g_half(m, out_node).unwrap_or(0);
        if (attr_start as i32).wrapping_sub(limit as i32) > 0 {
            h.call_m(
                0x195dac,
                &[
                    accum0,
                    text_str.wrapping_add(u32::from(text_off).wrapping_add(u32::from(limit))),
                    u32::from(attr_start.wrapping_sub(limit)),
                ],
                m,
            );
        }
        if plus_join != 0 {
            h.call_m(0x195c90, &[accum0, lit_plus], m);
        }
        limit = attr_start;
    }
    if (text_end as i32).wrapping_sub(limit as i32) > 0 {
        h.call_m(
            0x195dac,
            &[
                accum0,
                text_str.wrapping_add(u32::from(text_off).wrapping_add(u32::from(limit))),
                u32::from(text_end.wrapping_sub(limit)),
            ],
            m,
        );
    }
    let mut output_len: u16;
    if flag != 0 {
        h.call_m(
            0x1738e8,
            &[log, D_8f9532, 5, 0, lit_l1, rule_name, accum0],
            m,
        );
        let tmp = h.call_m(0x195d90, &[accum0], m);
        rc = h.call_m(f_set_input, &[ruleh_obj, ruleh_data, 0, accum0, tmp], m);
        if rc >= 0x80000000 {
            return rc;
        }
        let tmp = h.call_m(0x195d58, &[word, lit_normal], m);
        if tmp != 0 {
            let tmp2 = h.call_m(0x195d90, &[word], m);
            if tmp2.wrapping_add(rule_length).wrapping_add(2) < 0x80 {
                h.call_m(0x195c90, &[rule_name, lit_uscore], m);
                h.call_m(0x195c90, &[rule_name, word], m);
            }
        }
        rc = h.call_m(f_set_rules, &[ruleh_obj, ruleh_data, rule_name], m);
        if rc >= 0x80000000 {
            if !p_byte(m, rule_name.wrapping_add(rule_length), 0) {
                return FAIL_MEM;
            }
            rc = h.call_m(f_set_rules, &[ruleh_obj, ruleh_data, rule_name], m);
            if rc >= 0x80000000 {
                return rc;
            }
        }
        rc = h.call_m(
            f_get_output,
            &[ruleh_obj, ruleh_data, 0, out_text, out_len],
            m,
        );

        if rc >= 0x80000000 {
            return rc;
        }
        let output_text = g_word(m, out_text).unwrap_or(0);
        let out_len_r = g_half(m, out_len).unwrap_or(0);
        if !p_byte(m, output_text.wrapping_add(u32::from(out_len_r)), 0) {
            return FAIL_MEM;
        }
        h.call_m(
            0x1738e8,
            &[log, D_8f9532, 5, 0, lit_o1, rule_name, output_text],
            m,
        );
        if u32::from(out_len_r).wrapping_add(1) >= 0x81 {
            let resized = h.call_m(
                0x1963e0,
                &[heap, norm_r, u32::from(out_len_r).wrapping_add(1)],
                m,
            );
            if resized == 0 {
                h.call_m(0x173dc0, &[log, D_8f9532, 0x84d0, 0], m);
                return 0x89b0200a;
            }
            norm_r = resized;
            if !p_word(m, ctx.wrapping_add(0x54), resized) {
                return FAIL_MEM;
            }
        }
        h.call_m(0x195cd0, &[norm_r, output_text], m);
        output_len = out_len_r;
        limit = out_len_r;
    } else {
        let tmp = h.call_m(0x195d90, &[accum0], m);
        if tmp.wrapping_add(1) > 0x80 {
            let tmp2 = h.call_m(0x195d90, &[accum0], m);
            let resized = h.call_m(0x1963e0, &[heap, norm_r, tmp2.wrapping_add(1)], m);
            if resized == 0 {
                h.call_m(0x173dc0, &[log, D_8f9532, 0x84d0, 0], m);
                return 0x89b0200a;
            }
            norm_r = resized;
            if !p_word(m, ctx.wrapping_add(0x54), resized) {
                return FAIL_MEM;
            }
        }
        h.call_m(0x195cd0, &[norm_r, accum0], m);
        let tmp = h.call_m(0x195d90, &[accum0], m);
        output_len = tmp.wrapping_add(1) as u16;
        limit = output_len;
    }
    let mut pos: u16 = 0;
    history_count = 0;
    let mut language = b' ';
    let mut normalization_delta = 0i16;
    let mut text_normalized = 0u32;
    let mut record_generated = 0u32;
    let cur_node_ptr = ctx.wrapping_add(0x68);
    if output_len != 0 {
        let last_pos = output_len.wrapping_sub(1);
        loop {
            let norm_byte = g_byte(m, norm_r.wrapping_add(u32::from(pos))).unwrap_or(0);
            let prompt_ne = h.call_m(0x195d58, &[word, lit_prompt], m) != 0;
            if norm_byte == 0x12 && prompt_ne {
                let content_start = u32::from(pos).wrapping_add(1);
                let mut token_end = content_start as u16;
                while token_end < limit
                    && g_byte(m, norm_r.wrapping_add(u32::from(token_end))).unwrap_or(0) != 0x12
                {
                    token_end = token_end.wrapping_add(1);
                }
                if !p_byte(m, norm_r.wrapping_add(u32::from(token_end)), 0) {
                    return FAIL_MEM;
                }
                rc = h.call_m(
                    f_lookup_word,
                    &[lexh_obj, lexh_data, word, norm_r.wrapping_add(content_start)],
                    m,
                );
                if rc >= 0x80000000 {
                    return rc;
                }
                h.call_m(0x195cd0, &[accum0, norm_r.wrapping_add(content_start)], m);
                h.call_m(0x195cd0, &[rule_name, lit_spell], m);
                let spell_rule_length = h.call_m(0x195d90, &[rule_name], m) & 0xffff;
                h.call_m(
                    0x1738e8,
                    &[log, D_8f9532, 5, 0, lit_l1, rule_name, accum0],
                    m,
                );
                let tmp = h.call_m(0x195d90, &[accum0], m);
                rc = h.call_m(f_set_input, &[ruleh_obj, ruleh_data, 0, accum0, tmp], m);
                if rc >= 0x80000000 {
                    return rc;
                }
                if h.call_m(0x195d58, &[word, lit_normal], m) != 0 {
                    let tmp2 = h.call_m(0x195d90, &[word], m);
                    if tmp2.wrapping_add(spell_rule_length).wrapping_add(2) < 0x80 {
                        h.call_m(0x195c90, &[rule_name, lit_uscore], m);
                        h.call_m(0x195c90, &[rule_name, word], m);
                    }
                }
                rc = h.call_m(f_set_rules, &[ruleh_obj, ruleh_data, rule_name], m);
                if rc >= 0x80000000 {
                    if !p_byte(m, rule_name.wrapping_add(spell_rule_length), 0) {
                        return FAIL_MEM;
                    }
                    rc = h.call_m(f_set_rules, &[ruleh_obj, ruleh_data, rule_name], m);
                    if rc >= 0x80000000 {
                        return rc;
                    }
                }
                rc = h.call_m(
                    f_get_output,
                    &[ruleh_obj, ruleh_data, 0, out_text, out_len],
                    m,
                );
                if rc >= 0x80000000 {
                    return rc;
                }
                let output_text = g_word(m, out_text).unwrap_or(0);
                let output_len_r = g_half(m, out_len).unwrap_or(0);
                if !p_byte(m, output_text.wrapping_add(u32::from(output_len_r)), 0) {
                    return FAIL_MEM;
                }
                h.call_m(
                    0x1738e8,
                    &[log, D_8f9532, 5, 0, lit_o1, rule_name, output_text],
                    m,
                );
                let tmp = h.call_m(0x195d90, &[output_text], m) as i32;
                let tmp2 = h.call_m(0x195d90, &[accum0], m) as i32;
                let delta = (tmp.wrapping_sub(tmp2) as u16) as i16;
                if delta > 0 {
                    let tmp2b = h.call_m(0x195d90, &[text_str], m);
                    let resized = h.call_m(
                        0x1963e0,
                        &[
                            heap,
                            text_str,
                            (delta as i32 + tmp2b as i32 + 1) as u16 as u32,
                        ],
                        m,
                    );
                    if resized == 0 {
                        h.call_m(0x173dc0, &[log, D_8f9532, 0x84d0, 0], m);
                        return 0x89b0200a;
                    }
                    if !p_word(m, text, resized) {
                        return FAIL_MEM;
                    }
                    text_str = resized;
                }
                let grow_out_len = h.call_m(0x195d90, &[output_text], m);
                let grow_need = (text_start as i32
                    + grow_out_len as i32
                    + text_off as i32) as u16;
                let text_len = h.call_m(0x195d90, &[text_str], m);
                if (delta as i32 + text_len as i32 + 1) as u16 as u32
                    >= u32::from(grow_need)
                {
                    // fits; no error
                } else {
                    h.call_m(0x173dc0, &[log, D_8f9532, 0x84d1, 0], m);
                    return 0x89b02007;
                }
                let tmp2d = h.call_m(0x195d90, &[accum0], m);
                text_end = (tmp2d as u16).wrapping_add(text_start);
                if delta < 1 {
                    if delta < 0 {
                        let mut mv = text_end.wrapping_add(text_off);
                        loop {
                            let slen = h.call_m(0x195d90, &[text_str], m) & 0xffff;
                            if (mv as u32) > slen {
                                break;
                            }
                            let dst =
                                (mv as i32).wrapping_add(delta as i32) as u32;
                            let b = g_byte(m, text_str.wrapping_add(u32::from(mv)))
                                .unwrap_or(0);
                            if !p_byte(m, text_str.wrapping_add(dst), b) {
                                return FAIL_MEM;
                            }
                            mv = mv.wrapping_add(1);
                        }
                    }
                } else {
                    let tail = h.call_m(0x195d90, &[text_str], m) as u16;
                    let mut count = (tail as i32)
                        .wrapping_sub(text_off as i32)
                        .wrapping_sub(text_end as i32)
                        .wrapping_add(1);
                    let mut src = text_str.wrapping_add(u32::from(tail)).wrapping_add(1);
                    let mut dst = src.wrapping_add(delta as u16 as u32);
                    while count > 0 {
                        src = src.wrapping_sub(1);
                        dst = dst.wrapping_sub(1);
                        let b = g_byte(m, src).unwrap_or(0);
                        if !p_byte(m, dst, b) {
                            return FAIL_MEM;
                        }
                        count -= 1;
                    }
                }
                let ae8_len = h.call_m(0x195d90, &[output_text], m);
                h.call_m(
                    0x195ae8,
                    &[
                        text_str
                            .wrapping_add(u32::from(text_off))
                            .wrapping_add(u32::from(text_start)),
                        output_text,
                        ae8_len,
                    ],
                    m,
                );
                h.call_m(
                    0x1738e8,
                    &[log, D_8f9532, 5, 0, lit_norm2, text_str],
                    m,
                );
                let tmp2e = h.call_m(0x195d90, &[accum0], m);
                text_start = (tmp2e as u16).wrapping_add(text_start);
                loop {
                    let pidx = g_half(m, mi).unwrap_or(0);
                    let pcnt = g_half(m, n_markers).unwrap_or(0);
                    if pidx >= pcnt {
                        break;
                    }
                    let mark_at = markers
                        .wrapping_add(u32::from(pidx).wrapping_mul(MARKER_LEN))
                        .wrapping_add(MARKER_POS);
                    let mtime = g_word(m, mark_at).unwrap_or(0) as i32;
                    let d = mtime.wrapping_sub(base as i32) as u16 as i16;
                    if (text_end as i32) <= d as i32 {
                        break;
                    }
                    if !p_word(
                        m,
                        mark_at,
                        mtime.wrapping_add(text_off as i32) as u32,
                    ) {
                        return FAIL_MEM;
                    }
                    if !p_half(m, mi, pidx.wrapping_add(1)) {
                        return FAIL_MEM;
                    }
                }
                let mut node_start = 0u16;
                let mut node_end = 0u16;
                rc = h.call_m(
                    f_get_first,
                    &[handle_obj, handle_data, 1, 0, out_node],
                    m,
                );
                if rc >= 0x80000000 {
                    return rc;
                }
                let root = g_half(m, out_node).unwrap_or(0);
                rc = h.call_m(
                    f_get_first,
                    &[handle_obj, handle_data, 2, u32::from(root), out_node],
                    m,
                );
                if rc >= 0x80000000 {
                    return rc;
                }
                let mut current = g_half(m, out_node).unwrap_or(0);
                let mut tmp_cur = g_half(m, cur_node_ptr).unwrap_or(0) as i32;
                if tmp_cur != current as i32 {
                    loop {
                        rc = h.call_m(
                            f_get_attr,
                            &[
                                handle_obj,
                                handle_data,
                                u32::from(current),
                                2,
                                1,
                                out_node,
                                out_attr,
                            ],
                            m,
                        );
                        if rc >= 0x80000000 {
                            return rc;
                        }
                        node_end = g_half(m, out_node).unwrap_or(0);
                        if (text_end as i32)
                            <= (node_end as i32).wrapping_sub(text_off as i32)
                        {
                            node_end = (delta as i32 + node_end as i16 as i32) as u16;
                        }
                        if !p_half(m, out_node, node_end) {
                            return FAIL_MEM;
                        }
                        rc = h.call_m(
                            f_set_attr,
                            &[
                                handle_obj,
                                handle_data,
                                u32::from(current),
                                2,
                                1,
                                out_node,
                                out_wsize,
                            ],
                            m,
                        );
                        if rc >= 0x80000000 {
                            return rc;
                        }
                        rc = h.call_m(
                            f_next,
                            &[handle_obj, handle_data, u32::from(current), out_node],
                            m,
                        );
                        if rc >= 0x80000000 {
                            return rc;
                        }
                        current = g_half(m, out_node).unwrap_or(0);
                        tmp_cur = g_half(m, cur_node_ptr).unwrap_or(0) as i32;
                        if tmp_cur == current as i32 {
                            break;
                        }
                    }
                }
                if tmp_cur != 0 {
                    rc = h.call_m(
                        f_get_attr,
                        &[
                            handle_obj,
                            handle_data,
                            tmp_cur as u32,
                            1,
                            1,
                            out_node,
                            out_attr,
                        ],
                        m,
                    );
                    if rc >= 0x80000000 {
                        return rc;
                    }
                    node_start = g_half(m, out_node).unwrap_or(0);
                }
                loop {
                    let cur = g_half(m, cur_node_ptr).unwrap_or(0);
                    if !(node_start < text_end && cur != 0) {
                        break;
                    }
                    rc = h.call_m(
                        f_get_attr,
                        &[
                            handle_obj,
                            handle_data,
                            u32::from(cur),
                            0,
                            1,
                            out_type,
                            out_attr,
                        ],
                        m,
                    );
                    if rc >= 0x80000000 {
                        return rc;
                    }
                    let t = g_byte(m, out_type).unwrap_or(0);
                    if t == 5 || t == 6 || t == 4 || t == 9 || t == 10 || t == 7 {
                        rc = h.call_m(
                            f_get_attr,
                            &[
                                handle_obj,
                                handle_data,
                                u32::from(cur),
                                2,
                                1,
                                out_node,
                                out_attr,
                            ],
                            m,
                        );
                        if rc >= 0x80000000 {
                            return rc;
                        }
                        node_end = g_half(m, out_node).unwrap_or(0);
                        node_start = text_off.wrapping_add(node_start);
                        if text_end <= node_end {
                            node_end = (delta as i32 + node_end as i16 as i32) as u16;
                        }
                        node_end = (text_off as i32 + node_end as i16 as i32) as u16;
                        if !p_half(m, out_node, node_start) {
                            return FAIL_MEM;
                        }
                        rc = h.call_m(
                            f_set_attr,
                            &[
                                handle_obj,
                                handle_data,
                                u32::from(cur),
                                1,
                                1,
                                out_node,
                                out_wsize,
                            ],
                            m,
                        );
                        if rc >= 0x80000000 {
                            return rc;
                        }
                        if !p_half(m, out_node, node_end) {
                            return FAIL_MEM;
                        }
                        if !p_half(m, out_node, node_end) {
                            return FAIL_MEM;
                        }
                        rc = h.call_m(
                            f_set_attr,
                            &[
                                handle_obj,
                                handle_data,
                                u32::from(cur),
                                2,
                                1,
                                out_node,
                                out_wsize,
                            ],
                            m,
                        );
                        if rc >= 0x80000000 {
                            return rc;
                        }
                    }
                    rc = h.call_m(
                        f_next,
                        &[handle_obj, handle_data, u32::from(cur), cur_node_ptr],
                        m,
                    );
                    if rc >= 0x80000000 {
                        return rc;
                    }
                    let cur2 = g_half(m, cur_node_ptr).unwrap_or(0);
                    if cur2 != 0 {
                        rc = h.call_m(
                            f_get_attr,
                            &[
                                handle_obj,
                                handle_data,
                                u32::from(cur2),
                                1,
                                1,
                                out_node,
                                out_attr,
                            ],
                            m,
                        );
                        if rc >= 0x80000000 {
                            return rc;
                        }
                        node_start = g_half(m, out_node).unwrap_or(0);
                    }
                }
                normalization_delta =
                    (normalization_delta as i32 + delta as i32) as i16;
                text_normalized = 1;
                text_off = text_off.wrapping_add(delta as u16);
                if !p_half(m, ctx.wrapping_add(0x50), text_off) {
                    return FAIL_MEM;
                }
                if !p_byte(m, out_lang, language) {
                    return FAIL_MEM;
                }
                // The single-character G2P word path runs in-component
                // (seg1 r10): 1:1 port below, same as the nested 270b40.
                rc = fn_271130_m(m, h, loc, ctx, word, output_text, out_lang, lang_name);
                if rc >= 0x80000000 {
                    return rc;
                }
                language = g_byte(m, out_lang).unwrap_or(0);
                pos = token_end.wrapping_add(1);
                if pos >= limit {
                    break;
                }
                continue;
            }
            let nb2 = g_byte(m, norm_r.wrapping_add(u32::from(pos))).unwrap_or(0);
            let mut token_end = 0u16;
            let mut do_output = false;
            if nb2 != 0x15 && (nb2 != 0x12 || prompt_ne) {
                let c0 = g_byte(m, norm_r.wrapping_add(u32::from(pos))).unwrap_or(0);
                token_end = pos;
                if c0 == 0x11 {
                    while {
                        token_end = token_end.wrapping_add(1);
                        token_end < limit
                            && g_byte(m, norm_r.wrapping_add(u32::from(token_end)))
                                .unwrap_or(0)
                                != 0x11
                    } {
                        text_start = text_start.wrapping_add(1);
                    }
                    let hid = (history_count & 0x7f) as usize;
                    let hn = node_ids[hid];
                    rc = h.call_m(
                        f_get_ptr,
                        &[handle_obj, handle_data, u32::from(hn), 3, out_text, out_attr],
                        m,
                    );
                    if rc >= 0x80000000 {
                        return rc;
                    }
                    let orthography = g_word(m, out_text).unwrap_or(0);
                    rc = h.call_m(
                        f_get_ptr,
                        &[handle_obj, handle_data, u32::from(hn), 5, out_phon, out_attr],
                        m,
                    );
                    if rc >= 0x80000000 {
                        return rc;
                    }
                    let phonetic = g_word(m, out_phon).unwrap_or(0);
                    rc = h.call_m(
                        f_get_ptr,
                        &[handle_obj, handle_data, u32::from(hn), 6, out_ortho, out_attr],
                        m,
                    );
                    if rc >= 0x80000000 {
                        return rc;
                    }
                    let part_of_speech = g_word(m, out_ortho).unwrap_or(0);
                    rc = h.call_m(
                        f_get_attr,
                        &[
                            handle_obj,
                            handle_data,
                            u32::from(hn),
                            0,
                            1,
                            out_type,
                            out_attr,
                        ],
                        m,
                    );
                    if rc >= 0x80000000 {
                        return rc;
                    }
                    let saved_type = g_byte(m, out_type).unwrap_or(0);
                    let hid2 = (history_count & 0x7f) as usize;
                    let hn2 = node_ids[hid2];
                    rc = h.call_m(
                        f_get_ptr,
                        &[handle_obj, handle_data, u32::from(hn2), 9, out_tags, out_attr],
                        m,
                    );
                    history_count = history_count.wrapping_add(1) & 0xff;
                    if rc >= 0x80000000 {
                        return rc;
                    }
                    let stored_language = g_word(m, out_tags).unwrap_or(0);
                    if !f195e38_copy(m, lang_name, stored_language, 10) {
                        return FAIL_MEM;
                    }
                    language = g_byte(m, lang_name).unwrap_or(0);
                    if language == 0 {
                        language = b' ';
                    }
                    if saved_type != 4 {
                        rc = fn_270b40_m(
                            m,
                            h,
                            loc,
                            ctx,
                            orthography,
                            phonetic,
                            part_of_speech,
                            u32::from(language),
                            0,
                        );
                        if rc >= 0x80000000 {
                            return rc;
                        }
                    } else if h.call_m(0x195d58, &[orthography, lit_uscore], m) != 0 {
                        rc = fn_270b40_m(
                            m,
                            h,
                            loc,
                            ctx,
                            orthography,
                            phonetic,
                            part_of_speech,
                            u32::from(language),
                            1,
                        );
                        if rc >= 0x80000000 {
                            return rc;
                        }
                    }
                } else {
                    let letter_at = out_lang;
                    let lowered = h.call_m(0x240b28, &[u32::from(c0)], m) as u8;
                    if !p_byte(m, letter_at, lowered) {
                        return FAIL_MEM;
                    }
                    if !p_byte(m, letter_at.wrapping_add(1), 0) {
                        return FAIL_MEM;
                    }
                    rc = fn_270b40_m(m, h, loc, ctx, letter_at, EMPTY_S, EMPTY_S, 0x20, 0);
                    if rc >= 0x80000000 {
                        return rc;
                    }
                    text_start = text_start.wrapping_add(1);
                }
            } else {
                token_end = pos.wrapping_add(1);
                while {
                    let te = (token_end & 0xffff) as u16;
                    token_end = te;
                    te < limit
                        && {
                            let cc = g_byte(m, norm_r.wrapping_add(u32::from(te)))
                                .unwrap_or(0);
                            cc != 0x15 && cc != 0x12
                        }
                } {
                    token_end = token_end.wrapping_add(1);
                }
                if !p_byte(m, norm_r.wrapping_add(u32::from(token_end)), 0) {
                    return FAIL_MEM;
                }
                rc = fn_26fc54_m(
                    m,
                    h,
                    loc,
                    ctx,
                    word,
                    norm_r.wrapping_add(u32::from(pos)).wrapping_add(1),
                );
                if rc >= 0x80000000 {
                    return rc;
                }
                let tmp = h.call_m(0x195d90, &[accum0], m);
                text_start = (tmp as u16).wrapping_add(text_start);
                h.call_m(0x195cd0, &[rule_name, lit_oneword], m);
                let rule_length2 = h.call_m(0x195d90, &[rule_name], m) & 0xffff;
                h.call_m(
                    0x1738e8,
                    &[log, D_8f9532, 5, 0, lit_l1, rule_name, accum0],
                    m,
                );
                let tmp = h.call_m(0x195d90, &[accum0], m);
                rc = h.call_m(f_set_input, &[ruleh_obj, ruleh_data, 0, accum0, tmp], m);
                if rc >= 0x80000000 {
                    return rc;
                }
                h.call_m(
                    0x1738e8,
                    &[log, D_8f9532, 5, 0, lit_l3, rule_name, accum1],
                    m,
                );
                let tmp = h.call_m(0x195d90, &[accum1], m);
                rc = h.call_m(f_set_input, &[ruleh_obj, ruleh_data, 2, accum1, tmp], m);
                if rc >= 0x80000000 {
                    return rc;
                }
                h.call_m(
                    0x1738e8,
                    &[log, D_8f9532, 5, 0, lit_l4, rule_name, accum2],
                    m,
                );
                let tmp = h.call_m(0x195d90, &[accum2], m);
                rc = h.call_m(f_set_sep, &[ruleh_obj, ruleh_data, 3, accum2, tmp], m);
                if rc >= 0x80000000 {
                    return rc;
                }
                let mut rc2 = 0x89b02014u32;
                let mut _next_taken = false;
                let mut run_gen = true;
                if h.call_m(0x195d58, &[word, lit_normal], m) != 0 {
                    rc2 = h.call_m(
                        0x1966ac,
                        &[g_word(m, ctx.wrapping_add(0x174)).unwrap_or(0), word, out_entry],
                        m,
                    );
                }
                if rc2 < 0x80000000 {
                    let entry = g_word(m, out_entry).unwrap_or(0);
                    let dict_id = g_word(m, entry.wrapping_add(8)).unwrap_or(0);
                    let mut dict_ok = false;
                    if pos == 0 && token_end == last_pos {
                        rc = h.call_m(
                            f_insert_child,
                            &[handle_obj, handle_data, u32::from(start_r), out_node],
                            m,
                        );
                        if rc >= 0x80000000 {
                            return rc;
                        }
                        node_r = g_half(m, out_node).unwrap_or(0);
                        rc = h.call_m(
                            0x26dd20,
                            &[ctx, u32::from(node_r), handle_obj, handle_data, dict_id, word],
                            m,
                        );
                        if rc >= 0x80000000 {
                            return rc;
                        }
                        record_generated = 1;
                        dict_ok = true;
                    } else {
                        rc2 = h.call_m(
                            0x26dd20,
                            &[ctx, 0, handle_obj, handle_data, dict_id, word],
                            m,
                        );
                        if rc2 < 0x80000000 {
                            dict_ok = true;
                        }
                    }
                    if dict_ok {
                        if g_byte(m, ctx.wrapping_add(0x173)).unwrap_or(0) == 0 {
                            language = g_byte(m, word).unwrap_or(0);
                            if !f195e38_copy(m, lang_name, word, 10) {
                                return FAIL_MEM;
                            }
                        } else if language_suffix != 0 {
                            language = suffix_initial;
                            if !f195e38_copy(m, lang_name, language_suffix, 10) {
                                return FAIL_MEM;
                            }
                        }
                        _next_taken = true;
                        run_gen = false;
                    }
                }
                if run_gen {
                    if h.call_m(0x195d58, &[word, lit_normal], m) != 0 {
                        let wlen = h.call_m(0x195d90, &[word], m);
                        if wlen.wrapping_add(rule_length2).wrapping_add(2) < 0x80 {
                            h.call_m(0x195c90, &[rule_name, lit_uscore], m);
                            h.call_m(0x195c90, &[rule_name, word], m);
                            rc2 = h.call_m(
                                f_set_rules,
                                &[ruleh_obj, ruleh_data, rule_name],
                                m,
                            );
                        }
                    }
                    if rc2 < 0x80000000 {
                        if split_word == 0 {
                            language = g_byte(m, word).unwrap_or(0);
                            if !f195e38_copy(m, lang_name, word, 10) {
                                return FAIL_MEM;
                            }
                        } else if language_suffix != 0 {
                            language = suffix_initial;
                            if !f195e38_copy(m, lang_name, language_suffix, 10) {
                                return FAIL_MEM;
                            }
                        }
                        do_output = true;
                    } else {
                        let mut fbuf = 0u8;
                        let lidenx = g_byte(m, ctx.wrapping_add(0x1a4)).unwrap_or(0);
                        if lidenx != 0 {
                            if h.call_m(0x195d90, &[word], m) > 1
                                && h.call_m(0x195d30, &[word, 0x5f], m) == 0
                            {
                                rc = h.call_m(0x2c8100, &[ctx, out_fbuf], m);
                                if rc >= 0x80000000 {
                                    return rc;
                                }
                                fbuf = g_byte(m, out_fbuf).unwrap_or(0);
                            }
                            if fbuf != 0 {
                                if !p_byte(m, rule_name.wrapping_add(rule_length2), 0) {
                                    return FAIL_MEM;
                                }
                                h.call_m(0x195c90, &[rule_name, lit_escore], m);
                                language = b'e';
                                h.call_m(0x195cd0, &[lang_name, lit_e], m);
                                rc = h.call_m(
                                    f_set_rules,
                                    &[ruleh_obj, ruleh_data, rule_name],
                                    m,
                                );
                                if rc >= 0x80000000 {
                                    return rc;
                                }
                                do_output = true;
                            }
                        }
                        if !do_output {
                            rc = h.call_m(
                                0x1966ac,
                                &[
                                    g_word(m, ctx.wrapping_add(0x174)).unwrap_or(0),
                                    lit_normal,
                                    out_entry,
                                ],
                                m,
                            );
                            if rc >= 0x80000000 {
                                if !p_byte(m, rule_name.wrapping_add(rule_length2), 0) {
                                    return FAIL_MEM;
                                }
                                rc = h.call_m(
                                    f_set_rules,
                                    &[ruleh_obj, ruleh_data, rule_name],
                                    m,
                                );
                                if rc >= 0x80000000 {
                                    return rc;
                                }
                                do_output = true;
                            } else {
                                let entry = g_word(m, out_entry).unwrap_or(0);
                                let dict_id = g_word(m, entry.wrapping_add(8)).unwrap_or(0);
                                if pos == 0 && token_end == last_pos {
                                    rc = h.call_m(
                                        f_insert_child,
                                        &[
                                            handle_obj,
                                            handle_data,
                                            u32::from(start_r),
                                            out_node,
                                        ],
                                        m,
                                    );
                                    if rc >= 0x80000000 {
                                        return rc;
                                    }
                                    node_r = g_half(m, out_node).unwrap_or(0);
                                    rc = h.call_m(
                                        0x26dd20,
                                        &[
                                            ctx,
                                            u32::from(node_r),
                                            handle_obj,
                                            handle_data,
                                            dict_id,
                                            word,
                                        ],
                                        m,
                                    );
                                    if rc >= 0x80000000 {
                                        return rc;
                                    }
                                    record_generated = 1;
                                } else {
                                    rc = h.call_m(
                                        0x26dd20,
                                        &[ctx, 0, handle_obj, handle_data, dict_id, word],
                                        m,
                                    );
                                    if rc >= 0x80000000 {
                                        if !p_byte(
                                            m,
                                            rule_name.wrapping_add(rule_length2),
                                            0,
                                        ) {
                                            return FAIL_MEM;
                                        }
                                        rc = h.call_m(
                                            f_set_rules,
                                            &[ruleh_obj, ruleh_data, rule_name],
                                            m,
                                        );
                                        if rc >= 0x80000000 {
                                            return rc;
                                        }
                                        do_output = true;
                                    }
                                }
                                if !do_output {
                                    _next_taken = true;
                                }
                            }
                        }
                    }
                }
                if do_output {
                    rc = h.call_m(
                        f_get_output,
                        &[ruleh_obj, ruleh_data, 0, out_text, out_len],
                        m,
                    );
                    if rc >= 0x80000000 {
                        return rc;
                    }
                    let output_text = g_word(m, out_text).unwrap_or(0);
                    rc = h.call_m(
                        f_get_output,
                        &[ruleh_obj, ruleh_data, 2, out_tags, out_len],
                        m,
                    );
                    if rc >= 0x80000000 {
                        return rc;
                    }
                    let output_tags = g_word(m, out_tags).unwrap_or(0);
                    let mut output_tags_len = g_half(m, out_len).unwrap_or(0);
                    if !p_byte(
                        m,
                        output_tags.wrapping_add(u32::from(output_tags_len)),
                        0,
                    ) {
                        return FAIL_MEM;
                    }
                    rc = h.call_m(
                        f_get_output,
                        &[ruleh_obj, ruleh_data, 4, out_ortho, out_len],
                        m,
                    );
                    if rc >= 0x80000000 {
                        return rc;
                    }
                    let orthography = g_word(m, out_ortho).unwrap_or(0);
                    let mut output_len_r = g_half(m, out_len).unwrap_or(0);
                    if !p_byte(
                        m,
                        orthography.wrapping_add(u32::from(output_len_r)),
                        0,
                    ) {
                        return FAIL_MEM;
                    }
                    if !p_byte(
                        m,
                        output_text.wrapping_add(u32::from(output_len_r)),
                        0,
                    ) {
                        return FAIL_MEM;
                    }
                    h.call_m(
                        0x1738e8,
                        &[log, D_8f9532, 5, 0, lit_o1, rule_name, output_text],
                        m,
                    );
                    h.call_m(
                        0x1738e8,
                        &[log, D_8f9532, 5, 0, lit_o3, rule_name, output_tags],
                        m,
                    );
                    h.call_m(
                        0x1738e8,
                        &[log, D_8f9532, 5, 0, lit_o5, rule_name, orthography],
                        m,
                    );
                    let mut strip_pos = 0u16;
                    while strip_pos < output_len_r {
                        let b = g_byte(
                            m,
                            output_text.wrapping_add(u32::from(strip_pos)),
                        )
                        .unwrap_or(0);
                        if b == b'~' {
                            let mut ii = strip_pos;
                            while ii < output_len_r {
                                let bt = g_byte(
                                    m,
                                    output_text.wrapping_add(u32::from(ii).wrapping_add(1)),
                                )
                                .unwrap_or(0);
                                if !p_byte(m, output_text.wrapping_add(u32::from(ii)), bt)
                                {
                                    return FAIL_MEM;
                                }
                                let bg = g_byte(
                                    m,
                                    output_tags.wrapping_add(u32::from(ii).wrapping_add(1)),
                                )
                                .unwrap_or(0);
                                if !p_byte(m, output_tags.wrapping_add(u32::from(ii)), bg)
                                {
                                    return FAIL_MEM;
                                }
                                let bo = g_byte(
                                    m,
                                    orthography.wrapping_add(u32::from(ii).wrapping_add(1)),
                                )
                                .unwrap_or(0);
                                if !p_byte(m, orthography.wrapping_add(u32::from(ii)), bo)
                                {
                                    return FAIL_MEM;
                                }
                                ii = ii.wrapping_add(1);
                            }
                            output_len_r = output_len_r.wrapping_sub(1);
                            strip_pos = strip_pos.wrapping_sub(1);
                        } else if b == 0xad {
                            if !p_byte(
                                m,
                                output_text.wrapping_add(u32::from(strip_pos)),
                                b'~',
                            ) {
                                return FAIL_MEM;
                            }
                        }
                        strip_pos = strip_pos.wrapping_add(1);
                    }
                    if !p_byte(
                        m,
                        output_text.wrapping_add(u32::from(output_len_r)),
                        0,
                    ) {
                        return FAIL_MEM;
                    }
                    let mut phon_start = 0u16;
                    loop {
                        let cc = g_byte(
                            m,
                            output_tags.wrapping_add(u32::from(phon_start)),
                        )
                        .unwrap_or(0);
                        if cc != b' ' && cc != b'~' {
                            break;
                        }
                        phon_start = phon_start.wrapping_add(1);
                    }
                    if g_byte(m, output_tags.wrapping_add(u32::from(phon_start)))
                        .unwrap_or(0)
                        != 0
                    {
                        let mut jj = phon_start;
                        while {
                            let cc =
                                g_byte(m, output_tags.wrapping_add(u32::from(jj)))
                                    .unwrap_or(0);
                            cc != b' ' && cc != b'~'
                        } {
                            jj = jj.wrapping_add(1);
                        }
                        if !p_byte(m, output_tags.wrapping_add(u32::from(jj)), 0) {
                            return FAIL_MEM;
                        }
                    }
                    let mut node_start2 = 0u16;
                    loop {
                        let cc = g_byte(
                            m,
                            orthography.wrapping_add(u32::from(node_start2)),
                        )
                        .unwrap_or(0);
                        if cc != b' ' && cc != b'~' {
                            break;
                        }
                        node_start2 = node_start2.wrapping_add(1);
                    }
                    if g_byte(m, orthography.wrapping_add(u32::from(node_start2)))
                        .unwrap_or(0)
                        != 0
                    {
                        let mut jj = node_start2;
                        while {
                            let cc = g_byte(
                                m,
                                orthography.wrapping_add(u32::from(jj)),
                            )
                            .unwrap_or(0);
                            cc != b' ' && cc != b'~'
                        } {
                            jj = jj.wrapping_add(1);
                        }
                        if !p_byte(m, orthography.wrapping_add(u32::from(jj)), 0) {
                            return FAIL_MEM;
                        }
                    }
                    rc = fn_270b40_m(
                        m,
                        h,
                        loc,
                        ctx,
                        output_text,
                        output_tags.wrapping_add(u32::from(phon_start)),
                        orthography.wrapping_add(u32::from(node_start2)),
                        u32::from(language),
                        1,
                    );
                    if rc >= 0x80000000 {
                        return rc;
                    }
                }
                }
                // c3 round 24 (re-landing r22): C next_token (src/fn_26ff18.c)
                // runs pos = tokenEnd+1 for EVERY path including the letter
                // branch, which leaves do_output false. It lived inside
                // `if do_output` after the seg1 rewrite, hanging letters.
                pos = token_end.wrapping_add(1);
                if pos >= limit {
                    break;
                }
        }
    }
    if language == b' ' {
        language = 0;
    }
    // Re-read the shared buffers: nested 270b40 calls reallocate them
    // through the context on long words.
    let mut orth_now = g_word(m, ctx.wrapping_add(0x58)).unwrap_or(0);
    let mut phon_now = g_word(m, ctx.wrapping_add(0x5c)).unwrap_or(0);
    let mut pos_now = g_word(m, ctx.wrapping_add(0x60)).unwrap_or(0);
    let mut sep_now = g_word(m, ctx.wrapping_add(0x64)).unwrap_or(0);
    if record_generated == 0 {
        in_word = 0;
        h.call_m(0x195cd0, &[rule_name, lit_cross], m);
        let cross_len = h.call_m(0x195d90, &[rule_name], m);
        let _ = cross_len;
        let tmp = h.call_m(0x195d90, &[orth_now], m) as u16;
        if u32::from(tmp).wrapping_add(1) > 0x80 {
            let resized = h.call_m(
                0x1963e0,
                &[heap, accum0, u32::from(tmp).wrapping_add(1)],
                m,
            );
            if resized == 0 {
                h.call_m(0x173dc0, &[log, D_8f9532, 0x84d0, 0], m);
                return 0x89b0200a;
            }
            accum0 = resized;
            if !p_word(m, ctx.wrapping_add(0x40), resized) {
                return FAIL_MEM;
            }
            let resized = h.call_m(
                0x1963e0,
                &[heap, accum1, u32::from(tmp).wrapping_add(1)],
                m,
            );
            if resized == 0 {
                h.call_m(0x173dc0, &[log, D_8f9532, 0x84d0, 0], m);
                return 0x89b0200a;
            }
            accum1 = resized;
            if !p_word(m, ctx.wrapping_add(0x44), resized) {
                return FAIL_MEM;
            }
            let resized = h.call_m(
                0x1963e0,
                &[heap, accum2, u32::from(tmp).wrapping_add(1)],
                m,
            );
            if resized == 0 {
                h.call_m(0x173dc0, &[log, D_8f9532, 0x84d0, 0], m);
                return 0x89b0200a;
            }
            accum2 = resized;
            if !p_word(m, ctx.wrapping_add(0x48), resized) {
                return FAIL_MEM;
            }
            let accum3 = g_word(m, ctx.wrapping_add(0x4c)).unwrap_or(0);
            let resized = h.call_m(
                0x1963e0,
                &[heap, accum3, u32::from(tmp).wrapping_add(1)],
                m,
            );
            if resized == 0 {
                h.call_m(0x173dc0, &[log, D_8f9532, 0x84d0, 0], m);
                return 0x89b0200a;
            }
            if !p_word(m, ctx.wrapping_add(0x4c), resized) {
                return FAIL_MEM;
            }
        }
        h.call_m(0x195cd0, &[accum0, orth_now], m);
        h.call_m(0x195cd0, &[accum1, phon_now], m);
        h.call_m(0x195cd0, &[accum2, pos_now], m);
        let accum3 = g_word(m, ctx.wrapping_add(0x4c)).unwrap_or(0);
        h.call_m(0x195cd0, &[accum3, sep_now], m);
        let mut ci = 0u16;
        let cross_n = h.call_m(0x195d90, &[orth_now], m);
        if cross_n != 0 {
            loop {
                let cb = g_byte(m, accum0.wrapping_add(u32::from(ci))).unwrap_or(0);
                match cb {
                    0x11 => in_word = if in_word == 0 { 1 } else { 0 },
                    0x7e => {
                        if in_word != 0
                            && !p_byte(m, accum0.wrapping_add(u32::from(ci)), 0xad)
                        {
                            return FAIL_MEM;
                        }
                    }
                    _ => {}
                }
                ci = ci.wrapping_add(1);
                if !(ci < h.call_m(0x195d90, &[orth_now], m) as u16) {
                    break;
                }
            }
        }
        h.call_m(0x195cd0, &[orth_now, EMPTY_S], m);
        h.call_m(0x195cd0, &[phon_now, EMPTY_S], m);
        h.call_m(0x195cd0, &[pos_now, EMPTY_S], m);
        text_start = 0;
        loop {
            h.call_m(0x195cd0, &[rule_name, lit_cross], m);
            limit = h.call_m(0x195d90, &[rule_name], m) as u16;
            if text_start != 0 {
                h.call_m(0x195c90, &[orth_now, lit_sp], m);
                h.call_m(0x195c90, &[phon_now, lit_sp], m);
                h.call_m(0x195c90, &[pos_now, lit_sp], m);
            }
            text_end = text_start;
            let pp = accum0;
            loop {
                pos = text_end;
                let cc = g_byte(m, pp.wrapping_add(u32::from(pos))).unwrap_or(0);
                if cc == 0
                    || cc == b' '
                    || cc == b'\t'
                    || cc == b'\n'
                    || cc == b'\r'
                    || cc == b'_'
                {
                    break;
                }
                text_end = text_end.wrapping_add(1);
            }
            let saved_c = g_byte(m, pp.wrapping_add(u32::from(pos))).unwrap_or(0);
            if !p_byte(m, pp.wrapping_add(u32::from(pos)), 0) {
                return FAIL_MEM;
            }
            let accum1b = g_word(m, ctx.wrapping_add(0x44)).unwrap_or(0);
            let accum2b = g_word(m, ctx.wrapping_add(0x48)).unwrap_or(0);
            let accum3b = g_word(m, ctx.wrapping_add(0x4c)).unwrap_or(0);
            if !p_byte(m, accum1b.wrapping_add(u32::from(text_end)), 0) {
                return FAIL_MEM;
            }
            if !p_byte(m, accum2b.wrapping_add(u32::from(text_end)), 0) {
                return FAIL_MEM;
            }
            if !p_byte(m, accum3b.wrapping_add(u32::from(text_end)), 0) {
                return FAIL_MEM;
            }
            h.call_m(
                0x1738e8,
                &[log, D_8f9532, 5, 0, lit_l1, rule_name, accum0.wrapping_add(u32::from(text_start))],
                m,
            );
            let tmp = h.call_m(
                0x195d90,
                &[accum0.wrapping_add(u32::from(text_start))],
                m,
            );
            rc = h.call_m(
                f_set_input,
                &[ruleh_obj, ruleh_data, 0, accum0.wrapping_add(u32::from(text_start)), tmp],
                m,
            );
            if rc >= 0x80000000 {
                return rc;
            }
            h.call_m(
                0x1738e8,
                &[log, D_8f9532, 5, 0, lit_l2, accum1b.wrapping_add(u32::from(text_start))],
                m,
            );
            h.call_m(
                0x1738e8,
                &[log, D_8f9532, 5, 0, lit_l2, rule_name, accum1b.wrapping_add(u32::from(text_start))],
                m,
            );
            let tmp = h.call_m(
                0x195d90,
                &[accum1b.wrapping_add(u32::from(text_start))],
                m,
            );
            rc = h.call_m(
                f_set_input,
                &[ruleh_obj, ruleh_data, 1, accum1b.wrapping_add(u32::from(text_start)), tmp],
                m,
            );
            if rc >= 0x80000000 {
                return rc;
            }
            h.call_m(
                0x1738e8,
                &[log, D_8f9532, 5, 0, lit_l3, rule_name, accum2b.wrapping_add(u32::from(text_start))],
                m,
            );
            let tmp = h.call_m(
                0x195d90,
                &[accum2b.wrapping_add(u32::from(text_start))],
                m,
            );
            rc = h.call_m(
                f_set_input,
                &[ruleh_obj, ruleh_data, 2, accum2b.wrapping_add(u32::from(text_start)), tmp],
                m,
            );
            if rc >= 0x80000000 {
                return rc;
            }
            h.call_m(
                0x1738e8,
                &[log, D_8f9532, 5, 0, lit_l4, rule_name, accum3b.wrapping_add(u32::from(text_start))],
                m,
            );
            let tmp = h.call_m(
                0x195d90,
                &[accum3b.wrapping_add(u32::from(text_start))],
                m,
            );
            rc = h.call_m(
                f_set_sep,
                &[ruleh_obj, ruleh_data, 3, accum3b.wrapping_add(u32::from(text_start)), tmp],
                m,
            );
            if rc >= 0x80000000 {
                return rc;
            }
            if h.call_m(0x195d58, &[word, lit_normal], m) != 0 {
                let wlen = h.call_m(0x195d90, &[word], m);
                if wlen.wrapping_add(limit as u32).wrapping_add(2) < 0x80 {
                    h.call_m(0x195c90, &[rule_name, lit_uscore], m);
                    h.call_m(0x195c90, &[rule_name, word], m);
                }
            }
            rc = h.call_m(f_set_rules, &[ruleh_obj, ruleh_data, rule_name], m);
            if rc < 0x80000000 {
                if language == 0 {
                    if split_word == 0 {
                        language = g_byte(m, word).unwrap_or(0);
                        if !f195e38_copy(m, lang_name, word, 10) {
                            return FAIL_MEM;
                        }
                    } else if language_suffix != 0 {
                        language = suffix_initial;
                        if !f195e38_copy(m, lang_name, language_suffix, 10) {
                            return FAIL_MEM;
                        }
                    }
                }
            } else {
                if !p_byte(m, rule_name.wrapping_add(u32::from(limit)), 0) {
                    return FAIL_MEM;
                }
                rc = h.call_m(f_set_rules, &[ruleh_obj, ruleh_data, rule_name], m);
                if rc >= 0x80000000 {
                    return rc;
                }
            }
            rc = h.call_m(
                f_get_output,
                &[ruleh_obj, ruleh_data, 0, out_text, out_len],
                m,
            );
            if rc >= 0x80000000 {
                return rc;
            }
            let cross_text = g_word(m, out_text).unwrap_or(0);
            let cross_len_r = g_half(m, out_len).unwrap_or(0);
            if !p_byte(m, cross_text.wrapping_add(u32::from(cross_len_r)), 0) {
                return FAIL_MEM;
            }
            h.call_m(
                0x1738e8,
                &[log, D_8f9532, 5, 0, lit_o1, rule_name, cross_text],
                m,
            );
            rc = h.call_m(
                f_get_output,
                &[ruleh_obj, ruleh_data, 1, out_phon, out_len],
                m,
            );
            if rc >= 0x80000000 {
                return rc;
            }
            let cross_phon = g_word(m, out_phon).unwrap_or(0);
            let cross_phon_len = g_half(m, out_len).unwrap_or(0);
            if !p_byte(m, cross_phon.wrapping_add(u32::from(cross_phon_len)), 0) {
                return FAIL_MEM;
            }
            h.call_m(
                0x1738e8,
                &[log, D_8f9532, 5, 0, lit_o2, rule_name, cross_phon],
                m,
            );
            rc = h.call_m(
                f_get_output,
                &[ruleh_obj, ruleh_data, 2, out_tags, out_len],
                m,
            );
            if rc >= 0x80000000 {
                return rc;
            }
            let cross_tags = g_word(m, out_tags).unwrap_or(0);
            let cross_tags_len = g_half(m, out_len).unwrap_or(0);
            if !p_byte(m, cross_tags.wrapping_add(u32::from(cross_tags_len)), 0) {
                return FAIL_MEM;
            }
            h.call_m(
                0x1738e8,
                &[log, D_8f9532, 5, 0, lit_o3, rule_name, cross_tags],
                m,
            );
            limit = cross_tags_len;
            let mut stri = 0u16;
            if limit != 0 {
                loop {
                    let b = g_byte(m, cross_text.wrapping_add(u32::from(stri))).unwrap_or(0);
                    if b == b'~' {
                        let mut ppos = stri;
                        while ppos < limit {
                            let bt = g_byte(m, cross_text.wrapping_add(u32::from(ppos).wrapping_add(1))).unwrap_or(0);
                            if !p_byte(m, cross_text.wrapping_add(u32::from(ppos)), bt) {
                                return FAIL_MEM;
                            }
                            ppos = ppos.wrapping_add(1);
                        }
                        limit = limit.wrapping_sub(1);
                        stri = stri.wrapping_sub(1);
                    } else if b == 0xad {
                        if !p_byte(m, cross_text.wrapping_add(u32::from(stri)), b'~') {
                            return FAIL_MEM;
                        }
                    }
                    stri = stri.wrapping_add(1);
                    if !(stri < limit) {
                        break;
                    }
                }
            }
            limit = cross_tags_len;
            let mut stri = 0u16;
            if limit != 0 {
                loop {
                    let b = g_byte(m, cross_phon.wrapping_add(u32::from(stri))).unwrap_or(0);
                    if b == b'~' {
                        let mut ppos = stri;
                        while ppos < limit {
                            let bt = g_byte(m, cross_phon.wrapping_add(u32::from(ppos).wrapping_add(1))).unwrap_or(0);
                            if !p_byte(m, cross_phon.wrapping_add(u32::from(ppos)), bt) {
                                return FAIL_MEM;
                            }
                            ppos = ppos.wrapping_add(1);
                        }
                        limit = limit.wrapping_sub(1);
                        stri = stri.wrapping_sub(1);
                    } else if b == 0xad {
                        if !p_byte(m, cross_phon.wrapping_add(u32::from(stri)), b'~') {
                            return FAIL_MEM;
                        }
                    }
                    stri = stri.wrapping_add(1);
                    if !(stri < limit) {
                        break;
                    }
                }
            }
            limit = cross_tags_len;
            let mut stri = 0u16;
            if limit != 0 {
                loop {
                    let b = g_byte(m, cross_tags.wrapping_add(u32::from(stri))).unwrap_or(0);
                    if b == b'~' {
                        let mut ppos = stri;
                        while ppos < limit {
                            let bt = g_byte(m, cross_tags.wrapping_add(u32::from(ppos).wrapping_add(1))).unwrap_or(0);
                            if !p_byte(m, cross_tags.wrapping_add(u32::from(ppos)), bt) {
                                return FAIL_MEM;
                            }
                            ppos = ppos.wrapping_add(1);
                        }
                        limit = limit.wrapping_sub(1);
                        stri = stri.wrapping_sub(1);
                    } else if b == 0xad {
                        if !p_byte(m, cross_tags.wrapping_add(u32::from(stri)), b'~') {
                            return FAIL_MEM;
                        }
                    }
                    stri = stri.wrapping_add(1);
                    if !(stri < limit) {
                        break;
                    }
                }
            }
            let orth_len_now = h.call_m(0x195d90, &[orth_now], m);
            let need = (h.call_m(0x195d90, &[cross_text], m).wrapping_add(orth_len_now).wrapping_add(2)) as u16;
            if need < 0x81 {
                let resized = orth_now;
                let _ = resized;
            } else {
                let resized = h.call_m(0x1963e0, &[heap, orth_now, u32::from(need)], m);
                if resized == 0 {
                    h.call_m(0x173dc0, &[log, D_8f9532, 0x84d0, 0], m);
                    return 0x89b0200a;
                }
                orth_now = resized;
                if !p_word(m, ctx.wrapping_add(0x58), resized) {
                    return FAIL_MEM;
                }
            }
            h.call_m(0x195c90, &[orth_now, cross_text], m);
            let mut scan = 0u32;
            loop {
                let c2 = g_byte(m, cross_phon.wrapping_add(scan)).unwrap_or(0);
                if c2 != b' ' && c2 != b'~' {
                    break;
                }
                scan = scan.wrapping_add(1);
            }
            limit = scan as u16;
            if g_byte(m, cross_phon.wrapping_add(scan)).unwrap_or(0) != 0 {
                while {
                    let c3 = g_byte(m, cross_phon.wrapping_add(u32::from(limit))).unwrap_or(0);
                    c3 != b' ' && c3 != b'~'
                } {
                    limit = limit.wrapping_add(1);
                }
                if !p_byte(m, cross_phon.wrapping_add(u32::from(limit)), 0) {
                    return FAIL_MEM;
                }
            }
            let scan_off = scan;
            let tmpn = h.call_m(0x195d58, &[cross_phon.wrapping_add(scan_off), EMPTY_S], m);
            let (phondst, srcp) = if tmpn == 0 {
                (phon_now, STAR_S)
            } else {
                (phon_now, cross_phon.wrapping_add(scan_off))
            };
            h.call_m(0x195c90, &[phondst, srcp], m);
            let mut scan2 = 0u32;
            loop {
                let c2 = g_byte(m, cross_tags.wrapping_add(scan2)).unwrap_or(0);
                if c2 != b' ' && c2 != b'~' {
                    break;
                }
                scan2 = scan2.wrapping_add(1);
            }
            limit = scan2 as u16;
            if g_byte(m, cross_tags.wrapping_add(scan2)).unwrap_or(0) != 0 {
                while {
                    let c3 = g_byte(m, cross_tags.wrapping_add(u32::from(limit))).unwrap_or(0);
                    c3 != b' ' && c3 != b'~'
                } {
                    limit = limit.wrapping_add(1);
                }
                if !p_byte(m, cross_tags.wrapping_add(u32::from(limit)), 0) {
                    return FAIL_MEM;
                }
            }
            let scan2_off = scan2;
            let tmpp = h.call_m(0x195d58, &[cross_tags.wrapping_add(scan2_off), EMPTY_S], m);
            let (posdst, srcpp) = if tmpp == 0 {
                (pos_now, STAR_S)
            } else {
                (pos_now, cross_tags.wrapping_add(scan2_off))
            };
            h.call_m(0x195c90, &[posdst, srcpp], m);
            if saved_c == 0 {
                break;
            }
            text_start = text_end.wrapping_add(1);
        }
    }
    if !p_byte(m, phon_now.wrapping_add(1), 0) {
        return FAIL_MEM;
    }
    if !p_byte(m, pos_now.wrapping_add(1), 0) {
        return FAIL_MEM;
    }
    // cross_done:
    if record_generated == 0 {
        rc = h.call_m(
            f_insert_child,
            &[handle_obj, handle_data, u32::from(start_r), out_node],
            m,
        );
        if rc >= 0x80000000 {
            return rc;
        }
        node_r = g_half(m, out_node).unwrap_or(0);
    }
    // epi_done:
    rc = h.call_m(
        f_get_attr,
        &[handle_obj, handle_data, u32::from(start_r), 0, 1, out_type, out_attr],
        m,
    );
    if rc >= 0x80000000 {
        return rc;
    }
    let epi_type = g_byte(m, out_type).unwrap_or(0);
    rc = h.call_m(
        f_get_attr,
        &[handle_obj, handle_data, u32::from(start_r), 1, 1, out_node, out_attr],
        m,
    );
    if rc >= 0x80000000 {
        return rc;
    }
    attr_start = g_half(m, out_node).unwrap_or(0);
    if text_normalized != 0 && epi_type != 4 {
        attr_start =
            (text_off as i32 + attr_start as i32 - normalization_delta as i32) as u16;
    }
    if !p_half(m, out_node, attr_start) {
        return FAIL_MEM;
    }
    rc = h.call_m(
        f_set_attr,
        &[handle_obj, handle_data, u32::from(node_r), 1, 1, out_node, out_wsize],
        m,
    );
    if rc >= 0x80000000 {
        return rc;
    }
    if !p_byte(m, out_type, b'\t') {
        return FAIL_MEM;
    }
    rc = h.call_m(
        f_set_attr,
        &[handle_obj, handle_data, u32::from(node_r), 0, 1, out_type, out_attr],
        m,
    );
    if rc >= 0x80000000 {
        return rc;
    }
    rc = h.call_m(
        f_get_attr,
        &[handle_obj, handle_data, u32::from(end_r), 2, 1, out_node, out_attr],
        m,
    );
    if rc >= 0x80000000 {
        return rc;
    }
    let mut attr_end = g_half(m, out_node).unwrap_or(0);
    if text_normalized != 0 {
        attr_end =
            (text_off as i32 + attr_end as i32 - normalization_delta as i32) as u16;
    }
    attr_end = (normalization_delta as i32 + attr_end as i32) as u16;
    if !p_half(m, out_node, attr_end) {
        return FAIL_MEM;
    }
    rc = h.call_m(
        f_set_attr,
        &[handle_obj, handle_data, u32::from(node_r), 2, 1, out_node, out_wsize],
        m,
    );
    if rc >= 0x80000000 {
        return rc;
    }
    let orth_now2 = g_word(m, ctx.wrapping_add(0x58)).unwrap_or(0);
    let phon_now2 = g_word(m, ctx.wrapping_add(0x5c)).unwrap_or(0);
    let pos_now2 = g_word(m, ctx.wrapping_add(0x60)).unwrap_or(0);
    let tmp = h.call_m(0x195d90, &[orth_now2], m);
    rc = h.call_m(
        f_set_attr,
        &[handle_obj, handle_data, u32::from(node_r), 3, tmp.wrapping_add(1), orth_now2, out_wsize],
        m,
    );
    if rc >= 0x80000000 {
        return rc;
    }
    let tmp = h.call_m(0x195d90, &[phon_now2], m);
    rc = h.call_m(
        f_set_attr,
        &[handle_obj, handle_data, u32::from(node_r), 5, tmp.wrapping_add(1), phon_now2, out_wsize],
        m,
    );
    if rc >= 0x80000000 {
        return rc;
    }
    let tmp = h.call_m(0x195d90, &[pos_now2], m);
    rc = h.call_m(
        f_set_attr,
        &[handle_obj, handle_data, u32::from(node_r), 6, tmp.wrapping_add(1), pos_now2, out_wsize],
        m,
    );
    if rc >= 0x80000000 {
        return rc;
    }
    if !p_half(m, out_node, priority) {
        return FAIL_MEM;
    }
    rc = h.call_m(
        f_set_attr,
        &[handle_obj, handle_data, u32::from(node_r), 7, 1, out_node, out_wsize],
        m,
    );
    if rc >= 0x80000000 {
        return rc;
    }
    if g_byte(m, ctx.wrapping_add(0x2a8)).unwrap_or(0) != 0
        && h.call_m(0x195d58, &[lang_name, lit_normal], m) == 0
    {
        let lang_code = g_word(m, ctx.wrapping_add(0x2ac)).unwrap_or(0);
        h.call_m(0x195cd0, &[lang_name, lang_code], m);
        let mut li = 0u8;
        while li < 3 {
            let lb = g_byte(m, lang_name.wrapping_add(u32::from(li))).unwrap_or(0);
            let folded = h.call_m(0x240ba0, &[u32::from(lb)], m) as u8;
            if !p_byte(m, lang_name.wrapping_add(u32::from(li)), folded) {
                return FAIL_MEM;
            }
            li = li.wrapping_add(1);
        }
    }
    let tmp = h.call_m(0x195d90, &[lang_name], m);
    rc = h.call_m(
        f_set_attr,
        &[handle_obj, handle_data, u32::from(node_r), 9, tmp.wrapping_add(1), lang_name, out_wsize],
        m,
    );
    if rc < 0x80000000 && history_count != 0 {
        limit = 0;
        loop {
            if g_half(m, cur_node_ptr).unwrap_or(0) == node_ids[(limit & 0x7f) as usize] {
                rc = h.call_m(
                    f_next,
                    &[handle_obj, handle_data, g_half(m, cur_node_ptr).unwrap_or(0) as u32, cur_node_ptr],
                    m,
                );
                if rc >= 0x80000000 {
                    break;
                }
            }
            rc = h.call_m(
                f_remove,
                &[handle_obj, handle_data, u32::from(node_ids[(limit & 0x7f) as usize])],
                m,
            );
            if rc >= 0x80000000 {
                break;
            }
            limit = limit.wrapping_add(1);
            if history_count <= u32::from(limit) {
                return rc;
            }
        }
    }
    rc
}

// ===== seg1 round 4: the acronym rule's live dependencies =====
//
// `fn_2719ec` runs depes rules whose expansion step calls back into the
// dictionary (`0x1ea4e0`), which opens a broker resource (`0x179f30`).
// Both ride host interception here, in this file, so `engine_live.rs`
// is untouched: `XHost` serves the rule-module run (`0x2193a4`) with a
// `LexExpansion` that performs the ARM's positional `0x1ea4e0` calls,
// and `BrokerAdapter` serves the resource open through the live broker
// port. Everything else forwards to the caller's lane-b host.

// Guest scratch for the expansion's dictionary query: the NUL-terminated
// query copy, the result vector slot, the count halfword and the
// separator byte. Dead ARM stack below the 2719ec frame, inside the
// excluded stack range, clear of every lane-b/seg3 scratch window.
const EXPAND_SCRATCH: u32 = 0xe0fbe40;
const EXPAND_QUERY: u32 = 0x00;
const EXPAND_QUERY_MAX: usize = 128;
const EXPAND_OUT: u32 = 0x80;
const EXPAND_COUNT: u32 = 0x84;
const EXPAND_SEP: u32 = 0x86;

// Guest scratch for the live broker open: the file platform's 4096 bytes,
// the DataOpen globals' 24 bytes, the broker query's 16 bytes and 256
// bytes of log-literal space. Dead ARM stack below the 2719ec frame,
// inside the excluded stack range, clear of the lane-b/seg3 scratch
// windows and of engine_live's own broker block.
const BROKER_FILE_SCRATCH: u32 = 0xe0fad00;
const BROKER_DATA_SCRATCH: u32 = 0xe0fbd00;
const BROKER_QUERY_SCRATCH: u32 = 0xe0fbd20;
const BROKER_LIT_BASE: u32 = 0xe0fbd40;
const BROKER_LIT_END: u32 = 0xe0fbe40;

/// Plain forwarder: run a lane-b port on another lane-b host.
struct BAdapter<'a, H: Host + ?Sized> {
    inner: &'a mut H,
}

impl<H: Host + ?Sized> crate::component_live_b::Host for BAdapter<'_, H> {
    fn call_m(&mut self, entry: u32, args: &[u32], m: &mut Memory<'_>) -> u32 {
        self.inner.call_m(entry, args, m)
    }
    fn rom(&self) -> crate::rom_region::Rom {
        self.inner.rom()
    }
}

/// The depes lexicon expansion (the ROM's `0x1ea4e0` call inside the
/// acronym rule): resolve the rule entry for `code` through the depes
/// entries table at `ctx + 0x3a4` and run the certified dictionary
/// lookup, mapping the result table to the depes status/text. Anything
/// unresolvable fails closed, like the ARM's own missing-entry path.
/// Guest ends buffer for the depes expansion mirror, mirroring the
/// ROM's heap ends buffer (allocated per expansion, freed after the
/// splice). `buf`/`heap`/`len` describe the live buffer, if any.
struct EndsMirror {
    buf: u32,
    heap: u32,
    len: u32,
}
struct LexExpansion<'a, H: Host + ?Sized> {
    ctx: u32,
    inner: BAdapter<'a, H>,
    mirror: Option<EndsMirror>,
}

impl<H: Host + ?Sized> crate::component_live_c::VmExpansion for LexExpansion<'_, H> {
    fn expand_m(
        &mut self,
        handle: u32,
        selector: u32,
        code: u8,
        text: &[u8],
        out: &mut [u8],
        ends: &mut [i16],
        m: &mut Memory<'_>,
    ) -> Result<(i16, u8), crate::frontend_types::Error> {
        use crate::frontend_types::Error;
        // Mirror the previous expansion's final ends and release its
        // buffer before this expansion allocates, matching the ROM's
        // per-expansion alloc/use/free order.
        if self.mirror.is_some() {
            let prev = self.mirror.take();
            if let Some(p) = prev {
                for (i, &v) in ends.iter().take(p.len as usize).enumerate() {
                    let at = p.buf.wrapping_add((i as u32).wrapping_mul(2));
                    if g_range(m, at, 2) {
                        let off = (at.wrapping_sub(m.base)) as usize;
                        m.bytes[off..off + 2].copy_from_slice(&v.to_le_bytes());
                    }
                }
                self.inner.call_m(0x196428, &[p.heap, p.buf], m);
            }
        }
        if code == 0 {
            return Err(Error::InvalidResource);
        }
        // entry = entries[code-1], entries table at depes ctx+0x3a4.
        let table =
            g_word(m, self.ctx.wrapping_add(0x3a4)).ok_or(Error::InvalidResource)?;
        let entry = g_word(
            m,
            table.wrapping_add(u32::from(code).wrapping_sub(1).wrapping_mul(4)),
        )
        .ok_or(Error::InvalidResource)?;
        if entry == 0 {
            return Err(Error::InvalidResource);
        }
        // Stage the NUL-terminated query in guest scratch.
        let mut qlen = 0usize;
        while qlen < text.len() && qlen < EXPAND_QUERY_MAX && text[qlen] != 0 {
            qlen += 1;
        }
        if qlen >= EXPAND_QUERY_MAX {
            return Err(Error::InvalidResource);
        }
        let qbase = EXPAND_SCRATCH.wrapping_add(EXPAND_QUERY);
        for (i, &b) in text[..qlen].iter().enumerate() {
            if !p_byte(m, qbase.wrapping_add(i as u32), b) {
                return Err(Error::InvalidResource);
            }
        }
        if !p_byte(m, qbase.wrapping_add(qlen as u32), 0) {
            return Err(Error::InvalidResource);
        }
        // Fresh single-lookup count, matching the ROM's stack setup.
        let out_at = EXPAND_SCRATCH.wrapping_add(EXPAND_OUT);
        let count_at = EXPAND_SCRATCH.wrapping_add(EXPAND_COUNT);
        let sep_at = EXPAND_SCRATCH.wrapping_add(EXPAND_SEP);
        if !p_word(m, out_at, 0)
            || !p_half(m, count_at, 0xffff)
            || !p_byte(m, sep_at, 0)
        {
            return Err(Error::InvalidResource);
        }
        let rc = crate::component_live_b4::fn_1ea4e0_m(
            m,
            &mut self.inner,
            handle,
            selector,
            entry,
            qbase,
            out_at,
            count_at,
            sep_at,
        );
        let count0 = g_half(m, count_at).unwrap_or(0xffff);
        let sep0 = g_byte(m, sep_at).unwrap_or(0);
        if count0 == 1 {
            let r0 = g_word(m, g_word(m, out_at).unwrap_or(0)).unwrap_or(0);
        }
        if rc >= 0x80000000 {
            return Err(Error::InvalidResource);
        }
        let count = count0;
        if count != 1 {
            return Ok((0, 0));
        }
        let results = g_word(m, out_at).ok_or(Error::InvalidResource)?;
        if results == 0 {
            return Ok((0, 0));
        }
        // Join the result strings with the returned separator.
        let sep = g_byte(m, sep_at).ok_or(Error::InvalidResource)?;
        for e in ends.iter_mut() {
            *e = 0;
        }
        if ends.len() < crate::component_live_c::VM_LAYERS {
            return Err(Error::InvalidResource);
        }
        let mut pos = 0usize;
        let mut ri = 0u32;
        while u32::from(ri) < u32::from(count) {
            let sp =
                g_word(m, results.wrapping_add(ri * 4)).ok_or(Error::InvalidResource)?;
            if sp == 0 {
                return Err(Error::InvalidResource);
            }
            if ri != 0 {
                if pos >= out.len() {
                    return Err(Error::InvalidResource);
                }
                out[pos] = sep;
                pos += 1;
            }
            let mut si = 0u32;
            loop {
                let b = g_byte(m, sp.wrapping_add(si)).ok_or(Error::InvalidResource)?;
                if pos >= out.len() {
                    return Err(Error::InvalidResource);
                }
                out[pos] = b;
                pos += 1;
                si += 1;
                if b == 0 || si > 512 {
                    break;
                }
            }
            ri += 1;
        }
        // Mirror the ROM's in-place separator termination: the ARM
        // splices guest text directly and NULs each separator it passes.
        // NUL every separator in the used guest result strings so the
        // heap image matches (the VM NULs only its Rust copy).
        {
            let mut rj = 0u32;
            while u32::from(rj) < u32::from(count) {
                let spj = g_word(m, results.wrapping_add(rj.wrapping_mul(4))).unwrap_or(0);
                if spj == 0 {
                    break;
                }
                let mut sj = 0u32;
                loop {
                    let bj = g_byte(m, spj.wrapping_add(sj)).unwrap_or(0);
                    if bj == 0 || sj > 512 {
                        break;
                    }
                    if bj == sep {
                        if !p_byte(m, spj.wrapping_add(sj), 0) {
                            return Err(Error::InvalidResource);
                        }
                    }
                    sj = sj.wrapping_add(1);
                }
                rj = rj.wrapping_add(1);
            }
        }
        // Mirror the ROM's per-expansion ends allocation: free the
        // previous buffer (its ends were mirrored on entry), then take
        // a fresh heap buffer for this expansion's ends. The heap and
        // layer count follow the ROM (ctx+0x3a8, ctx+0x0c).
        let heap = g_word(m, self.ctx.wrapping_add(0x3a8)).unwrap_or(0);
        let layers = g_half(m, self.ctx.wrapping_add(0x0c)).unwrap_or(0);
        if heap == 0 || layers == 0 || layers as usize > crate::component_live_c::VM_LAYERS {
            return Err(Error::InvalidResource);
        }
        let buf = self.inner.call_m(0x196394, &[heap, u32::from(layers).wrapping_mul(2)], m);
        if buf == 0 || buf >= 0x80000000 {
            return Err(Error::InvalidResource);
        }
        self.mirror = Some(EndsMirror { buf, heap, len: u32::from(layers) });
        Ok((1, sep))
    }
    fn post_expand(&mut self, ends: &[i16], m: &mut Memory<'_>) {
        // Mirror the final ends to the guest buffer and release it,
        // matching the ROM's alloc/use/free per expansion. Best effort:
        // a short write still frees the buffer.
        if let Some(cur) = self.mirror.take() {
            for (i, &v) in ends.iter().take(cur.len as usize).enumerate() {
                let at = cur.buf.wrapping_add((i as u32).wrapping_mul(2));
                if g_range(m, at, 2) {
                    let off = (at.wrapping_sub(m.base)) as usize;
                    m.bytes[off..off + 2].copy_from_slice(&v.to_le_bytes());
                }
            }
            self.inner.call_m(0x196428, &[cur.heap, cur.buf], m);
        }
    }
}

/// `fn_2195e4` with the acronym expansion: the module table lives at
/// `ctx + 0x08` with its size at `ctx + 0x04`. Byte for byte the certified
/// `depes_execute_module_m` walk, except the VM runs with `LexExpansion`
/// so the acronym rule's expansion opcodes perform the ARM's positional
/// `0x1ea4e0` calls instead of failing closed.
fn depes_execute_module_expanded_m<H: Host + ?Sized>(
    m: &mut Memory<'_>,
    key: u32,
    ctx: u32,
    end_ptr: u32,
    w: &mut crate::component_live_c::VmWorkspace,
    h: &mut H,
) -> u32 {
    let size = g_word(m, ctx.wrapping_add(0x04)).unwrap_or(0);
    let data = g_word(m, ctx.wrapping_add(0x08)).unwrap_or(0);
    let mut offset = 0u32;
    while offset < size {
        let different = h.call_m(0x1769d0, &[data.wrapping_add(offset), key], m);
        if different == 0 {
            break;
        }
        offset += g_strlen(m, data.wrapping_add(offset)).unwrap_or(0) + 1;
        while offset & 3 != 0 {
            offset += 1;
        }
        let length = g_word(m, data.wrapping_add(offset)).unwrap_or(0);
        offset += 4 + length;
    }
    if size <= offset {
        return 0x89702004;
    }
    let mut position = offset + g_strlen(m, data.wrapping_add(offset)).unwrap_or(0) + 1;
    while position & 3 != 0 {
        position += 1;
    }
    let length = g_word(m, data.wrapping_add(position)).unwrap_or(0);
    position += 4;
    let mut buffer = [0u8; 104];
    let mut index = 0u32;
    while index < length && index < 104 {
        buffer[index as usize] = g_byte(m, data.wrapping_add(position).wrapping_add(index)).unwrap_or(0);
        index += 1;
    }
    if !p_half(m, ctx.wrapping_add(0x37c), 0) {
        return FAIL_MEM;
    }
    if !p_byte(m, ctx.wrapping_add(0x37b), 0) {
        return FAIL_MEM;
    }
    let half6c = g_half(m, g_word(m, ctx.wrapping_add(0x6c)).unwrap_or(0)).unwrap_or(0);
    if !p_half(m, ctx.wrapping_add(0x378), half6c) {
        return FAIL_MEM;
    }
    let buffer_size = g_half(m, ctx.wrapping_add(0x22)).unwrap_or(0);
    let slots = g_word(m, ctx.wrapping_add(0x3a0)).unwrap_or(0);
    let mut off = 0u32;
    while off < u32::from(buffer_size) {
        if !p_byte(m, slots.wrapping_add(off), 0) {
            return FAIL_MEM;
        }
        off += 1;
    }
    if !p_byte(m, ctx.wrapping_add(0x39a), 1) {
        return FAIL_MEM;
    }
    let mut ba = BrokerAdapter { inner: h };
    let mut index = 0u32;
    while index < length {
        let mut expansion = LexExpansion {
            ctx,
            inner: BAdapter { inner: &mut ba },
            mirror: None,
        };
        // Same tripwire as depes_execute_module_m: the C ignores the VM
        // return and reads the status byte, but a port-only resource
        // failure must stop the speak loudly, never degrade silently.
        if crate::component_live_c::vm_run_m(
            m,
            ctx,
            u32::from(buffer[index as usize]),
            1,
            end_ptr,
            w,
            &mut expansion,
        )
        .is_err()
        {
            return 0x8970200a;
        }
        index += 1;
    }
    let status = g_byte(m, ctx.wrapping_add(0x37b)).unwrap_or(0);
    if status == 0x9b {
        0x8970200a
    } else if status == 0x9c {
        0x89702000
    } else {
        0
    }
}

/// `fn_2193a4` with the acronym expansion: the engine/tag gate and the
/// two-context fallback of the certified `depes_exec_module_m`, running
/// the expanded module walk above.
pub(crate) fn depes_exec_module_expanded_m<H: Host + ?Sized>(
    m: &mut Memory<'_>,
    engine: u32,
    tag: u32,
    key: u32,
    w: &mut crate::component_live_c::VmWorkspace,
    h: &mut H,
) -> u32 {
    if engine == 0 || tag != 0xf37e {
        return 0x89702008;
    }
    let end_ptr = engine.wrapping_add(0x34);
    let mut result = 0x89702004u32;
    if g_word(m, engine.wrapping_add(0x14)).unwrap_or(0) != 0 {
        result = depes_execute_module_expanded_m(
            m,
            key,
            g_word(m, engine.wrapping_add(0x24)).unwrap_or(0),
            end_ptr,
            w,
            h,
        );
    }
    if g_word(m, engine.wrapping_add(0x10)).unwrap_or(0) != 0 && result == 0x89702004 {
        result = depes_execute_module_expanded_m(
            m,
            key,
            g_word(m, engine.wrapping_add(0x20)).unwrap_or(0),
            end_ptr,
            w,
            h,
        );
    }
    result
}

/// The live broker DataHost for the acronym path's `0x179f30` resource
/// open. Wiring mirrors `engine_live`'s `0x1795f0` arm: the US broker
/// globals over the archive the cartridge filled, with the live heap,
/// database query, file interface and logger riding this same lane-b
/// dispatcher. `real_log_args` is true so the heap compares byte for
/// byte; the varargs chunk service answers 0 like every lane.
struct AcronymBrokerHost<'a, H: Host + ?Sized> {
    host: &'a mut H,
    rom: crate::rom_region::Rom,
    lit_cursor: u32,
}

impl<H: Host + ?Sized> AcronymBrokerHost<'_, H> {
    fn translate(&self, entry: u32) -> u32 {
        crate::rom_region::to_us_guest(self.rom, entry)
    }
    fn forward(&mut self, m: &mut Memory<'_>, f: u32, a: &[u32]) -> u32 {
        let f = self.translate(f);
        self.host.call_m(f, a, m)
    }
}

impl<H: Host + ?Sized> crate::boot_part_b::Host for AcronymBrokerHost<'_, H> {
    fn real_log_args(&mut self) -> bool {
        true
    }
    fn call(&mut self, m: &mut Memory<'_>, f: u32, a: &[u32]) -> u32 {
        match f {
            0xffff0001 => 0,
            _ => self.forward(m, f, a),
        }
    }
    fn thread_id(&mut self) -> u32 {
        0x1ff82000
    }
    fn format_address(&mut self, format: &[u8]) -> Option<u32> {
        let at = (self.lit_cursor + 3) & !3;
        let end = at + format.len() as u32 + 1;
        if end > BROKER_LIT_END {
            return None;
        }
        self.lit_cursor = (end + 3) & !3;
        Some(at)
    }
}

impl<H: Host + ?Sized> crate::boot_part_b::OutputHost for AcronymBrokerHost<'_, H> {
    fn output(&mut self, _m: &mut Memory<'_>, _f: u32, _a: &[u32]) -> (u32, u32) {
        (0, 0)
    }
}

impl<H: Host + ?Sized> crate::boot_part_b::PropertyHost for AcronymBrokerHost<'_, H> {
    fn property(&mut self, _m: &mut Memory<'_>, _p: u32, _k: &[u8]) -> (u32, u32) {
        (0x8060_2001, 0)
    }
    fn insert(&mut self, _m: &mut Memory<'_>, _p: u32, _k: u32, _e: [u32; 3]) -> u32 {
        0x8150_6007
    }
}

impl<H: Host + ?Sized> crate::boot_part_b::QueryHost for AcronymBrokerHost<'_, H> {
    fn query_output(&mut self, m: &mut Memory<'_>, f: u32, a: &[u32]) -> (u32, u32) {
        if f != 0x1772b0 {
            return (0, 0);
        }
        let scratch = BROKER_QUERY_SCRATCH;
        let out = scratch + 12;
        let rc = crate::boot_port_2c::fn_1772b0(m, a[0], a[1], out, scratch, self);
        (rc, g_word(m, out).unwrap_or(0))
    }
    fn insert_record(&mut self, _m: &mut Memory<'_>, _db: u32, _k: u32, _r: [u32; 4]) -> u32 {
        0x8150_6007
    }
    fn open_pair(&mut self, _m: &mut Memory<'_>, _f: u32, _o: [u32; 2], _out: u32) -> u32 {
        0x8150_6007
    }
}

impl<H: Host + ?Sized> crate::boot_part_b::DataHost for AcronymBrokerHost<'_, H> {
    fn data_output(&mut self, m: &mut Memory<'_>, f: u32, a: &[u32], out: u32) -> u32 {
        // 0x173000 is served by the FilePlatform in fn_1795f0_live's
        // DataOpenHost; other file services fail closed.
        let _ = (m, f, a, out);
        0x8060_2001
    }
}

impl<H: Host + ?Sized> crate::component_live_a2::Calls for AcronymBrokerHost<'_, H> {
    fn call(&mut self, m: &mut Memory<'_>, entry: u32, args: &[u32]) -> u32 {
        self.forward(m, entry, args)
    }
}

impl<H: Host + ?Sized> crate::boot_part_e::Host for AcronymBrokerHost<'_, H> {
    fn call(&mut self, m: &mut Memory<'_>, f: u32, a: &[u32]) -> u32 {
        self.forward(m, f, a)
    }
}

impl<H: Host + ?Sized> crate::boot_port_2c::BrokerQuery for AcronymBrokerHost<'_, H> {
    fn db_lock(&mut self, m: &mut Memory<'_>, lock: u32) -> u32 {
        self.forward(m, 0x176f2c, &[lock])
    }
    fn db_unlock(&mut self, m: &mut Memory<'_>, lock: u32) -> u32 {
        self.forward(m, 0x176f5c, &[lock])
    }
    fn db_open(&mut self, m: &mut Memory<'_>, database: u32, name: u32, mode: u32, out: u32) -> u32 {
        crate::component_live_a2::alloc_handle_for_key_m(m, database, name, mode, out, self)
    }
    fn db_next(&mut self, m: &mut Memory<'_>, iterator: u32, key: u32, value: u32) -> u32 {
        crate::engine_components2::fn_196860(m, iterator, key, value)
    }
    fn db_close(&mut self, m: &mut Memory<'_>, iterator: u32) -> u32 {
        crate::component_live_a::release_owner_m(m, iterator, self)
    }
    fn db_compare(&mut self, m: &crate::boot::Memory<'_>, a: u32, b: u32) -> i32 {
        crate::fe_text::strcmp_ci_1769d0(m.bytes, a, b) as i32
    }
}

/// `fn_179f30` over the live engine: the flags-zero broker open of
/// `src/fn_179f30.c` (byte-identical to `leftovers_1::fn_179f30`, which
/// forwards to the same `fn_1795f0` body). The broker/file scratch lives
/// in dead ARM stack below the 2719ec frame; the ROMFS state is the live
/// engine's (the cartridge archive already in the arena).
///
/// Shared with the seg1 scheduler probes (lane us round 7): the EU/KR
/// 1d5ba8 entry states open their system dictionary here while the US
/// entry state arrives with it open, so the regional probes serve the
/// same port through their own region-aware host.
pub fn broker_179f30_live<H: Host + ?Sized>(
    m: &mut Memory<'_>,
    broker: u32,
    name: u32,
    out: u32,
    h: &mut H,
) -> u32 {
    let rom = h.rom();
    let globals = crate::engine_components2::broker_for(rom).data_open(BROKER_DATA_SCRATCH);
    let mut files = crate::engine_components2::live_fs::FilePlatform {
        env: crate::speech_part_1::ArchiveEnvironment {
            dot: crate::rom_region::addr(rom, 0x8cd208),
            parent: crate::rom_region::addr(rom, 0x8cd20c),
            locale: 0,
        },
        scratch: BROKER_FILE_SCRATCH,
        rb: crate::rom_region::addr(rom, 0x1730f8),
    };
    let mut host = AcronymBrokerHost {
        host: h,
        rom,
        lit_cursor: BROKER_LIT_BASE,
    };
    crate::engine_components2::fn_1795f0_live(m, broker, name, 0, out, &globals, &mut files, &mut host)
}

/// `fn_26b7ec` (src/fn_26b7ec.c 1:1): load the dictionary section
/// tables for `key` into the state block. `st` carries ST_TABLES/ST_H0/
/// ST_H1; `resource` is the opened resource handle for the payload map.
/// The map/unmap/compare/strlen/close calls ride the host. The two
/// transient words (map-out, section length) live in dead ARM stack
/// below the 2719ec frame.
const MAP_OUT: u32 = EXPAND_SCRATCH + 0x90;
const MAP_LEN: u32 = EXPAND_SCRATCH + 0x94;
pub fn fn_26b7ec_m<H: Host + ?Sized>(
    m: &mut Memory<'_>,
    h: &mut H,
    st: u32,
    key: u32,
    size: u32,
    resource: u32,
) -> u32 {
    let h0 = g_word(m, st.wrapping_add(0x10)).unwrap_or(0);
    let h1 = g_word(m, st.wrapping_add(0x14)).unwrap_or(0);
    let tables = g_word(m, st.wrapping_add(0x0c)).unwrap_or(0);
    if !p_word(m, MAP_OUT, 0) {
        return FAIL_MEM;
    }
    let mut rc = h.call_m(0x17a1a4, &[h0, h1, 0x28, size, MAP_OUT], m);
    let mut buf = g_word(m, MAP_OUT).unwrap_or(0);
    if rc != 0 {
        h.call_m(0x179f3c, &[h0, h1], m);
        if !p_word(m, st.wrapping_add(0x14), 0) {
            return FAIL_MEM;
        }
        return rc;
    }
    let mut pos = 0u32;
    while pos < size && h.call_m(0x1769d0, &[buf.wrapping_add(pos), key], m) != 0 {
        let mut next = h.call_m(0x195d90, &[buf.wrapping_add(pos)], m).wrapping_add(pos).wrapping_add(1);
        while next & 3 != 0 {
            next = next.wrapping_add(1);
        }
        pos = next.wrapping_add(4);
    }
    if pos >= size {
        h.call_m(0x179208, &[h0, h1, buf], m);
        return 0x89802014;
    }
    pos = pos.wrapping_add(h.call_m(0x195d90, &[buf.wrapping_add(pos)], m)).wrapping_add(1);
    while pos & 3 != 0 {
        pos = pos.wrapping_add(1);
    }
    let offset = g_word(m, buf.wrapping_add(pos)).unwrap_or(0);
    h.call_m(0x179208, &[h0, h1, buf], m);
    if !p_word(m, MAP_OUT, 0) {
        return FAIL_MEM;
    }
    rc = h.call_m(0x17a1a4, &[h0, h1, offset, 4, MAP_OUT], m);
    if rc != 0 {
        h.call_m(0x179f3c, &[h0, h1], m);
        if !p_word(m, st.wrapping_add(0x14), 0) {
            return FAIL_MEM;
        }
        return rc;
    }
    buf = g_word(m, MAP_OUT).unwrap_or(0);
    let length = g_word(m, buf).unwrap_or(0);
    if !p_word(m, MAP_LEN, length) {
        return FAIL_MEM;
    }
    h.call_m(0x179208, &[h0, h1, buf], m);
    rc = h.call_m(0x17a1a4, &[h0, resource, offset, length, tables], m);
    if rc != 0 {
        return rc;
    }
    let base = g_word(m, tables).unwrap_or(0);
    for i in 0..5u32 {
    }
    let rb = |mm: &Memory<'_>, o: u32| g_byte(mm, base.wrapping_add(o)).unwrap_or(0);
    let mut cursor = base.wrapping_add(0x48);
    macro_rules! put {
        ($i:expr, $v:expr) => {
            if !p_word(m, tables.wrapping_add(($i as u32).wrapping_mul(4)), $v) {
                return FAIL_MEM;
            }
        };
    }
    put!(1, cursor);
    let d45 = (i32::from(rb(m, 5)) - i32::from(rb(m, 4)) + 4) & !3;
    cursor = cursor.wrapping_add(d45 as u32);
    put!(2, cursor);
    let d67 = ((i32::from(rb(m, 7)) - i32::from(rb(m, 6))) * 2 + 5) & !3;
    cursor = cursor.wrapping_add(d67 as u32);
    put!(3, cursor);
    let c2e = (u32::from(g_half(m, base.wrapping_add(0x2e)).unwrap_or(0)).wrapping_add(3)) & !3;
    cursor = cursor.wrapping_add(c2e);
    put!(4, cursor);

    cursor = cursor.wrapping_add(u32::from(g_half(m, base.wrapping_add(0x2e)).unwrap_or(0)).wrapping_mul(4));
    put!(5, cursor);

    cursor = cursor.wrapping_add(u32::from(g_half(m, base.wrapping_add(0x2e)).unwrap_or(0)).wrapping_mul(4));
    put!(6, cursor);

    cursor = cursor.wrapping_add(u32::from(g_half(m, base.wrapping_add(0x1a)).unwrap_or(0)).wrapping_mul(4).wrapping_add(4));
    put!(7, cursor);

    cursor = cursor.wrapping_add(u32::from(g_half(m, base.wrapping_add(0x1c)).unwrap_or(0)).wrapping_mul(4).wrapping_add(4));
    put!(8, cursor);

    cursor = cursor.wrapping_add(u32::from(g_half(m, base.wrapping_add(0x1e)).unwrap_or(0)).wrapping_mul(4).wrapping_add(4));
    put!(9, cursor);

    cursor = cursor.wrapping_add(u32::from(g_half(m, base.wrapping_add(0x20)).unwrap_or(0)).wrapping_mul(4).wrapping_add(4));
    put!(10, cursor);

    cursor = cursor.wrapping_add(u32::from(g_half(m, base.wrapping_add(0x22)).unwrap_or(0)).wrapping_mul(4).wrapping_add(4));
    put!(11, cursor);

    let c24 = ((u32::from(g_half(m, base.wrapping_add(0x24)).unwrap_or(0))).wrapping_mul(2).wrapping_add(3)) & !3;
    cursor = cursor.wrapping_add(c24);
    put!(12, cursor);

    let c26 = ((u32::from(g_half(m, base.wrapping_add(0x26)).unwrap_or(0)).wrapping_add(3)) & !3);
    cursor = cursor.wrapping_add(c26);
    put!(13, cursor);

    let c28 = ((u32::from(g_half(m, base.wrapping_add(0x28)).unwrap_or(0)).wrapping_add(3)) & !3);
    cursor = cursor.wrapping_add(c28);
    put!(14, cursor);

    let c2a = ((u32::from(g_half(m, base.wrapping_add(0x2a)).unwrap_or(0))).wrapping_mul(2).wrapping_add(3)) & !3;
    cursor = cursor.wrapping_add(c2a);
    put!(15, cursor);

    let mut end = length.wrapping_add(offset);
    put!(16, end);

    let s3c = ((g_word(m, base.wrapping_add(0x3c)).unwrap_or(0).wrapping_add(3)) & !3);
    end = end.wrapping_add(s3c);
    put!(17, end);

    put!(18, end.wrapping_add((g_word(m, base.wrapping_add(0x40)).unwrap_or(0).wrapping_add(3)) & !3));

    0
}

/// `fn_26bb28` (src/fn_26bb28.c 1:1): release the loaded header through
/// the resource broker. Returns 0; the unmap status is ignored, like the
/// ROM.
pub fn fn_26bb28_m<H: Host + ?Sized>(m: &mut Memory<'_>, h: &mut H, st: u32, resource: u32) -> u32 {
    let h0 = g_word(m, st.wrapping_add(0x10)).unwrap_or(0);
    let tables = g_word(m, st.wrapping_add(0x0c)).unwrap_or(0);
    let buf = g_word(m, tables).unwrap_or(0);
    h.call_m(0x179208, &[h0, resource, buf], m);
    0
}

/// `fn_26bb44` (src/fn_26bb44.c 1:1): the result maximum, a pure read of
/// the header word's +0x30 halfword.
pub fn fn_26bb44_m(m: &Memory<'_>, st: u32) -> u32 {
    u32::from(g_half(m, g_word(m, g_word(m, st.wrapping_add(12)).unwrap_or(0)).unwrap_or(0).wrapping_add(0x30)).unwrap_or(0))
}

/// `fn_1ed0c4` (src/fn_1ed0c4.c 1:1): canonicalize `text` in place
/// through the replacement table. `mode` is the u16 at `obj + 0x9c`;
/// `table` is the 256-entry replacement table or 0 for pure lowercase.
/// Every string/log helper rides the host.
fn fn_1ed0c4_m(
    m: &mut Memory<'_>,
    h: &mut impl Host,
    obj: u32,
    table: u32,
    text: u32,
    cap: u32,
) -> u32 {
    let mode = g_half(m, obj.wrapping_add(0x9c)).unwrap_or(0);
    let owner = g_word(m, obj).unwrap_or(0);
    let logger = g_word(m, owner.wrapping_add(4)).unwrap_or(0);
    let len = h.call_m(0x195d90, &[text], m);
    if mode != 2 && len > 1 {
        let mut length = len as u16;
        let mut i = 0u16;
        while (i as u32) < (length as u32) {
            let b = g_byte(m, text.wrapping_add(u32::from(i))).unwrap_or(0);
            let rep = if table != 0 {
                g_word(m, table.wrapping_add(u32::from(b).wrapping_mul(4))).unwrap_or(0)
            } else {
                0
            };
            if table != 0 && rep != 0 {
                let extra = h.call_m(0x195d90, &[rep], m);
                let total = (length as u32).wrapping_add(extra);
                if (total as u16 as u32) > cap {
                    h.call_m(0x173dc0, &[logger, 0x8f8f44, 0xa029, 0], m);
                    return 0x89802007;
                }
                if !g_range(m, text.wrapping_add(u32::from(i)), extra.wrapping_add(1)) {
                    return FAIL_MEM;
                }
                h.call_m(
                    0x195b38,
                    &[
                        text.wrapping_add(u32::from(i)).wrapping_add(extra),
                        text.wrapping_add(u32::from(i)).wrapping_add(1),
                        (length as u32).wrapping_sub(u32::from(i)),
                    ],
                    m,
                );
                if !f195e38_copy(m, text.wrapping_add(u32::from(i)), rep, extra) {
                    return FAIL_MEM;
                }
                length = total.wrapping_sub(1) as u16;
            } else {
                let folded = h.call_m(0x240b28, &[u32::from(b)], m) as u8;
                if !p_byte(m, text.wrapping_add(u32::from(i)), folded) {
                    return FAIL_MEM;
                }
            }
            i = i.wrapping_add(1);
        }
    }
    0
}

/// `fn_1ed3b0` (src/fn_1ed3b0.c 1:1): the G2P canonical token
/// conversion. Copies the canonical token for the mode string into the
/// caller's buffer. The last-token remember gate (`f29e`) rides the
/// host's `0x1eb978` when set; the dictionary replace rides `fn_1ed0c4`.
fn fn_1ed3b0_m(
    m: &mut Memory<'_>,
    h: &mut impl Host,
    obj: u32,
    tag: u32,
    mode: u32,
    out: u32,
    cap: u32,
) -> u32 {
    let owner = g_word(m, obj).unwrap_or(0);
    let logger = g_word(m, owner.wrapping_add(0x10)).unwrap_or(0);
    let rc = h.call_m(0x176f94, &[obj, tag, 0xf37f, 0x2a4], m);
    if rc >= 0x80000000 {
        return 0x89802008;
    }
    h.call_m(0x1738e8, &[logger, 0x8f8f44, 4, 0, 0x1ed4c0], m);
    if out == 0 {
        h.call_m(0x173dc0, &[logger, 0x8f8f44, 0xa029, 0], m);
        return 0x89802007;
    }
    if g_byte(m, obj.wrapping_add(0x29e)).unwrap_or(0) != 0
        && h.call_m(0x195d58, &[mode, obj.wrapping_add(0x19e)], m) != 0
    {
        let rc = h.call_m(0x1eb978, &[obj, tag, mode], m);
        if rc >= 0x80000000 {
            return rc;
        }
        if !g_strcpy(m, obj.wrapping_add(0x19e), mode) {
            return FAIL_MEM;
        }
    }
    let f90 = g_word(m, obj.wrapping_add(0x90)).unwrap_or(0);
    let f2a0 = g_word(m, obj.wrapping_add(0x2a0)).unwrap_or(0);
    let table = if f2a0 != 0 { f2a0 } else { f90 };
    let rc = fn_1ed0c4_m(m, h, obj, table, out, cap);
    h.call_m(0x1738e8, &[logger, 0x8f8f44, 4, 0, 0x1ed4f0], m);
    rc
}


/// Interception adapter for the acronym path's broker open: serves
/// `0x179f30` through the live broker port, forwards everything else to
/// the caller's lane-b host.
struct BrokerAdapter<'a, H: Host + ?Sized> {
    inner: &'a mut H,
}

impl<H: Host + ?Sized> Host for BrokerAdapter<'_, H> {
    fn call_m(&mut self, entry: u32, args: &[u32], m: &mut Memory<'_>) -> u32 {
        if entry == 0x179f30 && args.len() >= 3 {
            let rc =
                broker_179f30_live(m, args[0], args[1], args[2], &mut *self.inner);
            return rc;
        }
        // Round 4 (seg1): the mapped dictionary load the acronym
        // expansion's 1ea4e0 needs. Ports below (src/fn_26b7ec.c,
        // src/fn_26bb28.c, src/fn_26bb44.c 1:1); every nested call
        // rides the caller's lane-b host.
        if entry == 0x26b7ec && args.len() >= 4 {
            let rc = fn_26b7ec_m(m, &mut *self.inner, args[0], args[1], args[2], args[3]);
            return rc;
        }
        if entry == 0x26bb28 && args.len() >= 2 {
            let rc = fn_26bb28_m(m, &mut *self.inner, args[0], args[1]);
            return rc;
        }
        if entry == 0x26bb44 && args.len() >= 1 {
            return fn_26bb44_m(m, args[0]);
        }
        let rc = self.inner.call_m(entry, args, m);
        if rc >= 0x80000000 {
        }
        rc
    }
    fn rom(&self) -> crate::rom_region::Rom {
        self.inner.rom()
    }
}

/// Interception host for `fn_2719ec`: the rule-module run (`0x2193a4`)
/// executes with the acronym expansion above; the broker open
/// (`0x179f30`) serves through the live broker port. Everything else
/// forwards to the caller's lane-b host. The depes workspace persists
/// across the module runs of one acronym call, like the segment host's.
pub struct XHost<'a, H: Host + ?Sized> {
    pub inner: &'a mut H,
    pub vm: crate::component_live_c::VmWorkspace,
}

impl<H: Host + ?Sized> Host for XHost<'_, H> {
    fn call_m(&mut self, entry: u32, args: &[u32], m: &mut Memory<'_>) -> u32 {
        // fn_1edd3c (seg1 r10): the lexicon delimiter test the G2P
        // single-word path reaches through the lexicon vtable. Direct port
        // below.
        if entry == 0x1edd3c && args.len() >= 3 {
            return f1edd3c_is_delim(m, self, args[0], args[1], args[2]);
        }
        if entry == 0x2193a4 && args.len() >= 3 {
            let (engine, tag, key) = (args[0], args[1], args[2]);
            let rc = depes_exec_module_expanded_m(
                m,
                engine,
                tag,
                key,
                &mut self.vm,
                &mut *self.inner,
            );
            return rc;
        }
        if entry == 0x179f30 && args.len() >= 3 {
            let rc =
                broker_179f30_live(m, args[0], args[1], args[2], &mut *self.inner);
            return rc;
        }
        self.inner.call_m(entry, args, m)
    }
    fn rom(&self) -> crate::rom_region::Rom {
        self.inner.rom()
    }
}
