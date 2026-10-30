//! Speech-one stream and component entry ports. All pointers are guest addresses.
use crate::{boot::Memory, fe_text, frontend, normalization};

/// Reentrant guest/platform calls receive the live caller-owned memory.
pub trait Host {
    fn call(&mut self, m: &mut Memory<'_>, method: u32, args: &[u32]) -> u32;
    fn get(
        &mut self,
        m: &mut Memory<'_>,
        method: u32,
        handle: u32,
        pos: u32,
        len: u32,
    ) -> (u32, u32, u32);
}
fn half(m: &Memory<'_>, p: u32) -> u16 {
    let i = m.offset(p);
    u16::from_le_bytes(m.bytes[i..i + 2].try_into().unwrap())
}
/// fn_17b7f0.
pub fn fn_17b7f0(m: &mut Memory<'_>, p: u32) -> u32 {
    if p == 0 { 0 } else { m.word(p + 12) }
}
/// fn_196980.
pub fn fn_196980(m: &mut Memory<'_>, a: u32, b: u32) -> u32 {
    if a == 0 {
        return if b == 0 { 0 } else { u32::MAX };
    }
    if b == 0 {
        return 1;
    }
    normalization::strcmp_at(m.bytes, m.offset(a) as u32, m.offset(b) as u32) as u32
}
/// fn_1770f8.
pub fn fn_1770f8(m: &mut Memory<'_>, a: u32, b: u32) -> u32 {
    if a == 0 {
        return if b == 0 { 0 } else { u32::MAX };
    }
    if b == 0 {
        return 1;
    }
    fe_text::strcmp_ci_1769d0(m.bytes, m.offset(a) as u32, m.offset(b) as u32) as u32
}
/// fn_172de0.
pub fn fn_172de0(m: &mut Memory<'_>, s: u32, v: u32, w: u32, h: &mut impl Host) -> u32 {
    if m.word(s + 12) != 0 {
        m.put_word(s + 28, v);
        return 0;
    }
    let inner = m.word(m.word(s) + 8);
    h.call(m, m.word(inner + 12), &[m.word(s + 16), v, w, 1])
}
/// fn_172d30. The get hook returns the callee's two output words.
pub fn fn_172d30(
    m: &mut Memory<'_>,
    dst: u32,
    n: u32,
    count: u32,
    s: u32,
    h: &mut impl Host,
) -> u32 {
    if m.word(s + 12) != 0 {
        let inner = m.word(m.word(s) + 8);
        let (rc, len, ptr) = h.get(
            m,
            m.word(inner + 48),
            m.word(s + 12),
            m.word(s + 28),
            n.wrapping_mul(count),
        );
        if rc < 0x80000000 {
            let d = m.offset(dst) as u32;
            let p = m.offset(ptr) as u32;
            fe_text::memcpy_195ae8(m.bytes, d, p, len);
            m.put_word(s + 28, m.word(s + 28).wrapping_add(len));
            let inner = m.word(m.word(s) + 8);
            if h.call(m, m.word(inner + 52), &[m.word(s + 12), ptr]) < 0x80000000 {
                return len;
            }
        }
        0
    } else {
        let inner = m.word(m.word(s) + 8);
        h.call(m, m.word(inner + 8), &[dst, n, count, m.word(s + 16)])
    }
}
/// fn_1b2760.
pub fn fn_1b2760(m: &mut Memory<'_>, s: u32, offset: u32, h: &mut impl Host) -> u32 {
    let length = m.word(s + 28);
    if offset > length {
        return normalization::code_4(0);
    }
    let block = half(m, s + 24) as u32;
    let mut pos = offset;
    let mut rem = length - offset;
    m.put_word(s + 48, rem);
    if block != 0 {
        let idx = normalization::udiv(offset, block);
        pos = pos.wrapping_add(idx.wrapping_mul(4));
        let next = block.wrapping_mul(idx.wrapping_add(1)).wrapping_sub(offset);
        rem = rem.min(next);
        m.put_word(s + 52, rem);
    }
    let method = m.word(m.word(s + 36) + 4);
    if h.call(
        m,
        method,
        &[m.word(s + 40), m.word(s + 20).wrapping_add(pos), 0],
    ) != 0
    {
        normalization::code_6(0)
    } else {
        0
    }
}

/// Stream callbacks may target an arena buffer or a four-byte C stack local.
/// A None destination represents that local, never a discarded arena write.
pub trait StreamHost: Host {
    fn read(
        &mut self,
        m: &mut Memory<'_>,
        method: u32,
        dst: Option<u32>,
        n: u32,
        stream: u32,
    ) -> u32;
}
fn stream_read(
    m: &mut Memory<'_>,
    p: u32,
    mut b: u32,
    n: u32,
    h: &mut impl StreamHost,
) -> (u32, Option<u32>) {
    let mut left = n;
    if half(m, p + 24) != 0 {
        let mut total = 0u32;
        while left != 0 {
            let chunk = left.min(m.word(p + 52));
            let got = h.read(m, m.word(m.word(p + 36)), Some(b), chunk, m.word(p + 40));
            if got != chunk {
                let eof = h.call(m, m.word(m.word(p + 36) + 8), &[m.word(p + 40)]);
                return (
                    if eof != 0 {
                        normalization::code_6(0)
                    } else {
                        normalization::code_4(0)
                    },
                    None,
                );
            }
            let rem = m.word(p + 52).wrapping_sub(got);
            m.put_word(p + 52, rem);
            if rem == 0 {
                if h.read(m, m.word(m.word(p + 36)), None, 4, m.word(p + 40)) != 4 {
                    let eof = h.call(m, m.word(m.word(p + 36) + 8), &[m.word(p + 40)]);
                    return (
                        if eof != 0 {
                            normalization::code_6(0)
                        } else {
                            normalization::code_4(0)
                        },
                        None,
                    );
                }
                let lim = half(m, p + 24) as u32;
                let rem = m.word(p + 48).wrapping_sub(total).wrapping_sub(got);
                m.put_word(p + 52, lim.min(rem));
            }
            left = left.wrapping_sub(got);
            b = b.wrapping_add(got);
            total = total.wrapping_add(got);
        }
        m.put_word(p + 48, m.word(p + 48).wrapping_sub(total));
        (0, Some(total))
    } else {
        unreachable!("chunk reader requires chunked stream")
    }
}
/// fn_1b2974.
pub fn fn_1b2974(
    m: &mut Memory<'_>,
    p: u32,
    b: u32,
    n: u32,
    out: u32,
    h: &mut impl StreamHost,
) -> u32 {
    if half(m, p + 24) == 0 {
        let got = h.read(m, m.word(m.word(p + 36)), Some(b), n, m.word(p + 40));
        m.put_word(out, got);
        if got != n && h.call(m, m.word(m.word(p + 36) + 8), &[m.word(p + 40)]) != 0 {
            return normalization::code_6(0);
        }
        return 0;
    }
    let (rc, got) = stream_read(m, p, b, n, h);
    if let Some(got) = got {
        m.put_word(out, got);
    }
    rc
}
/// fn_1b254c.
pub fn fn_1b254c(
    m: &mut Memory<'_>,
    p: u32,
    b: u32,
    n: u32,
    scratch: u32,
    h: &mut impl StreamHost,
) -> u32 {
    if m.word(p + 48) < n {
        return normalization::code_4(0);
    }
    let saved = m.word(scratch);
    let rc = fn_1b2974(m, p, b, n, scratch, h);
    m.put_word(scratch, saved);
    rc
}

/// Read-only literal inputs supplied from the caller's regional image.
pub struct ComponentEnvironment {
    pub cached_handle: [u8; 8],
    pub input_types: [u32; 2],
}
fn reset_handle(m: &mut Memory<'_>, p: u32, env: &ComponentEnvironment) {
    let mut pair = [0u8; 8];
    frontend::load_pair(&mut pair, &env.cached_handle);
    m.put_word(p, u32::from_le_bytes(pair[..4].try_into().unwrap()));
    m.put_word(p + 4, u32::from_le_bytes(pair[4..].try_into().unwrap()));
}
/// fn_1d7544.
pub fn fn_1d7544(m: &mut Memory<'_>, p: u32, tag: u32, h: &mut impl Host) -> u32 {
    let mut rc = frontend::validate_handle(p != 0, tag, 0xf380, 0x4c);
    if rc >= 0x80000000 {
        return 0x89902008;
    }
    for i in 0..2 {
        let at = p + 32 + i * 8;
        if m.word(at) != 0 {
            rc = h.call(
                m,
                m.word(m.word(p + 28) + 36),
                &[m.word(at), m.word(at + 4)],
            );
            m.put_word(at, 0);
            m.put_word(at + 4, 0);
        }
    }
    rc
}
/// fn_1e3e28.
pub fn fn_1e3e28(
    m: &mut Memory<'_>,
    p: u32,
    tag: u32,
    env: &ComponentEnvironment,
    h: &mut impl Host,
) -> u32 {
    let mut rc = frontend::validate_handle(p != 0, tag, 0x72a5, 0x2c);
    if rc >= 0x80000000 {
        return 0x8a702008;
    }
    for i in 0..2 {
        let at = p + 28 + i * 8;
        if m.word(at) != 0 {
            let r = h.call(
                m,
                m.word(m.word(p + 24) + 36),
                &[m.word(at), m.word(at + 4)],
            );
            if rc < 0x80000000 && r >= 0x80000000 {
                rc = r;
            }
            reset_handle(m, at, env);
        }
    }
    rc
}
/// fn_1d513c.
pub fn fn_1d513c(
    m: &mut Memory<'_>,
    p: u32,
    tag: u32,
    source: [u32; 2],
    env: &ComponentEnvironment,
    h: &mut impl Host,
) -> u32 {
    let mut rc = frontend::validate_handle(p != 0, tag, 0xf380, 0x4c);
    if rc >= 0x80000000 {
        return 0x89902008;
    }
    let mut count = 0;
    while count < 2 {
        let method = m.word(m.word(p + 28) + 32);
        rc = h.call(
            m,
            method,
            &[
                source[0],
                source[1],
                14,
                env.input_types[count],
                0,
                0,
                p + 32 + count as u32 * 8,
            ],
        );
        if rc >= 0x80000000 {
            break;
        }
        count += 1;
    }
    if rc >= 0x80000000 {
        for i in 0..count {
            let at = p + 32 + i as u32 * 8;
            if m.word(at) != 0 {
                h.call(
                    m,
                    m.word(m.word(p + 28) + 36),
                    &[m.word(at), m.word(at + 4)],
                );
                m.put_word(at, 0);
                m.put_word(at + 4, 0);
            }
        }
    }
    rc
}

struct MutexHost<'a, H>(&'a mut H);
pub trait HeapHost: Host {
    fn thread_id(&mut self) -> u32;
    fn arbiter_address(&self) -> u32;
}
impl<H: HeapHost> crate::boot::Calls for MutexHost<'_, H> {
    fn call(&mut self, m: &mut Memory<'_>, f: u32, a: &[u32]) -> u32 {
        self.0.call(m, f, a)
    }
    fn call_wide(&mut self, _: &mut Memory<'_>, _: u32, _: &[u32]) -> u64 {
        unreachable!("mutex operations do not call wide-return services")
    }
}
impl<H: HeapHost> crate::boot_part_b::Host for MutexHost<'_, H> {
    fn thread_id(&mut self) -> u32 {
        self.0.thread_id()
    }
    fn call(&mut self, m: &mut Memory<'_>, f: u32, a: &[u32]) -> u32 {
        match f {
            0x184f84 => {
                m.boot_heap_release(a[0], a[1]);
                0
            }
            0x183edc => {
                crate::boot_part_b2::fn_183edc(m, a[0], m.word(self.0.arbiter_address()), self);
                0
            }
            0x165d00 => {
                let g = crate::boot::Globals {
                    manager: 0,
                    callbacks: 0,
                    rules: 0,
                    components: 0,
                    dictionary: 0,
                    context: 0,
                    mode: 0,
                    semaphore: self.0.arbiter_address(),
                    service: 0,
                    command: 0,
                };
                m.fn_165d00(self, &g, a[0]);
                0
            }
            _ => self.0.call(m, f, a),
        }
    }
}
/// fn_11da30. table and optional_flag are regional global addresses.
pub fn fn_11da30(
    m: &mut Memory<'_>,
    x: u32,
    table: u32,
    optional_flag: u32,
    h: &mut impl HeapHost,
) {
    assert_eq!(m.base, 0, "guest heap needs zero-based caller workspace");
    for i in 0..3 {
        if i == 2 && m.byte(optional_flag + 1) == 0 {
            break;
        }
        let object = m.word(table + i * 4);
        if h.call(m, m.word(m.word(object) + 24), &[object, x]) != 0 {
            // The accepts callback may change the selected global slot.
            let object = m.word(table + i * 4);
            let mut host = MutexHost(h);
            crate::boot_part_b::fn_1840fc(m, object + 88, &mut host);
            crate::boot_part_b2::fn_19b78c(m, object, x, &mut host);
            crate::boot_part_b::fn_18423c(m, object + 88, &mut host);
            break;
        }
    }
}
/// fn_11db30. The real existing guest heap owns all block writes.
pub fn fn_11db30(
    m: &mut Memory<'_>,
    a: u32,
    index: u32,
    b: u32,
    c: u32,
    table: u32,
    h: &mut impl HeapHost,
) -> u32 {
    assert_eq!(m.base, 0, "guest heap needs zero-based caller workspace");
    let object = m.word(table + index * 4);
    crate::boot_part_b::fn_1840fc(m, object + 88, &mut MutexHost(h));
    let result = m.boot_heap_allocate([object, a, b, c, 0, 0]);
    crate::boot_part_b::fn_18423c(m, object + 88, &mut MutexHost(h));
    result
}

pub struct ArchiveEnvironment {
    pub dot: u32,
    pub parent: u32,
    pub locale: u32,
}
struct Resolve {
    path: u32,
    locale: u32,
}
impl crate::boot_part_b::Host for Resolve {
    fn thread_id(&mut self) -> u32 {
        unreachable!()
    }
    fn call(&mut self, m: &mut Memory<'_>, f: u32, a: &[u32]) -> u32 {
        assert_eq!(f, 0x19de08);
        crate::boot_part_b2::fn_19de08(m, a[0], self.path, self.locale)
    }
}
fn chdir(m: &mut Memory<'_>, archive: u32, path: u32, locale: u32) -> u32 {
    crate::boot_part_b::fn_19ddd0(m, archive, &mut Resolve { path, locale })
}
/// fn_16b554. scratch needs 80 bytes plus 32 per recursive directory level.
/// Temporary bytes are restored, as for the C stack.
pub fn fn_16b554(
    m: &mut Memory<'_>,
    list: u32,
    group: u32,
    name: u32,
    size: u32,
    scratch: u32,
    env: &ArchiveEnvironment,
) -> u32 {
    let at = m.offset(scratch);
    let mut saved = [0u8; 64];
    saved.copy_from_slice(&m.bytes[at..at + 64]);
    let result = archive_find(m, list, group, name, size, scratch, env);
    m.bytes[at..at + 64].copy_from_slice(&saved);
    result
}
fn archive_find(
    m: &mut Memory<'_>,
    list: u32,
    group: u32,
    name: u32,
    size: u32,
    scratch: u32,
    env: &ArchiveEnvironment,
) -> u32 {
    let mut len = 0;
    while len < 63 {
        let c = m.byte(name + len) as i8;
        if c == 0 {
            break;
        }
        m.put_half(list + 40 + len * 2, c as i16 as u16);
        len += 1;
    }
    m.put_half(list + 40 + len * 2, 0);
    let mut link = m.word(list + 8);
    while link != list + 8 {
        let archive = link + 8;
        let path = link + 36;
        if crate::boot_part_b2::fn_19de08(m, archive, path, env.locale) != u32::MAX
            && chdir(m, archive, path, env.locale) != 0
        {
            let mut index = u32::MAX;
            if group != 0 {
                for i in 0..4 {
                    m.put_half(scratch + i * 2, ((group >> (24 - i * 8)) & 255) as u16);
                }
                m.put_half(scratch + 8, 0);
                if crate::boot_part_b2::fn_19de08(m, archive, scratch, env.locale) != u32::MAX
                    && chdir(m, archive, scratch, env.locale) != 0
                {
                    index = crate::boot_part_b2::fn_19de08(m, archive, list + 40, env.locale);
                    chdir(m, archive, env.parent, env.locale);
                }
            } else {
                crate::boot_part_b2::fn_19dcd8(m, archive, env.dot, scratch + 32, env.locale);
                while crate::boot_part_b::fn_19dd30(m, scratch + 32, scratch + 48) != 0 {
                    let entry_name = m.word(scratch + 56);
                    if m.byte(scratch + 60) != 0 {
                        chdir(m, archive, entry_name, env.locale);
                        index = crate::boot_part_b2::fn_189040(
                            m,
                            archive,
                            list + 40,
                            env.dot,
                            env.parent,
                            env.locale,
                            scratch + 80,
                        );
                        chdir(m, archive, env.parent, env.locale);
                        if index != u32::MAX {
                            break;
                        }
                    } else if m.wide_case_compare(env.locale, list + 40, entry_name) == 0 {
                        index = m.word(scratch + 52);
                        break;
                    }
                }
            }
            chdir(m, archive, env.parent, env.locale);
            if index != u32::MAX {
                crate::boot_part_b::fn_188f9c(m, archive, index as i32, scratch + 16);
                let result = crate::boot_part_b::fn_189030(m, scratch + 16);
                if size != 0 {
                    m.put_word(size, m.word(scratch + 24));
                }
                if result != 0 {
                    return result;
                }
            }
        }
        link = m.word(link);
    }
    0
}

struct PlatformAdapter<'a, 'h, H>(&'a core::cell::RefCell<&'h mut H>);
impl<H: HeapHost> crate::bridge::Platform for PlatformAdapter<'_, '_, H> {
    fn thread_id(&mut self) -> u32 {
        self.0.borrow_mut().thread_id()
    }
    fn arbiter_address(&self) -> u32 {
        self.0.borrow().arbiter_address()
    }
    fn arbitrate(&mut self, m: &mut Memory<'_>, a: &[u32]) -> u32 {
        self.0.borrow_mut().call(m, 0x172d18, a)
    }
}
struct LiveAdapter<'a, 'h, H>(&'a core::cell::RefCell<&'h mut H>);
impl<H: HeapHost> crate::bridge::Live for LiveAdapter<'_, '_, H> {
    fn call<P: crate::bridge::Platform>(
        &mut self,
        b: &mut crate::bridge::Flat<'_, '_, '_, '_, P>,
        f: u32,
        a: &[u32],
    ) -> u32 {
        // No arena capture or nested Flat: the callback sees the live staged bytes.
        if f == 0x1749ac {
            fn_1749ac(&mut b.memory(), a[0], a[1], &mut **self.0.borrow_mut());
            0
        } else {
            self.0.borrow_mut().call(&mut b.memory(), f, a)
        }
    }
    fn log<P: crate::bridge::Platform>(
        &mut self,
        _: &mut crate::bridge::Flat<'_, '_, '_, '_, P>,
        _: u32,
        _: u32,
        _: &[u32],
    ) {
        unreachable!("tree teardown emits no log")
    }
}
/// fn_174b14. Outermost entry with caller-owned bridge workspace.
/// Existing Live callbacks must use fn_174b14_live without staging another Flat.
pub fn fn_174b14(m: &mut Memory<'_>, s: u32, workspace: &mut [u8], h: &mut impl HeapHost) {
    let cell = core::cell::RefCell::new(h);
    let mut platform = PlatformAdapter(&cell);
    let mut live = LiveAdapter(&cell);
    let mut regions = [crate::engine::Region {
        base: m.base,
        bytes: m.bytes,
    }];
    let mut engine = crate::engine::Memory::new(&mut regions).expect("guest region");
    let mut flat = crate::bridge::Flat::new(&mut engine, workspace, &mut platform, 0)
        .expect("caller workspace");
    fn_174b14_live(&mut flat, s, &mut live);
    flat.finish().expect("live map teardown");
}
/// Composition entry on the caller's already-live bridge.
pub(crate) fn fn_174b14_live<P: crate::bridge::Platform>(
    flat: &mut crate::bridge::Flat<'_, '_, '_, '_, P>,
    s: u32,
    live: &mut impl crate::bridge::Live,
) {
    if flat.word(s + 4) != 0 {
        flat.fn_19665c_with(flat.word(s + 4), live);
        flat.put_word(s + 4, 0);
    }
}
/// fn_174aac. ticket contains the five regional words of D_8d7614.
/// The legacy bridge's fn_19658c clone remains an explicit host dependency.
/// This entry is outside the round-3 call trees; speech_part_6 has a live clone port.
pub fn fn_174aac(
    m: &mut Memory<'_>,
    src: u32,
    arg: u32,
    out: u32,
    ticket: [u32; 5],
    workspace: &mut [u8],
    h: &mut impl HeapHost,
) -> u32 {
    let cell = core::cell::RefCell::new(h);
    let mut platform = PlatformAdapter(&cell);
    let mut live = LiveAdapter(&cell);
    let mut regions = [crate::engine::Region {
        base: m.base,
        bytes: m.bytes,
    }];
    let mut engine = crate::engine::Memory::new(&mut regions).expect("guest region");
    let mut flat = crate::bridge::Flat::new(&mut engine, workspace, &mut platform, 0)
        .expect("caller workspace");
    let rc = fn_174aac_live(&mut flat, src, arg, out, ticket, &mut live);
    flat.finish().expect("live attribute create");
    rc
}
/// Composition entry. The existing live bridge publishes only at its outer boundary.
pub(crate) fn fn_174aac_live<P: crate::bridge::Platform>(
    flat: &mut crate::bridge::Flat<'_, '_, '_, '_, P>,
    src: u32,
    arg: u32,
    out: u32,
    ticket: [u32; 5],
    live: &mut impl crate::bridge::Live,
) -> u32 {
    flat.put_word(out, flat.word(src));
    let old = flat.word(src + 4);
    let rc = if old != 0 {
        crate::bridge::Live::call(live, flat, 0x19658c, &[old, out + 4])
    } else {
        flat.fn_1964e0([
            arg,
            0,
            ticket[0],
            ticket[1],
            ticket[2],
            ticket[3],
            ticket[4],
            out + 4,
        ])
    };
    u32::from(rc < 0x80000000)
}

/// fn_17b9d4. Append a string after real reentrant capacity growth.
pub fn fn_17b9d4(
    m: &mut Memory<'_>,
    p: u32,
    text: u32,
    h: &mut impl crate::frontend::HeapVtableM,
) -> u32 {
    if p == 0 || text == 0 {
        return 0x8a402007;
    }
    let n = normalization::strlen8(m.bytes, m.offset(text) as u32);
    let rc = crate::render_part_b::fn_17b53c_m(m, p, m.word(p + 8).wrapping_add(n) as i32, h);
    if rc < 0x80000000 {
        let dst = m.word(p + 12).wrapping_add(m.word(p + 8));
        let d = m.offset(dst) as u32;
        let src = m.offset(text) as u32;
        crate::text_input::strncpy_at(m.bytes, d, src, n);
        m.put_word(p + 8, m.word(p + 8).wrapping_add(n));
        let end = m.offset(m.word(p + 12).wrapping_add(m.word(p + 8)));
        m.bytes[end] = 0;
    }
    rc
}

struct ReleaseMethod<'a, H> {
    base: u32,
    method: u32,
    host: &'a mut H,
}
impl<H: Host> crate::frontend::HeapVtableM for ReleaseMethod<'_, H> {
    fn alloc_at_m(&mut self, _: u32, _: u32, _: &mut [u8]) -> u32 {
        panic!("release-only adapter")
    }
    fn realloc_at_m(&mut self, _: u32, _: u32, _: u32, _: &mut [u8]) -> u32 {
        panic!("release-only adapter")
    }
    fn release_at_m(&mut self, obj: u32, p: u32, bytes: &mut [u8]) {
        self.host.call(
            &mut Memory {
                base: self.base,
                bytes,
            },
            self.method,
            &[obj, p],
        );
    }
}
fn release_live(m: &mut Memory<'_>, heap: u32, p: u32, h: &mut impl Host) {
    if heap == 0 || p == 0 {
        return;
    }
    let method = m.word(m.word(heap) + 12);
    let base = m.base;
    let offset = m.offset(heap) as u32;
    crate::frontend::heap_release_m(
        m.bytes,
        Some(offset),
        Some(p),
        &mut ReleaseMethod {
            base,
            method,
            host: h,
        },
    );
}
struct ArrayHost<'a, H>(&'a mut H);
impl<H: Host> crate::reentrant2::HooksM for ArrayHost<'_, H> {
    fn call_m(&mut self, f: u32, a: &[u32], m: &mut Memory<'_>) -> u32 {
        if f == 0x196428 {
            release_live(m, a[0], a[1], self.0);
            0
        } else if f == 0x1749ac {
            fn_1749ac(m, a[0], a[1], self.0);
            0
        } else {
            self.0.call(m, f, a)
        }
    }
}
/// fn_1749ac. Calls the landed reentrant array teardown and real heap release.
pub fn fn_1749ac(m: &mut Memory<'_>, p: u32, owner: u32, h: &mut impl Host) {
    if p == 0 {
        return;
    }
    if m.word(p + 8) != 0 {
        crate::reentrant2::destroy_array_m(m, m.word(p + 8), &mut ArrayHost(h));
        m.put_word(p + 8, 0);
    }
    if m.word(p) != 0 {
        release_live(m, owner, m.word(p), h);
        m.put_word(p, 0);
    }
    m.put_word(p + 4, 0);
    release_live(m, owner, p, h);
}

/// fn_1d453c. The 1758ac setter is a marked bridge-4 pending host stub.
pub fn fn_1d453c(m: &mut Memory<'_>, p: u32, tag: u32, key: u32, h: &mut impl Host) -> u32 {
    let mut rc = frontend::validate_handle(p != 0, tag, 0xf3c0, 0x358);
    if rc >= 0x80000000 {
        return 0x8bd02008;
    }
    if m.byte(p + 0x41) == 1 {
        // STUB bridge-4: replace when a live set_param_uint_m lands.
        h.call(
            m,
            0x1758ac,
            &[m.word(m.word(p) + 0x14), key, m.word(p + 0xb8)],
        );
        let at = m.offset(p + 0x41);
        m.bytes[at] = 0;
    }
    for start in [0x20, 0x30] {
        for i in 0..2 {
            let pair = p + start + i * 8;
            if m.word(pair) != 0 {
                rc = h.call(
                    m,
                    m.word(m.word(p + 0x10) + 0x24),
                    &[m.word(pair), m.word(pair + 4)],
                );
                m.put_word(pair, 0);
                m.put_word(pair + 4, 0);
            }
        }
    }
    rc
}

/// fn_1e4a3c. Scratch-free wrapper; 2401a0 is the bridge-4 pending host stub.
pub fn fn_1e4a3c(
    m: &mut Memory<'_>,
    p: u32,
    tag: u32,
    category: u32,
    h: &mut impl crate::speech_part_6::Host,
) -> u32 {
    if frontend::validate_handle(p != 0, tag, 0xe58c, 8) >= 0x80000000 {
        return 0x89602008;
    }
    let t = m.word(p);
    if frontend::validate_handle(t != 0, m.word(p + 4), 0xe58d, 0xf4) >= 0x80000000 {
        return 0x89602008;
    }
    h.log(
        m,
        m.word(m.word(t) + 16),
        m.word(t + 0x48),
        4,
        b"tokentn : ProcessStart Begin",
        &[],
    );
    if half(m, t + 0x6c) == 0 {
        h.error(m, m.word(m.word(t) + 16), category, 0x5dd8);
        return 0x89602000;
    }
    // STUB bridge-4: guest PCRE initialization needs a live adapter.
    let rc = h.call(
        m,
        0x2401a0,
        &[
            m.word(t + 0x30),
            m.word(t + 0x34),
            half(m, t + 0x4e) as u32,
            half(m, t + 0x50) as u32,
        ],
    );
    if rc >= 0x80000000 {
        h.error(m, m.word(m.word(t) + 16), m.word(t + 0x48), 0x5df3);
    }
    h.log(
        m,
        m.word(m.word(t) + 16),
        m.word(t + 0x48),
        4,
        b"tokentn : ProcessStart End (%x)",
        &[rc],
    );
    rc
}

struct CallHost<'a, 'h, H>(&'a core::cell::RefCell<&'h mut H>);
impl<H: Host> crate::boot::Calls for CallHost<'_, '_, H> {
    fn call(&mut self, m: &mut Memory<'_>, f: u32, a: &[u32]) -> u32 {
        self.0.borrow_mut().call(m, f, a)
    }
    fn call_wide(&mut self, _: &mut Memory<'_>, _: u32, _: &[u32]) -> u64 {
        panic!("no wide call in parameter lookup")
    }
}
struct ParamLock<'a, 'h, H> {
    cell: &'a core::cell::RefCell<&'h mut H>,
    object: u32,
    scratch: u32,
}
impl<H: Host> ParamLock<'_, '_, H> {
    fn run(&mut self, bytes: &mut [u8], f: u32) -> u32 {
        let mut m = Memory { base: 0, bytes };
        let owner = m.word(self.object + 4);
        m.boot_parameter_call(&mut CallHost(self.cell), f, &[owner], self.scratch)
            .expect("real resource lock")
    }
}
impl<H: Host> crate::reentrant2::StateLockM for ParamLock<'_, '_, H> {
    fn acquire_m(&mut self, bytes: &mut [u8]) -> u32 {
        self.run(bytes, 0x176f2c)
    }
    fn release_m(&mut self, bytes: &mut [u8]) -> u32 {
        self.run(bytes, 0x176f5c)
    }
}
fn uint_property(
    m: &mut Memory<'_>,
    object: u32,
    key: u32,
    out: u32,
    scratch: u32,
    h: &mut impl Host,
) -> u32 {
    assert_eq!(
        m.base, 0,
        "parameter adapters require a zero-based guest view"
    );
    let cell = core::cell::RefCell::new(h);
    let mut lock = ParamLock {
        cell: &cell,
        object,
        scratch,
    };
    crate::reentrant2::get_param_uint_m(
        m.bytes,
        if object == 0 { None } else { Some(object) },
        key,
        out,
        &mut lock,
        |a, b, bytes| {
            let mut m = Memory { base: 0, bytes };
            let map = m.word(object + 8);
            let f = m.word(m.word(map + 60) + 8);
            if f == 0x196980 {
                fn_196980(&mut m, a, b) as i32
            } else {
                cell.borrow_mut().call(&mut m, f, &[a, b]) as i32
            }
        },
    )
}
fn int_property(
    m: &mut Memory<'_>,
    object: u32,
    key: u32,
    out: u32,
    scratch: u32,
    table: &[u16; 256],
    h: &mut impl Host,
) -> u32 {
    assert_eq!(m.base, 0);
    let cell = core::cell::RefCell::new(h);
    let mut lock = ParamLock {
        cell: &cell,
        object,
        scratch,
    };
    crate::reentrant2::get_param_int_m(
        table,
        m.bytes,
        if object == 0 { None } else { Some(object) },
        key,
        out,
        &mut lock,
        |a, b, bytes| {
            let mut m = Memory { base: 0, bytes };
            let map = m.word(object + 8);
            let f = m.word(m.word(map + 60) + 8);
            if f == 0x196980 {
                fn_196980(&mut m, a, b) as i32
            } else {
                cell.borrow_mut().call(&mut m, f, &[a, b]) as i32
            }
        },
    )
}
pub struct MarkerEnvironment {
    pub mode: u32,
    pub enabled: u32,
    pub mime: u32,
    pub category: u32,
    pub format: u32,
    pub content_type: u32,
    pub ctype: [u16; 256],
}
/// fn_1e3938. scratch is 136 disjoint restored bytes; parameter methods are live.
pub fn fn_1e3938<H: Host + crate::speech_part_6::Host>(
    m: &mut Memory<'_>,
    p: u32,
    tag: u32,
    source: [u32; 2],
    env: &MarkerEnvironment,
    scratch: u32,
    h: &mut H,
) -> u32 {
    if frontend::validate_handle(p != 0, tag, 0x72a5, 0x2c) >= 0x80000000 {
        return 0x8a702008;
    }
    if m.word(p + 16) == 0 {
        return 0;
    }
    let at = m.offset(scratch);
    let mut saved = [0; 136];
    saved.copy_from_slice(&m.bytes[at..at + 136]);
    m.put_word(scratch, 0);
    m.put_word(scratch + 4, 1);
    let r = (|| {
        let rc = uint_property(
            m,
            m.word(m.word(p + 8) + 20),
            env.mode,
            scratch,
            scratch + 8,
            h,
        );
        if rc >= 0x80000000 {
            return rc;
        }
        if m.word(scratch) & 4 == 0 {
            return 0;
        }
        let rc = int_property(
            m,
            m.word(m.word(p + 8) + 20),
            env.enabled,
            scratch + 4,
            scratch + 8,
            &env.ctype,
            h,
        );
        if rc < 0x80000000 && m.word(scratch + 4) == 0 {
            return 0;
        }
        let mut rc = Host::call(
            h,
            m,
            m.word(m.word(p + 24) + 32),
            &[source[0], source[1], 0xa7, env.mime, 0, 0, p + 28],
        );
        let code = if rc >= 0x80000000 {
            0xb02f
        } else {
            let current = Host::call(h, m, m.word(m.word(p + 24) + 76), &[]);
            rc = Host::call(
                h,
                m,
                m.word(m.word(p + 24) + 32),
                &[source[0], source[1], 0xa7, env.mime, current, 1, p + 36],
            );
            0xb030
        };
        if rc >= 0x80000000 {
            Host::call(
                h,
                m,
                0x173dc0,
                &[
                    m.word(m.word(p + 8) + 16),
                    env.category,
                    code,
                    env.format,
                    env.content_type,
                    env.mime,
                ],
            );
        }
        rc
    })();
    m.bytes[at..at + 136].copy_from_slice(&saved);
    r
}

struct NormHost<'a, H>(&'a mut H);
struct NormHooks<'a, H>(&'a mut H);
impl<H: crate::speech_part_6::Host> crate::reentrant2::HooksM for NormHooks<'_, H> {
    fn call_m(&mut self, f: u32, a: &[u32], m: &mut Memory<'_>) -> u32 {
        self.0.call(m, f, a)
    }
}
impl<H: Host + crate::speech_part_6::Host> normalization::NormalizationHost for NormHost<'_, H> {
    fn calloc(&mut self, bytes: &mut [u8], heap: u32, n: u32, size: u32) -> Option<u32> {
        let mut m = Memory { base: 0, bytes };
        let p = crate::reentrant2::call_method_m(&mut m, heap, n, size, &mut NormHooks(self.0));
        if p == 0 { None } else { Some(p) }
    }
    fn alloc(&mut self, _: &mut [u8], _: u32, _: u32) -> Option<u32> {
        unreachable!()
    }
    fn realloc(&mut self, _: &mut [u8], _: u32, _: u32, _: u32) -> Option<u32> {
        unreachable!()
    }
    fn free(&mut self, bytes: &mut [u8], heap: u32, p: u32) {
        release_live(&mut Memory { base: 0, bytes }, heap, p, self.0);
    }
    fn resolve(&mut self, _: &mut [u8], _: u32, _: u32) -> (u32, u32) {
        unreachable!()
    }
}
fn token_oom(m: &mut Memory<'_>, p: u32, h: &mut impl crate::speech_part_6::Host) {
    h.error(m, m.word(m.word(p) + 16), m.word(p + 0x48), 0x5df0);
}
const TOKEN_BUFFERS: [u32; 7] = [0x88, 0x94, 0xa0, 0xcc, 0xd8, 0xe4, 0xac];
fn token_prepare<H: Host + crate::speech_part_6::Host>(
    m: &mut Memory<'_>,
    p: u32,
    h: &mut H,
) -> u32 {
    for off in TOKEN_BUFFERS {
        let r = normalization::alloc_destination(m.bytes, p, p + off, &mut NormHost(h));
        if r >= 0x80000000 {
            token_oom(m, p, h);
            return r;
        }
    }
    for i in 0..3 {
        let r = match i {
            0 => normalization::alloc_tables(m.bytes, p, &mut NormHost(h)),
            1 => normalization::alloc_record_second(m.bytes, p, &mut NormHost(h)),
            _ => normalization::alloc_record(m.bytes, p, &mut NormHost(h)),
        };
        if r >= 0x80000000 {
            token_oom(m, p, h);
            return r;
        }
    }
    for (off, n) in [(0x64, 20), (0x68, 24)] {
        let heap = m.word(m.word(p) + 4);
        let v = crate::reentrant2::call_method_m(m, heap, 1, n, &mut NormHooks(h));
        m.put_word(p + off, v);
        if v == 0 {
            token_oom(m, p, h);
            return 0x8960200a;
        }
    }
    0
}
fn token_cleanup<H: Host + crate::speech_part_6::Host>(m: &mut Memory<'_>, p: u32, h: &mut H) {
    if m.word(p + 0xc8) != 0 {
        release_live(m, m.word(m.word(p) + 4), m.word(p + 0xc8), h);
        m.put_word(p + 0xc8, 0);
    }
    for off in TOKEN_BUFFERS {
        let ctx = m.word(p);
        normalization::release_buffer(m.bytes, ctx, p + off, &mut NormHost(h));
    }
    normalization::release_pair(m.bytes, p, &mut NormHost(h));
    normalization::release_owned_second(m.bytes, p, &mut NormHost(h));
    normalization::release_owned(m.bytes, p, &mut NormHost(h));
    for off in [0x64, 0x68] {
        if m.word(p + off) != 0 {
            release_live(m, m.word(m.word(p) + 4), m.word(p + off), h);
            m.put_word(p + off, 0);
        }
    }
}
/// fn_1e4b7c. Calls real sentence, heap and normalization ports.
/// The host binds rule/diagnostic calls to fn_249cec and fn_24ac00 below.
/// scratch is eight disjoint bytes, restored by the sentence callee.
pub fn fn_1e4b7c<H: Host + crate::speech_part_6::Host>(
    m: &mut Memory<'_>,
    handle: u32,
    check: u32,
    args: [u32; 2],
    processed: u32,
    end_name: u32,
    scratch: u32,
    h: &mut H,
) -> u32 {
    assert_eq!(m.base, 0);
    if frontend::validate_handle(handle != 0, check, 0xe58c, 8) >= 0x80000000 {
        return 0x89602008;
    }
    let p = m.word(handle);
    if frontend::validate_handle(p != 0, m.word(handle + 4), 0xe58d, 0xf4) >= 0x80000000 {
        return 0x89602008;
    }
    crate::speech_part_6::Host::log(
        h,
        m,
        m.word(m.word(p) + 16),
        m.word(p + 0x48),
        4,
        b"tokentn : Process Begin",
        &[],
    );
    m.put_word(p + 0x14, args[1]);
    m.put_word(p + 0x10, args[0]);
    let logger = m.word(m.word(p) + 16);
    let level = Host::call(h, m, 0x173fd8, &[logger]);
    let at = m.offset(p + 0xc6);
    m.bytes[at] = u8::from(level >= 5);
    let mut r = crate::speech_part_6::fn_24c9fc(m, h, p, scratch);
    if r >= 0x80000000 {
        return r;
    }
    let mut finished = true;
    if m.word(p + 0x5c) != 0 {
        crate::speech_part_6::Host::log(
            h,
            m,
            m.word(m.word(p) + 16),
            m.word(p + 0x48),
            5,
            b"<LOGFE> processing sentence : _%s_",
            &[m.word(p + 0x5c)],
        );
        let n = normalization::strlen8(m.bytes, m.word(p + 0x5c)) as u16;
        m.put_half(p + 0x60, n);
        let heap = m.word(m.word(p) + 4);
        let buf = crate::reentrant2::call_method_m(m, heap, 1, n as u32 + 1, &mut NormHooks(h));
        m.put_word(p + 0xc8, buf);
        if buf == 0 {
            r = 0x8960200a;
            finished = false;
        } else {
            m.put_half(p + 0x62, 1);
            let mut i = 0u16;
            while i < n && m.byte(m.word(p + 0x5c) + i as u32) == b' ' {
                i = i.wrapping_add(1);
            }
            while i < n {
                if m.byte(m.word(p + 0x5c) + i as u32) == b' ' {
                    m.put_half(p + 0x62, half(m, p + 0x62).wrapping_add(1));
                    i = i.wrapping_add(1);
                    while i < n && m.byte(m.word(p + 0x5c) + i as u32) == b' ' {
                        i = i.wrapping_add(1);
                    }
                } else {
                    i = i.wrapping_add(1);
                }
            }
            r = token_prepare(m, p, h);
            if r >= 0x80000000 {
                finished = false;
            } else {
                // Unported outside-list engine callee.
                r = Host::call(h, m, 0x249cec, &[p]);
                if r >= 0x80000000 {
                    finished = false;
                } else if m.byte(p + 0xc6) == 1 {
                    // Unported outside-list diagnostic pass.
                    r = Host::call(h, m, 0x24ac00, &[p, end_name, 1]);
                }
            }
        }
    }
    if finished {
        let at = m.offset(processed);
        m.bytes[at] = 1;
    }
    token_cleanup(m, p, h);
    crate::speech_part_6::Host::log(
        h,
        m,
        m.word(m.word(p) + 16),
        m.word(p + 0x48),
        4,
        b"tokentn : Process End (%x)",
        &[r],
    );
    r
}

fn string_property(
    m: &mut Memory<'_>,
    object: u32,
    key: u32,
    initial: u32,
    scratch: u32,
    h: &mut impl Host,
) -> (u32, u32) {
    assert_eq!(m.base, 0);
    let cell = core::cell::RefCell::new(h);
    let mut lock = ParamLock {
        cell: &cell,
        object,
        scratch,
    };
    let mut value = initial;
    let rc = crate::reentrant2::fetch_param_m(
        m.bytes,
        if object == 0 { None } else { Some(object) },
        key,
        Some(&mut value),
        &mut lock,
        |a, b, bytes| {
            let mut m = Memory { base: 0, bytes };
            let map = m.word(object + 8);
            let f = m.word(m.word(map + 60) + 8);
            if f == 0x196980 {
                fn_196980(&mut m, a, b) as i32
            } else {
                cell.borrow_mut().call(&mut m, f, &[a, b]) as i32
            }
        },
    );
    (rc, value)
}
pub struct LanguageEnvironment {
    pub cached_handle: [u8; 8],
    pub depes: u32,
    pub languages: [u32; 2],
    pub voices: [u32; 2],
    pub category: u32,
    pub text_analysis: u32,
    pub yes: u32,
    pub bias: u32,
    pub original_voice: u32,
    pub voice: u32,
    pub lid_state: u32,
}
fn set_string(
    m: &mut Memory<'_>,
    object: u32,
    key: u32,
    value: u32,
    scratch: u32,
    h: &mut impl Host,
) {
    let cell = core::cell::RefCell::new(h);
    m.boot_parameter_call(
        &mut CallHost(&cell),
        0x1759a0,
        &[object, key, value],
        scratch,
    )
    .expect("nonnull property string");
}
fn language_open(
    m: &mut Memory<'_>,
    p: u32,
    input: [u32; 2],
    mime: u32,
    pos: u32,
    flag: u32,
    out: u32,
    h: &mut impl Host,
) -> u32 {
    h.call(
        m,
        m.word(m.word(p + 16) + 32),
        &[input[0], input[1], 0xbd, mime, pos, flag, out],
    )
}
/// fn_1d135c. Uses the live property fetch and uint getter. 1758ac remains
/// the marked pending setter. scratch is 132 disjoint restored guest bytes.
pub fn fn_1d135c<H: Host + crate::speech_part_6::Host>(
    m: &mut Memory<'_>,
    p: u32,
    tag: u32,
    input: [u32; 2],
    env: &LanguageEnvironment,
    scratch: u32,
    h: &mut H,
) -> u32 {
    let mut rc = frontend::validate_handle(p != 0, tag, 0xf3c0, 0x358);
    if rc >= 0x80000000 {
        return 0x8bd02008;
    }
    let at = m.offset(scratch);
    let mut saved = [0; 132];
    saved.copy_from_slice(&m.bytes[at..at + 132]);
    let r = (|| {
        if m.byte(p + 0xb6) != 0 {
            m.put_word(p + 0xb8, m.word(p + 0x90));
            let (r, text) = string_property(
                m,
                m.word(m.word(p) + 20),
                env.text_analysis,
                0,
                scratch + 4,
                h,
            );
            if r < 0x80000000
                && text != 0
                && m.byte(text) != 0
                && normalization::strcmp_at(
                    m.bytes,
                    m.offset(text) as u32,
                    m.offset(env.yes) as u32,
                ) == 0
            {
                let i = m.offset(p + 0x41);
                m.bytes[i] = 1;
                // STUB bridge-4: pending live property setter.
                Host::call(h, m, 0x1758ac, &[m.word(m.word(p) + 20), env.bias, 0]);
            } else {
                m.put_word(scratch, 0);
                if uint_property(m, m.word(m.word(p) + 20), env.bias, scratch, scratch + 4, h)
                    < 0x80000000
                    && m.word(scratch) != 0
                {
                    m.put_word(p + 0xb8, m.word(scratch));
                    let i = m.offset(p + 0x42);
                    m.bytes[i] = 1;
                }
            }
            let src = m.offset(m.word(p + 0x350)) as u32;
            let dst = m.offset(p + 0xbc) as u32;
            fe_text::memcpy_195ae8(m.bytes, dst, src, 4);
        }
        if m.byte(p + 0x43) == 0 {
            return rc;
        }
        if half(m, p + 0x4a) == 0 && half(m, p + 0x62) == 0 {
            crate::speech_part_6::Host::error(h, m, m.word(m.word(p) + 16), env.category, 0xbb82);
            return 0x8bd02003;
        }
        let mut languages = 0u32;
        let mut voices = 0u32;
        if language_open(m, p, input, env.depes, 0, 0, p + 0x20, h) < 0x80000000 {
            rc = Host::call(
                h,
                m,
                m.word(m.word(p + 16) + 36),
                &[m.word(p + 0x20), m.word(p + 0x24)],
            );
            reset_handle(
                m,
                p + 0x20,
                &ComponentEnvironment {
                    cached_handle: env.cached_handle,
                    input_types: [0; 2],
                },
            );
        } else {
            loop {
                rc = language_open(
                    m,
                    p,
                    input,
                    env.languages[languages as usize],
                    0,
                    0,
                    p + 0x20 + languages * 8,
                    h,
                );
                if rc >= 0x80000000 {
                    break;
                }
                languages += 1;
                if languages == 2 {
                    break;
                }
            }
            if rc < 0x80000000 {
                while rc < 0x80000000 && voices < 2 {
                    let pos = if voices == 1 {
                        Host::call(h, m, m.word(m.word(p + 16) + 76), &[])
                    } else {
                        0
                    };
                    rc = language_open(
                        m,
                        p,
                        input,
                        env.voices[voices as usize],
                        pos,
                        1,
                        p + 0x30 + voices * 8,
                        h,
                    );
                    if rc < 0x80000000 {
                        voices += 1;
                    }
                }
            }
            if rc < 0x80000000 {
                let (r, mut voice) = string_property(
                    m,
                    m.word(m.word(p) + 20),
                    env.original_voice,
                    0,
                    scratch + 4,
                    h,
                );
                if !(r < 0x80000000 && voice != 0 && m.byte(voice) != 0) {
                    let (r, v) = string_property(
                        m,
                        m.word(m.word(p) + 20),
                        env.voice,
                        voice,
                        scratch + 4,
                        h,
                    );
                    voice = v;
                    if r < 0x80000000 && voice != 0 && m.byte(voice) != 0 {
                        set_string(
                            m,
                            m.word(m.word(p) + 20),
                            env.original_voice,
                            voice,
                            scratch + 4,
                            h,
                        );
                    }
                }
                let (r, lid) =
                    string_property(m, m.word(m.word(p) + 20), env.lid_state, 0, scratch + 4, h);
                if r < 0x80000000 && lid != 0 && m.byte(lid) != 0 {
                    let i = m.offset(p + 0x320);
                    m.bytes[i] = 1;
                } else {
                    let i = m.offset(p + 0x320);
                    m.bytes[i] = 0;
                    let (r, v) = string_property(
                        m,
                        m.word(m.word(p) + 20),
                        env.voice,
                        voice,
                        scratch + 4,
                        h,
                    );
                    voice = v;
                    if r < 0x80000000 && voice != 0 && m.byte(voice) != 0 {
                        set_string(
                            m,
                            m.word(m.word(p) + 20),
                            env.original_voice,
                            voice,
                            scratch + 4,
                            h,
                        );
                    }
                }
            }
        }
        if rc >= 0x80000000 {
            for (start, n) in [(0x20, languages), (0x30, voices)] {
                for i in 0..n {
                    let pair = p + start + i * 8;
                    if m.word(pair) != 0 {
                        Host::call(
                            h,
                            m,
                            m.word(m.word(p + 16) + 36),
                            &[m.word(pair), m.word(pair + 4)],
                        );
                        m.put_word(pair, 0);
                        m.put_word(pair + 4, 0);
                    }
                }
            }
        }
        rc
    })();
    m.bytes[at..at + 132].copy_from_slice(&saved);
    r
}

pub struct StreamEnvironment {
    pub cached_handle: [u8; 8],
    pub default_context: u32,
    pub utf16: u32,
    pub depes: u32,
    pub markers: u32,
    pub input_mime: u32,
    pub category: u32,
    pub format: u32,
    pub content_name: u32,
    pub mime_name: u32,
    pub ids: [u16; 7],
}
struct LiveHeap<'a, H> {
    host: &'a mut H,
    table: u32,
}
impl<H: Host> frontend::HeapVtableM for LiveHeap<'_, H> {
    fn alloc_at_m(&mut self, obj: u32, n: u32, bytes: &mut [u8]) -> u32 {
        let mut m = Memory { base: 0, bytes };
        let f = m.word(self.table);
        self.host.call(&mut m, f, &[obj, n])
    }
    fn realloc_at_m(&mut self, obj: u32, p: u32, n: u32, bytes: &mut [u8]) -> u32 {
        let mut m = Memory { base: 0, bytes };
        let f = m.word(self.table + 8);
        self.host.call(&mut m, f, &[obj, p, n])
    }
    fn release_at_m(&mut self, obj: u32, p: u32, bytes: &mut [u8]) {
        let mut m = Memory { base: 0, bytes };
        let f = m.word(self.table + 12);
        self.host.call(&mut m, f, &[obj, p]);
    }
}
fn resolve_context(m: &Memory<'_>, a: u32, b: u32, env: &StreamEnvironment) -> (u32, u32) {
    let mut out = 0;
    let cached_a = u32::from_le_bytes(env.cached_handle[..4].try_into().unwrap());
    let cached_b = u32::from_le_bytes(env.cached_handle[4..].try_into().unwrap());
    let pointed = if a == 0 || [a, b] == [cached_a, cached_b] {
        None
    } else {
        Some(half(m, a))
    };
    let r = frontend::resolve_engine_context(
        a,
        pointed,
        b,
        &env.cached_handle,
        env.default_context,
        &mut out,
    );
    (r, out)
}
struct FetchHost<'a, H> {
    host: &'a mut H,
    env: &'a StreamEnvironment,
}
impl<H: Host> normalization::NormalizationHost for FetchHost<'_, H> {
    fn calloc(&mut self, bytes: &mut [u8], heap: u32, n: u32, size: u32) -> Option<u32> {
        let p = crate::reentrant2::call_method_m(
            &mut Memory { base: 0, bytes },
            heap,
            n,
            size,
            &mut ArrayHost(self.host),
        );
        if p == 0 { None } else { Some(p) }
    }
    fn resolve(&mut self, bytes: &mut [u8], a: u32, b: u32) -> (u32, u32) {
        resolve_context(&Memory { base: 0, bytes }, a, b, self.env)
    }
    fn alloc(&mut self, bytes: &mut [u8], heap: u32, n: u32) -> Option<u32> {
        let table = if heap == 0 {
            0
        } else {
            u32::from_le_bytes(bytes[heap as usize..heap as usize + 4].try_into().unwrap())
        };
        let p = frontend::heap_alloc_m(
            bytes,
            if heap == 0 { None } else { Some(heap) },
            n,
            &mut LiveHeap {
                host: self.host,
                table,
            },
        );
        if p == 0 { None } else { Some(p) }
    }
    fn realloc(&mut self, bytes: &mut [u8], heap: u32, old: u32, n: u32) -> Option<u32> {
        let table = if heap == 0 {
            0
        } else {
            u32::from_le_bytes(bytes[heap as usize..heap as usize + 4].try_into().unwrap())
        };
        let p = frontend::heap_realloc_m(
            bytes,
            if heap == 0 { None } else { Some(heap) },
            old,
            n,
            &mut LiveHeap {
                host: self.host,
                table,
            },
        );
        if p == 0 { None } else { Some(p) }
    }
    fn free(&mut self, bytes: &mut [u8], heap: u32, p: u32) {
        release_live(&mut Memory { base: 0, bytes }, heap, p, self.host);
    }
}
/// fn_1e57d0. Real resolver and fetch table; the host binds sequence reset
/// to fn_2643f8 below. scratch is 136 restored bytes.
pub fn fn_1e57d0<H: Host + crate::speech_part_6::Host>(
    m: &mut Memory<'_>,
    p: u32,
    tag: u32,
    input: [u32; 2],
    env: &StreamEnvironment,
    scratch: u32,
    h: &mut H,
) -> u32 {
    if frontend::validate_handle(p != 0, tag, 0x3815, 0x30) >= 0x80000000 {
        return 0x81102008;
    }
    assert_eq!(m.base, 0);
    let (r, ctx) = resolve_context(m, m.word(p), m.word(p + 4), env);
    if r >= 0x80000000 {
        return 0x81102007;
    }
    let be = m.word(p + 0x28);
    if be == 0 {
        return 0x81102007;
    }
    let mime = if m.byte(be + 0x77) == 0 {
        env.utf16
    } else {
        env.depes
    };
    let at = m.offset(scratch);
    let mut saved = [0; 136];
    saved.copy_from_slice(&m.bytes[at..at + 136]);
    let r = (|| {
        let open = |m: &mut Memory<'_>, h: &mut H, mime, pos, flag, out| {
            Host::call(
                h,
                m,
                m.word(m.word(p + 8) + 32),
                &[input[0], input[1], 17, mime, pos, flag, out],
            )
        };
        let mut rc = open(m, h, env.utf16, 0, 0, p + 12);
        if rc >= 0x80000000 {
            let (r, s) = string_property(m, m.word(ctx + 20), env.input_mime, 0, scratch + 4, h);
            let logger = m.word(ctx + 16);
            if r < 0x80000000 && s != 0 && m.byte(s) != 0 {
                Host::call(
                    h,
                    m,
                    0x173dc0,
                    &[logger, env.category, 0x76f, env.format, env.content_name, s],
                );
                rc = 0x8110210b;
            } else {
                crate::speech_part_6::Host::error(h, m, logger, env.category, 0x76c);
            }
            return rc;
        }
        rc = open(m, h, mime, 0, 1, p + 20);
        if rc >= 0x80000000 {
            Host::call(
                h,
                m,
                0x173dc0,
                &[
                    m.word(ctx + 16),
                    env.category,
                    0x76d,
                    env.format,
                    env.mime_name,
                    mime,
                ],
            );
            return rc;
        }
        let pos = Host::call(h, m, m.word(m.word(p + 8) + 76), &[]);
        rc = open(m, h, env.markers, pos, 1, p + 28);
        if rc >= 0x80000000 {
            crate::speech_part_6::Host::error(h, m, m.word(ctx + 16), env.category, 0x76e);
            return rc;
        }
        for id in env.ids {
            let mut item = 0;
            rc = normalization::fetch_entry(
                m.bytes,
                m.word(p + 36),
                id as u32,
                1,
                Some(&mut item),
                &mut FetchHost { host: h, env },
            );
            if rc >= 0x80000000 {
                return rc;
            }
            let j = m.offset(item + 8);
            m.bytes[j..j + 2].fill(0);
            let i = m.offset(item + 12);
            m.bytes[i] = 0;
        }
        // Live host binding to fn_2643f8 with the caller's regional environment.
        rc = Host::call(
            h,
            m,
            0x2643f8,
            &[m.word(p + 40), m.word(p + 12), m.word(p + 16)],
        );
        if rc < 0x80000000 { 0 } else { rc }
    })();
    m.bytes[at..at + 136].copy_from_slice(&saved);
    r
}

fn sh(m: &mut Memory<'_>, p: u32, v: u16) {
    let i = m.offset(p);
    m.bytes[i..i + 2].copy_from_slice(&v.to_le_bytes());
}
fn sb(m: &mut Memory<'_>, p: u32, v: u8) {
    let i = m.offset(p);
    m.bytes[i] = v;
}
fn sl(m: &Memory<'_>, p: u32) -> u32 {
    normalization::strlen8(m.bytes, p - m.base)
}
fn cp(m: &mut Memory<'_>, d: u32, s: u32) {
    normalization::strcpy_at(m.bytes, d - m.base, s - m.base);
}
fn cat(m: &mut Memory<'_>, d: u32, s: u32) {
    let end = d + sl(m, d);
    cp(m, end, s);
}
fn eq(m: &Memory<'_>, a: u32, b: u32) -> bool {
    normalization::strcmp_at(m.bytes, a - m.base, b - m.base) == 0
}
fn eq_lit(m: &Memory<'_>, a: u32, b: &[u8]) -> bool {
    let i = m.offset(a);
    m.bytes[i..].starts_with(b) && m.byte(a + b.len() as u32) == 0
}
fn fill(m: &mut Memory<'_>, p: u32, n: u32, v: u8) {
    let i = m.offset(p);
    m.bytes[i..i + n as usize].fill(v);
}
fn move_bytes(m: &mut Memory<'_>, d: u32, s: u32, n: u32) {
    let i = m.offset(s);
    let j = m.offset(d);
    m.bytes.copy_within(i..i + n as usize, j);
}
fn calloc_live(m: &mut Memory<'_>, heap: u32, n: u32, h: &mut impl Host) -> u32 {
    crate::reentrant2::call_method_m(m, heap, 1, n, &mut ArrayHost(h))
}
fn realloc_live(m: &mut Memory<'_>, heap: u32, p: u32, n: u32, h: &mut impl Host) -> u32 {
    assert_eq!(m.base, 0);
    frontend::heap_realloc_m(
        m.bytes,
        if heap == 0 { None } else { Some(heap) },
        p,
        n,
        &mut LiveHeap {
            host: h,
            table: if heap == 0 { 0 } else { m.word(heap) },
        },
    )
}
pub struct TagEnvironment {
    pub category: u32,
    pub key: u32,
    pub section: u32,
    pub lid: u32,
    pub latin: u32,
    pub underscore: u32,
    pub start_name: u32,
    pub end_name: u32,
    pub empty: u32,
    pub end_format: u32,
}
fn tag_ling(m: &mut Memory<'_>, p: u32, slot: u32, args: &[u32], h: &mut impl Host) -> u32 {
    h.call(m, m.word(m.word(p + 4) + slot), args)
}
/// fn_1d9508. Full edit pass; scratch is 176 restored bytes, disjoint
/// from every caller object and allocation.
pub fn fn_1d9508<H: Host + crate::speech_part_6::Host>(
    m: &mut Memory<'_>,
    p: u32,
    db: [u32; 2],
    x: u32,
    text: u32,
    e: &TagEnvironment,
    scratch: u32,
    h: &mut H,
) -> u32 {
    assert_eq!(m.base, 0);
    let at = m.offset(scratch);
    let mut saved = [0; 176];
    saved.copy_from_slice(&m.bytes[at..at + 176]);
    let mut cls = 0;
    let mut wi = 0;
    let mut lid = 0;
    let mut toks = 0;
    let mut strs = 0;
    let mut tab = 0;
    let mut dbg_buf = 0;
    let mut ns = 0u16;
    let mut nt = 0u16;
    let r = (|| {
        let s = scratch;
        // count/node/len/written/hasPron/tmp/type then pv/sep/val/pron/norm
        for off in [0, 6, 8, 12] {
            sh(m, s + off, 0);
        }
        let dbg = Host::call(h, m, 0x173fd8, &[m.word(m.word(p) + 16)]) >= 5;
        macro_rules! log {
            ($fmt:expr,$args:expr) => {
                crate::speech_part_6::Host::log(
                    h,
                    m,
                    m.word(m.word(p) + 16),
                    e.category,
                    5,
                    $fmt,
                    $args,
                )
            };
        }
        macro_rules! err {
            ($code:expr) => {
                crate::speech_part_6::Host::error(h, m, m.word(m.word(p) + 16), e.category, $code)
            };
        }
        macro_rules! call {
            ($slot:expr,$args:expr) => {
                tag_ling(m, p, $slot, $args, h)
            };
        }
        macro_rules! check {
            ($expr:expr) => {{
                let rc = $expr;
                if rc >= 0x80000000 {
                    return rc;
                }
                rc
            }};
        }
        macro_rules! alloc {
            ($size:expr) => {{
                let q = calloc_live(m, m.word(m.word(p) + 4), $size, h);
                if q == 0 {
                    err!(0x80e8);
                    return 0x89c0200a;
                }
                q
            }};
        }
        macro_rules! dump {
            ($name:expr) => {
                crate::speech_part_6::fn_1d8f28(
                    m,
                    h,
                    p,
                    db,
                    x,
                    text,
                    dbg_buf,
                    $name,
                    e.category,
                    e.empty,
                    s + 64,
                )
            };
        }
        macro_rules! edits {
            () => {{
                for i in 0..nt {
                    let t = toks + i as u32 * 14;
                    m.put_word(s + 48, 0);
                    call!(0x80, &[db[0], db[1], m.byte(t + 6) as u32, s + 48]);
                    let op = match m.byte(t + 7) {
                        1 => b" ** MODIFYTOPOS **".as_slice(),
                        2 => b" ** INSERT **".as_slice(),
                        3 => b" ** DELETE **".as_slice(),
                        _ => b"".as_slice(),
                    };
                    // The operation literal is supplied in restored local storage.
                    fill(m, s + 112, 32, 0);
                    let z = m.offset(s + 112);
                    m.bytes[z..z + op.len()].copy_from_slice(op);
                    if m.byte(t + 6) == 5 {
                        log!(
                            b"%d : [%d] %d,%d %s %s %c %s",
                            &[
                                i as u32,
                                half(m, t) as u32,
                                half(m, t + 2) as u32,
                                half(m, t + 4) as u32,
                                m.word(s + 48),
                                m.word(strs + half(m, t + 8) as u32 * 4),
                                m.byte(t + 10) as u32,
                                s + 112
                            ]
                        );
                    } else {
                        log!(
                            b"%d : [%d] %d,%d %s",
                            &[
                                i as u32,
                                half(m, t) as u32,
                                half(m, t + 2) as u32,
                                half(m, t + 4) as u32,
                                s + 112
                            ]
                        );
                    }
                }
            }};
        }
        let mut r = check!(call!(0x4c, &[db[0], db[1], 3, x, s]));
        if half(m, s) == 0 {
            if dbg {
                log!(b"no tokens to process for langmaps", &[]);
            }
            return r;
        }
        r = check!(call!(0x34, &[db[0], db[1], 3, x, s + 2]));
        let mut any = false;
        if half(m, s + 2) != 0 {
            loop {
                if !any {
                    r = check!(call!(
                        0x54,
                        &[db[0], db[1], half(m, s + 2) as u32, 0, 1, s + 12, s + 4]
                    ));
                    if m.byte(s + 12) == 6 {
                        r = check!(call!(
                            0x5c,
                            &[db[0], db[1], half(m, s + 2) as u32, 6, s + 8]
                        ));
                        if half(m, s + 8) == 1 {
                            any = true;
                        }
                    }
                }
                nt = nt.wrapping_add(1);
                if call!(0x3c, &[db[0], db[1], half(m, s + 2) as u32, s + 2]) != 0
                    || half(m, s + 2) == 0
                {
                    break;
                }
            }
        }
        if !any {
            log!(b"No LangMaps to process", &[]);
            return r;
        }
        let sent = sl(m, text) as u16;
        if dbg {
            dbg_buf = alloc!(sent as u32 + 1);
            cp(m, dbg_buf, text);
            dump!(e.start_name);
        }
        cls = alloc!(sl(m, text) + 1);
        wi = alloc!((sl(m, text) + 1) * 2);
        lid = alloc!(sl(m, text) + 1);
        toks = alloc!((nt as u32 + 1) * 14);
        strs = alloc!(24);
        let mut cap = 5u16;
        tab = alloc!(512);
        sh(m, s + 10, 0xffff);
        m.put_word(s + 16, 0);
        r = check!(Host::call(
            h,
            m,
            m.word(m.word(p + 28) + 28),
            &[
                m.word(p + 20),
                m.word(p + 24),
                e.section,
                e.key,
                s + 16,
                s + 10,
                s + 20
            ]
        ));
        let mut key = e.key;
        if half(m, s + 10) != 0 {
            key = m.word(m.word(s + 16));
            let q = fe_text::strchr_195d30(m.bytes, key, m.word(s + 20));
            if q != 0 {
                sb(m, q, 0);
            }
        }
        let buf = alloc!(sl(m, key) + 1);
        cp(m, buf, key);
        m.put_word(strs, buf);
        ns = 1;
        r = check!(call!(0x34, &[db[0], db[1], 3, x, s + 2]));
        nt = 0;
        while half(m, s + 2) != 0 {
            let t = toks + nt as u32 * 14;
            sh(m, t, half(m, s + 2));
            sb(m, t + 7, 0);
            sh(m, t + 8, 0);
            sb(m, t + 10, b' ');
            sb(m, t + 11, 0);
            sb(m, t + 12, 0);
            for (field, out) in [(1, t + 2), (2, t + 4), (0, t + 6)] {
                r = check!(call!(
                    0x54,
                    &[db[0], db[1], half(m, s + 2) as u32, field, 1, out, s + 4]
                ));
            }
            if m.byte(t + 6) == 5 {
                sh(m, s + 10, 0);
                if call!(0x5c, &[db[0], db[1], half(m, s + 2) as u32, 6, s + 10]) < 0x80000000
                    && half(m, s + 10) == 1
                {
                    sb(m, t + 12, 1);
                }
                r = check!(call!(
                    0x58,
                    &[db[0], db[1], half(m, s + 2) as u32, 4, s + 24, s + 4]
                ));
                if m.word(p + 52) == 0 {
                    return 0x89c0200a;
                }
                let val = m.word(s + 24);
                let bar = frontend::find_substr(m.bytes, m.word(p + 52), val);
                let vl = sl(m, val);
                let buf = alloc!(core::cmp::max(sl(m, m.word(strs)), vl) + 1);
                if eq_lit(m, val, b"phon")
                    || eq_lit(m, val, b"prompt")
                    || (!eq_lit(m, val, b"latin")
                        && ((bar != 0 && m.byte(bar + vl) == b'|')
                            || fe_text::strchr_195d30(m.bytes, val, b'_' as u32) != 0))
                {
                    cp(m, buf, val);
                    sb(m, t + 10, b'N');
                    for k in 0..vl {
                        if m.byte(val + k) == b'_' {
                            let ch = fe_text::fold_17c8dc(m.byte(val + k + 1) as u32) as u8;
                            sb(m, t + 10, ch);
                            m.put_word(tab + ch as u32 * 4, val + k + 1);
                            fill(m, buf + k, vl - k, 0);
                            break;
                        }
                    }
                } else {
                    cp(m, buf, m.word(strs));
                    let ch = fe_text::fold_17c8dc(m.byte(val) as u32) as u8;
                    sb(m, t + 10, ch);
                    m.put_word(tab + ch as u32 * 4, val);
                }
                for k in half(m, t + 2)..half(m, t + 4) {
                    sb(m, cls + k as u32, m.byte(t + 10));
                }
                let mut i = 0;
                while i < ns {
                    if fe_text::strcmp_ci_1769d0(m.bytes, m.word(strs + i as u32 * 4), buf) == 0 {
                        sh(m, t + 8, i);
                        release_live(m, m.word(m.word(p) + 4), buf, h);
                        break;
                    }
                    i += 1;
                }
                if i == ns {
                    if ns + 1 > cap {
                        let q =
                            realloc_live(m, m.word(m.word(p) + 4), strs, (cap as u32 + 6) * 4, h);
                        if q == 0 {
                            err!(0x80e8);
                            return 0x89c0200a;
                        }
                        strs = q;
                        cap = cap.wrapping_add(5);
                    }
                    m.put_word(strs + i as u32 * 4, buf);
                    ns = ns.wrapping_add(1);
                    sh(m, t + 8, i);
                }
                for k in half(m, t + 2)..half(m, t + 4) {
                    sh(m, wi + k as u32 * 2, half(m, t + 8));
                    sb(m, lid + k as u32, m.byte(t + 12));
                }
                if eq_lit(m, m.word(strs + half(m, t + 8) as u32 * 4), b"phon") {
                    for k in half(m, t + 2)..half(m, t + 4) {
                        sb(m, cls + k as u32, b'P');
                    }
                }
            }
            nt = nt.wrapping_add(1);
            if call!(0x3c, &[db[0], db[1], half(m, s + 2) as u32, s + 2]) != 0
                || half(m, s + 2) == 0
            {
                break;
            }
        }
        if dbg {
            log!(b"DUMP before processing", &[]);
            edits!();
            log!(b"Sent=%s", &[text]);
        }
        r = 0;
        sh(m, s + 8, 0);
        m.put_word(s + 28, 0);
        m.put_word(s + 32, 0);
        sh(m, s + 4, 0);
        for i in 0..nt {
            let t = toks + i as u32 * 14;
            if m.byte(t + 6) != 6 {
                continue;
            }
            r = check!(call!(0x5c, &[db[0], db[1], half(m, t) as u32, 6, s + 8]));
            if half(m, s + 8) != 1 {
                continue;
            }
            r = check!(call!(
                0x58,
                &[db[0], db[1], half(m, t) as u32, 6, s + 28, s + 4]
            ));
            r = check!(call!(
                0x58,
                &[db[0], db[1], half(m, t) as u32, 3, s + 32, s + 4]
            ));
            for j in 0..i {
                let tj = toks + j as u32 * 14;
                if m.byte(tj + 6) != 5
                    || half(m, t + 2) < half(m, tj + 2)
                    || half(m, t + 4) > half(m, tj + 4)
                {
                    continue;
                }
                let mut k = half(m, t + 4);
                while k < half(m, tj + 4) && m.byte(text + k as u32) == b' ' {
                    k = k.wrapping_add(1);
                }
                if half(m, t + 4) != k && half(m, tj + 4) == k {
                    sb(m, t + 11, 1);
                    log!(
                        b"spaced region found...adjust token end from %d to %d",
                        &[half(m, t + 4) as u32, half(m, tj + 4) as u32]
                    );
                    let n = (sl(m, m.word(s + 28))
                        + half(m, tj + 4).wrapping_sub(half(m, t + 4)) as u32)
                        as u16;
                    let buf = alloc!(n as u32 + 1);
                    cp(m, buf, m.word(s + 28));
                    fe_text::strncat_195dac(
                        m.bytes,
                        buf,
                        text + half(m, t + 4) as u32,
                        half(m, tj + 4).wrapping_sub(half(m, t + 4)) as u32,
                    );
                    log!(
                        b"modify LD_T_LANGMAP field from \"%s\" to \"%s\"",
                        &[m.word(s + 28), buf]
                    );
                    let n = n.wrapping_add(1);
                    r = call!(
                        0x50,
                        &[db[0], db[1], half(m, t) as u32, 6, n as u32, buf, s + 4]
                    );
                    if r < 0x80000000 {
                        if n == half(m, s + 4) {
                            cp(m, buf, m.word(s + 32));
                            fe_text::strncat_195dac(
                                m.bytes,
                                buf,
                                text + half(m, t + 4) as u32,
                                half(m, tj + 4).wrapping_sub(half(m, t + 4)) as u32,
                            );
                            log!(
                                b"modify LD_T_NORM field from \"%s\" to \"%s\"",
                                &[m.word(s + 32), buf]
                            );
                            r = call!(
                                0x50,
                                &[db[0], db[1], half(m, t) as u32, 3, n as u32, buf, s + 4]
                            );
                            if r >= 0x80000000 {
                                release_live(m, m.word(m.word(p) + 4), buf, h);
                                return r;
                            }
                            if n == half(m, s + 4) {
                                release_live(m, m.word(m.word(p) + 4), buf, h);
                                sh(m, t + 4, half(m, tj + 4));
                                r = call!(
                                    0x50,
                                    &[db[0], db[1], half(m, t) as u32, 2, 1, t + 4, s + 4]
                                );
                                break;
                            }
                        }
                        r = 0x89c02000;
                    }
                    release_live(m, m.word(m.word(p) + 4), buf, h);
                    break;
                }
            }
            if r >= 0x80000000 {
                return r;
            }
        }
        if dbg {
            log!(b"DUMP after langmap realign", &[]);
            edits!();
            log!(b"Sent=%s", &[text]);
            log!(b"Map =%s", &[cls]);
        }
        let mut i = 0u16;
        while i < nt {
            let t = toks + i as u32 * 14;
            if m.byte(t + 7) != 3 && m.byte(t + 6) == 6 {
                r = check!(call!(0x5c, &[db[0], db[1], half(m, t) as u32, 6, s + 8]));
                if half(m, s + 8) == 1 {
                    r = check!(call!(
                        0x58,
                        &[db[0], db[1], half(m, t) as u32, 6, s + 24, s + 4]
                    ));
                    let buf = alloc!(sl(m, m.word(s + 24)) as u16 as u32 + 1);
                    cp(m, buf, m.word(s + 24));
                    let mut ch = m.byte(buf);
                    for o in 0..half(m, t + 4).wrapping_sub(half(m, t + 2)) {
                        if m.byte(buf + o as u32) != b' ' {
                            ch = m.byte(buf + o as u32);
                        }
                        if ch != b' ' {
                            sb(m, cls + half(m, t + 2) as u32 + o as u32, ch);
                        }
                    }
                    let id = half(m, t);
                    release_live(m, m.word(m.word(p) + 4), buf, h);
                    i = 0;
                    while i < nt && half(m, toks + i as u32 * 14) != id {
                        i = i.wrapping_add(1);
                    }
                }
            }
            i = i.wrapping_add(1);
        }
        if dbg {
            log!(b"DUMP after inserting langMaps", &[]);
            log!(b"Sent=%s", &[text]);
            log!(b"Map =%s", &[cls]);
        }
        for i in 0..nt {
            let t = toks + i as u32 * 14;
            if m.byte(t + 6) != 6 || m.byte(t + 11) != 0 {
                continue;
            }
            let mut k = half(m, t + 4);
            while k < sent && m.byte(text + k as u32) != b' ' {
                k = k.wrapping_add(1);
            }
            if m.byte(text + k as u32) == b' ' {
                while k < sent && m.byte(text + k as u32) == b' ' {
                    k = k.wrapping_add(1);
                }
            }
            if k > sent {
                continue;
            }
            let mut bisect = false;
            if half(m, t + 4) != k {
                crate::speech_part_6::Host::log_guest(
                    h,
                    m,
                    m.word(m.word(p) + 16),
                    e.category,
                    5,
                    e.end_format,
                    &[i as u32, half(m, t + 4) as u32, k as u32, sent as u32],
                );
            }
            let mut j = i.wrapping_add(1);
            while j < nt {
                let tj = toks + j as u32 * 14;
                if half(m, tj + 2) < k
                    && m.byte(tj + 6) == 6
                    && m.byte(tj + 7) != 3
                    && half(m, wi + k as u32 * 2) != half(m, wi + half(m, tj + 2) as u32 * 2)
                {
                    bisect = true;
                }
                j = j.wrapping_add(1);
                let q = toks + j as u32 * 14;
                if j < nt && half(m, q + 2) >= k && m.byte(q + 6) == 6 {
                    break;
                }
                if bisect {
                    break;
                }
            }
            if bisect {
                let tj = toks + j as u32 * 14;
                log!(
                    b"block adjusting: bisects token rec %d -> %d,%d",
                    &[
                        half(m, tj) as u32,
                        half(m, tj + 2) as u32,
                        half(m, tj + 4) as u32
                    ]
                );
            } else {
                let ch = m.byte(cls + half(m, t + 2) as u32);
                for q in half(m, t + 4)..k {
                    sb(m, cls + q as u32, ch);
                }
                sh(m, t + 4, k);
                log!(
                    b"adjusted rec %d -> %d,%d",
                    &[half(m, t) as u32, half(m, t + 2) as u32, k as u32]
                );
            }
        }
        for i in 0..nt {
            let t = toks + i as u32 * 14;
            if m.byte(t + 6) != 6 {
                continue;
            }
            let from = half(m, t + 2);
            let mut b = from as i16;
            while b > 0 && m.byte(text + b as u32) != b' ' {
                b -= 1;
            }
            if b < 0 {
                continue;
            }
            let ub = b as u16;
            let nf = if m.byte(text + ub as u32) == b' ' && from != ub.wrapping_add(1) {
                ub.wrapping_add(1)
            } else if ub == 0 && from != 0 {
                ub
            } else {
                continue;
            };
            log!(
                b"consider adjusting start pos %d -> %d",
                &[from as u32, nf as u32]
            );
            let mut conflict = false;
            let mut j = 0;
            while j < nt && half(m, toks + j as u32 * 14 + 2) <= from {
                let tj = toks + j as u32 * 14;
                if j != i {
                    if m.byte(tj + 6) == 6 {
                        if half(m, tj + 2) <= nf && nf < half(m, tj + 4) {
                            conflict = true;
                            break;
                        }
                    } else if m.byte(tj + 6) == 5
                        && m.byte(tj + 7) != 3
                        && half(m, wi + ub as u32 * 2) != half(m, wi + half(m, tj + 2) as u32 * 2)
                    {
                        conflict = true;
                        break;
                    }
                }
                j += 1;
            }
            if conflict {
                let tj = toks + j as u32 * 14;
                log!(
                    b"block adjusting: bisects token rec %d -> %d,%d",
                    &[
                        half(m, tj) as u32,
                        half(m, tj + 2) as u32,
                        half(m, tj + 4) as u32
                    ]
                );
            } else {
                let ch = m.byte(cls + from as u32);
                for q in nf..from {
                    sb(m, cls + q as u32, ch);
                }
                sh(m, t + 2, nf);
                log!(
                    b"adjusted rec %d -> %d,%d",
                    &[half(m, t) as u32, nf as u32, half(m, t + 4) as u32]
                );
            }
        }
        if dbg {
            log!(b"After Adjusting End Check", &[]);
            log!(b"Sent=%s", &[text]);
            log!(b"Map =%s", &[cls]);
        }
        let sent = sl(m, text) as u16;
        let mut prev = m.byte(cls);
        let mut k = 0u16;
        while k < sent {
            let ch = m.byte(cls + k as u32);
            if ch != prev && m.byte(text + k as u32) == b' ' {
                let mut q = k.wrapping_add(1);
                while q < sent && m.byte(cls + q as u32) == ch && m.byte(text + q as u32) == b' ' {
                    q = q.wrapping_add(1);
                }
                if prev != b'P' {
                    log!(
                        b"Append region %d..%d to previous %c region",
                        &[k as u32, q as u32, prev as u32]
                    );
                    for v in k..q {
                        sb(m, cls + v as u32, prev);
                    }
                }
                k = q;
            } else if ch != prev {
                prev = ch;
            }
            k = k.wrapping_add(1);
        }
        if dbg {
            log!(b"After Region Check", &[]);
            log!(b"Sent=%s", &[text]);
            log!(b"Map =%s", &[cls]);
        }
        for i in 0..nt {
            let t = toks + i as u32 * 14;
            if m.byte(t + 6) == 5 && !eq_lit(m, m.word(strs + half(m, t + 8) as u32 * 4), b"phon") {
                sb(m, t + 7, 3);
            }
        }
        let mut pc = m.byte(cls);
        let mut pw = half(m, wi);
        let mut start = 0u16;
        while start < sent {
            let mut k = start;
            while k < sent && m.byte(cls + k as u32) == pc && half(m, wi + k as u32 * 2) == pw {
                k = k.wrapping_add(1);
            }
            let ch = m.byte(cls + start as u32);
            if k != start && ch != b'P' {
                let w = half(m, wi + start as u32 * 2);
                let l = m.byte(lid + start as u32);
                let mut pos = 0u16;
                let mut j = 0u16;
                while j < nt {
                    let t = toks + j as u32 * 14;
                    if m.byte(t + 7) == 3 {
                        j += 1;
                        continue;
                    }
                    if half(m, t + 2) > start {
                        pos = j;
                        break;
                    }
                    if half(m, t + 2) == start {
                        pos = nt;
                        if j + 1 < nt {
                            while j + 1 < nt
                                && half(m, toks + (j + 1) as u32 * 14 + 2) == start
                                && m.byte(toks + (j + 1) as u32 * 14 + 7) != 3
                            {
                                j += 1;
                            }
                            pos = j;
                        }
                        break;
                    }
                    j += 1;
                }
                let q = realloc_live(m, m.word(m.word(p) + 4), toks, (nt as u32 + 2) * 14, h);
                if q == 0 {
                    err!(0x80e8);
                    r = 0x89c0200a;
                } else {
                    toks = q;
                    if j == nt {
                        pos = nt;
                        log!(
                            b"insert %d,%d, [%s_%c] at end",
                            &[
                                start as u32,
                                k as u32,
                                m.word(strs + w as u32 * 4),
                                ch as u32
                            ]
                        );
                    } else {
                        log!(
                            b"insert %d,%d, [%s_%c] before rec %d",
                            &[
                                start as u32,
                                k as u32,
                                m.word(strs + w as u32 * 4),
                                ch as u32,
                                half(m, toks + pos as u32 * 14) as u32
                            ]
                        );
                        let mut z = nt.wrapping_add(1);
                        while z > pos {
                            move_bytes(m, toks + z as u32 * 14, toks + (z - 1) as u32 * 14, 14);
                            z -= 1;
                        }
                        fill(m, toks + pos as u32 * 14, 14, 0);
                    }
                    r = 0;
                    let t = toks + pos as u32 * 14;
                    sh(m, t, 0);
                    sh(m, t + 2, start);
                    sh(m, t + 4, k);
                    sh(m, t + 8, w);
                    sb(m, t + 10, ch);
                    sb(m, t + 7, 2);
                    sb(m, t + 6, 5);
                    sb(m, t + 12, l);
                    nt = nt.wrapping_add(1);
                }
            }
            start = k;
            if k < sent && (m.byte(cls + k as u32) != pc || half(m, wi + k as u32 * 2) != pw) {
                pc = m.byte(cls + k as u32);
                pw = half(m, wi + k as u32 * 2);
            }
        }
        if dbg {
            log!(b"DUMP before update lingdb", &[]);
            edits!();
            log!(b"Sent=%s", &[text]);
            log!(b"Map =%s", &[cls]);
        }
        for i in 0..nt {
            let t = toks + i as u32 * 14;
            match m.byte(t + 7) {
                2 => {
                    let mut q = i as i16 - 1;
                    while q >= 0 && m.byte(toks + q as u32 * 14 + 7) == 3 {
                        q -= 1;
                    }
                    if q >= 0 {
                        let id = half(m, toks + q as u32 * 14);
                        if id == 0 {
                            err!(0x80ec);
                            return 0x89c02000;
                        }
                        r = check!(call!(0x28, &[db[0], db[1], id as u32, s + 2]));
                    } else {
                        q = i as i16 + 1;
                        while (q as u16) < nt
                            && (m.byte(toks + q as u32 * 14 + 7) == 3
                                || half(m, toks + q as u32 * 14) == 0)
                        {
                            q = q.wrapping_add(1);
                        }
                        r = if (q as u16) < nt {
                            check!(call!(
                                0x2c,
                                &[db[0], db[1], half(m, toks + q as u32 * 14) as u32, s + 2]
                            ))
                        } else {
                            check!(call!(0x24, &[db[0], db[1], 3, x, s + 2]))
                        };
                    }
                    for (field, data) in [(0, t + 6), (1, t + 2), (2, t + 4)] {
                        r = check!(call!(
                            0x50,
                            &[db[0], db[1], half(m, s + 2) as u32, field, 1, data, s + 6]
                        ));
                    }
                    sh(m, t, half(m, s + 2));
                    if m.byte(t + 12) != 0 {
                        r = check!(call!(
                            0x50,
                            &[db[0], db[1], half(m, s + 2) as u32, 6, 4, e.lid, s + 6]
                        ));
                    }
                    let w = half(m, wi + half(m, t + 2) as u32 * 2);
                    let st = m.word(strs + w as u32 * 4);
                    let buf = alloc!(
                        (sl(m, st) + sl(m, e.underscore) + sl(m, e.latin) + 1) as u16 as u32 + 1
                    );
                    if w != 0 {
                        cp(m, buf, st);
                        if m.byte(t + 10) != b'N' {
                            cat(m, buf, e.underscore);
                            let v = m.word(tab + m.byte(t + 10) as u32 * 4);
                            cat(m, buf, if v != 0 { v } else { e.latin });
                        }
                    } else if m.byte(t + 10) == b'N' {
                        cp(m, buf, m.word(strs));
                    } else {
                        let v = m.word(tab + m.byte(t + 10) as u32 * 4);
                        cp(m, buf, if v != 0 { v } else { e.latin });
                    }
                    r = call!(
                        0x50,
                        &[
                            db[0],
                            db[1],
                            half(m, s + 2) as u32,
                            4,
                            (sl(m, buf) + 1) as u16 as u32,
                            buf,
                            s + 6
                        ]
                    );
                    release_live(m, m.word(m.word(p) + 4), buf, h);
                    if r >= 0x80000000 {
                        return r;
                    }
                }
                3 => {
                    if half(m, t) != 0 {
                        r = check!(call!(0x60, &[db[0], db[1], half(m, t) as u32]));
                    }
                }
                1 => {
                    r = check!(call!(
                        0x50,
                        &[db[0], db[1], half(m, t) as u32, 2, 1, t + 4, s + 6]
                    ));
                }
                _ => {}
            }
        }
        if dbg {
            dump!(e.end_name);
            log!(b"Map =%s", &[cls]);
            log!(b"Sent=%s", &[text]);
        }
        r
    })();
    for q in [wi, lid, cls, toks] {
        if q != 0 {
            release_live(m, m.word(m.word(p) + 4), q, h);
        }
    }
    if strs != 0 {
        for i in 0..ns {
            release_live(m, m.word(m.word(p) + 4), m.word(strs + i as u32 * 4), h);
            m.put_word(strs + i as u32 * 4, 0);
        }
        release_live(m, m.word(m.word(p) + 4), strs, h);
    }
    for q in [tab, dbg_buf] {
        if q != 0 {
            release_live(m, m.word(m.word(p) + 4), q, h);
        }
    }
    m.bytes[at..at + 176].copy_from_slice(&saved);
    r
}

pub struct ProcessEnvironment {
    pub normal: u32,
    pub phon: u32,
    pub default16: u32,
    pub latin16: u32,
    pub user16: u32,
    pub section: u32,
    pub esctn: u32,
    pub esclang: u32,
    pub domain: u32,
    pub extra: u32,
    pub pipeline: u32,
    pub pause: u32,
    pub mapped_pause: u32,
    pub token: crate::speech_part_6::TokenConstants,
    pub original_name: u32,
    pub base_name: u32,
    pub format2: u32,
    pub format4: u32,
}
fn process_ling(m: &mut Memory<'_>, p: u32, slot: u32, args: &[u32], h: &mut impl Host) -> u32 {
    h.call(m, m.word(m.word(p + 24) + slot), args)
}
fn process_dict(
    m: &mut Memory<'_>,
    p: u32,
    key: u32,
    e: &ProcessEnvironment,
    s: u32,
    h: &mut impl Host,
) -> u32 {
    h.call(
        m,
        m.word(m.word(p + 20) + 28),
        &[
            m.word(p + 12),
            m.word(p + 16),
            e.section,
            key,
            s,
            s + 4,
            s + 6,
        ],
    )
}
fn len16(m: &Memory<'_>, p: u32) -> u32 {
    fe_text::strlen16_176624(m.bytes, p - m.base) as u32
}
fn eq16(m: &Memory<'_>, a: u32, b: u32) -> bool {
    fe_text::strcmp16_1765ac(m.bytes, a - m.base, b - m.base) == 0
}
fn chop_entry(m: &mut Memory<'_>, s: u32) -> u32 {
    let mapped = m.word(m.word(s));
    let q = fe_text::strchr_195d30(m.bytes, mapped - m.base, m.byte(s + 6) as u32);
    if q != 0 {
        sb(m, q + m.base, 0);
    }
    mapped
}
fn pause_map(
    m: &mut Memory<'_>,
    p: u32,
    prefix: u32,
    name: u32,
    mapped: u32,
    key: u32,
    value: u32,
    scratch: u32,
    h: &mut impl Host,
) {
    m.put_word(value, 0);
    cp(m, key, prefix);
    if sl(m, name) < 255u32.wrapping_sub(sl(m, key)) {
        cat(m, key, name);
        if uint_property(m, m.word(m.word(p) + 20), key, value, scratch, h) < 0x80000000 {
            cp(m, key, prefix);
            cat(m, key, mapped);
            // STUB bridge-4: pending live unsigned property setter.
            h.call(m, 0x1758ac, &[m.word(m.word(p) + 20), key, m.word(value)]);
        }
    }
}
fn known_domain(
    m: &mut Memory<'_>,
    p: u32,
    name: u32,
    e: &ProcessEnvironment,
    s: u32,
    h: &mut impl Host,
) -> bool {
    let (rc, setting) = string_property(m, m.word(m.word(p) + 20), e.domain, 0, s, h);
    if rc >= 0x80000000 || setting == 0 || m.byte(setting) == 0 {
        return false;
    }
    let q = frontend::find_substr(m.bytes, setting, name);
    q != 0
        && (q == setting || m.byte(q - 1) == b'|')
        && (m.byte(q + sl(m, name)) == 0 || m.byte(q + sl(m, name)) == b'|')
}
fn extra_type(
    m: &mut Memory<'_>,
    p: u32,
    name: u32,
    e: &ProcessEnvironment,
    s: u32,
    h: &mut impl Host,
) -> bool {
    let (rc, setting) = string_property(m, m.word(m.word(p) + 20), e.extra, 0, s, h);
    rc < 0x80000000 && setting != 0 && eq(m, setting, name)
}
fn tn_name<H: Host + crate::speech_part_6::Host>(
    m: &mut Memory<'_>,
    p: u32,
    language: u32,
    ty: u32,
    name: &mut u32,
    e: &ProcessEnvironment,
    s: u32,
    h: &mut H,
) -> Result<(u32, u32), u32> {
    let mut length = 0u16;
    let mut user = 0;
    let heap = m.word(m.word(p) + 4);
    let mut rc;
    macro_rules! oom {
        () => {{
            crate::speech_part_6::Host::error(
                h,
                m,
                m.word(m.word(p) + 16),
                e.token.category,
                0x7530,
            );
            return Err(0x8990200a);
        }};
    }
    macro_rules! resize {
        ($n:expr) => {{
            let q = realloc_live(m, heap, *name, $n, h);
            if q == 0 {
                oom!();
            }
            *name = q;
        }};
    }
    macro_rules! lookup {
        ($key:expr) => {{
            sh(m, s + 4, 0xffff);
            let r = process_dict(m, p, $key, e, s, h);
            if r >= 0x80000000 {
                return Err(r);
            }
            r
        }};
    }
    let max = core::cmp::max(len16(m, e.default16), len16(m, ty));
    resize!(len16(m, language) + max + 2);
    if eq16(m, language, e.default16) && eq16(m, ty, e.default16) {
        cp(m, *name, e.normal);
    } else {
        sb(m, *name, 0);
        let append = true;
        if !eq16(m, language, e.default16) {
            let mut i = 0;
            while half(m, language + i * 2) != 0 {
                sb(
                    m,
                    *name + length as u32,
                    normalization::tolower_240ba0(half(m, language + i * 2) as u8 as u32) as u8,
                );
                length = length.wrapping_add(1);
                i += 1;
            }
            sb(m, *name + length as u32, 0);
            if m.byte(p + 48) == 1 && !eq(m, *name, e.normal) && m.byte(*name) != 0 {
                lookup!(*name);
                if half(m, s + 4) != 0 {
                    let mapped = chop_entry(m, s);
                    let capacity = len16(m, ty) + 1 + if mapped != 0 { sl(m, mapped) } else { 0 };
                    resize!(capacity + 1);
                    if mapped != 0 {
                        cp(m, *name, mapped);
                    } else {
                        release_live(m, heap, *name, h);
                        sb(m, *name, 0);
                    }
                    sl(m, *name);
                }
            }
            lookup!(e.esctn);
            let mut default = false;
            if half(m, s + 4) != 0 {
                let delimiter = m.byte(s + 6) as u32;
                let types = m.word(m.word(s));
                let capacity = len16(m, ty) + 1;
                rc = 0x89902000;
                let mut replacement = *name;
                if rc & 0x1fff == 0 {
                    loop {
                        let q = frontend::find_substr(m.bytes, types, replacement);
                        if q != 0 {
                            let tail = m.byte(q + sl(m, replacement)) as u32;
                            if tail == 0 || tail == delimiter || tail == b'|' as u32 {
                                rc = 0;
                                break;
                            }
                        }
                        if rc < 0x80000000 {
                            break;
                        }
                        let colon = normalization::strrchr_at(m.bytes, replacement, b':' as u32);
                        if rc & 0x1fff != 0 || colon == 0 {
                            if replacement != *name {
                                release_live(m, heap, replacement, h);
                            }
                            replacement = 0;
                            break;
                        }
                        let mut colon = colon;
                        if replacement == *name {
                            replacement = 0;
                            let q = realloc_live(
                                m,
                                heap,
                                0,
                                if *name != 0 {
                                    sl(m, *name) + capacity + 1
                                } else {
                                    capacity + 1
                                },
                                h,
                            );
                            if q == 0 {
                                crate::speech_part_6::Host::error(
                                    h,
                                    m,
                                    m.word(m.word(p) + 16),
                                    e.token.category,
                                    0x7530,
                                );
                                rc = 0x8990200a;
                                break;
                            }
                            if *name != 0 {
                                cp(m, q, *name);
                            } else {
                                release_live(m, heap, q, h);
                                sb(m, q, 0);
                            }
                            replacement = q;
                            colon = normalization::strrchr_at(m.bytes, replacement, b':' as u32);
                        }
                        sb(m, colon, 0);
                    }
                } else if rc >= 0x80000000 {
                    if replacement != *name {
                        release_live(m, heap, replacement, h);
                    }
                    replacement = 0;
                }
                if rc < 0x80000000 {
                    if replacement != *name {
                        if !known_domain(m, p, *name, e, s + 16, h)
                            && !extra_type(m, p, *name, e, s + 16, h)
                        {
                            Host::call(
                                h,
                                m,
                                0x173dc0,
                                &[
                                    m.word(m.word(p) + 16),
                                    e.token.category,
                                    0x7535,
                                    e.format4,
                                    e.original_name,
                                    *name,
                                    e.base_name,
                                    replacement,
                                ],
                            );
                        }
                        release_live(m, heap, *name, h);
                        *name = replacement;
                    }
                } else {
                    if rc & 0x1fff != 0 {
                        return Err(rc);
                    }
                    default = true;
                }
            } else if ![
                b"normal".as_slice(),
                b"sms",
                b"address",
                b"spell",
                b"NOR",
                b"SMS",
                b"ADR",
            ]
            .iter()
            .any(|v| eq_lit(m, *name, v))
            {
                default = true;
            }
            if default {
                if !known_domain(m, p, *name, e, s + 16, h)
                    && !extra_type(m, p, *name, e, s + 16, h)
                {
                    Host::call(
                        h,
                        m,
                        0x173dc0,
                        &[
                            m.word(m.word(p) + 16),
                            e.token.category,
                            0x7534,
                            e.format2,
                            e.original_name,
                            *name,
                        ],
                    );
                }
                if eq16(m, ty, e.default16) {
                    resize!(sl(m, e.normal) + len16(m, ty) + 2);
                    cp(m, *name, e.normal);
                } else {
                    sb(m, *name, 0);
                }
            }
            if append {
                length = sl(m, *name) as u16;
                if !eq16(m, ty, e.default16) && !eq(m, *name, e.normal) && m.byte(*name) != 0 {
                    sb(m, *name + length as u32, b'_');
                    length = length.wrapping_add(1);
                }
            }
        }
        if !eq16(m, ty, e.default16) && !eq(m, *name, e.normal) {
            if len16(m, ty) == 7 && fe_text::strstr16_176538(m.bytes, ty, e.user16) != 0 {
                user = 1;
                sh(m, ty + 6, 0);
            }
            let mut copy_type = false;
            if len16(m, ty) == 3 {
                for j in 0..3 {
                    sb(
                        m,
                        s + 8 + j,
                        normalization::tolower_240ba0(half(m, ty + j * 2) as u8 as u32) as u8,
                    );
                }
                sb(m, s + 11, 0);
                lookup!(e.esclang);
                if half(m, s + 4) != 0
                    && frontend::find_substr(m.bytes, m.word(m.word(s)), s + 8) != 0
                {
                    copy_type = true;
                }
            } else if eq16(m, ty, e.latin16) {
                copy_type = true;
            }
            if copy_type {
                let mut i = 0;
                while half(m, ty + i * 2) != 0 {
                    sb(
                        m,
                        *name + length as u32,
                        normalization::tolower_240ba0(half(m, ty + i * 2) as u8 as u32) as u8,
                    );
                    length = length.wrapping_add(1);
                    i += 1;
                }
            } else if m.byte(*name) != 0 {
                sb(m, *name + length.wrapping_sub(1) as u32, 0);
            } else {
                cp(m, *name, e.normal);
            }
        }
        if !m.bytes[m.offset(*name)..].starts_with(
            &m.bytes[m.offset(e.normal)..m.offset(e.normal) + sl(m, e.normal) as usize],
        ) && m.byte(*name) != 0
        {
            sb(m, *name + length as u32, 0);
        }
    }
    rc = lookup!(*name);
    let mut mapped = *name;
    if half(m, s + 4) != 0 {
        mapped = chop_entry(m, s);
        pause_map(
            m,
            p,
            e.mapped_pause,
            *name,
            mapped,
            s + 144,
            s + 12,
            s + 16,
            h,
        );
    }
    let _ = rc;
    Ok((mapped, user))
}
struct ProcessCallee<'a, H> {
    host: &'a mut H,
    scratch: u32,
}
impl<H: Host + crate::speech_part_6::Host> crate::speech_part_6::Host for ProcessCallee<'_, H> {
    fn call(&mut self, m: &mut Memory<'_>, f: u32, a: &[u32]) -> u32 {
        if f == 0x175a1c {
            let (r, v) = string_property(m, a[0], a[1], m.word(a[2]), self.scratch, self.host);
            m.put_word(a[2], v);
            r
        } else {
            Host::call(self.host, m, f, a)
        }
    }
    fn log(&mut self, m: &mut Memory<'_>, l: u32, c: u32, v: u32, f: &[u8], a: &[u32]) {
        crate::speech_part_6::Host::log(self.host, m, l, c, v, f, a);
    }
    fn log_guest(&mut self, m: &mut Memory<'_>, l: u32, c: u32, v: u32, f: u32, a: &[u32]) {
        crate::speech_part_6::Host::log_guest(self.host, m, l, c, v, f, a);
    }
    fn error(&mut self, m: &mut Memory<'_>, l: u32, c: u32, n: u32) {
        crate::speech_part_6::Host::error(self.host, m, l, c, n);
    }
}
fn process_token<H: Host + crate::speech_part_6::Host>(
    m: &mut Memory<'_>,
    p: u32,
    db: [u32; 2],
    cursor: u32,
    start: u32,
    end: u16,
    current: u32,
    user_text: u32,
    ty: u32,
    fallback: u32,
    text: u32,
    user: u32,
    e: &ProcessEnvironment,
    s: u32,
    h: &mut H,
) -> u32 {
    crate::speech_part_6::fn_1d5234(
        m,
        &mut ProcessCallee {
            host: h,
            scratch: s + 64,
        },
        p,
        db,
        cursor,
        start,
        end,
        current,
        user_text,
        ty,
        fallback,
        text,
        user,
        &e.token,
        s,
    )
}
/// fn_1d5ba8. Real resource methods, dictionary callbacks, property lookup,
/// token builder and sort. scratch is 768 disjoint restored guest bytes.
pub fn fn_1d5ba8<H: Host + crate::speech_part_6::Host>(
    m: &mut Memory<'_>,
    p: u32,
    tag: u32,
    db: [u32; 2],
    pending: u32,
    e: &ProcessEnvironment,
    scratch: u32,
    h: &mut H,
) -> u32 {
    if frontend::validate_handle(p != 0, tag, 0xf380, 0x4c) >= 0x80000000 {
        return 0x89902008;
    }
    assert_eq!(m.base, 0);
    let at = m.offset(scratch);
    let mut saved = [0; 768];
    saved.copy_from_slice(&m.bytes[at..at + 768]);
    let mut tokens = 0;
    let mut name = 0;
    let mut user_buffer = 0;
    let mut clean_name = false;
    let r = (|| {
        let s = scratch;
        // root,cursor,written,finished,end; textAvailable,tokenAvailable,textData,textBytes,tokenData,tokenBytes
        fill(m, s, 64, 0);
        let mut current = e.normal;
        let mut fallback = e.normal;
        let mut ty = e.default16;
        let mut language = e.default16;
        let mut user_active = false;
        let mut user_text = 0;
        let mut user = 0;
        sb(m, pending, 1);
        macro_rules! check {
            ($expr:expr) => {{
                let r = $expr;
                if r >= 0x80000000 {
                    return r;
                }
                r
            }};
        }
        macro_rules! res {
            ($slot:expr,$i:expr,$tail:expr) => {{
                let b = p + 32 + $i * 8;
                let t = $tail;
                let mut a = [0; 7];
                a[0] = m.word(b);
                a[1] = m.word(b + 4);
                a[2..2 + t.len()].copy_from_slice(t);
                Host::call(h, m, m.word(m.word(p + 28) + $slot), &a[..2 + t.len()])
            }};
        }
        macro_rules! ling {
            ($slot:expr,$tail:expr) => {{
                let t = $tail;
                let mut a = [0; 7];
                a[..2].copy_from_slice(&db);
                a[2..2 + t.len()].copy_from_slice(t);
                process_ling(m, p, $slot, &a[..2 + t.len()], h)
            }};
        }
        macro_rules! set {
            ($field:expr,$n:expr,$data:expr) => {
                ling!(0x50, &[half(m, s) as u32, $field, $n, $data, s + 4])
            };
        }
        macro_rules! log {
            ($fmt:expr,$a:expr) => {
                crate::speech_part_6::Host::log(
                    h,
                    m,
                    m.word(m.word(p) + 16),
                    e.token.category,
                    5,
                    $fmt,
                    $a,
                )
            };
        }
        macro_rules! error {
            ($code:expr) => {
                crate::speech_part_6::Host::error(
                    h,
                    m,
                    m.word(m.word(p) + 16),
                    e.token.category,
                    $code,
                )
            };
        }
        macro_rules! set_checked {
            ($expr:expr) => {{
                let r = $expr;
                if r >= 0x80000000 {
                    error!(0x7532);
                    return r;
                }
                r
            }};
        }
        let mut rc = check!(res!(0x48, 0, &[s + 12, s + 9]));
        rc = check!(res!(0x48, 1, &[s + 16, s + 9]));
        rc = check!(ling!(0x1c, &[]));
        rc = check!(ling!(0x34, &[1, 0, s]));
        if m.word(s + 12) == 0 {
            if m.byte(s + 9) == 0 && m.word(s + 16) == 0 {
                let (r, pipeline) =
                    string_property(m, m.word(m.word(p) + 20), e.pipeline, 0, s + 64, h);
                if r >= 0x80000000 || pipeline == 0 || m.byte(pipeline) == 0 {
                    sb(m, pending, 0);
                }
            } else {
                if m.word(s + 16) != 0 {
                    rc = check!(res!(0x2c, 1, &[s + 28, s + 32]));
                    rc = set_checked!(set!(1, (m.word(s + 32) / 24) as u16 as u32, m.word(s + 28)));
                    rc = check!(res!(0x30, 1, &[m.word(s + 32)]));
                }
                if m.byte(s + 9) != 0 {
                    sb(m, pending, 1);
                    sb(m, s + 8, 1);
                }
            }
            return set!(2, 1, s + 8);
        }
        rc = check!(set!(2, 1, s + 8));
        rc = check!(res!(0x2c, 0, &[s + 20, s + 24]));
        if m.word(s + 24) == 1 {
            rc = check!(res!(0x30, 0, &[1]));
            rc = check!(res!(0x2c, 1, &[s + 28, s + 32]));
            if m.word(s + 16) != 0 {
                rc = set_checked!(set!(1, (m.word(s + 32) / 24) as u16 as u32, m.word(s + 28)));
            }
            rc = res!(0x30, 1, &[m.word(s + 32)]);
            return if rc < 0x80000000 { 0 } else { rc };
        }
        rc = set_checked!(set!(0, m.word(s + 24) as u16 as u32, m.word(s + 20)));
        log!(b"Initializing LINGDB with %s", &[m.word(s + 20)]);
        rc = check!(res!(0x2c, 1, &[s + 28, s + 32]));
        let count = (m.word(s + 32) / 24) as u16;
        tokens = frontend::heap_alloc_m(
            m.bytes,
            Some(m.word(m.word(p) + 4)),
            m.word(s + 32),
            &mut LiveHeap {
                host: h,
                table: m.word(m.word(m.word(p) + 4)),
            },
        );
        if tokens == 0 {
            error!(0x7530);
            return 0x8990200a;
        }
        move_bytes(m, tokens, m.word(s + 28), m.word(s + 32));
        for i in 0..count {
            let t = tokens + i as u32 * 24;
            if half(m, t) == 1 {
                m.put_word(t + 16, 0);
            }
        }
        rc = check!(Host::call(
            h,
            m,
            m.word(m.word(p + 20) + 44),
            &[
                m.word(p + 12),
                m.word(p + 16),
                db[0],
                db[1],
                tokens,
                count as u32
            ]
        ));
        rc = check!(ling!(0x34, &[3, half(m, s) as u32, s + 2]));
        let first = m.word(tokens + 12);
        let mut last = first;
        clean_name = true;
        sh(m, s + 40, 0xffff);
        rc = check!(process_dict(m, p, current, e, s + 36, h));
        if half(m, s + 40) != 0 {
            let mapped = chop_entry(m, s + 36);
            pause_map(m, p, e.pause, current, mapped, s + 320, s + 48, s + 64, h);
            current = m.word(m.word(s + 36));
        }
        let mut index = 0u16;
        while index < count {
            let t = tokens + index as u32 * 24;
            let kind = half(m, t);
            if kind != 21 && kind != 34 && kind != 36 {
                index += 1;
                continue;
            }
            let mut length = 0u16;
            let mut ti = 0;
            let mut li = 0;
            let mut next = index.wrapping_add(1);
            while next < count
                && m.word(tokens + next as u32 * 24 + 12) == m.word(t + 12)
                && half(m, tokens + next as u32 * 24) != 34
            {
                let k = half(m, tokens + next as u32 * 24);
                if k == 36 {
                    ti = next;
                } else if k == 21 {
                    li = next;
                }
                next = next.wrapping_add(1);
            }
            if user_active && kind == 36 && li == 0 {
                index += 1;
                continue;
            }
            if m.word(t + 12) != first {
                while [b' ', b'_', b'\t', b'\n', b'\r'].contains(
                    &m.byte(
                        m.word(s + 20)
                            .wrapping_add(m.word(t + 12).wrapping_sub(first)),
                    ),
                ) {
                    m.put_word(t + 12, m.word(t + 12).wrapping_add(1));
                }
                if m.word(t + 12) != last {
                    log!(
                        b"Creating USER TN %s token at pos  %u,%u in lingdb",
                        &[
                            current,
                            last.wrapping_sub(first) as u16 as u32,
                            m.word(t + 12).wrapping_sub(first) as u16 as u32
                        ]
                    );
                    rc = check!(process_token(
                        m,
                        p,
                        db,
                        s + 2,
                        last.wrapping_sub(first) as u16 as u32,
                        m.word(t + 12).wrapping_sub(first) as u16,
                        current,
                        user_text,
                        ty,
                        fallback,
                        m.word(s + 20),
                        user,
                        e,
                        s + 64,
                        h
                    ));
                }
            }
            if kind == 21 {
                language = m.word(t + 20);
            } else if kind == 36 {
                ty = m.word(t + 20);
            }
            if ti != 0 {
                ty = m.word(tokens + ti as u32 * 24 + 20);
            }
            if li != 0 {
                language = m.word(tokens + li as u32 * 24 + 20);
            }
            user = 0;
            if kind == 34 {
                user_active = true;
                let q = realloc_live(
                    m,
                    m.word(m.word(p) + 4),
                    user_buffer,
                    len16(m, m.word(t + 20)) + 1,
                    h,
                );
                if q == 0 {
                    error!(0x7530);
                    clean_name = false;
                    return 0x8990200a;
                }
                user_buffer = q;
                let mut i = 0;
                while half(m, m.word(t + 20) + i * 2) != 0 {
                    let c = half(m, m.word(t + 20) + i * 2) as u8;
                    if c != b' ' {
                        sb(m, user_buffer + length as u32, c);
                        length = length.wrapping_add(1);
                    }
                    i += 1;
                }
                current = e.phon;
                user_text = user_buffer;
                sb(m, user_buffer + length as u32, 0);
                if name != 0 {
                    fallback = name;
                }
            } else {
                user_active = false;
                match tn_name(m, p, language, ty, &mut name, e, s + 36, h) {
                    Ok((v, u)) => {
                        current = v;
                        user = u;
                    }
                    Err(r) => return r,
                }
                user_text = 0;
            }
            last = m.word(t + 12);
            index = next;
        }
        log!(b"Creating USER TN %s token in lingdb", &[current]);
        rc = check!(process_token(
            m,
            p,
            db,
            s + 2,
            last.wrapping_sub(first) as u16 as u32,
            m.word(s + 24).wrapping_sub(1) as u16,
            current,
            user_text,
            ty,
            fallback,
            m.word(s + 20),
            user,
            e,
            s + 64,
            h
        ));
        let mut index = 0;
        while index < count && m.word(tokens + index as u32 * 24 + 12) == first {
            index += 1;
        }
        fn order(arena: &[u8], a: u32, b: u32) -> i32 {
            normalization::token_position_order(
                &arena[a as usize..a as usize + 24],
                &arena[b as usize..b as usize + 24],
            )
        }
        fe_text::qsort_2409d4(
            m.bytes,
            tokens + index as u32 * 24,
            (count - index) as u32,
            24,
            order,
        );
        rc = set_checked!(set!(1, count as u32, tokens));
        rc = check!(res!(0x30, 0, &[m.word(s + 24)]));
        rc = res!(0x30, 1, &[m.word(s + 32)]);
        rc
    })();
    if clean_name && name != 0 {
        release_live(m, m.word(m.word(p) + 4), name, h);
    }
    if user_buffer != 0 {
        release_live(m, m.word(m.word(p) + 4), user_buffer, h);
    }
    if tokens != 0 {
        release_live(m, m.word(m.word(p) + 4), tokens, h);
    }
    m.bytes[at..at + 768].copy_from_slice(&saved);
    r
}

// Reentrant conversions of the parameter-set call tree. Every callback gets the
// current arena, including comparison, allocation, before/after and unlock.
struct ParamCalls<'a, H> { host: &'a mut H, scratch: u32 }
impl<H: Host> crate::boot::Calls for ParamCalls<'_, H> {
    fn call(&mut self, m:&mut Memory<'_>, f:u32, a:&[u32])->u32 {
        parameter_call_m(m,f,a,self.scratch,self.host)
    }
    fn call_wide(&mut self,_:&mut Memory<'_>,_:u32,_:&[u32])->u64 { panic!("parameter wide call") }
}
struct ParamTree<'a,'h,H> {
    cell:&'a core::cell::RefCell<&'h mut H>, map:u32, scratch:u32,
}
impl<H:Host> frontend::TreeOpsM for ParamTree<'_,'_,H> {
    fn tree_cmp_m(&mut self,key:u32,node:u32,bytes:&mut [u8])->i32 {
        let mut m=Memory{base:0,bytes};
        let f=m.word(m.word(self.map+60)+8);
        parameter_call_m(&mut m,f,&[key,node],self.scratch,&mut **self.cell.borrow_mut()) as i32
    }
    fn tree_init_m(&mut self,key:u32,heap:u32,_node:u32,bytes:&mut [u8])->(u32,Option<u32>) {
        let mut m=Memory{base:0,bytes};
        let f=m.word(m.word(self.map+60));
        m.put_word(self.scratch,0);
        let rc=parameter_call_m(&mut m,f,&[key,heap,self.scratch],self.scratch+16,&mut **self.cell.borrow_mut());
        (rc,Some(m.word(self.scratch)))
    }
    fn tree_alloc_m(&mut self,arg:u32,heap:u32,bytes:&mut [u8])->(u32,u32) {
        let mut m=Memory{base:0,bytes};let f=m.word(m.word(self.map+60)+12);
        m.put_word(self.scratch,0);
        let rc=parameter_call_m(&mut m,f,&[arg,heap,self.scratch],self.scratch+16,&mut **self.cell.borrow_mut());
        (rc,m.word(self.scratch))
    }
    fn tree_free_node_m(&mut self,p:u32,heap:u32,bytes:&mut [u8]) {
        let mut m=Memory{base:0,bytes};let f=m.word(m.word(self.map+60)+4);
        parameter_call_m(&mut m,f,&[p,heap],self.scratch,&mut **self.cell.borrow_mut());
    }
    fn tree_free_handle_m(&mut self,p:u32,heap:u32,bytes:&mut [u8]) {
        let mut m=Memory{base:0,bytes};let f=m.word(m.word(self.map+60)+16);
        parameter_call_m(&mut m,f,&[p,heap],self.scratch,&mut **self.cell.borrow_mut());
    }
}
struct ParamHeap<'a,'h,H>{cell:&'a core::cell::RefCell<&'h mut H>,heap:u32,scratch:u32}
impl<H:Host> frontend::HeapVtableM for ParamHeap<'_,'_,H> {
    fn alloc_at_m(&mut self,obj:u32,n:u32,bytes:&mut [u8])->u32 {
        let mut m=Memory{base:0,bytes};let f=m.word(m.word(self.heap));
        parameter_call_m(&mut m,f,&[obj,n],self.scratch,&mut **self.cell.borrow_mut())
    }
    fn realloc_at_m(&mut self,obj:u32,p:u32,n:u32,bytes:&mut [u8])->u32 {
        let mut m=Memory{base:0,bytes};let f=m.word(m.word(self.heap)+8);
        parameter_call_m(&mut m,f,&[obj,p,n],self.scratch,&mut **self.cell.borrow_mut())
    }
    fn release_at_m(&mut self,obj:u32,p:u32,bytes:&mut [u8]) {
        let mut m=Memory{base:0,bytes};let f=m.word(m.word(self.heap)+12);
        parameter_call_m(&mut m,f,&[obj,p],self.scratch,&mut **self.cell.borrow_mut());
    }
}
fn param_callbacks(m:&mut Memory<'_>,p:u32,key:u32,value:u32,data:u32,s:u32,before:bool,h:&mut impl Host)->u32 {
    if p==0 || key==0 || value==0 || data==0 {return 0;}
    let mut rc=0;
    m.put_byte(s,1);
    for pass in 0..2 {
        let list=if pass==0 {m.word(value+8)} else {m.word(p+12)};
        if list==0 {continue;}
        let count=m.word(list+20);
        let mut i=0;
        while i<count && rc<0x80000000 {
            let table=frontend::ElementTable{data:&[],elem_size:half(m,list+4),count:m.word(list+20)};
            let mut off=0;
            if frontend::element_at(Some(&table),i,Some(&mut off))==0 {return 0x8b702100;}
            let c=m.word(list).wrapping_add(off);
            let f=m.word(c+if before {4}else{8});
            if before {
                if f!=0 {
                    rc=h.call(m,f,&[m.word(c),key,data,s]);
                    if rc<0x80000000 && m.byte(s)==0 {rc=0x8b70200f;break;}
                }
            } else {
                rc=h.call(m,f,&[m.word(c),key,data]);
                if rc>=0x80000000 {return rc;}
            }
            i+=1;
        }
    }
    if before {rc}else{0}
}
/// Live bindings used by fn_175170 and its string/uint callers. Scratch is 512
/// disjoint bytes; nested calls use a later range and all scratch is restored.
pub fn parameter_call_m(m:&mut Memory<'_>,f:u32,a:&[u32],s:u32,h:&mut impl Host)->u32 {
    assert_eq!(m.base,0);
    let off=m.offset(s);let mut saved=[0u8;512];saved.copy_from_slice(&m.bytes[off..off+512]);
    let r=(|| match f {
        0x175170=>fn_175170(m,a[0],a[1],a[2],a[3],a[4],s,h),
        0x196980=>fn_196980(m,a[0],a[1]),
        0x175850=>{
            let mut table=[0u16;256];
            for i in 0..256 {table[i]=if (b'0'..=b'9').contains(&(i as u8)) {4}else if i==32||(9..=13).contains(&i) {8}else{0};}
            int_property(m,a[0],a[1],a[2],s,&table,h)
        },
        0x176104=>param_callbacks(m,a[0],a[1],a[2],a[3],s,false,h),
        0x1966ac=>{
            let mut value=0;
            let map=a[0];
            let r=crate::reentrant2::lookup_value_m(m.bytes,if map==0 {None}else{Some(map)},a[1],Some(&mut value),|x,y,bytes|{
                let mut m=Memory{base:0,bytes};let f=m.word(m.word(map+60)+8);
                parameter_call_m(&mut m,f,&[x,y],s+16,h) as i32
            });
            if r<0x80000000 {m.put_word(a[2],value);}r
        }
        0x19668c=>{
            let map=a[0];let heap=if map==0 {0}else{m.word(map+56)};
            let cell=core::cell::RefCell::new(&mut *h);
            frontend::insert_guarded_m(m.bytes,if map==0 {None}else{Some(map)},a[1],a[2],
                &mut ParamHeap{cell:&cell,heap,scratch:s+64},
                &mut ParamTree{cell:&cell,map,scratch:s+32})
        }
        0x176054=>{
            let [slot,src,n,heap]:[u32;4]=a.try_into().unwrap();
            if slot==0 {return 0x8b702007;}
            let cell=core::cell::RefCell::new(&mut *h);
            let mut vt=ParamHeap{cell:&cell,heap,scratch:s+16};
            if n==0 {
                if m.word(slot)!=0 && m.word(slot+4)!=0 {frontend::heap_release_m(m.bytes,if heap==0 {None}else{Some(heap)},Some(m.word(slot)),&mut vt);}
                m.put_word(slot+4,0);m.put_word(slot,src);
            } else {
                let size=m.word(slot+4);
                if size==0 {
                    let p=frontend::heap_alloc_m(m.bytes,if heap==0 {None}else{Some(heap)},n,&mut vt);
                    m.put_word(slot,p);if p==0 {return 0x8b70200a;}
                } else if size<n {
                    let old=m.word(slot);
                    let p=frontend::heap_realloc_m(m.bytes,if heap==0 {None}else{Some(heap)},old,n,&mut vt);
                    if p==0 {return 0x8b70200a;}m.put_word(slot,p);
                }
                let dst=m.word(slot);fe_text::memcpy_195ae8(m.bytes,dst,src,n);m.put_word(slot+4,n);
            }0
        }
        0x174b38=>{
            let [p,key,data,size,update]:[u32;5]=a.try_into().unwrap();
            m.put_word(s,data);m.put_word(s+4,size);m.put_word(s+8,0);
            let list=m.word(p+16);let mut i=0;let mut r=0;
            while list!=0 && i<m.word(list+20) {
                let ent=m.word(list)+i*half(m,list+4) as u32;
                r=parameter_call_m(m,0x19668c,&[m.word(ent+4),key,s],s+64,h);
                if r&0x1fff==2 {
                    if update==0 {r=0;}else{
                        m.put_word(s+16,0);
                        r=parameter_call_m(m,0x1966ac,&[m.word(ent+4),key,s+16],s+64,h);
                        if r>=0x80000000 {r=(r&0x1fff)|0x8b702000;}else{
                            r=parameter_call_m(m,0x176054,&[m.word(s+16),data,size,m.word(m.word(ent))],s+64,h);
                        }
                    }
                }
                i+=1;if r>=0x80000000 {break;}
            }r
        }
        _=> {
            let mut calls=ParamCalls{host:h,scratch:s+32};
            if let Some(r)=m.boot_parameter_call(&mut calls,f,a,s) {r}else{calls.host.call(m,f,a)}
        }
    })();
    m.bytes[off..off+512].copy_from_slice(&saved);r
}
/// fn_175170. The lookup, callback gates, attribute propagation, insertion,
/// storage and notification all execute on live caller memory.
pub fn fn_175170(m:&mut Memory<'_>,p:u32,key:u32,data:u32,size:u32,update:u32,s:u32,h:&mut impl Host)->u32 {
    if p==0 || key==0 || data==0 {return 0x8b702007;}
    let off=m.offset(s);let mut saved=[0u8;64];saved.copy_from_slice(&m.bytes[off..off+64]);
    let rc=(||{
        let mut result=parameter_call_m(m,0x176f2c,&[m.word(p+4)],s+64,h);
        if result>=0x80000000 {return result;}
        m.put_word(s,0);
        let lookup=parameter_call_m(m,0x1966ac,&[m.word(p+8),key,s],s+64,h);
        let v=m.word(s);
        let old_size=if v==0 {0}else{m.word(v+4)};
        let old_data=if old_size==0 {0}else{m.word(v)};
        let changed=lookup>=0x80000000 || old_size!=size ||
            frontend::compare_bytes(&m.bytes[old_data as usize..old_data as usize+size as usize],&m.bytes[data as usize..data as usize+size as usize],size)!=0;
        if changed {
            let incoming=if lookup>=0x80000000 {
                m.put_word(s+4,data);m.put_word(s+8,size);m.put_word(s+12,0);s+4
            }else{v};
            result=param_callbacks(m,p,key,incoming,data,s+16,true,h);
            if result<0x80000000 {
                result=parameter_call_m(m,0x174b38,&[p,key,if update!=0 {data}else{old_data},if update!=0 {size}else{old_size},update],s+64,h);
                if result<0x80000000 {
                    result=if lookup<0x80000000 {
                        parameter_call_m(m,0x176054,&[v,data,size,m.word(p)],s+64,h)
                    }else{parameter_call_m(m,0x19668c,&[m.word(p+8),key,incoming],s+64,h)};
                    if result>=0x80000000 && lookup>=0x80000000 {result=(result&0x1fff)|0x8b702000;}
                    else if result<0x80000000 {
                        result=param_callbacks(m,p,key,incoming,data,s+16,false,h);
                        if result>=0x80000000 && lookup>=0x80000000 {result=(result&0x1fff)|0x8b702000;}
                    }
                }
            }
        }
        let r=parameter_call_m(m,0x176f5c,&[m.word(p+4)],s+64,h);
        if r>=0x80000000 && result<0x80000000 {r}else{result}
    })();
    m.bytes[off..off+64].copy_from_slice(&saved);rc
}

/// Regional constants needed by the sequence reset.
pub struct SequenceEnvironment {
    pub cached_handle:[u8;8], pub default_context:u32,
    pub eos:u32, pub direct_size:u32,
}
/// fn_2643f8, built in fn_263384. Engine calls must be bound to live ports.
pub fn fn_2643f8(m:&mut Memory<'_>,p:u32,a:u32,b:u32,e:&SequenceEnvironment,s:u32,h:&mut impl Host)->u32 {
    if p==0 {return 0x81102007;}
    let (r,t)=crate::render_part_c::resolve(m,[m.word(p),m.word(p+4)],&crate::render_part_c::Environment{cached_handle:e.cached_handle,default_context:e.default_context});
    if r>=0x80000000 {return 0x81102007;}
    let off=m.offset(s);let saved:[u8;16]=m.bytes[off..off+16].try_into().unwrap();
    m.put_word(s,0);m.put_word(s+4,0);m.put_word(s+8,0);
    let rc=(||{
        m.put_word(p+12,a);m.put_word(p+16,b);
        let r=parameter_call_m(m,0x175850,&[m.word(t+20),e.eos,s],s+64,h);
        m.put_byte(p+0x76,u8::from(r>=0x80000000 || m.word(s)!=0));
        let level=h.call(m,0x173fd8,&[m.word(t+16)]);
        m.put_byte(p+0x78,u8::from(level>=5));
        for at in [0x18,0x28,0x29] {m.put_byte(p+at,0);}
        for at in [0x1c,0x20,0x14,0x24] {m.put_word(p+at,0);}
        m.put_half(p+0x30,0);m.put_word(p+0x2c,0x400);
        let r=uint_property(m,m.word(t+20),e.direct_size,s+4,s+64,h);
        if r<0x80000000 && m.word(s+4)>=0x100 {m.put_word(p+0x2c,m.word(s+4));}
        let r=h.call(m,0x2c47d0,&[m.word(p+0x34),s+8]);if r>=0x80000000 {return r;}
        let r=h.call(m,0x2c2338,&[m.word(p+0x38),m.word(s+8)]);if r>=0x80000000 {return r;}
        if m.byte(p+0x77)!=0 {let r=h.call(m,0x2ba608,&[m.word(p+0x3c)]);if r>=0x80000000 {return r;}}
        0
    })();
    m.bytes[off..off+16].copy_from_slice(&saved);rc
}

fn rule_io<H:Host+crate::speech_part_6::Host>(m:&mut Memory<'_>,p:u32,id:u32,value:u32,field:u32,s:u32,h:&mut H)->u32 {
    m.put_half(s,0);m.put_half(s+4,0);
    let r=Host::call(h,m,m.word(m.word(p+12)+92),&[m.word(p+16),m.word(p+20),id,field,s]);
    let got=half(m,s);
    let mut second=0;
    if r<0x80000000 && got==1 {
        second=Host::call(h,m,m.word(m.word(p+12)+84),&[m.word(p+16),m.word(p+20),id,field,1,value,s+4]);
    }
    // Existing rule leaf consumes the actual callback results, not recordings.
    let mut frame=[0u8;0x530];
    for (at,v) in [(0x504,r),(0x508,got as u32),(0x518,0),(0x51c,half(m,s+4) as u32)] {
        frame[at..at+4].copy_from_slice(&v.to_le_bytes());
    }
    let rc=match field {
        1=>normalization::rule_store_first(&mut frame,0,0,0),
        2=>normalization::rule_store_second(&mut frame,0,0,0),
        _=>normalization::rule_store_query(&mut frame,0,0,0),
    };
    if rc>=0x80000000 {
        crate::speech_part_6::Host::error(h,m,m.word(m.word(p)+16),m.word(p+0x48),if field==0 {0x5de0}else if (r<0x80000000&&got==1)==(field==1) {0x5dde}else{0x5ddf});
    }
    rc
}
fn token_log<H:Host+crate::speech_part_6::Host>(m:&mut Memory<'_>,p:u32,level:u32,fmt:&[u8],a:&[u32],h:&mut H) {
    crate::speech_part_6::Host::log(h,m,m.word(m.word(p)+16),m.word(p+0x48),level,fmt,a);
}
/// fn_24ac00. Scratch is 160 disjoint bytes, restored on every exit. The
/// stream vtable may reenter and change the arena during every query.
pub fn fn_24ac00<H:Host+crate::speech_part_6::Host>(m:&mut Memory<'_>,p:u32,name:u32,mode:u32,s:u32,h:&mut H)->u32 {
    let off=m.offset(s);let mut saved=[0u8;160];saved.copy_from_slice(&m.bytes[off..off+160]);
    let rc=(||{
        if p==0 || m.word(p+0x5c)==0 {
            crate::speech_part_6::Host::error(h,m,m.word(m.word(p)+16),m.word(p+0x48),0x5df1);
            return normalization::error_code(7);
        }
        let prefix=if mode==1 {b"<LOGFE> ".as_slice()}else{b"".as_slice()};
        for (i,&b) in prefix.iter().chain(core::iter::once(&0)).enumerate(){m.put_byte(s+64+i as u32,b);}
        token_log(m,p,4,b"%stokentn : dumpTokenRecordsInLingDB : Begin : %s",&[s+64,name],h);
        for at in [s,s+2,s+4,s+6] {m.put_half(at,0);}m.put_word(s+12,0);m.put_word(s+16,0);m.put_byte(s+8,0);
        let mut rc=Host::call(h,m,m.word(m.word(p+12)+52),&[m.word(p+16),m.word(p+20),1,0,s]);
        if rc<0x80000000 {
            rc=Host::call(h,m,m.word(m.word(p+12)+52),&[m.word(p+16),m.word(p+20),3,half(m,s) as u32,s+2]);
            if rc==0 {
                while half(m,s+2)!=0 {
                    let cur=half(m,s+2) as u32;
                    rc=rule_io(m,p,cur,s+4,1,s+96,h);if rc>=0x80000000 {break;}
                    rc=rule_io(m,p,cur,s+6,2,s+96,h);if rc>=0x80000000 {break;}
                    rc=rule_io(m,p,cur,s+8,0,s+96,h);if rc>=0x80000000 {break;}
                    let kind=m.byte(s+8);
                    if kind==5 || kind==6 {
                        m.put_byte(s+32,0);m.put_half(s+20,0);m.put_half(s+22,0);rc=0;
                        let r=Host::call(h,m,m.word(m.word(p+12)+92),&[m.word(p+16),m.word(p+20),cur,4,s+20,s+32]);
                        if r<0x80000000 && half(m,s+20)==1 {
                            Host::call(h,m,m.word(m.word(p+12)+84),&[m.word(p+16),m.word(p+20),cur,4,32,s+32,s+22]);
                        }
                        if kind==6 {
                            m.put_word(s+12,0);
                            for (field,dst) in [(3,s+12),(6,s+16)] {
                                m.put_half(s+24,0);m.put_half(s+26,0);
                                let r=Host::call(h,m,m.word(m.word(p+12)+92),&[m.word(p+16),m.word(p+20),cur,field,s+24,dst]);
                                if r<0x80000000 && half(m,s+24)==1 {
                                    Host::call(h,m,m.word(m.word(p+12)+88),&[m.word(p+16),m.word(p+20),cur,field,dst,s+26]);
                                }
                            }
                        }
                    }
                    let begin=half(m,s+4) as u32;let end=half(m,s+6) as u32;
                    if end<begin || end>half(m,p+0x60) as u32 {
                        // Error formatting uses guest literals supplied by the host.
                        Host::call(h,m,0x173dc0,&[m.word(m.word(p)+16),m.word(p+0x48),0x5df7,begin,end,half(m,p+0x60) as u32]);
                        return normalization::error_code(0);
                    }
                    let dst=m.word(p+0xc8);let src=m.word(p+0x5c)+begin;
                    normalization::strn_copy(m.bytes,dst,src,end-begin);m.put_byte(dst+end-begin,0);
                    if kind<=4 {
                        m.put_word(s+28,0);
                        Host::call(h,m,m.word(m.word(p+12)+128),&[m.word(p+16),m.word(p+20),kind as u32,s+28]);
                        token_log(m,p,5,b"%s[dbId=%d] %3d,%3d %-15s \"%s\"",&[s+64,cur,begin,end,m.word(s+28),m.word(p+0xc8)],h);
                    }else if kind==6 {
                        token_log(m,p,5,b"%s[dbId=%d] %3d,%3d %-15s \"%s\" \"%s\" langMap=\"%s\"",&[s+64,cur,begin,end,s+32,m.word(p+0xc8),m.word(s+12),m.word(s+16)],h);
                    }else if kind==5 {
                        token_log(m,p,5,b"%s[dbId=%d] %3d,%3d ESC(%-9s) \"%s\" ",&[s+64,cur,begin,end,s+32,m.word(p+0xc8)],h);
                    }
                    if Host::call(h,m,m.word(m.word(p+12)+60),&[m.word(p+16),m.word(p+20),cur,s+2])!=0 {break;}
                }
            }
        }
        token_log(m,p,4,b"%stokentn : dumpTokenRecordsInLingDB End (%x)",&[s+64,rc],h);rc
    })();
    m.bytes[off..off+160].copy_from_slice(&saved);rc
}
/// fn_249cec. Uses 1152 disjoint scratch bytes, restored on return. Host
/// engine calls are live bindings, never captured rule or preparation results.
pub fn fn_249cec<H:Host+crate::speech_part_6::Host>(m:&mut Memory<'_>,p:u32,s:u32,h:&mut H)->u32 {
    let off=m.offset(s);let mut saved=[0u8;1152];saved.copy_from_slice(&m.bytes[off..off+1152]);
    let rc=(||{
        for i in 0..16 {m.put_byte(s+1000+i,0);}
        let begin=s+1000;let end=s+1002;let count=s+1004;let block=s+1006;let domain=s+1008;let token=s+1010;let next=s+1012;let tag=s+1014;
        let kind=s+1016;let trigger=s+1017;let db=s+1020;let active=s+1024;
        m.put_byte(kind,0);m.put_byte(trigger,0);m.put_word(db,0);m.put_word(active,0);
        token_log(m,p,4,b"tokentn : AllInOneLoop Begin",&[],h);
        let state=m.word(p+0x40);
        m.put_word(m.word(m.word(state)+4)+16,p);
        let mut rc=Host::call(h,m,0x24bbc8,&[p]);if rc>=0x80000000 {return rc;}
        rc=Host::call(h,m,0x24c470,&[p]);if rc>=0x80000000 {return rc;}
        m.put_word(m.word(m.word(state)+4)+8,m.word(p+0x5c));
        m.put_half(m.word(state)+28,0);m.put_half(m.word(state)+28,half(m,m.word(state)+28)|0x10);
        m.put_half(m.word(state)+18,half(m,p+0x60));
        rc=Host::call(h,m,m.word(m.word(p+12)+52),&[m.word(p+16),m.word(p+20),1,0,tag]);
        if rc>=0x80000000 {return rc;}
        rc=Host::call(h,m,m.word(m.word(p+12)+52),&[m.word(p+16),m.word(p+20),3,half(m,tag) as u32,token]);
        if rc!=0 {token_log(m,p,5,b"no basic token records to process",&[],h);return rc;}
        if half(m,token)!=0 {token_log(m,p,4,b"tokentn : start loop...",&[],h);}
        'work: while half(m,token)!=0 {
            rc=rule_io(m,p,half(m,token) as u32,begin,1,s+1080,h);if rc>=0x80000000 {return rc;}
            rc=rule_io(m,p,half(m,token) as u32,end,2,s+1080,h);if rc>=0x80000000 {return rc;}
            rc=rule_io(m,p,half(m,token) as u32,kind,0,s+1080,h);if rc>=0x80000000 {return rc;}
            if m.byte(kind)<=2 {
                m.put_half(count,0);
                crate::speech_part_2::fn_24eac4(m,p,s,count,half(m,begin) as i32,half(m,end) as i32,0);
                token_log(m,p,5,b"START basic token %d,%d ... %d active domains",&[half(m,begin) as u32,half(m,end) as u32,half(m,count) as u32],h);
                let mut i=0u16;
                while i<half(m,count) {
                    m.put_half(block,0);m.put_half(domain,0);
                    let st=m.word(state);m.put_half(st+22,0);m.put_half(st+20,half(m,begin));m.put_word(st+24,0);
                    let mut row=m.word(active);
                    rc=normalization::domain_row(m.bytes,p,half(m,s+i as u32*2) as u32,Some(&mut row));
                    m.put_word(active,row);if rc>=0x80000000 {break 'work;}
                    token_log(m,p,5,b"Domain %d : range %d,%d",&[i as u32,half(m,row) as i16 as i32 as u32,half(m,row+2) as i16 as i32 as u32],h);
                    m.put_half(m.word(state)+22,half(m,row+2));
                    let mut b=half(m,block);let mut d=half(m,domain);
                    rc=normalization::domain_entry(m.bytes,p,half(m,s+i as u32*2) as u32,&mut b,&mut d);
                    m.put_half(block,b);m.put_half(domain,d);if rc>=0x80000000 {break 'work;}
                    let mut table=m.word(db);rc=normalization::domain_value(m.bytes,p,b as u32,&mut table);m.put_word(db,table);
                    if rc>=0x80000000 {break 'work;}
                    if half(m,table+12)<d {
                        Host::call(h,m,0x173dc0,&[m.word(m.word(p)+16),m.word(p+0x48),0x5ddc,d as u32,half(m,table+12) as u32]);
                        return normalization::error_code(0);
                    }
                    let dom=m.word(table+8)+d as u32*28;
                    let prop=m.word(table+16)+half(m,dom+8) as u32*8;
                    token_log(m,p,5,b"doing domain %d, Domain[%s:%s] (applying %d rules)",&[i as u32,m.word(table),m.word(dom+20)+half(m,dom) as u32,half(m,prop+4) as u32],h);
                    let mut j=0u16;
                    while j<half(m,prop+4) {
                        let st=m.word(state);m.put_word(st+24,m.word(prop)+j as u32*6);m.put_half(st+16,0);
                        rc=normalization::indexed_pair(m.bytes,p,m.word(st+24),st+12,st+16,st+8,trigger);
                        if rc>=0x80000000 {break 'work;}
                        let tr=m.byte(trigger);let ty=m.byte(kind);
                        let apply=match tr {0=>true,1=>ty==2,2=>ty==1,3=>ty==0,4=>ty==0||ty==1,5=>ty==0||ty==2,6=>ty==1||ty==2,_=>false};
                        if apply {
                            let c=m.word(m.word(state)+4);let out=m.word(c);
                            m.put_byte(out+0x52,0);m.put_half(out+0x28,0);
                            m.put_word(c+8,m.word(p+0x5c));m.put_word(c+12,m.word(m.word(state)+8));
                            m.put_word(out+0x54,m.word(m.word(state)+24));m.put_half(out+0x58,half(m,block));m.put_half(out+0x5a,half(m,domain));
                            m.put_half(c+20,half(m,m.word(state)+16));m.put_half(c+28,half(m,m.word(state)+20));m.put_half(c+30,half(m,m.word(state)+22));
                            m.put_word(c+24,0);m.put_byte(c+22,0);
                            rc=Host::call(h,m,0x2b7118,&[p,m.word(state)]);if rc>=0x80000000 {break 'work;}
                            let c=m.word(m.word(state)+4);let status=m.word(c+4);
                            m.put_half(status,0);m.put_half(status+2,0);m.put_byte(c+22,0);
                        } else {
                            Host::call(h,m,0x1738e8,&[m.word(m.word(p)+16),m.word(p+0x48),5,0x249cec,m.word(m.word(state)+8),ty as u32,tr as u32]);
                        }
                        j=j.wrapping_add(1);
                    }
                    i=i.wrapping_add(1);
                }
                token_log(m,p,5,b"END of loop for basic token %d,%d",&[half(m,begin) as u32,half(m,end) as u32],h);
                rc=Host::call(h,m,0x2b8f08,&[p]);if rc>=0x80000000 {break 'work;}
                if normalization::item_value(m.bytes,p)!=0 {
                    m.put_half(next,0);rc=Host::call(h,m,0x2b4968,&[p,next]);
                    if ((if rc<0x80000000 {u32::MAX}else{0}) & half(m,next) as u32)!=0 {m.put_half(token,half(m,next));}
                    else {
                        crate::speech_part_6::Host::error(h,m,m.word(m.word(p)+16),m.word(p+0x48),0x5ddd);
                        if half(m,next)==0 {rc=normalization::error_code(0);}break 'work;
                    }
                }
                Host::call(h,m,0x24e1a0,&[p,0]);
                if rc>=0x80000000 {break 'work;}
            }
            if Host::call(h,m,m.word(m.word(p+12)+60),&[m.word(p+16),m.word(p+20),half(m,token) as u32,token])!=0 {break;}
        }
        Host::call(h,m,0x24e1a0,&[p,1]);
        token_log(m,p,4,b"tokentn : AllInOneLoop End (%x)",&[rc],h);rc
    })();
    m.bytes[off..off+1152].copy_from_slice(&saved);rc
}
