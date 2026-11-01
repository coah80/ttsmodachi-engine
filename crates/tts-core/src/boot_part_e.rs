
use crate::{boot::Memory, frontend};

pub trait Host {
    fn call(&mut self, m: &mut Memory<'_>, method: u32, arguments: &[u32]) -> u32;
}
struct Heap<'a, H> {
    base: u32,
    methods: [u32; 4],
    host: &'a mut H,
}
impl<H: Host> frontend::HeapVtableM for Heap<'_, H> {
    fn alloc_at_m(&mut self, obj: u32, n: u32, bytes: &mut [u8]) -> u32 {
        self.host.call(
            &mut Memory {
                base: self.base,
                bytes,
            },
            self.methods[0],
            &[obj, n],
        )
    }
    fn realloc_at_m(&mut self, obj: u32, p: u32, n: u32, bytes: &mut [u8]) -> u32 {
        self.host.call(
            &mut Memory {
                base: self.base,
                bytes,
            },
            self.methods[2],
            &[obj, p, n],
        )
    }
    fn release_at_m(&mut self, obj: u32, p: u32, bytes: &mut [u8]) {
        self.host.call(
            &mut Memory {
                base: self.base,
                bytes,
            },
            self.methods[3],
            &[obj, p],
        );
    }
}
fn heap<'a, H: Host>(m: &Memory<'_>, p: u32, h: &'a mut H) -> Heap<'a, H> {
    let vt = if p == 0 { 0 } else { m.word(p) };
    Heap {
        base: m.base,
        methods: if p == 0 {
            [0; 4]
        } else {
            [m.word(vt), m.word(vt + 4), m.word(vt + 8), m.word(vt + 12)]
        },
        host: h,
    }
}
fn nz(m: &Memory<'_>, p: u32) -> Option<u32> {
    if p == 0 {
        None
    } else {
        Some(p - m.base)
    }
}
pub(crate) fn alloc(m: &mut Memory<'_>, p: u32, n: u32, h: &mut impl Host) -> u32 {
    if p == 0 || n == 0 {
        return 0;
    }
    let off = nz(m, p);
    let mut vt = heap(m, p, h);
    frontend::heap_alloc_m(m.bytes, off, n, &mut vt)
}
pub(crate) fn release(m: &mut Memory<'_>, p: u32, v: u32, h: &mut impl Host) {
    if p == 0 || v == 0 {
        return;
    }
    let off = nz(m, p);
    let mut vt = heap(m, p, h);
    frontend::heap_release_m(m.bytes, off, if v == 0 { None } else { Some(v) }, &mut vt)
}
pub(crate) fn calloc(m: &mut Memory<'_>, p: u32, n: u32, size: u32, h: &mut impl Host) -> u32 {

    struct Select {
        args: [u32; 3],
    }
    impl frontend::MethodTable for Select {
        fn call_at(&mut self, o: u32, n: u32, s: u32) -> u32 {
            self.args = [o, n, s];
            1
        }
    }
    if p == 0 || n == 0 || size == 0 {
        return 0;
    }
    let mut select = Select { args: [0; 3] };
    frontend::call_method(m.bytes, nz(m, p), n, size, &mut select);
    h.call(m, m.word(m.word(p) + 4), &select.args)
}

pub fn fn_1963e0(m: &mut Memory<'_>, p: u32, old: u32, n: u32, h: &mut impl Host) -> u32 {
    if p == 0 || n == 0 {
        return 0;
    }
    let off = nz(m, p);
    let mut vt = heap(m, p, h);
    frontend::heap_realloc_m(m.bytes, off, old, n, &mut vt)
}

pub fn fn_1970ac(m: &mut Memory<'_>, p: u32, i: u32, out: u32) -> u32 {
    if p == 0 || out == 0 || i >= m.word(p + 20) {
        return 0;
    }
    let at = m.offset(p + 4);
    let size = u16::from_le_bytes(m.bytes[at..at + 2].try_into().unwrap()) as u32;
    m.put_word(out, m.word(p).wrapping_add(i.wrapping_mul(size)));
    1
}

pub fn fn_199134(
    m: &mut Memory<'_>,
    dst: u32,
    size: u32,
    count: u32,
    p: u32,
    h: &mut impl Host,
) -> u32 {
    if p == 0 || dst == 0 || size == 0 || count == 0 {
        return 0x80602007;
    }
    let f = m.word(m.word(m.word(p) + 8) + 8);
    if f == 0 {
        return 0x80602001;
    }
    h.call(m, f, &[dst, size, count, m.word(p + 8)])
}

pub fn fn_1991d0(m: &mut Memory<'_>, p: u32, h: &mut impl Host) -> u32 {
    if p == 0 {
        return 0x80602000;
    }
    let f = m.word(m.word(m.word(p) + 8) + 16);
    if f == 0 {
        return 0x80602000;
    }
    h.call(m, f, &[m.word(p + 8)])
}

pub fn fn_1990ec(m: &mut Memory<'_>, p: u32, h: &mut impl Host) -> u32 {
    if p == 0 {
        return 0x80602007;
    }
    let f = m.word(m.word(m.word(p) + 8) + 4);
    let rc = h.call(m, f, &[m.word(p + 8)]);
    if rc < 0x80000000 {
        release(m, m.word(p + 4), p, h)
    }
    rc
}

pub fn fn_199034(
    m: &mut Memory<'_>,
    src: u32,
    ctx: u32,
    c: u32,
    d: u32,
    out: u32,
    h: &mut impl Host,
) -> u32 {
    if src == 0 || ctx == 0 || out == 0 {
        return 0x80602007;
    }
    if m.word(m.word(src + 8)) == 0 {
        return 0x80602001;
    }
    let p = calloc(m, ctx, 1, 12, h);
    m.put_word(out, p);
    if p == 0 {
        return 0x8060200a;
    }
    m.put_word(p, src);
    m.put_word(m.word(out) + 4, ctx);
    let rc = h.call(
        m,
        m.word(m.word(src + 8)),
        &[m.word(src + 12), m.word(src + 4), c, d, m.word(out) + 8],
    );
    if rc >= 0x80000000 {
        release(m, ctx, m.word(out), h);
        m.put_word(out, 0)
    }
    rc
}

pub fn fn_17ad10(m: &mut Memory<'_>, o: u32, p: u32, h: &mut impl Host) {
    if p == 0 {
        return;
    }
    if m.word(p + 8) != 0 {
        release(m, o, m.word(p + 8), h)
    }
    if m.word(p + 16) != 0 {
        let mut i = 0;
        while i < half(m, p + 12) as u32 {
            release(m, o, m.word(m.word(p + 16) + i * 4), h);
            i += 1;
        }
        release(m, o, m.word(p + 16), h)
    }
    if m.word(p + 20) != 0 {
        release(m, o, m.word(p + 20), h)
    }
    release(m, o, p, h)
}
fn half(m: &Memory<'_>, p: u32) -> u16 {
    let i = m.offset(p);
    u16::from_le_bytes(m.bytes[i..i + 2].try_into().unwrap())
}
fn stream_call(m: &mut Memory<'_>, p: u32, slot: u32, a: &[u32], h: &mut impl Host) -> u32 {
    let s = m.word(p + 52);
    let vt = m.word(s);
    let this = s.wrapping_sub(m.word(vt + 28));
    let mut args = [0; 3];
    args[0] = this;
    args[1..1 + a.len()].copy_from_slice(a);
    h.call(m, m.word(vt + slot), &args[..a.len() + 1])
}

pub fn fn_17c9d8(
    m: &mut Memory<'_>,
    p: u32,
    a: u32,
    b: u32,
    c: u32,
    out: u32,
    h: &mut impl Host,
) -> u32 {
    let buf = alloc(m, m.word(m.word(p) + 4), c.wrapping_add(1), h);
    if buf == 0 {
        return 0x8c10200a;
    }
    let offset = m
        .word(
            m.word(p + 4)
                .wrapping_add(a.wrapping_mul(24))
                .wrapping_add(12),
        )
        .wrapping_add(b);
    let r = stream_call(m, p, 0, &[offset], h);
    if r != 0 {
        release(m, m.word(m.word(p) + 4), buf, h);
        return 0x8c102004;
    }
    let rc = stream_call(m, p, 4, &[buf, c], h);
    if rc == 0 {
        m.put_word(out, buf);
        rc
    } else {
        release(m, m.word(m.word(p) + 4), buf, h);
        0x8c102004
    }
}
fn length(m: &Memory<'_>, p: u32) -> u32 {
    crate::fe_text::strlen_195d90(m.bytes, p - m.base) as u32
}
fn dot(m: &Memory<'_>, p: u32) -> u32 {
    crate::pronunciation::class_index(&m.bytes[m.offset(p)..], b'.').map_or(0, |i| p + i as u32)
}
fn move_bytes(m: &mut Memory<'_>, d: u32, s: u32, n: u32) {
    crate::fe_text::memmove_195b38(m.bytes, d - m.base, s - m.base, n)
}

pub fn fn_17db60(
    m: &mut Memory<'_>,
    out: u32,
    a: u32,
    b: u32,
    flag: u32,
    allocator: u32,
    h: &mut impl Host,
) -> u32 {
    if a == 0 || b == 0 {
        return 0x81506000;
    }
    let mut la = length(m, a);
    let lb = length(m, b);
    let buf = alloc(m, allocator, la.wrapping_add(lb).wrapping_add(2), h);
    m.put_word(out, buf);
    if buf == 0 {
        return 0x8150600a;
    }
    crate::normalization::strcpy_at(m.bytes, buf - m.base, a - m.base);
    if flag != 0 {
        while la != 0 && m.byte(buf + la - 1) != b'/' {
            la -= 1
        }
        let at = m.offset(buf + la);
        m.bytes[at] = 0;
    }
    if lb != 0 && la != 0 && m.byte(a + la - 1) != b'/' {
        let i = m.offset(buf);
        crate::pronunciation::append_terminated(&mut m.bytes[i..], b"/\0");
    }
    crate::fe_text::strcat_195c90(m.bytes, buf - m.base, b - m.base);
    let mut p = dot(m, buf + 1);
    while p != 0 {
        if m.byte(p - 1) == b'/' {
            if m.byte(p + 1) == b'/' {
                let n = length(m, p + 2) + 1;
                move_bytes(m, p, p + 2, n);
            } else if m.byte(p + 1) == b'.' && m.byte(p + 2) == b'/' {
                let q = p;
                let mut named = false;
                p -= 1;
                while buf < p && m.byte(p - 1) != b'/' {
                    p -= 1;
                    if m.byte(p) != b'.' {
                        named = true
                    }
                }
                if !named {
                    p = q + 1
                } else {
                    let n = length(m, q + 3) + 1;
                    move_bytes(m, p, q + 3, n)
                }
            } else {
                p += 1
            }
        } else {
            p += 1
        }
        p = dot(m, p)
    }
    0
}

pub struct UnmapEnvironment {
    pub category: u32,
    pub begin_format: u32,
    pub end_format: u32,
    pub error_format: u32,
    pub labels: [u32; 3],
    pub scratch: u32,
}
struct UnmapHost<'a, H>(&'a mut H);
impl<H: Host> crate::boot_part_d::Host for UnmapHost<'_, H> {
    fn call_m(&mut self, f: u32, a: &[u32], m: &mut Memory<'_>) -> u32 {
        self.0.call(m, f, a)
    }
    fn params_call_m(
        &mut self,
        _q: u32,
        _p: &mut [u32; 4],
        _v: u32,
        _out: &mut u32,
        _flags: u32,
        _m: &mut Memory<'_>,
    ) -> u32 {
        panic!("unmap does not call archive loading")
    }
}
impl<H: Host> crate::log_live::LogMethodsM for UnmapHost<'_, H> {
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
impl<H: Host> crate::boot_part_b::Host for UnmapHost<'_, H> {
    fn thread_id(&mut self) -> u32 {
        panic!("unmap formatter has no thread-id call")
    }
    fn call(&mut self, m: &mut Memory<'_>, f: u32, a: &[u32]) -> u32 {
        match f {
            0x196394 => alloc(m, a[0], a[1], self.0),
            0x1963b8 => calloc(m, a[0], a[1], a[2], self.0),
            0x196428 => {
                release(m, a[0], a[1], self.0);
                0
            }
            _ => self.0.call(m, f, a),
        }
    }
}
impl<H: Host> crate::boot_part_b::FormatHost for UnmapHost<'_, H> {}

pub fn fn_179208(
    m: &mut Memory<'_>,
    hdata: u32,
    p: u32,
    buf: u32,
    e: &UnmapEnvironment,
    h: &mut impl Host,
) -> u32 {
    if hdata == 0 || p == 0 || buf == 0 {
        return 0x81506007;
    }
    assert_eq!(m.base, 0, "fn_17331c requires a zero-based arena");
    let context = m.word(m.word(hdata + 4) + 8);
    let at = m.offset(e.scratch);
    let mut saved = [0u8; 64];
    saved.copy_from_slice(&m.bytes[at..at + 64]);
    m.put_word(e.scratch, p);
    m.put_word(e.scratch + 4, buf);
    crate::log_live::fn_1738e8(
        m,
        m.word(hdata),
        e.category,
        4,
        0,
        e.begin_format,
        e.scratch,
        &mut UnmapHost(h),
    );
    let mut result = 0;
    let map = m.word(p + 4);
    if map != 0 {
        let entry = m.word(map + 4).wrapping_add(m.word(p + 8).wrapping_mul(24));
        if m.word(entry + 4) == 0 {
            release(m, m.word(context + 4), buf, h);
        }
    } else {
        result = crate::boot_part_d::fn_17331c(m, m.word(p), buf, &mut UnmapHost(h));
        if result >= 0x80000000 {
            for (i, v) in [e.labels[0], p, e.labels[1], buf, e.labels[2], result]
                .iter()
                .enumerate()
            {
                m.put_word(e.scratch + i as u32 * 4, *v);
            }
            crate::render_part_a::error_log(
                m,
                &mut UnmapHost(h),
                m.word(hdata),
                e.category,
                0x128,
                e.error_format,
                e.scratch,
                e.scratch + 24,
            );
        }
    }
    m.put_word(e.scratch, result);
    crate::log_live::fn_1738e8(
        m,
        m.word(hdata),
        e.category,
        4,
        0,
        e.end_format,
        e.scratch,
        &mut UnmapHost(h),
    );
    m.bytes[at..at + 64].copy_from_slice(&saved);
    result
}

use crate::{
    boot_part_c::{BootC5Host, BootCHost},
    boot_part_f,
};

pub struct ArchiveEnvironment {
    pub parameters: boot_part_f::Environment,
    pub stream_table: u32,
    pub wrapper_table: u32,
    pub validator: u32,
    pub substream_table: u32,
    pub nvar: u32,
    pub chinese: u32,
    pub scratch: u32,
}

pub trait ArchiveHost: Host {
    fn service(&mut self, m: &mut Memory<'_>, f: u32, a: &[u32], out: &mut [u32; 4]) -> u32;
    fn read(&mut self, m: &mut Memory<'_>, f: u32, ctx: u32, out: &mut [u8]) -> u32;

    fn streams(&self) -> (crate::boot_port_1::Streams, u32) {
        panic!("archive query requires caller-owned stream environment")
    }

    fn bind_engine_services(&self) -> bool {
        false
    }
}
pub(crate) struct ArchiveCalls<'a, H> {
    pub(crate) h: &'a mut H,
    pub(crate) e: &'a ArchiveEnvironment,
    pub(crate) wrapper: u32,
}
impl<H: ArchiveHost> boot_part_f::Host for ArchiveCalls<'_, H> {
    fn call(&mut self, m: &mut Memory<'_>, f: u32, a: &[u32]) -> u32 {
        if f == self.e.validator {
            crate::normalization::range_0_2(a[0])
        } else if Some(f) == archive_method(m, self.e.substream_table) {
            fn_1b3208(m, a[0])
        } else if Some(f) == archive_method(m, self.e.substream_table - 8) {
            fn_1b44cc(m, a[0], self.h)
        } else if Some(f) == archive_method(m, self.e.stream_table + 16) {
            fn_1aa390(m, a[0], a[1])
        } else if Some(f) == archive_method(m, self.e.stream_table + 52) {
            fn_1aa5e8(m, a[0], a[1], a[2], a[3], a[4], self.h)
        } else {
            self.h.call(m, f, a)
        }
    }
    fn compare(
        &mut self,
        m: &mut Memory<'_>,
        _f: u32,
        key: &mut boot_part_f::Key,
        value: &mut u32,
    ) -> u32 {
        boot_part_f::compare_parameter(m, *key, *value)
    }
}
impl<H: ArchiveHost> Host for ArchiveCalls<'_, H> {
    fn call(&mut self, m: &mut Memory<'_>, f: u32, a: &[u32]) -> u32 {
        boot_part_f::Host::call(self, m, f, a)
    }
}
impl<H: ArchiveHost> BootCHost for ArchiveCalls<'_, H> {
    fn address(&self, us: u32) -> u32 {
        match us {
            0x8faae0 => self.e.stream_table,
            0x917e3c => self.e.wrapper_table,
            0x917e04 => self.e.substream_table,
            0x1a8684 => self.e.validator,
            _ => us,
        }
    }
    fn invoke(&mut self, m: &mut Memory<'_>, f: u32, a: &[u32]) -> [u32; 3] {
        let mut out = [0; 4];
        let r = match f {
            0x1b2140 => {
                let mut status = 0;
                let r = boot_part_f::fn_1b2140(m, a[0], a[1], &mut status, self);
                out[0] = status as u32;
                if a[1] == 12 {
                    self.wrapper = r;
                }
                r
            }
            0x1b2128 => {
                boot_part_f::fn_1b2128(m, a[0], a[1], self);
                0
            }
            0x1b449c => boot_part_f::fn_1b449c(m, a[0], a[1], &self.e.parameters),
            0x1ba3c0 => boot_part_f::fn_1ba3c0(m, a[0], &self.e.parameters),
            0x195ae8 => {
                let d = a[0] - m.base;
                let s = a[1] - m.base;
                crate::normalization::memcpy_at(m.bytes, d, s, a[2]);
                a[0]
            }
            0x1a8cf8 => m.initialize_stream(self, a[0], a[1], a[2], a[3], a[4], a[5] as u8),
            0x1aaa80 => m.open_stream(self, a[..5].try_into().unwrap(), a[5]),
            0x1b4104 | 0x1babcc => {
                let slot = self.e.scratch + 96;
                let saved = m.word(slot);

                m.put_word(
                    slot,
                    if f == 0x1babcc {
                        m.word(self.wrapper + 8)
                    } else {
                        0
                    },
                );
                let r = if f == 0x1b4104 {
                    m.create_stream_wrapper(self, a[0], a[1], slot)
                } else {
                    boot_part_f::fn_1babcc(
                        m,
                        a[0],
                        a[1],
                        a[2],
                        a[3],
                        slot,
                        &self.e.parameters,
                        self,
                    )
                };
                out[0] = m.word(slot);
                m.put_word(slot, saved);
                r
            }
            0x1a86a0 => {
                let at = self.e.scratch + 104;
                let saved = m.word(at);
                m.put_word(at, 0);
                let r = m.validate_stream(self, a[0], a[1], at, at + 1);
                out[0] = m.byte(at) as u32;
                out[1] = m.byte(at + 1) as u32;
                m.put_word(at, saved);
                r
            }
            0x1b35a0 => boot_part_f::fn_1b35a0(
                m,
                a[0],
                a[1],
                a[2],
                a[3],
                a[4],
                a[5],
                a[6],
                &self.e.parameters,
                self,
            ),
            0x1b3b3c => boot_part_f::fn_1b3b3c(m, a[0], a[1], &self.e.parameters, self),
            0x1994d0 => crate::frontend::divmod_u32(a[0], a[1]).0,
            0x1ba3e0 => crate::normalization::adler32(
                m.bytes,
                a[0],
                if a[1] == 0 { 0 } else { a[1] - m.base },
                a[2],
            ),
            0x1b3c34 => boot_part_f::fn_1b3c34(m, a[0], a[1], a[2], &self.e.parameters, self),
            0x1b3ecc => boot_part_f::fn_1b3ecc(m, a[0], a[1], &self.e.parameters, self),
            0x1b3fe8 => {
                boot_part_f::fn_1b3fe8(m, a[0], a[1], a[2], &self.e.parameters, self);
                0
            }
            _ => self.h.service(m, f, a, &mut out),
        };
        [r, out[0], out[1]]
    }
}
impl<H: ArchiveHost> BootC5Host for ArchiveCalls<'_, H> {
    fn address(&self, us: u32) -> u32 {
        BootCHost::address(self, us)
    }
    fn read(&mut self, m: &mut Memory<'_>, f: u32, c: u32, b: &mut [u8]) -> u32 {
        self.h.read(m, f, c, b)
    }
    fn call(&mut self, m: &mut Memory<'_>, f: u32, a: &[u32], out: &mut [u32; 4]) -> u32 {
        if Some(f) == archive_method(m, self.e.substream_table + 36) {
            return fn_1b3064(m, a[0], out, self.h);
        }
        if Some(f) == archive_method(m, self.e.substream_table)
            || Some(f) == archive_method(m, self.e.substream_table - 8)
            || Some(f) == archive_method(m, self.e.stream_table + 16)
        {
            return boot_part_f::Host::call(self, m, f, a);
        }
        match f {
            0x1b36fc => m.set_stream_property(self, a.try_into().unwrap()) as u32,
            0x1b3368 => {
                let slot = self.e.scratch + 112;
                let saved = m.word(slot);
                m.put_word(slot, 0);
                let r = m.create_substream(self, a.try_into().unwrap(), slot);
                out[0] = m.word(slot);
                m.put_word(slot, saved);
                r
            }
            0x1b2140 | 0x1b3b3c | 0x1b3fe8 | 0x1b3ecc => {
                let r = self.invoke(m, f, a);
                out[0] = r[1];
                r[0]
            }
            _ => self.h.service(m, f, a, out),
        }
    }
}

pub fn fn_1a8cf8(
    m: &mut Memory<'_>,
    args: [u32; 6],
    e: &ArchiveEnvironment,
    h: &mut impl ArchiveHost,
) -> u32 {
    let [p, a, b, c, d, flag] = args;
    m.initialize_stream(
        &mut ArchiveCalls { h, e, wrapper: 0 },
        p,
        a,
        b,
        c,
        d,
        flag as u8,
    )
}

pub fn fn_1aaa80(
    m: &mut Memory<'_>,
    args: [u32; 5],
    out: u32,
    e: &ArchiveEnvironment,
    h: &mut impl ArchiveHost,
) -> u32 {
    m.open_stream(&mut ArchiveCalls { h, e, wrapper: 0 }, args, out)
}

pub fn fn_17cbb4(
    m: &mut Memory<'_>,
    p: u32,
    cfg: u32,
    mapped: u32,
    out: u32,
    mode: u32,
    e: &ArchiveEnvironment,
    h: &mut impl ArchiveHost,
) -> u32 {
    if p == 0 {
        return 0x8c102008;
    }
    if cfg == 0 || mapped == 0 {
        return 0x8c102007;
    }
    let at = m.offset(e.scratch);
    let mut saved = [0u8; 128];
    saved.copy_from_slice(&m.bytes[at..at + 128]);
    let rc = archive_load(m, p, cfg, mapped, out, mode, e, h);
    m.bytes[at..at + 128].copy_from_slice(&saved);
    rc
}
fn archive_load(
    m: &mut Memory<'_>,
    p: u32,
    cfg: u32,
    mapped: u32,
    out: u32,
    mode: u32,
    e: &ArchiveEnvironment,
    h: &mut impl ArchiveHost,
) -> u32 {
    let t = e.scratch;
    for off in (0..32).step_by(4) {
        m.put_word(t + off, 0);
    }
    m.put_word(p + 56, if mode == 0 { mapped } else { 0 });
    let mut allocation_failed = false;
    let mut rc = fn_1aaa80(m, [p + 12, p + 12, cfg, mapped, out], p + 48, e, h);
    if rc == 0 {
        let obj = m.word(p + 48);
        let vt = m.word(obj + 16);
        rc = fn_1aa5e8(
            m,
            (obj + 16).wrapping_sub(m.word(vt + 104)),
            0x1fe6ad1f,
            0,
            p + 52,
            t + 28,
            h,
        );
    }
    let mut format = 0x10000;
    if rc == 0 {
        let value = m.word(t + 28);
        let found = [0x50000, 0x40000, 0x30000, 0x20000, 0x10000]
            .into_iter()
            .find(|v| crate::normalization::compare_packed(value, *v) != 0);
        if let Some(v) = found {
            format = v;
        } else {
            rc = 1;
        }
    }
    if rc == 0 {
        rc = archive_read(m, p, t, 7, h);
        if rc == 0
            && (crate::normalization::memcmp_at(m.bytes, t - m.base, e.nvar - m.base, 4) != 0
                || m.byte(t + 4) > 5
                || (m.byte(t + 4) == 5
                    && (m.byte(t + 5) > 2 || (m.byte(t + 5) == 2 && m.byte(t + 6) > 3))))
        {
            rc = 1;
        }
    }
    if rc == 0 {
        rc = archive_read(m, p, p + 8, 4, h);
    }
    let mut offset = 11u32;
    if rc == 0 && m.word(p + 8) != 0 {
        let entries = calloc(m, m.word(m.word(p) + 4), m.word(p + 8), 24, h);
        m.put_word(p + 4, entries);
        if entries == 0 {
            allocation_failed = true;
            rc = 0x8c10200a;
        }
    }
    let mut i = 0;
    while rc == 0 && i < m.word(p + 8) {
        let entry = m.word(p + 4) + i * 24;
        rc = archive_read(m, p, t + 8, 2, h);
        if rc != 0 {
            break;
        }
        offset = offset.wrapping_add(2);
        let len = u16::from_le_bytes(
            m.bytes[m.offset(t + 8)..m.offset(t + 10)]
                .try_into()
                .unwrap(),
        ) as u32;
        let name = calloc(m, m.word(m.word(p) + 4), len + 1, 1, h);
        m.put_word(entry, name);
        if name == 0 {
            allocation_failed = true;
            rc = 0x8c10200a;
            break;
        }
        rc = archive_read(m, p, name, len, h);
        if rc != 0 {
            break;
        }
        offset = offset.wrapping_add(len);
        rc = archive_read(m, p, entry + 8, 4, h);
        if rc != 0 {
            break;
        }
        offset = offset.wrapping_add(4);
        let size = m.word(entry + 8);
        if i == 0 || mode != 0 {
            let data = calloc(m, m.word(m.word(p) + 4), 1, size, h);
            m.put_word(entry + 4, data);
            if data == 0 {
                allocation_failed = true;
                rc = 0x8c10200a;
                break;
            }
        }
        m.put_word(entry + 12, offset);
        if i == 0 || mode != 0 {
            rc = archive_read(m, p, m.word(entry + 4), size, h);
        } else {
            let stream = m.word(p + 52);
            let vt = m.word(stream);
            rc = h.call(
                m,
                m.word(vt),
                &[
                    stream.wrapping_sub(m.word(vt + 28)),
                    offset.wrapping_add(size),
                ],
            );
        }
        if rc != 0 {
            break;
        }
        offset = offset.wrapping_add(size);
        if i == 0
            && crate::normalization::strstr_at(
                m.bytes,
                m.word(entry + 4) - m.base,
                e.chinese - m.base,
            )
            .is_some()
            && format == 0x10000
        {
            rc = 1;
            break;
        }
        let mut chunks = 0u16;
        loop {
            rc = archive_read(m, p, t + 10, 1, h);
            if rc != 0 {
                break;
            }
            offset = offset.wrapping_add(1);
            rc = archive_read(m, p, t + 12, 8, h);
            if rc != 0 {
                break;
            }
            offset = offset.wrapping_add(8);
            let flag = m.byte(t + 10) as u32;
            let a = m.word(t + 12);
            let b = m.word(t + 16);
            if flag == 0 && a == 0 && b == 0 {
                break;
            }
            if m.word(entry + 8) < a || m.word(entry + 8) < flag.wrapping_mul(b).wrapping_add(a) {
                rc = 1;
                break;
            }
            if mode == 0 {
                if m.word(entry + 20) == (chunks as u32) * 100 {
                    chunks = chunks.wrapping_add(1);
                    let data = fn_1963e0(
                        m,
                        m.word(m.word(p) + 4),
                        m.word(entry + 16),
                        (chunks as u32) * 1200,
                        h,
                    );
                    m.put_word(entry + 16, data);
                    if data == 0 {
                        allocation_failed = true;
                        rc = 0x8c10200a;
                        break;
                    }
                }
                let rec = m.word(entry + 16) + m.word(entry + 20) * 12;
                let at = m.offset(rec);
                m.bytes[at] = flag as u8;
                m.put_word(rec + 4, a);
                m.put_word(rec + 8, b);
                m.put_word(entry + 20, m.word(entry + 20).wrapping_add(1));
            }
        }
        i += 1;
    }
    if rc == 0 {
        return 0;
    }
    let result = if allocation_failed {
        0x8c10200a
    } else {
        0x8c102004
    };
    crate::boot_part_b::fn_17ca94(m, p, &mut UnmapHost(&mut ArchiveCalls { h, e, wrapper: 0 }));
    result
}
fn archive_read(m: &mut Memory<'_>, p: u32, dst: u32, n: u32, h: &mut impl Host) -> u32 {
    let stream = m.word(p + 52);
    let vt = m.word(stream);
    h.call(
        m,
        m.word(vt + 4),
        &[stream.wrapping_sub(m.word(vt + 28)), dst, n],
    )
}

struct MethodAllocator<'a, H>(&'a mut H);
impl<H: ArchiveHost> boot_part_f::Host for MethodAllocator<'_, H> {
    fn call(&mut self, m: &mut Memory<'_>, f: u32, a: &[u32]) -> u32 {
        self.0.call(m, f, a)
    }
    fn compare(
        &mut self,
        m: &mut Memory<'_>,
        _: u32,
        key: &mut boot_part_f::Key,
        value: &mut u32,
    ) -> u32 {
        boot_part_f::compare_parameter(m, *key, *value)
    }
}

pub fn fn_1b3208(m: &mut Memory<'_>, p: u32) -> u32 {
    m.word(p + 28)
}

pub fn fn_1aa8f0(m: &mut Memory<'_>, p: u32, h: &mut impl boot_part_f::Host) -> u32 {
    let vt = m.word(p);
    h.call(m, m.word(vt + 16), &[p])
}

pub fn fn_172e18() -> u32 {
    0
}

pub fn fn_1aa390(m: &mut Memory<'_>, p: u32, id: u32) -> u32 {
    let mut count = 0u32;
    for i in 0..m.word(p + 40) {
        if m.word(m.word(p + 60).wrapping_add(i.wrapping_mul(12))) == id {
            count = count.wrapping_add(1);
        }
    }
    count
}

pub fn fn_1b44cc(m: &mut Memory<'_>, p: u32, h: &mut impl ArchiveHost) -> u32 {
    let refs = m.word(p + 4);
    if refs != 1 {
        m.put_word(p + 4, refs.wrapping_sub(1));
        return 0;
    }
    let owner = m.word(p + 8);
    if owner != 0 {
        if h.bind_engine_services() {
            crate::boot_port_1::fn_1ba384(m, owner, p, &mut MethodAllocator(h))
        } else {
            h.service(m, 0x1ba384, &[owner, p], &mut [0; 4])
        }
    } else {
        h.call(m, m.word(m.word(p) + 8), &[p])
    }
}

pub fn fn_1b3064(m: &mut Memory<'_>, p: u32, out: &mut [u32; 4], h: &mut impl ArchiveHost) -> u32 {
    let mut status = 0u8;
    if m.word(p + 24) == 0 {
        let data = boot_part_f::fn_1b2140(
            m,
            m.word(p + 68),
            m.word(p + 28),
            &mut status,
            &mut MethodAllocator(h),
        );
        m.put_word(p + 24, data);
        if status != 0 {
            return status as u32;
        }
    }
    let mut read = [0, 0, out[2], out[3]];
    let rc = if h.bind_engine_services() {
        fn_1b2e04(m, p, m.word(p + 24), m.word(p + 28), &mut read, h)
    } else {
        h.service(m, 0x1b2e04, &[p, m.word(p + 24), m.word(p + 28)], &mut read)
    };
    out[2] = read[2] & 255;
    out[3] = read[3] & 255;
    if rc != 0 {
        return (rc as u8) as u32;
    }
    if out[3] == 0 {
        out[0] = m.word(p + 24);
        out[1] = read[0];
    } else if m.byte(p + 72) == 0 {
        return crate::normalization::code_4(0);
    } else {
        boot_part_f::fn_1b2128(m, m.word(p + 68), m.word(p + 24), &mut MethodAllocator(h));
        m.put_word(p + 24, 0);
    }
    0
}

pub fn fn_1aa5e8(
    m: &mut Memory<'_>,
    p: u32,
    key: u32,
    occurrence: u32,
    out: u32,
    format: u32,
    h: &mut impl ArchiveHost,
) -> u32 {
    let heap = m.word(p + 68);
    if !h.bind_engine_services() {

        let mut values = [0; 4];
        let rc = h.service(m, 0x1a9a60, &[p, key, occurrence], &mut values);
        if rc != 0 {
            return rc;
        }
        if values[3] as u8 == 1 || values[1] == u32::MAX {
            return crate::normalization::code_4(0);
        }
        let mut stream = [0; 4];
        let at = m.offset(p + 44);
        let block = u16::from_le_bytes(m.bytes[at..at + 2].try_into().unwrap()) as u32;
        let rc = h.service(
            m,
            0x1b23bc,
            &[
                heap,
                heap,
                m.word(p + 48),
                m.word(p + 52),
                p + 12,
                values[2],
                values[0],
                block,
            ],
            &mut stream,
        );
        if rc != 0 {
            return rc;
        }
        h.call(m, m.word(m.word(p) + 12), &[p]);
        m.put_word(out, stream[0].wrapping_add(12));
        m.put_word(format, values[1]);
        return rc;
    }
    let (streams, scratch) = h.streams();

    let saved = core::array::from_fn::<_, 8, _>(|i| m.word(scratch + i as u32 * 4));
    let rc = crate::boot_port_1::fn_1a9a60(
        m,
        [p, key, occurrence, scratch, scratch + 4, scratch + 8, scratch + 12],
        scratch + 16,
        &streams,
        &mut MethodAllocator(h),
    );
    let values = [
        m.word(scratch),
        m.word(scratch + 4),
        m.word(scratch + 8),
        m.byte(scratch + 12) as u32,
    ];
    if rc != 0 {
        for (i, v) in saved.into_iter().enumerate() {
            m.put_word(scratch + i as u32 * 4, v);
        }
        return rc;
    }
    if values[3] == 1 || values[1] == u32::MAX {
        for (i, v) in saved.into_iter().enumerate() {
            m.put_word(scratch + i as u32 * 4, v);
        }
        return crate::normalization::code_4(0);
    }
    let at = m.offset(p + 44);
    let block = u16::from_le_bytes(m.bytes[at..at + 2].try_into().unwrap()) as u32;
    let rc = crate::boot_port_1::fn_1b23bc(
        m,
        [
            heap,
            heap,
            m.word(p + 48),
            m.word(p + 52),
            p + 12,
            values[2],
            values[0],
            block,
            scratch,
        ],
        &streams,
        &mut MethodAllocator(h),
    );
    let stream = m.word(scratch);
    for (i, v) in saved.into_iter().enumerate() {
        m.put_word(scratch + i as u32 * 4, v);
    }
    if rc != 0 {
        return rc;
    }
    h.call(m, m.word(m.word(p) + 12), &[p]);
    m.put_word(out, stream.wrapping_add(12));
    m.put_word(format, values[1]);
    rc
}

pub fn fn_1b2e04(
    m: &mut Memory<'_>,
    p: u32,
    buf: u32,
    len: u32,
    out: &mut [u32; 4],
    h: &mut impl ArchiveHost,
) -> u32 {
    let block = half(m, p + 36) as u32;
    out[2] = if block != 0 { 0 } else { 2 };
    out[3] = 0;
    let remaining = m.word(p + 56);
    if remaining == 0 {
        out[0] = 0;
        return 0;
    }
    if block == 0 {
        let count = remaining.min(len);
        let s = m.word(p + 48);
        let got = h.call(m, m.word(s), &[buf, 1, count, m.word(p + 52)]);
        if got != count {
            if h.call(m, m.word(s + 8), &[m.word(p + 52)]) != 0 {
                return crate::normalization::code_6(0);
            }
            out[3] = 1;
        }
        m.put_word(p + 56, remaining.wrapping_sub(got));
        out[0] = got;
        return 0;
    }
    let mut count = remaining.min(len);
    let mut total = 0u32;
    let mut b = buf;
    while count != 0 {
        let s = m.word(p + 48);
        let n = m.word(p + 60).min(count);
        let got = h.call(m, m.word(s), &[b, 1, n, m.word(p + 52)]);
        if got != n {
            if h.call(m, m.word(s + 8), &[m.word(p + 52)]) != 0 {
                return crate::normalization::code_6(0);
            }
            out[3] = 1;
            break;
        }
        if m.byte(p + 20) != 0 {
            let hash =
                crate::normalization::adler32(m.bytes, m.word(p + 64), b.wrapping_sub(m.base), got);
            m.put_word(p + 64, hash);
        }
        m.put_word(p + 60, m.word(p + 60).wrapping_sub(got));
        if m.word(p + 60) == 0 {
            let s = m.word(p + 48);
            let mut header = [0u8; 4];
            if h.read(m, m.word(s), m.word(p + 52), &mut header) != 4 {
                if h.call(m, m.word(s + 8), &[m.word(p + 52)]) != 0 {
                    return crate::normalization::code_6(0);
                }
                out[3] = 1;
                break;
            }
            if m.byte(p + 20) != 0 {
                if m.word(p + 64) != u32::from_le_bytes(header) {
                    out[2] = 1;
                    return 0;
                }
                m.put_word(p + 64, crate::normalization::adler32(m.bytes, 0, 0, 0));
            }
            let n = m.word(p + 56).wrapping_sub(total).wrapping_sub(got);
            m.put_word(p + 60, block.min(n));
        }
        count = count.wrapping_sub(got);
        b = b.wrapping_add(got);
        total = total.wrapping_add(got);
    }
    m.put_word(p + 56, m.word(p + 56).wrapping_sub(total));
    out[0] = total;
    0
}

fn archive_method(m: &Memory<'_>, slot: u32) -> Option<u32> {
    let at = slot.checked_sub(m.base)? as usize;
    let bytes = m.bytes.get(at..at.checked_add(4)?)?;
    Some(u32::from_le_bytes(bytes.try_into().ok()?))
}

pub trait SynchronizationHost {
    fn arbiter_address(&self) -> u32;
    fn arbitrate(&mut self, memory: &mut Memory<'_>, arguments: &[u32; 6]) -> u32;
}
struct SynchronizationCalls<'a, H>(&'a mut H);
impl<H: SynchronizationHost> crate::boot_part_b::Host for SynchronizationCalls<'_, H> {
    fn thread_id(&mut self) -> u32 {
        unreachable!("SDK event publication does not read thread identity")
    }
    fn call(&mut self, m: &mut Memory<'_>, f: u32, a: &[u32]) -> u32 {
        assert_eq!(
            f, 0x172d18,
            "synchronization reached a non-kernel dependency"
        );
        self.0.arbitrate(m, a.try_into().unwrap())
    }
}
impl<H: SynchronizationHost> crate::boot::Calls for SynchronizationCalls<'_, H> {
    fn call(&mut self, m: &mut Memory<'_>, f: u32, a: &[u32]) -> u32 {
        crate::boot_part_b::Host::call(self, m, f, a)
    }
    fn call_wide(&mut self, _: &mut Memory<'_>, _: u32, _: &[u32]) -> u64 {
        unreachable!("address arbitration returns a single word")
    }
}

pub fn fn_183dd4(m: &mut Memory<'_>, p: u32, h: &mut impl SynchronizationHost) {
    if m.word(p) == u32::MAX {
        m.put_word(p, 0);
        let handle = m.word(h.arbiter_address());
        h.arbitrate(m, &[handle, p, 0, 1, 0, 0]);
    } else if m.word(p) == u32::MAX - 1 {
        let global = h.arbiter_address();
        let handle = m.word(global);
        crate::boot_part_b2::fn_183edc(m, p + 4, handle, &mut SynchronizationCalls(h));
        m.put_word(p, 1);
        let handle = m.word(global);
        h.arbitrate(m, &[handle, p, 0, u32::MAX, 0, 0]);
        let globals = crate::boot::Globals {
            manager: 0,
            callbacks: 0,
            rules: 0,
            components: 0,
            dictionary: 0,
            context: 0,
            mode: 0,
            semaphore: global,
            service: 0,
            command: 0,
        };
        m.fn_165d00(&mut SynchronizationCalls(h), &globals, p + 4);
    }
}
