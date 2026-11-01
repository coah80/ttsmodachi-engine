
use crate::{boot::Memory, speech_part_4};

pub fn fn_129778(
    m: &mut Memory<'_>,
    host: &mut impl speech_part_4::Host,
    size: u32,
    fill: u32,
) -> u32 {
    speech_part_4::fn_129778(m, host, size, fill)
}
pub fn fn_196964(m: &mut Memory<'_>, host: &mut impl speech_part_4::Host, pointer: u32, heap: u32) {
    speech_part_4::fn_196964(m, host, pointer, heap)
}

pub trait Host {
    fn call(&mut self, m: &mut Memory<'_>, method: u32, args: &[u32]) -> u32;

    fn log(
        &mut self,
        m: &mut Memory<'_>,
        logger: u32,
        category: u32,
        level: u32,
        format: &[u8],
        values: &[u32],
    );
    fn log_guest(
        &mut self,
        m: &mut Memory<'_>,
        logger: u32,
        category: u32,
        level: u32,
        format: u32,
        values: &[u32],
    );
    fn error(&mut self, m: &mut Memory<'_>, logger: u32, category: u32, code: u32);
}
fn half(m: &Memory<'_>, p: u32) -> u16 {
    let i = m.offset(p);
    u16::from_le_bytes(m.bytes[i..i + 2].try_into().unwrap())
}
fn put_half(m: &mut Memory<'_>, p: u32, v: u16) {
    let i = m.offset(p);
    m.bytes[i..i + 2].copy_from_slice(&v.to_le_bytes());
}
fn byte(m: &mut Memory<'_>, p: u32, v: u8) {
    let i = m.offset(p);
    m.bytes[i] = v;
}
fn failed(r: u32) -> bool {
    r >= 0x80000000
}

pub fn fn_24c9fc(m: &mut Memory<'_>, h: &mut impl Host, p: u32, scratch: u32) -> u32 {
    let mut saved = [0; 8];
    saved.copy_from_slice(&m.bytes[m.offset(scratch)..m.offset(scratch) + 8]);
    put_half(m, scratch, 0);
    put_half(m, scratch + 2, 0);
    put_half(m, scratch + 4, 0);
    let logger = m.word(m.word(p) + 16);
    let category = m.word(p + 0x48);
    h.log(m, logger, category, 4, b"tokentn : ReadSentence Begin", &[]);
    m.put_word(p + 0x5c, 0);
    let rc = (|| {
        let mut r = h.call(
            m,
            m.word(m.word(p + 12) + 0x34),
            &[m.word(p + 16), m.word(p + 20), 1, 0, scratch],
        );
        if failed(r) {
            h.error(m, m.word(m.word(p) + 16), m.word(p + 0x48), 0x5de2);
            return crate::normalization::error_code(0);
        }
        let a = half(m, scratch) as u32;
        if a != 0
            && !failed(h.call(
                m,
                m.word(m.word(p + 12) + 0x5c),
                &[m.word(p + 16), m.word(p + 20), a, 0, scratch + 2],
            ))
            && half(m, scratch + 2) == 1
        {
            r = h.call(
                m,
                m.word(m.word(p + 12) + 0x58),
                &[m.word(p + 16), m.word(p + 20), a, 0, p + 0x5c, scratch + 4],
            );
            if failed(r) {
                h.error(m, m.word(m.word(p) + 16), m.word(p + 0x48), 0x5de2);
                return crate::normalization::error_code(0);
            }
        }
        h.log(
            m,
            m.word(m.word(p) + 16),
            m.word(p + 0x48),
            4,
            b"tokentn : ReadSentence End (%x)",
            &[r],
        );
        r
    })();
    let at = m.offset(scratch);
    m.bytes[at..at + 8].copy_from_slice(&saved);
    rc
}

pub fn fn_1d8f28(
    m: &mut Memory<'_>,
    h: &mut impl Host,
    c: u32,
    db: [u32; 2],
    x: u32,
    text: u32,
    buf: u32,
    name: u32,
    category: u32,
    empty: u32,
    scratch: u32,
) -> u32 {
    let at = m.offset(scratch);
    let mut saved = [0; 40];
    saved.copy_from_slice(&m.bytes[at..at + 40]);
    let r = (|| {
        let [a, b] = db;

        let s = scratch;
        for off in [10, 12] {
            put_half(m, s + off, 0);
        }
        for off in [16, 20, 24] {
            m.put_word(s + off, 0);
        }
        h.log(m, m.word(m.word(c) + 16), category, 5, b"%s", &[name]);
        let mut r = h.call(m, m.word(m.word(c + 4) + 0x34), &[a, b, 3, x, s + 8]);
        if failed(r) {
            return r;
        }
        if half(m, s + 8) != 0 {
            loop {
                let node = half(m, s + 8) as u32;
                for (key, out) in [(1, s), (2, s + 2), (0, s + 4)] {
                    r = h.call(
                        m,
                        m.word(m.word(c + 4) + 0x54),
                        &[a, b, node, key, 1, out, s + 6],
                    );
                    if failed(r) {
                        return r;
                    }
                }
                let ty = m.byte(s + 4);
                let mut have = false;
                if ty == 5 || ty == 6 {
                    r = h.call(
                        m,
                        m.word(m.word(c + 4) + 0x58),
                        &[a, b, node, 4, s + 16, s + 6],
                    );
                    if failed(r) {
                        return r;
                    }
                    if ty == 6 {
                        r = h.call(
                            m,
                            m.word(m.word(c + 4) + 0x58),
                            &[a, b, node, 3, s + 20, s + 6],
                        );
                        if failed(r) {
                            return r;
                        }
                        if !failed(h.call(
                            m,
                            m.word(m.word(c + 4) + 0x5c),
                            &[a, b, node, 6, s + 10],
                        )) && half(m, s + 10) == 1
                        {
                            r = h.call(
                                m,
                                m.word(m.word(c + 4) + 0x58),
                                &[a, b, node, 6, s + 24, s + 12],
                            );
                            if failed(r) {
                                return r;
                            }
                            have = true;
                        }
                    }
                }
                let from = half(m, s) as u32;
                let to = half(m, s + 2) as u32;
                let n = to.wrapping_sub(from);
                crate::text_input::strncpy_at(
                    m.bytes,
                    buf - m.base,
                    text.wrapping_add(from) - m.base,
                    n,
                );
                byte(m, buf.wrapping_add(n), 0);
                let logger = m.word(m.word(c) + 16);
                if ty <= 4 {
                    m.put_word(s + 28, 0);
                    h.call(m, m.word(m.word(c + 4) + 0x80), &[a, b, ty as u32, s + 28]);
                    h.log(
                        m,
                        m.word(m.word(c) + 16),
                        category,
                        5,
                        b"[dbId=%d] %3d,%3d %-15s \"%s\"",
                        &[node, from, to, m.word(s + 28), buf],
                    );
                } else if ty == 6 {
                    h.log(
                        m,
                        logger,
                        category,
                        5,
                        b"[dbId=%d] %3d,%3d %-15s \"%s\" \"%s\"",
                        &[
                            node,
                            from,
                            to,
                            m.word(s + 16),
                            m.word(s + 20),
                            if have { m.word(s + 24) } else { empty },
                        ],
                    );
                } else if ty == 5 {
                    h.log(
                        m,
                        logger,
                        category,
                        5,
                        b"[dbId=%d] %3d,%3d ESC(%-9s) \"%s\"",
                        &[node, from, to, m.word(s + 16), buf],
                    );
                }
                if h.call(m, m.word(m.word(c + 4) + 0x3c), &[a, b, node, s + 8]) != 0
                    || half(m, s + 8) == 0
                {
                    break;
                }
            }
        }
        r
    })();
    m.bytes[at..at + 40].copy_from_slice(&saved);
    r
}

pub struct TokenConstants {
    pub category: u32,
    pub langcode: u32,
    pub clm: u32,
    pub user: u32,
    pub end_format: u32,
    pub lid: u32,
    pub noclmset: u32,
    pub no: u32,
    pub yes: u32,
    pub marker: u32,
}
fn len8(m: &Memory<'_>, p: u32) -> u32 {
    crate::normalization::strlen8(m.bytes, p - m.base)
}
fn copy(m: &mut Memory<'_>, d: u32, s: u32) {
    crate::normalization::strcpy_at(m.bytes, d - m.base, s - m.base);
}
fn compare(m: &Memory<'_>, a: u32, b: u32) -> i32 {
    crate::normalization::strcmp_at(m.bytes, a - m.base, b - m.base)
}
fn find(m: &Memory<'_>, a: u32, b: u32) -> u32 {
    let p = crate::frontend::find_substr(m.bytes, a - m.base, b - m.base);
    if p == 0 {
        0
    } else {
        p + m.base
    }
}
fn ling(m: &mut Memory<'_>, h: &mut impl Host, p: u32, slot: u32, a: &[u32]) -> u32 {
    h.call(m, m.word(m.word(p + 0x18) + slot), a)
}
fn set(
    m: &mut Memory<'_>,
    h: &mut impl Host,
    p: u32,
    db: [u32; 2],
    node: u32,
    key: u32,
    n: u32,
    data: u32,
    written: u32,
) -> u32 {
    ling(m, h, p, 0x50, &[db[0], db[1], node, key, n, data, written])
}
fn get_property(m: &mut Memory<'_>, h: &mut impl Host, props: u32, key: u32, out: u32) -> u32 {
    if let Some(rc) = m.boot_lookup_pure(0x175a1c, &[props, key, out], 0x196980) {
        return rc;
    }
    h.call(m, 0x175a1c, &[props, key, out])
}
struct Heap<'a, H> {
    host: &'a mut H,
    base: u32,
    alloc: u32,
    free: u32,
}
impl<H: Host> crate::frontend::HeapVtableM for Heap<'_, H> {
    fn alloc_at_m(&mut self, obj: u32, n: u32, bytes: &mut [u8]) -> u32 {
        self.host.call(
            &mut Memory {
                base: self.base,
                bytes,
            },
            self.alloc,
            &[obj, n],
        )
    }
    fn realloc_at_m(&mut self, _: u32, _: u32, _: u32, _: &mut [u8]) -> u32 {
        unreachable!()
    }
    fn release_at_m(&mut self, obj: u32, p: u32, bytes: &mut [u8]) {
        self.host.call(
            &mut Memory {
                base: self.base,
                bytes,
            },
            self.free,
            &[obj, p],
        );
    }
}
fn alloc(m: &mut Memory<'_>, h: &mut impl Host, heap: u32, n: u32) -> u32 {
    if heap == 0 {
        return 0;
    }
    let method = m.word(m.word(heap));
    let mut v = Heap {
        host: h,
        base: m.base,
        alloc: method,
        free: 0,
    };
    crate::frontend::heap_alloc_m(m.bytes, Some(heap - m.base), n, &mut v)
}
fn release(m: &mut Memory<'_>, h: &mut impl Host, heap: u32, p: u32) {
    if heap == 0 {
        return;
    }
    let method = m.word(m.word(heap) + 12);
    let mut v = Heap {
        host: h,
        base: m.base,
        alloc: 0,
        free: method,
    };
    crate::frontend::heap_release_m(
        m.bytes,
        Some(heap - m.base),
        if p == 0 { None } else { Some(p) },
        &mut v,
    );
}

pub fn fn_1d5234(
    m: &mut Memory<'_>,
    h: &mut impl Host,
    p: u32,
    db: [u32; 2],
    cursor: u32,
    start: u32,
    end: u16,
    text: u32,
    phonemes: u32,
    language: u32,
    fallback: u32,
    _unused: u32,
    user: u32,
    k: &TokenConstants,
    scratch: u32,
) -> u32 {
    let at = m.offset(scratch);
    let mut saved = [0; 64];
    saved.copy_from_slice(&m.bytes[at..at + 64]);
    let s = scratch;

    put_half(m, s, 0);
    let original = half(m, cursor) as u32;
    let mut rc = 0;
    let r = (|| {
        let mut lang = s + 20;
        if crate::fe_text::strlen16_176624(m.bytes, language - m.base) == 3 {
            m.put_word(s + 12, 0);
            rc = get_property(m, h, m.word(m.word(p) + 20), k.langcode, s + 12);
            if failed(rc) {
                return rc;
            }
            for i in 0..3 {
                byte(
                    m,
                    lang + i,
                    crate::normalization::tolower_240ba0(half(m, language + i * 2) as u8 as u32)
                        as u8,
                );
            }
            byte(m, lang + 3, 0);
            if crate::fe_text::strcmp_ci_1769d0(m.bytes, lang - m.base, m.word(s + 12) - m.base)
                == 0
            {
                copy(m, lang, k.lid);
            }
        } else {
            copy(m, lang, k.lid);
        }
        if half(m, cursor) == 0 {
            return rc;
        }
        let mut created = false;
        loop {
            let node = half(m, cursor) as u32;
            rc = ling(m, h, p, 0x54, &[db[0], db[1], node, 1, 1, s, s + 2]);
            if failed(rc) {
                h.error(m, m.word(m.word(p) + 16), k.category, 0x7533);
                return rc;
            }
            let pos = half(m, s) as u32;
            if pos >= start {
                rc = ling(m, h, p, 0x54, &[db[0], db[1], node, 0, 1, s + 6, s + 2]);
                if failed(rc) {
                    h.error(m, m.word(m.word(p) + 16), k.category, 0x7533);
                    return rc;
                }
                if m.byte(s + 6) != 3 && pos != end as u32 {
                    rc = ling(m, h, p, 0x28, &[db[0], db[1], node, cursor]);
                    if failed(rc) {
                        return rc;
                    }
                    byte(m, s + 6, 5);
                    let node = half(m, cursor) as u32;
                    rc = set(m, h, p, db, node, 0, 1, s + 6, s + 4);
                    if failed(rc) {
                        h.error(m, m.word(m.word(p) + 16), k.category, 0x7532);
                        return rc;
                    }
                    h.log(
                        m,
                        m.word(m.word(p) + 16),
                        k.category,
                        5,
                        b"Creating token %u %u in lingdb",
                        &[pos, end as u32],
                    );

                    put_half(m, s + 10, end);
                    for (key, n, data) in [
                        (1, 1, s),
                        (2, 1, s + 10),
                        (4, (len8(m, text) + 1) & 65535, text),
                    ] {
                        rc = set(m, h, p, db, node, key, n, data, s + 4);
                        if failed(rc) {
                            h.error(m, m.word(m.word(p) + 16), k.category, 0x7532);
                            return rc;
                        }
                    }
                    if user != 0 {
                        rc = set(m, h, p, db, node, 6, 4, k.user, s + 4);
                        if failed(rc) {
                            return rc;
                        }
                    }
                    if phonemes != 0 {
                        let mut mark = find(m, phonemes, k.marker);
                        while mark != 0 {
                            byte(m, mark, b'#');
                            let n = (len8(m, phonemes) as u16 as u32)
                                .wrapping_sub(mark.wrapping_sub(phonemes) as u16 as u32)
                                .wrapping_sub(2);
                            crate::normalization::memmove_at(
                                m.bytes,
                                mark + 1 - m.base,
                                mark + 3 - m.base,
                                n,
                            );
                            mark = find(m, phonemes, k.marker);
                        }
                        let mut converted = phonemes;
                        if compare(m, lang, k.lid) != 0 {
                            let key = s + 28;
                            copy(m, key, k.clm);
                            crate::fe_text::strcat_195c90(m.bytes, key - m.base, lang - m.base);
                            m.put_word(s + 12, 0);
                            if failed(get_property(m, h, m.word(m.word(p) + 20), key, s + 12)) {
                                m.put_word(s + 12, 0);
                            }
                            if m.word(s + 12) == 0 {
                                copy(m, key, k.noclmset);
                                if !failed(get_property(m, h, m.word(m.word(p) + 20), key, s + 12))
                                    && find(m, m.word(s + 12), lang) != 0
                                {
                                    copy(m, s + 40, k.no);
                                    m.put_word(s + 12, s + 40);
                                } else {
                                    m.put_word(s + 12, 0);
                                }
                            }
                            for i in 0..3 {
                                let b = m.byte(lang + i);
                                byte(m, s + 24 + i, if b > 0x60 { b - 0x20 } else { b });
                            }
                            byte(m, s + 27, 0);
                            converted = alloc(
                                m,
                                h,
                                m.word(m.word(p) + 4),
                                (len8(m, phonemes) + 1).wrapping_mul(4),
                            );
                            if converted == 0 {
                                h.error(m, m.word(m.word(p) + 16), k.category, 0x84d0);
                                return 0x8990200a;
                            }
                            let mut converted_ready = false;
                            if m.word(p + 0x38) != 0 {
                                let setting = m.word(s + 12);
                                let capacity = if setting != 0 && compare(m, setting, k.yes) != 0 {
                                    0
                                } else {
                                    (len8(m, phonemes) + 1).wrapping_mul(4)
                                };
                                rc = h.call(
                                    m,
                                    m.word(m.word(p + 0x48) + 0x24),
                                    &[
                                        m.word(p + 0x40),
                                        m.word(p + 0x44),
                                        s + 24,
                                        phonemes,
                                        len8(m, phonemes) + 1,
                                        converted,
                                        capacity,
                                    ],
                                );
                                converted_ready = !failed(rc);
                            }
                            if converted_ready {
                                let setting = m.word(s + 12);
                                if setting != 0 && compare(m, setting, k.yes) != 0 {
                                    release(m, h, m.word(m.word(p) + 4), converted);
                                    converted = phonemes;
                                    rc = set(
                                        m,
                                        h,
                                        p,
                                        db,
                                        node,
                                        6,
                                        (len8(m, lang) + 1) & 65535,
                                        lang,
                                        s + 4,
                                    );
                                    if failed(rc) {
                                        return rc;
                                    }
                                }
                            } else {
                                put_half(m, s + 8, 0);
                                m.put_word(s + 16, 0);
                                rc = ling(m, h, p, 0x34, &[db[0], db[1], 1, 0, s + 8]);
                                if failed(rc) {
                                    h.error(m, m.word(m.word(p) + 16), k.category, 0x7532);
                                    return rc;
                                }
                                rc = ling(
                                    m,
                                    h,
                                    p,
                                    0x58,
                                    &[db[0], db[1], half(m, s + 8) as u32, 0, s + 16, s + 2],
                                );
                                if failed(rc) {
                                    h.error(m, m.word(m.word(p) + 16), k.category, 0x7532);
                                    return rc;
                                }
                                let length = half(m, s + 2);
                                let raw = m.word(s + 16);
                                if length != 0 {
                                    let count = length as u8;
                                    let n = if count == 0 { 256 } else { count as u32 };
                                    for i in 0..n {
                                        if m.byte(raw + i) == 0x15 {
                                            byte(m, raw + i, b' ');
                                        }
                                    }
                                }
                                rc = set(
                                    m,
                                    h,
                                    p,
                                    db,
                                    half(m, s + 8) as u32,
                                    0,
                                    length as u32,
                                    raw,
                                    s + 4,
                                );
                                if failed(rc) {
                                    h.error(m, m.word(m.word(p) + 16), k.category, 0x7532);
                                    return rc;
                                }
                                rc = set(
                                    m,
                                    h,
                                    p,
                                    db,
                                    node,
                                    4,
                                    (len8(m, fallback) + 1) & 65535,
                                    fallback,
                                    s + 4,
                                );
                                release(m, h, m.word(m.word(p) + 4), converted);
                                converted = 0;
                                if failed(rc) {
                                    h.error(m, m.word(m.word(p) + 16), k.category, 0x7532);
                                    return rc;
                                }
                            }
                        }
                        if converted != 0 {
                            rc = set(
                                m,
                                h,
                                p,
                                db,
                                node,
                                3,
                                (len8(m, converted) + 1) & 65535,
                                converted,
                                s + 4,
                            );
                        }
                        if converted != phonemes && converted != 0 {
                            release(m, h, m.word(m.word(p) + 4), converted);
                        }
                        if failed(rc) {
                            h.error(m, m.word(m.word(p) + 16), k.category, 0x7532);
                            return rc;
                        }
                    }
                    created = true;
                }
            }
            if half(m, s) == end {
                return rc;
            }
            rc = ling(
                m,
                h,
                p,
                0x3c,
                &[db[0], db[1], half(m, cursor) as u32, cursor],
            );
            if failed(rc) || created {
                return rc;
            }
            if half(m, cursor) == 0 {
                break;
            }
        }
        if half(m, s) as u32 != start {
            return rc;
        }
        rc = ling(m, h, p, 0x28, &[db[0], db[1], original, cursor]);
        if failed(rc) {
            return rc;
        }
        byte(m, s + 6, 5);
        let node = half(m, cursor) as u32;
        rc = set(m, h, p, db, node, 0, 1, s + 6, s + 4);
        if failed(rc) {
            h.error(m, m.word(m.word(p) + 16), k.category, 0x7532);
            return rc;
        }
        h.log_guest(
            m,
            m.word(m.word(p) + 16),
            k.category,
            5,
            k.end_format,
            &[half(m, s) as u32, end as u32],
        );
        put_half(m, s + 10, end);
        for (key, n, data) in [
            (1, 1, s),
            (2, 1, s + 10),
            (4, (len8(m, text) + 1) & 65535, text),
        ] {
            rc = set(m, h, p, db, node, key, n, data, s + 4);
            if failed(rc) {
                h.error(m, m.word(m.word(p) + 16), k.category, 0x7532);
                return rc;
            }
        }
        rc
    })();
    m.bytes[at..at + 64].copy_from_slice(&saved);
    r
}

pub fn fn_19658c(
    m: &mut Memory<'_>,
    h: &mut impl Host,
    t: u32,
    out: u32,
    heap_vt: &mut impl crate::frontend::HeapVtableM,
    ops: &mut impl crate::frontend::TreeOpsM,
) -> u32 {
    if t == 0 || out == 0 {
        return 0x89002007;
    }
    assert_eq!(m.base, 0, "tree ports need zero-based caller memory");
    let heap = m.word(t + 20);
    let o = if heap == 0 {
        0
    } else {
        let method = m.word(m.word(heap) + 4);
        let obj = m.word(heap + 4);
        struct Method<F>(F);
        impl<F: FnMut(u32, u32, u32) -> u32> crate::frontend::MethodTable for Method<F> {
            fn call_at(&mut self, o: u32, a: u32, b: u32) -> u32 {
                (self.0)(o, a, b)
            }
        }
        let mut header = [0; 8];
        header[4..8].copy_from_slice(&obj.to_le_bytes());
        crate::frontend::call_method(
            &header,
            Some(0),
            1,
            0x44,
            &mut Method(|o, a, b| h.call(m, method, &[o, a, b])),
        )
    };
    m.put_word(out, o);
    if o == 0 {
        return 0x8900200a;
    }
    for i in 0..5 {
        let v = m.word(t + i * 4);
        m.put_word(o + i * 4, v);
    }
    m.put_word(o + 20, m.word(t + 20));
    crate::frontend::init_link(m.bytes, o + 24, u32::MAX, o, m.word(t + 20));
    if m.byte(t + 24 + 40) != 0 {
        crate::frontend::set_link_flag(m.bytes, o + 24);
    }
    let mut n = crate::frontend::last_link(m.bytes, t + 24);
    let mut rc = 0;
    while n != 0 {
        let a = m.word(n + 16);
        let b = m.word(n + 20);
        rc = crate::frontend::tree_alloc_insert_m(m.bytes, o + 24, a, b, heap_vt, ops);
        n = crate::frontend::successor_node(m.bytes, t + 24, n);
        if failed(rc) {
            break;
        }
    }
    rc
}

pub trait FormatHost: speech_part_4::Host + crate::render_part_d2::LocaleHost {}
impl<T: speech_part_4::Host + crate::render_part_d2::LocaleHost> FormatHost for T {}
fn output(m: &mut Memory<'_>, h: &mut impl speech_part_4::Host, s: u32, v: u32) {
    let f = m.word(s + 4);
    let obj = m.word(s + 8);
    if f & !1 == 0x100560 {
        speech_part_4::fn_100560(m, v, obj);
    } else {
        h.call(m, f, &[v, obj]);
    }
}
fn numeric(
    m: &mut Memory<'_>,
    h: &mut impl speech_part_4::Host,
    s: u32,
    c: u32,
    va: u32,
    radix: u32,
    wide: bool,
    pointer: bool,
    tmp: u32,
) -> u32 {
    let flags = m.word(s);
    let mut v = if wide {
        let a = va.wrapping_add(7) & !7;
        m.word(a) as u64 | ((m.word(a + 4) as u64) << 32)
    } else if pointer {
        m.word(va) as u64
    } else {
        speech_part_4::fn_2ffdc2(m, m.word(va), s) as u64
    };
    let signed = radix == 10 && c != 117 && c != 245;
    if signed && !wide {
        v = if flags & 1024 != 0 {
            (v as i8 as i64) as u64
        } else if flags & 256 != 0 {
            (v as i16 as i64) as u64
        } else {
            (v as i32 as i64) as u64
        };
    }
    let negative = signed && (v as i64) < 0;
    if negative {
        v = v.wrapping_neg();
    }
    let mut n = 0;
    while v != 0 {
        let (quotient, digit) = if radix == 10 && !wide {
            let (q, r) = speech_part_4::fn_3003fc(m, v as u32);
            (q as u64, r as u8)
        } else {
            (v / radix as u64, (v % radix as u64) as u8)
        };
        byte(
            m,
            s + 36 + n,
            if digit < 10 {
                b'0' + digit
            } else {
                (if flags & 2048 != 0 { b'A' } else { b'a' }) + digit - 10
            },
        );
        n += 1;
        v = quotient;
    }
    let mut prefix = [0; 2];
    let mut count = 0;
    if signed {
        if negative {
            prefix[0] = b'-';
            count = 1;
        } else if flags & 2 != 0 {
            prefix[0] = b'+';
            count = 1;
        } else if flags & 4 != 0 {
            prefix[0] = b' ';
            count = 1;
        }
    } else if radix == 8 && flags & 8 != 0 && (n != 0 || flags & 32 != 0) {
        prefix[0] = b'0';
        count = 1;
        m.put_word(s + 28, m.word(s + 28).wrapping_sub(1));
    } else if radix == 16 && flags & 8 != 0 {
        if pointer {
            prefix[0] = b'@';
            count = 1;
        } else if n != 0 {
            prefix = [b'0', if flags & 2048 != 0 { b'X' } else { b'x' }];
            count = 2;
        }
    }
    let at = m.offset(tmp);
    let saved = [m.bytes[at], m.bytes[at + 1]];
    m.bytes[at..at + 2].copy_from_slice(&prefix);
    let result = speech_part_4::fn_2ffcfc(m, h, s, n, tmp, count);
    m.bytes[at..at + 2].copy_from_slice(&saved);
    result
}

pub(crate) fn fn_100418(m: &mut Memory<'_>, s: u32, _c: u32, va: u32) -> u32 {
    let p = m.word(va);
    let flags = m.word(s);
    let n = m.word(s + 32);
    if flags & 1024 != 0 {
        byte(m, p, n as u8);
    } else if flags & 256 != 0 {
        put_half(m, p, n as u16);
    } else if flags & 128 != 0 {
        m.put_word(p, n);
        m.put_word(p + 4, ((n as i32) >> 31) as u32);
    } else {
        m.put_word(p, n);
    }
    1
}
pub(crate) fn fn_100694(
    m: &mut Memory<'_>,
    h: &mut impl speech_part_4::Host,
    s: u32,
    c: u32,
    va: u32,
    tmp: u32,
) -> u32 {
    numeric(m, h, s, c, va, 10, false, false, tmp)
}
pub(crate) fn fn_10035c(
    m: &mut Memory<'_>,
    h: &mut impl speech_part_4::Host,
    s: u32,
    c: u32,
    va: u32,
    tmp: u32,
) -> u32 {
    numeric(m, h, s, c, va, 10, true, false, tmp)
}
pub(crate) fn fn_100448(
    m: &mut Memory<'_>,
    h: &mut impl speech_part_4::Host,
    s: u32,
    c: u32,
    va: u32,
    tmp: u32,
) -> u32 {
    numeric(m, h, s, c, va, 8, false, false, tmp)
}
pub(crate) fn fn_1004c0(
    m: &mut Memory<'_>,
    h: &mut impl speech_part_4::Host,
    s: u32,
    c: u32,
    va: u32,
    tmp: u32,
) -> u32 {
    m.put_word(s, m.word(s) | 32);
    m.put_word(s + 28, 8);
    numeric(m, h, s, c, va, 16, false, true, tmp)
}
pub(crate) fn fn_4a7aa4(
    m: &mut Memory<'_>,
    h: &mut impl speech_part_4::Host,
    s: u32,
    c: u32,
    va: u32,
    tmp: u32,
) -> u32 {
    numeric(m, h, s, c, va, 16, false, false, tmp)
}
pub(crate) fn fn_4a7a9c(
    m: &mut Memory<'_>,
    h: &mut impl speech_part_4::Host,
    s: u32,
    c: u32,
    va: u32,
    tmp: u32,
) -> u32 {
    numeric(m, h, s, c, va, 8, true, false, tmp)
}
pub(crate) fn fn_4a7a94(
    m: &mut Memory<'_>,
    h: &mut impl speech_part_4::Host,
    s: u32,
    c: u32,
    va: u32,
    tmp: u32,
) -> u32 {
    numeric(m, h, s, c, va, 16, true, false, tmp)
}
pub(crate) fn fn_302034(
    m: &mut Memory<'_>,
    h: &mut impl FormatHost,
    dst: u32,
    unit: u32,
    state: u32,
) -> u32 {
    let global = crate::render_part_d2::LocaleHost::locale_global(h, m);
    let slot = crate::speech_part_3::fn_302078(m, global);
    let locale = m.word(slot);
    if m.byte(locale + 257) == 1 {
        let field = locale + 263;
        let method = field.wrapping_add(m.word(field));
        return speech_part_4::Host::call(h, m, method, &[dst, unit, state]);
    }
    if unit > 255 || m.byte(locale + unit) == 0 {
        return u32::MAX;
    }
    let mut bytes = [0; 8];
    let mut conversion = [0; 8];
    let n = crate::boot_fmt::Host::encode(
        &mut crate::boot_fmt::CLocale,
        unit as u16,
        &mut bytes,
        &mut conversion,
    )
    .expect("single-byte locale");
    for (i, b) in bytes[..n].iter().enumerate() {
        byte(m, dst + i as u32, *b);
    }
    n as u32
}

fn format_text(
    m: &mut Memory<'_>,
    h: &mut impl FormatHost,
    s: u32,
    p: u32,
    one: bool,
    wide: bool,
    scratch: u32,
) -> u32 {
    let at = m.offset(scratch);
    let mut saved = [0; 32];
    saved.copy_from_slice(&m.bytes[at..at + 32]);
    for i in 0..8 {
        byte(m, scratch + i, 0);
    }
    let flags = m.word(s);
    let width = m.word(s + 20) != 0;
    let limit = if flags & 32 != 0 {
        m.word(s + 28)
    } else {
        u32::MAX
    };
    let mut units = 0u32;
    let mut n = 0u32;
    if !wide && !width {
        let count = if one { 1 } else { u32::MAX };
        speech_part_4::fn_100278(m, h, s, p, count);
        m.bytes[at..at + 32].copy_from_slice(&saved);
        return 1;
    }

    while (if wide && width {
        one || n < limit
    } else if flags & 32 != 0 {
        (n as i32) < (limit as i32)
    } else {
        true
    }) && (!one || units < 1)
    {
        let unit = if wide {
            half(m, p + units * 2) as u32
        } else {
            m.byte(p + units) as u32
        };
        if !one && unit == 0 {
            break;
        }
        if wide && width {
            n = n.wrapping_add(1);
        } else if wide {
            let len = fn_302034(m, h, scratch + 8, unit, scratch);
            if len != u32::MAX {
                if flags & 32 != 0 && n.wrapping_add(len) > limit {
                    break;
                }
                n = n.wrapping_add(len);
            }
        } else {
            if flags & 32 != 0 && n >= limit {
                break;
            }
            if crate::render_part_d2::fn_2ffae8(m, scratch + 8, unit, scratch, h) == 1 {
                n = n.wrapping_add(1);
            }
        }
        units = units.wrapping_add(1);
    }
    m.put_word(s + 24, m.word(s + 24).wrapping_sub(n));
    if wide && width {
        m.put_word(s + 32, m.word(s + 32).wrapping_add(n));
    }
    speech_part_4::fn_2ffc14(m, h, s);
    for i in 0..8 {
        byte(m, scratch + i, 0);
    }
    for i in 0..units {
        let unit = if wide {
            half(m, p + i * 2) as u32
        } else {
            m.byte(p + i) as u32
        };
        if wide && width {
            output(m, h, s, unit);
        } else if wide {
            let len = fn_302034(m, h, scratch + 8, unit, scratch);
            if len != u32::MAX {
                for j in 0..len {
                    output(m, h, s, m.byte(scratch + 8 + j) as u32);
                }
            }
        } else if crate::render_part_d2::fn_2ffae8(m, scratch + 8, unit, scratch, h) == 1 {
            output(m, h, s, half(m, scratch + 8) as u32);
            m.put_word(s + 32, m.word(s + 32).wrapping_add(1));
        }
    }
    if wide && !width {
        m.put_word(s + 32, m.word(s + 32).wrapping_add(n));
    }
    speech_part_4::fn_2ffc40(m, h, s);
    m.bytes[at..at + 32].copy_from_slice(&saved);
    1
}
pub(crate) fn fn_1007fc(
    m: &mut Memory<'_>,
    h: &mut impl FormatHost,
    s: u32,
    _c: u32,
    va: u32,
    tmp: u32,
) -> u32 {
    byte(m, s + 36, m.word(va) as u8);
    byte(m, s + 37, 0);
    format_text(m, h, s, s + 36, true, false, tmp)
}
pub(crate) fn fn_100834(
    m: &mut Memory<'_>,
    h: &mut impl FormatHost,
    s: u32,
    _c: u32,
    va: u32,
    tmp: u32,
) -> u32 {
    put_half(m, s + 36, m.word(va) as u16);
    put_half(m, s + 38, 0);
    format_text(m, h, s, s + 36, true, true, tmp)
}
pub(crate) fn fn_4a7a8c(
    m: &mut Memory<'_>,
    h: &mut impl FormatHost,
    s: u32,
    _c: u32,
    va: u32,
    tmp: u32,
) -> u32 {
    format_text(m, h, s, m.word(va), false, false, tmp)
}
pub(crate) fn fn_4a7a84(
    m: &mut Memory<'_>,
    h: &mut impl FormatHost,
    s: u32,
    _c: u32,
    va: u32,
    tmp: u32,
) -> u32 {
    format_text(m, h, s, m.word(va), false, true, tmp)
}

pub(crate) fn fn_1a7970(
    m: &mut Memory<'_>,
    h: &mut impl speech_part_4::Host,
    s: u32,
    c: u32,
    va: u32,
) -> u32 {
    fn_4a7a7c(m, h, s, c, va)
}
pub(crate) fn fn_1a7974(
    m: &mut Memory<'_>,
    h: &mut impl speech_part_4::Host,
    s: u32,
    c: u32,
    va: u32,
) -> u32 {
    fn_10086c(m, h, s, c, va)
}

pub fn replay_format(
    m: &mut Memory<'_>,
    h: &mut impl FormatHost,
    f: u32,
    s: u32,
    c: u32,
    va: u32,
    tmp: u32,
) -> u32 {
    match f {
        0x4a7a7c => fn_4a7a7c(m, h, s, c, va),
        0x10086c => fn_10086c(m, h, s, c, va),
        0x100418 => fn_100418(m, s, c, va),
        0x100694 => fn_100694(m, h, s, c, va, tmp),
        0x10035c => fn_10035c(m, h, s, c, va, tmp),
        0x100448 => fn_100448(m, h, s, c, va, tmp),
        0x1004c0 => fn_1004c0(m, h, s, c, va, tmp),
        0x4a7aa4 => fn_4a7aa4(m, h, s, c, va, tmp),
        0x4a7a9c => fn_4a7a9c(m, h, s, c, va, tmp),
        0x4a7a94 => fn_4a7a94(m, h, s, c, va, tmp),
        0x1007fc => fn_1007fc(m, h, s, c, va, tmp),
        0x100834 => fn_100834(m, h, s, c, va, tmp),
        0x4a7a8c => fn_4a7a8c(m, h, s, c, va, tmp),
        0x4a7a84 => fn_4a7a84(m, h, s, c, va, tmp),
        0x1a7970 => fn_1a7970(m, h, s, c, va),
        0x1a7974 => fn_1a7974(m, h, s, c, va),
        _ => panic!("unknown conversion entry"),
    }
}

fn float_emit(m: &mut Memory<'_>, h: &mut impl speech_part_4::Host, s: u32, b: u8) {
    output(m, h, s, b as u32);
    m.put_word(s + 32, m.word(s + 32).wrapping_add(1));
}
fn float_tail(buf: &mut [u8; 32], e: i32, letter: u8, min: usize) -> usize {
    let mut n = 0;
    crate::boot_fmt::exponent(buf, &mut n, e, letter, min);
    n
}
fn float_special(m: &mut Memory<'_>, h: &mut impl speech_part_4::Host, s: u32, bits: u64) -> Option<u32> {
    if bits >> 52 & 2047 != 2047 { return None; }
    let flags = m.word(s) & !16;
    m.put_word(s, flags);
    let sign = float_sign(flags, bits);
    m.put_word(s + 24, m.word(s + 24).wrapping_sub(3 + u32::from(sign != 0)));
    speech_part_4::fn_2ffc14(m, h, s);
    if sign != 0 { float_emit(m, h, s, sign); }
    let text = match (bits & 0x000f_ffff_ffff_ffff == 0, flags & 2048 != 0) {
        (true, false) => b"inf", (true, true) => b"INF",
        (false, false) => b"nan", (false, true) => b"NAN",
    };
    m.put_word(s + 32, m.word(s + 32).wrapping_add(3));
    for &b in text { output(m, h, s, b as u32); }
    speech_part_4::fn_2ffc40(m, h, s);
    Some(2)
}
fn float_sign(flags: u32, bits: u64) -> u8 {
    if bits >> 63 != 0 { b'-' } else if flags & 2 != 0 { b'+' }
    else if flags & 4 != 0 { b' ' } else { 0 }
}
pub(crate) fn fn_4a7a7c(m: &mut Memory<'_>, h: &mut impl speech_part_4::Host, s: u32, c: u32, va: u32) -> u32 {
    let va = va.wrapping_add(7) & !7;
    let bits = m.word(va) as u64 | ((m.word(va + 4) as u64) << 32);
    if let Some(rc) = float_special(m, h, s, bits) { return rc; }
    let flags = m.word(s);
    let sign = float_sign(flags, bits);
    let p = if flags & 32 != 0 { m.word(s + 28) as i32 } else { 6 };
    let d = crate::boot_fmt::rvct_decimal(bits, c as u8, p, 0);
    let e = if d.zero() { 0 } else { d.exponent() };
    let scientific = c == 101 || c == 103 && (e < -4 || e >= p.max(1));
    let mut places = if c == 103 {
        if scientific { p.max(1) - 1 } else { (p.max(1) - 1 - e).max(0) }
    } else { p };
    if c == 103 && flags & 8 == 0 {
        while places > 0 {
            if d.at(if scientific { (e - places) as i64 } else { -places as i64 }) != b'0' { break; }
            places -= 1;
        }
    }
    let first = if scientific { e } else { e.max(0) };
    let last = if scientific { e - places } else { -places };
    let point = places > 0 || flags & 8 != 0;
    let mut tail = [0; 32];
    let tn = if scientific { float_tail(&mut tail, e, if flags & 2048 != 0 { b'E' } else { b'e' }, 2) } else { 0 };
    let len = (first - last + 1) as u32 + u32::from(point) + u32::from(sign != 0) + tn as u32;
    m.put_word(s + 24, m.word(s + 24).wrapping_sub(len));
    if flags & 16 == 0 { speech_part_4::fn_2ffc14(m, h, s); }
    if sign != 0 { float_emit(m, h, s, sign); }
    if flags & 16 != 0 { speech_part_4::fn_2ffc14(m, h, s); }
    for power in (last..=first).rev() {
        float_emit(m, h, s, d.at(power as i64));
        if power == (if scientific { e } else { 0 }) && point {
            let point = h.decimal_point(m);
            float_emit(m, h, s, point);
        }
    }
    for &b in &tail[..tn] { float_emit(m, h, s, b); }
    speech_part_4::fn_2ffc40(m, h, s);
    2
}
pub(crate) fn fn_10086c(m: &mut Memory<'_>, h: &mut impl speech_part_4::Host, s: u32, _c: u32, va: u32) -> u32 {
    let va = va.wrapping_add(7) & !7;
    let bits = m.word(va) as u64 | ((m.word(va + 4) as u64) << 32);
    if let Some(rc) = float_special(m, h, s, bits) { return rc; }
    let flags = m.word(s);
    let sign = float_sign(flags, bits);
    let explicit = flags & 32 != 0;
    let p = if explicit { m.word(s + 28) as i32 } else { 13 };
    let exp = ((bits >> 52) & 2047) as i32;
    let frac = bits & 0x000f_ffff_ffff_ffff;
    let mut value = frac | if exp != 0 { 1 << 52 } else { 0 };
    let mut e = if exp != 0 { exp - 1023 } else if frac != 0 { -1022 } else { 0 };
    if (0..13).contains(&p) {
        let shift = 52 - 4 * p;
        let mask = (1u64 << shift) - 1;

        let rem = if p < 5 { ((bits >> 32) as u32).wrapping_shl((4 * p + 12) as u32) }
            else { (bits as u32).wrapping_shl((4 * p - 20) as u32) };
        let half = 1u32 << 31;
        let up = rem > half || rem == half && (value >> shift) & 1 != 0;
        value &= !mask;
        if up { value += 1 << shift; }
        if value >= 1 << 53 { value >>= 1; e += 1; }
    }
    let alphabet = if flags & 2048 != 0 { b"0123456789ABCDEF" } else { b"0123456789abcdef" };
    let mut head = [0; 32];
    let mut n = 0;
    head[n] = b'0' + (value >> 52) as u8; n += 1;
    if p > 0 || flags & 8 != 0 { head[n] = b'.'; n += 1; }
    for i in 0..p.min(13) { head[n] = alphabet[((value >> (48 - 4 * i)) & 15) as usize]; n += 1; }
    if !explicit {
        while head[n - 1] == b'0' { n -= 1; }
        if head[n - 1] == b'.' { n -= 1; }
    }
    let zeros = (p - 14).max(0);
    let mut tail = [0; 32];
    let tn = float_tail(&mut tail, e, if flags & 2048 != 0 { b'P' } else { b'p' }, 1);
    let len = n as u32 + zeros as u32 + tn as u32 + 2 + u32::from(sign != 0) + u32::from(p > 13);
    m.put_word(s + 24, m.word(s + 24).wrapping_sub(len));
    if flags & 16 == 0 { speech_part_4::fn_2ffc14(m, h, s); }

    m.put_word(s + 32, m.word(s + 32).wrapping_add(2 + u32::from(sign != 0)));
    if sign != 0 { output(m, h, s, sign as u32); }
    output(m, h, s, b'0' as u32);
    output(m, h, s, if flags & 2048 != 0 { b'X' as u32 } else { b'x' as u32 });
    if flags & 16 != 0 { speech_part_4::fn_2ffc14(m, h, s); }
    for &b in &head[..n] { float_emit(m, h, s, b); }
    m.put_word(s + 32, m.word(s + 32).wrapping_add(zeros as u32));
    for _ in 0..zeros { output(m, h, s, b'0' as u32); }
    for &b in &tail[..tn] { float_emit(m, h, s, b); }
    speech_part_4::fn_2ffc40(m, h, s);
    2
}
