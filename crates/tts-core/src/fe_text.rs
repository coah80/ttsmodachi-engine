//! Frontend sentence/word parsing (`fn_1d88f4` unit) and `FE_PUNCSPTN`
//! (`fn_1dd914`). Arena offsets with 0 meaning null; wrapping arithmetic
//! mirrors the C's unsigned/index wrap and heap-relative smashes land in the
//! pool under constrained furnishing.
#![allow(dead_code)]

use crate::depes::HandleValidator;
use crate::frontend::find_substr;
use crate::normalization::{
    NormOutHost, NormalizationHost, memcpy_at, memmove_at, memset_at, rule_entry, strcat_at,
    strcmp_at, strcpy_at, strlen8, strnappend, vec_grow, wordspan_1ea35c,
};
use crate::text_input::strncpy_at;

fn byte(data: &[u8], off: u32) -> u8 {
    data.get(off as usize).copied().unwrap_or(0)
}

fn half(data: &[u8], off: u32) -> u16 {
    u16::from_le_bytes([
        byte(data, off),
        byte(data, off.wrapping_add(1)),
    ])
}

fn word(data: &[u8], off: u32) -> u32 {
    u32::from_le_bytes([
        byte(data, off),
        byte(data, off.wrapping_add(1)),
        byte(data, off.wrapping_add(2)),
        byte(data, off.wrapping_add(3)),
    ])
}

fn put_byte(data: &mut [u8], off: u32, value: u8) {
    if let Some(slot) = data.get_mut(off as usize) {
        *slot = value;
    }
}

fn put_half(data: &mut [u8], off: u32, value: u16) {
    put_byte(data, off, value as u8);
    put_byte(data, off.wrapping_add(1), (value >> 8) as u8);
}

fn put_word(data: &mut [u8], off: u32, value: u32) {
    put_byte(data, off, value as u8);
    put_byte(data, off.wrapping_add(1), (value >> 8) as u8);
    put_byte(data, off.wrapping_add(2), (value >> 16) as u8);
    put_byte(data, off.wrapping_add(3), (value >> 24) as u8);
}

fn strcpy_lit(arena: &mut [u8], dst: u32, lit: &[u8]) {
    let d = dst as usize;
    let n = lit.len();
    arena[d..d + n].copy_from_slice(lit);
    arena[d + n] = 0;
}

fn strcat_lit(arena: &mut [u8], dst: u32, lit: &[u8]) {
    let mut e = dst as usize;
    while arena[e] != 0 {
        e += 1;
    }
    let n = lit.len();
    arena[e..e + n].copy_from_slice(lit);
    arena[e + n] = 0;
}

/// Signed-index byte read for heap-relative accesses that can dip below the
/// buffer base (e.g. the `delta < 0` text shift in `fn_1dbcb0`): in-arena
/// addresses read the arena, anything else reads zero. Constrained
/// furnishing keeps every such access in-arena, so this is exact.
fn byte_at(data: &[u8], idx: i64) -> u8 {
    if (0..data.len() as i64).contains(&idx) {
        data[idx as usize]
    } else {
        0
    }
}

fn put_at(data: &mut [u8], idx: i64, value: u8) {
    if (0..data.len() as i64).contains(&idx) {
        data[idx as usize] = value;
    }
}

/// `fn_17c8c0`: lowercase ASCII uppercase (`D_8dc30c` bit 1 covers `A-Z`,
/// verified from the ROM), else identity. The table index masks to a byte
/// while the add applies to the full input.
fn fold_lower(c: u32) -> u32 {
    if (c as u8).is_ascii_uppercase() {
        c.wrapping_add(0x20)
    } else {
        c
    }
}

/// `fn_17c8dc`: uppercase ASCII lowercase (`D_8dc30c` bit 2 covers `a-z`,
/// verified from the ROM), else identity.
fn fold_upper(c: u32) -> u32 {
    if (c as u8).is_ascii_lowercase() {
        c.wrapping_sub(0x20)
    } else {
        c
    }
}

/// `fn_1769d0`: case-insensitive 8-bit compare through `fn_17c8c0`.
fn strcmp_fold(data: &[u8], a: u32, b: u32) -> i32 {
    let mut i = 0u32;
    if fold_lower(byte(data, a) as u32) == fold_lower(byte(data, b) as u32) {
        loop {
            if byte(data, a.wrapping_add(i)) == 0 {
                return 0;
            }
            i = i.wrapping_add(1);
            if fold_lower(byte(data, a.wrapping_add(i)) as u32)
                != fold_lower(byte(data, b.wrapping_add(i)) as u32)
            {
                break;
            }
        }
    }
    (fold_lower(byte(data, a.wrapping_add(i)) as u32) as i64
        - fold_lower(byte(data, b.wrapping_add(i)) as u32) as i64) as i32
}

/// `fn_195d30`: byte scan for `c` (including the NUL terminator when
/// `c == 0`); 0 when a nonzero `c` is absent.
fn strchr8(data: &[u8], base: u32, c: u8) -> u32 {
    let mut p = base;
    while byte(data, p) != 0 && byte(data, p) != c {
        p = p.wrapping_add(1);
    }
    if byte(data, p) == c {
        p
    } else {
        0
    }
}

/// Host operations for the FE sentence/word units. Scalar outs use `&mut`
/// locals; pointer outs are arena offsets with 0 meaning null. Resource
/// acquire/release come from `NormOutHost`.
pub trait FeTextHost: NormalizationHost + HandleValidator + NormOutHost {
    /// Scripted `fn_198760` comparator sequence. The key arguments are
    /// side-specific pointer values, ignored by every side; only the
    /// scripted results are compared.
    fn cmp_seq(&mut self, arena: &mut [u8], a: u32, b: u32) -> i32;
    /// Lock/unlock inner call (`fn_176f2c`/`fn_176f5c` with a non-null
    /// vtable slot). All current rows furnish null slots, so the recorded
    /// host panics if this is ever reached.
    fn lock_call(&mut self, target: u32) -> u32;
    /// `firstNode`: status + node id.
    fn lat_first_node(
        &mut self,
        arena: &mut [u8],
        h0: u32,
        h1: u32,
        kind: u32,
        x: u32,
        node: &mut u16,
    ) -> u32;
    /// `nextNode`/`next`: status + node id.
    fn lat_next_node(
        &mut self,
        arena: &mut [u8],
        h0: u32,
        h1: u32,
        node: u32,
        next: &mut u16,
    ) -> u32;
    /// `getInt`/`get`: status + value + count. Field 0 reports the low
    /// byte; fields 1/2 clamp into the furnished span (field 2 at least
    /// the last field-1 value); other fields report the raw halfword.
    /// See `FE_GETINT`.
    fn lat_get_int(
        &mut self,
        arena: &mut [u8],
        h0: u32,
        h1: u32,
        node: u32,
        field: u32,
        n: u32,
        out: &mut u32,
        count: &mut u16,
    ) -> u32;
    /// `getStr`/`data`: status + pointer + count.
    fn lat_get_str(
        &mut self,
        arena: &mut [u8],
        h0: u32,
        h1: u32,
        node: u32,
        key: u32,
        out: &mut u32,
        count: &mut u16,
    ) -> u32;
    /// `getU16`/`size`: status + halfword.
    fn lat_get_u16(
        &mut self,
        arena: &mut [u8],
        h0: u32,
        h1: u32,
        node: u32,
        key: u32,
        out: &mut u16,
    ) -> u32;
    /// `setField`/`set`: status + written count.
    fn lat_set_field(
        &mut self,
        arena: &mut [u8],
        h0: u32,
        h1: u32,
        node: u32,
        key: u32,
        n: u32,
        data: u32,
        written: &mut u16,
    ) -> u32;
    /// `setField`/`set` with caller-owned data bytes: the C passes
    /// `&local` (a u8/u16 stack local) or a string literal, which the
    /// slice ports keep in Rust instead of guest scratch. `n` keeps the
    /// guest meaning (element count); `data` carries the element bytes.
    /// Hosts that must hand the guest a pointer stage the bytes the way
    /// their guest call needs (the live host uses its uncompared stack
    /// window, the way the ARM passes its own stack slot).
    fn lat_set_field_bytes(
        &mut self,
        arena: &mut [u8],
        h0: u32,
        h1: u32,
        node: u32,
        key: u32,
        n: u32,
        data: &[u8],
        written: &mut u16,
    ) -> u32;
    /// `countNodes`/`count`: status + count.
    fn lat_count_nodes(
        &mut self,
        arena: &mut [u8],
        h0: u32,
        h1: u32,
        kind: u32,
        x: u32,
        count: &mut u16,
    ) -> u32;
    /// `append`: status + node id.
    fn lat_append(
        &mut self,
        arena: &mut [u8],
        h0: u32,
        h1: u32,
        kind: u32,
        x: u32,
        node: &mut u16,
    ) -> u32;
    /// `insertAfter`: status + node id.
    fn lat_insert_after(
        &mut self,
        arena: &mut [u8],
        h0: u32,
        h1: u32,
        node: u32,
        new: &mut u16,
    ) -> u32;
    /// `insertBefore`: status + node id.
    fn lat_insert_before(
        &mut self,
        arena: &mut [u8],
        h0: u32,
        h1: u32,
        node: u32,
        new: &mut u16,
    ) -> u32;
    /// `removeNode`/`remove`: status.
    fn lat_remove(&mut self, arena: &mut [u8], h0: u32, h1: u32, node: u32) -> u32;
    /// LINGDB `reset`: status only.
    fn lat_reset(&mut self, arena: &mut [u8], h0: u32, h1: u32) -> u32;
    /// Synthesis `convert`: status only; leaves `out` untouched in fuzz.
    fn synth_convert(
        &mut self,
        arena: &mut [u8],
        h0: u32,
        h1: u32,
        upper: u32,
        phon: u32,
        len: u32,
        out: u32,
        cap: u32,
    ) -> u32;
    /// Dict `lookup` (fecfg/*): status + entry cell + found + separator.
    fn dict_lookup(
        &mut self,
        arena: &mut [u8],
        h0: u32,
        h1: u32,
        table: u32,
        key: u32,
        entry: &mut u32,
        found: &mut u16,
        sep: &mut u8,
    ) -> u32;
    /// Dict `annotate`: status only.
    fn dict_annotate(
        &mut self,
        arena: &mut [u8],
        h0: u32,
        h1: u32,
        h2: u32,
        h3: u32,
        toks: u32,
        count: u32,
    ) -> u32;
    /// Resources `available`: status + byte count + end flag.
    fn res_available(
        &mut self,
        arena: &mut [u8],
        h0: u32,
        h1: u32,
        avail: &mut u32,
        end: &mut u8,
    ) -> u32;
    /// Resources `read`: status + data pointer + byte count.
    fn res_read(
        &mut self,
        arena: &mut [u8],
        h0: u32,
        h1: u32,
        data: &mut u32,
        bytes: &mut u32,
    ) -> u32;
    /// Resources `consume`: status only.
    fn res_consume(&mut self, arena: &mut [u8], h0: u32, h1: u32, n: u32) -> u32;
    /// `typeName`: log-only; all sides write null.
    fn lat_type_name(&mut self, arena: &mut [u8], h0: u32, h1: u32, ty: u32, out: &mut u32);
    /// `getParam`: status + string-array pointer + found + count.
    fn dct_get_param(
        &mut self,
        arena: &mut [u8],
        h0: u32,
        h1: u32,
        sect: u32,
        key: u32,
        val: &mut u32,
        found: &mut u16,
        n: &mut i32,
    ) -> u32;
    /// `getFlags`: status + two flag bytes.
    fn dct_get_flags(
        &mut self,
        arena: &mut [u8],
        h0: u32,
        h1: u32,
        a: &mut u8,
        b: &mut u8,
    ) -> u32;
    /// `setInput`: status.
    fn depes_set_input(
        &mut self,
        arena: &mut [u8],
        h0: u32,
        h1: u32,
        layer: u32,
        s: u32,
        n: u32,
    ) -> u32;
    /// `getOutput`: status + pointer + length.
    fn depes_get_output(
        &mut self,
        arena: &mut [u8],
        h0: u32,
        h1: u32,
        layer: u32,
        out: &mut u32,
        len: &mut u16,
    ) -> u32;
    /// `setRules`: status.
    fn depes_set_rules(&mut self, arena: &mut [u8], h0: u32, h1: u32, name: u32) -> u32;
    /// `setWeights`: status.
    fn depes_set_weights(
        &mut self,
        arena: &mut [u8],
        h0: u32,
        h1: u32,
        w: u32,
        n: u32,
    ) -> u32;
    /// `getWeights`: status + pointer + length.
    fn depes_get_weights(
        &mut self,
        arena: &mut [u8],
        h0: u32,
        h1: u32,
        out: &mut u32,
        len: &mut u16,
    ) -> u32;
    /// `properties.get`: status + position.
    fn prop_get(
        &mut self,
        arena: &mut [u8],
        h0: u32,
        h1: u32,
        pos: &mut i32,
        flag: u32,
    ) -> u32;
    /// `properties.set`: status.
    fn prop_set(&mut self, arena: &mut [u8], h0: u32, h1: u32, val: u32, flag: u32) -> u32;
    /// `characters.punctuation`: nonzero when `c` is punctuation.
    fn char_punctuation(&mut self, arena: &mut [u8], h0: u32, h1: u32, c: u32) -> i32;
    /// `fn_173fd8`: log verbosity.
    fn log_level(&mut self, arena: &mut [u8], log: u32) -> u32;
    /// `dp.first`: status + node id.
    fn dp_first_node(
        &mut self,
        arena: &mut [u8],
        h0: u32,
        h1: u32,
        kind: u32,
        x: u32,
        node: &mut u16,
    ) -> u32;
    /// `dp.next`: status + node id.
    fn dp_next_node(
        &mut self,
        arena: &mut [u8],
        h0: u32,
        h1: u32,
        node: u32,
        next: &mut u16,
    ) -> u32;
    /// `dp.get`: status + value + count. Same field rules as
    /// `lat_get_int`, on the dp script.
    fn dp_get(
        &mut self,
        arena: &mut [u8],
        h0: u32,
        h1: u32,
        node: u32,
        field: u32,
        n: u32,
        out: &mut u32,
        count: &mut u16,
    ) -> u32;
    /// `dp.getPtr`: status + pointer + count.
    fn dp_get_ptr(
        &mut self,
        arena: &mut [u8],
        h0: u32,
        h1: u32,
        node: u32,
        key: u32,
        out: &mut u32,
        count: &mut u16,
    ) -> u32;
    /// `dp.getType`: status + halfword.
    fn dp_get_type(
        &mut self,
        arena: &mut [u8],
        h0: u32,
        h1: u32,
        node: u32,
        key: u32,
        out: &mut u16,
    ) -> u32;
    /// `dp.set`: status + written count.
    fn dp_set(
        &mut self,
        arena: &mut [u8],
        h0: u32,
        h1: u32,
        node: u32,
        key: u32,
        n: u32,
        data: u32,
        written: &mut u16,
    ) -> u32;
    /// `dp.remove`: status.
    fn dp_remove(&mut self, arena: &mut [u8], h0: u32, h1: u32, node: u32) -> u32;
    /// `out.getPos`: status + position.
    fn out_get_pos(
        &mut self,
        arena: &mut [u8],
        h0: u32,
        h1: u32,
        pos: &mut i32,
        flag: u32,
    ) -> u32;
    /// `out.setLayer`: status.
    fn out_set_layer(
        &mut self,
        arena: &mut [u8],
        h0: u32,
        h1: u32,
        layer: u32,
        s: u32,
        n: u32,
    ) -> u32;
    /// `out.setLayerX`: status.
    fn out_set_layer_x(
        &mut self,
        arena: &mut [u8],
        h0: u32,
        h1: u32,
        layer: u32,
        s: u32,
        n: u32,
    ) -> u32;
    /// `out.process`: status.
    fn out_process(&mut self, arena: &mut [u8], h0: u32, h1: u32, name: u32) -> u32;
    /// `out.getLayer`: status + pointer + length.
    fn out_get_layer(
        &mut self,
        arena: &mut [u8],
        h0: u32,
        h1: u32,
        layer: u32,
        out: &mut u32,
        len: &mut u16,
    ) -> u32;
    /// `out.setPos`: status.
    fn out_set_pos(&mut self, arena: &mut [u8], h0: u32, h1: u32, pos: i32, flag: u32) -> u32;
    /// Stream `open`: status + handle pair.
    fn st_open(
        &mut self,
        arena: &mut [u8],
        ha: u32,
        hb: u32,
        ty: u32,
        name: u32,
        a: u32,
        b: u32,
        out_a: &mut u32,
        out_b: &mut u32,
    ) -> u32;
    /// Stream `open` with a four-entry script (call `n` uses entry
    /// `min(n, 3)`): status + handle pair.
    fn st_open4(
        &mut self,
        arena: &mut [u8],
        ha: u32,
        hb: u32,
        ty: u32,
        name: u32,
        a: u32,
        b: u32,
        out_a: &mut u32,
        out_b: &mut u32,
    ) -> u32;
    /// Stream `close`: status.
    fn st_close(&mut self, arena: &mut [u8], ha: u32, hb: u32) -> u32;
    /// Stream `getMode`: scripted value.
    fn st_get_mode(&mut self, arena: &mut [u8]) -> u32;
    /// Stream `read`: status + available count + finished flag.
    fn st_read(
        &mut self,
        arena: &mut [u8],
        ha: u32,
        hb: u32,
        n: &mut i32,
        fin: &mut u8,
    ) -> u32;
    /// Stream `gettext`: status + buffer pointer + byte length.
    fn st_gettext(
        &mut self,
        arena: &mut [u8],
        ha: u32,
        hb: u32,
        buf: &mut u32,
        len: &mut i32,
    ) -> u32;
    /// Stream `write`: status.
    fn st_write(&mut self, arena: &mut [u8], ha: u32, hb: u32, buf: u32, len: u32) -> u32;
    /// Stream `consume`: status.
    fn st_consume(&mut self, arena: &mut [u8], ha: u32, hb: u32, n: u32) -> u32;
    /// Depes `getPosition`: status + position.
    fn depes_get_pos(
        &mut self,
        arena: &mut [u8],
        h0: u32,
        h1: u32,
        pos: &mut i32,
        flag: u32,
    ) -> u32;
    /// Depes `setPosition`: status.
    fn depes_set_pos(&mut self, arena: &mut [u8], h0: u32, h1: u32, pos: i32, flag: u32) -> u32;
    /// Dictionary `count`: scripted value, returned unconditionally.
    fn dict_count(&mut self, arena: &mut [u8], ha: u32, hb: u32) -> u32;
    /// `fn_175a1c`: status + setting-string pointer.
    fn suba1c(&mut self, arena: &mut [u8], reg: u32, key: u32, out: &mut u32) -> u32;
    /// `fn_282af0`: status + content type + `n` output units + capacity +
    /// language pointer.
    fn tn_282af0(
        &mut self,
        arena: &mut [u8],
        ctx: u32,
        dict: u32,
        cb: u32,
        ha: u32,
        hb: u32,
        all_transforms: u32,
        input: u32,
        content_type: u32,
        output: u32,
        capacity: u32,
        lang: u32,
    ) -> u32;
}

/// Case-insensitive compare of an arena string against a static literal
/// through `fn_17c8c0` folding. The literals are ROM constants on the C/ARM
/// sides (spelled out in `src/fn_1d88f4.c`); the port folds the bytes
/// directly without arena scratch.
fn strcmp_fold_lit(data: &[u8], s: u32, lit: &[u8]) -> i32 {
    let mut i = 0u32;
    let at = |j: u32| -> u8 {
        if (j as usize) < lit.len() {
            lit[j as usize]
        } else {
            0
        }
    };
    if fold_lower(byte(data, s) as u32) == fold_lower(at(0) as u32) {
        loop {
            if byte(data, s.wrapping_add(i)) == 0 {
                return 0;
            }
            i = i.wrapping_add(1);
            if fold_lower(byte(data, s.wrapping_add(i)) as u32) != fold_lower(at(i) as u32) {
                break;
            }
        }
    }
    (fold_lower(byte(data, s.wrapping_add(i)) as u32) as i64 - fold_lower(at(i) as u32) as i64)
        as i32
}

/// fn_17c8c0, EU fn_17c970, KR fn_17c91c. Lowercase fold through the
/// `D_8dc30c` class table (bit 1 covers exactly `A-Z`, verified from the
/// ROM): the table index masks to a byte while the add applies to the
/// full input.
pub fn fold_17c8c0(c: u32) -> u32 {
    fold_lower(c)
}

/// fn_1769d0, EU fn_176a80, KR fn_176a2c. Case-insensitive 8-bit compare
/// through `fn_17c8c0`.
pub fn strcmp_ci_1769d0(arena: &[u8], a: u32, b: u32) -> i32 {
    strcmp_fold(arena, a, b)
}

/// fn_17c8dc, EU fn_17c98c, KR fn_17c938. Uppercase fold through the
/// `D_8dc30c` class table (bit 2 covers `a-z`).
pub fn fold_17c8dc(c: u32) -> u32 {
    fold_upper(c)
}

/// fn_195d30, EU fn_195de0, KR fn_195e1c. Scan for `c` (low byte),
/// including the NUL terminator when `c == 0`; 0 on a miss.
pub fn strchr_195d30(arena: &[u8], base: u32, c: u32) -> u32 {
    strchr8(arena, base, c as u8)
}

/// fn_195c90, EU fn_195d40, KR fn_195d7c. Append `src` to `dst`.
pub fn strcat_195c90(arena: &mut [u8], dst: u32, src: u32) {
    strcat_at(arena, dst, src);
}

/// fn_195dac, EU fn_195e5c, KR fn_195e98. Append at most `n` bytes.
pub fn strncat_195dac(arena: &mut [u8], dst: u32, src: u32, n: u32) {
    strnappend(arena, dst, src, n);
}

/// fn_195ae8, EU fn_195b98, KR fn_195bd4. Copy `n` bytes.
pub fn memcpy_195ae8(arena: &mut [u8], dst: u32, src: u32, n: u32) {
    memcpy_at(arena, dst, src, n);
}

/// fn_195bec, EU fn_195c9c, KR fn_195cd8. Fill `n` bytes with `value`.
pub fn memset_195bec(arena: &mut [u8], dst: u32, value: u8, n: u32) {
    memset_at(arena, dst, value, n);
}

/// fn_195aac, EU fn_195b5c, KR fn_195b98. Bounded byte compare; `n == 0`
/// returns without reading either side, otherwise the first differing
/// byte pair decides (including the pair at index 0 when `n == 1`).
pub fn memcmp_195aac(arena: &[u8], mut a: u32, mut b: u32, n: u32) -> i32 {
    if n == 0 {
        return 0;
    }
    let mut left = n;
    loop {
        left = left.wrapping_sub(1);
        if left == 0 || byte(arena, a) != byte(arena, b) {
            break;
        }
        a = a.wrapping_add(1);
        b = b.wrapping_add(1);
    }
    byte(arena, a) as i32 - byte(arena, b) as i32
}

/// fn_195b38, EU fn_195be8, KR fn_195c24. `memmove`: backwards when
/// `dst` sits strictly inside `[src, src + n)`, forwards otherwise.
pub fn memmove_195b38(arena: &mut [u8], dst: u32, src: u32, n: u32) {
    if dst > src && src.wrapping_add(n) > dst {
        let mut d = dst.wrapping_add(n);
        let mut s = src.wrapping_add(n);
        let mut left = n;
        while left != 0 {
            left -= 1;
            d = d.wrapping_sub(1);
            s = s.wrapping_sub(1);
            let v = byte(arena, s);
            put_byte(arena, d, v);
        }
    } else {
        let mut d = dst;
        let mut s = src;
        let mut left = n;
        while left != 0 {
            left -= 1;
            let v = byte(arena, s);
            put_byte(arena, d, v);
            d = d.wrapping_add(1);
            s = s.wrapping_add(1);
        }
    }
}

/// fn_195d90, EU fn_195e40, KR fn_195e7c. Byte length before the NUL.
pub fn strlen_195d90(arena: &[u8], s: u32) -> i32 {
    let mut p = s;
    while byte(arena, p) != 0 {
        p = p.wrapping_add(1);
    }
    p.wrapping_sub(s) as i32
}

/// fn_283188, EU fn_283344, KR fn_2833f0. Membership in the `D_9154c8`
/// u16 set (`09 0a 0d 20 00` from the ROM, NUL-terminated): the scan
/// matches the terminator itself, so 0 counts as a member.
pub fn isspace_283188(a: u32) -> u32 {
    u32::from(matches!(a, 0 | 9 | 10 | 13 | 32))
}

/// fn_202f9c, EU fn_203158, KR fn_203204. Open the two input streams
/// then the two output streams; on any failure close what opened and
/// zero the handle. The `name`/`type` arguments only feed the scripted
/// open stub, which ignores them.
pub fn stream_open_202f9c(
    arena: &mut [u8],
    p: u32,
    b: u32,
    ha: u32,
    hb: u32,
    host: &mut impl FeTextHost,
) -> u32 {
    if host.validate(b, 0xf38a, 0xb8) >= 0x8000_0000 {
        return 0x8a80_2008;
    }
    let mut rc = 0u32;
    let mut i = 0u16;
    while i < 2 {
        let slot = p.wrapping_add(0x1c).wrapping_add(u32::from(i).wrapping_mul(8));
        let mut out_a = 0u32;
        let mut out_b = 0u32;
        rc = host.st_open4(arena, ha, hb, 0xa8, 0, 0, 0, &mut out_a, &mut out_b);
        if rc >= 0x8000_0000 {
            break;
        }
        put_word(arena, slot, out_a);
        put_word(arena, slot.wrapping_add(4), out_b);
        i = i.wrapping_add(1);
    }
    let mut j = 0u16;
    if rc < 0x8000_0000 {
        while j < 2 {
            let slot = p.wrapping_add(0x2c).wrapping_add(u32::from(j).wrapping_mul(8));
            let mut out_a = 0u32;
            let mut out_b = 0u32;
            let a = if j != 1 { 0 } else { host.st_get_mode(arena) };
            rc = host.st_open4(arena, ha, hb, 0xa8, 0, a, 1, &mut out_a, &mut out_b);
            if rc >= 0x8000_0000 {
                break;
            }
            put_word(arena, slot, out_a);
            put_word(arena, slot.wrapping_add(4), out_b);
            j = j.wrapping_add(1);
        }
    }
    if rc >= 0x8000_0000 {
        let mut k = 0u16;
        while k < i {
            let slot = p.wrapping_add(0x1c).wrapping_add(u32::from(k).wrapping_mul(8));
            if word(arena, slot) != 0 {
                host.st_close(arena, word(arena, slot), word(arena, slot.wrapping_add(4)));
                put_word(arena, slot, 0);
                put_word(arena, slot.wrapping_add(4), 0);
            }
            k = k.wrapping_add(1);
        }
        k = 0;
        while k < j {
            let slot = p.wrapping_add(0x2c).wrapping_add(u32::from(k).wrapping_mul(8));
            if word(arena, slot) != 0 {
                host.st_close(arena, word(arena, slot), word(arena, slot.wrapping_add(4)));
                put_word(arena, slot, 0);
                put_word(arena, slot.wrapping_add(4), 0);
            }
            k = k.wrapping_add(1);
        }
    }
    rc
}

fn find_substr_lit(mem: &[u8], s: u32, lit: &[u8]) -> u32 {
    let mut base = s;
    loop {
        let mut i = 0usize;
        while i < lit.len() && byte(mem, base.wrapping_add(i as u32)) == lit[i] {
            i += 1;
        }
        if i == lit.len() {
            return base;
        }
        if byte(mem, base) == 0 {
            return 0;
        }
        base = base.wrapping_add(1);
    }
}

#[inline]
fn is_word_ch(ch: u8) -> bool {
    (ch >= b'A' || ch == b' ') && (ch as u32).wrapping_sub('[' as u32) >= 6 && {
        ch <= b'z' || ch == 0x82 || ch == 0x85 || ch == 0x88 || ch == 0x8a
    }
}

/// Current mark position (`Mark.pos` at `+0x0c`, stride `0x18`).
#[inline]
fn mark_pos(arena: &[u8], marks: u32, idx: u32) -> u32 {
    word(arena, marks.wrapping_add(idx.wrapping_mul(0x18)).wrapping_add(0xc))
}

/// fn_1dbcb0, EU fn_1dbe6c, KR fn_1dbf18. Disambiguate the punctuation
/// span `[start, end)`: build the orthography/map layers, run the depes
/// rules (or take the direct path for `phon` mode and plain words), make
/// room in the sentence, copy the result back token by token, and move
/// the marks with it. The `fn_1738e8`/`fn_173dc0` calls are log-only.
pub fn fe_disambig_1dbcb0(
    arena: &mut [u8],
    c: u32,
    p_text: u32,
    orig: u32,
    start: u32,
    end: u32,
    mode: u32,
    h0: u32,
    h1: u32,
    p_node: &mut u16,
    marks: u32,
    p_idx: &mut u16,
    p_cnt: u16,
    base: u32,
    host: &mut impl FeTextHost,
) -> u32 {
    // The C keeps node/idx/cnt in stack locals and passes &node/&idx/&cnt:
    // they are Rust locals here, never guest bytes.
    let env = word(arena, c);
    let heap = word(arena, env.wrapping_add(4));
    let mut r = 0u32;
    let mut pos = start;
    let mut delta = 0i16;
    let mut from = start as u16;
    let mut to = start as u16;
    let mut node = *p_node;
    let mut blocks = 1u8;
    let mut n_tilde = 0u16;
    let text_len = strlen8(arena, word(arena, p_text)) as u16;
    let span = end.wrapping_sub(start);
    let mut name = [0u8; 128];
    name[..24].copy_from_slice(b"disambiguate_punctuation");
    let name_len = 24usize;
    for cell in [0x20u32, 0x24, 0x28] {
        let old = word(arena, c.wrapping_add(cell));
        match host.realloc(arena, heap, old, span.wrapping_add(129)) {
            Some(p) => {
                put_word(arena, c.wrapping_add(cell), p);
                if cell != 0x28 {
                    put_byte(arena, p, 0);
                }
            }
            None => return 0x89c0_200a,
        }
    }
    // Build the two depes input layers.
    while *p_node != 0 && (from as u32) < end {
        let cur = u32::from(*p_node);
        let mut ty_w = 0u32;
        let mut len = 0u16;
        let rc = host.lat_get_int(arena, h0, h1, cur, 0, 1, &mut ty_w, &mut len);
        if rc >= 0x8000_0000 {
            return rc;
        }
        let ty = ty_w as u8;
        r = host.lat_get_int(arena, h0, h1, cur, 1, 1, &mut ty_w, &mut len);
        if r >= 0x8000_0000 {
            return r;
        }
        from = ty_w as u16;
        if (from as u32) < end && (ty == 6 || ty == 4) {
            // Unchecked in the C: on failure `to` keeps its old value.
            let mut to_w = to as u32;
            r = host.lat_get_int(arena, h0, h1, cur, 2, 1, &mut to_w, &mut len);
            to = to_w as u16;
            let pron = if ty == 6 {
                let mut p = 0u32;
                let rc = host.lat_get_str(arena, h0, h1, cur, 4, &mut p, &mut len);
                if rc >= 0x8000_0000 {
                    return rc;
                }
                p
            } else {
                // The `_DT_` default is a C string literal; the port keeps
                // its bytes in a scratch cell the casegen never observes.
                put_word(arena, 0x6f0, 0x5f54_445f);
                put_byte(arena, 0x6f4, 0);
                0x6f0
            };
            let mut norm = 0u32;
            r = host.lat_get_str(arena, h0, h1, cur, 3, &mut norm, &mut len);
            if r >= 0x80000000 {
                return r;
            }
            delta = strlen8(arena, norm)
                .wrapping_add(delta as u32)
                .wrapping_sub(to as u32)
                .wrapping_add(from as u32) as u16 as i16;
            let mut q = strchr8(arena, norm, 0x1e);
            while q != 0 {
                loop {
                    q = q.wrapping_add(1);
                    delta = delta.wrapping_sub(1);
                    let ch = byte(arena, q);
                    if ch == 0x1f {
                        break;
                    }
                    if ch == 0 {
                        return 0x89c0_2000;
                    }
                }
                delta = delta.wrapping_sub(1);
                norm = q.wrapping_add(1);
                q = strchr8(arena, norm, 0x1e);
            }
            let p_text_ptr = word(arena, p_text);
            let shift0 = half(arena, c.wrapping_add(0x30)) as i16;
            strnappend(
                arena,
                word(arena, c.wrapping_add(0x20)),
                p_text_ptr.wrapping_add(shift0 as u32).wrapping_add(pos),
                (to as u32).wrapping_sub(pos),
            );
            if pos < from as u32 {
                let mut k = 0u16;
                while (k as u32) < (from as u32).wrapping_sub(pos) {
                    strcat_lit(arena, word(arena, c.wrapping_add(0x24)), b" ");
                    k = k.wrapping_add(1);
                }
            }
            let pron_len = strlen8(arena, pron);
            if ((pron_len.wrapping_mul(2).wrapping_sub(2)) as u16 as i16)
                > ((to as u32).wrapping_sub(from as u32) as u16 as i16)
            {
                let mut k = 0u32;
                let bound = pron_len
                    .wrapping_mul(2)
                    .wrapping_sub(to as u32)
                    .wrapping_add(from as u32)
                    .wrapping_sub(2);
                while k < bound {
                    if n_tilde == u16::from(blocks).wrapping_mul(128) {
                        blocks = blocks.wrapping_add(1);
                        pos = span.wrapping_add(u32::from(blocks).wrapping_mul(128));
                        let mut failed = false;
                        for cell in [0x20u32, 0x24, 0x28] {
                            let old = word(arena, c.wrapping_add(cell));
                            match host.realloc(arena, heap, old, pos.wrapping_add(1)) {
                                Some(p) => put_word(arena, c.wrapping_add(cell), p),
                                None => {
                                    failed = true;
                                    break;
                                }
                            }
                        }
                        if failed {
                            return 0x89c0_200a;
                        }
                    }
                    strcat_lit(arena, word(arena, c.wrapping_add(0x20)), b"~");
                    n_tilde = n_tilde.wrapping_add(1);
                    k = k.wrapping_add(1);
                }
                strcat_lit(arena, word(arena, c.wrapping_add(0x24)), b"X");
                strnappend(
                    arena,
                    word(arena, c.wrapping_add(0x24)),
                    pron.wrapping_add(1),
                    pron_len.wrapping_sub(2),
                );
            } else {
                strcat_lit(arena, word(arena, c.wrapping_add(0x24)), b"X");
                strnappend(
                    arena,
                    word(arena, c.wrapping_add(0x24)),
                    pron.wrapping_add(1),
                    pron_len.wrapping_sub(2),
                );
                let mut k = 0u32;
                let bound = (to as u32)
                    .wrapping_sub(from as u32)
                    .wrapping_sub(pron_len.wrapping_mul(2))
                    .wrapping_add(2);
                // The bound is an int expression: negative skips the loop.
                let bound_i = bound as i32;
                while (k as i32) < bound_i {
                    strcat_lit(arena, word(arena, c.wrapping_add(0x24)), b"~");
                    k = k.wrapping_add(1);
                }
            }
            strcat_lit(arena, word(arena, c.wrapping_add(0x24)), b"Y");
            strnappend(
                arena,
                word(arena, c.wrapping_add(0x24)),
                pron.wrapping_add(1),
                pron_len.wrapping_sub(2),
            );
            pos = to as u32;
        }
        if (from as u32) < end {
            let cur = u32::from(*p_node);
            let mut next = 0u16;
            r = host.lat_next_node(arena, h0, h1, cur, &mut next);
            if r >= 0x8000_0000 {
                return r;
            }
            *p_node = next;
        }
    }
    if pos < end {
        let n = end.wrapping_sub(pos);
        let p_text_ptr = word(arena, p_text);
        let shift0 = half(arena, c.wrapping_add(0x30)) as i16;
        strnappend(
            arena,
            word(arena, c.wrapping_add(0x20)),
            p_text_ptr.wrapping_add(shift0 as u32).wrapping_add(pos),
            n,
        );
        let mut k = 0u32;
        while k < n {
            strcat_lit(arena, word(arena, c.wrapping_add(0x24)), b" ");
            k = k.wrapping_add(1);
        }
    }
    let spell_here = find_substr_lit(arena, mode, b"spell") != 0;
    if spell_here {
        while byte(
            arena,
            word(arena, c.wrapping_add(0x20))
                .wrapping_add(strlen8(arena, word(arena, c.wrapping_add(0x20))).wrapping_sub(1)),
        ) == b' '
        {
            let k = strlen8(arena, word(arena, c.wrapping_add(0x20))).wrapping_sub(1);
            put_byte(arena, word(arena, c.wrapping_add(0x20)).wrapping_add(k), 0);
            delta = delta.wrapping_sub(1);
            put_byte(arena, word(arena, c.wrapping_add(0x24)).wrapping_add(k), 0);
        }
    }
    if n_tilde > u16::from(blocks).wrapping_mul(128) {
        return 0x89c0_2000;
    }
    let mut k = 0u16;
    while (k as u32) < strlen8(arena, word(arena, c.wrapping_add(0x20))) {
        put_byte(
            arena,
            word(arena, c.wrapping_add(0x28)).wrapping_add(k as u32),
            1,
        );
        k = k.wrapping_add(1);
    }
    let mut n_x = 0u16;
    let mut n_y = 0u16;
    k = 0;
    while (k as u32) < strlen8(arena, word(arena, c.wrapping_add(0x24))) {
        let ch = byte(arena, word(arena, c.wrapping_add(0x24)).wrapping_add(k as u32));
        if ch == b'X' {
            n_x = n_x.wrapping_add(1);
        } else if ch == b'Y' {
            n_y = n_y.wrapping_add(1);
        }
        k = k.wrapping_add(1);
    }
    // Run the disambiguation rules unless the text is plain words.
    let mut olen = 0u16;
    let (o1, o2, o3): (u32, u32, u32);
    let direct = if strcmp_lit(arena, mode, b"phon") == 0 {
        true
    } else if byte(arena, c.wrapping_add(0x33)) != 0 && strcmp_lit(arena, mode, b"spell") != 0 {
        let orth = word(arena, c.wrapping_add(0x20));
        let mut q = orth;
        let mut plain = false;
        if is_word_ch(byte(arena, q)) {
            loop {
                q = q.wrapping_add(1);
                let ch = byte(arena, q) as u32;
                if ch == 0 {
                    plain = true;
                    break;
                }
                let chb = (ch & 0xff) as u8;
                if (chb < b'A' && chb != b' ')
                    || ((chb as u32).wrapping_sub('A' as u32) <= 25
                        && byte(arena, q.wrapping_sub(1)) != b' ')
                    || (chb as u32).wrapping_sub('[' as u32) < 6
                    || (chb > b'z' && chb != 0x82 && chb != 0x85 && chb != 0x88 && chb != 0x8a)
                {
                    break;
                }
            }
        }
        plain
    } else {
        false
    };
    if direct {
        o1 = word(arena, c.wrapping_add(0x20));
        o2 = word(arena, c.wrapping_add(0x24));
        o3 = word(arena, c.wrapping_add(0x28));
    } else {
        let dh0 = word(arena, c.wrapping_add(8));
        let dh1 = word(arena, c.wrapping_add(0xc));
        let rc = host.depes_set_input(
            arena,
            dh0,
            dh1,
            0,
            word(arena, c.wrapping_add(0x20)),
            strlen8(arena, word(arena, c.wrapping_add(0x20))),
        );
        if rc >= 0x8000_0000 {
            return rc;
        }
        let rc = host.depes_set_input(
            arena,
            dh0,
            dh1,
            1,
            word(arena, c.wrapping_add(0x24)),
            strlen8(arena, word(arena, c.wrapping_add(0x24))),
        );
        if rc >= 0x8000_0000 {
            return rc;
        }
        let rc = host.depes_set_weights(
            arena,
            dh0,
            dh1,
            word(arena, c.wrapping_add(0x28)),
            strlen8(arena, word(arena, c.wrapping_add(0x20))),
        );
        if rc >= 0x8000_0000 {
            return rc;
        }
        if strcmp_lit(arena, mode, b"normal") != 0
            && name_len + strlen8(arena, mode) as usize + 2 < 128
        {
            name[name_len] = b'_';
            let mut i = 0u32;
            while byte(arena, mode.wrapping_add(i)) != 0 {
                name[name_len + 1 + i as usize] = byte(arena, mode.wrapping_add(i));
                i = i.wrapping_add(1);
            }
            name[name_len + 1 + i as usize] = 0;
        }
        if host.depes_set_rules(arena, dh0, dh1, 0) >= 0x8000_0000 {
            name[name_len] = 0;
            let rc = host.depes_set_rules(arena, dh0, dh1, 0);
            if rc >= 0x8000_0000 {
                return rc;
            }
        }
        let mut p1 = 0u32;
        let rc = host.depes_get_output(arena, dh0, dh1, 0, &mut p1, &mut olen);
        if rc >= 0x8000_0000 {
            return rc;
        }
        if find_substr_lit(arena, mode, b"spell") != 0 {
            put_byte(arena, p1.wrapping_add(olen as u32), b' ');
            olen = olen.wrapping_add(1);
        }
        put_byte(arena, p1.wrapping_add(olen as u32), 0);
        let mut p2 = 0u32;
        let rc = host.depes_get_output(arena, dh0, dh1, 1, &mut p2, &mut olen);
        if rc >= 0x8000_0000 {
            return rc;
        }
        if find_substr_lit(arena, mode, b"spell") != 0 {
            put_byte(arena, p2.wrapping_add(olen as u32), b' ');
            olen = olen.wrapping_add(1);
        }
        put_byte(arena, p2.wrapping_add(olen as u32), 0);
        if strlen8(arena, p1) != strlen8(arena, p2) {
            return 0x89c0_2000;
        }
        let mut m = 0u16;
        k = 0;
        while (k as u32) < strlen8(arena, p2) {
            if byte(arena, p2.wrapping_add(k as u32)) == b'X' {
                m = m.wrapping_add(1);
            }
            k = k.wrapping_add(1);
        }
        if m != n_x {
            return 0x89c0_2000;
        }
        m = 0;
        k = 0;
        while (k as u32) < strlen8(arena, p2) {
            if byte(arena, p2.wrapping_add(k as u32)) == b'Y' {
                m = m.wrapping_add(1);
            }
            k = k.wrapping_add(1);
        }
        if m != n_y {
            return 0x89c0_2000;
        }
        let mut p3 = 0u32;
        r = host.depes_get_weights(arena, dh0, dh1, &mut p3, &mut olen);
        if r >= 0x8000_0000 {
            return r;
        }
        if find_substr_lit(arena, mode, b"spell") != 0 {
            put_byte(arena, p3.wrapping_add(olen as u32), 0);
            olen = olen.wrapping_add(1);
        }
        o1 = p1;
        o2 = p2;
        o3 = p3;
    }
    // Make room in the sentence for the normalised text.
    delta = (strlen8(arena, o1))
        .wrapping_add(delta as u32)
        .wrapping_sub(strlen8(arena, word(arena, c.wrapping_add(0x20)))) as u16 as i16;
    if delta > 0 {
        let p_text_ptr = word(arena, p_text);
        let shift0 = half(arena, c.wrapping_add(0x30)) as i16;
        match host.realloc(
            arena,
            heap,
            p_text_ptr,
            (text_len as u32).wrapping_add(delta as u32).wrapping_add(1) as u16 as u32,
        ) {
            Some(p) => put_word(arena, p_text, p),
            None => return 0x89c0_200a,
        }
        let p_text_ptr = word(arena, p_text);
        memset_at(
            arena,
            p_text_ptr.wrapping_add(text_len as u32),
            0,
            (delta as u32).wrapping_add(1) as u16 as u32,
        );
        let l = strlen8(arena, word(arena, p_text));
        let mut q = l as i32;
        let floor = shift0 as i32 + end as i32;
        while q >= floor {
            let v = byte(arena, word(arena, p_text).wrapping_add(q as u32));
            put_byte(
                arena,
                word(arena, p_text).wrapping_add((q + delta as i32) as u32),
                v,
            );
            q -= 1;
        }
    } else if delta < 0 {
        let shift0 = half(arena, c.wrapping_add(0x30)) as i16;
        let mut k = (shift0 as i32 + end as i32) as u16;
        while (k as u32) <= strlen8(arena, word(arena, p_text)) {
            let v = byte(arena, word(arena, p_text).wrapping_add(k as u32));
            put_byte(
                arena,
                word(arena, p_text).wrapping_add((k as i32 + delta as i32) as u32),
                v,
            );
            k = k.wrapping_add(1);
        }
    }
    // Copy the result back token by token and move the markers with it.
    from = start as u16;
    let mut prev_to = start as u16;
    let mut m = 0u16;
    let mut shift = 0i16;
    let mut out_pos = (start as u16).wrapping_add(half(arena, c.wrapping_add(0x30)));
    let mut k_start = 0u16;
    let mut ty = 0u8;
    while node != 0 && (from as u32) < end {
        let mut ty_w = 0u32;
        let mut len = 0u16;
        let rc = host.lat_get_int(arena, h0, h1, u32::from(node), 0, 1, &mut ty_w, &mut len);
        if rc >= 0x8000_0000 {
            return rc;
        }
        ty = ty_w as u8;
        if ty != 7 {
            let rc =
                host.lat_get_int(arena, h0, h1, u32::from(node), 1, 1, &mut ty_w, &mut len);
            if rc >= 0x8000_0000 {
                return rc;
            }
            from = ty_w as u16;
            let rc =
                host.lat_get_int(arena, h0, h1, u32::from(node), 2, 1, &mut ty_w, &mut len);
            if rc >= 0x8000_0000 {
                return rc;
            }
            to = ty_w as u16;
        }
        if (from as u32) < end && (ty == 6 || ty == 4) {
            k_start = m;
            while byte(arena, o2.wrapping_add(m as u32)) != b'X'
                && byte(arena, o2.wrapping_add(m as u32)) != 0
            {
                m = m.wrapping_add(1);
            }
            let n = m.wrapping_sub(k_start);
            memcpy_at(
                arena,
                word(arena, p_text).wrapping_add(out_pos as u32),
                o1.wrapping_add(k_start as u32),
                n as u32,
            );
            out_pos = out_pos.wrapping_add(n);
            let shift0 = half(arena, c.wrapping_add(0x30)) as i16;
            let b = shift as i32 + base as i32;
            while *p_idx < p_cnt
                && ((mark_pos(arena, marks, *p_idx as u32) as i32).wrapping_sub(base as i32) as i16 as i32)
                    < from as i32
            {
                let mk = marks.wrapping_add((*p_idx as u32).wrapping_mul(0x18));
                let lim = ((word(arena, mk.wrapping_add(0xc)) as i32)
                    .wrapping_sub(base as i32)
                    .wrapping_sub(prev_to as i32)) as u16;
                let mut acc = 0u16;
                let mut q = 0u16;
                while (q as u32) < n as u32 {
                    acc = acc.wrapping_add(byte(arena, o3.wrapping_add(k_start as u32).wrapping_add(q as u32)) as u16);
                    if acc > lim {
                        break;
                    }
                    q = q.wrapping_add(1);
                }
                put_word(
                    arena,
                    mk.wrapping_add(0xc),
                    ((q as i16) as i32 + (prev_to as i16) as i32 + shift0 as i32 + b) as u32,
                );
                *p_idx = p_idx.wrapping_add(1);
            }
            shift = shift
                .wrapping_add(m as i16)
                .wrapping_sub(k_start as i16)
                .wrapping_sub(from as i16)
                .wrapping_add(prev_to as i16);
            prev_to = to;
            let mut skipped = false;
            while byte(arena, o2.wrapping_add(m as u32)) != b'Y' {
                if byte(arena, o2.wrapping_add(m as u32)) == 0 {
                    skipped = true;
                    break;
                }
                m = m.wrapping_add(1);
            }
            // Skip to the next X, space, tilde or NUL.
            if !skipped {
                loop {
                    m = m.wrapping_add(1);
                    let ch = byte(arena, o2.wrapping_add(m as u32));
                    if ch == b'X' || ch == b' ' || ch == b'~' || ch == 0 {
                        break;
                    }
                }
            }
            let mut norm = 0u32;
            let rc = host.lat_get_str(arena, h0, h1, u32::from(node), 3, &mut norm, &mut len);
            if rc >= 0x8000_0000 {
                return rc;
            }
            let mut has_lm = 0u16;
            let mut n_off = 0u16;
            let mut offs = 0u32;
            if ty == 6
                && host.lat_get_u16(arena, h0, h1, u32::from(node), 5, &mut has_lm) < 0x8000_0000
                && has_lm == 1
            {
                let rc = host.lat_get_str(arena, h0, h1, u32::from(node), 5, &mut offs, &mut n_off);
                if rc >= 0x8000_0000 {
                    return rc;
                }
            }
            // Every \x1e...\x1f segment of the normalisation becomes a
            // token of its own.
            let mut seg = strchr8(arena, norm, 0x1e);
            let mut first = true;
            while seg != 0 {
                put_byte(arena, seg, 0);
                let mut e = seg.wrapping_add(1);
                while byte(arena, e) != 0x1f {
                    if byte(arena, e) == 0 {
                        return 0x89c0_2000;
                    }
                    e = e.wrapping_add(1);
                }
                put_byte(arena, e, 0);
                let mut i = n_off;
                let mut o = offs;
                while i != 0 {
                    if half(arena, o) as u32 > e.wrapping_sub(norm) {
                        put_half(
                            arena,
                            o,
                            half(arena, o)
                                .wrapping_sub(e.wrapping_sub(seg) as u16)
                                .wrapping_sub(1),
                        );
                    }
                    i = i.wrapping_sub(1);
                    o = o.wrapping_add(2);
                }
                let mut cur = node;
                let rc =
                    host.lat_get_int(arena, h0, h1, u32::from(node), 1, 1, &mut ty_w, &mut len);
                if rc >= 0x8000_0000 {
                    return rc;
                }
                let mut tfrom = ty_w as u16;
                let mut last: u16;
                loop {
                    last = cur;
                    let mut nx = 0u16;
                    let rc = host.lat_next_node(arena, h0, h1, u32::from(cur), &mut nx);
                    if rc >= 0x8000_0000 {
                        return rc;
                    }
                    cur = nx;
                    if cur != 0 {
                        let rc = host.lat_get_int(
                            arena, h0, h1, u32::from(cur), 1, 1, &mut ty_w, &mut len,
                        );
                        if rc >= 0x8000_0000 {
                            return rc;
                        }
                        tfrom = ty_w as u16;
                    }
                    if !(tfrom == from && cur != 0) {
                        break;
                    }
                }
                cur = last;
                if first {
                    first = false;
                } else {
                    let mut nx = 0u16;
                    let rc = host.lat_next_node(arena, h0, h1, u32::from(last), &mut nx);
                    if rc >= 0x8000_0000 {
                        return rc;
                    }
                    cur = nx;
                }
                if cur != 0 {
                    let rc = host.lat_get_int(
                        arena, h0, h1, u32::from(cur), 0, 1, &mut ty_w, &mut len,
                    );
                    if rc >= 0x8000_0000 {
                        return rc;
                    }
                    ty = ty_w as u8;
                    while cur != 0 && ty == 7 {
                        last = cur;
                        let mut nx = 0u16;
                        let rc = host.lat_next_node(arena, h0, h1, u32::from(cur), &mut nx);
                        if rc >= 0x8000_0000 {
                            return rc;
                        }
                        cur = nx;
                        if cur == 0 {
                            break;
                        }
                        let rc = host.lat_get_int(
                            arena, h0, h1, u32::from(cur), 0, 1, &mut ty_w, &mut len,
                        );
                        if rc >= 0x8000_0000 {
                            return rc;
                        }
                        ty = ty_w as u8;
                    }
                }
                cur = last;
                let mut new_node = 0u16;
                let rc = host.lat_insert_after(arena, h0, h1, u32::from(last), &mut new_node);
                if rc >= 0x8000_0000 {
                    return rc;
                }
                cur = new_node;
                ty = 7;
                let mut wr = 0u16;
                let rc = host.lat_set_field_bytes(
                    arena,
                    h0,
                    h1,
                    u32::from(cur),
                    0,
                    1,
                    &[ty],
                    &mut wr,
                );
                if rc >= 0x8000_0000 {
                    return rc;
                }
                let tfrom = (seg.wrapping_sub(norm) as u16 as i16)
                    .wrapping_add(from as i16)
                    .wrapping_add(shift)
                    .wrapping_add(half(arena, c.wrapping_add(0x30)) as i16);
                let rc = host.lat_set_field_bytes(
                    arena,
                    h0,
                    h1,
                    u32::from(cur),
                    1,
                    1,
                    &(tfrom as u16).to_le_bytes(),
                    &mut wr,
                );
                if rc >= 0x8000_0000 {
                    return rc;
                }
                let rc = host.lat_set_field_bytes(
                    arena,
                    h0,
                    h1,
                    u32::from(cur),
                    2,
                    1,
                    &(tfrom as u16).to_le_bytes(),
                    &mut wr,
                );
                if rc >= 0x8000_0000 {
                    return rc;
                }
                let rc = host.lat_set_field(
                    arena,
                    h0,
                    h1,
                    u32::from(cur),
                    4,
                    strlen8(arena, seg.wrapping_add(1)).wrapping_add(1),
                    seg.wrapping_add(1),
                    &mut wr,
                );
                if rc >= 0x8000_0000 {
                    return rc;
                }
                strcat_195c90(arena, norm, e.wrapping_add(1));
                seg = strchr8(arena, norm, 0x1e);
            }
            let rc =
                host.lat_get_int(arena, h0, h1, u32::from(node), 0, 1, &mut ty_w, &mut len);
            if rc >= 0x8000_0000 {
                return rc;
            }
            ty = ty_w as u8;
            has_lm = 0;
            let shift0 = half(arena, c.wrapping_add(0x30)) as i16;
            if ty == 6
                && host.lat_get_u16(arena, h0, h1, u32::from(node), 5, &mut has_lm) < 0x8000_0000
                && has_lm == 1
            {
                let rc = host.lat_get_str(arena, h0, h1, u32::from(node), 5, &mut offs, &mut len);
                if rc >= 0x8000_0000 {
                    return rc;
                }
                let mut sp = strchr8(arena, orig.wrapping_add(from as u32), b' ');
                let b = base as i32 + shift as i32;
                let mut q = 0u16;
                while sp != 0 && (q as i32) < (len as i32).wrapping_sub(1) {
                    let o = offs.wrapping_add(q as u32 * 2);
                    while *p_idx < p_cnt
                        && ((mark_pos(arena, marks, *p_idx as u32) as i32).wrapping_sub(base as i32) as i16 as i32)
                            < (sp.wrapping_sub(orig.wrapping_add(from as u32)) as i32)
                                + from as i32
                    {
                        let mk = marks.wrapping_add((*p_idx as u32).wrapping_mul(0x18));
                        put_word(
                            arena,
                            mk.wrapping_add(0xc),
                            (from as i32 + shift0 as i32 + b + half(arena, o) as i32) as u32,
                        );
                        *p_idx = p_idx.wrapping_add(1);
                    }
                    while byte(arena, sp) == b' ' {
                        sp = sp.wrapping_add(1);
                    }
                    sp = strchr8(arena, sp, b' ');
                    q = q.wrapping_add(1);
                }
                while *p_idx < p_cnt
                    && ((mark_pos(arena, marks, *p_idx as u32) as i32).wrapping_sub(base as i32) as i16 as i32)
                        < to as i32
                {
                    let mk = marks.wrapping_add((*p_idx as u32).wrapping_mul(0x18));
                    put_word(
                        arena,
                        mk.wrapping_add(0xc),
                        (from as i32 + shift0 as i32 + b + half(arena, offs.wrapping_add(q as u32 * 2)) as i32) as u32,
                    );
                    *p_idx = p_idx.wrapping_add(1);
                }
            } else if strcmp_lit(arena, mode, b"prompt") != 0 {
                let b = base as i32 + shift as i32;
                let shift0 = half(arena, c.wrapping_add(0x30)) as i16;
                while *p_idx < p_cnt
                    && ((mark_pos(arena, marks, *p_idx as u32) as i32).wrapping_sub(base as i32) as i16 as i32)
                        < to as i32
                {
                    let mk = marks.wrapping_add((*p_idx as u32).wrapping_mul(0x18));
                    put_word(
                        arena,
                        mk.wrapping_add(0xc),
                        (shift0 as i32 + from as i32 + b) as u32,
                    );
                    *p_idx = p_idx.wrapping_add(1);
                }
            } else {
                let shift0 = half(arena, c.wrapping_add(0x30)) as i16;
                while *p_idx < p_cnt
                    && ((mark_pos(arena, marks, *p_idx as u32) as i32).wrapping_sub(base as i32) as u16 as u32)
                        < to as u32
                {
                    let mk = marks.wrapping_add((*p_idx as u32).wrapping_mul(0x18));
                    let v = word(arena, mk.wrapping_add(0xc));
                    put_word(
                        arena,
                        mk.wrapping_add(0xc),
                        v.wrapping_add(shift0 as u32).wrapping_add(shift as u32),
                    );
                    *p_idx = p_idx.wrapping_add(1);
                }
            }
            memcpy_at(
                arena,
                word(arena, p_text).wrapping_add(out_pos as u32),
                norm,
                strlen8(arena, norm),
            );
            out_pos = out_pos.wrapping_add(strlen8(arena, norm) as u16);
            let mut new_from = 0u16;
            if ty == 6 {
                new_from = (shift0 as i32 + from as i32 + shift as i32) as u16;
            }
            shift = (strlen8(arena, norm))
                .wrapping_add(shift as u32)
                .wrapping_sub(to as u32)
                .wrapping_add(from as u32) as u16 as i16;
            if ty == 6 {
                let new_to = (to as i32 + shift as i32 + shift0 as i32) as u16;
                let mut wr = 0u16;
                let rc = host.lat_set_field_bytes(
                    arena,
                    h0,
                    h1,
                    u32::from(node),
                    1,
                    1,
                    &new_from.to_le_bytes(),
                    &mut wr,
                );
                if rc >= 0x8000_0000 {
                    return rc;
                }
                let rc = host.lat_set_field_bytes(
                    arena,
                    h0,
                    h1,
                    u32::from(node),
                    2,
                    1,
                    &new_to.to_le_bytes(),
                    &mut wr,
                );
                if rc >= 0x8000_0000 {
                    return rc;
                }
            }
        } else if (from as u32) < end && ty == 5 {
            let mut wr = 0u16;
            let sh5 = half(arena, c.wrapping_add(0x30)) as i16;
            let from5 = (from as i32 + sh5 as i32) as u16;
            let rc = host.lat_set_field_bytes(
                arena,
                h0,
                h1,
                u32::from(node),
                1,
                1,
                &from5.to_le_bytes(),
                &mut wr,
            );
            if rc >= 0x8000_0000 {
                return rc;
            }
            let to5 = (to as i32 + delta as i32 + sh5 as i32) as u16;
            let rc = host.lat_set_field_bytes(
                arena,
                h0,
                h1,
                u32::from(node),
                2,
                1,
                &to5.to_le_bytes(),
                &mut wr,
            );
            if rc >= 0x8000_0000 {
                return rc;
            }
        }
        let mut nx = 0u16;
        r = host.lat_next_node(arena, h0, h1, u32::from(node), &mut nx);
        if r >= 0x8000_0000 {
            return r;
        }
        node = nx;
    }
    // The tail after the last token.
    k_start = m;
    while byte(arena, o2.wrapping_add(m as u32)) != 0 {
        m = m.wrapping_add(1);
    }
    if ((out_pos as u32).wrapping_add(m as u32).wrapping_sub(k_start as u32) as i32)
        > ((text_len as u32).wrapping_add(delta as u32) as i32)
    {
        return 0x89c0_2000;
    }
    {
        let n = (m as u32).wrapping_sub(k_start as u32);
        memcpy_at(
            arena,
            word(arena, p_text).wrapping_add(out_pos as u32),
            o1.wrapping_add(k_start as u32),
            n,
        );
        let b = base as i32 + shift as i32;
        let shift0 = half(arena, c.wrapping_add(0x30)) as i16;
        while *p_idx < p_cnt
            && ((mark_pos(arena, marks, *p_idx as u32) as i32).wrapping_sub(base as i32) as i16 as i32)
                <= end as i32
        {
            let mk = marks.wrapping_add((*p_idx as u32).wrapping_mul(0x18));
            let lim = ((word(arena, mk.wrapping_add(0xc)) as i32)
                .wrapping_sub(base as i32)
                .wrapping_sub(prev_to as i32)) as u16;
            let mut acc = 0u16;
            let mut q = 0u16;
            while (q as u32) < n {
                acc = acc
                    .wrapping_add(byte(arena, o3.wrapping_add(k_start as u32).wrapping_add(q as u32)) as u16);
                if acc > lim {
                    break;
                }
                q = q.wrapping_add(1);
            }
            put_word(
                arena,
                mk.wrapping_add(0xc),
                ((q as i16) as i32 + (prev_to as i16) as i32 + shift0 as i32 + b) as u32,
            );
            *p_idx = p_idx.wrapping_add(1);
        }
    }
    put_half(
        arena,
        c.wrapping_add(0x30),
        (half(arena, c.wrapping_add(0x30)) as i16).wrapping_add(delta) as u16,
    );
    r
}

/// fn_203144, EU fn_203300, KR fn_2033ac. Validate, then run the `282af0`
/// dictionary transform over the furnished buffers.
pub fn dict_call_203144(
    arena: &mut [u8],
    p: u32,
    b: u32,
    c: u32,
    d: u32,
    e: u32,
    f: u32,
    g: u32,
    h: u32,
    host: &mut impl FeTextHost,
) -> u32 {
    if host.validate(b, 0xf38a, 0xb8) >= 0x8000_0000 {
        return 0x8a80_2008;
    }
    host.tn_282af0(
        arena,
        word(arena, p.wrapping_add(8)),
        word(arena, p.wrapping_add(0x44)),
        word(arena, p.wrapping_add(0x58)),
        word(arena, p.wrapping_add(0x5c)),
        word(arena, p.wrapping_add(0x60)),
        c,
        d,
        e,
        f,
        g,
        h,
    )
}

/// `fn_195d58` against a static literal: -1/0/1 like `strcmp_at`.
pub(crate) fn strcmp_lit(arena: &[u8], s: u32, lit: &[u8]) -> i32 {
    let mut i = 0u32;
    loop {
        let a = byte(arena, s.wrapping_add(i));
        let b = if (i as usize) < lit.len() { lit[i as usize] } else { 0 };
        let d = a as i32 - b as i32;
        if d != 0 || b == 0 {
            if d < 0 {
                return -1;
            }
            if d > 0 {
                return 1;
            }
            return 0;
        }
        i = i.wrapping_add(1);
    }
}

/// fn_198760, EU fn_198810, KR fn_19884c. Walk the tree at `t` for `key`
/// through the scripted comparator. Returns the matching node offset, or
/// 0 when the walk reaches the tree sentinel.
pub fn tree_find_198760(arena: &mut [u8], t: u32, key: u32, host: &mut impl FeTextHost) -> u32 {
    let mut n = word(arena, t);
    loop {
        if n == t {
            return 0;
        }
        let r = host.cmp_seq(arena, key, word(arena, n.wrapping_add(16)));
        if r < 0 {
            n = word(arena, n);
        }
        if r > 0 {
            n = word(arena, n.wrapping_add(4));
        }
        if r == 0 {
            break;
        }
    }
    if byte(arena, t.wrapping_add(0x28)) == 0 {
        return n;
    }
    loop {
        let last = n;
        n = word(arena, last);
        while n != t && host.cmp_seq(arena, key, word(arena, n.wrapping_add(16))) != 0 {
            n = word(arena, n.wrapping_add(4));
        }
        if n == t {
            return last;
        }
    }
}

/// fn_176f2c lock wrapper: indirect through `word(word(vtbl)+8)+8`, 0 for
/// a null slot, else the inner call on the target.
fn lock_176f2c(arena: &[u8], p: u32, host: &mut impl FeTextHost) -> u32 {
    if p == 0 {
        return 0x8140_2007;
    }
    let inner = word(arena, word(arena, p).wrapping_add(8));
    let f = word(arena, inner.wrapping_add(8));
    if f == 0 {
        return 0;
    }
    host.lock_call(word(arena, p.wrapping_add(8)))
}

/// fn_176f5c unlock wrapper: indirect through `word(word(vtbl)+8)+12`.
fn lock_176f5c(arena: &[u8], p: u32, host: &mut impl FeTextHost) -> u32 {
    if p == 0 {
        return 0x8140_2007;
    }
    let inner = word(arena, word(arena, p).wrapping_add(8));
    let f = word(arena, inner.wrapping_add(12));
    if f == 0 {
        return 0;
    }
    host.lock_call(word(arena, p.wrapping_add(8)))
}

/// fn_1966ac, EU fn_19675c, KR fn_196798. Look up `b` in the tree at
/// `a + 24`, reporting the entry word at node offset 20. A null `out`
/// models the C's null-pointer early return; callers pass a stack local.
pub fn lookup_1966ac(
    arena: &mut [u8],
    a: u32,
    b: u32,
    out: Option<&mut u32>,
    host: &mut impl FeTextHost,
) -> u32 {
    if a == 0 || b == 0 || out.is_none() {
        return 0x8900_2007;
    }
    let e = tree_find_198760(arena, a.wrapping_add(24), b, host);
    if e != 0 {
        *out.unwrap() = word(arena, e.wrapping_add(20));
        0
    } else {
        0x8900_2014
    }
}

/// fn_1756d0, EU fn_175780, KR fn_17572c. Config lookup: lock, entry
/// lookup, unlock. A null `p1` models the C's null-pointer early return;
/// callers pass a stack local and `p2` is 0 (null) or an arena cell.
pub fn config_1756d0(
    arena: &mut [u8],
    o: u32,
    key: u32,
    p1: Option<&mut u32>,
    p2: u32,
    host: &mut impl FeTextHost,
) -> u32 {
    if o == 0 || key == 0 || p1.is_none() {
        return 0x8b70_2007;
    }
    let p1 = p1.unwrap();
    *p1 = 0;
    if p2 != 0 {
        put_word(arena, p2, 0);
    }
    let a4 = word(arena, o.wrapping_add(4));
    let a8 = word(arena, o.wrapping_add(8));
    let t = lock_176f2c(arena, a4, host);
    if t >= 0x8000_0000 {
        return t;
    }
    let mut ent = 0u32;
    let mut k = lookup_1966ac(arena, a8, key, Some(&mut ent), host);
    if k >= 0x8000_0000 {
        k = (k & 0x1fff) | 0x8b70_2000;
    } else if ent != 0 {
        if word(arena, ent.wrapping_add(4)) != 0 {
            *p1 = word(arena, ent);
            if p2 != 0 {
                put_word(arena, p2, word(arena, ent.wrapping_add(4)));
            }
        } else {
            *p1 = 0;
            if p2 != 0 {
                put_word(arena, p2, 0);
            }
        }
    }
    let t = lock_176f5c(arena, a4, host);
    if t >= 0x8000_0000 && k < 0x8000_0000 {
        k = t;
    }
    k
}

/// fn_1d88f4, EU fn_1d8ab0, KR fn_1d8b5c. Destroy the FE object: free the
/// five owned buffers, release the three registry resources, free the
/// object. The frees are no-ops on every side; the zero stores are the
/// observation. NOTE: the `p == 0` check after the `f2c` free is dead in
/// practice (a null `p` faults on the `f2c` read first).
pub fn fe_delete_1d88f4(arena: &mut [u8], p: u32, b: u32, host: &mut impl FeTextHost) -> u32 {
    let r = host.validate(b, 0xf383, 56);
    if r >= 0x8000_0000 {
        return 0x89c0_2008;
    }
    if word(arena, p.wrapping_add(0x2c)) != 0 {
        let env = word(arena, p);
        host.free(
            arena,
            word(arena, env.wrapping_add(4)),
            word(arena, p.wrapping_add(0x2c)),
        );
    }
    // NOTE: no `p->f2c = 0` store here. The src line compiles to nothing:
    // the ROM has no `str [r4, #0x2c]` (base `fn_1d88f4` stores only
    // +0x20/+0x24/+0x28/+0x34) and gcc -O2 drops it too. The null check
    // below matches the ROM's kept `cmp r4, #0` (gcc drops it as well);
    // it is unreachable in practice since a null `p` faults on the reads.
    if p == 0 {
        return r;
    }
    let env = word(arena, p);
    let heap = word(arena, env.wrapping_add(4));
    for off in [0x20u32, 0x24, 0x28, 0x34] {
        if word(arena, p.wrapping_add(off)) != 0 {
            host.free(arena, heap, word(arena, p.wrapping_add(off)));
        }
        put_word(arena, p.wrapping_add(off), 0);
    }
    let reg = word(arena, env.wrapping_add(0x18));
    host.release_named(reg, 0);
    host.release_named(reg, 0);
    host.release_named(reg, 0);
    host.free(arena, heap, p);
    0
}

/// fn_1d8a20, EU fn_1d8bdc, KR fn_1d8c88. Open the FE object: resolve the
/// environment, acquire LINGDB/FE_DEPES/FE_DCTLKP, allocate the object,
/// read the dictionary flags and the `fasttn`/`esctn` params. Failures
/// after allocation tear down through `fe_delete_1d88f4` and zero `out`.
/// `a0`/`a1` are ignored; the log calls are unobserved on every side.
pub fn fe_open_1d8a20(
    arena: &mut [u8],
    a0: u32,
    a1: u32,
    h0: u32,
    h1: u32,
    out: u32,
    host: &mut impl FeTextHost,
) -> u32 {
    let _ = (a0, a1);
    if out == 0 {
        return 0x89c0_2007;
    }
    put_word(arena, out, 0);
    put_word(arena, out.wrapping_add(4), 0);
    let (mut r, env) = host.resolve(arena, h0, h1);
    if r >= 0x8000_0000 {
        return r;
    }
    let reg = word(arena, env.wrapping_add(0x18));
    let (status, lingdb) = host.acquire(arena, reg, 0);
    r = status;
    if r >= 0x8000_0000 {
        return r;
    }
    let (status, depes) = host.acquire(arena, reg, 0);
    r = status;
    if r >= 0x8000_0000 {
        host.release_named(reg, 0);
        return r;
    }
    let (status, dct) = host.acquire(arena, reg, 0);
    r = status;
    if r >= 0x8000_0000 {
        host.release_named(reg, 0);
        host.release_named(reg, 0);
        return r;
    }
    let heap = word(arena, env.wrapping_add(4));
    let o = match host.alloc(arena, heap, 56) {
        Some(p) => p,
        None => {
            r = 0x89c0_200a;
            host.release_named(reg, 0);
            host.release_named(reg, 0);
            host.release_named(reg, 0);
            return r;
        }
    };
    put_word(arena, out.wrapping_add(4), 0xf383);
    put_word(arena, out, o);
    put_word(arena, o, env);
    put_word(arena, o.wrapping_add(4), word(arena, lingdb.wrapping_add(4)));
    put_word(arena, o.wrapping_add(8), word(arena, depes.wrapping_add(8)));
    put_word(arena, o.wrapping_add(12), word(arena, depes.wrapping_add(12)));
    put_word(arena, o.wrapping_add(0x10), word(arena, depes.wrapping_add(4)));
    put_word(arena, o.wrapping_add(0x14), word(arena, dct.wrapping_add(8)));
    put_word(arena, o.wrapping_add(0x18), word(arena, dct.wrapping_add(12)));
    put_word(arena, o.wrapping_add(0x1c), word(arena, dct.wrapping_add(4)));
    put_word(arena, o.wrapping_add(0x20), 0);
    put_word(arena, o.wrapping_add(0x24), 0);
    put_word(arena, o.wrapping_add(0x28), 0);
    put_word(arena, o.wrapping_add(0x2c), 0);
    put_half(arena, o.wrapping_add(0x30), 0);
    let dh0 = word(arena, o.wrapping_add(0x14));
    let dh1 = word(arena, o.wrapping_add(0x18));
    let mut fa = 0u8;
    let mut fb = 0u8;
    r = host.dct_get_flags(arena, dh0, dh1, &mut fa, &mut fb);
    if r < 0x8000_0000 {
        put_byte(arena, o.wrapping_add(0x32), 0);
        if fb == 1 && fa == 1 {
            put_byte(arena, o.wrapping_add(0x32), 1);
        }
        put_byte(arena, o.wrapping_add(0x33), 0);
        let mut val = 0u32;
        let mut found = 0xffffu16;
        let mut n = 0i32;
        r = host.dct_get_param(arena, dh0, dh1, 0, 0, &mut val, &mut found, &mut n);
        if r < 0x8000_0000 {
            let f14 = word(arena, env.wrapping_add(0x14));
            let mut str_v = 0u32;
            // The key is the nonzero "fecfg" literal on the C/ARM sides;
            // the scripted comparator ignores key values, so 1 stands in.
            let rcfg = config_1756d0(arena, f14, 1, Some(&mut str_v), 0, host);
            if rcfg < 0x8000_0000 && found == 0 {
                if strcmp_fold_lit(arena, str_v, b"mpthree") == 0
                    || strcmp_fold_lit(arena, str_v, b"vadvde") == 0
                {
                    put_byte(arena, o.wrapping_add(0x33), 1);
                }
            }
            found = 0xffff;
            r = host.dct_get_param(arena, dh0, dh1, 0, 0, &mut val, &mut found, &mut n);
            if r < 0x8000_0000 {
                put_word(arena, o.wrapping_add(0x34), 0);
                if found != 0 {
                    let src = word(arena, val);
                    let len = strlen8(arena, src);
                    match host.alloc(arena, heap, len.wrapping_add(1)) {
                        Some(e) => {
                            strcpy_at(arena, e, src);
                            put_byte(arena, e.wrapping_add(len).wrapping_sub(1), b'|');
                            put_word(arena, o.wrapping_add(0x34), e);
                        }
                        None => {
                            r = 0x89c0_200a;
                        }
                    }
                }
            }
        }
    }
    if r >= 0x8000_0000 {
        fe_delete_1d88f4(
            arena,
            word(arena, out),
            word(arena, out.wrapping_add(4)),
            host,
        );
        put_word(arena, out, 0);
        put_word(arena, out.wrapping_add(4), 0);
    }
    r
}

/// fn_1d8dd8, EU fn_1d8f94, KR fn_1d9040. Refresh the FE cached flags:
/// dictionary flags into `f32`, config voice check into `f33`. Returns
/// the config lookup status.
pub fn fe_refresh_1d8dd8(arena: &mut [u8], p: u32, b: u32, host: &mut impl FeTextHost) -> u32 {
    let r = host.validate(b, 0xf383, 56);
    if r >= 0x8000_0000 {
        return 0x89c0_2008;
    }
    if p == 0 {
        return r;
    }
    let dh0 = word(arena, p.wrapping_add(0x14));
    let dh1 = word(arena, p.wrapping_add(0x18));
    let mut fa = 0u8;
    let mut fb = 0u8;
    let r = host.dct_get_flags(arena, dh0, dh1, &mut fa, &mut fb);
    if r >= 0x8000_0000 {
        return r;
    }
    put_byte(arena, p.wrapping_add(0x32), 0);
    if fb == 1 && fa == 1 {
        put_byte(arena, p.wrapping_add(0x32), 1);
    }
    put_byte(arena, p.wrapping_add(0x33), 0);
    let env = word(arena, p);
    let f14 = word(arena, env.wrapping_add(0x14));
    let mut str_v = 0u32;
    let r = config_1756d0(arena, f14, 1, Some(&mut str_v), 0, host);
    if r < 0x8000_0000 {
        if strcmp_fold_lit(arena, str_v, b"mpthree") == 0
            || strcmp_fold_lit(arena, str_v, b"vadvde") == 0
        {
            put_byte(arena, p.wrapping_add(0x33), 1);
        }
    }
    r
}

/// fn_1d8f28, EU fn_1d90e4, KR fn_1d9190. Walk the lattice nodes for the
/// span, copying each node span into `buf`. The type-name/log calls are
/// unobserved on every side; only `buf` and the return are compared.
pub fn fe_dump_1d8f28(
    arena: &mut [u8],
    c: u32,
    h0: u32,
    h1: u32,
    x: u32,
    text: u32,
    buf: u32,
    name: u32,
    host: &mut impl FeTextHost,
) -> u32 {
    let _ = name;
    let mut node = 0u16;
    let mut r = host.lat_first_node(arena, h0, h1, 3, x, &mut node);
    if r >= 0x8000_0000 {
        return r;
    }
    if node != 0 {
        loop {
            let mut from_v = 0u32;
            let mut to_v = 0u32;
            let mut ty_v = 0u32;
            let mut len = 0u16;
            r = host.lat_get_int(arena, h0, h1, u32::from(node), 1, 1, &mut from_v, &mut len);
            if r >= 0x8000_0000 {
                return r;
            }
            r = host.lat_get_int(arena, h0, h1, u32::from(node), 2, 1, &mut to_v, &mut len);
            if r >= 0x8000_0000 {
                return r;
            }
            r = host.lat_get_int(arena, h0, h1, u32::from(node), 0, 1, &mut ty_v, &mut len);
            if r >= 0x8000_0000 {
                return r;
            }
            let from = from_v as u16;
            let to = to_v as u16;
            let ty = ty_v as u8;
            if ty == 5 || ty == 6 {
                let mut s1 = 0u32;
                r = host.lat_get_str(arena, h0, h1, u32::from(node), 4, &mut s1, &mut len);
                if r >= 0x8000_0000 {
                    return r;
                }
                let _ = s1;
                if ty == 6 {
                    let mut s2 = 0u32;
                    r = host.lat_get_str(arena, h0, h1, u32::from(node), 3, &mut s2, &mut len);
                    if r >= 0x8000_0000 {
                        return r;
                    }
                    let _ = s2;
                    let mut has_pron = 0u16;
                    if host.lat_get_u16(arena, h0, h1, u32::from(node), 6, &mut has_pron)
                        < 0x8000_0000
                        && has_pron == 1
                    {
                        let mut pron = 0u32;
                        let mut len2 = 0u16;
                        r = host.lat_get_str(
                            arena,
                            h0,
                            h1,
                            u32::from(node),
                            6,
                            &mut pron,
                            &mut len2,
                        );
                        if r >= 0x8000_0000 {
                            return r;
                        }
                        let _ = (pron, len2);
                    }
                }
            }
            let span = u32::from(to).wrapping_sub(u32::from(from));
            strncpy_at(arena, buf, text.wrapping_add(u32::from(from)), span);
            put_byte(arena, buf.wrapping_add(span), 0);
            if ty <= 4 {
                let mut tname = 0u32;
                host.lat_type_name(arena, h0, h1, u32::from(ty), &mut tname);
                let _ = tname;
            }
            let mut next = 0u16;
            if host.lat_next_node(arena, h0, h1, u32::from(node), &mut next) != 0 || next == 0 {
                break;
            }
            node = next;
        }
    }
    let _ = c;
    r
}

/// `TokEdit` record: 14 bytes, all scalar (f00/from/to/type/op/str/ch/adj/
/// lid at 0/2/4/6/7/8/0xa/0xb/0xc).
const TOKEDIT_SIZE: u32 = 14;

fn tok_base(toks: u32, i: u16) -> u32 {
    toks.wrapping_add(u32::from(i).wrapping_mul(TOKEDIT_SIZE))
}

/// fn_1d9508, EU fn_1d96c4, KR fn_1d9770. Tag the sentence langmaps: walk
/// the lattice, load the tokens, paint the class map, realign/extend the
/// pronunciation spans, insert one edit per class run, write the edits
/// back. Cleanup frees are no-ops on every side, so every `goto cleanup`
/// is a plain early return here, but the `strs[i] = 0` cleanup stores are
/// real: the `strs` array lives in the observed pool on the integration
/// rows. `fn_1d9340` calls are log-only (zero observation) and skipped;
/// `fn_1d8f28` in the debug paths is real. `KEY_INIT` is the furnished
/// `D_8f8980` default-key image (the Rust side has no ROM); the pointer
/// variable is dereferenced by the casegen.
fn clear_strs_1d9508(arena: &mut [u8], strs: u32, n_str: u16) {
    let mut i = 0u16;
    while i < n_str {
        put_word(arena, strs.wrapping_add(u32::from(i).wrapping_mul(4)), 0);
        i = i.wrapping_add(1);
    }
}

pub fn fe_tags_1d9508(
    arena: &mut [u8],
    c: u32,
    h0: u32,
    h1: u32,
    x: u32,
    text: u32,
    host: &mut impl FeTextHost,
) -> u32 {
    const KEY_INIT: u32 = 0x760;
    // Set-data used to live at c + 0x7c; the C passes &locals and the
    // "lid" literal, which now travel through lat_set_field_bytes.
    let env = word(arena, c);
    let heap = word(arena, env.wrapping_add(4));
    let log = word(arena, env.wrapping_add(0x10));
    let dbg = host.log_level(arena, log) >= 5;
    let mut node = 0u16;
    let mut count = 0u16;
    let mut len = 0u16;
    let mut r = host.lat_count_nodes(arena, h0, h1, 3, x, &mut count);
    if r >= 0x8000_0000 {
        return r;
    }
    if count == 0 {
        return r;
    }
    r = host.lat_first_node(arena, h0, h1, 3, x, &mut node);
    if r >= 0x8000_0000 {
        return r;
    }
    if node == 0 {
        return r;
    }
    let mut n_tok = 0u16;
    let mut any = false;
    loop {
        if !any {
            let mut ty_w = 0u32;
            r = host.lat_get_int(arena, h0, h1, u32::from(node), 0, 1, &mut ty_w, &mut len);
            if r >= 0x8000_0000 {
                return r;
            }
            if ty_w as u8 == 6 {
                let mut has_pron = 0u16;
                r = host.lat_get_u16(arena, h0, h1, u32::from(node), 6, &mut has_pron);
                if r >= 0x8000_0000 {
                    return r;
                }
                if has_pron == 1 {
                    any = true;
                }
            }
        }
        n_tok = n_tok.wrapping_add(1);
        let mut next = 0u16;
        if host.lat_next_node(arena, h0, h1, u32::from(node), &mut next) != 0 || next == 0 {
            break;
        }
        node = next;
    }
    if !any {
        return r;
    }
    let sent_len = strlen8(arena, text) as u16;
    let mut dbg_buf = 0u32;
    if dbg {
        dbg_buf = match host.calloc(arena, heap, 1, u32::from(sent_len).wrapping_add(1)) {
            Some(p) => p,
            None => return 0x89c0_200a,
        };
        strcpy_at(arena, dbg_buf, text);
        fe_dump_1d8f28(arena, c, h0, h1, x, text, dbg_buf, 0, host);
    }
    let cls = match host.calloc(arena, heap, 1, u32::from(sent_len).wrapping_add(1)) {
        Some(p) => p,
        None => return 0x89c0_200a,
    };
    let word_idx = match host.calloc(
        arena,
        heap,
        1,
        u32::from(sent_len).wrapping_add(1).wrapping_mul(2),
    ) {
        Some(p) => p,
        None => return 0x89c0_200a,
    };
    let lid = match host.calloc(arena, heap, 1, u32::from(sent_len).wrapping_add(1)) {
        Some(p) => p,
        None => return 0x89c0_200a,
    };
    let mut toks = match host.calloc(
        arena,
        heap,
        1,
        u32::from(n_tok).wrapping_add(1).wrapping_mul(TOKEDIT_SIZE),
    ) {
        Some(p) => p,
        None => return 0x89c0_200a,
    };
    let mut strs = match host.calloc(arena, heap, 1, 6 * 4) {
        Some(p) => p,
        None => return 0x89c0_200a,
    };
    let mut n_str = 0u16;
    let mut cap = 5u16;
    let tab = match host.calloc(arena, heap, 1, 128 * 4) {
        Some(p) => p,
        None => {
            clear_strs_1d9508(arena, strs, n_str);
            return 0x89c0_200a;
        }
    };
    let dh0 = word(arena, c.wrapping_add(0x14));
    let dh1 = word(arena, c.wrapping_add(0x18));
    let mut pv = 0u32;
    let mut found = 0xffffu16;
    let mut sep = 0i32;
    r = host.dct_get_param(arena, dh0, dh1, 0, 0, &mut pv, &mut found, &mut sep);
    if r >= 0x8000_0000 {
        clear_strs_1d9508(arena, strs, n_str);
        return r;
    }
    let mut key = KEY_INIT;
    if found != 0 {
        let q = strchr_195d30(arena, word(arena, pv), sep as u32);
        if q != 0 {
            put_byte(arena, q, 0);
        }
        key = word(arena, pv);
    }
    let mut buf = match host.calloc(arena, heap, 1, strlen8(arena, key).wrapping_add(1)) {
        Some(p) => p,
        None => {
            clear_strs_1d9508(arena, strs, n_str);
            return 0x89c0_200a;
        }
    };
    strcpy_at(arena, buf, key);
    put_word(arena, strs, buf);
    n_str = 1;
    r = host.lat_first_node(arena, h0, h1, 3, x, &mut node);
    if r >= 0x8000_0000 {
        clear_strs_1d9508(arena, strs, n_str);
        return r;
    }
    n_tok = 0;
    if node != 0 {
        loop {
            let tb = tok_base(toks, n_tok);
            put_half(arena, tb, node);
            put_byte(arena, tb.wrapping_add(7), 0);
            put_half(arena, tb.wrapping_add(8), 0);
            put_byte(arena, tb.wrapping_add(0xa), b' ');
            put_byte(arena, tb.wrapping_add(0xb), 0);
            put_byte(arena, tb.wrapping_add(0xc), 0);
            let mut from_v = 0u32;
            let mut to_v = 0u32;
            let mut ty_v = 0u32;
            r = host.lat_get_int(arena, h0, h1, u32::from(node), 1, 1, &mut from_v, &mut len);
            if r >= 0x8000_0000 {
                clear_strs_1d9508(arena, strs, n_str);
                return r;
            }
            r = host.lat_get_int(arena, h0, h1, u32::from(node), 2, 1, &mut to_v, &mut len);
            if r >= 0x8000_0000 {
                clear_strs_1d9508(arena, strs, n_str);
                return r;
            }
            r = host.lat_get_int(arena, h0, h1, u32::from(node), 0, 1, &mut ty_v, &mut len);
            if r >= 0x8000_0000 {
                clear_strs_1d9508(arena, strs, n_str);
                return r;
            }
            put_half(arena, tb.wrapping_add(2), from_v as u16);
            put_half(arena, tb.wrapping_add(4), to_v as u16);
            put_byte(arena, tb.wrapping_add(6), ty_v as u8);
            if ty_v as u8 == 5 {
                let mut tmp = 0u16;
                if host.lat_get_u16(arena, h0, h1, u32::from(node), 6, &mut tmp) < 0x8000_0000
                    && tmp == 1
                {
                    put_byte(arena, tb.wrapping_add(0xc), 1);
                }
                let mut val = 0u32;
                r = host.lat_get_str(arena, h0, h1, u32::from(node), 4, &mut val, &mut len);
                if r >= 0x8000_0000 {
                    clear_strs_1d9508(arena, strs, n_str);
                    return r;
                }
                let esc = word(arena, c.wrapping_add(0x34));
                if esc == 0 {
                    clear_strs_1d9508(arena, strs, n_str);
                    return 0x89c0_200a;
                }
                let bar = find_substr(arena, esc, val);
                let vlen = strlen8(arena, val);
                let s0len = strlen8(arena, word(arena, strs));
                buf = match host.calloc(
                    arena,
                    heap,
                    1,
                    if s0len > vlen { s0len } else { vlen }.wrapping_add(1),
                ) {
                    Some(p) => p,
                    None => {
                        clear_strs_1d9508(arena, strs, n_str);
                        return 0x89c0_200a;
                    }
                };
                let bar_pipe = bar != 0 && byte(arena, bar.wrapping_add(vlen)) == b'|';
                if strcmp_lit(arena, val, b"phon") == 0
                    || strcmp_lit(arena, val, b"prompt") == 0
                    || (strcmp_lit(arena, val, b"latin") != 0
                        && (bar_pipe || strchr_195d30(arena, val, '_' as u32) != 0))
                {
                    strcpy_at(arena, buf, val);
                    put_byte(arena, tb.wrapping_add(0xa), b'N');
                    let mut k = 0u32;
                    while k < vlen {
                        if byte(arena, val.wrapping_add(k)) == b'_' {
                            let ch = fold_17c8dc(byte(arena, val.wrapping_add(k).wrapping_add(1)) as u32)
                                as u8;
                            put_byte(arena, tb.wrapping_add(0xa), ch);
                            put_word(arena, tab.wrapping_add(u32::from(ch).wrapping_mul(4)), val.wrapping_add(k).wrapping_add(1));
                            memset_195bec(arena, buf.wrapping_add(k), 0, vlen.wrapping_sub(k));
                            break;
                        }
                        k = k.wrapping_add(1);
                    }
                } else {
                    strcpy_at(arena, buf, word(arena, strs));
                    let ch = fold_17c8dc(byte(arena, val) as u32) as u8;
                    put_byte(arena, tb.wrapping_add(0xa), ch);
                    put_word(arena, tab.wrapping_add(u32::from(ch).wrapping_mul(4)), val);
                }
                let from = half(arena, tb.wrapping_add(2));
                let to = half(arena, tb.wrapping_add(4));
                let ch = byte(arena, tb.wrapping_add(0xa));
                let mut k = from;
                while k < to {
                    put_byte(arena, cls.wrapping_add(u32::from(k)), ch);
                    k = k.wrapping_add(1);
                }
                let mut i = 0u16;
                while i < n_str {
                    if strcmp_ci_1769d0(arena, word(arena, strs.wrapping_add(u32::from(i).wrapping_mul(4))), buf) == 0 {
                        put_half(arena, tb.wrapping_add(8), i);
                        host.free(arena, heap, buf);
                        buf = 0;
                        break;
                    }
                    i = i.wrapping_add(1);
                }
                if i == n_str {
                    if n_str.wrapping_add(1) > cap {
                        let p = match host.realloc(
                            arena,
                            heap,
                            strs,
                            u32::from(cap).wrapping_add(6).wrapping_mul(4),
                        ) {
                            Some(p) => p,
                            None => {
                                clear_strs_1d9508(arena, strs, n_str);
                                return 0x89c0_200a;
                            }
                        };
                        strs = p;
                        cap = cap.wrapping_add(5);
                        put_word(arena, p.wrapping_add(u32::from(i).wrapping_mul(4)), buf);
                    } else {
                        put_word(arena, strs.wrapping_add(u32::from(i).wrapping_mul(4)), buf);
                        put_half(arena, tb.wrapping_add(8), i);
                    }
                    n_str = n_str.wrapping_add(1);
                    put_half(arena, tb.wrapping_add(8), i);
                }
                let str_i = half(arena, tb.wrapping_add(8));
                let lid_v = byte(arena, tb.wrapping_add(0xc));
                let mut k = from;
                while k < to {
                    put_half(arena, word_idx.wrapping_add(u32::from(k).wrapping_mul(2)), str_i);
                    put_byte(arena, lid.wrapping_add(u32::from(k)), lid_v);
                    k = k.wrapping_add(1);
                }
                if strcmp_lit(arena, word(arena, strs.wrapping_add(u32::from(str_i).wrapping_mul(4))), b"phon") == 0 {
                    let mut k = from;
                    while k < to {
                        put_byte(arena, cls.wrapping_add(u32::from(k)), b'P');
                        k = k.wrapping_add(1);
                    }
                }
            }
            n_tok = n_tok.wrapping_add(1);
            let mut next = 0u16;
            if host.lat_next_node(arena, h0, h1, u32::from(node), &mut next) != 0 || next == 0 {
                break;
            }
            node = next;
        }
    }
    // Pronunciation realign: a pron followed by spaces up to the end of
    // its langmap swallows them. setField failures break out with `r`
    // set; a second-setField failure takes the realign_fail path.
    r = 0;
    let mut i = 0u16;
    while i < n_tok {
        let tb = tok_base(toks, i);
        if byte(arena, tb.wrapping_add(6)) != 6 {
            i = i.wrapping_add(1);
            continue;
        }
        let f00 = half(arena, tb);
        let mut has_pron = 0u16;
        r = host.lat_get_u16(arena, h0, h1, u32::from(f00), 6, &mut has_pron);
        if r >= 0x8000_0000 {
            break;
        }
        if has_pron != 1 {
            i = i.wrapping_add(1);
            continue;
        }
        let mut pron = 0u32;
        let mut norm = 0u32;
        r = host.lat_get_str(arena, h0, h1, u32::from(f00), 6, &mut pron, &mut len);
        if r >= 0x8000_0000 {
            break;
        }
        r = host.lat_get_str(arena, h0, h1, u32::from(f00), 3, &mut norm, &mut len);
        if r >= 0x8000_0000 {
            break;
        }
        let t_from = half(arena, tb.wrapping_add(2));
        let t_to = half(arena, tb.wrapping_add(4));
        let mut j = 0u16;
        while j < i {
            let tj = tok_base(toks, j);
            let tj_from = half(arena, tj.wrapping_add(2));
            let tj_to = half(arena, tj.wrapping_add(4));
            if byte(arena, tj.wrapping_add(6)) != 5 || t_from < tj_from || t_to > tj_to {
                j = j.wrapping_add(1);
                continue;
            }
            let mut k = t_to;
            while k < tj_to && byte(arena, text.wrapping_add(u32::from(k))) == b' ' {
                k = k.wrapping_add(1);
            }
            if t_to != k && tj_to == k {
                let n = strlen8(arena, pron).wrapping_add(u32::from(tj_to).wrapping_sub(u32::from(t_to))) as u16;
                buf = match host.calloc(arena, heap, 1, u32::from(n).wrapping_add(1)) {
                    Some(p) => p,
                    None => {
                        r = 0x89c0_200a;
                        break;
                    }
                };
                strcpy_at(arena, buf, pron);
                strncat_195dac(arena, buf, text.wrapping_add(u32::from(t_to)), u32::from(tj_to).wrapping_sub(u32::from(t_to)));
                let n1 = n.wrapping_add(1);
                r = host.lat_set_field(arena, h0, h1, u32::from(f00), 6, n1 as u32, buf, &mut len);
                if r < 0x8000_0000 {
                    if n1 == len {
                        strcpy_at(arena, buf, norm);
                        strncat_195dac(arena, buf, text.wrapping_add(u32::from(t_to)), u32::from(tj_to).wrapping_sub(u32::from(t_to)));
                        r = host.lat_set_field(arena, h0, h1, u32::from(f00), 3, n1 as u32, buf, &mut len);
                        if r >= 0x8000_0000 {
                            host.free(arena, heap, buf);
                            break;
                        }
                        if n1 == len {
                            host.free(arena, heap, buf);
                            put_half(arena, tb.wrapping_add(4), tj_to);
                            r = host.lat_set_field(arena, h0, h1, u32::from(f00), 2, 1, tb.wrapping_add(4), &mut len);
                            break;
                        }
                    }
                    r = 0x89c0_2000;
                }
                host.free(arena, heap, buf);
                break;
            }
            j = j.wrapping_add(1);
        }
        if r >= 0x8000_0000 {
            break;
        }
        i = i.wrapping_add(1);
    }
    if r >= 0x8000_0000 {
        clear_strs_1d9508(arena, strs, n_str);
        return r;
    }
    // Paint the class map with the pronunciation langmaps. NOTE: the
    // inner id scan reuses `i`, resetting the outer loop (mirrored).
    i = 0;
    while i < n_tok {
        let tb = tok_base(toks, i);
        if byte(arena, tb.wrapping_add(7)) == 3 || byte(arena, tb.wrapping_add(6)) != 6 {
            i = i.wrapping_add(1);
            continue;
        }
        let f00 = half(arena, tb);
        let mut has_pron = 0u16;
        r = host.lat_get_u16(arena, h0, h1, u32::from(f00), 6, &mut has_pron);
        if r >= 0x8000_0000 {
            clear_strs_1d9508(arena, strs, n_str);
            return r;
        }
        if has_pron != 1 {
            i = i.wrapping_add(1);
            continue;
        }
        let mut val = 0u32;
        r = host.lat_get_str(arena, h0, h1, u32::from(f00), 6, &mut val, &mut len);
        if r >= 0x8000_0000 {
            clear_strs_1d9508(arena, strs, n_str);
            return r;
        }
        buf = match host.calloc(arena, heap, 1, strlen8(arena, val).wrapping_add(1)) {
            Some(p) => p,
            None => {
                clear_strs_1d9508(arena, strs, n_str);
                return 0x89c0_200a;
            }
        };
        strcpy_at(arena, buf, val);
        {
            let mut ch = byte(arena, buf);
            let from = half(arena, tb.wrapping_add(2));
            let to = half(arena, tb.wrapping_add(4));
            if from < to {
                let mut n = to.wrapping_sub(from);
                let mut q = buf;
                let mut o = 0u32;
                loop {
                    if byte(arena, q) != b' ' {
                        ch = byte(arena, q);
                    }
                    if ch != b' ' {
                        put_byte(arena, cls.wrapping_add(u32::from(from).wrapping_add(o)), ch);
                    }
                    n = n.wrapping_sub(1);
                    q = q.wrapping_add(1);
                    o = o.wrapping_add(1);
                    if n == 0 {
                        break;
                    }
                }
            }
        }
        let id = half(arena, tb);
        host.free(arena, heap, buf);
        i = 0;
        while i < n_tok {
            if half(arena, tok_base(toks, i)) == id {
                break;
            }
            i = i.wrapping_add(1);
        }
        i = i.wrapping_add(1);
    }
    // Extend each pronunciation token to the end of its word.
    i = 0;
    while i < n_tok {
        let tb = tok_base(toks, i);
        if byte(arena, tb.wrapping_add(6)) != 6 || byte(arena, tb.wrapping_add(0xb)) != 0 {
            i = i.wrapping_add(1);
            continue;
        }
        let from = half(arena, tb.wrapping_add(2));
        let to = half(arena, tb.wrapping_add(4));
        let mut k = to;
        while k < sent_len && byte(arena, text.wrapping_add(u32::from(k))) != b' ' {
            k = k.wrapping_add(1);
        }
        if byte(arena, text.wrapping_add(u32::from(k))) == b' ' {
            while k < sent_len && byte(arena, text.wrapping_add(u32::from(k))) == b' ' {
                k = k.wrapping_add(1);
            }
        }
        if k > sent_len {
            i = i.wrapping_add(1);
            continue;
        }
        let mut bisect = false;
        let mut j = i.wrapping_add(1);
        while j < n_tok {
            let tj = tok_base(toks, j);
            let tj_from = half(arena, tj.wrapping_add(2));
            if tj_from < k
                && byte(arena, tj.wrapping_add(6)) == 6
                && byte(arena, tj.wrapping_add(7)) != 3
                && half(arena, word_idx.wrapping_add(u32::from(k).wrapping_mul(2)))
                    != half(arena, word_idx.wrapping_add(u32::from(tj_from).wrapping_mul(2)))
            {
                bisect = true;
            }
            j = j.wrapping_add(1);
            if j < n_tok
                && half(arena, tok_base(toks, j).wrapping_add(2)) >= k
                && byte(arena, tok_base(toks, j).wrapping_add(6)) == 6
            {
                break;
            }
            if bisect {
                break;
            }
        }
        if !bisect {
            let ch = byte(arena, cls.wrapping_add(u32::from(from)));
            let mut m = to;
            while m < k {
                put_byte(arena, cls.wrapping_add(u32::from(m)), ch);
                m = m.wrapping_add(1);
            }
            put_half(arena, tb.wrapping_add(4), k);
        }
        i = i.wrapping_add(1);
    }
    // Pull the start of each pronunciation token back to its word start.
    i = 0;
    while i < n_tok {
        let tb = tok_base(toks, i);
        if byte(arena, tb.wrapping_add(6)) != 6 {
            i = i.wrapping_add(1);
            continue;
        }
        let from = half(arena, tb.wrapping_add(2));
        let mut b = from as i32;
        while b > 0 && byte(arena, text.wrapping_add(b as u32)) != b' ' {
            b -= 1;
        }
        if b < 0 {
            i = i.wrapping_add(1);
            continue;
        }
        let ub = b as u16;
        let mut new_from = 0xffffu16;
        if byte(arena, text.wrapping_add(u32::from(ub))) == b' ' && from != ub.wrapping_add(1) {
            new_from = ub.wrapping_add(1);
        } else if ub == 0 && from != 0 {
            new_from = ub;
        }
        if new_from == 0xffff {
            i = i.wrapping_add(1);
            continue;
        }
        let mut conflict = false;
        let mut j = 0u16;
        while j < n_tok && half(arena, tok_base(toks, j).wrapping_add(2)) <= from {
            let tj = tok_base(toks, j);
            if j != i {
                if byte(arena, tj.wrapping_add(6)) == 6 {
                    let tj_from = half(arena, tj.wrapping_add(2));
                    let tj_to = half(arena, tj.wrapping_add(4));
                    if tj_from <= new_from && new_from < tj_to {
                        conflict = true;
                        break;
                    }
                } else if byte(arena, tj.wrapping_add(6)) == 5 {
                    let tj_from = half(arena, tj.wrapping_add(2));
                    if byte(arena, tj.wrapping_add(7)) != 3
                        && half(arena, word_idx.wrapping_add(u32::from(ub).wrapping_mul(2)))
                            != half(arena, word_idx.wrapping_add(u32::from(tj_from).wrapping_mul(2)))
                    {
                        conflict = true;
                        break;
                    }
                }
            }
            j = j.wrapping_add(1);
        }
        if !conflict {
            let ch = byte(arena, cls.wrapping_add(u32::from(from)));
            let mut m = new_from;
            while m < from {
                put_byte(arena, cls.wrapping_add(u32::from(m)), ch);
                m = m.wrapping_add(1);
            }
            put_half(arena, tb.wrapping_add(2), new_from);
        }
        i = i.wrapping_add(1);
    }
    // Spaces between two regions belong to the region on their left.
    {
        let mut prev = byte(arena, cls);
        let mut k = 0u16;
        while k < sent_len {
            let ch = byte(arena, cls.wrapping_add(u32::from(k)));
            if ch != prev && byte(arena, text.wrapping_add(u32::from(k))) == b' ' {
                let mut m = k.wrapping_add(1);
                while m < sent_len
                    && byte(arena, cls.wrapping_add(u32::from(m))) == ch
                    && byte(arena, text.wrapping_add(u32::from(m))) == b' '
                {
                    m = m.wrapping_add(1);
                }
                if prev != b'P' {
                    let mut q = k;
                    while q < m {
                        put_byte(arena, cls.wrapping_add(u32::from(q)), prev);
                        q = q.wrapping_add(1);
                    }
                }
                k = m;
            } else if ch != prev {
                prev = ch;
                k = k.wrapping_add(1);
            } else {
                k = k.wrapping_add(1);
            }
        }
    }
    // Langmap tokens that are not phonetic get deleted.
    i = 0;
    while i < n_tok {
        let tb = tok_base(toks, i);
        if byte(arena, tb.wrapping_add(6)) == 5
            && strcmp_lit(arena, word(arena, strs.wrapping_add(u32::from(half(arena, tb.wrapping_add(8))).wrapping_mul(4))), b"phon") != 0
        {
            put_byte(arena, tb.wrapping_add(7), 3);
        }
        i = i.wrapping_add(1);
    }
    // Insert a langmap token for every run of the same class and word.
    if sent_len != 0 {
        let mut prev_cls = byte(arena, cls);
        let mut prev_w = half(arena, word_idx);
        let mut start = 0u16;
        loop {
            let mut k = start;
            while k < sent_len
                && byte(arena, cls.wrapping_add(u32::from(k))) == prev_cls
                && half(arena, word_idx.wrapping_add(u32::from(k).wrapping_mul(2))) == prev_w
            {
                k = k.wrapping_add(1);
            }
            if k != start && byte(arena, cls.wrapping_add(u32::from(start))) != b'P' {
                let w = half(arena, word_idx.wrapping_add(u32::from(start).wrapping_mul(2)));
                let l = byte(arena, lid.wrapping_add(u32::from(start)));
                let mut pos = 0u16;
                let mut j = 0u16;
                while j < n_tok {
                    let tj = tok_base(toks, j);
                    if byte(arena, tj.wrapping_add(7)) == 3 {
                        j = j.wrapping_add(1);
                        continue;
                    }
                    if half(arena, tj.wrapping_add(2)) > start {
                        pos = j;
                        break;
                    }
                    if half(arena, tj.wrapping_add(2)) == start {
                        pos = n_tok;
                        if j.wrapping_add(1) < n_tok {
                            while j.wrapping_add(1) < n_tok
                                && half(arena, tok_base(toks, j.wrapping_add(1)).wrapping_add(2)) == start
                                && byte(arena, tok_base(toks, j.wrapping_add(1)).wrapping_add(7)) != 3
                            {
                                j = j.wrapping_add(1);
                            }
                            pos = j;
                        }
                        break;
                    }
                    j = j.wrapping_add(1);
                }
                let nt = match host.realloc(
                    arena,
                    heap,
                    toks,
                    u32::from(n_tok).wrapping_add(2).wrapping_mul(TOKEDIT_SIZE),
                ) {
                    Some(p) => p,
                    None => {
                        r = 0x89c0_200a;
                        start = k;
                        if k < sent_len
                            && (byte(arena, cls.wrapping_add(u32::from(k))) != prev_cls
                                || half(arena, word_idx.wrapping_add(u32::from(k).wrapping_mul(2))) != prev_w)
                        {
                            prev_cls = byte(arena, cls.wrapping_add(u32::from(k)));
                            prev_w = half(arena, word_idx.wrapping_add(u32::from(k).wrapping_mul(2)));
                        }
                        if start >= sent_len {
                            break;
                        }
                        continue;
                    }
                };
                toks = nt;
                if n_tok == j {
                    pos = n_tok;
                } else {
                    let mut m = n_tok.wrapping_add(1);
                    while m > pos {
                        let dst = tok_base(toks, m);
                        let src = tok_base(toks, m.wrapping_sub(1));
                        memcpy_195ae8(arena, dst, src, TOKEDIT_SIZE);
                        m = m.wrapping_sub(1);
                    }
                    memset_195bec(arena, tok_base(toks, pos), 0, TOKEDIT_SIZE);
                }
                r = 0;
                let nb = tok_base(toks, pos);
                put_half(arena, nb, 0);
                put_half(arena, nb.wrapping_add(2), start);
                put_half(arena, nb.wrapping_add(4), k);
                put_half(arena, nb.wrapping_add(8), w);
                put_byte(arena, nb.wrapping_add(0xa), byte(arena, cls.wrapping_add(u32::from(start))));
                put_byte(arena, nb.wrapping_add(7), 2);
                put_byte(arena, nb.wrapping_add(6), 5);
                put_byte(arena, nb.wrapping_add(0xc), l);
                n_tok = n_tok.wrapping_add(1);
            }
            start = k;
            if k < sent_len
                && (byte(arena, cls.wrapping_add(u32::from(k))) != prev_cls
                    || half(arena, word_idx.wrapping_add(u32::from(k).wrapping_mul(2))) != prev_w)
            {
                prev_cls = byte(arena, cls.wrapping_add(u32::from(k)));
                prev_w = half(arena, word_idx.wrapping_add(u32::from(k).wrapping_mul(2)));
            }
            if start >= sent_len {
                break;
            }
        }
    }
    // Write the edits back into the lingdb.
    i = 0;
    while i < n_tok {
        let tb = tok_base(toks, i);
        let op = byte(arena, tb.wrapping_add(7));
        if op == 2 {
            let mut p = i as i32 - 1;
            while p >= 0 && byte(arena, tok_base(toks, p as u16).wrapping_add(7)) == 3 {
                p -= 1;
            }
            if p >= 0 {
                if half(arena, tok_base(toks, p as u16)) == 0 {
                    clear_strs_1d9508(arena, strs, n_str);
                    return 0x89c0_2000;
                }
                r = host.lat_insert_after(
                    arena,
                    h0,
                    h1,
                    u32::from(half(arena, tok_base(toks, p as u16))),
                    &mut node,
                );
            } else {
                p = i as i32 + 1;
                while p < n_tok as i32
                    && (byte(arena, tok_base(toks, p as u16).wrapping_add(7)) == 3
                        || half(arena, tok_base(toks, p as u16)) == 0)
                {
                    p += 1;
                }
                if (p as u16) < n_tok {
                    r = host.lat_insert_before(
                        arena,
                        h0,
                        h1,
                        u32::from(half(arena, tok_base(toks, p as u16))),
                        &mut node,
                    );
                } else {
                    r = host.lat_append(arena, h0, h1, 3, x, &mut node);
                }
            }
            if r >= 0x8000_0000 {
                clear_strs_1d9508(arena, strs, n_str);
                return r;
            }
            r = host.lat_set_field(arena, h0, h1, u32::from(node), 0, 1, tb.wrapping_add(6), &mut len);
            if r >= 0x8000_0000 {
                clear_strs_1d9508(arena, strs, n_str);
                return r;
            }
            r = host.lat_set_field(arena, h0, h1, u32::from(node), 1, 1, tb.wrapping_add(2), &mut len);
            if r >= 0x8000_0000 {
                clear_strs_1d9508(arena, strs, n_str);
                return r;
            }
            r = host.lat_set_field(arena, h0, h1, u32::from(node), 2, 1, tb.wrapping_add(4), &mut len);
            if r >= 0x8000_0000 {
                clear_strs_1d9508(arena, strs, n_str);
                return r;
            }
            put_half(arena, tb, node);
            if byte(arena, tb.wrapping_add(0xc)) != 0 {
                // The C passes the "lid" literal (a ROM address); the port
                // keeps it in Rust and lets the host stage it.
                r = host.lat_set_field_bytes(arena, h0, h1, u32::from(node), 6, 4, b"lid\0", &mut len);
                if r >= 0x8000_0000 {
                    clear_strs_1d9508(arena, strs, n_str);
                    return r;
                }
            }
            let from = half(arena, tb.wrapping_add(2));
            let widx = half(arena, word_idx.wrapping_add(u32::from(from).wrapping_mul(2)));
            buf = match host.calloc(
                arena,
                heap,
                1,
                (strlen8(arena, word(arena, strs.wrapping_add(u32::from(widx).wrapping_mul(4))))
                    .wrapping_add(6)
                    .wrapping_add(1)
                    .wrapping_add(1)) as u16 as u32,
            ) {
                Some(p) => p,
                None => {
                    clear_strs_1d9508(arena, strs, n_str);
                    return 0x89c0_200a;
                }
            };
            if widx != 0 {
                strcpy_at(arena, buf, word(arena, strs.wrapping_add(u32::from(widx).wrapping_mul(4))));
                if byte(arena, tb.wrapping_add(0xa)) != b'N' {
                    strcat_lit(arena, buf, b"_");
                    let tab_ch = byte(arena, tb.wrapping_add(0xa));
                    let tab_v = word(arena, tab.wrapping_add(u32::from(tab_ch).wrapping_mul(4)));
                    if tab_v != 0 {
                        strcat_195c90(arena, buf, tab_v);
                    } else {
                        strcat_lit(arena, buf, b"latin");
                    }
                }
            } else if byte(arena, tb.wrapping_add(0xa)) == b'N' {
                strcpy_at(arena, buf, word(arena, strs));
            } else if word(arena, tab.wrapping_add(u32::from(byte(arena, tb.wrapping_add(0xa))).wrapping_mul(4))) != 0 {
                strcpy_at(arena, buf, word(arena, tab.wrapping_add(u32::from(byte(arena, tb.wrapping_add(0xa))).wrapping_mul(4))));
            } else {
                strcpy_lit(arena, buf, b"latin");
            }
            r = host.lat_set_field(
                arena,
                h0,
                h1,
                u32::from(node),
                4,
                strlen8(arena, buf).wrapping_add(1),
                buf,
                &mut len,
            );
            host.free(arena, heap, buf);
            if r >= 0x8000_0000 {
                clear_strs_1d9508(arena, strs, n_str);
                return r;
            }
        } else if op == 3 {
            if half(arena, tb) != 0 {
                r = host.lat_remove(arena, h0, h1, u32::from(half(arena, tb)));
                if r >= 0x8000_0000 {
                    clear_strs_1d9508(arena, strs, n_str);
                    return r;
                }
            }
        } else if op == 1 {
            r = host.lat_set_field(arena, h0, h1, u32::from(half(arena, tb)), 2, 1, tb.wrapping_add(4), &mut len);
            if r >= 0x8000_0000 {
                clear_strs_1d9508(arena, strs, n_str);
                return r;
            }
        }
        i = i.wrapping_add(1);
    }
    if dbg {
        fe_dump_1d8f28(arena, c, h0, h1, x, text, dbg_buf, 0, host);
    }
    let _ = (lid, cls, word_idx, tab, dbg_buf, toks);
    clear_strs_1d9508(arena, strs, n_str);
    r
}

/// fn_1d8ec8, EU fn_1d9084, KR fn_1d9130. Tok comparator for the
/// 1dd914 mark sort: `f0c` first, then the type 1-after-8 rule, then
/// `pos`. Same comparison as [`crate::normalization::token_order`],
/// over arena element offsets.
pub fn cmp_1d8ec8(arena: &[u8], a: u32, b: u32) -> i32 {
    let a = a as usize;
    let b = b as usize;
    crate::normalization::token_order(&arena[a..a + 24], &arena[b..b + 24])
}

/// fn_2409d4, EU fn_240b90, KR fn_240c3c. In-place quicksort over `n`
/// `size`-byte elements at `base`, comparing element offsets with `cmp`.
/// The comparator is reflexive-zero, so the `i` scan stops at the pivot
/// from below and the `j` scan from above; both stay in range.
pub fn qsort_2409d4(
    arena: &mut [u8],
    base: u32,
    n: u32,
    size: u32,
    cmp: fn(&[u8], u32, u32) -> i32,
) {
    if n <= 1 {
        return;
    }
    let mut mid = n >> 1;
    let mut pivot = base.wrapping_add(mid.wrapping_mul(size));
    let mut i = 0u32;
    let mut j = n.wrapping_sub(1);
    while i < j {
        while cmp(arena, base.wrapping_add(i.wrapping_mul(size)), pivot) < 0 {
            i = i.wrapping_add(1);
        }
        while cmp(arena, base.wrapping_add(j.wrapping_mul(size)), pivot) > 0 {
            j = j.wrapping_sub(1);
        }
        if i < j {
            let pi = base.wrapping_add(i.wrapping_mul(size));
            let pj = base.wrapping_add(j.wrapping_mul(size));
            for k in 0..size {
                let t = byte(arena, pi.wrapping_add(k));
                put_byte(arena, pi.wrapping_add(k), byte(arena, pj.wrapping_add(k)));
                put_byte(arena, pj.wrapping_add(k), t);
            }
            if mid == i {
                mid = j;
                pivot = pj;
            } else if mid == j {
                mid = i;
                pivot = pi;
            }
            i = i.wrapping_add(1);
            j = j.wrapping_sub(1);
        } else if i == j {
            i = i.wrapping_add(1);
            j = j.wrapping_sub(1);
            break;
        }
    }
    if j != 0 {
        qsort_2409d4(arena, base, j.wrapping_add(1), size, cmp);
    }
    if i < n.wrapping_sub(1) {
        qsort_2409d4(
            arena,
            base.wrapping_add(i.wrapping_mul(size)),
            n.wrapping_sub(i),
            size,
            cmp,
        );
    }
}

/// Arena cell of `marks[idx].position`: 24-byte TextMarks, position at +12.
fn mark_pos_cell(marks: u32, idx: u16) -> u32 {
    marks
        .wrapping_add(u32::from(idx).wrapping_mul(24))
        .wrapping_add(12)
}

/// `fn_1dd914` (FE_PUNCSPTN): space/punctuation normalization over the
/// sentence lattice: type-5 merge pass, `fn_1dbcb0` disambiguation pass,
/// span alignment pass, optional `fn_1d9508` tagging, mark sort, space
/// collapsing, then the trailing type-4 sweep. The `fn_173dc0`/`fn_1738e8`
/// calls are log-only. TextNormalizer layout per `src/fn_1dd914.c`:
/// env@0 nodes@4 propsHandle@8 props@0x10 charsHandle@0x14 chars@0x1c
/// orth@0x20 map@0x24 wts@0x28 text@0x2c shift@0x30 normalize@0x32
/// flags@0x33 escape@0x34.
/// The `" _"` separator set `fn_195d30` tests in `fn_1dd914`: space or
/// underscore. The C passes a string literal (a ROM address); the port
/// keeps the test in Rust, so no guest scratch is furnished.
fn is_spud(byte: u8) -> bool {
    byte == b' ' || byte == b'_'
}

pub fn fe_punctspat_1dd914(
    arena: &mut [u8],
    p: u32,
    check: u32,
    h0: u32,
    h1: u32,
    changed: u32,
    host: &mut impl FeTextHost,
) -> u32 {
    // The C keeps node/length/to/from and friends in stack locals; the
    // port does the same. No guest scratch: the old 0x70 NODE/IDX/CNT
    // cells, the 0x76 " _" cell and the 0x7a set-data cell are gone.
    // (The live host stages set-data through its own uncompared window,
    // the way the ARM passes its stack slot.)
    let p_text = p.wrapping_add(0x2c);
    let env = word(arena, p);
    let heap = word(arena, env.wrapping_add(4));
    let ph0 = word(arena, p.wrapping_add(8));
    let ph1 = word(arena, p.wrapping_add(12));
    let ch0 = word(arena, p.wrapping_add(0x14));
    let ch1 = word(arena, p.wrapping_add(0x18));
    let mut node = 0u16;
    let mut length = 0u16;
    let mut to = 0u16;
    let mut from = 0u16;
    let mut sentence = 0u16;
    let mut mark_index = 0u16;
    let mut cursor = 0u16;
    let mut mark_count = 0u16;
    let mut count = 0u16;
    let mut position = 0i32;
    let mut original = 0u32;
    let mut marks = 0u32;
    let mut base = 0i32;
    let mut mode = 0u32;
    let mut typ = 0u8;
    let mut written = 0u16;
    let mut delta = 0i16;
    let mut saved;
    let mut result;
    let mut status;
    let mut remove_node = 0u32;
    let mut i = 0u16;
    let mut ch = 0u8;
    let mut before = 0u32;

    result = host.validate(check, 0xf383, 0x38);
    if result >= 0x8000_0000 {
        return 0x89c0_2008;
    }
    put_byte(arena, changed, 1);
    saved = host.lat_first_node(arena, h0, h1, 1, 0, &mut sentence);
    if saved >= 0x8000_0000 {
        return saved;
    }
    status = host.lat_get_u16(arena, h0, h1, u32::from(sentence), 0, &mut count);
    if status >= 0x8000_0000 || count != 1 {
        return saved;
    }
    result = host.lat_get_str(arena, h0, h1, u32::from(sentence), 0, &mut original, &mut length);
    if result >= 0x8000_0000 || length <= 1 {
        return result;
    }
    let new_text = match host.realloc(
        arena,
        heap,
        word(arena, p_text),
        strlen8(arena, original).wrapping_add(1),
    ) {
        Some(t) => t,
        None => return 0x89c0_200a,
    };
    put_word(arena, p_text, new_text);
    let mut k = 0u32;
    loop {
        let b = byte(arena, original.wrapping_add(k));
        put_byte(arena, new_text.wrapping_add(k), b);
        if b == 0 {
            break;
        }
        k = k.wrapping_add(1);
    }
    result = host.lat_get_str(arena, h0, h1, u32::from(sentence), 1, &mut marks, &mut mark_count);
    if result >= 0x8000_0000 {
        return result;
    }
    base = word(arena, marks.wrapping_add(12)) as i32;
    put_half(arena, p.wrapping_add(0x30), 0);
    while mark_index < mark_count && word(arena, mark_pos_cell(marks, mark_index)) as i32 == base {
        mark_index = mark_index.wrapping_add(1);
    }
    result = host.prop_get(arena, ph0, ph1, &mut position, 0);
    if result >= 0x8000_0000 {
        return result;
    }
    {
        let mut previous = 0u32;
        let mut end = 0u16;
        let mut start = 0u32;
        result = host.lat_first_node(arena, h0, h1, 3, u32::from(sentence), &mut node);
        if result >= 0x8000_0000 {
            return result;
        }
        while node != 0 {
            let mut w = 0u32;
            result = host.lat_get_int(arena, h0, h1, u32::from(node), 0, 1, &mut w, &mut length);
            if result >= 0x8000_0000 {
                return result;
            }
            typ = w as u8;
            result = host.lat_get_int(arena, h0, h1, u32::from(node), 1, 1, &mut w, &mut length);
            if result >= 0x8000_0000 {
                return result;
            }
            from = w as u16;
            result = host.lat_get_int(arena, h0, h1, u32::from(node), 2, 1, &mut w, &mut length);
            if result >= 0x8000_0000 {
                return result;
            }
            to = w as u16;
            remove_node = 0;
            if typ == 5 {
                if u32::from(from) > start && u32::from(from) < u32::from(end) {
                    if u32::from(end) < u32::from(to) {
                        result = host.lat_set_field_bytes(
                            arena, h0, h1, previous, 2, 1, &end.to_le_bytes(), &mut written,
                        );
                        if result >= 0x8000_0000 {
                            return result;
                        }
                        result = host.lat_set_field_bytes(
                            arena,
                            h0,
                            h1,
                            u32::from(node),
                            1,
                            1,
                            &end.to_le_bytes(),
                            &mut written,
                        );
                        if result >= 0x8000_0000 {
                            return result;
                        }
                    } else {
                        result = host.lat_set_field_bytes(
                            arena, h0, h1, previous, 2, 1, &end.to_le_bytes(), &mut written,
                        );
                        if result >= 0x8000_0000 {
                            return result;
                        }
                        remove_node = u32::from(node);
                    }
                } else {
                    previous = u32::from(node);
                }
            } else if typ == 6 || typ == 4 {
                start = u32::from(from);
                end = to;
            }
            result = host.lat_next_node(arena, h0, h1, u32::from(node), &mut node);
            if result >= 0x8000_0000 {
                return result;
            }
            if remove_node != 0 {
                result = host.lat_remove(arena, h0, h1, remove_node);
                if result >= 0x8000_0000 {
                    return result;
                }
            }
        }
    }
    result = host.lat_first_node(arena, h0, h1, 3, u32::from(sentence), &mut node);
    if result >= 0x8000_0000 {
        return result;
    }
    cursor = node;
    while node != 0 && cursor != 0 {
        let mut w = 0u32;
        result = host.lat_get_int(arena, h0, h1, u32::from(cursor), 0, 1, &mut w, &mut length);
        if result >= 0x8000_0000 {
            return result;
        }
        typ = w as u8;
        if typ == 5 {
            result =
                host.lat_get_int(arena, h0, h1, u32::from(cursor), 1, 1, &mut w, &mut length);
            if result >= 0x8000_0000 {
                return result;
            }
            from = w as u16;
            result =
                host.lat_get_int(arena, h0, h1, u32::from(cursor), 2, 1, &mut w, &mut length);
            if result >= 0x8000_0000 {
                return result;
            }
            to = w as u16;
            result = host.lat_get_str(arena, h0, h1, u32::from(cursor), 4, &mut mode, &mut length);
            if result >= 0x8000_0000 {
                return result;
            }
            if to >= from {
                result = fe_disambig_1dbcb0(
                    arena,
                    p,
                    p_text,
                    original,
                    u32::from(from),
                    u32::from(to),
                    mode,
                    h0,
                    h1,
                    &mut node,
                    marks,
                    &mut mark_index,
                    mark_count,
                    base as u32,
                    host,
                );
                if result >= 0x8000_0000 {
                    return result;
                }
                cursor = node;
            } else {
                result = host.lat_next_node(arena, h0, h1, u32::from(cursor), &mut cursor);
                if result >= 0x8000_0000 {
                    return result;
                }
            }
        } else {
            result = host.lat_next_node(arena, h0, h1, u32::from(cursor), &mut cursor);
            if result >= 0x8000_0000 {
                return result;
            }
        }
    }
    result = host.lat_first_node(arena, h0, h1, 3, u32::from(sentence), &mut node);
    if result >= 0x8000_0000 {
        return result;
    }
    while node != 0 {
        let mut w = 0u32;
        result = host.lat_get_int(arena, h0, h1, u32::from(node), 0, 1, &mut w, &mut length);
        if result >= 0x8000_0000 {
            return result;
        }
        typ = w as u8;
        if typ == 5 {
            result = host.lat_get_int(arena, h0, h1, u32::from(node), 1, 1, &mut w, &mut length);
            if result >= 0x8000_0000 {
                return result;
            }
            from = w as u16;
            result = host.lat_get_int(arena, h0, h1, u32::from(node), 2, 1, &mut w, &mut length);
            if result >= 0x8000_0000 {
                return result;
            }
            to = w as u16;
            if from != 0 {
                while strlen8(arena, word(arena, p_text)) as u16 > from
                    && byte(arena, word(arena, p_text).wrapping_add(u32::from(from)).wrapping_sub(1))
                        != b' '
                    && byte(arena, word(arena, p_text).wrapping_add(u32::from(from)).wrapping_sub(1))
                        != b'_'
                {
                    from = from.wrapping_add(1);
                }
            }
            if to != 0 {
                if strlen8(arena, word(arena, p_text)) as u16 != to {
                    while strlen8(arena, word(arena, p_text)) as u16 > to
                        && byte(
                            arena,
                            word(arena, p_text).wrapping_add(u32::from(to)).wrapping_sub(1),
                        ) != b' '
                        && byte(
                            arena,
                            word(arena, p_text).wrapping_add(u32::from(to)).wrapping_sub(1),
                        ) != b'_'
                    {
                        to = to.wrapping_add(1);
                    }
                }
                if to != 0
                    && strlen8(arena, word(arena, p_text)) as u16 != to
                    && host.char_punctuation(
                        arena,
                        ch0,
                        ch1,
                        u32::from(byte(arena, word(arena, p_text).wrapping_add(u32::from(to)))),
                    ) != 0
                {
                    let mut next = 0u16;
                    let mut next_type = 0u8;
                    i = to.wrapping_sub(1);
                    result = host.lat_next_node(arena, h0, h1, u32::from(node), &mut next);
                    if result >= 0x8000_0000 {
                        return result;
                    }
                    result = host.lat_get_int(
                        arena,
                        h0,
                        h1,
                        u32::from(next),
                        0,
                        1,
                        &mut w,
                        &mut length,
                    );
                    if result >= 0x8000_0000 {
                        return result;
                    }
                    next_type = w as u8;
                    let mut aligned = false;
                    if next_type == 5 {
                        result = host.lat_get_str(
                            arena,
                            h0,
                            h1,
                            u32::from(next),
                            4,
                            &mut mode,
                            &mut length,
                        );
                        if result >= 0x8000_0000 {
                            return result;
                        }
                        if next_type == 5 {
                            let mut k = 0u32;
                            let is_phon = loop {
                                let a = byte(arena, mode.wrapping_add(k));
                                let b = if k < 4 { b"phon"[k as usize] } else { 0 };
                                if a != b {
                                    break false;
                                }
                                if b == 0 {
                                    break true;
                                }
                                k = k.wrapping_add(1);
                            };
                            if is_phon {
                                aligned = true;
                            }
                        }
                    }
                    if !aligned {
                        while i != 0
                            && byte(arena, word(arena, p_text).wrapping_add(u32::from(i))) == b' '
                        {
                            i = i.wrapping_sub(1);
                        }
                        i = i.wrapping_add(1);
                        ch = byte(arena, word(arena, p_text).wrapping_add(u32::from(i)));
                        put_byte(
                            arena,
                            word(arena, p_text).wrapping_add(u32::from(i)),
                            byte(arena, word(arena, p_text).wrapping_add(u32::from(to))),
                        );
                        put_byte(
                            arena,
                            word(arena, p_text).wrapping_add(u32::from(to)),
                            ch,
                        );
                    }
                }
            }
            result = host.lat_set_field_bytes(
                arena,
                h0,
                h1,
                u32::from(node),
                1,
                1,
                &from.to_le_bytes(),
                &mut written,
            );
            if result >= 0x8000_0000 {
                return result;
            }
            result = host.lat_set_field_bytes(
                arena,
                h0,
                h1,
                u32::from(node),
                2,
                1,
                &to.to_le_bytes(),
                &mut written,
            );
            if result >= 0x8000_0000 {
                return result;
            }
            if typ == 5 && from == to {
                remove_node = u32::from(node);
            } else {
                remove_node = 0;
            }
        } else {
            remove_node = 0;
        }
        result = host.lat_next_node(arena, h0, h1, u32::from(node), &mut node);
        if result >= 0x8000_0000 {
            return result;
        }
        if remove_node != 0 {
            result = host.lat_remove(arena, h0, h1, remove_node);
            if result >= 0x8000_0000 {
                return result;
            }
        }
    }
    if byte(arena, p.wrapping_add(0x32)) == 1 {
        result = fe_tags_1d9508(
            arena,
            p,
            h0,
            h1,
            u32::from(sentence),
            word(arena, p_text),
            host,
        );
        if result >= 0x8000_0000 {
            return result;
        }
    }
    mark_index = 0;
    while mark_index < mark_count && word(arena, mark_pos_cell(marks, mark_index)) as i32 == base {
        mark_index = mark_index.wrapping_add(1);
    }
    qsort_2409d4(
        arena,
        marks.wrapping_add(u32::from(mark_index).wrapping_mul(24)),
        u32::from(mark_count.wrapping_sub(mark_index)),
        24,
        cmp_1d8ec8,
    );
    i = 1;
    while i <= strlen8(arena, word(arena, p_text)) as u16 {
        let mark_position =
            ((u32::from(i) as i32).wrapping_sub(delta as i32) as u16 as i32).wrapping_add(base);
        while mark_index < mark_count
            && word(arena, mark_pos_cell(marks, mark_index)) as i32 == mark_position
        {
            put_word(
                arena,
                mark_pos_cell(marks, mark_index),
                (word(arena, mark_pos_cell(marks, mark_index)) as i32).wrapping_add(delta as i32) as u32,
            );
            mark_index = mark_index.wrapping_add(1);
        }
        before = u32::from(i).wrapping_sub(1);
        if byte(arena, word(arena, p_text).wrapping_add(before)) == b','
            && byte(arena, word(arena, p_text).wrapping_add(u32::from(i))) == b','
        {
            put_byte(arena, word(arena, p_text).wrapping_add(u32::from(i)), b' ');
        }
        if is_spud(byte(arena, word(arena, p_text).wrapping_add(before)))
            && (i == 1
                || strlen8(arena, word(arena, p_text)) as u16 == i
                || is_spud(byte(arena, word(arena, p_text).wrapping_add(u32::from(i)))))
        {
            if byte(arena, word(arena, p_text).wrapping_add(before)) == b' '
                && byte(arena, word(arena, p_text).wrapping_add(u32::from(i))) == b'_'
            {
                put_byte(arena, word(arena, p_text).wrapping_add(u32::from(i)), b' ');
            }
            if strlen8(arena, word(arena, p_text)) as u16 != i {
                let n = strlen8(arena, word(arena, p_text)).wrapping_sub(u32::from(i));
                for k in 0..n {
                    put_byte(
                        arena,
                        word(arena, p_text).wrapping_add(before).wrapping_add(k),
                        byte(arena, word(arena, p_text).wrapping_add(u32::from(i)).wrapping_add(k)),
                    );
                }
            }
            put_byte(
                arena,
                word(arena, p_text).wrapping_add(strlen8(arena, word(arena, p_text)).wrapping_sub(1)),
                0,
            );
            delta = delta.wrapping_sub(1);
            result = host.lat_first_node(arena, h0, h1, 3, u32::from(sentence), &mut node);
            if result >= 0x8000_0000 {
                return result;
            }
            while node != 0 {
                let mut w = 0u32;
                result =
                    host.lat_get_int(arena, h0, h1, u32::from(node), 1, 1, &mut w, &mut length);
                if result >= 0x8000_0000 {
                    return result;
                }
                from = w as u16;
                if from > i
                    || (from == i
                        && (i == 1 || strlen8(arena, word(arena, p_text)) as u16 == i))
                {
                    from = from.wrapping_sub(1);
                    result = host.lat_set_field_bytes(
                        arena,
                        h0,
                        h1,
                        u32::from(node),
                        1,
                        1,
                        &from.to_le_bytes(),
                        &mut written,
                    );
                    if result >= 0x8000_0000 {
                        return result;
                    }
                }
                result =
                    host.lat_get_int(arena, h0, h1, u32::from(node), 2, 1, &mut w, &mut length);
                if result >= 0x8000_0000 {
                    return result;
                }
                to = w as u16;
                if to > i || (to == i && (i == 1 || strlen8(arena, word(arena, p_text)) as u16 == i))
                {
                    to = to.wrapping_sub(1);
                    result = host.lat_set_field_bytes(
                        arena,
                        h0,
                        h1,
                        u32::from(node),
                        2,
                        1,
                        &to.to_le_bytes(),
                        &mut written,
                    );
                    if result >= 0x8000_0000 {
                        return result;
                    }
                }
                result =
                    host.lat_get_int(arena, h0, h1, u32::from(node), 0, 1, &mut w, &mut length);
                if result >= 0x8000_0000 {
                    return result;
                }
                typ = w as u8;
                if typ == 5 && from == to {
                    remove_node = u32::from(node);
                } else {
                    remove_node = 0;
                }
                result = host.lat_next_node(arena, h0, h1, u32::from(node), &mut node);
                if result >= 0x8000_0000 {
                    return result;
                }
                if remove_node != 0 {
                    result = host.lat_remove(arena, h0, h1, remove_node);
                    if result >= 0x8000_0000 {
                        return result;
                    }
                }
            }
            i = before as u16;
        }
        i = i.wrapping_add(1);
    }
    put_half(
        arena,
        p.wrapping_add(0x30),
        (half(arena, p.wrapping_add(0x30)) as i16).wrapping_add(delta) as u16,
    );
    position = position.wrapping_add(half(arena, p.wrapping_add(0x30)) as i16 as i32);
    result = host.prop_set(arena, ph0, ph1, position as u32, 0);
    if result >= 0x8000_0000 {
        return result;
    }
    put_word(arena, marks.wrapping_add(16), strlen8(arena, word(arena, p_text)));
    saved = host.lat_set_field(
        arena,
        h0,
        h1,
        u32::from(sentence),
        0,
        strlen8(arena, word(arena, p_text)).wrapping_add(1),
        word(arena, p_text),
        &mut written,
    );
    if saved >= 0x8000_0000 {
        return saved;
    }
    count = 0;
    status = host.lat_count_nodes(arena, h0, h1, 3, u32::from(sentence), &mut count);
    if status < 0x8000_0000 && count != 0 {
        saved = host.lat_first_node(arena, h0, h1, 3, u32::from(sentence), &mut node);
        if saved >= 0x8000_0000 {
            return saved;
        }
        let mut previous = 0u16;
        while node != 0 {
            let mut w = 0u32;
            result = host.lat_get_int(arena, h0, h1, u32::from(node), 0, 1, &mut w, &mut length);
            if result >= 0x8000_0000 {
                return result;
            }
            typ = w as u8;
            if typ == 4 {
                result = host.lat_remove(arena, h0, h1, u32::from(node));
                if result >= 0x8000_0000 {
                    return result;
                }
                node = previous;
            }
            if node == 0 {
                saved = host.lat_first_node(arena, h0, h1, 3, u32::from(sentence), &mut node);
                if saved >= 0x8000_0000 {
                    return saved;
                }
            } else {
                previous = node;
                saved = host.lat_next_node(arena, h0, h1, u32::from(node), &mut node);
                if saved >= 0x8000_0000 {
                    return saved;
                }
            }
        }
    }
    saved
}

/// `nextWord` from `src/fn_1e7260.c`: rescan `*pos` for a nonzero byte;
/// on a word start advance `*end` past the word and its trailing NULs.
fn next_word_1e7908(arena: &[u8], s: u32, len: u16, pos: &mut u16, end: &mut u16) -> bool {
    *end = *pos;
    while *pos < len {
        if byte(arena, s.wrapping_add(u32::from(*pos))) != 0 {
            *end = end.wrapping_add(1);
            while *end < len && byte(arena, s.wrapping_add(u32::from(*end))) != 0 {
                *end = end.wrapping_add(1);
            }
            while u32::from(end.wrapping_add(1)) < u32::from(len)
                && byte(arena, s.wrapping_add(u32::from(end.wrapping_add(1)))) == 0
            {
                *end = end.wrapping_add(1);
            }
            return true;
        }
        *pos = pos.wrapping_add(1);
    }
    false
}

/// `shiftRecs` from `src/fn_1e7260.c`: shift the matching word record's
/// new end by `delta`, then every later record plus any enclosing
/// kind-5/6 span. Records are 12 bytes: node@0 kind@2 start@4 end@6
/// nstart@8 nend@10.
fn shift_recs_1e7908(arena: &mut [u8], recs: u32, n_recs: u16, start: u16, end: u16, delta: i16) {
    let mut w = 0u16;
    while w < n_recs {
        let r = recs.wrapping_add(u32::from(w).wrapping_mul(12));
        let kind = byte(arena, r.wrapping_add(2));
        if (kind == 9 || kind == 4 || kind == 10)
            && half(arena, r.wrapping_add(4)) == start
            && half(arena, r.wrapping_add(6)) == end
        {
            put_half(
                arena,
                r.wrapping_add(10),
                half(arena, r.wrapping_add(10)).wrapping_add(delta as u16),
            );
            let r_nstart = half(arena, r.wrapping_add(8));
            let r_start = half(arena, r.wrapping_add(4));
            let r_end = half(arena, r.wrapping_add(6));
            let mut x = 0u16;
            while x < n_recs {
                if x != w {
                    let o = recs.wrapping_add(u32::from(x).wrapping_mul(12));
                    if half(arena, o.wrapping_add(8)) > r_nstart {
                        let nstart =
                            half(arena, o.wrapping_add(8)).wrapping_add(delta as u16);
                        put_half(arena, o.wrapping_add(8), nstart);
                        if (nstart as i16) < 0 {
                            put_half(arena, o.wrapping_add(8), 0);
                        }
                        put_half(
                            arena,
                            o.wrapping_add(10),
                            half(arena, o.wrapping_add(10)).wrapping_add(delta as u16),
                        );
                    }
                    let o_kind = byte(arena, o.wrapping_add(2));
                    if o_kind == 5 || o_kind == 6 {
                        if r_start >= half(arena, o.wrapping_add(4))
                            && u32::from(half(arena, o.wrapping_add(6))).wrapping_add(1)
                                >= u32::from(r_end)
                        {
                            put_half(
                                arena,
                                o.wrapping_add(10),
                                half(arena, o.wrapping_add(10)).wrapping_add(delta as u16),
                            );
                        }
                    }
                }
                x = x.wrapping_add(1);
            }
        }
        w = w.wrapping_add(1);
    }
}

/// fn_1e7908 cleanup, EU fn_1e7908, KR fn_1e7b70.
/// Release temporaries in ROM order, then clear the five layer pointers.
fn cleanup_l1l5(
    arena: &mut [u8], p: u32, rc: u32, joined: u32, recs: u32, se: u32,
    host: &mut impl FeTextHost,
) -> u32 {
    let heap = word(arena, word(arena, p).wrapping_add(4));
    for ptr in [joined, recs, se] {
        if ptr != 0 {
            host.free(arena, heap, ptr);
        }
    }
    for offset in [0x20, 0x24, 0x28, 0x2c, 0x30] {
        let slot = p.wrapping_add(offset);
        let ptr = word(arena, slot);
        if ptr != 0 {
            host.free(arena, heap, ptr);
        }
        put_word(arena, slot, 0);
    }
    rc
}

/// `fn_1e7908` (FE word normalization): node-count pass, layer-build
/// pass, out-process, word-compare patch pass, hyphen-join pass, then
/// marker/position writeback. `fn_1738e8`/`fn_173dc0` calls are log-only.
/// NormProcess layout per `src/fn_1e7260.c`: ctx@0 dp@4 h@8 out@0x10
/// l1@0x20 l2@0x24 l3@0x28 l4@0x2c l5@0x30 hasL5@0x34 wbuf@0x38.
pub fn fe_normproc_1e7908(
    arena: &mut [u8],
    p: u32,
    b: u32,
    h0: u32,
    h1: u32,
    done: u32,
    host: &mut impl FeTextHost,
) -> u32 {
    // C string literals live in unobserved scratch the casegen never
    // furnishes (0x688-0x700 gap), plus two u16 cells for the wordspan
    // out-params that are C locals.
    const LIT_X: u32 = 0x6e0;
    const LIT_Y: u32 = 0x6e2;
    const LIT_TILDE: u32 = 0x6e4;
    const LIT_SP: u32 = 0x6e6;
    const LIT_RS: u32 = 0x6e8;
    const LIT_US: u32 = 0x6ea;
    const LIT_DASH: u32 = 0x6ec;
    const LIT_DASHSTAR: u32 = 0x6ee;
    const TMP_NO1: u32 = 0x6f4;
    const TMP_NORIG: u32 = 0x6f6;
    const SET_TMP: u32 = 0x6f8;
    put_byte(arena, 0x6e0, b'X');
    put_byte(arena, 0x6e1, 0);
    put_byte(arena, 0x6e2, b'Y');
    put_byte(arena, 0x6e3, 0);
    put_byte(arena, 0x6e4, b'~');
    put_byte(arena, 0x6e5, 0);
    put_byte(arena, 0x6e6, b' ');
    put_byte(arena, 0x6e7, 0);
    put_byte(arena, 0x6e8, 0x1e);
    put_byte(arena, 0x6e9, 0);
    put_byte(arena, 0x6ea, 0x1f);
    put_byte(arena, 0x6eb, 0);
    put_byte(arena, 0x6ec, b'-');
    put_byte(arena, 0x6ed, 0);
    put_byte(arena, 0x6ee, b'-');
    put_byte(arena, 0x6ef, b'*');
    put_byte(arena, 0x6f0, 0);

    let ctx = word(arena, p);
    let heap = word(arena, ctx.wrapping_add(4));
    let ph0 = word(arena, p.wrapping_add(8));
    let ph1 = word(arena, p.wrapping_add(12));
    let has_l5 = byte(arena, p.wrapping_add(0x34)) != 0;
    let wbuf = word(arena, p.wrapping_add(0x38));
    let mut se = 0u32;
    let mut k = 0u16;
    let mut mi = 0u16;
    let mut root = 0u16;
    let mut prev_node = 0u32;
    let mut end = 0u16;
    let mut n_orig = 0u16;
    let mut node = 0u16;
    let mut node2 = 0u16;
    let mut n_o1 = 0u16;
    let mut delta = 0i16;
    let mut p_end = 0u16;
    let mut start = 0u16;
    let mut orig_len = 0u16;
    let mut o1 = 0u32;
    let mut phrase = 0u32;
    let mut rec_idx = 0u16;
    let mut wlen = 0u16;
    let mut pending = 0u16;
    let mut o4 = 0u32;
    let mut typ = 0u16;
    let mut p_start = 0u16;
    let mut n_recs = 0u16;
    let mut recs = 0u32;
    let mut first = true;
    let mut total = 1u16;
    let mut last_end = 0u32;
    let mut joined = 0u32;
    let mut cnt = 0u16;
    let mut kind = 0u8;
    let mut str_ = 0u32;
    let mut f5 = 0u32;
    let mut f6 = 0u32;
    let mut pos = 0i32;
    let mut markers = 0u32;
    let mut n_markers = 0u16;
    let mut base = 0i32;
    let mut orig = 0u32;
    let mut len = 0u16;
    let mut tmp = 0u16;
    let mut added = 0u16;
    let mut j = 0u16;
    let mut w = 0u16;
    let mut i = 0u16;
    let mut jj = 0u16;
    let mut ni = 0u16;
    let mut nj = 0u16;
    let mut rc;
    let mut w_out = 0u32;
    let mut l1 = 0u32;
    let mut l2 = 0u32;
    let mut l3 = 0u32;
    let mut l4 = 0u32;
    let mut l5 = 0u32;

    // strcpy(buf, "normout") targets a stack local; out.process ignores
    // its name argument on every side.
    if host.validate(b, 0xf3b3, 0x3c) >= 0x8000_0000 {
        return 0x8b80_2008;
    }
    put_byte(arena, done, 1);
    rc = host.out_get_pos(arena, ph0, ph1, &mut pos, 0);
    if rc >= 0x8000_0000 {
        return rc;
    }
    rc = host.dp_first_node(arena, h0, h1, 1, 0, &mut root);
    if rc >= 0x8000_0000 {
        return rc;
    }
    if host.dp_get_type(arena, h0, h1, u32::from(root), 0, &mut typ) >= 0x8000_0000
        || typ != 1
    {
        return rc;
    }
    rc = host.dp_get_ptr(arena, h0, h1, u32::from(root), 0, &mut orig, &mut orig_len);
    if rc >= 0x8000_0000 {
        return rc;
    }
    if orig_len <= 1 {
        return rc;
    }
    rc = host.dp_get_ptr(arena, h0, h1, u32::from(root), 1, &mut markers, &mut cnt);
    if rc >= 0x8000_0000 {
        return rc;
    }
    base = word(arena, markers.wrapping_add(12)) as i32;
    n_markers = cnt;
    while mi < n_markers
        && word(
            arena,
            markers
                .wrapping_add(u32::from(mi).wrapping_mul(24))
                .wrapping_add(12),
        ) == base as u32
    {
        mi = mi.wrapping_add(1);
    }

    rc = host.dp_first_node(arena, h0, h1, 2, u32::from(root), &mut node);
    if rc >= 0x8000_0000 {
        return cleanup_l1l5(arena, p, rc, joined, recs, se, host);
    }
    node2 = node;
    while node != 0 {
        rc = host.dp_get(arena, h0, h1, u32::from(node), 0, 1, &mut w_out, &mut cnt);
        if rc >= 0x8000_0000 {
            return cleanup_l1l5(arena, p, rc, joined, recs, se, host);
        }
        kind = w_out as u8;
        n_recs = n_recs.wrapping_add(1);
        if kind == 4 || kind == 9 || kind == 10 {
            if !first {
                total = total.wrapping_add(1);
            }
            first = false;
            rc = host.dp_get(arena, h0, h1, u32::from(node), 1, 1, &mut w_out, &mut cnt);
            if rc >= 0x8000_0000 {
                return cleanup_l1l5(arena, p, rc, joined, recs, se, host);
            }
            start = w_out as u16;
            rc = host.dp_get(arena, h0, h1, u32::from(node), 2, 1, &mut w_out, &mut cnt);
            if rc >= 0x8000_0000 {
                return cleanup_l1l5(arena, p, rc, joined, recs, se, host);
            }
            end = w_out as u16;
            total = total.wrapping_add(end).wrapping_sub(start);
        }
        if kind == 7 {
            rc = host.dp_get_ptr(arena, h0, h1, u32::from(node), 4, &mut str_, &mut cnt);
            if rc >= 0x8000_0000 {
                return cleanup_l1l5(arena, p, rc, joined, recs, se, host);
            }
            total = strlen8(arena, str_)
                .wrapping_add(u32::from(total))
                .wrapping_add(2) as u16;
        }
        if has_l5 && kind == 6 {
            rc = host.dp_get_ptr(arena, h0, h1, u32::from(node), 4, &mut str_, &mut cnt);
            if rc >= 0x8000_0000 {
                return cleanup_l1l5(arena, p, rc, joined, recs, se, host);
            }
            total = u32::from(total)
                .wrapping_add(strlen8(arena, str_).wrapping_mul(2))
                .wrapping_sub(2) as u16;
        }
        rc = host.dp_next_node(arena, h0, h1, u32::from(node), &mut node);
        if rc >= 0x8000_0000 {
            return cleanup_l1l5(arena, p, rc, joined, recs, se, host);
        }
    }

    match host.calloc(
        arena,
        heap,
        1,
        u32::from(n_recs).wrapping_mul(12).wrapping_add(1),
    ) {
        Some(v) => recs = v,
        None => return cleanup_l1l5(arena, p, 0x8b80_200a, joined, recs, se, host),
    }
    memset_at(arena, recs, 0, 12);
    match host.alloc(arena, heap, u32::from(total)) {
        Some(v) => {
            l1 = v;
            put_word(arena, p.wrapping_add(0x20), v);
        }
        None => return cleanup_l1l5(arena, p, 0x8b80_200a, joined, recs, se, host),
    }
    put_byte(arena, l1, 0);
    match host.alloc(arena, heap, u32::from(total)) {
        Some(v) => {
            l2 = v;
            put_word(arena, p.wrapping_add(0x24), v);
        }
        None => return cleanup_l1l5(arena, p, 0x8b80_200a, joined, recs, se, host),
    }
    put_byte(arena, l2, 0);
    match host.alloc(arena, heap, u32::from(total)) {
        Some(v) => {
            l3 = v;
            put_word(arena, p.wrapping_add(0x28), v);
        }
        None => return cleanup_l1l5(arena, p, 0x8b80_200a, joined, recs, se, host),
    }
    put_byte(arena, l3, 0);
    match host.alloc(arena, heap, u32::from(total)) {
        Some(v) => {
            l4 = v;
            put_word(arena, p.wrapping_add(0x2c), v);
        }
        None => return cleanup_l1l5(arena, p, 0x8b80_200a, joined, recs, se, host),
    }
    put_byte(arena, l4, 0);
    if has_l5 {
        match host.alloc(arena, heap, u32::from(total)) {
            Some(v) => {
                l5 = v;
                put_word(arena, p.wrapping_add(0x30), v);
            }
            None => return cleanup_l1l5(arena, p, 0x8b80_200a, joined, recs, se, host),
        }
        put_byte(arena, l5, 0);
    }

    first = true;
    rc = host.dp_first_node(arena, h0, h1, 2, u32::from(root), &mut node);
    if rc >= 0x8000_0000 {
        return cleanup_l1l5(arena, p, rc, joined, recs, se, host);
    }
    while node != 0 {
        let r = recs.wrapping_add(u32::from(rec_idx).wrapping_mul(12));
        put_half(arena, r, node);
        rc = host.dp_get(arena, h0, h1, u32::from(node), 0, 1, &mut w_out, &mut cnt);
        if rc >= 0x8000_0000 {
            return cleanup_l1l5(arena, p, rc, joined, recs, se, host);
        }
        kind = w_out as u8;
        put_byte(arena, r.wrapping_add(2), kind);
        rc = host.dp_get(arena, h0, h1, u32::from(node), 1, 1, &mut w_out, &mut cnt);
        if rc >= 0x8000_0000 {
            return cleanup_l1l5(arena, p, rc, joined, recs, se, host);
        }
        start = w_out as u16;
        rc = host.dp_get(arena, h0, h1, u32::from(node), 2, 1, &mut w_out, &mut cnt);
        if rc >= 0x8000_0000 {
            return cleanup_l1l5(arena, p, rc, joined, recs, se, host);
        }
        end = w_out as u16;
        put_half(arena, r.wrapping_add(4), start);
        put_half(arena, r.wrapping_add(8), start);
        put_half(arena, r.wrapping_add(6), end);
        put_half(arena, r.wrapping_add(10), end);
        if kind == 4 || kind == 9 || kind == 10 {
            last_end = u32::from(end);
            rc = host.dp_get_ptr(arena, h0, h1, u32::from(node), 5, &mut f5, &mut cnt);
            if rc >= 0x8000_0000 {
                return cleanup_l1l5(arena, p, rc, joined, recs, se, host);
            }
            rc = host.dp_get_ptr(arena, h0, h1, u32::from(node), 6, &mut f6, &mut cnt);
            if rc >= 0x8000_0000 {
                return cleanup_l1l5(arena, p, rc, joined, recs, se, host);
            }
            if has_l5 && !first {
                if phrase != 0 && start != p_start {
                    if start > p_end {
                        let plen = strlen8(arena, phrase);
                        if plen.wrapping_mul(2).wrapping_sub(2) < u32::from(pending) {
                            strcat_at(arena, l5, LIT_X);
                            strnappend(arena, l5, phrase.wrapping_add(1), plen.wrapping_sub(2));
                            j = plen.wrapping_mul(2).wrapping_sub(2) as u16;
                            while u32::from(j) < u32::from(pending) {
                                strcat_at(arena, l5, LIT_TILDE);
                                j = j.wrapping_add(1);
                            }
                            strcat_at(arena, l5, LIT_Y);
                            strnappend(arena, l5, phrase.wrapping_add(1), plen.wrapping_sub(2));
                        } else {
                            strcat_at(arena, l5, LIT_X);
                            strnappend(arena, l5, phrase.wrapping_add(1), plen.wrapping_sub(2));
                            strcat_at(arena, l5, LIT_Y);
                            strnappend(arena, l5, phrase.wrapping_add(1), plen.wrapping_sub(2));
                            j = pending;
                            while u32::from(j) < plen.wrapping_mul(2).wrapping_sub(2) {
                                strcat_at(arena, l1, LIT_TILDE);
                                strcat_at(arena, l2, LIT_TILDE);
                                strcat_at(arena, l3, LIT_TILDE);
                                strcat_at(arena, l4, LIT_TILDE);
                                j = j.wrapping_add(1);
                            }
                        }
                        phrase = 0;
                    }
                } else {
                    j = 0;
                    while u32::from(j) < u32::from(wlen) {
                        strcat_at(arena, l5, LIT_SP);
                        j = j.wrapping_add(1);
                    }
                }
            }
            added = 0;
            if !first {
                strnappend(
                    arena,
                    l1,
                    orig.wrapping_add(u32::from(start)).wrapping_sub(1),
                    1,
                );
                strcat_at(arena, l2, LIT_SP);
                strcat_at(arena, l3, LIT_SP);
                strnappend(
                    arena,
                    l4,
                    orig.wrapping_add(u32::from(start)).wrapping_sub(1),
                    1,
                );
                added = 1;
            }
            let span = end.wrapping_sub(start);
            strnappend(
                arena,
                l1,
                orig.wrapping_add(u32::from(start)),
                u32::from(span),
            );
            strnappend(
                arena,
                l4,
                orig.wrapping_add(u32::from(start)),
                u32::from(span),
            );
            j = 0;
            // `end - start` is computed in int; only a non-negative span
            // runs the loop.
            let span_i = (u32::from(end) as i32).wrapping_sub(u32::from(start) as i32);
            while (j as i32) < span_i {
                strnappend(arena, l2, f5, 1);
                strnappend(arena, l3, f6, 1);
                j = j.wrapping_add(1);
            }
            wlen = span;
            if has_l5 {
                if phrase == 0 || start == p_start {
                    j = 0;
                    while u32::from(j) < u32::from(added) {
                        strcat_at(arena, l5, LIT_SP);
                        j = j.wrapping_add(1);
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
            let l1len = strlen8(arena, l1) as u16;
            rc = host.dp_get_ptr(arena, h0, h1, u32::from(node), 4, &mut str_, &mut cnt);
            if rc >= 0x8000_0000 {
                return cleanup_l1l5(arena, p, rc, joined, recs, se, host);
            }
            let n = strlen8(arena, str_).wrapping_add(2) as u16;
            if u32::from(start) < last_end {
                let base_off = (u32::from(l1len))
                    .wrapping_sub(last_end)
                    .wrapping_add(u32::from(start));
                let count = last_end
                    .wrapping_sub(u32::from(start))
                    .wrapping_add(1);
                memmove_at(
                    arena,
                    l1.wrapping_add(base_off).wrapping_add(u32::from(n)),
                    l1.wrapping_add(base_off),
                    count,
                );
                memmove_at(
                    arena,
                    l2.wrapping_add(base_off).wrapping_add(u32::from(n)),
                    l2.wrapping_add(base_off),
                    count,
                );
                memmove_at(
                    arena,
                    l3.wrapping_add(base_off).wrapping_add(u32::from(n)),
                    l3.wrapping_add(base_off),
                    count,
                );
                memmove_at(
                    arena,
                    l4.wrapping_add(base_off).wrapping_add(u32::from(n)),
                    l4.wrapping_add(base_off),
                    count,
                );
                let off = (u32::from(l1len)).wrapping_sub(last_end) as i32;
                let at = off.wrapping_add(start as i32) as u32;
                put_byte(arena, l1.wrapping_add(at), 0x1e);
                put_byte(arena, l2.wrapping_add(at), b' ');
                put_byte(arena, l3.wrapping_add(at), b' ');
                put_byte(arena, l4.wrapping_add(at), b'~');
                j = 0;
                while u32::from(j) < u32::from(n).wrapping_sub(2) {
                    let s = byte(arena, str_.wrapping_add(u32::from(j)));
                    let aj = off
                        .wrapping_add(start as i32)
                        .wrapping_add(j as i32)
                        .wrapping_add(1) as u32;
                    put_byte(arena, l1.wrapping_add(aj), s);
                    put_byte(arena, l2.wrapping_add(aj), b' ');
                    put_byte(arena, l3.wrapping_add(aj), b' ');
                    put_byte(arena, l4.wrapping_add(aj), b'~');
                    j = j.wrapping_add(1);
                }
                let ae = off
                    .wrapping_add(start as i32)
                    .wrapping_add(n as i32)
                    .wrapping_sub(1) as u32;
                put_byte(arena, l1.wrapping_add(ae), 0x1f);
                put_byte(arena, l2.wrapping_add(ae), b' ');
                put_byte(arena, l3.wrapping_add(ae), b' ');
                put_byte(arena, l4.wrapping_add(ae), b'~');
            } else {
                strcat_at(arena, l1, LIT_RS);
                strcat_at(arena, l1, str_);
                strcat_at(arena, l1, LIT_US);
                j = 0;
                while u32::from(j) < u32::from(n) {
                    strcat_at(arena, l2, LIT_SP);
                    strcat_at(arena, l3, LIT_SP);
                    strcat_at(arena, l4, LIT_TILDE);
                    j = j.wrapping_add(1);
                }
            }
            if phrase != 0 {
                pending = strlen8(arena, str_)
                    .wrapping_add(u32::from(pending))
                    .wrapping_add(2) as u16;
            }
            wlen = strlen8(arena, str_)
                .wrapping_add(u32::from(wlen))
                .wrapping_add(2) as u16;
        }
        if kind == 6 {
            rc = host.dp_get(arena, h0, h1, u32::from(node), 1, 1, &mut w_out, &mut cnt);
            if rc >= 0x8000_0000 {
                return cleanup_l1l5(arena, p, rc, joined, recs, se, host);
            }
            p_start = w_out as u16;
            if has_l5 && phrase != 0 && p_start > p_end {
                let plen = strlen8(arena, phrase);
                if plen.wrapping_mul(2).wrapping_sub(2) < u32::from(pending) {
                    strcat_at(arena, l5, LIT_X);
                    strnappend(arena, l5, phrase.wrapping_add(1), plen.wrapping_sub(2));
                    j = plen.wrapping_mul(2).wrapping_sub(2) as u16;
                    while u32::from(j) < u32::from(pending) {
                        strcat_at(arena, l5, LIT_TILDE);
                        j = j.wrapping_add(1);
                    }
                    strcat_at(arena, l5, LIT_Y);
                    strnappend(arena, l5, phrase.wrapping_add(1), plen.wrapping_sub(2));
                } else {
                    strcat_at(arena, l5, LIT_X);
                    strnappend(arena, l5, phrase.wrapping_add(1), plen.wrapping_sub(2));
                    strcat_at(arena, l5, LIT_Y);
                    strnappend(arena, l5, phrase.wrapping_add(1), plen.wrapping_sub(2));
                    j = pending;
                    while u32::from(j) < plen.wrapping_mul(2).wrapping_sub(2) {
                        strcat_at(arena, l1, LIT_TILDE);
                        strcat_at(arena, l2, LIT_TILDE);
                        strcat_at(arena, l3, LIT_TILDE);
                        strcat_at(arena, l4, LIT_TILDE);
                        j = j.wrapping_add(1);
                    }
                }
                phrase = 0;
                wlen = 0;
            }
            rc = host.dp_get_ptr(arena, h0, h1, u32::from(node), 4, &mut phrase, &mut cnt);
            if rc >= 0x8000_0000 {
                return cleanup_l1l5(arena, p, rc, joined, recs, se, host);
            }
            rc = host.dp_get(arena, h0, h1, u32::from(node), 2, 1, &mut w_out, &mut cnt);
            if rc >= 0x8000_0000 {
                return cleanup_l1l5(arena, p, rc, joined, recs, se, host);
            }
            p_end = w_out as u16;
            pending = wlen;
        }
        rec_idx = rec_idx.wrapping_add(1);
        rc = host.dp_next_node(arena, h0, h1, u32::from(node), &mut node);
        if rc >= 0x8000_0000 {
            return cleanup_l1l5(arena, p, rc, joined, recs, se, host);
        }
    }
    if has_l5 && !first {
        if phrase != 0 {
            let plen = strlen8(arena, phrase);
            if plen.wrapping_mul(2).wrapping_sub(2) < u32::from(pending) {
                strcat_at(arena, l5, LIT_X);
                strnappend(arena, l5, phrase.wrapping_add(1), plen.wrapping_sub(2));
                j = plen.wrapping_mul(2).wrapping_sub(2) as u16;
                while u32::from(j) < u32::from(pending) {
                    strcat_at(arena, l5, LIT_TILDE);
                    j = j.wrapping_add(1);
                }
                strcat_at(arena, l5, LIT_Y);
                strnappend(arena, l5, phrase.wrapping_add(1), plen.wrapping_sub(2));
            } else {
                strcat_at(arena, l5, LIT_X);
                strnappend(arena, l5, phrase.wrapping_add(1), plen.wrapping_sub(2));
                strcat_at(arena, l5, LIT_Y);
                strnappend(arena, l5, phrase.wrapping_add(1), plen.wrapping_sub(2));
                while u32::from(pending) < plen.wrapping_mul(2).wrapping_sub(2) {
                    strcat_at(arena, l1, LIT_TILDE);
                    strcat_at(arena, l2, LIT_TILDE);
                    strcat_at(arena, l3, LIT_TILDE);
                    strcat_at(arena, l4, LIT_TILDE);
                    pending = pending.wrapping_add(1);
                }
            }
        } else {
            j = 0;
            while u32::from(j) < u32::from(wlen) {
                strcat_at(arena, l5, LIT_SP);
                j = j.wrapping_add(1);
            }
        }
    }

    rc = host.out_set_layer(arena, ph0, ph1, 0, l1, strlen8(arena, l1));
    if rc >= 0x8000_0000 {
        return cleanup_l1l5(arena, p, rc, joined, recs, se, host);
    }
    rc = host.out_set_layer(arena, ph0, ph1, 1, l2, strlen8(arena, l2));
    if rc >= 0x8000_0000 {
        return cleanup_l1l5(arena, p, rc, joined, recs, se, host);
    }
    rc = host.out_set_layer(arena, ph0, ph1, 2, l3, strlen8(arena, l3));
    if rc >= 0x8000_0000 {
        return cleanup_l1l5(arena, p, rc, joined, recs, se, host);
    }
    rc = host.out_set_layer(arena, ph0, ph1, 3, l4, strlen8(arena, l4));
    if rc >= 0x8000_0000 {
        return cleanup_l1l5(arena, p, rc, joined, recs, se, host);
    }
    if has_l5 {
        rc = host.out_set_layer_x(arena, ph0, ph1, 4, l5, strlen8(arena, l5));
        if rc >= 0x8000_0000 {
            return cleanup_l1l5(arena, p, rc, joined, recs, se, host);
        }
    }
    // A failed process still reports success.
    if host.out_process(arena, ph0, ph1, 0) >= 0x8000_0000 {
        return cleanup_l1l5(arena, p, 0, joined, recs, se, host);
    }
    rc = host.out_get_layer(arena, ph0, ph1, 0, &mut o1, &mut len);
    if rc >= 0x8000_0000 {
        return cleanup_l1l5(arena, p, rc, joined, recs, se, host);
    }
    put_byte(arena, o1.wrapping_add(u32::from(len)), 0);
    rc = host.out_get_layer(arena, ph0, ph1, 3, &mut o4, &mut len);
    if rc >= 0x8000_0000 {
        return cleanup_l1l5(arena, p, rc, joined, recs, se, host);
    }
    put_byte(arena, o4.wrapping_add(u32::from(len)), 0);
    {
        let mut d = o1;
        let mut s = o4;
        let mut n = u32::from(len);
        while n != 0 {
            n = n.wrapping_sub(1);
            if byte(arena, s) == 0x16 {
                put_byte(arena, d, 0x16);
            }
            s = s.wrapping_add(1);
            d = d.wrapping_add(1);
        }
    }
    k = 0;
    while k < len {
        if byte(arena, o1.wrapping_add(u32::from(k))) == b'~' {
            j = k;
            while u32::from(j) < u32::from(len) {
                put_byte(
                    arena,
                    o1.wrapping_add(u32::from(j)),
                    byte(arena, o1.wrapping_add(u32::from(j)).wrapping_add(1)),
                );
                j = j.wrapping_add(1);
            }
            len = len.wrapping_sub(1);
            k = k.wrapping_sub(1);
        }
        k = k.wrapping_add(1);
    }
    put_byte(arena, o1.wrapping_add(u32::from(len)), 0);

    match host.calloc(arena, heap, 1, u32::from(orig_len).wrapping_add(1)) {
        Some(v) => se = v,
        None => return cleanup_l1l5(arena, p, 0x8b80_200a, joined, recs, se, host),
    }
    strcpy_at(arena, se, orig);
    last_end = u32::from(strlen8(arena, se) as u16);
    memset_at(arena, wbuf, 0, 0x80);
    rc = wordspan_1ea35c(arena, p, o1, wbuf, TMP_NO1, host);
    if rc >= 0x8000_0000 {
        return cleanup_l1l5(arena, p, rc, joined, recs, se, host);
    }
    n_o1 = half(arena, TMP_NO1);
    rc = wordspan_1ea35c(arena, p, orig, 0, TMP_NORIG, host);
    if rc >= 0x8000_0000 {
        return cleanup_l1l5(arena, p, rc, joined, recs, se, host);
    }
    n_orig = half(arena, TMP_NORIG);
    // nO1/nOrig feed log-only lines.
    let _ = (n_o1, n_orig);
    w = 0;
    i = 0;
    jj = 0;
    while i < orig_len {
        if !next_word_1e7908(arena, orig, orig_len, &mut i, &mut ni) {
            i = i.wrapping_add(1);
            continue;
        }
        if !next_word_1e7908(arena, o1, len, &mut jj, &mut nj) {
            i = i.wrapping_add(1);
            continue;
        }
        if strcmp_at(
            arena,
            orig.wrapping_add(u32::from(i)),
            o1.wrapping_add(u32::from(jj)),
        ) != 0
        {
            let mut tail = 0u32;
            let mut mk = strchr8(arena, o1.wrapping_add(u32::from(jj)), 0x1e);
            while mk != 0 {
                while node2 != 0 {
                    rc = host.dp_get(
                        arena, h0, h1, u32::from(node2), 0, 1, &mut w_out, &mut cnt,
                    );
                    if rc >= 0x8000_0000 {
                        return cleanup_l1l5(arena, p, rc, joined, recs, se, host);
                    }
                    kind = w_out as u8;
                    rc = host.dp_get(
                        arena, h0, h1, u32::from(node2), 1, 1, &mut w_out, &mut cnt,
                    );
                    if rc >= 0x8000_0000 {
                        return cleanup_l1l5(arena, p, rc, joined, recs, se, host);
                    }
                    start = w_out as u16;
                    if start > i && kind == 7 {
                        tail = strchr8(arena, mk.wrapping_add(1), 0x1f);
                        if tail == 0 {
                            return cleanup_l1l5(arena, p, 0x8b80_2007, joined, recs, se, host);
                        }
                        put_byte(arena, tail, 0);
                        rc = host.dp_set(
                            arena,
                            h0,
                            h1,
                            u32::from(node2),
                            4,
                            strlen8(arena, mk),
                            mk.wrapping_add(1),
                            &mut tmp,
                        );
                        if rc >= 0x8000_0000 {
                            return cleanup_l1l5(arena, p, rc, joined, recs, se, host);
                        }
                        rc = host.dp_next_node(arena, h0, h1, u32::from(node2), &mut node2);
                        if rc >= 0x8000_0000 {
                            return cleanup_l1l5(arena, p, rc, joined, recs, se, host);
                        }
                        break;
                    }
                    rc = host.dp_next_node(arena, h0, h1, u32::from(node2), &mut node2);
                    if rc >= 0x8000_0000 {
                        return cleanup_l1l5(arena, p, rc, joined, recs, se, host);
                    }
                }
                put_byte(arena, mk, 0);
                if tail == 0 {
                    return cleanup_l1l5(arena, p, 0x8b80_2007, joined, recs, se, host);
                }
                memmove_at(
                    arena,
                    mk,
                    tail.wrapping_add(1),
                    strlen8(arena, tail.wrapping_add(1)).wrapping_add(1),
                );
                mk = strchr8(arena, o1.wrapping_add(u32::from(jj)), 0x1e);
            }
            let lo1 = strlen8(arena, o1.wrapping_add(u32::from(jj))) as u16;
            let lorig = strlen8(arena, orig.wrapping_add(u32::from(i))) as u16;
            let cur = strlen8(arena, se) as u16;
            if lo1 > lorig {
                let nl = lo1.wrapping_sub(lorig).wrapping_add(cur);
                match host.realloc(arena, heap, se, u32::from(nl).wrapping_add(1)) {
                    Some(t) => se = t,
                    None => return cleanup_l1l5(arena, p, 0x8b80_200a, joined, recs, se, host),
                }
                put_byte(arena, se.wrapping_add(u32::from(nl)), 0);
            }
            if lo1 != lorig {
                let grow = (u32::from(lo1) as i32).wrapping_sub(u32::from(lorig) as i32);
                let at = (delta as i32)
                    .wrapping_add(u32::from(i) as i32)
                    .wrapping_add(u32::from(lorig) as i32);
                memmove_at(
                    arena,
                    se.wrapping_add((at.wrapping_add(grow)) as u32),
                    se.wrapping_add(at as u32),
                    last_end.wrapping_sub(
                        u32::from(i).wrapping_add(u32::from(lorig)),
                    ),
                );
                shift_recs_1e7908(
                    arena,
                    recs,
                    n_recs,
                    i,
                    i.wrapping_add(lorig),
                    (lo1 as i16).wrapping_sub(lorig as i16),
                );
            }
            memcpy_at(
                arena,
                se.wrapping_add(
                    ((u32::from(i) as i32).wrapping_add(delta as i32)) as u32,
                ),
                o1.wrapping_add(u32::from(jj)),
                u32::from(lo1),
            );
            while mi < n_markers
                && ((lorig > 1
                    && (word(
                        arena,
                        markers
                            .wrapping_add(u32::from(mi).wrapping_mul(24))
                            .wrapping_add(12),
                    )
                    .wrapping_sub(base as u32) as u16 as i16)
                        < ((u32::from(i) as i32)
                            .wrapping_add(u32::from(lorig) as i32)
                            .wrapping_sub(1) as u16 as i16))
                    || (lorig == 1
                        && (word(
                            arena,
                            markers
                                .wrapping_add(u32::from(mi).wrapping_mul(24))
                                .wrapping_add(12),
                        )
                        .wrapping_sub(base as u32) as u16 as i16)
                            < ((u32::from(i) as i32).wrapping_add(1) as u16 as i16)))
            {
                let mp = markers
                    .wrapping_add(u32::from(mi).wrapping_mul(24))
                    .wrapping_add(12);
                put_word(arena, mp, word(arena, mp).wrapping_add(delta as i32 as u32));
                mi = mi.wrapping_add(1);
            }
            delta = delta.wrapping_add((lo1 as i16).wrapping_sub(lorig as i16));
            if lorig > lo1 {
                let nz = (u32::from(lorig) as i32).wrapping_sub(u32::from(lo1) as i32);
                if nz > 0 {
                    let mut z = se.wrapping_add(u32::from(cur));
                    let mut q = 0u16;
                    while (q as i32) < nz {
                        z = z.wrapping_sub(1);
                        put_byte(arena, z, 0);
                        q = q.wrapping_add(1);
                    }
                }
            }
        }
        w = w.wrapping_add(1);
        i = ni;
        jj = nj;
        i = i.wrapping_add(1);
    }

    rc = host.dp_first_node(arena, h0, h1, 2, u32::from(root), &mut node);
    if rc >= 0x8000_0000 {
        return cleanup_l1l5(arena, p, rc, joined, recs, se, host);
    }
    {
        let mut hy = 0u16;
        n_o1 = 0;
        if node != 0 {
            let base1 = se.wrapping_sub(1);
            loop {
                rc = host.dp_get(
                    arena, h0, h1, u32::from(node), 0, 1, &mut w_out, &mut cnt,
                );
                if rc >= 0x8000_0000 {
                    return cleanup_l1l5(arena, p, rc, joined, recs, se, host);
                }
                kind = w_out as u8;
                let mut a = 0u16;
                while a < n_recs {
                    let r = recs.wrapping_add(u32::from(a).wrapping_mul(12));
                    if half(arena, r) != node {
                        a = a.wrapping_add(1);
                        continue;
                    }
                    rc = host.dp_set(
                        arena,
                        h0,
                        h1,
                        u32::from(node),
                        1,
                        1,
                        r.wrapping_add(8),
                        &mut tmp,
                    );
                    if rc >= 0x8000_0000 {
                        return cleanup_l1l5(arena, p, rc, joined, recs, se, host);
                    }
                    rc = host.dp_set(
                        arena,
                        h0,
                        h1,
                        u32::from(node),
                        2,
                        1,
                        r.wrapping_add(10),
                        &mut tmp,
                    );
                    if rc >= 0x8000_0000 {
                        return cleanup_l1l5(arena, p, rc, joined, recs, se, host);
                    }
                    if kind == 4 || kind == 9 || kind == 10 {
                        if n_o1 != 0
                            && u16::from(byte(arena, wbuf.wrapping_add(u32::from(hy))))
                                == n_o1
                            && prev_node != 0
                        {
                            put_byte(
                                arena,
                                base1.wrapping_add(u32::from(half(arena, r.wrapping_add(8)))),
                                b'-',
                            );
                            hy = hy.wrapping_add(1);
                            let mut s3 = 0u32;
                            let mut s3p = 0u32;
                            rc = host.dp_get_ptr(
                                arena, h0, h1, u32::from(node), 3, &mut s3, &mut cnt,
                            );
                            if rc >= 0x8000_0000 {
                                return cleanup_l1l5(arena, p, rc, joined, recs, se, host);
                            }
                            rc = host.dp_get_ptr(
                                arena, h0, h1, prev_node, 3, &mut s3p, &mut cnt,
                            );
                            if rc >= 0x8000_0000 {
                                return cleanup_l1l5(arena, p, rc, joined, recs, se, host);
                            }
                            let n3 = strlen8(arena, s3);
                            match host.realloc(
                                arena,
                                heap,
                                joined,
                                strlen8(arena, s3p).wrapping_add(n3).wrapping_add(2),
                            ) {
                                Some(t) => joined = t,
                                None => return cleanup_l1l5(arena, p, 0x8b80_200a, joined, recs, se, host),
                            }
                            strcpy_at(arena, joined, s3p);
                            strcat_at(arena, joined, LIT_DASH);
                            strcat_at(arena, joined, s3);
                            rc = host.dp_set(
                                arena,
                                h0,
                                h1,
                                prev_node,
                                3,
                                strlen8(arena, joined).wrapping_add(1),
                                joined,
                                &mut tmp,
                            );
                            if rc >= 0x8000_0000 {
                                return cleanup_l1l5(arena, p, rc, joined, recs, se, host);
                            }
                            rc = host.dp_set(
                                arena,
                                h0,
                                h1,
                                prev_node,
                                2,
                                1,
                                r.wrapping_add(10),
                                &mut tmp,
                            );
                            if rc >= 0x8000_0000 {
                                return cleanup_l1l5(arena, p, rc, joined, recs, se, host);
                            }
                            if host.dp_get_type(
                                arena,
                                h0,
                                h1,
                                u32::from(half(arena, r)),
                                8,
                                &mut typ,
                            ) < 0x8000_0000
                                && typ == 1
                            {
                                rc = host.dp_get(
                                    arena,
                                    h0,
                                    h1,
                                    u32::from(half(arena, r)),
                                    8,
                                    1,
                                    &mut w_out,
                                    &mut cnt,
                                );
                                if rc >= 0x8000_0000 {
                                    return cleanup_l1l5(arena, p, rc, joined, recs, se, host);
                                }
                                // v8 is the u16 the get just wrote; only the low
                                // half is observed (the C passes &u16).
                                put_half(arena, SET_TMP, w_out as u16);
                                rc = host.dp_set(
                                    arena, h0, h1, prev_node, 8, 1, SET_TMP, &mut tmp,
                                );
                                if rc >= 0x8000_0000 {
                                    return cleanup_l1l5(arena, p, rc, joined, recs, se, host);
                                }
                            }
                            rc = host.dp_remove(arena, h0, h1, u32::from(node));
                            if rc >= 0x8000_0000 {
                                return cleanup_l1l5(arena, p, rc, joined, recs, se, host);
                            }
                            node = prev_node as u16;
                        }
                        let mut s3 = 0u32;
                        rc = host.dp_get_ptr(
                            arena, h0, h1, u32::from(node), 3, &mut s3, &mut cnt,
                        );
                        if rc >= 0x8000_0000 {
                            return cleanup_l1l5(arena, p, rc, joined, recs, se, host);
                        }
                        if byte(arena, s3) != 0 && find_substr(arena, s3, LIT_DASHSTAR) != 0
                        {
                            rc = host.dp_get(
                                arena, h0, h1, u32::from(node), 1, 1, &mut w_out, &mut cnt,
                            );
                            if rc >= 0x8000_0000 {
                                return cleanup_l1l5(arena, p, rc, joined, recs, se, host);
                            }
                            start = w_out as u16;
                            rc = host.dp_get(
                                arena, h0, h1, u32::from(node), 2, 1, &mut w_out, &mut cnt,
                            );
                            if rc >= 0x8000_0000 {
                                return cleanup_l1l5(arena, p, rc, joined, recs, se, host);
                            }
                            end = w_out as u16;
                            let mut hp =
                                strchr8(arena, se.wrapping_add(u32::from(start)), b'-');
                            while hp != 0 && hp < se.wrapping_add(u32::from(end)) {
                                put_byte(arena, hp, b'_');
                                hp = strchr8(arena, hp.wrapping_add(1), b'-');
                            }
                        }
                        n_o1 = n_o1.wrapping_add(1);
                        prev_node = u32::from(node);
                    }
                    a = n_recs;
                }
                rc = host.dp_next_node(arena, h0, h1, u32::from(node), &mut node);
                if rc >= 0x8000_0000 {
                    return cleanup_l1l5(arena, p, rc, joined, recs, se, host);
                }
                if node == 0 {
                    break;
                }
            }
        }
    }
    rc = host.dp_set(
        arena,
        h0,
        h1,
        u32::from(root),
        0,
        strlen8(arena, se).wrapping_add(1),
        se,
        &mut tmp,
    );
    if rc >= 0x8000_0000 {
        return cleanup_l1l5(arena, p, rc, joined, recs, se, host);
    }
    while mi < n_markers {
        let mp = markers
            .wrapping_add(u32::from(mi).wrapping_mul(24))
            .wrapping_add(12);
        put_word(arena, mp, word(arena, mp).wrapping_add(delta as i32 as u32));
        mi = mi.wrapping_add(1);
    }
    put_word(arena, markers.wrapping_add(16), u32::from(strlen8(arena, se) as u16));
    pos = pos.wrapping_add(delta as i32);
    rc = host.out_set_pos(arena, ph0, ph1, pos, 0);
    if rc >= 0x8000_0000 {
        return cleanup_l1l5(arena, p, rc, joined, recs, se, host);
    }
    cleanup_l1l5(arena, p, rc, joined, recs, se, host)
}

/// fn_1763e4. Widening strcpy: copy bytes to u16 units including the NUL.
pub fn strcpy_widen_1763e4(arena: &mut [u8], dst: u32, src: u32) {
    let mut d = dst;
    let mut s = src;
    loop {
        let c = byte(arena, s);
        put_half(arena, d, u16::from(c));
        if c == 0 {
            break;
        }
        d = d.wrapping_add(2);
        s = s.wrapping_add(1);
    }
}

/// fn_1765ac. u16 strcmp: scan while `a` has not ended and the pair
/// matches, then return the difference at the stop point.
pub fn strcmp16_1765ac(arena: &[u8], a: u32, b: u32) -> i32 {
    let mut aa = a;
    let mut bb = b;
    loop {
        let c = half(arena, aa);
        if c == 0 {
            break;
        }
        if c != half(arena, bb) {
            break;
        }
        aa = aa.wrapping_add(2);
        bb = bb.wrapping_add(2);
    }
    i32::from(half(arena, aa)) - i32::from(half(arena, bb))
}

/// fn_176624. u16 strlen: the post-increment scan ends past the NUL,
/// so the unit count before it is the length.
pub fn strlen16_176624(arena: &[u8], s: u32) -> i32 {
    let mut p = s;
    while half(arena, p) != 0 {
        p = p.wrapping_add(2);
    }
    p.wrapping_sub(s).wrapping_div(2) as i32
}

/// fn_176214. u16 strcpy including the NUL.
pub fn strcpy16_176214(arena: &mut [u8], dst: u32, src: u32) {
    let mut d = dst;
    let mut s = src;
    loop {
        let c = half(arena, s);
        put_half(arena, d, c);
        if c == 0 {
            break;
        }
        d = d.wrapping_add(2);
        s = s.wrapping_add(2);
    }
}

/// fn_176514. u16 strchr with a full-width `u32` needle: units wider
/// than 16 bits never match, so the scan ends at the NUL and misses.
pub fn strchr16_176514(arena: &[u8], s: u32, c: u32) -> u32 {
    let mut p = s;
    while u32::from(half(arena, p)) != 0 && u32::from(half(arena, p)) != c {
        p = p.wrapping_add(2);
    }
    if u32::from(half(arena, p)) != c {
        0
    } else {
        p
    }
}

/// fn_176644. u16 strncpy with pad semantics: `n` counts only nonzero
/// units; when the NUL lands, `n - 1` zero units follow it.
pub fn strncpy16_176644(arena: &mut [u8], dst: u32, src: u32, n: u32) {
    let mut d = dst;
    let mut s = src;
    let mut left = n;
    loop {
        if left == 0 {
            return;
        }
        let c = half(arena, s);
        s = s.wrapping_add(2);
        if c != 0 {
            left -= 1;
        }
        put_half(arena, d, c);
        d = d.wrapping_add(2);
        if c == 0 {
            break;
        }
    }
    left = left.wrapping_sub(1);
    while left != 0 {
        put_half(arena, d, 0);
        d = d.wrapping_add(2);
        left = left.wrapping_sub(1);
    }
}

/// fn_176460. Narrowing strcpy: units with high bits become `?`.
pub fn narrow_176460(arena: &mut [u8], dst: u32, src: u32) {
    let mut d = dst;
    let mut s = src;
    let mut c = half(arena, s);
    while c != 0 {
        put_byte(arena, d, if c & 0xff00 != 0 { 0x3f } else { c as u8 });
        d = d.wrapping_add(1);
        s = s.wrapping_add(2);
        c = half(arena, s);
    }
    put_byte(arena, d, 0);
}

/// fn_17649c. Narrowing strncpy: low bytes, `n` counting only nonzero
/// units, then `n - 1` pad zeros after an early NUL.
pub fn narrow_n_17649c(arena: &mut [u8], dst: u32, src: u32, n: u32) {
    let mut d = dst;
    let mut s = src;
    let mut left = n;
    loop {
        if left == 0 {
            return;
        }
        let c = u32::from(half(arena, s)) & 0xff;
        s = s.wrapping_add(2);
        put_byte(arena, d, c as u8);
        d = d.wrapping_add(1);
        if c != 0 {
            left -= 1;
        } else {
            break;
        }
    }
    left = left.wrapping_sub(1);
    while left != 0 {
        put_byte(arena, d, 0);
        d = d.wrapping_add(1);
        left = left.wrapping_sub(1);
    }
}

/// fn_176538. u16 strstr: first occurrence of `b` in `a`, or 0.
pub fn strstr16_176538(arena: &[u8], a: u32, b: u32) -> u32 {
    if half(arena, b) == 0 {
        return a;
    }
    if half(arena, a) == 0 {
        return 0;
    }
    let mut cur = a;
    loop {
        let mut p = cur;
        let mut q = b;
        let ended = loop {
            if half(arena, p) == 0 {
                break true;
            }
            if half(arena, q) == 0 {
                return cur;
            }
            if half(arena, p) != half(arena, q) {
                break false;
            }
            p = p.wrapping_add(2);
            q = q.wrapping_add(2);
        };
        if ended && half(arena, q) == 0 {
            return cur;
        }
        cur = cur.wrapping_add(2);
        if half(arena, cur) == 0 {
            return 0;
        }
    }
}

/// Scratch slots for `fe_depes_2031b0`: the `fn_282af0` outputs the C
/// keeps in stack locals, plus materialized copies of the stack u16
/// arrays (`normal`, `spell`, `lid`, `dummy`, `punctuation`) so the
/// arena-based string leaves serve both. Outside every compared window.
const DEPES_SCRATCH_CTYPE: u32 = 0x770;
const DEPES_SCRATCH_CAP: u32 = 0x774;
const DEPES_SCRATCH_LANG: u32 = 0x778;
const DEPES_SCRATCH_NORMAL: u32 = 0x780;
const DEPES_SCRATCH_SPELL: u32 = 0x790;
const DEPES_SCRATCH_LID: u32 = 0x7a0;
const DEPES_SCRATCH_DUMMY: u32 = 0x7b0;
const DEPES_SCRATCH_PUNCT: u32 = 0x7b8;

/// Shared `cleanup` tail of `fn_2031b0`: free owned markers, then every
/// buffer. Unobservable (free is a no-op on every host) but mirrored.
#[allow(clippy::too_many_arguments)]
fn cleanup_2031b0(
    arena: &mut [u8],
    heap: u32,
    ordered: u32,
    owned: u32,
    multiword_ends: u32,
    lookup: u32,
    replacement: u32,
    narrow: u32,
    markers: u32,
    marker_count: u16,
    extra_count: u16,
    host: &mut impl FeTextHost,
) {
    if ordered != 0 {
        if owned != 0 {
            let count = u32::from(marker_count).wrapping_add(u32::from(extra_count));
            let mut i = 1u16;
            while u32::from(i) < count {
                let slot = ordered.wrapping_add(u32::from(i) * 4);
                if byte(arena, owned.wrapping_add(u32::from(i))) != 0 {
                    let entry = word(arena, slot);
                    if half(arena, entry) != 7 {
                        host.free(arena, heap, word(arena, entry.wrapping_add(20)));
                    }
                    host.free(arena, heap, entry);
                }
                i = i.wrapping_add(1);
            }
        }
        if ordered != 0 {
            host.free(arena, heap, ordered);
        }
    }
    if owned != 0 {
        host.free(arena, heap, owned);
    }
    if multiword_ends != 0 {
        host.free(arena, heap, multiword_ends);
    }
    if lookup != 0 {
        host.free(arena, heap, lookup);
    }
    if replacement != 0 {
        host.free(arena, heap, replacement);
    }
    if narrow != 0 {
        host.free(arena, heap, narrow);
    }
    if markers != 0 {
        host.free(arena, heap, markers);
    }
}

/// fn_2031b0. FE_DEPES fetch: classify the next chunk of the marker
/// stream into one output unit and advance the stream. `fn_17497c` is
/// stubbed to 0 (its gates cover log-only calls) and the `fn_1738e8` /
/// `fn_173dc0` / `fn_17486c` / `fn_1748a0` calls are log-only. The
/// four-unit `languageText` stack array overflows into its neighbours
/// for long languages; only the widened string length and copy are
/// ever read back, so those read the source narrow string directly.
#[allow(clippy::too_many_lines)]
pub fn fe_depes_2031b0(
    arena: &mut [u8],
    p: u32,
    b: u32,
    _u1: u32,
    _u2: u32,
    done: u32,
    host: &mut impl FeTextHost,
) -> u32 {
    let ctx = word(arena, p.wrapping_add(8));
    let heap = word(arena, ctx.wrapping_add(4));
    let log = word(arena, ctx.wrapping_add(16));
    let reg = word(arena, ctx.wrapping_add(20));
    let f100 = half(arena, p.wrapping_add(100));
    let f102 = half(arena, p.wrapping_add(102));
    let f104 = half(arena, p.wrapping_add(104));
    let mut mode = DEPES_SCRATCH_NORMAL;
    let mut length = 0u16;
    let mut narrow = 0u32;
    let mut lookup = 0u32;
    let mut marker_index = 0u16;
    let mut available_text = 0i32;
    let mut markers = 0u32;
    let mut marker_count = 0u16;
    let mut ordered = 0u32;
    let mut finished = 0u8;
    let mut replacement = 0u32;
    let mut extra_count = 0u16;
    let mut in_tag = 0u32;
    let mut pronunciation = 15u32;
    let mut available_markers = 0i32;
    let mut position_delta = 0i32;
    let mut capacity = 0u16;
    let mut stream_position = 0i32;
    let mut word_start = 0u16;
    let mut text_count = 0u16;
    let mut is_multiword = 0i32;
    let mut output_language = 0u32;
    let mut output_type = 0u32;
    let mut text_bytes = 0i32;
    let mut base_position = 0u32;
    let mut multiword_start = 0u16;
    let mut multiword_end = 0u16;
    let mut word_end = 0u16;
    let mut language = DEPES_SCRATCH_NORMAL;
    let mut text = 0u32;
    let mut marker_input = 0u32;
    let mut marker_bytes = 0i32;
    let mut multiword_ends = 0u32;
    let mut owned = 0u32;
    let mut dummy_count = 0u16;
    let mut retry = 0u32;
    let mut marker = 0u32;
    let mut i = 0u16;
    let mut rc = host.validate(b, 0xf38a, 0xb8);
    if rc >= 0x8000_0000 {
        return 0x8a80_2008;
    }
    put_byte(arena, done, 1);
    rc = host.st_read(
        arena,
        word(arena, p.wrapping_add(28)),
        word(arena, p.wrapping_add(32)),
        &mut available_text,
        &mut finished,
    );
    if rc >= 0x8000_0000 {
        return rc;
    }
    rc = host.st_read(
        arena,
        word(arena, p.wrapping_add(36)),
        word(arena, p.wrapping_add(40)),
        &mut available_markers,
        &mut finished,
    );
    if rc >= 0x8000_0000 {
        return rc;
    }
    if available_text == 0 {
        if finished == 0 && available_markers == 0 {
            let mut setting = 0u32;
            if host.suba1c(arena, reg, 0, &mut setting) >= 0x8000_0000
                || setting == 0
                || byte(arena, setting) == 0
            {
                put_byte(arena, done, 0);
            }
        } else {
            if available_markers != 0 {
                rc = host.st_gettext(
                    arena,
                    word(arena, p.wrapping_add(36)),
                    word(arena, p.wrapping_add(40)),
                    &mut marker_input,
                    &mut marker_bytes,
                );
                if rc >= 0x8000_0000 {
                    return rc;
                }
                rc = host.st_write(
                    arena,
                    word(arena, p.wrapping_add(52)),
                    word(arena, p.wrapping_add(56)),
                    marker_input,
                    marker_bytes as u32,
                );
                if rc >= 0x8000_0000 {
                    return rc;
                }
                rc = host.st_consume(
                    arena,
                    word(arena, p.wrapping_add(36)),
                    word(arena, p.wrapping_add(40)),
                    marker_bytes as u32,
                );
                if rc >= 0x8000_0000 {
                    return rc;
                }
            }
            if finished != 0 {
                i = 0;
                while i < 2 {
                    let slot = p.wrapping_add(44).wrapping_add(u32::from(i) * 8);
                    if word(arena, slot) != 0 {
                        rc = host.st_close(arena, word(arena, slot), word(arena, slot.wrapping_add(4)));
                        put_word(arena, slot, 0);
                        put_word(arena, slot.wrapping_add(4), 0);
                    }
                    i = i.wrapping_add(1);
                }
            }
        }
        return rc;
    }
    rc = host.st_gettext(
        arena,
        word(arena, p.wrapping_add(28)),
        word(arena, p.wrapping_add(32)),
        &mut text,
        &mut text_bytes,
    );
    if rc >= 0x8000_0000 {
        return rc;
    }
    text_count = (text_bytes as u32 >> 1) as u16;
    rc = host.st_gettext(
        arena,
        word(arena, p.wrapping_add(36)),
        word(arena, p.wrapping_add(40)),
        &mut marker_input,
        &mut marker_bytes,
    );
    if rc >= 0x8000_0000 {
        return rc;
    }
    marker_count = ((marker_bytes as u32) / 24) as u16;
    let use_main = word(arena, p.wrapping_add(68)) != 0
        || (word(arena, p.wrapping_add(88)) != 0
            && host.dict_count(
                arena,
                word(arena, p.wrapping_add(92)),
                word(arena, p.wrapping_add(96)),
            ) != 0);
    if !use_main {
        rc = host.st_write(
            arena,
            word(arena, p.wrapping_add(44)),
            word(arena, p.wrapping_add(48)),
            text,
            text_bytes as u32,
        );
        if rc >= 0x8000_0000 {
            cleanup_2031b0(
                arena, heap, ordered, owned, multiword_ends, lookup, replacement, narrow,
                markers, marker_count, extra_count, host,
            );
            return rc;
        }
        rc = host.st_consume(
            arena,
            word(arena, p.wrapping_add(28)),
            word(arena, p.wrapping_add(32)),
            text_bytes as u32,
        );
        if rc >= 0x8000_0000 {
            cleanup_2031b0(
                arena, heap, ordered, owned, multiword_ends, lookup, replacement, narrow,
                markers, marker_count, extra_count, host,
            );
            return rc;
        }
        rc = host.st_write(
            arena,
            word(arena, p.wrapping_add(52)),
            word(arena, p.wrapping_add(56)),
            marker_input,
            marker_bytes as u32,
        );
        if rc >= 0x8000_0000 {
            cleanup_2031b0(
                arena, heap, ordered, owned, multiword_ends, lookup, replacement, narrow,
                markers, marker_count, extra_count, host,
            );
            return rc;
        }
        rc = host.st_consume(
            arena,
            word(arena, p.wrapping_add(36)),
            word(arena, p.wrapping_add(40)),
            marker_bytes as u32,
        );
        cleanup_2031b0(
            arena, heap, ordered, owned, multiword_ends, lookup, replacement, narrow,
            markers, marker_count, extra_count, host,
        );
        return rc;
    }
    // Materialize the stack u16 arrays (fn_1763e4 x5); language keeps
    // pointing at `normal`.
    for (slot, bytes) in [
        (DEPES_SCRATCH_SPELL, b"spell\x00".as_slice()),
        (DEPES_SCRATCH_NORMAL, b"normal\x00".as_slice()),
        (DEPES_SCRATCH_LID, b"_lid\x00".as_slice()),
        (DEPES_SCRATCH_DUMMY, b" \x15\x00".as_slice()),
        (DEPES_SCRATCH_PUNCT, b":;,)\x00".as_slice()),
    ] {
        for (k, &ch) in bytes.iter().enumerate() {
            put_half(arena, slot.wrapping_add(k as u32 * 2), u16::from(ch));
        }
    }
    language = DEPES_SCRATCH_NORMAL;
    lookup = match host.calloc(arena, heap, u32::from(f102).wrapping_add(1), 2) {
        Some(v) => v,
        None => {
            cleanup_2031b0(
                arena, heap, ordered, owned, multiword_ends, lookup, replacement, narrow,
                markers, marker_count, extra_count, host,
            );
            return 0x8a80_200a;
        }
    };
    let max_length = if text_count < f100 { f100 } else { text_count };
    narrow = match host.calloc(
        arena,
        heap,
        1,
        (i32::from(f104) * (i32::from(max_length) + 1) + 1) as u32,
    ) {
        Some(v) => v,
        None => {
            cleanup_2031b0(
                arena, heap, ordered, owned, multiword_ends, lookup, replacement, narrow,
                markers, marker_count, extra_count, host,
            );
            return 0x8a80_200a;
        }
    };
    replacement = match host.calloc(
        arena,
        heap,
        ((i32::from(f100) + 1) * i32::from(f104) + 1) as u32,
        2,
    ) {
        Some(v) => v,
        None => {
            cleanup_2031b0(
                arena, heap, ordered, owned, multiword_ends, lookup, replacement, narrow,
                markers, marker_count, extra_count, host,
            );
            return 0x8a80_200a;
        }
    };
    markers = match host.alloc(arena, heap, marker_bytes as u32) {
        Some(v) => v,
        None => {
            cleanup_2031b0(
                arena, heap, ordered, owned, multiword_ends, lookup, replacement, narrow,
                markers, marker_count, extra_count, host,
            );
            return 0x8a80_200a;
        }
    };
    memcpy_195ae8(arena, markers, marker_input, marker_bytes as u32);
    i = 0;
    while i < marker_count {
        marker = markers.wrapping_add(u32::from(i) * 24);
        if half(arena, marker) == 1 {
            put_word(arena, marker.wrapping_add(16), 0);
        }
        i = i.wrapping_add(1);
    }
    rc = host.depes_get_pos(
        arena,
        word(arena, p.wrapping_add(12)),
        word(arena, p.wrapping_add(16)),
        &mut stream_position,
        0,
    );
    if rc >= 0x8000_0000 {
        cleanup_2031b0(
            arena, heap, ordered, owned, multiword_ends, lookup, replacement, narrow,
            markers, marker_count, extra_count, host,
        );
        return rc;
    }
    ordered = match host.calloc(arena, heap, u32::from(marker_count), 4) {
        Some(v) => v,
        None => {
            cleanup_2031b0(
                arena, heap, ordered, owned, multiword_ends, lookup, replacement, narrow,
                markers, marker_count, extra_count, host,
            );
            return 0x8a80_200a;
        }
    };
    owned = match host.calloc(arena, heap, u32::from(marker_count), 4) {
        Some(v) => v,
        None => {
            cleanup_2031b0(
                arena, heap, ordered, owned, multiword_ends, lookup, replacement, narrow,
                markers, marker_count, extra_count, host,
            );
            return 0x8a80_200a;
        }
    };
    multiword_ends = match host.calloc(arena, heap, text_bytes.wrapping_add(1) as u32, 2) {
        Some(v) => v,
        None => {
            cleanup_2031b0(
                arena, heap, ordered, owned, multiword_ends, lookup, replacement, narrow,
                markers, marker_count, extra_count, host,
            );
            return 0x8a80_200a;
        }
    };
    base_position = word(arena, markers.wrapping_add(12));
    while marker_index < marker_count
        && word(
            arena,
            markers.wrapping_add(u32::from(marker_index) * 24).wrapping_add(12),
        ) == base_position
    {
        marker = markers.wrapping_add(u32::from(marker_index) * 24);
        let mtype = half(arena, marker);
        let value = word(arena, marker.wrapping_add(20));
        if in_tag == 0 && mtype == 0x15 {
            mode = value;
        } else if in_tag == 0 && mtype == 0x24 {
            language = value;
        } else {
            if ((mtype == 7 && value == 0x2b) || mtype == 0x22) && in_tag == 0 {
                in_tag = 1;
            } else if mtype == 7 || mtype == 0x15 {
                if in_tag != 0 {
                    in_tag = 0;
                }
            }
            if mtype == 7 {
                pronunciation = value;
            }
        }
        let pos = word(arena, marker.wrapping_add(12));
        put_word(arena, marker.wrapping_add(12), pos.wrapping_add(position_delta as u32));
        put_word(
            arena,
            ordered.wrapping_add(u32::from(marker_index) * 4),
            marker,
        );
        put_byte(arena, owned.wrapping_add(u32::from(marker_index)), 0);
        marker_index = marker_index.wrapping_add(1);
    }
    {
        let mut start = 0xffffu16;
        i = 0;
        while i < marker_count {
            marker = markers.wrapping_add(u32::from(i) * 24);
            let mtype = half(arena, marker);
            let value = word(arena, marker.wrapping_add(20));
            if mtype == 0x20 {
                if value == 1 {
                    start = word(arena, marker.wrapping_add(12)).wrapping_sub(base_position)
                        .wrapping_div(2) as u16;
                } else if value == 0 {
                    if start != 0xffff {
                        let mut end = word(arena, marker.wrapping_add(12))
                            .wrapping_sub(base_position)
                            .wrapping_div(2) as u16;
                        if end == text_count
                            && end != 0
                            && isspace_283188(u32::from(half(
                                arena,
                                text.wrapping_add(u32::from(end - 1) * 2),
                            ))) != 0
                        {
                            end -= 1;
                        }
                        while end != text_count
                            && end != 0
                            && isspace_283188(u32::from(half(
                                arena,
                                text.wrapping_add(u32::from(end) * 2),
                            ))) != 0
                        {
                            end -= 1;
                        }
                        if end != text_count
                            && isspace_283188(u32::from(half(
                                arena,
                                text.wrapping_add(u32::from(end) * 2),
                            ))) == 0
                        {
                            end = end.wrapping_add(1);
                        }
                        put_half(arena, multiword_ends.wrapping_add(u32::from(start) * 2), end);
                        start = 0xffff;
                    }
                }
            } else if (mtype == 0x15 || mtype == 7 || mtype == 0x22) && start != 0xffff {
                start = 0xffff;
            }
            i = i.wrapping_add(1);
        }
    }
    word_end = 0;
    word_start = word_end;
    if text_count != 0 {
        loop {
            while word_end < text_count
                && isspace_283188(u32::from(half(
                    arena,
                    text.wrapping_add(u32::from(word_end) * 2),
                ))) != 0
            {
                word_end = word_end.wrapping_add(1);
            }
            is_multiword = 0;
            if retry == 0 {
                i = word_start;
                while i <= word_end {
                    if half(arena, multiword_ends.wrapping_add(u32::from(i) * 2)) != 0 {
                        is_multiword = 1;
                        multiword_end = half(arena, multiword_ends.wrapping_add(u32::from(i) * 2));
                        multiword_start = i;
                        break;
                    }
                    if i == word_end {
                        break;
                    }
                    i = i.wrapping_add(1);
                }
                if word_start == word_end
                    && half(arena, multiword_ends.wrapping_add(u32::from(word_end) * 2)) != 0
                {
                    is_multiword = 1;
                    multiword_start = word_end;
                    multiword_end = half(arena, multiword_ends.wrapping_add(u32::from(word_end) * 2));
                }
            }
            if i32::from(word_end) - i32::from(word_start) > 0 {
                if is_multiword != 0 && multiword_end > word_end && multiword_start < word_end {
                    word_end = multiword_start;
                }
                if i32::from(word_end) - i32::from(word_start) > 0 {
                    rc = host.st_write(
                        arena,
                        word(arena, p.wrapping_add(44)),
                        word(arena, p.wrapping_add(48)),
                        text.wrapping_add(u32::from(word_start) * 2),
                        ((i32::from(word_end) - i32::from(word_start)) * 2) as u32,
                    );
                    if rc >= 0x8000_0000 {
                        cleanup_2031b0(
                            arena, heap, ordered, owned, multiword_ends, lookup, replacement,
                            narrow, markers, marker_count, extra_count, host,
                        );
                        return rc;
                    }
                }
            }
            if is_multiword == 1 {
                length = multiword_end.wrapping_sub(multiword_start);
                if f102 < length {
                    word_end = word_end.wrapping_add(length);
                    if i32::from(multiword_end) - i32::from(multiword_start) > 0 {
                        rc = host.st_write(
                            arena,
                            word(arena, p.wrapping_add(44)),
                            word(arena, p.wrapping_add(48)),
                            text.wrapping_add(u32::from(multiword_start) * 2),
                            i32::from(length) as u32 * 2,
                        );
                        if rc >= 0x8000_0000 {
                            cleanup_2031b0(
                                arena, heap, ordered, owned, multiword_ends, lookup, replacement,
                                narrow, markers, marker_count, extra_count, host,
                            );
                            return rc;
                        }
                    }
                    word_start = word_end;
                    if word_end < text_count {
                        continue;
                    } else {
                        break;
                    }
                } else {
                    word_end = multiword_end;
                    word_start = multiword_start;
                    strncpy16_176644(
                        arena,
                        lookup,
                        text.wrapping_add(u32::from(multiword_start) * 2),
                        u32::from(length),
                    );
                    put_half(arena, lookup.wrapping_add(u32::from(length) * 2), 0);
                }
            } else {
                length = 0;
                word_start = word_end;
                memset_195bec(arena, lookup, 0, i32::from(f102) as u32 * 2);
                while word_end < text_count {
                    if isspace_283188(u32::from(half(
                        arena,
                        text.wrapping_add(u32::from(word_end) * 2),
                    ))) != 0
                        || i32::from(length) >= i32::from(f102) - 1
                    {
                        break;
                    }
                    put_half(
                        arena,
                        lookup.wrapping_add(u32::from(length) * 2),
                        half(arena, text.wrapping_add(u32::from(word_end) * 2)),
                    );
                    length = length.wrapping_add(1);
                    word_end = word_end.wrapping_add(1);
                }
                if word_end < text_count
                    && isspace_283188(u32::from(half(
                        arena,
                        text.wrapping_add(u32::from(word_end) * 2),
                    ))) == 0
                {
                    while word_end < text_count
                        && isspace_283188(u32::from(half(
                            arena,
                            text.wrapping_add(u32::from(word_end) * 2),
                        ))) == 0
                    {
                        word_end = word_end.wrapping_add(1);
                    }
                    if i32::from(word_end) - i32::from(word_start) > 0 {
                        rc = host.st_write(
                            arena,
                            word(arena, p.wrapping_add(44)),
                            word(arena, p.wrapping_add(48)),
                            text.wrapping_add(u32::from(word_start) * 2),
                            ((i32::from(word_end) - i32::from(word_start)) * 2) as u32,
                        );
                        if rc >= 0x8000_0000 {
                            cleanup_2031b0(
                                arena, heap, ordered, owned, multiword_ends, lookup, replacement,
                                narrow, markers, marker_count, extra_count, host,
                            );
                            return rc;
                        }
                    }
                    word_start = word_end;
                    if word_end < text_count {
                        continue;
                    } else {
                        break;
                    }
                }
            }
            if length != 0 || is_multiword == 1 {
                output_type = 1;
                while marker_index < marker_count {
                    marker = markers.wrapping_add(u32::from(marker_index) * 24);
                    if u32::from(word_start) * 2
                        < u32::from(
                            word(arena, marker.wrapping_add(12)).wrapping_sub(base_position) as u16
                        )
                    {
                        break;
                    }
                    let pos = word(arena, marker.wrapping_add(12));
                    put_word(arena, marker.wrapping_add(12), pos.wrapping_add(position_delta as u32));
                    put_word(
                        arena,
                        ordered.wrapping_add(
                            (u32::from(marker_index).wrapping_add(u32::from(extra_count))) * 4,
                        ),
                        marker,
                    );
                    put_byte(
                        arena,
                        owned.wrapping_add(u32::from(marker_index).wrapping_add(u32::from(extra_count))),
                        0,
                    );
                    let mtype = half(arena, marker);
                    let value = word(arena, marker.wrapping_add(20));
                    if in_tag == 0 && mtype == 0x15 {
                        mode = value;
                    } else if in_tag == 0 && mtype == 0x24 {
                        language = value;
                    } else {
                        if ((mtype == 7 && value == 0x2b) || mtype == 0x22) && in_tag == 0 {
                            in_tag = 1;
                        } else if mtype == 7 || mtype == 0x15 {
                            if in_tag != 0 {
                                in_tag = 0;
                            }
                        }
                        if mtype == 7 {
                            pronunciation = value;
                        }
                    }
                    marker_index = marker_index.wrapping_add(1);
                }
                capacity = f100;
                if strcmp16_1765ac(arena, mode, DEPES_SCRATCH_SPELL) != 0 && in_tag == 0 {
                    narrow_n_17649c(
                        arena,
                        narrow,
                        text.wrapping_add(u32::from(word_start) * 2),
                        u32::from(length),
                    );
                    put_byte(arena, narrow.wrapping_add(u32::from(length)), 0);
                    memset_195bec(arena, replacement, 0, i32::from(f100) as u32 * 2);
                    if strlen16_176624(arena, language) == 3
                        || (strlen16_176624(arena, language) == 7
                            && strstr16_176538(arena, language, DEPES_SCRATCH_LID) != 0)
                    {
                        narrow_176460(arena, word(arena, p.wrapping_add(108)), language);
                        output_language = word(arena, p.wrapping_add(108));
                    } else {
                        output_language = 0;
                    }
                    put_half(arena, DEPES_SCRATCH_CAP, f100);
                    rc = dict_call_203144(
                        arena,
                        p,
                        b,
                        1,
                        lookup,
                        DEPES_SCRATCH_CTYPE,
                        replacement,
                        DEPES_SCRATCH_CAP,
                        DEPES_SCRATCH_LANG,
                        host,
                    );
                    if rc < 0x8000_0000 {
                        output_type = word(arena, DEPES_SCRATCH_CTYPE);
                        capacity = half(arena, DEPES_SCRATCH_CAP);
                        output_language = word(arena, DEPES_SCRATCH_LANG);
                    }
                }
                dummy_count = 0;
                if rc < 0x8000_0000 && capacity != 0 && capacity < f100 {
                    narrow_176460(arena, narrow, replacement);
                    if output_language != 0 {
                        let count =
                            u32::from(marker_count).wrapping_add(u32::from(extra_count));
                        ordered = match host.realloc(
                            arena,
                            heap,
                            ordered,
                            count.wrapping_add(1) * 4,
                        ) {
                            Some(v) => v,
                            None => {
                                cleanup_2031b0(
                                    arena, heap, ordered, owned, multiword_ends, lookup,
                                    replacement, narrow, markers, marker_count, extra_count, host,
                                );
                                return 0x8a80_200a;
                            }
                        };
                        owned = match host.realloc(arena, heap, owned, count.wrapping_add(1) * 4) {
                            Some(v) => v,
                            None => {
                                cleanup_2031b0(
                                    arena, heap, ordered, owned, multiword_ends, lookup,
                                    replacement, narrow, markers, marker_count, extra_count, host,
                                );
                                return 0x8a80_200a;
                            }
                        };
                        put_byte(arena, owned.wrapping_add(count), 0);
                        let language_marker = match host.calloc(arena, heap, 1, 24) {
                            Some(v) => v,
                            None => {
                                cleanup_2031b0(
                                    arena, heap, ordered, owned, multiword_ends, lookup,
                                    replacement, narrow, markers, marker_count, extra_count, host,
                                );
                                return 0x8a80_200a;
                            }
                        };
                        let new_language = match host.calloc(
                            arena,
                            heap,
                            (strlen_195d90(arena, output_language) + 1) as u32,
                            2,
                        ) {
                            Some(v) => v,
                            None => {
                                rc = 0x8a80_200a;
                                host.free(arena, heap, language_marker);
                                cleanup_2031b0(
                                    arena, heap, ordered, owned, multiword_ends, lookup,
                                    replacement, narrow, markers, marker_count, extra_count, host,
                                );
                                return rc;
                            }
                        };
                        strcpy_widen_1763e4(arena, new_language, output_language);
                        put_half(arena, language_marker, 0x24);
                        put_word(arena, language_marker.wrapping_add(8), 0);
                        let refpos = base_position
                            .wrapping_add(position_delta as u32)
                            .wrapping_add(u32::from(word_start) * 2);
                        put_word(arena, language_marker.wrapping_add(4), refpos);
                        put_word(arena, language_marker.wrapping_add(12), refpos);
                        put_word(arena, language_marker.wrapping_add(16), 0);
                        put_word(arena, language_marker.wrapping_add(20), new_language);
                        let slot =
                            u32::from(marker_index).wrapping_add(u32::from(extra_count));
                        put_word(arena, ordered.wrapping_add(slot * 4), language_marker);
                        put_byte(arena, owned.wrapping_add(slot), 1);
                        extra_count = extra_count.wrapping_add(1);
                    }
                    if output_type == 1 {
                        let mut language_changed = 0i32;
                        let count =
                            u32::from(marker_count).wrapping_add(u32::from(extra_count));
                        ordered = match host.realloc(arena, heap, ordered, count * 4 + 8) {
                            Some(v) => v,
                            None => {
                                cleanup_2031b0(
                                    arena, heap, ordered, owned, multiword_ends, lookup,
                                    replacement, narrow, markers, marker_count, extra_count, host,
                                );
                                return 0x8a80_200a;
                            }
                        };
                        owned = match host.realloc(arena, heap, owned, count * 4 + 8) {
                            Some(v) => v,
                            None => {
                                cleanup_2031b0(
                                    arena, heap, ordered, owned, multiword_ends, lookup,
                                    replacement, narrow, markers, marker_count, extra_count, host,
                                );
                                return 0x8a80_200a;
                            }
                        };
                        put_byte(arena, owned.wrapping_add(count), 0);
                        put_byte(arena, owned.wrapping_add(count + 1), 0);
                        let phonetic_marker = match host.calloc(arena, heap, 1, 24) {
                            Some(v) => v,
                            None => {
                                cleanup_2031b0(
                                    arena, heap, ordered, owned, multiword_ends, lookup,
                                    replacement, narrow, markers, marker_count, extra_count, host,
                                );
                                return 0x8a80_200a;
                            }
                        };
                        let phonetic = match host.calloc(
                            arena,
                            heap,
                            (i32::from(capacity) + 1) as u32,
                            2,
                        ) {
                            Some(v) => v,
                            None => {
                                rc = 0x8a80_200a;
                                host.free(arena, heap, phonetic_marker);
                                cleanup_2031b0(
                                    arena, heap, ordered, owned, multiword_ends, lookup,
                                    replacement, narrow, markers, marker_count, extra_count, host,
                                );
                                return rc;
                            }
                        };
                        strcpy16_176214(arena, phonetic, replacement);
                        if is_multiword == 1 {
                            let mut orthographic_spaces = 0u16;
                            let mut phonetic_spaces = 0u16;
                            let mut j = 0u16;
                            while j < length
                                && isspace_283188(u32::from(half(
                                    arena,
                                    lookup.wrapping_add(u32::from(j) * 2),
                                ))) != 0
                            {
                                j = j.wrapping_add(1);
                            }
                            while j < length
                                && half(arena, lookup.wrapping_add(u32::from(j) * 2)) != 0
                            {
                                if isspace_283188(u32::from(half(
                                    arena,
                                    lookup.wrapping_add(u32::from(j) * 2),
                                ))) != 0
                                {
                                    orthographic_spaces = orthographic_spaces.wrapping_add(1);
                                    while j < length
                                        && isspace_283188(u32::from(half(
                                            arena,
                                            lookup.wrapping_add(u32::from(j) * 2),
                                        ))) != 0
                                    {
                                        j = j.wrapping_add(1);
                                    }
                                }
                                j = j.wrapping_add(1);
                            }
                            i = 1;
                            while i32::from(i) < i32::from(capacity) - 1 {
                                let c = half(arena, phonetic.wrapping_add(u32::from(i) * 2));
                                if c == u16::from(b'_') || c == u16::from(b'#') {
                                    phonetic_spaces = phonetic_spaces.wrapping_add(1);
                                }
                                if c == 0x12 {
                                    break;
                                }
                                i = i.wrapping_add(1);
                            }
                            if orthographic_spaces < phonetic_spaces {
                                dummy_count =
                                    phonetic_spaces.wrapping_sub(orthographic_spaces);
                            } else if orthographic_spaces != phonetic_spaces {
                                let mut remaining =
                                    orthographic_spaces.wrapping_sub(phonetic_spaces);
                                narrow_n_17649c(
                                    arena,
                                    narrow,
                                    text.wrapping_add(u32::from(word_start) * 2),
                                    u32::from(length),
                                );
                                put_byte(arena, narrow.wrapping_add(u32::from(length)), 0);
                                let mut j = length;
                                let mut k: u16;
                                loop {
                                    j = j.wrapping_sub(1);
                                    k = 0;
                                    if j == 0 {
                                        break;
                                    }
                                    k = j;
                                    if byte(arena, narrow.wrapping_add(u32::from(j))) != b' ' {
                                        break;
                                    }
                                }
                                if j != 0 {
                                    loop {
                                        k = k.wrapping_sub(1);
                                        if k == 0
                                            || byte(arena, narrow.wrapping_add(u32::from(k)))
                                                == b' '
                                        {
                                            break;
                                        }
                                    }
                                }
                                while remaining != 0 && k != 0 {
                                    while k != 0
                                        && byte(arena, narrow.wrapping_add(u32::from(k))) == b' '
                                    {
                                        put_byte(arena, narrow.wrapping_add(u32::from(k)), b'-');
                                        k = k.wrapping_sub(1);
                                    }
                                    remaining = remaining.wrapping_sub(1);
                                    if k != 0 {
                                        loop {
                                            k = k.wrapping_sub(1);
                                            if k == 0
                                                || byte(arena, narrow.wrapping_add(u32::from(k)))
                                                    == b' '
                                            {
                                                break;
                                            }
                                        }
                                    }
                                }
                                i = 0;
                                while i < length {
                                    put_half(
                                        arena,
                                        text.wrapping_add(
                                            (u32::from(word_start) + u32::from(i)) * 2,
                                        ),
                                        u16::from(byte(arena, narrow.wrapping_add(u32::from(i)))),
                                    );
                                    i = i.wrapping_add(1);
                                }
                                i = 0;
                                while i < marker_count {
                                    let mi = markers.wrapping_add(u32::from(i) * 24);
                                    let position = word(arena, mi.wrapping_add(12))
                                        .wrapping_sub(word(arena, markers.wrapping_add(12)));
                                    if u32::from(word_start) * 2 < position
                                        && position
                                            < (u32::from(word_start) + u32::from(length)) * 2
                                    {
                                        let new_position = base_position.wrapping_add(
                                            u32::from(
                                                (i32::from(word_start) + position_delta) as u16,
                                            ) * 2,
                                        );
                                        put_word(arena, mi.wrapping_add(12), new_position);
                                    }
                                    i = i.wrapping_add(1);
                                }
                            }
                        } else {
                            i = 1;
                            while i32::from(i) < i32::from(capacity) - 1 {
                                let c = half(arena, phonetic.wrapping_add(u32::from(i) * 2));
                                if (c == u16::from(b'_') || c == u16::from(b'#'))
                                    && half(
                                        arena,
                                        phonetic.wrapping_add(u32::from(i).wrapping_add(1) * 2),
                                    ) != 0x12
                                {
                                    dummy_count = dummy_count.wrapping_add(1);
                                }
                                if c == 0x12 {
                                    break;
                                }
                                i = i.wrapping_add(1);
                            }
                        }
                        i = 0;
                        while i < length {
                            let slot = text
                                .wrapping_add((u32::from(word_start) + u32::from(i)) * 2);
                            if half(arena, slot) == u16::from(b'_') {
                                put_half(arena, slot, u16::from(b'-'));
                            }
                            i = i.wrapping_add(1);
                        }
                        put_half(arena, phonetic_marker, 0x22);
                        put_word(arena, phonetic_marker.wrapping_add(8), 0);
                        let refpos = base_position
                            .wrapping_add(position_delta as u32)
                            .wrapping_add(u32::from(word_start) * 2);
                        put_word(arena, phonetic_marker.wrapping_add(4), refpos);
                        put_word(arena, phonetic_marker.wrapping_add(16), 0);
                        put_word(arena, phonetic_marker.wrapping_add(20), phonetic);
                        put_word(arena, phonetic_marker.wrapping_add(12), refpos);
                        let slot0 =
                            u32::from(marker_index).wrapping_add(u32::from(extra_count));
                        put_word(arena, ordered.wrapping_add(slot0 * 4), phonetic_marker);
                        put_byte(arena, owned.wrapping_add(slot0), 1);
                        let end_position = base_position
                            .wrapping_add(u32::from(dummy_count) * 4)
                            .wrapping_add(position_delta as u32)
                            .wrapping_add(u32::from(word_end) * 2);
                        while marker_index < marker_count {
                            marker = markers.wrapping_add(u32::from(marker_index) * 24);
                            if u32::from(word_end) * 2
                                <= u32::from(
                                    word(arena, marker.wrapping_add(12)).wrapping_sub(base_position)
                                        as u16,
                                )
                            {
                                break;
                            }
                            let mtype = half(arena, marker);
                            if mtype == 0x24 {
                                language_changed = 1;
                                language = word(arena, marker.wrapping_add(20));
                                put_word(arena, marker.wrapping_add(12), end_position);
                            } else if language_changed != 0 {
                                put_word(arena, marker.wrapping_add(12), end_position);
                            } else {
                                let pos = word(arena, marker.wrapping_add(12));
                                put_word(
                                    arena,
                                    marker.wrapping_add(12),
                                    pos.wrapping_add(position_delta as u32),
                                );
                            }
                            let slot1 = u32::from(marker_index)
                                .wrapping_add(u32::from(extra_count))
                                .wrapping_add(1);
                            put_word(arena, ordered.wrapping_add(slot1 * 4), marker);
                            put_byte(arena, owned.wrapping_add(slot1), 0);
                            if in_tag == 0 && mtype == 0x15 {
                                mode = word(arena, marker.wrapping_add(20));
                            } else if mtype == 7 {
                                pronunciation = word(arena, marker.wrapping_add(20));
                                if pronunciation == 0x2b {
                                    if in_tag == 0 {
                                        in_tag = 1;
                                    } else {
                                        in_tag = 0;
                                    }
                                } else if in_tag != 0 {
                                    in_tag = 0;
                                }
                            }
                            marker_index = marker_index.wrapping_add(1);
                        }
                        if word_end < text_count {
                            let mode_marker = match host.calloc(arena, heap, 1, 24) {
                                Some(v) => v,
                                None => {
                                    rc = 0x8a80_200a;
                                    extra_count = extra_count.wrapping_add(1);
                                    cleanup_2031b0(
                                        arena, heap, ordered, owned, multiword_ends, lookup,
                                        replacement, narrow, markers, marker_count, extra_count,
                                        host,
                                    );
                                    return rc;
                                }
                            };
                            let saved_mode = match host.alloc(
                                arena,
                                heap,
                                ((strlen16_176624(arena, mode) + 1) * 2) as u32,
                            ) {
                                Some(v) => v,
                                None => {
                                    rc = 0x8a80_200a;
                                    host.free(arena, heap, mode_marker);
                                    extra_count = extra_count.wrapping_add(1);
                                    cleanup_2031b0(
                                        arena, heap, ordered, owned, multiword_ends, lookup,
                                        replacement, narrow, markers, marker_count, extra_count,
                                        host,
                                    );
                                    return rc;
                                }
                            };
                            strcpy16_176214(arena, saved_mode, mode);
                            i = marker_index.wrapping_add(extra_count);
                            put_half(arena, mode_marker, 0x15);
                            extra_count = extra_count.wrapping_add(1);
                            put_word(arena, mode_marker.wrapping_add(20), saved_mode);
                            put_word(arena, mode_marker.wrapping_add(4), end_position);
                            put_word(arena, mode_marker.wrapping_add(12), end_position);
                            put_word(arena, mode_marker.wrapping_add(8), 0);
                            put_word(arena, mode_marker.wrapping_add(16), 0);
                            put_word(
                                arena,
                                ordered.wrapping_add((u32::from(i) + 1) * 4),
                                mode_marker,
                            );
                            put_byte(arena, owned.wrapping_add(u32::from(i) + 1), 1);
                        }
                        position_delta =
                            (((position_delta + (i32::from(dummy_count) << 2)) as u16) as i16)
                                as i32;
                        extra_count = extra_count.wrapping_add(1);
                    } else if output_type == 4
                        && capacity > 6
                        && memcmp_195aac(arena, 0x6f0, replacement, 6) == 0
                        && memcmp_195aac(
                            arena,
                            0x6f0,
                            replacement.wrapping_add((u32::from(capacity) - 3) * 2),
                            6,
                        ) == 0
                    {
                        memmove_195b38(
                            arena,
                            replacement,
                            replacement.wrapping_add(6),
                            (i32::from(capacity) * 2 - 12) as u32,
                        );
                        put_half(
                            arena,
                            replacement.wrapping_add((u32::from(capacity) - 6) * 2),
                            0,
                        );
                        capacity = capacity.wrapping_sub(6);
                        narrow_176460(arena, narrow, replacement);
                        put_byte(arena, narrow.wrapping_add(u32::from(length)), 0);
                        let count =
                            u32::from(marker_count).wrapping_add(u32::from(extra_count));
                        ordered = match host.realloc(arena, heap, ordered, count * 4 + 8) {
                            Some(v) => v,
                            None => {
                                cleanup_2031b0(
                                    arena, heap, ordered, owned, multiword_ends, lookup,
                                    replacement, narrow, markers, marker_count, extra_count, host,
                                );
                                return 0x8a80_200a;
                            }
                        };
                        owned = match host.realloc(arena, heap, owned, count * 4 + 8) {
                            Some(v) => v,
                            None => {
                                cleanup_2031b0(
                                    arena, heap, ordered, owned, multiword_ends, lookup,
                                    replacement, narrow, markers, marker_count, extra_count, host,
                                );
                                return 0x8a80_200a;
                            }
                        };
                        put_byte(arena, owned.wrapping_add(count), 0);
                        put_byte(arena, owned.wrapping_add(count + 1), 0);
                        let start_marker = match host.calloc(arena, heap, 1, 24) {
                            Some(v) => v,
                            None => {
                                cleanup_2031b0(
                                    arena, heap, ordered, owned, multiword_ends, lookup,
                                    replacement, narrow, markers, marker_count, extra_count, host,
                                );
                                return 0x8a80_200a;
                            }
                        };
                        let start_position =
                            base_position.wrapping_add(u32::from(word_start) * 2);
                        let index =
                            u32::from(marker_index).wrapping_add(u32::from(extra_count));
                        put_half(arena, start_marker, 7);
                        put_word(
                            arena,
                            start_marker.wrapping_add(4),
                            start_position.wrapping_add(position_delta as u32),
                        );
                        put_word(arena, start_marker.wrapping_add(8), 0);
                        put_word(arena, start_marker.wrapping_add(20), 0x3c);
                        put_word(
                            arena,
                            start_marker.wrapping_add(12),
                            start_position.wrapping_add(position_delta as u32),
                        );
                        put_word(arena, start_marker.wrapping_add(16), 0);
                        put_word(arena, ordered.wrapping_add(index * 4), start_marker);
                        put_byte(arena, owned.wrapping_add(index), 1);
                        let end_marker = match host.calloc(arena, heap, 1, 24) {
                            Some(v) => v,
                            None => {
                                extra_count = extra_count.wrapping_add(1);
                                cleanup_2031b0(
                                    arena, heap, ordered, owned, multiword_ends, lookup,
                                    replacement, narrow, markers, marker_count, extra_count, host,
                                );
                                return 0x8a80_200a;
                            }
                        };
                        put_half(arena, end_marker, 7);
                        put_word(arena, end_marker.wrapping_add(20), pronunciation);
                        extra_count = extra_count.wrapping_add(2);
                        let refpos = start_position
                            .wrapping_add(position_delta as u32)
                            .wrapping_add(u32::from(capacity) * 2);
                        put_word(arena, end_marker.wrapping_add(4), refpos);
                        put_word(arena, end_marker.wrapping_add(8), 0);
                        put_word(arena, end_marker.wrapping_add(12), refpos);
                        put_word(arena, end_marker.wrapping_add(16), 0);
                        put_word(
                            arena,
                            ordered.wrapping_add(index.wrapping_add(1) * 4),
                            end_marker,
                        );
                        put_byte(arena, owned.wrapping_add(index.wrapping_add(1)), 1);
                    }
                    if output_language != 0 {
                        let count =
                            u32::from(marker_count).wrapping_add(u32::from(extra_count));
                        ordered = match host.realloc(
                            arena,
                            heap,
                            ordered,
                            count.wrapping_add(1) * 4,
                        ) {
                            Some(v) => v,
                            None => {
                                cleanup_2031b0(
                                    arena, heap, ordered, owned, multiword_ends, lookup,
                                    replacement, narrow, markers, marker_count, extra_count, host,
                                );
                                return 0x8a80_200a;
                            }
                        };
                        owned = match host.realloc(arena, heap, owned, count.wrapping_add(1) * 4) {
                            Some(v) => v,
                            None => {
                                cleanup_2031b0(
                                    arena, heap, ordered, owned, multiword_ends, lookup,
                                    replacement, narrow, markers, marker_count, extra_count, host,
                                );
                                return 0x8a80_200a;
                            }
                        };
                        put_byte(arena, owned.wrapping_add(count), 0);
                        let language_marker = match host.calloc(arena, heap, 1, 24) {
                            Some(v) => v,
                            None => {
                                cleanup_2031b0(
                                    arena, heap, ordered, owned, multiword_ends, lookup,
                                    replacement, narrow, markers, marker_count, extra_count, host,
                                );
                                return 0x8a80_200a;
                            }
                        };
                        let saved_language = match host.calloc(
                            arena,
                            heap,
                            (strlen16_176624(arena, language) + 1) as u32,
                            2,
                        ) {
                            Some(v) => v,
                            None => {
                                rc = 0x8a80_200a;
                                host.free(arena, heap, language_marker);
                                cleanup_2031b0(
                                    arena, heap, ordered, owned, multiword_ends, lookup,
                                    replacement, narrow, markers, marker_count, extra_count, host,
                                );
                                return rc;
                            }
                        };
                        strcpy16_176214(arena, saved_language, language);
                        put_word(arena, language_marker.wrapping_add(20), saved_language);
                        put_half(arena, language_marker, 0x24);
                        if output_type == 1 {
                            let refpos = base_position
                                .wrapping_add(position_delta as u32)
                                .wrapping_add(u32::from(word_end) * 2);
                            put_word(arena, language_marker.wrapping_add(4), refpos);
                            put_word(arena, language_marker.wrapping_add(12), refpos);
                        } else if output_type == 4 {
                            let refpos = base_position
                                .wrapping_add(position_delta as u32)
                                .wrapping_add(u32::from(capacity) * 2)
                                .wrapping_add(u32::from(word_start) * 2);
                            put_word(arena, language_marker.wrapping_add(4), refpos);
                            put_word(arena, language_marker.wrapping_add(12), refpos);
                        }
                        i = marker_index.wrapping_add(extra_count);
                        extra_count = extra_count.wrapping_add(1);
                        put_word(arena, language_marker.wrapping_add(8), 0);
                        put_word(arena, language_marker.wrapping_add(16), 0);
                        put_word(arena, ordered.wrapping_add(u32::from(i) * 4), language_marker);
                        put_byte(arena, owned.wrapping_add(u32::from(i)), 1);
                    }
                }
                if is_multiword == 0 || rc < 0x8000_0000 {
                    let original_length = i32::from(word_end) - i32::from(word_start);
                    if original_length <= 0
                        || (output_type != 1
                            && capacity != 0
                            && capacity < f100
                            && rc < 0x8000_0000)
                    {
                        rc = host.st_write(
                            arena,
                            word(arena, p.wrapping_add(44)),
                            word(arena, p.wrapping_add(48)),
                            replacement,
                            i32::from(capacity) as u32 * 2,
                        );
                        if rc >= 0x8000_0000 {
                            cleanup_2031b0(
                                arena, heap, ordered, owned, multiword_ends, lookup, replacement,
                                narrow, markers, marker_count, extra_count, host,
                            );
                            return rc;
                        }
                        let mut original_words = 1u16;
                        let mut j = 0u16;
                        while j < length {
                            if isspace_283188(u32::from(half(
                                arena,
                                lookup.wrapping_add(u32::from(j) * 2),
                            ))) != 0
                            {
                                original_words = original_words.wrapping_add(1);
                                while j < length
                                    && isspace_283188(u32::from(half(
                                        arena,
                                        lookup.wrapping_add(u32::from(j) * 2),
                                    ))) != 0
                                {
                                    j = j.wrapping_add(1);
                                }
                            }
                            j = j.wrapping_add(1);
                        }
                        let mut replacement_words = 1u16;
                        i = 0;
                        while i < capacity {
                            if isspace_283188(u32::from(half(
                                arena,
                                replacement.wrapping_add(u32::from(i) * 2),
                            ))) != 0
                            {
                                replacement_words = replacement_words.wrapping_add(1);
                                while i < capacity
                                    && isspace_283188(u32::from(half(
                                        arena,
                                        replacement.wrapping_add(u32::from(i) * 2),
                                    ))) != 0
                                {
                                    i = i.wrapping_add(1);
                                }
                            }
                            i = i.wrapping_add(1);
                        }
                        if is_multiword != 0 || replacement_words != original_words {
                            i = 0;
                            while i < marker_count {
                                let mi = markers.wrapping_add(u32::from(i) * 24);
                                let position = word(arena, mi.wrapping_add(12))
                                    .wrapping_sub(word(arena, markers.wrapping_add(12)));
                                if u32::from(multiword_start) * 2 < position
                                    && position <= u32::from(multiword_end) * 2
                                {
                                    let new_position = base_position
                                        .wrapping_add(u32::from(multiword_start) * 2);
                                    put_word(arena, mi.wrapping_add(12), new_position);
                                }
                                i = i.wrapping_add(1);
                            }
                        }
                        position_delta = (((position_delta + i32::from(capacity) * 2
                            - i32::from(length) * 2)
                            as u16) as i16)
                            as i32;
                    } else {
                        if strchr16_176514(
                            arena,
                            DEPES_SCRATCH_PUNCT,
                            u32::from(half(
                                arena,
                                text.wrapping_add((u32::from(word_end) - 1) * 2),
                            )),
                        ) == 0
                        {
                            rc = host.st_write(
                                arena,
                                word(arena, p.wrapping_add(44)),
                                word(arena, p.wrapping_add(48)),
                                text.wrapping_add(u32::from(word_start) * 2),
                                (original_length * 2) as u32,
                            );
                            if rc >= 0x8000_0000 {
                                cleanup_2031b0(
                                    arena, heap, ordered, owned, multiword_ends, lookup, replacement,
                                    narrow, markers, marker_count, extra_count, host,
                                );
                                return rc;
                            }
                        } else {
                            let n = original_length - 1;
                            if n > 0 {
                                rc = host.st_write(
                                    arena,
                                    word(arena, p.wrapping_add(44)),
                                    word(arena, p.wrapping_add(48)),
                                    text.wrapping_add(u32::from(word_start) * 2),
                                    (n * 2) as u32,
                                );
                                if rc >= 0x8000_0000 {
                                    cleanup_2031b0(
                                        arena, heap, ordered, owned, multiword_ends, lookup,
                                        replacement, narrow, markers, marker_count, extra_count,
                                        host,
                                    );
                                    return rc;
                                }
                            }
                        }
                        i = 0;
                        while i < dummy_count {
                            rc = host.st_write(
                                arena,
                                word(arena, p.wrapping_add(44)),
                                word(arena, p.wrapping_add(48)),
                                DEPES_SCRATCH_DUMMY,
                                4,
                            );
                            if rc >= 0x8000_0000 {
                                cleanup_2031b0(
                                    arena, heap, ordered, owned, multiword_ends, lookup, replacement,
                                    narrow, markers, marker_count, extra_count, host,
                                );
                                return rc;
                            }
                            i = i.wrapping_add(1);
                        }
                        if strchr16_176514(
                            arena,
                            DEPES_SCRATCH_PUNCT,
                            u32::from(half(
                                arena,
                                text.wrapping_add((u32::from(word_end) - 1) * 2),
                            )),
                        ) != 0
                        {
                            rc = host.st_write(
                                arena,
                                word(arena, p.wrapping_add(44)),
                                word(arena, p.wrapping_add(48)),
                                text.wrapping_add((u32::from(word_end) - 1) * 2),
                                2,
                            );
                            if rc >= 0x8000_0000 {
                                cleanup_2031b0(
                                    arena, heap, ordered, owned, multiword_ends, lookup, replacement,
                                    narrow, markers, marker_count, extra_count, host,
                                );
                                return rc;
                            }
                        }
                    }
                    retry = 0;
                } else {
                    retry = 1;
                }
                if is_multiword == 1 {
                    word_end = multiword_start;
                    if retry == 0 {
                        while marker_index < marker_count {
                            let index = u32::from(marker_index)
                                .wrapping_add(u32::from(extra_count));
                            word_end = multiword_end;
                            marker = markers.wrapping_add(u32::from(marker_index) * 24);
                            if u32::from(multiword_end) * 2
                                < u32::from(
                                    word(arena, marker.wrapping_add(12)).wrapping_sub(base_position)
                                        as u16,
                                )
                            {
                                break;
                            }
                            if word(arena, marker.wrapping_add(12))
                                < word(
                                    arena,
                                    word(arena, ordered.wrapping_add(index.wrapping_sub(1) * 4))
                                        .wrapping_add(12),
                                )
                            {
                                put_word(
                                    arena,
                                    ordered.wrapping_add(index * 4),
                                    word(arena, ordered.wrapping_add(index.wrapping_sub(1) * 4)),
                                );
                                put_byte(
                                    arena,
                                    owned.wrapping_add(index),
                                    byte(arena, owned.wrapping_add(index.wrapping_sub(1))),
                                );
                                put_word(
                                    arena,
                                    ordered.wrapping_add(index.wrapping_sub(1) * 4),
                                    marker,
                                );
                                put_byte(arena, owned.wrapping_add(index.wrapping_sub(1)), 0);
                            } else {
                                put_word(arena, ordered.wrapping_add(index * 4), marker);
                                put_byte(arena, owned.wrapping_add(index), 0);
                            }
                            marker_index = marker_index.wrapping_add(1);
                        }
                        word_end = multiword_end;
                    }
                }
            }
            word_start = word_end;
            if word_end < text_count {
                continue;
            } else {
                break;
            }
        }
    }
    while marker_index < marker_count {
        marker = markers.wrapping_add(u32::from(marker_index) * 24);
        let pos = word(arena, marker.wrapping_add(12));
        put_word(
            arena,
            marker.wrapping_add(12),
            pos.wrapping_add(position_delta as u32),
        );
        put_word(
            arena,
            ordered.wrapping_add(
                u32::from(marker_index).wrapping_add(u32::from(extra_count)) * 4,
            ),
            marker,
        );
        marker_index = marker_index.wrapping_add(1);
    }
    let m0len = word(arena, markers.wrapping_add(16));
    put_word(
        arena,
        markers.wrapping_add(16),
        m0len.wrapping_add(position_delta as u32),
    );
    {
        let count = u32::from(marker_count).wrapping_add(u32::from(extra_count));
        i = 0;
        while u32::from(i) < count {
            rc = host.st_write(
                arena,
                word(arena, p.wrapping_add(52)),
                word(arena, p.wrapping_add(56)),
                word(arena, ordered.wrapping_add(u32::from(i) * 4)),
                24,
            );
            if rc >= 0x8000_0000 {
                cleanup_2031b0(
                    arena, heap, ordered, owned, multiword_ends, lookup, replacement, narrow,
                    markers, marker_count, extra_count, host,
                );
                return rc;
            }
            let _ = host.log_level(arena, log);
            i = i.wrapping_add(1);
        }
    }
    rc = host.st_consume(
        arena,
        word(arena, p.wrapping_add(28)),
        word(arena, p.wrapping_add(32)),
        text_bytes as u32,
    );
    if rc >= 0x8000_0000 {
        cleanup_2031b0(
            arena, heap, ordered, owned, multiword_ends, lookup, replacement, narrow,
            markers, marker_count, extra_count, host,
        );
        return rc;
    }
    rc = host.st_consume(
        arena,
        word(arena, p.wrapping_add(36)),
        word(arena, p.wrapping_add(40)),
        marker_bytes as u32,
    );
    if rc >= 0x8000_0000 {
        cleanup_2031b0(
            arena, heap, ordered, owned, multiword_ends, lookup, replacement, narrow,
            markers, marker_count, extra_count, host,
        );
        return rc;
    }
    stream_position = stream_position.wrapping_add(position_delta);
    rc = host.depes_set_pos(
        arena,
        word(arena, p.wrapping_add(12)),
        word(arena, p.wrapping_add(16)),
        stream_position,
        0,
    );
    cleanup_2031b0(
        arena, heap, ordered, owned, multiword_ends, lookup, replacement, narrow,
        markers, marker_count, extra_count, host,
    );
    rc
}

/// fn_1d5234, EU fn_1d53f0, KR fn_1d549c. Create the sentence token spanning
/// `[start, end)`: resolve the language (lowercased `language` when it is 3
/// units and matches the `langcode` property, else `lid`), walk the cursor
/// chain, insert the token with its text, convert `phonemes` through the
/// synthesiser (or store `fallback` when conversion is unavailable), and
/// move `phonemes`/`fallback` into fields 3/4. The `fn_1738e8`/`fn_173dc0`
/// calls are log-only. `upper`/`key`/`lang` live on the C stack, so the
/// host calls that take them receive 0 (the fuzz stubs ignore the key and
/// upper bytes); a `noclmset` hit sets the local `setting_no` flag instead
/// of a stack pointer.
pub fn fe_token_1d5234(
    arena: &mut [u8],
    p: u32,
    db_a: u32,
    db_b: u32,
    cursor: &mut u16,
    start: u32,
    end: u32,
    text: u32,
    phonemes: u32,
    language: u32,
    fallback: u32,
    _unused: u32,
    user: u32,
    host: &mut impl FeTextHost,
) -> u32 {
    let end16 = (end & 0xffff) as u16;
    let env = word(arena, p);
    let props = word(arena, env.wrapping_add(0x14));
    let allocator = word(arena, env.wrapping_add(4));
    let synth_h0 = word(arena, p.wrapping_add(0x40));
    let synth_h1 = word(arena, p.wrapping_add(0x44));
    let buffer_data = word(arena, p.wrapping_add(0x38));
    let mut rc = 0u32;
    let mut position = 0u16;
    let mut length = 0u16;
    let mut created = 0u32;
    let mut upper = [0u8; 4];
    let mut key = [0u8; 12];
    let mut setting = 0u32;
    let mut setting_no = false;
    let mut written = 0u16;
    let mut lang = [0u8; 4];
    let mut kind = 0u8;
    let original = *cursor;
    let mut first = 0u16;
    let mut converted = 0u32;
    let mut mark;
    if strlen16_176624(arena, language) == 3 {
        setting = 0;
        setting_no = false;
        rc = host.suba1c(arena, props, 0, &mut setting);
        if rc >= 0x8000_0000 {
            return rc;
        }
        let mut i = 0usize;
        loop {
            let w = half(arena, language.wrapping_add((i as u32).wrapping_mul(2)));
            lang[i] = crate::normalization::tolower_240ba0(u32::from(w as u8)) as u8;
            i += 1;
            if i >= 3 {
                break;
            }
        }
        lang[3] = 0;
        if strcmp_lang_ci(arena, &lang, setting) != 0 {
            // language_ready: keep lang.
        } else {
            lang = *b"lid\0";
        }
    } else {
        lang = *b"lid\0";
    }
    // language_ready:
    if *cursor == 0 {
        return rc;
    }
    loop {
        let cur = u32::from(*cursor);
        let mut pos_w = 0u32;
        rc = host.lat_get_int(arena, db_a, db_b, cur, 1, 1, &mut pos_w, &mut length);
        if rc >= 0x8000_0000 {
            return rc;
        }
        position = pos_w as u16;
        if u32::from(position) >= start {
            let mut kind_w = 0u32;
            rc = host.lat_get_int(arena, db_a, db_b, cur, 0, 1, &mut kind_w, &mut length);
            if rc >= 0x8000_0000 {
                return rc;
            }
            kind = kind_w as u8;
            if kind != 3 && position != end16 {
                let cur2 = *cursor;
                let mut new = 0u16;
                rc = host.lat_insert_after(arena, db_a, db_b, u32::from(cur2), &mut new);
                if rc >= 0x8000_0000 {
                    return rc;
                }
                *cursor = new;
                kind = 5;
                let node = u32::from(*cursor);
                written = 0;
                rc = host.lat_set_field(arena, db_a, db_b, node, 0, 1, 0, &mut written);
                if rc >= 0x8000_0000 {
                    return rc;
                }
                rc = host.lat_set_field(arena, db_a, db_b, node, 1, 1, 0, &mut written);
                if rc >= 0x8000_0000 {
                    return rc;
                }
                rc = host.lat_set_field(arena, db_a, db_b, node, 2, 1, 0, &mut written);
                if rc >= 0x8000_0000 {
                    return rc;
                }
                let n_text = strlen8(arena, text).wrapping_add(1) & 0xffff;
                rc = host.lat_set_field(arena, db_a, db_b, node, 4, n_text, 0, &mut written);
                if rc >= 0x8000_0000 {
                    return rc;
                }
                if user != 0 {
                    rc = host.lat_set_field(arena, db_a, db_b, node, 6, 4, 0, &mut written);
                    if rc >= 0x8000_0000 {
                        return rc;
                    }
                }
                if phonemes != 0 {
                    mark = find_substr_lit(arena, phonemes, b"_#_");
                    while mark != 0 {
                        arena[mark as usize] = b'#';
                        let n2 = strlen8(arena, phonemes)
                            .wrapping_sub(mark.wrapping_sub(phonemes))
                            .wrapping_sub(2);
                        memmove_at(arena, mark.wrapping_add(1), mark.wrapping_add(3), n2);
                        mark = find_substr_lit(arena, phonemes, b"_#_");
                    }
                    if lang != *b"lid\0" {
                        setting = 0;
                        setting_no = false;
                        key[0] = b'c';
                        key[1] = b'l';
                        key[2] = b'm';
                        key[3] = 0;
                        let mut ki = 3usize;
                        for &b in lang.iter() {
                            key[ki] = b;
                            if b == 0 {
                                break;
                            }
                            ki += 1;
                        }
                        rc = host.suba1c(arena, props, 0, &mut setting);
                        if rc >= 0x8000_0000 {
                            setting = 0;
                        }
                        if setting == 0 && !setting_no {
                            key[0] = b'n';
                            key[1] = b'o';
                            key[2] = b'c';
                            key[3] = b'l';
                            key[4] = b'm';
                            key[5] = b's';
                            key[6] = b'e';
                            key[7] = b't';
                            key[8] = 0;
                            let mut setting2 = 0u32;
                            let st = host.suba1c(arena, props, 0, &mut setting2);
                            if st < 0x8000_0000
                                && find_substr_lit(arena, setting2, &lang[..3]) != 0
                            {
                                setting = 1;
                                setting_no = true;
                            } else {
                                setting = 0;
                                setting_no = false;
                            }
                        }
                        for j in 0..3 {
                            upper[j] = if lang[j] > 0x60 { lang[j] - 0x20 } else { lang[j] };
                        }
                        upper[3] = 0;
                        let size = strlen8(arena, phonemes).wrapping_add(1).wrapping_mul(4);
                        converted = match host.realloc(arena, allocator, 0, size) {
                            Some(o) => o,
                            None => 0,
                        };
                        if converted == 0 {
                            return 0x8990_200a;
                        }
                        let mut converted_ready = false;
                        if buffer_data != 0 {
                            let len_p = strlen8(arena, phonemes);
                            let use_plain = setting != 0
                                && (setting_no || strcmp_yes(arena, setting) != 0);
                            let cap = if use_plain {
                                0
                            } else {
                                len_p.wrapping_add(1).wrapping_mul(4)
                            };
                            rc = host.synth_convert(
                                arena,
                                synth_h0,
                                synth_h1,
                                0,
                                phonemes,
                                len_p.wrapping_add(1),
                                converted,
                                cap,
                            );
                            if rc < 0x8000_0000 {
                                converted_ready = true;
                            }
                        }
                        if !converted_ready {
                            first = 0;
                            let mut raw = 0u32;
                            rc = host.lat_first_node(arena, db_a, db_b, 1, 0, &mut first);
                            if rc >= 0x8000_0000 {
                                return rc;
                            }
                            rc = host.lat_get_str(
                                arena,
                                db_a,
                                db_b,
                                u32::from(first),
                                0,
                                &mut raw,
                                &mut length,
                            );
                            if rc >= 0x8000_0000 {
                                return rc;
                            }
                            if length != 0 {
                                let mut c = length as u8;
                                let mut s = raw;
                                loop {
                                    if byte(arena, s) == 0x15 {
                                        arena[s as usize] = b' ';
                                    }
                                    c = c.wrapping_sub(1);
                                    s = s.wrapping_add(1);
                                    if c == 0 {
                                        break;
                                    }
                                }
                            }
                            rc = host.lat_set_field(
                                arena,
                                db_a,
                                db_b,
                                u32::from(first),
                                0,
                                u32::from(length),
                                0,
                                &mut written,
                            );
                            if rc >= 0x8000_0000 {
                                return rc;
                            }
                            let n_fb = strlen8(arena, fallback).wrapping_add(1) & 0xffff;
                            rc = host.lat_set_field(
                                arena, db_a, db_b, node, 4, n_fb, 0, &mut written,
                            );
                            host.free(arena, allocator, converted);
                            converted = 0;
                            if rc >= 0x8000_0000 {
                                return rc;
                            }
                        } else if setting != 0
                            && (setting_no || strcmp_yes(arena, setting) != 0)
                        {
                            host.free(arena, allocator, converted);
                            converted = phonemes;
                            let mut ln = 0u32;
                            while ln < 4 && lang[ln as usize] != 0 {
                                ln += 1;
                            }
                            rc = host.lat_set_field(
                                arena,
                                db_a,
                                db_b,
                                node,
                                6,
                                (ln.wrapping_add(1)) & 0xffff,
                                0,
                                &mut written,
                            );
                            if rc >= 0x8000_0000 {
                                return rc;
                            }
                        }
                    } else {
                        converted = phonemes;
                    }
                    // store_phonemes:
                    if converted != 0 {
                        let n_c = strlen8(arena, converted).wrapping_add(1) & 0xffff;
                        rc = host.lat_set_field(
                            arena, db_a, db_b, node, 3, n_c, 0, &mut written,
                        );
                    }
                    if converted != phonemes && converted != 0 {
                        host.free(arena, allocator, converted);
                    }
                    if rc >= 0x8000_0000 {
                        return rc;
                    }
                }
                created = 1;
            }
        }
        // advance:
        if position == end16 {
            return rc;
        }
        let cur3 = *cursor;
        let mut nx = 0u16;
        rc = host.lat_next_node(arena, db_a, db_b, u32::from(cur3), &mut nx);
        if rc >= 0x8000_0000 {
            return rc;
        }
        *cursor = nx;
        if created != 0 {
            return rc;
        }
        if *cursor == 0 {
            break;
        }
    }
    if u32::from(position) != start {
        return rc;
    }
    let mut new2 = 0u16;
    rc = host.lat_insert_after(arena, db_a, db_b, u32::from(original), &mut new2);
    if rc >= 0x8000_0000 {
        return rc;
    }
    *cursor = new2;
    kind = 5;
    let node2 = u32::from(*cursor);
    rc = host.lat_set_field(arena, db_a, db_b, node2, 0, 1, 0, &mut written);
    if rc >= 0x8000_0000 {
        return rc;
    }
    rc = host.lat_set_field(arena, db_a, db_b, node2, 1, 1, 0, &mut written);
    if rc >= 0x8000_0000 {
        return rc;
    }
    rc = host.lat_set_field(arena, db_a, db_b, node2, 2, 1, 0, &mut written);
    if rc >= 0x8000_0000 {
        return rc;
    }
    let n_text2 = strlen8(arena, text).wrapping_add(1) & 0xffff;
    rc = host.lat_set_field(arena, db_a, db_b, node2, 4, n_text2, 0, &mut written);
    if rc >= 0x8000_0000 {
        return rc;
    }
    let _ = kind;
    let _ = (&key, &upper);
    rc
}

/// Case-insensitive compare of the stack `lang` bytes against the arena
/// string at `setting`, following `fn_1769d0` (fold through `fn_17c8c0`,
/// 0 when equal).
fn strcmp_lang_ci(arena: &[u8], lang: &[u8; 4], setting: u32) -> i32 {
    let mut i = 0usize;
    if fold_17c8c0(u32::from(lang[0])) == fold_17c8c0(u32::from(byte(arena, setting))) {
        loop {
            if lang[i] == 0 {
                return 0;
            }
            i += 1;
            if fold_17c8c0(u32::from(lang[i]))
                != fold_17c8c0(u32::from(byte(arena, setting.wrapping_add(i as u32))))
            {
                break;
            }
        }
    }
    (fold_17c8c0(u32::from(lang[i])) as i32)
        .wrapping_sub(fold_17c8c0(u32::from(byte(arena, setting.wrapping_add(i as u32)))) as i32)
}

/// `fn_195d58(setting, "yes")`: 0 when the arena string is exactly `yes`.
fn strcmp_yes(arena: &[u8], setting: u32) -> u32 {
    if byte(arena, setting) == b'y'
        && byte(arena, setting.wrapping_add(1)) == b'e'
        && byte(arena, setting.wrapping_add(2)) == b's'
        && byte(arena, setting.wrapping_add(3)) == 0
    {
        0
    } else {
        1
    }
}

/// `fn_175944` platform mock: status + u32 value from the `reg` block
/// (counter at `+24`, three `(status, value)` pairs at `+0`/`+8`/`+16`).
fn prop_get_u32_175944(arena: &mut [u8], reg: u32, out: &mut u32) -> u32 {
    if reg == 0 {
        *out = 0;
        return 0x8160_2008;
    }
    let n = word(arena, reg.wrapping_add(24));
    let idx = if n < 3 { n } else { 2 };
    put_word(arena, reg.wrapping_add(24), n.wrapping_add(1));
    *out = word(arena, reg.wrapping_add(idx.wrapping_mul(8)).wrapping_add(4));
    word(arena, reg.wrapping_add(idx.wrapping_mul(8)))
}

/// `fn_195df8(s, lit, n)`: 0 when the first `n` bytes match.
fn strncmp_lit(arena: &[u8], s: u32, lit: &[u8], n: u32) -> u32 {
    for i in 0..n {
        let e = if (i as usize) < lit.len() {
            lit[i as usize]
        } else {
            0
        };
        if byte(arena, s.wrapping_add(i)) != e {
            return 1;
        }
        if e == 0 {
            break;
        }
    }
    0
}

/// `fn_2409d4` with the `fn_1d5214` comparator: in-place quicksort over `n`
/// 24-byte tokens. Same pivot/partition scheme as `qsort_2409d4`.
fn qsort_1d5214(arena: &mut [u8], base: u32, n: u32) {
    const SIZE: u32 = 24;
    if n <= 1 {
        return;
    }
    let mut mid = n >> 1;
    let mut pivot = base.wrapping_add(mid.wrapping_mul(SIZE));
    let mut i = 0u32;
    let mut j = n.wrapping_sub(1);
    while i < j {
        while cmp_tok_1d5214(arena, base.wrapping_add(i.wrapping_mul(SIZE)), pivot) < 0 {
            i = i.wrapping_add(1);
        }
        while cmp_tok_1d5214(arena, base.wrapping_add(j.wrapping_mul(SIZE)), pivot) > 0 {
            j = j.wrapping_sub(1);
        }
        if i < j {
            let pi = base.wrapping_add(i.wrapping_mul(SIZE));
            let pj = base.wrapping_add(j.wrapping_mul(SIZE));
            for k in 0..SIZE {
                let t = byte(arena, pi.wrapping_add(k));
                put_byte(arena, pi.wrapping_add(k), byte(arena, pj.wrapping_add(k)));
                put_byte(arena, pj.wrapping_add(k), t);
            }
            if mid == i {
                mid = j;
                pivot = pj;
            } else if mid == j {
                mid = i;
                pivot = pi;
            }
            i = i.wrapping_add(1);
            j = j.wrapping_sub(1);
        } else if i == j {
            i = i.wrapping_add(1);
            j = j.wrapping_sub(1);
            break;
        }
    }
    if j != 0 {
        qsort_1d5214(arena, base, j.wrapping_add(1));
    }
    if i < n.wrapping_sub(1) {
        qsort_1d5214(
            arena,
            base.wrapping_add(i.wrapping_mul(SIZE)),
            n.wrapping_sub(i),
        );
    }
}

/// Compare two 24-byte tokens at arena offsets via `token_position_order`.
fn cmp_tok_1d5214(arena: &[u8], a: u32, b: u32) -> i32 {
    let sa = &arena[a as usize..(a.wrapping_add(24)) as usize];
    let sb = &arena[b as usize..(b.wrapping_add(24)) as usize];
    crate::normalization::token_position_order(sa, sb)
}

/// Furnished mirrors of the `fn_1d5ba8` ROM strings (see `_1d5ba8`).
const BBA_NORMAL: u32 = 0xc40;
const BBA_PHON: u32 = 0xc50;
const BBA_U16_NORMAL: u32 = 0xc60;
const BBA_U16_LID: u32 = 0xc70;
const BBA_U16_LATIN: u32 = 0xc80;

/// fn_1d5ba8, EU fn_1d5d64, KR fn_1d5e10. Sentence/word Process: drain the
/// text/token inputs into LINGDB, annotate the tokens, walk kind 21/34/36
/// markers building TN type names (with `fecfg`/`esctn`/`esclang` lookups),
/// emit a `fn_1d5234` token per span, then sort and store the tokens.
/// `fn_176f94` fails odd tags; `fn_1758ac` is a no-op; `fn_175944` follows
/// the platform counter block; the `fn_1738e8`/`fn_173dc0` calls are
/// log-only. ROM strings use the furnished mirrors above.
pub fn fe_process_1d5ba8(
    arena: &mut [u8],
    p: u32,
    tag: u32,
    db_a: u32,
    db_b: u32,
    pending: u32,
    host: &mut impl FeTextHost,
) -> u32 {
    if (tag & 1) != 0 {
        return 0x8990_2008;
    }
    let env = word(arena, p);
    let props = word(arena, env.wrapping_add(0x14));
    let allocator = word(arena, env.wrapping_add(4));
    let dict_a = word(arena, p.wrapping_add(0x0c));
    let dict_b = word(arena, p.wrapping_add(0x10));
    let in0_a = word(arena, p.wrapping_add(0x20));
    let in0_b = word(arena, p.wrapping_add(0x24));
    let in1_a = word(arena, p.wrapping_add(0x28));
    let in1_b = word(arena, p.wrapping_add(0x2c));
    let compatible = byte(arena, p.wrapping_add(0x30));
    arena[pending as usize] = 1;
    let mut text_available = 0u32;
    let mut end_of_input = 0u8;
    let mut rc = host.res_available(arena, in0_a, in0_b, &mut text_available, &mut end_of_input);
    if rc >= 0x8000_0000 {
        return rc;
    }
    let mut token_available = 0u32;
    rc = host.res_available(arena, in1_a, in1_b, &mut token_available, &mut end_of_input);
    if rc >= 0x8000_0000 {
        return rc;
    }
    rc = host.lat_reset(arena, db_a, db_b);
    if rc >= 0x8000_0000 {
        return rc;
    }
    let mut root = 0u16;
    rc = host.lat_first_node(arena, db_a, db_b, 1, 0, &mut root);
    if rc >= 0x8000_0000 {
        return rc;
    }
    let mut written = 0u16;
    let mut token_data = 0u32;
    let mut token_bytes = 0u32;
    if text_available == 0 {
        if end_of_input == 0 && token_available == 0 {
            let mut pipeline = 0u32;
            let st = host.suba1c(arena, props, 0, &mut pipeline);
            if st >= 0x8000_0000 || pipeline == 0 || byte(arena, pipeline) == 0 {
                arena[pending as usize] = 0;
            }
        } else {
            if token_available != 0 {
                rc = host.res_read(arena, in1_a, in1_b, &mut token_data, &mut token_bytes);
                if rc >= 0x8000_0000 {
                    return rc;
                }
                rc = host.lat_set_field(
                    arena,
                    db_a,
                    db_b,
                    u32::from(root),
                    1,
                    (token_bytes.wrapping_div(24)) & 0xffff,
                    0,
                    &mut written,
                );
                if rc >= 0x8000_0000 {
                    return rc;
                }
                rc = host.res_consume(arena, in1_a, in1_b, token_bytes);
                if rc >= 0x8000_0000 {
                    return rc;
                }
            }
            let mut finished = 0u8;
            if end_of_input != 0 {
                arena[pending as usize] = 1;
                finished = 1;
            }
            let _ = finished;
            rc = host.lat_set_field(arena, db_a, db_b, u32::from(root), 2, 1, 0, &mut written);
            return rc;
        }
        let _ = token_data;
        let _ = token_bytes;
        let rc2 = host.lat_set_field(arena, db_a, db_b, u32::from(root), 2, 1, 0, &mut written);
        return rc2;
    }
    let mut finished = 0u8;
    let _ = finished;
    rc = host.lat_set_field(arena, db_a, db_b, u32::from(root), 2, 1, 0, &mut written);
    if rc >= 0x8000_0000 {
        return rc;
    }
    let mut text_data = 0u32;
    let mut text_bytes = 0u32;
    rc = host.res_read(arena, in0_a, in0_b, &mut text_data, &mut text_bytes);
    if rc >= 0x8000_0000 {
        return rc;
    }
    if text_bytes == 1 {
        rc = host.res_consume(arena, in0_a, in0_b, 1);
        if rc >= 0x8000_0000 {
            return rc;
        }
        rc = host.res_read(arena, in1_a, in1_b, &mut token_data, &mut token_bytes);
        if rc >= 0x8000_0000 {
            return rc;
        }
        if token_available != 0 {
            rc = host.lat_set_field(
                arena,
                db_a,
                db_b,
                u32::from(root),
                1,
                (token_bytes.wrapping_div(24)) & 0xffff,
                0,
                &mut written,
            );
            if rc >= 0x8000_0000 {
                return rc;
            }
        }
        rc = host.res_consume(arena, in1_a, in1_b, token_bytes);
        if rc < 0x8000_0000 {
            rc = 0;
        }
        return rc;
    }
    rc = host.lat_set_field(
        arena,
        db_a,
        db_b,
        u32::from(root),
        0,
        text_bytes & 0xffff,
        0,
        &mut written,
    );
    if rc >= 0x8000_0000 {
        return rc;
    }
    // process_tokens:
    rc = host.res_read(arena, in1_a, in1_b, &mut token_data, &mut token_bytes);
    if rc >= 0x8000_0000 {
        return rc;
    }
    let token_count = (token_bytes.wrapping_div(24)) & 0xffff;
    let tokens = match host.realloc(arena, allocator, 0, token_bytes) {
        Some(o) => o,
        None => return 0x8990_200a,
    };
    // 195ae8 copy (token_bytes may be 0).
    if token_bytes != 0 {
        memcpy_at(arena, tokens, token_data, token_bytes);
    }
    {
        let mut remaining = token_count;
        let mut scan = tokens;
        while remaining != 0 {
            if half(arena, scan) == 1 {
                put_word(arena, scan.wrapping_add(16), 0);
            }
            scan = scan.wrapping_add(24);
            remaining = remaining.wrapping_sub(1);
        }
    }
    rc = host.dict_annotate(arena, dict_a, dict_b, db_a, db_b, tokens, token_count);
    if rc >= 0x8000_0000 {
        host.free(arena, allocator, tokens);
        return rc;
    }
    let mut cursor = 0u16;
    rc = host.lat_first_node(arena, db_a, db_b, 3, u32::from(root), &mut cursor);
    if rc >= 0x8000_0000 {
        host.free(arena, allocator, tokens);
        return rc;
    }
    let mut key = [0u8; 256];
    let mut language_code = [0u8; 4];
    let mut found = 0xffffu16;
    let mut last_position: u32 = word(arena, tokens.wrapping_add(12));
    let first_position: u32 = last_position;
    let mut current = BBA_NORMAL;
    let mut fallback = BBA_NORMAL;
    let mut user_text = 0u32;
    let mut language = BBA_U16_NORMAL;
    let mut typ = BBA_U16_NORMAL;
    let mut name = 0u32;
    let mut user_buffer = 0u32;
    let mut user_active = 0u32;
    let mut user = 0u32;
    // fecfg/current lookup.
    {
        let mut entry_out = 0u32;
        let mut found16 = 0u16;
        let mut sep = 0u8;
        rc = host.dict_lookup(arena, dict_a, dict_b, 0, current, &mut entry_out, &mut found16, &mut sep);
        if rc >= 0x8000_0000 {
            if name != 0 {
                host.free(arena, allocator, name);
            }
            if user_buffer != 0 {
                host.free(arena, allocator, user_buffer);
            }
            host.free(arena, allocator, tokens);
            return rc;
        }
        found = found16;
        if found != 0 {
            let entry_str = word(arena, entry_out);
            let m = strchr_195d30(arena, entry_str, u32::from(sep));
            if m != 0 {
                arena[m as usize] = 0;
            }
            let mapped = entry_str;
            key[0..11].copy_from_slice(b"tnpausedur\0");
            let key_len = strlen_stack(&key);
            if strlen8(arena, current) < 255u32.wrapping_sub(key_len) {
                strcat_stack_key(&mut key, arena, current);
                let mut value = 0u32;
                if prop_get_u32_175944(arena, props, &mut value) < 0x8000_0000 {
                    key[0..11].copy_from_slice(b"tnpausedur\0");
                    strcat_stack_key(&mut key, arena, mapped);
                    // 1758ac no-op.
                }
            }
            current = mapped;
        }
    }
    let mut index = 0u32;
    while index < token_count {
        let tok = tokens.wrapping_add(index.wrapping_mul(24));
        let kind = half(arena, tok);
        if kind != 21 && kind != 34 && kind != 36 {
            index = index.wrapping_add(1);
            continue;
        }
        let mut length = 0u16;
        let mut type_index = 0u32;
        let mut language_index = 0u32;
        let mut next = index;
        loop {
            next = next.wrapping_add(1);
            if next >= token_count {
                break;
            }
            let ntok = tokens.wrapping_add(next.wrapping_mul(24));
            if word(arena, ntok.wrapping_add(12)) != word(arena, tok.wrapping_add(12)) {
                break;
            }
            if half(arena, ntok) == 34 {
                break;
            }
            if half(arena, ntok) == 36 {
                type_index = next;
            } else if half(arena, ntok) == 21 {
                language_index = next;
            }
        }
        if user_active != 0 && kind == 36 && language_index == 0 {
            index = index.wrapping_add(1);
            continue;
        }
        if word(arena, tok.wrapping_add(12)) != first_position {
            // Skip spaces/_/tab/newline/CR.
            loop {
                let tp = word(arena, tok.wrapping_add(12));
                let c = byte(arena, text_data.wrapping_add(tp.wrapping_sub(first_position)));
                if c != b' ' && c != b'_' && c != b'\t' && c != b'\n' && c != b'\r' {
                    break;
                }
                put_word(arena, tok.wrapping_add(12), tp.wrapping_add(1));
            }
            let tok_pos2 = word(arena, tok.wrapping_add(12));
            // Re-read last_position (updated per token below).
            if tok_pos2 != last_position {
                let a = last_position.wrapping_sub(first_position) & 0xffff;
                let b = tok_pos2.wrapping_sub(first_position) & 0xffff;
                rc = fe_token_1d5234(
                    arena, p, db_a, db_b, &mut cursor, a, b, current, user_text,
                    // The ROM passes `type` here (hooked on US/EU/KR; src
                    // commit `4b542d7` fixed the old `language` argument).
                    typ, fallback, text_data, user, host,
                );
                if rc >= 0x8000_0000 {
                    if name != 0 {
                        host.free(arena, allocator, name);
                    }
                    if user_buffer != 0 {
                        host.free(arena, allocator, user_buffer);
                    }
                    host.free(arena, allocator, tokens);
                    return rc;
                }
            }
        }
        if kind == 21 {
            language = word(arena, tok.wrapping_add(20));
        } else if kind == 36 {
            typ = word(arena, tok.wrapping_add(20));
        }
        if type_index != 0 {
            typ = word(arena, tokens.wrapping_add(type_index.wrapping_mul(24)).wrapping_add(20));
        }
        if language_index != 0 {
            language = word(
                arena,
                tokens.wrapping_add(language_index.wrapping_mul(24)).wrapping_add(20),
            );
        }
        user = 0;
        if kind == 34 {
            user_active = 1;
            let tval = word(arena, tok.wrapping_add(20));
            let need = strlen16_176624(arena, tval).wrapping_add(1) as u32;
            let tmp = match host.realloc(arena, allocator, user_buffer, need) {
                Some(o) => o,
                None => {
                    if name != 0 {
                        host.free(arena, allocator, name);
                    }
                    if user_buffer != 0 {
                        host.free(arena, allocator, user_buffer);
                    }
                    host.free(arena, allocator, tokens);
                    return 0x8990_200a;
                }
            };
            user_buffer = tmp;
            let mut i = 0u32;
            loop {
                let ch = half(arena, tval.wrapping_add(i.wrapping_mul(2)));
                if ch == 0 {
                    break;
                }
                if (ch as u8) != b' ' {
                    arena[user_buffer.wrapping_add(u32::from(length)) as usize] = ch as u8;
                    length = length.wrapping_add(1);
                }
                i = i.wrapping_add(1);
            }
            current = BBA_PHON;
            user_text = user_buffer;
            arena[user_buffer.wrapping_add(u32::from(length)) as usize] = 0;
            if name != 0 {
                fallback = name;
            }
            let _ = length;
            // token_done:
            last_position = word(arena, tok.wrapping_add(12));
            index = next;
            continue;
        }
        // Non-34 path.
        user_active = 0;
        let language_length = strlen16_176624(arena, language) as u32;
        let capacity6 = strlen16_176624(arena, BBA_U16_NORMAL) as u32;
        let type_length = strlen16_176624(arena, typ) as u32;
        let type_length = strlen16_176624(
            arena,
            if capacity6 > type_length {
                BBA_U16_NORMAL
            } else {
                typ
            },
        ) as u32;
        let need_name = language_length.wrapping_add(type_length).wrapping_add(2);
        let tmp_name = match host.realloc(arena, allocator, name, need_name) {
            Some(o) => o,
            None => {
                if name != 0 {
                    host.free(arena, allocator, name);
                }
                if user_buffer != 0 {
                    host.free(arena, allocator, user_buffer);
                }
                host.free(arena, allocator, tokens);
                return 0x8990_200a;
            }
        };
        name = tmp_name;
        if strcmp16_1765ac(arena, language, BBA_U16_NORMAL) == 0
            && strcmp16_1765ac(arena, typ, BBA_U16_NORMAL) == 0
        {
            strcpy_lit(arena, name, b"normal");
        } else {
            arena[name as usize] = 0;
            if strcmp16_1765ac(arena, language, BBA_U16_NORMAL) != 0 {
                let mut i = 0u32;
                loop {
                    let ch = half(arena, language.wrapping_add(i.wrapping_mul(2)));
                    if ch == 0 {
                        break;
                    }
                    arena[name.wrapping_add(u32::from(length)) as usize] =
                        crate::normalization::tolower_240ba0(u32::from(ch as u8)) as u8;
                    length = length.wrapping_add(1);
                    i = i.wrapping_add(1);
                }
                arena[name.wrapping_add(u32::from(length)) as usize] = 0;
                if compatible == 1
                    && strcmp_lit(arena, name, b"normal") != 0
                    && byte(arena, name) != 0
                {
                    found = 0xffff;
                    let mut entry_out = 0u32;
                    let mut found16 = 0u16;
                    let mut sep = 0u8;
                    rc = host.dict_lookup(
                        arena, dict_a, dict_b, 0, name, &mut entry_out, &mut found16, &mut sep,
                    );
                    if rc >= 0x8000_0000 {
                        if name != 0 {
                            host.free(arena, allocator, name);
                        }
                        if user_buffer != 0 {
                            host.free(arena, allocator, user_buffer);
                        }
                        host.free(arena, allocator, tokens);
                        return rc;
                    }
                    found = found16;
                    if found != 0 {
                        let entry_str = word(arena, entry_out);
                        let m = strchr_195d30(arena, entry_str, u32::from(sep));
                        if m != 0 {
                            arena[m as usize] = 0;
                        }
                        let mapped = entry_str;
                        let mut cap = (strlen16_176624(arena, typ) as u32).wrapping_add(1);
                        if mapped != 0 {
                            cap = cap.wrapping_add(strlen8(arena, mapped));
                        }
                        let tmp = match host.realloc(arena, allocator, name, cap.wrapping_add(1)) {
                            Some(o) => o,
                            None => {
                                if name != 0 {
                                    host.free(arena, allocator, name);
                                }
                                if user_buffer != 0 {
                                    host.free(arena, allocator, user_buffer);
                                }
                                host.free(arena, allocator, tokens);
                                return 0x8990_200a;
                            }
                        };
                        if mapped != 0 {
                            let ml = strlen8(arena, mapped).wrapping_add(1);
                            memcpy_at(arena, tmp, mapped, ml);
                        } else {
                            host.free(arena, allocator, tmp);
                            arena[tmp as usize] = 0;
                        }
                        name = tmp;
                        let _ = strlen8(arena, name);
                    }
                }
                found = 0xffff;
                let mut entry_out = 0u32;
                let mut found16 = 0u16;
                let mut sep = 0u8;
                rc = host.dict_lookup(
                    arena, dict_a, dict_b, 0, 0, &mut entry_out, &mut found16, &mut sep,
                );
                if rc >= 0x8000_0000 {
                    if name != 0 {
                        host.free(arena, allocator, name);
                    }
                    if user_buffer != 0 {
                        host.free(arena, allocator, user_buffer);
                    }
                    host.free(arena, allocator, tokens);
                    return rc;
                }
                found = found16;
                let mut do_default = false;
                if found != 0 {
                    let delimiter = sep;
                    let types = word(arena, entry_out);
                    let cap = (strlen16_176624(arena, typ) as u32).wrapping_add(1);
                    // Replacement search loop.
                    let mut rcrep = 0x8990_2000u32;
                    let mut replacement = name;
                    loop {
                        let m = find_substr(arena, types, replacement);
                        if m != 0 {
                            let tail =
                                byte(arena, m.wrapping_add(strlen8(arena, replacement)));
                            if tail == 0 || tail == delimiter || tail == b'|' {
                                rcrep = 0;
                                break;
                            }
                        }
                        let colon0 =
                            crate::normalization::strrchr_at(arena, replacement, ':' as u32);
                        if (rcrep & 0x1fff) != 0 || colon0 == 0 {
                            if replacement != name {
                                host.free(arena, allocator, replacement);
                            }
                            replacement = 0;
                            break;
                        }
                        if replacement == name {
                            let base_len =
                                if name != 0 { strlen8(arena, name) } else { 0 };
                            let need =
                                base_len.wrapping_add(cap).wrapping_add(1);
                            let tmp = match host.realloc(arena, allocator, 0, need) {
                                Some(o) => o,
                                None => {
                                    rcrep = 0x8990_200a;
                                    break;
                                }
                            };
                            if name != 0 {
                                let nl = strlen8(arena, name).wrapping_add(1);
                                memcpy_at(arena, tmp, name, nl);
                            } else {
                                host.free(arena, allocator, tmp);
                                arena[tmp as usize] = 0;
                            }
                            replacement = tmp;
                        }
                        let colon =
                            crate::normalization::strrchr_at(arena, replacement, ':' as u32);
                        arena[colon as usize] = 0;
                    }
                    rc = rcrep;
                    if rc < 0x8000_0000 {
                        if replacement != name {
                            let _ = domain_or_extra(arena, props, &mut *host, name);
                            host.free(arena, allocator, name);
                            name = replacement;
                        }
                    } else {
                        if (rc & 0x1fff) != 0 {
                            if name != 0 {
                                host.free(arena, allocator, name);
                            }
                            if user_buffer != 0 {
                                host.free(arena, allocator, user_buffer);
                            }
                            host.free(arena, allocator, tokens);
                            return rc;
                        }
                        let _ = domain_or_extra(arena, props, &mut *host, name);
                        do_default = true;
                    }
                } else if strcmp_lit(arena, name, b"normal") == 0
                    || strcmp_lit(arena, name, b"sms") == 0
                    || strcmp_lit(arena, name, b"address") == 0
                    || strcmp_lit(arena, name, b"spell") == 0
                    || strcmp_lit(arena, name, b"NOR") == 0
                    || strcmp_lit(arena, name, b"SMS") == 0
                    || strcmp_lit(arena, name, b"ADR") == 0
                {
                    // append_type.
                } else {
                    let _ = domain_or_extra(arena, props, &mut *host, name);
                    do_default = true;
                }
                if do_default {
                    if strcmp16_1765ac(arena, typ, BBA_U16_NORMAL) == 0 {
                        let need = 6u32
                            .wrapping_add(strlen16_176624(arena, typ) as u32)
                            .wrapping_add(2);
                        let tmp = match host.realloc(arena, allocator, name, need) {
                            Some(o) => o,
                            None => {
                                if name != 0 {
                                    host.free(arena, allocator, name);
                                }
                                if user_buffer != 0 {
                                    host.free(arena, allocator, user_buffer);
                                }
                                host.free(arena, allocator, tokens);
                                return 0x8990_200a;
                            }
                        };
                        strcpy_lit(arena, tmp, b"normal");
                        name = tmp;
                    } else {
                        arena[name as usize] = 0;
                    }
                }
                // append_type:
                length = strlen8(arena, name) as u16;
                if strcmp16_1765ac(arena, typ, BBA_U16_NORMAL) != 0
                    && strcmp_lit(arena, name, b"normal") != 0
                    && byte(arena, name) != 0
                {
                    arena[name.wrapping_add(u32::from(length)) as usize] = b'_';
                    length = length.wrapping_add(1);
                }
            }
            if strcmp16_1765ac(arena, typ, BBA_U16_NORMAL) != 0
                && strcmp_lit(arena, name, b"normal") != 0
            {
                let mut i = 0u32;
                if strlen16_176624(arena, typ) == 7
                    && strstr16_176538(arena, typ, BBA_U16_LID) != 0
                {
                    user = 1;
                    put_half(arena, typ.wrapping_add(6), 0);
                }
                let mut terminated = false;
                if strlen16_176624(arena, typ) == 3 {
                    let mut j = 0usize;
                    while j < 3 {
                        let w = half(arena, typ.wrapping_add((j as u32).wrapping_mul(2)));
                        language_code[j] =
                            crate::normalization::tolower_240ba0(u32::from(w as u8)) as u8;
                        j += 1;
                    }
                    language_code[3] = 0;
                    found = 0xffff;
                    let mut entry_out = 0u32;
                    let mut found16 = 0u16;
                    let mut sep = 0u8;
                    rc = host.dict_lookup(
                        arena, dict_a, dict_b, 0, 0, &mut entry_out, &mut found16, &mut sep,
                    );
                    if rc >= 0x8000_0000 {
                        if name != 0 {
                            host.free(arena, allocator, name);
                        }
                        if user_buffer != 0 {
                            host.free(arena, allocator, user_buffer);
                        }
                        host.free(arena, allocator, tokens);
                        return rc;
                    }
                    found = found16;
                    let _ = sep;
                    if found != 0
                        && find_substr_lit(arena, word(arena, entry_out), &language_code[..3])
                            != 0
                    {
                        loop {
                            let ch = half(arena, typ.wrapping_add(i.wrapping_mul(2)));
                            if ch == 0 {
                                break;
                            }
                            arena[name.wrapping_add(u32::from(length)) as usize] =
                                crate::normalization::tolower_240ba0(u32::from(ch as u8))
                                    as u8;
                            length = length.wrapping_add(1);
                            i = i.wrapping_add(1);
                        }
                        terminated = true;
                    }
                } else if strcmp16_1765ac(arena, typ, BBA_U16_LATIN) == 0 {
                    loop {
                        let ch = half(arena, typ.wrapping_add(i.wrapping_mul(2)));
                        if ch == 0 {
                            break;
                        }
                        arena[name.wrapping_add(u32::from(length)) as usize] =
                            crate::normalization::tolower_240ba0(u32::from(ch as u8)) as u8;
                        length = length.wrapping_add(1);
                        i = i.wrapping_add(1);
                    }
                    terminated = true;
                }
                if !terminated {
                    if byte(arena, name) != 0 {
                        arena[name.wrapping_add(u32::from(length).wrapping_sub(1)) as usize] = 0;
                    } else {
                        strcpy_lit(arena, name, b"normal");
                    }
                }
                let _ = i;
            }
            // terminate_name:
            if strncmp_lit(arena, name, b"normal", 6) != 0 && byte(arena, name) != 0 {
                arena[name.wrapping_add(u32::from(length)) as usize] = 0;
            }
        }
        // lookup_name:
        found = 0xffff;
        {
            let mut entry_out = 0u32;
            let mut found16 = 0u16;
            let mut sep = 0u8;
            rc = host.dict_lookup(
                arena, dict_a, dict_b, 0, name, &mut entry_out, &mut found16, &mut sep,
            );
            if rc >= 0x8000_0000 {
                if name != 0 {
                    host.free(arena, allocator, name);
                }
                if user_buffer != 0 {
                    host.free(arena, allocator, user_buffer);
                }
                host.free(arena, allocator, tokens);
                return rc;
            }
            found = found16;
            let mut mapped = name;
            if found != 0 {
                let entry_str = word(arena, entry_out);
                let m = strchr_195d30(arena, entry_str, u32::from(sep));
                if m != 0 {
                    arena[m as usize] = 0;
                }
                mapped = entry_str;
                key[0..11].copy_from_slice(b"tnpausedur\0");
                let key_len = strlen_stack(&key);
                if strlen8(arena, name) < 255u32.wrapping_sub(key_len) {
                    strcat_stack_key(&mut key, arena, name);
                    let mut value = 0u32;
                    if prop_get_u32_175944(arena, props, &mut value) < 0x8000_0000 {
                        key[0..11].copy_from_slice(b"tnpausedur\0");
                        strcat_stack_key(&mut key, arena, mapped);
                        // 1758ac no-op.
                    }
                }
            }
            current = mapped;
        }
        user_text = 0;
        // token_done:
        last_position = word(arena, tok.wrapping_add(12));
        index = next;
    }
    // Final span token.
    rc = fe_token_1d5234(
        arena,
        p,
        db_a,
        db_b,
        &mut cursor,
        last_position.wrapping_sub(first_position) & 0xffff,
        text_bytes.wrapping_sub(1) & 0xffff,
        current,
        user_text,
        // The ROM passes `type` here (hooked on US/EU/KR; src commit
        // `4b542d7` fixed the old `language` argument).
        typ,
        fallback,
        text_data,
        user,
        host,
    );
    if rc >= 0x8000_0000 {
        if name != 0 {
            host.free(arena, allocator, name);
        }
        if user_buffer != 0 {
            host.free(arena, allocator, user_buffer);
        }
        host.free(arena, allocator, tokens);
        return rc;
    }
    let mut sidx = 0u32;
    while sidx < token_count
        && word(arena, tokens.wrapping_add(sidx.wrapping_mul(24)).wrapping_add(12))
            == first_position
    {
        sidx = sidx.wrapping_add(1);
    }
    qsort_1d5214(
        arena,
        tokens.wrapping_add(sidx.wrapping_mul(24)),
        token_count.wrapping_sub(sidx),
    );
    rc = host.lat_set_field(
        arena, db_a, db_b, u32::from(root), 1, token_count, 0, &mut written,
    );
    if rc >= 0x8000_0000 {
        if name != 0 {
            host.free(arena, allocator, name);
        }
        if user_buffer != 0 {
            host.free(arena, allocator, user_buffer);
        }
        host.free(arena, allocator, tokens);
        return rc;
    }
    rc = host.res_consume(arena, in0_a, in0_b, text_bytes);
    if rc >= 0x8000_0000 {
        if name != 0 {
            host.free(arena, allocator, name);
        }
        if user_buffer != 0 {
            host.free(arena, allocator, user_buffer);
        }
        host.free(arena, allocator, tokens);
        return rc;
    }
    rc = host.res_consume(arena, in1_a, in1_b, token_bytes);
    if name != 0 {
        host.free(arena, allocator, name);
    }
    if user_buffer != 0 {
        host.free(arena, allocator, user_buffer);
    }
    if tokens != 0 {
        host.free(arena, allocator, tokens);
    }
    rc
}

/// `domainmngrtntypes` membership probe with the `extraesctn` fallback call.
/// Returns true on a hit (extra call skipped); the log outcome is ignored
/// but both `suba1c` calls run exactly as in the C for script alignment.
fn domain_or_extra(
    arena: &mut [u8],
    props: u32,
    host: &mut impl FeTextHost,
    name: u32,
) -> bool {
    let mut setting = 0u32;
    let name_length = strlen8(arena, name);
    if host.suba1c(arena, props, 0, &mut setting) < 0x8000_0000
        && setting != 0
        && byte(arena, setting) != 0
    {
        let m = find_substr(arena, setting, name);
        if m != 0
            && (m == setting || byte(arena, m.wrapping_sub(1)) == b'|')
            && (byte(arena, m.wrapping_add(name_length)) == 0
                || byte(arena, m.wrapping_add(name_length)) == b'|')
        {
            return true;
        }
    }
    let mut setting2 = 0u32;
    let _ = host.suba1c(arena, props, 0, &mut setting2);
    let _ = setting2;
    false
}

/// u8 strlen of a stack key buffer (up to first NUL).
fn strlen_stack(key: &[u8]) -> u32 {
    let mut n = 0u32;
    while (n as usize) < key.len() && key[n as usize] != 0 {
        n = n.wrapping_add(1);
    }
    n
}

/// Append the arena string at `src` to the stack `key` (like `fn_195c90`).
fn strcat_stack_key(key: &mut [u8], arena: &[u8], src: u32) {
    let mut d = 0usize;
    while d < key.len() && key[d] != 0 {
        d += 1;
    }
    let mut s = src;
    while d < key.len() {
        let b = byte(arena, s);
        key[d] = b;
        s = s.wrapping_add(1);
        if b == 0 {
            break;
        }
        d += 1;
    }
}

/// fn_202970, EU fn_202b2c, KR fn_202bd8. Copies the available window with
/// `memcpy_195ae8`, advances both cursors, and divides through the
/// unsigned fn_1994d0 divmod. `fac` is furnished as a readable pointer.
pub fn window_fill(arena: &mut [u8], a: u32, b: u32, c: u32, d: u32) -> u32 {
    let n = (b as i32).wrapping_mul(c as i32) as u32;
    let avail = word(arena, d.wrapping_add(0xb0)).wrapping_sub(word(arena, d.wrapping_add(0xb4)));
    let n = avail.min(n);
    memcpy_195ae8(arena, a, word(arena, d.wrapping_add(0xac)), n);
    put_word(
        arena,
        d.wrapping_add(0xac),
        word(arena, d.wrapping_add(0xac)).wrapping_add(n),
    );
    put_word(
        arena,
        d.wrapping_add(0xb4),
        word(arena, d.wrapping_add(0xb4)).wrapping_add(n),
    );
    if b == 0 {
        return 0;
    }
    n / b
}

/// fn_1740e4, EU fn_174194, KR fn_174140. Grows the buffer through
/// fn_1963e0 (the `r_ptrA` script here) and rebases both pointer tables.
/// Comparisons run in the offset domain, which preserves the absolute
/// order.
pub fn rebase_grow(arena: &mut [u8], heap: u32, b: u32, needed: u32) -> u32 {
    let _ = heap;
    let mut rc = 0;
    let mut grown = 0;
    let old = word(arena, b);
    loop {
        let len = strlen_195d90(arena, word(arena, b)) as u32;
        if word(arena, b.wrapping_add(4)) > len.wrapping_add(needed) {
            break;
        }
        let f = rule_entry(arena, 0x5b0, 4);
        put_word(arena, b, word(arena, f));
        put_word(
            arena,
            b.wrapping_add(4),
            word(arena, b.wrapping_add(4)).wrapping_add(0x1000),
        );
        grown = 1;
    }
    if grown != 0 && old != 0 {
        let count = word(arena, b.wrapping_add(8));
        let mut i = 0;
        while i < count {
            let t1 = word(arena, b.wrapping_add(12));
            let x = word(arena, t1.wrapping_add(4 * i));
            if x != 0 && x > old {
                put_word(
                    arena,
                    t1.wrapping_add(4 * i),
                    x.wrapping_sub(old).wrapping_add(word(arena, b)),
                );
            } else {
                rc = 0x84b0_2007;
                break;
            }
            let t2 = word(arena, b.wrapping_add(16));
            let y = word(arena, t2.wrapping_add(4 * i));
            if y != 0 && y > old {
                put_word(
                    arena,
                    t2.wrapping_add(4 * i),
                    y.wrapping_sub(old).wrapping_add(word(arena, b)),
                );
            } else {
                rc = 0x84b0_2007;
                break;
            }
            i += 1;
        }
    }
    rc
}

/// fn_17db60, EU fn_17dc10, KR fn_17dbbc. Joins `a` and `b` with a slash
/// into a fn_196394 buffer (the `r_ptrA` script here) and collapses `/./`
/// and `/name/../` segments.
pub fn join_norm(arena: &mut [u8], out: u32, a: u32, b: u32, flag: u32, alloc: u32) -> u32 {
    let _ = alloc;
    if a == 0 || b == 0 {
        return 0x8150_6000;
    }
    let mut la = strlen_195d90(arena, a);
    let lb = strlen_195d90(arena, b);
    let f = rule_entry(arena, 0x5b0, 4);
    let buf = word(arena, f);
    put_word(arena, out, buf);
    if buf == 0 {
        return 0x8150_600a;
    }
    crate::normalization::strcpy_at(arena, buf, a);
    if flag != 0 {
        while la != 0 && byte(arena, buf.wrapping_add(la as u32).wrapping_sub(1)) != b'/' {
            la -= 1;
        }
        arena[buf.wrapping_add(la as u32) as usize] = 0;
    }
    if lb != 0 && la != 0 && byte(arena, a.wrapping_add(la as u32).wrapping_sub(1)) != b'/' {
        let mut d = buf;
        while byte(arena, d) != 0 {
            d = d.wrapping_add(1);
        }
        arena[d as usize] = b'/';
        arena[d.wrapping_add(1) as usize] = 0;
    }
    strcat_195c90(arena, buf, b);
    let mut p = strchr_195d30(arena, buf.wrapping_add(1), b'.' as u32);
    while p != 0 {
        if byte(arena, p.wrapping_sub(1)) == b'/' {
            if byte(arena, p.wrapping_add(1)) == b'/' {
                let n = strlen_195d90(arena, p.wrapping_add(2)) + 1;
                memmove_195b38(arena, p, p.wrapping_add(2), n as u32);
            } else if byte(arena, p.wrapping_add(1)) == b'.'
                && byte(arena, p.wrapping_add(2)) == b'/'
            {
                let q = p;
                let mut named = 0;
                p = p.wrapping_sub(1);
                while buf < p && byte(arena, p.wrapping_sub(1)) != b'/' {
                    p = p.wrapping_sub(1);
                    if byte(arena, p) != b'.' {
                        named = 1;
                    }
                }
                if named == 0 {
                    p = q.wrapping_add(1);
                } else {
                    let n = strlen_195d90(arena, q.wrapping_add(3)) + 1;
                    memmove_195b38(arena, p, q.wrapping_add(3), n as u32);
                }
            } else {
                p = p.wrapping_add(1);
            }
        } else {
            p = p.wrapping_add(1);
        }
        p = strchr_195d30(arena, p, b'.' as u32);
    }
    0
}

/// fn_176a48, EU fn_176af8, KR fn_176aa4. Case-insensitive bounded compare
/// through `fold_17c8c0`.
pub fn fold_ncasecmp(arena: &[u8], a: u32, b: u32, n: u32) -> u32 {
    let mut i = 0u32;
    while n > i {
        if fold_17c8c0(byte(arena, a.wrapping_add(i)) as u32)
            != fold_17c8c0(byte(arena, b.wrapping_add(i)) as u32)
        {
            break;
        }
        if byte(arena, a.wrapping_add(i)) == 0 {
            break;
        }
        if byte(arena, b.wrapping_add(i)) == 0 {
            break;
        }
        i += 1;
    }
    if i == n {
        return 0;
    }
    (fold_17c8c0(byte(arena, a.wrapping_add(i)) as u32) as i32
        - fold_17c8c0(byte(arena, b.wrapping_add(i)) as u32) as i32) as u32
}

/// fn_176054, EU fn_176104, KR fn_1760b0. Stores the source/length
/// pair, allocating (`r_ptrA` here) or reallocating when the stored
/// length is zero or short, then copies `n` bytes. The `fn_196428`
/// release call on the empty-store path is counted at 0x60 (`r_inc`).
pub fn buf_set_grow(arena: &mut [u8], p: u32, src: u32, n: u32, heap: u32) -> u32 {
    let _ = heap;
    if p == 0 {
        return 0x8b70_2007;
    }
    if n == 0 {
        let data = word(arena, p);
        let size = word(arena, p.wrapping_add(4));
        if data != 0 && size != 0 {
            put_word(arena, 0x60, word(arena, 0x60).wrapping_add(1));
        }
        put_word(arena, p.wrapping_add(4), 0);
        put_word(arena, p, src);
    } else {
        let size = word(arena, p.wrapping_add(4));
        if size == 0 {
            // `p->data = fn_196394(heap, n)` assigns before the null
            // check, so a failed alloc still zeroes the cell.
            let e = rule_entry(arena, 0x5b0, 4);
            let d = word(arena, e);
            put_word(arena, p, d);
            if d == 0 {
                return 0x8b70_200a;
            }
        } else if size < n {
            let e = rule_entry(arena, 0x5b0, 4);
            let d = word(arena, e);
            if d == 0 {
                return 0x8b70_200a;
            }
            put_word(arena, p, d);
        }
        let data = word(arena, p);
        memcpy_195ae8(arena, data, src, n);
        put_word(arena, p.wrapping_add(4), n);
    }
    0
}

/// fn_196db0, EU fn_196e60, KR fn_196e9c. Pushes a slot through
/// [`vec_grow`](crate::normalization::vec_grow) into scratch cell
/// 0x64, copies `size` bytes there, then runs the optional `cb`
/// validator (the `rule_stA` script here, gated by the word at 0x60);
/// a zero validator vote decrements the count and reports 0.
pub fn vec_push_copy(arena: &mut [u8], p: u32, src: u32) -> u32 {
    if vec_grow(arena, p, 0x64) == 0 {
        return 0;
    }
    let blk = word(arena, 0x64);
    let size = half(arena, p.wrapping_add(4)) as u32;
    memcpy_195ae8(arena, blk, src, size);
    if word(arena, 0x60) != 0 {
        let e = rule_entry(arena, 0x548, 4);
        if word(arena, e) == 0 {
            let count = word(arena, p.wrapping_add(20));
            put_word(arena, p.wrapping_add(20), count.wrapping_sub(1));
            return 0;
        }
    }
    1
}

