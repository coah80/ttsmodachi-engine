//! Boot support in 0x170000..0x19ffff. Guest addresses refer to caller-owned memory.
//! External engine services remain explicit call boundaries, not host pointers.
use crate::boot::Memory;

/// Calls an external engine service or a guest vtable method.
/// Arguments and results are guest words. Services may mutate the caller's arena.
/// The differential suite checks these boundaries independently with scripted replies;
/// it does not establish an end-to-end cold boot.
pub trait Host {
    /// Unported archive services receive real memory and local outputs.
    fn archive_service(&mut self,_:&mut Memory<'_>,f:u32,_:&[u32],_:&mut [u32;4])->u32 {panic!("unbound archive service {f:x}")}
    fn archive_read(&mut self,_:&mut Memory<'_>,f:u32,_:u32,_:&mut [u8])->u32 {panic!("unbound archive read {f:x}")}

    fn call(&mut self, memory: &mut Memory<'_>, function: u32, args: &[u32]) -> u32;
    fn thread_id(&mut self) -> u32;
    /// Optional composed dictionary teardown. None retains the call boundary.
    fn release_dictionary(&mut self, _m:&mut Memory<'_>, _p:u32)->Option<u32> {None}
    /// The ROM-region address of a format literal, when the host can supply
    /// one. The ARM passes the literal's address to fn_173dc0 and fn_1738e8.
    /// The recorded-reply ports pass the literal's hash instead and leave
    /// this None.
    fn format_address(&mut self, _format:&[u8])->Option<u32> {None}
    /// Whether log calls carry the format's address and the real varargs.
    /// Hosts that compare the heap byte for byte need the real values;
    /// hosts that normalize log arguments (e.g. by hashing them for the
    /// trace) leave this false and receive the literal hashes.
    fn real_log_args(&mut self)->bool {false}
}

/// The broker format literals the query path logs with, in a fixed order.
/// A host supplies the ROM-region address of each literal in the same order
/// through Host::format_address; the literals are the ones the ARM's
/// fn_177e0c passes to fn_1738e8 and fn_173dc0.
pub const BROKER_FORMATS:[&[u8];7]=[
    b"%s%s%s%x",
    b"%s%s",
    b"brk_InterfaceQuery BrokerString=%s : Begin",
    b"brk_InterfaceQuery : osspi_DynModOpen %s, %d",
    b"brk_InterfaceQuery : osspi_DynModObjLoad %s",
    b"brk_InterfaceQuery : BrokerString=%s: Module Not Found (%x)",
    b"brk_InterfaceQuery BrokerString=%s : End",
];
fn call(m: &mut Memory<'_>, h: &mut impl Host, f: u32, a: &[u32]) -> u32 {
    h.call(m, f, a)
}
/// fn_17177c.
pub fn fn_17177c(m: &mut Memory<'_>, h: &mut impl Host) {
    call(m, h, 0x184c58, &[0]);
}
/// fn_187498.
pub fn fn_187498(m: &mut Memory<'_>, h: &mut impl Host) {
    call(m, h, 0x184c58, &[0]);
}
fn block(
    m: &mut Memory<'_>,
    heap: u32,
    b: u32,
    c: u32,
    d: u32,
    out: u32,
    err: u32,
    h: &mut impl Host,
) -> u32 {
    if c == 0 {
        return err | 7;
    }
    let p = call(m, h, 0x1963b8, &[heap, 1, 16]);
    m.put_word(out, p);
    if m.word(out) == 0 {
        return err | 10;
    }
    m.put_word(m.word(out), heap);
    m.put_word(m.word(out).wrapping_add(4), b);
    m.put_word(m.word(out).wrapping_add(8), c);
    m.put_word(m.word(out).wrapping_add(12), d);
    0
}
/// fn_172e20.
pub fn fn_172e20(m: &mut Memory<'_>, a: [u32; 5], h: &mut impl Host) -> u32 {
    block(m, a[0], a[1], a[2], a[3], a[4], 0x80602000, h)
}
/// fn_176dac.
pub fn fn_176dac(m: &mut Memory<'_>, a: [u32; 5], h: &mut impl Host) -> u32 {
    block(m, a[0], a[1], a[2], a[3], a[4], 0x81402000, h)
}
/// fn_172e84.
pub fn fn_172e84(m: &mut Memory<'_>, p: u32, h: &mut impl Host) -> u32 {
    if p == 0 {
        return 0x80602007;
    }
    call(m, h, 0x196428, &[m.word(p), p]);
    0
}
/// fn_176e10.
pub fn fn_176e10(m: &mut Memory<'_>, p: u32, h: &mut impl Host) -> u32 {
    if p == 0 || m.word(p) == 0 {
        return 0x81402007;
    }
    let v = m.word(p);
    call(m, h, 0x196428, &[m.word(v), v]);
    m.put_word(p, 0);
    0
}
/// fn_174988. The caller supplies the regional address of D_8d75cc.
pub fn fn_174988(m: &mut Memory<'_>, sel: u32, out: u32, table: u32) -> u32 {
    if sel <= 1 {
        m.put_word(out, table);
        0
    } else {
        0x84b02001
    }
}
/// fn_183aec.
pub fn fn_183aec(m: &mut Memory<'_>, p: u32, h: &mut impl Host) -> u32 {
    if p == 0 {
        return 0;
    }
    let method = m.word(m.word(p) + 48);
    call(m, h, method, &[p])
}
/// fn_1840fc. Exclusive acquisition is supplied by the platform service.
pub fn fn_1840fc(m: &mut Memory<'_>, p: u32, h: &mut impl Host) {
    if h.thread_id() != m.word(p + 4) {
        call(m, h, 0x183edc, &[p]);
        let tid = h.thread_id();
        m.put_word(p + 4, tid);
    }
    m.put_word(p + 8, m.word(p + 8).wrapping_add(1));
}
/// fn_1840e8.
pub fn fn_1840e8(m: &mut Memory<'_>, h: &mut impl Host) {
    let p = call(m, h, 0x1858e4, &[]);
    fn_1840fc(m, p + 52, h);
}
/// fn_18423c.
pub fn fn_18423c(m: &mut Memory<'_>, p: u32, h: &mut impl Host) {
    let n = m.word(p + 8).wrapping_sub(1);
    m.put_word(p + 8, n);
    if n == 0 {
        m.put_word(p + 4, 0);
        call(m, h, 0x165d00, &[p]);
    }
}
/// fn_185d40.
pub fn fn_185d40(m: &mut Memory<'_>, h: &mut impl Host) {
    let p = call(m, h, 0x1858e4, &[]);
    fn_18423c(m, p + 52, h);
}
/// fn_188f9c.
pub fn fn_188f9c(m: &mut Memory<'_>, t: u32, i: i32, out: u32) -> u32 {
    let entries = m.word(t + 4);
    if i >= 0 && (m.word(t + 12) as i32) > i {
        let e = entries.wrapping_add((i as u32).wrapping_mul(12));
        if m.word(e) & 0xff000000 == 0 {
            m.put_word(out, t);
            m.put_word(out + 4, m.word(e + 4));
            m.put_word(out + 8, m.word(e + 8));
            return 1;
        }
    }
    0
}
/// fn_189030.
pub fn fn_189030(m: &Memory<'_>, p: u32) -> u32 {
    m.word(m.word(p)).wrapping_add(m.word(p + 4))
}
fn method(m: &mut Memory<'_>, p: u32, a: u32, b: u32, slot: u32, h: &mut impl Host) -> u32 {
    if p == 0 || (slot == 24 && (a == 0 || b == 0)) {
        return 0x80b02007;
    }
    let f = m.word(m.word(p) + slot);
    if f == 0 {
        return 0x80b02001;
    }
    let obj = m.word(p + 4);
    if slot == 20 {
        call(m, h, f, &[obj, a])
    } else {
        call(m, h, f, &[obj, a, b])
    }
}
/// fn_196448.
pub fn fn_196448(m: &mut Memory<'_>, p: u32, a: u32, b: u32, h: &mut impl Host) -> u32 {
    method(m, p, a, b, 16, h)
}
/// fn_196474.
pub fn fn_196474(m: &mut Memory<'_>, p: u32, a: u32, h: &mut impl Host) -> u32 {
    method(m, p, a, 0, 20, h)
}
/// fn_1964a0.
pub fn fn_1964a0(m: &mut Memory<'_>, p: u32, a: u32, b: u32, h: &mut impl Host) -> u32 {
    method(m, p, a, b, 24, h)
}
/// fn_19af64.
pub fn fn_19af64(m: &Memory<'_>, p: u32) -> u32 {
    let n = m.word(p) >> 8;
    if n == 0 {
        m.word(p + 4)
    } else {
        n
    }
}
/// fn_198280.
pub fn fn_198280(m: &mut Memory<'_>, p: u32, h: &mut impl Host) -> u32 {
    if p != 0 {
        call(m, h, 0x19665c, &[p]);
    }
    0
}
fn equal(m: &Memory<'_>, p: u32, s: &[u8]) -> bool {
    for (i, &b) in s.iter().enumerate() {
        if m.byte(p + i as u32) != b {
            return false;
        }
    }
    true
}
/// fn_198358. Consume whitespace and XML comments, retaining partial progress on error.
pub fn fn_198358(m: &mut Memory<'_>, ctx: u32) -> u32 {
    loop {
        let mut p = m.word(ctx + 8);
        while matches!(m.byte(p), b' ' | b'\t' | b'\n' | b'\r') {
            p += 1;
            m.put_word(ctx + 8, p);
        }
        if !equal(m, p, b"<!--") {
            return 0;
        }
        let mut q = p;
        while m.byte(q) != 0 && !equal(m, q, b"-->") {
            q += 1;
        }
        if m.byte(q) == 0 || m.byte(q - 1) == b'-' {
            return 0x82206004;
        }
        m.put_word(ctx + 8, q + 3);
    }
}
/// fn_198414. Decode the five XML entities in place.
pub fn fn_198414(m: &mut Memory<'_>, s: u32) -> u32 {
    let mut p = s;
    while m.byte(p) != 0 && m.byte(p) != b'&' {
        p += 1
    }
    if m.byte(p) == 0 {
        return 0;
    }
    let mut w = p;
    while m.byte(p) != 0 {
        let c = m.byte(p);
        if c != b'&' {
            put_byte(m, w, c);
            p += 1;
        } else {
            let mut matched = false;
            for (text, c) in [
                (b"&amp;".as_slice(), b'&'),
                (b"&lt;", b'<'),
                (b"&gt;", b'>'),
                (b"&apos;", b'\''),
                (b"&quot;", b'"'),
            ] {
                if equal(m, p, text) {
                    p += text.len() as u32;
                    put_byte(m, w, c);
                    matched = true;
                    break;
                }
            }
            if !matched {
                return 0x82206004;
            }
        }
        w += 1;
    }
    put_byte(m, w, 0);
    0
}
/// fn_18cc30.
pub fn fn_18cc30(m: &mut Memory<'_>, p: u32, h: &mut impl Host) -> u32 {
    fn_1840fc(m, p + 100, h);
    let i = m.word(p + 32);
    let c = m.byte(
        m.word(p + 20)
            .wrapping_add(i.wrapping_mul(24))
            .wrapping_add(17),
    );
    let rc = u32::from(matches!(c, 0 | 1 | 3));
    fn_18423c(m, p + 100, h);
    rc
}
/// fn_18cf04.
pub fn fn_18cf04(m: &mut Memory<'_>, p: u32, out: u32, h: &mut impl PairHost) -> u32 {
    // PairHost returns the temporary pair by value; no guest stack address
    // is invented inside the caller's arena.
    let (rc, lo, hi) = h.pair(m, m.word(p) & !1);
    if rc & 0x80000000 == 0 {
        m.put_word(p + 12, lo);
        m.put_word(p + 16, hi);
        m.put_word(out, lo);
        m.put_word(out + 4, hi);
    } else {
        m.put_word(p + 12, 0);
        m.put_word(p + 16, 0);
    }
    rc
}
/// Pair-returning platform interface, avoiding an invented arena stack allocation.
pub trait PairHost: Host {
    fn pair(&mut self, m: &mut Memory<'_>, interface: u32) -> (u32, u32, u32);
}
/// fn_19acf4.
pub fn fn_19acf4(m: &mut Memory<'_>, p: u32, h: &mut impl Host) -> u32 {
    fn_1840fc(m, p + 32, h);
    let mut i = m.word(p + 8);
    let end = i.wrapping_add(m.word(p + 4));
    let mut rc = u32::MAX;
    while i < end {
        if m.byte(m.word(p + 20).wrapping_add(i.wrapping_mul(0xe8)) + 0xe4) == 0 {
            rc = i;
            break;
        }
        i = i.wrapping_add(1);
    }
    fn_18423c(m, p + 32, h);
    rc
}
fn half(m: &Memory<'_>, p: u32) -> u16 {
    u16::from_le_bytes([m.byte(p), m.byte(p + 1)])
}
/// fn_19ad68. Pattern matching remains the external wide-string service fn_1007d0.
pub fn fn_19ad68(
    m: &mut Memory<'_>,
    slot: u32,
    name: u32,
    context: u32,
    cb: u32,
    size: u32,
    mode: u32,
    pattern: u32,
    h: &mut impl Host,
) {
    let mut i = 0;
    loop {
        m.put_half(slot + i * 2, half(m, name + i * 2));
        i += 1;
        if half(m, name + i * 2) == 0 || i >= 95 {
            break;
        }
    }
    m.put_half(slot + i * 2, 0);
    for at in [0xc0, 0xc4, 0xc8, 0xcc, 0xd0, 0xd4] {
        m.put_word(slot + at, 0);
    }
    m.put_word(slot + 0xd8, context);
    m.put_word(slot + 0xdc, cb);
    m.put_word(slot + 0xe0, size);
    put_byte(m, slot + 0xe4, 1);
    put_byte(m, slot + 0xe5, mode as u8);
    let rc = call(m, h, 0x1007d0, &[name, pattern]);
    put_byte(m, slot + 0xe6, u8::from(rc != 0));
    put_byte(m, slot + 0xe7, 0);
}
/// fn_19ae08. Nintendo LZ10/LZ11, including extended size and overlapping matches.
pub fn fn_19ae08(m: &mut Memory<'_>, src: u32, mut dst: u32) {
    let ext = m.byte(src) & 15 != 0;
    let mut left = m.word(src) >> 8;
    let mut p = src + 4;
    if left == 0 {
        left = m.word(p);
        p += 4;
        if left == 0 {
            return;
        }
    }
    loop {
        let flags = m.byte(p);
        p += 1;
        for bit in 0..8 {
            if flags & (128 >> bit) != 0 {
                let b = m.byte(p) as u32;
                let n = b >> 4;
                let (len, disp, used) = if !ext {
                    (n + 3, ((b & 15) << 8) | m.byte(p + 1) as u32, 2)
                } else if n == 1 {
                    (
                        (((b & 15) << 12)
                            | ((m.byte(p + 1) as u32) << 4)
                            | (m.byte(p + 2) as u32 >> 4))
                            + 0x111,
                        ((m.byte(p + 2) as u32 & 15) << 8) | m.byte(p + 3) as u32,
                        4,
                    )
                } else if n == 0 {
                    (
                        (((b & 15) << 4) | (m.byte(p + 1) as u32 >> 4)) + 0x11,
                        ((m.byte(p + 1) as u32 & 15) << 8) | m.byte(p + 2) as u32,
                        3,
                    )
                } else {
                    (n + 1, ((b & 15) << 8) | m.byte(p + 1) as u32, 2)
                };
                p += used;
                let n = len.min(left);
                let mut from = dst.wrapping_sub(disp + 1);
                for _ in 0..n {
                    put_byte(m, dst, m.byte(from));
                    dst += 1;
                    from += 1
                }
                left -= n;
            } else {
                put_byte(m, dst, m.byte(p));
                dst += 1;
                p += 1;
                left -= 1;
            }
            if left == 0 {
                return;
            }
        }
    }
}
/// fn_19dd30.
pub fn fn_19dd30(m: &mut Memory<'_>, it: u32, out: u32) -> u32 {
    let t = m.word(it);
    let e = m.word(t + 4);
    let mut i = m.word(it + 8);
    loop {
        if !(m.word(it + 4) < i && i < m.word(it + 12)) {
            return 0;
        }
        m.put_word(out + 4, i);
        m.put_word(out, t);
        let at = e.wrapping_add(i.wrapping_mul(12));
        put_byte(m, out + 12, u8::from(m.word(at) & 0xff000000 != 0));
        let p = m.word(t + 16).wrapping_add(m.word(at) & 0xffffff);
        m.put_word(out + 8, p);
        if half(m, p) == 46 && half(m, p + 2) == 0 {
            i = i.wrapping_add(1);
            continue;
        }
        let next = if m.word(at) & 0xff000000 != 0 {
            m.word(at + 8)
        } else {
            i.wrapping_add(1)
        };
        m.put_word(it + 8, next);
        return 1;
    }
}
/// fn_19ddd0. The path resolver is an external engine boundary.
pub fn fn_19ddd0(m: &mut Memory<'_>, p: u32, h: &mut impl Host) -> u32 {
    let i = call(m, h, 0x19de08, &[p]);
    let e = m.word(p + 4);
    if (i as i32) >= 0 && m.word(e.wrapping_add(i.wrapping_mul(12))) & 0xff000000 != 0 {
        m.put_word(p + 24, i);
        1
    } else {
        0
    }
}
/// fn_183be4. Repeated reads advance a signed 64-bit file position.
pub fn fn_183be4(
    m: &mut Memory<'_>,
    s: u32,
    out: u32,
    mut src: u32,
    mut left: u32,
    h: &mut impl ReadHost,
) -> u32 {
    let mut total = 0u32;
    while left != 0 {
        let (rc, made) = h.read(m, m.word(s) & !1, m.word(s + 4), m.word(s + 8), src, left);
        if rc & 0x80000000 != 0 {
            return rc;
        }
        total = total.wrapping_add(made);
        let position = ((m.word(s + 8) as u64) << 32) | m.word(s + 4) as u64;
        let position = position.wrapping_add(made as i32 as i64 as u64);
        m.put_word(s + 4, position as u32);
        m.put_word(s + 8, (position >> 32) as u32);
        if made == left || made == 0 {
            break;
        }
        src = src.wrapping_add(made);
        left = left.wrapping_sub(made);
    }
    m.put_word(out, total);
    0
}
pub trait ReadHost: Host {
    fn read(
        &mut self,
        m: &mut Memory<'_>,
        handle: u32,
        lo: u32,
        hi: u32,
        src: u32,
        len: u32,
    ) -> (u32, u32);
}
/// fn_1992f8. Signed quotient/remainder correction around the unsigned divider.
/// The caller supplies the unsigned divider, including its divide-by-zero policy.
pub fn fn_1992f8(n: i64, d: i64, divide: impl FnOnce(u64, u64) -> (u64, u64)) -> (u64, u64) {
    let (mut q, mut r) = divide(n.unsigned_abs(), d.unsigned_abs());
    if (n < 0) != (d < 0) {
        q = q.wrapping_neg()
    }
    if n < 0 {
        r = r.wrapping_neg()
    }
    (q, r)
}

fn put_byte(m: &mut Memory<'_>, p: u32, v: u8) {
    let i = (p - m.base) as usize;
    m.bytes[i] = v;
}

/// fn_17e06c. Apply a property to the main object and its optional child list.
pub fn fn_17e06c(m: &mut Memory<'_>, a: u32, b: u32, h: &mut impl Host) -> u32 {
    let mut rc = call(m, h, 0x17dd30, &[a, m.word(b + 8)]);
    if m.word(b + 16) == 0 {
        return rc;
    }
    let mut i = 0u32;
    while rc < 0x80000000 && i < (half(m, b + 12) as u32) {
        let array = m.word(b + 16);
        rc = call(m, h, 0x17dd30, &[a, m.word(array + i * 4)]);
        i += 1;
    }
    rc
}
/// Handle resolution returns its temporary context by value, not through a fake arena stack.
pub trait ContextHost: Host {
    fn context(&mut self, m: &mut Memory<'_>, handle: [u32; 2]) -> (u32, u32);
}
/// fn_19854c. Validate the sequence handle and create a tagged, heap-owned entry.
pub fn fn_19854c(
    m: &mut Memory<'_>,
    handle: [u32; 2],
    arg: u32,
    out: u32,
    h: &mut impl ContextHost,
) -> u32 {
    if out == 0 || arg == 0 {
        return 0x88402007;
    }
    let (rc, context) = h.context(m, handle);
    if rc >= 0x80000000 {
        return rc;
    }
    let allocation = call(m, h, 0x1963b8, &[m.word(context + 4), 1, 0x124]);
    if allocation == 0 {
        return 0x8840200a;
    }
    m.put_word(allocation, handle[0]);
    m.put_word(allocation + 4, handle[1]);
    m.put_word(allocation + 8, context);
    m.put_word(out, allocation);
    m.put_word(out + 4, 0xeffb);
    rc
}

/// Out-pointer services whose output is local to the current call. The host
/// returns the status and the word written to the local, without guest-stack aliases.
pub trait OutputHost: Host {
    fn output(&mut self, memory: &mut Memory<'_>, function: u32, args: &[u32]) -> (u32, u32);
}
fn failed(rc: u32) -> bool {
    rc >= 0x8000_0000
}
fn hash_text(bytes: &[u8]) -> u32 {
    bytes
        .iter()
        .fold(2166136261u32, |h, &b| (h ^ b as u32).wrapping_mul(16777619))
}
fn hash_guest(m: &Memory<'_>, mut p: u32) -> u32 {
    let mut hash = 2166136261u32;
    while m.byte(p) != 0 {
        hash = (hash ^ m.byte(p) as u32).wrapping_mul(16777619);
        p += 1;
    }
    hash
}
/// A ROM string literal's guest address for a log vararg. The ARM passes
/// the literal's address (its code/data image holds the text); a host that
/// can materialize the text supplies the address, otherwise the literal's
/// hash travels as before. Only fn_1795f0's logs use this.
fn lit(m: &mut Memory<'_>, h: &mut impl Host, real: bool, text: &[u8]) -> u32 {
    if real {
        h.format_address(text).unwrap_or_else(|| hash_text(text))
    } else {
        hash_text(text)
    }
}
/// A guest string's log vararg: the address itself when the host compares
/// raw bytes, else the hash the recorded-reply convention used.
fn guest(m: &Memory<'_>, real: bool, p: u32) -> u32 {
    if real {
        p
    } else {
        hash_guest(m, p)
    }
}
fn log_error(
    m: &mut Memory<'_>,
    h: &mut impl Host,
    logger: u32,
    module: u32,
    code: u32,
    format: &[u8],
    args: &[u32],
) {
    let literal = h.format_address(format).unwrap_or_else(|| hash_text(format));
    call(m, h, 0x173dc0, &[logger, module, code, literal]);
    for chunk in args.chunks(6) {
        call(m, h, 0xffff0001, chunk);
    }
}
fn log_info(
    m: &mut Memory<'_>,
    h: &mut impl Host,
    logger: u32,
    level: u32,
    format: &[u8],
    args: &[u32],
) {
    let literal = h.format_address(format).unwrap_or_else(|| hash_text(format));
    call(
        m,
        h,
        0x1738e8,
        &[logger, 0x8d7668, level, 0, literal],
    );
    for chunk in args.chunks(6) {
        call(m, h, 0xffff0001, chunk);
    }
}
fn lock(m: &mut Memory<'_>, h: &mut impl Host, p: u32, slot: u32) -> u32 {
    if p == 0 {
        return 0x81402007;
    }
    let f = m.word(m.word(m.word(p) + 8) + slot);
    if f == 0 {
        0
    } else {
        call(m, h, f, &[m.word(p + 8)])
    }
}
/// fn_172d18. The six register words passed to SVC 0x22, including the
/// two stack arguments transferred into r4/r5 by the ROM wrapper.
pub fn fn_172d18(m: &mut Memory<'_>, args: [u32; 6], h: &mut impl Host) -> u32 {
    call(m, h, 0xffff0022, &args)
}
/// fn_174f9c. Remove the first callback whose complete three-word key matches.
pub fn fn_174f9c(
    m: &mut Memory<'_>,
    p: u32,
    key: u32,
    a: [u32; 3],
    h: &mut impl OutputHost,
) -> u32 {
    if p == 0 {
        return 0x8b702007;
    }
    let rc = lock(m, h, m.word(p + 4), 8);
    if failed(rc) {
        return rc;
    }
    let mut status = 0;
    let list = if key != 0 && m.byte(key) != 0 {
        let (rc, value) = h.output(m, 0x1966ac, &[m.word(p + 8), key]);
        if failed(rc) {
            status = 0x8b702000 | (rc & 0x1fff);
            0
        } else {
            m.word(value + 8)
        }
    } else {
        m.word(p + 12)
    };
    if !failed(status) {
        status = 0x8b702014;
        if list != 0 {
            let mut index = 0u32;
            loop {
                let (found, entry) = h.output(m, 0x1970ac, &[list, index]);
                if found == 0 {
                    break;
                }
                if [m.word(entry), m.word(entry + 4), m.word(entry + 8)] == a {
                    status = if call(m, h, 0x196e28, &[list, index]) == 0 {
                        0x8b702100
                    } else {
                        0
                    };
                    break;
                }
                index += 1;
            }
        }
    }
    let rc = lock(m, h, m.word(p + 4), 12);
    if failed(rc) && !failed(status) {
        status = rc;
    }
    status
}
fn destroy_mutex(m: &mut Memory<'_>, h: &mut impl Host, p: u32) -> u32 {
    if p == 0 {
        return 0x81402007;
    }
    let f = m.word(m.word(m.word(p) + 8) + 4);
    let rc = if f == 0 {
        0
    } else {
        call(m, h, f, &[m.word(p + 8)])
    };
    if !failed(rc) {
        call(m, h, 0x196428, &[m.word(p + 4), p]);
    }
    rc
}
fn release_config(m: &mut Memory<'_>, h: &mut impl Host, heap: u32, p: u32) {
    if p == 0 {
        return;
    }
    if m.word(p + 8) != 0 {
        call(m, h, 0x196428, &[heap, m.word(p + 8)]);
    }
    if m.word(p + 16) != 0 {
        for i in 0..m.half(p + 12) as u32 {
            call(m, h, 0x196428, &[heap, m.word(m.word(p + 16) + i * 4)]);
        }
        call(m, h, 0x196428, &[heap, m.word(p + 16)]);
    }
    if m.word(p + 20) != 0 {
        call(m, h, 0x196428, &[heap, m.word(p + 20)]);
    }
    call(m, h, 0x196428, &[heap, p]);
}
fn release_dictionary(m: &mut Memory<'_>, h: &mut impl Host, p: u32) -> u32 {
    if let Some(rc)=h.release_dictionary(m,p) {return rc;}
    if p == 0 {
        return 0x89002007;
    }
    call(m, h, 0x19869c, &[p + 24]);
    call(m, h, 0x196428, &[m.word(p + 20), p]);
    0
}
/// fn_177558. Release all broker-owned resources, retaining the first failure.
pub fn fn_177558(m: &mut Memory<'_>, p: u32, h: &mut impl Host) -> u32 {
    if p == 0 {
        return 0;
    }
    let context = m.word(p + 8);
    let mut status = 0;
    for off in [20, 16, 12, 28] {
        let resource = m.word(p + off);
        if resource != 0 {
            let rc = if off == 20 || off == 16 {
                release_dictionary(m, h, resource)
            } else {
                destroy_mutex(m, h, resource)
            };
            m.put_word(p + off, 0);
            if failed(rc) && !failed(status) {
                status = rc;
            }
        }
    }
    if m.word(p + 24) != 0 {
        release_config(m, h, m.word(context + 4), m.word(p + 24));
        m.put_word(p + 24, 0);
    }
    call(m, h, 0x196428, &[m.word(context + 4), p]);
    status
}
/// fn_17ca94. Destroy a resource archive and all caller-owned entry allocations.
pub fn fn_17ca94(m: &mut Memory<'_>, p: u32, h: &mut impl Host) -> u32 {
    if p == 0 {
        return 0x8c102008;
    }
    let stream = m.word(p + 52);
    if stream != 0 {
        let vt = m.word(stream);
        call(
            m,
            h,
            m.word(vt + 20),
            &[stream.wrapping_sub(m.word(vt + 28))],
        );
    }
    let interface = m.word(p + 48);
    if interface != 0 {
        call(m, h, m.word(m.word(interface) + 16), &[interface]);
    }
    if m.word(p + 64) != 0 {
        call(m, h, 0x196428, &[m.word(m.word(p) + 4), m.word(p + 64)]);
    }
    if m.word(p + 4) != 0 {
        let mut i = 0;
        while i < m.word(p + 8) {
            let entry = m.word(p + 4) + i * 24;
            for off in [0, 4, 16] {
                if m.word(entry + off) != 0 {
                    call(
                        m,
                        h,
                        0x196428,
                        &[m.word(m.word(p) + 4), m.word(entry + off)],
                    );
                }
            }
            i += 1;
        }
        call(m, h, 0x196428, &[m.word(m.word(p) + 4), m.word(p + 4)]);
    }
    call(m, h, 0x196428, &[m.word(m.word(p) + 4), p]);
    0
}
fn upper(c: u8) -> u8 {
    if c.is_ascii_lowercase() {
        c - 32
    } else {
        c
    }
}
/// fn_17ab44. Normalize the archive member spelling and increment its 16-bit refcount.
pub fn fn_17ab44(
    m: &mut Memory<'_>,
    logger: u32,
    broker: u32,
    name: u32,
    flags: u32,
    out: u32,
    h: &mut impl Host,
) -> u32 {
    let archive = m.word(broker + 8);
    let mut clean = [0u8; 132];
    let mut length = 0usize;
    while m.byte(name + length as u32) != 0 {
        length += 1;
    }
    if length < 128 {
        for (i, dst) in clean[..length].iter_mut().enumerate() {
            let c = m.byte(name + i as u32);
            *dst = if c.is_ascii_alphanumeric() {
                c
            } else if c == b'/' || c == b'_' {
                b'_'
            } else {
                b'-'
            };
        }
    }
    for i in 0..m.word(archive + 8) {
        let p = m.word(m.word(archive + 4) + i * 24);
        let mut j = 0;
        while upper(clean[j]) == upper(m.byte(p + j as u32)) {
            if clean[j] == 0 {
                m.put_word(out + 4, archive);
                m.put_word(out + 8, i);
                m.put_half(archive + 60, m.half(archive + 60).wrapping_add(1));
                return 0;
            }
            j += 1;
        }
    }
    if flags & 1 != 0 {
        log_info(
            m,
            h,
            logger,
            3,
            b"brk_DataOpen : SearchClcArchive failed: %s:%s",
            &[hash_guest(m, m.word(broker + 4)), hash_guest(m, name)],
        );
    } else {
        log_error(
            m,
            h,
            logger,
            0x8d7668,
            0x101,
            b"%s%s%s%s",
            &[
                hash_text(b"archive"),
                hash_guest(m, m.word(broker + 4)),
                hash_text(b"data"),
                hash_guest(m, name),
            ],
        );
    }
    0x8150600d
}
/// fn_17ef1c. Find the first advertised MIME prefix with an end or semicolon boundary.
pub fn fn_17ef1c(m: &mut Memory<'_>, p: u32, name: u32, out: u32, h: &mut impl OutputHost) -> u32 {
    if p == 0 || name == 0 || m.byte(name) == 0 {
        return 0x81202007;
    }
    m.put_word(out, 0);
    let mut i = 0u16;
    while i < m.half(p + 12) {
        let component = m.word(m.word(p + 8) + i as u32 * 4);
        let method = m.word(m.word(component + 8) + 28);
        if method != 0 {
            let (rc, list) = h.output(m, method, &[m.word(component + 12), m.word(component + 16)]);
            if !failed(rc) && list != 0 {
                let mut j = 0u16;
                while m.word(out) == 0 && m.word(list + j as u32 * 4) != 0 {
                    let entry = m.word(list + j as u32 * 4);
                    let mut n = 0;
                    while m.byte(entry + n) != 0 {
                        n += 1;
                    }
                    let mut same = true;
                    for k in 0..n {
                        if m.byte(name + k) != m.byte(entry + k) {
                            same = false;
                            break;
                        }
                    }
                    if same && (m.byte(name + n) == 0 || m.byte(name + n) == b';') {
                        m.put_word(out, component);
                    }
                    j = j.wrapping_add(1);
                }
            }
        }
        i = i.wrapping_add(1);
        if m.word(out) != 0 {
            break;
        }
    }
    if m.word(out) == 0 {
        0x81202014
    } else {
        0
    }
}
/// fn_180a44. Dispatch unload through TtsCompVt +0x24, not its load slot.
pub fn fn_180a44(m: &mut Memory<'_>, p: u32, a: u32, b: u32, h: &mut impl Host) -> u32 {
    if p == 0 {
        return 0x81202007;
    }
    let context = m.word(p);
    let mut rc = 0x81202008;
    let mut i = 0u16;
    while rc & 0x1fff == 8 && i < m.half(p + 12) {
        let component = m.word(m.word(p + 8) + i as u32 * 4);
        let method = m.word(m.word(component + 8) + 36);
        if method != 0 {
            rc = call(
                m,
                h,
                method,
                &[m.word(component + 12), m.word(component + 16), a, b],
            );
            if !failed(rc) {
                return rc;
            }
        }
        i = i.wrapping_add(1);
    }
    if failed(rc) {
        log_error(
            m,
            h,
            m.word(context + 16),
            0x8d7994,
            0x3ea9,
            b"%s%p%s%x",
            &[hash_text(b"handle"), a, hash_text(b"lhError"), rc],
        );
    }
    rc
}

/// fn_17dacc. The caller maps the regional builtin table and its strings into
/// the arena. Callback and module words are preserved without interpretation.
pub fn fn_17dacc(m: &mut Memory<'_>, key: u32, out: [u32; 3], table: u32) -> u32 {
    for p in out {
        m.put_word(p, 0);
    }
    let mut entry = table;
    while m.word(entry) != 0 {
        let mut a = key;
        let mut b = m.word(entry);
        while m.byte(a) != 0 && m.byte(a) == m.byte(b) {
            a += 1;
            b += 1;
        }
        if m.byte(a) == m.byte(b) {
            break;
        }
        entry += 16;
    }
    if m.word(entry) == 0 {
        return 0x8150600d;
    }
    if m.word(entry + 4) != 0 {
        m.put_word(out[0], m.word(entry + 4));
    } else {
        m.put_word(out[1], m.word(entry + 8));
        m.put_word(out[2], m.word(entry + 12));
    }
    0
}
/// fn_17c8fc. Allocator entry words are regional guest code addresses supplied
/// by the caller, not callable host pointers.
pub fn fn_17c8fc(
    m: &mut Memory<'_>,
    owner: [u32; 2],
    name: u32,
    out: u32,
    allocator_entries: [u32; 4],
    h: &mut impl OutputHost,
) -> u32 {
    if out == 0 {
        return 0x8c102007;
    }
    let (rc, context) = h.output(m, 0x19611c, &owner);
    if failed(rc) {
        return rc;
    }
    m.put_word(out, 0);
    let object = call(m, h, 0x1963b8, &[m.word(context + 4), 1, 68]);
    if object == 0 {
        return 0x8c10200a;
    }
    let mut len = 0u32;
    while m.byte(name + len) != 0 {
        len += 1;
    }
    let text = call(m, h, 0x196394, &[m.word(context + 4), len + 1]);
    m.put_word(object + 64, text);
    if text == 0 {
        return 0x8c10200a;
    }
    for i in 0..=len {
        m.put_byte(text + i, m.byte(name + i));
    }
    for (i, f) in allocator_entries.into_iter().enumerate() {
        m.put_word(object + 32 + i as u32 * 4, f);
    }
    let rc = call(
        m,
        h,
        0x1b20a4,
        &[object + 12, object + 32, m.word(context + 4)],
    );
    if rc != 0 {
        return 0x8c102004;
    }
    m.put_word(object, context);
    m.put_half(object + 60, 0);
    m.put_word(out, object);
    0
}
/// fn_179374. Map a broker resource or delegate the stream-backed operation.
/// A direct cached mapping does not acquire the broker lock.
pub fn fn_179374(
    m: &mut Memory<'_>,
    broker: u32,
    handle: u32,
    offset: u32,
    size: u32,
    out: u32,
    h: &mut impl Host,
) -> u32 {
    if broker == 0 || handle == 0 || size == 0 || out == 0 {
        return 0x81506007;
    }
    m.put_word(out, 0);
    log_info(
        m,
        h,
        m.word(broker),
        4,
        b"brk_DataMap : hData=%p Map Offset=%d Size=%d Data=%p : Begin",
        &[handle, offset, m.word(size), 0],
    );
    let mut rc = 0u32;
    let data = m.word(handle + 4);
    let mut cached = false;
    if data != 0 {
        let record = m.word(data + 4) + m.word(handle + 8) * 24;
        let total = m.word(record + 8);
        if offset > total {
            rc = 0x81506004;
        } else {
            if m.word(size) == 0 {
                m.put_word(size, total);
            }
            if total < m.word(size).wrapping_add(offset) {
                m.put_word(size, total.wrapping_sub(offset));
            }
            let mapped = m.word(record + 4);
            if mapped != 0 {
                m.put_word(out, mapped.wrapping_add(offset));
                cached = true;
            } else {
                rc = lock(m, h, m.word(m.word(broker + 4) + 28), 8);
                if !failed(rc) {
                    rc = call(
                        m,
                        h,
                        0x17c9d8,
                        &[data, m.word(handle + 8), offset, m.word(size), out],
                    );
                    if !failed(rc) {
                        rc = lock(m, h, m.word(m.word(broker + 4) + 28), 12);
                    }
                }
            }
        }
    } else {
        rc = call(m, h, 0x1731c4, &[m.word(handle), offset, size, out]);
    }
    if failed(rc) && !cached {
        let payload = [
            hash_text(b"hData"),
            handle,
            hash_text(b"offset"),
            offset,
            hash_text(b"size"),
            m.word(size),
            hash_text(b"lhError"),
            rc,
        ];
        log_error(
            m,
            h,
            m.word(broker),
            0x8d7668,
            0x11f,
            b"%s%p%s%u%s%u%s%x",
            &payload,
        );
    }
    log_info(
        m,
        h,
        m.word(broker),
        4,
        b"brk_DataMap : End (%x, %p, %u)",
        &[rc, m.word(out), m.word(size)],
    );
    rc
}

/// A vtable load method writes a handle pair local to the current call.
pub trait LoadHost: OutputHost {
    fn load(&mut self, memory: &mut Memory<'_>, method: u32, args: &[u32]) -> (u32, [u32; 2]);
}
/// fn_17f03c. Scratch is one caller-owned word reserved for local component
/// lookup. It is restored before invoking load or logging. null_pair is the
/// regional D_8d7660 value, mapped by the caller.
pub fn fn_17f03c(
    m: &mut Memory<'_>,
    p: u32,
    quiet: u32,
    name: u32,
    uri: u32,
    blob: [u32; 2],
    blob2: [u32; 2],
    out: u32,
    null_pair: [u32; 2],
    scratch: u32,
    h: &mut impl LoadHost,
) -> u32 {
    if p == 0 || name == 0 {
        return 0x81202007;
    }
    let context = m.word(p);
    if out != 0 {
        m.put_word(out, null_pair[0]);
        m.put_word(out + 4, null_pair[1]);
    }
    let saved = m.word(scratch);
    let mut rc = fn_17ef1c(m, p, name, scratch, h);
    let component = m.word(scratch);
    m.put_word(scratch, saved);
    let uri_hash = if uri == 0 {
        hash_text(b"")
    } else {
        hash_guest(m, uri)
    };
    if failed(rc) {
        log_error(
            m,
            h,
            m.word(context + 16),
            0x8d7994,
            0x3ecd,
            b"%s%s%s%s",
            &[
                hash_text(b"uri"),
                uri_hash,
                hash_text(b"contentType"),
                hash_guest(m, name),
            ],
        );
        return rc;
    }
    let method = m.word(m.word(component + 8) + 32);
    if method == 0 {
        log_error(
            m,
            h,
            m.word(context + 16),
            0x8d7994,
            0x3ea7,
            b"%s%s%s%s%s%s",
            &[
                hash_text(b"component"),
                hash_guest(m, m.word(component + 4)),
                hash_text(b"uri"),
                uri_hash,
                hash_text(b"contentType"),
                hash_guest(m, name),
            ],
        );
        return 0x81202007;
    }
    // A null URI is the empty string in the ROM. The caller's host receives
    // zero as the explicit empty-URI token rather than a host/static pointer.
    let (status, result) = h.load(
        m,
        method,
        &[
            m.word(component + 12),
            m.word(component + 16),
            name,
            uri,
            blob[0],
            blob[1],
            blob2[0],
            blob2[1],
        ],
    );
    rc = status;
    if !failed(rc) {
        if out != 0 {
            m.put_word(out, result[0]);
            m.put_word(out + 4, result[1]);
        }
    } else if quiet == 0 || rc & 0x1fff != 0x40c {
        let payload = [
            hash_text(b"component"),
            hash_guest(m, m.word(component + 4)),
            hash_text(b"uri"),
            uri_hash,
            hash_text(b"contentType"),
            hash_guest(m, name),
            hash_text(b"lhError"),
            rc,
        ];
        log_error(
            m,
            h,
            m.word(context + 16),
            0x8d7994,
            0x3ea6,
            b"%s%s%s%s%s%s%s%x",
            &payload,
        );
    }
    rc
}
/// fn_180b24. Close registered objects in reverse collection order, then
/// remove their registrations in a second reverse traversal.
pub fn fn_180b24(
    m: &mut Memory<'_>,
    owner: [u32; 2],
    collection: u32,
    h: &mut impl OutputHost,
) -> u32 {
    let (mut rc, context) = h.output(m, 0x19611c, &owner);
    if failed(rc) {
        return rc;
    }
    for mode in 1..=2 {
        let mut n = call(m, h, 0x1970a0, &[collection]);
        while n != 0 {
            let (found, cur) = h.output(m, 0x1970ac, &[collection, n - 1]);
            if found == 0 {
                break;
            }
            let name = m.word(cur);
            let (status, entry) = h.output(m, 0x17b058, &[m.word(context + 24), name]);
            if failed(status) {
                n -= 1;
                continue;
            }
            call(m, h, 0x17b1c4, &[m.word(context + 24), name]);
            if mode == 1 {
                let object = m.word(entry + 4);
                let r = call(
                    m,
                    h,
                    m.word(object + 20),
                    &[m.word(entry + 8), m.word(entry + 12)],
                );
                if failed(r) {
                    log_error(
                        m,
                        h,
                        m.word(context + 16),
                        0x8d7994,
                        0x3ee4,
                        b"%s%x%s%s",
                        &[
                            hash_text(b"lherror"),
                            rc,
                            hash_text(b"object"),
                            hash_guest(m, name),
                        ],
                    );
                    rc = r;
                } else {
                    let r = call(m, h, 0x178530, &[m.word(context + 12), object]);
                    if failed(r) {
                        rc = r;
                    }
                }
            } else {
                let r = call(m, h, 0x17b2b0, &[m.word(context + 24), name]);
                if failed(r) {
                    log_error(
                        m,
                        h,
                        m.word(context + 16),
                        0x8d7994,
                        0x3ee5,
                        b"%s%x%s%s",
                        &[
                            hash_text(b"lherror"),
                            rc,
                            hash_text(b"object"),
                            hash_guest(m, name),
                        ],
                    );
                    rc = r;
                }
            }
            n -= 1;
        }
    }
    rc
}

/// fn_173fb4. Varargs, the 20-byte params area and SECURE literal are
/// caller-mapped guest storage; formatting and broadcast execute the real ports.
pub fn fn_173fb4(
    m: &mut Memory<'_>,
    logger: u32,
    code: u32,
    format: u32,
    varargs: u32,
    params: u32,
    secure_name: u32,
    h: &mut impl FormatHost,
) {
    if logger != 0 {
        fn_173df4(m, logger, code, format, varargs, params, secure_name, h);
    }
}
/// fn_178510. Quiet-mode-zero interface query wrapper.
pub fn fn_178510(
    m: &mut Memory<'_>,
    broker: u32,
    name: u32,
    method: u32,
    handle_out: u32,
    interface_out: u32,
    h: &mut impl Host,
) -> u32 {
    call(
        m,
        h,
        0x177e0c,
        &[broker, name, method, 0, handle_out, interface_out],
    )
}

/// Dictionary operations with stack-local result/record storage represented
/// by values. Key bytes are explicit, not host pointers into guest code.
pub trait PropertyHost: OutputHost {
    fn property(&mut self, memory: &mut Memory<'_>, dictionary: u32, key: &[u8]) -> (u32, u32);
    fn insert(
        &mut self,
        memory: &mut Memory<'_>,
        dictionary: u32,
        key: u32,
        entry: [u32; 3],
    ) -> u32;
}
struct ReleaseHost<'a, H>(&'a mut H);
impl<H: Host> Host for ReleaseHost<'_, H> {
    fn call(&mut self, m: &mut Memory<'_>, f: u32, a: &[u32]) -> u32 {
        if f == 0x19665c {
            release_dictionary(m, self.0, a[0])
        } else {
            self.0.call(m, f, a)
        }
    }
    fn thread_id(&mut self) -> u32 {
        self.0.thread_id()
    }
}
/// fn_177124. Add a decoded add-on, transferring the record through the
/// dictionary boundary and releasing the resource on every owning path.
pub fn fn_177124(
    m: &mut Memory<'_>,
    source: u32,
    resource: u32,
    broker: u32,
    h: &mut impl PropertyHost,
) -> u32 {
    let (status, property) = h.property(m, resource, b"BROKERSTRING");
    if failed(status) {
        fn_198280(m, resource, &mut ReleaseHost(h));
        return 0x81506014;
    }
    let name = if property != 0 { m.word(property) } else { 0 };
    let context = m.word(broker + 8);
    if resource == 0 {
        return 0x81506007;
    }
    log_info(
        m,
        h,
        m.word(context + 16),
        4,
        b"brkhlp_AddNewAddOn: detect= %s",
        &[if name == 0 { 0 } else { hash_guest(m, name) }],
    );
    let mut rc = lock(m, h, m.word(broker + 28), 8);
    if !failed(rc) {
        rc = h.insert(m, m.word(broker + 16), name, [resource, name, 0]);
        if failed(rc) {
            log_error(
                m,
                h,
                m.word(context + 16),
                0x8d7668,
                0x121,
                b"%s%s",
                &[
                    hash_text(b"file"),
                    if source == 0 {
                        0
                    } else {
                        hash_guest(m, source)
                    },
                ],
            );
        }
    }
    let status = lock(m, h, m.word(broker + 28), 12);
    if failed(status) && !failed(rc) {
        rc = status;
    }
    fn_198280(m, resource, &mut ReleaseHost(h));
    if rc & 0x1fff == 2 {
        0
    } else {
        rc
    }
}
/// A decoded resource operation returns the two local out-pointer values.
pub trait ResourceHost: PropertyHost {
    fn decode(&mut self, memory: &mut Memory<'_>, args: [u32; 4]) -> (u32, u32, u32);
}
/// fn_17d168. source_label points to caller-mapped "BINBROKER", the ROM's
/// stack-local source label used only when add-on registration logs an error.
pub fn fn_17d168(
    m: &mut Memory<'_>,
    context: u32,
    object: u32,
    tag: u32,
    source_label: u32,
    h: &mut impl ResourceHost,
) -> u32 {
    if tag == 0 || object == 0 {
        return 0x81506006;
    }
    let mut p = tag;
    if !equal(m, tag, b"<?xml") {
        while m.byte(p) != 0 && m.byte(p) != b'\n' && m.byte(p) != b'\r' {
            p += 1;
        }
        if m.byte(p) == 0 {
            p = tag;
        } else {
            while m.byte(p) == b'\n' || m.byte(p) == b'\r' {
                p += 1;
            }
        }
    }
    let (mut rc, resource, readers) = h.decode(m, [m.word(object), m.word(object + 4), 0, p]);
    if failed(rc) {
        call(m, h, 0x173dc0, &[m.word(context + 16), 0x8d7668, 0x114, 0]);
        return rc;
    }
    if readers != 0 {
        let mut index = 0;
        loop {
            let (found, entry) = h.output(m, 0x1970ac, &[readers, index]);
            if found == 0 {
                break;
            }
            if !failed(rc) {
                rc = fn_177124(m, source_label, m.word(entry), object, h);
            } else {
                release_dictionary(m, h, m.word(entry));
            }
            index += 1;
        }
        call(m, h, 0x196c7c, &[readers]);
    } else {
        rc = fn_177124(m, source_label, resource, object, h);
    }
    if !failed(rc) {
        rc = lock(m, h, m.word(object + 28), 8);
        if !failed(rc) {
            if call(m, h, 0x196764, &[m.word(object + 16)]) == 0 {
                rc = 0x8150610a;
            }
            let status = lock(m, h, m.word(object + 28), 12);
            if failed(status) && !failed(rc) {
                rc = status;
            }
        }
    }
    rc
}

/// Broker constructor services not yet present in Rust. Local out-parameters
/// are returned by value, while the object fields remain guest addresses.
pub trait ConstructorHost: ResourceHost + DirectoryHost {
    fn context_pair(&mut self, m: &mut Memory<'_>, owner: [u32; 2]) -> (u32, u32);
    fn construct_output(&mut self, m: &mut Memory<'_>, f: u32, args: &[u32], out: u32) -> u32;
}
struct ConstructorCalls<'a, H>(&'a mut H, u32);
impl<H: ConstructorHost> Host for ConstructorCalls<'_, H> {
    fn call(&mut self, m: &mut Memory<'_>, f: u32, a: &[u32]) -> u32 {
        if f == 0x17dd30 {
            fn_17dd30(m, a[0], a[1], self.1, self.0)
        } else {
            self.0.call(m, f, a)
        }
    }
    fn thread_id(&mut self) -> u32 {
        self.0.thread_id()
    }
}
fn copy_string(m: &mut Memory<'_>, h: &mut impl Host, heap: u32, src: u32) -> u32 {
    if src == 0 {
        return 0;
    }
    let mut n = 0;
    while m.byte(src + n) != 0 {
        n += 1;
    }
    let dst = call(m, h, 0x1963b8, &[heap, 1, n + 1]);
    if dst != 0 {
        for i in 0..=n {
            let b = m.byte(src + i);
            put_byte(m, dst + i, b);
        }
    }
    dst
}
fn copy_config(m: &mut Memory<'_>, h: &mut impl Host, config: u32, heap: u32) -> u32 {
    let p = call(m, h, 0x1963b8, &[heap, 1, 24]);
    if p == 0 {
        return 0;
    }
    let ty = half(m, config);
    m.put_half(p, ty);
    m.put_word(p + 4, 0);
    let name = copy_string(m, h, heap, m.word(config + 8));
    m.put_word(p + 8, name);
    if m.word(config + 8) != 0 && name == 0 {
        release_config(m, h, heap, p);
        return 0;
    }
    m.put_half(p + 12, 0);
    let count = half(m, config + 12) as u32;
    let items = if count == 0 {
        0
    } else {
        call(m, h, 0x1963b8, &[heap, count, 4])
    };
    m.put_word(p + 16, items);
    if count != 0 && items == 0 {
        release_config(m, h, heap, p);
        return 0;
    }
    for i in 0..count {
        let text = copy_string(m, h, heap, m.word(m.word(config + 16) + i * 4));
        m.put_word(m.word(p + 16) + i * 4, text);
        if text == 0 {
            release_config(m, h, heap, p);
            return 0;
        }
        let n = half(m, p + 12);
        m.put_half(p + 12, n.wrapping_add(1));
    }
    let tag = copy_string(m, h, heap, m.word(config + 20));
    m.put_word(p + 20, tag);
    if m.word(config + 20) != 0 && tag == 0 {
        release_config(m, h, heap, p);
        return 0;
    }
    p
}
/// fn_177624. module points at the regional D_8d7668 structure, and
/// source_label is caller-mapped BINBROKER for the real fn_17d168 callee.
pub fn fn_177624(
    m: &mut Memory<'_>,
    owner: [u32; 2],
    config: u32,
    _flags: u32,
    out: u32,
    module: u32,
    source_label: u32,
    star: u32,
    h: &mut impl ConstructorHost,
) -> u32 {
    let ty = half(m, config);
    if ty != 2 && ty != 3 {
        return 0x81506001;
    }
    let (mut rc, context) = h.context_pair(m, owner);
    if failed(rc) {
        return rc;
    }
    log_info(m, h, m.word(context + 16), 4, b"brk_ClassOpen: Begin", &[]);
    let heap = m.word(context + 4);
    let p = call(m, h, 0x1963b8, &[heap, 1, 32]);
    if p == 0 {
        return 0x8150600a;
    }
    m.put_word(p, owner[0]);
    m.put_word(p + 4, owner[1]);
    m.put_word(p + 8, context);
    let data = copy_config(m, h, config, heap);
    m.put_word(p + 24, data);
    if data == 0 {
        rc = 0x8150600a;
    } else {
        rc = h.construct_output(m, 0x176e40, &[m.word(context + 8), heap], p + 12);
        if !failed(rc) {
            let a = [
                heap,
                0,
                m.word(module + 68),
                m.word(module + 72),
                m.word(module + 76),
                m.word(module + 80),
                m.word(module + 84),
            ];
            rc = h.construct_output(m, 0x1964e0, &a, p + 16);
        }
        if !failed(rc) {
            let a = [
                heap,
                0,
                m.word(module + 88),
                m.word(module + 92),
                m.word(module + 96),
                m.word(module + 100),
                m.word(module + 104),
            ];
            rc = h.construct_output(m, 0x1964e0, &a, p + 20);
        }
        if !failed(rc) {
            rc = h.construct_output(m, 0x176e40, &[m.word(context + 8), heap], p + 28);
        }
        if !failed(rc) {
            rc = match half(m, config) {
                2 => fn_17e06c(m, p, config, &mut ConstructorCalls(h, star)),
                3 => fn_17d168(m, context, p, m.word(config + 20), source_label, h),
                _ => 0x81506001,
            };
        }
    }
    if failed(rc) {
        for off in [20, 16, 12, 28] {
            let v = m.word(p + off);
            if v != 0 {
                if off == 20 || off == 16 {
                    release_dictionary(m, h, v);
                } else {
                    destroy_mutex(m, h, v);
                }
                m.put_word(p + off, 0);
            }
        }
        if m.word(p + 24) != 0 {
            release_config(m, h, m.word(context + 4), m.word(p + 24));
            m.put_word(p + 24, 0);
        }
        call(m, h, 0x196428, &[m.word(context + 4), p]);
    } else {
        m.put_word(out, p);
    }
    log_info(
        m,
        h,
        m.word(context + 16),
        4,
        b"brk_ClassOpen: End %x",
        &[rc],
    );
    rc
}

/// Interface-query services without a boot Rust implementation. The method
/// call writes its output directly into the guest record, just as in ARM.
pub trait QueryHost: PropertyHost {
    fn query_output(&mut self, m: &mut Memory<'_>, f: u32, args: &[u32]) -> (u32, u32);
    fn insert_record(&mut self, m: &mut Memory<'_>, db: u32, key: u32, record: [u32; 4]) -> u32;
    fn open_pair(&mut self, m: &mut Memory<'_>, f: u32, owner: [u32; 2], out: u32) -> u32;
}
fn query_error(
    m: &mut Memory<'_>,
    h: &mut impl Host,
    logger: u32,
    code: u32,
    name: u32,
    rc: u32,
    module: bool,
) {
    log_error(
        m,
        h,
        logger,
        0x8d7668,
        code,
        b"%s%s%s%x",
        &[
            hash_text(if module { b"module" } else { b"BrokerString" }),
            hash_guest(m, name),
            hash_text(if module { b"lherror" } else { b"lhError" }),
            rc,
        ],
    );
}
/// fn_177e0c. scratch contains three guest words for fn_17dacc's output
/// pointers. The caller maps the regional builtin table into its arena.
pub fn fn_177e0c(
    m: &mut Memory<'_>,
    broker: u32,
    name: u32,
    method: u32,
    quiet: u32,
    handle_out: u32,
    interface_out: u32,
    table: u32,
    scratch: u32,
    h: &mut impl QueryHost,
) -> u32 {
    if broker == 0 || name == 0 || interface_out == 0 {
        return 0x81506007;
    }
    let context = m.word(broker + 4);
    let runtime = m.word(context + 8);
    if handle_out != 0 {
        m.put_word(handle_out, 0);
        m.put_word(handle_out + 4, 0);
    }
    m.put_word(interface_out, 0);
    log_info(
        m,
        h,
        m.word(broker),
        4,
        b"brk_InterfaceQuery BrokerString=%s : Begin",
        &[hash_guest(m, name)],
    );
    let mut rc = lock(m, h, m.word(context + 12), 8);
    if failed(rc) {
        return rc;
    }
    let (status, mut record) = h.output(m, 0x1966ac, &[m.word(context + 20), name]);
    let mut success = false;
    if !failed(status) {
        if m.word(record + 4) != 0 {
            success = true;
        } else {
            rc = 0x8150600d;
            if quiet == 0 {
                query_error(m, h, m.word(broker), 0x103, name, rc, false);
            }
        }
    } else {
        rc = h.insert_record(m, m.word(context + 20), name, [0; 4]);
        if !failed(rc) {
            let result = h.output(m, 0x1966ac, &[m.word(context + 20), name]);
            rc = result.0;
            record = result.1;
        }
        if failed(rc) {
            log_error(
                m,
                h,
                m.word(broker),
                0x8d7668,
                200,
                b"%s%s",
                &[hash_text(b"module"), hash_guest(m, name)],
            );
        } else {
            let (entry_rc, entry) = h.query_output(m, 0x1772b0, &[broker, name]);
            let mut callback = 0;
            let mut class_id = 0;
            let mut class_count = 0;
            let mut builtin = failed(entry_rc);
            let mut ready = true;
            if !builtin {
                let (r, v) = h.property(m, m.word(entry), b"FULLPATHCOMPONENT");
                if failed(r) {
                    rc = 0x81506014;
                    if quiet == 0 {
                        query_error(m, h, m.word(broker), 0x104, name, rc, false);
                    }
                    builtin = true;
                } else {
                    if v != 0 {
                        class_id = m.word(v);
                    }
                    let (r, v) = h.property(m, m.word(entry), b"GETINTERFACE");
                    if failed(r) {
                        rc = 0x81506014;
                        ready = false;
                        if quiet == 0 {
                            query_error(m, h, m.word(broker), 0x106, name, rc, false);
                        }
                    } else {
                        rc = 0;
                        if v != 0 {
                            class_count = m.word(v);
                        }
                    }
                }
            }
            if builtin {
                let saved = [m.word(scratch), m.word(scratch + 4), m.word(scratch + 8)];
                for off in [0, 4, 8] {
                    m.put_word(scratch + off, 0);
                }
                rc = fn_17dacc(m, name, [scratch, scratch + 4, scratch + 8], table);
                callback = m.word(scratch);
                class_id = m.word(scratch + 4);
                class_count = m.word(scratch + 8);
                for (i, v) in saved.iter().enumerate() {
                    m.put_word(scratch + i as u32 * 4, *v);
                }
                if failed(rc) {
                    ready = false;
                    if quiet == 0 {
                        query_error(m, h, m.word(broker), 0x105, name, rc, false);
                    }
                }
            }
            if ready && callback == 0 {
                log_info(
                    m,
                    h,
                    m.word(broker),
                    3,
                    b"brk_InterfaceQuery : osspi_DynModOpen %s, %d",
                    &[hash_guest(m, class_id), 0],
                );
                let (r, obj) = h.query_output(
                    m,
                    0x172eac,
                    &[m.word(runtime + 28), m.word(runtime + 4), class_id, 0],
                );
                rc = r;
                m.put_word(record, obj);
                if failed(rc) {
                    ready = false;
                    if quiet == 0 {
                        query_error(m, h, m.word(broker), 0x107, name, rc, false);
                    }
                }
                if ready {
                    log_info(
                        m,
                        h,
                        m.word(broker),
                        3,
                        b"brk_InterfaceQuery : osspi_DynModObjLoad %s",
                        &[hash_guest(m, class_count)],
                    );
                    let (r, cb) = h.query_output(m, 0x172fac, &[m.word(record), class_count]);
                    rc = r;
                    callback = cb;
                    if failed(rc) {
                        ready = false;
                        if quiet == 0 {
                            query_error(m, h, m.word(broker), 0x108, name, rc, false);
                        }
                    }
                }
            }
            if ready && callback != 0 {
                let (r, itf) = h.query_output(m, callback, &[method]);
                rc = r;
                m.put_word(record + 4, itf);
                if failed(rc) {
                    ready = false;
                    if quiet == 0 {
                        query_error(m, h, m.word(broker), 0x109, name, rc, true);
                    }
                }
            }
            if ready && m.word(m.word(record + 4) + 8) != 0 {
                rc = h.open_pair(
                    m,
                    m.word(m.word(record + 4) + 8),
                    [m.word(context), m.word(context + 4)],
                    record + 8,
                );
                if failed(rc) {
                    ready = false;
                    if quiet == 0 {
                        query_error(m, h, m.word(broker), 0x10a, name, rc, true);
                    }
                }
            }
            success = ready;
        }
    }
    if success {
        m.put_word(interface_out, m.word(record + 4));
        if handle_out != 0 {
            m.put_word(handle_out, m.word(record + 8));
            m.put_word(handle_out + 4, m.word(record + 12));
        }
    } else {
        log_info(
            m,
            h,
            m.word(broker),
            3,
            b"brk_InterfaceQuery : BrokerString=%s: Module Not Found (%x)",
            &[hash_guest(m, name), rc],
        );
    }
    let status = lock(m, h, m.word(context + 12), 12);
    if failed(status) && !failed(rc) {
        rc = status;
    }
    log_info(
        m,
        h,
        m.word(broker),
        4,
        b"brk_InterfaceQuery BrokerString=%s : End",
        &[hash_guest(m, name)],
    );
    rc
}

/// RVCT formatting with caller-owned guest arguments and explicit locale hooks.
pub trait FormatHost: Host {
    fn rvct_vsnprintf(
        &mut self,
        m: &mut Memory<'_>,
        dst: u32,
        size: u32,
        fmt: u32,
        args: u32,
    ) -> i32 {
        struct Locale<'a, H: ?Sized>(&'a mut H);
        impl<H: FormatHost + ?Sized> crate::boot_fmt::Host for Locale<'_, H> {
            fn encode(
                &mut self,
                unit: u16,
                output: &mut [u8; 8],
                state: &mut [u8; 8],
            ) -> Option<usize> {
                self.0.format_encode(unit, output, state)
            }
            fn decimal_point(&mut self) -> u8 {
                self.0.format_decimal_point()
            }
        }
        crate::boot_fmt::fn_10012c(
            m,
            dst,
            size,
            fmt,
            &mut crate::boot_fmt::VaList { address: args },
            &mut Locale(self),
        ) as i32
    }
    /// Default ROM C locale, with platform-specific locale changes behind the host.
    fn format_encode(
        &mut self,
        unit: u16,
        output: &mut [u8; 8],
        _state: &mut [u8; 8],
    ) -> Option<usize> {
        if unit <= 255 {
            output[0] = unit as u8;
            Some(1)
        } else {
            None
        }
    }
    fn format_decimal_point(&mut self) -> u8 {
        b'.'
    }
}
fn string_length(m: &Memory<'_>, p: u32) -> u32 {
    let mut n = 0;
    while m.byte(p + n) != 0 {
        n += 1;
    }
    n
}
fn copy_bytes(m: &mut Memory<'_>, dst: u32, src: u32, n: u32) {
    for i in 0..n {
        let b = m.byte(src + i);
        put_byte(m, dst + i, b);
    }
}
fn marker(m: &Memory<'_>, p: u32) -> bool {
    m.byte(p) == b'{' && m.byte(p + 1) == b'*' && m.byte(p + 2) == b'}'
}
/// fn_173638. FormatParts is five guest words; reserved word 1 is untouched.
/// FormatHost defaults to the exact RVCT byte formatter.
pub fn fn_173638(
    m: &mut Memory<'_>,
    heap: u32,
    format: u32,
    args: u32,
    out: u32,
    h: &mut impl FormatHost,
) -> u32 {
    if format == 0 || out == 0 {
        return 0x84b02007;
    }
    let temp = call(
        m,
        h,
        0x196394,
        &[
            heap,
            string_length(m, format).wrapping_mul(4).wrapping_add(1),
        ],
    );
    if temp == 0 {
        return 0x84b0200a;
    }
    let text = call(m, h, 0x196394, &[heap, 4096]);
    m.put_word(out, text);
    if text == 0 {
        call(m, h, 0x196428, &[heap, temp]);
        return 0x84b0200a;
    }
    put_byte(m, temp, 0);
    let mut status = 0;
    let mut outside = false;
    let mut start = -1i32;
    let mut fields = 0u32;
    let mut length = 0u32;
    let mut i = 0u32;
    while m.byte(format + i) != 0 && !failed(status) {
        let c = m.byte(format + i) as u32;
        if c == b'%' as u32 {
            if start > -1 {
                start = -1;
            } else {
                start = i as i32;
            }
        } else if start == -1 {
            outside = true;
        } else if start > -1
            && (c.wrapping_sub(97) <= 25 || c.wrapping_sub(65) <= 25)
            && c != 108
            && c != 76
            && c != 104
        {
            if fields & 1 == 0 && c != 115 && c != 83 {
                status = 0x84b02007;
            } else {
                let old = length;
                let n = i.wrapping_sub(start as u32);
                length = length.wrapping_add(n + 4);
                copy_bytes(m, temp + old, format + start as u32, n + 1);
                let offset = old + n + 1;
                for (j, b) in b"{*}\0".iter().enumerate() {
                    put_byte(m, temp + offset + j as u32, *b);
                }
                start = -1;
                fields = fields.wrapping_add(1);
            }
        }
        i = i.wrapping_add(1);
    }
    if fields & 1 != 0 {
        status = 0x84b02007;
        fields = fields.wrapping_sub(1);
    } else if outside {
        status = 0x84b02007;
    }
    put_byte(m, m.word(out), 0);
    let text = m.word(out);
    let result = h.rvct_vsnprintf(m, text, 4096, temp, args);
    if result < 0 || result >= 4095 {
        put_byte(m, text + 4095, 0);
    }
    m.put_word(out + 8, 0);
    if fields as i32 > 1 {
        let names = call(m, h, 0x1963b8, &[heap, fields >> 1, 4]);
        m.put_word(out + 12, names);
        if names == 0 {
            call(m, h, 0x196428, &[heap, temp]);
            return 0x84b0200a;
        }
        let values = call(m, h, 0x1963b8, &[heap, fields >> 1, 4]);
        m.put_word(out + 16, values);
        if values == 0 {
            call(m, h, 0x196428, &[heap, temp]);
            return 0x84b0200a;
        }
    }
    let mut text = m.word(out);
    let mut i = 0u32;
    while (i as i32) < fields as i32 {
        let mut end = text;
        while m.byte(end) != 0 && !marker(m, end) {
            end += 1;
        }
        if m.byte(end) == 0 {
            break;
        }
        i += 1;
        put_byte(m, end, 0);
        let count = m.word(out + 8);
        if i & 1 != 0 {
            m.put_word(m.word(out + 12) + count * 4, text);
        } else {
            m.put_word(m.word(out + 16) + count * 4, text);
            m.put_word(out + 8, count.wrapping_add(1));
        }
        text = end + 3;
    }
    if i != fields {
        status = 0x84b02007;
    }
    call(m, h, 0x196428, &[heap, temp]);
    status
}
fn release_parameters(m: &mut Memory<'_>, heap: u32, params: u32, h: &mut impl Host) {
    for off in [0, 12, 16] {
        let p = m.word(params + off);
        if p != 0 {
            call(m, h, 0x196428, &[heap, p]);
            m.put_word(params + off, 0);
        }
    }
}
/// fn_173c70. params is a disjoint, writable, caller-owned 20-byte stack area.
pub fn fn_173c70(
    m: &mut Memory<'_>,
    engine: u32,
    category: u32,
    level: u32,
    format: u32,
    args: u32,
    params: u32,
    h: &mut impl FormatHost,
) -> u32 {
    if engine == 0 {
        return 0;
    }
    if category == 0 {
        return 0x84b02006;
    }
    for i in 0..20 {
        put_byte(m, params + i, 0);
    }
    if format != 0 {
        fn_173638(m, m.word(engine), format, args, params, h);
    }
    let mut result = 0;
    for off in [8, 16, 24, 32, 40, 52] {
        let methods = m.word(engine + off + 4);
        if methods != 0 && m.word(methods + 32) != 0 {
            let rc = call(
                m,
                h,
                m.word(methods + 32),
                &[
                    m.word(engine + off),
                    category,
                    level,
                    m.word(params + 8),
                    m.word(params + 12),
                    m.word(params + 16),
                ],
            );
            if failed(rc) {
                result = rc;
            }
        }
    }
    release_parameters(m, m.word(engine), params, h);
    result
}
/// fn_173df4. secure_name points to caller-mapped SECURE, not a host string.
/// params is disjoint caller-owned stack memory, as in fn_173c70.
pub fn fn_173df4(
    m: &mut Memory<'_>,
    engine: u32,
    arg: u32,
    format: u32,
    args: u32,
    params: u32,
    secure_name: u32,
    h: &mut impl FormatHost,
) -> u32 {
    if engine == 0 {
        return 0;
    }
    for i in 0..20 {
        put_byte(m, params + i, 0);
    }
    if format != 0 {
        fn_173638(m, m.word(engine), format, args, params, h);
        let secure = m.word(engine + 60);
        if secure != 0 && m.byte(secure) != 0 {
            let heap = m.word(engine);
            let n = m.word(params + 8).wrapping_add(1).wrapping_mul(4);
            let p = call(m, h, 0x1963e0, &[heap, m.word(params + 12), n]);
            if p != 0 {
                m.put_word(params + 12, p);
                let p = call(
                    m,
                    h,
                    0x1963e0,
                    &[
                        heap,
                        m.word(params + 16),
                        m.word(params + 8).wrapping_add(1).wrapping_mul(4),
                    ],
                );
                if p != 0 {
                    m.put_word(params + 16, p);
                    let count = m.word(params + 8);
                    m.put_word(m.word(params + 12) + count * 4, secure_name);
                    m.put_word(m.word(params + 16) + count * 4, secure);
                    m.put_word(params + 8, count.wrapping_add(1));
                }
            }
        }
    }
    let mut result = 0;
    for off in [8, 16, 24, 32, 40, 52] {
        let methods = m.word(engine + off + 4);
        if methods != 0 && m.word(methods + 36) != 0 {
            let rc = call(
                m,
                h,
                m.word(methods + 36),
                &[
                    m.word(engine + off),
                    arg,
                    m.word(params + 8),
                    m.word(params + 12),
                    m.word(params + 16),
                ],
            );
            if failed(rc) {
                result = rc;
            }
        }
    }
    release_parameters(m, m.word(engine), params, h);
    result
}

/// Filesystem operations return local out-parameters by value. join is the
/// unported fn_17db60 service, not a host path-library substitute.
pub trait DirectoryHost: Host {
    fn join(
        &mut self,
        m: &mut Memory<'_>,
        base: u32,
        name: u32,
        flag: u32,
        heap: u32,
    ) -> (u32, u32);
    fn first(
        &mut self,
        m: &mut Memory<'_>,
        source: u32,
        heap: u32,
        pattern: u32,
        flags: u32,
    ) -> (u32, u32, u32);
    fn next(&mut self, m: &mut Memory<'_>, iterator: u32) -> (u32, u32);
}
fn free_nonnull(m: &mut Memory<'_>, h: &mut impl Host, heap: u32, p: u32) {
    if p != 0 {
        call(m, h, 0x196428, &[heap, p]);
    }
}
fn dot_entry(m: &Memory<'_>, p: u32) -> bool {
    m.byte(p) == b'.' && (m.byte(p + 1) == 0 || (m.byte(p + 1) == b'.' && m.byte(p + 2) == 0))
}
/// fn_17dd30. star is the caller-mapped literal "*". Discovery fn_17e0bc
/// remains an explicit unported dependency, while recursive calls are real.
pub fn fn_17dd30(
    m: &mut Memory<'_>,
    context: u32,
    directory: u32,
    star: u32,
    h: &mut impl DirectoryHost,
) -> u32 {
    let input = m.word(context + 8);
    let heap = m.word(input + 4);
    let mut rc = call(m, h, 0x17e0bc, &[context, directory]);
    if failed(rc) {
        return rc;
    }
    if directory == 0 {
        return 0;
    }
    let (r, pattern) = h.join(m, directory, star, 0, heap);
    rc = r;
    if failed(rc) {
        return rc;
    }
    let (r, mut name, mut iterator) = h.first(m, m.word(input + 28), heap, pattern, 16);
    rc = r;
    let mut path = 0;
    if failed(rc) {
        if rc.wrapping_shl(19) == 0xa00000 {
            rc = 0;
        }
        free_nonnull(m, h, heap, pattern);
        if iterator != 0 {
            call(m, h, 0x173510, &[iterator]);
        }
        return rc;
    }
    loop {
        if dot_entry(m, name) {
            call(m, h, 0x196428, &[heap, name]);
            let (r, n) = h.next(m, iterator);
            rc = r;
            name = n;
            if rc.wrapping_shl(19) == 0xa00000 {
                rc = 0;
                name = 0;
                break;
            }
            if failed(rc) {
                if iterator != 0 {
                    call(m, h, 0x173510, &[iterator]);
                }
                free_nonnull(m, h, heap, name);
                free_nonnull(m, h, heap, pattern);
                return rc;
            }
        } else {
            let (r, p) = h.join(m, directory, name, 0, heap);
            rc = r;
            path = p;
            if failed(rc) {
                if iterator != 0 {
                    call(m, h, 0x173510, &[iterator]);
                }
                free_nonnull(m, h, heap, name);
                free_nonnull(m, h, heap, pattern);
                return rc;
            }
            break;
        }
    }
    free_nonnull(m, h, heap, name);
    free_nonnull(m, h, heap, pattern);
    if path == 0 {
        if iterator != 0 {
            call(m, h, 0x173510, &[iterator]);
        }
        return rc;
    }
    let mut again = true;
    loop {
        if !again {
            call(m, h, 0x173510, &[iterator]);
            return rc;
        }
        if iterator == 0 {
            return rc;
        }
        rc = fn_17dd30(m, context, path, star, h);
        call(m, h, 0x196428, &[heap, path]);
        path = 0;
        if failed(rc) {
            call(m, h, 0x173510, &[iterator]);
            return rc;
        }
        again = false;
        let (r, mut e) = h.next(m, iterator);
        rc = r;
        if failed(rc) {
            if rc.wrapping_shl(19) == 0xa00000 {
                rc = 0;
                continue;
            }
            call(m, h, 0x173510, &[iterator]);
            return rc;
        }
        loop {
            if dot_entry(m, e) {
                call(m, h, 0x196428, &[heap, e]);
                let (r, n) = h.next(m, iterator);
                rc = r;
                e = n;
                if rc.wrapping_shl(19) == 0xa00000 {
                    rc = 0;
                    e = 0;
                    break;
                }
                if failed(rc) {
                    break;
                }
            } else {
                break;
            }
        }
        if e == 0 && rc == 0 {
            continue;
        }
        let (r, p) = h.join(m, directory, e, 0, heap);
        rc = r;
        path = p;
        if failed(rc) {
            call(m, h, 0x196428, &[heap, e]);
            call(m, h, 0x173510, &[iterator]);
            return rc;
        }
        call(m, h, 0x196428, &[heap, e]);
        rc = 0;
        again = true;
    }
}

/// Host filesystem, dictionary and parser operations used by discovery.
pub trait DiscoveryHost: DirectoryHost + ResourceHost + crate::boot_part_b2::AttributeHost {
    fn remove_property(&mut self,m:&mut Memory<'_>,object:u32,key:&[u8])->u32;
}
pub struct DiscoveryGlobals {
    pub hdr_pattern:u32,
    pub dat_pattern:u32,
    pub one:u32,
    pub surrogate_table:u32,
    /// Disjoint, caller-owned output word for the fn_198294 local.
    pub scratch:u32,
}
fn property_value(m:&mut Memory<'_>,h:&mut impl PropertyHost,db:u32,key:&[u8])->(u32,u32) {
    let (rc,p)=h.property(m,db,key);
    if failed(rc) {(0x81506014,0)} else {(0,if p==0 {0} else {m.word(p)})}
}
fn discover_component(m:&mut Memory<'_>,entry:u32,input:u32,_collection:bool,g:&DiscoveryGlobals,h:&mut impl DiscoveryHost)->u32 {
    let mut key=&b"COMPONENT"[..];let (mut rc,mut component)=property_value(m,h,entry,key);let mut common=false;
    if failed(rc) {
        key=b"COMMONCOMPONENT";let r=property_value(m,h,entry,key);rc=r.0;component=r.1;
        if failed(rc) {return rc;}common=true;
    }
    let (rc,hdrfile)=property_value(m,h,entry,b"HDRFILE");if failed(rc) {return rc;}
    let (mut rc,path)=h.join(m,hdrfile,component,1,m.word(input+4));if failed(rc) {return rc;}
    rc=crate::boot_part_b2::AttributeHost::attribute(h,m,entry,b"FULLPATHCOMPONENT",[path,0,0]);
    if !failed(rc) || rc.wrapping_shl(19)==0x100000 {rc=h.remove_property(m,entry,key);}
    call(m,h,0x196428,&[m.word(input+4),path]);if failed(rc) {return rc;}
    if common {
        rc=crate::boot_part_b2::AttributeHost::attribute(h,m,entry,b"COMMON",[g.one,0,0]);
        if failed(rc) && rc.wrapping_shl(19)==0x100000 {rc=0;}
    }rc
}
struct DiscoveryAttributes<'a,H>(&'a mut H);
impl<H:DiscoveryHost> Host for DiscoveryAttributes<'_,H> {
    fn call(&mut self,m:&mut Memory<'_>,f:u32,a:&[u32])->u32 {
        if f==0x19665c {release_dictionary(m,self.0,a[0])} else {self.0.call(m,f,a)}
    }
    fn thread_id(&mut self)->u32 {self.0.thread_id()}
}
impl<H:DiscoveryHost> crate::boot_part_b2::AttributeHost for DiscoveryAttributes<'_,H> {
    fn create(&mut self,m:&mut Memory<'_>,ctx:u32,flag:u32,ticket:[u32;5],out:u32)->u32 {self.0.create(m,ctx,flag,ticket,out)}
    fn attribute(&mut self,m:&mut Memory<'_>,object:u32,key:&[u8],value:[u32;3])->u32 {self.0.attribute(m,object,key,value)}
}
/// fn_17e0bc body. The live entry binds filesystem and real XML parsing.
/// Header discovery and .dat surrogate registration. All stack
/// outputs are local values except the explicit reusable scratch word.
pub fn fn_17e0bc(m:&mut Memory<'_>,context:u32,directory:u32,g:&DiscoveryGlobals,h:&mut impl DiscoveryHost)->u32 {
    let input=m.word(context+8);let heap=m.word(input+4);let source=m.word(input+28);
    if directory==0 {return 0;}
    let mut rc;
    for dat in [false,true] {
        let (r,pattern)=h.join(m,directory,if dat {g.dat_pattern} else {g.hdr_pattern},0,heap);rc=r;
        if failed(rc) {return rc;}
        let (r,name,mut iterator)=h.first(m,source,heap,pattern,0);rc=r;let mut path=0;
        if !failed(rc) {
            let (r,p)=h.join(m,directory,name,0,heap);rc=r;path=p;
            if failed(rc) {if iterator!=0 {call(m,h,0x173510,&[iterator]);}iterator=0;}
            call(m,h,0x196428,&[heap,name]);
        } else if rc.wrapping_shl(19)==0xa00000 {rc=0;}
        call(m,h,0x196428,&[heap,pattern]);
        if failed(rc) {return rc;}
        if iterator==0 {if dat {return rc;}continue;}
        loop {
            if dat {
                log_info(m,h,m.word(input+16),4,b"brkhlp_FindHdrFileAndAddItem szFullPathName=%s",&[hash_guest(m,path)]);
                let mut slash=0;for i in 0..string_length(m,path) {if m.byte(path+i)==b'/' {slash=path+i;}}
                if slash==0 {slash=path.wrapping_sub(1);}
                let n=string_length(m,path).wrapping_sub(slash.wrapping_sub(path)).wrapping_sub(5);
                let surrogate=call(m,h,0x196394,&[heap,n.wrapping_add(1)]);if surrogate==0 {return 0x8150600a;}
                copy_bytes(m,surrogate,slash.wrapping_add(1),n);put_byte(m,surrogate+n,0);
                log_info(m,h,m.word(input+16),4,b"brkhlp_FindHdrFileAndAddItem szSurrogateBrokerString=%s",&[hash_guest(m,surrogate)]);
                let saved=m.word(g.scratch);
                // The ROM's parser may have set this local on an earlier header.
                // This branch starts with a null local, and fn_198294 writes it.
                m.put_word(g.scratch,0);
                rc=crate::boot_part_b2::fn_198294(m,heap,surrogate,path,g.scratch,g.surrogate_table,&mut DiscoveryAttributes(h));
                let entry=m.word(g.scratch);m.put_word(g.scratch,saved);
                if !failed(rc) {rc=fn_177124(m,path,entry,context,h);}
                if !failed(rc) {log_info(m,h,m.word(input+16),4,b"brkhlp_FindHdrFileAndAddItem AddedNewSurrogateBrokerString=%s",&[hash_guest(m,surrogate)]);}
                call(m,h,0x196428,&[heap,surrogate]);
            } else {
                let len=string_length(m,path);
                if len>4 && equal(m,path+len-4,b".hdr") {
                    let (r,resource,readers)=h.decode(m,[m.word(context),m.word(context+4),path,0]);rc=r;
                    if failed(rc) {
                        log_error(m,h,m.word(input+16),0x8d7668,276,b"%s%s",&[hash_text(b"file"),hash_guest(m,path)]);
                        if rc.wrapping_shl(19)!=0x500000 {rc=0;}
                    } else if readers!=0 {
                        let mut index=0;
                        loop {
                            let (found,p)=h.output(m,0x1970ac,&[readers,index]);if found==0 {break;}
                            let entry=m.word(p);
                            if !failed(rc) {
                                rc=discover_component(m,entry,input,true,g,h);
                                if rc==0x81506014 || !failed(rc) {rc=fn_177124(m,path,entry,context,h);}
                                else {release_dictionary(m,h,entry);}
                            } else {release_dictionary(m,h,entry);}
                            index+=1;
                        }
                        call(m,h,0x196c7c,&[readers]);
                    } else {
                        rc=discover_component(m,resource,input,false,g,h);
                        if rc==0x81506014 || !failed(rc) {rc=fn_177124(m,path,resource,context,h);}
                    }
                }
            }
            call(m,h,0x196428,&[heap,path]);
            if failed(rc) {call(m,h,0x173510,&[iterator]);return rc;}
            let (r,name)=h.next(m,iterator);rc=r;
            if failed(rc) {
                if rc.wrapping_shl(19)==0xa00000 {rc=0;}
                call(m,h,0x173510,&[iterator]);return rc;
            }
            let (r,p)=h.join(m,directory,name,0,heap);rc=r;path=p;
            call(m,h,0x196428,&[heap,name]);
            if failed(rc) {call(m,h,0x173510,&[iterator]);return rc;}
            rc=0;
        }
    }0
}

const BRIDGE_FAILURE: u32 = 0x81506001;

/// Platform operations needed by the real constructor/heap bridge.
pub trait ConstructorPlatform {
    fn thread_id(&mut self) -> u32;
    fn arbiter_address(&self) -> u32;
    fn arbitrate(&mut self, memory: &mut Memory<'_>, args: &[u32]) -> u32;
}
struct ConstructorOs<'a, P>(&'a mut P);
impl<P: ConstructorPlatform> crate::bridge::Platform for ConstructorOs<'_, P> {
    fn thread_id(&mut self) -> u32 { self.0.thread_id() }
    fn arbiter_address(&self) -> u32 { self.0.arbiter_address() }
    fn arbitrate(&mut self, m: &mut Memory<'_>, a: &[u32]) -> u32 {
        self.0.arbitrate(m, a)
    }
}
struct BridgedConstructor<'a, H, P> {
    inner: &'a mut H,
    platform: &'a mut P,
    workspace: &'a mut [u8],
    error: Option<crate::engine::Error>,
}
impl<H, P: ConstructorPlatform> BridgedConstructor<'_, H, P> {
    fn service(&mut self, m: &mut Memory<'_>, f: u32, a: &[u32]) -> u32 {
        let result = (|| {
            let mut regions = [crate::engine::Region {base:m.base,bytes:&mut *m.bytes}];
            let mut memory = crate::engine::Memory::new(&mut regions)?;
            let mut os = ConstructorOs(&mut *self.platform);
            let mut flat = crate::bridge::Flat::new(&mut memory, self.workspace, &mut os, 0)?;
            let rc = match f {
                0x176e40 => flat.fn_176e40(a.try_into().expect("three constructor words")),
                0x1964e0 => flat.fn_1964e0(a.try_into().expect("eight map words")),
                0x1963b8 => flat.fn_1963b8(a[0],a[1],a[2]),
                0x196394 => flat.fn_196394(a[0],a[1]),
                0x196428 => {flat.fn_196428(a[0],a[1]);0},
                0x19665c => flat.fn_19665c(a[0]),
                _ => return Err(crate::engine::Error::UnsupportedStage {entry:f}),
            };
            flat.finish()?;
            Ok(rc)
        })();
        match result {
            Ok(rc) => rc,
            Err(e) => {
                if self.error.is_none() {self.error=Some(e);}
                if matches!(f,0x1963b8|0x196394) {0} else {BRIDGE_FAILURE}
            }
        }
    }
}
impl<H: ConstructorHost,P: ConstructorPlatform> Host for BridgedConstructor<'_,H,P> {
    fn thread_id(&mut self)->u32 {self.platform.thread_id()}
    fn release_dictionary(&mut self,m:&mut Memory<'_>,p:u32)->Option<u32> {
        Some(self.service(m,0x19665c,&[p]))
    }
    fn call(&mut self,m:&mut Memory<'_>,f:u32,a:&[u32])->u32 {
        if matches!(f,0x1963b8|0x196394|0x196428|0x176e40|0x1964e0|0x19665c) {
            self.service(m,f,a)
        } else {self.inner.call(m,f,a)}
    }
}
impl<H: ConstructorHost,P: ConstructorPlatform> OutputHost for BridgedConstructor<'_,H,P> {
    fn output(&mut self,m:&mut Memory<'_>,f:u32,a:&[u32])->(u32,u32) {
        self.inner.output(m,f,a)
    }
}
impl<H: ConstructorHost,P: ConstructorPlatform> PropertyHost for BridgedConstructor<'_,H,P> {
    fn property(&mut self,m:&mut Memory<'_>,db:u32,key:&[u8])->(u32,u32) {
        self.inner.property(m,db,key)
    }
    fn insert(&mut self,m:&mut Memory<'_>,db:u32,key:u32,e:[u32;3])->u32 {
        self.inner.insert(m,db,key,e)
    }
}
impl<H: ConstructorHost,P: ConstructorPlatform> ResourceHost for BridgedConstructor<'_,H,P> {
    fn decode(&mut self,m:&mut Memory<'_>,a:[u32;4])->(u32,u32,u32) {
        self.inner.decode(m,a)
    }
}
impl<H: ConstructorHost,P: ConstructorPlatform> DirectoryHost for BridgedConstructor<'_,H,P> {
    fn join(&mut self,m:&mut Memory<'_>,base:u32,name:u32,flag:u32,heap:u32)->(u32,u32) {
        self.inner.join(m,base,name,flag,heap)
    }
    fn first(&mut self,m:&mut Memory<'_>,source:u32,heap:u32,pattern:u32,flags:u32)->(u32,u32,u32) {
        self.inner.first(m,source,heap,pattern,flags)
    }
    fn next(&mut self,m:&mut Memory<'_>,iterator:u32)->(u32,u32) {
        self.inner.next(m,iterator)
    }
}
impl<H: ConstructorHost,P: ConstructorPlatform> ConstructorHost for BridgedConstructor<'_,H,P> {
    fn context_pair(&mut self,m:&mut Memory<'_>,owner:[u32;2])->(u32,u32) {
        self.inner.context_pair(m,owner)
    }
    fn construct_output(&mut self,m:&mut Memory<'_>,f:u32,a:&[u32],out:u32)->u32 {
        match f {
            0x176e40 => self.service(m,f,&[a[0],a[1],out]),
            0x1964e0 => self.service(m,f,&[a[0],a[1],a[2],a[3],a[4],a[5],a[6],out]),
            _ => {
                self.error=Some(crate::engine::Error::UnsupportedStage{entry:f});
                BRIDGE_FAILURE
            }
        }
    }
}
/// fn_177624 with bridge-owned 176e40/1964e0 and guest allocator entries.
/// The workspace is caller-owned and indexed by absolute guest addresses.
/// Remaining callbacks still come from h; unsupported bridge methods return
/// an explicit engine error, never a successful scripted constructor result.
pub fn fn_177624_bridged(
    m:&mut Memory<'_>,owner:[u32;2],config:u32,flags:u32,out:u32,
    module:u32,source_label:u32,star:u32,h:&mut impl ConstructorHost,
    platform:&mut impl ConstructorPlatform,workspace:&mut [u8],
)->Result<u32,crate::engine::Error> {
    let mut host=BridgedConstructor {inner:h,platform,workspace,error:None};
    let rc=fn_177624(m,owner,config,flags,out,module,source_label,star,&mut host);
    match host.error {Some(e)=>Err(e),None=>Ok(rc)}
}

/// Broker output boundaries. Out-pointers remain guest addresses.
pub trait DataHost: QueryHost {
    fn data_output(&mut self,m:&mut Memory<'_>,f:u32,args:&[u32],out:u32)->u32;
}
/// Guest constants and a disjoint 24-byte caller-owned stack area.
pub struct DataOpenGlobals {
    pub module:u32,
    pub allocators:[u32;4],
    pub scratch:u32,
    pub fullpath_key:u32,
    pub add_item_format:u32,
    pub broker_label:u32,
    pub ctype_table:u32,
    pub brokerstring_key:u32,
}
fn find_byte(m:&Memory<'_>,mut p:u32,b:u8)->u32 {
    while m.byte(p)!=0 {if m.byte(p)==b {return p;}p+=1;}0
}
fn find_text(m:&Memory<'_>,mut p:u32,s:&[u8])->u32 {
    while m.byte(p)!=0 {
        let mut i=0;while i<s.len() && m.byte(p+i as u32)==s[i] {i+=1;}
        if i==s.len() {return p;}p+=1;
    }0
}
fn store_text(m:&mut Memory<'_>,p:u32,s:&[u8]) {for (i,b) in s.iter().chain(core::iter::once(&0)).enumerate() {put_byte(m,p+i as u32,*b);}}
fn equal_strings(m:&Memory<'_>,mut a:u32,mut b:u32)->bool {
    while m.byte(a)==m.byte(b) {if m.byte(a)==0 {return true;}a+=1;b+=1;}false
}
fn data_add_entry(m:&mut Memory<'_>,p:u32,handle:u32,name:u32,g:&DataOpenGlobals,h:&mut impl DataHost)->u32 {
    if handle==0 {return 0x81506007;}
    let ctx=m.word(p+4);let runtime=m.word(ctx+8);
    // The ROM passes the add-item format's address (D_177278) and the
    // broker string itself; hosts that normalize log arguments keep the
    // recorded-reply hashes.
    let real=h.real_log_args();
    let fmt=if real {g.add_item_format} else {hash_guest(m,g.add_item_format)};
    call(m,h,0x1738e8,&[m.word(runtime+16),g.module,4,0,fmt]);
    call(m,h,0xffff0001,&[guest(m,real,name)]);
    // The ROM passes &entry, the twelve byte record. Hosts that trace by
    // value keep the recorded contents; hosts that bind the real insert
    // pass the record itself out of the caller scratch.
    let rec=if real {
        let r=g.scratch+12;
        m.put_word(r,handle);m.put_word(r+4,name);m.put_word(r+8,0);
        [r,r+4,r+8]
    } else {
        [handle,name,0]
    };
    let mut rc=lock(m,h,m.word(ctx+28),8);
    if !failed(rc) {
        rc=h.insert(m,m.word(ctx+16),name,rec);
        if failed(rc) {log_error(m,h,m.word(runtime+16),0x8d7668,0x121,b"%s%s",&[guest(m,real,g.broker_label),guest(m,real,name)]);}
    }
    let status=lock(m,h,m.word(ctx+28),12);if failed(status)&&!failed(rc) {rc=status;}
    // The ROM releases the table only if the insert left the record's
    // handle word set; the database's own value creator clears it. Hosts
    // that trace by value always release, as before.
    if !real || m.word(rec[0])!=0 {release_dictionary(m,h,handle);}
    rc
}
/// fn_1795f0 body. The live entry binds file and archive implementations.
/// Archive/map bodies execute directly. Dictionary/query and logger boundaries
/// remain caller-supplied; this is not an end-to-end cold-boot certificate.
pub fn fn_1795f0(m:&mut Memory<'_>,broker:u32,mut name:u32,flags:u32,out:u32,g:&DataOpenGlobals,h:&mut impl DataHost)->u32 {
    if broker==0 || name==0 || out==0 {return 0x81506007;}
    let ctx=m.word(broker+4);let runtime=m.word(ctx+8);let heap=m.word(runtime+4);
    m.put_word(out,0);
    let real=h.real_log_args();
    log_info(m,h,m.word(broker),4,b"brk_DataOpen : BrokerString=%s : Begin",&[guest(m,real,name)]);
    let data=call(m,h,0x1963b8,&[heap,1,12]);
    if data==0 {log_info(m,h,m.word(broker),4,b"brk_DataOpen : End %x",&[0x8150600a]);return 0x8150600a;}
    let colon=find_byte(m,name,b':');let mut prefix=0;let mut lower=0;let mut path=0;let mut entry=0;
    let mut unlock=colon!=0;
    let mut rc='operation: {
        if colon!=0 && colon!=name {
            let rc=lock(m,h,m.word(ctx+12),8);
            if failed(rc) {unlock=false;break 'operation rc;}
            let length=colon-name;prefix=call(m,h,0x196394,&[heap,length+1]);
            if prefix==0 {break 'operation 0x8150600a;}
            copy_bytes(m,prefix,name,length);put_byte(m,prefix+length,0);name=prefix;
        }
        let (mut found,e)=h.query_output(m,0x1772b0,&[broker,name]);entry=e;
        if !failed(found) {log_info(m,h,m.word(broker),3,b"brk_DataOpen : match on broker string %s",&[guest(m,real,name)]);}
        else {
            let clean=call(m,h,0x196394,&[heap,string_length(m,name)+4]);
            if clean==0 {break 'operation 0x8150600a;}
            let mut i=0;
            while i<string_length(m,name) {
                let c=m.byte(name+i);let c=if c.is_ascii_alphanumeric() {c} else if c==b'/' || c==b'_' {b'_'} else {b'-'};
                put_byte(m,clean+i,c);i+=1;
            }put_byte(m,clean+i,0);
            if !equal_strings(m,name,clean) {let (r,e)=h.query_output(m,0x1772b0,&[broker,clean]);found=r;entry=e;}
            let mut surrogate=!failed(found);
            if failed(found) {
                copy_bytes(m,clean,name,string_length(m,name)+1);
                let suffix=find_text(m,clean,b"mpthreeml");
                if suffix!=0 {store_text(m,suffix,b"mpthreevadml");let (r,e)=h.query_output(m,0x1772b0,&[broker,clean]);found=r;entry=e;surrogate=!failed(found);}
            }
            if surrogate {log_info(m,h,m.word(broker),3,b"brk_DataOpen : match on Surrogate broker string %s",&[guest(m,real,clean)]);}
            call(m,h,0x196428,&[heap,clean]);
        }
        if !failed(found) {
            if m.word(entry+8)!=0 {break 'operation fn_17ab44(m,m.word(broker),entry,colon.wrapping_add(1),flags,data,h);}
            let (status,property)=h.property(m,m.word(entry),b"FULLPATHCOMPONENT");
            if !failed(status) && property!=0 {path=m.word(property);}
        } else if colon!=0 {
            let new_entry=call(m,h,0x1963b8,&[heap,1,string_length(m,name)+13]);
            if new_entry==0 {break 'operation 0x8150600a;}
            m.put_word(new_entry,new_entry+12);copy_bytes(m,new_entry+12,name,string_length(m,name)+1);
            let a=[heap,1,m.word(g.module+108),m.word(g.module+112),m.word(g.module+116),m.word(g.module+120),m.word(g.module+124)];
            let saved=m.word(g.scratch);let mut r=h.data_output(m,0x1964e0,&a,g.scratch);let table=m.word(g.scratch);m.put_word(g.scratch,saved);
            if failed(r) {break 'operation r;}
            // The new entry travels by address for hosts that bind the real
        // insert, by recorded contents for hosts that trace by value.
        let value=if real {[new_entry,new_entry+4,new_entry+8]} else {[m.word(new_entry),m.word(new_entry+4),m.word(new_entry+8)]};
        r=h.insert(m,table,g.brokerstring_key,value);
            if failed(r) {break 'operation r;}
            r=data_add_entry(m,broker,table,m.word(new_entry),g,h);
            if r&0x1fff!=2 && failed(r) {break 'operation r;}
            let (r,e)=h.query_output(m,0x1772b0,&[broker,name]);entry=e;if failed(r) {break 'operation r;}
        }
        if path==0 {
            lower=call(m,h,0x196394,&[heap,string_length(m,name)+1]);if lower==0 {break 'operation 0x8150600a;}
            path=lower;let mut i=0;
            loop {let c=m.byte(name+i);put_byte(m,lower+i,if m.half(g.ctype_table+(c as u32)*2)&1!=0 {c.wrapping_add(32)} else {c});i+=1;if i>string_length(m,name) {break;}}
        }
        let r=h.data_output(m,0x173000,&[m.word(runtime+28),heap,path],data);
        if failed(r) {
            if flags&1!=0 {log_info(m,h,m.word(broker),3,b"brk_DataOpen : osspi_DataOpen failed: %s, %x",&[guest(m,real,path),r]);}
            else if path!=name {let b=lit(m,h,real,b"BrokerString");let f=lit(m,h,real,b"file");let l=lit(m,h,real,b"lherror");log_error(m,h,m.word(broker),0x8d7668,0x113,b"%s%s%s%s%s%x",&[b,guest(m,real,name),f,guest(m,real,path),l,r]);}
            else {let b=lit(m,h,real,b"BrokerString");let l=lit(m,h,real,b"lherror");log_error(m,h,m.word(broker),0x8d7668,0x113,b"%s%s%x",&[b,guest(m,real,name),l,r]);}
            break 'operation r;
        }
        log_info(m,h,m.word(broker),3,b"brk_DataOpen : osspi_DataOpen %s, %p",&[guest(m,real,path),m.word(data)]);
        if colon==0 {unlock=false;break 'operation r;}
        let mut r=fn_17c8fc(m,[m.word(ctx),m.word(ctx+4)],name,entry+8,g.allocators,h);
        if failed(r) {break 'operation r;}
        r=call(m,h,0x1735e0,&[m.word(data),m.word(entry+8)]);
        if failed(r) {
            if r&0x1fff!=1 {m.put_word(entry+8,0);break 'operation r;}
            let saved=core::array::from_fn::<_,6,_>(|i|m.word(g.scratch+i as u32*4));
            m.put_word(g.scratch,0);m.put_word(g.scratch+4,0);m.put_word(g.scratch+8,0);m.put_word(g.scratch+12,0);
            r=fn_179374(m,broker,data,0,g.scratch+4,g.scratch,h);
            if !failed(r) {
                r=call(m,h,0x17cbb4,&[m.word(entry+8),g.module+128,g.scratch,g.scratch+12,1]);
                let status=call(m,h,0x179208,&[broker,data,m.word(g.scratch)]);if failed(status)&&!failed(r) {r=status;}
                let status=call(m,h,0x1730fc,&[m.word(data)]);m.put_word(data,0);if failed(status)&&!failed(r) {r=status;}
            }
            for (i,v) in saved.iter().enumerate() {m.put_word(g.scratch+i as u32*4,*v);}
            if failed(r) {break 'operation r;}
        }
        fn_17ab44(m,m.word(broker),entry,colon+1,flags,data,h)
    };
    if unlock {let status=lock(m,h,m.word(ctx+12),12);if failed(status)&&!failed(rc) {rc=status;}}
    if failed(rc) {call(m,h,0x196428,&[heap,data]);} else {m.put_word(out,data);}
    free_nonnull(m,h,heap,prefix);free_nonnull(m,h,heap,lower);
    log_info(m,h,m.word(broker),4,b"brk_DataOpen : End %x",&[rc]);rc
}

/// Live file-service composition for the two boot-part-b entry points.
///
/// The caller supplies a zero-based arena (the file ports use absolute guest
/// indexing), a disjoint 24-byte scratch area, and the regional "rb" string.
/// Scratch must not overlap DataOpenGlobals::scratch or DiscoveryGlobals::scratch,
/// live input/output, or host allocations. Words are restored on error returns too.
/// Nested host calls must use their own scratch. Unknown services are forwarded,
/// not synthesized.
pub struct LiveFileServices<'a, H> {
    pub inner: &'a mut H,
    pub scratch: u32,
    pub rb: u32,
    pub archive_params: [u32; 4],
    pub parser: Option<LiveParserGlobals>,
    pub archive: Option<&'a crate::boot_part_e::ArchiveEnvironment>,
    pub unmap: Option<&'a crate::boot_part_e::UnmapEnvironment>,
}
impl<H: Host> crate::boot_part_d::Host for LiveFileServices<'_, H> {
    fn call_m(&mut self, method:u32, args:&[u32], m:&mut Memory<'_>)->u32 {
        Host::call(self,m,method,args)
    }
    fn params_call_m(&mut self,q:u32,params:&mut [u32;4],p:u32,out:&mut u32,flags:u32,m:&mut Memory<'_>)->u32 {
        let s=self.scratch;
        let saved=core::array::from_fn::<_,6,_>(|i|m.word(s+4*i as u32));
        for (i,v) in params.iter().enumerate() {m.put_word(s+4*i as u32,*v);}
        m.put_word(s+16,*out);
        let e=self.archive.expect("live archive environment required");
        let rc=crate::boot_part_e::fn_17cbb4(m,q,s,p,s+16,flags,e,self);
        for (i,v) in params.iter_mut().enumerate() {*v=m.word(s+4*i as u32);}
        *out=m.word(s+16);
        for (i,v) in saved.iter().enumerate() {m.put_word(s+4*i as u32,*v);}
        rc
    }
}
impl<H: Host> crate::boot_part_e::Host for LiveFileServices<'_, H> {
    fn call(&mut self,m:&mut Memory<'_>,method:u32,a:&[u32])->u32 {
        Host::call(self,m,method,a)
    }
}
impl<H:Host> crate::boot_part_e::ArchiveHost for LiveFileServices<'_,H> {
    fn service(&mut self,m:&mut Memory<'_>,f:u32,a:&[u32],out:&mut [u32;4])->u32 {self.inner.archive_service(m,f,a,out)}
    fn read(&mut self,m:&mut Memory<'_>,f:u32,ctx:u32,out:&mut [u8])->u32 {self.inner.archive_read(m,f,ctx,out)}
}
impl<H: Host> LiveFileServices<'_, H> {
    /// Execute the real 173000, including allocator and source vtable dispatch.
    pub fn open(&mut self,m:&mut Memory<'_>,source:u32,heap:u32,path:u32,out:u32)->u32 {
        crate::boot_part_d::fn_173000(m,source,heap,path,out,self.rb,self)
    }
    /// Guest-output form also permits direct strict differential replay.
    pub fn join_into(&mut self,m:&mut Memory<'_>,out:u32,base:u32,name:u32,flag:u32,heap:u32)->u32 {
        crate::boot_part_e::fn_17db60(m,out,base,name,flag,heap,self)
    }
    pub fn first_into(&mut self,m:&mut Memory<'_>,source:u32,heap:u32,pattern:u32,flags:u32,name:u32,it:u32)->u32 {
        crate::boot_part_d::fn_17340c(m,source,heap,pattern,flags,name,it,self)
    }
    pub fn next_into(&mut self,m:&mut Memory<'_>,it:u32,name:u32)->u32 {
        crate::boot_part_d::fn_1734e8(m,it,name,self)
    }
}
impl<H: Host> Host for LiveFileServices<'_, H> {
    fn thread_id(&mut self)->u32 {self.inner.thread_id()}
    fn release_dictionary(&mut self,m:&mut Memory<'_>,p:u32)->Option<u32> {self.inner.release_dictionary(m,p)}
    fn call(&mut self,m:&mut Memory<'_>,f:u32,a:&[u32])->u32 {
        match f {
            0x196394 => crate::boot_part_e::alloc(m,a[0],a[1],self),
            0x1963b8 => crate::boot_part_e::calloc(m,a[0],a[1],a[2],self),
            0x1963e0 => crate::boot_part_e::fn_1963e0(m,a[0],a[1],a[2],self),
            0x196428 => {crate::boot_part_e::release(m,a[0],a[1],self);0},
            0x173510 => crate::boot_part_d::fn_173510(m,a[0],self),
            0x1730fc => crate::boot_part_d::fn_1730fc(m,a[0],self),
            0x17cbb4 => {
                let e=self.archive.expect("live archive environment required");
                crate::boot_part_e::fn_17cbb4(m,a[0],a[1],a[2],a[3],a[4],e,self)
            },
            0x179208 => {
                let e=self.unmap.expect("live unmap environment required");
                crate::boot_part_e::fn_179208(m,a[0],a[1],a[2],e,self)
            },
            0x1b20a4 => crate::normalization::quad_copy(m.bytes,a[0]-m.base,a[1]-m.base,a[2]),
            0x1735e0 => crate::boot_part_d::fn_1735e0(m,a[0],a[1],self.archive_params,self),
            _ => self.inner.call(m,f,a),
        }
    }
}
impl<H: OutputHost> OutputHost for LiveFileServices<'_, H> {
    fn output(&mut self,m:&mut Memory<'_>,f:u32,a:&[u32])->(u32,u32) {self.inner.output(m,f,a)}
}
impl<H: PropertyHost> PropertyHost for LiveFileServices<'_, H> {
    fn property(&mut self,m:&mut Memory<'_>,p:u32,key:&[u8])->(u32,u32) {self.inner.property(m,p,key)}
    fn insert(&mut self,m:&mut Memory<'_>,p:u32,key:u32,v:[u32;3])->u32 {self.inner.insert(m,p,key,v)}
}
impl<H: ResourceHost> ResourceHost for LiveFileServices<'_, H> {
    fn decode(&mut self,m:&mut Memory<'_>,a:[u32;4])->(u32,u32,u32) {
        let Some(g)=self.parser else {return self.inner.decode(m,a);};
        let saved=[m.word(g.outputs),m.word(g.outputs+4)];
        m.put_word(g.outputs,0);m.put_word(g.outputs+4,0);
        let rc=crate::boot_part_b2::fn_197c78(m,[a[0],a[1],a[2],a[3],g.outputs,g.outputs+4],g.table,g.scratch,self);
        let out=(rc,m.word(g.outputs),m.word(g.outputs+4));
        m.put_word(g.outputs,saved[0]);m.put_word(g.outputs+4,saved[1]);out
    }
}
impl<H: Host> DirectoryHost for LiveFileServices<'_, H> {
    fn join(&mut self,m:&mut Memory<'_>,base:u32,name:u32,flag:u32,heap:u32)->(u32,u32) {
        let s=self.scratch;let saved=m.word(s);m.put_word(s,0);
        let rc=self.join_into(m,s,base,name,flag,heap);let out=m.word(s);m.put_word(s,saved);(rc,out)
    }
    fn first(&mut self,m:&mut Memory<'_>,source:u32,heap:u32,pattern:u32,flags:u32)->(u32,u32,u32) {
        let s=self.scratch;let saved=[m.word(s),m.word(s+4)];m.put_word(s,0);m.put_word(s+4,0);
        let rc=self.first_into(m,source,heap,pattern,flags,s,s+4);
        let out=(rc,m.word(s),m.word(s+4));m.put_word(s,saved[0]);m.put_word(s+4,saved[1]);out
    }
    fn next(&mut self,m:&mut Memory<'_>,it:u32)->(u32,u32) {
        let s=self.scratch;let saved=m.word(s);m.put_word(s,0);
        let rc=self.next_into(m,it,s);let out=m.word(s);m.put_word(s,saved);(rc,out)
    }
}
impl<H: crate::boot_part_b2::AttributeHost + Host> crate::boot_part_b2::AttributeHost for LiveFileServices<'_, H> {
    fn create(&mut self,m:&mut Memory<'_>,ctx:u32,flag:u32,ticket:[u32;5],out:u32)->u32 {
        self.inner.create(m,ctx,flag,ticket,out)
    }
    fn attribute(&mut self,m:&mut Memory<'_>,object:u32,key:&[u8],value:[u32;3])->u32 {
        self.inner.attribute(m,object,key,value)
    }
}
impl<H: DiscoveryHost> DiscoveryHost for LiveFileServices<'_, H> {
    fn remove_property(&mut self,m:&mut Memory<'_>,object:u32,key:&[u8])->u32 {self.inner.remove_property(m,object,key)}
}
impl<H: QueryHost> QueryHost for LiveFileServices<'_, H> {
    fn query_output(&mut self,m:&mut Memory<'_>,f:u32,a:&[u32])->(u32,u32) {self.inner.query_output(m,f,a)}
    fn insert_record(&mut self,m:&mut Memory<'_>,db:u32,key:u32,record:[u32;4])->u32 {self.inner.insert_record(m,db,key,record)}
    fn open_pair(&mut self,m:&mut Memory<'_>,f:u32,owner:[u32;2],out:u32)->u32 {self.inner.open_pair(m,f,owner,out)}
}
impl<H: DataHost> DataHost for LiveFileServices<'_, H> {
    fn data_output(&mut self,m:&mut Memory<'_>,f:u32,a:&[u32],out:u32)->u32 {
        match f {
            0x173000 => self.open(m,a[0],a[1],a[2],out),
            _ => self.inner.data_output(m,f,a,out),
        }
    }
}

/// Caller-owned parser locals, disjoint from file and discovery scratch and allocations.
/// `scratch` needs 64 bytes plus 12 bytes per nested XML element.
#[derive(Clone, Copy)]
pub struct LiveParserGlobals { pub table:u32, pub scratch:u32, pub outputs:u32 }
/// Composition configuration. This is separate from the original entry's
/// constants so existing isolated callers keep their unchanged API.
#[derive(Clone, Copy)]
pub struct LiveFileGlobals<'a> {
    pub scratch: u32,
    pub rb: u32,
    pub archive_params: [u32; 4],
    pub parser: Option<LiveParserGlobals>,
    pub archive: Option<&'a crate::boot_part_e::ArchiveEnvironment>,
    pub unmap: Option<&'a crate::boot_part_e::UnmapEnvironment>,
}
/// 1795f0 with live allocator, source-open/close and archive-attach wrappers.
/// Dictionary/query/logger services remain explicit caller boundaries.
/// Both 17cbb4 and 179208 bind directly to the certified boot-part-e ports.
pub fn fn_1795f0_live(
    m:&mut Memory<'_>,broker:u32,name:u32,flags:u32,out:u32,
    globals:&DataOpenGlobals,files:&LiveFileGlobals,h:&mut impl DataHost,
)->u32 {
    let mut live=LiveFileServices {inner:h,scratch:files.scratch,rb:files.rb,archive_params:files.archive_params,parser:files.parser,archive:files.archive,unmap:files.unmap};
    fn_1795f0(m,broker,name,flags,out,globals,&mut live)
}
/// 17e0bc with live path join and directory iterator wrappers.
/// Executes the real boot-part-b2 decoder and recursive parser. No decoder stub is permitted.
pub fn fn_17e0bc_live(
    m:&mut Memory<'_>,context:u32,directory:u32,globals:&DiscoveryGlobals,
    files:&LiveFileGlobals,h:&mut impl DiscoveryHost,
)->u32 {
    let mut live=LiveFileServices {inner:h,scratch:files.scratch,rb:files.rb,archive_params:files.archive_params,parser:files.parser,archive:files.archive,unmap:files.unmap};
    assert!(files.parser.is_some(),"live discovery requires real parser configuration");
    fn_17e0bc(m,context,directory,globals,&mut live)
}
