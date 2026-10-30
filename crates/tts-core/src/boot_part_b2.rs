//! Boot part b2. Guest addresses and caller-owned memory, no native pointers.
use crate::boot::Memory;
use crate::boot_part_b::Host;

fn byte(m: &mut Memory<'_>, p: u32, v: u8) {
    let i = m.offset(p);
    m.bytes[i] = v;
}
fn half(m: &Memory<'_>, p: u32) -> u16 {
    u16::from_le_bytes([m.byte(p), m.byte(p.wrapping_add(1))])
}

/// fn_199458. The aligned RVCT copy advances r0, unlike ISO C memcpy.
/// Each LDM block is read before its STM, including when ranges overlap.
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

/// fn_1994cc. Tail call to the ROM's bx-lr leaf fn_1a2ea8.
pub fn fn_1994cc(r0: u32) -> u32 {
    crate::boot_part_c::return_leaf(r0)
}

/// fn_183edc. Uncontended exclusive updates cannot fail in this single-thread
/// arena. A platform wait must update the caller-owned semaphore before returning.
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

/// fn_184c58. The SVC veneer is the real part-c port.
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

/// fn_1858e4. Linker-removed weak atexit and guard-release calls are no-ops.
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

/// fn_18ac58. Object constructor is not yet a boot port.
pub fn fn_18ac58(m: &mut Memory<'_>, guard: u32, object: u32, _h: &mut impl Host) -> u32 {
    if m.word(guard) & 1 == 0 && m.acquire_guard(guard) != 0 {
        m.fn_13e4d0(object);
    }
    object
}

/// fn_19b1d0. Tick and sleep are platform boundaries.
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

/// fn_19b78c. Heap release is provided by the caller's engine dispatcher.
pub fn fn_19b78c(m: &mut Memory<'_>, p: u32, block: u32, h: &mut impl Host) {
    h.call(m, 0x184f84, &[p + 24, block]);
    m.put_word(p + 84, m.word(p + 84).wrapping_sub(1));
}

/// fn_19ba10. Scratch is 48 writable bytes disjoint from epoch and output,
/// restored after the real part-a clock and conversion consume the wrapping product.
/// scratch names 48 disjoint writable bytes. globals maps the regional clock.
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

/// fn_19de08. locale is the RVCT classification byte table.
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

/// fn_19dcd8. Uses the real path resolver.
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

/// Typed ABI for an unported broker's open method. The local output slot and
/// three-word path descriptor do not expose Rust stack addresses.
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
/// fn_19b054.
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
/// The ticket and attribute methods are explicit unported engine dependencies.
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
/// fn_198294.
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

/// fn_183524. Uses part-b slot search, initialization and fatal dispatch.
/// pattern is the caller-mapped regional D_8c81a0 string. semaphore is
/// the D_a7bb7c pointer cell; callback is the D_a7ba08 pointer cell.
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

/// Compose slot loading with the real resource reader and guest block heap.
/// Platform file operations remain explicit. The arena is absolute-indexed.
/// resource_scratch follows fn_132ab8's disjoint, restored 32-byte contract.
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

/// fn_189040. Each recursive archive directory needs 32 caller-owned scratch
/// bytes. Temporary iterator bytes are restored, as the ROM uses its stack.
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

/// fn_18cae8. KR omits the US tail byte and shifts later fields by four.
/// The caller selects the engine object's region, never a guessed layout.
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

/// fn_197254. Attribute parser, with the real entity decoder.
/// table is the region's D_8dc50c. out is a three-word parser result.
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

/// fn_197630. Caller-owned scratch is 12 bytes per active recursive element,
/// disjoint from context, output, text and table, and is restored on return.
/// Engine allocation and object methods remain Host calls.
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

/// fn_197c78. The real element/attribute parsers run below this entry.
/// scratch has 64 local bytes plus the element parser's recursive scratch,
/// disjoint from host allocations and all input/output, and is restored.
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
    // On an I/O failure the C releases its buffer before the common check.
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

/// Caller-owned inherited VFP bits, not floating-point defaults.
/// The caller must carry these across the calls preceding an event.
///
/// Round 7: the C reader search for 0x61cc/0x61dc finds only fn_183258
/// and fn_35b380. The latter drops the vector and frees its buffer without
/// reading elements. fn_183258 copies old records when growing the vector;
/// those transport reads do not consume either field. Event.first/last
/// are words +4/+28 in each 32-byte
/// record, not independent Speech fields at +0x110/+0x12c.
/// Three complete US ARM renders with stored words XOR-perturbed have
/// identical PCM and zero reads of those words. Every mapped non-stack
/// byte was compared: only the deliberately changed event bytes differ.
/// They are retained diagnostic state, not proven audio inputs. Preserve
/// the bits for exact whole-arena reproduction; do not infer a live game's
/// register values from memory-only snapshots. See round7 evidence.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct EventVfp {
    pub s16: u32,
    pub s17: u32,
}

impl EventVfp {
    /// The snapshot's MAIN_LOOP shim loads s16 from its guest literal pool.
    /// s17 remains the incoming caller register. Do not initialize it here:
    /// the memory-only PRE fixtures do not capture thread VFP registers.
    pub fn load_main_loop_s16(&mut self, m: &Memory<'_>, literal: u32) {
        self.s16 = m.word(literal);
    }
}

/// The KR Speech event vector is sixteen bytes later.
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

/// fn_183258 for live callers with an explicit tick (ARM 0x183258).
///
/// The record layout, the at-capacity decision and the write/advance are
/// exactly the ARM's: reserved = capacity & 0x3fffffff, count =
/// (end - begin) >> 5; when count >= reserved and growth is not owned
/// (no allocator, or flags & 0xc0000000 != 0x40000000) the ARM takes the
/// early return at +0x54/0x68 with no write and no end advance, and its
/// void return is ignored by fn_163a98. The live void convention is 0,
/// never an error: every live caller of the notify ignores the return.
///
/// The tick is a caller input (the ARM's getSystemTick is constant over
/// a live run; the seg lanes carry the last queue record's tick, the
/// backend carries its host tick). Growth owned by the app allocator is
/// unreachable in live images: the boot sizes the vector to 100 with
/// flags 0 and nothing ever grows it, so the owned-growth path the ARM
/// would take via the allocator never arises. If a future image shows
/// flags 0x40000000 at capacity, wire growth through fn_183258 above
/// with a host instead of extending this function.
///
/// Shared by the engine_live and seg7 notify arms so the at-capacity
/// behaviour lives in one place.
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
            // Growth unowned here (no app allocator bound in the live
            // path; flags are 0 in every live image): silent drop.
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

/// fn_183258. Uninitialized C event fields are live inherited VFP registers.
/// Allocation callbacks must preserve this state just as the ARM ABI does.
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

/// Platform tick plus guest allocator methods used by the event vector.
pub trait EventHost: Host {
    fn tick(&mut self, memory: &mut Memory<'_>) -> u64;
}

/// The composed boot uses an absolute, zero-based caller-owned arena.
/// No pointer rebasing or host allocation occurs in these adapters.
pub(crate) fn guest_184f84(m: &mut Memory<'_>, sub: u32, pointer: u32) {
    m.boot_heap_release(sub, pointer);
}

/// fn_19b78c composed with the real guest heap instead of a scripted release.
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

/// Reentrant map insertion. Heap and tree handlers borrow the current arena
/// only for the duration of each callback. Legacy adapters remain unchanged.
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

/// Dispatch the five map lifecycle methods through their live guest targets.
/// The host must bind real ports or stop at the exact missing callee. In
/// particular, bind lifecycle_call in the host for fn_19719c and fn_197218.
///
/// scratch is a caller-owned, disjoint four-byte word. Nested dispatchers need
/// distinct scratch words. No callback retains a view of the guest arena.
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
        // The callee writes the actual node slot, including on failure.
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

/// Defer only the final release so the real mutable tree teardown runs first.
/// release_cleared borrows its memory immutably and reports teardown by callback.
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

#[cfg(test)]
mod guest_tests {
    extern crate std;
    use super::*;
    struct Methods {
        output: u32,
        calls: usize,
    }
    impl crate::frontend::MethodTable for Methods {
        fn call_at(&mut self, context: u32, count: u32, size: u32) -> u32 {
            assert_eq!((context, count, size), (0x1234, 1, 0x44));
            self.calls += 1;
            self.output
        }
    }
    struct Heap {
        next: u32,
        freed: std::vec::Vec<(u32, u32)>,
    }
    impl crate::frontend::HeapVtable for Heap {
        fn alloc_at(&mut self, context: u32, size: u32) -> u32 {
            assert_eq!((context, size), (0x1234, 24));
            let n = self.next;
            self.next += 32;
            n
        }
        fn realloc_at(&mut self, _: u32, _: u32, _: u32) -> u32 {
            panic!("unexpected realloc")
        }
        fn release_at(&mut self, context: u32, object: u32) {
            self.freed.push((context, object));
        }
    }
    struct Tree;
    impl crate::frontend::TreeOps for Tree {
        fn tree_cmp(&mut self, key: u32, other: u32) -> i32 {
            key.cmp(&other) as i32
        }
        fn tree_init(&mut self, key: u32, _: u32, _: u32) -> (u32, Option<u32>) {
            (0, Some(key))
        }
        fn tree_alloc(&mut self, arg: u32, _: u32) -> (u32, u32) {
            (0, arg)
        }
        fn tree_free_node(&mut self, _: u32, _: u32) {}
        fn tree_free_handle(&mut self, _: u32, _: u32) {}
    }
    #[test]
    fn guest_frontend_success_and_guards() {
        let mut bytes = [0x55; 4096];
        let mut m = Memory {
            base: 0,
            bytes: &mut bytes,
        };
        m.put_word(0x104, 0x1234);
        let mut methods = Methods {
            output: 0x200,
            calls: 0,
        };
        let args = [0x100, 0, 0x601, 0x602, 0x603, 0x604, 0x605, 0x180];
        assert_eq!(guest_1964e0(&mut m, args, &mut methods), 0);
        assert_eq!(methods.calls, 1);
        assert_eq!(m.word(0x180), 0x200);
        assert_eq!(m.word(0x214), 0x100);
        assert_eq!(m.word(0x218), 0x218);
        let mut heap = Heap {
            next: 0x300,
            freed: std::vec::Vec::new(),
        };
        assert_eq!(
            guest_19668c(&mut m, 0x200, 10, 100, &mut heap, &mut Tree),
            0
        );
        assert_eq!(
            guest_19668c(&mut m, 0x200, 20, 200, &mut heap, &mut Tree),
            0
        );
        assert_eq!(m.word(0x230), 2);
        let mut destroyed = std::vec::Vec::new();
        assert_eq!(
            guest_19665c(&mut m, 0x200, &mut heap, &mut |p, h| destroyed.push((p, h))),
            0
        );
        assert_eq!(m.word(0x218), 0x218);
        assert_eq!(m.word(0x230), 0);
        assert_eq!(heap.freed.last(), Some(&(0x1234, 0x200)));
        assert_eq!(heap.freed.len(), 3);
        assert_eq!(destroyed.len(), 4);
        let before = m.bytes.to_vec();
        let mut bad = args;
        bad[3] = 0;
        assert_eq!(guest_1964e0(&mut m, bad, &mut methods), 0x89002007);
        assert_eq!(
            guest_19668c(&mut m, 0, 1, 2, &mut heap, &mut Tree),
            0x89002007
        );
        assert_eq!(
            guest_19665c(&mut m, 0, &mut heap, &mut |_, _| panic!("null clear")),
            0x89002007
        );
        assert_eq!(m.bytes, before);
        assert_eq!(methods.calls, 1);
    }
    #[test]
    fn guest_heap_release_merges_neighbors_and_restores_prefix() {
        let mut bytes = [0xa5; 2048];
        let mut m = Memory {
            base: 0,
            bytes: &mut bytes,
        };
        let outer = 0x100;
        for off in 0..88 {
            byte(&mut m, outer + off, 0);
        }
        let free = 0x400;
        let used = 0x440;
        let after = 0x480;
        for (p, size, prev, next, tag) in [
            (free, 48, 0, after, 0x4652u16),
            (used, 48, 0, 0, 0x5544),
            (after, 48, free, 0, 0x4652),
        ] {
            m.put_half(p, tag);
            m.put_half(p + 2, 0);
            m.put_word(p + 4, size);
            m.put_word(p + 8, prev);
            m.put_word(p + 12, next);
        }
        for (off, v) in [(0x3c, free), (0x40, after), (0x44, used), (0x48, used)] {
            m.put_word(outer + off, v);
        }
        let prefix = m.bytes[..88].to_vec();
        guest_184f84(&mut m, outer + 24, used + 16);
        assert_eq!(&m.bytes[..88], prefix);
        assert_eq!(m.word(outer + 0x3c), free);
        assert_eq!(m.word(outer + 0x40), free);
        assert_eq!(m.word(outer + 0x44), 0);
        assert_eq!(m.word(outer + 0x48), 0);
        assert_eq!(m.word(free + 4), 176);
    }
}

/// Regional literals and assertion return PCs used by fn_132ab8.
/// context is the address of the constructor's four-word vtable descriptor.
pub struct ResourceGlobals {
    pub context: u32,
    pub broker_table: u32,
    pub length_assert_pc: u32,
    pub read_assert_pc: u32,
}

/// fn_132ab8. Regional C/ARM/Rust differential coverage is recorded separately.
/// Read a resource through the real URI, size, read and close ports.
/// scratch is 32 caller-owned writable bytes, disjoint from the slot, locks,
/// resource buffer and all callback-accessible state. It is restored on return.
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
    // Explicit close and the C++ local's destructor both check the handle.
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

#[cfg(test)]
mod resource_tests {
    use super::*;
    struct Files {
        allocation: [u32; 4],
        reads: u32,
        closes: u32,
    }
    impl Host for Files {
        fn thread_id(&mut self) -> u32 {
            0x12345678
        }
        fn call(&mut self, _: &mut Memory<'_>, f: u32, a: &[u32]) -> u32 {
            match f {
                0x165d00 => {
                    assert_eq!(a, &[0x120]);
                    0
                }
                0x163f20 => {
                    self.allocation.copy_from_slice(a);
                    0x1800
                }
                0x11001100 => {
                    assert_eq!(a, &[0x700]);
                    self.closes += 1;
                    0
                }
                _ => panic!("unexpected resource call {f:x}"),
            }
        }
    }
    impl UriHost for Files {
        fn open(
            &mut self,
            _: &mut Memory<'_>,
            method: u32,
            broker: u32,
            kind: u32,
            path: u32,
            length: u32,
            mode: u32,
        ) -> (u32, u32) {
            assert_eq!(
                (method, broker, kind, path, length, mode),
                (0x11001000, 0x500, 4, 0x1008, 6, 1)
            );
            (0, 0x700)
        }
    }
    impl crate::boot_part_b::PairHost for Files {
        fn pair(&mut self, _: &mut Memory<'_>, handle: u32) -> (u32, u32, u32) {
            assert_eq!(handle, 0x700);
            (0, 4, 0)
        }
    }
    impl crate::boot_part_b::ReadHost for Files {
        fn read(
            &mut self,
            m: &mut Memory<'_>,
            handle: u32,
            lo: u32,
            hi: u32,
            dst: u32,
            len: u32,
        ) -> (u32, u32) {
            assert_eq!((handle, lo, hi, dst, len), (0x700, 0, 0, 0x1800, 4));
            m.put_word(dst, 0x78563412);
            self.reads += 1;
            (0, 4)
        }
    }
    #[test]
    fn resource_reader_uses_real_file_ports_and_restores_scratch() {
        for negative in [false, true] {
            let mut arena = [0u8; 0x4000];
            let mut m = Memory {
                base: 0,
                bytes: &mut arena,
            };
            m.put_word(0x124, 0x12345678);
            for (i, v) in "rom:/x".encode_utf16().enumerate() {
                m.put_half(0x1000 + i as u32 * 2, v);
            }
            byte(&mut m, 0x10e6, u8::from(negative));
            byte(&mut m, 0x10e5, 7);
            m.put_word(0x10d8, 0xaaa);
            m.put_word(0x10dc, 0xbbb);
            m.put_word(0x10e0, 13);
            m.put_word(0x300, 0x400);
            m.put_word(0x30c, 0x444);
            m.put_word(0x3d0, 0);
            m.put_word(0x3000, 0x726f6d);
            m.put_word(0x3008, 0x500);
            m.put_word(0x500, 0x600);
            m.put_word(0x600, 0x11001000);
            m.put_word(0x700, 0x740);
            m.put_word(0x770, 0x11001100);
            m.bytes[0x2000..0x2020].fill(0xa5);
            let mut host = Files {
                allocation: [0; 4],
                reads: 0,
                closes: 0,
            };
            fn_132ab8(
                &mut m,
                0x100,
                0x1000,
                0x2000,
                &ResourceGlobals {
                    context: 0x300,
                    broker_table: 0x3000,
                    length_assert_pc: 1,
                    read_assert_pc: 2,
                },
                &mut host,
            );
            assert_eq!(
                host.allocation,
                [
                    if negative { 0xbbb } else { 0xaaa },
                    4,
                    if negative { 0u32.wrapping_sub(13) } else { 13 },
                    7
                ]
            );
            assert_eq!((host.reads, host.closes), (1, 1));
            assert_eq!(m.word(0x10c0), 0x1800);
            assert_eq!(m.word(0x10cc), 4);
            assert_eq!(m.byte(0x10e4), 2);
            assert_eq!(m.word(0x1800), 0x78563412);
            assert_eq!(&m.bytes[0x2000..0x2020], &[0xa5; 32]);
        }
    }
}

/// XML value clone/ownership transfer. fn_19719c / EU 19724c / KR 197288.
pub fn fn_19719c(m: &mut Memory<'_>, src: u32, heap: u32, out: u32, h: &mut impl Host) -> u32 {
    let text = m.word(src);
    let mut len = 0u32;
    if text != 0 {
        while m.byte(text.wrapping_add(len)) != 0 { len = len.wrapping_add(1); }
    }
    let copy = h.call(m, 0x196394, &[heap, len.wrapping_add(13)]);
    if copy == 0 { return 0x8220600a; }
    // fn_195ae8 copies forward, one byte at a time.
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

/// XML value destructor. US 197218 / EU 1972c8 / KR 197304.
/// The two child maps are destroyed before the value allocation is released.
pub fn fn_197218(m: &mut Memory<'_>, value: u32, heap: u32, h: &mut impl Host) {
    if value != 0 {
        let attributes=m.word(value+4);
        if attributes!=0 { h.call(m,0x19665c,&[attributes]); }
        let children=m.word(value+8);
        if children!=0 { h.call(m,0x19665c,&[children]); }
        h.call(m,0x196428,&[heap,value]);
    }
}

/// Live map destruction, equivalent to bridge::Flat::fn_19665c_with, with
/// allocator entries kept behind Host rather than fixed platform methods.
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

/// Bind the existing M insertion port to a single reentrant host. Each borrow
/// ends before the next callback; all nested calls see the current guest bytes.
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

/// Real lifecycle bindings. Other dependencies remain explicit host calls.
/// scratch is 128 disjoint caller-owned bytes, restored by shared helpers.
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
