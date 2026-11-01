
use crate::{boot::Memory, frontend as f};
use core::cell::RefCell;
fn nz(p: u32) -> Option<u32> {
    (p != 0).then_some(p)
}

fn view_word(v: &mut [u8], p: usize, x: u32) {
    v[p..p + 4].copy_from_slice(&x.to_le_bytes());
}
fn teardown_lock(
    m: &mut Memory<'_>,
    p: u32,
    release: bool,
    h: &mut impl crate::render_part_c::Host,
) -> u32 {
    if p == 0 {
        return if release {
            f::release_lock(&[], None, |_| unreachable!())
        } else {
            f::acquire_lock(&[], None, |_| unreachable!())
        };
    }
    let offset = if release { 12 } else { 8 };
    let method = m.word(m.word(m.word(p) + 8) + offset);
    let mut view = [0u8; 40];
    view_word(&mut view, 0, 12);
    view_word(&mut view, 8, m.word(p + 8));
    view_word(&mut view, 20, 24);
    view_word(&mut view, 24 + offset as usize, method);
    let invoke = |arg| crate::render_part_c::Host::call(h, m, method, &[arg]);
    if release {
        f::release_lock(&view, Some(0), invoke)
    } else {
        f::acquire_lock(&view, Some(0), invoke)
    }
}
struct TeardownFree<'a, 'b, H> {
    m: &'a mut Memory<'b>,
    host: &'a mut H,
    heap: u32,
    p: u32,
}
impl<H: crate::render_part_c::Host> f::HeapVtable for TeardownFree<'_, '_, H> {
    fn alloc_at(&mut self, _: u32, _: u32) -> u32 {
        unreachable!()
    }
    fn realloc_at(&mut self, _: u32, _: u32, _: u32) -> u32 {
        unreachable!()
    }
    fn release_at(&mut self, _: u32, _: u32) {
        stage_release(self.m, self.heap, self.p, self.host);
    }
}
fn teardown_iterator(
    m: &mut Memory<'_>,
    map: u32,
    h: &mut impl crate::render_part_c::Host,
) -> (u32, u32) {
    let node = f::last_link(m.bytes, map + 24);
    let heap = m.word(map + 20);
    let mut view = [0u8; 112];
    view_word(&mut view, 20, 96);
    view_word(&mut view, 100, 0);
    view_word(&mut view, 24, if node == 0 { 24 } else { 48 });
    view_word(&mut view, 48, 24);
    struct Allocate<'a, 'b, H> {
        m: &'a mut Memory<'b>,
        h: &'a mut H,
        heap: u32,
        result: u32,
    }
    impl<H: crate::render_part_c::Host> f::MethodTable for Allocate<'_, '_, H> {
        fn call_at(&mut self, _: u32, n: u32, size: u32) -> u32 {
            self.result = stage_calloc(self.m, self.heap, n, size, self.h);
            if self.result == 0 {
                0
            } else {
                64
            }
        }
    }
    let mut relative = 0;
    let mut allocate = Allocate {
        m,
        h,
        heap,
        result: 0,
    };
    let rc = f::alloc_handle_for_key(
        &mut view,
        Some(0),
        0,
        1,
        Some(&mut relative),
        &mut allocate,
        |_, _| unreachable!(),
    );
    let result = allocate.result;
    if rc < 0x80000000 {
        assert_eq!(relative, 64);
        assert_eq!(u32::from_le_bytes(view[64..68].try_into().unwrap()), 0);
        assert_eq!(u32::from_le_bytes(view[72..76].try_into().unwrap()), if node==0 {0}else{48});
        m.put_word(result, map);
        m.put_word(
            result + 4,
            u32::from_le_bytes(view[68..72].try_into().unwrap()),
        );
        m.put_word(
            result + 8,
            if u32::from_le_bytes(view[72..76].try_into().unwrap()) == 0 {
                0
            } else {
                node
            },
        );
    }
    (rc, result)
}
fn teardown_iterator_close(
    m: &mut Memory<'_>,
    iterator: u32,
    h: &mut impl crate::render_part_c::Host,
) -> u32 {
    let heap = m.word(m.word(iterator) + 20);
    let mut view = [0u8; 72];
    view_word(&mut view, 0, 32);
    view_word(&mut view, 52, 64);
    f::release_owner(
        &view,
        Some(0),
        &mut TeardownFree {
            m,
            host: h,
            heap,
            p: iterator,
        },
    )
}
fn teardown_owner(m: &mut Memory<'_>, p: u32, h: &mut impl crate::render_part_c::Host) -> u32 {
    let method = m.word(m.word(m.word(p) + 8) + 4);
    let mut view = [0u8; 40];
    view_word(&mut view, 0, 12);
    view_word(&mut view, 4, 32);
    view_word(&mut view, 8, m.word(p + 8));
    view_word(&mut view, 20, 24);
    view_word(&mut view, 28, method);
    let shared = RefCell::new((m, h));
    struct Free<'a, 'b, 'c, 'd, H> {
        shared: &'a RefCell<(&'b mut Memory<'c>, &'d mut H)>,
        p: u32,
    }
    impl<H: crate::render_part_c::Host> f::HeapVtable for Free<'_, '_, '_, '_, H> {
        fn alloc_at(&mut self, _: u32, _: u32) -> u32 {
            unreachable!()
        }
        fn realloc_at(&mut self, _: u32, _: u32, _: u32) -> u32 {
            unreachable!()
        }
        fn release_at(&mut self, _: u32, _: u32) {
            let mut pair = self.shared.borrow_mut();
            let (m, h) = &mut *pair;
            let heap = m.word(self.p + 4);
            stage_release(m, heap, self.p, *h);
        }
    }
    f::release_guarded(&view, Some(0), &mut Free { shared: &shared, p }, |arg| {
        let mut pair = shared.borrow_mut();
        let (m, h) = &mut *pair;
        crate::render_part_c::Host::call(*h, m, method, &[arg])
    })
}
struct TeardownMap<'a, H>(&'a mut H);
impl<H: crate::render_part_c::Host> crate::boot_part_b::Host for TeardownMap<'_, H> {
    fn call(&mut self, m: &mut Memory<'_>, entry: u32, a: &[u32]) -> u32 {
        if entry == 0x196428 {
            stage_release(m, a[0], a[1], self.0);
            0
        } else {
            crate::render_part_c::Host::call(self.0, m, entry, a)
        }
    }
    fn thread_id(&mut self) -> u32 {
        unreachable!()
    }
}

pub fn fn_17af38(m: &mut Memory<'_>, object: u32, h: &mut impl crate::render_part_c::Host) -> u32 {
    assert_eq!(m.base, 0);
    if object == 0 {
        return 0x8b602007;
    }
    let mut rc = teardown_lock(m, m.word(object + 12), false, h);
    if rc >= 0x80000000 {
        return rc;
    }
    if m.word(object + 16) != 0 {
        let (status, iterator) = teardown_iterator(m, m.word(object + 16), h);
        if status < 0x80000000 {
            let (mut key, mut state) = (0, 0);
            loop {
                if f::step_iter(m.bytes, nz(iterator), Some(&mut key), Some(&mut state)) != 0 {
                    break;
                }
                if sh(m.bytes, state + 28) == 0 {
                    continue;
                }
                teardown_iterator_close(m, iterator, h);
                teardown_lock(m, m.word(object + 12), true, h);
                return 0x8b602011;
            }
            teardown_iterator_close(m, iterator, h);
        }
        if m.word(object + 16) != 0 {
            crate::boot_part_b2::guest_19665c_live(m, m.word(object + 16), &mut TeardownMap(h));
            m.put_word(object + 16, 0);
        }
    }
    let status = teardown_lock(m, m.word(object + 12), true, h);
    if status >= 0x80000000 && rc < 0x80000000 {
        rc = status;
    }
    if m.word(object + 12) != 0 {
        let status = teardown_owner(m, m.word(object + 12), h);
        if status >= 0x80000000 && rc < 0x80000000 {
            rc = status;
        }
    }
    stage_release(m, m.word(m.word(object + 8) + 4), object, h);
    rc
}

pub fn fn_196238(
    m: &mut Memory<'_>,
    handle: [u32; 2],
    e: &crate::render_part_c::Environment,
    h: &mut impl crate::render_part_c::Host,
) -> u32 {
    assert_eq!(m.base, 0);
    let mut pair = [0u8; 8];
    f::load_pair(&mut pair, &e.cached_handle);
    let (a, b) = (
        u32::from_le_bytes(pair[..4].try_into().unwrap()),
        u32::from_le_bytes(pair[4..].try_into().unwrap()),
    );
    let mut object = handle[0];
    let mut rc = 0;
    if f::match_halves(handle[0], handle[1], a, b) != 0 {
        object = e.default_context;
    } else {
        rc = f::validate_handle(object != 0, handle[1], 0x1e61, 32);
        if rc < 0x80000000 {
            if object == 0 || sh(m.bytes, object) != 0x1e61 {
                return 0x81602008;
            }
        } else {
            if rc & 0x1fff == 8 {
                return 0x81602008;
            }
            object = e.default_context;
        }
        if rc >= 0x80000000 {
            return rc;
        }
    }
    if object != 0 {
        let heap = m.word(object + 4);
        crate::fe_text::memset_195bec(m.bytes, object, 0, 32);
        stage_release(m, heap, object, h);
    }
    rc
}

pub struct CloseRuntime {
    pub category: u32,
    pub begin: u32,
    pub end: u32,
    pub error: u32,
    pub handle_key: u32,
    pub status_key: u32,
    pub scratch: u32,
    pub fold: [u16; 256],
}
pub trait CloseHost:
    f::HeapVtable
    + f::MethodTable
    + f::StateLock
    + crate::boot_part_b::FormatHost
    + crate::log_live::LogMethodsM
{
    fn select_close_lock(&mut self, lock: u32);
    fn compare_tree_keys(&mut self, a: u32, b: u32) -> i32;
    fn indirect(&mut self, m: &mut Memory<'_>, method: u32, args: &[u32]) -> u32;
}
struct Borrow<'a, 'b, H>(&'a RefCell<&'b mut H>);
impl<H: CloseHost> f::StateLock for Borrow<'_, '_, H> {
    fn acquire(&mut self) -> u32 {
        self.0.borrow_mut().acquire()
    }
    fn release(&mut self) -> u32 {
        self.0.borrow_mut().release()
    }
}
impl<H: CloseHost> f::HeapVtable for Borrow<'_, '_, H> {
    fn alloc_at(&mut self, o: u32, n: u32) -> u32 {
        self.0.borrow_mut().alloc_at(o, n)
    }
    fn realloc_at(&mut self, o: u32, p: u32, n: u32) -> u32 {
        self.0.borrow_mut().realloc_at(o, p, n)
    }
    fn release_at(&mut self, o: u32, p: u32) {
        self.0.borrow_mut().release_at(o, p)
    }
}
impl<H: CloseHost> f::MethodTable for Borrow<'_, '_, H> {
    fn call_at(&mut self, o: u32, n: u32, s: u32) -> u32 {
        self.0.borrow_mut().call_at(o, n, s)
    }
}
fn lookup(
    m: &mut Memory<'_>,
    ctx: u32,
    name: u32,
    k: &CloseRuntime,
    h: &mut impl CloseHost,
) -> u32 {
    h.select_close_lock(m.word(m.word(ctx + 4) + 28));
    let cell = RefCell::new(h);
    let mut out = 0;
    f::lookup_keyed_value(
        m.bytes,
        nz(ctx),
        nz(name),
        &mut out,
        &k.fold,
        &mut Borrow(&cell),
        &mut Borrow(&cell),
        &mut Borrow(&cell),
        |a, b| cell.borrow_mut().compare_tree_keys(a, b),
    );
    out
}
struct Calls<'a, H>(&'a mut H);
impl<H: CloseHost> crate::boot_part_b::Host for Calls<'_, H> {
    fn call(&mut self, m: &mut Memory<'_>, f: u32, a: &[u32]) -> u32 {
        if f == 0x196428 {
            f::heap_release(m.bytes, nz(a[0]), nz(a[1]), self.0);
            0
        } else {
            self.0.indirect(m, f, a)
        }
    }
    fn thread_id(&mut self) -> u32 {
        panic!("archive close does not request thread id")
    }
}
impl<H: CloseHost> crate::boot_part_e::Host for Calls<'_, H> {
    fn call(&mut self, m: &mut Memory<'_>, entry: u32, a: &[u32]) -> u32 {
        match entry {
            0x196394 => f::heap_alloc(m.bytes, nz(a[0]), a[1], self.0),
            0x1963b8 => f::call_method(m.bytes, nz(a[0]), a[1], a[2], self.0),
            0x1963e0 => f::heap_realloc(m.bytes, nz(a[0]), a[1], a[2], self.0),
            0x196428 => {
                f::heap_release(m.bytes, nz(a[0]), nz(a[1]), self.0);
                0
            }
            _ => self.0.indirect(m, entry, a),
        }
    }
}
impl<H: CloseHost> crate::boot_part_d::Host for Calls<'_, H> {
    fn call_m(&mut self, f: u32, a: &[u32], m: &mut Memory<'_>) -> u32 {
        self.0.indirect(m, f, a)
    }
    fn params_call_m(
        &mut self,
        _: u32,
        _: &mut [u32; 4],
        _: u32,
        _: &mut u32,
        _: u32,
        _: &mut Memory<'_>,
    ) -> u32 {
        panic!("close does not open archive")
    }
}
fn info(
    m: &mut Memory<'_>,
    h: &mut impl CloseHost,
    ctx: u32,
    k: &CloseRuntime,
    format: u32,
    args: &[u32],
) {
    let mut saved = [0u8; 64];
    saved.copy_from_slice(&m.bytes[k.scratch as usize..k.scratch as usize + 64]);
    for (i, &v) in args.iter().enumerate() {
        m.put_word(k.scratch + i as u32 * 4, v);
    }
    crate::log_live::fn_1738e8(m, m.word(ctx), k.category, 4, 0, format, k.scratch, h);
    m.bytes[k.scratch as usize..k.scratch as usize + 64].copy_from_slice(&saved);
}

pub fn fn_179f3c(
    m: &mut Memory<'_>,
    ctx: u32,
    handle: u32,
    k: &CloseRuntime,
    h: &mut impl CloseHost,
) -> u32 {
    assert_eq!(m.base, 0);
    if ctx == 0 || handle == 0 {
        return 0x81506007;
    }
    let box_ = m.word(m.word(ctx + 4) + 8);
    let heap = m.word(box_ + 4);
    let mut rc = 0;
    info(m, h, ctx, k, k.begin, &[handle]);
    let data = m.word(handle + 4);
    if data != 0 {
        let refs = u16::from_le_bytes([m.byte(data + 60), m.byte(data + 61)]).wrapping_sub(1);
        m.put_half(data + 60, refs);
        if refs == 0 {
            let mut found = lookup(m, ctx, m.word(data + 64), k, h);
            if found == 0 {
                let name = m.word(m.word(handle + 4) + 64);
                let size = f::strlen_bytes(&m.bytes[name as usize..]).wrapping_add(1);
                let copy = f::heap_alloc(m.bytes, nz(heap), size, h);
                if copy == 0 {
                    return 0x8150600a;
                }
                let name = m.word(m.word(handle + 4) + 64);
                let mut i = 0;
                while i < f::strlen_bytes(&m.bytes[name as usize..]) {
                    let c = m.byte(name + i);
                    let c = if c.is_ascii_alphanumeric() {
                        c
                    } else if c == b'/' || c == b'_' {
                        b'_'
                    } else {
                        b'-'
                    };
                    m.put_byte(copy + i, c);
                    i += 1;
                }
                m.put_byte(copy + i, 0);
                let name = m.word(m.word(handle + 4) + 64);
                if f::compare_str(&m.bytes[name as usize..], &m.bytes[copy as usize..]) != 0 {
                    found = lookup(m, ctx, copy, k, h);
                }
                f::heap_release(m.bytes, nz(heap), nz(copy), h);
            }
            if found != 0 {
                m.put_word(found + 8, 0);
            }
            let data = m.word(handle + 4);
            if m.word(data + 56) != 0 {
                crate::boot_part_d::fn_1730fc(m, m.word(data + 56), &mut Calls(h));
                m.put_word(m.word(handle + 4) + 56, 0);
            }
            crate::boot_part_b::fn_17ca94(m, m.word(handle + 4), &mut Calls(h));
            m.put_word(handle + 4, 0);
        }
    } else if m.word(handle) != 0 {
        rc = crate::boot_part_d::fn_1730fc(m, m.word(handle), &mut Calls(h));
        m.put_word(handle, 0);
    }
    if rc >= 0x80000000 {
        let mut saved = [0u8; 64];
        saved.copy_from_slice(&m.bytes[k.scratch as usize..k.scratch as usize + 64]);
        for (i, v) in [k.handle_key, handle, k.status_key, rc]
            .into_iter()
            .enumerate()
        {
            m.put_word(k.scratch + i as u32 * 4, v);
        }
        crate::render_part_a::error_log(
            m,
            h,
            m.word(ctx),
            k.category,
            0x118,
            k.error,
            k.scratch,
            k.scratch + 32,
        );
        m.bytes[k.scratch as usize..k.scratch as usize + 64].copy_from_slice(&saved);
    }
    f::heap_release(m.bytes, nz(heap), nz(handle), h);
    info(m, h, ctx, k, k.end, &[rc]);
    rc
}

pub fn fn_179f3c_m(
    m: &mut Memory<'_>,
    ctx: u32,
    handle: u32,
    k: &CloseRuntime,
    h: &mut (impl CloseHost + crate::boot_part_e::Host),
) -> u32 {
    assert_eq!(m.base, 0);
    if ctx == 0 || handle == 0 {
        return 0x81506007;
    }
    let box_ = m.word(m.word(ctx + 4) + 8);
    let heap = m.word(box_ + 4);
    let mut rc = 0;
    info(m, h, ctx, k, k.begin, &[handle]);
    let data = m.word(handle + 4);
    if data != 0 {
        let refs = u16::from_le_bytes([m.byte(data + 60), m.byte(data + 61)]).wrapping_sub(1);
        m.put_half(data + 60, refs);
        if refs == 0 {
            let mut found = lookup(m, ctx, m.word(data + 64), k, h);
            if found == 0 {
                let name = m.word(m.word(handle + 4) + 64);
                let size = f::strlen_bytes(&m.bytes[name as usize..]).wrapping_add(1);
                let copy = crate::boot_part_e::alloc(m, heap, size, h);
                if copy == 0 {
                    return 0x8150600a;
                }
                let name = m.word(m.word(handle + 4) + 64);
                let mut i = 0;
                while i < f::strlen_bytes(&m.bytes[name as usize..]) {
                    let c = m.byte(name + i);
                    let c = if c.is_ascii_alphanumeric() {
                        c
                    } else if c == b'/' || c == b'_' {
                        b'_'
                    } else {
                        b'-'
                    };
                    m.put_byte(copy + i, c);
                    i += 1;
                }
                m.put_byte(copy + i, 0);
                let name = m.word(m.word(handle + 4) + 64);
                if f::compare_str(&m.bytes[name as usize..], &m.bytes[copy as usize..]) != 0 {
                    found = lookup(m, ctx, copy, k, h);
                }
                crate::boot_part_e::release(m, heap, copy, h);
            }
            if found != 0 {
                m.put_word(found + 8, 0);
            }
            let data = m.word(handle + 4);
            if m.word(data + 56) != 0 {
                crate::boot_part_d::fn_1730fc(m, m.word(data + 56), &mut Calls(h));
                m.put_word(m.word(handle + 4) + 56, 0);
            }
            crate::boot_part_b::fn_17ca94(m, m.word(handle + 4), &mut Calls(h));
            m.put_word(handle + 4, 0);
        }
    } else if m.word(handle) != 0 {
        rc = crate::boot_part_d::fn_1730fc(m, m.word(handle), &mut Calls(h));
        m.put_word(handle, 0);
    }
    if rc >= 0x80000000 {
        let mut saved = [0u8; 64];
        saved.copy_from_slice(&m.bytes[k.scratch as usize..k.scratch as usize + 64]);
        for (i, v) in [k.handle_key, handle, k.status_key, rc]
            .into_iter()
            .enumerate()
        {
            m.put_word(k.scratch + i as u32 * 4, v);
        }
        crate::render_part_a::error_log(
            m,
            h,
            m.word(ctx),
            k.category,
            0x118,
            k.error,
            k.scratch,
            k.scratch + 32,
        );
        m.bytes[k.scratch as usize..k.scratch as usize + 64].copy_from_slice(&saved);
    }
    crate::boot_part_e::release(m, heap, handle, h);
    info(m, h, ctx, k, k.end, &[rc]);
    rc
}

pub trait NormalizerHost: crate::render_part_a2::RenderHost {

    fn normalize_core(&mut self, m: &mut [u8], args: &[u32]) -> u32;
}
impl<H: crate::render_part_a3::RegionHost> NormalizerHost for H {
    fn normalize_core(&mut self, m: &mut [u8], a: &[u32]) -> u32 {
        self.normalize_kernel(&mut Memory {base:0,bytes:m},a.try_into().unwrap())
    }
}
fn sw(m: &[u8], p: u32) -> u32 {
    u32::from_le_bytes(m[p as usize..p as usize + 4].try_into().unwrap())
}
fn sh(m: &[u8], p: u32) -> u16 {
    u16::from_le_bytes(m[p as usize..p as usize + 2].try_into().unwrap())
}
fn spw(m: &mut [u8], p: u32, v: u32) {
    m[p as usize..p as usize + 4].copy_from_slice(&v.to_le_bytes());
}
fn sph(m: &mut [u8], p: u32, v: u16) {
    m[p as usize..p as usize + 2].copy_from_slice(&v.to_le_bytes());
}
fn stream_entry(m: &[u8], p: u32, i: u32) -> Result<u32, u32> {
    if p == 0 {
        return Err(0x81102007);
    }
    let t = sw(m, p + 12);
    let mut out = 0;
    let rc = crate::normalization::entry_offset(
        sh(m, t + 2),
        sw(m, t + 4) as i32,
        sh(m, t + 8),
        sh(m, p + 16),
        i,
        &mut out,
    );
    if rc >= 0x80000000 {
        Err(rc)
    } else {
        Ok(out as u32)
    }
}
fn stream_count(m: &[u8], p: u32) -> Result<u16, u32> {
    if p == 0 {
        return Err(0x81102007);
    }
    let t = sw(m, p + 12);
    let mut n = 0;
    let rc = crate::normalization::inner_field(Some(&m[t as usize..]), Some(&mut n));
    if rc >= 0x80000000 {
        Err(rc)
    } else {
        Ok(n)
    }
}
fn stream_index(m: &mut [u8], p: u32, i: u32) -> u32 {
    if p == 0 {
        return 0x81102007;
    }
    let t = sw(m, p + 12);
    let mut out = sh(m, p + 16);
    let rc = crate::normalization::set_index(sh(m, t + 8), i as i32, &mut out);
    if rc < 0x80000000 {
        sph(m, p + 16, out);
    }
    rc
}

pub fn fn_254aac(
    m: &mut [u8],
    a: [u32; 5],
    constants: u32,
    scratch: u32,
    h: &mut impl NormalizerHost,
) -> u32 {
    let mut saved = [0u8; 128];
    saved.copy_from_slice(&m[scratch as usize..scratch as usize + 128]);
    let rc = normalize_inner(m, a, constants, scratch, h);
    m[scratch as usize..scratch as usize + 128].copy_from_slice(&saved);
    rc
}
fn normalize_inner<H: NormalizerHost>(
    m: &mut [u8],
    a: [u32; 5],
    constants: u32,
    s: u32,
    h: &mut H,
) -> u32 {
    use crate::normalization as n;
    let [obj, text, markers, _unused, data] = a;
    macro_rules! get {
        ($e:expr) => {
            match $e {
                Ok(p) => p,
                Err(r) => return r,
            }
        };
    }
    if obj == 0 || text == 0 || markers == 0 {
        return 0x81102007;
    }
    let mut base = get!(stream_entry(m, markers, 0));
    let mut count = get!(stream_count(m, markers));
    let origin = get!(stream_entry(m, markers, 0));
    let textbase = get!(stream_entry(m, text, 0));
    let current = get!(stream_entry(m, markers, 0xffff));

    let t = sw(m, markers + 12);
    let mut index = 0;
    let rc = n::current_index(
        Some(&m[t as usize..]),
        sh(m, markers + 16),
        Some(&mut index),
    );
    if rc >= 0x80000000 {
        return rc;
    }
    let mut segments = sh(m, current + 8);
    let original_name = sw(m, current + 12);
    let floatword = sw(m, current + 28);
    let heap = sw(m, sw(m, obj + 8) + 4);
    let mut gaps = 0;
    let mut positions = 0;
    let mut leading = 0u16;
    let mut event = [0u8; 40];
    event.copy_from_slice(&m[constants as usize + 0x170..constants as usize + 0x198]);
    m[s as usize + 32..s as usize + 72].copy_from_slice(&event);
    if m[obj as usize + 82] == 1 && segments != 0 {
        gaps = f::call_method(m, nz(heap), segments as u32 + 1, 3, h);
        if gaps == 0 {
            return 0x8110200a;
        }
        positions = f::call_method(m, nz(heap), segments as u32 + 1, 9, h);
        if positions == 0 {
            return 0x8110200a;
        }
        let p = sw(m, current + 4);
        let mut total = 0u16;
        for i in 0..segments as u32 {
            spw(m, positions + i * 8, sw(m, p + i * 8));
            spw(m, positions + i * 8 + 4, sw(m, p + i * 8 + 4));
        }
        for i in 0..segments as u32 {
            if i == 0 && sw(m, p) > sw(m, current + 12) {
                leading = (sw(m, p) >> 1) as u16;
                total = total.wrapping_add(leading);
            }
            let gap = if i + 1 < segments as u32 {
                let v = (sw(m, p + i * 8 + 8)
                    .wrapping_sub(sw(m, p + i * 8))
                    .wrapping_sub(sw(m, p + i * 8 + 4))
                    >> 1) as u16;
                total = total.wrapping_add(v);
                v
            } else {
                ((sw(m, current + 24) >> 1) as u16).wrapping_sub(total)
            };
            sph(m, gaps + i * 2, gap);
        }
        if leading != 0 {
            f::move_bytes(m, gaps + 2, gaps, segments as u32 * 2);
            sph(m, gaps, leading);
            segments = segments.wrapping_add(1);
        }
    }
    let mut offset = ((sw(m, current + 20).wrapping_sub(sw(m, origin + 20)) as u16) >> 1) as u16;
    let mut end = offset.wrapping_add((sw(m, current + 24) as u16) >> 1);
    sph(m, s, 0);
    sph(m, s + 2, 0);
    m[s as usize + 4] = 0;
    let call = |m: &mut [u8], h: &mut H, base: u32, count: u16, off: u16, target: u16| {
        h.normalize_core(
            m,
            &[
                obj,
                text,
                markers,
                data,
                origin,
                base,
                count as u32,
                textbase,
                floatword,
                current,
                off as u32,
                target as u32,
                s,
                s + 4,
                s + 2,
            ],
        )
    };
    if m[obj as usize + 82] != 0 && segments != 0 {
        let mut failed = false;
        for i in 0..segments as u32 {
            let target = offset.wrapping_add(sh(m, gaps + i * 2));
            let rc = call(m, h, base, count, offset, target);
            if rc >= 0x80000000 {
                failed = true;
                break;
            }
            offset = target.wrapping_add(sh(m, s));
            end = end.wrapping_add(sh(m, s));
            if m[s as usize + 4] == 1 && i + 1 < segments as u32 {
                let at = offset.wrapping_sub(sh(m, s + 2));
                let operation = (|| -> Result<(), u32> {
                    let rc = stream_index(m, text, at as u32);
                    if rc >= 0x80000000 {
                        return Err(rc);
                    }
                    let rc = n::insert_elements(m, text, constants + 8, 1, h);
                    if rc >= 0x80000000 {
                        return Err(rc);
                    }
                    base = stream_entry(m, markers, 0)?;
                    count = stream_count(m, markers)?;
                    for j in 1..count as u32 {
                        let p = base + j * 40;
                        if sw(m, base + 20).wrapping_add(at as u32 * 2) <= sw(m, p + 20) {
                            spw(m, p + 20, sw(m, p + 20).wrapping_add(2));
                        }
                    }
                    spw(m, base + 24, sw(m, base + 24).wrapping_add(2));
                    let rc = stream_index(m, markers, 1);
                    if rc >= 0x80000000 {
                        return Err(rc);
                    }
                    let ev = s + 32;
                    spw(m, ev + 28, 1);
                    spw(m, ev + 16, 0);
                    spw(m, ev + 24, 2);
                    let name = if leading != 0 {
                        sw(m, positions + i * 8).wrapping_add(original_name)
                    } else {
                        sw(m, positions + i * 8 + 8).wrapping_add(sw(m, base + 12))
                    };
                    spw(m, ev + 12, name);
                    spw(m, ev + 20, sw(m, base + 20).wrapping_add(at as u32 * 2));
                    let rc = n::insert_ordered(m, markers, ev, h);
                    if rc >= 0x80000000 {
                        return Err(rc);
                    }
                    base = stream_entry(m, markers, 0)?;
                    count = stream_count(m, markers)?;
                    offset = at.wrapping_add(sh(m, s + 2)).wrapping_add(1);
                    end = end.wrapping_add(1);
                    Ok(())
                })();
                if operation.is_err() {
                    failed = true;
                    break;
                }
            }
        }
        if !failed {
            call(m, h, base, count, offset, end);
        }
    } else {
        call(m, h, base, count, offset, end);
    }
    f::heap_release(m, nz(heap), nz(gaps), h);
    f::heap_release(m, nz(heap), nz(positions), h);
    0
}

pub trait OpenHost:
    CloseHost
    + crate::render_part_a::ReaderHost
    + crate::boot_part_b::DataHost
    + f::HeapVtable
    + f::MethodTable
    + crate::log_live::LogMethodsM
{
}
impl<T> OpenHost for T where
    T: CloseHost
        + crate::render_part_a::ReaderHost
        + crate::boot_part_b::DataHost
        + f::HeapVtable
        + f::MethodTable
        + crate::log_live::LogMethodsM
{
}
pub struct OpenRuntime {
    pub close: CloseRuntime,
    pub unmap: crate::boot_part_e::UnmapEnvironment,
    pub reader: crate::render_part_a::ReaderRuntime,
    pub context: crate::render_part_c::Environment,
    pub broker: crate::boot_part_b::DataOpenGlobals,
    pub array_methods: [u32; 3],
    pub rb: u32,
    pub big_endian: bool,
    pub begin: u32,
    pub end: u32,
    pub file_key: u32,
    pub mode_key: u32,
    pub version_key: u32,
    pub product_key: u32,
    pub error_format: u32,
    pub version_format: u32,

    pub scratch: u32,
}
fn literal(m: &mut Memory<'_>, p: u32, b: &[u8]) {
    for (i, &v) in b.iter().chain(core::iter::once(&0)).enumerate() {
        m.put_byte(p + i as u32, v);
    }
}
fn equals(m: &Memory<'_>, p: u32, b: &[u8]) -> bool {
    b.iter()
        .enumerate()
        .all(|(i, v)| m.byte(p + i as u32) == *v)
        && m.byte(p + b.len() as u32) == 0
}
fn open_log(
    m: &mut Memory<'_>,
    h: &mut impl OpenHost,
    env: u32,
    k: &OpenRuntime,
    fmt: u32,
    args: &[u32],
) {
    for (i, &v) in args.iter().enumerate() {
        m.put_word(k.scratch + 0x180 + i as u32 * 4, v);
    }
    crate::log_live::fn_1738e8(
        m,
        m.word(env + 16),
        k.reader.category,
        4,
        0,
        fmt,
        k.scratch + 0x180,
        h,
    );
}
fn open_error(
    m: &mut Memory<'_>,
    h: &mut impl OpenHost,
    env: u32,
    k: &OpenRuntime,
    code: u32,
    fmt: u32,
    args: &[u32],
) {
    for (i, &v) in args.iter().enumerate() {
        m.put_word(k.scratch + 0x180 + i as u32 * 4, v);
    }
    crate::render_part_a::error_log(
        m,
        h,
        m.word(env + 16),
        k.reader.category,
        code,
        fmt,
        k.scratch + 0x180,
        k.scratch + 0x1c0,
    );
}

pub fn fn_242fd0(
    m: &mut Memory<'_>,
    args: [u32; 9],
    k: &OpenRuntime,
    h: &mut impl OpenHost,
) -> u32 {
    assert_eq!(m.base, 0);
    let mut saved = [0u8; 1024];
    saved.copy_from_slice(&m.bytes[k.scratch as usize..k.scratch as usize + 1024]);
    let rc = open_inner(m, args, k, h);
    m.bytes[k.scratch as usize..k.scratch as usize + 1024].copy_from_slice(&saved);
    rc
}
fn close_open_record(m: &mut Memory<'_>, rec: u32, k: &OpenRuntime, h: &mut impl OpenHost) -> u32 {

    if m.byte(rec + 4) != 0 {
        return crate::normalization::close_owner_pair(m.bytes, rec, h);
    }
    let broker = m.word(m.word(rec) + 12);
    let handle = m.word(rec + 8);
    if handle != 0 && m.word(rec + 16) != 0 {
        let rc = crate::boot_part_e::fn_179208(
            m,
            broker,
            handle,
            m.word(rec + 16),
            &k.unmap,
            &mut Calls(h),
        );
        for off in [16, 20, 24] {
            m.put_word(rec + off, 0);
        }
        if rc >= 0x80000000 {
            return rc;
        }
    }
    if handle == 0 && m.word(rec + 12) != 0 && m.word(rec + 16) != 0 {
        let rc =
            crate::boot_part_d::fn_17331c(m, m.word(rec + 12), m.word(rec + 16), &mut Calls(h));
        for off in [16, 20, 24] {
            m.put_word(rec + off, 0);
        }
        if rc >= 0x80000000 {
            return rc;
        }
    }
    let handle = m.word(rec + 8);
    if handle != 0 {
        let rc = fn_179f3c(m, m.word(m.word(rec) + 12), handle, &k.close, h);
        m.put_word(rec + 8, 0);
        if rc >= 0x80000000 {
            return rc;
        }
    }
    if m.word(rec + 12) != 0 {
        let rc = crate::boot_part_d::fn_1730fc(m, m.word(rec + 12), &mut Calls(h));
        m.put_word(rec + 12, 0);
        return rc;
    }
    crate::normalization::close_owner_pair(m.bytes, rec, h)
}
fn close_open_reader(m: &mut Memory<'_>, p: u32, k: &OpenRuntime, h: &mut impl OpenHost) {
    let env = m.word(p + 8);
    open_log(m, h, env, k, k.reader.messages[2], &[p]);
    let rc = close_open_record(m, p + 24, k, h);
    let name = m.word(p + 16);
    if name != 0 {
        f::heap_release(m.bytes, nz(m.word(m.word(p + 8) + 4)), nz(name), h);
    }
    if m.word(p + 20) != 0 {
        crate::render_part_a::reader_release_array(m, h, m.word(p + 20));
    }
    if rc < 0x80000000 {
        f::heap_release(m.bytes, nz(m.word(m.word(p + 8) + 4)), nz(p), h);
    }
    open_log(m, h, env, k, k.reader.messages[3], &[rc]);
}
fn open_inner(m: &mut Memory<'_>, a: [u32; 9], k: &OpenRuntime, h: &mut impl OpenHost) -> u32 {
    use crate::{normalization as n, render_part_a as r};
    let [owner, tag, mode, filename, mapped, mapped_len, kind, version, out] = a;
    let (typ, quiet) = match mode {
        0 => (0, 0),
        1 => (1, 0),
        2 => (0, 1),
        3 => (1, 1),
        4 => (4, 0),
        _ => return 0x8a202007,
    };
    if filename == 0 || m.byte(filename) == 0 || kind == 0 {
        return 0x8a202007;
    }
    if typ == 0 && m.byte(kind) == 0 || typ == 4 && (mapped == 0 || mapped_len == 0) || out == 0 {
        return 0x8a202007;
    }
    m.put_word(out, 0);
    let cached = [
        u32::from_le_bytes(k.context.cached_handle[..4].try_into().unwrap()),
        u32::from_le_bytes(k.context.cached_handle[4..].try_into().unwrap()),
    ];
    let pointed = if [owner, tag] != cached && owner != 0 && tag == 0x1e61 {
        Some(u16::from_le_bytes([m.byte(owner), m.byte(owner + 1)]))
    } else {
        None
    };
    let mut env = 0;
    let mut rc = f::resolve_engine_context(
        owner,
        pointed,
        tag,
        &k.context.cached_handle,
        k.context.default_context,
        &mut env,
    );
    if rc >= 0x80000000 {
        return rc;
    }
    let major = version & 255;
    let minor = version >> 8 & 255;
    let patch = version >> 16 & 255;
    open_log(
        m,
        h,
        env,
        k,
        k.begin,
        &[mode, filename, kind, major, minor, patch],
    );
    let s = k.scratch;
    let rec = s;
    f::fill_bytes(m.bytes, rec, 0, 28);
    m.put_word(rec, env);
    let heap = m.word(env + 4);
    match typ {
        0 => {
            rc = crate::boot_part_b::fn_1795f0(
                m,
                m.word(env + 12),
                filename,
                quiet,
                rec + 8,
                &k.broker,
                h,
            );
            if rc >= 0x80000000 {
                return rc;
            }
        }
        1 => {
            rc = crate::boot_part_d::fn_173000(
                m,
                m.word(env + 28),
                heap,
                filename,
                rec + 12,
                k.rb,
                &mut Calls(h),
            );
            if rc >= 0x80000000 {
                if quiet == 1 {
                    open_log(m, h, env, k, k.begin, &[filename, rc]);
                } else {
                    open_error(
                        m,
                        h,
                        env,
                        k,
                        0x697a,
                        k.error_format,
                        &[k.file_key, filename, k.mode_key, rc],
                    );
                }
                return rc;
            }
        }
        4 => {
            m.put_word(rec + 16, mapped);
            m.put_word(rec + 24, mapped_len);
        }
        _ => unreachable!(),
    }
    let p = f::call_method(m.bytes, nz(heap), 1, 56, h);
    if p == 0 {
        open_error(m, h, env, k, 0x6978, 0, &[]);
        close_open_record(m, rec, k, h);
        return 0x8a20200a;
    }
    m.put_word(p, owner);
    m.put_word(p + 4, tag);
    m.put_word(p + 8, env);
    m.put_byte(p + 12, typ as u8);
    for off in (0..28).step_by(4) {
        m.put_word(p + 24 + off, m.word(rec + off));
    }
    let name = f::heap_alloc(
        m.bytes,
        nz(heap),
        f::strlen_bytes(&m.bytes[filename as usize..]) + 1,
        h,
    );
    m.put_word(p + 16, name);
    let id = s + 32;
    let len = s + 48;
    let data = s + 52;
    let chunk = s + 56;
    let buf = s + 64;
    let cap = s + 80;
    let tmp = s + 96;
    let num = s + 128;
    let end = s + 164;
    literal(m, buf, b"");
    m.put_word(cap, 16);
    let mut success = false;
    'operation: {
        if name == 0 {
            open_error(m, h, env, k, 0x6978, 0, &[]);
            rc = 0x8a20200a;
            break 'operation;
        }
        f::copy_str(m.bytes, name, filename);
        let mut array = 0;
        let ok = f::create_block(
            h,
            heap,
            k.array_methods[0],
            k.array_methods[1],
            k.array_methods[2],
            4,
            m.bytes,
            Some(&mut array),
        );
        m.put_word(p + 20, array);
        if ok == 0 {
            open_error(m, h, env, k, 0x6978, 0, &[]);
            rc = 0x8a20200a;
            break 'operation;
        }
        rc = r::open_reader_chunk(m, h, p, chunk, len, 0, &k.reader);
        if rc >= 0x80000000 {
            break 'operation;
        }
        rc = r::reader_chunk_data(m, h, p, 4, data, &k.reader);
        if rc >= 0x80000000 {
            break 'operation;
        }
        let be = k.big_endian;
        let alignment = if equals(m, chunk, if be { b"IFF " } else { b"RIFF" })
            || be && equals(m, chunk, b"RIFX")
        {
            2
        } else if equals(m, chunk, if be { b"IFF4" } else { b"RIF4" }) {
            4
        } else if equals(m, chunk, if be { b"IFF8" } else { b"RIF8" }) {
            8
        } else {
            u16::from_le_bytes([m.byte(p + 14), m.byte(p + 15)])
        };
        if alignment != 0 {
            m.put_half(p + 14, alignment);
        }
        if alignment == 0 || m.word(data) == 0 || m.word(len) < 4 {
            open_error(
                m,
                h,
                env,
                k,
                0x697c,
                k.error_format,
                &[k.file_key, m.word(p + 16)],
            );
            rc = 0x8a202004;
            break 'operation;
        }
        let src = m.word(data);
        crate::selection::arena_leaf::copy_bytes(
            &mut crate::selection::arena::Arena::new(0, m.bytes),
            id,
            src,
            4,
        );
        m.put_byte(id + 4, 0);
        if m.byte(p + 13) == 1 {
            for i in 0..4 {
                m.put_byte(id + i, m.byte(id + i) ^ 0xa9);
            }
        }
        if m.byte(kind) != 0 {
            if f::compare_str(&m.bytes[kind as usize..], &m.bytes[id as usize..]) != 0 {
                open_error(
                    m,
                    h,
                    env,
                    k,
                    0x697d,
                    k.error_format,
                    &[k.file_key, m.word(p + 16)],
                );
                rc = 0x8a202004;
                break 'operation;
            }
        } else {
            f::copy_str(m.bytes, kind, id);
        }
        m.put_word(p + 52, m.word(p + 52).wrapping_add(4));
        if typ != 0 && major == 0 {
            m.put_word(out, p);
            success = true;
            break 'operation;
        }
        rc = r::open_reader_chunk(m, h, p, chunk, len, data, &k.reader);
        if rc >= 0x80000000 || !equals(m, chunk, b"LIST") || m.word(len) <= 4 || m.word(data) == 0 {
            open_error(
                m,
                h,
                env,
                k,
                0x697e,
                k.error_format,
                &[k.file_key, m.word(p + 16)],
            );
            rc = 0x8a202004;
            break 'operation;
        }
        let src = m.word(data);
        crate::selection::arena_leaf::copy_bytes(
            &mut crate::selection::arena::Arena::new(0, m.bytes),
            id,
            src,
            4,
        );
        m.put_byte(id + 4, 0);
        if m.byte(p + 13) == 1 {
            for i in 0..4 {
                m.put_byte(id + i, m.byte(id + i) ^ 0xa9);
            }
        }
        if !equals(m, id, b"INFO") || (m.word(p + 52).wrapping_add(4) as i32) < 0 {
            open_error(
                m,
                h,
                env,
                k,
                0x697e,
                k.error_format,
                &[k.file_key, m.word(p + 16)],
            );
            rc = 0x8a202004;
            break 'operation;
        }
        m.put_word(p + 52, m.word(p + 52).wrapping_add(4));
        literal(m, id, b"VERS");
        rc = r::find_reader_chunk(m, h, p, id, 3, len, data, &k.reader);
        if rc < 0x80000000 {
            let src = m.word(data);
            let length = m.word(len);
            rc = r::read_chunk_string(
                &mut crate::selection::arena::Arena::new(0, m.bytes),
                p,
                src,
                length,
                0,
                buf,
                cap,
            );
        }
        if rc >= 0x80000000 {
            open_error(
                m,
                h,
                env,
                k,
                0x697e,
                k.error_format,
                &[k.file_key, m.word(p + 16)],
            );
            rc = 0x8a202004;
            break 'operation;
        }
        let fm = n::strtoul_scan(m.bytes, buf, end, 10, 0) as i32;
        let ep = m.word(end);
        let mut valid = fm >= 1 && ep != 0 && m.byte(ep) == b'.' && m.byte(ep + 1) != 0;
        let mut fn_ = 0i32;
        let mut fp = 0i32;
        if valid {
            fn_ = n::strtoul_scan(m.bytes, ep + 1, end, 10, 0) as i32;
            let ep = m.word(end);
            valid = fn_ >= 0 && ep != 0 && m.byte(ep) == b'.' && m.byte(ep + 1) != 0;
            if valid {
                fp = n::strtoul_scan(m.bytes, ep + 1, end, 10, 0) as i32;
                let ep = m.word(end);
                valid = fp >= 0 && ep != 0 && (m.byte(ep) == 0 || m.byte(ep) == b'.');
            }
        }
        if !valid {
            open_error(
                m,
                h,
                env,
                k,
                0x6980,
                k.error_format,
                &[k.version_key, m.word(p + 16)],
            );
            rc = 0x8a202004;
            break 'operation;
        }
        if major < fm as u32
            || major == fm as u32
                && (minor < fn_ as u32 || minor == fn_ as u32 && patch < fp as u32)
        {
            n::itoa(m.bytes, major, num, 10);
            f::copy_str(m.bytes, tmp, num);
            for v in [minor, patch] {
                literal(m, id, b".");
                f::append_str(m.bytes, tmp, id);
                n::itoa(m.bytes, v, num, 10);
                f::append_str(m.bytes, tmp, num);
            }
            open_error(
                m,
                h,
                env,
                k,
                0x6981,
                k.version_format,
                &[k.product_key, tmp, k.version_key, m.word(p + 16)],
            );
            rc = 0x8a202004;
            break 'operation;
        }
        rc = r::close_reader_chunk(m, h, p, &k.reader);
        if rc >= 0x80000000 {
            break 'operation;
        }
        rc = r::close_reader_chunk(m, h, p, &k.reader);
        if rc >= 0x80000000 {
            break 'operation;
        }
        m.put_word(out, p);
        success = true;
    }
    if !success {
        close_open_reader(m, p, k, h);
    }
    open_log(m, h, env, k, k.end, &[rc, m.word(out)]);
    rc
}

pub struct PipelineStageData<'a> {
    pub constants: u32,
    pub unit_prefix: u32,
    pub language: u32,
    pub category: u32,
    pub formats: [u32; 6],
    pub labels: [u32; 2],
    pub scratch: u32,
    pub fold: &'a [u16; 256],
    pub environment: &'a crate::render_part_c::Environment,
    pub regex: Option<&'a RefCell<crate::render_part_c::RegexData<'a, 'a>>>,
}
pub fn stage_calloc(
    m: &mut Memory<'_>,
    heap: u32,
    n: u32,
    size: u32,
    h: &mut impl crate::render_part_c::Host,
) -> u32 {
    if heap == 0 || n == 0 || size == 0 {
        return 0;
    }
    let method = m.word(m.word(heap) + 4);
    let mut header = [0u8; 8];
    header[4..].copy_from_slice(&m.word(heap + 4).to_le_bytes());
    struct Call<'a, 'b, H> {
        m: &'a mut Memory<'b>,
        h: &'a mut H,
        method: u32,
    }
    impl<H: crate::render_part_c::Host> f::MethodTable for Call<'_, '_, H> {
        fn call_at(&mut self, obj: u32, n: u32, size: u32) -> u32 {
            crate::render_part_c::Host::call(self.h, self.m, self.method, &[obj, n, size])
        }
    }
    f::call_method(&header, Some(0), n, size, &mut Call { m, h, method })
}
struct StageAllocator<'a, H> {
    host: &'a mut H,
    target: u32,
}
impl<H: crate::render_part_c::Host> f::HeapVtableM for StageAllocator<'_, H> {
    fn alloc_at_m(&mut self, o: u32, n: u32, m: &mut [u8]) -> u32 {
        crate::render_part_c::Host::call(
            self.host,
            &mut Memory { base: 0, bytes: m },
            self.target,
            &[o, n],
        )
    }
    fn realloc_at_m(&mut self, o: u32, p: u32, n: u32, m: &mut [u8]) -> u32 {
        crate::render_part_c::Host::call(
            self.host,
            &mut Memory { base: 0, bytes: m },
            self.target,
            &[o, p, n],
        )
    }
    fn release_at_m(&mut self, o: u32, p: u32, m: &mut [u8]) {
        crate::render_part_c::Host::call(
            self.host,
            &mut Memory { base: 0, bytes: m },
            self.target,
            &[o, p],
        );
    }
}
pub fn stage_allocate(
    m: &mut Memory<'_>,
    heap: u32,
    n: u32,
    h: &mut impl crate::render_part_c::Host,
) -> u32 {
    if heap == 0 || n == 0 {
        return 0;
    }
    let target = m.word(m.word(heap));
    f::heap_alloc_m(
        m.bytes,
        Some(heap),
        n,
        &mut StageAllocator { host: h, target },
    )
}
pub fn stage_resize(
    m: &mut Memory<'_>,
    heap: u32,
    p: u32,
    n: u32,
    h: &mut impl crate::render_part_c::Host,
) -> u32 {
    if heap == 0 || n == 0 {
        return 0;
    }
    let target = m.word(m.word(heap) + if p == 0 { 0 } else { 8 });
    f::heap_realloc_m(
        m.bytes,
        Some(heap),
        p,
        n,
        &mut StageAllocator { host: h, target },
    )
}
pub fn stage_release(
    m: &mut Memory<'_>,
    heap: u32,
    p: u32,
    h: &mut impl crate::render_part_c::Host,
) {
    if heap == 0 || p == 0 {
        return;
    }
    let target = m.word(m.word(heap) + 12);
    f::heap_release_m(
        m.bytes,
        Some(heap),
        Some(p),
        &mut StageAllocator { host: h, target },
    );
}
fn stage_free(
    m: &mut Memory<'_>,
    ctx: u32,
    p: u32,
    h: &mut impl crate::render_part_d2::PipelineHost,
) {
    let heap = m.word(ctx + 4);
    stage_release(m, heap, p, h);
}
struct StageServices<'a, H>(&'a mut H);
impl<H: crate::render_part_d2::PipelineHost> crate::boot_part_b::Host for StageServices<'_, H> {
    fn call(&mut self, m: &mut Memory<'_>, f: u32, a: &[u32]) -> u32 {
        match f {
            0x196394 => stage_allocate(m, a[0], a[1], self.0),
            0x1963e0 => stage_resize(m, a[0], a[1], a[2], self.0),
            0x1963b8 => stage_calloc(m, a[0], a[1], a[2], self.0),
            0x196428 => {
                stage_release(m, a[0], a[1], self.0);
                0
            }
            _ => crate::render_part_c::Host::call(self.0, m, f, a),
        }
    }
    fn thread_id(&mut self) -> u32 {
        panic!("stage formatting does not acquire a thread lock")
    }
}
impl<H: crate::render_part_d2::PipelineHost> crate::boot_part_b::FormatHost
    for StageServices<'_, H>
{
}
fn stage_info(
    m: &mut Memory<'_>,
    ctx: u32,
    k: &PipelineStageData<'_>,
    format: usize,
    args: &[u32],
    h: &mut impl crate::render_part_d2::PipelineHost,
) {
    for (i, v) in args.iter().enumerate() {
        m.put_word(k.scratch + 128 + i as u32 * 4, *v);
    }
    crate::log_live::fn_1738e8(
        m,
        m.word(ctx + 16),
        k.category,
        5,
        0,
        k.formats[format],
        k.scratch + 128,
        h,
    );
}
fn stage_error(
    m: &mut Memory<'_>,
    ctx: u32,
    k: &PipelineStageData<'_>,
    code: u32,
    format: u32,
    args: &[u32],
    h: &mut impl crate::render_part_d2::PipelineHost,
) {
    for (i, v) in args.iter().enumerate() {
        m.put_word(k.scratch + 128 + i as u32 * 4, *v);
    }
    crate::render_part_a::error_log(
        m,
        &mut StageServices(h),
        m.word(ctx + 16),
        k.category,
        code,
        format,
        k.scratch + 128,
        k.scratch + 160,
    );
}
struct StageProcessor<'a, H> {
    host: &'a mut H,
    one: u32,
}
impl<H: crate::render_part_d2::PipelineHost> f::HeapVtable for StageProcessor<'_, H> {
    fn alloc_at(&mut self, o: u32, n: u32) -> u32 {
        self.host.alloc_at(o, n)
    }
    fn realloc_at(&mut self, o: u32, p: u32, n: u32) -> u32 {
        self.host.realloc_at(o, p, n)
    }
    fn release_at(&mut self, o: u32, p: u32) {
        self.host.release_at(o, p)
    }
}
impl<H: crate::render_part_d2::PipelineHost> f::MethodTable for StageProcessor<'_, H> {
    fn call_at(&mut self, o: u32, n: u32, s: u32) -> u32 {
        self.host.call_at(o, n, s)
    }
}
impl<H: crate::render_part_d2::PipelineHost> crate::normalization::NormalizationHost
    for StageProcessor<'_, H>
{
    fn calloc(&mut self, m: &mut [u8], heap: u32, n: u32, s: u32) -> Option<u32> {
        self.host.calloc(m, heap, n, s)
    }
    fn alloc(&mut self, m: &mut [u8], heap: u32, n: u32) -> Option<u32> {
        self.host.alloc(m, heap, n)
    }
    fn realloc(&mut self, m: &mut [u8], heap: u32, p: u32, n: u32) -> Option<u32> {
        self.host.realloc(m, heap, p, n)
    }
    fn free(&mut self, m: &mut [u8], heap: u32, p: u32) {
        self.host.free(m, heap, p)
    }
    fn resolve(&mut self, m: &mut [u8], a: u32, b: u32) -> (u32, u32) {
        self.host.resolve(m, a, b)
    }
}
impl<H: crate::render_part_d2::PipelineHost> crate::selection::arena::Host
    for StageProcessor<'_, H>
{
    fn call(
        &mut self,
        m: &mut crate::selection::arena::Arena<'_>,
        f: u32,
        a: &[u32],
    ) -> (u32, u32) {
        let mut m = Memory {
            base: m.base,
            bytes: m.bytes,
        };
        (crate::render_part_c::Host::call(self.host, &mut m, f, a), 0)
    }
}
impl<H: crate::render_part_d2::PipelineHost> crate::render_part_a2::RenderHost
    for StageProcessor<'_, H>
{
    fn process(
        &mut self,
        m: &mut [u8],
        vt: u32,
        c: &crate::render_part_a2::ProcessCall,
    ) -> (u32, u32) {
        spw(m, self.one, 1);
        let method = sw(m, vt + 64);
        let rc = crate::render_part_c::Host::call(
            self.host,
            &mut Memory { base: 0, bytes: m },
            method,
            &[c.x, c.y, c.flag, c.b1, self.one, c.b2, c.h2, c.out],
        );
        (sw(m, self.one), rc)
    }
}

pub fn fn_2f0d3c(
    m: &mut Memory<'_>,
    a: [u32; 14],
    k: &PipelineStageData<'_>,
    h: &mut impl crate::render_part_d2::PipelineHost,
) -> u32 {
    assert_eq!(m.base, 0);
    let mut saved = [0u8; 512];
    saved.copy_from_slice(&m.bytes[k.scratch as usize..k.scratch as usize + 512]);
    let r = convert_stage(m, a, k, h);
    m.bytes[k.scratch as usize..k.scratch as usize + 512].copy_from_slice(&saved);
    r
}
fn convert_stage(
    m: &mut Memory<'_>,
    a: [u32; 14],
    k: &PipelineStageData<'_>,
    h: &mut impl crate::render_part_d2::PipelineHost,
) -> u32 {
    use crate::normalization as n;
    let [debug, ctx, plugin, x, y, h0, h1, text, events, object, language, outmax, inmax, scale] =
        a;
    let s = k.scratch;
    if plugin == 0 {
        return 0;
    }
    m.put_byte(s + 20, 0);
    let r = crate::render_part_c::Host::call(h, m, m.word(plugin + 60), &[x, y, s + 20]);
    if r >= 0x80000000 {
        return r;
    }
    if m.byte(s + 20) == 0 {
        return 0;
    }
    let mut table = 0;
    let r = n::fetch_entry(m.bytes, object, 0x3fd, 1, Some(&mut table), h);
    if r >= 0x80000000 {
        return r;
    }
    let mut textbase = match stream_entry(m.bytes, text, 0) {
        Ok(p) => p,
        Err(r) => return r,
    };
    let mut records = m.word(table + 4);
    let mut count = sh(m.bytes, table + 8);
    if count == 0 || sh(m.bytes, records + 10) != 0x4000 {
        return 0x81102000;
    }
    crate::fe_text::memcpy_195ae8(m.bytes, s + 32, k.constants + 0x78, 40);
    crate::fe_text::memcpy_195ae8(m.bytes, s + 72, k.constants + 0xa0, 40);
    m.put_word(s, 0);
    m.put_word(s + 8, 0);
    m.put_word(s + 16, 0);
    let mut names = 0;
    let mut debugbuf = 0;
    let mut last = None;
    let mut bypass = false;
    let outcome = (|| -> Result<(), u32> {
        macro_rules! ok {
            ($e:expr) => {{
                let r = $e;
                if r >= 0x80000000 {
                    return Err(r);
                }
            }};
        }
        let mut i = 1u16;
        while i < count {
            let current = records + i as u32 * 40;
            if sh(m.bytes, current + 10) != 0x20
                || m.word(current + 28) != 1
                || last == Some(m.word(current + 20))
            {
                i = i.wrapping_add(1);
                continue;
            }
            let position = m.word(current + 20);
            last = Some(position);
            let mut j = i.wrapping_add(1);
            while j < count
                && sh(m.bytes, records + j as u32 * 40 + 10) != 0x20
                && m.word(records + j as u32 * 40 + 28) == 0
            {
                j = j.wrapping_add(1);
            }
            if j >= count
                || sh(m.bytes, records + j as u32 * 40 + 10) != 0x20
                || m.word(records + j as u32 * 40 + 28) != 0
                || position == m.word(records + j as u32 * 40 + 20)
            {
                i = i.wrapping_add(1);
                continue;
            }
            ok!(stream_index(m.bytes, events, i as u32));
            let start = (position.wrapping_sub(m.word(records + 20)) as u16) >> 1;
            let finish = (m
                .word(records + j as u32 * 40 + 20)
                .wrapping_sub(m.word(records + 20)) as u16)
                >> 1;
            if debug == 1 {
                stage_info(m, ctx, k, 0, &[start as u32, finish as u32], h);
            }
            m.put_half(s + 4, inmax as u16);
            let length = finish as i32 - start as i32;
            if length > inmax as i32 {
                break;
            }
            m.put_half(s + 4, length as u16);
            if m.word(s) == 0 {
                let p = stage_calloc(m, m.word(ctx + 4), inmax.wrapping_add(1), 2, h);
                m.put_word(s, p);
                if p == 0 {
                    stage_error(m, ctx, k, 0x783, 0, &[], h);
                    return Err(0x8110200a);
                }
            }
            f::move_bytes(
                m.bytes,
                m.word(s),
                textbase.wrapping_add(start as u32 * 2),
                (length as u16) as u32 * 2,
            );
            if m.word(s + 8) == 0 {
                let bytes = outmax
                    .wrapping_add(1)
                    .wrapping_mul(scale)
                    .wrapping_mul(2)
                    .wrapping_add(2);
                let p = stage_calloc(m, m.word(ctx + 4), 1, bytes, h);
                m.put_word(s + 8, p);
                if p == 0 {
                    stage_error(m, ctx, k, 0x783, 0, &[], h);
                    return Err(0x8110200a);
                }
            }
            if names == 0 {
                ok!(n::open_iterator(
                    m.bytes,
                    h0,
                    h1,
                    object,
                    0x405,
                    Some(&mut names),
                    h
                ));
            }
            if debug == 1 {
                if debugbuf == 0 {
                    debugbuf =
                        stage_calloc(m, m.word(ctx + 4), outmax.max(inmax).wrapping_add(1), 1, h);
                    if debugbuf == 0 {
                        stage_error(m, ctx, k, 0x783, 0, &[], h);
                        return Err(0x8110200a);
                    }
                }
                n::u16_to_u8_ncopy(m.bytes, debugbuf, m.word(s), sh(m.bytes, s + 4) as u32);
                m.put_byte(debugbuf + sh(m.bytes, s + 4) as u32, 0);
                stage_info(m, ctx, k, 1, &[debugbuf], h);
            }
            ok!(crate::render_part_a2::parse_first_unit(
                m.bytes,
                plugin,
                x,
                y,
                s,
                1,
                inmax,
                outmax,
                s + 16,
                k.unit_prefix,
                &mut StageProcessor {
                    host: h,
                    one: s + 24
                }
            ));
            let kind = m.byte(s + 6);
            let convert = matches!(kind, 2 | 3 | 43)
                || (kind == 16
                    && f::compare_folded(
                        k.fold,
                        &m.bytes[language as usize..],
                        &m.bytes[k.language as usize..],
                    ) != 0);
            if convert {
                if debug == 1 {
                    n::u16_to_u8_ncopy(
                        m.bytes,
                        debugbuf,
                        m.word(s + 8),
                        sh(m.bytes, s + 12) as u32,
                    );
                    m.put_byte(debugbuf + sh(m.bytes, s + 12) as u32, 0);
                    stage_info(m, ctx, k, 2, &[debugbuf], h);
                }
                let args=[text,events,start as u32,s,0,object,m.word(s+16)];
                ok!(fn_253870(m,args,k.unit_prefix.wrapping_sub(24),k.scratch+256,&mut StageProcessor {host:h,one:k.scratch+248}));
                textbase = stream_entry(m.bytes, text, 0)?;
                let kind = m.byte(s + 6);
                if matches!(kind, 43 | 16) {
                    ok!(stream_index(m.bytes, events, i as u32));
                    let rec = s + 32;
                    m.put_word(rec + 12, m.word(records + i as u32 * 40 + 12));
                    m.put_word(rec + 16, sh(m.bytes, s + 4) as u32 * 2);
                    m.put_word(rec + 20, m.word(records + i as u32 * 40 + 20));
                    m.put_word(
                        rec + 24,
                        if kind == 43 {
                            sh(m.bytes, s + 12) as u32 * 2
                        } else {
                            m.word(records + j as u32 * 40 + 20)
                                .wrapping_sub(m.word(records + i as u32 * 40 + 20))
                        },
                    );
                    m.put_word(rec + 28, kind as u32);
                    ok!(n::insert_ordered(m.bytes, events, rec, h));
                    if kind == 16 {
                        records = stream_entry(m.bytes, events, 0)?;
                        count = stream_count(m.bytes, events)?;
                    }
                    let rec = s + 72;
                    m.put_word(rec + 12, m.word(records + i as u32 * 40 + 12));
                    m.put_word(rec + 16, sh(m.bytes, s + 4) as u32 * 2);
                    m.put_word(rec + 20, m.word(records + i as u32 * 40 + 20));
                    m.put_word(
                        rec + 24,
                        if kind == 43 {
                            sh(m.bytes, s + 12) as u32 * 2
                        } else {
                            m.word(records + j as u32 * 40 + 20)
                                .wrapping_sub(m.word(records + i as u32 * 40 + 20))
                        },
                    );
                    m.put_word(rec + 28, kind as u32);
                    let r = n::insert_ordered(m.bytes, events, rec, h);
                    if r >= 0x80000000 {
                        bypass = true;
                        return Err(r);
                    }
                    records = stream_entry(m.bytes, events, 0)?;
                    count = stream_count(m.bytes, events)?;
                }
            } else if debug == 1 {
                stage_info(m, ctx, k, 3, &[], h);
            }
            i = i.wrapping_add(1);
        }
        Ok(())
    })();
    if !bypass {
        stage_free(m, ctx, m.word(s), h);
        stage_free(m, ctx, m.word(s + 8), h);
        stage_free(m, ctx, debugbuf, h);
        if names != 0 {
            n::release_iterator(m.bytes, names, h);
        }
    }
    outcome.err().unwrap_or(0)
}

fn stage_at_end(m: &[u8], p: u32) -> Result<bool, u32> {
    if p == 0 {
        return Err(0x81102007);
    }
    Ok(sh(m, sw(m, p + 12) + 8) <= sh(m, p + 16))
}
fn stage_current_index(m: &[u8], p: u32) -> Result<u16, u32> {
    if p == 0 {
        return Err(0x81102007);
    }
    let t = sw(m, p + 12);
    let mut index = 0;
    let r = crate::normalization::current_index(
        Some(&m[t as usize..]),
        sh(m, p + 16),
        Some(&mut index),
    );
    if r >= 0x80000000 {
        Err(r)
    } else {
        Ok(index)
    }
}
fn stage_pattern(
    m: &mut [u8],
    owner: u32,
    key: u32,
    pattern: u32,
    subject: u32,
    len: u32,
    start: u32,
    k: &PipelineStageData<'_>,
    h: &mut impl crate::render_part_d2::PipelineHost,
) -> (i32, [i32; 2]) {
    if f::validate_handle(owner != 0, key, 0xdd57, 24) >= 0x80000000 {
        return (-16, [0; 2]);
    }
    let mut data = k
        .regex
        .expect("valid regex handle requires caller-owned regex workspace")
        .borrow_mut();
    let crate::render_part_c::RegexData { scratch, tables } = &mut *data;
    let mut offsets = [0i32; 30];
    let r = stage_exec_regex(
        m,
        owner,
        key,
        pattern,
        subject,
        len,
        start,
        16,
        &mut offsets,
        scratch,
        tables,
        h,
    );
    (r, [offsets[0], offsets[1]])
}

pub fn fn_2f15bc(
    m: &mut Memory<'_>,
    a: [u32; 9],
    k: &PipelineStageData<'_>,
    h: &mut impl crate::render_part_d2::PipelineHost,
) -> u32 {
    assert_eq!(m.base, 0);
    let mut saved = [0u8; 512];
    saved.copy_from_slice(&m.bytes[k.scratch as usize..k.scratch as usize + 512]);
    let result = regex_stage(m, a, k, h);
    m.bytes[k.scratch as usize..k.scratch as usize + 512].copy_from_slice(&saved);
    result
}
fn regex_stage(
    m: &mut Memory<'_>,
    a: [u32; 9],
    k: &PipelineStageData<'_>,
    h: &mut impl crate::render_part_d2::PipelineHost,
) -> u32 {
    use crate::normalization as n;
    let [h0, h1, text, events, object, patterns, owner, key, id] = a;
    let s = k.scratch;
    macro_rules! get {
        ($e:expr) => {
            match $e {
                Ok(v) => v,
                Err(r) => return r,
            }
        };
    }
    macro_rules! ok {
        ($e:expr) => {{
            let r = $e;
            if r >= 0x80000000 {
                return r;
            }
        }};
    }
    if text == 0 || events == 0 {
        return 0x81102007;
    }
    ok!(stream_index(m.bytes, text, 0));
    ok!(stream_index(m.bytes, events, 0));
    while !get!(stage_at_end(m.bytes, text)) {
        m.put_word(s + 40, m.word(k.constants + 4));
        let r = crate::render_part_a3::scan(m.bytes, events, 0x2b93c8, s + 40);
        if r >= 0x80000000 && r & 0x1fff != 0x14 {
            return r;
        }
        crate::fe_text::memcpy_195ae8(m.bytes, s, k.constants + 0x50, 40);
        let base = get!(stream_entry(m.bytes, events, 0));
        let origin = m.word(base + 20);
        let mut table = 0;
        ok!(n::fetch_entry(m.bytes, object, id, 1, Some(&mut table), h));
        let records = m.word(table + 4);
        let count = sh(m.bytes, table + 8);
        let (current, mut length) = match stream_entry(m.bytes, events, 0xffff) {
            Ok(p) => (Some(p), (m.word(p + 20).wrapping_sub(origin) >> 1) as u16),
            Err(r) if r & 0x1fff == 0xf => (None, get!(stream_count(m.bytes, text))),
            Err(r) => return r,
        };
        let mut last = 0u32;
        for i in 0..count as u32 {
            let p = records + i * 40;
            if current.is_some_and(|q| m.word(p + 20) >= m.word(q + 20)) {
                break;
            }
            if sh(m.bytes, p + 10) == 7 {
                last = i;
            }
        }
        let namebase = if last == 0 {
            m.word(records + 12)
        } else {
            m.word(records + last * 40 + 12)
                .wrapping_add(m.word(records + last * 40 + 16))
        };
        let start = get!(stage_current_index(m.bytes, text));
        length = length.wrapping_sub(start);
        ok!(stream_index(m.bytes, text, start as u32 + length as u32));
        m.put_word(s + 48, 0);
        ok!(n::insert_elements(m.bytes, text, s + 48, 2, h));
        let subject = get!(stream_entry(m.bytes, text, start as u32));
        let mut position = 0u16;
        let mut captured = (0u8, 0u16, 0u16);
        while (position as u32) < length as u32 * 2 {
            let mut index = 0u16;
            while index < sh(m.bytes, patterns) {
                let bytes = (length as u32 * 2) & 0xffff;
                let rc = if subject == 0 || bytes == 0 || bytes <= position as u32 {
                    0x81102007
                } else {
                    let (status, ctx) = crate::render_part_c::resolve(m, [h0, h1], k.environment);
                    if status >= 0x80000000 {
                        0x81102007
                    } else {
                        let p = m.word(patterns + 4) + index as u32 * 8;
                        let pattern = m.word(patterns + 8).wrapping_add(m.word(p));
                        let (result, offsets) = stage_pattern(
                            m.bytes,
                            owner,
                            key,
                            pattern,
                            subject,
                            bytes,
                            position as u32,
                            k,
                            h,
                        );
                        if result < 0 {
                            let status = crate::regex::error_status(result);
                            if status >= 0x80000000 && ((status >> 20) & !0x800) == 0xa5 {
                                let code = status & 0x1fff;
                                if code == 0xa {
                                    stage_error(m, ctx, k, 0x2afa, 0, &[], h);
                                } else if code != 0x14 {
                                    stage_error(
                                        m,
                                        ctx,
                                        k,
                                        0x2b13,
                                        k.formats[5],
                                        &[k.labels[0], status],
                                        h,
                                    );
                                }
                                status
                            } else {
                                0
                            }
                        } else {
                            if result > 0 {
                                captured = (
                                    m.byte(p + 4),
                                    offsets[0] as u16,
                                    offsets[1].wrapping_sub(offsets[0]) as u16,
                                );
                            }
                            0
                        }
                    }
                };
                if rc & 0x1fff == 0x14 {
                    index = index.wrapping_add(1);
                    continue;
                }
                if rc >= 0x80000000 {
                    return rc;
                }
                break;
            }
            if index >= sh(m.bytes, patterns) {
                position = position.wrapping_add(2);
            } else {
                m.put_word(s + 12, namebase.wrapping_add(position as u32));
                m.put_word(s + 16, captured.2 as u32);
                m.put_word(
                    s + 20,
                    origin
                        .wrapping_add(start as u32 * 2)
                        .wrapping_add(captured.1 as u32),
                );
                m.put_word(s + 24, captured.2 as u32);
                m.put_word(s + 28, captured.0 as u32);
                ok!(n::insert_ordered(m.bytes, events, s, h));
                ok!(n::fetch_entry(m.bytes, object, id, 1, Some(&mut table), h));
                position = captured.1.wrapping_add(captured.2);
            }
        }
        ok!(n::delete_range(
            m.bytes,
            text,
            start.wrapping_add(length),
            2
        ));
        let next = if current.is_some() {
            let p = get!(stream_entry(m.bytes, events, 0xffff));
            start
                .wrapping_add(length)
                .wrapping_add((m.word(p + 24) >> 1) as u16)
        } else {
            start.wrapping_add(length)
        };
        ok!(stream_index(m.bytes, text, next as u32));
        m.put_half(s + 44, 1);
        crate::render_part_a3::scan(m.bytes, events, 0x2b94ac, s + 44);
    }
    0
}
fn stage_exec_regex(
    m: &mut [u8],
    object: u32,
    key: u32,
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

    if crate::frontend::validate_handle(object != 0, key, 0xdd57, 24) >= 0x80000000 {
        return -16;
    }
    let entries = sw(m, object + 4);
    let count = sh(m, object + 8);
    let per = sh(m, object + 10);
    let size = sh(m, object + 12);
    let max = sh(m, object + 14);
    assert!(scratch.blocks.len() >= max as usize);
    assert!(
        scratch.frames.len() >= max as usize * per as usize
            && scratch.tags.len() >= max as usize * per as usize
    );
    for b in scratch.blocks.iter_mut() {
        *b = crate::regex::FrameBlockState::default();
    }
    for i in 0..count as usize {
        let p = sw(m, entries + i as u32 * 4);
        scratch.blocks[i] = crate::regex::FrameBlockState {
            present: p != 0,
            live: p != 0,
            used: if p != 0 { sw(m, p + 8) } else { 0 },
            count: if p != 0 { m[(p + 16) as usize] } else { 0 },
            code_id: if p != 0 { sw(m, p) } else { 0 },
            extra_id: if p != 0 { sw(m, p + 12) } else { 0 },
            entry_id: p,
        };
    }
    let plen = if pattern == 0 {
        0
    } else {
        sw(m, pattern + 4) as usize
    };
    assert!(plen <= scratch.program.len() && len as usize + 512 <= scratch.subject.len());
    scratch.program[..plen].copy_from_slice(&m[pattern as usize..pattern as usize + plen]);
    scratch.subject[..len as usize + 512].fill(0);
    scratch.subject[256..256 + len as usize]
        .copy_from_slice(&m[text as usize..text as usize + len as usize]);
    let code = pattern
        + if plen >= 40 {
            sh(m, pattern + 24) as u32 + sh(m, pattern + 26) as u32 * sh(m, pattern + 28) as u32
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
    let heap = sw(m, sw(m, object) + 4);
    let mut store = crate::regex::FrameStore {
        frames: scratch.frames,
        tags: scratch.tags,
        blocks: scratch.blocks,
        frames_per_block: per,
        frame_size: size,
        max_blocks: max,
        block_count: count,
        current_block: sh(m, object + 16),
        highest_block: sh(m, object + 18),
        max_frames: sh(m, object + 20),
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
        options,
        Some(offsets),
        30,
        tables,
    );
    sph(m, object + 8, store.block_count);
    sph(m, object + 16, store.current_block);
    sph(m, object + 18, store.highest_block);
    sph(m, object + 20, store.max_frames);
    for (i, b) in store.blocks.iter().enumerate() {
        if i < count as usize || b.entry_id != 0 {
            let old = sw(m, entries + i as u32 * 4);
            if i != 0 && i < count as usize && old != 0 {
                spw(m, old, 0);
                spw(m, old + 12, 0);
            }
            spw(m, entries + i as u32 * 4, b.entry_id);
        }
        if !b.present {
            continue;
        }
        spw(m, b.entry_id, b.code_id);
        spw(m, b.entry_id + 4, per as u32 * size as u32 + 1);
        spw(m, b.entry_id + 8, b.used);
        spw(m, b.entry_id + 12, b.extra_id);
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

pub fn fn_179f30(
    m: &mut Memory<'_>, broker: u32, name: u32, out: u32,
    globals: &crate::boot_part_b::DataOpenGlobals,
    h: &mut impl crate::boot_part_b::DataHost,
) -> u32 {
    crate::boot_part_b::fn_1795f0(m, broker, name, 0, out, globals, h)
}
struct MapCalls<'a, H>(&'a mut H);
impl<H: crate::boot_part_b::Host> crate::boot_part_b::Host for MapCalls<'_, H> {
    fn real_log_args(&mut self)->bool {self.0.real_log_args()}
    fn format_address_m(&mut self,m:&Memory<'_>,format:&[u8])->Option<u32> {self.0.format_address_m(m,format)}
    fn log_variadic(&mut self,m:&mut Memory<'_>,entry:u32,a:[u32;4],format:&[u8],varargs:&[u32])->bool {self.0.log_variadic(m,entry,a,format,varargs)}
    fn thread_id(&mut self) -> u32 { self.0.thread_id() }
    fn call(&mut self, m: &mut Memory<'_>, f: u32, a: &[u32]) -> u32 {
        match f {
            0x1731c4 => crate::boot_part_d::fn_1731c4(m,a[0],a[1],a[2],a[3],self),
            0x17c9d8 => crate::boot_part_e::fn_17c9d8(m,a[0],a[1],a[2],a[3],a[4],self),
            _ => self.0.call(m,f,a),
        }
    }
}
impl<H: crate::boot_part_b::Host> crate::boot_part_e::Host for MapCalls<'_, H> {
    fn call(&mut self, m: &mut Memory<'_>, f: u32, a: &[u32]) -> u32 {
        self.0.call(m,f,a)
    }
}
impl<H: crate::boot_part_b::Host> crate::boot_part_d::Host for MapCalls<'_, H> {
    fn call_m(&mut self, f:u32, a:&[u32], m:&mut Memory<'_>) -> u32 {
        self.0.call(m,f,a)
    }
    fn params_call_m(&mut self,_:u32,_:&mut [u32;4],_:u32,_:&mut u32,_:u32,_:&mut Memory<'_>)->u32 {
        panic!("map does not read archive parameters")
    }
}

pub fn fn_17a1a4(
    m: &mut Memory<'_>, broker:u32, handle:u32, offset:u32, requested:u32, out:u32,
    scratch:u32, unmap:&crate::boot_part_e::UnmapEnvironment,
    h:&mut impl crate::boot_part_b::Host,
) -> u32 {
    let saved=m.word(scratch);
    m.put_word(scratch,requested);
    let rc=crate::boot_part_b::fn_179374(m,broker,handle,offset,scratch,out,&mut MapCalls(h));
    let count=m.word(scratch);
    m.put_word(scratch,saved);
    if rc<0x80000000 && count<requested {
        crate::boot_part_e::fn_179208(m,broker,handle,m.word(out),unmap,&mut MapCalls(h));
        return 0x81506004;
    }
    rc
}

pub fn fn_2b94d8(m:&mut Memory<'_>,p:u32,value:u32,out:u32)->u32 {
    if p==0 || out==0{return 0x81102007;}
    m.put_word(out,value);
    let base=match stream_entry(m.bytes,p,0){Ok(b)=>b,Err(r)=>return if r&0x1fff==15 {0}else{r}};
    let count=match stream_count(m.bytes,p){Ok(n)=>n,Err(r)=>return r};
    let mut best=0;
    for i in 0..count as u32 {
        let e=base+i*40;
        if value<m.word(e+20){break;}
        if m.byte(e)==0 {
            let hi=m.word(e+20).wrapping_add(m.word(e+24));
            let sum=m.word(e+12).wrapping_add(m.word(e+16));
            if hi<=value && (best==0 || (m.word(best+20).wrapping_add(m.word(best+24))<=hi && m.word(best+12).wrapping_add(m.word(best+16))<=sum)){best=e;}
        }
    }
    if best!=0 {m.put_word(out,m.word(best+12).wrapping_add(m.word(best+16)).wrapping_sub(m.word(best+20).wrapping_add(m.word(best+24))).wrapping_add(m.word(out)));}
    0
}
fn kernel_span(m:&mut Memory<'_>,event:u32,source:u32,source_len:u32,offset:u32,length:u32,kind:u32) {
    for (off,v) in [(12,source),(16,source_len),(20,offset),(24,length),(28,kind)] {m.put_word(event+off,v);}
}
fn kernel_copy_span(m:&mut Memory<'_>,dst:u32,src:u32,kind:u32) {
    let a=[m.word(src+12),m.word(src+16),m.word(src+20),m.word(src+24)];kernel_span(m,dst,a[0],a[1],a[2],a[3],kind);
}

pub fn fn_253870(m:&mut Memory<'_>,a:[u32;7],constants:u32,scratch:u32,h:&mut impl crate::render_part_a2::RenderHost)->u32 {
    assert_eq!(m.base,0);
    let mut saved=[0u8;256];saved.copy_from_slice(&m.bytes[scratch as usize..scratch as usize+256]);
    let rc=kernel_replace(m,a,constants,scratch,h);
    m.bytes[scratch as usize..scratch as usize+256].copy_from_slice(&saved);rc
}
fn kernel_replace(m:&mut Memory<'_>,a:[u32;7],constants:u32,s:u32,h:&mut impl crate::render_part_a2::RenderHost)->u32 {
    use crate::normalization as n;
    let [text,markers,pos,unit,modify,resource,reference]=a;
    macro_rules! get {($v:expr)=>{match $v {Ok(v)=>v,Err(r)=>return r}}}
    macro_rules! ok {($v:expr)=>{{let r=$v;if r>=0x80000000{return r;}}}}
    let (start,finish,mark)=(s,s+40,s+80);
    for (dst,off) in [(start,0xa4),(finish,0xcc),(mark,0xf4)] {crate::fe_text::memcpy_195ae8(m.bytes,dst,constants+off,40);}
    if text==0||markers==0||unit==0{return 0x81102007;}
    let base=get!(stream_entry(m.bytes,markers,0));
    let origin=m.word(base+20);
    let mut current=get!(stream_entry(m.bytes,markers,0xffff));
    if modify==1 && sh(m.bytes,current+10)!=7{return 0x81102007;}
    let mut names=0;
    ok!(n::fetch_entry(m.bytes,resource,0x3f4,1,Some(&mut names),h));
    let original_source=m.word(current+12);let original_source_len=m.word(current+16);
    let original_offset=m.word(current+20);let original_length=m.word(current+24);
    let relative=original_offset.wrapping_sub(m.word(base+20));
    let within=pos.wrapping_mul(2).wrapping_sub(relative);
    let used=sh(m.bytes,unit+4) as u32*2;
    let mut exhausted_at=None;
    let input=sh(m.bytes,unit+4);ok!(n::delete_range(m.bytes,text,pos as u16,input));
    if sh(m.bytes,unit+12)!=0 {
        ok!(stream_index(m.bytes,text,pos));
        let ptr=m.word(unit+8);let len=sh(m.bytes,unit+12) as u32;ok!(n::insert_elements(m.bytes,text,ptr,len,h));
        if m.word(current+24).wrapping_add(relative)<=(sh(m.bytes,unit+4) as u32).wrapping_add(pos).wrapping_mul(2) {exhausted_at=Some(m.word(current+20));}
    }
    let delta=(sh(m.bytes,unit+12) as u32*2).wrapping_sub(sh(m.bytes,unit+4) as u32*2);
    let changed=if modify==1 {current}else{base};
    m.put_word(changed+24,m.word(changed+24).wrapping_add(delta));
    let mut index=0;
    ok!(n::current_index(Some(&m.bytes[m.word(markers+12) as usize..]),sh(m.bytes,markers+16),Some(&mut index)));
    if reference!=0 {
        crate::render_part_b::fn_176354(m,s+120,reference,3);m.put_half(s+126,0);m.put_half(mark+10,0x24);
        ok!(crate::render_part_a2::intern_string(m.bytes,names,s+120,mark+32,resource,h));
        if m.word(current+20).wrapping_sub(origin)<pos.wrapping_mul(2) {
            let kind=m.word(mark+28);kernel_span(m,mark,original_source.wrapping_add(within),0,origin.wrapping_add(pos*2),0,kind);
            ok!(stream_index(m.bytes,markers,1));ok!(n::insert_ordered(m.bytes,markers,mark,h));ok!(stream_index(m.bytes,markers,index as u32));
        }else{
            let source=m.word(current+12);let offset=m.word(current+20);let kind=m.word(mark+28);kernel_span(m,mark,source,0,offset,0,kind);
            ok!(stream_index(m.bytes,markers,1));ok!(n::insert_ordered(m.bytes,markers,mark,h));ok!(stream_index(m.bytes,markers,index as u32+1));current+=40;
        }
    }
    let count=get!(stream_count(m.bytes,markers));
    current+=40;index=index.wrapping_add(1);
    while index<count {
        let at=m.word(current+20);let begin=origin.wrapping_add(pos*2);
        if at>=begin {
            if at<origin.wrapping_add((sh(m.bytes,unit+4) as u32).wrapping_add(pos).wrapping_mul(2)){m.put_word(current+20,begin);}
            else{m.put_word(current+20,at.wrapping_add((sh(m.bytes,unit+12) as u32*2).wrapping_sub(sh(m.bytes,unit+4) as u32*2)));}
        }
        current+=40;index=index.wrapping_add(1);
    }
    if modify==0{return 0;}
    current=get!(stream_entry(m.bytes,markers,0xffff));
    let kind=m.byte(unit+6) as u32;
    if sh(m.bytes,unit+12)==0 || m.word(current+28)==kind {
        kernel_copy_span(m,start,current,kind);ok!(n::insert_ordered(m.bytes,markers,start,h));return 0;
    }
    let oldkind=m.word(current+28);
    if m.word(current+20).wrapping_sub(origin)<pos.wrapping_mul(2) {
        let at=origin.wrapping_add(pos*2);
        m.put_word(current+24,at.wrapping_sub(m.word(current+20)));
        let at=m.word(current+20).wrapping_add(m.word(current+24));ok!(fn_2b94d8(m,markers,at,current+16));
        m.put_word(current+16,m.word(current+16).wrapping_sub(m.word(current+12)));
        let length=sh(m.bytes,unit+12) as u32*2;kernel_span(m,finish,original_source.wrapping_add(within),if kind==16{0}else{used},at,length,kind);
        ok!(n::insert_ordered(m.bytes,markers,finish,h));
        if kind==16 {let length=sh(m.bytes,unit+12) as u32*2;kernel_span(m,start,original_source.wrapping_add(within),used,at,length,kind);ok!(n::insert_ordered(m.bytes,markers,start,h));}
    }else{
        m.put_word(current+16,sh(m.bytes,unit+4) as u32*2);m.put_word(current+24,sh(m.bytes,unit+12) as u32*2);m.put_word(current+28,kind);
        if kind==16 {kernel_copy_span(m,start,current,kind);ok!(n::insert_ordered(m.bytes,markers,start,h));}
    }
    if original_length.wrapping_add(relative)>(sh(m.bytes,unit+4) as u32).wrapping_add(pos).wrapping_mul(2) {
        let offset=origin.wrapping_add((sh(m.bytes,unit+12) as u32).wrapping_add(pos).wrapping_mul(2));let length=original_length.wrapping_add(relative).wrapping_sub((sh(m.bytes,unit+4) as u32).wrapping_add(pos).wrapping_mul(2));kernel_span(m,finish,original_source.wrapping_add(within).wrapping_add(used),original_source_len.wrapping_sub(within.wrapping_add(used)),offset,length,oldkind);
        ok!(n::insert_ordered(m.bytes,markers,finish,h));
    }
    let at=exhausted_at.unwrap_or_else(||origin.wrapping_add((sh(m.bytes,unit+12) as u32).wrapping_add(pos).wrapping_mul(2)));
    current=get!(stream_entry(m.bytes,markers,0));let count=get!(stream_count(m.bytes,markers));
    for i in 0..count as u32 {
        let e=current+i*40;if sh(m.bytes,e+10)==7 && m.word(e+28)==oldkind && m.word(e+20)==at {ok!(stream_index(m.bytes,markers,i));return 0;}
    }
    0
}

#[derive(Clone,Copy)]
pub struct KernelData<'a> {
    pub constants:u32,
    pub category:u32,
    pub language:u32,
    pub formats:[u32;6],
    pub scratch:u32,
    pub fold:&'a [u16;256],
}
fn kernel_pair(m:&Memory<'_>,events:u32,count:u16,lo:u32,hi:u32,first:u32)->Option<(u32,u32,u32,u32)> {
    for i in 0..count as u32 {
        let a=events+i*40;let offset=m.word(a+20);
        if sh(m.bytes,a+10)!=32 || m.word(a+28)!=1 || offset<lo || offset>hi {continue;}
        for j in i+1..count as u32 {
            let b=events+j*40;let end=m.word(b+20);
            if sh(m.bytes,b+10)==32 && m.word(b+28)==0 && end>=offset && end<=hi {
                return Some((((offset.wrapping_sub(m.word(first+20)))&65535)>>1,((end.wrapping_sub(m.word(first+20)))&65535)>>1,offset,end));
            }
        }
    }
    None
}
fn kernel_debug(m:&mut Memory<'_>,object:u32,k:&KernelData<'_>,event:usize,args:&[u32],h:&mut (impl crate::render_part_a2::RenderHost+crate::log_live::LogMethodsM)) {
    if m.byte(object+32)!=1 {return;}
    let logger=m.word(m.word(object+8)+16);
    if logger==0{return;}
    assert_ne!(k.category,0,"active kernel logger needs regional category");
    assert_ne!(k.formats[event],0,"active kernel logger needs regional formats");
    for (i,v) in args.iter().enumerate(){m.put_word(k.scratch+32+i as u32*4,*v);}
    crate::log_live::fn_1738e8(m,logger,k.category,5,0,k.formats[event],k.scratch+32,h);
}

pub fn fn_253fd0(
    m:&mut Memory<'_>,a:[u32;15],k:&KernelData<'_>,
    h:&mut (impl crate::render_part_a2::RenderHost+crate::log_live::LogMethodsM+crate::boot_part_b::FormatHost),
)->u32 {
    assert_eq!(m.base,0);
    let off=k.scratch as usize;let mut saved=[0u8;512];saved.copy_from_slice(&m.bytes[off..off+512]);
    let rc=kernel_normalize(m,a,k,h);
    m.bytes[off..off+512].copy_from_slice(&saved);rc
}
fn kernel_normalize(
    m:&mut Memory<'_>,a:[u32;15],k:&KernelData<'_>,
    h:&mut (impl crate::render_part_a2::RenderHost+crate::log_live::LogMethodsM+crate::boot_part_b::FormatHost),
)->u32 {
    use crate::normalization as n;
    let [object,text,markers,data,first,mut events,count,mut base,mode,current,mut pos,mut end,delta,inserted,max_length]=a;
    let mut count=count as u16;
    if object==0 || text==0 || markers==0 || events==0{return 0x81102007;}
    m.put_byte(inserted,0);m.put_half(delta,0);m.put_half(max_length,0);
    let unit=k.scratch;let parsed=k.scratch+20;let original_pos=pos;
    m.put_word(unit,0);m.put_word(unit+8,0);m.put_word(parsed,0);
    let mut paired=if mode==3 {kernel_pair(m,events,count,m.word(current+20),m.word(current+20).wrapping_add(m.word(current+24)),first)}else{None};
    if mode==3 {
        let (event,args)=match paired {Some((_,_,a,b))=>(0,[a,b]),None=>(1,[m.word(current+20),m.word(current+20).wrapping_add(m.word(current+24))])};
        kernel_debug(m,object,k,event,&args,h);
    }
    let mut debug=0;let mut debug_required=false;
    let heap=m.word(m.word(object+8)+4);let n=sh(m.bytes,object+62)as u32+1;let firstbuf=h.calloc(m.bytes,heap,n,2).unwrap_or(0);
    m.put_word(unit,firstbuf);
    let mut second=0;
    if firstbuf!=0 {
        let size=(sh(m.bytes,object+60) as u32+1).wrapping_mul(sh(m.bytes,object+64) as u32).wrapping_mul(2).wrapping_add(2);
        let heap=m.word(m.word(object+8)+4);second=h.calloc(m.bytes,heap,1,size).unwrap_or(0);m.put_word(unit+8,second);
        if second!=0 {debug_required=m.byte(object+32)==1;}
        if debug_required {let heap=m.word(m.word(object+8)+4);let n=sh(m.bytes,object+62)as u32+1;debug=h.calloc(m.bytes,heap,n,1).unwrap_or(0);}
    }
    let allocated=firstbuf!=0 && second!=0 && (!debug_required || debug!=0);
    if allocated {
        while pos<end {
            let mut stop=end;
            if mode==3 {
                while pos<end && crate::fe_text::strchr16_176514(m.bytes,k.constants+0x94,sh(m.bytes,base+pos*2) as u32)!=0 {pos=(pos+1)as u16 as u32;}
                if pos==original_pos || (pos>original_pos && pos<end && crate::fe_text::strchr16_176514(m.bytes,k.constants+0x94,sh(m.bytes,base+pos*2-2)as u32)!=0) {
                    let mut j=pos;
                    loop {j=(j+1)as u16 as u32;if j>=end || crate::fe_text::strchr16_176514(m.bytes,k.constants+0x94,sh(m.bytes,base+j*2)as u32)!=0 {break;}}
                    if j!=0 && j<end && crate::fe_text::strchr16_176514(m.bytes,k.constants+0x94,sh(m.bytes,base+j*2)as u32)!=0 {stop=j;}
                }
            }
            let mut use_pair=false;
            if let Some((begin,pair_end,_,_))=paired {
                if pos>=begin || begin<stop {
                    if pos<begin && begin<stop {stop=begin;}
                    else if pos>=begin {pos=begin;stop=pair_end;use_pair=true;}
                }
            }
            let length=stop.wrapping_sub(pos);
            if length as i32<=0 {break;}
            if length as i32>sh(m.bytes,max_length) as i32 {m.put_half(max_length,length as u16);}
            let input=base.wrapping_add(pos.wrapping_mul(2));
            if input==0 || length as u16==0 {break;}
            let amount=(length as u16).min(sh(m.bytes,object+62));
            m.put_half(unit+4,amount);
            crate::fe_text::memmove_195b38(m.bytes,firstbuf,input,amount as u32*2);
            if m.byte(object+32)==1 {n::u16_to_u8_ncopy(m.bytes,debug,firstbuf,amount as u32);m.put_byte(debug+amount as u32,0);kernel_debug(m,object,k,2,&[debug],h);}
            let mut rc=kernel_parse(m,object,unit,(mode==3)as u32,parsed,k,h);
            if rc<0x80000000 && mode!=3 {
                while sh(m.bytes,unit+4)>1 && sh(m.bytes,unit+4)<=amount && m.byte(unit+6)==0 {
                    m.put_half(unit+4,sh(m.bytes,unit+4)-1);
                    rc=kernel_parse(m,object,unit,0,parsed,k,h);
                    if rc>=0x80000000 {break;}
                }
            }
            if rc>=0x80000000 {break;}
            let kind=m.byte(unit+6);
            let convert=if kind==2 {
                if sh(m.bytes,second)<127 {m.put_byte(unit+6,3);}true
            }else if kind==43 || kind==3 {true}
            else if kind==16 {f::compare_folded(k.fold,&m.bytes[m.word(object+12)as usize..],&m.bytes[k.language as usize..])!=0}
            else{false};
            if convert {
                m.put_half(max_length,0);m.put_byte(inserted,1);
                if m.byte(object+32)==1 {let n=sh(m.bytes,unit+12)as u32;n::u16_to_u8_ncopy(m.bytes,debug,second,n);m.put_byte(debug+n,0);kernel_debug(m,object,k,3,&[debug],h);}
                let args=[text,markers,pos,unit,1,data,m.word(parsed)];
                if fn_253870(m,args,k.constants,k.scratch+128,h)>=0x80000000 {break;}
                base=match stream_entry(m.bytes,text,0){Ok(p)=>p,Err(_)=>break};
                let added=sh(m.bytes,unit+12);let removed=sh(m.bytes,unit+4);
                m.put_half(delta,sh(m.bytes,delta).wrapping_add(added).wrapping_sub(removed));
                pos=(pos.wrapping_add(added as u32))as u16 as u32;
                end=(end.wrapping_add(added as u32).wrapping_sub(removed as u32))as u16 as u32;
                events=match stream_entry(m.bytes,markers,0){Ok(p)=>p,Err(_)=>break};
                count=match stream_count(m.bytes,markers){Ok(n)=>n,Err(_)=>break};
            }else{
                if mode==3 {kernel_debug(m,object,k,4,&[],h);pos=stop;}
                pos=(pos+1)as u16 as u32;
            }
            if use_pair {
                let lo=m.word(first+20).wrapping_add(pos*2);let hi=m.word(current+20).wrapping_add(end*2);
                paired=kernel_pair(m,events,count,lo,hi,first);
                let (event,args)=match paired {Some((_,_,a,b))=>(0,[a,b]),None=>(5,[lo,hi])};
                kernel_debug(m,object,k,event,&args,h);
            }
        }
    }else{
        let logger=m.word(m.word(object+8)+16);
        if logger!=0 {assert_ne!(k.category,0,"active kernel logger needs regional category");}
        crate::render_part_a::error_log(m,h,logger,k.category,0x8110200a,0,k.scratch+32,k.scratch+64);
    }
    for ptr in [m.word(unit),m.word(unit+8),debug] {if ptr!=0 {let heap=m.word(m.word(object+8)+4);h.free(m.bytes,heap,ptr);}}
    0
}

fn kernel_parse(m:&mut Memory<'_>,object:u32,unit:u32,flag:u32,parsed:u32,k:&KernelData<'_>,h:&mut impl crate::render_part_a2::RenderHost)->u32 {
 let a=[m.word(object+36),m.word(object+40),m.word(object+44),sh(m.bytes,object+62)as u32,sh(m.bytes,object+60)as u32];
 crate::render_part_a2::parse_first_unit(m.bytes,a[0],a[1],a[2],unit,flag,a[3],a[4],parsed,k.constants+24,h)
}

pub struct KernelRegions<'a,'b,H> {
    pub host:&'a mut H,
    pub data:&'a KernelData<'b>,
    pub outer:u32,
}
impl<H:crate::render_part_a3::RegionHost> f::HeapVtable for KernelRegions<'_,'_,H>{
 fn alloc_at(&mut self,o:u32,n:u32)->u32{self.host.alloc_at(o,n)}
 fn realloc_at(&mut self,o:u32,p:u32,n:u32)->u32{self.host.realloc_at(o,p,n)}
 fn release_at(&mut self,o:u32,p:u32){self.host.release_at(o,p)}
}
impl<H:crate::render_part_a3::RegionHost> f::MethodTable for KernelRegions<'_,'_,H>{
 fn call_at(&mut self,o:u32,n:u32,s:u32)->u32{self.host.call_at(o,n,s)}
}
impl<H:crate::render_part_a3::RegionHost> crate::normalization::NormalizationHost for KernelRegions<'_,'_,H>{
 fn calloc(&mut self,m:&mut[u8],h:u32,n:u32,s:u32)->Option<u32>{self.host.calloc(m,h,n,s)}
 fn alloc(&mut self,m:&mut[u8],h:u32,n:u32)->Option<u32>{self.host.alloc(m,h,n)}
 fn realloc(&mut self,m:&mut[u8],h:u32,p:u32,n:u32)->Option<u32>{self.host.realloc(m,h,p,n)}
 fn free(&mut self,m:&mut[u8],h:u32,p:u32){self.host.free(m,h,p)}
 fn resolve(&mut self,m:&mut[u8],a:u32,b:u32)->(u32,u32){self.host.resolve(m,a,b)}
}
impl<H:crate::render_part_a3::RegionHost> crate::selection::arena::Host for KernelRegions<'_,'_,H>{
 fn call(&mut self,m:&mut crate::selection::arena::Arena<'_>,f:u32,a:&[u32])->(u32,u32){crate::selection::arena::Host::call(self.host,m,f,a)}
}
impl<H:crate::render_part_a3::RegionHost> crate::render_part_a2::RenderHost for KernelRegions<'_,'_,H>{
 fn process(&mut self,m:&mut[u8],v:u32,c:&crate::render_part_a2::ProcessCall)->(u32,u32){self.host.process(m,v,c)}
}
impl<H:crate::render_part_a3::RegionHost> crate::log_live::LogMethodsM for KernelRegions<'_,'_,H>{
 fn log_m(&mut self,f:u32,o:u32,c:u32,l:u32,a:u32,t:u32,v:u32,m:&mut Memory<'_>)->u32{self.host.log_m(f,o,c,l,a,t,v,m)}
}
impl<H:crate::render_part_a3::RegionHost> crate::boot_part_b::Host for KernelRegions<'_,'_,H>{
 fn thread_id(&mut self)->u32{self.host.thread_id()}
 fn call(&mut self,m:&mut Memory<'_>,f:u32,a:&[u32])->u32{crate::boot_part_b::Host::call(self.host,m,f,a)}
 fn release_dictionary(&mut self,m:&mut Memory<'_>,p:u32)->Option<u32>{self.host.release_dictionary(m,p)}
}
impl<H:crate::render_part_a3::RegionHost> crate::boot_part_b::FormatHost for KernelRegions<'_,'_,H>{}
impl<H:crate::render_part_a3::RegionHost> f::StateLock for KernelRegions<'_,'_,H>{
 fn acquire(&mut self)->u32{self.host.acquire()}
 fn release(&mut self)->u32{self.host.release()}
}
impl<H:crate::render_part_a3::RegionHost> crate::render_part_a3::Host for KernelRegions<'_,'_,H>{
 fn select_lock(&mut self,p:u32){self.host.select_lock(p)}
 fn level(&mut self,p:u32)->u32{self.host.level(p)}
 fn method(&mut self,m:&mut[u8],f:u32,a:&[u32])->u32{crate::render_part_a3::Host::method(self.host,m,f,a)}
}
impl<H:crate::render_part_a3::RegionHost> crate::render_part_c::Host for KernelRegions<'_,'_,H>{
 fn kernel_literals(&self)->[u32;7]{self.host.kernel_literals()}
 fn call(&mut self,m:&mut Memory<'_>,f:u32,a:&[u32])->u32{
  if f==0x253870 {fn_253870(m,a.try_into().unwrap(),self.data.constants,self.data.scratch+128,self)}
  else{crate::render_part_c::Host::call(self.host,m,f,a)}
 }
 fn output(&mut self,m:&mut Memory<'_>,f:u32,a:&[u32])->(u32,u32){crate::render_part_c::Host::output(self.host,m,f,a)}
}
impl<H:crate::render_part_a3::RegionHost> crate::render_part_a2::NativeNormalizeHost for KernelRegions<'_,'_,H>{
 fn normalize(&mut self,m:&mut[u8],f:u32,c:&crate::render_part_a2::NormalizeCall)->u32{self.host.normalize(m,f,c)}
}
impl<H:crate::render_part_a3::RegionHost> crate::render_part_a3::RegionHost for KernelRegions<'_,'_,H>{
 fn normalize_kernel(&mut self,m:&mut Memory<'_>,a:[u32;15])->u32 {let k=*self.data;fn_253fd0(m,a,&k,self)}
 fn unported_normalizer(&mut self,m:&mut[u8],f:u32,a:&[u32])->u32{
  match f {
   0x253fd0=>{let k=*self.data;fn_253fd0(&mut Memory {base:0,bytes:m},a.try_into().unwrap(),&k,self)},
   0x254aac=>fn_254aac(m,a.try_into().unwrap(),self.data.constants,self.outer,self),
   _=>self.host.unported_normalizer(m,f,a),
  }
 }
}
