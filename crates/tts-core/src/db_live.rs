//! Live database entries with caller-owned memory.
use crate::boot::Memory;

/// Platform logging for the pool reset. Guest strings remain guest addresses.
pub trait PoolHooksM {
    fn log_pool_m(&mut self, pool: u32, blocks: u32, mem: &mut Memory<'_>);
}

/// fn_180d00. Initialize the seven-word pool descriptor.
pub fn fn_180d00(mem: &mut Memory<'_>, p: u32, ctx: u32, name: u32, initial: u32, grow: u32) {
    crate::frontend::fill_bytes(mem.bytes, p.wrapping_sub(mem.base), 0, 28);
    mem.put_word(p, ctx);
    mem.put_word(p.wrapping_add(4), name);
    mem.put_word(p.wrapping_add(12), initial);
    mem.put_word(p.wrapping_add(16), grow);
}

/// fn_180ea0. Rewind and clear every block, retaining the allocated chain.
pub fn fn_180ea0(hooks: &mut impl PoolHooksM, mem: &mut Memory<'_>, p: u32) {
    if p == 0 || mem.word(p) == 0 {
        return;
    }
    let mut block = mem.word(p.wrapping_add(8));
    let mut count = 0u32;
    while block != 0 {
        count = count.wrapping_add(1);
        block = mem.word(block);
    }
    hooks.log_pool_m(p, count, mem);
    block = mem.word(p.wrapping_add(8));
    while block != 0 {
        let base = mem.word(block.wrapping_add(4));
        mem.put_word(block.wrapping_add(8), base);
        let end = mem.word(block.wrapping_add(12));
        crate::frontend::fill_bytes(
            mem.bytes,
            base.wrapping_sub(mem.base),
            0,
            end.wrapping_sub(base),
        );
        block = mem.word(block);
    }
    mem.put_word(p.wrapping_add(24), 0);
}

/// Reentrant dependency dispatcher. Internal targets must call their existing
/// Rust ports; only platform services may be supplied by the host.
/// The live getter dispatches 176f2c/176f5c and the guest tree comparator
/// through this trait, rather than dispatching 175944 as an opaque call.
pub trait DbHooksM: PoolHooksM {
    /// Map a US diagnostic literal address for the caller's ROM region.
    fn literal_m(&self, us: u32) -> u32 {
        us
    }
    fn call_m(&mut self, entry: u32, args: &[u32], mem: &mut Memory<'_>) -> u32;
}

/// Run the landed getter on the live zero-based arena. The lock wrapper and
/// comparator are re-read for each callback, including after host writes.
pub(crate) fn get_uint(h: &mut impl DbHooksM, mem: &mut Memory<'_>, obj: u32, key: u32, out: u32) -> u32 {
    assert_eq!(mem.base, 0, "open_db_m requires a zero-based arena");
    struct Lock<'a, H> {
        host: &'a core::cell::RefCell<&'a mut H>,
        obj: u32,
    }
    impl<H: DbHooksM> crate::reentrant2::StateLockM for Lock<'_, H> {
        fn acquire_m(&mut self, bytes: &mut [u8]) -> u32 {
            let mut m = Memory { base: 0, bytes };
            let wrapper = m.word(self.obj + 4);
            self.host.borrow_mut().call_m(0x176f2c, &[wrapper], &mut m)
        }
        fn release_m(&mut self, bytes: &mut [u8]) -> u32 {
            let mut m = Memory { base: 0, bytes };
            let wrapper = m.word(self.obj + 4);
            self.host.borrow_mut().call_m(0x176f5c, &[wrapper], &mut m)
        }
    }
    let host = core::cell::RefCell::new(h);
    let mut lock = Lock { host: &host, obj };
    crate::reentrant2::get_param_uint_m(
        mem.bytes,
        (obj != 0).then_some(obj),
        key,
        out,
        &mut lock,
        |a, b, bytes| {
            let mut m = Memory { base: 0, bytes };
            let tree = m.word(obj + 8) + 24;
            let target = m.word(m.word(tree + 36) + 8);
            host.borrow_mut().call_m(target, &[a, b], &mut m) as i32
        },
    )
}

/// Regional guest addresses, obtained from the verified entry's literal pool.
/// All strings stay addresses across mutable callbacks.
pub struct DbData {
    pub module: u32,
    pub open_begin: u32,
    pub open_end: u32,
    pub open_detail: u32,
    pub reset_begin: u32,
    pub reset_end: u32,
    pub pool_name_word: u32,
    pub keys: [u32; 5],
}

fn trace(
    h: &mut impl DbHooksM,
    mem: &mut Memory<'_>,
    log: u32,
    module: u32,
    level: u32,
    format: u32,
    values: &[u32],
) {
    let mut args = [0u32; 12];
    args[..5].copy_from_slice(&[log, module, level, 0, format]);
    args[5..5 + values.len()].copy_from_slice(values);
    h.call_m(0x1738e8, &args[..5 + values.len()], mem);
}
fn error(h: &mut impl DbHooksM, mem: &mut Memory<'_>, log: u32, module: u32, code: u32) {
    h.call_m(0x173dc0, &[log, module, code, 0], mem);
}

/// fn_15bab0, using the current arena at every dependency boundary.
/// `scratch` is a disjoint caller-owned guest word, restored before return.
pub fn ld_reset_m(
    h: &mut impl DbHooksM,
    mem: &mut Memory<'_>,
    p: u32,
    tag: u32,
    data: &DbData,
    scratch: u32,
) -> u32 {
    if crate::frontend::validate_handle(p != 0, tag, 0xb2c2, 0x38) >= 0x80000000 {
        return 0x89202008;
    }
    let saved = mem.word(scratch);
    mem.put_word(scratch, 0);
    let r = reset_inner(h, mem, p, data, scratch);
    mem.put_word(scratch, saved);
    r
}
fn reset_inner(
    h: &mut impl DbHooksM,
    mem: &mut Memory<'_>,
    p: u32,
    d: &DbData,
    scratch: u32,
) -> u32 {
    trace(
        h,
        mem,
        mem.word(mem.word(p) + 16),
        d.module,
        4,
        d.reset_begin,
        &[],
    );
    let mut r = 0;
    match mem.word(p + 32) {
        1 => {
            if mem.word(p + 48) == 0 {
                let b = h.call_m(
                    0x1963b8,
                    &[mem.word(mem.word(p) + 4), mem.word(p + 36), 4],
                    mem,
                );
                mem.put_word(p + 48, b);
                if b == 0 {
                    error(h, mem, mem.word(mem.word(p) + 16), d.module, 0x4e20);
                    return 0x8920200a;
                }
            }
            fn_180ea0(h, mem, p + 4);
            r = h.call_m(0x15b758, &[p, 1, scratch], mem);
            if r >= 0x80000000 {
                return r;
            }
        }
        2 => {}
        _ => {
            error(h, mem, mem.word(mem.word(p) + 16), d.module, 0x4e23);
            return 0x89203e00;
        }
    }
    mem.put_byte(p + 52, 0);
    trace(
        h,
        mem,
        mem.word(mem.word(p) + 16),
        d.module,
        4,
        d.reset_end,
        &[r],
    );
    r
}

/// fn_15bc18. Memory-passing open, including its real pool initializer/reset.
/// `mem` must be zero-based, as required by reentrant2::get_param_uint_m.
/// `scratch` is four disjoint guest words, restored on every normal return.
/// a[0..2] are the unused incoming handle; a[2..4] are the environment
/// pair; a[4] is the output pair. Scratch must not alias those objects.
pub fn open_db_m(
    h: &mut impl DbHooksM,
    mem: &mut Memory<'_>,
    a: [u32; 5],
    d: &DbData,
    scratch: u32,
) -> u32 {
    let saved: [u32; 4] = core::array::from_fn(|i| mem.word(scratch + i as u32 * 4));
    for i in 0..4 {
        mem.put_word(scratch + i * 4, 0);
    }
    let r = open_inner(h, mem, a, d, scratch);
    for (i, v) in saved.into_iter().enumerate() {
        mem.put_word(scratch + i as u32 * 4, v);
    }
    r
}
fn open_inner(h: &mut impl DbHooksM, mem: &mut Memory<'_>, a: [u32; 5], d: &DbData, s: u32) -> u32 {
    let out = a[4];
    mem.put_word(out, 0);
    mem.put_word(out + 4, 0);
    let rc = h.call_m(0x19611c, &[a[2], a[3], s], mem);
    if rc >= 0x80000000 {
        return rc;
    }
    let ctx = mem.word(s);
    trace(h, mem, mem.word(ctx + 16), d.module, 4, d.open_begin, &[]);
    let p = h.call_m(0x1963b8, &[mem.word(ctx + 4), 1, 0x38], mem);
    if p == 0 {
        error(h, mem, mem.word(ctx + 16), d.module, 0x4e20);
        return 0x8920200a;
    }
    mem.put_word(p, ctx);
    mem.put_word(p + 32, 1);
    for i in 0..5 {
        let dest = if i < 3 {
            p + 36 + i as u32 * 4
        } else {
            s + 4 + (i as u32 - 3) * 4
        };
        let r = get_uint(h, mem, mem.word(ctx + 20), d.keys[i], dest);
        if r >= 0x80000000 {
            mem.put_word(dest, [0x80, 0x80, u32::MAX, 0x2800, 0x1000][i]);
        }
    }
    trace(
        h,
        mem,
        mem.word(mem.word(p) + 16),
        d.module,
        5,
        d.open_detail,
        &[
            mem.word(p + 36),
            mem.word(p + 40),
            mem.word(p + 44),
            mem.word(s + 4),
            mem.word(s + 8),
            mem.word(p + 32),
        ],
    );
    fn_180d00(
        mem,
        p + 4,
        ctx,
        mem.word(d.pool_name_word),
        mem.word(s + 4),
        mem.word(s + 8),
    );
    mem.put_word(out, p);
    mem.put_word(out + 4, 0xb2c2);
    let r = ld_reset_m(h, mem, p, 0xb2c2, d, s + 12);
    if r >= 0x80000000 {
        close_db_m(h, mem, mem.word(out), mem.word(out + 4), d);
        mem.put_word(out, 0);
        mem.put_word(out + 4, 0);
    }
    trace(h, mem, mem.word(ctx + 16), d.module, 4, d.open_end, &[r]);
    r
}

/// fn_180d3c. Pool allocation with the current arena at its heap callback.
pub fn arena_alloc_m(h: &mut impl DbHooksM, mem: &mut Memory<'_>, arena: u32, size: u32) -> u32 {
    let n = size.wrapping_add(3) & !3;
    let mut prev = 0u32;
    let mut chunk = mem.word(arena + 8);
    let fit = loop {
        if chunk == 0 {
            break None;
        }
        if mem.word(chunk + 8).wrapping_add(n) <= mem.word(chunk + 12) {
            break Some(chunk);
        }
        prev = chunk;
        chunk = mem.word(chunk);
    };
    let chunk = match fit {
        Some(chunk) => chunk,
        None => {
            let mut grow = n;
            if mem.word(arena + 20) == 0 && mem.word(arena + 12) > grow {
                grow = mem.word(arena + 12);
            } else if mem.word(arena + 16) > grow {
                grow = mem.word(arena + 16);
            }
            let heap = mem.word(mem.word(arena) + 4);
            let chunk = h.call_m(0x1963b8, &[heap, 1, grow.wrapping_add(16)], mem);
            if chunk == 0 {
                return 0;
            }
            mem.put_word(chunk + 4, chunk.wrapping_add(16));
            mem.put_word(chunk, 0);
            mem.put_word(chunk + 8, chunk.wrapping_add(16));
            mem.put_word(chunk + 12, chunk.wrapping_add(16).wrapping_add(grow));
            mem.put_word(arena + 20, mem.word(arena + 20).wrapping_add(grow));
            if prev == 0 {
                mem.put_word(arena + 8, chunk);
            } else {
                mem.put_word(prev, chunk);
            }
            chunk
        }
    };
    mem.put_word(arena + 24, mem.word(arena + 24).wrapping_add(n));
    let result = mem.word(chunk + 8);
    mem.put_word(chunk + 8, result.wrapping_add(n));
    result
}

/// fn_15b758. Sizes and layouts are guest table addresses, never retained slices.
pub fn create_record_m(
    h: &mut impl DbHooksM,
    mem: &mut Memory<'_>,
    db: u32,
    rectype: u32,
    out: u32,
    sizes: u32,
    layouts: u32,
) -> u32 {
    if rectype.wrapping_sub(1) >= 4 {
        record_error(
            h,
            mem,
            db,
            0x4e2c,
            0x15ba10,
            &[
                h.literal_m(0x15b9ec),
                h.literal_m(0x15ba00),
                h.literal_m(0x15b9f4),
                rectype,
            ],
        );
        return 0x89203e09;
    }
    let count = mem.word(db.wrapping_add(32));
    if mem.word(db.wrapping_add(44)) < count || count == 0 {
        record_error(h, mem, db, 0x4e33, 0, &[]);
        return 0x89203e10;
    }
    let capacity = mem.word(db.wrapping_add(36));
    if capacity <= count {
        let new_capacity = capacity.wrapping_add(mem.word(db.wrapping_add(40)));
        let heap = mem.word(mem.word(db).wrapping_add(4));
        trace(
            h,
            mem,
            mem.word(mem.word(db) + 16),
            h.literal_m(0x8d7b56),
            5,
            h.literal_m(0x15ba28),
            &[capacity, new_capacity],
        );
        let grown = h.call_m(
            0x1963e0,
            &[
                heap,
                mem.word(db.wrapping_add(48)),
                new_capacity.wrapping_mul(4),
            ],
            mem,
        );
        if grown == 0 {
            record_error(h, mem, db, 0x4e24, 0, &[]);
            return 0x8920200a;
        }
        mem.put_word(db.wrapping_add(48), grown);
        mem.put_word(db.wrapping_add(36), new_capacity);
        let at = grown.wrapping_add(count.wrapping_mul(4)) as usize;
        let len = new_capacity.wrapping_sub(count).wrapping_mul(4) as usize;
        let at = at - mem.base as usize;
        mem.bytes[at..at + len].fill(0);
        trace(
            h,
            mem,
            mem.word(mem.word(db) + 16),
            h.literal_m(0x8d7b56),
            5,
            h.literal_m(0x15ba60),
            &[0],
        );
    }
    let base = rectype as usize * 4;
    let size = mem.word(sizes + base as u32);
    let record = arena_alloc_m(h, mem, db.wrapping_add(4), size);
    mem.put_word(out, record);
    if record == 0 {
        record_error(h, mem, db, 0x4e20, 0, &[]);
        return 0x8920200a;
    }
    let records = mem.word(db.wrapping_add(48));
    mem.put_word(records.wrapping_add(count.wrapping_mul(4)), record);
    put_half(mem, record, count as u16);
    mem.put_byte(record.wrapping_add(2), rectype as u8);
    let new_count = count.wrapping_add(1);
    mem.put_word(db.wrapping_add(32), new_count);
    let id = u32::from(half(mem, record));
    if new_count > id {
        let found = mem.word(records.wrapping_add(id.wrapping_mul(4)));
        if found != 0 {
            let ftype = mem.bytes[mem.offset(found.wrapping_add(2))] as usize;
            let layout = ftype.wrapping_mul(0x110);
            let lbase = mem.word(layouts + layout as u32);
            let nfields = half(mem, layouts + layout as u32 + 4);
            let mut field = 0u8;
            while (field as u32) < nfields {
                let fbase = layout + (field as usize) * 20;
                let kind = half(mem, layouts + fbase as u32 + 20);
                if kind == 0 {
                    let off = mem.word(layouts + fbase as u32 + 12);
                    mem.put_word(found.wrapping_add(off.wrapping_sub(lbase)), 0);
                }
                field = field.wrapping_add(1);
            }
            mem.put_byte(db + 52, 1);
            return 0;
        }
    }
    record_error(h, mem, db, 0x4e34, 0x15ba14, &[h.literal_m(0x15baa0), id]);
    0x89203e11
}

fn record_error(
    h: &mut impl DbHooksM,
    m: &mut Memory<'_>,
    db: u32,
    code: u32,
    format: u32,
    values: &[u32],
) {
    let mut a = [0; 8];
    a[..4].copy_from_slice(&[
        m.word(m.word(db) + 16),
        h.literal_m(0x8d7b56),
        code,
        if format == 0 { 0 } else { h.literal_m(format) },
    ]);
    a[4..4 + values.len()].copy_from_slice(values);
    h.call_m(0x173dc0, &a[..4 + values.len()], m);
}

fn half(m: &Memory<'_>, p: u32) -> u32 {
    let i = m.offset(p);
    u16::from_le_bytes(m.bytes[i..i + 2].try_into().unwrap()) as u32
}
fn put_half(m: &mut Memory<'_>, p: u32, v: u16) {
    let i = m.offset(p);
    m.bytes[i..i + 2].copy_from_slice(&v.to_le_bytes());
}

struct VecHooks<'a, H>(&'a mut H);
impl<H: DbHooksM> crate::reentrant::HooksM for VecHooks<'_, H> {
    fn call_m(&mut self, f: u32, a: &[u32], m: &mut Memory<'_>) -> u32 {
        self.0.call_m(f, a, m)
    }
}
/// fn_196cfc, using bridge-3's live vector growth.
pub fn append_slot_m(h: &mut impl DbHooksM, m: &mut Memory<'_>, p: u32, out: u32) -> u32 {
    crate::reentrant::vec_grow_m(m, &mut VecHooks(h), p, out)
}
/// fn_196db0. Scratch is one disjoint caller-owned guest word, restored.
pub fn append_element_m(
    h: &mut impl DbHooksM,
    m: &mut Memory<'_>,
    p: u32,
    src: u32,
    scratch: u32,
) -> u32 {
    let saved = m.word(scratch);
    m.put_word(scratch, 0);
    let rc = if append_slot_m(h, m, p, scratch) == 0 {
        0
    } else {
        let blk = m.word(scratch);
        let n = half(m, p + 4);
        let dst = m.offset(blk);
        let from = m.offset(src);
        // The real byte-copy port, without retaining source across a hook.
        if dst < from {
            let (lo, hi) = m.bytes.split_at_mut(from);
            crate::frontend::copy_bytes(&mut lo[dst..dst + n as usize], &hi[..n as usize], n);
        } else {
            let (lo, hi) = m.bytes.split_at_mut(dst);
            crate::frontend::copy_bytes(&mut hi[..n as usize], &lo[from..from + n as usize], n);
        }
        let cb = m.word(p + 8);
        if cb != 0 && h.call_m(cb, &[src, m.word(p + 28), blk], m) == 0 {
            m.put_word(p + 20, m.word(p + 20).wrapping_sub(1));
            0
        } else {
            1
        }
    };
    m.put_word(scratch, saved);
    rc
}
/// fn_175c40. Scratch holds a transient pair and one slot word, restored.
pub fn append_owner_m(
    h: &mut impl DbHooksM,
    m: &mut Memory<'_>,
    p: u32,
    out: u32,
    scratch: u32,
) -> u32 {
    if p == 0 || out == 0 {
        return 0x8b702007;
    }
    let mut rc = h.call_m(0x176f2c, &[m.word(p + 4)], m);
    if rc >= 0x80000000 {
        return rc;
    }
    let saved = [m.word(scratch), m.word(scratch + 4)];
    m.put_word(scratch, p);
    m.put_word(scratch + 4, 0);
    if append_element_m(h, m, m.word(p + 16), scratch, scratch + 8) == 0 {
        rc = 0x8b70200a;
    } else {
        let v = m.word(p + 16);
        let words = core::array::from_fn(|i| m.word(v + i as u32 * 4));
        let count = crate::frontend::word_at5(Some(&words));
        if crate::boot_part_e::fn_1970ac(m, v, count.wrapping_sub(1), out) == 0 {
            rc = 0x8b702100;
        }
    }
    for (i, v) in saved.into_iter().enumerate() {
        m.put_word(scratch + i as u32 * 4, v);
    }
    let release = h.call_m(0x176f5c, &[m.word(p + 4)], m);
    if release >= 0x80000000 && rc < 0x80000000 {
        rc = release;
    }
    rc
}
/// fn_1964e0. Allocate with live memory, then reuse the existing initializer.
pub fn fn_1964e0_m(h: &mut impl DbHooksM, m: &mut Memory<'_>, a: [u32; 8]) -> u32 {
    let valid = a[7] != 0 && a[2..7].iter().all(|v| *v != 0);
    let value = if valid {
        h.call_m(0x1963b8, &[a[0], 1, 68], m)
    } else {
        0
    };
    struct Completed(u32);
    impl crate::frontend::MethodTable for Completed {
        fn call_at(&mut self, _: u32, _: u32, _: u32) -> u32 {
            self.0
        }
    }
    assert_eq!(m.base, 0);
    crate::frontend::create_ctx_object(
        m.bytes,
        (a[0] != 0).then_some(a[0]),
        a[1],
        a[2],
        a[3],
        a[4],
        a[5],
        a[6],
        (a[7] != 0).then_some(a[7]),
        &mut Completed(value),
    )
}
/// fn_174aac. Regional ticket words stay guest addresses.
pub fn fn_174aac_m(
    h: &mut impl DbHooksM,
    m: &mut Memory<'_>,
    src: u32,
    arg: u32,
    out: u32,
    ticket: [u32; 5],
) -> u32 {
    m.put_word(out, m.word(src));
    let old = m.word(src + 4);
    let rc = if old != 0 {
        h.call_m(0x19658c, &[old, out + 4], m)
    } else {
        fn_1964e0_m(
            h,
            m,
            [
                arg,
                0,
                ticket[0],
                ticket[1],
                ticket[2],
                ticket[3],
                ticket[4],
                out + 4,
            ],
        )
    };
    u32::from(rc < 0x80000000)
}

/// fn_180f8c. Free the pool through live heap release callbacks.
pub fn free_pool_m(h: &mut impl DbHooksM, m: &mut Memory<'_>, p: u32) {
    if p == 0 || m.word(p) == 0 {
        return;
    }
    let mut b = m.word(p + 8);
    let mut count = 0;
    while b != 0 {
        count += 1;
        b = m.word(b);
    }
    trace(
        h,
        m,
        m.word(m.word(p) + 16),
        h.literal_m(0x8d7b56),
        5,
        h.literal_m(0x8dc928),
        &[
            m.word(p + 4),
            h.literal_m(0x18105c),
            m.word(p + 24),
            m.word(p + 20),
            m.word(p + 12),
            m.word(p + 16),
            count,
        ],
    );
    b = m.word(p + 8);
    while b != 0 {
        let next = m.word(b);
        h.call_m(0x196428, &[m.word(m.word(p) + 4), b], m);
        b = next;
    }
    m.put_word(p + 8, 0);
    m.put_word(p + 24, 0);
    m.put_word(p + 20, 0);
}
/// fn_15b5f0. Close with live pool reset/release; diagnostics use regional data.
pub fn close_db_m(h: &mut impl DbHooksM, m: &mut Memory<'_>, p: u32, tag: u32, d: &DbData) -> u32 {
    if crate::frontend::validate_handle(p != 0, tag, 0xb2c2, 0x38) >= 0x80000000 {
        return 0x89202008;
    }
    let ctx = m.word(p);
    trace(
        h,
        m,
        m.word(ctx + 16),
        d.module,
        4,
        h.literal_m(0x15b724),
        &[],
    );
    free_pool_m(h, m, p + 4);
    trace(
        h,
        m,
        m.word(m.word(p) + 16),
        d.module,
        4,
        h.literal_m(0x15b5b8),
        &[],
    );
    m.put_byte(p + 52, 0);
    if m.word(p + 48) != 0 {
        fn_180ea0(h, m, p + 4);
        m.put_word(p + 32, 1);
    }
    trace(
        h,
        m,
        m.word(m.word(p) + 16),
        d.module,
        4,
        h.literal_m(0x15b5d4),
        &[0],
    );
    if m.word(p + 48) != 0 {
        h.call_m(0x196428, &[m.word(m.word(p) + 4), m.word(p + 48)], m);
    }
    h.call_m(0x196428, &[m.word(ctx + 4), p], m);
    trace(
        h,
        m,
        m.word(ctx + 16),
        d.module,
        4,
        h.literal_m(0x15b73c),
        &[0],
    );
    0
}
