
use crate::boot::Memory;
use crate::boot_part_b::Host;

fn byte(m: &mut Memory<'_>, p: u32, v: u8) {
    let i = m.offset(p);
    m.bytes[i] = v;
}
fn half(m: &Memory<'_>, p: u32) -> u16 {
    u16::from_le_bytes([m.byte(p), m.byte(p.wrapping_add(1))])
}

pub fn fn_199458(m: &mut Memory<'_>, mut dst: u32, mut src: u32, mut n: u32) -> u32 {
    for size in [32u32, 16, 8, 4, 2, 1] {
        while n >= size {
            let mut block = [0u8; 32];
            for i in 0..size {
                block[i as usize] = m.byte(src.wrapping_add(i));
            }
            for i in 0..size {
                byte(m, dst.wrapping_add(i), block[i as usize]);
            }
            dst = dst.wrapping_add(size);
            src = src.wrapping_add(size);
            n -= size;
        }
    }
    dst
}

pub fn fn_1994cc(r0: u32) -> u32 {
    crate::boot_part_c::return_leaf(r0)
}

pub fn fn_183edc(m: &mut Memory<'_>, p: u32, handle: u32, h: &mut impl Host) {
    loop {
        let v = m.word(p) as i32;
        if v > 0 {
            m.put_word(p, v.wrapping_neg() as u32);
            return;
        }
        if v < 0 {
            m.put_word(p, v.wrapping_sub(1) as u32);
            break;
        }
    }
    loop {
        h.call(m, 0x172d18, &[handle, p, 1, 0, 0, 0]);
        let v = m.word(p) as i32;
        if v > 0 {
            m.put_word(p, 1i32.wrapping_sub(v) as u32);
            return;
        }
    }
}

pub fn fn_184c58(m: &mut Memory<'_>, a: u32, callback: u32, h: &mut impl Host) {
    let cb = m.word(callback);
    if cb != 0 {
        m.put_word(callback, 0);
        h.call(m, cb, &[a, 0, 0, 0]);
    }
    struct Svc<'a, 'b>(&'a mut Memory<'b>, &'a mut dyn Host);
    impl crate::boot_part_c::BootHost for Svc<'_, '_> {
        fn svc(&mut self, number: u8, registers: [u32; 4]) -> [u32; 4] {
            [self.1.call(self.0, number as u32, &registers), 0, 0, 0]
        }
        fn errno_address(&mut self) -> u32 {
            panic!("unused errno")
        }
        fn allocate(&mut self, _: &mut Memory<'_>, _: u32, _: u32) -> u32 {
            panic!("unused allocator")
        }
    }
    crate::boot_part_c::svc_3c(&mut Svc(m, h), [a, 0, 0, 0]);
}

pub fn fn_1858e4(m: &mut Memory<'_>, guard: u32, object: u32) -> u32 {
    if m.word(guard) & 1 == 0 && m.acquire_guard(guard) != 0 {
        for at in [0, 4, 16, 24, 52, 56, 64, 68, 72, 76] {
            m.put_word(object + at, 0);
        }
        m.put_word(object + 60, u32::MAX);
        for at in [8, 12, 14, 48] {
            byte(m, object + at, 0);
        }
        for at in [20, 28] {
            byte(m, object + at, 1);
        }
    }
    object
}

pub fn fn_18ac58(m: &mut Memory<'_>, guard: u32, object: u32, _h: &mut impl Host) -> u32 {
    if m.word(guard) & 1 == 0 && m.acquire_guard(guard) != 0 {
        m.fn_13e4d0(object);
    }
    object
}

pub fn fn_19b1d0(m: &mut Memory<'_>, a: u32, global: u32, h: &mut impl Host) -> u32 {
    let base = if a != 0 { 100 } else { 380 };
    let mut q = 0;
    if m.byte(global) == 0 {
        return 0;
    }
    if m.byte(global + 1) != 0 {
        let tick = h.call(m, 0x28, &[]);
        if tick & 16 != 0 {
            q = base / ((tick & 15) + 1);
        }
    }
    let t = m.word(global + 24).wrapping_add(q);
    let s = t as u16;
    if s == 0 {
        return t;
    }
    let us = s as u64 * 1000000;
    struct Sleep<'a, H>(&'a mut H, u32);
    impl<H: Host> crate::boot::Calls for Sleep<'_, H> {
        fn call(&mut self, m: &mut Memory<'_>, f: u32, a: &[u32]) -> u32 {
            self.1 = match f {
                0x11bcac => {
                    struct Platform<'a, 'b, H>(&'a mut Memory<'b>, &'a mut H);
                    impl<H: Host> crate::boot_part_c::BootHost for Platform<'_, '_, H> {
                        fn svc(&mut self, n: u8, a: [u32; 4]) -> [u32; 4] {
                            [self.1.call(self.0, n as u32, &a), 0, 0, 0]
                        }
                        fn errno_address(&mut self) -> u32 {
                            panic!("unused")
                        }
                        fn allocate(&mut self, _: &mut Memory<'_>, _: u32, _: u32) -> u32 {
                            panic!("unused")
                        }
                    }
                    crate::boot_part_c::svc_0a(&mut Platform(m, self.0), [a[0], a[1], 0, 0])[0]
                }
                0x11bc00 => crate::boot_part_c::delay_result(a[0] as i32) as u32,
                _ => panic!("unexpected sleep dependency"),
            };
            self.1
        }
        fn call_wide(&mut self, _: &mut Memory<'_>, _: u32, _: &[u32]) -> u64 {
            panic!("unused")
        }
    }
    let mut sleep = Sleep(h, 0);
    m.fn_166054(&mut sleep, us as u32, (us >> 32) as u32);
    sleep.1
}

pub fn fn_19b78c(m: &mut Memory<'_>, p: u32, block: u32, h: &mut impl Host) {
    h.call(m, 0x184f84, &[p + 24, block]);
    m.put_word(p + 84, m.word(p + 84).wrapping_sub(1));
}

pub fn fn_19ba10(
    m: &mut Memory<'_>,
    p: u32,
    epoch: u32,
    scratch: u32,
    h: &mut impl crate::boot::Kernel,
    globals: &crate::boot::TickGlobals,
) {
    let saved = core::array::from_fn::<_, 4, _>(|i| m.word(scratch + i as u32 * 4));
    let us = m.fn_1391a4(h, globals, scratch + 16).wrapping_mul(1000000);
    m.put_word(scratch, us as u32);
    m.put_word(scratch + 4, (us >> 32) as u32);
    m.put_word(scratch + 8, m.word(epoch));
    m.put_word(scratch + 12, m.word(epoch + 4));
    m.fn_137b30(scratch + 8, scratch);
    let lo = m.word(scratch + 8);
    let hi = m.word(scratch + 12);
    for (i, v) in saved.into_iter().enumerate() {
        m.put_word(scratch + i as u32 * 4, v);
    }
    m.put_word(p, lo);
    m.put_word(p + 4, hi);
}
pub trait WideHost: Host {
    fn wide(&mut self, m: &mut Memory<'_>, f: u32, args: &[u32]) -> u64;
}

pub fn fn_19de08(m: &Memory<'_>, tree: u32, mut path: u32, locale: u32) -> u32 {
    let entries = m.word(tree + 4);
    let mut index = m.word(tree + 24);
    loop {
        if half(m, path) == 0 {
            return index;
        }
        if half(m, path) == 47 {
            index = 0;
            path += 2;
            continue;
        }
        if half(m, path) == 46 {
            if half(m, path + 2) == 46 {
                if half(m, path + 4) == 47 {
                    path += 6;
                    index = m.word(entries + index * 12 + 4);
                    continue;
                }
                if half(m, path + 4) == 0 {
                    return m.word(entries + index * 12 + 4);
                }
            } else {
                if half(m, path + 2) == 47 {
                    path += 4;
                    continue;
                }
                if half(m, path + 2) == 0 {
                    return index;
                }
            }
        }
        let mut end = path;
        while half(m, end) != 0 && half(m, end) != 47 {
            end += 2;
        }
        let slash = half(m, end) == 47;
        let directory = entries + index * 12;
        index += 1;
        if index >= m.word(directory + 8) {
            return u32::MAX;
        }
        loop {
            let name = m.word(entries + index * 12);
            if name & 0xff000000 == 0 && slash {
                index += 1;
            } else {
                let mut candidate = m.word(tree + 16) + (name & 0xffffff);
                if half(m, candidate) == 46 && half(m, candidate + 2) == 0 {
                    index += 1;
                    continue;
                }
                let mut input = path;
                let mut mismatch = false;
                while half(m, candidate) != 0 {
                    let x = m.locale_lower(locale, half(m, input) as u32);
                    let y = m.locale_lower(locale, half(m, candidate) as u32);
                    input += 2;
                    candidate += 2;
                    if x != y {
                        mismatch = true;
                        break;
                    }
                }
                if !mismatch && (half(m, input) == 47 || half(m, input) == 0) {
                    break;
                }
                if name & 0xff000000 != 0 {
                    index = m.word(entries + index * 12 + 8);
                } else {
                    index += 1;
                }
            }
            if index >= m.word(directory + 8) {
                return u32::MAX;
            }
        }
        if !slash {
            return index;
        }
        path = end + 2;
    }
}

pub fn fn_19dcd8(m: &mut Memory<'_>, tree: u32, path: u32, out: u32, locale: u32) -> u32 {
    let i = fn_19de08(m, tree, path, locale);
    let entries = m.word(tree + 4);
    if (i as i32) >= 0 && m.word(entries + i * 12) & 0xff000000 != 0 {
        m.put_word(out, tree);
        m.put_word(out + 4, i);
        m.put_word(out + 8, i.wrapping_add(1));
        m.put_word(out + 12, m.word(entries + i * 12 + 8));
        1
    } else {
        0
    }
}

pub trait UriHost: Host {
    fn open(
        &mut self,
        m: &mut Memory<'_>,
        method: u32,
        broker: u32,
        kind: u32,
        path: u32,
        length: u32,
        mode: u32,
    ) -> (u32, u32);
}

pub fn fn_19b054(
    m: &mut Memory<'_>,
    out: u32,
    uri: u32,
    mode: u32,
    table: u32,
    h: &mut impl UriHost,
) -> u32 {
    let broker = m.morphology_lookup(uri, table);
    if broker == 0 {
        return 0xc8804465;
    }
    let mut after = uri;
    loop {
        let c = half(m, after);
        after = after.wrapping_add(2);
        if c == 58 {
            break;
        }
    }
    let len = m.fn_1003dc(after).wrapping_add(1).wrapping_mul(2);
    let method = m.word(m.word(broker));
    let (rc, slot) = h.open(m, method, broker, 4, after, len, mode);
    if rc < 0x80000000 {
        m.put_word(out, slot);
    }
    rc
}

pub trait AttributeHost: Host {
    fn create(
        &mut self,
        m: &mut Memory<'_>,
        ctx: u32,
        flag: u32,
        ticket: [u32; 5],
        out: u32,
    ) -> u32;
    fn attribute(&mut self, m: &mut Memory<'_>, object: u32, name: &[u8], value: [u32; 3]) -> u32;
}

pub fn fn_198294(
    m: &mut Memory<'_>,
    ctx: u32,
    b: u32,
    c: u32,
    out: u32,
    table: u32,
    h: &mut impl AttributeHost,
) -> u32 {
    let ticket = core::array::from_fn(|i| m.word(table + 20 + i as u32 * 4));
    let mut rc = h.create(m, ctx, 1, ticket, out);
    if rc < 0x80000000 {
        let o = m.word(out);
        rc = h.attribute(m, o, b"BROKERSTRING", [b, 0, 0]);
    }
    if rc < 0x80000000 {
        let o = m.word(out);
        rc = h.attribute(m, o, b"FULLPATHCOMPONENT", [c, 0, 0]);
    }
    if rc >= 0x80000000 {
        let o = m.word(out);
        h.call(m, 0x19665c, &[o]);
    }
    rc
}

pub fn fn_183524(
    m: &mut Memory<'_>,
    a: [u32; 6],
    pattern: u32,
    semaphore: u32,
    callback: u32,
    h: &mut impl Host,
) -> u32 {
    struct Dispatch<'a, H>(&'a mut H, u32, u32, u32);
    impl<H: Host> Host for Dispatch<'_, H> {
        fn thread_id(&mut self) -> u32 {
            self.0.thread_id()
        }
        fn call(&mut self, m: &mut Memory<'_>, f: u32, a: &[u32]) -> u32 {
            match f {
                0x183edc => {
                    fn_183edc(m, a[0], self.1, self.0);
                    0
                }
                0x184c58 => {
                    fn_184c58(m, a[0], self.2, self.0);
                    0
                }
                0x1840fc => {
                    crate::boot_part_b::fn_1840fc(m, a[0], self);
                    0
                }
                0x18423c => {
                    crate::boot_part_b::fn_18423c(m, a[0], self);
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
                        semaphore: self.3,
                        service: 0,
                        command: 0,
                    };
                    m.fn_165d00(self, &g, a[0]);
                    0
                }
                0x19af64 => crate::boot_part_b::fn_19af64(m, a[0]),
                0x19ae08 => {
                    crate::boot_part_b::fn_19ae08(m, a[0], a[1]);
                    0
                }
                0x163f20 => m.fn_163f20(self, a.try_into().unwrap()),
                0x132c90 => {
                    m.fn_132c90(self, a[0], a[1]);
                    0
                }
                0x19b78c => {
                    fn_19b78c(m, a[0], a[1], self);
                    0
                }
                0x1007d0 => m.wide_substring(a[0], a[1]),
                _ => self.0.call(m, f, a),
            }
        }
    }
    impl<H: Host> crate::boot::Calls for Dispatch<'_, H> {
        fn call(&mut self, m: &mut Memory<'_>, f: u32, a: &[u32]) -> u32 {
            <Self as Host>::call(self, m, f, a)
        }
        fn call_wide(&mut self, _: &mut Memory<'_>, _: u32, _: &[u32]) -> u64 {
            panic!("unused")
        }
    }
    let wait_handle = m.word(semaphore);
    let mut d = Dispatch(h, wait_handle, callback, semaphore);
    let i = crate::boot_part_b::fn_19acf4(m, a[0], &mut d);
    if i == u32::MAX {
        crate::boot_part_b::fn_187498(m, &mut d);
    }
    let slot = m.word(a[0] + 20).wrapping_add(i.wrapping_mul(0xe8));
    crate::boot_part_b::fn_19ad68(m, slot, a[1], a[2], a[3], a[4], a[5], pattern, &mut d);
    m.fn_163de0(&mut d, a[0], slot);
    i
}

#[allow(clippy::too_many_arguments)]
pub fn fn_183524_resource<H>(
    m: &mut Memory<'_>,
    a: [u32; 6],
    pattern: u32,
    semaphore: u32,
    callback: u32,
    resource_scratch: u32,
    globals: &ResourceGlobals,
    h: &mut H,
) -> u32
where
    H: UriHost + crate::boot_part_b::PairHost + crate::boot_part_b::ReadHost,
{
    struct Real<'a, H> {
        host: &'a mut H,
        semaphore: u32,
        callback: u32,
        scratch: u32,
        globals: &'a ResourceGlobals,
    }
    impl<H: UriHost + crate::boot_part_b::PairHost + crate::boot_part_b::ReadHost> Host
        for Real<'_, H>
    {
        fn thread_id(&mut self) -> u32 {
            self.host.thread_id()
        }
        fn call(&mut self, m: &mut Memory<'_>, f: u32, a: &[u32]) -> u32 {
            match f {
                0x132ab8 => {
                    let g = ResourceGlobals {
                        context: self.globals.context,
                        broker_table: self.globals.broker_table,
                        length_assert_pc: self.globals.length_assert_pc,
                        read_assert_pc: self.globals.read_assert_pc,
                    };
                    fn_132ab8(m, a[0], a[1], self.scratch, &g, self);
                    0
                }
                0x163f20 => m.fn_163f20(self, a.try_into().unwrap()),
                0x184f20 => m.boot_heap_allocate(a.try_into().unwrap()),
                0x184f84 => {
                    guest_184f84(m, a[0], a[1]);
                    0
                }
                0x1840fc => {
                    crate::boot_part_b::fn_1840fc(m, a[0], self);
                    0
                }
                0x18423c => {
                    crate::boot_part_b::fn_18423c(m, a[0], self);
                    0
                }
                0x183edc => {
                    fn_183edc(m, a[0], m.word(self.semaphore), self);
                    0
                }
                0x184c58 => {
                    fn_184c58(m, a[0], self.callback, self);
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
                        semaphore: self.semaphore,
                        service: 0,
                        command: 0,
                    };
                    m.fn_165d00(self, &g, a[0]);
                    0
                }
                _ => self.host.call(m, f, a),
            }
        }
    }
    impl<H: UriHost + crate::boot_part_b::PairHost + crate::boot_part_b::ReadHost>
        crate::boot::Calls for Real<'_, H>
    {
        fn call(&mut self, m: &mut Memory<'_>, f: u32, a: &[u32]) -> u32 {
            Host::call(self, m, f, a)
        }
        fn call_wide(&mut self, _: &mut Memory<'_>, _: u32, _: &[u32]) -> u64 {
            unreachable!("resource allocator has no wide calls")
        }
    }
    impl<H: UriHost + crate::boot_part_b::PairHost + crate::boot_part_b::ReadHost> UriHost
        for Real<'_, H>
    {
        fn open(
            &mut self,
            m: &mut Memory<'_>,
            method: u32,
            broker: u32,
            kind: u32,
            path: u32,
            len: u32,
            mode: u32,
        ) -> (u32, u32) {
            self.host.open(m, method, broker, kind, path, len, mode)
        }
    }
    impl<H: UriHost + crate::boot_part_b::PairHost + crate::boot_part_b::ReadHost>
        crate::boot_part_b::PairHost for Real<'_, H>
    {
        fn pair(&mut self, m: &mut Memory<'_>, handle: u32) -> (u32, u32, u32) {
            self.host.pair(m, handle)
        }
    }
    impl<H: UriHost + crate::boot_part_b::PairHost + crate::boot_part_b::ReadHost>
        crate::boot_part_b::ReadHost for Real<'_, H>
    {
        fn read(
            &mut self,
            m: &mut Memory<'_>,
            handle: u32,
            lo: u32,
            hi: u32,
            dst: u32,
            len: u32,
        ) -> (u32, u32) {
            self.host.read(m, handle, lo, hi, dst, len)
        }
    }
    assert_eq!(
        m.base, 0,
        "composed block heap requires absolute-index memory"
    );
    fn_183524(
        m,
        a,
        pattern,
        semaphore,
        callback,
        &mut Real {
            host: h,
            semaphore,
            callback,
            scratch: resource_scratch,
            globals,
        },
    )
}

pub fn fn_189040(
    m: &mut Memory<'_>,
    tree: u32,
    name: u32,
    dot: u32,
    parent: u32,
    locale: u32,
    scratch: u32,
) -> u32 {
    let mut saved = [0u8; 32];
    let at = m.offset(scratch);
    saved.copy_from_slice(&m.bytes[at..at + 32]);
    let mut result = u32::MAX;
    fn_19dcd8(m, tree, dot, scratch + 16, locale);
    while crate::boot_part_b::fn_19dd30(m, scratch + 16, scratch) != 0 {
        let entry_name = m.word(scratch + 8);
        if m.byte(scratch + 12) != 0 {
            change_directory(m, tree, entry_name, locale);
            result = fn_189040(m, tree, name, dot, parent, locale, scratch + 32);
            change_directory(m, tree, parent, locale);
            if result != u32::MAX {
                break;
            }
        } else if m.wide_case_compare(locale, name, entry_name) == 0 {
            result = m.word(scratch + 4);
            break;
        }
    }
    m.bytes[at..at + 32].copy_from_slice(&saved);
    result
}
fn change_directory(m: &mut Memory<'_>, tree: u32, path: u32, locale: u32) -> u32 {
    struct Resolve(u32, u32);
    impl Host for Resolve {
        fn thread_id(&mut self) -> u32 {
            panic!("resolver does not read TLS")
        }
        fn call(&mut self, m: &mut Memory<'_>, f: u32, a: &[u32]) -> u32 {
            assert_eq!(f, 0x19de08);
            fn_19de08(m, a[0], self.0, self.1)
        }
    }
    crate::boot_part_b::fn_19ddd0(m, tree, &mut Resolve(path, locale))
}

#[derive(Clone, Copy)]
pub enum ObjectRegion {
    UsEu,
    Kr,
}

pub fn fn_18cae8(m: &mut Memory<'_>, p: u32, region: ObjectRegion, h: &mut impl Host) {
    let kr = matches!(region, ObjectRegion::Kr);
    let shift = if kr { 4 } else { 0 };
    struct Reset<'a, H>(&'a mut H, bool);
    impl<H: Host> crate::boot_part_c::BootCHost for Reset<'_, H> {
        fn invoke(&mut self, m: &mut Memory<'_>, f: u32, a: &[u32]) -> [u32; 3] {
            if f == 0x1b033c {
                m.reset_cursor_in_region(
                    a[0],
                    if self.1 {
                        crate::boot_part_c::CursorRegion::Kr
                    } else {
                        crate::boot_part_c::CursorRegion::UsEu
                    },
                );
                [0; 3]
            } else {
                [self.0.call(m, f, a), 0, 0]
            }
        }
    }
    m.reset_playback(&mut Reset(h, kr), p);
    for at in [0x4bc8 - shift, 0x4bcc - shift, 0x381c, 0x3820, 0x3824] {
        m.put_word(p + at, 0);
    }
    byte(m, p + 0x4c30 - shift, 0);
    h.call(m, 0x1c1768, &[p, 0, 0]);
    h.call(m, 0x1a16a4, &[p + 0x3828]);
    m.put_word(p + 0x4bb4, u32::MAX);
    m.put_word(p + 0x4bb8, u32::MAX);
    if !kr {
        byte(m, p + 0x4bbc, 0);
    }
    let at = m.offset(p + 0x4c38 - shift);
    m.bytes[at..at + 2].copy_from_slice(&0x23u16.to_le_bytes());
    m.put_word(p + 0x4c6c - shift, 0);
    h.call(m, 0x1b89f8, &[p + 0x31a4]);
    h.call(m, 0x1b8890, &[p + 0x31f4]);
}

const PARSE_ERROR: u32 = 0x82206004;
fn xml_space(c: u8) -> bool {
    matches!(c, b' ' | b'\t' | b'\n' | b'\r')
}
fn xml_name(c: u8) -> bool {
    c.is_ascii_alphanumeric() || matches!(c, b'.' | b'-' | b'_' | b':')
}
fn xml_equal(m: &Memory<'_>, p: u32, text: &[u8]) -> bool {
    text.iter()
        .enumerate()
        .all(|(i, &b)| m.byte(p + i as u32) == b)
}
fn xml_string(m: &Memory<'_>, p: u32, text: &[u8]) -> bool {
    xml_equal(m, p, text) && m.byte(p + text.len() as u32) == 0
}
fn xml_skip(m: &mut Memory<'_>, ctx: u32) -> u32 {
    let mut p = m.word(ctx + 8);
    while xml_space(m.byte(p)) {
        p += 1;
        m.put_word(ctx + 8, p);
    }
    p
}
fn xml_create(
    m: &mut Memory<'_>,
    ctx: u32,
    flag: u32,
    table: u32,
    out: u32,
    h: &mut impl Host,
) -> u32 {
    let t = table + if flag == 0 { 0 } else { 20 };
    let args = [
        m.word(ctx),
        flag,
        m.word(t),
        m.word(t + 4),
        m.word(t + 8),
        m.word(t + 12),
        m.word(t + 16),
        out,
    ];
    h.call(m, 0x1964e0, &args)
}

pub fn fn_197254(m: &mut Memory<'_>, ctx: u32, out: u32, table: u32, h: &mut impl Host) -> u32 {
    loop {
        let mut p = xml_skip(m, ctx);
        let c = m.byte(p);
        if matches!(c, b'/' | b'>' | b'?') {
            return 0;
        }
        if !(c.is_ascii_alphabetic() || matches!(c, b'_' | b':')) {
            return PARSE_ERROR;
        }
        let name = p;
        p += 1;
        m.put_word(ctx + 8, p);
        while xml_name(m.byte(p)) {
            p += 1;
            m.put_word(ctx + 8, p);
        }
        if xml_space(m.byte(p)) {
            byte(m, p, 0);
            m.put_word(ctx + 8, p + 1);
            p = xml_skip(m, ctx);
        }
        if m.byte(p) != b'=' {
            return PARSE_ERROR;
        }
        byte(m, p, 0);
        m.put_word(ctx + 8, p + 1);
        p = xml_skip(m, ctx);
        let quote = m.byte(p);
        if !matches!(quote, b'"' | b'\'') {
            return PARSE_ERROR;
        }
        p += 1;
        let value = p;
        m.put_word(ctx + 8, p);
        while m.byte(p) != b'<' && m.byte(p) != quote {
            p += 1;
            m.put_word(ctx + 8, p);
        }
        if m.byte(p) != quote {
            return PARSE_ERROR;
        }
        byte(m, p, 0);
        m.put_word(ctx + 8, p + 1);
        let rc = crate::boot_part_b::fn_198414(m, value);
        if rc != 0 {
            return rc;
        }
        if out != 0 {
            if m.word(out + 4) == 0 {
                let rc = xml_create(m, ctx, 0, table, out + 4, h);
                if rc != 0 {
                    return rc;
                }
            }
            let rc = h.call(m, 0x19668c, &[m.word(out + 4), name, value]);
            if rc != 0 {
                return rc;
            }
        }
    }
}

pub fn fn_197630(
    m: &mut Memory<'_>,
    ctx: u32,
    out: u32,
    table: u32,
    scratch: u32,
    h: &mut impl Host,
) -> u32 {
    let saved = [m.word(scratch), m.word(scratch + 4), m.word(scratch + 8)];
    for at in [0, 4, 8] {
        m.put_word(scratch + at, 0);
    }
    let rc = xml_element(m, ctx, out, table, scratch, h);
    for (i, v) in saved.into_iter().enumerate() {
        m.put_word(scratch + i as u32 * 4, v);
    }
    rc
}
fn xml_element(
    m: &mut Memory<'_>,
    ctx: u32,
    out: u32,
    table: u32,
    tmp: u32,
    h: &mut impl Host,
) -> u32 {
    let mode = m.byte(ctx + 12);
    let mut end = 0;
    let mut selfclose = false;
    let mut p = xml_skip(m, ctx);
    if m.byte(p + 1) == b'!' {
        crate::boot_part_b::fn_198358(m, ctx);
        p = m.word(ctx + 8);
    }
    let mut tag = p;
    let mut rc = (|| {
        if m.byte(p) != b'<' {
            return PARSE_ERROR;
        }
        p += 1;
        tag = p;
        m.put_word(ctx + 8, p);
        while xml_name(m.byte(p)) {
            p += 1;
            m.put_word(ctx + 8, p);
        }
        let c = m.byte(p);
        if xml_space(c) {
            byte(m, p, 0);
            m.put_word(ctx + 8, p + 1);
            let rc = fn_197254(m, ctx, tmp, table, h);
            if rc != 0 {
                return rc;
            }
            p = m.word(ctx + 8);
            if m.byte(p) == b'>' {
                m.put_word(ctx + 8, p + 1);
            } else if m.byte(p) == b'/' && m.byte(p + 1) == b'>' {
                m.put_word(ctx + 8, p + 2);
                selfclose = true;
            } else {
                return PARSE_ERROR;
            }
        } else if c == b'>' {
            byte(m, p, 0);
            m.put_word(ctx + 8, p + 1);
        } else if c == b'/' && m.byte(p + 1) == b'>' {
            byte(m, p, 0);
            m.put_word(ctx + 8, p + 2);
            selfclose = true;
        } else {
            return PARSE_ERROR;
        }
        match mode {
            0 => {
                if !xml_string(m, tag, b"SCANSOFT") && !xml_string(m, tag, b"NUANCE") {
                    return PARSE_ERROR;
                }
                byte(m, ctx + 12, 1);
            }
            1 => {
                if !xml_string(m, tag, b"VERSION") {
                    return PARSE_ERROR;
                }
            }
            2 => {
                if !xml_string(m, tag, b"HEADER") {
                    return PARSE_ERROR;
                }
                byte(m, ctx + 12, 3);
            }
            _ => {}
        }
        if selfclose {
            return 0;
        }
        p = xml_skip(m, ctx);
        if m.byte(p + 1) == b'!' {
            crate::boot_part_b::fn_198358(m, ctx);
        }
        m.put_word(tmp, m.word(ctx + 8));
        let mut rc = 0;
        while rc == 0 {
            p = m.word(ctx + 8);
            if m.byte(p) == b'<' && m.byte(p + 1) == b'/' {
                break;
            }
            match m.byte(p) {
                0 => rc = PARSE_ERROR,
                b'<' => {
                    if m.word(tmp) == 0 || p == m.word(tmp) {
                        m.put_word(tmp, 0);
                        rc = fn_197630(m, ctx, tmp, table, tmp + 12, h);
                    } else {
                        rc = PARSE_ERROR;
                    }
                }
                _ => {
                    if m.word(tmp + 8) == 0 {
                        end = p;
                        m.put_word(ctx + 8, p + 1);
                    } else {
                        rc = PARSE_ERROR;
                    }
                }
            }
            p = xml_skip(m, ctx);
            if m.byte(p + 1) == b'!' {
                loop {
                    p = m.word(ctx + 8);
                    if !xml_equal(m, p, b"<!--") {
                        break;
                    }
                    let mut q = p;
                    while m.byte(q) != 0 && !xml_equal(m, q, b"-->") {
                        q += 1;
                    }
                    if m.byte(q) == 0 || m.byte(q - 1) == b'-' {
                        break;
                    }
                    m.put_word(ctx + 8, q + 3);
                    xml_skip(m, ctx);
                }
            }
        }
        if rc != 0 {
            return rc;
        }
        if m.word(tmp) != 0 {
            byte(m, if end != 0 { end + 1 } else { m.word(ctx + 8) }, 0);
        }
        let mut n = 0;
        while m.byte(tag + n) != 0 {
            n += 1;
        }
        p = m.word(ctx + 8) + 2;
        m.put_word(ctx + 8, p);
        for i in 0..n {
            if m.byte(p + i) != m.byte(tag + i) {
                return PARSE_ERROR;
            }
        }
        m.put_word(ctx + 8, p + n);
        p = xml_skip(m, ctx);
        if m.byte(p) != b'>' {
            return PARSE_ERROR;
        }
        m.put_word(ctx + 8, p + 1);
        if m.word(tmp) != 0 {
            crate::boot_part_b::fn_198414(m, m.word(tmp))
        } else {
            0
        }
    })();
    byte(m, ctx + 12, mode);
    if rc == 0 {
        match mode {
            0 | 2 => {
                if m.word(tmp + 4) == 0 && m.word(tmp + 8) != 0 {
                    m.put_word(out + 8, m.word(tmp + 8));
                    m.put_word(tmp + 8, 0);
                } else {
                    rc = PARSE_ERROR;
                }
            }
            1 => {
                let text = m.word(tmp);
                if m.word(tmp + 4) == 0
                    && m.word(tmp + 8) == 0
                    && text != 0
                    && (xml_string(m, text, b"SSFT_1.0") || xml_string(m, text, b"NUAN_1.0"))
                {
                    byte(m, ctx + 12, 2);
                } else {
                    rc = PARSE_ERROR;
                }
            }
            _ => {
                if m.word(out + 8) == 0 {
                    rc = xml_create(m, ctx, 1, table, out + 8, h);
                }
                if rc == 0 {
                    rc = h.call(m, 0x19668c, &[m.word(out + 8), tag, tmp]);
                    if rc == 0 {
                        for at in [0, 4, 8] {
                            m.put_word(tmp + at, 0);
                        }
                    }
                }
            }
        }
    }
    for at in [4, 8] {
        let obj = m.word(tmp + at);
        if obj != 0 {
            h.call(m, 0x19665c, &[obj]);
            m.put_word(tmp + at, 0);
        }
    }
    rc
}

fn xml_len(m: &Memory<'_>, p: u32) -> u32 {
    let mut n = 0;
    while m.byte(p + n) != 0 {
        n += 1;
    }
    n
}
fn xml_literal(m: &mut Memory<'_>, p: u32, s: &[u8]) {
    for (i, &b) in s.iter().enumerate() {
        byte(m, p + i as u32, b);
    }
    byte(m, p + s.len() as u32, 0);
}

pub fn fn_197c78(
    m: &mut Memory<'_>,
    a: [u32; 6],
    table: u32,
    scratch: u32,
    h: &mut impl Host,
) -> u32 {
    let saved = core::array::from_fn::<_, 16, _>(|i| m.word(scratch + i as u32 * 4));
    let rc = xml_header(m, a, table, scratch, h);
    for (i, v) in saved.into_iter().enumerate() {
        m.put_word(scratch + i as u32 * 4, v);
    }
    rc
}
fn xml_header(m: &mut Memory<'_>, a: [u32; 6], table: u32, s: u32, h: &mut impl Host) -> u32 {
    let [h0, h1, file, text, res, readers] = a;
    let has_file = file != 0 && m.byte(file) != 0;
    if !has_file && (text == 0 || m.byte(text) == 0) {
        return 0x82206007;
    }
    if res == 0 || readers == 0 {
        return 0x82206007;
    }
    m.put_word(s + 44, 0);
    let rc = h.call(m, 0x19611c, &[h0, h1, s + 44]);
    if rc >= 0x80000000 {
        return rc;
    }
    let handle = m.word(s + 44);
    let heap = m.word(handle + 4);
    m.put_word(res, 0);
    for at in [0, 4, 8, 12] {
        m.put_word(s + at, 0);
    }
    m.put_word(readers, 0);
    let oo = s + 16;
    let ab = s + 28;
    let mut buf = 0;
    let mut rc = 0;
    m.put_word(s, heap);
    if has_file {
        xml_literal(m, s + 52, b"rb");
        rc = h.call(
            m,
            0x199034,
            &[m.word(handle + 28), heap, file, s + 52, s + 40],
        );
        if rc >= 0x80000000 {
            rc = 0x82206003;
        } else {
            let stm = m.word(s + 40);
            let n = h.call(m, 0x1991d0, &[stm]);
            if n == 0 {
                rc = PARSE_ERROR;
            } else {
                buf = h.call(m, 0x196394, &[heap, n.wrapping_add(1)]);
                if buf == 0 {
                    rc = 0x8220600a;
                } else {
                    let read = h.call(m, 0x199134, &[buf, 1, n, stm]);
                    if read == 0 {
                        rc = PARSE_ERROR;
                    } else {
                        byte(m, buf + read, 0);
                    }
                }
            }
            h.call(m, 0x1990ec, &[stm]);
        }
    } else {
        let n = xml_len(m, text);
        if n == 0 {
            rc = PARSE_ERROR;
        } else {
            buf = h.call(m, 0x196394, &[heap, n.wrapping_add(1)]);
            if buf == 0 {
                rc = 0x8220600a;
            } else {
                for i in 0..=n {
                    byte(m, buf + i, m.byte(text + i));
                }
            }
        }
    }
    m.put_word(s + 4, buf);
    m.put_word(s + 8, buf);

    if rc >= 0x80000000 && buf != 0 {
        h.call(m, 0x196428, &[heap, buf]);
        buf = 0;
        m.put_word(s + 4, 0);
        m.put_word(s + 8, 0);
    }
    let mut idx = 0u32;
    while rc < 0x80000000 && (idx == 0 || m.byte(m.word(s + 8)) != 0) {
        for at in [0, 4, 8] {
            m.put_word(oo + at, 0);
        }
        rc = 0;
        let p = m.word(s + 8);
        if xml_equal(m, p, b"<?xml") && xml_space(m.byte(p + 5)) {
            m.put_word(s + 8, p + 6);
            let p = xml_skip(m, s);
            if xml_equal(m, p, b"version=\"1.0\"?>") || xml_equal(m, p, b"version='1.0'?>") {
                m.put_word(s + 8, p + 15);
            } else {
                for at in [0, 4, 8] {
                    m.put_word(ab + at, 0);
                }
                m.put_word(s + 48, 0);
                rc = fn_197254(m, s, ab, table, h);
                if rc == 0 {
                    rc = PARSE_ERROR;
                    if xml_equal(m, m.word(s + 8), b"?>") && m.word(ab + 4) != 0 {
                        xml_literal(m, s + 56, b"version");
                        let found = h.call(m, 0x1966ac, &[m.word(ab + 4), s + 56, s + 48]);
                        if found < 0x80000000 && xml_string(m, m.word(s + 48), b"1.0") {
                            rc = 0;
                            m.put_word(s + 8, m.word(s + 8) + 2);
                        }
                    }
                } else {
                    m.put_word(s + 8, m.word(s + 8) + 2);
                }
                let attr = m.word(ab + 4);
                if attr != 0 {
                    h.call(m, 0x19665c, &[attr]);
                    m.put_word(ab + 4, 0);
                }
            }
        }
        if rc < 0x80000000 {
            byte(m, s + 12, 0);
            rc = fn_197630(m, s, oo, table, s + 64, h);
            if rc == 0 {
                let obj = m.word(oo + 8);
                if obj == 0
                    || h.call(m, 0x196764, &[obj]) == 0
                    || m.word(oo + 4) != 0
                    || m.word(oo) != 0
                {
                    rc = PARSE_ERROR;
                } else {
                    xml_skip(m, s);
                }
            }
            let attr = m.word(oo + 4);
            if attr != 0 {
                h.call(m, 0x19665c, &[attr]);
                m.put_word(oo + 4, 0);
            }
            if rc == 0 {
                if has_file {
                    for at in [0, 4, 8] {
                        m.put_word(ab + at, 0);
                    }
                    m.put_word(ab, file);
                    xml_literal(m, s + 56, b"HDRFILE");
                    rc = h.call(m, 0x19668c, &[m.word(oo + 8), s + 56, ab]);
                }
                if rc == 0 {
                    if idx == 0 {
                        m.put_word(res, m.word(oo + 8));
                    } else if idx == 1 {
                        let args = [
                            heap,
                            m.word(table + 40),
                            m.word(table + 44),
                            m.word(table + 48),
                            8,
                            readers,
                        ];
                        if h.call(m, 0x196a64, &args) == 0 {
                            rc = 0x8220600a;
                        } else if h.call(m, 0x196db0, &[m.word(readers), res]) == 0 {
                            rc = 0x8220600a;
                        } else {
                            m.put_word(res, 0);
                            if h.call(m, 0x196db0, &[m.word(readers), oo + 8]) == 0 {
                                rc = 0x8220600a;
                            }
                        }
                    } else if h.call(m, 0x196db0, &[m.word(readers), oo + 8]) == 0 {
                        rc = 0x8220600a;
                    }
                    if rc == 0 {
                        idx += 1;
                    }
                }
            }
        }
        if rc != 0 {
            let obj = m.word(oo + 8);
            if obj != 0 {
                h.call(m, 0x19665c, &[obj]);
                m.put_word(oo + 8, 0);
            }
        }
    }
    if buf != 0 {
        h.call(m, 0x196428, &[heap, buf]);
        m.put_word(s + 4, 0);
        m.put_word(s + 8, 0);
    }
    if rc >= 0x80000000 {
        let arr = m.word(readers);
        if arr != 0 {
            m.put_word(s + 48, 0);
            let mut i = 0;
            while h.call(m, 0x1970ac, &[arr, i, s + 48]) != 0 {
                let ptr = m.word(s + 48);
                h.call(m, 0x19665c, &[m.word(ptr)]);
                i += 1;
            }
            h.call(m, 0x196c7c, &[arr]);
            m.put_word(readers, 0);
        }
        let obj = m.word(res);
        if obj != 0 {
            h.call(m, 0x19665c, &[obj]);
            m.put_word(res, 0);
        }
    }
    rc
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct EventVfp {
    pub s16: u32,
    pub s17: u32,
}

impl EventVfp {

    pub fn load_main_loop_s16(&mut self, m: &Memory<'_>, literal: u32) {
        self.s16 = m.word(literal);
    }
}

pub fn fn_183258_regional(
    m: &mut Memory<'_>,
    p: u32,
    kind: u32,
    region: ObjectRegion,
    vfp: &EventVfp,
    h: &mut impl EventHost,
) {
    let p = p.wrapping_add(if matches!(region, ObjectRegion::Kr) {
        16
    } else {
        0
    });
    fn_183258(m, p, kind, vfp, h);
}

pub fn fn_183258_live(
    m: &mut Memory<'_>,
    p: u32,
    kind: u32,
    s16: u32,
    s17: u32,
    tick: u64,
) -> u32 {
    let v = p.wrapping_add(0x61cc);
    let begin = m.word(v + 4);
    let end = m.word(v + 8);
    let capacity = m.word(v + 12);
    let count = end.wrapping_sub(begin) as i32 >> 5;
    let reserved = capacity & 0x3fffffff;
    if (reserved as i32) <= count {
        let grown = if reserved == 0 {
            1
        } else {
            reserved.wrapping_mul(2)
        };
        if (reserved as i32) < (grown as i32) {

            return 0;
        }
    }
    if end != 0 {
        for (i, value) in [
            kind,
            s16,
            tick as u32,
            (tick >> 32) as u32,
            0,
            0,
            0,
            s17,
        ]
        .into_iter()
        .enumerate()
        {
            m.put_word(end + i as u32 * 4, value);
        }
    }
    m.put_word(v + 8, end.wrapping_add(32));
    0
}

pub fn fn_183258(m: &mut Memory<'_>, p: u32, kind: u32, vfp: &EventVfp, h: &mut impl EventHost) {
    let tick = h.tick(m);
    let v = p.wrapping_add(0x61cc);
    let capacity = m.word(v + 12) & 0x3fffffff;
    let count = (m.word(v + 8).wrapping_sub(m.word(v + 4)) as i32) >> 5;
    if (capacity as i32) <= count {
        let grown = if capacity == 0 {
            1
        } else {
            capacity.wrapping_mul(2)
        };
        if (capacity as i32) < grown as i32 {
            let allocator = m.word(v);
            if allocator == 0 || m.word(v + 12) & 0xc0000000 != 0x40000000 {
                return;
            }
            let vt = m.word(allocator);
            let buffer = h.call(m, m.word(vt + 8), &[allocator, grown.wrapping_mul(32), 32]);
            let mut src = m.word(v + 4);
            let end = m.word(v + 8);
            let mut dst = buffer;
            let mut n = 0;
            if src != end {
                while src != end {
                    if dst != 0 {
                        let mut event = [0u32; 8];
                        for i in 0..8 {
                            event[i] = m.word(src + i as u32 * 4);
                        }
                        for i in 0..8 {
                            m.put_word(dst + i as u32 * 4, event[i]);
                        }
                    }
                    src = src.wrapping_add(32);
                    dst = dst.wrapping_add(32);
                }
                src = m.word(v + 4);
                n = (m.word(v + 8).wrapping_sub(src) as i32) >> 5;
            }
            if src != 0 {
                let allocator = m.word(v);
                let vt = m.word(allocator);
                h.call(m, m.word(vt + 12), &[allocator, src]);
            }
            m.put_word(v + 8, buffer.wrapping_add((n as u32).wrapping_mul(32)));
            m.put_word(v + 4, buffer);
            m.put_word(v + 12, (m.word(v + 12) & 0xc0000000) | grown);
        }
    }
    let entry = m.word(v + 8);
    if entry != 0 {
        for (i, value) in [
            kind,
            vfp.s16,
            tick as u32,
            (tick >> 32) as u32,
            0,
            0,
            0,
            vfp.s17,
        ]
        .into_iter()
        .enumerate()
        {
            m.put_word(entry + i as u32 * 4, value);
        }
    }
    m.put_word(v + 8, m.word(v + 8).wrapping_add(32));
}

pub trait EventHost: Host {
    fn tick(&mut self, memory: &mut Memory<'_>) -> u64;
}

pub(crate) fn guest_184f84(m: &mut Memory<'_>, sub: u32, pointer: u32) {
    m.boot_heap_release(sub, pointer);
}

pub fn fn_19b78c_guest(m: &mut Memory<'_>, object: u32, pointer: u32) {
    struct GuestHeap;
    impl Host for GuestHeap {
        fn thread_id(&mut self) -> u32 {
            unreachable!()
        }
        fn call(&mut self, m: &mut Memory<'_>, f: u32, a: &[u32]) -> u32 {
            assert_eq!(f, 0x184f84);
            guest_184f84(m, a[0], a[1]);
            0
        }
    }
    fn_19b78c(m, object, pointer, &mut GuestHeap);
}

fn nonnull(p: u32) -> Option<u32> {
    if p == 0 {
        None
    } else {
        Some(p)
    }
}

pub(crate) fn guest_1964e0(
    m: &mut Memory<'_>,
    a: [u32; 8],
    methods: &mut impl crate::frontend::MethodTable,
) -> u32 {
    assert_eq!(m.base, 0, "frontend ports require absolute guest memory");
    crate::frontend::create_ctx_object(
        m.bytes,
        nonnull(a[0]),
        a[1],
        a[2],
        a[3],
        a[4],
        a[5],
        a[6],
        nonnull(a[7]),
        methods,
    )
}

pub(crate) fn guest_19668c(
    m: &mut Memory<'_>,
    object: u32,
    key: u32,
    value: u32,
    heap: &mut impl crate::frontend::HeapVtable,
    tree: &mut impl crate::frontend::TreeOps,
) -> u32 {
    assert_eq!(m.base, 0, "frontend ports require absolute guest memory");
    crate::frontend::insert_guarded(m.bytes, nonnull(object), key, value, heap, tree)
}

pub fn guest_19668c_m(
    m: &mut Memory<'_>,
    object: u32,
    key: u32,
    value: u32,
    heap: &mut impl crate::frontend::HeapVtableM,
    tree: &mut impl crate::frontend::TreeOpsM,
) -> u32 {
    assert_eq!(m.base, 0, "frontend ports require absolute guest memory");
    crate::frontend::insert_guarded_m(m.bytes, nonnull(object), key, value, heap, tree)
}

pub struct FrontendTreeCalls<'a, H> {
    pub host: &'a mut H,
    pub vtable: u32,
    pub scratch: u32,
}
impl<H: Host> crate::frontend::TreeOpsM for FrontendTreeCalls<'_, H> {
    fn tree_cmp_m(&mut self, key: u32, other: u32, mem: &mut [u8]) -> i32 {
        let mut m = Memory { base: 0, bytes: mem };
        let target = m.word(self.vtable + 8);
        self.host.call(&mut m, target, &[key, other]) as i32
    }
    fn tree_init_m(&mut self, key: u32, heap: u32, node: u32, mem: &mut [u8]) -> (u32, Option<u32>) {
        let mut m = Memory { base: 0, bytes: mem };
        let target = m.word(self.vtable);
        let rc = self.host.call(&mut m, target, &[key, heap, node + 16]);

        (rc, None)
    }
    fn tree_alloc_m(&mut self, arg: u32, heap: u32, mem: &mut [u8]) -> (u32, u32) {
        let mut m = Memory { base: 0, bytes: mem };
        let saved = m.word(self.scratch);
        m.put_word(self.scratch, 0);
        let target = m.word(self.vtable + 12);
        let rc = self.host.call(&mut m, target, &[arg, heap, self.scratch]);
        let out = m.word(self.scratch);
        m.put_word(self.scratch, saved);
        (rc, out)
    }
    fn tree_free_node_m(&mut self, p: u32, heap: u32, mem: &mut [u8]) {
        let mut m = Memory { base: 0, bytes: mem };
        let target = m.word(self.vtable + 4);
        self.host.call(&mut m, target, &[p, heap]);
    }
    fn tree_free_handle_m(&mut self, p: u32, heap: u32, mem: &mut [u8]) {
        let mut m = Memory { base: 0, bytes: mem };
        let target = m.word(self.vtable + 16);
        self.host.call(&mut m, target, &[p, heap]);
    }
}

pub(crate) fn guest_19665c(
    m: &mut Memory<'_>,
    object: u32,
    heap: &mut impl crate::frontend::HeapVtable,
    destroy: &mut impl FnMut(u32, u32),
) -> u32 {
    assert_eq!(m.base, 0, "frontend ports require absolute guest memory");
    struct Release(Option<(u32, u32)>);
    impl crate::frontend::HeapVtable for Release {
        fn alloc_at(&mut self, _: u32, _: u32) -> u32 {
            unreachable!()
        }
        fn realloc_at(&mut self, _: u32, _: u32, _: u32) -> u32 {
            unreachable!()
        }
        fn release_at(&mut self, heap: u32, object: u32) {
            self.0 = Some((heap, object));
        }
    }
    let mut release = Release(None);
    let mut tree = None;
    let rc = crate::frontend::release_cleared(
        m.bytes,
        nonnull(object),
        &mut |p| tree = Some(p),
        &mut release,
    );
    if let Some(tree) = tree {
        crate::frontend::clear_tree(m.bytes, tree, heap, destroy);
    }
    if let Some((context, pointer)) = release.0 {
        heap.release_at(context, pointer);
    }
    rc
}

pub struct ResourceGlobals {
    pub context: u32,
    pub broker_table: u32,
    pub length_assert_pc: u32,
    pub read_assert_pc: u32,
}

pub fn fn_132ab8<H>(
    m: &mut Memory<'_>,
    manager: u32,
    slot: u32,
    scratch: u32,
    globals: &ResourceGlobals,
    h: &mut H,
) where
    H: UriHost + crate::boot_part_b::PairHost + crate::boot_part_b::ReadHost,
{
    let start = m.offset(scratch);
    let mut saved = [0u8; 32];
    saved.copy_from_slice(&m.bytes[start..start + 32]);
    crate::boot_part_b::fn_1840fc(m, manager + 32, h);
    byte(m, slot + 0xe4, 2);
    crate::boot_part_b::fn_18423c(m, manager + 32, h);
    let negative = m.byte(slot + 0xe6) != 0;
    let allocator = m.word(slot + if negative { 0xdc } else { 0xd8 });
    let arg = m.word(slot + 0xe0);
    let arg = if negative { arg.wrapping_neg() } else { arg };
    for off in (4..24).step_by(4) {
        m.put_word(scratch + off, 0);
    }
    let vtable = m.word(globals.context);
    m.put_word(scratch, vtable);
    let virtual_base = m.word(vtable.wrapping_sub(0x30));
    m.put_word(
        scratch.wrapping_add(virtual_base),
        m.word(globals.context + 12),
    );
    let rc = fn_19b054(m, scratch + 4, slot, 1, globals.broker_table, h);
    let special = ((rc >> 10) & 255) == 17 && (100..=179).contains(&(rc & 1023));
    if special {
        crate::boot_part_b::fn_187498(m, h);
    } else {
        let rc = crate::boot_part_b::fn_18cf04(m, scratch + 4, scratch + 24, h);
        if rc & 0x80000000 != 0 {
            h.call(m, 0x13792c, &[rc, globals.length_assert_pc, 1]);
        }
        let length = m.word(scratch + 24);
        m.put_word(slot + 0xcc, length);
        let mode = m.byte(slot + 0xe5) as u32;
        let buffer = h.call(m, 0x163f20, &[allocator, length, arg, mode]);
        m.put_word(slot + 0xc0, buffer);
        let rc = crate::boot_part_b::fn_183be4(
            m,
            scratch + 4,
            scratch + 24,
            buffer,
            m.word(slot + 0xcc),
            h,
        );
        if rc & 0x80000000 != 0 {
            h.call(m, 0x13792c, &[rc, globals.read_assert_pc, 1]);
        }
    }

    for _ in 0..2 {
        let handle = m.word(scratch + 4);
        if handle & !1 != 0 {
            if handle & 1 != 0 {
                crate::boot_part_b::fn_17177c(m, h);
            }
            crate::boot_part_b::fn_183aec(m, handle & !1, h);
            m.put_word(scratch + 4, 0);
        }
    }
    m.bytes[start..start + 32].copy_from_slice(&saved);
}

pub fn fn_19719c(m: &mut Memory<'_>, src: u32, heap: u32, out: u32, h: &mut impl Host) -> u32 {
    let text = m.word(src);
    let mut len = 0u32;
    if text != 0 {
        while m.byte(text.wrapping_add(len)) != 0 { len = len.wrapping_add(1); }
    }
    let copy = h.call(m, 0x196394, &[heap, len.wrapping_add(13)]);
    if copy == 0 { return 0x8220600a; }

    for i in 0..12 { byte(m, copy+i, m.byte(src+i)); }
    m.put_word(copy, copy+12);
    if len == 0 { byte(m, copy+12, 0); }
    else {
        let text = m.word(src);
        let mut i=0;
        loop {
            let c=m.byte(text+i); byte(m, copy+12+i, c);
            if c==0 {break;} i+=1;
        }
    }
    m.put_word(src+4,0); m.put_word(src+8,0); m.put_word(out,copy);
    0
}

pub fn fn_197218(m: &mut Memory<'_>, value: u32, heap: u32, h: &mut impl Host) {
    if value != 0 {
        let attributes=m.word(value+4);
        if attributes!=0 { h.call(m,0x19665c,&[attributes]); }
        let children=m.word(value+8);
        if children!=0 { h.call(m,0x19665c,&[children]); }
        h.call(m,0x196428,&[heap,value]);
    }
}

pub fn guest_19665c_live(m: &mut Memory<'_>, object:u32, h:&mut impl Host) -> u32 {
    fn destroy(m:&mut Memory<'_>, tree:u32, node:u32, h:&mut impl Host) {
        if node==tree {return;}
        destroy(m,tree,m.word(node),h);
        destroy(m,tree,m.word(node+4),h);
        let key=m.word(node+16);
        if key!=0 {h.call(m,m.word(m.word(tree+36)+4),&[key,m.word(tree+32)]);}
        let value=m.word(node+20);
        if value!=0 {h.call(m,m.word(m.word(tree+36)+16),&[value,m.word(tree+32)]);}
        h.call(m,0x196428,&[m.word(tree+32),node]);
    }
    if object==0 {return 0x89002007;}
    let tree=object+24;
    destroy(m,tree,m.word(tree),h);
    m.put_word(tree+24,0);m.put_word(tree,tree);m.put_word(tree+4,tree);
    h.call(m,0x196428,&[m.word(object+20),object]);
    0
}

pub fn guest_19668c_live(m:&mut Memory<'_>, object:u32, key:u32, value:u32,
    scratch:u32, h:&mut impl Host) -> u32 {
    if object==0 || key==0 || value==0 {return 0x89002007;}
    use core::cell::RefCell;
    struct Calls<'a,'b,H>(&'a RefCell<&'b mut H>);
    impl<H:Host> Host for Calls<'_, '_, H> {
        fn thread_id(&mut self)->u32 {self.0.borrow_mut().thread_id()}
        fn call(&mut self,m:&mut Memory<'_>,f:u32,a:&[u32])->u32 {self.0.borrow_mut().call(m,f,a)}
    }
    struct Heap<'a,'b,H>(&'a RefCell<&'b mut H>, u32);
    impl<H:Host> crate::frontend::HeapVtableM for Heap<'_,'_,H> {
        fn alloc_at_m(&mut self,context:u32,size:u32,bytes:&mut [u8])->u32 {
            let mut m=Memory {base:0,bytes};
            let f=m.word(m.word(self.1));
            let arg=context;
            self.0.borrow_mut().call(&mut m,f,&[arg,size])
        }
        fn realloc_at_m(&mut self,_:u32,_:u32,_:u32,_:&mut [u8])->u32 {unreachable!()}
        fn release_at_m(&mut self,context:u32,pointer:u32,bytes:&mut [u8]) {
            let mut m=Memory {base:0,bytes};
            let f=m.word(m.word(self.1)+12);
            let arg=context;
            self.0.borrow_mut().call(&mut m,f,&[arg,pointer]);
        }
    }
    let shared=RefCell::new(h);
    let mut calls=Calls(&shared);
    let mut heap=Heap(&shared,m.word(object+56));
    let mut tree=FrontendTreeCalls {host:&mut calls,vtable:object,scratch};
    guest_19668c_m(m,object,key,value,&mut heap,&mut tree)
}

pub fn lifecycle_call(m:&mut Memory<'_>,f:u32,a:&[u32],scratch:u32,h:&mut impl Host)->Option<u32> {
    struct Calls<'a,H>(&'a mut H);
    impl<H:Host> crate::boot::Calls for Calls<'_,H> {
        fn call_wide(&mut self,_:&mut Memory<'_>,f:u32,_:&[u32])->u64 {panic!("unexpected wide lifecycle callee {f:x}")}
        fn call(&mut self,m:&mut Memory<'_>,f:u32,a:&[u32])->u32 {self.0.call(m,f,a)}
    }
    impl<H:Host> crate::speech_part_4::Host for Calls<'_,H> {
        fn call(&mut self,m:&mut Memory<'_>,f:u32,a:&[u32])->u32 {self.0.call(m,f,a)}
    }
    Some(match f {
        0x19719c => fn_19719c(m,a[0],a[1],a[2],h),
        0x197218 => {fn_197218(m,a[0],a[1],h);0},
        0x196964 => {crate::speech_part_4::fn_196964(m,&mut Calls(h),a[0],a[1]);0},
        0x19690c|0x196980|0x1966ac|0x198760 => {
            m.boot_parameter_call(&mut Calls(h),f,a,scratch).expect("existing frontend binding")
        },
        0x19668c => guest_19668c_live(m,a[0],a[1],a[2],scratch,h),
        0x19665c => guest_19665c_live(m,a[0],h),
        0x1964e0 => {
            struct Alloc(u32);
            impl crate::frontend::MethodTable for Alloc {
                fn call_at(&mut self,_:u32,_:u32,_:u32)->u32 {self.0}
            }
            let valid=a[7]!=0 && a[2..7].iter().all(|x|*x!=0);
            let copy=if valid {h.call(m,0x1963b8,&[a[0],1,68])}else{0};
            guest_1964e0(m,a.try_into().unwrap(),&mut Alloc(copy))
        },
        _=>return None,
    })
}
