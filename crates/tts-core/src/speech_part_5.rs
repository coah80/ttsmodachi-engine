
use crate::boot::Memory;

pub trait Host: crate::boot_part_f::Host + crate::log_live::LogMethodsM {
    fn scratch(&self) -> u32;

    fn category(&self, us: u32) -> u32 {
        us
    }

    fn thread_id(&mut self) -> u32 {
        0
    }
    fn format(&self, callsite: u32) -> u32;
    fn dictionary_entries(
        &mut self,
        m: &mut Memory<'_>,
        dictionary: u32,
        entries: &mut u32,
        count: &mut u32,
    ) -> u32;
    fn record(
        &mut self,
        m: &mut Memory<'_>,
        record: u32,
        arg: u32,
        buffer: &mut RecordBuffer,
        found: &mut u8,
    ) -> u32;
    fn entries(
        &mut self,
        m: &mut Memory<'_>,
        a: u32,
        b: u32,
        c: u32,
        array: &mut u32,
        count: &mut u32,
    ) -> u32;
}
fn at(p: u32, i: i32) -> u32 {
    p.wrapping_add((i as u32).wrapping_mul(4))
}
fn get(m: &Memory<'_>, p: u32) -> i32 {
    m.word(p) as i32
}
fn asr(x: i32, n: u32) -> i32 {
    if n == 0 {
        x
    } else if n >= 32 {
        x >> 31
    } else {
        x >> n
    }
}
fn lsl(x: i32, n: u32) -> i32 {
    if n >= 32 {
        0
    } else {
        x.wrapping_shl(n)
    }
}
fn elem(m: &Memory<'_>, table: u32, j: i32, i: i32) -> u32 {
    at(m.word(at(table, j)), i)
}

pub fn fn_1b4238(m: &mut Memory<'_>, _msg: u32) -> u32 {
    let _ = m;
    3
}

pub fn fn_2a178c(m: &mut Memory<'_>, p: u32, g1: u32, g2: u32, _arg: u32) {
    let mut i = 0i32;
    let mut cursor = m.word(p + 0x40);
    if get(m, p + 0x20) <= 0 {
        return;
    }
    let sh2 = g1.wrapping_sub(g2 >> 1).wrapping_sub(1);
    let sh = g1 >> 1;
    let ds = g1.wrapping_sub(g2);
    loop {
        let raw = get(m, elem(m, m.word(p + 0x34), 0, i));
        let braw = get(m, elem(m, m.word(p + 0x30), 0, i));
        m.put_word(cursor, asr(raw, sh).wrapping_mul(asr(braw, sh)) as u32);
        let row = m.word(at(m.word(p + 0x3c), i));
        m.put_word(row + 4, 0);
        m.put_word(row, asr(raw, ds) as u32);
        m.put_word(row + 8, 0);
        let mut j = 1;
        while get(m, m.word(p + 0x28) + 0x18) > j {
            let range = m.word(at(m.word(m.word(p + 0x28) + 0x1c), j));
            let mut t = get(m, range);
            while get(m, range + 4).wrapping_add(1) > t {
                let k = i.wrapping_add(t);
                if k >= 0 && get(m, p + 0x20) > k {
                    let bank = m.word(p + 0x28);
                    let c = get(m, elem(m, m.word(bank + 0x20), j, t.wrapping_neg()));
                    if c != 0 {
                        let prev = get(m, cursor);
                        let s = asr(get(m, elem(m, m.word(p + 0x34), j, k)), sh);
                        let v = asr(get(m, elem(m, m.word(p + 0x30), j, k)), sh);
                        let u = t.wrapping_neg();
                        let q = asr((c >> 1).wrapping_mul(s), sh2);
                        m.put_word(
                            cursor,
                            c.wrapping_mul(asr(s.wrapping_mul(v), sh))
                                .wrapping_add(prev) as u32,
                        );
                        let coeff = m.word(bank + 0x24);
                        m.put_word(
                            row,
                            get(m, row).wrapping_add(get(m, elem(m, coeff, j, u)).wrapping_mul(q))
                                as u32,
                        );
                        if get(m, range + 4) >= u.wrapping_add(1) && get(m, p + 0x20) > i + 1 {
                            m.put_word(
                                row + 4,
                                get(m, row + 4)
                                    .wrapping_add(get(m, elem(m, coeff, j, u + 1)).wrapping_mul(q))
                                    as u32,
                            );
                        }
                        if get(m, range + 4) >= u.wrapping_add(2) && get(m, p + 0x20) > i + 2 {
                            m.put_word(
                                row + 8,
                                get(m, row + 8)
                                    .wrapping_add(get(m, elem(m, coeff, j, u + 2)).wrapping_mul(q))
                                    as u32,
                            );
                        }
                    }
                }
                t = t.wrapping_add(1);
            }
            j += 1;
        }
        i += 1;
        cursor = cursor.wrapping_add(4);
        if get(m, p + 0x20) <= i {
            break;
        }
    }
}
fn divide(a: i32, b: i32) -> i32 {
    crate::runtime::signed_division(a, b)
        .expect("ARM divide by zero")
        .quotient
}

pub fn fn_2a19ac(m: &mut Memory<'_>, context: u32, p: u32, n: u32, host: &mut impl Host) {
    let mut k = 0u32;
    let first = elem(m, m.word(p + 0x3c), 0, 0);
    while get(m, first) < 0x1fffffff {
        m.put_word(first, m.word(first).wrapping_shl(2));
        k = k.wrapping_add(2);
    }
    let s = n.wrapping_sub(k) >> 1;
    m.put_word(
        first,
        lsl(crate::selection::prediction_square_root(m.word(first)), s) as u32,
    );
    let h = n >> 1;
    for i in 0..get(m, p + 0x24).wrapping_sub(1) {
        let a = elem(m, m.word(p + 0x3c), 0, i + 1);
        m.put_word(a, lsl(divide(get(m, a), asr(get(m, first), h)), h) as u32);
    }
    for j in 1..get(m, p + 0x20) {
        for k in 1..get(m, p + 0x24) {
            let d = j - k;
            if d >= 0 {
                let v = asr(get(m, elem(m, m.word(p + 0x3c), d, k)), h);
                let a = elem(m, m.word(p + 0x3c), j, 0);
                m.put_word(a, get(m, a).wrapping_sub(v.wrapping_mul(v)) as u32);
            }
        }
        let a = elem(m, m.word(p + 0x3c), j, 0);
        if get(m, a) <= 0 {
            let category = host.category(0x8fa510);
            log(m, host, m.word(context + 16), category, 0, 0x2a19ac, &[]);
            return;
        }
        let mut k = 0u32;
        while get(m, a) < 0x1fffffff {
            m.put_word(a, m.word(a).wrapping_shl(2));
            k = k.wrapping_add(2);
        }
        m.put_word(
            a,
            lsl(
                crate::selection::prediction_square_root(m.word(a)),
                n.wrapping_sub(k) >> 1,
            ) as u32,
        );
        for col in 1..get(m, p + 0x24) {
            for l in 0..get(m, m.word(p + 0x28) + 0x30) {
                if col == get(m, p + 0x24) - 1 {
                    continue;
                }
                let r = m.word(at(m.word(p + 0x3c), j - l - 1));
                let b = elem(m, m.word(p + 0x3c), j, col);
                let v = asr(get(m, at(r, col - l)), h).wrapping_mul(asr(get(m, at(r, col + 1)), h));
                m.put_word(b, get(m, b).wrapping_sub(v) as u32);
            }
            let b = elem(m, m.word(p + 0x3c), j, col);
            m.put_word(b, lsl(divide(get(m, b), asr(get(m, a), h)), h) as u32);
        }
    }
}

pub fn fn_2e1fc8(m: &mut Memory<'_>, a: u32, b: u32, c: u32, d: u32, h: &mut impl Host) -> u32 {
    let r = h.call(m, 0x2fdc84, &[a, b, c, d]);
    if r == 0xfffffffe {
        0
    } else {
        r
    }
}

pub fn fn_2fdc84_m(
    m: &mut Memory<'_>,
    dst: u32,
    limit: u32,
    spec: u32,
    text: u32,
    h: &mut impl crate::boot_part_e::Host,
) -> u32 {
    fn strlen(m: &Memory<'_>, mut s: u32) -> u32 {
        let mut n = 0u32;
        while m.byte(s) != 0 {
            n = n.wrapping_add(1);
            s = s.wrapping_add(1);
        }
        n
    }

    fn first_mark(m: &Memory<'_>, mut s: u32) -> u32 {
        while m.byte(s) != 0 {
            if m.byte(s) == 0x06 {
                return s;
            }
            s = s.wrapping_add(1);
        }
        0
    }

    fn atoi_seq(m: &Memory<'_>, seq: &[u8]) -> i32 {
        let mut s = 0usize;
        while (m.half(0x8dc30c + ((seq[s] as u32) & 0xff) * 2) as u32) & 8 != 0 {
            s += 1;
        }
        let mut n = 0i32;
        let mut c = seq[s] as u32;
        s += 1;
        let sign = c;
        if c == 0x2d || c == 0x2b {
            c = seq[s] as u32;
            s += 1;
        }
        while (m.half(0x8dc30c + (c & 0xff) * 2) as u32) & 4 != 0 {
            n = (c as i32)
                .wrapping_add(n.wrapping_mul(10))
                .wrapping_sub(0x30);
            c = seq[s] as u32;
            s += 1;
        }
        if sign == 0x2d {
            n.wrapping_neg()
        } else {
            n
        }
    }
    let len = strlen(m, spec);
    if len.wrapping_sub(1) >= 12 {
        return 0;
    }
    let last = m.byte(spec.wrapping_add(len).wrapping_sub(1));
    let mut width = 0i32;
    let found = first_mark(m, spec);
    if found != 0 {

        let n = spec
            .wrapping_add(len)
            .wrapping_sub(found)
            .wrapping_sub(1);
        let mut stack = [0u8; 16];
        let mut i = 0u32;
        while i < n {
            stack[i as usize] = m.byte(found.wrapping_add(i));
            i = i.wrapping_add(1);
        }
        stack[n as usize] = 0;
        width = atoi_seq(m, &stack);
    }

    let f0 = m.byte(spec);
    let mut fill = 0x20u32;
    if f0 == 0x2d || f0 == 0x2b || f0 == 0x30 || f0 == 0x20 || f0 == 0x23 {
        fill = f0 as u32;
    }
    if last == b'p' {
        return h.call(m, 0x2fddec, &[0, 0x30, text, 0x10, 1, 8, dst, limit]);
    }
    if last != b's' {
        return 0;
    }
    if fill != 0x20 {
        if fill != 0x30 {
            return 0;
        }
        fill = 0x30;
    }
    let mut total = strlen(m, text);
    let mut count = 0u32;
    if total < width as u32 {
        count = (width as u32).wrapping_sub(total);
        total = total.wrapping_add(count);
    }
    if dst == 0 {
        return total;
    }
    if limit.wrapping_sub(dst) < total {
        return 0xfffffffe;
    }

    let mut i = 0u32;
    while i < count {
        m.put_byte(dst.wrapping_add(i), (fill & 0xff) as u8);
        i = i.wrapping_add(1);
    }
    let mut j = 0u32;
    while m.byte(text.wrapping_add(j)) != 0 {
        m.put_byte(dst.wrapping_add(count).wrapping_add(j), m.byte(text.wrapping_add(j)));
        j = j.wrapping_add(1);
    }
    total
}

pub fn fn_2e3f2c(m: &mut Memory<'_>, p: u32, a: u32, b: u32, h: &mut impl Host) -> u32 {
    if b == 0 {
        return 0;
    }
    if m.byte(p + 0x14) == 1 {
        h.call(m, 0x300a10, &[m.word(p + 0x24), a, b]);
        0
    } else {
        h.call(m, 0x3009a4, &[m.word(p + 0x24), p, a, b])
    }
}

pub fn fn_300a10<H: crate::boot_part_f::Host>(m: &mut Memory<'_>, a: u32, arr: u32, n: u32, h: &mut H) {
    let release_each = m.word(arr) == 1 || m.word(arr) == 3;
    if release_each {
        let span = n.wrapping_mul(32);
        let mut off = 0u32;
        while off < span {
            let f18 = m.word(arr.wrapping_add(off).wrapping_add(24));
            crate::boot_part_f::fn_1b2128(m, a, f18, h);
            off = off.wrapping_add(32);
        }
    }
    crate::boot_part_f::fn_1b2128(m, a, arr, h);
}

pub fn fn_2e453c<H: crate::boot_part_f::Host>(m: &mut Memory<'_>, context: u32, items: u32, h: &mut H) {
    if items == 0 {
        return;
    }
    let heap = m.word(context.wrapping_add(4));

    let count = m.word(items.wrapping_sub(4));
    let mut i = 0u32;
    while i < count {
        let data = m.word(items.wrapping_add(i.wrapping_mul(12)).wrapping_add(4));
        crate::boot_part_f::fn_1b2128(m, heap, data, h);
        i = i.wrapping_add(1);
    }
    crate::boot_part_f::fn_1b2128(m, heap, items.wrapping_sub(4), h);
}

pub fn fn_2b4004<H: crate::boot_part_f::Host>(m: &mut Memory<'_>, obj: u32, resources: u32, h: &mut H) -> u32 {
    let context = m.word(obj.wrapping_add(0x4c));
    fn_2e453c(m, context, resources, h);
    0
}

pub fn fn_2f91f8(
    m: &mut Memory<'_>,
    a: u32,
    b: u32,
    c: u32,
    out: u32,
    out_count: u32,
    h: &mut impl Host,
) -> u32 {
    let mut array = 0;
    let mut count = 0;
    m.put_word(out, 0);
    let rc = h.entries(m, a, b, c, &mut array, &mut count);
    if rc == 0 {
        if count > 16 {
            for i in 16..count {
                crate::boot_part_f::fn_1b2128(m, b, m.word(array + i * 4), h);
            }
            count = 16;
        }
        m.put_word(out, array);
        m.put_word(out_count, count);
    }
    rc
}

fn split_alternatives(
    m: &mut Memory<'_>,
    allocator: u32,
    text: u32,
    out: &mut u32,
    count: &mut u32,
    h: &mut impl crate::boot_part_f::Host,
) -> u8 {
    use crate::fe_text::{strchr_195d30, strlen_195d90};
    let mut error = 0u8;
    let mut start = text;
    let mut scan = text;
    *count = 0;
    let mut bar = strchr_195d30(m.bytes, scan, b'|' as u32);
    while bar != 0 {
        let mut closes = 0u32;
        let mut opens = 0u32;
        let mut open = strchr_195d30(m.bytes, scan, b'(' as u32);
        while open != 0 && open < bar {
            open = strchr_195d30(m.bytes, open.wrapping_add(1), b'(' as u32);
            opens += 1;
        }
        if opens != 0 {
            while closes < opens {
                let mut percent = strchr_195d30(m.bytes, scan.wrapping_add(1), b'%' as u32);
                loop {
                    scan = strchr_195d30(m.bytes, scan.wrapping_add(1), b')' as u32);
                    if scan == 0 || m.bytes[m.offset(scan.wrapping_sub(1))] != b'%' {
                        break;
                    }
                }
                while percent != 0 && percent < scan {
                    if m.bytes[m.offset(percent.wrapping_add(1))] == b'"' {
                        percent = strchr_195d30(m.bytes, percent.wrapping_add(2), b'"' as u32);
                        if percent == 0 {
                            *count = 0;
                            return error;
                        }
                        if percent > scan {
                            scan = strchr_195d30(m.bytes, percent.wrapping_add(1), b')' as u32);
                            while scan != 0 && m.bytes[m.offset(scan.wrapping_sub(1))] == b'%' {
                                scan = strchr_195d30(m.bytes, scan.wrapping_add(1), b')' as u32);
                            }
                        }
                    }
                    percent = strchr_195d30(m.bytes, percent.wrapping_add(1), b'%' as u32);
                }
                if scan == 0 {
                    *count = 0;
                    return error;
                }
                closes += 1;
            }
        }
        if closes == opens {
            if opens == 0 || scan < bar {
                *count += 1;
                *out = crate::boot_part_f::fn_1b218c(m, allocator, *out, *count * 4, &mut error, h);
                if error != 0 {
                    return error;
                }
                let length = bar.wrapping_sub(start) as i32;
                let slot = crate::boot_part_f::fn_1b2140(m, allocator, (length + 2) as u32, &mut error, h);
                if error != 0 {
                    return error;
                }
                for i in 0..length {
                    m.put_byte(slot.wrapping_add(i as u32), m.byte(start.wrapping_add(i as u32)));
                }
                m.put_byte(slot.wrapping_add(length as u32), 0);
                m.put_byte(slot.wrapping_add(length as u32 + 1), 0);
                m.put_word(*out + (*count - 1) * 4, slot);
                start = bar.wrapping_add(1);
            } else {
                scan = scan.wrapping_add(1);
            }
        }
        bar = strchr_195d30(m.bytes, scan, b'|' as u32);
    }

    *count += 1;
    *out = crate::boot_part_f::fn_1b218c(m, allocator, *out, *count * 4, &mut error, h);
    if error == 0 {
        let rest = strlen_195d90(m.bytes, start);
        let slot = crate::boot_part_f::fn_1b2140(m, allocator, (rest + 2) as u32, &mut error, h);
        if error == 0 {
            crate::normalization::strcpy_at(m.bytes, slot, start);
            m.put_byte(slot.wrapping_add(rest as u32 + 1), 0);
        }
    }
    error
}

pub fn fn_2f88d4_m(
    m: &mut Memory<'_>,
    context: u32,
    allocator: u32,
    text: u32,
    out: &mut u32,
    count: &mut u32,
    h: &mut impl crate::boot_part_f::Host,
) -> u8 {
    use crate::fe_text::{strchr_195d30, strlen_195d90};
    let _ = context;
    let mut alternatives = 0u32;
    let mut alternative_count = 0u32;
    let mut error = 0u8;
    let mut written = 0u32;
    *count = 0;
    *out = 0;
    let open = strchr_195d30(m.bytes, text, b'(' as u32);
    if open != 0 {
        let mut inner = open.wrapping_add(1);
        let mut percent = strchr_195d30(m.bytes, open.wrapping_add(2), b'%' as u32);
        let mut close = strchr_195d30(m.bytes, inner, b')' as u32);
        while close != 0 && m.bytes[m.offset(close.wrapping_sub(1))] == b'%' {
            close = strchr_195d30(m.bytes, close.wrapping_add(1), b')' as u32);
        }
        while percent != 0 && percent < close {
            if m.bytes[m.offset(percent.wrapping_add(1))] == b'"' {
                percent = strchr_195d30(m.bytes, percent.wrapping_add(2), b'"' as u32);
                if percent == 0 {
                    return error;
                }
                if percent > close {
                    close = strchr_195d30(m.bytes, percent.wrapping_add(1), b')' as u32);
                    while close != 0 && m.bytes[m.offset(close.wrapping_sub(1))] == b'%' {
                        close = strchr_195d30(m.bytes, close.wrapping_add(1), b')' as u32);
                    }
                }
            }
            percent = strchr_195d30(m.bytes, percent.wrapping_add(1), b'%' as u32);
        }
        if close == 0 {
            return error;
        }
        inner = strchr_195d30(m.bytes, inner, b'(' as u32);
        while inner != 0 && inner < close {
            percent = strchr_195d30(m.bytes, inner.wrapping_add(1), b'%' as u32);
            loop {
                close = strchr_195d30(m.bytes, close.wrapping_add(1), b')' as u32);
                if close == 0 || m.bytes[m.offset(close.wrapping_sub(1))] != b'%' {
                    break;
                }
            }
            while percent != 0 && percent < close {
                if m.bytes[m.offset(percent.wrapping_add(1))] == b'"' {
                    percent = strchr_195d30(m.bytes, percent.wrapping_add(2), b'"' as u32);
                    if percent == 0 {
                        return error;
                    }
                    if percent > close {
                        close = strchr_195d30(m.bytes, percent.wrapping_add(1), b')' as u32);
                        while close != 0 && m.bytes[m.offset(close.wrapping_sub(1))] == b'%' {
                            close = strchr_195d30(m.bytes, close.wrapping_add(1), b')' as u32);
                        }
                    }
                }
                percent = strchr_195d30(m.bytes, percent.wrapping_add(1), b'%' as u32);
            }
            if close == 0 {
                return error;
            }
            inner = strchr_195d30(m.bytes, inner.wrapping_add(1), b'(' as u32);
        }
        let prefix_length = open.wrapping_sub(text) as i32;
        let suffix_length = strlen_195d90(m.bytes, close) - 1;
        let length = close.wrapping_sub(open) as i32;
        let mut contents = crate::boot_part_f::fn_1b2140(m, allocator, length as u32, &mut error, h);
        if error != 0 {
            return error;
        }
        for i in 1..length {
            m.put_byte(contents.wrapping_add(i as u32 - 1), m.byte(open.wrapping_add(i as u32)));
        }
        m.put_byte(contents.wrapping_add(length as u32 - 1), 0);
        error = split_alternatives(m, allocator, contents, &mut alternatives, &mut alternative_count, h);
        if error != 0 {
            return error;
        }
        *out = crate::boot_part_f::fn_1b2140(m, allocator, alternative_count * 4, &mut error, h);
        if error != 0 {
            return error;
        }
        *count = alternative_count;
        for i in 0..alternative_count {
            let part_length = strlen_195d90(m.bytes, m.word(alternatives.wrapping_add(i * 4)));
            let total_length = prefix_length + part_length + suffix_length;
            let mut expanded = crate::boot_part_f::fn_1b2140(m, allocator, (total_length + 2) as u32, &mut error, h);
            if error != 0 {
                return error;
            }
            for j in 0..prefix_length {
                m.put_byte(expanded.wrapping_add(j as u32), m.byte(text.wrapping_add(j as u32)));
            }
            crate::normalization::strcpy_at(m.bytes, expanded.wrapping_add(prefix_length as u32), m.word(alternatives.wrapping_add(i * 4)));
            crate::normalization::strcpy_at(
                m.bytes,
                expanded.wrapping_add(prefix_length as u32 + part_length as u32),
                close.wrapping_add(1),
            );
            m.put_byte(expanded.wrapping_add(total_length as u32 + 1), 0);
            if strchr_195d30(m.bytes, expanded, b'(' as u32) != 0 {
                let mut nested = 0u32;
                let mut nested_count = 0u32;
                error = fn_2f88d4_m(m, context, allocator, expanded, &mut nested, &mut nested_count, h);
                if error != 0 {
                    return error;
                }
                let new_count = nested_count - 1 + *count;
                if new_count <= 16 {
                    *count = new_count;
                    *out = crate::boot_part_f::fn_1b218c(m, allocator, *out, new_count * 4, &mut error, h);
                    if error != 0 {
                        return error;
                    }
                    for j in 0..nested_count {
                        m.put_word(out.wrapping_add((written + j) * 4), m.word(nested.wrapping_add(j * 4)));
                    }
                    written += nested_count;
                } else if *count >= 16 {
                    for j in 0..nested_count {
                        let w = m.word(nested.wrapping_add(j * 4));
                        if w != 0 {
                            crate::boot_part_f::fn_1b2128(m, allocator, w, h);
                            m.put_word(nested.wrapping_add(j * 4), 0);
                        }
                    }
                } else {
                    *out = crate::boot_part_f::fn_1b218c(m, allocator, *out, 64, &mut error, h);
                    if error != 0 {
                        return error;
                    }
                    for j in 0..nested_count {
                        let w = m.word(nested.wrapping_add(j * 4));
                        if written < 16 {
                            m.put_word(out.wrapping_add(written * 4), w);
                            written += 1;
                        } else if w != 0 {
                            crate::boot_part_f::fn_1b2128(m, allocator, w, h);
                            m.put_word(nested.wrapping_add(j * 4), 0);
                        }
                    }
                    *count = 16;
                }
                crate::boot_part_f::fn_1b2128(m, allocator, nested, h);
                crate::boot_part_f::fn_1b2128(m, allocator, expanded, h);
            } else {
                m.put_word(out.wrapping_add(written * 4), expanded);
                written += 1;
            }
        }
        if contents != 0 {
            crate::boot_part_f::fn_1b2128(m, allocator, contents, h);
        }
        if alternatives != 0 {
            for i in 0..alternative_count {
                crate::boot_part_f::fn_1b2128(m, allocator, m.word(alternatives.wrapping_add(i * 4)), h);
            }
            crate::boot_part_f::fn_1b2128(m, allocator, alternatives, h);
        }
        return error;
    }
    if strchr_195d30(m.bytes, text, b'|' as u32) != 0 {
        *count = 0;
        return error;
    }
    *out = crate::boot_part_f::fn_1b2140(m, allocator, 4, &mut error, h);
    if error != 0 {
        return error;
    }
    let len = strlen_195d90(m.bytes, text);
    let slot = crate::boot_part_f::fn_1b2140(m, allocator, (len + 2) as u32, &mut error, h);
    m.put_word(*out, slot);
    if error != 0 {
        return error;
    }
    crate::normalization::strcpy_at(m.bytes, m.word(*out), text);
    m.put_byte(m.word(*out).wrapping_add(len as u32 + 1), 0);
    *count = 1;
    error
}

#[derive(Default)]
pub struct RecordBuffer {
    pub first: u32,
    pub source: u32,
    pub size: u32,
    pub flag: u8,
}
fn log(
    m: &mut Memory<'_>,
    h: &mut impl Host,
    logger: u32,
    category: u32,
    level: u32,
    site: u32,
    args: &[u32],
) -> u32 {
    if logger == 0 {
        return crate::log_live::fn_1738e8(m, 0, category, level, 0, 1, 0, h);
    }
    let scratch = h.scratch();
    let off = m.offset(scratch);
    let mut saved = [0u8; 128];
    assert!(args.len() <= 32);
    saved.copy_from_slice(&m.bytes[off..off + 128]);
    for (i, &arg) in args.iter().enumerate() {
        m.put_word(scratch + i as u32 * 4, arg);
    }
    let format = h.format(site);
    let rc = crate::log_live::fn_1738e8(m, logger, category, level, 0, format, scratch, h);
    m.bytes[off..off + 128].copy_from_slice(&saved);
    rc
}
fn log_state(
    m: &mut Memory<'_>,
    h: &mut impl Host,
    p: u32,
    level: u32,
    site: u32,
    args: &[u32],
) -> u32 {
    let logger = m.word(m.word(p) + 16);
    let category = m.word(p + 72);
    log(m, h, logger, category, level, site, args)
}
struct Formatter<'a, H>(&'a mut H);
impl<H: Host> crate::boot_part_b::Host for Formatter<'_, H> {
    fn call(&mut self, m: &mut Memory<'_>, f: u32, a: &[u32]) -> u32 {
        crate::boot_part_f::Host::call(self.0, m, f, a)
    }
    fn thread_id(&mut self) -> u32 {
        self.0.thread_id()
    }
}
impl<H: Host> crate::boot_part_b::FormatHost for Formatter<'_, H> {}
fn leaf(m: &mut Memory<'_>, f: u32, a: &[u32]) -> u32 {
    let mut arena = crate::selection::arena::Arena::new(m.base, m.bytes);
    crate::selection::arena_leaf::run(&mut arena, f, a)
}
fn cstr<'a>(m: &'a Memory<'_>, p: u32) -> &'a [u8] {
    let off = m.offset(p);
    let n = m.bytes[off..]
        .iter()
        .position(|&b| b == 0)
        .expect("unterminated string");
    &m.bytes[off..off + n + 1]
}

pub fn fn_24e004(m: &mut Memory<'_>, p: u32, label: u32, mode: u32, h: &mut impl Host) -> u32 {
    let collection = m.word(p + 0x44);
    if m.byte(p + 0xc6) == 0 {
        return 0;
    }
    let logger = m.word(m.word(p) + 16);
    let module = m.word(p + 0x48);
    if collection == 0 {
        let scratch = h.scratch();
        let off = m.offset(scratch);
        let mut saved = [0; 20];
        saved.copy_from_slice(&m.bytes[off..off + 20]);
        crate::render_part_a::error_log(
            m,
            &mut Formatter(h),
            logger,
            module,
            0x5df1,
            0,
            0,
            scratch,
        );
        m.bytes[off..off + 20].copy_from_slice(&saved);
        return crate::normalization::error_code(7);
    }

    let scratch = h.scratch() + 128;
    let off = m.offset(scratch);
    let mut saved = [0u8; 12];
    saved.copy_from_slice(&m.bytes[off..off + 12]);
    let description: &[u8] = if mode == 1 { b"<LOGFE> \0" } else { b"\0" };
    let mut local = [0u8; 24];
    local[12..12 + description.len()].copy_from_slice(description);
    crate::selection::arena_leaf::copy_string(
        &mut crate::selection::arena::Arena::new(0, &mut local),
        0,
        12,
    );
    m.bytes[off..off + description.len()].copy_from_slice(&local[..description.len()]);
    log_state(m, h, p, 5, 0x24e004, &[scratch, label]);
    if half(m, collection + 8) != 0 && m.word(collection + 4) != 0 {
        let mut node = m.word(collection + 4);
        while node != 0 {
            h.call(m, 0x24dcf0, &[p, node + 12, node + 8, mode]);
            node = m.word(node);
        }
    }
    log_state(m, h, p, 5, 0x24e100, &[scratch, 0]);
    m.bytes[off..off + 12].copy_from_slice(&saved);
    0
}
fn half(m: &Memory<'_>, p: u32) -> u16 {
    let i = m.offset(p);
    u16::from_le_bytes(m.bytes[i..i + 2].try_into().unwrap())
}

pub fn fn_2b24e8(m: &mut Memory<'_>, p: u32, h: &mut impl Host) -> u32 {
    let mut entries = 0;
    let mut count = 0;
    let rc = h.dictionary_entries(m, m.word(p + 60), &mut entries, &mut count) as u8;
    if rc != 0 {
        return rc as u32;
    }
    for index in 0..count {
        for field in 0..3u32 {
            let source = m.word(entries + index * 20 + 8 + field * 4);
            if source == 0 {
                continue;
            }
            if field == 0 && !crate::pronunciation::valid_language_list(Some(cstr(m, source))) {
                return crate::normalization::code_4(0);
            }
            let output = m.word(p + 68) + field * 4;
            let target = m.word(output);
            if target != 0 {
                let rc = if field == 0 {
                    let n = leaf(m, 0x195d90, &[target]);
                    leaf(m, 0x195df8, &[target, source, n])
                } else {
                    leaf(m, 0x195d58, &[target, source])
                };
                if rc != 0 {
                    return crate::normalization::code_4(0);
                }
            } else {
                let n = leaf(m, 0x195d90, &[source]);
                let mut error = 0;
                let ptr = crate::boot_part_f::fn_1b2140(
                    m,
                    m.word(p + 56),
                    n.wrapping_add(1),
                    &mut error,
                    h,
                );
                m.put_word(output, ptr);
                if error != 0 {
                    return error as u32;
                }
                leaf(m, 0x195e38, &[ptr, source, n.wrapping_add(1)]);
            }
        }
    }
    h.call(m, 0x2e39ec, &[m.word(p + 60), entries, count]) as u8 as u32
}

pub fn fn_2daf68(m: &mut Memory<'_>, p: u32, k: i32, flags: u32, q: u32) {
    let qn = get(m, q);
    let c = at(m.word(q + 4), qn.wrapping_mul(k));
    let data = m.word(p + 0x2c);
    let m0 = m.word(m.word(p + 0x30));
    let w = m.word(m.word(p + 0x3c));
    let mut cursor = crate::selection::FeatureCursor {
        input: 0,
        input_length: 0,
        current_input: m.word(p + 0x38),
        output: m.word(p + 0x2c),
        current_output: m.word(p + 0x40),
    };
    cursor.rewind(m.word(m.word(p + 0x30)));
    m.put_word(p + 0x38, cursor.current_input);
    m.put_word(p + 0x40, cursor.current_output);
    let out = m.word(m.word(p + 0x3c));
    let ls = get(m, p + 0x1c);
    let v = m.word(p + 0x30);
    let mean = m.word(v);
    let ad = m.word(at(v, ls + 1));
    let add = m.word(at(v, 2 * ls + 2));
    let value = get(m, mean)
        .wrapping_sub(get(m, ad + 4) >> 1)
        .wrapping_sub(get(m, add).wrapping_shl(1))
        .wrapping_add(get(m, add + 4));
    m.put_word(out, value as u32);
    let mut i = 1i32;
    while get(m, p + 0x20).wrapping_sub(1) > i {
        let t = get(m, at(mean, i))
            .wrapping_add(get(m, at(ad, i - 1)) >> 1)
            .wrapping_sub(get(m, at(ad, i + 1)) >> 1)
            .wrapping_add(get(m, at(add, i - 1)))
            .wrapping_sub(get(m, at(add, i)).wrapping_shl(1))
            .wrapping_add(get(m, at(add, i + 1)));
        m.put_word(at(out, i), t as u32);
        i += 1;
    }
    let t = (get(m, at(ad, i - 1)) >> 1)
        .wrapping_add(get(m, at(add, i)).wrapping_shl(1))
        .wrapping_add(get(m, at(add, i - 1)).wrapping_add(get(m, at(mean, i))));
    m.put_word(at(out, i), t as u32);
    for i in 0..get(m, p + 0x20) {
        let idx = get(m, p + 0x44).wrapping_add(i);
        if flags != 0 && idx >= 0 && m.byte(flags + (idx as u32 >> 3)) & (1 << (idx & 7)) != 0 {
            m.put_word(at(data, i), m.word(at(m0, i)).wrapping_shl(6));
            continue;
        }
        let mut acc = (get(m, c) as i64).wrapping_mul(get(m, at(w, i)) as i64);
        for kk in 1..get(m, q) {
            let mut s = 0i32;
            if i - kk >= 0 {
                s = get(m, at(w, i - kk));
            }
            if i + kk < get(m, p + 0x20) {
                s = s.wrapping_add(get(m, at(w, i + kk)));
            }
            acc = acc.wrapping_add((get(m, at(c, kk)) as i64).wrapping_mul(s as i64));
        }
        m.put_word(at(data, i), (acc >> 10) as u32);
    }
}

pub fn fn_301b70(
    m: &mut Memory<'_>,
    alloc: u32,
    arg: u32,
    query: u32,
    list: u32,
    out: u32,
    out_count: u32,
    h: &mut impl Host,
) -> u32 {
    let mut status = 0u8;
    let mut found = 0xff;
    let mut count = 0u32;
    let mut buf = 0u32;
    let mut node = m.word(list + 40);
    while node != 0 {
        let next = m.word(node + 76);
        let mut local = [0u8; 16];
        crate::selection::arena_leaf::fill_bytes(
            &mut crate::selection::arena::Arena::new(0, &mut local),
            0,
            0,
            16,
        );
        let mut rb = RecordBuffer {
            first: u32::from_le_bytes(local[0..4].try_into().unwrap()),
            source: u32::from_le_bytes(local[4..8].try_into().unwrap()),
            size: u32::from_le_bytes(local[8..12].try_into().unwrap()),
            flag: local[12],
        };
        let mut matches = true;
        if query != 0 {
            if get(m, query) != -1 && m.word(query) != m.word(node + 8) {
                matches = false;
            }
            if matches && get(m, query + 4) != -1 && m.word(query + 4) != m.word(node + 16) {
                matches = false;
            }
            for (q, n, strict) in [
                (query + 8, node + 20, true),
                (query + 16, node + 28, false),
                (query + 12, node + 24, false),
            ] {
                if !matches {
                    break;
                }
                let a = m.word(q);
                let b = m.word(n);
                if a != 0 && (b != 0 || strict) {
                    if b == 0 {
                        matches = false;
                    } else {
                        let f = if q == query + 12 { 0x195aac } else { 0x195d58 };
                        if leaf(m, f, &[a, b, 128]) != 0 {
                            matches = false;
                        }
                    }
                }
            }
        }
        if !matches {
            node = next;
            continue;
        }
        m.put_word(node + 68, 0xffffffff);
        loop {
            status = h.record(m, node + 32, arg, &mut rb, &mut found) as u8;
            if status != 0 {
                return status as u32;
            }
            if found == 0 {
                buf = crate::boot_part_f::fn_1b218c(
                    m,
                    alloc,
                    buf,
                    count.wrapping_mul(32).wrapping_add(32),
                    &mut status,
                    h,
                );
                if status != 0 {
                    return status as u32;
                }
                let slot = buf.wrapping_add(count.wrapping_mul(32));
                let mut size = 0u32;
                if m.word(node + 20) != 0 {
                    size = leaf(m, 0x195d90, &[m.word(node + 20)]).wrapping_add(1);
                }
                if m.word(node + 24) != 0 {
                    size = size.wrapping_add(128);
                }
                if m.word(node + 28) != 0 {
                    size =
                        size.wrapping_add(leaf(m, 0x195d90, &[m.word(node + 28)]).wrapping_add(1));
                }
                let (mut p5, mut pf, mut p8) = (0, 0, 0);
                let mut flag = 0;
                if size != 0 {
                    let mut ptr = crate::boot_part_f::fn_1b2140(m, alloc, size, &mut flag, h);
                    if flag != 0 {
                        return flag as u32;
                    }
                    if m.word(node + 20) != 0 {
                        p5 = ptr;
                        leaf(m, 0x195cd0, &[ptr, m.word(node + 20)]);
                        ptr = ptr.wrapping_add(leaf(m, 0x195d90, &[ptr]).wrapping_add(1));
                    }
                    if m.word(node + 24) != 0 {
                        pf = ptr;
                        leaf(m, 0x195ae8, &[ptr, m.word(node + 24), 128]);
                        ptr = ptr.wrapping_add(128);
                    }
                    if m.word(node + 28) != 0 {
                        p8 = ptr;
                        leaf(m, 0x195cd0, &[ptr, m.word(node + 28)]);
                    }
                }
                let data = crate::boot_part_f::fn_1b2140(m, alloc, rb.size, &mut flag, h);
                if flag != 0 {
                    return flag as u32;
                }
                leaf(m, 0x195ae8, &[data, rb.source, rb.size]);
                for (off, v) in [
                    (0, m.word(node + 8)),
                    (4, m.word(node + 12)),
                    (20, p8),
                    (12, p5),
                    (16, pf),
                    (8, m.word(node + 16)),
                    (24, data),
                    (28, rb.size),
                ] {
                    m.put_word(slot + off, v);
                }
                count = count.wrapping_add(1);
            }
            if rb.flag != 1 {
                break;
            }
        }
        node = m.word(node + 76);
    }
    m.put_word(out, buf);
    m.put_word(out_count, count);
    status as u32
}

pub fn fn_2da8c8<'a>(
    m: &mut Memory<'_>,
    _handle: u32,
    b: u32,
    c: u32,
    index: i32,
    n: i32,
    out: u32,
    v: u32,
    pdf: &crate::selection::PdfTable<'a>,
    quantized: &crate::selection::QuantizedPdfTable<'a>,
    mut load: impl FnMut(u32, u32) -> crate::selection::AcousticRecord<'a>,
) -> u32 {
    let mut rc = 0;
    let group = if index != 0 {
        get(m, at(v + 0x1e4, index))
    } else {
        0
    };
    let offset = if index != 0 {
        index
            .wrapping_sub(half(m, v + 0x1b8 + group as u32 * 2) as i32)
            .wrapping_sub(1)
    } else {
        0
    };
    for i in 0..n {
        let destination = at(m.word(out + 4), i);
        let mut sample = get(m, destination);
        rc = if index != 0 {
            quantized.read_first(
                b,
                c,
                group as u32,
                offset as usize,
                i as u32,
                16,
                &mut sample,
                &mut load,
            )
        } else {
            pdf.read_pair(b, c, i as u32, 16, &mut sample, None, &mut load)
        }
        .expect("invalid caller PDF projection");
        if rc >= 0x80000000 {
            return rc;
        }
        m.put_word(destination, sample as u32);
    }
    rc
}

fn bisect_check(m: &mut Memory<'_>, p: u32, range: u32, h: &mut impl Host) -> u32 {
    if p == 0 {
        return 0;
    }
    let scratch = h.scratch() + 0x180;
    let off = m.offset(scratch);
    let mut saved = [0u8; 64];
    saved.copy_from_slice(&m.bytes[off..off + 64]);
    m.bytes[off..off + 32].fill(0);
    let rc = bisect_inner(m, p, range, scratch, h);
    m.bytes[off..off + 64].copy_from_slice(&saved);
    rc
}
fn token_call(m: &mut Memory<'_>, h: &mut impl Host, p: u32, slot: u32, args: &[u32]) -> u32 {
    let method = m.word(m.word(p + 12) + slot);
    let mut words = [0u32; 7];
    words[0] = m.word(p + 16);
    words[1] = m.word(p + 20);
    words[2..2 + args.len()].copy_from_slice(args);
    h.call(m, method, &words[..2 + args.len()])
}
fn token_property(
    m: &mut Memory<'_>,
    p: u32,
    id: u32,
    field: u32,
    val: u32,
    scratch: u32,
    h: &mut impl Host,
) -> u32 {
    m.put_half(scratch, 0);
    m.put_half(scratch + 4, 0);
    let rc = token_call(m, h, p, 0x5c, &[id, field, scratch]);
    if rc < 0x80000000 && half(m, scratch) == 1 {
        token_call(m, h, p, 0x54, &[id, field, 1, val, scratch + 4]);
        if half(m, scratch + 4) == 1 {
            return 0;
        }
    } else if field == 0 {
        return 0;
    }
    let code = match (field, rc < 0x80000000 && half(m, scratch) == 1) {
        (1, true) => 0x5dde,
        (1, false) => 0x5ddf,
        (2, true) => 0x5ddf,
        (2, false) => 0x5dde,
        _ => 0x5de0,
    };
    let logger = m.word(m.word(p) + 16);
    let category = m.word(p + 72);
    crate::render_part_a::error_log(
        m,
        &mut Formatter(h),
        logger,
        category,
        code,
        0,
        0,
        scratch + 8,
    );
    crate::normalization::error_code(0)
}
fn bisect_inner(m: &mut Memory<'_>, p: u32, range: u32, s: u32, h: &mut impl Host) -> u32 {
    if token_call(m, h, p, 0x34, &[1, 0, s]) >= 0x80000000 {
        return 0;
    }
    if token_call(m, h, p, 0x34, &[3, half(m, s) as u32, s + 2]) != 0 || half(m, s + 2) == 0 {
        return 0;
    }
    loop {
        let token = half(m, s + 2) as u32;
        if token_property(m, p, token, 1, s + 4, s + 12, h) >= 0x80000000 {
            break;
        }
        if half(m, s + 4) as i32 > half(m, range + 2) as i16 as i32 {
            break;
        }
        if token_property(m, p, token, 2, s + 6, s + 12, h) >= 0x80000000 {
            break;
        }
        if token_property(m, p, token, 0, s + 8, s + 12, h) >= 0x80000000 {
            break;
        }
        let start = half(m, s + 4) as i32;
        let end = half(m, s + 6) as i32;
        let from = half(m, range) as i16 as i32;
        let to = half(m, range + 2) as i16 as i32;
        if m.byte(s + 8) <= 3
            && (from != start || to != end)
            && ((from > start && from < end) || (to > start && to < end))
        {
            log(
                m,
                h,
                m.word(m.word(p) + 16),
                m.word(p + 72),
                5,
                0x24aa08,
                &[from as u32, to as u32, start as u32, end as u32],
            );
            return 1;
        }
        if token_call(m, h, p, 0x3c, &[token, s + 2]) != 0 || half(m, s + 2) == 0 {
            break;
        }
    }
    0
}

pub fn fn_2e5adc(
    m: &mut Memory<'_>,
    p: u32,
    a: u32,
    b: u32,
    c: u32,
    token: u32,
    matched: u32,
    result: u32,
    h: &mut impl Host,
) -> u32 {
    log_state(m, h, p, 4, 0x2e5adc, &[]);
    let off = m.offset(result);
    m.bytes[off] = 1;
    if bisect_check(m, p, b, h) == 1 {
        log_state(m, h, p, 4, 0x2e5b30, &[m.byte(result) as u32]);
        m.bytes[off] = 0;
        return 0;
    }
    if m.byte(p + 0xc6) == 1 {
        h.call(m, 0x24dcf0, &[p, a, b, 0]);
    }
    assert_eq!(m.base, 0, "table_entry_values uses offset-addressed memory");
    let mut value = 0;
    let mut found = 0;
    let mut rule = 0;
    crate::normalization::table_entry_values(
        m.bytes,
        p,
        matched,
        &mut value,
        Some(&mut found),
        &mut rule,
    );
    let mut status = 0;
    if found != 0 {
        log_state(m, h, p, 5, 0x2e5b80, &[token]);
        m.put_half(p + 0x8e, 0);
        let text = m.word(p + 0x88);
        let text_off = m.offset(text);
        m.bytes[text_off] = 0;
        let entry = m.word(p + 0x68);
        m.put_word(entry + 12, a);
        m.put_word(entry + 8, b);
        status = h.call(m, 0x2e564c, &[p, c, rule, p + 0x88, 0]);
        if status >= 0x80000000 {
            return status;
        }
        let valid = half(m, p + 0x8e) != 0
            && leaf(m, 0x195d58, &[m.word(p + 0x88), h.format(0x2e5bfc)]) == 0;
        m.bytes[off] = u8::from(valid);
    }
    log_state(m, h, p, 4, 0x2e5c08, &[m.byte(result) as u32]);
    status
}
struct Normalize<'a, H>(&'a mut H);
impl<H: Host> crate::normalization::NormalizationHost for Normalize<'_, H> {
    fn calloc(&mut self, bytes: &mut [u8], heap: u32, count: u32, size: u32) -> Option<u32> {
        struct Method<'a, 'b, H> {
            m: Memory<'a>,
            h: &'b mut H,
            method: u32,
        }
        impl<H: Host> crate::frontend::MethodTable for Method<'_, '_, H> {
            fn call_at(&mut self, obj: u32, n: u32, size: u32) -> u32 {
                self.h.call(&mut self.m, self.method, &[obj, n, size])
            }
        }
        let m = Memory { base: 0, bytes };
        let mut local = [0u8; 8];
        if heap != 0 {
            local[4..8].copy_from_slice(&m.word(heap + 4).to_le_bytes());
        }
        let method = if heap != 0 {
            m.word(m.word(heap) + 4)
        } else {
            0
        };
        let r = crate::frontend::call_method(
            &local,
            if heap == 0 { None } else { Some(0) },
            count,
            size,
            &mut Method {
                m,
                h: self.0,
                method,
            },
        );
        if r == 0 {
            None
        } else {
            Some(r)
        }
    }
    fn alloc(&mut self, _: &mut [u8], _: u32, _: u32) -> Option<u32> {
        panic!("insert_node must use calloc")
    }
    fn realloc(&mut self, _: &mut [u8], _: u32, _: u32, _: u32) -> Option<u32> {
        panic!("insert_node must use calloc")
    }
    fn free(&mut self, _: &mut [u8], _: u32, _: u32) {
        panic!("unexpected node release")
    }
    fn resolve(&mut self, _: &mut [u8], _: u32, _: u32) -> (u32, u32) {
        panic!("unexpected node handle")
    }
}

pub fn fn_2b6afc(m: &mut Memory<'_>, callout: u32, h: &mut impl Host) -> u32 {
    let ctx = m.word(callout + 36);
    let span = m.word(ctx + 4);
    let record = m.word(ctx);
    if m.word(ctx + 24) >= 0x80000000 {
        return 1;
    }
    if half(m, ctx + 20) != 0 {
        m.put_word(callout + 28, half(m, ctx + 20) as u32 + 1);
    }
    log_state(
        m,
        h,
        m.word(ctx + 16),
        6,
        0x2b6afc,
        &[
            m.word(callout + 28),
            half(m, ctx + 20) as u32,
            m.word(callout + 20),
            m.word(callout + 24),
            m.word(ctx + 12),
        ],
    );
    let previous_to = half(m, span + 2) as i16 as i32;

    m.put_word(0xe0ff070u32.wrapping_add(4), m.word(ctx + 12));
    m.put_half(0xe0ff068u32, half(m, ctx + 20));
    if previous_to != 0 || half(m, span) != 0 {
        if get(m, callout + 20) < previous_to {
            log_state(
                m,
                h,
                m.word(ctx + 16),
                6,
                0x2b6c00,
                &[m.word(callout + 20), previous_to as u32],
            );
            return 1;
        }
        log_state(m, h, m.word(ctx + 16), 6, 0x2b6b80, &[]);
        let result = h.scratch() - 4;
        let off = m.offset(result);
        let saved = m.byte(result);
        m.bytes[off] = 0;
        let status = fn_2e5adc(
            m,
            m.word(ctx + 16),
            m.word(ctx),
            m.word(ctx + 4),
            m.word(ctx + 8),
            m.word(ctx + 12),
            m.word(m.word(ctx) + 84),
            result,
            h,
        );
        let valid = m.byte(result);
        m.bytes[off] = saved;
        if status >= 0x80000000 {
            m.put_word(ctx + 24, status);
            return 1;
        }
        if valid == 1 {
            let status = crate::normalization::insert_node(
                m.bytes,
                m.word(ctx + 16),
                m.word(ctx + 4),
                m.word(ctx),
                &mut Normalize(h),
            );
            if status >= 0x80000000 {
                m.put_word(ctx + 24, status);
                return 1;
            }
            let off = m.offset(ctx + 22);
            m.bytes[off] = 1;
        } else {
            log_state(m, h, m.word(ctx + 16), 6, 0x2b6bc0, &[]);
        }
        let span = m.word(ctx + 4);
        m.put_half(span, 0);
        m.put_half(span + 2, 0);
        let off = m.offset(ctx + 22);
        m.bytes[off] = 0;
        m.put_half(m.word(ctx) + 40, 0);
        if (half(m, span + 2) != 0 || half(m, span) != 0)
            && (half(m, span + 2) as i16 as i32 - half(m, span) as i16 as i32)
                >= get(m, callout + 24).wrapping_sub(get(m, callout + 20))
        {
            return 1;
        }
    }
    log_state(
        m,
        h,
        m.word(ctx + 16),
        6,
        0x2b6c20,
        &[
            m.word(callout + 20),
            m.word(callout + 24),
            m.word(callout + 28).wrapping_sub(1),
        ],
    );
    let from = get(m, callout + 20) as i16;
    let to = get(m, callout + 24) as i16;
    if from < (half(m, ctx + 28) as i16) || (half(m, ctx + 30) as i16) < to {
        log_state(m, h, m.word(ctx + 16), 6, 0x2b6c60, &[]);
        return 1;
    }

    let scratch = 0xe0ff070u32;

    let candidate = scratch;
    let candidate_span = scratch + 96;
    let valid = scratch + 92;
    m.put_half(candidate_span, from as u16);
    m.put_half(candidate_span + 2, to as u16);
    m.put_word(candidate + 84, m.word(m.word(ctx) + 84));
    m.put_half(candidate + 88, half(m, m.word(ctx) + 88));
    m.put_half(candidate + 90, half(m, m.word(ctx) + 90));
    let count = m.word(callout + 28).wrapping_sub(1) as u8;
    let suboff = m.offset(candidate + 82);
    m.bytes[suboff] = count;
    m.put_half(candidate + 40, 0);
    if count != 0 {
        let mut j = 0;
        let mut i = 2;
        while i < m.word(callout + 28).wrapping_mul(2) {
            let offsets = m.word(callout + 8);
            m.put_half(candidate + 42 + j * 4, m.word(offsets + i * 4) as u16);
            m.put_half(candidate + 44 + j * 4, m.word(offsets + (i + 1) * 4) as u16);
            j += 1;
            i += 2;
        }
    }
    let valid_off = m.offset(valid);
    m.bytes[valid_off] = 1;
    let status = fn_2e5adc(
        m,
        m.word(ctx + 16),
        candidate,
        candidate_span,
        m.word(ctx + 8),
        m.word(ctx + 12),
        m.word(m.word(ctx) + 84),
        valid,
        h,
    );
    if status >= 0x80000000 {
        m.put_word(ctx + 24, status);
    } else if m.byte(valid) == 0 {
        m.put_half(candidate + 40, 0);
        log_state(m, h, m.word(ctx + 16), 6, 0x2b6dd0, &[]);
        let span = m.word(ctx + 4);
        m.put_half(span, 0);
        m.put_half(span + 2, 0);
        let off = m.offset(ctx + 22);
        m.bytes[off] = 0;
        m.put_half(m.word(ctx) + 40, 0);
    } else {
        m.put_half(record + 40, half(m, candidate + 40));
        leaf(m, 0x195ae8, &[record, candidate, 40]);
        log_state(m, h, m.word(ctx + 16), 6, 0x2b6e20, &[]);
        m.put_half(span, m.word(callout + 20) as u16);
        m.put_half(span + 2, m.word(callout + 24) as u16);
        let off = m.offset(ctx + 22);
        m.bytes[off] = 1;
        if count != 0 {
            let suboff = m.offset(record + 82);
            m.bytes[suboff] = count;
            let mut j = 0;
            let mut i = 2;
            while i < m.word(callout + 28).wrapping_mul(2) {
                let offsets = m.word(callout + 8);
                m.put_half(record + 42 + j * 4, m.word(offsets + i * 4) as u16);
                m.put_half(record + 44 + j * 4, m.word(offsets + (i + 1) * 4) as u16);
                log_state(
                    m,
                    h,
                    m.word(ctx + 16),
                    6,
                    0x2b6e60,
                    &[
                        j,
                        half(m, record + 42 + j * 4) as i16 as u32,
                        half(m, record + 44 + j * 4) as i16 as u32,
                    ],
                );
                j += 1;
                i += 2;
            }
        }
    }
    1
}

pub struct FilterWork<'a> {
    pub coefficients: &'a mut [i32],
    pub input: &'a mut [i32],
    pub output: &'a mut [i32],
}
fn project_rows(m: &Memory<'_>, p: u32, work: &mut FilterWork<'_>) -> (usize, usize) {
    let n = m.word(p + 32) as usize;
    let width = (m.word(p + 36) as usize).max(3);
    assert!(
        work.coefficients.len() >= n * width && work.input.len() >= n && work.output.len() >= n,
        "filter workspace too small"
    );
    for i in 0..n {
        let row = m.word(m.word(p + 60) + i as u32 * 4);
        for j in 0..width {
            work.coefficients[i * width + j] = get(m, row + j as u32 * 4);
        }
    }
    (n, width)
}
struct FilterHooks<'a, H>(&'a mut H);
impl<H: Host> crate::reentrant::HooksM for FilterHooks<'_, H> {
    fn call_m(&mut self, f: u32, a: &[u32], m: &mut Memory<'_>) -> u32 {
        if f == 0x30209c {
            crate::runtime::signed_division(a[0] as i32, a[1] as i32)
                .expect("ARM divide by zero")
                .quotient as u32
        } else {
            self.0.call(m, f, a)
        }
    }
}

pub fn fn_2a2be0(
    m: &mut Memory<'_>,
    context: u32,
    p: u32,
    work: &mut FilterWork<'_>,
    h: &mut impl Host,
) {
    if m.word(p + 32) == 0 {
        return;
    }
    m.put_word(p + 56, m.word(m.word(p + 48)));
    let output = m.word(p + 44);
    m.put_word(p + 64, output);
    if m.word(p + 36) == 3 {
        let mut i = 0i32;
        let mut output = output;
        while i < get(m, p + 32) {
            let scale = get(m, elem(m, m.word(p + 52), 0, i));
            m.put_word(
                output,
                (scale >> 9).wrapping_mul(get(m, elem(m, m.word(p + 48), 0, i)) >> 9) as u32,
            );
            let row = m.word(at(m.word(p + 60), i));
            m.put_word(row + 8, 0);
            m.put_word(row, scale as u32);
            m.put_word(row + 4, 0);
            let mut j = 1i32;
            while j < get(m, m.word(p + 40) + 24) {
                let weight = m.word(at(m.word(m.word(p + 40) + 32), j));
                for (k, t) in [(i - 1, 1), (i, 0), (i + 1, -1)] {
                    let w = get(m, at(weight, t));
                    if w == 0 || k < 0 || k >= get(m, p + 32) {
                        continue;
                    }
                    let product = (get(m, elem(m, m.word(p + 52), j, k)) >> 9).wrapping_mul(w);
                    let value = product >> 9;
                    m.put_word(
                        output,
                        get(m, output).wrapping_add(
                            value.wrapping_mul(get(m, elem(m, m.word(p + 48), j, k)) >> 9),
                        ) as u32,
                    );
                    let coefficients = m.word(m.word(p + 40) + 36);
                    m.put_word(
                        row,
                        get(m, row)
                            .wrapping_add(value.wrapping_mul(get(m, elem(m, coefficients, j, t))))
                            as u32,
                    );
                    if t <= 0 && i + 1 < get(m, p + 32) {
                        m.put_word(
                            row + 4,
                            get(m, row + 4).wrapping_add(
                                value.wrapping_mul(get(m, elem(m, coefficients, j, t + 1))),
                            ) as u32,
                        );
                    }
                    if t == -1 && i + 2 < get(m, p + 32) {
                        m.put_word(
                            row + 8,
                            get(m, row + 8).wrapping_add(
                                value.wrapping_mul(get(m, elem(m, coefficients, j, 1))),
                            ) as u32,
                        );
                    }
                }
                j += 1;
            }
            i += 1;
            output = output.wrapping_add(4);
        }
    } else {
        fn_2a178c(m, p, 18, 18, output);
    }
    if m.word(p + 36) == 3 {
        let (n, width) = project_rows(m, p, work);
        let okay = crate::selection::BandFactorization {
            coefficients: &mut work.coefficients[..n * width],
            count: n,
            stride: width,
        }
        .factor_three(18)
        .expect("invalid filter factorization projection");
        for i in 0..n {
            let row = m.word(m.word(p + 60) + i as u32 * 4);
            for j in 0..width {
                m.put_word(row + j as u32 * 4, work.coefficients[i * width + j] as u32);
            }
        }
        if !okay {
            let category = h.category(0x8fa510);
            log(m, h, m.word(context + 16), category, 0, 0x2a19ac, &[]);
        }
    } else {
        fn_2a19ac(m, context, p, 18, h);
    }
    if m.word(p + 36) == 3 {
        crate::reentrant::forward_lattice_m(m, &mut FilterHooks(h), p, 18, 18, 22);
    } else {
        let (n, width) = project_rows(m, p, work);
        let input = m.word(p + 64);
        for i in 0..n {
            work.input[i] = get(m, input + i as u32 * 4);
        }
        crate::selection::BandSystem {
            coefficients: &work.coefficients[..n * width],
            stride: width,
            width: m.word(p + 36) as usize,
            input: &work.input[..n],
        }
        .forward(18, 18, 22, &mut work.output[..n])
        .expect("invalid generic forward projection");
        for i in 0..n {
            let output = m.word(p + 56);
            m.put_word(output + i as u32 * 4, work.output[i] as u32);
        }
    }
    let (n, width) = project_rows(m, p, work);
    let input = m.word(p + 56);
    for i in 0..n {
        work.input[i] = get(m, input + i as u32 * 4);
    }
    let system = crate::selection::BandSystem {
        coefficients: &work.coefficients[..n * width],
        stride: width,
        width: m.word(p + 36) as usize,
        input: &work.input[..n],
    };
    if m.word(p + 36) == 3 {
        system
            .backward_lattice(24, &mut work.output[..n])
            .expect("invalid lattice backward projection");
    } else {
        system
            .backward(24, 22, 18, &mut work.output[..n])
            .expect("invalid generic backward projection");
    }
    for i in 0..n {
        let output = m.word(p + 44);
        m.put_word(output + i as u32 * 4, work.output[i] as u32);
    }
}
