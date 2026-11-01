

use crate::boot::Memory;

pub struct Tables {

    pub base_vtable: u32,

    pub v_2fac04: u32,

    pub v_2fbf1c: u32,

    pub v_302e38: u32,

    pub v_30a714: u32,

    pub v_30a73c: u32,

    pub v_30b480: u32,

    pub v_30b634: u32,

    pub v_30b7bc: u32,

    pub v_30d2e8: u32,

    pub v_30d370: u32,

    pub v_30d41c: u32,

    pub v_30d50c: u32,

    pub v_30d594: u32,
}

pub trait Host: crate::boot_part_f::Host {

    fn literal(&mut self, _m: &mut Memory<'_>, us: u32) -> u32 {
        us
    }

    fn stream_read(
        &mut self,
        m: &mut Memory<'_>,
        method: u32,
        this: u32,
        tag: u32,
        out: &mut u32,
    ) -> u32;

    fn stream_release(&mut self, m: &mut Memory<'_>, method: u32, this: u32, value: u32)
        -> u32;

    fn stream_blocks(
        &mut self,
        m: &mut Memory<'_>,
        method: u32,
        this: u32,
        allocator: u32,
        tag: u32,
        out1: u32,
        out2: &mut u32,
    ) -> u32;
}

pub const TAG_FSFN: u32 = 0x4e46_5346;
pub const TAG_FSBM: u32 = 0x4d42_5346;
pub const TAG_FSID: u32 = 0x4449_5346;
pub const TAG_FSEN: u32 = 0x4e45_5346;
pub const TAG_FSOF: u32 = 0x464f_5346;
pub const TAG_FSHA: u32 = 0x4148_5346;
pub const TAG_FSLI: u32 = 0x494c_5346;
pub const TAG_FSCO: u32 = 0x4f43_5346;
pub const TAG_FNEN: u32 = 0x4e45_4e46;
pub const TAG_FNST: u32 = 0x5453_4e46;
pub const TAG_FNHA: u32 = 0x4148_4e46;
pub const TAG_HUTR: u32 = 0x5254_5548;
pub const TAG_HULE: u32 = 0x454c_5548;

fn vt(m: &Memory<'_>, stream: u32) -> u32 {
    m.word(stream)
}
fn self_ptr(m: &Memory<'_>, stream: u32) -> u32 {
    stream.wrapping_sub(m.word(vt(m, stream) + 0x68))
}
fn m00(m: &mut Memory<'_>, h: &mut impl Host, stream: u32, tag: u32) -> (u32, u32) {
    let method = m.word(vt(m, stream));
    let this = self_ptr(m, stream);
    let mut value = 0u32;
    let rc = h.stream_read(m, method, this, tag, &mut value);
    (rc, value)
}
fn m08(m: &mut Memory<'_>, h: &mut impl Host, stream: u32, value: u32) -> u32 {
    let method = m.word(vt(m, stream) + 8);
    let this = self_ptr(m, stream);
    h.stream_release(m, method, this, value)
}
fn m18(
    m: &mut Memory<'_>,
    h: &mut impl Host,
    stream: u32,
    allocator: u32,
    tag: u32,
    out1: u32,
) -> (u32, u32) {
    let method = m.word(vt(m, stream) + 0x18);
    let this = self_ptr(m, stream);
    let mut size = 0u32;
    let rc = h.stream_blocks(m, method, this, allocator, tag, out1, &mut size);
    (rc, size)
}

fn put_byte(m: &mut Memory<'_>, p: u32, v: u8) {
    let at = m.offset(p);
    m.bytes[at] = v;
}

fn env(t: &Tables) -> crate::boot_part_f::Environment {
    crate::boot_part_f::Environment {
        stream_base_vtable: t.base_vtable,
        stream_vtable: 0,
        vector_vtable: 0,
        compare: 0,
        accept: 0,
    }
}

pub fn fn_30a714(m: &mut Memory<'_>, t: &Tables, p: u32) -> u32 {
    let rc = crate::boot_part_f::fn_1ba3c0(m, p, &env(t));
    if rc == 0 {
        m.put_word(p, t.v_30a714);
    }
    rc
}

pub fn fn_30b180(
    m: &mut Memory<'_>,
    h: &mut impl Host,
    t: &Tables,
    heap: u32,
    allocator: u32,
    value: u32,
    out: u32,
) -> u32 {
    let mut status = 0u8;
    let p = crate::boot_part_f::fn_1b2140(m, heap, 0x1c, &mut status, h);
    if status == 0 {
        let rc = fn_30a73c(m, h, t, p, allocator, value);
        status = rc as u8;
        if rc == 0 {
            m.put_word(out, p);
        } else {
            crate::boot_part_f::fn_1b2128(m, heap, p, h);
            m.put_word(out, 0);
        }
    }
    u32::from(status)
}

pub fn fn_30b480(
    m: &mut Memory<'_>,
    h: &mut impl Host,
    t: &Tables,
    heap: u32,
    allocator: u32,
    value: u32,
    out: u32,
) -> u32 {
    let mut status = 0u8;
    let p = crate::boot_part_f::fn_1b2140(m, heap, 0x20, &mut status, h);
    if status == 0 {
        let rc = fn_30b7bc(m, h, t, p, allocator, value);
        if rc == 0 {
            m.put_word(p, t.v_30b480);
        }
        status = rc as u8;
        if rc == 0 {
            m.put_word(out, p);
        } else {
            crate::boot_part_f::fn_1b2128(m, heap, p, h);
            m.put_word(out, 0);
        }
    }
    u32::from(status)
}

pub fn fn_30b634(
    m: &mut Memory<'_>,
    h: &mut impl Host,
    t: &Tables,
    heap: u32,
    data: u32,
    out: u32,
) -> u32 {
    let mut status = 0u8;
    let p = crate::boot_part_f::fn_1b2140(m, heap, 0x10, &mut status, h);
    if status != 0 {
        return u32::from(status);
    }
    let rc = crate::boot_part_f::fn_1ba3c0(m, p, &env(t));
    if rc == 0 {
        m.put_word(p, t.v_30b634);
        let b = |m: &Memory<'_>, off: u32| m.byte(data + off);
        m.put_half(p + 8, u16::from_be_bytes([b(m, 0), b(m, 1)]));
        m.put_word(
            p + 4,
            (u32::from(b(m, 7)) << 24)
                | (u32::from(b(m, 8)) << 16)
                | (u32::from(b(m, 9)) << 8)
                | u32::from(b(m, 10)),
        );
        m.put_half(p + 0xa, u16::from_be_bytes([b(m, 4), b(m, 5)]));
        put_byte(m, p + 0xc, b(m, 2));
        put_byte(m, p + 0xd, b(m, 3));
        put_byte(m, p + 0xe, b(m, 6));
    }
    status = rc as u8;
    if rc == 0 {
        m.put_word(out, p);
    } else {
        crate::boot_part_f::fn_1b2128(m, heap, p, h);
        m.put_word(out, 0);
    }
    u32::from(status)
}

pub fn fn_30a73c(
    m: &mut Memory<'_>,
    h: &mut impl Host,
    t: &Tables,
    obj: u32,
    allocator: u32,
    stream: u32,
) -> u32 {
    let mut rc = crate::boot_part_f::fn_1ba3c0(m, obj, &env(t));
    if rc == 0 {
        m.put_word(obj, t.v_30a73c);
        m.put_word(obj + 0x18, allocator);
        let (r, s) = m18(m, h, stream, allocator, TAG_FNEN, obj + 8);
        rc = r;
        if rc == 0 {
            m.put_word(obj + 4, s >> 1);
            let (r, _) = m18(m, h, stream, m.word(obj + 0x18), TAG_FNST, obj + 0xc);
            rc = r;
            if rc == 0 {
                let (r, s) = m18(m, h, stream, m.word(obj + 0x18), TAG_FNHA, obj + 0x14);
                rc = r;
                if rc == 0 {
                    let quotient = crate::normalization::udiv(m.word(obj + 4) * 4, s);
                    m.put_word(obj + 0x10, quotient);
                    if quotient != 1 && quotient != 2 {
                        rc = 4;
                    }
                }
            }
        }
    }
    rc
}

pub fn fn_30d370(
    m: &mut Memory<'_>,
    t: &Tables,
    obj: u32,
    data: u32,
    count: u32,
    width: u32,
) -> u32 {
    let rc = crate::boot_part_f::fn_1ba3c0(m, obj, &env(t));
    if rc == 0 {
        m.put_word(obj + 4, count.wrapping_sub(1));
        m.put_word(obj + 8, count.wrapping_sub(1));
        m.put_word(obj + 0xc, data);
        m.put_word(obj + 0x10, width);
        m.put_word(obj, t.v_30d370);
    }
    rc
}

pub fn fn_30d594(m: &mut Memory<'_>, t: &Tables, obj: u32) -> u32 {
    let rc = crate::boot_part_f::fn_1ba3c0(m, obj, &env(t));
    if rc == 0 {
        m.put_word(obj, t.v_30d594);
    }
    rc
}

pub fn fn_30d41c(
    m: &mut Memory<'_>,
    h: &mut impl Host,
    t: &Tables,
    allocator: u32,
    data: u32,
    count: u32,
    out: u32,
) -> u32 {
    let mut status = 0u8;
    let p = crate::boot_part_f::fn_1b2140(m, allocator, 0x14, &mut status, h);
    if status == 0 {
        let rc = fn_30d370(m, t, p, data, count, 8);
        if rc == 0 {
            m.put_word(p, t.v_30d41c);
            m.put_word(out, p);
        } else {
            crate::boot_part_f::fn_1b2128(m, allocator, p, h);
            m.put_word(out, 0);
        }
        status = rc as u8;
    }
    u32::from(status)
}

pub fn fn_30d50c(
    m: &mut Memory<'_>,
    h: &mut impl Host,
    t: &Tables,
    allocator: u32,
    data: u32,
    count: u32,
    out: u32,
) -> u32 {
    let mut status = 0u8;
    let p = crate::boot_part_f::fn_1b2140(m, allocator, 0x14, &mut status, h);
    if status == 0 {
        let rc = fn_30d370(m, t, p, data, count, 0x10);
        if rc == 0 {
            m.put_word(p, t.v_30d50c);
            m.put_word(out, p);
        } else {
            crate::boot_part_f::fn_1b2128(m, allocator, p, h);
            m.put_word(out, 0);
        }
        status = rc as u8;
    }
    u32::from(status)
}

pub fn fn_30d2e8(
    m: &mut Memory<'_>,
    h: &mut impl Host,
    t: &Tables,
    allocator: u32,
    data: u32,
    count: u32,
    value: u32,
    out: u32,
) -> u32 {
    let mut status = 0u8;
    let p = crate::boot_part_f::fn_1b2140(m, allocator, 0x10, &mut status, h);
    if status == 0 {
        let rc = fn_30d594(m, t, p);
        if rc == 0 {
            m.put_word(p, t.v_30d2e8);
            m.put_word(p + 4, data);
            m.put_word(p + 8, count);
            m.put_word(p + 0xc, value);
        }
        status = rc as u8;
        if rc == 0 {
            m.put_word(out, p);
        } else {
            crate::boot_part_f::fn_1b2128(m, allocator, p, h);
            m.put_word(out, 0);
        }
    }
    u32::from(status)
}

pub fn fn_30b7bc(
    m: &mut Memory<'_>,
    h: &mut impl Host,
    t: &Tables,
    obj: u32,
    allocator: u32,
    stream: u32,
) -> u32 {
    let mut rc = crate::boot_part_f::fn_1ba3c0(m, obj, &env(t));
    if rc == 0 {
        m.put_word(obj, t.v_30b7bc);
        m.put_word(obj + 0x1c, allocator);
        let (r, _) = m18(m, h, stream, allocator, TAG_HUTR, obj + 4);
        rc = r;
        if rc == 0 {
            let table = m.word(obj + 4);
            let kind = u16::from_le_bytes([m.byte(table + 2), m.byte(table + 3)]);
            let count = u16::from_le_bytes([m.byte(table), m.byte(table + 1)]);
            if kind != 8 && kind != 16 {
                rc = 4;
            } else {
                if kind == 8 {
                    rc = fn_30d41c(m, h, t, allocator, table + 4, u32::from(count), obj + 0xc);
                } else {
                    rc = fn_30d50c(m, h, t, allocator, table + 4, u32::from(count), obj + 0xc);
                }
                if rc == 0 {
                    let (r, s) = m18(m, h, stream, m.word(obj + 0x1c), TAG_HULE, obj + 0x10);
                    rc = r;
                    if rc == 0 {
                        let codes = m.word(obj + 0x10);
                        let width = u16::from_le_bytes([m.byte(codes), m.byte(codes + 1)]);
                        let quotient = crate::normalization::udiv(
                            s.wrapping_sub(2) * 8,
                            u32::from(width),
                        );
                        rc = fn_30d2e8(
                            m,
                            h,
                            t,
                            allocator,
                            codes + 2,
                            u32::from(width),
                            quotient,
                            obj + 0x18,
                        );
                    }
                }
            }
        }
    }
    rc
}

pub fn fn_2fbf1c(
    m: &mut Memory<'_>,
    h: &mut impl Host,
    t: &Tables,
    obj: u32,
    allocator: u32,
    stream: u32,
) -> u32 {
    let rc = fn_30a714(m, t, obj);
    if rc != 0 {
        return rc;
    }
    m.put_word(obj, t.v_2fbf1c);
    m.put_word(obj + 0x30, allocator);
    let (rc, value) = m00(m, h, stream, TAG_FSFN);
    if rc != 0 {
        return rc;
    }
    let rc = fn_30b180(
        m,
        h,
        t,
        m.word(obj + 0x30),
        m.word(obj + 0x30),
        value,
        obj + 4,
    );
    if rc != 0 {
        return rc;
    }
    let rc = m08(m, h, stream, value);
    if rc != 0 {
        return rc;
    }
    let (rc, size) = m18(m, h, stream, m.word(obj + 0x30), TAG_FSBM, obj + 8);
    if rc != 0 {
        return rc;
    }
    m.put_word(obj + 0x10, size.wrapping_mul(8));
    let rc = fn_302e38(m, h, t, m.word(obj + 0x30), m.word(obj + 8), obj + 0xc);
    if rc != 0 {
        return rc;
    }
    let tags = [TAG_FSID, TAG_FSEN, TAG_FSOF, TAG_FSHA, TAG_FSLI];
    for (i, tag) in tags.iter().enumerate() {
        let (rc, value) = m00(m, h, stream, *tag);
        if rc != 0 {
            return rc;
        }
        let slot = obj + 0x14 + 4 * i as u32;
        let rc = fn_30b480(
            m,
            h,
            t,
            m.word(obj + 0x30),
            m.word(obj + 0x30),
            value,
            slot,
        );
        if rc != 0 {
            return rc;
        }
        let rc = m08(m, h, stream, value);
        if rc != 0 {
            return rc;
        }
    }
    let (rc, _) = m18(m, h, stream, m.word(obj + 0x30), TAG_FSCO, obj + 0x28);
    if rc != 0 {
        return rc;
    }
    fn_30b634(m, h, t, m.word(obj + 0x30), m.word(obj + 0x28), obj + 0x2c)
}

pub fn fn_302e38(
    m: &mut Memory<'_>,
    h: &mut impl Host,
    t: &Tables,
    allocator: u32,
    data: u32,
    out: u32,
) -> u32 {
    let mut status = 0u8;
    let p = crate::boot_part_f::fn_1b2140(m, allocator, 0x10, &mut status, h);
    if status != 0 {
        return u32::from(status);
    }
    let rc = crate::boot_part_f::fn_1ba3c0(m, p, &env(t));
    if rc == 0 {
        m.put_word(p + 4, data);
        m.put_word(p + 8, 0);
        m.put_word(p, t.v_302e38);
        m.put_word(p + 0xc, 0);
        let mut n = 4u32;
        while n != 0 {
            m.put_word(p + 0xc, m.word(p + 0xc).wrapping_add(1));
            n >>= 1;
        }
    }
    status = rc as u8;
    if rc == 0 {
        m.put_word(out, p);
    } else {
        crate::boot_part_f::fn_1b2128(m, allocator, p, h);
        m.put_word(out, 0);
    }
    u32::from(status)
}

pub fn fn_2fac04(
    m: &mut Memory<'_>,
    h: &mut impl Host,
    t: &Tables,
    allocator: u32,
    value: u32,
    size: u32,
    out: u32,
) -> u32 {
    let mut status = 0u8;
    let p = crate::boot_part_f::fn_1b2140(m, allocator, 8, &mut status, h);
    if status != 0 {
        return u32::from(status);
    }
    let mut rc = crate::boot_part_f::fn_1ba3c0(m, p, &env(t));
    if rc == 0 {
        m.put_word(p, t.v_2fac04);
        m.put_word(p + 4, value);
        if size != 0x18 {
            rc = 3;
        }
    }
    status = rc as u8;
    if rc == 0 {
        m.put_word(out, p);
    } else {
        crate::boot_part_f::fn_1b2128(m, allocator, p, h);
        m.put_word(out, 0);
    }
    u32::from(status)
}

pub fn fn_2fd9a0(
    m: &mut Memory<'_>,
    h: &mut impl Host,
    t: &Tables,
    heap: u32,
    allocator: u32,
    stream: u32,
    out: u32,
) -> u32 {
    let mut status = 0u8;
    let p = crate::boot_part_f::fn_1b2140(m, heap, 0x34, &mut status, h);
    if status == 0 {
        let rc = fn_2fbf1c(m, h, t, p, allocator, stream);
        status = rc as u8;
        if rc == 0 {
            m.put_word(out, p);
        } else {
            crate::boot_part_f::fn_1b2128(m, heap, p, h);
            m.put_word(out, 0);
        }
    }
    u32::from(status)
}
