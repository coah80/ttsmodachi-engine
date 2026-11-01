
use crate::{boot::Memory, normalization};

pub struct Environment {
    pub stream_base_vtable: u32,
    pub stream_vtable: u32,
    pub vector_vtable: u32,
    pub compare: u32,
    pub accept: u32,
}

#[derive(Clone, Copy)]
pub enum Key {
    Address(u32),
    Local(u32),
}
pub trait Host {
    fn call(&mut self, m: &mut Memory<'_>, method: u32, args: &[u32]) -> u32;

    fn compare(&mut self, m: &mut Memory<'_>, method: u32, key: &mut Key, value: &mut u32) -> u32;
}
fn byte(m: &mut Memory<'_>, p: u32, v: u8) {
    let i = m.offset(p);
    m.bytes[i] = v;
}

pub fn compare_parameter(m: &Memory<'_>, key: Key, value: u32) -> u32 {
    let a = match key {
        Key::Address(p) => m.word(p),
        Key::Local(v) => v,
    };
    let b = m.word(value);
    let mut frame = [0u8; 16];
    frame[0..4].copy_from_slice(&8u32.to_le_bytes());
    frame[4..8].copy_from_slice(&12u32.to_le_bytes());
    frame[8..12].copy_from_slice(&a.to_le_bytes());
    frame[12..16].copy_from_slice(&b.to_le_bytes());
    normalization::dbl_deref_ne(&frame, 0, 4)
}
pub fn fn_1b2128(m: &mut Memory<'_>, p: u32, x: u32, h: &mut impl Host) {
    if x != 0 {
        h.call(m, m.word(p + 12), &[m.word(p + 16), x]);
    }
}
pub fn fn_1b2140(m: &mut Memory<'_>, p: u32, size: u32, out: &mut u8, h: &mut impl Host) -> u32 {
    if size == 0 {
        *out = 9;
        return 0;
    }
    let r = h.call(m, m.word(p), &[m.word(p + 16), size]);
    *out = u8::from(r == 0);
    r
}
pub fn fn_1b218c(
    m: &mut Memory<'_>,
    p: u32,
    old: u32,
    size: u32,
    out: &mut u8,
    h: &mut impl Host,
) -> u32 {
    if size == 0 {
        *out = 9;
        return 0;
    }
    let r = h.call(m, m.word(p + 8), &[m.word(p + 16), old, size]);
    *out = u8::from(r == 0);
    r
}
pub fn fn_1ba3c0(m: &mut Memory<'_>, p: u32, e: &Environment) -> u32 {
    m.put_word(p, e.stream_base_vtable);
    0
}
pub fn fn_1ba85c(m: &mut Memory<'_>, v: u32, value: u32, h: &mut impl Host) -> u32 {
    let need = m.word(v + 8).wrapping_add(1);
    let mut status = 0;
    if m.word(v + 12) < need {
        let cap = m.word(v + 12).wrapping_add(m.word(v + 16)).max(need);
        let p = fn_1b218c(
            m,
            m.word(v + 20),
            m.word(v + 4),
            cap.wrapping_mul(4),
            &mut status,
            h,
        );
        if status == 0 {
            m.put_word(v + 4, p);
            m.put_word(v + 12, cap);
        }
    }
    if status == 0 {
        let n = m.word(v + 8);
        let base = m.word(v + 4);
        m.put_word(v + 8, n.wrapping_add(1));
        m.put_word(base.wrapping_add(n.wrapping_mul(4)), value);
    }
    status as u32
}
pub fn fn_1ba8e0(m: &mut Memory<'_>, v: u32, mut key: Key, cmp: u32, h: &mut impl Host) -> u32 {
    let mut i = 0u32;
    while i < m.word(v + 8) {
        let mut val = m.word(m.word(v + 4).wrapping_add(i.wrapping_mul(4)));
        if h.compare(m, cmp, &mut key, &mut val) == 0 {
            return val;
        }
        i = i.wrapping_add(1);
    }
    0
}

#[allow(clippy::too_many_arguments)]
pub fn fn_1b35a0(
    m: &mut Memory<'_>,
    s: u32,
    key: u32,
    value: u32,
    validate: u32,
    reserved: u32,
    out: u32,
    persistent: u32,
    e: &Environment,
    h: &mut impl Host,
) -> u32 {
    fn_1ba8e0(m, m.word(s + 8), Key::Local(key), e.compare, h);
    let mut status = 0;
    let p = fn_1b2140(m, m.word(s + 4), 28, &mut status, h);
    if status == 0 {
        m.put_word(p + 12, out);
        m.put_word(p, key);
        m.put_word(p + 4, value);
        if out != 0 {
            m.put_word(out, value);
        } else {
            m.put_word(p + 8, value);
        }
        m.put_word(p + 16, validate);
        m.put_word(p + 24, reserved);
        byte(m, p + 20, persistent as u8);
        byte(m, p + 21, 0);
        status = fn_1ba85c(m, m.word(s + 8), p, h) as u8;
    }
    status as u32
}
pub fn fn_1b3b3c(m: &mut Memory<'_>, s: u32, key: u32, e: &Environment, h: &mut impl Host) -> u32 {
    let p = fn_1ba8e0(m, m.word(s + 8), Key::Local(key), e.compare, h);
    let out = m.word(p + 12);
    if out != 0 {
        m.word(out)
    } else {
        m.word(p + 8)
    }
}
pub fn fn_1b3fe8(
    m: &mut Memory<'_>,
    s: u32,
    key: u32,
    value: u32,
    e: &Environment,
    h: &mut impl Host,
) {
    let p = fn_1ba8e0(m, m.word(s + 8), Key::Local(key), e.compare, h);
    let out = m.word(p + 12);
    if out != 0 {
        m.put_word(out, value);
    } else {
        m.put_word(p + 8, value);
    }
}

pub fn fn_1b449c(m: &mut Memory<'_>, p: u32, state: u32, e: &Environment) -> u32 {
    let status = fn_1ba3c0(m, p, e);
    if status == 0 {
        m.put_word(p + 8, 0);
        m.put_word(p, e.stream_vtable);
        m.put_word(p + 4, state);
    }
    status
}
#[allow(clippy::too_many_arguments)]
pub fn fn_1babcc(
    m: &mut Memory<'_>,
    heap: u32,
    x: u32,
    count: u32,
    mut growth: u32,
    out: u32,
    e: &Environment,
    h: &mut impl Host,
) -> u32 {
    let mut status = 0;
    let p = fn_1b2140(m, heap, 24, &mut status, h);
    if status != 0 {
        return status as u32;
    }
    let mut err = fn_1ba3c0(m, p, e) as u8;
    if err == 0 {
        m.put_word(p + 4, 0);
        m.put_word(p + 8, 0);
        m.put_word(p, e.vector_vtable);
        m.put_word(p + 12, 0);
        m.put_word(p + 16, growth);
        m.put_word(p + 20, x);
        if count >= 1 {
            if growth < count {
                growth = count;
            }
            let buf = fn_1b218c(
                m,
                m.word(p + 20),
                m.word(p + 4),
                growth.wrapping_mul(4),
                &mut err,
                h,
            );
            if err == 0 {
                m.put_word(p + 4, buf);
                m.put_word(p + 12, growth);
            }
        }
    }
    if err == 0 {
        m.put_word(out, p);
    } else {
        fn_1b2128(m, heap, p, h);
        m.put_word(out, 0);
    }
    err as u32
}

impl Environment {

    pub const US: Self = Self {
        stream_base_vtable: 0x917e80,
        stream_vtable: 0x917e8c,
        vector_vtable: 0x9c8954,
        compare: 0x1b3580,
        accept: 0x1b3c00,
    };

    pub const EU: Self = Self {
        stream_base_vtable: 0x9190c0,
        stream_vtable: 0x9190cc,
        vector_vtable: 0x9c9b94,
        compare: 0x1b35d4,
        accept: 0x1b3c54,
    };

    pub const KR: Self = Self {
        stream_base_vtable: 0x93b420,
        stream_vtable: 0x93b42c,
        vector_vtable: 0x9ebef4,
        compare: 0x1b37d0,
        accept: 0x1b3e50,
    };
}

pub fn fn_1b3c34(
    m: &mut Memory<'_>,
    s: u32,
    data: u32,
    add_missing: u32,
    e: &Environment,
    h: &mut impl Host,
) -> u32 {
    let at = m.offset(data);
    let count = u16::from_le_bytes(m.bytes[at..at + 2].try_into().unwrap()) as u32;
    let persistent_count = u16::from_le_bytes(m.bytes[at + 2..at + 4].try_into().unwrap()) as u32;
    let ordinary = data.wrapping_add(4);
    let persistent = ordinary.wrapping_add(count.wrapping_mul(8));
    for pass in 0..2 {
        let (mut cursor, n) = if pass == 0 {
            (persistent, persistent_count)
        } else {
            (ordinary, count)
        };
        let mut i = 0u16;
        while (i as u32) < n {
            let key = m.word(cursor);
            cursor = cursor.wrapping_add(4);
            let value = m.word(cursor);
            cursor = cursor.wrapping_add(4);
            let p = fn_1ba8e0(m, m.word(s + 8), Key::Local(key), e.compare, h);
            if p != 0 {
                let validate = m.word(p + 16);
                if validate != 0 {
                    let valid = if validate == e.accept {
                        normalization::code_1(value)
                    } else {
                        h.call(m, validate, &[value])
                    };
                    if valid == 0 {
                        return normalization::code_4(0);
                    }
                } else if m.byte(p + 21) != 0 && m.word(p + 4) != value {
                    return normalization::code_4(0);
                }
                let out = m.word(p + 12);
                if out != 0 {
                    m.put_word(out, value);
                } else {
                    m.put_word(p + 8, value);
                }
                if pass == 0 {
                    let p = fn_1ba8e0(m, m.word(s + 8), Key::Local(key), e.compare, h);
                    m.put_word(p + 4, value);
                    byte(m, p + 21, 1);
                }
            } else if add_missing != 0 {
                let status = fn_1b35a0(m, s, key, value, e.accept, 0, 0, 0, e, h);
                if status != 0 {
                    return status;
                }
            }
            i = i.wrapping_add(1);
        }
    }
    if persistent_count == 0 {
        let mut i = 0u16;
        while m.word(m.word(s + 8) + 8) > i as u32 {
            let table = m.word(s + 8);
            let p = {
                let arena = crate::selection::arena::Arena::new(m.base, m.bytes);
                crate::selection::arena_leaf::word_array_at(&arena, table, i as u32)
            };
            if m.byte(p + 20) == 0 {
                m.put_word(p + 4, m.word(p + 8));
            }
            i = i.wrapping_add(1);
        }
    }
    0
}
pub fn fn_1b3ecc(m: &mut Memory<'_>, s: u32, data: u32, e: &Environment, h: &mut impl Host) -> u32 {
    fn_1b3c34(m, s, data, 0, e, h)
}
