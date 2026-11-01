
use crate::{boot::Memory, fe_text};

fn half(m: &Memory<'_>, p: u32) -> u16 {
    let at = m.offset(p);
    u16::from_le_bytes(m.bytes[at..at + 2].try_into().unwrap())
}
fn byte(m: &mut Memory<'_>, p: u32, v: u8) {
    let at = m.offset(p);
    m.bytes[at] = v;
}
fn strlen(m: &Memory<'_>, p: u32) -> u16 {
    fe_text::strlen_195d90(m.bytes, p - m.base) as u16
}
fn move_bytes(m: &mut Memory<'_>, d: u32, s: u32, n: u32) {
    fe_text::memmove_195b38(m.bytes, d - m.base, s - m.base, n);
}
fn copy_bytes(m: &mut Memory<'_>, d: u32, s: u32, n: u32) {
    fe_text::memcpy_195ae8(m.bytes, d - m.base, s - m.base, n);
}

pub fn fn_1761d4(m: &mut Memory<'_>, dst: u32, mut src: u32) -> u32 {
    let mut d = dst;
    while half(m, d) != 0 {
        d = d.wrapping_add(2);
    }
    loop {
        let c = half(m, src);
        m.put_half(d, c);
        if c == 0 {
            return dst;
        }
        d = d.wrapping_add(2);
        src = src.wrapping_add(2);
    }
}

pub fn fn_176354(m: &mut Memory<'_>, dst: u32, mut src: u32, mut n: u32) -> u32 {
    let mut d = dst;
    while n != 0 {
        let c = m.byte(src);
        m.put_half(d, c as u16);
        if c == 0 {
            n -= 1;
            while n != 0 {
                m.put_half(d, 0);
                d = d.wrapping_add(2);
                n -= 1;
            }
            break;
        }
        n -= 1;
        d = d.wrapping_add(2);
        src = src.wrapping_add(1);
    }
    dst
}

pub fn fn_17b7e4(m: &Memory<'_>, object: u32) -> u32 {
    if object == 0 {
        0
    } else {
        m.word(object + 12)
    }
}

pub fn fn_17ba4c(m: &mut Memory<'_>, object: u32) -> u32 {
    if object == 0 {
        return 0x8a402007;
    }
    let text = m.word(object + 12);
    if text != 0 {
        byte(m, text, 0);
        m.put_word(object + 8, 0);
    }
    0
}

pub fn fn_17c4d8(x: i32) -> i32 {
    if x < 0 {
        x.wrapping_neg()
    } else {
        x
    }
}

pub fn fn_265be4(m: &Memory<'_>, base: u32, n: u32, lim: u32, idx: u32, flag: i32) -> u32 {
    let mut i = 0u32;
    let mut ret = m.word(base + 12);
    while i < n && i < idx {
        let e = base.wrapping_add(i.wrapping_mul(40));
        if (m.word(e + 20).wrapping_sub(m.word(base + 20)) as u16 as u32) >= lim {
            break;
        }
        let next = i.wrapping_add(1);
        if next >= n {
            break;
        }
        if (m.word(e + 60).wrapping_sub(m.word(base + 20)) as u16 as u32) < lim {
            i = next as u16 as u32;
        } else {
            break;
        }
    }
    if i != 0 {
        let e = base.wrapping_add(i.wrapping_mul(40));
        ret = if flag == 1 && matches!(half(m, e + 10), 1 | 41) {
            m.word(e + 12)
        } else {
            m.word(e + 16).wrapping_add(m.word(e + 12))
        };
    }
    ret
}

pub fn fn_26a6e4(
    m: &mut Memory<'_>,
    text: u32,
    pattern: u32,
    pattern_length: u32,
    text_length: &mut u16,
    mode: i32,
) {
    let mut index = 0u16;
    'scan: loop {
        while index < *text_length && m.byte(text + index as u32) != 0 {
            if index != 0 && mode == 1 {
                return;
            }
            if m.byte(text + index as u32) == m.byte(pattern) {
                let start = index;
                let mut matches = true;
                let mut j = 0u16;
                while matches {
                    if j as u32 >= pattern_length {
                        move_bytes(
                            m,
                            text + start as u32,
                            text + start as u32 + pattern_length,
                            (*text_length as u32)
                                .wrapping_sub(start as u32)
                                .wrapping_sub(pattern_length)
                                .wrapping_add(1),
                        );
                        index = (index as u32).wrapping_sub(pattern_length) as u16;
                        *text_length = (*text_length as u32).wrapping_sub(pattern_length) as u16;
                        continue 'scan;
                    }
                    if m.byte(text + index as u32) != m.byte(pattern + j as u32) {
                        matches = false;
                    }
                    index = index.wrapping_add(1);
                    j = j.wrapping_add(1);
                }
            }
            index = index.wrapping_add(1);
        }
        return;
    }
}

pub fn fn_266034(m: &mut Memory<'_>, table: u32, text: u32, pattern: u32) {
    let mut textlen = strlen(m, text);
    let patlen = strlen(m, pattern);
    let entries = m.word(table + 4);
    let count = half(m, table + 8) as u32;
    let mut removed = 0u16;
    let mut pos = 0u16;
    while pos < textlen && m.byte(text + pos as u32) != 0 {
        if m.byte(text + pos as u32) != m.byte(pattern) {
            pos = pos.wrapping_add(1);
            continue;
        }
        let start = pos as u32;
        let mut j = 0u16;
        let mut matches = true;
        while matches {
            if j >= patlen {
                let sl = start.wrapping_sub((removed as u32).wrapping_mul(patlen as u32));
                if count > 1 {
                    let mut idx = 1u32;
                    while idx < count {
                        let e = entries + idx * 40;
                        if (m.word(e + 20).wrapping_sub(m.word(entries + 20)) as u16 as u32) <= sl {
                            idx += 1;
                        } else {
                            while idx < count {
                                let e = entries + idx * 40;
                                if m.word(e + 20).wrapping_sub(m.word(entries + 20)) as u16
                                    >= patlen
                                {
                                    m.put_word(e + 20, m.word(e + 20).wrapping_sub(patlen as u32));
                                } else {
                                    m.put_word(e + 20, m.word(entries + 20));
                                }
                                idx += 1;
                            }
                        }
                    }
                }
                removed = removed.wrapping_add(1);
                break;
            }
            if m.byte(text + pos as u32) != m.byte(pattern + j as u32) {
                matches = false;
            }
            j = j.wrapping_add(1);
            pos = pos.wrapping_add(1);
        }
    }
    fn_26a6e4(m, text, pattern, patlen as u32, &mut textlen, 0);
    m.put_word(entries + 24, textlen as u32);
}

pub fn fn_2661a4(m: &mut Memory<'_>, table: u32, text: u32, pattern: u32, dict: u32, mode: i32) {
    let mut textlen = strlen(m, text);
    let patlen = strlen(m, pattern);
    let dictlen = strlen(m, dict) as u32;
    let delta = dictlen.wrapping_sub(patlen as u32) as i16 as i32 as u32;
    let entries = m.word(table + 4);
    let count = half(m, table + 8) as u32;
    let mut i = 0u16;
    while i < textlen && m.byte(text + i as u32) != 0 && (i == 0 || mode != 1) {
        if m.byte(text + i as u32) != m.byte(pattern) {
            i = i.wrapping_add(1);
            continue;
        }
        let start = i;
        let mut matches = true;
        let mut j = 0u16;
        while matches {
            if j >= patlen {
                let newlen = (strlen(m, text) as u32).wrapping_add(delta) as u16;
                move_bytes(
                    m,
                    (text + start as u32 + patlen as u32).wrapping_add(delta),
                    text + start as u32 + patlen as u32,
                    (textlen as u32)
                        .wrapping_sub(start as u32)
                        .wrapping_sub(patlen as u32)
                        .wrapping_add(1),
                );
                copy_bytes(m, text + start as u32, dict, dictlen);
                byte(m, text + newlen as u32, 0);
                textlen = newlen;
                let mut idx = 1;
                while idx < count
                    && (m
                        .word(entries + idx * 40 + 20)
                        .wrapping_sub(m.word(entries + 20)) as u16)
                        <= start
                {
                    idx += 1;
                }
                while idx < count {
                    let e = entries + idx * 40;
                    m.put_word(e + 20, m.word(e + 20).wrapping_add(delta));
                    idx += 1;
                }
                break;
            }
            if m.byte(text + i as u32) != m.byte(pattern + j as u32) {
                matches = false;
            }
            j = j.wrapping_add(1);
            i = i.wrapping_add(1);
        }
    }
    fn_26a6e4(m, text, pattern, patlen as u32, &mut textlen, mode);
    m.put_word(entries + 24, textlen as u32);
}

pub fn fn_1767c4(m: &mut Memory<'_>, text: u32, end: u32, mut base: i32) -> u32 {
    let mut p = text.wrapping_add(2);
    let mut c = half(m, text) as u32;
    while c & 0xff00 == 0 && (c == 32 || c.wrapping_sub(9) <= 4) {
        c = half(m, p) as u32;
        p = p.wrapping_add(2);
    }
    let negative = c == 45;
    if c == 45 || c == 43 {
        c = half(m, p) as u32;
        p = p.wrapping_add(2);
    }
    if base < 0 || base == 1 || base > 36 {
        if end != 0 {
            m.put_word(end, text);
        }
        return 0;
    }
    let mut prefix = false;
    if base == 0 {
        if c != 48 {
            base = 10;
        } else if matches!(half(m, p), 120 | 88) {
            base = 16;
            prefix = true;
        } else {
            base = 8;
        }
    } else if base == 16 && c == 48 && matches!(half(m, p), 120 | 88) {
        prefix = true;
    }
    if prefix {
        p = p.wrapping_add(2);
        c = half(m, p) as u32;
        p = p.wrapping_add(2);
    }
    let (cutoff, cutlim) = crate::frontend::divmod_u32(u32::MAX, base as u32);
    let mut value = 0u32;
    let mut digits = false;
    let mut overflow = false;
    loop {
        if c & 0xff00 != 0 {
            break;
        }
        let digit = if c.wrapping_sub(48) <= 9 {
            c - 48
        } else {
            if c.wrapping_sub(97) > 25 && c.wrapping_sub(65) > 25 {
                break;
            }
            fe_text::fold_17c8dc(c).wrapping_sub(65).wrapping_add(10)
        };
        if digit >= base as u32 {
            break;
        }
        digits = true;
        if value < cutoff || (value == cutoff && digit <= cutlim) {
            value = value.wrapping_mul(base as u32).wrapping_add(digit);
        } else {
            overflow = true;
        }
        c = half(m, p) as u32;
        p = p.wrapping_add(2);
    }
    p = p.wrapping_sub(2);
    if !digits {
        value = 0;
        p = text;
    } else if overflow {
        value = u32::MAX;
    }
    if end != 0 {
        m.put_word(end, p);
    }
    if negative {
        value.wrapping_neg()
    } else {
        value
    }
}

pub fn fn_17b53c(
    m: &mut Memory<'_>,
    object: u32,
    len: i32,
    host: &mut impl crate::frontend::HeapVtable,
) -> u32 {
    if object == 0 {
        return 0x8a402007;
    }
    let need = (len as u32).wrapping_add(1);
    if m.word(object + 4) < need {
        let heap = m.word(object);
        let heap = if heap == 0 { None } else { Some(heap - m.base) };
        let old = m.word(object + 12);
        let b = if old == 0 {
            let b = crate::frontend::heap_alloc(m.bytes, heap, need, host);
            if b != 0 {
                byte(m, b, 0);
            }
            b
        } else {
            crate::frontend::heap_realloc(m.bytes, heap, old, need, host)
        };
        if b == 0 {
            return 0x8a40200a;
        }
        m.put_word(object + 4, need);
        m.put_word(object + 12, b);
    }
    0
}

pub fn fn_17b53c_m(
    m: &mut Memory<'_>,
    object: u32,
    len: i32,
    host: &mut impl crate::frontend::HeapVtableM,
) -> u32 {
    if object == 0 {
        return 0x8a402007;
    }
    let need = (len as u32).wrapping_add(1);
    if m.word(object + 4) < need {
        let heap = m.word(object);
        let heap = if heap == 0 { None } else { Some(heap - m.base) };
        let old = m.word(object + 12);
        let b = if old == 0 {
            let b = crate::frontend::heap_alloc_m(m.bytes, heap, need, host);
            if b != 0 {
                byte(m, b, 0);
            }
            b
        } else {
            crate::frontend::heap_realloc_m(m.bytes, heap, old, need, host)
        };
        if b == 0 {
            return 0x8a40200a;
        }
        m.put_word(object + 4, need);
        m.put_word(object + 12, b);
    }
    0
}

pub trait ParameterHost: crate::frontend::HeapVtable + crate::frontend::TreeOps {
    fn invoke(&mut self, function: u32, args: &[u32]) -> u32;

    fn offer_value(&mut self, value: [u32; 3]) {
        let _ = value;
    }
    fn compare_key(&mut self, memory: &[u8], a: u32, b: u32) -> i32 {
        let _ = memory;
        self.tree_cmp(a, b)
    }
}
struct HeapProxy<'a, H>(&'a core::cell::RefCell<&'a mut H>);
impl<H: ParameterHost> crate::frontend::HeapVtable for HeapProxy<'_, H> {
    fn alloc_at(&mut self, o: u32, n: u32) -> u32 {
        self.0.borrow_mut().alloc_at(o, n)
    }
    fn realloc_at(&mut self, o: u32, p: u32, n: u32) -> u32 {
        self.0.borrow_mut().realloc_at(o, p, n)
    }
    fn release_at(&mut self, o: u32, p: u32) {
        self.0.borrow_mut().release_at(o, p)
    }
}
struct TreeProxy<'a, H>(&'a core::cell::RefCell<&'a mut H>, [u32; 3]);
impl<H: ParameterHost> crate::frontend::TreeOps for TreeProxy<'_, H> {
    fn tree_cmp(&mut self, a: u32, b: u32) -> i32 {
        self.0.borrow_mut().tree_cmp(a, b)
    }
    fn tree_init(&mut self, a: u32, b: u32, c: u32) -> (u32, Option<u32>) {
        self.0.borrow_mut().tree_init(a, b, c)
    }
    fn tree_alloc(&mut self, a: u32, b: u32) -> (u32, u32) {
        self.0.borrow_mut().offer_value(self.1);
        self.0.borrow_mut().tree_alloc(a, b)
    }
    fn tree_free_node(&mut self, a: u32, b: u32) {
        self.0.borrow_mut().tree_free_node(a, b)
    }
    fn tree_free_handle(&mut self, a: u32, b: u32) {
        self.0.borrow_mut().tree_free_handle(a, b)
    }
}
fn nz(p: u32) -> Option<u32> {
    if p == 0 {
        None
    } else {
        Some(p)
    }
}

fn erase_release(m: &mut Memory<'_>, tree: u32, node: u32, h: &mut impl ParameterHost) {
    crate::normalization::rb_erase(m.bytes, tree, node);
    let vt = m.word(tree + 36);
    let key = m.word(node + 16);
    if key != 0 {
        h.invoke(m.word(vt + 4), &[key, m.word(tree + 32)]);
    }
    let value = m.word(node + 20);
    if value != 0 {
        h.invoke(m.word(vt + 16), &[value, m.word(tree + 32)]);
    }
    crate::frontend::heap_release(m.bytes, nz(m.word(tree + 32)), nz(node), h);
}

pub fn fn_175a24(
    m: &mut Memory<'_>,
    p: u32,
    key: u32,
    scratch: u32,
    h: &mut impl ParameterHost,
) -> u32 {
    assert_eq!(m.base, 0, "parameter ports require flat guest memory");
    if p == 0 || key == 0 {
        return 0x8b702007;
    }
    let lock = m.word(p + 4);
    let rc = crate::frontend::acquire_lock(m.bytes, nz(lock), |target| {
        h.invoke(m.word(m.word(m.word(lock) + 8) + 8), &[target])
    });
    if rc >= 0x80000000 {
        return rc;
    }
    let mut value = 0;
    let rc =
        crate::frontend::lookup_value(m.bytes, nz(m.word(p + 8)), key, Some(&mut value), |a, b| {
            h.compare_key(m.bytes, a, b)
        });
    let mut result;
    if rc >= 0x80000000 {
        result = 0x8b702000 | (rc & 0x1fff);
    } else {
        let callbacks = m.word(value + 8);
        let count = if callbacks == 0 {
            0
        } else {
            let words = core::array::from_fn(|i| m.word(callbacks + i as u32 * 4));
            crate::frontend::word_at5(Some(&words))
        };
        if callbacks != 0 && count != 0 {
            result = 0x8b702011;
        } else {
            let size = m.word(value + 4);
            let data = if size == 0 { 0 } else { m.word(value) };
            let cell = core::cell::RefCell::new(&mut *h);
            result = crate::frontend::update_attr(
                m.bytes,
                p,
                key,
                data,
                size,
                0,
                scratch,
                &mut HeapProxy(&cell),
                &mut TreeProxy(&cell, [data, size, 0]),
                |a, b| cell.borrow_mut().tree_cmp(a, b),
            );
            if result < 0x80000000 {
                let mut found = None;
                let rc = crate::frontend::remove_tree_value(
                    m.bytes,
                    nz(m.word(p + 8)),
                    key,
                    |a, b| cell.borrow_mut().tree_cmp(a, b),
                    |t, n| found = Some((t, n)),
                );
                if let Some((t, n)) = found {
                    erase_release(m, t, n, &mut **cell.borrow_mut());
                }
                result = if rc >= 0x80000000 {
                    0x8b702000 | (rc & 0x1fff)
                } else {
                    rc
                };
            }
        }
    }
    let rc = crate::frontend::release_lock(m.bytes, nz(lock), |target| {
        h.invoke(m.word(m.word(m.word(lock) + 8) + 12), &[target])
    });
    if rc >= 0x80000000 && result < 0x80000000 {
        result = rc;
    }
    result
}

pub fn fn_1968cc(m: &mut Memory<'_>, p: u32, h: &mut impl ParameterHost) -> u32 {
    assert_eq!(m.base, 0, "tree ports require flat guest memory");
    if p == 0 {
        return 0x89002007;
    }
    let node = m.word(p + 4);
    if node == 0 {
        return 0x89002000;
    }
    erase_release(m, m.word(p) + 24, node, h);
    m.put_word(p + 4, 0);
    0
}

pub fn fn_1ef60c(
    m: &mut Memory<'_>,
    p: u32,
    mut s1: u32,
    s2: u32,
    out: u32,
    scratch: u32,
    h: &mut impl ParameterHost,
) -> u32 {
    assert_eq!(m.base, 0, "lookup ports require flat guest memory");
    let lid = scratch + 512;
    for (i, c) in b"_lid\0".iter().enumerate() {
        byte(m, lid + i as u32, *c);
    }
    crate::fe_text::strcpy_widen_1763e4(m.bytes, scratch + 520, lid);
    let lid = scratch + 520;
    for (i, c) in b"normal\0".iter().enumerate() {
        byte(m, scratch + 512 + i as u32, *c);
    }
    crate::fe_text::strcpy_widen_1763e4(m.bytes, scratch + 532, scratch + 512);
    if s1 == 0 {
        s1 = scratch + 532;
    }
    let short = crate::normalization::u16_strstr(m.bytes, s1, lid) != 0;
    let mut best = 0u32;
    for kind in 0..3 {
        let mut w = 0;
        if kind == 2 {
            byte(m, scratch, b'*');
            byte(m, scratch + 1, b'+');
            w = 2;
        } else {
            let n = if short {
                3
            } else {
                crate::normalization::u16_len(m.bytes, s1)
            };
            let mut i = 0;
            while i < n && w < 255 {
                let c = crate::normalization::tolower_240ba0(half(m, s1 + i * 2) as u32 & 255);
                byte(m, scratch + w, c as u8);
                i += 1;
                w += 1;
            }
            if w < 255 {
                byte(m, scratch + w, b'+');
                w += 1;
            }
            if kind == 1 && w < 255 {
                byte(m, scratch + w, b'*');
                w += 1;
            }
        }
        if kind != 1 {
            let mut i = 0;
            while i < crate::normalization::u16_len(m.bytes, s2) && w < 255 {
                byte(
                    m,
                    scratch + w,
                    crate::normalization::tolower_240ba0(half(m, s2 + i * 2) as u32 & 255) as u8,
                );
                i += 1;
                w += 1;
            }
        }
        byte(m, scratch + w, 0);
        let hit = crate::frontend::find_substr(m.bytes, m.word(p + 80), scratch);
        if hit != 0 && (best == 0 || hit < best) {
            best = hit;
        }
    }
    if best == 0 {
        return 0x8c902000;
    }
    while m.byte(best) != b'=' {
        if m.byte(best) == 0 {
            return 0x8c902000;
        }
        best += 1;
    }
    let mut j = 0;
    while j < 255 {
        let c = m.byte(best + 1 + j);
        if c == 0 || c == b',' {
            break;
        }
        byte(m, scratch + 256 + j, c);
        j += 1;
    }
    byte(m, scratch + 256 + j, 0);
    let mut value = 0;
    let rc = crate::frontend::lookup_value(
        m.bytes,
        nz(m.word(p + 76)),
        scratch + 256,
        Some(&mut value),
        |a, b| h.compare_key(m.bytes, a, b),
    );
    if rc < 0x80000000 {
        m.put_word(out, value);
    }
    rc
}

pub trait SequenceHost: ParameterHost + crate::frontend::MethodTable {
    fn cached_pair(&self) -> [u8; 8];
    fn default_context(&self) -> u32;
}
pub(crate) struct FetchHost<'a, H>(pub(crate) &'a mut H);
impl<H: SequenceHost> crate::normalization::NormalizationHost for FetchHost<'_, H> {
    fn calloc(&mut self, a: &mut [u8], heap: u32, n: u32, s: u32) -> Option<u32> {
        nz(crate::frontend::call_method(a, nz(heap), n, s, self.0))
    }
    fn alloc(&mut self, a: &mut [u8], heap: u32, n: u32) -> Option<u32> {
        nz(crate::frontend::heap_alloc(a, nz(heap), n, self.0))
    }
    fn realloc(&mut self, a: &mut [u8], heap: u32, p: u32, n: u32) -> Option<u32> {
        nz(crate::frontend::heap_realloc(a, nz(heap), p, n, self.0))
    }
    fn free(&mut self, a: &mut [u8], heap: u32, p: u32) {
        crate::frontend::heap_release(a, nz(heap), nz(p), self.0)
    }
    fn resolve(&mut self, a: &mut [u8], p: u32, tag: u32) -> (u32, u32) {
        let mut ctx = 0;
        let pair = self.0.cached_pair();
        let cached = [
            u32::from_le_bytes(pair[..4].try_into().unwrap()),
            u32::from_le_bytes(pair[4..].try_into().unwrap()),
        ];
        let word = if p == 0 || tag != 0x1e61 || [p, tag] == cached {
            None
        } else {
            Some(u16::from_le_bytes(
                a[p as usize..p as usize + 2].try_into().unwrap(),
            ))
        };
        let rc = crate::frontend::resolve_engine_context(
            p,
            word,
            tag,
            &pair,
            self.0.default_context(),
            &mut ctx,
        );
        (rc, ctx)
    }
}
pub(crate) fn fetch(
    m: &mut Memory<'_>,
    seq: u32,
    tag: u32,
    n: u32,
    out: &mut u32,
    h: &mut impl SequenceHost,
) -> u32 {
    assert_eq!(m.base, 0, "sequence ports require flat guest memory");
    crate::normalization::fetch_entry(m.bytes, seq, tag, n, Some(out), &mut FetchHost(h))
}

pub fn fn_265c94(
    m: &mut Memory<'_>,
    seq: u32,
    text: u32,
    pattern: u32,
    mut table: u32,
    val: u32,
    float_bits: u32,
    h: &mut impl SequenceHost,
) -> u32 {
    let mut textlen = strlen(m, text);
    let patlen = strlen(m, pattern);
    let mut entries = m.word(table + 4);
    let mut rc = 0;
    let mut removed = 0u16;
    let mut i = 0u16;
    while i < textlen && m.byte(text + i as u32) != 0 {
        if m.byte(text + i as u32) == m.byte(pattern) {
            let start = i as u32;
            let mut j = 0u16;
            let mut matched = true;
            while matched {
                if j >= patlen {
                    rc = fetch(
                        m,
                        seq,
                        0x3fd,
                        half(m, table + 8).wrapping_add(1) as u32,
                        &mut table,
                        h,
                    );
                    if rc >= 0x80000000 {
                        m.put_word(entries + 24, textlen as u32);
                        return rc;
                    }
                    entries = m.word(table + 4);
                    let count = half(m, table + 8) as u32;
                    let pos = start.wrapping_sub(removed as u32 * patlen as u32) as u16 as u32;
                    let slot = entries + count * 40;
                    let last =
                        m.word(slot - 40 + 20).wrapping_sub(m.word(entries + 20)) as u16 as u32;
                    if pos < last && count != 1 {
                        if count != 0 {
                            let mut idx = 1;
                            loop {
                                let e = entries + idx * 40;
                                let d =
                                    m.word(e + 20).wrapping_sub(m.word(entries + 20)) as u16 as u32;
                                if d <= pos {
                                    idx = (idx as u16).wrapping_add(1) as u32;
                                } else {
                                    break;
                                }
                                if half(m, table + 8) as u32 <= idx {
                                    break;
                                }
                            }
                            let boundary = fn_265be4(m, entries, count, pos, idx, 1);
                            let e = entries + idx * 40;
                            move_bytes(m, e + 40, e, count.wrapping_sub(idx) * 40);
                            let ne = m.word(table + 4) + idx * 40;
                            marker(
                                m,
                                ne,
                                val,
                                boundary,
                                m.word(entries + 20).wrapping_add(pos),
                                float_bits,
                            );
                            if half(m, table + 8) as u32 + 1 > 1 {
                                let split = (half(m, table + 8) as u32 & 1) + 1;
                                let mut at = 1;
                                while at < split {
                                    let e = entries + at * 40;
                                    let v = m.word(e + 20);
                                    at += 1;
                                    if v > m.word(ne + 20) {
                                        m.put_word(e + 20, v.wrapping_sub(patlen as u32));
                                    }
                                }
                                at = split;
                                while at < half(m, table + 8) as u32 + 1 {
                                    let e = entries + at * 40;
                                    let v = m.word(e + 20);
                                    let v2 = m.word(e + 60);
                                    at += 2;
                                    if v > m.word(ne + 20) {
                                        m.put_word(e + 20, v.wrapping_sub(patlen as u32));
                                    }
                                    if v2 > m.word(ne + 20) {
                                        m.put_word(e + 60, v2.wrapping_sub(patlen as u32));
                                    }
                                }
                            }
                        }
                    } else if count == 1 || pos >= last {
                        let boundary = if count == 1 {
                            m.word(entries + 12).wrapping_add(m.word(entries + 16)) >> 1
                        } else {
                            fn_265be4(
                                m,
                                entries,
                                count,
                                pos,
                                half(m, table + 8).wrapping_sub(1) as u32,
                                1,
                            )
                        };
                        marker(
                            m,
                            slot,
                            val,
                            boundary,
                            m.word(entries + 20).wrapping_add(pos),
                            float_bits,
                        );
                    }
                    removed = removed.wrapping_add(1);
                    m.put_half(table + 8, half(m, table + 8).wrapping_add(1));
                    m.put_word(entries + 24, textlen as u32);
                    break;
                }
                if m.byte(text + i as u32) != m.byte(pattern + j as u32) {
                    matched = false;
                }
                j = j.wrapping_add(1);
                i = i.wrapping_add(1);
            }
        }
        i = i.wrapping_add(1);
    }
    fn_26a6e4(m, text, pattern, patlen as u32, &mut textlen, 0);
    m.put_word(entries + 24, textlen as u32);
    rc
}
fn marker(m: &mut Memory<'_>, ne: u32, val: u32, boundary: u32, pos: u32, float_bits: u32) {
    m.put_half(ne + 10, val as u16);
    byte(m, ne, 1);
    m.put_word(ne + 12, boundary);
    m.put_word(ne + 16, 0);
    m.put_word(ne + 20, pos);
    m.put_word(ne + 24, 0);
    m.put_half(ne + 8, 0);
    m.put_word(ne + 4, 0);
    m.put_word(ne + 28, float_bits);
    m.put_word(ne + 32, 0);
}

pub enum RenderArgument<'a> {
    Word(u32),
    Guest(u32),
    Local(&'a [u8]),
}

pub trait RenderLog: SequenceHost {
    fn log_target(
        &mut self,
        m: &mut Memory<'_>,
        method: u32,
        object: u32,
        category: u32,
        level: u32,
        arg: u32,
        format: u32,
        args: &[RenderArgument<'_>],
    ) -> u32;
}

pub struct RenderEnvironment {
    pub category: u32,
    pub marker_format: u32,
    pub header_format: u32,
    pub offsets_format: u32,
    pub text_format: u32,
    pub marker_names: u32,
    pub tag_names: u32,
    pub special_tag_name: u32,
}
fn render_log(m:&mut Memory<'_>,ctx:u32,e:&RenderEnvironment,fmt:u32,
    args:&[RenderArgument<'_>],h:&mut impl RenderLog) {
    render_log_at_level(m,ctx,e,fmt,5,args,h);
}
fn render_log_at_level(
    m: &mut Memory<'_>,
    ctx: u32,
    e: &RenderEnvironment,
    fmt: u32,
    level: u32,
    args: &[RenderArgument<'_>],
    h: &mut impl RenderLog,
) {
    let logger = m.word(ctx + 16);

    let mut view = [0u8; 128];
    let p = if logger == 0 { 0 } else { 64 };
    if logger != 0 {
        view[64 + 0x30] = m.byte(logger + 0x30);
        view[64 + 0x31] = m.byte(logger + 0x31);
    }
    let rc = crate::normalization::log_fanout(&mut view, p, e.category, level, 0, fmt);
    if logger == 0 || rc >= 0x80000000 || level > view[64 + 0x30] as u32 || (view[64 + 0x31] != 0 && level > 4) || fmt == 0 {
        return;
    }
    for i in 0..6 {
        let slot = if i == 5 { 0x34 } else { 8 + i * 8 };
        let methods = m.word(logger + slot + 4);
        if methods != 0 && i != 1 {
            let method = m.word(methods + 24);
            if method != 0 {
                h.log_target(
                    m,
                    method,
                    m.word(logger + slot),
                    e.category,
                    level,
                    0,
                    fmt,
                    args,
                );
            }
        }
    }
}
fn render_alloc(m: &Memory<'_>, heap: u32, n: u32, h: &mut impl SequenceHost) -> u32 {
    crate::frontend::call_method(m.bytes, nz(heap), 1, n, h)
}
fn render_free(m: &Memory<'_>, heap: u32, p: u32, h: &mut impl SequenceHost) {
    crate::frontend::heap_release(m.bytes, nz(heap), nz(p), h);
}
fn narrow(m: &mut Memory<'_>, d: u32, s: u32, n: u32) {
    crate::fe_text::narrow_n_17649c(m.bytes, d, s, n);
}

pub fn fn_263384(
    m: &mut Memory<'_>,
    mode: i32,
    ctx: u32,
    seq: u32,
    tag: u32,
    label: u32,
    e: &RenderEnvironment,
    h: &mut impl RenderLog,
) -> u32 {
    if mode == 0 {
        return 0;
    }
    let mut element = 0;
    let rc = fetch(m, seq, tag, 1, &mut element, h);
    if rc >= 0x80000000 {
        return rc;
    }
    let len = half(m, element + 8) as u32;
    let mut buf = 0;
    if len != 0 {
        let n = if len > 0x3ff { 0x400 } else { len };
        buf = render_alloc(m, m.word(ctx + 4), n * 2 + 2, h);
        if buf == 0 {
            return 0x8110200a;
        }
        narrow(m, buf, m.word(element + 4), n);
        byte(m, buf + n, 0);
    }
    use RenderArgument::{Guest, Local, Word};
    let display = if buf != 0 { Guest(buf) } else { Local(b"") };
    render_log(
        m,
        ctx,
        e,
        e.text_format,
        &[Guest(label), Word(tag), Word(len), display],
        h,
    );
    if buf != 0 {
        render_free(m, m.word(ctx + 4), buf, h);
    }
    rc
}

pub fn fn_2634c0(
    m: &mut Memory<'_>,
    mode: i32,
    ctx: u32,
    seq: u32,
    tag: u32,
    label: u32,
    p6: i32,
    marks: u32,
    count: u32,
    e: &RenderEnvironment,
    h: &mut impl RenderLog,
) -> u32 {
    if mode == 0 {
        return 0;
    }
    use RenderArgument::{Guest, Local, Word};
    render_log(m, ctx, e, e.header_format, &[Guest(label), Word(tag)], h);
    let mut rc = 0;
    let (entries, count) = if marks != 0 {
        (marks, count)
    } else {
        let mut element = 0;
        rc = fetch(m, seq, tag, 1, &mut element, h);
        (m.word(element + 4), half(m, element + 8) as u32)
    };
    let mut xbuf = 0;
    let mut i = 0u16;
    while count > i as u32 {
        let at = entries + i as u32 * if marks == 0 { 40 } else { 24 };
        let mut buf = [0u8; 96];
        let (kind, mtag, refs, ext) = if marks != 0 {
            (
                half(m, at),
                m.word(at + 20),
                [
                    m.word(at + 4),
                    m.word(at + 8),
                    m.word(at + 12),
                    m.word(at + 16),
                ],
                0,
            )
        } else {
            let text = m.word(at + 32);
            if text != 0 {
                let n = (crate::fe_text::strlen16_176624(m.bytes, text) as u16 as u32).min(31);
                for j in 0..n {
                    let v = half(m, text + j * 2).to_le_bytes();
                    buf[32 + j as usize * 2..34 + j as usize * 2].copy_from_slice(&v);
                }
                crate::fe_text::narrow_n_17649c(&mut buf, 0, 32, n);
                buf[n as usize] = 0;
            }
            (
                half(m, at + 10),
                m.word(at + 28),
                [
                    m.word(at + 12),
                    m.word(at + 16),
                    m.word(at + 20),
                    m.word(at + 24),
                ],
                m.byte(at + 36),
            )
        };
        if kind == 0x20 {
            buf[0] = if mtag == 1 { b'1' } else { b'0' };
            buf[1] = 0;
        }
        let mut display = Local(b"");
        let name;
        if kind == 7 {
            name = if mtag & 255 == 0x2b {
                e.special_tag_name
            } else {
                let mut name = m.word(e.tag_names + 4);
                for k in 0..17 {
                    if m.byte(e.tag_names + k * 8) as u32 == mtag & 255 {
                        name = m.word(e.tag_names + k * 8 + 4);
                        break;
                    }
                }
                name
            };
        } else {
            let mut selected = m.word(e.marker_names + 4);
            for k in 0..40 {
                if half(m, e.marker_names + k * 8) == kind {
                    selected = m.word(e.marker_names + k * 8 + 4);
                    break;
                }
            }
            name = selected;
            if kind != 6
                && kind != 0xc
                && matches!(kind, 0x15 | 0x24 | 0x27 | 0x25 | 0x22)
                && p6 == 1
                && mtag != 0
            {
                let n = (crate::fe_text::strlen16_176624(m.bytes, mtag) as u16 as u32).min(0x400);
                let heap = m.word(ctx + 4);
                if xbuf == 0 {
                    xbuf = render_alloc(m, heap, n * 2 + 1, h);
                    if xbuf == 0 {
                        return 0x8110200a;
                    }
                } else if n > crate::fe_text::strlen_195d90(m.bytes, xbuf) as u32 {
                    xbuf = crate::frontend::heap_realloc(m.bytes, nz(heap), xbuf, n * 2 + 1, h);
                    if xbuf == 0 {
                        return 0x8110200a;
                    }
                }
                narrow(m, xbuf, mtag, n);
                byte(m, xbuf + n, 0);
                display = Guest(xbuf);
            }
        }
        let pre = if marks == 0 && m.byte(at) == 1 {
            &b"[INT] "[..]
        } else {
            &b""[..]
        };
        let n = buf[..32].iter().position(|b| *b == 0).unwrap_or(32);
        render_log(
            m,
            ctx,
            e,
            e.marker_format,
            &[
                Word(i as u32),
                Local(pre),
                Word(kind as u32),
                Guest(name),
                Word(refs[0]),
                Word(refs[1]),
                Word(refs[2]),
                Word(refs[3]),
                display,
                Word(ext as u32),
                Local(&buf[..n]),
            ],
            h,
        );
        if marks == 0 {
            let mut j = 0u16;
            while j < half(m, at + 8) {
                let p = m.word(at + 4) + j as u32 * 8;
                render_log(
                    m,
                    ctx,
                    e,
                    e.offsets_format,
                    &[Word(j as u32), Word(m.word(p)), Word(m.word(p + 4))],
                    h,
                );
                j = j.wrapping_add(1);
            }
        }
        i = i.wrapping_add(1);
    }
    if xbuf != 0 {
        render_free(m, m.word(ctx + 4), xbuf, h);
    }
    rc
}

pub struct RewriteEnvironment {

    pub patterns: u32,
    pub wide0: u32,
    pub wide1: u32,
    pub category: u32,
    pub empty: u32,
    pub space: u32,
}
fn add_word(m: &mut Memory<'_>, p: u32, v: u32) {
    m.put_word(p, m.word(p).wrapping_add(v));
}
fn rewrite_error(
    m: &mut Memory<'_>,
    ctx: u32,
    code: u32,
    e: &RewriteEnvironment,
    scratch: u32,
    h: &mut impl crate::boot_part_b::FormatHost,
) {
    crate::render_part_a::error_log(
        m,
        h,
        m.word(ctx + 16),
        e.category,
        code,
        0,
        scratch,
        scratch + 16,
    );
}

pub fn fn_2663b0(
    m: &mut Memory<'_>,
    ctx: u32,
    seq: u32,
    ptext: u32,
    mut markers: u32,
    mut textbuf: u32,
    e: &RewriteEnvironment,
    scratch: u32,
    h: &mut (impl SequenceHost + crate::boot_part_b::FormatHost),
) -> u32 {
    let sl = strlen(m, e.patterns + 11);
    let mut text = m.word(ptext);
    m.put_word(scratch, 0x1e);
    let mut rc;
    let mut need = (crate::fe_text::strlen16_176624(m.bytes, e.wide0) as u32)
        .wrapping_add(1)
        .wrapping_add(half(m, textbuf + 8) as u32) as u16;
    if need > half(m, textbuf + 10) {
        rc = fetch(m, seq, 0x3f4, need as u32, &mut textbuf, h);
        if rc >= 0x80000000 {
            return rc;
        }
    }
    let dst = m.word(textbuf + 4) + half(m, textbuf + 8) as u32 * half(m, textbuf + 2) as u32;
    crate::fe_text::strcpy16_176214(m.bytes, dst, e.wide0);
    m.put_half(
        dst + crate::fe_text::strlen16_176624(m.bytes, e.wide0) as u32 * 2,
        0,
    );
    let n0 = half(m, textbuf + 8);
    m.put_half(textbuf + 8, need);
    need = need
        .wrapping_add(crate::fe_text::strlen16_176624(m.bytes, e.wide1) as u16)
        .wrapping_add(1);
    if need > half(m, textbuf + 10) {
        rc = fetch(m, seq, 0x3f4, need as u32, &mut textbuf, h);
        if rc >= 0x80000000 {
            return rc;
        }
    }
    let dst = m.word(textbuf + 4) + half(m, textbuf + 8) as u32 * half(m, textbuf + 2) as u32;
    crate::fe_text::strcpy16_176214(m.bytes, dst, e.wide1);
    m.put_half(
        dst + crate::fe_text::strlen16_176624(m.bytes, e.wide1) as u32 * 2,
        0,
    );
    let n1 = half(m, textbuf + 8);
    m.put_half(textbuf + 8, need);
    for (c, n) in [(0x4c, n0), (0x44, n0), (0x50, n1)] {
        byte(m, scratch + 1, c);
        rc = fn_265c94(m, seq, text, scratch, markers, 0x24, n as u32, h);
        if rc >= 0x80000000 {
            return rc;
        }
    }
    fn_266034(m, markers, text, e.patterns + 14);
    fn_266034(m, markers, text, e.patterns + 2);
    fn_2661a4(m, markers, text, e.patterns + 8, e.patterns, 0);
    fn_2661a4(m, markers, text, e.patterns, e.empty, 1);
    fn_2661a4(m, markers, text, e.patterns, e.space, 0);
    fn_266034(m, markers, text, e.patterns + 4);
    let mut i = 1u16;
    let mut saw43 = false;
    if half(m, markers + 8) > 1 {
        let b = m.word(markers + 4);
        let first = m.word(b + 20);
        let mut k = 0;
        loop {
            let at = b + i as u32 * 40;
            let st = m.word(at + 20);
            if st == first {
                k = half(m, at + 10);
            }
            if st == first && k == 7 && m.word(at + 28) == 0x2b {
                saw43 = true;
            }
            if st > first {
                break;
            }
            i = i.wrapping_add(1);
            if i >= half(m, markers + 8) {
                break;
            }
        }
    }
    let tlen = strlen(m, text);
    if tlen > 1 && m.byte(text) == b' ' && !saw43 {
        let b = m.word(markers + 4);
        move_bytes(m, text, text + 1, tlen as u32);
        let mut split = (!half(m, markers + 8) & 1) + 1;
        let mut j = 1u16;
        while j < split {
            let at = b + j as u32 * 40;
            j = j.wrapping_add(1);
            if m.word(at + 20).wrapping_sub(m.word(b + 20)) > 1 {
                add_word(m, at + 20, u32::MAX);
            }
        }
        while split < half(m, markers + 8) {
            let at = b + split as u32 * 40;
            split = split.wrapping_add(2);
            if m.word(at + 20).wrapping_sub(m.word(b + 20)) > 1 {
                add_word(m, at + 20, u32::MAX);
            }
            if m.word(at + 60).wrapping_sub(m.word(b + 20)) > 1 {
                add_word(m, at + 60, u32::MAX);
            }
        }
        add_word(m, b + 24, u32::MAX);
    }
    i = 1;
    let mut base = m.word(markers + 4);
    while i < half(m, markers + 8) {
        let mut at = base + i as u32 * 40;
        if half(m, at + 10) == 7 && m.word(at + 28) == 0x2b {
            let mut elen = strlen(m, text) as u32;
            let mut saved = n1 as u32;
            let limit = m.word(at + 24);
            if (m.word(at + 20) < m.word(base + 20) || limit == 0)
                || (strlen(m, text) as u32) < limit
            {
                let p = m.word(at + 32);
                if p != 0 {
                    render_free(m, m.word(ctx + 4), p, h);
                    m.put_word(at + 32, 0);
                }
                rewrite_error(m, ctx, 0x782, e, scratch, h);
                return 0;
            }
            let mut lead = 0u16;
            let mut trail = limit as u16;
            let off = m.word(at + 20).wrapping_sub(m.word(base + 20));
            let mut words = 0u16;
            let mut k = 0u16;
            while k < limit as u16
                && crate::fe_text::strchr_195d30(
                    m.bytes,
                    e.patterns + 18,
                    m.byte(text + off + k as u32) as u32,
                ) != 0
            {
                k = k.wrapping_add(1);
                lead = lead.wrapping_add(1);
            }
            while trail != 0
                && crate::fe_text::strchr_195d30(
                    m.bytes,
                    e.patterns + 18,
                    m.byte(text + off + trail as u32 - 1) as u32,
                ) != 0
            {
                trail = trail.wrapping_sub(1);
            }
            if lead < trail {
                let mut p = lead;
                let mut last = b'_';
                loop {
                    let c = m.byte(text + off + p as u32);
                    if c == b'\\' {
                        if p as u32 + 1 < trail as u32 && m.byte(text + off + p as u32 + 1) == b'T'
                        {
                            let mut q = p.wrapping_add(2);
                            while q < trail && m.byte(text + off + q as u32) != b'\\' {
                                q = q.wrapping_add(1);
                            }
                            if m.byte(text + off + q as u32) == b'\\' {
                                p = q;
                            }
                        }
                    } else if (c == b'_' || c == b'#') && last != b'_' && last != b'#' {
                        words = words.wrapping_add(1);
                    }
                    if crate::fe_text::strchr_195d30(
                        m.bytes,
                        e.patterns + 18,
                        m.byte(text + off + p as u32) as u32,
                    ) == 0
                    {
                        last = m.byte(text + off + p as u32);
                    }
                    p = p.wrapping_add(1);
                    if p >= trail {
                        break;
                    }
                }
                if last != b'_' && last != b'#' {
                    words = words.wrapping_add(1);
                }
            }
            if words == 0 || trail <= lead {
                rewrite_error(m, ctx, 0x71a, e, scratch, h);
                let p = m.word(at + 32);
                if p != 0 {
                    render_free(m, m.word(ctx + 4), p, h);
                    m.put_word(at + 32, 0);
                }
                let count = half(m, markers + 8);
                if count as u32 != i as u32 + 1 {
                    move_bytes(m, at, at + 40, (count as u32 - i as u32) * 40);
                }
                m.put_half(markers + 8, count.wrapping_sub(1));
            } else {
                m.put_half(at + 10, 0x22);
                let add = trail
                    .wrapping_sub(lead)
                    .wrapping_add(1)
                    .wrapping_add(half(m, textbuf + 8));
                if add > half(m, textbuf + 10) {
                    rc = fetch(m, seq, 0x3f4, add as u32, &mut textbuf, h);
                    if rc >= 0x80000000 {
                        return rc;
                    }
                    base = m.word(markers + 4);
                    at = base + i as u32 * 40;
                }
                let dst =
                    m.word(textbuf + 4) + half(m, textbuf + 8) as u32 * half(m, textbuf + 2) as u32;
                let span = trail.wrapping_sub(lead) as u32;
                fn_176354(m, dst, text + off + lead as u32, span);
                m.put_half(dst + span * 2, 0);
                m.put_word(at + 28, half(m, textbuf + 8) as u32);
                m.put_half(textbuf + 8, add);
                let room = m.word(at + 24) & 0xffff;
                let want = words as u32 * sl as u32;
                if want != room && room <= want {
                    let nl = (strlen(m, text) as u32)
                        .wrapping_add(want - room)
                        .wrapping_add(1);
                    let np =
                        crate::frontend::heap_realloc(m.bytes, nz(m.word(ctx + 4)), text, nl, h);
                    if np == 0 {
                        return 0x8110200a;
                    }
                    text = np;
                    m.put_word(ptext, np);
                }
                if off < m.word(base + 24) {
                    let mv = m.word(at + 24) & 0xffff;
                    move_bytes(
                        m,
                        text + off,
                        text + off + mv,
                        elen.wrapping_sub(off).wrapping_sub(mv).wrapping_add(1),
                    );
                }
                for _ in 0..words {
                    let nl2 = strlen(m, text) as u32;
                    move_bytes(m, text + off + sl as u32, text + off, nl2.wrapping_sub(off));
                    byte(m, text + nl2 + sl as u32, 0);
                    move_bytes(m, text + off, e.patterns + 11, sl as u32);
                }
                let mut q = i.wrapping_add(1);
                while q < half(m, markers + 8) {
                    let f = base + q as u32 * 40;
                    q = q.wrapping_add(1);
                    if m.word(f + 20) > m.word(at + 20) {
                        let delta = m.word(at + 24).wrapping_sub(words as u32 * sl as u32);
                        m.put_word(f + 20, m.word(f + 20).wrapping_sub(delta));
                    }
                }
                m.put_word(at + 24, 0);
                let target = m.word(at + 20).wrapping_add(words as u32 * sl as u32);
                let cnt = half(m, markers + 8) as u32;
                let mut exists = false;
                let mut fidx = 1;
                for qq in 1..cnt {
                    let f = base + qq * 40;
                    if m.word(f + 20) != target {
                        if half(m, f + 10) == 0x15 {
                            fidx = qq;
                        }
                    } else if half(m, f + 10) == 0x15 {
                        exists = true;
                        break;
                    }
                }
                if !exists {
                    if cnt > 1 {
                        let b0 = m.word(base + 20);
                        let mut qq = 1u16;
                        loop {
                            let f = base + qq as u32 * 40;
                            if half(m, f + 10) == 0x15 {
                                saved = m.word(f + 28);
                            }
                            if m.word(f + 20).wrapping_sub(b0) > target {
                                break;
                            }
                            qq = qq.wrapping_add(1);
                            if qq as u32 >= cnt {
                                break;
                            }
                        }
                        fidx = qq as u32;
                    }
                    if cnt != fidx {
                        let mut q = fidx;
                        while q < cnt {
                            let f = base + q * 40;
                            if m.word(f + 20).wrapping_sub(m.word(base + 20)) != target {
                                break;
                            }
                            if half(m, f + 10) == 0x15 {
                                exists = true;
                                break;
                            }
                            q += 1;
                        }
                        if exists {
                            i = i.wrapping_add(1);
                            continue;
                        }
                        let nb = fn_265be4(
                            m,
                            base,
                            cnt,
                            target.wrapping_sub(m.word(base + 20)),
                            fidx,
                            0,
                        );
                        rc = fetch(
                            m,
                            seq,
                            0x3fd,
                            half(m, markers + 8).wrapping_add(1) as u32,
                            &mut markers,
                            h,
                        );
                        if rc >= 0x80000000 {
                            return rc;
                        }
                        base = m.word(markers + 4);
                        let at2 = base + fidx * 40;
                        move_bytes(
                            m,
                            at2 + 40,
                            at2,
                            (half(m, markers + 8) as u32).wrapping_sub(fidx) * 40,
                        );
                        marker(m, m.word(markers + 4) + fidx * 40, 0x15, nb, target, saved);
                        m.put_half(markers + 8, half(m, markers + 8).wrapping_add(1));
                    } else {
                        rc = fetch(
                            m,
                            seq,
                            0x3fd,
                            cnt.wrapping_add(1) as u16 as u32,
                            &mut markers,
                            h,
                        );
                        if rc >= 0x80000000 {
                            return rc;
                        }
                        base = m.word(markers + 4);
                        let ne = base + half(m, markers + 8) as u32 * 40;
                        let nb = if fidx == 1 {
                            m.word(base + 12).wrapping_add(m.word(base + 16)) >> 1
                        } else {
                            fn_265be4(
                                m,
                                base,
                                half(m, markers + 8) as u32,
                                target.wrapping_sub(m.word(base + 20)),
                                (fidx as u16).wrapping_sub(1) as u32,
                                0,
                            )
                        };
                        marker(m, ne, 0x15, nb, target, saved);
                        m.put_half(markers + 8, half(m, markers + 8).wrapping_add(1));
                    }
                }
            }
        }
        i = i.wrapping_add(1);
    }
    m.put_word(base + 24, strlen(m, text) as u32);
    0
}

pub fn fn_1740e4(m: &mut Memory<'_>, heap: u32, b: u32, needed: u32,
    h: &mut impl crate::boot_part_e::Host) -> u32 {
    let old = m.word(b);
    let mut grown = false;
    while m.word(b + 4) <= (fe_text::strlen_195d90(m.bytes, m.word(b) - m.base) as u32).wrapping_add(needed) {
        let p = crate::boot_part_e::fn_1963e0(m, heap, m.word(b), m.word(b + 4).wrapping_add(0x1000), h);
        m.put_word(b, p);
        if b == 0 { return 0x84b0200a; }
        m.put_word(b + 4, m.word(b + 4).wrapping_add(0x1000));
        grown = true;
    }
    if grown && old != 0 {
        for i in 0..m.word(b + 8) {
            for off in [12,16] {
                let at = m.word(b + off).wrapping_add(i.wrapping_mul(4));
                let x = m.word(at);
                if x == 0 || x <= old { return 0x84b02007; }
                m.put_word(at, x.wrapping_sub(old).wrapping_add(m.word(b)));
            }
        }
    }
    0
}

fn format_cleanup(m: &mut Memory<'_>, heap: u32, st: u32, h: &mut impl crate::boot_part_e::Host) {
    for off in [0,12,16] {
        let p = m.word(st+off);
        if p != 0 {
            crate::boot_part_e::release(m,heap,p,h);
            m.put_word(st+off,0);
        }
    }
}

pub fn fn_1741dc(m: &mut Memory<'_>, a: u32, b: u32, fmt: u32, mut va: u32,
    scratch: u32, h: &mut impl crate::boot_part_e::Host) -> u32 {
    if a == 0 { return 0; }
    let st=scratch; let num=scratch+32; let empty=scratch+68;
    for off in (0..20).step_by(4) {m.put_word(st+off,0);}
    m.put_word(empty,0);
    let heap=m.word(a);
    let mut ret=0;
    if fmt != 0 {
        let mut i=0u32; let mut npct=0u32;
        let mut valid=true;
        while m.byte(fmt+i)!=0 {
            match m.byte(fmt+i) {
                b'%' => {
                    i+=1;npct+=1;
                    let c=m.byte(fmt+i);
                    if c==b'.' {
                        if m.byte(fmt+i+1)!=b'*' || !matches!(m.byte(fmt+i+2),b's'|b'S') { valid=false;break; }
                        i+=3;
                    } else if matches!(c,b's'|b'S'|b'd'|b'u') {i+=1;}
                    else {valid=false;break;}
                }
                b' ' => i+=1,
                _ => {valid=false;break;}
            }
        }
        if !valid || npct&1!=0 {return 0x84b02007;}
        let base=crate::boot_part_e::alloc(m,heap,0x1000,h);
        m.put_word(st,base);
        if base==0 {return 0x84b0200a;}
        byte(m,base,0);m.put_word(st+4,0x1000);
        if npct>1 {
            for off in [12,16] {
                let p=crate::boot_part_e::calloc(m,heap,npct>>1,4,h);
                m.put_word(st+off,p);
                if p==0 {
                    format_cleanup(m,heap,st,h);
                    return 0x84b0200a;
                }
            }
        }
        i=0;let mut sl=0u32;
        while m.byte(fmt+i)!=0 {
            if m.byte(fmt+i)==b' ' {i+=1;continue;}
            if m.byte(fmt+i)!=b'%' {ret=0x84b02007;break;}
            let mut j=i+1;let mut prec=u32::MAX;
            if m.byte(fmt+j)==b'.' {prec=m.word(va);va+=4;j+=2;}
            let c=m.byte(fmt+j);
            let arg=m.word(va);va+=4;
            let seg;
            if c==b's' {
                let arg=if arg==0 {empty} else {arg};
                let need=if prec==u32::MAX {fe_text::strlen_195d90(m.bytes,arg-m.base) as u32} else {prec};
                ret=fn_1740e4(m,heap,st,need.wrapping_add(2),h);
                if ret>=0x80000000 {break;}
                let base=m.word(st);
                seg=base+fe_text::strlen_195d90(m.bytes,base-m.base) as u32;
                fe_text::strncat_195dac(m.bytes,base-m.base,arg-m.base,need);
                let end=base+fe_text::strlen_195d90(m.bytes,base-m.base) as u32;
                byte(m,end,b' ');byte(m,end+1,0);
            } else if c==b'S' {
                let arg=if arg==0 {empty} else {arg};
                let wide=if prec==u32::MAX {arg} else {
                    let p=crate::boot_part_e::calloc(m,heap,prec.wrapping_add(1),2,h);
                    if p==0 {ret=0x84b0200a;break;}
                    fe_text::strncpy16_176644(m.bytes,p-m.base,arg-m.base,prec);p
                };
                let need=crate::normalization::utf8_len_u16(m.bytes,wide-m.base);
                ret=fn_1740e4(m,heap,st,need.wrapping_add(2),h);
                if ret<0x80000000 {
                    let base=m.word(st);
                    seg=base+fe_text::strlen_195d90(m.bytes,base-m.base) as u32;
                    let n=fe_text::strlen16_176624(m.bytes,wide-m.base) as u32+1;
                    crate::normalization::transcode_u16(m.bytes,wide-m.base,n,0,seg-m.base,u32::MAX,scratch+80-m.base,0);
                    let end=base+fe_text::strlen_195d90(m.bytes,base-m.base) as u32;
                    byte(m,end,b' ');byte(m,end+1,0);
                } else {
                    if prec!=u32::MAX {crate::boot_part_e::release(m,heap,wide,h);}
                    break;
                }
                if prec!=u32::MAX {crate::boot_part_e::release(m,heap,wide,h);}
            } else if c==b'd' || c==b'u' {
                if c==b'd' {crate::normalization::itoa(m.bytes,arg,num-m.base,10);}
                else {crate::normalization::udiv_itoa(m.bytes,arg,num-m.base,10);}
                let len=fe_text::strlen_195d90(m.bytes,num-m.base) as u32;
                ret=fn_1740e4(m,heap,st,len+2,h);
                if ret>=0x80000000 {break;}
                let base=m.word(st);
                seg=base+fe_text::strlen_195d90(m.bytes,base-m.base) as u32;
                fe_text::strcat_195c90(m.bytes,base-m.base,num-m.base);
                let end=base+fe_text::strlen_195d90(m.bytes,base-m.base) as u32;
                byte(m,end,b' ');byte(m,end+1,0);
            } else {ret=0x84b02007;break;}
            let table=m.word(st+if sl&1==0 {12} else {16});
            m.put_word(table+(sl>>1)*4,seg);
            if sl&1!=0 {m.put_word(st+8,m.word(st+8)+1);}
            sl+=1;i=j+1;
        }
        if ret<0x80000000 {
            let count=m.word(st+8);
            if count!=0 {
                for k in 0..count {
                    for off in [12,16] {
                        let p=m.word(m.word(st+off)+k*4);
                        if p!=0 && p.wrapping_sub(1)>m.word(st) && m.byte(p-1)==b' ' {byte(m,p-1,0);}
                    }
                }
                let last=m.word(m.word(st+16)+(count-1)*4);
                if last!=0 {
                    let len=fe_text::strlen_195d90(m.bytes,last-m.base) as u32;
                    if m.byte(last.wrapping_add(len).wrapping_sub(1))==b' ' {
                        let len=fe_text::strlen_195d90(m.bytes,last-m.base) as u32;
                        byte(m,last.wrapping_add(len).wrapping_sub(1),0);
                    }
                }
            }
        }
    }
    if ret<0x80000000 {
        for k in 0..6 {
            let off=if k==5 {52} else {8+k*8};
            let tab=m.word(a+off+4);
            if tab!=0 {
                let method=m.word(tab+48);
                if method!=0 {
                    let rc=h.call(m,method,&[m.word(a+off),b,m.word(st+8),m.word(st+12),m.word(st+16)]);
                    if rc>=0x80000000 {ret=rc;}
                }
            }
        }
    }
    format_cleanup(m,heap,st,h);
    ret
}

pub struct LifecycleEnvironment {
    pub engine: crate::render_part_c::Environment,
    pub parameter_pairs: u32,
    pub parameter_triple: u32,
}

pub trait LifecycleHost: ParameterHost + crate::frontend::MethodTable + crate::render_part_b2::CleanupHost {
    fn cleanup_environment(&self) -> crate::render_part_b2::CleanupEnvironment;
}
pub(crate) fn lifecycle_context(m: &Memory<'_>, p: u32, e: &LifecycleEnvironment) -> (u32,u32) {
    let a=m.word(p);let b=m.word(p+4);
    let cached=[u32::from_le_bytes(e.engine.cached_handle[..4].try_into().unwrap()),
        u32::from_le_bytes(e.engine.cached_handle[4..].try_into().unwrap())];
    let word=if [a,b]!=cached && a!=0 && b==0x1e61 {Some(half(m,a))} else {None};
    let mut out=0;
    let rc=crate::frontend::resolve_engine_context(a,word,b,&e.engine.cached_handle,e.engine.default_context,&mut out);
    (rc,out)
}
struct LifecycleCalls<'a,H> { host: &'a mut H, scratch:u32 }
impl<H:LifecycleHost> crate::frontend::HeapVtable for LifecycleCalls<'_,H> {
    fn alloc_at(&mut self,o:u32,n:u32)->u32 {self.host.alloc_at(o,n)}
    fn realloc_at(&mut self,o:u32,p:u32,n:u32)->u32 {self.host.realloc_at(o,p,n)}
    fn release_at(&mut self,o:u32,p:u32) {self.host.release_at(o,p)}
}
impl<H:LifecycleHost> crate::frontend::MethodTable for LifecycleCalls<'_,H> {
    fn call_at(&mut self,o:u32,n:u32,s:u32)->u32 {self.host.call_at(o,n,s)}
}
impl<H:LifecycleHost> crate::render_part_c::Host for LifecycleCalls<'_,H> {
    fn call(&mut self,m:&mut Memory<'_>,entry:u32,a:&[u32])->u32 {
        if entry==0x175a24 {fn_175a24(m,a[0],a[1],self.scratch,self.host)}
        else {self.host.invoke(entry,a)}
    }
    fn output(&mut self,_:&mut Memory<'_>,entry:u32,_:&[u32])->(u32,u32) {
        panic!("unrequested output method {entry:x}")
    }
}
struct LifecyclePcre<'a,H> {host:&'a mut H}
impl<H:LifecycleHost> crate::regex::PcreHost for LifecyclePcre<'_,H> {
    fn validate(&mut self,o:u32,k:u32,t:u32,s:u32)->u32 {crate::frontend::validate_handle(o!=0,k,t,s)}
    fn resolve_owner(&mut self,_:u32,_:i32)->(u32,u32) {panic!("deinit cannot resolve owner")}
    fn heap_of(&mut self,_:u32)->u32 {panic!("deinit cannot query heap")}
    fn alloc(&mut self,_:u32,_:u32,_:u32)->Option<u32> {panic!("deinit cannot allocate")}
    fn free(&mut self,heap:u32,p:u32) {

        if heap!=0 && p!=0 {self.host.release_at(heap,p);}
    }
}
fn lifecycle_pcre(m:&mut Memory<'_>,o:u32,key:u32,blocks:&mut [crate::regex::FrameBlockState],h:&mut impl LifecycleHost) {
    let rc=crate::frontend::validate_handle(o!=0,key,0xdd57,24);
    if rc>=0x80000000 {
        let mut p=LifecyclePcre{host:h};
        crate::regex::pcre_close(&mut p,0,o,key);
        return;
    }
    let heap=m.word(m.word(o)+4);
    let object=if heap==0 {0} else {m.word(heap+4)};
    let table=m.word(o+4);let count=half(m,o+8);
    assert!(blocks.len()>=count as usize);
    for i in 0..count as usize {
        let p=m.word(table+i as u32*4);
        blocks[i]=crate::regex::FrameBlockState {present:p!=0,live:p!=0,entry_id:p,
            code_id:if p==0 {0} else {m.word(p)},
            extra_id:if p==0 {0} else {m.word(p+12)},..Default::default()};
    }
    let mut p=LifecyclePcre{host:h};
    let mut store=crate::regex::FrameStore {
        frames:&mut [],tags:&mut [],blocks,
        frames_per_block:half(m,o+10),frame_size:half(m,o+12),max_blocks:half(m,o+14),
        block_count:count,current_block:half(m,o+16),highest_block:half(m,o+20),
        max_frames:half(m,o+18),fresh:crate::regex::MatchFrame::default(),heap:object,blocks_id:nz(table),
    };
    crate::regex::pcre_deinit(&mut p,o,key,&mut store);

    for i in 0..count as usize {
        let entry=m.word(table+i as u32*4);
        if entry!=0 {m.put_word(entry,0);m.put_word(entry+12,0);}
        m.put_word(table+i as u32*4,0);
    }
    m.put_half(o+8,store.block_count);
    m.put_half(o+16,store.current_block);
    m.put_half(o+18,store.max_frames);
    m.put_half(o+20,store.highest_block);
    m.put_word(o+4,store.blocks_id.unwrap_or(0));
    crate::regex::pcre_close(&mut p,object,o,key);
}

pub fn fn_263abc(m:&mut Memory<'_>,o:u32,e:&LifecycleEnvironment,scratch:u32,
    blocks:&mut [crate::regex::FrameBlockState],h:&mut impl LifecycleHost)->u32 {
    if o==0 {return 0x81102007;}
    let (rc,t)=lifecycle_context(m,o,e);
    if rc>=0x80000000 {return 0x81102007;}
    if m.word(o+0x7c)!=0 {
        crate::render_part_c::fn_2c6370(m,m.word(o+0x7c),&mut LifecycleCalls{host:h,scratch});
        m.put_word(o+0x7c,0);
    }
    let cleanup=h.cleanup_environment();
    crate::render_part_b2::fn_26a444(m,o,&cleanup,h);
    let pair=e.engine.cached_handle;
    let a=u32::from_le_bytes(pair[..4].try_into().unwrap());
    let b=u32::from_le_bytes(pair[4..].try_into().unwrap());
    if crate::frontend::match_halves(m.word(o+0x44),m.word(o+0x48),a,b)==0 {
        lifecycle_pcre(m,m.word(o+0x44),m.word(o+0x48),blocks,h);
    }
    crate::render_part_c::fn_2b92d8(m,t,e.parameter_pairs,e.parameter_triple,&mut LifecycleCalls{host:h,scratch});
    crate::frontend::heap_release(m.bytes,nz(m.word(t+4)),Some(o),h);
    0
}

pub struct VoiceEnvironment {
    pub log: RenderEnvironment,
    pub log_transcription: u32,
    pub log_dummy: u32,
    pub log_fewer: u32,
    pub whitespace: u32,
    pub separators: u32,
    pub end_transcription: u32,
    pub normal: u32,
    pub stream: crate::render_part_b2::StreamEnvironment,
}
pub trait VoiceHost: RenderLog + crate::boot_part_e::Host + crate::boot_part_b::FormatHost + crate::render_part_b2::StreamHost {}
fn voice_grow(m:&mut Memory<'_>,slot:u32,heap:u32,n:u32,h:&mut impl VoiceHost)->bool {
    let p=crate::boot_part_e::fn_1963e0(m,heap,m.word(slot),n,h);
    if p!=0 {m.put_word(slot,p);}
    p!=0
}
fn voice_insert(m:&mut Memory<'_>,events:u32,owned:u32,count:u32,index:u32,value:u32,kind:u16,pos:u32) {
    let n=m.word(count);let ev=m.word(events);let own=m.word(owned);
    if n>index {
        move_bytes(m,ev+(index+1)*24,ev+index*24,(n-index)*24);
        move_bytes(m,own+index+1,own+index,n-index);
    }
    m.put_word(count,n+1);
    let at=ev+index*24;
    m.put_half(at,kind);m.put_word(at+20,value);m.put_word(at+12,pos);
    m.put_word(at+4,if index==0 {0} else {m.word(at-24+4)});
    m.put_word(at+8,0);m.put_word(at+16,0);byte(m,own+index,1);
}
fn wide_len(m:&Memory<'_>,p:u32)->u32 {fe_text::strlen16_176624(m.bytes,p-m.base) as u32}
fn in_wide_set(m:&Memory<'_>,set:u32,c:u16)->bool {
    fe_text::strchr16_176514(m.bytes,set-m.base,c as u32)!=0
}

pub fn fn_1ef960(m:&mut Memory<'_>,a:[u32;12],e:&VoiceEnvironment,h:&mut impl VoiceHost)->u32 {
    let [p,config,text,start,mut end,events,owned,group,index,count,voice,language]=a;
    let env=m.word(p);let heap=m.word(env+4);let base=m.word(m.word(events)+12);
    m.put_word(config+52,m.word(p+72));
    let rc=crate::render_part_b2::fn_26d330(m,config,m.word(text),start,end,voice,language,&e.stream,h);
    if rc>=0x80000000 {return rc;}
    m.put_word(p+72,m.word(config+52));
    if m.byte(m.word(config+52))==0 {return rc;}
    let result=(||->Result<u32,()> {
        if !voice_grow(m,events,heap,(m.word(count)+2)*24,h) {return Err(());}
        if !voice_grow(m,owned,heap,m.word(count)+2,h) {return Err(());}
        let mut trans=crate::boot_part_e::calloc(m,heap,fe_text::strlen_195d90(m.bytes,m.word(p+72)-m.base) as u32+2,2,h);
        if trans==0 {return Err(());}
        fe_text::strcpy_widen_1763e4(m.bytes,trans-m.base,m.word(p+72)-m.base);
        crate::normalization::append_narrow(m.bytes,trans-m.base,e.end_transcription-m.base);
        render_log_at_level(m,env,&e.log,e.log_transcription,4,&[
            RenderArgument::Guest(m.word(config+72)),RenderArgument::Word(start)],h);
        voice_insert(m,events,owned,count,group,trans,0x22,start+base);
        let after=index+1;
        let mut ph=start/2;
        while in_wide_set(m,e.whitespace,half(m,m.word(text)+ph*2)) {
            if ph>=end/2 {break;} ph+=1;
        }
        let mut words=0;let mut cursor=group;
        if ph<end/2 {
            loop {
                while !in_wide_set(m,e.whitespace,half(m,m.word(text)+ph*2)) {if ph>=end/2 {break;}ph+=1;}
                let tpos=base+ph*2;
                while cursor<m.word(count) && m.word(m.word(events)+cursor*24+12)<tpos {
                    m.put_word(m.word(events)+cursor*24+12,base+start+m.word(m.word(config+720)+words*4)*2);
                    cursor+=1;
                }
                while in_wide_set(m,e.whitespace,half(m,m.word(text)+ph*2)) {if ph>=end/2 {break;}ph+=1;}
                words+=1;
                if ph>=end/2 {break;}
            }
        }
        let end_base=end+base;let mut k=cursor;
        while k<m.word(count) && m.word(m.word(events)+k*24+12)<end_base {
            m.put_word(m.word(events)+k*24+12,end_base);k+=1;
        }
        let ins=(fe_text::strlen_195d90(m.bytes,m.word(config+716)-m.base) as u32*2).wrapping_sub(end).wrapping_add(start+2) as i16 as i32;
        if ins>0 && !voice_grow(m,text,heap,(ins as u32+wide_len(m,m.word(text))*2+2) as u16 as u32,h) {return Err(());}
        if ins!=0 {
            let t=m.word(text);let n=wide_len(m,t);
            move_bytes(m,t+(end/2).wrapping_add((ins/2) as u32).wrapping_mul(2),t+(end&!1),n*2-end+2);
            add_word(m,m.word(events)+16,ins as u32);
            for k in cursor..m.word(count) {
                let at=m.word(events)+k*24+12;
                if m.word(at)>=end_base {add_word(m,at,ins as u32);}
            }
            end=end.wrapping_add(ins as u32);
        }
        let mut i=0;
        while i<(fe_text::strlen_195d90(m.bytes,m.word(config+716)-m.base) as u32) {
            m.put_half(m.word(text)+(start/2+i)*2,m.byte(m.word(config+716)+i) as u16);i+=1;
        }
        m.put_half(m.word(text)+(start/2+i)*2,32);
        while in_wide_set(m,e.separators,half(m,m.word(text)+ph*2)) {ph+=1;}
        let mut end_half=end/2;
        while end_half!=0 && in_wide_set(m,e.separators,half(m,m.word(text)+(end_half-1)*2)) {end_half-=1;}
        k=ph;let mut nph=1u16;
        while k<end_half {
            let c=half(m,m.word(text)+k*2);
            if c==0 || !in_wide_set(m,e.separators,c) {k+=1;continue;}
            nph=nph.wrapping_add(1);
            while k<end_half && in_wide_set(m,e.separators,half(m,m.word(text)+k*2)) {k+=1;}
            k+=1;
        }
        let mut n=1;let mut i=0;let s=m.word(p+72);
        loop {
            let c=m.byte(s+i);
            if c==0 || c==18 {break;}
            if c==95 || (c==35 && m.byte(s+i+1)!=0) {n+=1;}
            i+=1;
        }
        let mut dummies=0;
        if n>nph as u32 {
            dummies=n-nph as u32;
            render_log_at_level(m,env,&e.log,e.log_dummy,5,&[RenderArgument::Word(dummies),RenderArgument::Word(nph as u32+1),RenderArgument::Word(n+1)],h);
            if !voice_grow(m,text,heap,wide_len(m,m.word(text))*2+dummies*4+2,h) {return Err(());}
            let mut pos=end/2;
            loop {
                let ch=half(m,m.word(text)+pos*2);
                if ch!=0 && ch!=32 {
                    let iface=m.word(p+36);let method=m.word(iface+48);
                    if crate::boot_part_e::Host::call(h,m,method,&[m.word(p+28),m.word(p+32),ch as u32&255])==0 {break;}
                }
                pos=pos.wrapping_sub(1);
            }
            pos+=1;
            let t=m.word(text);let len=wide_len(m,t);
            move_bytes(m,t+(pos+dummies*2)*2,t+pos*2,(len-pos)*2+2);
            let mut kk=0;while kk<dummies*2 {m.put_half(t+(pos+kk)*2,32);m.put_half(t+(pos+kk+1)*2,21);kk+=2;}
            end_half+=dummies*4;
            add_word(m,m.word(events)+16,dummies*4);
            for k in group..m.word(count) {
                let at=m.word(events)+k*24+12;
                if m.word(at)>=base+pos*2 {add_word(m,at,dummies*4);}
            }
        } else if n<(nph as u32) {
            render_log_at_level(m,env,&e.log,e.log_fewer,5,&[RenderArgument::Word(nph as u32),RenderArgument::Word(n)],h);
            let mut r=end/2-1;
            while r!=0 && half(m,m.word(text)+r*2)==32 {r-=1;}
            if r!=0 {loop {r-=1;if r==0 || half(m,m.word(text)+r*2)==32 {break;}}}
            while k!=0 && r!=0 {
                while half(m,m.word(text)+r*2)==32 {
                    m.put_half(m.word(text)+r*2,45);r-=1;if r==0 {break;}
                }
                if r==0 {k-=1;break;}
                k-=1;
                loop {r-=1;if r==0 || half(m,m.word(text)+r*2)==32 {break;}}
            }
        }
        let buf=crate::boot_part_e::calloc(m,heap,wide_len(m,language)+1,2,h);
        if buf==0 {return Err(());}
        fe_text::strcpy16_176214(m.bytes,buf-m.base,language-m.base);
        voice_insert(m,events,owned,count,after,buf,0x15,base+end+dummies*4);
        let mut evcur=group;let mut widx=0;
        while widx<(half(m,config+200) as u32) {
            let target=half(m,config+72+widx*2) as u32;
            let grow=if target==0 {1} else {3};
            if !voice_grow(m,events,heap,(m.word(count)+grow)*24,h) {return Err(());}
            if !voice_grow(m,owned,heap,m.word(count)+grow,h) {return Err(());}
            let mut cur=ph;
            if target!=0 {
                let mut nsep=0;
                loop {
                    if in_wide_set(m,e.separators,half(m,m.word(text)+cur*2)) {
                        nsep+=1;
                        while cur<end_half && in_wide_set(m,e.separators,half(m,m.word(text)+cur*2)) {cur+=1;}
                    }
                    cur+=1;if target<=nsep {break;}
                }
            }
            if cur>start {cur-=1;}
            if target!=0 {
                let lim=base+cur*2-2;
                while m.word(m.word(events)+evcur*24+12)<lim {evcur+=1;}
                let buf=crate::boot_part_e::calloc(m,heap,7,2,h);
                if buf==0 {return Err(());}
                fe_text::strcpy_widen_1763e4(m.bytes,buf-m.base,e.normal-m.base);
                voice_insert(m,events,owned,count,evcur,buf,0x15,lim);evcur+=1;
            }
            let lim=base+cur*2;
            while m.word(m.word(events)+evcur*24+12)<lim {evcur+=1;}
            let buf=crate::boot_part_e::calloc(m,heap,4,2,h);
            if buf==0 {return Err(());}
            fe_text::strcpy16_176214(m.bytes,buf-m.base,config+202+widx*8-m.base);
            voice_insert(m,events,owned,count,evcur,buf,0x24,lim);
            let idx4=evcur+1;
            if target!=0 {
                let have=if widx==0 {0} else {half(m,config+72+(widx-1)*2) as u32};
                let mut t=trans;
                if target>have {
                    let mut kk=have;
                    loop {
                        let c=half(m,t)&255;t+=2;
                        if c==95 || c==35 {kk+=1;}
                        if target<=kk {break;}
                    }
                }
                let buf=crate::boot_part_e::calloc(m,heap,wide_len(m,t)+1,2,h);
                if buf==0 {return Err(());}
                fe_text::strcpy16_176214(m.bytes,buf-m.base,t-m.base);
                m.put_half(t-2,20);m.put_half(t,0);
                render_log_at_level(m,env,&e.log,e.log_transcription,4,&[RenderArgument::Guest(m.word(p+72)),RenderArgument::Word(start)],h);
                voice_insert(m,events,owned,count,idx4,buf,0x22,lim);trans=buf;
            }
            evcur=idx4+1;widx+=1;
        }
        Ok(rc)
    })();
    match result {
        Ok(rc)=>rc,
        Err(())=>{
            crate::render_part_a::error_log(m,h,m.word(env+16),e.log.category,60000,0,0,0);
            0x8c90200a
        }
    }
}

pub struct SetupEnvironment<'a> {
    pub lifecycle: &'a LifecycleEnvironment,
    pub resources: &'a crate::render_part_c2::ResourceEnvironment,
    pub conversion: &'a crate::render_part_c2::Environment<'a>,
    pub dictionaries: &'a crate::render_part_d::DictionaryEnvironment,
    pub words: &'a crate::render_part_c::ConstructorData,
    pub parser: &'a crate::render_part_c::ParserConstructorDataM<'a>,
    pub broker_table: u32,
    pub brkd_name: u32,
    pub brkd_callbacks: [u32;2],
    pub user_names: [u32;2],
    pub error_category: u32,
}
pub trait SetupHost: LifecycleHost + SequenceHost + crate::render_part_c2::ResourceHost
    + crate::render_part_d::DictionaryHost + crate::boot_part_b::QueryHost + crate::render_part_c::ParserMethodsM {}

pub fn fn_263c68(m:&mut Memory<'_>,o:u32,flag:u32,e:&SetupEnvironment<'_>,scratch:u32,
    blocks:&mut [crate::regex::FrameBlockState],mut compare:impl FnMut(u32,u32)->i32,
    callback:&mut impl FnMut(u32,u32,&mut [u8;32]),h:&mut impl SetupHost)->u32 {
    let mut saved=[0;512];let at=m.offset(scratch);saved.copy_from_slice(&m.bytes[at..at+512]);
    let result=(|| {
    let handles=[m.word(o),m.word(o+4)];
    let mut rc=crate::render_part_c::fn_2c464c(m,handles,m.word(o+0x44),m.word(o+0x48),o+0x34,m.word(o+0x7c),&e.lifecycle.engine,h);
    if rc>=0x80000000 {return rc;}
    for (i,v) in [e.brkd_name,e.brkd_callbacks[0],e.brkd_callbacks[1],o].into_iter().enumerate() {m.put_word(scratch+i as u32*4,v);}
    rc=crate::render_part_c::fn_2c61a0(m,m.word(o+0x7c),scratch,1,h);
    if rc>=0x80000000 {return rc;}
    rc=crate::render_part_c2::fn_2c5ef4(m,m.word(o+0x7c),e.brkd_name,m.byte(o+0x80) as u32,scratch+32,e.resources,h);
    if rc>=0x80000000 {return rc;}
    if o==0 {return 0x81102007;}
    let (rc,t)=lifecycle_context(m,o,e.lifecycle);
    if rc>=0x80000000 {return 0x81102007;}
    let (rc,t2)=lifecycle_context(m,o,e.lifecycle);
    if rc>=0x80000000 {return 0x81102007;}
    let buf=m.word(o+0x70);
    if buf!=0 {
        let mut i=0u16;
        while (i as u32)<fe_text::strlen_195d90(m.bytes,buf-m.base) as u32 {
            byte(m,buf+i as u32,fe_text::fold_17c8c0(m.byte(buf+i as u32) as u32) as u8);
            i=i.wrapping_add(1);
        }
        let pair=scratch+224;
        let rc=crate::boot_part_b::fn_177e0c(m,m.word(t2+12),buf,1,1,pair,o+0x64,e.broker_table,scratch+240,h);
        if rc<0x80000000 {
            let method=m.word(m.word(o+0x64)+16);
            let rc=crate::boot_part_b::Host::call(h,m,method,&[m.word(pair),m.word(pair+4),handles[0],handles[1],o+0x68]);
            if rc>=0x80000000 {return rc;}
        }
    } else {
        crate::render_part_a::error_log(m,h,m.word(t2+16),e.error_category,0x713,0,0,0);
    }
    if m.byte(o+0x77)!=0 {
        let temp=scratch+256;
        let mut rc={let (rc,entry)=crate::render_part_c2::registry_acquire(m,m.word(t+24),e.user_names[0],temp,h);m.put_word(temp,entry);rc};
        if rc>=0x80000000 {
            byte(m,flag,1);fn_263abc(m,o,e.lifecycle,scratch+288,blocks,h);return rc;
        }
        let u=m.word(temp);
        for i in 0..3 {m.put_word(o+0x58+i*4,m.word(u+4+i*4));}
        rc={let (rc,entry)=crate::render_part_c2::registry_acquire(m,m.word(t+24),e.user_names[1],temp,h);m.put_word(temp,entry);rc};
        if rc>=0x80000000 {
            crate::render_part_c2::registry_release(m,m.word(t+24),e.user_names[0],false,h);
            for off in [0x58,0x5c,0x60] {m.put_word(o+off,0);}
            byte(m,flag,1);fn_263abc(m,o,e.lifecycle,scratch+288,blocks,h);return rc;
        }
        let u=m.word(temp);
        for i in 0..3 {m.put_word(o+0x4c+i*4,m.word(u+4+i*4));}
        rc=crate::render_part_c::fn_2ba304(m,handles,[
            m.word(o+0x44),m.word(o+0x48),o+0x3c,m.word(o+0x58),m.word(o+0x5c),m.word(o+0x60),
            m.word(u+4),m.word(u+8),m.word(u+12),m.word(o+0x7c),m.word(o+0x64),m.word(o+0x68),m.word(o+0x6c)],
            &e.lifecycle.engine,e.words,&mut compare,h);
        if rc>=0x80000000 {
            for (i,off) in [0x58,0x4c].into_iter().enumerate() {
                if m.word(o+off)!=0 {
                    crate::render_part_c2::registry_release(m,m.word(t+24),e.user_names[i],false,h);
                    for j in 0..3 {m.put_word(o+off+j*4,0);}
                }
            }
            byte(m,flag,1);fn_263abc(m,o,e.lifecycle,scratch+288,blocks,h);return rc;
        }
    } else {
        let rc=crate::text_input::alloc_handle_item(m.bytes,handles[0],handles[1],o+0x3c,&mut FetchHost(h));
        if rc>=0x80000000 {return rc;}
    }
    let rc=crate::render_part_c::fn_2bf760_m(m,handles,[
        m.word(o+0x44),m.word(o+0x48),m.byte(o+0x77) as u32,o+0x38,m.word(o+0x7c)],
        &e.lifecycle.engine,e.parser,h);
    if rc>=0x80000000 {return rc;}
    let rc=crate::render_part_c2::fn_2c6194(m,m.word(o+0x7c),m.byte(o+0x80) as u32,scratch+32,e.resources,h);
    if rc>=0x80000000 {return rc;}
    crate::render_part_c2::fn_2c3068(m,handles,o+0x40,e.conversion,e.dictionaries,scratch+32,h)
    })();
    m.bytes[at..at+512].copy_from_slice(&saved);result
}

pub struct BlockEnvironment<'a> {
    pub lifecycle: &'a LifecycleEnvironment,
    pub log: &'a RenderEnvironment,
    pub marker: &'a crate::render_part_b2::Environment,
    pub clear_active: [u16;3],
    pub clear_all: [u16;6],
    pub readmode: u32,
    pub resetlen: u32,
    pub mode_key: u32,
    pub position_key: u32,
    pub labels: [u32;24],
    pub marker_log: u32,
    pub normalizer_defaults: &'a [u8;52],
}

pub trait BlockHost: RenderLog + LifecycleHost + crate::render_part_c::Host
    + crate::render_part_b2::Host + crate::frontend::StateLock
    + crate::boot_part_e::Host {
    fn existing_port(&mut self,m:&mut Memory<'_>,entry:u32,args:&[u32])->u32;
}

mod block_parser_sealed { pub trait Sealed {} }

pub struct BlockParserBridge<'a,'b,'c,H,L,O,B,E,C,A> {
    pub runtime: &'a mut crate::render_part_c2::ParserRuntime<'b,'c,H,L,O,B,E,C>,
    pub allocator: &'a mut A,
}
pub trait BlockParser: block_parser_sealed::Sealed {
    fn parse(&mut self,m:&mut Memory<'_>,args:&[u32])->u32;
}
impl<H,L,O,B,E,C,A> block_parser_sealed::Sealed for BlockParserBridge<'_, '_, '_,H,L,O,B,E,C,A> {}
impl<H,L,O,B,E,C,A> BlockParser for BlockParserBridge<'_, '_, '_,H,L,O,B,E,C,A>
where H:crate::render_part_c2::ResourceHost+ParameterHost,
 L:crate::frontend::StateLock, O:crate::frontend::TreeOpsM,
 B:crate::frontend::BeforeChange,E:crate::frontend::ElementCallback,
 C:FnMut(u32,u32)->i32,A:crate::render_part_c2::ParserAllocatorM {
    fn parse(&mut self,m:&mut Memory<'_>,a:&[u32])->u32 {
        crate::render_part_c2::fn_2c0f74_m(m,a[0],a[1],a[2],a[3],a[4],a[5],a[6],a[7],a[8],a[9],self.runtime,self.allocator)
    }
}

fn block_call(m:&mut Memory<'_>,f:u32,a:&[u32],e:&BlockEnvironment<'_>,scratch:u32,h:&mut impl BlockHost,parser:&mut impl BlockParser)->u32 {
    match f {
        0x2c0f74=>parser.parse(m,a),
        0x252f88=>{
            let mut p=0; let rc=fetch(m,a[0],a[1],a[2],&mut p,h);if rc<0x80000000 {m.put_word(a[3],p);}rc
        }
        0x263384=>fn_263384(m,a[0] as i32,a[1],a[2],a[3],a[4],e.log,h),
        0x2634c0=>fn_2634c0(m,a[0] as i32,a[1],a[2],a[3],a[4],a[5] as i32,a[6],a[7],e.log,h),
        0x2c47d0=>crate::render_part_c::fn_2c47d0(m,a[0],a[1],&e.lifecycle.engine,h),
        0x2c4728=>crate::render_part_c::fn_2c4728(m,a[0],a[1]),
        0x2c4750=>{
            if a[0]==0 {crate::text_input::mark_flag(None)}
            else {let at=m.offset(a[0]);crate::text_input::mark_flag(Some(&mut m.bytes[at..]))}
        }
        0x2c2408=>{
            let at=m.offset(a[0]);crate::text_input::normalize_window(&mut m.bytes[at..])
        }
        0x2c242c=>crate::normalization::triple_zero(m.bytes,a[0]),
        0x2c4768=>crate::normalization::value_fetch(m.bytes,a[0],a[1]),
        0x1756d0=>{
            let mut p=0;
            let rc=crate::frontend::lookup_param(m.bytes,nz(a[0]),a[1],Some(&mut p),None,h,|x,y|crate::normalization::strcmp_at(m.bytes,x,y));
            if a[2]!=0 {m.put_word(a[2],p);}rc
        }
        0x175944=>{
            let mut p=0;let mut size=0;
            let rc=crate::frontend::lookup_param(m.bytes,nz(a[0]),a[1],Some(&mut p),Some(&mut size),h,|x,y|crate::normalization::strcmp_at(m.bytes,x,y));
            let value=if rc<0x80000000 && size>1 {crate::frontend::parse_uint(m.bytes,nz(p)) as u32} else {0};
            m.put_word(a[2],value);rc
        }
        0x17497c=>crate::normalization::flag_byte(m.bytes,a[0]),
        0x17486c=>{
            if crate::normalization::flag_byte(m.bytes,a[0])!=0 {
                fn_1741dc(m,a[0],a[1],a[2],scratch, scratch+64,h)
            } else {0}
        }
        0x26a610=>crate::render_part_b2::fn_26a610(m,a[0],a[1],e.marker,h),
        0x2c3e78=>crate::text_input::split_spans(m.bytes,a[0],a[1],a[2],a[3],scratch,e.normalizer_defaults,&mut FetchHost(h)),
        0x2bb4c8=>crate::render_part_c::fn_2bb4c8(m,a[0],a[1],h),
        _=>h.existing_port(m,f,a),
    }
}
fn block_text(m:&mut Memory<'_>,o:u32,ctx:u32,sp:u32,tag:u32,label:usize,e:&BlockEnvironment<'_>,h:&mut impl BlockHost) {
    fn_263384(m,m.byte(o+0x78) as i32,ctx,sp,tag,e.labels[label],e.log,h);
}
fn block_mark(m:&mut Memory<'_>,o:u32,ctx:u32,sp:u32,tag:u32,label:usize,e:&BlockEnvironment<'_>,h:&mut impl BlockHost) {
    fn_2634c0(m,m.byte(o+0x78) as i32,ctx,sp,tag,e.labels[label],0,0,0,e.log,h);
}

pub fn fn_2645b4(m:&mut Memory<'_>,o:u32,sp:u32,param3:u32,e:&BlockEnvironment<'_>,scratch:u32,h:&mut impl BlockHost,parser:&mut impl BlockParser)->u32 {
    let mut saved=[0;320];let at=m.offset(scratch);saved.copy_from_slice(&m.bytes[at..at+320]);
    let result=(|| {
    if o==0 || sp==0 {return 0x81102007;}
    let (rc,ctx)=lifecycle_context(m,o,e.lifecycle);
    if rc>=0x80000000 {return 0x81102007;}
    let s=scratch;let flag50=s;let v48=s+4;let v44=s+8;let b4c=s+12;let s70=s+16;
    let v64=s+20;let v54=s+24;let mrk=s+32;let v8=s+64;let out=s+68;
    byte(m,flag50,1);byte(m,b4c,0);m.put_half(mrk+10,0);
    for at in [v48,v44,s70,v64,v54,v8] {m.put_word(at,0);}
    macro_rules! call {($f:expr,$a:expr)=>{block_call(m,$f,$a,e,s+96,h,parser)};}
    if m.byte(o+0x81)!=0 {
        byte(m,o+0x18,0);for off in [0x1c,0x20,0x24] {m.put_word(o+off,0);}
        byte(m,o+0x28,0);byte(m,o+0x29,0);m.put_word(out,0);
        let rc=call!(0x2c47d0,&[m.word(o+0x34),out]);if rc>=0x80000000 {return rc;}
    }
    let active=m.byte(o+0x18) as u32!=0;
    if !active {
        for tag in [0x3f3,0x3fd] {
            m.put_word(out,0);
            let rc=call!(0x252f88,&[sp,tag,1,out]);
            if rc<0x80000000 {
                let el=m.word(out);let mk=m.word(el+4);let mut i=1u16;
                while i<half(m,el+8) {
                    let p=mk+i as u32*40+32;
                    if m.word(p)!=0 {render_free(m,m.word(ctx+4),m.word(p),h);m.put_word(p,0);}
                    i=i.wrapping_add(1);
                }
            }
        }
    }
    let tags=if active {&e.clear_active[..]} else {&e.clear_all[..]};
    for &tag in tags {
        m.put_word(out,0);
        let rc=call!(0x252f88,&[sp,tag as u32,1,out]);
        if rc<0x80000000 && tag!=0x3f4 {m.put_half(m.word(out)+8,0);}
    }
    byte(m,s+72,0);
    let method=m.word(m.word(o+8)+0x48);
    let rc=crate::boot_part_e::Host::call(h,m,method,&[m.word(o+12),m.word(o+16),v8,s+72]);
    if rc>=0x80000000 {return rc;}
    if m.byte(s+72)!=0 && m.word(v8)==0 {return 0x81102017;}
    if half(m,o+0x30)==0 && m.word(ctx+20)!=0 {
        if call!(0x1756d0,&[m.word(ctx+20),e.readmode,s70,0])<0x80000000 {
            m.put_half(o+0x30,(m.byte(m.word(s70)) as i32-48) as u16);
        }
    }
    let mut rc=call!(0x2bfa90,&[m.word(o+0x38),sp,flag50,o+0x30]);
    if rc>=0x80000000 {return rc;}
    while m.byte(flag50)==0 {
        let mut rc=0;
        let step=(||->u32 {
            if m.byte(o+0x18) as u32==0 || (m.byte(o+0x29)==0 && m.byte(o+0x28)==1) {
                let old=m.word(o+0x1c);let method=m.word(m.word(o+8)+0x2c);
                rc=crate::boot_part_e::Host::call(h,m,method,&[m.word(o+12),m.word(o+16),o+0x14,o+0x1c]);
                if rc>=0x80000000 {return rc;}
                if m.byte(o+0x18) as u32==0 {call!(0x1748a0,&[m.word(ctx+16),0x20,e.labels[0],e.labels[1],m.word(o+0x14),m.word(o+0x1c)]);}
                byte(m,o+0x18,1);
                render_log(m,ctx,e.log,e.labels[2],&[RenderArgument::Word(m.word(o+0x1c))],h);
                m.put_word(out,0);
                let rc2=call!(0x175944,&[m.word(ctx+20),e.resetlen,out]);
                if rc2&0x1fff==0x14 {m.put_word(out,0);} else if rc2>=0x80000000 {return rc;}
                render_log(m,ctx,e.log,e.labels[3],&[RenderArgument::Word(m.word(out))],h);
                if m.word(out)!=0 {
                    m.put_word(s+76,0);
                    let method=m.word(m.word(o+8)+0x44);
                    rc=crate::boot_part_e::Host::call(h,m,method,&[m.word(o+12),m.word(o+16),s+76]);
                    if rc>=0x80000000 {return rc;}
                    render_log(m,ctx,e.log,e.labels[4],&[RenderArgument::Word(m.word(s+76))],h);
                    if m.byte(o+0x28)==0 {
                        rc=call!(0x2c4728,&[m.word(o+0x34),m.word(s+76)]);if rc>=0x80000000 {return rc;}
                    }
                } else {
                    rc=call!(0x2c4750,&[m.word(o+0x34)]);if rc>=0x80000000 {return rc;}
                    if half(m,o+0x30)==0 && m.word(ctx+20)!=0 && call!(0x1756d0,&[m.word(ctx+20),e.readmode,s70,0])<0x80000000 {
                        m.put_half(o+0x30,(m.byte(m.word(s70)) as i32-48) as u16);
                    }
                }
                if m.byte(o+0x28)==1 && old!=0 && m.word(o+0x1c)==old {byte(m,o+0x29,1);}else{byte(m,o+0x28,0);}
            }
            rc=call!(0x252f88,&[sp,0x3e8,1,out]);if rc>=0x80000000 {return rc;}
            let el=m.word(out);
            if m.byte(el+12)!=0 {
                if m.word(o+0x1c)!=0 {
                    let method=m.word(m.word(o+8)+0x30);
                    rc=crate::boot_part_e::Host::call(h,m,method,&[m.word(o+12),m.word(o+16),m.word(o+0x1c)]);
                    if rc>=0x80000000 {return rc;}
                }
                let rc=crate::render_part_c::fn_2b92d8(m,ctx,e.lifecycle.parameter_pairs,e.lifecycle.parameter_triple,&mut LifecycleCalls{host:h,scratch:s+96});
                if rc>=0x80000000 {return rc;}
                return 0x81102017;
            }
            let diff=m.word(o+0x1c).wrapping_sub(m.word(o+0x20).wrapping_mul(2));
            if diff<=1 {
                if m.byte(o+0x29)!=0 || m.byte(o+0x28)!=0 {byte(m,el+12,1);}
                else {
                    byte(m,o+0x28,1);
                    if m.byte(o+0x81)!=0 && m.word(o+0x1c)!=0 {
                        let method=m.word(m.word(o+8)+0x30);
                        rc=crate::boot_part_e::Host::call(h,m,method,&[m.word(o+12),m.word(o+16),m.word(o+0x1c)]);
                        if rc>=0x80000000 {return rc;}
                    }
                    return 0x81102800;
                }
            } else {
                let n=if m.byte(o+0x81)!=0 {diff as u16 as u32} else {0x80};
                rc=call!(0x252f88,&[sp,0x3e8,n,out]);if rc>=0x80000000 {return rc;}
                if half(m,m.word(out)+8)!=0 {
                    let count=half(m,m.word(out)+8) as u32;
                    rc=call!(0x252f88,&[sp,0x3e8,(count+n) as u16 as u32,out]);if rc>=0x80000000 {return rc;}
                }
                let el=m.word(out);let count=half(m,el+8) as u32;
                copy_bytes(m,m.word(el+4)+count*2,m.word(o+0x14)+m.word(o+0x20)*2,diff.min(n*2));
                let used=if diff>n*2 {n} else {diff/2};
                add_word(m,o+0x20,used);m.put_half(el+8,(count+used) as u16);
                rc=call!(0x2c2408,&[m.word(o+0x38)]);
            }
            rc
        })();
        if step>=0x80000000 {return step;}
        block_text(m,o,ctx,sp,0x3e8,5,e,h);block_mark(m,o,ctx,sp,0x3f3,6,e,h);block_text(m,o,ctx,sp,0x3f2,6,e,h);
        let r4=call!(0x2c4914,&[m.word(o+0x34),sp]);
        block_mark(m,o,ctx,sp,0x3f3,7,e,h);block_text(m,o,ctx,sp,0x3f2,7,e,h);
        if r4&0x1fff==9 {continue;}if r4>=0x80000000 {return r4;}
        for tag in [0x3f3,0x3fd] {block_mark(m,o,ctx,sp,tag,8,e,h);}
        for tag in [0x3f2,0x3fc] {block_text(m,o,ctx,sp,tag,8,e,h);}
        let rc=call!(0x2c0014,&[m.word(o+0x38),sp,m.byte(o+0x76) as u32,half(m,o+0x30) as i16 as i32 as u32]);
        if rc>=0x80000000 {return rc;}
        block_mark(m,o,ctx,sp,0x3f3,9,e,h);
        if m.byte(o+0x81)!=0 {byte(m,flag50,1);break;}
        let rc=call!(0x2bfa90,&[m.word(o+0x38),sp,flag50,o+0x30]);if rc>=0x80000000 {return rc;}
    }
    block_mark(m,o,ctx,sp,0x3f3,10,e,h);m.put_word(v44,0);m.put_word(v48,0);
    rc=call!(0x2c0f74,&[m.word(o+0x38),sp,v44,v48,b4c,param3,o+0x30,v64,v54,mrk]);
    if rc>=0x80000000 {return rc;}
    let mut name=m.word(e.log.marker_names+4);
    for k in 0..40 {if half(m,e.log.marker_names+k*8)==half(m,mrk+10) {name=m.word(e.log.marker_names+k*8+4);break;}}
    render_log(m,ctx,e.log,e.marker_log,&[RenderArgument::Word(half(m,o+0x30) as u32),RenderArgument::Word(m.word(v64)),RenderArgument::Word(m.word(v54)),RenderArgument::Guest(name)],h);
    if call!(0x17497c,&[m.word(ctx+16)])!=0 {call!(0x17486c,&[m.word(ctx+16),0x21,0]);}
    block_mark(m,o,ctx,sp,0x3f3,11,e,h);block_text(m,o,ctx,sp,0x3fc,11,e,h);block_mark(m,o,ctx,sp,0x3fd,12,e,h);
    let save0=m.word(v44);let save1=m.word(v48);let reset=m.byte(b4c);
    rc=call!(0x252f88,&[sp,0x3fc,1,out]);
    if rc<0x80000000 {
        let el=m.word(out);let count=half(m,el+8) as u32;
        if count!=0 && m.byte(o+0x78)==1 {
            let n=count.min(0x400);let buf=crate::frontend::call_method(m.bytes,nz(m.word(ctx+4)),1,n*2+2,h);
            if buf==0 {return 0x8110200a;}
            narrow(m,buf,m.word(el+4),n);byte(m,buf+n,0);
            render_log(m,ctx,e.log,e.labels[13],&[RenderArgument::Word(0x3fc),RenderArgument::Word(count),RenderArgument::Guest(buf)],h);
            render_free(m,m.word(ctx+4),buf,h);
        }
        add_word(m,o+0x24,save0.wrapping_add(save1).wrapping_mul(half(m,el+2) as u32));
        if reset==1 || m.word(o+0x24)>m.word(o+0x2c) {
            let method=m.word(m.word(o+8)+0x30);
            rc=crate::boot_part_e::Host::call(h,m,method,&[m.word(o+12),m.word(o+16),m.word(o+0x24)]);
            if rc<0x80000000 {
                rc=call!(0x2c4768,&[m.word(o+0x34),v8]);
                if rc<0x80000000 {
                    rc=call!(0x2c1e2c,&[m.word(o+0x38),sp]);
                    if rc<0x80000000 && half(m,o+0x30)!=1 {rc=call!(0x1758ac,&[m.word(ctx+20),e.mode_key,half(m,o+0x30) as i16 as i32 as u32]);}
                    if rc<0x80000000 {rc=call!(0x2c242c,&[m.word(o+0x38)]);}
                    if rc<0x80000000 {rc=call!(0x1758ac,&[m.word(ctx+20),e.position_key,m.word(o+0x24)]);}
                    if rc<0x80000000 {
                        byte(m,o+0x18,0);for off in [0x1c,0x20,0x14,0x24] {m.put_word(o+off,0);}
                        byte(m,o+0x28,0);byte(m,o+0x29,0);
                        rc=call!(0x252f88,&[sp,0x3e8,1,out]);if rc<0x80000000 {byte(m,m.word(out)+12,0);}
                    }
                }
            }
        }
    }
    if rc>=0x80000000 {return rc;}
    if m.byte(param3)==1 {render_log(m,ctx,e.log,e.labels[14],&[],h);return rc;}
    if m.byte(b4c)==1 {render_log(m,ctx,e.log,e.labels[15],&[],h);}
    if m.byte(o+0x77)!=0 {
        let rc2=call!(0x252f88,&[sp,0x3f4,1,out]);if rc2<0x80000000 {m.put_half(o+0x74,half(m,m.word(out)+8));}
        rc=call!(0x26a610,&[sp,0x3f3]);if rc>=0x80000000 {return rc;}
        block_mark(m,o,ctx,sp,0x3fd,16,e,h);
        let r4=call!(0x2baf40,&[m.word(o+0x3c),sp,m.word(v64),m.word(v54),mrk]);
        block_mark(m,o,ctx,sp,0x3fd,17,e,h);if r4>=0x80000000 {return r4;}
    } else {
        rc=call!(0x2c3e78,&[m.word(o+0x3c),sp,m.word(v64),m.word(v54)]);if rc>=0x80000000 {return rc;}
    }
    block_text(m,o,ctx,sp,0x3fc,18,e,h);block_mark(m,o,ctx,sp,0x3fd,19,e,h);
    let rc2=call!(0x252f88,&[sp,0x3fd,1,out]);
    if rc2>=0x80000000 {return rc2;}
    let el=m.word(out);let mk=m.word(el+4);let mut i=1u16;
    while i<half(m,el+8) {
        let at=mk+i as u32*40;
        if half(m,at+10)==7 && m.word(at+28)>=0x46 {
            let cur=m.word(at+20);i=i.wrapping_add(1);
            while i<half(m,el+8) && half(m,mk+i as u32*40+10)!=7 {i=i.wrapping_add(1);}
            let end=if i>=half(m,el+8) {(m.word(mk+20)+m.word(mk+24)) as u16 as u32}else{m.word(mk+i as u32*40+20)};
            let mut lo=0u16;let mut hi=0u16;let mut k=1u16;
            while k<half(m,el+8) && m.word(mk+k as u32*40+20)<=end {
                let p=mk+k as u32*40;let pos=m.word(p+20);
                if half(m,p+10)==1 && pos>=cur && pos<end {if lo==0 {lo=k;}hi=k;}
                k=k.wrapping_add(1);
            }
            if lo!=0 && hi!=0 && lo!=hi {
                let first=mk+lo as u32*40;let last=mk+hi as u32*40;
                m.put_word(first+24,m.word(last+20).wrapping_add(m.word(last+24)).wrapping_sub(m.word(first+20)));
                m.put_word(first+16,m.word(last+12).wrapping_add(m.word(last+16)).wrapping_sub(m.word(first+12)));
                while hi>lo {
                    let p=mk+hi as u32*40;let pos=m.word(p+20);
                    if half(m,p+10)==1 && pos>=cur && pos<end {
                        let n=half(m,el+8);
                        if hi.wrapping_add(1)!=n {move_bytes(m,p,p+40,(n-hi) as u32*40);}
                        m.put_half(el+8,n.wrapping_sub(1));
                    }
                    hi-=1;
                }
            }
        } else {i=i.wrapping_add(1);}
    }
    block_text(m,o,ctx,sp,0x3fc,19,e,h);block_mark(m,o,ctx,sp,0x3fd,20,e,h);
    rc=call!(0x2c322c,&[m.word(o+0x40),sp]);if rc>=0x80000000 {return rc;}
    block_text(m,o,ctx,sp,0x3fc,20,e,h);block_mark(m,o,ctx,sp,0x3fd,20,e,h);
    if m.byte(o+0x77)!=0 {rc=call!(0x2bb4c8,&[m.word(o+0x3c),sp]);if rc>=0x80000000 {return rc;}}
    rc=call!(0x252f88,&[sp,0x3fd,1,out]);if rc>=0x80000000 {return rc;}
    let el=m.word(out);let mk=m.word(el+4);let mut w=1u16;
    for k in 1..half(m,el+8) {
        if half(m,mk+k as u32*40+10)!=0x4000 {move_bytes(m,mk+w as u32*40,mk+k as u32*40,40);w=w.wrapping_add(1);}
    }
    m.put_half(el+8,w);
    if m.byte(o+0x81)!=0 && m.word(o+0x1c)!=0 {
        let method=m.word(m.word(o+8)+0x30);
        rc=crate::boot_part_e::Host::call(h,m,method,&[m.word(o+12),m.word(o+16),m.word(o+0x1c)]);
        if rc>=0x80000000 {return rc;}
    }
    0
    })();
    m.bytes[at..at+320].copy_from_slice(&saved);result
}
