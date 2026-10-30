//! Live, caller-owned component memory. Guest pointers stay scalar across calls.
use crate::{boot::Memory, frontend};
use crate::rom_region::{Rom, addr, to_us};

/// Reentrant calls. Only platform services may be supplied by a non-Rust host.
pub trait Host {
    fn call_m(&mut self, entry: u32, args: &[u32], m: &mut Memory<'_>) -> u32;
    /// The ROM whose guest memory is live. US by default so US-only hosts
    /// are unchanged; region-aware hosts (c4's TokenHost/DictEnv, the
    /// call_m_with shim) carry Eu/Kr and every data literal follows it.
    fn rom(&self) -> Rom {
        Rom::Us
    }
}

/// Read a guest word. `None` when the address is outside the caller's arena.
/// A live port must not panic on a hostile arena: every guest pointer read on
/// a port path goes through these helpers and fails closed instead.
fn guest_word(m: &Memory<'_>, p: u32) -> Option<u32> {
    let at = p.checked_sub(m.base)? as usize;
    let b = m.bytes.get(at..at.checked_add(4)?)?;
    Some(u32::from_le_bytes(b.try_into().ok()?))
}

/// Read a guest half word. `None` when the address is outside the arena.
fn guest_half(m: &Memory<'_>, p: u32) -> Option<u16> {
    let at = p.checked_sub(m.base)? as usize;
    let b = m.bytes.get(at..at.checked_add(2)?)?;
    Some(u16::from_le_bytes(b.try_into().ok()?))
}

/// Read a guest byte. `None` when the address is outside the arena.
fn guest_byte(m: &Memory<'_>, p: u32) -> Option<u8> {
    let at = p.checked_sub(m.base)? as usize;
    Some(*m.bytes.get(at)?)
}

/// True when `len` bytes at `p` are inside the caller's arena.
fn guest_range(m: &Memory<'_>, p: u32, len: u32) -> bool {
    let at = match p.checked_sub(m.base) {
        Some(v) => v as usize,
        None => return false,
    };
    match at.checked_add(len as usize) {
        Some(end) => end <= m.bytes.len(),
        None => false,
    }
}


/// fn_2401a0. Direct guest-memory form of regex::pcre_init.
/// No arena borrow survives a callback. Allocation sizes use guest wrapping arithmetic.
/// A malformed object (an unreadable owner, heap or block pointer) fails closed
/// with 0x8a50200a instead of reading outside the arena.
pub fn pcre_init_m(
    m: &mut Memory<'_>,
    p: u32,
    key: u32,
    capacity: i32,
    width: i32,
    h: &mut impl Host,
) -> u32 {
    let valid = frontend::validate_handle(p != 0, key, 0xdd57, 24);
    if valid >= 0x80000000 {
        return valid;
    }
    if !guest_range(m, p, 24) {
        return 0x8a50200a;
    }
    let owner = m.word(p);
    let log = match guest_word(m, owner.wrapping_add(16)) {
        Some(v) => v,
        None => return 0x8a50200a,
    };
    let heap = match guest_word(m, owner.wrapping_add(4)) {
        Some(v) => v,
        None => return 0x8a50200a,
    };
    h.call_m(0x1738e8, &[log, addr(h.rom(), 0x914050), 4, 0, 0x240334], m);
    m.put_half(p + 12, 284);
    m.put_half(p + 8, 0);
    m.put_half(p + 14, capacity as u16);
    m.put_half(p + 16, 0);
    m.put_half(p + 18, 0);
    m.put_half(p + 20, 0);
    m.put_half(p + 10, width as u16);
    let array = h.call_m(
        0x1963b8,
        &[heap, 1, (capacity as u32).wrapping_add(1).wrapping_mul(4)],
        m,
    );
    m.put_word(p + 4, array);
    let mut status = 0;
    if array == 0 || !guest_range(m, array, 4) {
        status = 0x8a50200a;
    } else {
        let header = h.call_m(0x1963b8, &[heap, 1, 20], m);
        m.put_word(m.word(p + 4) + m.half(p + 8) as u32 * 4, header);
        if header == 0 || !guest_range(m, header, 16) {
            status = 0x8a50200a;
        } else {
            let size = (m.half(p + 12) as u32)
                .wrapping_mul(m.half(p + 10) as u32)
                .wrapping_add(1);
            m.put_word(header + 4, size);
            m.put_word(header + 8, 0);
            m.put_word(header, 0);
            let data = h.call_m(0x1963b8, &[heap, 1, size], m);
            m.put_word(header, data);
            if data == 0 || !guest_range(m, header + 16, 1) {
                status = 0x8a50200a;
            } else {
                let at = m.offset(header + 16);
                m.bytes[at] = 0;
                m.put_word(header + 12, 0);
                let extra = h.call_m(
                    0x1963b8,
                    &[heap, 1, m.half(p + 10) as u32 + 1],
                    m,
                );
                m.put_word(header + 12, extra);
                if extra == 0 {
                    status = 0x8a50200a;
                } else {
                    m.put_half(p + 8, m.half(p + 8).wrapping_add(1));
                    m.put_half(p + 18, m.half(p + 8));
                }
            }
        }
    }
    if status >= 0x80000000 {
        h.call_m(0x173dc0, &[log, addr(h.rom(), 0x914050), 1000, 0], m);
        h.call_m(0x23ffd4, &[p, key], m);
    }
    h.call_m(
        0x1738e8,
        &[log, addr(h.rom(), 0x914050), 4, 0, 0x240350, status],
        m,
    );
    status
}

/// fn_1e4a3c. PCRE startup with live heap and logging dispatch.
///
/// The caller supplies a `Host` that dispatches the guest calls (0x1738e8,
/// 0x173dc0, 0x1963b8, 0x23ffd4). `LiveHost` needs a `scratch` that addresses
/// 48 writable bytes inside the arena for the logger varargs; a scratch
/// outside the arena is ignored and the logger call carries no varargs. An
/// unreadable token pointer fails closed with 0x89602008: the port never
/// reads outside the caller's arena.
pub fn fn_1e4a3c_m(m: &mut Memory<'_>, p: u32, key: u32, h: &mut impl Host) -> u32 {
    if frontend::validate_handle(p != 0, key, 0xe58c, 8) >= 0x80000000 {
        return 0x89602008;
    }
    let t = match guest_word(m, p) {
        Some(v) => v,
        None => return 0x89602008,
    };
    let tag = match guest_word(m, p.wrapping_add(4)) {
        Some(v) => v,
        None => return 0x89602008,
    };
    if frontend::validate_handle(t != 0, tag, 0xe58d, 0xf4) >= 0x80000000 {
        return 0x89602008;
    }
    let ctx = match guest_word(m, t) {
        Some(v) => v,
        None => return 0x89602008,
    };
    let log = match guest_word(m, ctx.wrapping_add(16)) {
        Some(v) => v,
        None => return 0x89602008,
    };
    let id = match guest_word(m, t.wrapping_add(0x48)) {
        Some(v) => v,
        None => return 0x89602008,
    };
    h.call_m(0x1738e8, &[log, id, 4, 0, 0x1e4b44], m);
    let flag = match guest_half(m, t.wrapping_add(0x6c)) {
        Some(v) => v,
        None => return 0x89602008,
    };
    if flag == 0 {
        h.call_m(0x173dc0, &[log, addr(h.rom(), 0x8f8bec), 0x5dd8, 0], m);
        return 0x89602000;
    }
    let object = match guest_word(m, t.wrapping_add(0x30)) {
        Some(v) => v,
        None => return 0x89602008,
    };
    let pcre_key = match guest_word(m, t.wrapping_add(0x34)) {
        Some(v) => v,
        None => return 0x89602008,
    };
    let capacity = match guest_half(m, t.wrapping_add(0x4e)) {
        Some(v) => v,
        None => return 0x89602008,
    };
    let width = match guest_half(m, t.wrapping_add(0x50)) {
        Some(v) => v,
        None => return 0x89602008,
    };
    let rc = pcre_init_m(m, object, pcre_key, capacity as i32, width as i32, h);
    if rc >= 0x80000000 {
        h.call_m(0x173dc0, &[log, id, 0x5df3, 0], m);
    }
    h.call_m(0x1738e8, &[log, id, 4, 0, 0x1e4b68, rc], m);
    rc
}

/// fn_2c1e2c: apply the SET PARAM marker of each of the four kinds.
/// 1:1 with src/fn_2c1e2c.c over the live arena: engine from handles,
/// parser arrays 0x3f4/0x3fd, per-tag value scan with the tag-7 toggle,
/// then set-int / set-string / clear per kind through the host. The
/// D_9c5630 parameter table is ROM data read at its guest address.
#[allow(clippy::too_many_arguments)]
pub(crate) fn fn_2c1e2c_m(m: &mut Memory<'_>, h: &mut impl Host, ctx: u32, handle: u32, scratch: u32) -> u32 {
    const TABLE: u32 = 0x9c5630;
    let rom = h.rom();
    let table = crate::rom_region::addr(rom, TABLE);
    let etags = [
        m.half(table),
        m.half(table.wrapping_add(8)),
        m.half(table.wrapping_add(16)),
        m.half(table.wrapping_add(24)),
    ];
    let enames = [
        m.word(table.wrapping_add(4)),
        m.word(table.wrapping_add(12)),
        m.word(table.wrapping_add(20)),
        m.word(table.wrapping_add(28)),
    ];
    // Engine from the handle pair (C: fn_19611c(ctx->handles, &engine)).
    let mut result = h.call_m(0x19611c, &[m.word(ctx), m.word(ctx.wrapping_add(4)), scratch], m);
    if result >= 0x80000000 {
        return 0x81102007;
    }
    let engine = m.word(scratch);
    // First array (C: params valid only when this call succeeds).
    result = h.call_m(0x252f88, &[handle, 0x3f4, 1, scratch], m);
    let mut params = 0u32;
    if result < 0x80000000 {
        let array = m.word(scratch);
        params = m.word(array.wrapping_add(4));
    }
    // Second array (C: hard error when this call fails).
    result = h.call_m(0x252f88, &[handle, 0x3fd, 1, scratch], m);
    if result >= 0x80000000 {
        return result;
    }
    let array = m.word(scratch);
    let count = m.half(array.wrapping_add(8));
    let base = m.word(array.wrapping_add(4));
    let props = m.word(engine.wrapping_add(24));
    let heap = m.word(engine.wrapping_add(4));
    let setting = m.word(ctx.wrapping_add(96));
    // Caller-owned copy of the default parameter name (C: ParameterName tmpname).
    let mut defname = [0u16; 8];
    for i in 0..8 {
        defname[i] = m.half(table.wrapping_add(100 + i as u32 * 2));
    }
    let mut done = [false; 4];
    let mut k = 1u16;
    // Marker field layout (C: ParserMarker, 40 bytes).
    while (k as u32) < count as u32 {
        let tag = m.half(base.wrapping_add(k as u32 * 40 + 10));
        let mut t = 0usize;
        while t < 4 && etags[t] != tag {
            t += 1;
        }
        if t >= 4 || done[t] {
            k = k.wrapping_add(1);
            continue;
        }
        let mut value = m.word(base.wrapping_add(k as u32 * 40 + 28));
        if tag == 7 {
            // Tag-7 toggle scan (C: seven block).
            let mut toggle = value == 0x2b;
            let mut j = k.wrapping_add(1);
            if (j as u32) < count as u32 {
                loop {
                    let ntag = m.half(base.wrapping_add(j as u32 * 40 + 10));
                    let ne = m.word(base.wrapping_add(j as u32 * 40 + 28));
                    if ntag == 7 {
                        if ne != 0x2b {
                            value = ne;
                            toggle = false;
                        } else if !toggle {
                            toggle = true;
                        } else {
                            toggle = false;
                            value = 15;
                        }
                    }
                    j = j.wrapping_add(1);
                    if !((j as u32) < count as u32) {
                        break;
                    }
                }
            }
            if toggle {
                value = 15;
            }
        } else {
            // Forward scan with stride 1 then stride 2 (C: x = 1 & ~n;
            // the stride-1 loop runs iff x == 1, covering only base[j]).
            let j0 = (k as u32).wrapping_add(1);
            if j0 < count as u32 {
                let rest = (count as u32).wrapping_sub(k as u32);
                let n = rest.wrapping_sub(2);
                let x = 1u32 & !n;
                if x == 1 && m.half(base.wrapping_add(j0 as u16 as u32 * 40 + 10)) == tag {
                    value = m.word(base.wrapping_add(j0 as u16 as u32 * 40 + 28));
                }
                let mut q2 = (x as u16).wrapping_add(k).wrapping_add(1);
                if (q2 as u32) < count as u32 {
                    loop {
                        if m.half(base.wrapping_add(q2 as u32 * 40 + 10)) == tag {
                            value = m.word(base.wrapping_add(q2 as u32 * 40 + 28));
                        }
                        let q2p1 = q2.wrapping_add(1);
                        if m.half(base.wrapping_add(q2p1 as u32 * 40 + 10)) == tag {
                            value = m.word(base.wrapping_add(q2p1 as u32 * 40 + 28));
                        }
                        q2 = q2.wrapping_add(2);
                        if !((q2 as u32) < count as u32) {
                            break;
                        }
                    }
                }
            }
        }
        // Dispatch on the tag (C: dispatch/is7/is31/is2136/set/clear).
        let mut clear = false;
        match tag {
            7 => clear = value == 15,
            31 => clear = value == setting,
            21 | 36 => {
                if params != 0 {
                    let s = params.wrapping_add(value.wrapping_mul(2));
                    let mut eq = true;
                    for i in 0..8 {
                        let a = m.half(s.wrapping_add(i as u32 * 2));
                        if a != defname[i as usize] {
                            eq = false;
                            break;
                        }
                        if a == 0 {
                            break;
                        }
                    }
                    clear = eq;
                }
            }
            _ => {}
        }
        if clear {
            result = h.call_m(0x175a24, &[props, enames[t]], m);
            if result & 0x1fff == 0x14 {
                result = 0;
            } else {
                if result >= 0x80000000 {
                    return result;
                }
                result = 0;
            }
        } else {
            match tag {
                7 | 31 => {
                    result = h.call_m(0x1758ac, &[props, enames[t], value], m);
                    if result >= 0x80000000 {
                        return result;
                    }
                }
                21 | 36 => {
                    let s = params.wrapping_add(value.wrapping_mul(2));
                    let len = crate::frontend::u16_len(m.bytes, s);
                    let buf = h.call_m(0x1963b8, &[heap, 1, len.wrapping_add(1)], m);
                    if buf == 0 {
                        return 0x8110200a;
                    }
                    crate::component_live_c2::fn_17649c(m, buf, s, len);
                    m.put_byte(buf.wrapping_add(len), 0);
                    result = h.call_m(0x1759a0, &[props, enames[t], buf], m);
                    if result >= 0x80000000 {
                        h.call_m(0x196428, &[heap, buf], m);
                        return result;
                    }
                    h.call_m(0x196428, &[heap, buf], m);
                }
                _ => {}
            }
        }
        done[t] = true;
        k = k.wrapping_add(1);
    }
    // Clear every kind never set (C: final done[] sweep).
    let mut t = 0usize;
    while t < 4 {
        if !done[t] {
            result = h.call_m(0x175a24, &[props, enames[t]], m);
            if result & 0x1fff == 0x14 {
                result = 0;
            } else {
                if result >= 0x80000000 {
                    return result;
                }
                result = 0;
            }
        }
        t += 1;
    }
    result
}

/// fn_19672c inline (src/fn_19672c.c): erase the map entry for `key`.
/// Mirrors engine_live::fn_19672c_m over this host's dispatcher: find via
/// the map comparator, flag-walk, rb_erase, then release key, value and
/// node through the map vtable and heap.
fn fn_19672c_inline(m: &mut Memory<'_>, h: &mut impl Host, p: u32, key: u32) -> u32 {
    if p == 0 || key == 0 {
        return 0x89002007;
    }
    let o = p.wrapping_add(0x18);
    let cmp = m.word(m.word(o.wrapping_add(36)).wrapping_add(8));
    let mut n = m.word(o);
    let mut found = 0u32;
    while n != o {
        let r = h.call_m(cmp, &[key, m.word(n.wrapping_add(16))], m) as i32;
        if r < 0 {
            n = m.word(n);
        } else if r > 0 {
            n = m.word(n.wrapping_add(4));
        } else {
            found = n;
            break;
        }
        if n == 0 {
            break;
        }
    }
    if found == 0 {
        return 0x89002014;
    }
    if m.byte(o.wrapping_add(40)) != 0 {
        let mut last = found;
        loop {
            let mut cur = m.word(last);
            while cur != o {
                if h.call_m(cmp, &[key, m.word(cur.wrapping_add(16))], m) as i32 == 0 {
                    break;
                }
                cur = m.word(cur.wrapping_add(4));
            }
            if cur == o {
                break;
            }
            last = cur;
        }
        found = last;
    }
    crate::normalization::rb_erase(m.bytes, o, found);
    let heap = m.word(o.wrapping_add(0x20));
    let vt = m.word(o.wrapping_add(0x24));
    let k = m.word(found.wrapping_add(0x10));
    if k != 0 {
        h.call_m(m.word(vt.wrapping_add(4)), &[k, heap], m);
    }
    let v = m.word(found.wrapping_add(0x14));
    if v != 0 {
        h.call_m(m.word(vt.wrapping_add(0x10)), &[v, heap], m);
    }
    h.call_m(0x196428, &[heap, found], m);
    0
}

/// fn_175a24: clear the named parameter (src/fn_175a24.c).
/// Lock, look up the value, refuse when callbacks are set, else push the
/// cleared value through the update entry and remove the key. `scratch` is
/// one writable guest word for the lookup out-param.
pub(crate) fn fn_175a24_m(m: &mut Memory<'_>, h: &mut impl Host, props: u32, key: u32, scratch: u32) -> u32 {
    if props == 0 || key == 0 {
        return 0x8b702007;
    }
    let lock = m.word(props.wrapping_add(12));
    let mut result = h.call_m(0x176f2c, &[lock], m);
    if result >= 0x80000000 {
        return result;
    }
    let values = m.word(props.wrapping_add(16));
    let rc = h.call_m(0x1966ac, &[values, key, scratch], m);
    if rc < 0x80000000 {
        let value = m.word(scratch);
        let (data, size) = if value == 0 || m.word(value.wrapping_add(4)) == 0 {
            (0, 0)
        } else {
            (m.word(value), m.word(value.wrapping_add(4)))
        };
        if value != 0 && m.word(value.wrapping_add(8)) != 0
            && h.call_m(0x1970a0, &[m.word(value.wrapping_add(8))], m) != 0
        {
            result = 0x8b702011;
        } else {
            result = h.call_m(0x174b38, &[props, key, data, size, 0], m);
            if result < 0x80000000 {
                let rm = fn_19672c_inline(m, h, values, key);
                result = rm;
                if rm >= 0x80000000 {
                    result = (rm & 0x1fff) | 0x8b702000;
                }
            }
        }
    } else {
        result = (rc & 0x1fff) | 0x8b702000;
    }
    let unlock = h.call_m(0x176f5c, &[lock], m);
    if unlock >= 0x80000000 && result < 0x80000000 {
        result = unlock;
    }
    result
}
pub struct LiveHost<'a> {
    pub platform: &'a mut dyn crate::engine::HeapPlatform,
    pub fault: Option<crate::engine::Error>,
    pub scratch: u32,
}

impl LiveHost<'_> {
    /// Record an unbound or unusable guest call and return the failure
    /// status. The live ports fail closed: they never panic on a caller
    /// arena, a missing adapter or a malformed guest pointer.
    fn faulted(&mut self, f: u32) -> u32 {
        panic!("FAULTED {:#x}", f);
    }
    /// True when `len` bytes at `addr` are writable inside the arena.
    fn scratch_ok(&self, m: &Memory<'_>, addr: u32, len: u32) -> bool {
        guest_range(m, addr, len)
    }
}
/// A [`LiveHost`] call carrying the live region explicitly. The [`Host`]
/// impl routes through here with the entry-derived region; region-aware
/// callers (component_live_c4) pass their own `rom` with regional entries.
impl LiveHost<'_> {
    pub fn call_m_with(
        &mut self,
        f: u32,
        a: &[u32],
        m: &mut Memory<'_>,
        rom: Rom,
    ) -> u32 {
        self.dispatch(f, a, m, rom)
    }

    fn dispatch(&mut self, f: u32, a: &[u32], m: &mut Memory<'_>, rom: Rom) -> u32 {
        // Regional entries name the same port the US dispatch serves. The
        // caller's region disambiguates overlapping addresses.
        let f = to_us(rom, f);
        // back with this token for its varargs (lane c3's MorphHost serves
        // the same token with 0). It carries no state.
        if f == 0xffff0001 {
            return 0;
        }
        if f == 0x15db2c {
            let at = addr(rom, 0x8d7bb8) + a[2] * 8;
            let table = [[m.word(at), m.word(at + 4)]];
            let mut out = 0;
            let rc = crate::linguistic::field_property(&table, 0, &mut out);
            m.put_word(a[3], out);
            return rc;
        }
        if f == 0x249cec {
            return crate::speech_part_1::fn_249cec(m, a[0], 0x4200, self);
        }
        if f == 0x24ac00 {
            return crate::speech_part_1::fn_24ac00(m, a[0], a[1], a[2], 0x4800, self);
        }
        if f == 0x24bbc8 {
            let shared = speech_literals(m, rom);
            let literals = domain_literals(m, rom);
            return crate::speech_part_2::fn_24bbc8(m, a[0], &shared, &literals, 0x5000, self);
        }
        if f == 0x24c470 {
            let literals = speech_literals(m, rom);
            return crate::speech_part_2::fn_24c470(m, a[0], &literals, 0x5800, self);
        }
        // fn_226ef8 (round 1, seg5): the shared-stream copy the write
        // path runs through the stream's copy slot (state+32). The
        // certified port ignores its first two args and retains at p.
        if f == 0x226ef8 {
            return crate::speech_part_2::fn_226ef8(m, a[0], a[1], a[2]);
        }
        if f == 0x15cdcc {
            let mut descs = [0u8; 1360];
            let anchor = addr(rom, 0x8d7ca0);
            descs.copy_from_slice(&m.bytes[anchor as usize..anchor as usize + 1360]);
            // The original compares the descriptor anchor with D_8d7c98.
            let word_anchor = addr(rom, 0x8d7c98);
            return write_field_m(
                m.bytes,
                nz(a[0]),
                a[1],
                a[2],
                a[3],
                a[4],
                nz(a[5]),
                a[6],
                &descs,
                word_anchor,
                &mut WithRom { inner: &mut *self, rom },
            );
        }
        if f == 0x15bf74 {
            
            let mut args = [0u32; 5];
            let n = a.len().min(5);
            args[..n].copy_from_slice(&a[..n]);
            let mut parent_types = [0u8; 8];
            if let Some(s) = m.bytes.get(addr(rom, 0x8d7b50) as usize..addr(rom, 0x8d7b50) as usize + 8) {
                parent_types.copy_from_slice(s);
            }
            let scratch = self.scratch;
            return fn_15bf74_m(
                m,
                args,
                scratch.wrapping_add(0x300),
                &parent_types,
                &mut WithRom { inner: &mut *self, rom },
            );
        }
        if f == 0x199458 {
            if a.len() < 3 {
                return self.faulted(f);
            }
            return crate::boot_part_b2::fn_199458(m, a[0], a[1], a[2]);
        }
        if f == 0x30253c {
            if a.len() < 3 {
                return self.faulted(f);
            }
            return fn_30253c_m(m, a[0], a[1], a[2]);
        }
        // EU/KR 206be4's archive read reaches the stream's extend callback
        // through its regional twin (EU 0x172ec0 / KR 0x172e6c); the ROM
        // returns 0 (src/fn_172e10.c), so the port answers 0 with no guest
        // write. US never issues this call.
        if f == 0x172e10 {
            return 0;
        }
        if f == 0x30209c {
            if a.len() < 2 {
                return self.faulted(f);
            }
            // The runtime returns the packed 64-bit value; the dispatcher
            // carries the low word, the quotient.
            return fn_30209c_m(a[0] as i32, a[1] as i32) as u32;
        }
        if matches!(f, 0x26b2f8 | 0x26b37c | 0x26b444 | 0x26b530) {
            // The morphology family of the fe_text unit. Lane c4 owns the
            // ports; the flat view lets them run against the caller's arena.
            if a.is_empty() {
                return self.faulted(f);
            }
            let mut regions = [crate::engine::Region {
                base: 0,
                bytes: m.bytes,
            }];
            let mut view = match crate::engine::Memory::new(&mut regions) {
                Ok(v) => v,
                Err(e) => {
                    self.fault = Some(e);
                    return 0x80002001;
                }
            };
            let rc = match f {
                0x26b2f8 => {
                    crate::component_live_c4::fn_26b2f8_m(&mut view, a[0], a[1], a[2])
                }
                0x26b37c => crate::component_live_c4::fn_26b37c_m(&mut view, a[0]),
                0x26b444 => crate::component_live_c4::fn_26b444_m(&mut view, a[0]),
                _ => crate::component_live_c4::fn_26b530_m(&mut view, a[0]),
            };
            return match rc {
                Ok(v) => v,
                Err(e) => {
                    self.fault = Some(e);
                    0x80002001
                }
            };
        }
        if f == 0x15c1e4 || f == 0x15c2f0 {
            // The new record pointer and the created id live in the
            // caller's scratch. The target kept them on its stack, so the
            // port restores the scratch and leaves no byte behind.
            let at = self.scratch.wrapping_add(0x300) as usize;
            if m.bytes.len() < at + 16 {
                return self.faulted(f);
            }
            let mut saved = [0u8; 16];
            saved.copy_from_slice(&m.bytes[at..at + 16]);
            let mut args = [0u32; 4];
            let n = a.len().min(4);
            args[..n].copy_from_slice(&a[..n]);
            let scratch = self.scratch;
            let rc = if f == 0x15c1e4 {
                fn_15c1e4_m(m, args, scratch.wrapping_add(0x300), &mut WithRom { inner: &mut *self, rom })
            } else {
                fn_15c2f0_m(m, args, scratch.wrapping_add(0x300), &mut WithRom { inner: &mut *self, rom })
            };
            m.bytes[at..at + 16].copy_from_slice(&saved);
            return rc;
        }
        if f == 0x15d83c {
            let mut args = [0u32; 3];
            let n = a.len().min(3);
            args[..n].copy_from_slice(&a[..n]);
            return fn_15d83c_m(m, args, &mut WithRom { inner: &mut *self, rom });
        }
        if f == 0x15da64 {
            let mut args = [0u32; 3];
            let n = a.len().min(3);
            args[..n].copy_from_slice(&a[..n]);
            return fn_15da64_m(m, args, &mut WithRom { inner: &mut *self, rom });
        }
        // The depes layer and module group of the fn_1e7908 capture. The
        // lane c ports own the behaviour; the adapter keeps the arena.
        if f == 0x218f88 || f == 0x218ba4 {
            if a.len() < 5 {
                return self.faulted(f);
            }
            let mut adapter = ComponentCHost { host: self, rom };
            return crate::component_live_c::depes_set_layer_m(
                m,
                a[0],
                a[1],
                a[2],
                a[3],
                a[4],
                &mut adapter,
            );
        }
        if f == 0x2193a4 {
            if a.len() < 3 {
                return self.faulted(f);
            }
            let mut workspace = crate::component_live_c::VmWorkspace::default();
            let mut adapter = ComponentCHost { host: self, rom };
            return crate::component_live_c::depes_exec_module_m(
                m,
                a[0],
                a[1],
                a[2],
                &mut workspace,
                &mut adapter,
            );
        }
        // fn_1ed1d0: fe_dctlkp_ConvertToCanonicalToken (lane us r23).
        // The certified canonicalizer port runs here over the live arena.
        if f == 0x1ed1d0 {
            if a.len() < 4 {
                return self.faulted(f);
            }
            let mut adapter = ComponentCHost { host: self, rom };
            return crate::component_live_c::dctlkp_canonical_token_m(
                m,
                a[0],
                a[1],
                a[2],
                a[3],
                &mut adapter,
            );
        }
        // fn_175a24: clear the named parameter (lane us r23).
        if f == 0x175a24 {
            if a.len() < 2 {
                return self.faulted(f);
            }
            let sc = self.scratch;
            return fn_175a24_m(m, self, a[0], a[1], sc);
        }
        // fn_2c1e2c: SET PARAM marker parser (lane us r23).
        // The 1:1 port above runs here over the live arena.
        if f == 0x2c1e2c {
            if a.len() < 2 {
                return self.faulted(f);
            }
            let sc = self.scratch;
            return fn_2c1e2c_m(m, self, a[0], a[1], sc);
        }
        // The heap ports of the record allocator. The block headers they
        // write are part of the persistent image.
        if f == 0x1850c0 {
            if a.len() < 4 || !guest_range(m, a[0], 64) {
                return self.faulted(f);
            }
            return crate::boot_port_1::fn_1850c0(m, a[0], a[1], a[2] as i32);
        }
        if f == 0x1852e8 {
            if a.len() < 4 || !guest_range(m, a[0], 64) {
                return self.faulted(f);
            }
            return crate::boot_port_1::fn_1852e8(m, a[0], a[1], a[2]);
        }
        if f == 0x19b814 {
            if a.len() < 5 || !guest_range(m, a[0], 32) || !guest_range(m, a[1], 16) {
                return self.faulted(f);
            }
            return crate::boot_port_1::fn_19b814(m, a[0], a[1], a[2], a[3], a[4]);
        }
        if f == 0x17c7d0 {
            let at = addr(rom, 0x8dc30c) as usize;
            let v = match m.bytes.get(at..at + 512) {
                Some(t) => fn_17c7d0_m(t, a[0]),
                None => 0,
            };
            return v;
        }
        // The remaining calls of the 1f98c8 and 1e7908 captures that reach
        // an existing Rust port. Every guest range is checked first: the
        // live ports fail closed instead of indexing outside the arena.
        if f == 0x195ae8 || f == 0x195b38 {
            if !guest_range(m, a[0], a[2]) || !guest_range(m, a[1], a[2]) {
                return self.faulted(f);
            }
            return crate::frontend::move_bytes(m.bytes, a[0], a[1], a[2]);
        }
        if f == 0x195bec {
            if !guest_range(m, a[0], a[2]) {
                return self.faulted(f);
            }
            return crate::frontend::fill_bytes(m.bytes, a[0], a[1], a[2]);
        }
        if f == 0x195c90 {
            let dl = match guest_str_len(m, a[0]) {
                Some(v) => v,
                None => return self.faulted(f),
            };
            let sl = match guest_str_len(m, a[1]) {
                Some(v) => v,
                None => return self.faulted(f),
            };
            if !guest_range(m, a[0], dl.wrapping_add(sl).wrapping_add(1)) {
                return self.faulted(f);
            }
            return crate::frontend::append_str(m.bytes, a[0], a[1]);
        }
        if f == 0x195cd0 {
            let sl = match guest_str_len(m, a[1]) {
                Some(v) => v,
                None => return self.faulted(f),
            };
            if !guest_range(m, a[0], sl.wrapping_add(1)) {
                return self.faulted(f);
            }
            return crate::frontend::copy_str(m.bytes, a[0], a[1]);
        }
        if f == 0x195d30 {
            if guest_str_len(m, a[0]).is_none() {
                return self.faulted(f);
            }
            return crate::frontend::find_byte(m.bytes, a[0], a[1]);
        }
        if f == 0x195d58 {
            let al = match guest_str_len(m, a[0]) {
                Some(v) => v,
                None => return self.faulted(f),
            };
            let bl = match guest_str_len(m, a[1]) {
                Some(v) => v,
                None => return self.faulted(f),
            };
            // Compare with the terminators attached: without them equal
            // strings, prefixes and empty literals read past the slice.
            let left = match m.bytes.get(a[0] as usize..a[0] as usize + al.wrapping_add(1) as usize) {
                Some(v) => v,
                None => return self.faulted(f),
            };
            let right = match m.bytes.get(a[1] as usize..a[1] as usize + bl.wrapping_add(1) as usize) {
                Some(v) => v,
                None => return self.faulted(f),
            };
            return crate::frontend::compare_str(left, right) as u32;
        }
        if f == 0x1960a8 {
            if guest_str_len(m, a[0]).is_none() || guest_str_len(m, a[1]).is_none() {
                return self.faulted(f);
            }
            return crate::frontend::find_substr(m.bytes, a[0], a[1]);
        }
        if f == 0x196764 {
            return crate::selection::arena_leaf::word_at_12(
                &crate::selection::arena::Arena::new(m.base, m.bytes),
                a[0],
            );
        }
        if f == 0x1994d0 {
            return crate::frontend::divmod_u32(a[0], a[1]).0;
        }
        if f == 0x176ac8 {
            if !guest_range(m, a[1], 64) {
                return self.faulted(f);
            }
            return crate::normalization::itoa(m.bytes, a[0], a[1], a[2]);
        }
        if f == 0x17c550 {
            let at = m.offset(a[0]);
            let text = match m.bytes.get(at..) {
                Some(v) => v,
                None => return self.faulted(f),
            };
            return crate::pronunciation::parse_int(text) as u32;
        }
        if f == 0x15b758 {
            
            let mut adapter = DbAdapter(self);
            return crate::db_live::create_record_m(
                &mut adapter,
                m,
                a[0],
                a[1],
                a[2],
                addr(rom, 0x8d8274),
                addr(rom, 0x8d7ca0),
            );
        }
        if f == 0x180d3c {
            let mut adapter = DbAdapter(self);
            return crate::db_live::arena_alloc_m(&mut adapter, m, a[0], a[1]);
        }
        if f == 0x176f94 {
            return crate::frontend::validate_handle(a[0] != 0, a[1], a[2], a[3]);
        }
        if f == 0x195d90 {
            // `guest_str_len` already reports the C length (the offset of
            // the first NUL); slicing the terminator off and scanning the
            // remainder for one panics on every input, so return it.
            return match guest_str_len(m, a[0]) {
                Some(v) => v,
                None => self.faulted(f),
            };
        }
        if f == 0x195dac {
            return crate::component_live_c::fn_195dac_m(m, a[0], a[1], a[2] as i32);
        }
        if f == 0x240b28 {
            return crate::component_live_c::fn_240b28(a[0]);
        }
        if f == 0x1852d8 {
            if !guest_range(m, a[0], 0x36) {
                return self.faulted(f);
            }
            return crate::component_live_c::fn_1852d8_m(m, a[0], a[1] as u16) as u32;
        }
        if f == 0x1853c4 {
            if !guest_range(m, a[0], 0x39) {
                return self.faulted(f);
            }
            return crate::component_live_c::fn_1853c4_m(m, a[0], a[1] as i8) as u32;
        }
        if f == 0x2191c8 {
            return crate::component_live_c::depes_get_layer_m(m, a[0], a[1], a[2], a[3], a[4]);
        }
        if f == 0x1769d0 {
            if guest_str_len(m, a[0]).is_none() || guest_str_len(m, a[1]).is_none() {
                return self.faulted(f);
            }
            return crate::fe_text::strcmp_ci_1769d0(m.bytes, a[0], a[1]) as u32;
        }
        if f == 0x17c8c0 {
            return crate::fe_text::fold_17c8c0(a[0]);
        }
        if f == 0x19b78c {
            crate::boot_part_b2::fn_19b78c_guest(m, a[0], a[1]);
            return 0;
        }
        if f == 0x26bb54 {
            return crate::component_live_c::fn_26bb54_m(m, a[0]) as u32;
        }
        // The broker and stream range of the 1f98c8 capture. The lane c
        // ports own these; the ABI is the shared broker range.
        if matches!(
            f,
            0x17a1a4
                | 0x179208
                | 0x17c9d8
                | 0x179374
                | 0x1b2760
                | 0x172de0
                | 0x172d30
                | 0x4a794c
                | 0x4a78f0
                | 0x1b254c
                | 0x1b2974
        ) {
            let scratch = self.scratch.wrapping_add(0x400);
            let mut adapter = ComponentCHost { host: self, rom };
            return crate::component_live_c::broker_range_live_m(
                m,
                f,
                a,
                scratch,
                &mut adapter,
            );
        }
        if f == 0x1fef0c {
            let mut adapter = PromptDomainAdapter {
                io: PreprocessAdapter {
                    host: self,
                    processor: a[0],
                    scratch: 0xe00,
                },
            };
            return crate::frontend_part_b::fn_1fef0c(
                m.bytes,
                a[0],
                a[1],
                a[2],
                a[3],
                a[4],
                [a[5], a[6]],
                [a[7], a[8]],
                [a[9], a[10]],
                a[11],
                &mut adapter,
            );
        }
        if f == 0x15c544 {
            let mut regions = [crate::engine::Region {
                base: 0,
                bytes: m.bytes,
            }];
            let mut view = match crate::engine::Memory::new(&mut regions) {
                Ok(v) => v,
                Err(e) => {
                    self.fault = Some(e);
                    return 0x80002001;
                }
            };
            // The child lookup reads the region's parent-type table.
            let mut calls = crate::engine_components::RegionalCalls { rom };
            return match crate::engine_components::InternalCalls::call(
                &mut calls,
                &mut view,
                f,
                a,
            ) {
                Ok(rc) => rc,
                Err(e) => {
                    self.fault = Some(e);
                    0x80002001
                }
            };
        }
        if matches!(f, 0x15d594 | 0x15d378 | 0x15d0c4 | 0x15c7d8) {
            return database_read_m(m, f, a, &mut WithRom { inner: &mut *self, rom });
        }

        if matches!(
            f,
            0x14f010
                | 0x14f918
                | 0x151744
                | 0x196db0
                | 0x196a64
                | 0x14e084
                | 0x14df44
                | 0x19690c
                | 0x14e0b4
                | 0x19668c
                | 0x176054
                | 0x1758ac
                | 0x175850
                | 0x176f2c
                | 0x176f5c
                | 0x174a04
                | 0x1749ac
        ) {
            let mut saved = [0u8; 4352];
            if m.bytes.len() < 0x4100 {
                return self.faulted(f);
            }
            saved.copy_from_slice(&m.bytes[0x3000..0x4100]);
            let mut calls = crate::component_live_a2::LiveCalls::new(self.platform, 0x3000).with_rom(rom);
            let rc = crate::component_live_a2::Calls::call(&mut calls, m, f, a);
            self.fault = calls.fault();
            m.bytes[0x3000..0x4100].copy_from_slice(&saved);
            if self.fault.is_some() {
                return 0x80002001;
            }
            return rc;
        }
        if matches!(f, 0x175944 | 0x175a1c | 0x1756d0) {
            return get_property_m(m, f, a, &mut WithRom { inner: &mut *self, rom });
        }
        if matches!(f, 0x1966ac | 0x196980 | 0x175170 | 0x174b38 | 0x176104 | 0x1759a0) {
            return crate::speech_part_1::parameter_call_m(m, f, a, 0x1800, self);
        }
        if f == 0x2643f8 {
            
            let env = crate::speech_part_1::SequenceEnvironment {
                cached_handle: core::array::from_fn(|i| {
                    m.byte(addr(rom, 0x8d7660) + i as u32)
                }),
                default_context: addr(rom, 0x8d7640),
                eos: literal(m.bytes, b"eosdetection"),
                direct_size: literal(m.bytes, b"ppreaddirectsize"),
            };
            return crate::speech_part_1::fn_2643f8(m, a[0], a[1], a[2], &env, 0x900, self);
        }
        if f == 0x2c47d0 {
            return reset_cursor_m(m, a[0], a[1], &mut WithRom { inner: &mut *self, rom });
        }
        if f == 0x2c2338 {
            return reset_markers_m(m, a[0], a[1], 0xb00, &mut WithRom { inner: &mut *self, rom });
        }
        if f == 0x24c9fc {
            return crate::speech_part_6::fn_24c9fc(m, self, a[0], 0xa00);
        }
        if f == 0x151610 || f == 0x1516c8 {
            let cell = core::cell::RefCell::new(&mut *self);
            let mut vt = StreamHeap {
                cell: &cell,
                heap: a[1],
            };
            if f == 0x151610 {
                return duplicate_typed_string_m(m.bytes, nz(a[0]), nz(a[1]), nz(a[2]), &mut vt);
            }
            release_typed_buffer_m(m.bytes, nz(a[0]), a[1], &mut vt);
            return 0;
        }
        if f == 0x1963e0 {
            return crate::boot_part_e::fn_1963e0(m, a[0], a[1], a[2], self);
        }
        if f == 0x1503b0 || f == 0x15017c {
            return stream_call_m(m, f, a, &mut WithRom { inner: &mut *self, rom });
        }
        if f == 0x14e614 {
            let log = m.word(m.word(a[0] + 4) + 16);
            let level = self.call_m(0x173fd8, &[log], m);
            crate::frontend::log_write(m.bytes, a[0], nz(a[1]), a[2], level);
            return 0;
        }
        if f == 0x173fd8 {
            return crate::component_live_a::max_slot_level_m(m, a[0], self);
        }
        if f == 0x196794 {
            return alloc_handle_for_key_live(m, a[0], a[1], a[2], a[3], &mut WithRom { inner: &mut *self, rom });
        }
        if f == 0x196860 {
            let mut key = 0;
            let mut value = 0;
            let rc = frontend::step_iter(
                m.bytes,
                nz(a[0]),
                nz(a[1]).map(|_| &mut key),
                nz(a[2]).map(|_| &mut value),
            );
            if a[1] != 0 {
                m.put_word(a[1], key);
            }
            if a[2] != 0 {
                m.put_word(a[2], value);
            }
            return rc;
        }
        if f == 0x196830 {
            return crate::component_live_a::release_owner_m(m, a[0], self);
        }
        if f == 0x1970a0 {
            return crate::normalization::idx5(m.bytes, a[0]);
        }
        if f == 0x1fe600 {
            let args: [u32; 5] = match a.try_into() {
                Ok(v) => v,
                Err(_) => return self.faulted(f),
            };
            let scratch = self.scratch;
            return fn_1fe600_m(m, args, scratch + 0x200, &mut WithRom { inner: &mut *self, rom });
        }
        if f == 0x18f488 {
            return frontend::finish_request();
        }
        if f == 0x1ce56c {
            return frontend::streaming_mode(
                (a[0] != 0).then(|| (m.byte(m.word(a[0]) + 4), m.byte(m.word(a[0]) + 0x10c))),
            );
        }
        if f == 0x17497c {
            return crate::normalization::flag_byte(m.bytes, a[0]);
        }
        if f == 0x1970ac {
            return crate::boot_part_e::fn_1970ac(m, a[0], a[1], a[2]);
        }
        if f == 0x19611c {
            
            let anchor = addr(rom, 0x8d7660);
            let mut pair = [0u8; 8];
            pair.copy_from_slice(&m.bytes[anchor as usize..anchor as usize + 8]);
            let pointed = (a[0] != 0).then(|| m.half(a[0]));
            let mut ctx = 0;
            let rc = frontend::resolve_engine_context(
                a[0],
                pointed,
                a[1],
                &pair,
                addr(rom, 0x8d7640),
                &mut ctx,
            );
            m.put_word(a[2], ctx);
            return rc;
        }
        if f == 0x23ffd4 {
            return pcre_deinit_m(m, a[0], a[1], &mut WithRom { inner: &mut *self, rom });
        }
        if f == 0x2401a0 {
            return pcre_init_m(m, a[0], a[1], a[2] as i32, a[3] as i32, &mut WithRom { inner: &mut *self, rom });
        }
        match f {
            0x15140c | 0x14fe9c | 0x21958c | 0x219554 => {
                let mut regions = [crate::engine::Region {
                    base: m.base,
                    bytes: m.bytes,
                }];
                let mut view = match crate::engine::Memory::new(&mut regions) {
                    Ok(v) => v,
                    Err(e) => {
                        self.fault = Some(e);
                        return 0x80002001;
                    }
                };
                let mut reads = crate::engine_components2::RegionalReads { rom };
                let rc = crate::engine_components::InternalCalls::call(
                    &mut reads,
                    &mut view,
                    f,
                    a,
                );
                return match rc {
                    Ok(v) => v,
                    Err(e) => {
                        self.fault = Some(e);
                        0x80002001
                    }
                };
            }
            0x1963b8 => return crate::boot_part_e::calloc(m, a[0], a[1], a[2], self),
            0x196394 => return crate::boot_part_e::alloc(m, a[0], a[1], self),
            0x196428 => {
                crate::boot_part_e::release(m, a[0], a[1], self);
                return 0;
            }
            0x4a7758 => {
                let table = match guest_word(m, a[0]) {
                    Some(v) => v,
                    None => return self.faulted(f),
                };
                match guest_word(m, table + 8) {
                    Some(0) | None => return self.faulted(f),
                    Some(_) => {}
                }
                return m.fn_4a7758(self, [a[0], a[1], a[2]]);
            }
            0x163f20 => return m.fn_163f20(self, [a[0], a[1], a[2], a[3]]),
            0x39f5e8 => return m.fn_39f5e8(self, [a[0], a[1], a[2]]),
            0x184f20 => {
                let args: [u32; 6] = match a.try_into() {
                    Ok(v) => v,
                    Err(_) => return self.faulted(f),
                };
                return m.boot_heap_allocate(args);
            }
            0x184f84 => {
                m.boot_heap_release(a[0], a[1]);
                return 0;
            }
            0x1840fc => {
                crate::boot_part_b::fn_1840fc(m, a[0], self);
                return 0;
            }
            0x18423c => {
                crate::boot_part_b::fn_18423c(m, a[0], self);
                return 0;
            }
            0x183edc => {
                let handle = m.word(self.platform.arbiter_address());
                crate::boot_part_b2::fn_183edc(m, a[0], handle, self);
                return 0;
            }
            0x39f5b8 => {
                crate::speech_part_3::fn_39f5b8(m, a[0], a[1], self);
                return 0;
            }
            0x4a774c => return crate::speech_part_3::fn_4a774c(m, a[0], a[1], self),
            0x4a779c => return m.fn_4a779c(self, a[0], a[1]),
            // fn_4a79b8: the heap realloc the stream growth path needs
            // (round 27b, wire-a): the 22d6dc insertion pump grows its
            // stream through here on the full run. Same shape as the
            // heap arms above; the certified speech_part_3 port runs
            // over this host.
            0x4a79b8 => {
                if a.len() < 3 {
                    return self.faulted(f);
                }
                return crate::speech_part_3::fn_4a79b8(m, a[0], a[1], a[2], self);
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
                    semaphore: self.platform.arbiter_address(),
                    service: 0,
                    command: 0,
                };
                m.fn_165d00(self, &g, a[0]);
                return 0;
            }
            0x172d18 => return self.platform.arbitrate(m, a),
            _ => {}
        }
        // The varargs block lives in the caller's arena. A scratch outside
        // the arena is unusable: the logger call then carries no varargs
        // instead of writing outside the caller's memory.
        let scratch = self.scratch;
        let scratch_ok = self.scratch_ok(m, scratch, 48);
        let mut boot = crate::engine::SpeechBootHost {
            platform: self.platform,
            fault: self.fault,
        };
        // Short log calls are zero-padded the way the live engine pads
        // them: a caller whose format takes no varargs leaves the tail
        // undefined in the ARM, so it is zero here.
        let rc = if f == 0x1738e8 {
            if !scratch_ok {
                let mut args = [0u32; 6];
                let n = a.len().min(6);
                args[..n].copy_from_slice(&a[..n]);
                boot.invoke(m, f, &args)
            } else {
                let mut saved = [0u32; 12];
                for (i, v) in saved.iter_mut().enumerate() {
                    *v = m.word(scratch + i as u32 * 4);
                }
                let mut head = [0u32; 5];
                let n = a.len().min(5);
                head[..n].copy_from_slice(&a[..n]);
                for (i, v) in a.get(5..).unwrap_or(&[]).iter().enumerate() {
                    m.put_word(scratch + i as u32 * 4, *v);
                }
                let rc = boot.invoke(m, f, &[head[0], head[1], head[2], head[3], head[4], scratch]);
                for (i, v) in saved.iter().enumerate() {
                    m.put_word(scratch + i as u32 * 4, *v);
                }
                rc
            }
        } else if f == 0x173dc0 {
            boot.invoke(m, f, &[a[0], a[1], a[2], a[3], 0, 0])
        } else {
            boot.invoke(m, f, a)
        };
        self.fault = boot.fault;
        if self.fault.is_some() {
            // An unbound guest dependency fails closed. The caller checks
            // `fault` after the port returns; the port itself never panics.
            return 0x80002001;
        }
        rc
    }
}

impl Host for LiveHost<'_> {
    fn call_m(&mut self, f: u32, a: &[u32], m: &mut Memory<'_>) -> u32 {
        // Regional entries name the same port: normalize, then run the US
        // dispatch with the entry's region. Region-aware callers use
        // `call_m_with` with their own `rom` instead.
        let rom = Rom::from_entry(f);
        self.call_m_with(f, a, m, rom)
    }
}

/// A [`Host`] view of a [`LiveHost`] that reports the live region.
/// `dispatch` passes this (not `self`) to the free ports so their data
/// literals follow `rom` while nested dispatches keep working.
struct WithRom<'x, 'a> {
    inner: &'x mut LiveHost<'a>,
    rom: Rom,
}
impl Host for WithRom<'_, '_> {
    fn call_m(&mut self, entry: u32, args: &[u32], m: &mut Memory<'_>) -> u32 {
        self.inner.call_m_with(entry, args, m, self.rom)
    }
    fn rom(&self) -> Rom {
        self.rom
    }
}

impl crate::boot::Calls for LiveHost<'_> {
    fn call(&mut self, m: &mut Memory<'_>, f: u32, a: &[u32]) -> u32 {
        self.call_m(f, a, m)
    }
    fn call_wide(&mut self, _: &mut Memory<'_>, f: u32, _: &[u32]) -> u64 {
        // No live port returns a 64-bit value. The fault is recorded and the
        // caller sees zero instead of a panic.
        self.faulted(f);
        0
    }
}
impl crate::boot_part_e::Host for LiveHost<'_> {
    fn call(&mut self, m: &mut Memory<'_>, f: u32, a: &[u32]) -> u32 {
        self.call_m(f, a, m)
    }
}

/// fn_23ffd4. Memory-passing failure cleanup, retaining callback write order.
pub fn pcre_deinit_m(m: &mut Memory<'_>, p: u32, key: u32, h: &mut impl Host) -> u32 {
    let rc = frontend::validate_handle(p != 0, key, 0xdd57, 24);
    if rc >= 0x80000000 {
        return rc;
    }
    if !guest_range(m, p, 24) {
        return rc;
    }
    let owner = m.word(p);
    let log = match guest_word(m, owner.wrapping_add(16)) {
        Some(v) => v,
        None => return rc,
    };
    let heap = match guest_word(m, owner.wrapping_add(4)) {
        Some(v) => v,
        None => return rc,
    };
    // Source-backed logging stays live even during partial initialization.
    h.call_m(0x1738e8, &[log, addr(h.rom(), 0x914050), 4, 0, 0x240178], m);
    let blocks = m.word(p + 4);
    let count = m.half(p + 8);
    let mut i = 1u16;
    while i < count {
        let slot = blocks.wrapping_add(i as u32 * 4);
        if !guest_range(m, slot, 4) {
            break;
        }
        let entry = m.word(slot);
        if entry != 0 {
            h.call_m(0x196428, &[heap, m.word(entry)], m);
            m.put_word(entry, 0);
            if guest_range(m, entry + 12, 4) {
                h.call_m(0x196428, &[heap, m.word(entry + 12)], m);
                m.put_word(entry + 12, 0);
            }
        }
        h.call_m(0x196428, &[heap, m.word(slot)], m);
        m.put_word(slot, 0);
        i = i.wrapping_add(1);
    }
    m.put_half(p + 8, 1);
    m.put_half(p + 16, 0);
    m.put_half(p + 18, 0);
    m.put_half(p + 20, 0);
    if blocks != 0 && guest_range(m, blocks, 4) {
        let entry = m.word(blocks);
        if entry != 0 {
            h.call_m(0x196428, &[heap, m.word(entry)], m);
            m.put_word(entry, 0);
            if guest_range(m, entry + 12, 4) {
                h.call_m(0x196428, &[heap, m.word(entry + 12)], m);
                m.put_word(entry + 12, 0);
            }
            h.call_m(0x196428, &[heap, entry], m);
            m.put_word(blocks, 0);
        }
        h.call_m(0x196428, &[heap, blocks], m);
        m.put_word(p + 4, 0);
    }
    h.call_m(
        0x1738e8,
        &[log, addr(h.rom(), 0x914050), 4, 0, 0x240190, rc],
        m,
    );
    0
}

impl crate::boot_part_b::Host for LiveHost<'_> {
    fn thread_id(&mut self) -> u32 {
        self.platform.thread_id()
    }
    fn call(&mut self, m: &mut Memory<'_>, f: u32, a: &[u32]) -> u32 {
        self.call_m(f, a, m)
    }
}
impl crate::speech_part_3::Host for LiveHost<'_> {
    fn call(&mut self, m: &mut Memory<'_>, f: u32, a: &[u32]) -> u32 {
        self.call_m(f, a, m)
    }
}

use crate::frontend::u16_len;
fn mem_word(b: &[u8], p: u32) -> u32 {
    u32::from_le_bytes(b[p as usize..p as usize + 4].try_into().unwrap())
}
fn mem_store(b: &mut [u8], p: u32, v: u32) {
    b[p as usize..p as usize + 4].copy_from_slice(&v.to_le_bytes());
}
fn mem_store16(b: &mut [u8], p: u32, v: u16) {
    b[p as usize..p as usize + 2].copy_from_slice(&v.to_le_bytes());
}
pub trait PreprocessHooksM {
    /// `fn_176f94`: validate the processor.
    fn check_m(&mut self, proc: u32, tag: u32, mem: &mut [u8]) -> u32;
    /// Stream-table available call.
    fn available_m(
        &mut self,
        h0: u32,
        h1: u32,
        avail: &mut u32,
        ended: &mut u8,
        mem: &mut [u8],
    ) -> u32;
    /// Stream-table transfer call.
    fn transfer_m(
        &mut self,
        oh0: u32,
        oh1: u32,
        ih0: u32,
        ih1: u32,
        count: &mut u32,
        mem: &mut [u8],
    ) -> u32;
    /// Stream-table release call.
    fn release_m(&mut self, h0: u32, h1: u32, mem: &mut [u8]) -> u32;
    /// Stream-table read call.
    fn read_stream_m(
        &mut self,
        h0: u32,
        h1: u32,
        ptr: &mut u32,
        len: &mut u32,
        mem: &mut [u8],
    ) -> u32;
    /// `fn_196394`: allocate `size` bytes from `heap`; `None` on failure.
    fn alloc_text_m(&mut self, heap: u32, size: u32, mem: &mut [u8]) -> Option<u32>;
    /// `fn_1963b8`: allocate `size` bytes from `heap`; `None` on failure.
    fn alloc_events_m(&mut self, heap: u32, size: u32, mem: &mut [u8]) -> Option<u32>;
    /// `fn_173dc0`: allocation-failure trace.
    fn log_no_memory_m(&mut self, log: u32, mem: &mut [u8]);
    /// Position-table get call.
    fn get_position_m(&mut self, h0: u32, h1: u32, pos: &mut u32, mem: &mut [u8]) -> u32;
    /// Transform-table transform call.
    fn transform_m(
        &mut self,
        h0: u32,
        h1: u32,
        text: &mut u32,
        events: &mut u32,
        count: &mut u32,
        mem: &mut [u8],
    ) -> u32;
    /// Stream-table write call.
    fn write_stream_m(&mut self, h0: u32, h1: u32, data: u32, len: u32, mem: &mut [u8]) -> u32;
    /// Stream-table consume call.
    fn consume_m(&mut self, h0: u32, h1: u32, bytes: u32, mem: &mut [u8]) -> u32;
    /// Position-table set call.
    fn set_position_m(&mut self, h0: u32, h1: u32, pos: u32, mem: &mut [u8]) -> u32;
    /// `fn_196428`: free `ptr` back to `heap`.
    fn free_buf_m(&mut self, heap: u32, ptr: u32, mem: &mut [u8]);
}
pub fn preprocess_prompt_m<H: PreprocessHooksM>(
    hooks: &mut H,
    mem: &mut [u8],
    proc: u32,
    tag: u32,
    _engine0: u32,
    _engine1: u32,
    active: u32,
) -> u32 {
    let mut result = hooks.check_m(proc, tag, mem);
    if result >= 0x80000000 {
        return 0x8c802008;
    }
    mem[active as usize] = 1;
    let heap = mem_word(mem, mem_word(mem, proc).wrapping_add(4));
    let log = mem_word(mem, mem_word(mem, proc).wrapping_add(16));
    let (ih00, ih01) = (
        mem_word(mem, proc.wrapping_add(32)),
        mem_word(mem, proc.wrapping_add(36)),
    );
    let (ih10, ih11) = (
        mem_word(mem, proc.wrapping_add(40)),
        mem_word(mem, proc.wrapping_add(44)),
    );
    let mut available_text = 0u32;
    let mut ended = 0u8;
    result = hooks.available_m(ih00, ih01, &mut available_text, &mut ended, mem);
    if result >= 0x80000000 {
        return result;
    }
    let mut available_events = 0u32;
    result = hooks.available_m(ih10, ih11, &mut available_events, &mut ended, mem);
    if result >= 0x80000000 {
        return result;
    }
    if available_text == 0 {
        if available_events | ended as u32 == 0 {
            mem[active as usize] = 0;
        } else {
            if available_events != 0 {
                let mut event_count = 0u32;
                result = hooks.transfer_m(
                    mem_word(mem, proc.wrapping_add(56)),
                    mem_word(mem, proc.wrapping_add(60)),
                    ih10,
                    ih11,
                    &mut event_count,
                    mem,
                );
                if result >= 0x80000000 {
                    return result;
                }
            }
            if ended != 0 {
                for o in [48u32, 56u32] {
                    if mem_word(mem, proc.wrapping_add(o)) != 0 {
                        result = hooks.release_m(
                            mem_word(mem, proc.wrapping_add(o)),
                            mem_word(mem, proc.wrapping_add(o + 4)),
                            mem,
                        );
                        mem_store(mem, proc.wrapping_add(o), 0);
                        mem_store(mem, proc.wrapping_add(o + 4), 0);
                    }
                }
            }
        }
        return result;
    }
    let mut input_text = 0u32;
    let mut text_length = 0u32;
    result = hooks.read_stream_m(ih00, ih01, &mut input_text, &mut text_length, mem);
    if result >= 0x80000000 {
        return result;
    }
    text_length >>= 1;
    let bytes = text_length.wrapping_mul(2);
    let Some(text) = hooks.alloc_text_m(heap, bytes.wrapping_add(2), mem) else {
        hooks.log_no_memory_m(log, mem);
        return 0x8c80200a;
    };
    mem.copy_within(
        input_text as usize..(input_text.wrapping_add(bytes)) as usize,
        text as usize,
    );
    mem_store16(mem, text.wrapping_add(bytes), 0);
    let mut input_events = 0u32;
    let mut original_events = 0u32;
    result = hooks.read_stream_m(ih10, ih11, &mut input_events, &mut original_events, mem);
    if result >= 0x80000000 {
        hooks.free_buf_m(heap, text, mem);
        return result;
    }
    original_events /= 24;
    let event_bytes = original_events.wrapping_mul(24);
    let Some(events) = hooks.alloc_events_m(heap, event_bytes, mem) else {
        hooks.log_no_memory_m(log, mem);
        hooks.free_buf_m(heap, text, mem);
        return 0x8c80200a;
    };
    mem.copy_within(
        input_events as usize..(input_events.wrapping_add(event_bytes)) as usize,
        events as usize,
    );
    let (ph0, ph1) = (
        mem_word(mem, proc.wrapping_add(20)),
        mem_word(mem, proc.wrapping_add(24)),
    );
    let mut position = 0u32;
    result = hooks.get_position_m(ph0, ph1, &mut position, mem);
    if result >= 0x80000000 {
        hooks.free_buf_m(heap, text, mem);
        hooks.free_buf_m(heap, events, mem);
        return result;
    }
    let (th0, th1) = (
        mem_word(mem, proc.wrapping_add(12)),
        mem_word(mem, proc.wrapping_add(16)),
    );
    let mut ttext = text;
    let mut tevents = events;
    let mut event_count = original_events;
    result = hooks.transform_m(th0, th1, &mut ttext, &mut tevents, &mut event_count, mem);
    if result >= 0x80000000 {
        hooks.free_buf_m(heap, text, mem);
        hooks.free_buf_m(heap, events, mem);
        return result;
    }
    let growth = u16_len(mem, ttext).wrapping_sub(text_length);
    let evpos = mem_word(mem, tevents.wrapping_add(16));
    mem_store(
        mem,
        tevents.wrapping_add(16),
        evpos.wrapping_add(growth.wrapping_mul(2)),
    );
    if u16_len(mem, ttext) != 0 {
        result = hooks.write_stream_m(
            mem_word(mem, proc.wrapping_add(48)),
            mem_word(mem, proc.wrapping_add(52)),
            ttext,
            u16_len(mem, ttext).wrapping_mul(2),
            mem,
        );
        if result >= 0x80000000 {
            hooks.free_buf_m(heap, text, mem);
            hooks.free_buf_m(heap, events, mem);
            return result;
        }
    }
    result = hooks.consume_m(ih00, ih01, bytes, mem);
    if result >= 0x80000000 {
        hooks.free_buf_m(heap, text, mem);
        hooks.free_buf_m(heap, events, mem);
        return result;
    }
    result = hooks.write_stream_m(
        mem_word(mem, proc.wrapping_add(56)),
        mem_word(mem, proc.wrapping_add(60)),
        tevents,
        event_count.wrapping_mul(24),
        mem,
    );
    if result >= 0x80000000 {
        hooks.free_buf_m(heap, text, mem);
        hooks.free_buf_m(heap, events, mem);
        return result;
    }
    result = hooks.consume_m(ih10, ih11, event_bytes, mem);
    if result >= 0x80000000 {
        hooks.free_buf_m(heap, text, mem);
        hooks.free_buf_m(heap, events, mem);
        return result;
    }
    position = position.wrapping_add(growth.wrapping_mul(2));
    result = hooks.set_position_m(ph0, ph1, position, mem);
    hooks.free_buf_m(heap, text, mem);
    hooks.free_buf_m(heap, events, mem);
    result
}

/// Prompt root adapter. scratch is 64 disjoint, restored caller-owned bytes.
pub struct PreprocessAdapter<'a, H> {
    pub host: &'a mut H,
    pub processor: u32,
    pub scratch: u32,
}
impl<H: Host> PreprocessAdapter<'_, H> {
    fn invoke(&mut self, mem: &mut [u8], f: u32, a: &[u32]) -> u32 {
        self.host.call_m(
            f,
            a,
            &mut Memory {
                base: 0,
                bytes: mem,
            },
        )
    }
    fn method(&self, mem: &[u8], field: u32, slot: u32) -> u32 {
        mem_word(mem, mem_word(mem, self.processor + field) + slot)
    }
    fn out(&mut self, mem: &mut [u8], f: u32, a: &[u32], initial: [u32; 3]) -> (u32, [u32; 3]) {
        let at = self.scratch as usize;
        let saved: [u8; 64] = mem[at..at + 64].try_into().unwrap();
        for (i, v) in initial.iter().enumerate() {
            mem_store(mem, self.scratch + i as u32 * 4, *v);
        }
        let rc = self.invoke(mem, f, a);
        let values = core::array::from_fn(|i| mem_word(mem, self.scratch + i as u32 * 4));
        mem[at..at + 64].copy_from_slice(&saved);
        (rc, values)
    }
}
impl<H: Host> PreprocessHooksM for PreprocessAdapter<'_, H> {
    fn check_m(&mut self, p: u32, key: u32, _: &mut [u8]) -> u32 {
        frontend::validate_handle(p != 0, key, 0xf424, 0x40)
    }
    fn available_m(&mut self, a: u32, b: u32, n: &mut u32, ended: &mut u8, mem: &mut [u8]) -> u32 {
        let (rc, v) = self.out(
            mem,
            self.method(mem, 4, 0x48),
            &[a, b, self.scratch, self.scratch + 4],
            [0; 3],
        );
        *n = v[0];
        *ended = v[1] as u8;
        rc
    }
    fn transfer_m(&mut self, a: u32, b: u32, c: u32, d: u32, n: &mut u32, mem: &mut [u8]) -> u32 {
        let (rc, v) = self.out(
            mem,
            self.method(mem, 4, 0x40),
            &[a, b, c, d, 0, self.scratch],
            [*n, 0, 0],
        );
        *n = v[0];
        rc
    }
    fn release_m(&mut self, a: u32, b: u32, mem: &mut [u8]) -> u32 {
        self.invoke(mem, self.method(mem, 4, 0x24), &[a, b])
    }
    fn read_stream_m(&mut self, a: u32, b: u32, p: &mut u32, n: &mut u32, mem: &mut [u8]) -> u32 {
        let (rc, v) = self.out(
            mem,
            self.method(mem, 4, 0x2c),
            &[a, b, self.scratch, self.scratch + 4],
            [*p, *n, 0],
        );
        *p = v[0];
        *n = v[1];
        rc
    }
    fn alloc_text_m(&mut self, heap: u32, size: u32, mem: &mut [u8]) -> Option<u32> {
        let p = self.invoke(mem, 0x196394, &[heap, size]);
        (p != 0).then_some(p)
    }
    fn alloc_events_m(&mut self, heap: u32, size: u32, mem: &mut [u8]) -> Option<u32> {
        let p = self.invoke(mem, 0x1963b8, &[heap, 1, size]);
        (p != 0).then_some(p)
    }
    fn log_no_memory_m(&mut self, log: u32, mem: &mut [u8]) {
        
        self.invoke(mem, 0x173dc0, &[log, addr(self.host.rom(), 0x8f9424), 0xdea8, 0]);
    }
    fn get_position_m(&mut self, a: u32, b: u32, pos: &mut u32, mem: &mut [u8]) -> u32 {
        let (rc, v) = self.out(
            mem,
            self.method(mem, 28, 0x3c),
            &[a, b, self.scratch, 0],
            [*pos, 0, 0],
        );
        *pos = v[0];
        rc
    }
    fn transform_m(
        &mut self,
        a: u32,
        b: u32,
        text: &mut u32,
        events: &mut u32,
        count: &mut u32,
        mem: &mut [u8],
    ) -> u32 {
        let (rc, v) = self.out(
            mem,
            self.method(mem, 8, 0x38),
            &[a, b, self.scratch, self.scratch + 4, self.scratch + 8],
            [*text, *events, *count],
        );
        *text = v[0];
        *events = v[1];
        *count = v[2];
        rc
    }
    fn write_stream_m(&mut self, a: u32, b: u32, data: u32, len: u32, mem: &mut [u8]) -> u32 {
        self.invoke(mem, self.method(mem, 4, 0x34), &[a, b, data, len])
    }
    fn consume_m(&mut self, a: u32, b: u32, n: u32, mem: &mut [u8]) -> u32 {
        self.invoke(mem, self.method(mem, 4, 0x30), &[a, b, n])
    }
    fn set_position_m(&mut self, a: u32, b: u32, pos: u32, mem: &mut [u8]) -> u32 {
        self.invoke(mem, self.method(mem, 28, 0x38), &[a, b, pos, 0])
    }
    fn free_buf_m(&mut self, heap: u32, p: u32, mem: &mut [u8]) {
        self.invoke(mem, 0x196428, &[heap, p]);
    }
}
/// fn_1f48cc. Same body as frontend::preprocess_prompt; callbacks take the arena.
pub fn fn_1f48cc_m(m: &mut Memory<'_>, args: [u32; 5], scratch: u32, h: &mut impl Host) -> u32 {
    assert_eq!(m.base, 0);
    let mut hooks = PreprocessAdapter {
        host: h,
        processor: args[0],
        scratch,
    };
    preprocess_prompt_m(
        &mut hooks, m.bytes, args[0], args[1], args[2], args[3], args[4],
    )
}

pub trait VoiceNodeAllocatorM: frontend::HeapVtableM {
    fn alloc_items_at_m(&mut self, ctx: u32, count: u32, width: u32, mem: &mut [u8]) -> u32;
}
pub trait DomainApiM {
    fn resolve_m(&mut self, first: u32, second: u32, context: &mut u32, mem: &mut [u8]) -> u32;
    fn find_m(&mut self, map: u32, name: u32, item: &mut u32, mem: &mut [u8]) -> u32;
    fn append_m(&mut self, list: u32, item: u32, mem: &mut [u8]) -> u32;
    fn pop_m(&mut self, list: u32, mem: &mut [u8]);
    fn remove_m(&mut self, list: u32, index: u32, mem: &mut [u8]);
    fn duplicate_m(&mut self, heap: u32, text: u32, copy: &mut u32, mem: &mut [u8]) -> u32;
    fn free_m(&mut self, heap: u32, text: u32, mem: &mut [u8]);
    fn slot_m(&mut self, list: u32, index: u32, item: &mut u32, mem: &mut [u8]) -> u32;
    fn build_m(&mut self, mem: &mut [u8], database: u32, args: u32, output: u32) -> u32;
    /// Diagnostic-only fn_1738e8 and fn_173dc0 boundaries.
    fn trace_m(&mut self, log: u32, code: u32, name: u32, status: u32, mem: &mut [u8]);
}
pub trait DomainConsultationM: DomainApiM + VoiceNodeAllocatorM {
    fn allocate_name_m(&mut self, heap: u32, size: u32, mem: &mut [u8]) -> u32;
    fn map_iter_m(&mut self, map: u32, iterator: &mut u32, mem: &mut [u8]) -> u32;
    fn map_next_m(&mut self, iterator: u32, key: &mut u32, value: &mut u32, mem: &mut [u8]) -> u32;
    fn map_close_m(&mut self, iterator: u32, mem: &mut [u8]);
    fn expand_original_m(
        &mut self,
        mem: &mut [u8],
        database: u32,
        text: u32,
        entries: u32,
        count: u32,
        head: u32,
    ) -> u32;
    /// The original optional begin/end text and marker diagnostics.
    fn consultation_log_m(
        &mut self,
        mem: &mut [u8],
        log: u32,
        text: u32,
        entries: u32,
        count: u32,
        status: Option<u32>,
    );
}
pub fn duplicate_voice_name_m(
    mem: &mut [u8],
    ctx: u32,
    source: u32,
    out: u32,
    heap: &mut impl frontend::HeapVtableM,
) -> u32 {
    let mut len = 0u32;
    while mem[source.wrapping_add(len) as usize] != 0 {
        len = len.wrapping_add(1);
    }
    let size = len.wrapping_add(1);
    let dst = heap.alloc_at_m(ctx, size, mem);
    mem_store(mem, out, dst);
    if dst == 0 {
        return 0x88e0200a;
    }
    // strcpy reads the source after the allocation result is stored.
    let mut i = 0u32;
    loop {
        let byte = mem[source.wrapping_add(i) as usize];
        mem[dst.wrapping_add(i) as usize] = byte;
        if byte == 0 {
            break;
        }
        i = i.wrapping_add(1);
    }
    0
}
pub fn update_voice_node_m(
    mem: &mut [u8],
    ctx: u32,
    node: u32,
    kind: u32,
    value: u32,
    amount: u32,
    name: Option<u32>,
    count: u32,
    items: Option<u32>,
    heap: &mut impl VoiceNodeAllocatorM,
) -> u32 {
    if (kind == 1 && (name.is_some() || items.is_some())) || name.is_some() != items.is_some() {
        return 0x88e02000;
    }
    let old_name = mem_word(mem, node.wrapping_add(12));
    if old_name != 0 {
        heap.release_at_m(ctx, old_name, mem);
        mem_store(mem, node.wrapping_add(12), 0);
    }
    let old_items = mem_word(mem, node.wrapping_add(20));
    if old_items != 0 {
        heap.release_at_m(ctx, old_items, mem);
        mem_store(mem, node.wrapping_add(20), 0);
        mem_store16(mem, node.wrapping_add(16), 0);
    }
    mem[node as usize] = kind as u8;
    mem_store(mem, node.wrapping_add(4), value);
    mem_store(mem, node.wrapping_add(8), amount);
    if let (Some(name), Some(items)) = (name, items) {
        duplicate_voice_name_m(mem, ctx, name, node.wrapping_add(12), heap);
        let block = heap.alloc_items_at_m(ctx, count, 4, mem);
        mem_store(mem, node.wrapping_add(20), block);
        if block != 0 {
            move_bytes(mem, block, items, count.wrapping_mul(4));
            mem_store16(mem, node.wrapping_add(16), count as u16);
        }
        if mem_word(mem, node.wrapping_add(12)) == 0 || block == 0 {
            return 0x88e0200a;
        }
    }
    0
}
pub fn create_voice_node_m(
    mem: &mut [u8],
    ctx: u32,
    kind: u32,
    value: u32,
    value2: u32,
    name: Option<u32>,
    count: u32,
    items: Option<u32>,
    heap: &mut impl VoiceNodeAllocatorM,
    dup: &mut impl FnMut(u32, u32, &mut u32, &mut [u8]),
) -> u32 {
    if kind == 1 && (name.is_some() || items.is_some()) {
        return 0;
    }
    match (name, items) {
        (None, Some(_)) | (Some(_), None) => return 0,
        _ => {}
    }
    let node = heap.alloc_items_at_m(ctx, 1, 28, mem);
    if node == 0 {
        return 0;
    }
    mem_store(mem, node.wrapping_add(12), 0);
    mem_store16(mem, node.wrapping_add(16), 0);
    mem_store(mem, node.wrapping_add(20), 0);
    mem[node as usize] = kind as u8;
    mem_store(mem, node.wrapping_add(4), value);
    mem_store(mem, node.wrapping_add(8), value2);
    if let (Some(n), Some(it)) = (name, items) {
        let mut dst = 0u32;
        dup(ctx, n, &mut dst, mem);
        mem_store(mem, node.wrapping_add(12), dst);
        let blk = heap.alloc_items_at_m(ctx, count, 4, mem);
        mem_store(mem, node.wrapping_add(20), blk);
        if blk != 0 {
            move_bytes(mem, blk, it, count.wrapping_mul(4));
            mem_store16(mem, node.wrapping_add(16), count as u16);
        }
        if dst == 0 || blk == 0 {
            let mut cur = node;
            loop {
                let next = mem_word(mem, cur.wrapping_add(24));
                let nm = mem_word(mem, cur.wrapping_add(12));
                if nm != 0 {
                    heap.release_at_m(ctx, nm, mem);
                }
                let itw = mem_word(mem, cur.wrapping_add(20));
                if itw != 0 {
                    heap.release_at_m(ctx, itw, mem);
                }
                heap.release_at_m(ctx, cur, mem);
                if next == 0 {
                    break;
                }
                cur = next;
            }
            return 0;
        }
    }
    node
}
pub fn release_node_chain_m(
    mem: &mut [u8],
    n: Option<u32>,
    owner: Option<u32>,
    vt: &mut impl frontend::HeapVtableM,
) {
    let mut cur = n;
    while let Some(node) = cur {
        let fc = mem_word(mem, node.wrapping_add(12));
        let f14 = mem_word(mem, node.wrapping_add(20));
        cur = nz(mem_word(mem, node.wrapping_add(24)));
        if fc != 0 {
            frontend::heap_release_m(mem, owner, nz(fc), vt);
        }
        if f14 != 0 {
            frontend::heap_release_m(mem, owner, nz(f14), vt);
        }
        frontend::heap_release_m(mem, owner, Some(node), vt);
    }
}
fn domain_context_m(
    mem: &mut [u8],
    handle: Option<u32>,
    tag: u32,
    api: &mut impl DomainApiM,
) -> Result<(u32, u32), u32> {
    let rc = frontend::validate_handle(handle.is_some(), tag, 0x11d, 0x48);
    if rc >= 0x80000000 {
        return Err(rc);
    }
    let handle = handle.unwrap();
    let mut context = 0;
    let rc = api.resolve_m(
        mem_word(mem, handle),
        mem_word(mem, handle.wrapping_add(4)),
        &mut context,
        mem,
    );
    if rc >= 0x80000000 {
        Err(rc)
    } else {
        Ok((context, rc))
    }
}
pub fn enable_domain_m(
    mem: &mut [u8],
    handle: Option<u32>,
    tag: u32,
    name: u32,
    api: &mut impl DomainApiM,
) -> u32 {
    let (context, _) = match domain_context_m(mem, handle, tag, api) {
        Ok(v) => v,
        Err(rc) => return rc,
    };
    let handle = handle.unwrap();
    let log = mem_word(mem, context.wrapping_add(16));
    let mut item = 0;
    let mut rc = api.find_m(mem_word(mem, handle.wrapping_add(60)), name, &mut item, mem);
    if rc < 0x80000000 {
        if api.append_m(mem_word(mem, handle.wrapping_add(64)), item, mem) != 0 {
            api.trace_m(log, 0, name, rc, mem);
            return rc;
        }
        api.trace_m(log, 0xb798, 0, 0, mem);
        rc = 0x88e0200a;
    }
    api.trace_m(log, 0xb7a0, name, rc, mem);
    rc
}
fn consult_active_domains_m(
    mem: &mut [u8],
    manager: u32,
    text: u32,
    entries: u32,
    count: u32,
    head: u32,
    api: &mut impl DomainConsultationM,
) -> u32 {
    let ctx = mem_word(mem, manager + 8);
    let log = mem_word(mem, ctx + 16);
    api.consultation_log_m(mem, log, text, entries, count, None);
    let mut iterator = 0;
    let mut key = 0;
    let mut value = 0;
    if api.map_iter_m(mem_word(mem, manager + 60), &mut iterator, mem) < 0x80000000 {
        while api.map_next_m(iterator, &mut key, &mut value, mem) < 0x80000000 {
            if value != 0 {
                mem[(value + 12) as usize] = 0;
            }
        }
        api.map_close_m(iterator, mem);
    }
    let list = mem_word(mem, manager + 64);
    let mut i = if list == 0 {
        0
    } else {
        mem_word(mem, list + 20)
    };
    let mut domain = 0;
    let mut database = 0;
    let mut rc = 0;
    'domains: while i != 0 && api.slot_m(list, i - 1, &mut domain, mem) != 0 {
        let d = mem_word(mem, domain);
        if d == 0 || mem[(d + 12) as usize] == 0 {
            let databases = mem_word(mem, d + 8);
            let mut j = if databases == 0 {
                0
            } else {
                mem_word(mem, databases + 20)
            };
            while j != 0 && api.slot_m(databases, j - 1, &mut database, mem) != 0 {
                rc =
                    api.expand_original_m(mem, mem_word(mem, database), text, entries, count, head);
                if rc >= 0x80000000 {
                    break 'domains;
                }
                j -= 1;
            }
            if d != 0 {
                mem[(d + 12) as usize] = 1;
            }
        }
        i -= 1;
    }
    api.consultation_log_m(mem, log, text, entries, count, Some(rc));
    rc
}
pub fn consult_original_domains_m(
    mem: &mut [u8],
    handle: Option<u32>,
    tag: u32,
    text: u32,
    entries: u32,
    count: u32,
    api: &mut impl DomainConsultationM,
) -> u32 {
    let (ctx, mut rc) = match domain_context_m(mem, handle, tag, api) {
        Ok(v) => v,
        Err(rc) => return rc,
    };
    let manager = handle.unwrap();
    let heap = mem_word(mem, ctx + 4);
    let log = mem_word(mem, ctx + 16);
    api.trace_m(log, 0, 0, 0, mem);
    mem[(manager + 68) as usize] = 1;
    let entry = mem_word(mem, entries);
    let head = create_voice_node_m(
        mem,
        heap,
        1,
        mem_word(mem, entry + 12),
        mem_word(mem, entry + 16),
        None,
        0,
        None,
        api,
        &mut |_, _, _, _| unreachable!(),
    );
    if head == 0 {
        api.trace_m(log, 0xb798, 0, 0, mem);
        return 0x88e0200a;
    }
    let mut node = head;
    let mut first = None;
    let mut second = None;
    let mut i = 0u16;
    while i < mem_half(mem, count) {
        let e = mem_word(mem, entries).wrapping_add((i as u32) * 24);
        let kind = mem_half(mem, e);
        let value = mem_word(mem, e + 20);
        let position = mem_word(mem, e + 12);
        i = i.wrapping_add(1);
        if kind == 12 {
            if value != 0 && mem_half(mem, value) != 0 && !domain_wide_equal(mem, Some(value), None)
            {
                let name = api.allocate_name_m(heap, u16_len(mem, value).wrapping_add(1), mem);
                if name == 0 {
                    api.trace_m(log, 0xb798, 0, 0, mem);
                    rc = 0x88e0200a;
                    break;
                }
                recode_176460(mem, name, value);
                enable_domain_m(mem, handle, tag, name, api);
                api.free_m(heap, name, mem);
            } else {
                let list = mem_word(mem, manager + 64);
                let n = if list == 0 {
                    0
                } else {
                    mem_word(mem, list + 20)
                };
                if n > mem_word(mem, manager + 52) {
                    api.pop_m(list, mem);
                    // The C logs this successful pop before reading the next marker.
                    api.trace_m(log, 0, 0, 1, mem);
                } else {
                    api.trace_m(log, 0xb7a5, 0, 0, mem);
                }
            }
            continue;
        }
        let boundary = matches!(kind, 29 | 8 | 40 | 26);
        let style = (kind == 21 && !domain_wide_equal(mem, first, Some(value))) || kind == 36;
        if !boundary && !style {
            continue;
        }
        let mut new_kind = mem[node as usize] as u32;
        if !boundary {
            new_kind = if domain_wide_equal(mem, Some(value), None)
                && ((kind == 21 && domain_wide_equal(mem, second, None))
                    || (kind == 36 && domain_wide_equal(mem, first, None)))
            {
                1
            } else {
                0
            };
            if kind == 21 {
                first = Some(value);
            } else {
                second = Some(value);
            }
        }
        let start = mem_word(mem, node + 4);
        let length = mem_word(mem, node + 8);
        if position > start && position.wrapping_sub(start) != length {
            let amount = position.wrapping_sub(start);
            rc = update_voice_node_m(
                mem,
                heap,
                node,
                mem[node as usize] as u32,
                start,
                amount,
                None,
                0,
                None,
                api,
            );
            if rc >= 0x80000000 {
                break;
            }
            let rest = length.wrapping_sub(mem_word(mem, node + 8));
            let next = create_voice_node_m(
                mem,
                heap,
                new_kind,
                position,
                rest,
                None,
                0,
                None,
                api,
                &mut |_, _, _, _| unreachable!(),
            );
            if next == 0 {
                api.trace_m(log, 0xb798, 0, 0, mem);
                rc = 0x88e0200a;
                break;
            }
            mem_store(mem, node + 24, next);
            node = next;
        } else if !boundary && position == start {
            rc = update_voice_node_m(mem, heap, node, new_kind, start, length, None, 0, None, api);
            if rc >= 0x80000000 {
                break;
            }
        }
    }
    if i >= mem_half(mem, count) && rc < 0x80000000 && u16_len(mem, mem_word(mem, text)) != 0 {
        rc = consult_active_domains_m(mem, manager, text, entries, count, head, api);
    }
    release_node_chain_m(mem, Some(head), nz(heap), api);
    api.trace_m(log, 0, 0, rc, mem);
    rc
}
fn domain_wide_equal(mem: &[u8], a: Option<u32>, b: Option<u32>) -> bool {
    let normal = [110u16, 111, 114, 109, 97, 108, 0];
    let mut i = 0usize;
    loop {
        let x = a.map_or_else(
            || normal[i],
            |p| mem_half(mem, p.wrapping_add((i * 2) as u32)),
        );
        let y = b.map_or_else(
            || normal[i],
            |p| mem_half(mem, p.wrapping_add((i * 2) as u32)),
        );
        if x != y {
            return false;
        }
        if x == 0 {
            return true;
        }
        i += 1;
    }
}
fn recode_176460(mem: &mut [u8], dst: u32, src: u32) {
    let mut s = src;
    let mut d = dst;
    loop {
        let unit = mem_half(mem, s);
        if unit == 0 {
            break;
        }
        mem[d as usize] = if unit & 0xff00 != 0 { b'?' } else { unit as u8 };
        s = s.wrapping_add(2);
        d = d.wrapping_add(1);
    }
    mem[d as usize] = 0;
}
fn mem_half(b: &[u8], p: u32) -> u16 {
    u16::from_le_bytes(b[p as usize..p as usize + 2].try_into().unwrap())
}
fn nz(p: u32) -> Option<u32> {
    (p != 0).then_some(p)
}
use crate::frontend::move_bytes;

/// Live domain hooks. A separate, restored scratch range holds guest ABI outs.
pub struct DomainAdapter<'a, H> {
    pub io: PreprocessAdapter<'a, H>,
    pub trace_count: u32,
    pub heap: u32,
}
impl<H: Host> frontend::HeapVtableM for DomainAdapter<'_, H> {
    fn alloc_at_m(&mut self, heap: u32, size: u32, mem: &mut [u8]) -> u32 {
        self.io.invoke(mem, 0x196394, &[heap, size])
    }
    fn realloc_at_m(&mut self, heap: u32, p: u32, size: u32, mem: &mut [u8]) -> u32 {
        self.io.invoke(mem, 0x1963e0, &[heap, p, size])
    }
    fn release_at_m(&mut self, heap: u32, p: u32, mem: &mut [u8]) {
        if self.heap != 0 && heap == mem_word(mem, self.heap + 4) {
            self.io.invoke(
                mem,
                mem_word(mem, mem_word(mem, self.heap) + 12),
                &[heap, p],
            );
        } else {
            self.io.invoke(mem, 0x196428, &[heap, p]);
        }
    }
}
impl<H: Host> VoiceNodeAllocatorM for DomainAdapter<'_, H> {
    fn alloc_items_at_m(&mut self, heap: u32, n: u32, width: u32, mem: &mut [u8]) -> u32 {
        self.io.invoke(mem, 0x1963b8, &[heap, n, width])
    }
}
fn literal(mem: &[u8], text: &[u8]) -> u32 {
    let end = mem.len().min(0x500000);
    let range = &mem[0x100000..end];
    (0x100000
        + range
            .windows(text.len() + 1)
            .position(|w| &w[..text.len()] == text && w[text.len()] == 0)
            .expect("mapped source literal")) as u32
}
impl<H: Host> DomainApiM for DomainAdapter<'_, H> {
    fn resolve_m(&mut self, a: u32, b: u32, out: &mut u32, mem: &mut [u8]) -> u32 {
        let (rc, v) = self.io.out(mem, 0x19611c, &[a, b, self.io.scratch], [0; 3]);
        *out = v[0];
        if rc < 0x80000000 {
            self.heap = mem_word(mem, *out + 4);
        }
        rc
    }
    fn find_m(&mut self, map: u32, key: u32, out: &mut u32, mem: &mut [u8]) -> u32 {
        let (rc, v) = self
            .io
            .out(mem, 0x1966ac, &[map, key, self.io.scratch], [0; 3]);
        *out = v[0];
        rc
    }
    fn append_m(&mut self, list: u32, item: u32, mem: &mut [u8]) -> u32 {
        let (rc, v) = self.io.out(mem, 0x196db0, &[list, self.io.scratch], [0; 3]);
        if rc != 0 {
            mem_store(mem, v[0], item);
        }
        rc
    }
    fn pop_m(&mut self, list: u32, mem: &mut [u8]) {
        self.io.invoke(mem, 0x1970dc, &[list]);
    }
    fn remove_m(&mut self, list: u32, index: u32, mem: &mut [u8]) {
        self.io.invoke(mem, 0x196e28, &[list, index]);
    }
    fn duplicate_m(&mut self, heap: u32, text: u32, out: &mut u32, mem: &mut [u8]) -> u32 {
        let (rc, v) = self
            .io
            .out(mem, 0x281618, &[heap, text, self.io.scratch], [0; 3]);
        *out = v[0];
        rc
    }
    fn free_m(&mut self, heap: u32, p: u32, mem: &mut [u8]) {
        self.io.invoke(mem, 0x196428, &[heap, p]);
    }
    fn slot_m(&mut self, list: u32, index: u32, out: &mut u32, mem: &mut [u8]) -> u32 {
        let (rc, v) = self
            .io
            .out(mem, 0x1970ac, &[list, index, self.io.scratch], [0; 3]);
        *out = v[0];
        rc
    }
    fn build_m(&mut self, _mem: &mut [u8], _: u32, _: u32, _: u32) -> u32 {
        panic!("DomainApi::build is not used by consult_original_domains")
    }
    fn trace_m(&mut self, log: u32, code: u32, name: u32, status: u32, mem: &mut [u8]) {
        // The trace adapter only sees the raw arena bytes; live arenas are
        // flat with base 0, anything else falls back to the US literal.
        
        if code == 0xb7a0 {
            let fmt = literal(mem, b"%s%s%s%x");
            let domain = literal(mem, b"domain");
            let error = literal(mem, b"lhError");
            self.io.invoke(
                mem,
                0x173dc0,
                &[log, addr(self.io.host.rom(), 0x8f95d4), code, fmt, domain, name, error, status],
            );
        } else if code != 0 {
            self.io.invoke(mem, 0x173dc0, &[log, addr(self.io.host.rom(), 0x8f95d4), code, 0]);
        } else if name != 0 {
            let fmt = literal(mem, b"    Enable domain [%s]");
            self.io
                .invoke(mem, 0x1738e8, &[log, addr(self.io.host.rom(), 0x8f95d4), 4, 0, fmt, name]);
        } else if status == 1 {
            // This event does not advance the enclosing entry/exit trace pair.
            let fmt = literal(mem, b"    Disable last enabled domain");
            self.io.invoke(mem, 0x1738e8, &[log, addr(self.io.host.rom(), 0x8f95d4), 4, 0, fmt]);
        } else {
            let fmt = literal(
                mem,
                if self.trace_count == 0 {
                    b"Entering domain_mngr_ConsultOriOrth"
                } else {
                    b"Exiting domain_mngr_ConsultOriOrth"
                },
            );
            self.trace_count += 1;
            self.io.invoke(mem, 0x1738e8, &[log, addr(self.io.host.rom(), 0x8f95d4), 4, 0, fmt]);
        }
    }
}
impl<H: Host> DomainConsultationM for DomainAdapter<'_, H> {
    fn allocate_name_m(&mut self, heap: u32, n: u32, mem: &mut [u8]) -> u32 {
        self.io.invoke(mem, 0x196394, &[heap, n])
    }
    fn map_iter_m(&mut self, map: u32, out: &mut u32, mem: &mut [u8]) -> u32 {
        let (rc, v) = self
            .io
            .out(mem, 0x196794, &[map, 0, 0, self.io.scratch], [0; 3]);
        *out = v[0];
        rc
    }
    fn map_next_m(&mut self, it: u32, key: &mut u32, value: &mut u32, mem: &mut [u8]) -> u32 {
        let (rc, v) = self.io.out(
            mem,
            0x196860,
            &[it, self.io.scratch, self.io.scratch + 4],
            [0; 3],
        );
        *key = v[0];
        *value = v[1];
        rc
    }
    fn map_close_m(&mut self, it: u32, mem: &mut [u8]) {
        self.io.invoke(mem, 0x196830, &[it]);
    }
    fn expand_original_m(
        &mut self,
        mem: &mut [u8],
        db: u32,
        text: u32,
        entries: u32,
        count: u32,
        head: u32,
    ) -> u32 {
        self.io
            .invoke(mem, 0x27f7cc, &[db, text, entries, count, head])
    }
    fn consultation_log_m(
        &mut self,
        mem: &mut [u8],
        log: u32,
        text: u32,
        entries: u32,
        count: u32,
        status: Option<u32>,
    ) {
        if self.io.invoke(mem, 0x17497c, &[log]) != 0 {
            // The logger retains the live text and marker addresses, not copied replies.
            let fmt = literal(mem, b"%s");
            let label = literal(mem, if status.is_some() { b"END" } else { b"BEG" });
            self.io.invoke(mem, 0x17486c, &[log, 0xd2, fmt, label, 0]);
            let label = literal(mem, b"%s");
            let mime = literal(mem, b"text/plain;charset=utf-16");
            self.io.invoke(
                mem,
                0x1748a0,
                &[
                    log,
                    0xd3,
                    label,
                    mime,
                    mem_word(mem, text),
                    u16_len(mem, mem_word(mem, text)) * 2,
                ],
            );
            let mime = literal(mem, b"application/x-realspeak-markers-pp;version=4.0");
            self.io.invoke(
                mem,
                0x1748a0,
                &[
                    log,
                    0xd3,
                    label,
                    mime,
                    mem_word(mem, entries),
                    mem_half(mem, count) as u32 * 24,
                ],
            );
        }
    }
}
pub fn fn_1fe600_m(m: &mut Memory<'_>, a: [u32; 5], scratch: u32, h: &mut impl Host) -> u32 {
    assert_eq!(m.base, 0);
    let mut api = DomainAdapter {
        io: PreprocessAdapter {
            host: h,
            processor: 0,
            scratch,
        },
        trace_count: 0,
        heap: 0,
    };
    consult_original_domains_m(m.bytes, nz(a[0]), a[1], a[2], a[3], a[4], &mut api)
}

/// Method-table callback with a live arena, used by fn_196794.
pub trait MethodTableM {
    fn call_at_m(&mut self, obj: u32, n: u32, size: u32, mem: &mut [u8]) -> u32;
}
/// frontend::alloc_handle_for_key with live allocator/comparator callbacks.
pub fn alloc_handle_for_key_m(
    mem: &mut [u8],
    p: Option<u32>,
    key: u32,
    ordered: u32,
    out: Option<&mut u32>,
    vt: &mut impl MethodTableM,
    mut cmp: impl FnMut(u32, u32, &mut [u8]) -> i32,
) -> u32 {
    let (Some(p), Some(out)) = (p, out) else {
        return 0x89002007;
    };
    let v = if key != 0 {
        let tree = p + 24;
        if ordered != 0 {
            let head = mem_word(mem, tree);
            let flag = mem[(tree + 40) as usize];
            crate::reentrant2::find_node_m(mem, tree, head, flag, key, &mut cmp).unwrap_or(0)
        } else {
            search_node_m(mem, tree, key, &mut cmp)
        }
    } else {
        frontend::last_link(mem, p + 24)
    };
    if key != 0 && v == 0 {
        return 0x89002014;
    }
    let heap = mem_word(mem, p + 20);
    let o = if heap == 0 {
        0
    } else {
        vt.call_at_m(mem_word(mem, heap + 4), 1, 12, mem)
    };
    *out = o;
    if o == 0 {
        return 0x8900200a;
    }
    mem_store(mem, o, p);
    mem_store(mem, o + 4, 0);
    mem_store(mem, o + 8, v);
    0
}
fn search_node_m(
    mem: &mut [u8],
    tree: u32,
    key: u32,
    cmp: &mut impl FnMut(u32, u32, &mut [u8]) -> i32,
) -> u32 {
    let mut cur = mem_word(mem, tree);
    let mut prev = 0;
    let flag = mem[(tree + 40) as usize];
    while cur != tree {
        let rc = cmp(key, mem_word(mem, cur + 16), mem);
        if rc > 0 {
            cur = mem_word(mem, cur + 4);
        } else if rc < 0 || flag != 0 {
            prev = cur;
            cur = mem_word(mem, cur);
        } else {
            return cur;
        }
    }
    prev
}
struct IteratorMethod<'a, 'h, H> {
    cell: &'a core::cell::RefCell<&'h mut H>,
    heap: u32,
    out: u32,
}
impl<H: Host> MethodTableM for IteratorMethod<'_, '_, H> {
    fn call_at_m(&mut self, _obj: u32, n: u32, size: u32, mem: &mut [u8]) -> u32 {
        let mut m = Memory {
            base: 0,
            bytes: mem,
        };
        let p = self
            .cell
            .borrow_mut()
            .call_m(0x1963b8, &[self.heap, n, size], &mut m);
        // ARM publishes the out pointer before initializing the iterator header.
        m.put_word(self.out, p);
        p
    }
}
pub(crate) fn alloc_handle_for_key_live(
    m: &mut Memory<'_>,
    p: u32,
    key: u32,
    ordered: u32,
    out: u32,
    h: &mut impl Host,
) -> u32 {
    assert_eq!(m.base, 0);
    if p == 0 || out == 0 {
        return 0x89002007;
    }
    let heap = m.word(p + 20);
    let compare = m.word(m.word(p + 60) + 8);
    let cell = core::cell::RefCell::new(h);
    let mut local = 0;
    alloc_handle_for_key_m(
        m.bytes,
        nz(p),
        key,
        ordered,
        Some(&mut local),
        &mut IteratorMethod {
            cell: &cell,
            heap,
            out,
        },
        |a, b, mem| {
            cell.borrow_mut().call_m(
                compare,
                &[a, b],
                &mut Memory {
                    base: 0,
                    bytes: mem,
                },
            ) as i32
        },
    )
}

pub fn compact_stream_buffer_m(
    mem: &mut [u8],
    state: u32,
    required: u32,
    vt: &mut impl frontend::HeapVtableM,
    dtor: &mut impl FnMut(u32, u32, &mut [u8]),
) {
    let buffer = mem_word(mem, state + 60);
    if buffer == 0 || mem[(state + 40) as usize] != 0 {
        return;
    }
    let write = mem_word(mem, state + 44);
    let minimum = {
        let readers = mem_word(&*mem, state + 12);
        if readers == 0 {
            write
        } else {
            let base = mem_word(&*mem, readers);
            let table = frontend::ElementTable {
                data: &mem[base as usize..],
                elem_size: mem_half(&*mem, readers + 4),
                count: mem_word(&*mem, readers + 20),
            };
            let mut minimum = write;
            let mut index = 0u32;
            loop {
                let mut off = 0u32;
                if frontend::element_at(Some(&table), index, Some(&mut off)) != 1 {
                    break;
                }
                let stream = mem_word(&*mem, base.wrapping_add(off));
                let rp = mem_word(&*mem, stream.wrapping_add(8));
                if rp < minimum {
                    minimum = rp;
                }
                index = index.wrapping_add(1);
            }
            minimum
        }
    };
    let old = mem_word(mem, state + 56);
    let stride = mem_half(mem, state + 28) as u32;
    let mut discarded = minimum.wrapping_sub(old);
    if stride > 1 {
        let (_, rem) = frontend::divmod_u32(minimum.wrapping_sub(old), stride);
        discarded = discarded.wrapping_sub(rem);
    }
    if mem_word(mem, state + 36) != 0 && discarded != 0 {
        let heap = mem_word(mem, mem_word(mem, state + 4) + 4);
        let end = buffer.wrapping_add(discarded);
        let mut element = (mem_word(mem, state + 52).wrapping_sub(old)).wrapping_add(buffer);
        let mut dpos = mem_word(mem, state + 52);
        while element < end {
            dtor(element, heap, mem);
            element = element.wrapping_add(stride);
            dpos = dpos.wrapping_add(stride);
        }
        mem_store(mem, state + 52, dpos);
    }
    let config = mem_word(mem, state);
    let mut shrink = false;
    if required == 0 {
        if mem_word(mem, config + 24) <= discarded {
            shrink = true;
        } else {
            return;
        }
    } else if discarded < required && discarded < mem_word(mem, config + 20) {
        return;
    }
    let remaining = write.wrapping_sub(minimum);
    if remaining != 0 {
        move_bytes(mem, buffer, buffer.wrapping_add(discarded), remaining);
    }
    mem_store(mem, state + 56, old.wrapping_add(discarded));
    if shrink {
        let capacity = mem_word(mem, state + 64);
        let heap = mem_word(mem, mem_word(mem, state + 4) + 4);
        if capacity == discarded || write == minimum {
            frontend::heap_release_m(mem, nz(heap), Some(buffer), vt);
            mem_store(mem, state + 60, 0);
            mem_store(mem, state + 64, 0);
        } else {
            let newbuf = frontend::heap_realloc_m(
                mem,
                nz(heap),
                buffer,
                capacity.wrapping_sub(discarded),
                vt,
            );
            if newbuf != 0 && newbuf < 0x80000000 {
                // A failed nested heap call is not a pointer (round
                // 27b: same class as the write_stream_m store).
                mem_store(mem, state + 60, newbuf);
                mem_store(mem, state + 64, capacity.wrapping_sub(discarded));
            }
        }
    }
}
pub fn read_direct_end_m(
    mem: &mut [u8],
    stream: Option<u32>,
    b: u32,
    bytes_read: u32,
    vt: &mut impl frontend::HeapVtableM,
    dtor: &mut impl FnMut(u32, u32, &mut [u8]),
) -> u32 {
    let rc = frontend::validate_handle(stream.is_some(), b, 0x47de, 12);
    if rc >= 0x80000000 {
        return rc;
    }
    let base = stream.unwrap();
    if mem[base as usize] != 0 {
        return 0x88f02007;
    }
    let state = mem_word(mem, base + 4);
    let position = mem_word(mem, base + 8);
    let available = mem_word(mem, state + 44).wrapping_sub(position);
    let (position, rc) = if bytes_read > available {
        (position.wrapping_add(available), 0x88f02007)
    } else {
        (position.wrapping_add(bytes_read), rc)
    };
    mem_store(mem, base + 8, position);
    if position > mem_word(mem, state + 48) {
        mem_store(mem, state + 48, position);
    }
    compact_stream_buffer_m(mem, state, 0, vt, dtor);
    rc
}
pub fn write_stream_m(
    mem: &mut [u8],
    object: Option<u32>,
    tag: u32,
    source: Option<u32>,
    count: u32,
    vt: &mut impl frontend::HeapVtableM,
    copy: &mut impl FnMut(u32, u32, u32, &mut [u8]) -> u32,
    dtor: &mut impl FnMut(u32, u32, &mut [u8]),
) -> u32 {
    if frontend::validate_handle(object.is_some(), tag, 0x47de, 12) >= 0x80000000 {
        return 0x81706008;
    }
    let stream = object.unwrap();
    let Some(src) = source else {
        return 0x88f02007;
    };
    if count == 0 {
        return 0x88f02007;
    }
    let kind = mem[stream as usize];
    if kind != 1 && kind != 2 {
        return 0x88f02007;
    }
    let state = mem_word(mem, stream + 4);
    let written = mem_word(mem, state + 44);
    let base = mem_word(mem, state + 56);
    if written < base {
        return 0x88f02005;
    }
    let heap = mem_word(mem, mem_word(mem, state + 4) + 4);
    let mut buffered = 1u32;
    let need0 = written.wrapping_sub(base).wrapping_add(count);
    if mem_word(mem, state + 64) < need0 {
        let storage = mem_word(mem, state + 12);
        let stored = if storage == 0 {
            frontend::word_at5(None)
        } else {
            let mut words = [0u32; 6];
            for (index, slot) in words.iter_mut().enumerate() {
                *slot = mem_word(mem, storage + index as u32 * 4);
            }
            frontend::word_at5(Some(&words))
        };
        if mem[(state + 40) as usize] != 0 || stored != 0 {
            let capacity = mem_word(mem, state + 64);
            compact_stream_buffer_m(mem, state, need0.wrapping_sub(capacity), vt, dtor);
            let need = mem_word(mem, state + 44)
                .wrapping_sub(mem_word(mem, state + 56))
                .wrapping_add(count);
            if mem_word(mem, state + 64) < need {
                let minimum = mem_word(mem, mem_word(mem, state) + 16);
                let need = if minimum >= need { minimum } else { need };
                let buffer = mem_word(mem, state + 60);
                let grown = if buffer != 0 {
                    frontend::heap_realloc_m(mem, nz(heap), buffer, need, vt)
                } else {
                    frontend::heap_alloc_m(mem, nz(heap), need, vt)
                };
                if grown == 0 {
                    return 0x88f0200a;
                }
                if grown >= 0x80000000 {
                    // A failed nested heap call is not a pointer:
                    // propagate it instead of storing it as the stream
                    // buffer (round 27b: a stored 0x80002001 later
                    // tripped a split_at_mut in the buffered path).
                    return grown;
                }
                mem_store(mem, state + 60, grown);
                mem_store(mem, state + 64, need);
            }
        } else {
            buffered = 0;
        }
    }
    if buffered == 1 {
        let position = mem_word(mem, state + 44).wrapping_sub(mem_word(mem, state + 56));
        let buffer = mem_word(mem, state + 60);
        let dst = buffer.wrapping_add(position);
        let n = count as usize;
        let (src_us, dst_us) = (src as usize, dst as usize);
        if src_us < dst_us {
            let (head, tail) = mem.split_at_mut(dst_us);
            frontend::copy_bytes(&mut tail[..n], &head[src_us..src_us + n], count);
        } else if src_us > dst_us {
            let (head, tail) = mem.split_at_mut(src_us);
            frontend::copy_bytes(&mut head[dst_us..dst_us + n], &tail[..n], count);
        }
        if mem_word(mem, state + 32) != 0 {
            let stride = mem_half(mem, state + 28) as u32;
            let mut offset = 0u32;
            loop {
                if count <= offset {
                    break;
                }
                let at = position.wrapping_add(offset);
                if copy(src.wrapping_add(offset), heap, buffer.wrapping_add(at), mem) == 0 {
                    if mem_word(mem, state + 36) != 0 {
                        let mut done = 0u32;
                        while done < offset {
                            dtor(buffer.wrapping_add(position.wrapping_add(done)), heap, mem);
                            done = done.wrapping_add(stride);
                        }
                    }
                    return 0x88f0200a;
                }
                offset = offset.wrapping_add(stride);
            }
        }
    } else {
        let at = mem_word(mem, state + 56);
        mem_store(mem, state + 56, at.wrapping_add(count));
    }
    let at = mem_word(mem, state + 44);
    mem_store(mem, state + 44, at.wrapping_add(count));
    0
}

struct StreamHeap<'a, 'h, H> {
    cell: &'a core::cell::RefCell<&'h mut H>,
    heap: u32,
}
impl<H: Host> frontend::HeapVtableM for StreamHeap<'_, '_, H> {
    fn alloc_at_m(&mut self, obj: u32, n: u32, mem: &mut [u8]) -> u32 {
        let f = mem_word(mem, mem_word(mem, self.heap));
        self.cell.borrow_mut().call_m(
            f,
            &[obj, n],
            &mut Memory {
                base: 0,
                bytes: mem,
            },
        )
    }
    fn realloc_at_m(&mut self, obj: u32, p: u32, n: u32, mem: &mut [u8]) -> u32 {
        let f = mem_word(mem, mem_word(mem, self.heap) + 8);
        self.cell.borrow_mut().call_m(
            f,
            &[obj, p, n],
            &mut Memory {
                base: 0,
                bytes: mem,
            },
        )
    }
    fn release_at_m(&mut self, obj: u32, p: u32, mem: &mut [u8]) {
        let f = mem_word(mem, mem_word(mem, self.heap) + 12);
        self.cell.borrow_mut().call_m(
            f,
            &[obj, p],
            &mut Memory {
                base: 0,
                bytes: mem,
            },
        );
    }
}
pub fn stream_call_m(m: &mut Memory<'_>, entry: u32, a: &[u32], h: &mut impl Host) -> u32 {
    assert_eq!(m.base, 0);
    if frontend::validate_handle(a[0] != 0, a[1], 0x47de, 12) >= 0x80000000 {
        return 0x81706008;
    }
    let state = m.word(a[0] + 4);
    let heap = m.word(m.word(state + 4) + 4);
    let cell = core::cell::RefCell::new(&mut *h);
    let mut vt = StreamHeap { cell: &cell, heap };
    let mut destroy = |p, heap, mem: &mut [u8]| {
        let f = mem_word(mem, state + 36);
        cell.borrow_mut().call_m(
            f,
            &[p, heap],
            &mut Memory {
                base: 0,
                bytes: mem,
            },
        );
    };
    match entry {
        0x1503b0 => {
            let mut copy = |src, heap, dst, mem: &mut [u8]| {
                let f = mem_word(mem, state + 32);
                cell.borrow_mut().call_m(
                    f,
                    &[src, heap, dst],
                    &mut Memory {
                        base: 0,
                        bytes: mem,
                    },
                )
            };
            let rc = write_stream_m(
                m.bytes,
                nz(a[0]),
                a[1],
                nz(a[2]),
                a[3],
                &mut vt,
                &mut copy,
                &mut destroy,
            );
            if rc == 0 {
                cell.borrow_mut().call_m(0x14e614, &[state, a[2], a[3]], m);
            }
            rc
        }
        0x15017c => read_direct_end_m(m.bytes, nz(a[0]), a[1], a[2], &mut vt, &mut destroy),
        _ => unreachable!("stream entry"),
    }
}

pub fn duplicate_typed_string_m(
    mem: &mut [u8],
    p: Option<u32>,
    heap: Option<u32>,
    out: Option<u32>,
    vt: &mut impl frontend::HeapVtableM,
) -> u32 {
    let (Some(p), Some(out)) = (p, out) else {
        return 1;
    };
    if !STRING_KINDS.contains(&mem_half(&*mem, p)) {
        return 1;
    }
    let s = mem_word(mem, p + 20);
    let n = if s == 0 { 0 } else { u16_len(mem, s) };
    let d = frontend::heap_alloc_m(mem, heap, n.wrapping_mul(2).wrapping_add(2), vt);
    if d == 0 {
        return 0;
    }
    if n == 0 {
        let at = d as usize;
        mem[at..at + 2].copy_from_slice(&0u16.to_le_bytes());
    } else {
        frontend::u16_copy(mem, d, s);
    }
    mem_store(mem, out + 20, d);
    1
}
pub fn release_typed_buffer_m(
    mem: &mut [u8],
    p: Option<u32>,
    heap: u32,
    vt: &mut impl frontend::HeapVtableM,
) {
    let Some(p) = p else {
        return;
    };
    if STRING_KINDS.contains(&mem_half(mem, p)) && mem_word(mem, p + 20) != 0 {
        frontend::heap_release_m(mem, nz(heap), Some(mem_word(mem, p + 20)), vt);
        mem_store(mem, p + 20, 0);
    }
}
const STRING_KINDS: &[u16] = &[6, 12, 21, 50, 36, 39, 37, 34, 22, 40, 26, 29];

impl crate::speech_part_1::Host for LiveHost<'_> {
    fn call(&mut self, m: &mut Memory<'_>, f: u32, a: &[u32]) -> u32 {
        self.call_m(f, a, m)
    }
    fn get(
        &mut self,
        m: &mut Memory<'_>,
        f: u32,
        handle: u32,
        pos: u32,
        len: u32,
    ) -> (u32, u32, u32) {
        let s = self.scratch + 0x100;
        let saved = [m.word(s), m.word(s + 4)];
        m.put_word(s, 0);
        m.put_word(s + 4, 0);
        let rc = self.call_m(f, &[handle, pos, len, s, s + 4], m);
        let got = (rc, m.word(s), m.word(s + 4));
        m.put_word(s, saved[0]);
        m.put_word(s + 4, saved[1]);
        got
    }
}
impl crate::speech_part_6::Host for LiveHost<'_> {
    fn call(&mut self, m: &mut Memory<'_>, f: u32, a: &[u32]) -> u32 {
        self.call_m(f, a, m)
    }
    fn log(
        &mut self,
        m: &mut Memory<'_>,
        logger: u32,
        category: u32,
        level: u32,
        format: &[u8],
        values: &[u32],
    ) {
        let filtered = logger == 0
            || level > m.byte(logger + 48) as u32
            || (m.byte(logger + 49) != 0 && level > 4);
        let fmt = if filtered {
            0
        } else {
            literal(m.bytes, format)
        };
        let mut args = [0u32; 17];
        args[..5].copy_from_slice(&[logger, category, level, 0, fmt]);
        args[5..5 + values.len()].copy_from_slice(values);
        self.call_m(0x1738e8, &args[..5 + values.len()], m);
    }
    fn log_guest(
        &mut self,
        m: &mut Memory<'_>,
        logger: u32,
        category: u32,
        level: u32,
        fmt: u32,
        values: &[u32],
    ) {
        let mut args = [0u32; 17];
        args[..5].copy_from_slice(&[logger, category, level, 0, fmt]);
        args[5..5 + values.len()].copy_from_slice(values);
        self.call_m(0x1738e8, &args[..5 + values.len()], m);
    }
    fn error(&mut self, m: &mut Memory<'_>, logger: u32, category: u32, code: u32) {
        self.call_m(0x173dc0, &[logger, category, code, 0], m);
    }
}
/// Live root binding; scratch is the existing port's 136 restored bytes.
pub fn fn_1e57d0_m<H: crate::speech_part_1::Host + crate::speech_part_6::Host>(
    m: &mut Memory<'_>,
    a: [u32; 4],
    env: &crate::speech_part_1::StreamEnvironment,
    scratch: u32,
    h: &mut H,
) -> u32 {
    crate::speech_part_1::fn_1e57d0(m, a[0], a[1], [a[2], a[3]], env, scratch, h)
}
/// Live root binding; scratch is the existing port's 132 restored bytes.
pub fn fn_1d135c_m<H: crate::speech_part_1::Host + crate::speech_part_6::Host>(
    m: &mut Memory<'_>,
    a: [u32; 4],
    env: &crate::speech_part_1::LanguageEnvironment,
    scratch: u32,
    h: &mut H,
) -> u32 {
    crate::speech_part_1::fn_1d135c(m, a[0], a[1], [a[2], a[3]], env, scratch, h)
}
/// Live root binding; scratch is eight restored bytes. end_name is a mapped literal.
pub fn fn_1e4b7c_m<H: crate::speech_part_1::Host + crate::speech_part_6::Host>(
    m: &mut Memory<'_>,
    a: [u32; 5],
    end_name: u32,
    scratch: u32,
    h: &mut H,
) -> u32 {
    crate::speech_part_1::fn_1e4b7c(m, a[0], a[1], [a[2], a[3]], a[4], end_name, scratch, h)
}
struct ComponentCalls<'a, H>(&'a mut H);
impl<H: Host> crate::engine_components::InternalCalls for ComponentCalls<'_, H> {
    fn call(
        &mut self,
        m: &mut crate::engine::Memory<'_, '_>,
        f: u32,
        a: &[u32],
    ) -> Result<u32, crate::engine::Error> {
        let mut arena = crate::bridge::boot_view(m, 0, 1)?;
        Ok(self.0.call_m(f, a, &mut arena))
    }
}
/// Existing live component body over the same arena, without a copy or refill.
pub fn fn_1e5bbc_m(
    m: &mut Memory<'_>,
    a: [u32; 5],
    scratch: u32,
    h: &mut impl Host,
) -> Result<u32, crate::engine::Error> {
    assert_eq!(m.base, 0);
    let mut regions = [crate::engine::Region {
        base: 0,
        bytes: m.bytes,
    }];
    let mut view = crate::engine::Memory::new(&mut regions)?;
    crate::engine_components::process_1e5bbc(
        &mut view,
        a[0],
        a[1],
        a[4],
        scratch,
        &mut ComponentCalls(h),
    )
}

/// fn_26a42c. Read the backend input-format flag into a caller-owned byte.
pub fn fn_26a42c(m: &mut Memory<'_>, p: u32, out: u32) -> u32 {
    if p == 0 {
        return 0x81102007;
    }
    let at = m.offset(out);
    m.bytes[at] = m.byte(p + 0x77);
    0
}

struct NormAdapter<'a, H>(&'a mut H);
impl<H: Host> crate::normalization::NormalizationHost for NormAdapter<'_, H> {
    fn calloc(&mut self, bytes: &mut [u8], heap: u32, n: u32, size: u32) -> Option<u32> {
        nz(self
            .0
            .call_m(0x1963b8, &[heap, n, size], &mut Memory { base: 0, bytes }))
    }
    fn alloc(&mut self, bytes: &mut [u8], heap: u32, n: u32) -> Option<u32> {
        nz(self
            .0
            .call_m(0x196394, &[heap, n], &mut Memory { base: 0, bytes }))
    }
    fn realloc(&mut self, bytes: &mut [u8], heap: u32, p: u32, n: u32) -> Option<u32> {
        nz(self
            .0
            .call_m(0x1963e0, &[heap, p, n], &mut Memory { base: 0, bytes }))
    }
    fn free(&mut self, bytes: &mut [u8], heap: u32, p: u32) {
        self.0
            .call_m(0x196428, &[heap, p], &mut Memory { base: 0, bytes });
    }
    fn resolve(&mut self, bytes: &mut [u8], a: u32, b: u32) -> (u32, u32) {
        let mut m = Memory { base: 0, bytes };
        let saved = m.word(0xc00);
        let rc = self.0.call_m(0x19611c, &[a, b, 0xc00], &mut m);
        let ctx = m.word(0xc00);
        m.put_word(0xc00, saved);
        (rc, ctx)
    }
}
/// Reentrant fn_2c47d0. Heap release executes on the current arena.
pub fn reset_cursor_m(m: &mut Memory<'_>, p: u32, out: u32, h: &mut impl Host) -> u32 {
    if p == 0 {
        return 0x80002007;
    }
    
    let env = crate::render_part_c::Environment {
        cached_handle: core::array::from_fn(|i| m.byte(addr(h.rom(), 0x8d7660) + i as u32)),
        default_context: addr(h.rom(), 0x8d7640),
    };
    let (rc, owner) = crate::render_part_c::resolve(m, [m.word(p), m.word(p + 4)], &env);
    if rc >= 0x80000000 {
        return 0x80002007;
    }
    m.put_word(p + 12, 0);
    m.put_word(p + 16, 0);
    m.put_byte(p + 21, 0);
    let q = m.word(p + 8);
    let mut rc = crate::normalization::byte_clear_4a(m.bytes, q);
    let s = 0xc10;
    let saved = m.word(s);
    let mut entries = 0;
    if rc < 0x80000000 {
        m.put_word(s, 0);
        rc = crate::render_part_d::initial_values(
            m.bytes,
            q,
            s,
            0xc20,
            addr(h.rom(), 0xa093e6),
            &mut NormAdapter(h),
        );
        entries = m.word(s);
        if rc < 0x80000000 {
            for i in 1..40u32 {
                let entry = entries + i * 40;
                if m.half(entry + 10) == 31 {
                    m.put_word(out, m.word(entry + 28));
                }
                if matches!(
                    m.half(entry + 10),
                    6 | 12 | 21 | 50 | 36 | 39 | 37 | 34 | 22 | 40 | 26 | 29
                ) && m.word(entry + 28) != 0
                {
                    h.call_m(0x196428, &[m.word(owner + 4), m.word(entry + 28)], m);
                    m.put_word(entry + 28, 0);
                }
            }
        }
    }
    if entries != 0 {
        h.call_m(0x196428, &[m.word(owner + 4), entries], m);
    }
    m.put_word(s, saved);
    rc
}
/// Reentrant fn_2c2338. Scratch is a restored four-byte getter output.
pub fn reset_markers_m(m: &mut Memory<'_>, p: u32, value: u32, s: u32, h: &mut impl Host) -> u32 {
    
    let env = crate::render_part_c::Environment {
        cached_handle: core::array::from_fn(|i| m.byte(addr(h.rom(), 0x8d7660) + i as u32)),
        default_context: addr(h.rom(), 0x8d7640),
    };
    let (rc, ctx) = crate::render_part_c::resolve(m, [m.word(p), m.word(p + 4)], &env);
    if rc >= 0x80000000 {
        return 0x81102007;
    }
    m.put_word(p + 96, value);
    m.put_word(p + 84, 0);
    m.put_half(p + 100, 200);
    let saved = m.word(s);
    m.put_word(s, 0);
    let key = literal(m.bytes, b"ppmaxmarkersinemptytext");
    let rc = h.call_m(0x175944, &[m.word(ctx + 20), key, s], m);
    let n = m.word(s);
    m.put_word(s, saved);
    if rc & 0x1fff == 0x14 {
        return 0;
    }
    if rc >= 0x80000000 {
        return rc;
    }
    if n.wrapping_sub(1) < 0xfffe {
        m.put_half(p + 100, n as u16);
    } else {
        h.call_m(0x173dc0, &[m.word(ctx + 16), addr(h.rom(), 0x9c5650), 0x71c, 0], m);
    }
    rc
}

struct PropertyLockM<'a, 'h, H> {
    cell: &'a core::cell::RefCell<&'h mut H>,
    obj: u32,
}
impl<H: Host> crate::reentrant2::StateLockM for PropertyLockM<'_, '_, H> {
    fn acquire_m(&mut self, bytes: &mut [u8]) -> u32 {
        let mut m = Memory { base: 0, bytes };
        let p = m.word(self.obj + 4);
        self.cell.borrow_mut().call_m(0x176f2c, &[p], &mut m)
    }
    fn release_m(&mut self, bytes: &mut [u8]) -> u32 {
        let mut m = Memory { base: 0, bytes };
        let p = m.word(self.obj + 4);
        self.cell.borrow_mut().call_m(0x176f5c, &[p], &mut m)
    }
}
/// Landed reentrant parameter lookup/getter with ABI output slots.
pub fn get_property_m(m: &mut Memory<'_>, f: u32, a: &[u32], h: &mut impl Host) -> u32 {
    let cell = core::cell::RefCell::new(h);
    let obj = a[0];
    let mut lock = PropertyLockM { cell: &cell, obj };
    let cmp = |x, y, bytes: &mut [u8]| {
        let mut m = Memory { base: 0, bytes };
        let target = m.word(m.word(m.word(obj + 8) + 60) + 8);
        cell.borrow_mut().call_m(target, &[x, y], &mut m) as i32
    };
    if f == 0x175944 {
        return crate::reentrant2::get_param_uint_m(m.bytes, nz(obj), a[1], a[2], &mut lock, cmp);
    }
    if obj == 0 || a[1] == 0 || a[2] == 0 {
        return 0x8b702007;
    }
    let size_out = if f == 0x1756d0 { a[3] } else { 0 };
    // fn_1756d0 publishes its ABI outputs before acquiring the lock and
    // before unlocking. Keeping these in Rust locals would hide their writes
    // from reentrant lock callbacks and overwrite the unlocker's mutations.
    m.put_word(a[2], 0);
    if size_out != 0 {
        m.put_word(size_out, 0);
    }
    let mut result = cell.borrow_mut().call_m(0x176f2c, &[m.word(obj + 4)], m);
    if result >= 0x80000000 {
        return result;
    }
    let map = m.word(obj + 8);
    let mut entry = 0;
    let rc = crate::reentrant2::lookup_value_m(
        m.bytes,
        nz(map),
        a[1],
        Some(&mut entry),
        |x, y, bytes| {
            let mut m = Memory { base: 0, bytes };
            let target = m.word(m.word(map + 60) + 8);
            cell.borrow_mut().call_m(target, &[x, y], &mut m) as i32
        },
    );
    if rc >= 0x80000000 {
        result = (rc & 0x1fff) | 0x8b702000;
    } else {
        result = 0;
        if entry != 0 {
            if m.word(entry + 4) != 0 {
                m.put_word(a[2], m.word(entry));
                if size_out != 0 {
                    m.put_word(size_out, m.word(entry + 4));
                }
            } else {
                m.put_word(a[2], 0);
                if size_out != 0 {
                    m.put_word(size_out, 0);
                }
            }
        }
    }
    let released = cell.borrow_mut().call_m(0x176f5c, &[m.word(obj + 4)], m);
    if released >= 0x80000000 && result < 0x80000000 {
        result = released;
    }
    result
}

/// Project a single live record into the certified read-only database ports.
pub fn database_read_m(m: &mut Memory<'_>, f: u32, a: &[u32], h: &mut impl Host) -> u32 {
    use crate::linguistic::{Database, Field, FieldDatabase, Record};
    let p = a[0];
    let valid = frontend::validate_handle(p != 0, a[1], 0xb2c2, 0x38) < 0x80000000;
    let record = if valid && a[2] < m.word(p + 32) {
        m.word(m.word(p + 48) + a[2] * 4)
    } else {
        0
    };
    let kind = if record == 0 { 0 } else { m.byte(record + 2) };
    let mut records = [None; 2];
    if record != 0 {
        records[0] = Some(Record {
            id: m.half(record),
            kind,
            ..Record::default()
        });
    }
    if f == 0x15c7d8 {
        if record != 0 {
            let next = m.word(record + 8);
            if next != 0 {
                records[1] = Some(Record {
                    id: m.half(next),
                    kind: m.byte(next + 2),
                    ..Record::default()
                });
                records[0].as_mut().unwrap().next = Some(1);
            }
        }
        let db = Database {
            records: &records,
            parents: &[],
            allocated: false,
            valid,
        };
        let mut out = m.half(a[3]);
        let rc = db.next_in_sentence(0, &mut out);
        if rc < 0x80000000 {
            m.put_half(a[3], out);
        }
        if rc >= 0x80000000 && valid {
            database_error(m, f, a, record, kind, rc, h);
        }
        return rc;
    }
    let layout = addr(h.rom(), 0x8d7ca0) + kind as u32 * 272;
    if !guest_range(m, layout, 272) {
        return 0x89203e0d;
    }
    let nf = if kind == 0 {
        0
    } else {
        m.half(layout + 4) as usize
    };
    if nf > 13 {
        return 0x89203e0d;
    }
    let base = m.word(layout);
    let mut fields = [Field {
        data: None,
        state: 0,
        fixed_count: 0,
        count: None,
        element_size: 0,
    }; 13];
    let mut addresses = [0; 13];
    for i in 0..nf {
        let desc = layout + 12 + i as u32 * 20;
        let fixed = m.half(desc + 8);
        let address = record.wrapping_add(m.word(desc).wrapping_sub(base));
        let address = if fixed == 0 { m.word(address) } else { address };
        addresses[i] = address;
        let count = m.word(desc + 4);
        let count = if count == 0 {
            None
        } else {
            Some(m.half(record.wrapping_add(count.wrapping_sub(base))))
        };
        let size = m.half(desc + 10);
        let n = count.unwrap_or(fixed) as usize * size as usize;
        fields[i] = Field {
            data: if address == 0 {
                None
            } else {
                Some(&m.bytes[address as usize..address as usize + n])
            },
            state: m.byte(record.wrapping_add(m.word(layout + 8).wrapping_sub(base)) + i as u32),
            fixed_count: fixed,
            count,
            element_size: size,
        };
    }
    let slices = [&fields[..nf], &[]];
    let db = FieldDatabase {
        records: &records,
        fields: &slices,
        valid,
    };
    let rc = if f == 0x15d594 {
        let mut out = m.half(a[4]);
        let rc = db.field_state(0, a[3], &mut out);
        if rc < 0x80000000 {
            m.put_half(a[4], out);
        }
        rc
    } else if f == 0x15d378 {
        let mut out = None;
        let mut count = m.half(a[5]);
        let rc = db.field_data(0, a[3], &mut out, &mut count);
        if rc < 0x80000000 {
            m.put_word(a[4], addresses[a[3] as usize]);
            m.put_half(a[5], count);
        }
        rc
    } else {
        let mut output = [0u8; 8192];
        let mut written = 0;
        let rc = db
            .read_field(0, a[3], a[4], &mut output, &mut written)
            .expect("database read workspace");
        if rc < 0x80000000 {
            let n = written as usize * fields[a[3] as usize].element_size as usize;
            let at = a[5] as usize;
            m.bytes[at..at + n].copy_from_slice(&output[..n]);
        }
        m.put_half(a[6], written);
        rc
    };
    if rc >= 0x80000000 && valid {
        database_error(m, f, a, record, kind, rc, h);
    }
    rc
}

pub trait PromptTokenHooksM {
    /// `fn_176f94`: validate the prompter.
    fn check(&mut self, p: u32, check: u32, f1: u32, f2: u32, mem: &mut [u8]) -> u32;
    /// Token-table first call.
    fn first(&mut self, h0: u32, h1: u32, a: u32, b: u32, out: &mut u16, mem: &mut [u8]) -> u32;
    /// Token-table getCount call.
    fn get_count(
        &mut self,
        h0: u32,
        h1: u32,
        root: u16,
        flag: u32,
        attr: &mut u16,
        mem: &mut [u8],
    ) -> u32;
    /// Token-table getPointer call; `ptr` is an arena offset.
    fn get_pointer(
        &mut self,
        h0: u32,
        h1: u32,
        root: u16,
        flag: u32,
        ptr: &mut u32,
        len: &mut u16,
        mem: &mut [u8],
    ) -> u32;
    /// Prompt-table process call; `None` is the null pointer.
    #[allow(clippy::too_many_arguments)]
    fn process(
        &mut self,
        t0: u32,
        t1: u32,
        text: &mut Option<u32>,
        records: &mut Option<u32>,
        count: &mut u16,
        h0: u32,
        h1: u32,
        f0: u32,
        f1: u32,
        s0: u32,
        s1: u32,
        delta: &mut i16,
        mem: &mut [u8],
    ) -> u32;
    /// Token-table kind probe; shares the get script with `get_pause`.
    fn get_kind(
        &mut self,
        h0: u32,
        h1: u32,
        id: u16,
        f1: u32,
        f2: u32,
        kind: &mut u8,
        count: &mut u16,
        mem: &mut [u8],
    ) -> u32;
    /// Token-table pause probe; shares the get script with `get_kind`.
    fn get_pause(
        &mut self,
        h0: u32,
        h1: u32,
        id: u16,
        f1: u32,
        f2: u32,
        pause: &mut u16,
        count: &mut u16,
        mem: &mut [u8],
    ) -> u32;
    /// Token-table next call.
    fn next(&mut self, h0: u32, h1: u32, token: u16, out: &mut u16, mem: &mut [u8]) -> u32;
    /// Token-table set call; `ptr` is an arena offset.
    #[allow(clippy::too_many_arguments)]
    fn set(
        &mut self,
        h0: u32,
        h1: u32,
        id: u16,
        flag: u32,
        count: u32,
        ptr: u32,
        written: &mut u16,
        mem: &mut [u8],
    ) -> u32;
    /// Token-table pause store; the C passes a stack slot, so the
    /// value crosses the hook instead of a pointer. Shares the set
    /// script with `set`.
    #[allow(clippy::too_many_arguments)]
    fn set_pause(
        &mut self,
        h0: u32,
        h1: u32,
        id: u16,
        flag: u32,
        count: u32,
        value: u16,
        written: &mut u16,
        mem: &mut [u8],
    ) -> u32;
    /// Position-table get call.
    fn get_position(&mut self, h0: u32, h1: u32, pos: &mut u32, mem: &mut [u8]) -> u32;
    /// Position-table set call.
    fn set_position(&mut self, h0: u32, h1: u32, pos: u32, mem: &mut [u8]) -> u32;
    /// `fn_196394`: allocate `size` bytes from `heap`; `None` on failure.
    fn alloc_text(&mut self, heap: u32, size: u32, mem: &mut [u8]) -> Option<u32>;
    /// `fn_1963b8`: allocate `size` bytes from `heap`; `None` on failure.
    fn alloc_records(&mut self, heap: u32, size: u32, mem: &mut [u8]) -> Option<u32>;
    /// `fn_196428`: free `ptr` back to `heap`.
    fn free_buf(&mut self, heap: u32, ptr: u32, mem: &mut [u8]);
    /// `fn_173dc0`: allocation-failure trace.
    fn log_no_memory(&mut self, log: u32, code: u32, flag: u32, mem: &mut [u8]);
    /// `fn_1738e8`: pause-store trace.
    fn log_pause(&mut self, log: u32, level: u32, flag: u32, mem: &mut [u8]);
}

/// fn_1f3ea0 / EU fn_1f405c / KR fn_1f4108.
///
/// Resolve the prompt text and its records,
/// run the transform, walk the token chain
/// storing a pause of 5 across phrase-kind
/// boundaries, store a surviving result back,
/// shift the first record and the stream
/// position by the returned delta and release
/// both buffers. A failing check reports
/// 0x8ba02008 and an allocation failure
/// 0x8ba0200a; a rejected root returns the
/// root probe's status, and every other
/// failure returns its raw status.
pub fn prompt_token_m<H: PromptTokenHooksM>(
    hooks: &mut H,
    mem: &mut [u8],
    p: u32,
    check: u32,
    h0: u32,
    h1: u32,
    applied: u32,
) -> u32 {
    let mut result = hooks.check(p, check, 0xf382, 0x34, mem);
    if result >= 0x80000000 {
        return 0x8ba02008;
    }
    mem[applied as usize] = 1;
    let ctx = mem_word(mem, p);
    let heap = mem_word(mem, ctx.wrapping_add(4));
    let log = mem_word(mem, ctx.wrapping_add(16));
    let mut root = 0u16;
    result = hooks.first(h0, h1, 1, 0, &mut root, mem);
    if result >= 0x80000000 {
        return result;
    }
    let mut attr = 0u16;
    let status = hooks.get_count(h0, h1, root, 0, &mut attr, mem);
    if status >= 0x80000000 || attr != 1 {
        return result;
    }
    let mut source = 0u32;
    let mut length = 0u16;
    let status = hooks.get_pointer(h0, h1, root, 0, &mut source, &mut length, mem);
    if status >= 0x80000000 || length <= 1 {
        return status;
    }
    let mut source_rec = 0u32;
    result = hooks.get_pointer(h0, h1, root, 1, &mut source_rec, &mut length, mem);
    if result >= 0x80000000 {
        return result;
    }
    let mut record_count = length;
    let size = crate::normalization::strlen8(mem, source).wrapping_add(1);
    let Some(buf) = hooks.alloc_text(heap, size, mem) else {
        hooks.log_no_memory(log, 0xb090, 0, mem);
        return 0x8ba0200a;
    };
    crate::normalization::strcpy_at(mem, buf, source);
    let mut text = Some(buf);
    let bytes = (record_count as u32).wrapping_mul(24);
    let Some(rec) = hooks.alloc_records(heap, bytes, mem) else {
        hooks.log_no_memory(log, 0xb090, 0, mem);
        hooks.free_buf(heap, buf, mem);
        return 0x8ba0200a;
    };
    if bytes != 0 {
        mem.copy_within(
            source_rec as usize..(source_rec.wrapping_add(bytes)) as usize,
            rec as usize,
        );
    }
    let mut records = Some(rec);
    let mut delta = 0i16;
    result = hooks.process(
        mem_word(mem, p.wrapping_add(16)),
        mem_word(mem, p.wrapping_add(20)),
        &mut text,
        &mut records,
        &mut record_count,
        h0,
        h1,
        mem_word(mem, p.wrapping_add(36)),
        mem_word(mem, p.wrapping_add(40)),
        mem_word(mem, p.wrapping_add(44)),
        mem_word(mem, p.wrapping_add(48)),
        &mut delta,
        mem,
    );
    if result >= 0x80000000 {
        if let Some(buf) = text {
            hooks.free_buf(heap, buf, mem);
        }
        if let Some(rec) = records {
            hooks.free_buf(heap, rec, mem);
        }
        return result;
    }
    let mut token = 0u16;
    result = hooks.first(h0, h1, 2, root as u32, &mut token, mem);
    if result >= 0x80000000 {
        if let Some(buf) = text {
            hooks.free_buf(heap, buf, mem);
        }
        if let Some(rec) = records {
            hooks.free_buf(heap, rec, mem);
        }
        return result;
    }
    let mut previous = 0u16;
    let mut previous_kind = 4u8;
    let mut kind = 11u8;
    let mut count = 0u16;
    let mut written = 0u16;
    while token != 0 {
        result = hooks.get_kind(h0, h1, token, 0, 1, &mut kind, &mut count, mem);
        if result >= 0x80000000 {
            if let Some(buf) = text {
                hooks.free_buf(heap, buf, mem);
            }
            if let Some(rec) = records {
                hooks.free_buf(heap, rec, mem);
            }
            return result;
        }
        if matches!(kind, 4 | 9 | 10 | 14 | 12 | 13 | 11) {
            if previous != 0 {
                let closing = matches!(previous_kind, 14 | 12) && matches!(kind, 4 | 9 | 10 | 13);
                let opening = matches!(previous_kind, 4 | 9 | 10 | 13 | 11) && kind == 14;
                if closing || opening {
                    let mut pause = 0u16;
                    result = hooks.get_pause(h0, h1, previous, 8, 1, &mut pause, &mut count, mem);
                    if result >= 0x80000000 {
                        if let Some(buf) = text {
                            hooks.free_buf(heap, buf, mem);
                        }
                        if let Some(rec) = records {
                            hooks.free_buf(heap, rec, mem);
                        }
                        return result;
                    }
                    if pause == 0 {
                        pause = 5;
                        result = hooks.set_pause(h0, h1, previous, 8, 1, pause, &mut written, mem);
                        if result >= 0x80000000 {
                            if let Some(buf) = text {
                                hooks.free_buf(heap, buf, mem);
                            }
                            if let Some(rec) = records {
                                hooks.free_buf(heap, rec, mem);
                            }
                            return result;
                        }
                        hooks.log_pause(log, 5, 0, mem);
                    }
                }
            }
            previous = token;
            previous_kind = kind;
        }
        result = hooks.next(h0, h1, token, &mut token, mem);
        if result >= 0x80000000 {
            if let Some(buf) = text {
                hooks.free_buf(heap, buf, mem);
            }
            if let Some(rec) = records {
                hooks.free_buf(heap, rec, mem);
            }
            return result;
        }
    }
    if let Some(buf) = text {
        let n = crate::normalization::strlen8(mem, buf).wrapping_add(1) as u16 as u32;
        result = hooks.set(h0, h1, root, 0, n, buf, &mut written, mem);
        if result >= 0x80000000 {
            hooks.free_buf(heap, buf, mem);
            if let Some(rec) = records {
                hooks.free_buf(heap, rec, mem);
            }
            return result;
        }
    }
    if let Some(rec) = records {
        if record_count != 0 {
            let pos = mem_word(mem, rec.wrapping_add(16));
            mem_store(mem, rec.wrapping_add(16), pos.wrapping_add(delta as u32));
            result = hooks.set(h0, h1, root, 1, record_count as u32, rec, &mut written, mem);
            if result >= 0x80000000 {
                if let Some(buf) = text {
                    hooks.free_buf(heap, buf, mem);
                }
                hooks.free_buf(heap, rec, mem);
                return result;
            }
        }
    }
    let mut position = 0u32;
    result = hooks.get_position(
        mem_word(mem, p.wrapping_add(28)),
        mem_word(mem, p.wrapping_add(32)),
        &mut position,
        mem,
    );
    if result >= 0x80000000 {
        if let Some(buf) = text {
            hooks.free_buf(heap, buf, mem);
        }
        if let Some(rec) = records {
            hooks.free_buf(heap, rec, mem);
        }
        return result;
    }
    position = position.wrapping_add(delta as u32);
    result = hooks.set_position(
        mem_word(mem, p.wrapping_add(28)),
        mem_word(mem, p.wrapping_add(32)),
        position,
        mem,
    );
    if let Some(buf) = text {
        hooks.free_buf(heap, buf, mem);
    }
    if let Some(rec) = records {
        hooks.free_buf(heap, rec, mem);
    }
    result
}

pub struct PromptTokenAdapter<'a, H> {
    io: PreprocessAdapter<'a, H>,
}
impl<H: Host> PromptTokenHooksM for PromptTokenAdapter<'_, H> {
    fn check(&mut self, p: u32, key: u32, x: u32, y: u32, _: &mut [u8]) -> u32 {
        frontend::validate_handle(p != 0, key, x, y)
    }
    fn first(&mut self, a: u32, b: u32, x: u32, y: u32, out: &mut u16, mem: &mut [u8]) -> u32 {
        let s = self.io.scratch;
        let (rc, v) = self.io.out(
            mem,
            self.io.method(mem, 4, 52),
            &[a, b, x, y, s],
            [*out as u32, 0, 0],
        );
        *out = v[0] as u16;
        rc
    }
    fn get_count(&mut self, a: u32, b: u32, x: u16, y: u32, out: &mut u16, mem: &mut [u8]) -> u32 {
        let s = self.io.scratch;
        let (rc, v) = self.io.out(
            mem,
            self.io.method(mem, 4, 92),
            &[a, b, x as u32, y, s],
            [*out as u32, 0, 0],
        );
        *out = v[0] as u16;
        rc
    }
    fn get_pointer(
        &mut self,
        a: u32,
        b: u32,
        x: u16,
        y: u32,
        out: &mut u32,
        n: &mut u16,
        mem: &mut [u8],
    ) -> u32 {
        let s = self.io.scratch;
        let (rc, v) = self.io.out(
            mem,
            self.io.method(mem, 4, 88),
            &[a, b, x as u32, y, s, s + 4],
            [*out, *n as u32, 0],
        );
        *out = v[0];
        *n = v[1] as u16;
        rc
    }
    fn process(
        &mut self,
        a: u32,
        b: u32,
        text: &mut Option<u32>,
        records: &mut Option<u32>,
        n: &mut u16,
        h0: u32,
        h1: u32,
        f0: u32,
        f1: u32,
        s0: u32,
        s1: u32,
        delta: &mut i16,
        mem: &mut [u8],
    ) -> u32 {
        let s = self.io.scratch;
        let at = s as usize;
        let saved: [u8; 64] = mem[at..at + 64].try_into().unwrap();
        mem_store(mem, s, text.unwrap_or(0));
        mem_store(mem, s + 4, records.unwrap_or(0));
        mem_store16(mem, s + 8, *n);
        mem_store16(mem, s + 12, *delta as u16);
        let rc = self.io.invoke(
            mem,
            self.io.method(mem, 12, 52),
            &[a, b, s, s + 4, s + 8, h0, h1, f0, f1, s0, s1, s + 12],
        );
        *text = nz(mem_word(mem, s));
        *records = nz(mem_word(mem, s + 4));
        *n = mem_half(mem, s + 8);
        *delta = mem_half(mem, s + 12) as i16;
        mem[at..at + 64].copy_from_slice(&saved);
        rc
    }
    fn get_kind(
        &mut self,
        a: u32,
        b: u32,
        id: u16,
        f1: u32,
        f2: u32,
        kind: &mut u8,
        n: &mut u16,
        mem: &mut [u8],
    ) -> u32 {
        let s = self.io.scratch;
        let (rc, v) = self.io.out(
            mem,
            self.io.method(mem, 4, 84),
            &[a, b, id as u32, f1, f2, s, s + 4],
            [*kind as u32, *n as u32, 0],
        );
        *kind = v[0] as u8;
        *n = v[1] as u16;
        rc
    }
    fn get_pause(
        &mut self,
        a: u32,
        b: u32,
        id: u16,
        f1: u32,
        f2: u32,
        pause: &mut u16,
        n: &mut u16,
        mem: &mut [u8],
    ) -> u32 {
        let s = self.io.scratch;
        let (rc, v) = self.io.out(
            mem,
            self.io.method(mem, 4, 84),
            &[a, b, id as u32, f1, f2, s, s + 4],
            [*pause as u32, *n as u32, 0],
        );
        *pause = v[0] as u16;
        *n = v[1] as u16;
        rc
    }
    fn next(&mut self, a: u32, b: u32, id: u16, out: &mut u16, mem: &mut [u8]) -> u32 {
        let s = self.io.scratch;
        let (rc, v) = self.io.out(
            mem,
            self.io.method(mem, 4, 60),
            &[a, b, id as u32, s],
            [*out as u32, 0, 0],
        );
        *out = v[0] as u16;
        rc
    }
    fn set(
        &mut self,
        a: u32,
        b: u32,
        id: u16,
        flag: u32,
        n: u32,
        ptr: u32,
        out: &mut u16,
        mem: &mut [u8],
    ) -> u32 {
        let s = self.io.scratch;
        let (rc, v) = self.io.out(
            mem,
            self.io.method(mem, 4, 80),
            &[a, b, id as u32, flag, n, ptr, s],
            [*out as u32, 0, 0],
        );
        *out = v[0] as u16;
        rc
    }
    fn set_pause(
        &mut self,
        a: u32,
        b: u32,
        id: u16,
        flag: u32,
        n: u32,
        value: u16,
        out: &mut u16,
        mem: &mut [u8],
    ) -> u32 {
        let s = self.io.scratch;
        let (rc, v) = self.io.out(
            mem,
            self.io.method(mem, 4, 80),
            &[a, b, id as u32, flag, n, s, s + 4],
            [value as u32, *out as u32, 0],
        );
        *out = v[1] as u16;
        rc
    }
    fn get_position(&mut self, a: u32, b: u32, out: &mut u32, mem: &mut [u8]) -> u32 {
        let s = self.io.scratch;
        let (rc, v) = self.io.out(
            mem,
            self.io.method(mem, 24, 60),
            &[a, b, s, 0],
            [*out, 0, 0],
        );
        *out = v[0];
        rc
    }
    fn set_position(&mut self, a: u32, b: u32, pos: u32, mem: &mut [u8]) -> u32 {
        self.io
            .invoke(mem, self.io.method(mem, 24, 56), &[a, b, pos, 0])
    }
    fn alloc_text(&mut self, heap: u32, size: u32, mem: &mut [u8]) -> Option<u32> {
        nz(self.io.invoke(mem, 0x196394, &[heap, size]))
    }
    fn alloc_records(&mut self, heap: u32, size: u32, mem: &mut [u8]) -> Option<u32> {
        nz(self.io.invoke(mem, 0x1963b8, &[heap, 1, size]))
    }
    fn free_buf(&mut self, heap: u32, ptr: u32, mem: &mut [u8]) {
        self.io.invoke(mem, 0x196428, &[heap, ptr]);
    }
    fn log_no_memory(&mut self, log: u32, code: u32, flag: u32, mem: &mut [u8]) {
        
        self.io.invoke(mem, 0x173dc0, &[log, addr(self.io.host.rom(), 0x8f9334), code, flag]);
    }
    fn log_pause(&mut self, log: u32, level: u32, flag: u32, mem: &mut [u8]) {
        
        self.io
            .invoke(mem, 0x1738e8, &[log, addr(self.io.host.rom(), 0x8f9334), level, flag, addr(self.io.host.rom(), 0x8f93b4)]);
    }
}
/// Live prompt root. Scratch is 64 disjoint restored bytes for ABI outputs.
pub fn fn_1f3ea0_m(m: &mut Memory<'_>, a: [u32; 5], scratch: u32, h: &mut impl Host) -> u32 {
    prompt_token_m(
        &mut PromptTokenAdapter {
            io: PreprocessAdapter {
                host: h,
                processor: a[0],
                scratch,
            },
        },
        m.bytes,
        a[0],
        a[1],
        a[2],
        a[3],
        a[4],
    )
}

struct PromptDomainAdapter<'a, H> {
    io: PreprocessAdapter<'a, H>,
}
impl<H: Host> crate::frontend_part_b::PromptHost for PromptDomainAdapter<'_, H> {
    fn prompt(
        &mut self,
        op: crate::frontend_part_b::PromptOp,
        mem: &mut [u8],
        a: &[u32],
        _: &mut [u32; 9],
    ) -> [u32; 3] {
        use crate::frontend_part_b::PromptOp::*;
        let s = self.io.scratch;
        let rc = match op {
            Validate => frontend::validate_handle(a[0] != 0, a[1], a[2], a[3]),
            Session => {
                let (rc, v) = self.io.out(mem, 0x19611c, &[a[0], a[1], s], [0; 3]);
                return [rc, v[0], 0];
            }
            Clear => {
                let mut m = Memory {
                    base: 0,
                    bytes: mem,
                };
                destroy_items_m(&mut m, a[0], self.io.host)
            }
            Allocate => self.io.invoke(mem, 0x196394, a),
            Free => {
                self.io.invoke(mem, 0x196428, a);
                0
            }
            Count => crate::normalization::idx5(mem, a[0]),
            CreateNode => {
                let mut heap = DomainAdapter {
                    io: PreprocessAdapter {
                        host: self.io.host,
                        processor: 0,
                        scratch: s + 64,
                    },
                    trace_count: 0,
                    heap: a[0],
                };
                create_voice_node_m(
                    mem,
                    a[0],
                    a[1],
                    a[2],
                    a[3],
                    nz(a[4]),
                    a[5],
                    nz(a[6]),
                    &mut heap,
                    &mut |_, _, _, _| unreachable!("payload duplication uses reentrant allocator"),
                )
            }
            UpdateNode => {
                let mut heap = DomainAdapter {
                    io: PreprocessAdapter {
                        host: self.io.host,
                        processor: 0,
                        scratch: s + 64,
                    },
                    trace_count: 0,
                    heap: a[0],
                };
                update_voice_node_m(
                    mem,
                    a[0],
                    a[1],
                    a[2],
                    a[3],
                    a[4],
                    nz(a[5]),
                    a[6],
                    nz(a[7]),
                    &mut heap,
                )
            }
            DestroyNodes => {
                let mut heap = DomainAdapter {
                    io: PreprocessAdapter {
                        host: self.io.host,
                        processor: 0,
                        scratch: s + 64,
                    },
                    trace_count: 0,
                    heap: a[1],
                };
                release_node_chain_m(mem, nz(a[0]), nz(a[1]), &mut heap);
                0
            }
            Consult => crate::frontend_part_b::fn_200390(
                mem,
                a[0],
                a[1],
                a[2],
                a[3],
                a[4],
                [a[5], a[6]],
                a[7],
                self,
            ),
            Iter => {
                let (rc, v) = self.io.out(mem, 0x196794, &[a[0], a[1], a[2], s], [0; 3]);
                return [rc, v[0], 0];
            }
            Next => {
                let (rc, v) = self.io.out(mem, 0x196860, &[a[0], s, s + 4], [0; 3]);
                return [rc, v[0], v[1]];
            }
            Close => self.io.invoke(mem, 0x196830, a),
            Mark => {
                if a[0] != 0 {
                    mem[(a[0] + 12) as usize] = a[1] as u8;
                }
                0
            }
            Marked => {
                if a[0] == 0 {
                    0
                } else {
                    mem[(a[0] + 12) as usize] as u32
                }
            }
            Slot => {
                let (rc, v) = self.io.out(mem, 0x1970ac, &[a[0], a[1], s], [0; 3]);
                return [rc, v[0], 0];
            }
            Pop => self.io.invoke(mem, 0x1970dc, a),
            Enable => self.io.invoke(mem, 0x1fe0e4, a),
            Build => self.io.invoke(mem, 0x1fe418, a),
            Insert => self.io.invoke(mem, 0x276c50, a),
            Expand => self.io.invoke(mem, 0x27feb4, a),
            Write => {
                let mut m = Memory {
                    base: 0,
                    bytes: mem,
                };
                let saved = m.word(s);
                m.put_word(s, a[4]);
                let method = m.word(m.word(self.io.processor + 12) + 52);
                let rc = self.io.host.call_m(
                    method,
                    &[a[0], a[1], if a[2] == 0 { s } else { a[2] }, a[3]],
                    &mut m,
                );
                m.put_word(s, saved);
                rc
            }
        };
        [rc, 0, 0]
    }
}
/// Reentrant fn_19703c: reread descriptor before invoking each destructor.
pub fn destroy_items_m(m: &mut Memory<'_>, p: u32, h: &mut impl Host) -> u32 {
    if p == 0 {
        return 0;
    }
    if m.word(p + 12) != 0 {
        let data = m.word(p);
        let stride = m.half(p + 4) as u32;
        let count = m.word(p + 20);
        let ctx = m.word(p + 28);
        for i in 0..count {
            h.call_m(m.word(p + 12), &[data + i * stride, ctx], m);
        }
    }
    m.put_word(p + 20, 0);
    1
}

pub fn write_field_m(
    mem: &mut [u8],
    object: Option<u32>,
    tag: u32,
    record: u32,
    field: u32,
    count: u32,
    input: Option<u32>,
    written: u32,
    descs: &[u8],
    word_anchor: u32,
    host: &mut impl Host,
) -> u32 {
    if frontend::validate_handle(object.is_some(), tag, 0xb2c2, 0x38) >= 0x80000000 {
        return 0x89202008;
    }
    let db = object.unwrap();
    let table = mem_word(mem, db + 48);
    let data = if record < mem_word(mem, db + 32) {
        mem_word(mem, table.wrapping_add(record.wrapping_mul(4)))
    } else {
        0
    };
    if data == 0 {
        return 0x89203e11;
    }
    let ty = mem[data as usize + 2];
    let layout = match descs.get(ty as usize * 272..) {
        Some(layout) if layout.len() >= 272 => layout,
        _ => return 0x89203e0d,
    };
    let base = u32::from_le_bytes(layout[0..4].try_into().unwrap());
    let nfields = u16::from_le_bytes(layout[4..6].try_into().unwrap()) as u32;
    let states = u32::from_le_bytes(layout[8..12].try_into().unwrap());
    if ty == 0 || field >= nfields {
        return 0x89203e0d;
    }
    let field_off = 12 + field as usize * 20;
    let fdata = u32::from_le_bytes(layout[field_off..field_off + 4].try_into().unwrap());
    let fcount = u32::from_le_bytes(layout[field_off + 4..field_off + 8].try_into().unwrap());
    let flength = u16::from_le_bytes(layout[field_off + 8..field_off + 10].try_into().unwrap());
    let fsize = u16::from_le_bytes(layout[field_off + 10..field_off + 12].try_into().unwrap());
    let dest = data.wrapping_add(fdata.wrapping_sub(base));
    let cnt = if fcount != 0 {
        Some(data.wrapping_add(fcount.wrapping_sub(base)))
    } else {
        None
    };
    mem[data
        .wrapping_add(states.wrapping_sub(base))
        .wrapping_add(field) as usize] = 1;
    let src = input.unwrap_or(0);
    if flength == 0 {
        let bytes = (fsize as u32).wrapping_mul(count);
        let p = crate::db_live::arena_alloc_m(
            &mut DbAdapter(host),
            &mut Memory {
                base: 0,
                bytes: mem,
            },
            db + 4,
            bytes,
        );
        mem_store(mem, dest, p);
        if p == 0 {
            return 0x8920200a;
        }
        frontend::move_bytes(mem, p, src, bytes);
        if fdata == word_anchor {
            let mut i = 0u32;
            while i < count {
                let entry = src.wrapping_add(i.wrapping_mul(24));
                let etype = mem_half(mem, entry);
                if matches!(
                    etype,
                    6 | 12 | 21 | 50 | 36 | 39 | 37 | 34 | 22 | 40 | 26 | 29
                ) {
                    let str_ptr = mem_word(mem, entry + 20);
                    let n = if str_ptr == 0 {
                        0
                    } else {
                        crate::normalization::u16_len(mem, str_ptr)
                    };
                    let d = crate::db_live::arena_alloc_m(
                        &mut DbAdapter(host),
                        &mut Memory {
                            base: 0,
                            bytes: mem,
                        },
                        db + 4,
                        n.wrapping_mul(2).wrapping_add(2),
                    );
                    if d == 0 {
                        return 0x8920200a;
                    }
                    if n == 0 {
                        mem_store16(mem, d, 0);
                    } else {
                        crate::normalization::u16_copy(mem, d, str_ptr);
                    }
                    mem_store(mem, p.wrapping_add(i.wrapping_mul(24)).wrapping_add(20), d);
                }
                i = i.wrapping_add(1);
            }
        }
        if let Some(cnt) = cnt {
            mem_store16(mem, cnt, count as u16);
        }
        mem_store16(mem, written, count as u16);
    } else {
        let length = if flength as u32 > count {
            count
        } else {
            flength as u32
        };
        frontend::move_bytes(mem, dest, src, (fsize as u32).wrapping_mul(length));
        if let Some(cnt) = cnt {
            mem_store16(mem, cnt, length as u16);
        }
        mem_store16(mem, written, length as u16);
    }
    mem[db as usize + 52] = 1;
    0
}

struct DbAdapter<'a, H>(&'a mut H);
impl<H: Host> crate::db_live::PoolHooksM for DbAdapter<'_, H> {
    fn log_pool_m(&mut self, p: u32, n: u32, m: &mut Memory<'_>) {
        
        self.0.call_m(
            0x1738e8,
            &[
                m.word(m.word(p) + 16),
                addr(self.0.rom(), 0x8d7b56),
                5,
                0,
                addr(self.0.rom(), 0x8dc928),
                m.word(p + 4),
                addr(self.0.rom(), 0x180f78),
                m.word(p + 24),
                m.word(p + 20),
                m.word(p + 12),
                m.word(p + 16),
                n,
            ],
            m,
        );
    }
}
impl<H: Host> crate::db_live::DbHooksM for DbAdapter<'_, H> {
    fn call_m(&mut self, f: u32, a: &[u32], m: &mut Memory<'_>) -> u32 {
        self.0.call_m(f, a, m)
    }
}

pub(crate) fn speech_literals(m: &Memory<'_>, rom: Rom) -> crate::speech_part_2::SpeechLiterals {
    crate::speech_part_2::SpeechLiterals {
        text: [
            0x24ed34, 0x24ed18, 0x24ed20, 0x193c10, 0x24ed4c, 0x24ed64, 0x24ed7c, 0x24eda4,
            addr(rom, 0x8f8f44), 0x1ee188, 0x1ee6bc, addr(rom, 0x8f8fd8), 0x24c858, 0x24c880, 0x24c8b8, addr(rom, 0x914a38),
        ],
        end: 0x24c9d0,
        errors: [0x24ea48, 0x24ee48, 0x1ec3f4, 0x250d48, 0x207740],
    }
}
pub(crate) fn domain_literals(m: &Memory<'_>, rom: Rom) -> crate::speech_part_2::DomainOpenLiterals {
    crate::speech_part_2::DomainOpenLiterals {
        formats: [
            0x24bf5c, 0x24bf8c, 0x24bfd4, 0x24c008, 0x24c390, 0x24c3b0, 0x14f410, 0x24c3d8,
            addr(rom, 0x9149ec), 0x24c40c, 0x24c448, 0,
        ],
        trace: 0x24bf84,
        remapped: 0x24bfbc,
        empty: 0x100006,
        virtual_domain: 0x24bfb0,
    }
}

impl crate::speech_part_2::Host for LiveHost<'_> {
    fn call(&mut self, m: &mut Memory<'_>, f: u32, a: &[u32]) -> u32 {
        self.call_m(f, a, m)
    }
}
impl crate::boot_part_b::FormatHost for LiveHost<'_> {}
impl crate::log_live::LogMethodsM for LiveHost<'_> {
    fn log_m(
        &mut self,
        f: u32,
        p: u32,
        category: u32,
        level: u32,
        arg: u32,
        fmt: u32,
        va: u32,
        m: &mut Memory<'_>,
    ) -> u32 {
        if f == 0x1738e8 {
            let mut boot = crate::engine::SpeechBootHost {
                platform: self.platform,
                fault: self.fault,
            };
            let rc = boot.invoke(m, f, &[p, category, level, arg, fmt, va]);
            self.fault = boot.fault;
            rc
        } else {
            self.call_m(f, &[p, category, level, arg, fmt, va], m)
        }
    }
}

fn database_error(
    m: &mut Memory<'_>,
    f: u32,
    a: &[u32],
    record: u32,
    kind: u8,
    rc: u32,
    h: &mut impl Host,
) {
    let log = m.word(m.word(a[0]) + 16);
    
    let fmt = addr(h.rom(), 0x8d7b56);
    if record == 0 {
        h.call_m(
            0x173dc0,
            &[log, fmt, 0x4e34, addr(h.rom(), 0x14e438), addr(h.rom(), 0x15baa0), a[2]],
            m,
        );
        return;
    }
    let method = match f {
        0x15c7d8 => addr(h.rom(), 0x15c988),
        0x15d594 => addr(h.rom(), 0x15d6ac),
        0x15d378 => addr(h.rom(), 0x15d574),
        _ => addr(h.rom(), 0x15d34c),
    };
    if f == 0x15c7d8 {
        h.call_m(
            0x173dc0,
            &[
                log,
                fmt,
                0x4e2c,
                addr(h.rom(), 0x15c498),
                addr(h.rom(), 0x15b9ec),
                method,
                addr(h.rom(), 0x15b9f4),
                kind as u32,
                addr(h.rom(), 0x15baa0),
                a[2],
            ],
            m,
        );
    } else if rc == 0x89203e0d || (rc == 0x89203e11 && f == 0x15d0c4) {
        h.call_m(
            0x173dc0,
            &[
                log,
                fmt,
                if rc == 0x89203e0d { 0x4e30 } else { 0x4e34 },
                addr(h.rom(), 0x15c714),
                addr(h.rom(), 0x15b9ec),
                method,
                addr(h.rom(), 0x15b9f4),
                kind as u32,
                addr(h.rom(), 0x15baa0),
                a[2],
                addr(h.rom(), 0x15d084),
                a[3],
            ],
            m,
        );
    } else {
        h.call_m(
            0x173dc0,
            &[
                log,
                fmt,
                if rc == 0x89203e0e { 0x4e31 } else { 0x4e34 },
                addr(h.rom(), 0x15c718),
                addr(h.rom(), 0x15b9f4),
                kind as u32,
                addr(h.rom(), 0x15baa0),
                a[2],
                addr(h.rom(), 0x15d364),
                a[3],
            ],
            m,
        );
    }
}

/// The lane c host view of the live host. The lane c ports take their own
/// host trait; this forwards the guest calls and the thread id.
struct ComponentCHost<'a, 'b> {
    host: &'a mut LiveHost<'b>,
    rom: Rom,
}
impl crate::component_live_c::Host for ComponentCHost<'_, '_> {
    fn call_m(&mut self, entry: u32, args: &[u32], m: &mut Memory<'_>) -> u32 {
        self.host.call_m_with(entry, args, m, self.rom)
    }
    fn rom(&self) -> Rom {
        self.rom
    }
    fn thread_id_m(&mut self) -> u32 {
        self.host.platform.thread_id()
    }
}

/// fn_17c7d0 (EU fn_17c930 / KR fn_17ca90). `D_8dc30c[c & 0xff] & 4`: the
/// character class bit table. The table is ROM data at 0x8dc30c, so the
/// caller passes it in: the native model places it in a fixed section, the
/// ARM reads it from the ROM image and the regional ROMs move it.
pub fn fn_17c7d0_m(table: &[u8], c: u32) -> u32 {
    let at = (c & 0xff) as usize * 2;
    match table.get(at..at + 2) {
        Some(b) => u16::from_le_bytes([b[0], b[1]]) as u32 & 4,
        None => 0,
    }
}

/// The length of a guest NUL terminated string, or `None` when it is not
/// terminated inside the arena.
fn guest_str_len(m: &Memory<'_>, p: u32) -> Option<u32> {
    let mut i = 0u32;
    loop {
        match guest_byte(m, p.checked_add(i)?) {
            Some(0) => return Some(i),
            Some(_) => i = i.checked_add(1)?,
            None => return None,
        }
    }
}

/// fn_15bf74 (LD_createChildRecord). Creates a child record of `child_type`
/// under the parent record `index` and links it into the parent's child list.
///
/// The ROM's `LingRecordNode` layout: id@0 u16, type@2 u8, parent@4,
/// next@8, prev@12, children[0]@16, children[1]@20. The database layout:
/// owner@0, count@32, records@48, dirty@52. `D_8d7b50[child_type]` is the
/// parent type the child type expects.
///
/// `args` is the caller's block: object, tag, child type, parent index and
/// the out pointer that receives the new record id. `scratch` is a writable
/// word inside the arena that receives the new record pointer.
/// `parent_types` is `D_8d7b50`, the ROM data that maps a child type to the
/// parent type it expects; the caller passes it in because the regional
/// ROMs move it.
pub fn fn_15bf74_m(
    m: &mut Memory<'_>,
    args: [u32; 5],
    scratch: u32,
    parent_types: &[u8],
    h: &mut impl Host,
) -> u32 {
    let (object, tag, child_type, index, out) = (args[0], args[1], args[2], args[3], args[4]);
    if frontend::validate_handle(object != 0, tag, 0xb2c2, 0x38) >= 0x80000000 {
        return 0x89202008;
    }
    let db = object;
    if !guest_range(m, db, 56) {
        return 0x89202008;
    }
    let owner = match guest_word(m, db) {
        Some(v) => v,
        None => return 0x89202008,
    };
    let log = match guest_word(m, owner.wrapping_add(16)) {
        Some(v) => v,
        None => return 0x89202008,
    };
    
    let count = match guest_word(m, db.wrapping_add(32)) {
        Some(v) => v,
        None => return 0x89202008,
    };
    let table = match guest_word(m, db.wrapping_add(48)) {
        Some(v) => v,
        None => return 0x89202008,
    };
    let parent = if index < count {
        match guest_word(m, table.wrapping_add(index.wrapping_mul(4))) {
            Some(v) => v,
            None => return 0x89202008,
        }
    } else {
        0
    };
    if parent == 0 {
        h.call_m(
            0x173dc0,
            &[log, addr(h.rom(), 0x8d7b56), 0x4e34, addr(h.rom(), 0x15c1a0), addr(h.rom(), 0x15c1b0), index],
            m,
        );
        return 0x89203e11;
    }
    let parent_type = match guest_byte(m, parent.wrapping_add(2)) {
        Some(v) => v,
        None => return 0x89202008,
    };
    if child_type < 2 {
        h.call_m(
            0x173dc0,
            &[log, addr(h.rom(), 0x8d7b56), 0x4e25, addr(h.rom(), 0x15c1a0), addr(h.rom(), 0x15c194), child_type],
            m,
        );
        return 0x89203e02;
    }
    let want = match parent_types.get(child_type as usize) {
        Some(v) => *v,
        None => return 0x89203e03,
    };
    if want != parent_type {
        h.call_m(
            0x173dc0,
            &[
                log,
                addr(h.rom(), 0x8d7b56),
                0x4e26,
                addr(h.rom(), 0x15c1c8),
                addr(h.rom(), 0x15c194),
                child_type,
                addr(h.rom(), 0x15c1bc),
                parent_type as u32,
            ],
            m,
        );
        return 0x89203e03;
    }
    if !guest_range(m, scratch, 4) {
        return 0x89202008;
    }
    let rom = h.rom();
    let rc = {
        let mut adapter = DbAdapter(h);
        crate::db_live::create_record_m(
            &mut adapter,
            m,
            db,
            child_type,
            scratch,
            addr(rom, 0x8d8274),
            addr(rom, 0x8d7ca0),
        )
    };
    if rc >= 0x80000000 {
        return rc;
    }
    let node = match guest_word(m, scratch) {
        Some(v) => v,
        None => return 0x89202008,
    };
    if !guest_range(m, node, 24) {
        return 0x89202008;
    }
    // The ROM clears the first child slot and the previous link before the
    // node is linked into the list (0x15c09c/0x15c0a0/0x15c0a4).
    m.put_word(node.wrapping_add(4), parent);
    m.put_word(node.wrapping_add(16), 0);
    m.put_word(node.wrapping_add(12), 0);
    let mut found = false;
    let mut i = 0u32;
    while i < 2 && !found {
        let slot = parent.wrapping_add(16).wrapping_add(i.wrapping_mul(4));
        let child = match guest_word(m, slot) {
            Some(v) => v,
            None => return 0x89202008,
        };
        if child != 0 {
            let kind = match guest_byte(m, child.wrapping_add(2)) {
                Some(v) => v,
                None => return 0x89202008,
            };
            if kind == child_type as u8 {
                found = true;
                m.put_word(node.wrapping_add(8), child);
                m.put_word(child.wrapping_add(12), node);
                m.put_word(slot, node);
            }
        }
        i = i.wrapping_add(1);
    }
    if !found {
        let mut j = 0u32;
        while j < 2 {
            let child =
                match guest_word(m, parent.wrapping_add(16).wrapping_add(j.wrapping_mul(4))) {
                    Some(v) => v,
                    None => return 0x89202008,
                };
            if child == 0 {
                break;
            }
            j = j.wrapping_add(1);
        }
        if j < 2 {
            m.put_word(
                parent.wrapping_add(16).wrapping_add(j.wrapping_mul(4)),
                node,
            );
            m.put_word(node.wrapping_add(8), 0);
        } else {
            h.call_m(
                0x173dc0,
                &[
                    log,
                    addr(h.rom(), 0x8d7b56),
                    0x4e3e,
                    addr(h.rom(), 0x15c1c8),
                    addr(h.rom(), 0x15c194),
                    child_type,
                    addr(h.rom(), 0x15c1bc),
                    parent_type as u32,
                ],
                m,
            );
            return 0x89203e1b;
        }
    }
    let id = match guest_half(m, node) {
        Some(v) => v,
        None => return 0x89202008,
    };
    if !guest_range(m, out, 2) {
        return 0x89202008;
    }
    m.put_half(out, id);
    m.put_byte(db.wrapping_add(52), 1);
    rc
}

// ===== fn_30209c / fn_30253c: the RVCT runtime helpers ====================

/// fn_30209c. The hand written RVCT signed divide: the quotient is the low
/// word of the result and the remainder the high word, the way the callers
/// read the packed 64-bit value.
///
/// The ARM's zero-divisor path (build/target/fn_30209c.s, the tail at
/// 0x3022b0) calls the divide-by-zero handler 0x1a2ea4 and then returns the
/// dividend in the remainder word with a zero quotient.
pub fn fn_30209c_m(dividend: i32, divisor: i32) -> u64 {
    let (quotient, remainder) = if divisor == 0 {
        (0, dividend)
    } else {
        (
            dividend.wrapping_div(divisor),
            dividend.wrapping_rem(divisor),
        )
    };
    ((remainder as u32 as u64) << 32) | (quotient as u32 as u64)
}

/// fn_30253c. The hand written ARM libc memcpy front end: a forward byte
/// copy that returns the end of the copy. Every store in the target
/// (build/target/fn_30253c.s) uses the post-increment form, so the ARM
/// leaves r0 at dst+n.
pub fn fn_30253c_m(m: &mut Memory<'_>, dst: u32, src: u32, n: u32) -> u32 {
    if !guest_range(m, src, n) || !guest_range(m, dst, n) {
        return 0;
    }
    let at = (src - m.base) as usize;
    let to = (dst - m.base) as usize;
    let len = n as usize;
    let mut i = 0usize;
    while i < len {
        m.bytes[to + i] = m.bytes[at + i];
        i += 1;
    }
    dst.wrapping_add(n)
}

// ===== fn_15c1e4 / fn_15c2f0 / fn_15d83c / fn_15da64: the LD_* record ops ==

/// The record store header of fn_15c1e4: owner@0, count@32, records@48,
/// dirty@52. The `LingRecordNode` layout is id@0 u16, type@2 u8, data@4,
/// next@8, prev@12, children[0]@16, children[1]@20.
fn rs_count(m: &Memory<'_>, db: u32) -> Option<u32> {
    guest_word(m, db.wrapping_add(32))
}

/// The record at `index` of the store, or `None` when the table is unreadable.
fn rs_record(m: &Memory<'_>, db: u32, index: u32) -> Option<u32> {
    let table = guest_word(m, db.wrapping_add(48))?;
    guest_word(m, table.wrapping_add(index.wrapping_mul(4)))
}

/// The logger of the store's owner.
fn rs_log(m: &Memory<'_>, db: u32) -> Option<u32> {
    let owner = guest_word(m, db)?;
    guest_word(m, owner.wrapping_add(16))
}

/// fn_15c1e4 (LD_createNext). Creates a record of the type of the record at
/// `index` and links it directly after that record.
///
/// `args` is the caller's block: database, tag, index and the output word.
/// `scratch` addresses four writable bytes for the new record pointer.
pub fn fn_15c1e4_m(m: &mut Memory<'_>, args: [u32; 4], scratch: u32, h: &mut impl Host) -> u32 {
    let (db, tag, index, out) = (args[0], args[1], args[2], args[3]);
    if frontend::validate_handle(db != 0, tag, 0xb2c2, 0x38) >= 0x80000000 {
        return 0x89202008;
    }
    let log = match rs_log(m, db) {
        Some(v) => v,
        None => return 0x89202008,
    };
    
    let count = match rs_count(m, db) {
        Some(v) => v,
        None => return 0x89202008,
    };
    if index >= count {
        h.call_m(0x173dc0, &[log, addr(h.rom(), 0x8d7b56), 0x4e34, addr(h.rom(), 0x15c1a0), addr(h.rom(), 0x15c1b0), index], m);
        return 0x89203e11;
    }
    let rec = match rs_record(m, db, index) {
        Some(v) => v,
        None => return 0x89202008,
    };
    if rec == 0 {
        h.call_m(0x173dc0, &[log, addr(h.rom(), 0x8d7b56), 0x4e34, addr(h.rom(), 0x15c1a0), addr(h.rom(), 0x15c1b0), index], m);
        return 0x89203e11;
    }
    let kind = match guest_byte(m, rec.wrapping_add(2)) {
        Some(v) => v,
        None => return 0x89202008,
    };
    if kind == 1 {
        h.call_m(0x173dc0, &[log, addr(h.rom(), 0x8d7b56), 0x4e27, 0], m);
        return 0x89203e04;
    }
    if !guest_range(m, scratch, 4) {
        return 0x89202008;
    }
    let rom = h.rom();
    let rc = {
        let mut adapter = DbAdapter(h);
        crate::db_live::create_record_m(
            &mut adapter,
            m,
            db,
            u32::from(kind),
            scratch,
            addr(rom, 0x8d8274),
            addr(rom, 0x8d7ca0),
        )
    };
    if rc >= 0x80000000 {
        return rc;
    }
    let node = match guest_word(m, scratch) {
        Some(v) => v,
        None => return 0x89202008,
    };
    if !guest_range(m, node, 24) || !guest_range(m, rec, 20) {
        return 0x89202008;
    }
    // The ROM links the new node between the record and the record's next
    // (0x15c25c..0x15c280): data, prev, next, then the next's prev.
    let data = match guest_word(m, rec.wrapping_add(4)) {
        Some(v) => v,
        None => return 0x89202008,
    };
    let next = match guest_word(m, rec.wrapping_add(8)) {
        Some(v) => v,
        None => return 0x89202008,
    };
    m.put_word(node.wrapping_add(4), data);
    m.put_word(node.wrapping_add(12), rec);
    m.put_word(node.wrapping_add(8), next);
    m.put_word(rec.wrapping_add(8), node);
    m.put_word(node.wrapping_add(16), 0);
    if next != 0 {
        if !guest_range(m, next, 16) {
            return 0x89202008;
        }
        m.put_word(next.wrapping_add(12), node);
    }
    let id = match guest_half(m, node) {
        Some(v) => v,
        None => return 0x89202008,
    };
    if !guest_range(m, out, 2) {
        return 0x89202008;
    }
    m.put_half(out, id);
    m.put_byte(db.wrapping_add(52), 1);
    rc
}

/// fn_15c2f0 (LD_createPrev). Creates the record that precedes the record at
/// `index`: the record the store points at with its `ref2` word, or the
/// child record its `ref` word names.
///
/// `args` is the caller's block: database, tag, index and the output word.
pub fn fn_15c2f0_m(m: &mut Memory<'_>, args: [u32; 4], scratch: u32, h: &mut impl Host) -> u32 {
    let (db, tag, index, out) = (args[0], args[1], args[2], args[3]);
    if frontend::validate_handle(db != 0, tag, 0xb2c2, 0x38) >= 0x80000000 {
        return 0x89202008;
    }
    let log = match rs_log(m, db) {
        Some(v) => v,
        None => return 0x89202008,
    };
    
    let count = match rs_count(m, db) {
        Some(v) => v,
        None => return 0x89202008,
    };
    if index >= count {
        h.call_m(0x173dc0, &[log, addr(h.rom(), 0x8d7b56), 0x4e34, addr(h.rom(), 0x15c1a0), addr(h.rom(), 0x15c1b0), index], m);
        return 0x89203e11;
    }
    let rec = match rs_record(m, db, index) {
        Some(v) => v,
        None => return 0x89202008,
    };
    if rec == 0 {
        h.call_m(0x173dc0, &[log, addr(h.rom(), 0x8d7b56), 0x4e34, addr(h.rom(), 0x15c1a0), addr(h.rom(), 0x15c1b0), index], m);
        return 0x89203e11;
    }
    let kind = match guest_byte(m, rec.wrapping_add(2)) {
        Some(v) => v,
        None => return 0x89202008,
    };
    if kind == 1 {
        h.call_m(0x173dc0, &[log, addr(h.rom(), 0x8d7b56), 0x4e28, 0], m);
        return 0x89203e05;
    }
    if !guest_range(m, scratch, 4) {
        return 0x89202008;
    }
    let ref2 = match guest_word(m, rec.wrapping_add(12)) {
        Some(v) => v,
        None => return 0x89202008,
    };
    let rc;
    if ref2 != 0 {
        let id = match guest_half(m, ref2) {
            Some(v) => v,
            None => return 0x89202008,
        };
        rc = fn_15c1e4_m(m, [db, tag, u32::from(id), scratch], scratch.wrapping_add(4), h);
    } else {
        let reference = match guest_word(m, rec.wrapping_add(4)) {
            Some(v) => v,
            None => return 0x89202008,
        };
        if reference == 0 {
            h.call_m(
                0x173dc0,
                &[
                    log,
                    addr(h.rom(), 0x8d7b56),
                    0x4e29,
                    addr(h.rom(), 0x15c498),
                    addr(h.rom(), 0x15c490),
                    addr(h.rom(), 0x15c480),
                    addr(h.rom(), 0x15c474),
                    u32::from(kind),
                    addr(h.rom(), 0x15c1b0),
                    index,
                ],
                m,
            );
            return 0x89203e06;
        }
        let id = match guest_half(m, reference) {
            Some(v) => v,
            None => return 0x89202008,
        };
        let mut parent_types = [0u8; 8];
        if let Some(s) = m.bytes.get(addr(h.rom(), 0x8d7b50) as usize..addr(h.rom(), 0x8d7b50) as usize + 8) {
            parent_types.copy_from_slice(s);
        }
        rc = fn_15bf74_m(
            m,
            [db, tag, u32::from(kind), u32::from(id), scratch],
            scratch.wrapping_add(4),
            &parent_types,
            h,
        );
    }
    if rc >= 0x80000000 {
        return rc;
    }
    let result = match guest_half(m, scratch) {
        Some(v) => v,
        None => return 0x89202008,
    };
    if !guest_range(m, out, 2) {
        return 0x89202008;
    }
    m.put_half(out, result);
    m.put_byte(db.wrapping_add(52), 1);
    rc
}

/// fn_15d83c. Removes the record at `index` and, when `index` is not the
/// root, the records that follow it. The record's children go first.
fn fn_15d83c_run(m: &mut Memory<'_>, db: u32, index: u32, root: u32, h: &mut impl Host) -> u32 {
    let log = match rs_log(m, db) {
        Some(v) => v,
        None => return 0x89202008,
    };
    
    let count = match rs_count(m, db) {
        Some(v) => v,
        None => return 0x89202008,
    };
    if index >= count {
        h.call_m(0x173dc0, &[log, addr(h.rom(), 0x8d7b56), 0x4e34, addr(h.rom(), 0x15c1a0), addr(h.rom(), 0x15d82c), index], m);
        return 0x89203e11;
    }
    let rec = match rs_record(m, db, index) {
        Some(v) => v,
        None => return 0x89202008,
    };
    if rec == 0 {
        h.call_m(0x173dc0, &[log, addr(h.rom(), 0x8d7b56), 0x4e34, addr(h.rom(), 0x15c1a0), addr(h.rom(), 0x15d82c), index], m);
        return 0x89203e11;
    }
    if !guest_range(m, rec, 24) {
        return 0x89202008;
    }
    let mut rc = 0u32;
    let mut i = 0u32;
    while i < 2 {
        let child = match guest_word(m, rec.wrapping_add(16).wrapping_add(i.wrapping_mul(4))) {
            Some(v) => v,
            None => return 0x89202008,
        };
        if child == 0 {
            break;
        }
        let id = match guest_half(m, child) {
            Some(v) => v,
            None => return 0x89202008,
        };
        rc = fn_15d83c_run(m, db, u32::from(id), root, h);
        if rc != 0 {
            return rc;
        }
        i = i.wrapping_add(1);
    }
    let next = match guest_word(m, rec.wrapping_add(8)) {
        Some(v) => v,
        None => return 0x89202008,
    };
    if index != root {
        if next != 0 {
            let id = match guest_half(m, next) {
                Some(v) => v,
                None => return 0x89202008,
            };
            rc = fn_15d83c_run(m, db, u32::from(id), root, h);
            if rc != 0 {
                return rc;
            }
        }
    } else {
        let prev = match guest_word(m, rec.wrapping_add(12)) {
            Some(v) => v,
            None => return 0x89202008,
        };
        if prev != 0 {
            if !guest_range(m, prev, 16) {
                return 0x89202008;
            }
            if next == 0 {
                m.put_word(prev.wrapping_add(8), 0);
            } else {
                if !guest_range(m, next, 16) {
                    return 0x89202008;
                }
                m.put_word(next.wrapping_add(12), prev);
                m.put_word(prev.wrapping_add(8), next);
            }
        } else {
            let parent = match guest_word(m, rec.wrapping_add(4)) {
                Some(v) => v,
                None => return 0x89202008,
            };
            if parent == 0 {
                return 0x89202008;
            }
            if !guest_range(m, parent, 24) {
                return 0x89202008;
            }
            let mut slot = 0u32;
            let mut found = false;
            let mut j = 0u32;
            while j < 2 && !found {
                let child = match guest_word(m, parent.wrapping_add(16).wrapping_add(j.wrapping_mul(4)))
                {
                    Some(v) => v,
                    None => return 0x89202008,
                };
                if child == 0 {
                    break;
                }
                if child == rec {
                    slot = j;
                    found = true;
                }
                j = j.wrapping_add(1);
            }
            if !found {
                h.call_m(
                    0x173dc0,
                    &[
                        log,
                        addr(h.rom(), 0x8d7b56),
                        0x4e32,
                        addr(h.rom(), 0x15c1a0),
                        addr(h.rom(), 0x15d834),
                        u32::from(kind_of(m, rec)),
                        index,
                    ],
                    m,
                );
                return 0x89203e0f;
            }
            let slot_at = parent.wrapping_add(16).wrapping_add(slot.wrapping_mul(4));
            if next != 0 {
                m.put_word(slot_at, next);
                m.put_word(next.wrapping_add(12), 0);
            } else {
                m.put_word(slot_at, 0);
                let mut k = slot.wrapping_add(1);
                while k < 2 {
                    let child =
                        match guest_word(m, parent.wrapping_add(16).wrapping_add(k.wrapping_mul(4)))
                        {
                            Some(v) => v,
                            None => return 0x89202008,
                        };
                    if child == 0 {
                        break;
                    }
                    m.put_word(parent.wrapping_add(16).wrapping_add((k - 1).wrapping_mul(4)), child);
                    m.put_word(parent.wrapping_add(16).wrapping_add(k.wrapping_mul(4)), 0);
                    k = k.wrapping_add(1);
                }
            }
        }
    }
    let table = match guest_word(m, db.wrapping_add(48)) {
        Some(v) => v,
        None => return 0x89202008,
    };
    if !guest_range(m, table.wrapping_add(index.wrapping_mul(4)), 4) {
        return 0x89202008;
    }
    m.put_word(table.wrapping_add(index.wrapping_mul(4)), 0);
    rc
}

/// The type byte of a record.
fn kind_of(m: &Memory<'_>, rec: u32) -> u8 {
    guest_byte(m, rec.wrapping_add(2)).unwrap_or(0)
}

/// fn_15d83c: remove the record at `index` and the records after it.
pub fn fn_15d83c_m(m: &mut Memory<'_>, args: [u32; 3], h: &mut impl Host) -> u32 {
    fn_15d83c_run(m, args[0], args[1], args[2], h)
}

/// fn_15da64: mark the store dirty and remove the record at `index`.
pub fn fn_15da64_m(m: &mut Memory<'_>, args: [u32; 3], h: &mut impl Host) -> u32 {
    let (db, tag, index) = (args[0], args[1], args[2]);
    if frontend::validate_handle(db != 0, tag, 0xb2c2, 0x38) >= 0x80000000 {
        return 0x89202008;
    }
    if !guest_range(m, db, 56) {
        return 0x89202008;
    }
    m.put_byte(db.wrapping_add(52), 1);
    fn_15d83c_run(m, db, index, index, h)
}

// ===== fn_1ed954: the fe_dctlkp G2P convert entry ========================

/// The `fn_1ed954` log format of the "Entering" trace.
const G2P_ENTERING: u32 = 0x1eda60;
/// The `fn_1ed954` log format of the "Leaving" trace.
const G2P_LEAVING: u32 = 0x1eda90;

/// fn_1ed954 (fe_dctlkp_G2PConvertToCanonicalWord).
///
/// The exact ARM call is `fn_1ed954(ctx, tag, text, buffer, capacity)`:
/// r0 = the `fe_dctlkp` object, r1 = the handle tag (0xf37f), r2 = the
/// request text, r3 = the text buffer the replacement pass rewrites and
/// the fifth argument (the stack slot at sp+0x20) = the buffer capacity.
///
/// The object holds the character class at +0x64, the mode half word at
/// +0x9c, the canonical name at +0x19e, the refresh flag at +0x29e and
/// the two replacement tables at +0x2a0 and +0x90.
pub fn fn_1ed954_m(
    m: &mut Memory<'_>,
    h: &mut impl Host,
    ctx: u32,
    tag: u32,
    text: u32,
    buffer: u32,
    capacity: u32,
) -> u32 {
    if frontend::validate_handle(ctx != 0, tag, 0xf37f, 0x2a4) >= 0x80000000 {
        return 0x89802008;
    }
    let owner = match guest_word(m, ctx) {
        Some(v) => v,
        None => return 0x89802008,
    };
    let logger = guest_word(m, owner.wrapping_add(0x10)).unwrap_or(0);
    h.call_m(
        0x1738e8,
        &[logger, addr(h.rom(), 0x8f8f44), 4, 0, G2P_ENTERING],
        m,
    );
    if buffer == 0 {
        h.call_m(0x173dc0, &[logger, addr(h.rom(), 0x8f8f44), 0xa029, 0], m);
        return 0x89802007;
    }
    let flag = guest_byte(m, ctx.wrapping_add(0x29e)).unwrap_or(0);
    if flag != 0 {
        let name = ctx.wrapping_add(0x19e);
        if !same_guest_string(m, text, name) {
            let rc = h.call_m(0x1eb978, &[ctx, tag, text], m);
            if rc >= 0x80000000 {
                return rc;
            }
            copy_guest_string(m, name, text);
        }
    }
    let mut replacements = guest_word(m, ctx.wrapping_add(0x2a0)).unwrap_or(0);
    if replacements == 0 {
        replacements = guest_word(m, ctx.wrapping_add(0x90)).unwrap_or(0);
    }
    let rc = crate::component_live_b3::fn_1ed520_m(m, h, ctx, replacements, buffer, capacity);
    h.call_m(
        0x1738e8,
        &[logger, addr(h.rom(), 0x8f8f44), 4, 0, G2P_LEAVING],
        m,
    );
    rc
}

/// Whether two guest strings hold the same bytes.
fn same_guest_string(m: &Memory<'_>, a: u32, b: u32) -> bool {
    let mut at = 0u32;
    loop {
        let x = guest_byte(m, a.wrapping_add(at));
        let y = guest_byte(m, b.wrapping_add(at));
        match (x, y) {
            (Some(x), Some(y)) => {
                if x != y {
                    return false;
                }
                if x == 0 {
                    return true;
                }
            }
            _ => return false,
        }
        at = at.wrapping_add(1);
    }
}

/// Copy a guest string, terminator included.
fn copy_guest_string(m: &mut Memory<'_>, dst: u32, src: u32) {
    let mut at = 0u32;
    loop {
        let byte = match guest_byte(m, src.wrapping_add(at)) {
            Some(v) => v,
            None => return,
        };
        if !guest_range(m, dst.wrapping_add(at), 1) {
            return;
        }
        m.put_byte(dst.wrapping_add(at), byte);
        if byte == 0 {
            return;
        }
        at = at.wrapping_add(1);
    }
}

// ===== fn_26c1c0: the fe_text morphology trie ============================

/// The `TrieReader` of fn_26c1c0: the byte position at +0, the bit index at
/// +4, the data page at +8, the tables at +12 and the two broker handles at
/// +16 and +20.
const TRIE_POS: u32 = 0x00;
const TRIE_BIT: u32 = 0x04;
const TRIE_DATA: u32 = 0x08;
const TRIE_TABLES: u32 = 0x0c;
const TRIE_HANDLE0: u32 = 0x10;
const TRIE_HANDLE1: u32 = 0x14;

/// The `TrieTables` of fn_26c1c0.
const TRIE_HEADER: u32 = 0x00;
const TRIE_ALPHABET: u32 = 0x04;
const TRIE_INITIAL_CHARS: u32 = 0x0c;
const TRIE_INITIAL_OFFSETS: u32 = 0x10;
const TRIE_INITIAL_INDICES: u32 = 0x14;
const TRIE_COUNT_TREE: u32 = 0x18;
const TRIE_CHAR_TREE: u32 = 0x1c;
const TRIE_DELTA_TREE: u32 = 0x20;
const TRIE_COUNTS: u32 = 0x2c;
const TRIE_CHARS: u32 = 0x30;
const TRIE_DELTAS: u32 = 0x34;
const TRIE_DATA_OFFSET: u32 = 0x40;

/// The `TrieHeader` of fn_26c1c0.
const TRIE_FIRST_CHAR: u32 = 0x04;
const TRIE_ALIGN_MASK: u32 = 0x0f;
const TRIE_TERMINATOR: u32 = 0x10;
const TRIE_INITIAL_BIT: u32 = 0x11;
const TRIE_INLINE_LIMIT: u32 = 0x12;
const TRIE_DIRECT_COUNT: u32 = 0x18;
const TRIE_COUNT_ROOT: u32 = 0x1a;
const TRIE_CHAR_ROOT: u32 = 0x1c;
const TRIE_DELTA_ROOT: u32 = 0x1e;
const TRIE_INITIAL_COUNT: u32 = 0x2e;
const TRIE_PAGE_SIZE: u32 = 0x32;
const TRIE_INITIAL_POS: u32 = 0x38;

/// The status the trie port uses when the caller's arena is not readable.
const TRIE_FAIL: i32 = -1;

/// fn_26c1c0 (the fe_text morphology trie lookup).
///
/// The exact ARM call is `fn_26c1c0(reader, text, start, end, fold)`:
/// r0 = the `TrieReader`, r1 = the text, r2 = the first index, r3 = the
/// last index and the fifth argument (the stack slot) = the fold flag.
///
/// Lane b4 binds this from `fn_26bb6c` as
/// `component_live_b::fn_26c1c0_m(m, h, state, text, first as i32,
/// last as i32, fold)`, where `state` is its `DictionaryState`.
///
/// The reader walks the trie through the morphology family of the fe_text
/// unit (0x26b2f8, 0x26b37c, 0x26b444 and 0x26b530, lane c4 owns those
/// ports) and through the broker range (0x17a1a4 and 0x179208), so those
/// six callees go through the host.
pub fn fn_26c1c0_m(
    m: &mut Memory<'_>,
    h: &mut impl Host,
    reader: u32,
    text: u32,
    start: i32,
    end: i32,
    fold: i32,
) -> i32 {
    let tables = match guest_word(m, reader.wrapping_add(TRIE_TABLES)) {
        Some(v) if v != 0 => v,
        _ => return TRIE_FAIL,
    };
    let header = match guest_word(m, tables.wrapping_add(TRIE_HEADER)) {
        Some(v) if v != 0 => v,
        _ => return TRIE_FAIL,
    };
    let first_char = guest_byte(m, header.wrapping_add(TRIE_FIRST_CHAR)).unwrap_or(0);
    let terminator = guest_byte(m, header.wrapping_add(TRIE_TERMINATOR)).unwrap_or(0);
    let alphabet = guest_word(m, tables.wrapping_add(TRIE_ALPHABET)).unwrap_or(0);
    let byte = match guest_byte(m, text.wrapping_add(start as u32)) {
        Some(v) => v,
        None => return TRIE_FAIL,
    };
    let wanted = if fold == 0 {
        byte
    } else {
        crate::component_live_c::fn_240b28(u32::from(byte)) as u8
    };
    let wanted = match guest_byte(m, alphabet.wrapping_add(u32::from(wanted.wrapping_sub(first_char)))) {
        Some(v) => v,
        None => return TRIE_FAIL,
    };
    let initial_count = guest_half(m, header.wrapping_add(TRIE_INITIAL_COUNT)).unwrap_or(0);
    let initial_chars = guest_word(m, tables.wrapping_add(TRIE_INITIAL_CHARS)).unwrap_or(0);
    let initial_offsets = guest_word(m, tables.wrapping_add(TRIE_INITIAL_OFFSETS)).unwrap_or(0);
    let initial_indices = guest_word(m, tables.wrapping_add(TRIE_INITIAL_INDICES)).unwrap_or(0);
    let mut index = 0u16;
    let mut initial_index = 0u32;
    while index < initial_count {
        let c = guest_byte(m, initial_chars.wrapping_add(u32::from(index))).unwrap_or(0);
        if c >= wanted {
            break;
        }
        let step = guest_word(m, initial_indices.wrapping_add(u32::from(index) * 4)).unwrap_or(0);
        initial_index = initial_index.wrapping_add(step);
        index += 1;
    }
    if index >= initial_count {
        return TRIE_FAIL;
    }
    let c = guest_byte(m, initial_chars.wrapping_add(u32::from(index))).unwrap_or(0);
    if c != wanted {
        return TRIE_FAIL;
    }
    let mut result = initial_index;
    let mut position = guest_word(m, initial_offsets.wrapping_add(u32::from(index) * 4)).unwrap_or(0);
    let inline_limit = guest_byte(m, header.wrapping_add(TRIE_INLINE_LIMIT)).unwrap_or(0);
    if position <= u32::from(inline_limit) {
        let initial_pos = guest_word(m, header.wrapping_add(TRIE_INITIAL_POS)).unwrap_or(0);
        let initial_bit = guest_byte(m, header.wrapping_add(TRIE_INITIAL_BIT)).unwrap_or(0);
        m.put_word(reader.wrapping_add(TRIE_POS), initial_pos);
        m.put_byte(reader.wrapping_add(TRIE_BIT), initial_bit);
        position = initial_pos.wrapping_mul(8).wrapping_add(u32::from(initial_bit));
    }
    let length = (end - start + 1) as u16;
    let mut step = 1u16;
    let mut run_index = 0u16;
    loop {
        let wanted = if step == length {
            terminator
        } else {
            let at = text.wrapping_add(u32::from(step.wrapping_add(start as u16)));
            let byte = match guest_byte(m, at) {
                Some(v) => v,
                None => return TRIE_FAIL,
            };
            let folded = if fold != 0 {
                crate::component_live_c::fn_240b28(u32::from(byte)) as u8
            } else {
                byte
            };
            match guest_byte(
                m,
                alphabet.wrapping_add(u32::from(folded.wrapping_sub(first_char))),
            ) {
                Some(v) => v,
                None => return TRIE_FAIL,
            }
        };
        let mut skipped = 0u32;
        let next = match trie_step(
            m,
            h,
            reader,
            tables,
            header,
            wanted,
            &mut position,
            &mut run_index,
            &mut skipped,
        ) {
            Some(v) => v,
            None => return TRIE_FAIL,
        };
        result = result.wrapping_add(skipped);
        if wanted == terminator {
            break;
        }
        step = step.wrapping_add(1);
        let _ = next;
    }
    if position == 1 {
        result as i32
    } else {
        TRIE_FAIL
    }
}

/// One trie step. `None` when the walk misses.
#[allow(clippy::too_many_arguments)]
fn trie_step(
    m: &mut Memory<'_>,
    h: &mut impl Host,
    reader: u32,
    tables: u32,
    header: u32,
    wanted: u8,
    position: &mut u32,
    run_index: &mut u16,
    skipped: &mut u32,
) -> Option<u32> {
    let byte_pos = *position >> 3;
    let data_offset = guest_word(m, tables.wrapping_add(TRIE_DATA_OFFSET)).unwrap_or(0);
    let page_size = guest_half(m, header.wrapping_add(TRIE_PAGE_SIZE)).unwrap_or(0);
    let handle0 = guest_word(m, reader.wrapping_add(TRIE_HANDLE0)).unwrap_or(0);
    let handle1 = guest_word(m, reader.wrapping_add(TRIE_HANDLE1)).unwrap_or(0);
    // The data page comes from the broker: the page pointer lands in the
    // reader's data word.
    let page_slot = reader.wrapping_add(TRIE_DATA);
    let rc = h.call_m(
        0x17a1a4,
        &[handle0, handle1, data_offset.wrapping_add(byte_pos), u32::from(page_size), page_slot],
        m,
    );
    if rc != 0 {
        return None;
    }
    let page = guest_word(m, page_slot).unwrap_or(0);
    m.put_word(reader.wrapping_add(TRIE_POS), 0);
    let mut bit = (*position & 7) as u8;
    m.put_byte(reader.wrapping_add(TRIE_BIT), bit);
    *skipped = 0;
    let count_tree = guest_word(m, tables.wrapping_add(TRIE_COUNT_TREE)).unwrap_or(0);
    let char_tree = guest_word(m, tables.wrapping_add(TRIE_CHAR_TREE)).unwrap_or(0);
    let delta_tree = guest_word(m, tables.wrapping_add(TRIE_DELTA_TREE)).unwrap_or(0);
    let counts = guest_word(m, tables.wrapping_add(TRIE_COUNTS)).unwrap_or(0);
    let chars = guest_word(m, tables.wrapping_add(TRIE_CHARS)).unwrap_or(0);
    let deltas = guest_word(m, tables.wrapping_add(TRIE_DELTAS)).unwrap_or(0);
    let count_root = guest_half(m, header.wrapping_add(TRIE_COUNT_ROOT)).unwrap_or(0);
    let char_root = guest_half(m, header.wrapping_add(TRIE_CHAR_ROOT)).unwrap_or(0);
    let delta_root = guest_half(m, header.wrapping_add(TRIE_DELTA_ROOT)).unwrap_or(0);
    let direct_count = guest_half(m, header.wrapping_add(TRIE_DIRECT_COUNT)).unwrap_or(0);
    let terminator = guest_byte(m, header.wrapping_add(TRIE_TERMINATOR)).unwrap_or(0);
    let inline_limit = guest_byte(m, header.wrapping_add(TRIE_INLINE_LIMIT)).unwrap_or(0);
    let align_mask = guest_byte(m, header.wrapping_add(TRIE_ALIGN_MASK)).unwrap_or(0);
    let mut count = {
        let node = match fn_26b2f8_m(m, reader, count_tree, u32::from(count_root)) {
            Some(v) => v,
            None => return None,
        };
        let value = guest_half(m, counts.wrapping_add(node.wrapping_mul(2))).unwrap_or(0);
        value
    };
    let mut index = 1u16;
    let mut next = 0u32;
    let mut current = 0u8;
    let mut terminal = false;
    if count <= direct_count {
        let node = match fn_26b2f8_m(m, reader, char_tree, u32::from(char_root)) {
                Some(v) => v,
                None => return None,
            };
        current = guest_byte(m, chars.wrapping_add(node)).unwrap_or(0);
        if current != terminator {
            next = fn_26b444_m(m, reader).unwrap_or(0);
            if next == 0 {
                next = fn_26b530_m(m, reader).unwrap_or(0);
            }
            if current != wanted {
                *skipped = fn_26b37c_m(m, reader).unwrap_or(0);
            }
        } else if current != wanted {
            *skipped = 1;
        }
        while index < count && current < wanted {
            let node = match fn_26b2f8_m(m, reader, delta_tree, u32::from(delta_root)) {
                    Some(v) => v,
                    None => return None,
                };
            let delta = guest_byte(m, deltas.wrapping_add(node)).unwrap_or(0);
            current = current.wrapping_add(delta);
            if current != terminator {
                next = fn_26b444_m(m, reader).unwrap_or(0);
                if next == 0 {
                    next = fn_26b530_m(m, reader).unwrap_or(0);
                }
                if current != wanted {
                    *skipped = skipped.wrapping_add(fn_26b37c_m(m, reader).unwrap_or(0));
                }
            } else if current != wanted {
                *skipped = skipped.wrapping_add(1);
            }
            index += 1;
        }
        if current != wanted {
            h.call_m(0x179208, &[handle0, handle1, page], m);
            return None;
        }
        if wanted == terminator {
            terminal = true;
        } else if next <= u32::from(inline_limit) {
            if index < count {
                let _ = fn_26b37c_m(m, reader);
            }
            while index < count {
                let node = match fn_26b2f8_m(m, reader, delta_tree, u32::from(delta_root)) {
                    Some(v) => v,
                    None => return None,
                };
                let delta = guest_byte(m, deltas.wrapping_add(node)).unwrap_or(0);
                current = current.wrapping_add(delta);
                if current != terminator {
                    if fn_26b444_m(m, reader).unwrap_or(0) == 0 {
                        let _ = fn_26b530_m(m, reader);
                    }
                    if index < count.wrapping_sub(1) {
                        let _ = fn_26b37c_m(m, reader);
                    }
                }
                index += 1;
            }
            bit = align_bit(m, reader, align_mask);
            let pos = guest_word(m, reader.wrapping_add(TRIE_POS)).unwrap_or(0);
            *position = u32::from(bit).wrapping_add(pos.wrapping_add(byte_pos).wrapping_mul(8));
            h.call_m(0x179208, &[handle0, handle1, page], m);
            return Some(*position);
        } else {
            *position = next;
        }
    } else {
        count = count.wrapping_sub(direct_count);
        index = 0;
        loop {
            let node = match fn_26b2f8_m(m, reader, char_tree, u32::from(char_root)) {
                Some(v) => v,
                None => return None,
            };
            current = guest_byte(m, chars.wrapping_add(node)).unwrap_or(0);
            index += 1;
            if index > *run_index {
                break;
            }
        }
        if current != wanted {
            h.call_m(0x179208, &[handle0, handle1, page], m);
            return None;
        }
        index = run_index.wrapping_add(1);
        *run_index = index;
        *skipped = 0;
        if count <= index {
            *run_index = 0;
            if wanted == terminator {
                terminal = true;
            } else {
                let value = fn_26b444_m(m, reader).unwrap_or(0);
                if value != 0 {
                    if value <= u32::from(inline_limit) {
                        bit = align_bit(m, reader, align_mask);
                        let pos = guest_word(m, reader.wrapping_add(TRIE_POS)).unwrap_or(0);
                        *position =
                            u32::from(bit).wrapping_add(pos.wrapping_add(byte_pos).wrapping_mul(8));
                        h.call_m(0x179208, &[handle0, handle1, page], m);
                        return Some(*position);
                    }
                    *position = value;
                } else {
                    *position = fn_26b530_m(m, reader).unwrap_or(0);
                }
            }
        }
    }
    if terminal {
        h.call_m(0x179208, &[handle0, handle1, page], m);
        *position = 1;
        return Some(1);
    }
    h.call_m(0x179208, &[handle0, handle1, page], m);
    if *position == 0 {
        None
    } else {
        Some(*position)
    }
}

/// fn_26b2f8: walk the trie one bit at a time.
///
/// The target's reader holds the byte position at +0, the bit index at +4
/// and the data page at +8 (build/target/fn_26b2f8.s: `ldr r4, [r3, #8]`).
/// The tree is an array of two half word children; a node whose first
/// child is 0xffff is a leaf.
fn fn_26b2f8_m(m: &mut Memory<'_>, reader: u32, tree: u32, index: u32) -> Option<u32> {
    let mut position = guest_word(m, reader)?;
    let mut bit = guest_byte(m, reader.wrapping_add(4))?;
    let data = guest_word(m, reader.wrapping_add(8))?;
    let mut node = index;
    loop {
        let first = guest_half(m, tree.wrapping_add(node.wrapping_mul(4)))?;
        if first == u16::MAX {
            break;
        }
        let second = guest_half(m, tree.wrapping_add(node.wrapping_mul(4)).wrapping_add(2))?;
        let current = guest_byte(m, data.wrapping_add(position))?;
        // The target's shift masks the bit into the byte: a bit of eight or
        // more never selects the first child.
        let mask = if bit < 8 { 1u8 << bit } else { 0 };
        node = if current & mask != 0 {
            u32::from(first)
        } else {
            u32::from(second)
        };
        if bit < 7 {
            bit += 1;
        } else {
            position = position.wrapping_add(1);
            bit = 0;
        }
    }
    m.put_word(reader, position);
    m.put_byte(reader.wrapping_add(4), bit);
    Some(node)
}

/// fn_26b37c: read the count value out of the count table.
///
/// The reader holds the tables at +12; the tables hold the header at +0,
/// the trie at +36 and the counts at +56.
fn fn_26b37c_m(m: &mut Memory<'_>, reader: u32) -> Option<u32> {
    let tables = guest_word(m, reader.wrapping_add(0x0c))?;
    let header = guest_word(m, tables)?;
    let bits = guest_byte(m, header.wrapping_add(0x0d))? as u32;
    let index = guest_half(m, header.wrapping_add(0x20))? as u32;
    let tree = guest_word(m, tables.wrapping_add(36))?;
    let data = guest_word(m, tables.wrapping_add(56))?;
    let node = fn_26b2f8_m(m, reader, tree, index)?;
    if bits <= 16 {
        return guest_half(m, data.wrapping_add(node.wrapping_mul(2))).map(u32::from);
    }
    let position = node.wrapping_mul(bits);
    let shift = position & 15;
    let word = (position & 0xfffff) >> 4;
    let low = guest_half(m, data.wrapping_add(word.wrapping_mul(2)))? as u32 >> shift;
    let remaining = 32 - shift;
    if bits > remaining {
        let middle = guest_half(m, data.wrapping_add(word.wrapping_mul(2) + 2))? as u32;
        let high = guest_half(m, data.wrapping_add(word.wrapping_mul(2) + 4))? as u32;
        let mask = 0xffffu32 >> ((48 - bits) - shift);
        Some(
            low.wrapping_add(middle.wrapping_shl(16 - shift))
                .wrapping_add((high & mask).wrapping_shl(remaining)),
        )
    } else {
        let next = guest_half(m, data.wrapping_add(word.wrapping_mul(2) + 2))? as u32;
        let mask = 0xffffu32 >> ((32 - bits) - shift);
        Some(low.wrapping_add((next & mask).wrapping_shl(16 - shift)))
    }
}

/// fn_26b444: read the packed value out of the offset table.
///
/// The reader holds the tables at +12; the tables hold the header at +0,
/// the trie at +40 and the packed values at +60.
fn fn_26b444_m(m: &mut Memory<'_>, reader: u32) -> Option<u32> {
    let tables = guest_word(m, reader.wrapping_add(0x0c))?;
    let header = guest_word(m, tables)?;
    let width = guest_byte(m, header.wrapping_add(0x0c))? as u32;
    let index = guest_half(m, header.wrapping_add(0x22))? as u32;
    let tree = guest_word(m, tables.wrapping_add(40))?;
    let data = guest_word(m, tables.wrapping_add(60))?;
    let node = fn_26b2f8_m(m, reader, tree, index)?;
    let bitpos = node.wrapping_mul(width);
    let shift = bitpos & 0xf;
    let remaining = 0x10 - shift;
    let word = (bitpos & 0xfffff) >> 4;
    let mut value;
    if remaining < width {
        let low = guest_half(m, data.wrapping_add(word.wrapping_mul(2)))? as u32 >> shift;
        let out_shift = 0x20 - shift;
        if out_shift < width {
            let middle = guest_half(m, data.wrapping_add(word.wrapping_mul(2) + 2))? as u32;
            let last = guest_half(m, data.wrapping_add(word.wrapping_mul(2) + 4))? as u32;
            let mask = 0xffffu32 >> ((0x30 - width) - shift);
            value = low
                .wrapping_add(middle.wrapping_shl(remaining))
                .wrapping_add((last & mask).wrapping_shl(out_shift));
        } else {
            let next = guest_half(m, data.wrapping_add(word.wrapping_mul(2) + 2))? as u32;
            let mask = 0xffffu32 >> ((0x20 - width) - shift);
            value = low.wrapping_add((next & mask).wrapping_shl(remaining));
        }
    } else {
        let low = guest_half(m, data.wrapping_add(word.wrapping_mul(2)))? as u32 >> shift;
        value = low & (0xffffu32 >> (0x10 - width));
    }
    let limit = guest_byte(m, header.wrapping_add(0x12))? as u32;
    if limit < value {
        let out_shift = guest_byte(m, header.wrapping_add(0x0e))? as u32;
        value = value.wrapping_shl(out_shift);
    }
    Some(value)
}

/// fn_26b530: read the value out of the byte stream.
///
/// The reader holds the byte position at +0, the bit index at +4, the data
/// page at +8 and the tables at +12; the width and the output shift are
/// the header's words at +12 and +14.
fn fn_26b530_m(m: &mut Memory<'_>, reader: u32) -> Option<u32> {
    let position = guest_word(m, reader)?;
    let bit = guest_byte(m, reader.wrapping_add(4))? as u32;
    let base = guest_word(m, reader.wrapping_add(8))?;
    let tables = guest_word(m, reader.wrapping_add(0x0c))?;
    let header = guest_word(m, tables)?;
    let width = guest_byte(m, header.wrapping_add(0x0c))? as u32;
    let out_shift = guest_byte(m, header.wrapping_add(0x0e))? as u32;
    let end = width.wrapping_add(bit);
    let mut value = guest_byte(m, base.wrapping_add(position))? as u32 >> bit;
    if end > 8 {
        let mut index = 1u32;
        if end > 16 {
            loop {
                let byte = guest_byte(m, base.wrapping_add(position.wrapping_add(index)))? as u32;
                value = value.wrapping_add(byte.wrapping_shl(index.wrapping_mul(8).wrapping_sub(bit)));
                index = (index + 1) & 0xff;
                if end <= index.wrapping_mul(8).wrapping_add(8) {
                    break;
                }
            }
        }
        let bits = index.wrapping_mul(8);
        let mut remaining = bits.wrapping_add(8);
        remaining -= bit;
        remaining -= width;
        let byte = guest_byte(m, base.wrapping_add(position.wrapping_add(index)))? as u32;
        let mask = 0xffu32 >> remaining;
        value = value.wrapping_add((byte & mask).wrapping_shl(bits.wrapping_sub(bit)));
    } else {
        value &= 0xffu32 >> (8 - width);
    }
    m.put_word(reader, position.wrapping_add(((bit + width) & 0x38) >> 3));
    m.put_byte(reader.wrapping_add(4), ((bit + width) & 7) as u8);
    Some(value.wrapping_shl(out_shift))
}

/// The bit alignment the inline path applies.
///
/// The C aligns the reader's live bit (`reader->bit`, advanced by the
/// 0x26b2f8/0x26b444/0x26b530/0x26b37c walks above), never the step-entry
/// bit: src/fn_26c1c0.c:120-127,158-164; the ROM object reloads
/// `[r4, #4]` each iteration (fn_26c1c0+0x5cc/+0x72c).
fn align_bit(m: &mut Memory<'_>, reader: u32, mask: u8) -> u8 {
    let mut bit = guest_byte(m, reader.wrapping_add(TRIE_BIT)).unwrap_or(0);
    while bit & mask != 0 {
        bit = bit.wrapping_add(1);
    }
    if bit == 8 {
        bit = 0;
        let pos = guest_word(m, reader.wrapping_add(TRIE_POS)).unwrap_or(0);
        m.put_word(reader.wrapping_add(TRIE_POS), pos.wrapping_add(1));
    }
    m.put_byte(reader.wrapping_add(TRIE_BIT), bit);
    bit
}

// ===== fn_1e7908: the FE normout processor ==============================

/// The process-object vtable slots of fn_1e7908.
const NORM_DP_FIRST: u32 = 0x34;
const NORM_DP_NEXT: u32 = 0x3c;
const NORM_DP_SET: u32 = 0x50;
const NORM_DP_GET: u32 = 0x54;
const NORM_DP_GET_PTR: u32 = 0x58;
const NORM_DP_GET_TYPE: u32 = 0x5c;
const NORM_DP_REMOVE: u32 = 0x60;
const NORM_OUT_SET_LAYER: u32 = 0x24;
const NORM_OUT_GET_LAYER: u32 = 0x28;
const NORM_OUT_PROCESS: u32 = 0x2c;
const NORM_OUT_SET_POS: u32 = 0x38;
const NORM_OUT_GET_POS: u32 = 0x3c;
const NORM_OUT_SET_LAYER_X: u32 = 0x40;

/// One process word record: node, kind, start, end, nstart, nend.
const NORM_RECORD_LEN: u32 = 12;
/// One process marker: type, refPos, refLen, curPos, curLen.
const NORM_MARKER_LEN: u32 = 24;

/// The length of a guest string. `None` when it is not terminated in the arena.
fn norm_strlen(m: &Memory<'_>, s: u32) -> Option<u32> {
    let mut i = 0u32;
    loop {
        match guest_byte(m, s.checked_add(i)?) {
            Some(0) => return Some(i),
            Some(_) => i = i.checked_add(1)?,
            None => return None,
        }
    }
}

/// Write one guest byte. False when the address is outside the arena.
fn norm_put_byte(m: &mut Memory<'_>, p: u32, v: u8) -> bool {
    if !guest_range(m, p, 1) {
        return false;
    }
    let at = (p - m.base) as usize;
    m.bytes[at] = v;
    true
}

/// Write one guest half word. False when the address is outside the arena.
fn norm_put_half(m: &mut Memory<'_>, p: u32, v: u16) -> bool {
    if !guest_range(m, p, 2) {
        return false;
    }
    let at = (p - m.base) as usize;
    m.bytes[at..at + 2].copy_from_slice(&v.to_le_bytes());
    true
}

/// Write one guest word. False when the address is outside the arena.
fn norm_put_word(m: &mut Memory<'_>, p: u32, v: u32) -> bool {
    if !guest_range(m, p, 4) {
        return false;
    }
    let at = (p - m.base) as usize;
    m.bytes[at..at + 4].copy_from_slice(&v.to_le_bytes());
    true
}

/// fn_195dac: append at most `n` bytes of `src` to `dst` and terminate it.
fn norm_append_n(m: &mut Memory<'_>, dst: u32, src: u32, n: u32) -> bool {
    let mut at = dst;
    loop {
        match guest_byte(m, at) {
            Some(0) => break,
            Some(_) => match at.checked_add(1) {
                Some(v) => at = v,
                None => return false,
            },
            None => return false,
        }
    }
    let mut left = n;
    let mut sp = src;
    while left > 0 {
        left -= 1;
        let value = match guest_byte(m, sp) {
            Some(v) => v,
            None => return false,
        };
        sp = sp.wrapping_add(1);
        if !norm_put_byte(m, at, value) {
            return false;
        }
        at = at.wrapping_add(1);
        if value == 0 {
            return true;
        }
    }
    norm_put_byte(m, at, 0)
}

/// fn_195c90: append the whole `src` string to `dst`.
fn norm_append(m: &mut Memory<'_>, dst: u32, src: u32) -> bool {
    match norm_strlen(m, src) {
        Some(len) => norm_append_n(m, dst, src, len),
        None => false,
    }
}

/// fn_195b38: move `n` bytes inside the arena.
fn norm_move(m: &mut Memory<'_>, dst: u32, src: u32, n: u32) -> bool {
    if !guest_range(m, dst, n) || !guest_range(m, src, n) {
        return false;
    }
    let d = (dst - m.base) as usize;
    let s = (src - m.base) as usize;
    m.bytes.copy_within(s..s + n as usize, d);
    true
}

/// fn_195d30: find `c` in the guest string `s`. Zero when it is absent.
fn norm_find_byte(m: &Memory<'_>, s: u32, c: u32) -> u32 {
    let mut i = 0u32;
    loop {
        match guest_byte(m, s.wrapping_add(i)) {
            Some(v) => {
                if u32::from(v) == c {
                    return s.wrapping_add(i);
                }
                if v == 0 {
                    return 0;
                }
            }
            None => return 0,
        }
        i = i.wrapping_add(1);
    }
}

/// fn_195d58: compare two guest strings.
fn norm_compare(m: &Memory<'_>, a: u32, b: u32) -> i32 {
    let mut i = 0u32;
    loop {
        let x = guest_byte(m, a.wrapping_add(i)).unwrap_or(0);
        let y = guest_byte(m, b.wrapping_add(i)).unwrap_or(0);
        if x != y {
            return i32::from(x) - i32::from(y);
        }
        if y == 0 {
            return 0;
        }
        i = i.wrapping_add(1);
    }
}

/// fn_1960a8: find `needle` in the guest string `hay`. Zero when it is absent.
fn norm_find_sub(m: &Memory<'_>, hay: u32, needle: u32) -> u32 {
    let nlen = match norm_strlen(m, needle) {
        Some(v) => v,
        None => return 0,
    };
    if nlen == 0 {
        return hay;
    }
    let mut i = 0u32;
    loop {
        let at = hay.wrapping_add(i);
        if norm_compare(m, at, needle) == 0 {
            return at;
        }
        match guest_byte(m, at) {
            Some(0) | None => return 0,
            Some(_) => i = i.wrapping_add(1),
        }
    }
}

/// Call one vtable slot of the guest process object.
fn norm_vt(
    m: &mut Memory<'_>,
    h: &mut impl Host,
    vt: u32,
    slot: u32,
    args: &[u32],
) -> u32 {
    match guest_word(m, vt.wrapping_add(slot)) {
        Some(f) => h.call_m(f, args, m),
        None => 0x80002001,
    }
}

/// fn_1e7908: the FE normout processor.
///
/// The target walks the depes tree through the process vtable, builds the
/// four output layers in the guest heap and writes the normalised text back
/// through the output vtable. The port issues the same guest calls, in the
/// target's order; the data stays in the guest arena.
pub fn fn_1e7908_m(
    m: &mut Memory<'_>,
    p: u32,
    b: u32,
    h0: u32,
    h1: u32,
    done: u32,
    scratch: u32,
    h: &mut impl Host,
) -> u32 {
    const FRAME: u32 = 256;
    if !guest_range(m, scratch, FRAME) {
        return 0x8b802008;
    }
    let at = (scratch - m.base) as usize;
    let mut saved = [0u8; 256];
    saved.copy_from_slice(&m.bytes[at..at + FRAME as usize]);
    let rc = fn_1e7908_run(m, p, b, h0, h1, done, scratch, h);
    m.bytes[at..at + FRAME as usize].copy_from_slice(&saved);
    rc
}

/// The normout processor. The scratch block holds the target's frame
/// temporaries; the caller owns it and the port restores it.
fn fn_1e7908_run(
    m: &mut Memory<'_>,
    p: u32,
    b: u32,
    h0: u32,
    h1: u32,
    done: u32,
    scratch: u32,
    h: &mut impl Host,
) -> u32 {
    let lit = scratch;
    let empty = scratch + 0x0a;
    let sp = scratch + 0x0b;
    let til = scratch + 0x0d;
    let xil = scratch + 0x0f;
    let yil = scratch + 0x11;
    let e1 = scratch + 0x13;
    let e2 = scratch + 0x15;
    let dash = scratch + 0x17;
    let s_pos = scratch + 0x20;
    let s_root = scratch + 0x24;
    let s_type = scratch + 0x26;
    let s_orig = scratch + 0x28;
    let s_origlen = scratch + 0x2c;
    let s_markers = scratch + 0x2e;
    let s_cnt = scratch + 0x32;
    let s_kind = scratch + 0x34;
    let s_start = scratch + 0x38;
    let s_end = scratch + 0x3c;
    let s_pstart = scratch + 0x40;
    let s_pend = scratch + 0x44;
    let s_str = scratch + 0x48;
    let s_f5 = scratch + 0x4c;
    let s_f6 = scratch + 0x50;
    let s_node = scratch + 0x54;
    let s_tmp = scratch + 0x56;
    let s_no1 = scratch + 0x58;
    let s_norig = scratch + 0x5a;
    let s_out1 = scratch + 0x60;
    let s_out4 = scratch + 0x64;
    let s_len = scratch + 0x68;

    let at = (lit - m.base) as usize;
    m.bytes[at..at + 8].copy_from_slice(b"normout\0");
    m.bytes[at + 0x0a] = 0;
    m.bytes[at + 0x0b] = b' ';
    m.bytes[at + 0x0c] = 0;
    m.bytes[at + 0x0d] = b'~';
    m.bytes[at + 0x0e] = 0;
    m.bytes[at + 0x0f] = b'X';
    m.bytes[at + 0x10] = 0;
    m.bytes[at + 0x11] = b'Y';
    m.bytes[at + 0x12] = 0;
    m.bytes[at + 0x13] = 0x1e;
    m.bytes[at + 0x14] = 0;
    m.bytes[at + 0x15] = 0x1f;
    m.bytes[at + 0x16] = 0;
    m.bytes[at + 0x17] = b'-';
    m.bytes[at + 0x18] = b'*';
    m.bytes[at + 0x19] = 0;

    if crate::frontend::validate_handle(p != 0, b, 0xf3b3, 0x3c) >= 0x80000000 {
        return 0x8b802008;
    }
    if !norm_put_byte(m, done, 1) {
        return 0x8b802008;
    }
    let ctx = match guest_word(m, p) {
        Some(v) => v,
        None => return 0x8b802008,
    };
    let dp = match guest_word(m, p + 4) {
        Some(v) => v,
        None => return 0x8b802008,
    };
    let out = match guest_word(m, p + 0x10) {
        Some(v) => v,
        None => return 0x8b802008,
    };
    let ph0 = match guest_word(m, p + 8) {
        Some(v) => v,
        None => return 0x8b802008,
    };
    let ph1 = match guest_word(m, p + 0xc) {
        Some(v) => v,
        None => return 0x8b802008,
    };
    let has_l5 = guest_byte(m, p + 0x34).unwrap_or(0) != 0;
    let heap = match guest_word(m, ctx + 4) {
        Some(v) => v,
        None => return 0x8b802008,
    };

    let mut recs = 0u32;
    let mut se = 0u32;
    let mut joined = 0u32;
    let mut l1 = 0u32;
    let mut l2 = 0u32;
    let mut l3 = 0u32;
    let mut l4 = 0u32;
    let mut l5 = 0u32;

    // out->getPos(p->h, &pos, 0)
    let rc = norm_vt(m, h, out, NORM_OUT_GET_POS, &[ph0, ph1, s_pos, 0]);
    if rc >= 0x80000000 {
        return rc;
    }
    let mut pos = guest_word(m, s_pos).unwrap_or(0) as i32;

    // dp->first(h, 1, 0, &root)
    let rc = norm_vt(m, h, dp, NORM_DP_FIRST, &[h0, h1, 1, 0, s_root]);
    if rc >= 0x80000000 {
        return rc;
    }
    let root = guest_half(m, s_root).unwrap_or(0) as u32;
    // dp->getType(h, root, 0, &type)
    let rc = norm_vt(m, h, dp, NORM_DP_GET_TYPE, &[h0, h1, root, 0, s_type]);
    if rc >= 0x80000000 {
        return rc;
    }
    if guest_half(m, s_type).unwrap_or(0) != 1 {
        return rc;
    }
    // dp->getPtr(h, root, 0, &orig, &origLen)
    let rc = norm_vt(m, h, dp, NORM_DP_GET_PTR, &[h0, h1, root, 0, s_orig, s_origlen]);
    if rc >= 0x80000000 {
        return rc;
    }
    let orig = guest_word(m, s_orig).unwrap_or(0);
    let orig_len = guest_half(m, s_origlen).unwrap_or(0) as u32;
    if orig_len <= 1 {
        return rc;
    }
    // dp->getPtr(h, root, 1, &markers, &cnt)
    let rc = norm_vt(m, h, dp, NORM_DP_GET_PTR, &[h0, h1, root, 1, s_markers, s_cnt]);
    if rc >= 0x80000000 {
        return rc;
    }
    let markers = guest_word(m, s_markers).unwrap_or(0);
    let n_markers = guest_half(m, s_cnt).unwrap_or(0) as u32;
    let base = guest_word(m, markers + 12).unwrap_or(0) as i32;
    let mut mi = 0u32;
    while mi < n_markers
        && guest_word(m, markers + mi * NORM_MARKER_LEN + 12).unwrap_or(0) as i32 == base
    {
        mi += 1;
    }

    // The counting pass: the total size of the four layers.
    let rc = norm_vt(m, h, dp, NORM_DP_FIRST, &[h0, h1, 2, root, s_node]);
    if rc >= 0x80000000 {
        return rc;
    }
    let mut node = guest_half(m, s_node).unwrap_or(0) as u32;
    let mut node2 = node;
    let mut n_recs = 0u32;
    let mut total = 1u32;
    let mut first = true;
    while node != 0 {
        let rc = norm_vt(m, h, dp, NORM_DP_GET, &[h0, h1, node, 0, 1, s_kind, s_cnt]);
        if rc >= 0x80000000 {
            return rc;
        }
        let kind = guest_byte(m, s_kind).unwrap_or(0);
        n_recs += 1;
        if kind == 4 || kind == 9 || kind == 10 {
            if !first {
                total = total.wrapping_add(1);
            }
            first = false;
            let rc = norm_vt(m, h, dp, NORM_DP_GET, &[h0, h1, node, 1, 1, s_start, s_cnt]);
            if rc >= 0x80000000 {
                return rc;
            }
            let start = guest_half(m, s_start).unwrap_or(0) as u32;
            let rc = norm_vt(m, h, dp, NORM_DP_GET, &[h0, h1, node, 2, 1, s_end, s_cnt]);
            if rc >= 0x80000000 {
                return rc;
            }
            let end = guest_half(m, s_end).unwrap_or(0) as u32;
            total = total.wrapping_add(end).wrapping_sub(start);
        }
        if kind == 7 {
            let rc = norm_vt(m, h, dp, NORM_DP_GET_PTR, &[h0, h1, node, 4, s_str, s_cnt]);
            if rc >= 0x80000000 {
                return rc;
            }
            let text = guest_word(m, s_str).unwrap_or(0);
            let len = norm_strlen(m, text).unwrap_or(0);
            total = total.wrapping_add(len).wrapping_add(2);
        }
        if has_l5 && kind == 6 {
            let rc = norm_vt(m, h, dp, NORM_DP_GET_PTR, &[h0, h1, node, 4, s_str, s_cnt]);
            if rc >= 0x80000000 {
                return rc;
            }
            let text = guest_word(m, s_str).unwrap_or(0);
            let len = norm_strlen(m, text).unwrap_or(0);
            total = total.wrapping_add(len.wrapping_mul(2)).wrapping_sub(2);
        }
        let rc = norm_vt(m, h, dp, NORM_DP_NEXT, &[h0, h1, node, s_node]);
        if rc >= 0x80000000 {
            return rc;
        }
        node = guest_half(m, s_node).unwrap_or(0) as u32;
    }

    // recs = fn_1963b8(heap, 1, nRecs * 12 + 1)
    recs = h.call_m(0x1963b8, &[heap, 1, n_recs * NORM_RECORD_LEN + 1], m);
    if recs == 0 {
        return 0x8b80200a;
    }
    for i in 0..12u32 {
        if !norm_put_byte(m, recs + i, 0) {
            return 0x8b80200a;
        }
    }
    l1 = h.call_m(0x196394, &[heap, total], m);
    if l1 == 0 {
        return 0x8b80200a;
    }
    norm_put_byte(m, l1, 0);
    l2 = h.call_m(0x196394, &[heap, total], m);
    if l2 == 0 {
        return 0x8b80200a;
    }
    norm_put_byte(m, l2, 0);
    l3 = h.call_m(0x196394, &[heap, total], m);
    if l3 == 0 {
        return 0x8b80200a;
    }
    norm_put_byte(m, l3, 0);
    l4 = h.call_m(0x196394, &[heap, total], m);
    if l4 == 0 {
        return 0x8b80200a;
    }
    norm_put_byte(m, l4, 0);
    if has_l5 {
        l5 = h.call_m(0x196394, &[heap, total], m);
        if l5 == 0 {
            return 0x8b80200a;
        }
        norm_put_byte(m, l5, 0);
    }
    norm_put_word(m, p + 0x20, l1);
    norm_put_word(m, p + 0x24, l2);
    norm_put_word(m, p + 0x28, l3);
    norm_put_word(m, p + 0x2c, l4);
    norm_put_word(m, p + 0x30, l5);

    // The building pass: fill the record array and the four layers.
    first = true;
    let rc = norm_vt(m, h, dp, NORM_DP_FIRST, &[h0, h1, 2, root, s_node]);
    if rc >= 0x80000000 {
        return norm_cleanup(m, h, heap, p, recs, se, [l1, l2, l3, l4, l5], joined, rc);
    }
    node = guest_half(m, s_node).unwrap_or(0) as u32;
    let mut rec_idx = 0u32;
    let mut wlen = 0u32;
    let mut pending = 0u32;
    let mut phrase = 0u32;
    let mut p_start = 0u32;
    let mut p_end = 0u32;
    let mut last_end = 0u32;
    while node != 0 {
        let rec = recs + rec_idx * NORM_RECORD_LEN;
        norm_put_half(m, rec, node as u16);
        let rc = norm_vt(m, h, dp, NORM_DP_GET, &[h0, h1, node, 0, 1, s_kind, s_cnt]);
        if rc >= 0x80000000 {
            return norm_cleanup(m, h, heap, p, recs, se, [l1, l2, l3, l4, l5], joined, rc);
        }
        let kind = guest_byte(m, s_kind).unwrap_or(0);
        norm_put_byte(m, rec + 2, kind);
        let rc = norm_vt(m, h, dp, NORM_DP_GET, &[h0, h1, node, 1, 1, s_start, s_cnt]);
        if rc >= 0x80000000 {
            return norm_cleanup(m, h, heap, p, recs, se, [l1, l2, l3, l4, l5], joined, rc);
        }
        let start = guest_half(m, s_start).unwrap_or(0) as u32;
        let rc = norm_vt(m, h, dp, NORM_DP_GET, &[h0, h1, node, 2, 1, s_end, s_cnt]);
        if rc >= 0x80000000 {
            return norm_cleanup(m, h, heap, p, recs, se, [l1, l2, l3, l4, l5], joined, rc);
        }
        let end = guest_half(m, s_end).unwrap_or(0) as u32;
        norm_put_half(m, rec + 4, start as u16);
        norm_put_half(m, rec + 8, start as u16);
        norm_put_half(m, rec + 6, end as u16);
        norm_put_half(m, rec + 10, end as u16);
        if kind == 4 || kind == 9 || kind == 10 {
            last_end = end;
            let rc = norm_vt(m, h, dp, NORM_DP_GET_PTR, &[h0, h1, node, 5, s_f5, s_cnt]);
            if rc >= 0x80000000 {
                return norm_cleanup(m, h, heap, p, recs, se, [l1, l2, l3, l4, l5], joined, rc);
            }
            let f5 = guest_word(m, s_f5).unwrap_or(0);
            let rc = norm_vt(m, h, dp, NORM_DP_GET_PTR, &[h0, h1, node, 6, s_f6, s_cnt]);
            if rc >= 0x80000000 {
                return norm_cleanup(m, h, heap, p, recs, se, [l1, l2, l3, l4, l5], joined, rc);
            }
            let f6 = guest_word(m, s_f6).unwrap_or(0);
            if has_l5 && !first {
                if phrase != 0 && start != p_start {
                    if start > p_end {
                        let plen = norm_strlen(m, phrase).unwrap_or(0);
                        if plen.wrapping_mul(2).wrapping_sub(2) < pending {
                            norm_append(m, l5, xil);
                            norm_append_n(m, l5, phrase + 1, plen - 2);
                            let mut j = plen.wrapping_mul(2).wrapping_sub(2);
                            while j < pending {
                                norm_append(m, l5, til);
                                j += 1;
                            }
                            norm_append(m, l5, yil);
                            norm_append_n(m, l5, phrase + 1, plen - 2);
                        } else {
                            norm_append(m, l5, xil);
                            norm_append_n(m, l5, phrase + 1, plen - 2);
                            norm_append(m, l5, yil);
                            norm_append_n(m, l5, phrase + 1, plen - 2);
                            let mut j = pending;
                            while j < plen.wrapping_mul(2).wrapping_sub(2) {
                                norm_append(m, l1, til);
                                norm_append(m, l2, til);
                                norm_append(m, l3, til);
                                norm_append(m, l4, til);
                                j += 1;
                            }
                        }
                        phrase = 0;
                    }
                } else {
                    let mut j = 0u32;
                    while j < wlen {
                        norm_append(m, l5, sp);
                        j += 1;
                    }
                }
            }
            let mut added = 0u32;
            if !first {
                norm_append_n(m, l1, orig.wrapping_add(start).wrapping_sub(1), 1);
                norm_append(m, l2, sp);
                norm_append(m, l3, sp);
                norm_append_n(m, l4, orig.wrapping_add(start).wrapping_sub(1), 1);
                added = 1;
            }
            norm_append_n(m, l1, orig.wrapping_add(start), end - start);
            norm_append_n(m, l4, orig.wrapping_add(start), end - start);
            let mut j = 0u32;
            while j < end - start {
                norm_append_n(m, l2, f5, 1);
                norm_append_n(m, l3, f6, 1);
                j += 1;
            }
            wlen = end - start;
            if has_l5 {
                if phrase == 0 || start == p_start {
                    let mut j = 0u32;
                    while j < added {
                        norm_append(m, l5, sp);
                        j += 1;
                    }
                    if phrase != 0 {
                        pending = wlen;
                    }
                } else {
                    pending = wlen.wrapping_add(added).wrapping_add(pending);
                }
            }
            first = false;
        }
        if kind == 7 {
            let l1len = norm_strlen(m, l1).unwrap_or(0);
            let rc = norm_vt(m, h, dp, NORM_DP_GET_PTR, &[h0, h1, node, 4, s_str, s_cnt]);
            if rc >= 0x80000000 {
                return norm_cleanup(m, h, heap, p, recs, se, [l1, l2, l3, l4, l5], joined, rc);
            }
            let text = guest_word(m, s_str).unwrap_or(0);
            let tlen = norm_strlen(m, text).unwrap_or(0);
            let n = tlen + 2;
            if start < last_end {
                let back = last_end - start + 1;
                let off = l1len.wrapping_sub(last_end);
                norm_move(m, l1 + off + start + n, l1 + off + start, back);
                norm_move(m, l2 + off + start + n, l2 + off + start, back);
                norm_move(m, l3 + off + start + n, l3 + off + start, back);
                norm_move(m, l4 + off + start + n, l4 + off + start, back);
                norm_put_byte(m, l1 + off + start, 0x1e);
                norm_put_byte(m, l2 + off + start, b' ');
                norm_put_byte(m, l3 + off + start, b' ');
                norm_put_byte(m, l4 + off + start, b'~');
                let mut j = 0u32;
                while j < n - 2 {
                    let v = guest_byte(m, text + j).unwrap_or(0);
                    norm_put_byte(m, l1 + off + start + j + 1, v);
                    norm_put_byte(m, l2 + off + start + j + 1, b' ');
                    norm_put_byte(m, l3 + off + start + j + 1, b' ');
                    norm_put_byte(m, l4 + off + start + j + 1, b'~');
                    j += 1;
                }
                norm_put_byte(m, l1 + off + start + n - 1, 0x1f);
                norm_put_byte(m, l2 + off + start + n - 1, b' ');
                norm_put_byte(m, l3 + off + start + n - 1, b' ');
                norm_put_byte(m, l4 + off + start + n - 1, b'~');
            } else {
                norm_append(m, l1, e1);
                norm_append(m, l1, text);
                norm_append(m, l1, e2);
                let mut j = 0u32;
                while j < n {
                    norm_append(m, l2, sp);
                    norm_append(m, l3, sp);
                    norm_append(m, l4, til);
                    j += 1;
                }
            }
            if phrase != 0 {
                pending = tlen.wrapping_add(pending).wrapping_add(2);
            }
            wlen = tlen.wrapping_add(wlen).wrapping_add(2);
        }
        if kind == 6 {
            let rc = norm_vt(m, h, dp, NORM_DP_GET, &[h0, h1, node, 1, 1, s_pstart, s_cnt]);
            if rc >= 0x80000000 {
                return norm_cleanup(m, h, heap, p, recs, se, [l1, l2, l3, l4, l5], joined, rc);
            }
            p_start = guest_half(m, s_pstart).unwrap_or(0) as u32;
            if has_l5 && phrase != 0 && p_start > p_end {
                let plen = norm_strlen(m, phrase).unwrap_or(0);
                if plen.wrapping_mul(2).wrapping_sub(2) < pending {
                    norm_append(m, l5, xil);
                    norm_append_n(m, l5, phrase + 1, plen - 2);
                    let mut j = plen.wrapping_mul(2).wrapping_sub(2);
                    while j < pending {
                        norm_append(m, l5, til);
                        j += 1;
                    }
                    norm_append(m, l5, yil);
                    norm_append_n(m, l5, phrase + 1, plen - 2);
                } else {
                    norm_append(m, l5, xil);
                    norm_append_n(m, l5, phrase + 1, plen - 2);
                    norm_append(m, l5, yil);
                    norm_append_n(m, l5, phrase + 1, plen - 2);
                    let mut j = pending;
                    while j < plen.wrapping_mul(2).wrapping_sub(2) {
                        norm_append(m, l1, til);
                        norm_append(m, l2, til);
                        norm_append(m, l3, til);
                        norm_append(m, l4, til);
                        j += 1;
                    }
                }
                phrase = 0;
                wlen = 0;
            }
            let rc = norm_vt(m, h, dp, NORM_DP_GET_PTR, &[h0, h1, node, 4, s_str, s_cnt]);
            if rc >= 0x80000000 {
                return norm_cleanup(m, h, heap, p, recs, se, [l1, l2, l3, l4, l5], joined, rc);
            }
            phrase = guest_word(m, s_str).unwrap_or(0);
            let rc = norm_vt(m, h, dp, NORM_DP_GET, &[h0, h1, node, 2, 1, s_pend, s_cnt]);
            if rc >= 0x80000000 {
                return norm_cleanup(m, h, heap, p, recs, se, [l1, l2, l3, l4, l5], joined, rc);
            }
            p_end = guest_half(m, s_pend).unwrap_or(0) as u32;
            pending = wlen;
        }
        rec_idx += 1;
        let rc = norm_vt(m, h, dp, NORM_DP_NEXT, &[h0, h1, node, s_node]);
        if rc >= 0x80000000 {
            return norm_cleanup(m, h, heap, p, recs, se, [l1, l2, l3, l4, l5], joined, rc);
        }
        node = guest_half(m, s_node).unwrap_or(0) as u32;
    }
    if has_l5 && !first {
        if phrase != 0 {
            let plen = norm_strlen(m, phrase).unwrap_or(0);
            if plen.wrapping_mul(2).wrapping_sub(2) < pending {
                norm_append(m, l5, xil);
                norm_append_n(m, l5, phrase + 1, plen - 2);
                let mut j = plen.wrapping_mul(2).wrapping_sub(2);
                while j < pending {
                    norm_append(m, l5, til);
                    j += 1;
                }
                norm_append(m, l5, yil);
                norm_append_n(m, l5, phrase + 1, plen - 2);
            } else {
                norm_append(m, l5, xil);
                norm_append_n(m, l5, phrase + 1, plen - 2);
                norm_append(m, l5, yil);
                norm_append_n(m, l5, phrase + 1, plen - 2);
                while pending < plen.wrapping_mul(2).wrapping_sub(2) {
                    norm_append(m, l1, til);
                    norm_append(m, l2, til);
                    norm_append(m, l3, til);
                    norm_append(m, l4, til);
                    pending += 1;
                }
            }
        } else {
            let mut j = 0u32;
            while j < wlen {
                norm_append(m, l5, sp);
                j += 1;
            }
        }
    }

    // The four layers go out through the output vtable.
    let layers = [l1, l2, l3, l4];
    for (index, buffer) in layers.iter().enumerate() {
        let len = norm_strlen(m, *buffer).unwrap_or(0);
        let rc = norm_vt(
            m,
            h,
            out,
            NORM_OUT_SET_LAYER,
            &[ph0, ph1, index as u32, *buffer, len],
        );
        if rc >= 0x80000000 {
            return norm_cleanup(m, h, heap, p, recs, se, [l1, l2, l3, l4, l5], joined, rc);
        }
    }
    if has_l5 {
        let len = norm_strlen(m, l5).unwrap_or(0);
        let rc = norm_vt(
            m,
            h,
            out,
            NORM_OUT_SET_LAYER_X,
            &[ph0, ph1, 4, l5, len],
        );
        if rc >= 0x80000000 {
            return norm_cleanup(m, h, heap, p, recs, se, [l1, l2, l3, l4, l5], joined, rc);
        }
    }
    let rc = norm_vt(m, h, out, NORM_OUT_PROCESS, &[ph0, ph1, lit]);
    if rc >= 0x80000000 {
        return norm_cleanup(m, h, heap, p, recs, se, [l1, l2, l3, l4, l5], joined, 0);
    }
    let rc = norm_vt(m, h, out, NORM_OUT_GET_LAYER, &[ph0, ph1, 0, s_out1, s_len]);
    if rc >= 0x80000000 {
        return norm_cleanup(m, h, heap, p, recs, se, [l1, l2, l3, l4, l5], joined, rc);
    }
    let o1 = guest_word(m, s_out1).unwrap_or(0);
    let mut len = guest_half(m, s_len).unwrap_or(0) as u32;
    norm_put_byte(m, o1 + len, 0);
    let rc = norm_vt(m, h, out, NORM_OUT_GET_LAYER, &[ph0, ph1, 3, s_out4, s_len]);
    if rc >= 0x80000000 {
        return norm_cleanup(m, h, heap, p, recs, se, [l1, l2, l3, l4, l5], joined, rc);
    }
    let o4 = guest_word(m, s_out4).unwrap_or(0);
    len = guest_half(m, s_len).unwrap_or(0) as u32;
    norm_put_byte(m, o4 + len, 0);
    // The 0x16 markers of the fourth layer move into the first.
    let mut i = 0u32;
    while i < len {
        if guest_byte(m, o4 + i).unwrap_or(0) == 0x16 {
            norm_put_byte(m, o1 + i, 0x16);
        }
        i += 1;
    }
    // The 0x7e fillers are dropped from the first layer.
    let mut k = 0u32;
    while k < len {
        if guest_byte(m, o1 + k).unwrap_or(0) == b'~' {
            let mut j = k;
            while j < len {
                let v = guest_byte(m, o1 + j + 1).unwrap_or(0);
                norm_put_byte(m, o1 + j, v);
                j += 1;
            }
            len -= 1;
            norm_put_byte(m, o1 + len, 0);
            if k == 0 {
                continue;
            }
            k -= 1;
        }
        k += 1;
    }
    norm_put_byte(m, o1 + len, 0);

    // se = fn_1963b8(heap, 1, origLen + 1)
    se = h.call_m(0x1963b8, &[heap, 1, orig_len + 1], m);
    if se == 0 {
        return norm_cleanup(m, h, heap, p, recs, se, [l1, l2, l3, l4, l5], joined, 0x8b80200a);
    }
    if !norm_append(m, se, orig) {
        return norm_cleanup(m, h, heap, p, recs, se, [l1, l2, l3, l4, l5], joined, 0x8b80200a);
    }
    let mut last_end = norm_strlen(m, se).unwrap_or(0);
    let wbuf = match guest_word(m, p + 0x38) {
        Some(v) => v,
        None => {
            return norm_cleanup(m, h, heap, p, recs, se, [l1, l2, l3, l4, l5], joined, 0x8b80200a)
        }
    };
    for i in 0..0x80u32 {
        norm_put_byte(m, wbuf + i, 0);
    }
    let rc = norm_words(m, h, p, o1, wbuf, s_no1);
    if rc >= 0x80000000 {
        return norm_cleanup(m, h, heap, p, recs, se, [l1, l2, l3, l4, l5], joined, rc);
    }
    let rc = norm_words(m, h, p, orig, 0, s_norig);
    if rc >= 0x80000000 {
        return norm_cleanup(m, h, heap, p, recs, se, [l1, l2, l3, l4, l5], joined, rc);
    }

    // The word comparison pass.
    let mut delta = 0i32;
    let mut i = 0u32;
    let mut jj = 0u32;
    let mut w = 0u32;
    while i < orig_len {
        let ni = match norm_next_word(m, orig, orig_len, &mut i) {
            Some(v) => v,
            None => break,
        };
        let nj = match norm_next_word(m, o1, len, &mut jj) {
            Some(v) => v,
            None => break,
        };
        if norm_compare(m, orig + i, o1 + jj) != 0 {
            let mut tail = 0u32;
            let mut mk = norm_find_byte(m, o1 + jj, 0x1e);
            while mk != 0 {
                while node2 != 0 {
                    let rc = norm_vt(m, h, dp, NORM_DP_GET, &[h0, h1, node2, 0, 1, s_kind, s_cnt]);
                    if rc >= 0x80000000 {
                        return norm_cleanup(m, h, heap, p, recs, se, [l1, l2, l3, l4, l5], joined, rc);
                    }
                    let kind = guest_byte(m, s_kind).unwrap_or(0);
                    let rc = norm_vt(m, h, dp, NORM_DP_GET, &[h0, h1, node2, 1, 1, s_start, s_cnt]);
                    if rc >= 0x80000000 {
                        return norm_cleanup(m, h, heap, p, recs, se, [l1, l2, l3, l4, l5], joined, rc);
                    }
                    let start = guest_half(m, s_start).unwrap_or(0) as u32;
                    if start > i && kind == 7 {
                        tail = norm_find_byte(m, mk + 1, 0x1f);
                        if tail == 0 {
                            return norm_cleanup(
                                m,
                                h,
                                heap,
                                p,
                                recs,
                                se,
                                [l1, l2, l3, l4, l5],
                                joined,
                                0x8b802007,
                            );
                        }
                        norm_put_byte(m, tail, 0);
                        let mlen = norm_strlen(m, mk).unwrap_or(0);
                        norm_put_half(m, s_tmp, mlen as u16);
                        let rc = norm_vt(
                            m,
                            h,
                            dp,
                            NORM_DP_SET,
                            &[h0, h1, node2, 4, mlen, mk + 1, s_tmp],
                        );
                        if rc >= 0x80000000 {
                            return norm_cleanup(m, h, heap, p, recs, se, [l1, l2, l3, l4, l5], joined, rc);
                        }
                        let rc = norm_vt(m, h, dp, NORM_DP_NEXT, &[h0, h1, node2, s_node]);
                        if rc >= 0x80000000 {
                            return norm_cleanup(m, h, heap, p, recs, se, [l1, l2, l3, l4, l5], joined, rc);
                        }
                        node2 = guest_half(m, s_node).unwrap_or(0) as u32;
                        break;
                    }
                    let rc = norm_vt(m, h, dp, NORM_DP_NEXT, &[h0, h1, node2, s_node]);
                    if rc >= 0x80000000 {
                        return norm_cleanup(m, h, heap, p, recs, se, [l1, l2, l3, l4, l5], joined, rc);
                    }
                    node2 = guest_half(m, s_node).unwrap_or(0) as u32;
                }
                norm_put_byte(m, mk, 0);
                if tail == 0 {
                    return norm_cleanup(
                        m,
                        h,
                        heap,
                        p,
                        recs,
                        se,
                        [l1, l2, l3, l4, l5],
                        joined,
                        0x8b802007,
                    );
                }
                let n = norm_strlen(m, tail + 1).unwrap_or(0) + 1;
                norm_move(m, mk, tail + 1, n);
                mk = norm_find_byte(m, o1 + jj, 0x1e);
            }
            let lo1 = norm_strlen(m, o1 + jj).unwrap_or(0);
            let lorig = norm_strlen(m, orig + i).unwrap_or(0);
            let cur = norm_strlen(m, se).unwrap_or(0);
            if lo1 > lorig {
                let nl = lo1 - lorig + cur;
                let t = h.call_m(0x1963e0, &[heap, se, nl + 1], m);
                if t == 0 {
                    return norm_cleanup(m, h, heap, p, recs, se, [l1, l2, l3, l4, l5], joined, 0x8b80200a);
                }
                se = t;
                norm_put_byte(m, se + nl, 0);
            }
            if lo1 != lorig {
                let at = (delta as u32)
                    .wrapping_add(i)
                    .wrapping_add(lorig)
                    .wrapping_add(lo1.wrapping_sub(lorig));
                let from = (delta as u32).wrapping_add(i).wrapping_add(lorig);
                let count = last_end.wrapping_sub(i.wrapping_add(lorig));
                norm_move(m, se.wrapping_add(at), se.wrapping_add(from), count);
                crate::normalization::shift_word_records(
                    m.bytes,
                    recs,
                    n_recs,
                    i,
                    i.wrapping_add(lorig),
                    lo1.wrapping_sub(lorig) as i32,
                );
            }
            norm_move(m, se.wrapping_add((delta as u32).wrapping_add(i)), o1 + jj, lo1);
            while mi < n_markers {
                let cur_pos = guest_word(m, markers + mi * NORM_MARKER_LEN + 12).unwrap_or(0) as i32;
                let limit = if lorig > 1 {
                    i.wrapping_add(lorig).wrapping_sub(1) as i16 as i32
                } else {
                    i.wrapping_add(1) as i16 as i32
                };
                if (cur_pos.wrapping_sub(base) as i16 as i32) >= limit {
                    break;
                }
                norm_put_word(m, markers + mi * NORM_MARKER_LEN + 12, (cur_pos + delta) as u32);
                mi += 1;
            }
            delta = (delta + lo1 as i32 - lorig as i32) as i32;
            if lorig > lo1 {
                let mut z = cur;
                let mut q = 0u32;
                while q < lorig - lo1 {
                    z = z.wrapping_sub(1);
                    norm_put_byte(m, se + z, 0);
                    q += 1;
                }
            }
        }
        w += 1;
        i = ni;
        jj = nj;
    }

    // The final pass over the tree.
    let rc = norm_vt(m, h, dp, NORM_DP_FIRST, &[h0, h1, 2, root, s_node]);
    if rc >= 0x80000000 {
        return norm_cleanup(m, h, heap, p, recs, se, [l1, l2, l3, l4, l5], joined, rc);
    }
    node = guest_half(m, s_node).unwrap_or(0) as u32;
    let mut hy = 0u32;
    let mut n_o1 = 0u32;
    let mut prev_node = 0u32;
    if node != 0 {
        let base1 = se.wrapping_sub(1);
        loop {
            let rc = norm_vt(m, h, dp, NORM_DP_GET, &[h0, h1, node, 0, 1, s_kind, s_cnt]);
            if rc >= 0x80000000 {
                return norm_cleanup(m, h, heap, p, recs, se, [l1, l2, l3, l4, l5], joined, rc);
            }
            let kind = guest_byte(m, s_kind).unwrap_or(0);
            let mut a = 0u32;
            while a < n_recs {
                let rec = recs + a * NORM_RECORD_LEN;
                if guest_half(m, rec).unwrap_or(0) as u32 != node {
                    a += 1;
                    continue;
                }
                let nstart = guest_half(m, rec + 8).unwrap_or(0);
                let nend = guest_half(m, rec + 10).unwrap_or(0);
                norm_put_half(m, s_tmp, nstart);
                let rc = norm_vt(m, h, dp, NORM_DP_SET, &[h0, h1, node, 1, 1, s_tmp, s_tmp]);
                if rc >= 0x80000000 {
                    return norm_cleanup(m, h, heap, p, recs, se, [l1, l2, l3, l4, l5], joined, rc);
                }
                norm_put_half(m, s_tmp, nend);
                let rc = norm_vt(m, h, dp, NORM_DP_SET, &[h0, h1, node, 2, 1, s_tmp, s_tmp]);
                if rc >= 0x80000000 {
                    return norm_cleanup(m, h, heap, p, recs, se, [l1, l2, l3, l4, l5], joined, rc);
                }
                if kind == 4 || kind == 9 || kind == 10 {
                    let marker = guest_byte(m, wbuf + hy).unwrap_or(0) as u32;
                    if n_o1 != 0 && marker == n_o1 && prev_node != 0 {
                        norm_put_byte(m, base1 + guest_half(m, rec + 8).unwrap_or(0) as u32, b'-');
                        hy += 1;
                        let rc = norm_vt(m, h, dp, NORM_DP_GET_PTR, &[h0, h1, node, 3, s_f5, s_cnt]);
                        if rc >= 0x80000000 {
                            return norm_cleanup(m, h, heap, p, recs, se, [l1, l2, l3, l4, l5], joined, rc);
                        }
                        let s3 = guest_word(m, s_f5).unwrap_or(0);
                        let rc = norm_vt(m, h, dp, NORM_DP_GET_PTR, &[h0, h1, prev_node, 3, s_f6, s_cnt]);
                        if rc >= 0x80000000 {
                            return norm_cleanup(m, h, heap, p, recs, se, [l1, l2, l3, l4, l5], joined, rc);
                        }
                        let s3p = guest_word(m, s_f6).unwrap_or(0);
                        let n3 = norm_strlen(m, s3).unwrap_or(0);
                        let want = norm_strlen(m, s3p).unwrap_or(0) + n3 + 2;
                        let t = h.call_m(0x1963e0, &[heap, joined, want], m);
                        if t == 0 {
                            return norm_cleanup(m, h, heap, p, recs, se, [l1, l2, l3, l4, l5], joined, 0x8b80200a);
                        }
                        joined = t;
                        norm_append(m, joined, s3p);
                        norm_append(m, joined, sp);
                        norm_append(m, joined, s3);
                        norm_put_half(m, s_tmp, norm_strlen(m, joined).unwrap_or(0) as u16 + 1);
                        let rc = norm_vt(
                            m,
                            h,
                            dp,
                            NORM_DP_SET,
                            &[h0, h1, prev_node, 3, norm_strlen(m, joined).unwrap_or(0) + 1, joined, s_tmp],
                        );
                        if rc >= 0x80000000 {
                            return norm_cleanup(m, h, heap, p, recs, se, [l1, l2, l3, l4, l5], joined, rc);
                        }
                        norm_put_half(m, s_tmp, nend);
                        let rc = norm_vt(m, h, dp, NORM_DP_SET, &[h0, h1, prev_node, 2, 1, s_tmp, s_tmp]);
                        if rc >= 0x80000000 {
                            return norm_cleanup(m, h, heap, p, recs, se, [l1, l2, l3, l4, l5], joined, rc);
                        }
                        let rc = norm_vt(m, h, dp, NORM_DP_GET_TYPE, &[h0, h1, guest_half(m, rec).unwrap_or(0) as u32, 8, s_type]);
                        if rc < 0x80000000 && guest_half(m, s_type).unwrap_or(0) == 1 {
                            let rc = norm_vt(
                                m,
                                h,
                                dp,
                                NORM_DP_GET,
                                &[h0, h1, guest_half(m, rec).unwrap_or(0) as u32, 8, 1, s_end, s_cnt],
                            );
                            if rc >= 0x80000000 {
                                return norm_cleanup(m, h, heap, p, recs, se, [l1, l2, l3, l4, l5], joined, rc);
                            }
                            let v8 = guest_half(m, s_end).unwrap_or(0);
                            norm_put_half(m, s_tmp, v8);
                            let rc = norm_vt(m, h, dp, NORM_DP_SET, &[h0, h1, prev_node, 8, 1, s_tmp, s_tmp]);
                            if rc >= 0x80000000 {
                                return norm_cleanup(m, h, heap, p, recs, se, [l1, l2, l3, l4, l5], joined, rc);
                            }
                        }
                        let rc = norm_vt(m, h, dp, NORM_DP_REMOVE, &[h0, h1, node]);
                        if rc >= 0x80000000 {
                            return norm_cleanup(m, h, heap, p, recs, se, [l1, l2, l3, l4, l5], joined, rc);
                        }
                        node = prev_node;
                    }
                    let rc = norm_vt(m, h, dp, NORM_DP_GET_PTR, &[h0, h1, node, 3, s_f5, s_cnt]);
                    if rc >= 0x80000000 {
                        return norm_cleanup(m, h, heap, p, recs, se, [l1, l2, l3, l4, l5], joined, rc);
                    }
                    let s3 = guest_word(m, s_f5).unwrap_or(0);
                    if norm_compare(m, s3, empty) != 0 && norm_find_sub(m, s3, dash) != 0 {
                        let rc = norm_vt(m, h, dp, NORM_DP_GET, &[h0, h1, node, 1, 1, s_start, s_cnt]);
                        if rc >= 0x80000000 {
                            return norm_cleanup(m, h, heap, p, recs, se, [l1, l2, l3, l4, l5], joined, rc);
                        }
                        let start = guest_half(m, s_start).unwrap_or(0) as u32;
                        let rc = norm_vt(m, h, dp, NORM_DP_GET, &[h0, h1, node, 2, 1, s_end, s_cnt]);
                        if rc >= 0x80000000 {
                            return norm_cleanup(m, h, heap, p, recs, se, [l1, l2, l3, l4, l5], joined, rc);
                        }
                        let end = guest_half(m, s_end).unwrap_or(0) as u32;
                        let mut hp = norm_find_byte(m, se.wrapping_add(start), u32::from(b'-'));
                        while hp != 0 && hp < se + end {
                            norm_put_byte(m, hp, b'_');
                            hp = norm_find_byte(m, hp.wrapping_add(1), u32::from(b'-'));
                        }
                    }
                    n_o1 += 1;
                    prev_node = node;
                }
                a = n_recs;
            }
            let rc = norm_vt(m, h, dp, NORM_DP_NEXT, &[h0, h1, node, s_node]);
            if rc >= 0x80000000 {
                return norm_cleanup(m, h, heap, p, recs, se, [l1, l2, l3, l4, l5], joined, rc);
            }
            node = guest_half(m, s_node).unwrap_or(0) as u32;
            if node == 0 {
                break;
            }
        }
    }
    // dp->set(h, root, 0, len(se) + 1, se)
    let slen = norm_strlen(m, se).unwrap_or(0);
    norm_put_half(m, s_tmp, 0);
    let rc = norm_vt(m, h, dp, NORM_DP_SET, &[h0, h1, root, 0, slen + 1, se, s_tmp]);
    if rc >= 0x80000000 {
        return norm_cleanup(m, h, heap, p, recs, se, [l1, l2, l3, l4, l5], joined, rc);
    }
    // The markers move with the text.
    while mi < n_markers {
        let cur_pos = guest_word(m, markers + mi * NORM_MARKER_LEN + 12).unwrap_or(0) as i32;
        norm_put_word(m, markers + mi * NORM_MARKER_LEN + 12, (cur_pos + delta) as u32);
        mi += 1;
    }
    norm_put_word(m, markers + 16, slen);
    pos = (pos as i32 + delta) as i32;
    let rc = norm_vt(m, h, out, NORM_OUT_SET_POS, &[ph0, ph1, pos as u32, 0]);
    if rc >= 0x80000000 {
        return norm_cleanup(m, h, heap, p, recs, se, [l1, l2, l3, l4, l5], joined, rc);
    }
    norm_cleanup(m, h, heap, p, recs, se, [l1, l2, l3, l4, l5], joined, rc)
}

/// fn_1e7908's cleanup: release the guest buffers and clear the process slots.
fn norm_cleanup(
    m: &mut Memory<'_>,
    h: &mut impl Host,
    heap: u32,
    p: u32,
    recs: u32,
    se: u32,
    layers: [u32; 5],
    joined: u32,
    rc: u32,
) -> u32 {
    if joined != 0 {
        h.call_m(0x196428, &[heap, joined], m);
    }
    if recs != 0 {
        h.call_m(0x196428, &[heap, recs], m);
    }
    if se != 0 {
        h.call_m(0x196428, &[heap, se], m);
    }
    for (index, buffer) in layers.iter().enumerate() {
        if *buffer != 0 {
            h.call_m(0x196428, &[heap, *buffer], m);
            norm_put_word(m, p + 0x20 + (index as u32) * 4, 0);
        }
    }
    rc
}

/// fn_1ea35c: count the words of a guest string.
///
/// The separators (space, underscore and 0x16) are NULed in place. When a
/// marker buffer is given, every 0x16 byte records the word index that
/// follows it; the buffer grows by one 0x80 block when it is full.
fn norm_words(
    m: &mut Memory<'_>,
    h: &mut impl Host,
    p: u32,
    text: u32,
    wbuf: u32,
    out: u32,
) -> u32 {
    let mut n = 0u16;
    let mut count = 0u16;
    let mut raw = 0u32;
    while guest_byte(m, text.wrapping_add(raw)).unwrap_or(0) != 0 {
        raw = raw.wrapping_add(1);
    }
    let len = raw as u16;
    let mut i = 0u16;
    norm_put_half(m, out, 0);
    while i < len {
        let ch = guest_byte(m, text.wrapping_add(u32::from(i))).unwrap_or(0);
        if ch == b' ' || ch == b'_' || ch == 0x16 {
            if wbuf != 0 {
                if n >= 0x80 {
                    let base = (u32::from(n) >> 7) << 7;
                    let heap = guest_word(m, guest_word(m, p).unwrap_or(0).wrapping_add(4)).unwrap_or(0);
                    let old = guest_word(m, p.wrapping_add(0x38)).unwrap_or(0);
                    let rc = h.call_m(0x1963e0, &[heap, old, base + 0x80], m);
                    if rc == 0 {
                        return 0x8b80200a;
                    }
                    norm_put_word(m, p.wrapping_add(0x38), rc);
                    let mut index = 0u32;
                    while index < base + 0x80 {
                        norm_put_byte(m, rc.wrapping_add(index), 0);
                        index += 1;
                    }
                }
                if ch == 0x16 {
                    let buf = guest_word(m, p.wrapping_add(0x38)).unwrap_or(0);
                    norm_put_byte(m, buf.wrapping_add(u32::from(n)), count.wrapping_add(1) as u8);
                    n = n.wrapping_add(1);
                }
            }
            norm_put_byte(m, text.wrapping_add(u32::from(i)), 0);
            let mut k = i.wrapping_add(1);
            while k < len {
                let c = guest_byte(m, text.wrapping_add(u32::from(k))).unwrap_or(0);
                if c != b' ' && c != b'_' && c != 0x16 {
                    break;
                }
                norm_put_byte(m, text.wrapping_add(u32::from(k)), 0);
                k = k.wrapping_add(1);
            }
            count = count.wrapping_add(1);
            i = k.wrapping_sub(1);
        }
        i = i.wrapping_add(1);
    }
    if len != 0 && i != 0 && guest_byte(m, text.wrapping_add(u32::from(i)).wrapping_sub(1)).unwrap_or(0) != 0 {
        count = count.wrapping_add(1);
    }
    norm_put_half(m, out, count);
    0
}

/// The next word of a guest string, as fn_1e7908's nextWord does.
fn norm_next_word(m: &Memory<'_>, s: u32, len: u32, pos: &mut u32) -> Option<u32> {
    let mut end = *pos;
    while *pos < len {
        if guest_byte(m, s + *pos).unwrap_or(0) != 0 {
            end += 1;
            while end < len && guest_byte(m, s + end).unwrap_or(0) != 0 {
                end += 1;
            }
            while end + 1 < len && guest_byte(m, s + end + 1).unwrap_or(0) == 0 {
                end += 1;
            }
            return Some(end);
        }
        *pos += 1;
    }
    None
}
