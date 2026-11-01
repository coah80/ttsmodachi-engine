

use crate::{boot::Memory, frontend};
const INVALID: u32 = 0x80602007;
const MISSING: u32 = 0x80602001;
const NOMEM: u32 = 0x8060200a;

pub trait Host {
    fn call_m(&mut self, method: u32, args: &[u32], m: &mut Memory<'_>) -> u32;

    fn params_call_m(
        &mut self,
        q: u32,
        params: &mut [u32; 4],
        p: u32,
        out: &mut u32,
        flags: u32,
        m: &mut Memory<'_>,
    ) -> u32;
}
fn method(m: &Memory<'_>, object: u32, offset: u32) -> u32 {
    m.word(m.word(object + 8) + offset)
}
struct Heap<'a, H> {
    host: &'a mut H,
    methods: [u32; 4],
}
impl<H: Host> frontend::HeapVtableM for Heap<'_, H> {
    fn alloc_at_m(&mut self, obj: u32, n: u32, mem: &mut [u8]) -> u32 {
        self.host.call_m(
            self.methods[0],
            &[obj, n],
            &mut Memory {
                base: 0,
                bytes: mem,
            },
        )
    }
    fn realloc_at_m(&mut self, obj: u32, p: u32, n: u32, mem: &mut [u8]) -> u32 {
        self.host.call_m(
            self.methods[2],
            &[obj, p, n],
            &mut Memory {
                base: 0,
                bytes: mem,
            },
        )
    }
    fn release_at_m(&mut self, obj: u32, p: u32, mem: &mut [u8]) {
        self.host.call_m(
            self.methods[3],
            &[obj, p],
            &mut Memory {
                base: 0,
                bytes: mem,
            },
        );
    }
}
fn heap<H>(m: &Memory<'_>, p: u32, host: H) -> (H, [u32; 4]) {
    let vt = if p == 0 { 0 } else { m.word(p) };
    (
        host,
        if p == 0 {
            [0; 4]
        } else {
            core::array::from_fn(|i| m.word(vt + i as u32 * 4))
        },
    )
}

struct Request(Option<[u32; 3]>);
impl frontend::MethodTable for Request {
    fn call_at(&mut self, obj: u32, a: u32, b: u32) -> u32 {
        self.0 = Some([obj, a, b]);
        0
    }
}
fn alloc(m: &mut Memory<'_>, p: u32, n: u32, h: &mut impl Host) -> u32 {
    assert_eq!(m.base, 0);
    let mut request = Request(None);
    frontend::call_method(
        m.bytes,
        core::num::NonZeroU32::new(p).map(|v| v.get()),
        1,
        n,
        &mut request,
    );
    match request.0 {
        Some(a) => h.call_m(m.word(m.word(p) + 4), &a, m),
        None => 0,
    }
}
fn free(m: &mut Memory<'_>, p: u32, q: u32, h: &mut impl Host) {
    if p == 0 || q == 0 {
        return;
    }
    assert_eq!(m.base, 0);
    let (host, methods) = heap(m, p, h);
    frontend::heap_release_m(
        m.bytes,
        if p == 0 { None } else { Some(p) },
        if q == 0 { None } else { Some(q) },
        &mut Heap { host, methods },
    );
}
fn realloc(m: &mut Memory<'_>, p: u32, q: u32, n: u32, h: &mut impl Host) -> u32 {
    if p == 0 || n == 0 {
        return 0;
    }
    assert_eq!(m.base, 0);
    let (host, methods) = heap(m, p, h);
    frontend::heap_realloc_m(
        m.bytes,
        if p == 0 { None } else { Some(p) },
        q,
        n,
        &mut Heap { host, methods },
    )
}

pub fn fn_176fd4(m: &mut Memory<'_>, out: u32, pair: [u32; 2]) {
    m.put_word(out, pair[0]);
    m.put_word(out + 4, pair[1]);
}

pub fn fn_172eac(
    m: &mut Memory<'_>,
    src: u32,
    ctx: u32,
    c: u32,
    d: u32,
    out: u32,
    h: &mut impl Host,
) -> u32 {
    if src == 0 || ctx == 0 || out == 0 {
        return INVALID;
    }
    if method(m, src, 0x54) == 0 {
        return MISSING;
    }
    let p = alloc(m, ctx, 12, h);
    m.put_word(out, p);
    if p == 0 {
        return NOMEM;
    }
    m.put_word(p, src);
    m.put_word(p + 4, ctx);
    let r = h.call_m(
        method(m, src, 0x54),
        &[m.word(src + 12), m.word(src + 4), c, d, m.word(out) + 8],
        m,
    );
    if r >= 0x80000000 {
        free(m, ctx, m.word(out), h);
        m.put_word(out, 0)
    }
    r
}

pub fn fn_172f64(m: &mut Memory<'_>, p: u32, h: &mut impl Host) -> u32 {
    if p == 0 {
        return INVALID;
    }
    let r = h.call_m(method(m, m.word(p), 0x58), &[m.word(p + 8)], m);
    if r < 0x80000000 {
        free(m, m.word(p + 4), p, h)
    }
    r
}

pub fn fn_172fac(m: &mut Memory<'_>, p: u32, x: u32, y: u32, h: &mut impl Host) -> u32 {
    if p == 0 || x == 0 || y == 0 {
        return INVALID;
    }
    h.call_m(method(m, m.word(p), 0x5c), &[m.word(p + 8), x, y], m)
}

pub fn fn_173000(
    m: &mut Memory<'_>,
    src: u32,
    ctx: u32,
    c: u32,
    out: u32,
    rb: u32,
    h: &mut impl Host,
) -> u32 {
    if src == 0 || ctx == 0 || out == 0 || c == 0 {
        return INVALID;
    }
    let p = alloc(m, ctx, 32, h);
    m.put_word(out, p);
    if p == 0 {
        return NOMEM;
    }
    m.put_word(p, src);
    m.put_word(p + 4, ctx);
    let get = method(m, src, 40);
    let mut r = if get != 0 {
        h.call_m(
            get,
            &[m.word(src + 12), m.word(src + 4), c, m.word(out) + 12],
            m,
        )
    } else {
        MISSING
    };
    if r & 0x1fff == 1 {
        r = h.call_m(
            method(m, src, 0),
            &[m.word(src + 12), m.word(src + 4), c, rb, m.word(out) + 16],
            m,
        );
    }
    if r >= 0x80000000 {
        free(m, ctx, m.word(out), h);
        m.put_word(out, 0)
    }
    r
}

pub fn fn_1730fc(m: &mut Memory<'_>, p: u32, h: &mut impl Host) -> u32 {
    if p == 0 {
        return INVALID;
    }
    let mut r = 0;
    if m.word(p + 12) != 0 {
        r = h.call_m(method(m, m.word(p), 44), &[m.word(p + 12)], m)
    } else {
        if m.word(p + 20) != 0 {
            free(m, m.word(p + 4), m.word(p + 20), h)
        }
        if m.word(p + 16) != 0 {
            r = h.call_m(method(m, m.word(p), 4), &[m.word(p + 16)], m)
        }
    }
    if r < 0x80000000 {
        free(m, m.word(p + 4), p, h)
    }
    r
}

pub fn fn_173180(
    m: &mut Memory<'_>,
    dst: u32,
    src: u32,
    path: u32,
    n: u32,
    flags: u32,
    h: &mut impl Host,
) -> u32 {
    if dst == 0 || src == 0 || path == 0 {
        return INVALID;
    }
    let f = method(m, dst, 60);
    if f == 0 {
        return MISSING;
    }
    h.call_m(f, &[m.word(dst + 12), m.word(dst + 4), path, n, flags], m)
}

pub fn fn_1731c4(
    m: &mut Memory<'_>,
    s: u32,
    pos: u32,
    len: u32,
    out: u32,
    h: &mut impl Host,
) -> u32 {
    if s == 0 || len == 0 || out == 0 {
        return INVALID;
    }
    if m.byte(s + 8) != 0 {
        return 0x80602011;
    }
    m.put_word(out, 0);
    let r;
    if m.word(s + 12) != 0 {
        r = h.call_m(
            method(m, m.word(s), 48),
            &[m.word(s + 12), pos, len, out],
            m,
        );
        if r >= 0x80000000 {
            return r;
        }
    } else {
        if m.word(len) == 0 {
            let n = h.call_m(method(m, m.word(s), 16), &[m.word(s + 16)], m);
            m.put_word(len, n);
            if n <= pos {
                return 0x80602004;
            }
            m.put_word(len, n.wrapping_sub(pos));
        }
        r = h.call_m(method(m, m.word(s), 12), &[m.word(s + 16), pos, 0, 1], m);
        if r >= 0x80000000 {
            return r;
        }
        if m.word(s + 24) < m.word(len) {
            let t = realloc(m, m.word(s + 4), m.word(s + 20), m.word(len), h);
            if t == 0 {
                return NOMEM;
            }
            m.put_word(s + 20, t);
            m.put_word(s + 24, m.word(len));
        }
        let n = h.call_m(
            method(m, m.word(s), 8),
            &[m.word(s + 20), 1, m.word(len), m.word(s + 16)],
            m,
        );
        if n == 0 || n == u32::MAX {
            return 0x80602004;
        }
        m.put_word(len, n);
        m.put_word(out, m.word(s + 20));
    }
    let at = m.offset(s + 8);
    m.bytes[at] = 1;
    r
}

pub fn fn_17331c(m: &mut Memory<'_>, p: u32, a: u32, h: &mut impl Host) -> u32 {
    if p == 0 || a == 0 {
        return INVALID;
    }
    if m.byte(p + 8) == 0 {
        return 0x80602011;
    }
    let mut r = 0;
    if m.word(p + 12) != 0 {
        r = h.call_m(method(m, m.word(p), 52), &[m.word(p + 12), a], m);
        if r >= 0x80000000 {
            return r;
        }
    } else if m.word(p + 24) > 0x400 {
        free(m, m.word(p + 4), m.word(p + 20), h);
        m.put_word(p + 20, 0);
        m.put_word(p + 24, 0);
    }
    let at = m.offset(p + 8);
    m.bytes[at] = 0;
    r
}

pub fn fn_1733a0(m: &mut Memory<'_>, p: u32, h: &mut impl Host) -> u32 {
    if p == 0 {
        return INVALID;
    }
    let stream = m.word(p + 12);
    if stream != 0 {
        let f = method(m, m.word(p), 56);
        if f != 0 {
            return h.call_m(f, &[stream], m);
        }
    } else {
        let handle = m.word(p + 16);
        if handle != 0 {
            let r = h.call_m(method(m, m.word(p), 4), &[handle], m);
            m.put_word(p + 16, 0);
            return r;
        }
    }
    0
}

pub fn fn_17340c(
    m: &mut Memory<'_>,
    o: u32,
    al: u32,
    a: u32,
    b: u32,
    c: u32,
    out: u32,
    h: &mut impl Host,
) -> u32 {
    if o == 0 || al == 0 || out == 0 || a == 0 || c == 0 {
        return INVALID;
    }
    if method(m, o, 72) == 0 {
        return MISSING;
    }
    let p = alloc(m, al, 12, h);
    m.put_word(out, p);
    if p == 0 {
        return NOMEM;
    }
    m.put_word(p, o);
    m.put_word(p + 4, al);
    let r = h.call_m(
        method(m, o, 72),
        &[m.word(o + 12), m.word(o + 4), a, b, c, m.word(out) + 8],
        m,
    );
    if r >= 0x80000000 {
        free(m, al, m.word(out), h);
        m.put_word(out, 0)
    }
    r
}

pub fn fn_173510(m: &mut Memory<'_>, p: u32, h: &mut impl Host) -> u32 {
    if p == 0 {
        return INVALID;
    }
    let r = h.call_m(method(m, m.word(p), 80), &[m.word(p + 8)], m);
    if r < 0x80000000 {
        free(m, m.word(p + 4), p, h)
    }
    r
}

pub fn fn_1735e0(m: &mut Memory<'_>, p: u32, q: u32, params: [u32; 4], h: &mut impl Host) -> u32 {
    let mut copied = params;
    let mut out = 0;
    if p == 0 || q == 0 {
        return INVALID;
    }
    h.params_call_m(q, &mut copied, p, &mut out, 0, m)
}

pub fn fn_1734e8(m: &mut Memory<'_>, s: u32, arg: u32, h: &mut impl Host) -> u32 {
    if s == 0 || arg == 0 {
        return INVALID;
    }
    h.call_m(method(m, m.word(s), 0x4c), &[m.word(s + 8), arg], m)
}
