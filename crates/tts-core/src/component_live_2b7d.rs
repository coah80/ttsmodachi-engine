//! Live ports for the ISTOKEN rule subtree reached from the 1e4b7c number/date
//! validation (seg1 r5): `0x2b8504` (stub) → `0x2b7fdc` (ISTOKEN) → `0x2b7d80`
//! (section search over the lex vtable) + `0x2b79b4` (dictionary word scan),
//! with the buffer helpers `0x2e6008`, `0x24d068`, `0x24cf10`, `0x24d204`,
//! `0x24d2c8`, the canonical-word `0x1ed6c4` (+ `0x1ed520`), and the pure
//! leaves. All 1:1 from `src/fn_*.c`; locals live in Rust locals, guest
//! temporaries go through the live guest heap. No statics, no unsafe.
use crate::boot::Memory;
use crate::component_live_b::Host;

/// Unsigned byte strlen.
fn s_len(m: &Memory<'_>, s: u32) -> u32 {
    let mut n = 0u32;
    loop {
        if m.byte(s.wrapping_add(n)) == 0 {
            break;
        }
        n = n.wrapping_add(1);
    }
    n
}

/// fn_195cd0. strcpy, returns dst.
fn s_cpy(m: &mut Memory<'_>, dst: u32, src: u32) -> u32 {
    let mut i = 0u32;
    loop {
        let c = m.byte(src.wrapping_add(i));
        m.put_byte(dst.wrapping_add(i), c);
        if c == 0 {
            break;
        }
        i = i.wrapping_add(1);
    }
    dst
}

/// fn_195dac. Append at most n bytes of src at the end of the dst string.
fn s_nappend(m: &mut Memory<'_>, dst: u32, src: u32, n: u32) -> u32 {
    let mut d = dst;
    while m.byte(d) != 0 {
        d = d.wrapping_add(1);
    }
    let mut i = 0u32;
    while i < n {
        let c = m.byte(src.wrapping_add(i));
        m.put_byte(d, c);
        if c == 0 {
            return dst;
        }
        d = d.wrapping_add(1);
        i = i.wrapping_add(1);
    }
    m.put_byte(d, 0);
    dst
}

/// fn_195ae8. memcpy of n bytes, returns dst. For every realistic n
/// (n - 1 <= 0x7fffffff, and n == 0 copies nothing) this moves exactly the
/// n bytes; the C's giant-n wrap path is unreached on live inputs.
fn s_copy(m: &mut Memory<'_>, dst: u32, src: u32, n: u32) -> u32 {
    let mut i = 0u32;
    while i < n {
        m.put_byte(dst.wrapping_add(i), m.byte(src.wrapping_add(i)));
        i = i.wrapping_add(1);
    }
    dst
}

/// fn_195df8. memcmp over n bytes.
fn s_cmp(m: &Memory<'_>, a: u32, b: u32, n: u32) -> i32 {
    if n == 0 {
        return 0;
    }
    let mut i = 0u32;
    let mut nn = n;
    loop {
        nn = nn.wrapping_sub(1);
        if nn == 0 {
            break;
        }
        let ca = m.byte(a.wrapping_add(i));
        if ca == 0 || ca != m.byte(b.wrapping_add(i)) {
            break;
        }
        i = i.wrapping_add(1);
    }
    m.byte(a.wrapping_add(i)) as i32 - m.byte(b.wrapping_add(i)) as i32
}

/// fn_1e3eec. Error code for n.
fn e_code(n: u32) -> u32 {
    n | 0x89602000
}

fn h_1963e0(h: &mut impl Host, m: &mut Memory<'_>, heap: u32, ptr: u32, size: u32) -> u32 {
    h.call_m(0x1963e0, &[heap, ptr, size], m)
}
fn h_log(h: &mut impl Host, m: &mut Memory<'_>, log: u32, id: u32, code: u32) -> u32 {
    h.call_m(0x173dc0, &[log, id, code, 0], m)
}

/// fn_2e6008. Range selector: idx 0 reads base, else the indexed entry.
pub fn fn_2e6008_m(
    m: &mut Memory<'_>,
    p: u32,
    out: u32,
    h: &mut impl Host,
) -> u32 {
    let sel = m.word(p.wrapping_add(0x68));
    let idx = m.half(sel.wrapping_add(20)) as u32;
    if idx == 0 {
        let base = m.word(sel.wrapping_add(8));
        m.put_half(out, m.half(base));
        m.put_half(out.wrapping_add(2), m.half(base.wrapping_add(2)));
    } else {
        let tbl = m.word(sel.wrapping_add(12));
        if idx > m.byte(tbl.wrapping_add(0x52)) as u32 {
            let ctx = m.word(p);
            h_log(h, m, m.word(ctx.wrapping_add(16)), m.word(p.wrapping_add(0x48)), 0x5dc8);
            return 0;
        }
        let e = tbl.wrapping_add(0x26).wrapping_add(idx.wrapping_mul(4));
        m.put_half(out, m.half(e));
        m.put_half(out.wrapping_add(2), m.half(e.wrapping_add(2)));
    }
    1
}

fn grow_24d068(
    h: &mut impl Host,
    m: &mut Memory<'_>,
    o: u32,
    b: u32,
    cap: u16,
) -> u32 {
    let ctx = m.word(o);
    let heap = m.word(ctx.wrapping_add(4));
    let data = h_1963e0(h, m, heap, m.word(b), cap as u32 + 1);
    if data == 0 {
        h_log(h, m, m.word(ctx.wrapping_add(16)), m.word(o.wrapping_add(0x48)), 0x5df0);
        return e_code(10);
    }
    m.put_word(b, data);
    m.put_half(b.wrapping_add(4), cap);
    0
}

/// fn_24d068. Append s to buf (8-byte buf: data, cap u16, len u16).
pub fn fn_24d068_m(
    m: &mut Memory<'_>,
    o: u32,
    b: u32,
    s: u32,
    h: &mut impl Host,
) -> u32 {
    if s == 0 || m.byte(s) == 0 {
        return 0;
    }
    let n = s_len(m, s) as u16;
    let cap = m.half(b.wrapping_add(4));
    let len = m.half(b.wrapping_add(6));
    if cap <= len.wrapping_add(n) {
        let r = grow_24d068(h, m, o, b, cap.wrapping_add(n));
        if r >= 0x80000000 {
            return r;
        }
        if m.half(b.wrapping_add(6)) == 0 {
            m.put_byte(m.word(b), 0);
        }
    }
    s_nappend(m, m.word(b), s, n as u32);
    let len2 = m.half(b.wrapping_add(6));
    m.put_half(b.wrapping_add(6), len2.wrapping_add(n));
    0
}

fn grow_24cf10(
    h: &mut impl Host,
    m: &mut Memory<'_>,
    s: u32,
    d: u32,
    n: u16,
) -> u32 {
    let ctx = m.word(s);
    let heap = m.word(ctx.wrapping_add(4));
    let b = h_1963e0(h, m, heap, m.word(d), n as u32 + 1);
    if b == 0 {
        h_log(h, m, m.word(ctx.wrapping_add(16)), m.word(s.wrapping_add(0x48)), 0x5df0);
        return e_code(10);
    }
    m.put_word(d, b);
    m.put_half(d.wrapping_add(4), n);
    0
}

/// fn_24cf10. Grow d to f8 when f4 > f8; clear when non-null.
pub fn fn_24cf10_m(m: &mut Memory<'_>, s: u32, d: u32, h: &mut impl Host) -> u32 {
    if m.half(d.wrapping_add(4)) > m.half(d.wrapping_add(8)) {
        let r = grow_24cf10(h, m, s, d, m.half(d.wrapping_add(8)));
        if r >= 0x80000000 {
            return r;
        }
    }
    if m.word(d) != 0 {
        m.put_byte(m.word(d), 0);
        m.put_half(d.wrapping_add(6), 0);
    }
    0
}

fn grow_24d204(
    h: &mut impl Host,
    m: &mut Memory<'_>,
    p: u32,
    buf: u32,
    need: u32,
) -> u32 {
    let env = m.word(p);
    let heap = m.word(env.wrapping_add(4));
    let mem = h_1963e0(h, m, heap, m.word(buf), need.wrapping_add(1));
    if mem == 0 {
        h_log(h, m, m.word(env.wrapping_add(16)), m.word(p.wrapping_add(0x48)), 0x5df0);
        return e_code(10);
    }
    m.put_word(buf, mem);
    m.put_half(buf.wrapping_add(4), need as u16);
    0
}

/// fn_24d204. Append src[start..end] to buf (12-byte buf here: data, cap,
/// len, max).
pub fn fn_24d204_m(
    m: &mut Memory<'_>,
    p: u32,
    buf: u32,
    start: u16,
    end: u16,
    h: &mut impl Host,
) -> u32 {
    let n = end.wrapping_sub(start) as u32;
    let len = m.half(buf.wrapping_add(6)) as u32;
    let need = len.wrapping_add(n);
    if len.wrapping_add(n) >= m.half(buf.wrapping_add(4)) as u32 {
        let r = grow_24d204(h, m, p, buf, need);
        if r >= 0x80000000 {
            return r;
        }
        if m.half(buf.wrapping_add(6)) == 0 {
            m.put_byte(m.word(buf), 0);
        }
    }
    let src = m.word(p.wrapping_add(0x5c));
    s_nappend(m, m.word(buf), src.wrapping_add(start as u32), n);
    let len2 = m.half(buf.wrapping_add(6));
    m.put_half(buf.wrapping_add(6), len2.wrapping_add(n as u16));
    0
}

fn grow_24d2c8(
    h: &mut impl Host,
    m: &mut Memory<'_>,
    p: u32,
    b: u32,
    cap: u16,
) -> u32 {
    let ctx = m.word(p);
    let heap = m.word(ctx.wrapping_add(4));
    let buf = h_1963e0(h, m, heap, m.word(b), cap as u32 + 1);
    if buf == 0 {
        h_log(h, m, m.word(ctx.wrapping_add(16)), m.word(p.wrapping_add(0x48)), 0x5df0);
        return e_code(10);
    }
    m.put_word(b, buf);
    m.put_half(b.wrapping_add(4), cap);
    0
}

/// fn_24d2c8. Append s plus NUL to buf (8-byte buf: data, cap u16, len u16).
pub fn fn_24d2c8_m(
    m: &mut Memory<'_>,
    p: u32,
    b: u32,
    s: u32,
    h: &mut impl Host,
) -> u32 {
    if s == 0 || m.byte(s) == 0 {
        return 0;
    }
    let n = s_len(m, s);
    let n1 = n.wrapping_add(1);
    let need = (m.half(b.wrapping_add(6)) as u32).wrapping_add(n1);
    if (m.half(b.wrapping_add(4)) as u32) <= need {
        let r = grow_24d2c8(h, m, p, b, need as u16);
        if r >= 0x80000000 {
            return r;
        }
    }
    let len = m.half(b.wrapping_add(6)) as u32;
    s_copy(m, m.word(b).wrapping_add(len), s, n);
    m.put_byte(m.word(b).wrapping_add(len).wrapping_add(n), 0);
    m.put_half(b.wrapping_add(6), (len.wrapping_add(n1)) as u16);
    0
}

/// fn_2b79b4. Dictionary word scan over separator-split fields.
#[allow(clippy::too_many_arguments)]
pub fn fn_2b79b4_m(
    m: &mut Memory<'_>,
    search: u32,
    text: u32,
    separator: u32,
    word_field: u32,
    parse_arguments: u32,
    argument_field: u32,
    domain_field: u32,
    matched: u32,
    h: &mut impl Host,
) -> u32 {
    let owner = m.word(search);
    let context = m.word(search.wrapping_add(0x48));
    let logger = m.word(owner.wrapping_add(0x10));
    let length = s_len(m, text) as u16;
    let mut result = 0u32;
    let mut word_end = 0u32;
    let mut argument_start = 0u32;
    let mut position = 0u32;
    let mut start = 0u32;
    let mut word_start = 0u32;
    let mut argument_end = 0u32;
    let mut domain_start = 0u32;
    let mut domain_end = 0u32;
    let mut last_field = word_field;
    let mut field;
    if argument_field > last_field {
        last_field = argument_field;
    }
    if domain_field > last_field {
        last_field = domain_field;
    }
    m.put_byte(matched, 0);
    while position < length as u32 && m.byte(text.wrapping_add(position)) != separator as u8 {
        position = (position as u16).wrapping_add(1) as u32;
    }
    if m.byte(text.wrapping_add(position)) != separator as u8 {
        h.call_m(0x173dc0, &[logger, context, 0x5dc3, 0], m);
        m.put_byte(matched, 0);
        return result;
    }
    field = 1u32;
    if argument_field == 1 {
        argument_start = start;
        argument_end = position;
    }
    if domain_field == 1 {
        domain_end = position;
        domain_start = start;
    }
    if word_field == 1 {
        word_end = position;
        word_start = start;
    }
    if last_field != 1 {
        loop {
            position = (position as u16).wrapping_add(1) as u32;
            start = position;
            while position < length as u32
                && m.byte(text.wrapping_add(position)) != separator as u8
            {
                position = (position as u16).wrapping_add(1) as u32;
            }
            if start != position && start == length as u32 {
                h.call_m(0x173dc0, &[logger, context, 0x5dc3, 0], m);
                m.put_byte(matched, 0);
                return result;
            }
            field = (field as u8).wrapping_add(1) as u32;
            if field == argument_field {
                argument_start = start;
                argument_end = position;
            }
            if field == domain_field {
                domain_start = start;
                domain_end = position;
            }
            if field == word_field {
                word_start = start;
                word_end = position;
            }
            if field == last_field {
                break;
            }
        }
    }
    let entry = m.word(search.wrapping_add(0x68));
    let search_length = s_len(m, m.word(entry)) as u16;
    let word_length = word_end.wrapping_sub(word_start);
    if word_length == 0
        || word_length != search_length as u32
        || s_cmp(m, text.wrapping_add(word_start), m.word(entry), search_length as u32) != 0
    {
        m.put_byte(matched, 0);
        return result;
    }
    if domain_start != domain_end {
        let domain = m.word(entry.wrapping_add(0x10));
        if s_str(m, text.wrapping_add(domain_start), domain) == 0 {
            let trace = m.byte(search.wrapping_add(0xc6));
            if trace == 1 {
                h.call_m(0x1738e8, &[logger, context, 5, 0], m);
            }
            m.put_byte(matched, 0);
            return result;
        }
    }
    if parse_arguments == 1 {
        if argument_start != argument_end {
            m.put_byte(
                text.wrapping_add(argument_end.wrapping_sub(argument_start)),
                0,
            );
            result = fn_24d068_m(m, search, search.wrapping_add(0xcc), text, h);
            if result >= 0x80000000 {
                return result;
            }
        } else {
            h.call_m(0x173dc0, &[logger, context, 0x5dc4, 0], m);
            m.put_byte(matched, 0);
            return result;
        }
    }
    m.put_byte(matched, 1);
    result
}

/// fn_195d58. strcmp over NUL-terminated bytes.
fn s_strcmp(m: &Memory<'_>, a: u32, b: u32) -> i32 {
    let mut i = 0u32;
    loop {
        let ca = m.byte(a.wrapping_add(i));
        let cb = m.byte(b.wrapping_add(i));
        if ca != cb {
            return if ca < cb { -1 } else { 1 };
        }
        if ca == 0 {
            return 0;
        }
        i = i.wrapping_add(1);
    }
}

/// fn_1960a8. strstr: first occurrence of s2 in s1, or 0.
fn s_str(m: &Memory<'_>, s1: u32, s2: u32) -> u32 {
    if m.byte(s2) == 0 {
        return s1;
    }
    let mut s = s1;
    while m.byte(s) != 0 {
        let mut p1 = s;
        let mut p2 = s2;
        while m.byte(p1) != 0 && m.byte(p2) != 0 && m.byte(p1) == m.byte(p2) {
            p1 = p1.wrapping_add(1);
            p2 = p2.wrapping_add(1);
        }
        if m.byte(p2) == 0 {
            return s;
        }
        s = s.wrapping_add(1);
    }
    0
}

/// Route the lex vtable f48 slot (0x1ec7cc) through the certified lane-b3
/// port (total == 1 is the plain lookup). The ARM keeps the 260-byte
/// scratch on its stack (no 1963b8 in 1ec7cc); mirror with a
/// saved/restored FRAME slice so the heap matches the ARM exactly.
fn route_1ec7cc(
    m: &mut Memory<'_>,
    h: &mut crate::component_live_b2::TokenHost<'_>,
    ctx: u32,
    handle: u32,
    key: u32,
    text: u32,
    out: u32,
    count: u32,
    separator: u32,
    total: u32,
) -> u32 {
    let fb = 0x1600u32;
    let mut sv = [0u32; 65];
    for (i, v) in sv.iter_mut().enumerate() {
        *v = m.word(fb.wrapping_add(i as u32 * 4));
    }
    let scratch = fb;
    let rc = crate::component_live_b3::fn_1ec7cc_m(
        m, h, scratch, ctx, handle, key, text, out, count, separator, total,
    );
    for (i, v) in sv.iter().enumerate() {
        m.put_word(fb.wrapping_add(i as u32 * 4), *v);
    }
    rc
}

/// fn_2b7d80. Section search over the lex vtable: f48 lists candidates
/// (ptr/cnt/flag out), then each candidate runs fn_2b79b4 until term sets.
#[allow(clippy::too_many_arguments)]
pub fn fn_2b7d80_m(
    m: &mut Memory<'_>,
    p: u32,
    a2: u32,
    a3: u32,
    a4: u32,
    a5: u32,
    a6: u32,
    term: u32,
    a8: u32,
    z: u32,
    h: &mut crate::component_live_b2::TokenHost<'_>,
) -> u32 {
    let vt = m.word(p.wrapping_add(0x18));
    let handle = m.word(p.wrapping_add(0x1c));
    let tag = m.word(p.wrapping_add(0x20));
    let text = m.word(p.wrapping_add(0xe4));
    let args = m.word(p.wrapping_add(0x68));
    let domain = m.word(args.wrapping_add(0x10));
    // Guest out-params for the vtable call (ptr, cnt, flag). The ARM keeps
    // them on its stack (no 1963b8 in 2b7d80); mirror with a saved/restored
    // FRAME slice so the heap matches the ARM exactly.
    let fb = 0x1070u32;
    let mut sv = [0u32; 3];
    for (i, v) in sv.iter_mut().enumerate() {
        *v = m.word(fb.wrapping_add(i as u32 * 4));
    }
    let outs = fb;
    m.put_byte(term, 0);
    // The section slots are bound directly (same heap): routing them back
    // through dispatch would lose the heap (TokenHost.heap is the owner,
    // not an allocator).
    let mut rc = route_1ec7cc(
        m, h, handle, tag, a2, text, outs, outs.wrapping_add(4),
        outs.wrapping_add(8), a8,
    );
    if rc >= 0x80000000 {
        for (i, v) in sv.iter().enumerate() {
            m.put_word(fb.wrapping_add(i as u32 * 4), *v);
        }
        return rc;
    }
    let ptr = m.word(outs);
    let mut cnt = m.half(outs.wrapping_add(4)) as u32;
    let flag = m.byte(outs.wrapping_add(8)) as u32;
    if cnt == 0 {
        if z == 1 {
            for (i, v) in sv.iter().enumerate() {
                m.put_word(fb.wrapping_add(i as u32 * 4), *v);
            }
            return rc;
        }
        rc = fn_1ed6c4_m(m, handle, tag, domain, text, h);
        if s_len(m, text) == 0 {
            for (i, v) in sv.iter().enumerate() {
                m.put_word(fb.wrapping_add(i as u32 * 4), *v);
            }
            return rc;
        }
        rc = route_1ec7cc(
            m, h, handle, tag, a2, text, outs, outs.wrapping_add(4),
            outs.wrapping_add(8), a8,
        );
        cnt = m.half(outs.wrapping_add(4)) as u32;
        if (((rc >= 0x80000000) as u32).wrapping_sub(1) & cnt) == 0 || cnt == 0 {
            for (i, v) in sv.iter().enumerate() {
                m.put_word(fb.wrapping_add(i as u32 * 4), *v);
            }
            return rc;
        }
    }
    let mut i = 0u32;
    while m.byte(term) == 0
        && i < cnt
        && {
            rc = fn_2b79b4_m(
                m, p, m.word(ptr.wrapping_add(i.wrapping_mul(4))), flag, a3, a4, a5, a6, term, h,
            );
            rc < 0x80000000
        }
    {
        i = (i as u16).wrapping_add(1) as u32;
    }
    for (i, v) in sv.iter().enumerate() {
        m.put_word(fb.wrapping_add(i as u32 * 4), *v);
    }
    rc
}

/// fn_195e38. Bounded strcpy: copy src into dst, zero-pad to n.
fn s_copyn(m: &mut Memory<'_>, dst: u32, src: u32, n: u32) -> u32 {
    let mut d = dst;
    let mut s = src;
    let mut left = n;
    loop {
        if left == 0 {
            return dst;
        }
        let c = m.byte(s);
        m.put_byte(d, c);
        d = d.wrapping_add(1);
        s = s.wrapping_add(1);
        if c == 0 {
            break;
        }
        left = left.wrapping_sub(1);
    }
    if left > 0x7fffffff {
        left = left.wrapping_sub(1);
        while left != 0 {
            m.put_byte(d, 0);
            d = d.wrapping_add(1);
            left = left.wrapping_sub(1);
        }
    } else {
        let mut i = (left as i32).wrapping_sub(1);
        if i > 0 {
            while i != 0 {
                m.put_byte(d, 0);
                d = d.wrapping_add(1);
                i -= 1;
            }
        }
    }
    dst
}

/// fn_195d30. strchr: first occurrence of c in s, or 0.
fn s_chr(m: &Memory<'_>, s: u32, c: u32) -> u32 {
    let c = (c & 0xff) as u8;
    let mut p = s;
    loop {
        let b = m.byte(p);
        if b == 0 || b == c {
            break;
        }
        p = p.wrapping_add(1);
    }
    if m.byte(p) == c {
        p
    } else {
        0
    }
}

/// fn_240b28. CP437 tolower (ASCII A-Z plus accented Latin).
fn s_tolower(c: u32) -> u32 {
    if c.wrapping_sub(0x41) <= 0x19 {
        return (c + 0x20) & 0xff;
    }
    match c {
        0x80 => 0x87,
        0x8e => 0x84,
        0x8f => 0x86,
        0x90 => 0x82,
        0x92 => 0x91,
        0x99 => 0x94,
        0x9a => 0x81,
        0xa5 => 0xa4,
        _ => c,
    }
}

/// fn_195b38. memmove, returns dst.
fn s_move(m: &mut Memory<'_>, dst: u32, src: u32, n: u32) -> u32 {
    if dst > src && src.wrapping_add(n) > dst {
        let mut s = src.wrapping_add(n);
        let mut d = dst.wrapping_add(n);
        let mut k = n;
        while k != 0 {
            k -= 1;
            s -= 1;
            d -= 1;
            m.put_byte(d, m.byte(s));
        }
    } else {
        let mut s = src;
        let mut d = dst;
        let mut k = n;
        while k != 0 {
            k -= 1;
            m.put_byte(d, m.byte(s));
            d += 1;
            s += 1;
        }
    }
    dst
}

/// fn_1ed520. Replacement + case pass over text (capacity-checked).
pub fn fn_1ed520_m(
    m: &mut Memory<'_>,
    ctx: u32,
    replacements: u32,
    text: u32,
    capacity: u32,
    h: &mut impl Host,
) -> u32 {
    let owner = m.word(ctx);
    let char_class = m.word(ctx.wrapping_add(0x64));
    let mode = m.half(ctx.wrapping_add(0x9c));
    let mut start = 0u16;
    while (start as u32) < s_len(m, text) && s_chr(m, char_class, m.byte(text.wrapping_add(start as u32)) as u32) != 0 {
        start = start.wrapping_add(1);
    }
    let mut j = (s_len(m, text) as u32).wrapping_sub(1);
    while (j as i32) > 0 && s_chr(m, char_class, m.byte(text.wrapping_add(j)) as u32) != 0 {
        j = j.wrapping_sub(1);
    }
    m.put_byte(text.wrapping_add((j as u16).wrapping_add(1) as u32), 0);
    if mode != 2 && (s_len(m, text) as u16).wrapping_sub(start) > 1 {
        let mut end = s_len(m, text);
        let mut i = start as u32;
        while i < end {
            let rep = if replacements != 0 {
                m.word(replacements.wrapping_add(m.byte(text.wrapping_add(i)) as u32 * 4))
            } else {
                0
            };
            if rep != 0 {
                let extra = s_len(m, rep);
                let total = end.wrapping_add(extra);
                if total as u16 > capacity as u16 {
                    h.call_m(0x173dc0, &[m.word(owner), 0x8f8f44u32, 0xa029, 0], m);
                    return 0x89802007;
                }
                s_move(m, text.wrapping_add(i).wrapping_add(extra), text.wrapping_add(i).wrapping_add(1), end.wrapping_sub(i));
                s_copyn(m, text.wrapping_add(i), rep, extra);
                end = total.wrapping_sub(1);
            } else {
                let c = m.byte(text.wrapping_add(i));
                m.put_byte(text.wrapping_add(i), s_tolower(c as u32) as u8);
            }
            i = i.wrapping_add(1);
        }
    }
    let sl = s_len(m, text.wrapping_add(start as u32));
    s_move(m, text, text.wrapping_add(start as u32), sl.wrapping_add(1));
    0
}

/// fn_1ed6c4. fe_dctlkp canonical word: validate, maybe cached-refresh,
/// then the mode pass (0 = replacement table, else trim + case).
pub fn fn_1ed6c4_m(
    m: &mut Memory<'_>,
    ctx: u32,
    handle: u32,
    key: u32,
    text: u32,
    h: &mut impl Host,
) -> u32 {
    let owner = m.word(ctx);
    let logger = m.word(owner.wrapping_add(16));
    let mut result = h.call_m(0x176f94, &[ctx, handle, 0xf37f, 0x2a4], m);
    if result >= 0x80000000 {
        return 0x89802008;
    }
    h.call_m(0x1738e8, &[logger, 0x8f8f44, 4, 0, 0], m);
    if text == 0 {
        h.call_m(0x173dc0, &[logger, 0x8f8f44, 0xa029, 0], m);
        return 0x89802007;
    }
    if s_len(m, text) == 0 {
        return result;
    }
    if m.byte(ctx.wrapping_add(0x29e)) != 0 {
        if s_strcmp(m, key, ctx.wrapping_add(0x19e)) == 0 {
            result = h.call_m(0x1eb978, &[ctx, handle, key], m);
            if result >= 0x80000000 {
                return result;
            }
            s_cpy(m, ctx.wrapping_add(0x19e), key);
        }
    }
    if m.half(ctx.wrapping_add(0x9c)) == 0 {
        result = fn_1ed520_m(
            m,
            ctx,
            m.word(ctx.wrapping_add(0x90)),
            text,
            s_len(m, text).wrapping_add(1),
            h,
        );
        if result >= 0x80000000 {
            return result;
        }
    } else {
        let char_class = m.word(ctx.wrapping_add(0x64));
        let mut start = 0u16;
        while (start as u32) < s_len(m, text)
            && s_chr(m, char_class, m.byte(text.wrapping_add(start as u32)) as u32) != 0
        {
            start = start.wrapping_add(1);
        }
        let mut end = (s_len(m, text) as u32).wrapping_sub(1);
        while (end as i32) > 0
            && s_chr(m, char_class, m.byte(text.wrapping_add(end)) as u32) != 0
        {
            end = end.wrapping_sub(1);
        }
        m.put_byte(text.wrapping_add((end as u16).wrapping_add(1) as u32), 0);
        if m.half(ctx.wrapping_add(0x9c)) != 2
            && (s_len(m, text) as u16).wrapping_sub(start) > 1
        {
            let mut i = start as u32;
            while i < s_len(m, text) {
                let c = m.byte(text.wrapping_add(i));
                m.put_byte(text.wrapping_add(i), s_tolower(c as u32) as u8);
                i = i.wrapping_add(1);
            }
        }
        let sl = s_len(m, text.wrapping_add(start as u32));
        s_move(m, text, text.wrapping_add(start as u32), sl.wrapping_add(1));
    }
    h.call_m(0x1738e8, &[logger, 0x8f8f44, 4, 0, 1], m);
    result
}

/// fn_2b7fdc. ISTOKEN rule: range-select, dictionary search over two
/// sections, table add on a term hit, TRUE/FALSE appended to output.
#[allow(clippy::too_many_arguments)]
pub fn fn_2b7fdc_m(
    m: &mut Memory<'_>,
    p: u32,
    out_flag: u32,
    z: u32,
    h: &mut crate::component_live_b2::TokenHost<'_>,
) -> u32 {
    let ctx = m.word(p);
    let log_id = m.word(p.wrapping_add(0x48));
    let log = m.word(ctx.wrapping_add(16));
    let args = m.word(p.wrapping_add(0x68));
    let trace = m.byte(p.wrapping_add(0xc6));
    let mut flag = 0u32;
    // Range + term/anchor live in Rust locals (the C frame). The ARM keeps
    // term, scratch and the 2e6008 pair on its own stack (no 1963b8 in
    // 2b7fdc); mirror them with a saved/restored FRAME slice so the heap
    // matches the ARM exactly.
    let fb = 0x1060u32;
    let mut sv = [0u32; 3];
    for (i, v) in sv.iter_mut().enumerate() {
        *v = m.word(fb.wrapping_add(i as u32 * 4));
    }
    let mut range = [0u16; 2];
    let termh = fb;
    let scratch = fb.wrapping_add(4);
    let rpair = fb.wrapping_add(8);
    m.put_byte(termh, 0);
    let r = fn_2b7fdc_inner(m, p, out_flag, z, h, ctx, log_id, log, args, trace, &mut flag, &mut range, termh, scratch, rpair);
    for (i, v) in sv.iter().enumerate() {
        m.put_word(fb.wrapping_add(i as u32 * 4), *v);
    }
    r
}

#[allow(clippy::too_many_arguments)]
fn fn_2b7fdc_inner(
    m: &mut Memory<'_>,
    p: u32,
    out_flag: u32,
    z: u32,
    h: &mut crate::component_live_b2::TokenHost<'_>,
    ctx: u32,
    log_id: u32,
    log: u32,
    args: u32,
    trace: u8,
    flag: &mut u32,
    range: &mut [u16; 2],
    term: u32,
    scratch: u32,
    rpair: u32,
) -> u32 {
    let _ = ctx;
    let output = m.word(args.wrapping_add(4));
    let out = [range[0] as u32, range[1] as u32];
    let _ = out;
    let mut range_w = [0u32; 1];
    let _ = range_w;
    // 2e6008 writes (start s16, end s16) into the caller FRAME pair
    // (rpair), like the ARM stack slot.
    let mut r = fn_2e6008_m(m, p, rpair, h);
    range[0] = m.half(rpair) as u16;
    range[1] = m.half(rpair.wrapping_add(2)) as u16;
    if r == 0 {
        r = fn_24d068_m(m, p, output, 0x2b82f8, h);
        if r >= 0x80000000 {
            return r;
        }
    } else if range[0] == 0xffff && range[1] == 0xffff {
        if trace == 1 {
            h.call_m(0x1738e8, &[log, log_id, 5, 0, 2], m);
        }
        r = fn_24d068_m(m, p, output, 0x2b8310, h);
        if r >= 0x80000000 {
            return r;
        }
    } else {
        let buf_d8 = p.wrapping_add(0xd8);
        let buf_e4 = p.wrapping_add(0xe4);
        let mut rc = fn_24cf10_m(m, p, buf_d8, h);
        if rc >= 0x80000000 {
            return e_code(8);
        }
        rc = fn_24cf10_m(m, p, buf_e4, h);
        if rc >= 0x80000000 {
            return e_code(8);
        }
        rc = fn_24d204_m(m, p, buf_e4, range[0], range[1], h);
        if rc >= 0x80000000 {
            return rc;
        }
        rc = fn_24d204_m(m, p, buf_d8, range[0], range[1], h);
        if rc >= 0x80000000 {
            return rc;
        }
        if trace == 1 {
            h.call_m(0x1738e8, &[log, log_id, 5, 0, 3], m);
        }
        m.put_half(p.wrapping_add(0xd2), 0);
        // ROM 2b7fdc+0x1d0..0x1e0: NUL the +0xcc buffer as well as its
        // length (the dictionary scan appends into it next). Without the
        // NUL, appends accumulate past the length and corrupt the heap.
        let ccbuf = m.word(p.wrapping_add(0xcc));
        if ccbuf != 0 {
            m.put_byte(ccbuf, 0);
        }
        m.put_byte(scratch, 0);
        m.put_byte(term, 0);
        let table = m.word(args.wrapping_add(12));
        let index = m.half(args.wrapping_add(20)) as u32;
        let domain = m.word(args.wrapping_add(16));
        let key_f1 = m.byte(p.wrapping_add(0xf1));
        let key_f0 = m.byte(p.wrapping_add(0xf0));
        rc = fn_2b7d80_m(m, p, 0x9b2593, 1, 0, 0, 2, term, 1, z, h);
        if rc >= 0x80000000 {
            return check_2b7fdc(m, p, output, term, flag, table, index, h, rc);
        }
        if m.byte(term) == 1 {
            return check_2b7fdc(m, p, output, term, flag, table, index, h, rc);
        }
        let d8data = m.word(buf_d8);
        let e4data = m.word(buf_e4);
        s_cpy(m, e4data, d8data);
        rc = fn_2b7d80_m(m, p, 0x9b258c, 2, 1, 1, 3, term, key_f1 as u32, z, h);
        if rc >= 0x80000000 {
            return check_2b7fdc(m, p, output, term, flag, table, index, h, rc);
        }
        if m.byte(term) == 1 {
            *flag = 1;
        } else {
            let d8data = m.word(buf_d8);
            let e4data = m.word(buf_e4);
            s_cpy(m, e4data, d8data);
            rc = fn_2b7d80_m(m, p, 0x9b258f, 2, 0, 0, 3, term, key_f0 as u32, z, h);
        }
        return check_2b7fdc(m, p, output, term, flag, table, index, h, rc);
    }
    if trace == 1 {
        h.call_m(0x1738e8, &[log, log_id, 5, 0, 4], m);
    }
    m.put_byte(out_flag, r as u8);
    r
}

/// Tail of fn_2b7fdc after the section searches (check_r).
#[allow(clippy::too_many_arguments)]
fn check_2b7fdc(
    m: &mut Memory<'_>,
    p: u32,
    output: u32,
    term: u32,
    flag: &mut u32,
    table: u32,
    index: u32,
    h: &mut crate::component_live_b2::TokenHost<'_>,
    rc: u32,
) -> u32 {
    if rc >= 0x80000000 {
        return rc;
    }
    let r;
    if m.byte(term) == 0 {
        r = fn_24d068_m(m, p, output, 0x2b82f8, h);
        if r >= 0x80000000 {
            return r;
        }
    } else if m.byte(term) == 1 {
        if *flag == 1 {
            *flag = 0;
            // 1:1 table update (src/fn_2b7fdc.c): entries[count] = {index,
            // bufAC.len}, then 24d2c8, then count++ on success. The table
            // may live on the guest stack (as on ARM), leaving the
            // leftovers a later fn_2b6afc Match reuses.
            let count = m.half(table.wrapping_add(40)) as u32;
            m.put_half(table.wrapping_add(count.wrapping_mul(4)), index as u16);
            m.put_half(
                table.wrapping_add(count.wrapping_mul(4)).wrapping_add(2),
                m.half(p.wrapping_add(178)),
            );
            let sc = m.word(p.wrapping_add(0xcc));
            let rc2 = fn_24d2c8_m(m, p, p.wrapping_add(0xac), sc, h);
            if rc2 < 0x80000000 {
                *flag = m.byte(term) as u32;
                m.put_half(table.wrapping_add(40), count.wrapping_add(1) as u16);
            }
            if rc2 >= 0x80000000 {
                return rc2;
            }
            if *flag == 1 {
                r = fn_24d068_m(m, p, output, 0x2b8310, h);
                if r >= 0x80000000 {
                    return r;
                }
            } else {
                r = fn_24d068_m(m, p, output, 0x2b82f8, h);
                if r >= 0x80000000 {
                    return r;
                }
            }
        } else {
            r = fn_24d068_m(m, p, output, 0x2b8310, h);
            if r >= 0x80000000 {
                return r;
            }
        }
    } else {
        r = fn_24d068_m(m, p, output, 0x2b82f8, h);
        if r >= 0x80000000 {
            return r;
        }
    }
    r
}

/// fn_2b8504. Stub: tail-calls fn_2b7fdc with z = 0.
pub fn fn_2b8504_m(
    m: &mut Memory<'_>,
    p: u32,
    flag: u32,
    h: &mut crate::component_live_b2::TokenHost<'_>,
) -> u32 {
    fn_2b7fdc_m(m, p, flag, 0, h)
}

/// fn_2e5ec4. Split str on spaces into a/b (two words exactly); 1/0.
pub fn fn_2e5ec4_m(
    m: &mut Memory<'_>,
    p: u32,
    a: u32,
    b: u32,
    h: &mut impl Host,
) -> u32 {
    let ctx = m.word(p);
    let id = m.word(p.wrapping_add(0x48));
    let log = m.word(ctx.wrapping_add(16));
    let strp = m.word(p.wrapping_add(0x68));
    let s = m.word(strp);
    let n = s_len(m, s) as u16;
    let mut i = 0u16;
    while (i as u32) < n as u32 && m.byte(s.wrapping_add(i as u32)) != b' ' {
        i = i.wrapping_add(1);
    }
    if i == n {
        h.call_m(0x173dc0, &[log, id, 0x5dc7, 0], m);
        return 0;
    }
    s_copyn(m, a, s, i as u32);
    m.put_byte(a.wrapping_add(i as u32), 0);
    let mut j = i.wrapping_add(1);
    while (j as u32) < n as u32 && m.byte(s.wrapping_add(j as u32)) == b' ' {
        j = j.wrapping_add(1);
    }
    let start = j;
    if start == n {
        h.call_m(0x173dc0, &[log, id, 0x5dc8, 0], m);
        return 0;
    }
    j = start;
    while (j as u32) < n as u32 && m.byte(s.wrapping_add(j as u32)) != b' ' {
        j = j.wrapping_add(1);
    }
    if j != n {
        h.call_m(0x173dc0, &[log, id, 0x5dc8, 0], m);
        return 0;
    }
    s_copyn(m, b, s.wrapping_add(start as u32), (n as u32).wrapping_sub(start as u32));
    m.put_byte(b.wrapping_add((n as u32).wrapping_sub(start as u32)), 0);
    1
}

/// fn_2b73d0. _AND_ rule combinator: split, compare both halves to TRUE,
/// append TRUE/FALSE to the output list. Pool strings: TRUE at 0x2b750c,
/// FALSE at 0x2b7514 (verified against the ROM pool).
pub fn fn_2b73d0_m(
    m: &mut Memory<'_>,
    p: u32,
    out: u32,
    h: &mut impl Host,
) -> u32 {
    fn_2b73d0_8868_m(m, p, out, h, false, 0x2b750c, 0x2b7514)
}

/// fn_2b8868. _OR_ rule combinator: same shape with OR logic. Pool
/// strings: TRUE at 0x2b89a8, FALSE at 0x2b89b0 (verified).
pub fn fn_2b8868_m(
    m: &mut Memory<'_>,
    p: u32,
    out: u32,
    h: &mut impl Host,
) -> u32 {
    fn_2b73d0_8868_m(m, p, out, h, true, 0x2b89a8, 0x2b89b0)
}

fn fn_2b73d0_8868_m(
    m: &mut Memory<'_>,
    p: u32,
    out: u32,
    h: &mut impl Host,
    is_or: bool,
    s_true: u32,
    s_false: u32,
) -> u32 {
    let log_id = m.word(p.wrapping_add(0x48));
    let sub = m.word(p);
    let log = m.word(sub.wrapping_add(16));
    let args = m.word(p.wrapping_add(0x68));
    let trace = m.byte(p.wrapping_add(0xc6));
    if trace == 1 {
        h.call_m(0x1738e8, &[log, log_id, 5, 0, 0], m);
    }
    m.put_byte(out, 0);
    // The ARM keeps the two split words on its stack (no 1963b8 in 2b73d0
    // or 2b8868); mirror with a saved/restored FRAME slice so the heap
    // matches the ARM exactly.
    let fb = 0x1a00u32;
    let mut sv = [0u32; 16];
    for (i, v) in sv.iter_mut().enumerate() {
        *v = m.word(fb.wrapping_add(i as u32 * 4));
    }
    let ga = fb;
    let gb = fb.wrapping_add(32);
    let mut r;
    if fn_2e5ec4_m(m, p, ga, gb, h) == 0 {
        m.put_byte(out, 0);
        r = 0;
    } else {
        let a_true = s_strcmp(m, ga, s_true) == 0;
        let b_true = s_strcmp(m, gb, s_true) == 0;
        let c = if (a_true && b_true) || (is_or && (a_true || b_true)) {
            1
        } else {
            0
        };
        m.put_byte(out, c);
        r = c as u32;
    }
    // vals is used as the Buf* directly (its first word is the data).
    let vals = m.word(args.wrapping_add(4));
    if m.byte(out) == 1 {
        r = fn_24d068_m(m, p, vals, s_true, h);
    } else {
        r = fn_24d068_m(m, p, vals, s_false, h);
    }
    if r >= 0x80000000 {
        for (i, v) in sv.iter().enumerate() {
            m.put_word(fb.wrapping_add(i as u32 * 4), *v);
        }
        return r;
    }
    if trace == 1 {
        h.call_m(0x1738e8, &[log, log_id, 5, 0, 1], m);
    }
    for (i, v) in sv.iter().enumerate() {
        m.put_word(fb.wrapping_add(i as u32 * 4), *v);
    }
    r
}

/// fn_2b5c18. _STRING_TTS check: append args->arg0 to the arg1 buffer.
/// 1:1 from src/fn_2b5c18.c (offsets +0x48/+0x64/+0xc6 verified against
/// the ROM disassembly; formats 0x2b5cd0/0x2b5ce8 from its pool).
pub fn fn_2b5c18_m(
    m: &mut Memory<'_>,
    p: u32,
    out: u32,
    h: &mut impl Host,
) -> u32 {
    let ctx = m.word(p);
    let logobj = m.word(p.wrapping_add(0x48));
    let args = m.word(p.wrapping_add(0x64));
    let trace = m.byte(p.wrapping_add(0xc6));
    let log = m.word(ctx.wrapping_add(16));
    m.put_byte(out, 0);
    if trace == 1 {
        h.call_m(0x1738e8, &[log, logobj, 5, 0, 0x2b5cd0, m.word(args)], m);
    }
    let r = fn_24d068_m(m, p, m.word(args.wrapping_add(4)), m.word(args), h);
    if r >= 0x80000000 {
        return r;
    }
    if trace == 1 {
        h.call_m(
            0x1738e8,
            &[log, logobj, 5, 0, 0x2b5ce8, m.word(m.word(args.wrapping_add(4)))],
            m,
        );
    }
    m.put_byte(out, 1);
    r
}

/// fn_24ce90. Clear the two marker halves at +0xbc/+0xc4.
pub fn fn_24ce90_m(m: &mut Memory<'_>, p: u32) {
    m.put_half(p.wrapping_add(0xbc), 0);
    m.put_half(p.wrapping_add(0xc4), 0);
}

/// fn_24e948. Sorted-list lookup by key; out-params receive guest
/// addresses of the matching key/value words. Returns 0.
pub fn fn_24e948_m(
    m: &mut Memory<'_>,
    p: u32,
    key: i32,
    key_out: u32,
    val_out: u32,
) -> u32 {
    let list = m.word(p.wrapping_add(68));
    if m.half(list.wrapping_add(8)) != 0 {
        let mut e = m.word(list.wrapping_add(4));
        if e != 0 {
            loop {
                let ek = m.half(e.wrapping_add(8)) as i16 as i32;
                if ek == key {
                    m.put_word(key_out, e.wrapping_add(8));
                    m.put_word(val_out, e.wrapping_add(12));
                    return 0;
                }
                if ek > key {
                    return 0;
                }
                e = m.word(e);
                if e == 0 {
                    break;
                }
            }
        }
    }
    0
}

/// fn_24ee58. Error code for flags (1e3eec).
fn fn_24ee58_m(flags: u32) -> u32 {
    e_code(flags)
}

/// fn_2511ac. Table entry lookup by the guest (f2, f4) index triple.
pub fn fn_2511ac_m(
    m: &mut Memory<'_>,
    p: u32,
    idx: u32,
    out: u32,
    out_len: u32,
    out_ptr: u32,
    h: &mut impl Host,
) -> u32 {
    let ctx = m.word(p);
    let log = m.word(ctx.wrapping_add(16));
    let id = m.word(p.wrapping_add(0x48));
    let list = m.word(m.word(p.wrapping_add(0x38)).wrapping_add(8));
    if list == 0 {
        h.call_m(0x173dc0, &[log, id, 0x5df1, 0], m);
        return fn_24ee58_m(7);
    }
    let idx2 = m.half(idx.wrapping_add(2)) as u32;
    let idx4 = m.half(idx.wrapping_add(4)) as u32;
    // ROM: A = list->f4; G = A[idx2]; t = G->f4 (three dereferences).
    let arr = m.word(list.wrapping_add(4));
    let g = m.word(arr.wrapping_add(idx2.wrapping_mul(4)));
    let t = m.word(g.wrapping_add(4));
    let e = m.word(t.wrapping_add(0x18)).wrapping_add(idx4.wrapping_mul(24));
    // ROM: out = e->f2 + t->fc (+0xc), out_ptr = e->fc (+0xc) or 0.
    m.put_word(out, m.half(e.wrapping_add(2)) as u32 + m.word(t.wrapping_add(0x0c)));
    m.put_half(out_len, m.half(e.wrapping_add(0x12)));
    let f12 = m.half(e.wrapping_add(0x12));
    m.put_word(out_ptr, if f12 == 0 { 0 } else { m.word(e.wrapping_add(0x0c)) });
    0
}

/// fn_2b4968. DoExpansions driver: scan text for ranges, look up each
/// expansion, apply it via 2b4260, insert the token, delete subsumed tokens.
pub fn fn_2b4968_m(
    m: &mut Memory<'_>,
    owner: u32,
    created: u32,
    h: &mut crate::component_live_b2::TokenHost<'_>,
) -> u32 {
    let ctx = m.word(owner);
    let log = m.word(ctx.wrapping_add(16));
    let log_tag = m.word(owner.wrapping_add(0x48));
    let text = m.word(owner.wrapping_add(0x5c));
    let text_length = m.half(owner.wrapping_add(0x60));
    let state = m.word(owner.wrapping_add(0x64));
    let debug = m.byte(owner.wrapping_add(0xc6));
    h.call_m(0x1738e8, &[log, log_tag, 4, 0, 6], m);
    if text == 0 {
        return e_code(7);
    }
    if debug == 1 {
        let r = h.call_m(0x24ac00, &[owner, 0x2b4c98, 0], m);
        let _ = r;
    }
    // Guest out-words for the range/token lookup and the 2511ac triple.
    // ARM keeps these 6 words on its stack (no 1963b8 in 2b4968); mirror
    // that with a saved/restored FRAME slice so the heap matches the ARM.
    let fb_out = 0x1900u32;
    let mut sv_out = [0u32; 6];
    for (i, v) in sv_out.iter_mut().enumerate() {
        *v = m.word(fb_out.wrapping_add(i as u32 * 4));
    }
    let outs = fb_out;
    let rp = outs;
    let tp = outs.wrapping_add(4);
    let iname = outs.wrapping_add(8);
    let ilen = outs.wrapping_add(12);
    let iexp = outs.wrapping_add(16);
    let afterp = outs.wrapping_add(20);
    let mut result = 0u32;
    let mut cursor = 0u16;
    m.put_half(created, 0);
    while (cursor as u32) < text_length as u32 {
        let range: u32;
        let token: u32;
        result = fn_24e948_m(m, owner, cursor as i32, rp, tp);
        range = m.word(rp);
        token = m.word(tp);
        if range == 0 || m.half(range) as u32 != cursor as u32 {
            cursor = cursor.wrapping_add(1);
            continue;
        }
        let rstart = m.half(range) as u32;
        let rend = m.half(range.wrapping_add(2)) as u32;
        result = fn_2511ac_m(
            m, owner,
            m.word(token.wrapping_add(0x54)),
            iname, ilen, iexp, h,
        );
        if result >= 0x80000000 {
            break;
        }
        let expansion = m.word(iexp);
        let mut input_words = 1u16;
        let mut i = rstart as u16;
        {
            let end = rend;
            while (end as u16) > i && m.byte(text.wrapping_add(i as u32)) == b' ' {
                i = i.wrapping_add(1);
            }
            while (end as u16) > i {
                if m.byte(text.wrapping_add(i as u32)) == b' ' {
                    input_words = input_words.wrapping_add(1);
                    i = i.wrapping_add(1);
                    while (end as u16) > i && m.byte(text.wrapping_add(i as u32)) == b' ' {
                        i = i.wrapping_add(1);
                    }
                } else {
                    i = i.wrapping_add(1);
                }
            }
        }
        fn_24ce90_m(m, owner);
        i = rstart as u16;
        while (i as u32) < rend && m.byte(text.wrapping_add(i as u32)) == b' ' {
            i = i.wrapping_add(1);
        }
        let bc = m.half(owner.wrapping_add(0xc4)) as u32;
        let bds = m.word(owner.wrapping_add(0xc0));
        m.put_half(bds.wrapping_add(bc.wrapping_mul(2)), i);
        m.put_half(owner.wrapping_add(0xc4), (bc as u16).wrapping_add(1));
        let mut j = rstart as u16;
        while (j as u32) < rend {
            if m.byte(text.wrapping_add(j as u32)) == b' ' {
                while (j as u32) < rend && m.byte(text.wrapping_add(j as u32)) == b' ' {
                    j = j.wrapping_add(1);
                }
                let bc2 = m.half(owner.wrapping_add(0xc4)) as u32;
                m.put_half(bds.wrapping_add(bc2.wrapping_mul(2)), j);
                m.put_half(owner.wrapping_add(0xc4), (bc2 as u16).wrapping_add(1));
            } else {
                j = j.wrapping_add(1);
            }
        }
        if debug == 1 {
            h.call_m(0x1738e8, &[log, log_tag, 5, 0, 7], m);
        }
        if expansion == 0 {
            h.call_m(0x173dc0, &[log, log_tag, 0x5dcf, 0], m);
            result = e_code(0x10a);
            break;
        }
        // positions[*] = 9999; clear expanded/alignment; set state.
        let npos = m.half(owner.wrapping_add(0xc4)) as u32;
        let poss = m.word(owner.wrapping_add(0xb8));
        for k in 0..npos {
            m.put_half(poss.wrapping_add(k.wrapping_mul(2)), 9999);
        }
        let expb = owner.wrapping_add(0x88);
        let alnb = owner.wrapping_add(0x94);
        m.put_half(expb.wrapping_add(6), 0);
        m.put_byte(m.word(expb), 0);
        m.put_half(alnb.wrapping_add(6), 0);
        m.put_byte(m.word(alnb), 0);
        m.put_word(state.wrapping_add(12), token);
        m.put_word(state.wrapping_add(8), range);
        m.put_half(state.wrapping_add(16), 0);
        result = h.call_m(0x2b4260, &[owner, expansion, expb, alnb], m);
        if result >= 0x80000000 {
            break;
        }
        if m.half(expb.wrapping_add(6)) == 0 {
            h.call_m(0x173dc0, &[log, log_tag, 0x5dd0, 0], m);
            result = e_code(0x10a);
            break;
        }
        h.call_m(0x1738e8, &[log, log_tag, 5, 0, 8], m);
        h.call_m(0x1738e8, &[log, log_tag, 5, 0, 9], m);
        let mut output_length = s_len(m, m.word(expb));
        let mut output_words = 1u16;
        i = 0;
        while (i as u32) < output_length && m.byte(m.word(expb).wrapping_add(i as u32)) == b' ' {
            i = i.wrapping_add(1);
        }
        while (i as u32) < output_length {
            if m.byte(m.word(expb).wrapping_add(i as u32)) == b' ' {
                output_words = output_words.wrapping_add(1);
                i = i.wrapping_add(1);
                while (i as u32) < rend && m.byte(m.word(expb).wrapping_add(i as u32)) == b' ' {
                    i = i.wrapping_add(1);
                }
            } else {
                i = i.wrapping_add(1);
            }
        }
        let mut invalid = 0u8;
        if output_words != input_words {
            invalid = 1;
            h.call_m(0x1738e8, &[log, log_tag, 5, 0, 10], m);
        } else if input_words > 1 {
            for q in 0..npos {
                if m.half(poss.wrapping_add(q.wrapping_mul(2))) == 9999 {
                    i = 0;
                    while (i as u32) < output_length && m.byte(m.word(expb).wrapping_add(i as u32)) == b' ' {
                        i = i.wrapping_add(1);
                    }
                    output_words = 0;
                    if q != 0 {
                        i = 0;
                        while (i as u32) < output_length {
                            if m.byte(m.word(expb).wrapping_add(i as u32)) == b' ' {
                                output_words = output_words.wrapping_add(1);
                                if output_words as u32 == q {
                                    break;
                                }
                                i = i.wrapping_add(1);
                                while (i as u32) < rend && m.byte(m.word(expb).wrapping_add(i as u32)) == b' ' {
                                    i = i.wrapping_add(1);
                                }
                            } else {
                                i = i.wrapping_add(1);
                            }
                        }
                    }
                    if output_words as u32 == q {
                        m.put_half(poss.wrapping_add(q.wrapping_mul(2)), i);
                        h.call_m(0x1738e8, &[log, log_tag, 5, 0, 11], m);
                    } else {
                        invalid = 1;
                        m.put_half(poss.wrapping_add(q.wrapping_mul(2)), 0);
                        h.call_m(0x173dc0, &[log, log_tag, 0x5dd1, 0], m);
                    }
                }
            }
        }
        for q in 0..npos {
            let pq = m.half(poss.wrapping_add(q.wrapping_mul(2))) as u32;
            if pq == 9999 || pq > output_length {
                invalid = 1;
            }
            if pq != 9999 && q != 0 {
                let pp = m.half(poss.wrapping_add(q.wrapping_sub(1).wrapping_mul(2))) as u32;
                if pp != 9999 && pp > pq {
                    invalid = 1;
                }
            }
        }
        if invalid == 1 {
            for q in 0..npos {
                m.put_half(poss.wrapping_add(q.wrapping_mul(2)), 0);
            }
        }
        i = 0;
        j = output_length.wrapping_sub(1) as u16;
        while m.byte(m.word(expb).wrapping_add(i as u32)) == b' ' {
            i = i.wrapping_add(1);
        }
        let exb = m.word(expb);
        let alb = m.word(alnb);
        let alen = m.half(alnb.wrapping_add(6)) as u32;
        while m.byte(exb.wrapping_add(j as u32)) == b' ' {
            m.put_byte(exb.wrapping_add(j as u32), 0);
            if (j as u32) < alen {
                m.put_byte(alb.wrapping_add(j as u32), 0);
            }
            j = j.wrapping_sub(1);
        }
        if i != 0 {
            h.call_m(0x1738e8, &[log, log_tag, 5, 0, 12], m);
            for q in 0..npos {
                if input_words > 1 {
                    let pq = m.half(poss.wrapping_add(q.wrapping_mul(2)));
                    if pq != 0 {
                        m.put_half(poss.wrapping_add(q.wrapping_mul(2)), pq.wrapping_add(i));
                        h.call_m(0x1738e8, &[log, log_tag, 5, 0, 13], m);
                    }
                }
            }
        }
        if m.half(alnb.wrapping_add(6)) as u32 != alen {
            // NOTE: alen was captured before the trim; the C compares the
            // live length. Re-read to match exactly.
        }
        if m.half(alnb.wrapping_add(6)) != m.half(expb.wrapping_add(6)) {
            h.call_m(0x173dc0, &[log, log_tag, 0x5dfb, 0], m);
            m.put_half(alnb.wrapping_add(6), 0);
            m.put_byte(m.word(alnb), 0);
        }
        {
            let al = s_len(m, m.word(alnb));
            let mut prev = b' ';
            for kk in 0..al {
                let c = m.byte(m.word(alnb).wrapping_add(kk));
                if c == prev {
                    if c != b' ' {
                        m.put_byte(m.word(alnb).wrapping_add(kk), b' ');
                    }
                } else if c == b'N' || c == b'L' {
                    prev = c;
                }
            }
            if prev == b' ' {
                m.put_half(alnb.wrapping_add(6), 0);
                m.put_byte(m.word(alnb), 0);
            }
        }
        h.call_m(0x1738e8, &[log, log_tag, 4, 0, 14], m);
        if m.half(alnb.wrapping_add(6)) != 0 {
            let alen2 = s_len(m, m.word(alnb));
            let exlen = s_len(m, m.word(expb));
            if alen2 != exlen {
                h.call_m(0x173dc0, &[log, log_tag, 0x5dfd, 0], m);
                m.put_half(alnb.wrapping_add(6), 0);
                m.put_byte(m.word(alnb), 0);
            }
        }
        m.put_half(afterp, 0);
        result = h.call_m(
            0x24b290,
            &[owner, m.half(range) as u32, m.half(range.wrapping_add(2)) as u32, afterp],
            m,
        );
        if result >= 0x80000000 {
            h.call_m(0x173dc0, &[log, log_tag, 0x5dd2, 0], m);
            break;
        }
        h.call_m(0x1738e8, &[log, log_tag, 5, 0, 15], m);
        m.put_half(created, 0);
        // 7th arg is the loop range (start/end become the new token's
        // fields 1/2): ROM passes [sp+0x18], the same word 24b8d0 takes.
        result = h.call_m(
            0x24b540,
            &[owner, m.word(iname), m.word(expb), i as u32, m.word(afterp) as u32, created, range],
            m,
        );
        if result >= 0x80000000 {
            h.call_m(0x173dc0, &[log, log_tag, 0x5dd3, 0], m);
            break;
        }
        h.call_m(0x1738e8, &[log, log_tag, 5, 0, 16], m);
        if debug == 1 {
            let r2 = h.call_m(0x24ac00, &[owner, 0x2b5644, 0], m);
            let _ = r2;
        }
        result = h.call_m(0x24b8d0, &[owner, range], m);
        if result >= 0x80000000 {
            h.call_m(0x173dc0, &[log, log_tag, 0x5dd4, 0], m);
            break;
        }
        if debug == 1 {
            let r2 = h.call_m(0x24ac00, &[owner, 0x2b56c4, 0], m);
            let _ = r2;
        }
        cursor = m.half(range.wrapping_add(2));
    }
    h.call_m(0x1738e8, &[log, log_tag, 4, 0, 17], m);
    for (i, v) in sv_out.iter().enumerate() {
        m.put_word(fb_out.wrapping_add(i as u32 * 4), *v);
    }
    result
}

/// fn_195bec. memset, returns dst.
fn s_set(m: &mut Memory<'_>, dst: u32, val: u32, n: u32) -> u32 {
    let mut d = dst;
    let mut k = n;
    if k != 0 {
        loop {
            m.put_byte(d, (val & 0xff) as u8);
            d = d.wrapping_add(1);
            k -= 1;
            if k == 0 {
                break;
            }
        }
    }
    dst
}

/// fn_24cea0. Allocate d->f8 bytes; clear on success.
pub fn fn_24cea0_m(
    m: &mut Memory<'_>,
    s: u32,
    d: u32,
    h: &mut impl Host,
) -> u32 {
    let ctx = m.word(s);
    let f4 = m.word(ctx.wrapping_add(4));
    let want = m.half(s.wrapping_add(0x4c));
    m.put_half(d.wrapping_add(8), want);
    let b = h.call_m(0x1963b8, &[f4, 1, want as u32], m);
    m.put_word(d, b);
    if b != 0 {
        m.put_byte(b, 0);
        m.put_half(d.wrapping_add(4), want);
        m.put_half(d.wrapping_add(6), 0);
        return 0;
    }
    h.call_m(0x173dc0, &[m.word(ctx.wrapping_add(16)), m.word(s.wrapping_add(0x48)), 0x5df0, 0], m);
    e_code(10)
}

/// fn_24d030. Release buf block if live; zero length/aux. Buf layout per
/// the ROM (src/fn_24d030.c): buf@+0, len@+4, aux@+6; heap at ctx+4.
/// o is the context (heap at o+4); b is the 12-byte string shell.
pub fn fn_24d030_m(m: &mut Memory<'_>, o: u32, b: u32, h: &mut impl Host) {
    let heap = m.word(o.wrapping_add(4));
    // ROM checks len at b+4, not b+2.
    if m.half(b.wrapping_add(4)) != 0 && m.word(b) != 0 {
        h.call_m(0x196428, &[heap, m.word(b)], m);
        m.put_word(b, 0);
    }
    m.put_half(b.wrapping_add(4), 0);
    m.put_half(b.wrapping_add(6), 0);
}

/// fn_24d13c. Append src string onto dst (grow via heap).
pub fn fn_24d13c_m(
    m: &mut Memory<'_>,
    o: u32,
    dst: u32,
    src: u32,
    h: &mut impl Host,
) -> u32 {
    let ctx = m.word(o);
    let heap = m.word(ctx.wrapping_add(4));
    let log = m.word(ctx.wrapping_add(16));
    let tag = m.word(o.wrapping_add(0x48));
    let add = m.half(src.wrapping_add(6)) as u32;
    if add == 0 {
        return 0;
    }
    if (m.half(dst.wrapping_add(4)) as u32)
        <= (m.half(dst.wrapping_add(6)) as u32).wrapping_add(add)
    {
        let ncap = add.wrapping_add(m.half(dst.wrapping_add(4)) as u32);
        let mem = h.call_m(0x1963e0, &[heap, m.word(dst), ncap.wrapping_add(1)], m);
        if mem == 0 || mem >= 0x80000000 {
            h.call_m(0x173dc0, &[log, tag, 0x5df0, 0], m);
            return e_code(10);
        }
        m.put_word(dst, mem);
        m.put_half(dst.wrapping_add(4), ncap as u16);
        if m.half(dst.wrapping_add(6)) == 0 {
            m.put_byte(m.word(dst), 0);
        }
    }
    s_nappend(m, m.word(dst), m.word(src), add);
    let len = m.half(dst.wrapping_add(6));
    m.put_half(dst.wrapping_add(6), len.wrapping_add(add as u16));
    0
}

/// fn_24d8c8. Single item lookup through fn_1966ac.
pub fn fn_24d8c8_m(
    m: &mut Memory<'_>,
    this: u32,
    idx: u32,
    arg: u32,
    out: u32,
    h: &mut impl Host,
) -> u32 {
    let items = m.word(this.wrapping_add(60));
    m.put_word(out, 0);
    let item = m.word(items.wrapping_add((idx as u32).wrapping_mul(4)));
    h.call_m(0x1966ac, &[item, arg, out], m);
    0
}

/// fn_24a5d4. Token index get/set through the text vtable.
pub fn fn_24a5d4_m(
    m: &mut Memory<'_>,
    p: u32,
    id: u32,
    val: u32,
    h: &mut impl Host,
) -> u32 {
    // ARM stack temporaries: the FRAME slice, saved and restored, so the
    // heap traffic matches the ARM exactly.
    let fb = 0x1000u32;
    let sv0 = m.word(fb);
    let sv1 = m.word(fb.wrapping_add(4));
    let s1 = fb;
    let s2 = fb.wrapping_add(4);
    m.put_half(s1, 0);
    m.put_half(s2, 0);
    let vt = m.word(p.wrapping_add(0x0c));
    let handle0 = m.word(p.wrapping_add(0x10));
    let handle1 = m.word(p.wrapping_add(0x14));
    let set = m.word(vt.wrapping_add(0x54));
    let get = m.word(vt.wrapping_add(0x5c));
    // Field 1 is the token index (ROM: mov r3, #1); the store passes
    // the count 1 like every other +0x54-family call (rule_io).
    let mut r = h.call_m(get, &[handle0, handle1, id, 1, s1], m);
    if r < 0x80000000 && m.half(s1) == 1 {
        r = h.call_m(set, &[handle0, handle1, id, 1, 1, val, s2], m);
        if m.half(s2) != 1 {
            let base = m.word(p);
            h.call_m(0x173dc0, &[m.word(base.wrapping_add(16)), m.word(p.wrapping_add(0x48)), 0x5dde, 0], m);
            r = e_code(0);
        } else {
            r = 0;
        }
    } else {
        let base = m.word(p);
        h.call_m(0x173dc0, &[m.word(base.wrapping_add(16)), m.word(p.wrapping_add(0x48)), 0x5ddf, 0], m);
        r = e_code(0);
    }
    m.put_word(fb, sv0);
    m.put_word(fb.wrapping_add(4), sv1);
    r
}

/// fn_24a6bc. Token position get/set through the text vtable.
pub fn fn_24a6bc_m(
    m: &mut Memory<'_>,
    p: u32,
    id: u32,
    val: u32,
    h: &mut impl Host,
) -> u32 {
    // ARM stack temporaries: the FRAME slice, saved and restored, so the
    // heap traffic matches the ARM exactly.
    let fb = 0x1000u32;
    let sv0 = m.word(fb);
    let sv1 = m.word(fb.wrapping_add(4));
    let s1 = fb;
    let s2 = fb.wrapping_add(4);
    m.put_half(s1, 0);
    m.put_half(s2, 0);
    let vt = m.word(p.wrapping_add(0x0c));
    let handle0 = m.word(p.wrapping_add(0x10));
    let handle1 = m.word(p.wrapping_add(0x14));
    let set = m.word(vt.wrapping_add(0x54));
    let get = m.word(vt.wrapping_add(0x5c));
    let base = m.word(p);
    let tag = m.word(p.wrapping_add(0x48));
    // Field 2 is the token position (ROM: mov r3, #2).
    let mut r = h.call_m(get, &[handle0, handle1, id, 2, s1], m);
    if r < 0x80000000 && m.half(s1) == 1 {
        r = h.call_m(set, &[handle0, handle1, id, 2, 1, val, s2], m);
        if m.half(s2) != 1 {
            h.call_m(0x173dc0, &[m.word(base.wrapping_add(16)), tag, 0x5ddf, 0], m);
            r = e_code(0);
        } else {
            r = 0;
        }
    } else {
        h.call_m(0x173dc0, &[m.word(base.wrapping_add(16)), tag, 0x5dde, 0], m);
        r = e_code(0);
    }
    m.put_word(fb, sv0);
    m.put_word(fb.wrapping_add(4), sv1);
    r
}

/// fn_24a7a4. Token type query through the text vtable.
pub fn fn_24a7a4_m(
    m: &mut Memory<'_>,
    p: u32,
    a: u32,
    b: u32,
    h: &mut impl Host,
) -> u32 {
    // ARM stack temporaries: the FRAME slice, saved and restored, so the
    // heap traffic matches the ARM exactly.
    let fb = 0x1000u32;
    let sv0 = m.word(fb);
    let sv1 = m.word(fb.wrapping_add(4));
    let s1 = fb;
    let s2 = fb.wrapping_add(4);
    m.put_half(s1, 0);
    m.put_half(s2, 0);
    let vt = m.word(p.wrapping_add(0x0c));
    let handle0 = m.word(p.wrapping_add(0x10));
    let handle1 = m.word(p.wrapping_add(0x14));
    let query = m.word(vt.wrapping_add(0x5c));
    let get = m.word(vt.wrapping_add(0x54));
    let env = m.word(p);
    let w48 = m.word(p.wrapping_add(0x48));
    let mut r = 0u32;
    let ok = h.call_m(query, &[handle0, handle1, a, 0, s1], m);
    if ok < 0x80000000 && m.half(s1) == 1 {
        // Field 0 is the token kind; count 1 rides the stack like the ARM.
        r = h.call_m(get, &[handle0, handle1, a, 0, 1, b, s2], m);
        if m.half(s2) != 1 {
            h.call_m(0x173dc0, &[m.word(env.wrapping_add(16)), w48, 0x5de0, 0], m);
            r = e_code(0);
        }
    }
    m.put_word(fb, sv0);
    m.put_word(fb.wrapping_add(4), sv1);
    r
}

/// fn_24cdac. Allocate the two expansion text buffers (+0xc0, +0xb8) sized
/// (half(p+0x62)+1)*2, 1:1 from the ROM (src/fn_24cdac.c; ldrh [r4,#98]).
/// On failure the first buffer stays allocated while the ROM logs 0x5df0.
pub fn fn_24cdac_m(
    m: &mut Memory<'_>,
    p: u32,
    h: &mut impl Host,
) -> u32 {
    let ctx = m.word(p);
    let heap = m.word(ctx.wrapping_add(4));
    let size = (m.half(p.wrapping_add(0x62)) as u32)
        .wrapping_add(1)
        .wrapping_mul(2);
    let b0 = h.call_m(0x1963b8, &[heap, 1, size], m);
    m.put_word(p.wrapping_add(0xc0), b0);
    if b0 != 0 && b0 < 0x80000000 {
        m.put_half(p.wrapping_add(0xc4), 0);
        let b1 = h.call_m(0x1963b8, &[heap, 1, size], m);
        m.put_word(p.wrapping_add(0xb8), b1);
        if b1 != 0 && b1 < 0x80000000 {
            m.put_half(p.wrapping_add(0xbc), 0);
            return 0;
        }
    }
    h.call_m(
        0x173dc0,
        &[
            m.word(ctx.wrapping_add(16)),
            m.word(p.wrapping_add(0x48)),
            0x5df0,
            0,
        ],
        m,
    );
    e_code(10)
}

/// fn_24cb74. Allocate the +0x78 record (calloc 1x16); tag 3 on success.
/// 1:1 from src/fn_24cb74.c (ROM-verified seg1 r8): heap at ctx+4, log at
/// ctx+16, id at p+0x48.
pub fn fn_24cb74_m(m: &mut Memory<'_>, p: u32, h: &mut impl Host) -> u32 {
    let ctx = m.word(p);
    let heap = m.word(ctx.wrapping_add(4));
    let b = h.call_m(0x1963b8, &[heap, 1, 16], m);
    m.put_word(p.wrapping_add(0x78), b);
    if b == 0 || b >= 0x80000000 {
        h.call_m(
            0x173dc0,
            &[
                m.word(ctx.wrapping_add(16)),
                m.word(p.wrapping_add(0x48)),
                0x5df0,
                0,
            ],
            m,
        );
        return e_code(10);
    }
    m.put_half(p.wrapping_add(0x7c), 3);
    m.put_half(p.wrapping_add(0x7e), 0);
    0
}

/// fn_24cbd4. Release the +0x78 buffer and clear its count words.
/// 1:1 from src/fn_24cbd4.c (ROM-verified seg1 r8).
pub fn fn_24cbd4_m(m: &mut Memory<'_>, p: u32, h: &mut impl Host) -> u32 {
    let heap = m.word(m.word(p).wrapping_add(4));
    let b = m.word(p.wrapping_add(0x78));
    if b != 0 {
        h.call_m(0x196428, &[heap, b], m);
        m.put_word(p.wrapping_add(0x78), 0);
    }
    m.put_half(p.wrapping_add(0x7c), 0);
    m.put_half(p.wrapping_add(0x7e), 0);
    0
}

/// fn_24cc08. Grow the +0x78 slot array to n and clear the used count.
/// 1:1 from src/fn_24cc08.c (ROM-verified seg1 r8): realloc through
/// 0x1963e0 with (k+1)*4; log 0x5df0 on failure.
pub fn fn_24cc08_m(m: &mut Memory<'_>, p: u32, n: u32, h: &mut impl Host) -> u32 {
    if n != 0 {
        let k = if n > 3 {
            n
        } else {
            if n != 3 || m.half(p.wrapping_add(0x7c)) <= 3 {
                m.put_half(p.wrapping_add(0x7e), 0);
                return 0;
            }
            3
        };
        let ctx = m.word(p);
        let heap = m.word(ctx.wrapping_add(4));
        let old = m.word(p.wrapping_add(0x78));
        let r = h.call_m(0x1963e0, &[heap, old, k.wrapping_add(1).wrapping_mul(4)], m);
        if r == 0 || r >= 0x80000000 {
            h.call_m(
                0x173dc0,
                &[
                    m.word(ctx.wrapping_add(16)),
                    m.word(p.wrapping_add(0x48)),
                    0x5df0,
                    0,
                ],
                m,
            );
            return e_code(10);
        }
        m.put_word(p.wrapping_add(0x78), r);
        m.put_half(p.wrapping_add(0x7c), k as u16);
    }
    m.put_half(p.wrapping_add(0x7e), 0);
    0
}

/// fn_24ce40. Release the two expansion text buffers and clear the slots.
pub fn fn_24ce40_m(
    m: &mut Memory<'_>,
    p: u32,
    h: &mut impl Host,
) -> u32 {
    let heap = m.word(m.word(p).wrapping_add(4));
    let b0 = m.word(p.wrapping_add(0xc0));
    if b0 != 0 {
        h.call_m(0x196428, &[heap, b0], m);
        m.put_word(p.wrapping_add(0xc0), 0);
    }
    m.put_half(p.wrapping_add(0xc4), 0);
    let b1 = m.word(p.wrapping_add(0xb8));
    if b1 != 0 {
        h.call_m(0x196428, &[heap, b1], m);
        m.put_word(p.wrapping_add(0xb8), 0);
    }
    m.put_half(p.wrapping_add(0xbc), 0);
    0
}

// ===== seg1 r7: the 2b4968 DoExpansions subtree =====
// All 1:1 from src/fn_*.c, checked against the ROM disassembly. Struct
// shells the ARM keeps on its stack live in one guest heap block per call
// (freed at every exit, so the allocator order is preserved); the vtable
// calls go through the host. Expansion strings are 12 bytes on the ARM
// (buf@0, cap@4, len@6), not the 10 of src/fn_2b4968.c: expb is owner+0x88
// and alnb is owner+0x94, positions is owner+0xb8, boundaries owner+0xc0,
// count owner+0xc4, debug owner+0xc6.

/// fn_24cfa0. Ensure buf capacity exceeds n; clear on success.
pub fn fn_24cfa0_m(
    m: &mut Memory<'_>,
    s: u32,
    d: u32,
    n: u16,
    h: &mut impl Host,
) -> u32 {
    let ctx = m.word(s);
    let heap = m.word(ctx.wrapping_add(4));
    if (m.half(d.wrapping_add(4)) as u32) <= n as u32 {
        // seg6 r22: 1:1 with src/fn_24cfa0.c grow_buffer, which calls
        // fn_1963e0 (realloc), not fn_1963b8 (calloc). The old buffer rides
        // a1; a zero old block allocates, like heap_realloc_m old==0.
        let b = h.call_m(0x1963e0, &[heap, m.word(d), n as u32 + 1], m);
        if b == 0 || b >= 0x80000000 {
            let log = m.word(ctx.wrapping_add(16));
            let tag = m.word(s.wrapping_add(0x48));
            h.call_m(0x173dc0, &[log, tag, 0x5df0, 0], m);
            return e_code(10);
        }
        m.put_word(d, b);
        m.put_half(d.wrapping_add(4), n);
    }
    m.put_byte(m.word(d), 0);
    m.put_half(d.wrapping_add(6), 0);
    0
}

/// fn_24b290. Find the token node for [position, index].
/// TextMethods: get slot +0x34, next slot +0x3c (verified on ROM).
pub fn fn_24b290_m(
    m: &mut Memory<'_>,
    text: u32,
    position: u32,
    index: u32,
    out: u32,
    h: &mut impl Host,
) -> u32 {
    if text == 0 {
        return e_code(7);
    }
    m.put_half(out, 0);
    let vt = m.word(text.wrapping_add(0x0c));
    let h0 = m.word(text.wrapping_add(0x10));
    let h1 = m.word(text.wrapping_add(0x14));
    let get = m.word(vt.wrapping_add(0x34));
    let next = m.word(vt.wrapping_add(0x3c));
    // ARM stack temporaries in the FRAME slice (saved/restored).
    let fb = 0x1010u32;
    let mut sv = [0u32; 4];
    for (i, v) in sv.iter_mut().enumerate() {
        *v = m.word(fb.wrapping_add(i as u32 * 4));
    }
    // root, node, nodeIndex, nodePosition, type.
    let t = fb;
    let root = t;
    let node = t.wrapping_add(2);
    let nidx = t.wrapping_add(4);
    let npos = t.wrapping_add(6);
    let typ = t.wrapping_add(8);
    for a in [root, node, nidx, npos] {
        m.put_half(a, 0);
    }
    m.put_byte(typ, 0);
    let mut rc = h.call_m(get, &[h0, h1, 1, 0, root], m);
    let done = if rc >= 0x80000000 {
        true
    } else {
        rc = h.call_m(get, &[h0, h1, 3, m.half(root) as u32, node], m);
        rc != 0
    };
    if !done {
        while m.half(node) != 0 {
            let nid = m.half(node) as u32;
            rc = fn_24a6bc_m(m, text, nid, nidx, h);
            if rc >= 0x80000000 {
                break;
            }
            rc = fn_24a5d4_m(m, text, nid, npos, h);
            if rc >= 0x80000000 {
                break;
            }
            rc = fn_24a7a4_m(m, text, nid, typ, h);
            if rc >= 0x80000000 {
                break;
            }
            let ty = m.byte(typ);
            let ni = m.half(nidx) as u32;
            let np = m.half(npos) as u32;
            if ty != 5 && ni == index {
                m.put_half(out, nid as u16);
                loop {
                    let cur = m.half(node) as u32;
                    rc = h.call_m(next, &[h0, h1, cur, node], m);
                    if rc != 0 || m.half(node) == 0 {
                        break;
                    }
                    let nid2 = m.half(node) as u32;
                    rc = fn_24a6bc_m(m, text, nid2, nidx, h);
                    if rc >= 0x80000000 {
                        break;
                    }
                    rc = fn_24a5d4_m(m, text, nid2, npos, h);
                    if rc >= 0x80000000 {
                        break;
                    }
                    rc = fn_24a7a4_m(m, text, nid2, typ, h);
                    if rc >= 0x80000000 {
                        break;
                    }
                    if m.byte(typ) == 6 {
                        break;
                    }
                    let np2 = m.half(npos) as u32;
                    if np2 < position {
                        m.put_half(out, nid2 as u16);
                        break;
                    }
                    if np2 > position {
                        break;
                    }
                }
                break;
            }
            if position <= np {
                loop {
                    m.put_half(out, m.half(node));
                    rc = h.call_m(next, &[h0, h1, m.half(node) as u32, node], m);
                    if rc != 0 {
                        break;
                    }
                    if m.half(node) == 0 {
                        break;
                    }
                    let nid3 = m.half(node) as u32;
                    rc = fn_24a6bc_m(m, text, nid3, nidx, h);
                    if rc >= 0x80000000 {
                        break;
                    }
                    rc = fn_24a5d4_m(m, text, nid3, npos, h);
                    if rc >= 0x80000000 {
                        break;
                    }
                    if m.half(npos) as u32 != position {
                        break;
                    }
                    if (m.half(nidx) as u32) < index {
                        continue;
                    }
                    break;
                }
                break;
            }
            if ty != 5 && ni > index {
                break;
            }
            rc = h.call_m(next, &[h0, h1, nid, node], m);
            if rc != 0 {
                break;
            }
        }
    }
    for (i, v) in sv.iter().enumerate() {
        m.put_word(fb.wrapping_add(i as u32 * 4), *v);
    }
    rc
}

/// fn_24b540. Save an expansion into the token store.
/// EngineMethods: first +0x34, find +0x24, lookup +0x28, set +0x50.
#[allow(clippy::too_many_arguments)]
pub fn fn_24b540_m(
    m: &mut Memory<'_>,
    p: u32,
    name: u32,
    text: u32,
    offset: u32,
    key: u32,
    token: u32,
    values: u32,
    h: &mut impl Host,
) -> u32 {
    let ctx = m.word(p);
    let log = m.word(ctx.wrapping_add(16));
    let tag = m.word(p.wrapping_add(0x48));
    let mt = m.word(p.wrapping_add(0x0c));
    let h0 = m.word(p.wrapping_add(0x10));
    let h1 = m.word(p.wrapping_add(0x14));
    let first = m.word(mt.wrapping_add(0x34));
    let find = m.word(mt.wrapping_add(0x24));
    let lookup = m.word(mt.wrapping_add(0x28));
    let set = m.word(mt.wrapping_add(0x50));
    // ARM stack temporaries in the FRAME slice (saved/restored).
    let fb = 0x1030u32;
    let mut sv = [0u32; 4];
    for (i, v) in sv.iter_mut().enumerate() {
        *v = m.word(fb.wrapping_add(i as u32 * 4));
    }
    let t = fb;
    let first_out = t;
    let count = t.wrapping_add(4);
    m.put_half(first_out, 0);
    m.put_half(count, 0);
    h.call_m(0x1738e8, &[log, tag, 4, 0], m);
    let mut rc = if key == 0 {
        let r = h.call_m(first, &[h0, h1, 1, 0, first_out], m);
        if r >= 0x80000000 {
            r
        } else {
            h.call_m(find, &[h0, h1, 3, m.half(first_out) as u32, token], m)
        }
    } else {
        h.call_m(lookup, &[h0, h1, key, token], m)
    };
    if rc < 0x80000000 {
        let tid = m.half(token) as u32;
        // kind 6, then the two value halves, then name+text, entries, extra.
        let kb = fb.wrapping_add(8);
        m.put_byte(kb, 6);
        {
            rc = h.call_m(set, &[h0, h1, tid, 0, 1, kb, count], m);
            if rc < 0x80000000 {
                rc = if m.half(count) != 1 {
                    e_code(0)
                } else {
                    h.call_m(set, &[h0, h1, tid, 1, 1, values, count], m)
                };
            }
            if rc < 0x80000000 {
                rc = if m.half(count) != 1 {
                    e_code(0)
                } else {
                    h.call_m(set, &[h0, h1, tid, 2, 1, values.wrapping_add(2), count], m)
                };
            }
            if rc < 0x80000000 {
                rc = if m.half(count) != 1 {
                    e_code(0)
                } else {
                    let n = s_len(m, name).wrapping_add(1);
                    rc = h.call_m(set, &[h0, h1, tid, 4, n, name, count], m);
                    if rc < 0x80000000 && m.half(count) as u32 != n {
                        e_code(0)
                    } else {
                        rc
                    }
                };
            }
            if rc < 0x80000000 {
                let tx = text.wrapping_add(offset);
                let n = s_len(m, tx).wrapping_add(1);
                rc = h.call_m(set, &[h0, h1, tid, 3, n, tx, count], m);
                if rc < 0x80000000 && m.half(count) as u32 != n {
                    rc = e_code(0);
                }
            }
            if rc < 0x80000000 {
                // entries (rule store blob at p+0xb8, count at p+0xc4)
                // and the extra layer (p+0x94, enabled at p+0x9a).
                let ec = m.half(p.wrapping_add(0xc4)) as u32;
                if ec > 1 {
                    let ent = m.word(p.wrapping_add(0xb8));
                    rc = h.call_m(set, &[h0, h1, tid, 5, ec, ent, count], m);
                    if rc < 0x80000000 && m.half(count) as u32 != ec {
                        rc = e_code(0);
                    }
                }
            }
            if rc < 0x80000000 {
                let extra_on = m.half(p.wrapping_add(0x9a));
                if extra_on != 0 {
                    let ex = m.word(p.wrapping_add(0x94)).wrapping_add(offset);
                    let ln = s_len(m, ex);
                    if ln != 0 {
                        rc = h.call_m(set, &[h0, h1, tid, 6, ln + 1, ex, count], m);
                        if rc < 0x80000000 && m.half(count) as u32 != ln + 1 {
                            rc = e_code(0);
                        }
                    }
                }
            }
        }
    }
    for (i, v) in sv.iter().enumerate() {
        m.put_word(fb.wrapping_add(i as u32 * 4), *v);
    }
    h.call_m(0x1738e8, &[log, tag, 4, 0], m);
    rc
}

/// fn_24b8d0. Delete tokens subsumed by [start, end).
/// TokenMethods: find +0x34, next +0x3c, remove +0x60.
pub fn fn_24b8d0_m(
    m: &mut Memory<'_>,
    ctx: u32,
    range: u32,
    h: &mut impl Host,
) -> u32 {
    if ctx == 0 {
        return e_code(7);
    }
    let owner = m.word(ctx);
    let logger = m.word(owner.wrapping_add(16));
    let log = m.word(ctx.wrapping_add(0x48));
    h.call_m(0x1738e8, &[logger, log, 4, 0], m);
    let mt = m.word(ctx.wrapping_add(0x0c));
    let h0 = m.word(ctx.wrapping_add(0x10));
    let h1 = m.word(ctx.wrapping_add(0x14));
    let find = m.word(mt.wrapping_add(0x34));
    let next = m.word(mt.wrapping_add(0x3c));
    let remove = m.word(mt.wrapping_add(0x60));
    // ARM stack temporaries in the FRAME slice (saved/restored).
    let fb = 0x1050u32;
    let mut sv = [0u32; 4];
    for (i, v) in sv.iter_mut().enumerate() {
        *v = m.word(fb.wrapping_add(i as u32 * 4));
    }
    let t = fb;
    let group = t;
    let cur = t.wrapping_add(2);
    let start = t.wrapping_add(4);
    let end = t.wrapping_add(6);
    let ty = t.wrapping_add(8);
    m.put_half(group, 0);
    m.put_half(cur, 0);
    m.put_half(start, 0);
    m.put_half(end, 0);
    m.put_byte(ty, 0);
    let rstart = m.half(range) as u32;
    let rend = m.half(range.wrapping_add(2)) as u32;
    let mut previous = 0u32;
    let mut rc = h.call_m(find, &[h0, h1, 1, 0, group], m);
    if rc < 0x80000000 {
        rc = h.call_m(find, &[h0, h1, 3, m.half(group) as u32, cur], m);
        if rc == 0 {
            while m.half(cur) != 0 {
                let cid = m.half(cur) as u32;
                rc = fn_24a5d4_m(m, ctx, cid, start, h);
                if rc >= 0x80000000 || rend <= m.half(start) as u32 {
                    break;
                }
                rc = fn_24a6bc_m(m, ctx, cid, end, h);
                if rc >= 0x80000000 {
                    break;
                }
                rc = fn_24a7a4_m(m, ctx, cid, ty, h);
                if rc >= 0x80000000 {
                    break;
                }
                let cs = m.half(start) as u32;
                let ce = m.half(end) as u32;
                let ck = m.byte(ty);
                if rstart <= cs && ce <= rend && (ck == 0 || ck == 1 || ck == 2 || ck == 3) {
                    rc = h.call_m(remove, &[h0, h1, cid], m);
                    if rc != 0 {
                        h.call_m(0x173dc0, &[logger, log, 0x5de1, 0], m);
                        break;
                    }
                    h.call_m(0x1738e8, &[logger, log, 5, 0], m);
                    if previous == 0 {
                        rc = h.call_m(find, &[h0, h1, 3, m.half(group) as u32, cur], m);
                        if rc == 0 {
                            continue;
                        }
                        break;
                    }
                    m.put_half(cur, previous as u16);
                    rc = h.call_m(next, &[h0, h1, previous, cur], m);
                    if rc != 0 {
                        break;
                    }
                    continue;
                }
                previous = cid;
                rc = h.call_m(next, &[h0, h1, previous, cur], m);
                if rc != 0 {
                    break;
                }
            }
        }
    }
    h.call_m(0x1738e8, &[logger, log, 4, 0], m);
    for (i, v) in sv.iter().enumerate() {
        m.put_word(fb.wrapping_add(i as u32 * 4), *v);
    }
    rc
}

/// fn_2e4e80. Run the depes layer function and merge its output maps.
/// DepesInterface slots: setInput +0x24, setMarkers +0x30, run +0x2c,
/// getMarkers +0x34, getOutput +0x28 (verified on ROM).
#[allow(clippy::too_many_arguments)]
pub fn fn_2e4e80_m(
    m: &mut Memory<'_>,
    this: u32,
    function: u32,
    word: u32,
    map_words: u32,
    done: u32,
    out_map: u32,
    h: &mut impl Host,
) -> u32 {
    let ctx = m.word(this);
    let log = m.word(ctx.wrapping_add(16));
    let tag = m.word(this.wrapping_add(0x48));
    let debug = m.byte(this.wrapping_add(0xc6));
    m.put_byte(done, 0);
    // Frame for the interface out-words (the ARM stack): text, textLen,
    // alignment, alignmentLen, markers, markerCount. Saved/restored.
    // Frame slice clear of the 2b4260 depth slices (0x1100 + depth*96).
    let fb = 0x1500u32;
    let mut sv = [0u32; 8];
    for (i, v) in sv.iter_mut().enumerate() {
        *v = m.word(fb.wrapping_add(i as u32 * 4));
    }
    let f = fb;
    let tp = f;
    let tl = f.wrapping_add(4);
    let ap = f.wrapping_add(8);
    let al = f.wrapping_add(12);
    let mp = f.wrapping_add(16);
    let mc = f.wrapping_add(20);
    let input = m.word(this.wrapping_add(0x64));
    let in_text = m.word(input);
    let in_out = m.word(input.wrapping_add(4));
    // DepesInput: text@0, output@4, reserved[2]@8, offset@16.
    let in_off = m.half(input.wrapping_add(16));
    let iface = m.word(this.wrapping_add(0x24));
    let hh0 = m.word(this.wrapping_add(0x28));
    let hh1 = m.word(this.wrapping_add(0x2c));
    let set_input = m.word(iface.wrapping_add(0x24));
    let set_markers = m.word(iface.wrapping_add(0x30));
    let run = m.word(iface.wrapping_add(0x2c));
    let get_markers = m.word(iface.wrapping_add(0x34));
    let get_output = m.word(iface.wrapping_add(0x28));
    let mut rc;
    // Real NULL/""/empty gate on the input text.
    let is_null = null_or_empty(m, in_text);
    if is_null {
        rc = fn_24d068_m(m, this, in_out, 0, h);
        if rc >= 0x80000000 {
            for (i, v) in sv.iter().enumerate() {
                m.put_word(fb.wrapping_add(i as u32 * 4), *v);
            }
            return rc;
        }
    } else {
        if debug == 1 {
            h.call_m(0x1738e8, &[log, tag, 5, 0], m);
        }
        let length = s_len(m, in_text);
        // setInput takes the length twice on the ROM.
        rc = h.call_m(set_input, &[hh0, hh1, 0, in_text, length, length], m);
        if rc >= 0x80000000 {
            h.call_m(0x173dc0, &[log, tag, 0x5dca, 0], m);
        } else {
            rc = fn_24cfa0_m(m, this, this.wrapping_add(0xa0), length as u16, h);
            if rc >= 0x80000000 {
                // falls to finish
            } else {
                let spaces = m.word(this.wrapping_add(0xa0));
                let n = s_len(m, in_text);
                let mut i = 0u32;
                while i < n {
                    m.put_byte(
                        spaces.wrapping_add(i),
                        if m.byte(in_text.wrapping_add(i)) == b' ' { 1 } else { 0 },
                    );
                    i = i.wrapping_add(1);
                }
                rc = h.call_m(set_markers, &[hh0, hh1, spaces, length], m);
                if rc >= 0x80000000 {
                    h.call_m(0x173dc0, &[log, tag, 0x5dcb, 0], m);
                } else {
                    // functionName + 1 (skip the '_' prefix).
                    let fnp = m.byte(function);
                    let _ = fnp;
                    rc = h.call_m(run, &[hh0, hh1, function.wrapping_add(1)], m);
                    if rc >= 0x80000000 {
                        h.call_m(0x173dc0, &[log, tag, 0x5dcc, 0], m);
                    } else {
                        rc = h.call_m(get_markers, &[hh0, hh1, mp, mc], m);
                        if rc >= 0x80000000 {
                            h.call_m(0x173dc0, &[log, tag, 0x5dcd, 0], m);
                        } else {
                            rc = h.call_m(get_output, &[hh0, hh1, 0, tp, tl], m);
                            if rc >= 0x80000000 {
                                h.call_m(0x173dc0, &[log, tag, 0x5dce, 0], m);
                            } else {
                                let text = m.word(tp);
                                let text_len = m.half(tl) as u32;
                                m.put_byte(text.wrapping_add(text_len), 0);
                                if debug == 1 {
                                    h.call_m(0x1738e8, &[log, tag, 5, 0], m);
                                }
                                rc = h.call_m(get_output, &[hh0, hh1, 1, ap, al], m);
                                if rc >= 0x80000000 {
                                    h.call_m(0x173dc0, &[log, tag, 0x5dfa, 0], m);
                                } else {
                                    let align = m.word(ap);
                                    let align_len = m.half(al) as u32;
                                    m.put_byte(align.wrapping_add(align_len), 0);
                                    if debug == 1 {
                                        h.call_m(0x1738e8, &[log, tag, 5, 0], m);
                                    }
                                    let mut k = 0u32;
                                    while k < align_len {
                                        let c = m.byte(align.wrapping_add(k));
                                        if c == b'~' {
                                            m.put_byte(align.wrapping_add(k), b' ');
                                        } else if c != b' ' && (c == b'N' || c == b'L') {
                                            if k != 0 && m.byte(text.wrapping_add(k - 1)) != b' ' {
                                                let mut j = k;
                                                while j != 0 && m.byte(text.wrapping_add(j)) != b' ' {
                                                    j -= 1;
                                                }
                                                if m.byte(text.wrapping_add(j)) == b' '
                                                    && j + 1 < align_len
                                                    && m.byte(text.wrapping_add(j + 1)) != b' '
                                                {
                                                    m.put_byte(align.wrapping_add(j + 1), c);
                                                    m.put_byte(align.wrapping_add(k), b' ');
                                                    k = j;
                                                }
                                                if j == 0 && m.byte(align) != b' ' {
                                                    if debug == 1 {
                                                        h.call_m(0x1738e8, &[log, tag, 5, 0], m);
                                                    }
                                                }
                                            }
                                        } else if c != b' ' && c != b'N' && c != b'L' {
                                            m.put_byte(align.wrapping_add(k), b' ');
                                        }
                                        k += 1;
                                    }
                                    rc = fn_24d068_m(m, this, in_out, text, h);
                                    if rc < 0x80000000 {
                                        rc = fn_24d068_m(m, this, out_map, align, h);
                                    }
                                    if rc < 0x80000000 && map_words == 1 {
                                        let markers = m.word(mp);
                                        let mcount = m.half(mc) as u32;
                                        let mut cnt = 0u32;
                                        let mut i2 = 0u32;
                                        while i2 < mcount {
                                            if m.byte(markers.wrapping_add(i2)) != 0 {
                                                if i2 > text_len
                                                    || (i2 < text_len
                                                        && m.byte(text.wrapping_add(i2)) != b' ')
                                                {
                                                    if debug == 1 {
                                                        h.call_m(0x1738e8, &[log, tag, 5, 0], m);
                                                    }
                                                } else {
                                                    cnt += 1;
                                                }
                                            }
                                            i2 += 1;
                                        }
                                        let positions = m.word(this.wrapping_add(0xb8));
                                        let pcount = m.half(this.wrapping_add(0xc4)) as u32;
                                        m.put_half(positions.wrapping_add(word * 2), in_off);
                                        if cnt != 0 {
                                            let mut w = 0u32;
                                            let mut j2 = 0u32;
                                            let mut last = 0u32;
                                            while j2 < mcount {
                                                if m.byte(markers.wrapping_add(j2)) != 0 {
                                                    if j2 <= text_len
                                                        && (j2 >= text_len
                                                            || m.byte(text.wrapping_add(j2)) == b' ')
                                                    {
                                                        last = j2;
                                                    }
                                                    if word + w + 1 < pcount {
                                                        m.put_half(
                                                            positions.wrapping_add(
                                                                word.wrapping_add(w + 1) * 2,
                                                            ),
                                                            in_off.wrapping_add(last as u16),
                                                        );
                                                        if debug == 1 {
                                                            h.call_m(
                                                                0x1738e8,
                                                                &[log, tag, 5, 0],
                                                                m,
                                                            );
                                                        }
                                                    }
                                                    w += 1;
                                                }
                                                j2 += 1;
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    if debug == 1 {
        h.call_m(0x1738e8, &[log, tag, 5, 0], m);
    }
    m.put_byte(done, 1);
    for (i, v) in sv.iter().enumerate() {
        m.put_word(fb.wrapping_add(i as u32 * 4), *v);
    }
    rc
}

/// True when the guest string is null, "NULL" or empty.
fn null_or_empty(m: &Memory<'_>, s: u32) -> bool {
    if s == 0 {
        return true;
    }
    // Compare against the "NULL" pool word without naming an address: the
    // ARM compares with fn_195d58(s, "NULL") and fn_195d58(s, "").
    if m.byte(s) == 0 {
        return true;
    }
    if m.byte(s) == b'N'
        && m.byte(s.wrapping_add(1)) == b'U'
        && m.byte(s.wrapping_add(2)) == b'L'
        && m.byte(s.wrapping_add(3)) == b'L'
        && m.byte(s.wrapping_add(4)) == 0
    {
        return true;
    }
    false
}

/// fn_2b4260. Apply one expansion node: recurse into subexpansions, run
/// the type callback or the depes layer function, append into text/alignment.
/// Shells are 12 bytes; the frame lives in the FRAME slice (saved/restored).
/// Heap alloc/free order mirrors the ROM exactly (matched traffic).
/// `depth` selects the FRAME slice (0x1100 + depth*96): each nested
/// invocation needs its own shells, the way the ARM uses a fresh stack
/// frame per call. Dispatch passes 0; recursion passes depth + 1.
#[allow(clippy::too_many_arguments)]
pub fn fn_2b4260_m(
    m: &mut Memory<'_>,
    owner: u32,
    node: u32,
    text: u32,
    alignment: u32,
    h: &mut impl Host,
    depth: u32,
) -> u32 {
    let ctx = m.word(owner);
    let log = m.word(ctx.wrapping_add(16));
    let tag = m.word(owner.wrapping_add(0x48));
    h.call_m(0x1738e8, &[log, tag, 4, 0], m);
    // Frame: child_text, child_alignment, function_alignment, expanded,
    // expanded_alignment, output (12 bytes each), callback word, changed.
    let fb = 0x1100u32.wrapping_add(depth.wrapping_mul(96));
    let mut sv = [0u32; 20];
    for (i, v) in sv.iter_mut().enumerate() {
        *v = m.word(fb.wrapping_add(i as u32 * 4));
    }
    let child_text = fb;
    let child_alignment = fb.wrapping_add(12);
    let function_alignment = fb.wrapping_add(24);
    let expanded = fb.wrapping_add(36);
    let expanded_alignment = fb.wrapping_add(48);
    let output = fb.wrapping_add(60);
    let cb_slot = fb.wrapping_add(72);
    let changed = fb.wrapping_add(76);
    for b in [child_text, child_alignment, function_alignment, expanded, expanded_alignment, output] {
        s_set(m, b, 0, 12);
    }
    m.put_byte(changed, 0);
    // Pre-loop allocs return directly on failure (no cleanup), like the ROM.
    let mut rc = fn_24cea0_m(m, owner, child_text, h);
    if rc >= 0x80000000 {
        for (i, v) in sv.iter().enumerate() {
            m.put_word(fb.wrapping_add(i as u32 * 4), *v);
        }
        return rc;
    }
    rc = fn_24cea0_m(m, owner, child_alignment, h);
    if rc >= 0x80000000 {
        for (i, v) in sv.iter().enumerate() {
            m.put_word(fb.wrapping_add(i as u32 * 4), *v);
        }
        return rc;
    }
    rc = fn_24cea0_m(m, owner, function_alignment, h);
    if rc >= 0x80000000 {
        for (i, v) in sv.iter().enumerate() {
            m.put_word(fb.wrapping_add(i as u32 * 4), *v);
        }
        return rc;
    }
    rc = fn_24cea0_m(m, owner, expanded, h);
    if rc >= 0x80000000 {
        for (i, v) in sv.iter().enumerate() {
            m.put_word(fb.wrapping_add(i as u32 * 4), *v);
        }
        return rc;
    }
    rc = fn_24cea0_m(m, owner, expanded_alignment, h);
    if rc >= 0x80000000 {
        for (i, v) in sv.iter().enumerate() {
            m.put_word(fb.wrapping_add(i as u32 * 4), *v);
        }
        return rc;
    }
    rc = 0;
    {
        let state = m.word(owner.wrapping_add(0x64));
        let nchild = m.byte(node.wrapping_add(12)) as u32;
        let children = m.word(node.wrapping_add(8));
        let mut i = 0u32;
        while i < nchild {
            h.call_m(0x1738e8, &[log, tag, 5, 0], m);
            let child = m.word(children.wrapping_add(i * 4));
            rc = fn_2b4260_m(m, owner, child, child_text, child_alignment, h, depth.wrapping_add(1));
            if rc >= 0x80000000 {
                break;
            }
            if m.half(child_text.wrapping_add(6)) != 0 {
                h.call_m(0x1738e8, &[log, tag, 5, 0], m);
                if m.half(child_text.wrapping_add(6)) != 0 {
                    let ntype = m.byte(m.word(node));
                    if ntype == 0 {
                        let off = m.half(state.wrapping_add(16)) as u32;
                        let add = s_len(m, m.word(child_text));
                        m.put_half(state.wrapping_add(16), off.wrapping_add(add) as u16);
                    }
                    rc = fn_24d13c_m(m, owner, expanded, child_text, h);
                    if rc >= 0x80000000 {
                        break;
                    }
                    rc = fn_24d13c_m(m, owner, expanded_alignment, child_alignment, h);
                    if rc >= 0x80000000 {
                        break;
                    }
                    m.put_half(child_text.wrapping_add(6), 0);
                    m.put_byte(m.word(child_text), 0);
                    m.put_half(child_alignment.wrapping_add(6), 0);
                    m.put_byte(m.word(child_alignment), 0);
                }
            }
            i += 1;
        }
        if rc < 0x80000000 {
            fn_24d030_m(m, ctx, child_text, h);
            fn_24d030_m(m, ctx, child_alignment, h);
            fn_24d030_m(m, ctx, function_alignment, h);
            let ntype = m.byte(m.word(node));
            if m.half(expanded.wrapping_add(6)) != 0 && (ntype == 2 || ntype == 1 || ntype == 3) {
                h.call_m(0x1738e8, &[log, tag, 5, 0], m);
            }
            rc = fn_24cea0_m(m, owner, output, h);
            if rc < 0x80000000 {
                if ntype == 2 || ntype == 1 {
                    let ntype_ptr = m.word(node);
                    rc = fn_24d8c8_m(m, owner, 1, ntype_ptr, cb_slot, h);
                    if rc < 0x80000000 {
                        let cb = m.word(cb_slot);
                        m.put_word(state, m.word(node.wrapping_add(20)));
                        m.put_word(state.wrapping_add(4), output);
                        m.put_half(state.wrapping_add(18), m.half(node.wrapping_add(4)));
                        let handled = fb.wrapping_add(77);
                        m.put_byte(handled, 0);
                        rc = h.call_m(cb, &[owner, handled], m);
                        if rc < 0x80000000 && m.byte(handled) != 0 {
                            rc = fn_24d13c_m(m, owner, text, output, h);
                            if rc < 0x80000000 {
                                s_set(m, m.word(output), 0x20, m.half(output.wrapping_add(6)) as u32);
                                rc = fn_24d13c_m(m, owner, alignment, output, h);
                            }
                        }
                    }
                } else if ntype == 3 {
                    let mut end = 9999u32;
                    let mut spaces = 0u16;
                    let positions = m.word(state.wrapping_add(12));
                    let pcount = m.byte(positions.wrapping_add(82)) as u32;
                    let mut j = 0u32;
                    while j < nchild {
                        let child = m.word(children.wrapping_add(j * 4));
                        if m.byte(m.word(child)) == 2 {
                            let idx = m.half(child.wrapping_add(4)) as u32;
                            if idx > pcount {
                                h.call_m(0x173dc0, &[log, tag, 0x5dc8, 0], m);
                                let _ = e_code(0);
                            }
                            if idx == 0 {
                                end = m.half(m.word(state.wrapping_add(8))) as u32;
                            } else if m.half(positions.wrapping_add(42).wrapping_add(idx.wrapping_sub(1) * 4)) as i16 != -1
                                && end > m.half(positions.wrapping_add(42).wrapping_add(idx.wrapping_sub(1) * 4)) as u32
                            {
                                end = m.half(positions.wrapping_add(42).wrapping_add(idx.wrapping_sub(1) * 4)) as u32;
                            }
                        }
                        j += 1;
                    }
                    let mut found = 0u32;
                    let bcount = m.half(owner.wrapping_add(0xc4)) as u32;
                    if bcount != 0 {
                        let mut boundary = m.word(owner.wrapping_add(0xc0));
                        let mut c = bcount;
                        while c != 0 {
                            if m.half(boundary) as u32 == end {
                                found = 1;
                            }
                            boundary = boundary.wrapping_add(2);
                            c -= 1;
                        }
                        if found == 1 {
                            let mut pos = m.half(m.word(state.wrapping_add(8)));
                            let textp = m.word(owner.wrapping_add(0x5c));
                            while (pos as u32) < end && m.byte(textp.wrapping_add(pos as u32)) == b' ' {
                                pos = pos.wrapping_add(1);
                            }
                            let mut p2 = pos;
                            while (p2 as u32) < end {
                                if m.byte(textp.wrapping_add(p2 as u32)) == b' ' {
                                    spaces = spaces.wrapping_add(1);
                                    while (p2 as u32) < end
                                        && m.byte(textp.wrapping_add(p2 as u32)) == b' '
                                    {
                                        p2 = p2.wrapping_add(1);
                                    }
                                } else {
                                    p2 = p2.wrapping_add(1);
                                }
                            }
                        }
                    }
                    m.put_word(state, m.word(expanded));
                    m.put_word(state.wrapping_add(4), output);
                    m.put_byte(changed, 0);
                    rc = fn_2e4e80_m(
                        m,
                        owner,
                        m.word(node.wrapping_add(24)),
                        spaces as u32,
                        found,
                        changed,
                        function_alignment,
                        h,
                    );
                    if rc < 0x80000000 && m.byte(changed) != 0 {
                        rc = fn_24d13c_m(m, owner, text, output, h);
                        if rc < 0x80000000 {
                            rc = fn_24d13c_m(m, owner, alignment, function_alignment, h);
                        }
                    }
                }
                if rc < 0x80000000 {
                    fn_24d030_m(m, ctx, output, h);
                    if ntype == 0 {
                        rc = fn_24d13c_m(m, owner, text, expanded, h);
                        if rc < 0x80000000 {
                            rc = fn_24d13c_m(m, owner, alignment, expanded_alignment, h);
                        }
                    }
                }
            }
        }
    }
    fn_24d030_m(m, ctx, function_alignment, h);
    fn_24d030_m(m, ctx, output, h);
    fn_24d030_m(m, ctx, expanded, h);
    fn_24d030_m(m, ctx, expanded_alignment, h);
    fn_24d030_m(m, ctx, child_text, h);
    fn_24d030_m(m, ctx, child_alignment, h);
    h.call_m(0x1738e8, &[log, tag, 4, 0], m);
    for (i, v) in sv.iter().enumerate() {
        m.put_word(fb.wrapping_add(i as u32 * 4), *v);
    }
    rc
}

/// fn_2b570c. _MATCH rule callback: append the matched text or slice the
/// input range, then map positions. 1:1 from src/fn_2b570c.c.
pub fn fn_2b570c_m(
    m: &mut Memory<'_>,
    p: u32,
    out: u32,
    h: &mut impl Host,
) -> u32 {
    fn find_match(m: &Memory<'_>, table: u32, index: u32, found: &mut u32) -> u8 {
        let count = m.half(table.wrapping_add(40)) as u32;
        let mut i = 0u32;
        while i < count {
            if m.half(table.wrapping_add(i.wrapping_mul(4))) as u32 == index {
                *found = i;
                return 1;
            }
            i += 1;
        }
        0
    }
    fn is_alnum(ch: u32) -> bool {
        ch.wrapping_sub(b'a' as u32) <= (b'z' - b'a') as u32
            || ch.wrapping_sub(b'A' as u32) <= (b'Z' - b'A') as u32
            || ch.wrapping_sub(b'0' as u32) <= 9
    }
    fn is_punct(ch: u32) -> bool {
        ch.wrapping_sub(b'!' as u32) < 15
            || ch.wrapping_sub(b':' as u32) < 7
            || ch.wrapping_sub(b'[' as u32) < 6
            || ch.wrapping_sub(b'{' as u32) < 4
    }
    fn map_position(m: &mut Memory<'_>, p: u32, input: u16, output: u16) {
        let in_pos = m.word(p.wrapping_add(0xc0));
        let out_pos = m.word(p.wrapping_add(0xb8));
        let args = m.word(p.wrapping_add(0x64));
        let count = m.half(p.wrapping_add(0xc4)) as u32;
        let mut i = 0u32;
        while i < count {
            if m.half(in_pos.wrapping_add(i.wrapping_mul(2))) == input {
                m.put_half(
                    out_pos.wrapping_add(i.wrapping_mul(2)),
                    m.half(args.wrapping_add(16)).wrapping_add(output),
                );
                break;
            }
            i += 1;
        }
    }
    let ctx = m.word(p);
    let log = m.word(ctx.wrapping_add(16));
    let tag = m.word(p.wrapping_add(0x48));
    let trace = m.byte(p.wrapping_add(0xc6));
    let args = m.word(p.wrapping_add(0x64));
    let mut found = 0u32;
    let mut result = 0u32;
    m.put_byte(out, 0);
    if trace == 1 {
        h.call_m(0x1738e8, &[log, tag, 5, 0], m);
    }
    let index = m.half(args.wrapping_add(18)) as u32;
    let output = m.word(args.wrapping_add(4));
    let table = m.word(args.wrapping_add(12));
    if index == 0 {
        if find_match(m, table, 0, &mut found) == 1 {
            let s = m.word(p.wrapping_add(0xac)).wrapping_add(
                m.half(table.wrapping_add(found.wrapping_mul(4)).wrapping_add(2)) as u32,
            );
            result = fn_24d068_m(m, p, output, s, h);
            if result >= 0x80000000 {
                return result;
            }
        } else {
            let range = m.word(args.wrapping_add(8));
            result = fn_24d204_m(
                m,
                p,
                output,
                m.half(range) as u16,
                m.half(range.wrapping_add(2)) as u16,
                h,
            );
            if result >= 0x80000000 {
                return result;
            }
        }
    } else {
        let idx = index.wrapping_sub(1);
        if idx > m.byte(table.wrapping_add(82)) as u32 {
            h.call_m(0x173dc0, &[log, tag, 0x5dc8, 0], m);
            m.put_byte(out, 0);
            return result;
        }
        let range = table.wrapping_add(42).wrapping_add(idx.wrapping_mul(4));
        let r0 = m.half(range) as i16 as i32;
        let r1 = m.half(range.wrapping_add(2)) as i16 as i32;
        if r0 == -1 && r1 == -1 {
            result = fn_24d068_m(m, p, output, 0, h);
            if result >= 0x80000000 {
                return result;
            }
        } else {
            let text: u32;
            if find_match(m, table, idx.wrapping_add(1), &mut found) == 1 {
                let s = m.word(p.wrapping_add(0xac)).wrapping_add(
                    m.half(table.wrapping_add(found.wrapping_mul(4)).wrapping_add(2)) as u32,
                );
                result = fn_24d068_m(m, p, output, s, h);
                if result >= 0x80000000 {
                    return result;
                }
                text = s;
            } else {
                let arange = m.word(args.wrapping_add(8));
                let a0 = m.half(arange) as u32;
                let a1 = m.half(arange.wrapping_add(2)) as u32;
                let scratch = m.word(p.wrapping_add(0xc8));
                let input = m.word(p.wrapping_add(0x5c));
                s_copy(m, scratch, input.wrapping_add(a0), a1.wrapping_sub(a0));
                m.put_byte(scratch.wrapping_add(a1.wrapping_sub(a0)), 0);
                text = scratch;
                result = fn_24d204_m(m, p, output, r0 as u16, r1 as u16, h);
                if result >= 0x80000000 {
                    return result;
                }
            }
            let length = s_len(m, text);
            let mut i = 0u32;
            while i < length {
                if i != 0 {
                    if m.byte(text.wrapping_add(i)) != b' ' {
                        i += 1;
                        continue;
                    }
                    while i < length
                        && m.byte(text.wrapping_add(i)) == b' '
                        && i + 1 < length
                        && m.byte(text.wrapping_add(i + 1)) == b' '
                    {
                        i += 1;
                    }
                }
                let mut input = (r0 + i as i32) as u16;
                map_position(m, p, input, i as u16);
                let in_text = m.word(p.wrapping_add(0x5c));
                let in_len = m.half(p.wrapping_add(0x60)) as u32;
                if input != 0
                    && (input as u32) < in_len
                    && is_alnum(m.byte(in_text.wrapping_add(input as u32)) as u32)
                    && is_punct(m.byte(in_text.wrapping_add(input as u32 - 1)) as u32)
                {
                    input = input.wrapping_sub(1);
                    while input > 0
                        && is_punct(m.byte(in_text.wrapping_add(input as u32)) as u32)
                        && is_punct(m.byte(in_text.wrapping_add(input as u32 - 1)) as u32)
                    {
                        input = input.wrapping_sub(1);
                    }
                    if m.byte(in_text.wrapping_add(input as u32)) == b' '
                        || (m.byte(in_text.wrapping_add(input as u32)) as u32).wrapping_sub(9) <= 4
                    {
                        input = input.wrapping_add(1);
                    }
                    map_position(m, p, input, i as u16);
                }
                i += 1;
            }
        }
    }
    if trace == 1 {
        h.call_m(0x1738e8, &[log, tag, 5, 0], m);
    }
    m.put_byte(out, 1);
    result
}

/// fn_2190b4. Depes setMarkers: copy the markers buffer, zero the tail.
/// Slots verified on ROM: buf +0x2c, total +0x32, limit +0x34 (s16).
pub fn fn_2190b4_m(
    m: &mut Memory<'_>,
    p: u32,
    a: u32,
    dst: u32,
    mut len: u32,
    h: &mut impl Host,
) -> u32 {
    let rc = h.call_m(0x176f94, &[p, a, 0xf37e, 0x40], m);
    if rc >= 0x80000000 {
        return 0x89702008;
    }
    let ctx = m.word(p);
    let log = m.word(ctx.wrapping_add(16));
    h.call_m(0x1738e8, &[log, 0x8fa16c, 4, 0], m);
    if len > 10000 {
        len = 10000;
    }
    let limit = m.half(p.wrapping_add(0x34)) as i16 as i32;
    if dst == 0 || (len as i32) > limit {
        h.call_m(0x173dc0, &[log, 0x8fa16c, 0xa411, 0], m);
        return 0x89702007;
    }
    let buf = m.word(p.wrapping_add(0x2c));
    let total = m.half(p.wrapping_add(0x32)) as u32;
    s_copy(m, buf, dst, len);
    s_set(m, buf.wrapping_add(len), 0, total.wrapping_sub(len));
    h.call_m(0x1738e8, &[log, 0x8fa16c, 4, 0], m);
    rc
}

/// fn_2192c0. Depes getMarkers: return the markers buffer and length.
/// Slots verified on ROM: marker +0x2c, len +0x34.
pub fn fn_2192c0_m(
    m: &mut Memory<'_>,
    p: u32,
    a: u32,
    out: u32,
    out_len: u32,
    h: &mut impl Host,
) -> u32 {
    let rc = h.call_m(0x176f94, &[p, a, 0xf37e, 0x40], m);
    if rc >= 0x80000000 {
        return 0x89702008;
    }
    let ctx = m.word(p);
    let log = m.word(ctx.wrapping_add(16));
    h.call_m(0x1738e8, &[log, 0x8fa16c, 4, 0], m);
    if out == 0 {
        h.call_m(0x173dc0, &[log, 0x8fa16c, 0xa411, 0], m);
        return 0x89702007;
    }
    m.put_half(out_len, m.half(p.wrapping_add(0x34)));
    m.put_word(out, m.word(p.wrapping_add(0x2c)));
    h.call_m(0x1738e8, &[log, 0x8fa16c, 4, 0], m);
    rc
}
