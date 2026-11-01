
use crate::boot::Memory;
use crate::boot_part_b::Host as CallHost;

pub trait Host: crate::render_part_a::ReaderHost {}
impl<T: crate::render_part_a::ReaderHost> Host for T {}
pub struct Environment {
    pub reader: crate::render_part_a::ReaderRuntime,

    pub tags: [u32; 4],
    pub node_tags: [u32; 12],
    pub sub_tag: u32,
    pub detach_messages: [u32; 2],
    pub close_messages: [u32; 2],

    pub scratch: u32,
}
struct Heap<'a, H>(&'a mut H);
impl<H: CallHost> crate::boot_part_e::Host for Heap<'_, H> {
    fn call(&mut self, m: &mut Memory<'_>, f: u32, a: &[u32]) -> u32 {
        self.0.call(m, f, a)
    }
}
struct Node<'a, H>(&'a mut H);
impl<H: CallHost> crate::boot_port_5::Host for Node<'_, H> {
    fn call(&mut self, m: &mut Memory<'_>, f: u32, a: &[u32]) -> u32 {
        self.0.call(m, f, a)
    }
}
struct Log<'a, H>(&'a mut H);
impl<H: CallHost> crate::log_live::LogMethodsM for Log<'_, H> {
    fn log_m(
        &mut self,
        f: u32,
        o: u32,
        c: u32,
        l: u32,
        a: u32,
        fmt: u32,
        va: u32,
        m: &mut Memory<'_>,
    ) -> u32 {
        self.0.call(m, f, &[o, c, l, a, fmt, va])
    }
}
fn alloc(m: &mut Memory<'_>, h: &mut impl CallHost, ctx: u32, n: u32) -> u32 {
    crate::boot_part_e::calloc(m, m.word(m.word(ctx) + 4), 1, n, &mut Heap(h))
}
fn free(m: &mut Memory<'_>, h: &mut impl CallHost, heap: u32, p: u32) {
    crate::boot_part_e::release(m, heap, p, &mut Heap(h));
}
fn release(m: &mut Memory<'_>, h: &mut impl CallHost, ctx: u32, p: u32) {
    free(m, h, m.word(m.word(ctx) + 4), p)
}
fn error(m: &mut Memory<'_>, h: &mut impl Host, ctx: u32, code: u32, e: &Environment) {
    crate::render_part_a::error_log(
        m,
        h,
        m.word(m.word(ctx) + 16),
        m.word(ctx + 72),
        code,
        0,
        0,
        e.scratch + 128,
    );
}
fn trace(m: &mut Memory<'_>, h: &mut impl CallHost, logger: u32, module: u32, fmt: u32, args: u32) {
    crate::log_live::fn_1738e8(m, logger, module, 4, 0, fmt, args, &mut Log(h));
}
fn copy(m: &mut Memory<'_>, d: u32, s: u32, n: u32) {
    let d = m.offset(d) as u32;
    let s = m.offset(s) as u32;
    crate::fe_text::memcpy_195ae8(m.bytes, d, s, n);
}
fn byte(m: &mut Memory<'_>, p: u32, v: u8) {
    let at = m.offset(p);
    m.bytes[at] = v;
}
fn half(m: &Memory<'_>, p: u32) -> u16 {
    m.half(p) as u16
}
fn saved(m: &Memory<'_>, e: &Environment) -> [u8; 1024] {
    m.bytes[m.offset(e.scratch)..m.offset(e.scratch) + 1024]
        .try_into()
        .unwrap()
}
fn restore(m: &mut Memory<'_>, e: &Environment, s: [u8; 1024]) {
    let at = m.offset(e.scratch);
    m.bytes[at..at + 1024].copy_from_slice(&s);
}

pub fn fn_250a3c(m: &mut Memory<'_>, h: &mut impl Host, ctx: u32, e: &Environment) -> u32 {
    let save = saved(m, e);
    let f0 = m.word(ctx);
    let module = m.word(ctx + 72);
    let logger = m.word(f0 + 16);
    let r = m.word(m.word(ctx + 56) + 8);
    let result = if r == 0 {
        crate::render_part_a::error_log(m, h, logger, module, 0x5df1, 0, 0, e.scratch + 128);
        crate::normalization::forward_error(7)
    } else {
        trace(m, h, logger, module, e.close_messages[0], 0);
        let arr = m.word(r + 4);
        if arr != 0 {
            let mut i = 0u16;
            while half(m, r) > i {
                let p = arr + i as u32 * 4;
                if m.word(p) != 0 {
                    fn_24f124(m, h, ctx, m.word(p), e);
                    m.put_word(p, 0);
                }
                i = i.wrapping_add(1);
            }
            free(m, h, m.word(f0 + 4), arr);
            m.put_word(r + 4, 0);
        }
        m.put_word(e.scratch, 0);
        trace(m, h, logger, module, e.close_messages[1], e.scratch);
        free(m, h, m.word(f0 + 4), r);
        0
    };
    restore(m, e, save);
    result
}

pub fn fn_24f124(
    m: &mut Memory<'_>,
    h: &mut impl Host,
    ctx: u32,
    slot: u32,
    e: &Environment,
) -> u32 {
    let a = m.word(slot + 4);
    let b = m.word(slot + 8);
    let c = m.word(slot + 12);
    let mut i = 0u16;
    if a != 0 && matches!(m.byte(slot), 0 | 2 | 3 | 5) {
        release(m, h, ctx, m.word(a));
        m.put_word(a, 0);
        if m.word(a + 24) != 0 {
            while half(m, a + 4) > i {
                let row = m.word(a + 24) + i as u32 * 24;
                for off in [8, 12] {
                    if m.word(row + off) != 0 {

                        h.call(m, 0x2b6234, &[m.word(ctx), m.word(row + off), 1]);
                        release(m, h, ctx, m.word(row + off));
                        m.put_word(row + off, 0);
                    }
                }
                i = i.wrapping_add(1);
            }
            release(m, h, ctx, m.word(a + 24));
            m.put_word(a + 24, 0);
        }
        if m.word(a + 28) != 0 {
            release(m, h, ctx, m.word(a + 28));
            m.put_word(a + 28, 0);
        }
        release(m, h, ctx, m.word(slot + 4));
        m.put_word(slot + 4, 0);
    }
    if b != 0 && matches!(m.byte(slot), 1 | 2 | 4 | 5) {
        if m.word(b) != 0 {
            release(m, h, ctx, m.word(b));
            m.put_word(b, 0);
        }
        if m.word(b + 24) != 0 {
            i = 0;
            while half(m, b + 22) > i {
                let p = m.word(b + 24) + i as u32 * 4;
                release(m, h, ctx, m.word(p));
                m.put_word(p, 0);
                i = i.wrapping_add(1);
            }
            release(m, h, ctx, m.word(b + 24));
            m.put_word(b + 24, 0);
        }
        if m.word(b + 8) != 0 {
            i = 0;
            while half(m, b + 12) > i {
                let row = m.word(b + 8) + i as u32 * 28;
                release(m, h, ctx, m.word(row + 20));
                m.put_word(row + 20, 0);
                if m.word(row + 12) != 0 {
                    release(m, h, ctx, m.word(row + 12));
                    m.put_word(row + 12, 0);
                }
                i = i.wrapping_add(1);
            }
            if m.word(b + 16) != 0 {
                i = 0;
                while half(m, b + 20) > i {
                    let row = m.word(b + 16) + i as u32 * 8;
                    if m.word(row) != 0 {
                        release(m, h, ctx, m.word(row));
                        m.put_word(row, 0);
                    }
                    i = i.wrapping_add(1);
                }
                release(m, h, ctx, m.word(b + 16));
                m.put_word(b + 16, 0);
            }
            release(m, h, ctx, m.word(b + 8));
            m.put_word(b + 8, 0);
        }
        release(m, h, ctx, m.word(slot + 8));
        m.put_word(slot + 8, 0);
    }
    if c != 0 {
        if m.word(c + 4) != 0 {
            i = 0;
            while half(m, c + 8) > i {
                let row = m.word(c + 4) + i as u32 * 4;
                if m.word(row) != 0 {
                    let v = m.word(row);
                    if m.word(v) != 0 {
                        release(m, h, ctx, m.word(v));
                        m.put_word(m.word(row), 0);
                    }
                    if m.word(m.word(row) + 12) != 0 {
                        let mut j = 0u16;
                        while half(m, m.word(row) + 8) > j {
                            let p = m.word(m.word(row) + 12) + j as u32 * 4;
                            if m.word(p) != 0 {
                                release(m, h, ctx, m.word(p));
                                m.put_word(m.word(m.word(row) + 12) + j as u32 * 4, 0);
                            }
                            j = j.wrapping_add(1);
                        }
                        release(m, h, ctx, m.word(m.word(row) + 12));
                        m.put_word(m.word(row) + 12, 0);
                    }
                    m.put_half(m.word(row) + 8, 0);
                    release(m, h, ctx, m.word(row));
                    m.put_word(row, 0);
                }
                i = i.wrapping_add(1);
            }
            release(m, h, ctx, m.word(c + 4));
            m.put_word(c + 4, 0);
        }
        m.put_half(c + 8, 0);
        release(m, h, ctx, c);
    }
    if m.word(slot + 16) != 0 {
        crate::render_part_d::release_chunk(m.bytes, m.word(slot + 16), h);
        m.put_word(slot + 16, 0);
        m.put_word(slot + 20, 0);
    }
    release(m, h, ctx, slot);
    let _ = e;
    0
}

enum Abort {
    Fail(u32),
    Unwind(u32),
}
type ReadResult<T> = Result<T, Abort>;
fn checked(rc: u32) -> ReadResult<u32> {
    if rc >= 0x80000000 {
        Err(Abort::Fail(rc))
    } else {
        Ok(rc)
    }
}
struct Loader<'a, 'b, 'c, H> {
    m: &'a mut Memory<'b>,
    h: &'a mut H,
    ctx: u32,
    rd: u32,
    e: &'c Environment,
    base: u32,
    size: u32,
    off: u32,
    sl: u8,
    rc: u32,
}
impl<H: Host> Loader<'_, '_, '_, H> {
    fn open(&mut self, tag: usize) -> ReadResult<()> {
        let s = self.e.scratch;
        self.m.put_word(s, self.size);
        self.m.put_word(s + 4, self.base);
        let rc = crate::render_part_a::find_reader_chunk(
            self.m,
            self.h,
            self.rd,
            self.e.tags[tag],
            1,
            s,
            s + 4,
            &self.e.reader,
        );
        self.size = self.m.word(s);
        self.base = self.m.word(s + 4);
        self.rc = rc;
        checked(rc)?;
        self.off = 0;
        Ok(())
    }
    fn close(&mut self) -> ReadResult<()> {
        self.rc = crate::render_part_a::close_reader_chunk(self.m, self.h, self.rd, &self.e.reader);
        checked(self.rc)?;
        Ok(())
    }
    fn u16(&mut self) -> u16 {
        let v = half(self.m, self.base.wrapping_add(self.off));
        self.off = self.off.wrapping_add(2);
        v
    }
    fn u32(&mut self) -> u32 {
        let v = self.m.word(self.base.wrapping_add(self.off));
        self.off = self.off.wrapping_add(4);
        v
    }
    fn h16(&mut self, p: u32) {
        let v = self.u16();
        self.m.put_half(p, v);
    }
    fn b16(&mut self, p: u32) {
        let v = self.u16();
        byte(self.m, p, v as u8);
    }
    fn field_alloc(&mut self, field: u32, n: u32) -> ReadResult<u32> {

        let p = alloc(self.m, self.h, self.ctx, n);
        self.m.put_word(field, p);
        if p == 0 {
            error(self.m, self.h, self.ctx, 0x5df0, self.e);
            Err(Abort::Fail(crate::normalization::forward_error(10)))
        } else {
            Ok(p)
        }
    }
    fn element_alloc(&mut self, array_field: u32, index: u16, n: u32) -> ReadResult<u32> {
        let p = alloc(self.m, self.h, self.ctx, n);

        self.m
            .put_word(self.m.word(array_field) + index as u32 * 4, p);
        if p == 0 {
            error(self.m, self.h, self.ctx, 0x5df0, self.e);
            Err(Abort::Fail(crate::normalization::forward_error(10)))
        } else {
            Ok(p)
        }
    }
    fn array(&mut self, field: u32, count: u32, width: u32, reset: Option<u32>) -> ReadResult<()> {
        let p = alloc(self.m, self.h, self.ctx, count.wrapping_mul(width));
        self.m.put_word(field, p);
        if p == 0 {
            error(self.m, self.h, self.ctx, 0x5df0, self.e);
            if let Some(p) = reset {
                self.m.put_half(p, 0);
            }
            return Err(Abort::Fail(crate::normalization::forward_error(10)));
        }
        Ok(())
    }
    fn string(&mut self, dest: u32, n: u16) -> ReadResult<()> {
        let s = self.e.scratch + 8;
        self.m.put_word(s, n as u32);
        self.rc = crate::render_part_a::read_chunk_string(
            &mut crate::selection::arena::Arena::new(self.m.base, self.m.bytes),
            self.rd,
            self.base,
            self.size,
            self.off,
            dest,
            s,
        );
        checked(self.rc)?;
        if self.m.word(s) != n as u32 {
            return Err(Abort::Unwind(self.rc));
        }
        self.off = self.off.wrapping_add(n as u32);
        Ok(())
    }
    fn node(&mut self, field: u32) -> ReadResult<()> {
        let sub = self.field_alloc(field, 28)?;
        self.m.put_word(sub, self.e.sub_tag);
        byte(self.m, sub + 12, 0);
        for off in [8, 20, 16, 24] {
            self.m.put_word(sub + off, 0);
        }
        let s = self.e.scratch + 12;
        self.m.put_word(s, self.off);
        self.rc = crate::boot_port_5::fn_2b6674(
            self.m,
            &mut Node(self.h),
            self.ctx,
            self.rd,
            self.m.word(self.ctx),
            sub,
            self.base,
            s,
            self.size,
            self.e.node_tags,
            self.e.scratch + 64,
        );
        self.off = self.m.word(s);
        checked(self.rc)?;
        Ok(())
    }
    fn load(&mut self, slot: u32, d: u32, extra: u32) -> ReadResult<u32> {
        self.open(0)?;
        let t = self.u16();
        byte(self.m, slot, t as u8);
        self.close()?;
        if matches!(self.m.byte(slot), 3 | 4 | 5) {
            let c = self.field_alloc(slot + 12, 12)?;
            self.open(1)?;
            self.sl = 1;
            self.b16(c);
            if self.m.byte(c) == 0 {
                self.h16(c + 8);
                if half(self.m, c + 8) != 0 {
                    self.array(c + 4, half(self.m, c + 8) as u32, 4, Some(c + 8))?;
                    let mut i = 0u16;
                    while half(self.m, c + 8) > i {
                        self.element_alloc(c + 4, i, 16)?;
                        i = i.wrapping_add(1);
                    }
                }
                let mut k = 0u16;
                while half(self.m, c + 8) > k {
                    let dd = self.m.word(self.m.word(c + 4) + k as u32 * 4);
                    let n = self.u16().wrapping_add(1);
                    let p = self.field_alloc(dd, n as u32)?;
                    self.string(p, n)?;
                    self.h16(dd + 4);
                    self.h16(dd + 6);
                    self.h16(dd + 8);
                    if half(self.m, dd + 8) != 0 {
                        self.array(dd + 12, half(self.m, dd + 8) as u32, 4, Some(dd + 8))?;
                        let mut j = 0u16;
                        while half(self.m, dd + 8) > j {
                            let n = self.u16().wrapping_add(1);
                            let p = self.element_alloc(dd + 12, j, n as u32)?;
                            self.string(p, n)?;
                            j = j.wrapping_add(1);
                        }
                    }
                    k = k.wrapping_add(1);
                }
            }
            self.sl = self.sl.wrapping_sub(1);
            self.close()?;
        }
        if matches!(self.m.byte(slot), 0 | 2 | 3 | 5) {
            self.field_alloc(slot + 4, 32)?;
        }
        if matches!(self.m.byte(slot), 1 | 2 | 4 | 5) {
            self.field_alloc(slot + 8, 28)?;
        }
        if matches!(self.m.byte(slot), 0 | 2 | 3 | 5) {
            let a = self.m.word(slot + 4);
            self.open(2)?;
            self.sl = self.sl.wrapping_add(1);
            self.m.put_word(slot + 20, self.base);
            let n = self.u16().wrapping_add(1);
            let p = self.field_alloc(a, n as u32)?;
            self.string(p, n)?;
            self.h16(a + 4);
            self.h16(a + 6);
            let v34 = self.u32();
            let v38 = self.u32();
            self.m.put_word(a + 8, self.off);
            self.m.put_word(a + 12, self.base.wrapping_add(self.off));
            self.off = self.off.wrapping_add(v34);
            let v3c = self.u32();
            self.off = self.off.wrapping_add(v3c);
            self.m.put_word(a + 20, self.off);
            self.m.put_word(a + 16, self.base.wrapping_add(self.off));
            self.off = self.off.wrapping_add(v38);
            if half(self.m, a + 4) != 0 {
                self.array(a + 24, half(self.m, a + 4) as u32, 24, None)?;
            }
            let mut j = 0u16;
            while half(self.m, a + 4) > j {
                let row = self.m.word(a + 24) + j as u32 * 24;
                self.b16(row);
                self.h16(row + 2);
                self.h16(row + 4);
                self.h16(row + 16);
                self.h16(row + 18);
                self.h16(row + 20);
                self.m.put_word(row + 8, 0);
                if half(self.m, row + 16) != 0 {
                    self.node(row + 8)?;
                }
                self.m.put_word(row + 12, 0);
                if half(self.m, row + 18) != 0 {
                    self.node(row + 12)?;
                }
                j = j.wrapping_add(1);
            }
            if half(self.m, a + 6) != 0 {
                self.array(a + 28, half(self.m, a + 6) as u32, 16, None)?;
            }
            j = 0;
            while half(self.m, a + 6) > j {
                let row = self.m.word(a + 28) + j as u32 * 16;
                let value = if d == 1 {
                    self.u32()
                } else {
                    self.u16() as u32
                };
                self.m.put_word(row + 4, value);
                self.h16(row + 8);
                self.h16(row + 10);
                self.h16(row);
                self.b16(row + 12);
                j = j.wrapping_add(1);
            }
            let s = self.e.scratch + 4;
            self.m.put_word(s, self.base);
            self.rc = fn_243ad4(self.m, self.h, self.rd, slot + 16, s, self.e);
            self.base = self.m.word(s);
            checked(self.rc)?;
            self.sl = self.sl.wrapping_sub(1);
            self.m.put_word(slot + 20, self.base);
            self.m
                .put_word(a + 12, self.base.wrapping_add(self.m.word(a + 8)));
            self.m
                .put_word(a + 16, self.base.wrapping_add(self.m.word(a + 20)));
            self.close()?;
        }
        if matches!(self.m.byte(slot), 1 | 2 | 4 | 5) {
            let b = self.m.word(slot + 8);
            self.open(3)?;
            self.sl = self.sl.wrapping_add(1);
            let n = self.u16().wrapping_add(1);
            let p = self.field_alloc(b, n as u32)?;
            self.string(p, n)?;
            self.b16(b + 4);
            self.h16(b + 12);
            self.h16(b + 22);
            self.m.put_word(b + 24, 0);
            if half(self.m, b + 22) != 0 {
                self.array(b + 24, half(self.m, b + 22) as u32, 4, None)?;
            }
            let mut k = 0u16;
            while half(self.m, b + 22) > k {
                let n = self.u16().wrapping_add(1);
                let p = self.element_alloc(b + 24, k, n as u32)?;
                self.string(p, n)?;
                k = k.wrapping_add(1);
            }
            if half(self.m, b + 12) != 0 {
                self.array(b + 8, half(self.m, b + 12) as u32, 28, None)?;
            }
            k = 0;
            while half(self.m, b + 12) > k {
                let row = self.m.word(b + 8) + k as u32 * 28;
                let n = self.u16();
                self.m.put_half(row + 24, n);
                let p = self.field_alloc(row + 20, n as u32 + 1)?;
                copy(self.m, p, self.base.wrapping_add(self.off), n as u32);
                byte(self.m, p + n as u32, 0);
                self.off = self.off.wrapping_add(n as u32);
                self.h16(row);
                self.h16(row + 2);
                self.b16(row + 26);
                self.b16(row + 27);
                self.b16(row + 6);
                if self.m.byte(row + 6) == 1 {
                    self.h16(row + 4);
                    self.h16(row + 8);
                }
                if extra == 0 {
                    self.h16(row + 16);
                    let mut i = 0u16;
                    while half(self.m, row + 16) > i {
                        self.u16();
                        self.u16();
                        i = i.wrapping_add(1);
                    }
                }
                k = k.wrapping_add(1);
            }
            self.h16(b + 20);
            if half(self.m, b + 20) != 0 {
                self.array(b + 16, half(self.m, b + 20) as u32, 8, None)?;
            }
            let mut idx = 0u32;
            while half(self.m, b + 20) > idx as u16 {
                let row = self.m.word(b + 16) + idx * 8;
                self.h16(row + 4);
                if half(self.m, row + 4) != 0 {
                    self.array(row, half(self.m, row + 4) as u32, 6, None)?;
                    let mut j = 0u16;
                    while half(self.m, row + 4) > j {
                        let p = self.m.word(row) + j as u32 * 6;
                        self.m.put_half(p + 2, 0);
                        if extra == 0 {
                            self.h16(p);
                            self.h16(p + 4);
                        } else {
                            let a = self.m.byte(self.base.wrapping_add(self.off));
                            self.off = self.off.wrapping_add(1);
                            self.m.put_half(p, a as u16);
                            let b = self.m.byte(self.base.wrapping_add(self.off));
                            self.off = self.off.wrapping_add(1);
                            self.m.put_half(p + 4, b as u16);
                        }
                        j = j.wrapping_add(1);
                    }
                }
                idx = idx.wrapping_add(1);
            }
            self.sl = self.sl.wrapping_sub(1);
            self.close()?;
        }
        Ok(self.rc)
    }
}

pub fn fn_24f570(
    m: &mut Memory<'_>,
    h: &mut impl Host,
    ctx: u32,
    rd: u32,
    slot: u32,
    d: u32,
    extra: u32,
    e: &Environment,
) -> u32 {
    let save = saved(m, e);
    let mut l = Loader {
        m,
        h,
        ctx,
        rd,
        e,
        base: 0,
        size: 0,
        off: 0,
        sl: 0,
        rc: 0,
    };
    let rc = match l.load(slot, d, extra) {
        Ok(r) | Err(Abort::Unwind(r)) => r,
        Err(Abort::Fail(r)) => {
            error(l.m, l.h, ctx, 0x5df4, e);
            r
        }
    };
    while l.sl != 0 {
        let _ = crate::render_part_a::close_reader_chunk(l.m, l.h, rd, &e.reader);
        l.sl = l.sl.wrapping_sub(1);
    }
    restore(l.m, e, save);
    rc
}

pub fn fn_243ad4(
    m: &mut Memory<'_>,
    h: &mut impl Host,
    rd: u32,
    out: u32,
    data: u32,
    e: &Environment,
) -> u32 {
    if rd == 0 || out == 0 || data == 0 {
        return 0x8a202007;
    }
    if m.word(rd + 40) == 0 {
        return 0x8a202011;
    }
    let save = saved(m, e);
    let va = e.scratch + 40;
    m.put_word(va, rd);
    trace(
        m,
        h,
        m.word(m.word(rd + 8) + 16),
        e.reader.category,
        e.detach_messages[0],
        va,
    );
    let p = crate::boot_part_e::calloc(m, m.word(m.word(rd + 8) + 4), 1, 28, &mut Heap(h));
    m.put_word(out, p);
    let mut rc = 0;
    if p == 0 {
        crate::render_part_a::error_log(
            m,
            h,
            m.word(m.word(rd + 8) + 16),
            e.reader.category,
            0x6978,
            0,
            0,
            e.scratch + 128,
        );
        rc = 0x8a20200a;
    } else {
        let s = e.scratch + 32;
        m.put_word(s, 0);
        if crate::normalization::chunk_last(m.bytes, m.word(rd + 20), s) == 0 {
            m.put_word(va, e.reader.file_key);
            m.put_word(va + 4, m.word(rd + 16));
            crate::render_part_a::error_log(
                m,
                h,
                m.word(m.word(rd + 8) + 16),
                e.reader.category,
                0x6985,
                e.reader.pair_format,
                va,
                e.scratch + 128,
            );
            rc = 0x8a202000;
        } else {
            let chunk = m.word(s);
            if !(m.byte(rd + 28) == 0
                && m.word(rd + 44) == m.word(rd + 52)
                && m.word(rd + 48) == m.word(chunk + 12))
            {
                rc = crate::render_part_a::map_reader_window(
                    m,
                    h,
                    rd,
                    m.word(rd + 52),
                    m.word(chunk + 12).wrapping_sub(m.word(rd + 52)),
                    rd + 24,
                    &e.reader,
                );
            }
            if rc < 0x80000000 {
                match m.byte(rd + 12) {
                    0 => {
                        crate::normalization::data_freeze(
                            m.bytes,
                            m.word(m.word(rd + 8) + 12),
                            m.word(rd + 32),
                        );
                    }
                    1 => {
                        crate::boot_part_d::fn_1733a0(m, m.word(rd + 36), &mut Stream(h));
                    }
                    _ => {}
                }
                copy(m, m.word(out), rd + 24, 28);
                if m.byte(rd + 12) != 4 {
                    byte(m, rd + 28, 1);
                }
                m.put_word(data, m.word(m.word(out) + 16));
            }
        }
        if rc >= 0x80000000 {
            free(m, h, m.word(m.word(rd + 8) + 4), m.word(out));
            m.put_word(out, 0);
        }
    }
    m.put_word(va, rc);
    m.put_word(va + 4, m.word(out));
    trace(
        m,
        h,
        m.word(m.word(rd + 8) + 16),
        e.reader.category,
        e.detach_messages[1],
        va,
    );
    let final_out = m.word(out);
    let final_data = m.word(data);
    restore(m, e, save);
    m.put_word(out, final_out);
    m.put_word(data, final_data);
    rc
}

struct Stream<'a, H>(&'a mut H);
impl<H: CallHost> crate::boot_part_d::Host for Stream<'_, H> {
    fn params_call_m(
        &mut self,
        _: u32,
        _: &mut [u32; 4],
        _: u32,
        _: &mut u32,
        _: u32,
        _: &mut Memory<'_>,
    ) -> u32 {
        unreachable!("stream close never creates parameter blocks")
    }
    fn call_m(&mut self, f: u32, a: &[u32], m: &mut Memory<'_>) -> u32 {
        self.0.call(m, f, a)
    }
}
