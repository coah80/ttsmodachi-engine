
use crate::{boot::Memory, frontend};

pub struct Environment {
    pub cached_handle: [u8; 8],
    pub default_context: u32,
}

pub trait Host: frontend::HeapVtable + frontend::MethodTable {

    fn kernel_literals(&self)->[u32;7] {[0;7]}
    fn call(&mut self, memory: &mut Memory<'_>, entry: u32, args: &[u32]) -> u32;
    fn output(&mut self, memory: &mut Memory<'_>, entry: u32, args: &[u32]) -> (u32, u32);
}
fn half(m: &Memory<'_>, p: u32) -> u16 {
    u16::from_le_bytes([m.byte(p), m.byte(p + 1)])
}
fn opt(m: &Memory<'_>, p: u32) -> Option<u32> {
    if p == 0 {
        None
    } else {
        Some(m.offset(p) as u32)
    }
}
pub(crate) fn resolve(m: &Memory<'_>, h: [u32; 2], e: &Environment) -> (u32, u32) {
    let mut out = 0;
    let cached = [
        u32::from_le_bytes(e.cached_handle[..4].try_into().unwrap()),
        u32::from_le_bytes(e.cached_handle[4..].try_into().unwrap()),
    ];

    let pointed_word = if h != cached && h[0] != 0 && h[1] == 0x1e61 {
        Some(half(m, h[0]))
    } else {
        None
    };
    let rc = frontend::resolve_engine_context(
        h[0],
        pointed_word,
        h[1],
        &e.cached_handle,
        e.default_context,
        &mut out,
    );
    (rc, out)
}
fn release(m: &Memory<'_>, heap: u32, p: u32, host: &mut impl Host) {

    frontend::heap_release(
        m.bytes,
        opt(m, heap),
        if p == 0 { None } else { Some(p) },
        host,
    );
}
fn clear_cursor_byte(m: &mut Memory<'_>, p: u32) -> u32 {
    if p == 0 {
        return crate::normalization::byte_clear_4a(m.bytes, 0);
    }
    let offset = m.offset(p);
    if offset != 0 {
        return crate::normalization::byte_clear_4a(m.bytes, offset as u32);
    }

    let mut view = [0u8; 76];
    view[75] = m.bytes[74];
    let rc = crate::normalization::byte_clear_4a(&mut view, 1);
    m.bytes[74] = view[75];
    rc
}

pub fn fn_2c4728(m: &mut Memory<'_>, p: u32, value: u32) -> u32 {
    if p == 0 {
        return 0x80002007;
    }
    m.put_word(p + 12, value);
    let i = m.offset(p + 21);
    m.bytes[i] = 1;
    m.put_word(p + 16, value);
    let q = m.word(p + 8);
    clear_cursor_byte(m, q)
}

pub fn fn_2c61a0(
    m: &mut Memory<'_>,
    p: u32,
    descriptors: u32,
    n: i32,
    host: &mut impl Host,
) -> u32 {
    if p == 0 {
        return 0x81102007;
    }
    let count = half(m, p + 16) as u32;
    let heap = m.word(m.word(p + 8) + 4);
    if count == 0 {
        let b = frontend::call_method(
            m.bytes,
            opt(m, heap),
            1,
            (n as u32).wrapping_mul(16).wrapping_add(1),
            host,
        );
        m.put_word(p + 12, b);
        if b == 0 {
            return 0x8110200a;
        }
    } else {
        let b = frontend::heap_realloc(
            m.bytes,
            opt(m, heap),
            m.word(p + 12),
            count
                .wrapping_add(n as u32)
                .wrapping_mul(16)
                .wrapping_add(1),
            host,
        );
        if b == 0 {
            return 0x8110200a;
        }
        m.put_word(p + 12, b);
    }
    for i in 0..n.max(0) as u32 {
        let dst = m
            .word(p + 12)
            .wrapping_add((half(m, p + 16) as u32 + i) * 16);
        let src = descriptors.wrapping_add(i * 16);
        let d = m.offset(dst) as u32;
        let s = m.offset(src) as u32;
        crate::normalization::memcpy_at(m.bytes, d, s, 16);
    }
    m.put_half(p + 16, (half(m, p + 16) as i32).wrapping_add(n) as u16);
    0
}

pub fn fn_2c62e8(m: &mut Memory<'_>, p: u32, host: &mut impl Host) -> u32 {
    if p == 0 {
        return 0x81102007;
    }
    let mut i = 0u16;
    while i < half(m, p + 16) {
        let b = m.word(p + 12) + i as u32 * 16;
        let arg = m.word(b + 12);
        if arg != 0 {
            let rc = host.call(m, m.word(b + 8), &[arg]);
            if rc >= 0x80000000 {
                return rc;
            }
        }
        i = i.wrapping_add(1);
    }
    release(m, m.word(m.word(p + 8) + 4), m.word(p + 12), host);
    m.put_word(p + 12, 0);
    m.put_half(p + 16, 0);
    0
}

pub fn fn_2c6370(m: &mut Memory<'_>, p: u32, host: &mut impl Host) -> u32 {
    if p == 0 {
        return 0x81102007;
    }
    let context = m.word(p + 8);
    let rc = fn_2c62e8(m, p, host);
    if rc >= 0x80000000 {
        return rc;
    }
    if m.word(p + 12) != 0 {
        release(m, m.word(context + 4), m.word(p + 12), host);
        m.put_half(p + 16, 0);
    }
    release(m, m.word(context + 4), p, host);
    0
}

pub fn fn_2c3e2c(m: &mut Memory<'_>, p: u32, e: &Environment, host: &mut impl Host) -> u32 {
    if p == 0 {
        return 0x81102007;
    }
    let (rc, c) = resolve(m, [m.word(p), m.word(p + 4)], e);
    if rc >= 0x80000000 {
        return 0x81102007;
    }
    release(m, m.word(c + 4), p, host);
    0
}

pub fn fn_2c464c(
    m: &mut Memory<'_>,
    handle: [u32; 2],
    a: u32,
    b: u32,
    out: u32,
    data: u32,
    e: &Environment,
    host: &mut impl Host,
) -> u32 {
    if out == 0 {
        return 0x80002007;
    }
    let (rc, c) = resolve(m, handle, e);
    if rc >= 0x80000000 {
        return 0x80002007;
    }
    m.put_word(out, 0);
    let heap = m.word(c + 4);
    let p = frontend::heap_alloc(m.bytes, opt(m, heap), 24, host);
    if p == 0 {
        return 0x8000200a;
    }
    let i = m.offset(p) as u32;
    crate::normalization::memset_at(m.bytes, i, 0, 24);
    m.put_word(p, handle[0]);
    m.put_word(p + 4, handle[1]);
    let rc = host.call(m, 0x2eee7c, &[handle[0], handle[1], a, b, p + 8, data]);
    if rc < 0x80000000 {
        m.put_word(out, p);
        return 0;
    }
    release(m, heap, p, host);
    rc
}

pub fn fn_2c47d0(
    m: &mut Memory<'_>,
    p: u32,
    out: u32,
    e: &Environment,
    host: &mut impl Host,
) -> u32 {
    if p == 0 {
        return 0x80002007;
    }
    let (rc, owner) = resolve(m, [m.word(p), m.word(p + 4)], e);
    if rc >= 0x80000000 {
        return 0x80002007;
    }
    m.put_word(p + 12, 0);
    m.put_word(p + 16, 0);
    let i = m.offset(p + 21);
    m.bytes[i] = 0;
    let q = m.word(p + 8);
    let mut rc = clear_cursor_byte(m, q);
    let mut entries = 0;
    if rc < 0x80000000 {
        (rc, entries) = host.output(m, 0x2ef034, &[q]);
        if rc < 0x80000000 {
            for i in 1..40u32 {
                let entry = entries + i * 40;
                if half(m, entry + 10) == 31 {
                    m.put_word(out, m.word(entry + 28));
                }
                if matches!(
                    half(m, entry + 10),
                    6 | 12 | 21 | 50 | 36 | 39 | 37 | 34 | 22 | 40 | 26 | 29
                ) && m.word(entry + 28) != 0
                {
                    release(m, m.word(owner + 4), m.word(entry + 28), host);
                    m.put_word(entry + 28, 0);
                }
            }
        }
    }
    if entries != 0 {
        release(m, m.word(owner + 4), entries, host);
    }
    rc
}

pub fn fn_2b92d8(
    m: &mut Memory<'_>,
    p: u32,
    tables: u32,
    triple: u32,
    host: &mut impl Host,
) -> u32 {
    for i in 0..4u32 {
        let rc = host.call(m, 0x175a24, &[m.word(p + 20), m.word(tables + i * 8 + 4)]);
        if rc & 0x1fff != 20 && rc >= 0x80000000 {
            return rc;
        }
    }

    let t = [m.word(triple), m.word(triple + 4), m.word(triple + 8)];
    for key in t {
        let rc = host.call(m, 0x175a24, &[m.word(p + 20), key]);
        if rc & 0x1fff != 20 && rc >= 0x80000000 {
            return rc;
        }
    }
    0
}

pub fn fn_2c3008(m: &mut Memory<'_>, p: u32, e: &Environment, host: &mut impl Host) -> u32 {
    if p == 0 {
        return 0x81102007;
    }
    let (rc, c) = resolve(m, [m.word(p), m.word(p + 4)], e);
    if rc >= 0x80000000 {
        return 0x81102007;
    }
    host.call(m, 0x2eca2c, &[m.word(p + 12)]);
    host.call(m, 0x2eca2c, &[m.word(p + 8)]);
    release(m, m.word(c + 4), p, host);
    0
}

pub fn fn_2c3308(
    m: &mut Memory<'_>,
    p: u32,
    s: u32,
    out: u32,
    sizes: u32,
    host: &mut impl Host,
) -> u32 {
    if out != 0 {
        let at = m.offset(out);
        m.bytes[at] = 0;
    } else {
        m.put_half(sizes, 1);
    }
    let n = crate::normalization::strlen8(m.bytes, m.offset(s) as u32) as u16 as u32;
    if n == 0 || p == 0 {
        return 0;
    }
    let mut q = s;
    while m.byte(q) != 0 {
        if m.byte(q) == b' ' {
            let i = m.offset(q);
            m.bytes[i] = 0x11;
        }
        q += 1;
    }
    let mut i = 0i32;
    while (n as i32) > i {
        let (tlen, tok) = host.output(m, 0x2eccb4, &[p, s.wrapping_add(i as u32)]);
        if (tlen as i32) <= 0 {
            i = i.wrapping_add(1);
        } else {
            let a = crate::normalization::strlen8(m.bytes, m.offset(tok) as u32);
            if out != 0 {
                let b = crate::normalization::strlen8(m.bytes, m.offset(out) as u32);
                if a.wrapping_add(b).wrapping_add(1) > half(m, sizes) as u32 {
                    return 0x81102009;
                }
                let d = m.offset(out) as u32;
                let s = m.offset(tok) as u32;
                crate::normalization::strcat_at(m.bytes, d, s);
            } else {
                m.put_half(sizes, a.wrapping_add(half(m, sizes) as u32) as u16);
            }
            i = i.wrapping_add(tlen as i32);
        }
    }
    if out != 0 {
        let mut q = out;
        while m.byte(q) != 0 {
            if m.byte(q) == b' ' {
                let i = m.offset(q);
                m.bytes[i] = 0x11;
            }
            q += 1;
        }
    }
    0
}

pub fn fn_2b0c04(
    m: &mut Memory<'_>,
    p: u32,
    e: &Environment,
    classes: &[u16; 256],
    host: &mut impl Host,
) -> u32 {
    let (rc, context) = resolve(m, [m.word(p), m.word(p + 4)], e);
    if rc >= 0x80000000 {
        return rc;
    }
    let error;
    if m.byte(p + 8) == 0 && m.byte(p + 136) == 0 {
        host.call(m, 0x173dc0, &[m.word(context + 16), 0x3a9a]);
        error = 0x88102007;
    } else {
        let flag = m.offset(m.word(p + 268));
        m.bytes[flag] = 0;
        if frontend::compare_folded(classes, &m.bytes[m.offset(p + 8)..], b"UTF-8\0") == 0 {
            let flag = m.offset(m.word(p + 268));
            m.bytes[flag] = 1;
            m.put_word(p + 264, 0);
            return 0;
        }
        host.call(m, 0x173dc0, &[m.word(context + 16), 0x3a9c, p + 8, p + 136]);
        error = 0x88102001;
    }
    m.put_word(p + 264, error);
    error
}

pub fn fn_2b0d28(
    m: &mut Memory<'_>,
    p: u32,
    a: [u32; 7],
    e: &Environment,
    tables: &crate::normalization::DecoderTables<'_>,
    host: &mut impl Host,
) -> u32 {
    m.put_word(a[2], 0);
    m.put_word(a[5], 0);
    let (rc, context) = resolve(m, [m.word(p), m.word(p + 4)], e);
    if rc >= 0x80000000 {
        return rc;
    }
    if m.byte(m.word(p + 268)) != 1 {
        return 0x8810200e;
    }
    let input = opt(m, a[0]).unwrap_or(0);
    let output = opt(m, a[3]).unwrap_or(0);
    let positions = opt(m, a[6]).unwrap_or(0);

    let (mut rc, consumed, written) =
        crate::normalization::decode_run(m.bytes, input, a[1], output, a[4], positions, tables);
    m.put_word(a[2], consumed);
    m.put_word(a[5], written);
    if rc < 0x80000000 && consumed != a[1] {
        rc = 0x8bc02009;
    }
    m.put_word(a[5], m.word(a[5]).wrapping_mul(2));
    match rc & 0x1fff {
        0 | 7 => (),
        9 => {
            host.call(m, 0x173dc0, &[m.word(context + 16), 0x3aa7]);
        }
        _ => {
            host.call(m, 0x173dc0, &[m.word(context + 16), 0x3aa8, rc]);
        }
    }
    rc
}

pub fn fn_2b9708(m: &mut Memory<'_>, key: u32, table: u32, special: u32) -> u32 {
    if key == 43 {
        return special;
    }
    for i in 0..17 {
        let p = table + i * 8;
        if m.byte(p) as u32 == key {
            return m.word(p + 4);
        }
    }
    m.word(table + 4)
}

pub fn fn_2b9cb0(m: &mut Memory<'_>, p: u32, host: &mut impl Host) -> u32 {
    if half(m, p + 44) != 0 {
        if m.word(p + 48) != 0 {
            let mut i = 0u16;
            while i < half(m, p + 44) {
                for off in [4, 12, 20] {
                    let at = m.word(p + 48) + i as u32 * 28 + off;
                    let value = m.word(at);
                    if value != 0 {
                        release(m, m.word(m.word(p + 8) + 4), value, host);

                        m.put_word(m.word(p + 48) + i as u32 * 28 + off, 0);
                    }
                }
                i = i.wrapping_add(1);
            }
            release(m, m.word(m.word(p + 8) + 4), m.word(p + 48), host);
            m.put_word(p + 48, 0);
        }
        m.put_half(p + 44, 0);
    }
    0
}

fn payload_half(m: &Memory<'_>, address: u32) -> u16 {
    let mut local = [0u8; 2];
    let at = m.offset(address);
    frontend::copy_bytes(&mut local, &m.bytes[at..at + 2], 2);
    u16::from_le_bytes(local)
}

fn calloc_bytes(m: &mut Memory<'_>, heap: u32, size: u32, host: &mut impl Host) -> u32 {
    frontend::call_method(m.bytes, opt(m, heap), 1, size, host)
}

pub fn fn_2b9da4(
    m: &mut Memory<'_>,
    p: u32,
    _a: u32,
    _b: u32,
    data: u32,
    host: &mut impl Host,
) -> u32 {
    if half(m, p + 44) != 0 {
        fn_2b9cb0(m, p, host);
    }
    let dst = m.offset(p + 44) as u32;
    let src = m.offset(data) as u32;
    crate::normalization::memcpy_at(m.bytes, dst, src, 2);
    let array = calloc_bytes(
        m,
        m.word(m.word(p + 8) + 4),
        (half(m, p + 44) as u32).wrapping_mul(28).wrapping_add(1),
        host,
    );
    m.put_word(p + 48, array);
    if array == 0 {
        m.put_half(p + 44, 0);
        fn_2b9cb0(m, p, host);
        return 0x8110200a;
    }
    let mut cursor = 2u32;
    let mut i = 0u16;
    while i < half(m, p + 44) {
        for off in [0, 1, 24] {
            let value = payload_half(m, data.wrapping_add(cursor)) as u8;
            cursor = cursor.wrapping_add(2);
            let at = m.offset(m.word(p + 48) + i as u32 * 28 + off);
            m.bytes[at] = value;
        }
        for (len_off, ptr_off) in [(2, 4), (8, 12), (16, 20)] {
            let n = payload_half(m, data.wrapping_add(cursor));
            cursor = cursor.wrapping_add(2);
            let mut buf = 0;
            if n != 0 {
                buf = calloc_bytes(m, m.word(m.word(p + 8) + 4), n as u32 + 1, host);
                if buf == 0 {
                    fn_2b9cb0(m, p, host);
                    return 0x8110200a;
                }
                for j in 0..n as u32 {
                    let value = payload_half(m, data.wrapping_add(cursor)) as u8;
                    cursor = cursor.wrapping_add(2);
                    let at = m.offset(buf + j);
                    m.bytes[at] = value;
                }
            }
            let rec = m.word(p + 48) + i as u32 * 28;
            m.put_half(rec + len_off, n);
            m.put_word(rec + ptr_off, buf);
        }
        i = i.wrapping_add(1);
    }
    0
}

fn destroy_callback(m: &mut Memory<'_>, object: u32, e: &Environment, host: &mut impl Host) -> u32 {
    use crate::render_part_a2::{destroy_object, ContextGlobals, ObjectHost};
    struct Bridge<'a, 'b, H> {
        memory: &'a mut Memory<'b>,
        host: &'a mut H,
        object: u32,
        context: u32,
    }
    fn word_at(bytes: &mut [u8], at: usize, value: u32) {
        bytes[at..at + 4].copy_from_slice(&value.to_le_bytes());
    }
    fn heap_window(m: &Memory<'_>, context: u32, view: &mut [u8]) {
        let heap = m.word(context + 4);
        word_at(view, 128 + 4, if heap == 0 { 0 } else { 384 });
        if heap != 0 {
            word_at(view, 384 + 4, m.word(heap + 4));
        }
    }
    impl<H: Host> frontend::HeapVtable for Bridge<'_, '_, H> {
        fn alloc_at(&mut self, _: u32, _: u32) -> u32 {
            unreachable!("destroy allocates nothing")
        }
        fn realloc_at(&mut self, _: u32, _: u32, _: u32) -> u32 {
            unreachable!("destroy reallocates nothing")
        }
        fn release_at(&mut self, cookie: u32, object: u32) {
            debug_assert_eq!(object, 16);
            self.host.release_at(cookie, self.object);
        }
    }
    impl<H: Host> ObjectHost for Bridge<'_, '_, H> {
        fn destroy(&mut self, view: &mut [u8], method: u32, first: u32, second: u32) -> u32 {
            let rc = self.host.call(self.memory, method, &[first, second]);
            if rc < 0x80000000 {
                heap_window(self.memory, self.context, view);
            }
            rc
        }
    }
    let (rc, context) = resolve(m, [m.word(object), m.word(object + 4)], e);
    let mut view = [0u8; 400];
    word_at(&mut view, 16, 128);
    word_at(&mut view, 20, if rc < 0x80000000 { 0x1e61 } else { 0xffff });
    view[128..130].copy_from_slice(&0x1e61u16.to_le_bytes());
    if rc < 0x80000000 {
        word_at(&mut view, 16 + 48, 256);
        word_at(&mut view, 256 + 40, m.word(m.word(object + 48) + 40));
        word_at(&mut view, 16 + 52, m.word(object + 52));
        word_at(&mut view, 16 + 56, m.word(object + 56));
        heap_window(m, context, &mut view);
    }
    let globals = ContextGlobals {
        pair: [0; 8],
        default_context: 128,
    };
    let mut bridge = Bridge {
        memory: m,
        host,
        object,
        context,
    };
    destroy_object(&mut view, 16, &globals, &mut bridge)
}

pub fn fn_2ba2ac(m: &mut Memory<'_>, p: u32, e: &Environment, host: &mut impl Host) -> u32 {
    if p == 0 {
        return 0x81102007;
    }
    let (rc, env) = resolve(m, [m.word(p), m.word(p + 4)], e);
    if rc >= 0x80000000 {
        return 0x81102007;
    }
    let cb = m.word(p + 12);
    if cb != 0 {
        destroy_callback(m, cb, e, host);
    }
    release(m, m.word(env + 4), p, host);
    0
}

pub fn fn_2ba6b4(m: &mut Memory<'_>, p: u32, host: &mut impl Host) -> u32 {
    if p == 0 {
        return 0x81102007;
    }
    let lang = m.word(p + 64);
    if lang != 0 {

        let _ = frontend::check_handle(m.word(m.word(p + 8) + 20) != 0);
    }
    let rc = fn_2581ac(m, m.word(p + 12), host);
    if rc < 0x80000000 {
        0
    } else {
        rc
    }
}

pub fn fn_2bb4c8(m: &mut Memory<'_>, p: u32, request: u32, host: &mut impl Host) -> u32 {
    if p == 0 || request == 0 {
        return 0x81102007;
    }
    host.call(m, 0x259aa8, &[m.word(p + 12), request])
}

fn skip_index(
    m: &Memory<'_>,
    table: u32,
    mut index: u16,
    max: u16,
    back: bool,
    separators: &[u8],
) -> u16 {
    if table == 0 {
        return index;
    }
    while if back { index != 0 } else { index < max } {
        let mut view = [0u8; 8];
        let ch = half(m, table + index as u32 * 2).to_le_bytes();
        if back {
            view[3..5].copy_from_slice(&ch);
            view[5] = 1;
            crate::normalization::skip_matched_back(&mut view, 1, 5, separators);
            if view[5] == 1 {
                break;
            }
            index = index.wrapping_sub(1);
        } else {
            view[1..3].copy_from_slice(&ch);
            crate::normalization::skip_matched(&mut view, 1, 5, 1, separators);
            if view[5] == 0 {
                break;
            }
            index = index.wrapping_add(1);
        }
    }
    index
}

struct NormalHost<'a, H> {
    environment: &'a Environment,
    host: &'a mut H,
}
impl<H: Host> crate::normalization::NormalizationHost for NormalHost<'_, H> {
    fn calloc(&mut self, arena: &mut [u8], heap: u32, n: u32, size: u32) -> Option<u32> {

        if heap == 0 || n == 0 || size == 0 {
            return None;
        }
        let mut m = Memory { base: 0, bytes: arena };
        let p = self.host.call(&mut m, 0x1963b8, &[heap, n, size]);
        if p == 0 { None } else { Some(p) }
    }
    fn alloc(&mut self, arena: &mut [u8], heap: u32, size: u32) -> Option<u32> {
        if heap == 0 || size == 0 {
            return None;
        }
        let mut m = Memory { base: 0, bytes: arena };
        let p = self.host.call(&mut m, 0x196394, &[heap, size]);
        if p == 0 { None } else { Some(p) }
    }
    fn realloc(&mut self, arena: &mut [u8], heap: u32, old: u32, size: u32) -> Option<u32> {
        if heap == 0 || size == 0 {
            return None;
        }
        let mut m = Memory { base: 0, bytes: arena };
        let p = self.host.call(&mut m, 0x1963e0, &[heap, old, size]);
        if p == 0 { None } else { Some(p) }
    }
    fn free(&mut self, arena: &mut [u8], heap: u32, p: u32) {
        if heap == 0 || p == 0 {
            return;
        }
        let mut m = Memory { base: 0, bytes: arena };
        self.host.call(&mut m, 0x196428, &[heap, p]);
    }
    fn resolve(&mut self, arena: &mut [u8], h0: u32, h1: u32) -> (u32, u32) {
        resolve(
            &Memory {
                base: 0,
                bytes: arena,
            },
            [h0, h1],
            self.environment,
        )
    }
}
fn iterator_entry(m: &Memory<'_>, p: u32, n: u32, out: &mut i32) -> u32 {
    if p == 0 {
        return 0x81102007;
    }
    let table = m.word(p + 12);
    crate::normalization::entry_offset(
        half(m, table + 2),
        m.word(table + 4) as i32,
        half(m, table + 8),
        half(m, p + 16),
        n,
        out,
    )
}
pub(crate) fn iterator_count(m: &Memory<'_>, p: u32, out: &mut u16) -> u32 {
    if p == 0 {
        return crate::normalization::inner_field(None, Some(out));
    }
    let table = m.word(p + 12);
    crate::normalization::inner_field(Some(&m.bytes[m.offset(table)..]), Some(out))
}

pub fn fn_2bb4e4(
    m: &mut Memory<'_>,
    object: u32,
    table: u32,
    text: u32,
    out: u32,
    e: &Environment,
    host: &mut impl Host,
) -> u32 {
    assert_eq!(m.base, 0, "fn_2bb4e4 requires absolute-index bridge memory");
    let length = crate::normalization::strlen8(m.bytes, text) as u16;
    let h = [m.word(object), m.word(object + 4)];
    let mut iterator = 0;
    let mut bridge = NormalHost {
        environment: e,
        host,
    };
    let rc = crate::normalization::open_iterator(
        m.bytes,
        h[0],
        h[1],
        table,
        0x3fd,
        Some(&mut iterator),
        &mut bridge,
    );
    if rc >= 0x80000000 {
        return rc;
    }
    let result = (|| {
        let mut entry = 0;
        let rc = iterator_entry(m, iterator, 0, &mut entry);
        if rc >= 0x80000000 {
            return rc;
        }
        let first = entry as u32;
        let mut saved = [0u8; 40];
        saved.copy_from_slice(&m.bytes[first as usize..first as usize + 40]);
        m.put_word(first + 16, 0);
        m.put_word(first + 24, 0);
        let rc = iterator_entry(m, iterator, 0, &mut entry);
        if rc >= 0x80000000 {
            return rc;
        }
        m.bytes[entry as usize..entry as usize + 40].copy_from_slice(&saved);
        let mut count = 0;
        let rc = iterator_count(m, iterator, &mut count);
        if rc >= 0x80000000 {
            return rc;
        }
        let rc = iterator_entry(m, iterator, 0, &mut entry);
        if rc >= 0x80000000 {
            return rc;
        }
        let events = entry as u32;
        for i in 1..count {
            let p = events + i as u32 * 40;
            let kind = half(m, p + 10);
            if kind != 1 && kind != 37 && kind != 14 {
                continue;
            }
            let mut pos = m.word(p + 20).wrapping_sub(m.word(events + 20)) as u16;
            if m.byte(text + pos as u32) == b' ' {
                while pos < length && m.byte(text + pos as u32) == b' ' {
                    pos = pos.wrapping_add(1);
                    m.put_word(p + 20, m.word(p + 20).wrapping_add(1));
                }
            } else {
                while pos != 0 && m.byte(text + pos as u32) != b' ' {
                    if m.byte(text + pos as u32 - 1) == b' ' {
                        break;
                    }
                    pos = pos.wrapping_sub(1);
                    m.put_word(p + 20, m.word(p + 20).wrapping_sub(1));
                }
            }
            let begin = m.word(p + 20).wrapping_sub(m.word(events + 20)) as u16;
            if kind != 1 {
                continue;
            }
            let mut end = begin;
            while end < length && m.byte(text + end as u32) != b' ' {
                end = end.wrapping_add(1);
            }
            let mut j = i;
            while j < count {
                let q = events + j as u32 * 40;
                if half(m, q + 10) == 21 && m.word(q + 20) > m.word(p + 20) {
                    break;
                }
                j = j.wrapping_add(1);
            }
            let span = (end as u32).wrapping_sub(begin as u32);
            if j == count
                || m.word(p + 20).wrapping_add(span) <= m.word(events + j as u32 * 40 + 20)
            {
                m.put_word(p + 24, span);
            }
        }
        let mut i = 1u16;
        while i < count {
            let p = events + i as u32 * 40;
            if half(m, p + 10) == 0x4000 {
                i = i.wrapping_add(1);
                continue;
            }
            if (i as u32) + 1 < count as u32 {
                let q = p + 40;
                if m.word(p + 20) > m.word(q + 20)
                    || (m.word(p + 20) == m.word(q + 20)
                        && (m.word(p + 24) > m.word(q + 24)
                            || (m.word(p + 24) == m.word(q + 24)
                                && m.word(p + 12) > m.word(q + 12))))
                {
                    let mut swap = [0u8; 120];
                    swap[40..120].copy_from_slice(&m.bytes[p as usize..p as usize + 80]);
                    crate::normalization::memcpy_at(&mut swap, 0, 80, 40);
                    swap.copy_within(40..80, 80);
                    crate::normalization::memcpy_at(&mut swap, 40, 0, 40);
                    m.bytes[p as usize..p as usize + 80].copy_from_slice(&swap[40..120]);
                    i = 0;
                    continue;
                }
            }
            i = i.wrapping_add(1);
        }
        i = 1;
        while i < count {
            let p = events + i as u32 * 40;
            let mut merged = false;
            if half(m, p + 10) == 1 {
                let mut j = i.wrapping_add(1);
                while j < count && m.word(events + j as u32 * 40 + 20) == m.word(p + 20) {
                    let q = events + j as u32 * 40;
                    if half(m, q + 10) == 1 {
                        let length = m
                            .word(q + 12)
                            .wrapping_sub(m.word(events + 12))
                            .wrapping_add(m.word(q + 16))
                            .wrapping_sub(m.word(p + 12).wrapping_sub(m.word(events + 12)));
                        m.put_word(p + 16, length);
                        if (j as u32) + 1 < count as u32 {
                            crate::normalization::memmove_at(
                                m.bytes,
                                q,
                                q + 40,
                                (count as u32 - j as u32 - 1) * 40,
                            );
                        }
                        count = count.wrapping_sub(1);
                        merged = true;
                        break;
                    }
                    j = j.wrapping_add(1);
                }
            }
            if !merged {
                i = i.wrapping_add(1);
            }
        }
        m.put_half(out, count);
        rc
    })();
    if iterator != 0 {
        crate::normalization::release_iterator(m.bytes, iterator, &mut bridge);
    }
    result
}

pub fn fn_2bf6c4(
    m: &mut Memory<'_>,
    p: u32,
    e: &Environment,
    resource_name: u32,
    registry: &frontend::ReleaseScripts,
    callback: Option<impl FnMut(u32, u32, &mut [u8; 32])>,
    host: &mut impl Host,
) -> u32 {
    if p == 0 {
        return 0x81102007;
    }
    let (rc, ctx) = resolve(m, [m.word(p), m.word(p + 4)], e);
    if rc >= 0x80000000 {
        return 0x81102007;
    }
    if m.word(p + 20) != 0 {
        assert_eq!(
            m.base, 0,
            "registry release requires absolute-index bridge memory"
        );
        let comp = m.word(ctx + 24);
        let _ = frontend::release_ling_entry(
            m.bytes,
            if comp == 0 { None } else { Some(comp) },
            if resource_name == 0 {
                None
            } else {
                Some(resource_name)
            },
            registry,
            callback,
        );
    }
    let array = m.word(p + 32);
    if array != 0 {
        release(m, m.word(ctx + 4), array, host);
        m.put_word(p + 32, 0);
        m.put_half(p + 36, 0);
    }
    release(m, m.word(ctx + 4), p, host);
    0
}

pub struct ParserConstructorData<'a> {

    pub names: [u32; 4],

    pub callbacks: [u32; 2],
    pub acquire: frontend::AcquireScripts,
    pub release: frontend::ReleaseScripts,
    pub classes: &'a [u16; 256],

    pub stack: u32,
}

pub fn fn_2bf760(
    m: &mut Memory<'_>,
    h: [u32; 2],
    a: [u32; 5],
    e: &Environment,
    data: &ParserConstructorData<'_>,
    release_callback: Option<impl FnMut(u32, u32, &mut [u8; 32])>,
    host: &mut impl Host,
) -> u32 {
    assert_eq!(m.base, 0, "fn_2bf760 requires absolute-index bridge memory");
    let out = a[3];
    if out == 0 {
        return 0x81102007;
    }
    let (rc, ctx) = resolve(m, h, e);
    if rc >= 0x80000000 {
        return 0x81102007;
    }
    m.put_word(out, 0);
    let heap = m.word(ctx + 4);
    let p = frontend::heap_alloc(
        m.bytes,
        if heap == 0 { None } else { Some(heap) },
        104,
        host,
    );
    if p == 0 {
        return 0x8110200a;
    }
    crate::normalization::memset_at(m.bytes, p, 0, 104);
    m.put_word(p, h[0]);
    m.put_word(p + 4, h[1]);
    m.put_word(p + 8, ctx);
    m.put_word(p + 12, a[0]);
    m.put_word(p + 16, a[1]);
    m.put_word(p + 40, a[4]);
    let stack = data.stack;
    let mut saved = [0u8; 32];
    saved.copy_from_slice(&m.bytes[stack as usize..stack as usize + 32]);
    for (i, v) in [data.names[0], data.callbacks[0], data.callbacks[1], p]
        .iter()
        .enumerate()
    {
        m.put_word(stack + i as u32 * 4, *v);
    }
    let mut rc = fn_2c61a0(m, a[4], stack, 1, host);
    let mut success = false;
    let mut allocation_failure = false;
    if rc < 0x80000000 {
        m.put_word(stack + 16, 0);
        let comp = m.word(ctx + 24);
        let query = frontend::acquire_ling_entry(
            m.bytes,
            if comp == 0 { None } else { Some(comp) },
            if data.names[1] == 0 {
                None
            } else {
                Some(data.names[1])
            },
            Some(stack + 16),
            &data.acquire,
        );
        if query < 0x80000000 {
            let module = m.word(stack + 16);
            m.put_word(p + 20, m.word(module + 4));
            m.put_word(p + 24, m.word(module + 8));
            m.put_word(p + 28, m.word(module + 12));
        }
        let interface = m.word(p + 20);
        if interface == 0 {
            success = true;
        } else {
            m.put_word(stack + 20, 0);
            m.put_half(stack + 24, 0xffff);
            let flag_at = m.offset(stack + 26);
            m.bytes[flag_at] = 0;
            rc = host.call(
                m,
                m.word(interface + 28),
                &[
                    m.word(p + 24),
                    m.word(p + 28),
                    data.names[2],
                    data.names[3],
                    stack + 20,
                    stack + 24,
                    stack + 26,
                ],
            );
            if rc < 0x80000000 {
                let list = m.word(stack + 20);
                if half(m, stack + 24) == 1 && list != 0 {
                    let flag = m.byte(stack + 26);
                    let text = m.word(list);
                    let delimiter = frontend::find_byte(m.bytes, text, flag as u32);
                    if delimiter != 0 {
                        let at = m.offset(delimiter);
                        m.bytes[at] = 0;
                    }
                    let mut s = text;
                    m.put_half(p + 36, 1);
                    loop {
                        let delimiter = frontend::find_byte(m.bytes, s, b'|' as u32);
                        if delimiter == 0 {
                            break;
                        }
                        m.put_half(p + 36, half(m, p + 36).wrapping_add(1));
                        s = delimiter + 1;
                    }
                    let heap = m.word(ctx + 4);
                    let array = frontend::call_method(
                        m.bytes,
                        if heap == 0 { None } else { Some(heap) },
                        1,
                        1 + half(m, p + 36) as u32 * 2,
                        host,
                    );
                    m.put_word(p + 32, array);
                    if array == 0 {
                        rc = 0x8110200a;
                        allocation_failure = true;
                    } else {
                        s = m.word(list);
                        m.put_half(p + 36, 0);
                        while s != 0 && m.byte(s) != flag {
                            let delimiter = frontend::find_byte(m.bytes, s, b'|' as u32);
                            if delimiter != 0 {
                                let at = m.offset(delimiter);
                                m.bytes[at] = 0;
                            }
                            let value = frontend::parse_int(data.classes, m.bytes, s);
                            let n = half(m, p + 36) as u32;
                            m.put_half(m.word(p + 32) + n * 2, value as u16);
                            m.put_half(p + 36, (n as u16).wrapping_add(1));
                            if delimiter == 0 {
                                break;
                            }
                            s = delimiter + 1;
                        }
                        success = true;
                    }
                } else {
                    success = true;
                }
            }
        }
    }
    m.bytes[stack as usize..stack as usize + 32].copy_from_slice(&saved);
    if success {
        m.put_word(out, p);
        return 0;
    }

    if !allocation_failure {
        fn_2bf6c4(
            m,
            p,
            e,
            data.names[1],
            &data.release,
            release_callback,
            host,
        );
    }
    rc
}

pub fn fn_2ba608(
    m: &mut Memory<'_>,
    p: u32,
    e: &Environment,
    langcode: u32,
    category: u32,
    stack: u32,
    compare: impl FnMut(u32, u32) -> i32,
    host: &mut (impl Host + frontend::StateLock),
) -> u32 {
    if p == 0 {
        return 0x81102007;
    }
    let (rc, ctx) = resolve(m, [m.word(p), m.word(p + 4)], e);
    if rc >= 0x80000000 {
        return 0x81102007;
    }
    if half(m, p + 32) != 0 {
        assert_eq!(
            m.base, 0,
            "parameter lookup requires absolute-index bridge memory"
        );
        let params = m.word(m.word(p + 8) + 20);
        let mut value = m.word(p + 64);
        let rc = frontend::lookup_param(
            m.bytes,
            if params == 0 { None } else { Some(params) },
            langcode,
            Some(&mut value),
            None,
            host,
            compare,
        );
        m.put_word(p + 64, value);
        if rc >= 0x80000000 {
            return rc;
        }
        let rc = host.call(m, 0x25807c, &[m.word(p + 12)]);
        return if rc < 0x80000000 { 0 } else { rc };
    }
    struct Log<'a, H>(&'a mut H);
    impl<H: Host> crate::boot_part_b::Host for Log<'_, H> {
        fn call(&mut self, m: &mut Memory<'_>, f: u32, a: &[u32]) -> u32 {
            self.0.call(m, f, a)
        }
        fn thread_id(&mut self) -> u32 {
            unreachable!("no thread query in null-format logger")
        }
    }
    impl<H: Host> crate::boot_part_b::FormatHost for Log<'_, H> {}
    let mut saved = [0u8; 20];
    let off = m.offset(stack);
    saved.copy_from_slice(&m.bytes[off..off + 20]);
    crate::render_part_a::fn_173dc0(
        m,
        &mut Log(host),
        m.word(ctx + 16),
        category,
        0x712,
        0,
        0,
        stack,
    );
    m.bytes[off..off + 20].copy_from_slice(&saved);
    0x81102007
}

pub struct ConstructorData {
    pub parameter_names: [u32; 3],
    pub descriptor_names: [u32; 2],
    pub symbol_callbacks: [u32; 2],
    pub error_callbacks: [u32; 2],

    pub stack: u32,
}

pub fn fn_2ba304(
    m: &mut Memory<'_>,
    h: [u32; 2],
    a: [u32; 13],
    e: &Environment,
    data: &ConstructorData,
    mut compare: impl FnMut(u32, u32) -> i32,
    host: &mut (impl Host + frontend::StateLock),
) -> u32 {
    assert_eq!(m.base, 0, "fn_2ba304 requires absolute-index bridge memory");
    let out = a[2];
    if out == 0 {
        return 0x81102007;
    }
    let (rc, env) = resolve(m, h, e);
    if rc >= 0x80000000 {
        return 0x81102007;
    }
    m.put_word(out, 0);
    let heap = m.word(env + 4);
    let p = frontend::heap_alloc(m.bytes, opt(m, heap), 76, host);
    if p == 0 {
        return 0x8110200a;
    }
    let off = m.offset(p);
    crate::normalization::memset_at(m.bytes, off as u32, 0, 76);
    m.put_word(p, h[0]);
    m.put_word(p + 4, h[1]);
    m.put_word(p + 8, env);
    m.put_word(p + 16, a[9]);
    m.put_word(p + 24, a[0]);
    m.put_word(p + 28, a[1]);
    m.put_word(p + 52, a[3]);
    m.put_word(p + 56, a[4]);
    m.put_word(p + 60, a[5]);
    let stack = data.stack;
    let mut saved = [0u8; 36];
    saved.copy_from_slice(&m.bytes[stack as usize..stack as usize + 36]);
    m.put_word(stack, 0);
    for (i, field, default) in [(0, 70, 512), (1, 68, 512), (2, 72, 1)] {
        m.put_half(p + field, default);
        let params = m.word(env + 20);
        let rc = frontend::get_param_uint(
            m.bytes,
            if params == 0 { None } else { Some(params) },
            data.parameter_names[i],
            stack,
            host,
            &mut compare,
        );
        if rc < 0x80000000 {
            m.put_half(p + field, m.word(stack) as u16);
        }
    }
    let rc = host.call(
        m,
        0x25795c,
        &[
            h[0],
            h[1],
            p + 12,
            a[3],
            a[4],
            a[5],
            a[6],
            a[7],
            a[8],
            a[10],
            a[11],
            a[12],
            half(m, p + 70) as u32,
            half(m, p + 68) as u32,
            half(m, p + 72) as u32,
        ],
    );
    let rc = if rc >= 0x80000000 {
        rc
    } else {
        for (i, values) in [
            [
                data.descriptor_names[0],
                data.symbol_callbacks[0],
                data.symbol_callbacks[1],
                p,
            ],
            [
                data.descriptor_names[1],
                data.error_callbacks[0],
                data.error_callbacks[1],
                p,
            ],
        ]
        .iter()
        .enumerate()
        {
            for (j, value) in values.iter().enumerate() {
                m.put_word(stack + (i * 16 + j * 4) as u32, *value);
            }
        }
        fn_2c61a0(m, a[9], stack, 2, host)
    };
    m.bytes[stack as usize..stack as usize + 36].copy_from_slice(&saved);
    if rc < 0x80000000 {
        m.put_word(out, p);
        return 0;
    }

    fn_2ba2ac(m, p, e, host);
    rc
}

pub fn fn_2581ac(m: &mut Memory<'_>, p: u32, host: &mut impl Host) -> u32 {
    if p == 0 {
        return 0x81102007;
    }
    let plugin = m.word(p + 48);
    let method = m.word(plugin + 48);
    let first = m.word(p + 52);
    let second = m.word(p + 56);
    let rc = host.call(m, method, &[first, second]);
    if rc >= 0x80000000 {
        return rc;
    }
    if m.word(p + 12) != 0 {
        let _ = frontend::check_handle(m.word(m.word(p + 8) + 20) != 0);
    }
    let buffer = m.word(p + 68);
    if buffer != 0 {
        release(m, m.word(m.word(p + 8) + 4), buffer, host);
        m.put_word(p + 68, 0);
        m.put_half(p + 74, 0);
    }
    rc
}

pub fn fn_2be754(
    m: &mut Memory<'_>,
    _unused: u32,
    timeline: u32,
    list: u32,
    position: u32,
    index: u32,
    separators: &[u8],
) -> u32 {
    let events = m.word(list + 4);
    let mut i = half(m, index);
    let candidate = |m: &Memory<'_>, i: u16| {
        let event = events + i as u32 * 40;
        let delta = m.word(event + 20).wrapping_sub(m.word(events + 20));
        let typ = half(m, event + 10);
        (delta as u16 as u32) > half(m, position) as u32 * half(m, timeline + 2) as u32
            && (((typ == 2 || typ == 3) && m.word(event + 28) == 1) || typ == 12 || typ == 20)
    };
    while i < half(m, list + 8) && !candidate(m, i) {
        i = i.wrapping_add(1);
    }
    while i < half(m, list + 8) {
        if candidate(m, i) {
            let delta = m
                .word(events + i as u32 * 40 + 20)
                .wrapping_sub(m.word(events + 20));
            let middle = frontend::divmod_u32(delta, 2).0 as u16;
            let table = m.word(timeline + 4);
            let lower = skip_index(m, table, middle, 0, true, separators)
                .wrapping_mul(half(m, timeline + 2));
            let upper = skip_index(m, table, middle, half(m, timeline + 8), false, separators)
                .wrapping_mul(half(m, timeline + 2));
            let mut found = false;
            for j in 0..half(m, list + 8) as u32 {
                let event = events + j * 40;
                let relative = m.word(event + 20).wrapping_sub(m.word(events + 20)) as u16;
                if relative > upper {
                    break;
                }
                if half(m, event + 10) == 3 && m.word(event + 28) == 0 && relative >= lower {
                    found = true;
                    break;
                }
            }
            if !found {
                break;
            }
        }
        i = i.wrapping_add(1);
    }
    let result = if i < half(m, list + 8) {
        frontend::divmod_u32(
            m.word(events + i as u32 * 40 + 20)
                .wrapping_sub(m.word(events + 20)) as u16 as u32,
            half(m, timeline + 2) as u32,
        )
        .0
    } else {
        half(m, timeline + 8) as u32
    };
    m.put_half(position, result as u16);
    m.put_half(index, i);
    0
}

pub struct DumpData {
    pub kinds: [(u16, u32); 40],
    pub symbols: [(u8, u32); 17],
    pub plus: u32,
    pub category: u32,

    pub strings: [u32; 5],

    pub stack: u32,
}
fn dump_log(
    m: &mut Memory<'_>,
    logger: u32,
    data: &DumpData,
    format: u32,
    args: &[u32],
    host: &mut impl crate::log_live::LogMethodsM,
) {
    for (i, value) in args.iter().enumerate() {
        m.put_word(data.stack + i as u32 * 4, *value);
    }
    crate::log_live::fn_1738e8(m, logger, data.category, 5, 0, format, data.stack, host);
}

pub fn fn_2b9770(
    m: &mut Memory<'_>,
    flag: u32,
    ctx: u32,
    s1: u32,
    s2: u32,
    tag: u32,
    data: &DumpData,
    host: &mut (impl Host + crate::log_live::LogMethodsM),
) -> u32 {
    if flag == 0 {
        return 0;
    }
    let mut len = 0;
    let rc = iterator_count(m, s1, &mut len);
    if rc >= 0x80000000 {
        return rc;
    }
    let mut buf = 0;
    if len != 0 {
        let mut text = 0;
        let rc = iterator_entry(m, s1, 0, &mut text);
        if rc >= 0x80000000 {
            return rc;
        }
        buf = frontend::call_method(
            m.bytes,
            opt(m, m.word(ctx + 4)),
            1,
            len as u32 * 2 + 2,
            host,
        );
        if buf == 0 {
            return 0x8110200a;
        }
        let dst = m.offset(buf) as u32;
        let src = m.offset(text as u32) as u32;
        crate::fe_text::narrow_n_17649c(m.bytes, dst, src, len as u32);
        let at = m.offset(buf + len as u32);
        m.bytes[at] = 0;
    }
    let mut saved = [0u8; 128];
    let scratch = m.offset(data.stack);
    saved.copy_from_slice(&m.bytes[scratch..scratch + 128]);
    let logger = m.word(ctx + 16);
    dump_log(
        m,
        logger,
        data,
        data.strings[2],
        &[
            tag,
            len as u32,
            if len == 0 { data.strings[0] } else { buf },
        ],
        host,
    );
    let mut records = 0;
    let mut rc = iterator_entry(m, s2, 0, &mut records);
    let mut count = 0;
    if rc < 0x80000000 {
        rc = iterator_count(m, s2, &mut count);
    }
    if rc < 0x80000000 {
        for i in 0..count as u32 {
            let rec = records as u32 + i * 40;
            let kind = half(m, rec + 10);
            let value = m.word(rec + 28);
            let name = if kind == 7 {
                if value as u8 == 43 {
                    data.plus
                } else {
                    data.symbols
                        .iter()
                        .find(|x| x.0 == value as u8)
                        .unwrap_or(&data.symbols[0])
                        .1
                }
            } else {
                data.kinds
                    .iter()
                    .find(|x| x.0 == kind)
                    .unwrap_or(&data.kinds[0])
                    .1
            };
            let namebuf = data.stack + 64;
            let text = m.word(rec + 32);
            if text == 0 {
                let at = m.offset(namebuf);
                m.bytes[at] = 0;
            } else {
                let src = m.offset(text) as u32;
                let n = crate::fe_text::strlen16_176624(m.bytes, src) as u16;
                let n = if n > 32 { 31 } else { n } as u32;
                let dst = m.offset(namebuf) as u32;
                crate::fe_text::narrow_n_17649c(m.bytes, dst, src, n);
                let at = m.offset(namebuf + n);
                m.bytes[at] = 0;
            }
            if kind == 32 {
                let at = m.offset(namebuf);
                m.bytes[at] = if value == 1 { b'1' } else { b'0' };
                m.bytes[at + 1] = 0;
            }
            dump_log(
                m,
                m.word(ctx + 16),
                data,
                data.strings[3],
                &[
                    i,
                    data.strings[if m.byte(rec) == 1 { 1 } else { 0 }],
                    kind as u32,
                    name,
                    m.word(rec + 12),
                    m.word(rec + 16),
                    m.word(rec + 20),
                    m.word(rec + 24),
                    namebuf,
                ],
                host,
            );
            let mut k = 0u32;
            while k < half(m, rec + 8) as u32 {
                let ents = m.word(rec + 4) + k * 8;
                dump_log(
                    m,
                    m.word(ctx + 16),
                    data,
                    data.strings[4],
                    &[k, m.word(ents), m.word(ents + 4)],
                    host,
                );
                k += 1;
            }
        }
    }
    m.bytes[scratch..scratch + 128].copy_from_slice(&saved);
    if buf != 0 {
        release(m, m.word(ctx + 4), buf, host);
    }
    rc
}

pub fn fn_2be178(m: &mut Memory<'_>, _unused: u32, a: u32, b: u32, d: u32, e: u32) -> u32 {
    let limit = half(m, a + 8);
    let n = half(m, d);
    let events = m.word(b + 4);
    if n < limit {
        let n = n.wrapping_add(1);
        m.put_half(d, n);
        let mut i = half(m, e);
        while i < half(m, b + 8) {
            if m.word(events + 20)
                .wrapping_add(n as u32 * half(m, a + 2) as u32)
                <= m.word(events + i as u32 * 40 + 20)
            {
                break;
            }
            i = i.wrapping_add(1);
        }
        m.put_half(e, i);
    } else {
        m.put_half(e, half(m, b + 8));
    }
    0
}

pub fn fn_2bd850(m: &mut Memory<'_>, current: u32, previous: u32) -> u32 {
    if current == 0 || previous == 0 {
        return 0;
    }
    fn_2bd850_slices(
        Some(&m.bytes[m.offset(current)..]),
        Some(&m.bytes[m.offset(previous)..]),
    )
}

pub fn fn_2bd850_slices(current: Option<&[u8]>, previous: Option<&[u8]>) -> u32 {
    let (Some(current), Some(previous)) = (current, previous) else {
        return 0;
    };
    let word = |bytes: &[u8], at: usize| u32::from_le_bytes(bytes[at..at + 4].try_into().unwrap());
    (word(previous, 20).wrapping_add(word(previous, 24)) <= word(current, 20)) as u32
}

pub struct BoundaryData<'a> {
    pub separators: u32,
    pub max_length_key: u32,
    pub ctype: &'a [u16; 256],
    pub category: u32,

    pub scratch: u32,
}
pub trait BoundaryHost: Host + frontend::StateLock + crate::boot_part_b::FormatHost {}
impl<T: Host + frontend::StateLock + crate::boot_part_b::FormatHost> BoundaryHost for T {}
fn boundary_cap(
    m: &mut Memory<'_>,
    ctx: u32,
    data: &BoundaryData<'_>,
    host: &mut impl BoundaryHost,
    compare: &mut impl FnMut(u32, u32) -> i32,
) -> u32 {
    let properties = m.word(ctx + 20);
    let mut cap = 750;
    if properties != 0 {
        let saved = m.word(data.scratch);
        let rc = frontend::get_param_int(
            data.ctype,
            m.bytes,
            Some(properties),
            data.max_length_key,
            data.scratch,
            host,
            compare,
        );
        let value = (m.word(data.scratch) as i32).wrapping_mul(2);
        m.put_word(data.scratch, saved);
        if rc < 0x80000000 {
            cap = if value < 2 {
                2
            } else if value > 10000 {
                10000
            } else {
                (value as u16 as u32 + 1) & !1
            };
        }
    }
    cap
}
fn boundary_error(
    m: &mut Memory<'_>,
    ctx: u32,
    code: u32,
    data: &BoundaryData<'_>,
    host: &mut impl BoundaryHost,
) {
    let mut saved = [0u8; 20];
    let at = m.offset(data.scratch);
    saved.copy_from_slice(&m.bytes[at..at + 20]);
    crate::render_part_a::error_log(
        m,
        host,
        m.word(ctx + 16),
        data.category,
        code,
        0,
        0,
        data.scratch,
    );
    m.bytes[at..at + 20].copy_from_slice(&saved);
}

fn boundary_forward(
    m: &mut Memory<'_>,
    text: u32,
    idx: u32,
    limit: u32,
    newline: bool,
    separators: u32,
) {
    if text == 0 || idx == 0 {
        return;
    }
    while (half(m, idx) as u32) < limit {
        let c = half(m, text + half(m, idx) as u32 * 2);
        let stop = if newline {
            c == 10
        } else {
            crate::fe_text::strchr16_176514(m.bytes, m.offset(separators) as u32, c as u32) != 0
        };
        if stop {
            break;
        }
        m.put_half(idx, half(m, idx).wrapping_add(1));
    }
}
fn boundary_backward(m: &mut Memory<'_>, text: u32, idx: u32, separators: u32) {
    if text == 0 || idx == 0 {
        return;
    }
    while half(m, idx) != 0 {
        let c = half(m, text + half(m, idx) as u32 * 2);
        if crate::fe_text::strchr16_176514(m.bytes, m.offset(separators) as u32, c as u32) != 0 {
            break;
        }
        m.put_half(idx, half(m, idx) - 1);
    }
}

pub fn fn_2be1fc(
    m: &mut Memory<'_>,
    ctx: u32,
    a: u32,
    q: u32,
    idx: u32,
    out: u32,
    data: &BoundaryData<'_>,
    host: &mut impl BoundaryHost,
    compare: &mut impl FnMut(u32, u32) -> i32,
) -> u32 {
    assert_eq!(m.base, 0, "property ports use offset-addressed memory");
    let events = m.word(q + 4);
    let old = half(m, idx);
    boundary_forward(
        m,
        m.word(a + 4),
        idx,
        half(m, a + 8) as u32,
        false,
        data.separators,
    );
    let cap = boundary_cap(m, ctx, data, host, compare);
    if half(m, idx) as u32 > cap {
        boundary_error(m, ctx, 0x716, data, host);
        m.put_half(idx, cap as u16);
    }
    let cur = half(m, idx);
    if cur == old {
        m.put_half(out, half(m, q + 8));
        return 0;
    }
    let n = half(m, q + 8) as u32;
    let mut i = 0u32;
    let mut mode = false;
    let mut save = 0u32;
    while i < n {
        let ev = events + i * 40;
        if cur as u32
            <= frontend::divmod_u32(m.word(ev + 20).wrapping_sub(m.word(events + 20)), 2).0
            || mode
        {
            break;
        }
        if half(m, ev + 10) == 7 {
            if m.word(ev + 28) == 43 {
                mode = true;
                save = i;
            } else {
                mode = false;
            }
        }
        i += 1;
    }
    if mode {
        let ev = events + save * 40;
        let quot = frontend::divmod_u32(m.word(ev + 20).wrapping_sub(m.word(events + 20)), 2).0;
        if quot as u16 == 0 && half(m, ev + 10) == 7 {
            i = save + 1;
            while i < n && half(m, events + i * 40 + 10) != 7 {
                i += 1;
            }
            if i < n && half(m, events + i * 40 + 10) == 7 {
                m.put_half(
                    idx,
                    frontend::divmod_u32(
                        m.word(events + i * 40 + 20)
                            .wrapping_sub(m.word(events + 20)),
                        2,
                    )
                    .0 as u16,
                );
                m.put_half(out, (i + 1) as u16);
            }
        } else {
            m.put_half(idx, quot as u16);
            m.put_half(out, save as u16);
            return 0;
        }
    }
    let mut k = half(m, out);
    while k < half(m, q + 8) {
        if (half(m, idx) as u32 * half(m, a + 2) as u32).wrapping_add(m.word(events + 20))
            <= m.word(events + k as u32 * 40 + 20)
        {
            break;
        }
        k = k.wrapping_add(1);
    }
    m.put_half(out, k);
    0
}

pub fn fn_2be4a0(
    m: &mut Memory<'_>,
    ctx: u32,
    a: u32,
    q: u32,
    idx: u32,
    out: u32,
    data: &BoundaryData<'_>,
    host: &mut impl BoundaryHost,
    compare: &mut impl FnMut(u32, u32) -> i32,
) -> u32 {
    assert_eq!(m.base, 0, "property ports use offset-addressed memory");
    let events = m.word(q + 4);
    let entry = half(m, idx);
    boundary_forward(
        m,
        m.word(a + 4),
        idx,
        half(m, a + 8) as u32,
        true,
        data.separators,
    );
    let cap = boundary_cap(m, ctx, data, host, compare);
    if half(m, idx) as u32 > cap {
        m.put_half(idx, cap as u16);
        if cap == half(m, a + 8) as u32 {
            m.put_half(idx, cap.wrapping_sub(1) as u16);
        }
        boundary_backward(m, m.word(a + 4), idx, data.separators);
        if half(m, idx) == 0 {
            m.put_half(idx, cap as u16);
        }
        let mut i = 0;
        let mut mode = false;
        let mut save = 0;
        let n = half(m, q + 8) as u32;
        while i < n {
            let ev = events + i * 40;
            if frontend::divmod_u32(m.word(ev + 20).wrapping_sub(m.word(events + 20)), 2).0
                > half(m, idx) as u32
            {
                break;
            }
            if half(m, ev + 10) == 7 {
                if m.word(ev + 28) == 43 {
                    save = i;
                    mode = !mode;
                } else {
                    mode = false;
                }
            }
            i += 1;
        }
        if mode {
            m.put_half(
                idx,
                frontend::divmod_u32(
                    m.word(events + save * 40 + 20)
                        .wrapping_sub(m.word(events + 20)),
                    2,
                )
                .0 as u16,
            );
        }
        boundary_error(m, ctx, 0x717, data, host);
    }
    let cur = half(m, idx);
    if cur == entry {
        m.put_half(out, half(m, q + 8));
        return 0;
    }
    let mut j = half(m, out);
    while j < half(m, q + 8) {
        let ev = events + j as u32 * 40;
        if (cur as u32 * half(m, a + 2) as u32).wrapping_add(m.word(events + 20)) <= m.word(ev + 20)
        {
            break;
        }
        let rel = m.word(ev + 20).wrapping_sub(m.word(events + 20)) as u16;
        if rel as u32 <= cur as u32 * half(m, a + 2) as u32 {
            let t = half(m, ev + 10);
            if (t == 12 || t == 20) && m.word(ev + 20) != m.word(events + 20) {
                m.put_half(
                    idx,
                    frontend::divmod_u32(rel as u32, half(m, a + 2) as u32).0 as u16,
                );
                m.put_half(out, j);
                return 0;
            }
        }
        j = j.wrapping_add(1);
    }
    m.put_half(out, j);
    0
}

pub trait ParserMethodsM {
    fn method_m(&mut self, method: u32, args: &[u32], mem: &mut Memory<'_>) -> u32;
}
pub struct ParserConstructorDataM<'a> {
    pub names: [u32; 4],
    pub callbacks: [u32; 2],
    pub classes: &'a [u16; 256],

    pub stack: u32,
}
fn parser_alloc_m(
    m: &mut Memory<'_>,
    heap: u32,
    a: u32,
    b: Option<u32>,
    host: &mut impl ParserMethodsM,
) -> u32 {
    if heap == 0 || a == 0 || b == Some(0) {
        return 0;
    }
    let vt = m.word(heap);
    let object = m.word(heap + 4);
    if let Some(b) = b {
        host.method_m(m.word(vt + 4), &[object, a, b], m)
    } else {
        host.method_m(m.word(vt), &[object, a], m)
    }
}
fn parser_free_m(m: &mut Memory<'_>, heap: u32, p: u32, host: &mut impl ParserMethodsM) {
    if heap != 0 && p != 0 {
        host.method_m(m.word(m.word(heap) + 12), &[m.word(heap + 4), p], m);
    }
}

pub fn fn_2c61a0_m(
    m: &mut Memory<'_>,
    p: u32,
    descriptors: u32,
    n: i32,
    host: &mut impl ParserMethodsM,
) -> u32 {
    if p == 0 {
        return 0x81102007;
    }
    let count = half(m, p + 16) as u32;
    let heap = m.word(m.word(p + 8) + 4);
    if count == 0 {
        let b = parser_alloc_m(
            m,
            heap,
            1,
            Some((n as u32).wrapping_mul(16).wrapping_add(1)),
            host,
        );
        m.put_word(p + 12, b);
        if b == 0 {
            return 0x8110200a;
        }
    } else {
        let size = count
            .wrapping_add(n as u32)
            .wrapping_mul(16)
            .wrapping_add(1);
        let old = m.word(p + 12);
        let b = if heap == 0 || size == 0 {
            0
        } else if old == 0 {
            parser_alloc_m(m, heap, size, None, host)
        } else {
            host.method_m(m.word(m.word(heap) + 8), &[m.word(heap + 4), old, size], m)
        };
        if b == 0 {
            return 0x8110200a;
        }
        m.put_word(p + 12, b);
    }
    for i in 0..n.max(0) as u32 {
        let dst = m.word(p + 12) + (half(m, p + 16) as u32 + i) * 16;
        crate::normalization::memcpy_at(m.bytes, dst, descriptors + i * 16, 16);
    }
    m.put_half(p + 16, (half(m, p + 16) as i32).wrapping_add(n) as u16);
    0
}
fn registry_lock_m(
    m: &mut Memory<'_>,
    owner: u32,
    unlock: bool,
    host: &mut impl ParserMethodsM,
) -> u32 {
    if owner == 0 {
        return 0x81402007;
    }
    let slot = m.word(m.word(m.word(owner) + 8) + if unlock { 12 } else { 8 });
    if slot == 0 {
        0
    } else {
        host.method_m(slot, &[m.word(owner + 8)], m)
    }
}

pub(crate) fn registry_find_m(
    m: &mut Memory<'_>,
    map: u32,
    key: u32,
    host: &mut impl ParserMethodsM,
) -> Option<u32> {
    let tree = map + 24;
    let mut node = m.word(tree);
    while node != tree {
        let method = m.word(m.word(tree + 36) + 8);
        let order = host.method_m(method, &[key, m.word(node + 16)], m) as i32;
        if order < 0 {
            node = m.word(node);
        } else if order > 0 {
            node = m.word(node + 4);
        } else {
            break;
        }
    }
    if node == tree {
        return None;
    }
    if m.byte(tree + 40) == 0 {
        return Some(node);
    }
    loop {
        let last = node;
        node = m.word(last);
        while node != tree {
            let method = m.word(m.word(tree + 36) + 8);
            if host.method_m(method, &[key, m.word(node + 16)], m) == 0 {
                break;
            }
            node = m.word(node + 4);
        }
        if node == tree {
            return Some(last);
        }
    }
}
fn registry_lookup_m(
    m: &mut Memory<'_>,
    map: u32,
    key: u32,
    host: &mut impl ParserMethodsM,
) -> (u32, u32) {
    if map == 0 || key == 0 {
        return (0x89002007, 0);
    }
    match registry_find_m(m, map, key, host) {
        Some(n) => (0, m.word(n + 20)),
        None => (0x89002014, 0),
    }
}

pub fn fn_17b058_m(
    m: &mut Memory<'_>,
    o: u32,
    name: u32,
    out: u32,
    host: &mut impl ParserMethodsM,
) -> u32 {
    if o == 0 || name == 0 || out == 0 {
        return 0x8b602007;
    }
    let lock = registry_lock_m(m, m.word(o + 12), false, host);
    if lock >= 0x80000000 {
        return lock;
    }
    let (mut rc, slot) = registry_lookup_m(m, m.word(o + 16), name, host);
    if rc >= 0x80000000 {
        rc = (rc & 0x1fff) | 0x8b602000;
    } else {
        m.put_word(out, slot);
        m.put_half(slot + 28, half(m, slot + 28).wrapping_add(1));
    }
    let status = registry_lock_m(m, m.word(o + 12), true, host);
    if status >= 0x80000000 && rc < 0x80000000 {
        rc = status;
    }
    rc
}
pub(crate) fn registry_release_m(
    m: &mut Memory<'_>,
    o: u32,
    name: u32,
    scratch: u32,
    host: &mut impl ParserMethodsM,
) -> u32 {
    if o == 0 || name == 0 {
        return 0x8b602007;
    }
    let lock = registry_lock_m(m, m.word(o + 12), false, host);
    if lock >= 0x80000000 {
        return lock;
    }
    let (mut rc, entry) = registry_lookup_m(m, m.word(o + 16), name, host);
    if rc >= 0x80000000 {
        rc = (rc & 0x1fff) | 0x8b602000;
    } else {
        let refs = half(m, entry + 28);
        if refs == 0 {
            rc = 0x8b602011;
        } else if refs != 1 || m.word(entry + 24) == 0 {
            m.put_half(entry + 28, refs - 1);
        } else {
            let mut saved = [0; 32];
            saved.copy_from_slice(&m.bytes[scratch as usize..scratch as usize + 32]);
            crate::normalization::memcpy_at(m.bytes, scratch, entry, 32);
            let map = m.word(o + 16);
            if let Some(node) = registry_find_m(m, map, name, host) {
                let tree = map + 24;
                crate::normalization::rb_erase(m.bytes, tree, node);
                let key = m.word(node + 16);
                if key != 0 {
                    host.method_m(m.word(m.word(tree + 36) + 4), &[key, m.word(tree + 32)], m);
                }
                let value = m.word(node + 20);
                if value != 0 {
                    host.method_m(
                        m.word(m.word(tree + 36) + 16),
                        &[value, m.word(tree + 32)],
                        m,
                    );
                }
                parser_free_m(m, m.word(tree + 32), node, host);
                rc = 0;
            } else {
                rc = 0x89002014;
            }
            host.method_m(
                m.word(scratch + 24),
                &[m.word(o), m.word(o + 4), scratch],
                m,
            );
            m.bytes[scratch as usize..scratch as usize + 32].copy_from_slice(&saved);
        }
    }
    let status = registry_lock_m(m, m.word(o + 12), true, host);
    if status >= 0x80000000 && rc < 0x80000000 {
        rc = status;
    }
    rc
}
pub fn fn_2bf6c4_m(
    m: &mut Memory<'_>,
    p: u32,
    e: &Environment,
    name: u32,
    scratch: u32,
    host: &mut impl ParserMethodsM,
) -> u32 {
    if p == 0 {
        return 0x81102007;
    }
    let (rc, ctx) = resolve(m, [m.word(p), m.word(p + 4)], e);
    if rc >= 0x80000000 {
        return 0x81102007;
    }
    if m.word(p + 20) != 0 {
        registry_release_m(m, m.word(ctx + 24), name, scratch, host);
    }
    let array = m.word(p + 32);
    if array != 0 {
        parser_free_m(m, m.word(ctx + 4), array, host);
        m.put_word(p + 32, 0);
        m.put_half(p + 36, 0);
    }
    parser_free_m(m, m.word(ctx + 4), p, host);
    0
}

pub fn fn_2bf760_m(
    m: &mut Memory<'_>,
    h: [u32; 2],
    a: [u32; 5],
    e: &Environment,
    data: &ParserConstructorDataM<'_>,
    host: &mut impl ParserMethodsM,
) -> u32 {
    assert_eq!(m.base, 0, "fn_2bf760 requires absolute-index bridge memory");
    let out = a[3];
    if out == 0 {
        return 0x81102007;
    }
    let (rc, ctx) = resolve(m, h, e);
    if rc >= 0x80000000 {
        return 0x81102007;
    }
    m.put_word(out, 0);
    let heap = m.word(ctx + 4);
    let p = parser_alloc_m(m, heap, 104, None, host);
    if p == 0 {
        return 0x8110200a;
    }
    crate::normalization::memset_at(m.bytes, p, 0, 104);
    m.put_word(p, h[0]);
    m.put_word(p + 4, h[1]);
    m.put_word(p + 8, ctx);
    m.put_word(p + 12, a[0]);
    m.put_word(p + 16, a[1]);
    m.put_word(p + 40, a[4]);
    let stack = data.stack;
    let mut saved = [0u8; 32];
    saved.copy_from_slice(&m.bytes[stack as usize..stack as usize + 32]);
    for (i, v) in [data.names[0], data.callbacks[0], data.callbacks[1], p]
        .iter()
        .enumerate()
    {
        m.put_word(stack + i as u32 * 4, *v);
    }
    let mut rc = fn_2c61a0_m(m, a[4], stack, 1, host);
    let mut success = false;
    let mut allocation_failure = false;
    if rc < 0x80000000 {
        m.put_word(stack + 16, 0);
        let comp = m.word(ctx + 24);
        let query = fn_17b058_m(m, comp, data.names[1], stack + 16, host);
        if query < 0x80000000 {
            let module = m.word(stack + 16);
            m.put_word(p + 20, m.word(module + 4));
            m.put_word(p + 24, m.word(module + 8));
            m.put_word(p + 28, m.word(module + 12));
        }
        let interface = m.word(p + 20);
        if interface == 0 {
            success = true;
        } else {
            m.put_word(stack + 20, 0);
            m.put_half(stack + 24, 0xffff);
            let flag_at = m.offset(stack + 26);
            m.bytes[flag_at] = 0;
            rc = host.method_m(
                m.word(interface + 28),
                &[
                    m.word(p + 24),
                    m.word(p + 28),
                    data.names[2],
                    data.names[3],
                    stack + 20,
                    stack + 24,
                    stack + 26,
                ],
                m,
            );
            if rc < 0x80000000 {
                let list = m.word(stack + 20);
                if half(m, stack + 24) == 1 && list != 0 {
                    let flag = m.byte(stack + 26);
                    let text = m.word(list);
                    let delimiter = frontend::find_byte(m.bytes, text, flag as u32);
                    if delimiter != 0 {
                        let at = m.offset(delimiter);
                        m.bytes[at] = 0;
                    }
                    let mut s = text;
                    m.put_half(p + 36, 1);
                    loop {
                        let delimiter = frontend::find_byte(m.bytes, s, b'|' as u32);
                        if delimiter == 0 {
                            break;
                        }
                        m.put_half(p + 36, half(m, p + 36).wrapping_add(1));
                        s = delimiter + 1;
                    }
                    let heap = m.word(ctx + 4);
                    let array =
                        parser_alloc_m(m, heap, 1, Some(1 + half(m, p + 36) as u32 * 2), host);
                    m.put_word(p + 32, array);
                    if array == 0 {
                        rc = 0x8110200a;
                        allocation_failure = true;
                    } else {
                        s = m.word(list);
                        m.put_half(p + 36, 0);
                        while s != 0 && m.byte(s) != flag {
                            let delimiter = frontend::find_byte(m.bytes, s, b'|' as u32);
                            if delimiter != 0 {
                                let at = m.offset(delimiter);
                                m.bytes[at] = 0;
                            }
                            let value = frontend::parse_int(data.classes, m.bytes, s);
                            let n = half(m, p + 36) as u32;
                            m.put_half(m.word(p + 32) + n * 2, value as u16);
                            m.put_half(p + 36, (n as u16).wrapping_add(1));
                            if delimiter == 0 {
                                break;
                            }
                            s = delimiter + 1;
                        }
                        success = true;
                    }
                } else {
                    success = true;
                }
            }
        }
    }
    m.bytes[stack as usize..stack as usize + 32].copy_from_slice(&saved);
    if success {
        m.put_word(out, p);
        return 0;
    }

    if !allocation_failure {
        fn_2bf6c4_m(m, p, e, data.names[1], stack + 32, host);
    }
    rc
}

pub struct ParserData<'a> {
    pub eos: &'a [u8; 40],
    pub separators: &'a [u8],
    pub category: u32,
    pub formats: [u32; 5],
    pub extra_key: u32,

    pub scratch: u32,
}
pub(crate) fn set_iterator_index(m: &mut Memory<'_>, it: u32, index: i32) -> u32 {
    if it == 0 {
        return 0x81102007;
    }
    let mut out = half(m, it + 16);
    let rc = crate::normalization::set_index(half(m, m.word(it + 12) + 8), index, &mut out);
    if rc < 0x80000000 {
        m.put_half(it + 16, out);
    }
    rc
}
pub(crate) fn iterator_table(m: &Memory<'_>, it: u32) -> (u32, u32) {
    let mut table = 0;
    let rc = crate::normalization::field_value(
        if it == 0 {
            None
        } else {
            Some(&m.bytes[it as usize..])
        },
        Some(&mut table),
    );
    (rc, table)
}
pub(crate) fn parser_log(
    m: &mut Memory<'_>,
    logger: u32,
    data: &ParserData<'_>,
    format: u32,
    args: &[u32],
    host: &mut impl crate::log_live::LogMethodsM,
) {
    for (i, v) in args.iter().enumerate() {
        m.put_word(data.scratch + 1100 + i as u32 * 4, *v);
    }
    crate::log_live::fn_1738e8(
        m,
        logger,
        data.category,
        5,
        0,
        format,
        data.scratch + 1100,
        host,
    );
}

pub fn fn_2bfa90(
    m: &mut Memory<'_>,
    p: u32,
    text: u32,
    out: u32,
    mode: u32,
    e: &Environment,
    data: &ParserData<'_>,
    host: &mut (impl BoundaryHost + crate::log_live::LogMethodsM),
    compare: &mut impl FnMut(u32, u32) -> i32,
) -> u32 {
    assert_eq!(m.base, 0);
    if p == 0 || text == 0 || out == 0 {
        return 0x81102007;
    }
    let (status, engine) = resolve(m, [m.word(p), m.word(p + 4)], e);
    if status >= 0x80000000 {
        return 0x81102007;
    }
    let s = data.scratch;
    let mut saved = [0u8; 1200];
    saved.copy_from_slice(&m.bytes[s as usize..s as usize + 1200]);
    m.bytes[s as usize..s as usize + 40].copy_from_slice(data.eos);
    let mut bridge = NormalHost {
        environment: e,
        host,
    };
    let mut timeline = 0;
    let mut list = 0;
    let mut strings = 0;
    let mut edit = 0;
    let rc = (|| {
        if half(m, p + 48) == 0 {
            let mut work = [0; 20];
            work.copy_from_slice(&m.bytes[s as usize + 40..s as usize + 60]);
            crate::render_part_a::error_log(
                m,
                bridge.host,
                m.word(engine + 16),
                data.category,
                0x710,
                0,
                0,
                s + 40,
            );
            m.bytes[s as usize + 40..s as usize + 60].copy_from_slice(&work);
            return 0x81102007;
        }
        let rc = crate::normalization::fetch_entry(
            m.bytes,
            text,
            0x3f2,
            1,
            Some(&mut timeline),
            &mut bridge,
        );
        if rc >= 0x80000000 {
            m.bytes[out as usize] = 0;
            return 0;
        }
        let rc = crate::normalization::fetch_entry(
            m.bytes,
            text,
            0x3f3,
            1,
            Some(&mut list),
            &mut bridge,
        );
        if rc >= 0x80000000 {
            return rc;
        }
        let events = m.word(list + 4);
        let count = half(m, list + 8) as u32;
        let rc = crate::normalization::fetch_entry(
            m.bytes,
            text,
            0x3f4,
            1,
            Some(&mut strings),
            &mut bridge,
        );
        if rc >= 0x80000000 {
            return rc;
        }
        let rc = crate::normalization::open_iterator(
            m.bytes,
            m.word(p),
            m.word(p + 4),
            text,
            0x3f3,
            Some(&mut edit),
            &mut bridge,
        );
        if rc >= 0x80000000 {
            return rc;
        }

        let _result = (|| {
            let mut boundary = 0;
            let mut scratch = 0;
            let rc = crate::normalization::scan_table(
                m.bytes,
                timeline,
                Some(&mut boundary),
                list,
                Some(&mut scratch),
                1,
                data.separators,
            );
            if rc >= 0x80000000 {
                return rc;
            }
            let scale = half(m, timeline + 2) as u32;
            let mut i = 1;
            let mut markers = 0u16;
            while i < count
                && m.word(events + i * 40 + 20)
                    .wrapping_sub(m.word(events + 20))
                    <= (boundary as u32 * scale).wrapping_add(m.word(events + 20))
            {
                let t = half(m, events + i * 40 + 10);
                if t != 0x4000 && t != 3 {
                    markers = markers.wrapping_add(1);
                }
                i += 1;
            }
            if markers >= half(m, p + 100) {
                markers = 0;
                i = 1;
                let (rc, table) = iterator_table(m, edit);
                if rc >= 0x80000000 {
                    return rc;
                }
                list = table;
                while i < count
                    && m.word(events + i * 40 + 20)
                        .wrapping_sub(m.word(events + 20))
                        <= scale * boundary as u32
                {
                    let event = events + i * 40;
                    let t = half(m, event + 10);
                    if t != 0x4000 && t != 3 {
                        markers = markers.wrapping_add(1);
                    }
                    if markers == half(m, p + 100) {
                        let rc = set_iterator_index(m, edit, i as i32);
                        if rc >= 0x80000000 {
                            return rc;
                        }
                        m.put_word(s + 12, m.word(event + 12));
                        m.put_word(s + 20, m.word(event + 20));
                        let rc = crate::text_input::insert_span(m.bytes, edit, s, 0, &mut bridge);
                        if rc >= 0x80000000 {
                            return rc;
                        }
                        parser_log(
                            m,
                            m.word(m.word(p + 8) + 16),
                            data,
                            data.formats[0],
                            &[i],
                            bridge.host,
                        );
                        m.bytes[out as usize] = 1;
                        return 0;
                    }
                    i += 1;
                }
            } else if markers == 0 && boundary > 0x400 {
                let (rc, table) = iterator_table(m, edit);
                if rc >= 0x80000000 {
                    return rc;
                }
                list = table;
                boundary = 0x400;
                m.put_word(s + 12, m.word(events + 12));
                m.put_word(s + 20, m.word(events + 20));
                i = 0;
                while i < count
                    && half(m, events + i * 40 + 10) == 0x4000
                    && m.word(events + i * 40 + 20)
                        .wrapping_sub(m.word(events + 20))
                        <= scale * boundary as u32
                {
                    m.put_word(s + 12, m.word(events + i * 40 + 12));
                    m.put_word(s + 20, m.word(events + i * 40 + 20));
                    i += 1;
                }
                let rc = set_iterator_index(m, edit, i as i32);
                if rc >= 0x80000000 {
                    return rc;
                }
                let rc = crate::text_input::insert_span(m.bytes, edit, s, 0, &mut bridge);
                if rc >= 0x80000000 {
                    return rc;
                }
                parser_log(
                    m,
                    m.word(m.word(p + 8) + 16),
                    data,
                    data.formats[1],
                    &[i],
                    bridge.host,
                );
                m.bytes[out as usize] = 1;
                return 0;
            }
            let rc = crate::normalization::scan_table(
                m.bytes,
                timeline,
                Some(&mut boundary),
                list,
                Some(&mut scratch),
                2,
                data.separators,
            );
            if rc >= 0x80000000 {
                return rc;
            }
            let count = half(m, list + 8) as u32;
            for i in 1..count {
                let event = events + i * 40;
                if m.word(event + 20) > (boundary as u32 * 2).wrapping_add(m.word(events + 20)) {
                    break;
                }
                if half(m, event + 10) == 20 {
                    m.put_half(mode, m.word(event + 28) as u16);
                }
            }
            let selected = half(m, mode) as i16;
            let rc = if (2..=4).contains(&selected) {
                parser_boundary_callback(
                    m,
                    timeline,
                    boundary as u32,
                    out,
                    selected,
                    pd_separators(data),
                )
            } else {
                let mut name = 0;
                frontend::fetch_param(
                    m.bytes,
                    if m.word(engine + 20) == 0 {
                        None
                    } else {
                        Some(m.word(engine + 20))
                    },
                    data.extra_key,
                    Some(&mut name),
                    bridge.host,
                    compare,
                );
                let mut value = m.byte(out);
                let rc = crate::text_input::check_boundary(
                    m.bytes,
                    timeline,
                    list,
                    strings,
                    boundary as u32,
                    name,
                    &mut value,
                    s + 64,
                    data.separators,
                );
                m.bytes[out as usize] = value;
                rc
            };
            if rc >= 0x80000000 || m.byte(out) != 0 || m.byte(timeline + 12) == 0 {
                return rc;
            }
            let mut found = false;
            for i in 0..half(m, list + 8) as u32 {
                if matches!(half(m, m.word(list + 4) + i * 40 + 10), 26 | 29 | 40 | 8) {
                    found = true;
                    break;
                }
            }
            m.bytes[out as usize] = found as u8;
            rc
        })();
        crate::normalization::release_iterator(m.bytes, edit, &mut bridge);
        edit = 0;
        0
    })();
    m.bytes[s as usize..s as usize + 1200].copy_from_slice(&saved);
    rc
}

pub struct SyncData {
    pub category: u32,
    pub formats: [u32; 2],
    pub stack: u32,
}
fn sync_log(
    m: &mut Memory<'_>,
    p: u32,
    data: &SyncData,
    format: u32,
    args: &[u32],
    host: &mut impl crate::log_live::LogMethodsM,
) {
    let mut saved = [0u8; 16];
    let at = m.offset(data.stack);
    saved.copy_from_slice(&m.bytes[at..at + 16]);
    for (i, v) in args.iter().enumerate() {
        m.put_word(data.stack + i as u32 * 4, *v);
    }
    crate::log_live::fn_1738e8(
        m,
        m.word(m.word(p + 8) + 16),
        data.category,
        5,
        0,
        format,
        data.stack,
        host,
    );
    m.bytes[at..at + 16].copy_from_slice(&saved);
}
fn sync_fetch(
    m: &mut Memory<'_>,
    res: u32,
    id: u32,
    n: u32,
    e: &Environment,
    host: &mut impl Host,
) -> (u32, u32) {
    let mut out = 0;
    let mut bridge = NormalHost {
        environment: e,
        host,
    };
    let rc = crate::normalization::fetch_entry(m.bytes, res, id, n, Some(&mut out), &mut bridge);
    (rc, out)
}
fn sync_type(m: &Memory<'_>, event: u32) -> u16 {
    half(m, event + 10)
}
fn sync_pos(m: &Memory<'_>, event: u32) -> u32 {
    m.word(event + 12)
}
fn sync_len(m: &Memory<'_>, event: u32) -> u32 {
    m.word(event + 16)
}

pub fn fn_2ba6f0(
    m: &mut Memory<'_>,
    p: u32,
    res: u32,
    off: u32,
    reset: u32,
    src: u32,
    nsrc: u32,
    e: &Environment,
    data: &SyncData,
    host: &mut (impl Host + crate::log_live::LogMethodsM),
) -> u32 {
    assert_eq!(m.base, 0);
    sync_log(m, p, data, data.formats[0], &[reset, off], host);
    let (rc, mut sec) = sync_fetch(m, res, 0x3fd, 1, e, host);
    if rc >= 0x80000000 {
        return rc;
    }
    let mut elems = m.word(sec + 4);
    let count = half(m, sec + 8) as u32;
    let heap = m.word(m.word(p + 8) + 4);
    let map = frontend::call_method(m.bytes, opt(m, heap), 1, count * 2 + 1, host);
    if map == 0 {
        return 0x8110200a;
    }
    let mut cur = 0;
    for i in 0..count {
        let event = elems + i * 40;
        let t = sync_type(m, event);
        let mut v = 0;
        if matches!(t, 7 | 31 | 36 | 21 | 999) || m.byte(event) == 0 {
            if m.byte(event) == 0 {
                for k in cur..nsrc {
                    if sync_type(m, src + k * 40) == t || t == 999 {
                        v = k + 1;
                        cur += 1;
                        break;
                    }
                }
            } else if t != 0x4000
                && t != 999
                && m.byte(event) == 1
                && sync_pos(m, event) == sync_pos(m, elems)
            {
                for k in cur..nsrc {
                    if (sync_type(m, src + k * 40) == t || t == 999)
                        && sync_pos(m, src + k * 40) == sync_pos(m, elems)
                    {
                        v = k + 1;
                        cur += 1;
                        break;
                    }
                }
            }
        }
        m.put_half(map + i * 2, v as u16);
    }
    for i in 0..count {
        let event = elems + i * 40;
        let t = sync_type(m, event);
        if matches!(t, 7 | 31 | 36 | 21 | 999) || m.byte(event) == 0 {
            let v = half(m, map + i * 2) as u32;
            if v != 0 {
                m.put_word(event + 12, sync_pos(m, src + (v - 1) * 40));
                m.put_word(event + 16, sync_len(m, src + (v - 1) * 40));
                if sync_pos(m, event) == sync_pos(m, elems)
                    && sync_pos(m, elems).wrapping_add(sync_len(m, elems))
                        >= sync_pos(m, event).wrapping_sub(off)
                {
                    m.put_word(event + 12, sync_pos(m, event).wrapping_sub(off));
                }
            }
        }
    }
    let sum = off.wrapping_add(reset);
    m.put_word(elems + 12, sync_pos(m, elems).wrapping_sub(sum));
    m.put_word(elems + 16, sync_len(m, elems).wrapping_add(sum));
    for i in 1..count {
        let event = elems + i * 40;
        if m.word(event + 20) == m.word(elems + 20)
            && sync_pos(m, elems).wrapping_add(sum) >= sync_pos(m, event)
        {
            continue;
        }
        let t = sync_type(m, event);
        if t == 0x4000 || (t != 999 && m.byte(event) != 0) {
            continue;
        }
        let mut start = 0;
        for k in (1..count).rev() {
            if k != i
                && sync_type(m, elems + k * 40) != 0x4000
                && sync_pos(m, event) <= sync_pos(m, elems + k * 40)
            {
                start = k;
            }
        }
        let mut delta = sync_len(m, event);
        if start != 0 && m.byte(elems) != 0 {
            delta = sync_pos(m, event)
                .wrapping_add(sync_len(m, event))
                .wrapping_sub(sync_pos(m, elems + start * 40));
        }
        if delta != 0 {
            for k in start..count {
                let next = elems + k * 40;
                if k != i
                    && sync_type(m, next) != 999
                    && m.byte(next) == 1
                    && sync_pos(m, next) >= sync_pos(m, event)
                {
                    if sync_pos(m, next).wrapping_add(delta)
                        <= sync_pos(m, elems).wrapping_add(sync_len(m, elems))
                    {
                        m.put_word(next + 12, sync_pos(m, next).wrapping_add(delta));
                    } else {
                        break;
                    }
                }
            }
        }
    }
    for i in 1..count {
        let event = elems + i * 40;
        if sync_type(m, event) != 41 {
            continue;
        }
        let near = |kind: u16, m: &Memory<'_>| {
            if i + 1 < count
                && sync_type(m, event + 40) == kind
                && m.word(event + 60) == m.word(event + 20)
            {
                i + 1
            } else if i > 1
                && sync_type(m, event - 40) == kind
                && m.word(event - 20) == m.word(event + 20)
            {
                i - 1
            } else {
                0
            }
        };
        let m39 = near(39, m);
        let m14 = near(14, m);
        for j in 1..count {
            let next = elems + j * 40;
            if sync_pos(m, event).wrapping_add(sync_len(m, event)) < sync_pos(m, next) {
                break;
            }
            if j <= i
                || sync_pos(m, next) == 0
                || m.byte(next) != 0
                || matches!(sync_type(m, next), 0x4000 | 7)
                || sync_pos(m, event) > sync_pos(m, next)
                || sync_pos(m, next) >= sync_pos(m, event).wrapping_add(sync_len(m, event))
            {
                continue;
            }
            if sync_pos(m, event)
                .wrapping_add(sync_len(m, event))
                .wrapping_add(sync_len(m, next))
                <= sync_pos(m, elems).wrapping_add(sync_len(m, elems))
            {
                m.put_word(
                    event + 16,
                    sync_len(m, event).wrapping_add(sync_len(m, next)),
                );
            }
            if m39 != 0 {
                m.put_word(elems + m39 * 40 + 16, sync_len(m, event));
            }
            if m14 != 0 {
                m.put_word(elems + m14 * 40 + 16, sync_len(m, event));
            }
        }
    }
    let (rc, sec2) = sync_fetch(m, res, 0x3fc, 1, e, host);
    if rc >= 0x80000000 {
        return rc;
    }
    let mut i = 1;
    while i < count {
        let event = elems + i * 40;
        if sync_type(m, event) == 7 && m.word(event + 28) == 43 {
            let mut matched = false;
            for k in i + 1..count {
                if sync_type(m, elems + k * 40) == 999
                    && m.word(elems + k * 40 + 20)
                        == m.word(event + 20).wrapping_add(m.word(event + 24))
                {
                    i = k;
                    m.put_word(
                        event + 16,
                        sync_pos(m, elems + k * 40)
                            .wrapping_add(sync_len(m, elems + k * 40))
                            .wrapping_sub(sync_pos(m, event)),
                    );
                    matched = true;
                    break;
                }
            }
            if !matched
                && m.word(event + 20).wrapping_add(m.word(event + 24))
                    == m.word(elems + 20)
                        .wrapping_add(half(m, sec2 + 8) as u32 * 2)
            {
                m.put_word(
                    event + 16,
                    sync_pos(m, elems)
                        .wrapping_add(sync_len(m, elems))
                        .wrapping_sub(sync_pos(m, event)),
                );
                let (rc, grown) = sync_fetch(
                    m,
                    res,
                    0x3fd,
                    half(m, sec + 8).wrapping_add(1) as u32,
                    e,
                    host,
                );
                if rc >= 0x80000000 {
                    break;
                }
                sec = grown;
                elems = m.word(sec + 4);
                crate::normalization::memmove_at(
                    m.bytes,
                    elems + (i + 1) * 40,
                    elems + i * 40,
                    (half(m, sec + 8) as u32 - i) * 40,
                );
                let next = elems + (i + 1) * 40;
                let event = elems + i * 40;
                m.put_half(next + 10, 7);
                m.put_word(next + 28, 15);
                m.bytes[next as usize] = 1;
                m.put_word(next + 12, sync_pos(m, event));
                m.put_word(next + 16, 0);
                m.put_word(
                    next + 20,
                    m.word(event + 20).wrapping_add(m.word(event + 24)),
                );
                m.put_word(next + 24, 0);
                m.put_half(next + 8, 0);
                m.put_word(next + 4, 0);
                m.put_word(next + 32, 0);
                m.put_half(sec + 8, half(m, sec + 8).wrapping_add(1));
                i = count;
            }
        }
        i += 1;
    }
    release(m, m.word(m.word(p + 8) + 4), map, host);
    0
}
fn sync_level(m: &mut Memory<'_>, p: u32, host: &mut impl Host) -> u32 {
    if p == 0 {
        return 0;
    }
    let mut max = 1;
    for i in 0..6 {
        let pair = p + if i == 5 { 52 } else { 8 + i * 8 };
        let vt = m.word(pair + 4);
        if vt != 0 {
            let method = m.word(vt + 40);
            if method != 0 {
                max = max.max(host.call(m, method, &[m.word(pair)]));
            }
        }
    }
    if m.byte(p + 49) != 0 {
        max.min(4)
    } else {
        max
    }
}

pub fn fn_2baf40(
    m: &mut Memory<'_>,
    p: u32,
    res: u32,
    off: u32,
    reset: u32,
    reference: u32,
    e: &Environment,
    data: &SyncData,
    pipeline: &crate::render_part_d2::PipelineData<'_>,
    normal: &crate::render_part_a3::Environment<'_>,
    regions: &crate::render_part_a3::RegionData<'_>,
    host: &mut (impl crate::render_part_a3::RegionHost + crate::render_part_d2::PipelineHost),
) -> u32 {
    assert_eq!(m.base, 0);
    if p == 0 || res == 0 {
        return 0x81102007;
    }
    if resolve(m, [m.word(p), m.word(p + 4)], e).0 >= 0x80000000 {
        return 0x81102007;
    }
    let (rc, sec) = sync_fetch(m, res, 0x3fc, 1, e, host);
    if rc >= 0x80000000 {
        return rc;
    }
    if half(m, sec + 8) == 0 {
        sync_log(m, p, data, data.formats[1], &[], host);
        return rc;
    }
    let mut edit = 0;
    let rc = crate::normalization::open_iterator(
        m.bytes,
        m.word(p),
        m.word(p + 4),
        res,
        0x3fd,
        Some(&mut edit),
        &mut NormalHost {
            environment: e,
            host,
        },
    );
    if rc >= 0x80000000 {
        return rc;
    }
    let (mut status, sec3) = sync_fetch(m, res, 0x3fd, 1, e, host);
    let mut src = 0;
    let mut cnt = 0;
    if status < 0x80000000 {
        let elems = m.word(sec3 + 4);
        let mut count = half(m, sec3 + 8) as u32;
        let mut i = 1;
        while i < count {
            let event = elems + i * 40;
            if sync_type(m, event) == 999
                && m.word(event + 20) == m.word(elems + 20)
                && m.word(event + 28) == 0
            {
                if i + 1 < count {
                    crate::normalization::memmove_at(m.bytes, event, event + 40, (count - i) * 40);
                }
                count -= 1;
                i = 1;
            } else {
                i += 1;
            }
        }
        let select = |m: &Memory<'_>, event: u32| {
            let t = sync_type(m, event);
            if t == 0x4000 || m.byte(event) != 0 {
                sync_pos(m, event) == sync_pos(m, elems)
                    && matches!(t, 7 | 31 | 36 | 21)
                    && m.byte(event) == 1
            } else {
                true
            }
        };
        for i in 1..count {
            if select(m, elems + i * 40) {
                cnt += 1;
            }
        }
        if cnt != 0 {
            let heap = m.word(m.word(p + 8) + 4);
            src = frontend::call_method(m.bytes, opt(m, heap), 1, cnt * 40 + 1, host);
            if src == 0 {
                status = 0x8110200a;
            }
        }
        if status < 0x80000000 {
            cnt = 0;
            for i in 1..count {
                let event = elems + i * 40;
                let t = sync_type(m, event);
                if t == 0x4000 || m.byte(event) != 0 {
                    if select(m, event) {
                        m.put_word(event + 12, sync_pos(m, event).wrapping_add(off));
                        crate::normalization::memcpy_at(m.bytes, src + cnt * 40, event, 40);
                        cnt += 1;
                    }
                } else {
                    crate::normalization::memcpy_at(m.bytes, src + cnt * 40, event, 40);
                    cnt += 1;
                    if sync_type(m, reference) != 0
                        && sync_type(m, reference) == t
                        && sync_pos(m, reference) == sync_pos(m, event)
                        && sync_len(m, reference) == sync_len(m, event)
                    {
                        m.put_word(event + 12, sync_pos(m, event).wrapping_add(off));
                        m.put_word(event + 16, 0);
                    }
                }
            }
            m.put_half(sec3 + 8, count as u16);
            m.put_word(elems + 12, sync_pos(m, elems).wrapping_add(off));
            m.put_word(elems + 16, sync_len(m, elems).wrapping_sub(off));
            for i in 1..count {
                let event = elems + i * 40;
                if sync_type(m, event) == 0x4000
                    || m.byte(event) != 0
                    || sync_pos(m, event) <= sync_pos(m, elems)
                {
                    continue;
                }
                for j in i + 1..count {
                    let next = elems + j * 40;
                    if m.byte(next) == 0 {
                        m.put_word(
                            next + 12,
                            sync_pos(m, next).wrapping_sub(sync_len(m, event)),
                        );
                    }
                }
                m.put_word(event + 16, 0);
            }
        }
    }
    if status < 0x80000000 {
        let debug = (sync_level(m, m.word(m.word(p + 8) + 16), host) >= 5) as u32;
        let pipeline_args = [
            m.word(p + 52),
            m.word(p + 56),
            m.word(p + 60),
            debug,
            m.word(p),
            m.word(p + 4),
            res,
            p + 32,
            m.word(p + 24),
            m.word(p + 28),
            m.word(p + 64),
            half(m, p + 68) as u32,
            half(m, p + 70) as u32,
            half(m, p + 72) as u32,
        ];
        let rc = crate::render_part_d2::fn_2efd50(m, pipeline_args, pipeline, host);
        if rc >= 0x80000000 {
            return rc;
        }
        let object = m.word(p + 12);
        let stage=pipeline.stages.expect("live region normalization requires kernel stage data");
        let literals=Host::kernel_literals(host);
        let kernel=crate::leftovers_1::KernelData {constants:normal.constants,category:literals[0],language:stage.language,formats:core::array::from_fn(|i|literals[i+1]),scratch:stage.scratch,fold:stage.fold};
        let rc = crate::render_part_a3::normalize_regions(
            m.bytes,
            object,
            res,
            p + 44,
            normal,
            regions,
            &mut crate::leftovers_1::KernelRegions {host,data:&kernel,outer:pipeline.scratch},
        );
        if rc >= 0x80000000 {
            return rc;
        }
        status = fn_2ba6f0(m, p, res, off, reset, src, cnt, e, data, host);
    }
    if cnt != 0 {
        release(m, m.word(m.word(p + 8) + 4), src, host);
    }
    crate::normalization::release_iterator(
        m.bytes,
        edit,
        &mut NormalHost {
            environment: e,
            host,
        },
    );
    status
}

fn pcre_word(m: &[u8], p: u32) -> u32 {
    u32::from_le_bytes(m[p as usize..p as usize + 4].try_into().unwrap())
}
fn pcre_half(m: &[u8], p: u32) -> u16 {
    u16::from_le_bytes(m[p as usize..p as usize + 2].try_into().unwrap())
}
fn pcre_put_word(m: &mut [u8], p: u32, v: u32) {
    m[p as usize..p as usize + 4].copy_from_slice(&v.to_le_bytes());
}
fn pcre_put_half(m: &mut [u8], p: u32, v: u16) {
    m[p as usize..p as usize + 2].copy_from_slice(&v.to_le_bytes());
}
fn exec_sentence_pattern(
    m: &mut [u8],
    ctx: u32,
    pattern: u32,
    text: u32,
    len: u32,
    start: u32,
    options: u32,
    offsets: &mut [i32; 30],
    scratch: &mut crate::render_part_d::EscapeScratch<'_>,
    tables: &crate::regex::PcreTables<'_>,
    host: &mut impl crate::normalization::NormalizationHost,
) -> i32 {
    let object = pcre_word(m, ctx + 12);
    let key = pcre_word(m, ctx + 16);

    if crate::frontend::validate_handle(object != 0, key, 0xdd57, 24) >= 0x80000000 {
        return -16;
    }
    let entries = pcre_word(m, object + 4);
    let count = pcre_half(m, object + 8);
    let per = pcre_half(m, object + 10);
    let size = pcre_half(m, object + 12);
    let max = pcre_half(m, object + 14);
    assert!(scratch.blocks.len() >= max as usize);
    assert!(
        scratch.frames.len() >= max as usize * per as usize
            && scratch.tags.len() >= max as usize * per as usize
    );
    for b in scratch.blocks.iter_mut() {
        *b = crate::regex::FrameBlockState::default();
    }
    for i in 0..count as usize {
        let p = pcre_word(m, entries + i as u32 * 4);
        scratch.blocks[i] = crate::regex::FrameBlockState {
            present: p != 0,
            live: p != 0,
            used: if p != 0 { pcre_word(m, p + 8) } else { 0 },
            count: if p != 0 { m[(p + 16) as usize] } else { 0 },
            code_id: if p != 0 { pcre_word(m, p) } else { 0 },
            extra_id: if p != 0 { pcre_word(m, p + 12) } else { 0 },
            entry_id: p,
        };
    }
    let plen = if pattern == 0 {
        0
    } else {
        pcre_word(m, pattern + 4) as usize
    };
    assert!(plen <= scratch.program.len() && len as usize + 512 <= scratch.subject.len());
    scratch.program[..plen].copy_from_slice(&m[pattern as usize..pattern as usize + plen]);
    scratch.subject[..len as usize + 512].fill(0);
    scratch.subject[256..256 + len as usize]
        .copy_from_slice(&m[text as usize..text as usize + len as usize]);
    let code = pattern
        + if plen >= 40 {
            pcre_half(m, pattern + 24) as u32
                + pcre_half(m, pattern + 26) as u32 * pcre_half(m, pattern + 28) as u32
        } else {
            0
        };
    let delta = 256u32.wrapping_sub(text);
    let zero = [0u8; 284];
    for f in scratch.frames.iter_mut() {
        *f = crate::render_part_d::load_match_frame(
            &zero,
            0,
            code,
            delta,
            scratch.blocks,
            per as u32 * size as u32,
        );
    }
    for i in 0..count as usize {
        let b = scratch.blocks[i];
        if !b.present {
            continue;
        }
        for j in 0..per as usize {
            let slot = i * per as usize + j;
            scratch.tags[slot] = m[b.extra_id as usize + j];
            scratch.frames[slot] = crate::render_part_d::load_match_frame(
                m,
                b.code_id + j as u32 * size as u32,
                code,
                delta,
                scratch.blocks,
                per as u32 * size as u32,
            );
        }
    }
    let heap = pcre_word(m, pcre_word(m, object) + 4);
    let mut store = crate::regex::FrameStore {
        frames: scratch.frames,
        tags: scratch.tags,
        blocks: scratch.blocks,
        frames_per_block: per,
        frame_size: size,
        max_blocks: max,
        block_count: count,
        current_block: pcre_half(m, object + 16),
        highest_block: pcre_half(m, object + 18),
        max_frames: pcre_half(m, object + 20),
        fresh: crate::regex::MatchFrame::default(),
        heap,
        blocks_id: Some(entries),
    };
    let rc = crate::regex::pcre_exec_simple(
        &mut crate::render_part_d::MatchHost { memory: m, host },
        object,
        key,
        &mut store,
        &scratch.program[..plen],
        None,
        &scratch.subject[..len as usize + 512],
        256,
        len as usize,
        start as i32,
        0,
        Some(offsets),
        30,
        tables,
    );
    pcre_put_half(m, object + 8, store.block_count);
    pcre_put_half(m, object + 16, store.current_block);
    pcre_put_half(m, object + 18, store.highest_block);
    pcre_put_half(m, object + 20, store.max_frames);
    for (i, b) in store.blocks.iter().enumerate() {
        if i < count as usize || b.entry_id != 0 {
            let old = pcre_word(m, entries + i as u32 * 4);
            if i != 0 && i < count as usize && old != 0 {
                pcre_put_word(m, old, 0);
                pcre_put_word(m, old + 12, 0);
            }
            pcre_put_word(m, entries + i as u32 * 4, b.entry_id);
        }
        if !b.present {
            continue;
        }
        pcre_put_word(m, b.entry_id, b.code_id);
        pcre_put_word(m, b.entry_id + 4, per as u32 * size as u32 + 1);
        pcre_put_word(m, b.entry_id + 8, b.used);
        pcre_put_word(m, b.entry_id + 12, b.extra_id);
        m[(b.entry_id + 16) as usize] = b.count;
        if b.code_id == 0 || b.extra_id == 0 {
            continue;
        }
        for j in 0..per as usize {
            let slot = i * per as usize + j;
            m[b.extra_id as usize + j] = store.tags[slot];
            crate::render_part_d::save_match_frame(
                m,
                b.code_id + j as u32 * size as u32,
                &store.frames[slot],
                code,
                delta,
                store.blocks,
            );
        }
    }
    rc
}

pub struct SentenceData<'a, 'b> {
    pub parser: &'a ParserData<'b>,
    pub labels: [u32; 2],
    pub nfa_format: u32,
    pub nfa_words: [u32; 2],
    pub dictionary_name: u32,
}
pub struct RegexData<'a, 'b> {
    pub scratch: &'a mut crate::render_part_d::EscapeScratch<'b>,
    pub tables: &'a crate::regex::PcreTables<'b>,
}
fn non_final_abbreviation(
    m: &mut Memory<'_>,
    p: u32,
    text: u32,
    pos: u32,
    width: u32,
    data: &SentenceData<'_, '_>,
    host: &mut (impl Host + crate::log_live::LogMethodsM),
) -> bool {
    if p == 0 || m.word(p + 20) == 0 || pos < width || width == 0 {
        return false;
    }
    let end = frontend::divmod_u32(pos, width).0;
    if end == 0 || !matches!(half(m, text + (end - 1) * 2), 0x2e | 0xff0e) {
        return false;
    }
    let pd = data.parser;
    let s = pd.scratch;
    let index = s + 346;
    m.put_half(index, end.wrapping_sub(1) as u16);

    if m.byte(p + 80) == 1 {
        crate::normalization::trim_trailing(
            m.bytes,
            text,
            index,
            m.word(p + 32),
            half(m, p + 36) as u32,
            pd.separators,
        );
    } else {
        crate::normalization::trim_trailing_keep_cjk(
            m.bytes,
            text,
            index,
            m.word(p + 32),
            half(m, p + 36) as u32,
            pd.separators,
        );
        m.put_half(index, half(m, index).wrapping_add(1));
    }
    crate::normalization::skip_matched(m.bytes, text, index, end, pd.separators);
    let start = half(m, index) as u32;
    let length = end.wrapping_sub(start) as u16;
    if end.wrapping_sub(start) >= 16 || length < 2 {
        return false;
    }
    let word = s + 64;
    let hex = s + 192;
    let display = s + 256;
    let digits = s + 320;
    crate::normalization::memset_at(m.bytes, word, 0x30, 128);
    crate::normalization::memcpy_at(m.bytes, word, text + start * 2, length as u32 * width);
    m.put_half(word + length as u32 * 2, 0);
    for i in 0..length as u32 {
        let ch = half(m, word + i * 2) as u32;
        if ch == 0xff0e || ch.wrapping_sub(0xff21) <= 25 || ch.wrapping_sub(0xff41) <= 25 {
            let c = (ch & 255) + 32;
            if c == 0x2e || c.wrapping_sub(97) <= 25 || c.wrapping_sub(65) <= 25 {
                m.put_half(word + i * 2, c as u16);
            }
        }
    }
    crate::normalization::memset_at(m.bytes, hex, 0x30, 64);
    let mut hexpos = 0;
    for i in 0..length as u32 {
        crate::normalization::memset_at(m.bytes, digits, 0, 5);
        crate::normalization::itoa(m.bytes, half(m, word + i * 2) as u32, digits, 16);
        let n = crate::normalization::strlen8(m.bytes, digits);
        crate::normalization::memcpy_at(m.bytes, hex + hexpos + 4 - n, digits, n);
        hexpos += 4;
    }
    m.bytes[(hex + hexpos) as usize] = 0;
    let logger = m.word(m.word(p + 8) + 16);
    if sync_level(m, logger, host) >= 5 {
        crate::fe_text::narrow_n_17649c(m.bytes, display, word, length as u32);
        m.bytes[(display + length as u32) as usize] = 0;
        parser_log(
            m,
            logger,
            pd,
            pd.formats[4],
            &[display, hex, start, end],
            host,
        );
    }
    let result = s + 340;
    let found = s + 344;
    m.put_word(result, 0);
    m.put_half(found, 0xffff);
    let method = m.word(m.word(p + 20) + 28);
    let rc = host.call(
        m,
        method,
        &[
            m.word(p + 24),
            m.word(p + 28),
            data.dictionary_name,
            hex,
            result,
            found,
            s + 328,
        ],
    );
    if rc < 0x80000000 {
        if sync_level(m, logger, host) >= 5 {
            parser_log(
                m,
                logger,
                pd,
                data.nfa_format,
                &[
                    display,
                    data.nfa_words[if half(m, found) == 1 { 0 } else { 1 }],
                    start,
                    end,
                ],
                host,
            );
        }
        if half(m, found) == 1 {
            return true;
        }
    }
    false
}
fn sentence_blocked(
    m: &mut Memory<'_>,
    p: u32,
    text: u32,
    end: u32,
    offset: u16,
    length: u16,
    tokens: u32,
    data: &SentenceData<'_, '_>,
    host: &mut (impl Host + crate::log_live::LogMethodsM),
) -> bool {
    if text == 0 || end <= offset as u32 || offset & 1 != 0 || length & 1 != 0 {
        return true;
    }
    if non_final_abbreviation(m, p, text, offset as u32, 2, data, host) {
        return true;
    }
    let events = m.word(tokens + 4);
    let mut phon = false;
    let mut multi = false;
    for i in 0..half(m, tokens + 8) as u32 {
        let event = events + i * 40;
        if m.word(event + 20).wrapping_sub(m.word(events + 20)) > offset as u32 {
            break;
        }
        match half(m, event + 10) {
            7 => {
                if m.word(event + 28) == 43 {
                    phon = !phon;
                } else {
                    phon = false;
                }
            }
            32 => multi = m.word(event + 28) == 1,
            _ => {}
        }
    }
    phon || multi
}

pub fn fn_2be984(
    m: &mut Memory<'_>,
    p: u32,
    use_end: u32,
    all: u32,
    input: u32,
    base: u32,
    start: u32,
    mut end: u32,
    output: u32,
    state: i32,
    e: &Environment,
    data: &SentenceData<'_, '_>,
    regex: &mut RegexData<'_, '_>,
    host: &mut (impl Host + crate::log_live::LogMethodsM),
) -> u32 {
    assert_eq!(m.base, 0);
    let pd = data.parser;
    let s = pd.scratch;
    let mut saved = [0u8; 1200];
    saved.copy_from_slice(&m.bytes[s as usize..s as usize + 1200]);
    crate::normalization::memset_at(m.bytes, s, 0, 40);
    m.put_half(s + 10, 2);
    m.bytes[s as usize] = 1;
    m.put_word(s + 28, 1);
    let result = (|| {
        if p == 0 || input == 0 || output == 0 {
            return 0x81102007;
        }
        let (count, list) = if use_end == 0 {
            (half(m, p + 64), m.word(p + 60))
        } else {
            (half(m, p + 72), m.word(p + 68))
        };
        let text = m.word(input + 4);
        end = (half(m, input + 2) as u32 * end) as u16 as u32;
        let (mut rc, tokens) = iterator_table(m, output);
        if rc >= 0x80000000 {
            return rc;
        }
        for index in 0..count as u32 {
            rc = set_iterator_index(m, output, state);
            if rc >= 0x80000000 {
                break;
            }
            let pattern = half(m, list + index * 2) as u32;
            let mut pos = (half(m, input + 2) as u32 * start) as u16 as u32;
            let record = m.word(p + 52) + pattern * 8;
            let program = m.word(record).wrapping_add(m.word(p + 56));
            while pos < end {
                let mut captures = [0i32; 30];
                let status = exec_sentence_pattern(
                    m.bytes,
                    p,
                    program,
                    text,
                    end,
                    pos,
                    0,
                    &mut captures,
                    regex.scratch,
                    regex.tables,
                    &mut NormalHost {
                        environment: e,
                        host,
                    },
                );
                if status < 0 {
                    break;
                }
                parser_log(
                    m,
                    m.word(m.word(p + 8) + 16),
                    pd,
                    pd.formats[2],
                    &[
                        pattern,
                        pos,
                        m.byte(record + 4) as u32,
                        data.labels[(use_end != 0) as usize],
                        index,
                        count as u32,
                    ],
                    host,
                );
                let capture = m.byte(record + 4) as usize;
                let at = if capture < 254 { capture * 2 } else { 0 };
                let next = captures[at + 1] as u16;
                if capture < 254 {
                    m.put_word(s + 20, base.wrapping_add(captures[at] as u32));
                    m.put_word(
                        s + 24,
                        (captures[at + 1] as u32).wrapping_sub(captures[at] as u32),
                    );
                } else {
                    m.put_word(
                        s + 20,
                        base.wrapping_add(captures[if capture == 255 { 1 } else { 0 }] as u32),
                    );
                    m.put_word(s + 24, 0);
                }
                if sentence_blocked(
                    m,
                    p,
                    text,
                    end,
                    m.word(s + 20).wrapping_sub(base) as u16,
                    m.word(s + 24) as u16,
                    tokens,
                    data,
                    host,
                ) {
                    parser_log(m, m.word(m.word(p + 8) + 16), pd, pd.formats[3], &[], host);
                } else {
                    rc = crate::text_input::insert_span(
                        m.bytes,
                        output,
                        s,
                        0,
                        &mut NormalHost {
                            environment: e,
                            host,
                        },
                    );
                    if rc >= 0x80000000 {
                        return rc;
                    }
                }
                if all == 0 {
                    break;
                }
                pos = next as u32;
            }
        }
        rc
    })();
    m.bytes[s as usize..s as usize + 1200].copy_from_slice(&saved);
    result
}

pub(crate) fn pd_separators<'a>(data: &ParserData<'a>) -> &'a [u8] {
    data.separators
}
pub(crate) fn parser_boundary_callback(
    m: &mut Memory<'_>,
    timeline: u32,
    position: u32,
    out: u32,
    mode: i16,
    separators: &[u8],
) -> u32 {
    let mut next = position as u16;
    if mode != 2 && m.byte(timeline + 12) == 0 {
        let text = m.word(timeline + 4);
        let limit = half(m, timeline + 8);
        if text != 0 {
            while next < limit {
                let c = half(m, text + next as u32 * 2);
                if (mode == 3 && crate::normalization::u16chr(separators, 0, c).is_some())
                    || (mode == 4 && c == 10)
                {
                    break;
                }
                next = next.wrapping_add(1);
            }
        }
        m.bytes[out as usize] = (limit > next) as u8;
    } else {
        m.bytes[out as usize] = (half(m, timeline + 8) as u32 > position) as u8;
    }
    0
}

pub struct MarkerInsertData<'a> {
    pub dump: &'a DumpData,
    pub phases: [u32; 3],
    pub format: u32,
    pub category: u32,

    pub filter: u32,

    pub stack: u32,
}

pub fn fn_256fdc(m: &mut Memory<'_>, a: u32, b: u32, c: u32, capacity: u32) -> u32 {
    if c == 0 {
        return 0x81102007;
    }
    let mut state = b;
    m.bytes[c as usize] = 0;
    if matches!(b, 1 | 5 | 6 | 7 | 8) {
        state = if a == 0 || a == 43 { 16 } else { a };
    } else if b == 2 {
        state = 16;
    }
    if state & 255 == a {
        return 0;
    }
    let mut n = 0;
    if a != 0 {
        if a == 3 || a == 4 {
            if capacity <= 2 {
                return 0x81102009;
            }
            m.bytes[c as usize] = 17;
            m.bytes[c as usize + 1] = 31;
            n = 2;
        } else {
            if capacity <= 1 {
                return 0x81102009;
            }
            m.bytes[c as usize] = 31;
            n = 1;
        }
    }
    let bytes: &[u8] = match b {
        0 => &[],
        1 | 5 | 6 | 7 | 8 => {
            if a == 16 {
                &[]
            } else {
                &[30, 80]
            }
        }
        2 | 16 => &[30, 80],
        3 | 4 => &[30, 76, 16],
        43 => &[30, 47, 43],
        _ => &[],
    };
    let guard = if b == 16 { 3 } else { bytes.len() as u32 };
    if !bytes.is_empty() && n + guard >= capacity {
        return 0x81102009;
    }
    for (i, v) in bytes.iter().enumerate() {
        m.bytes[(c + n) as usize + i] = *v;
    }
    m.bytes[(c + n + bytes.len() as u32) as usize] = 0;
    0
}
fn marker_scan(m: &mut Memory<'_>, it: u32, filter: u32, advance: bool) -> u32 {
    if it == 0 {
        return 0x81102007;
    }
    let table = m.word(it + 12);
    let base = m.word(table + 4);
    let mut cursor = half(m, it + 16);
    let mut one = 1u16;
    let ids = [filter as u16, (filter >> 16) as u16, 0, 0];
    let rc = crate::normalization::scan_forward(
        &m.bytes[base as usize..],
        half(m, table + 2),
        half(m, table + 8),
        &mut cursor,
        &mut |event| {
            if advance {
                if one == 0 {
                    1
                } else {
                    one -= 1;
                    0
                }
            } else {
                crate::normalization::id_in_list(
                    &ids,
                    u16::from_le_bytes(event[10..12].try_into().unwrap()),
                )
            }
        },
    );
    m.put_half(it + 16, cursor);
    rc
}
fn current_iterator_index(m: &Memory<'_>, it: u32, out: &mut u16) -> u32 {
    if it == 0 {
        return crate::normalization::current_index(None, 0, Some(out));
    }
    crate::normalization::current_index(
        Some(&m.bytes[m.word(it + 12) as usize..]),
        half(m, it + 16),
        Some(out),
    )
}

pub fn fn_259aa8(
    m: &mut Memory<'_>,
    p: u32,
    request: u32,
    e: &Environment,
    data: &MarkerInsertData<'_>,
    host: &mut (impl Host + crate::log_live::LogMethodsM),
) -> u32 {
    assert_eq!(m.base, 0);
    if p == 0 || request == 0 {
        return 0x81102007;
    }
    let s = data.stack;
    let mut saved = [0u8; 160];
    saved.copy_from_slice(&m.bytes[s as usize..s as usize + 160]);
    let mut text = 0;
    let mut markers = 0;
    let mut bridge = NormalHost {
        environment: e,
        host,
    };
    let result = (|| {
        let rc = crate::normalization::open_iterator(
            m.bytes,
            m.word(p),
            m.word(p + 4),
            request,
            0x3fc,
            Some(&mut text),
            &mut bridge,
        );
        if rc >= 0x80000000 {
            return rc;
        }
        let rc = crate::normalization::open_iterator(
            m.bytes,
            m.word(p),
            m.word(p + 4),
            request,
            0x3fd,
            Some(&mut markers),
            &mut bridge,
        );
        if rc >= 0x80000000 {
            return rc;
        }
        fn_2b9770(
            m,
            m.byte(p + 32) as u32,
            m.word(p + 8),
            text,
            markers,
            data.phases[0],
            data.dump,
            bridge.host,
        );
        let mut events = 0;
        let mut count = 0;
        let rc = iterator_entry(m, markers, 0, &mut events);
        if rc >= 0x80000000 {
            return rc;
        }
        let rc = iterator_count(m, markers, &mut count);
        if rc >= 0x80000000 {
            return rc;
        }
        for i in 1..count as u32 {
            let event = events as u32 + i * 40;
            if half(m, event + 10) != 7 || m.word(event + 28) != 43 {
                continue;
            }
            for j in i + 1..count as u32 {
                let other = events as u32 + j * 40;
                if matches!(half(m, other + 10), 999 | 7)
                    && m.word(other + 20) >= m.word(event + 20)
                    && m.word(other + 20) < m.word(event + 20).wrapping_add(m.word(event + 24))
                {
                    if m.word(other + 20) == m.word(event + 20) {
                        m.put_word(event + 24, 0);
                        m.put_word(s, i);
                        crate::log_live::fn_1738e8(
                            m,
                            m.word(m.word(p + 8) + 16),
                            data.category,
                            5,
                            0,
                            data.format,
                            s,
                            bridge.host,
                        );
                    }
                    break;
                }
            }
        }
        fn_2b9770(
            m,
            m.byte(p + 32) as u32,
            m.word(p + 8),
            text,
            markers,
            data.phases[1],
            data.dump,
            bridge.host,
        );
        let rc = set_iterator_index(m, text, 0);
        if rc >= 0x80000000 {
            return rc;
        }
        let rc = set_iterator_index(m, markers, 0);
        if rc >= 0x80000000 {
            return rc;
        }
        let rc = iterator_entry(m, markers, 0, &mut events);
        if rc >= 0x80000000 {
            return rc;
        }
        let rc = iterator_count(m, markers, &mut count);
        if rc >= 0x80000000 {
            return rc;
        }
        let mut rc = marker_scan(m, markers, data.filter, false);
        if rc >= 0x80000000 && rc & 0x1fff != 0x14 {
            return rc;
        }
        let fallback = if rc < 0x80000000 { 16 } else { 0 };
        let mut old = 0;
        let mut previous = 0;
        let narrow = s + 32;
        let wide = s + 64;
        while rc < 0x80000000 {
            let mut current = 0;
            let mut first = 0;
            let mut index = 0;
            let status = iterator_entry(m, markers, 0xffff, &mut current);
            if status >= 0x80000000 {
                return status;
            }
            let status = current_iterator_index(m, markers, &mut index);
            if status >= 0x80000000 {
                return status;
            }
            let status = iterator_entry(m, markers, 0, &mut first);
            if status >= 0x80000000 {
                return status;
            }
            let current = current as u32;
            if m.word(current + 28) == 43
                && previous != 0
                && m.word(first as u32 + previous * 40 + 28) == 43
            {
                old = 15;
            }
            let next = m.word(current + 28) & 255;
            let status = fn_256fdc(m, old & 255, next, narrow, 16);
            if status >= 0x80000000 {
                return status;
            }
            if matches!(next, 1 | 5 | 6 | 7 | 8) {
                if old & 255 == 0 || old & 255 == 43 {
                    old = fallback;
                }
            } else if next == 2 {
                old = fallback;
            } else {
                old = next;
            }
            let n = crate::normalization::strlen8(m.bytes, narrow) as u16 as u32;
            if n != 0 {
                let offset = m
                    .word(current + 20)
                    .wrapping_sub(m.word(events as u32 + 20)) as u16;
                let status = set_iterator_index(m, text, (offset >> 1) as i32);
                if status >= 0x80000000 {
                    return status;
                }
                crate::fe_text::strcpy_widen_1763e4(m.bytes, wide, narrow);
                let status =
                    crate::normalization::insert_elements(m.bytes, text, wide, n, &mut bridge);
                if status >= 0x80000000 {
                    return status;
                }
                let mut one = 0;
                let status = current_iterator_index(m, markers, &mut one);
                if status >= 0x80000000 {
                    return status;
                }
                for j in one as u32..count as u32 {
                    let event = events as u32 + j * 40;
                    m.put_word(
                        event + 20,
                        m.word(event + 20).wrapping_add(n * 2) as u16 as u32,
                    );
                }
            }
            previous = index as u32;
            marker_scan(m, markers, 0, true);
            rc = marker_scan(m, markers, data.filter, false);
            if rc >= 0x80000000 && rc & 0x1fff != 0x14 {
                return rc;
            }
        }
        let rc = fn_256fdc(m, old & 255, 0, narrow, 16);
        if rc >= 0x80000000 {
            return rc;
        }
        let n = crate::normalization::strlen8(m.bytes, narrow) as u16 as u32;
        if n != 0 {
            let mut count = 0;
            let rc = iterator_count(m, text, &mut count);
            if rc >= 0x80000000 {
                return rc;
            }
            let rc = set_iterator_index(m, text, count as i32);
            if rc >= 0x80000000 {
                return rc;
            }
            crate::fe_text::strcpy_widen_1763e4(m.bytes, wide, narrow);
            let rc = crate::normalization::insert_elements(m.bytes, text, wide, n, &mut bridge);
            if rc >= 0x80000000 {
                return rc;
            }
        }
        fn_2b9770(
            m,
            m.byte(p + 32) as u32,
            m.word(p + 8),
            text,
            markers,
            data.phases[2],
            data.dump,
            bridge.host,
        );
        0
    })();
    if markers != 0 {
        crate::normalization::release_iterator(m.bytes, markers, &mut bridge);
    }
    if text != 0 {
        crate::normalization::release_iterator(m.bytes, text, &mut bridge);
    }
    m.bytes[s as usize..s as usize + 160].copy_from_slice(&saved);
    result
}
