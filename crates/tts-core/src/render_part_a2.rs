//! Render helpers on caller-owned memory. Pointers are byte offsets, with zero null.
//! This module uses the normalization and frontend ports for engine callees.

use crate::{frontend, normalization};

fn word(m: &[u8], p: u32) -> u32 {
    u32::from_le_bytes(m[p as usize..p as usize + 4].try_into().unwrap())
}
fn half(m: &[u8], p: u32) -> u16 {
    u16::from_le_bytes(m[p as usize..p as usize + 2].try_into().unwrap())
}
fn put_word(m: &mut [u8], p: u32, v: u32) {
    m[p as usize..p as usize + 4].copy_from_slice(&v.to_le_bytes());
}
fn entry(m: &[u8], obj: u32, requested: u32) -> Result<u32, u32> {
    if obj == 0 {
        return Err(0x8110_2007);
    }
    let t = word(m, obj.wrapping_add(12));
    let mut out = 0;
    let rc = normalization::entry_offset(
        half(m, t.wrapping_add(2)),
        word(m, t.wrapping_add(4)) as i32,
        half(m, t.wrapping_add(8)),
        half(m, obj.wrapping_add(16)),
        requested,
        &mut out,
    );
    if rc >= 0x8000_0000 {
        Err(rc)
    } else {
        Ok(out as u32)
    }
}

/// fn_255a54 / EU fn_255c10 / KR fn_255cbc.
/// Removes the current token's byte range and adjusts following token spans.
/// The unused first and fourth arguments are still checked, as in the ROM.
pub fn remove_token_span(m: &mut [u8], a: u32, b: u32, c: u32, d: u32) -> u32 {
    if a == 0 || b == 0 || c == 0 || d == 0 {
        return 0x8110_2007;
    }
    let o4 = match entry(m, c, 0) {
        Ok(p) => p,
        Err(r) => return r,
    };
    let _o8 = match entry(m, b, 0) {
        Ok(p) => p,
        Err(r) => return r,
    };
    let mut p = match entry(m, c, 0xffff) {
        Ok(p) => p,
        Err(r) => return r,
    };
    let len = word(m, p.wrapping_add(24)) as u16;
    let offset = word(m, p.wrapping_add(20)).wrapping_sub(word(m, o4.wrapping_add(20))) as u16;
    let r = normalization::delete_range(m, b, offset, len);
    if r >= 0x8000_0000 {
        return r;
    }
    put_word(m, p.wrapping_add(24), 0);
    let t = word(m, c.wrapping_add(12));
    let mut cursor = 0;
    let r = normalization::current_index(
        Some(&m[t as usize..]),
        half(m, c.wrapping_add(16)),
        Some(&mut cursor),
    );
    if r >= 0x8000_0000 {
        return r;
    }
    let mut limit = 0;
    let r = normalization::inner_field(Some(&m[t as usize..]), Some(&mut limit));
    if r >= 0x8000_0000 {
        return r;
    }
    loop {
        p = p.wrapping_add(40);
        cursor = cursor.wrapping_add(1);
        if cursor >= limit {
            break;
        }
        let end_offset = (offset as u32).wrapping_add(len as u32);
        let origin = word(m, o4.wrapping_add(20));
        let end = origin.wrapping_add(end_offset);
        if end <= word(m, p.wrapping_add(20)) {
            let v = word(m, p.wrapping_add(20)).wrapping_sub(len as u32);
            put_word(m, p.wrapping_add(20), v);
        } else {
            let start = origin.wrapping_add(offset as u32);
            put_word(m, p.wrapping_add(20), start);
            let v = start.wrapping_add(word(m, p.wrapping_add(24)));
            put_word(
                m,
                p.wrapping_add(24),
                if v < end { 0 } else { v.wrapping_sub(end) },
            );
        }
    }
    0
}

/// Caller-owned equivalents of the context resolver's two read-only globals.
pub struct ContextGlobals {
    /// D_8d7660, cached handle pair in little-endian byte order.
    pub pair: [u8; 8],
    /// Address token of D_8d7640.
    pub default_context: u32,
}

/// Native object methods remain at the host boundary. Heap release uses the
/// existing frontend HeapVtable, not a scripted engine routine.
pub trait ObjectHost: frontend::HeapVtable {
    /// Calls the method pointer with the two original integer bit patterns.
    fn destroy(&mut self, memory: &mut [u8], method: u32, first: u32, second: u32) -> u32;
}

/// fn_258018 / EU fn_2581d4 / KR fn_258280.
/// Resolves the environment, destroys the plugin instance, then releases it.
pub fn destroy_object(
    m: &mut [u8],
    obj: u32,
    globals: &ContextGlobals,
    host: &mut impl ObjectHost,
) -> u32 {
    if obj == 0 {
        return 0x8110_2007;
    }
    let handle = word(m, obj);
    let tag = word(m, obj.wrapping_add(4));
    let cached =
        frontend::match_halves(handle, tag, word(&globals.pair, 0), word(&globals.pair, 4)) != 0;
    // The cache and tag checks precede the handle dereference in the ROM.
    let handle_word = if handle == 0 {
        None
    } else if cached || tag != 0x1e61 {
        Some(0)
    } else {
        Some(half(m, handle))
    };
    let mut ctx = 0;
    let rc = frontend::resolve_engine_context(
        handle,
        handle_word,
        tag,
        &globals.pair,
        globals.default_context,
        &mut ctx,
    );
    if rc >= 0x8000_0000 {
        return 0x8110_2007;
    }
    let plugin = word(m, obj.wrapping_add(48));
    let method = word(m, plugin.wrapping_add(40));
    let first = word(m, obj.wrapping_add(52));
    let second = word(m, obj.wrapping_add(56));
    let rc = host.destroy(m, method, first, second);
    if rc >= 0x8000_0000 {
        return rc;
    }
    let heap = word(m, ctx.wrapping_add(4));
    frontend::heap_release(
        m,
        if heap == 0 { None } else { Some(heap) },
        Some(obj),
        host,
    );
    0
}

fn put_half(m: &mut [u8], p: u32, v: u16) {
    m[p as usize..p as usize + 2].copy_from_slice(&v.to_le_bytes());
}

/// The two words the ROM reloads from `D_914b18 + 0x20` into its stack
/// argument for [`PREDICATE_KIND_SEVEN`]. Callers copy them from the constants
/// blob; the predicate only reads them.
pub const PREDICATE_KIND_SEVEN: u32 = 0x2b9414;

/// The eight arguments the processor behind `fn_2535cc` receives. `b1` and
/// `b2` are addresses of the two scratch buffers, `h2` the address of the
/// halfword count field, and `out` the caller's status cell.
pub struct ProcessCall {
    pub x: u32,
    pub y: u32,
    pub flag: u32,
    pub b1: u32,
    pub b2: u32,
    pub h2: u32,
    pub out: u32,
}

/// Native boundaries reached by the render processors.
pub trait RenderHost:
    frontend::HeapVtable
    + frontend::MethodTable
    + normalization::NormalizationHost
    + crate::selection::arena::Host
{
    /// Calls the unit processor at `vto + 0x40` with the eight ROM arguments.
    /// Returns `(one, status)`: the processor's `one` cell and its result.
    fn process(&mut self, memory: &mut [u8], vto: u32, call: &ProcessCall) -> (u32, u32);
}

/// Runs `fn_252948` against an arena-held cursor cell and restores it. The ROM
/// keeps the index on its stack; this port borrows a spare arena cell so the
/// crate's own forward search stays in use, then puts the cell back.
fn search_at(
    m: &mut [u8],
    host: &mut impl RenderHost,
    context: u32,
    callback: u32,
    argument: u32,
    start: u16,
    scratch: u32,
) -> (u32, u16) {
    let at = scratch as usize;
    let saved = [m[at], m[at + 1]];
    put_half(m, scratch, start);
    let rc = crate::render_part_a::search_forward(
        &mut crate::selection::arena::Arena::new(0, m),
        host,
        context,
        callback,
        argument,
        scratch,
    );
    let index = half(m, scratch);
    m[at..at + 2].copy_from_slice(&saved);
    (rc, index)
}

/// fn_253204.
/// Deletes the token run starting at index `n` and folds its trailing records
/// into the entry list. `argument` points at the predicate's word pair and
/// `scratch` is the spare halfword cell used for the search cursor.
pub fn remove_native_blanks(
    m: &mut [u8],
    context: u32,
    stream: u32,
    mut n: u32,
    destination: u32,
    argument: u32,
    scratch: u32,
    host: &mut impl RenderHost,
) -> u32 {
    if stream == 0 || destination == 0 {
        return 0x8110_2007;
    }
    let first = match entry(m, stream, 0) {
        Ok(p) => p,
        Err(r) => return r,
    };
    let mut count = 0u16;
    {
        let table = word(m, stream.wrapping_add(12));
        let rc = normalization::inner_field(Some(&m[table as usize..]), Some(&mut count));
        if rc >= 0x8000_0000 {
            return rc;
        }
    }
    let (mut rc, mut index) =
        search_at(m, host, stream, PREDICATE_KIND_SEVEN, argument, 0, scratch);
    let mut flag = 0;
    if (index as u32) < n {
        let list = match entry(m, stream, index as u32) {
            Ok(p) => p,
            Err(r) => return r,
        };
        let end = list.wrapping_add((count as u32).wrapping_sub(index as u32).wrapping_mul(40));
        let mut p = list.wrapping_add(40);
        while p < end && flag == 0 {
            let join = word(m, list.wrapping_add(20)).wrapping_add(word(m, list.wrapping_add(24)));
            if join == word(m, p.wrapping_add(20))
                && half(m, p.wrapping_add(10)) == 7
                && matches!(word(m, p.wrapping_add(28)), 14 | 15 | 16 | 44)
            {
                flag = 1;
            }
            if join < word(m, p.wrapping_add(20)) {
                break;
            }
            p = p.wrapping_add(40);
        }
        if flag == 1 {
            let (r, _) = search_at(m, host, stream, PREDICATE_KIND_SEVEN, argument, 0, scratch);
            if r >= 0x8000_0000 && (r & 0x1fff) != 0x14 {
                return r;
            }
            return finish(m, stream, n, count);
        }
    }
    loop {
        if rc >= 0x8000_0000 {
            if (rc & 0x1fff) != 0x14 {
                return rc;
            }
            break;
        }
        if index as u32 >= n {
            break;
        }
        let list = match entry(m, stream, index as u32) {
            Ok(p) => p,
            Err(r) => return r,
        };
        let offset = (word(m, list.wrapping_add(20)).wrapping_sub(word(m, first.wrapping_add(20)))
            as u16)
            >> 1;
        let length = (word(m, list.wrapping_add(24)) as u16) >> 1;
        let erased = normalization::delete_range(m, destination, offset, length);
        if erased >= 0x8000_0000 {
            return erased;
        }
        let join = word(m, list.wrapping_add(20)).wrapping_add(word(m, list.wrapping_add(24)));
        let end = list.wrapping_add((count as u32).wrapping_sub(index as u32).wrapping_mul(40));
        let mut p = list.wrapping_add(40);
        while p < end {
            if join == word(m, p.wrapping_add(20))
                && half(m, p.wrapping_add(10)) == 7
                && word(m, p.wrapping_add(28)) != 43
            {
                put_word(m, p.wrapping_add(12), word(m, list.wrapping_add(12)));
                let f10 = word(m, p.wrapping_add(16)).wrapping_add(word(m, list.wrapping_add(16)));
                put_word(m, p.wrapping_add(16), f10);
                let k = half(m, p.wrapping_add(8));
                let heap = word(m, context.wrapping_add(4));
                let ents = word(m, p.wrapping_add(4));
                let buffer = if ents != 0 {
                    let grown = reallocate_bytes(
                        m,
                        heap,
                        ents,
                        (k as u32 + 1).wrapping_mul(8).wrapping_add(1),
                        host,
                    );
                    if grown == 0 {
                        return 0x8110_200a;
                    }
                    put_word(m, p.wrapping_add(4), grown);
                    put_half(m, p.wrapping_add(8), k.wrapping_add(1));
                    grown
                } else {
                    let made = allocate_items(m, heap, 1, 9, host);
                    if made == 0 {
                        return 0x8110_200a;
                    }
                    put_word(m, p.wrapping_add(4), made);
                    put_half(m, p.wrapping_add(8), 1);
                    made
                };
                put_word(
                    m,
                    buffer.wrapping_add(k as u32 * 8),
                    word(m, list.wrapping_add(12)).wrapping_sub(word(m, first.wrapping_add(12))),
                );
                put_word(
                    m,
                    buffer.wrapping_add(k as u32 * 8).wrapping_add(4),
                    word(m, list.wrapping_add(16)),
                );
            }
            let moved = if join <= word(m, p.wrapping_add(20)) {
                word(m, p.wrapping_add(20)).wrapping_sub(word(m, list.wrapping_add(24)))
            } else {
                word(m, list.wrapping_add(20))
            };
            put_word(m, p.wrapping_add(20), moved);
            p = p.wrapping_add(40);
        }
        if word(m, list.wrapping_add(4)) != 0 {
            let heap = word(m, context.wrapping_add(4));
            release_bytes(m, heap, word(m, list.wrapping_add(4)), host);
            put_word(m, list.wrapping_add(4), 0);
        }
        let deleted = normalization::delete_range(m, stream, index, 1);
        if deleted >= 0x8000_0000 {
            return deleted;
        }
        count = count.wrapping_sub(1);
        n = (n as u16).wrapping_sub(1) as u32;
        let (r, next) = search_at(
            m,
            host,
            stream,
            PREDICATE_KIND_SEVEN,
            argument,
            index,
            scratch,
        );
        rc = r;
        index = next;
    }
    finish(m, stream, n, count)
}

fn finish(m: &mut [u8], stream: u32, n: u32, count: u16) -> u32 {
    let mut index = half(m, stream.wrapping_add(16));
    let rc = normalization::set_index(count, n as i32, &mut index);
    if rc >= 0x8000_0000 {
        return rc;
    }
    put_half(m, stream.wrapping_add(16), index);
    0
}

/// fn_2535cc / EU fn_253788 / KR fn_253834. Recovered from the target: eight
/// arguments, `r0` a vtable, `r3` the unit state.
pub fn parse_first_unit(
    m: &mut [u8],
    vtable: u32,
    x: u32,
    y: u32,
    state: u32,
    flag: u32,
    capacity: u32,
    count: u32,
    out: u32,
    constants: u32,
    host: &mut impl RenderHost,
) -> u32 {
    if state == 0 {
        return 0x8110_2007;
    }
    let used = half(m, state.wrapping_add(4));
    m[state as usize + 6] = 0;
    if used as u32 > capacity {
        return 0;
    }
    let first = word(m, state);
    put_half(m, first.wrapping_add(used as u32 * 2), 0);
    put_half(m, state.wrapping_add(0xc), count as u16);
    put_word(m, out, 0);
    let second = word(m, state.wrapping_add(8));
    let call = ProcessCall {
        x,
        y,
        flag,
        b1: word(m, state),
        b2: second,
        h2: state.wrapping_add(0xc),
        out,
    };
    let (one, rc) = host.process(m, vtable, &call);
    if rc != 0 {
        return 0;
    }
    let second = word(m, state.wrapping_add(8));
    let mut width = half(m, state.wrapping_add(0xc));
    if half(m, second.wrapping_add(width as u32 * 2).wrapping_sub(2)) == 0 {
        width = width.wrapping_sub(1);
        put_half(m, state.wrapping_add(0xc), width);
    }
    if one != 4 {
        m[state as usize + 6] = 43;
        return 0;
    }
    if width < 7 {
        m[state as usize + 6] = 2;
        return 0;
    }
    if normalization::memcmp_at(m, constants, second, 6) != 0 {
        m[state as usize + 6] = 2;
        return 0;
    }
    if normalization::memcmp_at(
        m,
        constants,
        second.wrapping_add(width as u32 * 2).wrapping_sub(6),
        6,
    ) != 0
    {
        m[state as usize + 6] = 2;
        return 0;
    }
    normalization::memmove_at(m, second, second.wrapping_add(4), width as u32 * 2 - 8);
    put_half(
        m,
        second.wrapping_add((width as u32).wrapping_sub(4) * 2),
        0,
    );
    put_half(m, second, 0x10);
    put_half(
        m,
        second.wrapping_add((width as u32).wrapping_sub(5) * 2),
        0x11,
    );
    put_half(m, state.wrapping_add(0xc), width.wrapping_sub(4));
    m[state as usize + 6] = 0x10;
    0
}

/// fn_253768 / EU fn_253924 / KR fn_2539d0. Recovers a unit's index in a
/// terminated 16-bit string array, appending it when absent.
pub fn intern_string(
    m: &mut [u8],
    mut list: u32,
    text: u32,
    out: u32,
    owner: u32,
    host: &mut impl RenderHost,
) -> u32 {
    let mut rc = 0;
    let mut index = 0u32;
    let mut item = word(m, list.wrapping_add(4));
    loop {
        if item == 0 {
            break;
        }
        if (half(m, list.wrapping_add(8)) as u32) <= index {
            break;
        }
        let len = crate::fe_text::strlen16_176624(m, item) as u32;
        if crate::fe_text::strcmp16_1765ac(m, text, item) == 0 {
            put_word(m, out, index);
            return rc;
        }
        index = index.wrapping_add(len).wrapping_add(1);
        item = item.wrapping_add(len.wrapping_mul(2)).wrapping_add(2);
    }
    let count = half(m, list.wrapping_add(8)) as u32;
    put_word(m, out, count);
    let need = (crate::fe_text::strlen16_176624(m, text) as u32).wrapping_add(1) as u16 as u32;
    let capacity = half(m, list.wrapping_add(0xa));
    let total = count.wrapping_add(need) as u16;
    if (capacity as u32) < total as u32 {
        let mut moved = 0u32;
        rc =
            normalization::fetch_entry(m, owner, 0x3f4, total as u32, Some(&mut moved), &mut *host);
        if rc >= 0x8000_0000 {
            return rc;
        }
        list = moved;
    }
    let count = half(m, list.wrapping_add(8)) as u32;
    let destination = word(m, list.wrapping_add(4)).wrapping_add(count.wrapping_mul(2));
    crate::fe_text::memcpy_195ae8(m, destination, text, need.wrapping_mul(2));
    put_half(m, list.wrapping_add(8), total);
    rc
}

/// A record table entry (`E152`) is 40 bytes wide.
fn record(base: u32, index: u32) -> u32 {
    base.wrapping_add(index.wrapping_mul(40))
}

fn stream_count(m: &[u8], stream: u32) -> Result<u16, u32> {
    let table = word(m, stream.wrapping_add(12));
    let mut out = 0u16;
    let rc = normalization::inner_field(Some(&m[table as usize..]), Some(&mut out));
    if rc >= 0x8000_0000 { Err(rc) } else { Ok(out) }
}

fn set_stream_index(m: &mut [u8], stream: u32, index: i32) -> u32 {
    let table = word(m, stream.wrapping_add(12));
    let count = half(m, table.wrapping_add(8));
    let mut out = half(m, stream.wrapping_add(16));
    let rc = normalization::set_index(count, index, &mut out);
    if rc >= 0x8000_0000 {
        return rc;
    }
    put_half(m, stream.wrapping_add(16), out);
    0
}

/// fn_2571e0 / EU fn_25739c / KR fn_257448. Merges the records between equal
/// marker runs, then rewrites or drops the boundary records. `staging` is a
/// caller-owned 40-byte cell standing in for the ROM's stack copy of the
/// constants block at `constants + 0x528`.
pub fn merge_marker_runs(
    m: &mut [u8],
    a: u32,
    b: u32,
    c: u32,
    _constants: u32,
    staging: u32,
    host: &mut impl RenderHost,
) -> u32 {
    if b == 0 || c == 0 {
        return 0x8110_2007;
    }
    let mut base = match entry(m, c, 0) {
        Ok(p) => p,
        Err(r) => return r,
    };
    let mut limit = match stream_count(m, c) {
        Ok(v) => v,
        Err(r) => return r,
    };
    let heap = word(m, word(m, a.wrapping_add(8)).wrapping_add(4));
    let mut w36 = 0i32;
    let mut w0 = 0i32;
    let mut w108 = 0i32;
    let mut w112 = 0i32;
    let mut w116 = 0i32;
    'outer: loop {
        if w0 >= limit as i32 {
            return 0;
        }
        let mut r3v = 0i32;
        loop {
            if w0 >= limit as i32 {
                break;
            }
            if half(m, record(base, w0 as u32).wrapping_add(10)) == 7 {
                break;
            }
            w0 += 1;
        }
        if w0 < limit as i32 && half(m, record(base, w0 as u32).wrapping_add(10)) == 7 {
            w36 = w0;
            w108 = word(m, record(base, w0 as u32).wrapping_add(28)) as i32;
        }
        let ent = record(base, w36 as u32);
        if half(m, ent.wrapping_add(10)) == 7 && word(m, ent.wrapping_add(28)) == 43 {
            if m[ent as usize + 36] != 0 {
                w0 += 1;
                continue 'outer;
            }
        }
        let mut r7v = w0 as u16;
        loop {
            if r7v >= limit {
                break;
            }
            let e7 = record(base, r7v as u32);
            if half(m, e7.wrapping_add(10)) != 7 || word(m, e7.wrapping_add(28)) as i32 == w108 {
                if half(m, e7.wrapping_add(10)) == 36 {
                    r3v = 1;
                }
                r7v = r7v.wrapping_add(1);
                continue;
            }
            break;
        }
        while r7v != w0 as u16 {
            r7v = r7v.wrapping_sub(1);
            if r7v <= w0 as u16 {
                break;
            }
            let e7 = record(base, r7v as u32);
            if half(m, e7.wrapping_add(10)) == 7 && word(m, e7.wrapping_add(28)) as i32 == w108 {
                break;
            }
        }
        if w108 == 43 && r3v == 1 {
            w0 += 1;
            continue 'outer;
        }
        if w108 == 43 {
            let e7 = record(base, r7v as u32);
            if half(m, e7.wrapping_add(10)) == 7
                && word(m, e7.wrapping_add(28)) == 43
                && m[a as usize + 82] == 1
            {
                w0 += 1;
                continue 'outer;
            }
        }
        if w0 == r7v as i32 || limit <= r7v {
            w0 = (w0 as u16).wrapping_add(1) as i32;
            continue 'outer;
        }
        if w0 != r7v as i32 && limit > r7v {
            let ent = record(base, w36 as u32);
            let ip28 = word(m, ent.wrapping_add(28)) as i32;
            let mut r3x = 0i32;
            w116 = 0;
            w112 = 0;
            if ip28 == 16 {
                let prev = ent.wrapping_sub(40);
                if w36 - 1 > 0
                    && half(m, prev.wrapping_add(10)) == 998
                    && word(m, prev.wrapping_add(20)) == word(m, ent.wrapping_add(20))
                {
                    w112 = w36 - 1;
                }
                let next = ent.wrapping_add(40);
                if w36 + 1 < limit as i32
                    && half(m, next.wrapping_add(10)) == 998
                    && word(m, ent.wrapping_add(20)) == word(m, next.wrapping_add(20))
                {
                    w112 = w36 + 1;
                }
                if r7v as i32 - 1 > 0 {
                    let back = record(base, r7v as u32).wrapping_sub(40);
                    if half(m, back.wrapping_add(10)) == 998
                        && word(m, back.wrapping_add(20))
                            == word(m, record(base, r7v as u32).wrapping_add(20))
                    {
                        r3x = r7v as i32 - 1;
                    }
                }
                if r7v as i32 + 1 < limit as i32 {
                    let ahead = record(base, r7v as u32).wrapping_add(40);
                    if half(m, ahead.wrapping_add(10)) == 998
                        && word(m, ahead.wrapping_add(20))
                            == word(m, record(base, r7v as u32).wrapping_add(20))
                    {
                        r3x = r7v as i32 + 1;
                    }
                }
                if w112 != 0 && r3x != 0 {
                    w116 = 1;
                }
            }
            {
                let e7 = record(base, r7v as u32);
                if m[e7 as usize] == 0
                    && word(m, e7.wrapping_add(20)) == word(m, ent.wrapping_add(20))
                {
                    put_word(m, ent.wrapping_add(12), word(m, e7.wrapping_add(12)));
                    put_word(m, ent.wrapping_add(16), word(m, e7.wrapping_add(16)));
                }
                let w24 = word(m, e7.wrapping_add(20))
                    .wrapping_sub(word(m, ent.wrapping_add(20)))
                    .wrapping_add(word(m, e7.wrapping_add(24)));
                put_word(m, ent.wrapping_add(24), w24);
                let w16 = word(m, e7.wrapping_add(16)).wrapping_add(
                    word(m, e7.wrapping_add(12)).wrapping_sub(word(m, ent.wrapping_add(12))),
                );
                put_word(m, ent.wrapping_add(16), w16);
            }
            let mut r8v2 = w36;
            while r8v2 <= r7v as i32 {
                let e4 = record(base, r8v2 as u32);
                let e5 = record(base, w36 as u32);
                let fpv = half(m, e4.wrapping_add(8));
                let r6v = half(m, e5.wrapping_add(8));
                if fpv != 0 && e4 != e5 {
                    let existing = word(m, e5.wrapping_add(4));
                    if existing == 0 {
                        let made = allocate_items(m, heap, fpv as u32, 9, host);
                        put_word(m, e5.wrapping_add(4), made);
                        if made == 0 {
                            return 0x8110_200a;
                        }
                        put_half(m, e5.wrapping_add(8), fpv);
                    } else {
                        let size = 1u32
                            .wrapping_add((r6v as u32).wrapping_add(fpv as u32).wrapping_mul(8));
                        let grown = reallocate_bytes(m, heap, existing, size, host);
                        if grown == 0 {
                            return 0x8110_200a;
                        }
                        put_word(m, e5.wrapping_add(4), grown);
                        put_half(m, e5.wrapping_add(8), r6v.wrapping_add(fpv));
                    }
                    // Elements are eight bytes wide: two words copied in order.
                    let c8 = half(m, e4.wrapping_add(8)) as u32;
                    let s4 = word(m, e4.wrapping_add(4));
                    let s5 = word(m, e5.wrapping_add(4));
                    let mut i = 0u32;
                    while i < c8 {
                        let from = s4.wrapping_add(i.wrapping_mul(8));
                        let hi = word(m, from);
                        let to = s5.wrapping_add((r6v as u32).wrapping_add(i).wrapping_mul(8));
                        put_word(m, to, hi);
                        let lo = word(m, from.wrapping_add(4));
                        put_word(m, to.wrapping_add(4), lo);
                        i += 1;
                    }
                    let old = word(m, e4.wrapping_add(4));
                    if old != 0 {
                        release_bytes(m, heap, old, host);
                        put_word(m, e4.wrapping_add(4), 0);
                        put_half(m, e4.wrapping_add(8), 0);
                    }
                }
                if r8v2 != w36 && word(m, e4.wrapping_add(32)) != 0 {
                    let p32 = word(m, e4.wrapping_add(32));
                    release_bytes(m, heap, p32, host);
                    put_word(m, e4.wrapping_add(32), 0);
                }
                r8v2 += 1;
            }
        }
        while r7v > w0 as u16 {
            let e7 = record(base, r7v as u32);
            let mut settled = false;
            if w116 == 1 && half(m, e7.wrapping_add(10)) == 7 {
                if r7v as i32 - 1 > 0 {
                    let back = e7.wrapping_sub(40);
                    if half(m, back.wrapping_add(10)) == 998
                        && word(m, back.wrapping_add(20)) == word(m, e7.wrapping_add(20))
                    {
                        put_half(m, e7.wrapping_add(10), 41);
                        settled = true;
                    }
                }
                if !settled && r7v as i32 + 1 < limit as i32 {
                    let ahead = e7.wrapping_add(40);
                    if half(m, ahead.wrapping_add(10)) == 998
                        && word(m, ahead.wrapping_add(20)) == word(m, e7.wrapping_add(20))
                    {
                        put_half(m, e7.wrapping_add(10), 41);
                        settled = true;
                    }
                }
            }
            if !settled {
                let kind = half(m, e7.wrapping_add(10));
                let drop = if kind == 0x4000 {
                    true
                } else if kind != 7 {
                    false
                } else if m[e7 as usize] == 0 {
                    put_half(m, e7.wrapping_add(10), 999);
                    false
                } else {
                    m[e7 as usize] == 1
                };
                if drop {
                    let r = normalization::delete_range(m, c, r7v, 1);
                    if r >= 0x8000_0000 {
                        return r;
                    }
                    limit = limit.wrapping_sub(1);
                }
            }
            r7v = r7v.wrapping_sub(1);
        }
        if w116 == 1 {
            let e112 = record(base, w112 as u32);
            let mut block = [0u8; 40];
            let from = e112 as usize;
            block.copy_from_slice(&m[from..from + 40]);
            block[10] = 41;
            block[11] = 0;
            let r = set_stream_index(m, c, w112 + 1);
            if r >= 0x8000_0000 {
                return r;
            }
            let at = staging as usize;
            let mut saved = [0u8; 40];
            saved.copy_from_slice(&m[at..at + 40]);
            m[at..at + 40].copy_from_slice(&block);
            let r = normalization::insert_ordered(m, c, staging, host);
            m[at..at + 40].copy_from_slice(&saved);
            if r >= 0x8000_0000 {
                return r;
            }
            base = match entry(m, c, 0) {
                Ok(p) => p,
                Err(r) => return r,
            };
            limit = match stream_count(m, c) {
                Ok(v) => v,
                Err(r) => return r,
            };
        } else {
            w0 += 1;
        }
    }
}

/// fn_2551ac / EU fn_255368 / KR fn_255414.
/// Replaces the current token's text, then shifts later token spans.
/// `scratch` is a caller-owned 64-byte frame; its original bytes are restored.
/// `constants` points at D_914b18.
pub fn replace_token_text(
    m: &mut [u8],
    a: u32,
    b: u32,
    c: u32,
    d: u32,
    text: u32,
    constants: u32,
    scratch: u32,
    host: &mut impl RenderHost,
) -> u32 {
    if a == 0 || c == 0 {
        return 0x8110_2007;
    }
    let mut saved = [0u8; 64];
    saved.copy_from_slice(&m[scratch as usize..scratch as usize + 64]);
    let rc = replace_token_text_inner(m, a, b, c, d, text, constants, scratch, host);
    m[scratch as usize..scratch as usize + 64].copy_from_slice(&saved);
    rc
}
fn replace_token_text_inner(
    m: &mut [u8],
    a: u32,
    b: u32,
    c: u32,
    d: u32,
    text: u32,
    constants: u32,
    frame: u32,
    host: &mut impl RenderHost,
) -> u32 {
    let first = match entry(m, b, 0) {
        Ok(p) => p,
        Err(r) => return r,
    };
    let mut current = match entry(m, b, 0xffff) {
        Ok(p) => p,
        Err(r) => return r,
    };
    let old_len = word(m, current + 24) as u16;
    let offset = word(m, current + 20).wrapping_sub(word(m, first + 20)) as u16;
    if d != 0 && text != 0 {
        // fn_176354 widens at most three bytes, then the caller terminates.
        crate::render_part_b::fn_176354(
            &mut crate::boot::Memory { base: 0, bytes: m },
            frame,
            text,
            3,
        );
        put_half(m, frame + 6, 0);
        let mut arr = 0;
        let rc = normalization::fetch_entry(m, d, 1012, 1, Some(&mut arr), host);
        if rc >= 0x8000_0000 {
            return rc;
        }
        let mut event = [0u8; 40];
        event.copy_from_slice(&m[constants as usize + 408..constants as usize + 448]);
        m[frame as usize + 8..frame as usize + 48].copy_from_slice(&event);
        put_half(m, frame + 18, 36);
        let rc = intern_string(m, arr, frame, frame + 36, d, host);
        if rc >= 0x8000_0000 {
            return rc;
        }
        put_word(m, frame + 20, word(m, current + 12));
        put_word(m, frame + 24, 0);
        put_word(m, frame + 28, word(m, current + 20));
        put_word(m, frame + 32, 0);
        let rc = set_stream_index(m, b, 1);
        if rc >= 0x8000_0000 {
            return rc;
        }
        let rc = normalization::insert_ordered(m, b, frame + 8, host);
        if rc >= 0x8000_0000 {
            return rc;
        }
    }
    let mut len = match stream_count(m, c) {
        Ok(n) => n,
        Err(r) => return r,
    };
    if len != 0 {
        let compare_len = len.min(old_len);
        let replacement = match entry(m, c, 0) {
            Ok(p) => p,
            Err(r) => return r,
        };
        let original = match entry(m, a, 0) {
            Ok(p) => p,
            Err(r) => return r,
        };
        let mut same = 0u16;
        while same < compare_len {
            if m[(original + offset as u32 + same as u32) as usize]
                != m[(replacement + same as u32) as usize]
            {
                break;
            }
            if m[(original + offset as u32 + same as u32 + 1) as usize]
                != m[(replacement + same as u32 + 1) as usize]
            {
                break;
            }
            same = same.wrapping_add(2);
        }
        let rc = normalization::delete_range(m, a, offset >> 1, old_len >> 1);
        if rc >= 0x8000_0000 {
            return rc;
        }
        let rc = set_stream_index(m, a, (offset >> 1) as i32);
        if rc >= 0x8000_0000 {
            return rc;
        }
        let rc = normalization::insert_elements(m, a, replacement, (len >> 1) as u32, host);
        if rc >= 0x8000_0000 {
            return rc;
        }
        let rc = normalization::delete_range(m, c, 0, len);
        if rc >= 0x8000_0000 {
            return rc;
        }
    } else {
        let rc = normalization::delete_range(m, a, offset >> 1, old_len >> 1);
        if rc >= 0x8000_0000 {
            return rc;
        }
    }
    let alternate = old_len != len;
    put_word(m,current+24,len as u32);
    let table = word(m, b + 12);
    let mut cursor = 0;
    let rc = normalization::current_index(
        Some(&m[table as usize..]),
        half(m, b + 16),
        Some(&mut cursor),
    );
    if rc >= 0x8000_0000 {
        return rc;
    }
    let limit = match stream_count(m, b) {
        Ok(n) => n,
        Err(r) => return r,
    };
    let origin = word(m, first + 20);
    loop {
        current = current.wrapping_add(40);
        cursor = cursor.wrapping_add(1);
        if cursor >= limit {
            break;
        }
        let end = origin.wrapping_add(offset as u32 + old_len as u32);
        let next = if alternate && end > word(m, current + 20) {
            origin.wrapping_add(offset as u32)
        } else {
            word(m, current + 20)
                .wrapping_add(len as u32)
                .wrapping_sub(old_len as u32)
        };
        put_word(m, current + 20, next);
    }
    0
}

// These frontend wrappers read only heap+4 before dispatching. Snapshot that
// eight-byte header into a base-zero view so the platform callback can mutate
// the original caller arena without aliasing a shared and mutable Rust slice.
struct HeapCall<'a, H> {
    memory: &'a mut [u8],
    host: &'a mut H,
    heap: u32,
}
impl<H: RenderHost> frontend::MethodTable for HeapCall<'_, H> {
    fn call_at(&mut self, _: u32, count: u32, size: u32) -> u32 {
        self.host
            .calloc(self.memory, self.heap, count, size)
            .unwrap_or(0)
    }
}
impl<H: RenderHost> frontend::HeapVtable for HeapCall<'_, H> {
    fn alloc_at(&mut self, _: u32, size: u32) -> u32 {
        self.host.alloc(self.memory, self.heap, size).unwrap_or(0)
    }
    fn realloc_at(&mut self, _: u32, old: u32, size: u32) -> u32 {
        self.host
            .realloc(self.memory, self.heap, old, size)
            .unwrap_or(0)
    }
    fn release_at(&mut self, _: u32, object: u32) {
        self.host.free(self.memory, self.heap, object);
    }
}
fn heap_header(m: &[u8], heap: u32) -> [u8; 8] {
    let mut header = [0; 8];
    if heap != 0 {
        header.copy_from_slice(&m[heap as usize..heap as usize + 8]);
    }
    header
}
fn allocate_items(
    m: &mut [u8],
    heap: u32,
    count: u32,
    size: u32,
    host: &mut impl RenderHost,
) -> u32 {
    let header = heap_header(m, heap);
    frontend::call_method(
        &header,
        if heap == 0 { None } else { Some(0) },
        count,
        size,
        &mut HeapCall {
            memory: m,
            host,
            heap,
        },
    )
}
fn allocate_bytes(m: &mut [u8], heap: u32, size: u32, host: &mut impl RenderHost) -> u32 {
    let header = heap_header(m, heap);
    frontend::heap_alloc(
        &header,
        if heap == 0 { None } else { Some(0) },
        size,
        &mut HeapCall {
            memory: m,
            host,
            heap,
        },
    )
}
fn reallocate_bytes(
    m: &mut [u8],
    heap: u32,
    old: u32,
    size: u32,
    host: &mut impl RenderHost,
) -> u32 {
    let header = heap_header(m, heap);
    frontend::heap_realloc(
        &header,
        if heap == 0 { None } else { Some(0) },
        old,
        size,
        &mut HeapCall {
            memory: m,
            host,
            heap,
        },
    )
}
fn release_bytes(m: &mut [u8], heap: u32, object: u32, host: &mut impl RenderHost) {
    let header = heap_header(m, heap);
    frontend::heap_release(
        &header,
        if heap == 0 { None } else { Some(0) },
        if object == 0 { None } else { Some(object) },
        &mut HeapCall {
            memory: m,
            host,
            heap,
        },
    );
}

/// fn_256360 / EU fn_25651c / KR fn_2565c8.
/// Appends the two fixed UTF-16 markers before replacing the token span.
pub fn append_marker_text(
    m: &mut [u8],
    a: u32,
    b: u32,
    c: u32,
    d: u32,
    constants: u32,
    scratch: u32,
    host: &mut impl RenderHost,
) -> u32 {
    if a == 0 || b == 0 || c == 0 || d == 0 {
        return 0x8110_2007;
    }
    for source in [constants.wrapping_add(4), constants] {
        let len = crate::fe_text::strlen16_176624(m, source) as u32;
        let rc = normalization::insert_elements(m, d, source, (len << 1) & 0xffff, host);
        if rc >= 0x8000_0000 {
            return rc;
        }
    }
    replace_token_text(m, b, c, d, 0, 0, constants, scratch, host)
}

/// fn_25552c / EU fn_2556e8 / KR fn_255794.
/// Runs the unit processor and replaces the selected token span.
/// Scratch is 120 disjoint caller-owned bytes, restored before returning.
pub fn process_token_span<H: RenderHost + crate::boot_part_b::FormatHost>(
    m: &mut [u8],
    a: u32,
    b: u32,
    c: u32,
    d: u32,
    e: u32,
    constants: u32,
    scratch: u32,
    host: &mut H,
) -> u32 {
    if a == 0 || b == 0 || c == 0 {
        return 0x8110_2007;
    }
    let mut saved = [0u8; 120];
    saved.copy_from_slice(&m[scratch as usize..scratch as usize + 120]);
    let rc = process_token_span_inner(m, a, b, c, d, e, constants, scratch, host);
    m[scratch as usize..scratch as usize + 120].copy_from_slice(&saved);
    rc
}
fn process_token_span_inner<H: RenderHost + crate::boot_part_b::FormatHost>(
    m: &mut [u8],
    a: u32,
    b: u32,
    c: u32,
    d: u32,
    e: u32,
    constants: u32,
    s: u32,
    host: &mut H,
) -> u32 {
    let first = match entry(m, c, 0) {
        Ok(p) => p,
        Err(r) => return r,
    };
    let bytes = match entry(m, b, 0) {
        Ok(p) => p,
        Err(r) => return r,
    };
    let current = match entry(m, c, 0xffff) {
        Ok(p) => p,
        Err(r) => return r,
    };
    let width = (word(m, current + 24) as u16) >> 1;
    let offset = (word(m, current + 20).wrapping_sub(word(m, first + 20)) as u16) >> 1;
    let capacity = half(m, a + 62);
    if capacity < width {
        return 0;
    }
    put_word(m, s, 0);
    put_word(m, s + 8, 0);
    put_half(m, s + 4, width);
    put_word(m, s + 16, 0);
    let ctx = word(m, a + 8);
    let heap = word(m, ctx + 4);
    let first_buffer = allocate_items(m, heap, capacity as u32 + 1, 2, host);
    put_word(m, s, first_buffer);
    let mut second_buffer = 0;
    let mut rc = 0x8110_200a;
    if first_buffer != 0 {
        normalization::memmove_at(m, first_buffer, bytes + offset as u32 * 2, width as u32 * 2);
        let size = (half(m, a + 60) as u32 + 1)
            .wrapping_mul(half(m, a + 64) as u32)
            .wrapping_mul(2)
            .wrapping_add(2);
        second_buffer = allocate_items(m, heap, 1, size, host);
        put_word(m,s+8,second_buffer);
    }
    if first_buffer == 0 || second_buffer == 0 {
        let logger = word(m, ctx + 16);
        crate::render_part_a::error_log(
            &mut crate::boot::Memory { base: 0, bytes: m },
            host,
            logger,
            constants + 0x38,
            0x783,
            0,
            0,
            s + 96,
        );
    } else {
        rc = parse_first_unit(
            m,
            word(m, a + 36),
            word(m, a + 40),
            word(m, a + 44),
            s,
            1,
            capacity as u32,
            half(m, a + 60) as u32,
            s + 16,
            constants,
            host,
        );
        if rc < 0x8000_0000 {
            let kind = m[s as usize + 6];
            if kind != 0 {
                if kind == 16 || kind == 43 {
                    put_word(m, current + 28, kind as u32);
                }
                rc = normalization::insert_elements(
                    m,
                    d,
                    second_buffer,
                    (half(m, s + 12).wrapping_shl(1)) as u32,
                    host,
                );
                if rc < 0x8000_0000 {
                    rc =
                        replace_token_text(m, b, c, d, e, word(m, s + 16), constants, s + 32, host);
                }
            }
        }
    }
    release_bytes(m, heap, first_buffer, host);
    release_bytes(m, heap, second_buffer, host);
    rc
}

/// Arguments and out-cells for the engine's normalization method at vtable +32.
/// The mode is an integer bit pattern on the stack, not a VFP argument.
pub struct NormalizeCall {
    pub args: [u32; 11],
    pub count_cell: u32,
    pub mode: u32,
    pub pos_cell: u32,
    pub table_cell: u32,
}
pub trait NativeNormalizeHost: RenderHost + crate::log_live::LogMethodsM {
    fn normalize(&mut self, m: &mut [u8], method: u32, call: &NormalizeCall) -> u32;
}

fn shrink_native_table(m: &mut [u8], a: u32, heap: u32, host: &mut impl RenderHost) -> u32 {
    if half(m,a+74)>20 {
        let p=reallocate_bytes(m,heap,word(m,a+68),80,host);
        if p==0 {return 0x8110_200a;}
        put_word(m,a+68,p); put_half(m,a+74,10);
    }
    0
}

/// fn_255be8 / EU 255da4 / KR 255e50. Normalizes a native word run.
/// Scratch requires 192 disjoint bytes, restored even on early return.
pub fn normalize_native(m: &mut [u8], a:u32,b:u32,c:u32,d:u32,mode:u32,e:u32,
    constants:u32,scratch:u32,host:&mut impl NativeNormalizeHost)->u32 {
    let mut saved=[0u8;192];
    saved.copy_from_slice(&m[scratch as usize..scratch as usize+192]);
    let rc=normalize_native_inner(m,a,b,c,d,mode,e,constants,scratch,host);
    m[scratch as usize..scratch as usize+192].copy_from_slice(&saved);
    rc
}
fn normalize_native_inner(m:&mut [u8],a:u32,b:u32,c:u32,d:u32,mode:u32,e:u32,k:u32,s:u32,
    host:&mut impl NativeNormalizeHost)->u32 {
    if a==0 || b==0 || c==0 || d==0 {return 0x8110_2007;}
    let mut arr=0;
    let r=normalization::fetch_entry(m,e,1012,1,Some(&mut arr),host);
    if r>=0x8000_0000 {return r;}
    let first=match entry(m,c,0){Ok(p)=>p,Err(r)=>return r};
    let origin=word(m,first+20); let native_origin=word(m,first+12);
    let bytes=match entry(m,b,0){Ok(p)=>p,Err(r)=>return r};
    let current=match entry(m,c,0xffff){Ok(p)=>p,Err(r)=>return r};
    let n=word(m,current+24);
    let off=word(m,current+20).wrapping_sub(origin) as u16;
    let size=n.wrapping_add(1) as u16 as u32;
    let heap=word(m,word(m,a+8)+4);
    let buffer=allocate_bytes(m,heap,size,host);
    if buffer==0 {return 0x8110_200a;}
    crate::fe_text::memset_195bec(m,buffer,0,size);
    let len=(size as u16).wrapping_sub(1) as u32;
    normalization::memcpy_at(m,buffer,bytes+off as u32,len);
    let arr_start=half(m,arr+8);
    m[a as usize+76]=1; m[a as usize+77]=1;
    put_half(m,s,0);put_word(m,s+4,0);
    let call=NormalizeCall{args:[word(m,a+52),word(m,a+56),buffer,(n<<15)>>16,len,b,c,d,e,arr,1012],
        count_cell:s,mode,pos_cell:s+4,table_cell:a+68};
    let mut rc=host.normalize(m,word(m,word(m,a+48)+32),&call);
    let pos_table=word(m,s+4);
    let mut direct_return=false;
    if rc<0x8000_0000 {
        rc=(|| {
            let count=match stream_count(m,b){Ok(n)=>n,Err(r)=>return r};
            if count==0 {return 0;}
            let output=match entry(m,b,0){Ok(p)=>p,Err(r)=>return r};
            let _=match entry(m,c,0){Ok(p)=>p,Err(r)=>{direct_return=true;return r}};
            let _=match stream_count(m,c){Ok(n)=>n,Err(r)=>{direct_return=true;return r}};
            let native_start=word(m,current+12);
            let shifts=word(m,current+4);let shift_count=half(m,current+8);
            let delta=native_start.wrapping_sub(native_origin);
            for i in 0..shift_count as u32 {
                let p=shifts+i*8; put_word(m,p,word(m,p).wrapping_sub(delta));
            }
            let start=word(m,current+20).wrapping_sub(origin) as u16 as u32;
            let end=start+half(m,s) as u32;
            let mut markers=0u16;
            for i in start..end {if m[(output+i) as usize]==16 {markers=markers.wrapping_add(1);}}
            normalization::memcpy_at(m,s+16,k+448,40);
            let event=s+16;
            let mut scan=0u16;let mut arr_offset=0u16;let mut acc=0u32;let mut added=0u16;let mut pos_index=1u16;
            let mut rc=0;
            let mut i=start as u16;
            while (i as u32)<end {
                if m[(output+i as u32) as usize]==16 {
                    let index=arr_start as u32+arr_offset as u32;
                    let text=word(m,arr+4).wrapping_add(index.wrapping_mul(half(m,arr+2) as u32));
                    let text_len=crate::fe_text::strlen16_176624(m,text) as u16;
                    if text_len!=0 {
                        let table=word(m,a+68);let total=half(m,a+72);
                        while scan<total && m[(table+4+scan as u32*8) as usize]==1 {scan=scan.wrapping_add(1);}
                        let next_scan=scan.wrapping_add(1);
                        let mut next=next_scan;
                        while next<total && m[(table+4+next as u32*8) as usize]==1 {next=next.wrapping_add(1);}
                        let mut from=0u32;let mut span=0u16;
                        if total>scan {
                            from=half(m,table+scan as u32*8) as u32;
                            let to=if total==next {len} else {half(m,table+next as u32*8) as u32};
                            span=to.wrapping_sub(from) as u16;scan=next_scan;
                        } else if m[a as usize+76]==1 {
                            span=if markers==1 {word(m,current+16) as u16}
                                else {(normalization::udiv(text_len as u32,half(m,a+80) as u32)*2) as u16};
                        }
                        let mut adjustment=0;
                        for j in 0..shift_count as u32 {
                            let p=shifts+j*8;let key=word(m,p);
                            if key>from.wrapping_add(span as u32).wrapping_add(acc){break;}
                            if key==from.wrapping_add(acc){acc=acc.wrapping_add(word(m,p+4));}
                            if key>from.wrapping_add(acc) && key<=from.wrapping_add(acc).wrapping_add(span as u32) {adjustment=word(m,p+4);}
                        }
                        let at=native_start.wrapping_add(from).wrapping_add(acc).wrapping_add(added as u32);
                        let byte_at=origin.wrapping_add(i as u32);
                        put_half(m,event+10,39);put_word(m,event+12,at);put_word(m,event+16,span as u32);
                        put_word(m,event+20,byte_at);put_word(m,event+24,text_len as u32*2);put_word(m,event+28,index);
                        rc=normalization::insert_ordered(m,c,event,host);if rc>=0x8000_0000{return rc;}
                        put_half(m,event+10,41);put_word(m,event+28,0);
                        rc=normalization::insert_ordered(m,c,event,host);if rc>=0x8000_0000{return rc;}
                        if pos_table!=0 {
                            let pos=if half(m,pos_table) as u32+1>pos_index as u32 {half(m,pos_table+pos_index as u32*2) as u32}
                            else {
                                let format=b"[UTOI] (missing POS for normalized word %d\0";
                                m[s as usize+120..s as usize+120+format.len()].copy_from_slice(format);
                                put_word(m,s+176,pos_index as u32);
                                let logger=word(m,word(m,a+8)+16);
                                crate::log_live::fn_1738e8(&mut crate::boot::Memory{base:0,bytes:m},
                                    logger,k+0x38,5,0,s+120,s+176,host);
                                0
                            };
                            put_half(m,event+10,14);put_word(m,event+12,at);
                            if m[a as usize+76]==1 {added=added.wrapping_add(span);}
                            put_word(m,event+16,span as u32);
                            if word(m,event+20)!=origin {put_word(m,event+20,byte_at.wrapping_add(2));}
                            put_word(m,event+24,0);put_word(m,event+28,pos);
                            rc=normalization::insert_ordered(m,c,event,host);if rc>=0x8000_0000{return rc;}
                            pos_index=pos_index.wrapping_add(1);
                        }
                        arr_offset=arr_offset.wrapping_add(text_len).wrapping_add(1);
                        acc=acc.wrapping_add(adjustment);
                    }
                }
                i=i.wrapping_add(1);
            }
            let shrink=shrink_native_table(m,a,heap,host);
            if shrink>=0x8000_0000 {shrink} else {rc}
        })();
    }
    if direct_return {return rc;}
    release_bytes(m,heap,buffer,host);release_bytes(m,heap,pos_table,host);rc
}
/// fn_256320 / EU 2564dc / KR 256588.
pub fn normalize_native_zero(m:&mut [u8],a:u32,b:u32,c:u32,d:u32,e:u32,k:u32,s:u32,h:&mut impl NativeNormalizeHost)->u32{
    normalize_native(m,a,b,c,d,0,e,k,s,h)
}
/// fn_256340 / EU 2564fc / KR 2565a8.
pub fn normalize_native_one(m:&mut [u8],a:u32,b:u32,c:u32,d:u32,e:u32,k:u32,s:u32,h:&mut impl NativeNormalizeHost)->u32{
    normalize_native(m,a,b,c,d,1,e,k,s,h)
}

/// fn_2563f4 / EU 2565b0 / KR 25665c. Converts a UTF-16 run to native text,
/// replaces it, and inserts native-boundary events. Scratch is 192 bytes.
pub fn convert_native(m:&mut [u8],a:u32,b:u32,c:u32,d:u32,e:u32,k:u32,s:u32,host:&mut impl RenderHost)->u32 {
    let mut saved=[0u8;192];saved.copy_from_slice(&m[s as usize..s as usize+192]);
    let rc=convert_native_inner(m,a,b,c,d,e,k,s,host);
    m[s as usize..s as usize+192].copy_from_slice(&saved);rc
}
fn convert_native_inner(m:&mut [u8],a:u32,b:u32,c:u32,d:u32,e:u32,k:u32,s:u32,host:&mut impl RenderHost)->u32 {
    if a==0 || b==0 || c==0 || d==0 {return 0x8110_2007;}
    let first=match entry(m,c,0){Ok(p)=>p,Err(r)=>return r};
    let bytes=match entry(m,b,0){Ok(p)=>p,Err(r)=>return r};
    let ents=match entry(m,c,0){Ok(p)=>p,Err(r)=>return r};
    let count=match stream_count(m,c){Ok(n)=>n,Err(r)=>return r};
    let current=match entry(m,c,0xffff){Ok(p)=>p,Err(r)=>return r};
    let native_start=word(m,current+12);let offset=word(m,current+20);
    let length=word(m,current+24);let shifts=word(m,current+4);
    let shift_count=half(m,current+8);let kind=word(m,current+28);
    let text=bytes+(offset.wrapping_sub(word(m,first+20)) as u16 as u32);
    for i in 0..shift_count as u32 {
        let p=shifts+i*8;
        put_word(m,p,word(m,p).wrapping_sub(native_start.wrapping_sub(word(m,first+12))));
    }
    let n=length as u16 as u32;if n==0 {return 0;}
    let capacity=1+(n>>1);let heap=word(m,word(m,a+8)+4);
    let narrow=allocate_bytes(m,heap,capacity,host);
    if narrow==0{return 0x8110_200a;}
    crate::fe_text::memset_195bec(m,narrow,0,capacity);
    put_half(m,a+72,0);m[a as usize+76]=1;m[a as usize+77]=1;
    let mut wide=0;
    let mut direct_return=false;
    let rc=(||{
        let cap=capacity-1;
        if text==0 || cap==0 {return 0x8110_2007;}
        let mut pos=0u16;let mut out=0u16;
        while (out as u32)<cap && (pos as u32)<n {
            let ch=m[(text+pos as u32) as usize];
            let high=m[(text+pos.wrapping_add(1) as u32) as usize];
            let skip=high==0 && ch==13 && pos as u32+1<n &&
                m[(text+pos.wrapping_add(2) as u32) as usize]==10 && m[(text+pos.wrapping_add(3) as u32) as usize]==0;
            if !skip {
                let v=if (ch==2 && high==48)||(ch==183 && high==0)||(ch==38 && high==32){46}
                    else if ch==172 && high==32 {128}
                    else if high!=0 {if high==48 || high==255 {ch.wrapping_add(32)} else {32}}
                    else if (9..=13).contains(&ch) {32}
                    else if (ch as u32).wrapping_sub(15)>=161 {32} else {ch};
                m[(narrow+out as u32) as usize]=v;
                let starts=(out==0 && v!=32)||(out!=0 && v!=32 && m[(narrow+out as u32-1) as usize]==32);
                if starts && m[a as usize+77]==1 {
                    let used=half(m,a+72);let alloc=half(m,a+74);
                    if used as u32+1>alloc as u32 {
                        let p=reallocate_bytes(m,heap,word(m,a+68),81+alloc as u32*8,host);
                        if p==0{return 0x8110_200a;}
                        put_word(m,a+68,p);put_half(m,a+74,alloc.wrapping_add(10));
                    }
                    let p=word(m,a+68)+used as u32*8;
                    put_half(m,p,pos);put_half(m,p+2,0);m[p as usize+4]=0;put_half(m,p+6,out);
                    put_half(m,a+72,used.wrapping_add(1));
                }
                out=out.wrapping_add(1);
            }
            pos=pos.wrapping_add(2);
        }
        m[(narrow+out as u32) as usize]=0;
        let table=word(m,a+68);
        for i in 0..half(m,a+72) as u32 {
            let p=table+i*8;
            let span=if half(m,a+72) as u32<=i+1 {out.wrapping_sub(half(m,p+6))}
            else {
                let mut j=half(m,p+14).wrapping_sub(1);
                while j!=0 && m[(narrow+j as u32) as usize]==32 {j=j.wrapping_sub(1);}
                j.wrapping_sub(half(m,p+6)).wrapping_add(1)
            };
            put_half(m,p+2,span);
        }
        if n>pos as u32 {return 0x8110_2009;}
        if e!=0 {
            let len=crate::fe_text::strlen16_176624(m,k) as u32;
            let r=normalization::insert_elements(m,d,k,(len*2)&65535,host);if r>=0x8000_0000{return r;}
        }
        let size=(crate::fe_text::strlen_195d90(m,narrow) as u32+1)*2;
        wide=allocate_bytes(m,heap,size,host);if wide==0{return 0x8110_200a;}
        crate::fe_text::memset_195bec(m,wide,0,size);
        crate::fe_text::strcpy_widen_1763e4(m,wide,narrow);
        let len=crate::fe_text::strlen16_176624(m,wide) as u32;
        let r=normalization::insert_elements(m,d,wide,(len*2)&65535,host);if r>=0x8000_0000{return r;}
        if e!=0 {
            let len=crate::fe_text::strlen16_176624(m,k+4) as u32;
            let r=normalization::insert_elements(m,d,k+4,(len*2)&65535,host);if r>=0x8000_0000{return r;}
        }
        let mut rc=replace_token_text(m,b,c,d,0,0,k,s+64,host);
        if rc>=0x8000_0000{return rc;}
        let mut overlaps=0u16;
        if kind==16 && count>1 {
            let end=length.wrapping_add(offset);
            for i in 1..count as u32 {
                let p=ents+i*40;let at=word(m,p+20);
                if at>end {break;}
                if half(m,p+10)==998 && at>=offset && end>=word(m,p+24).wrapping_add(at) {overlaps=overlaps.wrapping_add(1);}
            }
        }
        if word(m,current+28)!=43 && half(m,a+72)!=0 {
            normalization::memcpy_at(m,s,k+488,40);
            let mut scan=0u16;let mut acc=0u32;let mut i=0u16;
            loop {
                let total=half(m,a+72);let table=word(m,a+68);
                while scan<total && m[(table+4+scan as u32*8) as usize]==1 {scan=scan.wrapping_add(1);}
                let next_scan=scan.wrapping_add(1);let mut next=next_scan;
                while next<total && m[(table+4+next as u32*8) as usize]==1 {next=next.wrapping_add(1);}
                let mut from=0u32;let mut span=0u16;let mut byte_off=0u32;let mut native_len=0u32;
                if total>scan {
                    let p=table+scan as u32*8;let at=half(m,p+6);
                    from=half(m,p) as u32;byte_off=(at as u32*2)&65535;
                    let end=if total==next {(n>>1).wrapping_sub(half(m,table+next as u32*8-2) as u32)}
                        else {(half(m,table+6+next as u32*8) as u32).wrapping_sub(at as u32)};
                    span=end as u16;
                    native_len=half(m,p+2) as u32*2;
                    scan=next_scan;
                }
                let mut adjustment=0;
                for j in 0..shift_count as u32 {
                    let p=shifts+j*8;let key=word(m,p);
                    if key!=0 && key>from.wrapping_add(span as u32).wrapping_add(acc){break;}
                    if key==from.wrapping_add(acc){acc=word(m,p+4);}
                    if key>from.wrapping_add(acc) && key<=from.wrapping_add(acc).wrapping_add(span as u32) {adjustment=word(m,p+4);}
                }
                put_half(m,s+10,41);
                put_word(m,s+24,span as u32*2);
                put_word(m,s+12,native_start.wrapping_add(from).wrapping_add(acc));
                put_word(m,s+16,native_len.wrapping_add(adjustment));
                put_word(m,s+20,offset.wrapping_add(byte_off));
                if overlaps<=1 {rc=normalization::insert_ordered(m,c,s,host);if rc>=0x8000_0000{return rc;}}
                let _=match entry(m,c,0){Ok(p)=>p,Err(r)=>{direct_return=true;return r}};
                let _=match stream_count(m,c){Ok(n)=>n,Err(r)=>{direct_return=true;return r}};
                i=i.wrapping_add(1);acc=acc.wrapping_add(adjustment);
                if i>=half(m,a+72){break;}
            }
        }
        let r=shrink_native_table(m,a,heap,host);
        if r>=0x8000_0000 {r} else {rc}
    })();
    if direct_return {return rc;}
    release_bytes(m,heap,narrow,host);release_bytes(m,heap,wide,host);rc
}
/// fn_256df4 / EU 256fb0 / KR 25705c.
pub fn convert_native_plain(m:&mut [u8],a:u32,b:u32,c:u32,d:u32,k:u32,s:u32,h:&mut impl RenderHost)->u32 {
    convert_native(m,a,b,c,d,0,k,s,h)
}
/// fn_256fc8 / EU 257184 / KR 257230.
pub fn convert_native_marked(m:&mut [u8],a:u32,b:u32,c:u32,d:u32,k:u32,s:u32,h:&mut impl RenderHost)->u32 {
    convert_native(m,a,b,c,d,1,k,s,h)
}
