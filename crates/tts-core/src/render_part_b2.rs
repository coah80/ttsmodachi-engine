

use crate::{boot::Memory, fe_text, frontend, normalization};
use normalization::NormalizationHost as _;

pub struct Environment {
    pub normal: u32,
    pub suffix: u32,
    pub boundary_tags: [u32; 6],
    pub cached_handle: [u8; 8],
    pub default_context: u32,
}

pub trait Host: frontend::HeapVtable + frontend::MethodTable {}
fn half(m: &Memory<'_>, p: u32) -> u16 {
    u16::from_le_bytes([m.byte(p), m.byte(p + 1)])
}
fn put_byte(m: &mut Memory<'_>, p: u32, v: u8) {
    let at = m.offset(p);
    m.bytes[at] = v;
}
fn copy(m: &mut Memory<'_>, d: u32, s: u32, n: u32) {
    fe_text::memcpy_195ae8(m.bytes, d - m.base, s - m.base, n);
}
fn mov(m: &mut Memory<'_>, d: u32, s: u32, n: u32) {
    fe_text::memmove_195b38(m.bytes, d - m.base, s - m.base, n);
}
fn record(m: &mut Memory<'_>, d: u32, s: u32) {
    let mut temp = [0; 40];
    temp.copy_from_slice(&m.bytes[m.offset(s)..m.offset(s) + 40]);
    let at = m.offset(d);
    m.bytes[at..at + 40].copy_from_slice(&temp);
}
fn swap(m: &mut Memory<'_>, a: u32, b: u32) {
    let mut temp = [0; 40];
    temp.copy_from_slice(&m.bytes[m.offset(a)..m.offset(a) + 40]);
    copy(m, a, b, 40);
    let at = m.offset(b);
    m.bytes[at..at + 40].copy_from_slice(&temp);
}
fn cmp16(m: &Memory<'_>, a: u32, b: u32) -> i32 {
    fe_text::strcmp16_1765ac(m.bytes, a - m.base, b - m.base)
}
fn len(m: &Memory<'_>, p: u32) -> u32 {
    fe_text::strlen_195d90(m.bytes, p - m.base) as u32
}
fn opt(m: &Memory<'_>, p: u32) -> Option<u32> {
    if p == 0 {
        None
    } else {
        Some(p - m.base)
    }
}
fn free(m: &Memory<'_>, heap: u32, p: u32, h: &mut impl Host) {
    frontend::heap_release(
        m.bytes,
        opt(m, heap),
        if p == 0 { None } else { Some(p) },
        h,
    );
}
fn realloc(m: &Memory<'_>, heap: u32, p: u32, n: u32, h: &mut impl Host) -> u32 {
    frontend::heap_realloc(m.bytes, opt(m, heap), p, n, h)
}
struct FetchHost<'a, H> {
    host: &'a mut H,
    env: &'a Environment,
    base: u32,
}
impl<H: Host> normalization::NormalizationHost for FetchHost<'_, H> {
    fn calloc(&mut self, a: &mut [u8], heap: u32, count: u32, size: u32) -> Option<u32> {
        let p = frontend::call_method(
            a,
            if heap == 0 {
                None
            } else {
                Some(heap - self.base)
            },
            count,
            size,
            self.host,
        );
        if p == 0 {
            None
        } else {
            Some(p - self.base)
        }
    }
    fn alloc(&mut self, a: &mut [u8], heap: u32, n: u32) -> Option<u32> {
        let p = frontend::heap_alloc(
            a,
            if heap == 0 {
                None
            } else {
                Some(heap - self.base)
            },
            n,
            self.host,
        );
        if p == 0 {
            None
        } else {
            Some(p - self.base)
        }
    }
    fn realloc(&mut self, a: &mut [u8], heap: u32, p: u32, n: u32) -> Option<u32> {
        let p = frontend::heap_realloc(
            a,
            if heap == 0 {
                None
            } else {
                Some(heap - self.base)
            },
            if p == 0 { 0 } else { p + self.base },
            n,
            self.host,
        );
        if p == 0 {
            None
        } else {
            Some(p - self.base)
        }
    }
    fn free(&mut self, a: &mut [u8], heap: u32, p: u32) {
        frontend::heap_release(
            a,
            if heap == 0 {
                None
            } else {
                Some(heap - self.base)
            },
            if p == 0 { None } else { Some(p + self.base) },
            self.host,
        );
    }
    fn resolve(&mut self, a: &mut [u8], lo: u32, hi: u32) -> (u32, u32) {
        let cache = &self.env.cached_handle;
        let cached = [
            u32::from_le_bytes(cache[..4].try_into().unwrap()),
            u32::from_le_bytes(cache[4..].try_into().unwrap()),
        ];
        let pointed = if [lo, hi] != cached && lo != 0 && hi == 0x1e61 {
            let at = (lo - self.base) as usize;
            Some(u16::from_le_bytes(a[at..at + 2].try_into().unwrap()))
        } else {
            None
        };
        let mut out = 0;
        let rc = frontend::resolve_engine_context(
            lo,
            pointed,
            hi,
            cache,
            self.env.default_context,
            &mut out,
        );
        (rc, if out == 0 { 0 } else { out - self.base })
    }
}
fn fetch(
    m: &mut Memory<'_>,
    seq: u32,
    tag: u32,
    n: u32,
    e: &Environment,
    h: &mut impl Host,
) -> (u32, u32) {
    let base = m.base;
    let mut out = 0;
    if base == 0 || seq == 0 {
        let rc = normalization::fetch_entry(
            m.bytes,
            seq,
            tag,
            n,
            Some(&mut out),
            &mut FetchHost {
                host: h,
                env: e,
                base,
            },
        );
        return (rc, out);
    }

    let lo = m.word(seq);
    let hi = m.word(seq + 4);
    let (status, _) = FetchHost {
        host: h,
        env: e,
        base,
    }
    .resolve(m.bytes, lo, hi);
    if status >= 0x80000000 || n == 0 {
        let rc = normalization::fetch_entry(
            m.bytes,
            seq - base,
            tag,
            n,
            Some(&mut out),
            &mut FetchHost {
                host: h,
                env: e,
                base,
            },
        );
        return (rc, if out == 0 { 0 } else { out + base });
    }
    let table = m.word(seq + 12);
    let count = half(m, seq + 8);
    let mut entry = 0;
    for i in 0..count {
        let p = table + i as u32 * 20;
        if half(m, p + 4) as u32 == tag {
            entry = p;
            break;
        }
    }
    m.put_word(seq + 12, if table == 0 { 0 } else { table - base });
    if entry != 0 && m.word(entry + 8) != 0 {
        m.put_word(entry + 8, m.word(entry + 8) - base);
    }
    let rc = normalization::fetch_entry(
        m.bytes,
        seq - base,
        tag,
        n,
        Some(&mut out),
        &mut FetchHost {
            host: h,
            env: e,
            base,
        },
    );
    m.put_word(seq + 12, table);
    if entry != 0 && m.word(entry + 8) != 0 {
        m.put_word(entry + 8, m.word(entry + 8) + base);
    }
    (rc, if out == 0 { 0 } else { out + base })
}
fn sort(m: &mut Memory<'_>, records: u32, count: u16) {
    let mut i = 1u16;
    while i < count {
        let p = records + i as u32 * 40;
        if i as u32 + 1 < count as u32 && m.word(p + 60) < m.word(p + 20) {
            swap(m, p, p + 40);
            i = 0;
        } else {
            i = i.wrapping_add(1);
        }
    }
}
fn tag_order(m: &mut Memory<'_>, records: u32, count: u16) {
    let mut i = 1u16;
    while i < count {
        let p = records + i as u32 * 40;
        if i as u32 + 1 < count as u32
            && m.word(p + 20) == m.word(p + 60)
            && half(m, p + 10) != 0x4000
            && half(m, p + 10) != 0x24
            && half(m, p + 50) == 0x24
        {
            swap(m, p, p + 40);
            i = 1;
        } else {
            i = i.wrapping_add(1);
        }
    }
}

pub fn fn_26a610(
    m: &mut Memory<'_>,
    seq: u32,
    tag: u32,
    e: &Environment,
    h: &mut impl Host,
) -> u32 {
    let (rc, a) = fetch(m, seq, tag, 1, e, h);
    if rc >= 0x80000000 {
        return rc;
    }
    let n = half(m, a + 8);
    let r = m.word(a + 4);
    if n == 0 || half(m, r + 10) != 0x4000 {
        return 0x81102000;
    }
    sort(m, r, n);
    rc
}

pub fn fn_26a7b8(m: &mut Memory<'_>, r: u32, count: u32, _unused: u32, text: u32, e: &Environment) {
    tag_order(m, r, half(m, count));
    let mut last = e.normal;
    let mut i = 1u16;
    while i < half(m, count) {
        let p = r + i as u32 * 40;
        match half(m, p + 10) {
            0x24 => last = m.word(p + 28),
            0x27 => {
                if m.word(p + 20) != m.word(r + 20) {
                    m.put_word(p + 20, m.word(p + 20).wrapping_add(1));
                }
                m.put_half(p + 10, 0x25);
            }
            0x29 => {
                if cmp16(m, last, e.normal) == 0 {
                    let rel = m.word(p + 20).wrapping_sub(m.word(r + 20));
                    if m.word(p + 20) != m.word(r + 20)
                        && m.byte(text.wrapping_add(rel).wrapping_sub(1)) == b' '
                    {
                        m.put_word(p + 20, m.word(p + 20).wrapping_add(1));
                    }
                }
                m.put_half(p + 10, 1);
            }
            _ => {}
        }
        i = i.wrapping_add(1);
    }
    sort(m, r, half(m, count));
    tag_order(m, r, half(m, count));
    let mut out = 1u16;
    i = 1;
    while i < half(m, count) {
        let p = r + i as u32 * 40;
        let mut matched = 0;
        if half(m, p + 10) == 0x24 {
            let mut prev = i - 1;
            while prev != 0 {
                if half(m, r + prev as u32 * 40 + 10) == 0x24 {
                    break;
                }
                prev -= 1;
            }
            if prev != 0 {
                let q = r + prev as u32 * 40;
                if cmp16(m, m.word(p + 28), m.word(q + 28)) != 0 && m.word(p + 20) == m.word(q + 20)
                {
                    matched = q;
                }
            }
        }
        if matched != 0 {
            let flag = m.byte(matched);
            let replace =
                (flag == 1 && m.word(matched + 20) != m.word(r + 20)) || (flag != 0 && flag != 1);
            let invalidate =
                (flag == 1 && m.word(matched + 20) == m.word(r + 20) && m.byte(p) == 0)
                    || (flag == 0 && m.byte(p) == 0);
            if replace {
                record(m, matched, p);
            } else if invalidate {
                m.put_half(matched + 10, 999);
                record(m, r + out as u32 * 40, p);
                out = out.wrapping_add(1);
            }
        } else {
            record(m, r + out as u32 * 40, p);
            out = out.wrapping_add(1);
        }
        i = i.wrapping_add(1);
    }
    m.put_half(count, out);
}

pub fn fn_268a14(
    m: &mut Memory<'_>,
    seq: u32,
    mut a: u32,
    e: &Environment,
    h: &mut impl Host,
) -> u32 {
    let mut r = m.word(a + 4);
    let mut out = 1u16;
    let mut i = 1u16;
    while i < half(m, a + 8) {
        let mut next = i.wrapping_add(1);
        while next < half(m, a + 8) && half(m, r + next as u32 * 40 + 10) == 999 {
            next = next.wrapping_add(1);
        }
        let p = r + i as u32 * 40;
        let kind = half(m, p + 10);
        let at_start = m.word(p + 20) == m.word(r + 20);
        let redundant = at_start
            && (kind == 21 || kind == 7)
            && next < half(m, a + 8)
            && m.word(r + next as u32 * 40 + 20) == m.word(r + 20)
            && half(m, r + next as u32 * 40 + 10) == kind;
        if !redundant && !((kind == 999 || kind == 0) && !(at_start && kind == 7)) {
            record(m, r + out as u32 * 40, p);
            out = out.wrapping_add(1);
        }
        i = i.wrapping_add(1);
    }
    m.put_half(a + 8, out);
    let mut pending = false;
    let mut j = 1u16;
    while j < half(m, a + 8) {
        let mut p = r + j as u32 * 40;
        let kind = half(m, p + 10);
        if kind == 7 || kind == 36 {
            let start = m.word(p + 20);
            let end = m.word(r + 20).wrapping_add(m.word(r + 24));
            if start.wrapping_add(m.word(p + 24)) > end {
                if end < start {
                    m.put_word(p + 24, 0);
                    m.put_word(p + 20, end);
                } else {
                    m.put_word(p + 24, end.wrapping_sub(start));
                }
            }
            if kind == 7 && m.word(p + 28) == 43 {
                if pending {
                    pending = false;
                    j = j.wrapping_add(1);
                    continue;
                } else {
                    pending = true;
                    j = j.wrapping_add(1);
                    continue;
                }
            }
        }
        if pending && kind == 7 {
            if m.word(p + 28) == 43 || m.word(p + 28) == 15 {
                m.put_word(p + 28, 43);
            } else {
                let (rc, got) = fetch(m, seq, 0x3fd, half(m, a + 8).wrapping_add(1) as u32, e, h);
                if rc >= 0x80000000 {
                    return rc;
                }
                a = got;
                r = m.word(a + 4);
                p = r + j as u32 * 40;
                mov(m, p + 40, p, (half(m, a + 8) as u32 - j as u32) * 40);
                copy(m, p, p + 40, 40);
                m.put_word(p + 28, 43);
                m.put_half(a + 8, half(m, a + 8).wrapping_add(1));
            }
            pending = false;
        }
        j = j.wrapping_add(1);
    }
    0
}

pub fn fn_26abd0(
    m: &mut Memory<'_>,
    ctx: u32,
    seq: u32,
    text_out: u32,
    e: &Environment,
    h: &mut impl Host,
) -> u32 {
    let mut text = m.word(text_out);
    let mut length = len(m, text) as u16;
    let (mut rc, mut a) = fetch(m, seq, 0x3fd, 1, e, h);
    if rc >= 0x80000000 {
        return rc;
    }
    let mut count = half(m, a + 8);
    let mut r = m.word(a + 4);
    if count == 0 || half(m, r + 10) != 0x4000 {
        return 0x81102000;
    }
    let mut i = 1u16;
    while i < count {
        let marker = r + i as u32 * 40;
        let offset = m.word(marker + 20).wrapping_sub(m.word(r + 20)) as u16;
        if half(m, marker + 10) == 21 && offset != 0 && m.byte(text + offset as u32 - 1) != b' ' {
            let mut boundary = false;
            let space = m.byte(text + offset as u32) == b' ';
            if !space {
                let mut prev = i - 1;
                while prev != 0 {
                    let kind = half(m, r + prev as u32 * 40 + 10);
                    if kind == 21 {
                        break;
                    }
                    if kind == 34 {
                        boundary = true;
                        break;
                    }
                    prev -= 1;
                }
            }
            if !boundary && !space {
                let resized = realloc(m, m.word(ctx + 4), text, length as u32 + 2, h);
                if resized == 0 {
                    return 0x8110200a;
                }
                text = resized;
                mov(
                    m,
                    text + offset as u32 + 1,
                    text + offset as u32,
                    length as u32 - offset as u32,
                );
                length = length.wrapping_add(1);
                put_byte(m, text + offset as u32, b' ');
                put_byte(m, text + length as u32, 0);
                let mut run = 0u32;
                let mut pos = offset.wrapping_add(1);
                while pos < length && m.byte(text + pos as u32) != b' ' {
                    run += 1;
                    pos = pos.wrapping_add(1);
                }
                let mut j = i.wrapping_add(1);
                while j < count {
                    let p = r + j as u32 * 40;
                    if m.word(p + 20) >= m.word(marker + 20) {
                        m.put_word(p + 20, m.word(p + 20).wrapping_add(1));
                    }
                    j = j.wrapping_add(1);
                }
                m.put_word(marker + 20, m.word(marker + 20).wrapping_add(1));
                m.put_word(r + 24, m.word(r + 24).wrapping_add(1));
                j = 1;
                while j < count {
                    let mut p = r + j as u32 * 40;
                    let cut = m.word(r + 20).wrapping_add(offset as u32);
                    if half(m, p + 10) == 1
                        && cut > m.word(p + 20)
                        && m.word(p + 20).wrapping_add(m.word(p + 24)) > cut
                    {
                        let split = m.word(p + 24).wrapping_add(
                            m.word(p + 20)
                                .wrapping_sub(m.word(r + 20))
                                .wrapping_sub(offset as u32)
                                .wrapping_mul(2),
                        ) as i16;
                        let mut next = j;
                        let mut found = 0;
                        while next < count {
                            let q = r + next as u32 * 40;
                            if half(m, q + 10) == 21 && m.word(q + 20) > m.word(p + 20) {
                                found = q;
                                break;
                            }
                            next = next.wrapping_add(1);
                        }
                        let mut keep = false;
                        if found != 0 {
                            let tag = m.word(found + 28);

                            let mut units = [0u16; 256];
                            let mut k = 0;
                            loop {
                                units[k] = half(m, tag + k as u32 * 2);
                                if units[k] == 0 {
                                    break;
                                }
                                k += 1;
                            }
                            let mut display = [0u8; 256];
                            crate::frontend_runtime::utf16_to_ascii(&mut display, &units[..=k])
                                .expect("tag display");
                            for (idx, &candidate) in e.boundary_tags.iter().enumerate() {
                                let equal = if idx == 1 {
                                    normalization::u16_ncmp(
                                        m.bytes,
                                        tag - m.base,
                                        candidate - m.base,
                                        6,
                                    ) == 0
                                } else {
                                    cmp16(m, tag, candidate) == 0
                                };
                                if equal {
                                    keep = true;
                                    break;
                                }
                            }
                        }
                        if !keep {
                            let (status, got) =
                                fetch(m, seq, 0x3fd, half(m, a + 8).wrapping_add(1) as u32, e, h);
                            rc = status;
                            if rc >= 0x80000000 {
                                return rc;
                            }
                            a = got;
                            r = m.word(a + 4);
                            p = r + j as u32 * 40;
                            mov(m, p + 40, p, (count as u32 - j as u32) * 40);
                            copy(m, p, p + 40, 40);
                            count = half(m, a + 8).wrapping_add(1);
                            m.put_half(a + 8, count);
                            let ref1 = m.word(p + 16);
                            let half = m.word(p + 24).wrapping_sub(split as i32 as u32) >> 1;
                            m.put_word(p + 16, half.wrapping_mul(2));
                            m.put_word(p + 24, half);
                            m.put_word(p + 56, ref1.wrapping_sub(half.wrapping_mul(2)));
                            m.put_word(p + 52, m.word(p + 12).wrapping_add(half.wrapping_mul(2)));
                            m.put_word(p + 60, m.word(p + 20).wrapping_add(half).wrapping_add(1));
                            m.put_word(p + 64, run);
                        } else {
                            m.put_word(
                                p + 24,
                                cut.wrapping_sub(m.word(p + 20)) as i16 as i32 as u32,
                            );
                        }
                    } else {
                        j = j.wrapping_add(1);
                        if m.word(r + j as u32 * 40 + 20)
                            > m.word(r + 20).wrapping_add(offset as u32)
                        {
                            break;
                        }
                    }
                }
            }
        }
        i = i.wrapping_add(1);
    }
    m.put_word(text_out, text);
    rc
}

pub fn fn_266fc4(
    m: &mut Memory<'_>,
    ctx: u32,
    r: u32,
    count: u32,
    text: u32,
    e: &Environment,
    h: &mut impl Host,
) {
    let text_len = len(m, text);
    let mut i = 1u16;
    'markers: while i < half(m, count) {
        let p = r + i as u32 * 40;
        if half(m, p + 10) == 7 && matches!(m.word(p + 28), 15 | 16) {
            let mut j = 0u16;
            while j < half(m, p + 8) {
                let pairs = m.word(p + 4);
                if m.word(pairs + j as u32 * 8) == 0 {
                    let mut k = 1u16;
                    while k < half(m, count) {
                        let c = r + k as u32 * 40;
                        if m.word(p + 20) < m.word(c + 20) {
                            break;
                        }
                        if half(m, c + 10) == 998
                            && m.word(p + 12) == m.word(c + 12)
                            && m.word(p + 28) == m.word(c + 28)
                            && k as u32 + 1 < half(m, count) as u32
                            && half(m, c + 50) == 41
                        {
                            let base = m.word(r + 12).wrapping_add(m.word(r + 16));
                            let v = m
                                .word(p + 12)
                                .wrapping_add(m.word(pairs + j as u32 * 8 + 4))
                                .min(base);
                            m.put_word(c + 52, v);
                            m.put_word(c + 56, m.word(c + 16));
                            m.put_half(c + 10, 0);
                        }
                        k = k.wrapping_add(1);
                    }
                    let base = m.word(r + 12).wrapping_add(m.word(r + 16));
                    let v = m
                        .word(pairs + j as u32 * 8 + 4)
                        .wrapping_add(m.word(p + 12))
                        .min(base);
                    m.put_word(p + 12, v);
                }
                j = j.wrapping_add(1);
            }
            if m.word(p + 4) == 0 {
                let mut k = 1u16;
                while k < half(m, count) && m.word(r + k as u32 * 40 + 20) <= m.word(p + 20) {
                    let c = r + k as u32 * 40;
                    if half(m, c + 10) == 998
                        && m.word(p + 12) == m.word(c + 12)
                        && m.word(p + 28) == m.word(c + 28)
                    {
                        if k as u32 + 1 < half(m, count) as u32
                            && half(m, c + 50) == 41
                            && m.word(c + 52) == m.word(p + 12)
                        {
                            m.put_word(c + 56, m.word(c + 16));
                            m.put_word(c + 52, m.word(p + 12));
                            m.put_half(c + 10, 0);
                        } else {
                            let mut previous = k;
                            while previous > 1 {
                                let prev = r + previous as u32 * 40;
                                if half(m, prev + 10) == 41 {
                                    if m.word(prev + 12) < m.word(p + 12) {
                                        break;
                                    }
                                    if m.word(prev + 12) == m.word(p + 12) {
                                        m.put_word(prev + 12, m.word(r + i as u32 * 40 + 12));
                                        m.put_word(prev + 16, m.word(c + 16));
                                        m.put_half(c + 10, 0);
                                        i = i.wrapping_add(1);
                                        continue 'markers;
                                    }
                                }
                                previous = previous.wrapping_sub(1);
                            }
                        }
                    }
                    k = k.wrapping_add(1);
                }
            } else {
                free(m, m.word(ctx + 4), m.word(p + 4), h);
                m.put_word(p + 4, 0);
                m.put_half(p + 8, 0);
            }
        }
        i = i.wrapping_add(1);
    }
    let mut k = 1u16;
    while k < half(m, count) {
        let p = r + k as u32 * 40;
        if half(m, p + 10) == 998 {
            let mut j = 1u16;
            while j < half(m, count) && m.word(r + j as u32 * 40 + 20) <= m.word(p + 20) {
                let c = r + j as u32 * 40;
                if half(m, c + 10) == 41 && m.word(c + 12) == m.word(p + 12) {
                    m.put_word(c + 16, m.word(p + 16));
                    if j as u32 + 1 < half(m, count) as u32
                        && half(m, c + 50) == 39
                        && m.word(c + 60) == m.word(c + 20)
                    {
                        m.put_word(c + 56, m.word(p + 16));
                    } else if j > 1 && half(m, c - 30) == 39 && m.word(c - 20) == m.word(c + 20) {
                        m.put_word(c - 24, m.word(p + 16));
                    }
                    break;
                }
                j = j.wrapping_add(1);
            }
        }
        k = k.wrapping_add(1);
    }
    let mut out = 1u16;
    let mut read = 1u16;
    while read < half(m, count) {
        let p = r + read as u32 * 40;
        let d = r + out as u32 * 40;
        let kind = half(m, p + 10);
        if kind == 39 {
            record(m, d, p);
            m.put_word(
                d + 20,
                m.word(r + 20)
                    .wrapping_add(m.word(p + 20).wrapping_sub(m.word(r + 20)) >> 1),
            );
            out = out.wrapping_add(1);
        } else if kind == 7 {
            match m.word(p + 28) {
                43 => {
                    if m.word(p + 24) == 0 {
                        if m.word(p + 32) != 0 {
                            free(m, m.word(ctx + 4), m.word(p + 32), h);
                            m.put_word(p + 32, 0);
                        }
                        if m.word(p + 4) != 0 {
                            free(m, m.word(ctx + 4), m.word(p + 4), h);
                            m.put_word(p + 4, 0);
                            m.put_half(p + 8, 0);
                        }
                    } else {
                        let mut off =
                            (m.word(p + 20).wrapping_sub(m.word(r + 20)) >> 1) as u16 as u32;
                        let want = len(m, e.suffix) as u16 as u32;
                        let lim = off.wrapping_sub(want) as u16 as u32;
                        if text_len as u16 as u32 > lim {
                            let n = len(m, e.suffix) as u16 as u32;
                            if normalization::strncmp_at(
                                m.bytes,
                                text.wrapping_add(lim) - m.base,
                                e.suffix - m.base,
                                n,
                            ) == 0
                            {
                                if m.word(p + 4) != 0 {
                                    free(m, m.word(ctx + 4), m.word(p + 4), h);
                                    m.put_word(p + 4, 0);
                                    m.put_half(p + 8, 0);
                                }
                                record(m, d, p);
                                m.put_word(d + 20, m.word(r + 20).wrapping_add(off));
                                loop {
                                    off = off.wrapping_add(1) as u16 as u32;
                                    if off >= text_len as u16 as u32 || m.byte(text + off) == 31 {
                                        break;
                                    }
                                }
                                if m.byte(text + off) != 31 {
                                    m.put_word(d + 24, m.word(d + 24) >> 1);
                                } else {
                                    m.put_word(
                                        d + 24,
                                        off.wrapping_sub(len(m, e.suffix) as u16 as u32)
                                            .wrapping_sub(lim),
                                    );
                                }
                                out = out.wrapping_add(1);
                            }
                        }
                    }
                }
                15 | 16 => {
                    record(m, d, p);
                    m.put_word(
                        d + 20,
                        m.word(r + 20)
                            .wrapping_add(m.word(p + 20).wrapping_sub(m.word(r + 20)) >> 1),
                    );
                    if m.word(p + 24) != 0 {
                        m.put_word(d + 24, m.word(p + 24) >> 1);
                    }
                    out = out.wrapping_add(1);
                }
                _ => {
                    if m.word(p + 4) != 0 {
                        free(m, m.word(ctx + 4), m.word(p + 4), h);
                        m.put_word(p + 4, 0);
                        m.put_half(p + 8, 0);
                    }
                }
            }
        } else {
            record(m, d, p);
            m.put_word(
                d + 20,
                m.word(r + 20)
                    .wrapping_add(m.word(p + 20).wrapping_sub(m.word(r + 20)) >> 1),
            );
            out = out.wrapping_add(1);
        }
        read = read.wrapping_add(1);
    }
    m.put_half(count, out);
    let mut o = 1u16;
    let mut n = 1u16;
    while o < half(m, count) {
        let p = r + o as u32 * 40;
        let d = r + n as u32 * 40;
        if half(m, p + 10) == 7 && m.word(p + 28) == 43 {
            record(m, d, p);
        } else {
            let mut j = 1u16;
            let mut hit = None;
            while j < half(m, count) && hit.is_none() {
                if j != o {
                    let c = r + j as u32 * 40;
                    if half(m, c + 10) == 7
                        && m.word(c + 28) == 43
                        && m.word(c + 20) <= m.word(p + 20)
                        && m.word(p + 20) < m.word(c + 20).wrapping_add(m.word(c + 24))
                    {
                        hit = Some(m.word(c + 20));
                    }
                }
                j = j.wrapping_add(1);
            }
            record(m, d, p);
            if let Some(start) = hit {
                m.put_word(d + 20, start);
            }
        }
        n = n.wrapping_add(1);
        o = o.wrapping_add(1);
    }
    m.put_half(count, n);
    let mut q = 1u16;
    while q < half(m, count) {
        let p = r + q as u32 * 40;
        if matches!(half(m, p + 10), 998 | 0) {
            let n = half(m, count);
            if q as u32 + 1 < n as u32 {
                mov(m, p, p + 40, (n as u32 - q as u32 - 1) * 40);
            }
            m.put_half(count, n.wrapping_sub(1));
        } else {
            q = q.wrapping_add(1);
        }
    }
}

pub struct ExtendedEnvironment {
    pub markers: Environment,
    pub padding: u32,
    pub separators: u32,
}
fn len16(m: &Memory<'_>, p: u32) -> u32 {
    fe_text::strlen16_176624(m.bytes, p - m.base) as u32
}
fn copy16(m: &mut Memory<'_>, d: u32, s: u32, n: u32) {
    fe_text::strncpy16_176644(m.bytes, d - m.base, s - m.base, n);
}
fn separator(m: &Memory<'_>, e: &ExtendedEnvironment, c: u16) -> bool {
    fe_text::strchr16_176514(m.bytes, e.separators - m.base, c as u32) != 0
}
fn count_span(m: &Memory<'_>, text: u32, n: u16, e: &ExtendedEnvironment) -> (u16, u16, u16) {
    let mut first = 0u16;
    let mut limit = n;
    while first < n && separator(m, e, half(m, text + first as u32 * 2)) {
        first = first.wrapping_add(1);
    }
    let mut scan = n.wrapping_sub(1);
    while scan != 0 && separator(m, e, half(m, text + scan as u32 * 2)) {
        limit = limit.wrapping_sub(1);
        scan = scan.wrapping_sub(1);
    }
    scan = first;
    let mut previous = b'_' as u16;
    let mut count = 0u16;
    while scan < limit && half(m, text + scan as u32 * 2) != 18 {
        let c = half(m, text + scan as u32 * 2);
        if c == b'\\' as u16 {
            if limit as u32 > scan as u32 + 1
                && half(m, text + (scan as u32 + 1) * 2) == b'T' as u16
            {
                let mut close = scan.wrapping_add(2);
                while close < limit && half(m, text + close as u32 * 2) != b'\\' as u16 {
                    close = close.wrapping_add(1);
                }
                if half(m, text + close as u32 * 2) == b'\\' as u16 {
                    scan = close;
                }
            }
        } else if c == b'_' as u16 || c == b'#' as u16 {
            if previous != b'_' as u16 && previous != b'#' as u16 {
                count = count.wrapping_add(1);
            }
        }
        let c = half(m, text + scan as u32 * 2);
        if !separator(m, e, c) {
            previous = c;
        }
        scan = scan.wrapping_add(1);
    }
    if previous != b'_' as u16 && previous != b'#' as u16 {
        count = count.wrapping_add(1);
    }
    (count, first, limit)
}

pub fn fn_26783c(
    m: &mut Memory<'_>,
    context: u32,
    sequence: u32,
    e: &ExtendedEnvironment,
    h: &mut impl Host,
) -> u32 {
    let padding_length = len16(m, e.padding) as u16;
    let (rc, mut tb) = fetch(m, sequence, 0x3fc, 1, &e.markers, h);
    if rc >= 0x80000000 {
        return rc;
    }
    let mut text = m.word(tb + 4);
    let (rc, mut mb) = fetch(m, sequence, 0x3fd, 1, &e.markers, h);
    if rc >= 0x80000000 {
        return rc;
    }
    let mut markers = m.word(mb + 4);
    let (rc, mut sb) = fetch(m, sequence, 0x3f4, 1, &e.markers, h);
    if rc >= 0x80000000 {
        return rc;
    }
    let mut last_boundary = 0u16;
    let mut normal_index = u32::MAX;
    let mut index = 1u16;
    while index < half(m, mb + 8) {
        let mut current = markers + index as u32 * 40;
        if half(m, current + 10) == 21 {
            last_boundary = index;
            index = index.wrapping_add(1);
            continue;
        }
        if half(m, current + 10) != 7 || m.word(current + 28) != 43 || m.word(current + 32) == 0 {
            index = index.wrapping_add(1);
            continue;
        }
        let next_index = index as u32 + 1;
        let offset = m.word(current + 20).wrapping_sub(m.word(markers + 20)) as u16;
        let mut next = next_index as u16;
        while next < half(m, mb + 8) && half(m, markers + next as u32 * 40 + 10) != 7 {
            next = next.wrapping_add(1);
        }
        let end_offset = if next == half(m, mb + 8) {
            m.word(markers + 24) as u16
        } else {
            m.word(markers + next as u32 * 40 + 20)
                .wrapping_sub(m.word(markers + 20)) as u16
        };
        let start = (offset >> 1) as u32;
        let end = (end_offset >> 1) as u32;
        let replacement = m.word(current + 32);
        let mut original_words = 0u16;
        if m.byte(current + 36) != 0 {
            m.put_word(current + 32, 0);
            let span_length = end.wrapping_sub(start) as u16;
            if span_length != 0 {
                let (count, first, limit) = count_span(m, text + start * 2, span_length, e);
                original_words = count;
                if original_words != 0 {
                    let first_index = start.wrapping_add(first as u32) as u16;
                    let span_count = start
                        .wrapping_add(limit as u32)
                        .wrapping_sub(first_index as u32)
                        as u16;
                    let required = half(m, sb + 8).wrapping_add(span_count).wrapping_add(1);
                    if half(m, sb + 10) < required {
                        let (rc, b) = fetch(m, sequence, 0x3f4, required as u32, &e.markers, h);
                        if rc >= 0x80000000 {
                            return rc;
                        }
                        sb = b;
                    }
                    let stored = m.word(sb + 4) + half(m, sb + 8) as u32 * half(m, sb + 2) as u32;
                    copy16(m, stored, text + first_index as u32 * 2, span_count as u32);
                    m.put_half(stored + span_count as u32 * 2, 0);
                    m.put_half(current + 10, 34);
                    m.put_word(current + 28, half(m, sb + 8) as u32);
                    m.put_half(sb + 8, required);
                }
            }
        }
        let mut length = len16(m, replacement) as u16;
        let mut scan = 0u16;
        while scan < length {
            let c = half(m, replacement + scan as u32 * 2);
            if !separator(m, e, c) && c != 95 {
                break;
            }
            scan = scan.wrapping_add(1);
        }
        if scan != 0
            && scan < length
            && !separator(m, e, half(m, replacement + scan as u32 * 2))
            && half(m, replacement + scan as u32 * 2) != 95
        {
            let remaining = length - scan;
            mov(
                m,
                replacement,
                replacement + scan as u32 * 2,
                remaining as u32 * 2,
            );
            length = remaining;
            m.put_half(replacement + length as u32 * 2, 0);
        }
        if length != 0 {
            let last = length - 1;
            scan = last;
            while scan != 0 {
                let c = half(m, replacement + scan as u32 * 2);
                if !separator(m, e, c) && c != 95 {
                    break;
                }
                scan = scan.wrapping_sub(1);
            }
            if scan != 0
                && scan != last
                && !separator(m, e, half(m, replacement + scan as u32 * 2))
                && half(m, replacement + scan as u32 * 2) != 95
            {
                m.put_half(replacement + (scan as u32 + 1) * 2, 0);
                length = len16(m, replacement) as u16;
            }
        }
        scan = 0;
        let mut words = if length > 0 && half(m, replacement) != 0 {
            1u16
        } else {
            0
        };
        while scan < length && half(m, replacement + scan as u32 * 2) != 0 {
            if separator(m, e, half(m, replacement + scan as u32 * 2))
                || half(m, replacement + scan as u32 * 2) == 95
            {
                while scan < length
                    && (separator(m, e, half(m, replacement + scan as u32 * 2))
                        || half(m, replacement + scan as u32 * 2) == 95)
                {
                    scan = scan.wrapping_add(1);
                }
                if length > scan && half(m, replacement + scan as u32 * 2) != 0 {
                    words = words.wrapping_add(1);
                }
            }
            scan = scan.wrapping_add(1);
        }
        let mut padding = 0u16;
        let mut excess = 0u16;
        let mut padding_chars = 0u16;
        if m.byte(current + 36) != 0 && original_words != 0 && original_words != words {
            if original_words > words {
                padding = original_words - words;
                padding_chars = padding_length.wrapping_mul(padding);
            } else {
                excess = words - original_words;
            }
        }
        let new_length = length as u32 + padding_chars as u32;
        let delta: i16;
        if new_length > end - start {
            let added = new_length.wrapping_sub(end - start) as u16;
            delta = added as i16;
            let (rc, b) = fetch(
                m,
                sequence,
                0x3fc,
                half(m, tb + 8).wrapping_add(added) as u32,
                &e.markers,
                h,
            );
            if rc >= 0x80000000 {
                return rc;
            }
            tb = b;
            text = m.word(tb + 4);
            mov(
                m,
                text + (end + added as u32) * 2,
                text + end * 2,
                (half(m, tb + 8) as u32 - end) * 2,
            );
            m.put_half(tb + 8, half(m, tb + 8).wrapping_add(added));
        } else {
            let removed = (end - start - new_length) as u16;
            delta = (removed as i16).wrapping_neg();
            mov(
                m,
                text + (end - removed as u32) * 2,
                text + end * 2,
                (half(m, tb + 8) as u32 - end) * 2,
            );
            m.put_half(tb + 8, half(m, tb + 8).wrapping_sub(removed));
        }
        copy16(m, text + start * 2, replacement, length as u32);
        for scan in 0..padding {
            copy16(
                m,
                text + (start + length as u32 + scan as u32 * padding_length as u32) * 2,
                e.padding,
                padding_length as u32,
            );
        }
        free(m, m.word(context + 4), replacement, h);
        if excess != 0 {
            scan = start
                .wrapping_add(padding as u32 * padding_length as u32)
                .wrapping_add(length as u32)
                .wrapping_sub(1) as u16;
            while scan != 0
                && half(m, tb + 8) > scan
                && (separator(m, e, half(m, text + scan as u32 * 2))
                    || half(m, text + scan as u32 * 2) == 95)
            {
                scan = scan.wrapping_sub(1);
            }
            while scan != 0
                && half(m, tb + 8) > scan
                && !separator(m, e, half(m, text + scan as u32 * 2))
                && half(m, text + scan as u32 * 2) != 95
            {
                scan = scan.wrapping_sub(1);
            }
            while half(m, tb + 8) > scan && excess != 0 {
                if scan == 0 {
                    break;
                }
                while scan != 0
                    && half(m, tb + 8) > scan
                    && (separator(m, e, half(m, text + scan as u32 * 2))
                        || half(m, text + scan as u32 * 2) == 95)
                {
                    m.put_half(text + scan as u32 * 2, 45);
                    scan = scan.wrapping_sub(1);
                }
                if scan == 0 {
                    break;
                }
                excess = excess.wrapping_sub(1);
                while scan != 0
                    && half(m, tb + 8) > scan
                    && !separator(m, e, half(m, text + scan as u32 * 2))
                    && half(m, text + scan as u32 * 2) != 95
                {
                    scan = scan.wrapping_sub(1);
                }
            }
            for s in 1..half(m, mb + 8) {
                let p = markers + s as u32 * 40;
                let lower = m.word(markers + 20).wrapping_add((start * 2) as u16 as u32);
                if m.word(p + 20) > lower
                    && m.word(markers + 20).wrapping_add((end * 2) as u16 as u32) > m.word(p + 20)
                {
                    m.put_word(p + 20, lower);
                }
            }
        }
        let byte_start = (start * 2) as u16 as u32;
        let byte_end = (end * 2) as u16 as u32;
        m.put_word(markers + 24, half(m, tb + 8) as u32 * 2);
        let byte_delta = delta.wrapping_mul(2) as i32 as u32;
        for s in 1..half(m, mb + 8) {
            let p = markers + s as u32 * 40;
            let base = m.word(markers + 20);
            let position = m.word(p + 20);
            let lower = base.wrapping_add(byte_start);
            let upper = base.wrapping_add(byte_end);
            if position > lower && upper > position {
                m.put_word(p + 20, lower);
            } else if position >= upper {
                m.put_word(p + 20, position.wrapping_add(byte_delta));
            } else if half(m, p + 10) == 1
                && position.wrapping_add(m.word(p + 24)) > base.wrapping_add(m.word(markers + 24))
                && position
                    .wrapping_add(m.word(p + 24))
                    .wrapping_add(byte_delta)
                    != 0
            {
                m.put_word(p + 24, m.word(p + 24).wrapping_add(byte_delta));
            }
        }
        if m.byte(current + 36) == 0 || original_words == 0 {
            let mut found = 1u16;
            while found < half(m, mb + 8) {
                let p = markers + found as u32 * 40;
                if half(m, p + 10) == 1 && m.word(p + 20) == m.word(current + 20) {
                    break;
                }
                found = found.wrapping_add(1);
            }
            if found < half(m, mb + 8) && half(m, markers + found as u32 * 40 + 10) == 1 {
                fix_word_length(m, markers, markers + found as u32 * 40, text, e);
            }
            if half(m, mb + 8) as u32 != next_index {
                mov(
                    m,
                    current,
                    current + 40,
                    (half(m, mb + 8) as u32 - index as u32) * 40,
                );
            }
            index = index.wrapping_sub(1);
            m.put_half(mb + 8, half(m, mb + 8).wrapping_sub(1));
        } else {
            let position = m
                .word(markers + 20)
                .wrapping_add(byte_start)
                .wrapping_add(new_length * 2);
            let mut found = 1u16;
            while found < half(m, mb + 8) {
                let p = markers + found as u32 * 40;
                if m.word(p + 20) == position && half(m, p + 10) == 21 {
                    break;
                }
                if m.word(p + 20) > position {
                    break;
                }
                found = found.wrapping_add(1);
            }
            let ready = found < half(m, mb + 8)
                && m.word(markers + found as u32 * 40 + 20) == position
                && half(m, markers + found as u32 * 40 + 10) == 21;
            if !ready {
                let (rc, b) = fetch(
                    m,
                    sequence,
                    0x3fd,
                    half(m, mb + 8).wrapping_add(1) as u32,
                    &e.markers,
                    h,
                );
                if rc >= 0x80000000 {
                    return rc;
                }
                mb = b;
                markers = m.word(mb + 4);
                let boundary = markers + next as u32 * 40;
                mov(
                    m,
                    boundary + 40,
                    boundary,
                    (half(m, mb + 8) as u32 - next as u32) * 40,
                );
                copy(m, boundary, boundary + 40, 40);
                m.put_half(boundary + 10, 21);
                m.put_word(boundary + 20, position);
                m.put_word(boundary + 24, 0);
                m.put_word(boundary + 16, 0);
                m.put_word(boundary + 32, 0);
                if last_boundary != 0 {
                    m.put_word(
                        boundary + 28,
                        m.word(markers + last_boundary as u32 * 40 + 28),
                    );
                } else if normal_index != u32::MAX {
                    m.put_word(boundary + 28, normal_index);
                } else {
                    let mut offset = 0u32;
                    let mut candidate = m.word(sb + 4);
                    while candidate != 0 && offset < half(m, sb + 8) as u32 {
                        let n = len16(m, candidate);
                        if cmp16(m, candidate, e.markers.normal) == 0 {
                            normal_index = offset;
                            break;
                        }
                        offset = offset.wrapping_add(n + 1);
                        candidate = candidate.wrapping_add((n + 1) * 2);
                    }
                    if normal_index == u32::MAX {
                        let n = len16(m, e.markers.normal);
                        let required = (half(m, sb + 8) as u32 + n + 1) as u16;
                        if half(m, sb + 10) < required {
                            let (rc, b) = fetch(m, sequence, 0x3f4, required as u32, &e.markers, h);
                            if rc >= 0x80000000 {
                                return rc;
                            }
                            sb = b;
                        }
                        let stored =
                            m.word(sb + 4) + half(m, sb + 8) as u32 * half(m, sb + 2) as u32;
                        fe_text::strcpy16_176214(
                            m.bytes,
                            stored - m.base,
                            e.markers.normal - m.base,
                        );
                        m.put_half(stored + n * 2, 0);
                        normal_index = half(m, sb + 8) as u32;
                        m.put_half(sb + 8, required);
                    }
                    m.put_word(boundary + 28, normal_index);
                }
                m.put_half(mb + 8, half(m, mb + 8).wrapping_add(1));
            }
            let boundary = markers + next as u32 * 40;
            current = markers + index as u32 * 40;
            for s in 0..2 {
                let position = m.word(if s != 0 { boundary + 20 } else { current + 20 });
                let at = (position.wrapping_sub(m.word(markers + 20)) >> 1) as u16;
                let excluded;
                if s != 0 {
                    if at == 0
                        || at >= half(m, tb + 8)
                        || half(m, text + at as u32 * 2) == 32
                        || half(m, text + (at as u32 - 1) * 2) == 32
                    {
                        continue;
                    }
                    excluded = next;
                } else {
                    if at == 0 || at == 1 || half(m, text + (at as u32 - 1) * 2) == 32 {
                        continue;
                    }
                    excluded = half(m, mb + 8).wrapping_add(1);
                }
                let (rc, b) = fetch(
                    m,
                    sequence,
                    0x3fc,
                    half(m, tb + 8).wrapping_add(1) as u32,
                    &e.markers,
                    h,
                );
                if rc >= 0x80000000 {
                    return rc;
                }
                tb = b;
                text = m.word(tb + 4);
                mov(
                    m,
                    text + (at as u32 + 1) * 2,
                    text + at as u32 * 2,
                    (half(m, tb + 8) as u32 - at as u32) * 2,
                );
                m.put_half(text + at as u32 * 2, 32);
                m.put_half(tb + 8, half(m, tb + 8).wrapping_add(1));
                for n in 1..half(m, mb + 8) {
                    let p = markers + n as u32 * 40;
                    if s == 0 {
                        if position > m.word(p + 20) {
                            continue;
                        }
                        if half(m, p + 10) != 21 || m.word(p + 20) != position {
                            m.put_word(p + 20, m.word(p + 20).wrapping_add(2));
                        }
                    } else if n != excluded && position <= m.word(p + 20) {
                        m.put_word(p + 20, m.word(p + 20).wrapping_add(2));
                    }
                }
                m.put_word(markers + 24, m.word(markers + 24).wrapping_add(2));
            }
            if half(m, current + 10) == 34 && next < half(m, mb + 8) && half(m, boundary + 10) == 21
            {
                let mut found = 1u16;
                while found < half(m, mb + 8) {
                    let p = markers + found as u32 * 40;
                    if half(m, p + 10) == 1 && m.word(p + 20) == m.word(current + 20) {
                        break;
                    }
                    found = found.wrapping_add(1);
                }
                let p = markers + found as u32 * 40;
                if half(m, p + 10) == 1 && m.word(p + 20) == m.word(current + 20) {
                    fix_word_length(m, markers, p, text, e);
                    found = found.wrapping_add(1);
                    while found < half(m, mb + 8) {
                        let p = markers + found as u32 * 40;
                        if half(m, p + 10) == 1
                            && m.word(p + 20) >= m.word(current + 20)
                            && m.word(p + 20) < m.word(markers + next as u32 * 40 + 20)
                        {
                            if half(m, mb + 8) != found.wrapping_add(1) {
                                mov(m, p, p + 40, (half(m, mb + 8) as u32 - found as u32) * 40);
                            }
                            found = found.wrapping_sub(1);
                            next = next.wrapping_sub(1);
                            m.put_half(mb + 8, half(m, mb + 8).wrapping_sub(1));
                        }
                        found = found.wrapping_add(1);
                    }
                }
            }
            index = next.wrapping_sub(1);
        }
        index = index.wrapping_add(1);
    }
    for s in 0..half(m, mb + 8) {
        let p = markers + s as u32 * 40;
        let replacement = m.word(p + 32);
        if replacement != 0 {
            free(m, m.word(context + 4), replacement, h);
            m.put_word(p + 32, 0);
        }
    }
    0
}
fn fix_word_length(
    m: &mut Memory<'_>,
    markers: u32,
    word: u32,
    text: u32,
    e: &ExtendedEnvironment,
) {
    let mut at = (m.word(word + 20).wrapping_sub(m.word(markers + 20)) >> 1) as u16;
    let limit = (m.word(markers + 24) >> 1) as u16;
    let mut count = 0u16;
    while at < limit && !separator(m, e, half(m, text + at as u32 * 2)) {
        at = at.wrapping_add(1);
        count = count.wrapping_add(1);
    }
    m.put_word(word + 24, count as u32 * 2);
}

pub trait WriterHost: Host + crate::render_part_b::RenderLog {
    fn write(
        &mut self,
        memory: &mut Memory<'_>,
        entry: u32,
        handle: [u32; 2],
        data: u32,
        size: u32,
    ) -> u32;
}

pub struct WriterEnvironment {
    pub before_write: u32,
    pub diagnostics: crate::render_part_b::RenderEnvironment,
    pub log_record: u32,
    pub content_type: u32,
}

pub fn fn_26b140(
    m: &mut Memory<'_>,
    context: u32,
    writer: u32,
    handle: [u32; 2],
    source: u32,
    info: u32,
    force: bool,
    e: &WriterEnvironment,
    h: &mut impl WriterHost,
) -> u32 {
    assert_eq!(
        m.base, 0,
        "writer diagnostics require absolute-index memory"
    );
    if !force && half(m, info + 8) == 1 && half(m, source + 10) == 0x4000 {
        return 0;
    }
    let heap = m.word(context + 4);
    let markers = frontend::call_method(
        m.bytes,
        opt(m, heap),
        1,
        half(m, info + 8) as u32 * 24 + 1,
        h,
    );
    if markers == 0 {
        return 0x8110200a;
    }
    let mut i = 0u16;
    while i < half(m, info + 8) {
        let dst = markers + i as u32 * 24;
        let src = source + i as u32 * 40;
        m.put_half(dst, half(m, src + 10));
        i = i.wrapping_add(1);
        for j in 0..5 {
            m.put_word(dst + 4 + j * 4, m.word(src + 12 + j * 4));
        }
    }
    debug_markers(
        m,
        m.byte(writer + 120) as u32,
        context,
        0,
        e.before_write,
        1,
        markers,
        half(m, info + 8) as u32,
        &e.diagnostics,
        h,
    );
    let result = h.write(
        m,
        m.word(m.word(writer + 8) + 52),
        handle,
        markers,
        half(m, info + 8) as u32 * 24,
    );
    let log = m.word(context + 16);
    if result < 0x80000000
        && normalization::flag_byte(m.bytes, if log == 0 { 0 } else { log - m.base }) != 0
    {

        normalization::fanout_call(
            m.bytes,
            log - m.base,
            0x22,
            e.log_record,
            e.content_type,
            markers,
            half(m, info + 8) as u32 * 24,
        );
    }
    free(m, heap, markers, h);
    result
}

pub trait SequenceWriterHost: WriterHost {
    fn position(&mut self, memory: &mut Memory<'_>, entry: u32, handle: [u32; 2]) -> (u32, u32);
}
pub struct SequenceWriterEnvironment {
    pub extended: ExtendedEnvironment,
    pub writer: WriterEnvironment,
    pub labels: [u32; 4],
    pub text_content_type: u32,
    pub empty_string: u32,
}
fn adjust_kind(k: u16) -> bool {
    matches!(k, 6 | 12 | 21 | 50 | 36 | 39 | 37 | 34 | 22 | 40 | 26 | 29)
}

pub fn fn_269ee4(
    m: &mut Memory<'_>,
    context: u32,
    writer: u32,
    sequence: u32,
    text_handle: [u32; 2],
    marker_handle: [u32; 2],
    e: &SequenceWriterEnvironment,
    h: &mut impl SequenceWriterHost,
) -> u32 {
    assert_eq!(
        m.base, 0,
        "writer diagnostics require absolute-index memory"
    );
    if writer == 0 || sequence == 0 {
        return 0x81102007;
    }
    let (rc, position) = h.position(m, m.word(m.word(writer + 8) + 68), text_handle);
    if rc >= 0x80000000 {
        return rc;
    }
    let mode = m.byte(writer + 120) as u32;
    debug_markers(
        m,
        mode,
        context,
        sequence,
        e.labels[0],
        0,
        0,
        0,
        &e.writer.diagnostics,
        h,
    );
    crate::render_part_b::fn_263384(
        m,
        mode as i32,
        context,
        sequence,
        0x3fc,
        e.labels[0],
        &e.writer.diagnostics,
        h,
    );
    let rc = fn_26783c(m, context, sequence, &e.extended, h);
    if rc >= 0x80000000 {
        return rc;
    }
    debug_markers(
        m,
        mode,
        context,
        sequence,
        e.labels[1],
        0,
        0,
        0,
        &e.writer.diagnostics,
        h,
    );
    crate::render_part_b::fn_263384(
        m,
        mode as i32,
        context,
        sequence,
        0x3fc,
        e.labels[1],
        &e.writer.diagnostics,
        h,
    );
    let (rc, buffer) = fetch(m, sequence, 0x3fc, 1, &e.extended.markers, h);
    if rc >= 0x80000000 {
        return rc;
    }
    let wrote = half(m, buffer + 8) != 0;
    let data = if wrote { m.word(buffer + 4) } else { 0 };
    let size = if wrote {
        half(m, buffer + 8) as u32 * half(m, buffer + 2) as u32
    } else {
        0
    };
    if wrote {
        let rc = h.write(m, m.word(m.word(writer + 8) + 52), text_handle, data, size);
        if rc >= 0x80000000 {
            return rc;
        }
    }
    let log = m.word(context + 16);
    if normalization::flag_byte(m.bytes, if log == 0 { 0 } else { log - m.base }) != 0 {
        normalization::fanout_call(
            m.bytes,
            log - m.base,
            0x22,
            e.empty_string,
            e.text_content_type,
            data,
            size,
        );
    }
    if wrote {
        m.put_half(buffer + 8, 0);
    }
    let (rc, buffer) = fetch(m, sequence, 0x3f4, 1, &e.extended.markers, h);
    let text = if rc < 0x80000000 {
        m.word(buffer + 4)
    } else {
        0
    };
    let (rc, markers) = fetch(m, sequence, 0x3fd, 1, &e.extended.markers, h);
    if rc >= 0x80000000 {
        return rc;
    }
    let source = m.word(markers + 4);
    if half(m, markers + 8) == 0 || half(m, source + 10) != 0x4000 {
        return 0x81102000;
    }
    let delta = m.word(source + 20).wrapping_sub(position);
    let n = half(m, markers + 8);
    for i in 0..n {
        let p = source + i as u32 * 40;
        m.put_word(p + 20, m.word(p + 20).wrapping_sub(delta));
    }
    for i in 0..n {
        let p = source + i as u32 * 40;
        if adjust_kind(half(m, p + 10)) {
            let arg = m.word(p + 28);
            m.put_word(
                p + 28,
                if arg != u32::MAX {
                    text.wrapping_add(arg.wrapping_mul(2))
                } else {
                    0
                },
            );
        }
    }
    debug_markers(
        m,
        mode,
        context,
        sequence,
        e.labels[2],
        1,
        0,
        0,
        &e.writer.diagnostics,
        h,
    );
    let rc = fn_268a14(m, sequence, markers, &e.extended.markers, h);
    if rc >= 0x80000000 {
        return rc;
    }
    let source = m.word(markers + 4);
    debug_markers(
        m,
        mode,
        context,
        sequence,
        e.labels[3],
        1,
        0,
        0,
        &e.writer.diagnostics,
        h,
    );
    let rc = fn_26b140(
        m,
        context,
        writer,
        marker_handle,
        source,
        markers,
        wrote,
        &e.writer,
        h,
    );
    if rc < 0x80000000 {
        0
    } else {
        rc
    }
}

pub trait StreamHost:
    Host + crate::boot_part_b::FormatHost + crate::render_part_b::RenderLog
{
    fn stream_call(
        &mut self,
        m: &mut Memory<'_>,
        entry: u32,
        args: &[u32],
        output: &mut [u32; 2],
    ) -> u32;
    fn stream_parameter(
        &mut self,
        m: &mut Memory<'_>,
        entry: u32,
        handle: [u32; 2],
        text: &[u16],
    ) -> u32;
}
pub struct StreamEnvironment {
    pub default_language: u32,
    pub automatic_language: u32,
    pub empty: u32,
    pub underscore: u32,
    pub hash: u32,
    pub long_query: u32,
    pub short_query: u32,
    pub log_category: u32,
    pub query_format: u32,

    pub log_scratch: u32,
}
fn stream_error(m: &mut Memory<'_>, logger: u32, e: &StreamEnvironment, h: &mut impl StreamHost) {
    if logger == 0 {
        return;
    }
    let mut saved = [0u8; 20];
    let at = m.offset(e.log_scratch);
    saved.copy_from_slice(&m.bytes[at..at + 20]);
    crate::boot_part_b::fn_173c70(m, logger, e.log_category, 60000, 0, 0, e.log_scratch, h);
    m.bytes[at..at + 20].copy_from_slice(&saved);
}
fn stream_api(m: &Memory<'_>, p: u32, slot: u32) -> u32 {
    m.word(m.word(p + 24) + slot)
}
fn stream_invoke(
    m: &mut Memory<'_>,
    p: u32,
    slot: u32,
    tail: &[u32],
    out: &mut [u32; 2],
    h: &mut impl StreamHost,
) -> u32 {
    let mut args = [0; 7];
    args[0] = m.word(p + 36);
    args[1] = m.word(p + 40);
    args[2..2 + tail.len()].copy_from_slice(tail);
    h.stream_call(m, stream_api(m, p, slot), &args[..2 + tail.len()], out)
}
fn stream_parameter(
    m: &mut Memory<'_>,
    p: u32,
    source: u32,
    prefix: &[u8],
    h: &mut impl StreamHost,
) -> u32 {
    let mut arena = [0u8; 4096];
    arena[2048..2048 + prefix.len()].copy_from_slice(prefix);
    fe_text::strcpy_widen_1763e4(&mut arena, 0, 2048);
    let source_units = len16(m, source) as usize;
    let n = prefix.len() + source_units + 1;
    assert!(
        n < 1024,
        "stream parameter exceeds the original stack buffer"
    );
    let source_at = m.offset(source);
    arena[2048..2048 + (source_units + 1) * 2]
        .copy_from_slice(&m.bytes[source_at..source_at + (source_units + 1) * 2]);
    crate::render_part_b::fn_1761d4(
        &mut Memory {
            base: 0,
            bytes: &mut arena,
        },
        0,
        2048,
    );
    arena[2048] = 92;
    arena[2049] = 0;

    fe_text::strcpy_widen_1763e4(&mut arena, (n as u32 - 1) * 2, 2048);
    let mut text = [0u16; 1024];
    for i in 0..n {
        text[i] = u16::from_le_bytes([arena[i * 2], arena[i * 2 + 1]]);
    }
    h.stream_parameter(
        m,
        m.word(m.word(p + 20) + 52),
        [m.word(p + 44), m.word(p + 48)],
        &text[..n],
    )
}

pub fn fn_26d330(
    m: &mut Memory<'_>,
    p: u32,
    text: u32,
    start: u32,
    mut end: u32,
    t1: u32,
    t2: u32,
    e: &StreamEnvironment,
    h: &mut impl StreamHost,
) -> u32 {
    m.put_half(p + 0xc8, 0);
    frontend::copy_str(m.bytes, m.word(p + 52) - m.base, e.empty - m.base);
    let mut code = [0u8; 4];
    if t1 != 0 {
        let src = if cmp16(m, t1, e.default_language) != 0 {
            t1
        } else {
            p + 64
        };
        for k in 0..3 {
            code[k] = normalization::tolower_240ba0(half(m, src + k as u32 * 2) as u8 as u32) as u8;
        }
    }
    while end > start {
        if half(m, text + (end & !1) - 2) as u8 > 32 {
            break;
        }
        end = end.wrapping_sub(2);
    }
    if end == start {
        return 0;
    }
    let owner = m.word(p);
    let heap = m.word(owner + 4);
    let nd = frontend::call_method(
        m.bytes,
        opt(m, heap),
        half(m, m.word(p + 12) + 8) as u32,
        1,
        h,
    );
    if nd == 0 {
        stream_error(m, m.word(owner + 16), e, h);
        return 0x8c90200a;
    }
    let mut out = [0; 2];
    let mut rc = stream_invoke(m, p, 32, &[], &mut out, h);

    let mut cleanup = true;
    let result = (|| {
        if rc >= 0x80000000 {
            return rc;
        }
        if t1 != 0 {
            rc = stream_parameter(m, p, t1, b"\x1b\\lang=", h);
            if rc >= 0x80000000 {
                return rc;
            }
        }
        if cmp16(m, t2, e.default_language) != 0 && cmp16(m, t2, e.automatic_language) != 0 {
            rc = stream_parameter(m, p, t2, b"Calling Process for %s", h);
            if rc >= 0x80000000 {
                return rc;
            }
        }
        rc = h.stream_call(
            m,
            m.word(m.word(p + 20) + 52),
            &[
                m.word(p + 44),
                m.word(p + 48),
                text + (start & !1),
                end.wrapping_sub(start),
            ],
            &mut out,
        );
        if rc >= 0x80000000 {
            return rc;
        }
        let mut idx = 0u16;
        while idx < half(m, m.word(p + 12) + 8) {
            let it = m.word(m.word(m.word(p + 12) + 4) + idx as u32 * 4);
            info_log(
                m,
                m.word(owner + 16),
                e.log_category,
                4,
                e.query_format,
                &[crate::render_part_b::RenderArgument::Word(m.word(it + 4))],
                h,
            );
            rc = h.stream_call(
                m,
                m.word(m.word(it + 8) + 52),
                &[
                    m.word(it + 12),
                    m.word(it + 16),
                    m.word(p + 36),
                    m.word(p + 40),
                    nd + idx as u32,
                ],
                &mut out,
            );
            if stream_api(m, p, 100) != 0 {
                let mut b = [0; 2];
                let r = stream_invoke(m, p, 104, &[0], &mut b, h);
                if r < 0x80000000 && b[0] as u8 != 0 {
                    stream_invoke(m, p, 100, &[0, m.word(it + 4), 1], &mut b, h);
                }
            }
            idx = idx.wrapping_add(1);
            if rc >= 0x80000000 {
                return rc;
            }
        }
        out[0] = 0;
        rc = stream_invoke(m, p, 52, &[1, 0, 0], &mut out, h);
        let o28 = out[0] as u16;
        if rc >= 0x80000000 {
            return rc;
        }
        out[0] = 0;
        let probe = stream_invoke(m, p, 92, &[o28 as u32, 0, 0], &mut out, h);
        let mut o12 = out[0] as u16;
        let mut o16 = 0u16;
        let mut b24 = 0u16;
        let mut b20 = 0u16;
        let mut tb1 = 0u8;
        if probe < 0x80000000 && o12 == 1 {
            out = [m.word(p + 0x2cc), b20 as u32];
            rc = stream_invoke(m, p, 88, &[o28 as u32, 0, p + 0x2cc, 0], &mut out, h);
            m.put_word(p + 0x2cc, out[0]);
            b20 = out[1] as u16;
            if rc >= 0x80000000 {
                return rc;
            }
            out[0] = o12 as u32;
            let probe = stream_invoke(m, p, 76, &[2, o28 as u32, 0], &mut out, h);
            o12 = out[0] as u16;
            if probe < 0x80000000 && o12 != 0 {
                out[0] = o16 as u32;
                rc = stream_invoke(m, p, 52, &[2, o28 as u32, 0], &mut out, h);
                o16 = out[0] as u16;
                if rc >= 0x80000000 {
                    return rc;
                }
            }
            m.put_half(p + 56, 0);
            m.put_half(p + 58, 0);
            let mut niter = 0u16;
            let mut tb2 = 0u32;
            let mut tb3 = 0u32;
            while o16 != 0 {
                out = [tb1 as u32, b20 as u32];
                rc = stream_invoke(m, p, 84, &[o16 as u32, 0, 1, 0, 0], &mut out, h);
                tb1 = out[0] as u8;
                b20 = out[1] as u16;
                if rc >= 0x80000000 {
                    return rc;
                }
                if matches!(tb1, 4 | 9 | 10) {
                    out = [tb2, b20 as u32];
                    rc = stream_invoke(m, p, 88, &[o16 as u32, 3, 0, 0], &mut out, h);
                    tb2 = out[0];
                    b20 = out[1] as u16;
                    if rc >= 0x80000000 {
                        return rc;
                    }
                    if cmp16(m, t2, e.automatic_language) == 0 {
                        out = [tb3, b20 as u32];
                        rc = stream_invoke(m, p, 88, &[o16 as u32, 9, 0, 0], &mut out, h);
                        tb3 = out[0];
                        b20 = out[1] as u16;
                        if rc >= 0x80000000 {
                            return rc;
                        }
                        let mut buf = [0u8; 4];
                        for k in 0..3 {
                            let c = if m.byte(tb3) == 0 {
                                half(m, p + 64 + k as u32 * 2) as u8
                            } else {
                                m.byte(tb3 + k as u32)
                            };
                            buf[k] = normalization::tolower_240ba0(c as u32) as u8;
                        }
                        if frontend::compare_str(&code, &buf) != 0 && half(m, p + 0xc8) < 64 {
                            let oc = half(m, p + 0xc8);

                            let mut local = [0u8; 16];
                            local[8..12].copy_from_slice(&buf);
                            fe_text::strcpy_widen_1763e4(&mut local, 0, 8);
                            let dest = p + 0xca + oc as u32 * 8;
                            let n = (buf.iter().position(|&c| c == 0).unwrap() + 1) * 2;
                            let at = m.offset(dest);
                            m.bytes[at..at + n].copy_from_slice(&local[..n]);
                            m.put_half(p + 0xc8, oc.wrapping_add(1));
                            m.put_half(p + 72 + oc as u32 * 2, niter);
                        }
                        niter = niter.wrapping_add(1);
                        for k in 0..4 {
                            code[k] = normalization::tolower_240ba0(buf[k] as u32) as u8;
                        }
                    }
                }
                out[0] = o12 as u32;
                let probe = stream_invoke(m, p, 92, &[o16 as u32, 8, 0], &mut out, h);
                o12 = out[0] as u16;
                if probe < 0x80000000 && o12 == 1 {
                    out = [b24 as u32, b20 as u32];
                    let probe = stream_invoke(m, p, 84, &[o16 as u32, 8, 1, 0, 0], &mut out, h);
                    b20 = out[1] as u16;
                    if probe >= 0x80000000 {
                        cleanup = false;
                        return probe;
                    }
                    b24 = out[0] as u16;
                }
                let n = len(m, tb2);
                if n != 0
                    && frontend::compare_str(
                        &m.bytes[m.offset(tb2)..],
                        &m.bytes[m.offset(e.underscore)..],
                    ) != 0
                {
                    let old_len = half(m, p + 56);
                    if n.wrapping_add(1).wrapping_add(old_len as u32) >= 1024 {
                        let nb = realloc(
                            m,
                            heap,
                            m.word(p + 52),
                            n.wrapping_add(2).wrapping_add(old_len as u32),
                            h,
                        );
                        if nb == 0 {
                            stream_error(m, m.word(owner + 16), e, h);
                            return 0x8c90200a;
                        }
                        m.put_word(p + 52, nb);
                    }
                    m.put_half(p + 56, old_len.wrapping_add(n as u16).wrapping_add(1));
                    let dst = m.word(p + 52);
                    frontend::append_str(m.bytes, dst - m.base, tb2 - m.base);
                    frontend::append_str(
                        m.bytes,
                        dst - m.base,
                        if b24 != 0 {
                            e.hash - m.base
                        } else {
                            e.underscore - m.base
                        },
                    );
                }
                if b24 != 0 {
                    put_byte(
                        m,
                        m.word(p + 52)
                            .wrapping_add(half(m, p + 56) as u32)
                            .wrapping_sub(1),
                        35,
                    );
                }
                out[0] = o16 as u32;
                rc = stream_invoke(m, p, 60, &[o16 as u32, 0], &mut out, h);
                o16 = out[0] as u16;
                if rc >= 0x80000000 {
                    return rc;
                }
            }
        }
        let buffer = m.word(p + 52);
        let last = buffer.wrapping_add(half(m, p + 56) as u32).wrapping_sub(1);
        if m.byte(last) == 95 {
            put_byte(m, last, 0);
            m.put_half(p + 56, half(m, p + 56).wrapping_sub(1));
        }
        for (query, n) in [(e.long_query, 6), (e.short_query, 4)] {
            let hit = frontend::find_substr(m.bytes, m.word(p + 52) - m.base, query - m.base);
            if hit != 0 {
                let hit = hit + m.base;
                let nbytes = len(m, hit + n);
                mov(m, hit, hit + n, nbytes + 1);
                m.put_half(p + 56, half(m, p + 56).wrapping_sub(n as u16));
            }
        }
        out[0] = o12 as u32;
        let probe = stream_invoke(m, p, 92, &[o28 as u32, 1, 0], &mut out, h);
        if probe >= 0x80000000 || out[0] as u16 != 1 {
            return rc;
        }
        out = [0; 2];
        rc = stream_invoke(m, p, 88, &[o28 as u32, 1, 0, 0], &mut out, h);
        if rc >= 0x80000000 {
            cleanup = false;
            return rc;
        }
        let p24 = out[0];
        let o08 = out[1] as u16;
        let mut acc = 0u16;
        for i in 0..o08 {
            if half(m, p24 + i as u32 * 24) == 1 {
                acc = acc.wrapping_add(1);
            }
        }
        let nb = realloc(m, heap, m.word(p + 0x2d0), acc as u32 * 4, h);
        if nb == 0 {
            stream_error(m, m.word(owner + 16), e, h);
            return 0x8c90200a;
        }
        m.put_word(p + 0x2d0, nb);
        let mut i2 = 0u16;
        for i in 0..o08 {
            if half(m, p24 + i as u32 * 24) == 1 {
                m.put_word(
                    nb + i2 as u32 * 4,
                    m.word(p24 + i as u32 * 24 + 12)
                        .wrapping_sub(m.word(p24 + 12)),
                );
                i2 = i2.wrapping_add(1);
            }
        }
        rc
    })();
    if cleanup {
        free(m, heap, nd, h);
    }
    result
}

pub trait RenderWriterHost: SequenceWriterHost + StreamHost + crate::render_part_c::Host {}
pub struct RenderWriterEnvironment {
    pub sequence: SequenceWriterEnvironment,
    pub latin: u32,
    pub rewrite: crate::render_part_b::RewriteEnvironment,
    pub labels: [u32; 11],
    pub content_type: u32,
    pub log_category: u32,
    pub input_format: u32,
    pub output_format: u32,

    pub scratch: u32,
}
fn blank(m: &Memory<'_>, text: u32, mut at: u16, end: u32) -> bool {
    while (at as u32) < end {
        if !matches!(m.byte(text + at as u32), b' ' | b'\t' | b'\n' | b'\r') {
            return false;
        }
        at = at.wrapping_add(1);
    }
    true
}

pub fn fn_268cbc(
    m: &mut Memory<'_>,
    context: u32,
    state: u32,
    sequence: u32,
    text_handle: [u32; 2],
    marker_handle: [u32; 2],
    e: &RenderWriterEnvironment,
    h: &mut impl RenderWriterHost,
) -> u32 {
    assert_eq!(m.base, 0, "render callees require absolute-index memory");
    if state == 0 || sequence == 0 {
        return 0x81102007;
    }
    let (rc, position) = h.position(m, m.word(m.word(state + 8) + 68), text_handle);
    if rc >= 0x80000000 {
        return rc;
    }
    let at = m.offset(e.scratch);
    let mut saved = [0u8; 64];
    saved.copy_from_slice(&m.bytes[at..at + 64]);
    let mut owned = 0u32;
    let ee = &e.sequence.extended.markers;
    let mode = m.byte(state + 120) as u32;
    let heap = m.word(context + 4);
    let result = (|| {
        let (rc, mut text_buffer) = fetch(m, sequence, 0x3f4, 1, ee, h);
        let mut text = if rc < 0x80000000 {
            m.word(text_buffer + 4)
        } else {
            0
        };
        let (rc, markers) = fetch(m, sequence, 0x3fd, 1, ee, h);
        if rc >= 0x80000000 {
            return rc;
        }
        let mut records = m.word(markers + 4);
        if half(m, markers + 8) == 0 || half(m, records + 10) != 0x4000 {
            return 0x81102000;
        }
        let delta = m.word(records + 20).wrapping_sub(position);
        for i in 0..half(m, markers + 8) {
            let p = records + i as u32 * 40;
            m.put_word(p + 20, m.word(p + 20).wrapping_sub(delta));
        }
        debug_markers(
            m,
            mode,
            context,
            sequence,
            e.labels[0],
            0,
            0,
            0,
            &e.sequence.writer.diagnostics,
            h,
        );
        crate::render_part_b::fn_263384(
            m,
            mode as i32,
            context,
            sequence,
            0x3fc,
            e.labels[0],
            &e.sequence.writer.diagnostics,
            h,
        );
        let (rc, string_buffer) = fetch(m, sequence, 0x3fc, 1, ee, h);
        if rc >= 0x80000000 {
            return rc;
        }
        owned = frontend::call_method(
            m.bytes,
            opt(m, heap),
            1,
            half(m, string_buffer + 8) as u32 + 1,
            h,
        );
        if owned == 0 {
            return 0x8110200a;
        }
        fe_text::narrow_n_17649c(
            m.bytes,
            owned - m.base,
            m.word(string_buffer + 4) - m.base,
            half(m, string_buffer + 8) as u32,
        );
        put_byte(m, owned + half(m, string_buffer + 8) as u32, 0);
        info_log(
            m,
            m.word(context + 16),
            e.log_category,
            4,
            e.input_format,
            &[crate::render_part_b::RenderArgument::Guest(owned)],
            h,
        );
        let rc = fn_26a610(m, sequence, 0x3fd, ee, h);
        if rc >= 0x80000000 {
            return rc;
        }
        debug_markers(
            m,
            mode,
            context,
            sequence,
            e.labels[1],
            0,
            0,
            0,
            &e.sequence.writer.diagnostics,
            h,
        );
        fn_266fc4(m, context, records, markers + 8, owned, ee, h);
        debug_markers(
            m,
            mode,
            context,
            sequence,
            e.labels[2],
            0,
            0,
            0,
            &e.sequence.writer.diagnostics,
            h,
        );
        m.put_word(e.scratch, owned);
        let rc = crate::render_part_b::fn_2663b0(
            m,
            context,
            sequence,
            e.scratch,
            markers,
            text_buffer,
            &e.rewrite,
            e.scratch + 4,
            h,
        );
        owned = m.word(e.scratch);
        if rc >= 0x80000000 {
            return rc;
        }
        let (rc, b) = fetch(m, sequence, 0x3f4, 1, ee, h);
        if rc < 0x80000000 {
            text_buffer = b;
            text = m.word(b + 4);
        }
        records = m.word(markers + 4);
        for i in 0..half(m, markers + 8) {
            let p = records + i as u32 * 40;
            if adjust_kind(half(m, p + 10)) {
                let tag = m.word(p + 28);
                m.put_word(
                    p + 28,
                    if tag != u32::MAX {
                        text.wrapping_add(tag.wrapping_mul(2))
                    } else {
                        0
                    },
                );
            }
        }
        debug_markers(
            m,
            mode,
            context,
            sequence,
            e.labels[3],
            1,
            0,
            0,
            &e.sequence.writer.diagnostics,
            h,
        );
        fn_26a7b8(m, records, markers + 8, sequence, owned, ee);
        debug_markers(
            m,
            mode,
            context,
            sequence,
            e.labels[4],
            0,
            0,
            0,
            &e.sequence.writer.diagnostics,
            h,
        );
        let cenv = crate::render_part_c::Environment {
            cached_handle: ee.cached_handle,
            default_context: ee.default_context,
        };
        let rc = crate::render_part_c::fn_2bb4e4(
            m,
            m.word(state + 60),
            sequence,
            owned,
            markers + 8,
            &cenv,
            h,
        );
        if rc >= 0x80000000 {
            return rc;
        }
        debug_markers(
            m,
            mode,
            context,
            sequence,
            e.labels[5],
            1,
            0,
            0,
            &e.sequence.writer.diagnostics,
            h,
        );
        let mut i = 1u16;
        while i < half(m, markers + 8) {
            let region = records + i as u32 * 40;
            if half(m, region + 10) == 36
                && m.word(region + 28) != 0
                && cmp16(m, m.word(region + 28), e.latin) == 0
            {
                let start = m.word(region + 20).wrapping_sub(m.word(records + 20)) as u16;
                let mut saw = false;
                let mut j = i.wrapping_add(1);
                while j < half(m, markers + 8) {
                    let next = records + j as u32 * 40;
                    if half(m, next + 10) == 36 {
                        saw = true;
                        if (m.word(next + 28) == 0 || cmp16(m, m.word(next + 28), e.latin) != 0)
                            && blank(
                                m,
                                owned,
                                start,
                                m.word(next + 20).wrapping_sub(m.word(records + 20)),
                            )
                        {
                            m.put_word(next + 12, m.word(region + 12));
                            m.put_word(
                                next + 16,
                                m.word(next + 16).wrapping_add(m.word(region + 16)),
                            );
                            m.put_word(next + 20, m.word(region + 20));
                            m.put_word(
                                next + 24,
                                m.word(next + 24).wrapping_add(m.word(region + 24)),
                            );
                            if m.byte(region) == 1 {
                                mov(
                                    m,
                                    region,
                                    region + 40,
                                    (half(m, markers + 8) as u32 - i as u32) * 40,
                                );
                                m.put_half(markers + 8, half(m, markers + 8).wrapping_sub(1));
                            } else {
                                m.put_half(region + 10, 999);
                            }
                            i = j;
                        }
                        break;
                    }
                    j = j.wrapping_add(1);
                }
                if !saw && blank(m, owned, start, m.word(records + 24)) {
                    if m.byte(region) == 1 {
                        if (i as u32 + 1) < half(m, markers + 8) as u32 {
                            mov(
                                m,
                                region,
                                region + 40,
                                (half(m, markers + 8) as u32 - i as u32) * 40,
                            );
                        }
                        m.put_half(markers + 8, half(m, markers + 8).wrapping_sub(1));
                    } else {
                        m.put_half(region + 10, 999);
                    }
                }
            }
            i = i.wrapping_add(1);
        }
        if half(m, markers + 8) > 1 {
            let mut output = 1u16;
            for i in 1..half(m, markers + 8) {
                let p = records + i as u32 * 40;
                if half(m, p + 10) == 36 {
                    let mut previous = i.wrapping_sub(1);
                    while previous != 0 && half(m, records + previous as u32 * 40 + 10) != 36 {
                        previous = previous.wrapping_sub(1);
                    }
                    if previous == 0
                        || cmp16(
                            m,
                            m.word(p + 28),
                            m.word(records + previous as u32 * 40 + 28),
                        ) != 0
                    {
                        record(m, records + output as u32 * 40, p);
                        output = output.wrapping_add(1);
                    } else if m.byte(p) == 0 {
                        m.put_half(p + 10, 999);
                        record(m, records + output as u32 * 40, p);
                        output = output.wrapping_add(1);
                    }
                } else {
                    record(m, records + output as u32 * 40, p);
                    output = output.wrapping_add(1);
                }
            }
            m.put_half(markers + 8, output);
        }
        debug_markers(
            m,
            mode,
            context,
            sequence,
            e.labels[6],
            1,
            0,
            0,
            &e.sequence.writer.diagnostics,
            h,
        );
        let mut count = half(m, markers + 8) as u32;
        let mut length = len(m, owned) as u16;
        for i in 1..count {
            if half(m, records + i * 40 + 10) == 34 {
                let mut j = i + 1;
                while j < count {
                    let boundary = records + j * 40;
                    if half(m, boundary + 10) == 21 {
                        let split = m.word(boundary + 20).wrapping_sub(m.word(records + 20)) as u16;
                        if split >= length
                            || matches!(m.byte(owned + split as u32), b' ' | b'\t' | b'\n' | b'\r')
                        {
                            break;
                        }
                        let nb = realloc(m, heap, owned, length as u32 + 2, h);
                        if nb == 0 {
                            return 0x8110200a;
                        }
                        owned = nb;
                        mov(
                            m,
                            owned + split as u32 + 1,
                            owned + split as u32,
                            length as u32 - split as u32,
                        );
                        length = length.wrapping_add(1);
                        put_byte(m, owned + split as u32, 32);
                        put_byte(m, owned + length as u32, 0);
                        for k in j..count {
                            let p = records + k * 40;
                            if m.word(p + 20) > m.word(boundary + 20) {
                                m.put_word(p + 20, m.word(p + 20).wrapping_add(1));
                            }
                        }
                        m.put_word(records + 24, m.word(records + 24).wrapping_add(1));
                        break;
                    }
                    j += 1;
                }
            }
        }
        debug_markers(
            m,
            mode,
            context,
            sequence,
            e.labels[7],
            1,
            0,
            0,
            &e.sequence.writer.diagnostics,
            h,
        );
        m.put_word(e.scratch, owned);
        let rc = fn_26abd0(m, context, sequence, e.scratch, ee, h);
        owned = m.word(e.scratch);
        if rc >= 0x80000000 {
            return rc;
        }
        debug_markers(
            m,
            mode,
            context,
            sequence,
            e.labels[8],
            0,
            0,
            0,
            &e.sequence.writer.diagnostics,
            h,
        );
        let (rc, _) = fetch(m, sequence, 0x3f4, 1, ee, h);
        if rc >= 0x80000000 {
            return rc;
        }
        records = m.word(markers + 4);
        i = 1;
        while i < half(m, markers + 8) {
            let current = records + i as u32 * 40;
            if half(m, current + 10) != 34 || m.word(current + 32) == 0 {
                i = i.wrapping_add(1);
                continue;
            }
            let mut following = i.wrapping_add(1);
            while following < half(m, markers + 8)
                && half(m, records + following as u32 * 40 + 10) != 21
            {
                following = following.wrapping_add(1);
            }
            if following == half(m, markers + 8) {
                free(m, heap, m.word(current + 32), h);
                m.put_word(current + 32, 0);
                crate::boot_part_b::fn_173c70(
                    m,
                    m.word(context + 16),
                    e.log_category,
                    0x789,
                    0,
                    0,
                    e.scratch + 4,
                    h,
                );
                return 0x81102000;
            }
            let boundary = records + following as u32 * 40;
            let start = m.word(current + 20).wrapping_sub(m.word(records + 20)) as u16;
            let end = m.word(boundary + 20).wrapping_sub(m.word(records + 20)) as u16;
            if m.byte(current + 36) == 0 {
                let old_length = len(m, owned) as u16;
                let change = start.wrapping_sub(end) as i16 as i32 as u32;
                mov(
                    m,
                    owned + start as u32,
                    owned + end as u32,
                    (old_length as i32 - end as i32 + 1) as u32,
                );
                let length = len(m, owned);
                m.put_word(records + 24, length);
                for p in 1..count {
                    let item = records + p * 40;
                    let item_start = m.word(item + 20);
                    let base = m.word(records + 20);
                    if item_start > base.wrapping_add(end as u32) {
                        if base.wrapping_add(length) > item_start {
                            m.put_word(item + 20, base.wrapping_add(end as u32));
                        }
                    } else if item_start < base.wrapping_add(length) {
                        if half(m, item + 10) == 1 {
                            let item_end = item_start.wrapping_add(m.word(item + 24));
                            if base.wrapping_add(length) < item_end
                                && item_end.wrapping_add(change) != 0
                            {
                                m.put_word(item + 24, m.word(item + 24).wrapping_add(change));
                            }
                        }
                    } else {
                        m.put_word(item + 20, item_start.wrapping_add(change));
                    }
                }
                free(m, heap, m.word(current + 32), h);
                m.put_word(current + 32, 0);
                if half(m, markers + 8) != following {
                    mov(
                        m,
                        current,
                        current + 40,
                        (half(m, markers + 8) as u32 - i as u32) * 40,
                    );
                }
                m.put_half(markers + 8, half(m, markers + 8).wrapping_sub(1));
                if half(m, markers + 8).wrapping_sub(1) != following.wrapping_add(1) {
                    mov(
                        m,
                        boundary,
                        boundary + 40,
                        (half(m, markers + 8) as u32 - 1 - following as u32) * 40,
                    );
                }
                m.put_half(markers + 8, half(m, markers + 8).wrapping_sub(1));
                count = half(m, markers + 8) as u32;
                i = following;
            } else {
                free(m, heap, m.word(current + 32), h);
                m.put_word(current + 32, 0);
                i = following;
            }
            i = i.wrapping_add(1);
        }
        debug_markers(
            m,
            mode,
            context,
            sequence,
            e.labels[9],
            1,
            0,
            0,
            &e.sequence.writer.diagnostics,
            h,
        );
        let rc = fn_268a14(m, sequence, markers, ee, h);
        if rc >= 0x80000000 {
            return rc;
        }
        records = m.word(markers + 4);
        info_log(
            m,
            m.word(context + 16),
            e.sequence.writer.diagnostics.category,
            5,
            e.output_format,
            &[crate::render_part_b::RenderArgument::Guest(owned)],
            h,
        );
        debug_markers(
            m,
            mode,
            context,
            sequence,
            e.labels[10],
            1,
            0,
            0,
            &e.sequence.writer.diagnostics,
            h,
        );
        let force = len(m, owned) != 0;
        let size = if force { len(m, owned) + 1 } else { 0 };
        if force {
            let rc = h.write(m, m.word(m.word(state + 8) + 52), text_handle, owned, size);
            if rc >= 0x80000000 {
                return rc;
            }
        }
        let log = m.word(context + 16);
        if normalization::flag_byte(m.bytes, if log == 0 { 0 } else { log - m.base }) != 0 {
            normalization::fanout_call(
                m.bytes,
                log - m.base,
                0x22,
                e.sequence.empty_string,
                e.content_type,
                if force { owned } else { 0 },
                size,
            );
        }
        m.put_half(string_buffer + 8, 0);
        let rc = fn_26b140(
            m,
            context,
            state,
            marker_handle,
            records,
            markers,
            force,
            &e.sequence.writer,
            h,
        );
        let capacity = half(m, state + 116);
        if rc < 0x80000000 && capacity != 0 && m.word(text_buffer + 4) != 0 {
            let nb = realloc(
                m,
                heap,
                m.word(text_buffer + 4),
                capacity as u32 * half(m, text_buffer + 2) as u32,
                h,
            );
            if nb == 0 {
                return 0x8110200a;
            }
            m.put_word(text_buffer + 4, nb);
            m.put_half(text_buffer + 8, capacity);
            m.put_half(text_buffer + 10, capacity);
        }
        rc
    })();
    if owned != 0 {
        free(m, heap, owned, h);
    }
    m.bytes[at..at + 64].copy_from_slice(&saved);
    result
}

pub trait WriterLiveHost: crate::render_part_c::Host + SequenceWriterHost {}
impl<T: crate::render_part_c::Host + SequenceWriterHost> WriterLiveHost for T {}

struct LiveHeap<'a, H> {
    heap: u32,
    host: &'a mut H,
}
impl<H: crate::render_part_c::Host> crate::frontend::HeapVtableM for LiveHeap<'_, H> {
    fn alloc_at_m(&mut self, object: u32, size: u32, mem: &mut [u8]) -> u32 {
        assert_eq!(
            object,
            crate::frontend::mem_word(mem, self.heap + 4),
            "writer heap changed heaps"
        );
        let method = crate::frontend::mem_word(mem, crate::frontend::mem_word(mem, self.heap));
        crate::render_part_c::Host::call(
            self.host,
            &mut Memory { base: 0, bytes: mem },
            method,
            &[object, size],
        )
    }
    fn realloc_at_m(&mut self, object: u32, old: u32, size: u32, mem: &mut [u8]) -> u32 {
        assert_eq!(
            object,
            crate::frontend::mem_word(mem, self.heap + 4),
            "writer heap changed heaps"
        );
        let method =
            crate::frontend::mem_word(mem, crate::frontend::mem_word(mem, self.heap) + 8);
        crate::render_part_c::Host::call(
            self.host,
            &mut Memory { base: 0, bytes: mem },
            method,
            &[object, old, size],
        )
    }
    fn release_at_m(&mut self, object: u32, pointer: u32, mem: &mut [u8]) {
        assert_eq!(
            object,
            crate::frontend::mem_word(mem, self.heap + 4),
            "writer heap changed heaps"
        );
        let method =
            crate::frontend::mem_word(mem, crate::frontend::mem_word(mem, self.heap) + 12);
        crate::render_part_c::Host::call(
            self.host,
            &mut Memory { base: 0, bytes: mem },
            method,
            &[object, pointer],
        );
    }
}

struct LiveNorm<'a, H> {
    host: &'a mut H,
    cached_handle: [u8; 8],
    default_context: u32,
}
impl<H: WriterLiveHost> crate::normalization::NormalizationHost for LiveNorm<'_, H> {
    fn calloc(&mut self, a: &mut [u8], heap: u32, count: u32, size: u32) -> Option<u32> {
        if heap == 0 || count == 0 || size == 0 {
            return None;
        }
        let object = crate::frontend::mem_word(a, heap + 4);
        let method = crate::frontend::mem_word(a, crate::frontend::mem_word(a, heap) + 4);
        let p = crate::render_part_c::Host::call(
            self.host,
            &mut Memory { base: 0, bytes: a },
            method,
            &[object, count, size],
        );
        if p == 0 {
            None
        } else {
            Some(p)
        }
    }
    fn alloc(&mut self, a: &mut [u8], heap: u32, n: u32) -> Option<u32> {
        if heap == 0 || n == 0 {
            return None;
        }
        let mut v = LiveHeap { heap, host: &mut *self.host };
        let p = crate::frontend::heap_alloc_m(a, Some(heap), n, &mut v);
        if p == 0 {
            None
        } else {
            Some(p)
        }
    }
    fn realloc(&mut self, a: &mut [u8], heap: u32, p: u32, n: u32) -> Option<u32> {
        if heap == 0 || n == 0 {
            return None;
        }
        let mut v = LiveHeap { heap, host: &mut *self.host };
        let q = crate::frontend::heap_realloc_m(a, Some(heap), p, n, &mut v);
        if q == 0 {
            None
        } else {
            Some(q)
        }
    }
    fn free(&mut self, a: &mut [u8], heap: u32, p: u32) {
        if heap == 0 || p == 0 {
            return;
        }
        let mut v = LiveHeap { heap, host: &mut *self.host };
        crate::frontend::heap_release_m(a, Some(heap), Some(p), &mut v);
    }
    fn resolve(&mut self, a: &mut [u8], lo: u32, hi: u32) -> (u32, u32) {
        let cache = &self.cached_handle;
        let cached = [
            u32::from_le_bytes(cache[..4].try_into().unwrap()),
            u32::from_le_bytes(cache[4..].try_into().unwrap()),
        ];
        let pointed = if [lo, hi] != cached && lo != 0 && hi == 0x1e61 {
            let at = lo as usize;
            Some(u16::from_le_bytes(a[at..at + 2].try_into().unwrap()))
        } else {
            None
        };
        let mut out = 0;
        let rc = crate::frontend::resolve_engine_context(
            lo,
            pointed,
            hi,
            cache,
            self.default_context,
            &mut out,
        );
        (rc, out)
    }
}

pub fn fetch_m(
    m: &mut Memory<'_>,
    seq: u32,
    tag: u32,
    n: u32,
    e: &Environment,
    h: &mut impl WriterLiveHost,
) -> (u32, u32) {
    assert_eq!(m.base, 0, "writer fetch requires absolute-index memory");
    let mut out = 0;
    let mut norm = LiveNorm {
        host: h,
        cached_handle: e.cached_handle,
        default_context: e.default_context,
    };
    let rc = normalization::fetch_entry(m.bytes, seq, tag, n, Some(&mut out), &mut norm);
    (rc, out)
}

fn free_m(m: &mut Memory<'_>, heap: u32, p: u32, h: &mut impl WriterLiveHost) {
    if heap == 0 || p == 0 {
        return;
    }
    let mut v = LiveHeap { heap, host: &mut *h };
    crate::frontend::heap_release_m(m.bytes, Some(heap), Some(p), &mut v);
}

fn live_call_method(
    m: &mut Memory<'_>,
    heap: u32,
    count: u32,
    size: u32,
    h: &mut impl WriterLiveHost,
) -> u32 {
    if heap == 0 || count == 0 || size == 0 {
        return 0;
    }
    let object = m.word(heap + 4);
    let method = m.word(m.word(heap) + 4);
    crate::render_part_c::Host::call(h, m, method, &[object, count, size])
}

pub fn fn_268a14_m(
    m: &mut Memory<'_>,
    seq: u32,
    mut a: u32,
    e: &Environment,
    h: &mut impl WriterLiveHost,
) -> u32 {
    let mut r = m.word(a + 4);
    let mut out = 1u16;
    let mut i = 1u16;
    while i < half(m, a + 8) {
        let mut next = i.wrapping_add(1);
        while next < half(m, a + 8) && half(m, r + next as u32 * 40 + 10) == 999 {
            next = next.wrapping_add(1);
        }
        let p = r + i as u32 * 40;
        let kind = half(m, p + 10);
        let at_start = m.word(p + 20) == m.word(r + 20);
        let redundant = at_start
            && (kind == 21 || kind == 7)
            && next < half(m, a + 8)
            && m.word(r + next as u32 * 40 + 20) == m.word(r + 20)
            && half(m, r + next as u32 * 40 + 10) == kind;
        if !redundant && !((kind == 999 || kind == 0) && !(at_start && kind == 7)) {
            record(m, r + out as u32 * 40, p);
            out = out.wrapping_add(1);
        }
        i = i.wrapping_add(1);
    }
    m.put_half(a + 8, out);
    let mut pending = false;
    let mut j = 1u16;
    while j < half(m, a + 8) {
        let mut p = r + j as u32 * 40;
        let kind = half(m, p + 10);
        if kind == 7 || kind == 36 {
            let start = m.word(p + 20);
            let end = m.word(r + 20).wrapping_add(m.word(r + 24));
            if start.wrapping_add(m.word(p + 24)) > end {
                if end < start {
                    m.put_word(p + 24, 0);
                    m.put_word(p + 20, end);
                } else {
                    m.put_word(p + 24, end.wrapping_sub(start));
                }
            }
            if kind == 7 && m.word(p + 28) == 43 {
                if pending {
                    pending = false;
                    j = j.wrapping_add(1);
                    continue;
                } else {
                    pending = true;
                    j = j.wrapping_add(1);
                    continue;
                }
            }
        }
        if pending && kind == 7 {
            if m.word(p + 28) == 43 || m.word(p + 28) == 15 {
                m.put_word(p + 28, 43);
            } else {
                let (rc, got) = fetch_m(m, seq, 0x3fd, half(m, a + 8).wrapping_add(1) as u32, e, h);
                if rc >= 0x80000000 {
                    return rc;
                }
                a = got;
                r = m.word(a + 4);
                p = r + j as u32 * 40;
                mov(m, p + 40, p, (half(m, a + 8) as u32 - j as u32) * 40);
                copy(m, p, p + 40, 40);
                m.put_word(p + 28, 43);
                m.put_half(a + 8, half(m, a + 8).wrapping_add(1));
            }
            pending = false;
        }
        j = j.wrapping_add(1);
    }
    0
}

pub fn fn_26783c_m(
    m: &mut Memory<'_>,
    context: u32,
    sequence: u32,
    e: &ExtendedEnvironment,
    h: &mut impl WriterLiveHost,
) -> u32 {
    let padding_length = len16(m, e.padding) as u16;
    let (rc, mut tb) = fetch_m(m, sequence, 0x3fc, 1, &e.markers, h);
    if rc >= 0x80000000 {
        return rc;
    }
    let mut text = m.word(tb + 4);
    let (rc, mut mb) = fetch_m(m, sequence, 0x3fd, 1, &e.markers, h);
    if rc >= 0x80000000 {
        return rc;
    }
    let mut markers = m.word(mb + 4);
    let (rc, mut sb) = fetch_m(m, sequence, 0x3f4, 1, &e.markers, h);
    if rc >= 0x80000000 {
        return rc;
    }
    let mut last_boundary = 0u16;
    let mut normal_index = u32::MAX;
    let mut index = 1u16;
    while index < half(m, mb + 8) {
        let mut current = markers + index as u32 * 40;
        if half(m, current + 10) == 21 {
            last_boundary = index;
            index = index.wrapping_add(1);
            continue;
        }
        if half(m, current + 10) != 7 || m.word(current + 28) != 43 || m.word(current + 32) == 0 {
            index = index.wrapping_add(1);
            continue;
        }
        let next_index = index as u32 + 1;
        let offset = m.word(current + 20).wrapping_sub(m.word(markers + 20)) as u16;
        let mut next = next_index as u16;
        while next < half(m, mb + 8) && half(m, markers + next as u32 * 40 + 10) != 7 {
            next = next.wrapping_add(1);
        }
        let end_offset = if next == half(m, mb + 8) {
            m.word(markers + 24) as u16
        } else {
            m.word(markers + next as u32 * 40 + 20)
                .wrapping_sub(m.word(markers + 20)) as u16
        };
        let start = (offset >> 1) as u32;
        let end = (end_offset >> 1) as u32;
        let replacement = m.word(current + 32);
        let mut original_words = 0u16;
        if m.byte(current + 36) != 0 {
            m.put_word(current + 32, 0);
            let span_length = end.wrapping_sub(start) as u16;
            if span_length != 0 {
                let (count, first, limit) = count_span(m, text + start * 2, span_length, e);
                original_words = count;
                if original_words != 0 {
                    let first_index = start.wrapping_add(first as u32) as u16;
                    let span_count = start
                        .wrapping_add(limit as u32)
                        .wrapping_sub(first_index as u32)
                        as u16;
                    let required = half(m, sb + 8).wrapping_add(span_count).wrapping_add(1);
                    if half(m, sb + 10) < required {
                        let (rc, b) = fetch_m(m, sequence, 0x3f4, required as u32, &e.markers, h);
                        if rc >= 0x80000000 {
                            return rc;
                        }
                        sb = b;
                    }
                    let stored = m.word(sb + 4) + half(m, sb + 8) as u32 * half(m, sb + 2) as u32;
                    copy16(m, stored, text + first_index as u32 * 2, span_count as u32);
                    m.put_half(stored + span_count as u32 * 2, 0);
                    m.put_half(current + 10, 34);
                    m.put_word(current + 28, half(m, sb + 8) as u32);
                    m.put_half(sb + 8, required);
                }
            }
        }
        let mut length = len16(m, replacement) as u16;
        let mut scan = 0u16;
        while scan < length {
            let c = half(m, replacement + scan as u32 * 2);
            if !separator(m, e, c) && c != 95 {
                break;
            }
            scan = scan.wrapping_add(1);
        }
        if scan != 0
            && scan < length
            && !separator(m, e, half(m, replacement + scan as u32 * 2))
            && half(m, replacement + scan as u32 * 2) != 95
        {
            let remaining = length - scan;
            mov(
                m,
                replacement,
                replacement + scan as u32 * 2,
                remaining as u32 * 2,
            );
            length = remaining;
            m.put_half(replacement + length as u32 * 2, 0);
        }
        if length != 0 {
            let last = length - 1;
            scan = last;
            while scan != 0 {
                let c = half(m, replacement + scan as u32 * 2);
                if !separator(m, e, c) && c != 95 {
                    break;
                }
                scan = scan.wrapping_sub(1);
            }
            if scan != 0
                && scan != last
                && !separator(m, e, half(m, replacement + scan as u32 * 2))
                && half(m, replacement + scan as u32 * 2) != 95
            {
                m.put_half(replacement + (scan as u32 + 1) * 2, 0);
                length = len16(m, replacement) as u16;
            }
        }
        scan = 0;
        let mut words = if length > 0 && half(m, replacement) != 0 {
            1u16
        } else {
            0
        };
        while scan < length && half(m, replacement + scan as u32 * 2) != 0 {
            if separator(m, e, half(m, replacement + scan as u32 * 2))
                || half(m, replacement + scan as u32 * 2) == 95
            {
                while scan < length
                    && (separator(m, e, half(m, replacement + scan as u32 * 2))
                        || half(m, replacement + scan as u32 * 2) == 95)
                {
                    scan = scan.wrapping_add(1);
                }
                if length > scan && half(m, replacement + scan as u32 * 2) != 0 {
                    words = words.wrapping_add(1);
                }
            }
            scan = scan.wrapping_add(1);
        }
        let mut padding = 0u16;
        let mut excess = 0u16;
        let mut padding_chars = 0u16;
        if m.byte(current + 36) != 0 && original_words != 0 && original_words != words {
            if original_words > words {
                padding = original_words - words;
                padding_chars = padding_length.wrapping_mul(padding);
            } else {
                excess = words - original_words;
            }
        }
        let new_length = length as u32 + padding_chars as u32;
        let delta: i16;
        if new_length > end - start {
            let added = new_length.wrapping_sub(end - start) as u16;
            delta = added as i16;
            let (rc, b) = fetch_m(
                m,
                sequence,
                0x3fc,
                half(m, tb + 8).wrapping_add(added) as u32,
                &e.markers,
                &mut *h,
            );
            if rc >= 0x80000000 {
                return rc;
            }
            tb = b;
            text = m.word(tb + 4);
            mov(
                m,
                text + (end + added as u32) * 2,
                text + end * 2,
                (half(m, tb + 8) as u32 - end) * 2,
            );
            m.put_half(tb + 8, half(m, tb + 8).wrapping_add(added));
        } else {
            let removed = (end - start - new_length) as u16;
            delta = (removed as i16).wrapping_neg();
            mov(
                m,
                text + (end - removed as u32) * 2,
                text + end * 2,
                (half(m, tb + 8) as u32 - end) * 2,
            );
            m.put_half(tb + 8, half(m, tb + 8).wrapping_sub(removed));
        }
        copy16(m, text + start * 2, replacement, length as u32);
        for scan in 0..padding {
            copy16(
                m,
                text + (start + length as u32 + scan as u32 * padding_length as u32) * 2,
                e.padding,
                padding_length as u32,
            );
        }
        free_m(m, m.word(context + 4), replacement, h);
        if excess != 0 {
            scan = start
                .wrapping_add(padding as u32 * padding_length as u32)
                .wrapping_add(length as u32)
                .wrapping_sub(1) as u16;
            while scan != 0
                && half(m, tb + 8) > scan
                && (separator(m, e, half(m, text + scan as u32 * 2))
                    || half(m, text + scan as u32 * 2) == 95)
            {
                scan = scan.wrapping_sub(1);
            }
            while scan != 0
                && half(m, tb + 8) > scan
                && !separator(m, e, half(m, text + scan as u32 * 2))
                && half(m, text + scan as u32 * 2) != 95
            {
                scan = scan.wrapping_sub(1);
            }
            while half(m, tb + 8) > scan && excess != 0 {
                if scan == 0 {
                    break;
                }
                while scan != 0
                    && half(m, tb + 8) > scan
                    && (separator(m, e, half(m, text + scan as u32 * 2))
                        || half(m, text + scan as u32 * 2) == 95)
                {
                    m.put_half(text + scan as u32 * 2, 45);
                    scan = scan.wrapping_sub(1);
                }
                if scan == 0 {
                    break;
                }
                excess = excess.wrapping_sub(1);
                while scan != 0
                    && half(m, tb + 8) > scan
                    && !separator(m, e, half(m, text + scan as u32 * 2))
                    && half(m, text + scan as u32 * 2) != 95
                {
                    scan = scan.wrapping_sub(1);
                }
            }
            for s in 1..half(m, mb + 8) {
                let p = markers + s as u32 * 40;
                let lower = m.word(markers + 20).wrapping_add((start * 2) as u16 as u32);
                if m.word(p + 20) > lower
                    && m.word(markers + 20).wrapping_add((end * 2) as u16 as u32) > m.word(p + 20)
                {
                    m.put_word(p + 20, lower);
                }
            }
        }
        let byte_start = (start * 2) as u16 as u32;
        let byte_end = (end * 2) as u16 as u32;
        m.put_word(markers + 24, half(m, tb + 8) as u32 * 2);
        let byte_delta = delta.wrapping_mul(2) as i32 as u32;
        for s in 1..half(m, mb + 8) {
            let p = markers + s as u32 * 40;
            let base = m.word(markers + 20);
            let position = m.word(p + 20);
            let lower = base.wrapping_add(byte_start);
            let upper = base.wrapping_add(byte_end);
            if position > lower && upper > position {
                m.put_word(p + 20, lower);
            } else if position >= upper {
                m.put_word(p + 20, position.wrapping_add(byte_delta));
            } else if half(m, p + 10) == 1
                && position.wrapping_add(m.word(p + 24)) > base.wrapping_add(m.word(markers + 24))
                && position
                    .wrapping_add(m.word(p + 24))
                    .wrapping_add(byte_delta)
                    != 0
            {
                m.put_word(p + 24, m.word(p + 24).wrapping_add(byte_delta));
            }
        }
        if m.byte(current + 36) == 0 || original_words == 0 {
            let mut found = 1u16;
            while found < half(m, mb + 8) {
                let p = markers + found as u32 * 40;
                if half(m, p + 10) == 1 && m.word(p + 20) == m.word(current + 20) {
                    break;
                }
                found = found.wrapping_add(1);
            }
            if found < half(m, mb + 8) && half(m, markers + found as u32 * 40 + 10) == 1 {
                fix_word_length(m, markers, markers + found as u32 * 40, text, e);
            }
            if half(m, mb + 8) as u32 != next_index {
                mov(
                    m,
                    current,
                    current + 40,
                    (half(m, mb + 8) as u32 - index as u32) * 40,
                );
            }
            index = index.wrapping_sub(1);
            m.put_half(mb + 8, half(m, mb + 8).wrapping_sub(1));
        } else {
            let position = m
                .word(markers + 20)
                .wrapping_add(byte_start)
                .wrapping_add(new_length * 2);
            let mut found = 1u16;
            while found < half(m, mb + 8) {
                let p = markers + found as u32 * 40;
                if m.word(p + 20) == position && half(m, p + 10) == 21 {
                    break;
                }
                if m.word(p + 20) > position {
                    break;
                }
                found = found.wrapping_add(1);
            }
            let ready = found < half(m, mb + 8)
                && m.word(markers + found as u32 * 40 + 20) == position
                && half(m, markers + found as u32 * 40 + 10) == 21;
            if !ready {
                let (rc, b) = fetch_m(
                    m,
                    sequence,
                    0x3fd,
                    half(m, mb + 8).wrapping_add(1) as u32,
                    &e.markers,
                    &mut *h,
                );
                if rc >= 0x80000000 {
                    return rc;
                }
                mb = b;
                markers = m.word(mb + 4);
                let boundary = markers + next as u32 * 40;
                mov(
                    m,
                    boundary + 40,
                    boundary,
                    (half(m, mb + 8) as u32 - next as u32) * 40,
                );
                copy(m, boundary, boundary + 40, 40);
                m.put_half(boundary + 10, 21);
                m.put_word(boundary + 20, position);
                m.put_word(boundary + 24, 0);
                m.put_word(boundary + 16, 0);
                m.put_word(boundary + 32, 0);
                if last_boundary != 0 {
                    m.put_word(
                        boundary + 28,
                        m.word(markers + last_boundary as u32 * 40 + 28),
                    );
                } else if normal_index != u32::MAX {
                    m.put_word(boundary + 28, normal_index);
                } else {
                    let mut offset = 0u32;
                    let mut candidate = m.word(sb + 4);
                    while candidate != 0 && offset < half(m, sb + 8) as u32 {
                        let n = len16(m, candidate);
                        if cmp16(m, candidate, e.markers.normal) == 0 {
                            normal_index = offset;
                            break;
                        }
                        offset = offset.wrapping_add(n + 1);
                        candidate = candidate.wrapping_add((n + 1) * 2);
                    }
                    if normal_index == u32::MAX {
                        let n = len16(m, e.markers.normal);
                        let required = (half(m, sb + 8) as u32 + n + 1) as u16;
                        if half(m, sb + 10) < required {
                            let (rc, b) = fetch_m(m, sequence, 0x3f4, required as u32, &e.markers, h);
                            if rc >= 0x80000000 {
                                return rc;
                            }
                            sb = b;
                        }
                        let stored =
                            m.word(sb + 4) + half(m, sb + 8) as u32 * half(m, sb + 2) as u32;
                        fe_text::strcpy16_176214(
                            m.bytes,
                            stored - m.base,
                            e.markers.normal - m.base,
                        );
                        m.put_half(stored + n * 2, 0);
                        normal_index = half(m, sb + 8) as u32;
                        m.put_half(sb + 8, required);
                    }
                    m.put_word(boundary + 28, normal_index);
                }
                m.put_half(mb + 8, half(m, mb + 8).wrapping_add(1));
            }
            let boundary = markers + next as u32 * 40;
            current = markers + index as u32 * 40;
            for s in 0..2 {
                let position = m.word(if s != 0 { boundary + 20 } else { current + 20 });
                let at = (position.wrapping_sub(m.word(markers + 20)) >> 1) as u16;
                let excluded;
                if s != 0 {
                    if at == 0
                        || at >= half(m, tb + 8)
                        || half(m, text + at as u32 * 2) == 32
                        || half(m, text + (at as u32 - 1) * 2) == 32
                    {
                        continue;
                    }
                    excluded = next;
                } else {
                    if at == 0 || at == 1 || half(m, text + (at as u32 - 1) * 2) == 32 {
                        continue;
                    }
                    excluded = half(m, mb + 8).wrapping_add(1);
                }
                let (rc, b) = fetch_m(
                    m,
                    sequence,
                    0x3fc,
                    half(m, tb + 8).wrapping_add(1) as u32,
                    &e.markers,
                    &mut *h,
                );
                if rc >= 0x80000000 {
                    return rc;
                }
                tb = b;
                text = m.word(tb + 4);
                mov(
                    m,
                    text + (at as u32 + 1) * 2,
                    text + at as u32 * 2,
                    (half(m, tb + 8) as u32 - at as u32) * 2,
                );
                m.put_half(text + at as u32 * 2, 32);
                m.put_half(tb + 8, half(m, tb + 8).wrapping_add(1));
                for n in 1..half(m, mb + 8) {
                    let p = markers + n as u32 * 40;
                    if s == 0 {
                        if position > m.word(p + 20) {
                            continue;
                        }
                        if half(m, p + 10) != 21 || m.word(p + 20) != position {
                            m.put_word(p + 20, m.word(p + 20).wrapping_add(2));
                        }
                    } else if n != excluded && position <= m.word(p + 20) {
                        m.put_word(p + 20, m.word(p + 20).wrapping_add(2));
                    }
                }
                m.put_word(markers + 24, m.word(markers + 24).wrapping_add(2));
            }
            if half(m, current + 10) == 34 && next < half(m, mb + 8) && half(m, boundary + 10) == 21
            {
                let mut found = 1u16;
                while found < half(m, mb + 8) {
                    let p = markers + found as u32 * 40;
                    if half(m, p + 10) == 1 && m.word(p + 20) == m.word(current + 20) {
                        break;
                    }
                    found = found.wrapping_add(1);
                }
                let p = markers + found as u32 * 40;
                if half(m, p + 10) == 1 && m.word(p + 20) == m.word(current + 20) {
                    fix_word_length(m, markers, p, text, e);
                    found = found.wrapping_add(1);
                    while found < half(m, mb + 8) {
                        let p = markers + found as u32 * 40;
                        if half(m, p + 10) == 1
                            && m.word(p + 20) >= m.word(current + 20)
                            && m.word(p + 20) < m.word(markers + next as u32 * 40 + 20)
                        {
                            if half(m, mb + 8) != found.wrapping_add(1) {
                                mov(m, p, p + 40, (half(m, mb + 8) as u32 - found as u32) * 40);
                            }
                            found = found.wrapping_sub(1);
                            next = next.wrapping_sub(1);
                            m.put_half(mb + 8, half(m, mb + 8).wrapping_sub(1));
                        }
                        found = found.wrapping_add(1);
                    }
                }
            }
            index = next.wrapping_sub(1);
        }
        index = index.wrapping_add(1);
    }
    for s in 0..half(m, mb + 8) {
        let p = markers + s as u32 * 40;
        let replacement = m.word(p + 32);
        if replacement != 0 {
            free_m(m, m.word(context + 4), replacement, h);
            m.put_word(p + 32, 0);
        }
    }
    0
}

pub fn fn_26b140_m(
    m: &mut Memory<'_>,
    context: u32,
    writer: u32,
    handle: [u32; 2],
    source: u32,
    info: u32,
    force: bool,
    e: &WriterEnvironment,
    h: &mut impl WriterLiveHost,
) -> u32 {
    assert_eq!(
        m.base, 0,
        "writer diagnostics require absolute-index memory"
    );
    if !force && half(m, info + 8) == 1 && half(m, source + 10) == 0x4000 {
        return 0;
    }
    let heap = m.word(context + 4);
    let markers = live_call_method(
        m,
        heap,
        1,
        half(m, info + 8) as u32 * 24 + 1,
        &mut *h,
    );
    if markers == 0 {
        return 0x8110200a;
    }
    let mut i = 0u16;
    while i < half(m, info + 8) {
        let dst = markers + i as u32 * 24;
        let src = source + i as u32 * 40;
        m.put_half(dst, half(m, src + 10));
        i = i.wrapping_add(1);
        for j in 0..5 {
            m.put_word(dst + 4 + j * 4, m.word(src + 12 + j * 4));
        }
    }
    debug_markers(
        m,
        m.byte(writer + 120) as u32,
        context,
        0,
        e.before_write,
        1,
        markers,
        half(m, info + 8) as u32,
        &e.diagnostics,
        &mut *h,
    );
    let result = h.write(
        m,
        m.word(m.word(writer + 8) + 52),
        handle,
        markers,
        half(m, info + 8) as u32 * 24,
    );
    let log = m.word(context + 16);
    if result < 0x80000000
        && normalization::flag_byte(m.bytes, if log == 0 { 0 } else { log - m.base }) != 0
    {

        normalization::fanout_call(
            m.bytes,
            log - m.base,
            0x22,
            e.log_record,
            e.content_type,
            markers,
            half(m, info + 8) as u32 * 24,
        );
    }
    free_m(m, heap, markers, h);
    result
}

pub fn fn_269ee4_m(
    m: &mut Memory<'_>,
    context: u32,
    writer: u32,
    sequence: u32,
    text_handle: [u32; 2],
    marker_handle: [u32; 2],
    e: &SequenceWriterEnvironment,
    h: &mut impl WriterLiveHost,
) -> u32 {
    assert_eq!(
        m.base, 0,
        "writer diagnostics require absolute-index memory"
    );
    if writer == 0 || sequence == 0 {
        return 0x81102007;
    }
    let (rc, position) = h.position(m, m.word(m.word(writer + 8) + 68), text_handle);
    if rc >= 0x80000000 {
        return rc;
    }
    let mode = m.byte(writer + 120) as u32;
    debug_markers(
        m,
        mode,
        context,
        sequence,
        e.labels[0],
        0,
        0,
        0,
        &e.writer.diagnostics,
        &mut *h,
    );
    crate::render_part_b::fn_263384(
        m,
        mode as i32,
        context,
        sequence,
        0x3fc,
        e.labels[0],
        &e.writer.diagnostics,
        &mut *h,
    );
    let rc = fn_26783c_m(m, context, sequence, &e.extended, h);
    if rc >= 0x80000000 {
        return rc;
    }
    debug_markers(
        m,
        mode,
        context,
        sequence,
        e.labels[1],
        0,
        0,
        0,
        &e.writer.diagnostics,
        &mut *h,
    );
    crate::render_part_b::fn_263384(
        m,
        mode as i32,
        context,
        sequence,
        0x3fc,
        e.labels[1],
        &e.writer.diagnostics,
        &mut *h,
    );
    let (rc, buffer) = fetch_m(m, sequence, 0x3fc, 1, &e.extended.markers, h);
    if rc >= 0x80000000 {
        return rc;
    }
    let wrote = half(m, buffer + 8) != 0;
    let data = if wrote { m.word(buffer + 4) } else { 0 };
    let size = if wrote {
        half(m, buffer + 8) as u32 * half(m, buffer + 2) as u32
    } else {
        0
    };
    if wrote {
        let rc = h.write(m, m.word(m.word(writer + 8) + 52), text_handle, data, size);
        if rc >= 0x80000000 {
            return rc;
        }
    }
    let log = m.word(context + 16);
    if normalization::flag_byte(m.bytes, if log == 0 { 0 } else { log - m.base }) != 0 {
        normalization::fanout_call(
            m.bytes,
            log - m.base,
            0x22,
            e.empty_string,
            e.text_content_type,
            data,
            size,
        );
    }
    if wrote {
        m.put_half(buffer + 8, 0);
    }
    let (rc, buffer) = fetch_m(m, sequence, 0x3f4, 1, &e.extended.markers, h);
    let text = if rc < 0x80000000 {
        m.word(buffer + 4)
    } else {
        0
    };
    let (rc, markers) = fetch_m(m, sequence, 0x3fd, 1, &e.extended.markers, h);
    if rc >= 0x80000000 {
        return rc;
    }
    let source = m.word(markers + 4);
    if half(m, markers + 8) == 0 || half(m, source + 10) != 0x4000 {
        return 0x81102000;
    }
    let delta = m.word(source + 20).wrapping_sub(position);
    let n = half(m, markers + 8);
    for i in 0..n {
        let p = source + i as u32 * 40;
        m.put_word(p + 20, m.word(p + 20).wrapping_sub(delta));
    }
    for i in 0..n {
        let p = source + i as u32 * 40;
        if adjust_kind(half(m, p + 10)) {
            let arg = m.word(p + 28);
            m.put_word(
                p + 28,
                if arg != u32::MAX {
                    text.wrapping_add(arg.wrapping_mul(2))
                } else {
                    0
                },
            );
        }
    }
    debug_markers(
        m,
        mode,
        context,
        sequence,
        e.labels[2],
        1,
        0,
        0,
        &e.writer.diagnostics,
        &mut *h,
    );
    let rc = fn_268a14_m(m, sequence, markers, &e.extended.markers, h);
    if rc >= 0x80000000 {
        return rc;
    }
    let source = m.word(markers + 4);
    debug_markers(
        m,
        mode,
        context,
        sequence,
        e.labels[3],
        1,
        0,
        0,
        &e.writer.diagnostics,
        &mut *h,
    );
    let rc = fn_26b140_m(
        m,
        context,
        writer,
        marker_handle,
        source,
        markers,
        wrote,
        &e.writer,
        &mut *h,
    );
    if rc < 0x80000000 {
        0
    } else {
        rc
    }
}

pub fn fn_26a3e4_m(
    m: &mut Memory<'_>,
    context: u32,
    state: u32,
    sequence: u32,
    first: [u32; 2],
    second: [u32; 2],
    e: &RenderWriterEnvironment,
    h: &mut impl WriterLiveHost,
) -> u32 {
    if m.byte(state + 119) != 0 {
        panic!("the live render reached the byte-text writer fn_268cbc (state+119 != 0)");
    } else {
        fn_269ee4_m(m, context, state, sequence, first, second, &e.sequence, &mut *h)
    }
}

pub fn fn_26a3e4(
    m: &mut Memory<'_>,
    context: u32,
    state: u32,
    sequence: u32,
    first: [u32; 2],
    second: [u32; 2],
    e: &RenderWriterEnvironment,
    h: &mut impl RenderWriterHost,
) -> u32 {
    if m.byte(state + 119) != 0 {
        fn_268cbc(m, context, state, sequence, first, second, e, h)
    } else {
        fn_269ee4(m, context, state, sequence, first, second, &e.sequence, h)
    }
}

fn debug_markers(
    m: &mut Memory<'_>,
    mode: u32,
    ctx: u32,
    seq: u32,
    label: u32,
    kind: i32,
    records: u32,
    count: u32,
    e: &crate::render_part_b::RenderEnvironment,
    h: &mut impl WriterHost,
) {
    crate::render_part_b::fn_2634c0(
        m,
        mode as i32,
        ctx,
        seq,
        0x3fd,
        label,
        kind,
        records,
        count,
        e,
        h,
    );
}

fn info_log(
    m: &mut Memory<'_>,
    logger: u32,
    category: u32,
    level: u32,
    format: u32,
    args: &[crate::render_part_b::RenderArgument<'_>],
    h: &mut impl crate::render_part_b::RenderLog,
) -> u32 {
    let mut view = [0u8; 128];
    let p = if logger == 0 { 0 } else { 64 };
    if logger != 0 {
        view[64 + 0x30] = m.byte(logger + 0x30);
        view[64 + 0x31] = m.byte(logger + 0x31);
    }
    let rc = normalization::log_fanout(&mut view, p, category, level, 0, format);
    if logger == 0
        || rc >= 0x80000000
        || level > view[64 + 0x30] as u32
        || (view[64 + 0x31] != 0 && level > 4)
        || format == 0
    {
        return rc;
    }
    let mut result = 0;
    for i in 0..6 {
        let slot = if i == 5 { 0x34 } else { 8 + i * 8 };
        let methods = m.word(logger + slot + 4);
        if methods != 0 && (i != 1 || level <= 1) {
            let method = m.word(methods + 24);
            if method != 0 {
                let r = h.log_target(
                    m,
                    method,
                    m.word(logger + slot),
                    category,
                    level,
                    0,
                    format,
                    args,
                );
                if r >= 0x80000000 {
                    result = r;
                }
            }
        }
    }
    result
}

pub struct CleanupEnvironment {
    pub engine: crate::render_part_c::Environment,
    pub user_names: [u32; 2],
    pub dictionary_name: u32,
    pub scratch: u32,
}

pub trait CleanupHost: crate::render_part_c::Host + crate::render_part_d::DictionaryHost {
    fn cleanup_dependency(&mut self, m: &mut Memory<'_>, entry: u32, args: &[u32]) -> u32;
}
struct CleanupMethods<'a,H>(&'a mut H);
impl<H: CleanupHost> crate::reentrant2::HooksM for CleanupMethods<'_,H> {
    fn call_m(&mut self,entry:u32,args:&[u32],m:&mut Memory<'_>)->u32 {
        crate::render_part_c::Host::call(self.0,m,entry,args)
    }
}
impl<H: CleanupHost> crate::render_part_c::ParserMethodsM for CleanupMethods<'_,H> {
    fn method_m(&mut self, entry:u32, args:&[u32], m:&mut Memory<'_>)->u32 {
        crate::render_part_c::Host::call(self.0,m,entry,args)
    }
}
fn cleanup_empty(e:&CleanupEnvironment)->[u32;2] {
    [u32::from_le_bytes(e.engine.cached_handle[..4].try_into().unwrap()),u32::from_le_bytes(e.engine.cached_handle[4..].try_into().unwrap())]
}
fn cleanup_context(m:&Memory<'_>,p:u32,e:&CleanupEnvironment)->(u32,u32) {
    let h=[m.word(p),m.word(p+4)];
    let cached=[u32::from_le_bytes(e.engine.cached_handle[..4].try_into().unwrap()),
        u32::from_le_bytes(e.engine.cached_handle[4..].try_into().unwrap())];
    let pointed=if h!=cached && h[0]!=0 && h[1]==0x1e61 {Some(half(m,h[0]))} else {None};
    let mut out=0;
    let rc=frontend::resolve_engine_context(h[0],pointed,h[1],&e.engine.cached_handle,e.engine.default_context,&mut out);
    (rc,out)
}

pub fn fn_26a444(m:&mut Memory<'_>,p:u32,e:&CleanupEnvironment,h:&mut impl CleanupHost)->u32 {
    let mut rc=crate::render_part_c::fn_2c3008(m,m.word(p+0x40),&e.engine,h);
    if rc>=0x80000000 {return rc;}
    if p==0 {return 0x81102007;}
    let (r,owner)=cleanup_context(m,p,e);
    if r>=0x80000000 {return 0x81102007;}
    if m.word(p+0x3c)!=0 {
        rc=if m.byte(p+0x77)!=0 {
            crate::render_part_c::fn_2ba2ac(m,m.word(p+0x3c),&e.engine,h)
        } else {
            crate::render_part_c::fn_2c3e2c(m,m.word(p+0x3c),&e.engine,h)
        };
        if rc>=0x80000000 {return rc;}
    }
    let mut cleanup=0;
    if m.word(p+0x70)!=0 {
        let (r,secondary)=cleanup_context(m,p,e);
        if r>=0x80000000 {cleanup=0x81102007;}
        else {
            crate::render_part_c::Host::call(h,m,0x196428,&[m.word(secondary+4),m.word(p+0x70)]);
            m.put_word(p+0x70,0);cleanup=r;
        }
    }
    if m.word(p+0x64)!=0 {
        let method=m.word(m.word(p+0x64)+20);
        cleanup=crate::render_part_c::Host::call(h,m,method,&[m.word(p+0x68),m.word(p+0x6c)]);
        if m.word(p+0x64)!=0 {
            cleanup=frontend::release_broker_interface(opt(m,m.word(owner+12)),opt(m,m.word(p+0x64)));
            m.put_word(p+0x64,0);
            let empty=cleanup_empty(e);m.put_word(p+0x68,empty[0]);m.put_word(p+0x6c,empty[1]);
        }
    }
    if m.byte(p+0x77)!=0 {
        for (i,off) in [0x58,0x4c].into_iter().enumerate() {
            if m.word(p+off)!=0 {
                crate::render_part_c2::registry_release(m,m.word(owner+24),e.user_names[i],false,h);
                for j in 0..3 {m.put_word(p+off+j*4,0);}
            }
        }
    }
    rc=cleanup;
    if rc>=0x80000000 {return rc;}
    if m.word(p+0x38)!=0 {
        rc=crate::render_part_c::fn_2bf6c4_m(m,m.word(p+0x38),&e.engine,e.dictionary_name,e.scratch,&mut CleanupMethods(h));
    }
    if rc>=0x80000000 {return rc;}
    if m.word(p+0x34)!=0 {return h.cleanup_dependency(m,0x2c4778,&[m.word(p+0x34)]);}
    rc
}
fn first_error(error:&mut u32,rc:u32) {
    if rc>=0x80000000 && *error<0x80000000 {*error=rc;}
}

pub fn fn_26c858(m:&mut Memory<'_>,p:u32,e:&CleanupEnvironment,conversion:&crate::render_part_c2::Environment<'_>,h:&mut (impl CleanupHost + crate::render_part_c2::ResourceHost))->u32 {
    let mut error=0;
    let mut i=0u16;
    if m.word(p+12)!=0 {
        while i<half(m,m.word(p+12)+8) {
            let s=m.word(m.word(m.word(p+12)+4)+i as u32*4);
            let method=m.word(m.word(s+8)+0x38);
            first_error(&mut error,crate::render_part_c::Host::call(h,m,method,&[m.word(s+12),m.word(s+16)]));
            i=i.wrapping_add(1);
        }
    }
    if m.word(p+0x3c)!=0 {
        h.cleanup_dependency(m,0x179f3c,&[m.word(m.word(p)+12)]);
        m.put_word(p+0x3c,0);
    }
    if frontend::match_halves(m.word(p+0x2c),m.word(p+0x30),cleanup_empty(e)[0],cleanup_empty(e)[1])==0 {
        let method=m.word(m.word(p+0x14)+0x24);
        let rc=crate::render_part_c::Host::call(h,m,method,&[m.word(p+0x2c),m.word(p+0x30)]);
        let empty=cleanup_empty(e);m.put_word(p+0x2c,empty[0]);m.put_word(p+0x30,empty[1]);
        first_error(&mut error,rc);
    }
    for (object,handle) in [(0x14,0x1c),(0x18,0x24)] {
        if m.word(p+handle)!=0 {
            let method=m.word(m.word(p+object)+20);
            first_error(&mut error,crate::render_part_c::Host::call(h,m,method,&[m.word(p+handle),m.word(p+handle+4)]));
        }
    }
    if m.word(p+12)!=0 {
        let rc=crate::render_part_c2::fn_2c66c8(m,m.word(p+12),conversion,h);
        if rc>=0x80000000 {error=rc;}
        m.put_word(p+12,0);
    }
    for (off,name) in [(0x18,e.user_names[0]),(0x14,e.user_names[1])] {
        if m.word(p+off)!=0 {
            let rc=crate::render_part_c2::registry_release(m,m.word(m.word(p)+24),name,false,h);
            if rc>=0x80000000 {error=rc;}
            m.put_word(p+off,0);
        }
    }
    if m.word(p+16)!=0 {
        let rc=crate::render_part_c2::fn_2c7f24(m,[m.word(p+4),m.word(p+8)],m.word(p+16),e.scratch,conversion,h);
        if rc>=0x80000000 {error=rc;}
        crate::reentrant2::destroy_array_m(m,m.word(p+16),&mut CleanupMethods(h));
        m.put_word(p+16,0);
    }
    let owner=m.word(p);
    if m.word(owner+24)!=0 {
        let rc=crate::leftovers_1::fn_17af38(m,m.word(owner+24),h);
        m.put_word(owner+24,0);first_error(&mut error,rc);
    }
    if m.word(owner+20)!=0 {
        let rc=h.cleanup_dependency(m,0x174e00,&[m.word(owner+20)]);
        m.put_word(owner+20,0);first_error(&mut error,rc);
    }
    let heap=m.word(owner+4);
    if m.word(p+4)!=0 {
        first_error(&mut error,crate::leftovers_1::fn_196238(m,[m.word(p+4),m.word(p+8)],&e.engine,h));
    }
    if m.word(p+0x2d0)!=0 {
        crate::render_part_c::Host::call(h,m,0x196428,&[heap,m.word(p+0x2d0)]);
    }
    crate::render_part_c::Host::call(h,m,0x196428,&[heap,p]);
    error
}
