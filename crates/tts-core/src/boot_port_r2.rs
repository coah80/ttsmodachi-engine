

use crate::boot::{Calls, Memory};
use crate::cold_boot::{fn_129778, RouteHost};

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

fn rh(m: &Memory<'_>, addr: u32) -> u16 {
    let o = addr.wrapping_sub(m.base) as usize;
    u16::from_le_bytes([m.bytes[o], m.bytes[o + 1]])
}

fn wh(m: &mut Memory<'_>, addr: u32, v: u16) {
    let o = addr.wrapping_sub(m.base) as usize;
    m.bytes[o..o + 2].copy_from_slice(&v.to_le_bytes());
}

pub fn route_or_host(m: &mut Memory<'_>, h: &mut impl Calls, f: u32, a: &[u32]) -> u32 {
    if let Some(r) = crate::cold_boot::route_call(m, &mut *h, f, a) {
        return r;
    }
    h.call(m, f, a)
}

struct Prealloc {
    object: u32,
    size: u32,
    result: u32,
}
impl crate::frontend::HeapVtable for Prealloc {
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
            crate::cold_boot::fn_173dc0(
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
    crate::log_live::fn_1738e8(
        m,
        logger,
        rd(m, 0x190E84),
        4,
        0,
        0x191A64,
        scratch,
        &mut crate::cold_boot::RouteHost { inner: &mut *h },
    );
    let r = crate::cold_boot::fn_19611c(m, rd(m, owner.wrapping_add(8)), rd(m, owner.wrapping_add(12)), scratch.wrapping_add(8));
    if r >= 0x80000000 {
        return crate::cold_boot::fn_160fec(r);
    }
    if user == 0 {
        return 0x80000006;
    }
    let u4 = rd(m, user.wrapping_add(4));
    if u4 == 0 {
        return 0x80000012;
    }
    let r = fn_18f24c(m, &mut *h, owner, rd(m, 0x191A7C), 2, scratch.wrapping_add(4), scratch.wrapping_add(8));
    if r >= 0x80000000 {
        return crate::cold_boot::fn_160fec(r);
    }
    wr(m, owner.wrapping_add(0x170), u4);
    wr(m, owner.wrapping_add(0x16C), rd(m, user));
    let _ = crate::engine_live::fn_1959dc_m(m, owner, rb(m, scratch.wrapping_add(4)) as u32, &mut *h);
    wr(m, scratch, 0x191A94);
    crate::log_live::fn_1738e8(
        m,
        logger,
        rd(m, 0x190E84),
        4,
        0,
        0x191A94,
        scratch,
        &mut crate::cold_boot::RouteHost { inner: &mut *h },
    );
    crate::cold_boot::fn_160fec(r)
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
    crate::boot_part_d::fn_176fd4(m, scratch, [rd(m, 0x176FE8), rd(m, 0x176FEC)]);
    if tag != rd(m, 0x1940CC) || owner == 0 {
        return 0x80000008;
    }
    let logger = rd(m, rd(m, owner.wrapping_add(0x10)).wrapping_add(0x10));
    wr(m, scratch, 0x1945EC);
    crate::log_live::fn_1738e8(
        m,
        logger,
        rd(m, 0x19460C),
        4,
        0,
        0x1945EC,
        scratch,
        &mut crate::cold_boot::RouteHost { inner: &mut *h },
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
    let r = fn_18f24c(m, &mut *h, owner, rd(m, 0x194618), 2, scratch.wrapping_add(0x14), scratch.wrapping_add(0x10));
    if r >= 0x80000000 {
        return crate::cold_boot::fn_160fec(r);
    }
    let mut r6v = crate::frontend::copy_utf16_as_ascii(&mut *m.bytes, Some(r6), scratch.wrapping_add(0x18), 0x80);
    if r6v >= 0x80000000 {
        wr(m, r4, 0);
        wr(m, r4.wrapping_add(4), 0);
    } else {
        crate::boot_part_d::fn_176fd4(m, scratch.wrapping_add(0xA0), [rd(m, 0x176FE8), rd(m, 0x176FEC)]);
        wr(m, scratch.wrapping_add(0x10), scratch.wrapping_add(0x98));
        wr(m, scratch.wrapping_add(8), rd(m, scratch.wrapping_add(0xA0)));
        wr(m, scratch.wrapping_add(12), rd(m, scratch.wrapping_add(0xA4)));
        wr(m, scratch, r7);
        wr(m, scratch.wrapping_add(4), r8);
        let vp = rd(m, rd(m, rd(m, owner.wrapping_add(4)).wrapping_add(0xBC)).wrapping_add(0x24));
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
    let _ = crate::engine_live::fn_1959dc_m(m, owner, rb(m, scratch.wrapping_add(0x14)) as u32, &mut *h);
    wr(m, scratch, 0x194628);
    crate::log_live::fn_1738e8(
        m,
        logger,
        rd(m, 0x19460C),
        4,
        0,
        0x194628,
        scratch,
        &mut crate::cold_boot::RouteHost { inner: &mut *h },
    );
    crate::cold_boot::fn_160fec(r6v)
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
    let sp = crate::cold_boot::fn_129778(m, &mut *h, 0x28);
    if sp == 0 {
        let _ = route_or_host(m, &mut *h, 0x176F5C, &[rd(m, o.wrapping_add(12))]);
        return 0x8000000A;
    }
    wr(m, sp.wrapping_add(0x20), 0);
    let r = route_or_host(m, &mut *h, 0x1966AC, &[rd(m, o.wrapping_add(16)), key, sp.wrapping_add(0x20)]);
    r5 = r;
    if r < 0x80000000 {
        route_or_host(m, &mut *h, 0x195AE8, &[rd(m, sp.wrapping_add(0x20)), key, 0x14]);
    } else {
        crate::cold_boot::fn_195bec(m, sp, 0, 0x20);
        route_or_host(m, &mut *h, 0x195AE8, &[sp, key, 0x14]);
        wh(m, sp.wrapping_add(0x1C), 0);
        r5 = route_or_host(m, &mut *h, 0x19668C, &[rd(m, o.wrapping_add(16)), key, sp]);
    }
    let r0 = route_or_host(m, &mut *h, 0x176F5C, &[rd(m, o.wrapping_add(12))]);
    if r0 >= 0x80000000 && r5 < 0x80000000 {
        r5 = r0;
    }
    r5
}

pub fn fn_1966ac(m: &mut Memory<'_>, h: &mut impl Calls, map: u32, key: u32, out: u32) -> u32 {
    if map == 0 || key == 0 || out == 0 {
        return 0x89002007;
    }
    let node = tree_find(m, &mut *h, map.wrapping_add(24), key);
    if node == 0 {
        0x89002014
    } else {
        wr(m, out, rd(m, node.wrapping_add(20)));
        0
    }
}

fn tree_find(m: &mut Memory<'_>, h: &mut impl Calls, tree: u32, key: u32) -> u32 {
    let cmp_fn = rd(m, rd(m, tree.wrapping_add(36)).wrapping_add(8));
    let mut node = rd(m, tree);
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

pub fn fn_19668c(
    m: &mut Memory<'_>,
    h: &mut impl Calls,
    map: u32,
    key: u32,
    incoming: u32,
) -> u32 {
    if map == 0 || key == 0 || incoming == 0 {
        return 0x89002007;
    }
    let s = crate::cold_boot::fn_129778(m, &mut *h, 4);
    if s == 0 {
        return 0x8900200A;
    }
    tree_insert_node(m, &mut *h, map.wrapping_add(24), key, incoming, s)
}

fn tree_insert_node(
    m: &mut Memory<'_>,
    h: &mut impl Calls,
    tree: u32,
    key: u32,
    incoming: u32,
    s: u32,
) -> u32 {
    let heap = rd(m, tree.wrapping_add(32));
    let vt = rd(m, tree.wrapping_add(36));
    let node = route_or_host(m, &mut *h, 0x196394, &[heap, 24]);
    if node == 0 {
        return 0x8900200A;
    }
    wr(m, s, 0);
    let r = if incoming == 0 {
        0
    } else {
        route_or_host(m, &mut *h, rd(m, vt.wrapping_add(12)), &[incoming, heap, s])
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
    #[allow(dead_code)]
    n: usize,
    at: usize,
    key: u32,
    heap: u32,
    node: u32,
    rc: u32,
    created: u32,
}

impl crate::frontend::TreeOps for LinkResults {
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
        path[n] = (k, cmp);
        n += 1;
        if rb(m, tree.wrapping_add(40)) == 0 && cmp == 0 {
            return 0x89002002;
        }
        x = rd(m, x.wrapping_add(if cmp < 0 { 0 } else { 4 }));
    }
    wr(m, s, rd(m, node.wrapping_add(16)));
    let rc = route_or_host(m, &mut *h, rd(m, vt), &[key, heap, s]);
    let created = rd(m, s);
    let mut ops = LinkResults { path, n, at: 0, key, heap, node, rc, created };
    crate::frontend::tree_insert(&mut *m.bytes, tree, node, key, &mut ops)
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
    let mut vt = Prealloc { object, size: n, result: allocated };
    crate::frontend::clone_str(&mut *m.bytes, Some(src), Some(heap), Some(out), &mut vt)
}

pub fn fn_196394(m: &mut Memory<'_>, h: &mut impl Calls, heap: u32, size: u32) -> u32 {
    if heap == 0 || size == 0 {
        return 0;
    }
    let target = rd(m, rd(m, heap));
    let args = [rd(m, heap.wrapping_add(4)), size];
    if let Some(r) = crate::cold_boot::route_call(m, &mut *h, target, &args) {
        r
    } else {
        h.call(m, target, &args)
    }
}

pub fn fn_176054(
    m: &mut Memory<'_>,
    h: &mut impl Calls,
    slot: u32,
    src: u32,
    n: u32,
    heap: u32,
) -> u32 {
    if slot == 0 {
        return 0x8B702007;
    }
    if n != 0 && rd(m, slot.wrapping_add(4)) == 0 {
        let allocated = crate::boot_port_r2::fn_196394(m, &mut *h, heap, n);
        let object = if heap == 0 { 0 } else { rd(m, heap.wrapping_add(4)) };
        let mut vt = Prealloc { object, size: n, result: allocated };
        crate::frontend::store_sized(&mut *m.bytes, Some(slot), src, n, heap, &mut vt)
    } else if (n != 0 && rd(m, slot.wrapping_add(4)) < n)
        || (n == 0 && rd(m, slot) != 0 && rd(m, slot.wrapping_add(4)) != 0)
    {
        h.call(m, 0x176054, &[slot, src, n, heap])
    } else {
        let mut vt = Prealloc { object: 0, size: 0, result: 0 };
        crate::frontend::store_sized(&mut *m.bytes, Some(slot), src, n, heap, &mut vt)
    }
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
    crate::boot_port_1::fn_176c58(m, value, buf, 10);
    let n = crate::boot_port_1::fn_195d90(m, buf).wrapping_add(1);
    route_or_host(m, &mut *h, 0x175170, &[obj, key, buf, n, 1])
}

pub fn fn_1759e0(m: &mut Memory<'_>, h: &mut impl Calls, obj: u32, key: u32, s: u32) -> u32 {

    let s = if s == 0 { 0x1759DCu32 } else { s };
    let n = crate::boot_port_1::fn_195d90(m, s).wrapping_add(1);
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
        crate::log_live::fn_1738e8(m, logger, rd(m, 0x193470), 4, 0, 0x193450, sp, &mut rh);
    }
    if r4 == 0 {
        return 0x80000006;
    }
    let mut r7 = crate::cold_boot::fn_19611c(m, rd(m, r6.wrapping_add(8)), rd(m, r6.wrapping_add(12)), sp.wrapping_add(0x130));
    if r7 >= 0x80000000 {
        return crate::cold_boot::fn_160fec(r7);
    }

    route_or_host(m, &mut *h, 0x195AE8, &[sp.wrapping_add(0x14), r6.wrapping_add(0x36), 0x114]);
    let mut sl = rb(m, r6.wrapping_add(0x34)) as u32;
    wr(m, sp.wrapping_add(0x1b8), rb(m, r6.wrapping_add(0x35)) as u32);
    let r2err = rd(m, 0x192CAC);

    macro_rules! logd {
        ($c:expr, $d:expr, $s:expr, $v:expr) => {{
            wr(m, sp, $s);
            wr(m, sp.wrapping_add(4), $v);
            crate::cold_boot::fn_173dc0(m, &mut *h, logger, rd(m, 0x193470), $c, $d, sp, sp.wrapping_add(4));
        }};
    }

    macro_rules! exit20 {
        () => {{
            if rd(m, sp.wrapping_add(0x1c4)) != 0 && rb(m, sp.wrapping_add(0x1bc)) != 0 {
                let _ = crate::engine_live::fn_1959dc_m(m, r6, rb(m, sp.wrapping_add(0x1bc)) as u32, &mut *h);
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
        return loop2(m, h, logger, r6, r4, r8, sp, &mut sb, &mut fp, &mut sl, &mut r7);
    }

    let mut i = 0u32;
    loop {
        if i >= r8 {
            break;
        }
        let v = rb(m, r4.wrapping_add(i.wrapping_mul(2))) as u32;
        if v == 1 || v == 7 || v == 0xf || v == 8 || v == 0x13 {
            sb = 1;
            return loop2(m, h, logger, r6, r4, r8, sp, &mut sb, &mut fp, &mut sl, &mut r7);
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
                let _ = crate::engine_live::fn_1959dc_m(m, r6, rb(m, sp.wrapping_add(0x1bc)) as u32, &mut *h);
            }
            return exit38(m, h, logger, *r7, sp);
        }};
    }
    if r8 > 0 {
        let mut i = 0u32;
        loop {
            let v = rb(m, r4.wrapping_add(i.wrapping_mul(2))) as u32;
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
        let v = rb(m, r4.wrapping_add(r5.wrapping_mul(2))) as u32;
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
    let b = rb(m, r4.wrapping_add(r5.wrapping_mul(2))) as u32;
    wr(m, sp, 0x193BF8);
    wr(m, sp.wrapping_add(4), b);
    wr(m, sp.wrapping_add(8), 0x193BF0);
    wr(m, sp.wrapping_add(12), *r7);
    crate::cold_boot::fn_173dc0(m, &mut *h, logger, rd(m, 0x193470), rd(m, 0x193484), 0x193C04, sp, sp.wrapping_add(4));
}

#[allow(clippy::too_many_arguments)]
fn case_frequency(m:&mut Memory<'_>,h:&mut impl Calls,logger:u32,r6:u32,r4:u32,r5:u32,sp:u32,sb:&mut u32,r7:&mut u32)->Flow {
    let v=rh(m,r4+r5*0x102+2);
    if matches!(v,8|11|16|22) {
        *sb=1;wh(m,sp+0x74,v);wr(m,sp+0x1cc,1);
        return check_tail(m,h,logger,r6,r4,r5,sp,r7);
    }
    wr(m,sp,0x1934b0);wr(m,sp+4,v as u32);
    crate::cold_boot::fn_173dc0(m,h,logger,rd(m,0x193470),rd(m,0x1934c4),0x1934bc,sp,sp+4);
    *r7=rd(m,0x1934c8);Flow::Exit20
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
    let v = rh(m, r4.wrapping_add(r5.wrapping_mul(2)).wrapping_add(2)) as u32;
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
    let src = r4.wrapping_add(r5.wrapping_mul(2)).wrapping_add(2);
    let _ = crate::frontend::copy_utf16_as_ascii(&mut *m.bytes, Some(src), sp.wrapping_add(0x14), 0x40);
    wh(m, sp.wrapping_add(0x10), 0);
    *sb = 1;
    if crate::boot_port_1::fn_195d90(m, sp.wrapping_add(0x14)) != 3 {
        return check_tail(m, h, logger, r6, r4, r5, sp, r7);
    }
    let r0 = route_or_host(
        m,
        &mut *h,
        0x192E5C,
        &[rd(m, r6.wrapping_add(16)), sp.wrapping_add(0x12c), sp.wrapping_add(0x10), sp.wrapping_add(0x14)],
    );
    if r0 >= 0x80000000 {

        return check_tail(m, h, logger, r6, r4, r5, sp, r7);
    }
    copy_bytes(m, sp.wrapping_add(0x14), rd(m, sp.wrapping_add(0x12c)));
    route_or_host(m, &mut *h, 0x196428, &[rd(m, rd(m, r6.wrapping_add(16)).wrapping_add(4)), rd(m, sp.wrapping_add(0x12c))]);
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
    let src = r4.wrapping_add(r5.wrapping_mul(2)).wrapping_add(2);
    let _ = crate::frontend::copy_utf16_as_ascii(&mut *m.bytes, Some(src), sp.wrapping_add(off), cap);
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
    let src = r4.wrapping_add(r5.wrapping_mul(2)).wrapping_add(2);
    let _ = crate::frontend::copy_utf16_as_ascii(&mut *m.bytes, Some(src), sp.wrapping_add(0x76), 0x20);
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
    let v = rh(m, r4.wrapping_add(r5.wrapping_mul(2)).wrapping_add(2)) as u32;
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
    crate::cold_boot::fn_173dc0(m, &mut *h, logger, rd(m, 0x193470), rd(m, 0x193B80), 0x193B78, sp, sp.wrapping_add(4));
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
    let v = rh(m, r4.wrapping_add(r5.wrapping_mul(2)).wrapping_add(2)) as u32;
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
    crate::cold_boot::fn_173dc0(m, &mut *h, logger, rd(m, 0x193470), rd(m, 0x1937D8), 0x1934C0, sp, sp.wrapping_add(4));
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
    let v = rh(m, r4.wrapping_add(r5.wrapping_mul(2)).wrapping_add(2)) as u32;
    if v == 3 || v == 0xc {
        *sl = v & 0xff;
        return check_tail(m, h, logger, r6, r4, r5, sp, r7);
    }
    wr(m, sp, 0x19380C);
    wr(m, sp.wrapping_add(4), v);
    crate::cold_boot::fn_173dc0(m, &mut *h, logger, rd(m, 0x193470), rd(m, 0x193818), 0x19347C, sp, sp.wrapping_add(4));
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
    let v = rh(m, r4.wrapping_add(r5.wrapping_mul(2)).wrapping_add(2)) as u32;
    if v == 1 || v == 2 {
        wr(m, sp.wrapping_add(0x1b8), v & 0xff);
        return check_tail(m, h, logger, r6, r4, r5, sp, r7);
    }
    wr(m, sp, 0x193820);
    wr(m, sp.wrapping_add(4), v);
    crate::cold_boot::fn_173dc0(m, &mut *h, logger, rd(m, 0x193470), rd(m, 0x19382C), 0x1934C0, sp, sp.wrapping_add(4));
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
    let v = rh(m, r4.wrapping_add(r5.wrapping_mul(2)).wrapping_add(2)) as u32;
    wh(m, sp.wrapping_add(0x10), v as u16);
    *r7 = route_or_host(
        m,
        &mut *h,
        0x192E5C,
        &[rd(m, r6.wrapping_add(16)), sp.wrapping_add(0x12c), sp.wrapping_add(0x10), sp.wrapping_add(0x1c0)],
    );
    if *r7 >= 0x80000000 {
        log_at_a28(m, h, logger, r6, r4, r5, sp, r7);
        return Flow::Exit20;
    }
    copy_bytes(m, sp.wrapping_add(0x14), rd(m, sp.wrapping_add(0x12c)));
    route_or_host(m, &mut *h, 0x196428, &[rd(m, rd(m, r6.wrapping_add(16)).wrapping_add(4)), rd(m, sp.wrapping_add(0x12c))]);
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
    let v = rh(m, r4.wrapping_add(r5.wrapping_mul(2)).wrapping_add(2)) as u32;
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
    let v = rh(m, r4.wrapping_add(r5.wrapping_mul(2)).wrapping_add(2)) as u32;
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
    let v = rh(m, r4.wrapping_add(r5.wrapping_mul(2)).wrapping_add(2)) as u32;
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
    let v = rh(m, r4.wrapping_add(r5.wrapping_mul(2)).wrapping_add(2)) as u32;
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
    let src = r4.wrapping_add(r5.wrapping_mul(2)).wrapping_add(2);
    let _ = crate::frontend::copy_utf16_as_ascii(&mut *m.bytes, Some(src), sp.wrapping_add(0x138), 0x80);
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
    let src = r4.wrapping_add(r5.wrapping_mul(2)).wrapping_add(2);
    let _ = crate::frontend::copy_utf16_as_ascii(&mut *m.bytes, Some(src), sp.wrapping_add(0x138), 0x80);
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
    let b = rb(m, r4.wrapping_add(r5.wrapping_mul(2))) as u32;
    wr(m, sp, 0x193BF8);
    wr(m, sp.wrapping_add(4), b);
    crate::cold_boot::fn_173dc0(m, &mut *h, logger, rd(m, 0x193470), rd(m, 0x193C18), 0x193C10, sp, sp.wrapping_add(4));
    *r7 = rd(m, 0x1934C8);
    if rd(m, sp.wrapping_add(0x1c4)) != 0 && rb(m, sp.wrapping_add(0x1bc)) != 0 {
        let _ = crate::engine_live::fn_1959dc_m(m, r6, rb(m, sp.wrapping_add(0x1bc)) as u32, &mut *h);
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
                let _ = crate::engine_live::fn_1959dc_m(m, r6, rb(m, sp.wrapping_add(0x1bc)) as u32, &mut *h);
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
    crate::cold_boot::fn_173dc0(m, &mut *h, logger, rd(m, 0x193470), rd(m, 0x193C1C), 0x193C08, sp, sp.wrapping_add(4));
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
            indirect_call(m, h, logger, r6, sp, r4lit, r7);
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
    _sp: u32,
    _r4lit: u32,
    r7: &mut u32,
) {
    let tgt = rd(m, rd(m, rd(m, r6.wrapping_add(4)).wrapping_add(0xbc)).wrapping_add(0x60));
    *r7 = route_or_host(m, &mut *h, tgt, &[rd(m, r6.wrapping_add(0x2c)), rd(m, r6.wrapping_add(0x30)), 0, 0]);
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
    crate::cold_boot::fn_173dc0(m, &mut *h, logger, rd(m, 0x193470), rd(m, 0x193D6C), 0x193C08, sp, sp.wrapping_add(4));
    if rd(m, sp.wrapping_add(0x1c4)) != 0 && rb(m, sp.wrapping_add(0x1bc)) != 0 {
        let _ = crate::engine_live::fn_1959dc_m(m, r6, rb(m, sp.wrapping_add(0x1bc)) as u32, &mut *h);
    }
    exit38(m, h, logger, *r7, sp)
}

#[allow(clippy::too_many_arguments)]

fn exit38(m: &mut Memory<'_>, h: &mut impl Calls, logger: u32, r7: u32, sp: u32) -> u32 {
    wr(m, sp, 0x193D70);
    {
        let mut rh = RouteHost { inner: &mut *h };
        crate::log_live::fn_1738e8(m, logger, rd(m, 0x193470), 4, 0, 0x193D70, sp, &mut rh);
    }
    crate::cold_boot::fn_160fec(r7)
}

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
        indirect_call(m, h, logger, r6, sp, rd(m, 0x193D6C), r7);
        if *r7 >= 0x80000000 {
            return tail_c28_logged(m, h, logger, r6, sp, r7);
        }
    }
    wb(m, r6.wrapping_add(0x34), (*sl & 0xff) as u8);
    wb(m, r6.wrapping_add(0x35), rb(m, sp.wrapping_add(0x1b8)));
    if *sb == 0 {
        if rd(m, sp.wrapping_add(0x1c4)) != 0 && rb(m, sp.wrapping_add(0x1bc)) != 0 {
            let _ = crate::engine_live::fn_1959dc_m(m, r6, rb(m, sp.wrapping_add(0x1bc)) as u32, &mut *h);
        }
        return exit38(m, h, logger, *r7, sp);
    }
    route_or_host(m, &mut *h, 0x195AE8, &[r6.wrapping_add(0x36), sp.wrapping_add(0x14), 0x114]);
    if rd(m, sp.wrapping_add(0x1c4)) != 0 && rb(m, sp.wrapping_add(0x1bc)) != 0 {
        let _ = crate::engine_live::fn_1959dc_m(m, r6, rb(m, sp.wrapping_add(0x1bc)) as u32, &mut *h);
    }
    exit38(m, h, logger, *r7, sp)
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

pub fn fn_1517e8(m: &mut Memory<'_>, h: &mut impl Calls, ctx: u32, r1: u32, out1: u32, out2: u32) -> u32 {
    let _ = out1;
    wr(m, out2, 0);
    let r = crate::boot_port_1::fn_176f94(m, ctx, r1, rd(m, 0x1518F0), 0x14C);
    if r >= 0x80000000 {
        if r == 0x400000 {
            return rd(m, 0x1518F4);
        }
        return r;
    }
    let r = crate::cold_boot::fn_19611c(m, rd(m, ctx), rd(m, ctx.wrapping_add(4)), out1);
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
        return crate::cold_boot::fn_173dc0(
            m, &mut *h, logger, rd(m, 0x17EEE8), rd(m, 0x15B0C0), rd(m, 0x15B0CC),
            scratch, scratch.wrapping_add(4),
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
        let e = rd(m, rd(m, ctx.wrapping_add(8)).wrapping_add(r6.wrapping_mul(4)));
        let f = rd(m, rd(m, e.wrapping_add(8)).wrapping_add(0x1c));
        if f != 0 {
            let r = route_or_host(
                m, &mut *h, f,
                &[rd(m, e.wrapping_add(0xc)), rd(m, e.wrapping_add(0x10)), slot],
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
                    let sb = crate::boot_port_1::fn_195d90(m, cand);
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
    crate::boot_part_d::fn_176fd4(m, sp, [p0, p1]);
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
        crate::boot_part_d::fn_176fd4(m, sp, [p0, p1]);
        let (q0, q1) = (rd(m, sp), rd(m, sp.wrapping_add(4)));
        wr(m, out, q0);
        wr(m, out.wrapping_add(4), q1);
    }
    let mut r4 = fn_17ef1c(m, &mut *h, ctx, u7, sp.wrapping_add(0x28), sp.wrapping_add(0x30));
    if r4 >= 0x80000000 {
        wr(m, sp.wrapping_add(8), 0x17F220);
        wr(m, sp.wrapping_add(12), r4);
        wr(m, sp, 0x17F22C);
        wr(m, sp.wrapping_add(4), key);
        let logger = rd(m, r8.wrapping_add(0x10));
        crate::cold_boot::fn_173dc0(
            m, &mut *h, logger, rd(m, 0x17EEE8), rd(m, 0x17F23C), 0x17F230,
            sp, sp.wrapping_add(4),
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
        crate::cold_boot::fn_173dc0(
            m, &mut *h, logger, rd(m, 0x17EEE8), rd(m, 0x17F274), 0x17F264,
            sp, sp.wrapping_add(4),
        );
        return sl;
    }
    wr(m, sp.wrapping_add(0x10), sp.wrapping_add(0x20));
    wr(m, sp.wrapping_add(8), f16);
    wr(m, sp.wrapping_add(12), f19);
    wr(m, sp, p0);
    wr(m, sp.wrapping_add(4), p1);
    let (v0, v1) = (rd(m, vres.wrapping_add(0xc)), rd(m, vres.wrapping_add(0x10)));
    r4 = route_or_host(m, &mut *h, ip, &[v0, v1, u7, key]);
    if r4 >= 0x80000000 {
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
    wr(m, sp.wrapping_add(4), rd(m, rd(m, sp.wrapping_add(0x28)).wrapping_add(4)));
    wr(m, sp.wrapping_add(8), 0x17F22C);
    wr(m, sp.wrapping_add(12), key);
    let _ = f16;
    let logger = rd(m, r8.wrapping_add(0x10));
    crate::cold_boot::fn_173dc0(
        m, &mut *h, logger, rd(m, 0x17EEE8), rd(m, 0x17F260), 0x17F24C,
        sp, sp.wrapping_add(4),
    );
    r4
}

#[allow(clippy::too_many_arguments)]
pub fn fn_151914(
    m: &mut Memory<'_>,
    h: &mut impl Calls,
    a0: u32,
    a1: u32,
    a2: u32,
    a3: u32,
    c4: u32,
    c5: u32,
    w8: u32,
    w12: u32,
    w16: u32,
) -> u32 {
    let sb = a3;
    let sp = fn_129778(m, &mut *h, 0x20);
    if sp == 0 {
        return 0x8900200A;
    }
    wr(m, sp.wrapping_add(0x18), 0);
    wr(m, sp.wrapping_add(0x14), 0);
    if sb != 0 || (c4 != 0 && c5 != 0) {
        if a2 != 0 && w16 != 0 {
            wr(m, w16, 0);
            wr(m, w16.wrapping_add(4), 0);
            let r = fn_1517e8(m, &mut *h, a0, a1, sp.wrapping_add(0x14), sp.wrapping_add(0x18));
            if r >= 0x80000000 {
                return r;
            }

            return fn_151914_open(m, h, sb, a2, c4, c5, w8, w12, w16, sp);
        }
    }
    rd(m, 0x151BCC)
}

#[allow(clippy::too_many_arguments)]
fn fn_151914_open(
    m: &mut Memory<'_>,
    h: &mut impl Calls,
    sb: u32,
    a2: u32,
    c4: u32,
    c5: u32,
    w8: u32,
    w12: u32,
    w16: u32,
    sp: u32,
) -> u32 {
    let mut sl = 0u32;
    let fp = rd(m, sp.wrapping_add(0x14));
    let r6 = rd(m, sp.wrapping_add(0x18));
    wr(m, sp, 0x151B50);
    {
        let r0 = if sb != 0 { sb } else { 0x151900 };
        wr(m, sp.wrapping_add(8), r0);
        wr(m, sp.wrapping_add(12), c4);
        wr(m, sp.wrapping_add(16), c5);
    }
    let logger = rd(m, fp.wrapping_add(0x10));
    {
        let mut rh = RouteHost { inner: &mut *h };
        crate::log_live::fn_1738e8(m, logger, rd(m, 0x151B80), 4, 0, 0x151B50, sp, &mut rh);
    }
    wr(m, sp.wrapping_add(4), 0x151B84);
    wr(m, sp, 2);
    let mut r4 = route_or_host(m, &mut *h, 0x176F2C, &[rd(m, r6.wrapping_add(0x2c))]);
    if r4 >= 0x80000000 {

        wr(m, sp, 0x151BA0);
        wr(m, sp.wrapping_add(8), 0x151B98);
        wr(m, sp.wrapping_add(12), r4);
        let logger = rd(m, fp.wrapping_add(0x10));
        crate::cold_boot::fn_173dc0(
            m, &mut *h, logger, rd(m, 0x151B80), rd(m, 0x151BB8), 0x151BAC,
            sp, sp.wrapping_add(8),
        );
        r4 = rd(m, 0x151BD4);
    } else {
        let b = rb(m, r6.wrapping_add(0x30)) as u32;
        if b < 7 && !matches!(b, 0 | 2) {
            sl = b;
            wb(m, r6.wrapping_add(0x30), (rd(m, sp) & 0xff) as u8);
        } else {
            wr(m, sp, 0x151BA0);
            let logger = rd(m, fp.wrapping_add(0x10));
            crate::cold_boot::fn_173dc0(
                m, &mut *h, logger, rd(m, 0x151B80), rd(m, 0x151BC8), 0x151BC0,
                sp, sp.wrapping_add(4),
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

    let r0 = rd(m, r6);
    if rd(m, r0.wrapping_add(0x7c)) == 0 {
        let logger = rd(m, fp.wrapping_add(0x10));
        crate::cold_boot::fn_173dc0(
            m, &mut *h, logger, rd(m, 0x151B80), rd(m, 0x151BF4), 0,
            sp, sp.wrapping_add(4),
        );
        r4 = rd(m, 0x151BCC);
    } else {
        wr(m, sp.wrapping_add(0x10), w16);
        wr(m, sp, c4);
        wr(m, sp.wrapping_add(4), c5);
        r4 = fn_17f03c(m, &mut *h, r6, 0, a2, sb, c4, c5, w8, w12, w16);
    }

    let s15 = fn_129778(m, &mut *h, 8);
    if s15 == 0 {
        return 0x8900200A;
    }
    let _ = fn_15af34(m, &mut *h, rd(m, sp.wrapping_add(0x14)), r6, sl & 0xff, s15);
    wr(m, sp.wrapping_add(8), rd(m, w16));
    wr(m, sp, 0x151BD0);
    wr(m, sp.wrapping_add(4), r4);
    let logger = rd(m, fp.wrapping_add(0x10));
    {
        let mut rh = RouteHost { inner: &mut *h };
        crate::log_live::fn_1738e8(m, logger, rd(m, 0x151B80), 4, 0, 0x151BD0, sp, &mut rh);
    }
    r4
}
