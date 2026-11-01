

use crate::boot::{Calls, Memory};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LiteralM {
    NumberOutputTranscriptions,
    PoolFile,
    PoolReportFormat,
    FreePoolLabel,
    DictionaryFile,
    DictionaryTag,
    InvalidTranscriptionFormat,
    ReleaseTranscriptionsFormat,
    FactorFile,
    FactorError,
}

pub trait HooksM {
    fn call_m(&mut self, address: u32, args: &[u32], mem: &mut Memory<'_>) -> u32;
    fn literal_m(&self, _literal: LiteralM) -> u32 {
        u32::MAX
    }
}
impl<T: Calls + ?Sized> HooksM for T {
    fn call_m(&mut self, address: u32, args: &[u32], mem: &mut Memory<'_>) -> u32 {
        self.call(mem, address, args)
    }
}
fn h(m: &Memory<'_>, p: u32) -> u32 {
    u16::from_le_bytes(m.bytes[m.offset(p)..m.offset(p) + 2].try_into().unwrap()) as u32
}
fn b(m: &mut Memory<'_>, p: u32, v: u8) {
    let i = m.offset(p);
    m.bytes[i] = v;
}
fn row(m: &Memory<'_>, p: u32, i: u32) -> u32 {
    m.word(m.word(p.wrapping_add(60)).wrapping_add(i.wrapping_mul(4)))
}
fn at(m: &Memory<'_>, p: u32, i: u32) -> i32 {
    m.word(p.wrapping_add(i.wrapping_mul(4))) as i32
}
fn set(m: &mut Memory<'_>, p: u32, i: u32, v: i32) {
    m.put_word(p.wrapping_add(i.wrapping_mul(4)), v as u32);
}
fn l(x: i32, s: u32) -> i32 {
    let s = s & 255;
    if s >= 32 {
        0
    } else {
        x.wrapping_shl(s)
    }
}
fn r(x: i32, s: u32) -> i32 {
    x >> (s & 255).min(31)
}

pub fn contains_m(
    m: &mut Memory<'_>,
    hooks: &mut impl HooksM,
    p: u32,
    key: u32,
    size: u32,
    found: u32,
    scratch: u32,
) -> u32 {
    m.put_word(scratch, 0);
    b(m, found, 0);
    let rc = hooks.call_m(
        0x17a1a4,
        &[m.word(p + 16), m.word(p + 20), 40, size, scratch],
        m,
    );
    if rc != 0 {
        hooks.call_m(0x179f3c, &[m.word(p + 16), m.word(p + 20)], m);
        m.put_word(p + 20, 0);
        return rc;
    }
    let data = m.word(scratch);
    let mut pos = 0u32;
    while pos < size && hooks.call_m(0x1769d0, &[data.wrapping_add(pos), key], m) != 0 {
        let n = hooks.call_m(0x195d90, &[data.wrapping_add(pos)], m);
        pos = n.wrapping_add(pos).wrapping_add(4) & !3;
        pos = pos.wrapping_add(4);
    }
    if pos < size {
        b(m, found, 1);
    }
    hooks.call_m(0x179208, &[m.word(p + 16), m.word(p + 20), data], m);
    0
}

pub fn vec_grow_m(m: &mut Memory<'_>, hooks: &mut impl HooksM, p: u32, out: u32) -> u32 {
    if p == 0 || out == 0 {
        return 0;
    }
    let count = m.word(p + 20);
    if m.word(p + 16) < count.wrapping_add(1) {
        let g = h(m, p + 24);
        let n = hooks
            .call_m(0x1994d0, &[count, g], m)
            .wrapping_add(1)
            .wrapping_mul(g);
        let data = m.word(p);
        let bytes = h(m, p + 4).wrapping_mul(n);
        let heap = m.word(p + 28);
        if data != 0 {
            let d = hooks.call_m(0x1963e0, &[heap, data, bytes], m);
            if d == 0 {
                return 0;
            }
            m.put_word(p, d);
        } else {
            let d = hooks.call_m(0x196394, &[heap, bytes], m);
            m.put_word(p, d);
            if d == 0 {
                return 0;
            }
        }
        m.put_word(p + 16, n);
    }
    let count = m.word(p + 20);
    m.put_word(out, count.wrapping_mul(h(m, p + 4)).wrapping_add(m.word(p)));
    m.put_word(p + 20, m.word(p + 20).wrapping_add(1));
    1
}

pub fn free_pool_m(m: &mut Memory<'_>, hooks: &mut impl HooksM, p: u32) {
    if p == 0 || m.word(p) == 0 {
        return;
    }
    let mut block = m.word(p + 8);
    let mut count = 0u32;
    while block != 0 {
        count = count.wrapping_add(1);
        block = m.word(block);
    }
    let file = hooks.literal_m(LiteralM::PoolFile);
    let format = hooks.literal_m(LiteralM::PoolReportFormat);
    let label = hooks.literal_m(LiteralM::FreePoolLabel);
    hooks.call_m(
        0x1738e8,
        &[
            m.word(m.word(p) + 16),
            file,
            3,
            0,
            format,
            m.word(p + 4),
            label,
            m.word(p + 20),
            m.word(p + 16),
            m.word(p + 12),
            count,
        ],
        m,
    );
    block = m.word(p + 8);
    while block != 0 {
        let next = m.word(block);
        hooks.call_m(0x196428, &[m.word(m.word(p) + 4), block], m);
        block = next;
    }
    m.put_word(p + 8, 0);
    m.put_word(p + 20, 0);
    m.put_word(p + 16, 0);
}

pub fn factor_lattice_m(m: &mut Memory<'_>, hooks: &mut impl HooksM, context: u32, p: u32) {
    let mut shift = 0u32;
    while at(m, row(m, p, 0), 0) < 0x1fffffff {
        shift = shift.wrapping_add(2);
        set(m, row(m, p, 0), 0, l(at(m, row(m, p, 0), 0), 2));
    }
    let value = l(
        hooks.call_m(0x29b570, &[at(m, row(m, p, 0), 0) as u32], m) as i32,
        16u32.wrapping_sub(shift) >> 1,
    );
    set(m, row(m, p, 0), 0, value);
    let first = row(m, p, 0);
    let v = hooks.call_m(0x30209c, &[at(m, first, 1) as u32, r(value, 8) as u32], m);
    set(m, first, 1, l(v as i32, 8));
    let first = row(m, p, 0);
    let v = hooks.call_m(
        0x30209c,
        &[at(m, first, 2) as u32, r(at(m, first, 0), 8) as u32],
        m,
    );
    set(m, first, 2, l(v as i32, 8));
    let mut i = 1u32;
    loop {
        let x = r(at(m, row(m, p, i - 1), 1), 8);
        let mut value = at(m, row(m, p, i), 0).wrapping_sub(x.wrapping_mul(x));
        set(m, row(m, p, i), 0, value);
        if i > 1 {
            let x = r(at(m, row(m, p, i - 2), 2), 8);
            value = value.wrapping_sub(x.wrapping_mul(x));
            set(m, row(m, p, i), 0, value);
        }
        if value <= 0 {
            let file = hooks.literal_m(LiteralM::FactorFile);
            let message = hooks.literal_m(LiteralM::FactorError);
            hooks.call_m(0x1738e8, &[m.word(context + 16), file, 0, 0, message], m);
            return;
        }
        let mut shift = 0u32;
        if i == 1 {
            while at(m, row(m, p, i), 0) < 0x1fffffff {
                shift = shift.wrapping_add(2);
                set(m, row(m, p, i), 0, l(at(m, row(m, p, i), 0), 2));
            }
            value = at(m, row(m, p, i), 0);
        } else {
            while value < 0x1fffffff {
                value = l(value, 2);
                shift = shift.wrapping_add(2);
            }
        }
        let normalized = l(
            hooks.call_m(0x29b570, &[value as u32], m) as i32,
            16u32.wrapping_sub(shift) >> 1,
        );
        let divisor = r(normalized, 8);
        set(m, row(m, p, i), 0, normalized);
        let x = r(at(m, row(m, p, i - 1), 1), 8);
        let y = r(at(m, row(m, p, i - 1), 2), 8);
        let value = at(m, row(m, p, i), 1).wrapping_sub(x.wrapping_mul(y));
        set(m, row(m, p, i), 1, value);
        let current = row(m, p, i);
        let v = hooks.call_m(0x30209c, &[value as u32, divisor as u32], m);
        set(m, current, 1, l(v as i32, 8));
        let current = row(m, p, i);
        let v = hooks.call_m(0x30209c, &[at(m, current, 2) as u32, divisor as u32], m);
        set(m, current, 2, l(v as i32, 8));
        i = i.wrapping_add(1);
        if i as i32 >= m.word(p + 32) as i32 {
            return;
        }
    }
}

pub fn forward_lattice_m(
    m: &mut Memory<'_>,
    hooks: &mut impl HooksM,
    p: u32,
    bits: u32,
    c: u32,
    d: u32,
) {
    let shift = d.wrapping_sub(bits);
    let q = r(at(m, row(m, p, 0), 0), c.wrapping_sub(bits >> 1));
    let out = m.word(p + 56);
    let x = at(m, m.word(p + 64), 0);
    let upper = if shift & 255 >= 32 {
        0
    } else {
        (i32::MAX as u32) >> (shift & 255)
    } as i32;
    let (value, s) = if x > upper || x < r(i32::MIN, shift) {
        (l(x, shift.wrapping_sub(2)), 2u32.wrapping_add(bits >> 1))
    } else {
        (l(x, shift), bits >> 1)
    };
    let v = hooks.call_m(0x30209c, &[value as u32, q as u32], m);
    set(m, out, 0, l(v as i32, s));
    let half = d >> 1;
    if m.word(p + 32) as i32 > 1 {
        let coefficient = at(m, row(m, p, 0), 1);
        let acc = if coefficient != 0 {
            r(coefficient, half).wrapping_mul(r(at(m, m.word(p + 56), 0), half))
        } else {
            0
        };
        let value = r(at(m, m.word(p + 64), 1), bits.wrapping_sub(c)).wrapping_sub(acc);
        let divisor = r(at(m, row(m, p, 1), 0), half);
        let out = m.word(p + 56);
        let v = hooks.call_m(0x30209c, &[value as u32, divisor as u32], m);
        set(m, out, 1, l(v as i32, half));
    }
    let mut i = 2u32;
    while (i as i32) < m.word(p + 32) as i32 {
        let out = m.word(p + 56);
        let sum = r(at(m, row(m, p, i - 1), 1), half)
            .wrapping_mul(r(at(m, m.word(p + 56), i - 1), half))
            .wrapping_add(
                r(at(m, row(m, p, i - 2), 2), half)
                    .wrapping_mul(r(at(m, m.word(p + 56), i - 2), half)),
            );
        let value = r(at(m, m.word(p + 64), i), bits.wrapping_sub(c)).wrapping_sub(sum);
        let q = r(at(m, row(m, p, i), 0), half);
        let v = if q == 0 {
            value
        } else {
            hooks.call_m(0x30209c, &[value as u32, q as u32], m) as i32
        };
        set(m, out, i, l(v, half));
        i = i.wrapping_add(1);
    }
}

pub fn lookup_transcriptions_m(
    m: &mut Memory<'_>,
    hooks: &mut impl HooksM,
    owner: u32,
    dictionary: u32,
    language: u32,
    word: u32,
    output: u32,
    capacity: u32,
    content_type: u32,
    scratch: u32,
) -> u32 {
    for offset in [0, 4, 8, 12, 16, 20] {
        m.put_word(scratch + offset, 0);
    }
    let context = m.word(owner + 8);
    let dictionary_context = m.word(dictionary + 8);
    let setting_key = hooks.literal_m(LiteralM::NumberOutputTranscriptions);
    let status = hooks.call_m(0x175944, &[m.word(context + 20), setting_key, scratch], m);
    let limit = if status < 0x80000000 {
        h(m, scratch)
    } else {
        1
    };
    if owner == 0
        || m.word(owner + 36) == 0
        || dictionary_context == 0
        || word == 0
        || m.byte(word) == 0
        || capacity == 0
    {
        return 0x85002007;
    }
    let interface = m.word(dictionary + 40);
    let vt = m.word(interface);
    let object = interface.wrapping_sub(m.word(vt + 24));
    if hooks.call_m(m.word(vt + 8), &[object, scratch + 4], m) != 0
        || m.word(m.word(scratch + 4)) == 0
    {
        return 0x85002014;
    }
    for i in 0..3 {
        let lang = m.word(m.word(scratch + 4));
        let c = hooks.call_m(0x17c8c0, &[m.byte(lang + i) as u32], m);
        b(m, scratch + 8 + i, c as u8);
    }
    if m.word(language) != 0 && hooks.call_m(0x1960a8, &[m.word(language), scratch + 8], m) == 0 {
        return 0x85002014;
    }
    m.put_word(language, m.word(m.word(scratch + 4)));
    let interface = m.word(dictionary + 40);
    let vt = m.word(interface);
    let mut status = hooks.call_m(
        m.word(vt),
        &[
            interface.wrapping_sub(m.word(vt + 24)),
            word,
            scratch + 12,
            scratch + 16,
        ],
        m,
    );
    let records = m.word(scratch + 12);
    let count = m.word(scratch + 16);
    if status == 0 && count != 0 && records != 0 {
        let kind = m.byte(records) as u32;
        m.put_word(content_type, kind);
        match kind {
            1 => {
                m.put_half(output, 0);
                let mut length = 0u32;
                let mut i = 0u32;
                while i < limit.min(count) {
                    let record = records.wrapping_add(i.wrapping_mul(12));
                    if m.byte(record) == 1 {
                        let text = m.word(record + 4);
                        let mut n = m.word(record + 8);
                        for j in 0..n {
                            if m.byte(text.wrapping_add(j)) == 0 {
                                n = j;
                                break;
                            }
                        }
                        if n.wrapping_add(1) > h(m, capacity) {
                            status = 0x85002009;
                            break;
                        }
                        if i != 0 {
                            m.put_word(scratch + 20, 0x12);
                            hooks.call_m(0x176314, &[output, scratch + 20], m);
                            length = length.wrapping_add(1);
                        }
                        hooks.call_m(0x176314, &[output, text], m);
                        length = length.wrapping_add(n);
                    }
                    i = i.wrapping_add(1);
                }
                if status == 0 {
                    m.put_half(output.wrapping_add(length.wrapping_mul(2)), 0);
                    m.put_half(capacity, length as u16);
                }
            }
            4 => {
                let text = m.word(records + 4);
                let length = hooks.call_m(0x17bfc0, &[text], m);
                if length > h(m, capacity) {
                    status = 0x85002009;
                } else {
                    status = hooks.call_m(0x17bd74, &[text, output], m);
                    if status < 0x80000000 {
                        m.put_half(capacity, length.wrapping_sub(1) as u16);
                    }
                }
            }
            _ => {
                let file = hooks.literal_m(LiteralM::DictionaryFile);
                let format = hooks.literal_m(LiteralM::InvalidTranscriptionFormat);
                let tag = hooks.literal_m(LiteralM::DictionaryTag);
                hooks.call_m(
                    0x173dc0,
                    &[m.word(context + 16), file, 0x117c, format, tag, kind],
                    m,
                );
                status = 0x85002014;
            }
        }
    } else if status < 0x80000000 {
        status = 0x85002014;
    }
    if records != 0 {
        let interface = m.word(dictionary + 40);
        let vt = m.word(interface);
        let result = hooks.call_m(
            m.word(vt + 4),
            &[interface.wrapping_sub(m.word(vt + 24)), records],
            m,
        );
        status = result;
        if result != 0 {
            let file = hooks.literal_m(LiteralM::DictionaryFile);
            let format = hooks.literal_m(LiteralM::ReleaseTranscriptionsFormat);
            hooks.call_m(
                0x1738e8,
                &[m.word(context + 16), file, 3, 0, format, result],
                m,
            );
            return result.wrapping_add(0x1ea0) | 0x85002000;
        }
    }
    status
}

pub fn release_shared_stream_m(m: &mut Memory<'_>, hooks: &mut impl HooksM, p: u32) {
    let refs = h(m, p + 0x1c);
    if refs == 0 {
        return;
    }
    m.put_half(p + 0x1c, refs.wrapping_sub(1) as u16);
    if h(m, p + 0x1c) != 0 {
        return;
    }
    for i in 0..5 {
        let filter = p + 0x26c + i * 0x48;
        for offset in (0x2c..=0x40).step_by(4) {
            m.put_word(filter + offset, 0);
        }
        free_pool_m(m, hooks, filter);
    }
    for offset in [0x430, 0x42c] {
        if m.word(p + offset) != 0 {
            hooks.call_m(
                0x196428,
                &[m.word(m.word(p + 0x18) + 4), m.word(p + offset)],
                m,
            );
            m.put_word(p + offset, 0);
        }
    }
    free_pool_m(m, hooks, p);
    free_pool_m(m, hooks, p + 0x24);
}
