

use crate::boot::{Calls, Memory};
use crate::boot_live;

fn rd(m: &Memory<'_>, addr: u32) -> u32 {
    let o = addr.wrapping_sub(m.base) as usize;
    u32::from_le_bytes(m.bytes[o..o + 4].try_into().unwrap())
}

fn wr(m: &mut Memory<'_>, addr: u32, v: u32) {
    let o = addr.wrapping_sub(m.base) as usize;
    m.bytes[o..o + 4].copy_from_slice(&v.to_le_bytes());
}

fn rb(m: &Memory<'_>, addr: u32) -> u8 {
    m.bytes[addr.wrapping_sub(m.base) as usize]
}

fn wb(m: &mut Memory<'_>, addr: u32, v: u8) {
    m.bytes[addr.wrapping_sub(m.base) as usize] = v;
}

fn wh(m: &mut Memory<'_>, addr: u32, v: u16) {
    let o = addr.wrapping_sub(m.base) as usize;
    m.bytes[o..o + 2].copy_from_slice(&v.to_le_bytes());
}

fn rh(m: &Memory<'_>, addr: u32) -> u16 {
    let o = addr.wrapping_sub(m.base) as usize;
    u16::from_le_bytes(m.bytes[o..o + 2].try_into().unwrap())
}

pub const MANAGER_GLOBAL: u32 = 0xACB550;

pub const ALLOC_TABLE: u32 = 0xB6F4F8;

pub const PATH_TABLE: u32 = 0xA7B72C;

pub const HEAP_TABLE_ROOT: u32 = 0xB6F4EC;

pub const FILL_FLAG: u32 = 0xA7B8E8;

pub const LOADER_VTABLE: u32 = 0xA35F9C;

pub const VOICE_DATA: u32 = 0xA7B67C;

pub const POOL_FLAG: u32 = 0xA89938;

pub const POOL_HEAD: u32 = 0xBFC1B0;

pub const HEAP_VTABLE: u32 = 0xA478AC;

pub const TPRIO_A: u32 = 0x1221CC;
pub const TPRIO_B: u32 = 0x1221D0;
pub const TSVC_CTX: u32 = 0x1221D4;

pub const ARBITER_ADDRESS: u32 = 0xA7BB7C;

pub const CB_A: u32 = 0x8C0314;
pub const CB_B: u32 = 0x8C0324;
pub const CB_C: u32 = 0x8C0340;

pub const GAME_ALLOC_TABLE: u32 = 0xA35F9C;

pub const THREAD_ENTRY: u32 = 0x3596B4;
pub const THREAD_STACK: u32 = 0xB6B300;
pub const THREAD_D0: u32 = 0x845B40;
pub const THREAD_D1: u32 = 0x845C68;
pub const THREAD_D2: u32 = 0x845BEC;

pub const LOADER_SIZE: u32 = 0x354;
pub const LOADER_QUEUE: u32 = 0x8;
pub const LOADER_BUF_STRIDE: u32 = 0xA4;
pub const LOADER_HEADER_REQ: u32 = 0x340;
pub const LOADER_COMPONENTS_REQ: u32 = 0x344;
pub const LOADER_STATE: u32 = 0x348;

pub const QUEUE_TAG: u32 = 0xA4865C;

pub const QUEUE_VTABLE: u32 = 0xA486CC;

pub fn fn_13d4fc(m: &mut Memory<'_>, p: u32) {
    wr(m, p, QUEUE_TAG);
}

pub fn fn_708534(m: &mut Memory<'_>, p: u32) -> u32 {
    fn_13d4fc(m, p);
    wr(m, p, QUEUE_VTABLE);
    for i in 0..3u32 {
        let head = p + 8 + i * 12;
        wr(m, head - 4, 0);
        wr(m, head, head);
        wr(m, head + 4, head);
    }
    p
}

pub fn fn_4e0060(m: &mut Memory<'_>, h: &mut impl Calls, loader: u32) -> u32 {
    wr(m, loader, LOADER_VTABLE);
    wr(m, loader + 4, 0);
    let q = fn_708534(m, loader + LOADER_QUEUE);

    let base = q.wrapping_sub(8);
    for i in 0..4 {
        let b = base + 0xB0 + i * LOADER_BUF_STRIDE;
        wr(m, b, 0);
        wr(m, b + 4, 0);
    }
    wr(m, base + LOADER_HEADER_REQ, 0);
    wr(m, base + LOADER_COMPONENTS_REQ, 0);
    wr(m, base + LOADER_STATE, 0);
    let heap = fn_10edc0(m, h, 1, 0x400000);
    wr(m, base + 4, heap);
    base
}

pub fn request_boot_archive(
    m: &mut Memory<'_>,
    h: &mut impl Calls,
    loader: u32,
    path_slot: u32,
    req_offset: u32,
) -> u32 {
    let allocator = rd(m, ALLOC_TABLE + 4);
    let manager = rd(m, MANAGER_GLOBAL);
    let path = rd(m, PATH_TABLE + path_slot * 4);
    let req = h.call(m, 0x163d34, &[manager, path, loader, allocator, 4]);
    wr(m, loader + req_offset, req);
    req
}

pub fn fn_4dff90(m: &mut Memory<'_>, h: &mut impl Calls, loader: u32) -> u32 {
    request_boot_archive(m, h, loader, 1, LOADER_COMPONENTS_REQ)
}

pub fn fn_4dffd4(m: &mut Memory<'_>, h: &mut impl Calls, loader: u32) -> u32 {
    request_boot_archive(m, h, loader, 0, LOADER_HEADER_REQ)
}

pub fn fn_4dfef8(m: &mut Memory<'_>, loader: u32) {
    let manager = rd(m, MANAGER_GLOBAL);
    let table = rd(m, manager + 0x14);
    let idx = rd(m, loader + 0x340);
    let entry = table + idx.wrapping_mul(0xE8);
    let str_ptr = rd(m, entry + 0xC0);
    let len = rd(m, entry + 0xCC);
    if str_ptr != 0 && len != 0 {
        wb(m, str_ptr + len - 1, 0);
    }
}

pub fn fn_4dfea8(m: &mut Memory<'_>, loader: u32) {
    let manager = rd(m, MANAGER_GLOBAL);
    let slots = rd(m, manager + 0x14);
    let idx = rd(m, loader + 0x344);
    let blob = rd(m, slots + idx.wrapping_mul(0xE8) + 0xC0);
    m.open_archive(blob, loader + 0xB0 + 8);
    wh(m, loader + 0xB0 + 36, 0);
    m.append_archive(loader + 8, loader + 0xB0);
}

pub struct HeapCalls<'a, H: Calls> {
    pub inner: &'a mut H,
}

impl<H: Calls> crate::boot_part_b::Host for HeapCalls<'_, H> {
    fn call(&mut self, m: &mut Memory<'_>, f: u32, a: &[u32]) -> u32 {
        if let Some(r) = route_call(m, &mut *self.inner, f, a) {
            return r;
        }
        self.inner.call(m, f, a)
    }
    fn thread_id(&mut self) -> u32 {
        0x1FF82000
    }
}

impl<H: Calls> crate::speech_part_1::Host for HeapCalls<'_, H> {
    fn call(&mut self, m: &mut Memory<'_>, f: u32, a: &[u32]) -> u32 {
        if let Some(r) = route_call(m, &mut *self.inner, f, a) {
            return r;
        }
        self.inner.call(m, f, a)
    }
    fn get(
        &mut self,
        _: &mut Memory<'_>,
        _: u32,
        _: u32,
        _: u32,
        _: u32,
    ) -> (u32, u32, u32) {
        panic!("stream call on the cold-boot heap path")
    }
}

impl<H: Calls> crate::speech_part_1::HeapHost for HeapCalls<'_, H> {
    fn thread_id(&mut self) -> u32 {
        0x1FF82000
    }
    fn arbiter_address(&self) -> u32 {
        ARBITER_ADDRESS
    }
}

pub fn fn_12973c(m: &mut Memory<'_>, h: &mut impl Calls, size: u32, idx: u32) -> u32 {
    let table = HEAP_TABLE_ROOT;
    let fill = rb(m, FILL_FLAG) as u32;
    crate::speech_part_1::fn_11db30(m, size, idx, 32, fill, table, &mut HeapCalls { inner: h })
}

pub fn fn_39aa64(m: &mut Memory<'_>, h: &mut impl Calls, mgr: u32, idx: u32) -> u32 {
    let table = rd(m, mgr + 20);
    let entry = table + idx.wrapping_mul(0xE8);
    if rb(m, entry + 0xE4) != 4 {
        return 0;
    }
    let ptr = rd(m, entry + 0xC0);
    if ptr != 0 && rd(m, entry + 0xC8) == 0 {
        m.fn_132c90(h, rd(m, entry + 0xD8), ptr);
    }
    wh(m, entry, 0);
    for (off, size) in [
        (0xC0u32, 4u32),
        (0xC4, 4),
        (0xC8, 4),
        (0xCC, 4),
        (0xD0, 4),
        (0xD4, 4),
        (0xD8, 4),
        (0xDC, 4),
        (0xE0, 4),
        (0xE4, 1),
        (0xE5, 1),
        (0xE6, 1),
    ] {
        for i in 0..size {
            wb(m, entry + off + i, 0);
        }
    }
    1
}

pub fn fn_122058(m: &mut Memory<'_>, p: u32) {
    m.fn_165c84(p);
    wr(m, p + 4, 0);
    wr(m, p + 8, 0);
}

pub fn fn_129778(m: &mut Memory<'_>, h: &mut impl Calls, size: u32) -> u32 {
    let table = HEAP_TABLE_ROOT;
    let fill = rb(m, FILL_FLAG) as u32;
    crate::speech_part_1::fn_11db30(m, size, 0, 32, fill, table, &mut HeapCalls { inner: h })
}

pub fn fn_19b9f8(m: &mut Memory<'_>, p: u32, a: u32) {
    wh(m, p + 10, a as u16);
    wr(m, p, 0);
    wr(m, p + 4, 0);
    wh(m, p + 8, 0);
}

pub fn fn_1852c0(m: &mut Memory<'_>, c: u32, i: u32) -> u32 {
    if i == 0 {
        rd(m, c)
    } else {
        let t = rh(m, c + 10) as u32;
        rd(m, t.wrapping_add(i).wrapping_add(4))
    }
}

pub fn fn_185208(m: &mut Memory<'_>, e: u32, o: u32) -> u32 {
    let mut i = 0u32;
    loop {
        let cur = fn_1852c0(m, e, i);
        if cur == 0 {
            return 0;
        }
        if rd(m, cur + 0x18) > o {
            i = cur;
            continue;
        }
        if o >= rd(m, cur + 0x1C) {
            i = cur;
            continue;
        }
        let r = fn_185208(m, cur + 12, o);
        return if r == 0 { cur } else { r };
    }
}

pub fn fn_19b7ac(m: &mut Memory<'_>, r: u32, o: u32) {
    let head = rd(m, r);
    let h0 = rh(m, r + 10) as u32 + o;
    if head == 0 {
        return;
    }
    let r4 = rd(m, r + 4);
    wr(m, h0 + 4, 0);
    wr(m, h0, head);
    let h2 = r4 + rh(m, r + 10) as u32;
    wr(m, h2 + 4, o);
    wr(m, r + 4, o);
    wh(m, r + 8, rh(m, r + 8).wrapping_add(1));
}

pub fn fn_185308(m: &mut Memory<'_>, p: u32, magic: u32, start: u32, end: u32, mode: u32) {
    wr(m, p, magic);
    wr(m, p + 0x20, mode & 0xFF);
    wr(m, p + 0x18, start);
    wr(m, p + 0x1C, end);
    fn_19b9f8(m, p + 12, 4);
    if rb(m, POOL_FLAG) == 0 {
        fn_19b9f8(m, POOL_HEAD, 4);
        wb(m, POOL_FLAG, 1);
    }
    let mut res = POOL_HEAD;
    let mut e = 0u32;
    loop {
        e = fn_1852c0(m, res, e);
        if e == 0 {
            break;
        }
        if rd(m, e + 24) > p {
            res = POOL_HEAD;
            continue;
        }
        if p >= rd(m, e + 28) {
            res = POOL_HEAD;
            continue;
        }
        let r = fn_185208(m, e + 12, p);
        res = (if r == 0 { e } else { r }) + 12;
        break;
    }
    fn_19b7ac(m, res, p);
}

pub fn fn_166fd8(m: &mut Memory<'_>, p: u32, a1: u32, a2: u32, a3: u32) -> u32 {
    let start = a1.wrapping_add(3) & !3;
    let end = a1.wrapping_add(a2) & !3;
    if end >= start && end.wrapping_sub(start) >= 20 {
        fn_185308(m, p, 0x45585048, start, end, a3);
        wh(m, p + 52, 0);
        wh(m, p + 54, 0);
        let b = rd(m, p + 24);
        let span = rd(m, p + 28).wrapping_sub(b).wrapping_sub(16);
        wh(m, b, 0x4652);
        wh(m, b + 2, 0);
        wr(m, b + 4, span);
        wr(m, b + 8, 0);
        wr(m, b + 12, 0);
        wr(m, p + 36, b);
        wr(m, p + 40, b);
        wr(m, p + 44, 0);
        wr(m, p + 48, 0);
        return p;
    }
    0
}

pub fn fn_137a78(m: &mut Memory<'_>, h: &mut impl Calls, o: u32, a: u32, b: u32, c: u32) {
    let q = fn_166fd8(m, o + 0x18, a, b, c as u16 as u32);
    if q == 0 {
        crate::boot_part_b::fn_17177c(m, &mut HeapCalls { inner: h });
    }
    wr(m, o + 0x54, 0);
}

struct LockHost<'a, H: Calls> {
    inner: &'a mut H,
}

impl<H: Calls> crate::boot_part_b::Host for LockHost<'_, H> {
    fn call(&mut self, m: &mut Memory<'_>, f: u32, a: &[u32]) -> u32 {
        match f {
            0x183edc => {
                crate::boot_part_b2::fn_183edc(m, a[0], ARBITER_ADDRESS, self);
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
                    semaphore: ARBITER_ADDRESS,
                    service: 0,
                    command: 0,
                };
                m.fn_165d00(&mut *self.inner, &g, a[0]);
                0
            }
            _ => self.inner.call(m, f, a),
        }
    }
    fn thread_id(&mut self) -> u32 {
        0x1FF82000
    }
}

fn heap_contains(m: &mut Memory<'_>, h: &mut impl Calls, obj: u32, arg: u32) -> u32 {
    let mut lh = LockHost { inner: h };
    crate::boot_part_b::fn_1840fc(m, obj + 0x58, &mut lh);
    let r = crate::speech_part_3::fn_7e715c(m, obj, arg);
    crate::boot_part_b::fn_18423c(m, obj + 0x58, &mut lh);
    r
}

fn call_m24(m: &mut Memory<'_>, h: &mut impl Calls, obj: u32, arg: u32) -> u32 {
    let vt = rd(m, obj);
    if rd(m, vt.wrapping_add(0x18)) == 0x84C160 {
        return heap_contains(m, h, obj, arg);
    }
    let m24 = rd(m, vt.wrapping_add(0x18));
    h.call(m, m24, &[obj, arg])
}

pub fn fn_122c44(m: &mut Memory<'_>, h: &mut impl Calls, voice: u32, arg: u32) -> u32 {
    if call_m24(m, &mut *h, voice, arg) == 0 {
        return 0;
    }
    let mut cand = voice;
    loop {
        let mgr = rd(m, cand + 16);
        if mgr == 0 {
            return cand;
        }
        let mut e = mgr.wrapping_sub(4);
        if e == 0 {
            return cand;
        }
        loop {
            if call_m24(m, &mut *h, e, arg) != 0 {
                cand = e;
                break;
            }

            let m1 = rd(m, cand + 16);
            let mut r1 = 0;
            if m1 != 0 {
                r1 = rd(m, m1);
                if r1 != 0 {
                    r1 = r1.wrapping_sub(4);
                }
            }
            if r1 == e {
                return cand;
            }
            let nx = rd(m, e.wrapping_add(8));
            if nx == 0 {
                return cand;
            }
            e = nx.wrapping_sub(4);
        }
    }
}

pub fn fn_122cd8(m: &mut Memory<'_>, o: u32, item: u32) {
    wr(m, o + 12, item);
    let t = rd(m, item + 16);
    let head = o.wrapping_add(4);
    if t == 0 {
        wr(m, o + 8, head);
        wr(m, o + 4, head);
        wr(m, item + 16, o);
    } else {
        let tt = rd(m, t);
        wr(m, o + 8, t);
        wr(m, tt.wrapping_add(4), head);
        wr(m, o + 4, tt);
        wr(m, t, head);
    }
}

pub fn fn_10edc0(m: &mut Memory<'_>, h: &mut impl Calls, idx: u32, base: u32) -> u32 {
    let off = base.wrapping_add(100);
    let table = HEAP_TABLE_ROOT;
    let fill = rb(m, FILL_FLAG) as u32;
    let p = crate::speech_part_1::fn_11db30(
        m,
        off,
        idx,
        32,
        fill,
        table,
        &mut HeapCalls { inner: &mut *h },
    );
    let voice = rd(m, table.wrapping_add(idx.wrapping_mul(4)));
    if fn_122c44(m, &mut *h, voice, p) != voice {
        return 0;
    }
    let o = p.wrapping_add(off).wrapping_sub(100);
    if o == 0 {
        fn_122cd8(m, o, voice);
        return 0;
    }
    for w in [4u32, 8, 12, 16, 20, 24, 0x54] {
        wr(m, o + w, 0);
    }
    wr(m, o + 0x58, 0);
    wr(m, o + 0x5C, 0);
    wr(m, o + 0x60, 0xFFFF_FFFF);
    wr(m, o, HEAP_VTABLE);
    fn_137a78(m, h, o, p, off.wrapping_sub(100), 0);
    fn_122058(m, o + 0x58);
    fn_122cd8(m, o, voice);
    o
}

pub fn fn_1220e8(
    m: &mut Memory<'_>,
    h: &mut impl Calls,
    obj: u32,
    desc: u32,
    entry: u32,
    ctx: u32,
    stack: u32,
    prio: u32,
    a6: u32,
    a7: u32,
    out: u32,
) -> u32 {
    let d0 = rd(m, desc);
    let f04 = rd(m, desc + 4);
    let f08 = rd(m, desc + 8);
    let f0c = rd(m, desc + 12);
    let size = stack.wrapping_sub(d0) & !7;
    let stk = if a7 == 0 { 0 } else { stack };

    let mut rt = Rt { inner: &mut *h };
    rt.call(m, f04, &[ctx, size]);
    let buf = size.wrapping_sub(20) & !7;
    wr(m, buf, f08);
    wr(m, buf + 16, stk);
    wr(m, buf + 12, size);
    wr(m, buf + 4, f0c);
    wr(m, buf + 8, entry);
    let pr = if prio <= 32 {
        prio + 32
    } else {
        let a = rd(m, TPRIO_A).wrapping_add(prio);
        if a <= 39 {
            a + 24
        } else {
            let b = rd(m, TPRIO_B).wrapping_add(prio);
            if b > 64 {
                0xFFFF_FFFF
            } else {
                b
            }
        }
    };
    wr(m, out, 0);
    let rc = h.call(m, 0x13098c, &[out, rd(m, TSVC_CTX), buf, buf, pr, a6]);
    if rc >> 31 != 0 {
        return rc;
    }
    wr(m, obj, rd(m, out));
    wb(m, obj + 4, 0);
    wb(m, obj + 5, 0);
    0
}

pub fn fn_35aa40(m: &mut Memory<'_>, h: &mut impl Calls, engine: u32, tmp: u32) -> u32 {
    let sub = engine + 0x6000;

    let heap = fn_10edc0(m, &mut *h, 1, 0x200000);
    wr(m, engine + 0x100, heap);
    fn_122058(m, engine + 0x110);

    let raw = fn_129778(m, &mut *h, LOADER_SIZE);
    let loader = if raw != 0 { fn_4e0060(m, &mut *h, raw) } else { 0 };
    wr(m, sub + 0x1BC, loader);

    fn_4dff90(m, &mut *h, loader);
    fn_4dfea8(m, loader);
    fn_4dffd4(m, &mut *h, loader);

    let scr1 = fn_12973c(m, &mut *h, 0xFA0, 1);
    wr(m, sub + 0x19C, scr1);
    let scr2 = fn_12973c(m, &mut *h, 0x3000, 1);
    wr(m, sub + 0x194, scr2);

    wh(m, engine + 0x12C, 0x520);
    fn_4dfef8(m, loader);
    wr(m, engine + 0x130, loader);
    wr(m, engine + 0x134, CB_A);
    wr(m, engine + 0x138, engine + 0xFC);
    wr(m, engine + 0x13C, 0);
    wr(m, engine + 0x140, 0);

    wr(m, engine + 0xFC, GAME_ALLOC_TABLE);
    wr(m, engine + 0x144, CB_B);
    wr(m, engine + 0x148, 0);
    wr(m, engine + 0x14C, loader);
    wr(m, engine + 0x150, CB_C);
    wr(m, engine + 0x154, 0);

    fn_190984(m, &mut *h, engine + 0x12C, engine + 0x124, tmp + 64);
    if rd(m, engine + 0x130) != 0 {
        wr(m, engine + 0x130, 0);
        fn_4dff54(m, &mut *h, loader);
    }

    let mut rt = Rt { inner: &mut *h };
    m.fn_165b88(&mut rt, engine + 0x170, 0);

    wr(m, tmp, 4);
    wr(m, tmp + 4, THREAD_D0);
    wr(m, tmp + 8, THREAD_D1);
    wr(m, tmp + 12, THREAD_D2);
    wr(m, tmp + 16, engine);
    let prio = rb(m, engine + 0x178);
    let trc = fn_1220e8(
        m,
        &mut *h,
        engine + 0x164,
        tmp,
        THREAD_ENTRY,
        tmp + 16,
        THREAD_STACK,
        prio as u32,
        0,
        0,
        tmp + 36,
    );

    if trc & 0x7E00000 != 0x600000 && trc >> 31 != 0 {
        h.call(m, 0x136c5c, &[trc]);
    }

    wr(m, engine + 0x180, rd(m, VOICE_DATA + 36));
    wb(m, engine + 0x158, 0);
    wr(m, engine + 0x15C, 0x6000);
    wr(m, engine + 0x160, engine + 0x18C);

    queue_rotate(m, h, sub, tmp + 20);

    let p = fn_129778(m, &mut *h, 0x4C70);
    let ap = if p != 0 { fn_744200(m, &mut *h, p) } else { 0 };
    wr(m, sub + 0x1B8, ap);
    fn_743be8(m, &mut *h, ap, 0x1800, f32::from_bits(0x467A0000));
    0
}

pub fn queue_rotate(m: &mut Memory<'_>, h: &mut impl Calls, sub: u32, tmp: u32) {
    const Q: u32 = 0x1CC;
    let alloc = rd(m, ALLOC_TABLE);
    let vt = rd(m, alloc);

    let mut rt = Rt { inner: &mut *h };
    let begin = rt.call(m, rd(m, vt + 8), &[alloc, 100 * 32, 32]);
    wr(m, tmp, alloc);
    wr(m, tmp + 4, begin);
    wr(m, tmp + 8, begin);
    wr(m, tmp + 12, 100);
    for off in [0u32, 4, 8, 12] {
        let a = sub + Q + off;
        let b = rd(m, a);
        wr(m, a, rd(m, tmp + off));
        wr(m, tmp + off, b);
    }

    wr(m, tmp + 8, rd(m, tmp + 4));
    let (dalloc, dbegin) = (rd(m, tmp), rd(m, tmp + 4));
    if dalloc != 0 && dbegin != 0 {
        let mut rt = Rt { inner: &mut *h };
        rt.call(m, rd(m, rd(m, dalloc) + 12), &[dalloc, dbegin]);
    }
}

pub fn fn_4dff54(m: &mut Memory<'_>, h: &mut impl Calls, loader: u32) -> u32 {
    let manager = rd(m, MANAGER_GLOBAL);
    let table = rd(m, manager + 0x14);
    let idx = rd(m, loader + 0x340);
    let entry = table + idx.wrapping_mul(0xE8);
    if rb(m, entry + 0xE4) != 4 {
        return 0;
    }
    let ptr = rd(m, entry + 0xC0);
    if ptr != 0 && rd(m, entry + 0xC8) == 0 {
        m.fn_132c90(h, rd(m, entry + 0xD8), ptr);
    }
    for (off, size) in [
        (0u32, 2u32),
        (0xC0, 4),
        (0xC4, 4),
        (0xC8, 4),
        (0xCC, 4),
        (0xD0, 4),
        (0xD4, 4),
        (0xD8, 4),
        (0xDC, 4),
        (0xE0, 4),
        (0xE4, 1),
        (0xE5, 1),
        (0xE6, 1),
    ] {
        for i in 0..size {
            wb(m, entry + off + i, 0);
        }
    }
    1
}

pub const RULES_NAMES: u32 = 0xA7B774;

pub const COMPONENTS_NAMES: u32 = 0xA7B734;

pub const DICT_NAMES: u32 = 0xA7B754;

pub const SEMAPHORE: u32 = 0xA7BB7C;

pub fn fn_11cc0c_full(
    m: &mut Memory<'_>,
    h: &mut impl Calls,
    engine: u32,
    mode: u8,
    roots: &[u32; 9],
    scratch: u32,
) {
    let g = crate::boot::Globals {
        manager: MANAGER_GLOBAL,
        callbacks: ALLOC_TABLE,
        rules: RULES_NAMES,
        components: COMPONENTS_NAMES,
        dictionary: DICT_NAMES,
        context: 0,
        mode: 0,
        semaphore: SEMAPHORE,
        service: 0,
        command: 0,
    };

    let mut rt = Rt { inner: h };
    m.boot_archive_prefix(&mut rt, &g, engine, mode, 0);
    let d = boot_live::RootData { words: *roots };
    boot_live::root_tail_m(m, &mut rt, engine, mode as u32, 0, &d, scratch);
}

pub fn v_alloc(
    m: &mut Memory<'_>,
    h: &mut impl Calls,
    method: u32,
    a: u32,
    b: u32,
    c: u32,
) -> u32 {
    if method == 0 {
        return 0;
    }
    match method {
        0x4A7758 => {
            let mut rh = RouteHost { inner: h };
            m.fn_4a7758(&mut rh, [a, b, c])
        }
        0x39F5E8 => {
            let mut rh = RouteHost { inner: h };
            m.fn_39f5e8(&mut rh, [a, b, c])
        }
        _ => h.call(m, method, &[a, b, c]),
    }
}

pub fn vcall(m: &mut Memory<'_>, h: &mut impl Calls, method: u32, args: &[u32]) -> u32 {
    h.call(m, method, args)
}

pub struct RouteHost<'a, H: Calls> {
    pub inner: &'a mut H,
}

impl<H: Calls> crate::boot::Calls for RouteHost<'_, H> {
    fn call(&mut self, m: &mut Memory<'_>, f: u32, a: &[u32]) -> u32 {

        if f == 0 {
            return 0;
        }
        match f {
            0x4A7758 if a.len() >= 3 => {
                let mut rh = RouteHost { inner: &mut *self.inner };
                m.fn_4a7758(&mut rh, [a[0], a[1], a[2]])
            }
            0x184F20 if a.len() >= 6 => {
                fn_184f20(m, a[0], a[1], a[2], a[3], a[4], a[5])
            }
            0x39F5E8 if a.len() >= 3 => {
                let mut rh = RouteHost { inner: &mut *self.inner };
                m.fn_39f5e8(&mut rh, [a[0], a[1], a[2]])
            }
            _ => {
                if let Some(r) = route_call(m, &mut *self.inner, f, a) {
                    return r;
                }
                self.inner.call(m, f, a)
            }
        }
    }
    fn call_wide(&mut self, m: &mut Memory<'_>, f: u32, a: &[u32]) -> u64 {
        self.inner.call_wide(m, f, a)
    }
}

impl<H: Calls> crate::boot_part_b::Host for RouteHost<'_, H> {
    fn call(&mut self, m: &mut Memory<'_>, f: u32, a: &[u32]) -> u32 {
        crate::boot::Calls::call(self, m, f, a)
    }
    fn thread_id(&mut self) -> u32 {
        0x1FF82000
    }
}

impl<H: Calls> crate::boot_part_b::FormatHost for RouteHost<'_, H> {}

impl<H: Calls> crate::log_live::LogMethodsM for RouteHost<'_, H> {
    fn log_m(
        &mut self,
        method: u32,
        object: u32,
        category: u32,
        level: u32,
        arg: u32,
        format: u32,
        varargs: u32,
        mem: &mut Memory<'_>,
    ) -> u32 {
        vcall(
            mem,
            self.inner,
            method,
            &[object, category, level, arg, format, varargs],
        )
    }
}

pub fn fn_195bec(m: &mut Memory<'_>, dst: u32, val: u32, n: u32) -> u32 {
    for i in 0..n {
        wb(m, dst.wrapping_add(i), val as u8);
    }
    dst
}

pub fn fn_845b40(m: &mut Memory<'_>, a: u32, b: u32) -> u32 {
    if b != 0 {
        let v = rd(m, a);
        wr(m, b, v);
        v
    } else {
        a
    }
}

pub fn fn_3023b0(m: &mut Memory<'_>, dst: u32, n: u32) -> u32 {
    for i in 0..n {
        wb(m, dst.wrapping_add(i), 0);
    }
    dst.wrapping_add(n)
}

pub const SLOT1_HEAP: u32 = 0xB6F5CC;

pub const BOOT_DOT: u32 = 0x8CD208;
pub const BOOT_PARENT: u32 = 0x8CD20C;

pub const SLOT1_FRESH: u32 = 0x15600000;
pub const SLOT1_SPAN: u32 = 0x09FFFF0;

pub const MGR_CAP: u32 = 8;
pub const SLOT_STRIDE: u32 = 0xE8;

pub const MGR_REGISTRY: u32 = 0x40;

pub const REG_ENTRY: u32 = 12;

pub fn bootstrap_slot1_free(m: &mut Memory<'_>) {
    wh(m, SLOT1_FRESH, 0x4652);
    wh(m, SLOT1_FRESH.wrapping_add(2), 0);
    wr(m, SLOT1_FRESH.wrapping_add(4), SLOT1_SPAN);
    wr(m, SLOT1_FRESH.wrapping_add(8), 0);
    wr(m, SLOT1_FRESH.wrapping_add(12), 0);
    wr(m, SLOT1_HEAP.wrapping_add(0x3C), SLOT1_FRESH);
    wr(m, SLOT1_HEAP.wrapping_add(0x40), SLOT1_FRESH);
}

pub fn bootstrap_manager(m: &mut Memory<'_>, h: &mut impl Calls) -> u32 {
    let table = fn_129778(m, h, MGR_CAP.wrapping_mul(SLOT_STRIDE));
    if table == 0 {
        return 0;
    }
    fn_3023b0(m, table, MGR_CAP.wrapping_mul(SLOT_STRIDE));
    let mgr = fn_129778(m, h, 0x1000);
    if mgr == 0 {
        return 0;
    }
    fn_3023b0(m, mgr, 0x1000);
    wr(m, mgr.wrapping_add(4), MGR_CAP);
    wr(m, mgr.wrapping_add(8), 0);
    wr(m, mgr.wrapping_add(0x14), table);
    fn_122058(m, mgr.wrapping_add(0x20));
    fn_122058(m, mgr.wrapping_add(0x2C));
    wr(m, MANAGER_GLOBAL, mgr);
    mgr
}

pub fn registry_find(m: &Memory<'_>, mgr: u32, slot: u32) -> Option<(u32, u32)> {
    let reg = mgr.wrapping_add(MGR_REGISTRY);
    let n = rd(m, reg);
    for i in 0..n.min(MGR_CAP) {
        let e = reg.wrapping_add(4).wrapping_add(i.wrapping_mul(REG_ENTRY));
        let np = rd(m, e);
        if wide_equal(m, np, slot) {
            return Some((rd(m, e.wrapping_add(4)), rd(m, e.wrapping_add(8))));
        }
    }
    None
}

fn wide_equal(m: &Memory<'_>, a: u32, b: u32) -> bool {
    if a == 0 {
        return false;
    }
    for i in 0..95u32 {
        let x = rh(m, a.wrapping_add(i.wrapping_mul(2)));
        let y = rh(m, b.wrapping_add(i.wrapping_mul(2)));
        if x != y {
            return false;
        }
        if x == 0 {
            return true;
        }
    }
    false
}

pub fn route_call(m: &mut Memory<'_>, h: &mut impl Calls, f: u32, a: &[u32]) -> Option<u32> {
    match f {
        0x165C84 => {
            m.fn_165c84(a[0]);
            Some(0)
        }
        0x3023B0 => Some(fn_3023b0(m, a[0], a[1])),
        0x845B40 => Some(fn_845b40(m, a[0], a[1])),
        0x129778 => Some(fn_129778(m, h, a[0])),
        0x1963B8 => {
            if a.len() >= 3 {
                Some(fn_1963b8(m, &mut *h, a[0], a[1], a[2]))
            } else {
                None
            }
        }
        0x177624 => {

            if a.len() >= 5 {
                let scratch = fn_129778(m, &mut *h, 32);
                if scratch == 0 {
                    return None;
                }
                Some(fn_177624(m, &mut *h, a[0], a[1], a[2], a[3], a[4], scratch))
            } else {
                None
            }
        }
        0x176E40 => {
            if a.len() >= 3 {
                Some(fn_176e40(m, &mut *h, a[0], a[1], a[2]))
            } else {
                None
            }
        }
        0x1964E0 => {
            if a.len() >= 8 {
                Some(fn_1964e0(
                    m, &mut *h, a[0], a[1], a[2], a[3], a[4], a[5], a[6], a[7],
                ))
            } else {
                None
            }
        }
        0x174CA8 => {

            if a.len() >= 4 {
                Some(fn_174ca8(m, &mut *h, a[1], a[3]))
            } else {
                None
            }
        }
        0x39F5E8 => {
            let mut rh = RouteHost { inner: h };
            Some(m.fn_39f5e8(&mut rh, [a[0], a[1], a[2]]))
        }
        0x1840FC => {
            let mut rh = RouteHost { inner: h };
            crate::boot_part_b::fn_1840fc(m, a[0], &mut rh);
            Some(0)
        }
        0x18423C => {
            let mut rh = RouteHost { inner: h };
            crate::boot_part_b::fn_18423c(m, a[0], &mut rh);
            Some(0)
        }
        0x183EDC => {
            let mut rh = RouteHost { inner: h };
            crate::boot_part_b2::fn_183edc(m, a[0], ARBITER_ADDRESS, &mut rh);
            Some(0)
        }
        0x165D00 => {

            let g = crate::boot::Globals {
                manager: 0,
                callbacks: 0,
                rules: 0,
                components: 0,
                dictionary: 0,
                context: 0,
                mode: 0,
                semaphore: SEMAPHORE,
                service: 0,
                command: 0,
            };
            m.fn_165d00(h, &g, a[0]);
            Some(0)
        }
        0x191948 => {

            if a.len() >= 3 && (a[1] != 0x684F424A || a[0] == 0) {
                Some(0x80000008)
            } else if a.len() >= 3 {
                let scratch = fn_129778(m, &mut *h, 32);
                if scratch == 0 {
                    return None;
                }
                Some(crate::boot_port_r2::fn_191948(m, &mut *h, a[0], a[1], a[2], scratch))
            } else {
                None
            }
        }
        0x19310C => {

            if a.len() >= 4 && (a[1] != 0x684F424A || a[0] == 0) {
                Some(0x80000008)
            } else if a.len() >= 4 {
                let scratch = fn_129778(m, &mut *h, 0x1DC);
                if scratch == 0 {
                    return None;
                }
                Some(crate::boot_port_r2::fn_19310c(m, &mut *h, a[0], a[1], a[2], a[3], scratch))
            } else {
                None
            }
        }
        0x194454 => {

            if a.len() >= 6 && (a[1] != 0x684F424A || a[0] == 0) {
                Some(0x80000008)
            } else if a.len() >= 6 {
                let scratch = fn_129778(m, &mut *h, 0xC0);
                if scratch == 0 {
                    return None;
                }
                Some(crate::boot_port_r2::fn_194454(
                    m, &mut *h, a[0], a[1], a[2], a[3], a[4], a[5], scratch,
                ))
            } else {
                None
            }
        }
        0x19126C => {

            if a.len() >= 6 && (a[1] != 0x68434C53 || a[0] == 0) {
                Some(0x80000008)
            } else if a.len() >= 6 {
                let scratch = fn_129778(m, &mut *h, 512);
                if scratch == 0 {
                    return None;
                }
                Some(fn_19126c_full(
                    m, &mut *h, a[0], a[1], a[2], a[3], a[4], a[5], scratch,
                ))
            } else {
                None
            }
        }
        0x4A779C => {

            if a.len() >= 2 {
                let target = rd(m, rd(m, a[0]).wrapping_add(8));
                let args = [a[0], a[1], 4];
                if let Some(r) = route_call(m, &mut *h, target, &args) {
                    Some(r)
                } else {
                    Some(h.call(m, target, &args))
                }
            } else {
                None
            }
        }
        0x176F2C | 0x176F5C => {

            if a.is_empty() || a[0] == 0 {
                Some(0x81402007)
            } else {
                let inner = rd(m, rd(m, a[0]).wrapping_add(8));
                let slot = if f == 0x176F2C { 8 } else { 12 };
                let target = rd(m, inner.wrapping_add(slot));
                if target == 0 {
                    Some(0)
                } else {
                    Some(h.call(m, target, &[rd(m, a[0].wrapping_add(8))]))
                }
            }
        }
        0x196394 => {
            if a.len() >= 2 {
                Some(crate::boot_port_r2::fn_196394(m, &mut *h, a[0], a[1]))
            } else {
                None
            }
        }
        0x176054 => {
            if a.len() >= 4 {
                Some(crate::boot_port_r2::fn_176054(
                    m, &mut *h, a[0], a[1], a[2], a[3],
                ))
            } else {
                None
            }
        }
        0x174A04 => {
            if a.len() >= 3 {
                Some(crate::boot_port_r2::fn_174a04(m, &mut *h, a[0], a[1], a[2]))
            } else {
                None
            }
        }
        0x17AD98 => {
            if a.len() >= 3 {
                Some(crate::boot_port_r2::fn_17ad98(m, &mut *h, a[0], a[1], a[2]))
            } else {
                None
            }
        }
        0x195AE8 => {

            if a.len() >= 3 {
                let (dst, src, n) = (a[0], a[1], a[2]);
                for i in 0..n {
                    wb(m, dst.wrapping_add(i), rb(m, src.wrapping_add(i)));
                }
                Some(dst)
            } else {
                None
            }
        }
        0x195BEC => {
            if a.len() >= 3 {
                Some(fn_195bec(m, a[0], a[1], a[2]))
            } else {
                None
            }
        }
        0x1966AC => {
            if a.len() >= 3 {
                Some(crate::boot_port_r2::fn_1966ac(m, &mut *h, a[0], a[1], a[2]))
            } else {
                None
            }
        }
        0x19668C => {
            if a.len() >= 3 {
                Some(crate::boot_port_r2::fn_19668c(m, &mut *h, a[0], a[1], a[2]))
            } else {
                None
            }
        }
        0x196428 => {
            if a.len() >= 2 {
                Some(crate::boot_port_r2::fn_196428(m, &mut *h, a[0], a[1]))
            } else {
                None
            }
        }
        0x196980 => {
            if a.len() >= 2 {
                Some(crate::speech_part_1::fn_196980(m, a[0], a[1]))
            } else {
                None
            }
        }
        0x19690C => {
            if a.len() >= 3 {
                Some(crate::boot_port_r2::fn_19690c(m, &mut *h, a[0], a[1], a[2]))
            } else {
                None
            }
        }
        0x1758F8 => {

            if a.len() >= 3 {
                Some(crate::boot_port_r2::fn_1758f8(m, &mut *h, a[0], a[1], a[2]))
            } else {
                None
            }
        }
        0x1759E0 => {

            if a.len() >= 3 {
                Some(crate::boot_port_r2::fn_1759e0(m, &mut *h, a[0], a[1], a[2]))
            } else {
                None
            }
        }
        0x176F94 => {

            if a.len() >= 4 {
                Some(crate::boot_port_1::fn_176f94(m, a[0], a[1], a[2], a[3]))
            } else {
                None
            }
        }
        0x195DF8 => {

            if a.len() >= 3 {
                Some(crate::boot_port_r2::fn_195df8(m, a[0], a[1], a[2]))
            } else {
                None
            }
        }
        0x1517E8 => {

            if a.len() >= 4 {
                Some(crate::boot_port_r2::fn_1517e8(m, &mut *h, a[0], a[1], a[2], a[3]))
            } else {
                None
            }
        }
        0x15AF34 => {

            if a.len() >= 3 {
                let scratch = fn_129778(m, &mut *h, 8);
                if scratch == 0 {
                    return None;
                }
                Some(crate::boot_port_r2::fn_15af34(m, &mut *h, a[0], a[1], a[2], scratch))
            } else {
                None
            }
        }
        0x17EF1C => {

            if a.len() >= 3 {
                let scratch = fn_129778(m, &mut *h, 4);
                if scratch == 0 {
                    return None;
                }
                Some(crate::boot_port_r2::fn_17ef1c(m, &mut *h, a[0], a[1], a[2], scratch))
            } else {
                None
            }
        }
        0x17F03C => {

            if a.len() >= 9 {
                Some(crate::boot_port_r2::fn_17f03c(
                    m, &mut *h, a[0], a[1], a[2], a[3], a[4], a[5], a[6], a[7], a[8],
                ))
            } else {
                None
            }
        }

        0x163D34 => {
            if a.len() >= 5 {
                let mut rh = RouteHost { inner: h };
                let mode = rb(m, FILL_FLAG) as u32;
                let six = [a[0], a[1], a[2], a[3], a[4], mode];
                let r = crate::boot_part_b2::fn_183524(
                    m, six, 0x8C81A0, 0xA7BB7C, 0xA7BA08, &mut rh,
                );
                Some(r)
            } else {
                None
            }
        }
        0x163F20 => {
            if a.len() >= 4 {
                let mut rh = RouteHost { inner: h };
                Some(m.fn_163f20(&mut rh, [a[0], a[1], a[2], a[3]]))
            } else {
                None
            }
        }
        0x184F84 => {
            if a.len() >= 2 {
                m.boot_heap_release(a[0], a[1]);
                Some(0)
            } else {
                None
            }
        }
        0x19B78C => {
            if a.len() >= 2 {
                let mut rh = RouteHost { inner: h };
                crate::boot_part_b2::fn_19b78c(m, a[0], a[1], &mut rh);
                Some(0)
            } else {
                None
            }
        }
        0x16B540 => {

            if a.len() >= 3 {
                let scratch = fn_129778(m, &mut *h, 256);
                if scratch == 0 {
                    return None;
                }
                let env = crate::speech_part_1::ArchiveEnvironment {
                    dot: BOOT_DOT,
                    parent: BOOT_PARENT,
                    locale: 0,
                };
                Some(crate::boot_part_a::fn_16b554_m(
                    m,
                    a[0].wrapping_add(8),
                    0,
                    a[1],
                    a[2],
                    scratch,
                    &env,
                ))
            } else {
                None
            }
        }
        0x132AB8 => {

            if a.len() < 2 {
                return None;
            }
            let mgr = a[0];
            let slot = a[1];
            let mut rh = RouteHost { inner: &mut *h };
            crate::boot_part_b::fn_1840fc(m, mgr.wrapping_add(32), &mut rh);
            wb(m, slot.wrapping_add(0xe4), 2);
            crate::boot_part_b::fn_18423c(m, mgr.wrapping_add(32), &mut rh);
            let negative = rb(m, slot.wrapping_add(0xe6)) != 0;
            let allocator =
                rd(m, slot.wrapping_add(if negative { 0xdc } else { 0xd8 }));
            let arg = rd(m, slot.wrapping_add(0xe0));
            let arg = if negative { arg.wrapping_neg() } else { arg };
            let mode = rb(m, slot.wrapping_add(0xe5)) as u32;
            let (blob, len) = match registry_find(m, mgr, slot) {
                Some(v) => v,
                None => {
                    return None;
                }
            };
            let mut rh = RouteHost { inner: h };
            let buf = m.fn_163f20(&mut rh, [allocator, len, arg, mode]);
            if buf == 0 {
                return None;
            }
            for i in 0..len {
                wb(m, buf.wrapping_add(i), rb(m, blob.wrapping_add(i)));
            }
            wr(m, slot.wrapping_add(0xc0), buf);
            wr(m, slot.wrapping_add(0xcc), len);
            Some(0)
        }
        _ => None,
    }
}

pub struct Rt<'a, H: Calls> {
    pub inner: &'a mut H,
}

impl<H: Calls> Calls for Rt<'_, H> {
    fn call(&mut self, m: &mut Memory<'_>, f: u32, a: &[u32]) -> u32 {
        if let Some(r) = route_call(m, &mut *self.inner, f, a) {
            return r;
        }
        self.inner.call(m, f, a)
    }
    fn call_wide(&mut self, m: &mut Memory<'_>, f: u32, a: &[u32]) -> u64 {
        self.inner.call_wide(m, f, a)
    }
}

fn rf(m: &Memory<'_>, addr: u32) -> f32 {
    f32::from_le_bytes(
        m.bytes[addr.wrapping_sub(m.base) as usize..][..4].try_into().unwrap(),
    )
}

fn wf(m: &mut Memory<'_>, addr: u32, v: f32) {
    let o = addr.wrapping_sub(m.base) as usize;
    m.bytes[o..o + 4].copy_from_slice(&v.to_le_bytes());
}

pub fn fn_16f8c0(x: u32) -> u32 {
    match x {
        0..=2 => 127,
        3 => 24,
        4 => 42,
        5 => 208,
        6 => 658,
        7 => 18,
        8 => 8,
        9 => 13,
        10 => 52,
        _ => 0xFFFF_FFFF,
    }
}

pub fn fn_16fb50(m: &Memory<'_>, p: u32) -> u32 {
    match rb(m, p + 4) {
        0..=2 => rb(m, p + 4) as u32,
        5 => 3,
        _ => 5,
    }
}

pub fn fn_16fb14(m: &Memory<'_>, p: u32) -> u32 {
    rb(m, 0x8C80C3 + rb(m, p + 5) as u32) as u32
}

pub fn fn_199404(m: &mut Memory<'_>, p: u32, n: u32) -> u32 {
    fn_3023b0(m, p, n)
}

pub fn fn_19c674(m: &Memory<'_>, x: f32) -> f32 {
    let a = x.abs();
    let a = if a.to_bits() >= 0x47800000 {
        crate::math::remainderf(a, 65536.0)
    } else {
        a
    };
    let w = a as u32 & 0xFFFF;
    let frac = a - w as f32;
    let e = 0x8CBFE0 + (w & 255) * 16;
    rf(m, e + 4) + frac * rf(m, e + 12)
}

pub fn fn_1acf74(m: &Memory<'_>, x: f32) -> f32 {
    let a = x.abs();
    let a = if a.to_bits() >= 0x47800000 {
        crate::math::remainderf(a, 65536.0)
    } else {
        a
    };
    let w = a as u32 & 0xFFFF;
    let frac = a - w as f32;
    let e = 0x8CBFE0 + (w & 255) * 16;
    let t = rf(m, e) + frac * rf(m, e + 8);
    if x < 0.0 {
        -t
    } else {
        t
    }
}

pub fn fn_1c1780(m: &mut Memory<'_>, obj: u32, q: u32, kind: u32, arg: i32, val: f32) -> u32 {
    let mut v = arg.wrapping_add(rd(m, obj + 0x64) as i32);
    let i0 = rd(m, q);
    let end = rd(m, q + 4);
    if i0 != end {
        let mut i = i0;
        loop {
            v = v.wrapping_sub(rd(m, q + (i * 5 + 6) * 4) as i32);
            i += 1;
            if i >= 256 {
                i = 0;
            }
            if end == i {
                break;
            }
        }
        if v < 0 {
            v = 0;
        }
    }
    let slot = q + (end * 5) * 4;
    wr(m, slot + 8, kind);
    wr(m, slot + 0x18, v as u32);
    wf(m, slot + 0xC, val);
    let mut w = rd(m, q + 4).wrapping_add(1);
    if w >= 256 {
        w = 0;
    }
    wr(m, q + 4, w);
    0
}

pub fn fn_1c1824(
    m: &mut Memory<'_>,
    obj: u32,
    q: u32,
    kind: u32,
    arg: i32,
    x: f32,
    y: f32,
) -> u32 {
    let mut v = arg.wrapping_add(rd(m, obj + 0x64) as i32);
    let i = rd(m, q);
    let end = rd(m, q + 4);
    if i != end {
        let mut j = i;
        loop {
            v = v.wrapping_sub(rd(m, q + (j * 5 + 6) * 4) as i32);
            j += 1;
            if j >= 256 {
                j = 0;
            }
            if end == j {
                break;
            }
        }
        if v < 0 {
            v = 0;
        }
    }
    let slot = q + (end * 5) * 4;
    wr(m, slot + 8, kind);
    wr(m, slot + 0x18, v as u32);
    wf(m, slot + 0xC, x);
    wf(m, slot + 0x10, y);
    let mut w = rd(m, q + 4).wrapping_add(1);
    if w >= 256 {
        w = 0;
    }
    wr(m, q + 4, w);
    0
}

pub fn fn_1bd29c(m: &mut Memory<'_>, obj: u32, arg: i32, v: f32) -> u32 {
    fn_1c1780(m, obj, obj + 0xE8, 8, arg, v)
}
pub fn fn_1bd27c(m: &mut Memory<'_>, obj: u32, arg: i32, v: f32) -> u32 {
    fn_1c1780(m, obj, obj + 0xE8, 0, arg, v)
}
pub fn fn_1bd26c(m: &mut Memory<'_>, obj: u32, arg: i32, v: f32) -> u32 {
    fn_1c1780(m, obj, obj + 0xE8, 2, arg, v)
}
pub fn fn_1bd28c(m: &mut Memory<'_>, obj: u32, arg: i32, v: f32) -> u32 {
    fn_1c1780(m, obj, obj + 0xE8, 1, arg, v)
}
pub fn fn_1c18c8(m: &mut Memory<'_>, obj: u32, arg: i32, v: f32) -> u32 {
    fn_1c1780(m, obj, obj + 0xE8, 3, arg, v)
}
pub fn fn_1bd2ac(m: &mut Memory<'_>, obj: u32, arg: i32, v: f32) -> u32 {
    fn_1c1780(m, obj, obj + 0xE8, 7, arg, v)
}
pub fn fn_1c16c4(m: &mut Memory<'_>, obj: u32, arg: i32) -> u32 {
    fn_1c1780(m, obj, obj + 0xE8, 14, arg, 0.0)
}
pub fn fn_1c1814(m: &mut Memory<'_>, obj: u32, arg: i32, x: f32, y: f32) -> u32 {
    fn_1c1824(m, obj, obj + 0xE8, 11, arg, x, y)
}
pub fn fn_1bd2bc(m: &mut Memory<'_>, obj: u32, arg: i32, x: f32, y: f32) -> u32 {
    fn_1c1824(m, obj, obj + 0xE8, 12, arg, x, y)
}
pub fn fn_1bdff0(m: &mut Memory<'_>, obj: u32, en: u32, harm: u32, delay: u32, ratio: f32) -> u32 {
    fn_1c1824(
        m,
        obj,
        obj + 232,
        harm + 4,
        delay as i32,
        if en != 0 { 1.0 } else { 0.0 },
        ratio,
    )
}

pub fn fn_1c16dc(m: &mut Memory<'_>, h: &mut impl Calls, o: u32) {
    if rb(m, o + 4) == 0 {
        return;
    }
    for &base in &[o + 0xE8, o + 0x14F0] {
        while rd(m, base) != rd(m, base + 4) {
            let i = rd(m, base);
            h.call(m, 0x1BCD84, &[o, base, rd(m, base + (i * 5 + 6) * 4)]);
        }
    }
}

pub fn fn_1b8890(m: &mut Memory<'_>, p: u32) {
    let b = rd(m, p + 0x1C);
    let n = rd(m, b + 8);
    for i in 0..n {
        wh(m, rd(m, b + 4) + i * 2, 0);
    }
    wr(m, b + 12, 0);
    wf(m, p + 0x20, 0.0);
}

pub fn fn_1b89f8(m: &mut Memory<'_>, o: u32) {
    for &off in &[0x34u32, 0x38, 0x3C] {
        let b = rd(m, o + off);
        let n = rd(m, b + 8);
        for i in 0..n {
            wf(m, rd(m, b + 4) + i * 4, 0.0);
        }
        wr(m, b + 12, 0);
    }
    wf(m, o + 0x4C, 0.0);
}

pub fn fn_1be020(m: &mut Memory<'_>, h: &mut impl Calls, p: u32) {
    const V1: f32 = 1.0;
    wb(m, p + 0x4BD0, 0);
    wf(m, p + 0x4BD4, V1);
    wb(m, p + 0x4BD8, 0);
    wb(m, p + 0x4BD9, 0);
    wr(m, p + 0x4BDC, 0);
    wf(m, p + 0x4BE0, V1);
    wf(m, p + 0x4BE4, V1);
    wf(m, p + 0x4BE8, V1);
    wf(m, p + 0x4BEC, V1);
    wf(m, p + 0x4BF0, V1);
    wf(m, p + 0x4BF4, V1);
    wf(m, p + 0x4BF8, 0.05);
    wf(m, p + 0x4BFC, V1);
    wf(m, p + 0x4C00, 185.0);
    wf(m, p + 0x4C04, V1);
    wf(m, p + 0x4C08, V1);
    wf(m, p + 0x4C0C, V1);
    wf(m, p + 0x4C10, V1);
    wf(m, p + 0x4C14, V1);
    wf(m, p + 0x4C18, V1);
    wf(m, p + 0x4C1C, 120.0);
    wb(m, p + 0x4C30, 0);
    wr(m, p + 0x4C34, 1);
    let vol = rf(m, p + 0x4BD4) * rf(m, p + 0x4BFC);
    fn_1c16dc(m, h, p);
    fn_1c16c4(m, p, 0);
    fn_1bd29c(m, p, 0, vol);
    fn_1bd27c(m, p, 0, rf(m, p + 0x4BE4));
    fn_1bd26c(m, p, 0, rf(m, p + 0x4C04));
    fn_1bd28c(m, p, 0, rf(m, p + 0x4BE8));
    fn_1c18c8(m, p, 0, rf(m, p + 0x4C00));
    let e0 = rf(m, p + 0x4BEC);
    fn_1bdff0(m, p, (e0.to_bits() != 0x3F800000) as u32, 0, 0, e0);
    let e1 = rf(m, p + 0x4BF0);
    fn_1bdff0(m, p, (e1.to_bits() != 0x3F800000) as u32, 1, 0, e1);
    let e2 = rf(m, p + 0x4BF4);
    fn_1bdff0(m, p, (e2.to_bits() != 0x3F800000) as u32, 2, 0, e2);
    fn_1bd2ac(m, p, 0, rf(m, p + 0x4BF8));
    fn_1c1814(m, p, 0, rf(m, p + 0x4C20), rf(m, p + 0x4C24));
    fn_1bd2bc(m, p, 0, rf(m, p + 0x4C28), rf(m, p + 0x4C2C));
    fn_1b8890(m, p + 0x31F4);
    fn_1b89f8(m, p + 0x31A4);
}

pub fn fn_129760(m: &mut Memory<'_>, h: &mut impl Calls, size: u32) -> u32 {
    fn_129778(m, h, size)
}

pub fn fn_741d64(m: &mut Memory<'_>, h: &mut impl Calls, p: u32) {
    if rb(m, p + 4) == 0 {
        return;
    }
    for &off in &[0x2Cu32, 0x30, 0x3C, 0x50, 0x24, 0x20] {
        let q = rd(m, p + off);
        if q != 0 {
            h.call(m, 0x11DA2C, &[q]);
            wr(m, p + off, 0);
        }
    }
    let f44 = rd(m, p + 0x44);
    if f44 != 0 {
        let r = h.call(m, 0x7414C4, &[f44]);
        h.call(m, 0x302B1C, &[r]);
        wr(m, p + 0x44, 0);
    }
    wb(m, p + 4, 0);
}

pub fn fn_741424(m: &mut Memory<'_>, h: &mut impl Calls, s: u32, a: u32, n: u32) -> u32 {
    wf(m, s + 0x1C, 0.0);
    wf(m, s + 0x20, 0.0);
    wf(m, s + 0x24, 0.0);
    wf(m, s + 0x28, fn_19c674(m, 0.0320000723) * 2.0);
    wr(m, s + 4, n);
    wr(m, s + 0x30, 0);
    wr(m, s, a);
    let q = fn_129760(m, h, n * 4);
    wr(m, s + 0x30, q);
    wr(m, s + 0x18, 0xFFFF_FFFF);
    wr(m, s + 8, a);
    wr(m, s + 0xC, 0);
    wb(m, s + 0x10, 1);
    wr(m, s + 0x14, 0);
    s
}

pub fn fn_741bf8(m: &mut Memory<'_>, o: u32) {
    if rb(m, o + 4) == 0 {
        return;
    }
    wr(m, o + 0xE8, 0);
    wr(m, o + 0xEC, 0);
    wr(m, o + 0x14F0, 0);
    wr(m, o + 0x14F4, 0);
    let n = rd(m, o + 0x40);
    let arr = rd(m, o + 0x3C);
    for i in 0..n {
        wr(m, arr + i * 4, 0);
    }
    wb(m, o + 0x48, 1);
    wr(m, o + 0x54, 0);
    wf(m, o + 0x58, 0.0);
    wf(m, o + 0x5C, 0.0);
    wr(m, o + 0x60, 0);
    wr(m, o + 0x64, 0);
    wr(m, o + 0x68, 0);
    wh(m, o + 0x34, 0);
    wh(m, o + 0x36, 0);
    let rate = rf(m, o + 8);
    wf(m, o + 0x6C, 1.0);
    wf(m, o + 0x70, 1.0);
    wf(m, o + 0x74, 1.0);
    wf(m, o + 0x78, 1.0);
    wf(m, o + 0x7C, rate * (1.0 / 185.0));
    wf(m, o + 0x80, rate * (1.0 / 185.0));
    wf(m, o + 0x84, 0.0);
    wf(m, o + 0x88, 0.0);
    wf(m, o + 0x8C, 1.0);
    wf(m, o + 0x90, 1.0);
    wf(m, o + 0x94, 1.0);
    wf(m, o + 0x98, 1.0);
    wf(m, o + 0x9C, 1.0);
    wf(m, o + 0xA0, 1.0);
    wf(m, o + 0xA4, 0.0);
    wr(m, o + 0xA8, 0);
    wr(m, o + 0xAC, 0);
    wf(m, o + 0xB0, 0.0);
    wr(m, o + 0xB4, 0);
    wr(m, o + 0xB8, 0);
    wb(m, o + 0xBC, 0);
    wf(m, o + 0xC0, 1.0);
    wf(m, o + 0xCC, 0.0);
    wb(m, o + 0xBD, 0);
    wf(m, o + 0xC4, 1.0);
    wf(m, o + 0xD0, 0.0);
    wb(m, o + 0xBE, 0);
    wf(m, o + 0xC8, 1.0);
    wf(m, o + 0xD4, 0.0);
    wb(m, o + 0xE0, 0);
    wf(m, o + 0xE4, 0.0);
    wr(m, o + 0x28F8 + 0x10, 0x80000000u32);
    wr(m, o + 0x28F8 + 0x24, 0x80000000u32);
}

pub fn fn_741994(m: &mut Memory<'_>, h: &mut impl Calls, p: u32, a: i32, b: i32, c: i32, d: f32) -> u32 {

    if rb(m, p + 4) != 0 {
        fn_741d64(m, h, p);
    }
    wf(m, p + 8, d);
    wf(m, p + 28, 0.125);
    wf(m, p + 24, 24.0);
    wr(m, p + 12, (d * 0.005) as i32 as u32);
    let q0 = fn_129778(m, h, 52);
    let q = if q0 != 0 {
        fn_741424(m, h, q0, (d / c as f32) as i32 as u32, (d / b as f32) as i32 as u32)
    } else {
        0
    };
    wr(m, p + 68, q);
    wr(m, p + 20, rd(m, q + 4) * 2);
    if a < rd(m, p + 20) as i32 {
        return 1;
    }
    let f16 = rd(m, q + 4) * 2 * 4;
    wr(m, p + 16, f16);
    let sz = f16 * 4 + 4;
    let f80 = fn_129760(m, h, sz);
    wr(m, p + 80, f80);
    wr(m, p + 40, f16);
    let t36 = fn_129760(m, h, f16 * 4);
    wr(m, p + 36, t36);
    for i in 0..rd(m, p + 40) {
        let v = 0.5 - fn_19c674(m, i as f32 * 6.2831853 / (rd(m, p + 40).wrapping_sub(1)) as f32 * 40.743664) * 0.5;
        wf(m, t36 + i * 4, v);
    }
    let t32 = fn_129760(m, h, 4096);
    wr(m, p + 32, t32);
    for i in 0..1024u32 {
        wf(
            m,
            t32 + i * 4,
            fn_1acf74(m, i as f32 * 6.2831853 * 0.0009775171 * 40.743664),
        );
    }
    wr(m, p + 56, a as u32);
    let f44 = fn_129760(m, h, (a as u32).wrapping_mul(2));
    wr(m, p + 44, f44);
    wr(m, p + 100, 0);
    let qq = rd(m, p + 68);
    let f64 = (rd(m, p + 16) * 2) as f32 + a as f32 * 24.0
        + (1.0 + rd(m, qq + 4) as f32 * 8.0) as i32 as f32;
    let f64i = f64 as i32 as u32;
    wr(m, p + 64, f64i);
    let f60 = fn_129760(m, h, f64i.wrapping_mul(4));
    wr(m, p + 60, f60);
    wb(m, p + 4, c as u8);
    fn_741bf8(m, p);
    0
}

pub fn fn_741e1c(m: &mut Memory<'_>, p: u32) {
    wr(m, p, 0xA49C88);
    wb(m, p + 4, 0);
    for &o in &[0x20u32, 0x24, 0x2C, 0x30, 0x3C, 0x44, 0x50] {
        wr(m, p + o, 0);
    }
    wb(m, p + 0x2920, 1);
}

pub fn fn_75bcf8(m: &mut Memory<'_>, h: &mut impl Calls, p: u32) -> u32 {
    wr(m, p, 0xA34CD8);
    wr(m, p + 4, 16000);
    let q = fn_129778(m, h, 16);
    let b = if q == 0 {
        0
    } else {
        wr(m, q, 0xA24804);
        wr(m, q + 12, 0);
        let data = fn_129760(m, h, 16000);
        wr(m, q + 4, data);
        wr(m, q + 8, 8000);
        for i in 0..8000u32 {
            wh(m, data + i * 2, 0);
        }
        wr(m, q + 12, 0);
        q
    };
    wr(m, p + 28, b);
    wf(m, p + 32, 0.0);
    wr(m, p + 8, 127);
    wr(m, p + 12, 64);
    wr(m, p + 16, 0);
    wr(m, p + 20, 8000);
    p
}

pub fn fn_741580(m: &mut Memory<'_>, p: u32) -> u32 {
    wr(m, p, 0xA49C58);
    wb(m, p + 4, 0);
    wr(m, p + 8, 0);
    wr(m, p + 12, 0);
    wr(m, p + 16, 0xA49C68);
    wr(m, p + 20, 0);
    fn_3023b0(m, p + 0x18, 0x400);
    let s = p + 0x418;
    wr(m, s, 0xA49C48);
    wr(m, s + 0x404, 0);
    wr(m, s + 0x408, 0);
    wr(m, s + 0x40C, 0);
    for i in 0..0x80u32 {
        wr(m, s + 4 + i * 4, 0);
        wr(m, s + 0x204 + i * 4, 0);
    }
    wr(m, s + 0x410, 0);
    wr(m, p + 0x714, 0);
    wr(m, p + 0xB18, 0);
    wr(m, p + 0xF1C, 0);
    wr(m, p + 0xF20, 0);
    wr(m, p + 0xF24, 0);
    wr(m, p + 0xF68, 0);
    wr(m, p + 0xF70, 0xFFFF_FFFF);
    wr(m, p + 0xF6C, 0);
    wr(m, p + 0xF80, 0);
    fn_3023b0(m, p + 0x82C, 0x100);
    fn_3023b0(m, p + 0x92C, 0x200);
    p
}

pub fn fn_13fc74(m: &mut Memory<'_>, h: &mut impl Calls, p: u32) -> u32 {
    wr(m, p + 4, 0x7FD8);
    wr(m, p, 0xA35B7C);
    wr(m, p + 68, 16384);
    wr(m, p + 64, 0);
    for &off in &[52u32, 56, 60] {
        let q = fn_129778(m, h, 16);
        let d = if q == 0 {
            0
        } else {
            wr(m, q, 0xA247EC + 8);
            wr(m, q + 12, 0);
            let f4 = fn_129760(m, h, 4096);
            wr(m, q + 4, f4);
            wr(m, q + 8, 1024);
            for i in 0..1024u32 {
                wf(m, f4 + i * 4, 0.0);
            }
            wr(m, q + 12, 0);
            q
        };
        wr(m, p + off, d);
    }
    let sine = fn_129760(m, h, 2048);
    wr(m, p + 72, sine);
    for i in 0..1024u32 {
        let v = fn_1acf74(m, i as f32 * 6.2831802 * 0.0009765625 * 40.743664) * 32767.0;
        wh(m, sine + i * 2, v as i16 as u16);
    }
    wf(m, p + 76, 0.0);
    for i in 0..11u32 {
        wr(m, p + 8 + i * 4, fn_16f8c0(i));
    }
    p
}

pub fn fn_744200(m: &mut Memory<'_>, h: &mut impl Calls, p: u32) -> u32 {
    fn_741e1c(m, p);
    let q1 = fn_13fc74(m, h, p + 0x31A4);
    let q2 = fn_75bcf8(m, h, q1 + 0x50);
    let b = q2.wrapping_sub(0x3000).wrapping_sub(0x1F4);
    let s = b + 0x3218;
    wr(m, s, 0xA49C78);
    wr(m, s + 0x181 * 4, 0);
    wr(m, s + 0x182 * 4, 0);
    let p2 = fn_741580(m, b + 0x3828);
    let q = p2.wrapping_sub(0x3828);
    wr(m, q + 0x4BC0, q);
    q
}

pub fn fn_743be8(m: &mut Memory<'_>, h: &mut impl Calls, p: u32, arg: i32, f: f32) -> u32 {
    let eng = fn_16fb50(m, 0xACB580);
    let lang = fn_16fb14(m, 0xACB580);
    let (mut v2, mut v3) = (45i32, 360i32);
    if eng != 1 {
        match lang {
            3 | 4 => {
                v3 = 420;
            }
            5 => {
                v2 = 55;
                v3 = 430;
            }
            6 => {
                v2 = 60;
                v3 = 460;
            }
            _ => {}
        }
    }
    fn_741994(m, h, p, arg, v2, v3, f);
    wf(m, p + 0x4BC4, f);
    fn_199404(m, 0xB6B300, 256);
    for i in 0..31u32 {
        wb(m, 0xB6B300 + rh(m, 0xA7B798 + i * 2) as u32, 1);
    }
    if rd(m, 0xA7B794) == 0 {
        let lazy = fn_129760(m, h, 0x204CC);
        wr(m, 0xA7B794, lazy);
    }
    wr(m, p + 0x4C40, 0);
    wf(m, p + 0x4C44, 1.3333334);
    wf(m, p + 0x4C48, 0.6666667);
    wr(m, p + 0x4C64, 2);
    wf(m, p + 0x4C54, 0.75);
    wf(m, p + 0x4C58, 1.5);
    wf(m, p + 0x4C5C, 0.75);
    wr(m, p + 0x4C68, 3);
    fn_1be020(m, h, p);
    0
}

pub fn fn_195ae8(m: &mut Memory<'_>, dst: u32, src: u32, n: u32) -> u32 {
    for i in 0..n {
        wb(m, dst.wrapping_add(i), rb(m, src.wrapping_add(i)));
    }
    dst
}

pub fn fn_160fec(code: u32) -> u32 {
    if code < 0x80000000 {
        return 0;
    }
    let r = code & 0x80001FFF;
    if r == 0 {
        return 0x8000010E;
    }
    let known = (1..=6).contains(&r)
        || r == 0x1FFF
        || (0x80000000..=0x80000013).contains(&r)
        || (0x80000100..=0x80000113).contains(&r)
        || matches!(
            r,
            0x80000200
                | 0x80000300
                | 0x80000400
                | 0x8000040E
                | 0x80000416
                | 0x80000880
                | 0x80000881
        )
        || (0x80000800..=0x8000080C).contains(&r);
    if known {
        r
    } else {
        0x80000000
    }
}

pub fn fn_196318(m: &mut Memory<'_>, h: &mut impl Calls, iface: u32, ctx: u32, out: u32) -> u32 {
    if iface == 0 {
        return 0x80B02007;
    }
    let node = v_alloc(m, h, rd(m, iface + 4), ctx, 1, 8);
    wr(m, out, node);
    if node == 0 {
        return 0x80B0200A;
    }
    wr(m, node, iface);
    wr(m, node + 4, ctx);
    0
}

pub fn fn_196368(m: &mut Memory<'_>, h: &mut impl Calls, node: u32) -> u32 {
    if node == 0 {
        return 0x80B02007;
    }
    let iface = rd(m, node);
    let ctx = rd(m, node + 4);
    if rd(m, iface + 12) == 0 {
        return 0;
    }
    h.call(m, rd(m, iface + 12), &[ctx, node]);
    0
}

pub fn fn_1963b8(m: &mut Memory<'_>, h: &mut impl Calls, p: u32, a: u32, b: u32) -> u32 {
    if p == 0 || a == 0 || b == 0 {
        return 0;
    }
    let vt = rd(m, p);
    v_alloc(m, h, rd(m, vt + 4), rd(m, p + 4), a, b)
}

pub fn fn_15174c(m: &mut Memory<'_>, sel: u32, out: u32) -> u32 {
    if sel == 0 || sel == 1 {
        wr(m, out, 0x8D77B4);
        0
    } else {
        0x88F02001
    }
}

pub fn fn_15db5c(m: &mut Memory<'_>, sel: u32, out: u32) -> u32 {
    if sel == 0 || sel == 1 {
        wr(m, out, 0x8D81F0);
        0
    } else {
        0x89202001
    }
}

pub fn fn_157a54(m: &mut Memory<'_>, sel: u32, out: u32) -> u32 {
    let lit_a = rd(m, 0x157EB4);
    let lit_b = rd(m, 0x157EB8);
    if sel == 0 || sel == 3 {
        wr(m, out, lit_b);
        0
    } else {
        lit_a
    }
}

pub fn fn_174988(m: &mut Memory<'_>, sel: u32, out: u32) -> u32 {
    if sel == 0 || sel == 1 {
        wr(m, out, rd(m, 0x1749A4));
        0
    } else {
        rd(m, 0x1749A8)
    }
}

pub fn fn_160aa8(m: &mut Memory<'_>, id: u32, out: u32) -> u32 {
    if id == 0 || id == 1 {
        wr(m, out, rd(m, 0x160B04));
        0
    } else {
        rd(m, 0x160B08)
    }
}

pub fn fn_173dc0(
    m: &mut Memory<'_>,
    h: &mut impl Calls,
    a: u32,
    b: u32,
    c: u32,
    d: u32,
    va: u32,
    params: u32,
) -> u32 {
    if a == 0 {
        return 0;
    }
    if b == 0 {
        return 0x84B02006;
    }
    let mut rh = RouteHost { inner: h };
    crate::boot_part_b::fn_173c70(m, a, b, c, d, va, params, &mut rh)
}

pub fn fn_14de40(m: &mut Memory<'_>, h: &mut impl Calls, log: u32, handle: u32, typ: u32) -> u32 {
    if log == 0 {
        return 0;
    }
    if rd(m, log + 0x38) != 0 {
        return 0x84B02000;
    }
    wr(m, log + 0x34, handle);
    wr(m, log + 0x38, typ);
    if typ != 0 {
        let sev = rd(m, typ + 0x28);
        if sev != 0 {
            let v = vcall(m, h, sev, &[handle]);
            if v > rb(m, log + 0x30) as u32 {
                wb(m, log + 0x30, v as u8);
            }
        }
    }
    if rb(m, log + 0x40) == 0
        && typ != 0
        && rd(m, typ + 0x30) != 0
        && rd(m, typ + 0x34) != 0
    {
        wb(m, log + 0x40, 1);
    }
    0
}

pub fn fn_1759a0(
    m: &mut Memory<'_>,
    h: &mut impl Calls,
    a: u32,
    b: u32,
    s: u32,
    scratch: u32,
) -> u32 {
    let str_ptr = if s == 0 {
        wb(m, scratch, 0);
        scratch
    } else {
        s
    };
    let n = crate::boot_port_1::fn_195d90(m, str_ptr).wrapping_add(1);
    let mut hh = HeapCalls { inner: &mut *h };
    crate::speech_part_1::fn_175170(m, a, b, str_ptr, n, 0, scratch + 8, &mut hh)
}

pub fn fn_1757b8(
    m: &mut Memory<'_>,
    h: &mut impl Calls,
    p: u32,
    key: u32,
    val: u32,
    scratch: u32,
) -> u32 {
    crate::boot_port_1::fn_176c58(m, val, scratch, 10);
    let n = crate::boot_port_1::fn_195d90(m, scratch).wrapping_add(1);
    let mut hh = HeapCalls { inner: &mut *h };
    crate::speech_part_1::fn_175170(m, p, key, scratch, n, 0, scratch + 40, &mut hh)
}

pub fn fn_160b0c(m: &mut Memory<'_>, h: &mut impl Calls, heap: u32, out: u32) -> u32 {
    if out == 0 {
        return 0x87F66006;
    }
    let vt = rd(m, heap);
    let obj = rd(m, heap + 4);
    let o = vcall(m, h, rd(m, vt), &[obj, 8]);
    if o == 0 {
        return 0x87F6600A;
    }
    wr(m, o + 4, heap);
    wh(m, o, 0);
    wh(m, o + 2, 0);
    wr(m, out, o);
    0
}

pub fn fn_176e40(m: &mut Memory<'_>, h: &mut impl Calls, o: u32, alloc: u32, out: u32) -> u32 {
    if o == 0 || alloc == 0 || out == 0 {
        return 0x81402007;
    }
    let n = fn_1963b8(m, h, alloc, 1, 12);
    wr(m, out, n);
    if n == 0 {
        return 0x8140200A;
    }
    wr(m, n, o);
    wr(m, n + 4, alloc);
    let create = rd(m, rd(m, o + 8));
    if create != 0 {
        let rc = vcall(m, h, create, &[rd(m, o + 12), rd(m, o + 4), n + 8]);
        if rc >= 0x80000000 {
            h.call(m, 0x196428, &[alloc, n]);
            wr(m, out, 0);
        }
        return rc;
    }
    0
}

pub const BROKER_MODULE_NAME: u32 = 0x8D7668;
pub const FMT_BRK_BEGIN_ID: u32 = 0x177C5C;
pub const FMT_BRK_BEGIN_NULL: u32 = 0x177C88;
pub const FMT_BRK_END: u32 = 0x177CB4;

pub fn fn_177b7c(
    m: &mut Memory<'_>,
    h: &mut impl Calls,
    ident: u32,
    broker: u32,
    out: u32,
    scratch: u32,
) -> u32 {
    wr(m, out, 0);
    if broker == 0 {
        return 0x81506007;
    }
    let context = rd(m, broker + 8);
    let logger = rd(m, context + 12);
    let heap = rd(m, context + 4);
    let mut rh = RouteHost { inner: h };
    if ident != 0 {
        wr(m, scratch, ident);
        crate::log_live::fn_1738e8(
            m,
            logger,
            BROKER_MODULE_NAME,
            4,
            0,
            FMT_BRK_BEGIN_ID,
            scratch,
            &mut rh,
        );
    } else {
        crate::log_live::fn_1738e8(
            m,
            logger,
            BROKER_MODULE_NAME,
            4,
            0,
            FMT_BRK_BEGIN_NULL,
            scratch,
            &mut rh,
        );
    }
    let object = fn_1963b8(m, &mut *h, heap, 1, 12);
    if object == 0 {
        fn_173dc0(m, &mut *h, logger, BROKER_MODULE_NAME, 200, 0, scratch, scratch + 4);
        return 0x8150600A;
    }
    wr(m, object, logger);
    wr(m, object + 4, broker);
    wr(m, out, object);
    let mut rh = RouteHost { inner: h };
    crate::log_live::fn_1738e8(m, logger, BROKER_MODULE_NAME, 4, 0, FMT_BRK_END, scratch, &mut rh);
    0
}

pub fn fn_184f20(
    m: &mut Memory<'_>,
    p: u32,
    a: u32,
    b: u32,
    c: u32,
    d: u32,
    e: u32,
) -> u32 {
    let sub = p.wrapping_add(24);
    crate::boot_port_1::fn_1852d8(m, sub, c, 0);
    crate::boot_port_1::fn_1852e8(m, sub, d, 0);
    crate::boot_port_1::fn_1853c4(m, sub, e, 0);
    let r = crate::boot_port_1::fn_1850c0(m, sub, a, b as i32);
    if r != 0 {
        wr(m, p + 84, rd(m, p + 84).wrapping_add(1));
    }
    r
}

pub fn fn_19611c(m: &mut Memory<'_>, hp0: u32, hp1: u32, out: u32) -> u32 {
    let b0 = rd(m, 0x176FE8);
    let b1 = rd(m, 0x176FEC);
    if hp0 == b0 && hp1 == b1 {
        wr(m, out, rd(m, 0x196304));
        return 0;
    }
    let r = crate::boot_port_1::fn_176f94(m, hp0, hp1, rd(m, 0x196308), 0x20);
    if r >= 0x80000000 {
        let mut rr = r;
        if rr & 0x1FFF == 8 {
            rr = 0x81602008;
        }
        wr(m, out, rd(m, 0x196304));
        return rr;
    }
    wr(m, out, hp0);
    if hp0 == 0 || rh(m, hp0) != 0x1E61 {
        wr(m, out, rd(m, 0x196304));
        return 0x81602008;
    }
    r
}

pub const W_USER: u32 = 0x20;
pub const W_RESOURCE: u32 = 0x24;
pub const W_PROVIDER: u32 = 0x34;
pub const W_ALLOCATOR: u32 = 0x4;
pub const W_CALLBACK: u32 = 0x9C;
pub const W_CALLBACK_FLAGS: u32 = 0xA4;
pub const W_ENGINE: u32 = 0xA8;
pub const W_CONTEXT: u32 = 0xB0;
pub const W_LOGGER: u32 = 0xB4;
pub const W_STREAM: u32 = 0xB8;
pub const W_CALLBACK_API: u32 = 0xBC;
pub const W_CALLBACK_HANDLE: u32 = 0xC0;
pub const W_VOICE_API: u32 = 0xC8;
pub const W_VOICE: u32 = 0xCC;
pub const W_SYNTHSTREAM: u32 = 0xD4;
pub const W_LINGDB: u32 = 0xD8;

pub fn fn_190984(
    m: &mut Memory<'_>,
    h: &mut impl Calls,
    req: u32,
    out: u32,
    scratch: u32,
) -> u32 {
    if req == 0 || out == 0 {
        return 0x80000006;
    }
    let status0 = 0x80002300u32;
    if rh(m, req) != 0x520 {
        return fn_160fec(status0);
    }
    let allocator = rd(m, req + 8);
    let resource = rd(m, req + 0x18);
    if allocator == 0 || resource == 0 {
        return fn_160fec(status0);
    }
    fn_195bec(m, out, 0, 8);
    wr(m, scratch + 8, 0);
    wr(m, scratch + 12, 0);
    wr(m, scratch + 16, 0);

    let alloc_method = rd(m, allocator + 4);
    let worker = v_alloc(m, h, alloc_method, rd(m, req + 12), 1, 0xE0);
    if worker == 0 {
        return fn_160fec(0x8000000A);
    }
    wr(m, worker, 0x73434C53);
    wr(m, worker + 4, rd(m, allocator));
    wr(m, worker + 8, rd(m, allocator + 4));
    wr(m, worker + 12, rd(m, allocator + 8));
    wr(m, worker + 16, rd(m, allocator + 12));
    let opt_params = rd(m, req + 16);
    if opt_params != 0 {
        for i in 0..4u32 {
            wr(m, worker + W_RESOURCE + i * 4, rd(m, opt_params + i * 4));
        }
    }
    for i in 0..6u32 {
        let src_off = if i < 5 { i * 4 } else { 0x18 };
        wr(m, worker + W_PROVIDER + [0, 4, 8, 12, 16, 24][i as usize], rd(m, resource + src_off));
    }
    let opt_source = rd(m, req + 28);
    if opt_source != 0 {
        for i in 0..5u32 {
            wr(m, worker + 0x5C + i * 4, rd(m, opt_source + i * 4));
        }
    }
    let opt_handle = rd(m, req + 36);
    if opt_handle != 0 {
        fn_195ae8(m, worker + W_CALLBACK, opt_handle, 8);
        wr(m, worker + W_CALLBACK_FLAGS, rd(m, req + 40));
    }
    wr(m, worker + W_USER, rd(m, req + 12));
    let mut status;
    status = fn_196318(m, h, worker + W_ALLOCATOR, rd(m, worker + W_USER), scratch + 8);
    if status >= 0x80000000 {
        return cleanup_190984(m, h, req, out, worker, scratch, status);
    }
    {
        let mut hh = HeapCalls { inner: &mut *h };
        status = crate::boot_part_b::fn_176dac(
            m,
            [
                rd(m, scratch + 8),
                rd(m, worker + W_USER),
                worker + W_RESOURCE,
                rd(m, req + 20),
                scratch + 12,
            ],
            &mut hh,
        );
    }
    if status >= 0x80000000 {
        return cleanup_190984(m, h, req, out, worker, scratch, status);
    }
    {
        let mut hh = HeapCalls { inner: &mut *h };
        status = crate::boot_part_b::fn_172e20(
            m,
            [
                rd(m, scratch + 8),
                rd(m, worker + W_USER),
                worker + W_PROVIDER,
                rd(m, req + 32),
                scratch + 16,
            ],
            &mut hh,
        );
    }
    if status >= 0x80000000 {
        return cleanup_190984(m, h, req, out, worker, scratch, status);
    }
    {
        let engine_id = rd(m, scratch + 8);
        let resource_id = rd(m, scratch + 12);
        let mut rh = RouteHost { inner: &mut *h };
        status = m.fn_1961d0(
            &mut rh,
            [engine_id, resource_id, 0, 0, worker + W_ENGINE],
        );
    }
    if status >= 0x80000000 {
        return cleanup_190984(m, h, req, out, worker, scratch, status);
    }
    status = fn_19611c(
        m,
        rd(m, worker + W_ENGINE),
        rd(m, worker + W_ENGINE + 4),
        worker + W_CONTEXT,
    );
    let context = rd(m, worker + W_CONTEXT);
    wr(m, context + 0x1C, rd(m, scratch + 16));
    if status >= 0x80000000 {
        return cleanup_190984(m, h, req, out, worker, scratch, status);
    }
    if rd(m, req + 4) == 0 {
        return cleanup_190984(m, h, req, out, worker, scratch, 0x80000012);
    }
    wh(m, scratch + 0x20, 3);
    wr(m, scratch + 0x34, rd(m, req + 4));
    status = h.call(
        m,
        0x177624,
        &[
            rd(m, worker + W_ENGINE),
            rd(m, worker + W_ENGINE + 4),
            scratch + 0x20,
            0,
            worker + W_STREAM,
        ],
    );
    if status >= 0x80000000 {
        return cleanup_190984(m, h, req, out, worker, scratch, status);
    }
    status = fn_177b7c(m, h, 0, rd(m, worker + W_STREAM), context + 12, scratch + 4);
    if status >= 0x80000000 {
        return cleanup_190984(m, h, req, out, worker, scratch, status);
    }
    status = fn_17ae04(
        m,
        h,
        0,
        rd(m, worker + W_ENGINE),
        rd(m, worker + W_ENGINE + 4),
        context + 0x18,
        scratch + 4,
    );
    if status >= 0x80000000 {
        return cleanup_190984(m, h, req, out, worker, scratch, status);
    }
    status = h.call(
        m,
        0x174CA8,
        &[
            0,
            rd(m, worker + W_ENGINE),
            rd(m, worker + W_ENGINE + 4),
            context + 0x14,
        ],
    );
    if status >= 0x80000000 {
        return cleanup_190984(m, h, req, out, worker, scratch, status);
    }
    status = h.call(
        m,
        0x14D764,
        &[
            rd(m, worker + W_ENGINE),
            rd(m, worker + W_ENGINE + 4),
            0,
            worker + W_LOGGER,
        ],
    );
    if status >= 0x80000000 {
        return cleanup_190984(m, h, req, out, worker, scratch, status);
    }
    {
        let mut rh = RouteHost { inner: &mut *h };
        status = m.fn_14dc94(&mut rh, rd(m, worker + W_LOGGER), context + 16);
    }
    if status >= 0x80000000 {
        return cleanup_190984(m, h, req, out, worker, scratch, status);
    }

    fn_195bec(m, scratch + 0x18, 0, 8);
    wr(m, scratch + 0x18, worker + W_CALLBACK);
    wr(m, scratch + 0x1C, rd(m, 0x190EA0));
    if rd(m, worker + W_CALLBACK) != 0 || rd(m, worker + W_CALLBACK + 4) != 0 {
        status = fn_14de40(m, h, rd(m, context + 16), worker + W_CALLBACK, rd(m, 0x190EA0));
        if status >= 0x80000000 {
            return cleanup_190984(m, h, req, out, worker, scratch, status);
        }
    }
    status = fn_174988(m, 1, scratch);
    if status < 0x80000000 {

        let mut hh = HeapCalls { inner: &mut *h };
        let _ = crate::boot_part_b::fn_196448(
            m,
            rd(m, scratch + 8),
            rd(m, scratch),
            rd(m, context + 16),
            &mut hh,
        );
    }
    status = fn_160aa8(m, 1, worker + W_VOICE_API);
    if status >= 0x80000000 {
        return cleanup_190984(m, h, req, out, worker, scratch, status);
    }
    wr(m, scratch + 0x18, rd(m, 0x176FE8));
    wr(m, scratch + 0x1C, rd(m, 0x176FEC));
    status = h.call(
        m,
        rd(m, rd(m, worker + W_VOICE_API) + 16),
        &[
            rd(m, scratch + 0x18),
            rd(m, scratch + 0x1C),
            rd(m, worker + W_ENGINE),
            rd(m, worker + W_ENGINE + 4),
            worker + W_VOICE,
        ],
    );
    if status >= 0x80000000 {
        return cleanup_190984(m, h, req, out, worker, scratch, status);
    }
    status = fn_157a54(m, 3, worker + W_CALLBACK_API);
    if status >= 0x80000000 {
        return cleanup_190984(m, h, req, out, worker, scratch, status);
    }
    status = h.call(
        m,
        rd(m, rd(m, worker + W_CALLBACK_API) + 8),
        &[
            rd(m, worker + W_ENGINE),
            rd(m, worker + W_ENGINE + 4),
            worker + W_CALLBACK_HANDLE,
        ],
    );
    if status >= 0x80000000 {
        return cleanup_190984(m, h, req, out, worker, scratch, status);
    }
    status = fn_15174c(m, 1, worker + W_SYNTHSTREAM);
    if status >= 0x80000000 {
        return cleanup_190984(m, h, req, out, worker, scratch, status);
    }
    status = fn_15db5c(m, 1, worker + W_LINGDB);
    if status < 0x80000000 {
        wr(m, out, worker);
        wr(m, out + 4, 0x68434C53);
    }
    fn_160fec(status)
}

fn cleanup_190984(
    m: &mut Memory<'_>,
    h: &mut impl Calls,
    req: u32,
    _out: u32,
    worker: u32,
    scratch: u32,
    status: u32,
) -> u32 {
    if rd(m, worker + W_CONTEXT) != 0 {
        h.call(m, 0x1906CC, &[worker, 0]);
        return fn_160fec(status);
    }
    let provider_id = rd(m, scratch + 16);
    if provider_id != 0 {
        h.call(m, 0x172E84, &[provider_id]);
    }
    if rd(m, scratch + 12) != 0 {
        h.call(m, 0x176E10, &[scratch + 12]);
    }
    let engine_id = rd(m, scratch + 8);
    if engine_id != 0 {
        fn_196368(m, h, engine_id);
    }
    if worker != 0 {
        let iface = rd(m, req + 8);
        h.call(m, rd(m, iface + 12), &[rd(m, req + 12), worker]);
    }
    fn_160fec(status)
}

pub fn fn_177624(
    m: &mut Memory<'_>,
    h: &mut impl Calls,
    hp0: u32,
    hp1: u32,
    pattern: u32,
    _zero: u32,
    out: u32,
    scratch: u32,
) -> u32 {
    let t = rh(m, pattern) as u32;
    if t != 2 && t != 3 {
        epi_log_177624(m, h, 0, scratch);
        return hp1;
    }
    wr(m, scratch, 0);
    let rc = fn_19611c(m, hp0, hp1, scratch);
    if rc >= 0x80000000 {
        epi_log_177624(m, h, 0, scratch);
        return hp1;
    }
    let ctx = rd(m, scratch);
    {
        let mut rhh = RouteHost { inner: &mut *h };
        crate::log_live::fn_1738e8(
            m,
            rd(m, ctx.wrapping_add(0x10)),
            rd(m, 0x177298),
            4,
            0,
            0x177A04,
            scratch.wrapping_add(4),
            &mut rhh,
        );
    }
    let heap = rd(m, ctx.wrapping_add(4));
    let sl = fn_1963b8(m, &mut *h, heap, 1, 0x20);
    if sl == 0 {
        epi_log_177624(m, h, ctx, scratch);
        return rd(m, 0x177260);
    }
    wr(m, sl, hp0);
    wr(m, sl.wrapping_add(4), hp1);
    wr(m, sl.wrapping_add(8), ctx);
    let sb = heap;
    let r4 = fn_1963b8(m, &mut *h, sb, 1, 0x18);
    if r4 == 0 {
        return teardown_177624(m, h, ctx, sl, rd(m, 0x177260), scratch);
    }
    wh(m, r4, rh(m, pattern));
    wr(m, r4.wrapping_add(4), 0);

    let s7 = rd(m, pattern.wrapping_add(8));
    if s7 == 0 {
        wr(m, r4.wrapping_add(8), 0);
    } else {
        let n = crate::boot_port_1::fn_195d90(m, s7).wrapping_add(1);
        let cp = fn_1963b8(m, &mut *h, sb, 1, n);
        if cp != 0 {
            fn_195ae8(m, cp, s7, n);
        }
        wr(m, r4.wrapping_add(8), cp);
        if cp == 0 {
            h.call(m, 0x17AD10, &[sb, r4]);
            return teardown_177624(m, h, ctx, sl, rd(m, 0x177260), scratch);
        }
    }
    wh(m, r4.wrapping_add(0xC), 0);
    let cnt = rh(m, pattern.wrapping_add(0xC)) as u32;
    if cnt == 0 {
        wr(m, r4.wrapping_add(0x10), 0);
    } else {
        let arr = fn_1963b8(m, &mut *h, sb, 1, 4);
        if arr == 0 {
            h.call(m, 0x17AD10, &[sb, r4]);
            return teardown_177624(m, h, ctx, sl, rd(m, 0x177260), scratch);
        }
        wr(m, r4.wrapping_add(0x10), arr);
        let mut idx = 0u32;
        while rh(m, pattern.wrapping_add(0xC)) as u32 > idx {
            let s = rd(m, rd(m, pattern.wrapping_add(0x10)).wrapping_add(idx.wrapping_mul(4)));
            let mut r7 = 0u32;
            if s != 0 {
                let n = crate::boot_port_1::fn_195d90(m, s).wrapping_add(1);
                r7 = fn_1963b8(m, &mut *h, sb, 1, n);
                if r7 != 0 {
                    fn_195ae8(m, r7, s, n);
                }
            }
            wr(m, arr.wrapping_add(idx.wrapping_mul(4)), r7);
            if r7 == 0 {

                let done_n = rh(m, r4.wrapping_add(0xC)) as u32;
                for i in 0..done_n {
                    h.call(m, 0x196428, &[sb, rd(m, arr.wrapping_add(i.wrapping_mul(4)))]);
                }
                h.call(m, 0x196428, &[sb, arr]);
                wr(m, r4.wrapping_add(0x10), 0);
                h.call(m, 0x196428, &[sb, rd(m, r4.wrapping_add(8))]);
                h.call(m, 0x196428, &[sb, r4]);
                return teardown_177624(m, h, ctx, sl, rd(m, 0x177260), scratch);
            }
            wh(m, r4.wrapping_add(0xC), rh(m, r4.wrapping_add(0xC)).wrapping_add(1));
            idx += 1;
        }
    }

    let s14 = rd(m, pattern.wrapping_add(0x14));
    if s14 == 0 {
        wr(m, r4.wrapping_add(0x14), 0);
    } else {
        let n = crate::boot_port_1::fn_195d90(m, s14).wrapping_add(1);
        let cp = fn_1963b8(m, &mut *h, sb, 1, n);
        if cp != 0 {
            fn_195ae8(m, cp, s14, n);
        }
        wr(m, r4.wrapping_add(0x14), cp);
        if cp == 0 {
            h.call(m, 0x17AD10, &[sb, r4]);
            return teardown_177624(m, h, ctx, sl, rd(m, 0x177260), scratch);
        }
    }
    wr(m, sl.wrapping_add(0x18), r4);

    let r = fn_176e40(
        m,
        &mut *h,
        rd(m, ctx.wrapping_add(8)),
        rd(m, ctx.wrapping_add(4)),
        sl.wrapping_add(0xC),
    );
    if r >= 0x80000000 {
        return teardown_177624(m, h, ctx, sl, r, scratch);
    }
    let cfg = rd(m, 0x177298);

    let r = fn_1964e0(
        m,
        &mut *h,
        heap,
        0,
        rd(m, cfg.wrapping_add(0x44)),
        rd(m, cfg.wrapping_add(0x48)),
        rd(m, cfg.wrapping_add(0x4C)),
        rd(m, cfg.wrapping_add(0x50)),
        rd(m, cfg.wrapping_add(0x54)),
        sl.wrapping_add(0x10),
    );
    if r >= 0x80000000 {
        return teardown_177624(m, h, ctx, sl, r, scratch);
    }
    let r = fn_1964e0(
        m,
        &mut *h,
        heap,
        0,
        rd(m, cfg.wrapping_add(0x58)),
        rd(m, cfg.wrapping_add(0x5C)),
        rd(m, cfg.wrapping_add(0x60)),
        rd(m, cfg.wrapping_add(0x64)),
        rd(m, cfg.wrapping_add(0x68)),
        sl.wrapping_add(0x14),
    );
    if r >= 0x80000000 {
        return teardown_177624(m, h, ctx, sl, r, scratch);
    }
    let r = fn_176e40(
        m,
        &mut *h,
        rd(m, ctx.wrapping_add(8)),
        rd(m, ctx.wrapping_add(4)),
        sl.wrapping_add(0x1C),
    );
    if r >= 0x80000000 {
        return teardown_177624(m, h, ctx, sl, r, scratch);
    }

    let r = if t == 2 {
        h.call(m, 0x17E06C, &[sl, pattern])
    } else {
        h.call(m, 0x17D168, &[ctx, sl, rd(m, pattern.wrapping_add(0x14))])
    };
    if r < 0x80000000 {
        wr(m, out, sl);
        epi_log_177624(m, h, ctx, scratch);
        return r;
    }
    teardown_177624(m, h, ctx, sl, r, scratch)
}

fn epi_log_177624(m: &mut Memory<'_>, h: &mut impl Calls, ctx: u32, scratch: u32) {
    let logger = if ctx == 0 { 0 } else { rd(m, ctx.wrapping_add(0x10)) };
    let mut rhh = RouteHost { inner: &mut *h };
    let _ = crate::log_live::fn_1738e8(m, logger, 0, 4, 0, 0, scratch.wrapping_add(4), &mut rhh);
}

pub fn fn_1964e0(
    m: &mut Memory<'_>,
    h: &mut impl Calls,
    heap: u32,
    fp: u32,
    a2: u32,
    a3: u32,
    a4: u32,
    a5: u32,
    a6: u32,
    out: u32,
) -> u32 {

    if a2 == 0 || a3 == 0 || a4 == 0 || a5 == 0 || a6 == 0 || out == 0 {
        return rd(m, 0x196A54);
    }
    let obj = fn_1963b8(m, &mut *h, heap, 1, 0x44);
    wr(m, out, obj);
    if obj == 0 {
        return rd(m, 0x196A50);
    }
    wr(m, obj, a2);
    wr(m, obj.wrapping_add(4), a3);
    wr(m, obj.wrapping_add(8), a4);
    wr(m, obj.wrapping_add(12), a5);
    wr(m, obj.wrapping_add(16), a6);
    wr(m, obj.wrapping_add(0x14), heap);
    h.call(m, 0x198728, &[obj.wrapping_add(0x18), 0xFFFF_FFFF, obj]);
    if fp == 1 {
        h.call(m, 0x198FA8, &[obj.wrapping_add(0x18)]);
    }
    0
}

pub fn fn_19126c_full(
    m: &mut Memory<'_>,
    h: &mut impl Calls,
    owner: u32,
    tag: u32,
    user: u32,
    argument: u32,
    out: u32,
    flags: u32,
    scratch: u32,
) -> u32 {
    let want_tag = rd(m, 0x190E8C);
    if owner == 0 || tag != want_tag {
        return 0x80000008;
    }
    if rd(m, owner) != want_tag.wrapping_add(0x0B000000) {
        return 0x80000008;
    }

    {
        let mgr = rd(m, owner.wrapping_add(176));
        let mut rhh = RouteHost { inner: &mut *h };
        crate::log_live::fn_1738e8(
            m,
            rd(m, mgr.wrapping_add(0x10)),
            rd(m, 0x190E7C),
            4,
            0,
            0x1915A8,
            scratch.wrapping_add(384),
            &mut rhh,
        );
    }

    let state = v_alloc(m, &mut *h, rd(m, owner.wrapping_add(8)), user, 1, 0x1AC);
    if state == 0 {
        return 0x8000000A;
    }
    wr(m, state.wrapping_add(0x14), user);
    fn_195ae8(m, state.wrapping_add(0x18), owner.wrapping_add(0x9C), 8);
    wr(m, state.wrapping_add(0x20), argument);
    let wc = rd(m, owner.wrapping_add(220));
    wr(m, owner.wrapping_add(220), wc.wrapping_add(1));

    let tcell = scratch;
    wr(m, tcell, 0);
    let r = fn_196318(m, &mut *h, owner.wrapping_add(4), user, tcell);
    if r >= 0x80000000 {
        return cleanup_19126c(m, h, owner, state, user, r, scratch);
    }
    wr(m, state.wrapping_add(4), owner);
    wr(m, state.wrapping_add(0x184), flags);
    wr(m, state.wrapping_add(0x188), 0);
    wr(m, state.wrapping_add(0x18C), 0);
    let fp = rd(m, 0x191224);
    wr(m, state, fp.wrapping_add(0x0B000000));
    wr(m, state.wrapping_add(0x1A8), 0);
    wb(m, state.wrapping_add(0x34), 0xC);
    wb(m, state.wrapping_add(0x35), 1);

    let fcell = scratch.wrapping_add(4);
    wr(m, fcell, 0);
    let r = fn_177b7c(m, &mut *h, 0x1915B8, rd(m, owner.wrapping_add(184)), fcell, scratch.wrapping_add(8));
    if r >= 0x80000000 {
        fail_log_19126c(m, h, state, r, 0x1915E0);
        return cleanup_19126c(m, h, owner, state, user, r, scratch);
    }
    let fact = rd(m, fcell);

    let mgr = rd(m, owner.wrapping_add(176));
    let task = rd(m, tcell);
    let r = fn_176e40(m, &mut *h, rd(m, mgr.wrapping_add(8)), task, state.wrapping_add(0x24));
    if r >= 0x80000000 {
        fail_log_19126c(m, h, state, r, 0x1915EC);
        h.call(m, 0x177CC8, &[fact]);
        return cleanup_19126c(m, h, owner, state, user, r, scratch);
    }

    let r = {
        let mut rhh = RouteHost { inner: &mut *h };
        m.fn_1961d0(&mut rhh, [task, rd(m, mgr.wrapping_add(8)), fact, 0, state.wrapping_add(8)])
    };
    if r >= 0x80000000 {
        fail_log_19126c(m, h, state, r, 0x1915F0);
        h.call(m, 0x177CC8, &[fact]);
        h.call(m, 0x176EDC, &[rd(m, state.wrapping_add(0x24))]);
        return cleanup_19126c(m, h, owner, state, user, r, scratch);
    }

    let r = fn_19611c(m, rd(m, state.wrapping_add(8)), rd(m, state.wrapping_add(12)), state.wrapping_add(0x10));
    if r >= 0x80000000 {
        fail_log_19126c(m, h, state, r, 0x1915F0);
        h.call(m, 0x177CC8, &[fact]);
        h.call(m, 0x176EDC, &[rd(m, state.wrapping_add(0x24))]);
        return cleanup_19126c(m, h, owner, state, user, r, scratch);
    }
    let ctx = rd(m, state.wrapping_add(0x10));
    wr(m, ctx.wrapping_add(0x1C), rd(m, mgr.wrapping_add(0x1C)));
    wb(m, state.wrapping_add(0x28), 1);
    let logger = rd(m, owner.wrapping_add(180));
    if logger != 0 {
        let mut rhh = RouteHost { inner: &mut *h };
        m.fn_14dc94(&mut rhh, logger, ctx.wrapping_add(0x10));
    }

    if rd(m, state.wrapping_add(0x18)) != 0 && rd(m, state.wrapping_add(0x1C)) != 0 {
        let r = fn_14de40(m, &mut *h, rd(m, ctx.wrapping_add(0x10)), state.wrapping_add(0x18), rd(m, 0x190EA0));
        let _ = r;
    }

    let r = h.call(
        m,
        0x174CA8,
        &[0, rd(m, state.wrapping_add(8)), rd(m, state.wrapping_add(12)), ctx.wrapping_add(0x14)],
    );
    if r >= 0x80000000 {
        fail_log_19126c(m, h, state, r, 0x1915F4);
        return cleanup_19126c(m, h, owner, state, user, r, scratch);
    }

    let r = fn_17ae04(m, &mut *h, 0, rd(m, state.wrapping_add(8)), rd(m, state.wrapping_add(12)), ctx.wrapping_add(0x18), scratch.wrapping_add(52));
    if r >= 0x80000000 {
        fail_log_19126c(m, h, state, r, 0x1915F8);
        return cleanup_19126c(m, h, owner, state, user, r, scratch);
    }

    let tab = rd(m, ctx.wrapping_add(0x14));
    let r = {
        let mut hh = HeapCalls { inner: &mut *h };
        crate::boot_port_1::fn_1758ac(m, tab, 0x1918D8, 1, scratch.wrapping_add(284), &mut hh)
    };
    if r >= 0x80000000 {
        fail_log_19126c(m, h, state, r, 0x191910);
        return cleanup_19126c(m, h, owner, state, user, r, scratch);
    }
    let r = {
        let mut hh = HeapCalls { inner: &mut *h };
        crate::boot_port_1::fn_1758ac(m, tab, 0x1918E0, 250, scratch.wrapping_add(284), &mut hh)
    };
    if r >= 0x80000000 {
        fail_log_19126c(m, h, state, r, 0x191910);
        return cleanup_19126c(m, h, owner, state, user, r, scratch);
    }
    let r = fn_1759a0(m, &mut *h, tab, rd(m, 0x1918F8), rd(m, 0x1918F4), scratch.wrapping_add(84));
    if r >= 0x80000000 {
        fail_log_19126c(m, h, state, r, 0x191910);
        return cleanup_19126c(m, h, owner, state, user, r, scratch);
    }
    let r = fn_1757b8(m, &mut *h, tab, 0x1918FC, 0, scratch.wrapping_add(164));
    if r >= 0x80000000 {
        fail_log_19126c(m, h, state, r, 0x191910);
        return cleanup_19126c(m, h, owner, state, user, r, scratch);
    }

    let st = scratch.wrapping_add(364);
    fn_195bec(m, st, 0, 0x14);
    wb(m, st, 1);
    wr(m, st.wrapping_add(4), rd(m, owner.wrapping_add(0xD4)));
    wr(m, st.wrapping_add(8), rd(m, 0x176FE4));
    wr(m, st.wrapping_add(12), rd(m, 0x176FE8));
    wr(m, st.wrapping_add(16), 0);
    let r = h.call(m, 0x17B0F8, &[rd(m, ctx.wrapping_add(0x18)), rd(m, 0x191914), st]);
    if r >= 0x80000000 {
        fail_log_19126c(m, h, state, r, 0x191918);
        return cleanup_19126c(m, h, owner, state, user, r, scratch);
    }
    wr(m, st.wrapping_add(4), rd(m, owner.wrapping_add(0xD8)));
    let r = h.call(m, 0x17B0F8, &[rd(m, ctx.wrapping_add(0x18)), 0x19191C, st]);
    if r >= 0x80000000 {
        fail_log_19126c(m, h, state, r, 0x19191C);
        return cleanup_19126c(m, h, owner, state, user, r, scratch);
    }

    let setup = rd(m, rd(m, owner.wrapping_add(188)).wrapping_add(0x10));
    let r = h.call(
        m,
        setup,
        &[
            rd(m, owner.wrapping_add(192)),
            rd(m, owner.wrapping_add(196)),
            rd(m, state.wrapping_add(8)),
            rd(m, state.wrapping_add(12)),
            state.wrapping_add(0x2C),
            state,
            0,
        ],
    );
    if r >= 0x80000000 {
        fail_log_19126c(m, h, state, r, 0x191930);
        return cleanup_19126c(m, h, owner, state, user, r, scratch);
    }

    let r = fn_160b0c(m, &mut *h, task, state.wrapping_add(0x1A8));
    if r >= 0x80000000 {
        fail_log_19126c(m, h, state, r, 0x19193C);
        return cleanup_19126c(m, h, owner, state, user, r, scratch);
    }
    wr(m, out, state);
    wr(m, out.wrapping_add(4), fp);

    {
        let ctx2 = rd(m, state.wrapping_add(0x10));
        let mut rhh = RouteHost { inner: &mut *h };
        crate::log_live::fn_1738e8(
            m,
            rd(m, ctx2.wrapping_add(0x10)),
            rd(m, 0x190E84),
            4,
            0,
            0,
            scratch.wrapping_add(384),
            &mut rhh,
        );
    }
    0
}

fn fail_log_19126c(m: &mut Memory<'_>, h: &mut impl Calls, state: u32, r5: u32, c_addr: u32) {
    let ctx = rd(m, state.wrapping_add(0x10));
    let logger = if ctx == 0 { 0 } else { rd(m, ctx.wrapping_add(0x10)) };
    if logger == 0 {
        return;
    }
    let mut rh = RouteHost { inner: &mut *h };
    crate::boot_part_b::fn_173c70(
        m,
        logger,
        rd(m, 0x190E84),
        rd(m, c_addr),
        0x1915D8,
        0x1915D0,
        r5,
        &mut rh,
    );
}

fn cleanup_19126c(
    m: &mut Memory<'_>,
    h: &mut impl Calls,
    owner: u32,
    state: u32,
    user: u32,
    err: u32,
    scratch: u32,
) -> u32 {
    let ctx = rd(m, state.wrapping_add(0x10));
    if ctx != 0 {
        h.call(m, 0x190EDC, &[state, rd(m, 0x191224)]);
    } else {
        h.call(m, rd(m, owner.wrapping_add(0x10)), &[user, state]);
        let wc = rd(m, owner.wrapping_add(220));
        wr(m, owner.wrapping_add(220), wc.wrapping_sub(1));
        let task = rd(m, scratch);
        if task != 0 {
            fn_196368(m, &mut *h, task);
        }
    }
    let mgr = rd(m, owner.wrapping_add(176));
    let logger = rd(m, mgr.wrapping_add(0x10));
    if logger != 0 {
        let mut rh = RouteHost { inner: &mut *h };
        crate::boot_part_b::fn_173c70(m, logger, rd(m, 0x190E84), rd(m, 0x191940), 0, 0, err, &mut rh);
    }
    fn_160fec(err)
}

pub fn fn_196a64(
    m: &mut Memory<'_>,
    h: &mut impl Calls,
    heap: u32,
    sb: u32,
    c: u32,
    d: u32,
    e: u32,
    out: u32,
) -> u32 {
    if (c == 0) != (d == 0) || out == 0 {
        return 0;
    }
    let obj = fn_1963b8(m, &mut *h, heap, 1, 0x20);
    wr(m, out, obj);
    if obj == 0 {
        return 0;
    }
    wr(m, obj, 0);
    wr(m, obj.wrapping_add(0x10), 0);
    wr(m, obj.wrapping_add(0x14), 0);
    wr(m, obj.wrapping_add(4), sb);
    wr(m, obj.wrapping_add(4), c);
    wr(m, obj.wrapping_add(8), d);
    wh(m, obj.wrapping_add(0x18), e as u16);
    wr(m, obj.wrapping_add(0x1C), heap);
    1
}

pub fn fn_174ca8(m: &mut Memory<'_>, h: &mut impl Calls, engine: u32, out: u32) -> u32 {
    if out == 0 {
        return rd(m, 0x17551C);
    }
    let heap = rd(m, engine.wrapping_add(4));
    let err_lit = rd(m, 0x175516);
    let obj = fn_1963b8(m, &mut *h, heap, 1, 0x14);
    wr(m, out, obj);
    if obj == 0 {
        return err_lit;
    }
    wr(m, obj, heap);
    let r = fn_176e40(m, &mut *h, rd(m, engine.wrapping_add(8)), heap, obj.wrapping_add(4));
    if r >= 0x80000000 {
        return teardown_174ca8(m, h, engine, out, obj, r);
    }
    let cfg = rd(m, 0x175514);
    let r = fn_196a64(
        m,
        &mut *h,
        heap,
        0,
        rd(m, cfg),
        rd(m, cfg.wrapping_add(4)),
        rd(m, cfg.wrapping_add(8)),
        rd(m, cfg.wrapping_add(12)),
    );
    if r == 0 {
        return teardown_174ca8(m, h, engine, out, obj, err_lit);
    }
    let r = fn_196a64(
        m,
        &mut *h,
        heap,
        rd(m, cfg.wrapping_add(0x14)),
        rd(m, cfg.wrapping_add(0x18)),
        rd(m, cfg.wrapping_add(0x1C)),
        2,
        obj.wrapping_add(0x10),
    );
    if r != 0 {
        return r;
    }
    teardown_174ca8(m, h, engine, out, obj, err_lit)
}

fn teardown_174ca8(
    m: &mut Memory<'_>,
    h: &mut impl Calls,
    engine: u32,
    out: u32,
    obj: u32,
    err: u32,
) -> u32 {
    if rd(m, out) == 0 {
        return err;
    }
    if rd(m, obj.wrapping_add(0x10)) != 0 {
        h.call(m, 0x196C7C, &[rd(m, obj.wrapping_add(0x10))]);
    }
    if rd(m, obj.wrapping_add(0xC)) != 0 {
        h.call(m, 0x196C7C, &[rd(m, obj.wrapping_add(0xC))]);
    }
    if rd(m, obj.wrapping_add(8)) != 0 {
        h.call(m, 0x19665C, &[rd(m, obj.wrapping_add(8))]);
    }
    if rd(m, obj.wrapping_add(4)) != 0 {
        h.call(m, 0x176EDC, &[rd(m, obj.wrapping_add(4))]);
    }
    h.call(m, 0x196428, &[rd(m, engine.wrapping_add(4)), obj]);
    wr(m, out, 0);
    err
}

fn teardown_177624(
    m: &mut Memory<'_>,
    h: &mut impl Calls,
    ctx: u32,
    sl: u32,
    err: u32,
    scratch: u32,
) -> u32 {
    let heap = rd(m, ctx.wrapping_add(4));
    if rd(m, sl.wrapping_add(0x14)) != 0 {
        h.call(m, 0x19665C, &[rd(m, sl.wrapping_add(0x14))]);
        wr(m, sl.wrapping_add(0x14), 0);
    }
    if rd(m, sl.wrapping_add(0x10)) != 0 {
        h.call(m, 0x19665C, &[rd(m, sl.wrapping_add(0x10))]);
        wr(m, sl.wrapping_add(0x10), 0);
    }
    if rd(m, sl.wrapping_add(0xC)) != 0 {
        h.call(m, 0x176EDC, &[rd(m, sl.wrapping_add(0xC))]);
        wr(m, sl.wrapping_add(0xC), 0);
    }
    if rd(m, sl.wrapping_add(0x1C)) != 0 {
        h.call(m, 0x176EDC, &[rd(m, sl.wrapping_add(0x1C))]);
        wr(m, sl.wrapping_add(0x1C), 0);
    }
    if rd(m, sl.wrapping_add(0x18)) != 0 {
        h.call(m, 0x17AD10, &[heap, rd(m, sl.wrapping_add(0x18))]);
        wr(m, sl.wrapping_add(0x18), 0);
    }
    h.call(m, 0x196428, &[heap, sl]);
    epi_log_177624(m, h, ctx, scratch);
    err
}

pub fn fn_17ae04(
    m: &mut Memory<'_>,
    h: &mut impl Calls,
    _a0: u32,
    hp0: u32,
    hp1: u32,
    out: u32,
    scratch: u32,
) -> u32 {
    wr(m, scratch, 0);
    let mut rc = fn_19611c(m, hp0, hp1, scratch);
    if rc >= 0x80000000 {
        return rc;
    }
    if out == 0 {
        return 0x8B602007;
    }
    let ctx = rd(m, scratch);
    let o = fn_1963b8(m, h, rd(m, ctx + 4), 1, 20);
    wr(m, out, o);
    if o == 0 {
        return 0x8B60200A;
    }
    wr(m, scratch + 4, hp0);
    wr(m, scratch + 8, hp1);
    fn_195ae8(m, o, scratch + 4, 8);
    wr(m, o + 8, ctx);
    rc = fn_176e40(m, h, rd(m, ctx + 8), rd(m, ctx + 4), o + 12);
    if rc >= 0x80000000 {
        h.call(m, 0x196428, &[rd(m, ctx + 4), o]);
        wr(m, out, 0);
        return rc;
    }
    let w0 = rd(m, 0x8D773C);
    let w1 = rd(m, 0x8D773C + 4);
    let w2 = rd(m, 0x8D773C + 8);
    let w3 = rd(m, 0x8D773C + 12);
    let w4 = rd(m, 0x8D773C + 16);
    rc = h.call(
        m,
        0x1964E0,
        &[rd(m, ctx + 4), 0, w0, w1, w2, w3, w4, o + 16],
    );
    if rc < 0x80000000 {
        return 0;
    }
    h.call(m, 0x176EDC, &[rd(m, o + 12)]);
    h.call(m, 0x196428, &[rd(m, ctx + 4), o]);
    wr(m, out, 0);
    0x8B60200A
}
