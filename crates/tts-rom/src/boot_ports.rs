#![allow(unused_assignments)]
use tts_core::boot::{Calls, Memory};
use tts_core::boot_live;

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
pub const LOADER_HEADER_REQ: u32 = 0x340;
pub const LOADER_COMPONENTS_REQ: u32 = 0x344;

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

pub fn fn_19af64(m: &Memory<'_>, src: u32) -> u32 {
    let w = rd(m, src);
    let b = rb(m, src + 3) as u32;
    let v = ((w & 0x00FF_FFFF) | (b << 24)) >> 8;
    if v != 0 {
        return v;
    }
    let w = rd(m, src + 4);
    let b = rb(m, src + 7) as u32;
    (w & 0x00FF_FFFF) | (b << 24)
}

pub fn fn_19ae08(m: &mut Memory<'_>, src: u32, mut dst: u32) {
    let w = rd(m, src);
    let b3 = rb(m, src + 3) as u32;
    let long_mode = (rb(m, src) & 0xF) != 0;
    let mut rem = ((w & 0x00FF_FFFF) | (b3 << 24)) >> 8;
    let mut ip = src + 4;
    if rem == 0 {
        let w = rd(m, ip);
        let b = rb(m, ip + 3) as u32;
        ip += 4;
        rem = (w & 0x00FF_FFFF) | (b << 24);
        if rem == 0 {
            return;
        }
    }
    loop {
        let mut tag = rb(m, ip);
        ip += 1;
        let mut n = 0u32;
        loop {
            if (tag & 0x80) == 0 {
                let v = rb(m, ip);
                ip += 1;
                rem -= 1;
                wb(m, dst, v);
                dst += 1;
            } else {
                let peek = rb(m, ip) as u32;
                let nib = peek >> 4;
                let len: u32;
                if !long_mode {
                    len = nib + 3;
                } else if nib == 1 {
                    let b1 = rb(m, ip + 1) as u32;
                    let b2 = rb(m, ip + 2) as u32;
                    ip += 2;
                    len = (((peek & 0xF) << 12) | (b1 << 4) | (b2 >> 4)).wrapping_add(0x100 + 0x11);
                } else if nib == 0 {
                    let b1 = rb(m, ip + 1) as u32;
                    ip += 1;
                    len = (((peek & 0xF) << 4) | (b1 >> 4)).wrapping_add(0x11);
                } else {
                    len = nib + 1;
                }
                let hi = rb(m, ip) as u32;
                ip += 1;
                let mut take = len;
                if take > rem {
                    take = rem;
                }
                let lo = rb(m, ip) as u32;
                ip += 1;
                let off = (lo | ((hi & 0xF) << 8)).wrapping_add(1);
                rem -= take;
                if take != 0 {
                    let base = dst;
                    for i in 0..take {
                        let v = rb(m, base.wrapping_sub(off).wrapping_add(i));
                        wb(m, base.wrapping_add(i), v);
                    }
                    dst = base.wrapping_add(take);
                }
            }
            if rem == 0 {
                return;
            }
            n += 1;
            tag = tag.wrapping_shl(1);
            if n >= 8 {
                break;
            }
        }
    }
}

pub fn fn_4e0060(m: &mut Memory<'_>, h: &mut impl Calls, loader: u32) -> u32 {
    wr(m, loader, LOADER_VTABLE);
    wr(m, loader + 4, 0);
    let q = fn_708534(m, loader + 8);

    let base = q.wrapping_sub(8);
    wr(m, loader + 0xB0, 0);
    wr(m, loader + 0xB4, 0);
    for off in [
        0x154u32, 0x158, 0x1F8, 0x1FC, 0x29C, 0x2A0, 0x340, 0x344, 0x348,
    ] {
        wr(m, base + off, 0);
    }

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

pub fn fn_4dfef8(m: &mut Memory<'_>, loader: u32) -> u32 {
    let manager = rd(m, MANAGER_GLOBAL);
    let table = rd(m, manager + 0x14);
    let idx = rd(m, loader + 0x340);
    let entry = table + idx.wrapping_mul(0xE8);
    let str_ptr = rd(m, entry + 0xC0);
    let len = rd(m, entry + 0xCC);
    if str_ptr != 0 && len != 0 {
        wb(m, str_ptr + len - 1, 0);
    }
    str_ptr
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

impl<H: Calls> tts_core::boot_part_b::Host for HeapCalls<'_, H> {
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

impl<H: Calls> tts_core::speech_part_1::Host for HeapCalls<'_, H> {
    fn call(&mut self, m: &mut Memory<'_>, f: u32, a: &[u32]) -> u32 {
        if let Some(r) = route_call(m, &mut *self.inner, f, a) {
            return r;
        }
        self.inner.call(m, f, a)
    }
    fn get(
        &mut self,
        m: &mut Memory<'_>,
        f: u32,
        obj: u32,
        offset: u32,
        count: u32,
    ) -> (u32, u32, u32) {
        let sp = fn_129778(m, self.inner, 8);
        wr(m, sp, count);
        wr(m, sp + 4, 0);
        let rc = route_or_host(m, self.inner, f, &[obj, offset, sp, sp + 4]);
        (rc, rd(m, sp), rd(m, sp + 4))
    }
}

impl<H: Calls> tts_core::speech_part_1::HeapHost for HeapCalls<'_, H> {
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
    tts_core::speech_part_1::fn_11db30(m, size, idx, 32, fill, table, &mut HeapCalls { inner: h })
}

pub fn fn_122058(m: &mut Memory<'_>, p: u32) {
    m.fn_165c84(p);
    wr(m, p + 4, 0);
    wr(m, p + 8, 0);
}

pub fn fn_129778(m: &mut Memory<'_>, h: &mut impl Calls, size: u32) -> u32 {
    let table = HEAP_TABLE_ROOT;
    let fill = rb(m, FILL_FLAG) as u32;
    tts_core::speech_part_1::fn_11db30(m, size, 0, 32, fill, table, &mut HeapCalls { inner: h })
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
        tts_core::boot_part_b::fn_17177c(m, &mut HeapCalls { inner: h });
    }
    wr(m, o + 0x54, 0);
}

struct LockHost<'a, H: Calls> {
    inner: &'a mut H,
}

impl<H: Calls> tts_core::boot_part_b::Host for LockHost<'_, H> {
    fn call(&mut self, m: &mut Memory<'_>, f: u32, a: &[u32]) -> u32 {
        match f {
            0x183edc => {
                tts_core::boot_part_b2::fn_183edc(m, a[0], ARBITER_ADDRESS, self);
                0
            }
            0x165d00 => {
                let g = tts_core::boot::Globals {
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
    tts_core::boot_part_b::fn_1840fc(m, obj + 0x58, &mut lh);
    let r = tts_core::speech_part_3::fn_7e715c(m, obj, arg);
    tts_core::boot_part_b::fn_18423c(m, obj + 0x58, &mut lh);
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
        let mgr = rd(m, cand + 0x10);
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

            let m1 = rd(m, cand + 0x10);
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

            if e == 0 {
                return cand;
            }
        }
    }
}

pub fn fn_122cd8(m: &mut Memory<'_>, o: u32, item: u32) {
    wr(m, o + 12, item);
    let t = rd(m, item + 0x10);
    let head = o.wrapping_add(4);
    if t == 0 {
        wr(m, o + 8, head);

        wr(m, o + 4, head);

        wr(m, item + 0x10, head);
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
    let p = tts_core::speech_part_1::fn_11db30(
        m,
        off,
        idx,
        32,
        fill,
        table,
        &mut HeapCalls { inner: &mut *h },
    );

    let voice = rd(m, table.wrapping_add(idx.wrapping_mul(4)));
    let c44 = fn_122c44(m, &mut *h, voice, p);

    if c44 != voice {
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

    let loader = if raw != 0 {
        fn_4e0060(m, &mut *h, raw)
    } else {
        0
    };
    wr(m, sub + 0x1BC, loader);

    fn_4dff90(m, &mut *h, loader);

    fn_4dfea8(m, loader);

    fn_4dffd4(m, &mut *h, loader);

    let scr1 = fn_12973c(m, &mut *h, 0xFA0, 1);
    wr(m, sub + 0x19C, scr1);
    let scr2 = fn_12973c(m, &mut *h, 0x3000, 1);
    wr(m, sub + 0x194, scr2);

    wh(m, engine + 0x12C, 0x520);

    let hdr = fn_4dfef8(m, loader);
    wr(m, engine + 0x130, hdr);
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
    let g = tts_core::boot::Globals {
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

pub fn v_alloc(m: &mut Memory<'_>, h: &mut impl Calls, method: u32, a: u32, b: u32, c: u32) -> u32 {
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
    route_or_host(m, h, method, args)
}

pub struct RouteHost<'a, H: Calls> {
    pub inner: &'a mut H,
}

impl<H: Calls> tts_core::boot::Calls for RouteHost<'_, H> {
    fn format_address_m(&mut self, m: &Memory<'_>, format: &[u8]) -> Option<u32> {
        self.inner
            .real_log_args()
            .then(|| rom_text(m, format.strip_suffix(&[0]).unwrap_or(format)))
    }
    fn real_log_args(&mut self) -> bool {
        self.inner.real_log_args()
    }
    fn log_variadic(
        &mut self,
        m: &mut Memory<'_>,
        entry: u32,
        a: [u32; 4],
        format: &[u8],
        varargs: &[u32],
    ) -> bool {
        if !self.inner.real_log_args() {
            return false;
        }
        let format = format.strip_suffix(&[0]).unwrap_or(format);
        let fmt = rom_text(m, format);
        let sp = fn_129778(m, self.inner, (varargs.len() as u32 * 4).max(16));
        for (i, v) in varargs.iter().enumerate() {
            wr(m, sp + i as u32 * 4, *v);
        }
        if entry == 0x1738E8 {
            route_or_host(m, self.inner, 0x1738E8, &[a[0], a[1], a[2], a[3], fmt, sp]);
        } else {
            assert_eq!(entry, 0x173dc0);
            route_or_host(m, self.inner, 0x173DC0, &[a[0], a[1], a[2], fmt, sp]);
        }
        true
    }
    fn call(&mut self, m: &mut Memory<'_>, f: u32, a: &[u32]) -> u32 {
        match f {
            0x4A7758 if a.len() >= 3 => {
                let mut rh = RouteHost {
                    inner: &mut *self.inner,
                };
                m.fn_4a7758(&mut rh, [a[0], a[1], a[2]])
            }
            0x184F20 if a.len() >= 6 => fn_184f20(m, a[0], a[1], a[2], a[3], a[4], a[5]),
            0x39F5E8 if a.len() >= 3 => {
                let mut rh = RouteHost {
                    inner: &mut *self.inner,
                };
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

impl<H: Calls> tts_core::boot_part_b::Host for RouteHost<'_, H> {
    fn real_log_args(&mut self) -> bool {
        self.inner.real_log_args()
    }
    fn format_address_m(&mut self, m: &Memory<'_>, format: &[u8]) -> Option<u32> {
        tts_core::boot::Calls::format_address_m(self, m, format)
    }
    fn log_variadic(
        &mut self,
        m: &mut Memory<'_>,
        entry: u32,
        a: [u32; 4],
        format: &[u8],
        varargs: &[u32],
    ) -> bool {
        tts_core::boot::Calls::log_variadic(self, m, entry, a, format, varargs)
    }
    fn call(&mut self, m: &mut Memory<'_>, f: u32, a: &[u32]) -> u32 {
        tts_core::boot::Calls::call(self, m, f, a)
    }
    fn thread_id(&mut self) -> u32 {
        0x1FF82000
    }
}

impl<H: Calls> tts_core::boot_part_b::FormatHost for RouteHost<'_, H> {}

impl<H: Calls> tts_core::boot_part_d::Host for RouteHost<'_, H> {
    fn call_m(&mut self, f: u32, a: &[u32], m: &mut Memory<'_>) -> u32 {
        route_or_host(m, &mut *self.inner, f, a)
    }
    fn params_call_m(
        &mut self,
        q: u32,
        params: &mut [u32; 4],
        p: u32,
        out: &mut u32,
        flags: u32,
        m: &mut Memory<'_>,
    ) -> u32 {
        cold_archive_attach(m, self.inner, q, params, p, out, flags)
    }
}

impl<H: Calls> tts_core::boot_part_f::Host for RouteHost<'_, H> {
    fn call(&mut self, m: &mut Memory<'_>, method: u32, args: &[u32]) -> u32 {
        tts_core::boot::Calls::call(self, m, method, args)
    }
    fn compare(
        &mut self,
        m: &mut Memory<'_>,
        _method: u32,
        key: &mut tts_core::boot_part_f::Key,
        value: &mut u32,
    ) -> u32 {
        tts_core::boot_part_f::compare_parameter(m, *key, *value)
    }
}

impl<H: Calls> tts_core::log_live::LogMethodsM for RouteHost<'_, H> {
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

pub const SLOT0_OBJ: u32 = 0x40000;
pub const SLOT2_OBJ: u32 = 0x40100;
pub const SLOT0_BLOCK: u32 = 0x12000000;
pub const SLOT0_SPAN: u32 = 0x2000000;
pub const SLOT2_BLOCK_OFF: u32 = 0x50000;
pub const SLOT2_SPAN: u32 = 0x100000;
pub const SLOT_VTABLE: u32 = 0xA478AC;
pub const SLOT_MAGIC: u32 = 0x45585048;
pub const SLOT_M24: u32 = 0x84C160;
pub fn bootstrap_heap0(m: &mut Memory<'_>, engine: u32) {
    fn free_block(m: &mut Memory<'_>, at: u32, payload: u32) {
        wh(m, at, 0x4652);
        wh(m, at.wrapping_add(2), 0);
        wr(m, at.wrapping_add(4), payload);
        wr(m, at.wrapping_add(8), 0);
        wr(m, at.wrapping_add(12), 0);
    }
    fn slot_obj(m: &mut Memory<'_>, obj: u32, block: u32, lo: u32, hi: u32) {
        wr(m, obj, SLOT_VTABLE);
        wr(m, obj.wrapping_add(0x24), SLOT_MAGIC);
        wr(m, obj.wrapping_add(0x30), lo);
        wr(m, obj.wrapping_add(0x34), hi);
        wr(m, obj.wrapping_add(0x38), 0);
        wr(m, obj.wrapping_add(0x3C), block);
        wr(m, obj.wrapping_add(0x40), block);
        wr(m, obj.wrapping_add(0x44), 0);
        wr(m, obj.wrapping_add(0x48), 0);
        wh(m, obj.wrapping_add(0x4C), 0);
        wh(m, obj.wrapping_add(0x4E), 0);
        wr(m, obj.wrapping_add(0x54), 0);
        wr(m, obj.wrapping_add(0x58), 1);
        wr(m, obj.wrapping_add(88), 1);
    }
    let s0 = engine.wrapping_add(SLOT0_OBJ);
    let s2 = engine.wrapping_add(SLOT2_OBJ);
    wr(m, HEAP_TABLE_ROOT, s0);
    wr(m, HEAP_TABLE_ROOT.wrapping_add(4), SLOT1_HEAP);
    wr(m, HEAP_TABLE_ROOT.wrapping_add(8), s2);
    wr(m, SLOT_VTABLE.wrapping_add(0x18), SLOT_M24);
    slot_obj(
        m,
        s0,
        SLOT0_BLOCK,
        SLOT0_BLOCK,
        SLOT0_BLOCK.wrapping_add(SLOT0_SPAN),
    );
    slot_obj(
        m,
        SLOT1_HEAP,
        SLOT1_FRESH,
        SLOT1_FRESH,
        SLOT1_FRESH.wrapping_add(SLOT1_SPAN),
    );
    let s2b = engine.wrapping_add(SLOT2_BLOCK_OFF);
    slot_obj(m, s2, s2b, s2b, s2b.wrapping_add(SLOT2_SPAN));
    free_block(m, SLOT0_BLOCK, SLOT0_SPAN.wrapping_sub(16));
    free_block(
        m,
        engine.wrapping_add(SLOT2_BLOCK_OFF),
        SLOT2_SPAN.wrapping_sub(16),
    );
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

pub fn bootstrap_archive_alloc(m: &mut Memory<'_>, _h: &mut impl Calls) {
    let state = rd(m, 0x10F144);
    let root = rd(m, 0x10F140).wrapping_sub(0x54);
    let vt = rd(m, 0x10F14C);
    for i in 0..3 {
        let obj = state + 0x10 + i * 8;
        wr(m, obj, vt);
        wr(m, obj + 4, 0);
        wr(m, root + 12 + i * 4, obj);
    }
    wr(m, rd(m, root + 12) + 4, rd(m, root));
    wr(m, rd(m, root + 16) + 4, rd(m, root + 4));
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

pub fn route_or_host(m: &mut Memory<'_>, h: &mut impl Calls, f: u32, a: &[u32]) -> u32 {
    if let Some(r) = route_call(m, &mut *h, f, a) {
        return r;
    }
    h.call(m, f, a)
}

struct Prealloc {
    object: u32,
    size: u32,
    result: u32,
}
impl tts_core::frontend::HeapVtable for Prealloc {
    fn alloc_at(&mut self, object: u32, size: u32) -> u32 {
        assert_eq!((object, size), (self.object, self.size));
        self.result
    }
    fn realloc_at(&mut self, _: u32, _: u32, _: u32) -> u32 {
        panic!("unexpected reallocate")
    }
    fn release_at(&mut self, _: u32, _: u32) {
        panic!("unexpected release")
    }
}

pub fn route_call(m: &mut Memory<'_>, h: &mut impl Calls, f: u32, a: &[u32]) -> Option<u32> {
    match f {
        0x4A7758 if a.len() >= 3 => {
            let mut rh = RouteHost { inner: h };
            Some(m.fn_4a7758(&mut rh, [a[0], a[1], a[2]]))
        }
        0x184F20 if a.len() >= 6 => Some(fn_184f20(m, a[0], a[1], a[2], a[3], a[4], a[5])),
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
        0x4A779C if a.len() >= 2 => {
            let mut rh = RouteHost { inner: &mut *h };
            Some(m.fn_4a779c(&mut rh, a[0], a[1]))
        }
        0x196394 => {
            if a.len() >= 2 {
                Some(fn_196394(m, &mut *h, a[0], a[1]))
            } else {
                None
            }
        }
        0x1963E0 => {
            if a.len() >= 3 {
                Some(fn_1963e0(m, &mut *h, a[0], a[1], a[2]))
            } else {
                None
            }
        }
        0x1994D0 => {
            if a.len() >= 2 {
                Some(fn_1994d0(a[0], a[1]))
            } else {
                None
            }
        }
        0x196CFC => {
            if a.len() >= 2 {
                Some(fn_196cfc(m, &mut *h, a[0], a[1]))
            } else {
                None
            }
        }
        0x195D30 => {
            if a.len() >= 2 {
                Some(fn_195d30(m, a[0], a[1]))
            } else {
                None
            }
        }
        0x197254 => {
            if a.len() >= 2 {
                Some(fn_197254(m, &mut *h, a[0], a[1]))
            } else {
                None
            }
        }
        0x176054 => {
            if a.len() >= 4 {
                let (slot, src, n, heap) = (a[0], a[1], a[2], a[3]);
                if slot == 0 {
                    Some(0x8B702007)
                } else if n != 0 && rd(m, slot.wrapping_add(4)) == 0 {
                    let allocated = match route_call(m, &mut *h, 0x196394, &[heap, n]) {
                        Some(r) => r,
                        None => h.call(m, 0x196394, &[heap, n]),
                    };
                    let object = if heap == 0 {
                        0
                    } else {
                        rd(m, heap.wrapping_add(4))
                    };
                    let mut vt = Prealloc {
                        object,
                        size: n,
                        result: allocated,
                    };
                    Some(tts_core::frontend::store_sized(
                        &mut *m.bytes,
                        Some(slot),
                        src,
                        n,
                        heap,
                        &mut vt,
                    ))
                } else if (n != 0 && rd(m, slot.wrapping_add(4)) < n)
                    || (n == 0 && rd(m, slot) != 0 && rd(m, slot.wrapping_add(4)) != 0)
                {
                    None
                } else {
                    let mut vt = Prealloc {
                        object: 0,
                        size: 0,
                        result: 0,
                    };
                    Some(tts_core::frontend::store_sized(
                        &mut *m.bytes,
                        Some(slot),
                        src,
                        n,
                        heap,
                        &mut vt,
                    ))
                }
            } else {
                None
            }
        }
        0x174A04 => {
            if a.len() >= 3 {
                Some(fn_174a04(m, &mut *h, a[0], a[1], a[2]))
            } else {
                None
            }
        }
        0x17AD98 => {
            if a.len() >= 3 {
                Some(fn_17ad98(m, &mut *h, a[0], a[1], a[2]))
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
        0x17B058 => {
            if a.len() >= 3 {
                Some(fn_17b058(m, &mut *h, a[0], a[1], a[2]))
            } else {
                None
            }
        }
        0x17B0F8 => {
            if a.len() >= 3 {
                Some(fn_17b0f8(m, &mut *h, a[0], a[1], a[2]))
            } else {
                None
            }
        }
        0x1966AC => {
            if a.len() >= 3 {
                Some(fn_1966ac(m, &mut *h, a[0], a[1], a[2]))
            } else {
                None
            }
        }
        0x17A1A4 if a.len() >= 5 => {
            let sp = fn_129778(m, h, 256);
            let e = tts_core::component_live_c::unmap_environment_us(sp + 16);
            let mut rh = RouteHost { inner: h };
            Some(tts_core::boot_port_1::fn_17a1a4(
                m, a[0], a[1], a[2], a[3], a[4], sp, &e, &mut rh,
            ))
        }
        0x179208 if a.len() >= 3 => {
            let sp = fn_129778(m, h, 256);
            let e = tts_core::component_live_c::unmap_environment_us(sp);
            let mut rh = RouteHost { inner: h };
            Some(tts_core::boot_part_e::fn_179208(
                m, a[0], a[1], a[2], &e, &mut rh,
            ))
        }
        0x1B254C if a.len() >= 3 => {
            let sp = fn_129778(m, h, 4);
            let mut hh = HeapCalls { inner: h };
            Some(tts_core::speech_part_1::fn_1b254c(
                m, a[0], a[1], a[2], sp, &mut hh,
            ))
        }
        0x1B2974 if a.len() >= 4 => {
            let mut hh = HeapCalls { inner: h };
            Some(tts_core::speech_part_1::fn_1b2974(
                m, a[0], a[1], a[2], a[3], &mut hh,
            ))
        }
        0x1B2760 if a.len() >= 2 => {
            let mut hh = HeapCalls { inner: h };
            Some(tts_core::speech_part_1::fn_1b2760(m, a[0], a[1], &mut hh))
        }
        0x1B4528 if !a.is_empty() => {
            tts_core::boot_port_1::fn_1b4528(m, a[0], 0, 0);
            Some(0)
        }
        0x1B2CDC if !a.is_empty() => {
            let mut rh = RouteHost { inner: h };
            Some(tts_core::boot_port_1::fn_1b2cdc(m, a[0], &mut rh))
        }
        0x1BA82C if !a.is_empty() => {
            let mut rh = RouteHost { inner: h };
            Some(tts_core::boot_port_1::fn_1ba82c(m, a[0], &mut rh))
        }
        0x1B350C if !a.is_empty() => {
            let mut rh = RouteHost { inner: h };
            Some(tts_core::boot_port_1::fn_1b350c(m, a[0], &mut rh))
        }
        0x172D30 if a.len() >= 4 => {
            let mut hh = HeapCalls { inner: h };
            Some(tts_core::speech_part_1::fn_172d30(
                m, a[0], a[1], a[2], a[3], &mut hh,
            ))
        }
        0x172DE0 if a.len() >= 3 => {
            let mut hh = HeapCalls { inner: h };
            Some(tts_core::speech_part_1::fn_172de0(
                m, a[0], a[1], a[2], &mut hh,
            ))
        }
        0x1735E0 if a.len() >= 2 => {
            let p = rd(m, 0x173634);
            let params = std::array::from_fn(|i| rd(m, p + i as u32 * 4));
            let mut rh = RouteHost { inner: h };
            Some(tts_core::boot_part_d::fn_1735e0(
                m, a[0], a[1], params, &mut rh,
            ))
        }
        0x4A79B0 if !a.is_empty() => Some(tts_core::boot_port_6::fn_4a79b0(m, a[0])),
        0x4A794C if a.len() >= 4 => Some(tts_core::speech_part_3::fn_4a794c(
            m, a[0], a[1], a[2], a[3],
        )),
        0x4A78F0 if a.len() >= 4 => Some(tts_core::speech_part_3::fn_4a78f0(
            m, a[0], a[1], a[2], a[3],
        )),
        0x4A77AC if a.len() >= 5 => Some(fn_4a77ac(m, h, a)),
        0x173000 if a.len() >= 4 => {
            let mut rh = RouteHost { inner: h };
            Some(tts_core::boot_part_d::fn_173000(
                m, a[0], a[1], a[2], a[3], 0x1730F8, &mut rh,
            ))
        }
        0x177044 if a.len() >= 2 => {
            let p = a[0];
            if p != 0 {
                if rd(m, p + 4) != 0 {
                    route_or_host(m, h, 0x19665C, &[rd(m, p + 4)]);
                }
                if rd(m, p + 8) != 0 {
                    route_or_host(m, h, 0x19665C, &[rd(m, p + 8)]);
                }
                fn_196428(m, h, a[1], p);
            }
            Some(0)
        }
        0x196964 if a.len() >= 2 => Some(fn_196428(m, h, a[1], a[0])),
        0x198FB4 if a.len() >= 3 => {
            let mut rh = RouteHost { inner: h };
            tts_core::engine_live::fn_198fb4_m(m, a[0], a[1], a[2], &mut rh);
            Some(0)
        }
        0x198FA8 => {
            if a.len() >= 1 {
                wb(m, a[0].wrapping_add(0x28), 1);
                Some(0)
            } else {
                None
            }
        }
        0x19668C => {
            if a.len() >= 3 {
                Some(fn_19668c(m, &mut *h, a[0], a[1], a[2]))
            } else {
                None
            }
        }
        0x14DC30 => {
            if a.len() >= 1 {
                Some(fn_14dc30(m, &mut *h, a[0]))
            } else {
                None
            }
        }
        0x17AF38 => {
            if a.len() >= 1 {
                Some(fn_17af38(m, &mut *h, a[0]))
            } else {
                None
            }
        }
        0x174E00 => {
            if a.len() >= 1 {
                Some(fn_174e00(m, &mut *h, a[0]))
            } else {
                None
            }
        }
        0x176E10 => {
            if a.len() >= 1 {
                Some(fn_176e10(m, &mut *h, a[0]))
            } else {
                None
            }
        }
        0x172E84 => {
            if a.len() >= 1 {
                Some(fn_172e84(m, &mut *h, a[0]))
            } else {
                None
            }
        }
        0x196C7C => {
            if a.len() >= 1 {
                Some(fn_196c7c(m, &mut *h, a[0]))
            } else {
                None
            }
        }
        0x177CC8 => {
            if a.len() >= 1 {
                Some(fn_177cc8(m, &mut *h, a[0]))
            } else {
                None
            }
        }
        0x177558 => {
            if a.len() >= 1 {
                Some(fn_177558(m, &mut *h, a[0]))
            } else {
                None
            }
        }
        0x196238 => {
            if a.len() >= 2 {
                Some(fn_196238(m, &mut *h, a[0], a[1]))
            } else {
                None
            }
        }
        0x196368 => {
            if a.len() >= 1 {
                Some(fn_196368(m, &mut *h, a[0]))
            } else {
                None
            }
        }
        0x199034 => {
            if a.len() >= 5 {
                Some(fn_199034(m, &mut *h, a[0], a[1], a[2], a[3], a[4]))
            } else {
                None
            }
        }
        0x1990EC => {
            if a.len() >= 1 {
                Some(fn_1990ec(m, &mut *h, a[0]))
            } else {
                None
            }
        }
        0x196DB0 => {
            if a.len() >= 2 {
                Some(fn_196db0(m, &mut *h, a[0], a[1]))
            } else {
                None
            }
        }
        0x196A64 => {
            if a.len() >= 6 {
                Some(fn_196a64(m, &mut *h, a[0], a[1], a[2], a[3], a[4], a[5]))
            } else {
                None
            }
        }
        0x1991D0 => {
            if a.len() >= 1 {
                Some(fn_1991d0(m, &mut *h, a[0]))
            } else {
                None
            }
        }
        0x199134 => {
            if a.len() >= 4 {
                Some(fn_199134(m, &mut *h, a[0], a[1], a[2], a[3]))
            } else {
                None
            }
        }
        0x1970AC => {
            if a.len() >= 3 {
                Some(fn_1970ac(m, a[0], a[1], a[2]))
            } else {
                None
            }
        }
        0x195D90 => {
            if a.len() >= 1 {
                Some(fn_195d90(m, a[0]))
            } else {
                None
            }
        }
        0x195CD0 => {
            if a.len() >= 2 {
                Some(fn_195cd0(m, a[0], a[1]))
            } else {
                None
            }
        }
        0x196448 => {
            if a.len() >= 3 {
                Some(fn_196448(m, &mut *h, a[0], a[1], a[2]))
            } else {
                None
            }
        }
        0x1D75AC if a.len() >= 2 => Some(tts_core::boot_port_2::fn_1d75ac(
            m,
            a[0],
            a[1],
            rd(m, 0x1D75D0),
        )),
        0x1D88C8 if a.len() >= 2 => Some(tts_core::boot_port_2::fn_1d88c8(
            m,
            a[0],
            a[1],
            rd(m, 0x1D88EC),
        )),
        0x1DE860 if a.len() >= 2 => Some(tts_core::boot_port_2::fn_1de860(
            m,
            a[0],
            a[1],
            rd(m, 0x1DE878),
        )),
        0x1E3ECC if a.len() >= 2 => Some(tts_core::boot_port_2::fn_1e3ecc(
            m,
            a[0],
            a[1],
            rd(m, 0x1E3EE4),
        )),
        0x1E5110 if a.len() >= 2 => Some(tts_core::boot_port_2::fn_1e5110(
            m,
            a[0],
            a[1],
            rd(m, 0x1E524C),
        )),
        0x1E60CC if a.len() >= 2 => Some(tts_core::boot_port_2::fn_1e60cc(
            m,
            a[0],
            a[1],
            rd(m, 0x1E60E8),
        )),
        0x1EA344 if a.len() >= 2 => Some(tts_core::boot_port_2::fn_1ea344(
            m,
            a[0],
            a[1],
            rd(m, 0x1EA4D8),
        )),
        0x1EE6D8 if a.len() >= 2 => Some(tts_core::boot_port_2::fn_1ee6d8(
            m,
            a[0],
            a[1],
            rd(m, 0x1EE7F0),
        )),
        0x20129C if a.len() >= 2 => Some(tts_core::boot_port_3::fn_20129c(
            m,
            a[0],
            a[1],
            rd(m, 0x2012B4),
        )),
        0x206788 if a.len() >= 2 => Some(tts_core::boot_port_3::fn_206788(
            m,
            a[0],
            a[1],
            rd(m, 0x2067A0),
        )),
        0x20714C if a.len() >= 2 => Some(tts_core::boot_port_3::fn_20714c(
            m,
            a[0],
            a[1],
            rd(m, 0x207164),
        )),
        0x1CF83C if a.len() >= 2 => Some(tts_core::boot_port_1::fn_1cf83c(
            m,
            a[0],
            a[1],
            rd(m, 0x1CF864),
        )),
        0x1D4990 if a.len() >= 2 => Some(tts_core::boot_port_1::fn_1d4990(
            m,
            a[0],
            a[1],
            rd(m, 0x1D49AC),
        )),
        0x1E33D0 if a.len() >= 2 => {
            if a[0] <= 2 {
                wr(m, a[1], rd(m, 0x1E3588));
                Some(0)
            } else {
                Some(rd(m, 0x1E2530))
            }
        }
        0x1E03C8 if a.len() >= 2 => {
            if a[0] <= 1 {
                wr(m, a[1], rd(m, 0x1E03E8));
                Some(0)
            } else {
                Some(rd(m, 0x1E03EC))
            }
        }
        0x1F11EC if a.len() >= 2 => {
            if a[0] <= 1 {
                wr(m, a[1], rd(m, 0x1F121C));
                Some(0)
            } else {
                Some(rd(m, 0x1F1220))
            }
        }
        0x1F2808 if a.len() >= 2 => {
            if a[0] <= 1 {
                wr(m, a[1], rd(m, 0x1F2828));
                Some(0)
            } else {
                Some(rd(m, 0x1F282C))
            }
        }
        0x1F3AA8 if a.len() >= 2 => {
            if a[0] <= 1 {
                wr(m, a[1], rd(m, 0x1F3AC0));
                Some(0)
            } else {
                Some(rd(m, 0x1F3AC4))
            }
        }
        0x1F446C if a.len() >= 2 => {
            if a[0] <= 1 {
                wr(m, a[1], rd(m, 0x1F4490));
                Some(0)
            } else {
                Some(rd(m, 0x1F4494))
            }
        }
        0x1F4D58 if a.len() >= 2 => {
            if a[0] <= 1 {
                wr(m, a[1], rd(m, 0x1F4D8C));
                Some(0)
            } else {
                Some(rd(m, 0x1F4D90))
            }
        }
        0x1F7EA8 if a.len() >= 2 => {
            if a[0] <= 1 {
                wr(m, a[1], rd(m, 0x1F7EC8));
                Some(0)
            } else {
                Some(rd(m, 0x1F7ECC))
            }
        }
        0x1FD6EC if a.len() >= 2 => {
            if a[0] <= 1 {
                wr(m, a[1], rd(m, 0x1FD84C));
                Some(0)
            } else {
                Some(rd(m, 0x1FD850))
            }
        }
        0x20F794 if a.len() >= 2 => {
            if a[0] <= 1 {
                wr(m, a[1], rd(m, 0x20FC88));
                Some(0)
            } else {
                Some(rd(m, 0x20FC8C))
            }
        }
        0x2113B0 if a.len() >= 2 => {
            if a[0] <= 1 {
                wr(m, a[1], rd(m, 0x2113CC));
                Some(0)
            } else {
                Some(rd(m, 0x2113D0))
            }
        }
        0x215B08 if a.len() >= 2 => {
            if a[0] <= 1 {
                wr(m, a[1], rd(m, 0x2162A4));
                Some(0)
            } else {
                Some(rd(m, 0x2162A8))
            }
        }
        0x21CDFC if a.len() >= 2 => {
            if a[0] <= 1 {
                wr(m, a[1], rd(m, 0x21CE24));
                Some(0)
            } else {
                Some(rd(m, 0x21CE28))
            }
        }
        0x21CE2C if a.len() >= 2 => {
            if a[0] <= 1 {
                wr(m, a[1], rd(m, 0x21CFD0));
                Some(0)
            } else {
                Some(rd(m, 0x21CFD4))
            }
        }
        0x2205DC if a.len() >= 2 => {
            if a[0] <= 1 {
                wr(m, a[1], rd(m, 0x220608));
                Some(0)
            } else {
                Some(rd(m, 0x22060C))
            }
        }
        0x2227DC if a.len() >= 2 => {
            if a[0] <= 1 {
                wr(m, a[1], rd(m, 0x222810));
                Some(0)
            } else {
                Some(rd(m, 0x222814))
            }
        }
        0x222F7C if a.len() >= 2 => {
            if a[0] <= 1 {
                wr(m, a[1], rd(m, 0x222FA8));
                Some(0)
            } else {
                Some(rd(m, 0x222FAC))
            }
        }
        0x224544 if a.len() >= 2 => {
            if a[0] <= 1 {
                wr(m, a[1], rd(m, 0x224564));
                Some(0)
            } else {
                Some(rd(m, 0x224568))
            }
        }
        0x225064 if a.len() >= 2 => {
            if a[0] <= 1 {
                wr(m, a[1], rd(m, 0x225088));
                Some(0)
            } else {
                Some(rd(m, 0x225084))
            }
        }
        0x22AD74 if a.len() >= 2 => {
            if a[0] <= 1 {
                wr(m, a[1], rd(m, 0x22AD98));
                Some(0)
            } else {
                Some(rd(m, 0x22AD9C))
            }
        }
        0x22E1F8 if a.len() >= 2 => {
            if a[0] <= 1 {
                wr(m, a[1], rd(m, 0x22E218));
                Some(0)
            } else {
                Some(rd(m, 0x22E21C))
            }
        }
        0x2304F0 if a.len() >= 2 => {
            if a[0] <= 1 {
                wr(m, a[1], rd(m, 0x2306F8));
                Some(0)
            } else {
                Some(rd(m, 0x2306FC))
            }
        }
        0x1795F0 if a.len() >= 4 => {
            let cfg = dctlkp_config(m, h);
            let mut rh = RouteHost { inner: h };
            let d = tts_core::boot_part_a::Dctlkp::new(
                &mut rh,
                &cfg,
                tts_core::component_live_c::DictionaryWorkspace::default(),
            );
            Some(d.broker_open(m, a[0], a[1], a[2], a[3]))
        }
        0x179F3C if a.len() >= 2 => {
            let scratch = fn_129778(m, h, 256);
            let fold = std::array::from_fn(|i| rh(m, rd(m, 0x17C8F8) + i as u32 * 2));
            let cfg = tts_core::leftovers_1::CloseRuntime {
                category: rd(m, 0x1795CC),
                begin: 0x17A14C,
                end: 0x17A18C,
                error: 0x17A180,
                handle_key: 0x17A178,
                status_key: 0x17A170,
                scratch,
                fold,
            };
            let mut rh = RouteHost { inner: h };
            Some(tts_core::boot_part_a::fn_179f3c_m(
                m, a[0], a[1], &cfg, &mut rh,
            ))
        }
        0x17A1F4 if a.len() >= 2 => {
            if a[0] == 0 || a[1] == 0 {
                Some(rd(m, 0x1799D0))
            } else if rd(m, a[1] + 4) == 0 {
                route_call(m, h, 0x1733A0, &[rd(m, a[1])])
            } else {
                Some(0)
            }
        }
        0x1733A0 if !a.is_empty() => {
            let mut rh = RouteHost { inner: h };
            Some(tts_core::boot_part_d::fn_1733a0(m, a[0], &mut rh))
        }
        0x2413C4 if a.len() >= 5 => Some(fn_2413c4(m, h, a[0], a[1], a[2], a[3], a[4])),
        0x243AD4 if a.len() >= 3 => Some(fn_243ad4(m, h, a[0], a[1], a[2])),
        0x244324 if !a.is_empty() => Some(fn_244324(m, h, a[0])),
        0x2421B0 if !a.is_empty() => Some(fn_2421b0(m, h, a[0])),
        0x17C9D8 if a.len() >= 5 => {
            let mut rh = RouteHost { inner: h };
            Some(tts_core::boot_part_e::fn_17c9d8(
                m, a[0], a[1], a[2], a[3], a[4], &mut rh,
            ))
        }
        0x1731C4 if a.len() >= 4 => {
            let mut rh = RouteHost { inner: h };
            Some(tts_core::boot_part_d::fn_1731c4(
                m, a[0], a[1], a[2], a[3], &mut rh,
            ))
        }
        0x242C9C if a.len() >= 5 => Some(fn_242c9c(m, h, a[0], a[1], a[2], a[3], a[4])),
        0x179F30 if a.len() >= 3 => route_call(m, h, 0x1795F0, &[a[0], a[1], 0, a[2]]),
        0x179374 if a.len() >= 5 => {
            let mut rh = RouteHost { inner: h };
            Some(tts_core::boot_part_b::fn_179374(
                m, a[0], a[1], a[2], a[3], a[4], &mut rh,
            ))
        }
        0x241E74 if a.len() >= 4 => Some(fn_241e74(m, h, a[0], a[1], a[2], a[3])),
        0x24277C if a.len() >= 4 => Some(fn_24277c(m, h, a[0], a[1], a[2], a[3])),
        0x242590 if a.len() >= 3 => Some(fn_242590(m, h, a[0], a[1], a[2])),
        0x2422AC if !a.is_empty() => Some(fn_2422ac(m, a[0])),
        0x242444 if a.len() >= 6 => Some(tts_core::render_part_a::read_chunk_string(
            &mut tts_core::selection::arena::Arena::new(0, m.bytes),
            a[0],
            a[1],
            a[2],
            a[3],
            a[4],
            a[5],
        )),
        0x243A94 if a.len() >= 7 => Some(fn_242fd0(
            m,
            h,
            [a[0], a[1], a[2], a[3], 0, 0, a[4], a[5] & 0xffffff, a[6]],
        )),
        0x242FD0 if a.len() >= 9 => Some(fn_242fd0(m, h, a[..9].try_into().unwrap())),
        0x1E4270 if a.len() >= 5 => Some(fn_1e4270(m, h, a[2], a[3], a[4])),
        0x24D5A4 | 0x24D600 if a.len() >= 3 => {
            if a[0] != 0 && a[1] != 0 && a[2] != 0 {
                wr(m, a[2], a[0]);
                Some(0)
            } else {
                Some(tts_core::normalization::forward_error(7))
            }
        }
        0x24D5E0 | 0x24D63C if !a.is_empty() => Some(a[0]),
        0x24D5E4 if a.len() >= 2 => Some(match rb(m, a[0]).cmp(&rb(m, a[1])) {
            std::cmp::Ordering::Less => u32::MAX,
            std::cmp::Ordering::Equal => 0,
            _ => 1,
        }),
        0x244318 if !a.is_empty() => Some(if a[0] == 0 { 0 } else { rd(m, a[0] + 52) }),
        0x1E36BC if a.len() >= 5 => Some(fn_1e36bc(m, h, a[2], a[3], a[4])),
        0x22CD30 if a.len() >= 5 => Some(fn_22cd30(m, h, a[2], a[3], a[4])),
        0x2956F4 if a.len() >= 2 => Some(fn_2956f4(m, h, a[0], a[1])),
        0x2955C0 if !a.is_empty() => {
            fn_2955c0(m, h, a[0]);
            Some(a[0])
        }
        0x29556C if a.len() >= 2 => {
            fn_29556c(m, a[0], a[1] as i32);
            Some(a[0])
        }
        0x2D0A5C if a.len() >= 2 => {
            let template = rd(m, 0x2D076C);
            let mut host = RouteHost { inner: h };
            Some(tts_core::boot_port_6::fn_2d0a5c(
                m, &mut host, a[0], a[1], template,
            ))
        }
        0x2D09A4 | 0x2D0A08 if !a.is_empty() => {
            let template = rd(m, 0x2D076C);
            if f == 0x2D09A4 {
                tts_core::boot_port_6::fn_2d09a4(m, a[0], template);
            } else {
                tts_core::boot_port_6::fn_2d0a08(m, a[0], template);
            }
            Some(a[0])
        }
        0x2CFD6C if a.len() >= 2 => {
            let tables = [rd(m, 0x2D0740), rd(m, 0x2D0744), rd(m, 0x2D0748)];
            let scratch = fn_129778(m, h, 4);
            let mut host = RouteHost { inner: h };
            Some(tts_core::boot_port_6::fn_2cfd6c(
                m, &mut host, a[0], a[1], tables, scratch,
            ))
        }
        0x2296B0 if a.len() >= 5 => Some(fn_2296b0(m, h, a[2], a[3], a[4])),
        0x228C94 if !a.is_empty() => Some(fn_228c94(m, h, a[0])),
        0x228780 if a.len() >= 3 => Some(fn_228780(m, h, a[0], a[1], a[2])),
        0x2A9D5C | 0x2A9DA0 | 0x2A9E20 | 0x2A9E48 | 0x2A9E64 if a.len() >= 2 => {
            Some(acoustic_control(m, f, a[0], a[1]))
        }
        0x2AA3E4 if a.len() >= 11 => {
            let k = token_block_env(m, h).reader;
            let frame = fn_129778(m, h, 256);
            let tables = std::array::from_fn(|i| rd(m, 0x2DB994 + i as u32 * 4));
            let mut host = RouteHost { inner: h };
            Some(tts_core::boot_port_5::fn_2aa3e4(
                m,
                &mut host,
                a[..11].try_into().unwrap(),
                frame,
                &k,
                &tables,
            ))
        }
        0x2AA294 if !a.is_empty() => {
            let mut host = RouteHost { inner: h };
            Some(tts_core::boot_port_5::fn_2aa294(m, &mut host, a[0]))
        }
        0x228B48 if a.len() >= 7 => Some(fn_228b48(
            m,
            h,
            a[0],
            a[1],
            a[2] as i32,
            a[3] as i32,
            a[4],
            a[5],
            a[6],
        )),
        0x2A9608 if a.len() >= 2 => {
            let mut value = 0;
            let r = tts_core::selection::inverse_percent(a[0] as i32, &mut value);
            wr(m, a[1], value as u32);
            Some(r)
        }
        0x2A2F04 if a.len() >= 6 => Some(fn_2a2f04(
            m,
            h,
            a[0],
            a[1],
            a[2] as i32,
            a[3],
            a[4],
            a[5],
            rd(m, 0x2A3338),
        )),
        0x2DB19C if a.len() >= 3 => {
            fn_3023b0(m, a[0], 24);
            wr(m, a[0], a[1]);
            wr(m, a[0] + 4, a[2]);
            wr(m, a[0] + 12, 51200);
            Some(51200)
        }
        0x2DB1D0 if a.len() >= 2 => Some(fn_private_heap(m, h, a[0], 0, a[1])),
        0x2DB4A0 if !a.is_empty() => Some(fn_private_heap(m, h, a[0], 2, 0)),
        0x2A9274 if a.len() >= 5 => Some(fn_2a9274(m, h, a[0], a[1], a[2], a[3], a[4])),
        0x2A90C4 if a.len() >= 4 => Some(fn_2a90c4(m, h, a[0], a[1], a[2], a[3])),
        0x2A4620 if a.len() >= 3 => {
            let k = model_literals(m, h, 0x2D9C0C);
            let l = tts_core::boot_port_5::PoolLiterals {
                category: rd(m, 0x2DB3C4),
                format: 0x2DB384,
                after_load: 0x2A4734,
                before_free: 0x2DB564,
                scratch: fn_129778(m, h, 32),
            };
            let names = rd(m, 0x2D9D80);
            let mut host = RouteHost { inner: h };
            Some(tts_core::boot_port_5::fn_2a4620(
                m, &mut host, a[0], a[1], a[2], l, &k, names,
            ))
        }
        0x2DA04C if a.len() >= 5 => Some(fn_2da04c(m, h, a[0], a[1], a[2], a[3], a[4])),
        0x2F7C44 | 0x2F6A50 | 0x2F7760 | 0x2F80B0 => {
            let k = model_literals(m, h, f);
            let unmap = dctlkp_config(m, h).unmap;
            let r = tts_core::boot_port_6::StreamRuntime {
                unmap: &unmap,
                scratch: fn_129778(m, h, 64),
            };
            let mut host = RouteHost { inner: h };
            Some(match f {
                0x2F7C44 => {
                    tts_core::boot_port_6::fn_2f7c44(m, &mut host, a[0], a[1], a[2], a[3], &k, &r)
                }
                0x2F6A50 => {
                    tts_core::boot_port_6::fn_2f6a50(m, &mut host, a[0], a[1], a[2], &k, &r)
                }
                0x2F7760 => {
                    tts_core::boot_port_6::fn_2f7760(m, &mut host, a[0], a[1], a[2], a[3], &k, &r)
                }
                _ => tts_core::boot_port_6::fn_2f80b0(
                    m,
                    &mut host,
                    a[0],
                    a[1],
                    a[2],
                    a[3],
                    a[4],
                    a[5] as i32,
                    &k,
                    &r,
                ),
            })
        }
        0x2A41E0 if a.len() >= 3 => {
            let category = rd(m, 0x2A43D4);
            let mut host = RouteHost { inner: h };
            Some(tts_core::boot_port_5::fn_2a41e0(
                m,
                &mut host,
                a[0],
                a[1],
                a[2],
                [
                    0x2A43D8, 0x2A43F4, 0x2A4410, 0x2A442C, 0x2A4448, 0x2A4464, 0x2A4480, 0x2A43A8,
                    0x2A44E4,
                ],
                category,
            ))
        }
        0x4A7AAC if !a.is_empty() => {
            let allocator = rd(m, a[0] + 12);
            route_or_host(m, h, rd(m, rd(m, allocator) + 12), &[allocator, a[0]]);
            Some(0)
        }
        0x2A0178 if a.len() >= 6 => {
            let sp = fn_129778(m, h, 96);
            let mut host = RouteHost { inner: h };
            Some(tts_core::boot_port_3b::open_voice_object_m(
                m, &mut host, a[0], a[1], a[2], a[3], a[4], a[5], sp,
            ))
        }
        0x2A4864 if !a.is_empty() => {
            tts_core::selection::arena_mid::default_voice_config(
                &mut tts_core::selection::arena::Arena::new(0, m.bytes),
                a[0],
            );
            Some(100)
        }
        0x2A4180 if a.len() >= 2 => {
            let mut host = RouteHost { inner: h };
            tts_core::boot_port_5::fn_2a4180(m, &mut host, a[0], a[1]);
            Some(a[0])
        }
        0x2A9090 if !a.is_empty() => {
            let mut host = RouteHost { inner: h };
            Some(tts_core::boot_port_3b::fn_2a9090(m, &mut host, a[0]))
        }
        0x175850 if a.len() >= 3 => {
            let sp = fn_129778(m, h, 8);
            wr(m, sp, 0);
            wr(m, sp + 4, 0);
            let r = fn_1756d0(m, h, a[0], a[1], sp, sp + 4);
            let n = if r < 0x80000000 && rd(m, sp + 4) > 1 {
                parse_decimal(&m.bytes[rd(m, sp) as usize..]) as u32
            } else {
                0
            };
            wr(m, a[2], n);
            Some(r)
        }
        0x2271F4 if a.len() >= 5 => Some(fn_2271f4(m, h, a[2], a[3], a[4])),
        0x2252B0 if !a.is_empty() => {
            let scratch = fn_129778(m, h, 1024);
            let owned_literals: Vec<_> = tts_core::boot_port_3b::LITERAL_ADDRESSES
                .iter()
                .map(|&p| {
                    let n = fn_195d90(m, p) as usize;
                    (
                        m.bytes[(p - m.base) as usize..(p - m.base) as usize + n].to_vec(),
                        p,
                    )
                })
                .collect();
            let literals: Vec<_> = owned_literals
                .iter()
                .map(|(text, p)| (text.as_slice(), *p))
                .collect();
            let classes = std::array::from_fn(|i| rh(m, rd(m, 0x17C8F8) + i as u32 * 2));
            let e = tts_core::boot_port_3b::Environment {
                scratch,
                version: rd(m, rd(m, 0x22563C)),
                module: rd(m, 0x2269F4),
                character_classes: &classes,
                literals: &literals,
            };
            let mut host = RouteHost { inner: h };
            Some(tts_core::boot_port_3b::fn_2252b0(m, a[0], &e, &mut host))
        }
        0x2100F0 if a.len() >= 5 => Some(fn_2100f0(m, h, a[2], a[3], a[4])),
        0x1F4E2C if a.len() >= 5 => Some(fn_1f4e2c(m, h, a[2], a[3], a[4])),
        0x1F3AC8 if a.len() >= 5 => Some(fn_1f3ac8(m, h, a[2], a[3], a[4])),
        0x1E7318 if a.len() >= 5 => Some(fn_1e7318(m, h, a[2], a[3], a[4])),
        0x1DE880 if a.len() >= 5 => Some(fn_1de880(m, h, a[2], a[3], a[4])),
        0x1F2830 if a.len() >= 5 => Some(fn_1f2830(m, h, a[2], a[3], a[4])),
        0x1F1224 if a.len() >= 5 => Some(fn_1f1224(m, h, a[2], a[3], a[4])),
        0x1F80D4 if !a.is_empty() => Some(fn_1f80d4(m, h, a[0])),
        0x1F87B8 if a.len() >= 3 => {
            wr(m, a[2], a[0]);
            Some(0)
        }
        0x1F87B4 if !a.is_empty() => Some(a[0]),
        0x1F87C4 if a.len() >= 5 => Some(fn_1f87c4(m, h, a[2], a[3], a[4])),
        0x1D8A20 if a.len() >= 5 => Some(fn_1d8a20(m, h, a[2], a[3], a[4])),
        0x20683C if a.len() >= 5 => {
            let pair = pair_176fd4(m);
            let scratch = fn_129778(m, h, 160);
            let common = tts_core::boot_port_3::Environment {
                null_pair: std::array::from_fn(|i| pair[i / 4].to_le_bytes()[i % 4]),
                default_context: rd(m, 0x196304),
                inet_name: rom_text(m, b"INET"),
                module: rd(m, 0x206B0C),
                scratch,
            };
            let e = tts_core::boot_port_3::ConstructorEnvironment {
                common,
                names: [0x20681C, 0x206824, 0x206830],
                section: 0x206B04,
                keys: [0x206AF8, 0x206AF8],
            };
            let mut host = RouteHost { inner: h };
            Some(tts_core::boot_port_3::fn_20683c(
                m, a[0], a[1], a[2], a[3], a[4], &e, &mut host,
            ))
        }
        0x2512DC if a.len() >= 3 => Some(fn_2512dc(m, h, a[0], a[2])),
        0x24F570 if a.len() >= 5 => {
            let e = token_block_env(m, h);
            let mut host = RouteHost { inner: h };
            Some(tts_core::boot_port_4b::fn_24f570(
                m, &mut host, a[0], a[1], a[2], a[3], a[4], &e,
            ))
        }
        0x1E5278 if !a.is_empty() => {
            let k = token_config(m);
            let sp = fn_129778(m, h, 128);
            let mut host = RouteHost { inner: h };
            Some(tts_core::boot_port_2::fn_1e5278(m, a[0], &k, sp, &mut host))
        }
        0x1E512C if !a.is_empty() => {
            let mut host = RouteHost { inner: h };
            Some(tts_core::boot_port_2::fn_1e512c(m, a[0], &mut host))
        }
        0x24DAC8 if !a.is_empty() => Some(fn_24dac8(m, h, a[0])),
        0x24D640 if !a.is_empty() => {
            let r = token_data_new(
                m,
                h,
                a[0],
                a[0] + 60,
                8,
                [0x24D700, 0x24D720],
                [0x24D744, 0x24D744],
            );
            Some(r)
        }
        0x24D84C if a.len() >= 2 => Some(fn_24d84c(m, h, a[0], a[1])),
        0x24D8B8 if a.len() >= 3 => {
            wr(m, rd(m, a[0] + 60) + a[1] * 4, a[2]);
            Some(0)
        }
        0x2520E4 if !a.is_empty() => Some(fn_2520e4(m, h, a[0])),
        0x24D3BC if !a.is_empty() => Some(fn_24d3bc(m, h, a[0])),
        0x24EF54 if a.len() >= 3 => Some(fn_24ef54(m, h, a[2])),
        0x2B645C if a.len() >= 2 => Some(token_data_new(
            m,
            h,
            a[0],
            a[1],
            8,
            [0x2B6550, 0x2B6570],
            [0x2B6548, 0x2B654C],
        )),
        0x2B5CEC if a.len() >= 2 => Some(token_data_new(
            m,
            h,
            a[0],
            a[1],
            2,
            [0x2B5DC8, 0x2B5DE4],
            [0x2B5E04, 0x2B5E08],
        )),
        0x250918 if a.len() >= 2 => Some(token_data_new(
            m,
            h,
            a[0],
            a[1],
            8,
            [0x2509FC, 0x250A18],
            [0x250A38, 0x24FE88],
        )),
        0x1E3EF8 if a.len() >= 2 => {
            let mut host = RouteHost { inner: h };
            Some(tts_core::boot_port_2::fn_1e3ef8(m, a[0], a[1], &mut host))
        }
        0x200A2C if a.len() >= 5 => Some(fn_200a2c(m, h, a[2], a[3], a[4])),
        0x1D79DC if a.len() >= 5 => {
            let pair = pair_176fd4(m);
            let g = tts_core::boot_port_2::ContextGlobals {
                empty: std::array::from_fn(|i| pair[i / 4].to_le_bytes()[i % 4]),
                fallback: rd(m, 0x196304),
            };
            let mut host = RouteHost { inner: h };
            Some(tts_core::boot_port_2::fn_1d79dc(
                m, a[2], a[3], a[4], &g, &mut host,
            ))
        }
        0x1D7C3C if a.len() >= 4 => {
            let sp = fn_129778(m, h, 8);
            let mut host = RouteHost { inner: h };
            Some(tts_core::boot_port_2::fn_1d7c3c(
                m, a[0], a[1], a[2], a[3], sp, &mut host,
            ))
        }
        0x1EC8B4 if a.len() >= 4 => Some(fn_1ec8b4(m, h, a[0], a[1], a[2], a[3])),
        0x1D4D6C if a.len() >= 5 => Some(fn_1d4d6c(m, h, a[2], a[3], a[4])),
        0x178510 if a.len() >= 5 => Some(route_or_host(
            m,
            h,
            0x177E0C,
            &[a[0], a[1], a[2], 0, a[3], a[4]],
        )),
        0x2198B4 if a.len() >= 5 => Some(fn_2198b4(m, h, a[2], a[3], a[4])),
        0x1F4558 if a.len() >= 5 => Some(fn_1f4558(m, h, a[2], a[3], a[4])),
        0x1EF0A0 if a.len() >= 5 => {
            let e = extra_environment(m, h);
            let mut host = ColdExtra(h);
            Some(tts_core::boot_port_2c::fn_1ef0a0(
                m, a[2], a[3], a[4], &e, &mut host,
            ))
        }
        0x1EE9F8 if !a.is_empty() => {
            let e = extra_environment(m, h);
            let mut host = ColdExtra(h);
            Some(tts_core::boot_port_2c::fn_1ee9f8(m, a[0], &e, &mut host))
        }
        0x2029DC if a.len() >= 5 => Some(fn_2029dc(m, h, a[2], a[3], a[4])),
        0x15F594 if a.len() >= 5 => Some(fn_15f594(m, h, a[0], a[1], a[2], a[3], a[4])),
        0x1CFFA4 if a.len() >= 5 => {
            let strings = tts_core::boot_port_1c::STRING_ADDRESSES;
            let globals = rd(m, 0x1CFBB4);
            let pair = pair_176fd4(m);
            let scratch = fn_129778(m, h, 8192);
            let data = tts_core::boot_port_1c::Data {
                strings,
                callback: [rd(m, globals + 0x17c), rd(m, globals + 0x180)],
                destroy_callback: [rd(m, 0x8F884C), rd(m, 0x8F8850)],
                default_environment: rd(m, 0x196304),
                null_handle: ((pair[1] as u64) << 32 | pair[0] as u64).to_le_bytes(),
                ctype: rd(m, 0x17C8F8),
                category: 0x8F87BC,
                owner_defaults: [rd(m, 0x8D7628), rd(m, 0x8D762C), rd(m, 0x8D7630)],
                scratch,
            };
            let mut host = RouteHost { inner: h };
            Some(tts_core::boot_port_1c::fn_1cffa4(
                m,
                [a[0], a[1]],
                [a[2], a[3]],
                a[4],
                &data,
                &mut host,
            ))
        }
        0x1ECD04 if a.len() >= 4 => Some(fn_1ecd04(m, h, a[0], a[1], a[2], a[3])),
        0x2ECAAC if a.len() >= 4 => Some(fn_2ecaac(m, h, a[0], a[1], a[2], a[3])),
        0x2C3068 if a.len() >= 3 => Some(fn_2c3068(m, h, a[0], a[1], a[2])),
        0x2BF244 if a.len() >= 5 => {
            let shared: std::cell::RefCell<&mut dyn Calls> = std::cell::RefCell::new(h);
            let mut calls = SharedCalls(&shared);
            let mut host = RouteHost { inner: &mut calls };
            let mut chunks = ColdChunks(&shared);
            Some(tts_core::text_input::read_dictionary(
                m.bytes,
                a[0],
                a[1],
                a[3],
                a[4],
                &mut host,
                &mut chunks,
            ))
        }
        0x2EE920 if a.len() >= 5 => {
            let mut host = RouteHost { inner: h };
            Some(tts_core::render_part_d::read_escape_chunk(
                m.bytes, a[0], a[1], a[2], a[3], a[4], &mut host,
            ))
        }
        0x2C3DB8 if a.len() >= 3 => {
            let mut host = RouteHost { inner: h };
            Some(tts_core::text_input::alloc_handle_item(
                m.bytes, a[0], a[1], a[2], &mut host,
            ))
        }
        0x2BF760 if a.len() >= 7 => {
            let scratch = fn_129778(m, h, 64);
            let pair = pair_176fd4(m);
            let classes = std::array::from_fn(|i| rh(m, rd(m, 0x17C8F8) + i as u32 * 2));
            let env = tts_core::render_part_c::Environment {
                cached_handle: ((pair[1] as u64) << 32 | pair[0] as u64).to_le_bytes(),
                default_context: rd(m, 0x196304),
            };
            let data = tts_core::render_part_c::ParserConstructorDataM {
                names: [0x2BFA68, 0x2BF754, 0x2BFA88, 0x2BFA78],
                callbacks: [rd(m, 0x2BFA70), rd(m, 0x2BFA74)],
                classes: &classes,
                stack: scratch,
            };
            let mut host = RouteHost { inner: h };
            Some(tts_core::render_part_c::fn_2bf760_m(
                m,
                [a[0], a[1]],
                [a[2], a[3], a[4], a[5], a[6]],
                &env,
                &data,
                &mut host,
            ))
        }
        0x1EA4E0 if a.len() >= 7 => {
            let cfg = dctlkp_config(m, h);
            let mut host = RouteHost { inner: h };
            let dict = tts_core::boot_part_a::Dctlkp::new(
                &mut host,
                &cfg,
                tts_core::component_live_c::DictionaryWorkspace::default(),
            );
            Some(dict.query(m, a[..7].try_into().unwrap()))
        }
        0x263B70 if a.len() >= 4 => Some(fn_263b70(m, h, a)),
        0x2EEE7C if a.len() >= 6 => Some(fn_2eee7c(m, h, a)),
        0x2C5EF4 if a.len() >= 3 => Some(fn_2c5ef4(m, h, a[0], a[1], a[2])),
        0x2C6194 if a.len() >= 2 => Some(fn_2c5ef4(m, h, a[0], 0, a[1])),
        0x2C6434 if a.len() >= 4 => Some(fn_2c6434(m, h, a[0], a[1], a[2], a[3])),
        0x263C68 if a.len() >= 2 => Some(fn_263c68(m, h, a[0], a[1])),
        0x2C464C if a.len() >= 6 => Some(fn_2c464c(m, h, a)),
        0x1E555C if a.len() >= 5 => Some(fn_1e555c(m, h, a[2], a[3], a[4])),
        0x26402C if a.len() >= 4 => Some(fn_26402c(m, h, a[0], a[1], a[2], a[3])),
        0x252CF8 if a.len() >= 3 => {
            let sp = fn_129778(m, h, 8);
            let rc = fn_19611c(m, a[0], a[1], sp);
            if a[2] == 0 || rc >= 0x80000000 {
                Some(0x81102007)
            } else {
                wr(m, a[2], 0);
                let obj = fn_196394(m, h, rd(m, rd(m, sp) + 4), 16);
                if obj == 0 {
                    Some(0x8110200a)
                } else {
                    fn_3023b0(m, obj, 16);
                    wr(m, obj, a[0]);
                    wr(m, obj + 4, a[1]);
                    wr(m, a[2], obj);
                    Some(0)
                }
            }
        }
        0x22AF2C if a.len() >= 5 => Some(fn_22af2c(m, h, a[2], a[3], a[4])),
        0x23FE84 if a.len() >= 3 => Some(fn_23fe84(m, h, a[0], a[1], a[2])),
        0x2AE690 if a.len() >= 3 => Some(fn_2ae690(m, h, a[0], a[1], a[2])),
        0x1757B8 if a.len() >= 3 => {
            let sp = fn_129778(m, h, 4096);
            route_or_host(m, h, 0x176AC8, &[a[2], sp, 10]);
            let n = fn_195d90(m, sp) + 1;
            Some(route_or_host(m, h, 0x175170, &[a[0], a[1], sp, n, 0]))
        }
        0x201748 if a.len() >= 4 => Some(fn_201748(m, h, a[0], a[1], a[2], a[3])),
        0x281AA4 if a.len() >= 2 => {
            let n = if rd(m, a[1]) == 0 {
                rd(m, a[0] + 4)
            } else {
                rd(m, rd(m, a[1]) + 8)
            };
            wr(m, a[1], n);
            Some(if n == 0 { 0 } else { rd(m, n + 4) })
        }
        0x2818B8 if a.len() >= 2 => Some(fn_2818b8(m, h, a[0], a[1])),
        0x1AA1F0 if a.len() >= 5 => {
            let scratch = fn_129778(m, h, 64);
            let e = tts_core::boot_port_1::Streams {
                parameters: tts_core::boot_part_f::Environment::US,
                plain: rd(m, 0x1B2534),
                encoded: rd(m, 0x1B2D3C),
                seek: 0,
                seek_adjustment: 12,
            };
            let mut host = RouteHost { inner: h };
            Some(tts_core::boot_port_1::fn_1aa1f0(
                m,
                a[..5].try_into().unwrap(),
                scratch,
                &e,
                &mut host,
            ))
        }
        0x1A8F1C if !a.is_empty() => {
            let mut host = RouteHost { inner: h };
            Some(tts_core::boot_port_1::fn_1a8f1c(m, a[0], &mut host))
        }
        0x1B44C8 | 0x1BA3D0 => Some(0),
        0x1B3208 if !a.is_empty() => Some(tts_core::boot_part_e::fn_1b3208(m, a[0])),
        0x1B44CC if !a.is_empty() => {
            let scratch = fn_129778(m, h, 8192);
            let mut host = ColdArchive {
                inner: h,
                scratch,
                plain: rd(m, 0x1B2534),
                encoded: rd(m, 0x1B2D3C),
            };
            Some(tts_core::boot_part_e::fn_1b44cc(m, a[0], &mut host))
        }
        0x1B3138 if !a.is_empty() => Some(a[0]),
        0x1B3210 if !a.is_empty() => Some(rd(m, a[0] + 40)),
        0x1B313C if a.len() >= 2 => {
            let scratch = fn_129778(m, h, 8192);
            let mut data = vec![0; rd(m, a[0] + 40).max(4) as usize];
            let mut host = ColdArchive {
                inner: h,
                scratch,
                plain: rd(m, 0x1B2534),
                encoded: rd(m, 0x1B2D3C),
            };
            Some(tts_core::boot_port_1::fn_1b313c(
                m,
                a[0],
                a[1],
                scratch + 1536,
                &mut data,
                &mut host,
            ))
        }
        0x1AA9E0 if a.len() >= 2 => {
            let mut host = RouteHost { inner: h };
            Some(tts_core::boot_port_1::fn_1aa9e0(m, a[0], a[1], &mut host))
        }
        0x1AA8FC if a.len() >= 2 => {
            wb(m, a[0] + 72, 0);
            let p = a[1];
            let vt = rd(m, p);
            let x = route_or_host(m, h, rd(m, vt + 64), &[p.wrapping_sub(rd(m, vt + 104))]);
            let vt = rd(m, p);
            let y = route_or_host(m, h, rd(m, vt + 64), &[p.wrapping_sub(rd(m, vt + 104))]);
            Some(route_or_host(m, h, rd(m, rd(m, y) + 16), &[x]))
        }
        0x1AA848 if a.len() >= 4 => {
            let sp = fn_129778(m, h, 4);
            let vt = rd(m, a[0] + 20);
            let r = route_or_host(
                m,
                h,
                rd(m, vt + 76),
                &[a[0].wrapping_sub(rd(m, vt + 92)) + 20, a[1], a[2], a[3], sp],
            );
            Some(if r == 0 && rb(m, sp) == 1 { 4 } else { r })
        }
        0x1AA3CC if a.len() >= 7 => {
            let sp = fn_129778(m, h, 4);
            let vt = rd(m, a[0] + 20);
            let r = route_or_host(
                m,
                h,
                rd(m, vt + 24),
                &[
                    a[0].wrapping_sub(rd(m, vt + 92)) + 20,
                    a[1],
                    a[2],
                    a[3],
                    a[4],
                    a[5],
                    a[6],
                    sp,
                ],
            );
            Some(if r == 0 && rb(m, sp) != 0 { 4 } else { r })
        }
        0x1AA890 if a.len() >= 5 => {
            let sp = fn_129778(m, h, 12);
            wr(m, sp + 4, 0);
            let r = route_or_host(
                m,
                h,
                0x1A9A60,
                &[a[0], a[1], a[2], sp, sp + 4, sp + 8, a[4]],
            );
            if r == 0 {
                wb(m, a[3], u8::from(rd(m, sp + 4) != u32::MAX));
            }
            Some(r)
        }
        0x1A9A60 if a.len() >= 7 => {
            let sp = fn_129778(m, h, 8192);
            let e = tts_core::boot_part_e::ArchiveEnvironment {
                parameters: tts_core::boot_part_f::Environment::US,
                stream_table: 0x8FAAE0,
                wrapper_table: 0x917E3C,
                validator: 0x1A8684,
                substream_table: 0x917E04,
                nvar: 0x17D078,
                chinese: 0x17D080,
                scratch: sp + 2048,
            };
            let mut host = ColdArchive {
                inner: h,
                scratch: sp,
                plain: rd(m, 0x1B2534),
                encoded: rd(m, 0x1B2D3C),
            };
            Some(tts_core::boot_port_5b::fn_1a9a60(
                m,
                a[..7].try_into().unwrap(),
                sp + 1536,
                &e,
                &mut host,
            ))
        }
        0x1AA830 if !a.is_empty() => Some(rd(m, a[0] + 32)),
        0x1AA838 if !a.is_empty() => Some(rd(m, a[0] + 24)),
        0x1AA840 if !a.is_empty() => Some(rd(m, a[0] + 28)),
        0x1AA8DC if !a.is_empty() => Some(u32::from(rb(m, a[0] + 81) == 0)),
        0x1AA8F0 if !a.is_empty() => Some(route_or_host(m, h, rd(m, rd(m, a[0]) + 16), &[a[0]])),
        0x281B30 if a.len() >= 4 => {
            let n = a[1]
                .wrapping_mul(a[2])
                .min(rd(m, a[3] + 24).wrapping_sub(rd(m, a[3] + 28)));
            if n != 0 {
                fn_195ae8(m, a[0], rd(m, a[3] + 20) + rd(m, a[3] + 28), n);
                wr(m, a[3] + 28, rd(m, a[3] + 28) + n);
            }
            Some(route_or_host(m, h, 0x1994D0, &[n, a[1]]))
        }
        0x281B8C if a.len() >= 3 => {
            let mut stream = tts_core::frontend_resources::MemoryStream {
                data: Some(&m.bytes[rd(m, a[0] + 20) as usize..]),
                length: rd(m, a[0] + 24),
                position: rd(m, a[0] + 28),
            };
            let r = stream.seek(a[1] as i32, a[2]);
            wr(m, a[0] + 28, stream.position);
            Some(r)
        }
        0x281AF4 => Some(0),
        0x281AFC if !a.is_empty() => {
            let p = a[0];
            if p != 0 && rd(m, p + 20) != 0 {
                if rb(m, p + 17) != 0 {
                    fn_196428(m, h, rd(m, p), rd(m, p + 20));
                }
                wr(m, p + 20, 0);
            }
            Some(0)
        }
        0x2B3F04 if a.len() >= 3 => {
            let mut host = RouteHost { inner: h };
            Some(tts_core::boot_port_5::fn_2b3f04(
                m, &mut host, a[0], a[1], a[2],
            ))
        }
        0x2B4010 if a.len() >= 8 => Some(fn_2b4010(m, h, a[..8].try_into().unwrap())),
        0x281F34 if a.len() >= 2 => Some(fn_281f34(m, h, a[0], a[1])),
        0x281C14 if a.len() >= 3 => {
            let methods = [
                rd(m, 0x281E80),
                rd(m, 0x281E84),
                rd(m, 0x281E88),
                rd(m, 0x281E8C),
                rd(m, 0x281E90),
                rd(m, 0x281E94),
                rd(m, 0x281E9C),
                rd(m, 0x281E98),
            ];
            let scratch = fn_129778(m, h, 32);
            let category = rd(m, 0x281EA4);
            let mut host = RouteHost { inner: h };
            Some(tts_core::boot_port_5::fn_281c14(
                m,
                &mut host,
                [a[0], a[1]],
                a[2],
                methods,
                category,
                scratch,
            ))
        }
        0x1FF754 if a.len() >= 3 => {
            let r = route_or_host(m, h, 0x176F94, &[a[0], a[1], 0x11d, 72]);
            if r >= 0x80000000 {
                Some(r)
            } else {
                let sp = fn_129778(m, h, 4);
                let r = fn_19611c(m, rd(m, a[0]), rd(m, a[0] + 4), sp);
                if r < 0x80000000 {
                    wr(m, a[2], rd(m, 0x1FFAAC));
                }
                Some(r)
            }
        }
        0x200C88 if a.len() >= 3 => {
            let r = route_or_host(m, h, 0x176F94, &[a[0], a[1], rd(m, 0x200C08), 32]);
            if r >= 0x80000000 {
                Some(rd(m, 0x20125C))
            } else {
                Some(route_or_host(
                    m,
                    h,
                    rd(m, rd(m, a[0] + 8) + 72),
                    &[rd(m, a[0] + 12), rd(m, a[0] + 16), a[2]],
                ))
            }
        }
        0x205F40 if a.len() >= 9 => Some(fn_205f40(m, h, a[..9].try_into().unwrap())),
        0x17486C if a.len() >= 4 => {
            let scratch = fn_129778(m, h, 160);
            let mut host = RouteHost { inner: h };
            Some(tts_core::boot_port_1::fn_17486c(
                m, a[0], a[1], a[2], a[3], scratch, &mut host,
            ))
        }
        0x205F00 if a.len() >= 3 => {
            let r = route_or_host(m, h, 0x176F94, &[a[0], a[1], rd(m, 0x20636C), 184]);
            if r >= 0x80000000 {
                Some(rd(m, 0x206370))
            } else {
                let table = rd(
                    m,
                    if rd(m, a[0] + 88) != 0 {
                        0x206374
                    } else {
                        0x206378
                    },
                );
                wr(m, a[2], table);
                Some(0)
            }
        }
        0x1E5F2C if a.len() >= 3 => {
            let r = route_or_host(m, h, 0x176F94, &[a[0], a[1], rd(m, 0x1E5544), 48]);
            if r >= 0x80000000 {
                Some(rd(m, 0x1E5548))
            } else if a[0] == 0 {
                Some(rd(m, 0x1E554C))
            } else {
                route_call(m, h, 0x265B3C, &[rd(m, a[0] + 40), a[2]])
            }
        }
        0x265B3C if a.len() >= 2 => {
            let mut host = RouteHost { inner: h };
            Some(tts_core::boot_port_5::fn_265b3c(m, &mut host, a[0], a[1]))
        }
        0x22BDDC if a.len() >= 3 => {
            let r = route_or_host(m, h, 0x176F94, &[a[0], a[1], 0x10d, 56]);
            if r < 0x80000000 {
                wr(m, a[2], rd(m, 0x22C2E4));
                Some(0)
            } else {
                Some(r)
            }
        }
        0x230C58 if a.len() >= 5 => Some(fn_230c58(m, h, a[2], a[3], a[4])),
        0x230A14 if a.len() >= 5 => Some(fn_230a14(m, h, a[2], a[3], a[4])),
        0x2B0B48 if !a.is_empty() => {
            let mut rh = RouteHost { inner: h };
            Some(tts_core::normalization::alloc_xcoder_flag(
                m.bytes, a[0], &mut rh,
            ))
        }
        0x1E0F10 if a.len() >= 5 => Some(fn_1e0f10(m, h, a[2], a[3], a[4])),
        0x1E0400 if a.len() >= 2 => Some(fn_1e0400(m, h, a[0], a[1])),
        0x1CEDE4 if a.len() >= 5 => {
            let sp = fn_129778(m, h, 8);
            let rc = fn_19611c(m, a[2], a[3], sp);
            if rc >= 0x80000000 {
                Some(rc)
            } else {
                let obj = fn_1963b8(m, h, rd(m, rd(m, sp) + 4), 1, 4);
                if obj == 0 {
                    Some(rd(m, 0x1CE8AC))
                } else {
                    wr(m, obj, rd(m, sp));
                    wr(m, a[4], obj);
                    wr(m, a[4] + 4, rd(m, 0x1CEF70));
                    Some(rc)
                }
            }
        }
        0x217AC8 if a.len() >= 4 => Some(fn_217ac8(m, h, a[0], a[1], a[2], a[3])),
        0x2171BC if a.len() >= 5 => Some(fn_2171bc(m, h, a[0], a[1], a[2], a[3], a[4])),
        0x217E24 if a.len() >= 4 => Some(fn_217e24(m, h, a[0], a[1], a[2], a[3])),
        0x21834C if a.len() >= 5 => Some(fn_21834c(m, h, a[2], a[3], a[4])),
        0x2181B8 if a.len() >= 3 => Some(depes_path(m, h, a[0], a[1], a[2], false)),
        0x217F64 if a.len() >= 3 => Some(depes_path(m, h, a[0], a[1], a[2], true)),
        0x1EC070 if a.len() >= 5 => Some(fn_1ec070(m, h, a)),
        0x1FD9F8 if a.len() >= 5 => {
            let scratch = fn_129778(m, h, 4096);
            let p = pair_176fd4(m);
            let cfg = tts_core::boot_part_a::DomainConfig {
                pair: ((p[1] as u64) << 32 | p[0] as u64).to_le_bytes(),
                default: rd(m, 0x196304),
                cfg: rd(m, 0x1FDCC0),
                names: [
                    rom_text(m, b"LINGDB"),
                    rom_text(m, b"SYNTHSTREAM"),
                    0,
                    0,
                    rom_text(m, b"AUDIOFETCHER"),
                    rom_text(m, b"INET"),
                ],
                category: rd(m, 0x1FD9A8),
                scratch,
            };
            let mut rh = RouteHost { inner: h };
            Some(tts_core::boot_part_a::fn_1fd9f8(
                m,
                a[0],
                a[1],
                [a[2], a[3]],
                a[4],
                &cfg,
                &mut rh,
            ))
        }
        0x201548 if a.len() >= 5 => {
            let sp = fn_129778(m, h, 4096);
            let p = pair_176fd4(m);
            let e = tts_core::boot_port_3::Environment {
                null_pair: ((p[1] as u64) << 32 | p[0] as u64).to_le_bytes(),
                default_context: rd(m, 0x196304),
                inet_name: 0x20166C,
                module: rd(m, 0x201528),
                scratch: sp,
            };
            let mut rh = RouteHost { inner: h };
            Some(tts_core::boot_port_3::fn_201548(
                m, a[0], a[1], a[2], a[3], a[4], &e, &mut rh,
            ))
        }
        0x2028C4 if a.len() >= 2 => {
            if a[0] <= 2 {
                wr(m, a[1], rd(m, 0x2028EC));
                Some(0)
            } else {
                Some(rd(m, 0x201B3C))
            }
        }
        0x22E378 if a.len() >= 5 => {
            let scratch = fn_129778(m, h, 4096);
            let p = pair_176fd4(m);
            let cfgp = rd(m, 0x22E67C);
            let cfg = tts_core::boot_part_a::AudioFetchConfig {
                pair: ((p[1] as u64) << 32 | p[0] as u64).to_le_bytes(),
                default: rd(m, 0x196304),
                callbacks: std::array::from_fn(|j| {
                    std::array::from_fn(|i| {
                        rd(m, cfgp + if j == 0 { 0x74 } else { 0x88 } + i as u32 * 4)
                    })
                }),
                inet: 0x22E354,
                category: rd(m, 0x22E350),
                formats: [0x22E684, 0x22E330, 0x22E35C, 0x22E6A4],
                scratch,
            };
            let mut rh = RouteHost { inner: h };
            Some(tts_core::boot_part_a::fn_22e378(
                m,
                a[0],
                a[1],
                [a[2], a[3]],
                a[4],
                &cfg,
                &mut rh,
            ))
        }
        0x22E220 if a.len() >= 3 => {
            if a[2] == 0 {
                Some(rd(m, 0x22E324))
            } else {
                wr(m, a[2], 0);
                wr(m, a[2] + 4, 0);
                let sp = fn_129778(m, h, 4);
                let rc = fn_19611c(m, a[0], a[1], sp);
                if rc < 0x80000000 {
                    wr(m, a[2], rd(m, sp));
                    wr(m, a[2] + 4, rd(m, 0x22E328));
                }
                Some(rc)
            }
        }
        0x1E67C4 if a.len() >= 2 => {
            if a[0] <= 1 {
                wr(m, a[1], rd(m, 0x1E67E0));
                Some(0)
            } else {
                Some(rd(m, 0x1E67E4))
            }
        }
        0x1E6DD8 if a.len() >= 2 => {
            if a[0] <= 1 {
                wr(m, a[1], rd(m, 0x1E6DF4));
                Some(0)
            } else {
                Some(rd(m, 0x1E6DF8))
            }
        }
        0x1E723C if a.len() >= 2 => {
            if a[0] <= 1 {
                wr(m, a[1], rd(m, 0x1E7258));
                Some(0)
            } else {
                Some(rd(m, 0x1E725C))
            }
        }
        0x200374 if a.len() >= 2 => {
            if a[0] <= 1 {
                wr(m, a[1], rd(m, 0x2007A0));
                Some(0)
            } else {
                Some(rd(m, 0x1FFAA8))
            }
        }
        0x2195C8 if a.len() >= 2 => {
            if a[0] <= 1 {
                wr(m, a[1], rd(m, 0x219784));
                Some(0)
            } else {
                Some(rd(m, 0x219788))
            }
        }
        0x29F4EC if a.len() >= 2 => {
            if a[0] <= 1 {
                wr(m, a[1], rd(m, 0x29F588));
                Some(0)
            } else {
                Some(rd(m, 0x29F58C))
            }
        }
        0x2A0064 if a.len() >= 2 => {
            if a[0] <= 1 {
                wr(m, a[1], rd(m, 0x2A0088));
                Some(0)
            } else {
                Some(rd(m, 0x2A008C))
            }
        }
        0x22C5CC if a.len() >= 2 => Some(tts_core::boot_port_4::fn_22c5cc(
            m,
            a[0],
            a[1],
            rd(m, 0x22C5EC),
        )),
        0x228760 if a.len() >= 2 => Some(tts_core::boot_port_4::fn_228760(
            m,
            a[0],
            a[1],
            rd(m, 0x228778),
        )),
        0x232300 if a.len() >= 2 => Some(tts_core::boot_port_4::fn_232300(
            m,
            a[0],
            a[1],
            rd(m, 0x232348),
        )),
        0x23231C if a.len() >= 2 => Some(tts_core::boot_port_4::fn_23231c(
            m,
            a[0],
            a[1],
            rd(m, 0x232350),
        )),
        0x151774 if a.len() >= 3 => {
            let mut rh = RouteHost { inner: h };
            Some(tts_core::boot_port_1::fn_151774(
                m, a[0], a[1], a[2], 0x1517C8, &mut rh,
            ))
        }
        0x196FBC if a.len() >= 2 => {
            let mut rh = RouteHost { inner: h };
            Some(boot_live::remove_element_ptr_m(m, &mut rh, a[0], a[1]))
        }
        0x1749AC if a.len() >= 2 => {
            let mut hh = HeapCalls { inner: h };
            tts_core::speech_part_1::fn_1749ac(m, a[0], a[1], &mut hh);
            Some(0)
        }
        0x174B14 if !a.is_empty() => {
            let p = rd(m, a[0] + 4);
            if p != 0 {
                route_or_host(m, h, 0x19665C, &[p]);
                wr(m, a[0] + 4, 0);
            }
            Some(0)
        }
        0x174AAC if a.len() >= 3 => {
            wr(m, a[2], rd(m, a[0]));
            let old = rd(m, a[0] + 4);
            let rc = if old != 0 {
                route_or_host(m, h, 0x19658C, &[old, a[2] + 4])
            } else {
                let t = rd(m, 0x175514);
                fn_1964e0(
                    m,
                    h,
                    a[1],
                    0,
                    rd(m, t),
                    rd(m, t + 4),
                    rd(m, t + 8),
                    rd(m, t + 12),
                    rd(m, t + 16),
                    a[2] + 4,
                )
            };
            Some(u32::from(rc < 0x80000000))
        }
        0x175C40 if a.len() >= 2 => Some(fn_175c40(m, h, a[0], a[1])),
        0x175EF0 if a.len() >= 2 => {
            let mut rh = RouteHost { inner: h };
            Some(boot_live::remove_guarded_m(m, &mut rh, a[0], a[1]))
        }
        0x1970A0 if !a.is_empty() => Some(if a[0] == 0 { 0 } else { rd(m, a[0] + 20) }),
        0x158C4C if a.len() >= 4 => Some(boot_live::set_pipeline_m(
            &mut ColdPipeHost { inner: h },
            m.bytes,
            a[0],
            a[1],
            a[2],
            a[3],
        )),
        0x160604 if a.len() >= 5 => Some(fn_160604(m, h, [a[0], a[1], a[2], a[3], a[4]])),
        0x15EEF0 if a.len() >= 6 => Some(fn_15eef0(m, h, [a[0], a[1], a[2], a[3], a[4], a[5]])),
        0x1900F8 if a.len() >= 3 => Some(fn_1900f8(m, h, a[0], a[1], a[2])),
        0x1789B0 if a.len() >= 3 => {
            let sp = fn_129778(m, h, 4096);
            let mut out = rd(m, a[2]);
            let mut rh = RouteHost { inner: h };
            let rc = boot_live::open_component_iter_m(
                m.bytes,
                (a[0] != 0).then_some(a[0]),
                (a[1] != 0).then_some(a[1]),
                Some(&mut out),
                sp,
                &mut rh,
            );
            wr(m, a[2], out);
            Some(rc)
        }
        0x178C24 if a.len() >= 3 => {
            assert_eq!(a[2], 0, "MISSING_178C24_ATTR_KEYS");
            let sp = fn_129778(m, h, 4096);
            let mut key = if a[1] == 0 { 0 } else { rd(m, a[1]) };
            let table = std::array::from_fn(|i| rh(m, rd(m, 0x17C8F8) + i as u32 * 2));
            let mut rh = RouteHost { inner: h };
            let rc = boot_live::next_component_entry_m(
                m.bytes,
                (a[0] != 0).then_some(a[0]),
                if a[1] == 0 { None } else { Some(&mut key) },
                None,
                &table,
                [0; 3],
                sp,
                &mut rh,
            );
            if a[1] != 0 {
                wr(m, a[1], key);
            }
            Some(rc)
        }
        0x1788C0 if !a.is_empty() => {
            let sp = fn_129778(m, h, 4096);
            let mut rh = RouteHost { inner: h };
            Some(boot_live::close_component_iter_m(
                m.bytes,
                (a[0] != 0).then_some(a[0]),
                sp,
                &mut rh,
            ))
        }
        0x1785C8 if a.len() >= 5 => Some(fn_1785c8(m, h, a[0], a[1], a[2], a[3], a[4])),
        0x178818 if a.len() >= 2 => Some(fn_178818(m, h, a[0], a[1])),
        0x1906CC => {
            if a.len() >= 2 {
                Some(fn_1906cc(m, &mut *h, a[0], a[1]))
            } else {
                None
            }
        }
        0x197C78 => {
            if a.len() >= 6 {
                Some(fn_197c78(m, &mut *h, a[0], a[1], a[2], a[3], a[4], a[5]))
            } else {
                None
            }
        }
        0x17A3B8 if a.len() >= 5 => {
            let sp = fn_129778(m, h, 4096);
            let table = std::array::from_fn(|i| rh(m, rd(m, 0x17C8F8) + i as u32 * 2));
            let mut out = rd(m, a[4]);
            let mut rh = RouteHost { inner: h };
            let rc = boot_live::open_tag_iter_m(
                m.bytes,
                (a[0] != 0).then_some(a[0]),
                (a[1] != 0).then_some(a[1]),
                (a[2] != 0).then_some(a[2]),
                a[3],
                Some(&mut out),
                sp,
                &table,
                sp + 128,
                &mut rh,
            );
            wr(m, a[4], out);
            Some(rc)
        }
        0x17A814 if a.len() >= 4 => {
            let mut key = rd(m, a[1]);
            let mut value = rd(m, a[2]);
            let mut attrs = if a[3] == 0 { 0 } else { rd(m, a[3]) };
            let rc = tts_core::frontend::next_tag_entry(
                m.bytes,
                (a[0] != 0).then_some(a[0]),
                Some(&mut key),
                Some(&mut value),
                if a[3] == 0 { None } else { Some(&mut attrs) },
            );
            wr(m, a[1], key);
            wr(m, a[2], value);
            if a[3] != 0 {
                wr(m, a[3], attrs);
            }
            Some(rc)
        }
        0x17A2EC if !a.is_empty() => {
            let sp = fn_129778(m, h, 4096);
            let mut rh = RouteHost { inner: h };
            Some(boot_live::close_tag_iter_m(
                m.bytes,
                (a[0] != 0).then_some(a[0]),
                sp,
                &mut rh,
            ))
        }
        0x2324D0 if a.len() >= 5 => Some(fn_2324d0(m, h, a[0], a[1], a[2], a[3], a[4])),
        0x2325E4 if !a.is_empty() => {
            if a[0] == 0 {
                Some(rd(m, 0x2325B8))
            } else {
                fn_196428(m, h, rd(m, rd(m, a[0]) + 4), a[0]);
                Some(0)
            }
        }
        0x177DD8 if a.len() >= 2 => {
            if a[0] == 0 || a[1] == 0 {
                if a[0] == 0 && a[1] != 0 {
                    wr(m, a[1], 0);
                }
                Some(rd(m, 0x177274))
            } else {
                wr(m, a[1], rd(m, a[0] + 4));
                Some(0)
            }
        }
        0x196770 if !a.is_empty() => {
            if a[0] == 0 {
                Some(rd(m, 0x196A54))
            } else {
                fn_19869c(m, h, a[0] + 24);
                Some(0)
            }
        }
        0x173DC0 if a.len() >= 4 => {
            let sp = fn_129778(m, h, 512);
            let va = a.get(4).copied().unwrap_or(sp + 256);
            Some(tts_core::cold_boot::fn_173dc0(
                m, h, a[0], a[1], a[2], a[3], va, sp,
            ))
        }
        0x1738E8 if a.len() >= 5 => {
            let va = if a.len() >= 6 {
                a[5]
            } else {
                fn_129778(m, h, 32)
            };
            let mut rh = RouteHost { inner: h };
            Some(tts_core::log_live::fn_1738e8(
                m, a[0], a[1], a[2], a[3], a[4], va, &mut rh,
            ))
        }
        0x176AC8 | 0x176C58 | 0x195B38 | 0x195E38 if a.len() >= 3 => m.boot_round10_call(f, a),
        0x195C90 if a.len() >= 2 => m.boot_round10_call(f, a),
        0x15ACCC if !a.is_empty() => {
            let sp = fn_129778(m, h, 112);
            let rc = fn_1756d0(m, h, a[0], 0x15ADD8, sp + 100, 0);
            let src = rd(m, sp + 100);
            if rc >= 0x80000000 || src == 0 {
                Some(0)
            } else {
                fn_3023b0(m, sp, 100);
                let n = fn_195d90(m, src);
                let kn = fn_195d90(m, 0x15ADFC);
                assert!(n <= 100 && kn != 0);
                for i in 0..n {
                    wb(m, sp + i, rb(m, src + i) ^ rb(m, 0x15ADFC + i % kn));
                }
                Some(u32::from(fn_1769d0(m, sp, 0x15AE00) == 0))
            }
        }
        0x160AA8 if a.len() >= 2 => Some(fn_160aa8(m, a[0], a[1])),
        0x15174C if a.len() >= 2 => Some(fn_15174c(m, a[0], a[1])),
        0x15DB5C if a.len() >= 2 => Some(fn_15db5c(m, a[0], a[1])),
        0x174988 if a.len() >= 2 => Some(fn_174988(m, a[0], a[1])),
        0x1529B4 if a.len() >= 2 => Some(fn_1529b4(m, h, a[0], a[1])),
        0x152D48 if a.len() >= 4 => Some(fn_152d48(m, h, a[0], a[1], a[2], a[3])),
        0x151840 if a.len() >= 4 => {
            let table = core::array::from_fn(|i| rh(m, 0x8dc30c + i as u32 * 2));
            let pair = core::array::from_fn(|i| rb(m, 0x8d7660 + i as u32));
            let mut valid = 0;
            let rc = tts_core::frontend::validate_param(
                m.bytes,
                &table,
                (a[0] != 0).then_some(a[0]),
                &m.bytes[a[1] as usize..],
                (a[2] != 0).then_some(a[2]),
                &mut valid,
                &pair,
                0x8d7640,
            );
            wb(m, a[3], valid);
            Some(rc)
        }
        0x151DF8 if a.len() >= 3 => {
            let (p, name, value) = (a[0], a[1], a[2]);
            if p == 0 || value == 0 || rb(m, value) == 0 {
                return Some(0);
            }
            let sp = fn_129778(m, h, 20);
            wr(m, sp + 12, 0);
            let rc = fn_19611c(m, rd(m, p), rd(m, p + 4), sp + 12);
            if rc >= 0x80000000 {
                return Some(0);
            }
            wr(m, sp, 0x151f0c);
            wr(m, sp + 4, name);
            wr(m, sp + 8, value);
            route_or_host(
                m,
                h,
                0x1738E8,
                &[
                    rd(m, rd(m, sp + 12) + 16),
                    rd(m, 0x151b80),
                    4,
                    0,
                    0x151f0c,
                    sp + 4,
                ],
            );
            for (key, off) in [(0x151f24, 0x38), (0x151f3c, 0x34), (0x151f48, 0xe8)] {
                if fn_195d58(m, name, key) == 0 {
                    let v = route_or_host(m, h, 0x176CD8, &[value]);
                    wr(m, p + off, v);
                    return Some(0);
                }
            }
            Some(if fn_195d58(m, name, 0x151f60) == 0 {
                route_or_host(m, h, 0x15AF9C, &[rd(m, sp + 12), p, value])
            } else {
                0
            })
        }
        0x151F80 if a.len() >= 4 => {
            fn_151f80(m, h, a[0], a[1], a[2], a[3]);
            Some(a[0])
        }
        0x17EAF4 if a.len() >= 4 => {
            if a[3] != 0 {
                wr(m, a[3], a[2]);
            }
            Some(0)
        }
        0x17EB74 => Some(rd(m, rd(m, 0x17EB88))),
        0x17EB3C if !a.is_empty() => {
            let p = rd(m, 0x17EB88);
            let v = rd(m, p);
            wr(m, p, v.wrapping_add(1));
            Some(a[0].wrapping_mul(rd(m, 0x17EB8C)))
        }
        0x173FB4 if a.len() >= 4 => {
            let sp = fn_129778(m, h, 32);
            let secure = rom_text(m, b"SECURE");
            let mut host = RouteHost { inner: h };
            tts_core::boot_part_b::fn_173fb4(m, a[0], a[1], a[2], a[3], sp, secure, &mut host);
            Some(a[0])
        }
        0x17EB90 if a.len() >= 3 => {
            let sp = fn_129778(m, h, 4);
            let mut rh = RouteHost { inner: h };
            Some(boot_live::license_token_m(m, &mut rh, a[0], a[1], a[2], sp))
        }
        0x1CE428 if a.len() >= 6 => {
            if a[0] == 0 || a[1] == 0 {
                Some(rd(m, 0x1CDFC0))
            } else if rb(m, rd(m, a[0]) + 4) == 0 {
                Some(0)
            } else {
                panic!("unbound active file logger at 0x1ce428");
            }
        }
        0x18F3FC if a.len() >= 6 => {
            route_or_host(
                m,
                h,
                rd(m, a[0]),
                &[rd(m, a[0] + 8), a[2], a[3], a[4], a[5]],
            );
            Some(0)
        }
        0x18F430 if a.len() >= 6 => {
            let buf = fn_129778(m, h, 0x800);
            m.boot_vsnprintf(buf, 0x800, a[4], a[5]);
            let target = rd(m, a[0] + 4);
            Some(route_or_host(m, h, target, &[target, a[2], buf]))
        }
        0x4A7ADC => Some(a.first().copied().unwrap_or(0)),
        0x18F488 => Some(tts_core::frontend::finish_request()),
        0x1CE56C if !a.is_empty() => {
            Some(tts_core::frontend::streaming_mode((a[0] != 0).then(|| {
                let p = rd(m, a[0]);
                (rb(m, p + 4), rb(m, p + 0x10C))
            })))
        }
        0x1CE1BC if a.len() >= 3 => {
            let rc = fn_176f94(m, a[0], a[1], rd(m, 0x1CDFB8), 0x198);
            if rc >= 0x80000000 {
                Some((rc & 0x7fffffff) | rd(m, 0x1CDFBC))
            } else if a[2] == 0 {
                Some(rd(m, 0x1CDFC0))
            } else {
                wr(m, a[2], 0);
                let p = fn_1963b8(m, h, rd(m, rd(m, a[0]) + 4), 1, 4);
                if p == 0 {
                    Some(rd(m, 0x1CDFC8))
                } else {
                    wr(m, p, a[0]);
                    wr(m, a[2], p);
                    Some(rc)
                }
            }
        }
        0x1CDB80 if a.len() >= 3 => Some(fn_1cdb80(m, h, a[0], a[1], a[2])),
        0x17C550 if !a.is_empty() => Some(parse_decimal(&m.bytes[a[0] as usize..]) as u32),
        0x176CD8 if !a.is_empty() => {
            Some(tts_core::frontend::parse_uint(m.bytes, (a[0] != 0).then_some(a[0])) as u32)
        }
        0x1CE58C if a.len() >= 2 => {
            if a[0] <= 1 {
                wr(m, a[1], rd(m, 0x1CE5A8));
                Some(0)
            } else {
                Some(rd(m, 0x1CE5AC))
            }
        }
        0x198414 if !a.is_empty() => Some(tts_core::boot_part_b::fn_198414(m, a[0])),
        0x17F614 if a.len() >= 8 => {
            let env = pipeline_environment(m, h);
            let mut saved = vec![0; tts_core::boot_port_1b::SCRATCH_BYTES as usize];
            let mut ph = PipelineHost { inner: h };
            Some(tts_core::boot_port_1b::fn_17f614(
                m,
                a[..8].try_into().unwrap(),
                &env,
                &mut saved,
                &mut ph,
            ))
        }
        0x197630 => {
            if a.len() >= 2 {
                Some(fn_197630(m, &mut *h, a[0], a[1]))
            } else {
                None
            }
        }
        0x1756D0 if a.len() >= 4 => Some(fn_1756d0(m, h, a[0], a[1], a[2], a[3])),
        0x175170 if a.len() >= 4 => {
            let sp = fn_129778(m, h, 4096);
            let mut hh = HeapCalls { inner: h };
            Some(tts_core::speech_part_1::fn_175170(
                m,
                a[0],
                a[1],
                a[2],
                a[3],
                a.get(4).copied().unwrap_or(0),
                sp,
                &mut hh,
            ))
        }
        0x1759A0 if a.len() >= 3 => {
            let sp = fn_129778(m, h, 4168);
            Some(fn_1759a0(m, h, a[0], a[1], a[2], sp))
        }
        0x1757A8 => {
            if a.len() >= 1 {
                Some(fn_1757a8(m, a[0]))
            } else {
                None
            }
        }
        0x1758AC => {
            if a.len() >= 3 {
                Some(fn_1758ac(m, &mut *h, a[0], a[1], a[2]))
            } else {
                None
            }
        }
        0x175944 => {
            if a.len() >= 3 {
                Some(fn_175944(m, &mut *h, a[0], a[1], a[2]))
            } else {
                None
            }
        }
        0x173FD8 => {
            if a.len() >= 1 {
                Some(fn_173fd8(m, &mut *h, a[0]))
            } else {
                None
            }
        }
        0x17B1C4 => {
            if a.len() >= 2 {
                Some(fn_17b1c4(m, &mut *h, a[0], a[1]))
            } else {
                None
            }
        }
        0x175A1C => {
            if a.len() >= 3 {
                Some(fn_1756d0(m, &mut *h, a[0], a[1], a[2], 0))
            } else {
                None
            }
        }
        0x15AF9C => {
            if a.len() >= 3 {
                Some(fn_15af9c(m, &mut *h, a[0], a[1], a[2]))
            } else {
                None
            }
        }
        0x15B128 => {
            if a.len() >= 3 {
                Some(fn_15b128(m, &mut *h, a[0], a[1], a[2]))
            } else {
                None
            }
        }
        0x19611C => {
            if a.len() >= 3 {
                Some(fn_19611c(m, a[0], a[1], a[2]))
            } else {
                None
            }
        }
        0x15BC18 => {
            if a.len() >= 5 {
                Some(fn_15bc18(m, &mut *h, a[0], a[1], a[2], a[3], a[4]))
            } else {
                None
            }
        }
        0x39F5B8 => {
            if a.len() >= 2 {
                Some(fn_39f5b8(m, &mut *h, a[0], a[1], 0, 0))
            } else {
                None
            }
        }
        0x196428 => {
            if a.len() >= 2 {
                Some(fn_196428(m, &mut *h, a[0], a[1]))
            } else {
                None
            }
        }
        0x15AE20 => {
            if a.len() >= 4 {
                Some(fn_15ae20(m, &mut *h, a[0], a[1], a[2], a[3]))
            } else {
                None
            }
        }
        0x151914 => {
            if a.len() >= 9 {
                Some(fn_151914(
                    m, &mut *h, a[0], a[1], a[2], a[3], a[4], a[5], a[6], a[7], a[8],
                ))
            } else {
                None
            }
        }
        0x157A80 => {
            if a.len() >= 8 {}

            if a.len() >= 8 {
                Some(fn_157a80(
                    m, &mut *h, a[0], a[1], a[2], a[3], a[4], a[5], a[6], a[7],
                ))
            } else {
                None
            }
        }
        0x196980 => {
            if a.len() >= 2 {
                Some(tts_core::speech_part_1::fn_196980(m, a[0], a[1]))
            } else {
                None
            }
        }
        0x1770F8 => {
            if a.len() >= 2 {
                Some(fn_1770f8(m, a[0], a[1]))
            } else {
                None
            }
        }
        0x177038 if a.len() >= 3 => Some(tts_core::boot_port_1::fn_177038(m, a[0], a[1], a[2])),
        0x177028 => {
            if a.len() >= 3 {
                Some(tts_core::boot_port_1::fn_177028(m, a[0], a[1], a[2]))
            } else {
                None
            }
        }
        0x177080 => {
            if a.len() >= 3 {
                let mut rh = RouteHost { inner: &mut *h };
                Some(tts_core::boot_port_1::fn_177080(
                    m, a[0], a[1], a[2], &mut rh,
                ))
            } else {
                None
            }
        }
        0x1773EC => {
            if a.len() >= 3 {
                let mut rh = RouteHost { inner: &mut *h };
                Some(tts_core::boot_port_1::fn_1773ec(
                    m,
                    a[0],
                    a[1],
                    a[2],
                    pair_176fd4(m),
                    &mut rh,
                ))
            } else {
                None
            }
        }
        0x19690C => {
            if a.len() >= 3 {
                Some(fn_19690c(m, &mut *h, a[0], a[1], a[2]))
            } else {
                None
            }
        }
        0x19719C => {
            if a.len() >= 3 {
                Some(fn_19719c(m, &mut *h, a[0], a[1], a[2]))
            } else {
                None
            }
        }
        0x13098C => {
            if a.is_empty() {
                None
            } else {
                if a[0] != 0 {
                    wr(m, a[0], 0);
                }
                Some(0)
            }
        }
        0x157080 => {
            if a.len() >= 3 {
                Some(fn_157080(m, &mut *h, a[0], a[1], a[2]))
            } else {
                None
            }
        }
        0x18F24C => {
            if a.len() >= 5 {
                Some(fn_18f24c(m, &mut *h, a[0], a[1], a[2], a[3], a[4]))
            } else {
                None
            }
        }
        0x191E40 => {
            if a.len() >= 3 {
                Some(fn_191e40(m, &mut *h, a[0], a[1], a[2]))
            } else {
                None
            }
        }
        0x177E0C => {
            if a.len() >= 6 {
                Some(fn_177e0c(m, &mut *h, a[0], a[1], a[2], a[3], a[4], a[5]))
            } else {
                None
            }
        }
        0x157A54 => {
            if a.len() >= 2 {
                Some(fn_157a54(m, a[0], a[1]))
            } else {
                None
            }
        }
        0x14D764 => {
            if a.len() >= 4 {
                Some(fn_14d764(m, &mut *h, a[0], a[1], a[2], a[3]))
            } else {
                None
            }
        }
        0x196764 => {
            if a.len() >= 1 {
                Some(fn_196764(m, a[0]))
            } else {
                None
            }
        }
        0x19665C => {
            if a.len() >= 1 {
                Some(fn_19665c(m, &mut *h, a[0]))
            } else {
                None
            }
        }
        0x176EDC => {
            if a.len() >= 1 {
                Some(fn_176edc(m, &mut *h, a[0]))
            } else {
                None
            }
        }
        0x174E74 => {
            if a.len() >= 5 {
                Some(fn_174e74(m, &mut *h, a[0], a[1], a[2], a[3], a[4]))
            } else {
                None
            }
        }
        0x172EAC => {
            if a.len() >= 5 {
                Some(fn_172eac(m, &mut *h, a[0], a[1], a[2], a[3], a[4]))
            } else {
                None
            }
        }
        0x17DACC => {
            if a.len() >= 4 {
                Some(fn_17dacc(m, &mut *h, a[0], a[1], a[2], a[3]))
            } else {
                None
            }
        }
        0x1772B0 => {
            if a.len() >= 3 {
                Some(fn_1772b0(m, &mut *h, a[0], a[1], a[2]))
            } else {
                None
            }
        }
        0x17C8C0 => {
            if a.len() >= 1 {
                Some(fn_17c8c0(m, a[0]))
            } else {
                None
            }
        }
        0x1769D0 => {
            if a.len() >= 2 {
                Some(fn_1769d0(m, a[0], a[1]))
            } else {
                None
            }
        }
        0x198F74 => {
            if a.len() >= 1 {
                Some(fn_198f74(m, a[0]))
            } else {
                None
            }
        }
        0x198A80 => {
            if a.len() >= 2 {
                Some(fn_198a80(m, a[0], a[1]))
            } else {
                None
            }
        }
        0x198760 => {
            if a.len() >= 2 {
                Some(fn_198760(m, &mut *h, a[0], a[1]))
            } else {
                None
            }
        }
        0x1987FC => {
            if a.len() >= 2 {
                Some(fn_1987fc(m, &mut *h, a[0], a[1]))
            } else {
                None
            }
        }
        0x196794 => {
            if a.len() >= 4 {
                Some(fn_196794(m, &mut *h, a[0], a[1], a[2], a[3]))
            } else {
                None
            }
        }
        0x196860 => {
            if a.len() >= 3 {
                Some(fn_196860(m, &mut *h, a[0], a[1], a[2]))
            } else {
                None
            }
        }
        0x196830 => {
            if a.len() >= 1 {
                Some(fn_196830(m, &mut *h, a[0]))
            } else {
                None
            }
        }
        0x4A79B8 => {
            if a.len() >= 3 {
                Some(fn_4a79b8(m, &mut *h, a[0], a[1], a[2]))
            } else {
                None
            }
        }
        0x4A774C => {
            if a.len() >= 4 {
                Some(fn_4a774c(m, &mut *h, a[0], a[1], a[2], a[3]))
            } else if a.len() >= 2 {
                Some(fn_4a774c(m, &mut *h, a[0], a[1], 0, 0))
            } else {
                None
            }
        }
        0x19869C => {
            if a.len() >= 1 {
                Some(fn_19869c(m, &mut *h, a[0]))
            } else {
                None
            }
        }
        0x17AD10 => {
            if a.len() >= 2 {
                Some(fn_17ad10(m, &mut *h, a[0], a[1]))
            } else {
                None
            }
        }
        0x19600C => {
            if a.len() >= 2 {
                Some(fn_19600c(m, a[0], a[1]))
            } else {
                None
            }
        }
        0x177124 => {
            if a.len() >= 3 {
                Some(fn_177124(m, &mut *h, a[0], a[1], a[2]))
            } else {
                None
            }
        }
        0x17D168 => {
            if a.len() >= 3 {
                Some(fn_17d168(m, &mut *h, a[0], a[1], a[2]))
            } else {
                None
            }
        }
        0x15F164 => {
            if a.len() >= 5 {
                Some(fn_15f164(m, &mut *h, a[0], a[1], a[2], a[3], a[4]))
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
            tts_core::boot_part_b::fn_1840fc(m, a[0], &mut rh);
            Some(0)
        }
        0x18423C => {
            let mut rh = RouteHost { inner: h };
            tts_core::boot_part_b::fn_18423c(m, a[0], &mut rh);
            Some(0)
        }
        0x183EDC => {
            if a.is_empty() {
                return None;
            }
            let p = a[0];
            if p != 0 {
                let v = rd(m, p) as i32;
                if v > 0 {
                    wr(m, p, v.wrapping_neg() as u32);
                } else {
                    wr(m, p, 0xFFFF_FFFF);
                }
            }
            Some(0)
        }
        0x165D00 => {
            let g = tts_core::boot::Globals {
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
                Some(fn_191948(m, &mut *h, a[0], a[1], a[2], scratch))
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
                Some(fn_19310c(m, &mut *h, a[0], a[1], a[2], a[3], scratch))
            } else {
                None
            }
        }
        0x1758F8 => {
            if a.len() >= 3 {
                Some(fn_1758f8(m, &mut *h, a[0], a[1], a[2]))
            } else {
                None
            }
        }
        0x1759E0 => {
            if a.len() >= 3 {
                Some(fn_1759e0(m, &mut *h, a[0], a[1], a[2]))
            } else {
                None
            }
        }
        0x176F94 => {
            if a.len() >= 4 {
                Some(fn_176f94(m, a[0], a[1], a[2], a[3]))
            } else {
                None
            }
        }
        0x195DF8 => {
            if a.len() >= 3 {
                Some(fn_195df8(m, a[0], a[1], a[2]))
            } else {
                None
            }
        }
        0x1517E8 => {
            if a.len() >= 4 {
                Some(fn_1517e8(m, &mut *h, a[0], a[1], a[2], a[3]))
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
                Some(fn_15af34(m, &mut *h, a[0], a[1], a[2], scratch))
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
                Some(fn_17ef1c(m, &mut *h, a[0], a[1], a[2], scratch))
            } else {
                None
            }
        }
        0x17F03C => {
            if a.len() >= 9 {
                Some(fn_17f03c(
                    m, &mut *h, a[0], a[1], a[2], a[3], a[4], a[5], a[6], a[7], a[8],
                ))
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
                Some(fn_194454(
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

        0x163D34 => {
            if a.len() >= 5 {
                let mut rh = RouteHost { inner: h };
                let mode = rb(m, FILL_FLAG) as u32;
                let six = [a[0], a[1], a[2], a[3], a[4], mode];
                let r = tts_core::boot_part_b2::fn_183524(
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
        0x19AF64 => {
            if a.len() >= 1 {
                Some(fn_19af64(m, a[0]))
            } else {
                None
            }
        }
        0x19AE08 => {
            if a.len() >= 2 {
                fn_19ae08(m, a[0], a[1]);
                Some(0)
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
                tts_core::boot_part_b2::fn_19b78c(m, a[0], a[1], &mut rh);
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
                let env = tts_core::speech_part_1::ArchiveEnvironment {
                    dot: BOOT_DOT,
                    parent: BOOT_PARENT,
                    locale: 0,
                };
                Some(tts_core::boot_part_a::fn_16b554_m(
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
            tts_core::boot_part_b::fn_1840fc(m, mgr.wrapping_add(32), &mut rh);
            wb(m, slot.wrapping_add(0xe4), 2);
            tts_core::boot_part_b::fn_18423c(m, mgr.wrapping_add(32), &mut rh);
            let negative = rb(m, slot.wrapping_add(0xe6)) != 0;
            let allocator = rd(m, slot.wrapping_add(if negative { 0xdc } else { 0xd8 }));
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
        _ => m.boot_round10_call(f, a),
    }
}

pub struct Rt<'a, H: Calls> {
    pub inner: &'a mut H,
}

impl<H: Calls> Calls for Rt<'_, H> {
    fn real_log_args(&mut self) -> bool {
        self.inner.real_log_args()
    }
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
        m.bytes[addr.wrapping_sub(m.base) as usize..][..4]
            .try_into()
            .unwrap(),
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
        tts_core::math::remainderf(a, 65536.0)
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
        tts_core::math::remainderf(a, 65536.0)
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

pub fn fn_1c1824(m: &mut Memory<'_>, obj: u32, q: u32, kind: u32, arg: i32, x: f32, y: f32) -> u32 {
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
            route_or_host(m, &mut *h, 0x11DA2C, &[q]);
            wr(m, p + off, 0);
        }
    }
    let f44 = rd(m, p + 0x44);
    if f44 != 0 {
        let r = route_or_host(m, &mut *h, 0x7414C4, &[f44]);
        route_or_host(m, &mut *h, 0x302B1C, &[r]);
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

pub fn fn_741994(
    m: &mut Memory<'_>,
    h: &mut impl Calls,
    p: u32,
    a: i32,
    b: i32,
    c: i32,
    d: f32,
) -> u32 {
    if rb(m, p + 4) != 0 {
        fn_741d64(m, h, p);
    }
    wf(m, p + 8, d);
    wf(m, p + 28, 0.125);
    wf(m, p + 24, 24.0);
    wr(m, p + 12, (d * 0.005) as i32 as u32);
    let q0 = fn_129778(m, h, 52);
    let q = if q0 != 0 {
        fn_741424(
            m,
            h,
            q0,
            (d / c as f32) as i32 as u32,
            (d / b as f32) as i32 as u32,
        )
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
        let v = 0.5
            - fn_19c674(
                m,
                i as f32 * 6.2831853 / (rd(m, p + 40).wrapping_sub(1)) as f32 * 40.743664,
            ) * 0.5;
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
    let f64 = (rd(m, p + 16) * 2) as f32
        + a as f32 * 24.0
        + (1.0 + rd(m, qq + 4) as f32 * 8.0) as i32 as f32;
    let f64i = f64 as i32 as u32;
    wr(m, p + 64, f64i);
    let f60 = fn_129760(m, h, f64i.wrapping_mul(4));
    wr(m, p + 60, f60);
    wb(m, p + 4, 1);
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
    route_or_host(m, &mut *h, rd(m, iface + 12), &[ctx, node]);
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
    tts_core::boot_part_b::fn_173c70(m, a, b, c, d, va, params, &mut rh)
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
    if rb(m, log + 0x40) == 0 && typ != 0 && rd(m, typ + 0x30) != 0 && rd(m, typ + 0x34) != 0 {
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
    let n = tts_core::boot_port_1::fn_195d90(m, str_ptr).wrapping_add(1);
    let work = if a == 0 {
        scratch + 8
    } else {
        fn_129778(m, h, 4096)
    };
    let mut hh = HeapCalls { inner: &mut *h };
    tts_core::speech_part_1::fn_175170(m, a, b, str_ptr, n, 0, work, &mut hh)
}

pub fn fn_1757b8(
    m: &mut Memory<'_>,
    h: &mut impl Calls,
    p: u32,
    key: u32,
    val: u32,
    scratch: u32,
) -> u32 {
    tts_core::boot_port_1::fn_176c58(m, val, scratch, 10);
    let n = tts_core::boot_port_1::fn_195d90(m, scratch).wrapping_add(1);
    let work = if p == 0 {
        scratch + 40
    } else {
        fn_129778(m, h, 4096)
    };
    let mut hh = HeapCalls { inner: &mut *h };
    tts_core::speech_part_1::fn_175170(m, p, key, scratch, n, 0, work, &mut hh)
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
            route_or_host(m, &mut *h, 0x196428, &[alloc, n]);
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
    let logger = rd(m, context + 16);
    let heap = rd(m, context + 4);

    let mut rh = RouteHost { inner: h };
    if ident != 0 {
        wr(m, scratch, ident);
        tts_core::log_live::fn_1738e8(
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
        tts_core::log_live::fn_1738e8(
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
        fn_173dc0(
            m,
            &mut *h,
            logger,
            BROKER_MODULE_NAME,
            200,
            0,
            scratch,
            scratch + 4,
        );
        return 0x8150600A;
    }
    wr(m, object, logger);
    wr(m, object + 4, broker);
    wr(m, out, object);
    let mut rh = RouteHost { inner: h };
    tts_core::log_live::fn_1738e8(
        m,
        logger,
        BROKER_MODULE_NAME,
        4,
        0,
        FMT_BRK_END,
        scratch,
        &mut rh,
    );
    0
}

pub fn fn_184f20(m: &mut Memory<'_>, p: u32, a: u32, b: u32, c: u32, d: u32, e: u32) -> u32 {
    let sub = p.wrapping_add(0x18);
    {
        let mut walk = String::new();
        let mut blk = rd(m, sub.wrapping_add(0x24));
        for _ in 0..12 {
            if blk == 0 {
                break;
            }
            walk.push_str(&format!(" [{:#x}+{:#x}]", blk, rd(m, blk.wrapping_add(4))));
            blk = rd(m, blk.wrapping_add(12));
        }
    }
    tts_core::boot_port_1::fn_1852d8(m, sub, c, 0);
    tts_core::boot_port_1::fn_1852e8(m, sub, d, 0);
    tts_core::boot_port_1::fn_1853c4(m, sub, e, 0);
    let r = tts_core::boot_port_1::fn_1850c0(m, sub, a, b as i32);
    {
        let mut walk = String::new();
        let mut blk = rd(m, sub.wrapping_add(0x24));
        for _ in 0..12 {
            if blk == 0 {
                break;
            }
            walk.push_str(&format!(" [{:#x}+{:#x}]", blk, rd(m, blk.wrapping_add(4))));
            blk = rd(m, blk.wrapping_add(12));
        }
    }
    if r != 0 {
        wr(m, p + 84, rd(m, p + 84).wrapping_add(1));
    }
    r
}

pub fn pair_176fd4(m: &Memory<'_>) -> [u32; 2] {
    let pp = rd(m, 0x176FE8);
    [rd(m, pp), rd(m, pp.wrapping_add(4))]
}

pub fn fn_176fc0_eq(buf: u32, a: u32, p0: u32, p1: u32) -> u32 {
    if a == p1 && buf == p0 {
        1
    } else {
        0
    }
}
pub fn fn_19611c(m: &mut Memory<'_>, buf: u32, a: u32, out: u32) -> u32 {
    let p = pair_176fd4(m);
    let r8 = rd(m, 0x196304);

    if fn_176fc0_eq(buf, a, p[0], p[1]) != 0 {
        wr(m, out, r8);
        return 0;
    }
    let sb = rd(m, 0x196308);
    let r5 = fn_176f94(m, buf, a, sb, 0x20);
    let r0load = rd(m, 0x19630C);

    if r5 < 0x80000000 {
        wr(m, out, buf);
        if buf != 0 && rh(m, buf) as u32 == sb {
            return r5;
        }
        wr(m, out, r8);
        return r0load;
    }
    wr(m, out, r8);
    if r5 == 0x400000 {
        return r0load;
    }
    r5
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

pub fn fn_190984(m: &mut Memory<'_>, h: &mut impl Calls, req: u32, out: u32, scratch: u32) -> u32 {
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
        wr(
            m,
            worker + W_PROVIDER + [0, 4, 8, 12, 16, 24][i as usize],
            rd(m, resource + src_off),
        );
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
    status = fn_196318(
        m,
        h,
        worker + W_ALLOCATOR,
        rd(m, worker + W_USER),
        scratch + 8,
    );
    if status >= 0x80000000 {
        return cleanup_190984(m, h, req, out, worker, scratch, status);
    }
    {
        let mut hh = HeapCalls { inner: &mut *h };
        status = tts_core::boot_part_b::fn_176dac(
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
        status = tts_core::boot_part_b::fn_172e20(
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
        status = m.fn_1961d0(&mut rh, [engine_id, resource_id, 0, 0, worker + W_ENGINE]);
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
        status = fn_14de40(
            m,
            h,
            rd(m, context + 16),
            worker + W_CALLBACK,
            rd(m, 0x190EA0),
        );
        if status >= 0x80000000 {
            return cleanup_190984(m, h, req, out, worker, scratch, status);
        }
    }
    status = fn_174988(m, 1, scratch);
    if status < 0x80000000 {
        let mut hh = HeapCalls { inner: &mut *h };
        let _ = tts_core::boot_part_b::fn_196448(
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

    let vpp = rd(m, 0x176FE8);
    wr(m, scratch + 0x18, rd(m, vpp));
    wr(m, scratch + 0x1C, rd(m, vpp.wrapping_add(4)));
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
        wr(m, out + 4, rd(m, 0x190E8C));
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
        route_or_host(m, &mut *h, 0x1906CC, &[worker, 0]);
        return fn_160fec(status);
    }
    let provider_id = rd(m, scratch + 16);
    if provider_id != 0 {
        route_or_host(m, &mut *h, 0x172E84, &[provider_id]);
    }
    if rd(m, scratch + 12) != 0 {
        route_or_host(m, &mut *h, 0x176E10, &[scratch + 12]);
    }
    let engine_id = rd(m, scratch + 8);
    if engine_id != 0 {
        fn_196368(m, h, engine_id);
    }
    if worker != 0 {
        let iface = rd(m, req + 8);
        route_or_host(m, &mut *h, rd(m, iface + 12), &[rd(m, req + 12), worker]);
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
    let _s14 = rd(m, pattern.wrapping_add(0x14));

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
        tts_core::log_live::fn_1738e8(
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
        let n = tts_core::boot_port_1::fn_195d90(m, s7).wrapping_add(1);
        let cp = fn_1963b8(m, &mut *h, sb, 1, n);
        if cp != 0 {
            fn_195ae8(m, cp, s7, n);
        }
        wr(m, r4.wrapping_add(8), cp);
        if cp == 0 {
            route_or_host(m, &mut *h, 0x17AD10, &[sb, r4]);
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
            route_or_host(m, &mut *h, 0x17AD10, &[sb, r4]);
            return teardown_177624(m, h, ctx, sl, rd(m, 0x177260), scratch);
        }
        wr(m, r4.wrapping_add(0x10), arr);
        let mut idx = 0u32;
        while rh(m, pattern.wrapping_add(0xC)) as u32 > idx {
            let s = rd(
                m,
                rd(m, pattern.wrapping_add(0x10)).wrapping_add(idx.wrapping_mul(4)),
            );
            let mut r7 = 0u32;
            if s != 0 {
                let n = tts_core::boot_port_1::fn_195d90(m, s).wrapping_add(1);
                r7 = fn_1963b8(m, &mut *h, sb, 1, n);
                if r7 != 0 {
                    fn_195ae8(m, r7, s, n);
                }
            }
            wr(m, arr.wrapping_add(idx.wrapping_mul(4)), r7);
            if r7 == 0 {
                let done_n = rh(m, r4.wrapping_add(0xC)) as u32;
                for i in 0..done_n {
                    route_or_host(
                        m,
                        &mut *h,
                        0x196428,
                        &[sb, rd(m, arr.wrapping_add(i.wrapping_mul(4)))],
                    );
                }
                route_or_host(m, &mut *h, 0x196428, &[sb, arr]);
                wr(m, r4.wrapping_add(0x10), 0);
                route_or_host(m, &mut *h, 0x196428, &[sb, rd(m, r4.wrapping_add(8))]);
                route_or_host(m, &mut *h, 0x196428, &[sb, r4]);
                return teardown_177624(m, h, ctx, sl, rd(m, 0x177260), scratch);
            }
            wh(
                m,
                r4.wrapping_add(0xC),
                rh(m, r4.wrapping_add(0xC)).wrapping_add(1),
            );
            idx += 1;
        }
    }

    let s14 = rd(m, pattern.wrapping_add(0x14));
    if s14 == 0 {
        wr(m, r4.wrapping_add(0x14), 0);
    } else {
        let n = tts_core::boot_port_1::fn_195d90(m, s14).wrapping_add(1);
        let cp = fn_1963b8(m, &mut *h, sb, 1, n);
        if cp != 0 {
            fn_195ae8(m, cp, s14, n);
        }
        wr(m, r4.wrapping_add(0x14), cp);
        if cp == 0 {
            route_or_host(m, &mut *h, 0x17AD10, &[sb, r4]);
            return teardown_177624(m, h, ctx, sl, rd(m, 0x177260), scratch);
        }
    }
    wr(m, sl.wrapping_add(0x18), r4);

    let r = fn_176e40(
        m,
        &mut *h,
        rd(m, ctx.wrapping_add(8)),
        heap,
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
        heap,
        sl.wrapping_add(0x1C),
    );
    if r >= 0x80000000 {
        return teardown_177624(m, h, ctx, sl, r, scratch);
    }

    let r = if t == 2 {
        route_or_host(m, &mut *h, 0x17E06C, &[sl, pattern])
    } else {
        let rr = route_or_host(
            m,
            &mut *h,
            0x17D168,
            &[ctx, sl, rd(m, pattern.wrapping_add(0x14))],
        );

        rr
    };
    if r < 0x80000000 {
        wr(m, out, sl);
        epi_log_177624(m, h, ctx, scratch);
        return r;
    }
    teardown_177624(m, h, ctx, sl, r, scratch)
}

fn epi_log_177624(m: &mut Memory<'_>, h: &mut impl Calls, ctx: u32, scratch: u32) {
    let logger = if ctx == 0 {
        0
    } else {
        rd(m, ctx.wrapping_add(0x10))
    };
    let mut rhh = RouteHost { inner: &mut *h };
    let _ = tts_core::log_live::fn_1738e8(m, logger, 0, 4, 0, 0, scratch.wrapping_add(4), &mut rhh);
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

    fn_198728(m, obj.wrapping_add(0x18), obj.wrapping_add(8), obj, heap);
    if fp == 1 {
        route_or_host(m, &mut *h, 0x198FA8, &[obj.wrapping_add(0x18)]);
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
        tts_core::log_live::fn_1738e8(
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
    let r = fn_177b7c(
        m,
        &mut *h,
        0x1915B8,
        rd(m, owner.wrapping_add(184)),
        fcell,
        scratch.wrapping_add(8),
    );
    if r >= 0x80000000 {
        fail_log_19126c(m, h, state, r, 0x1915E0);
        return cleanup_19126c(m, h, owner, state, user, r, scratch);
    }
    let fact = rd(m, fcell);

    let mgr = rd(m, owner.wrapping_add(176));
    let task = rd(m, tcell);
    let r = fn_176e40(
        m,
        &mut *h,
        rd(m, mgr.wrapping_add(8)),
        task,
        state.wrapping_add(0x24),
    );
    if r >= 0x80000000 {
        fail_log_19126c(m, h, state, r, 0x1915EC);
        route_or_host(m, &mut *h, 0x177CC8, &[fact]);
        return cleanup_19126c(m, h, owner, state, user, r, scratch);
    }

    let r = {
        let mut rhh = RouteHost { inner: &mut *h };
        m.fn_1961d0(
            &mut rhh,
            [
                task,
                rd(m, mgr.wrapping_add(8)),
                fact,
                0,
                state.wrapping_add(8),
            ],
        )
    };
    if r >= 0x80000000 {
        fail_log_19126c(m, h, state, r, 0x1915F0);
        route_or_host(m, &mut *h, 0x177CC8, &[fact]);
        route_or_host(m, &mut *h, 0x176EDC, &[rd(m, state.wrapping_add(0x24))]);
        return cleanup_19126c(m, h, owner, state, user, r, scratch);
    }

    let r = fn_19611c(
        m,
        rd(m, state.wrapping_add(8)),
        rd(m, state.wrapping_add(12)),
        state.wrapping_add(0x10),
    );
    if r >= 0x80000000 {
        fail_log_19126c(m, h, state, r, 0x1915F0);
        route_or_host(m, &mut *h, 0x177CC8, &[fact]);
        route_or_host(m, &mut *h, 0x176EDC, &[rd(m, state.wrapping_add(0x24))]);
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
        let r = fn_14de40(
            m,
            &mut *h,
            rd(m, ctx.wrapping_add(0x10)),
            state.wrapping_add(0x18),
            rd(m, 0x190EA0),
        );
        let _ = r;
    }

    let r = h.call(
        m,
        0x174CA8,
        &[
            0,
            rd(m, state.wrapping_add(8)),
            rd(m, state.wrapping_add(12)),
            ctx.wrapping_add(0x14),
        ],
    );
    if r >= 0x80000000 {
        fail_log_19126c(m, h, state, r, 0x1915F4);
        return cleanup_19126c(m, h, owner, state, user, r, scratch);
    }

    let r = fn_17ae04(
        m,
        &mut *h,
        0,
        rd(m, state.wrapping_add(8)),
        rd(m, state.wrapping_add(12)),
        ctx.wrapping_add(0x18),
        scratch.wrapping_add(52),
    );
    if r >= 0x80000000 {
        fail_log_19126c(m, h, state, r, 0x1915F8);
        return cleanup_19126c(m, h, owner, state, user, r, scratch);
    }

    let tab = rd(m, ctx.wrapping_add(0x14));
    {
        let map = rd(m, tab.wrapping_add(8));
        let heap = if map == 0 {
            0
        } else {
            rd(m, map.wrapping_add(56))
        };
        let _f = if heap == 0 {
            rd(m, 0)
        } else {
            rd(m, rd(m, heap))
        };
        let t16 = rd(m, tab.wrapping_add(16));
        let _sh = rd(m, 0x1525E000);

        if t16 != 0 {
            let cnt = rd(m, t16.wrapping_add(20));
            let esz = rd(m, t16.wrapping_add(4));

            for i in 0..cnt.min(6) {
                let ent = rd(m, t16).wrapping_add(i.wrapping_mul(esz));
                let nm = rd(m, ent.wrapping_add(4));
                let nh = if nm == 0 {
                    0
                } else {
                    rd(m, nm.wrapping_add(56))
                };
                let _nf = if nh == 0 {
                    0xEEEE_EEEE
                } else {
                    rd(m, rd(m, nh))
                };
            }
        }
    }
    let r = {
        let mut hh = HeapCalls { inner: &mut *h };
        tts_core::boot_port_1::fn_1758ac(m, tab, 0x1918D8, 1, scratch.wrapping_add(284), &mut hh)
    };
    if r >= 0x80000000 {
        fail_log_19126c(m, h, state, r, 0x191910);
        return cleanup_19126c(m, h, owner, state, user, r, scratch);
    }
    let r = {
        let mut hh = HeapCalls { inner: &mut *h };
        tts_core::boot_port_1::fn_1758ac(m, tab, 0x1918E0, 250, scratch.wrapping_add(284), &mut hh)
    };
    if r >= 0x80000000 {
        fail_log_19126c(m, h, state, r, 0x191910);
        return cleanup_19126c(m, h, owner, state, user, r, scratch);
    }

    let r = fn_1759a0(
        m,
        &mut *h,
        tab,
        rd(m, 0x1918F8),
        rd(m, 0x1918F4),
        scratch.wrapping_add(84),
    );
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

    let r = route_or_host(
        m,
        &mut *h,
        0x17B0F8,
        &[rd(m, ctx.wrapping_add(0x18)), rd(m, 0x191914), st],
    );
    if r >= 0x80000000 {
        fail_log_19126c(m, h, state, r, 0x191918);
        return cleanup_19126c(m, h, owner, state, user, r, scratch);
    }

    wr(m, st.wrapping_add(4), rd(m, owner.wrapping_add(0xD8)));
    let r = route_or_host(
        m,
        &mut *h,
        0x17B0F8,
        &[rd(m, ctx.wrapping_add(0x18)), 0x19191C, st],
    );
    if r >= 0x80000000 {
        fail_log_19126c(m, h, state, r, 0x19191C);
        return cleanup_19126c(m, h, owner, state, user, r, scratch);
    }

    fn_195bec(m, st, 0, 0x14);
    wr(m, st.wrapping_add(4), rd(m, 0x191928));
    wr(m, st.wrapping_add(16), rd(m, 0x19192C));

    let vr = fn_157080(
        m,
        &mut *h,
        rd(m, state.wrapping_add(8)),
        rd(m, state.wrapping_add(12)),
        owner.wrapping_add(192),
    );

    if vr >= 0x80000000 {
        fail_log_19126c(m, h, state, vr, 0x191930);
        return cleanup_19126c(m, h, owner, state, user, vr, scratch);
    }
    let voice_api = rd(m, owner.wrapping_add(188));
    let setup = rd(m, voice_api.wrapping_add(0x10));

    let r = route_or_host(
        m,
        &mut *h,
        setup,
        &[
            rd(m, owner.wrapping_add(192)),
            rd(m, owner.wrapping_add(196)),
            rd(m, state.wrapping_add(8)),
            rd(m, state.wrapping_add(12)),
            st,
            state,
            0,
            state.wrapping_add(0x2C),
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
        tts_core::log_live::fn_1738e8(
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

pub fn fn_191e40(m: &mut Memory<'_>, h: &mut impl Calls, a0: u32, a1: u32, a2: u32) -> u32 {
    let sp = fn_129778(m, &mut *h, 0x18);
    if sp == 0 {
        return 0x80000000;
    }
    for i in (0..0x18u32).step_by(4) {
        wr(m, sp.wrapping_add(i), 0);
    }

    tts_core::boot_part_d::fn_176fd4(m, sp, pair_176fd4(m));
    wr(m, sp.wrapping_add(0x10), rd(m, sp));
    wr(m, sp.wrapping_add(0x14), rd(m, sp.wrapping_add(4)));
    wb(m, sp.wrapping_add(8), 0);
    if a1 != rd(m, 0x191224) || a0 == 0 || rd(m, a0) != rd(m, 0x191224).wrapping_add(0x0B000000) {
        return 0x80000008;
    }
    {
        let ctx = rd(m, a0.wrapping_add(0x10));
        let mut rhh = RouteHost { inner: &mut *h };
        tts_core::log_live::fn_1738e8(
            m,
            rd(m, ctx.wrapping_add(0x10)),
            rd(m, 0x191FA0),
            4,
            0,
            0x191FC8,
            sp,
            &mut rhh,
        );
    }

    let r = route_or_host(
        m,
        &mut *h,
        0x18F24C,
        &[a0, 0x191FA8, 3, sp.wrapping_add(8), sp],
    );
    if r >= 0x80000000 {
        return fn_160fec(r);
    }

    let mut r5 = rd(m, 0x191DF0);
    if a2 != 0 && rd(m, a2.wrapping_add(8)) != 0 && rd(m, a2.wrapping_add(4)) != 0 {
        if rb(m, a2) != 0 {
            r5 = rd(m, 0x191FE4);
        } else if rd(m, a0.wrapping_add(0x170)) != 0 {
            let q = route_or_host(m, &mut *h, 0x18FDC0, &[a0, a2, sp.wrapping_add(0x10)]);
            if q >= 0x80000000 {
                r5 = q;
            } else {
                let qr = route_or_host(
                    m,
                    &mut *h,
                    0x190410,
                    &[
                        a0,
                        0,
                        rd(m, sp.wrapping_add(0x10)),
                        rd(m, sp.wrapping_add(0x14)),
                    ],
                );
                r5 = qr;
            }
        } else {
            let _ = route_or_host(m, &mut *h, 0x1959DC, &[a0]);
        }
    }
    {
        let ctx = rd(m, a0.wrapping_add(0x10));
        let mut rhh = RouteHost { inner: &mut *h };
        tts_core::log_live::fn_1738e8(
            m,
            rd(m, ctx.wrapping_add(0x10)),
            rd(m, 0x191FA0),
            4,
            0,
            0x191FE8,
            sp,
            &mut rhh,
        );
    }
    if rb(m, sp.wrapping_add(8)) != 0 {
        let _ = route_or_host(m, &mut *h, 0x1959DC, &[a0]);
    }
    fn_160fec(r5)
}

fn fail_log_19126c(m: &mut Memory<'_>, h: &mut impl Calls, state: u32, r5: u32, c_addr: u32) {
    let ctx = rd(m, state.wrapping_add(0x10));
    let logger = if ctx == 0 {
        0
    } else {
        rd(m, ctx.wrapping_add(0x10))
    };
    if logger == 0 {
        return;
    }
    let sp = fn_129778(m, h, 8);
    wr(m, sp, r5);
    wr(m, sp.wrapping_add(4), 0);
    tts_core::cold_boot::fn_173dc0(
        m,
        h,
        logger,
        rd(m, 0x190E84),
        rd(m, c_addr),
        0x1915D8,
        sp,
        sp,
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
        route_or_host(m, &mut *h, 0x190EDC, &[state, rd(m, 0x191224)]);
    } else {
        route_or_host(m, &mut *h, rd(m, owner.wrapping_add(0x10)), &[user, state]);
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
        tts_core::boot_part_b::fn_173c70(
            m,
            logger,
            rd(m, 0x190E84),
            rd(m, 0x191940),
            0,
            0,
            err,
            &mut rh,
        );
    }
    fn_160fec(err)
}

pub fn fn_18f24c(
    m: &mut Memory<'_>,
    h: &mut impl Calls,
    o: u32,
    v: u32,
    n: u32,
    out: u32,
    ls: u32,
) -> u32 {
    let mut r5 = route_or_host(m, &mut *h, 0x176F2C, &[rd(m, o.wrapping_add(0x24))]);
    if r5 < 0x80000000 {
        if rb(m, o.wrapping_add(0x28)) == 1 {
            wb(m, out, 1);
            wb(m, o.wrapping_add(0x28), n as u8);
        } else {
            let logger = rd(m, rd(m, o.wrapping_add(0x10)).wrapping_add(0x10));
            wr(m, ls, 0x18F308);
            wr(m, ls.wrapping_add(4), v);
            tts_core::cold_boot::fn_173dc0(
                m,
                h,
                logger,
                rd(m, 0x18F338),
                rd(m, 0x18F32C),
                0x18F330,
                ls,
                ls.wrapping_add(4),
            );
            r5 = rd(m, 0x18F33C);
        }
        let r0 = route_or_host(m, &mut *h, 0x176F5C, &[rd(m, o.wrapping_add(0x24))]);
        if r0 >= 0x80000000 && r5 < 0x80000000 {
            r5 = r0;
        }
    }
    r5
}

pub fn fn_191948(
    m: &mut Memory<'_>,
    h: &mut impl Calls,
    owner: u32,
    tag: u32,
    user: u32,
    scratch: u32,
) -> u32 {
    if tag != rd(m, 0x191224) || owner == 0 {
        return 0x80000008;
    }
    let logger = rd(m, rd(m, owner.wrapping_add(0x10)).wrapping_add(0x10));
    wr(m, scratch, 0x191A64);
    tts_core::log_live::fn_1738e8(
        m,
        logger,
        rd(m, 0x190E84),
        4,
        0,
        0x191A64,
        scratch,
        &mut RouteHost { inner: &mut *h },
    );
    let r = fn_19611c(
        m,
        rd(m, owner.wrapping_add(8)),
        rd(m, owner.wrapping_add(12)),
        scratch.wrapping_add(8),
    );
    if r >= 0x80000000 {
        return tts_core::cold_boot::fn_160fec(r);
    }
    if user == 0 {
        return 0x80000006;
    }
    let u4 = rd(m, user.wrapping_add(4));
    if u4 == 0 {
        return 0x80000012;
    }
    let r = fn_18f24c(
        m,
        &mut *h,
        owner,
        rd(m, 0x191A7C),
        2,
        scratch.wrapping_add(4),
        scratch.wrapping_add(8),
    );
    if r >= 0x80000000 {
        return tts_core::cold_boot::fn_160fec(r);
    }
    wr(m, owner.wrapping_add(0x170), u4);
    wr(m, owner.wrapping_add(0x16C), rd(m, user));
    let _ = tts_core::engine_live::fn_1959dc_m(
        m,
        owner,
        rb(m, scratch.wrapping_add(4)) as u32,
        &mut *h,
    );
    wr(m, scratch, 0x191A94);
    tts_core::log_live::fn_1738e8(
        m,
        logger,
        rd(m, 0x190E84),
        4,
        0,
        0x191A94,
        scratch,
        &mut RouteHost { inner: &mut *h },
    );
    tts_core::cold_boot::fn_160fec(r)
}

pub fn fn_194454(
    m: &mut Memory<'_>,
    h: &mut impl Calls,
    owner: u32,
    tag: u32,
    r6: u32,
    r8: u32,
    r4: u32,
    r7: u32,
    scratch: u32,
) -> u32 {
    tts_core::boot_part_d::fn_176fd4(m, scratch, pair_176fd4(m));
    if tag != rd(m, 0x1940CC) || owner == 0 {
        return 0x80000008;
    }
    let logger = rd(m, rd(m, owner.wrapping_add(0x10)).wrapping_add(0x10));
    wr(m, scratch, 0x1945EC);
    tts_core::log_live::fn_1738e8(
        m,
        logger,
        rd(m, 0x19460C),
        4,
        0,
        0x1945EC,
        scratch,
        &mut RouteHost { inner: &mut *h },
    );
    if r6 == 0 {
        return 0x80000006;
    }
    if r7 == 0 || r4 == 0 {
        return 0x80000006;
    }
    if r8 == 0 {
        return 0x80000007;
    }
    let r = fn_18f24c(
        m,
        &mut *h,
        owner,
        rd(m, 0x194618),
        2,
        scratch.wrapping_add(0x14),
        scratch.wrapping_add(0x10),
    );
    if r >= 0x80000000 {
        return tts_core::cold_boot::fn_160fec(r);
    }
    let mut r6v = tts_core::frontend::copy_utf16_as_ascii(
        &mut *m.bytes,
        Some(r6),
        scratch.wrapping_add(0x18),
        0x80,
    );
    if r6v >= 0x80000000 {
        wr(m, r4, 0);
        wr(m, r4.wrapping_add(4), 0);
    } else {
        tts_core::boot_part_d::fn_176fd4(m, scratch.wrapping_add(0xA0), pair_176fd4(m));
        wr(m, scratch.wrapping_add(0x10), scratch.wrapping_add(0x98));
        wr(
            m,
            scratch.wrapping_add(8),
            rd(m, scratch.wrapping_add(0xA0)),
        );
        wr(
            m,
            scratch.wrapping_add(12),
            rd(m, scratch.wrapping_add(0xA4)),
        );
        wr(m, scratch, r7);
        wr(m, scratch.wrapping_add(4), r8);

        let vp = rd(
            m,
            rd(m, rd(m, owner.wrapping_add(4)).wrapping_add(0xBC)).wrapping_add(0x24),
        );

        r6v = h.call(
            m,
            vp,
            &[
                rd(m, owner.wrapping_add(0x2C)),
                rd(m, owner.wrapping_add(0x30)),
                scratch.wrapping_add(0x18),
                0,
                r7,
                r8,
                rd(m, scratch.wrapping_add(8)),
                rd(m, scratch.wrapping_add(12)),
                rd(m, scratch.wrapping_add(0x10)),
            ],
        );
        if r6v >= 0x80000000 {
            wr(m, r4, 0);
            wr(m, r4.wrapping_add(4), 0);
        } else {
            wr(m, r4, rd(m, scratch.wrapping_add(0x98)));
            wr(m, r4.wrapping_add(4), rd(m, scratch.wrapping_add(0x9C)));
        }
    }
    let _ = tts_core::engine_live::fn_1959dc_m(
        m,
        owner,
        rb(m, scratch.wrapping_add(0x14)) as u32,
        &mut *h,
    );
    wr(m, scratch, 0x194628);
    tts_core::log_live::fn_1738e8(
        m,
        logger,
        rd(m, 0x19460C),
        4,
        0,
        0x194628,
        scratch,
        &mut RouteHost { inner: &mut *h },
    );
    tts_core::cold_boot::fn_160fec(r6v)
}

pub fn fn_174a04(m: &mut Memory<'_>, h: &mut impl Calls, src: u32, heap: u32, out: u32) -> u32 {
    if src == 0 || out == 0 {
        return 0;
    }
    wr(m, out, 0);
    let object = route_or_host(m, &mut *h, 0x1963B8, &[heap, 1, 12]);
    if object == 0 {
        return 0x8B70200A;
    }
    let rc = route_or_host(
        m,
        &mut *h,
        0x176054,
        &[object, rd(m, src), rd(m, src.wrapping_add(4)), heap],
    );
    if rc < 0x80000000 {
        wr(m, out, object);
        return 0;
    }
    let callbacks = rd(m, object.wrapping_add(8));
    if callbacks != 0 {
        route_or_host(m, &mut *h, 0x196C7C, &[callbacks]);
        wr(m, object.wrapping_add(8), 0);
    }
    let data = rd(m, object);
    if data != 0 {
        route_or_host(m, &mut *h, 0x196428, &[heap, data]);
        wr(m, object, 0);
    }
    wr(m, object.wrapping_add(4), 0);
    route_or_host(m, &mut *h, 0x196428, &[heap, object]);
    rc
}

pub fn fn_17ad98(m: &mut Memory<'_>, h: &mut impl Calls, src: u32, heap: u32, out: u32) -> u32 {
    if src == 0 || out == 0 {
        return 0x8B602007;
    }
    wr(m, out, 0);
    let object = route_or_host(m, &mut *h, 0x196394, &[heap, 32]);
    if object == 0 {
        return 0x8B60200A;
    }
    route_or_host(m, &mut *h, 0x195AE8, &[object, src, 32]);
    wr(m, out, object);
    0
}

pub fn fn_17b0f8(m: &mut Memory<'_>, h: &mut impl Calls, o: u32, key: u32, st: u32) -> u32 {
    if o == 0 || key == 0 || st == 0 {
        return rd(m, 0x17B4EC);
    }
    let mut r5 = route_or_host(m, &mut *h, 0x176F2C, &[rd(m, o.wrapping_add(12))]);
    if r5 >= 0x80000000 {
        return r5;
    }
    let sp = fn_129778(m, &mut *h, 0x28);
    if sp == 0 {
        let _ = route_or_host(m, &mut *h, 0x176F5C, &[rd(m, o.wrapping_add(12))]);
        return 0x8000000A;
    }
    wr(m, sp.wrapping_add(0x20), 0);
    let r = route_or_host(
        m,
        &mut *h,
        0x1966AC,
        &[rd(m, o.wrapping_add(16)), key, sp.wrapping_add(0x20)],
    );
    r5 = r;
    if r < 0x80000000 {
        route_or_host(
            m,
            &mut *h,
            0x195AE8,
            &[rd(m, sp.wrapping_add(0x20)), st, 0x14],
        );
    } else {
        fn_195bec(m, sp, 0, 0x20);
        route_or_host(m, &mut *h, 0x195AE8, &[sp, st, 0x14]);
        wh(m, sp.wrapping_add(0x1C), 0);
        r5 = route_or_host(m, &mut *h, 0x19668C, &[rd(m, o.wrapping_add(16)), key, sp]);
    }
    let r0 = route_or_host(m, &mut *h, 0x176F5C, &[rd(m, o.wrapping_add(12))]);
    if r0 >= 0x80000000 && r5 < 0x80000000 {
        r5 = r0;
    }
    r5
}

pub fn fn_17b058(m: &mut Memory<'_>, h: &mut impl Calls, comp: u32, name: u32, out: u32) -> u32 {
    if comp == 0 || name == 0 || out == 0 {
        return rd(m, 0x17B4EC);
    }
    let r = route_or_host(m, &mut *h, 0x176F2C, &[rd(m, comp.wrapping_add(12))]);
    if r >= 0x80000000 {
        return r;
    }
    let sp = fn_129778(m, &mut *h, 4);
    if sp == 0 {
        let _ = route_or_host(m, &mut *h, 0x176F5C, &[rd(m, comp.wrapping_add(12))]);
        return 0x8000000A;
    }
    let mut r4 = route_or_host(
        m,
        &mut *h,
        0x1966AC,
        &[rd(m, comp.wrapping_add(16)), name, sp],
    );
    if r4 < 0x80000000 {
        let q = rd(m, sp);
        wr(m, out, q);
        wh(
            m,
            q.wrapping_add(0x1C),
            rh(m, q.wrapping_add(0x1C)).wrapping_add(1),
        );
    } else {
        r4 = (r4 & 0x1FFF) | rd(m, 0x17B4FC);
    }
    let u = route_or_host(m, &mut *h, 0x176F5C, &[rd(m, comp.wrapping_add(12))]);
    if u >= 0x80000000 {
        if r4 < 0x80000000 {
            r4 = u;
        }
        return r4;
    }
    r4
}

pub fn fn_1966ac(m: &mut Memory<'_>, h: &mut impl Calls, map: u32, key: u32, out: u32) -> u32 {
    if map == 0 || key == 0 || out == 0 {
        return 0x89002007;
    }
    let node = tree_find(m, &mut *h, map.wrapping_add(0x18), key);
    if node == 0 {
        0x89002014
    } else {
        wr(m, out, rd(m, node.wrapping_add(20)));
        0
    }
}

fn tree_find(m: &mut Memory<'_>, h: &mut impl Calls, tree: u32, key: u32) -> u32 {
    let cmp_fn = rd(m, rd(m, tree.wrapping_add(0x24)).wrapping_add(8));
    let mut node = rd(m, tree);

    if node == tree {}
    let mut found = 0;
    while node != tree {
        let cmp = route_or_host(m, &mut *h, cmp_fn, &[key, rd(m, node.wrapping_add(16))]) as i32;
        if cmp == 0 {
            found = node;
            break;
        }

        node = rd(m, node.wrapping_add(if cmp < 0 { 0 } else { 4 }));
    }
    found
}

pub fn fn_19668c(m: &mut Memory<'_>, h: &mut impl Calls, map: u32, key: u32, incoming: u32) -> u32 {
    {
        let mut kb = Vec::new();
        for i in 0..40u32 {
            let b = rb(m, key.wrapping_add(i));
            if b == 0 {
                break;
            }
            kb.push(b);
        }
    }
    if map == 0 || key == 0 || incoming == 0 {
        return 0x89002007;
    }
    let s = fn_129778(m, &mut *h, 4);
    if s == 0 {
        return 0x8900200A;
    }
    tree_insert_node(m, &mut *h, map.wrapping_add(0x18), key, incoming, s)
}

fn tree_insert_node(
    m: &mut Memory<'_>,
    h: &mut impl Calls,
    tree: u32,
    key: u32,
    incoming: u32,
    s: u32,
) -> u32 {
    let heap = rd(m, tree.wrapping_add(0x20));
    let vt = rd(m, tree.wrapping_add(0x24));
    let node = route_or_host(m, &mut *h, 0x196394, &[heap, 24]);
    if node == 0 {
        return 0x8900200A;
    }
    wr(m, s, 0);
    let copyfn = rd(m, vt.wrapping_add(12));
    let r = if incoming == 0 {
        0
    } else {
        route_or_host(m, &mut *h, copyfn, &[incoming, heap, s])
    };
    let value = rd(m, s);

    if r >= 0x80000000 {
        route_or_host(m, &mut *h, 0x196428, &[heap, node]);
        return r;
    }
    for off in [0u32, 4, 8, 16] {
        wr(m, node.wrapping_add(off), 0);
    }
    wb(m, node.wrapping_add(12), 0);
    wr(m, node.wrapping_add(20), value);
    let r = tree_link(m, &mut *h, tree, node, key, heap, vt, s);
    if r < 0x80000000 {
        return 0;
    }
    let k = rd(m, node.wrapping_add(16));
    if k != 0 {
        route_or_host(m, &mut *h, rd(m, vt.wrapping_add(4)), &[k, heap]);
    }
    if rd(m, node.wrapping_add(20)) != 0 {
        route_or_host(m, &mut *h, rd(m, vt.wrapping_add(16)), &[value, heap]);
    }
    route_or_host(m, &mut *h, 0x196428, &[heap, node]);
    r
}

struct LinkResults {
    path: [(u32, i32); 64],
    at: usize,
    key: u32,
    heap: u32,
    node: u32,
    rc: u32,
    created: u32,
}

impl tts_core::frontend::TreeOps for LinkResults {
    fn tree_cmp(&mut self, key: u32, node_key: u32) -> i32 {
        assert_eq!(key, self.key);
        let (expected, result) = self.path[self.at];
        self.at += 1;
        assert_eq!(node_key, expected);
        result
    }
    fn tree_init(&mut self, key: u32, heap: u32, node: u32) -> (u32, Option<u32>) {
        assert_eq!((key, heap, node), (self.key, self.heap, self.node));
        (self.rc, Some(self.created))
    }
    fn tree_alloc(&mut self, _: u32, _: u32) -> (u32, u32) {
        panic!("unexpected tree allocation")
    }
    fn tree_free_node(&mut self, _: u32, _: u32) {
        panic!("unexpected tree free")
    }
    fn tree_free_handle(&mut self, _: u32, _: u32) {
        panic!("unexpected tree handle free")
    }
}

#[allow(clippy::too_many_arguments)]
fn tree_link(
    m: &mut Memory<'_>,
    h: &mut impl Calls,
    tree: u32,
    node: u32,
    key: u32,
    heap: u32,
    vt: u32,
    s: u32,
) -> u32 {
    let mut path = [(0u32, 0i32); 64];
    let mut n = 0usize;
    let mut x = rd(m, tree);

    while x != tree {
        assert!(n < path.len(), "invalid boot tree depth");
        let k = rd(m, x.wrapping_add(16));
        let cmp = route_or_host(m, &mut *h, rd(m, vt.wrapping_add(8)), &[key, k]) as i32;
        {
            let mut kb = Vec::new();
            for i in 0..24u32 {
                let b = rb(m, k.wrapping_add(i));
                if b == 0 {
                    break;
                }
                kb.push(b);
            }
        }
        path[n] = (k, cmp);
        n += 1;
        if rb(m, tree.wrapping_add(0x28)) == 0 && cmp == 0 {
            return 0x89002002;
        }
        x = rd(m, x.wrapping_add(if cmp < 0 { 0 } else { 4 }));
    }
    wr(m, s, rd(m, node.wrapping_add(16)));
    let rc = route_or_host(m, &mut *h, rd(m, vt), &[key, heap, s]);
    let created = rd(m, s);
    let _pre = rd(m, tree);
    let mut ops = LinkResults {
        path,
        at: 0,
        key,
        heap,
        node,
        rc,
        created,
    };
    let r = tts_core::frontend::tree_insert(&mut *m.bytes, tree, node, key, &mut ops);

    r
}

pub fn fn_1770f8(m: &Memory<'_>, key: u32, nodekey: u32) -> u32 {
    let r2 = key | nodekey;
    if r2 == 0 {
        return 0;
    }
    if key == 0 {
        return 0xFFFF_FFFF;
    }
    if nodekey == 0 {
        return 1;
    }
    fn_1769d0(m, key, nodekey)
}

pub fn fn_17c8c0(m: &Memory<'_>, byte: u32) -> u32 {
    let tbl = rd(m, 0x17C8F8);
    if rh(m, tbl + ((byte & 255) * 2)) & 1 != 0 {
        byte.wrapping_add(32)
    } else {
        byte
    }
}
pub fn fn_1769d0(m: &Memory<'_>, key: u32, nodekey: u32) -> u32 {
    let mut i = 0;
    loop {
        let a = fn_17c8c0(m, rb(m, key + i) as u32);
        let b = fn_17c8c0(m, rb(m, nodekey + i) as u32);
        if a != b {
            return a.wrapping_sub(b);
        }
        if rb(m, key + i) == 0 {
            return 0;
        }
        i += 1;
    }
}

pub fn fn_196428(m: &mut Memory<'_>, h: &mut impl Calls, heap: u32, obj: u32) -> u32 {
    if heap != 0 && obj != 0 {
        route_or_host(
            m,
            &mut *h,
            rd(m, rd(m, heap).wrapping_add(12)),
            &[rd(m, heap.wrapping_add(4)), obj],
        );
    }
    0
}

pub fn fn_19690c(m: &mut Memory<'_>, h: &mut impl Calls, src: u32, heap: u32, out: u32) -> u32 {
    if src == 0 || heap == 0 || out == 0 {
        return 0x89002007;
    }
    let mut n = 0u32;
    while rb(m, src.wrapping_add(n)) != 0 {
        n = n.wrapping_add(1);
    }
    n = n.wrapping_add(1);
    let allocated = route_or_host(m, &mut *h, 0x196394, &[heap, n]);
    let object = rd(m, heap.wrapping_add(4));
    let mut vt = Prealloc {
        object,
        size: n,
        result: allocated,
    };
    tts_core::frontend::clone_str(&mut *m.bytes, Some(src), Some(heap), Some(out), &mut vt)
}

pub fn fn_19719c(m: &mut Memory<'_>, h: &mut impl Calls, cell: u32, heap: u32, out: u32) -> u32 {
    let s = rd(m, cell);
    let len = if s == 0 { 0 } else { fn_195d90(m, s) };
    let obj = route_or_host(m, &mut *h, 0x196394, &[heap, len.wrapping_add(0xD)]);
    if obj == 0 {
        return rd(m, 0x1975FC);
    }
    fn_195ae8(m, obj, cell, 12);
    wr(m, obj, obj.wrapping_add(0xC));
    if len != 0 {
        fn_195cd0(m, obj.wrapping_add(0xC), s);
    } else {
        wb(m, obj.wrapping_add(0xC), 0);
    }
    wr(m, cell.wrapping_add(4), 0);
    wr(m, cell.wrapping_add(8), 0);
    wr(m, out, obj);
    0
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
    wr(m, obj.wrapping_add(8), c);
    wr(m, obj.wrapping_add(0xC), d);
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
    let r = fn_176e40(
        m,
        &mut *h,
        rd(m, engine.wrapping_add(8)),
        heap,
        obj.wrapping_add(4),
    );
    if r >= 0x80000000 {
        return teardown_174ca8(m, h, engine, out, obj, r);
    }
    let cfg = rd(m, 0x175514);

    let r = fn_1964e0(
        m,
        &mut *h,
        heap,
        0,
        rd(m, cfg),
        rd(m, cfg.wrapping_add(4)),
        rd(m, cfg.wrapping_add(8)),
        rd(m, cfg.wrapping_add(12)),
        rd(m, cfg.wrapping_add(16)),
        obj.wrapping_add(8),
    );
    if r >= 0x80000000 {
        return teardown_174ca8(m, h, engine, out, obj, r);
    }

    let r = fn_196a64(
        m,
        &mut *h,
        heap,
        rd(m, cfg.wrapping_add(0x14)),
        rd(m, cfg.wrapping_add(0x18)),
        rd(m, cfg.wrapping_add(0x1C)),
        1,
        obj.wrapping_add(0xC),
    );
    if r == 0 {
        return teardown_174ca8(m, h, engine, out, obj, err_lit);
    }
    let r = fn_196a64(
        m,
        &mut *h,
        heap,
        rd(m, cfg.wrapping_add(0x20)),
        rd(m, cfg.wrapping_add(0x24)),
        rd(m, cfg.wrapping_add(0x28)),
        2,
        obj.wrapping_add(0x10),
    );
    if r != 0 {
        return r;
    }
    teardown_174ca8(m, h, engine, out, obj, err_lit)
}

pub fn fn_172eac(
    m: &mut Memory<'_>,
    h: &mut impl Calls,
    r0: u32,
    r1: u32,
    r2: u32,
    r3: u32,
    out: u32,
) -> u32 {
    if r0 == 0 || r1 == 0 || out == 0 {
        return rd(m, 0x1730EC);
    }
    if rd(m, rd(m, r0.wrapping_add(8)).wrapping_add(0x54)) == 0 {
        return rd(m, 0x1730F4);
    }
    let obj = fn_1963b8(m, &mut *h, r1, 1, 12);
    wr(m, out, obj);
    if obj == 0 {
        return rd(m, 0x1730F0);
    }
    wr(m, obj, r0);
    wr(m, obj.wrapping_add(4), r1);
    let ip = rd(m, rd(m, r0.wrapping_add(8)).wrapping_add(0x54));
    let rc = h.call(
        m,
        ip,
        &[
            rd(m, r0.wrapping_add(16)),
            rd(m, r0.wrapping_add(4)),
            r2,
            r3,
            obj.wrapping_add(8),
        ],
    );
    if rc >= 0x80000000 {
        fn_196428(m, &mut *h, r1, obj);
        wr(m, out, 0);
    }
    rc
}

pub fn fn_17dacc(
    m: &mut Memory<'_>,
    _h: &mut impl Calls,
    sb: u32,
    r1: u32,
    r2: u32,
    r3: u32,
) -> u32 {
    wr(m, r1, 0);
    wr(m, r2, 0);
    wr(m, r3, 0);
    let mut r4 = rd(m, 0x17DB58);
    loop {
        if rd(m, r4) == 0 {
            return rd(m, 0x17DB5C);
        }
        if fn_195d58(m, sb, rd(m, r4)) != 0 {
            r4 = r4.wrapping_add(0x10);
            continue;
        }
        if rd(m, r4) == 0 {
            return rd(m, 0x17DB5C);
        }
        let v4 = rd(m, r4.wrapping_add(4));
        if v4 != 0 {
            wr(m, r1, v4);
            return 0;
        }
        wr(m, r2, rd(m, r4.wrapping_add(8)));
        wr(m, r3, rd(m, r4.wrapping_add(12)));
        return 0;
    }
}

pub fn fn_198f74(m: &Memory<'_>, head: u32) -> u32 {
    let first = rd(m, head);
    if first == head {
        return 0;
    }
    let mut node = first;
    let mut next = rd(m, node);
    while next != head {
        node = next;
        next = rd(m, next);
    }
    node
}

pub fn fn_198a80(m: &Memory<'_>, key: u32, pair: u32) -> u32 {
    let mut prev = rd(m, pair.wrapping_add(4));
    if prev == key {
        return rd(m, pair.wrapping_add(8));
    }
    let mut cur = rd(m, prev);
    while cur != key {
        prev = cur;
        cur = rd(m, cur);
    }
    prev
}

pub fn fn_198760(m: &mut Memory<'_>, h: &mut impl Calls, map: u32, key: u32) -> u32 {
    let mut node = rd(m, map);
    if node == map {
        return 0;
    }
    loop {
        let cmp_obj = rd(m, map.wrapping_add(0x24));
        let target = rd(m, cmp_obj.wrapping_add(8));
        let c = route_or_host(m, &mut *h, target, &[key, rd(m, node.wrapping_add(0x10))]);
        if c == 0 {
            return node;
        } else if (c as i32) > 0 {
            node = rd(m, node.wrapping_add(4));
        } else {
            node = rd(m, node);
        }
        if node == map {
            return 0;
        }
    }
}

pub fn fn_1987fc(m: &mut Memory<'_>, h: &mut impl Calls, map: u32, key: u32) -> u32 {
    let mut node = rd(m, map);
    if node == map {
        return 0;
    }
    let mut cand = 0u32;
    loop {
        let cmp_obj = rd(m, map.wrapping_add(0x24));
        let target = rd(m, cmp_obj.wrapping_add(8));
        let c = route_or_host(m, &mut *h, target, &[key, rd(m, node.wrapping_add(0x10))]);
        if (c as i32) > 0 {
            node = rd(m, node.wrapping_add(4));
        } else if (c as i32) < 0 {
            cand = node;
            node = rd(m, node);
        } else if rb(m, map.wrapping_add(0x28)) == 0 {
            return node;
        } else {
            cand = node;
            node = rd(m, node);
        }
        if node == map {
            return cand;
        }
    }
}

pub fn fn_196794(
    m: &mut Memory<'_>,
    h: &mut impl Calls,
    map: u32,
    key: u32,
    extra: u32,
    out: u32,
) -> u32 {
    if map == 0 || out == 0 {
        return rd(m, 0x196A54);
    }
    let found = if key == 0 {
        fn_198f74(m, map.wrapping_add(0x18))
    } else if extra == 0 {
        fn_1987fc(m, &mut *h, map.wrapping_add(0x18), key)
    } else {
        fn_198760(m, &mut *h, map.wrapping_add(0x18), key)
    };
    if key != 0 && found == 0 {
        return rd(m, 0x196A58);
    }
    let slot = fn_1963b8(m, &mut *h, rd(m, map.wrapping_add(0x14)), 1, 12);
    wr(m, out, slot);
    if slot == 0 {
        return rd(m, 0x196A50);
    }
    wr(m, slot, map);
    let obj = rd(m, out);
    wr(m, obj.wrapping_add(4), 0);
    wr(m, rd(m, out).wrapping_add(8), found);
    0
}

pub fn fn_196860(m: &mut Memory<'_>, h: &mut impl Calls, map: u32, out1: u32, out2: u32) -> u32 {
    if map == 0 || out1 == 0 || out2 == 0 {
        return rd(m, 0x196A54);
    }
    let cursor = rd(m, map.wrapping_add(8));
    if cursor == 0 {
        wr(m, out1, 0);
        wr(m, out2, 0);
        return rd(m, 0x196A5C);
    }
    wr(m, map.wrapping_add(4), cursor);
    wr(m, out1, rd(m, cursor.wrapping_add(0x10)));
    wr(
        m,
        out2,
        rd(m, rd(m, map.wrapping_add(4)).wrapping_add(0x14)),
    );
    let pair_lo = rd(m, map);
    let next = fn_198a80(m, pair_lo.wrapping_add(0x18), rd(m, map.wrapping_add(4)));
    let _ = h;
    wr(m, map.wrapping_add(8), next);
    0
}

pub fn fn_196830(m: &mut Memory<'_>, h: &mut impl Calls, it: u32) -> u32 {
    if it == 0 {
        return rd(m, 0x196A54);
    }
    fn_196428(m, &mut *h, rd(m, rd(m, it).wrapping_add(0x14)), it);
    0
}

pub fn fn_177e0c(
    m: &mut Memory<'_>,
    h: &mut impl Calls,
    a0: u32,
    a1: u32,
    a2: u32,
    a3: u32,
    r8: u32,
    sb: u32,
) -> u32 {
    if a0 == 0 || sb == 0 {
        return rd(m, 0x177294);
    }
    let f = fn_129778(m, &mut *h, 0x44);
    if f == 0 {
        return rd(m, 0x177294);
    }
    wr(m, f.wrapping_add(0x38), 0);
    wr(m, f.wrapping_add(0x14), 0);
    wr(m, f.wrapping_add(0x18), 0);
    wr(m, f.wrapping_add(0x24), 0);
    wr(m, f.wrapping_add(0x1C), 0);
    wr(m, f.wrapping_add(0x20), 0);
    let r7 = a3;
    let mut fp = 0u32;

    let mut sl = 0u32;

    let r0a04 = rd(m, a0.wrapping_add(4));
    if r8 != 0 {
        sl = rd(m, r0a04.wrapping_add(8));
        if sl != 0 {
            tts_core::boot_part_d::fn_176fd4(m, f, pair_176fd4(m));
            wr(m, r8, rd(m, f));
            wr(m, r8.wrapping_add(4), rd(m, f.wrapping_add(4)));
        }
    }
    wr(m, sb, 0);
    fn_195bec(m, f.wrapping_add(0x28), 0, 0x10);
    wr(m, f, 0x1781D8);
    wr(m, f.wrapping_add(4), a1);
    {
        let mut rh = RouteHost { inner: &mut *h };
        tts_core::log_live::fn_1738e8(m, rd(m, a0), 0x121, 4, 0, 0x1781D8, f, &mut rh);
    }
    let lock_arg = rd(m, r0a04.wrapping_add(0xC));
    let mut r4 = route_or_host(m, &mut *h, 0x176F2C, &[lock_arg]);
    if r4 >= 0x80000000 {
        return r4;
    }
    let map14 = rd(m, r0a04.wrapping_add(0x14));

    r4 = route_or_host(m, &mut *h, 0x1966AC, &[map14, a1, f.wrapping_add(0x20)]);

    if r4 >= 0x80000000 {
        r4 = route_or_host(m, &mut *h, 0x19668C, &[map14, a1, f.wrapping_add(0x28)]);

        if r4 >= 0x80000000 {
            log_177e0c(
                m,
                &mut *h,
                a0,
                a1,
                rd(m, 0x178264),
                0x178230,
                0x17822C,
                f,
                r4,
            );
            return tail_177e0c(m, &mut *h, a0, a1, f, r4);
        }
        r4 = route_or_host(m, &mut *h, 0x1966AC, &[map14, a1, f.wrapping_add(0x20)]);

        if r4 >= 0x80000000 {
            log_177e0c(
                m,
                &mut *h,
                a0,
                a1,
                rd(m, 0x178260),
                0x17821C,
                0x178204,
                f,
                r4,
            );
            return tail_177e0c(m, &mut *h, a0, a1, f, r4);
        }

        r4 = fn_1772b0(m, &mut *h, a0, a1, f.wrapping_add(0x38));

        if r4 >= 0x80000000 {
            r4 = route_or_host(
                m,
                &mut *h,
                0x17DACC,
                &[
                    a1,
                    f.wrapping_add(0x24),
                    f.wrapping_add(0x14),
                    f.wrapping_add(0x18),
                ],
            );

            if r4 < 0x80000000 {
                return proceed_177e0c(m, &mut *h, a0, a1, a2, r7, f, r8, sb, fp, sl, r4);
            }
            if r7 != 0 {
                return tail_177e0c(m, &mut *h, a0, a1, f, r4);
            }
            log_177e0c(
                m,
                &mut *h,
                a0,
                a1,
                rd(m, 0x178264),
                0x17821C,
                0x178204,
                f,
                r4,
            );
            return tail_177e0c(m, &mut *h, a0, a1, f, r4);
        }

        let hit_obj = rd(m, f.wrapping_add(0x38));

        let q = route_or_host(
            m,
            &mut *h,
            0x1966AC,
            &[rd(m, hit_obj), 0x17823C, f.wrapping_add(0xC)],
        );

        r4 = rd(m, 0x17735C);
        if q >= 0x80000000 {
            if r7 != 0 {
                return tail_177e0c(m, &mut *h, a0, a1, f, r4);
            }
            log_177e0c(m, &mut *h, a0, a1, 0x104, 0x17821C, 0x178204, f, r4);
            r4 = route_or_host(
                m,
                &mut *h,
                0x17DACC,
                &[
                    a1,
                    f.wrapping_add(0x24),
                    f.wrapping_add(0x14),
                    f.wrapping_add(0x18),
                ],
            );
            if r4 < 0x80000000 {
                return proceed_177e0c(m, &mut *h, a0, a1, a2, r7, f, r8, sb, fp, sl, r4);
            }
            if r7 != 0 {
                return tail_177e0c(m, &mut *h, a0, a1, f, r4);
            }
            log_177e0c(
                m,
                &mut *h,
                a0,
                a1,
                rd(m, 0x178264),
                0x17821C,
                0x178204,
                f,
                r4,
            );
            return tail_177e0c(m, &mut *h, a0, a1, f, r4);
        }
        let rq = rd(m, f.wrapping_add(0xC));
        if rq != 0 {
            let rq4 = rd(m, rq.wrapping_add(4));
            if rq4 != 0 {
                wr(m, f.wrapping_add(0x14), rq4);
                fp = rd(m, rq);
            }
        }
        r4 = 0;
        return proceed_177e0c(m, &mut *h, a0, a1, a2, r7, f, r8, sb, fp, sl, r4);
    }

    let h20 = rd(m, f.wrapping_add(0x20));
    if rd(m, h20.wrapping_add(4)) != 0 {
        return publish_177e0c(m, &mut *h, a0, a1, f, h20, r8, sb, r4);
    }
    r4 = rd(m, 0x1779FC);
    if r7 != 0 {
        return tail_177e0c(m, &mut *h, a0, a1, f, r4);
    }
    log_177e0c(
        m,
        &mut *h,
        a0,
        a1,
        rd(m, 0x178228),
        0x17821C,
        0x1781FC,
        f,
        r4,
    );
    tail_177e0c(m, &mut *h, a0, a1, f, r4)
}

fn log_177e0c(
    m: &mut Memory<'_>,
    h: &mut impl Calls,
    a0: u32,
    a1: u32,
    code: u32,
    dstr: u32,
    va: u32,
    f: u32,
    r4: u32,
) {
    wr(m, f.wrapping_add(8), va);
    wr(m, f.wrapping_add(0xC), r4);
    wr(m, f, va);
    wr(m, f.wrapping_add(4), a1);
    tts_core::cold_boot::fn_173dc0(m, &mut *h, rd(m, a0), rd(m, 0x177298), code, dstr, va, f);
}

fn tail_177e0c(m: &mut Memory<'_>, h: &mut impl Calls, a0: u32, a1: u32, f: u32, r4: u32) -> u32 {
    wr(m, f.wrapping_add(8), r4);
    wr(m, f, 0x1784A8);
    wr(m, f.wrapping_add(4), a1);
    {
        let mut rh = RouteHost { inner: &mut *h };
        tts_core::log_live::fn_1738e8(m, rd(m, a0), rd(m, 0x178494), 3, 0, 0x1784A8, f, &mut rh);
    }
    final_177e0c(m, &mut *h, a0, a1, f, r4)
}

fn final_177e0c(
    m: &mut Memory<'_>,
    h: &mut impl Calls,
    a0: u32,
    a1: u32,
    f: u32,
    mut r4: u32,
) -> u32 {
    let unr = route_or_host(
        m,
        &mut *h,
        0x176F5C,
        &[rd(m, rd(m, a0.wrapping_add(4)).wrapping_add(0xC))],
    );
    if unr >= 0x80000000 && r4 < 0x80000000 {
        r4 = unr;
    }
    wr(m, f, 0x1784E4);
    wr(m, f.wrapping_add(4), a1);
    {
        let mut rh = RouteHost { inner: &mut *h };
        tts_core::log_live::fn_1738e8(m, rd(m, a0), rd(m, 0x178494), 4, 0, 0x1784E4, f, &mut rh);
    }
    r4
}

fn publish_177e0c(
    m: &mut Memory<'_>,
    h: &mut impl Calls,
    a0: u32,
    a1: u32,
    f: u32,
    slot: u32,
    r8: u32,
    sb: u32,
    r4: u32,
) -> u32 {
    let s4 = rd(m, slot.wrapping_add(4));
    wr(m, sb, s4);
    if r8 != 0 {
        wr(m, r8, rd(m, slot.wrapping_add(8)));
        wr(m, r8.wrapping_add(4), rd(m, slot.wrapping_add(12)));
    }
    final_177e0c(m, &mut *h, a0, a1, f, r4)
}

fn blx_177e0c(
    m: &mut Memory<'_>,
    h: &mut impl Calls,
    a0: u32,
    a1: u32,
    a2: u32,
    r7: u32,
    f: u32,
    r8: u32,
    sb: u32,
) -> u32 {
    let b20 = rd(m, f.wrapping_add(0x20));
    let tgt = rd(m, f.wrapping_add(0x24));

    let r = route_or_host(m, &mut *h, tgt, &[a2, b20.wrapping_add(4)]);

    if r < 0x80000000 {
        return voice_177e0c(m, &mut *h, a0, a1, f, r7, r8, sb, r);
    }
    if r7 != 0 {
        return tail_177e0c(m, &mut *h, a0, a1, f, r);
    }
    log_177e0c(
        m,
        &mut *h,
        a0,
        a1,
        rd(m, 0x1784A0),
        0x17821C,
        0x178232,
        f,
        r,
    );
    tail_177e0c(m, &mut *h, a0, a1, f, r)
}

fn voice_177e0c(
    m: &mut Memory<'_>,
    h: &mut impl Calls,
    a0: u32,
    a1: u32,
    f: u32,
    r7: u32,
    r8: u32,
    sb: u32,
    r4: u32,
) -> u32 {
    let v20 = rd(m, f.wrapping_add(0x20));
    let v1 = rd(m, v20.wrapping_add(4));
    let m3 = rd(m, v1.wrapping_add(8));
    if m3 == 0 {
        return publish_177e0c(m, &mut *h, a0, a1, f, v20, r8, sb, r4);
    }

    let ip = rd(m, a0.wrapping_add(4));
    let r = route_or_host(
        m,
        &mut *h,
        m3,
        &[rd(m, ip), rd(m, ip.wrapping_add(4)), v20.wrapping_add(8)],
    );
    if r < 0x80000000 {
        return publish_177e0c(m, &mut *h, a0, a1, f, v20, r8, sb, r);
    }
    if r7 != 0 {
        return tail_177e0c(m, &mut *h, a0, a1, f, r);
    }
    log_177e0c(
        m,
        &mut *h,
        a0,
        a1,
        rd(m, 0x1784A4),
        0x17821C,
        0x17822C,
        f,
        r,
    );
    tail_177e0c(m, &mut *h, a0, a1, f, r)
}

#[allow(clippy::too_many_arguments)]
fn proceed_177e0c(
    m: &mut Memory<'_>,
    h: &mut impl Calls,
    a0: u32,
    a1: u32,
    a2: u32,
    r7: u32,
    f: u32,
    r8: u32,
    sb: u32,
    fp: u32,
    mut sl: u32,
    r4: u32,
) -> u32 {
    let _ = (fp, r4);

    if rd(m, f.wrapping_add(0x24)) == 0 {
        wr(m, f, 0x178268);
        wr(m, f.wrapping_add(4), rd(m, f.wrapping_add(0x14)));
        wr(m, f.wrapping_add(8), rd(m, f.wrapping_add(0x1C)));
        {
            let mut rh = RouteHost { inner: &mut *h };
            tts_core::log_live::fn_1738e8(
                m,
                rd(m, a0),
                rd(m, 0x178494),
                3,
                0,
                0x178268,
                f,
                &mut rh,
            );
        }
        let f20 = rd(m, f.wrapping_add(0x20));
        wr(m, f, f20);
        sl = sl.wrapping_add(4);
        let r = fn_172eac(
            m,
            &mut *h,
            rd(m, sl.wrapping_add(0x18)),
            rd(m, sl),
            rd(m, f.wrapping_add(0x14)),
            rd(m, f.wrapping_add(0x1C)),
            f20,
        );

        if r >= 0x80000000 {
            if r7 != 0 {
                return tail_177e0c(m, &mut *h, a0, a1, f, r);
            }
            log_177e0c(
                m,
                &mut *h,
                a0,
                a1,
                rd(m, 0x178298),
                0x17821C,
                0x178204,
                f,
                r,
            );
            return tail_177e0c(m, &mut *h, a0, a1, f, r);
        }
        wr(m, f, 0x178468);
        wr(m, f.wrapping_add(4), rd(m, f.wrapping_add(0x18)));
        {
            let mut rh = RouteHost { inner: &mut *h };
            tts_core::log_live::fn_1738e8(m, rd(m, a0), 0x64616F4C, 3, 0, 0x178468, f, &mut rh);
        }
        let t0 = rd(m, f.wrapping_add(0x20));
        let t1 = rd(m, f.wrapping_add(0x18));
        let r = {
            let mut rh = RouteHost { inner: &mut *h };
            tts_core::boot_part_d::fn_172fac(m, rd(m, t0), t1, f.wrapping_add(0x24), &mut rh)
        };

        if r >= 0x80000000 {
            if r7 != 0 {
                return tail_177e0c(m, &mut *h, a0, a1, f, r);
            }
            log_177e0c(m, &mut *h, a0, a1, 0x108, 0x17821C, 0x17820C, f, r);
            return tail_177e0c(m, &mut *h, a0, a1, f, r);
        }
        if rd(m, f.wrapping_add(0x24)) != 0 {
            return blx_177e0c(m, &mut *h, a0, a1, a2, r7, f, r8, sb);
        }
        return voice_177e0c(m, &mut *h, a0, a1, f, r7, r8, sb, r);
    }
    blx_177e0c(m, &mut *h, a0, a1, a2, r7, f, r8, sb)
}

pub fn fn_1772b0(m: &mut Memory<'_>, h: &mut impl Calls, a0: u32, a1: u32, a2: u32) -> u32 {
    let default = rd(m, 0x1779FC);

    if a0 == 0 || a1 == 0 {
        return default;
    }
    wr(m, a2, 0);
    let ctx = rd(m, rd(m, a0.wrapping_add(4)).wrapping_add(0x1C));
    let mut r4 = route_or_host(m, &mut *h, 0x176F2C, &[ctx]);
    if r4 < 0x80000000 {
        let frame = fn_129778(m, &mut *h, 16);
        if frame == 0 {
            return default;
        }
        wr(m, frame, 0);
        wr(m, frame.wrapping_add(4), 0);
        wr(m, frame.wrapping_add(8), 0);
        r4 = fn_196794(
            m,
            &mut *h,
            rd(m, rd(m, a0.wrapping_add(4)).wrapping_add(0x10)),
            a1,
            1,
            frame,
        );
        if r4 < 0x80000000 {
            r4 = fn_196860(
                m,
                &mut *h,
                rd(m, frame),
                frame.wrapping_add(4),
                frame.wrapping_add(8),
            );
            if r4 < 0x80000000 {
                loop {
                    let c = fn_1769d0(m, a1, rd(m, frame.wrapping_add(4))) as i32;
                    if c == 0 {
                        wr(m, a2, rd(m, frame.wrapping_add(8)));
                        break;
                    } else if c > 0 {
                        break;
                    }
                    r4 = fn_196860(
                        m,
                        &mut *h,
                        rd(m, frame),
                        frame.wrapping_add(4),
                        frame.wrapping_add(8),
                    );
                    if r4 >= 0x80000000 {
                        break;
                    }
                }
            }
            fn_196830(m, &mut *h, rd(m, frame));
        } else if r4.wrapping_shl(19) == 0xA00000 {
            r4 = default;
        }
        let ctx2 = rd(m, rd(m, a0.wrapping_add(4)).wrapping_add(0x1C));
        let unr = route_or_host(m, &mut *h, 0x176F5C, &[ctx2]);
        if unr < 0x80000000 {
            if r4 >= 0x80000000 {
                return r4;
            }
            if rd(m, a2) == 0 {
                return default;
            }
            return r4;
        } else if r4 < 0x80000000 {
            r4 = unr;
        }
    }
    r4
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
        route_or_host(m, &mut *h, 0x196C7C, &[rd(m, obj.wrapping_add(0x10))]);
    }
    if rd(m, obj.wrapping_add(0xC)) != 0 {
        route_or_host(m, &mut *h, 0x196C7C, &[rd(m, obj.wrapping_add(0xC))]);
    }
    if rd(m, obj.wrapping_add(8)) != 0 {
        route_or_host(m, &mut *h, 0x19665C, &[rd(m, obj.wrapping_add(8))]);
    }
    if rd(m, obj.wrapping_add(4)) != 0 {
        route_or_host(m, &mut *h, 0x176EDC, &[rd(m, obj.wrapping_add(4))]);
    }
    route_or_host(m, &mut *h, 0x196428, &[rd(m, engine.wrapping_add(4)), obj]);
    wr(m, out, 0);
    err
}

pub fn fn_198728(m: &mut Memory<'_>, sub: u32, minus_one: u32, owner_obj: u32, heap: u32) {
    wr(m, sub.wrapping_add(0x18), 0);
    wb(m, sub.wrapping_add(0xC), 1);
    wb(m, sub.wrapping_add(0x28), 0);
    wr(m, sub.wrapping_add(0x24), owner_obj);
    wr(m, sub, sub);
    wr(m, sub.wrapping_add(0x1C), minus_one);
    wr(m, sub.wrapping_add(0x20), heap);
    wr(m, sub.wrapping_add(4), sub);
    wr(m, sub.wrapping_add(8), sub);
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
        route_or_host(m, &mut *h, 0x19665C, &[rd(m, sl.wrapping_add(0x14))]);
        wr(m, sl.wrapping_add(0x14), 0);
    }
    if rd(m, sl.wrapping_add(0x10)) != 0 {
        route_or_host(m, &mut *h, 0x19665C, &[rd(m, sl.wrapping_add(0x10))]);
        wr(m, sl.wrapping_add(0x10), 0);
    }
    if rd(m, sl.wrapping_add(0xC)) != 0 {
        route_or_host(m, &mut *h, 0x176EDC, &[rd(m, sl.wrapping_add(0xC))]);
        wr(m, sl.wrapping_add(0xC), 0);
    }
    if rd(m, sl.wrapping_add(0x1C)) != 0 {
        route_or_host(m, &mut *h, 0x176EDC, &[rd(m, sl.wrapping_add(0x1C))]);
        wr(m, sl.wrapping_add(0x1C), 0);
    }
    if rd(m, sl.wrapping_add(0x18)) != 0 {
        route_or_host(m, &mut *h, 0x17AD10, &[heap, rd(m, sl.wrapping_add(0x18))]);
        wr(m, sl.wrapping_add(0x18), 0);
    }
    route_or_host(m, &mut *h, 0x196428, &[heap, sl]);
    epi_log_177624(m, h, ctx, scratch);

    err
}

pub fn fn_174e74(
    m: &mut Memory<'_>,
    h: &mut impl Calls,
    a0: u32,
    a1: u32,
    a2: u32,
    a3: u32,
    a4: u32,
) -> u32 {
    const LIT_NULL: u32 = 0x8B702007;
    const LIT_FAIL: u32 = 0x8B70200A;
    const LIT_ERR: u32 = 0x8B702000;

    if a0 == 0 || a4 == 0 {
        return LIT_NULL;
    }
    let cell = fn_129778(m, &mut *h, 16);

    if cell == 0 {
        return LIT_FAIL;
    }
    fn_3023b0(m, cell, 16);
    wr(m, cell.wrapping_add(4), a2);
    wr(m, cell.wrapping_add(8), a3);
    wr(m, cell.wrapping_add(12), a4);
    let mut r4 = route_or_host(m, &mut *h, 0x176F2C, &[rd(m, a0.wrapping_add(4))]);
    if r4 >= 0x80000000 {
        return r4;
    }
    let r7 = LIT_FAIL;
    if a1 != 0 && rb(m, a1) != 0 {
        r4 = fn_1966ac(m, &mut *h, rd(m, a0.wrapping_add(8)), a1, cell);
        if r4 >= 0x80000000 {
            r4 = (r4 & 0x1FFF) | LIT_ERR;
        } else {
            let o = rd(m, cell);
            if rd(m, o.wrapping_add(8)) == 0 {
                let t1 = rd(m, 0x8D7628);
                let t2 = rd(m, 0x8D7628 + 4);
                let t3 = rd(m, 0x8D7628 + 8);
                if fn_196a64(m, &mut *h, rd(m, a0), t1, t2, t3, 1, o.wrapping_add(8)) == 0 {
                    r4 = r7;
                }
            }
            if r4 != r7 {
                let oo = rd(m, cell);
                if fn_196db0(m, &mut *h, rd(m, oo.wrapping_add(8)), cell.wrapping_add(4)) != 0 {
                } else {
                    r4 = r7;
                }
            }
        }
    } else {
        if fn_196db0(m, &mut *h, rd(m, a0.wrapping_add(12)), cell.wrapping_add(4)) != 0 {
        } else {
            r4 = r7;
        }
    }
    let unlock = route_or_host(m, &mut *h, 0x176F5C, &[rd(m, a0.wrapping_add(4))]);
    if unlock >= 0x80000000 && r4 < 0x80000000 {
        r4 = unlock;
    }

    r4
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
        route_or_host(m, &mut *h, 0x196428, &[rd(m, ctx + 4), o]);
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
    route_or_host(m, &mut *h, 0x176EDC, &[rd(m, o + 12)]);
    route_or_host(m, &mut *h, 0x196428, &[rd(m, ctx + 4), o]);
    wr(m, out, 0);
    0x8B60200A
}

fn strlen_bt(m: &Memory<'_>, s: u32) -> u32 {
    let mut n = 0u32;
    while rb(m, s.wrapping_add(n)) != 0 {
        n = n.wrapping_add(1);
    }
    n
}

fn copy_bytes(m: &mut Memory<'_>, dst: u32, src: u32) {
    let mut i = 0u32;
    loop {
        let b = rb(m, src.wrapping_add(i));
        wb(m, dst.wrapping_add(i), b);
        if b == 0 {
            break;
        }
        i = i.wrapping_add(1);
    }
}

pub fn fn_1758f8(m: &mut Memory<'_>, h: &mut impl Calls, obj: u32, key: u32, value: u32) -> u32 {
    let buf = fn_129778(m, &mut *h, 44);
    if buf == 0 {
        return 0x8900200A;
    }
    tts_core::boot_port_1::fn_176c58(m, value, buf, 10);
    let n = strlen_bt(m, buf).wrapping_add(1);
    route_or_host(m, &mut *h, 0x175170, &[obj, key, buf, n, 1])
}

pub fn fn_1759e0(m: &mut Memory<'_>, h: &mut impl Calls, obj: u32, key: u32, s: u32) -> u32 {
    let s = if s == 0 { 0x1759DCu32 } else { s };
    let n = strlen_bt(m, s).wrapping_add(1);
    route_or_host(m, &mut *h, 0x175170, &[obj, key, s, n, 1])
}

pub fn fn_19310c(
    m: &mut Memory<'_>,
    h: &mut impl Calls,
    owner: u32,
    tag: u32,
    spec: u32,
    count: u32,
    sp: u32,
) -> u32 {
    let r4 = spec;
    let r8 = count;
    let mut sb = 0u32;
    let mut fp = 0u32;
    for off in [0x1ccu32, 0x130, 0x12c, 0x1c0, 0x1c4] {
        wr(m, sp.wrapping_add(off), 0);
    }
    wh(m, sp.wrapping_add(0x10), 0);
    wb(m, sp.wrapping_add(0x1bc), 0);
    let romtag = rd(m, 0x19265C);
    if tag != romtag || owner == 0 {
        return 0x80000008;
    }
    if rd(m, owner) != romtag.wrapping_add(0xB000000) {
        return 0x80000008;
    }
    let r6 = owner;
    wr(m, sp, 0x193450);
    let logger = rd(m, rd(m, r6.wrapping_add(0x10)).wrapping_add(0x10));
    {
        let mut rh = RouteHost { inner: &mut *h };
        tts_core::log_live::fn_1738e8(m, logger, rd(m, 0x193470), 4, 0, 0x193450, sp, &mut rh);
    }
    if r4 == 0 {
        return 0x80000006;
    }
    let mut r7 = fn_19611c(
        m,
        rd(m, r6.wrapping_add(8)),
        rd(m, r6.wrapping_add(12)),
        sp.wrapping_add(0x130),
    );
    if r7 >= 0x80000000 {
        return tts_core::cold_boot::fn_160fec(r7);
    }

    route_or_host(
        m,
        &mut *h,
        0x195AE8,
        &[sp.wrapping_add(0x14), r6.wrapping_add(0x36), 0x114],
    );
    let mut sl = rb(m, r6.wrapping_add(0x34)) as u32;
    wr(
        m,
        sp.wrapping_add(0x1b8),
        rb(m, r6.wrapping_add(0x35)) as u32,
    );
    let r2err = rd(m, 0x192CAC);

    macro_rules! logd {
        ($c:expr, $d:expr, $s:expr, $v:expr) => {{
            wr(m, sp, $s);
            wr(m, sp.wrapping_add(4), $v);
            tts_core::cold_boot::fn_173dc0(
                m,
                &mut *h,
                logger,
                rd(m, 0x193470),
                $c,
                $d,
                sp,
                sp.wrapping_add(4),
            );
        }};
    }

    macro_rules! exit20 {
        () => {{
            if rd(m, sp.wrapping_add(0x1c4)) != 0 && rb(m, sp.wrapping_add(0x1bc)) != 0 {
                let _ = tts_core::engine_live::fn_1959dc_m(
                    m,
                    r6,
                    rb(m, sp.wrapping_add(0x1bc)) as u32,
                    &mut *h,
                );
            }
            return exit38(m, h, logger, r7, sp);
        }};
    }
    let sl84 = sl & 8 != 0;
    let sl44 = sl84 && sl & 4 != 0;
    if !sl44 {
        if !(r8 <= 1 && rb(m, r4) == 0xd) {
            logd!(rd(m, 0x193484), 0x19347c, 0x193474, r7);
            r7 = r2err;
            return exit38(m, h, logger, r7, sp);
        }
    }
    if rb(m, sp.wrapping_add(0xa6)) != 0 {
        return loop2(
            m, h, logger, r6, r4, r8, sp, &mut sb, &mut fp, &mut sl, &mut r7,
        );
    }

    let mut i = 0u32;
    loop {
        if i >= r8 {
            break;
        }
        let v = rb(m, r4.wrapping_add(i.wrapping_mul(0x102))) as u32;
        if v == 1 || v == 7 || v == 0xf || v == 8 || v == 0x13 {
            sb = 1;
            return loop2(
                m, h, logger, r6, r4, r8, sp, &mut sb, &mut fp, &mut sl, &mut r7,
            );
        }
        i = i.wrapping_add(1);
    }

    r7 = r2err;
    logd!(rd(m, 0x193484), 0x19347c, 0x193474, r7);
    exit20!()
}

#[allow(clippy::too_many_arguments)]
fn loop2(
    m: &mut Memory<'_>,
    h: &mut impl Calls,
    logger: u32,
    r6: u32,
    r4: u32,
    r8: u32,
    sp: u32,
    sb: &mut u32,
    fp: &mut u32,
    sl: &mut u32,
    r7: &mut u32,
) -> u32 {
    macro_rules! exit20 {
        () => {{
            if rd(m, sp.wrapping_add(0x1c4)) != 0 && rb(m, sp.wrapping_add(0x1bc)) != 0 {
                let _ = tts_core::engine_live::fn_1959dc_m(
                    m,
                    r6,
                    rb(m, sp.wrapping_add(0x1bc)) as u32,
                    &mut *h,
                );
            }
            return exit38(m, h, logger, *r7, sp);
        }};
    }
    if r8 > 0 {
        let mut i = 0u32;
        loop {
            let v = rb(m, r4.wrapping_add(i.wrapping_mul(0x102))) as u32;
            if v != 3 && v != 4 && v != 2 && v != 5 {
                wr(m, sp.wrapping_add(0x1c4), 1);
                let ls = sp.wrapping_add(0x1d4);
                let r = fn_18f24c(m, &mut *h, r6, 0x193488, 2, sp.wrapping_add(0x1bc), ls);
                *r7 = r;
                if r >= 0x80000000 {
                    wb(m, sp.wrapping_add(0x1bc), 0);
                    return exit38(m, h, logger, *r7, sp);
                }
                break;
            }
            i = (i.wrapping_add(1) & 0xffff) as u32;
            if i >= r8 {
                break;
            }
        }
    }

    let mut r5 = 0u32;
    if r8 == 0 {
        return tail(m, h, logger, r6, sp, sb, fp, sl, r7);
    }
    loop {
        let v = rb(m, r4.wrapping_add(r5.wrapping_mul(0x102))) as u32;
        if v < 0x18 {
            let target = match v {
                0x1 => case_frequency(m, &mut *h, logger, r6, r4, r5, sp, sb, r7),
                0x2 => case_1758f8(m, &mut *h, logger, r6, r4, r5, sp, 0x1937ec, r7),
                0x3 => case_1758f8(m, &mut *h, logger, r6, r4, r5, sp, 0x1934a0, r7),
                0x4 => case_1758f8(m, &mut *h, logger, r6, r4, r5, sp, 0x1934a8, r7),
                0x5 => case_1758f8(m, &mut *h, logger, r6, r4, r5, sp, 0x1937f4, r7),
                0x6 => case_1758f8(m, &mut *h, logger, r6, r4, r5, sp, 0x193800, r7),
                0x7 => case_7(m, &mut *h, logger, r6, r4, r5, sp, sb, r7),
                0x8 => case_copy(m, &mut *h, logger, r6, r4, r5, sp, 0x54, 0x20, sb, r7),
                0x9 => case_copy(m, &mut *h, logger, r6, r4, r5, sp, 0x96, 0x10, sb, r7),
                0xb => case_b(m, &mut *h, logger, r6, r4, r5, sp, r7),
                0xc => case_c(m, &mut *h, logger, r6, r4, r5, sp, r7),
                0xd => case_d(m, &mut *h, logger, r6, r4, r5, sp, sl, r7),
                0xe => case_e(m, &mut *h, logger, r6, r4, r5, sp, r7),
                0xf => case_f(m, &mut *h, logger, r6, r4, r5, sp, sb, r7),
                0x10 => case_10(m, &mut *h, logger, r6, r4, r5, sp, r7),
                0x11 => case_1758f8(m, &mut *h, logger, r6, r4, r5, sp, 0x193b54, r7),
                0x12 => case_12(m, &mut *h, logger, r6, r4, r5, sp, r7),
                0x13 => case_copy_fp(m, &mut *h, logger, r6, r4, r5, sp, sb, fp, r7),
                0x14 => case_14(m, &mut *h, logger, r6, r4, r5, sp, r7),
                0x15 => case_15(m, &mut *h, logger, r6, r4, r5, sp, r7),
                0x16 => case_16(m, &mut *h, logger, r6, r4, r5, sp, r7),
                0x17 => case_17(m, &mut *h, logger, r6, r4, r5, sp, r7),
                _ => return case_default(m, &mut *h, logger, r6, r4, r5, sp, r7),
            };
            match target {
                Flow::Next => {}
                Flow::Exit20 => exit20!(),
            }
        } else {
            return case_default(m, &mut *h, logger, r6, r4, r5, sp, r7);
        }
        r5 = r5.wrapping_add(1) & 0xffff;
        if r5 >= r8 {
            return tail(m, h, logger, r6, sp, sb, fp, sl, r7);
        }
    }
}

enum Flow {
    Next,
    Exit20,
}

#[allow(clippy::too_many_arguments)]
fn check_tail(
    m: &mut Memory<'_>,
    h: &mut impl Calls,
    logger: u32,
    r6: u32,
    r4: u32,
    r5: u32,
    sp: u32,
    r7: &mut u32,
) -> Flow {
    if *r7 < 0x80000000 {
        Flow::Next
    } else {
        log_at_a28(m, h, logger, r6, r4, r5, sp, r7);
        Flow::Exit20
    }
}

#[allow(clippy::too_many_arguments)]
fn log_at_a28(
    m: &mut Memory<'_>,
    h: &mut impl Calls,
    logger: u32,
    _r6: u32,
    r4: u32,
    r5: u32,
    sp: u32,
    r7: &mut u32,
) {
    let b = rb(m, r4.wrapping_add(r5.wrapping_mul(0x102))) as u32;
    wr(m, sp, 0x193BF8);
    wr(m, sp.wrapping_add(4), b);
    wr(m, sp.wrapping_add(8), 0x193BF0);
    wr(m, sp.wrapping_add(12), *r7);
    tts_core::cold_boot::fn_173dc0(
        m,
        &mut *h,
        logger,
        rd(m, 0x193470),
        rd(m, 0x193484),
        0x193C04,
        sp,
        sp.wrapping_add(4),
    );
}

#[allow(clippy::too_many_arguments)]
fn case_frequency(
    m: &mut Memory<'_>,
    h: &mut impl Calls,
    logger: u32,
    r6: u32,
    r4: u32,
    r5: u32,
    sp: u32,
    sb: &mut u32,
    r7: &mut u32,
) -> Flow {
    let v = rh(m, r4 + r5 * 0x102 + 2);
    if matches!(v, 8 | 11 | 16 | 22) {
        *sb = 1;
        wh(m, sp + 0x74, v);
        wr(m, sp + 0x1cc, 1);
        return check_tail(m, h, logger, r6, r4, r5, sp, r7);
    }
    wr(m, sp, 0x1934b0);
    wr(m, sp + 4, v as u32);
    tts_core::cold_boot::fn_173dc0(
        m,
        h,
        logger,
        rd(m, 0x193470),
        rd(m, 0x1934c4),
        0x1934bc,
        sp,
        sp + 4,
    );
    *r7 = rd(m, 0x1934c8);
    Flow::Exit20
}

#[allow(clippy::too_many_arguments)]
fn case_1758f8(
    m: &mut Memory<'_>,
    h: &mut impl Calls,
    logger: u32,
    r6: u32,
    r4: u32,
    r5: u32,
    sp: u32,
    key: u32,
    r7: &mut u32,
) -> Flow {
    let v = rh(m, r4.wrapping_add(r5.wrapping_mul(0x102)).wrapping_add(2)) as u32;
    let obj = rd(m, rd(m, sp.wrapping_add(0x130)).wrapping_add(0x14));
    *r7 = fn_1758f8(m, &mut *h, obj, key, v);
    check_tail(m, h, logger, r6, r4, r5, sp, r7)
}

#[allow(clippy::too_many_arguments)]
fn case_7(
    m: &mut Memory<'_>,
    h: &mut impl Calls,
    logger: u32,
    r6: u32,
    r4: u32,
    r5: u32,
    sp: u32,
    sb: &mut u32,
    r7: &mut u32,
) -> Flow {
    let src = r4.wrapping_add(r5.wrapping_mul(0x102)).wrapping_add(2);
    let _ = tts_core::frontend::copy_utf16_as_ascii(
        &mut *m.bytes,
        Some(src),
        sp.wrapping_add(0x14),
        0x40,
    );
    wh(m, sp.wrapping_add(0x10), 0);
    *sb = 1;
    if strlen_bt(m, sp.wrapping_add(0x14)) != 3 {
        return check_tail(m, h, logger, r6, r4, r5, sp, r7);
    }
    let r0 = route_or_host(
        m,
        &mut *h,
        0x192E5C,
        &[
            rd(m, r6.wrapping_add(16)),
            sp.wrapping_add(0x12c),
            sp.wrapping_add(0x10),
            sp.wrapping_add(0x14),
        ],
    );
    if r0 >= 0x80000000 {
        return check_tail(m, h, logger, r6, r4, r5, sp, r7);
    }
    copy_bytes(m, sp.wrapping_add(0x14), rd(m, sp.wrapping_add(0x12c)));
    route_or_host(
        m,
        &mut *h,
        0x196428,
        &[
            rd(m, rd(m, r6.wrapping_add(16)).wrapping_add(4)),
            rd(m, sp.wrapping_add(0x12c)),
        ],
    );
    wr(m, sp.wrapping_add(0x12c), 0);
    check_tail(m, h, logger, r6, r4, r5, sp, r7)
}

#[allow(clippy::too_many_arguments)]
fn case_copy(
    m: &mut Memory<'_>,
    h: &mut impl Calls,
    logger: u32,
    r6: u32,
    r4: u32,
    r5: u32,
    sp: u32,
    off: u32,
    cap: u32,
    sb: &mut u32,
    r7: &mut u32,
) -> Flow {
    let src = r4.wrapping_add(r5.wrapping_mul(0x102)).wrapping_add(2);
    let _ = tts_core::frontend::copy_utf16_as_ascii(
        &mut *m.bytes,
        Some(src),
        sp.wrapping_add(off),
        cap,
    );
    *sb = 1;
    check_tail(m, h, logger, r6, r4, r5, sp, r7)
}

#[allow(clippy::too_many_arguments)]
fn case_copy_fp(
    m: &mut Memory<'_>,
    h: &mut impl Calls,
    logger: u32,
    r6: u32,
    r4: u32,
    r5: u32,
    sp: u32,
    sb: &mut u32,
    fp: &mut u32,
    r7: &mut u32,
) -> Flow {
    let src = r4.wrapping_add(r5.wrapping_mul(0x102)).wrapping_add(2);
    let _ = tts_core::frontend::copy_utf16_as_ascii(
        &mut *m.bytes,
        Some(src),
        sp.wrapping_add(0x76),
        0x20,
    );
    *sb = 1;
    *fp = 1;
    check_tail(m, h, logger, r6, r4, r5, sp, r7)
}

#[allow(clippy::too_many_arguments)]
fn case_b(
    m: &mut Memory<'_>,
    h: &mut impl Calls,
    logger: u32,
    r6: u32,
    r4: u32,
    r5: u32,
    sp: u32,
    r7: &mut u32,
) -> Flow {
    let v = rh(m, r4.wrapping_add(r5.wrapping_mul(0x102)).wrapping_add(2)) as u32;
    let obj = rd(m, rd(m, sp.wrapping_add(0x130)).wrapping_add(0x14));
    if v == 1 {
        *r7 = fn_1759e0(m, &mut *h, obj, rd(m, 0x193B8C), rd(m, 0x193B84));
        return check_tail(m, h, logger, r6, r4, r5, sp, r7);
    }
    if v == 2 {
        *r7 = fn_1759e0(m, &mut *h, obj, rd(m, 0x193B8C), rd(m, 0x193B88));
        return check_tail(m, h, logger, r6, r4, r5, sp, r7);
    }
    wr(m, sp, 0x193B6C);
    wr(m, sp.wrapping_add(4), v);
    tts_core::cold_boot::fn_173dc0(
        m,
        &mut *h,
        logger,
        rd(m, 0x193470),
        rd(m, 0x193B80),
        0x193B78,
        sp,
        sp.wrapping_add(4),
    );
    *r7 = rd(m, 0x1934C8);
    Flow::Exit20
}

#[allow(clippy::too_many_arguments)]
fn case_c(
    m: &mut Memory<'_>,
    h: &mut impl Calls,
    logger: u32,
    r6: u32,
    r4: u32,
    r5: u32,
    sp: u32,
    r7: &mut u32,
) -> Flow {
    let v = rh(m, r4.wrapping_add(r5.wrapping_mul(0x102)).wrapping_add(2)) as u32;
    let obj = rd(m, rd(m, sp.wrapping_add(0x130)).wrapping_add(0x14));
    if v == 0 {
        *r7 = fn_1758f8(m, &mut *h, obj, 0x1937E0, 0);
        return check_tail(m, h, logger, r6, r4, r5, sp, r7);
    }
    if v == 1 {
        *r7 = fn_1758f8(m, &mut *h, obj, 0x1937E0, rd(m, 0x1937DC));
        return check_tail(m, h, logger, r6, r4, r5, sp, r7);
    }
    wr(m, sp, 0x1937CC);
    wr(m, sp.wrapping_add(4), v);
    tts_core::cold_boot::fn_173dc0(
        m,
        &mut *h,
        logger,
        rd(m, 0x193470),
        rd(m, 0x1937D8),
        0x1934C0,
        sp,
        sp.wrapping_add(4),
    );
    *r7 = rd(m, 0x19381C);
    Flow::Exit20
}

#[allow(clippy::too_many_arguments)]
fn case_d(
    m: &mut Memory<'_>,
    h: &mut impl Calls,
    logger: u32,
    r6: u32,
    r4: u32,
    r5: u32,
    sp: u32,
    sl: &mut u32,
    r7: &mut u32,
) -> Flow {
    let v = rh(m, r4.wrapping_add(r5.wrapping_mul(0x102)).wrapping_add(2)) as u32;
    if v == 3 || v == 0xc {
        *sl = v & 0xff;
        return check_tail(m, h, logger, r6, r4, r5, sp, r7);
    }
    wr(m, sp, 0x19380C);
    wr(m, sp.wrapping_add(4), v);
    tts_core::cold_boot::fn_173dc0(
        m,
        &mut *h,
        logger,
        rd(m, 0x193470),
        rd(m, 0x193818),
        0x19347C,
        sp,
        sp.wrapping_add(4),
    );
    *r7 = rd(m, 0x19381C);
    Flow::Exit20
}

#[allow(clippy::too_many_arguments)]
fn case_e(
    m: &mut Memory<'_>,
    h: &mut impl Calls,
    logger: u32,
    r6: u32,
    r4: u32,
    r5: u32,
    sp: u32,
    r7: &mut u32,
) -> Flow {
    let v = rh(m, r4.wrapping_add(r5.wrapping_mul(0x102)).wrapping_add(2)) as u32;
    if v == 1 || v == 2 {
        wr(m, sp.wrapping_add(0x1b8), v & 0xff);
        return check_tail(m, h, logger, r6, r4, r5, sp, r7);
    }
    wr(m, sp, 0x193820);
    wr(m, sp.wrapping_add(4), v);
    tts_core::cold_boot::fn_173dc0(
        m,
        &mut *h,
        logger,
        rd(m, 0x193470),
        rd(m, 0x19382C),
        0x1934C0,
        sp,
        sp.wrapping_add(4),
    );
    *r7 = rd(m, 0x19381C);
    Flow::Exit20
}

#[allow(clippy::too_many_arguments)]
fn case_f(
    m: &mut Memory<'_>,
    h: &mut impl Calls,
    logger: u32,
    r6: u32,
    r4: u32,
    r5: u32,
    sp: u32,
    sb: &mut u32,
    r7: &mut u32,
) -> Flow {
    let v = rh(m, r4.wrapping_add(r5.wrapping_mul(0x102)).wrapping_add(2)) as u32;
    wh(m, sp.wrapping_add(0x10), v as u16);
    *r7 = route_or_host(
        m,
        &mut *h,
        0x192E5C,
        &[
            rd(m, r6.wrapping_add(16)),
            sp.wrapping_add(0x12c),
            sp.wrapping_add(0x10),
            sp.wrapping_add(0x1c0),
        ],
    );
    if *r7 >= 0x80000000 {
        log_at_a28(m, h, logger, r6, r4, r5, sp, r7);
        return Flow::Exit20;
    }
    copy_bytes(m, sp.wrapping_add(0x14), rd(m, sp.wrapping_add(0x12c)));
    route_or_host(
        m,
        &mut *h,
        0x196428,
        &[
            rd(m, rd(m, r6.wrapping_add(16)).wrapping_add(4)),
            rd(m, sp.wrapping_add(0x12c)),
        ],
    );
    wr(m, sp.wrapping_add(0x12c), 0);
    *sb = 1;
    Flow::Next
}

#[allow(clippy::too_many_arguments)]
fn case_10(
    m: &mut Memory<'_>,
    h: &mut impl Calls,
    logger: u32,
    r6: u32,
    r4: u32,
    r5: u32,
    sp: u32,
    r7: &mut u32,
) -> Flow {
    let v = rh(m, r4.wrapping_add(r5.wrapping_mul(0x102)).wrapping_add(2)) as u32;
    if v > 0x400 {
        *r7 = rd(m, 0x19381C);
        return Flow::Exit20;
    }
    let obj = rd(m, rd(m, sp.wrapping_add(0x130)).wrapping_add(0x14));
    *r7 = fn_1758f8(m, &mut *h, obj, 0x193830, v);
    check_tail(m, h, logger, r6, r4, r5, sp, r7)
}

#[allow(clippy::too_many_arguments)]
fn case_12(
    m: &mut Memory<'_>,
    h: &mut impl Calls,
    logger: u32,
    r6: u32,
    r4: u32,
    r5: u32,
    sp: u32,
    r7: &mut u32,
) -> Flow {
    let v = rh(m, r4.wrapping_add(r5.wrapping_mul(0x102)).wrapping_add(2)) as u32;
    if v.wrapping_sub(0x19) >= rd(m, 0x193B64) {
        *r7 = rd(m, 0x19381C);
        log_at_a28(m, h, logger, r6, r4, r5, sp, r7);
        return Flow::Exit20;
    }
    let obj = rd(m, rd(m, sp.wrapping_add(0x130)).wrapping_add(0x14));
    *r7 = fn_1758f8(m, &mut *h, obj, rd(m, 0x193B68), v);
    check_tail(m, h, logger, r6, r4, r5, sp, r7)
}

#[allow(clippy::too_many_arguments)]
fn case_14(
    m: &mut Memory<'_>,
    h: &mut impl Calls,
    logger: u32,
    r6: u32,
    r4: u32,
    r5: u32,
    sp: u32,
    r7: &mut u32,
) -> Flow {
    let v = rh(m, r4.wrapping_add(r5.wrapping_mul(0x102)).wrapping_add(2)) as u32;
    let obj = rd(m, rd(m, sp.wrapping_add(0x130)).wrapping_add(0x14));
    if v <= 2 {
        let s = [0x193B90u32, 0x193BA4, 0x193BB4][v as usize];
        *r7 = fn_1759e0(m, &mut *h, obj, 0x193B98, s);
        return check_tail(m, h, logger, r6, r4, r5, sp, r7);
    }
    *r7 = rd(m, 0x19381C);
    Flow::Exit20
}

#[allow(clippy::too_many_arguments)]
fn case_15(
    m: &mut Memory<'_>,
    h: &mut impl Calls,
    logger: u32,
    r6: u32,
    r4: u32,
    r5: u32,
    sp: u32,
    r7: &mut u32,
) -> Flow {
    let v = rh(m, r4.wrapping_add(r5.wrapping_mul(0x102)).wrapping_add(2)) as u32;
    let obj = rd(m, rd(m, sp.wrapping_add(0x130)).wrapping_add(0x14));
    if v == 0 {
        *r7 = fn_1759e0(m, &mut *h, obj, 0x193BC4, 0x193BBC);
        return check_tail(m, h, logger, r6, r4, r5, sp, r7);
    }
    if v == 1 {
        *r7 = fn_1759e0(m, &mut *h, obj, 0x193BC4, 0x193BC0);
        return check_tail(m, h, logger, r6, r4, r5, sp, r7);
    }
    *r7 = rd(m, 0x19381C);
    Flow::Exit20
}

#[allow(clippy::too_many_arguments)]
fn case_16(
    m: &mut Memory<'_>,
    h: &mut impl Calls,
    logger: u32,
    r6: u32,
    r4: u32,
    r5: u32,
    sp: u32,
    r7: &mut u32,
) -> Flow {
    let src = r4.wrapping_add(r5.wrapping_mul(0x102)).wrapping_add(2);
    let _ = tts_core::frontend::copy_utf16_as_ascii(
        &mut *m.bytes,
        Some(src),
        sp.wrapping_add(0x138),
        0x80,
    );
    let obj = rd(m, rd(m, sp.wrapping_add(0x130)).wrapping_add(0x14));
    *r7 = fn_1759e0(m, &mut *h, obj, 0x193BD4, sp.wrapping_add(0x138));
    check_tail(m, h, logger, r6, r4, r5, sp, r7)
}

#[allow(clippy::too_many_arguments)]
fn case_17(
    m: &mut Memory<'_>,
    h: &mut impl Calls,
    logger: u32,
    r6: u32,
    r4: u32,
    r5: u32,
    sp: u32,
    r7: &mut u32,
) -> Flow {
    let src = r4.wrapping_add(r5.wrapping_mul(0x102)).wrapping_add(2);
    let _ = tts_core::frontend::copy_utf16_as_ascii(
        &mut *m.bytes,
        Some(src),
        sp.wrapping_add(0x138),
        0x80,
    );
    let obj = rd(m, rd(m, sp.wrapping_add(0x130)).wrapping_add(0x14));
    *r7 = fn_1759e0(m, &mut *h, obj, 0x193BE4, sp.wrapping_add(0x138));
    check_tail(m, h, logger, r6, r4, r5, sp, r7)
}

#[allow(clippy::too_many_arguments)]
fn case_default(
    m: &mut Memory<'_>,
    h: &mut impl Calls,
    logger: u32,
    r6: u32,
    r4: u32,
    r5: u32,
    sp: u32,
    r7: &mut u32,
) -> u32 {
    let b = rb(m, r4.wrapping_add(r5.wrapping_mul(0x102))) as u32;
    wr(m, sp, 0x193BF8);
    wr(m, sp.wrapping_add(4), b);
    tts_core::cold_boot::fn_173dc0(
        m,
        &mut *h,
        logger,
        rd(m, 0x193470),
        rd(m, 0x193C18),
        0x193C10,
        sp,
        sp.wrapping_add(4),
    );
    *r7 = rd(m, 0x1934C8);
    if rd(m, sp.wrapping_add(0x1c4)) != 0 && rb(m, sp.wrapping_add(0x1bc)) != 0 {
        let _ = tts_core::engine_live::fn_1959dc_m(
            m,
            r6,
            rb(m, sp.wrapping_add(0x1bc)) as u32,
            &mut *h,
        );
    }
    exit38(m, h, logger, *r7, sp)
}

#[allow(clippy::too_many_arguments)]
fn tail(
    m: &mut Memory<'_>,
    h: &mut impl Calls,
    logger: u32,
    r6: u32,
    sp: u32,
    sb: &mut u32,
    fp: &mut u32,
    sl: &mut u32,
    r7: &mut u32,
) -> u32 {
    macro_rules! exit20 {
        () => {{
            if rd(m, sp.wrapping_add(0x1c4)) != 0 && rb(m, sp.wrapping_add(0x1bc)) != 0 {
                let _ = tts_core::engine_live::fn_1959dc_m(
                    m,
                    r6,
                    rb(m, sp.wrapping_add(0x1bc)) as u32,
                    &mut *h,
                );
            }
            return exit38(m, h, logger, *r7, sp);
        }};
    }
    if *sb == 0 {
        if *r7 >= 0x80000000 {
            exit20!();
        }
    } else {
        *r7 = route_or_host(m, &mut *h, 0x1900F8, &[r6, 0, sp.wrapping_add(0x14)]);
        if *r7 < 0x80000000 {
            return tail_c28(m, h, logger, r6, sp, sb, fp, sl, r7);
        }
        if rb(m, sp.wrapping_add(0x14)) == 0 && rb(m, sp.wrapping_add(0x54)) == 0 {
            log_b28(m, h, logger, sp, *r7);
            exit20!();
        }
        if rd(m, sp.wrapping_add(0x1cc)) == 0 {
            wh(m, sp.wrapping_add(0x74), 0);
            if *fp == 0 {
                wb(m, sp.wrapping_add(0x76), 0);
            }
        } else if *fp == 0 {
            wb(m, sp.wrapping_add(0x76), 0);
        }

        let _ = route_or_host(m, &mut *h, 0x1900F8, &[r6, 0, sp.wrapping_add(0x14)]);
        log_b28(m, h, logger, sp, *r7);
        exit20!();
    }
    tail_c28(m, h, logger, r6, sp, sb, fp, sl, r7)
}

fn log_b28(m: &mut Memory<'_>, h: &mut impl Calls, logger: u32, sp: u32, r7: u32) {
    wr(m, sp, 0x193BF0);
    wr(m, sp.wrapping_add(4), r7);
    tts_core::cold_boot::fn_173dc0(
        m,
        &mut *h,
        logger,
        rd(m, 0x193470),
        rd(m, 0x193C1C),
        0x193C08,
        sp,
        sp.wrapping_add(4),
    );
}

#[allow(clippy::too_many_arguments)]
fn tail_c28(
    m: &mut Memory<'_>,
    h: &mut impl Calls,
    logger: u32,
    r6: u32,
    sp: u32,
    sb: &mut u32,
    fp: &mut u32,
    sl: &mut u32,
    r7: &mut u32,
) -> u32 {
    let r1 = if *sl & 8 != 0 && *sl & 4 != 0 { 1 } else { 0 };
    let b34 = rb(m, r6.wrapping_add(0x34)) as u32;
    let r0 = if b34 & 8 != 0 && b34 & 4 != 0 { 1 } else { 0 };
    let r4lit = rd(m, 0x193D6C);
    if r1 != r0 {
        if *sl & 8 != 0 && *sl & 4 != 0 {
            *sb = 1;
        } else {
            indirect_call(m, h, logger, r6, 0, r4lit, r7);
            if *r7 >= 0x80000000 {
                return tail_c28_logged(m, h, logger, r6, sp, r7);
            }
            return tail_cbc(m, h, logger, r6, sp, sb, fp, sl, r7);
        }
    }
    tail_cbc(m, h, logger, r6, sp, sb, fp, sl, r7)
}

#[allow(clippy::too_many_arguments)]
fn indirect_call(
    m: &mut Memory<'_>,
    h: &mut impl Calls,
    _logger: u32,
    r6: u32,
    name: u32,
    _r4lit: u32,
    r7: &mut u32,
) {
    let tgt = rd(
        m,
        rd(m, rd(m, r6.wrapping_add(4)).wrapping_add(0xbc)).wrapping_add(0x60),
    );
    *r7 = route_or_host(
        m,
        &mut *h,
        tgt,
        &[
            rd(m, r6.wrapping_add(0x2c)),
            rd(m, r6.wrapping_add(0x30)),
            name,
            0,
        ],
    );
}

fn tail_c28_logged(
    m: &mut Memory<'_>,
    h: &mut impl Calls,
    logger: u32,
    r6: u32,
    sp: u32,
    r7: &mut u32,
) -> u32 {
    wr(m, sp, 0x193BF0);
    wr(m, sp.wrapping_add(4), *r7);
    tts_core::cold_boot::fn_173dc0(
        m,
        &mut *h,
        logger,
        rd(m, 0x193470),
        rd(m, 0x193D6C),
        0x193C08,
        sp,
        sp.wrapping_add(4),
    );
    if rd(m, sp.wrapping_add(0x1c4)) != 0 && rb(m, sp.wrapping_add(0x1bc)) != 0 {
        let _ = tts_core::engine_live::fn_1959dc_m(
            m,
            r6,
            rb(m, sp.wrapping_add(0x1bc)) as u32,
            &mut *h,
        );
    }
    exit38(m, h, logger, *r7, sp)
}

#[allow(clippy::too_many_arguments)]
fn tail_cbc(
    m: &mut Memory<'_>,
    h: &mut impl Calls,
    logger: u32,
    r6: u32,
    sp: u32,
    sb: &mut u32,
    _fp: &mut u32,
    sl: &mut u32,
    r7: &mut u32,
) -> u32 {
    if *sb == 1 && *sl & 8 != 0 && *sl & 4 != 0 {
        indirect_call(m, h, logger, r6, sp + 0xA6, rd(m, 0x193D6C), r7);
        if *r7 >= 0x80000000 {
            return tail_c28_logged(m, h, logger, r6, sp, r7);
        }
    }
    wb(m, r6.wrapping_add(0x34), (*sl & 0xff) as u8);
    wb(m, r6.wrapping_add(0x35), rb(m, sp.wrapping_add(0x1b8)));
    if *sb == 0 {
        if rd(m, sp.wrapping_add(0x1c4)) != 0 && rb(m, sp.wrapping_add(0x1bc)) != 0 {
            let _ = tts_core::engine_live::fn_1959dc_m(
                m,
                r6,
                rb(m, sp.wrapping_add(0x1bc)) as u32,
                &mut *h,
            );
        }
        return exit38(m, h, logger, *r7, sp);
    }
    route_or_host(
        m,
        &mut *h,
        0x195AE8,
        &[r6.wrapping_add(0x36), sp.wrapping_add(0x14), 0x114],
    );
    if rd(m, sp.wrapping_add(0x1c4)) != 0 && rb(m, sp.wrapping_add(0x1bc)) != 0 {
        let _ = tts_core::engine_live::fn_1959dc_m(
            m,
            r6,
            rb(m, sp.wrapping_add(0x1bc)) as u32,
            &mut *h,
        );
    }
    exit38(m, h, logger, *r7, sp)
}

pub fn fn_176f94(m: &Memory<'_>, a0: u32, a1: u32, a2: u32, a3: u32) -> u32 {
    let ip = rd(m, 0x176FE4);
    if a1 != a2 {
        return ip;
    }
    if a3 == 0 {
        return 0;
    }
    if a0 == 0 {
        return ip;
    }
    0
}

pub fn fn_195df8(m: &Memory<'_>, mut s1: u32, mut s2: u32, n: u32) -> u32 {
    if n == 0 {
        return 0;
    }
    let mut r3 = 0u32;
    let mut r2 = n;
    loop {
        r2 = r2.wrapping_sub(1);
        if r2 != 0 {
            let b = rb(m, s1) as u32;
            if b == 0 {
                break;
            }
            r3 = b;
        }
        let ip = rb(m, s2) as u32;
        if r3 != ip {
            break;
        }
        s1 = s1.wrapping_add(1);
        s2 = s2.wrapping_add(1);
    }
    (rb(m, s1) as u32).wrapping_sub(rb(m, s2) as u32)
}

pub fn fn_1517e8(
    m: &mut Memory<'_>,
    _h: &mut impl Calls,
    ctx: u32,
    r1: u32,
    out1: u32,
    out2: u32,
) -> u32 {
    let _ = out1;
    wr(m, out2, 0);
    let r = fn_176f94(m, ctx, r1, rd(m, 0x1518F0), 0x14C);
    if r >= 0x80000000 {
        if r == 0x400000 {
            return rd(m, 0x1518F4);
        }
        return r;
    }
    let r = fn_19611c(m, rd(m, ctx), rd(m, ctx.wrapping_add(4)), out1);
    if r >= 0x80000000 {
        return r;
    }
    wr(m, out2, ctx);
    r
}

pub fn fn_15af34(
    m: &mut Memory<'_>,
    h: &mut impl Calls,
    a0: u32,
    a1: u32,
    a2: u32,
    scratch: u32,
) -> u32 {
    if a2 == 0 {
        return a0;
    }
    let r = route_or_host(m, &mut *h, 0x176F2C, &[rd(m, a1.wrapping_add(0x2c))]);
    if r >= 0x80000000 {
        wr(m, scratch, rd(m, 0x15B0B0));
        wr(m, scratch.wrapping_add(4), r);
        let logger = rd(m, a0.wrapping_add(0x10));
        return tts_core::cold_boot::fn_173dc0(
            m,
            &mut *h,
            logger,
            rd(m, 0x17EEE8),
            rd(m, 0x15B0C0),
            rd(m, 0x15B0CC),
            scratch,
            scratch.wrapping_add(4),
        );
    }
    wb(m, a1.wrapping_add(0x30), (a2 & 0xff) as u8);
    rd(m, a1.wrapping_add(0x2c))
}

pub fn fn_17ef1c(
    m: &mut Memory<'_>,
    h: &mut impl Calls,
    ctx: u32,
    s: u32,
    out: u32,
    slot: u32,
) -> u32 {
    if ctx == 0 || s == 0 {
        return rd(m, 0x17F218);
    }
    if rb(m, s) == 0 {
        return rd(m, 0x17F218);
    }
    let mut r6 = 0u32;
    wr(m, out, 0);

    loop {
        let count = rh(m, ctx.wrapping_add(0xc)) as u32;
        if r6 >= count {
            return rd(m, 0x17F21C);
        }
        wr(m, slot, 0);
        let e = rd(
            m,
            rd(m, ctx.wrapping_add(8)).wrapping_add(r6.wrapping_mul(4)),
        );
        let f = rd(m, rd(m, e.wrapping_add(8)).wrapping_add(0x1c));
        if f != 0 {
            let r = route_or_host(
                m,
                &mut *h,
                f,
                &[
                    rd(m, e.wrapping_add(0xc)),
                    rd(m, e.wrapping_add(0x10)),
                    slot,
                ],
            );
            if r < 0x80000000 && rd(m, slot) != 0 {
                let mut r4 = 0u32;
                loop {
                    if rd(m, out) != 0 {
                        break;
                    }
                    let cand = rd(m, rd(m, slot).wrapping_add(r4.wrapping_mul(4)));
                    if cand == 0 {
                        break;
                    }
                    let sb = strlen_bt(m, cand);
                    if fn_195df8(m, s, cand, sb) == 0 {
                        let b = rb(m, s.wrapping_add(sb)) as u32;
                        if b == 0 || b == 0x3b {
                            wr(m, out, e);
                        }
                    }
                    r4 = (r4.wrapping_add(1) & 0xffff) as u32;
                }
            }
        }
        r6 = (r6.wrapping_add(1) & 0xffff) as u32;
    }
}

#[allow(clippy::too_many_arguments)]
pub fn fn_17f03c(
    m: &mut Memory<'_>,
    h: &mut impl Calls,
    ctx: u32,
    sb2: u32,
    u7: u32,
    key: u32,
    p0: u32,
    p1: u32,
    f16: u32,
    f19: u32,
    out: u32,
) -> u32 {
    let sp = fn_129778(m, &mut *h, 0x34);
    if sp == 0 {
        return 0x8900200A;
    }
    tts_core::boot_part_d::fn_176fd4(m, sp, pair_176fd4(m));
    let (o0, o1) = (rd(m, sp), rd(m, sp.wrapping_add(4)));
    wr(m, sp.wrapping_add(0x20), o0);
    wr(m, sp.wrapping_add(0x24), o1);
    let sl = rd(m, 0x17F218);
    if ctx == 0 || u7 == 0 {
        return sl;
    }
    let r8 = rd(m, ctx);
    let mut key = key;
    if key == 0 {
        key = 0x17EF08;
    }
    if out != 0 {
        tts_core::boot_part_d::fn_176fd4(m, sp, pair_176fd4(m));
        let (q0, q1) = (rd(m, sp), rd(m, sp.wrapping_add(4)));
        wr(m, out, q0);
        wr(m, out.wrapping_add(4), q1);
    }
    let mut r4 = fn_17ef1c(
        m,
        &mut *h,
        ctx,
        u7,
        sp.wrapping_add(0x28),
        sp.wrapping_add(0x30),
    );
    if r4 >= 0x80000000 {
        wr(m, sp.wrapping_add(8), 0x17F220);
        wr(m, sp.wrapping_add(12), r4);
        wr(m, sp, 0x17F22C);
        wr(m, sp.wrapping_add(4), key);
        let logger = rd(m, r8.wrapping_add(0x10));
        tts_core::cold_boot::fn_173dc0(
            m,
            &mut *h,
            logger,
            rd(m, 0x17EEE8),
            rd(m, 0x17F23C),
            0x17F230,
            sp,
            sp.wrapping_add(4),
        );
        return r4;
    }
    let vres = rd(m, sp.wrapping_add(0x28));
    let ip = rd(m, rd(m, vres.wrapping_add(8)).wrapping_add(0x20));
    if ip == 0 {
        wr(m, sp.wrapping_add(0x10), 0x17F220);
        wr(m, sp.wrapping_add(0x14), r4);
        wr(m, sp.wrapping_add(8), 0x17F22C);
        wr(m, sp.wrapping_add(12), key);
        wr(m, sp, 0x17EEF4);
        wr(m, sp.wrapping_add(4), rd(m, vres.wrapping_add(4)));
        let logger = rd(m, r8.wrapping_add(0x10));
        tts_core::cold_boot::fn_173dc0(
            m,
            &mut *h,
            logger,
            rd(m, 0x17EEE8),
            rd(m, 0x17F274),
            0x17F264,
            sp,
            sp.wrapping_add(4),
        );
        return sl;
    }
    wr(m, sp.wrapping_add(0x10), sp.wrapping_add(0x20));
    wr(m, sp.wrapping_add(8), f16);
    wr(m, sp.wrapping_add(12), f19);
    wr(m, sp, p0);
    wr(m, sp.wrapping_add(4), p1);
    let (v0, v1) = (
        rd(m, vres.wrapping_add(0xc)),
        rd(m, vres.wrapping_add(0x10)),
    );
    r4 = route_or_host(
        m,
        &mut *h,
        ip,
        &[v0, v1, u7, key, p0, p1, f16, f19, sp.wrapping_add(0x20)],
    );
    if r4 < 0x80000000 {
        if out != 0 {
            let (c0, c1) = (rd(m, sp.wrapping_add(0x20)), rd(m, sp.wrapping_add(0x24)));
            wr(m, out, c0);
            wr(m, out.wrapping_add(4), c1);
        }
        return r4;
    }
    if sb2 == 0 {
        return log_17f164(m, h, r8, sp, key, u7, f16, r4);
    }
    if rd(m, 0x17F240) == r4 {
        return r4;
    }
    log_17f164(m, h, r8, sp, key, u7, f16, r4)
}

#[allow(clippy::too_many_arguments)]
fn log_17f164(
    m: &mut Memory<'_>,
    h: &mut impl Calls,
    r8: u32,
    sp: u32,
    key: u32,
    u7: u32,
    f16: u32,
    r4: u32,
) -> u32 {
    wr(m, sp.wrapping_add(0x18), 0x17F244);
    wr(m, sp.wrapping_add(0x1c), r4);
    wr(m, sp.wrapping_add(0x10), 0x17F220);
    wr(m, sp.wrapping_add(0x14), u7);
    wr(m, sp, 0x17EEF4);
    wr(
        m,
        sp.wrapping_add(4),
        rd(m, rd(m, sp.wrapping_add(0x28)).wrapping_add(4)),
    );
    wr(m, sp.wrapping_add(8), 0x17F22C);
    wr(m, sp.wrapping_add(12), key);
    let _ = f16;
    let logger = rd(m, r8.wrapping_add(0x10));
    tts_core::cold_boot::fn_173dc0(
        m,
        &mut *h,
        logger,
        rd(m, 0x17EEE8),
        rd(m, 0x17F260),
        0x17F24C,
        sp,
        sp.wrapping_add(4),
    );
    r4
}

fn tail_log_157080(m: &mut Memory<'_>, h: &mut impl Calls, ctx: u32, sp: u32, r4: u32) {
    let logger = rd(m, ctx.wrapping_add(0x10));
    wr(m, sp, 0x157950);
    wr(m, sp.wrapping_add(4), r4);
    let mut rh = RouteHost { inner: &mut *h };
    tts_core::log_live::fn_1738e8(
        m,
        logger,
        rd(m, 0x157904),
        4,
        0,
        0x157950,
        sp.wrapping_add(4),
        &mut rh,
    );
}

fn fail_157080(m: &mut Memory<'_>, h: &mut impl Calls, ctx: u32, sp: u32, r4: u32) -> u32 {
    let _ = route_or_host(m, &mut *h, 0x1520B0, &[ctx, sp.wrapping_add(0x14)]);
    wr(m, sp.wrapping_add(0x14), 0);
    wr(m, sp.wrapping_add(8), 0);
    tail_log_157080(m, h, ctx, sp, r4);
    r4
}

fn publish_157080(
    m: &mut Memory<'_>,
    h: &mut impl Calls,
    ctx: u32,
    sp: u32,
    out: u32,
    o: u32,
    r4: u32,
) -> u32 {
    wr(m, out, o);
    wr(m, out.wrapping_add(4), rd(m, 0x157A0C));

    tail_log_157080(m, h, ctx, sp, r4);
    r4
}

fn leases_157080(m: &mut Memory<'_>, h: &mut impl Calls, o: u32, sp: u32) {
    wr(m, o, rd(m, sp.wrapping_add(0x58)));
    wr(m, o.wrapping_add(4), rd(m, sp.wrapping_add(0x5C)));
    let _ = route_or_host(m, &mut *h, 0x176AC8, &[5, o.wrapping_add(0x1E), 10]);
    let _ = route_or_host(m, &mut *h, 0x195C90, &[o.wrapping_add(0x1E), 0x1574AC]);
    let r = route_or_host(m, &mut *h, 0x195D90, &[o.wrapping_add(0x1E)]);
    let t = r.wrapping_add(o).wrapping_add(0x1E);
    let _ = route_or_host(m, &mut *h, 0x176AC8, &[2, t, 10]);
    let _ = route_or_host(m, &mut *h, 0x195C90, &[t, 0x1574A4]);
    let r2 = route_or_host(m, &mut *h, 0x195D90, &[t]);
    let t2 = r2.wrapping_add(o).wrapping_add(0x1E);
    let _ = route_or_host(m, &mut *h, 0x176AC8, &[3, t2, 10]);
}

fn voice_slots_157080(
    m: &mut Memory<'_>,
    h: &mut impl Calls,
    ctx: u32,
    sp: u32,
    o: u32,
    logger: u32,
) -> u32 {
    let c14 = rd(m, ctx.wrapping_add(0x14));
    let p1 = route_or_host(m, &mut *h, 0x175944, &[c14, 0x1574AC, o.wrapping_add(0x30)]);
    if p1 >= 0x80000000 {
        wr(m, o.wrapping_add(0x30), 0x7FFF_FFFF);
        wr(m, o.wrapping_add(0x38), 0x7FFF_FFFF);
    } else {
        let _ = route_or_host(m, &mut *h, 0x175944, &[c14, 0x1574C0, o.wrapping_add(0x40)]);
        if rd(m, o.wrapping_add(0x30)).wrapping_add(rd(m, o.wrapping_add(0x40))) != 0 {
            let _ = route_or_host(m, &mut *h, 0x175944, &[c14, 0x1574D8, o.wrapping_add(0x38)]);
            let _ = route_or_host(m, &mut *h, 0x175944, &[c14, 0x1574F8, o.wrapping_add(0x44)]);
        }
    }
    let _ = route_or_host(m, &mut *h, 0x175944, &[c14, 0x15751C, o.wrapping_add(0x48)]);
    let _ = route_or_host(m, &mut *h, 0x175944, &[c14, 0x157530, o.wrapping_add(0x58)]);
    if rd(m, o.wrapping_add(0x48)).wrapping_add(rd(m, o.wrapping_add(0x58))) != 0 {
        let _ = route_or_host(m, &mut *h, 0x175944, &[c14, 0x157548, o.wrapping_add(0x50)]);
        let _ = route_or_host(m, &mut *h, 0x175944, &[c14, 0x1574FC, o.wrapping_add(0x5C)]);
    }
    let v = route_or_host(m, &mut *h, 0x15ACCC, &[c14]);
    let mut r8 = 0u32;
    if v != 0 {
        wr(m, o.wrapping_add(0x60), 2);
        wr(m, o.wrapping_add(0x68), 2);
        r8 = 2;
    }
    let fp = rd(m, 0x15756C);
    wr(m, sp.wrapping_add(0x44), 0);
    let mut r5 = 0u32;
    loop {
        let b = o.wrapping_add(r5.wrapping_mul(24));
        let s30 = rd(m, b.wrapping_add(0x30));
        let s40 = rd(m, b.wrapping_add(0x40));
        let sum = s30.wrapping_add(s40);
        r8 = s40.wrapping_add(s30.wrapping_add(r8));

        if sum != 0 && rd(m, b.wrapping_add(0x38)) == 0 {
            tts_core::cold_boot::fn_173dc0(
                m,
                &mut *h,
                logger,
                rd(m, 0x156624),
                r5.wrapping_add(0x3EEB),
                0,
                sp,
                sp,
            );
            wr(m, b.wrapping_add(0x38), sum);
        }

        if sum < rd(m, b.wrapping_add(0x38)) {
            tts_core::cold_boot::fn_173dc0(
                m,
                &mut *h,
                logger,
                rd(m, 0x156624),
                r5.wrapping_add(0x3EED),
                0,
                sp,
                sp,
            );
            wr(m, b.wrapping_add(0x38), sum);
        }
        if rd(m, b.wrapping_add(0x40)) != 0 && rd(m, b.wrapping_add(0x44)) < 0x1E {
            wr(m, sp, 0x157580);
            wr(m, sp.wrapping_add(4), 0x1E);
            wr(m, sp.wrapping_add(8), 0x157578);
            wr(m, sp.wrapping_add(0xC), 0x157570);
            tts_core::cold_boot::fn_173dc0(
                m,
                &mut *h,
                logger,
                rd(m, 0x156624),
                rd(m, 0x15758C),
                rd(m, 0x157588),
                sp,
                sp.wrapping_add(4),
            );
            wr(m, b.wrapping_add(0x44), 0x1E);
        }
        let str_c = if r5 == 0 { 0x157570 } else { 0x157574 };
        wr(m, sp.wrapping_add(0xC), rd(m, b.wrapping_add(0x10)));
        wr(m, sp.wrapping_add(8), rd(m, b.wrapping_add(0x30)));
        wr(m, sp.wrapping_add(4), str_c);
        wr(m, sp.wrapping_add(0x10), rd(m, b.wrapping_add(8)));
        wr(m, sp, fp);
        {
            let mut rh = RouteHost { inner: &mut *h };
            tts_core::log_live::fn_1738e8(
                m,
                logger,
                rd(m, 0x156624),
                4,
                0,
                fp,
                sp.wrapping_add(4),
                &mut rh,
            );
        }
        r5 = r5.wrapping_add(1) & 0xFF;
        if r5 > 1 {
            break;
        }
    }
    r8
}

fn magazine_157080(
    m: &mut Memory<'_>,
    h: &mut impl Calls,
    ctx: u32,
    sp: u32,
    logger: u32,
) -> Result<(), u32> {
    let c14 = rd(m, ctx.wrapping_add(0x14));
    let rc = route_or_host(
        m,
        &mut *h,
        0x175A1C,
        &[c14, 0x1578B0, sp.wrapping_add(0x28)],
    );
    let cell = rd(m, sp.wrapping_add(0x28));
    let live = rc < 0x80000000 && cell != 0 && rb(m, cell) != 0;
    if !live {
        wr(m, sp.wrapping_add(0x28), 0x1578C4);

        let scr = fn_129778(m, &mut *h, 0x100);
        if scr == 0 {
            return Err(0x8900200A);
        }
        let r = fn_1759a0(m, &mut *h, c14, 0x1578B0, 0x1578C4, scr);
        wr(m, sp.wrapping_add(0x44), r);
    } else {
        wr(m, sp, 0x1578D4);
        wr(m, sp.wrapping_add(4), cell);
        let mut rh = RouteHost { inner: &mut *h };
        tts_core::log_live::fn_1738e8(
            m,
            logger,
            rd(m, 0x157904),
            4,
            0,
            0x1578D4,
            sp.wrapping_add(4),
            &mut rh,
        );
    }
    let r44 = rd(m, sp.wrapping_add(0x44));
    if r44 >= 0x80000000 {
        return Err(r44);
    }
    let _ = route_or_host(
        m,
        &mut *h,
        0x175A1C,
        &[
            rd(m, ctx.wrapping_add(0x14)),
            0x1578B0,
            sp.wrapping_add(0x40),
        ],
    );
    Ok(())
}

fn register_157080(
    m: &mut Memory<'_>,
    h: &mut impl Calls,
    ctx: u32,
    sp: u32,
    out: u32,
    o: u32,
    logger: u32,
    r4: u32,
    s0: u32,
    s1: u32,
) -> u32 {
    wr(m, sp, o.wrapping_add(8));
    wr(m, sp.wrapping_add(4), o.wrapping_add(0x10));

    let rc = route_or_host(
        m,
        &mut *h,
        0x177E0C,
        &[
            rd(m, ctx.wrapping_add(0xC)),
            0x157938,
            1,
            1,
            o.wrapping_add(8),
            o.wrapping_add(0x10),
        ],
    );
    if rc >= 0x80000000 {
        return publish_157080(m, h, ctx, sp, out, o, r4);
    }
    wr(m, sp, o.wrapping_add(0x14));
    let slot = rd(m, o.wrapping_add(0x10));
    let target = rd(m, slot.wrapping_add(0x10));
    let r = route_or_host(
        m,
        &mut *h,
        target,
        &[
            rd(m, o.wrapping_add(8)),
            rd(m, o.wrapping_add(0xC)),
            s0,
            s1,
            o.wrapping_add(0x14),
            rd(m, sp.wrapping_add(4)),
            rd(m, sp.wrapping_add(8)),
            rd(m, sp.wrapping_add(0xC)),
        ],
    );
    if r >= 0x80000000 {
        return fail_157080(m, h, ctx, sp, r);
    }
    if rd(m, ctx.wrapping_add(0x18)) == 0 {
        return publish_157080(m, h, ctx, sp, out, o, r);
    }
    let _ = fn_195bec(m, sp.wrapping_add(0x2C), 0, 0x14);
    wb(m, sp.wrapping_add(0x2C), 1);
    wr(m, sp.wrapping_add(0x30), rd(m, o.wrapping_add(0x10)));
    wr(m, sp.wrapping_add(0x34), rd(m, o.wrapping_add(0x14)));
    wr(m, sp.wrapping_add(0x38), rd(m, o.wrapping_add(0x18)));
    wr(m, sp.wrapping_add(0x3C), 0);
    let r2 = route_or_host(
        m,
        &mut *h,
        0x17B0F8,
        &[
            rd(m, ctx.wrapping_add(0x18)),
            0x157928,
            sp.wrapping_add(0x2C),
        ],
    );
    if r2 < 0x80000000 {
        wb(m, o.wrapping_add(0x1C), 1);
        return publish_157080(m, h, ctx, sp, out, o, r2);
    }
    wr(m, sp, 0x157494);
    wr(m, sp.wrapping_add(4), r2);
    tts_core::cold_boot::fn_173dc0(
        m,
        &mut *h,
        logger,
        rd(m, 0x157904),
        rd(m, 0x15794C),
        0x157944,
        sp,
        sp.wrapping_add(4),
    );
    fail_157080(m, h, ctx, sp, r2)
}

pub fn fn_157080(m: &mut Memory<'_>, h: &mut impl Calls, a0: u32, a1: u32, out: u32) -> u32 {
    let sp = fn_129778(m, &mut *h, 0x88);
    if sp == 0 {
        return 0x8900200A;
    }

    wr(m, sp.wrapping_add(0x58), a0);
    wr(m, sp.wrapping_add(0x5C), a1);
    wr(m, sp.wrapping_add(0x18), 0);
    wr(m, sp.wrapping_add(0x14), 0);
    wr(m, sp.wrapping_add(0x1C), 0);
    wr(m, sp.wrapping_add(0x40), 0);
    if out == 0 {
        return rd(m, 0x156604);
    }

    tts_core::boot_part_d::fn_176fd4(m, sp, pair_176fd4(m));
    wr(m, out, rd(m, sp));
    wr(m, out.wrapping_add(4), rd(m, sp.wrapping_add(4)));
    let r = fn_19611c(m, a0, a1, sp.wrapping_add(0x18));
    if r >= 0x80000000 {
        return r;
    }
    let ctx = rd(m, sp.wrapping_add(0x18));
    let logger = rd(m, ctx.wrapping_add(0x10));
    wr(m, sp, 0x157478);
    {
        let mut rh = RouteHost { inner: &mut *h };
        tts_core::log_live::fn_1738e8(
            m,
            logger,
            rd(m, 0x156624),
            4,
            0,
            0x157478,
            sp.wrapping_add(4),
            &mut rh,
        );
    }
    let heap = rd(m, ctx.wrapping_add(4));
    let o = fn_1963b8(m, &mut *h, heap, 1, 0x98);
    if o == 0 {
        tts_core::cold_boot::fn_173dc0(m, &mut *h, logger, rd(m, 0x156624), 0x3E80, 0, sp, sp);
        return rd(m, 0x1574A0);
    }
    wr(m, sp.wrapping_add(0x14), o);
    let mut r4 = fn_176e40(
        m,
        &mut *h,
        rd(m, ctx.wrapping_add(8)),
        heap,
        o.wrapping_add(0x90),
    );
    if r4 >= 0x80000000 {
        wr(m, sp, 0x157494);
        wr(m, sp.wrapping_add(4), r4);
        tts_core::cold_boot::fn_173dc0(
            m,
            &mut *h,
            logger,
            rd(m, 0x156624),
            rd(m, 0x15749C),
            0x15705C,
            sp,
            sp.wrapping_add(4),
        );
        return fail_157080(m, h, ctx, sp, r4);
    }
    leases_157080(m, h, o, sp);
    if rd(m, ctx.wrapping_add(0x14)) == 0 {
        r4 = route_or_host(m, &mut *h, 0x174CA8, &[0, a0, a1, ctx.wrapping_add(0x14)]);
        if r4 >= 0x80000000 {
            wr(m, sp, 0x157494);
            wr(m, sp.wrapping_add(4), r4);
            tts_core::cold_boot::fn_173dc0(
                m,
                &mut *h,
                logger,
                rd(m, 0x156624),
                rd(m, 0x1574A8),
                0x15705C,
                sp,
                sp.wrapping_add(4),
            );
            return fail_157080(m, h, ctx, sp, r4);
        }
        wb(m, o.wrapping_add(0x1D), 1);
    }
    let r8 = voice_slots_157080(m, h, ctx, sp, o, logger);
    if r8 == 0 {
        tts_core::cold_boot::fn_173dc0(
            m,
            &mut *h,
            logger,
            rd(m, 0x156620),
            rd(m, 0x1578AC),
            0,
            sp,
            sp,
        );
    }
    if let Err(e) = magazine_157080(m, h, ctx, sp, logger) {
        return fail_157080(m, h, ctx, sp, e);
    }
    let rc = route_or_host(m, h, 0x152D48, &[ctx, o, 4, 3]);
    wb(m, o + 0x94, u8::from(rc >= 0x80000000));
    for (feature, mode) in [(0, 0), (3, 1), (0, 2)] {
        let rc = route_or_host(m, h, 0x152D48, &[ctx, o, feature, mode]);
        if rc >= 0x80000000 {
            return fail_157080(m, h, ctx, sp, rc);
        }
    }

    if rd(m, ctx.wrapping_add(0x18)) != 0 {
        let q = fn_17b058(
            m,
            &mut *h,
            rd(m, ctx.wrapping_add(0x18)),
            0x157928,
            sp.wrapping_add(0x1C),
        );

        if q < 0x80000000 {
            let cell = rd(m, sp.wrapping_add(0x1C));
            wr(m, o.wrapping_add(0x10), rd(m, cell.wrapping_add(4)));
            wr(m, o.wrapping_add(0x14), rd(m, cell.wrapping_add(8)));
            wr(m, o.wrapping_add(0x18), rd(m, cell.wrapping_add(0xC)));
            return publish_157080(m, h, ctx, sp, out, o, r4);
        }
        return register_157080(m, h, ctx, sp, out, o, logger, r4, a0, a1);
    }
    register_157080(m, h, ctx, sp, out, o, logger, r4, a0, a1)
}

pub fn fn_15f164(
    m: &mut Memory<'_>,
    h: &mut impl Calls,
    _a0: u32,
    _a1: u32,
    a2: u32,
    a3: u32,
    a4: u32,
) -> u32 {
    let sp = fn_129778(m, &mut *h, 0x28);
    if sp == 0 {
        return 0x8900200A;
    }
    let r5 = a4;
    wr(m, sp.wrapping_add(8), 0);
    if r5 == 0 {
        return rd(m, 0x15F398);
    }
    wr(m, r5, 0);
    wr(m, r5.wrapping_add(4), 0);
    let r = fn_19611c(m, a2, a3, sp.wrapping_add(8));
    if r >= 0x80000000 {
        return r;
    }
    let ctx = rd(m, sp.wrapping_add(8));
    let logger = rd(m, ctx.wrapping_add(0x10));
    wr(m, sp, 0x15F39C);
    {
        let mut rh = RouteHost { inner: &mut *h };
        tts_core::log_live::fn_1738e8(
            m,
            logger,
            rd(m, 0x15F034),
            4,
            0,
            0x15F39C,
            sp.wrapping_add(4),
            &mut rh,
        );
    }
    let r4 = fn_1963b8(m, &mut *h, rd(m, ctx.wrapping_add(4)), 1, 0x20);
    if r4 == 0 {
        tts_core::cold_boot::fn_173dc0(
            m,
            &mut *h,
            logger,
            rd(m, 0x15F034),
            rd(m, 0x15F3EC),
            0,
            sp,
            sp,
        );

        return join_15f164(m, h, ctx, sp, logger, r5, r4, 0, rd(m, 0x15F3F0));
    }
    wr(m, r4, a2);
    wr(m, r4.wrapping_add(4), a3);
    wr(m, r4.wrapping_add(8), ctx);
    let q = route_or_host(
        m,
        &mut *h,
        0x175944,
        &[
            rd(m, ctx.wrapping_add(0x14)),
            0x15F3B8,
            r4.wrapping_add(0x14),
        ],
    );
    if q < 0x80000000 {
        wr(m, sp, 0x15F3D0);
        wr(m, sp.wrapping_add(4), rd(m, r4.wrapping_add(0x14)));
        let mut rh = RouteHost { inner: &mut *h };
        tts_core::log_live::fn_1738e8(
            m,
            logger,
            rd(m, 0x15F034),
            4,
            0,
            0x15F3D0,
            sp.wrapping_add(4),
            &mut rh,
        );
    }
    let mut r7 = fn_176e40(
        m,
        &mut *h,
        rd(m, ctx.wrapping_add(8)),
        rd(m, ctx.wrapping_add(4)),
        r4.wrapping_add(0x1C),
    );
    if r7 < 0x80000000 {
        r7 = route_or_host(
            m,
            &mut *h,
            0x177DD8,
            &[rd(m, ctx.wrapping_add(0xC)), r4.wrapping_add(0xC)],
        );
        if r7 < 0x80000000 {
            r7 = fn_177b7c(
                m,
                &mut *h,
                0,
                rd(m, r4.wrapping_add(0xC)),
                r4.wrapping_add(0x10),
                sp.wrapping_add(0x10),
            );
            if r7 < 0x80000000 {
                wr(m, r5, r4);
                wr(m, r5.wrapping_add(4), 0x15C);
                return tail_15f164(m, h, ctx, sp, logger, r7);
            }
        }
    }

    join_15f164(m, h, ctx, sp, logger, r5, r4, 0x15C, r7)
}

fn join_15f164(
    m: &mut Memory<'_>,
    h: &mut impl Calls,
    ctx: u32,
    sp: u32,
    logger: u32,
    r5: u32,
    r4: u32,
    r8store: u32,
    tail_r7: u32,
) -> u32 {
    wr(m, r5, r4);
    wr(m, r5.wrapping_add(4), r8store);
    let r8 = r4;
    let rc = fn_176f94(m, r4, 0x15C, 0x15C, 0x20);
    if rc >= 0x80000000 {
        wr(m, r5, 0);
        wr(m, r5.wrapping_add(4), 0);
        return tail_15f164(m, h, ctx, sp, logger, tail_r7);
    }
    let r6 = r8;
    let b8 = rd(m, r6.wrapping_add(8));
    wr(m, sp, 0x15F128);
    {
        let mut rh = RouteHost { inner: &mut *h };
        tts_core::log_live::fn_1738e8(
            m,
            rd(m, b8.wrapping_add(0x10)),
            rd(m, 0x15F034),
            4,
            0,
            0x15F128,
            sp.wrapping_add(4),
            &mut rh,
        );
    }
    let mut r4rc = rc;
    if rd(m, r6.wrapping_add(0x10)) != 0 {
        let q = route_or_host(m, &mut *h, 0x177CC8, &[rd(m, r6.wrapping_add(0x10))]);
        if q >= 0x80000000 {
            r4rc = q;
        }
    }
    if rd(m, r6.wrapping_add(0x1C)) != 0 {
        let q = route_or_host(m, &mut *h, 0x176EDC, &[rd(m, r6.wrapping_add(0x1C))]);
        wr(m, r6.wrapping_add(0x1C), 0);
        if r4rc < 0x80000000 {
            r4rc = q;
        }
    }
    let _ = fn_196428(m, &mut *h, rd(m, b8.wrapping_add(4)), r6);
    wr(m, sp, 0x15F144);
    wr(m, sp.wrapping_add(4), r4rc);
    {
        let mut rh = RouteHost { inner: &mut *h };
        tts_core::log_live::fn_1738e8(
            m,
            rd(m, b8.wrapping_add(0x10)),
            rd(m, 0x15F034),
            4,
            0,
            0x15F144,
            sp.wrapping_add(4),
            &mut rh,
        );
    }
    tail_15f164(m, h, ctx, sp, logger, r4rc)
}

fn tail_15f164(
    m: &mut Memory<'_>,
    h: &mut impl Calls,
    ctx: u32,
    sp: u32,
    logger: u32,
    r7: u32,
) -> u32 {
    let _ = ctx;
    wr(m, sp, 0x15F3F4);
    wr(m, sp.wrapping_add(4), r7);
    let mut rh = RouteHost { inner: &mut *h };
    tts_core::log_live::fn_1738e8(
        m,
        logger,
        rd(m, 0x15F034),
        4,
        0,
        0x15F3F4,
        sp.wrapping_add(4),
        &mut rh,
    );
    r7
}

pub fn fn_195d58(m: &Memory<'_>, a: u32, b: u32) -> u32 {
    let mut i = 0u32;
    loop {
        let x = rb(m, a.wrapping_add(i));
        let y = rb(m, b.wrapping_add(i));
        if x != y {
            return 1;
        }
        if x == 0 {
            return 0;
        }
        i = i.wrapping_add(1);
    }
}

pub fn fn_14d6dc(m: &mut Memory<'_>, h: &mut impl Calls, r5: u32) -> u32 {
    if r5 == 0 {
        return 0;
    }
    let mut r4 = 0u32;
    loop {
        let r6 = r5.wrapping_add(r4.wrapping_mul(20));
        if rd(m, r6.wrapping_add(0x14)) != 0 {
            let q = route_or_host(
                m,
                &mut *h,
                0x178530,
                &[rd(m, rd(m, r5.wrapping_add(8)).wrapping_add(0xC))],
            );
            if q >= 0x80000000 {
                return q;
            }
            wr(m, r6.wrapping_add(0x14), 0);
        }
        r4 = r4.wrapping_add(1) & 0xFF;
        if r4 >= 5 {
            break;
        }
    }
    let sp = fn_129778(m, &mut *h, 0x18);
    if sp == 0 {
        return 0x80000000;
    }
    let r = fn_19611c(m, rd(m, r5), rd(m, r5.wrapping_add(4)), sp);
    if r >= 0x80000000 {
        return r;
    }
    fn_196428(m, &mut *h, rd(m, rd(m, sp).wrapping_add(4)), r5)
}

pub fn fn_14d764(
    m: &mut Memory<'_>,
    h: &mut impl Calls,
    a0: u32,
    a1: u32,
    _a2: u32,
    out: u32,
) -> u32 {
    let sp = fn_129778(m, &mut *h, 0xC0);
    if sp == 0 {
        return 0x8900200A;
    }
    wr(m, sp.wrapping_add(0x90), 0);
    if out == 0 {
        return rd(m, 0x14D990);
    }
    let r = fn_19611c(m, a0, a1, sp.wrapping_add(0x90));
    let sb = rd(m, 0x14D98C);
    if r >= 0x80000000 {
        return sb;
    }
    wr(m, out, 0);
    let ctx = rd(m, sp.wrapping_add(0x90));
    let r5 = fn_1963b8(m, &mut *h, rd(m, ctx.wrapping_add(4)), 1, 0x70);
    if r5 == 0 {
        return rd(m, 0x14D994);
    }
    let _ = fn_195bec(m, r5, 0, 0x70);
    wr(m, r5.wrapping_add(4), a1);
    wr(m, r5, a0);
    wr(m, r5.wrapping_add(8), ctx);
    wr(m, sp.wrapping_add(0x98), 0);
    wr(m, sp.wrapping_add(0x94), 0);
    wr(m, sp.wrapping_add(4), 0);
    wr(m, sp.wrapping_add(0xC), 0);
    wr(m, sp.wrapping_add(0x9C), 0);
    let r = fn_19611c(m, a0, a1, sp.wrapping_add(0x98));
    if r >= 0x80000000 {
        fn_14d6dc(m, h, r5);
        return sb;
    }
    let ctx2 = rd(m, sp.wrapping_add(0x98));
    if rd(m, ctx2.wrapping_add(0x14)) == 0 {
        fn_14d6dc(m, h, r5);
        return sb;
    }
    wr(m, sp, sp.wrapping_add(0x9C));

    let q = route_or_host(
        m,
        &mut *h,
        0x17A3B8,
        &[
            rd(m, rd(m, r5.wrapping_add(8)).wrapping_add(0xC)),
            0x14D99C,
            0x14D998,
            1,
            sp.wrapping_add(0x9C),
        ],
    );
    if q != 0 {
        return publish_14d764(m, h, r5, out);
    }
    if rd(m, sp.wrapping_add(0x9C)) == 0 {
        return publish_14d764(m, h, r5, out);
    }

    scan_14d764(m, h, r5, out, sp)
}

fn copy_14d764(m: &mut Memory<'_>, h: &mut impl Calls, r5: u32, out: u32, sp: u32) -> u32 {
    wr(m, sp.wrapping_add(4), 0x14DC0C);
    let _ = route_or_host(
        m,
        &mut *h,
        0x195CD0,
        &[sp.wrapping_add(0x10), rd(m, sp.wrapping_add(0xC))],
    );
    let _ = route_or_host(m, &mut *h, 0x195C90, &[sp.wrapping_add(0x10), 0x14DC20]);
    wr(m, sp.wrapping_add(0xC), r5);

    if magazine_14d764(m, h, r5, out, sp) {
        return scan_14d764(m, h, r5, out, sp);
    }
    fail_17a2ec_14d764(m, h, r5, out, sp)
}

fn scan_14d764(m: &mut Memory<'_>, h: &mut impl Calls, r5: u32, out: u32, sp: u32) -> u32 {
    loop {
        let q = route_or_host(
            m,
            &mut *h,
            0x17A814,
            &[
                rd(m, sp.wrapping_add(0x9C)),
                sp.wrapping_add(4),
                sp.wrapping_add(0xC),
                0,
            ],
        );
        if q >= 0x80000000 {
            return fail_17a2ec_14d764(m, h, r5, out, sp);
        }
        if rd(m, sp.wrapping_add(4)) == 0 {
            continue;
        }
        if rd(m, sp.wrapping_add(0xC)) == 0 {
            continue;
        }
        let s = rd(m, sp.wrapping_add(4));
        if fn_195d58(m, s, 0x14D9A4) == 0 {
            continue;
        }

        let mut store: Option<u32> = None;
        let mut copy = false;
        if fn_195d58(m, s, 0x14D9B4) == 0 {
            store = Some(0x14D9C4);
        } else if fn_195d58(m, s, 0x14D9D0) == 0 {
            store = Some(0x14D9E4);
        } else if fn_195d58(m, s, 0x14DA00) == 0 {
            store = Some(0x14DA18);
        } else if fn_195d58(m, s, 0x14DA28) == 0 {
            copy = true;
        } else if fn_195d58(m, s, 0x14DA48) == 0 {
            store = Some(0x14DA5C);
        } else if fn_195d58(m, s, 0x14DA74) == 0 {
            store = Some(0x14DA8C);
        } else if fn_195d58(m, s, 0x14DAA8) == 0 {
            store = Some(0x14DAB4);
        } else if fn_195d58(m, s, 0x14DAC0) == 0 {
            store = Some(0x14DAD4);
        } else if fn_195d58(m, s, 0x14DAE4) == 0 {
            store = Some(0x14DAF4);
        }
        if copy {
            return copy_14d764(m, h, r5, out, sp);
        }
        if let Some(v) = store {
            wr(m, sp.wrapping_add(4), v);
        }
        if magazine_14d764(m, h, r5, out, sp) {
            continue;
        }
        return fail_17a2ec_14d764(m, h, r5, out, sp);
    }
}

fn magazine_14d764(m: &mut Memory<'_>, h: &mut impl Calls, _r5: u32, _out: u32, sp: u32) -> bool {
    let ctx14 = rd(m, rd(m, sp.wrapping_add(0x98)).wrapping_add(0x14));
    let q = route_or_host(
        m,
        &mut *h,
        0x175A1C,
        &[ctx14, rd(m, sp.wrapping_add(4)), sp.wrapping_add(0x94)],
    );
    if q < 0x80000000 {
        let _ = route_or_host(m, &mut *h, 0x1757A8, &[rd(m, sp.wrapping_add(0x94))]);
        return true;
    }
    let q = route_or_host(
        m,
        &mut *h,
        0x1759A0,
        &[
            ctx14,
            rd(m, sp.wrapping_add(4)),
            rd(m, sp.wrapping_add(0xC)),
        ],
    );
    q < 0x80000000
}

fn fail_17a2ec_14d764(m: &mut Memory<'_>, h: &mut impl Calls, r5: u32, out: u32, sp: u32) -> u32 {
    let _ = route_or_host(m, &mut *h, 0x17A2EC, &[rd(m, sp.wrapping_add(0x9C))]);
    publish_14d764(m, h, r5, out)
}

fn publish_14d764(m: &mut Memory<'_>, h: &mut impl Calls, r5: u32, out: u32) -> u32 {
    let r8 = rd(m, 0x14DC2C);
    let mut r4 = 0u32;
    loop {
        let e = rd(m, r8.wrapping_add(r4.wrapping_mul(4)));
        if e != 0 {
            let b = r5.wrapping_add(r4.wrapping_mul(5).wrapping_mul(4));
            let _ = route_or_host(
                m,
                &mut *h,
                0x177E0C,
                &[
                    rd(m, rd(m, r5.wrapping_add(8)).wrapping_add(0xC)),
                    e,
                    1,
                    1,
                    b.wrapping_add(0x18),
                    b.wrapping_add(0x14),
                ],
            );
        }
        r4 = r4.wrapping_add(1) & 0xFF;
        if r4 >= 5 {
            break;
        }
    }
    wr(m, out, r5);
    0
}

pub fn fn_17d168(m: &mut Memory<'_>, h: &mut impl Calls, a0: u32, a1: u32, a2: u32) -> u32 {
    let sp = fn_129778(m, &mut *h, 0x30);
    if sp == 0 {
        return 0x8900200A;
    }
    wr(m, sp.wrapping_add(0xC), 0);
    wr(m, sp.wrapping_add(8), 0);
    wr(m, sp.wrapping_add(0x10), rd(m, 0x17D31C));
    wr(m, sp.wrapping_add(0x14), rd(m, 0x17D320));
    wr(m, sp.wrapping_add(0x18), rd(m, 0x17D324));

    if a2 == 0 || a1 == 0 {
        return rd(m, 0x17D328);
    }
    {
        let mut bytes = Vec::new();
        for i in 0..80u32 {
            let b = rb(m, a2.wrapping_add(i));
            if b == 0 {
                break;
            }
            bytes.push(b);
        }
    }
    let r3 = if fn_195df8(m, a2, 0x17D32C, 5) == 0 {
        a2
    } else {
        let q = route_or_host(m, &mut *h, 0x19600C, &[a2, 0x17D334]);
        if q == 0 {
            a2
        } else {
            let mut p = q;
            loop {
                let b = rb(m, p);
                if b != 0xA && b != 0xD {
                    break;
                }
                p = p.wrapping_add(1);
            }
            p
        }
    };
    wr(m, sp, sp.wrapping_add(0xC));
    wr(m, sp.wrapping_add(4), sp.wrapping_add(8));

    let mut r4 = route_or_host(
        m,
        &mut *h,
        0x197C78,
        &[
            rd(m, a1),
            rd(m, a1.wrapping_add(4)),
            0,
            r3,
            sp.wrapping_add(0xC),
            sp.wrapping_add(8),
        ],
    );
    if r4 >= 0x80000000 {
        tts_core::cold_boot::fn_173dc0(
            m,
            &mut *h,
            rd(m, a0.wrapping_add(0x10)),
            rd(m, 0x17D338),
            0x114,
            0,
            sp,
            sp.wrapping_add(4),
        );
        return r4;
    }

    if rd(m, sp.wrapping_add(8)) != 0 {
        let mut r5 = 0u32;
        loop {
            let q = route_or_host(m, &mut *h, 0x1970AC, &[rd(m, sp.wrapping_add(8)), r5, sp]);

            if q == 0 {
                break;
            }
            let cell = rd(m, sp);
            if r4 >= 0x80000000 {
                r4 = route_or_host(m, &mut *h, 0x19665C, &[rd(m, cell)]);
            } else {
                r4 = route_or_host(
                    m,
                    &mut *h,
                    0x177124,
                    &[sp.wrapping_add(0x10), rd(m, cell), a1],
                );
            }
            r5 = r5.wrapping_add(1);
        }
        let _ = route_or_host(m, &mut *h, 0x196C7C, &[rd(m, sp.wrapping_add(8))]);
        wr(m, sp.wrapping_add(8), 0);
    } else {
        r4 = route_or_host(
            m,
            &mut *h,
            0x177124,
            &[sp.wrapping_add(0x10), rd(m, sp.wrapping_add(0xC)), a1],
        );
    }
    if r4 >= 0x80000000 {
        return r4;
    }

    r4 = route_or_host(m, &mut *h, 0x176F2C, &[rd(m, a1.wrapping_add(0x1C))]);
    if r4 >= 0x80000000 {
        return r4;
    }
    r4 = route_or_host(m, &mut *h, 0x196764, &[rd(m, a1.wrapping_add(0x10))]);
    if r4 == 0 {
        r4 = rd(m, 0x17D33C);
    }
    let q = route_or_host(m, &mut *h, 0x176F5C, &[rd(m, a1.wrapping_add(0x1C))]);
    if q < 0x80000000 {
        return r4;
    }
    if r4 < 0x80000000 {
        r4 = q;
    }
    r4
}

pub fn fn_196764(m: &Memory<'_>, a0: u32) -> u32 {
    if a0 == 0 {
        0
    } else {
        rd(m, a0.wrapping_add(0x30))
    }
}

pub fn fn_19665c(m: &mut Memory<'_>, h: &mut impl Calls, a0: u32) -> u32 {
    if a0 == 0 {
        return rd(m, 0x196A54);
    }

    let _ = route_or_host(m, &mut *h, 0x19869C, &[a0.wrapping_add(0x18)]);
    let _ = fn_196428(m, &mut *h, rd(m, a0.wrapping_add(0x14)), a0);
    0
}

pub fn fn_176edc(m: &mut Memory<'_>, h: &mut impl Calls, a0: u32) -> u32 {
    if a0 == 0 {
        return rd(m, 0x176F8C);
    }
    let obj = rd(m, rd(m, a0).wrapping_add(8));
    let target = rd(m, obj.wrapping_add(4));
    if target != 0 {
        let r = route_or_host(m, &mut *h, target, &[rd(m, a0.wrapping_add(8))]);
        if r >= 0x80000000 {
            return r;
        }
    }
    let _ = fn_196428(m, &mut *h, rd(m, a0.wrapping_add(4)), a0);
    0
}

pub fn fn_4a774c(
    m: &mut Memory<'_>,
    h: &mut impl Calls,
    a0: u32,
    a1: u32,
    a2: u32,
    a3: u32,
) -> u32 {
    let target = rd(m, rd(m, a0).wrapping_add(0xC));
    route_or_host(m, &mut *h, target, &[a0, a1, a2, a3])
}

pub fn fn_17ad10(m: &mut Memory<'_>, h: &mut impl Calls, a0: u32, a1: u32) -> u32 {
    if a1 == 0 {
        return a0;
    }
    if rd(m, a1.wrapping_add(8)) != 0 {
        let _ = fn_196428(m, &mut *h, a0, rd(m, a1.wrapping_add(8)));
    }
    if rd(m, a1.wrapping_add(0x10)) != 0 {
        let count = rh(m, a1.wrapping_add(0xC)) as i16 as i32;
        if count > 0 {
            let mut r4 = 0u32;
            loop {
                let e = rd(
                    m,
                    rd(m, a1.wrapping_add(0x10)).wrapping_add(r4.wrapping_mul(4)),
                );
                let _ = fn_196428(m, &mut *h, a0, e);
                r4 = r4.wrapping_add(1);
                if (rh(m, a1.wrapping_add(0xC)) as i16 as i32) <= r4 as i32 {
                    break;
                }
            }
        }
        let _ = fn_196428(m, &mut *h, a0, rd(m, a1.wrapping_add(0x10)));
    }
    if rd(m, a1.wrapping_add(0x14)) != 0 {
        let _ = fn_196428(m, &mut *h, a0, rd(m, a1.wrapping_add(0x14)));
    }
    let _ = fn_196428(m, &mut *h, a0, a1);
    0
}

pub fn fn_19600c(m: &Memory<'_>, a: u32, charset: u32) -> u32 {
    let mut bm = [0u8; 32];
    let mut i = 0u32;
    loop {
        let c = rb(m, charset.wrapping_add(i));
        if c == 0 {
            break;
        }
        bm[(c as usize) >> 3] |= 1 << ((c as usize) & 7);
        i = i.wrapping_add(1);
    }
    let mut j = 0u32;
    loop {
        let c = rb(m, a.wrapping_add(j));
        if c == 0 {
            return 0;
        }
        if bm[(c as usize) >> 3] & (1 << ((c as usize) & 7)) != 0 {
            return a;
        }
        j = j.wrapping_add(1);
    }
}

pub fn fn_177124(m: &mut Memory<'_>, h: &mut impl Calls, a0: u32, a1: u32, a2: u32) -> u32 {
    let sp = fn_129778(m, &mut *h, 0x18);
    if sp == 0 {
        return 0x8900200A;
    }

    let r8 = a0;
    let r5 = a2;
    let mut r4 = a1;
    wr(m, sp, 0);
    let mut r6 = 0u32;

    let r = route_or_host(m, &mut *h, 0x1966AC, &[a1, 0x177264, sp]);
    let r7;
    if r >= 0x80000000 {
        r7 = rd(m, 0x17725C);
    } else {
        r7 = 0;
        if rd(m, sp) != 0 {
            r6 = rd(m, rd(m, sp));
        }
    }
    if r7 >= 0x80000000 {
        let _ = route_or_host(m, &mut *h, 0x198280, &[a1]);
        return r7;
    }
    if a1 == 0 {
        r4 = rd(m, 0x177274);
        return r4;
    }
    let r7 = rd(m, r5.wrapping_add(8));
    let _ = fn_195bec(m, sp.wrapping_add(8), 0, 0xC);
    wr(m, sp.wrapping_add(8), r4);
    wr(m, sp.wrapping_add(0xC), r6);
    wr(m, sp, 0x177278);
    wr(m, sp.wrapping_add(4), r6);
    {
        let mut rh = RouteHost { inner: &mut *h };
        tts_core::log_live::fn_1738e8(
            m,
            rd(m, r7.wrapping_add(0x10)),
            rd(m, 0x177298),
            4,
            0,
            0x177278,
            sp.wrapping_add(4),
            &mut rh,
        );
    }
    r4 = route_or_host(m, &mut *h, 0x176F2C, &[rd(m, r5.wrapping_add(0x1C))]);

    if r4 < 0x80000000 {
        r4 = route_or_host(
            m,
            &mut *h,
            0x19668C,
            &[
                rd(m, r5.wrapping_add(0x10)),
                rd(m, sp.wrapping_add(0xC)),
                sp.wrapping_add(8),
            ],
        );

        if r4 >= 0x80000000 {
            wr(m, sp, 0x17729C);
            wr(m, sp.wrapping_add(4), r8);
            tts_core::cold_boot::fn_173dc0(
                m,
                &mut *h,
                rd(m, r7.wrapping_add(0x10)),
                rd(m, 0x1772AC),
                rd(m, 0x177298),
                0x1772A4,
                sp,
                sp.wrapping_add(4),
            );
        }
    }
    let q = route_or_host(m, &mut *h, 0x176F5C, &[rd(m, r5.wrapping_add(0x1C))]);
    if q >= 0x80000000 {
        if r4 < 0x80000000 {
            r4 = q;
        }
    }
    if rd(m, sp.wrapping_add(8)) != 0 {
        let _ = route_or_host(m, &mut *h, 0x198280, &[rd(m, sp.wrapping_add(8))]);
    }

    if r4.wrapping_shl(19) == 0x100000 {
        r4 = 0;
    }
    r4
}

pub fn fn_19869c(m: &mut Memory<'_>, h: &mut impl Calls, a0: u32) -> u32 {
    let r4 = a0;
    let r5 = rd(m, a0);
    if r5 != r4 {
        let _ = route_or_host(m, &mut *h, 0x198FB4, &[r4, rd(m, r5), r4]);
        let _ = route_or_host(m, &mut *h, 0x198FB4, &[r4, rd(m, r5.wrapping_add(4)), r4]);

        if rd(m, r5.wrapping_add(0x10)) != 0 {
            let vt = rd(m, r4.wrapping_add(0x24));
            let _ = route_or_host(
                m,
                &mut *h,
                rd(m, vt.wrapping_add(4)),
                &[rd(m, r5.wrapping_add(0x10)), rd(m, r4.wrapping_add(0x20))],
            );
        }
        if rd(m, r5.wrapping_add(0x14)) != 0 {
            let vt = rd(m, r4.wrapping_add(0x24));
            let _ = route_or_host(
                m,
                &mut *h,
                rd(m, vt.wrapping_add(0x10)),
                &[rd(m, r5.wrapping_add(0x14)), rd(m, r4.wrapping_add(0x20))],
            );
        }
        let _ = fn_196428(m, &mut *h, rd(m, r4.wrapping_add(0x20)), r5);
    }
    wr(m, r4.wrapping_add(0x18), 0);
    wr(m, r4, r4);
    wr(m, r4.wrapping_add(4), r4);
    0
}

pub fn fn_15ae20(
    m: &mut Memory<'_>,
    _h: &mut impl Calls,
    a: u32,
    b: u32,
    ctx: u32,
    lic: u32,
) -> u32 {
    wr(m, lic, 0);
    let r = fn_176f94(m, a, b, rd(m, 0x15B0AC), 0x98);
    if r >= 0x80000000 {
        if r.wrapping_shl(19) == 0x400000 {
            return rd(m, 0x15A780);
        }
        return r;
    }
    wr(m, lic, a);
    if ctx == 0 {
        return r;
    }
    fn_19611c(m, rd(m, a), rd(m, a.wrapping_add(4)), ctx)
}

#[allow(clippy::too_many_arguments)]
pub fn fn_151914(
    m: &mut Memory<'_>,
    h: &mut impl Calls,
    a0: u32,
    a1: u32,
    a2: u32,
    a3: u32,
    s0: u32,
    s1: u32,
    s2: u32,
    s3: u32,
    s4: u32,
) -> u32 {
    let sb = a3;
    let sp = fn_129778(m, &mut *h, 0x20);
    if sp == 0 {
        return 0x8900200A;
    }
    wr(m, sp.wrapping_add(0x18), 0);
    wr(m, sp.wrapping_add(0x14), 0);

    if sb != 0 || (s0 != 0 && s1 != 0) {
        if a2 != 0 && s4 != 0 {
            wr(m, s4, 0);
            wr(m, s4.wrapping_add(4), 0);
            let r = fn_1517e8(
                m,
                &mut *h,
                a0,
                a1,
                sp.wrapping_add(0x14),
                sp.wrapping_add(0x18),
            );
            if r >= 0x80000000 {
                return r;
            }

            return fn_151914_open(m, h, sb, a2, s0, s1, s2, s3, s4, sp);
        }
    }
    rd(m, 0x151BBC)
}

#[allow(clippy::too_many_arguments)]
fn fn_151914_open(
    m: &mut Memory<'_>,
    h: &mut impl Calls,
    sb: u32,
    a2: u32,
    s0: u32,
    s1: u32,
    s2: u32,
    s3: u32,
    s4: u32,
    sp: u32,
) -> u32 {
    let mut sl = 0u32;
    let fp = rd(m, sp.wrapping_add(0x14));
    let r6 = rd(m, sp.wrapping_add(0x18));

    wr(m, sp, 0x151B50);
    wr(m, sp.wrapping_add(4), a2);
    {
        let r0 = if sb != 0 { sb } else { 0x151900 };
        wr(m, sp.wrapping_add(8), r0);
        wr(m, sp.wrapping_add(12), s0);
        wr(m, sp.wrapping_add(16), s1);
    }
    let logger = rd(m, fp.wrapping_add(0x10));
    {
        let mut rh = RouteHost { inner: &mut *h };
        tts_core::log_live::fn_1738e8(m, logger, rd(m, 0x151B80), 4, 0, 0x151B50, sp, &mut rh);
    }
    wr(m, sp.wrapping_add(4), 0x151B84);
    wr(m, sp, 2);
    let mut r4 = route_or_host(m, &mut *h, 0x176F2C, &[rd(m, r6.wrapping_add(0x2c))]);
    if r4 >= 0x80000000 {
        wr(m, sp, 0x151BA0);
        wr(m, sp.wrapping_add(8), 0x151B98);
        wr(m, sp.wrapping_add(12), r4);
        let logger = rd(m, fp.wrapping_add(0x10));
        tts_core::cold_boot::fn_173dc0(
            m,
            &mut *h,
            logger,
            rd(m, 0x151B80),
            rd(m, 0x151BB8),
            0x151BAC,
            sp,
            sp.wrapping_add(8),
        );
    } else {
        let b = rb(m, r6.wrapping_add(0x30)) as u32;
        if b < 7 && !matches!(b, 0 | 2) {
            sl = b;
            wb(m, r6.wrapping_add(0x30), (rd(m, sp) & 0xff) as u8);
        } else {
            wr(m, sp, 0x151BA0);
            let logger = rd(m, fp.wrapping_add(0x10));
            tts_core::cold_boot::fn_173dc0(
                m,
                &mut *h,
                logger,
                rd(m, 0x151B80),
                rd(m, 0x151BC8),
                0x151BC0,
                sp,
                sp.wrapping_add(4),
            );
            r4 = rd(m, 0x151BCC);
        }

        let u = route_or_host(m, &mut *h, 0x176F5C, &[rd(m, r6.wrapping_add(0x2c))]);
        if u >= 0x80000000 {
            if r4 < 0x80000000 {
                r4 = u;
            }
            return r4;
        }
        if r4 >= 0x80000000 {
            return r4;
        }
    }

    let resource = rd(m, r6.wrapping_add(0x7c));
    if resource == 0 {
        let logger = rd(m, fp.wrapping_add(0x10));
        tts_core::cold_boot::fn_173dc0(
            m,
            &mut *h,
            logger,
            rd(m, 0x151B80),
            rd(m, 0x151BF4),
            0,
            sp,
            sp.wrapping_add(4),
        );
        r4 = rd(m, 0x151BCC);
    } else {
        wr(m, sp.wrapping_add(0x10), s4);
        wr(m, sp, s0);
        wr(m, sp.wrapping_add(4), s1);
        wr(m, sp.wrapping_add(8), s2);
        wr(m, sp.wrapping_add(12), s3);
        r4 = fn_17f03c(m, &mut *h, resource, 0, a2, sb, s0, s1, s2, s3, s4);
    }

    let s15 = fn_129778(m, &mut *h, 8);
    if s15 == 0 {
        return 0x8900200A;
    }
    let _ = fn_15af34(m, &mut *h, rd(m, sp.wrapping_add(0x14)), r6, sl & 0xff, s15);
    wr(m, sp.wrapping_add(8), rd(m, s4));
    wr(m, sp, 0x151BD0);
    wr(m, sp.wrapping_add(4), r4);
    let logger = rd(m, fp.wrapping_add(0x10));
    {
        let mut rh = RouteHost { inner: &mut *h };
        tts_core::log_live::fn_1738e8(m, logger, rd(m, 0x151B80), 4, 0, 0x151BD0, sp, &mut rh);
    }
    r4
}

fn ensure_ac(m: &mut Memory<'_>, h: &mut impl Calls, params: u32, name: u32, def: u32, pv: u32) {
    wr(m, pv, 0);
    let r = route_or_host(m, &mut *h, 0x175A1C, &[params, name, pv]);
    if r < 0x80000000 && rd(m, pv) != 0 {
        route_or_host(m, &mut *h, 0x1757A8, &[params]);
    } else {
        route_or_host(m, &mut *h, 0x1758AC, &[params, name, def]);
    }
}

fn ensure_a0(
    m: &mut Memory<'_>,
    h: &mut impl Calls,
    params: u32,
    name: u32,
    defaddr: u32,
    pv: u32,
) {
    wr(m, pv, 0);
    let r = route_or_host(m, &mut *h, 0x175A1C, &[params, name, pv]);
    if r < 0x80000000 && rd(m, pv) != 0 {
        route_or_host(m, &mut *h, 0x1757A8, &[params]);
    } else {
        route_or_host(m, &mut *h, 0x1759A0, &[params, name, defaddr]);
    }
}

fn fail_157a80(
    m: &mut Memory<'_>,
    h: &mut impl Calls,
    ctx: u32,
    sp: u32,
    out: u32,
    r4: u32,
) -> u32 {
    let _ = route_or_host(m, &mut *h, 0x1523CC, &[ctx, 1, sp.wrapping_add(0x14)]);
    wr(m, sp.wrapping_add(0x14), 0);
    wr(m, out, 0);
    wr(m, out.wrapping_add(4), 0);
    end_157a80(m, h, ctx, sp, r4, 0)
}

fn end_157a80(m: &mut Memory<'_>, h: &mut impl Calls, ctx: u32, sp: u32, r4: u32, o: u32) -> u32 {
    let logger = rd(m, ctx.wrapping_add(0x10));
    wr(m, sp, 0x158900);
    wr(m, sp.wrapping_add(4), r4);
    wr(m, sp.wrapping_add(8), o);
    {
        let mut rh = RouteHost { inner: &mut *h };
        tts_core::log_live::fn_1738e8(m, logger, rd(m, 0x157904), 4, 0, 0x158900, sp, &mut rh);
    }
    r4
}

#[allow(clippy::too_many_arguments)]
pub fn fn_157a80(
    m: &mut Memory<'_>,
    h: &mut impl Calls,
    a: u32,
    b: u32,
    c: u32,
    d: u32,
    opts: u32,
    sink: u32,
    mime: u32,
    out: u32,
) -> u32 {
    let sp = fn_129778(m, &mut *h, 0x60);
    if sp == 0 {
        return 0x8900200A;
    }

    let r = route_or_host(
        m,
        &mut *h,
        0x15AE20,
        &[a, b, sp.wrapping_add(0x10), sp.wrapping_add(0x24)],
    );
    if r >= 0x80000000 {
        return rd(m, 0x157A10);
    }
    if out == 0 {
        return rd(m, 0x157EC0);
    }

    let r = fn_19611c(m, c, d, sp.wrapping_add(0x10));
    if r >= 0x80000000 {
        return r;
    }
    let ctx = rd(m, sp.wrapping_add(0x10));
    let params = rd(m, ctx.wrapping_add(0x14));
    if params == 0 {
        return rd(m, 0x157EC4);
    }

    let mime1 = if mime != 0 { mime } else { 0x157A50 };
    let logger = rd(m, ctx.wrapping_add(0x10));
    wr(m, sp, 0x157EC8);
    wr(m, sp.wrapping_add(4), opts);
    wr(m, sp.wrapping_add(8), sink);
    wr(m, sp.wrapping_add(0xC), mime1);
    {
        let mut rh = RouteHost { inner: &mut *h };
        tts_core::log_live::fn_1738e8(m, logger, rd(m, 0x157904), 4, 0, 0x157EC8, sp, &mut rh);
    }

    let heap = rd(m, ctx.wrapping_add(4));
    let o = fn_1963b8(m, &mut *h, heap, 1, 0x14C);
    if o == 0 {
        tts_core::cold_boot::fn_173dc0(m, &mut *h, logger, rd(m, 0x157904), 0x3E80, 0, sp, sp);
        return rd(m, 0x1574A0);
    }
    wr(m, out, o);
    wr(m, out.wrapping_add(4), rd(m, 0x157EF0));

    wr(m, o, c);
    wr(m, o.wrapping_add(4), d);
    wr(m, o.wrapping_add(8), o);
    wr(m, o.wrapping_add(12), rd(m, 0x157EF0));

    wr(m, o.wrapping_add(0x10), rd(m, sp.wrapping_add(0x24)));
    for i in 0..5u32 {
        wr(
            m,
            o.wrapping_add(0x14 + 4 * i),
            rd(m, opts.wrapping_add(4 * i)),
        );
    }
    wr(m, o.wrapping_add(0x28), sink);
    wb(m, o.wrapping_add(0x30), 1);
    wb(m, o.wrapping_add(0x69), 3);
    wr(m, o.wrapping_add(0x140), 0);

    let mut r4 = fn_176e40(
        m,
        &mut *h,
        rd(m, ctx.wrapping_add(8)),
        heap,
        o.wrapping_add(0x2C),
    );
    if r4 >= 0x80000000 {
        wr(m, sp, 0x157EF4);
        wr(m, sp.wrapping_add(4), r4);
        tts_core::cold_boot::fn_173dc0(
            m,
            &mut *h,
            logger,
            rd(m, 0x157904),
            rd(m, 0x157EFC),
            0x157944,
            sp,
            sp.wrapping_add(4),
        );
        return fail_157a80(m, h, ctx, sp, out, r4);
    }

    let dbase = rd(m, 0x157EBC);
    wr(m, sp.wrapping_add(0x28), rd(m, dbase.wrapping_add(0x138)));
    let copy = rd(m, dbase.wrapping_add(0x13C));
    wr(m, sp.wrapping_add(0x40), rd(m, dbase.wrapping_add(4)));
    wr(m, sp, 1);
    wr(m, sp.wrapping_add(4), o.wrapping_add(0x3C));
    let comp = rd(m, ctx.wrapping_add(0x18));
    if fn_196a64(
        m,
        &mut *h,
        heap,
        rd(m, sp.wrapping_add(0x28)),
        copy,
        rd(m, sp.wrapping_add(0x40)),
        1,
        o.wrapping_add(0x3C),
    ) == 0
    {
        r4 = rd(m, 0x1574A0);
        tts_core::cold_boot::fn_173dc0(m, &mut *h, logger, rd(m, 0x157904), 0x3E80, 0, sp, sp);
        return fail_157a80(m, h, ctx, sp, out, r4);
    }

    let stream_name = rd(m, 0x157F00);
    let mut rq = route_or_host(
        m,
        &mut *h,
        0x17B058,
        &[comp, stream_name, sp.wrapping_add(0x18)],
    );
    if rq >= 0x80000000 {
        wr(m, sp, rd(m, o.wrapping_add(0x3C)));
        wr(m, sp.wrapping_add(4), 0);
        r4 = route_or_host(
            m,
            &mut *h,
            0x17F278,
            &[c, d, stream_name, 0x157F04, rd(m, o.wrapping_add(0x3C)), 0],
        );
        if r4 >= 0x80000000 {
            return fail_157a80(m, h, ctx, sp, out, r4);
        }
        r4 = route_or_host(
            m,
            &mut *h,
            0x17B058,
            &[comp, stream_name, sp.wrapping_add(0x18)],
        );
        if r4 >= 0x80000000 {
            wr(m, sp, 0x157EF4);
            wr(m, sp.wrapping_add(4), r4);
            tts_core::cold_boot::fn_173dc0(
                m,
                &mut *h,
                logger,
                rd(m, 0x157904),
                rd(m, 0x157F10),
                0x157944,
                sp,
                sp.wrapping_add(4),
            );
            return fail_157a80(m, h, ctx, sp, out, r4);
        }
        rq = r4;
    }
    wr(
        m,
        o.wrapping_add(0x40),
        rd(m, rd(m, sp.wrapping_add(0x18)).wrapping_add(4)),
    );

    let mut rl = route_or_host(
        m,
        &mut *h,
        0x17B058,
        &[comp, 0x157F14, sp.wrapping_add(0x18)],
    );
    if rl >= 0x80000000 {
        wr(m, sp, rd(m, o.wrapping_add(0x3C)));
        wr(m, sp.wrapping_add(4), 0);
        r4 = route_or_host(
            m,
            &mut *h,
            0x17F278,
            &[c, d, 0x157F14, 0x157F1C, rd(m, o.wrapping_add(0x3C)), 0],
        );
        if r4 >= 0x80000000 {
            return fail_157a80(m, h, ctx, sp, out, r4);
        }
        r4 = route_or_host(
            m,
            &mut *h,
            0x17B058,
            &[comp, 0x157F14, sp.wrapping_add(0x18)],
        );
        if r4 >= 0x80000000 {
            wr(m, sp, 0x157EF4);
            wr(m, sp.wrapping_add(4), r4);
            tts_core::cold_boot::fn_173dc0(
                m,
                &mut *h,
                logger,
                rd(m, 0x157904),
                rd(m, 0x157F2C),
                0x157F24,
                sp,
                sp.wrapping_add(4),
            );
            return fail_157a80(m, h, ctx, sp, out, r4);
        }
        rl = r4;
    }
    let _ = rl;
    wr(
        m,
        o.wrapping_add(0x54),
        rd(m, rd(m, sp.wrapping_add(0x18)).wrapping_add(4)),
    );
    let _ = rq;

    let lic = rd(m, sp.wrapping_add(0x24));
    if rd(m, lic.wrapping_add(0x10)) != 0 {
        let r = route_or_host(
            m,
            &mut *h,
            0x17B058,
            &[comp, 0x157F30, sp.wrapping_add(0x18)],
        );
        if r >= 0x80000000 {
            fn_195bec(m, sp.wrapping_add(0x40), 0, 20);
            wb(m, sp.wrapping_add(0x40), 1);
            wr(m, sp.wrapping_add(0x44), rd(m, lic + 0x10));
            wr(m, sp.wrapping_add(0x48), rd(m, lic + 0x14));
            wr(m, sp.wrapping_add(0x4C), rd(m, lic + 0x18));
            r4 = route_or_host(
                m,
                &mut *h,
                0x17B0F8,
                &[comp, 0x157F30, sp.wrapping_add(0x40)],
            );
            if r4 >= 0x80000000 {
                wr(m, sp, 0x157EF4);
                wr(m, sp.wrapping_add(4), r4);
                tts_core::cold_boot::fn_173dc0(
                    m,
                    &mut *h,
                    logger,
                    rd(m, 0x157904),
                    rd(m, 0x15794C),
                    0x157F24,
                    sp,
                    sp.wrapping_add(4),
                );
                return fail_157a80(m, h, ctx, sp, out, r4);
            }
            wb(m, o.wrapping_add(0x59), 1);
            return voice_inet_157a80(m, h, ctx, sp, out, o, c, d, mime, opts, params, logger, r4);
        }
        let _ = route_or_host(m, &mut *h, 0x17B1C4, &[comp, 0x157F30]);
    }

    let r = route_or_host(
        m,
        &mut *h,
        0x17B058,
        &[comp, 0x157F3C, sp.wrapping_add(0x18)],
    );
    if r >= 0x80000000 {
        fn_195bec(m, sp.wrapping_add(0x40), 0, 20);
        wb(m, sp.wrapping_add(0x40), 3);
        wr(m, sp.wrapping_add(0x44), rd(m, 0x157EB8));
        wr(m, sp.wrapping_add(0x48), rd(m, out));
        wr(m, sp.wrapping_add(0x4C), rd(m, out.wrapping_add(4)));
        r4 = route_or_host(
            m,
            &mut *h,
            0x17B0F8,
            &[comp, 0x157F3C, sp.wrapping_add(0x40)],
        );
        if r4 >= 0x80000000 {
            wr(m, sp, 0x157EF4);
            wr(m, sp.wrapping_add(4), r4);
            tts_core::cold_boot::fn_173dc0(
                m,
                &mut *h,
                logger,
                rd(m, 0x157904),
                rd(m, 0x15794C),
                0x157F24,
                sp,
                sp.wrapping_add(4),
            );
            return fail_157a80(m, h, ctx, sp, out, r4);
        }
        wb(m, o.wrapping_add(0x59), 1);
    } else {
        let _ = route_or_host(m, &mut *h, 0x17B1C4, &[comp, 0x157F3C]);
    }
    voice_inet_157a80(m, h, ctx, sp, out, o, c, d, mime, opts, params, logger, r4)
}

#[allow(clippy::too_many_arguments)]
fn voice_inet_157a80(
    m: &mut Memory<'_>,
    h: &mut impl Calls,
    ctx: u32,
    sp: u32,
    out: u32,
    o: u32,
    c: u32,
    d: u32,
    mime: u32,
    _opts: u32,
    params: u32,
    logger: u32,
    mut r4: u32,
) -> u32 {
    let comp = rd(m, ctx.wrapping_add(0x18));

    let ri = route_or_host(
        m,
        &mut *h,
        0x17B058,
        &[comp, 0x157F44, sp.wrapping_add(0x18)],
    );
    if ri < 0x80000000 {
        let q = rd(m, sp.wrapping_add(0x18));
        wr(m, o.wrapping_add(0x5C), rd(m, q.wrapping_add(4)));
        wr(m, o.wrapping_add(0x60), rd(m, q.wrapping_add(8)));
        wr(m, o.wrapping_add(0x64), rd(m, q.wrapping_add(12)));
    }

    fn_195bec(m, sp.wrapping_add(0x34), 0, 12);
    wr(m, sp.wrapping_add(0x34), o);
    wr(m, sp.wrapping_add(0x38), rd(m, 0x157F4C));
    wr(m, sp.wrapping_add(0x3C), rd(m, 0x157F50));
    if route_or_host(m, &mut *h, 0x173FD8, &[rd(m, ctx.wrapping_add(0x10))]) >= 4 {
        wr(m, sp, rd(m, sp.wrapping_add(0x3C)));
        r4 = route_or_host(
            m,
            &mut *h,
            0x174E74,
            &[
                params,
                0,
                o,
                rd(m, sp.wrapping_add(0x38)),
                rd(m, sp.wrapping_add(0x3C)),
            ],
        );
        if r4 < 0x80000000 {
            wb(m, o.wrapping_add(0x68), 1);
        } else {
            tts_core::cold_boot::fn_173dc0(
                m,
                &mut *h,
                logger,
                rd(m, 0x157904),
                rd(m, 0x1582B0),
                0,
                sp,
                sp,
            );
        }
    }

    let pv = sp.wrapping_add(0xC);
    ensure_ac(m, h, params, 0x1582B4, 0x32, pv);
    ensure_ac(m, h, params, 0x1582BC, 0x64, pv);
    ensure_ac(m, h, params, 0x1582C4, 0x50, pv);
    ensure_ac(m, h, params, 0x1582CC, 2, pv);
    ensure_ac(m, h, params, 0x1582D8, 1, pv);
    ensure_ac(m, h, params, 0x1582E4, 0, pv);
    ensure_ac(m, h, params, rd(m, 0x158708), rd(m, 0x158704), pv);
    ensure_a0(m, h, params, rd(m, 0x15870C), 0x1582C0, pv);
    ensure_ac(m, h, params, rd(m, 0x158710), 0x1000, pv);
    ensure_a0(m, h, params, 0x15871C, 0x158714, pv);
    ensure_a0(m, h, params, rd(m, 0x15872C), 0x1582C0, pv);
    ensure_a0(m, h, params, 0x158730, 0x1582C0, pv);
    ensure_a0(m, h, params, rd(m, 0x15873C), 0x1582C0, pv);

    r4 = route_or_host(
        m,
        &mut *h,
        0x175944,
        &[params, rd(m, 0x158708), o.wrapping_add(0x38)],
    );
    if r4 >= 0x80000000 {
        return fail_logged_3ee8_157a80(m, h, ctx, sp, out, logger, r4);
    }
    r4 = route_or_host(
        m,
        &mut *h,
        0x175944,
        &[params, 0x1582E4, o.wrapping_add(0x34)],
    );
    if r4 >= 0x80000000 {
        return fail_logged_3ee8_157a80(m, h, ctx, sp, out, logger, r4);
    }

    if rb(m, o.wrapping_add(0x68)) == 0 {
        let tab = rd(m, 0x158754);
        let mut i = 0u32;
        let mut fp = 0u32;
        loop {
            let nm = rd(m, tab.wrapping_add(i.wrapping_mul(4)));
            if nm == 0 {
                break;
            }
            fp = 0xA00000;
            wr(m, sp, rd(m, sp.wrapping_add(0x3C)));
            r4 = route_or_host(
                m,
                &mut *h,
                0x174E74,
                &[
                    params,
                    nm,
                    o,
                    rd(m, sp.wrapping_add(0x38)),
                    rd(m, sp.wrapping_add(0x3C)),
                ],
            );
            if r4 < 0x80000000 {
                wb(m, o.wrapping_add(0x68), 1);
            } else {
                wr(m, sp, 0x158758);
                wr(m, sp.wrapping_add(4), nm);
                wr(m, sp.wrapping_add(8), 0x158744);
                wr(m, sp.wrapping_add(0xC), r4);
                tts_core::cold_boot::fn_173dc0(
                    m,
                    &mut *h,
                    logger,
                    rd(m, 0x157904),
                    rd(m, 0x1582B0),
                    0x158764,
                    sp,
                    sp.wrapping_add(4),
                );

                if fp != r4 {
                    return fail_157a80(m, h, ctx, sp, out, r4);
                }
            }
            i = i.wrapping_add(1);
        }
    }

    r4 = route_or_host(m, &mut *h, 0x17EB90, &[c, d, o.wrapping_add(0x74)]);
    if r4 >= 0x80000000 {
        wr(m, sp, 0x158744);
        wr(m, sp.wrapping_add(4), r4);
        tts_core::cold_boot::fn_173dc0(
            m,
            &mut *h,
            logger,
            rd(m, 0x157904),
            rd(m, 0x1582B0),
            0x15874C,
            sp,
            sp.wrapping_add(4),
        );
        return fail_157a80(m, h, ctx, sp, out, r4);
    }

    wb(
        m,
        o.wrapping_add(0x70),
        rb(m, rd(m, sp.wrapping_add(0x24)).wrapping_add(0x94)),
    );
    fn_195bec(m, sp.wrapping_add(0x40), 0, 20);
    wb(m, sp.wrapping_add(0x40), 1);
    wr(m, sp.wrapping_add(0x44), rd(m, 0x1588F0));
    wr(m, sp.wrapping_add(0x48), rd(m, out));
    wr(m, sp.wrapping_add(0x4C), rd(m, out.wrapping_add(4)));
    r4 = route_or_host(
        m,
        &mut *h,
        0x17B0F8,
        &[comp, rd(m, 0x1588F4), sp.wrapping_add(0x40)],
    );
    if r4 >= 0x80000000 {
        wr(m, sp, 0x158744);
        wr(m, sp.wrapping_add(4), r4);
        tts_core::cold_boot::fn_173dc0(
            m,
            &mut *h,
            logger,
            rd(m, 0x157904),
            rd(m, 0x1588F8),
            0x15874C,
            sp,
            sp.wrapping_add(4),
        );
        return fail_157a80(m, h, ctx, sp, out, r4);
    }
    wb(m, o.wrapping_add(0x78), 1);

    if mime != 0 && rb(m, mime) != 0 {
        let param_cb = rd(m, o.wrapping_add(0x3C));
        wr(m, sp, rd(m, 0x1588FC));
        wr(m, sp.wrapping_add(4), param_cb);
        wr(m, sp.wrapping_add(8), o.wrapping_add(0x7C));
        wr(m, sp.wrapping_add(0xC), sp.wrapping_add(0x1C));
        r4 = route_or_host(
            m,
            &mut *h,
            0x17F614,
            &[
                rd(m, o),
                rd(m, o.wrapping_add(4)),
                mime,
                0,
                rd(m, 0x1588FC),
                param_cb,
                o.wrapping_add(0x7C),
                sp.wrapping_add(0x1C),
            ],
        );
        if r4 >= 0x80000000 {
            return fail_157a80(m, h, ctx, sp, out, r4);
        }
        r4 = route_or_host(m, &mut *h, 0x15B128, &[ctx, o, rd(m, o.wrapping_add(0x7C))]);
        wb(m, o.wrapping_add(0x69), (r4 & 0xFF) as u8);
        r4 = route_or_host(m, &mut *h, 0x1529B4, &[ctx, o]);
        if r4 >= 0x80000000 {
            return fail_157a80(m, h, ctx, sp, out, r4);
        }
    }

    let r = route_or_host(
        m,
        &mut *h,
        0x175A1C,
        &[params, rd(m, 0x15872C), sp.wrapping_add(0x20)],
    );
    if r < 0x80000000 {
        r4 = route_or_host(
            m,
            &mut *h,
            0x15AF9C,
            &[ctx, o, rd(m, sp.wrapping_add(0x20))],
        );
        if r4 >= 0x80000000 {
            return fail_157a80(m, h, ctx, sp, out, r4);
        }
    }
    end_157a80(m, h, ctx, sp, r4, o)
}

fn fail_logged_3ee8_157a80(
    m: &mut Memory<'_>,
    h: &mut impl Calls,
    ctx: u32,
    sp: u32,
    out: u32,
    logger: u32,
    r4: u32,
) -> u32 {
    wr(m, sp, 0x158744);
    wr(m, sp.wrapping_add(4), r4);
    tts_core::cold_boot::fn_173dc0(
        m,
        &mut *h,
        logger,
        rd(m, 0x157904),
        rd(m, 0x158740),
        0x15874C,
        sp,
        sp.wrapping_add(4),
    );
    fail_157a80(m, h, ctx, sp, out, r4)
}

fn exit38(m: &mut Memory<'_>, h: &mut impl Calls, logger: u32, r7: u32, sp: u32) -> u32 {
    wr(m, sp, 0x193D70);
    {
        let mut rh = RouteHost { inner: &mut *h };
        tts_core::log_live::fn_1738e8(m, logger, rd(m, 0x193470), 4, 0, 0x193D70, sp, &mut rh);
    }
    tts_core::cold_boot::fn_160fec(r7)
}

pub fn fn_14dc30(m: &mut Memory<'_>, h: &mut impl Calls, a0: u32) -> u32 {
    if a0 == 0 {
        return 0;
    }
    let mut r4 = 0u32;
    loop {
        let slot = a0.wrapping_add(r4);
        let mut tgt = rd(m, slot.wrapping_add(0xC));
        if tgt != 0 {
            tgt = rd(m, tgt.wrapping_add(0x14));
            if tgt != 0 {
                let arg = rd(m, slot.wrapping_add(8));
                if arg != 0 {
                    let r = route_or_host(m, &mut *h, tgt, &[arg]);
                    if r >= 0x80000000 {
                        return r;
                    }
                }
            }
        }
        r4 = r4.wrapping_add(1) & 0xFF;
        if r4 >= 5 {
            break;
        }
    }
    let _ = route_or_host(m, &mut *h, 0x196428, &[rd(m, a0), a0]);
    0
}

pub fn fn_174e00(m: &mut Memory<'_>, h: &mut impl Calls, a0: u32) -> u32 {
    if a0 == 0 {
        return rd(m, 0x17551C);
    }
    if rd(m, a0.wrapping_add(0x10)) != 0 {
        fn_196c7c(m, &mut *h, rd(m, a0.wrapping_add(0x10)));
        wr(m, a0.wrapping_add(0x10), 0);
    }
    if rd(m, a0.wrapping_add(0xC)) != 0 {
        fn_196c7c(m, &mut *h, rd(m, a0.wrapping_add(0xC)));
        wr(m, a0.wrapping_add(0xC), 0);
    }
    if rd(m, a0.wrapping_add(8)) != 0 {
        fn_19665c(m, &mut *h, rd(m, a0.wrapping_add(8)));
        wr(m, a0.wrapping_add(8), 0);
    }
    if rd(m, a0.wrapping_add(4)) != 0 {
        fn_176edc(m, &mut *h, rd(m, a0.wrapping_add(4)));
    }
    let _ = route_or_host(m, &mut *h, 0x196428, &[rd(m, a0), a0]);
    0
}

pub fn fn_176e10(m: &mut Memory<'_>, h: &mut impl Calls, cell: u32) -> u32 {
    if cell == 0 || rd(m, cell) == 0 {
        return rd(m, 0x17788C);
    }
    let _ = route_or_host(m, &mut *h, 0x196428, &[rd(m, rd(m, cell)), rd(m, cell)]);
    wr(m, cell, 0);
    0
}

pub fn fn_172e84(m: &mut Memory<'_>, h: &mut impl Calls, node: u32) -> u32 {
    if node == 0 {
        return rd(m, 0x1774E8);
    }
    let _ = route_or_host(m, &mut *h, 0x196428, &[rd(m, node), node]);
    0
}

pub fn fn_196c7c(m: &mut Memory<'_>, h: &mut impl Calls, a0: u32) -> u32 {
    if a0 == 0 {
        return 0;
    }
    if rd(m, a0) != 0 && rd(m, a0.wrapping_add(0xC)) != 0 {
        let limit0 = rd(m, a0.wrapping_add(0x14));
        if (limit0 as i32) > 0 {
            let mut i = 0u32;
            loop {
                let step = rh(m, a0.wrapping_add(4)) as u32;
                let arg0 = rd(m, a0).wrapping_add(i.wrapping_mul(step));
                let cb = rd(m, a0.wrapping_add(0xC));
                let _ = route_or_host(m, &mut *h, cb, &[arg0, rd(m, a0.wrapping_add(0x1C))]);
                i = i.wrapping_add(1);
                if rd(m, a0.wrapping_add(0x14)) <= i {
                    break;
                }
            }
        }
    }
    let _ = route_or_host(
        m,
        &mut *h,
        0x196428,
        &[rd(m, a0.wrapping_add(0x1C)), rd(m, a0)],
    );
    let _ = route_or_host(m, &mut *h, 0x196428, &[rd(m, a0.wrapping_add(0x1C)), a0]);
    1
}

pub fn fn_177cc8(m: &mut Memory<'_>, h: &mut impl Calls, a0: u32) -> u32 {
    if a0 == 0 {
        return rd(m, 0x177274);
    }
    let mut r6 = 0u32;
    let inner = rd(m, a0.wrapping_add(4));
    let entry = rd(m, inner.wrapping_add(8));
    let pool = rd(m, 0x177298);
    let _ = route_or_host(
        m,
        &mut *h,
        0x1738E8,
        &[rd(m, entry.wrapping_add(0x10)), pool, 4, 0, 0x177D80],
    );
    if rd(m, rd(m, inner.wrapping_add(8)).wrapping_add(0xC)) == a0 {
        let q = route_or_host(m, &mut *h, 0x176F2C, &[rd(m, inner.wrapping_add(0xC))]);
        if q >= 0x80000000 {
            return r6;
        }
        let w = route_or_host(m, &mut *h, 0x196770, &[rd(m, inner.wrapping_add(0x14))]);
        r6 = w;
        let v = route_or_host(m, &mut *h, 0x176F5C, &[rd(m, inner.wrapping_add(0xC))]);
        if v >= 0x80000000 && r6 < 0x80000000 {
            r6 = v;
        }
    }
    let _ = route_or_host(m, &mut *h, 0x196428, &[rd(m, entry.wrapping_add(4)), a0]);
    let _ = route_or_host(
        m,
        &mut *h,
        0x1738E8,
        &[rd(m, entry.wrapping_add(0x10)), pool, 4, 0, 0x177D98],
    );
    r6
}

pub fn fn_177558(m: &mut Memory<'_>, h: &mut impl Calls, a0: u32) -> u32 {
    if a0 == 0 {
        return 0;
    }
    let mut r5 = 0u32;
    let r6 = rd(m, a0.wrapping_add(8));
    if rd(m, a0.wrapping_add(0x14)) != 0 {
        let q = fn_19665c(m, &mut *h, rd(m, a0.wrapping_add(0x14)));
        if q >= 0x80000000 {
            r5 = q;
        }
        wr(m, a0.wrapping_add(0x14), 0);
    }
    if rd(m, a0.wrapping_add(0x10)) != 0 {
        let q = fn_19665c(m, &mut *h, rd(m, a0.wrapping_add(0x10)));
        wr(m, a0.wrapping_add(0x10), 0);
        if q >= 0x80000000 && r5 < 0x80000000 {
            r5 = q;
        }
    }
    if rd(m, a0.wrapping_add(0xC)) != 0 {
        let q = fn_176edc(m, &mut *h, rd(m, a0.wrapping_add(0xC)));
        wr(m, a0.wrapping_add(0xC), 0);
        if q >= 0x80000000 && r5 < 0x80000000 {
            r5 = q;
        }
    }
    if rd(m, a0.wrapping_add(0x1C)) != 0 {
        let q = fn_176edc(m, &mut *h, rd(m, a0.wrapping_add(0x1C)));
        wr(m, a0.wrapping_add(0x1C), 0);
        if q >= 0x80000000 && r5 < 0x80000000 {
            r5 = q;
        }
    }
    if rd(m, a0.wrapping_add(0x18)) != 0 {
        fn_17ad10(
            m,
            &mut *h,
            rd(m, r6.wrapping_add(4)),
            rd(m, a0.wrapping_add(0x18)),
        );
        wr(m, a0.wrapping_add(0x18), 0);
    }
    let _ = route_or_host(m, &mut *h, 0x196428, &[rd(m, r6.wrapping_add(4)), a0]);
    r5
}

pub fn fn_196238(m: &mut Memory<'_>, h: &mut impl Calls, a0: u32, a1: u32) -> u32 {
    let sp = fn_129778(m, &mut *h, 0xC);
    if sp == 0 {
        return 0x80000000;
    }
    for i in (0..0xCu32).step_by(4) {
        wr(m, sp.wrapping_add(i), 0);
    }
    tts_core::boot_part_d::fn_176fd4(m, sp, pair_176fd4(m));
    let r = route_or_host(
        m,
        &mut *h,
        0x176FC0,
        &[a0, a1, rd(m, sp), rd(m, sp.wrapping_add(4))],
    );
    if r >= 0x80000000 {
        let r5 = rd(m, 0x196304);
        if r5 != 0 {
            fn_195bec(m, r5, 0, 0x20);
            let _ = route_or_host(m, &mut *h, 0x196428, &[rd(m, r5.wrapping_add(4)), r5]);
        }
        return 0;
    }
    let half = rd(m, 0x196308);
    let lit = rd(m, 0x19630C);
    let q = route_or_host(m, &mut *h, 0x176F94, &[a0, a1, half, 0x20]);
    if q >= 0x80000000 {
        if q == 0x400000 {
            return lit;
        }
        return q;
    }
    if a0 == 0 || (rh(m, a0) as u32) != half {
        return lit;
    }

    fn_195bec(m, a0, 0, 0x20);
    let _ = route_or_host(m, &mut *h, 0x196428, &[rd(m, a0.wrapping_add(4)), a0]);
    q
}

pub fn fn_17af38(m: &mut Memory<'_>, h: &mut impl Calls, a0: u32) -> u32 {
    if a0 == 0 {
        return rd(m, 0x17B4EC);
    }
    let sp = fn_129778(m, &mut *h, 0x10);
    if sp == 0 {
        return 0x80000000;
    }
    for i in (0..0x10u32).step_by(4) {
        wr(m, sp.wrapping_add(i), 0);
    }
    let mut r5 = route_or_host(m, &mut *h, 0x176F2C, &[rd(m, a0.wrapping_add(0xC))]);
    if r5 >= 0x80000000 {
        return r5;
    }
    if rd(m, a0.wrapping_add(0x10)) != 0 {
        let q = route_or_host(
            m,
            &mut *h,
            0x196794,
            &[rd(m, a0.wrapping_add(0x10)), 0, 1, sp],
        );
        if q < 0x80000000 {
            loop {
                let v = route_or_host(
                    m,
                    &mut *h,
                    0x196860,
                    &[rd(m, sp), sp.wrapping_add(8), sp.wrapping_add(4)],
                );
                if v != 0 {
                    let _ = route_or_host(m, &mut *h, 0x196830, &[rd(m, sp)]);
                    break;
                }
                if (rh(m, rd(m, sp.wrapping_add(4)).wrapping_add(0x1C)) as u32) == 0 {
                    continue;
                }
                let _ = route_or_host(m, &mut *h, 0x196830, &[rd(m, sp)]);
                let _ = route_or_host(m, &mut *h, 0x176F5C, &[rd(m, a0.wrapping_add(0xC))]);
                return rd(m, 0x17B4F8);
            }
        }
        if rd(m, a0.wrapping_add(0x10)) != 0 {
            fn_19665c(m, &mut *h, rd(m, a0.wrapping_add(0x10)));
            wr(m, a0.wrapping_add(0x10), 0);
        }
    }
    let v = route_or_host(m, &mut *h, 0x176F5C, &[rd(m, a0.wrapping_add(0xC))]);
    if v >= 0x80000000 && r5 < 0x80000000 {
        r5 = v;
    }
    if rd(m, a0.wrapping_add(0xC)) != 0 {
        let w = fn_176edc(m, &mut *h, rd(m, a0.wrapping_add(0xC)));
        if w >= 0x80000000 && r5 < 0x80000000 {
            r5 = w;
        }
    }
    let _ = route_or_host(
        m,
        &mut *h,
        0x196428,
        &[rd(m, rd(m, a0.wrapping_add(8)).wrapping_add(4)), a0],
    );
    r5
}

pub fn fn_199034(
    m: &mut Memory<'_>,
    h: &mut impl Calls,
    a0: u32,
    a1: u32,
    a2: u32,
    _a3: u32,
    s0: u32,
) -> u32 {
    if a0 == 0 || a1 == 0 || s0 == 0 {
        return rd(m, 0x1992E8);
    }
    if rd(m, rd(m, a0.wrapping_add(8))) == 0 {
        return rd(m, 0x1992EC);
    }
    let node = fn_1963b8(m, &mut *h, a1, 1, 0xC);
    wr(m, s0, node);
    if node == 0 {
        return rd(m, 0x1992F0);
    }
    wr(m, node, a0);
    wr(m, node.wrapping_add(4), a1);
    let stack_word = rd(m, s0).wrapping_add(8);
    let tgt = rd(m, rd(m, a0.wrapping_add(8)));
    let r = route_or_host(
        m,
        &mut *h,
        tgt,
        &[
            rd(m, a0.wrapping_add(0xC)),
            rd(m, a0.wrapping_add(4)),
            a2,
            stack_word,
        ],
    );
    if r < 0x80000000 {
        return r;
    }
    let _ = route_or_host(m, &mut *h, 0x196428, &[a1, rd(m, s0)]);
    wr(m, s0, 0);
    r
}

pub fn fn_1990ec(m: &mut Memory<'_>, h: &mut impl Calls, a0: u32) -> u32 {
    if a0 == 0 {
        return rd(m, 0x1992E8);
    }
    let tgt = rd(m, rd(m, rd(m, a0).wrapping_add(8)).wrapping_add(4));
    let r5 = route_or_host(m, &mut *h, tgt, &[rd(m, a0.wrapping_add(8))]);
    if r5 >= 0x80000000 {
        return r5;
    }
    let _ = route_or_host(m, &mut *h, 0x196428, &[rd(m, a0.wrapping_add(4)), a0]);
    r5
}

pub fn fn_1994d0(len: u32, quantum: u32) -> u32 {
    if quantum == 0 {
        0xFFFF_FFFF
    } else {
        len / quantum
    }
}

pub fn fn_196394(m: &mut Memory<'_>, h: &mut impl Calls, heap: u32, size: u32) -> u32 {
    if heap == 0 || size == 0 {
        return 0;
    }
    let target = rd(m, rd(m, heap));
    let args = [rd(m, heap.wrapping_add(4)), size];
    if let Some(r) = route_call(m, &mut *h, target, &args) {
        return r;
    }
    h.call(m, target, &args)
}

pub fn fn_1963e0(m: &mut Memory<'_>, h: &mut impl Calls, heap: u32, old: u32, size: u32) -> u32 {
    if heap == 0 || size == 0 {
        return 0;
    }
    let vt = rd(m, heap);
    let obj = rd(m, heap.wrapping_add(4));
    if old == 0 {
        let target = rd(m, vt);
        let args = [obj, size];
        if let Some(r) = route_call(m, &mut *h, target, &args) {
            return r;
        }
        return h.call(m, target, &args);
    }
    let target = rd(m, vt.wrapping_add(8));
    let args = [obj, old, size];

    if let Some(r) = route_call(m, &mut *h, target, &args) {
        return r;
    }
    h.call(m, target, &args)
}

pub fn fn_4a79b8(m: &mut Memory<'_>, h: &mut impl Calls, obj: u32, old: u32, newsize: u32) -> u32 {
    let inner = rd(m, obj.wrapping_add(4));
    let lock = inner.wrapping_add(0x58);
    route_or_host(m, h, 0x1840FC, &[lock]);
    let ext = fn_6bab68(m, inner, old, newsize);
    route_or_host(m, h, 0x18423C, &[lock]);
    if ext != 0 {
        return old;
    }
    let vt = rd(m, obj);
    let newbuf = route_or_host(m, h, rd(m, vt.wrapping_add(8)), &[obj, newsize, 4]);
    if newbuf != 0 {
        fn_3023b0(m, newbuf, newsize);
    }
    let oldsize = rd(m, old.wrapping_sub(12));
    for i in 0..oldsize.min(newsize) {
        wb(m, newbuf.wrapping_add(i), rb(m, old.wrapping_add(i)));
    }
    route_or_host(m, h, rd(m, vt.wrapping_add(12)), &[obj, old]);
    newbuf
}

pub fn fn_6bab68(m: &mut Memory<'_>, outer: u32, old: u32, size: u32) -> u32 {
    assert!(outer >= 88);
    let mut saved = [0u8; 88];
    saved.copy_from_slice(&m.bytes[..88]);
    m.bytes.copy_within(outer as usize..outer as usize + 88, 0);
    let result = tts_core::heap::EngineHeap::from_arena(m.bytes)
        .block_resize(old, size)
        .unwrap_or(0);
    m.bytes.copy_within(0..88, outer as usize);
    m.bytes[..88].copy_from_slice(&saved);
    result
}

pub fn fn_196cfc(m: &mut Memory<'_>, h: &mut impl Calls, a0: u32, out: u32) -> u32 {
    if a0 == 0 || out == 0 {
        return 0;
    }
    let len = rd(m, a0.wrapping_add(0x14));
    let cap = rd(m, a0.wrapping_add(0x10));
    if len.wrapping_add(1) > cap {
        let quantum = rh(m, a0.wrapping_add(0x18)) as u32;
        let newcap = fn_1994d0(len, quantum)
            .wrapping_add(1)
            .wrapping_mul(quantum);
        let stride = rh(m, a0.wrapping_add(0x4)) as u32;
        let heap = rd(m, a0.wrapping_add(0x1C));
        let old = rd(m, a0);
        let nb = if old == 0 {
            fn_196394(m, h, heap, stride.wrapping_mul(newcap))
        } else {
            fn_1963e0(m, h, heap, old, stride.wrapping_mul(newcap))
        };

        if nb == 0 {
            return 0;
        }
        wr(m, a0, nb);
        wr(m, a0.wrapping_add(0x10), newcap);
    }
    let n = rd(m, a0.wrapping_add(0x14));
    let stride = rh(m, a0.wrapping_add(0x4)) as u32;
    wr(m, out, rd(m, a0).wrapping_add(n.wrapping_mul(stride)));
    wr(m, a0.wrapping_add(0x14), n.wrapping_add(1));
    1
}

pub fn fn_195d30(m: &Memory<'_>, s: u32, ch: u32) -> u32 {
    let want = (ch & 0xFF) as u8;
    let mut p = s;
    loop {
        let b = rb(m, p);
        if b == 0 {
            return 0;
        }
        if b == want {
            return p;
        }
        p = p.wrapping_add(1);
    }
}

pub fn fn_196db0(m: &mut Memory<'_>, h: &mut impl Calls, a0: u32, a1: u32) -> u32 {
    let sp = fn_129778(m, &mut *h, 8);
    if sp == 0 {
        return 0x80000000;
    }
    wr(m, sp, 0);
    wr(m, sp.wrapping_add(4), 0);
    if fn_196cfc(m, &mut *h, a0, sp) == 0 {
        return 0;
    }
    let _ = route_or_host(
        m,
        &mut *h,
        0x195AE8,
        &[rd(m, sp), a1, rh(m, a0.wrapping_add(4)) as u32],
    );
    if rd(m, a0.wrapping_add(8)) == 0 {
        return 1;
    }

    let r = route_or_host(
        m,
        &mut *h,
        rd(m, a0.wrapping_add(8)),
        &[a1, rd(m, a0.wrapping_add(0x1C)), rd(m, sp)],
    );

    if r == 0 {
        wr(
            m,
            a0.wrapping_add(0x14),
            rd(m, a0.wrapping_add(0x14)).wrapping_sub(1),
        );
        return 0;
    }
    1
}

pub fn fn_1991d0(m: &mut Memory<'_>, h: &mut impl Calls, a0: u32) -> u32 {
    if a0 == 0 {
        return rd(m, 0x1992F4);
    }
    let v0 = rd(m, a0);
    if v0 == 0 {
        return rd(m, 0x1992F4);
    }
    let tgt = rd(m, rd(m, v0.wrapping_add(8)).wrapping_add(0x10));
    if tgt == 0 {
        return rd(m, 0x1992F4);
    }
    route_or_host(m, &mut *h, tgt, &[rd(m, a0.wrapping_add(8))])
}

pub fn fn_199134(
    m: &mut Memory<'_>,
    h: &mut impl Calls,
    a0: u32,
    a1: u32,
    a2: u32,
    a3: u32,
) -> u32 {
    if a3 == 0 || a0 == 0 {
        return rd(m, 0x1992E8);
    }
    if a1 == 0 || a2 == 0 {
        return rd(m, 0x1992E8);
    }
    let v0 = rd(m, a3);
    let tgt = rd(m, rd(m, v0.wrapping_add(8)).wrapping_add(8));
    if tgt == 0 {
        return rd(m, 0x1992E8);
    }
    route_or_host(m, &mut *h, tgt, &[rd(m, a3.wrapping_add(8))])
}

pub fn fn_1970ac(m: &mut Memory<'_>, a0: u32, a1: u32, a2: u32) -> u32 {
    if a0 == 0 || a2 == 0 {
        return 0;
    }
    if (rd(m, a0.wrapping_add(0x14)) as i32) <= (a1 as i32) {
        return 0;
    }
    wr(
        m,
        a2,
        rd(m, a0).wrapping_add(a1.wrapping_mul(rh(m, a0.wrapping_add(4)) as u32)),
    );
    1
}

pub fn fn_195d90(m: &Memory<'_>, a0: u32) -> u32 {
    let mut n = 0u32;
    loop {
        if rb(m, a0.wrapping_add(n)) == 0 {
            break;
        }
        n = n.wrapping_add(1);
    }
    n
}

pub fn fn_195cd0(m: &mut Memory<'_>, a0: u32, a1: u32) -> u32 {
    let mut i = 0u32;
    loop {
        let b = rb(m, a1.wrapping_add(i));
        wb(m, a0.wrapping_add(i), b);
        if b == 0 {
            break;
        }
        i = i.wrapping_add(1);
    }
    a0
}

pub fn fn_196448(m: &mut Memory<'_>, h: &mut impl Calls, a0: u32, _a1: u32, _a2: u32) -> u32 {
    if a0 == 0 {
        return rd(m, 0x1964D4);
    }
    let tgt = rd(m, rd(m, rd(m, a0)).wrapping_add(0x10));
    if tgt == 0 {
        return rd(m, 0x1964DC);
    }
    route_or_host(m, &mut *h, tgt, &[rd(m, a0.wrapping_add(4))])
}

pub fn fn_39f5b8(
    m: &mut Memory<'_>,
    h: &mut impl Calls,
    a0: u32,
    a1: u32,
    _a2: u32,
    _a3: u32,
) -> u32 {
    let head = rd(m, a0.wrapping_add(4));
    let anchor = head.wrapping_add(0x58);
    let _ = route_or_host(m, &mut *h, 0x1840FC, &[anchor]);
    let _ = route_or_host(m, &mut *h, 0x19B78C, &[head, a1]);
    route_or_host(m, &mut *h, 0x18423C, &[anchor])
}

pub fn fn_1906cc(m: &mut Memory<'_>, h: &mut impl Calls, a0: u32, a1: u32) -> u32 {
    let sp = fn_129778(m, &mut *h, 0x30);
    if sp == 0 {
        return 0x80000000;
    }
    for i in (0..0x30u32).step_by(4) {
        wr(m, sp.wrapping_add(i), 0);
    }
    if a1 != rd(m, 0x190E8C) {
        return 0x80000008;
    }
    if rd(m, a0) != rd(m, 0x190E8C).wrapping_add(0xB000000) {
        return 0x80000008;
    }
    if a0 == 0 {
        return 0x80000008;
    }
    if rd(m, a0.wrapping_add(0xDC)) != 0 {
        return 0x80000011;
    }
    if fn_19611c(
        m,
        rd(m, a0.wrapping_add(0xA8)),
        rd(m, a0.wrapping_add(0xAC)),
        sp,
    ) >= 0x80000000
    {
        return 0x80000008;
    }
    let obj = rd(m, a0.wrapping_add(0xB0));
    let r6 = rd(m, obj.wrapping_add(4));
    wr(m, sp.wrapping_add(4), rd(m, obj.wrapping_add(8)));
    let r8 = rd(m, a0.wrapping_add(0x20));
    let r7 = rd(m, obj.wrapping_add(0x1C));
    wr(m, a0, rd(m, 0x190E90));
    let mut r5 = 0u32;
    if rd(m, a0.wrapping_add(0xC0)) != 0 {
        r5 = route_or_host(
            m,
            &mut *h,
            rd(m, rd(m, a0.wrapping_add(0xBC)).wrapping_add(0xC)),
            &[rd(m, a0.wrapping_add(0xC0)), rd(m, a0.wrapping_add(0xC4))],
        );
        if r5 >= 0x80000000 {
            wr(m, a0, 0);
            return fn_160fec(r5);
        }
        tts_core::boot_part_d::fn_176fd4(m, sp.wrapping_add(8), pair_176fd4(m));
        wr(m, a0.wrapping_add(0xC0), rd(m, sp.wrapping_add(8)));
        wr(m, a0.wrapping_add(0xC4), rd(m, sp.wrapping_add(0xC)));
    }
    if rd(m, a0.wrapping_add(0xCC)) != 0 {
        r5 = route_or_host(
            m,
            &mut *h,
            rd(m, rd(m, a0.wrapping_add(0xC8)).wrapping_add(0x14)),
            &[rd(m, a0.wrapping_add(0xCC)), rd(m, a0.wrapping_add(0xD0))],
        );
        if r5 >= 0x80000000 {
            wr(m, a0, 0);
            return fn_160fec(r5);
        }
        tts_core::boot_part_d::fn_176fd4(m, sp.wrapping_add(8), pair_176fd4(m));
        wr(m, a0.wrapping_add(0xCC), rd(m, sp.wrapping_add(8)));
        wr(m, a0.wrapping_add(0xD0), rd(m, sp.wrapping_add(0xC)));
    }
    let ctx = rd(m, sp);
    let sub10 = rd(m, rd(m, ctx).wrapping_add(0x10));
    if sub10 != 0 {
        let _ = route_or_host(m, &mut *h, 0x196448, &[r6, 0, 0]);
        r5 = route_or_host(m, &mut *h, 0x14DC30, &[rd(m, rd(m, sp).wrapping_add(0x10))]);
        if r5 >= 0x80000000 {
            wr(m, a0, 0);
            return fn_160fec(r5);
        }
        wr(m, rd(m, sp).wrapping_add(0x10), 0);
    }
    if rd(m, a0.wrapping_add(0xB4)) != 0 {
        r5 = fn_14d6dc(m, &mut *h, rd(m, a0.wrapping_add(0xB4)));
        if r5 >= 0x80000000 {
            wr(m, a0, 0);
            return fn_160fec(r5);
        }
        wr(m, a0.wrapping_add(0xB4), 0);
    }
    let ctx2 = rd(m, sp);
    let sub18 = rd(m, rd(m, ctx2).wrapping_add(0x18));
    if sub18 != 0 {
        r5 = route_or_host(m, &mut *h, 0x17AF38, &[sub18]);
        if r5 >= 0x80000000 {
            wr(m, a0, 0);
            return fn_160fec(r5);
        }
        wr(m, rd(m, sp).wrapping_add(0x18), 0);
    }
    let sub14 = rd(m, rd(m, rd(m, sp)).wrapping_add(0x14));
    if sub14 != 0 {
        r5 = route_or_host(m, &mut *h, 0x174E00, &[sub14]);
        if r5 >= 0x80000000 {
            wr(m, a0, 0);
            return fn_160fec(r5);
        }
        wr(m, rd(m, sp).wrapping_add(0x14), 0);
    }
    let sub0c = rd(m, rd(m, rd(m, sp)).wrapping_add(0xC));
    if sub0c != 0 {
        r5 = route_or_host(m, &mut *h, 0x177CC8, &[sub0c]);
        if r5 >= 0x80000000 {
            wr(m, a0, 0);
            return fn_160fec(r5);
        }
        wr(m, rd(m, sp).wrapping_add(0xC), 0);
    }
    if rd(m, a0.wrapping_add(0xB8)) != 0 {
        r5 = fn_177558(m, &mut *h, rd(m, a0.wrapping_add(0xB8)));
        if r5 >= 0x80000000 {
            wr(m, a0, 0);
            return fn_160fec(r5);
        }
        wr(m, a0.wrapping_add(0xB8), 0);
    }
    if rd(m, a0.wrapping_add(0xA8)) != 0 {
        r5 = route_or_host(
            m,
            &mut *h,
            0x196238,
            &[rd(m, a0.wrapping_add(0xA8)), rd(m, a0.wrapping_add(0xAC))],
        );
        if r5 >= 0x80000000 {
            wr(m, a0, 0);
            return fn_160fec(r5);
        }
        tts_core::boot_part_d::fn_176fd4(m, sp.wrapping_add(8), pair_176fd4(m));
        wr(m, a0.wrapping_add(0xA8), rd(m, sp.wrapping_add(8)));
        wr(m, a0.wrapping_add(0xAC), rd(m, sp.wrapping_add(0xC)));
    }
    if r7 != 0 {
        r5 = fn_172e84(m, &mut *h, r7);
        if r5 >= 0x80000000 {
            wr(m, a0, 0);
            return fn_160fec(r5);
        }
    }
    if rd(m, sp.wrapping_add(4)) != 0 {
        r5 = fn_176e10(m, &mut *h, sp.wrapping_add(4));
        if r5 >= 0x80000000 {
            wr(m, a0, 0);
            return fn_160fec(r5);
        }
    }
    if r6 != 0 {
        r5 = route_or_host(m, &mut *h, 0x196368, &[r6]);
        if r5 >= 0x80000000 {
            wr(m, a0, 0);
            return fn_160fec(r5);
        }
    }
    let _ = route_or_host(m, &mut *h, rd(m, a0.wrapping_add(0x10)), &[r8, a0]);
    fn_160fec(r5)
}

#[allow(clippy::too_many_arguments)]
pub fn fn_197c78(
    m: &mut Memory<'_>,
    h: &mut impl Calls,
    a0: u32,
    a1: u32,
    a2: u32,
    a3: u32,
    s0: u32,
    s1: u32,
) -> u32 {
    let sp = fn_129778(m, &mut *h, 0x4C);
    if sp == 0 {
        return 0x80000000;
    }
    for i in (0..0x4Cu32).step_by(4) {
        wr(m, sp.wrapping_add(i), 0);
    }
    let table = rd(m, 0x19762C);
    wr(m, sp.wrapping_add(0x28), rd(m, table.wrapping_add(0x28)));
    wr(m, sp.wrapping_add(0x2C), rd(m, table.wrapping_add(0x2C)));
    wr(m, sp.wrapping_add(0x30), rd(m, table.wrapping_add(0x30)));

    if a2 == 0 || rb(m, a2) == 0 {
        if a3 == 0 || rb(m, a3) == 0 {
            return rd(m, 0x198064);
        }
    }
    if s0 == 0 || s1 == 0 {
        return rd(m, 0x198064);
    }
    let status = fn_19611c(m, a0, a1, sp.wrapping_add(0x40));
    if status >= 0x80000000 {
        return status;
    }
    wr(m, s0, 0);
    fn_195bec(m, sp.wrapping_add(8), 0, 0x10);
    wr(m, s1, 0);
    let fp = rd(m, 0x197600);
    let r6 = rd(m, sp.wrapping_add(0x40));
    let r5;
    if a2 != 0 && rb(m, a2) != 0 {
        wr(m, sp, sp.wrapping_add(0x24));
        wr(m, sp.wrapping_add(8), rd(m, r6.wrapping_add(4)));

        let q = fn_199034(
            m,
            &mut *h,
            rd(m, r6.wrapping_add(0x1C)),
            rd(m, r6.wrapping_add(4)),
            a2,
            0x19805C,
            sp.wrapping_add(0x24),
        );
        if q >= 0x80000000 {
            let _ = fn_1990ec(m, &mut *h, rd(m, sp.wrapping_add(0x24)));
            if rd(m, sp.wrapping_add(0xC)) != 0 {
                let _ = route_or_host(
                    m,
                    &mut *h,
                    0x196428,
                    &[rd(m, r6.wrapping_add(4)), rd(m, sp.wrapping_add(0xC))],
                );
                wr(m, sp.wrapping_add(0xC), 0);
                wr(m, sp.wrapping_add(0x10), 0);
            }
            return unwind_197c78(m, h, sp, s0, s1, rd(m, 0x198060), a2);
        }
        let v = fn_1991d0(m, &mut *h, rd(m, sp.wrapping_add(0x24)));
        wr(m, sp, v);

        let mut dd8 = false;
        if v == 0 {
            r5 = fp;
            dd8 = true;
        } else {
            let w = route_or_host(
                m,
                &mut *h,
                0x196394,
                &[rd(m, r6.wrapping_add(4)), v.wrapping_add(1)],
            );
            wr(m, sp.wrapping_add(0xC), w);
            if w == 0 {
                r5 = rd(m, 0x1974FC);
                dd8 = true;
            } else {
                let x = fn_199134(m, &mut *h, w, 1, rd(m, sp), rd(m, sp.wrapping_add(0x24)));
                if x == 0 {
                    r5 = fp;
                    dd8 = true;
                } else {
                    wb(m, rd(m, sp.wrapping_add(0xC)).wrapping_add(x), 0);
                    r5 = 0;
                }
            }
        }
        let _ = fn_1990ec(m, &mut *h, rd(m, sp.wrapping_add(0x24)));
        if dd8 {
            if rd(m, sp.wrapping_add(0xC)) != 0 {
                let _ = route_or_host(
                    m,
                    &mut *h,
                    0x196428,
                    &[rd(m, r6.wrapping_add(4)), rd(m, sp.wrapping_add(0xC))],
                );
                wr(m, sp.wrapping_add(0xC), 0);
                wr(m, sp.wrapping_add(0x10), 0);
            }
        } else {
            wr(m, sp.wrapping_add(0x10), rd(m, sp.wrapping_add(0xC)));
        }
        return unwind_197c78(m, h, sp, s0, s1, r5, a2);
    }

    wr(m, sp, a3);
    wr(m, sp.wrapping_add(8), rd(m, r6.wrapping_add(4)));
    if fn_195d90(m, rd(m, sp)) == 0 {
        return unwind_197c78(m, h, sp, s0, s1, fp, a2);
    }
    let w = route_or_host(
        m,
        &mut *h,
        0x196394,
        &[
            rd(m, r6.wrapping_add(4)),
            fn_195d90(m, rd(m, sp)).wrapping_add(1),
        ],
    );
    wr(m, sp.wrapping_add(0xC), w);
    if w == 0 {
        return unwind_197c78(m, h, sp, s0, s1, rd(m, 0x1975FC), a2);
    }
    fn_195cd0(m, w, rd(m, sp));
    wr(m, sp.wrapping_add(0x10), rd(m, sp.wrapping_add(0xC)));
    unwind_197c78(m, h, sp, s0, s1, 0, a2)
}

fn unwind_197c78(
    m: &mut Memory<'_>,
    h: &mut impl Calls,
    sp: u32,
    s0: u32,
    s1: u32,
    r5: u32,
    a2: u32,
) -> u32 {
    let mut r5 = r5;
    let mut r4 = 0u32;

    let mut skip_head = false;
    loop {
        if !skip_head && rd(m, sp.wrapping_add(0x20)) != 0 {
            fn_19665c(m, &mut *h, rd(m, sp.wrapping_add(0x20)));
            wr(m, sp.wrapping_add(0x20), 0);
        }
        skip_head = false;

        if r5 < 0x80000000 {
            if r4 != 0 && rb(m, rd(m, sp.wrapping_add(0x10))) == 0 {
            } else {
                r5 = main_197c78(m, h, sp);
                if r5 >= 0x80000000 {
                    continue;
                }

                let (pr5, via_head) = parse_197c78(m, h, sp, s0, s1, &mut r4, a2);
                r5 = pr5;
                skip_head = !via_head;
                continue;
            }
        }

        if rd(m, sp.wrapping_add(0xC)) != 0 {
            let r0 = rd(m, sp.wrapping_add(0x40));
            let _ = route_or_host(
                m,
                &mut *h,
                0x196428,
                &[rd(m, r0.wrapping_add(4)), rd(m, sp.wrapping_add(0xC))],
            );
            wr(m, sp.wrapping_add(0xC), 0);
            wr(m, sp.wrapping_add(0x10), 0);
        }

        if r5 >= 0x80000000 && rd(m, s1) != 0 {
            r4 = 0;
            wr(m, sp.wrapping_add(4), 0);
            loop {
                if fn_1970ac(m, rd(m, s1), r4, sp.wrapping_add(4)) == 0 {
                    break;
                }
                fn_19665c(m, &mut *h, rd(m, rd(m, sp.wrapping_add(4))));
                r4 = r4.wrapping_add(1);
            }
            fn_196c7c(m, &mut *h, rd(m, s1));
            wr(m, s1, 0);
        }
        if r5 < 0x80000000 {
            return r5;
        }
        if rd(m, s0) != 0 {
            fn_19665c(m, &mut *h, rd(m, s0));
            wr(m, s0, 0);
        }
        return r5;
    }
}

fn main_197c78(m: &mut Memory<'_>, h: &mut impl Calls, sp: u32) -> u32 {
    fn_195bec(m, sp.wrapping_add(0x18), 0, 0xC);
    let r6 = sp.wrapping_add(8);
    if fn_195df8(m, rd(m, sp.wrapping_add(0x10)), 0x198068, 5) != 0 {
        return 0;
    }

    let mut cur = rd(m, sp.wrapping_add(0x10));
    let b5 = rb(m, cur.wrapping_add(5));
    if b5 != 0x20 && b5 != 0x09 && b5 != 0x0A && b5 != 0x0D {
        return 0;
    }
    cur = cur.wrapping_add(6);
    loop {
        let b = rb(m, cur);
        if b != 0x20 && b != 0x09 && b != 0x0A && b != 0x0D {
            break;
        }
        cur = cur.wrapping_add(1);
    }
    wr(m, sp.wrapping_add(0x10), cur);
    if fn_195df8(m, cur, 0x198070, 0xF) != 0 && fn_195df8(m, cur, 0x198080, 0xF) != 0 {
        wr(m, sp.wrapping_add(0x10), cur.wrapping_add(0xF));
        return 0;
    }

    wr(m, sp.wrapping_add(4), 0);
    fn_195bec(m, sp.wrapping_add(0x34), 0, 0xC);
    let mut r5 = route_or_host(m, &mut *h, 0x197254, &[r6, sp.wrapping_add(0x34)]);

    if r5 == 0 {
        if rd(m, sp.wrapping_add(0x38)) != 0
            && fn_195df8(m, rd(m, sp.wrapping_add(0x10)), 0x198090, 2) == 0
        {
            let v = route_or_host(
                m,
                &mut *h,
                0x1966AC,
                &[rd(m, sp.wrapping_add(0x38)), 0x198094, sp.wrapping_add(4)],
            );
            if v < 0x80000000 && fn_195d58(m, rd(m, sp.wrapping_add(4)), 0x19809C) == 0 {
                wr(
                    m,
                    sp.wrapping_add(0x10),
                    rd(m, sp.wrapping_add(0x10)).wrapping_add(2),
                );
                return r5;
            }
        }
        r5 = rd(m, 0x197600);
    } else {
        wr(
            m,
            sp.wrapping_add(0x10),
            rd(m, sp.wrapping_add(0x10)).wrapping_add(2),
        );
    }

    if rd(m, sp.wrapping_add(0x38)) != 0 {
        fn_19665c(m, &mut *h, rd(m, sp.wrapping_add(0x38)));
        wr(m, sp.wrapping_add(0x38), 0);
    }
    r5
}

fn parse_197c78(
    m: &mut Memory<'_>,
    h: &mut impl Calls,
    sp: u32,
    s0: u32,
    s1: u32,
    r4: &mut u32,
    a2: u32,
) -> (u32, bool) {
    wb(m, sp.wrapping_add(0x14), 0);
    let mut r5 = route_or_host(
        m,
        &mut *h,
        0x197630,
        &[sp.wrapping_add(8), sp.wrapping_add(0x18)],
    );
    if r5 != 0 {
        let tp = rd(m, sp.wrapping_add(0x10));
        let mut bytes = Vec::new();
        for i in 0..600u32 {
            let b = rb(m, tp.wrapping_add(i));
            if b == 0 {
                break;
            }
            bytes.push(b);
        }
    }

    if r5 == 0 {
        if rd(m, sp.wrapping_add(0x20)) != 0
            && fn_196764(m, rd(m, sp.wrapping_add(0x20))) != 0
            && rd(m, sp.wrapping_add(0x1C)) == 0
            && rd(m, sp.wrapping_add(0x18)) == 0
        {
            let mut cur = rd(m, sp.wrapping_add(0x10));
            loop {
                let b = rb(m, cur);
                if b != 0x20 && b != 0x09 && b != 0x0A && b != 0x0D {
                    break;
                }
                cur = cur.wrapping_add(1);
            }
            wr(m, sp.wrapping_add(0x10), cur);
        } else {
            r5 = rd(m, 0x197600);
        }
    }

    if rd(m, sp.wrapping_add(0x1C)) != 0 {
        fn_19665c(m, &mut *h, rd(m, sp.wrapping_add(0x1C)));
        wr(m, sp.wrapping_add(0x1C), 0);
    }

    if r5 != 0 {
        return (r5, true);
    }

    if a2 != 0 && rb(m, a2) != 0 {
        fn_195bec(m, sp.wrapping_add(0x34), 0, 0xC);
        wr(m, sp.wrapping_add(0x34), a2);
        r5 = fn_19668c(
            m,
            &mut *h,
            rd(m, sp.wrapping_add(0x20)),
            0x198278,
            sp.wrapping_add(0x34),
        );

        if r5 != 0 {
            return (r5, true);
        }
    }

    if *r4 == 0 {
        wr(m, s0, rd(m, sp.wrapping_add(0x20)));
    } else if *r4 == 1 {
        wr(m, sp, 8);
        wr(m, sp.wrapping_add(4), s1);
        let head = rd(m, rd(m, sp.wrapping_add(0x40)).wrapping_add(4));

        if fn_196a64(
            m,
            &mut *h,
            head,
            rd(m, sp.wrapping_add(0x28)),
            rd(m, sp.wrapping_add(0x2C)),
            rd(m, sp.wrapping_add(0x30)),
            rd(m, sp),
            rd(m, sp.wrapping_add(4)),
        ) == 0
        {
            return (rd(m, 0x1975FC), true);
        }
        if fn_196db0(m, &mut *h, rd(m, s1), s0) == 0 {
            return (rd(m, 0x1975FC), true);
        }
        wr(m, s0, 0);

        if fn_196db0(m, &mut *h, rd(m, s1), sp.wrapping_add(0x20)) == 0 {
            return (rd(m, 0x1975FC), true);
        }
    } else if fn_196db0(m, &mut *h, rd(m, s1), sp.wrapping_add(0x20)) == 0 {
        return (rd(m, 0x1975FC), true);
    }

    *r4 = r4.wrapping_add(1);

    (r5, false)
}

pub fn fn_197630(m: &mut Memory<'_>, h: &mut impl Calls, a0: u32, a1: u32) -> u32 {
    let sp = fn_129778(m, &mut *h, 0x48);
    if sp == 0 {
        return 0x80000000;
    }
    for i in (0..0x48u32).step_by(4) {
        wr(m, sp.wrapping_add(i), 0);
    }
    let r4 = a0;
    let r8 = a1;
    let mut r6 = 0u32;
    let mut r7 = 0u32;
    let sb = rb(m, r4.wrapping_add(0xC));
    {
        let c0 = rd(m, r4.wrapping_add(8));
        let mut bytes = Vec::new();
        for i in 0..24u32 {
            let b = rb(m, c0.wrapping_add(i));
            if b == 0 {
                break;
            }
            bytes.push(b);
        }
    }
    let sl = rd(m, 0x197600);
    wr(m, sp.wrapping_add(0x1C), 0x197610);
    wr(m, sp, 0);
    fn_195bec(m, sp.wrapping_add(0x10), 0, 0xC);

    loop {
        let cur = rd(m, r4.wrapping_add(8));
        let b = rb(m, cur);
        if b == 0x20 || b == 0x09 || b == 0x0A || b == 0x0D {
            wr(m, r4.wrapping_add(8), cur.wrapping_add(1));
        } else {
            break;
        }
    }
    let mut cur = rd(m, r4.wrapping_add(8));
    if rb(m, cur.wrapping_add(1)) == 0x21 {
        let _ = route_or_host(m, &mut *h, 0x198358, &[r4]);
    }
    cur = rd(m, r4.wrapping_add(8));
    if rb(m, cur) != 0x3C {
        r6 = sl;
    } else {
        wr(m, r4.wrapping_add(8), cur.wrapping_add(1));
        cur = rd(m, r4.wrapping_add(8));
        wr(m, sp.wrapping_add(0x1C), cur);
        loop {
            let p = rd(m, r4.wrapping_add(8));
            let b = rb(m, p);
            let mut adv = false;
            if b.wrapping_sub(0x61) <= 0x19 {
                adv = true;
            } else if b.wrapping_sub(0x41) <= 0x19 {
                adv = true;
            } else if b.wrapping_sub(0x30) <= 0x09 {
                adv = true;
            } else if b == 0x2E || b == 0x2D {
                adv = true;
            } else if b == 0x5F || b == 0x3A {
                adv = true;
            }
            if adv {
                wr(m, r4.wrapping_add(8), p.wrapping_add(1));
            } else {
                cur = p;
                break;
            }
        }
        let b = rb(m, cur);
        if b == 0x20 || b == 0x09 || b == 0x0A || b == 0x0D {
            wb(m, cur, 0);
            wr(m, r4.wrapping_add(8), cur.wrapping_add(1));
            r6 = route_or_host(m, &mut *h, 0x197254, &[r4, sp.wrapping_add(0x10)]);
            if r6 == 0 {
                let p = rd(m, r4.wrapping_add(8));
                let c = rb(m, p);
                if c == 0x3E {
                    wr(m, r4.wrapping_add(8), p.wrapping_add(1));
                } else if c == 0x2F && rb(m, p.wrapping_add(1)) == 0x3E {
                    r7 = 1;
                    wr(m, r4.wrapping_add(8), p.wrapping_add(2));
                } else {
                    r6 = sl;
                }
            }

            tagend_197630(m, &mut *h, sp, r4, r8, &mut r6, &mut r7, sb, sl);
        } else if b == 0x3E {
            wb(m, cur, 0);
            wr(m, r4.wrapping_add(8), cur.wrapping_add(1));
            tagend_197630(m, &mut *h, sp, r4, r8, &mut r6, &mut r7, sb, sl);
        } else if b == 0x2F {
            if rb(m, cur.wrapping_add(1)) != 0x3E {
                r6 = sl;
            } else {
                wb(m, cur, 0);
                wr(m, r4.wrapping_add(8), cur.wrapping_add(2));
                r7 = 1;
            }
            tagend_197630(m, &mut *h, sp, r4, r8, &mut r6, &mut r7, sb, sl);
        } else {
            r6 = sl;
            tagend_197630(m, &mut *h, sp, r4, r8, &mut r6, &mut r7, sb, sl);
        }
    }

    check_197630(m, &mut *h, sp, r4, r8, &mut r6, &mut r7, sb, sl)
}

fn tagend_197630(
    m: &mut Memory<'_>,
    _h: &mut impl Calls,
    sp: u32,
    r4: u32,
    _r8: u32,
    r6: &mut u32,
    _r7: &mut u32,
    sb: u8,
    sl: u32,
) {
    let mode = rb(m, r4.wrapping_add(0xC));
    if mode == 0 {
        if fn_195d58(m, rd(m, sp.wrapping_add(0x1C)), 0x197B08) != 0
            && fn_195d58(m, rd(m, sp.wrapping_add(0x1C)), 0x197B14) != 0
        {
            *r6 = sl;
        } else {
            wb(m, r4.wrapping_add(0xC), 1);
        }
    } else if mode == 1 {
        if fn_195d58(m, rd(m, sp.wrapping_add(0x1C)), 0x197B1C) != 0 {
            *r6 = sl;
        }
    } else if mode == 2 {
        if fn_195d58(m, rd(m, sp.wrapping_add(0x1C)), 0x197B24) != 0 {
            *r6 = sl;
        } else {
            wb(m, r4.wrapping_add(0xC), 3);
        }
    }
    let _ = sb;
}

fn check_197630(
    m: &mut Memory<'_>,
    h: &mut impl Calls,
    sp: u32,
    r4: u32,
    r8: u32,
    r6: &mut u32,
    r7: &mut u32,
    sb: u8,
    sl: u32,
) -> u32 {
    if (*r6 | *r7) == 0 {
        scan_197630(m, &mut *h, sp, r4, r8, r6, r7, sb, sl)
    } else {
        exit_197630(m, &mut *h, sp, r4, r8, r6, r7, sb, sl)
    }
}

fn scan_197630(
    m: &mut Memory<'_>,
    h: &mut impl Calls,
    sp: u32,
    r4: u32,
    r8: u32,
    r6: &mut u32,
    r7: &mut u32,
    sb: u8,
    sl: u32,
) -> u32 {
    loop {
        let cur = rd(m, r4.wrapping_add(8));
        let b = rb(m, cur);
        if b == 0x20 || b == 0x09 || b == 0x0A || b == 0x0D {
            wr(m, r4.wrapping_add(8), cur.wrapping_add(1));
        } else {
            break;
        }
    }
    let cur = rd(m, r4.wrapping_add(8));
    if rb(m, cur.wrapping_add(1)) == 0x21 {
        tts_core::boot_part_b::fn_198358(m, r4);
    }
    let cur = rd(m, r4.wrapping_add(8));
    wr(m, sp.wrapping_add(0x10), cur);
    exit2_197630(m, &mut *h, sp, r4, r8, r6, r7, sb, sl)
}

fn resolve_voice_record_197630(
    m: &mut Memory<'_>,
    h: &mut impl Calls,
    sp: u32,
    r4: u32,
    r8: u32,
    r6: &mut u32,
    r7: &mut u32,
    sb: u8,
    sl: u32,
) -> u32 {
    let cur = rd(m, r4.wrapping_add(8));
    if rb(m, cur.wrapping_add(1)) != 0x21 {
        return exit2_197630(m, &mut *h, sp, r4, r8, r6, r7, sb, sl);
    }
    loop {
        let p = rd(m, r4.wrapping_add(8));
        wr(m, r4.wrapping_add(8), p.wrapping_add(1));
        let p2 = rd(m, r4.wrapping_add(8));
        let hit = fn_195df8(m, p2, 0x197B2C, 4) == 0;
        if !hit {
            let nx = rd(m, r4.wrapping_add(8));
            if nx != p2 {
                continue;
            }
            return exit2_197630(m, &mut *h, sp, r4, r8, r6, r7, sb, sl);
        }

        let rc = route_or_host(m, &mut *h, 0x1960A8, &[rd(m, r4.wrapping_add(8)), 0x197B34]);
        let mut r0 = rc;
        if rc != 0 {
            if rb(m, rc.wrapping_sub(1)) == 0x2D {
                r0 = rc.wrapping_add(3);
                wr(m, r4.wrapping_add(8), r0);
                return exit2_197630(m, &mut *h, sp, r4, r8, r6, r7, sb, sl);
            }
        }

        loop {
            let b = rb(m, r0);
            if b == 0x20 || b == 0x09 || b == 0x0A || b == 0x0D {
                r0 = r0.wrapping_add(1);
                wr(m, r4.wrapping_add(8), r0);
            } else {
                break;
            }
        }
        let nx = rd(m, r4.wrapping_add(8));
        if nx != p2 {
            continue;
        }
        return exit2_197630(m, &mut *h, sp, r4, r8, r6, r7, sb, sl);
    }
}

fn exit2_197630(
    m: &mut Memory<'_>,
    h: &mut impl Calls,
    sp: u32,
    r4: u32,
    r8: u32,
    r6: &mut u32,
    r7: &mut u32,
    sb: u8,
    sl: u32,
) -> u32 {
    if (*r6 | *r7) != 0 {
        return exit_197630(m, &mut *h, sp, r4, r8, r6, r7, sb, sl);
    }
    let cur = rd(m, r4.wrapping_add(8));
    {
        let mut bytes = Vec::new();
        for i in 0..12u32 {
            let b = rb(m, cur.wrapping_add(i));
            if b == 0 {
                break;
            }
            bytes.push(b);
        }
    }
    let b = rb(m, cur);
    if b == 0x3C && rb(m, cur.wrapping_add(1)) == 0x2F {
        return exit_197630(m, &mut *h, sp, r4, r8, r6, r7, sb, sl);
    }
    content_197630(m, &mut *h, sp, r4, r8, r6, r7, sb, sl, cur, b)
}

fn tail_197630(
    m: &mut Memory<'_>,
    h: &mut impl Calls,
    sp: u32,
    r4: u32,
    r8: u32,
    r6: &mut u32,
    r7: &mut u32,
    sb: u8,
    sl: u32,
) -> u32 {
    loop {
        let cur = rd(m, r4.wrapping_add(8));
        let b = rb(m, cur);
        if b == 0x20 || b == 0x09 || b == 0x0A || b == 0x0D {
            wr(m, r4.wrapping_add(8), cur.wrapping_add(1));
        } else {
            break;
        }
    }
    resolve_voice_record_197630(m, &mut *h, sp, r4, r8, r6, r7, sb, sl)
}

fn content_197630(
    m: &mut Memory<'_>,
    h: &mut impl Calls,
    sp: u32,
    r4: u32,
    r8: u32,
    r6: &mut u32,
    r7: &mut u32,
    sb: u8,
    sl: u32,
    cur: u32,
    b: u8,
) -> u32 {
    if b == 0 {
        *r6 = sl;
        return tail_197630(m, &mut *h, sp, r4, r8, r6, r7, sb, sl);
    }
    if b == 0x3C {
        let anchor = rd(m, sp.wrapping_add(0x10));
        if anchor != 0 && cur != anchor {
            *r6 = sl;
            return tail_197630(m, &mut *h, sp, r4, r8, r6, r7, sb, sl);
        }
        wr(m, sp.wrapping_add(0x10), 0);
        *r6 = fn_197630(m, &mut *h, r4, sp.wrapping_add(0x10));
        return tail_197630(m, &mut *h, sp, r4, r8, r6, r7, sb, sl);
    }
    if rd(m, sp.wrapping_add(0x18)) == 0 {
        wr(m, sp, cur);
        wr(m, r4.wrapping_add(8), cur.wrapping_add(1));
    } else {
        *r6 = sl;
    }
    tail_197630(m, &mut *h, sp, r4, r8, r6, r7, sb, sl)
}

fn epi_197630(
    m: &mut Memory<'_>,
    h: &mut impl Calls,
    sp: u32,
    _r4: u32,
    _r8: u32,
    r6: u32,
    _r7: u32,
    _sb: u8,
    _sl: u32,
) -> u32 {
    {}
    if rd(m, sp.wrapping_add(0x14)) != 0 {
        fn_19665c(m, &mut *h, rd(m, sp.wrapping_add(0x14)));
    }
    if rd(m, sp.wrapping_add(0x18)) != 0 {
        fn_19665c(m, &mut *h, rd(m, sp.wrapping_add(0x18)));
    }
    wr(m, sp.wrapping_add(0x18), 0);
    r6
}

fn exit_197630(
    m: &mut Memory<'_>,
    h: &mut impl Calls,
    sp: u32,
    r4: u32,
    r8: u32,
    r6: &mut u32,
    r7: &mut u32,
    sb: u8,
    sl: u32,
) -> u32 {
    if *r6 == 0 {
        if rd(m, sp.wrapping_add(0x10)) != 0 {
            let s0 = rd(m, sp);
            if s0 != 0 {
                wb(m, s0.wrapping_add(1), 0);
            } else {
                let cur = rd(m, r4.wrapping_add(8));
                wb(m, cur, 0);
            }
        }
        return ret_197630(m, &mut *h, sp, r4, r8, r6, r7, sb, sl);
    }
    ret_197630(m, &mut *h, sp, r4, r8, r6, r7, sb, sl)
}

fn ret_197630(
    m: &mut Memory<'_>,
    h: &mut impl Calls,
    sp: u32,
    r4: u32,
    r8: u32,
    r6: &mut u32,
    r7: &mut u32,
    sb: u8,
    sl: u32,
) -> u32 {
    if (*r6 | *r7) != 0 {
        return store_197630(m, &mut *h, sp, r4, r8, r6, r7, sb, sl);
    }
    close_197630(m, &mut *h, sp, r4, r8, r6, r7, sb, sl)
}

fn close_197630(
    m: &mut Memory<'_>,
    h: &mut impl Calls,
    sp: u32,
    r4: u32,
    r8: u32,
    r6: &mut u32,
    r7: &mut u32,
    sb: u8,
    sl: u32,
) -> u32 {
    let r5 = fn_195d90(m, rd(m, sp.wrapping_add(0x1C)));
    let cur = rd(m, r4.wrapping_add(8)).wrapping_add(2);
    wr(m, r4.wrapping_add(8), cur);
    if fn_195df8(m, cur, rd(m, sp.wrapping_add(0x1C)), r5) != 0 {
        *r6 = sl;
        wb(m, r4.wrapping_add(0xC), sb);
        return epi_197630(m, &mut *h, sp, r4, r8, *r6, *r7, sb, sl);
    }

    let mut r0 = cur.wrapping_add(r5);
    wr(m, r4.wrapping_add(8), r0);
    loop {
        r0 = rd(m, r4.wrapping_add(8));
        let b = rb(m, r0);
        if b == 0x20 || b == 0x09 || b == 0x0A || b == 0x0D {
            wr(m, r4.wrapping_add(8), r0.wrapping_add(1));
        } else {
            break;
        }
    }
    let end = rd(m, r4.wrapping_add(8));
    if rb(m, end) != 0x3E {
        *r6 = sl;
        wb(m, r4.wrapping_add(0xC), sb);
        return epi_197630(m, &mut *h, sp, r4, r8, *r6, *r7, sb, sl);
    }
    wr(m, r4.wrapping_add(8), end.wrapping_add(1));
    if rd(m, sp.wrapping_add(0x10)) != 0 {
        *r6 = route_or_host(m, &mut *h, 0x198414, &[rd(m, sp.wrapping_add(0x10))]);
    }
    store_197630(m, &mut *h, sp, r4, r8, r6, r7, sb, sl)
}

fn store_197630(
    m: &mut Memory<'_>,
    h: &mut impl Calls,
    sp: u32,
    r4: u32,
    r8: u32,
    r6: &mut u32,
    r7: &mut u32,
    sb: u8,
    sl: u32,
) -> u32 {
    if *r6 != 0 {
        return epi_197630(m, &mut *h, sp, r4, r8, *r6, *r7, sb, sl);
    }
    wb(m, r4.wrapping_add(0xC), sb);
    let _ = sl;
    if sb == 0 {
        if rd(m, sp.wrapping_add(0x14)) != 0 || rd(m, sp.wrapping_add(0x18)) == 0 {
            *r6 = rd(m, 0x197600);
            return epi_197630(m, &mut *h, sp, r4, r8, *r6, *r7, sb, sl);
        }
        wr(m, r8.wrapping_add(8), rd(m, sp.wrapping_add(0x18)));
        wr(m, sp.wrapping_add(0x18), 0);
    } else if sb == 1 {
        if rd(m, sp.wrapping_add(0x14)) != 0 {
            *r6 = rd(m, 0x197600);
            return epi_197630(m, &mut *h, sp, r4, r8, *r6, *r7, sb, sl);
        }
        if rd(m, sp.wrapping_add(0x18)) != 0 {
            *r6 = rd(m, 0x197600);
            return epi_197630(m, &mut *h, sp, r4, r8, *r6, *r7, sb, sl);
        }
        if rd(m, sp.wrapping_add(0x10)) == 0 {
            *r6 = rd(m, 0x197600);
            return epi_197630(m, &mut *h, sp, r4, r8, *r6, *r7, sb, sl);
        }
        if fn_195d58(m, rd(m, sp.wrapping_add(0x10)), 0x197C60) != 0
            && fn_195d58(m, rd(m, sp.wrapping_add(0x10)), 0x197C6C) != 0
        {
            *r6 = rd(m, 0x197600);
            return epi_197630(m, &mut *h, sp, r4, r8, *r6, *r7, sb, sl);
        }

        wb(m, r4.wrapping_add(0xC), 2);
    } else if sb == 2 {
        if rd(m, sp.wrapping_add(0x14)) != 0 || rd(m, sp.wrapping_add(0x18)) == 0 {
            *r6 = rd(m, 0x197600);
            return epi_197630(m, &mut *h, sp, r4, r8, *r6, *r7, sb, sl);
        }
        wr(m, r8.wrapping_add(8), rd(m, sp.wrapping_add(0x18)));
        wr(m, sp.wrapping_add(0x18), 0);
    } else if rd(m, r8.wrapping_add(8)) == 0 {
        let tab = rd(m, 0x19762C);
        wr(m, sp.wrapping_add(0xC), r8.wrapping_add(8));
        wr(m, sp, rd(m, tab.wrapping_add(0x1C)));
        wr(m, sp.wrapping_add(4), rd(m, tab.wrapping_add(0x20)));
        wr(m, sp.wrapping_add(8), rd(m, tab.wrapping_add(0x24)));
        let rc = fn_1964e0(
            m,
            &mut *h,
            rd(m, r4),
            1,
            rd(m, tab.wrapping_add(0x14)),
            rd(m, tab.wrapping_add(0x18)),
            rd(m, tab.wrapping_add(0x1C)),
            rd(m, tab.wrapping_add(0x20)),
            rd(m, tab.wrapping_add(0x24)),
            r8.wrapping_add(8),
        );
        *r6 = rc;
        if rc != 0 {
            return epi_197630(m, &mut *h, sp, r4, r8, *r6, *r7, sb, sl);
        }
        *r6 = fn_19668c(
            m,
            &mut *h,
            rd(m, r8.wrapping_add(8)),
            rd(m, sp.wrapping_add(0x1C)),
            sp.wrapping_add(0x10),
        );
        if *r6 == 0 {
            wr(m, sp.wrapping_add(0x10), 0);
            wr(m, sp.wrapping_add(0x14), 0);
        }
    } else {
        *r6 = fn_19668c(
            m,
            &mut *h,
            rd(m, r8.wrapping_add(8)),
            rd(m, sp.wrapping_add(0x1C)),
            sp.wrapping_add(0x10),
        );
        if *r6 == 0 {
            wr(m, sp.wrapping_add(0x10), 0);
            wr(m, sp.wrapping_add(0x14), 0);
        }
    }
    epi_197630(m, &mut *h, sp, r4, r8, *r6, *r7, sb, sl)
}

pub fn fn_1757a8(m: &Memory<'_>, a0: u32) -> u32 {
    if a0 == 0 {
        rd(m, 0x17551C)
    } else {
        0
    }
}

pub fn fn_1758ac(m: &mut Memory<'_>, h: &mut impl Calls, a0: u32, a1: u32, a2: u32) -> u32 {
    let sp = fn_129778(m, h, 4208);
    let mut hh = HeapCalls { inner: h };
    tts_core::boot_port_1::fn_1758ac(m, a0, a1, a2, sp, &mut hh)
}

pub fn fn_175944(m: &mut Memory<'_>, h: &mut impl Calls, a0: u32, a1: u32, a2: u32) -> u32 {
    let sp = fn_129778(m, &mut *h, 8);
    if sp == 0 {
        return 0x80000000;
    }
    wr(m, sp, 0);
    wr(m, sp.wrapping_add(4), 0);
    let r5 = route_or_host(m, &mut *h, 0x1756D0, &[a0, a1, sp, sp.wrapping_add(4)]);
    if r5 < 0x80000000 && rd(m, sp.wrapping_add(4)) > 1 {
        let v = route_or_host(m, &mut *h, 0x176CD8, &[rd(m, sp)]);
        wr(m, a2, v);
    } else {
        wr(m, a2, 0);
    }
    r5
}

pub fn fn_17b1c4(m: &mut Memory<'_>, h: &mut impl Calls, a0: u32, a1: u32) -> u32 {
    if a0 == 0 {
        return rd(m, 0x17B4EC);
    }
    let sp = fn_129778(m, &mut *h, 0x28);
    if sp == 0 {
        return 0x80000000;
    }
    for i in (0..0x28u32).step_by(4) {
        wr(m, sp.wrapping_add(i), 0);
    }
    let r4 = a0;
    let r6 = a1;
    wr(m, sp.wrapping_add(0x20), 0);
    let mut r5 = route_or_host(m, &mut *h, 0x176F2C, &[rd(m, r4.wrapping_add(0xC))]);
    if r5 >= 0x80000000 {
        return r5;
    }
    r5 = fn_1966ac(
        m,
        &mut *h,
        rd(m, r4.wrapping_add(0x10)),
        r6,
        sp.wrapping_add(0x20),
    );
    if r5 >= 0x80000000 {
        r5 = (r5.wrapping_shl(1).wrapping_shr(1)) | rd(m, 0x17B4FC);
    } else {
        let r1 = rd(m, sp.wrapping_add(0x20));
        let hw = rh(m, r1.wrapping_add(0x1C)) as u32;
        if hw == 1 {
            if rd(m, r1.wrapping_add(0x18)) != 0 {
                fn_195ae8(m, sp, rd(m, sp.wrapping_add(0x20)), 0x20);
                r5 = route_or_host(m, &mut *h, 0x19672C, &[rd(m, r4.wrapping_add(0x10)), r6]);
                let r3 = rd(m, sp.wrapping_add(0x18));
                r5 = h.call(m, r3, &[rd(m, r4), rd(m, r4.wrapping_add(4)), sp]);
            } else {
                let r0 = hw.wrapping_sub(1);
                wh(m, r1.wrapping_add(0x1C), r0 as u16);
            }
        } else if hw == 0 {
            r5 = rd(m, 0x17B4F8);
        } else {
            let r0 = hw.wrapping_sub(1);
            wh(m, r1.wrapping_add(0x1C), r0 as u16);
        }
    }
    let r0 = route_or_host(m, &mut *h, 0x176F5C, &[rd(m, r4.wrapping_add(0xC))]);
    if r0 < 0x80000000 && r5 < 0x80000000 {
        r5 = r0;
    }
    r5
}

pub fn fn_15af9c(m: &mut Memory<'_>, h: &mut impl Calls, a0: u32, a1: u32, a2: u32) -> u32 {
    let sp = fn_129778(m, &mut *h, 0x18);
    if sp == 0 {
        return 0x80000000;
    }
    for i in (0..0x18u32).step_by(4) {
        wr(m, sp.wrapping_add(i), 0);
    }
    let r6 = a0;
    let r4 = a1;
    let r5 = a2;

    let pp = rd(m, 0x176FE8);
    let pair = [rd(m, pp), rd(m, pp.wrapping_add(4))];
    tts_core::boot_part_d::fn_176fd4(m, sp, pair);
    if rd(m, r4.wrapping_add(0x144)) != 0 {
        let r3 = rd(m, r4.wrapping_add(0x148));
        let r0 = rd(m, r4.wrapping_add(8));
        let r1 = rd(m, r4.wrapping_add(0xC));
        let _ = route_or_host(
            m,
            &mut *h,
            0x151BF8,
            &[r0, r1, rd(m, r4.wrapping_add(0x144)), r3],
        );
        tts_core::boot_part_d::fn_176fd4(m, sp, pair);
        wr(m, r4.wrapping_add(0x144), rd(m, sp));
        wr(m, r4.wrapping_add(0x148), rd(m, sp.wrapping_add(4)));
    }
    let mut r8 = 0u32;
    if r5 != 0 && rb(m, r5) != 0 {
        let mut r7 = fn_195d90(m, 0x15B0D0)
            .wrapping_add(fn_195d90(m, r5))
            .wrapping_add(fn_195d90(m, 0x15B0F0));
        r7 = route_or_host(
            m,
            &mut *h,
            0x196394,
            &[
                rd(m, r6.wrapping_add(4)),
                r7.wrapping_add(fn_195d90(m, 0x15B0F0)).wrapping_add(1),
            ],
        );
        fn_195cd0(m, r7, 0x15B0D0);
        let _ = route_or_host(m, &mut *h, 0x195C90, &[r7, r5]);
        let _ = route_or_host(m, &mut *h, 0x195C90, &[r7, 0x15B0F0]);
        let r1 = fn_195d90(m, r7);
        wr(m, sp.wrapping_add(0x10), r4.wrapping_add(0x144));
        wr(m, sp.wrapping_add(8), rd(m, sp));
        wr(m, sp.wrapping_add(0xC), rd(m, sp.wrapping_add(4)));
        wr(m, sp.wrapping_add(4), r1);
        wr(m, sp, r7);
        r8 = route_or_host(
            m,
            &mut *h,
            0x151914,
            &[
                rd(m, r4.wrapping_add(8)),
                rd(m, r4.wrapping_add(0xC)),
                0x15B104,
                rd(m, 0x15B100),
                r7,
                r1,
                rd(m, sp.wrapping_add(8)),
                rd(m, sp.wrapping_add(0xC)),
                r4.wrapping_add(0x144),
            ],
        );
        let _ = route_or_host(m, &mut *h, 0x196428, &[rd(m, r6.wrapping_add(4)), r7]);
    }
    r8
}

pub fn fn_15b128(m: &mut Memory<'_>, h: &mut impl Calls, a0: u32, a1: u32, a2: u32) -> u32 {
    let sp = fn_129778(m, &mut *h, 8);
    if sp == 0 {
        return 0x80000000;
    }
    wr(m, sp, 0);
    wr(m, sp.wrapping_add(4), 0);
    let r6 = a0;
    let r4 = a1;
    let r5 = a2;
    let kp = rd(m, r4.wrapping_add(0x10));
    if route_or_host(
        m,
        &mut *h,
        0x19611C,
        &[rd(m, kp), rd(m, kp.wrapping_add(4)), sp.wrapping_add(4)],
    ) >= 0x80000000
    {
    } else {
        let g = route_or_host(
            m,
            &mut *h,
            0x15ACCC,
            &[rd(m, rd(m, sp.wrapping_add(4)).wrapping_add(0x14))],
        );
        if g != 0 {
            return 2;
        }
    }
    if r5 == 0 {
        return 0;
    }
    let key = rd(m, r4.wrapping_add(0x10));
    if rd(m, key.wrapping_add(0x4C)) == 0 && rd(m, key.wrapping_add(0x58)) == 0 {
        return 0;
    }

    if route_or_host(
        m,
        &mut *h,
        0x175A1C,
        &[rd(m, r6.wrapping_add(0x14)), rd(m, 0x15B278), sp],
    ) >= 0x80000000
    {
        return 0;
    }
    if rd(m, sp) == 0 {
        return 0;
    }
    if route_or_host(m, &mut *h, 0x1769D0, &[rd(m, sp), rd(m, 0x15B27C)]) != 0 {
        return 0;
    }
    let hcount = rh(m, r5.wrapping_add(0xC)) as u32;
    let mut i = 0u32;
    loop {
        if hcount <= i {
            return 1;
        }
        let e = rd(m, rd(m, r5.wrapping_add(8)).wrapping_add(i.wrapping_mul(4)));
        let mm = rd(m, e.wrapping_add(4));
        if route_or_host(m, &mut *h, 0x176A48, &[mm, 0x15B280, 8]) == 0 {
            return 0;
        }
        if route_or_host(m, &mut *h, 0x176A48, &[mm, 0x15B28C, 7]) != 0 {
            i += 1;
            continue;
        }
        if route_or_host(m, &mut *h, 0x1769D0, &[mm, 0x15B294, 7]) == 0 {
            i += 1;
            continue;
        }
        return 0;
    }
}

pub fn fn_15bc18(
    m: &mut Memory<'_>,
    h: &mut impl Calls,
    _a0: u32,
    _a1: u32,
    a2: u32,
    a3: u32,
    a4: u32,
) -> u32 {
    let sp = fn_129778(m, &mut *h, 0x2C);
    if sp == 0 {
        return 0x80000000;
    }
    for i in (0..0x2Cu32).step_by(4) {
        wr(m, sp.wrapping_add(i), 0);
    }
    let r5 = a4;
    wr(m, sp.wrapping_add(0x20), 0);
    wr(m, sp.wrapping_add(0x24), 0);
    wr(m, sp.wrapping_add(0x1C), 0);
    wr(m, r5, 0);
    wr(m, r5.wrapping_add(4), 0);
    let rc11 = route_or_host(m, &mut *h, 0x19611C, &[a2, a3, sp.wrapping_add(0x1C)]);
    if rc11 >= 0x80000000 {
        return rc11;
    }
    {
        let ctx = rd(m, sp.wrapping_add(0x1C));
        let mut rh = RouteHost { inner: &mut *h };
        tts_core::log_live::fn_1738e8(
            m,
            rd(m, ctx.wrapping_add(0x10)),
            rd(m, 0x15B5D0),
            4,
            0,
            0x15BE18,
            rd(m, sp.wrapping_add(4)),
            &mut rh,
        );
    }
    let r4 = fn_1963b8(
        m,
        &mut *h,
        rd(m, rd(m, sp.wrapping_add(0x1C)).wrapping_add(4)),
        1,
        0x38,
    );
    if r4 == 0 {
        return 0;
    }
    wr(m, r4.wrapping_add(0x20), 1);
    wr(m, r4, rd(m, sp.wrapping_add(0x1C)));
    let p1 = rd(m, 0x15BE30);
    let p2 = rd(m, 0x15BE48);
    let p3 = rd(m, 0x15BE60);
    let p4 = rd(m, 0x15BE74);
    let p5 = rd(m, 0x15BE90);
    let tab = rd(m, rd(m, sp.wrapping_add(0x1C)).wrapping_add(0x14));
    if route_or_host(m, &mut *h, 0x175944, &[tab, p1, r4.wrapping_add(0x24)]) >= 0x80000000 {
        wr(m, r4.wrapping_add(0x24), 0x80);
    }
    if route_or_host(m, &mut *h, 0x175944, &[tab, p2, r4.wrapping_add(0x28)]) >= 0x80000000 {
        wr(m, r4.wrapping_add(0x28), 0x80);
    }
    if route_or_host(m, &mut *h, 0x175944, &[tab, p3, r4.wrapping_add(0x2C)]) >= 0x80000000 {
        wr(m, r4.wrapping_add(0x2C), 0xFFFF_FFFF);
    }
    if route_or_host(m, &mut *h, 0x175944, &[tab, p4, sp.wrapping_add(0x20)]) >= 0x80000000 {
        wr(m, sp.wrapping_add(0x20), 0x2800);
    }
    if route_or_host(m, &mut *h, 0x175944, &[tab, p5, sp.wrapping_add(0x24)]) >= 0x80000000 {
        wr(m, sp.wrapping_add(0x24), 0x1000);
    }
    wr(m, sp.wrapping_add(0x18), rd(m, r4.wrapping_add(0x20)));
    wr(m, sp.wrapping_add(0xC), rd(m, r4.wrapping_add(0x2C)));
    wr(m, sp.wrapping_add(0x10), rd(m, sp.wrapping_add(0x20)));
    wr(m, sp.wrapping_add(0x14), rd(m, sp.wrapping_add(0x24)));
    wr(m, sp, rd(m, 0x15BEA8));
    wr(m, sp.wrapping_add(4), rd(m, r4.wrapping_add(0x24)));
    wr(m, sp.wrapping_add(8), rd(m, r4.wrapping_add(0x28)));
    {
        let ctx = rd(m, r4);
        let mut rh = RouteHost { inner: &mut *h };
        tts_core::log_live::fn_1738e8(
            m,
            rd(m, rd(m, ctx.wrapping_add(0x10))),
            rd(m, 0x15B5D0),
            5,
            0,
            rd(m, sp),
            rd(m, sp.wrapping_add(4)),
            &mut rh,
        );
    }
    let r = route_or_host(
        m,
        &mut *h,
        0x180D00,
        &[
            r4.wrapping_add(4),
            rd(m, r4.wrapping_add(0x20)),
            rd(m, 0x15BEAC),
            0,
            rd(m, sp.wrapping_add(0xC)),
            rd(m, sp.wrapping_add(0x10)),
            rd(m, sp.wrapping_add(0x14)),
            rd(m, sp.wrapping_add(0x18)),
        ],
    );
    let _ = r;
    wr(m, r5, r4);
    wr(m, r5.wrapping_add(4), rd(m, 0x15B5B0));
    let mut r4c = route_or_host(m, &mut *h, 0x15BAB0, &[r4]);
    if r4c >= 0x80000000 {
        let _ = route_or_host(
            m,
            &mut *h,
            0x15B5F0,
            &[rd(m, r5), rd(m, r5.wrapping_add(4))],
        );
        wr(m, r5, 0);
        wr(m, r5.wrapping_add(4), 0);
    } else {
        r4c = r4;
    }
    {
        let ctx = rd(m, sp.wrapping_add(0x1C));
        let mut rh = RouteHost { inner: &mut *h };
        tts_core::log_live::fn_1738e8(
            m,
            rd(m, ctx.wrapping_add(0x10)),
            rd(m, 0x15B5D0),
            4,
            0,
            0x15BEB0,
            r4c,
            &mut rh,
        );
    }
    r4c
}

pub fn fn_197254(m: &mut Memory<'_>, h: &mut impl Calls, a0: u32, a1: u32) -> u32 {
    let sp = fn_129778(m, &mut *h, 0x1C);
    if sp == 0 {
        return 0x80000000;
    }
    for i in (0..0x1Cu32).step_by(4) {
        wr(m, sp.wrapping_add(i), 0);
    }
    let r4 = a0;
    let r7 = a1;
    let mut r2 = 0u32;

    loop {
        let cur = rd(m, r4.wrapping_add(8));
        let b = rb(m, cur);
        if b == 0x20 || b == 0x09 || b == 0x0A || b == 0x0D {
            wr(m, r4.wrapping_add(8), cur.wrapping_add(1));
        } else {
            break;
        }
    }
    let r8 = rd(m, 0x197600);
    wr(m, sp.wrapping_add(0x18), r7.wrapping_add(4));
    let sb = 0x26u8;
    let sl = 0x3Cu8;
    let fp = 0x3Eu8;
    let mut cur = rd(m, r4.wrapping_add(8));
    let mut b = rb(m, cur);
    if b == 0x2F || b == 0x3E || b == 0x3F {
        return r8;
    }

    let d = b.wrapping_sub(0x61);
    let ok = if d <= 0x19 {
        true
    } else {
        let d = b.wrapping_sub(0x41);
        if d <= 0x19 {
            true
        } else {
            b == 0x5F || b == 0x3A
        }
    };
    if !ok {
        return r8;
    }

    let mut r0 = b;
    wr(m, sp.wrapping_add(0x14), cur);
    cur = cur.wrapping_add(1);
    wr(m, r4.wrapping_add(8), cur);
    let mut r1 = cur;
    loop {
        r0 = rb(m, r1);
        let mut adv = false;
        if r0.wrapping_sub(0x61) <= 0x19 {
            adv = true;
        } else if r0.wrapping_sub(0x41) <= 0x19 {
            adv = true;
        } else if r0.wrapping_sub(0x30) <= 0x09 {
            adv = true;
        } else if r0 == 0x2E || r0 == 0x2D {
            adv = true;
        } else if r0 == 0x5F || r0 == 0x3A {
            adv = true;
        }
        if !adv {
            break;
        }
        r1 = r1.wrapping_add(1);
        wr(m, r4.wrapping_add(8), r1);
    }

    let b2 = rb(m, r1);
    if b2 == 0x20 || b2 == 0x09 || b2 == 0x0A || b2 == 0x0D {
        wb(m, r1, 0);
        cur = rd(m, r4.wrapping_add(8)).wrapping_add(1);
        loop {
            let bb = rb(m, cur);
            if bb == 0x20 || bb == 0x09 || bb == 0x0A || bb == 0x0D {
                cur = cur.wrapping_add(1);
                wr(m, r4.wrapping_add(8), cur);
            } else {
                break;
            }
        }

        r1 = cur;
    }
    let r0a = r1;
    b = rb(m, r1);
    if b != 0x3D {
        return r8;
    }
    wb(m, r0a, 0);

    cur = rd(m, r4.wrapping_add(8)).wrapping_add(1);
    loop {
        let bb = rb(m, cur);
        if bb == 0x20 || bb == 0x09 || bb == 0x0A || bb == 0x0D {
            cur = cur.wrapping_add(1);
            wr(m, r4.wrapping_add(8), cur);
        } else {
            break;
        }
    }

    r1 = cur;

    let r3 = rb(m, r1);
    if r3 != 0x22 && r3 != 0x27 {
        return r8;
    }
    let quote = r3;
    cur = r1.wrapping_add(1);
    wr(m, r4.wrapping_add(8), cur);
    wr(m, sp.wrapping_add(0x10), cur);

    loop {
        let c = rb(m, cur);
        if c == 0x3C || c == quote {
            break;
        }
        cur = cur.wrapping_add(1);
        wr(m, r4.wrapping_add(8), cur);
    }
    if rb(m, cur) != quote {
        return r8;
    }
    wb(m, cur, 0);
    wr(m, r4.wrapping_add(8), cur.wrapping_add(1));
    let r5 = fn_195d30(m, rd(m, sp.wrapping_add(0x10)), 0x26);
    if r5 != 0 {
        let mut r6 = r5;
        let mut p = r5;
        loop {
            let c = rb(m, p);
            if c == 0 {
                break;
            }
            if c != 0x26 {
                p = p.wrapping_add(1);
                wb(m, r6, c);
                r6 = r6.wrapping_add(1);
                continue;
            }
            if fn_195df8(m, p, 0x197604, 5) == 0 {
                p = p.wrapping_add(5);
                wb(m, r6, sb);
                r6 = r6.wrapping_add(1);
            } else if fn_195df8(m, p, 0x19760C, 4) == 0 {
                p = p.wrapping_add(4);
                wb(m, r6, sl);
                r6 = r6.wrapping_add(1);
            } else if fn_195df8(m, p, 0x197614, 4) == 0 {
                p = p.wrapping_add(4);
                wb(m, r6, fp);
                r6 = r6.wrapping_add(1);
            } else if fn_195df8(m, p, 0x19761C, 6) == 0 {
                p = p.wrapping_add(6);
                wb(m, r6, 0x27);
            } else if fn_195df8(m, p, 0x197628, 6) == 0 {
                p = p.wrapping_add(6);
                wb(m, r6, 0x22);
            } else {
                return r8;
            }
        }
        wb(m, r6, 0);
    }
    if r7 == 0 {
        return r2;
    }
    if rd(m, r7.wrapping_add(4)) == 0 {
        wr(m, sp.wrapping_add(0xC), rd(m, sp.wrapping_add(0x18)));
        let table = rd(m, 0x19762C);
        wr(m, sp, rd(m, table.wrapping_add(8)));
        wr(m, sp.wrapping_add(4), rd(m, table.wrapping_add(0xC)));
        wr(m, sp.wrapping_add(8), rd(m, table.wrapping_add(0x10)));

        r2 = fn_1964e0(
            m,
            &mut *h,
            rd(m, r4),
            0,
            rd(m, table),
            rd(m, table.wrapping_add(4)),
            rd(m, sp),
            rd(m, sp.wrapping_add(4)),
            rd(m, sp.wrapping_add(8)),
            rd(m, sp.wrapping_add(0xC)),
        );
        if r2 != 0 {
            return r2;
        }
    }
    r2 = fn_19668c(
        m,
        &mut *h,
        rd(m, r7.wrapping_add(4)),
        rd(m, sp.wrapping_add(0x14)),
        rd(m, sp.wrapping_add(0x10)),
    );
    r2
}

pub fn fn_173fd8(m: &mut Memory<'_>, h: &mut impl Calls, a0: u32) -> u32 {
    if a0 == 0 {
        return 0;
    }
    let mut r6 = 1u32;
    for i in 0..5u32 {
        let entry = a0.wrapping_add(i.wrapping_mul(8));
        let m1 = rd(m, entry.wrapping_add(0xC));
        if m1 == 0 {
            continue;
        }
        let m2 = rd(m, m1.wrapping_add(0x28));
        if m2 == 0 {
            continue;
        }
        let rc = h.call(m, m2, &[rd(m, entry.wrapping_add(8))]);
        if rc > r6 {
            r6 = rc;
        }
    }
    let t = rd(m, a0.wrapping_add(0x38));
    if t != 0 {
        let mt = rd(m, t.wrapping_add(0x28));
        if mt != 0 {
            let rc = h.call(m, mt, &[rd(m, a0.wrapping_add(0x34))]);
            if rc > r6 {
                r6 = rc;
            }
        }
    }
    if rb(m, a0.wrapping_add(0x31)) != 0 && r6 > 4 {
        r6 = 4;
    }
    r6
}

pub fn fn_14d4e4(m: &mut Memory<'_>, dst: u32, size: u32, src: u32, max: u32) -> u32 {
    let r4 = size.wrapping_sub(1);
    let lim = if r4 > max { max } else { r4 };
    let mut count: u32 = 0;
    let mut ip = dst;
    let mut s = src;
    if lim != 0 {
        loop {
            let c = rh(m, s);
            count = count.wrapping_add(1);
            wh(m, ip, c);
            ip = ip.wrapping_add(2);
            s = s.wrapping_add(2);
            if rh(m, s) == 0 {
                break;
            }

            if lim <= count {
                break;
            }
        }
    }
    wh(m, ip, 0);
    count
}

pub fn fn_128468(m: &mut Memory<'_>, obj: u32, a: u32) {
    let v = if a == 0 { rd(m, obj + 0x8C) } else { a };
    wr(m, obj + 0x88, v);
}

pub fn fn_19be58(m: &mut Memory<'_>, p: u32) {
    for i in (0..0x18u32).step_by(4) {
        wr(m, p.wrapping_add(i), 0);
    }
    wb(m, p.wrapping_add(0x11), 0);
}

pub fn fn_129750(m: &mut Memory<'_>, h: &mut impl Calls, size: u32) -> u32 {
    let table = HEAP_TABLE_ROOT;
    let fill = rb(m, FILL_FLAG) as u32;
    tts_core::speech_part_1::fn_11db30(m, size, 1, 32, fill, table, &mut HeapCalls { inner: h })
}

pub fn fn_128478(m: &mut Memory<'_>, h: &mut impl Calls, obj: u32, psize: u32, n: u32) -> u32 {
    wr(m, obj, rd(m, 0x12868C));
    wr(m, obj + 4, 0);
    wr(m, obj + 8, 0x40000000);
    wb(m, obj + 0xC, 0);
    wr(m, obj + 0x10, 0x00000000);
    wr(m, obj + 0x1C, 0);
    wr(m, obj + 0x20, 1);
    wr(m, obj + 0x24, 0);
    wr(m, obj + 0x2C, 0);
    wr(m, obj + 0x30, 0x3F800000);
    wr(m, obj + 0x34, 0x3F800000);
    for o in [0x38u32, 0x3C, 0x40, 0x44, 0x48] {
        wr(m, obj + o, 0);
    }
    wr(m, obj + 0x4C, 0);
    for o in [0x50u32, 0x54, 0x58] {
        wr(m, obj + o, 0);
    }
    wr(m, obj + 0x5C, 0);
    wr(m, obj + 0x60, 0xFFFF_FFFF);
    wr(m, obj + 0x64, 0);
    wr(m, obj + 0x68, 0);
    wr(m, obj + 0x6C, 0xFFFF_FFFF);
    for o in [0x70u32, 0x74, 0x78, 0x7C, 0x80] {
        wr(m, obj + o, 0);
    }
    wr(m, obj + 0x88, 0xC800);
    wr(m, obj + 0x8C, 0xC800);
    wb(m, obj + 0x9C, 0);
    wb(m, obj + 0x9D, 0);
    wr(m, obj + 0xA0, 0);
    wr(m, obj + 0xA4, 0);
    wb(m, obj + 0xA8, 0);
    wb(m, obj + 0xA9, 1);
    let count = n.clamp(1, 4);
    wr(m, obj + 0x18, count);
    wr(m, obj + 0x84, psize >> 1);

    let structs = fn_12973c(m, &mut *h, count.wrapping_mul(24), 1);
    wr(m, obj + 0x14, structs);

    let rows = fn_12973c(m, &mut *h, count.wrapping_mul(4), 1);
    wr(m, obj + 0x90, rows);
    for i in 0..count {
        let buf = fn_129750(m, &mut *h, psize);
        wr(m, rows.wrapping_add(i.wrapping_mul(4)), buf);
        fn_199404(m, buf, psize);
        fn_19be58(m, structs.wrapping_add(i.wrapping_mul(24)));
    }
    wr(m, obj + 0x78, rd(m, rows));
    wr(
        m,
        obj + 0x7C,
        rd(m, rows.wrapping_add(count.wrapping_mul(4))),
    );
    let aux = fn_129778(m, &mut *h, 0x50);
    wr(m, obj + 0x98, aux);
    fn_122058(m, obj + 0x58);
    fn_122058(m, obj + 0x64);
    {
        let mut rt = Rt { inner: &mut *h };
        m.fn_165b88(&mut rt, obj + 0x70, 0);
    }

    let gbl = rd(m, 0xACB570);
    let f1 = rd(m, 0x12869C);
    let f2 = rd(m, 0x1286A0);
    wr(m, gbl.wrapping_add(0x10), f1);
    wr(m, gbl.wrapping_add(0x14), obj);
    wr(m, gbl.wrapping_add(0x18), f2);
    wr(m, gbl.wrapping_add(0x1C), obj);
    obj
}

pub fn fn_10de80(m: &mut Memory<'_>, h: &mut impl Calls, obj: u32, psize: u32, n: u32) -> u32 {
    fn_128478(m, &mut *h, obj, psize, n);
    wr(m, obj, rd(m, 0x10DFD8));
    wr(m, obj + 0x100, 0);
    wr(m, obj + 0xFC, rd(m, 0x10DFDC));
    wr(m, obj + 0x104, 0);
    wr(m, obj + 0x108, 0);
    wr(m, obj + 0x10C, 0xFFFF_FFFF);
    wr(m, obj + 0x110, 0);
    wr(m, obj + 0x114, 0);
    wr(m, obj + 0x118, 0xFFFF_FFFF);
    wr(m, obj + 0x164, 0);
    wb(m, obj + 0x168, 1);
    wr(m, obj + 0x170, 0);
    wr(m, obj + 0x174, 0);
    wb(m, obj + 0x178, 0x11);
    wb(m, obj + 0x17A, 1);
    wr(m, obj + 0x6190, 0);
    wr(m, obj + 0x6194, 0);
    wr(m, obj + 0x6198, 0x1800);
    wr(m, obj + 0x619C, 0);
    wb(m, obj + 0x61A0, 0);
    wb(m, obj + 0x61A1, 0);
    wb(m, obj + 0x61A2, 0);
    wb(m, obj + 0x61A3, 0);
    wb(m, obj + 0x61A4, 0);
    wr(m, obj + 0x61C0, rd(m, 0x10DFE0));
    wr(m, obj + 0x61C4, 0);
    wr(m, obj + 0x61CC, 0);
    wr(m, obj + 0x61D0, 0);
    wr(m, obj + 0x61D4, 0);
    wr(m, obj + 0x61D8, rd(m, obj + 0x61D8) & !0xC000_0000);
    wr(m, obj + 0x61E0, 0);
    wr(m, obj + 0x61E4, 0);
    wr(m, obj + 0x61E8, 0);
    wr(m, obj + 0x61EC, 0);
    for i in 0..5u32 {
        wh(m, obj + 0x6A00 + i * 2, 0x32);
    }
    wh(m, obj + 0x6A0A, 0);
    fn_3023b0(m, obj + 0x18C, 0x6000);
    let count = fn_14d4e4(m, obj + 0x18C, 0x3000, rd(m, 0x10DFE4), rd(m, 0x10DFE8));
    wr(m, obj + 0x618C, count);
    fn_128468(m, obj, 0x7800);
    wr(m, obj + 0x8C, 0x7800);
    obj
}

pub fn fn_194790(m: &mut Memory<'_>, h: &mut impl Calls, a0: u32, a1: u32) -> u32 {
    let gbl = rd(m, 0x1940CC);
    if a1 != gbl {
        return 0x8000_0008;
    }
    if a0 == 0 || rd(m, a0) != gbl.wrapping_add(0x0B00_0000) {
        return 0x8000_0008;
    }
    let r4 = a0;
    let inner = rd(m, r4 + 0x10);
    let backend = rd(m, inner + 0x10);
    let sp = fn_129778(m, h, 8);
    route_or_host(
        m,
        h,
        0x1738E8,
        &[backend, rd(m, 0x19460C), 4, 0, 0x1948E8, sp],
    );
    let r0 = h.call(m, 0x176F2C, &[rd(m, r4 + 0x24)]);
    if r0 >= 0x8000_0000 {
        wr(m, sp, rd(m, 0x1948FC));
        wr(m, sp + 4, r0);
        let logger = rd(m, rd(m, r4 + 0x10) + 0x10);
        tts_core::cold_boot::fn_173dc0(
            m,
            h,
            logger,
            rd(m, 0x19460C),
            rd(m, 0x194904),
            rd(m, 0x194900),
            sp,
            sp,
        );
    }
    let r6 = rb(m, r4 + 0x28);
    let r5 = h.call(m, 0x176F5C, &[rd(m, r4 + 0x24)]);
    if r5 >= 0x8000_0000 {
        return r5;
    }
    if r6 != 3 && r6 != 4 {
        return 0x8000_0011;
    }
    r5
}

pub fn fn_35a9b0(m: &mut Memory<'_>, h: &mut impl Calls, obj: u32) {
    if rb(m, obj + 0x9D) != 0 {
        wb(m, obj + 0x61A1, 1);
        let a = rd(m, obj + 0x11C);
        let b = rd(m, obj + 0x120);
        fn_194790(m, &mut *h, a, b);
    }
    wr(m, obj + 0x61AC, 0);
    wr(m, obj + 0x61B0, 0);
    wb(m, obj + 0x61A0, 0);
    wr(m, obj + 0x80, 0);
    wb(m, obj + 0x61A2, 0);
}

pub fn fn_35adf8(m: &mut Memory<'_>, h: &mut impl Calls, obj: u32) {
    fn_35a9b0(m, &mut *h, obj);
    wb(m, obj + 0x9D, 1);
    wb(m, obj + 0x61A0, 0);
    wb(m, obj + 0x61A4, 0);
    wr(m, obj + 0x80, 0);
    wb(m, obj + 0x61A2, 0);
    wb(m, obj + 0x61A1, 0);
    wb(m, obj + 0x61B4, 0);
    audio_start_fields(m, h, obj, 0x3e80);

    let r1 = rd(m, obj + 0x61D0);
    let r3init = rd(m, obj + 0x61D4);
    let gt = r3init > r1;
    let mut r0 = if gt {
        r3init
            .wrapping_sub(r1)
            .wrapping_shl(0x1A)
            .wrapping_shr(0x1F)
    } else {
        0
    };
    let r3sub = (r3init as i32).wrapping_sub(r1 as i32);
    let ip = 1i32.wrapping_add(r3sub >> 5);
    r0 = r0.wrapping_add(1);
    if ip > r0 as i32 {
        let r3v = ip;
        r0 = r0.wrapping_add(2);
        while r3v > r0 as i32 {
            r0 = r0.wrapping_add(2);
        }
    }
    wr(m, obj + 0x61D4, r1);
}

pub fn fn_35a958(m: &mut Memory<'_>, h: &mut impl Calls, obj: u32) {
    if rb(m, obj + 0x61A0) != 0 {
        return;
    }
    if rb(m, obj + 0x9D) != 0 {
        fn_35a9b0(m, &mut *h, obj);
        wb(m, obj + 0x61A0, 1);
    } else {
        fn_35adf8(m, &mut *h, obj);
    }
}

pub fn fn_107230(m: &mut Memory<'_>, obj: u32) {
    wr(m, obj + 4, 0);
    wr(m, obj, rd(m, 0x107250));
    wr(m, obj + 8, 0);
    wr(m, obj + 0x10, 7);
}

pub fn fn_voice_setup(m: &mut Memory<'_>, h: &mut impl Calls, engine: u32) -> u32 {
    fn_10de80(m, &mut *h, engine, 0x4B000, 3);

    let hnd = fn_129778(m, &mut *h, 0x14);
    if hnd == 0 {
        return 0x8000000A;
    }
    fn_107230(m, hnd);
    wr(m, hnd + 8, engine);
    wb(m, hnd + 0xC, 1);
    wb(m, hnd + 0xD, 1);
    hnd
}

pub fn fn_1756d0(
    m: &mut Memory<'_>,
    h: &mut impl Calls,
    p: u32,
    key: u32,
    out: u32,
    len: u32,
) -> u32 {
    if p == 0 || key == 0 || out == 0 {
        return rd(m, 0x17551C);
    }
    let sp = fn_129778(m, h, 4);
    wr(m, sp, 0);
    wr(m, out, 0);
    if len != 0 {
        wr(m, len, 0);
    }
    let lock = rd(m, p.wrapping_add(4));
    let acq = route_or_host(m, h, 0x176F2C, &[lock]);
    if acq >= 0x80000000 {
        return acq;
    }
    let mut rc = fn_1966ac(m, h, rd(m, p.wrapping_add(8)), key, sp);
    if rc >= 0x80000000 {
        rc = (rc & 0x1fff) | rd(m, 0x175518);
    } else {
        let cell = rd(m, sp);
        if cell != 0 && rd(m, cell.wrapping_add(4)) != 0 {
            wr(m, out, rd(m, cell));
            if len != 0 {
                wr(m, len, rd(m, cell.wrapping_add(4)));
            }
        }
    }
    let unlock = route_or_host(m, h, 0x176F5C, &[lock]);
    if unlock >= 0x80000000 && rc < 0x80000000 {
        rc = unlock;
    }
    rc
}

struct PipelineHost<'a, H> {
    inner: &'a mut H,
}
impl<H: Calls> tts_core::boot_port_1b::Host for PipelineHost<'_, H> {
    fn owns_broker(&mut self) -> bool {
        true
    }
    fn owns_logger(&mut self) -> bool {
        true
    }
    fn call(&mut self, m: &mut Memory<'_>, f: u32, a: &[u32]) -> u32 {
        route_or_host(m, self.inner, f, a)
    }
    fn thread_id(&mut self) -> u32 {
        0x1FF82000
    }
    fn broker_table(&mut self) -> u32 {
        0x8DC588
    }
}
fn rom_text(m: &Memory<'_>, text: &[u8]) -> u32 {
    let image = &m.bytes[0x100000..0xA70000];
    let found = if text.is_empty() {
        memchr::memchr(0, image)
    } else {
        memchr::memmem::find_iter(image, text).find(|i| image.get(i + text.len()) == Some(&0))
    };
    found
        .map(|i| i as u32 + 0x100000)
        .unwrap_or_else(|| panic!("missing linked ROM text of length {}", text.len()))
}
fn pipeline_environment(
    m: &mut Memory<'_>,
    h: &mut impl Calls,
) -> tts_core::boot_port_1b::Environment {
    use tts_core::boot_port_1b::{ComponentBinding, Environment, LITERAL_ADDRESSES, SCRATCH_BYTES};
    let literals = std::array::from_fn(|i| {
        if i == 38 {
            rd(m, 0x17EEE8)
        } else {
            LITERAL_ADDRESSES[i]
        }
    });
    let scratch = fn_129778(m, h, SCRATCH_BYTES);
    let component_scratch = fn_129778(m, h, 4096);
    assert!(scratch != 0 && component_scratch != 0);
    let mut formats = [0; 12];
    formats[7] = literals[31];
    formats[8] = literals[40];
    formats[9] = literals[37];
    let mut names = [0; 16];
    names[10] = literals[41];
    let p = pair_176fd4(m);
    let null_pair = ((p[1] as u64) << 32 | p[0] as u64).to_le_bytes();
    Environment {
        literals,
        scratch,
        fold: std::array::from_fn(|i| rh(m, rd(m, 0x17C8F8) + i as u32 * 2)),
        null_pair,
        default_context: rd(m, 0x196304),
        component: ComponentBinding {
            scratch: component_scratch,
            arbiter: ARBITER_ADDRESS,
            categories: [rd(m, 0x177298), rd(m, 0x17EEE8), 0x8D7994, rd(m, 0x157904)],
            formats,
            error_labels: [literals[19], literals[20]],
            names,
        },
        broker_formats: std::array::from_fn(|i| tts_core::boot_part_b::BROKER_FORMATS[i]),
    }
}

fn param_default(
    m: &mut Memory<'_>,
    h: &mut impl Calls,
    params: u32,
    key: u32,
    default: u32,
    cell: u32,
) -> Result<u32, u32> {
    wr(m, cell, 0);
    let rc = fn_1756d0(m, h, params, key, cell, 0);
    if rc >= 0x80000000 {
        let work = fn_129778(m, h, 4168);
        let rc = fn_1759a0(m, h, params, key, default, work);
        if rc >= 0x80000000 {
            return Err(rc);
        }
        wr(m, cell, default);
    }
    Ok(rd(m, cell))
}
pub fn fn_1cdb80(m: &mut Memory<'_>, h: &mut impl Calls, a: u32, b: u32, out: u32) -> u32 {
    if out == 0 {
        return rd(m, 0x1CDFC0);
    }
    wr(m, out, 0);
    wr(m, out + 4, 0);
    let sp = fn_129778(m, h, 64);
    let rc = fn_19611c(m, a, b, sp + 16);
    if rc >= 0x80000000 {
        return rd(m, 0x1CDFC4);
    }
    let ctx = rd(m, sp + 16);
    let heap = rd(m, ctx + 4);
    let obj = fn_1963b8(m, h, heap, 1, 0x198);
    if obj == 0 {
        return rd(m, 0x1CDFC8);
    }
    wr(m, obj, ctx);
    let mut rc = fn_176e40(m, h, rd(m, ctx + 8), heap, obj + 0x190);
    if rc < 0x80000000 {
        let params = rd(m, ctx + 20);
        rc = if params == 0 { rd(m, 0x1CDFC0) } else { 0 };
        if rc < 0x80000000 {
            let result = (|| -> Result<(), u32> {
                let v = param_default(m, h, params, 0x1CDFCC, 0x1CDFDC, sp + 4)?;
                wb(
                    m,
                    obj + 4,
                    u8::from(
                        parse_decimal(&m.bytes[v as usize..]) != 0
                            || matches!(rb(m, v), b't' | b'T'),
                    ),
                );
                let v = param_default(m, h, params, 0x1CDFE0, 0x1CDFDC, sp + 4)?;
                let n = parse_decimal(&m.bytes[v as usize..]);
                if n < 0 {
                    return Err(rd(m, 0x1CDFC4));
                }
                wb(m, obj + 0x10C, n as u8);
                wb(m, obj + 0x10D, 1);
                wh(m, obj + 0x18E, 0);
                let v = param_default(m, h, params, 0x1CDFF8, 0x1CDFF4, sp)?;
                let mut cur = v;
                if rb(m, cur) != 0 {
                    wb(m, obj + 0x10D, 0);
                    loop {
                        while rb(m, cur) == b' ' {
                            cur += 1;
                        }
                        if rb(m, cur) == 0 {
                            break;
                        }
                        if matches!(rb(m, cur), b'a' | b'A')
                            && matches!(rb(m, cur + 1), b'l' | b'L')
                            && matches!(rb(m, cur + 2), b'l' | b'L')
                            && matches!(rb(m, cur + 3), 0 | b' ')
                        {
                            wb(m, obj + 0x10D, 1);
                            cur += 3;
                        } else {
                            let mut n = 0u16;
                            while rb(m, cur).is_ascii_digit() {
                                n = n.wrapping_mul(10).wrapping_add((rb(m, cur) - b'0') as u16);
                                cur += 1;
                            }
                            if !matches!(rb(m, cur), 0 | b' ') {
                                return Err(rd(m, 0x1CDFC4));
                            }
                            let count = rh(m, obj + 0x18E);
                            if count < 64 {
                                wh(m, obj + 0x10E + 2 * count as u32, n);
                                wh(m, obj + 0x18E, count + 1);
                            }
                        }
                    }
                    if rh(m, obj + 0x18E) >= 64 {
                        return Err(rd(m, 0x1CDFC4));
                    }
                }
                let v = param_default(m, h, params, 0x1CE010, 0x1CDFF4, sp)?;
                fn_195cd0(m, obj + 6, v);
                let v = param_default(m, h, params, 0x1CE024, 0x1CDFDC, sp + 12)?;
                wr(m, obj + 0x108, parse_decimal(&m.bytes[v as usize..]) as u32);
                let v = param_default(m, h, params, 0x1CE038, 0x1CDFDC, sp + 4)?;
                wb(
                    m,
                    obj + 5,
                    u8::from(
                        parse_decimal(&m.bytes[v as usize..]) != 0
                            || matches!(rb(m, v), b't' | b'T'),
                    ),
                );
                let r = route_or_host(
                    m,
                    h,
                    0x2324D0,
                    &[
                        ctx,
                        rb(m, obj + 5) as u32,
                        obj + 6,
                        rd(m, obj + 0x108),
                        obj + 0x194,
                    ],
                );
                if r >= 0x80000000 {
                    return Err(r);
                }
                if rb(m, obj + 4) != 0 {
                    route_or_host(m, h, 0x232610, &[rd(m, obj + 0x194)]);
                }
                Ok(())
            })();
            rc = result.err().unwrap_or(0);
        }
    }
    wr(m, out, obj);
    wr(m, out + 4, rd(m, 0x1CDFB8));
    if rc >= 0x80000000 {
        if rd(m, obj + 0x194) != 0 {
            route_or_host(m, h, 0x2325E4, &[rd(m, obj + 0x194)]);
        }
        if rd(m, obj + 0x190) != 0 {
            fn_176edc(m, h, rd(m, obj + 0x190));
        }
        fn_196428(m, h, heap, obj);
        wr(m, out, 0);
        wr(m, out + 4, 0);
    }
    rc
}

fn parse_decimal(text: &[u8]) -> i32 {
    let mut i = 0;
    while matches!(text.get(i), Some(9..=13) | Some(32)) {
        i += 1;
    }
    let negative = text.get(i) == Some(&b'-');
    if matches!(text.get(i), Some(b'-' | b'+')) {
        i += 1;
    }
    let mut n = 0i32;
    while let Some(c @ b'0'..=b'9') = text.get(i) {
        n = n.wrapping_mul(10).wrapping_add((c - b'0') as i32);
        i += 1;
    }
    if negative {
        n.wrapping_neg()
    } else {
        n
    }
}

pub fn fn_2324d0(
    m: &mut Memory<'_>,
    h: &mut impl Calls,
    ctx: u32,
    flag: u32,
    prefix: u32,
    count: u32,
    out: u32,
) -> u32 {
    if out == 0 {
        return rd(m, 0x2325B8);
    }
    wr(m, out, 0);
    let obj = fn_1963b8(m, h, rd(m, ctx + 4), 1, 0x218);
    if obj == 0 {
        return rd(m, 0x2325BC);
    }
    wr(m, obj, ctx);
    wb(m, obj + 4, 0);
    fn_195cd0(m, obj + 5, prefix);
    fn_195cd0(m, obj + 0x105, 0x2325C0);
    wr(m, obj + 0x208, if flag != 0 { 0 } else { count });
    wr(m, obj + 0x20C, 0);
    wr(m, obj + 0x210, 0);
    wr(m, obj + 0x214, 0);
    let sp = fn_129778(m, h, 12);
    fn_195ae8(m, sp, 0x2325C4, 12);
    let sep = if rb(m, obj + 5) == 0 {
        0x2325C0
    } else {
        0x2325D0
    };
    m.boot_sprintf(
        obj + 0x105,
        0x2325D4,
        &[obj + 5, sep, sp, rd(m, obj + 0x210)],
    );
    wr(m, obj + 0x210, rd(m, obj + 0x210).wrapping_add(1));
    wr(m, out, obj);
    0
}

pub fn fn_1900f8(
    m: &mut Memory<'_>,
    h: &mut impl Calls,
    object: u32,
    mode: u32,
    voice: u32,
) -> u32 {
    let sp = fn_129778(m, h, 0x160);
    wr(m, sp + 0x130, 0);
    wr(m, sp + 4, 0);
    let rc = fn_19611c(m, rd(m, object + 8), rd(m, object + 12), sp + 0x130);
    if rc >= 0x80000000 {
        return rc;
    }
    let ctx = rd(m, sp + 0x130);
    let converted = sp + 8;
    if mode == 0 {
        fn_3023b0(m, converted, 0x128);
        fn_195cd0(m, converted, voice);
        fn_195cd0(m, converted + 0x88, voice + 0x40);
        let freq = rh(m, voice + 0x60) as u32;
        wr(
            m,
            converted + 0xb8,
            match freq {
                11 => rd(m, 0x1903B8),
                22 => rd(m, 0x1903BC),
                _ => freq * 1000,
            },
        );
        fn_195cd0(m, converted + 0xbc, voice + 0x62);
        fn_195cd0(m, converted + 0xf4, voice + 0x82);
        fn_195cd0(m, converted + 0x104, 0x1903C8);
        let methods = rd(m, object + 4);
        let rc = route_or_host(
            m,
            h,
            rd(m, rd(m, methods + 0xc8) + 0x30),
            &[
                rd(m, methods + 0xcc),
                rd(m, methods + 0xd0),
                rd(m, ctx + 12),
                rd(m, ctx + 20),
                converted,
            ],
        );
        if rc >= 0x80000000 {
            return rc;
        }
        fn_195cd0(m, voice, converted);
        fn_195cd0(m, voice + 0x40, converted + 0x88);
        wh(m, voice + 0x60, (rd(m, converted + 0xb8) / 1000) as u16);
        fn_195cd0(m, voice + 0x62, converted + 0xbc);
        fn_195cd0(m, voice + 0x82, converted + 0xf4);
    }
    route_or_host(
        m,
        h,
        0x176C58,
        &[rh(m, voice + 0x60) as u32, sp + 0x138, 10],
    );
    let spec = voice + 0x92;
    fn_195cd0(m, spec, 0x1903D0);
    for src in [voice, 0x1903D8, voice + 0x40, 0x1903D8, sp + 0x138] {
        route_or_host(m, h, 0x195C90, &[spec, src]);
    }
    let full = (0..4).all(|i| rb(m, voice + 0x62 + i).to_ascii_lowercase() == b"full"[i as usize])
        && rb(m, voice + 0x66) == 0;
    if rb(m, voice + 0x62) != 0 && !full {
        route_or_host(m, h, 0x195C90, &[spec, 0x1903D8]);
        route_or_host(m, h, 0x195C90, &[spec, voice + 0x62]);
    }
    match rb(m, voice + 0x112) {
        0 | 2 => {
            for src in [0x1903D8, voice + 0x82, 0x1903E0] {
                route_or_host(m, h, 0x195C90, &[spec, src]);
            }
        }
        1 => {
            route_or_host(m, h, 0x195C90, &[spec, 0x1903E8]);
        }
        _ => return rd(m, 0x1903DC),
    }
    let qrc = fn_1785c8(m, h, rd(m, ctx + 12), spec, 0x1903FC, 1, sp + 4);
    if qrc >= 0x80000000 {
        return rd(m, 0x19040C);
    }
    fn_178818(m, h, rd(m, ctx + 12), rd(m, sp + 4));
    rc
}
fn fn_178818(m: &mut Memory<'_>, h: &mut impl Calls, broker: u32, tag: u32) -> u32 {
    if broker == 0 || tag == 0 {
        return rd(m, 0x178588);
    }
    let sp = fn_129778(m, h, 8);
    wr(m, sp, tag);
    route_or_host(
        m,
        h,
        0x1738E8,
        &[rd(m, broker), rd(m, 0x178494), 4, 0, 0x178878, sp],
    );
    route_or_host(
        m,
        h,
        0x1738E8,
        &[rd(m, broker), rd(m, 0x178494), 4, 0, 0x1788A8, sp],
    );
    0
}
fn fn_1785c8(
    m: &mut Memory<'_>,
    h: &mut impl Calls,
    broker: u32,
    name: u32,
    key: u32,
    flags: u32,
    out: u32,
) -> u32 {
    if broker == 0 || rd(m, broker + 4) == 0 || name == 0 || key == 0 || out == 0 {
        return rd(m, 0x178588);
    }
    wr(m, out, 0);
    let sp = fn_129778(m, h, 64);
    wr(m, sp, name);
    wr(m, sp + 4, key);
    route_or_host(
        m,
        h,
        0x1738E8,
        &[rd(m, broker), rd(m, 0x178494), 4, 0, 0x178738, sp],
    );
    let mut rc = fn_1772b0(m, h, broker, name, sp + 24);
    if rc >= 0x80000000 {
        rc = rd(m, 0x1779FC);
    } else {
        rc = fn_1966ac(m, h, rd(m, rd(m, sp + 24)), key, sp + 28);
        if rc >= 0x80000000 {
            rc = rd(m, 0x17876C);
        } else {
            let value = rd(m, sp + 28);
            if value != 0 {
                wr(m, out, rd(m, value));
            }
            rc = 0;
        }
    }
    if rc >= 0x80000000 {
        if flags & 1 != 0 {
            wr(m, sp + 8, rc);
            route_or_host(
                m,
                h,
                0x1738E8,
                &[rd(m, broker), rd(m, 0x178494), 3, 0, 0x178770, sp],
            );
        } else {
            panic!("MISSING_1785C8_FATAL_LOG rc={rc:#x}");
        }
    }
    wr(m, sp, rd(m, out));
    wr(
        m,
        sp + 4,
        if rd(m, out) == 0 {
            0x1787B4
        } else {
            rd(m, out)
        },
    );
    route_or_host(
        m,
        h,
        0x1738E8,
        &[rd(m, broker), rd(m, 0x178494), 4, 0, 0x1787DC, sp],
    );
    rc
}

fn fn_15eef0(m: &mut Memory<'_>, h: &mut impl Calls, a: [u32; 6]) -> u32 {
    if fn_176f94(m, a[0], a[1], 0x15c, 32) >= 0x80000000 {
        return rd(m, 0x15F00C);
    }
    if a[4] == 0 {
        return rd(m, 0x15F010);
    }
    let sp = fn_129778(m, h, 0x1600);
    let hdr = sp + 0x130;
    let rec = sp + 8;
    let inner = rd(m, a[0] + 8);
    for i in 0..32 {
        wb(m, hdr + i, rb(m, inner + i));
    }
    if a[2] != 0 {
        wr(m, hdr + 12, a[2]);
    }
    if a[3] != 0 {
        wr(m, hdr + 20, a[3]);
    }
    for i in 0..0x128 {
        wb(m, rec + i, rb(m, a[4] + i));
    }
    route_or_host(
        m,
        h,
        0x1738E8,
        &[rd(m, hdr + 16), rd(m, 0x15F034), 4, 0, 0x15F014, sp],
    );
    let keys = [
        0x15DF60, 0x15DF6C, 0x15DF78, 0x15DF84, 0x15DF90, 0x15DF98, 0x15DFA0, 0x15DFA4, 0x15DFB0,
    ];
    let table = std::array::from_fn(|i| rh(m, rd(m, 0x17C8F8) + i as u32 * 2));
    let pipeline = 0x15E410;
    let priority = 0x15EDDC;
    let query = 0x15E430;
    let us = 0x15E400;
    let mx = 0x15E408;
    let seg = 0;
    let mut rh = RouteHost { inner: h };
    let rc = boot_live::match_voice_m(
        m.bytes,
        hdr,
        rec,
        a[5],
        &keys,
        pipeline,
        priority,
        query,
        us,
        mx,
        seg,
        sp + 0x200,
        &table,
        sp + 0x400,
        &mut rh,
    );

    if rc < 0x80000000 {
        if rb(m, rec + 0xf4) == 0 {
            fn_195cd0(m, rec + 0xf4, 0x15F038);
        }
        if rb(m, rec + 0x104) == 0 {
            fn_195cd0(m, rec + 0x104, 0x15F040);
        }
        for i in 0..0x128 {
            wb(m, a[4] + i, rb(m, rec + i));
        }
    }
    wr(m, sp, rc);
    route_or_host(
        m,
        h,
        0x1738E8,
        &[rd(m, hdr + 16), rd(m, 0x15F034), 4, 0, 0x15F044, sp],
    );
    rc
}
fn fn_160604(m: &mut Memory<'_>, h: &mut impl Calls, a: [u32; 5]) -> u32 {
    let mut rc = fn_15eef0(m, h, [a[0], a[1], a[2], a[3], a[4], 1]);
    if rc < 0x80000000 {
        return rc;
    }
    if fn_176f94(m, a[0], a[1], 0x15c, 32) >= 0x80000000 {
        return rd(m, 0x160544);
    }
    let obj = a[0];
    if rd(m, obj + 20) == 0 {
        return rc;
    }
    let lock = rd(m, obj + 28);
    let acquire = route_or_host(m, h, 0x176F2C, &[lock]);
    if acquire >= 0x80000000 {
        return acquire;
    }
    let sp = fn_129778(m, h, 16);
    let stamp = route_or_host(m, h, 0x173558, &[rd(m, rd(m, obj + 8) + 28), sp]);
    if (rd(m, obj + 20) as i32) < stamp.wrapping_sub(rd(m, obj + 24)) as i32 {
        wr(m, sp, stamp.wrapping_sub(rd(m, obj + 24)));
        route_or_host(
            m,
            h,
            0x1738E8,
            &[
                rd(m, rd(m, obj + 8) + 16),
                rd(m, 0x160194),
                4,
                0,
                0x16073C,
                sp,
            ],
        );
        if route_or_host(m, h, 0x177A98, &[rd(m, obj + 12)]) < 0x80000000 {
            rc = fn_15eef0(m, h, [a[0], a[1], a[2], a[3], a[4], 1]);
            wr(m, obj + 24, stamp);
        }
    }
    let release = route_or_host(m, h, 0x176F5C, &[lock]);
    if release >= 0x80000000 && rc < 0x80000000 {
        rc = release;
    }
    rc
}

struct ColdPipeHost<'a, H> {
    inner: &'a mut H,
}
impl<H: Calls> ColdPipeHost<'_, H> {
    fn invoke(&mut self, bytes: &mut [u8], f: u32, a: &[u32]) -> u32 {
        route_or_host(&mut Memory { base: 0, bytes }, self.inner, f, a)
    }
    fn out(&mut self, bytes: &mut [u8], f: u32, a: &[u32]) -> (u32, u32) {
        let sp = self.invoke(bytes, 0x129778, &[16]);
        let mut args = a.to_vec();
        args.push(sp);
        let rc = self.invoke(bytes, f, &args);
        (
            rc,
            u32::from_le_bytes(bytes[sp as usize..sp as usize + 4].try_into().unwrap()),
        )
    }
}
impl<H: Calls> boot_live::PipeHooksM for ColdPipeHost<'_, H> {
    fn check_open_m(&mut self, obj: u32, b: u32, bytes: &mut [u8]) -> u32 {
        let m = Memory { base: 0, bytes };
        fn_176f94(&m, obj, b, 0xcdac, 32)
    }
    fn resolve_ctx_m(&mut self, a: u32, b: u32, bytes: &mut [u8]) -> (u32, u32) {
        let sp = self.invoke(bytes, 0x129778, &[16]);
        let mut m = Memory { base: 0, bytes };
        let rc = fn_19611c(&mut m, a, b, sp);
        (rc, rd(&m, sp))
    }
    fn set_textanalysis_m(&mut self, params: u32, on: bool, bytes: &mut [u8]) {
        self.invoke(
            bytes,
            0x1759A0,
            &[params, 0x159054, if on { 0x1590AC } else { 0x159050 }],
        );
    }
    fn lock_m(&mut self, p: u32, bytes: &mut [u8]) -> u32 {
        self.invoke(bytes, 0x176F2C, &[p])
    }
    fn unlock_m(&mut self, p: u32, bytes: &mut [u8]) -> u32 {
        self.invoke(bytes, 0x176F5C, &[p])
    }
    fn query_pending_m(&mut self, p: u32, bytes: &mut [u8]) -> (u32, u32) {
        self.out(bytes, 0x175C40, &[p])
    }
    fn trace_pipe_m(
        &mut self,
        a: u32,
        b: u32,
        name: u32,
        cs: u32,
        stream: u32,
        bytes: &mut [u8],
    ) -> (u32, u32, u8) {
        let sp = self.invoke(bytes, 0x129778, &[16]);
        let typ = u32::from_le_bytes(bytes[0x1588FC..0x158900].try_into().unwrap());
        let rc = self.invoke(bytes, 0x17F614, &[a, b, name, cs, typ, stream, sp, sp + 4]);
        (
            rc,
            u32::from_le_bytes(bytes[sp as usize..sp as usize + 4].try_into().unwrap()),
            bytes[sp as usize + 4],
        )
    }
    fn query_param_m(&mut self, p: u32, key: &[u8], bytes: &mut [u8]) -> (u32, u32) {
        let m = Memory { base: 0, bytes };
        let k = rom_text(&m, key);
        self.out(m.bytes, 0x175A1C, &[p, k])
    }
    fn start_object_m(&mut self, c: u32, o: u32, bytes: &mut [u8]) -> u32 {
        self.invoke(bytes, 0x1529B4, &[c, o])
    }
    fn lic_count_m(&mut self, f: u32, bytes: &mut [u8]) -> u32 {
        self.invoke(bytes, 0x17EB74, &[f])
    }
    fn lic_feat_m(&mut self, f: u32, bytes: &mut [u8]) -> u32 {
        self.invoke(bytes, 0x17EB3C, &[f])
    }
    fn release_token_m(&mut self, t: u32, bytes: &mut [u8]) {
        self.invoke(bytes, 0x17EB58, &[t]);
    }
    fn notify_lic_m(&mut self, c: u32, ch: u32, k: u32, mode: u32, bytes: &mut [u8]) {
        self.invoke(bytes, 0x151F80, &[c, ch, k, mode]);
    }
    fn release_pipe_m(&mut self, p: u32, bytes: &mut [u8]) -> u32 {
        self.invoke(bytes, 0x17EC38, &[p])
    }
    fn rollback_pending_m(&mut self, p: u32, t: u32, bytes: &mut [u8]) -> u32 {
        self.invoke(bytes, 0x175CE0, &[p, t])
    }
    fn release_pending_m(&mut self, p: u32, t: u32, bytes: &mut [u8]) -> u32 {
        self.invoke(bytes, 0x175EF0, &[p, t])
    }
    fn reactivate_m(&mut self, p: u32, bytes: &mut [u8]) {
        self.invoke(bytes, 0x1805F0, &[p]);
    }
    fn license_kind_m(&mut self, c: u32, o: u32, t: u32, bytes: &mut [u8]) -> u32 {
        self.invoke(bytes, 0x15B128, &[c, o, t])
    }
}

fn fn_175c40(m: &mut Memory<'_>, h: &mut impl Calls, p: u32, out: u32) -> u32 {
    if p == 0 || out == 0 {
        return rd(m, 0x17551C);
    }
    let lock = rd(m, p + 4);
    let mut rc = route_or_host(m, h, 0x176F2C, &[lock]);
    if rc >= 0x80000000 {
        return rc;
    }
    let sp = fn_129778(m, h, 8);
    wr(m, sp, p);
    wr(m, sp + 4, 0);
    let arr = rd(m, p + 16);
    if fn_196db0(m, h, arr, sp) == 0 {
        rc = rd(m, 0x175510);
    } else {
        let count = if arr == 0 { 0 } else { rd(m, arr + 20) };
        if fn_1970ac(m, arr, count.wrapping_sub(1), out) == 0 {
            rc = rd(m, 0x175528);
        }
    }
    let release = route_or_host(m, h, 0x176F5C, &[lock]);
    if release >= 0x80000000 && rc < 0x80000000 {
        rc = release;
    }
    rc
}

impl<H: Calls> tts_core::boot_part_a::AudioFetchHooksM for RouteHost<'_, H> {
    fn call(&mut self, m: &mut Memory<'_>, f: u32, a: &[u32]) -> u32 {
        route_or_host(m, self.inner, f, a)
    }
    fn parser_m(&mut self, m: &mut Memory<'_>, f: u32, a: &[u32]) -> u32 {
        route_or_host(m, self.inner, f, a)
    }
}

impl<H: Calls> tts_core::boot_port_3::Host for RouteHost<'_, H> {
    fn call(&mut self, m: &mut Memory<'_>, f: u32, a: &[u32]) -> u32 {
        route_or_host(m, self.inner, f, a)
    }
    fn thread_id(&mut self) -> u32 {
        0x1FF82000
    }
}

fn dctlkp_config(m: &mut Memory<'_>, h: &mut impl Calls) -> tts_core::boot_part_a::DctlkpConfig {
    use tts_core::boot_part_a::{DctlkpConfig, DctlkpStrings};
    let scratch = fn_129778(m, h, 4096);
    let p = pair_176fd4(m);
    DctlkpConfig {
        pair: ((p[1] as u64) << 32 | p[0] as u64).to_le_bytes(),
        default: rd(m, 0x196304),
        category: 0x8F8F44,
        scratch,
        strings: DctlkpStrings {
            lingdb: rom_text(m, b"LINGDB"),
            sysdct: rom_text(m, b"sysdct"),
            lid: rom_text(m, b"lid"),
            punc: rom_text(m, b"punc"),
            fecfg: rom_text(m, b"fecfg"),
            langcode: rom_text(m, b"langcode"),
            fecfg_dir: rom_text(m, b"fecfg"),
            fedataprefix: rom_text(m, b"fedataprefix"),
            fedatapackaging: rom_text(m, b"fedatapackaging"),
            clc: rom_text(m, b"clc"),
            clc_slash: rom_text(m, b"clc/"),
            slash: rom_text(m, b"/"),
            colon: rom_text(m, b":"),
            sysdct_slash: rom_text(m, b"sysdct/"),
        },
        broker: tts_core::boot_port_2c::BrokerGlobals {
            module: 0x8D7668,
            brokerstring_key: 0x177264,
            add_item_format: 0x177278,
            broker_label: 0x17729C,
            fullpath_key: 0x17823C,
            ctype_table: rd(m, 0x17C8F8),
            parameters_desc: 0x8D7614,
            linguistic_desc: 0x8D773C,
        },
        allocators: [0x196394, 0x1963B8, 0x1963E0, 0x196428],
        unmap: tts_core::boot_part_e::UnmapEnvironment {
            category: 0x8D7668,
            begin_format: 0x179310,
            end_format: 0x17935C,
            error_format: 0x17934C,
            labels: [0x179344, 0x17933C, 0x1791D8],
            scratch: scratch + 2048,
        },
    }
}
fn fn_1ec070(m: &mut Memory<'_>, h: &mut impl Calls, a: &[u32]) -> u32 {
    use tts_core::boot_part_a::Dctlkp;
    let cfg = dctlkp_config(m, h);
    let mut rh = RouteHost { inner: h };
    let d = Dctlkp::new(
        &mut rh,
        &cfg,
        tts_core::component_live_c::DictionaryWorkspace::default(),
    );
    d.open(m, a[0], a[1], a[2], a[3], a[4])
}

fn fn_4a77ac(m: &mut Memory<'_>, h: &mut impl Calls, a: &[u32]) -> u32 {
    let [archive, allocator, path, mode, out] = a[..5].try_into().unwrap();
    let file = route_or_host(m, h, rd(m, rd(m, allocator) + 8), &[allocator, 16, 4]);
    assert_ne!(file, 0, "4a77ac file allocation failed");
    wr(m, out, file);
    wr(m, file + 12, allocator);
    if path == 0 || mode == 0 || archive == 0 {
        return 0x80000007;
    }
    if rb(m, mode) != b'r' {
        return 0x80000000;
    }
    let n = fn_195d90(m, path);
    if (0..n).any(|i| rb(m, path + i) == b'.') {
        return 0x80000000;
    }
    assert!(n < 256, "4a77ac source path exceeds ARM frame");
    let sp = fn_129778(m, h, 4096);
    for i in 0..n {
        let b = rb(m, path + i);
        wb(m, sp + i, if b == b'/' { b'_' } else { b });
    }
    let count = if n + 4 >= 256 { 255 - n } else { 4 };
    for i in 0..count {
        wb(m, sp + n + i, rb(m, 0x4A78E8 + i));
    }
    wb(m, sp + n + count, 0);
    wb(m, sp + 255, 0);
    let env = tts_core::speech_part_1::ArchiveEnvironment {
        dot: BOOT_DOT,
        parent: BOOT_PARENT,
        locale: 0,
    };
    let data = tts_core::boot_part_a::fn_16b554_m(m, archive + 8, 0, sp, file + 8, sp + 512, &env);
    wr(m, file, data);
    if data != 0 {
        wr(m, file + 4, data);
        return 0;
    }
    route_or_host(m, h, rd(m, rd(m, allocator) + 12), &[allocator, file]);
    0x80000014
}

struct ColdArchive<'a, H> {
    inner: &'a mut H,
    scratch: u32,
    plain: u32,
    encoded: u32,
}
impl<H: Calls> tts_core::boot_part_e::Host for ColdArchive<'_, H> {
    fn call(&mut self, m: &mut Memory<'_>, f: u32, a: &[u32]) -> u32 {
        route_or_host(m, self.inner, f, a)
    }
}
impl<H: Calls> tts_core::boot_part_e::ArchiveHost for ColdArchive<'_, H> {
    fn service(&mut self, m: &mut Memory<'_>, f: u32, a: &[u32], out: &mut [u32; 4]) -> u32 {
        assert_eq!(f, 0x1b2e04, "MISSING_ARCHIVE_SERVICE");
        let sp = self.scratch + 256;
        wr(m, sp, out[0]);
        wb(m, sp + 4, out[1] as u8);
        wb(m, sp + 5, out[2] as u8);
        let mut workspace = vec![0; a[2].max(4) as usize];
        let rc = boot_live::stream_read_m(
            m,
            self,
            a[0],
            a[1],
            a[2],
            sp,
            sp + 4,
            sp + 5,
            &mut workspace,
        );
        out[0] = rd(m, sp);
        out[1] = rb(m, sp + 4) as u32;
        out[2] = rb(m, sp + 5) as u32;
        rc
    }
    fn read(&mut self, m: &mut Memory<'_>, f: u32, ctx: u32, out: &mut [u8]) -> u32 {
        let dst = fn_129778(m, self.inner, (out.len() as u32).max(4));
        m.bytes[dst as usize..dst as usize + out.len()].copy_from_slice(out);
        let rc = route_or_host(m, self.inner, f, &[dst, 1, out.len() as u32, ctx]);
        out.copy_from_slice(&m.bytes[dst as usize..dst as usize + out.len()]);
        rc
    }
    fn bind_engine_services(&self) -> bool {
        true
    }
    fn streams(&self) -> (tts_core::boot_port_1::Streams, u32) {
        (
            tts_core::boot_port_1::Streams {
                parameters: tts_core::boot_part_f::Environment::US,
                plain: self.plain,
                encoded: self.encoded,
                seek: 0,
                seek_adjustment: 12,
            },
            self.scratch + 512,
        )
    }
}
fn cold_archive_attach(
    m: &mut Memory<'_>,
    h: &mut impl Calls,
    q: u32,
    params: &mut [u32; 4],
    p: u32,
    out: &mut u32,
    flags: u32,
) -> u32 {
    let sp = fn_129778(m, h, 4096);
    for (i, v) in params.iter().enumerate() {
        wr(m, sp + i as u32 * 4, *v);
    }
    wr(m, sp + 16, *out);
    let e = tts_core::boot_part_e::ArchiveEnvironment {
        parameters: tts_core::boot_part_f::Environment::US,
        stream_table: 0x8FAAE0,
        wrapper_table: 0x917E3C,
        validator: 0x1A8684,
        substream_table: 0x917E04,
        nvar: 0x17D078,
        chinese: 0x17D080,
        scratch: sp + 64,
    };
    let mut ah = ColdArchive {
        inner: h,
        scratch: sp,
        plain: rd(m, 0x1B2534),
        encoded: rd(m, 0x1B2D3C),
    };
    let rc = tts_core::boot_part_e::fn_17cbb4(m, q, sp, p, sp + 16, flags, &e, &mut ah);
    for (i, v) in params.iter_mut().enumerate() {
        *v = rd(m, sp + i as u32 * 4);
    }
    *out = rd(m, sp + 16);
    rc
}

impl<H: Calls> tts_core::speech_part_1::StreamHost for HeapCalls<'_, H> {
    fn read(&mut self, m: &mut Memory<'_>, f: u32, dst: Option<u32>, n: u32, stream: u32) -> u32 {
        let dst = dst.unwrap_or_else(|| fn_129778(m, self.inner, n.max(4)));
        route_or_host(m, self.inner, f, &[dst, 1, n, stream])
    }
}

impl<H: Calls> tts_core::boot_part_e::Host for RouteHost<'_, H> {
    fn call(&mut self, m: &mut Memory<'_>, f: u32, a: &[u32]) -> u32 {
        route_or_host(m, self.inner, f, a)
    }
}

fn depes_path(
    m: &mut Memory<'_>,
    h: &mut impl Calls,
    ctx: u32,
    out: u32,
    mode: u32,
    voice_specific: bool,
) -> u32 {
    let params = rd(m, ctx + 20);
    let sp = fn_129778(m, h, 32);
    let mut query = |m: &mut Memory<'_>, key: &[u8], cell: u32| {
        let key = rom_text(m, key);
        let rc = fn_1756d0(m, h, params, key, cell, 0);
        (rc, rd(m, cell))
    };
    let (rc, language) = query(m, b"langcode", sp);
    if rc >= 0x80000000 {
        return rc;
    }
    let voice = if voice_specific {
        let (r, v) = query(m, b"voice", sp + 4);
        if r >= 0x80000000 {
            return r;
        }
        v
    } else {
        0
    };
    let (rc, config) = query(m, b"fecfg", sp + 8);
    if rc >= 0x80000000 {
        return rc;
    }
    wb(m, out, 0);
    let (prc, prefix) = query(m, b"fedataprefix", sp + 12);
    if prc < 0x80000000 && prefix != 0 && rb(m, prefix) != 0 {
        tts_core::fe_text::strcat_195c90(m.bytes, out, prefix);
    }
    let (pkg_rc, pkg) = query(m, b"fedatapackaging", sp + 16);
    let append = |m: &mut Memory<'_>, src: u32| {
        assert_ne!(src, 0, "depes path parameter missing");
        tts_core::fe_text::strcat_195c90(m.bytes, out, src);
    };
    let slash = rom_text(m, b"/");
    if pkg_rc < 0x80000000 && pkg != 0 && fn_195d58(m, pkg, rom_text(m, b"clc")) == 0 {
        append(m, rom_text(m, b"clc/"));
        append(m, language);
        if voice_specific {
            let n = fn_195d90(m, voice);
            let uni = (0..n.saturating_sub(2))
                .any(|i| (0..3).all(|j| rb(m, voice + i + j) == b"uni"[j as usize]));
            if !uni {
                append(m, slash);
                append(m, voice);
            }
        }
        append(m, slash);
        append(m, config);
        append(m, rom_text(m, b":"));
    }
    append(m, rom_text(m, b"depes/"));
    append(m, language);
    if voice_specific {
        append(m, slash);
        append(m, voice);
    }
    if mode == 0 {
        append(m, slash);
        append(m, config);
    }
    let n = fn_195d90(m, out) as u16;
    for i in 0..n as u32 {
        wb(
            m,
            out + i,
            tts_core::normalization::tolower_240ba0(rb(m, out + i) as u32) as u8,
        );
    }
    rc
}
fn fn_21834c(m: &mut Memory<'_>, h: &mut impl Calls, a: u32, b: u32, out: u32) -> u32 {
    if out == 0 {
        return rd(m, 0x217E00);
    }
    let sp = fn_129778(m, h, 0x140);
    wr(m, out, 0);
    wr(m, out + 4, 0);
    wh(m, sp + 4, 0);
    fn_195cd0(m, sp + 8, 0x218164);
    let mut rc = fn_19611c(m, a, b, sp + 0x108);
    if rc >= 0x80000000 {
        return rc;
    }
    let ctx = rd(m, sp + 0x108);
    let heap = rd(m, ctx + 4);
    let reg = rd(m, ctx + 24);
    rc = route_or_host(m, h, 0x17B058, &[reg, 0x218714, sp + 0x10c]);
    if rc >= 0x80000000 {
        return rc;
    }
    let obj = fn_1963b8(m, h, heap, 1, 0x40);
    if obj == 0 {
        route_or_host(m, h, 0x17B1C4, &[reg, 0x218714]);
        return rd(m, 0x217DDC);
    }
    wr(m, out, obj);
    wr(m, out + 4, rd(m, 0x218720));
    wr(m, obj, ctx);
    let info = rd(m, sp + 0x10c);
    for (dst, src) in [(4, 8), (8, 12), (12, 4)] {
        wr(m, obj + dst, rd(m, info + src));
    }
    wh(m, obj + 0x32, 0x400);
    for voice in [false, true] {
        let (handle_off, size_off) = if voice { (20, 28) } else { (16, 24) };
        for mode in 0..=1 {
            rc = depes_path(m, h, ctx, sp + 8, mode, voice);
            if rc < 0x80000000 {
                rc = route_or_host(
                    m,
                    h,
                    0x217E24,
                    &[ctx, sp + 8, obj + handle_off, obj + size_off],
                );
            }
            if rc >= 0x80000000 && mode == 1 {
                break;
            }
            if rc < 0x80000000 && rd(m, obj + handle_off) != 0 {
                break;
            }
        }
        if rc >= 0x80000000 {
            route_or_host(m, h, 0x216E8C, &[obj, rd(m, out + 4)]);
            wr(m, out, 0);
            wr(m, out + 4, 0);
            return rc;
        }
    }
    if rd(m, obj + 16) == 0 && rd(m, obj + 20) == 0 {
        rc = rd(m, 0x218B10);
    } else {
        for (handle_off, grammar_off) in [(16, 32), (20, 36)] {
            if rd(m, obj + handle_off) == 0 {
                continue;
            }
            let g = fn_1963b8(m, h, heap, 1, 0x3bc);
            wr(m, obj + grammar_off, g);
            if g == 0 {
                rc = rd(m, 0x217DDC);
                break;
            }
            wr(m, g + 0x3a8, heap);
            for (dst, src) in [(0x3ac, 8), (0x3b0, 12), (0x3b4, 4)] {
                wr(m, g + dst, rd(m, info + src));
            }
        }
        if rc < 0x80000000 {
            rc = route_or_host(m, h, 0x217AC8, &[obj, rd(m, out + 4), 0x218724, sp + 4]);
        }
    }
    if rc >= 0x80000000 {
        route_or_host(m, h, 0x216E8C, &[obj, rd(m, out + 4)]);
        wr(m, out, 0);
        wr(m, out + 4, 0);
    }
    rc
}

fn fn_217e24(
    m: &mut Memory<'_>,
    h: &mut impl Calls,
    ctx: u32,
    name: u32,
    out: u32,
    size: u32,
) -> u32 {
    let broker = rd(m, ctx + 12);
    let rc = route_or_host(m, h, 0x1795F0, &[broker, name, 1, out]);
    if rc >= 0x80000000 {
        wr(m, out, 0);
        return 0;
    }
    let sp = fn_129778(m, h, 8);
    let rc = route_or_host(m, h, 0x17A1A4, &[broker, rd(m, out), 0, 0x48, sp + 4]);
    if rc >= 0x80000000 {
        route_or_host(m, h, 0x179F3C, &[broker, rd(m, out)]);
        wr(m, out, 0);
        return rc;
    }
    let base = rd(m, sp + 4);
    if base & 3 != 0
        || rd(m, base) != 0xFEFF
        || route_or_host(m, h, 0x176A48, &[base + 4, 0x217F4C, 8]) != 0
        || route_or_host(m, h, 0x176A48, &[base + 12, 0x217F58, 5]) != 0
    {
        return rd(m, 0x217F48);
    }
    let total = rd(m, base + 20);
    route_or_host(m, h, 0x179208, &[broker, rd(m, out), base]);
    wr(m, size, total.wrapping_sub(0x48));
    rc
}

fn depes_read(
    m: &mut Memory<'_>,
    base: u32,
    size: u32,
    offset: &mut u32,
    count: u32,
    out: u32,
    copy: bool,
    align: u32,
) -> u32 {
    *offset = offset.wrapping_add(align - 1) & !(align - 1);
    let end = offset.wrapping_add(count);
    if end > size {
        return rd(m, 0x217088);
    }
    if copy {
        for i in 0..count {
            wb(m, out + i, rb(m, base + *offset + i));
        }
    } else {
        wr(m, out, base + *offset);
    }
    *offset = end;
    0
}
fn fn_2171bc(
    m: &mut Memory<'_>,
    h: &mut impl Calls,
    ctx: u32,
    name: u32,
    handle: u32,
    size: u32,
    g: u32,
) -> u32 {
    let sp = fn_129778(m, h, 32);
    let broker = rd(m, ctx + 12);
    let heap = rd(m, ctx + 4);
    let rc = route_or_host(m, h, 0x17A1A4, &[broker, handle, 0x48, size, sp]);
    if rc >= 0x80000000 {
        return rc;
    }
    let base = rd(m, sp);
    let mut off = 0u32;
    while fn_1769d0(m, base + off, name) != 0 && size > off {
        off = (off + fn_195d90(m, base + off) + 4) & !3;
        off += 8;
    }
    if off >= size {
        route_or_host(m, h, 0x179208, &[broker, handle, base]);
        return rd(m, 0x2179C0);
    }
    off = (off + fn_195d90(m, base + off) + 4) & !3;
    let section = rd(m, base + off);
    let size = rd(m, base + off + 4);
    route_or_host(m, h, 0x179208, &[broker, handle, base]);
    let rc = route_or_host(m, h, 0x17A1A4, &[broker, handle, section, size, sp]);
    if rc >= 0x80000000 {
        return rc;
    }
    let base = rd(m, sp);
    wr(m, g, base);
    off = 0;
    macro_rules! read {
        ($n:expr,$out:expr,$copy:expr,$align:expr) => {{
            let n = $n;
            let out = $out;
            let r = depes_read(m, base, size, &mut off, n, out, $copy, $align);
            if r >= 0x80000000 {
                return r;
            }
        }};
    }
    read!(4, g + 4, true, 4);
    read!(rd(m, g + 4), g + 8, false, 1);
    read!(4, sp + 4, true, 4);
    wh(m, g + 12, rd(m, sp + 4) as u16);
    for field in [0x10, 0x14, 0x18, 0x1c] {
        read!(4, g + field, true, 4);
    }
    read!(4, sp + 4, true, 4);
    wh(m, g + 0x22, rd(m, sp + 4) as u16);
    read!(4, sp + 4, true, 4);
    wh(m, g + 0x20, rd(m, sp + 4) as u16);
    read!(4, g + 0x24, true, 4);
    read!(rd(m, g + 0x10), g + 0x28, false, 1);
    let strings = fn_196394(m, h, heap, rd(m, g + 0x10).wrapping_mul(4));
    wr(m, g + 0x3a4, strings);
    if strings == 0 {
        return rd(m, 0x217DDC);
    }
    let count = rd(m, g + 0x10) as u16 as i16;
    for i in 0..count.max(0) as u32 {
        depes_read(m, base, size, &mut off, 1, strings + i * 4, false, 1);
        if rb(m, rd(m, strings + i * 4)) != 0 {
            while rb(m, base + off) != 0 {
                off += 1;
            }
            off += 1;
        }
    }
    read!(rd(m, g + 0x10).wrapping_mul(2), g + 0x2c, false, 2);
    read!(rd(m, g + 0x10).wrapping_mul(2), g + 0x30, false, 2);
    read!(rd(m, g + 0x14).wrapping_mul(0xa2), g + 0x34, false, 1);
    read!(rd(m, g + 0x18).wrapping_mul(0xa2), g + 0x38, false, 1);
    read!(rd(m, g + 0x18).wrapping_mul(0xa2), g + 0x3c, false, 1);
    read!(
        rd(m, g + 0x18).wrapping_mul(rd(m, g + 0x1c)),
        g + 0x40,
        false,
        1
    );
    read!(rh(m, g + 0x22).wrapping_mul(2) as u32, g + 0x44, false, 1);
    read!(rh(m, g + 0x20).wrapping_mul(4) as u32, g + 0x48, false, 4);
    for field in (0x4c..=0x64).step_by(4) {
        read!(rh(m, g + 0x20) as u32, g + field, false, 1);
    }
    read!(rd(m, g + 0x24), g + 0x68, false, 1);
    let layers = fn_1963b8(m, h, heap, rh(m, g + 12) as i16 as i32 as u32, 4);
    wr(m, g + 0x39c, layers);
    if layers == 0 {
        return rd(m, 0x217DDC);
    }
    let data = fn_196394(m, h, heap, rh(m, g + 0x22).wrapping_add(1) as u32);
    wr(m, g + 0x3a0, data);
    if data == 0 {
        rd(m, 0x217DDC)
    } else {
        0
    }
}
fn fn_217ac8(
    m: &mut Memory<'_>,
    h: &mut impl Calls,
    obj: u32,
    tag: u32,
    name: u32,
    out: u32,
) -> u32 {
    if fn_176f94(m, obj, tag, rd(m, 0x216E44), 0x40) >= 0x80000000 {
        return rd(m, 0x216E48);
    }
    wh(m, out, 0);
    let mut rc = 0;
    for (handle_off, g_off, size_off) in [(16, 32, 24), (20, 36, 28)] {
        if rd(m, obj + handle_off) == 0 {
            continue;
        }
        let g = rd(m, obj + g_off);
        if g == 0 {
            rc = rd(m, 0x217E00);
            break;
        }
        rc = fn_2171bc(
            m,
            h,
            rd(m, obj),
            name,
            rd(m, obj + handle_off),
            rd(m, obj + size_off),
            g,
        );
        if rc >= 0x80000000 {
            break;
        }
        if rh(m, out) != 0 && rh(m, out) as i32 != rh(m, g + 12) as i16 as i32 {
            rc = rd(m, 0x217E00) + 0x12;
            break;
        }
        wh(m, out, rh(m, g + 12));
        wr(m, g + 0x6c, obj + 0x32);
    }
    if rc >= 0x80000000 {
        route_or_host(m, h, 0x216C24, &[obj, tag]);
        return rc;
    }
    let heap = rd(m, rd(m, obj) + 4);
    let rows = fn_1963b8(m, h, heap, rh(m, out) as u32, 4);
    wr(m, obj + 0x28, rows);
    if rows == 0 {
        route_or_host(m, h, 0x216C24, &[obj, tag]);
        return rd(m, 0x217DDC);
    }
    wh(m, obj + 0x30, rh(m, out));
    for i in 0..(rh(m, out) as i16).max(0) as u32 {
        wr(m, rows + i * 4, 0);
        let row = fn_196394(m, h, heap, 0x400);
        wr(m, rows + i * 4, row);
        if row == 0 {
            return rd(m, 0x217DDC);
        }
        for (handle_off, g_off) in [(16, 32), (20, 36)] {
            if rd(m, obj + handle_off) != 0 {
                wr(m, rd(m, rd(m, obj + g_off) + 0x39c) + i * 4, row);
            }
        }
    }
    let flags = fn_1963b8(m, h, heap, 0x400, 1);
    wr(m, obj + 0x2c, flags);
    if flags == 0 {
        return rd(m, 0x217DDC);
    }
    for (handle_off, g_off) in [(16, 32), (20, 36)] {
        if rd(m, obj + handle_off) != 0 {
            wr(m, rd(m, obj + g_off) + 0x3b8, flags);
        }
    }
    rc
}

fn fn_1e0f10(m: &mut Memory<'_>, h: &mut impl Calls, a: u32, b: u32, out: u32) -> u32 {
    if out == 0 {
        return rd(m, 0x1E10D8);
    }
    wr(m, out, 0);
    wr(m, out + 4, 0);
    let sp = fn_129778(m, h, 32);
    let rc = fn_19611c(m, a, b, sp);
    if rc >= 0x80000000 {
        return rc;
    }
    let ctx = rd(m, sp);
    let obj = fn_1963b8(m, h, rd(m, ctx + 4), 1, 0x238);
    if obj == 0 {
        return rd(m, 0x1E1130);
    }
    wr(m, obj, a);
    wr(m, obj + 4, b);
    wr(m, obj + 8, ctx);
    let mut rc = fn_1756d0(m, h, rd(m, ctx + 20), 0x1E10F4, sp + 4, 0);
    if rc < 0x80000000 {
        rc = fn_1e0400(m, h, obj, rd(m, sp + 4));
        if rd(m, sp + 4) != 0 {
            route_or_host(m, h, 0x1757A8, &[rd(m, ctx + 20), rd(m, sp + 4)]);
        }
    }
    if rc < 0x80000000 {
        let cb = [
            rd(m, 0x1E10FC),
            rd(m, 0x1E1104),
            rd(m, 0x1E110C),
            rd(m, 0x1E1100),
            rd(m, 0x1E1108),
        ];
        rc = route_or_host(
            m,
            h,
            0x1964E0,
            &[
                rd(m, ctx + 4),
                0,
                cb[0],
                cb[1],
                cb[2],
                cb[3],
                cb[4],
                obj + 0x234,
            ],
        );
    }
    if rc < 0x80000000 {
        wr(m, out, obj);
        wr(m, out + 4, rd(m, 0x1E0ED0));
    } else {
        route_or_host(m, h, 0x1E0D14, &[obj, rd(m, 0x1E0ED0)]);
    }
    rc
}
fn fn_1e0400(m: &mut Memory<'_>, h: &mut impl Calls, p: u32, voice: u32) -> u32 {
    let sp = fn_129778(m, h, 2048);
    let ctx = rd(m, p + 8);
    let params = rd(m, ctx + 20);
    for off in [12, 20, 28, 36, 16, 24, 32, 40, 0x22c, 0x230] {
        if rd(m, p + off) != 0 {
            route_or_host(m, h, 0x2412A0, &[rd(m, p + off)]);
            wr(m, p + off, 0);
        }
    }
    let mut query = |m: &mut Memory<'_>, key: &[u8], cell: u32| {
        let key = rom_text(m, key);
        let rc = fn_1756d0(m, h, params, key, cell, 0);
        (rc, rd(m, cell))
    };
    let (rc, language) = query(m, b"langcode", sp + 1024);
    let language = if rc >= 0x80000000 { 0x1E0850 } else { language };
    let (prc, prefix) = query(m, b"fedataprefix", sp + 1028);
    for path in [sp, sp + 128] {
        wb(m, path, 0);
        if prc < 0x80000000 && prefix != 0 && rb(m, prefix) != 0 {
            fn_195cd0(m, path, prefix);
        }
    }
    let (rc, pack) = query(m, b"fedatapackaging", sp + 1032);
    if rc < 0x80000000 && pack != 0 && fn_195d58(m, pack, rom_text(m, b"clc")) == 0 {
        let (rc, cfg) = query(m, b"fecfg", sp + 1036);
        if rc >= 0x80000000 {
            return rc;
        }
        for (path, include_voice) in [(sp, true), (sp + 128, false)] {
            tts_core::fe_text::strcat_195c90(m.bytes, path, rom_text(m, b"clc/"));
            tts_core::fe_text::strcat_195c90(m.bytes, path, language);
            let n = fn_195d90(m, voice);
            let uni = (0..n.saturating_sub(2))
                .any(|i| (0..3).all(|j| rb(m, voice + i + j) == b"uni"[j as usize]));
            if include_voice && !uni {
                for src in [rom_text(m, b"/"), voice] {
                    tts_core::fe_text::strcat_195c90(m.bytes, path, src);
                }
            }
            for src in [rom_text(m, b"/"), cfg, rom_text(m, b":")] {
                tts_core::fe_text::strcat_195c90(m.bytes, path, src);
            }
        }
    }
    let flat = rd(m, ctx + 12) != 0;
    for (path, which) in [(sp, 0), (sp + 128, 1)] {
        let key: &[u8] = match (flat, which) {
            (true, 0) => b"phonmap/",
            (true, _) => b"phonmap_ipa/",
            (false, 0) => b"phonmap_",
            _ => b"phonmap_ipa_",
        };
        for src in [rom_text(m, key), language] {
            tts_core::fe_text::strcat_195c90(m.bytes, path, src);
        }
        if which == 0 {
            for src in [rom_text(m, if flat { b"/" } else { b"_" }), voice] {
                tts_core::fe_text::strcat_195c90(m.bytes, path, src);
            }
        }
        if !flat {
            tts_core::fe_text::strcat_195c90(m.bytes, path, rom_text(m, b".dat"));
        }
    }
    let tags: [&[u8]; 10] = [
        b"LANG", b"LEXA", b"SLAN", b"SLEX", b"VOIC", b"VEXA", b"SVOI", b"SVEX", b"IPA ", b"SAPI",
    ];
    let slots = [12, 20, 28, 36, 16, 24, 32, 40, 0x22c, 0x230];
    let mut status = 0;
    for which in 0..2 {
        let path = sp + which * 128;
        let mode = which * 2 + if flat { 0 } else { 1 };
        let out = sp + 1040;
        status = route_or_host(
            m,
            h,
            0x243A94,
            &[
                rd(m, p),
                rd(m, p + 4),
                mode,
                path,
                rom_text(m, b"PHMP"),
                rd(m, rd(m, 0x1E0844)),
                out,
            ],
        );
        if status >= 0x80000000 {
            return if which != 0 { 0 } else { status };
        }
        let handle = rd(m, out);
        let tag = sp + 1044;
        let size = sp + 1052;
        let data = sp + 1056;
        loop {
            wr(m, size, 0);
            wr(m, data, 0);
            if route_or_host(m, h, 0x24277C, &[handle, tag, size, data]) >= 0x80000000 {
                break;
            }
            if fn_195d58(m, tag, rom_text(m, b"LINF")) == 0
                || fn_195d58(m, tag, rom_text(m, b"VINF")) == 0
            {
                let mut off = 0;
                while status < 0x80000000 && off < rd(m, size) {
                    wb(m, sp + 256, 0);
                    wb(m, sp + 512, 0);
                    wr(m, sp + 1060, 256);
                    status = route_or_host(
                        m,
                        h,
                        0x242444,
                        &[handle, rd(m, data), rd(m, size), off, sp + 512, sp + 1060],
                    );
                    if status >= 0x80000000 || off >= rd(m, size) {
                        break;
                    }
                    off += rd(m, sp + 1060);
                    wr(m, sp + 1060, 256);
                    status = route_or_host(
                        m,
                        h,
                        0x242444,
                        &[handle, rd(m, data), rd(m, size), off, sp + 256, sp + 1060],
                    );
                    if status >= 0x80000000 {
                        break;
                    }
                    if fn_1769d0(m, sp + 512, rom_text(m, b"version")) == 0 {
                        fn_195cd0(m, p + 0x2c + which * 0x100, sp + 256);
                    }
                }
            } else {
                for (i, key) in tags.iter().enumerate() {
                    if fn_195d58(m, tag, rom_text(m, key)) == 0 {
                        status = route_or_host(
                            m,
                            h,
                            0x2413C4,
                            &[rd(m, p), rd(m, p + 4), path, handle, out],
                        );
                        wr(m, p + slots[i], rd(m, out));
                        break;
                    }
                }
            }
            let r = route_or_host(m, h, 0x2422AC, &[handle]);
            if r >= 0x80000000 && status < 0x80000000 {
                status = r;
            }
        }
        if handle != 0 {
            let r = route_or_host(m, h, 0x2421B0, &[handle]);
            if r >= 0x80000000 && status < 0x80000000 {
                status = r;
            }
        }
    }
    status
}

fn fn_242fd0(m: &mut Memory<'_>, h: &mut impl Calls, a: [u32; 9]) -> u32 {
    let [owner, tag, mode, filename, mapped, mapped_len, kind, version, out] = a;
    let (typ, quiet) = match mode {
        0 => (0, 0),
        1 => (1, 0),
        2 => (0, 1),
        3 => (1, 1),
        4 => (4, 0),
        _ => return 0x8a202007,
    };
    if filename == 0
        || rb(m, filename) == 0
        || kind == 0
        || out == 0
        || (typ == 0 && rb(m, kind) == 0)
        || (typ == 4 && (mapped == 0 || mapped_len == 0))
    {
        return 0x8a202007;
    }
    wr(m, out, 0);
    let sp = fn_129778(m, h, 512);
    let rc = fn_19611c(m, owner, tag, sp);
    if rc >= 0x80000000 {
        return rc;
    }
    let ctx = rd(m, sp);
    let heap = rd(m, ctx + 4);
    let rec = sp + 4;
    fn_3023b0(m, rec, 28);
    wr(m, rec, ctx);
    let rc = match typ {
        0 => route_or_host(m, h, 0x1795F0, &[rd(m, ctx + 12), filename, quiet, rec + 8]),
        1 => route_or_host(m, h, 0x173000, &[rd(m, ctx + 28), heap, filename, rec + 12]),
        4 => {
            wr(m, rec + 16, mapped);
            wr(m, rec + 24, mapped_len);
            0
        }
        _ => unreachable!(),
    };
    if rc >= 0x80000000 {
        return rc;
    }
    let p = fn_1963b8(m, h, heap, 1, 56);
    if p == 0 {
        panic!("MISSING_242FD0_ALLOCATION_FAILURE_CLEANUP");
    }
    wr(m, p, owner);
    wr(m, p + 4, tag);
    wr(m, p + 8, ctx);
    wb(m, p + 12, typ as u8);
    for off in (0..28).step_by(4) {
        wr(m, p + 24 + off, rd(m, rec + off));
    }
    let name = fn_196394(m, h, heap, fn_195d90(m, filename) + 1);
    wr(m, p + 16, name);
    if name == 0 {
        panic!("MISSING_242FD0_NAME_FAILURE_CLEANUP");
    }
    fn_195cd0(m, name, filename);
    let cb = rd(m, 0x24347C);
    let ok = fn_196a64(
        m,
        h,
        heap,
        rd(m, cb),
        rd(m, cb + 4),
        rd(m, cb + 8),
        4,
        p + 20,
    );
    if ok == 0 {
        panic!("MISSING_242FD0_ARRAY_FAILURE_CLEANUP");
    }
    let id = sp + 32;
    let len = sp + 48;
    let data = sp + 52;
    let chunk = sp + 56;
    let buf = sp + 64;
    let cap = sp + 80;
    let end = sp + 84;
    wr(m, cap, 16);
    wb(m, buf, 0);
    let mut rc = route_or_host(m, h, 0x24277C, &[p, chunk, len, 0]);
    if rc < 0x80000000 {
        rc = route_or_host(m, h, 0x242590, &[p, 4, data]);
    }
    if rc < 0x80000000 {
        let be = rb(m, rd(m, 0x243480)) == 0;
        let eq = |m: &Memory<'_>, at: u32, key: &[u8]| {
            key.iter()
                .enumerate()
                .all(|(i, v)| rb(m, at + i as u32) == *v)
                && rb(m, at + key.len() as u32) == 0
        };
        let align =
            if eq(m, chunk, if be { b"IFF " } else { b"RIFF" }) || be && eq(m, chunk, b"RIFX") {
                2
            } else if eq(m, chunk, if be { b"IFF4" } else { b"RIF4" }) {
                4
            } else if eq(m, chunk, if be { b"IFF8" } else { b"RIF8" }) {
                8
            } else {
                rh(m, p + 14)
            };

        if align != 0 {
            wh(m, p + 14, align);
        }
        if align == 0 || rd(m, data) == 0 || rd(m, len) < 4 {
            rc = 0x8a202004;
        } else {
            for i in 0..4 {
                wb(
                    m,
                    id + i,
                    rb(m, rd(m, data) + i) ^ if rb(m, p + 13) == 1 { 0xa9 } else { 0 },
                );
            }
            wb(m, id + 4, 0);
            if rb(m, kind) != 0 {
                if fn_195d58(m, kind, id) != 0 {
                    rc = 0x8a202004;
                }
            } else {
                fn_195cd0(m, kind, id);
            }
            if rc < 0x80000000 {
                wr(m, p + 52, rd(m, p + 52) + 4);
                if typ != 0 && version & 255 == 0 {
                    wr(m, out, p);
                    return rc;
                }
                rc = route_or_host(m, h, 0x24277C, &[p, chunk, len, data]);
                if rc >= 0x80000000 || !eq(m, chunk, b"LIST") || rd(m, len) <= 4 || rd(m, data) == 0
                {
                    rc = 0x8a202004;
                } else {
                    for i in 0..4 {
                        wb(
                            m,
                            id + i,
                            rb(m, rd(m, data) + i) ^ if rb(m, p + 13) == 1 { 0xa9 } else { 0 },
                        );
                    }
                    wb(m, id + 4, 0);
                    if !eq(m, id, b"INFO") || (rd(m, p + 52).wrapping_add(4) as i32) < 0 {
                        rc = 0x8a202004;
                    } else {
                        wr(m, p + 52, rd(m, p + 52) + 4);
                        fn_195cd0(m, id, rom_text(m, b"VERS"));
                        rc = route_or_host(m, h, 0x242C9C, &[p, id, 3, len, data]);
                        if rc < 0x80000000 {
                            rc = route_or_host(
                                m,
                                h,
                                0x242444,
                                &[p, rd(m, data), rd(m, len), 0, buf, cap],
                            );
                        }
                        if rc < 0x80000000 {
                            let mut got = [0; 3];
                            let mut at = buf;
                            let mut valid = true;
                            for i in 0..3 {
                                got[i] =
                                    tts_core::normalization::strtoul_scan(m.bytes, at, end, 10, 0);
                                let ep = rd(m, end);
                                if (got[i] as i32) < if i == 0 { 1 } else { 0 }
                                    || ep == 0
                                    || if i < 2 {
                                        rb(m, ep) != b'.' || rb(m, ep + 1) == 0
                                    } else {
                                        rb(m, ep) != 0 && rb(m, ep) != b'.'
                                    }
                                {
                                    valid = false;
                                    break;
                                }
                                at = ep + 1;
                            }
                            let want = [version & 255, version >> 8 & 255, version >> 16 & 255];
                            if !valid || want < got {
                                rc = 0x8a202004;
                            } else {
                                rc = route_or_host(m, h, 0x2422AC, &[p]);
                                if rc < 0x80000000 {
                                    rc = route_or_host(m, h, 0x2422AC, &[p]);
                                }
                            }
                        } else {
                            rc = 0x8a202004;
                        }
                    }
                }
            }
        }
    }

    if rc < 0x80000000 {
        wr(m, out, p);
    } else {
        route_or_host(m, h, 0x2421B0, &[p]);
    }
    rc
}

fn cold_reader_unmap(m: &mut Memory<'_>, h: &mut impl Calls, r: u32, rec: u32) -> u32 {
    let mut rc = 0;
    if rb(m, rec + 4) == 0 {
        if rd(m, rec + 8) != 0 {
            rc = route_or_host(
                m,
                h,
                0x179208,
                &[rd(m, rd(m, r + 8) + 12), rd(m, rec + 8), rd(m, rec + 16)],
            );
        } else if rd(m, rec + 12) != 0 {
            rc = route_or_host(m, h, 0x17331C, &[rd(m, rec + 12), rd(m, rec + 16)]);
        }
    }
    if rd(m, rec + 8) != 0 || rd(m, rec + 12) != 0 {
        for off in [16, 24, 20] {
            wr(m, rec + off, 0);
        }
    }
    rc
}
fn fn_241e74(m: &mut Memory<'_>, h: &mut impl Calls, r: u32, pos: u32, len: u32, rec: u32) -> u32 {
    let sp = fn_129778(m, h, 8);
    wr(m, sp, len.max(32));
    let ctx = rd(m, r + 8);
    if rb(m, rec + 4) != 0 {
        for off in [8, 12, 16, 20, 24] {
            wr(m, rec + off, 0);
        }
        wb(m, rec + 4, 0);
    } else if rd(m, rec + 16) != 0 {
        let rc = cold_reader_unmap(m, h, r, rec);
        if rc >= 0x80000000 {
            return rc;
        }
    }
    let mut rc = 0;
    match rb(m, r + 12) {
        0 => {
            if rd(m, rec + 8) == 0 {
                rc = route_or_host(m, h, 0x179F30, &[rd(m, ctx + 12), rd(m, r + 16), rec + 8]);
            }
            if rc < 0x80000000 {
                rc = route_or_host(
                    m,
                    h,
                    0x179374,
                    &[rd(m, ctx + 12), rd(m, rec + 8), pos, sp, rec + 16],
                );
            }
        }
        1 => {
            if rd(m, rec + 12) == 0 {
                rc = route_or_host(
                    m,
                    h,
                    0x173000,
                    &[rd(m, ctx + 28), rd(m, ctx + 4), rd(m, r + 16), rec + 12],
                );
            }
            if rc < 0x80000000 {
                rc = route_or_host(m, h, 0x1731C4, &[rd(m, rec + 12), pos, sp, rec + 16]);
            }
        }
        4 => {
            if pos.wrapping_add(len) > rd(m, rec + 24) {
                rc = 0x8a202004;
            }
        }
        _ => rc = 0x8a202001,
    }
    if rc < 0x80000000 && rd(m, sp) < len {
        cold_reader_unmap(m, h, r, rec);
        rc = 0x8a202004;
    }
    if rd(m, rec + 8) != 0 || rd(m, rec + 12) != 0 {
        if rc < 0x80000000 {
            wr(m, rec + 20, pos);
            wr(m, rec + 24, rd(m, sp).wrapping_add(pos));
        } else {
            for off in [16, 24, 20] {
                wr(m, rec + off, 0);
            }
        }
    }
    rc
}
fn fn_24277c(
    m: &mut Memory<'_>,
    h: &mut impl Calls,
    r: u32,
    name: u32,
    out_len: u32,
    out_data: u32,
) -> u32 {
    if r == 0 || out_len == 0 {
        return 0x8a202007;
    }
    let sp = fn_129778(m, h, 32);
    wr(m, sp, 0);
    let arr = rd(m, r + 20);
    let last = tts_core::normalization::chunk_last(m.bytes, arr, sp);
    let parent = rd(m, sp);
    let mut rc = 0;
    if last != 0 {
        if rd(m, r + 52).wrapping_add(8) > rd(m, parent + 12) {
            rc = 0x8a202014;
        } else if rd(m, r + 40) == 0
            || rd(m, r + 44) > rd(m, r + 52)
            || rd(m, r + 52).wrapping_add(8) > rd(m, r + 48)
        {
            rc = fn_241e74(m, h, r, rd(m, r + 52), 8, r + 24);
        }
    } else {
        rc = fn_241e74(m, h, r, 0, 8, r + 24);
        wr(m, r + 52, 0);
        if rc < 0x80000000 && matches!(rb(m, rd(m, r + 40)), 0xe0 | 0xfb) {
            wb(m, r + 13, 1);
        }
    }
    if rc < 0x80000000 {
        let off = rd(m, r + 52).wrapping_sub(rd(m, r + 44));
        let src = rd(m, r + 40);
        if src != 0 && off.wrapping_add(4) <= rd(m, r + 48) && name != 0 {
            for i in 0..4 {
                wb(
                    m,
                    name + i,
                    rb(m, src + off + i) ^ if rb(m, r + 13) == 1 { 0xa9 } else { 0 },
                );
            }
            wb(m, name + 4, 0);
        }
        wr(m, out_len, rd(m, src + off + 4));
        wr(m, r + 52, rd(m, r + 52) + 8);
        if !(0..4).all(|i| {
            let c = rb(m, name + i);
            c.is_ascii_alphanumeric() || c == b' '
        }) {
            rc = 0x8a202004;
        }
        if rc < 0x80000000 && parent != 0 {
            let len = rd(m, out_len);
            let end = rd(m, parent + 12);
            if len > end || (rd(m, r + 52).wrapping_add(len) as i32) > end as i32 {
                rc = 0x8a202004;
            }
        }
        if rc < 0x80000000 && out_data != 0 {
            let pos = rd(m, r + 52);
            let len = rd(m, out_len);
            if rd(m, r + 40) == 0 || rd(m, r + 44) > pos || pos.wrapping_add(len) > rd(m, r + 48) {
                rc = fn_241e74(m, h, r, pos, len, r + 24);
            }
        }
        if rc < 0x80000000 {
            let entry = sp + 8;
            fn_3023b0(m, entry, 16);
            wr(m, entry + 4, r);
            fn_195cd0(m, entry, name);
            wr(m, entry + 8, rd(m, r + 52));
            wr(m, entry + 12, rd(m, r + 52).wrapping_add(rd(m, out_len)));
            if fn_196db0(m, h, rd(m, r + 20), entry) != 0 {
                if out_data != 0 {
                    wr(
                        m,
                        out_data,
                        rd(m, r + 40).wrapping_add(rd(m, r + 52).wrapping_sub(rd(m, r + 44))),
                    );
                }
            } else {
                rc = 0x8a20200a;
            }
        }
    }
    if rc >= 0x80000000 {
        wb(m, name, 0);
        wr(m, out_len, 0);
        if out_data != 0 {
            wr(m, out_data, 0);
        }
    }
    rc
}
fn fn_2422ac(m: &mut Memory<'_>, r: u32) -> u32 {
    if r == 0 {
        return 0x8a202007;
    }
    let arr = rd(m, r + 20);
    let count = rd(m, arr + 20);
    if count == 0 {
        return 0x8a202000;
    }
    let last = rd(m, arr) + (count - 1) * rh(m, arr + 4) as u32;
    let size = rd(m, last + 12);
    let align = rh(m, r + 14) as u32;
    assert_ne!(align, 0, "ARM reader alignment divide by zero");
    wr(m, r + 52, size.wrapping_add(align - 1) / align * align);
    if tts_core::normalization::chunk_pop(m.bytes, arr) == 0 {
        0x8a202008
    } else {
        0
    }
}
fn fn_242590(m: &mut Memory<'_>, h: &mut impl Calls, r: u32, len: u32, out: u32) -> u32 {
    if r == 0 || len == 0 || out == 0 {
        return 0x8a202007;
    }
    if rd(m, r + 40) == 0 {
        return 0x8a202011;
    }
    let pos = rd(m, r + 52);
    let mut rc = 0;
    if rd(m, r + 40) == 0 || rd(m, r + 44) > pos || pos.wrapping_add(len) > rd(m, r + 48) {
        rc = fn_241e74(m, h, r, pos, len, r + 24);
    }
    wr(
        m,
        out,
        if rc >= 0x80000000 {
            0
        } else {
            rd(m, r + 40).wrapping_add(rd(m, r + 52).wrapping_sub(rd(m, r + 44)))
        },
    );
    rc
}

fn fn_242c9c(
    m: &mut Memory<'_>,
    h: &mut impl Calls,
    r: u32,
    name: u32,
    mode: u32,
    out_len: u32,
    out_data: u32,
) -> u32 {
    if r == 0 || out_len == 0 {
        return 0x8a202007;
    }
    let count = tts_core::normalization::idx5(m.bytes, rd(m, r + 20));
    if count == 0 {
        return 0x8a202007;
    }
    let sp = fn_129778(m, h, 32);
    wr(m, sp, 0);
    let source = rd(m, r + 20);
    let saved = rd(m, r + 52);
    let mut rh = RouteHost { inner: h };
    if tts_core::reentrant2::clone_table_m(m, source, sp, &mut rh) == 0 {
        return 0x8a20200a;
    }
    let clone = rd(m, sp);
    let mut cleanup = false;
    let mut rc = 0;
    match mode {
        0 => {
            wr(m, r + 52, 12);
            for _ in 1..count {
                tts_core::normalization::chunk_pop(m.bytes, rd(m, r + 20));
            }
        }
        1 => {
            if count != 1 {
                if tts_core::normalization::span_at(m.bytes, rd(m, r + 20), 1, sp + 4) == 0 {
                    cleanup = true;
                } else {
                    wr(m, r + 52, rd(m, rd(m, sp + 4) + 12));
                    for _ in 1..count {
                        tts_core::normalization::chunk_pop(m.bytes, rd(m, r + 20));
                    }
                }
            }
        }
        2 => {
            if tts_core::normalization::chunk_last(m.bytes, rd(m, r + 20), sp + 4) == 0 {
                cleanup = true;
            } else {
                wr(m, r + 52, rd(m, rd(m, sp + 4) + 8));
            }
        }
        3 => {}
        _ => cleanup = true,
    }
    if !cleanup {
        loop {
            rc = fn_24277c(m, rh.inner, r, sp + 8, out_len, 0);
            if rc >= 0x80000000 {
                cleanup = true;
                break;
            }
            if fn_195d58(m, sp + 8, name) == 0 {
                break;
            }
            rc = fn_2422ac(m, r);
            if rc >= 0x80000000 {
                cleanup = true;
                break;
            }
        }
        if !cleanup && out_data != 0 {
            rc = fn_242590(m, rh.inner, r, rd(m, out_len), out_data);
            if rc >= 0x80000000 {
                cleanup = true;
            }
        }
    }
    if cleanup {
        rc = 0x8a202014;
        if clone != 0 {
            fn_196c7c(m, rh.inner, rd(m, r + 20));
            wr(m, r + 20, clone);
            wr(m, r + 52, saved);
        }
        wr(m, out_len, 0);
        if out_data != 0 {
            wr(m, out_data, 0);
        }
    } else {
        fn_196c7c(m, rh.inner, clone);
    }
    rc
}

fn fn_244324(m: &mut Memory<'_>, h: &mut impl Calls, rec: u32) -> u32 {
    if rb(m, rec + 4) != 0 {
        for off in [8, 12, 16, 20, 24] {
            wr(m, rec + off, 0);
        }
        wb(m, rec + 4, 0);
        return 0;
    }
    if rd(m, rec + 16) != 0 {
        let ctx = rd(m, rec);
        let mut rc = 0;
        if rd(m, rec + 8) != 0 {
            rc = route_or_host(
                m,
                h,
                0x179208,
                &[rd(m, ctx + 12), rd(m, rec + 8), rd(m, rec + 16)],
            );
        } else if rd(m, rec + 12) != 0 {
            rc = route_or_host(m, h, 0x17331C, &[rd(m, rec + 12), rd(m, rec + 16)]);
        }
        if rd(m, rec + 8) != 0 || rd(m, rec + 12) != 0 {
            for off in [16, 24, 20] {
                wr(m, rec + off, 0);
            }
        }
        wr(m, rec + 16, 0);
        if rc >= 0x80000000 {
            return rc;
        }
    }
    if rd(m, rec + 8) != 0 {
        let rc = route_or_host(m, h, 0x179F3C, &[rd(m, rd(m, rec) + 12), rd(m, rec + 8)]);
        wr(m, rec + 8, 0);
        if rc >= 0x80000000 {
            return rc;
        }
    }
    if rd(m, rec + 12) != 0 {
        let rc = route_or_host(m, h, 0x1730FC, &[rd(m, rec + 12)]);
        wr(m, rec + 12, 0);
        return rc;
    }
    0
}
fn fn_2421b0(m: &mut Memory<'_>, h: &mut impl Calls, r: u32) -> u32 {
    if r == 0 {
        return 0x8a202007;
    }
    let heap = rd(m, rd(m, r + 8) + 4);
    let rc = fn_244324(m, h, r + 24);
    if rd(m, r + 16) != 0 {
        fn_196428(m, h, heap, rd(m, r + 16));
    }
    if rd(m, r + 20) != 0 {
        fn_196c7c(m, h, rd(m, r + 20));
    }
    if rc < 0x80000000 {
        fn_196428(m, h, heap, r);
    }
    rc
}

fn fn_243ad4(m: &mut Memory<'_>, h: &mut impl Calls, r: u32, out: u32, data: u32) -> u32 {
    if r == 0 || out == 0 || data == 0 {
        return 0x8a202007;
    }
    if rd(m, r + 40) == 0 {
        return 0x8a202011;
    }
    let heap = rd(m, rd(m, r + 8) + 4);
    let p = fn_1963b8(m, h, heap, 1, 28);
    wr(m, out, p);
    if p == 0 {
        return 0x8a20200a;
    }
    let sp = fn_129778(m, h, 8);
    wr(m, sp, 0);
    let mut rc = 0;
    if tts_core::normalization::chunk_last(m.bytes, rd(m, r + 20), sp) == 0 {
        rc = 0x8a202000;
    } else {
        let chunk = rd(m, sp);
        if !(rb(m, r + 28) == 0
            && rd(m, r + 44) == rd(m, r + 52)
            && rd(m, r + 48) == rd(m, chunk + 12))
        {
            rc = fn_241e74(
                m,
                h,
                r,
                rd(m, r + 52),
                rd(m, chunk + 12).wrapping_sub(rd(m, r + 52)),
                r + 24,
            );
        }
        if rc < 0x80000000 {
            match rb(m, r + 12) {
                0 => {
                    route_or_host(m, h, 0x17A1F4, &[rd(m, rd(m, r + 8) + 12), rd(m, r + 32)]);
                }
                1 => {
                    route_or_host(m, h, 0x1733A0, &[rd(m, r + 36)]);
                }
                _ => {}
            }
            for i in 0..28 {
                wb(m, p + i, rb(m, r + 24 + i));
            }
            if rb(m, r + 12) != 4 {
                wb(m, r + 28, 1);
            }
            wr(m, data, rd(m, p + 16));
        }
    }
    if rc >= 0x80000000 {
        fn_196428(m, h, heap, p);
        wr(m, out, 0);
    }
    rc
}
fn fn_2413c4(
    m: &mut Memory<'_>,
    h: &mut impl Calls,
    object: u32,
    tag: u32,
    name: u32,
    reader: u32,
    out: u32,
) -> u32 {
    if name == 0 || reader == 0 || out == 0 {
        return 0x8a202007;
    }
    wr(m, out, 0);
    let sp = fn_129778(m, h, 32);
    let rc = fn_19611c(m, object, tag, sp);
    if rc >= 0x80000000 {
        return rc;
    }
    let ctx = rd(m, sp);
    let heap = rd(m, ctx + 4);
    let obj = fn_1963b8(m, h, heap, 1, 44);
    if obj == 0 {
        return 0x8a20200a;
    }
    wr(m, obj, object);
    wr(m, obj + 4, tag);
    wr(m, obj + 8, ctx);
    let named = fn_196394(m, h, heap, fn_195d90(m, name) + 1);
    wr(m, obj + 12, named);
    if named == 0 {
        fn_196428(m, h, heap, obj);
        return 0x8a20200a;
    }
    fn_195cd0(m, named, name);
    let buf = sp + 4;
    let len = sp + 12;
    let data = sp + 16;
    let mut rc = fn_24277c(m, h, reader, buf, len, data);
    if rc < 0x80000000 && (fn_195d58(m, buf, rom_text(m, b"STMC")) != 0 || rd(m, len) < 10) {
        rc = 0x8a202004;
    }
    if rc < 0x80000000 {
        let cur = rd(m, data);
        for (i, off) in [16, 18, 20, 22, 32].iter().enumerate() {
            wh(m, obj + off, rh(m, cur + i as u32 * 2));
        }
        fn_2422ac(m, reader);
        for (key, width, pointer, record) in [
            (b"STMF" as &[u8], 22, 24, 28),
            (b"STMR" as &[u8], 32, 36, 40),
        ] {
            if width == 32 && rh(m, obj + 32) == 0 {
                break;
            }
            rc = fn_24277c(m, h, reader, buf, len, data);
            if rc >= 0x80000000 {
                break;
            }
            if fn_195d58(m, buf, rom_text(m, key)) != 0
                || (rh(m, obj + 18) as u32 + rh(m, obj + 20) as u32) * rh(m, obj + width) as u32
                    > rd(m, len)
            {
                rc = 0x8a202004;
                break;
            }
            if fn_243ad4(m, h, reader, obj + record, data) < 0x80000000 {
                wr(m, obj + pointer, rd(m, data));
            }
            fn_2422ac(m, reader);
        }
    }
    if rc < 0x80000000 {
        wr(m, out, obj);
    } else {
        fn_2422ac(m, reader);
        route_or_host(m, h, 0x2412A0, &[obj]);
    }
    rc
}

impl<H: Calls> tts_core::normalization::NormalizationHost for RouteHost<'_, H> {
    fn calloc(&mut self, b: &mut [u8], heap: u32, n: u32, size: u32) -> Option<u32> {
        let r = fn_1963b8(&mut Memory { base: 0, bytes: b }, self.inner, heap, n, size);
        (r != 0).then_some(r)
    }
    fn alloc(&mut self, b: &mut [u8], heap: u32, size: u32) -> Option<u32> {
        let r = fn_196394(&mut Memory { base: 0, bytes: b }, self.inner, heap, size);
        (r != 0).then_some(r)
    }
    fn realloc(&mut self, b: &mut [u8], heap: u32, p: u32, size: u32) -> Option<u32> {
        let r = fn_1963e0(&mut Memory { base: 0, bytes: b }, self.inner, heap, p, size);
        (r != 0).then_some(r)
    }
    fn free(&mut self, b: &mut [u8], heap: u32, p: u32) {
        fn_196428(&mut Memory { base: 0, bytes: b }, self.inner, heap, p);
    }
    fn resolve(&mut self, b: &mut [u8], a: u32, c: u32) -> (u32, u32) {
        let mut m = Memory { base: 0, bytes: b };
        let sp = fn_129778(&mut m, self.inner, 8);
        let rc = fn_19611c(&mut m, a, c, sp);
        (rc, rd(&m, sp))
    }
}
fn fn_230a14(m: &mut Memory<'_>, h: &mut impl Calls, a: u32, b: u32, out: u32) -> u32 {
    let pair = pair_176fd4(m);
    if [a, b] == pair || out == 0 {
        return rd(m, 0x230B90);
    }
    wr(m, out, 0);
    wr(m, out + 4, 0);
    let sp = fn_129778(m, h, 16);
    let rc = fn_19611c(m, a, b, sp);
    if rc >= 0x80000000 {
        return rd(m, 0x230B90);
    }
    let ctx = rd(m, sp);
    let obj = fn_1963b8(m, h, rd(m, ctx + 4), 1, 300);
    if obj == 0 {
        return rd(m, 0x230BB0);
    }
    wr(m, obj + 0x108, rd(m, 0x230B90));
    wr(m, obj, a);
    wr(m, obj + 4, b);
    fn_195cd0(m, obj + 0x88, 0x230B94);
    let rc = route_or_host(m, h, 0x17B058, &[rd(m, ctx + 24), 0x230B9C, sp + 4]);
    if rc < 0x80000000 {
        wr(m, obj + 0x110, rd(m, rd(m, sp + 4) + 4));
    }
    let rc = route_or_host(m, h, 0x2B0B48, &[obj]);
    if rc < 0x80000000 {
        wr(m, out, obj);
        wr(m, out + 4, 0x354);
    } else {
        route_or_host(m, h, 0x230980, &[obj, 0x354]);
    }
    rc
}

impl<H: Calls> tts_core::component_live_b::Host for RouteHost<'_, H> {
    fn call_m(&mut self, f: u32, a: &[u32], m: &mut Memory<'_>) -> u32 {
        route_or_host(m, self.inner, f, a)
    }
}
fn fn_23fe84(m: &mut Memory<'_>, h: &mut impl Calls, a: u32, b: u32, out: u32) -> u32 {
    if out == 0 {
        return rd(m, 0x23FF58);
    }
    wr(m, out, 0);
    wr(m, out + 4, 0);
    let sp = fn_129778(m, h, 8);
    let rc = fn_19611c(m, a, b, sp);
    if rc >= 0x80000000 {
        return rc;
    }
    let obj = fn_1963b8(m, h, rd(m, rd(m, sp) + 4), 1, 24);
    if obj == 0 {
        return 0x8a50200a;
    }
    wr(m, obj, rd(m, sp));
    wr(m, out, obj);
    wr(m, out + 4, rd(m, 0x23FE40));
    rc
}
fn fn_2ae690(m: &mut Memory<'_>, h: &mut impl Calls, a: u32, b: u32, out: u32) -> u32 {
    if out == 0 {
        return 0x8a502007;
    }
    let sp = fn_129778(m, h, 8);
    let rc = fn_19611c(m, a, b, sp);
    if rc >= 0x80000000 {
        return 0x8a502007;
    }
    wr(m, out, 0);
    let heap = rd(m, rd(m, sp) + 4);
    let p = fn_196394(m, h, heap, 16);
    if p == 0 {
        return 0x8a50200a;
    }
    fn_3023b0(m, p, 16);
    wr(m, p, a);
    wr(m, p + 4, b);
    let mut rc = fn_23fe84(m, h, a, b, p + 8);
    if rc < 0x80000000 {
        let mut rh = RouteHost { inner: h };
        rc = tts_core::component_live_b::pcre_init_m(
            m,
            rd(m, p + 8),
            rd(m, p + 12),
            10,
            10,
            &mut rh,
        );
    }
    if rc >= 0x80000000 {
        fn_196428(m, h, heap, p);
    } else {
        wr(m, out, p);
    }
    rc
}
fn fn_22af2c(m: &mut Memory<'_>, h: &mut impl Calls, a: u32, b: u32, out: u32) -> u32 {
    if out == 0 {
        return 0x88602007;
    }
    let pair = pair_176fd4(m);
    wr(m, out, pair[0]);
    wr(m, out + 4, pair[1]);
    let sp = fn_129778(m, h, 64);
    let rc = fn_19611c(m, a, b, sp);
    if rc >= 0x80000000 {
        return 0x88602007;
    }
    let ctx = rd(m, sp);
    let heap = rd(m, ctx + 4);
    let reg = rd(m, ctx + 24);
    let params = rd(m, ctx + 20);
    let obj = fn_1963b8(m, h, heap, 1, 56);
    if obj == 0 {
        return 0x8860200a;
    }
    wr(m, obj, a);
    wr(m, obj + 4, b);
    let mut rc = route_or_host(m, h, 0x17B058, &[reg, 0x22AF20, sp + 4]);
    if rc < 0x80000000 {
        wr(m, obj + 32, rd(m, rd(m, sp + 4) + 4));
        if route_or_host(m, h, 0x17B058, &[reg, 0x22AF18, sp + 4]) < 0x80000000 {
            let info = rd(m, sp + 4);
            for (dst, src) in [(8, 4), (12, 8), (16, 12)] {
                wr(m, obj + dst, rd(m, info + src));
            }
        }
        rc = fn_2ae690(m, h, a, b, obj + 20);
    }
    let globals = rd(m, 0x22AF14);
    if rc < 0x80000000
        && fn_196a64(
            m,
            h,
            heap,
            rd(m, globals + 0x80),
            rd(m, globals + 0x84),
            rd(m, globals + 0x88),
            1,
            obj + 24,
        ) == 0
    {
        rc = 0x88602007;
    }
    let markup = 0x22AE14;
    if rc < 0x80000000 {
        if fn_1756d0(m, h, params, markup, sp + 8, 0) >= 0x80000000 {
            wb(m, obj + 28, 0);
            let work = fn_129778(m, h, 4096);
            rc = fn_1759a0(m, h, params, markup, 0x22B20C, work);
        } else {
            let p = rd(m, sp + 8);
            let val = p != 0
                && rb(m, p) != 0
                && (fn_1769d0(m, p, 0x22AE28) == 0 || fn_195d58(m, p, 0x22AE30) == 0);
            wb(m, obj + 28, val as u8);
        }
    }
    if rc < 0x80000000 {
        rc = fn_174e74(
            m,
            h,
            params,
            markup,
            obj,
            rd(m, globals + 0x90),
            rd(m, globals + 0x94),
        );
    }
    if rc < 0x80000000 {
        wb(m, obj + 52, 0);
        if fn_1756d0(m, h, params, 0x22B214, sp + 12, 0) < 0x80000000
            && rd(m, sp + 12) != 0
            && rb(m, rd(m, sp + 12)) != 0
        {
            wb(m, obj + 52, 1);
        }
        rc = route_or_host(m, h, 0x1757B8, &[params, 0x22B228, 1]);
    }
    if rc >= 0x80000000 {
        route_or_host(m, h, 0x22AE34, &[obj]);
        return rc;
    }
    wr(m, out, obj);
    wr(m, out + 4, rd(m, 0x22B234));
    0
}

impl<H: Calls> tts_core::boot_port_5::Host for RouteHost<'_, H> {
    fn call(&mut self, m: &mut Memory<'_>, f: u32, a: &[u32]) -> u32 {
        route_or_host(m, self.inner, f, a)
    }
}
impl<H: Calls> tts_core::boot_port_5::ContextHost for RouteHost<'_, H> {
    fn context_pair(&mut self, m: &mut Memory<'_>) -> [u8; 8] {
        let p = pair_176fd4(m);
        ((p[1] as u64) << 32 | p[0] as u64).to_le_bytes()
    }
    fn default_context(&mut self, m: &mut Memory<'_>) -> u32 {
        rd(m, 0x196304)
    }
}
fn fn_26402c(m: &mut Memory<'_>, h: &mut impl Calls, a: u32, b: u32, vt: u32, out: u32) -> u32 {
    if out == 0 {
        return 0x81102007;
    }
    let sp = fn_129778(m, h, 32);
    let rc = fn_19611c(m, a, b, sp);
    if rc >= 0x80000000 {
        return 0x81102007;
    }
    wr(m, out, 0);
    let ctx = rd(m, sp);
    let p = fn_196394(m, h, rd(m, ctx + 4), 0x88);
    if p == 0 {
        return 0x8110200a;
    }
    fn_3023b0(m, p, 0x88);
    wr(m, p, a);
    wr(m, p + 4, b);
    wr(m, p + 8, vt);
    wh(m, p + 0x82, 30);
    wh(m, p + 0x84, 50);
    for (key, off) in [(0x264278, 0x82), (0x264290, 0x84)] {
        if route_or_host(m, h, 0x175944, &[rd(m, ctx + 20), key, sp + 4]) < 0x80000000 {
            wh(m, p + off, rd(m, sp + 4) as u16);
        }
    }
    wb(m, p + 0x80, (rb(m, rd(m, 0x2642A4)) != 0) as u8);
    let pair = pair_176fd4(m);
    wr(m, p + 0x68, pair[0]);
    wr(m, p + 0x6c, pair[1]);
    let mut rc = {
        let mut rh = RouteHost { inner: h };
        tts_core::text_input::alloc_break_config(m.bytes, a, b, p + 0x7c, &mut rh)
    };
    if rc < 0x80000000 {
        rc = fn_23fe84(m, h, a, b, p + 0x44);
    }
    if rc < 0x80000000 {
        let mut host = RouteHost { inner: h };
        rc = tts_core::component_live_b::pcre_init_m(
            m,
            rd(m, p + 0x44),
            rd(m, p + 0x48),
            rh(m, p + 0x82) as i32,
            rh(m, p + 0x84) as i32,
            &mut host,
        );
    }
    if rc < 0x80000000 {
        wb(m, sp + 8, 0);
        rc = route_or_host(m, h, 0x263C68, &[p, sp + 8]);
        if rc >= 0x80000000 && rb(m, sp + 8) != 0 {
            return rc;
        }
    }
    if rc >= 0x80000000 {
        route_or_host(m, h, 0x263ABC, &[p]);
        return rc;
    }
    wb(m, p + 0x81, 0);
    if fn_1756d0(m, h, rd(m, ctx + 20), 0x2642CC, sp + 12, 0) < 0x80000000
        && rd(m, sp + 12) != 0
        && rb(m, rd(m, sp + 12)) != 0
    {
        wb(m, p + 0x81, 1);
    }
    wr(m, out, p);
    rc
}
fn fn_1e555c(m: &mut Memory<'_>, h: &mut impl Calls, a: u32, b: u32, out: u32) -> u32 {
    if out == 0 {
        return 0x81102007;
    }
    let sp = fn_129778(m, h, 16);
    let rc = fn_19611c(m, a, b, sp);
    if rc >= 0x80000000 {
        return 0x81102007;
    }
    wr(m, out, 0);
    wr(m, out + 4, 0);
    let ctx = rd(m, sp);
    let p = fn_196394(m, h, rd(m, ctx + 4), 48);
    if p == 0 {
        return 0x8110200a;
    }
    fn_3023b0(m, p, 48);
    wr(m, p, a);
    wr(m, p + 4, b);
    let mut rc = route_or_host(m, h, 0x17B058, &[rd(m, ctx + 24), 0x1E5550, sp + 4]);
    if rc < 0x80000000 {
        let vt = rd(m, rd(m, sp + 4) + 4);
        wr(m, p + 8, vt);
        rc = fn_26402c(m, h, a, b, vt, p + 40);
    }
    if rc < 0x80000000 {
        rc = route_or_host(m, h, 0x252CF8, &[a, b, p + 36]);
    }
    if rc < 0x80000000 {
        let table = rd(m, 0x1E5774);
        for i in 0..7 {
            let at = table + i * 6;
            let mut host = RouteHost { inner: h };
            rc = tts_core::boot_port_5::fn_252e1c(
                m,
                &mut host,
                rd(m, p + 36),
                rh(m, at) as u32,
                rh(m, at + 2) as u32,
                rh(m, at + 4) as u32,
            );
            if rc >= 0x80000000 {
                break;
            }
        }
    }
    if rc >= 0x80000000 {
        route_or_host(m, h, 0x1E5438, &[p, rd(m, 0x1E5544)]);
        return rc;
    }
    if fn_1756d0(m, h, rd(m, ctx + 20), 0x1E5778, sp + 8, 0) < 0x80000000
        && rd(m, sp + 8) != 0
        && rb(m, rd(m, sp + 8)) != 0
    {
        wb(m, p + 44, 1);
    }
    wr(m, out, p);
    wr(m, out + 4, rd(m, 0x1E5544));
    0
}

impl<H: Calls> tts_core::render_part_c::ParserMethodsM for RouteHost<'_, H> {
    fn method_m(&mut self, f: u32, a: &[u32], m: &mut Memory<'_>) -> u32 {
        route_or_host(m, self.inner, f, a)
    }
}
fn fn_263c68(m: &mut Memory<'_>, h: &mut impl Calls, o: u32, flag: u32) -> u32 {
    let sp = fn_129778(m, h, 64);
    let a = rd(m, o);
    let b = rd(m, o + 4);
    let mut rc = route_or_host(
        m,
        h,
        0x2C464C,
        &[
            a,
            b,
            rd(m, o + 0x44),
            rd(m, o + 0x48),
            o + 0x34,
            rd(m, o + 0x7c),
        ],
    );
    if rc >= 0x80000000 {
        return rc;
    }
    for (i, v) in [0x264010, 0x263B70, 0x263A64, o].iter().enumerate() {
        wr(m, sp + i as u32 * 4, *v);
    }
    {
        let mut rh = RouteHost { inner: h };
        rc = tts_core::render_part_c::fn_2c61a0_m(m, rd(m, o + 0x7c), sp, 1, &mut rh);
    }
    if rc >= 0x80000000 {
        return rc;
    }
    rc = route_or_host(
        m,
        h,
        0x2C5EF4,
        &[rd(m, o + 0x7c), 0x264010, rb(m, o + 0x80) as u32],
    );
    if rc >= 0x80000000 {
        return rc;
    }
    rc = fn_19611c(m, a, b, sp + 16);
    if rc >= 0x80000000 {
        return 0x81102007;
    }
    let ctx = rd(m, sp + 16);
    let buf = rd(m, o + 0x70);
    if buf != 0 {
        for i in 0..fn_195d90(m, buf) {
            let c = rb(m, buf + i);
            wb(m, buf + i, fn_17c8c0(m, c as u32) as u8);
        }
        rc = fn_177e0c(m, h, rd(m, ctx + 12), buf, 1, 1, sp + 24, o + 0x64);
        if rc < 0x80000000 {
            rc = route_or_host(
                m,
                h,
                rd(m, rd(m, o + 0x64) + 16),
                &[rd(m, sp + 24), rd(m, sp + 28), a, b, o + 0x68],
            );
            if rc >= 0x80000000 {
                return rc;
            }
        }
    } else {
        route_or_host(
            m,
            h,
            0x173DC0,
            &[rd(m, ctx + 16), rd(m, 0x26400C), rd(m, 0x264008), 0],
        );
    }
    if rb(m, o + 0x77) != 0 {
        rc = route_or_host(m, h, 0x17B058, &[rd(m, ctx + 24), 0x264018, sp + 32]);
        if rc >= 0x80000000 {
            wb(m, flag, 1);
            route_or_host(m, h, 0x263ABC, &[o]);
            return rc;
        }
        let first = rd(m, sp + 32);
        for i in 0..3 {
            wr(m, o + 0x58 + i * 4, rd(m, first + 4 + i * 4));
        }
        rc = route_or_host(m, h, 0x17B058, &[rd(m, ctx + 24), 0x264020, sp + 32]);
        if rc >= 0x80000000 {
            route_or_host(m, h, 0x17B1C4, &[rd(m, ctx + 24), 0x264018]);
            for off in [0x58, 0x5c, 0x60] {
                wr(m, o + off, 0);
            }
            wb(m, flag, 1);
            route_or_host(m, h, 0x263ABC, &[o]);
            return rc;
        }
        let second = rd(m, sp + 32);
        for i in 0..3 {
            wr(m, o + 0x4c + i * 4, rd(m, second + 4 + i * 4));
        }
        rc = route_or_host(
            m,
            h,
            0x2BA304,
            &[
                a,
                b,
                rd(m, o + 0x44),
                rd(m, o + 0x48),
                o + 0x3c,
                rd(m, o + 0x58),
                rd(m, o + 0x5c),
                rd(m, o + 0x60),
                rd(m, o + 0x4c),
                rd(m, o + 0x50),
                rd(m, o + 0x54),
                rd(m, o + 0x7c),
                rd(m, o + 0x64),
                rd(m, o + 0x68),
                rd(m, o + 0x6c),
            ],
        );
        if rc >= 0x80000000 {
            for (name, off) in [(0x264018, 0x58), (0x264020, 0x4c)] {
                if rd(m, o + off) != 0 {
                    route_or_host(m, h, 0x17B1C4, &[rd(m, ctx + 24), name]);
                    for i in 0..3 {
                        wr(m, o + off + i * 4, 0);
                    }
                }
            }
            wb(m, flag, 1);
            route_or_host(m, h, 0x263ABC, &[o]);
            return rc;
        }
    } else {
        let pair = pair_176fd4(m);
        rc = route_or_host(m, h, 0x2C3DB8, &[a, b, o + 0x3c, 0, pair[0], pair[1], 0]);
        if rc >= 0x80000000 {
            return rc;
        }
    }
    rc = route_or_host(
        m,
        h,
        0x2BF760,
        &[
            a,
            b,
            rd(m, o + 0x44),
            rd(m, o + 0x48),
            rb(m, o + 0x77) as u32,
            o + 0x38,
            rd(m, o + 0x7c),
        ],
    );
    if rc >= 0x80000000 {
        return rc;
    }
    rc = route_or_host(m, h, 0x2C6194, &[rd(m, o + 0x7c), rb(m, o + 0x80) as u32]);
    if rc >= 0x80000000 {
        return rc;
    }
    route_or_host(m, h, 0x2C3068, &[a, b, o + 0x40])
}
fn fn_2c464c(m: &mut Memory<'_>, h: &mut impl Calls, a: &[u32]) -> u32 {
    if a[4] == 0 {
        return 0x80002007;
    }
    let sp = fn_129778(m, h, 8);
    if fn_19611c(m, a[0], a[1], sp) >= 0x80000000 {
        return 0x80002007;
    }
    wr(m, a[4], 0);
    let heap = rd(m, rd(m, sp) + 4);
    let p = fn_196394(m, h, heap, 24);
    if p == 0 {
        return 0x8000200a;
    }
    fn_3023b0(m, p, 24);
    wr(m, p, a[0]);
    wr(m, p + 4, a[1]);
    let rc = route_or_host(m, h, 0x2EEE7C, &[a[0], a[1], a[2], a[3], p + 8, a[5]]);
    if rc < 0x80000000 {
        wr(m, a[4], p);
        0
    } else {
        fn_196428(m, h, heap, p);
        rc
    }
}

fn fn_2eee7c(m: &mut Memory<'_>, h: &mut impl Calls, a: &[u32]) -> u32 {
    if a[4] == 0 {
        return 0x80002007;
    }
    let sp = fn_129778(m, h, 32);
    if fn_19611c(m, a[0], a[1], sp) >= 0x80000000 {
        return 0x80002007;
    }
    wr(m, a[4], 0);
    let ctx = rd(m, sp);
    let heap = rd(m, ctx + 4);
    let obj = fn_1963b8(m, h, heap, 1, 76);
    if obj == 0 {
        return 0x8000200a;
    }
    wr(m, obj, ctx);
    wr(m, obj + 4, a[5]);
    wr(m, obj + 12, a[2]);
    wr(m, obj + 16, a[3]);
    for (i, v) in [0x2EEFB8, rd(m, 0x2EEFC0), 0x2EDBF0, obj]
        .iter()
        .enumerate()
    {
        wr(m, sp + 8 + i as u32 * 4, *v);
    }
    let rc = {
        let mut rh = RouteHost { inner: h };
        tts_core::render_part_c::fn_2c61a0_m(m, a[5], sp + 8, 1, &mut rh)
    };
    if rc >= 0x80000000 {
        fn_196428(m, h, heap, obj);
        return rc;
    }
    wh(m, obj + 72, 0x200);
    if route_or_host(m, h, 0x175944, &[rd(m, ctx + 20), 0x2EEFC4, sp + 4]) < 0x80000000 {
        wh(m, obj + 72, (rd(m, sp + 4) * 2 + 64) as u16);
    }
    wr(m, a[4], obj);
    0
}
fn fn_2c5ef4(m: &mut Memory<'_>, h: &mut impl Calls, config: u32, filter: u32, mode: u32) -> u32 {
    if config == 0 {
        return 0x81102007;
    }
    let sp = fn_129778(m, h, 256);
    let params = rd(m, rd(m, config + 8) + 20);
    let rc = fn_1756d0(m, h, params, 0x2C6148, sp, 0);
    if rc >= 0x80000000 {
        return rc;
    }
    let language = rd(m, sp);
    for i in 0..fn_195d90(m, language) {
        wb(
            m,
            language + i,
            fn_17c8c0(m, rb(m, language + i) as u32) as u8,
        );
    }
    fn_1756d0(m, h, params, 0x2C6154, sp + 4, 0);
    fn_1756d0(m, h, params, 0x2C6164, sp + 8, 0);
    let prefix = rd(m, sp + 4);
    let packaging = rd(m, sp + 8);
    let path = sp + 32;
    wb(m, path, 0);
    if prefix != 0 && rb(m, prefix) != 0 {
        fn_195cd0(m, path, prefix);
    }
    let clc = packaging != 0 && fn_195d58(m, packaging, 0x2C6174) == 0;
    if clc {
        let rc = fn_1756d0(m, h, params, 0x2C6178, sp + 12, 0);
        if rc >= 0x80000000 {
            return rc;
        }
        for src in [0x2C6180, language, 0x2C6188, rd(m, sp + 12), 0x2C618C] {
            tts_core::fe_text::strcat_195c90(m.bytes, path, src);
        }
    }
    for src in [0x2C6190, language] {
        tts_core::fe_text::strcat_195c90(m.bytes, path, src);
    }
    for i in 0..fn_195d90(m, path) {
        wb(m, path + i, fn_17c8c0(m, rb(m, path + i) as u32) as u8);
    }
    let mut rc = fn_2c6434(m, h, config, path, filter, mode);
    if rc >= 0x80000000 && !clc {
        wb(m, path + fn_195d90(m, path) - 4, 0);
        rc = fn_2c6434(m, h, config, path, filter, mode);
    }
    rc
}
fn fn_2c6434(
    m: &mut Memory<'_>,
    h: &mut impl Calls,
    config: u32,
    path: u32,
    filter: u32,
    mode: u32,
) -> u32 {
    let sp = fn_129778(m, h, 48);
    let broker = rd(m, rd(m, config + 8) + 12);
    let rc = route_or_host(m, h, 0x1795F0, &[broker, path, 1, sp]);
    if rc >= 0x80000000 {
        return rc;
    }
    let rc = route_or_host(m, h, 0x179F3C, &[broker, rd(m, sp)]);
    if rc >= 0x80000000 {
        return rc;
    }
    let rc = route_or_host(
        m,
        h,
        0x243A94,
        &[
            rd(m, config),
            rd(m, config + 4),
            0,
            path,
            rd(m, 0x2C663C),
            rd(m, rd(m, 0x2C6638)),
            sp + 4,
        ],
    );
    if rc >= 0x80000000 {
        return rc;
    }
    let reader = rd(m, sp + 4);
    let mut result;
    loop {
        result = fn_24277c(m, h, reader, sp + 8, sp + 16, sp + 20);
        if result >= 0x80000000 {
            break;
        }
        for i in 0..rh(m, config + 16) as u32 {
            let handler = rd(m, config + 12) + i * 16;
            if fn_195d58(m, rd(m, handler), sp + 8) == 0 {
                if filter == 0 || fn_195d58(m, filter, sp + 8) == 0 {
                    result = route_or_host(
                        m,
                        h,
                        rd(m, handler + 4),
                        &[
                            rd(m, handler + 12),
                            reader,
                            rd(m, sp + 16),
                            rd(m, sp + 20),
                            mode,
                        ],
                    );
                }
                break;
            }
        }
        let rc = fn_2422ac(m, reader);
        if rc >= 0x80000000 && result < 0x80000000 {
            result = rc;
        }
        if result >= 0x80000000 {
            break;
        }
    }
    if result.wrapping_shl(19) == 0xa00000 {
        result = 0;
    }
    let rc = fn_2421b0(m, h, reader);
    if rc >= 0x80000000 && result < 0x80000000 {
        result = rc;
    }
    result
}

fn fn_263b70(m: &mut Memory<'_>, h: &mut impl Calls, a: &[u32]) -> u32 {
    let o = a[0];
    if rd(m, o + 0x70) != 0 {
        return 0;
    }
    let sp = fn_129778(m, h, 16);
    let rc = fn_19611c(m, rd(m, o), rd(m, o + 4), sp);
    if rc >= 0x80000000 {
        return 0x81102007;
    }
    wb(m, o + 0x77, rh(m, a[3]) as u8);
    wr(m, sp + 4, rd(m, a[3] + 2) + 1);
    let p = fn_1963b8(m, h, rd(m, rd(m, sp) + 4), 1, rd(m, sp + 4));
    wr(m, o + 0x70, p);
    if p == 0 {
        return 0x8110200a;
    }
    let rc = route_or_host(
        m,
        h,
        0x242444,
        &[a[1], a[3], a[2], rd(m, a[3] + 6) + 10, p, sp + 4],
    );
    if rc >= 0x80000000 || rd(m, o + 0x70) == 0 {
        0x81102007
    } else {
        rc
    }
}

impl<H: Calls> tts_core::render_part_d::ResourceHost for RouteHost<'_, H> {
    fn release_chunk(&mut self, b: &mut [u8], record: u32) -> u32 {
        let mut m = Memory { base: 0, bytes: b };
        if record == 0 || rd(&m, record + 16) == 0 {
            return 0x8a202007;
        }
        let heap = rd(&m, rd(&m, record) + 4);
        let rc = fn_244324(&mut m, self.inner, record);
        fn_196428(&mut m, self.inner, heap, record);
        rc
    }
}
impl<H: Calls> tts_core::render_part_d::EscapeReaderHost for RouteHost<'_, H> {
    fn next_chunk(
        &mut self,
        b: &mut [u8],
        reader: u32,
        size: u32,
        data: u32,
    ) -> (u32, [u8; 8], u32, u32) {
        let mut m = Memory { base: 0, bytes: b };
        let sp = fn_129778(&mut m, self.inner, 16);
        wr(&mut m, sp + 8, size);
        wr(&mut m, sp + 12, data);
        let rc = fn_24277c(&mut m, self.inner, reader, sp, sp + 8, sp + 12);
        let mut tag = [0; 8];
        for i in 0..8 {
            tag[i] = rb(&m, sp + i as u32);
        }
        (rc, tag, rd(&m, sp + 8), rd(&m, sp + 12))
    }
    fn advance_chunk(&mut self, b: &mut [u8], reader: u32) -> u32 {
        fn_2422ac(&mut Memory { base: 0, bytes: b }, reader)
    }
    fn detach_chunk(
        &mut self,
        b: &mut [u8],
        reader: u32,
        chunk: u32,
        data: u32,
    ) -> (u32, u32, u32) {
        let mut m = Memory { base: 0, bytes: b };
        let sp = fn_129778(&mut m, self.inner, 8);
        wr(&mut m, sp, chunk);
        wr(&mut m, sp + 4, data);
        let rc = fn_243ad4(&mut m, self.inner, reader, sp, sp + 4);
        (rc, rd(&m, sp), rd(&m, sp + 4))
    }
}

struct SharedCalls<'a, 'b>(&'a std::cell::RefCell<&'b mut dyn Calls>);
impl Calls for SharedCalls<'_, '_> {
    fn call(&mut self, m: &mut Memory<'_>, f: u32, a: &[u32]) -> u32 {
        self.0.borrow_mut().call(m, f, a)
    }
    fn call_wide(&mut self, m: &mut Memory<'_>, f: u32, a: &[u32]) -> u64 {
        self.0.borrow_mut().call_wide(m, f, a)
    }
    fn real_log_args(&mut self) -> bool {
        self.0.borrow_mut().real_log_args()
    }
}
struct ColdChunks<'a, 'b>(&'a std::cell::RefCell<&'b mut dyn Calls>);
impl tts_core::text_input::ChunkHost for ColdChunks<'_, '_> {
    fn detach(&mut self, b: &mut [u8], reader: u32, record: &mut u32, data: &mut u32) -> u32 {
        let mut calls = SharedCalls(self.0);
        let mut m = Memory { base: 0, bytes: b };
        let sp = fn_129778(&mut m, &mut calls, 8);
        wr(&mut m, sp, *record);
        wr(&mut m, sp + 4, *data);
        let rc = fn_243ad4(&mut m, &mut calls, reader, sp, sp + 4);
        *record = rd(&m, sp);
        *data = rd(&m, sp + 4);
        rc
    }
    fn release(&mut self, record: u32) -> u32 {
        panic!("MISSING_CHUNK_RELEASE_MEMORY record={record:#x}")
    }
}

fn fn_1ecd04(
    m: &mut Memory<'_>,
    h: &mut impl Calls,
    o: u32,
    tag: u32,
    name: u32,
    found: u32,
) -> u32 {
    let valid = fn_176f94(m, o, tag, 0xf37f, 0x2a4);
    if valid >= 0x80000000 {
        return 0x89802008;
    }
    wb(m, found, 0);
    if name == 0 {
        return 0x89802007;
    }
    for i in 0..rh(m, o + 0x88) as u32 {
        if fn_195d58(m, name, rd(m, rd(m, o + 0x68) + i * 4)) == 0 && (i as u16 as i16) >= 0 {
            wb(m, found, 1);
            return valid;
        }
    }
    let sp = fn_129778(m, h, 16);
    let broker = rd(m, rd(m, o) + 12);
    let mut last = 0;
    for (i, path) in [o + 0x9e, 0x1ECF5C, 0x1ECF64].iter().enumerate() {
        let rc = route_or_host(
            m,
            h,
            0x1795F0,
            &[broker, *path, if i == 0 { 0 } else { 1 }, sp],
        );
        if rc >= 0x80000000 {
            return if i == 0 { rc } else { 0 };
        }
        let st = o + 8 + i as u32 * 32;
        let mut host = RouteHost { inner: h };
        last = tts_core::reentrant::contains_m(
            m,
            &mut host,
            st,
            name,
            rd(m, o + 4 + i as u32 * 32),
            found,
            sp + 4,
        );
        if last >= 0x80000000 {
            return last;
        }
        route_or_host(m, h, 0x179F3C, &[broker, rd(m, sp)]);
        if rb(m, found) != 0 {
            return last;
        }
    }
    last
}
fn fn_2ecaac(m: &mut Memory<'_>, h: &mut impl Calls, name: u32, a: u32, b: u32, out: u32) -> u32 {
    if out == 0 {
        return 0x81102007;
    }
    let sp = fn_129778(m, h, 16);
    if fn_19611c(m, a, b, sp) >= 0x80000000 {
        return 0x81102007;
    }
    wr(m, out, 0);
    let ctx = rd(m, sp);
    let heap = rd(m, ctx + 4);
    let p = fn_196394(m, h, heap, 20);
    if p == 0 {
        return 0x8110200a;
    }
    fn_3023b0(m, p, 20);
    wr(m, p, ctx);
    let copy = fn_196394(m, h, heap, fn_195d90(m, name) + 1);
    wr(m, p + 4, copy);
    let mut rc = 0x8110200a;
    if copy != 0 {
        fn_195cd0(m, copy, name);
        rc = route_or_host(m, h, 0x17B058, &[rd(m, ctx + 24), 0x2ECAA0, sp + 4]);
        if rc < 0x80000000 {
            let info = rd(m, sp + 4);
            for (dst, src) in [(8, 4), (12, 8), (16, 12)] {
                wr(m, p + dst, rd(m, info + src));
            }
            wb(m, sp + 8, 0);
            rc = route_or_host(
                m,
                h,
                rd(m, rd(m, p + 8) + 64),
                &[rd(m, p + 12), rd(m, p + 16), name, sp + 8],
            );
            if rc < 0x80000000 {
                if rb(m, sp + 8) != 0 {
                    wr(m, out, p);
                    return rc;
                }
                rc = 0x81102001;
            }
        }
    }
    if rd(m, p + 8) != 0 {
        route_or_host(m, h, 0x17B1C4, &[rd(m, ctx + 24), 0x2ECAA0]);
    }
    if rd(m, p + 4) != 0 {
        fn_196428(m, h, heap, rd(m, p + 4));
    }
    fn_196428(m, h, heap, p);
    rc
}
fn fn_2c3068(m: &mut Memory<'_>, h: &mut impl Calls, a: u32, b: u32, out: u32) -> u32 {
    if out == 0 {
        return 0x81102007;
    }
    let sp = fn_129778(m, h, 32);
    if fn_19611c(m, a, b, sp) >= 0x80000000 {
        return 0x81102007;
    }
    wr(m, out, 0);
    let ctx = rd(m, sp);
    let heap = rd(m, ctx + 4);
    let p = fn_196394(m, h, heap, 16);
    if p == 0 {
        return 0x8110200a;
    }
    fn_3023b0(m, p, 16);
    wr(m, p, a);
    wr(m, p + 4, b);
    let mut rc = fn_1756d0(m, h, rd(m, ctx + 20), 0x2C31E8, sp + 4, 0);
    if rc < 0x80000000 {
        for (pool, off) in [(0x2C31F4, 8), (0x2C31F8, 12)] {
            fn_195cd0(m, sp + 8, rd(m, pool));
            tts_core::fe_text::strcat_195c90(m.bytes, sp + 8, rd(m, sp + 4));
            rc = fn_2ecaac(m, h, sp + 8, a, b, p + off);
            if rc >= 0x80000000 && rc.wrapping_shl(19) != 0x80000 {
                break;
            }
        }
        if rc < 0x80000000 || rc.wrapping_shl(19) == 0x80000 {
            wr(m, out, p);
            return 0;
        }
    }
    route_or_host(m, h, 0x2C3008, &[p]);
    rc
}

fn fn_15f594(
    m: &mut Memory<'_>,
    h: &mut impl Calls,
    o: u32,
    tag: u32,
    name: u32,
    user: u32,
    max: u32,
) -> u32 {
    if fn_176f94(m, o, tag, 0x15c, 32) >= 0x80000000 {
        return 0x89502008;
    }
    if name == 0 || max == 0 {
        return 0x89502007;
    }
    let user = if rd(m, max) == 0 { 0 } else { user };
    let ctx = rd(m, o + 8);
    let sp = fn_129778(m, h, 160);
    let work = fn_129778(m, h, 4096);
    let mut rc = fn_177b7c(m, h, name, rd(m, o + 12), sp, work);
    if rc >= 0x80000000 {
        return rc;
    }
    let comp = rd(m, sp);
    rc = route_or_host(m, h, 0x1789B0, &[comp, 0x15F924, sp + 4]);
    if rc >= 0x80000000 {
        fn_177cc8(m, h, comp);
        return rc;
    }
    let it = rd(m, sp + 4);
    let mut count = 0;
    let entry = sp + 32;
    loop {
        if route_or_host(m, h, 0x178C24, &[it, sp + 8, 0]) >= 0x80000000 {
            break;
        }
        fn_3023b0(m, entry, 86);
        if route_or_host(
            m,
            h,
            0x17A3B8,
            &[rd(m, ctx + 12), rd(m, sp + 8), rd(m, 0x15F934), 1, sp + 12],
        ) < 0x80000000
        {
            let tags = rd(m, sp + 12);
            loop {
                wr(m, sp + 16, 0);
                wr(m, sp + 20, 0);
                if route_or_host(m, h, 0x17A814, &[tags, sp + 16, sp + 20, 0]) >= 0x80000000 {
                    break;
                }
                let key = rd(m, sp + 16);
                let value = rd(m, sp + 20);
                if key == 0 {
                    continue;
                }
                for (text, off) in [(0x15F938, 0), (0x15F944, 64), (0x15F950, 72)] {
                    if fn_195d58(m, key, text) == 0 {
                        fn_195cd0(m, entry + off, value);
                    }
                }
                if fn_195d58(m, key, 0x15F95C) == 0 {
                    let value =
                        tts_core::normalization::strtoul_scan(m.bytes, value, 0, 10, 0) as u16;
                    wh(m, entry + 84, value);
                }
            }
            route_or_host(m, h, 0x17A2EC, &[tags]);
        }
        if user != 0 {
            if (0..count).any(|i| fn_1769d0(m, entry, user + i * 86) == 0) {
                continue;
            }
            if rd(m, max) != 0 {
                if rd(m, max) <= count {
                    rc = 9;
                } else {
                    for i in 0..86 {
                        wb(m, user + count * 86 + i, rb(m, entry + i));
                    }
                }
            }
        }
        count += 1;
    }
    wr(m, max, count);
    route_or_host(m, h, 0x1788C0, &[it]);
    fn_177cc8(m, h, comp);
    rc
}

fn fn_2029dc(m: &mut Memory<'_>, h: &mut impl Calls, a: u32, b: u32, out: u32) -> u32 {
    if out == 0 {
        return rd(m, 0x202D2C);
    }
    wr(m, out, 0);
    wr(m, out + 4, 0);
    let sp = fn_129778(m, h, 32);
    let rc = fn_19611c(m, a, b, sp);
    if rc >= 0x80000000 {
        return rc;
    }
    let ctx = rd(m, sp);
    let reg = rd(m, ctx + 24);
    let heap = rd(m, ctx + 4);
    let rc = route_or_host(m, h, 0x17B058, &[reg, 0x202D30, sp + 4]);
    if rc >= 0x80000000 {
        return rc;
    }
    let rc = route_or_host(m, h, 0x17B058, &[reg, 0x202D3C, sp + 8]);
    if rc >= 0x80000000 {
        route_or_host(m, h, 0x17B1C4, &[reg, 0x202D30]);
        return rc;
    }
    wr(m, sp + 12, 0);
    route_or_host(m, h, 0x17B058, &[reg, 0x202D48, sp + 12]);
    let obj = fn_1963b8(m, h, heap, 1, 184);
    if obj == 0 {
        panic!("MISSING_2029DC_ALLOCATION_FAILURE_CLEANUP");
    }
    let block = fn_196394(m, h, heap, 8);
    wr(m, obj + 108, block);
    if block == 0 {
        panic!("MISSING_2029DC_BLOCK_FAILURE_CLEANUP");
    }
    wr(m, obj, a);
    wr(m, obj + 4, b);
    wr(m, obj + 8, ctx);
    wr(m, obj + 24, rd(m, rd(m, sp + 4) + 4));
    let depes = rd(m, sp + 8);
    for (dst, src) in [(20, 4), (12, 8), (16, 12)] {
        wr(m, obj + dst, rd(m, depes + src));
    }
    let other = rd(m, sp + 12);
    if other != 0 && rd(m, other + 4) != 0 {
        for (dst, src) in [(88, 4), (92, 8), (96, 12)] {
            wr(m, obj + dst, rd(m, other + src));
        }
    }
    for (off, default, key) in [
        (102, 512, 0x202D58),
        (100, 512, 0x202D6C),
        (104, 1, 0x202D80),
    ] {
        wh(m, obj + off, default);
        if route_or_host(m, h, 0x175944, &[rd(m, ctx + 20), key, sp + 16]) < 0x80000000 {
            wh(m, obj + off, rd(m, sp + 16) as u16);
        }
    }
    for (off, pool) in [
        (112, 0x202D9C),
        (116, 0x202DA0),
        (120, 0x202DA4),
        (124, 0x202DA8),
        (148, 0x202DAC),
        (152, 0x202DB0),
        (160, 0x202DB4),
        (156, 0x202DB8),
    ] {
        wr(m, obj + off, rd(m, pool));
    }
    wb(m, obj + 164, 0);
    for i in 0..16 {
        wb(m, obj + 128 + i, rb(m, obj + 112 + i));
    }
    wr(m, obj + 144, heap);
    wr(m, out, obj);
    wr(m, out + 4, rd(m, 0x202DC8));
    0
}

struct ColdExtra<'a, H>(&'a mut H);
impl<H: Calls> tts_core::boot_port_2c::Host for ColdExtra<'_, H> {
    fn parameter(&mut self, m: &mut Memory<'_>, p: u32, k: u32) -> (u32, u32) {
        let sp = fn_129778(m, self.0, 8);
        let rc = fn_1756d0(m, self.0, p, k, sp, 0);
        (rc, rd(m, sp))
    }
    fn set_parameter(&mut self, m: &mut Memory<'_>, p: u32, k: u32, v: u32) -> u32 {
        let sp = fn_129778(m, self.0, 4096);
        fn_1759a0(m, self.0, p, k, v, sp)
    }
    fn set_owner(&mut self, m: &mut Memory<'_>, p: u32, k: u32, o: u32, b: u32, f: f32) {
        route_or_host(m, self.0, 0x174F9C, &[p, k, o, b, f.to_bits()]);
    }
    fn context(&mut self, m: &mut Memory<'_>, a: u32, b: u32) -> (u32, u32) {
        let sp = fn_129778(m, self.0, 8);
        let rc = fn_19611c(m, a, b, sp);
        (rc, rd(m, sp))
    }
    fn context_into(&mut self, m: &mut Memory<'_>, a: u32, b: u32, out: u32) -> u32 {
        fn_19611c(m, a, b, out)
    }
    fn resource_open(&mut self, m: &mut Memory<'_>, p: u32, k: u32) -> (u32, u32) {
        let sp = fn_129778(m, self.0, 8);
        let rc = fn_17b058(m, self.0, p, k, sp);
        (rc, rd(m, sp))
    }
    fn release_resource(&mut self, m: &mut Memory<'_>, p: u32, k: u32) {
        fn_17b1c4(m, self.0, p, k);
    }
    fn calloc(&mut self, m: &mut Memory<'_>, p: u32, n: u32, s: u32) -> u32 {
        fn_1963b8(m, self.0, p, n, s)
    }
    fn allocate(&mut self, m: &mut Memory<'_>, p: u32, n: u32) -> u32 {
        fn_196394(m, self.0, p, n)
    }
    fn release(&mut self, m: &mut Memory<'_>, p: u32, v: u32) {
        fn_196428(m, self.0, p, v);
    }
    fn map_create(&mut self, m: &mut Memory<'_>, p: u32, flag: u32, cb: [u32; 5], out: u32) -> u32 {
        assert_eq!(cb, tts_core::boot_port_2c::token::ALL);
        fn_1964e0(
            m,
            self.0,
            p,
            flag,
            rd(m, 0x1EF398),
            rd(m, 0x1EF3A0),
            rd(m, 0x1EF3A8),
            rd(m, 0x1EF39C),
            rd(m, 0x1EF3A4),
            out,
        )
    }
    fn map_lookup(&mut self, m: &mut Memory<'_>, p: u32, k: u32) -> (u32, u32) {
        let sp = fn_129778(m, self.0, 8);
        let rc = fn_1966ac(m, self.0, p, k, sp);
        (rc, rd(m, sp))
    }
    fn map_insert(&mut self, m: &mut Memory<'_>, p: u32, k: u32, v: u32) -> u32 {
        fn_19668c(m, self.0, p, k, v)
    }
    fn map_remove(&mut self, m: &mut Memory<'_>, p: u32, k: u32) -> u32 {
        route_or_host(m, self.0, 0x19672C, &[p, k])
    }
    fn clear_map(&mut self, m: &mut Memory<'_>, p: u32) {
        route_or_host(m, self.0, 0x196770, &[p]);
    }
    fn register_owner(
        &mut self,
        m: &mut Memory<'_>,
        p: u32,
        k: u32,
        o: u32,
        b: u32,
        f: f32,
    ) -> u32 {
        fn_174e74(m, self.0, p, k, o, b, f.to_bits())
    }
    fn cleanup(&mut self, m: &mut Memory<'_>, p: u32) -> u32 {
        route_or_host(m, self.0, 0x26C858, &[p])
    }
    fn engine_call(&mut self, m: &mut Memory<'_>, f: u32, a: &[u32]) -> u32 {
        route_or_host(m, self.0, f, a)
    }
    fn secondary_parameters(&mut self, m: &mut Memory<'_>, a: [u32; 4]) -> u32 {
        route_or_host(m, self.0, 0x174CA8, &a)
    }
    fn secondary_linguistic(&mut self, m: &mut Memory<'_>, a: [u32; 4], _: u32) -> u32 {
        route_or_host(m, self.0, 0x17AE04, &a)
    }
    fn broker_open(&mut self, m: &mut Memory<'_>, p: u32, k: u32, out: u32) -> u32 {
        route_or_host(m, self.0, 0x179F30, &[p, k, out])
    }
    fn log(
        &mut self,
        m: &mut Memory<'_>,
        log: u32,
        module: u32,
        code: u32,
        fmt: u32,
        a: u32,
        b: u32,
    ) {
        let sp = fn_129778(m, self.0, 16);
        wr(m, sp, a);
        wr(m, sp + 4, b);
        route_or_host(m, self.0, 0x173DC0, &[log, module, code, fmt, sp]);
    }
}
fn extra_environment(
    m: &mut Memory<'_>,
    h: &mut impl Calls,
) -> tts_core::boot_port_2c::Environment {
    use tts_core::boot_port_2c::{lit, Environment};
    let scratch = fn_129778(m, h, 2048);
    let env = Environment {
        d8: rd(m, 0x1EE9AC),
        d905a: 0x8F905A,
        literals: lit::ADDRESSES,
        scratch: scratch + 512,
    };

    env
}

fn fn_1f4558(m: &mut Memory<'_>, h: &mut impl Calls, a: u32, b: u32, out: u32) -> u32 {
    if out == 0 {
        return rd(m, 0x1F4D70);
    }
    wr(m, out, 0);
    wr(m, out + 4, 0);
    let sp = fn_129778(m, h, 32);
    let rc = fn_19611c(m, a, b, sp);
    if rc >= 0x80000000 {
        return rc;
    }
    let ctx = rd(m, sp);
    let heap = rd(m, ctx + 4);
    let reg = rd(m, ctx + 24);
    let obj = fn_1963b8(m, h, heap, 1, 64);
    if obj == 0 {
        return rd(m, 0x1F4D7C);
    }
    wr(m, obj, ctx);
    let mut rc = 0;
    for (i, name) in [0x1F4534, 0x1F4540, 0x1F454C].iter().enumerate() {
        rc = fn_17b058(m, h, reg, *name, sp + 4 + i as u32 * 4);
        if rc >= 0x80000000 {
            break;
        }
    }
    if rc < 0x80000000 {
        wr(m, obj + 4, rd(m, rd(m, sp + 4) + 4));
        for (slot, fields) in [(sp + 8, [8, 12, 16]), (sp + 12, [28, 20, 24])] {
            let info = rd(m, slot);
            for (i, off) in fields.iter().enumerate() {
                wr(m, obj + off, rd(m, info + 4 + i as u32 * 4));
            }
        }
    }
    wr(m, out, obj);
    wr(m, out + 4, rd(m, 0x1F452C));
    if rc >= 0x80000000 {
        for (off, name) in [(4, 0x1F4534), (8, 0x1F4540), (28, 0x1F454C)] {
            if rd(m, obj + off) != 0 {
                fn_17b1c4(m, h, reg, name);
            }
        }
        fn_196428(m, h, heap, obj);
        wr(m, out, 0);
        wr(m, out + 4, 0);
    }
    rc
}

fn fn_2198b4(m: &mut Memory<'_>, h: &mut impl Calls, a: u32, b: u32, out: u32) -> u32 {
    if out == 0 {
        return rd(m, 0x219B5C);
    }
    wr(m, out, 0);
    wr(m, out + 4, 0);
    let sp = fn_129778(m, h, 64);
    fn_3023b0(m, sp, 64);
    let rc = fn_19611c(m, a, b, sp + 28);
    if rc >= 0x80000000 {
        return rc;
    }
    let ctx = rd(m, sp + 28);
    let reg = rd(m, ctx + 24);
    let names = [0x219890, 0x2198A8, 0x21989C];
    let slots = [sp + 44, sp + 52, sp + 48];
    for i in 0..3 {
        let rc = fn_17b058(m, h, reg, names[i], slots[i]);
        if rc >= 0x80000000 {
            for name in &names[..i] {
                fn_17b1c4(m, h, reg, *name);
            }
            return rc;
        }
    }
    let obj = fn_196394(m, h, rd(m, ctx + 4), 76);
    if obj == 0 {
        route_or_host(
            m,
            h,
            0x173DC0,
            &[rd(m, ctx + 16), rd(m, 0x219B78), rd(m, 0x219B74), 0],
        );
        for name in names {
            fn_17b1c4(m, h, reg, name);
        }
        return rd(m, 0x219B7C);
    }
    wr(m, obj, ctx);
    wr(m, obj + 4, 0);
    wr(m, obj + 8, 0);
    wr(m, obj + 28, rd(m, rd(m, sp + 44) + 4));
    for (slot, fields) in [(sp + 48, [12, 4, 8]), (sp + 52, [24, 16, 20])] {
        let info = rd(m, slot);
        for (i, off) in fields.iter().enumerate() {
            wr(m, obj + off, rd(m, info + 4 + i as u32 * 4));
        }
    }
    wb(m, obj + 64, 0);
    wb(m, obj + 65, 0);
    wr(m, obj + 68, 0);
    wh(m, obj + 72, 0);
    let mut rc = 0;
    for (i, key) in [0x219B60, 0x219B80].iter().enumerate() {
        if i == 1 {
            wh(m, sp + 16, 0);
        }
        rc = route_or_host(
            m,
            h,
            rd(m, rd(m, obj + 12) + 28),
            &[
                rd(m, obj + 4),
                rd(m, obj + 8),
                0x219B6C,
                *key,
                sp + 12,
                sp + 16,
                sp + 20,
            ],
        );
        if rc >= 0x80000000 {
            route_or_host(m, h, 0x219790, &[rd(m, out), rd(m, out + 4)]);
            wr(m, out, 0);
            wr(m, out + 4, 0);
        }
        if rh(m, sp + 16) == 1 && rd(m, sp + 12) != 0 && rb(m, rd(m, rd(m, sp + 12))) == b'1' {
            wb(m, obj + 64 + i as u32, 1);
        }
    }
    wr(m, out, obj);
    wr(m, out + 4, rd(m, 0x219888));
    rc
}

fn fn_1d4d6c(m: &mut Memory<'_>, h: &mut impl Calls, a: u32, b: u32, out: u32) -> u32 {
    if out == 0 {
        return rd(m, 0x1D503C);
    }
    wr(m, out, 0);
    wr(m, out + 4, 0);
    let sp = fn_129778(m, h, 96);
    fn_3023b0(m, sp, 96);
    let rc = fn_19611c(m, a, b, sp + 8);
    if rc >= 0x80000000 {
        return rc;
    }
    let ctx = rd(m, sp + 8);
    let reg = rd(m, ctx + 24);
    let names = [0x1D4AB4, 0x1D4ABC, 0x1D4AC8];
    let slots = [sp + 60, sp + 56, sp + 64];
    for i in 0..3 {
        let rc = fn_17b058(m, h, reg, names[i], slots[i]);
        if rc >= 0x80000000 {
            for name in &names[..i] {
                fn_17b1c4(m, h, reg, *name);
            }
            return rc;
        }
    }
    let obj = fn_1963b8(m, h, rd(m, ctx + 4), 1, 76);
    if obj == 0 {
        route_or_host(
            m,
            h,
            0x173DC0,
            &[rd(m, ctx + 16), rd(m, 0x1D504C), rd(m, 0x1D5048), 0],
        );
        for name in names {
            fn_17b1c4(m, h, reg, name);
        }
        return rd(m, 0x1D5050);
    }
    wr(m, out, obj);
    wr(m, out + 4, rd(m, 0x1D4AAC));
    wr(m, obj, ctx);
    wr(m, obj + 4, a);
    wr(m, obj + 8, b);
    wr(m, obj + 24, rd(m, rd(m, sp + 60) + 4));
    wr(m, obj + 28, rd(m, rd(m, sp + 56) + 4));
    for (i, off) in [20, 12, 16].iter().enumerate() {
        wr(m, obj + off, rd(m, rd(m, sp + 64) + 4 + i as u32 * 4));
    }
    let pair = pair_176fd4(m);
    wr(m, sp + 48, pair[0]);
    wr(m, sp + 52, pair[1]);
    wb(m, sp + 24, 1);
    let mut rc = route_or_host(
        m,
        h,
        0x178510,
        &[rd(m, ctx + 12), 0x1D5040, 1, sp + 48, sp + 28],
    );
    if rc < 0x80000000 {
        wr(m, obj + 72, rd(m, sp + 28));
        rc = route_or_host(
            m,
            h,
            rd(m, rd(m, obj + 20) + 76),
            &[rd(m, obj + 12), rd(m, obj + 16), sp + 12, sp + 44],
        );
        if rc < 0x80000000 {
            wb(
                m,
                obj + 48,
                u8::from(rb(m, sp + 44) == 1 && rb(m, sp + 12) == 1),
            );
            rc = route_or_host(
                m,
                h,
                rd(m, rd(m, obj + 72) + 16),
                &[pair[0], pair[1], a, b, obj + 64],
            );
            if rc < 0x80000000 {
                wr(m, sp + 32, rd(m, obj + 64));
                wr(m, sp + 36, rd(m, obj + 68));
                rc = fn_17b0f8(m, h, reg, 0x1D5044, sp + 24);
                if rc < 0x80000000 {
                    rc = fn_1d4ad8(m, h, obj);
                }
            }
        }
    }
    if rc >= 0x80000000 {
        route_or_host(m, h, 0x1D49B4, &[obj, rd(m, out + 4)]);
        wr(m, out, 0);
        wr(m, out + 4, 0);
    }
    rc
}
fn fn_1d4ad8(m: &mut Memory<'_>, h: &mut impl Calls, obj: u32) -> u32 {
    let sp = fn_129778(m, h, 288);
    fn_3023b0(m, sp, 288);
    let path = sp + 8;
    let ctx = rd(m, obj);
    let params = rd(m, ctx + 20);
    let broker = rd(m, ctx + 12);
    let rc = route_or_host(m, h, 0x175A1C, &[params, 0x1D4D1C, sp + 4]);
    if rc >= 0x80000000 {
        return rc;
    }
    let rc = route_or_host(m, h, 0x175A1C, &[params, 0x1D4D28, sp + 268]);
    if rc < 0x80000000 && rd(m, sp + 268) != 0 && rb(m, rd(m, sp + 268)) != 0 {
        fn_195cd0(m, path, rd(m, sp + 268));
    }
    let rc = route_or_host(m, h, 0x175A1C, &[params, 0x1D4D38, sp + 272]);
    if rc < 0x80000000 && rd(m, sp + 272) != 0 && fn_195d58(m, rd(m, sp + 272), 0x1D4D48) == 0 {
        route_or_host(m, h, 0x175A1C, &[params, 0x1D4D4C, sp + 264]);
        for src in [
            0x1D4D58,
            rd(m, sp + 4),
            0x1D4D5C,
            if rd(m, sp + 264) == 0 {
                0x1D4D24
            } else {
                rd(m, sp + 264)
            },
            0x1D4D60,
        ] {
            route_or_host(m, h, 0x195C90, &[path, src]);
        }
    }
    for src in [0x1D4D64, rd(m, sp + 4)] {
        route_or_host(m, h, 0x195C90, &[path, src]);
    }
    let mut i = 0;
    while i < (fn_195d90(m, path) as u16) as u32 {
        wb(m, path + i, rb(m, path + i).to_ascii_lowercase());
        i += 1;
    }
    let rc = route_or_host(m, h, 0x1795F0, &[broker, path, 1, obj + 52]);
    if rc >= 0x80000000 {
        wr(m, obj + 52, 0);
        wr(m, obj + 56, 0);
        wr(m, obj + 60, 0);
        return 0;
    }
    wr(m, obj + 60, 0);
    let mut rc = route_or_host(
        m,
        h,
        0x179374,
        &[broker, rd(m, obj + 52), 0, obj + 60, obj + 56],
    );
    if rc < 0x80000000 {
        rc = route_or_host(
            m,
            h,
            rd(m, rd(m, obj + 72) + 28),
            &[
                rd(m, obj + 64),
                rd(m, obj + 68),
                rd(m, obj + 56),
                rd(m, obj + 60),
            ],
        );
        if rc < 0x80000000 {
            return rc;
        }
        route_or_host(m, h, 0x179208, &[broker, rd(m, obj + 52), rd(m, obj + 56)]);
    }
    route_or_host(m, h, 0x179F3C, &[broker, rd(m, obj + 52)]);
    wr(m, obj + 52, 0);
    wr(m, obj + 56, 0);
    wr(m, obj + 60, 0);
    rc
}

fn fn_1ec8b4(m: &mut Memory<'_>, h: &mut impl Calls, obj: u32, tag: u32, oa: u32, ob: u32) -> u32 {
    if fn_176f94(m, obj, tag, 0xF37F, 0x2A4) >= 0x80000000 {
        return rd(m, 0x1EBD1C);
    }
    wb(m, oa, 0);
    wb(m, ob, 0);
    let sp = fn_129778(m, h, 64);
    wh(m, sp + 24, rd(m, 0x1EBD70) as u16);
    let rc = route_or_host(
        m,
        h,
        0x1EA4E0,
        &[obj, tag, 0x1ECADC, 0x1ECAD0, sp + 32, sp + 24, sp + 20],
    );
    if rh(m, sp + 24) == 0 {
        return rc;
    }
    let text = rd(m, rd(m, sp + 32));
    let mut pos = text;
    while rb(m, pos) != 0 && rb(m, pos) != rb(m, sp + 20) {
        pos += 1;
    }
    if rb(m, pos) != 0 {
        wb(m, pos, 0);
    }
    wb(m, ob, 1);
    let mut parts = [0i32; 3];
    let mut at = text;
    let mut valid = true;
    for i in 0..3 {
        parts[i] = tts_core::normalization::strtoul_scan(m.bytes, at, sp + 40, 10, 0) as i32;
        let end = rd(m, sp + 40);
        if parts[i] < if i == 0 { 1 } else { 0 }
            || end == 0
            || (if i < 2 {
                rb(m, end) != b'.'
            } else {
                rb(m, end) != 0 && rb(m, end) != b'.'
            })
        {
            valid = false;
            break;
        }
        at = end + 1;
    }
    let log = rd(m, rd(m, obj) + 16);
    let module = rd(m, 0x1ECAE8);
    if !valid {
        route_or_host(m, h, 0x173DC0, &[log, module, rd(m, 0x1ECAE4), 0]);
        return rd(m, 0x1ECAEC);
    }
    let required = rd(m, rd(m, 0x1ECACC));
    let req = [
        (required & 255) as i32,
        ((required >> 8) & 255) as i32,
        ((required >> 16) & 255) as i32,
    ];
    if req < parts {
        for (i, v) in [req[0] as u32, req[1] as u32, req[2] as u32, text]
            .iter()
            .enumerate()
        {
            wr(m, sp + 4 + i as u32 * 4, *v);
        }
        route_or_host(m, h, 0x1738E8, &[log, module, 5, 0, 0x1ECAF0, sp + 4]);
    } else if parts != [1, 0, 0] {
        wb(m, oa, 1);
    }
    rc
}

fn fn_200a2c(m: &mut Memory<'_>, h: &mut impl Calls, a: u32, b: u32, out: u32) -> u32 {
    if out == 0 {
        return rd(m, 0x200BE4);
    }
    wr(m, out, 0);
    wr(m, out + 4, 0);
    let sp = fn_129778(m, h, 24);
    let rc = fn_19611c(m, a, b, sp);
    if rc >= 0x80000000 {
        return rc;
    }
    let ctx = rd(m, sp);
    let reg = rd(m, ctx + 24);
    let names = [0x200BE8, 0x200BF0, 0x200BFC];
    for i in 0..3 {
        let rc = fn_17b058(m, h, reg, names[i], sp + 4 + i as u32 * 4);
        if rc >= 0x80000000 {
            for name in &names[..i] {
                fn_17b1c4(m, h, reg, *name);
            }
            return rc;
        }
    }
    let obj = fn_196394(m, h, rd(m, ctx + 4), 32);
    if obj == 0 {
        route_or_host(
            m,
            h,
            0x173DC0,
            &[rd(m, ctx + 16), rd(m, 0x200C10), rd(m, 0x200C0C), 0],
        );
        for name in names {
            fn_17b1c4(m, h, reg, name);
        }
        return rd(m, 0x200C14);
    }
    wr(m, obj, ctx);
    wr(m, obj + 4, rd(m, rd(m, sp + 4) + 4));
    for (slot, off) in [(sp + 8, 8), (sp + 12, 20)] {
        for i in 0..3 {
            wr(m, obj + off + i * 4, rd(m, rd(m, slot) + 4 + i * 4));
        }
    }
    wr(m, out, obj);
    wr(m, out + 4, rd(m, 0x200C08));
    0
}

fn token_log(
    m: &mut Memory<'_>,
    h: &mut impl Calls,
    o: u32,
    format: u32,
    level: u32,
    args: &[u32],
) {
    let sp = fn_129778(m, h, 32);
    for (i, v) in args.iter().enumerate() {
        wr(m, sp + i as u32 * 4, *v);
    }
    route_or_host(
        m,
        h,
        0x1738E8,
        &[rd(m, rd(m, o) + 16), rd(m, o + 72), level, 0, format, sp],
    );
}
fn token_data_new(
    m: &mut Memory<'_>,
    h: &mut impl Calls,
    o: u32,
    out: u32,
    size: u32,
    formats: [u32; 2],
    codes: [u32; 2],
) -> u32 {
    if out == 0 {
        route_or_host(
            m,
            h,
            0x173DC0,
            &[rd(m, rd(m, o) + 16), rd(m, o + 72), rd(m, codes[0]), 0],
        );
        return tts_core::normalization::forward_error(7);
    }
    token_log(m, h, o, formats[0], 4, &[]);
    wr(m, out, 0);
    let p = fn_1963b8(m, h, rd(m, rd(m, o) + 4), 1, size);
    if p == 0 {
        route_or_host(
            m,
            h,
            0x173DC0,
            &[rd(m, rd(m, o) + 16), rd(m, o + 72), rd(m, codes[1]), 0],
        );
        return tts_core::normalization::forward_error(10);
    }
    fn_3023b0(m, p, size);
    wr(m, out, p);
    token_log(m, h, o, formats[1], 4, &[0]);
    0
}
fn fn_24ef54(m: &mut Memory<'_>, h: &mut impl Calls, o: u32) -> u32 {
    token_log(m, h, o, 0x24F0EC, 4, &[]);
    let p = fn_1963b8(m, h, rd(m, rd(m, o) + 4), 1, 12);
    if p == 0 {
        route_or_host(
            m,
            h,
            0x173DC0,
            &[rd(m, rd(m, o) + 16), rd(m, o + 72), rd(m, 0x24F120), 0],
        );
        return tts_core::normalization::error_code(10);
    }
    wr(m, o + 56, p);
    let mut rc = 0;
    for (f, out) in [(0x2B645C, p), (0x2B5CEC, p + 4), (0x250918, p + 8)] {
        rc = route_or_host(m, h, f, &[o, out]);
        if rc >= 0x80000000 {
            break;
        }
    }
    if rc >= 0x80000000 {
        token_log(m, h, o, 0x24EF1C, 4, &[]);
        let mut close = 0;
        for f in [0x2B6594, 0x250A3C, 0x2B5E0C] {
            let r = route_or_host(m, h, f, &[o]);
            if r >= 0x80000000 {
                close = r;
            }
        }
        fn_196428(m, h, rd(m, rd(m, o) + 4), p);
        wr(m, o + 56, 0);
        token_log(m, h, o, 0x24EF38, 4, &[close]);
    }
    token_log(m, h, o, 0x24F104, 4, &[rc]);
    rc
}
fn fn_1e4270(m: &mut Memory<'_>, h: &mut impl Calls, a: u32, b: u32, out: u32) -> u32 {
    if out == 0 {
        return rd(m, 0x1E4664);
    }
    wr(m, out, 0);
    wr(m, out + 4, 0);
    let sp = fn_129778(m, h, 64);
    fn_3023b0(m, sp, 64);
    let rc = fn_19611c(m, a, b, sp + 12);
    if rc >= 0x80000000 {
        return rc;
    }
    let ctx = rd(m, sp + 12);
    let heap = rd(m, ctx + 4);
    let reg = rd(m, ctx + 24);
    let log = rd(m, ctx + 16);
    let cat = rd(m, 0x1E4224);
    route_or_host(m, h, 0x1738E8, &[log, cat, 4, 0, 0x1E4668, sp]);
    let pair = fn_1963b8(m, h, heap, 1, 8);
    let mut obj = 0;
    let mut rc = rd(m, 0x1E4200);
    if pair != 0 {
        obj = fn_1963b8(m, h, heap, 1, 244);
        if obj != 0 {
            wr(m, pair, obj);
            wr(m, pair + 4, rd(m, 0x1E4228));
            wr(m, obj, ctx);
            wr(m, obj + 4, a);
            wr(m, obj + 8, b);
            wr(m, obj + 72, cat);
            wb(m, obj + 240, 1);
            wb(m, obj + 241, 1);
            for (key, off, default) in [(0x1E4680, 76, 32), (0x1E4694, 78, 5), (0x1E46B0, 80, 20)] {
                wh(m, obj + off, default);
                if route_or_host(m, h, 0x175944, &[rd(m, ctx + 20), key, sp + 32]) < 0x80000000 {
                    wh(m, obj + off, rd(m, sp + 32) as u16);
                }
            }
            rc = route_or_host(m, h, 0x23FE84, &[a, b, obj + 48]);
            if rc < 0x80000000 {
                rc = route_or_host(m, h, 0x1E3EF8, &[obj, 0x1E46D0]);
            }
            if rc < 0x80000000 {
                for (key, fields) in [
                    (0x1E4230, [24, 28, 32]),
                    (0x1E423C, [36, 40, 44]),
                    (0x1E4248, [12, 0, 0]),
                ] {
                    let r = fn_17b058(m, h, reg, key, sp + 28);
                    if r >= 0x80000000 {
                        if key == 0x1E4248 {
                            rc = r;
                        } else {
                            route_or_host(
                                m,
                                h,
                                0x173DC0,
                                &[
                                    log,
                                    cat,
                                    rd(m, if key == 0x1E4230 { 0x1E46D4 } else { 0x1E46D8 }),
                                    0,
                                ],
                            );
                        }
                        break;
                    }
                    for (i, off) in fields.iter().enumerate() {
                        if *off != 0 {
                            wr(m, obj + off, rd(m, rd(m, sp + 28) + 4 + i as u32 * 4));
                        }
                    }
                }
                if rc < 0x80000000 && rd(m, obj + 12) != 0 {
                    rc = route_or_host(m, h, 0x24EF54, &[a, b, obj]);
                    for f in [0x24DAC8, 0x24D640, 0x2520E4, 0x24D3BC, 0x1E5278] {
                        if rc < 0x80000000 {
                            rc = route_or_host(m, h, f, &[obj]);
                        }
                    }
                    if rc < 0x80000000 {
                        let count = rh(m, obj + 108) as u32;
                        for i in 0..count {
                            token_log(m, h, obj, 0x1E46DC, 5, &[rd(m, rd(m, obj + 112) + i * 4)]);
                        }
                        if count == 0 {
                            route_or_host(m, h, 0x173DC0, &[log, cat, rd(m, 0x1E4764), 0]);
                            rc = rd(m, 0x1E41F8);
                        } else {
                            rc = route_or_host(m, h, 0x2512DC, &[obj, count, rd(m, obj + 112)]);
                            if rc < 0x80000000 {
                                rc = route_or_host(m, h, 0x1E512C, &[obj]);
                            }
                        }
                    }
                }
            }
        }
    } else {
        route_or_host(m, h, 0x173DC0, &[log, cat, rd(m, 0x1E46CC), 0]);
    }
    if rc >= 0x80000000 {
        if pair != 0 {
            route_or_host(m, h, 0x1E4008, &[ctx, pair]);
        }
    } else {
        wr(m, out, pair);
        wr(m, out + 4, rd(m, 0x1E46F8));
    }
    wr(m, sp, rc);
    route_or_host(m, h, 0x1738E8, &[log, cat, 4, 0, 0x1E4768, sp]);
    let _ = obj;
    rc
}

fn fn_24dac8(m: &mut Memory<'_>, h: &mut impl Calls, o: u32) -> u32 {
    token_log(m, h, o, 0x24DCA4, 4, &[]);
    let heap = rd(m, rd(m, o) + 4);
    let p = fn_1963b8(m, h, heap, 1, 48);
    let mut rc = 0;
    if p == 0 {
        route_or_host(
            m,
            h,
            0x173DC0,
            &[rd(m, rd(m, o) + 16), rd(m, o + 72), rd(m, 0x24DCC4), 0],
        );
        return tts_core::normalization::error_code(10);
    }
    wr(m, o + 68, p);
    let cb = rd(m, 0x24DCC8);
    if route_or_host(
        m,
        h,
        0x196A64,
        &[heap, rd(m, cb), rd(m, cb + 4), rd(m, cb + 8), 5, p],
    ) == 0
    {
        rc = tts_core::normalization::error_code(10);
    }
    if rc < 0x80000000 {
        for (off, value) in [(20, 104), (16, 0), (22, 0), (24, 50), (18, 5)] {
            wh(m, p + off, value);
        }
        let arr = fn_1963b8(m, h, heap, 1, 612);
        wr(m, p + 12, arr);
        if arr == 0 {
            rc = tts_core::normalization::error_code(10);
        } else {
            wr(m, arr + 4, 521);
            let data = fn_1963b8(m, h, heap, 1, 521);
            wr(m, arr, data);
            if data == 0 {
                rc = tts_core::normalization::error_code(10);
            } else {
                wh(m, p + 16, 1);
            }
        }
    }
    if rc >= 0x80000000 {
        route_or_host(
            m,
            h,
            0x173DC0,
            &[rd(m, rd(m, o) + 16), rd(m, o + 72), rd(m, 0x24DCC4), 0],
        );
        route_or_host(m, h, 0x24D8FC, &[o]);
    }
    token_log(m, h, o, 0x24DCCC, 4, &[rc]);
    rc
}
fn fn_24d84c(m: &mut Memory<'_>, h: &mut impl Calls, o: u32, out: u32) -> u32 {
    let cb = rd(m, 0x24D8F4);
    let r = route_or_host(
        m,
        h,
        0x1964E0,
        &[
            rd(m, rd(m, o) + 4),
            0,
            rd(m, cb),
            rd(m, cb + 4),
            rd(m, cb + 8),
            rd(m, cb + 12),
            rd(m, cb + 16),
            out,
        ],
    );
    if r >= 0x80000000 {
        route_or_host(
            m,
            h,
            0x173DC0,
            &[rd(m, rd(m, o) + 16), rd(m, o + 72), rd(m, 0x24D8F8), 0],
        );
    }
    r
}
fn fn_24d3bc(m: &mut Memory<'_>, h: &mut impl Calls, o: u32) -> u32 {
    let sp = fn_129778(m, h, 8);
    token_log(m, h, o, 0x24D4BC, 4, &[]);
    let mut rc = fn_24d84c(m, h, o, sp);
    if rc < 0x80000000 {
        let table = rd(m, 0x24D4B8);
        for i in 0..2 {
            rc = route_or_host(
                m,
                h,
                0x19668C,
                &[rd(m, sp), rd(m, table + i * 8), rd(m, table + i * 8 + 4)],
            );
            if rc >= 0x80000000 {
                break;
            }
        }
        if rc < 0x80000000 {
            wr(m, rd(m, o + 60) + 4, rd(m, sp));
        } else {
            route_or_host(
                m,
                h,
                0x173DC0,
                &[rd(m, rd(m, o) + 16), rd(m, o + 72), rd(m, 0x24D4DC), 0],
            );
            route_or_host(m, h, 0x19665C, &[rd(m, sp)]);
        }
    }
    token_log(m, h, o, 0x24D4E0, 4, &[rc]);
    rc
}
fn fn_2520e4(m: &mut Memory<'_>, h: &mut impl Calls, o: u32) -> u32 {
    token_log(m, h, o, 0x2523E8, 4, &[]);
    let heap = rd(m, rd(m, o) + 4);
    let tok = fn_1963b8(m, h, heap, 1, 4);
    if tok == 0 {
        route_or_host(
            m,
            h,
            0x173DC0,
            &[rd(m, rd(m, o) + 16), rd(m, o + 72), rd(m, 0x252404), 0],
        );
        return tts_core::normalization::error_code(10);
    }
    wr(m, o + 64, tok);
    let mut targets = [tok, 0, 0, 0, 0];
    let mut rc = 0;
    for (i, size) in [36, 20, 32, 4, 92].iter().enumerate() {
        let p = fn_1963b8(m, h, heap, 1, *size);
        let dest = match i {
            0 => tok,
            1 => rd(m, tok),
            2 => rd(m, tok) + 4,
            3 => rd(m, rd(m, tok) + 4) + 4,
            _ => rd(m, rd(m, tok) + 4),
        };
        wr(m, dest, p);
        targets[i] = p;
        if p == 0 {
            rc = tts_core::normalization::error_code(10);
            break;
        }
    }
    if rc < 0x80000000 {
        let a = targets[0];
        let b = targets[1];
        let c = targets[2];
        wr(m, b, rd(m, b) | 4);
        wr(m, b + 12, c);
        let sp = fn_129778(m, h, 8);
        token_log(m, h, o, 0x252408, 4, &[]);
        rc = fn_24d84c(m, h, o, sp);
        if rc < 0x80000000 {
            let table = rd(m, 0x252430);
            for i in 0..8 {
                rc = route_or_host(
                    m,
                    h,
                    0x19668C,
                    &[rd(m, sp), rd(m, table + i * 8), rd(m, table + i * 8 + 4)],
                );
                if rc >= 0x80000000 {
                    route_or_host(
                        m,
                        h,
                        0x173DC0,
                        &[rd(m, rd(m, o) + 16), rd(m, o + 72), rd(m, 0x252434), 0],
                    );
                    break;
                }
            }
            if rc < 0x80000000 {
                wr(m, rd(m, o + 60), rd(m, sp));
            } else {
                route_or_host(m, h, 0x19665C, &[rd(m, sp)]);
            }
        }
        let _ = a;
        token_log(m, h, o, 0x252438, 4, &[rc]);
    } else {
        route_or_host(
            m,
            h,
            0x173DC0,
            &[rd(m, rd(m, o) + 16), rd(m, o + 72), rd(m, 0x252404), 0],
        );
    }
    if rc >= 0x80000000 {
        route_or_host(m, h, 0x25251C, &[o]);
    }
    token_log(m, h, o, 0x252460, 4, &[rc]);
    rc
}

fn token_config(m: &Memory<'_>) -> tts_core::boot_port_2::ConfigStrings {
    tts_core::boot_port_2::ConfigStrings {
        langcode: rom_text(m, b"langcode"),
        voice: rom_text(m, b"voice"),
        fecfg: rom_text(m, b"fecfg"),
        prefix: rom_text(m, b"fedataprefix"),
        packaging: rom_text(m, b"fedatapackaging"),
        clc: rom_text(m, b"clc"),
        clc_slash: rom_text(m, b"clc/"),
        slash: rom_text(m, b"/"),
        colon: rom_text(m, b":"),
        sysdct_slash: rom_text(m, b"sysdct/"),
    }
}
impl<H: Calls> tts_core::boot_port_2::Parts for RouteHost<'_, H> {
    fn get_part(
        &mut self,
        m: &mut Memory<'_>,
        f: u32,
        pair: [u32; 2],
        tn: bool,
        prior: tts_core::boot_port_2::Part,
    ) -> tts_core::boot_port_2::Part {
        let sp = fn_129778(m, self.inner, 16);
        wr(m, sp, prior.result);
        wh(m, sp + 4, prior.tag);
        wb(m, sp + 8, prior.separator);
        let rc = route_or_host(
            m,
            self.inner,
            f,
            &[
                pair[0],
                pair[1],
                0x1E5264,
                if tn { 0x1E526C } else { 0x1E5258 },
                sp,
                sp + 4,
                sp + 8,
            ],
        );
        tts_core::boot_port_2::Part {
            status: rc,
            result: rd(m, sp),
            tag: rh(m, sp + 4) as u16,
            separator: rb(m, sp + 8),
        }
    }
}

impl<H: Calls> tts_core::frontend::BlockHeap for RouteHost<'_, H> {
    fn alloc_block(&mut self, _: u32, _: u32) -> u32 {
        panic!("reader heap call lacks memory")
    }
    fn realloc_block(&mut self, _: u32, _: u32, _: u32) -> u32 {
        panic!("reader realloc call lacks memory")
    }
    fn alloc_cells(&mut self, _: u32, _: u32, _: u32) -> u32 {
        panic!("reader cells call lacks memory")
    }
}
impl<H: Calls> tts_core::render_part_a::ReaderHost for RouteHost<'_, H> {
    fn reader_call(&mut self, m: &mut Memory<'_>, f: u32, a: &[u32]) -> Option<u32> {
        Some(route_or_host(m, self.inner, f, a))
    }
    fn array_release_method(&mut self, _: u32, _: u32) {
        panic!("reader release call lacks memory")
    }
    fn array_destructor(&mut self, _: u32, _: u32, _: u32) {
        panic!("reader destructor call lacks memory")
    }
    fn array_destructor_m(&mut self, bytes: &mut [u8], f: u32, p: u32, c: u32) {
        route_or_host(&mut Memory { base: 0, bytes }, self.inner, f, &[p, c]);
    }
    fn array_initializer(&mut self, bytes: &mut [u8], f: u32, p: u32, c: u32, d: u32) -> u32 {
        route_or_host(&mut Memory { base: 0, bytes }, self.inner, f, &[p, c, d])
    }
}
fn token_block_env(m: &mut Memory<'_>, h: &mut impl Calls) -> tts_core::boot_port_4b::Environment {
    let scratch = fn_129778(m, h, 1024);
    let texts = [
        b"Unmap %p (%u - %u)".as_slice(),
        b"Map offset %u, %u bytes",
        b"ssftriff_reader_ObjClose : Begin (%p)",
        b"ssftriff_reader_ObjClose : End (%x)",
        b"ssftriff_reader_CloseChunk : Begin (%p, %s)",
        b"ssftriff_reader_CloseChunk : End (%x)",
        b"ssftriff_reader_GetChunkData : Begin (%p, %u, %p)",
        b"ssftriff_reader_GetChunkData : End (%x, %p)",
        b"ssftriff_reader_OpenChunk : Begin (%p)",
        b"Chunk open failed: no more chunks",
        b"ssftriff_reader_OpenChunk : End (%x, %s, %u, %p)",
        b"ssftriff_reader_FindChunk : Begin (%p, %s, %d)",
        b"ssftriff_reader_FindChunk : End (%x, %u, %p)",
        b"",
        b"",
        b"",
    ];
    let reader = tts_core::render_part_a::ReaderRuntime {
        category: 0x914878,
        messages: texts.map(|t| rom_text(m, t)),
        pair_format: rom_text(m, b"%s%s"),
        triple_format: rom_text(m, b"%s%s%s%s%s%u"),
        four_format: rom_text(m, b"%s%s%s%s"),
        file_key: rom_text(m, b"file"),
        chunk_key: rom_text(m, b"chunk"),
        parent_key: rom_text(m, b"parentChunk"),
        offset_key: rom_text(m, b"fileOffset"),
        scratch: scratch + 256,
    };
    let pools = [
        0x2B6AAC, 0x2B6AB0, 0x2B6544, 0x2B653C, 0x2B6548, 0x2B6AB4, 0x2B6AB8, 0x2B6ABC, 0x2B6AC4,
        0x2B6AC8, 0x2B6540, 0x2B6AC0,
    ];
    tts_core::boot_port_4b::Environment {
        reader,
        tags: [b"HEAD".as_slice(), b"REMP", b"TDAT", b"DDAT"].map(|t| rom_text(m, t)),
        node_tags: pools.map(|p| rd(m, p)),
        sub_tag: 0x914AC0,
        detach_messages: [
            rom_text(m, b"ssftriff_reader_DetachChunkData : Begin (%p)"),
            rom_text(m, b"ssftriff_reader_DetachChunkData : End (%x, %p)"),
        ],
        close_messages: [rom_text(m, b""), rom_text(m, b"")],
        scratch,
    }
}
fn token_error_cold(
    m: &mut Memory<'_>,
    h: &mut impl Calls,
    o: u32,
    code: u32,
    format: u32,
    args: &[u32],
) {
    let sp = fn_129778(m, h, 32);
    for (i, v) in args.iter().enumerate() {
        wr(m, sp + i as u32 * 4, *v);
    }
    route_or_host(
        m,
        h,
        0x173DC0,
        &[rd(m, rd(m, o) + 16), rd(m, o + 72), code, format, sp],
    );
}
fn fn_2512dc(m: &mut Memory<'_>, h: &mut impl Calls, o: u32, tokens: u32) -> u32 {
    let sp = fn_129778(m, h, 272);
    fn_3023b0(m, sp, 272);
    let buf = sp;
    let lst = rd(m, rd(m, o + 56) + 8);
    let ctx = rd(m, o);
    let heap = rd(m, ctx + 4);
    let params = rd(m, ctx + 20);
    let idx = rb(m, o + 116) as u32;
    if lst == 0 || rh(m, lst) != 0 {
        token_error_cold(m, h, o, if lst == 0 { 0x5DF1 } else { 0x5DF7 }, 0, &[]);
        return tts_core::normalization::forward_error(7);
    }
    if (rh(m, o + 108) as u32) < idx {
        token_error_cold(
            m,
            h,
            o,
            0x5DF8,
            rom_text(m, b"%s%d%s%d"),
            &[
                rom_text(m, b"invalid language identifier index"),
                idx,
                rom_text(m, b"max="),
                rh(m, o + 108) as u32,
            ],
        );
        return tts_core::normalization::forward_error(0);
    }
    let mut parameter = |m: &mut Memory<'_>, key: &[u8], cell: u32| {
        wr(m, cell, 0);
        let r = fn_1756d0(m, h, params, rom_text(m, key), cell, 0);
        (r, rd(m, cell))
    };
    let (r, lang) = parameter(m, b"langcode", sp + 128);
    let lang = if r < 0x80000000 {
        lang
    } else {
        rom_text(m, b"")
    };
    let (r, fecfg) = parameter(m, b"fecfg", sp + 132);
    let optional = r < 0x80000000;
    drop(parameter);
    let mut opened = false;
    let mut rc = 0;
    for mode in if optional { vec![2, 0] } else { vec![0] } {
        wb(m, buf, 0);
        wr(m, sp + 136, 0);
        let r = fn_1756d0(m, h, params, rom_text(m, b"fedataprefix"), sp + 136, 0);
        let prefix = rd(m, sp + 136);
        if r < 0x80000000 && prefix != 0 && rb(m, prefix) != 0 {
            fn_195cd0(m, buf, prefix);
        }
        wr(m, sp + 140, 0);
        let r = fn_1756d0(m, h, params, rom_text(m, b"fedatapackaging"), sp + 140, 0);
        let packaging = rd(m, sp + 140);
        if r < 0x80000000 && packaging != 0 && fn_195d58(m, packaging, rom_text(m, b"clc")) == 0 {
            for src in [
                rom_text(m, b"clc/"),
                lang,
                rom_text(m, b"/"),
                fecfg,
                rom_text(m, b":"),
            ] {
                route_or_host(m, h, 0x195C90, &[buf, src]);
            }
        }
        for src in [rom_text(m, b"tokentn/"), rd(m, tokens + idx * 4)] {
            route_or_host(m, h, 0x195C90, &[buf, src]);
        }
        if mode == 2 {
            for src in [rom_text(m, b"/"), fecfg] {
                route_or_host(m, h, 0x195C90, &[buf, src]);
            }
        }
        rc = fn_242fd0(
            m,
            h,
            [
                rd(m, o + 4),
                rd(m, o + 8),
                mode,
                buf,
                0,
                0,
                0x914ABB,
                rd(m, 0x914AB8),
                sp + 144,
            ],
        );
        if rc < 0x80000000 {
            opened = true;
            break;
        }
    }
    if !opened {
        token_error_cold(
            m,
            h,
            o,
            0x5DF9,
            rom_text(m, b"%s%s"),
            &[rom_text(m, b"Missing broker header file"), buf],
        );
        return tts_core::normalization::forward_error(3);
    }
    let reader = rd(m, sp + 144);
    let mut flags = [0; 2];
    let result = (|| -> Result<u32, u32> {
        let r = route_or_host(
            m,
            h,
            0x242C9C,
            &[reader, rom_text(m, b"CFIG"), 0, sp + 148, sp + 152],
        );
        if r >= 0x80000000 && r.wrapping_shl(19) != 0xA00000 {
            return Err(r);
        }
        if r.wrapping_shl(19) != 0xA00000 {
            let data = rd(m, sp + 152);
            for i in 0..2 {
                if rh(m, data) > i as u16 && rh(m, data + 2 + i as u32 * 2) == 1 {
                    flags[i] = 1;
                }
            }
            let r = route_or_host(m, h, 0x2422AC, &[reader]);
            if r >= 0x80000000 {
                return Err(r);
            }
        }
        let r = route_or_host(
            m,
            h,
            0x242C9C,
            &[reader, rom_text(m, b"FILE"), 0, sp + 148, sp + 152],
        );
        if r >= 0x80000000 {
            return Err(r);
        }
        wh(m, lst, rh(m, rd(m, sp + 152)) as u16);
        let r = route_or_host(m, h, 0x2422AC, &[reader]);
        if r >= 0x80000000 {
            return Err(r);
        }
        let count = rh(m, lst) as u32;
        let arr = fn_1963b8(m, h, heap, 1, 1 + count * 4);
        wr(m, lst + 4, arr);
        if arr == 0 {
            return Err(tts_core::normalization::forward_error(10));
        }
        for i in 0..count {
            let el = fn_1963b8(m, h, heap, 1, 24);
            wr(m, arr + i * 4, el);
            if el == 0 {
                return Err(tts_core::normalization::forward_error(10));
            }
            let r = route_or_host(m, h, 0x24F570, &[o, reader, el, flags[0], flags[1]]);
            if r >= 0x80000000 {
                route_or_host(m, h, 0x24F124, &[o, el]);
                wr(m, arr + i * 4, 0);
                return Err(r);
            }
            token_log(m, h, o, rom_text(m, b"loaded data block %d"), 4, &[i]);
        }
        Ok(0)
    })();
    route_or_host(m, h, 0x2421B0, &[reader]);
    if let Err(r) = result {
        token_error_cold(m, h, o, 0x5DF4, 0, &[]);
        return r;
    }
    finish_token_blocks(m, h, o, lst, heap)
}

fn finish_token_blocks(m: &mut Memory<'_>, h: &mut impl Calls, o: u32, lst: u32, heap: u32) -> u32 {
    let cnt = rh(m, lst) as u32;
    let rc = 0;
    let arr = rd(m, lst + 4);
    for i in 0..cnt {
        let el = rd(m, arr + 4 * i);
        let sub_list = rd(m, el + 8);
        if el == 0 || sub_list == 0 {
            continue;
        }
        let ecount = rh(m, sub_list + 0xc) as u32;
        let earr = rd(m, sub_list + 8);
        for j in 0..ecount {
            let sub = earr + 0x1c * j;
            if rb(m, sub + 0x1b) == 1 {
                let name = rd(m, sub + 0x14).wrapping_add(rh(m, sub) as u32);
                let first = rd(m, o + 0x54);
                if first != 0 {
                    if tts_core::normalization::strcmp_at(m.bytes, first, name) != 0 {
                        token_error_cold(
                            m,
                            h,
                            o,
                            0x5df5,
                            rom_text(m, b"%s%s%s%s"),
                            &[
                                rom_text(m, b"first assigned"),
                                first,
                                rom_text(m, b"clashes with"),
                                name,
                            ],
                        );
                        let rc = tts_core::normalization::forward_error(0);

                        return rc;
                    }
                } else {
                    let n = tts_core::boot_port_1::fn_195d90(m, name) + 1;
                    let p = fn_1963b8(m, h, heap, 1, n);
                    wr(m, o + 0x54, p);
                    if p == 0 {
                        token_error_cold(m, h, o, 0x5df0, 0, &[]);
                        let rc = tts_core::normalization::forward_error(10);

                        return rc;
                    }
                    tts_core::frontend::copy_str(m.bytes, p, name);
                    wh(m, o + 0x58, i as u16);
                    wh(m, o + 0x5a, j as u16);
                }
            }
            if rb(m, sub + 6) == 1 {
                let gcount = rh(m, sub_list + 0x12) as u32;
                let garr = rd(m, sub_list + 0x10);
                let limit = rh(m, sub_list + 0x14) as u32;
                for k in 0..gcount {
                    let g = garr + 8 * k;
                    let entries = rd(m, g);
                    let count = rh(m, g + 4) as u32;
                    for n in 0..count {
                        let mut found = false;
                        for w in 0..cnt {
                            let e2 = rd(m, arr + 4 * w);
                            let kind = rb(m, e2);
                            if kind == 1 || kind == 4 {
                                continue;
                            }
                            if limit == 0 {
                                continue;
                            }
                            let v = rh(m, entries + 6 * n) as u32;
                            if limit <= v {
                                continue;
                            }
                            let list = rd(m, sub_list + 0x18);
                            let target = rd(m, list + 4 * v);
                            let name = rd(m, rd(m, e2 + 4));
                            if tts_core::normalization::strcmp_at(m.bytes, name, target) == 0 {
                                wh(m, entries + 6 * n + 2, w as u16);
                                found = true;
                                break;
                            }
                        }
                        if !found {
                            token_error_cold(m, h, o, 0x5ded, 0, &[]);

                            return rc;
                        }
                    }
                }
            }
        }
        for w in 0..cnt {
            let e2 = rd(m, arr + 4 * w);
            let rec_list = rd(m, e2 + 0xc);
            if e2 == 0 || rec_list == 0 {
                continue;
            }
            let recs = rd(m, rec_list + 4);
            let count = rh(m, rec_list + 8) as u32;
            for k in 0..count {
                let rec = recs + 8 * k;
                let name = rd(m, rec);
                for j in 0..ecount {
                    let sub = earr + 0x1c * j;
                    let other = rd(m, sub + 0x14).wrapping_add(rh(m, sub) as u32);
                    if tts_core::normalization::strcmp_at(m.bytes, name, other) == 0 {
                        wh(m, rec + 6, i as u16);
                    }
                }
            }
        }
    }
    for i in 0..cnt {
        let el = rd(m, arr + 4 * i);
        let sub_list = rd(m, el + 8);
        if el == 0 || sub_list == 0 {
            continue;
        }
        let ecount = rh(m, sub_list + 0xc) as u32;
        let earr = rd(m, sub_list + 8);
        for j in 0..ecount {
            let sub = earr + 0x1c * j;
            if rh(m, sub + 0x10) != 0 && (rb(m, sub + 6) == 0 || rh(m, sub_list + 0x12) == 0) {
                token_error_cold(m, h, o, 0x5dfe, 0, &[]);
            }
        }
    }

    if rd(m, o + 84) == 0 {
        token_error_cold(m, h, o, 0x5DF6, 0, &[]);
        return tts_core::normalization::forward_error(0);
    }
    rc
}

fn fn_1d8a20(m: &mut Memory<'_>, h: &mut impl Calls, a: u32, b: u32, out: u32) -> u32 {
    if out == 0 {
        return rd(m, 0x1D8D9C);
    }
    wr(m, out, 0);
    wr(m, out + 4, 0);
    let sp = fn_129778(m, h, 64);
    fn_3023b0(m, sp, 64);
    let rc = fn_19611c(m, a, b, sp + 16);
    if rc >= 0x80000000 {
        return rc;
    }
    let ctx = rd(m, sp + 16);
    let reg = rd(m, ctx + 24);
    let names = [0x1D8A00, 0x1D8A08, 0x1D8A14];
    for i in 0..3 {
        let r = fn_17b058(m, h, reg, names[i], sp + 40 + i as u32 * 4);
        if r >= 0x80000000 {
            for name in &names[..i] {
                fn_17b1c4(m, h, reg, *name);
            }
            return r;
        }
    }
    let o = fn_196394(m, h, rd(m, ctx + 4), 56);
    if o == 0 {
        route_or_host(
            m,
            h,
            0x173DC0,
            &[rd(m, ctx + 16), rd(m, 0x1D8DB4), rd(m, 0x1D8DB0), 0],
        );
        for name in names {
            fn_17b1c4(m, h, reg, name);
        }
        return rd(m, 0x1D8DB8);
    }
    wr(m, out, o);
    wr(m, out + 4, rd(m, 0x1D89F8));
    wr(m, o, ctx);
    wr(m, o + 4, rd(m, rd(m, sp + 40) + 4));
    for (slot, fields) in [(sp + 44, [16, 8, 12]), (sp + 48, [28, 20, 24])] {
        for (i, off) in fields.iter().enumerate() {
            wr(m, o + off, rd(m, rd(m, slot) + 4 + i as u32 * 4));
        }
    }
    for off in [32, 36, 40, 44] {
        wr(m, o + off, 0);
    }
    wh(m, o + 48, 0);
    let mut rc = route_or_host(
        m,
        h,
        rd(m, rd(m, o + 28) + 76),
        &[rd(m, o + 20), rd(m, o + 24), sp + 24, sp + 28],
    );
    if rc < 0x80000000 {
        wb(
            m,
            o + 50,
            u8::from(rb(m, sp + 28) == 1 && rb(m, sp + 24) == 1),
        );
        wb(m, o + 51, 0);
        wh(m, sp + 32, u16::MAX);
        rc = route_or_host(
            m,
            h,
            rd(m, rd(m, o + 28) + 28),
            &[
                rd(m, o + 20),
                rd(m, o + 24),
                0x1D8DA8,
                0x1D8DA0,
                sp + 36,
                sp + 32,
                sp + 52,
            ],
        );
        if rc < 0x80000000 {
            let r = fn_1756d0(m, h, rd(m, ctx + 20), 0x1D8DA8, sp + 56, 0);
            if r < 0x80000000
                && rh(m, sp + 32) == 0
                && (fn_1769d0(m, rd(m, sp + 56), 0x1D8DBC) == 0
                    || fn_1769d0(m, rd(m, sp + 56), 0x1D8DC4) == 0)
            {
                wb(m, o + 51, 1);
            }
            wh(m, sp + 32, u16::MAX);
            rc = route_or_host(
                m,
                h,
                rd(m, rd(m, o + 28) + 28),
                &[
                    rd(m, o + 20),
                    rd(m, o + 24),
                    0x1D8DA8,
                    0x1D8DCC,
                    sp + 36,
                    sp + 32,
                    sp + 52,
                ],
            );
            if rc < 0x80000000 {
                wr(m, o + 52, 0);
                if rh(m, sp + 32) != 0 {
                    let text = rd(m, rd(m, sp + 36));
                    let len = fn_195d90(m, text);
                    let p = fn_196394(m, h, rd(m, ctx + 4), len + 1);
                    wr(m, o + 52, p);
                    if p == 0 {
                        route_or_host(
                            m,
                            h,
                            0x173DC0,
                            &[rd(m, ctx + 16), rd(m, 0x1D8DB4), rd(m, 0x1D8DD4), 0],
                        );
                        rc = rd(m, 0x1D8DB8);
                    } else {
                        fn_195cd0(m, p, text);
                        wb(m, p + len - 1, b'|');
                    }
                }
            }
        }
    }
    if rc >= 0x80000000 {
        route_or_host(m, h, 0x1D88F4, &[o, rd(m, out + 4)]);
        wr(m, out, 0);
        wr(m, out + 4, 0);
    }
    rc
}

impl<H: Calls> tts_core::boot_port_5::ConfigHost for RouteHost<'_, H> {
    fn string(
        &mut self,
        m: &mut Memory<'_>,
        f: u32,
        pair: [u32; 2],
        section: u32,
        key: u32,
        value: &mut u32,
        len: &mut u16,
        sep: &mut u8,
    ) -> u32 {
        let sp = fn_129778(m, self.inner, 12);
        wr(m, sp, *value);
        wh(m, sp + 4, *len);
        wb(m, sp + 6, *sep);
        let rc = route_or_host(
            m,
            self.inner,
            f,
            &[pair[0], pair[1], section, key, sp, sp + 4, sp + 6],
        );
        *value = rd(m, sp);
        *len = rh(m, sp + 4);
        *sep = rb(m, sp + 6);
        rc
    }
}
fn fn_1f87c4(m: &mut Memory<'_>, h: &mut impl Calls, a: u32, b: u32, out: u32) -> u32 {
    if out == 0 {
        return rd(m, 0x1F8B20);
    }
    wr(m, out, 0);
    wr(m, out + 4, 0);
    let sp = fn_129778(m, h, 128);
    fn_3023b0(m, sp, 128);
    let r = fn_19611c(m, a, b, sp);
    if r >= 0x80000000 {
        return r;
    }
    let ctx = rd(m, sp);
    let heap = rd(m, ctx + 4);
    let reg = rd(m, ctx + 24);
    let names = [0x1F8B24, 0x1F8B2C, 0x1F8B38, 0x1F8B44, 0x1F8B48];
    for i in 0..5 {
        let r = fn_17b058(m, h, reg, names[i], sp + 4 + i as u32 * 4);
        if r >= 0x80000000 {
            for name in &names[..i] {
                fn_17b1c4(m, h, reg, *name);
            }
            return r;
        }
    }
    let o = fn_1963b8(m, h, heap, 1, rd(m, 0x1F809C));
    if o == 0 {
        route_or_host(
            m,
            h,
            0x173DC0,
            &[rd(m, ctx + 16), rd(m, 0x1F8B10), rd(m, 0x1F8B18), 0],
        );
        for i in [0, 1, 2, 4, 3] {
            fn_17b1c4(m, h, reg, names[i]);
        }
        return rd(m, 0x1F8B1C);
    }
    wr(m, out, o);
    wr(m, out + 4, rd(m, 0x1F80A0));
    wr(m, o, ctx);
    wr(m, o + 4, a);
    wr(m, o + 8, b);
    wr(m, o + 12, rd(m, rd(m, sp + 4) + 4));
    for i in 1..5 {
        let info = rd(m, sp + 4 + i * 4);
        for (j, off) in [12 + i * 12, 4 + i * 12, 8 + i * 12].iter().enumerate() {
            wr(m, o + off, rd(m, info + 4 + j as u32 * 4));
        }
    }
    let mut rc = route_or_host(
        m,
        h,
        rd(m, rd(m, o + 24) + 76),
        &[rd(m, o + 16), rd(m, o + 20), sp + 24, sp + 28],
    );
    if rc >= 0x80000000 {
        for i in [0, 1, 2, 4, 3] {
            fn_17b1c4(m, h, reg, names[i]);
        }
        return rc;
    }
    wb(
        m,
        o + 0x173,
        u8::from(rb(m, sp + 24) == 1 && rb(m, sp + 28) == 1),
    );
    wb(m, o + 0x172, 1);
    wr(m, o + 0x174, 0);
    rc = fn_1756d0(m, h, rd(m, ctx + 20), 0x1F8F78, o + 0x2AC, 0);
    if rc < 0x80000000 {
        for off in [64, 68, 72, 76, 84, 88, 92, 96, 100] {
            let p = fn_196394(m, h, heap, 128);
            wr(m, o + off, p);
            if p == 0 {
                rc = rd(m, 0x1F8B1C);
                break;
            }
        }
        if rc < 0x80000000 {
            fn_195cd0(m, o + 0x72, 0x1F8F84);
            let keys = [
                b"mdeminwrdlen_".as_slice(),
                b"mdemaxwrdlen_",
                b"mdemaxnrmorph_",
                b"mdefirstletter_",
                b"mdetype_",
                b"mdeemptyafx_",
                b"mdenondctdps_",
                b"mdedps_",
            ]
            .map(|s| rom_text(m, s));
            let table = std::array::from_fn(|i| rh(m, rd(m, 0x17C8F8) + i as u32 * 2));
            let work = fn_129778(m, h, 256);
            let mut host = RouteHost { inner: h };
            rc = tts_core::boot_port_5::fn_275204(
                m,
                &mut host,
                o,
                0x1F8F84,
                keys,
                rom_text(m, b"fecfg"),
                work,
                &table,
            );
        }
        if rc < 0x80000000 {
            wh(m, sp + 36, u16::MAX);
            rc = route_or_host(
                m,
                h,
                rd(m, rd(m, o + 24) + 28),
                &[
                    rd(m, o + 16),
                    rd(m, o + 20),
                    rd(m, 0x1F8F98),
                    0x1F8F8C,
                    sp + 32,
                    sp + 36,
                    sp + 40,
                ],
            );
            if rc < 0x80000000 && rh(m, sp + 36) != 0 {
                let text = rd(m, rd(m, sp + 32));
                let mut at = text;
                while rb(m, at) != 0 && rb(m, at) != rb(m, sp + 40) {
                    at += 1;
                }
                if rb(m, at) != 0 {
                    wb(m, at, 0);
                }
                wb(m, o + 0x172, parse_decimal(&m.bytes[text as usize..]) as u8);
            }
        }
        if rc < 0x80000000 {
            rc = route_or_host(
                m,
                h,
                0x1964E0,
                &[
                    heap,
                    0,
                    rd(m, 0x1F8F9C),
                    rd(m, 0x1F8FA0),
                    rd(m, 0x1F8FAC),
                    rd(m, 0x1F8FA4),
                    rd(m, 0x1F8FA8),
                    o + 0x174,
                ],
            );
        }
        if rc < 0x80000000 {
            for i in 0..4 {
                wr(m, o + 0x180 + i * 4, rd(m, 0x1F8FB0 + i * 4));
            }
            rc = route_or_host(m, h, 0x1B20A4, &[o + 0x190, o + 0x180, heap]);
            if rc != 0 {
                rc = rc.wrapping_add(0x1EA0) | 0x89B02000;
            }
        }
        if rc < 0x80000000 {
            rc = route_or_host(m, h, 0x1F80D4, &[o]);
        }
        if rc < 0x80000000 {
            wh(m, sp + 36, u16::MAX);
            rc = route_or_host(
                m,
                h,
                rd(m, rd(m, o + 24) + 28),
                &[
                    rd(m, o + 16),
                    rd(m, o + 20),
                    0x1F8FC8,
                    0x1F8FC0,
                    sp + 32,
                    sp + 36,
                    sp + 40,
                ],
            );
            if rc < 0x80000000 {
                wr(m, o + 0x2B0, 0);
                if rh(m, sp + 36) != 0 {
                    let text = rd(m, rd(m, sp + 32));
                    let len = fn_195d90(m, text);
                    let p = fn_196394(m, h, heap, len + 1);
                    wr(m, o + 0x2B0, p);
                    if p == 0 {
                        rc = rd(m, 0x1F8B1C);
                    } else {
                        fn_195cd0(m, p, text);
                        wb(m, p + len - 1, b'|');
                    }
                }
            }
        }
        if rc < 0x80000000 {
            wb(m, o + 0x2B4, 1);
            fn_195cd0(m, sp + 44, 0x1F8FD0);
            for c in 33..176 {
                if rb(m, o + 0x2B4) == 0 {
                    break;
                }
                wb(m, sp + 47, c as u8);
                wh(m, sp + 36, u16::MAX);
                rc = route_or_host(
                    m,
                    h,
                    rd(m, rd(m, o + 24) + 28),
                    &[
                        rd(m, o + 16),
                        rd(m, o + 20),
                        0x1F8FC8,
                        sp + 44,
                        sp + 32,
                        sp + 36,
                        sp + 40,
                    ],
                );
                if rc >= 0x80000000 {
                    break;
                }
                if rh(m, sp + 36) == 0 {
                    wb(m, o + 0x2B4, 0);
                } else {
                    let dst = o + 0x2B5 + c * 3;
                    fn_195cd0(m, dst, 0x1F8F80);
                    let text = rd(m, rd(m, sp + 32));
                    let n = fn_195d90(m, text).wrapping_sub(1);
                    route_or_host(m, h, 0x195DAC, &[dst, text, n]);
                }
            }
        }
    }
    if rc >= 0x80000000 {
        route_or_host(m, h, 0x1F7ED0, &[o, rd(m, out + 4)]);
        wr(m, out, 0);
        wr(m, out + 4, 0);
    }
    rc
}

fn fn_1f80d4(m: &mut Memory<'_>, h: &mut impl Calls, o: u32) -> u32 {
    let sp = fn_129778(m, h, 640);
    fn_3023b0(m, sp, 640);
    let ctx = rd(m, o);
    let params = rd(m, ctx + 20);
    let path = sp;
    let buf = sp + 256;
    let value = sp + 512;
    let count = sp + 516;
    let sep = sp + 520;
    for (i, (key, offset, invert)) in [
        (b"dtlhp2lhtts".as_slice(), 0x178, false),
        (b"phonlhp2lhtts".as_slice(), 0x179, true),
        (b"extramaintrans".as_slice(), 0x17D, false),
        (b"lidenx".as_slice(), 0x1A4, false),
        (b"lidenxchars".as_slice(), 0x1A6, false),
        (b"dpsmultitrans".as_slice(), 0x2A6, false),
    ]
    .iter()
    .enumerate()
    {
        if i == 3 {
            let r = fn_1756d0(m, h, params, rom_text(m, b"mlset"), sp + 524, 0);
            let mut flag = false;
            if r < 0x80000000 && fn_195d90(m, rd(m, sp + 524)) > 3 {
                flag =
                    fn_1756d0(m, h, params, rom_text(m, b"extclccfg"), sp + 528, 0) >= 0x80000000;
            }
            wb(m, o + 0x2A8, u8::from(flag));
        }
        wh(m, count, u16::MAX);
        let rc = route_or_host(
            m,
            h,
            rd(m, rd(m, o + 24) + 28),
            &[
                rd(m, o + 16),
                rd(m, o + 20),
                0x1F84F0,
                rom_text(m, key),
                value,
                count,
                sep,
            ],
        );
        if rc >= 0x80000000 {
            return rc;
        }
        let mut flag = rh(m, count) != 0;
        if i == 2 && !flag {
            flag = route_or_host(m, h, 0x1960A8, &[0x1F8518, rd(m, o + 0x2AC)]) != 0;
        }
        if i < 4 {
            wb(m, o + offset, u8::from(flag ^ invert));
            if i == 3 {
                wb(m, o + 0x1A5, 0);
            }
        } else {
            let text = if flag {
                let text = rd(m, rd(m, value));
                let mut p = text;
                while rb(m, p) != 0 && rb(m, p) != rb(m, sep) {
                    p += 1;
                }
                if rb(m, p) != 0 {
                    wb(m, p, 0);
                }
                text
            } else {
                rom_text(m, b"")
            };
            if i == 4 {
                fn_195cd0(m, o + offset, text);
            } else {
                wb(
                    m,
                    o + offset,
                    if flag {
                        parse_decimal(&m.bytes[text as usize..]) as u8
                    } else {
                        0
                    },
                );
            }
        }
    }
    for (key, slot) in [
        (b"langcode".as_slice(), sp + 532),
        (b"fecfg".as_slice(), sp + 536),
    ] {
        let r = fn_1756d0(m, h, params, rom_text(m, key), slot, 0);
        if r >= 0x80000000 {
            return r;
        }
    }
    let lang = rd(m, sp + 532);
    let cfg = rd(m, sp + 536);
    let r = fn_1756d0(m, h, params, rom_text(m, b"fedatapackaging"), sp + 540, 0);
    if r < 0x80000000
        && rd(m, sp + 540) != 0
        && fn_195d58(m, rd(m, sp + 540), rom_text(m, b"clc")) == 0
    {
        for src in [
            rom_text(m, b"clc/"),
            lang,
            rom_text(m, b"/"),
            cfg,
            rom_text(m, b":"),
        ] {
            route_or_host(m, h, 0x195C90, &[path, src]);
        }
    }
    for src in [rom_text(m, b"dt/"), lang, rom_text(m, b"/"), cfg] {
        route_or_host(m, h, 0x195C90, &[path, src]);
    }
    let n = fn_195d90(m, path);
    for i in 0..n {
        wb(m, path + i, rb(m, path + i).to_ascii_lowercase());
    }
    let rc = fn_242fd0(
        m,
        h,
        [
            rd(m, o + 4),
            rd(m, o + 8),
            2,
            path,
            0,
            0,
            rd(m, 0x1F85B8),
            rd(m, rd(m, 0x1F84E0)),
            sp + 544,
        ],
    );
    if rc >= 0x80000000 {
        return 0;
    }
    let reader = rd(m, sp + 544);
    loop {
        let rc = route_or_host(m, h, 0x24277C, &[reader, sp + 548, sp + 556, sp + 552]);
        if rc >= 0x80000000 {
            break;
        }
        wr(m, sp + 560, 256);
        let rc = route_or_host(
            m,
            h,
            0x242444,
            &[reader, rd(m, sp + 552), rd(m, sp + 556), 0, buf, sp + 560],
        );
        if rc >= 0x80000000 {
            return rc;
        }
        let entry = fn_196394(m, h, rd(m, ctx + 4), 12);
        if entry == 0 {
            return rd(m, 0x1F8B1C);
        }
        let rc = route_or_host(m, h, 0x243AD4, &[reader, entry, entry + 4]);
        if rc >= 0x80000000 {
            return rc;
        }
        let n = rd(m, sp + 560).wrapping_add(3) & !3;
        let rc = route_or_host(
            m,
            h,
            0x2B1A74,
            &[o + 0x190, o + 0x190, 0, rd(m, entry + 4) + n, 0, entry + 8],
        );
        if rc != 0 {
            route_or_host(m, h, 0x243D50, &[rd(m, entry)]);
            fn_196428(m, h, rd(m, ctx + 4), entry);
            return rc.wrapping_add(0x1EA0) | 0x89B02000;
        }
        let mut text = buf;
        loop {
            let mut end = text;
            while rb(m, end) != 0 && rb(m, end) != b',' {
                end += 1;
            }
            let more = rb(m, end) != 0;
            if more {
                wb(m, end, 0);
            }
            let rc = route_or_host(m, h, 0x19668C, &[rd(m, o + 0x174), text, entry]);
            if rc >= 0x80000000 {
                route_or_host(m, h, 0x243D50, &[rd(m, entry)]);
                fn_196428(m, h, rd(m, ctx + 4), entry);
                return rc;
            }
            if !more {
                break;
            }
            text = end + 1;
        }
        let rc = route_or_host(m, h, 0x2422AC, &[reader]);
        if rc >= 0x80000000 {
            return rc;
        }
    }
    route_or_host(m, h, 0x2421B0, &[reader])
}

fn fn_1f1224(m: &mut Memory<'_>, h: &mut impl Calls, a: u32, b: u32, out: u32) -> u32 {
    if out == 0 {
        return rd(m, 0x1F1370);
    }
    wr(m, out, 0);
    wr(m, out + 4, 0);
    let sp = fn_129778(m, h, 16);
    let r = fn_19611c(m, a, b, sp);
    if r >= 0x80000000 {
        return r;
    }
    let ctx = rd(m, sp);
    let reg = rd(m, ctx + 24);
    let names = [0x1F1374, 0x1F137C];
    for i in 0..2 {
        let r = fn_17b058(m, h, reg, names[i], sp + 4 + i as u32 * 4);
        if r >= 0x80000000 {
            for name in &names[..i] {
                fn_17b1c4(m, h, reg, *name);
            }
            return r;
        }
    }
    let p = fn_1963b8(m, h, rd(m, ctx + 4), 1, 32);
    if p == 0 {
        route_or_host(
            m,
            h,
            0x173DC0,
            &[rd(m, ctx + 16), rd(m, 0x1F1390), rd(m, 0x1F138C), 0],
        );
        for name in names {
            fn_17b1c4(m, h, reg, name);
        }
        return rd(m, 0x1F1394);
    }
    wr(m, p, ctx);
    wr(m, p + 4, rd(m, rd(m, sp + 4) + 4));
    for (i, off) in [16, 8, 12].iter().enumerate() {
        wr(m, p + off, rd(m, rd(m, sp + 8) + 4 + i as u32 * 4));
    }
    wr(m, out, p);
    wr(m, out + 4, rd(m, 0x1F1388));
    0
}

fn fn_1f2830(m: &mut Memory<'_>, h: &mut impl Calls, a: u32, b: u32, out: u32) -> u32 {
    if out == 0 {
        return rd(m, 0x1F2A74);
    }
    wr(m, out, 0);
    wr(m, out + 4, 0);
    let sp = fn_129778(m, h, 24);
    let r = fn_19611c(m, a, b, sp);
    if r >= 0x80000000 {
        return r;
    }
    let ctx = rd(m, sp);
    let reg = rd(m, ctx + 24);
    let names = [0x1F2A78, 0x1F2A80, 0x1F2A8C];
    for (i, slot) in [sp + 4, sp + 12, sp + 8].iter().enumerate() {
        let r = fn_17b058(m, h, reg, names[i], *slot);
        if r >= 0x80000000 {
            for name in &names[..i] {
                fn_17b1c4(m, h, reg, *name);
            }
            return r;
        }
    }
    let o = fn_196394(m, h, rd(m, ctx + 4), 48);
    if o == 0 {
        route_or_host(
            m,
            h,
            0x173DC0,
            &[rd(m, ctx + 16), rd(m, 0x1F2AB8), rd(m, 0x1F2AB4), 0],
        );
        for i in [0, 2, 1] {
            fn_17b1c4(m, h, reg, names[i]);
        }
        return rd(m, 0x1F2ABC);
    }
    wr(m, o, ctx);
    wr(m, o + 4, rd(m, rd(m, sp + 4) + 4));
    for (slot, fields) in [(sp + 8, [16, 8, 12]), (sp + 12, [28, 20, 24])] {
        for (i, off) in fields.iter().enumerate() {
            wr(m, o + off, rd(m, rd(m, slot) + 4 + i as u32 * 4));
        }
    }
    wr(m, o + 32, 0);
    wr(m, o + 36, 0);
    wh(m, o + 40, 0);
    let r = fn_1756d0(m, h, rd(m, ctx + 20), 0x1F2A98, sp + 16, 0);
    let n = if r < 0x80000000 && rd(m, sp + 16) != 0 {
        parse_decimal(&m.bytes[rd(m, sp + 16) as usize..]) as u16
    } else {
        0
    };
    wh(m, o + 42, n);
    let r = fn_1756d0(m, h, rd(m, ctx + 20), 0x1F2AC0, sp + 16, 0);
    wb(
        m,
        o + 44,
        u8::from(r < 0x80000000 && fn_195d90(m, rd(m, sp + 16)) > 3),
    );
    wr(m, out, o);
    wr(m, out + 4, rd(m, 0x1F2AC8));
    if r < 0x80000000 && rb(m, o + 44) != 0 {
        r
    } else {
        0
    }
}

fn fn_1de880(m: &mut Memory<'_>, h: &mut impl Calls, a: u32, b: u32, out: u32) -> u32 {
    if out == 0 {
        return rd(m, 0x1DEA44);
    }
    wr(m, out, 0);
    wr(m, out + 4, 0);
    let sp = fn_129778(m, h, 16);
    let r = fn_19611c(m, a, b, sp);
    if r >= 0x80000000 {
        return r;
    }
    let ctx = rd(m, sp);
    let reg = rd(m, ctx + 24);
    let names = [0x1DEA48, 0x1DEA50, 0x1DEA5C];
    for i in 0..3 {
        let r = fn_17b058(m, h, reg, names[i], sp + 4 + i as u32 * 4);
        if r >= 0x80000000 {
            for name in names[..i].iter().rev() {
                fn_17b1c4(m, h, reg, *name);
            }
            return r;
        }
    }
    let o = fn_1963b8(m, h, rd(m, ctx + 4), 1, 40);
    if o == 0 {
        route_or_host(
            m,
            h,
            0x173DC0,
            &[rd(m, ctx + 16), rd(m, 0x1DEA70), rd(m, 0x1DEA6C), 0],
        );
        for name in names {
            fn_17b1c4(m, h, reg, name);
        }
        return rd(m, 0x1DEA74);
    }
    wr(m, o, ctx);
    wr(m, o + 4, rd(m, rd(m, sp + 4) + 4));
    for (slot, fields) in [(sp + 8, [16, 8, 12]), (sp + 12, [28, 20, 24])] {
        for (i, off) in fields.iter().enumerate() {
            wr(m, o + off, rd(m, rd(m, slot) + 4 + i as u32 * 4));
        }
    }
    wr(m, out, o);
    wr(m, out + 4, rd(m, 0x1DEA68));
    0
}

fn fn_1e7318(m: &mut Memory<'_>, h: &mut impl Calls, a: u32, b: u32, out: u32) -> u32 {
    if out == 0 {
        return rd(m, 0x1E7664);
    }
    wr(m, out, 0);
    wr(m, out + 4, 0);
    let sp = fn_129778(m, h, 32);
    fn_3023b0(m, sp, 32);
    let r = fn_19611c(m, a, b, sp);
    if r >= 0x80000000 {
        return r;
    }
    let ctx = rd(m, sp);
    let reg = rd(m, ctx + 24);
    let names = [0x1E72F8, 0x1E7300, 0x1E730C];
    for i in 0..3 {
        let r = fn_17b058(m, h, reg, names[i], sp + 4 + i as u32 * 4);
        if r >= 0x80000000 {
            for name in &names[..i] {
                fn_17b1c4(m, h, reg, *name);
            }
            return r;
        }
    }
    let o = fn_196394(m, h, rd(m, ctx + 4), 60);
    let extra = if o == 0 {
        0
    } else {
        fn_196394(m, h, rd(m, ctx + 4), 128)
    };
    if o == 0 || extra == 0 {
        route_or_host(
            m,
            h,
            0x173DC0,
            &[rd(m, ctx + 16), rd(m, 0x1E7684), rd(m, 0x1E7668), 0],
        );
        for name in names {
            fn_17b1c4(m, h, reg, name);
        }
        if o != 0 {
            fn_196428(m, h, rd(m, ctx + 4), o);
        }
        return rd(m, 0x1E766C);
    }
    wr(m, o, ctx);
    wr(m, o + 4, rd(m, rd(m, sp + 4) + 4));
    for (slot, fields) in [(sp + 8, [16, 8, 12]), (sp + 12, [28, 20, 24])] {
        for (i, off) in fields.iter().enumerate() {
            wr(m, o + off, rd(m, rd(m, slot) + 4 + i as u32 * 4));
        }
    }
    for off in [32, 36, 40, 44, 48] {
        wr(m, o + off, 0);
    }
    wb(m, o + 52, 0);
    wr(m, o + 56, extra);
    let rc = route_or_host(
        m,
        h,
        rd(m, rd(m, o + 28) + 28),
        &[
            rd(m, o + 20),
            rd(m, o + 24),
            0x1E767C,
            0x1E7670,
            sp + 16,
            sp + 20,
            sp + 24,
        ],
    );
    if rc >= 0x80000000 {
        route_or_host(m, h, 0x1E7260, &[rd(m, out), rd(m, out + 4)]);
        wr(m, out, 0);
        wr(m, out + 4, 0);
    }
    if rh(m, sp + 20) == 1 && rd(m, sp + 16) != 0 && rb(m, rd(m, rd(m, sp + 16))) == b'1' {
        wb(m, o + 52, 1);
    }
    wr(m, out, o);
    wr(m, out + 4, rd(m, 0x1E72F0));
    rc
}

fn fn_1f3ac8(m: &mut Memory<'_>, h: &mut impl Calls, a: u32, b: u32, out: u32) -> u32 {
    if out == 0 {
        return rd(m, 0x1F3CE4);
    }
    wr(m, out, 0);
    wr(m, out + 4, 0);
    let sp = fn_129778(m, h, 24);
    let r = fn_19611c(m, a, b, sp);
    if r >= 0x80000000 {
        return r;
    }
    let ctx = rd(m, sp);
    let reg = rd(m, ctx + 24);
    let names = [0x1F3CE8, 0x1F3CF0, 0x1F3CFC, 0x1F3D08];
    for i in 0..4 {
        let r = fn_17b058(m, h, reg, names[i], sp + 4 + i as u32 * 4);
        if r >= 0x80000000 {
            for name in &names[..i.min(2)] {
                fn_17b1c4(m, h, reg, *name);
            }
            return r;
        }
    }
    let o = fn_196394(m, h, rd(m, ctx + 4), 52);
    if o == 0 {
        route_or_host(
            m,
            h,
            0x173DC0,
            &[rd(m, ctx + 16), rd(m, 0x1F3D1C), rd(m, 0x1F3D18), 0],
        );
        for name in names {
            fn_17b1c4(m, h, reg, name);
        }
        return rd(m, 0x1F3D20);
    }
    wr(m, o, ctx);
    wr(m, o + 4, rd(m, rd(m, sp + 4) + 4));
    wr(m, o + 8, rd(m, rd(m, sp + 8) + 4));
    for (slot, off) in [(sp + 12, 12), (sp + 16, 24)] {
        for i in 0..3 {
            wr(m, o + off + i * 4, rd(m, rd(m, slot) + 4 + i * 4));
        }
    }
    wr(m, out, o);
    wr(m, out + 4, rd(m, 0x1F3D14));
    0
}

fn fn_1f4e2c(m: &mut Memory<'_>, h: &mut impl Calls, a: u32, b: u32, out: u32) -> u32 {
    if out == 0 {
        return rd(m, 0x1F5100);
    }
    wr(m, out, 0);
    wr(m, out + 4, 0);
    let sp = fn_129778(m, h, 32);
    fn_3023b0(m, sp, 32);
    let r = fn_19611c(m, a, b, sp);
    if r >= 0x80000000 {
        return r;
    }
    let ctx = rd(m, sp);
    let reg = rd(m, ctx + 24);
    let names = [0x1F4E0C, 0x1F4E14, 0x1F4E20];
    for i in 0..3 {
        let r = fn_17b058(m, h, reg, names[i], sp + 4 + i as u32 * 4);
        if r >= 0x80000000 {
            for name in names[..i].iter().rev() {
                fn_17b1c4(m, h, reg, *name);
            }
            return r;
        }
    }
    let o = fn_196394(m, h, rd(m, ctx + 4), 64);
    if o == 0 {
        route_or_host(
            m,
            h,
            0x173DC0,
            &[rd(m, ctx + 16), rd(m, 0x1F511C), rd(m, 0x1F5118), 0],
        );
        for name in names {
            fn_17b1c4(m, h, reg, name);
        }
        return rd(m, 0x1F5120);
    }
    wr(m, o, ctx);
    wr(m, o + 4, rd(m, rd(m, sp + 4) + 4));
    for (slot, fields) in [(sp + 8, [16, 8, 12]), (sp + 12, [28, 20, 24])] {
        for (i, off) in fields.iter().enumerate() {
            wr(m, o + off, rd(m, rd(m, slot) + 4 + i as u32 * 4));
        }
    }
    for off in [32, 36, 40, 44, 48, 52, 56] {
        wr(m, o + off, 0);
    }
    wb(m, o + 60, 0);
    let rc = route_or_host(
        m,
        h,
        rd(m, rd(m, o + 28) + 28),
        &[
            rd(m, o + 20),
            rd(m, o + 24),
            0x1F5110,
            0x1F5104,
            sp + 16,
            sp + 20,
            sp + 24,
        ],
    );
    if rc >= 0x80000000 {
        route_or_host(m, h, 0x1F4D94, &[rd(m, out), rd(m, out + 4)]);
        wr(m, out, 0);
        wr(m, out + 4, 0);
    }
    if rh(m, sp + 20) == 1 && rd(m, sp + 16) != 0 && rb(m, rd(m, rd(m, sp + 16))) == b'1' {
        wb(m, o + 60, 1);
    }
    wb(m, o + 61, 0);
    let r = fn_1756d0(m, h, rd(m, ctx + 20), 0x1F5124, sp + 28, 0);
    if r < 0x80000000 && rd(m, sp + 28) != 0 && rb(m, rd(m, sp + 28)) != 0 {
        wb(m, o + 61, 1);
    }
    wr(m, out, o);
    wr(m, out + 4, rd(m, 0x1F4E04));
    rc
}

fn fn_2100f0(m: &mut Memory<'_>, h: &mut impl Calls, a: u32, b: u32, out: u32) -> u32 {
    if out == 0 {
        return rd(m, 0x2102C4);
    }
    wr(m, out, 0);
    wr(m, out + 4, 0);
    let sp = fn_129778(m, h, 24);
    let r = fn_19611c(m, a, b, sp);
    if r >= 0x80000000 {
        return r;
    }
    let ctx = rd(m, sp);
    let reg = rd(m, ctx + 24);
    let names = [0x2102C8, 0x2102D0, 0x2102DC];
    for i in 0..3 {
        let r = fn_17b058(m, h, reg, names[i], sp + 4 + i as u32 * 4);
        if r >= 0x80000000 {
            for name in &names[..i] {
                fn_17b1c4(m, h, reg, *name);
            }
            return r;
        }
    }
    let o = fn_196394(m, h, rd(m, ctx + 4), 52);
    if o == 0 {
        route_or_host(
            m,
            h,
            0x173DC0,
            &[rd(m, ctx + 16), rd(m, 0x210304), rd(m, 0x210300), 0],
        );
        for name in names {
            fn_17b1c4(m, h, reg, name);
        }
        return rd(m, 0x210308);
    }
    wr(m, o, ctx);
    wr(m, o + 4, rd(m, rd(m, sp + 4) + 4));
    wr(m, o + 8, rd(m, rd(m, sp + 8) + 4));
    for (i, off) in [20, 12, 16].iter().enumerate() {
        wr(m, o + off, rd(m, rd(m, sp + 12) + 4 + i as u32 * 4));
    }
    wb(m, o + 48, 0);
    let r = route_or_host(m, h, 0x175944, &[rd(m, ctx + 20), 0x2102E8, sp + 16]);
    if r < 0x80000000 && rd(m, sp + 16) == 1 {
        wb(m, o + 48, 1);
    }
    wr(m, out, o);
    wr(m, out + 4, rd(m, 0x2102FC));
    0
}

impl<H: Calls> tts_core::boot_port_3b::Host for RouteHost<'_, H> {
    fn call(&mut self, m: &mut Memory<'_>, f: u32, a: &[u32]) -> u32 {
        if matches!(f, 0x173DC0 | 0x1738E8) {
            assert_ne!(
                a.get(1),
                Some(&u32::MAX),
                "MISSING_SELECTION_LOG_LITERAL f={f:#x}"
            );
        }
        route_or_host(m, self.inner, f, a)
    }
}
fn fn_2271f4(m: &mut Memory<'_>, h: &mut impl Calls, a: u32, b: u32, out: u32) -> u32 {
    if out == 0 {
        return rd(m, 0x227540);
    }
    let sp = fn_129778(m, h, 80);
    let rc = fn_19611c(m, a, b, sp);
    if rc >= 0x80000000 {
        return rc;
    }
    let ctx = rd(m, sp);
    let heap = rd(m, ctx + 4);
    let log = rd(m, ctx + 16);
    let module = rd(m, 0x2269F4);
    let config = rd(m, 0x227194);
    wr(m, out, 0);
    wr(m, out + 4, 0);
    route_or_host(m, h, 0x1738E8, &[log, module, 4, 0, 0x227544, sp + 4]);
    let o = fn_1963b8(m, h, heap, 1, rd(m, 0x227198));
    let mut rc;
    if o == 0 {
        route_or_host(m, h, 0x173DC0, &[log, module, rd(m, 0x227564), 0]);
        rc = rd(m, 0x227568);
    } else {
        wr(m, o, a);
        wr(m, o + 4, b);
        wr(m, o + 8, ctx);
        let r = fn_17b058(m, h, rd(m, ctx + 24), 0x2271C4, sp + 4);
        if r < 0x80000000 {
            wr(m, o + 12, rd(m, rd(m, sp + 4) + 4));
        } else {
            route_or_host(m, h, 0x173DC0, &[log, module, rd(m, 0x22756C), 0]);
        }
        rc = route_or_host(m, h, 0x2252B0, &[o]);
        if rc < 0x80000000 {
            let callbacks = [rd(m, config + 144), rd(m, config + 148)];
            rc = route_or_host(
                m,
                h,
                0x174E74,
                &[
                    rd(m, ctx + 20),
                    rd(m, 0x227570),
                    o,
                    callbacks[0],
                    callbacks[1],
                ],
            );
            for i in 0..3 {
                if rc >= 0x80000000 {
                    break;
                }
                let name = rd(m, config + 152 + i * 4);
                if name == 0 {
                    break;
                }
                let r = route_or_host(m, h, 0x175850, &[rd(m, ctx + 20), name, sp + 8]);
                if r >= 0x80000000 {
                    wr(m, sp + 8, rd(m, config + 4 + i * 4));
                    rc = route_or_host(m, h, 0x1757B8, &[rd(m, ctx + 20), name, rd(m, sp + 8)]);
                }
                if rc >= 0x80000000 {
                    break;
                }
                let value = rd(m, sp + 8);
                if fn_195d58(m, name, rd(m, 0x227588)) == 0 {
                    rc = route_or_host(m, h, 0x2A9608, &[value, o + 0x740]);
                    if rc >= 0x80000000 {
                        break;
                    }
                    wr(m, sp + 16, rd(m, 0x227590));
                    wr(m, sp + 20, value);
                    wr(m, sp + 24, rd(m, o + 0x740));
                    route_or_host(
                        m,
                        h,
                        0x1738E8,
                        &[log, module, 3, 0, rd(m, 0x227590), sp + 20],
                    );
                } else if fn_195d58(m, name, rd(m, 0x22758C)) == 0 {
                    if (value as i32) < 0 {
                        rc = rd(m, 0x227594);
                        break;
                    }
                    wr(m, o + 0x744, value);
                    wr(m, sp + 16, value);
                    route_or_host(
                        m,
                        h,
                        0x1738E8,
                        &[log, module, 3, 0, rd(m, 0x227598), sp + 16],
                    );
                }
                rc = route_or_host(
                    m,
                    h,
                    0x174E74,
                    &[rd(m, ctx + 20), name, o, callbacks[0], callbacks[1]],
                );
            }
        }
        wr(m, out, o);
        wr(m, out + 4, rd(m, 0x22719C));
        if rc < 0x80000000 {
            if route_or_host(m, h, 0x173FD8, &[log]) >= 2 {
                route_or_host(m, h, 0x19854C, &[a, b, 6, o + 0x7C0]);
            }
            wb(m, o + 0x7F4, 0);
        } else {
            route_or_host(m, h, 0x226F68, &[o, rd(m, out + 4)]);
            wr(m, out, 0);
            wr(m, out + 4, 0);
        }
    }
    wr(m, sp + 4, rc);
    route_or_host(m, h, 0x1738E8, &[log, module, 4, 0, 0x2275F0, sp + 4]);
    rc
}

impl<H: Calls> tts_core::boot_port_6::Host for RouteHost<'_, H> {
    fn format_address_m(&mut self, m: &Memory<'_>, format: &[u8]) -> Option<u32> {
        tts_core::boot::Calls::format_address_m(self, m, format)
    }
    fn real_log_args(&mut self) -> bool {
        self.inner.real_log_args()
    }
    fn log_variadic(
        &mut self,
        m: &mut Memory<'_>,
        entry: u32,
        a: [u32; 4],
        format: &[u8],
        varargs: &[u32],
    ) -> bool {
        tts_core::boot::Calls::log_variadic(self, m, entry, a, format, varargs)
    }

    fn thread_id(&mut self) -> u32 {
        0x1FF82000
    }
    fn call(&mut self, m: &mut Memory<'_>, f: u32, a: &[u32]) -> u32 {
        route_or_host(m, self.inner, f, a)
    }
}
fn model_literals(
    m: &mut Memory<'_>,
    h: &mut impl Calls,
    f: u32,
) -> tts_core::boot_port_6::Literals {
    let mut formats = [0; 16];
    let texts: &[&[u8]] = match f {
        0x2D9C0C => &[],
        0x2F7C44 => &[
            b"LoadQuantizedDurFile: invalid quantization format",
            b"LoadQuantizedDurFile: invalid value for code length",
            b"LoadDurFile : the number of HMM states is illegal",
        ],
        0x2F6A50 => &[
            b"LoadVQQuantizedCepFile: invalid quantization format",
            b"ReadQuantDescr: number of streams is wrong\n",
            b"LoadVQQuantizedCepFile: number of substreams is too large\n",
        ],
        0x2F7760 => &[b"ReadQCodeBookSetHdr: invalid quantization format"],
        0x2F80B0 => &[
            b"LoadQuantizedFreqFile: invalid quantization format",
            b"LoadQuantizedFreqFile: invalid value for number of streams",
            b"LoadQuantizedFreqFile: invalid value for code length",
        ],
        _ => panic!("unbound model diagnostic {f:#x}"),
    };
    for (i, text) in texts.iter().enumerate() {
        formats[i] = rom_text(m, text);
    }
    if f == 0x2D9C0C {
        formats[0] = 0x2D9D84;
    }
    if f == 0x2F7760 {
        formats[1] = 0xA1F6B4;
        formats[2] = 0xA1F670;
        formats[3] = rom_text(m, b"LoadQuantizedCodebook: invalid value for code length\n");
    }
    if f == 0x2F80B0 {
        formats[3] = 0xA1F6F8;
    }
    tts_core::boot_port_6::Literals {
        category: 0x917328,
        formats,
        scratch: fn_129778(m, h, 32),
    }
}
fn fn_2da04c(
    m: &mut Memory<'_>,
    h: &mut impl Calls,
    ctx: u32,
    p: u32,
    mode: u32,
    files: u32,
    count: u32,
) -> u32 {
    wr(m, p + 40, p);
    let r = route_or_host(m, h, 0x2F7C44, &[ctx, p + 40, files + 64, 15]);
    if r != 0 {
        return r;
    }
    wr(m, p + 24, rd(m, p + 48));
    wr(m, p + 0x1A4, p);
    let r = route_or_host(m, h, 0x2F6A50, &[ctx, p + 0x1A4, files]);
    if r != 0 {
        return r;
    }
    let log = rd(m, ctx + 16);
    let module = 0x8FA510;
    if rh(m, p + 0x1A8).wrapping_sub(1) >= 2 {
        route_or_host(m, h, 0x1738E8, &[log, module, 0, 0, 0x9C7F28]);
        return 0x84E02004;
    }
    if rb(m, mode + 0x114) == 3 && rh(m, p + 0x1B4) <= rh(m, p + 0x1B2) {
        route_or_host(m, h, 0x1738E8, &[log, module, 1, 0, 0x9C7F70]);
    }
    wr(
        m,
        p + 36,
        (rh(m, p + 0x1B2) as u32 + 1) * rh(m, p + 0x1AC) as u32,
    );
    wr(m, p + 0x254, p);
    let r = route_or_host(m, h, 0x2F7760, &[ctx, p + 0x254, count, 16]);
    if r != 0 {
        return r;
    }
    if rb(m, mode + 69) != 0 {
        route_or_host(
            m,
            h,
            0x1738E8,
            &[
                log,
                module,
                0,
                0,
                rom_text(
                    m,
                    b"LoadModelFiles : Quantized VCF file is not supported!\n",
                ),
            ],
        );
        return 0x84E02004;
    }
    if rb(m, mode + 70) != 0 {
        wr(m, p + 0x12C, p);
        let r = route_or_host(
            m,
            h,
            0x2F80B0,
            &[ctx, p + 0x12C, rd(m, p + 24), files + 96, 18, 0x140000],
        );
        if r != 0 {
            route_or_host(
                m,
                h,
                0x1738E8,
                &[
                    log,
                    module,
                    0,
                    0,
                    rom_text(m, b"LoadModelFiles : Error loading the MVF PDF's!\n"),
                ],
            );
            return r;
        }
        wr(m, p + 32, rd(m, p + 0x138));
    }
    wr(m, p + 0xB4, p);
    let r = route_or_host(
        m,
        h,
        0x2F80B0,
        &[ctx, p + 0xB4, rd(m, p + 24), files + 32, 20, 0x3999A0],
    );
    if r != 0 {
        route_or_host(
            m,
            h,
            0x1738E8,
            &[
                log,
                module,
                0,
                0,
                rom_text(m, b"LoadModelFiles : Error loading the Log F0 PDF's!\n"),
            ],
        );
    } else {
        wr(m, p + 28, rd(m, p + 0xC0));
    }
    r
}

struct ColdPool<'a, 'b, H> {
    m: &'a mut Memory<'b>,
    h: &'a mut H,
    name: u32,
}
impl<H: Calls> tts_core::selection::PoolHost for ColdPool<'_, '_, H> {
    fn block(&self, i: usize) -> tts_core::selection::PoolBlock {
        let p = i as u32;
        let next = rd(self.m, p);
        tts_core::selection::PoolBlock {
            next: if next == 0 { None } else { Some(next as usize) },
            base: rd(self.m, p + 4),
            current: rd(self.m, p + 8),
            end: rd(self.m, p + 12),
        }
    }
    fn set_block(&mut self, block: usize, b: tts_core::selection::PoolBlock) {
        for (i, v) in [b.next.unwrap_or(0) as u32, b.base, b.current, b.end]
            .iter()
            .enumerate()
        {
            wr(self.m, block as u32 + i as u32 * 4, *v);
        }
    }
    fn allocate(&mut self, owner: u32, n: u32) -> Option<(usize, u32)> {
        let p = fn_1963b8(self.m, self.h, rd(self.m, owner + 4), 1, n);
        (p != 0).then_some((p as usize, p))
    }
    fn free(&mut self, owner: u32, p: usize) {
        fn_196428(self.m, self.h, rd(self.m, owner + 4), p as u32);
    }
    fn clear(&mut self, p: u32, n: u32) {
        fn_3023b0(self.m, p, n);
    }
    fn report(&mut self, p: &tts_core::selection::PrivateHeap<'_>, label: &[u8], blocks: u32) {
        let label = rom_text(self.m, label);
        let sp = fn_129778(self.m, self.h, 24);
        for (i, v) in [self.name, label, p.allocated, p.pool, p.expected, blocks]
            .iter()
            .enumerate()
        {
            wr(self.m, sp + i as u32 * 4, *v);
        }
        route_or_host(
            self.m,
            self.h,
            0x1738E8,
            &[rd(self.m, p.owner + 16), 0x917328, 3, 0, 0x2DB384, sp],
        );
    }
}
fn fn_private_heap(m: &mut Memory<'_>, h: &mut impl Calls, p: u32, op: u32, size: u32) -> u32 {
    if p == 0 {
        return 0;
    }
    let next = rd(m, p + 8);
    let mut pool = tts_core::selection::PrivateHeap {
        owner: rd(m, p),
        name: b"",
        first_block: if next == 0 { None } else { Some(next as usize) },
        expected: rd(m, p + 12),
        pool: rd(m, p + 16),
        allocated: rd(m, p + 20),
    };
    let name = rd(m, p + 4);
    let mut host = ColdPool { m, h, name };
    let result = match op {
        0 => pool.allocate(size, &mut host).map_or(0, |a| a.address),
        2 => {
            pool.free(&mut host);
            0
        }
        _ => panic!("unbound pool operation {op}"),
    };
    wr(host.m, p + 8, pool.first_block.unwrap_or(0) as u32);
    wr(host.m, p + 16, pool.pool);
    wr(host.m, p + 20, pool.allocated);
    result
}
fn fn_2a90c4(m: &mut Memory<'_>, h: &mut impl Calls, p: u32, off: u32, n: u32, out: u32) -> u32 {
    if out != 0 {
        wr(m, out, 0);
    }
    let data = rd(m, p + 12);
    let pos = rd(m, p + 8);
    if data != 0 && pos <= off && off.wrapping_add(n) <= pos.wrapping_add(rd(m, p + 16)) {
        if out != 0 {
            wr(m, out, data + off - pos);
        }
        return 0;
    }
    if data != 0 {
        let rc = route_or_host(m, h, 0x179208, &[rd(m, p), rd(m, p + 4), data]);
        wr(m, p + 8, 0);
        wr(m, p + 12, 0);
        wr(m, p + 16, 0);
        if rc >= 0x80000000 {
            return rc;
        }
    }
    let rc = route_or_host(m, h, 0x17A1A4, &[rd(m, p), rd(m, p + 4), off, n, p + 12]);
    if rc >= 0x80000000 {
        return rc;
    }
    wr(m, p + 8, off);
    wr(m, p + 16, n);
    if out != 0 {
        wr(m, out, rd(m, p + 12));
    }
    rc
}
fn fn_2a9274(
    m: &mut Memory<'_>,
    h: &mut impl Calls,
    pool: u32,
    out: u32,
    size: u32,
    mut count: u32,
    st: u32,
) -> u32 {
    let pos = rd(m, st + 12);
    let limit = rd(m, st + 8);
    if size.wrapping_mul(count).wrapping_add(pos) > limit {
        count = route_or_host(m, h, 0x1994D0, &[limit.wrapping_sub(pos), size]);
    }
    if count == 0 {
        return 0;
    }
    if rb(m, st) == 2 {
        let cache = rd(m, st + 28);
        let mut total = size.wrapping_mul(count);
        if pos == 0
            && total <= limit
            && (rd(m, cache + 12) == 0 || rd(m, cache + 8) != rd(m, st + 4))
        {
            total = limit;
        }
        let rc = fn_2a90c4(m, h, cache, rd(m, st + 4).wrapping_add(pos), total, st + 20);
        if rc >= 0x80000000 {
            return 0;
        }
        wr(m, st + 16, rd(m, st + 4).wrapping_add(pos));
        wr(m, st + 24, total);
    }
    let at = rd(m, st + 20)
        .wrapping_add(rd(m, st + 4))
        .wrapping_add(pos)
        .wrapping_sub(rd(m, st + 16));
    if rb(m, st) != 0 || (pos.wrapping_add(3) & !3) != pos {
        let p = fn_private_heap(m, h, pool, 0, size.wrapping_mul(count));
        wr(m, out, p);
        if p == 0 {
            return 0;
        }
        fn_195ae8(m, p, at, size.wrapping_mul(count));
    } else {
        wr(m, out, at);
    }
    wr(m, st + 12, pos.wrapping_add(size.wrapping_mul(count)));
    count
}
fn cold_window_pair(m: &Memory<'_>, p: u32, i: i32) -> u32 {
    rd(m, rd(m, p + 28).wrapping_add((i as u32).wrapping_mul(4)))
}

fn fn_2a2f04(
    m: &mut Memory<'_>,
    h: &mut impl Calls,
    ctx: u32,
    p: u32,
    nw: i32,
    dim: u32,
    n: u32,
    wins: u32,
    name: u32,
) -> u32 {
    let init = tts_core::selection::PrivateHeap::initialize(ctx, b"");
    for (i, v) in [
        init.owner,
        name,
        0,
        init.expected,
        init.pool,
        init.allocated,
    ]
    .into_iter()
    .enumerate()
    {
        wr(m, p + i as u32 * 4, v);
    }
    let c = nw.wrapping_add(1);
    wr(m, p + 24, c as u32);
    wr(m, p + 12, (c as u32).wrapping_mul(28));
    let result = (|| {
        let a = fn_private_heap(m, h, p, 0, rd(m, p + 24).wrapping_mul(4));
        wr(m, p + 28, a);
        if a == 0 {
            return false;
        }
        let mut i = 0i32;
        while i < (rd(m, p + 24) as i32) {
            let q = fn_private_heap(m, h, p, 0, 8);
            wr(m, rd(m, p + 28) + (i as u32) * 4, q);
            if q == 0 {
                return false;
            }
            i += 1;
        }
        let q = fn_private_heap(m, h, p, 0, rd(m, p + 24).wrapping_mul(4));
        wr(m, p + 32, q);
        if q == 0 {
            return false;
        }
        let q = fn_private_heap(m, h, p, 0, rd(m, p + 24).wrapping_mul(4));
        wr(m, p + 36, q);
        if q == 0 {
            return false;
        }
        let pair = cold_window_pair(m, p, 0);
        wr(m, pair + 4, 0);
        wr(m, cold_window_pair(m, p, 0), 0);
        let q = fn_private_heap(m, h, p, 0, 4);
        wr(m, rd(m, p + 32), q);
        if q == 0 {
            return false;
        }
        let q = fn_private_heap(m, h, p, 0, 4);
        wr(m, rd(m, p + 36), q);
        if q == 0 {
            return false;
        }
        let shift = dim.wrapping_sub(dim >> 1) & 255;
        wr(
            m,
            rd(m, rd(m, p + 32)),
            if shift >= 32 { 0 } else { 1u32 << shift },
        );
        let shift = n.wrapping_add(n >> 1).wrapping_sub(dim) & 255;
        wr(
            m,
            rd(m, rd(m, p + 36)),
            if shift >= 32 { 0 } else { 1u32 << shift },
        );
        i = 1;
        while i < (rd(m, p + 24) as i32) {
            let win = rd(m, wins + (i as u32 - 1) * 4);
            wr(m, win + 12, 0);
            let out = rd(m, p + 32) + i as u32 * 4;
            let got = fn_2a9274(m, h, p, out, 4, u32::MAX, win) as i32;
            let half = got / 2;
            let q = half / 2;
            let a = rd(m, rd(m, p + 32) + i as u32 * 4);
            wr(
                m,
                rd(m, p + 36) + i as u32 * 4,
                a.wrapping_add((half as u32) * 4),
            );
            let a = rd(m, rd(m, p + 32) + i as u32 * 4);
            wr(
                m,
                rd(m, p + 32) + i as u32 * 4,
                a.wrapping_add((q as u32) * 4),
            );
            let a = rd(m, rd(m, p + 36) + i as u32 * 4);
            wr(
                m,
                rd(m, p + 36) + i as u32 * 4,
                a.wrapping_add((q as u32) * 4),
            );
            let pair = cold_window_pair(m, p, i);
            wr(m, pair, q.wrapping_neg() as u32);
            wr(
                m,
                cold_window_pair(m, p, i) + 4,
                if half & 1 == 0 { q.wrapping_sub(1) } else { q } as u32,
            );
            i += 1;
        }
        true
    })();
    if !result {
        fn_private_heap(m, h, p, 2, 0);
        return 0x84e0200a;
    }
    wr(m, p + 44, 0);
    wr(m, p + 40, 0);
    let count = rd(m, p + 24) as i32;
    let mut i = if count > 0 { count & 1 } else { 0 };
    if i == 1 {
        let row = cold_window_pair(m, p, 0);
        wr(m, p + 40, (rd(m, row) as i32).min(0) as u32);
        wr(m, p + 44, (rd(m, row + 4) as i32).max(0) as u32);
    }
    while i < count {
        let row = cold_window_pair(m, p, i);
        let mut a = rd(m, row) as i32;
        let b = (rd(m, row + 4) as i32).max(rd(m, p + 44) as i32);
        wr(m, p + 40, a.min(rd(m, p + 40) as i32) as u32);
        wr(m, p + 44, b as u32);
        a = a.min(rd(m, cold_window_pair(m, p, i + 1)) as i32);
        wr(m, p + 40, a as u32);
        let b2 = rd(m, cold_window_pair(m, p, i + 1) + 4) as i32;
        wr(m, p + 44, b.max(b2) as u32);
        i += 2;
    }
    wr(
        m,
        p + 48,
        (rd(m, p + 40) as i32).max(rd(m, p + 44) as i32) as u32,
    );
    0
}

fn fn_228b48(
    m: &mut Memory<'_>,
    h: &mut impl Calls,
    o: u32,
    key: u32,
    min: i32,
    max: i32,
    default: u32,
    record: u32,
    out: u32,
) -> u32 {
    let ctx = rd(m, o + 8);
    let params = rd(m, ctx + 20);
    let mut rc = route_or_host(m, h, 0x175850, &[params, key, out]);
    let value = rd(m, out) as i32;
    let replacement = if rc >= 0x80000000 {
        Some(default)
    } else if value < min {
        Some(min as u32)
    } else if value > max {
        Some(max as u32)
    } else {
        None
    };
    if let Some(v) = replacement {
        rc = route_or_host(m, h, 0x1757B8, &[params, key, v]);
        wr(m, out, v);
        if rc >= 0x80000000 {
            let sp = fn_129778(m, h, 8);
            wr(m, sp, 0x228C78);
            wr(m, sp + 4, key);
            route_or_host(
                m,
                h,
                0x173DC0,
                &[
                    rd(m, ctx + 16),
                    rd(m, 0x228994),
                    rd(m, 0x228C8C),
                    0x228C84,
                    sp,
                ],
            );
            return rc;
        }
    }
    rc = route_or_host(
        m,
        h,
        0x174E74,
        &[
            params,
            key,
            rd(m, record),
            rd(m, record + 4),
            rd(m, record + 8),
        ],
    );
    if rc >= 0x80000000 {
        route_or_host(
            m,
            h,
            0x173DC0,
            &[rd(m, ctx + 16), rd(m, 0x228994), rd(m, 0x228C90), 0],
        );
    }
    rc
}
fn fn_2296b0(m: &mut Memory<'_>, h: &mut impl Calls, a: u32, b: u32, out: u32) -> u32 {
    if out == 0 {
        return rd(m, 0x2299C4);
    }
    let sp = fn_129778(m, h, 48);
    let rc = fn_19611c(m, a, b, sp);
    if rc >= 0x80000000 {
        return rc;
    }
    let ctx = rd(m, sp);
    let log = rd(m, ctx + 16);
    let module = rd(m, 0x228994);
    wr(m, out, 0);
    wr(m, out + 4, 0);
    route_or_host(m, h, 0x1738E8, &[log, module, 4, 0, 0x2299C8, sp + 4]);
    let o = fn_1963b8(m, h, rd(m, ctx + 4), 1, 112);
    if o == 0 {
        route_or_host(m, h, 0x173DC0, &[log, module, rd(m, 0x2299F8), 0]);
        return rd(m, 0x2299FC);
    }
    wr(m, o, a);
    wr(m, o + 4, b);
    wr(m, o + 8, ctx);
    let rc = route_or_host(m, h, 0x176E40, &[rd(m, ctx + 8), rd(m, ctx + 4), o + 12]);
    let callbacks = rd(m, 0x2299C0);
    wr(m, sp + 8, o);
    wr(m, sp + 12, rd(m, callbacks + 4));
    wr(m, sp + 16, rd(m, callbacks + 8));
    let mut rc = rc;
    for (key, min, max, default) in [
        (0x229698, 0, 100, 80),
        (0x2296A0, 50, 400, 100),
        (0x2296A8, 1, 100, 50),
        (rd(m, 0x2299E4), 1, i32::MAX, 256),
    ] {
        if rc >= 0x80000000 {
            break;
        }
        rc = fn_228b48(m, h, o, key, min, max, default, sp + 8, sp + 4);
        if key == rd(m, 0x2299E4) && rc < 0x80000000 {
            wr(m, o + 20, rd(m, sp + 4));
        }
    }
    if rc < 0x80000000 {
        rc = route_or_host(m, h, 0x228C94, &[o]);
    }
    if rc < 0x80000000 {
        rc = fn_17b058(m, h, rd(m, ctx + 24), 0x2299E8, sp + 4);
        if rc < 0x80000000 {
            wr(m, o + 16, rd(m, rd(m, sp + 4) + 4));
            wr(m, out, o);
            wr(m, out + 4, rd(m, 0x2299F4));
        }
    }
    if rc >= 0x80000000 {
        route_or_host(m, h, 0x229A20, &[o, rd(m, 0x2299F4)]);
        wr(m, out, 0);
        wr(m, out + 4, 0);
    }
    wr(m, sp + 4, rc);
    route_or_host(m, h, 0x1738E8, &[log, module, 4, 0, 0x229A04, sp + 4]);
    rc
}

fn acoustic_control(m: &mut Memory<'_>, f: u32, p: u32, v: u32) -> u32 {
    let mut c = tts_core::selection::AcousticControls {
        rate: rd(m, p + 0x18) as i32,
        combined_rate: rd(m, p + 0x1c) as i32,
        pitch: rh(m, p + 0x20) as i16,
        pitch_scale: rd(m, p + 0x24) as i32,
        duration: rd(m, p + 0x28) as i32,
        frequency: rd(m, p + 0x2c),
    };
    let rc = match f {
        0x2A9D5C => c.set_rate(v),
        0x2A9DA0 => c.set_duration(v),
        0x2A9E20 => c.set_frequency(v),
        0x2A9E48 => c.set_pitch(v as i32),
        0x2A9E64 => c.set_pitch_scale(v),
        _ => unreachable!(),
    };
    wr(m, p + 0x18, c.rate as u32);
    wr(m, p + 0x1c, c.combined_rate as u32);
    wh(m, p + 0x20, c.pitch as u16);
    wr(m, p + 0x24, c.pitch_scale as u32);
    wr(m, p + 0x28, c.duration as u32);
    wr(m, p + 0x2c, c.frequency);
    rc as u32
}
fn fn_228780(m: &mut Memory<'_>, h: &mut impl Calls, p: u32, key: u32, v: u32) -> u32 {
    if eqz(m, key, b"audiooutputbufsamples") {
        let frame = rd(m, p + 24);
        let mut n = v.max(1);
        if frame != 0 && n % frame != 0 && n > frame {
            n -= n % frame;
        }
        wr(m, p + 20, n.max(frame));
        return 0;
    }
    let mut rc = route_or_host(m, h, 0x176F2C, &[rd(m, p + 12)]);
    if rc >= 0x80000000 {
        return rc;
    }
    if eqz(m, key, b"volume") {
        if v < 101 {
            wh(m, p + 36, v as u16);
        } else {
            rc = 0x84e0200f;
        }
    } else if eqz(m, key, b"rate") || eqz(m, key, b"pitch") {
        let rate = eqz(m, key, b"rate");
        let sp = fn_129778(m, h, 16);
        let converted;
        if rate {
            rc = route_or_host(m, h, 0x2A9608, &[v, sp + 8]);
            if rc >= 0x80000000 {
                return rc;
            }
            converted = rd(m, sp + 8);
        } else if v.wrapping_sub(1) >= 100 {
            route_or_host(m, h, 0x176F5C, &[rd(m, p + 12)]);
            return 0x84e0200f;
        } else {
            converted = v.wrapping_sub(50).wrapping_mul(5);
        }
        wr(m, sp, v);
        wr(m, sp + 4, converted);
        let fmt = rom_text(
            m,
            if rate {
                b"Set rate %d (synth %d)"
            } else {
                b"Set pitch %d (synth %d)"
            },
        );
        route_or_host(
            m,
            h,
            0x1738E8,
            &[rd(m, rd(m, p + 8) + 16), rd(m, 0x228994), 3, 0, fmt, sp],
        );
        if acoustic_control(
            m,
            if rate { 0x2A9E64 } else { 0x2A9E48 },
            rd(m, p + 40),
            converted,
        ) != 0
        {
            rc = 0x84e0200f;
        }
    } else if eqz(m, key, b"ratewpm") {
        rc = 0x84e02001;
    }
    route_or_host(m, h, 0x176F5C, &[rd(m, p + 12)]);
    rc
}
fn eqz(m: &Memory<'_>, p: u32, s: &[u8]) -> bool {
    let at = p.wrapping_sub(m.base) as usize;
    m.bytes.get(at..at + s.len()) == Some(s) && m.bytes.get(at + s.len()) == Some(&0)
}
fn fn_228c94(m: &mut Memory<'_>, h: &mut impl Calls, p: u32) -> u32 {
    let sp = fn_129778(m, h, 320);
    fn_3023b0(m, sp, 320);
    let ctx = rd(m, p + 8);
    let params = rd(m, ctx + 20);
    let voice = sp;
    let equalizer = sp + 4;
    let slot = sp + 8;
    let result = (|| -> Result<u32, u32> {
        let number = |m: &mut Memory<'_>, h: &mut _, key: &[u8]| -> Result<u32, u32> {
            let k = rom_text(m, key);
            let r = route_or_host(m, h, 0x175850, &[params, k, slot]);
            if r >= 0x80000000 {
                synth_parameter_error(m, h, ctx, k, sp + 288);
                Err(r)
            } else {
                Ok(rd(m, slot))
            }
        };
        let freq = number(m, h, b"frequencyhz")?;
        let key = rom_text(m, b"bet2voicequality");
        let r = route_or_host(m, h, 0x175A1C, &[params, key, slot]);
        let v = rd(m, slot);
        let mut path = b"synth".to_vec();
        if r < 0x80000000 && v != 0 && rb(m, v) != 0 {
            path.push(b'/');
            let n = fn_195d90(m, v);
            path.extend_from_slice(&m.bytes[v as usize..(v + n) as usize]);
            route_or_host(m, h, 0x1757A8, &[params, v]);
        } else {
            path.extend_from_slice(b"/med");
        }
        path.extend_from_slice(b"/fxd/bet2f");
        path.extend_from_slice(((freq as i32) / 1000).to_string().as_bytes());
        assert!(path.len() < 64);
        let at = (sp + 32) as usize;
        m.bytes[at..at + path.len()].copy_from_slice(&path);
        wb(m, sp + 32 + path.len() as u32, 0);
        let version = rd(m, rd(m, 0x229078));
        let hmex = rom_text(m, b"HMEX");
        let r = route_or_host(
            m,
            h,
            0x243A94,
            &[rd(m, p), rd(m, p + 4), 0, sp + 32, hmex, version, voice],
        );
        if r >= 0x80000000 {
            return Err(r);
        }
        if rd(m, p + 40) != 0 {
            route_or_host(m, h, 0x2AA294, &[p + 40]);
            wr(m, p + 40, 0);
        }
        if rd(m, p + 44) != 0 {
            route_or_host(m, h, 0x243D50, &[rd(m, p + 44)]);
            wr(m, p + 44, 0);
        }
        wr(m, p + 28, freq);
        let quality = synth_required(m, h, ctx, b"bet2voiceexcitetype", slot, sp + 288)?;
        wb(m, p + 38, quality as u8);
        let alpha = synth_required(m, h, ctx, b"bet2voicealpha", slot, sp + 288)?;
        let frame = number(m, h, b"bet2voiceframesize")?;
        let samples = freq.wrapping_mul(frame) / 1000;
        wr(m, p + 24, samples);
        if samples != 0 {
            let n = rd(m, p + 20);
            let rem = n % samples;
            if rem != 0 {
                wr(m, p + 20, n.wrapping_add(samples) - rem);
            }
        }
        let volume = synth_required(m, h, ctx, b"bet2voicevol", slot, sp + 288)?;
        wr(m, p + 32, volume);
        let vuv = synth_required(m, h, ctx, b"bet2voicevuvrelamp", slot, sp + 288)?;
        let mvf = synth_required(m, h, ctx, b"bet2voicemvfboost", slot, sp + 288)?;
        let order = synth_required(m, h, ctx, b"bet2voiceorder", slot, sp + 288)?;
        let key = rom_text(m, b"bet2voicevsynthinit");
        route_or_host(m, h, 0x175A1C, &[params, key, sp + 12]);
        let key = rom_text(m, b"bet2cpupower");
        let r = route_or_host(m, h, 0x175A1C, &[params, key, slot]);
        let v = rd(m, slot);
        let mut cpu = 1;
        if r < 0x80000000 && v != 0 && rb(m, v) != 0 {
            let power = parse_decimal(&m.bytes[v as usize..]) as u32;
            if power <= 4 {
                cpu = power;
            }
            route_or_host(m, h, 0x1757A8, &[params, v]);
        }
        let key = rom_text(m, b"bet2equalizersetting");
        let r = route_or_host(m, h, 0x175850, &[params, key, slot]);
        let setting = rd(m, slot);
        let mut eq = 0;
        if r < 0x80000000 && (setting as i32) > 0 {
            let path = rom_text(m, b"equalizer/bet2");
            let tag = rom_text(m, b"EQUL");
            let r = route_or_host(
                m,
                h,
                0x243A94,
                &[rd(m, p), rd(m, p + 4), 0, path, tag, version, equalizer],
            );
            if r >= 0x80000000 {
                return Err(r);
            }
            loop {
                let r = route_or_host(
                    m,
                    h,
                    0x24277C,
                    &[rd(m, equalizer), sp + 160, sp + 176, sp + 180],
                );
                if r >= 0x80000000 {
                    return Err(r);
                }
                let data = rd(m, sp + 180);
                let size = rd(m, sp + 176);
                if eqz(m, sp + 160, b"EQUS")
                    && rd(m, data) == setting
                    && size > 8
                    && rd(m, data + 4) == freq
                {
                    let r = route_or_host(m, h, 0x243AD4, &[rd(m, equalizer), p + 44, sp + 184]);
                    if r >= 0x80000000 {
                        return Err(r);
                    }
                    wr(m, p + 48, rd(m, sp + 184) + 8);
                    wr(m, p + 56, freq);
                    wr(m, p + 52, (size >> 2) - 2);
                    eq = p + 48;
                    break;
                }
                let r = route_or_host(m, h, 0x2422AC, &[rd(m, equalizer)]);
                if r >= 0x80000000 {
                    return Err(r);
                }
            }
        }
        for off in [20, 24, 28] {
            wr(m, sp + 96 + off, 0x4000);
        }
        let text = if rd(m, sp + 12) == 0 {
            rom_text(m, b"")
        } else {
            rd(m, sp + 12)
        };
        if route_or_host(
            m,
            h,
            0x2AA3E4,
            &[
                p + 40,
                ctx,
                sp + 96,
                rd(m, voice),
                frame,
                order,
                alpha,
                freq,
                cpu,
                text,
                eq,
            ],
        ) != 0
        {
            return Err(0x84e02000);
        }
        if acoustic_control(m, 0x2A9DA0, rd(m, p + 40), vuv) != 0
            || acoustic_control(m, 0x2A9E20, rd(m, p + 40), mvf) != 0
        {
            return Err(0x84e02000);
        }
        if acoustic_control(m, 0x2A9D5C, rd(m, p + 40), volume) != 0 {
            return Err(0x84e02000);
        }
        for key in [b"volume".as_slice(), b"rate", b"pitch"] {
            let k = rom_text(m, key);
            let r = route_or_host(m, h, 0x175850, &[params, k, slot]);
            if r >= 0x80000000 {
                return Err(r);
            }
            let r = fn_228780(m, h, p, k, rd(m, slot));
            if r >= 0x80000000 {
                return Err(r);
            }
        }
        Ok(0)
    })();
    let mut rc = match result {
        Ok(v) | Err(v) => v,
    };
    for slot in [voice, equalizer] {
        if rd(m, slot) != 0 {
            let r = route_or_host(m, h, 0x2421B0, &[rd(m, slot)]);
            if rc < 0x80000000 && r >= 0x80000000 {
                rc = r;
            }
        }
    }
    rc
}
fn synth_parameter_error(m: &mut Memory<'_>, h: &mut impl Calls, ctx: u32, key: u32, sp: u32) {
    wr(m, sp, rom_text(m, b"parameter"));
    wr(m, sp + 4, key);
    let fmt = rom_text(m, b"%s%s");
    route_or_host(
        m,
        h,
        0x173DC0,
        &[rd(m, ctx + 16), rd(m, 0x228994), 0x659b, fmt, sp],
    );
}
fn synth_required(
    m: &mut Memory<'_>,
    h: &mut impl Calls,
    ctx: u32,
    key: &[u8],
    slot: u32,
    sp: u32,
) -> Result<u32, u32> {
    let k = rom_text(m, key);
    let r = route_or_host(m, h, 0x175A1C, &[rd(m, ctx + 20), k, slot]);
    let v = rd(m, slot);
    if r >= 0x80000000 || v == 0 || rb(m, v) == 0 {
        synth_parameter_error(m, h, ctx, k, sp);
        return Err(r);
    }
    let n = parse_decimal(&m.bytes[v as usize..]) as u32;
    route_or_host(m, h, 0x1757A8, &[rd(m, ctx + 20), v]);
    Ok(n)
}

fn fn_22cd30(m: &mut Memory<'_>, h: &mut impl Calls, a: u32, b: u32, out: u32) -> u32 {
    if out == 0 {
        return rd(m, 0x22D0DC);
    }
    let sp = fn_129778(m, h, 48);
    let r = fn_19611c(m, a, b, sp);
    if r >= 0x80000000 {
        return r;
    }
    let ctx = rd(m, sp);
    let module = rd(m, 0x22CB30);
    let log = rd(m, ctx + 16);
    wr(m, out, 0);
    wr(m, out + 4, 0);
    route_or_host(m, h, 0x1738E8, &[log, module, 4, 0, 0x22D0E0, sp + 8]);
    let p = fn_1963b8(m, h, rd(m, ctx + 4), 1, 116);
    if p == 0 {
        route_or_host(m, h, 0x173DC0, &[log, module, 0xa11, 0]);
        return 0x8b90200a;
    }
    wr(m, p, a);
    wr(m, p + 4, b);
    wr(m, p + 8, ctx);
    let rc = (|| -> Result<u32, u32> {
        let key = rom_text(m, b"SYNTHSTREAM");
        let r = fn_17b058(m, h, rd(m, ctx + 24), key, sp + 4);
        if r >= 0x80000000 {
            route_or_host(m, h, 0x173DC0, &[log, module, 0xa0e, 0]);
            return Err(r);
        }
        wr(m, p + 12, rd(m, rd(m, sp + 4) + 4));
        let key = rom_text(m, b"AUDIOFETCHER");
        let r = fn_17b058(m, h, rd(m, ctx + 24), key, sp + 4);
        let res = rd(m, sp + 4);
        if r < 0x80000000 && rb(m, res) == 1 && rd(m, res + 4) != 0 {
            wr(m, p + 48, rd(m, res + 4));
            wr(m, p + 52, rd(m, res + 8));
            wr(m, p + 56, rd(m, res + 12));
        } else {
            if r < 0x80000000 {
                route_or_host(m, h, 0x17B1C4, &[rd(m, ctx + 24), key]);
            }
            route_or_host(m, h, 0x173DC0, &[log, module, 0xa00, 0]);
        }
        let keys = [
            rd(m, 0x22CCEC),
            rd(m, 0x22D118),
            rom_text(m, b"volume"),
            rom_text(m, b"rate"),
            rd(m, 0x22D434),
            rd(m, 0x22D438),
        ];
        for (i, key) in keys.into_iter().enumerate() {
            let slot = if i < 2 { p + 60 + i as u32 * 4 } else { sp + 8 };
            let r = route_or_host(m, h, 0x175944, &[rd(m, ctx + 20), key, slot]);
            if r >= 0x80000000 {
                let default = [8000, 1024, 80, 100, 1, 1][i];
                wr(m, slot, default);
                let r = route_or_host(m, h, 0x1758AC, &[rd(m, ctx + 20), key, default]);
                if r >= 0x80000000 {
                    let fmt = rom_text(m, b"%s%s%s%x");
                    wr(m, sp + 16, rom_text(m, b"parameter"));
                    wr(m, sp + 20, key);
                    wr(m, sp + 24, rom_text(m, b"lhError"));
                    wr(m, sp + 28, r);
                    route_or_host(m, h, 0x173DC0, &[log, module, 0xa13, fmt, sp + 16]);
                    return Err(r);
                }
            }
            if i == 2 || i == 3 {
                wh(m, p + 96 + (i as u32 - 2) * 2, rd(m, slot) as u16);
            } else if i >= 4 {
                wb(m, p + 100 + i as u32 - 4, u8::from(rd(m, slot) != 0));
            }
        }
        let converter = route_or_host(m, h, 0x2956F4, &[ctx, rd(m, p + 60) & 0xffff]);
        wr(m, p + 104, converter);
        if converter == 0 {
            return Err(0x8b90200a);
        }
        route_or_host(m, h, 0x2955C0, &[converter]);
        route_or_host(m, h, 0x29556C, &[converter, 100]);
        for off in [108, 112] {
            let q = fn_196394(m, h, rd(m, converter + 256), 2048);
            wr(m, p + off, q);
            if q == 0 {
                return Err(0x8b90200a);
            }
        }
        let config = rd(m, 0x22CB30);
        for key in keys {
            let r = route_or_host(
                m,
                h,
                0x174E74,
                &[
                    rd(m, ctx + 20),
                    key,
                    p,
                    rd(m, config + 0x84),
                    rd(m, config + 0x88),
                ],
            );
            if r >= 0x80000000 {
                return Err(r);
            }
        }
        wr(m, out, p);
        wr(m, out + 4, rd(m, 0x22CCC8));
        Ok(0)
    })();
    let r = match rc {
        Ok(r) => r,
        Err(r) => {
            let fmt = rom_text(m, b"%s%x");
            wr(m, sp + 16, rom_text(m, b"lhError"));
            wr(m, sp + 20, r);
            route_or_host(m, h, 0x173DC0, &[log, module, 0xa14, fmt, sp + 16]);
            route_or_host(m, h, 0x22CB4C, &[p, rd(m, 0x22CCC8)]);
            r
        }
    };
    wr(m, sp + 8, r);
    let fmt = rom_text(m, b"Leaving audioins_ObjOpen: %x");
    route_or_host(m, h, 0x1738E8, &[log, module, 4, 0, fmt, sp + 8]);
    r
}
fn fn_29556c(m: &mut Memory<'_>, p: u32, percent: i32) {
    if percent <= 0 {
        return;
    }
    let bound = rh(m, p + 18) as i16 as i32;
    let scaled = (bound * (percent as i16 as i32) / 100) as i16;
    wh(m, p + 20, scaled as u16);
    let d = tts_core::runtime::signed_division(scaled as i32, bound).unwrap();
    wh(m, p + 14, d.quotient as u16);
    wh(m, p + 34, rd(m, p + 28).wrapping_sub(scaled as u32) as u16);
}
fn fn_2955c0(m: &mut Memory<'_>, h: &mut impl Calls, p: u32) {
    for off in (48..72).step_by(4) {
        route_or_host(m, h, 0x2D0A08, &[rd(m, p + off)]);
    }
    wr(m, p + 244, u32::MAX);
    wr(m, p + 240, u32::MAX);
    wb(m, p + 248, 1);
    for off in [76, 80, 72] {
        wr(m, p + off, 0);
    }
    wb(m, p + 229, 0);
    fn_3023b0(m, rd(m, p + 96), 512);
    fn_3023b0(m, p + 100, 128);
    for off in [22, 24, 44] {
        wh(m, p + off, 0);
    }
}
fn fn_2956f4(m: &mut Memory<'_>, h: &mut impl Calls, ctx: u32, rate: u32) -> u32 {
    let heap = rd(m, ctx + 4);
    let p = fn_1963b8(m, h, heap, 260, 1);
    if p == 0 {
        return 0;
    }
    wr(m, p + 252, ctx);
    wr(m, p + 256, heap);
    wh(m, p, rate as u16);
    let overlap = ((rate as u16 as i32) * 10 / 1000) as i16;
    wh(m, p + 2, 10);
    wh(m, p + 4, overlap as u16);
    wh(m, p + 16, overlap.wrapping_mul(2) as u16);
    wh(m, p + 18, overlap as u16);
    let samples = ((rate as u16 as i32) * 4 / 1000) as i16;
    wr(m, p + 8, 4);
    wh(m, p + 12, samples as u16);
    let tail = if samples < overlap {
        samples
    } else {
        samples.wrapping_mul(2).wrapping_sub(overlap)
    };
    wh(m, p + 32, tail as u16);
    let n = (tail as i32 + (samples as i32 + overlap.wrapping_mul(2) as i32) * 2 - overlap as i32)
        as u32;
    wr(m, p + 28, n);
    for i in 0..6 {
        let q = route_or_host(m, h, 0x2D0A5C, &[heap, n >> (i / 2)]);
        wr(m, p + 48 + i * 4, q);
        if q == 0 {
            panic!("missing29564c allocation cleanup");
        }
    }
    for i in 0..6 {
        let q = rd(m, p + 48 + i * 4);
        route_or_host(m, h, 0x2D09A4, &[q]);
        wr(m, q + 12, n >> (i / 2));
    }
    let q = route_or_host(m, h, 0x2CFD6C, &[heap, 0]);
    wr(m, p + 84, q);
    if q == 0 {
        panic!("missing29564c aux cleanup");
    }
    let q = fn_1963b8(m, h, heap, 128, 4);
    wr(m, p + 96, q);
    if q == 0 {
        panic!("missing29564c block cleanup");
    }
    let count = overlap as i32 * 2;
    let table = fn_196394(m, h, heap, count.wrapping_mul(2) as u32);
    wr(m, p + 36, table);
    if table == 0 {
        panic!("missing29564c table cleanup");
    }
    for i in 0..count {
        let phase = tts_core::runtime::signed_division(i.wrapping_mul(0x3243f), overlap as i32)
            .unwrap()
            .quotient;
        let sine = tts_core::prosody::window_sine(phase);
        let value = (0x10000i32.wrapping_sub(sine) >> 1).min(0xffff);
        wh(m, table + i as u32 * 2, value as u16);
    }
    wh(m, p + 14, 100);
    let scaled = ((overlap as i32) * 100 / 100) as i16;
    wh(m, p + 32, scaled as u16);
    let d = tts_core::runtime::signed_division(scaled as i32, overlap as i32).unwrap();
    wh(m, p + 14, d.quotient as u16);
    wh(m, p + 34, n.wrapping_sub(scaled as u32) as u16);
    fn_2955c0(m, h, p);
    p
}

fn fn_1e36bc(m: &mut Memory<'_>, h: &mut impl Calls, a: u32, b: u32, out: u32) -> u32 {
    if out == 0 {
        return rd(m, 0x1E38EC);
    }
    let sp = fn_129778(m, h, 16);
    let rc = fn_19611c(m, a, b, sp);
    if rc >= 0x80000000 {
        return rc;
    }
    let ctx = rd(m, sp);
    let module = rd(m, 0x1E3684);
    wr(m, out, 0);
    wr(m, out + 4, 0);
    route_or_host(
        m,
        h,
        0x1738E8,
        &[rd(m, ctx + 16), module, 4, 0, 0x1E38F0, sp + 8],
    );
    let p = fn_1963b8(m, h, rd(m, ctx + 4), 1, 44);
    if p == 0 {
        return rd(m, 0x1E390C);
    }
    wr(m, p, a);
    wr(m, p + 4, b);
    wr(m, p + 8, ctx);
    let rc = fn_17b058(m, h, rd(m, ctx + 24), 0x1E3688, sp + 4);
    if rc >= 0x80000000 {
        route_or_host(
            m,
            h,
            0x173DC0,
            &[rd(m, ctx + 16), module, rd(m, 0x1E3910), 0],
        );
        panic!("missing phonmap wrapper cleanup after {rc:#x}");
    }
    wr(m, p + 24, rd(m, rd(m, sp + 4) + 4));
    let mut rc = fn_17b058(m, h, rd(m, ctx + 24), 0x1E3694, sp + 4);
    if rc >= 0x80000000 {
        route_or_host(
            m,
            h,
            0x173DC0,
            &[rd(m, ctx + 16), module, rd(m, 0x1E3934), 0],
        );
        rc = 0;
    } else {
        let info = rd(m, sp + 4);
        for off in [4, 8, 12] {
            wr(m, p + 8 + off, rd(m, info + off));
        }
    }
    wr(m, out, p);
    wr(m, out + 4, rd(m, 0x1E365C));
    wr(m, sp + 8, rc);
    route_or_host(
        m,
        h,
        0x1738E8,
        &[rd(m, ctx + 16), module, 4, 0, 0x1E3914, sp + 8],
    );
    rc
}

fn fn_230c58(m: &mut Memory<'_>, h: &mut impl Calls, a: u32, b: u32, out: u32) -> u32 {
    let empty = pair_176fd4(m);
    if (a == empty[0] && b == empty[1]) || out == 0 {
        return rd(m, 0x230B90);
    }
    wr(m, out, 0);
    wr(m, out + 4, 0);
    let sp = fn_129778(m, h, 8);
    let r = fn_19611c(m, a, b, sp);
    if r >= 0x80000000 {
        return rd(m, 0x230B90);
    }
    let ctx = rd(m, sp);
    let p = fn_1963b8(m, h, rd(m, ctx + 4), 1, 36);
    if p == 0 {
        route_or_host(
            m,
            h,
            0x173DC0,
            &[rd(m, ctx + 16), rd(m, 0x230BAC), 0x3a98, 0],
        );
        return 0x8810200a;
    }
    wr(m, p, a);
    wr(m, p + 4, b);
    let r = fn_17b058(m, h, rd(m, ctx + 24), 0x230B9C, sp + 4);
    if r >= 0x80000000 {
        route_or_host(
            m,
            h,
            0x173DC0,
            &[rd(m, ctx + 16), rd(m, 0x230BAC), 0x3ab0, 0],
        );
        fn_196428(m, h, rd(m, ctx + 4), p);
        return r;
    }
    wr(m, p + 8, rd(m, rd(m, sp + 4) + 4));
    wr(m, out, p);
    wr(m, out + 4, rd(m, 0x2311B4));
    r
}

fn dict_trace(m: &mut Memory<'_>, h: &mut impl Calls, p: u32, sp: u32, end: bool, source: u32) {
    let fmt = rom_text(m, if end { b"%s%s %s%u" } else { b"%s%s %s%s" });
    wr(m, sp, rom_text(m, if end { b"END" } else { b"BEG" }));
    wr(m, sp + 4, 0);
    wr(m, sp + 8, rom_text(m, if end { b"RETC" } else { b"NAME" }));
    wr(m, sp + 12, source);
    route_or_host(m, h, 0x17486C, &[rd(m, rd(m, p + 8) + 16), 0x65, fmt, sp]);
}
fn fn_205f40(m: &mut Memory<'_>, h: &mut impl Calls, a: [u32; 9]) -> u32 {
    let [p, tag, ctype, source, data, size, op0, op1, out] = a;
    let rc = route_or_host(m, h, 0x176F94, &[p, tag, rd(m, 0x20636C), 184]);
    if rc >= 0x80000000 {
        return rd(m, 0x206370);
    }
    wr(m, out, 0);
    wr(m, out + 4, 0);
    let ctx = rd(m, p + 8);
    let broker = rd(m, ctx + 12);
    let module = rd(m, 0x20531C);
    let log = rd(m, ctx + 16);
    if ctype == 0
        || out == 0
        || (source == 0 && (data == 0 || size == 0))
        || (source != 0 && data == 0 && size != 0)
    {
        route_or_host(m, h, 0x173DC0, &[log, module, 0x9c41, 0]);
        return 0x8a802007;
    }
    let sp = fn_129778(m, h, 1200);
    let input = sp;
    let buf = sp + 32;
    let tmp = sp + 1064;
    let va = sp + 1100;
    fn_3023b0(m, sp, 1200);
    for (i, v) in [
        if data != 0 && size != 0 { 0 } else { source },
        source,
        data,
        size,
        ctype,
        op0,
        op1,
    ]
    .into_iter()
    .enumerate()
    {
        wr(m, input + i as u32 * 4, v);
    }
    wr(m, tmp + 4, rom_text(m, b"fedatapackaging"));
    dict_trace(m, h, p, va, false, source);
    let mut mapped = false;
    let result = (|| -> Result<u32, u32> {
        if size == 0
            && tts_core::frontend::find_substr(m.bytes, ctype, rom_text(m, b";loader=broker")) != 0
        {
            let r = route_or_host(m, h, 0x1795F0, &[broker, source, 1, p + 60]);
            if r >= 0x80000000 {
                let k = rom_text(m, b"fedatapackaging");
                let r = route_or_host(m, h, 0x175A1C, &[rd(m, ctx + 20), k, tmp + 8]);
                let v = rd(m, tmp + 8);
                if r >= 0x80000000 || v == 0 || !eqz(m, v, b"clc") || source == 0 {
                    return Ok(0);
                }
                for (key, slot) in [
                    (b"langcode".as_slice(), tmp),
                    (b"fecfg".as_slice(), tmp + 4),
                ] {
                    let k = rom_text(m, key);
                    let r = route_or_host(m, h, 0x175A1C, &[rd(m, ctx + 20), k, slot]);
                    if r >= 0x80000000 {
                        return Err(r);
                    }
                }
                wb(m, buf, 0);
                for s in [
                    rom_text(m, b"clc/"),
                    rd(m, tmp),
                    rom_text(m, b"/"),
                    rd(m, tmp + 4),
                    rom_text(m, b":"),
                    source,
                ] {
                    route_or_host(m, h, 0x195C90, &[buf, s]);
                }
                let r = route_or_host(m, h, 0x1795F0, &[broker, buf, 1, p + 60]);
                if r >= 0x80000000 {
                    return Ok(0);
                }
            }
            mapped = true;
            let r = route_or_host(
                m,
                h,
                0x179374,
                &[broker, rd(m, p + 60), 0, input + 12, p + 64],
            );
            if r >= 0x80000000 {
                return Err(r);
            }
            wr(m, input, 0);
            let t = rom_text(m, b"application/edct-bin-dictionary");
            route_or_host(m, h, 0x195CD0, &[buf, t]);
            let marker = rom_text(m, b"mode=");
            let mode = tts_core::frontend::find_substr(m.bytes, ctype, marker);
            if mode != 0 {
                let mut end = mode;
                while rb(m, end) != 0 && rb(m, end) != b';' {
                    end += 1;
                }
                let sep = rom_text(m, b";");
                route_or_host(m, h, 0x195C90, &[buf, sep]);
                route_or_host(m, h, 0x195DAC, &[buf, mode, (end - mode) & 0xffff]);
            }
            wr(m, input + 16, buf);
            wr(m, input + 8, rd(m, p + 64));
        }
        if rd(m, p + 88) != 0 {
            let f = rd(m, rd(m, p + 88) + 28);
            let r = route_or_host(m, h, f, &[rd(m, p + 92), rd(m, p + 96), input, out]);
            if rd(m, p + 60) != 0 {
                route_or_host(m, h, 0x179208, &[broker, rd(m, p + 60), rd(m, p + 64)]);
                wr(m, p + 64, 0);
                route_or_host(m, h, 0x179F3C, &[broker, rd(m, p + 60)]);
                wr(m, p + 60, 0);
            }
            return if r >= 0x80000000 { Err(r) } else { Ok(r) };
        }
        if rd(m, input) != 0 && rd(m, input + 8) == 0 {
            wr(m, va, rom_text(m, b"uri"));
            wr(m, va + 4, rd(m, input));
            let fmt = rom_text(m, b"%s%s");
            route_or_host(m, h, 0x173DC0, &[log, module, 0x9c43, fmt, va]);
            return Err(0x8a802001);
        }
        wr(m, p + 168, rd(m, input + 8));
        wr(m, p + 172, rd(m, p + 168));
        wr(m, p + 176, rd(m, input + 12));
        wr(m, p + 180, 0);
        let t = rd(m, input + 16);
        let edct = rom_text(m, b"application/edct-bin-dictionary");
        let cdct = rom_text(m, b"application/cdct-bin-dictionary");
        let kind = if fn_1769d0(m, t, edct) == 0 {
            0
        } else if fn_1769d0(m, t, cdct) == 0 {
            1
        } else {
            2
        };
        if kind < 2 {
            let r = if kind == 0 {
                route_or_host(
                    m,
                    h,
                    0x2B4010,
                    &[p + 128, p + 128, tmp + 4, p + 148, p, p + 164, 1, tmp + 12],
                )
            } else {
                route_or_host(m, h, 0x2B22B0, &[p + 128, p + 128, p + 148, p, 0, tmp + 12])
            };
            let r = if r == 0 {
                route_or_host(
                    m,
                    h,
                    if kind == 0 { 0x2B3F04 } else { 0x2B2210 },
                    &[rd(m, tmp + 12), 0x18b54089, p + 68],
                )
            } else {
                r
            };
            if r != 0 {
                wr(m, p + 68, 0);
                return Err(r.wrapping_add(0x1ea0) | 0x8a802000);
            }
            let pair = if kind == 0 { p + 80 } else { p + 72 };
            wr(m, pair, rd(m, tmp + 12));
            wr(m, pair + 4, 0xf38a);
            wr(m, out, rd(m, tmp + 12));
            wr(m, out + 4, 0xf38a);
        } else {
            let ptr = rom_text(m, b"application/x-vocalizer-userdct-pointer");
            if fn_1769d0(m, t, ptr) == 0 {
                wr(m, p + 68, rd(m, input + 8));
            }
        }
        Ok(0)
    })();
    let rc = match result {
        Ok(r) => r,
        Err(r) => {
            if mapped && rd(m, p + 60) != 0 {
                if rd(m, p + 64) != 0 {
                    route_or_host(m, h, 0x179208, &[broker, rd(m, p + 60), rd(m, p + 64)]);
                    wr(m, p + 64, 0);
                }
                route_or_host(m, h, 0x179F3C, &[broker, rd(m, p + 60)]);
                wr(m, p + 60, 0);
            }
            r
        }
    };
    dict_trace(m, h, p, va, true, rc);
    rc
}

fn fn_2818b8(m: &mut Memory<'_>, h: &mut impl Calls, list: u32, value: u32) -> u32 {
    let n = fn_1963b8(m, h, rd(m, list), 1, 12);
    if n == 0 {
        return rd(m, 0x281AEC);
    }
    wr(m, n, u32::MAX);
    wr(m, n + 4, value);
    wr(m, n + 8, 0);
    let mut head = rd(m, list + 4);
    if head == 0 {
        wr(m, list + 4, n);
    } else if rd(m, head) == u32::MAX {
        wr(m, n + 8, head);
        wr(m, list + 4, n);
        return 0;
    } else {
        while rd(m, head + 8) != 0 {
            head = rd(m, head + 8);
        }
        wr(m, head + 8, n);
    }
    wr(m, list + 8, n);
    0
}
fn fn_201748(m: &mut Memory<'_>, h: &mut impl Calls, p: u32, tag: u32, d: u32, out: u32) -> u32 {
    if d == 0 || out == 0 {
        return 0x85002007;
    }
    wr(m, out, 0);
    wr(m, out + 4, 0);
    let r = route_or_host(m, h, 0x176F94, &[p, tag, 0x2f0, 48]);
    if r >= 0x80000000 {
        return r;
    }
    let sp = fn_129778(m, h, 96);
    let r = fn_19611c(m, rd(m, p), rd(m, p + 4), sp);
    if r >= 0x80000000 {
        return r;
    }
    let ctx = rd(m, sp);
    let module = rd(m, 0x201528);
    let log = rd(m, ctx + 16);
    let uri = rd(m, d);
    let empty = 0x8F972C;
    if uri != 0 {
        wr(m, sp + 4, 0);
        loop {
            let e = route_or_host(m, h, 0x281AA4, &[p + 24, sp + 4]);
            if e == 0 {
                break;
            }
            if fn_195d58(m, uri, rd(m, e + 4)) == 0 {
                wr(m, sp + 16, uri);
                let fmt = rom_text(m, b"dcteg_LoadDictionary failed, already loaded: %s");
                route_or_host(m, h, 0x1738E8, &[log, module, 3, 0, fmt, sp + 16]);
                return 0x85002418;
            }
        }
    }
    let e = fn_1963b8(m, h, rd(m, ctx + 4), 1, 44);
    if e == 0 {
        route_or_host(m, h, 0x173DC0, &[log, module, 0x1130, 0]);
        return 0x8500200a;
    }
    wr(m, e, rd(m, ctx + 4));
    let mut hnd = 0;
    let rc = (|| -> Result<u32, u32> {
        let mut typ = rd(m, d + 16);
        if uri != 0 {
            if rd(m, p + 12) == 0 {
                let k = rom_text(m, b"INET");
                let r = fn_17b058(m, h, rd(m, rd(m, p + 8) + 24), k, sp + 8);
                if r >= 0x80000000 {
                    route_or_host(m, h, 0x173DC0, &[log, module, 0x117a, 0]);
                    return Err(0x85002001);
                }
                let info = rd(m, sp + 8);
                for off in [4, 8, 12] {
                    wr(m, p + 8 + off, rd(m, info + off));
                }
            }
            wr(m, sp + 8, 0);
            wr(m, sp + 12, 0);
            let r = route_or_host(
                m,
                h,
                rd(m, rd(m, p + 12) + 52),
                &[
                    rd(m, p + 16),
                    rd(m, p + 20),
                    uri,
                    rd(m, d + 20),
                    rd(m, d + 24),
                    0,
                    sp + 8,
                    0,
                    sp + 12,
                    e + 20,
                    e + 24,
                ],
            );
            hnd = rd(m, sp + 8);
            if r >= 0x80000000 {
                return Err(r);
            }
            if typ == 0 || rb(m, typ) == 0 {
                typ = rd(m, sp + 12);
            }
            if typ == 0 || rb(m, typ) == 0 {
                wr(m, sp + 16, rom_text(m, b"uri"));
                wr(m, sp + 20, uri);
                route_or_host(m, h, 0x173DC0, &[log, module, 0x113f, 0x20151C, sp + 16]);
                return Err(0x85003d22);
            }
        } else if rd(m, d + 8) == 0 {
            route_or_host(m, h, 0x173DC0, &[log, module, 0x1138, 0]);
            return Err(0x85002007);
        }
        let types = [0x201498, 0x2014B8, 0x2014D8];
        let mut kind = 3;
        if typ != 0 {
            for (i, t) in types.into_iter().enumerate() {
                if tts_core::frontend::find_substr(m.bytes, typ, t) != 0 {
                    kind = i;
                    break;
                }
            }
            if kind == 3
                && tts_core::frontend::find_substr(
                    m.bytes,
                    typ,
                    rom_text(m, b"application/octet-stream"),
                ) != 0
            {
                kind = 0;
            }
        }
        wr(
            m,
            e + 8,
            if kind < 3 {
                [0x8F978F, 0x8F97AF, 0x8F97CF][kind]
            } else {
                0x8F9776
            },
        );
        let mode = rom_text(m, b"mode=langoverwriting");
        wr(
            m,
            e + 12,
            if typ != 0 && tts_core::frontend::find_substr(m.bytes, typ, mode) != 0 {
                0x8F97F7
            } else {
                0x8F980C
            },
        );
        if uri == 0 {
            if kind == 3 {
                return Err(0x85002416);
            }
            wr(m, e + 24, if kind == 2 { 4 } else { rd(m, d + 12) });
            wr(m, e + 20, rd(m, d + 8));
        }
        let name = if rd(m, d + 4) == 0 {
            empty
        } else {
            rd(m, d + 4)
        };
        let q = fn_196394(m, h, rd(m, ctx + 4), fn_195d90(m, name) + 1);
        wr(m, e + 4, q);
        if q == 0 {
            return Err(0x8500200a);
        }
        fn_195cd0(m, q, name);
        if !types.iter().any(|t| fn_195d58(m, rd(m, e + 8), *t) == 0) {
            return Err(0x85002007);
        }
        if rd(m, p + 36) == 0 {
            let r = route_or_host(m, h, 0x281C14, &[rd(m, p), rd(m, p + 4), p + 36]);
            if r >= 0x80000000 {
                return Err(r);
            }
        }
        let r = route_or_host(m, h, 0x281F34, &[rd(m, p + 36), e]);
        if r >= 0x80000000 {
            return Err(r);
        }
        wb(m, e + 16, 1);
        let r = fn_2818b8(m, h, p + 24, e);
        if r >= 0x80000000 {
            wr(m, sp + 16, rom_text(m, b"name"));
            wr(
                m,
                sp + 20,
                if rd(m, e + 4) == 0 {
                    empty
                } else {
                    rd(m, e + 4)
                },
            );
            let fmt = rom_text(m, b"%s%s");
            route_or_host(m, h, 0x173DC0, &[log, module, 0x1142, fmt, sp + 16]);
            return Err(r | 0x85002000);
        }
        wr(m, out, e);
        wr(m, out + 4, 0x2f1);
        Ok(r)
    })();
    if hnd != 0 {
        route_or_host(
            m,
            h,
            rd(m, rd(m, p + 12) + 56),
            &[rd(m, p + 16), rd(m, p + 20), hnd],
        );
    }
    match rc {
        Ok(r) => r,
        Err(r) => {
            route_or_host(m, h, 0x2012BC, &[p, e]);
            r
        }
    }
}

fn fn_281f34(m: &mut Memory<'_>, h: &mut impl Calls, p: u32, e: u32) -> u32 {
    let sp = fn_129778(m, h, 48);
    let rc = fn_19611c(m, rd(m, p), rd(m, p + 4), sp);
    if rc >= 0x80000000 {
        return rc;
    }
    let ctx = rd(m, sp);
    let t = rd(m, e + 8);
    wr(m, e + 28, 0);
    let edct = 0x281EB0;
    let cdct = 0x281ED0;
    let user = 0x281EF0;
    let mut rc = rc;
    if fn_195d58(m, t, edct) == 0 || fn_195d58(m, t, cdct) == 0 {
        let ed = fn_195d58(m, t, edct) == 0;
        let obj = if ed { e + 32 } else { e + 36 };
        let r = if ed {
            route_or_host(
                m,
                h,
                0x2B4010,
                &[p + 24, p + 24, sp + 24, p + 44, e, p + 60, 1, obj],
            )
        } else {
            route_or_host(m, h, 0x2B22B0, &[p + 24, p + 24, p + 44, e, 0, obj])
        };
        if r != 0 {
            dictionary_attach_error(m, h, ctx, e, if ed { 0x117e } else { 0x1181 }, r, sp + 8);
            wr(m, e + 40, 0);
            rc = r.wrapping_add(0x1ea0) | 0x85002000;
        } else if ed && rd(m, e + 20) != 0 && rb(m, e + 17) == 0 {
            dictionary_attach_error(m, h, ctx, e, 0x117f, 0, sp + 8);
            rc = 0x85002019;
        } else {
            let r = route_or_host(
                m,
                h,
                if ed { 0x2B3F04 } else { 0x2B2210 },
                &[rd(m, obj), 0x18b54089, e + 40],
            );
            if r != 0 {
                dictionary_attach_error(m, h, ctx, e, if ed { 0x1180 } else { 0x1182 }, r, sp + 8);
                wr(m, e + 40, 0);
                rc = r.wrapping_add(0x1ea0) | 0x85002000;
            }
        }
    } else if fn_195d58(m, t, user) == 0 {
        wr(m, e + 40, rd(m, e + 20));
    } else {
        let fmt = rom_text(m, b"%s%s");
        wr(m, sp + 8, rom_text(m, b"contentType"));
        wr(m, sp + 12, t);
        route_or_host(
            m,
            h,
            0x173DC0,
            &[rd(m, ctx + 16), rd(m, 0x281EA4), 0x1133, fmt, sp + 8],
        );
        rc = 0x85002416;
    }
    if rc >= 0x80000000 && (rd(m, e + 32) != 0 || rd(m, e + 36) != 0) {
        route_or_host(m, h, 0x281D8C, &[p, e]);
    }
    rc
}
fn dictionary_attach_error(
    m: &mut Memory<'_>,
    h: &mut impl Calls,
    ctx: u32,
    e: u32,
    code: u32,
    rc: u32,
    sp: u32,
) {
    wr(m, sp, rom_text(m, b"name"));
    wr(m, sp + 4, rd(m, e + 4));
    let fmt = if code == 0x117f {
        rom_text(m, b"%s%s")
    } else {
        wr(m, sp + 8, rom_text(m, b"code"));
        wr(m, sp + 12, rc);
        rom_text(m, b"%s%s%s%x")
    };
    route_or_host(
        m,
        h,
        0x173DC0,
        &[rd(m, ctx + 16), rd(m, 0x281EA4), code, fmt, sp],
    );
}

impl<H: Calls> tts_core::boot_port_5::Host for ColdArchive<'_, H> {
    fn call(&mut self, m: &mut Memory<'_>, f: u32, a: &[u32]) -> u32 {
        route_or_host(m, self.inner, f, a)
    }
}
impl<H: Calls> tts_core::boot_port_6::Host for ColdArchive<'_, H> {
    fn call(&mut self, m: &mut Memory<'_>, f: u32, a: &[u32]) -> u32 {
        route_or_host(m, self.inner, f, a)
    }
    fn real_log_args(&mut self) -> bool {
        self.inner.real_log_args()
    }
    fn format_address_m(&mut self, m: &Memory<'_>, format: &[u8]) -> Option<u32> {
        self.inner
            .real_log_args()
            .then(|| rom_text(m, format.strip_suffix(&[0]).unwrap_or(format)))
    }
    fn log_variadic(
        &mut self,
        m: &mut Memory<'_>,
        entry: u32,
        a: [u32; 4],
        format: &[u8],
        va: &[u32],
    ) -> bool {
        let mut host = RouteHost { inner: self.inner };
        tts_core::boot::Calls::log_variadic(&mut host, m, entry, a, format, va)
    }
}
impl<H: Calls> tts_core::boot_part_f::Host for ColdArchive<'_, H> {
    fn call(&mut self, m: &mut Memory<'_>, f: u32, a: &[u32]) -> u32 {
        route_or_host(m, self.inner, f, a)
    }
    fn compare(
        &mut self,
        m: &mut Memory<'_>,
        _: u32,
        k: &mut tts_core::boot_part_f::Key,
        v: &mut u32,
    ) -> u32 {
        tts_core::boot_part_f::compare_parameter(m, *k, *v)
    }
}
impl<H: Calls> tts_core::log_live::LogMethodsM for ColdArchive<'_, H> {
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
        route_or_host(m, self.inner, f, &[o, c, l, a, fmt, va])
    }
}
impl<H: Calls> tts_core::speech_part_5::Host for ColdArchive<'_, H> {
    fn scratch(&self) -> u32 {
        self.scratch + 1024
    }
    fn format(&self, site: u32) -> u32 {
        panic!("missing dictionary speech format {site:#x}")
    }
    fn dictionary_entries(
        &mut self,
        m: &mut Memory<'_>,
        p: u32,
        entries: &mut u32,
        count: &mut u32,
    ) -> u32 {
        let sp = self.scratch + 1408;
        wr(m, sp, *entries);
        wr(m, sp + 4, *count);
        let r = tts_core::boot_port_6::fn_2e4098(m, self, p, sp, sp + 4);
        *entries = rd(m, sp);
        *count = rd(m, sp + 4);
        r
    }
    fn record(
        &mut self,
        _: &mut Memory<'_>,
        _: u32,
        _: u32,
        _: &mut tts_core::speech_part_5::RecordBuffer,
        _: &mut u8,
    ) -> u32 {
        panic!("missing dictionary speech record")
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
        panic!("missing dictionary speech entries")
    }
}
fn fn_2b4010(m: &mut Memory<'_>, h: &mut impl Calls, a: [u32; 8]) -> u32 {
    let scratch = fn_129778(m, h, 8192);
    let encrypted = std::array::from_fn(|i| rb(m, rd(m, 0x2FEA7C) + i as u32));
    let plain = std::array::from_fn(|i| rb(m, rd(m, 0x2FF468) + i as u32));
    let e = tts_core::boot_port_5::DictionaryEnvironment {
        base_vtable: rd(m, 0x2E4DC0),
        interface_vtable: rd(m, 0x2FBF14),
        dictionary_vtable: rd(m, 0x2B2D18),
        collection_vtable: rd(m, 0x2E4624),
        token_vtable: rd(m, 0x2F91F4),
        parameters: &tts_core::boot_part_f::Environment::US,
        encrypted,
        plain,
    };
    let archive = tts_core::boot_part_e::ArchiveEnvironment {
        parameters: tts_core::boot_part_f::Environment::US,
        stream_table: 0x8FAAE0,
        wrapper_table: 0x917E3C,
        validator: 0x1A8684,
        substream_table: 0x917E04,
        nvar: 0x17D078,
        chinese: 0x17D080,
        scratch: scratch + 2048,
    };
    let mut host = ColdArchive {
        inner: h,
        scratch,
        plain: rd(m, 0x1B2534),
        encoded: rd(m, 0x1B2D3C),
    };
    tts_core::boot_port_5::fn_2b4010(m, &mut host, a, &e, &archive, scratch + 1536)
}

fn fn_151f80(
    m: &mut Memory<'_>,
    h: &mut impl Calls,
    ctx: u32,
    channel: u32,
    kind: u32,
    level: u32,
) {
    if kind > 1 {
        return;
    }
    let sp = fn_129778(m, h, 64);
    let text = sp + 32;
    let name = rom_text(m, if kind == 0 { b"tts" } else { b"cpr" });
    fn_195cd0(m, text, name);
    if rb(m, channel + 148) == 0 {
        let s = rom_text(m, b",unthrottled");
        route_or_host(m, h, 0x195C90, &[text, s]);
    }
    let base = channel + 48 + kind * 24;
    let feature = rb(m, rd(m, 0x152078) + kind) as u32;
    let ports = route_or_host(m, h, 0x17EB74, &[feature]);
    for (i, v) in [
        rom_text(m, b"LUSED"),
        ports.wrapping_add(rd(m, base + 12)),
        rom_text(m, b"LMAX"),
        rd(m, base + 4).wrapping_add(rd(m, base + 16)),
        rom_text(m, b"OMAX"),
        rd(m, base + 8),
        rom_text(m, b"LFEAT"),
        text,
    ]
    .into_iter()
    .enumerate()
    {
        wr(m, sp + i as u32 * 4, v);
    }
    let fmt = rom_text(m, b"%s%u%s%u%s%u%s%s");
    route_or_host(m, h, 0x173FB4, &[rd(m, ctx + 16), level, fmt, sp]);
}
fn fn_152d48(
    m: &mut Memory<'_>,
    h: &mut impl Calls,
    ctx: u32,
    p: u32,
    feature: u32,
    mode: u32,
) -> u32 {
    if mode == 2 {
        wr(m, p + 100, 2);
        wr(m, p + 104, 2);
        return 0;
    }
    let kind = match feature {
        0 => 0,
        3 => 1,
        4 => 3,
        _ => return 0x81202007,
    };
    let base = p + 48 + kind * 24;
    let requested = if kind == 3 { 1 } else { rd(m, base) };
    if kind != 3 && requested == 0 {
        return 0;
    }
    let sp = fn_129778(m, h, 20);
    wr(m, sp, 0);
    let rc = route_or_host(m, h, 0x17EAF4, &[0, feature, requested, sp]);
    let available = rd(m, sp);
    if kind != 3 {
        wr(m, base + 4, if rc == 0 { available } else { 0 });
        if rc == 0 && available < requested {
            let fmt = rom_text(m, b"%s%u%s%u");
            wr(m, sp + 4, rom_text(m, b"configuredPorts"));
            wr(m, sp + 8, requested);
            wr(m, sp + 12, rom_text(m, b"availablePorts"));
            wr(m, sp + 16, available);
            route_or_host(
                m,
                h,
                0x173DC0,
                &[rd(m, ctx + 16), rd(m, 0x151B80), 0x3ef6 + kind, fmt, sp + 4],
            );
            return 0x81203d06;
        }
    }
    if rc == 0 || (kind == 3 && rc == 4) {
        fn_151f80(m, h, ctx, p, kind, 10);
        0
    } else if rc == 4 {
        0x81203d07
    } else {
        0x81203d06
    }
}
fn license_error(
    m: &mut Memory<'_>,
    h: &mut impl Calls,
    ctx: u32,
    code: u32,
    label: &[u8],
    value: u32,
    sp: u32,
) {
    wr(m, sp, rom_text(m, label));
    wr(m, sp + 4, value);
    let fmt = rom_text(m, b"%s%lu");
    route_or_host(
        m,
        h,
        0x173DC0,
        &[rd(m, ctx + 16), rd(m, 0x151B80), code, fmt, sp],
    );
}
fn fn_1529b4(m: &mut Memory<'_>, h: &mut impl Calls, ctx: u32, o: u32) -> u32 {
    if rb(m, o + 106) != 0 {
        return 0x81202011;
    }
    let mut kind = rb(m, o + 105) as u32;
    let p = rd(m, o + 16);
    let mutex = rd(m, p + 144);
    let mut rc = route_or_host(m, h, 0x176F2C, &[mutex]);
    let sp = fn_129778(m, h, 32);
    if rc >= 0x80000000 {
        wr(m, sp, rom_text(m, b"function"));
        wr(
            m,
            sp + 4,
            rom_text(m, b"ttseg_loc_LicensingIncreaseInstanceCount"),
        );
        wr(m, sp + 8, rom_text(m, b"lhError"));
        wr(m, sp + 12, rc);
        let fmt = rom_text(m, b"%s%s%s%x");
        route_or_host(
            m,
            h,
            0x173DC0,
            &[rd(m, ctx + 16), rd(m, 0x151B80), 0x3ef0, fmt, sp],
        );
        return rc;
    }
    if kind == 2 {
        let base = p + 96;
        if rd(m, base + 12) < rd(m, base + 4) {
            wr(m, base + 12, rd(m, base + 12) + 1);
            wb(m, o + 106, 1);
        } else {
            license_error(m, h, ctx, 0x3ef3, b"licensePorts", rd(m, base + 4), sp);
            rc = 0x81203d06;
        }
        fn_151f80(m, h, ctx, p, kind, if rc >= 0x80000000 { 7 } else { 5 });
    } else if kind <= 1 {
        if kind == 1
            && rd(m, p + 84) >= rd(m, p + 76)
            && route_or_host(m, h, 0x17EB74, &[3]) >= rd(m, p + 88)
            && (rd(m, p + 60) < rd(m, p + 52)
                || route_or_host(m, h, 0x17EB74, &[0]) < rd(m, p + 88))
        {
            license_error(m, h, ctx, 0x3ef5, b"cprLicensePorts", rd(m, p + 76), sp);
            fn_151f80(m, h, ctx, p, kind, 7);
            kind = 0;
            wb(m, o + 105, 0);
        }
        let base = p + 48 + kind * 24;
        let feature = rb(m, rd(m, 0x152078) + kind) as u32;
        if rd(m, base + 4) > rd(m, base + 12) {
            wr(m, base + 12, rd(m, base + 12) + 1);
            wb(m, o + 106, 1);
        } else if route_or_host(m, h, 0x17EB74, &[feature]) < rd(m, base + 16) {
            let token = route_or_host(m, h, 0x17EB3C, &[feature]);
            wr(m, o + 108, token);
            if token != u32::MAX {
                wb(m, o + 106, 2);
            }
        }
        let level = if rb(m, o + 106) == 0 {
            license_error(
                m,
                h,
                ctx,
                0x3ef3 + kind,
                b"licensePorts",
                rd(m, base + 4),
                sp,
            );
            rc = 0x81203d06;
            7
        } else {
            let ports = route_or_host(m, h, 0x17EB74, &[feature]);
            if ports.wrapping_add(rd(m, base + 12)) > rd(m, base + 8) {
                let ports = route_or_host(m, h, 0x17EB74, &[feature]);
                wr(m, sp, rom_text(m, b"instanceCount"));
                wr(m, sp + 4, ports.wrapping_add(rd(m, base + 12)));
                wr(m, sp + 8, rom_text(m, b"threshold"));
                wr(m, sp + 12, rd(m, base + 8));
                let fmt = rom_text(m, b"%s%lu%s%lu");
                route_or_host(
                    m,
                    h,
                    0x173DC0,
                    &[rd(m, ctx + 16), rd(m, 0x151B80), 0x3ef1 + kind, fmt, sp],
                );
            }
            5
        };
        fn_151f80(m, h, ctx, p, kind, level);
    }
    route_or_host(m, h, 0x176F5C, &[rd(m, rd(m, o + 16) + 144)]);
    rc
}

pub fn startup_output_lock(m: &mut Memory<'_>, h: &mut impl Calls) {
    let guard = rd(m, 0x18aca0);
    let object = rd(m, 0x18aca4);
    let p = {
        let mut host = RouteHost { inner: h };
        tts_core::boot_part_b2::fn_18ac58(m, guard, object, &mut host)
    };
    fn_122058(m, p + 0x1c4);
    sdk_audio_pool(m);
}

fn sdk_audio_pool(m: &mut Memory<'_>) {
    let p = rd(m, 0x825c08);
    wr(m, p + 16, 0);
    wr(m, p + 20, 0);
    wr(m, p + 24, u32::MAX);
    for i in 0..24 {
        let outer = p + 28 + i * 108;
        let inner = p + 0xa3c + i * 124;
        wr(m, outer, i);
        for off in (0x34..=0x60).step_by(4) {
            wr(m, outer + off, rd(m, 0x6c51e8));
        }
        wr(m, p + 0x15dc + i * 4, outer);
        wr(m, inner, i);
        for off in (0x38..=0x64).step_by(4) {
            wr(m, inner + off, rd(m, 0x6c53c0));
        }
        wr(m, inner + 0x70, 0);
        wr(m, inner + 0x74, 0);
        wr(m, inner + 0x78, u32::MAX);
        fn_122058(m, inner + 0x70);
        wh(m, inner + 4, 1);
        wh(m, inner + 0x6c, rh(m, inner + 0x6c) | 0x8000);
        wr(m, outer + 0x68, inner);
    }
    for off in [4, 8, 0] {
        wr(m, p + off, 0);
    }
    wh(m, p + 12, 0);
    wb(m, p + 14, 0);
    fn_122058(m, p + 16);
}

fn sdk_voice_reset(m: &mut Memory<'_>, p: u32) {
    let one = rd(m, 0x1c48cc);
    let zero = rd(m, 0x1c48d4);
    wb(m, p + 4, 2);
    wr(m, p + 0x18, rd(m, 0x1c48d0));
    wr(m, p + 0x1c, one);
    for off in [5, 7] {
        wb(m, p + off, 0);
    }
    for off in [8, 12, 16] {
        wr(m, p + off, 0);
    }
    wh(m, p + 20, 0);
    for off in [0x20, 0x24, 0x28, 0x2c, 0x30] {
        wr(m, p + off, 0);
    }
    for off in [0x34, 0x38, 0x64] {
        wr(m, p + off, one);
    }
    for off in (0x3c..=0x60).step_by(4) {
        wr(m, p + off, zero);
    }
    let q = rd(m, p + 0x68);
    wb(m, q + 13, 2);
    wb(m, q + 12, 0);
    wr(m, q + 8, 0);
    wb(m, q + 0x6e, 0);
    wb(m, q + 0x6f, 0);
    wh(m, q + 0x1e, (rh(m, q + 0x1e) & !0x7f) | 5);
    wr(m, q + 0x68, rd(m, 0x1c7244));
    wh(m, q + 0x6c, rh(m, q + 0x6c) | 1);
    for off in [0x38, 0x3c] {
        wr(m, q + off, rd(m, 0x1c7244));
    }
    for off in (0x40..=0x64).step_by(4) {
        wr(m, q + off, rd(m, 0x1c7248));
    }
    wr(m, q + 0x20, rd(m, 0x1c724c));
    wr(m, q + 0x24, rd(m, 0x1c7244));
    wh(m, q + 0x6c, rh(m, q + 0x6c) | 0x27);
    wb(m, q + 14, 0);
    wb(m, q + 15, 0);
    for off in [16, 20, 24, 0x2c, 0x30] {
        wr(m, q + off, 0);
    }
    for off in [28, 0x34, 0x36, 6] {
        wh(m, q + off, 0);
    }
}

fn sdk_voice_create(
    m: &mut Memory<'_>,
    h: &mut impl Calls,
    priority: u32,
    callback: u32,
    arg: u32,
) -> u32 {
    if priority >= 0x8000 {
        return 0;
    }
    let p = rd(m, 0x1bb7b4);
    route_or_host(m, h, 0x1840fc, &[p + 16]);
    let mask = rd(m, p);
    let index = (!mask).trailing_zeros();
    if mask.count_ones() == 24 {
        panic!("missing SDK voice eviction");
    }
    let mut voice = 0;
    if index < 24 {
        wr(m, p, mask | (1 << index));
        wh(m, p + 12, rh(m, p + 12).wrapping_add(1));
        voice = rd(m, p + 0x15dc + index * 4);
        sdk_voice_reset(m, voice);
    }
    assert!(voice != 0, "SDK voice pool exhausted");
    wr(m, voice + 0x20, priority);
    let mut cursor = rd(m, p + 4);
    if cursor == 0 {
        wr(m, voice + 0x24, 0);
        wr(m, p + 4, voice);
        wr(m, voice + 0x28, 0);
        wr(m, p + 8, voice);
    } else {
        loop {
            if rd(m, cursor + 0x20) <= priority {
                let prev = rd(m, cursor + 0x24);
                wr(m, voice + 0x28, cursor);
                wr(m, voice + 0x24, prev);
                if prev != 0 {
                    wr(m, prev + 0x28, voice);
                } else {
                    wr(m, p + 4, voice);
                }
                wr(m, cursor + 0x24, voice);
                break;
            }
            let next = rd(m, cursor + 0x28);
            if next != 0 {
                cursor = next;
                continue;
            }
            wr(m, cursor + 0x28, voice);
            wr(m, voice + 0x24, cursor);
            wr(m, voice + 0x28, 0);
            wr(m, p + 8, voice);
            break;
        }
    }
    route_or_host(m, h, 0x18423c, &[p + 16]);
    let inner = rd(m, voice + 0x68);
    wb(m, inner + 13, 2);
    wh(m, inner + 4, rh(m, inner + 4).wrapping_add(1));
    wr(m, voice + 0x2c, callback);
    wr(m, voice + 0x30, arg);
    voice
}

fn sdk_queue_append(m: &mut Memory<'_>, h: &mut impl Calls, outer: u32, node: u32) {
    let p = rd(m, outer + 0x68);
    if rd(m, node + 4) == 0 {
        wb(m, node + 17, 3);
        return;
    }
    wr(m, node + 20, 0);
    wb(m, node + 17, 1);
    route_or_host(m, h, 0x1840fc, &[p + 0x70]);
    let mut cursor = rd(m, p + 0x30);
    if cursor == 0 {
        wr(m, p + 0x30, node);
    } else {
        while rd(m, cursor + 20) != 0 {
            cursor = rd(m, cursor + 20);
        }
        wr(m, cursor + 20, node);
    }
    let mut serial = rh(m, p + 6);
    if serial == 0 {
        serial = 1;
        wh(m, p + 6, serial);
    }
    wh(m, node + 18, serial);
    wh(m, p + 6, serial.wrapping_add(1));
    route_or_host(m, h, 0x18423c, &[p + 0x70]);
}

fn audio_start_fields(m: &mut Memory<'_>, h: &mut impl Calls, p: u32, frequency: u32) {
    let outer = sdk_voice_create(m, h, rd(m, 0x75b3d0), 0, 0);
    assert!(outer != 0, "SDK voice allocation");
    wr(m, p + 4, outer);
    let inner = rd(m, outer + 0x68);
    wh(m, inner + 0x1e, (rh(m, inner + 0x1e) & !15) | 5);
    wr(m, outer + 0x18, frequency);
    wr(m, inner + 0x20, frequency);
    wh(m, inner + 0x6c, rh(m, inner + 0x6c) | 2);
    wr(m, outer + 0x1c, rd(m, 0x75b3d4));
    wr(m, inner + 0x24, rd(m, 0x75b3d4));
    let mode = rb(m, p + 0xa9);
    wb(m, outer + 7, if mode != 0 { 2 } else { 0 });
    wb(m, inner + 15, rb(m, outer + 7));
    if mode != 0 {
        let pair = rd(m, 0x75b3d8);
        for (src, dst) in [(0, 12), (4, 16)] {
            wr(m, outer + dst, rd(m, pair + src));
            wr(m, inner + dst + 8, rd(m, pair + src));
        }
        wh(m, outer + 20, rh(m, pair + 8));
        wh(m, inner + 28, rh(m, pair + 8));
        wb(m, outer + 7, rb(m, outer + 7) | 2);
        wb(m, inner + 15, rb(m, outer + 7));
    }
    wh(m, inner + 0x6c, rh(m, inner + 0x6c) | 0x14);
    wr(m, outer + 0x64, rf(m, p + 8).to_bits());
    wr(m, inner + 0x68, rf(m, p + 8).to_bits());
    wh(m, inner + 0x6c, rh(m, inner + 0x6c) | 1);
    wr(m, p + 0x1c, 0);
    wr(m, p + 0x24, 1);
    let count = rd(m, p + 0x18);
    assert!(count != 0);
    let next = 1 % count;
    wr(m, p + 0x20, next);
    let rows = rd(m, p + 0x90);
    wr(m, p + 0x78, rd(m, rows));
    wr(m, p + 0x7c, rd(m, rows + next * 4));
    for off in [0x28, 0xa0, 0xa4, 0x94] {
        wr(m, p + off, 0);
    }
    for i in 0..count {
        let row = rd(m, p + 0x14) + i * 24;
        fn_19be58(m, row);
        let buffer = rd(m, rows + i * 4);
        wb(m, buffer, 0);
        wb(m, buffer + 1, 0);
        wr(m, row, buffer);
        wr(m, row + 4, 1);
        wb(m, row + 16, 0);
        sdk_queue_append(m, h, outer, row);
    }
}
