
use crate::{frontend as f, normalization as n, render_part_a2 as a2};
fn w(m: &[u8], p: u32) -> u32 {
    u32::from_le_bytes(m[p as usize..p as usize + 4].try_into().unwrap())
}
fn h(m: &[u8], p: u32) -> u16 {
    u16::from_le_bytes(m[p as usize..p as usize + 2].try_into().unwrap())
}
fn pw(m: &mut [u8], p: u32, v: u32) {
    m[p as usize..p as usize + 4].copy_from_slice(&v.to_le_bytes());
}
fn ph(m: &mut [u8], p: u32, v: u16) {
    m[p as usize..p as usize + 2].copy_from_slice(&v.to_le_bytes());
}
fn nz(p: u32) -> Option<u32> {
    (p != 0).then_some(p)
}
pub(crate) fn entry(m: &[u8], o: u32, i: u32) -> Result<u32, u32> {
    if o == 0 {
        return Err(0x81102007);
    }
    let t = w(m, o + 12);
    let mut p = 0;
    let r = n::entry_offset(
        h(m, t + 2),
        w(m, t + 4) as i32,
        h(m, t + 8),
        h(m, o + 16),
        i,
        &mut p,
    );
    if r >= 0x80000000 {
        Err(r)
    } else {
        Ok(p as u32)
    }
}

pub trait Host: a2::RenderHost + crate::boot_part_b::FormatHost + f::StateLock {
    fn select_lock(&mut self, lock: u32);
    fn level(&mut self, instance: u32) -> u32;
    fn method(&mut self, m: &mut [u8], method: u32, args: &[u32]) -> u32;
}

pub struct Environment<'a> {
    pub constants: u32,
    pub scratch: u32,
    pub asian_key: u32,
    pub lang_key: u32,
    pub thai: u32,
    pub category: u32,
    pub ctype: &'a [u16; 256],
    pub context: &'a a2::ContextGlobals,
}
fn lookup(
    m: &[u8],
    obj: u32,
    key: u32,
    out: &mut u32,
    size: Option<&mut u32>,
    host: &mut impl Host,
) -> u32 {
    if obj != 0 {
        host.select_lock(w(m, obj + 4));
    }
    f::lookup_param(m, nz(obj), key, Some(out), size, host, |a, b| {
        match (a, b) {
            (0, 0) => 0,
            (0, _) => -1,
            (_, 0) => 1,
            _ => f::compare_str(&m[a as usize..], &m[b as usize..]),
        }
    })
}

pub fn initialize_object(
    m: &mut [u8],
    obj: u32,
    e: &Environment<'_>,
    host: &mut impl Host,
    compare_keys: impl FnMut(u32, u32) -> i32,
) -> u32 {
    if obj == 0 {
        return 0x81102007;
    }
    let ctx = w(m, obj + 8);
    let level = f::max_slot_level(m, nz(w(m, ctx + 16)), |p| host.level(p));
    m[obj as usize + 32] = u8::from(level >= 5);
    let plugin = w(m, obj + 48);
    let r = host.method(m, w(m, plugin + 44), &[w(m, obj + 52), w(m, obj + 56)]);
    if r >= 0x80000000 {
        return r;
    }
    let params = w(m, ctx + 20);
    let saved = w(m, e.scratch);
    if params != 0 {
        host.select_lock(w(m, params + 4));
    }
    let r = f::get_param_int(
        e.ctype,
        m,
        nz(params),
        e.asian_key,
        e.scratch,
        host,
        compare_keys,
    );
    let out = w(m, e.scratch);
    pw(m, e.scratch, saved);
    if r < 0x80000000 && out == 1 {
        m[obj as usize + 82] = 1;
    }
    let mut text = w(m, obj + 12);
    let r = lookup(m, params, e.lang_key, &mut text, None, host);
    pw(m, obj + 12, text);
    if r >= 0x80000000 {
        return r;
    }
    ph(m, obj + 80, 4);
    if f::compare_str(&m[text as usize..], &m[e.thai as usize..]) == 0 {
        ph(m, obj + 80, 2);
    }
    if h(m, obj + 74) != 0 {
        return 0;
    }
    let buf = crate::frontend::call_method(m, nz(w(m, ctx + 4)), 1, 80, host);
    pw(m, obj + 68, buf);
    if buf == 0 {
        return 0x8110200a;
    }
    ph(m, obj + 74, 10);
    0
}

pub fn normalize_full_span(
    m: &mut [u8],
    obj: u32,
    b: u32,
    c: u32,
    d: u32,
    fetch: u32,
    e: &Environment<'_>,
    host: &mut impl Host,
) -> u32 {
    if obj == 0 || b == 0 || c == 0 || d == 0 {
        return 0x81102007;
    }
    let mut arr = 0;
    let r = n::fetch_entry(m, fetch, 1012, 1, Some(&mut arr), host);
    if r >= 0x80000000 {
        return r;
    }
    let first = match entry(m, c, 0) {
        Ok(p) => p,
        Err(r) => return r,
    };
    let bytes = match entry(m, b, 0) {
        Ok(p) => p,
        Err(r) => return r,
    };
    let last = match entry(m, c, 0xffff) {
        Ok(p) => p,
        Err(r) => return r,
    };
    let len = w(m, last + 24);
    let width = len.wrapping_shl(15).wrapping_shr(16);
    let offset = w(m, last + 20).wrapping_sub(w(m, first + 20)) as u16;
    if width == 0 {
        return 0;
    }
    let heap = w(m, w(m, obj + 8) + 4);
    let size = len.wrapping_add(1) as u16 as u32;
    let buf = f::heap_alloc(m, nz(heap), size, host);
    if buf == 0 {
        return 0x8110200a;
    }
    n::memset_at(m, buf, 0, size);
    let count = size.wrapping_sub(1) as u16;
    n::memmove_at(m, buf, bytes + offset as u32, count as u32);
    let s = e.scratch;
    let saved = [m[s as usize + 124], m[s as usize + 125]];
    ph(m, s + 124, 0);
    let method = w(m, w(m, obj + 48) + 36);
    let mut r = host.method(
        m,
        method,
        &[
            w(m, obj + 52),
            w(m, obj + 56),
            buf,
            width,
            count as u32,
            d,
            fetch,
            arr,
            1012,
            s + 124,
            u32::from(w(m, last + 28) == 44),
            buf,
        ],
    );
    m[s as usize + 124..s as usize + 126].copy_from_slice(&saved);
    if r < 0x80000000 {
        r = a2::replace_token_text(m, b, c, d, 0, 0, e.constants, s, host);
    }
    f::heap_release(m, nz(heap), Some(buf), host);
    r
}
fn resolve(m: &[u8], pair: [u32; 2], globals: &a2::ContextGlobals) -> (u32, u32) {
    let a = w(&globals.pair, 0);
    let b = w(&globals.pair, 4);
    let pointed = if pair[0] != 0 && pair != [a, b] && pair[1] == 0x1e61 {
        Some(h(m, pair[0]))
    } else {
        None
    };
    let mut ctx = 0;
    let r = f::resolve_engine_context(
        pair[0],
        pointed,
        pair[1],
        &globals.pair,
        globals.default_context,
        &mut ctx,
    );
    (r, ctx)
}
fn log(m: &mut [u8], ctx: u32, code: u32, e: &Environment<'_>, host: &mut impl Host) {
    let mut saved = [0; 20];
    saved.copy_from_slice(&m[e.scratch as usize..e.scratch as usize + 20]);
    let logger = w(m, ctx + 16);
    crate::render_part_a::error_log(
        &mut crate::boot::Memory { base: 0, bytes: m },
        host,
        logger,
        e.category,
        code,
        0,
        0,
        e.scratch,
    );
    m[e.scratch as usize..e.scratch as usize + 20].copy_from_slice(&saved);
}

pub fn create_object(
    m: &mut [u8],
    args: &[u32; 15],
    e: &Environment<'_>,
    host: &mut impl Host,
) -> u32 {
    if args[2] == 0 {
        return 0x81102007;
    }
    let (r, ctx) = resolve(m, [args[0], args[1]], e.context);
    if r >= 0x80000000 {
        return 0x81102007;
    }
    pw(m, args[2], 0);
    let heap = w(m, ctx + 4);
    let obj = f::heap_alloc(m, nz(heap), 84, host);
    if obj == 0 {
        return 0x8110200a;
    }
    n::memset_at(m, obj, 0, 84);
    for (off, v) in [
        (0, args[0]),
        (4, args[1]),
        (8, ctx),
        (16, args[6]),
        (20, args[7]),
        (24, args[8]),
        (36, args[3]),
        (40, args[4]),
        (44, args[5]),
        (48, args[9]),
        (52, args[10]),
        (56, args[11]),
    ] {
        pw(m, obj + off, v);
    }
    for (off, v) in [(60, args[12]), (62, args[13]), (64, args[14])] {
        ph(m, obj + off, v as u16);
    }
    m[obj as usize + 76] = 1;
    m[obj as usize + 77] = 1;
    let s = e.scratch;
    let mut saved = [0; 256];
    saved.copy_from_slice(&m[s as usize..s as usize + 256]);
    let mut no_words = false;
    let r = create_inner(m, args, obj, ctx, e, host, &mut no_words);
    m[s as usize..s as usize + 256].copy_from_slice(&saved);
    if r < 0x80000000 {
        pw(m, args[2], obj);
        0
    } else {
        f::heap_release(m, nz(w(m, ctx + 4)), Some(obj), host);
        if no_words {
            log(m, ctx, 0x711, e, host);
        }
        r
    }
}
fn text(m: &mut [u8], p: u32, s: &[u8]) {
    m[p as usize..p as usize + s.len()].copy_from_slice(s);
}
fn config(
    m: &mut [u8],
    args: &[u32; 15],
    obj: u32,
    key: &[u8],
    count: u16,
    e: &Environment<'_>,
    host: &mut impl Host,
) -> u32 {
    let s = e.scratch;
    text(m, s + 160, b"fecfg\0");
    text(m, s + 176, key);
    ph(m, s + 144, count);
    host.method(
        m,
        w(m, args[6] + 28),
        &[
            w(m, obj + 20),
            w(m, obj + 24),
            s + 160,
            s + 176,
            s + 140,
            s + 144,
            s + 148,
        ],
    )
}
fn trim_config(m: &mut [u8], e: &Environment<'_>) -> u32 {
    let s = e.scratch;
    let p = w(m, w(m, s + 140));
    let delimiter = m[s as usize + 148] as u32;
    let found = f::find_byte(m, p, delimiter);
    if found != 0 {
        m[found as usize] = 0;
    }
    p
}
fn available(
    m: &mut [u8],
    args: &[u32; 15],
    obj: u32,
    key: u32,
    e: &Environment<'_>,
    host: &mut impl Host,
) -> u32 {
    host.method(
        m,
        w(m, args[6] + 64),
        &[w(m, obj + 20), w(m, obj + 24), key, e.scratch + 152],
    )
}
fn create_inner(
    m: &mut [u8],
    args: &[u32; 15],
    obj: u32,
    ctx: u32,
    e: &Environment<'_>,
    host: &mut impl Host,
    no_words: &mut bool,
) -> u32 {
    let s = e.scratch;
    text(m, s, b"win936\0");
    text(m, s + 16, b"cpextensions\0");
    text(m, s + 40, b"xlit23\0");
    if args[9] == 0 {
        log(m, ctx, 0x70e, e, host);
        return 0x81102000;
    }
    for (key, dst, code) in [
        (b"transcoder\0".as_slice(), s, 0x709),
        (b"translitdef\0".as_slice(), s + 40, 0x70b),
    ] {
        let r = config(m, args, obj, key, 0, e, host);
        if r >= 0x80000000 || h(m, s + 144) == 0 {
            log(m, ctx, code, e, host);
            return 0x81102000;
        }
        if h(m, s + 144) == 1 && w(m, s + 140) != 0 {
            let p = trim_config(m, e);
            f::copy_str(m, dst, p);
        }
    }
    n::memset_at(m, s + 64, 0, 7);
    text(m, s + 64, b"word\0");
    let r = available(m, args, obj, s + 64, e, host);
    if r >= 0x80000000 {
        return r;
    }
    let mut words = 0u32;
    if m[s as usize + 152] != 0 {
        words = 1;
    } else {
        loop {
            n::memset_at(m, s + 64, 0, 7);
            text(m, s + 64, b"word\0");
            n::itoa(m, words, s + 80, 10);
            f::append_str(m, s + 64, s + 80);
            let r = available(m, args, obj, s + 64, e, host);
            if r >= 0x80000000 {
                return r;
            }
            if m[s as usize + 152] == 0 {
                if words == 0 {
                    *no_words = true;
                    return 0x8110210a;
                }
                break;
            }
            words = words.wrapping_add(1);
        }
    }
    ph(m, obj + 28, words as u16);
    let mut version = config(m, args, obj, b"versioninfo\0", 0xffff, e, host);
    if h(m, s + 144) != 0 {
        let p = trim_config(m, e);
        version = 0;
        let v1 = n::strtoul_scan(m, p, s + 156, 10, 0) as i32;
        let ep = w(m, s + 156);
        if v1 >= 1 && ep != 0 && m[ep as usize] == b'.' {
            let v2 = n::strtoul_scan(m, ep + 1, s + 156, 10, 0) as i32;
            let ep = w(m, s + 156);
            if v2 >= 0 && ep != 0 && m[ep as usize] == b'.' {
                let v3 = n::strtoul_scan(m, ep + 1, s + 156, 10, 0) as i32;
                let ep = w(m, s + 156);
                if v3 >= 0
                    && ep != 0
                    && (m[ep as usize] == 0 || m[ep as usize] == b'.')
                    && v1 >= 5
                    && v2 >= 2
                    && v3 >= 1
                {
                    version = 1;
                }
            }
        }
    }
    m[obj as usize + 30] = version as u8;
    config(m, args, obj, b"useppdomains\0", 0xffff, e, host);
    let mut domains = 0;
    if h(m, s + 144) != 0 {
        let p = trim_config(m, e);
        if f::compare_str(&m[p as usize..], b"1\0") == 0 {
            domains = 1;
        }
    }
    let r = available(m, args, obj, s + 16, e, host);
    if r >= 0x80000000 {
        return r;
    }
    if m[s as usize + 152] == 0 {
        m[s as usize + 16] = 0;
    }
    let r = host.method(
        m,
        w(m, args[9] + 28),
        &[
            w(m, obj + 52),
            w(m, obj + 56),
            obj + 16,
            s + 40,
            s,
            domains,
            s + 16,
        ],
    );
    if r >= 0x80000000 {
        log(m, ctx, 0x70f, e, host);
    }
    r
}

pub trait RegionHost: Host + crate::render_part_c::Host + a2::NativeNormalizeHost {

    fn normalize_kernel(&mut self,_:&mut crate::boot::Memory<'_>,_:[u32;15])->u32 {panic!("live normalizer kernel must be bound")}

    fn unported_normalizer(&mut self, m: &mut [u8], entry: u32, args: &[u32]) -> u32;
}

pub struct RegionData<'a> {
    pub dump: &'a crate::render_part_c::DumpData,
    pub phases: [u32; 9],
    pub formats: [u32; 3],
    pub marker_names: u32,
    pub mime: u32,
}
fn diagnostic(
    m: &mut [u8],
    ctx: u32,
    event: u32,
    args: &[u32],
    data: &RegionData<'_>,
    host: &mut impl RegionHost,
) {
    let scratch = data.dump.stack;
    let mut saved = [0; 64];
    saved.copy_from_slice(&m[scratch as usize..scratch as usize + 64]);
    for (i, &v) in args.iter().enumerate() {
        pw(m, scratch + i as u32 * 4, v);
    }
    let logger = w(m, ctx + 16);
    crate::log_live::fn_1738e8(
        &mut crate::boot::Memory { base: 0, bytes: m },
        logger,
        data.dump.category,
        5,
        0,
        data.formats[event as usize],
        scratch,
        host,
    );
    m[scratch as usize..scratch as usize + 64].copy_from_slice(&saved);
}

fn raw_log(
    m: &mut [u8],
    ctx: u32,
    id: u32,
    text: u32,
    len: u32,
    data: &RegionData<'_>,
    host: &mut impl RegionHost,
) -> u32 {
    let logger = w(m, ctx + 16);
    let flag = n::flag_byte(m, logger);
    if logger == 0 || flag == 0 {
        return flag;
    }
    if text == 0 {
        return 0x84b02006;
    }
    let mut result = 0;
    for i in 0..6u32 {
        let pair = logger + if i == 5 { 52 } else { 8 + i * 8 };
        let vt = w(m, pair + 4);
        if vt != 0 {
            let method = w(m, vt + 52);
            if method != 0 {
                let r = host.method(
                    m,
                    method,
                    &[w(m, pair), id, data.dump.strings[0], data.mime, text, len],
                );
                if r >= 0x80000000 {
                    result = r;
                }
            }
        }
    }
    result
}
fn count(m: &[u8], o: u32) -> Result<u16, u32> {
    if o == 0 {
        return Err(0x81102007);
    }
    let mut v = 0;
    let t = w(m, o + 12);
    let r = n::inner_field(Some(&m[t as usize..]), Some(&mut v));
    if r >= 0x80000000 {
        Err(r)
    } else {
        Ok(v)
    }
}
pub(crate) fn index(m: &mut [u8], o: u32, i: u32) -> u32 {
    if o == 0 {
        return 0x81102007;
    }
    let t = w(m, o + 12);
    let mut v = h(m, o + 16);
    let r = n::set_index(h(m, t + 8), i as i32, &mut v);
    if r < 0x80000000 {
        ph(m, o + 16, v);
    }
    r
}
fn list32(m: &[u8], list: u32, value: u32) -> bool {
    let mut i = 0u16;
    loop {
        let v = w(m, list + i as u32 * 4);
        if v == 0 || v == value {
            return v == value;
        }
        i = i.wrapping_add(1);
    }
}
fn predicate(m: &[u8], kind: u32, arg: u32, ent: &[u8]) -> i32 {
    if arg == 0 {
        return 0;
    }
    if kind == 0x2b93c8 {
        let id = h(ent, 10);
        let mut i = 0u16;
        loop {
            let v = h(m, arg + i as u32 * 2);
            if v == 0 || v == id {
                return i32::from(v == id);
            }
            i = i.wrapping_add(1);
        }
    }
    if h(ent, 10) != 7 {
        return 0;
    }
    let member = list32(m, arg, w(ent, 28));
    i32::from(if kind == 0x2b9414 { member } else { !member })
}
pub(crate) fn scan(m: &mut [u8], o: u32, kind: u32, arg: u32) -> u32 {
    if o == 0 || kind == 0 {
        return 0x81102007;
    }
    let t = w(m, o + 12);
    let mut cursor = h(m, o + 16);
    let mut one = if kind == 0x2b94ac { h(m, arg) } else { 0 };
    let data = w(m, t + 4);
    let r = n::scan_forward(
        &m[data as usize..],
        h(m, t + 2),
        h(m, t + 8),
        &mut cursor,
        &mut |ent| {
            if kind == 0x2b94ac {
                if one == 0 {
                    1
                } else {
                    one = one.wrapping_sub(1);
                    0
                }
            } else {
                predicate(m, kind, arg, ent)
            }
        },
    );
    ph(m, o + 16, cursor);
    if kind == 0x2b94ac {
        ph(m, arg, one);
    }
    r
}
struct Predicates;
impl crate::selection::arena::Host for Predicates {
    fn call(
        &mut self,
        m: &mut crate::selection::arena::Arena<'_>,
        id: u32,
        a: &[u32],
    ) -> (u32, u32) {
        if a[0] == 0 || a[1] == 0 {
            return (0, 0);
        }
        let ent = a[1];
        let arg = a[0];
        if m.u16(ent + 10) != 7 {
            return (0, 0);
        }
        let value = m.u32(ent + 28);
        let mut i = 0u16;
        let member = loop {
            let v = m.u32(arg + i as u32 * 4);
            if v == 0 || v == value {
                break v == value;
            }
            i = i.wrapping_add(1);
        };
        (u32::from(if id == 0x2b9414 { member } else { !member }), 0)
    }
}
fn search(m: &mut [u8], o: u32, arg: u32, out: u32, back: bool) -> u32 {
    let mut a = crate::selection::arena::Arena::new(0, m);
    if back {
        crate::render_part_a::search_backward(&mut a, &mut Predicates, o, 0x2b9460, arg, out)
    } else {
        crate::render_part_a::search_forward(&mut a, &mut Predicates, o, 0x2b9460, arg, out)
    }
}
fn advance(m: &mut [u8], o: u32, s: u32) {
    ph(m, s, 1);
    scan(m, o, 0x2b94ac, s);
}
fn set_has(m: &[u8], set: u32, n: u32, v: u32) -> bool {
    let mut k = 0u16;
    while (k as u32) < n {
        if m[(set + k as u32) as usize] as u32 == v {
            return true;
        }
        k = k.wrapping_add(1);
    }
    false
}
fn dump(
    m: &mut [u8],
    obj: u32,
    a: u32,
    b: u32,
    phase: u32,
    data: &RegionData<'_>,
    host: &mut impl RegionHost,
) {
    let flag = m[obj as usize + 32] as u32;
    let ctx = w(m, obj + 8);
    crate::render_part_c::fn_2b9770(
        &mut crate::boot::Memory { base: 0, bytes: m },
        flag,
        ctx,
        a,
        b,
        data.phases[phase as usize],
        data.dump,
        host,
    );
}

pub fn normalize_regions(
    m: &mut [u8],
    obj: u32,
    d: u32,
    rules: u32,
    e: &Environment<'_>,
    data: &RegionData<'_>,
    host: &mut impl RegionHost,
) -> u32 {
    if obj == 0 || d == 0 {
        return 0x81102007;
    }
    let mut saved = [0; 1024];
    saved.copy_from_slice(&m[e.scratch as usize..e.scratch as usize + 1024]);
    let mut text = 0;
    let mut markers = 0;
    let mut r = n::open_iterator(m, w(m, obj), w(m, obj + 4), d, 0x3fc, Some(&mut text), host);
    if r < 0x80000000 {
        r = n::open_iterator(
            m,
            w(m, obj),
            w(m, obj + 4),
            d,
            0x3fd,
            Some(&mut markers),
            host,
        );
    }
    if r < 0x80000000 {
        r = regions_inner(m, obj, d, rules, text, markers, e, data, host);
    }
    if markers != 0 {
        n::release_iterator(m, markers, host);
    }
    if text != 0 {
        n::release_iterator(m, text, host);
    }
    m[e.scratch as usize..e.scratch as usize + 1024].copy_from_slice(&saved);
    r
}
fn regions_inner<H: RegionHost>(
    m: &mut [u8],
    obj: u32,
    d: u32,
    rules: u32,
    text_stream: u32,
    markers: u32,
    e: &Environment<'_>,
    data: &RegionData<'_>,
    host: &mut H,
) -> u32 {

    let run = |m: &mut [u8], host: &mut H| -> Result<u32, u32> {
        macro_rules! check {
            ($r:expr) => {{
                let r = $r;
                if r >= 0x80000000 {
                    return Err(r);
                }
                r
            }};
        }
        let s = e.scratch;
        let cur = s;
        let blk = s + 16;
        let out = s + 48;
        let rec = s + 64;
        let temp = s + 128;
        let aux = s + 256;
        dump(m, obj, text_stream, markers, 0, data, host);
        n::memmove_at(m, blk, e.constants + 0x11c, 24);
        n::memmove_at(m, cur, e.constants + 0x28, 8);
        if text_stream == 0 || markers == 0 {
            return Err(0x81102007);
        }
        check!(index(m, text_stream, 0));
        check!(index(m, markers, 0));
        let mut r = scan(m, markers, 0x2b9414, cur);
        while r < 0x80000000 {
            let rc = search(m, markers, blk, out, true);
            let before = if rc & 0x1fff == 0x14 {
                0xffff
            } else {
                check!(rc);
                h(m, out)
            };
            let rc = search(m, markers, blk, out + 2, false);
            let after = if rc & 0x1fff == 0x14 {
                count(m, markers)?
            } else {
                check!(rc);
                h(m, out + 2)
            };
            if before != 0xffff {
                check!(index(m, markers, before as u32));
            }
            check!(a2::remove_native_blanks(
                m,
                w(m, obj + 8),
                markers,
                after as u32,
                text_stream,
                e.constants + 0x20,
                aux,
                host
            ));
            r = scan(m, markers, 0x2b9414, cur);
        }
        if r & 0x1fff != 0x14 {
            return Err(r);
        }
        dump(m, obj, text_stream, markers, 1, data, host);
        pw(m, cur, w(m, obj + 8));
        let arr = entry(m, markers, 0)?;
        let cnt = count(m, markers)?;
        let mut i = 0u16;
        while i < h(m, rules) {
            let rule = w(m, rules + 4) + i as u32 * 28;
            let w0 = w(m, rule);
            let cb = h(m, rule + 8);
            let sb = w(m, rule + 12);
            let cc = h(m, rule + 16);
            let sc = w(m, rule + 20);
            let once = w(m, rule + 24) & 255;
            let hi = w0 >> 16;
            let mid = w0 >> 8 & 255;
            let lo = w0 & 255;
            let mut ip = 1u16;
            while ip < cnt
                && !(h(m, arr + ip as u32 * 40 + 10) == 7
                    && (once == 1 || w(m, arr + ip as u32 * 40 + 28) == lo))
            {
                ip = ip.wrapping_add(1);
            }
            let mut left = 0i32;
            let mut right = 0i32;
            while ip < cnt {
                for _ in 0..hi {
                    let mut j = ip.wrapping_sub(1);
                    left = ip as i32;
                    while right < (j as i32) {
                        let ent = arr + j as u32 * 40;
                        if h(m, ent + 10) == 7 {
                            if set_has(m, w(m, rule + 4), hi, w(m, ent + 28)) {
                                left = j as i32;
                            } else {
                                break;
                            }
                        }
                        j = j.wrapping_sub(1);
                    }
                }
                if cb != 0 {
                    right = ip as i32;
                    let mut j = ip.wrapping_add(1);
                    while j < cnt {
                        let ent = arr + j as u32 * 40;
                        if h(m, ent + 10) == 7 {
                            if set_has(m, sb, cb as u32, w(m, ent + 28)) {
                                right = j as i32;
                            } else {
                                break;
                            }
                        }
                        j = j.wrapping_add(1);
                    }
                } else if cc != 0 {
                    right = ip as i32;
                    let j = ip.wrapping_add(1);
                    if j < cnt {
                        let ent = arr + j as u32 * 40;

                        while h(m, ent + 10) != 7 {
                            core::hint::spin_loop();
                        }
                        if set_has(m, sc, cc as u32, w(m, ent + 28)) {
                            right = j as i32;
                        }
                    }
                }
                if left != right {
                    diagnostic(
                        m,
                        w(m, obj + 8),
                        0,
                        &[i as u32, left as u32, right as u32],
                        data,
                        host,
                    );
                    let mut k = left as u16;
                    while k as i32 <= right {
                        let ent = arr + k as u32 * 40;
                        if h(m, ent + 10) == 7 && (mid == 3 || w(m, ent + 28) != 1) {
                            pw(m, ent + 28, mid);
                        }
                        k = k.wrapping_add(1);
                    }
                }
                if once == 1 {
                    ip = cnt;
                } else {
                    ip = (right as u16).wrapping_add(1);
                    while ip < cnt
                        && !(h(m, arr + ip as u32 * 40 + 10) == 7
                            && w(m, arr + ip as u32 * 40 + 28) == lo)
                    {
                        ip = ip.wrapping_add(1);
                    }
                }
            }
            i = i.wrapping_add(1);
        }
        dump(m, obj, text_stream, markers, 2, data, host);
        check!(a2::merge_marker_runs(
            m,
            obj,
            text_stream,
            markers,
            e.constants,
            temp,
            host
        ));
        dump(m, obj, text_stream, markers, 3, data, host);
        let mut previous = 0u32;
        pw(m, blk, w(m, e.constants + 12));
        let ctx = w(m, obj + 8);
        let plugin = w(m, obj + 36);
        if plugin != 0 {
            m[(out + 4) as usize] = 0;
            check!(host.method(
                m,
                w(m, plugin + 60),
                &[w(m, obj + 40), w(m, obj + 44), out + 4]
            ));
            if m[(out + 4) as usize] != 0 {
                let mut iterator = 0;
                let phase = (|| -> Result<(), u32> {
                    if n::flag_byte(m, w(m, ctx + 16)) != 0 {
                        let text = entry(m, text_stream, 0)?;
                        let cnt = count(m, text_stream)?;
                        raw_log(m, ctx, 0x68, text, (cnt as u32) << 1, data, host);
                    }
                    let cnt = count(m, text_stream)?;
                    let mut p = 0;
                    let r =
                        n::fetch_entry(m, d, 0x405, cnt.wrapping_add(1) as u32, Some(&mut p), host);
                    if r >= 0x80000000 {
                        if r & 0x1fff == 0x14 {
                            return Ok(());
                        }
                        return Err(r);
                    }
                    check!(n::open_iterator(
                        m,
                        w(m, obj),
                        w(m, obj + 4),
                        d,
                        0x405,
                        Some(&mut iterator),
                        host
                    ));
                    check!(index(m, text_stream, 0));
                    check!(index(m, markers, 0));
                    let mut r = scan(m, markers, 0x2b93c8, blk);
                    while r < 0x80000000 {
                        let ent = entry(m, markers, 0xffff)?;
                        let kind = w(m, ent + 28);
                        match kind {
                            2 | 3 | 5 => {
                                check!(crate::leftovers_1::fn_254aac(
                                    m,
                                    [obj, text_stream, markers, iterator, d],
                                    e.constants,
                                    e.scratch + 768,
                                    host
                                ));
                            }
                            4 | 6 => {
                                check!(a2::process_token_span(
                                    m,
                                    obj,
                                    text_stream,
                                    markers,
                                    iterator,
                                    d,
                                    e.constants,
                                    aux,
                                    host
                                ));
                            }
                            _ => {}
                        }
                        advance(m, markers, out + 6);
                        r = scan(m, markers, 0x2b93c8, blk);
                    }
                    if r & 0x1fff == 0x14 {
                        Ok(())
                    } else {
                        Err(r)
                    }
                })();
                if n::flag_byte(m, w(m, ctx + 16)) != 0 {
                    let text = entry(m, text_stream, 0)?;
                    let cnt = count(m, text_stream)?;
                    raw_log(m, ctx, 0x69, text, (cnt as u32) << 1, data, host);
                }
                if iterator != 0 {
                    n::release_iterator(m, iterator, host);
                }
                phase?;
            }
        }
        dump(m, obj, text_stream, markers, 4, data, host);
        check!(a2::merge_marker_runs(
            m,
            obj,
            text_stream,
            markers,
            e.constants,
            temp,
            host
        ));
        dump(m, obj, text_stream, markers, 5, data, host);
        n::memmove_at(m, blk, e.constants + 0x15c, 20);
        n::memmove_at(m, cur, e.constants + 0x30, 8);
        check!(index(m, text_stream, 0));
        check!(index(m, markers, 0));
        let mut r = scan(m, markers, 0x2b9414, cur);
        while r < 0x80000000 {
            let rc = search(m, markers, blk, out, true);
            let before = if rc & 0x1fff == 0x14 {
                0xffff
            } else {
                check!(rc);
                h(m, out)
            };
            let rc = search(m, markers, blk, out + 2, false);
            let after = if rc & 0x1fff == 0x14 {
                0xffff
            } else {
                check!(rc);
                h(m, out + 2)
            };
            let a = if before == 0xffff {
                0
            } else {
                w(m, entry(m, markers, before as u32)? + 28) & 255
            };
            let b = if after == 0xffff {
                0
            } else {
                w(m, entry(m, markers, after as u32)? + 28) & 255
            };
            if a == 0 || b == 0 || a == b || b == 14 || b == 44 {
                advance(m, markers, out + 6);
            } else {
                check!(index(m, markers, before as u32));
                check!(a2::remove_native_blanks(
                    m,
                    ctx,
                    markers,
                    after as u32,
                    text_stream,
                    e.constants + 0x20,
                    aux,
                    host
                ));
            }
            r = scan(m, markers, 0x2b9414, cur);
        }
        if r & 0x1fff != 0x14 {
            return Err(r);
        }
        dump(m, obj, text_stream, markers, 6, data, host);
        n::memmove_at(m, rec, e.constants + 0x134, 40);
        let mut tab = 0;
        check!(n::fetch_entry(m, d, 0x3f4, 1, Some(&mut tab), host));
        let cnt = count(m, markers)?;
        let arr = entry(m, markers, 0)?;
        if cnt > 1 {
            let mut i = 1u16;
            let mut flag = true;
            'spell: while i < cnt {
                let ent = arr + i as u32 * 40;
                if h(m, ent + 10) == 21 {

                    if tab != 0
                        && n::u16_cmp(m, e.constants + 0x7a, w(m, tab + 4) + w(m, ent + 28) * 4)
                            == 0
                    {
                        flag = true;
                    } else {
                        flag = false;
                        i = i.wrapping_add(1);
                        continue;
                    }
                } else if !flag {
                    i = i.wrapping_add(1);
                    continue;
                }
                if h(m, ent + 10) != 7
                    || w(m, ent + 24) != 2
                    || !matches!(w(m, ent + 28), 3 | 4 | 5 | 6 | 7 | 8)
                {
                    i = i.wrapping_add(1);
                    continue;
                }
                let start = w(m, ent + 20);
                let len = w(m, ent + 24);
                let w12 = w(m, ent + 12);
                let w16 = w(m, ent + 16);
                let mut mark = 0xffff;
                let mut j = 1u16;
                while j < i {
                    let p = arr + j as u32 * 40;
                    if h(m, p + 10) == 7 && w(m, p + 28) == 1 {
                        mark = w(m, p + 20);
                    }
                    j = j.wrapping_add(1);
                }
                j = 1;
                while j < cnt {
                    let p = arr + j as u32 * 40;
                    if w(m, p + 20) > start {
                        break;
                    }
                    if h(m, p + 10) == 0x28 {
                        if (mark == 0xffff && w(m, p + 24).wrapping_add(w(m, p + 20)) == start)
                            || (mark != 0xffff
                                && w(m, arr + 20) == mark
                                && w(m, arr + 20) <= w(m, p + 20)
                                && w(m, p + 20) <= start)
                        {
                            break 'spell;
                        }
                    }
                    j = j.wrapping_add(1);
                }
                let mut end = 0xffff;
                let mut endlen = 0xffff;
                j = i.wrapping_add(1);
                while j < cnt {
                    let p = arr + j as u32 * 40;
                    if start.wrapping_add(len) != w(m, p + 20) {
                        break;
                    }
                    if h(m, p + 10) == 7 && w(m, p + 28) == 1 {
                        end = w(m, p + 20);
                        endlen = w(m, p + 24);
                        break;
                    }
                    j = j.wrapping_add(1);
                }
                if (mark == 0xffff && w(m, arr + 20) != start)
                    || (mark != 0xffff && w(m, arr + 20) != mark)
                {
                    break;
                }
                let overall_end = w(m, arr + 24).wrapping_add(w(m, arr + 20));
                if if end != 0xffff {
                    overall_end != end.wrapping_add(endlen)
                } else {
                    start.wrapping_add(len) != overall_end
                } {
                    break;
                }
                pw(m, ent + 28, 14);
                ph(m, rec + 16, 21);
                check!(a2::intern_string(
                    m,
                    tab,
                    e.constants + 0x88,
                    rec + 28,
                    d,
                    host
                ));
                pw(m, rec + 12, w12);
                pw(m, rec + 16, 0);
                pw(m, rec + 32, w(m, ent + 20));
                pw(m, rec + 36, 0);
                check!(index(m, markers, i as u32));
                check!(n::insert_ordered(m, markers, rec, host));
                ph(m, rec + 16, 21);
                check!(a2::intern_string(
                    m,
                    tab,
                    e.constants + 0x7a,
                    rec + 28,
                    d,
                    host
                ));
                pw(m, rec + 12, w12.wrapping_add(w16));
                pw(m, rec + 16, 0);
                pw(m, rec + 32, start.wrapping_add(len));
                pw(m, rec + 36, 0);
                check!(n::insert_ordered(m, markers, rec, host));
                break;
            }
        }
        dump(m, obj, text_stream, markers, 7, data, host);
        pw(m, blk, w(m, e.constants + 16));
        let mut iterator = 0;
        let mut phase_rc = 0u32;
        let phase = (|| -> Result<(), u32> {
            let cnt = count(m, text_stream)?;
            let mut tab = 0;
            let r = n::fetch_entry(
                m,
                d,
                0x405,
                cnt.wrapping_add(1) as u32,
                Some(&mut tab),
                host,
            );

            if r >= 0x80000000 {
                return Ok(());
            }
            check!(n::open_iterator(
                m,
                w(m, obj),
                w(m, obj + 4),
                d,
                0x405,
                Some(&mut iterator),
                host
            ));
            check!(index(m, text_stream, 0));
            check!(index(m, markers, 0));
            let mut r = scan(m, markers, 0x2b93c8, blk);
            while r < 0x80000000 {
                let ent = entry(m, markers, 0xffff)?;
                n::memmove_at(m, rec, ent, 40);
                let v = w(m, ent + 28);
                let name = crate::render_part_c::fn_2b9708(
                    &mut crate::boot::Memory { base: 0, bytes: m },
                    v & 255,
                    data.marker_names,
                    data.dump.plus,
                );
                diagnostic(
                    m,
                    ctx,
                    1,
                    &[name, w(m, ent + 20), w(m, ent + 24)],
                    data,
                    host,
                );
                match v {
                    1 => {
                        check!(a2::append_marker_text(
                            m,
                            obj,
                            text_stream,
                            markers,
                            iterator,
                            e.constants,
                            aux,
                            host
                        ));
                    }
                    14 | 44 => {
                        check!(normalize_full_span(
                            m,
                            obj,
                            text_stream,
                            markers,
                            iterator,
                            d,
                            e,
                            host
                        ));
                    }
                    2 | 7 => {
                        check!(a2::normalize_native_one(
                            m,
                            obj,
                            text_stream,
                            markers,
                            iterator,
                            d,
                            e.constants,
                            aux,
                            host
                        ));
                    }
                    3 | 4 | 15 | 16 | 43 | 60 | 70 | 71 => {
                        check!(a2::convert_native_plain(
                            m,
                            obj,
                            text_stream,
                            markers,
                            iterator,
                            e.constants,
                            aux,
                            host
                        ));
                    }
                    5 => {
                        if matches!(previous as u8, 0 | 16) {
                            check!(a2::normalize_native_one(
                                m,
                                obj,
                                text_stream,
                                markers,
                                iterator,
                                d,
                                e.constants,
                                aux,
                                host
                            ));
                        } else {
                            check!(a2::normalize_native_zero(
                                m,
                                obj,
                                text_stream,
                                markers,
                                iterator,
                                d,
                                e.constants,
                                aux,
                                host
                            ));
                        }
                    }
                    6 | 8 => {
                        if matches!(previous as u8, 0 | 16) {
                            check!(a2::convert_native_marked(
                                m,
                                obj,
                                text_stream,
                                markers,
                                iterator,
                                e.constants,
                                aux,
                                host
                            ));
                        } else {
                            check!(a2::convert_native_plain(
                                m,
                                obj,
                                text_stream,
                                markers,
                                iterator,
                                e.constants,
                                aux,
                                host
                            ));
                        }
                    }
                    _ => {
                        check!(a2::remove_token_span(
                            m,
                            obj,
                            text_stream,
                            markers,
                            iterator
                        ));
                    }
                }
                check!(index(m, markers, 0));
                loop {
                    let r = scan(m, markers, 0x2b93c8, blk);
                    if r >= 0x80000000 && r & 0x1fff != 0x14 {
                        return Err(r);
                    }
                    let ent = entry(m, markers, 0xffff)?;
                    if w(m, ent + 12) == w(m, rec + 12)
                        && w(m, ent + 16) == w(m, rec + 16)
                        && w(m, ent + 20) == w(m, rec + 20)
                        && crate::fe_text::memcmp_195aac(m, rec + 28, ent + 28, 4) == 0
                    {
                        break;
                    }
                    advance(m, markers, out + 6);
                }
                let ent = entry(m, markers, 0xffff)?;
                let v = w(m, ent + 28) & 255;
                if matches!(v, 1 | 5 | 6 | 7 | 8) {
                    if matches!(previous as u8, 0 | 43) {
                        previous = 16;
                    }
                } else {
                    previous = if v == 2 { 16 } else { v };
                }
                advance(m, markers, out + 6);
                r = scan(m, markers, 0x2b93c8, blk);
            }
            if r & 0x1fff != 0x14 {
                return Err(r);
            }
            let arr = entry(m, markers, 0)?;
            let mut cnt = count(m, markers)?;
            let mut i = 1u16;
            while i < cnt {
                let ent = arr + i as u32 * 40;
                if h(m, ent + 10) == 7
                    && w(m, ent + 24) == 0
                    && matches!(w(m, ent + 28) & 255, 2 | 5 | 7)
                {
                    let p = w(m, ent + 4);
                    if p != 0 {
                        f::heap_release(m, nz(w(m, ctx + 4)), Some(p), host);
                        pw(m, ent + 4, 0);
                        ph(m, ent + 8, 0);
                    }
                    check!(n::delete_range(m, markers, i, 1));
                    cnt = cnt.wrapping_sub(1);
                    diagnostic(m, ctx, 2, &[i as u32], data, host);
                }
                i = i.wrapping_add(1);
            }
            Ok(())
        })();
        if let Err(r) = phase {
            phase_rc = r;
        }
        if iterator != 0 {
            n::release_iterator(m, iterator, host);
        }
        if phase_rc >= 0x80000000 {
            return Err(phase_rc);
        }
        dump(m, obj, text_stream, markers, 8, data, host);
        Ok(phase_rc)
    };
    match run(m, host) {
        Ok(r) | Err(r) => r,
    }
}
