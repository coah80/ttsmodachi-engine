//! Boot-path lane 4. Addresses are guest addresses in caller-owned memory.
use crate::boot::Memory;
fn put(m: &mut Memory<'_>, p: u32, v: u32) {
    let at = (p - m.base) as usize;
    m.bytes[at..at + 4].copy_from_slice(&v.to_le_bytes());
}
/// Return the region's static descriptor, supplied by the image loader.
pub fn fn_228760(m: &mut Memory<'_>, a: u32, out: u32, descriptor: u32) -> u32 {
    if a <= 1 {
        put(m, out, descriptor);
        0
    } else {
        0x84e02001
    }
}

/// Region-specific static descriptor getter.
pub fn fn_22c5cc(m: &mut Memory<'_>, a: u32, out: u32, descriptor: u32) -> u32 {
    if a <= 1 {
        put(m, out, descriptor);
        0
    } else {
        0x88602001
    }
}
pub fn fn_232300(m: &mut Memory<'_>, a: u32, out: u32, descriptor: u32) -> u32 {
    if a == 0 || a == 1 {
        put(m, out, descriptor);
        0
    } else {
        0x88102001
    }
}
pub fn fn_23231c(m: &mut Memory<'_>, a: u32, out: u32, descriptor: u32) -> u32 {
    if a == 0 || a == 1 {
        put(m, out, descriptor);
        0
    } else {
        0x88102001
    }
}
fn release(m: &mut Memory<'_>, heap: u32, p: u32, h: &mut impl crate::frontend::HeapVtableM) {
    assert_eq!(m.base, 0, "heap port uses absolute guest indexes");
    crate::frontend::heap_release_m(
        m.bytes,
        (heap != 0).then_some(heap),
        (p != 0).then_some(p),
        h,
    );
}
pub fn fn_24752c(m: &mut Memory<'_>, obj: u32, h: &mut impl crate::frontend::HeapVtableM) -> u32 {
    let heap = m.word(m.word(obj) + 4);
    release(m, heap, m.word(obj + 0x48), h);
    put(m, obj + 0x48, 0);
    0
}
pub fn fn_2475f8(m: &mut Memory<'_>, obj: u32, h: &mut impl crate::frontend::HeapVtableM) -> u32 {
    let heap = m.word(m.word(obj) + 4);
    release(m, heap, m.word(obj + 0x4c), h);
    put(m, obj + 0x4c, 0);
    0
}
/// Free regex/string arrays and their elements with the real reentrant heap port.
pub fn fn_248210(
    m: &mut Memory<'_>,
    obj: u32,
    field: u32,
    count: u32,
    h: &mut impl crate::frontend::HeapVtableM,
) {
    // The field must be mapped. Every graph edge is reloaded across callbacks.
    if m.word(field) == 0 {
        return;
    }
    let mut i = 0u32;
    while i < count {
        let row = i.wrapping_mul(16);
        let elem = m.word(m.word(field) + row + 8);
        if elem != 0 {
            release(m, m.word(m.word(obj) + 4), elem, h);
        }
        if m.word(m.word(field) + row + 4) != 0 {
            let mut j = 0u32;
            while j < m.word(m.word(field) + row + 12) {
                let sub = m.word(m.word(field) + row + 4);
                release(m, m.word(m.word(obj) + 4), m.word(sub + j * 4), h);
                let sub = m.word(m.word(field) + row + 4);
                put(m, sub + j * 4, 0);
                j += 1;
            }
            let sub = m.word(m.word(field) + row + 4);
            release(m, m.word(m.word(obj) + 4), sub, h);
        }
        i += 1;
    }
    release(m, m.word(m.word(obj) + 4), m.word(field), h);
    put(m, field, 0);
}

/// Read a three-byte, NUL-padded key through the existing strncpy port.
fn key3(m: &Memory<'_>, p: u32) -> [u8; 4] {
    let mut local = [0u8; 8];
    let at = (p - m.base) as usize;
    local[4..7].copy_from_slice(&m.bytes[at..at + 3]);
    crate::frontend::copy_str_n(&mut local, 0, 4, 3);
    local[..4].try_into().unwrap()
}
pub fn fn_248a28(m: &mut Memory<'_>, base: u32, limit: u32, cursor: u32) -> u32 {
    loop {
        let p = base.wrapping_add(m.word(cursor));
        let key = key3(m, p);
        if crate::frontend::compare_str(b"dum\0", &key) == 0
            && m.bytes[(p + 3 - m.base) as usize] == 0
            && m.bytes[(p + 4 - m.base) as usize] == 0
        {
            put(m, cursor, m.word(cursor).wrapping_add(5));
            return 0;
        }
        let mut zeros = 0;
        let mut cur;
        loop {
            cur = m.word(cursor);
            if cur >= limit {
                break;
            }
            let b = m.bytes[(base.wrapping_add(cur) - m.base) as usize];
            cur = cur.wrapping_add(1);
            put(m, cursor, cur);
            if b == 0 {
                zeros += 1;
            }
            if zeros >= 2 {
                break;
            }
        }
        if cur == limit {
            return 0x8bf02000;
        }
    }
}

/// Live dependencies and immutable regional addresses. Scratch is a disjoint
/// caller-owned 160-byte range. Nested calls need a different range.
#[derive(Clone, Copy)]
pub struct Environment {
    pub null_pair: [u8; 8],
    pub default_context: u32,
    pub module: u32,
    pub synthstream: u32,
    pub utf16: u32,
    pub begin: u32,
    pub end: u32,
    pub handlers: [u32; 5],
    pub runtime_callbacks: [u32; 3],
    pub scratch: u32,
    /// Disjoint 1024-byte staging area for the block-data teardown port.
    pub block_scratch: u32,
}
struct Calls<'a, H>(&'a mut H);
impl<H: crate::boot_part_b::Host> crate::boot::Calls for Calls<'_, H> {
    fn call(&mut self, m: &mut Memory<'_>, f: u32, a: &[u32]) -> u32 {
        if f == 0x1963b8 {
            return crate::boot_part_e::calloc(m, a[0], a[1], a[2], self);
        }
        self.0.call(m, f, a)
    }
    fn call_wide(&mut self, _: &mut Memory<'_>, _: u32, _: &[u32]) -> u64 {
        unreachable!()
    }
}
impl<H: crate::boot_part_b::Host> crate::component_live_a2::Calls for Calls<'_, H> {
    fn call(&mut self, m: &mut Memory<'_>, f: u32, a: &[u32]) -> u32 {
        crate::boot::Calls::call(self, m, f, a)
    }
}
impl<H: crate::boot_part_b::Host> crate::boot_part_e::Host for Calls<'_, H> {
    fn call(&mut self, m: &mut Memory<'_>, f: u32, a: &[u32]) -> u32 {
        self.0.call(m, f, a)
    }
}
fn calloc(
    m: &mut Memory<'_>,
    heap: u32,
    n: u32,
    size: u32,
    h: &mut impl crate::boot_part_b::Host,
) -> u32 {
    crate::boot_part_e::calloc(m, heap, n, size, &mut Calls(h))
}
fn free(m: &mut Memory<'_>, heap: u32, p: u32, h: &mut impl crate::boot_part_b::Host) {
    crate::boot_part_e::release(m, heap, p, &mut Calls(h));
}
fn error(
    m: &mut Memory<'_>,
    logger: u32,
    module: u32,
    code: u32,
    e: &Environment,
    h: &mut impl crate::boot_part_b::FormatHost,
) {
    crate::render_part_a::error_log(m, h, logger, module, code, 0, 0, e.scratch);
}
struct Log<'a, H>(&'a mut H);
impl<H: crate::boot_part_b::Host> crate::log_live::LogMethodsM for Log<'_, H> {
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
fn trace(
    m: &mut Memory<'_>,
    logger: u32,
    module: u32,
    fmt: u32,
    rc: u32,
    e: &Environment,
    h: &mut impl crate::boot_part_b::Host,
) {
    let saved = m.word(e.scratch);
    put(m, e.scratch, rc);
    crate::log_live::fn_1738e8(m, logger, module, 4, 0, fmt, e.scratch, &mut Log(h));
    put(m, e.scratch, saved);
}
fn info(
    m: &mut Memory<'_>,
    obj: u32,
    fmt: u32,
    rc: u32,
    e: &Environment,
    h: &mut impl crate::boot_part_b::Host,
) {
    trace(m, m.word(m.word(obj) + 16), m.word(obj + 72), fmt, rc, e, h);
}
fn context(m: &Memory<'_>, object: u32, tag: u32, e: &Environment) -> (u32, u32) {
    let mut ctx = 0;
    let rc = crate::frontend::resolve_engine_context(
        object,
        (object != 0).then(|| m.half(object) as u16),
        tag,
        &e.null_pair,
        e.default_context,
        &mut ctx,
    );
    (rc, ctx)
}
/// Count and group three-byte keys in a bounded two-string record stream.
pub fn fn_2484e8(
    m: &mut Memory<'_>,
    a: [u32; 8],
    e: &Environment,
    h: &mut impl crate::boot_part_b::FormatHost,
) -> u32 {
    assert_eq!(m.base, 0, "this adapter requires absolute arena indexes");
    let [obj, base, limit, mut cursor, tbl, n, cnt, out] = a;
    put(m, out, 0);
    crate::frontend::fill_bytes(m.bytes, tbl, 0, n.wrapping_mul(4));
    crate::frontend::fill_bytes(m.bytes, cnt, 0, n.wrapping_mul(4));
    let mut j = 0;
    if cursor <= limit {
        if cursor >= limit {
            return 0;
        }
        loop {
            let p = base.wrapping_add(cursor);
            let key = key3(m, p);
            if crate::frontend::compare_str(b"dum\0", &key) == 0
                && m.byte(p + 3) == 0
                && m.byte(p + 4) == 0
            {
                return 0;
            }
            let mut i = 0;
            let found = loop {
                let at = tbl + i * 4;
                if crate::frontend::compare_str(&key, &m.bytes[at as usize..]) == 0
                    || m.byte(at) == 0
                {
                    break true;
                }
                i += 1;
                if i >= n {
                    break false;
                }
            };
            let mut insert = true;
            if found {
                put(m, cnt + i * 4, m.word(cnt + i * 4).wrapping_add(1));
                j = i;
                if m.byte(tbl + i * 4) != 0 {
                    insert = false;
                }
            }
            if insert {
                if j >= n {
                    break;
                }
                let at = tbl + j * 4;
                let temp = e.scratch + 24;
                let saved = m.word(temp);
                put(m, temp, u32::from_le_bytes(key));
                crate::frontend::copy_str(m.bytes, at, temp);
                put(m, temp, saved);
                put(m, out, m.word(out).wrapping_add(1));
            }
            let mut zeros = 0;
            loop {
                if cursor >= limit {
                    break;
                }
                let b = m.byte(base.wrapping_add(cursor));
                cursor = cursor.wrapping_add(1);
                if b == 0 {
                    zeros += 1;
                }
                if zeros >= 2 {
                    break;
                }
            }
            if cursor >= limit {
                return 0;
            }
        }
    }
    error(m, m.word(m.word(obj) + 16), e.module, 0xc353, e, h);
    1
}
/// Open the block-data root. Allocation uses the real fn_1963b8 dispatch.
pub fn fn_250918(
    m: &mut Memory<'_>,
    obj: u32,
    out: u32,
    e: &Environment,
    h: &mut impl crate::boot_part_b::FormatHost,
) -> u32 {
    assert_eq!(m.base, 0, "this adapter requires absolute arena indexes");
    let ctx = m.word(obj);
    let logger = m.word(ctx + 16);
    let module = m.word(obj + 72);
    if out == 0 {
        error(m, logger, module, 0x5df1, e, h);
        return crate::normalization::forward_error(7);
    }
    trace(m, logger, module, e.begin, 0, e, h);
    put(m, out, 0);
    let p = calloc(m, m.word(ctx + 4), 1, 8, h);
    if p == 0 {
        error(m, logger, module, 0x5df0, e, h);
        return crate::normalization::forward_error(10);
    }
    crate::frontend::fill_bytes(m.bytes, p, 0, 8);
    m.put_half(p, 0);
    put(m, p + 4, 0);
    put(m, out, p);
    trace(m, logger, module, e.end, 0, e, h);
    0
}
/// Open the runtime-data manager, including its real callback-block constructor.
pub fn fn_24dac8(
    m: &mut Memory<'_>,
    obj: u32,
    e: &Environment,
    h: &mut impl crate::boot_part_b::FormatHost,
) -> u32 {
    assert_eq!(m.base, 0, "this adapter requires absolute arena indexes");
    info(m, obj, e.begin, 0, e, h);
    let p = calloc(m, m.word(m.word(obj) + 4), 1, 0x30, h);
    if p == 0 {
        error(m, m.word(m.word(obj) + 16), m.word(obj + 72), 0x5df0, e, h);
        return crate::normalization::error_code(10);
    }
    put(m, obj + 68, p);
    let [b, c, d] = e.runtime_callbacks;
    let mut rc = 0;
    if crate::component_live_a2::create_block_m(
        m,
        [m.word(m.word(obj) + 4), b, c, d, 5, p],
        &mut Calls(h),
    ) == 0
    {
        error(m, m.word(m.word(obj) + 16), m.word(obj + 72), 0x5df0, e, h);
        rc = crate::normalization::error_code(10);
    }
    if rc < 0x80000000 {
        put(m, p + 4, 0);
        m.put_half(p + 8, 0);
        for (off, v) in [(20, 0x68), (16, 0), (22, 0), (24, 0x32), (18, 5)] {
            m.put_half(p + off, v);
        }
        let arr = calloc(m, m.word(m.word(obj) + 4), 1, 0x264, h);
        put(m, p + 12, arr);
        if arr == 0 {
            error(m, m.word(m.word(obj) + 16), m.word(obj + 72), 0x5df0, e, h);
            rc = crate::normalization::error_code(10);
        } else {
            let entry = m.word(p + 12) + u32::from(m.half(p + 16)) * 12;
            put(
                m,
                entry + 4,
                u32::from(m.half(p + 20)) * u32::from(m.half(p + 18)) + 1,
            );
            put(m, entry + 8, 0);
            put(m, entry, 0);
            let data = calloc(m, m.word(m.word(obj) + 4), 1, m.word(entry + 4), h);
            put(m, entry, data);
            if data == 0 {
                error(m, m.word(m.word(obj) + 16), m.word(obj + 72), 0x5df0, e, h);
                rc = crate::normalization::error_code(10);
            } else {
                m.put_half(p + 16, (m.half(p + 16) as u16).wrapping_add(1));
            }
        }
    }
    if rc >= 0x80000000 {
        fn_24d8fc(m, obj, e, h);
    }
    info(m, obj, e.end, rc, e, h);
    rc
}
struct CompletedAllocation(u32);
impl crate::frontend::MethodTable for CompletedAllocation {
    fn call_at(&mut self, _: u32, _: u32, _: u32) -> u32 {
        self.0
    }
}
/// Construct the normalizer's context map using fn_1964e0's existing port.
pub fn fn_24d84c(
    m: &mut Memory<'_>,
    obj: u32,
    out: u32,
    e: &Environment,
    h: &mut impl crate::boot_part_b::FormatHost,
) -> u32 {
    assert_eq!(m.base, 0, "this adapter requires absolute arena indexes");
    let a = e.handlers;
    let heap = m.word(m.word(obj) + 4);
    let p = if out != 0 && a.iter().all(|v| *v != 0) {
        calloc(m, heap, 1, 68, h)
    } else {
        0
    };
    let rc = crate::frontend::create_ctx_object(
        m.bytes,
        (heap != 0).then_some(heap),
        0,
        a[0],
        a[1],
        a[2],
        a[3],
        a[4],
        (out != 0).then_some(out),
        &mut CompletedAllocation(p),
    );
    if rc < 0x80000000 {
        return 0;
    }
    error(m, m.word(m.word(obj) + 16), m.word(obj + 72), 0x5de7, e, h);
    rc
}
/// Open the lightweight transcoder handle, resolving the engine and resource
/// through their existing ports. Resource temporaries are restored on exit.
pub fn fn_230c58(
    m: &mut Memory<'_>,
    object: u32,
    tag: u32,
    out: u32,
    e: &Environment,
    h: &mut impl crate::boot_part_b::FormatHost,
) -> u32 {
    assert_eq!(m.base, 0, "this adapter requires absolute arena indexes");
    let mut pair = [0u8; 8];
    crate::frontend::load_pair(&mut pair, &e.null_pair);
    let pair0 = u32::from_le_bytes(pair[..4].try_into().unwrap());
    let pair1 = u32::from_le_bytes(pair[4..].try_into().unwrap());
    if crate::frontend::match_halves(object, tag, pair0, pair1) != 0 || out == 0 {
        return 0x88102007;
    }
    put(m, out, 0);
    put(m, out + 4, 0);
    let (rc, ctx) = context(m, object, tag, e);
    if rc >= 0x80000000 {
        return 0x88102007;
    }
    let p = calloc(m, m.word(ctx + 4), 1, 36, h);
    if p == 0 {
        error(m, m.word(ctx + 16), e.module, 0x3a98, e, h);
        return 0x8810200a;
    }
    put(m, p, object);
    put(m, p + 4, tag);
    let saved = m.word(e.scratch);
    put(m, e.scratch, 0);
    let rc = crate::render_part_c::fn_17b058_m(
        m,
        m.word(ctx + 24),
        e.synthstream,
        e.scratch,
        &mut Parser(h),
    );
    let lk = m.word(e.scratch);
    put(m, e.scratch, saved);
    if rc >= 0x80000000 {
        error(m, m.word(ctx + 16), e.module, 0x3ab0, e, h);
        free(m, m.word(ctx + 4), p, h);
        return rc;
    }
    put(m, p + 8, m.word(lk + 4));
    put(m, out, p);
    put(m, out + 4, 0x356);
    rc
}

struct Parser<'a, H>(&'a mut H);
impl<H: crate::boot_part_b::Host> crate::render_part_c::ParserMethodsM for Parser<'_, H> {
    fn method_m(&mut self, f: u32, a: &[u32], m: &mut Memory<'_>) -> u32 {
        self.0.call(m, f, a)
    }
}
struct Norm<'a, H> {
    h: &'a mut H,
    e: &'a Environment,
    resolved: u32,
}
impl<H: crate::boot_part_b::Host> crate::normalization::NormalizationHost for Norm<'_, H> {
    fn calloc(&mut self, b: &mut [u8], heap: u32, n: u32, s: u32) -> Option<u32> {
        let p = calloc(&mut Memory { base: 0, bytes: b }, heap, n, s, self.h);
        (p != 0).then_some(p)
    }
    fn alloc(&mut self, b: &mut [u8], heap: u32, s: u32) -> Option<u32> {
        let p = crate::boot_part_e::alloc(
            &mut Memory { base: 0, bytes: b },
            heap,
            s,
            &mut Calls(self.h),
        );
        (p != 0).then_some(p)
    }
    fn realloc(&mut self, b: &mut [u8], heap: u32, p: u32, s: u32) -> Option<u32> {
        let p = crate::boot_part_e::fn_1963e0(
            &mut Memory { base: 0, bytes: b },
            heap,
            p,
            s,
            &mut Calls(self.h),
        );
        (p != 0).then_some(p)
    }
    fn free(&mut self, b: &mut [u8], heap: u32, p: u32) {
        free(&mut Memory { base: 0, bytes: b }, heap, p, self.h);
    }
    fn resolve(&mut self, b: &mut [u8], p: u32, t: u32) -> (u32, u32) {
        let pair = context(&Memory { base: 0, bytes: b }, p, t, self.e);
        self.resolved = pair.1;
        pair
    }
}
/// Open the full transcoder and its one-byte flag through the existing port.
pub fn fn_230a14(
    m: &mut Memory<'_>,
    object: u32,
    tag: u32,
    out: u32,
    e: &Environment,
    h: &mut impl crate::boot_part_b::FormatHost,
) -> u32 {
    assert_eq!(m.base, 0, "this adapter requires absolute arena indexes");
    let mut pair = [0u8; 8];
    crate::frontend::load_pair(&mut pair, &e.null_pair);
    let pair0 = u32::from_le_bytes(pair[..4].try_into().unwrap());
    let pair1 = u32::from_le_bytes(pair[4..].try_into().unwrap());
    if crate::frontend::match_halves(object, tag, pair0, pair1) != 0 || out == 0 {
        return 0x88102007;
    }
    put(m, out, 0);
    put(m, out + 4, 0);
    let (rc, ctx) = context(m, object, tag, e);
    if rc >= 0x80000000 {
        return 0x88102007;
    }
    let p = calloc(m, m.word(ctx + 4), 1, 300, h);
    if p == 0 {
        error(m, m.word(ctx + 16), e.module, 0x3a98, e, h);
        return 0x8810200a;
    }
    put(m, p + 0x108, 0x88102007);
    put(m, p, object);
    put(m, p + 4, tag);
    crate::frontend::copy_str(m.bytes, p + 0x88, e.utf16);
    let saved = m.word(e.scratch);
    put(m, e.scratch, 0);
    let rc = crate::render_part_c::fn_17b058_m(
        m,
        m.word(ctx + 24),
        e.synthstream,
        e.scratch,
        &mut Parser(h),
    );
    let lk = m.word(e.scratch);
    put(m, e.scratch, saved);
    if rc >= 0x80000000 {
        error(m, m.word(ctx + 16), e.module, 0x3ab0, e, h);
    } else {
        put(m, p + 0x110, m.word(lk + 4));
    }
    let (rc, flag_ctx) = {
        let mut norm = Norm { h, e, resolved: 0 };
        let rc = crate::normalization::alloc_xcoder_flag(m.bytes, p, &mut norm);
        (rc, norm.resolved)
    };
    if rc == 0x8810200a {
        error(m, m.word(flag_ctx + 16), e.module, 0x3a98, e, h);
    }
    if rc < 0x80000000 {
        put(m, out, p);
        put(m, out + 4, 0x354);
        return rc;
    }
    free(m, m.word(ctx + 4), p, h);
    if lk != 0 {
        crate::render_part_c::registry_release_m(
            m,
            m.word(ctx + 24),
            e.synthstream,
            e.scratch,
            &mut Parser(h),
        );
    }
    rc
}

// Lane 5's fn_2b645c has landed. This adapter retains live memory on all calls.
impl<H: crate::boot_part_b::Host> crate::boot_port_5::Host for Calls<'_, H> {
    fn call(&mut self, m: &mut Memory<'_>, f: u32, a: &[u32]) -> u32 {
        self.0.call(m, f, a)
    }
}
fn close_data(
    m: &mut Memory<'_>,
    obj: u32,
    e: &Environment,
    k: &crate::render_part_a::ReaderRuntime,
    h: &mut impl crate::render_part_a::ReaderHost,
) -> u32 {
    let p = m.word(obj + 56);
    if p == 0 {
        return crate::normalization::error_code(8);
    }
    info(m, obj, e.begin, 0, e, h);
    let mut rc = 0;
    // 250a3c is lane 4b's port; the two other closers are local.
    let block = block_environment(e, k);
    for f in [0x2b6594, 0x250a3c, 0x2b5e0c] {
        let r = match f {
            0x2b6594 => fn_2b6594(m,obj,e,h),
            0x2b5e0c => fn_2b5e0c(m,obj,e,h),
            _ => crate::boot_port_4b::fn_250a3c(m, h, obj, &block),
        };
        if r >= 0x80000000 {
            rc = r;
        }
    }
    free(m, m.word(m.word(obj) + 4), p, h);
    put(m, obj + 56, 0);
    info(m, obj, e.end, rc, e, h);
    rc
}
/// Open all three FE data roots, cleaning up partial construction on failure.
pub fn fn_24ef54(
    m: &mut Memory<'_>,
    obj: u32,
    child_formats: [[u32; 2]; 4],
    e: &Environment,
    k: &crate::render_part_a::ReaderRuntime,
    h: &mut impl crate::render_part_a::ReaderHost,
) -> u32 {
    assert_eq!(m.base, 0, "this adapter requires absolute arena indexes");
    info(m, obj, e.begin, 0, e, h);
    let p = calloc(m, m.word(m.word(obj) + 4), 1, 12, h);
    if p == 0 {
        error(m, m.word(m.word(obj) + 16), m.word(obj + 72), 0x5df0, e, h);
        return crate::normalization::error_code(10);
    }
    put(m, obj + 56, p);
    let mut rc =
        crate::boot_port_5::fn_2b645c(m, &mut Calls(h), obj, p, child_formats[0], e.scratch);
    if rc < 0x80000000 {
        info(m, obj, child_formats[1][0], 0, e, h);
        rc = crate::normalization::open_domain_handle(
            m.bytes,
            obj,
            p + 4,
            &mut Norm { h, e, resolved: 0 },
        );
        if rc >= 0x80000000 {
            error(m, m.word(m.word(obj) + 16), m.word(obj + 72), 0x5df0, e, h);
        } else {
            info(m, obj, child_formats[1][1], rc, e, h);
        }
    }
    let block_env = Environment {
        begin: child_formats[2][0],
        end: child_formats[2][1],
        ..*e
    };
    if rc < 0x80000000 {
        rc = fn_250918(m, obj, p + 8, &block_env, h);
    }
    if rc >= 0x80000000 {
        let close_env = Environment {
            begin: child_formats[3][0],
            end: child_formats[3][1],
            ..*e
        };
        close_data(m, obj, &close_env, k, h);
    }
    info(m, obj, e.end, rc, e, h);
    rc
}

struct Destroy<'a, H>(&'a mut H);
impl<H: crate::boot_part_b::Host> crate::boot::Calls for Destroy<'_, H> {
    fn call(&mut self, m: &mut Memory<'_>, f:u32, a:&[u32])->u32 {
        if f==0x196428 { free(m,a[0],a[1],self.0); 0 } else {self.0.call(m,f,a)}
    }
    fn call_wide(&mut self,_:&mut Memory<'_>,_:u32,_:&[u32])->u64 {unreachable!()}
}
/// Close runtime data, including array destructors and every entry buffer.
pub fn fn_24d8fc(m:&mut Memory<'_>,obj:u32,e:&Environment,h:&mut impl crate::boot_part_b::FormatHost)->u32 {
    if m.word(obj+68)==0 {return crate::normalization::error_code(8);}
    info(m,obj,e.begin,0,e,h);
    if m.word(m.word(obj+68))!=0 {
        crate::reentrant2::destroy_array_m(m,m.word(m.word(obj+68)),&mut Destroy(h));
        put(m,m.word(obj+68),0);
    }
    if m.word(m.word(obj+68)+12)!=0 {
        if m.half(m.word(obj+68)+16)>1 {
            let mut i=1u16;
            while i<m.half(m.word(obj+68)+16) {
                let entry=m.word(m.word(obj+68)+12)+u32::from(i)*12;
                if m.word(entry)!=0 {
                    free(m,m.word(m.word(obj)+4),m.word(entry),h);
                    let entry=m.word(m.word(obj+68)+12)+u32::from(i)*12;
                    put(m,entry,0);
                }
                i=i.wrapping_add(1);
            }
            let p=m.word(obj+68);
            if m.half(p+16)>1 {
                crate::frontend::fill_bytes(m.bytes,m.word(p+12)+12,0,u32::from(m.half(p+16)-1)*12);
                m.put_half(m.word(obj+68)+16,1);
            }
        }
        m.put_half(m.word(obj+68)+22,0);
    }
    if m.word(m.word(obj+68)+12)!=0 {
        let entry=m.word(m.word(obj+68)+12);
        if m.word(entry)!=0 {
            free(m,m.word(m.word(obj)+4),m.word(entry),h);
            put(m,m.word(m.word(obj+68)+12),0);
        }
        free(m,m.word(m.word(obj)+4),m.word(m.word(obj+68)+12),h);
        put(m,m.word(obj+68)+12,0);
    }
    m.put_half(m.word(obj+68)+8,0);
    if m.word(obj+68)!=0 {
        free(m,m.word(m.word(obj)+4),m.word(obj+68),h);
        put(m,obj+68,0);
    }
    info(m,obj,e.end,0,e,h);
    0
}
/// Release a function-data node. Flag one recursively releases child nodes.
pub fn fn_2b6234(m:&mut Memory<'_>,ctx:u32,n:u32,flag:u32,h:&mut impl crate::boot_part_b::Host) {
    if n==0{return;}
    for off in [20,16,24] {
        if m.word(n+off)!=0 {
            free(m,m.word(ctx+4),m.word(n+off),h);
            put(m,n+off,0);
        }
    }
    if m.word(n+8)!=0 {
        let mut i=0u16;
        while u32::from(i)<u32::from(m.byte(n+12)) {
            if flag==1 {
                fn_2b6234(m,ctx,m.word(m.word(n+8)+u32::from(i)*4),flag,h);
                free(m,m.word(ctx+4),m.word(m.word(n+8)+u32::from(i)*4),h);
            }
            put(m,m.word(n+8)+u32::from(i)*4,0);
            i=i.wrapping_add(1);
        }
        free(m,m.word(ctx+4),m.word(n+8),h);
        let at=m.offset(n+12);m.bytes[at]=0;
        put(m,n+8,0);
    }
}
pub fn fn_2b6310(m:&mut Memory<'_>,obj:u32,p:u32,h:&mut impl crate::boot_part_b::Host) {
    let ctx=m.word(obj);
    let count=m.half(p);
    if count>0 {
        let arr=m.word(p+4);
        for i in 0..count {
            let child=m.word(arr+u32::from(i)*4);
            if child!=0 {fn_2b6234(m,ctx,child,0,h);free(m,m.word(ctx+4),child,h);}
        }
    }
    if m.word(p+4)!=0 {free(m,m.word(ctx+4),m.word(p+4),h);put(m,p+4,0);}
    m.put_half(p,0);
}
pub fn fn_2b6594(m:&mut Memory<'_>,obj:u32,e:&Environment,h:&mut impl crate::boot_part_b::FormatHost)->u32 {
    let ctx=m.word(obj);let module=m.word(obj+72);
    let p=if m.word(obj+56)!=0 {m.word(m.word(obj+56))}else{0};
    if p==0{return crate::normalization::forward_error(8);}
    trace(m,m.word(ctx+16),module,e.begin,0,e,h);
    fn_2b6310(m,obj,p,h);
    info(m,obj,e.end,0,e,h);
    free(m,m.word(ctx+4),p,h);
    0
}
pub fn fn_2b5e0c(m:&mut Memory<'_>,obj:u32,e:&Environment,h:&mut impl crate::boot_part_b::FormatHost)->u32 {
    let ctx=m.word(obj);let module=m.word(obj+72);
    let p=m.word(m.word(obj+56)+4);
    if p==0 {
        error(m,m.word(ctx+16),module,0x5df1,e,h);
        return crate::normalization::forward_error(7);
    }
    trace(m,m.word(ctx+16),module,e.begin,0,e,h);
    info(m,obj,e.end,0,e,h);
    free(m,m.word(m.word(obj)+4),p,h);
    0
}

struct Compiler<'a,'b,H> {
    m:&'a mut Memory<'b>, h:&'a mut H, allocation:u32,
}
impl<H:crate::boot_part_b::Host> crate::regex::PcreHost for Compiler<'_, '_,H> {
    fn validate(&mut self,p:u32,key:u32,tag:u32,size:u32)->u32 {
        crate::frontend::validate_handle(p!=0,key,tag,size)
    }
    fn resolve_owner(&mut self,_:u32,_:i32)->(u32,u32){unreachable!()}
    fn heap_of(&mut self,_:u32)->u32 {unreachable!()}
    fn alloc(&mut self,heap:u32,n:u32,size:u32)->Option<u32> {
        let _=(heap,n,size);
        (self.allocation!=0).then_some(self.allocation)
    }
    fn free(&mut self,heap:u32,p:u32){free(self.m,heap,p,self.h);}
}
/// Bind the existing PCRE compiler to live guest allocation. These callers discard
/// the compiler's local error string and offset. Output capacity is caller-owned.
fn compile_pattern(m:&mut Memory<'_>,obj:u32,key:u32,pattern:&[u8],tables:&crate::regex::PcreTables<'_>,
    output:&mut [u8],h:&mut impl crate::boot_part_b::Host)->u32 {
    let heap=if obj!=0 {m.word(m.word(obj)+4)}else{0};
    struct Probe { valid:bool,size:u32 }
    impl crate::regex::PcreHost for Probe {
        fn validate(&mut self,_:u32,key:u32,tag:u32,size:u32)->u32 {crate::frontend::validate_handle(self.valid,key,tag,size)}
        fn resolve_owner(&mut self,_:u32,_:i32)->(u32,u32){unreachable!()}
        fn heap_of(&mut self,_:u32)->u32{unreachable!()}
        fn alloc(&mut self,_:u32,n:u32,size:u32)->Option<u32>{self.size=n.wrapping_mul(size);None}
        fn free(&mut self,_:u32,_:u32){unreachable!()}
    }
    let mut off=0;
    let mut probe=Probe{valid:obj!=0,size:0};
    let _=crate::regex::compile(&mut probe,obj,key,heap,pattern,0,tables,output,Some(&mut off));
    if probe.size==0{return 0;}
    let p=crate::boot_part_e::alloc(m,heap,probe.size,&mut Calls(h));
    if p==0{return 0;}
    let at=m.offset(p);
    assert!(probe.size as usize<=output.len());
    output[..probe.size as usize].copy_from_slice(&m.bytes[at..at+probe.size as usize]);
    let mut c=Compiler{m,h,allocation:p};
    match crate::regex::compile(&mut c,obj,key,heap,pattern,0,tables,output,Some(&mut off)) {
        Ok(Some(result))=>{
            let p=c.allocation;
            let at=c.m.offset(p);c.m.bytes[at..at+result.size].copy_from_slice(&output[..result.size]);
            p
        },
        Ok(None)|Err(crate::regex::CompileFail::Invalid(_))=>0,
        Err(other)=>panic!("PCRE caller scratch exhausted: {other:?}"),
    }
}
pub fn fn_247334(m:&mut Memory<'_>,obj:u32,tables:&crate::regex::PcreTables<'_>,
    output:&mut [u8],e:&Environment,h:&mut impl crate::boot_part_b::FormatHost)->u32 {
    if m.word(obj+72)!=0 {
        error(m,m.word(m.word(obj)+16),e.module,0xc353,e,h);return 0x8bf02000;
    }
    // ABI-local literals and sprintf output do not alias guest objects.
    let mut local=[0u8;1024];
    let mut mem=Memory{base:0,bytes:&mut local};
    let strings:[&[u8];7]=[
        b"(%s%s*(%%%s|%%\"%s+\"){0,}(&%s%s*(%%%s|%%\"%s+\"){0,}){0,})|(%s%s)\0",
        b"[!#\\$&'\\(\\)\\*\\+,\\-\\./0123456789:;<=>\\?@A-Z\\[\\\\\\]\\^_`a-z{\\|}\\~]\0",
        b"[0\\-\\+6\\!~\\\\F`K12345<>}{:]\0",
        b"[\"#\\$&'\\.789=\\?@A-Z^_a-z]\0",
        b"(\\\\)?[!\\?,\\.;:]\0", b"\\*\0", b"\0"];
    let mut ptrs=[0u32;7];let mut at=512;
    for (i,s) in strings.iter().enumerate(){ptrs[i]=at as u32;mem.bytes[at..at+s.len()].copy_from_slice(s);at+=s.len();}
    mem.boot_sprintf(0,ptrs[0],&[ptrs[3],ptrs[2],ptrs[1],ptrs[1],ptrs[3],ptrs[2],ptrs[1],ptrs[1],ptrs[5],ptrs[4]]);
    let p=compile_pattern(m,m.word(obj+64),m.word(obj+68),&mem.bytes[..512],tables,output,h);
    put(m,obj+72,p);
    if p!=0 {return 0;}
    error(m,m.word(m.word(obj)+16),e.module,0xc353,e,h);0x8bf02000
}
pub fn fn_247550(m:&mut Memory<'_>,obj:u32,tables:&crate::regex::PcreTables<'_>,
    output:&mut [u8],e:&Environment,h:&mut impl crate::boot_part_b::FormatHost)->u32 {
    if m.word(obj+76)!=0 {
        error(m,m.word(m.word(obj)+16),e.module,0xc353,e,h);return 0x8bf02000;
    }
    let p=compile_pattern(m,m.word(obj+64),m.word(obj+68),b"(^|[^\\\\])([\\$][0-9]+)\0\0\0\0\0",tables,output,h);
    put(m,obj+76,p);
    if p!=0 {return 0;}
    error(m,m.word(m.word(obj)+16),e.module,0xc353,e,h);0x8bf02000
}

struct MapRuntime<'a,H>{h:&'a mut H,e:&'a Environment,keys:[(u32,u8);2]}
impl<H:crate::boot_part_b::Host> crate::boot_part_b::Host for MapRuntime<'_,H> {
    fn thread_id(&mut self)->u32 {self.h.thread_id()}
    fn call(&mut self,m:&mut Memory<'_>,f:u32,a:&[u32])->u32 {
        if f==self.e.handlers[0] {return crate::normalization::publish_object(m.bytes,a[0],a[1],a[2]);}
        if f==self.e.handlers[1] || f==self.e.handlers[4] {return 0;}
        if f==self.e.handlers[2] {return crate::normalization::compare_byte(self.keys.iter().find(|v|v.0==a[0]).map_or_else(||m.byte(a[0]),|v|v.1),self.keys.iter().find(|v|v.0==a[1]).map_or_else(||m.byte(a[1]),|v|v.1)) as u32;}
        if f==self.e.handlers[3] {return crate::normalization::publish_object_second(m.bytes,a[0],a[1],a[2]);}
        match f {
            0x196428=>{free(m,a[0],a[1],self.h);0},
            0x196394=>crate::boot_part_e::alloc(m,a[0],a[1],&mut Calls(self.h)),
            _=>self.h.call(m,f,a)
        }
    }
}
/// Open and populate the two-entry normalizer map. pairs is D_914a88,
/// supplied by the regional image loader; scratch is disjoint caller memory.
pub fn fn_24d3bc(m:&mut Memory<'_>,obj:u32,pairs:[[u32;2];2],key_bytes:[u8;2],e:&Environment,
    h:&mut impl crate::boot_part_b::FormatHost)->u32 {
    let saved=m.word(e.scratch);
    info(m,obj,e.begin,0,e,h);
    let mut rc=fn_24d84c(m,obj,e.scratch,e,h);
    if rc<0x80000000 {
        let p=m.word(e.scratch);
        let mut failed=false;
        for pair in pairs {
            rc=crate::boot_part_b2::guest_19668c_live(m,p,pair[0],pair[1],e.scratch+4,&mut MapRuntime{h,e,keys:[(pairs[0][0],key_bytes[0]),(pairs[1][0],key_bytes[1])]});
            if rc>=0x80000000 {
                error(m,m.word(m.word(obj)+16),m.word(obj+72),0x5dca,e,h);
                crate::boot_part_b2::guest_19665c_live(m,p,&mut MapRuntime{h,e,keys:[(pairs[0][0],key_bytes[0]),(pairs[1][0],key_bytes[1])]});
                failed=true;break;
            }
        }
        if !failed {
            let mut value=[m.word(m.word(obj+60)+4) as i32];
            rc=crate::normalization::set_item(&mut value,0,p as i32) as u32;
            put(m,m.word(obj+60)+4,value[0] as u32);
        }
    }
    put(m,e.scratch,saved);
    info(m,obj,e.end,rc,e,h);
    rc
}

struct GroupRelease<'a,H>{h:&'a mut H,obj:u32}
impl<H:crate::boot_part_b::Host> crate::frontend::HeapVtableM for GroupRelease<'_,H> {
    fn alloc_at_m(&mut self,_:u32,_:u32,_:&mut [u8])->u32{unreachable!()}
    fn realloc_at_m(&mut self,_:u32,_:u32,_:u32,_:&mut [u8])->u32{unreachable!()}
    fn release_at_m(&mut self,obj:u32,p:u32,bytes:&mut [u8]) {
        let mut m=Memory{base:0,bytes};
        let heap=m.word(m.word(self.obj)+4);
        let f=m.word(m.word(heap)+12);
        self.h.call(&mut m,f,&[obj,p]);
    }
}
/// Group and compile a record stream. workspace is 320 caller-owned scratch
/// bytes. The compiler output is separate host-side caller-owned storage.
pub fn fn_248688(m:&mut Memory<'_>,a:[u32;7],workspace:u32,tables:&crate::regex::PcreTables<'_>,
    output:&mut [u8],e:&Environment,h:&mut impl crate::boot_part_b::FormatHost)->u32 {
    let [obj,out,count,mode,base,limit,cursor]=a;
    let saved:[u8;320]=core::array::from_fn(|i|m.byte(workspace+i as u32));
    let rc=groups_inner(m,a,workspace,tables,output,e,h);
    let at=m.offset(workspace);m.bytes[at..at+320].copy_from_slice(&saved);
    let _=(obj,out,count,mode,base,limit,cursor);
    rc
}
fn groups_inner(m:&mut Memory<'_>,a:[u32;7],w:u32,tables:&crate::regex::PcreTables<'_>,
    output:&mut [u8],e:&Environment,h:&mut impl crate::boot_part_b::FormatHost)->u32 {
    let [obj,out,count,mode,base,limit,cursor]=a;
    put(m,count,0);
    if fn_2484e8(m,[obj,base,limit,cursor,w,40,w+160,count],e,h)!=0{return 0x8bf02000;}
    if m.word(count)==0{return 0;}
    put(m,out,0);
    let p=crate::boot_part_e::alloc(m,m.word(m.word(obj)+4),m.word(count).wrapping_mul(16),&mut Calls(h));
    put(m,out,p);
    if p==0 {error(m,m.word(m.word(obj)+16),e.module,0xc350,e,h);return 0x8bf0200a;}
    crate::frontend::fill_bytes(m.bytes,m.word(out),0,m.word(count).wrapping_mul(16));
    let mut i=0;
    while i<m.word(count) {
        let mut position=cursor;
        put(m,m.word(out)+i*16+4,0);
        put(m,m.word(out)+i*16+8,0);
        crate::frontend::copy_str(m.bytes,m.word(out)+i*16,w+i*4);
        if mode==0 {
            let p=crate::boot_part_e::alloc(m,m.word(m.word(obj)+4),m.word(w+160+i*4).wrapping_mul(4),&mut Calls(h));
            put(m,m.word(out)+i*16+4,p);
            if p==0 {return group_fail(m,obj,out,i,0x8bf0200a,e,h);}
        }else{put(m,m.word(out)+i*16+4,0);}
        put(m,m.word(out)+i*16+12,m.word(w+160+i*4));
        let p=crate::boot_part_e::alloc(m,m.word(m.word(obj)+4),m.word(w+160+i*4).wrapping_mul(4),&mut Calls(h));
        put(m,m.word(out)+i*16+8,p);
        if p==0{return group_fail(m,obj,out,i,0x8bf0200a,e,h);}
        let mut k=0;
        while k<m.word(w+160+i*4) {
            let mut done=false;
            let mut key=key3(m,base.wrapping_add(position));
            while crate::frontend::compare_str(&m.bytes[(m.word(out)+i*16) as usize..],&key)!=0 && limit>position && !done {
                let mut zeros=0;
                loop {
                    if position>=limit{break;}
                    let b=m.byte(base.wrapping_add(position));position=position.wrapping_add(1);
                    if b==0{zeros+=1;}
                    if zeros>=2{break;}
                }
                key=key3(m,base.wrapping_add(position));
                done=position==limit;
            }
            if position>=limit{return group_fail(m,obj,out,i,0x8bf02000,e,h);}
            put(m,m.word(m.word(out)+i*16+8)+k*4,base.wrapping_add(position).wrapping_add(3));
            if mode==0 {
                let src=base.wrapping_add(position).wrapping_add(3);
                // Pattern is copied before borrowing the arena for callbacks.
                let mut pattern=[0u8;512];let mut n=0;
                while n<508 {let b=m.byte(src+n as u32);pattern[n]=b;n+=1;if b==0{break;}}
                assert!(n<508,"pattern scratch too small");
                let re=compile_pattern(m,m.word(obj+64),m.word(obj+68),&pattern,tables,output,h);
                put(m,m.word(m.word(out)+i*16+4)+k*4,re);
            }
            position=position.wrapping_add(1);k+=1;
        }
        i+=1;
    }
    0
}
fn group_fail(m:&mut Memory<'_>,obj:u32,out:u32,i:u32,rc:u32,e:&Environment,h:&mut impl crate::boot_part_b::FormatHost)->u32 {
    error(m,m.word(m.word(obj)+16),e.module,if rc==0x8bf0200a {0xc350}else{0xc353},e,h);
    fn_248210(m,obj,out,i,&mut GroupRelease{h,obj});
    rc
}

struct HeapCalls<'a, H>(&'a mut H);
impl<H: crate::boot_part_b::Host> crate::boot_part_e::Host for HeapCalls<'_, H> {
    fn call(&mut self, m: &mut Memory<'_>, method: u32, args: &[u32]) -> u32 {
        self.0.call(m, method, args)
    }
}
fn arena<'a>(m: &'a mut Memory<'_>) -> crate::selection::arena::Arena<'a> {
    crate::selection::arena::Arena::new(m.base, m.bytes)
}
fn same_string(m: &mut Memory<'_>, left: u32, right: &[u8]) -> bool {
    let mut a = arena(m);
    for (i, b) in right.iter().enumerate() {
        if a.u8(left + i as u32) != *b {
            return false;
        }
    }
    a.u8(left + right.len() as u32) == 0
}
fn block_environment(
    e: &Environment,
    r: &crate::render_part_a::ReaderRuntime,
) -> crate::boot_port_4b::Environment {
    crate::boot_port_4b::Environment {
        reader: crate::render_part_a::ReaderRuntime {
            category: r.category,
            messages: r.messages,
            pair_format: r.pair_format,
            triple_format: r.triple_format,
            four_format: r.four_format,
            file_key: r.file_key,
            chunk_key: r.chunk_key,
            parent_key: r.parent_key,
            offset_key: r.offset_key,
            scratch: r.scratch,
        },
        tags: [e.module; 4],
        node_tags: [e.module; 12],
        sub_tag: e.module,
        detach_messages: [e.module; 2],
        close_messages: [e.module; 2],
        scratch: e.block_scratch,
    }
}
/// fn_2413c4. Open the string-map reader: read the STMC, STMF and STMR chunks
/// of the attached reader and publish the reader object through `out`.
pub fn fn_2413c4(
    m: &mut Memory<'_>,
    h: &mut impl crate::render_part_a::ReaderHost,
    object: u32,
    tag: u32,
    name: u32,
    reader: u32,
    out: u32,
    e: &Environment,
    k: &crate::render_part_a::ReaderRuntime,
) -> u32 {
    if name == 0 || reader == 0 || out == 0 {
        return 0x8a202007;
    }
    put(m, out, 0);
    // The context pointer lives outside the compared arena, as the ROM keeps
    // it on the stack.
    let ctx_slot = k.scratch + 0x140;
    let rc = crate::boot_port_1::fn_19611c(
        m,
        [object, tag],
        ctx_slot,
        e.null_pair,
        e.default_context,
    );
    if rc >= 0x80000000 {
        return rc;
    }
    let ctx = m.word(ctx_slot);
    let logger = m.word(ctx + 16);
    let heap = m.word(ctx + 4);
    let _ = crate::log_live::fn_1738e8(m, logger, e.module, 4, 0, 0, 0, &mut Log(h));
    let obj = crate::boot_part_e::calloc(m, heap, 1, 0x2c, &mut HeapCalls(h));
    if obj == 0 {
        crate::render_part_a::error_log(m, h, logger, e.module, 0x6d60, 0, 0, e.scratch);
        return 0x8a20200a;
    }
    put(m, obj, object);
    put(m, obj + 4, tag);
    put(m, obj + 8, ctx);
    let length = crate::fe_text::strlen_195d90(m.bytes, name).wrapping_add(1) as u32;
    let named = crate::boot_part_e::alloc(m, heap, length, &mut HeapCalls(h));
    put(m, obj + 0xc, named);
    if named == 0 {
        crate::render_part_a::error_log(m, h, logger, e.module, 0x6d60, 0, 0, e.scratch);
        return fn_2413c4_tail(m, h, obj, reader, e, k, false, 0x8a20200a);
    }
    {
        let mut a = arena(m);
        crate::selection::arena_leaf::copy_string(&mut a, named, name);
    }
    let buf = k.scratch + 0x100;
    let len_slot = k.scratch + 0x108;
    let cur_slot = k.scratch + 0x10c;
    let mut rc = crate::render_part_a::open_reader_chunk(
        m, h, reader, buf, len_slot, cur_slot, k,
    );
    if rc >= 0x80000000 {
        return fn_2413c4_tail(m, h, obj, reader, e, k, true, rc);
    }
    if !same_string(m, buf, b"STMC") {
        crate::render_part_a::error_log(m, h, logger, e.module, 0x6d61, 0, 0, e.scratch);
        return fn_2413c4_tail(m, h, obj, reader, e, k, true, 0x8a202004);
    }
    if m.word(len_slot) < 10 {
        crate::render_part_a::error_log(m, h, logger, e.module, 0x6d62, 0, 0, e.scratch);
        return fn_2413c4_tail(m, h, obj, reader, e, k, true, 0x8a202004);
    }
    {
        let cur = m.word(cur_slot);
        for i in 0..5u32 {
            let value = u16::from_le_bytes(
                m.bytes[(cur + i * 2) as usize..(cur + i * 2 + 2) as usize]
                    .try_into()
                    .unwrap(),
            );
            let at = match i {
                0 => obj + 0x10,
                1 => obj + 0x12,
                2 => obj + 0x14,
                3 => obj + 0x16,
                _ => obj + 0x20,
            };
            m.put_half(at, value);
        }
    }
    let _ = crate::render_part_a::close_reader_chunk(m, h, reader, k);
    rc = crate::render_part_a::open_reader_chunk(m, h, reader, buf, len_slot, cur_slot, k);
    if rc >= 0x80000000 {
        return fn_2413c4_tail(m, h, obj, reader, e, k, true, rc);
    }
    if !same_string(m, buf, b"STMF") {
        crate::render_part_a::error_log(m, h, logger, e.module, 0x6d61, 0, 0, e.scratch);
        return fn_2413c4_tail(m, h, obj, reader, e, k, true, 0x8a202004);
    }
    let span = u32::from(m.half(obj + 0x12))
        .wrapping_add(u32::from(m.half(obj + 0x14)))
        .wrapping_mul(u32::from(m.half(obj + 0x16)));
    if span > m.word(len_slot) {
        crate::render_part_a::error_log(m, h, logger, e.module, 0x6d62, 0, 0, e.scratch);
        return fn_2413c4_tail(m, h, obj, reader, e, k, true, 0x8a202004);
    }
    let block = block_environment(e, k);
    let detached = crate::boot_port_4b::fn_243ad4(m, h, reader, obj + 0x1c, cur_slot, &block);
    if detached < 0x80000000 {
        put(m, obj + 0x18, m.word(cur_slot));
    }
    let _ = crate::render_part_a::close_reader_chunk(m, h, reader, k);
    if m.half(obj + 0x20) == 0 {
        put(m, out, obj);
        let _ = crate::log_live::fn_1738e8(m, logger, e.module, 4, 0, 0, 0, &mut Log(h));
        return rc;
    }
    rc = crate::render_part_a::open_reader_chunk(m, h, reader, buf, len_slot, cur_slot, k);
    if rc >= 0x80000000 {
        return fn_2413c4_tail(m, h, obj, reader, e, k, true, rc);
    }
    if !same_string(m, buf, b"STMR") {
        crate::render_part_a::error_log(m, h, logger, e.module, 0x6d61, 0, 0, e.scratch);
        return fn_2413c4_tail(m, h, obj, reader, e, k, true, 0x8a202004);
    }
    let span = u32::from(m.half(obj + 0x12))
        .wrapping_add(u32::from(m.half(obj + 0x14)))
        .wrapping_mul(u32::from(m.half(obj + 0x20)));
    if span > m.word(len_slot) {
        crate::render_part_a::error_log(m, h, logger, e.module, 0x6d62, 0, 0, e.scratch);
        return fn_2413c4_tail(m, h, obj, reader, e, k, true, 0x8a202004);
    }
    let detached = crate::boot_port_4b::fn_243ad4(m, h, reader, obj + 0x28, cur_slot, &block);
    if detached < 0x80000000 {
        put(m, obj + 0x24, m.word(cur_slot));
    }
    let _ = crate::render_part_a::close_reader_chunk(m, h, reader, k);
    put(m, out, obj);
    let _ = crate::log_live::fn_1738e8(m, logger, e.module, 4, 0, 0, 0, &mut Log(h));
    rc
}
fn fn_2413c4_tail(
    m: &mut Memory<'_>,
    h: &mut impl crate::render_part_a::ReaderHost,
    obj: u32,
    reader: u32,
    e: &Environment,
    k: &crate::render_part_a::ReaderRuntime,
    close: bool,
    rc: u32,
) -> u32 {
    if close {
        let _ = crate::render_part_a::close_reader_chunk(m, h, reader, k);
    }
    let ctx = m.word(obj + 8);
    let logger = m.word(ctx + 16);
    let heap = m.word(ctx + 4);
    let _ = crate::log_live::fn_1738e8(m, logger, e.module, 4, 0, 0, 0, &mut Log(h));
    let mut rc2 = 0;
    if m.word(obj + 0x1c) != 0 {
        let r = crate::render_part_d::release_chunk(m.bytes, m.word(obj + 0x1c), h);
        if r >= 0x80000000 {
            rc2 = r;
        }
    }
    if m.word(obj + 0x28) != 0 {
        let r = crate::render_part_d::release_chunk(m.bytes, m.word(obj + 0x28), h);
        if r >= 0x80000000 && rc2 < 0x80000000 {
            rc2 = r;
        }
    }
    if m.word(obj + 0xc) != 0 {
        crate::boot_part_e::release(m, heap, m.word(obj + 0xc), &mut HeapCalls(h));
    }
    if rc2 < 0x80000000 {
        crate::boot_part_e::release(m, heap, obj, &mut HeapCalls(h));
    }
    let _ = crate::log_live::fn_1738e8(m, logger, e.module, 4, 0, 0, 0, &mut Log(h));
    let _ = crate::log_live::fn_1738e8(m, m.word(ctx + 16), e.module, 4, 0, 0, 0, &mut Log(h));
    rc
}

/// The select_bet2 engine. The parameter keys are the guest addresses of the
/// module's own name table; the module data is ROM data, so the caller
/// supplies the keys, the defaults and the log formats.
pub struct SelectEnvironment {
    /// D_8fa504.openNames: the rate key, the wait-factor key and the end.
    pub open_names: [u32; 3],
    /// D_8fa504.closeNames: four keys and the end.
    pub close_names: [u32; 4],
    /// D_8fa504.defaults: the fallback values of the two open keys.
    pub defaults: [u32; 2],
    /// The rate key, the same pointer as open_names[0].
    pub rate_name: u32,
    /// The wait-factor key, the same pointer as open_names[1].
    pub wait_name: u32,
    /// D_225118, the key the engine registers itself under.
    pub owner_key: u32,
    /// D_8fa510, the module name of the log category.
    pub module: u32,
    /// The "SYNTHSTREAM" resource name.
    pub synthstream: u32,
    /// D_8fa504.openCbs and closeCbs: the check and set callbacks.
    pub open_cbs: [u32; 2],
    pub close_cbs: [u32; 2],
    /// The log formats: the open trace, the rate and wait-factor traces, the
    /// open leave trace, the close trace and the close leave trace.
    pub formats: [u32; 6],
    /// A disjoint scratch area; its 1024 bytes are restored on return.
    pub scratch: u32,
    /// D_8d7640, the default context of the engine.
    pub default_context: u32,
}
fn select_saved(m: &mut Memory<'_>, at: u32) -> [u8; 1024] {
    m.bytes[m.offset(at)..m.offset(at) + 1024]
        .try_into()
        .unwrap()
}
fn select_restore(m: &mut Memory<'_>, at: u32, s: [u8; 1024]) {
    let o = m.offset(at);
    m.bytes[o..o + 1024].copy_from_slice(&s);
}
fn select_trace(
    m: &mut Memory<'_>,
    h: &mut impl crate::boot_part_b::Host,
    logger: u32,
    module: u32,
    fmt: u32,
    args: &[u32],
    e: &SelectEnvironment,
) {
    let at = e.scratch;
    for (i, v) in args.iter().enumerate() {
        m.put_word(at + 4 * i as u32, *v);
    }
    let _ = crate::log_live::fn_1738e8(m, logger, module, 4, 0, fmt, at, &mut Log(h));
}
fn select_error(
    m: &mut Memory<'_>,
    h: &mut impl crate::boot_part_b::FormatHost,
    logger: u32,
    module: u32,
    code: u32,
    e: &SelectEnvironment,
) {
    let _ = crate::render_part_a::error_log(m, h, logger, module, code, 0, 0, e.scratch);
}
/// fn_2271f4. Open the select_bet2 engine: resolve the context, allocate the
/// engine, load the voice (the 2252b0 stub of lane 3b) and register the
/// parameter owners.
pub fn fn_2271f4(
    m: &mut Memory<'_>,
    h_ctx: u32,
    ctx_arg: u32,
    out: u32,
    e: &SelectEnvironment,
    h: &mut impl crate::boot_part_b::FormatHost,
) -> u32 {
    assert_eq!(m.base, 0, "this adapter requires absolute arena indexes");
    if out == 0 {
        return 0x84e02007;
    }
    let saved = select_saved(m, e.scratch);
    let slot = e.scratch;
    put(m, slot, 0);
    let rc = crate::boot_port_1::fn_19611c(
        m,
        [h_ctx, ctx_arg],
        slot,
        [0u8; 8],
        e.default_context,
    );
    if rc >= 0x80000000 {
        select_restore(m, e.scratch, saved);
        return rc;
    }
    let ctx = m.word(slot);
    put(m, out, 0);
    put(m, out + 4, 0);
    let logger = m.word(ctx + 16);
    select_trace(m, h, logger, e.module, e.formats[0], &[], e);
    let mut rc = 0;
    let engine = calloc(m, m.word(ctx + 4), 1, 0x7f8, h);
    if engine == 0 {
        select_error(m, h, logger, e.module, 25000, e);
        rc = 0x84e0200a;
    } else {
        put(m, engine, h_ctx);
        put(m, engine + 4, ctx_arg);
        put(m, engine + 8, ctx);
        put(m, slot, 0);
        let r = h.call(m, 0x17b058, &[m.word(ctx + 24), e.synthstream, slot]);
        if r < 0x80000000 {
            let res = m.word(slot);
            put(m, engine + 12, m.word(res + 4));
        } else {
            select_error(m, h, logger, e.module, 25001, e);
        }
        rc = h.call(m, 0x2252b0, &[engine]);
        if rc < 0x80000000 {
            rc = h.call(
                m,
                0x174e74,
                &[
                    m.word(ctx + 20),
                    e.owner_key,
                    engine,
                    e.open_cbs[0],
                    e.open_cbs[1],
                ],
            );
            if rc >= 0x80000000 {
                select_error(m, h, logger, e.module, 25002, e);
            }
            let mut i = 0usize;
            while rc < 0x80000000 && e.open_names[i] != 0 {
                let name = e.open_names[i];
                put(m, slot, 0);
                let r = h.call(m, 0x175850, &[m.word(ctx + 20), name, slot]);
                let mut val = m.word(slot);
                if r >= 0x80000000 {
                    val = e.defaults[i];
                    rc = h.call(m, 0x1757b8, &[m.word(ctx + 20), name, val]);
                }
                if rc < 0x80000000 {
                    rc = select_int_param(m, h, engine, name, val, e);
                    if rc < 0x80000000 {
                        rc = h.call(
                            m,
                            0x174e74,
                            &[
                                m.word(ctx + 20),
                                name,
                                engine,
                                e.open_cbs[0],
                                e.open_cbs[1],
                            ],
                        );
                    }
                }
                if rc >= 0x80000000 {
                    select_error(m, h, logger, e.module, 25002, e);
                }
                i += 1;
            }
        }
        if rc < 0x80000000 {
            if h.call(m, 0x173fd8, &[logger]) >= 2 {
                let _ = h.call(m, 0x19854c, &[h_ctx, ctx_arg, 6, engine + 0x7c0]);
            }
            put(m, out, engine);
            put(m, out + 4, 0x8e84);
            m.put_byte(engine + 0x7f4, 0);
        } else {
            put(m, out, engine);
            put(m, out + 4, 0x8e84);
            fn_226f68(m, engine, 0x8e84, e, h);
            put(m, out, 0);
            put(m, out + 4, 0);
        }
    }
    select_trace(m, h, logger, e.module, e.formats[3], &[rc], e);
    select_restore(m, e.scratch, saved);
    rc
}
fn select_int_param(
    m: &mut Memory<'_>,
    h: &mut impl crate::boot_part_b::FormatHost,
    engine: u32,
    name: u32,
    v: u32,
    e: &SelectEnvironment,
) -> u32 {
    let ctx = m.word(engine + 8);
    let logger = m.word(ctx + 16);
    if name == e.rate_name {
        let rc = h.call(m, 0x2a9608, &[v, engine + 0x740]);
        if rc < 0x80000000 {
            select_trace(
                m,
                h,
                logger,
                e.module,
                e.formats[1],
                &[v, m.word(engine + 0x740)],
                e,
            );
        }
        rc
    } else if name == e.wait_name {
        if v & 0x8000_0000 != 0 {
            0x84e0200f
        } else {
            put(m, engine + 0x744, v);
            select_trace(m, h, logger, e.module, e.formats[2], &[v], e);
            0
        }
    } else {
        0
    }
}
/// fn_226f68. Close the select_bet2 engine: drop the parameter owners, close
/// the synth stream and release the voice blocks.
pub fn fn_226f68(
    m: &mut Memory<'_>,
    engine: u32,
    tag: u32,
    e: &SelectEnvironment,
    h: &mut impl crate::boot_part_b::FormatHost,
) -> u32 {
    let saved = select_saved(m, e.scratch);
    let mut rc = crate::frontend::validate_handle(engine != 0, tag, 0x8e84, 0x7f8);
    if rc >= 0x80000000 {
        select_restore(m, e.scratch, saved);
        return 0x84e02008;
    }
    let ctx = m.word(engine + 8);
    let logger = m.word(ctx + 16);
    select_trace(m, h, logger, e.module, e.formats[4], &[], e);
    if m.word(engine + 0x7c0) != 0 {
        let r = h.call(
            m,
            0x1985c4,
            &[m.word(engine + 0x7c0), m.word(engine + 0x7c4)],
        );
        if r >= 0x80000000 {
            rc = r;
        }
    }
    let mut i = 0usize;
    while e.close_names[i] != 0 {
        let r = h.call(
            m,
            0x174f9c,
            &[
                m.word(ctx + 20),
                e.close_names[i],
                engine,
                e.close_cbs[0],
                e.close_cbs[1],
            ],
        );
        if rc < 0x80000000 && r >= 0x80000000 {
            rc = r;
        }
        i += 1;
    }
    if m.word(engine + 12) != 0 {
        let r = h.call(m, 0x17b1c4, &[m.word(ctx + 24), e.synthstream]);
        if rc < 0x80000000 && r >= 0x80000000 {
            rc = r;
        }
    }
    let blocks = [0usize, 1, 3, 4]
        .iter()
        .any(|i| m.word(engine + 0x638 + 0x34 * *i as u32 + 0x1c) != 0);
    if blocks {
        if m.word(engine + 0x790) != 0 {
            let _ = h.call(m, 0x2a9090, &[engine + 0x78c]);
            let _ = h.call(m, 0x179f3c, &[m.word(ctx + 12), m.word(engine + 0x790)]);
            put(m, engine + 0x790, 0);
        }
        for i in 0..5u32 {
            if m.word(engine + 0x638 + 0x34 * i + 0x1c) != 0 {
                let _ = h.call(m, 0x2a320c, &[ctx, engine + 0x638 + 0x34 * i]);
            }
        }
        let _ = h.call(m, 0x2a41b8, &[engine + 0x1ac]);
        let _ = h.call(m, 0x2a0090, &[m.word(engine + 16)]);
        put(m, engine + 16, 0);
        let count = m.half(engine + 0x788) as u32;
        for i in 0..count {
            let _ = h.call(m, 0x243d50, &[m.word(engine + 0x748 + 4 * i)]);
        }
        m.put_half(engine + 0x788, 0);
    }
    crate::boot_part_e::release(m, m.word(ctx + 4), engine, &mut HeapCalls(h));
    select_trace(m, h, logger, e.module, e.formats[5], &[rc], e);
    select_restore(m, e.scratch, saved);
    rc
}
// ----------------------------------------------------------------------
// Round 5: fn_2512dc, the token data loader.
//
// The round-4 binding plan is followed exactly:
//   fn_243a94 forwards to fn_242fd0 (the arena-aware boot_port_3b port),
//   fn_242c9c is render_part_a::find_reader_chunk, fn_2422ac is
//   render_part_a::close_reader_chunk, fn_2421b0 is
//   render_part_a::close_reader, fn_24f570 and fn_24f124 are boot_port_4b's
//   ports, fn_175a1c is speech_part_2::fetch_parameter_m, fn_3023b0 is a
//   memset and the string helpers are fe_text's strcat_195c90 and
//   memcpy_195ae8. Only the named-parameter lookup stays a host service,
//   because its tree comparator is a guest method pointer.
// ----------------------------------------------------------------------

/// The named-parameter lookup of the token loader. The caller binds it to
/// `speech_part_2::fetch_parameter_m`, whose tree comparator is the
/// guest's own method pointer.
pub trait TokenHost:
    crate::boot_port_4b::Host + crate::boot_port_3b::OpenHostM + crate::boot_part_b::Host
{
    /// fn_175a1c. A named parameter string: the status and the guest
    /// address of the value.
    fn parameter(&mut self, m: &mut Memory<'_>, params: u32, key: u32) -> (u32, u32);
}

/// The literal texts the token loader reads. Every address is a guest
/// address of a NUL-terminated string.
pub struct TokenTexts {
    pub lang_index: u32,
    pub max: u32,
    pub langcode: u32,
    pub fecfg: u32,
    pub prefix: u32,
    pub packaging: u32,
    pub clc: u32,
    pub clc_slash: u32,
    pub tokentn_slash: u32,
    pub slash: u32,
    pub colon: u32,
    pub missing: u32,
    pub first_assigned: u32,
    pub clashes: u32,
    pub loaded: u32,
    pub empty: u32,
    pub cfig: u32,
    pub file: u32,
}

/// The four log formats of the token loader.
pub struct TokenFormats {
    pub index: u32,
    pub missing: u32,
    pub clash: u32,
    pub loaded: u32,
}

/// The caller-owned data of the token data loader.
pub struct TokenEnvironment {
    /// The reader runtime: the category, the formats and the scratch.
    pub reader: crate::render_part_a::ReaderRuntime,
    /// The block loader runtime: the tags, the messages and the scratch.
    pub block: crate::boot_port_4b::Environment,
    /// The open runtime of fn_242fd0: the broker, the context and the
    /// scratch.
    pub open: crate::leftovers_1::OpenRuntime,
    /// D_914ab8, the data version the file is compared against.
    pub version: u32,
    /// D_914abb, the resource type of the token file.
    pub kind: u32,
    /// 256 disjoint bytes: the 128-byte path buffer, the log argument area,
    /// the logger's parameter scratch and three words. Restored on return.
    pub scratch: u32,
    /// The literal texts.
    pub texts: TokenTexts,
    /// The log formats.
    pub formats: TokenFormats,
}
/// The row's stack frame. The ROM keeps fn_2512dc's locals, the reader's
/// own locals and the reader open's locals on the stack; the port keeps
/// them in the arena's scratch, so the whole frame is restored before the
/// row returns. The frame runs from the row's own scratch to the
/// character-class table that follows the open scratch: 0xb00 bytes.
const TOKEN_SCRATCH: usize = 0xb00;
const TOKEN_BUF: u32 = 0;
const TOKEN_ARGS: u32 = 0x80;
const TOKEN_PARAMS: u32 = 0xc0;
const TOKEN_XTRA: u32 = 0xd4;
const TOKEN_DATA: u32 = 0xd8;
const TOKEN_CHUNK: u32 = 0xdc;
fn token_saved(m: &mut Memory<'_>, at: u32) -> [u8; TOKEN_SCRATCH] {
    let o = m.offset(at);
    m.bytes[o..o + TOKEN_SCRATCH].try_into().unwrap()
}
fn token_restore(m: &mut Memory<'_>, at: u32, s: [u8; TOKEN_SCRATCH]) {
    let o = m.offset(at);
    m.bytes[o..o + TOKEN_SCRATCH].copy_from_slice(&s);
}
fn token_error(
    m: &mut Memory<'_>,
    h: &mut impl TokenHost,
    logger: u32,
    category: u32,
    code: u32,
    format: u32,
    args: &[u32],
    e: &TokenEnvironment,
) {
    let area = e.scratch + TOKEN_ARGS;
    for (i, v) in args.iter().enumerate() {
        m.put_word(area + 4 * i as u32, *v);
    }
    let _ = crate::render_part_a::error_log(
        m,
        h,
        logger,
        category,
        code,
        format,
        area,
        e.scratch + TOKEN_PARAMS,
    );
}
fn token_trace(
    m: &mut Memory<'_>,
    h: &mut impl TokenHost,
    logger: u32,
    category: u32,
    format: u32,
    args: &[u32],
    e: &TokenEnvironment,
) {
    let area = e.scratch + TOKEN_ARGS;
    for (i, v) in args.iter().enumerate() {
        m.put_word(area + 4 * i as u32, *v);
    }
    let _ = crate::log_live::fn_1738e8(
        m,
        logger,
        category,
        4,
        0,
        format,
        area,
        &mut Log(h),
    );
}
/// fn_243a94. Open the token file through the arena-aware fn_242fd0.
fn token_open(
    m: &mut Memory<'_>,
    h: &mut impl TokenHost,
    o: u32,
    mode: u32,
    path: u32,
    e: &TokenEnvironment,
) -> u32 {
    let args = [
        m.word(o + 4),
        m.word(o + 8),
        mode,
        path,
        0,
        0,
        e.kind,
        e.version,
        e.scratch + TOKEN_CHUNK,
    ];
    crate::boot_port_3b::fn_242fd0_m(m, args, &e.open, h)
}
/// fn_2512dc. Load the token data blocks of one language.
pub fn fn_2512dc(
    m: &mut Memory<'_>,
    o: u32,
    _count: u32,
    tokens: u32,
    e: &TokenEnvironment,
    h: &mut impl TokenHost,
) -> u32 {
    assert_eq!(m.base, 0, "this adapter requires absolute arena indexes");
    let saved = token_saved(m, e.scratch);
    let ctx = m.word(o);
    let heap = m.word(ctx + 4);
    let logger = m.word(ctx + 0x10);
    let params = m.word(ctx + 0x14);
    let category = m.word(o + 0x48);
    let buf = e.scratch + TOKEN_BUF;
    let t = &e.texts;
    let start = m.offset(buf);
    m.bytes[start..start + 128].fill(0);
    let lst = m.word(m.word(o + 0x38) + 8);
    if lst == 0 {
        token_error(m, h, logger, category, 0x5df1, 0, &[], e);
        let rc = crate::normalization::forward_error(7);
        token_restore(m, e.scratch, saved);
        return rc;
    }
    if m.half(lst) != 0 {
        token_error(m, h, logger, category, 0x5df7, 0, &[], e);
        let rc = crate::normalization::forward_error(7);
        token_restore(m, e.scratch, saved);
        return rc;
    }
    let idx = m.byte(o + 0x74) as u32;
    let ocount = m.half(o + 0x6c) as u32;
    if ocount < idx {
        token_error(
            m,
            h,
            logger,
            category,
            0x5df8,
            e.formats.index,
            &[t.lang_index, idx, t.max, ocount],
            e,
        );
        let rc = crate::normalization::forward_error(0);
        token_restore(m, e.scratch, saved);
        return rc;
    }
    let mut langcode = t.empty;
    let (r, v) = h.parameter(m, params, t.langcode);
    if r < 0x80000000 {
        langcode = v;
    }
    let mut rc = 0u32;
    let mut chunk = 0u32;
    let mut opened = false;
    let (r, fecfg) = h.parameter(m, params, t.fecfg);
    if r < 0x80000000 {
        let (r, prefix) = h.parameter(m, params, t.prefix);
        if r < 0x80000000 && prefix != 0 && m.byte(prefix) != 0 {
            crate::frontend::copy_str(m.bytes, buf, prefix);
        }
        let (r, packaging) = h.parameter(m, params, t.packaging);
        if r < 0x80000000
            && packaging != 0
            && crate::normalization::strcmp_at(m.bytes, packaging, t.clc) == 0
        {
            crate::frontend::append_str(m.bytes, buf, t.clc_slash);
            crate::frontend::append_str(m.bytes, buf, langcode);
            crate::frontend::append_str(m.bytes, buf, t.slash);
            crate::frontend::append_str(m.bytes, buf, fecfg);
            crate::frontend::append_str(m.bytes, buf, t.colon);
        }
        crate::frontend::append_str(m.bytes, buf, t.tokentn_slash);
        crate::frontend::append_str(m.bytes, buf, m.word(tokens + 4 * idx));
        crate::frontend::append_str(m.bytes, buf, t.slash);
        crate::frontend::append_str(m.bytes, buf, fecfg);
        rc = token_open(m, h, o, 2, buf, e);
        if rc < 0x80000000 {
            opened = true;
        }
    }
    if !opened {
        let (r, prefix) = h.parameter(m, params, t.prefix);
        if r < 0x80000000 && prefix != 0 && m.byte(prefix) != 0 {
            crate::frontend::copy_str(m.bytes, buf, prefix);
        } else {
            crate::frontend::copy_str(m.bytes, buf, t.empty);
        }
        let (r, packaging) = h.parameter(m, params, t.packaging);
        if r < 0x80000000
            && packaging != 0
            && crate::normalization::strcmp_at(m.bytes, packaging, t.clc) == 0
        {
            crate::frontend::append_str(m.bytes, buf, t.clc_slash);
            crate::frontend::append_str(m.bytes, buf, langcode);
            crate::frontend::append_str(m.bytes, buf, t.slash);
            crate::frontend::append_str(m.bytes, buf, fecfg);
            crate::frontend::append_str(m.bytes, buf, t.colon);
        }
        crate::frontend::append_str(m.bytes, buf, t.tokentn_slash);
        crate::frontend::append_str(m.bytes, buf, m.word(tokens + 4 * idx));
        rc = token_open(m, h, o, 0, buf, e);
        if rc >= 0x80000000 {
            token_error(
                m,
                h,
                logger,
                category,
                0x5df9,
                e.formats.missing,
                &[t.missing, buf],
                e,
            );
            let rc = crate::normalization::forward_error(3);
            token_restore(m, e.scratch, saved);
            return rc;
        }
    }
    chunk = m.word(e.scratch + TOKEN_CHUNK);
    let mut flag1 = 0u32;
    let mut flag2 = 0u32;
    let mut fail = false;
    rc = crate::render_part_a::find_reader_chunk(
        m,
        h,
        chunk,
        t.cfig,
        0,
        e.scratch + TOKEN_XTRA,
        e.scratch + TOKEN_DATA,
        &e.reader,
    );
    if rc >= 0x80000000 && (rc << 19) != 0xa00000 {
        fail = true;
    } else if (rc << 19) != 0xa00000 {
        let data = m.word(e.scratch + TOKEN_DATA);
        let h0 = m.half(data);
        if h0 != 0 {
            if m.half(data + 2) == 1 {
                flag1 = 1;
            }
            if h0 >= 2 && m.half(data + 4) == 1 {
                flag2 = 1;
            }
        }
        rc = crate::render_part_a::close_reader_chunk(m, h, chunk, &e.reader);
        if rc >= 0x80000000 {
            fail = true;
        }
    }
    if !fail {
        rc = crate::render_part_a::find_reader_chunk(
            m,
            h,
            chunk,
            t.file,
            0,
            e.scratch + TOKEN_XTRA,
            e.scratch + TOKEN_DATA,
            &e.reader,
        );
        if rc >= 0x80000000 {
            fail = true;
        } else {
            let data = m.word(e.scratch + TOKEN_DATA);
            m.put_half(lst, m.half(data));
            rc = crate::render_part_a::close_reader_chunk(m, h, chunk, &e.reader);
            if rc >= 0x80000000 {
                fail = true;
            }
        }
    }
    let cnt = m.half(lst) as u32;
    let mut done = false;
    if !fail {
        let arr = calloc(m, heap, 1, 1 + cnt * 4, h);
        m.put_word(lst + 4, arr);
        if arr == 0 {
            token_error(m, h, logger, category, 0x5df0, 0, &[], e);
            rc = crate::normalization::forward_error(10);
            if rc >= 0x80000000 {
                fail = true;
            } else {
                done = true;
            }
        } else {
            for i in 0..cnt {
                let el = calloc(m, heap, 1, 24, h);
                m.put_word(arr + 4 * i, el);
                if el == 0 {
                    token_error(m, h, logger, category, 0x5df0, 0, &[], e);
                    rc = crate::normalization::forward_error(10);
                    if rc >= 0x80000000 {
                        fail = true;
                    } else {
                        done = true;
                    }
                    break;
                }
                rc = crate::boot_port_4b::fn_24f570(m, h, o, chunk, el, flag1, flag2, &e.block);
                if rc >= 0x80000000 {
                    crate::boot_port_4b::fn_24f124(m, h, o, arr + 4 * i, &e.block);
                    m.put_word(arr + 4 * i, 0);
                    token_error(m, h, logger, category, 0x5dec, 0, &[], e);
                    fail = true;
                    break;
                }
                token_trace(
                    m,
                    h,
                    logger,
                    category,
                    e.formats.loaded,
                    &[i],
                    e,
                );
            }
        }
    }
    if !fail && !done {
        crate::render_part_a::close_reader(m, h, chunk, &e.reader);
        chunk = 0;
        let arr = m.word(lst + 4);
        for i in 0..cnt {
            let el = m.word(arr + 4 * i);
            let sub_list = m.word(el + 8);
            if el == 0 || sub_list == 0 {
                continue;
            }
            let ecount = m.half(sub_list + 0xc) as u32;
            let earr = m.word(sub_list + 8);
            for j in 0..ecount {
                let sub = earr + 0x1c * j;
                if m.byte(sub + 0x1b) == 1 {
                    let name = m.word(sub + 0x14).wrapping_add(m.half(sub) as u32);
                    let first = m.word(o + 0x54);
                    if first != 0 {
                        if crate::normalization::strcmp_at(m.bytes, first, name) != 0 {
                            token_error(
                                m,
                                h,
                                logger,
                                category,
                                0x5df5,
                                e.formats.clash,
                                &[t.first_assigned, first, t.clashes, name],
                                e,
                            );
                            let rc = crate::normalization::forward_error(0);
                            token_restore(m, e.scratch, saved);
                            return rc;
                        }
                    } else {
                        let n = crate::boot_port_1::fn_195d90(m, name) + 1;
                        let p = calloc(m, heap, 1, n, h);
                        m.put_word(o + 0x54, p);
                        if p == 0 {
                            token_error(m, h, logger, category, 0x5df0, 0, &[], e);
                            let rc = crate::normalization::forward_error(10);
                            token_restore(m, e.scratch, saved);
                            return rc;
                        }
                        crate::frontend::copy_str(m.bytes, p, name);
                        m.put_half(o + 0x58, i as u16);
                        m.put_half(o + 0x5a, j as u16);
                    }
                }
                if m.byte(sub + 6) == 1 {
                    let gcount = m.half(sub_list + 0x12) as u32;
                    let garr = m.word(sub_list + 0x10);
                    let limit = m.half(sub_list + 0x14) as u32;
                    for k in 0..gcount {
                        let g = garr + 8 * k;
                        let entries = m.word(g);
                        let count = m.half(g + 4) as u32;
                        for n in 0..count {
                            let mut found = false;
                            for w in 0..cnt {
                                let e2 = m.word(arr + 4 * w);
                                let kind = m.byte(e2);
                                if kind == 1 || kind == 4 {
                                    continue;
                                }
                                if limit == 0 {
                                    continue;
                                }
                                let v = m.half(entries + 6 * n) as u32;
                                if limit <= v {
                                    continue;
                                }
                                let list = m.word(sub_list + 0x18);
                                let target = m.word(list + 4 * v);
                                let name = m.word(m.word(e2 + 4));
                                if crate::normalization::strcmp_at(m.bytes, name, target) == 0 {
                                    m.put_half(entries + 6 * n + 2, w as u16);
                                    found = true;
                                    break;
                                }
                            }
                            if !found {
                                token_error(m, h, logger, category, 0x5ded, 0, &[], e);
                                token_restore(m, e.scratch, saved);
                                return rc;
                            }
                        }
                    }
                }
            }
            for w in 0..cnt {
                let e2 = m.word(arr + 4 * w);
                let rec_list = m.word(e2 + 0xc);
                if e2 == 0 || rec_list == 0 {
                    continue;
                }
                let recs = m.word(rec_list + 4);
                let count = m.half(rec_list + 8) as u32;
                for k in 0..count {
                    let rec = recs + 8 * k;
                    let name = m.word(rec);
                    for j in 0..ecount {
                        let sub = earr + 0x1c * j;
                        let other = m.word(sub + 0x14).wrapping_add(m.half(sub) as u32);
                        if crate::normalization::strcmp_at(m.bytes, name, other) == 0 {
                            m.put_half(rec + 6, i as u16);
                        }
                    }
                }
            }
        }
        for i in 0..cnt {
            let el = m.word(arr + 4 * i);
            let sub_list = m.word(el + 8);
            if el == 0 || sub_list == 0 {
                continue;
            }
            let ecount = m.half(sub_list + 0xc) as u32;
            let earr = m.word(sub_list + 8);
            for j in 0..ecount {
                let sub = earr + 0x1c * j;
                if m.half(sub + 0x10) != 0
                    && (m.byte(sub + 6) == 0 || m.half(sub_list + 0x12) == 0)
                {
                    token_error(m, h, logger, category, 0x5dfe, 0, &[], e);
                }
            }
        }
        if rc >= 0x80000000 {
            fail = true;
        } else if m.word(o + 0x54) == 0 {
            token_error(m, h, logger, category, 0x5df6, 0, &[], e);
            let rc = crate::normalization::forward_error(0);
            token_restore(m, e.scratch, saved);
            return rc;
        }
    }
    if fail {
        token_error(m, h, logger, category, 0x5df4, 0, &[], e);
    }
    if chunk != 0 {
        crate::render_part_a::close_reader(m, h, chunk, &e.reader);
    }
    token_restore(m, e.scratch, saved);
    rc
}
