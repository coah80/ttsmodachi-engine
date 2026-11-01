
use crate::boot::Memory;

pub fn fn_4a79b0(m: &mut Memory<'_>, p: u32) -> u32 {
    m.word(p + 8)
}

pub fn fn_2d075c(m: &mut Memory<'_>, p: u32, template: u32) {
    let pair = [m.word(template), m.word(template + 4)];
    m.put_word(p, pair[0]);
    m.put_word(p + 4, pair[1]);
}
pub fn fn_2d09a4(m: &mut Memory<'_>, p: u32, template: u32) {
    if p == 0 {
        return;
    }
    m.put_word(p + 12, 0);
    let mut i = 0;
    while i < m.word(p + 8) {
        let data = m.word(p + 20);
        fn_2d075c(m, data + i * 8, template);
        i += 1;
    }
}
pub fn fn_2d0a08(m: &mut Memory<'_>, p: u32, template: u32) {
    if p == 0 {
        return;
    }
    let mut i = 0;
    while i < m.word(p + 8) {
        let data = m.word(p + 20);
        fn_2d075c(m, data + i * 8, template);
        i += 1;
    }
}
pub fn fn_2dd870(m: &mut Memory<'_>, p: u32, value: u32) -> u32 {
    m.put_word(p + 0x1dc, 0);
    m.put_word(p + 0x1d8, value);
    for off in [0x1e8, 0x1ec, 0x1e0, 0x1e4] {
        m.put_word(p + off, 0);
    }
    m.put_word(p + 0x1f0, 1);
    0
}
pub fn fn_2ddf4c(m: &mut Memory<'_>, p: u32, rate: i32) -> u32 {
    if p != 0 && rate > 0 {
        m.put_word(p, rate as u32);
        m.put_word(p + 4, rate as u32 / 2);
        let scale = match rate {
            8000 => Some(0x41893),
            11025 => Some(0x2f8df),
            12000 => Some(0x2bb0d),
            16000 => Some(0x20c4a),
            22050 => Some(0x17c70),
            _ => None,
        };
        if let Some(scale) = scale {
            m.put_word(p + 8, scale);
        }
    }
    rate as u32
}
pub fn fn_6baddc(m: &mut Memory<'_>, _heap: u32, block: u32) -> u32 {
    m.word(block.wrapping_sub(12))
}
pub fn fn_2d9b88(m: &mut Memory<'_>, owner: u32, state: u32) -> u32 {
    let at = m.offset(state) as u32;
    crate::fe_text::memset_195bec(m.bytes, at, 0, 0x44);
    m.put_word(state, owner);
    0
}

pub trait Host {
    fn format_address_m(&mut self,_m:&Memory<'_>,_format:&[u8])->Option<u32> {None}
    fn real_log_args(&mut self)->bool {false}
    fn log_variadic(&mut self,_m:&mut Memory<'_>,_entry:u32,_a:[u32;4],_format:&[u8],_varargs:&[u32])->bool {false}

    fn thread_id(&mut self) -> u32 {
        panic!("unbound platform thread id")
    }
    fn call(&mut self, m: &mut Memory<'_>, method: u32, args: &[u32]) -> u32;
}
struct HeapMethods<'a, H> {
    host: &'a mut H,
    base: u32,
    methods: [u32; 4],
}
impl<H: Host> crate::frontend::HeapVtableM for HeapMethods<'_, H> {
    fn alloc_at_m(&mut self, obj: u32, size: u32, bytes: &mut [u8]) -> u32 {
        self.host.call(
            &mut Memory {
                base: self.base,
                bytes,
            },
            self.methods[0],
            &[obj, size],
        )
    }
    fn realloc_at_m(&mut self, obj: u32, old: u32, size: u32, bytes: &mut [u8]) -> u32 {
        self.host.call(
            &mut Memory {
                base: self.base,
                bytes,
            },
            self.methods[2],
            &[obj, old, size],
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
fn methods<'a, H: Host>(m: &Memory<'_>, h: &'a mut H, heap: u32) -> HeapMethods<'a, H> {
    let mut slots = [0; 4];
    if heap != 0 {
        let vt = m.word(heap);
        for (i, slot) in slots.iter_mut().enumerate() {
            *slot = m.word(vt + i as u32 * 4);
        }
    }
    HeapMethods {
        host: h,
        base: m.base,
        methods: slots,
    }
}
fn alloc(m: &mut Memory<'_>, h: &mut impl Host, heap: u32, size: u32) -> u32 {
    if heap == 0 || size == 0 {
        return 0;
    }
    let mut vt = methods(m, h, heap);
    let off = if heap == 0 { None } else { Some(heap - m.base) };
    crate::frontend::heap_alloc_m(m.bytes, off, size, &mut vt)
}
fn realloc(m: &mut Memory<'_>, h: &mut impl Host, heap: u32, p: u32, size: u32) -> u32 {
    if heap == 0 || size == 0 {
        return 0;
    }
    let mut vt = methods(m, h, heap);
    let off = if heap == 0 { None } else { Some(heap - m.base) };
    crate::frontend::heap_realloc_m(m.bytes, off, p, size, &mut vt)
}
fn free(m: &mut Memory<'_>, h: &mut impl Host, heap: u32, p: u32) {
    if heap == 0 || p == 0 {
        return;
    }
    let mut vt = methods(m, h, heap);
    let off = if heap == 0 { None } else { Some(heap - m.base) };
    crate::frontend::heap_release_m(m.bytes, off, if p == 0 { None } else { Some(p) }, &mut vt);
}
pub fn fn_2d0a5c(m: &mut Memory<'_>, h: &mut impl Host, heap: u32, n: u32, template: u32) -> u32 {
    let p = alloc(m, h, heap, 28);
    if p == 0 {
        return 0;
    }
    m.put_word(p + 8, n);
    m.put_word(p + 12, 0);
    m.put_word(p + 4, 0);
    m.put_word(p + 16, 0);
    m.put_half(p, 0x5622);
    m.put_word(p + 24, heap);
    let data = alloc(m, h, heap, m.word(p + 8).wrapping_mul(8));
    m.put_word(p + 20, data);
    if data == 0 {
        free(m, h, m.word(p + 24), p);
        return 0;
    }
    m.put_word(p + 12, 0);
    let mut i = 0;
    while i < m.word(p + 8) {
        let data = m.word(p + 20);
        fn_2d075c(m, data + i * 8, template);
        i += 1;
    }
    p
}
pub fn fn_2db9b0(m: &mut Memory<'_>, h: &mut impl Host, heap: u32, p: u32, count: u32) -> u32 {
    init_buffer(m, h, heap, p, count, 1)
}
pub fn fn_2dba38(m: &mut Memory<'_>, h: &mut impl Host, heap: u32, p: u32, count: u32) -> u32 {
    init_buffer(m, h, heap, p, count, 2)
}
fn init_buffer(
    m: &mut Memory<'_>,
    h: &mut impl Host,
    heap: u32,
    p: u32,
    count: u32,
    shift: u32,
) -> u32 {
    let size = count.wrapping_shl(shift);
    m.put_word(p, heap);
    m.put_word(p + 4, 0);
    m.put_word(p + 12, 0);
    m.put_word(p + 8, 0);
    let data = alloc(m, h, heap, size);
    m.put_word(p + 4, data);
    if data == 0 {
        return u32::MAX;
    }
    m.put_word(p + 8, count);
    m.put_word(p + 12, count);
    let at = m.offset(data) as u32;
    crate::fe_text::memset_195bec(m.bytes, at, 0, size);
    0
}
pub fn fn_2dcf84(m: &mut Memory<'_>, p: u32) -> u32 {
    for off in [0xcc, 0xbc, 0x10c, 0x154, 0x170] {
        m.put_word(p + off, 0);
    }
    m.put_word(p + 0x16c, u32::MAX);
    m.put_word(p + 0x168, 4);
    m.put_half(p + 0x164, 2);
    m.put_word(p + 0x17c, 0);
    let samples = crate::morphology::duration_samples(m.word(p + 4), 4);
    m.put_word(p + 0x174, samples);
    0
}
pub fn fn_2cf75c(m: &mut Memory<'_>, h: &mut impl Host, p: u32, order: u32) -> u32 {
    if p == 0 {
        return 0;
    }
    if m.word(p + 16) == order {
        return 1;
    }
    let capacity = m.word(p + 20);
    if order <= capacity {
        m.put_word(p + 16, order);
        filter_indices(m, p);
        m.put_word(p + 28, m.word(p + 16).wrapping_add(1) >> 1);
        return 1;
    }
    m.put_word(p + 20, order);
    for (off, mult, extra) in [(0, 2u32, 2u32), (4, 4, 4), (8, 4, 4), (12, 8, 8)] {
        let block = realloc(
            m,
            h,
            m.word(p + 32),
            m.word(p + off),
            m.word(p + 20).wrapping_mul(mult).wrapping_add(extra),
        );
        if block == 0 {
            m.put_word(p + 16, 0);
            return 0;
        }
        m.put_word(p + off, block);
    }
    filter_indices(m, p);
    let mut i = m.word(p + 16).wrapping_add(1);
    while i <= m.word(p + 20) {
        m.put_half(m.word(p) + i * 2, 0);
        m.put_word(m.word(p + 4) + i * 4, 0);
        i = i.wrapping_add(1);
    }
    m.put_word(p + 16, m.word(p + 20));
    m.put_word(p + 28, m.word(p + 20).wrapping_add(1) >> 1);
    1
}
fn filter_indices(m: &mut Memory<'_>, p: u32) {
    let mut i = 0i32;
    while i < (m.word(p + 20).wrapping_add(1) as i32) {
        let divisor = m.word(p + 16).wrapping_add(1) as i32;
        m.put_word(
            m.word(p + 8) + (i as u32) * 4,
            i.wrapping_add(1).wrapping_rem(divisor) as u32,
        );
        i = i.wrapping_add(1);
    }
    i = 0;
    while i < (m.word(p + 20).wrapping_mul(2).wrapping_add(2) as i32) {
        let divisor = m.word(p + 16).wrapping_add(1) as i32;
        m.put_word(
            m.word(p + 12) + (i as u32) * 4,
            i.wrapping_rem(divisor) as u32,
        );
        i = i.wrapping_add(1);
    }
}

pub fn fn_2cfc1c(m: &mut Memory<'_>, h: &mut impl Host, pp: u32) {
    if pp == 0 {
        return;
    }
    let p = m.word(pp);
    if p == 0 {
        return;
    }
    let heap = m.word(p + 32);
    for off in [0, 4, 8, 12] {
        let block = m.word(p + off);
        if block != 0 {
            free(m, h, m.word(p + 32), block);
        }
        m.put_word(p + off, 0);
    }
    for off in [16, 20, 24, 28] {
        m.put_word(p + off, 0);
    }
    free(m, h, heap, m.word(pp));
    m.put_word(pp, 0);
}

pub fn fn_2cfd6c(
    m: &mut Memory<'_>,
    h: &mut impl Host,
    heap: u32,
    kind: u32,
    tables: [u32; 3],
    scratch: u32,
) -> u32 {
    if kind > 3 {
        return 0;
    }
    let p = alloc(m, h, heap, 36);
    if p == 0 {
        return 0;
    }
    m.put_word(p + 32, heap);
    for off in [16, 20, 24, 28, 0, 4, 8, 12] {
        m.put_word(p + off, 0);
    }
    let order = [7, 25, 5, 3][kind as usize];
    if fn_2cf75c(m, h, p, order) != 1 {
        let old = m.word(scratch);
        m.put_word(scratch, p);
        fn_2cfc1c(m, h, scratch);
        m.put_word(scratch, old);
        return 0;
    }
    let count = [8, 26, 5, 3][kind as usize];
    for i in 0..count {
        let value = if kind == 3 {
            [0xf703, 0x48ea, 0x48ea][i as usize]
        } else {
            let at = m.offset(tables[kind as usize] + i * 2);
            u16::from_le_bytes(m.bytes[at..at + 2].try_into().unwrap())
        };
        m.put_half(m.word(p) + i * 2, value);
    }
    p
}

pub fn fn_2dd064(m: &mut Memory<'_>, h: &mut impl Host, p: u32) -> u32 {
    let half = |m: &Memory<'_>, off: u32| {
        let at = m.offset(p + off);
        i16::from_le_bytes(m.bytes[at..at + 2].try_into().unwrap()) as i32
    };
    let count = half(m, 0x158).max(half(m, 0x15c));
    m.put_word(p + 0x3c, count as u32);
    for off in [0xb8, 0xc8, 0x178] {
        let heap = m.word(m.word(p) + 4);
        if fn_2db9b0(m, h, heap, p + off, count as u32) != 0 {
            return u32::MAX;
        }
        if off == 0xb8 {
            m.put_word(p + 0xc0, 0);
        }
    }
    0
}

pub fn fn_6bab68(m: &mut Memory<'_>, heap: u32, block: u32, size: u32) -> u32 {
    assert_eq!(m.base, 0);
    assert!(heap >= 88);
    let mut saved = [0; 88];
    saved.copy_from_slice(&m.bytes[..88]);
    m.bytes.copy_within(heap as usize..heap as usize + 88, 0);
    let result = crate::heap::EngineHeap::from_arena(m.bytes)
        .block_resize(block, size)
        .unwrap_or(0);
    m.bytes.copy_within(0..88, heap as usize);
    m.bytes[..88].copy_from_slice(&saved);
    result
}

fn half(m: &Memory<'_>, p: u32) -> u16 {
    let at = m.offset(p);
    u16::from_le_bytes(m.bytes[at..at + 2].try_into().unwrap())
}
struct Platform<'a, H>(&'a mut H);
impl<H: Host> crate::boot_part_f::Host for Platform<'_, H> {
    fn call(&mut self, m: &mut Memory<'_>, f: u32, a: &[u32]) -> u32 {
        self.0.call(m, f, a)
    }
    fn compare(
        &mut self,
        _: &mut Memory<'_>,
        _: u32,
        _: &mut crate::boot_part_f::Key,
        _: &mut u32,
    ) -> u32 {
        panic!("unexpected comparator")
    }
}
fn object_alloc(m: &mut Memory<'_>, h: &mut impl Host, heap: u32, n: u32, err: &mut u8) -> u32 {
    crate::boot_part_f::fn_1b2140(m, heap, n, err, &mut Platform(h))
}
fn object_free(m: &mut Memory<'_>, h: &mut impl Host, heap: u32, p: u32) {
    crate::boot_part_f::fn_1b2128(m, heap, p, &mut Platform(h));
}

fn calloc(m: &mut Memory<'_>, h: &mut impl Host, heap: u32, n: u32, size: u32) -> u32 {
    struct Method<'a, 'b, H> {
        m: &'a mut Memory<'b>,
        h: &'a mut H,
        f: u32,
    }
    impl<H: Host> crate::frontend::MethodTable for Method<'_, '_, H> {
        fn call_at(&mut self, obj: u32, a: u32, b: u32) -> u32 {
            self.h.call(self.m, self.f, &[obj, a, b])
        }
    }
    let mut record = [0u8; 12];
    let f = if heap == 0 {
        0
    } else {
        m.word(m.word(heap) + 4)
    };
    let obj = if heap == 0 { 0 } else { m.word(heap + 4) };
    record[8..12].copy_from_slice(&obj.to_le_bytes());
    crate::frontend::call_method(
        &record,
        if heap == 0 { None } else { Some(4) },
        n,
        size,
        &mut Method { m, h, f },
    )
}
pub fn fn_2dd914(m: &mut Memory<'_>, h: &mut impl Host, p: u32, b: u32) -> u32 {
    if m.word(p + 0x1f0) == 0 {
        m.put_word(p + 0x1dc, b);
        let n = b.wrapping_add(m.word(p + 0x1e4).wrapping_mul(2));
        if n != m.word(p + 0x1ec) {
            let q = calloc(m, h, m.word(m.word(p) + 4), 1, n.wrapping_mul(4));
            if q == 0 {
                return u32::MAX;
            }
            m.put_word(p + 0x1e8, q);
            m.put_word(p + 0x1ec, n);
        }
    }
    0
}
pub fn fn_2dd97c(m: &mut Memory<'_>, h: &mut impl Host, p: u32, s: u32) -> u32 {
    if s == 0 {
        m.put_word(p + 0x1f0, 1);
        m.put_word(p + 0x1d4, 0);
        return 0;
    }
    let cur = m.word(p + 0x1d4);
    m.put_word(p + 0x1f0, 0);
    if cur == s {
        return 0;
    }
    m.put_word(p + 0x1d4, s);
    let q = calloc(
        m,
        h,
        m.word(m.word(p) + 4),
        1,
        m.word(s + 4).wrapping_mul(4),
    );
    if q == 0 {
        return u32::MAX;
    }
    m.put_word(p + 0x1e0, q);
    m.put_word(p + 0x1e4, m.word(m.word(p + 0x1d4) + 4));
    let mut i = 0;
    while i < (m.word(p + 0x1e4) as i32) {
        m.put_word(m.word(p + 0x1e0) + i as u32 * 4, 0);
        i += 1;
    }
    0
}
pub fn fn_30098c(
    m: &mut Memory<'_>,
    h: &mut impl Host,
    heap: u32,
    _dictionary: u32,
    entries: u32,
    _count: u32,
) -> u32 {
    object_free(m, h, heap, entries);
    0
}

pub fn fn_300974(
    m: &mut Memory<'_>,
    h: &mut impl Host,
    heap: u32,
    _dictionary: u32,
    entries: u32,
    _count: u32,
) -> u32 {
    object_free(m, h, heap, entries);
    0
}

pub fn fn_2da9c0(m: &mut Memory<'_>, a: u32, p: u32, name: u32) {
    let v = crate::selection::PrivateHeap::initialize(a, &[]);
    let at = m.offset(p) as u32;
    crate::fe_text::memset_195bec(m.bytes, at, 0, 24);
    m.put_word(p, v.owner);
    m.put_word(p + 4, name);
    m.put_word(p + 12, v.expected);
}
pub fn fn_2f9740(
    m: &mut Memory<'_>,
    h: &mut impl Host,
    a: u32,
    b: u32,
    out: u32,
    vtable: u32,
    e: &crate::boot_part_f::Environment,
) -> u32 {
    let mut err = 0;
    let p = object_alloc(m, h, a, 28, &mut err);
    if err != 0 {
        return err as u32;
    }
    err = crate::boot_part_f::fn_1ba3c0(m, p, e) as u8;
    if err == 0 {
        m.put_word(p, vtable);
        m.put_word(p + 4, b);
        m.put_word(p + 12, 50);
        m.put_word(p + 20, 50);
        let q = object_alloc(m, h, b, m.word(p + 12), &mut err);
        m.put_word(p + 8, q);
        if err == 0 {
            let q = object_alloc(m, h, m.word(p + 4), m.word(p + 20), &mut err);
            m.put_word(p + 16, q);
            if err == 0 {
                m.put_word(p + 24, 0);
            }
        }
    }
    if err == 0 {
        m.put_word(out, p);
    } else {
        object_free(m, h, a, p);
        m.put_word(out, 0);
    }
    err as u32
}
pub fn fn_2e4598(
    m: &mut Memory<'_>,
    h: &mut impl Host,
    a: u32,
    b: u32,
    out: u32,
    vtable: u32,
    token_vtable: u32,
    e: &crate::boot_part_f::Environment,
) -> u32 {
    let mut err = 0;
    let p = object_alloc(m, h, a, 12, &mut err);
    if err == 0 {
        err = crate::boot_part_f::fn_1ba3c0(m, p, e) as u8;
        if err == 0 {
            m.put_word(p, vtable);
            m.put_word(p + 4, b);
            err = fn_2f9740(m, h, b, b, p + 8, token_vtable, e) as u8;
        }
        if err == 0 {
            m.put_word(out, p);
        } else {
            object_free(m, h, a, p);
            m.put_word(out, 0);
        }
    }
    err as u32
}
pub fn fn_2fbe90(
    m: &mut Memory<'_>,
    h: &mut impl Host,
    a: u32,
    out: &mut u32,
    vt: u32,
    e: &crate::boot_part_f::Environment,
) -> u32 {
    let mut err = 0;
    let p = object_alloc(m, h, a, 16, &mut err);
    if err == 0 {
        let rc = crate::boot_part_f::fn_1b449c(m, p, 1, e);
        if rc == 0 {
            m.put_word(p + 12, vt);
            m.put_word(p, vt - 24);
        }
        err = rc as u8;
        if rc == 0 {
            m.put_word(p + 8, a);
            *out = p;
        } else {
            object_free(m, h, a, p);
            *out = 0;
        }
    }
    err as u32
}
pub fn fn_2e4afc(
    m: &mut Memory<'_>,
    h: &mut impl Host,
    p: u32,
    a: u32,
    vt: u32,
    iface_vt: u32,
    e: &crate::boot_part_f::Environment,
) -> u32 {
    let mut rc = crate::boot_part_f::fn_1b449c(m, p, 1, e);
    if rc == 0 {
        m.put_word(p + 16, 0);
        m.put_word(p, vt - 24);
        m.put_word(p + 12, vt);
        m.put_word(p + 24, a);
        let mut q = 0;
        rc = fn_2fbe90(m, h, a, &mut q, iface_vt, e);
        if rc == 0 {
            m.put_word(p + 20, q + 12);
        }
    }
    rc
}

pub fn fn_c7233c(m: &mut Memory<'_>, handle: u32, state_pointer: u32, pair: u32) {
    let state = m.word(state_pointer);
    let first = m.word(handle);
    let second = m.word(handle + 4);
    m.put_word(pair + 4, first);
    m.put_word(pair, second);
    m.put_word(state + 12, 0);
    let at = m.offset(state);
    m.bytes[at] = 0;
    m.put_word(state + 16, 0);
    for off in 3..8 {
        let at = m.offset(state + off);
        m.bytes[at] = 50;
    }
    let at = m.offset(state + 8);
    m.bytes[at] = 0;
}

pub fn fn_c72000(m: &mut Memory<'_>, r0: u32, r8: u32, sp: u32, state_pointer: u32, pair: u32) {
    fn_c7233c(m, r8, state_pointer, pair);
    m.put_word(sp, r0);
}

struct Log<'a, H>(&'a mut H);
impl<H: Host> crate::log_live::LogMethodsM for Log<'_, H> {
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

pub struct Literals {
    pub category: u32,
    pub formats: [u32; 16],
    pub scratch: u32,
}
fn diagnostic(
    m: &mut Memory<'_>,
    h: &mut impl Host,
    ctx: u32,
    k: &Literals,
    which: usize,
    level: u32,
    args: &[u32],
) {
    let mut saved = [0u8; 16];
    assert!(args.len() <= 4);
    let at = m.offset(k.scratch);
    saved.copy_from_slice(&m.bytes[at..at + 16]);
    for (i, &a) in args.iter().enumerate() {
        m.put_word(k.scratch + i as u32 * 4, a);
    }
    crate::log_live::fn_1738e8(
        m,
        m.word(ctx + 16),
        k.category,
        level,
        0,
        k.formats[which],
        k.scratch,
        &mut Log(h),
    );
    m.bytes[at..at + 16].copy_from_slice(&saved);
}
fn copy(m: &mut Memory<'_>, dst: u32, src: u32, n: u32) {
    let d = m.offset(dst) as u32;
    let s = m.offset(src) as u32;
    crate::fe_text::memcpy_195ae8(m.bytes, d, s, n);
}
fn strlen(m: &Memory<'_>, p: u32) -> u32 {
    crate::fe_text::strlen_195d90(m.bytes, m.offset(p) as u32) as u32
}
fn strcpy(m: &mut Memory<'_>, d: u32, s: u32) {
    let d = m.offset(d) as u32;
    let s = m.offset(s) as u32;
    crate::normalization::strcpy_at(m.bytes, d, s);
}
pub fn fn_2d9c0c(
    m: &mut Memory<'_>,
    h: &mut impl Host,
    p: u32,
    idx: u32,
    d: u32,
    k: &Literals,
    names: u32,
) -> u32 {
    let hdr = m.word(d + 20).wrapping_add(m.word(d + 4));
    if m.word(d + 8) < 8 || m.byte(hdr) != 0x30 {
        diagnostic(m, h, m.word(p), k, 0, 0, &[m.word(names + idx * 4)]);
        return 0x84e02004;
    }
    let e8 = p + idx * 2;
    let e5 = p + idx * 4;
    copy(m, e8 + 56, hdr + 2, 2);
    let off = half(m, hdr + 4) as u32;
    copy(m, e8 + 4, hdr + 6, 2);
    m.put_word(e5 + 16, hdr + 8);
    let q = calloc(m, h, m.word(m.word(p) + 4), 8, half(m, e8 + 56) as u32);
    m.put_word(e5 + 36, q);
    if q == 0 {
        return 0x84e0200a;
    }
    let mut cur = hdr + off + 8 + (off & 1);
    let mut i = 0u32;
    while i < (half(m, e8 + 56) as u32) {
        copy(m, m.word(e5 + 36) + i * 8, cur, 2);
        cur += 2;
        copy(m, m.word(e5 + 36) + 2 + i * 8, cur, 2);
        let dat = cur + 2;
        m.put_word(m.word(e5 + 36) + 4 + i * 8, dat);
        let j = 2 + i * 8;
        i = (i + 1) as u16 as u32;
        cur = dat + half(m, m.word(e5 + 36) + j) as u32 * 6;
    }
    0
}
pub fn fn_300714(m: &mut Memory<'_>, h: &mut impl Host, p: u32, out: u32, out_count: u32) -> u32 {
    let heap = m.word(p + 36);
    let mut node = m.word(p + 40);
    let mut count = 0;
    let mut bytes = 0u32;
    while node != 0 {
        bytes = bytes.wrapping_add(20);
        for off in [4, 12] {
            let s = m.word(node + off);
            if s != 0 {
                bytes = bytes.wrapping_add(strlen(m, s) + 1);
            }
        }
        if m.word(node + 8) != 0 {
            bytes = bytes.wrapping_add(128);
        }
        count += 1;
        node = m.word(node + 32);
    }
    let mut err = 0;
    if bytes != 0 {
        let entries = object_alloc(m, h, heap, bytes, &mut err);
        if err != 0 {
            return err as u32;
        }
        node = m.word(p + 40);
        let mut entry = entries;
        let mut strings = entries + count * 20;
        while node != 0 {
            let record = m.word(node);
            m.put_word(entry, m.word(record + 8));
            m.put_word(entry + 4, m.word(record + 16));
            for off in [12, 16, 8] {
                m.put_word(entry + off, 0);
            }
            for (off, dst) in [(4, 8), (12, 16)] {
                let s = m.word(node + off);
                if s != 0 {
                    m.put_word(entry + dst, strings);
                    strcpy(m, strings, s);
                    strings += strlen(m, s) + 1;
                }
            }
            let s = m.word(node + 8);
            if s != 0 {
                m.put_word(entry + 12, strings);
                copy(m, strings, s, 128);
                strings += 128;
            }
            node = m.word(node + 32);
            entry += 20;
        }
        m.put_word(out, entries);
        m.put_word(out_count, count);
    }
    err as u32
}

pub fn fn_300594(
    m: &mut Memory<'_>,
    h: &mut impl Host,
    heap: u32,
    dictionary: u32,
    out: u32,
    out_count: u32,
) -> u32 {
    let mut node = m.word(dictionary + 40);
    let mut count = 0u32;
    let mut bytes = 0u32;
    while node != 0 {
        bytes = bytes.wrapping_add(20);
        let language = m.word(node + 20);
        if language != 0 {
            bytes = bytes.wrapping_add(strlen(m, language) + 1);
        }
        let encoding = m.word(node + 28);
        if encoding != 0 {
            bytes = bytes.wrapping_add(strlen(m, encoding) + 1);
        }
        if m.word(node + 24) != 0 {
            bytes = bytes.wrapping_add(128);
        }
        node = m.word(node + 76);
        count += 1;
    }
    let mut err = 0u8;
    if bytes != 0 {
        let entries = object_alloc(m, h, heap, bytes, &mut err);
        if err != 0 {
            return err as u32;
        }
        node = m.word(dictionary + 40);
        let mut entry = entries;
        let mut strings = entries + count * 20;
        while node != 0 {
            m.put_word(entry, m.word(node + 8));
            m.put_word(entry + 4, m.word(node + 16));
            m.put_word(entry + 8, 0);
            m.put_word(entry + 12, 0);
            m.put_word(entry + 16, 0);
            let language = m.word(node + 20);
            if language != 0 {
                m.put_word(entry + 8, strings);
                strcpy(m, strings, language);
                strings += strlen(m, language) + 1;
            }
            let encoding = m.word(node + 28);
            if encoding != 0 {
                m.put_word(entry + 16, strings);
                strcpy(m, strings, encoding);
                strings += strlen(m, encoding) + 1;
            }
            let mode = m.word(node + 24);
            if mode != 0 {
                m.put_word(entry + 12, strings);
                copy(m, strings, mode, 128);
                strings += 128;
            }
            node = m.word(node + 76);
            entry += 20;
        }
        m.put_word(out, entries);
        m.put_word(out_count, count);
    }
    err as u32
}

pub fn fn_2e39ec(m: &mut Memory<'_>, h: &mut impl Host, p: u32, entries: u32, n: u32) -> u32 {
    if n == 0 {
        return 0;
    }
    if m.byte(p + 20) == 1 {
        fn_30098c(m, h, m.word(p + 36), p, entries, n)
    } else {
        fn_300974(m, h, m.word(p + 36), p, entries, n)
    }
}
pub fn fn_2e4098(m: &mut Memory<'_>, h: &mut impl Host, p: u32, out: u32, n: u32) -> u32 {
    m.put_word(out, 0);
    m.put_word(n, 0);
    if m.byte(p + 20) == 1 {
        fn_300714(m, h, p, out, n)
    } else {
        fn_300594(m, h, m.word(p + 36), p, out, n)
    }
}
pub fn fn_3010e8(m: &mut Memory<'_>, h: &mut impl Host, heap: u32, input: u32, out: u32) -> u32 {
    m.put_word(out, 0);
    if m.word(input) & 0xffffff != 6 || m.word(input + 8) != 0 {
        return 3;
    }

    let mut err = 0;
    let p = zero_object(m, h, heap, 44, &mut err);
    if err != 0 {
        return err as u32;
    }
    let at = m.offset(p + 20);
    m.bytes[at] = 1;
    m.put_word(p + 36, heap);
    m.put_word(p + 12, m.word(input + 12));
    m.put_word(p + 16, m.word(input + 16));
    let mut s = input + 24;
    let n = strlen(m, s) + 1;
    m.put_word(p, if n != 1 { s } else { 0 });
    s += n;
    let n = strlen(m, s) + 1;
    m.put_word(p + 4, if n != 1 { s } else { 0 });
    let tab = (s + n + 3) & !3;
    let count = m.word(input + 20);
    let end = tab + count * 4;
    if count != 0 {
        let q = zero_object(m, h, heap, 36, &mut err);
        m.put_word(p + 40, q);
        if err != 0 {
            return err as u32;
        }
        let mut cur = q;
        for i in 0..count {
            let record = end + m.word(tab + i * 4);
            m.put_word(cur, record);
            let mut s = record + 48;
            for (field, size) in [(20, 0), (24, 129), (28, 0)] {
                if m.word(m.word(cur) + field) != 0 {
                    m.put_word(cur + field - 16, s);
                    s += if size == 0 { strlen(m, s) + 1 } else { size };
                } else {
                    m.put_word(cur + field - 16, 0);
                }
            }
            let a = (s + 3) & !3;
            m.put_word(cur + 16, a);
            let c = m.word(m.word(cur) + 32);
            m.put_word(cur + 20, a + 4 + c * 4);
            let a = (m.word(cur + 20) + m.word(a + c * 4) + 3) & !3;
            m.put_word(cur + 24, a);
            m.put_word(cur + 28, a + 4 + c * 4);
            if i + 1 < count {
                let next = zero_object(m, h, heap, 36, &mut err);
                m.put_word(cur + 32, next);
                if err != 0 {
                    return err as u32;
                }
            }

            let c = m.word(m.word(cur) + 32);
            let mut j = 0i32;
            while j < (c as i32) {
                let arr = m.word(cur + 24);
                let a = m.word(arr + j as u32 * 4) as i32;
                let b = (m.word(arr + (j as u32 + 1) * 4) as i32).wrapping_abs() as u32;
                if a < 0 {
                    let mut q = (m.word(cur + 28).wrapping_sub(a as u32) + 1) & !1;
                    let end = m.word(cur + 28).wrapping_add(b);
                    loop {
                        let v = half(m, q) as u32;
                        q = (q + 2 + v + 1) & !1;
                        if q + 2 >= end {
                            break;
                        }
                    }
                }
                j += 1;
            }
            if i + 1 < count {
                cur = m.word(cur + 32);
            }
        }
    }
    m.put_word(out, p);
    err as u32
}
fn zero_object(m: &mut Memory<'_>, h: &mut impl Host, heap: u32, n: u32, err: &mut u8) -> u32 {

    crate::boot_port_1::fn_1b20dc_flag(m, heap, 1, n, err, &mut Platform(h))
}

pub fn fn_3014fc(m: &mut Memory<'_>, h: &mut impl Host, alloc: u32, arg: u32, out: u32) -> u32 {
    let mut err = 0u8;
    let o = zero_object(m, h, alloc, 0x30, &mut err);
    if err != 0 {
        return err as u32;
    }
    let at = m.offset(o + 20);
    m.bytes[at] = 0;
    for off in [0, 4, 12, 16, 40, 44] {
        m.put_word(o + off, 0);
    }
    m.put_word(o + 36, alloc);
    if arg != 0 {
        err = fn_2fea84(m, h, o, arg) as u8;
        if err != 0 {
            return err as u32;
        }
    }
    m.put_word(out, o);
    err as u32
}

pub fn fn_2e3ac0(
    m: &mut Memory<'_>,
    h: &mut impl Host,
    heap: u32,
    mut image: u32,
    mode: u32,
    out: u32,
    encrypted: &[u8; 4],
    plain: &[u8; 4],
) -> u32 {
    let mut wrapped = 0;
    if image != 0 {
        let mut cursor = m.offset(image);
        let result = crate::phonemes::unwrap_dictionary(
            m.bytes,
            &mut cursor,
            &mut wrapped,
            crate::phonemes::DictionarySignatures { encrypted, plain },
        )
        .expect("valid mapped dictionary");
        if result != 0 {
            return result as u32;
        }
        image = m.base + cursor as u32;
    }
    if image == 0 && mode == 1 {
        return 3;
    }
    let result = if mode == 1 {
        fn_3010e8(m, h, heap, image, out)
    } else {
        fn_3014fc(m, h, heap, image, out)
    };
    if result == 0 {
        let p = m.word(out);
        m.put_word(p + 28, 0);
        m.put_word(p + 24, 0);
        let at = m.offset(p + 32);
        m.bytes[at] = wrapped;
    }
    result
}
fn window_index(
    m: &mut Memory<'_>,
    kind: u32,
    index: u32,
    width: u32,
    out: u32,
    tables: &[u32; 7],
) -> i32 {
    let mut selected = None;
    let rc = crate::selection::window_table_index(kind as i32, index, width as i32, &mut selected);
    if let Some((table, index)) = selected {
        m.put_word(
            out,
            tables[table] + index as u32 * if table == 0 { 12 } else { 16 },
        );
    }
    rc
}
pub fn fn_2dc798(
    m: &mut Memory<'_>,
    h: &mut impl Host,
    p: u32,
    arg: u32,
    a2: u32,
    a3: u32,
    tables: &[u32; 7],
) -> u32 {
    if p == 0 || m.word(p + 0x60) == 0 {
        return u32::MAX;
    }
    m.put_word(p + 0x1b0, arg);
    let tab = match m.word(p + 4) {
        12000 => 0x5eb852,
        8000 => 0x4f5c29,
        10000 => 0x59999a,
        11025 => 0x5c28f6,
        16000 => 0x6b851f,
        22050 => 0x70a3d7,
        _ => 0,
    };
    m.put_word(p + 0x1b0, tab);
    let rem = 0x1000000u32
        .wrapping_sub(crate::selection::rounded_product(tab as i32, tab as i32, 24) as u32);
    m.put_word(p + 0x1b4, rem);
    m.put_half(p + 0x1b8, ((tab as i32) >> 10) as u16);
    m.put_half(p + 0x1ba, ((rem as i32) >> 10) as u16);
    if tab == 0 {
        return u32::MAX;
    }
    if fn_2db9b0(
        m,
        h,
        m.word(m.word(p) + 4),
        p + 0x50,
        m.word(p + 0x60).wrapping_add(1),
    ) != 0
    {
        return u32::MAX;
    }
    if (m.word(p + 0x60) as i32) < 4 {
        return u32::MAX;
    }
    let at = m.offset(p + 0x1bd);
    m.bytes[at] = a2 as u8;
    let at = m.offset(p + 0x1bc);
    m.bytes[at] = (a2 >> 8) as u8;
    m.put_half(p + 0x1d2, ((a2 & 255) * 2 + 2) as u16);
    if window_index(m, (a2 >> 16) & 255, a2 & 255, a3 & 255, p + 0x1ac, tables) != 0 {
        return u32::MAX;
    }
    if window_index(
        m,
        a2 >> 24,
        m.byte(p + 0x1bc) as u32,
        (a3 >> 8) & 255,
        p + 0x1a8,
        tables,
    ) != 0
    {
        return u32::MAX;
    }
    let n = (m.byte(p + 0x1bc) as u32)
        .wrapping_mul(m.word(p + 0x60).wrapping_add(2))
        .wrapping_add(m.byte(p + 0x1bc) as u32)
        .wrapping_add(m.byte(p + 0x1bd) as u32 * 2 + 3);
    if fn_2dba38(m, h, m.word(m.word(p) + 4), p + 0x188, n) != 0 {
        return u32::MAX;
    }
    if fn_2dba38(
        m,
        h,
        m.word(m.word(p) + 4),
        p + 0x198,
        m.word(p + 0x60).wrapping_add(1),
    ) != 0
    {
        return u32::MAX;
    }
    let mut j = m.byte(p + 0x1bc) as i32;
    while j >= -1 {
        m.put_half(
            (p + 0x1c0).wrapping_add((j as u32).wrapping_mul(2)),
            (j as u32).wrapping_mul(m.word(p + 0x60).wrapping_add(2)) as u16,
        );
        j -= 1;
    }
    0
}

struct Pool<'a, 'b, H> {
    m: &'a mut Memory<'b>,
    h: &'a mut H,
}
impl<H: Host> crate::selection::PoolHost for Pool<'_, '_, H> {
    fn block(&self, i: usize) -> crate::selection::PoolBlock {
        let p = i as u32;
        let n = self.m.word(p);
        crate::selection::PoolBlock {
            next: if n == 0 { None } else { Some(n as usize) },
            base: self.m.word(p + 4),
            current: self.m.word(p + 8),
            end: self.m.word(p + 12),
        }
    }
    fn set_block(&mut self, i: usize, b: crate::selection::PoolBlock) {
        let p = i as u32;
        self.m.put_word(p, b.next.unwrap_or(0) as u32);
        self.m.put_word(p + 4, b.base);
        self.m.put_word(p + 8, b.current);
        self.m.put_word(p + 12, b.end);
    }
    fn allocate(&mut self, o: u32, n: u32) -> Option<(usize, u32)> {
        let p = calloc(self.m, self.h, self.m.word(o + 4), 1, n);
        if p == 0 {
            None
        } else {
            Some((p as usize, p))
        }
    }
    fn free(&mut self, o: u32, i: usize) {
        free(self.m, self.h, self.m.word(o + 4), i as u32);
    }
    fn clear(&mut self, p: u32, n: u32) {
        let at = self.m.offset(p);
        crate::fe_text::memset_195bec(self.m.bytes, at as u32, 0, n);
    }
    fn report(&mut self, _: &crate::selection::PrivateHeap<'_>, _: &[u8], _: u32) {
        unreachable!()
    }
}
fn pool_alloc(m: &mut Memory<'_>, h: &mut impl Host, p: u32, n: u32) -> u32 {
    let first = m.word(p + 8);
    let mut pool = crate::selection::PrivateHeap {
        owner: m.word(p),
        name: &[],
        first_block: if first == 0 {
            None
        } else {
            Some(first as usize)
        },
        expected: m.word(p + 12),
        pool: m.word(p + 16),
        allocated: m.word(p + 20),
    };
    let result = pool
        .allocate(n, &mut Pool { m, h })
        .map_or(0, |a| a.address);
    m.put_word(p + 8, pool.first_block.unwrap_or(0) as u32);
    m.put_word(p + 16, pool.pool);
    m.put_word(p + 20, pool.allocated);
    result
}
impl<H: Host> crate::boot_part_b::Host for Platform<'_, H> {
    fn format_address_m(&mut self,m:&Memory<'_>,format:&[u8])->Option<u32> {self.0.format_address_m(m,format)}
    fn real_log_args(&mut self)->bool {self.0.real_log_args()}
    fn log_variadic(&mut self,m:&mut Memory<'_>,entry:u32,a:[u32;4],format:&[u8],varargs:&[u32])->bool {self.0.log_variadic(m,entry,a,format,varargs)}

    fn call(&mut self, m: &mut Memory<'_>, f: u32, a: &[u32]) -> u32 {
        self.0.call(m, f, a)
    }
    fn thread_id(&mut self) -> u32 {
        self.0.thread_id()
    }
}
impl<H: Host> crate::boot_part_e::Host for Platform<'_, H> {
    fn call(&mut self, m: &mut Memory<'_>, f: u32, a: &[u32]) -> u32 {
        self.0.call(m, f, a)
    }
}
pub struct StreamRuntime<'a> {
    pub unmap: &'a crate::boot_part_e::UnmapEnvironment,
    pub scratch: u32,
}
fn cache_read(
    m: &mut Memory<'_>,
    h: &mut impl Host,
    p: u32,
    off: u32,
    n: u32,
    out: Option<u32>,
    r: &StreamRuntime<'_>,
) -> u32 {
    if let Some(out) = out {
        m.put_word(out, 0);
    }
    let data = m.word(p + 12);
    if data != 0 {
        if m.word(p + 8) <= off && off.wrapping_add(n) <= m.word(p + 16).wrapping_add(m.word(p + 8))
        {

            let at = m.offset(data);
            let len = m.word(p + 16) as usize;
            let mut cache = crate::selection::BlockCache {
                data: Some(&m.bytes[at..at + len]),
                position: m.word(p + 8),
                length: len as u32,
            };
            let _ = cache
                .read(off, n, |_| unreachable!(), |_, _| unreachable!())
                .expect("mapped cache window");
            if let Some(out) = out {
                m.put_word(out, data + off - m.word(p + 8));
            }
            return 0;
        }
        let rc = crate::boot_part_e::fn_179208(
            m,
            m.word(p),
            m.word(p + 4),
            data,
            r.unmap,
            &mut Platform(h),
        );
        m.put_word(p + 8, 0);
        m.put_word(p + 12, 0);
        m.put_word(p + 16, 0);
        if rc >= 0x80000000 {
            return rc;
        }
    }
    let rc = crate::leftovers_1::fn_17a1a4(
        m,
        m.word(p),
        m.word(p + 4),
        off,
        n,
        p + 12,
        r.scratch,
        r.unmap,
        &mut Platform(h),
    );
    if rc >= 0x80000000 {
        return rc;
    }
    m.put_word(p + 8, off);
    m.put_word(p + 16, n);
    if let Some(out) = out {
        m.put_word(out, m.word(p + 12));
    }
    rc
}
fn stream_read(
    m: &mut Memory<'_>,
    h: &mut impl Host,
    st: u32,
    size: u32,
    count: u32,
    dst: &mut [u8],
    r: &StreamRuntime<'_>,
) -> u32 {
    let kind = m.byte(st);
    let mut pos = m.word(st + 12);
    let limit = m.word(st + 8);
    let align = match size {
        1 => 1,
        2 => 2,
        3..=7 => 4,
        8 => 8,
        _ => 4,
    };
    crate::selection::align_feature_cursor(&mut pos, limit, align).unwrap();
    m.put_word(st + 12, pos);
    let count = if size.wrapping_mul(count).wrapping_add(pos) > limit {
        limit.wrapping_sub(pos) / size
    } else {
        count
    };
    if count == 0 {
        return 0;
    }
    if kind == 2 {
        let mut n = size.wrapping_mul(count);
        let cache = m.word(st + 28);
        if pos == 0
            && n <= limit
            && (m.word(cache + 12) == 0 || m.word(cache + 8) != m.word(st + 4))
        {
            n = limit;
        }
        let rc = cache_read(
            m,
            h,
            cache,
            m.word(st + 4).wrapping_add(pos),
            n,
            Some(st + 20),
            r,
        );
        if rc >= 0x80000000 {
            return 0;
        }
        m.put_word(st + 16, m.word(st + 4).wrapping_add(pos));
        m.put_word(st + 24, n);
    }
    let window = m.offset(m.word(st + 20));
    let mut state = crate::selection::WindowState {
        kind: 0,
        base: m.word(st + 4),
        limit,
        pos: m.word(st + 12),
        origin: m.word(st + 16),
        window: Some(&m.bytes[window..]),
        lastlen: m.word(st + 24),
    };
    let mut cache = crate::selection::BlockCache {
        data: None,
        position: 0,
        length: 0,
    };
    let count = crate::selection::read_window(
        dst,
        size,
        count,
        &mut state,
        &mut cache,
        |_| unreachable!(),
        |_, _| unreachable!(),
    )
    .expect("mapped stream");
    m.put_word(st + 12, state.pos);
    count
}
fn read16(m: &mut Memory<'_>, h: &mut impl Host, st: u32, v: &mut u16, r: &StreamRuntime<'_>) {
    let mut bytes = v.to_le_bytes();
    stream_read(m, h, st, 2, 1, &mut bytes, r);
    *v = u16::from_le_bytes(bytes);
}
fn read_to(
    m: &mut Memory<'_>,
    h: &mut impl Host,
    st: u32,
    size: u32,
    n: u32,
    dst: u32,
    r: &StreamRuntime<'_>,
) -> u32 {
    let mut buffer = [0u8; 624];
    let len = (size * n) as usize;
    assert!(len <= buffer.len());
    let count = stream_read(m, h, st, size, n, &mut buffer[..len], r);
    if count != 0 {
        let bytes = size.wrapping_mul(count);
        let start = m.word(st + 12).wrapping_sub(bytes);

        m.put_word(st + 12, start);
        let src = m
            .word(st + 20)
            .wrapping_add(m.word(st + 4))
            .wrapping_add(start)
            .wrapping_sub(m.word(st + 16));
        copy(m, dst, src, bytes);
        m.put_word(st + 12, m.word(st + 12).wrapping_add(bytes));
    }
    count
}
fn align_stream(m: &mut Memory<'_>, st: u32) {
    let mut pos = m.word(st + 12);
    crate::selection::align_feature_cursor(&mut pos, m.word(st + 8), 4).unwrap();
    m.put_word(st + 12, pos);
}
fn arm_lsl(v: u32, n: u32) -> u32 {
    if n & 255 >= 32 {
        0
    } else {
        v << (n & 255)
    }
}
fn arm_lsr(v: u32, n: u32) -> u32 {
    if n & 255 >= 32 {
        0
    } else {
        v >> (n & 255)
    }
}
fn arm_asr(v: i32, n: u32) -> i32 {
    v >> ((n & 255).min(31))
}
pub fn fn_2f7c44(
    m: &mut Memory<'_>,
    h: &mut impl Host,
    ctx: u32,
    out: u32,
    st: u32,
    base: u32,
    k: &Literals,
    r: &StreamRuntime<'_>,
) -> u32 {
    let mut rc = 0;
    if m.word(st + 28) != 0 {
        rc = cache_read(m, h, m.word(st + 28), m.word(st + 4), 256, None, r);
        if rc >= 0x80000000 {
            return rc;
        }
    }
    let mut tmp = 0;
    read16(m, h, st, &mut tmp, r);
    if tmp != 1 {
        diagnostic(m, h, ctx, k, 0, 0, &[]);
        return 0x84e02004;
    }
    read16(m, h, st, &mut tmp, r);
    read16(m, h, st, &mut tmp, r);
    m.put_word(out + 4, tmp as u32);
    if (tmp as u32).wrapping_sub(1) >= 2 {
        diagnostic(m, h, ctx, k, 1, 0, &[]);
        return 0x84e02004;
    }
    read16(m, h, st, &mut tmp, r);
    m.put_word(out + 8, tmp as u32);
    if tmp == 0 || tmp > 7 {
        diagnostic(m, h, ctx, k, 2, 0, &[]);
        return 0x84e02004;
    }
    align_stream(m, st);
    let mut i = 0;
    while i < (m.word(out + 8) << 1) {
        read_to(m, h, st, 2, 1, out + 74 + i * 2, r);
        read_to(m, h, st, 2, 1, out + 102 + i * 2, r);
        i += 1;
    }
    let mut x = 0;
    read16(m, h, st, &mut x, r);
    align_stream(m, st);
    i = 0;
    while i < m.word(out + 8) << 1 {
        let mut v = 0;
        read16(m, h, st, &mut v, r);
        let v = if x as u32 > base {
            arm_lsr(v as u32, x as u32 - base)
        } else {
            arm_lsl(v as u32, base - x as u32)
        };
        m.put_word(out + 16 + i * 4, v);
        i += 1;
    }
    m.put_half(out + 72, base as u16);
    read16(m, h, st, &mut tmp, r);
    m.put_word(out + 12, tmp as u32);
    align_stream(m, st);
    if m.byte(st) == 0 {
        m.put_word(out + 132, m.word(st + 20));
        m.put_word(out + 136, m.word(st + 12));
    } else {
        m.put_word(out + 132, 0);
        m.put_word(out + 136, m.word(st + 4).wrapping_add(m.word(st + 12)));
    }
    m.put_word(
        st + 12,
        m.word(out + 12)
            .wrapping_mul(m.word(out + 4).wrapping_mul(m.word(out + 8)) << 1)
            .wrapping_add(m.word(st + 12)),
    );
    rc
}
pub fn fn_2f80b0(
    m: &mut Memory<'_>,
    h: &mut impl Host,
    ctx: u32,
    out: u32,
    count: u32,
    st: u32,
    base: u32,
    limit: i32,
    k: &Literals,
    r: &StreamRuntime<'_>,
) -> u32 {
    let mut rc = 0;
    if m.word(st + 28) != 0 {
        rc = cache_read(m, h, m.word(st + 28), m.word(st + 4), 256, None, r);
        if rc >= 0x80000000 {
            return rc;
        }
    }
    m.put_word(out + 8, count);
    let mut tmp = 0;
    read16(m, h, st, &mut tmp, r);
    if tmp != 1 {
        diagnostic(m, h, ctx, k, 0, 0, &[]);
        return 0x84e02004;
    }
    read16(m, h, st, &mut tmp, r);
    read16(m, h, st, &mut tmp, r);
    m.put_word(out + 12, tmp as u32);
    if tmp == 0 || tmp > 3 {
        diagnostic(m, h, ctx, k, 1, 0, &[]);
        return 0x84e02004;
    }
    read16(m, h, st, &mut tmp, r);
    m.put_word(out + 4, tmp as u32);
    if (tmp as u32).wrapping_sub(1) >= 2 {
        diagnostic(m, h, ctx, k, 2, 0, &[]);
        return 0x84e02004;
    }
    align_stream(m, st);
    read_to(m, h, st, 2, count, out + 18, r);
    align_stream(m, st);
    let mut sl = 0;
    while sl < (m.word(out + 12) as i32) {
        for j in 0..3 {
            read_to(m, h, st, 2, 1, out + 32 + (sl as u32 * 3 + j) * 2, r);
            read_to(m, h, st, 2, 1, out + 50 + (sl as u32 * 3 + j) * 2, r);
        }
        sl += 1;
    }
    let mut x = 0;
    read16(m, h, st, &mut x, r);
    align_stream(m, st);
    sl = 0;
    while sl < (m.word(out + 12) as i32) {
        for j in 0..3 {
            let mut v = 0;
            read16(m, h, st, &mut v, r);
            let v = if x as u32 > base {
                arm_asr(v as i16 as i32, x as u32 - base)
            } else {
                arm_lsl(v as i16 as i32 as u32, base - x as u32) as i32
            };
            m.put_word(out + 68 + (sl as u32 * 3 + j) * 4, v as u32);
        }
        sl += 1;
    }
    m.put_half(out + 104, base as u16);
    let v = m.word(out + 68) as i32;
    let sel = if v < limit {
        diagnostic(m, h, ctx, k, 3, 3, &[v as u32, base, limit as u32]);
        limit
    } else {
        (v >> 2).wrapping_mul(13) >> 2
    };
    m.put_word(out + 108, sel as u32);
    m.put_word(out + 112, if m.byte(st) == 0 { m.word(st + 20) } else { 0 });
    let table = pool_alloc(m, h, m.word(out), count.wrapping_shl(2));
    m.put_word(out + 116, table);
    if table == 0 {
        return 0x84e0200a;
    }
    let t = m
        .word(out + 4)
        .wrapping_mul(m.word(out + 12))
        .wrapping_mul(3) as u16 as u32;
    let pad = 4u32.wrapping_sub(t) & 3;
    m.put_half(out + 16, (t + pad) as u16);
    let mut j = 0i32;
    while j < (m.word(out + 8) as i32) {
        align_stream(m, st);
        let pos = m.word(st + 12);
        m.put_word(
            m.word(out + 116) + j as u32 * 4,
            if m.byte(st) == 0 {
                pos
            } else {
                m.word(st + 4).wrapping_add(pos)
            },
        );
        m.put_word(
            st + 12,
            (half(m, out + 18 + j as u32 * 2) as u32)
                .wrapping_mul(half(m, out + 16) as u32)
                .wrapping_sub(pad)
                .wrapping_add(m.word(st + 12)),
        );
        j += 1;
    }
    rc
}

pub fn fn_2f7760(
    m: &mut Memory<'_>,
    h: &mut impl Host,
    ctx: u32,
    out: u32,
    st: u32,
    base: u32,
    k: &Literals,
    r: &StreamRuntime<'_>,
) -> u32 {
    let mut fmt = 0;
    read16(m, h, st, &mut fmt, r);
    if fmt != 1 {
        diagnostic(m, h, ctx, k, 0, 0, &[]);
        return 0x84e02004;
    }
    read16(m, h, st, &mut fmt, r);
    let mut count = 0;
    read16(m, h, st, &mut count, r);
    if count > 150 {
        diagnostic(m, h, ctx, k, 1, 0, &[]);
        return 0x84e02004;
    }
    align_stream(m, st);
    let mut skip = [0u8; 624];
    stream_read(m, h, st, 4, count as u32, &mut skip, r);
    let books = pool_alloc(m, h, m.word(out), count as u32 * 112);
    m.put_word(out + 8, books);
    if books == 0 {
        return 0x84e0200a;
    }
    m.put_half(out + 6, count);
    m.put_half(out + 4, 0);
    for i in 0..count as u32 {
        let bk = m.word(out + 8) + i * 112;
        let arena = m.word(out);
        let mut b = base;
        read_to(m, h, st, 2, 1, bk, r);
        if half(m, bk) > 12 {
            diagnostic(m, h, ctx, k, 2, 0, &[]);
            return 0x84e02004;
        }
        read_to(m, h, st, 2, 1, bk + 2, r);
        read_to(m, h, st, 2, 1, bk + 4, r);
        if (half(m, bk + 4) as u32).wrapping_sub(1) >= 2 {
            diagnostic(m, h, ctx, k, 3, 0, &[]);
            return 0x84e02004;
        }
        align_stream(m, st);
        let mut j = 0;
        while j < half(m, bk) as u32 {
            read_to(m, h, st, 2, 1, bk + 8 + j * 2, r);
            read_to(m, h, st, 2, 1, bk + 32 + j * 2, r);
            j += 1;
        }
        let mut x = 0;
        read16(m, h, st, &mut x, r);
        align_stream(m, st);
        j = 0;
        while j < half(m, bk) as u32 {
            let mut v = 0;
            read16(m, h, st, &mut v, r);
            let v = if x as u32 > b {
                v as i16 as i32 as u32
            } else {
                if (x as u32 + 16) < b {
                    b = (x as u32 + 16) as u16 as u32;
                }
                arm_lsl(v as i16 as i32 as u32, b - x as u32)
            };
            m.put_word(bk + 60 + j * 4, v);
            j += 1;
        }
        m.put_half(bk + 56, if x as u32 <= b { b as u16 } else { x });
        align_stream(m, st);
        let n = half(m, bk) as u32 * half(m, bk + 2) as u32;
        if read_block(m, h, arena, bk + 108, half(m, bk + 4) as u32, n, st, r) < n {
            return 0x84e0200a;
        }
    }
    m.put_half(out + 4, count);
    0
}
pub fn fn_2f6a50(
    m: &mut Memory<'_>,
    h: &mut impl Host,
    ctx: u32,
    out: u32,
    st: u32,
    k: &Literals,
    r: &StreamRuntime<'_>,
) -> u32 {
    let mut err = 0;
    if m.word(st + 28) != 0 {
        err = cache_read(m, h, m.word(st + 28), m.word(st + 4), 512, None, r);
        if err >= 0x80000000 {
            return err;
        }
    }
    let mut t = 0;
    read16(m, h, st, &mut t, r);
    if t != 1 {
        diagnostic(m, h, ctx, k, 0, 0, &[]);
        return 0x84e02004;
    }
    read16(m, h, st, &mut t, r);
    read16(m, h, st, &mut t, r);
    m.put_half(out + 10, t);
    read16(m, h, st, &mut t, r);
    let n2 = t.wrapping_mul(2);
    m.put_half(out + 8, t);
    read16(m, h, st, &mut t, r);
    m.put_half(out + 14, t);
    let at = m.offset(out + 18);
    m.bytes[at] = 2;
    m.put_half(out + 16, t.wrapping_add(1));
    read16(m, h, st, &mut t, r);
    if t != 1 {
        return 0x84e02004;
    }
    read16(m, h, st, &mut t, r);
    m.put_half(out + 6, t);
    read16(m, h, st, &mut t, r);
    m.put_half(out + 4, t);
    align_stream(m, st);
    let subs = pool_alloc(m, h, m.word(out), n2 as u32 * 200);
    m.put_word(out + 164, subs);
    if subs == 0 {
        return 0x84e0200a;
    }
    for i in 0..n2 as u32 {
        let e = m.word(out + 164) + i * 200;
        err = 0;
        read_to(m, h, st, 2, 1, e + 2, r);
        let count = half(m, e + 2) as u32;
        if count == 0 || count > 24 {
            diagnostic(m, h, ctx, k, 1, 0, &[]);
            err = 0x84e02004;
        } else {
            align_stream(m, st);
            let mut j = 0;
            while j < half(m, e + 2) as u32 {
                read_to(m, h, st, 2, 1, e + 102 + j * 2, r);
                read_to(m, h, st, 2, 1, e + 150 + j * 2, r);
                j += 1;
            }
            read_to(m, h, st, 2, 1, e + 100, r);
            align_stream(m, st);
            read_to(m, h, st, 4, half(m, e + 2) as u32, e + 4, r);
        }
        if err >= 0x80000000 {
            return err;
        }
    }
    read16(m, h, st, &mut t, r);
    m.put_half(out + 12, t);
    if t > 24 {
        diagnostic(m, h, ctx, k, 2, 0, &[]);
        return 0x84e02004;
    }
    align_stream(m, st);
    read_to(m, h, st, 2, half(m, out + 12) as u32, out + 20, r);
    let mut j = 0u32;
    for i in 1..half(m, out + 14) as u32 + 1 {
        if j + 1 < half(m, out + 12) as u32 && i > half(m, out + 22 + j * 2) as u32 {
            j += 1;
        }
        m.put_word(out + 64 + i * 4, j);
    }
    align_stream(m, st);
    read_block(
        m,
        h,
        m.word(out),
        out + 168,
        2,
        half(m, out + 10) as u32,
        st,
        r,
    );
    m.put_word(out + 168, m.word(out + 168).wrapping_sub(4));
    let fac = pool_alloc(m, h, m.word(out), half(m, out + 10) as u32 * 8);
    m.put_word(out + 172, fac);
    if fac == 0 {
        return 0x84e0200a;
    }
    m.put_word(out + 172, m.word(out + 172).wrapping_sub(16));
    align_stream(m, st);
    let mut i = 2u32;
    while i < half(m, out + 10) as u32 + 2 {
        align_stream(m, st);
        let fac = m.word(out + 172);
        if m.byte(st) == 0 {
            m.put_word(fac + i * 8, m.word(st + 20));
            m.put_word(fac + i * 8 + 4, m.word(st + 12));
        } else {
            m.put_word(fac + i * 8, 0);
            m.put_word(
                fac + i * 8 + 4,
                m.word(st + 4).wrapping_add(m.word(st + 12)),
            );
        }
        let stride =
            half(m, out + 6) as u32 + half(m, out + 4) as u32 * half(m, out + 8) as u32 * 2;
        m.put_word(
            st + 12,
            m.word(st + 12)
                .wrapping_add(half(m, m.word(out + 168) + i * 2) as u32 * stride),
        );
        i += 1;
    }
    err
}
pub fn fn_2da04c(
    m: &mut Memory<'_>,
    h: &mut impl Host,
    ctx: u32,
    out: u32,
    mode: u32,
    files: u32,
    count: u32,
    k: &Literals,
    r: &StreamRuntime<'_>,
) -> u32 {
    m.put_word(out + 0x28, out);
    let rc = fn_2f7c44(m, h, ctx, out + 0x28, files + 64, 15, k, r);
    if rc != 0 {
        return rc;
    }
    m.put_word(out + 0x18, m.word(out + 0x30));
    m.put_word(out + 0x1a4, out);
    let rc = fn_2f6a50(m, h, ctx, out + 0x1a4, files, k, r);
    if rc != 0 {
        return rc;
    }
    if half(m, out + 0x1a8) < 1 || half(m, out + 0x1a8) > 2 {
        diagnostic(m, h, ctx, k, 4, 0, &[]);
        return 0x84e02004;
    }
    if m.byte(mode + 0x114) == 3 && half(m, out + 0x1b4) <= half(m, out + 0x1b2) {
        diagnostic(m, h, ctx, k, 5, 1, &[]);
    }
    m.put_word(
        out + 0x24,
        (half(m, out + 0x1b2) as u32 + 1) * half(m, out + 0x1ac) as u32,
    );
    m.put_word(out + 0x254, out);
    let rc = fn_2f7760(m, h, ctx, out + 0x254, count, 16, k, r);
    if rc != 0 {
        return rc;
    }
    if m.byte(mode + 0x45) != 0 {
        diagnostic(m, h, ctx, k, 6, 0, &[]);
        return 0x84e02004;
    }
    if m.byte(mode + 0x46) != 0 {
        m.put_word(out + 0x12c, out);
        let rc = fn_2f80b0(
            m,
            h,
            ctx,
            out + 0x12c,
            m.word(out + 0x18),
            files + 96,
            18,
            0x140000,
            k,
            r,
        );
        if rc != 0 {
            diagnostic(m, h, ctx, k, 7, 0, &[]);
            return rc;
        }
        m.put_word(out + 0x20, m.word(out + 0x138));
    }
    m.put_word(out + 0xb4, out);
    let rc = fn_2f80b0(
        m,
        h,
        ctx,
        out + 0xb4,
        m.word(out + 0x18),
        files + 32,
        20,
        0x3999a0,
        k,
        r,
    );
    if rc != 0 {
        diagnostic(m, h, ctx, k, 8, 0, &[]);
    } else {
        m.put_word(out + 0x1c, m.word(out + 0xc0));
    }
    rc
}

fn decode_delta(m: &mut Memory<'_>, p: u32, dim: i16) {
    let n = ((dim as u16).wrapping_shl(1) >> 1) as i32;
    let odd = if (dim as i32).wrapping_shl(17) == 0 {
        0
    } else {
        dim as i32 & 1
    };
    let at = |i: i32| p.wrapping_add((i as u32).wrapping_mul(2));
    if odd == 1 {
        let last = n - 1;
        let prev = if last > 0 { half(m, at(last - 1)) } else { 0 };
        m.put_half(at(last), half(m, at(last)) ^ prev);
    }
    let mut k = n - odd - 1;
    while k >= 0 {
        let prev = if k > 0 { half(m, at(k - 1)) } else { 0 };
        m.put_half(at(k), half(m, at(k)) ^ prev);
        let prev = if k - 1 > 0 { half(m, at(k - 2)) } else { 0 };
        m.put_half(at(k - 1), half(m, at(k - 1)) ^ prev);
        k -= 2;
    }
}

pub fn fn_2dd148<H: Host + crate::render_part_a::ReaderHost>(
    m: &mut Memory<'_>,
    h: &mut H,
    p: u32,
    reader: u32,
    frame: u32,
    k: &crate::render_part_a::ReaderRuntime,
) -> u32 {
    if p == 0 || reader == 0 {
        return u32::MAX;
    }
    let at = m.offset(frame);
    let mut saved = [0u8; 128];
    saved.copy_from_slice(&m.bytes[at..at + 128]);
    m.bytes[at..at + 128].fill(0);
    let at_runtime = m.offset(k.scratch);
    let mut saved_runtime = [0u8; 512];
    saved_runtime.copy_from_slice(&m.bytes[at_runtime..at_runtime + 512]);
    let rc = read_extra(m, h, p, reader, frame, k);
    m.bytes[at_runtime..at_runtime + 512].copy_from_slice(&saved_runtime);
    let at = m.offset(frame);
    m.bytes[at..at + 128].copy_from_slice(&saved);
    rc
}
fn read_extra<H: Host + crate::render_part_a::ReaderHost>(
    m: &mut Memory<'_>,
    h: &mut H,
    p: u32,
    reader: u32,
    s: u32,
    k: &crate::render_part_a::ReaderRuntime,
) -> u32 {
    let tag = s;
    let size = s + 8;
    let entry = s + 12;
    let buf = s + 16;
    let n = s + 80;
    if crate::render_part_a::open_reader_chunk(m, h, reader, tag, size, entry, k) >= 0x80000000 {
        return u32::MAX;
    }
    if crate::fe_text::strcmp_lit(m.bytes, m.offset(tag) as u32, b"EXHD") != 0 {
        return u32::MAX;
    }
    m.put_word(n, 64);
    let data = m.word(entry);
    let length = m.word(size);
    let rc = crate::render_part_a::read_chunk_string(
        &mut crate::selection::arena::Arena::new(m.base, m.bytes),
        reader,
        data,
        length,
        0,
        buf,
        n,
    );
    if rc >= 0x80000000 || m.word(n).wrapping_add(16) < m.word(size) {
        return u32::MAX;
    }
    let mut src = m.word(entry).wrapping_add(m.word(n));
    copy(m, p + 0x150, src, 4);
    src += 4;
    for off in [0x158, 0x15a, 0x15c, 0x15e, 0x160, 0x162] {
        copy(m, p + off, src, 2);
        src += 2;
    }
    if crate::render_part_a::close_reader_chunk(m, h, reader, k) >= 0x80000000 {
        return u32::MAX;
    }
    if crate::render_part_a::open_reader_chunk(m, h, reader, tag, size, entry, k) >= 0x80000000 {
        return u32::MAX;
    }
    if crate::fe_text::strcmp_lit(m.bytes, m.offset(tag) as u32, b"EXDT") != 0 {
        return u32::MAX;
    }
    if half(m, p + 0x162) & 1 == 0 {
        let elems = m.word(p + 0x154);
        if m.word(elems) != 0 {
            free(m, h, 0, m.word(elems));
            free(m, h, 0, elems);
        }
        return u32::MAX;
    }
    let elems = calloc(m, h, m.word(m.word(p) + 4), m.word(p + 0x150), 16);
    m.put_word(p + 0x154, elems);
    if elems == 0 {
        return u32::MAX;
    }
    let dims = (half(m, p + 0x158) as i16 as i32).wrapping_add(half(m, p + 0x15c) as i16 as i32);
    let q = calloc(
        m,
        h,
        m.word(m.word(p) + 4),
        (dims as u32).wrapping_mul(m.word(p + 0x150)),
        2,
    );
    m.put_word(m.word(p + 0x154), q);
    if q == 0 {
        return u32::MAX;
    }
    let mut i = 1i32;
    while i < m.word(p + 0x150) as i32 {
        let elems = m.word(p + 0x154);
        let delta = (i as u32)
            .wrapping_mul(half(m, p + 0x158) as i16 as i32 as u32)
            .wrapping_mul(2);
        m.put_word(elems + i as u32 * 16, m.word(elems).wrapping_add(delta));
        i += 1;
    }
    i = 0;
    while i < m.word(p + 0x150) as i32 {
        let elems = m.word(p + 0x154);
        let last = elems.wrapping_add(m.word(p + 0x150).wrapping_sub(1).wrapping_mul(16));
        let delta = (half(m, p + 0x15c) as i16 as i32 as u32)
            .wrapping_mul(i as u32)
            .wrapping_mul(2)
            .wrapping_add((half(m, p + 0x158) as i16 as i32 as u32).wrapping_mul(2));
        m.put_word(elems + i as u32 * 16 + 4, m.word(last).wrapping_add(delta));
        i += 1;
    }
    src = m.word(entry);
    i = 0;
    while i < m.word(p + 0x150) as i32 {
        for (dim, off) in [(0x158, 0), (0x15c, 4)] {
            let elems = m.word(p + 0x154);
            let n = half(m, p + dim).wrapping_shl(1) as u32;
            copy(m, m.word(elems + i as u32 * 16 + off), src, n);
            src = src.wrapping_add(n >> 1 << 1);
            decode_delta(
                m,
                m.word(m.word(p + 0x154) + i as u32 * 16 + off),
                half(m, p + dim) as i16,
            );
        }
        let tmp = m.word(src);
        src += 4;
        m.put_word(m.word(p + 0x154) + i as u32 * 16 + 8, tmp);
        copy(m, m.word(p + 0x154) + i as u32 * 16 + 12, src, 2);
        src += 2;
        copy(m, m.word(p + 0x154) + i as u32 * 16 + 14, src, 2);
        src += 2;
        i += 1;
    }
    if crate::render_part_a::close_reader_chunk(m, h, reader, k) >= 0x80000000 {
        u32::MAX
    } else {
        0
    }
}

struct PoolBridge<'a, 'b, H> {
    h: &'a mut H,
    r: &'a StreamRuntime<'b>,
}
struct NoRegex;
impl crate::regex::PcreHost for NoRegex {
    fn validate(&mut self, _: u32, _: u32, _: u32, _: u32) -> u32 {
        unreachable!()
    }
    fn resolve_owner(&mut self, _: u32, _: i32) -> (u32, u32) {
        unreachable!()
    }
    fn heap_of(&mut self, _: u32) -> u32 {
        unreachable!()
    }
    fn alloc(&mut self, _: u32, _: u32, _: u32) -> Option<u32> {
        unreachable!()
    }
    fn free(&mut self, _: u32, _: u32) {
        unreachable!()
    }
}
impl<H: Host> crate::boot_port_5::Host for PoolBridge<'_, '_, H> {
    fn call(&mut self, m: &mut Memory<'_>, f: u32, a: &[u32]) -> u32 {
        self.h.call(m, f, a)
    }
}
impl<H: Host> crate::boot_part_f::Host for PoolBridge<'_, '_, H> {
    fn call(&mut self, m: &mut Memory<'_>, f: u32, a: &[u32]) -> u32 {
        if f == 0x17a1a4 {
            return crate::leftovers_1::fn_17a1a4(
                m,
                a[0],
                a[1],
                a[2],
                a[3],
                a[4],
                self.r.scratch,
                self.r.unmap,
                &mut Platform(self.h),
            );
        }
        self.h.call(m, f, a)
    }
    fn compare(
        &mut self,
        _: &mut Memory<'_>,
        _: u32,
        _: &mut crate::boot_part_f::Key,
        _: &mut u32,
    ) -> u32 {
        unreachable!()
    }
}
impl<H: Host> crate::log_live::LogMethodsM for PoolBridge<'_, '_, H> {
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
        self.h.call(m, f, &[o, c, l, a, fmt, va])
    }
}
impl<H: Host> crate::speech_part_5::Host for PoolBridge<'_, '_, H> {
    fn scratch(&self) -> u32 {
        self.r.scratch
    }
    fn format(&self, _: u32) -> u32 {
        unreachable!("unrelated speech formatter")
    }
    fn dictionary_entries(&mut self, _: &mut Memory<'_>, _: u32, _: &mut u32, _: &mut u32) -> u32 {
        unreachable!()
    }
    fn record(
        &mut self,
        _: &mut Memory<'_>,
        _: u32,
        _: u32,
        _: &mut crate::speech_part_5::RecordBuffer,
        _: &mut u8,
    ) -> u32 {
        unreachable!()
    }
    fn entries(
        &mut self,
        _: &mut Memory<'_>,
        _: u32,
        _: u32,
        _: u32,
        _: &mut u32,
        _: &mut u32,
    ) -> u32 {
        unreachable!()
    }
}
impl<H: Host> crate::speech_part_3::SpeechHost for PoolBridge<'_, '_, H> {}
impl<H: Host> crate::frontend::HeapVtableM for PoolBridge<'_, '_, H> {
    fn alloc_at_m(&mut self, _: u32, _: u32, _: &mut [u8]) -> u32 {
        unreachable!("2a9274 uses its live allocator wrapper")
    }
    fn realloc_at_m(&mut self, _: u32, _: u32, _: u32, _: &mut [u8]) -> u32 {
        unreachable!()
    }
    fn release_at_m(&mut self, _: u32, _: u32, _: &mut [u8]) {
        unreachable!()
    }
}
impl<H: Host> crate::speech_part_3::SpeechRound4Host for PoolBridge<'_, '_, H> {
    type RegexHost = NoRegex;
    fn regex_exec(
        &mut self,
        _: &mut Memory<'_>,
        _: &[u32],
        _: &mut [i32; 100],
        _: crate::speech_part_3::RegexEntry<NoRegex>,
        _: fn(&mut Memory<'_>, u32, &mut Self) -> u32,
    ) -> i32 {
        unreachable!()
    }
    fn literal(&self, a: u32) -> u32 {
        match a {
            0x179208 => self.r.unmap.category,
            0x17920c => self.r.unmap.begin_format,
            0x179210 => self.r.unmap.end_format,
            0x179214 => self.r.unmap.error_format,
            0x179218 => self.r.unmap.labels[0],
            0x17921c => self.r.unmap.labels[1],
            0x179220 => self.r.unmap.labels[2],
            _ => unreachable!("unrelated speech literal"),
        }
    }
    fn pool_name(&self, _: u32) -> u32 {
        unreachable!()
    }
}
fn read_block(
    m: &mut Memory<'_>,
    h: &mut impl Host,
    arena: u32,
    out: u32,
    size: u32,
    count: u32,
    st: u32,
    r: &StreamRuntime<'_>,
) -> u32 {
    let l = crate::boot_port_5::PoolLiterals {
        category: r.unmap.category,
        format: r.unmap.begin_format,
        after_load: 0,
        before_free: 0,
        scratch: r.scratch,
    };
    crate::boot_port_5::fn_2a9274(m, &mut PoolBridge { h, r }, arena, out, size, count, st, l)
}

#[derive(Clone, Copy)]
pub struct ImageBlock {
    pub offset: u32,
    pub limit: u32,
    pub base: u32,
}

pub fn fn_30c24c(block: &mut ImageBlock, size: u32, base: u32) -> u32 {
    block.offset = 0;
    block.limit = size;
    block.base = base;
    0
}

pub fn fn_30c28c(block: &mut ImageBlock, align: u32) -> u32 {
    let offset = block.offset;
    let mut pad = 0u32;
    if align != 0 {
        if let Ok(d) = crate::output::unsigned_division(offset, align) {
            if d[1] != 0 {
                pad = align - d[1];
            }
        }
    }
    if block.base != 0 && offset.wrapping_add(pad) > block.limit {
        return 3;
    }
    if pad != 0 {
        block.offset = offset.wrapping_add(pad);
    }
    0
}

pub fn fn_30c358(
    m: &mut Memory<'_>,
    block: &mut ImageBlock,
    size: u32,
    out: &mut u32,
) -> u32 {
    let _ = m;
    if block.base != 0 {
        *out = block.base.wrapping_add(block.offset);
    } else {
        *out = 0;
    }
    let next = block.offset.wrapping_add(size);
    block.offset = next;
    if block.base != 0 && next > block.limit {
        return 3;
    }
    0
}

pub fn fn_30c3b8(block: &mut ImageBlock, out: &mut u32) -> u32 {
    *out = block.offset;
    0
}

pub fn fn_2fa994(m: &mut Memory<'_>, s: u32) -> u32 {
    let at = m.offset(s) as usize;
    let mut n = 0u32;
    loop {
        let p = at + n as usize * 2;
        if p + 2 > m.bytes.len() {
            return n;
        }
        let v = u16::from_le_bytes([m.bytes[p], m.bytes[p + 1]]);
        if v == 0 {
            return n;
        }
        n += 1;
    }
}

pub fn fn_2e2764(m: &mut Memory<'_>, s: u32) -> i32 {
    let at = m.offset(s) as usize;
    let mut n = 0i32;
    let mut i = 0usize;
    loop {
        let p = at + i * 2;
        if p + 2 > m.bytes.len() {
            return n;
        }
        let ch = u16::from_le_bytes([m.bytes[p], m.bytes[p + 1]]);
        if ch == 0 {
            return n;
        }
        i += 1;
        if ch < 0x80 {
            n += 1;
        } else if ch < 0x800 {
            n += 2;
        } else if ch.wrapping_sub(0xd800) < 0x800 {
            return -1;
        } else {
            n += 3;
        }
    }
}

pub fn fn_2e2654(
    m: &mut Memory<'_>,
    src: u32,
    src_end: u32,
    dst: &mut u32,
    dst_end: u32,
) -> u32 {
    let mut p = src;
    let mut out = *dst;
    let mut err = 0u32;
    while p < src_end {
        let at = m.offset(p) as usize;
        if at + 2 > m.bytes.len() {
            err = 3;
            break;
        }
        let ch = u16::from_le_bytes([m.bytes[at], m.bytes[at + 1]]);
        p = p.wrapping_add(2);
        if ch.wrapping_sub(0xd800) < 0x800 {
            err = 1;
            p = p.wrapping_sub(2);
            break;
        }
        let width: u32 = if ch < 0x80 {
            0
        } else if ch < 0x800 {
            1
        } else {
            2
        };
        let q = out.wrapping_add(width);
        if q.wrapping_add(1) > dst_end {
            err = 3;
            p = p.wrapping_sub(2);
            break;
        }
        let qat = m.offset(q) as usize;
        match width {
            0 => m.bytes[qat] = ch as u8,
            1 => {
                m.bytes[qat - 1] = (0xc0 | (ch >> 6)) as u8;
                m.bytes[qat] = (0x80 | (ch & 0x3f)) as u8;
            }
            _ => {
                m.bytes[qat - 2] = (0xe0 | (ch >> 12)) as u8;
                m.bytes[qat - 1] = (0x80 | ((ch >> 6) & 0x3f)) as u8;
                m.bytes[qat] = (0x80 | (ch & 0x3f)) as u8;
            }
        }
        out = q.wrapping_add(1);
    }
    *dst = out;
    err
}

pub fn fn_2e2298(
    m: &mut Memory<'_>,
    h: &mut impl Host,
    heap: u32,
    src: u32,
    out_buf: &mut u32,
    out_len: &mut u32,
) -> u32 {
    let n = fn_2fa994(m, src);
    let end = src.wrapping_add(n.wrapping_mul(2));
    if src != end {
        let need = fn_2e2764(m, src);
        if need < 0 {
            return 3;
        }
        if *out_len <= need as u32 {
            if heap == 0 {
                return 7;
            }
            let mut err = 0u8;
            let buf = object_resize(m, h, heap, *out_buf, need as u32 + 1, &mut err);
            *out_buf = buf;
            if err != 0 {
                return err as u32;
            }
            *out_len = need as u32 + 1;
        }
        let mut dst = *out_buf;
        let dst_end = dst.wrapping_add(need as u32);
        if fn_2e2654(m, src, end, &mut dst, dst_end) != 0 {
            return 3;
        }
        let at = m.offset(dst) as usize;
        m.bytes[at] = 0;
        0
    } else {
        if *out_buf == 0 {
            if heap == 0 {
                return 7;
            }
            let mut err = 0u8;
            *out_buf = object_alloc(m, h, heap, 1, &mut err);
            if err != 0 {
                return err as u32;
            }
            *out_len = 1;
        }
        let at = m.offset(*out_buf) as usize;
        m.bytes[at] = 0;
        0
    }
}

struct MapRecords {
    count: u32,
    data: u32,
    grow: u32,
    end: u32,
    unit: u32,
    heap: u32,
}

fn map_records(m: &Memory<'_>, map: u32) -> MapRecords {
    MapRecords {
        count: m.word(map + 0x0c),
        data: m.word(map + 0x10),
        grow: m.word(map + 0x14),
        end: m.word(map + 0x18),
        unit: m.word(map + 0x1c),
        heap: m.word(map + 0x20),
    }
}

fn put_map_records(m: &mut Memory<'_>, map: u32, r: MapRecords) {
    m.put_word(map + 0x0c, r.count);
    m.put_word(map + 0x10, r.data);
    m.put_word(map + 0x14, r.grow);
    m.put_word(map + 0x18, r.end);
    m.put_word(map + 0x1c, r.unit);
    m.put_word(map + 0x20, r.heap);
}

pub fn fn_308c28(m: &mut Memory<'_>, map: u32, heap: u32, unit: u32, grow: u32) {
    for off in [0x0c, 0x10, 0x14, 0x18, 0x1c, 0x20] {
        m.put_word(map + off, 0);
    }
    m.put_word(map + 0x14, grow);
    m.put_word(map + 0x1c, unit);
    m.put_word(map + 0x20, heap);
}

pub fn fn_308d58(
    m: &mut Memory<'_>,
    h: &mut impl Host,
    map: u32,
    src: u32,
) -> u32 {
    let mut r = map_records(m, map);
    if r.end <= r.count {
        let end = r.end.wrapping_add(r.grow);
        r.end = end;

        m.put_word(map + 0x18, end);
        let mut err = 0u8;
        r.data = object_resize(m, h, r.heap, r.data, end.wrapping_mul(r.unit), &mut err);

        m.put_word(map + 0x10, r.data);
        if err != 0 {
            return 0xffff_ffff;
        }
    }
    let index = r.count;
    copy_bytes(m, r.data.wrapping_add(index.wrapping_mul(r.unit)), src, r.unit);
    r.count = index.wrapping_add(1);
    put_map_records(m, map, r);
    index
}

pub fn fn_308ddc(
    m: &mut Memory<'_>,
    h: &mut impl Host,
    map: u32,
    src: u32,
    n: u32,
    size: u32,
) -> u32 {
    let mut r = map_records(m, map);
    let mut pad = 0u32;
    if size != 0 {
        if let Ok(d) = crate::output::unsigned_division(r.count, size) {
            if d[1] != 0 {
                pad = size - d[1];
            }
        }
    }
    let index = r.count.wrapping_add(pad);
    let end = index.wrapping_add(n);
    if end > r.end {
        let mut newend = r.end.wrapping_add(r.grow);
        if newend <= end {
            newend = end;
        }
        let mut err = 0u8;
        r.data = object_resize(m, h, r.heap, r.data, newend.wrapping_mul(r.unit), &mut err);

        m.put_word(map + 0x10, r.data);
        if err != 0 {
            return 0xffff_ffff;
        }
        r.end = newend;
    }
    copy_bytes(
        m,
        r.data.wrapping_add(index.wrapping_mul(r.unit)),
        src,
        r.unit.wrapping_mul(n),
    );
    r.count = end;
    put_map_records(m, map, r);
    index
}

pub fn fn_309048(
    m: &mut Memory<'_>,
    h: &mut impl Host,
    heap: u32,
    count: u32,
    size: u32,
) -> u32 {

    let mut err = 0u8;
    let node = crate::boot_port_1::fn_1b20dc_flag(m, heap, 1, 20, &mut err, &mut Platform(h));
    if err != 0 {
        return 0;
    }
    m.put_word(node + 12, size);
    m.put_word(node + 16, heap);
    let limit = if count == 0 {
        m.word(node).wrapping_add(size)
    } else {
        count
    };
    if m.word(node) > limit {
        return node;
    }
    let mut err2 = 0u8;
    let grown = object_resize(m, h, heap, node, 20u32.wrapping_add(20u32.wrapping_mul(limit)), &mut err2);
    if err2 != 0 {
        return 0;
    }
    let mut i = m.word(grown).wrapping_add(1);
    m.put_word(grown + 4, i);
    m.put_word(grown, limit);
    while i < limit {
        let v = i.wrapping_add(1);
        m.put_word(grown.wrapping_add(20u32.wrapping_mul(i)).wrapping_add(8), v);
        i = v;
    }
    m.put_word(grown.wrapping_add(20u32.wrapping_mul(i)).wrapping_add(8), 0);
    grown
}

pub fn fn_30a2d4(
    m: &mut Memory<'_>,
    h: &mut impl Host,
    map: u32,
    heap: u32,
    lo: u32,
    hi: u32,
) -> u32 {
    let tree = fn_309048(m, h, heap, 0, lo);
    m.put_word(map, tree);
    if tree == 0 {
        return 1;
    }
    m.put_word(map + 4, 0x30a2a8);
    m.put_word(map + 8, 0x30a2bc);
    m.put_word(map + 0x24, 0xffff_ffff);
    m.put_word(map + 0x28, heap);
    fn_308c28(m, map, heap, 1, hi);
    let rc = fn_308d58(m, h, map, 0);
    if rc == 0xffff_ffff {
        return 1;
    }
    0
}

pub fn fn_309210(
    m: &mut Memory<'_>,
    h: &mut impl Host,
    map: u32,
    key: u32,
) -> u32 {
    let mut tree = m.word(map);
    if tree == 0 {
        return 0xffff_ffff;
    }
    if m.word(tree + 4) == 0 {
        let limit = m.word(tree);
        let grown = limit.wrapping_add(m.word(tree + 12));
        if limit <= grown {
            let mut err = 0u8;
            tree = object_resize(
                m,
                h,
                m.word(tree + 16),
                tree,
                20u32.wrapping_add(20u32.wrapping_mul(grown)),
                &mut err,
            );
            if err != 0 {
                return 0xffff_ffff;
            }
            let mut index = m.word(tree).wrapping_add(1);
            m.put_word(tree + 4, index);
            m.put_word(tree, grown);
            if index < grown {
                loop {
                    let v = index.wrapping_add(1);
                    m.put_word(tree.wrapping_add(20u32.wrapping_mul(index)).wrapping_add(8), v);
                    index = v;
                    if index >= grown {
                        break;
                    }
                }
            }
            m.put_word(tree.wrapping_add(20u32.wrapping_mul(index)).wrapping_add(8), 0);
            m.put_word(map, tree);
        }
    }
    let index = m.word(tree + 4);
    let node = tree.wrapping_add(20u32.wrapping_mul(index));
    m.put_word(tree + 4, m.word(node + 8));
    m.put_byte(node + 16, 0);
    m.put_word(node, key);
    m.put_word(node + 8, 0);
    m.put_word(node + 4, key);
    m.put_word(node + 12, 0);
    tree = m.word(map);
    let mut current = m.word(tree + 8);
    let mut top = current;
    let mut above = 0u32;
    let mut parent = 0u32;
    if current == 0 {
        m.put_word(tree + 8, index);
        return 0;
    }
    let mut cmp = 0i32;
    loop {
        let curn = tree.wrapping_add(20u32.wrapping_mul(current));
        if m.byte(curn + 16) != 0 {
            top = current;
            above = parent;
        }
        cmp = map_compare(m, map, key, m.word(curn + 4));
        if cmp == 0 {
            let old = m.word(curn + 4);
            m.put_word(tree.wrapping_add(20u32.wrapping_mul(index)).wrapping_add(8), m.word(tree + 4));
            m.put_word(tree + 4, index);
            m.put_word(curn + 4, key);
            if m.word(curn) == 0xffff_ffff {
                m.put_word(curn, key);
            }
            return old;
        }
        parent = current;
        if cmp < 0 {
            if m.word(curn + 8) == 0 {
                break;
            }
            current = m.word(curn + 8);
        } else {
            if m.word(curn + 12) == 0 {
                break;
            }
            current = m.word(curn + 12);
        }
    }
    if cmp < 0 {
        m.put_word(tree.wrapping_add(20u32.wrapping_mul(parent)).wrapping_add(8), index);
    } else {
        m.put_word(tree.wrapping_add(20u32.wrapping_mul(parent)).wrapping_add(12), index);
    }
    let dir: i32 = if map_compare(m, map, key, m.word(tree.wrapping_add(20u32.wrapping_mul(top)).wrapping_add(4))) < 0 {
        1
    } else {
        -1
    };
    {
        let topn = tree.wrapping_add(20u32.wrapping_mul(top));
        let mut child = if dir > 0 { m.word(topn + 8) } else { m.word(topn + 12) };
        if child != index {
            loop {
                let curn = tree.wrapping_add(20u32.wrapping_mul(child));
                let cmp2 = map_compare(m, map, key, m.word(curn + 4));
                let next = if cmp2 < 0 { m.word(curn + 8) } else { m.word(curn + 12) };
                m.put_byte(curn + 16, if cmp2 < 0 { 1 } else { 0xff });
                if next == index {
                    break;
                }
                child = next;
            }
        }
    }
    {
        let topn = tree.wrapping_add(20u32.wrapping_mul(top));
        let balance = m.byte(topn + 16) as i8;
        if balance == 0 {
            m.put_byte(topn + 16, if dir > 0 { 1 } else { 0xff });
            return 0;
        }
        if dir + balance as i32 == 0 {
            m.put_byte(topn + 16, 0);
            return 0;
        }
        let root;
        if dir > 0 {
            let left = m.word(topn + 8);
            let leftn = tree.wrapping_add(20u32.wrapping_mul(left));
            if m.byte(leftn + 16) == 1 {
                m.put_word(topn + 8, m.word(leftn + 12));
                m.put_word(leftn + 12, top);
                m.put_byte(topn + 16, 0);
                m.put_byte(leftn + 16, 0);
                root = left;
            } else {
                let mid = m.word(leftn + 12);
                let midn = tree.wrapping_add(20u32.wrapping_mul(mid));
                let midbal = m.byte(midn + 16) as i8;
                m.put_word(leftn + 12, m.word(midn + 8));
                m.put_word(topn + 8, m.word(midn + 12));
                m.put_word(midn + 8, left);
                m.put_word(midn + 12, top);
                match midbal {
                    -1 => {
                        m.put_byte(leftn + 16, 1);
                        m.put_byte(topn + 16, 0);
                    }
                    0 => {
                        m.put_byte(leftn + 16, 0);
                        m.put_byte(topn + 16, 0);
                    }
                    1 => {
                        m.put_byte(leftn + 16, 0);
                        m.put_byte(topn + 16, 0xff);
                    }
                    _ => {}
                }
                m.put_byte(midn + 16, 0);
                root = mid;
            }
        } else {
            let right = m.word(topn + 12);
            let rightn = tree.wrapping_add(20u32.wrapping_mul(right));
            if m.byte(rightn + 16) == 0xff {
                m.put_word(topn + 12, m.word(rightn + 8));
                m.put_word(rightn + 8, top);
                m.put_byte(topn + 16, 0);
                m.put_byte(rightn + 16, 0);
                root = right;
            } else {
                let mid = m.word(rightn + 8);
                let midn = tree.wrapping_add(20u32.wrapping_mul(mid));
                let midbal = m.byte(midn + 16) as i8;
                m.put_word(rightn + 8, m.word(midn + 12));
                m.put_word(topn + 12, m.word(midn + 8));
                m.put_word(midn + 12, right);
                m.put_word(midn + 8, top);
                match midbal {
                    -1 => {
                        m.put_byte(topn + 16, 1);
                        m.put_byte(rightn + 16, 0);
                    }
                    0 => {
                        m.put_byte(rightn + 16, 0);
                        m.put_byte(topn + 16, 0);
                    }
                    1 => {
                        m.put_byte(rightn + 16, 0xff);
                        m.put_byte(topn + 16, 0);
                    }
                    _ => {}
                }
                m.put_byte(midn + 16, 0);
                root = mid;
            }
        }
        if above == 0 {
            m.put_word(tree + 8, root);
        } else {
            let up = tree.wrapping_add(20u32.wrapping_mul(above));
            if m.word(up + 8) == top {
                m.put_word(up + 8, root);
            } else if m.word(up + 12) == top {
                m.put_word(up + 12, root);
            }
        }
    }
    0
}

fn map_compare(m: &mut Memory<'_>, map: u32, a: u32, b: u32) -> i32 {
    let base = m.word(map + 0x10);
    let unit = m.word(map + 0x1c);
    let x = base.wrapping_add(a.wrapping_mul(unit));
    let y = base.wrapping_add(b.wrapping_mul(unit));
    let mut i = 0usize;
    loop {
        let xa = m.offset(x) as usize + i;
        let ya = m.offset(y) as usize + i;
        if xa >= m.bytes.len() || ya >= m.bytes.len() {
            return 0;
        }
        let p = m.bytes[xa];
        let q = m.bytes[ya];
        if p != q {
            return p as i32 - q as i32;
        }
        if p == 0 {
            return 0;
        }
        i += 1;
    }
}

pub fn fn_30a3a4(
    m: &mut Memory<'_>,
    h: &mut impl Host,
    map: u32,
    key: u32,
    data: u32,
    len: u32,
    flag: &mut u8,
) -> u32 {
    let keylen = strlen(m, key).wrapping_add(1);
    *flag = 1;
    let keypos = fn_308ddc(m, h, map, key, keylen, 1);
    if keypos == 0xffff_ffff {
        return 1;
    }
    let slot = fn_308ddc(m, h, map, 0, 4, 4);
    if slot == 0xffff_ffff {
        return 1;
    }
    let slot_at = m
        .word(map + 0x10)
        .wrapping_add(slot.wrapping_mul(m.word(map + 0x1c)));
    m.put_word(slot_at, len);
    if fn_308ddc(m, h, map, data, len, 1) == 0xffff_ffff {
        return 1;
    }
    let node = fn_309210(m, h, map, keypos);
    if node == 0xffff_ffff {
        return 1;
    }
    if fn_308ddc(m, h, map, 0, 4, 4) == 0xffff_ffff {
        return 1;
    }
    if node != 0 {
        let rec = m.word(map + 0x10).wrapping_add(node.wrapping_mul(m.word(map + 0x1c)));
        let n = strlen(m, rec).wrapping_add(1);
        let slot = rec.wrapping_add(n);
        let slot = (slot.wrapping_add(3)) & !3u32;
        let word = m.word(slot);
        let addr = slot.wrapping_add(4).wrapping_add(word);
        let dst = (addr.wrapping_add(3)) & !3u32;
        m.put_word(dst, keypos);
        *flag = 0;
    }
    0
}

fn copy_bytes(m: &mut Memory<'_>, dst: u32, src: u32, n: u32) {
    let d = m.offset(dst) as usize;
    let s = m.offset(src) as usize;
    if s >= m.bytes.len() || d >= m.bytes.len() {
        return;
    }
    let count = (n as usize).min(m.bytes.len() - d).min(m.bytes.len() - s);
    m.bytes.copy_within(s..s + count, d);
}

fn object_resize(
    m: &mut Memory<'_>,
    h: &mut impl Host,
    heap: u32,
    old: u32,
    size: u32,
    err: &mut u8,
) -> u32 {
    crate::boot_part_f::fn_1b218c(m, heap, old, size, err, &mut Platform(h))
}

fn fn_300c2c(m: &mut Memory<'_>, h: &mut impl Host, obj: u32, out: &mut u32) -> u8 {
    let mut err = 0u8;
    let mut last = m.word(obj + 40);
    let p = crate::boot_port_1::fn_1b20dc_flag(
        m,
        m.word(obj + 36),
        1,
        80,
        &mut err,
        &mut Platform(h),
    );
    if err == 0 {
        if last == 0 {
            m.put_word(obj + 40, p);
        } else {
            while m.word(last + 76) != 0 {
                last = m.word(last + 76);
            }
            m.put_word(last + 76, p);
        }
        m.put_word(p + 76, 0);
        *out = p;
    }
    err
}

pub fn fn_2fea84(m: &mut Memory<'_>, h: &mut impl Host, obj: u32, arg: u32) -> u32 {
    let alloc = m.word(obj + 36);
    let mut block = ImageBlock {
        offset: 0,
        limit: 0,
        base: 0,
    };
    let mut err = fn_30c24c(&mut block, m.word(arg + 4), arg);
    if err != 0 {
        return err as u32;
    }
    err = fn_30c28c(&mut block, 4);
    if err != 0 {
        return err as u32;
    }
    let mut header = 0u32;
    err = fn_30c358(m, &mut block, 0x18, &mut header);
    if err != 0 {
        return err as u32;
    }
    if m.word(header) & 0x00ff_ffff != 6 {
        return 4;
    }
    if m.word(header + 8) != 0 {
        return 4;
    }
    m.put_word(obj + 12, m.word(header + 0x0c));
    m.put_word(obj + 16, m.word(header + 0x10));
    let count = m.word(header + 0x14);
    let mut str_ptr = 0u32;
    err = fn_30c28c(&mut block, 1);
    if err != 0 {
        return err as u32;
    }
    err = fn_30c358(m, &mut block, 0, &mut str_ptr);
    if err != 0 {
        return err as u32;
    }
    let mut len = strlen(m, str_ptr).wrapping_add(1);
    err = fn_30c358(m, &mut block, len, &mut str_ptr);
    if err != 0 {
        return err as u32;
    }
    if len == 1 {
        m.put_word(obj, 0);
    } else {
        let mut e = 0u8;
        let p = object_alloc(m, h, alloc, len, &mut e);
        m.put_word(obj, p);
        if e != 0 {
            return e as u32;
        }
        strcpy(m, p, str_ptr);
    }
    let mut str2_ptr = 0u32;
    err = fn_30c28c(&mut block, 1);
    if err != 0 {
        return err as u32;
    }
    err = fn_30c358(m, &mut block, 0, &mut str2_ptr);
    if err != 0 {
        return err as u32;
    }
    len = strlen(m, str2_ptr).wrapping_add(1);
    err = fn_30c358(m, &mut block, len, &mut str2_ptr);
    if err != 0 {
        return err as u32;
    }
    if len == 1 {
        m.put_word(obj + 4, 0);
    } else {
        let mut e = 0u8;
        let p = object_alloc(m, h, alloc, len, &mut e);
        m.put_word(obj + 4, p);
        if e != 0 {
            return e as u32;
        }
        strcpy(m, p, str2_ptr);
    }
    let mut keys = 0u32;
    err = fn_30c28c(&mut block, 4);
    if err != 0 {
        return err as u32;
    }
    err = fn_30c358(m, &mut block, count.wrapping_mul(4), &mut keys);
    if err != 0 {
        return err as u32;
    }
    err = fn_30c28c(&mut block, 4);
    if err != 0 {
        return err as u32;
    }
    let mut peek = 0u32;
    err = fn_30c358(m, &mut block, 0, &mut peek);
    if err != 0 {
        return err as u32;
    }
    if count == 0 {
        return 0;
    }
    let mut counter = 0u32;
    loop {
        let mut node = 0u32;
        let e = fn_300c2c(m, h, obj, &mut node);
        if e != 0 {
            return e as u32;
        }
        err = fn_30c28c(&mut block, 4);
        if err != 0 {
            return err as u32;
        }
        err = fn_30c28c(&mut block, 4);
        if err != 0 {
            return err as u32;
        }
        let mut rec = 0u32;
        err = fn_30c358(m, &mut block, 0x30, &mut rec);
        if err != 0 {
            return err as u32;
        }
        let flat = if m.word(rec + 4) == 0 { 1 } else { 0 };
        m.put_word(node + 8, m.word(rec + 8));
        m.put_word(node + 16, m.word(rec + 0x10));
        m.put_word(node + 12, m.word(rec + 0x0c));
        m.put_word(node, m.word(rec + 0x20));
        m.put_word(node + 4, m.word(rec + 0x24));
        m.put_word(node + 20, 0);
        if m.word(rec + 0x14) == 1 {
            err = fn_30c358(m, &mut block, 0, &mut peek);
            if err != 0 {
                return err as u32;
            }
            err = fn_30c358(m, &mut block, 0x81, &mut peek);
            if err != 0 {
                return err as u32;
            }
            let mut e = 0u8;
            let p = object_alloc(m, h, alloc, 0x81, &mut e);
            m.put_word(node + 24, p);
            if e != 0 {
                return e as u32;
            }
            copy_bytes(m, p, peek, 0x80);
            let at = m.offset(p) as usize + 0x80;
            m.bytes[at] = 0;
        } else {
            m.put_word(node + 24, 0);
        }
        err = fn_30c28c(&mut block, 1);
        if err != 0 {
            return err as u32;
        }
        if m.word(rec + 0x1c) != 0 {
            err = fn_30c358(m, &mut block, 0, &mut peek);
            if err != 0 {
                return err as u32;
            }
            len = strlen(m, peek).wrapping_add(1);
            err = fn_30c358(m, &mut block, len, &mut peek);
            if err != 0 {
                return err as u32;
            }
            let mut e = 0u8;
            let p = object_alloc(m, h, alloc, len, &mut e);
            m.put_word(node + 28, p);
            if e != 0 {
                return e as u32;
            }
            strcpy(m, p, peek);
        } else {
            m.put_word(node + 28, 0);
        }
        err = fn_30a2d4(m, h, node + 32, alloc, 0x2710, 0xc350);
        if err != 0 {
            return err as u32;
        }
        err = fn_30c28c(&mut block, 4);
        if err != 0 {
            return err as u32;
        }
        let mut entry = 0u32;
        err = fn_30c358(m, &mut block, m.word(node).wrapping_mul(4).wrapping_add(4), &mut entry);
        if err != 0 {
            return err as u32;
        }
        err = fn_30c28c(&mut block, if flat != 0 { 1 } else { 2 });
        if err != 0 {
            return err as u32;
        }
        let mut data = 0u32;
        err = fn_30c358(m, &mut block, m.word(rec + 0x28), &mut data);
        if err != 0 {
            return err as u32;
        }
        err = fn_30c28c(&mut block, 4);
        if err != 0 {
            return err as u32;
        }
        let mut entry2 = 0u32;
        err = fn_30c358(m, &mut block, m.word(node).wrapping_mul(4).wrapping_add(4), &mut entry2);
        if err != 0 {
            return err as u32;
        }
        let mut start = 0u32;
        err = fn_30c3b8(&mut block, &mut start);
        if err != 0 {
            return err as u32;
        }
        err = fn_30c28c(&mut block, 1);
        if err != 0 {
            return err as u32;
        }
        err = fn_30c358(m, &mut block, 1, &mut peek);
        if err != 0 {
            return err as u32;
        }
        let mut buf = 0u32;
        let mut i = 0u32;
        while i < m.word(node) {
            let e = m.word(entry);
            let e2 = m.word(entry2);
            let span0 = m.word(entry2 + 4);
            let mut span = span0 as i32;
            let mut flag = 0xffu8;
            if flat != 0 {
                let n = strlen(m, data.wrapping_add(e)).wrapping_add(1);
                let mut e3 = 0u8;
                buf = object_resize(m, h, alloc, buf, n, &mut e3);
                if e3 != 0 {
                    return e3 as u32;
                }
                strcpy(m, buf, data.wrapping_add(e));
            } else {
                let mut out_len = 0u32;
                let rc = fn_2e2298(
                    m,
                    h,
                    alloc,
                    data.wrapping_add(e),
                    &mut buf,
                    &mut out_len,
                );
                if rc != 0 {
                    return rc;
                }
            }
            if span < 0 {
                span = -span;
            }
            if (e2 as i32) < 0 {
                let used = (e2 as i32).wrapping_neg() as u32;
                err = fn_30c28c(&mut block, 2);
                if err != 0 {
                    return err as u32;
                }
                if used >= span as u32 {
                    entry = entry.wrapping_add(4);
                    entry2 = entry2.wrapping_add(4);
                    i += 1;
                    continue;
                }
                loop {
                    let mut consumed = 0u32;
                    err = fn_30c358(m, &mut block, 2, &mut peek);
                    if err != 0 {
                        return err as u32;
                    }
                    err = fn_30c28c(&mut block, 1);
                    if err != 0 {
                        return err as u32;
                    }
                    let value = m.word(peek) & 0xffff;
                    let mut payload = 0u32;
                    err = fn_30c358(m, &mut block, value, &mut payload);
                    if err != 0 {
                        return err as u32;
                    }
                    let rc = fn_30a3a4(m, h, node + 32, buf, payload, value, &mut flag);
                    if rc != 0 {
                        return rc;
                    }
                    err = fn_30c28c(&mut block, 2);
                    if err != 0 {
                        return err as u32;
                    }
                    err = fn_30c3b8(&mut block, &mut consumed);
                    if err != 0 {
                        return err as u32;
                    }
                    consumed = consumed.wrapping_sub(start);
                    if consumed >= span as u32 {
                        break;
                    }
                }
            } else {
                let mut consumed = 0u32;
                err = fn_30c28c(&mut block, 1);
                if err != 0 {
                    return err as u32;
                }
                err = fn_30c3b8(&mut block, &mut consumed);
                if err != 0 {
                    return err as u32;
                }
                consumed = consumed.wrapping_sub(start);
                span -= consumed as i32;
                let mut payload = 0u32;
                err = fn_30c358(m, &mut block, span as u32, &mut payload);
                if err != 0 {
                    return err as u32;
                }
                let rc = fn_30a3a4(m, h, node + 32, buf, payload, span as u32, &mut flag);
                if rc != 0 {
                    return rc;
                }
            }
            entry = entry.wrapping_add(4);
            entry2 = entry2.wrapping_add(4);
            i += 1;
        }
        object_free(m, h, alloc, buf);
        keys = keys.wrapping_add(4);
        counter += 1;
        if counter < count {
            continue;
        }
        break;
    }
    0
}
