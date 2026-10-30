//! Text-normalization and tokenization leaves reached by the enu corpus.
//!
//! These are the small, self-contained members of slice 2's normalization
//! section. Pointer fields are byte offsets into the caller's borrowed arena,
//! with 0 meaning null, matching the convention already used by `depes`.

use crate::depes::HandleValidator;

fn byte(data: &[u8], offset: u32) -> u8 {
    data.get(offset as usize).copied().unwrap_or(0)
}

fn half(data: &[u8], offset: u32) -> u16 {
    byte(data, offset) as u16 | (byte(data, offset.wrapping_add(1)) as u16) << 8
}

fn signed_half(data: &[u8], offset: u32) -> i16 {
    half(data, offset) as i16
}

fn word(data: &[u8], offset: u32) -> u32 {
    half(data, offset) as u32 | (half(data, offset.wrapping_add(2)) as u32) << 16
}

fn put_half(data: &mut [u8], offset: u32, value: u16) {
    if let Some(slot) = data.get_mut(offset as usize) {
        *slot = value as u8;
    }
    if let Some(slot) = data.get_mut(offset.wrapping_add(1) as usize) {
        *slot = (value >> 8) as u8;
    }
}

fn put_word(data: &mut [u8], offset: u32, value: u32) {
    put_half(data, offset, value as u16);
    put_half(data, offset.wrapping_add(2), (value >> 16) as u16);
}

/// fn_24ce90, EU fn_24d04c, KR fn_24d0f8. Clears the two 16-bit marker words.
pub fn clear_markers(object: &mut [u8]) {
    put_half(object, 0xbc, 0);
    put_half(object, 0xc4, 0);
}

/// fn_24d8b8, EU fn_24da74, KR fn_24db20. Stores `value` at `items[index]`.
pub fn set_item(items: &mut [i32], index: u32, value: i32) -> i32 {
    if let Some(slot) = items.get_mut(index as usize) {
        *slot = value;
    }
    0
}

/// fn_24d5e4, EU fn_24d7a0, KR fn_24d84c. Unsigned byte comparison.
pub fn byte_compare_at(bytes: &[u8], a: u32, b: u32) -> u32 {
    compare_byte(bytes[a as usize], bytes[b as usize]) as u32
}

/// fn_24d5e4, EU fn_24d7a0, KR fn_24d84c. Unsigned byte comparison.
pub fn compare_byte(a: u8, b: u8) -> i32 {
    let (x, y) = (a as u32, b as u32);
    if x < y {
        -1
    } else if x == y {
        0
    } else {
        1
    }
}

/// fn_252b0c, EU fn_252cc8, KR fn_252d74. Reads the +0x0c field.
pub fn field_value(object: Option<&[u8]>, out: Option<&mut u32>) -> u32 {
    match (object, out) {
        (Some(object), Some(out)) => {
            *out = word(object, 0xc);
            0
        }
        _ => 0x81102007,
    }
}

/// fn_252a74, EU fn_252c30, KR fn_252cdc. Reads `inner->val` at +0x08.
pub fn inner_field(inner: Option<&[u8]>, out: Option<&mut u16>) -> u32 {
    match (inner, out) {
        (Some(inner), Some(out)) => {
            *out = half(inner, 8);
            0
        }
        _ => 0x81102007,
    }
}

/// fn_252a94, EU fn_252c50, KR fn_252cfc. Returns the current index when the
/// table still has an entry.
pub fn current_index(table: Option<&[u8]>, index: u16, out: Option<&mut u16>) -> u32 {
    match (table, out) {
        (Some(table), Some(out)) => {
            if half(table, 8) > index {
                *out = index;
                0
            } else {
                0x8110200f
            }
        }
        _ => 0x81102007,
    }
}

/// fn_2527d0, EU fn_25298c, KR fn_252a38. Clamps and stores an index.
pub fn set_index(count: u16, index: i32, out: &mut u16) -> u32 {
    let resolved = if index >= 0 {
        if (count as i32) < index {
            return 0x8110200f;
        }
        index
    } else {
        let value = index + count as i32;
        if value < 0 {
            return 0x8110200f;
        }
        value
    };
    *out = resolved as u16;
    0
}

/// fn_252ac4, EU fn_252c80, KR fn_252d2c. Byte offset of a table entry.
pub fn entry_offset(
    size: u16,
    base: i32,
    count: u16,
    index: u16,
    requested: u32,
    out: &mut i32,
) -> u32 {
    let requested = if requested == 0xffff {
        index as u32
    } else {
        requested
    };
    if count as u32 <= requested {
        return 0x8110200f;
    }
    *out = requested
        .wrapping_mul(size as u32)
        .wrapping_add(base as u32) as i32;
    0
}

/// fn_2b9380, EU fn_2b953c, KR fn_2b95e8. Ordered key comparison (high, low).
pub fn key_le(a_hi: u32, a_lo: u32, b_hi: u32, b_lo: u32) -> i32 {
    if a_hi == b_hi {
        (a_lo <= b_lo) as i32
    } else {
        (a_hi <= b_hi) as i32
    }
}

/// fn_2b93c8, EU fn_2b9584, KR fn_2b9630. Membership in a 16-bit list.
pub fn id_in_list(list: &[u16], id: u16) -> i32 {
    let mut index = 0;
    while list.get(index).copied().unwrap_or(0) != 0 && id != list[index] {
        index += 1;
    }
    (id == list.get(index).copied().unwrap_or(0)) as i32
}

/// fn_24e698, EU fn_24e854, KR fn_24e900. Sorted-list lookup with a default.
pub fn lookup_default(
    arena: &[u8],
    head: u32,
    count: u16,
    key: i16,
    default: u16,
    out: &mut u16,
) -> i32 {
    if count != 0 {
        let mut node = head;
        *out = default;
        while node != 0 {
            let node_key = signed_half(arena, node.wrapping_add(8));
            if key == node_key {
                *out = half(arena, node.wrapping_add(10));
            } else if node_key > key {
                break;
            }
            node = word(arena, node);
        }
    }
    0
}

/// fn_24e948, EU fn_24eb04, KR fn_24ebb0. Finds an entry and reports the
/// addresses of its key and value words as arena offsets.
pub fn find_entry(
    arena: &[u8],
    head: u32,
    count: u16,
    key: i32,
    key_out: &mut u32,
    value_out: &mut u32,
) -> i32 {
    if count != 0 && head != 0 {
        let mut node = head;
        loop {
            if signed_half(arena, node.wrapping_add(8)) as i32 == key {
                *key_out = node.wrapping_add(8);
                *value_out = node.wrapping_add(12);
                return 0;
            }
            if signed_half(arena, node.wrapping_add(8)) as i32 > key {
                return 0;
            }
            node = word(arena, node);
            if node == 0 {
                return 0;
            }
        }
    }
    0
}

fn unlink_node(arena: &mut [u8], list_head: u32, node: u32) {
    let next = word(arena, node);
    let prev = word(arena, node.wrapping_add(4));
    if prev != 0 {
        put_word(arena, prev, next);
    } else {
        if next != 0 {
            put_word(arena, next.wrapping_add(4), prev);
        }
        put_word(arena, list_head, next);
    }
    if next != 0 {
        put_word(arena, next.wrapping_add(4), prev);
    }
}

/// fn_24e8a0, EU fn_24ea5c, KR fn_24eb08. Removes the first entry matching
/// key, sub-key and tag.
pub fn remove_entry(
    arena: &mut [u8],
    list: u32,
    head: u32,
    count: i16,
    key: i16,
    sub: i16,
    tag: i32,
) -> i32 {
    let mut node = if count != 0 { head } else { 0 };
    if count != 0 && node != 0 {
        while node != 0 {
            if signed_half(arena, node.wrapping_add(8)) == key
                && signed_half(arena, node.wrapping_add(10)) == sub
                && word(arena, node.wrapping_add(0x60)) as i32 == tag
            {
                unlink_node(arena, list.wrapping_add(4), node);
                return 0;
            }
            if signed_half(arena, node.wrapping_add(8)) > key {
                return 0;
            }
            node = word(arena, node);
        }
    }
    0
}

fn unlink_range(arena: &mut [u8], list: u32, node: u32) -> u32 {
    let next = word(arena, node);
    let prev = word(arena, node.wrapping_add(4));
    if prev != 0 {
        put_word(arena, prev, next);
    } else {
        if next != 0 {
            put_word(arena, next.wrapping_add(4), prev);
        }
        put_word(arena, list.wrapping_add(4), next);
    }
    if next != 0 {
        put_word(
            arena,
            next.wrapping_add(4),
            word(arena, node.wrapping_add(4)),
        );
    }
    next
}

/// fn_24e7c4, EU fn_24e980, KR fn_24ea2c. Removes entries in a key range.
pub fn remove_range(
    arena: &mut [u8],
    list: u32,
    count: u16,
    head: u32,
    key: u16,
    bound: i32,
) -> u32 {
    let want = key as i32;
    let mut node = if count != 0 { head } else { 0 };
    if count != 0 && node != 0 {
        loop {
            if node == 0 {
                break;
            }
            let node_key = signed_half(arena, node.wrapping_add(8)) as i32;
            let node_bound = signed_half(arena, node.wrapping_add(10)) as i32;
            if node_key == want && node_bound == bound {
                node = word(arena, node);
                continue;
            }
            if node_key >= want {
                if node_bound <= bound {
                    node = unlink_range(arena, list, node);
                } else if bound <= node_key {
                    node = word(arena, node);
                } else {
                    node = unlink_range(arena, list, node);
                }
                continue;
            }
            if bound < node_key {
                return 0;
            }
            node = word(arena, node);
        }
    }
    0
}

/// fn_252818, EU fn_2529d4, KR fn_252a80. Scans forward until the callback
/// accepts an entry or the table is exhausted. The callback receives the
/// element-to-end slice, matching the C pointer it can read past the stride.
pub fn scan_forward(
    base: &[u8],
    stride: u16,
    count: u16,
    index: &mut u16,
    accept: &mut impl FnMut(&[u8]) -> i32,
) -> u32 {
    let mut offset = *index as usize * stride as usize;
    while *index < count && accept(&base[offset..]) == 0 {
        *index += 1;
        offset += stride as usize;
    }
    if *index >= count {
        0x81102014
    } else {
        0
    }
}

/// fn_2528a0, EU fn_252a5c, KR fn_252b08. Scans backward until the callback
/// accepts an entry.
pub fn scan_backward(
    base: &[u8],
    stride: u16,
    count: u16,
    index: &mut u16,
    accept: &mut impl FnMut(&[u8]) -> i32,
) -> u32 {
    let mut current = *index;
    if count <= current {
        current = count.wrapping_sub(1);
    }
    let mut offset = current as usize * stride as usize;
    while current != 0 && accept(&base[offset..offset + stride as usize]) == 0 {
        current = current.wrapping_sub(1);
        offset = offset.wrapping_sub(stride as usize);
    }
    *index = current;
    if current != 0 {
        return 0;
    }
    if accept(&base[offset..offset + stride as usize]) != 0 {
        return 0;
    }
    0x81102014
}

/// fn_1dbc6c, EU fn_1dbe28, KR fn_1dbed4. Counts the length prefixes whose
/// running sum still fits under `limit`, stopping before the first that
/// overflows.
pub fn count_fitting(limit: u16, count: u16, lens: &[u8]) -> u16 {
    let mut index = 0u16;
    let mut sum = 0u16;
    while index < count {
        sum = sum.wrapping_add(lens.get(index as usize).copied().unwrap_or(0) as u16);
        if sum > limit {
            break;
        }
        index += 1;
    }
    index
}

/// fn_1d8ec8, EU fn_1d9084, KR fn_1d9130. Orders two token records: same-group
/// tokens sort by position, except a type-1/type-8 pair, which separates to
/// +/-100.
pub fn token_order(a: &[u8], b: &[u8]) -> i32 {
    let (a_kind, b_kind) = (half(a, 0), half(b, 0));
    let (a_group, b_group) = (word(a, 0xc) as i32, word(b, 0xc) as i32);
    if a_group == b_group {
        if a_kind == 1 && b_kind == 8 {
            return 100;
        }
        if a_kind == 8 && b_kind == 1 {
            return -100;
        }
        return (word(a, 4) as i32).wrapping_sub(word(b, 4) as i32);
    }
    a_group.wrapping_sub(b_group)
}

/// fn_1d5214, EU fn_1d53d0, KR fn_1d547c. Orders two 24-byte token records by
/// `token` at +12, breaking ties by `position` at +4.
pub fn token_position_order(a: &[u8], b: &[u8]) -> i32 {
    let (a_token, b_token) = (word(a, 12) as i32, word(b, 12) as i32);
    if a_token == b_token {
        (word(a, 4) as i32).wrapping_sub(word(b, 4) as i32)
    } else {
        a_token.wrapping_sub(b_token)
    }
}

/// fn_1e7748, EU fn_1e7904, KR fn_1e79b0. Advances `start` to the next non-zero
/// byte and `end` over that run plus its trailing zero padding.
pub fn next_run(start: &mut u16, end: &mut u16, buffer: &[u8], length: u32) -> i32 {
    *end = *start;
    while (*start as u32) < length {
        if byte(buffer, *start as u32) != 0 {
            *end = end.wrapping_add(1);
            while (*end as u32) < length && byte(buffer, *end as u32) != 0 {
                *end = end.wrapping_add(1);
            }
            while (*end as u32) + 1 < length && byte(buffer, (*end as u32) + 1) == 0 {
                *end = end.wrapping_add(1);
            }
            return 1;
        }
        *start = start.wrapping_add(1);
    }
    0
}

/// fn_284ba8, EU fn_284d64, KR fn_284e10. Unsigned byte less-than.
pub fn byte_less(a: u8, b: u8) -> i32 {
    (a < b) as i32
}

/// fn_1d8eac, EU fn_1d9068, KR fn_1d9114. Validates an FE_HMOGRPH handle and
/// remaps a failure to the component's own status.
pub fn check_object(handle: u32, host: &mut impl HandleValidator) -> u32 {
    let status = host.validate(handle, 0xf383, 56);
    if status >= 0x8000_0000 {
        0x89c02008
    } else {
        status
    }
}

/// fn_1e772c, EU fn_1e78e8, KR fn_1e7994. Validates an FE_NORMOUT handle and
/// remaps a failure to the component's own status.
pub fn check_normalizer(handle: u32, host: &mut impl HandleValidator) -> u32 {
    let status = host.validate(handle, 0xf3b3, 0x3c);
    if status >= 0x8000_0000 {
        0x8b802008
    } else {
        status
    }
}

/// Host operations the normalization and tokenization trees need from the
/// platform layer (`fn_1963b8`, `fn_196394`, `fn_1963e0`, `fn_196428`,
/// `fn_19611c`). All pointers are arena offsets with 0 meaning null, matching
/// the convention used throughout this module. Diagnostic logging
/// (`fn_1738e8`, `fn_173dc0`) has no observable effect on any path ported here
/// and is not modelled.
pub trait NormalizationHost {
    /// `fn_1963b8`: `count` items of `size` bytes, zeroed. `None` on refusal.
    fn calloc(&mut self, arena: &mut [u8], heap: u32, count: u32, size: u32) -> Option<u32>;
    /// `fn_196394`: `size` bytes. `None` on refusal.
    fn alloc(&mut self, arena: &mut [u8], heap: u32, size: u32) -> Option<u32>;
    /// `fn_1963e0`: resize `ptr` to `size` bytes. `None` on refusal.
    fn realloc(&mut self, arena: &mut [u8], heap: u32, ptr: u32, size: u32) -> Option<u32>;
    /// `fn_196428`: release `ptr`.
    fn free(&mut self, arena: &mut [u8], heap: u32, ptr: u32);
    /// `fn_19611c`: resolve the context for the `(h0, h1)` handle pair.
    /// Returns `(status, context)`, with the context as an arena offset.
    fn resolve(&mut self, arena: &mut [u8], h0: u32, h1: u32) -> (u32, u32);
}

/// fn_1e3eec, EU fn_1e40a8, KR fn_1e4154. Tags a diagnostic code with the
/// tokenizer facility.
pub fn error_code(code: u32) -> u32 {
    code | 0x8960_2000
}

/// fn_24ee58, EU fn_24f014, KR fn_24f0c0. Forwards to `fn_1e3eec`.
pub fn forward_error(code: u32) -> u32 {
    error_code(code)
}

/// fn_1994d0. Unsigned division. The original jumps into the shared
/// `fn_30209c` divide loop, so the quotient matches a plain division.
/// The divisor must be nonzero: zero goes to the firmware's divide-error
/// handler instead of returning.
pub(crate) fn udiv(value: u32, divisor: u32) -> u32 {
    value / divisor
}

/// fn_195d90. Length of a NUL-terminated byte string.
pub(crate) fn strlen8(data: &[u8], base: u32) -> u32 {
    let mut length = 0u32;
    while byte(data, base.wrapping_add(length)) != 0 {
        length = length.wrapping_add(1);
    }
    length
}

/// fn_195c90. Appends the NUL-terminated string at `src` to the one at `dst`.
/// Both live in the same arena.
pub(crate) fn strcat_at(arena: &mut [u8], dst: u32, src: u32) {
    let mut d = dst;
    while byte(arena, d) != 0 {
        d = d.wrapping_add(1);
    }
    let mut s = src;
    loop {
        let c = byte(arena, s);
        if let Some(slot) = arena.get_mut(d as usize) {
            *slot = c;
        }
        if c == 0 {
            break;
        }
        d = d.wrapping_add(1);
        s = s.wrapping_add(1);
    }
}

/// fn_195dac. Appends at most `n` bytes from `src` to the string at `dst`,
/// stopping early after copying a NUL and always NUL-terminating otherwise.
pub(crate) fn strnappend(arena: &mut [u8], dst: u32, src: u32, n: u32) {
    let mut d = dst;
    while byte(arena, d) != 0 {
        d = d.wrapping_add(1);
    }
    let mut remaining = n;
    let mut s = src;
    while remaining != 0 {
        remaining = remaining.wrapping_sub(1);
        let c = byte(arena, s);
        if let Some(slot) = arena.get_mut(d as usize) {
            *slot = c;
        }
        d = d.wrapping_add(1);
        s = s.wrapping_add(1);
        if c == 0 {
            return;
        }
    }
    if let Some(slot) = arena.get_mut(d as usize) {
        *slot = 0;
    }
}

/// fn_195ae8. Copies `n` bytes from `src` to `dst` (memcpy).
pub(crate) fn memcpy_at(arena: &mut [u8], dst: u32, src: u32, n: u32) {
    let mut index = 0u32;
    while index != n {
        let c = byte(arena, src.wrapping_add(index));
        if let Some(slot) = arena.get_mut(dst.wrapping_add(index) as usize) {
            *slot = c;
        }
        index = index.wrapping_add(1);
    }
}

/// fn_195b38. Copies `n` bytes handling overlap (memmove).
pub(crate) fn memmove_at(arena: &mut [u8], dst: u32, src: u32, n: u32) {
    if dst > src && src.wrapping_add(n) > dst {
        let mut remaining = n;
        while remaining != 0 {
            remaining = remaining.wrapping_sub(1);
            let c = byte(arena, src.wrapping_add(remaining));
            if let Some(slot) = arena.get_mut(dst.wrapping_add(remaining) as usize) {
                *slot = c;
            }
        }
    } else {
        memcpy_at(arena, dst, src, n);
    }
}

/// fn_195bec. Fills `n` bytes at `dst` with `value` (memset).
pub(crate) fn memset_at(arena: &mut [u8], dst: u32, value: u8, n: u32) {
    let mut index = 0u32;
    while index != n {
        if let Some(slot) = arena.get_mut(dst.wrapping_add(index) as usize) {
            *slot = value;
        }
        index = index.wrapping_add(1);
    }
}

/// fn_195aac. Compares `n` bytes (memcmp).
pub(crate) fn memcmp_at(arena: &[u8], a: u32, b: u32, n: u32) -> i32 {
    if n == 0 {
        return 0;
    }
    let mut index = 0u32;
    let mut remaining = n;
    while {
        remaining = remaining.wrapping_sub(1);
        remaining != 0
    } && byte(arena, a.wrapping_add(index)) == byte(arena, b.wrapping_add(index))
    {
        index = index.wrapping_add(1);
    }
    byte(arena, a.wrapping_add(index)) as i32 - byte(arena, b.wrapping_add(index)) as i32
}

/// fn_176514. Finds `c` in a NUL-terminated u16 string. Returns the byte
/// offset of the match, or `None`.
pub(crate) fn u16chr(data: &[u8], base: u32, c: u16) -> Option<u32> {
    let mut offset = base;
    while half(data, offset) != 0 && half(data, offset) != c {
        offset = offset.wrapping_add(2);
    }
    if half(data, offset) != c {
        return None;
    }
    Some(offset)
}

/// fn_176624. Length of a NUL-terminated u16 string.
pub(crate) fn u16len(data: &[u8], base: u32) -> u32 {
    let mut length = 0u32;
    while half(data, base.wrapping_add(length * 2)) != 0 {
        length = length.wrapping_add(1);
    }
    length
}

/// fn_1765ac. u16-string comparison. Compares while `a` has not ended and the
/// pair matches, then returns the difference at the stop point.
pub(crate) fn u16cmp(data: &[u8], a: u32, b: u32) -> i32 {
    let mut aa = a;
    let mut bb = b;
    loop {
        let c = half(data, aa) as u32;
        if c == 0 {
            break;
        }
        if c != half(data, bb) as u32 {
            break;
        }
        aa = aa.wrapping_add(2);
        bb = bb.wrapping_add(2);
    }
    half(data, aa) as i32 - half(data, bb) as i32
}

/// fn_176460. Narrows a NUL-terminated u16 string to bytes, replacing any
/// unit above 0xff with `?`.
pub(crate) fn narrow_at(arena: &mut [u8], dst: u32, src: u32) {
    let mut d = dst;
    let mut s = src;
    loop {
        let c = half(arena, s);
        if c == 0 {
            break;
        }
        if let Some(slot) = arena.get_mut(d as usize) {
            *slot = if c & 0xff00 != 0 { b'?' } else { c as u8 };
        }
        d = d.wrapping_add(1);
        s = s.wrapping_add(2);
    }
    if let Some(slot) = arena.get_mut(d as usize) {
        *slot = 0;
    }
}

/// fn_1960a8. Finds the NUL-terminated `needle` in `haystack`, returning the
/// byte offset of the match, or `None`. An empty needle matches at the start.
pub(crate) fn strstr_at(data: &[u8], haystack: u32, needle: u32) -> Option<u32> {
    if byte(data, needle) == 0 {
        return Some(haystack);
    }
    let mut h = haystack;
    while byte(data, h) != 0 {
        let mut ph = h;
        let mut pn = needle;
        while byte(data, ph) != 0 && byte(data, pn) != 0 && byte(data, ph) == byte(data, pn) {
            ph = ph.wrapping_add(1);
            pn = pn.wrapping_add(1);
        }
        if byte(data, pn) == 0 {
            return Some(h);
        }
        h = h.wrapping_add(1);
    }
    None
}

/// fn_1970ac. Returns the `idx`-th fixed-size row of the table at `table`.
pub(crate) fn table_row(data: &[u8], table: u32, idx: u32) -> Option<u32> {
    if table == 0 || idx >= word(data, table.wrapping_add(20)) {
        return None;
    }
    Some(word(data, table).wrapping_add(idx.wrapping_mul(half(data, table.wrapping_add(4)) as u32)))
}

/// fn_24d030, EU fn_24d1ec, KR fn_24d298. Releases a heap buffer and clears
/// its length words.
pub fn release_buffer(arena: &mut [u8], owner: u32, buf: u32, host: &mut impl NormalizationHost) {
    if half(arena, buf.wrapping_add(4)) != 0 && word(arena, buf) != 0 {
        host.free(arena, word(arena, owner.wrapping_add(4)), word(arena, buf));
        put_word(arena, buf, 0);
    }
    put_half(arena, buf.wrapping_add(4), 0);
    put_half(arena, buf.wrapping_add(6), 0);
}

fn release_owned_at(arena: &mut [u8], obj: u32, buf_field: u32, host: &mut impl NormalizationHost) {
    let buf = word(arena, obj.wrapping_add(buf_field));
    if buf != 0 {
        let owner = word(arena, obj);
        host.free(arena, word(arena, owner.wrapping_add(4)), buf);
        put_word(arena, obj.wrapping_add(buf_field), 0);
    }
    put_half(arena, obj.wrapping_add(buf_field).wrapping_add(4), 0);
    put_half(arena, obj.wrapping_add(buf_field).wrapping_add(6), 0);
}

/// fn_24cbd4, EU fn_24cd90, KR fn_24ce3c. Releases the +0x78 buffer and clears
/// its count words.
pub fn release_owned(arena: &mut [u8], obj: u32, host: &mut impl NormalizationHost) {
    release_owned_at(arena, obj, 0x78, host);
}

/// fn_24ccf0, EU fn_24ceac, KR fn_24cf58. Releases the +0x80 buffer and clears
/// its count words.
pub fn release_owned_second(arena: &mut [u8], obj: u32, host: &mut impl NormalizationHost) {
    release_owned_at(arena, obj, 0x80, host);
}

/// fn_24ce40, EU fn_24cffc, KR fn_24d0a8. Releases both heap buffers and
/// clears their length words.
pub fn release_pair(arena: &mut [u8], obj: u32, host: &mut impl NormalizationHost) {
    let heap = word(arena, word(arena, obj).wrapping_add(4));
    if word(arena, obj.wrapping_add(0xc0)) != 0 {
        host.free(arena, heap, word(arena, obj.wrapping_add(0xc0)));
        put_word(arena, obj.wrapping_add(0xc0), 0);
    }
    put_half(arena, obj.wrapping_add(0xc4), 0);
    if word(arena, obj.wrapping_add(0xb8)) != 0 {
        host.free(arena, heap, word(arena, obj.wrapping_add(0xb8)));
        put_word(arena, obj.wrapping_add(0xb8), 0);
    }
    put_half(arena, obj.wrapping_add(0xbc), 0);
}

/// fn_24e16c, EU fn_24e328, KR fn_24e3d4. Reads the item's +0x08 value, or
/// zero when no item is selected.
pub fn item_value(arena: &[u8], obj: u32) -> u16 {
    let item = word(arena, obj.wrapping_add(0x44));
    if item == 0 {
        return 0;
    }
    half(arena, item.wrapping_add(8))
}

/// fn_250c90, EU fn_250e4c, KR fn_250ef8. Reads the `idx`-th domain value.
pub fn domain_value(arena: &[u8], obj: u32, idx: u32, out: &mut u32) -> u32 {
    let inner = word(arena, obj.wrapping_add(0x38));
    let list = word(arena, inner.wrapping_add(8));
    if list != 0 {
        if idx < half(arena, list) as u32 {
            let items = word(arena, list.wrapping_add(4));
            let entry = word(arena, items.wrapping_add(idx.wrapping_mul(4)));
            *out = word(arena, entry.wrapping_add(8));
            return 0;
        }
        return forward_error(25);
    }
    forward_error(7)
}

/// fn_2510a8, EU fn_251264, KR fn_251310. Reads one table entry's value and
/// length words.
pub fn table_entry_values(
    arena: &[u8],
    obj: u32,
    idx: u32,
    out: &mut u32,
    out_len: Option<&mut u16>,
    out_ptr: &mut u32,
) -> u32 {
    let inner = word(arena, obj.wrapping_add(0x38));
    let list = word(arena, inner.wrapping_add(8));
    if list == 0 {
        return forward_error(7);
    }
    let groups = word(arena, list.wrapping_add(4));
    let group = word(
        arena,
        groups.wrapping_add((half(arena, idx.wrapping_add(2)) as u32).wrapping_mul(4)),
    );
    let table = word(arena, group.wrapping_add(4));
    let entries = word(arena, table.wrapping_add(0x18));
    let entry = entries.wrapping_add((half(arena, idx.wrapping_add(4)) as u32).wrapping_mul(0x18));
    *out =
        word(arena, table.wrapping_add(12)).wrapping_add(half(arena, entry.wrapping_add(2)) as u32);
    if let Some(out_len) = out_len {
        *out_len = half(arena, entry.wrapping_add(0x10));
    }
    *out_ptr = if half(arena, entry.wrapping_add(0x10)) == 0 {
        0
    } else {
        word(arena, entry.wrapping_add(8))
    };
    0
}

/// fn_24ea58, EU fn_24ec14, KR fn_24ecc0. Looks up one domain row.
pub fn domain_row(arena: &[u8], obj: u32, arg: u32, out: Option<&mut u32>) -> u32 {
    let table_ptr = word(arena, obj.wrapping_add(0x44));
    let row = table_row(arena, word(arena, table_ptr), arg);
    match (row, out) {
        (Some(row), Some(out)) => {
            *out = row;
            0
        }
        _ => forward_error(0),
    }
}

/// fn_24edbc, EU fn_24ef78, KR fn_24f024. Reads one domain entry's two ids.
pub fn domain_entry(arena: &[u8], obj: u32, idx: u32, out_a: &mut u16, out_b: &mut u16) -> u32 {
    let table_ptr = word(arena, obj.wrapping_add(0x44));
    match table_row(arena, word(arena, table_ptr), idx) {
        Some(entry) => {
            *out_a = half(arena, entry.wrapping_add(6));
            *out_b = half(arena, entry.wrapping_add(4));
            0
        }
        None => forward_error(0),
    }
}

/// fn_2e6008, EU fn_2e61c4, KR fn_2e6270. Selects one candidate pair.
pub fn select_pair(arena: &[u8], obj: u32, out: &mut [u8]) -> u32 {
    let sel = word(arena, obj.wrapping_add(0x68));
    let idx = half(arena, sel.wrapping_add(0x14)) as u32;
    if idx == 0 {
        let base = word(arena, sel.wrapping_add(8));
        put_half(out, 0, half(arena, base));
        put_half(out, 2, half(arena, base.wrapping_add(2)));
    } else {
        let table = word(arena, sel.wrapping_add(12));
        if idx > byte(arena, table.wrapping_add(0x52)) as u32 {
            return 0;
        }
        let entry = table.wrapping_add(0x26).wrapping_add(idx.wrapping_mul(4));
        let entry_y = table
            .wrapping_add(0x26)
            .wrapping_add((half(arena, sel.wrapping_add(0x14)) as u32).wrapping_mul(4))
            .wrapping_add(2);
        put_half(out, 0, half(arena, entry));
        put_half(out, 2, half(arena, entry_y));
    }
    1
}

fn calloc_field(
    arena: &mut [u8],
    obj: u32,
    ptr_field: u32,
    tag_field: u32,
    count: u32,
    size: u32,
    host: &mut impl NormalizationHost,
) -> u32 {
    let ctx = word(arena, obj);
    let heap = word(arena, ctx.wrapping_add(4));
    let block = host.calloc(arena, heap, count, size).unwrap_or(0);
    put_word(arena, obj.wrapping_add(ptr_field), block);
    if block == 0 {
        return error_code(10);
    }
    put_half(arena, obj.wrapping_add(tag_field), 3);
    put_half(arena, obj.wrapping_add(tag_field).wrapping_add(2), 0);
    0
}

/// fn_24cb74, EU fn_24cd30, KR fn_24cddc. Allocates the +0x78 record.
pub fn alloc_record(arena: &mut [u8], obj: u32, host: &mut impl NormalizationHost) -> u32 {
    calloc_field(arena, obj, 0x78, 0x7c, 1, 16, host)
}

/// fn_24cc90, EU fn_24ce4c, KR fn_24cef8. Allocates the +0x80 record.
pub fn alloc_record_second(arena: &mut [u8], obj: u32, host: &mut impl NormalizationHost) -> u32 {
    calloc_field(arena, obj, 0x80, 0x84, 1, 16, host)
}

/// fn_24cdac, EU fn_24cf68, KR fn_24d014. Allocates the two +0x62-sized tables.
pub fn alloc_tables(arena: &mut [u8], obj: u32, host: &mut impl NormalizationHost) -> u32 {
    let ctx = word(arena, obj);
    let heap = word(arena, ctx.wrapping_add(4));
    let size = (half(arena, obj.wrapping_add(0x62)) as u32)
        .wrapping_add(1)
        .wrapping_mul(2);
    let first = host.calloc(arena, heap, 1, size).unwrap_or(0);
    put_word(arena, obj.wrapping_add(0xc0), first);
    if first != 0 {
        put_half(arena, obj.wrapping_add(0xc4), 0);
        let second = host.calloc(arena, heap, 1, size).unwrap_or(0);
        put_word(arena, obj.wrapping_add(0xb8), second);
        if second != 0 {
            put_half(arena, obj.wrapping_add(0xbc), 0);
            return 0;
        }
    }
    error_code(10)
}

/// fn_24cea0, EU fn_24d05c, KR fn_24d108. Allocates a destination buffer of
/// the source's length.
pub fn alloc_destination(
    arena: &mut [u8],
    src: u32,
    dst: u32,
    host: &mut impl NormalizationHost,
) -> u32 {
    let length = half(arena, src.wrapping_add(0x4c));
    put_half(arena, dst.wrapping_add(8), length);
    let ctx = word(arena, src);
    let heap = word(arena, ctx.wrapping_add(4));
    let block = host.calloc(arena, heap, 1, length as u32).unwrap_or(0);
    put_word(arena, dst, block);
    if block == 0 {
        return error_code(10);
    }
    if let Some(slot) = arena.get_mut(block as usize) {
        *slot = 0;
    }
    put_half(arena, dst.wrapping_add(4), length);
    put_half(arena, dst.wrapping_add(6), 0);
    0
}

/// fn_24cf10, EU fn_24d0cc, KR fn_24d178. Shrinks an oversized buffer and
/// resets it.
pub fn shrink_buffer(
    arena: &mut [u8],
    src: u32,
    dst: u32,
    host: &mut impl NormalizationHost,
) -> u32 {
    let mut result = 0;
    if half(arena, dst.wrapping_add(4)) > half(arena, dst.wrapping_add(8)) {
        let ctx = word(arena, src);
        let heap = word(arena, ctx.wrapping_add(4));
        let size = (half(arena, dst.wrapping_add(8)) as u32).wrapping_add(1);
        match host.realloc(arena, heap, word(arena, dst), size) {
            Some(block) => {
                put_word(arena, dst, block);
                put_half(arena, dst.wrapping_add(4), half(arena, dst.wrapping_add(8)));
            }
            None => result = error_code(10),
        }
    }
    let buf = word(arena, dst);
    if buf != 0 {
        if let Some(slot) = arena.get_mut(buf as usize) {
            *slot = 0;
        }
        put_half(arena, dst.wrapping_add(6), 0);
    }
    result
}

/// fn_24d204, EU fn_24d3c0, KR fn_24d46c. Appends a slice of the source text
/// to a buffer, growing it first.
pub fn append_slice(
    arena: &mut [u8],
    obj: u32,
    buf: u32,
    start: u32,
    end: u32,
    host: &mut impl NormalizationHost,
) -> u32 {
    let n = end.wrapping_sub(start) as u16 as u32;
    let need = half(arena, buf.wrapping_add(6)) as u32 + n;
    if half(arena, buf.wrapping_add(6)) as u32 + n >= half(arena, buf.wrapping_add(4)) as u32 {
        let env = word(arena, obj);
        let heap = word(arena, env.wrapping_add(4));
        let cap = need as u16;
        match host.realloc(arena, heap, word(arena, buf), cap as u32 + 1) {
            Some(mem) => {
                put_word(arena, buf, mem);
                put_half(arena, buf.wrapping_add(4), cap);
            }
            None => return error_code(10),
        }
        if half(arena, buf.wrapping_add(6)) == 0 {
            if let Some(slot) = arena.get_mut(word(arena, buf) as usize) {
                *slot = 0;
            }
        }
    }
    let src = word(arena, obj.wrapping_add(92)).wrapping_add(start);
    strnappend(arena, word(arena, buf), src, n);
    let len = half(arena, buf.wrapping_add(6));
    put_half(arena, buf.wrapping_add(6), len.wrapping_add(n as u16));
    0
}

/// fn_24d2c8, EU fn_24d484, KR fn_24d530. Appends a string plus its
/// terminator to a buffer, growing it first.
pub fn append_string(
    arena: &mut [u8],
    obj: u32,
    buf: u32,
    text: u32,
    host: &mut impl NormalizationHost,
) -> u32 {
    if text == 0 || byte(arena, text) == 0 {
        return 0;
    }
    let n = strlen8(arena, text) as u16;
    let need = half(arena, buf.wrapping_add(6)) as u32 + n as u32 + 1;
    if (half(arena, buf.wrapping_add(4)) as u32) <= need {
        let env = word(arena, obj);
        let heap = word(arena, env.wrapping_add(4));
        let cap = need as u16;
        match host.realloc(arena, heap, word(arena, buf), cap as u32 + 1) {
            Some(mem) => {
                put_word(arena, buf, mem);
                put_half(arena, buf.wrapping_add(4), cap);
            }
            None => return error_code(10),
        }
        if half(arena, buf.wrapping_add(6)) == 0 {
            if let Some(slot) = arena.get_mut(word(arena, buf) as usize) {
                *slot = 0;
            }
        }
    }
    let data = word(arena, buf);
    let len = half(arena, buf.wrapping_add(6));
    memcpy_at(arena, data.wrapping_add(len as u32), text, n as u32);
    if let Some(slot) = arena.get_mut(data.wrapping_add(len as u32).wrapping_add(n as u32) as usize)
    {
        *slot = 0;
    }
    put_half(
        arena,
        buf.wrapping_add(6),
        len.wrapping_add(n).wrapping_add(1),
    );
    0
}

/// fn_24d13c, EU fn_24d2f8, KR fn_24d3a4. Concatenates one string buffer onto
/// another, growing the destination first.
pub fn concat_buffers(
    arena: &mut [u8],
    obj: u32,
    dst: u32,
    src: u32,
    host: &mut impl NormalizationHost,
) -> u32 {
    let add = half(arena, src.wrapping_add(6)) as u32;
    if add == 0 {
        return 0;
    }
    if (half(arena, dst.wrapping_add(4)) as u32) <= half(arena, dst.wrapping_add(6)) as u32 + add {
        let env = word(arena, obj);
        let heap = word(arena, env.wrapping_add(4));
        let cap = add.wrapping_add(half(arena, dst.wrapping_add(4)) as u32) as u16;
        match host.realloc(arena, heap, word(arena, dst), cap as u32 + 1) {
            Some(mem) => {
                put_word(arena, dst, mem);
                put_half(arena, dst.wrapping_add(4), cap);
            }
            None => return error_code(10),
        }
        if half(arena, dst.wrapping_add(6)) == 0 {
            if let Some(slot) = arena.get_mut(word(arena, dst) as usize) {
                *slot = 0;
            }
        }
    }
    strcat_at(arena, word(arena, dst), word(arena, src));
    let len = half(arena, dst.wrapping_add(6)).wrapping_add(half(arena, src.wrapping_add(6)));
    put_half(arena, dst.wrapping_add(6), len);
    0
}

/// fn_24d068, EU fn_24d224, KR fn_24d2d0. Appends a string to a buffer,
/// growing it first.
pub fn push_string(
    arena: &mut [u8],
    obj: u32,
    buf: u32,
    text: u32,
    host: &mut impl NormalizationHost,
) -> u32 {
    if text == 0 || byte(arena, text) == 0 {
        return 0;
    }
    let n = strlen8(arena, text) as u16;
    if half(arena, buf.wrapping_add(4)) as u32 <= half(arena, buf.wrapping_add(6)) as u32 + n as u32
    {
        let env = word(arena, obj);
        let heap = word(arena, env.wrapping_add(4));
        let cap = half(arena, buf.wrapping_add(4)).wrapping_add(n);
        match host.realloc(arena, heap, word(arena, buf), cap as u32 + 1) {
            Some(mem) => {
                put_word(arena, buf, mem);
                put_half(arena, buf.wrapping_add(4), cap);
            }
            None => return error_code(10),
        }
        if half(arena, buf.wrapping_add(6)) == 0 {
            if let Some(slot) = arena.get_mut(word(arena, buf) as usize) {
                *slot = 0;
            }
        }
    }
    strcat_at(arena, word(arena, buf), text);
    let len = half(arena, buf.wrapping_add(6));
    put_half(arena, buf.wrapping_add(6), len.wrapping_add(n));
    0
}

/// fn_252c34, EU fn_252df0, KR fn_252e9c. Deletes `len` elements at `offset`,
/// closing the gap and fixing the cursor.
pub fn delete_range(arena: &mut [u8], obj: u32, offset: u16, len: u16) -> u32 {
    if obj == 0 {
        return 0x8110_2007;
    }
    let buf = word(arena, obj.wrapping_add(12));
    let count = half(arena, buf.wrapping_add(8));
    if count <= offset {
        return 0x8110_200f;
    }
    if len == 0 {
        return 0;
    }
    let (len, tail) = if (offset as u32) + (len as u32) <= count as u32 {
        (len, count.wrapping_sub(offset).wrapping_sub(len))
    } else {
        (count.wrapping_sub(offset), 0)
    };
    let elem = half(arena, buf.wrapping_add(2)) as u32;
    let data = word(arena, buf.wrapping_add(4));
    memmove_at(
        arena,
        data.wrapping_add(offset as u32 * elem),
        data.wrapping_add(offset.wrapping_add(len) as u32 * elem),
        elem.wrapping_mul(tail as u32),
    );
    put_half(arena, buf.wrapping_add(8), count.wrapping_sub(len));
    let pos = half(arena, obj.wrapping_add(16));
    if pos > offset {
        if (pos as u32) < (offset as u32) + (len as u32) {
            put_half(arena, obj.wrapping_add(16), offset);
        } else {
            put_half(arena, obj.wrapping_add(16), pos.wrapping_sub(len));
        }
    }
    0
}

/// fn_252f88, EU fn_253144, KR fn_2531f0. Finds the `id` entry of a fetch
/// table, growing its buffer to `n` units, and returns the entry's array
/// header as an arena offset. Entry buffers hold `width`-rounded units of
/// `stride` bytes; the divisor is always the nonzero entry width.
pub fn fetch_entry(
    arena: &mut [u8],
    obj: u32,
    id: u32,
    n: u32,
    out: Option<&mut u32>,
    host: &mut impl NormalizationHost,
) -> u32 {
    if obj == 0 || n == 0 || out.is_none() {
        return 0x8110_2007;
    }
    let out = out.unwrap();
    let (status, ctx) = host.resolve(arena, word(arena, obj), word(arena, obj.wrapping_add(4)));
    if status >= 0x8000_0000 {
        return 0x8110_2007;
    }
    *out = 0;
    let count = half(arena, obj.wrapping_add(8));
    let tab = word(arena, obj.wrapping_add(12));
    let mut i = 0u16;
    while i < count && half(arena, tab.wrapping_add(i as u32 * 20).wrapping_add(4)) as u32 != id {
        i = i.wrapping_add(1);
    }
    if count <= i {
        return 0x8110_2014;
    }
    let entry = tab.wrapping_add(i as u32 * 20);
    let cap = half(arena, entry.wrapping_add(14));
    if n > cap as u32 {
        let width = half(arena, entry);
        if n > 0xffffu32.wrapping_sub(width as u32) {
            return 0x8110_2007;
        }
        let size = udiv(n, width as u32)
            .wrapping_add(1)
            .wrapping_mul(width as u32) as u16;
        let stride = half(arena, entry.wrapping_add(6)) as u32;
        let heap = word(arena, ctx.wrapping_add(4));
        let old = word(arena, entry.wrapping_add(8));
        let grown = if old != 0 {
            host.realloc(arena, heap, old, stride.wrapping_mul(size as u32))
        } else {
            host.alloc(arena, heap, stride.wrapping_mul(size as u32))
        };
        let buf = match grown {
            Some(buf) => buf,
            None => return 0x8110_200a,
        };
        if old != buf {
            put_word(arena, entry.wrapping_add(8), buf);
        }
        let used = half(arena, entry.wrapping_add(12)) as u32;
        memset_at(
            arena,
            buf.wrapping_add(used.wrapping_mul(stride)),
            0,
            (size as u32).wrapping_sub(used).wrapping_mul(stride),
        );
        put_half(arena, entry.wrapping_add(14), size);
    }
    *out = entry.wrapping_add(4);
    0
}

/// fn_2526a0, EU fn_25285c, KR fn_252908. Releases a fetch iterator.
pub fn release_iterator(arena: &mut [u8], obj: u32, host: &mut impl NormalizationHost) -> u32 {
    if obj == 0 {
        return 0x8110_2007;
    }
    let (status, ctx) = host.resolve(arena, word(arena, obj), word(arena, obj.wrapping_add(4)));
    if status >= 0x8000_0000 {
        return 0x8110_2007;
    }
    host.free(arena, word(arena, ctx.wrapping_add(4)), obj);
    0
}

/// fn_2526ec, EU fn_2528a8, KR fn_252954. Opens a fetch iterator over the
/// `id` entry of the fetch table at `table`.
pub fn open_iterator(
    arena: &mut [u8],
    h0: u32,
    h1: u32,
    table: u32,
    id: u32,
    out: Option<&mut u32>,
    host: &mut impl NormalizationHost,
) -> u32 {
    if out.is_none() || table == 0 {
        return 0x8110_2007;
    }
    let out = out.unwrap();
    let (status, ctx) = host.resolve(arena, h0, h1);
    if status >= 0x8000_0000 {
        return 0x8110_2007;
    }
    *out = 0;
    let heap = word(arena, ctx.wrapping_add(4));
    let obj = match host.alloc(arena, heap, 0x14) {
        Some(obj) => obj,
        None => return 0x8110_200a,
    };
    memset_at(arena, obj, 0, 0x14);
    put_word(arena, obj.wrapping_add(8), table);
    put_word(arena, obj, h0);
    put_word(arena, obj.wrapping_add(4), h1);
    let mut entry = 0u32;
    let status = fetch_entry(arena, table, id, 1, Some(&mut entry), host);
    if status >= 0x8000_0000 {
        let (resolve_status, ctx2) =
            host.resolve(arena, word(arena, obj), word(arena, obj.wrapping_add(4)));
        if resolve_status < 0x8000_0000 {
            host.free(arena, word(arena, ctx2.wrapping_add(4)), obj);
        }
        return status;
    }
    put_word(arena, obj.wrapping_add(12), entry);
    *out = obj;
    0
}

/// fn_252b5c, EU fn_252d18, KR fn_252dc4. Inserts `n` elements from `src` at
/// the cursor, growing the array first.
pub fn insert_elements(
    arena: &mut [u8],
    obj: u32,
    src: u32,
    n: u32,
    host: &mut impl NormalizationHost,
) -> u32 {
    if obj == 0 || src == 0 {
        return 0x8110_2007;
    }
    if n == 0 {
        return 0;
    }
    let mut arr = word(arena, obj.wrapping_add(12));
    let count = half(arena, arr.wrapping_add(8));
    let cap = half(arena, arr.wrapping_add(10));
    if (cap as u32) < (count as u32).wrapping_add(n) {
        let kind = half(arena, arr) as u32;
        let heap = word(arena, obj.wrapping_add(8));
        // The original passes `&q->arr` through, so a failed grow still
        // stores whatever the fetch wrote (zero past resolve).
        let mut grown = word(arena, obj.wrapping_add(12));
        let status = fetch_entry(
            arena,
            heap,
            kind,
            (count.wrapping_add(n as u16)) as u32,
            Some(&mut grown),
            host,
        );
        put_word(arena, obj.wrapping_add(12), grown);
        if status >= 0x8000_0000 {
            return status;
        }
        arr = grown;
    }
    let arr = arr;
    let count = half(arena, arr.wrapping_add(8));
    let pos = half(arena, obj.wrapping_add(16));
    let elem = half(arena, arr.wrapping_add(2)) as u32;
    let data = word(arena, arr.wrapping_add(4));
    if pos < count {
        memmove_at(
            arena,
            data.wrapping_add((pos as u32).wrapping_add(n).wrapping_mul(elem)),
            data.wrapping_add((pos as u32).wrapping_mul(elem)),
            (count.wrapping_sub(pos) as u32).wrapping_mul(elem),
        );
    }
    memmove_at(
        arena,
        data.wrapping_add(pos as u32 * elem),
        src,
        elem.wrapping_mul(n),
    );
    put_half(arena, arr.wrapping_add(8), count.wrapping_add(n as u16));
    put_half(arena, obj.wrapping_add(16), pos.wrapping_add(n as u16));
    0
}

/// fn_2b9608, EU fn_2b97c4, KR fn_2b9870. Finds the ordered slot for an event
/// and inserts it there.
pub fn insert_ordered(
    arena: &mut [u8],
    obj: u32,
    event: u32,
    host: &mut impl NormalizationHost,
) -> u32 {
    if obj == 0 || event == 0 {
        return 0x8110_2007;
    }
    let tab = word(arena, obj.wrapping_add(12));
    let stride = half(arena, tab.wrapping_add(2));
    let base = word(arena, tab.wrapping_add(4));
    let count = half(arena, tab.wrapping_add(8));
    let mut index = half(arena, obj.wrapping_add(16));
    let event_hi = word(arena, event.wrapping_add(20));
    let event_lo = word(arena, event.wrapping_add(24));
    let mut accept =
        |element: &[u8]| key_le(event_hi, event_lo, word(element, 20), word(element, 24));
    let status = scan_forward(
        &arena[base as usize..],
        stride,
        count,
        &mut index,
        &mut accept,
    );
    put_half(arena, obj.wrapping_add(16), index);
    if status >= 0x8000_0000 && status & 0x1fff != 0x14 {
        return status;
    }
    let status = insert_elements(arena, obj, event, 1, host);
    if status < 0x80000000 {
        0
    } else {
        status
    }
}

fn insert_sorted(arena: &mut [u8], table: u32, node: u32) {
    let head = word(arena, table.wrapping_add(4));
    if head == 0 {
        put_word(arena, table.wrapping_add(4), node);
        return;
    }
    let key = signed_half(arena, node.wrapping_add(8));
    let mut current = head;
    loop {
        if signed_half(arena, current.wrapping_add(8)) == key {
            let sub = signed_half(arena, node.wrapping_add(10));
            if sub >= signed_half(arena, current.wrapping_add(10)) {
                loop {
                    let next = word(arena, current);
                    if current != 0
                        && signed_half(arena, current.wrapping_add(8)) == key
                        && next != 0
                        && signed_half(arena, next.wrapping_add(8)) == key
                        && signed_half(arena, next.wrapping_add(10)) < sub
                    {
                        current = next;
                    } else {
                        let next = word(arena, current);
                        put_word(arena, current, node);
                        put_word(arena, node.wrapping_add(4), current);
                        if next != 0 {
                            put_word(arena, node, next);
                            put_word(arena, next.wrapping_add(4), node);
                        }
                        break;
                    }
                }
            } else {
                loop {
                    let prev = word(arena, current.wrapping_add(4));
                    if current != 0
                        && signed_half(arena, current.wrapping_add(8)) == key
                        && prev != 0
                        && signed_half(arena, prev.wrapping_add(8)) == key
                        && signed_half(arena, prev.wrapping_add(10)) > sub
                    {
                        current = prev;
                    } else {
                        let prev = word(arena, current.wrapping_add(4));
                        put_word(arena, current.wrapping_add(4), node);
                        put_word(arena, node, current);
                        if prev == 0 {
                            put_word(arena, table.wrapping_add(4), node);
                        } else {
                            put_word(arena, node.wrapping_add(4), prev);
                            put_word(arena, prev, node);
                        }
                        break;
                    }
                }
            }
            return;
        }
        if key <= signed_half(arena, current.wrapping_add(8)) {
            let prev = word(arena, current.wrapping_add(4));
            put_word(arena, current.wrapping_add(4), node);
            put_word(arena, node, current);
            if prev == 0 {
                put_word(arena, table.wrapping_add(4), node);
            } else {
                put_word(arena, node.wrapping_add(4), prev);
                put_word(arena, prev, node);
            }
            return;
        }
        let prev = current;
        current = word(arena, current);
        if current == 0 {
            put_word(arena, prev, node);
            put_word(arena, node.wrapping_add(4), prev);
            return;
        }
    }
}

/// fn_24e358, EU fn_24e514, KR fn_24e5c0. Allocates a keyed node from the
/// block table and inserts it into the sorted list.
pub fn insert_node(
    arena: &mut [u8],
    obj: u32,
    keys: u32,
    data: u32,
    host: &mut impl NormalizationHost,
) -> u32 {
    let table = word(arena, obj.wrapping_add(0x44));
    if keys == 0 || data == 0 {
        return error_code(7);
    }
    let count = half(arena, table.wrapping_add(16));
    if count == 0 {
        return error_code(0);
    }
    let mut index = half(arena, table.wrapping_add(22));
    let ents = word(arena, table.wrapping_add(12));
    let extra = half(arena, table.wrapping_add(20)) as u32;
    let mut entry = ents.wrapping_add(index as u32 * 12);
    if word(arena, entry.wrapping_add(4)) <= word(arena, entry.wrapping_add(8)).wrapping_add(extra)
    {
        let max = half(arena, table.wrapping_add(24));
        if (count as u32 + 1) < max as u32 {
            if (count as u32) <= (index as u32) + 1 {
                let fresh = ents.wrapping_add(count as u32 * 12);
                let cap = extra.wrapping_mul(half(arena, table.wrapping_add(18)) as u32) + 1;
                put_word(arena, fresh.wrapping_add(4), cap);
                put_word(arena, fresh.wrapping_add(8), 0);
                put_word(arena, fresh, 0);
                let ctx = word(arena, obj);
                let heap = word(arena, ctx.wrapping_add(4));
                match host.calloc(arena, heap, 1, cap) {
                    Some(buf) => put_word(arena, fresh, buf),
                    None => return error_code(10),
                }
                put_half(arena, table.wrapping_add(16), count.wrapping_add(1));
            }
            index = index.wrapping_add(1);
            put_half(arena, table.wrapping_add(22), index);
            entry = ents.wrapping_add(index as u32 * 12);
        } else {
            return error_code(0);
        }
    }
    let used = word(arena, entry.wrapping_add(8));
    put_word(arena, entry.wrapping_add(8), used.wrapping_add(extra));
    let node = word(arena, entry).wrapping_add(used);
    put_word(arena, node, 0);
    put_word(arena, node.wrapping_add(4), 0);
    memcpy_at(arena, node.wrapping_add(8), keys, 4);
    memcpy_at(arena, node.wrapping_add(12), data, 0x5c);
    insert_sorted(arena, table, node);
    let total = half(arena, table.wrapping_add(8));
    put_half(arena, table.wrapping_add(8), total.wrapping_add(1));
    0
}

/// fn_2e60d4, EU fn_2e6290, KR fn_2e633c. Counts leading separator units and
/// the index entries before them. `separators` is the borrowed ROM table.
pub fn scan_table(
    arena: &[u8],
    table: u32,
    out_a: Option<&mut u16>,
    index: u32,
    out_b: Option<&mut u16>,
    mode: u32,
    separators: &[u8],
) -> u32 {
    if table == 0 || out_a.is_none() {
        return 0x8110_2007;
    }
    let out_a = out_a.unwrap();
    if out_b.is_some() && index == 0 {
        return 0x8110_2007;
    }
    if mode == 0 || mode == 1 {
        let units = word(arena, table.wrapping_add(4));
        let mut span = 0u32;
        while span < half(arena, table.wrapping_add(8)) as u32
            && u16chr(separators, 0, half(arena, units.wrapping_add(span * 2))).is_some()
        {
            span = span.wrapping_add(1);
        }
        *out_a = span as u16;
    }
    if mode == 0 || mode == 2 {
        if let Some(out_b) = out_b {
            *out_b = 0;
            if *out_a == 0 {
                return 0;
            }
            let entries = word(arena, index.wrapping_add(4));
            let stride = half(arena, table.wrapping_add(2)) as u32;
            let first = word(arena, entries.wrapping_add(0x14));
            let mut count = 0u16;
            while (count as u32) < half(arena, index.wrapping_add(8)) as u32
                && (*out_a as u32).wrapping_mul(stride).wrapping_add(first)
                    > word(
                        arena,
                        entries.wrapping_add(count as u32 * 40).wrapping_add(0x14),
                    )
            {
                count = count.wrapping_add(1);
            }
            *out_b = count;
        }
    }
    0
}

/// fn_2e61d0, EU fn_2e638c, KR fn_2e6438. Advances `idx` past separator units
/// below `max`.
pub fn skip_matched(arena: &mut [u8], table: u32, idx: u32, max: u32, separators: &[u8]) {
    if table != 0 && idx != 0 {
        while (half(arena, idx) as u32) < max
            && u16chr(
                separators,
                0,
                half(arena, table.wrapping_add(half(arena, idx) as u32 * 2)),
            )
            .is_some()
        {
            let next = half(arena, idx);
            put_half(arena, idx, next.wrapping_add(1));
        }
    }
}

/// fn_2e62ac, EU fn_2e6468, KR fn_2e6514. Moves `idx` back over separator
/// units.
pub fn skip_matched_back(arena: &mut [u8], table: u32, idx: u32, separators: &[u8]) {
    if table != 0 && idx != 0 {
        while half(arena, idx) != 0
            && u16chr(
                separators,
                0,
                half(arena, table.wrapping_add(half(arena, idx) as u32 * 2)),
            )
            .is_some()
        {
            let next = half(arena, idx);
            put_half(arena, idx, next.wrapping_sub(1));
        }
    }
}

fn matches_separator(
    arena: &[u8],
    c: u16,
    set: u32,
    nset: u32,
    current: u16,
    original: u16,
) -> bool {
    let mut i = 0u32;
    while i < nset {
        if half(arena, set.wrapping_add(i * 2)) == c
            && (current != original || half(arena, set.wrapping_add(i * 2)) != 0x2e)
        {
            return true;
        }
        i = i.wrapping_add(1) as u16 as u32;
    }
    false
}

/// fn_2e634c, EU fn_2e6508, KR fn_2e65b4. Trims trailing units: separators
/// from the ROM table when `nset` is zero, otherwise the borrowed set.
pub fn trim_trailing(
    arena: &mut [u8],
    string: u32,
    len: u32,
    set: u32,
    nset: u32,
    separators: &[u8],
) {
    if string != 0 && len != 0 {
        if nset == 0 {
            while half(arena, len) != 0
                && u16chr(
                    separators,
                    0,
                    half(arena, string.wrapping_add(half(arena, len) as u32 * 2)),
                )
                .is_none()
            {
                let next = half(arena, len);
                put_half(arena, len, next.wrapping_sub(1));
            }
        } else {
            let original = half(arena, len);
            let mut current: u16;
            loop {
                current = half(arena, len);
                if current == 0 {
                    break;
                }
                if matches_separator(
                    arena,
                    half(arena, string.wrapping_add(current as u32 * 2)),
                    set,
                    nset,
                    current,
                    original,
                ) {
                    break;
                }
                put_half(arena, len, current.wrapping_sub(1));
            }
            if half(arena, len) != original {
                put_half(arena, len, current.wrapping_add(1));
            }
        }
    }
}

/// fn_2e6410, EU fn_2e65cc, KR fn_2e6678. Trims trailing units, keeping CJK
/// units and the set's separators.
pub fn trim_trailing_keep_cjk(
    arena: &mut [u8],
    string: u32,
    len: u32,
    set: u32,
    nset: u32,
    separators: &[u8],
) {
    fn is_trimmed(c: u16) -> bool {
        (c as u32).wrapping_sub(0x4e00) >= 0x5200
    }
    if string != 0 && len != 0 {
        if nset == 0 {
            loop {
                if half(arena, len) == 0 {
                    break;
                }
                let c = half(arena, string.wrapping_add(half(arena, len) as u32 * 2));
                if u16chr(separators, 0, c).is_some() {
                    break;
                }
                if !is_trimmed(c) {
                    break;
                }
                let next = half(arena, len);
                put_half(arena, len, next.wrapping_sub(1));
            }
        } else {
            let original = half(arena, len);
            loop {
                let current = half(arena, len);
                if current == 0 {
                    break;
                }
                let c = half(arena, string.wrapping_add(current as u32 * 2));
                if matches_separator(arena, c, set, nset, current, original) {
                    break;
                }
                if !is_trimmed(c) {
                    break;
                }
                put_half(arena, len, current.wrapping_sub(1));
            }
        }
    }
}

/// fn_2e64e4, EU fn_2e66a0, KR fn_2e674c. Trims trailing separators, then the
/// trailing pattern run, keeping the last pattern match.
pub fn trim_pattern(arena: &mut [u8], pattern: u32, string: u32, len: u32, separators: &[u8]) {
    if string == 0 || len == 0 {
        return;
    }
    let at = |arena: &[u8]| half(arena, string.wrapping_add(half(arena, len) as u32 * 2));
    // loop1: skip trailing separators; the first other unit enters loop2.
    loop {
        if half(arena, len) == 0 {
            break;
        }
        if u16chr(separators, 0, at(arena)).is_none() {
            // loop2: skip trailing non-pattern units up to a separator.
            loop {
                if half(arena, len) == 0 {
                    break;
                }
                if u16chr(separators, 0, at(arena)).is_some() {
                    break;
                }
                if u16chr(arena, pattern, at(arena)).is_none() {
                    let next = half(arena, len);
                    put_half(arena, len, next.wrapping_sub(1));
                } else {
                    break;
                }
            }
            break;
        }
        let next = half(arena, len);
        put_half(arena, len, next.wrapping_sub(1));
    }
    if u16chr(arena, pattern, at(arena)).is_some() {
        return;
    }
    let saved = half(arena, len);
    loop {
        if half(arena, len) == 0 {
            break;
        }
        if u16chr(arena, pattern, at(arena)).is_some() {
            break;
        }
        let next = half(arena, len);
        put_half(arena, len, next.wrapping_sub(1));
    }
    if u16chr(arena, pattern, at(arena)).is_none() {
        put_half(arena, len, saved);
    }
}

/// fn_176314, EU fn_1763c4, KR fn_176370. Appends the NUL-terminated narrow
/// string at `src` to the NUL-terminated wide string at `dst`, zero-extending
/// each byte. Returns `dst`.
pub fn append_narrow(arena: &mut [u8], dst: u32, src: u32) -> u32 {
    let mut d = dst;
    while half(arena, d) != 0 {
        d = d.wrapping_add(2);
    }
    let mut s = src;
    loop {
        let c = byte(arena, s);
        put_half(arena, d, c as u16);
        if c == 0 {
            break;
        }
        s = s.wrapping_add(1);
        d = d.wrapping_add(2);
    }
    dst
}

/// Borrowed ROM tables for [`decode_units`]: the extra-bytes table
/// `D_8dc1f3`, the lead-byte mask table `D_8dc1e6`, and the offset table
/// `D_8dc2f4`. Each slice is one contiguous window of the caller's mapped
/// image; `mask_base`/`base_base` are the offsets of `D_8dc1e6`/`D_8dc2f4`
/// inside their windows. The mask window must also cover the six bytes
/// before `D_8dc1e6`: the original indexes `mask[extra - 6]` with `extra`
/// as low as 0.
pub struct DecoderTables<'a> {
    pub extra: &'a [u8],
    pub mask: &'a [u8],
    pub mask_base: u32,
    pub base: &'a [u8],
    pub base_base: u32,
}

/// fn_17bb44, EU fn_17bbf4, KR fn_17bba0. Decodes up to `length` engine bytes
/// at `input` into `capacity` u16 units at `output`, following the borrowed
/// [`DecoderTables`]. Optional out-words `consumed`/`written` report the
/// input offset and unit count; `positions` (0 for null) receives two source
/// offsets per unit. Matches the original's one-past-`length` lookahead read
/// on truncated multibyte tails: the caller furnishes that slack byte in the
/// arena, so the in-arena read here sees the same byte the original loads.
pub fn decode_units(
    arena: &mut [u8],
    input: u32,
    length: u32,
    consumed: u32,
    output: u32,
    capacity: u32,
    written: u32,
    positions: u32,
    tables: &DecoderTables<'_>,
) -> u32 {
    let (mut status, offset, count) =
        decode_run(arena, input, length, output, capacity, positions, tables);
    if consumed != 0 {
        put_word(arena, consumed, offset);
    }
    if written != 0 {
        put_word(arena, written, count);
    }
    if status < 0x8000_0000 && offset != length {
        status = 0x8bc0_2009;
    }
    status
}

/// Shared [`decode_units`] loop. Returns `(status, offset, count)` before the
/// trailing capacity-exhausted adjustment; out-cells are the caller's job so
/// [`decode_str`] can discard everything but the count, matching the original.
pub(crate) fn decode_run(
    arena: &mut [u8],
    input: u32,
    length: u32,
    output: u32,
    capacity: u32,
    mut positions: u32,
    tables: &DecoderTables<'_>,
) -> (u32, u32, u32) {
    fn table_byte(window: &[u8], at: u32) -> u8 {
        window.get(at as usize).copied().unwrap_or(0)
    }
    fn table_word(window: &[u8], at: u32) -> u32 {
        let mut value = 0u32;
        for lane in 0..4 {
            value |= (table_byte(window, at.wrapping_add(lane)) as u32) << (lane * 8);
        }
        value
    }
    let mut status = 0u32;
    let mut offset = 0u32;
    let mut count = 0u32;
    let invalid_code = 0x8bc0_2105u32;
    while offset < length && count < capacity {
        let here = input.wrapping_add(offset);
        let lead = byte(arena, here);
        let extra = table_byte(tables.extra, lead as u32) as u32;
        if length < offset.wrapping_add(extra) {
            status = 0x0bc0_2007u32;
            put_half(arena, output.wrapping_add(count.wrapping_mul(2)), 0);
            break;
        }
        let mask_hi = table_byte(tables.mask, tables.mask_base.wrapping_add(extra));
        let mask_lo = table_byte(
            tables.mask,
            tables.mask_base.wrapping_add(extra).wrapping_sub(6),
        );
        if lead & mask_hi != mask_lo {
            status = invalid_code;
            put_half(arena, output.wrapping_add(count.wrapping_mul(2)), 0);
            break;
        }
        if positions != 0 {
            put_word(arena, positions, offset);
            put_word(arena, positions.wrapping_add(4), offset);
            positions = positions.wrapping_add(8);
        }
        let mut code = lead as u32;
        offset = offset.wrapping_add(1);
        let mut valid = true;
        match extra {
            0 => {}
            1 => {
                let cont = byte(arena, input.wrapping_add(offset));
                if cont & 0xc0 != 0x80 {
                    valid = false;
                } else {
                    code = code.wrapping_shl(6).wrapping_add(cont as u32);
                    offset = offset.wrapping_add(1);
                }
            }
            2 => {
                let first = byte(arena, input.wrapping_add(offset));
                let second = byte(arena, input.wrapping_add(offset).wrapping_add(1));
                if code == 0xe0 && first < 0xa0 {
                    valid = false;
                } else if first & 0xc0 != 0x80 || second & 0xc0 != 0x80 {
                    valid = false;
                } else {
                    code = code.wrapping_shl(6).wrapping_add(first as u32);
                    code = code.wrapping_shl(6).wrapping_add(second as u32);
                    offset = offset.wrapping_add(2);
                }
            }
            3 => {
                let first = byte(arena, input.wrapping_add(offset));
                let second = byte(arena, input.wrapping_add(offset).wrapping_add(1));
                let third = byte(arena, input.wrapping_add(offset).wrapping_add(2));
                if code == 0xf0 && first < 0x90 {
                    valid = false;
                } else if code == 0xf4 && first > 0x8f {
                    valid = false;
                } else if first & 0xc0 != 0x80 || second & 0xc0 != 0x80 || third & 0xc0 != 0x80 {
                    valid = false;
                } else {
                    code = code.wrapping_shl(6).wrapping_add(first as u32);
                    code = code.wrapping_shl(6).wrapping_add(second as u32);
                    code = code.wrapping_shl(6).wrapping_add(third as u32);
                    offset = offset.wrapping_add(3);
                }
            }
            _ => {
                valid = false;
            }
        }
        if !valid {
            status = invalid_code;
            put_half(arena, output.wrapping_add(count.wrapping_mul(2)), 0);
            break;
        }
        code = code.wrapping_sub(table_word(
            tables.base,
            tables.base_base.wrapping_add(extra.wrapping_mul(4)),
        ));
        if code >> 16 != 0 {
            status = invalid_code;
            put_half(arena, output.wrapping_add(count.wrapping_mul(2)), 0);
            break;
        }
        put_half(
            arena,
            output.wrapping_add(count.wrapping_mul(2)),
            code as u16,
        );
        count = count.wrapping_add(1);
    }
    (status, offset, count)
}

/// fn_17bfc0, EU fn_17c070, KR fn_17c01c. Counts the decoded units of the
/// NUL-terminated engine string at `p`, plus one, stepping over continuation
/// bytes with the borrowed extra-bytes table `D_8dc1f3`.
pub fn decoded_count(arena: &[u8], p: u32, extra: &[u8]) -> i32 {
    let mut n = 1i32;
    let mut at = p;
    let mut c = byte(arena, at);
    at = at.wrapping_add(1);
    while c != 0 {
        n = n.wrapping_add(1);
        at = at.wrapping_add(extra.get(c as usize).copied().unwrap_or(0) as u32);
        c = byte(arena, at);
        at = at.wrapping_add(1);
    }
    n
}

/// fn_17bd74, EU fn_17be24, KR fn_17bdd0. Decodes the NUL-terminated engine
/// string at `s` into `out`, returning the unit count. The status from the
/// shared [`decode_units`] core is discarded, matching the original.
pub fn decode_str(arena: &mut [u8], s: u32, out: u32, tables: &DecoderTables<'_>) -> u32 {
    let length = strlen8(arena, s).wrapping_add(1);
    let (_, _, count) = decode_run(arena, s, length, out, 0xffff_ffff, 0, tables);
    count
}

fn check_tagged(
    handle: u32,
    tag: u32,
    size: u32,
    error: u32,
    host: &mut impl HandleValidator,
) -> u32 {
    let status = host.validate(handle, tag, size);
    if status >= 0x8000_0000 {
        error
    } else {
        status
    }
}

/// fn_1de844, EU fn_1dea00, KR fn_1deaac. Validates an FE_PUNCSPTN handle and
/// remaps a failure to the component's own status.
pub fn check_punctuation(handle: u32, host: &mut impl HandleValidator) -> u32 {
    check_tagged(handle, 0xf383, 56, 0x89c0_2008, host)
}

/// fn_1ea328, EU fn_1ea4e4, KR fn_1ea590. Validates an FE_DCTLKP handle and
/// remaps a failure to the component's own status.
pub fn check_dctlkp(handle: u32, host: &mut impl HandleValidator) -> u32 {
    check_tagged(handle, 0xf3b3, 60, 0x8b80_2008, host)
}

/// fn_206bc8, EU fn_206d84, KR fn_206e30. Validates a packaging handle and
/// remaps a failure to the component's own status.
pub fn check_packaging(handle: u32, host: &mut impl HandleValidator) -> u32 {
    check_tagged(handle, 0xf382, 44, 0x89b0_2008, host)
}

/// fn_207130, EU fn_2072ec, KR fn_207398. Validates a packaging handle and
/// remaps a failure to the component's own status.
pub fn check_packaging_second(handle: u32, host: &mut impl HandleValidator) -> u32 {
    check_tagged(handle, 0xf382, 44, 0x89b0_2008, host)
}

/// fn_24d5a4, EU fn_24d760, KR fn_24d80c. Publishes `object` to `out` unless
/// any input is null.
pub fn publish_object(arena: &mut [u8], object: u32, key: u32, out: u32) -> u32 {
    if object == 0 || key == 0 || out == 0 {
        error_code(7)
    } else {
        put_word(arena, out, object);
        0
    }
}

/// fn_24d600, EU fn_24d7bc, KR fn_24d868. Publishes `object` to `out` unless
/// any input is null.
pub fn publish_object_second(arena: &mut [u8], object: u32, key: u32, out: u32) -> u32 {
    if object == 0 || key == 0 || out == 0 {
        error_code(7)
    } else {
        put_word(arena, out, object);
        0
    }
}

/// fn_195d58, EU fn_195e08, KR fn_195e44. Byte-string order: -1, 0 or 1.
pub fn strcmp_at(arena: &[u8], a: u32, b: u32) -> i32 {
    let mut a = a;
    let mut b = b;
    loop {
        let d = byte(arena, a) as i32 - byte(arena, b) as i32;
        if d != 0 || byte(arena, b) == 0 {
            if d < 0 {
                return -1;
            }
            if d > 0 {
                return 1;
            }
            return 0;
        }
        a = a.wrapping_add(1);
        b = b.wrapping_add(1);
    }
}

/// fn_195cd0, EU fn_195d80, KR fn_195dbc. Copies the NUL-terminated string at
/// `src` to `dst`, returning `dst`.
pub fn strcpy_at(arena: &mut [u8], dst: u32, src: u32) -> u32 {
    let mut d = dst;
    let mut s = src;
    loop {
        let c = byte(arena, s);
        if let Some(slot) = arena.get_mut(d as usize) {
            *slot = c;
        }
        if c == 0 {
            break;
        }
        d = d.wrapping_add(1);
        s = s.wrapping_add(1);
    }
    dst
}

/// fn_240ba0, EU fn_240d5c, KR fn_240e08. Latin-1-ish tolower with 0x9f/0x8a
/// quirks; non-letters (including wide inputs) pass through unmasked.
pub fn tolower_240ba0(c: u32) -> u32 {
    if c == 0x9f {
        return 0xff;
    }
    if c == 0x8a || c == 0x8c {
        return c.wrapping_add(0x10) & 0xff;
    }
    let lower = c.wrapping_sub(0x41) <= 0x19
        || c.wrapping_sub(0xc0) <= 0x16
        || c.wrapping_sub(0xd8) <= 6
        || c == 0xdf;
    if !lower {
        return c;
    }
    c.wrapping_add(0x20) & 0xff
}

/// fn_195cf8, EU fn_195da8, KR fn_195de4. Last occurrence of `c` (low byte)
/// in the NUL-terminated string at `s`, or 0.
pub fn strrchr_at(arena: &[u8], s: u32, c0: u32) -> u32 {
    let c = (c0 & 0xff) as u8;
    let mut end = s;
    while byte(arena, end) != 0 {
        end = end.wrapping_add(1);
    }
    let mut p = end;
    loop {
        if byte(arena, p) == c {
            return p;
        }
        if p == s {
            break;
        }
        p = p.wrapping_sub(1);
    }
    0
}

/// fn_195df8, EU fn_195ea8, KR fn_195ee4. Compares up to `n` bytes; stops at
/// the first difference or at `a`'s NUL.
pub fn strncmp_at(arena: &[u8], a: u32, b: u32, n: u32) -> i32 {
    if n == 0 {
        return 0;
    }
    let mut n = n;
    let mut a = a;
    let mut b = b;
    loop {
        n = n.wrapping_sub(1);
        if n == 0 {
            break;
        }
        if byte(arena, a) == 0 {
            break;
        }
        if byte(arena, a) != byte(arena, b) {
            break;
        }
        a = a.wrapping_add(1);
        b = b.wrapping_add(1);
    }
    byte(arena, a) as i32 - byte(arena, b) as i32
}

/// fn_252cf8, EU fn_252eb4, KR fn_252f60. Allocates a 16-byte fetch table for
/// the `(h0, h1)` handle pair.
pub fn alloc_fetch_table(
    arena: &mut [u8],
    h0: u32,
    h1: u32,
    out: u32,
    host: &mut impl NormalizationHost,
) -> u32 {
    if out == 0 {
        return 0x8110_2007;
    }
    let (status, env) = host.resolve(arena, h0, h1);
    if status >= 0x8000_0000 {
        return 0x8110_2007;
    }
    put_word(arena, out, 0);
    let heap = word(arena, env.wrapping_add(4));
    let Some(table) = host.alloc(arena, heap, 16) else {
        return 0x8110_200a;
    };
    for index in 0..16 {
        if let Some(slot) = arena.get_mut(table as usize + index) {
            *slot = 0;
        }
    }
    put_word(arena, table, h0);
    put_word(arena, table.wrapping_add(4), h1);
    put_word(arena, out, table);
    0
}

fn grow_slots(
    arena: &mut [u8],
    obj: u32,
    n: u32,
    arr: u32,
    cnt: u32,
    used: u32,
    host: &mut impl NormalizationHost,
) -> u32 {
    if n != 0 {
        let k = if n > 3 {
            n
        } else {
            if n != 3 || half(arena, obj.wrapping_add(cnt)) <= 3 {
                put_half(arena, obj.wrapping_add(used), 0);
                return 0;
            }
            3
        };
        let ctx = word(arena, obj);
        let heap = word(arena, ctx.wrapping_add(4));
        let old = word(arena, obj.wrapping_add(arr));
        let size = k.wrapping_add(1).wrapping_mul(4);
        let Some(block) = host.realloc(arena, heap, old, size) else {
            return error_code(10);
        };
        put_word(arena, obj.wrapping_add(arr), block);
        put_half(arena, obj.wrapping_add(cnt), k as u16);
    }
    put_half(arena, obj.wrapping_add(used), 0);
    0
}

/// fn_24cc08, EU fn_24cdc4, KR fn_24ce70. Grows the +0x78 slot array and
/// clears the used count.
pub fn grow_slots_first(
    arena: &mut [u8],
    obj: u32,
    n: u32,
    host: &mut impl NormalizationHost,
) -> u32 {
    grow_slots(arena, obj, n, 0x78, 0x7c, 0x7e, host)
}

/// fn_24cd24, EU fn_24cee0, KR fn_24cf8c. Grows the +0x80 slot array and
/// clears the used count.
pub fn grow_slots_second(
    arena: &mut [u8],
    obj: u32,
    n: u32,
    host: &mut impl NormalizationHost,
) -> u32 {
    grow_slots(arena, obj, n, 0x80, 0x84, 0x86, host)
}

/// fn_24cfa0, EU fn_24d15c, KR fn_24d208. Grows the destination buffer to `n`
/// and clears it. `n` is used full-width for the compare and grow size,
/// matching the ARM (`cmp r0, r2` with no narrowing); only the stored
/// capacity is the low half.
pub fn reset_buffer(
    arena: &mut [u8],
    src: u32,
    dst: u32,
    n: u32,
    host: &mut impl NormalizationHost,
) -> u32 {
    let ret = 0;
    if half(arena, dst.wrapping_add(4)) as u32 <= n {
        let ctx = word(arena, src);
        let heap = word(arena, ctx.wrapping_add(4));
        let old = word(arena, dst);
        let size = n.wrapping_add(1);
        let Some(block) = host.realloc(arena, heap, old, size) else {
            return error_code(10);
        };
        put_word(arena, dst, block);
        put_half(arena, dst.wrapping_add(4), n as u16);
    }
    if ret < 0x8000_0000 {
        let buf = word(arena, dst);
        if let Some(slot) = arena.get_mut(buf as usize) {
            *slot = 0;
        }
        put_half(arena, dst.wrapping_add(6), 0);
    }
    ret
}

/// fn_2511ac, EU fn_251368, KR fn_251414. Reads one indexed table entry's
/// value, length and pointer.
pub fn indexed_entry(
    arena: &mut [u8],
    obj: u32,
    idx: u32,
    out: u32,
    out_len: u32,
    out_ptr: u32,
) -> u32 {
    let inner = word(arena, obj.wrapping_add(0x38));
    let list = word(arena, inner.wrapping_add(8));
    if list == 0 {
        return forward_error(7);
    }
    let groups = word(arena, list.wrapping_add(4));
    let f2 = half(arena, idx.wrapping_add(2)) as u32;
    let group = word(arena, groups.wrapping_add(f2.wrapping_mul(4)));
    let table = word(arena, group.wrapping_add(4));
    let entries = word(arena, table.wrapping_add(0x18));
    let f4 = half(arena, idx.wrapping_add(4)) as u32;
    let entry = entries.wrapping_add(f4.wrapping_mul(24));
    let sum = (half(arena, entry.wrapping_add(2)) as u32)
        .wrapping_add(word(arena, table.wrapping_add(12)));
    put_word(arena, out, sum);
    let len = half(arena, entry.wrapping_add(0x12));
    put_half(arena, out_len, len);
    put_word(
        arena,
        out_ptr,
        if len == 0 {
            0
        } else {
            word(arena, entry.wrapping_add(12))
        },
    );
    0
}

/// fn_25123c, EU fn_2513f8, KR fn_2514a4. Reads one two-level table entry.
pub fn indexed_pair(
    arena: &mut [u8],
    obj: u32,
    idx: u32,
    o1: u32,
    o2: u32,
    o3: u32,
    o4: u32,
) -> u32 {
    let inner = word(arena, obj.wrapping_add(0x38));
    let b = word(arena, inner.wrapping_add(8));
    if b == 0 {
        return forward_error(7);
    }
    let slots = word(arena, b.wrapping_add(4));
    let i1 = half(arena, idx.wrapping_add(2)) as u32;
    let slot = word(arena, slots.wrapping_add(i1.wrapping_mul(4)));
    let c = word(arena, slot.wrapping_add(4));
    let f18 = word(arena, c.wrapping_add(0x18));
    let f1c = word(arena, c.wrapping_add(0x1c));
    let i2 = half(arena, idx.wrapping_add(4)) as u32;
    let e = f18.wrapping_add(i2.wrapping_mul(0x18));
    let e2 = half(arena, e.wrapping_add(4)) as u32;
    let f = f1c.wrapping_add(e2.wrapping_mul(0x10));
    put_half(arena, o2, half(arena, f.wrapping_add(10)));
    let flag = byte(arena, f.wrapping_add(12));
    if let Some(slot) = arena.get_mut(o4 as usize) {
        *slot = flag;
    }
    let sum = word(arena, f.wrapping_add(4)).wrapping_add(word(arena, c.wrapping_add(0x10)));
    put_word(arena, o1, sum);
    let sum3 = word(arena, c.wrapping_add(12)).wrapping_add(half(arena, e.wrapping_add(2)) as u32);
    put_word(arena, o3, sum3);
    0
}

/// fn_250b80, EU fn_250d3c, KR fn_250de8. Searches the named-entry groups for
/// `name`, writing the value and ids on a match.
pub fn find_named(
    arena: &[u8],
    ctx: u32,
    name: u32,
    value: &mut u32,
    first: &mut u16,
    second: &mut u16,
) -> u32 {
    let groups = word(arena, ctx.wrapping_add(0x38));
    let table = word(arena, groups.wrapping_add(8));
    if table == 0 {
        return 0;
    }
    let mut i = 0u16;
    while i < half(arena, table) {
        let refs = word(arena, table.wrapping_add(4));
        let gref = word(arena, refs.wrapping_add(i as u32 * 4));
        let group = word(arena, gref.wrapping_add(12));
        if group != 0 && byte(arena, group) == 0 {
            let mut j = 0u16;
            while j < half(arena, group.wrapping_add(8)) {
                let entries = word(arena, group.wrapping_add(4));
                let entry = word(arena, entries.wrapping_add(j as u32 * 4));
                let mut k = 0u16;
                while k < half(arena, entry.wrapping_add(8)) {
                    let names = word(arena, entry.wrapping_add(12));
                    let candidate = word(arena, names.wrapping_add(k as u32 * 4));
                    if strcmp_at(arena, candidate, name) == 0 {
                        *value = word(arena, entry);
                        *first = half(arena, entry.wrapping_add(4));
                        *second = half(arena, entry.wrapping_add(6));
                        return 1;
                    }
                    k = k.wrapping_add(1);
                }
                j = j.wrapping_add(1);
            }
        }
        i = i.wrapping_add(1);
    }
    0
}

/// Host lookup for `fn_24d8c8` (`fn_1966ac`, owned by slice 5).
pub trait LookupHost {
    /// Returns the value `fn_1966ac` would store through `out`.
    fn lookup(&mut self, obj: u32, arg: u32) -> u32;
}

/// fn_24d8c8, EU fn_24da84, KR fn_24db30. Dispatches one item lookup.
pub fn dispatch_lookup(
    arena: &mut [u8],
    obj: u32,
    idx: u32,
    arg: u32,
    out: u32,
    host: &mut impl LookupHost,
) -> u32 {
    let items = word(arena, obj.wrapping_add(60));
    put_word(arena, out, 0);
    let item = word(arena, items.wrapping_add(idx.wrapping_mul(4)));
    let value = host.lookup(item, arg);
    put_word(arena, out, value);
    0
}

/// Host PCRE deinit for `fn_1e502c` (`fn_23ffd4`, owned by half 2a).
pub trait PcreDeinitHost {
    /// Returns the status `fn_23ffd4` would log.
    fn deinit(&mut self, a: u32, b: u32) -> u32;
}

/// fn_1e502c, EU fn_1e51e8, KR fn_1e5294. Ends TOKENTN processing: validates
/// both handles, runs PCRE deinit for its logged status, and returns 0.
pub fn end_token_processing(
    arena: &[u8],
    obj: u32,
    b: u32,
    validator: &mut impl HandleValidator,
    pcre: &mut impl PcreDeinitHost,
) -> u32 {
    if validator.validate(b, 0xe58c, 8) >= 0x8000_0000 {
        return 0x8960_2008;
    }
    let inner = word(arena, obj);
    let inner_b = word(arena, obj.wrapping_add(4));
    if validator.validate(inner_b, 0xe58d, 0xf4) >= 0x8000_0000 {
        return 0x8960_2008;
    }
    let v0 = word(arena, inner.wrapping_add(0x30));
    let v1 = word(arena, inner.wrapping_add(0x34));
    let _ = pcre.deinit(v0, v1);
    0
}

/// fn_2b0b48, EU fn_2b0d04, KR fn_2b0db0. Allocates the transcoder flag byte.
pub fn alloc_xcoder_flag(arena: &mut [u8], obj: u32, host: &mut impl NormalizationHost) -> u32 {
    let h0 = word(arena, obj);
    let h1 = word(arena, obj.wrapping_add(4));
    let (status, ctx) = host.resolve(arena, h0, h1);
    if status >= 0x8000_0000 {
        return status;
    }
    let heap = word(arena, ctx.wrapping_add(4));
    let block = host.alloc(arena, heap, 1).unwrap_or(0);
    put_word(arena, obj.wrapping_add(268), block);
    if block == 0 {
        return 0x8810_200a;
    }
    if let Some(slot) = arena.get_mut(block as usize) {
        *slot = 0;
    }
    status
}

/// Host resource operations for the FE_NORMOUT open/close/teardown functions
/// (`fn_1e7260`, `fn_1e7318`, `fn_1e7688`): named resource acquire/release
/// (`fn_17b058`, `fn_17b1c4`) and the indirect CTLKLP `getString` call.
/// Names are arena offsets of NUL-terminated strings.
pub trait NormOutHost: NormalizationHost {
    /// `fn_17b058`: acquire the named resource. Returns `(status, resource)`,
    /// with the resource as an arena offset.
    fn acquire(&mut self, arena: &mut [u8], resources: u32, name: u32) -> (u32, u32);
    /// `fn_17b1c4`: release the named resource.
    fn release_named(&mut self, resources: u32, name: u32);
    /// Indirect `getString` through the acquired CTLKLP vtable: the callee
    /// takes `char***`, so this writes the string-slot pointer (`0x770`) to
    /// `out_value` and the value offset to the slot, the length half to
    /// `out_len` and the separator byte to `out_sep`, returning the status.
    /// A null script value writes a null `out_value`, matching the caller's
    /// `value != 0` guard.
    fn get_string(
        &mut self,
        arena: &mut [u8],
        ctlkp_h0: u32,
        ctlkp_h1: u32,
        out_value: u32,
        out_len: u32,
        out_sep: u32,
    ) -> u32;
}

/// String pool layout for the output open/close functions: `"LINGDB"`,
/// `"FE_DEPES"` and `"FE_DCTLKP"` live at these offsets from the `names` base.
pub const NAME_LINGDB: u32 = 0;
/// Offset of `"FE_DEPES"` within the output name pool.
pub const NAME_OUT_FE_DEPES: u32 = 7;
/// Offset of `"FE_DCTLKP"` within the output name pool.
pub const NAME_OUT_FE_DCTLKP: u32 = 16;

/// fn_1e7260, EU fn_1e741c, KR fn_1e74c8. Tears down the normalization output
/// object: validates the handle, releases a live `extra` block, clears it,
/// detaches the three resources and frees the object. A null object returns
/// the validator status unchanged. `names` is the string pool above.
pub fn close_output(
    arena: &mut [u8],
    obj: u32,
    key: u32,
    names: u32,
    validator: &mut impl HandleValidator,
    host: &mut impl NormOutHost,
) -> u32 {
    let status = validator.validate(key, 0xf3b3, 0x3c);
    if status >= 0x8000_0000 {
        return 0x8b80_2008;
    }
    if obj != 0 {
        let core = word(arena, obj);
        let heap = word(arena, core.wrapping_add(4));
        let resources = word(arena, core.wrapping_add(0x18));
        let extra = word(arena, obj.wrapping_add(0x38));
        if extra != 0 {
            host.free(arena, heap, extra);
        }
        put_word(arena, obj.wrapping_add(0x38), 0);
        host.release_named(resources, names.wrapping_add(NAME_LINGDB));
        host.release_named(resources, names.wrapping_add(NAME_OUT_FE_DEPES));
        host.release_named(resources, names.wrapping_add(NAME_OUT_FE_DCTLKP));
        host.free(arena, heap, obj);
        return 0;
    }
    status
}

/// fn_1e7318, EU fn_1e74d4, KR fn_1e7580. Opens the normalization output
/// object: resolves the engine, acquires the LINGDB/FE_DEPES/FE_DCTLKP
/// resources, allocates the object and its `extra` block, then reads the
/// `usetntag` flag through the CTLKLP `getString`. A null `out` handle
/// returns `0x8b802007` before touching anything. `names` is the string pool
/// above; the handle pair is passed as its two words. `scratch` holds the
/// value/length/separator outputs like the C stack locals, outside any
/// observation window.
pub fn open_output(
    arena: &mut [u8],
    h0: u32,
    h1: u32,
    out: u32,
    names: u32,
    scratch: u32,
    validator: &mut impl HandleValidator,
    host: &mut impl NormOutHost,
) -> u32 {
    if out == 0 {
        return 0x8b80_2007;
    }
    put_word(arena, out, 0);
    put_word(arena, out.wrapping_add(4), 0);
    let (status, core) = host.resolve(arena, h0, h1);
    if status >= 0x8000_0000 {
        return status;
    }
    let heap = word(arena, core.wrapping_add(4));
    let resources = word(arena, core.wrapping_add(0x18));
    let (status, lingdb) = host.acquire(arena, resources, names.wrapping_add(NAME_LINGDB));
    if status >= 0x8000_0000 {
        return status;
    }
    let (status, depes) = host.acquire(arena, resources, names.wrapping_add(NAME_OUT_FE_DEPES));
    if status >= 0x8000_0000 {
        host.release_named(resources, names.wrapping_add(NAME_LINGDB));
        return status;
    }
    let (status, ctlkp) = host.acquire(arena, resources, names.wrapping_add(NAME_OUT_FE_DCTLKP));
    if status >= 0x8000_0000 {
        host.release_named(resources, names.wrapping_add(NAME_LINGDB));
        host.release_named(resources, names.wrapping_add(NAME_OUT_FE_DEPES));
        return status;
    }
    let object = match host.alloc(arena, heap, 0x3c) {
        Some(object) => object,
        None => {
            host.release_named(resources, names.wrapping_add(NAME_LINGDB));
            host.release_named(resources, names.wrapping_add(NAME_OUT_FE_DEPES));
            host.release_named(resources, names.wrapping_add(NAME_OUT_FE_DCTLKP));
            return 0x8b80_200a;
        }
    };
    let lingdb_vtable = word(arena, lingdb.wrapping_add(4));
    let depes_h0 = word(arena, depes.wrapping_add(8));
    let depes_h1 = word(arena, depes.wrapping_add(12));
    let depes_vtable = word(arena, depes.wrapping_add(4));
    let ctlkp_h0 = word(arena, ctlkp.wrapping_add(8));
    let ctlkp_h1 = word(arena, ctlkp.wrapping_add(12));
    let ctlkp_vtable = word(arena, ctlkp.wrapping_add(4));
    put_word(arena, object, core);
    put_word(arena, object.wrapping_add(4), lingdb_vtable);
    put_word(arena, object.wrapping_add(8), depes_h0);
    put_word(arena, object.wrapping_add(12), depes_h1);
    put_word(arena, object.wrapping_add(16), depes_vtable);
    put_word(arena, object.wrapping_add(20), ctlkp_h0);
    put_word(arena, object.wrapping_add(24), ctlkp_h1);
    put_word(arena, object.wrapping_add(28), ctlkp_vtable);
    for index in 0..5 {
        put_word(arena, object.wrapping_add(32).wrapping_add(index * 4), 0);
    }
    if let Some(slot) = arena.get_mut(object.wrapping_add(52) as usize) {
        *slot = 0;
    }
    let extra = match host.alloc(arena, heap, 0x80) {
        Some(extra) => extra,
        None => {
            host.release_named(resources, names.wrapping_add(NAME_LINGDB));
            host.release_named(resources, names.wrapping_add(NAME_OUT_FE_DEPES));
            host.release_named(resources, names.wrapping_add(NAME_OUT_FE_DCTLKP));
            host.free(arena, heap, object);
            return 0x8b80_200a;
        }
    };
    put_word(arena, object.wrapping_add(56), extra);
    // The flag scratch: value pointer, length half and separator byte.
    let value_cell = scratch;
    let len_cell = scratch.wrapping_add(4);
    let sep_cell = scratch.wrapping_add(6);
    let status = host.get_string(arena, ctlkp_h0, ctlkp_h1, value_cell, len_cell, sep_cell);
    if status >= 0x8000_0000 {
        close_output(arena, word(arena, out), word(arena, out.wrapping_add(4)), names, validator, host);
        put_word(arena, out, 0);
        put_word(arena, out.wrapping_add(4), 0);
    }
    if tag_is_one(arena, value_cell, len_cell) {
        if let Some(slot) = arena.get_mut(object.wrapping_add(52) as usize) {
            *slot = 1;
        }
    }
    put_word(arena, out, object);
    put_word(arena, out.wrapping_add(4), 0xf3b3);
    status
}

/// Shared `usetntag` test for `fn_1e7318`/`fn_1e7688`: the length half is 1,
/// the value pointer is non-null, and the first byte behind the string slot
/// is `b'1'`.
fn tag_is_one(arena: &[u8], value_cell: u32, len_cell: u32) -> bool {
    if half(arena, len_cell) != 1 {
        return false;
    }
    let value = word(arena, value_cell);
    if value == 0 {
        return false;
    }
    let slot = word(arena, value);
    if slot == 0 {
        return false;
    }
    byte(arena, slot) == b'1'
}

/// fn_1e7688, EU fn_1e7844, KR fn_1e78f0. Re-reads the `usetntag` flag:
/// validates the handle, clears `f34`, and sets it when the CTLKLP
/// `getString` succeeds with a one-character `"1"` value. `scratch` holds
/// the value/length/separator outputs above the object window.
pub fn refresh_output(
    arena: &mut [u8],
    obj: u32,
    key: u32,
    scratch: u32,
    validator: &mut impl HandleValidator,
    host: &mut impl NormOutHost,
) -> u32 {
    let status = validator.validate(key, 0xf3b3, 0x3c);
    if status >= 0x8000_0000 {
        return 0x8b80_2008;
    }
    if obj != 0 {
        if let Some(slot) = arena.get_mut(obj.wrapping_add(52) as usize) {
            *slot = 0;
        }
        let ctlkp_h0 = word(arena, obj.wrapping_add(20));
        let ctlkp_h1 = word(arena, obj.wrapping_add(24));
        let value_cell = scratch;
        let len_cell = scratch.wrapping_add(4);
        let sep_cell = scratch.wrapping_add(6);
        let status = host.get_string(arena, ctlkp_h0, ctlkp_h1, value_cell, len_cell, sep_cell);
        if status < 0x8000_0000 && tag_is_one(arena, value_cell, len_cell) {
            if let Some(slot) = arena.get_mut(obj.wrapping_add(52) as usize) {
                *slot = 1;
            }
        }
        return status;
    }
    status
}

/// One normalization word record for `fn_1e77ec`: 12 bytes.
pub const WORD_RECORD_LEN: u32 = 12;

/// fn_1e77ec, EU fn_1e79a8, KR fn_1e7a54. Shifts word-record spans after an
/// edit at `start`: `start` and `ref_length` are consumed full-width like the
/// ARM (`cmp r0, r7` with no narrowing); only the stored fields are halves.
/// `delta` is the signed half of `current_length - ref_length`. Records live
/// at `records`, `count` of them.
/// fn_1ea35c, EU fn_1ea518, KR fn_1ea5c4. Split `text` on runs of
/// space/`_`/`0x16` (NULling the separators), count the words into `out`,
/// and record one `count + 1` marker per `0x16` byte in the caller buffer
/// when `wbuf` is set (grown by `realloc` past 128 markers). The caller
/// preallocates the buffer: stores below 128 markers never allocate.
/// The `fn_173dc0` call on realloc failure is log-only.
pub fn wordspan_1ea35c(
    arena: &mut [u8],
    ctx: u32,
    text: u32,
    wbuf: u32,
    out: u32,
    host: &mut impl NormalizationHost,
) -> u32 {
    let mut n = 0u16;
    let mut count = 0u16;
    let mut raw = 0u32;
    while byte(arena, text.wrapping_add(raw)) != 0 {
        raw = raw.wrapping_add(1);
    }
    let len = raw as u16;
    let mut i = 0u16;
    put_half(arena, out, 0);
    while i < len {
        let ch = byte(arena, text.wrapping_add(u32::from(i)));
        if ch == b' ' || ch == b'_' || ch == 0x16 {
            if wbuf != 0 {
                if n >= 0x80 {
                    let base = (u32::from(n) >> 7) << 7;
                    let heap = word(arena, word(arena, ctx).wrapping_add(4));
                    let old = word(arena, ctx.wrapping_add(0x38));
                    match host.realloc(arena, heap, old, base + 0x80) {
                        Some(p) => {
                            put_word(arena, ctx.wrapping_add(0x38), p);
                            memset_at(arena, p, 0, base + 0x80);
                        }
                        None => return 0x8b80_200a,
                    }
                }
                if ch == 0x16 {
                    let buf = word(arena, ctx.wrapping_add(0x38));
                    arena[buf.wrapping_add(u32::from(n)) as usize] =
                        count.wrapping_add(1) as u8;
                    n = n.wrapping_add(1);
                }
            }
            arena[text.wrapping_add(u32::from(i)) as usize] = 0;
            let mut k = i.wrapping_add(1);
            while k < len {
                let c = byte(arena, text.wrapping_add(u32::from(k)));
                if c != b' ' && c != b'_' && c != 0x16 {
                    break;
                }
                arena[text.wrapping_add(u32::from(k)) as usize] = 0;
                k = k.wrapping_add(1);
            }
            count = count.wrapping_add(1);
            i = k.wrapping_sub(1);
        }
        i = i.wrapping_add(1);
    }
    if len != 0 && i != 0 && byte(arena, text.wrapping_add(u32::from(i)).wrapping_sub(1)) != 0 {
        count = count.wrapping_add(1);
    }
    put_half(arena, out, count);
    0
}

pub fn shift_word_records(
    arena: &mut [u8],
    records: u32,
    count: u32,
    start: u32,
    end: u32,
    delta: i32,
) {
    // Matches shiftRecs in src/fn_1e7260.c: (start, end, delta) come
    // straight from the fn_1e7908 word-compare pass as
    // (i, i + lorig, (s16)(lo1 - lorig)).
    let delta = (delta as i16) as u16;
    let mut i = 0u16;
    while (i as u32) < count {
        let record = records.wrapping_add((i as u32).wrapping_mul(WORD_RECORD_LEN));
        let kind = byte(arena, record.wrapping_add(2));
        if (kind == 9 || kind == 4 || kind == 10)
            && half(arena, record.wrapping_add(4)) as u32 == start
            && half(arena, record.wrapping_add(6)) as u32 == end
        {
            let anchor = half(arena, record.wrapping_add(8));
            put_half(
                arena,
                record.wrapping_add(10),
                half(arena, record.wrapping_add(10)).wrapping_add(delta),
            );
            let mut j = 0u16;
            while (j as u32) < count {
                if i != j {
                    let other = records.wrapping_add((j as u32).wrapping_mul(WORD_RECORD_LEN));
                    if half(arena, other.wrapping_add(8)) > anchor {
                        let shifted = half(arena, other.wrapping_add(8)).wrapping_add(delta);
                        put_half(arena, other.wrapping_add(8), shifted);
                        if (shifted as i16) < 0 {
                            put_half(arena, other.wrapping_add(8), 0);
                        }
                        put_half(
                            arena,
                            other.wrapping_add(10),
                            half(arena, other.wrapping_add(10)).wrapping_add(delta),
                        );
                    }
                    let other_kind = byte(arena, other.wrapping_add(2));
                    if (other_kind == 5 || other_kind == 6)
                        && half(arena, record.wrapping_add(4)) >= half(arena, other.wrapping_add(4))
                        && half(arena, other.wrapping_add(6)).wrapping_add(1)
                            >= half(arena, record.wrapping_add(6))
                    {
                        put_half(
                            arena,
                            other.wrapping_add(10),
                            half(arena, other.wrapping_add(10)).wrapping_add(delta),
                        );
                    }
                }
                j = j.wrapping_add(1);
            }
        }
        i = i.wrapping_add(1);
    }
}

/// fn_1a9948, EU fn_1a999c, KR fn_1a9a34. Compares two packed words by their
/// high half and second byte.
pub fn compare_packed(a: u32, b: u32) -> u32 {
    let high_a = a >> 16;
    let high_b = b >> 16;
    let low_a = (a << 16) >> 24;
    let low_b = (b << 16) >> 24;
    u32::from(high_a == high_b && low_a <= low_b)
}

/// fn_1baa14, EU fn_1baac4, KR fn_1bac64. Reads the indexed entry of a
/// pointer table.
pub fn table_entry(arena: &[u8], tbl: u32, index: u32) -> u32 {
    let base = word(arena, tbl.wrapping_add(4));
    if base == 0 {
        return 0;
    }
    word(arena, base.wrapping_add(index.wrapping_mul(4)))
}

/// fn_24d504, EU fn_24d6c0, KR fn_24d76c. Logs ObjClose; always succeeds.
pub fn obj_close(_arena: &[u8], _obj: u32) -> u32 {
    0
}

/// fn_250d5c, EU fn_250f18, KR fn_250fc4. Resolves a domain item slot.
pub fn domain_item(arena: &mut [u8], owner: u32, domain: u32, index: u32, out: u32) -> u32 {
    let inner = word(arena, owner.wrapping_add(0x38));
    let domains = word(arena, inner.wrapping_add(8));
    if domains == 0 {
        return error_code(7);
    }
    if domain >= half(arena, domains) as u32 {
        return error_code(25);
    }
    let items = word(arena, domains.wrapping_add(4));
    let entry = word(arena, items.wrapping_add(domain.wrapping_mul(4)));
    let list = word(arena, entry.wrapping_add(8));
    if index >= half(arena, list.wrapping_add(12)) as u32 {
        return error_code(25);
    }
    let base = word(arena, list.wrapping_add(8));
    put_word(arena, out, base.wrapping_add(index.wrapping_mul(28)));
    0
}

/// fn_250e50, EU fn_25100c, KR fn_2510b8. Resolves a domain priority slot.
pub fn domain_priority(
    arena: &mut [u8],
    owner: u32,
    domain: u32,
    priority: u32,
    out: u32,
) -> u32 {
    let state = word(arena, owner.wrapping_add(0x38));
    let list = word(arena, state.wrapping_add(8));
    if list == 0 {
        return error_code(7);
    }
    if domain >= half(arena, list) as u32 {
        return error_code(25);
    }
    let items = word(arena, list.wrapping_add(4));
    let entry = word(arena, items.wrapping_add(domain.wrapping_mul(4)));
    let priorities = word(arena, entry.wrapping_add(8));
    if priority >= half(arena, priorities.wrapping_add(20)) as u32 {
        return error_code(25);
    }
    let base = word(arena, priorities.wrapping_add(16));
    put_word(arena, out, base.wrapping_add(priority.wrapping_mul(8)));
    0
}

/// fn_24d640, EU fn_24d7fc, KR fn_24d8a8. Allocates the two-word entries
/// array.
pub fn alloc_entries(arena: &mut [u8], obj: u32, host: &mut impl NormalizationHost) -> u32 {
    let ctx = word(arena, obj);
    let heap = word(arena, ctx.wrapping_add(4));
    match host.calloc(arena, heap, 1, 8) {
        Some(block) => {
            put_word(arena, obj.wrapping_add(0x3c), block);
            0
        }
        None => error_code(10),
    }
}

/// fn_24e6f0, EU fn_24e8ac, KR fn_24e958. Collects the leading key-matching
/// nodes' data pointers.
pub fn collect_matches(
    arena: &mut [u8],
    obj: u32,
    key: u32,
    host: &mut impl NormalizationHost,
) -> u32 {
    let list = word(arena, obj.wrapping_add(0x44));
    if half(arena, list.wrapping_add(8)) == 0 {
        return 0;
    }
    let key0 = half(arena, key);
    let key1 = half(arena, key.wrapping_add(2));
    let matches = |arena: &[u8], e: u32| {
        half(arena, e.wrapping_add(8)) == key0 && half(arena, e.wrapping_add(10)) == key1
    };
    let mut node = word(arena, list.wrapping_add(4));
    let mut n: u32 = 0;
    while node != 0 {
        if !matches(arena, node) {
            break;
        }
        node = word(arena, node);
        n = n.wrapping_add(1);
    }
    let status = grow_slots_second(arena, obj, n, host);
    if status >= 0x8000_0000 {
        return status;
    }
    let mut node = word(arena, list.wrapping_add(4));
    while node != 0 {
        if !matches(arena, node) {
            break;
        }
        let out = word(arena, obj.wrapping_add(0x80));
        let nout = half(arena, obj.wrapping_add(0x86));
        put_word(
            arena,
            out.wrapping_add((nout as u32).wrapping_mul(4)),
            node.wrapping_add(12),
        );
        put_half(arena, obj.wrapping_add(0x86), nout.wrapping_add(1));
        node = word(arena, node);
    }
    status
}

/// fn_25251c, EU fn_2526d8, KR fn_252784. Releases the tokenizer locator
/// tree.
pub fn close_token_tree(arena: &mut [u8], obj: u32, host: &mut impl NormalizationHost) -> u32 {
    let owner = word(arena, obj);
    let mgr = word(arena, owner.wrapping_add(4));
    let tok = word(arena, obj.wrapping_add(0x40));
    let first = word(arena, tok);
    if first != 0 {
        let sub = word(arena, first.wrapping_add(4));
        if sub != 0 {
            let f4 = word(arena, sub.wrapping_add(4));
            if f4 != 0 {
                host.free(arena, mgr, f4);
                put_word(arena, sub.wrapping_add(4), 0);
            }
            let f0 = word(arena, sub);
            if f0 != 0 {
                host.free(arena, mgr, f0);
                put_word(arena, sub, 0);
            }
            host.free(arena, mgr, sub);
            put_word(arena, first.wrapping_add(4), 0);
        }
        let f0 = word(arena, first);
        if f0 != 0 {
            host.free(arena, mgr, f0);
            put_word(arena, first, 0);
        }
        host.free(arena, mgr, first);
        put_word(arena, tok, 0);
    }
    if tok != 0 {
        host.free(arena, mgr, tok);
        put_word(arena, obj.wrapping_add(0x40), 0);
    }
    0
}

/// fn_25113c, EU fn_2512f8, KR fn_2513a4. Reads a block field plus a record
/// half.
pub fn block_field(arena: &mut [u8], obj: u32, hdr: u32, out: u32) -> u32 {
    let mid = word(arena, obj.wrapping_add(0x38));
    let sub = word(arena, mid.wrapping_add(8));
    if sub == 0 {
        return error_code(7);
    }
    let tab = word(arena, sub.wrapping_add(4));
    let h2 = half(arena, hdr.wrapping_add(2)) as u32;
    let h4 = half(arena, hdr.wrapping_add(4)) as u32;
    let entry = word(arena, tab.wrapping_add(h2.wrapping_mul(4)));
    let bp = word(arena, entry.wrapping_add(4));
    let add = word(arena, bp.wrapping_add(12));
    let rec = word(arena, bp.wrapping_add(24));
    let value = half(
        arena,
        rec.wrapping_add(h4.wrapping_mul(24)).wrapping_add(2),
    ) as u32;
    put_word(arena, out, value.wrapping_add(add));
    0
}

/// fn_2b5cec, EU fn_2b5ea8, KR fn_2b5f54. Opens a domain-defs handle.
pub fn open_domain_handle(
    arena: &mut [u8],
    obj: u32,
    out: u32,
    host: &mut impl NormalizationHost,
) -> u32 {
    if out == 0 {
        return error_code(7);
    }
    let ctx = word(arena, obj);
    let heap = word(arena, ctx.wrapping_add(4));
    put_word(arena, out, 0);
    match host.calloc(arena, heap, 1, 2) {
        Some(handle) => {
            put_half(arena, handle, 0);
            put_word(arena, out, handle);
            0
        }
        None => error_code(10),
    }
}

/// fn_250f5c, EU fn_251118, KR fn_2511c4. Collects table indices matching
/// the id list, or all live indices when `flag` is 1.
pub fn match_ids(
    arena: &mut [u8],
    obj: u32,
    n: u32,
    ids: u32,
    out: u32,
    num: u32,
    flag: u32,
) -> u32 {
    let grp = word(arena, obj.wrapping_add(0x38));
    let tab = word(arena, grp.wrapping_add(8));
    if tab == 0 {
        return error_code(7);
    }
    if flag == 1 {
        let mut cnt = half(arena, tab);
        if cnt != 0 {
            let arr = word(arena, tab.wrapping_add(4));
            let mut i: u32 = 0;
            loop {
                let entry = word(arena, arr.wrapping_add(i.wrapping_mul(4)));
                let live = word(arena, entry.wrapping_add(8));
                let count = half(arena, num);
                if live != 0 && (count as u32).wrapping_add(1) < 500 {
                    put_half(arena, out.wrapping_add((count as u32).wrapping_mul(2)), i as u16);
                    put_half(arena, num, count.wrapping_add(1));
                }
                cnt = cnt.wrapping_sub(1);
                i = i.wrapping_add(1);
                if cnt == 0 {
                    break;
                }
            }
        }
    } else {
        let count = half(arena, tab);
        let arr = word(arena, tab.wrapping_add(4));
        let mut i: u16 = 0;
        while i < count {
            let entry = word(arena, arr.wrapping_add((i as u32).wrapping_mul(4)));
            let val = word(arena, entry.wrapping_add(8));
            let mut j: u16 = 0;
            while val != 0 && (j as u32) < n {
                let id = word(arena, ids.wrapping_add((j as u32).wrapping_mul(4)));
                let name = word(arena, val);
                let total = half(arena, num);
                if strcmp_at(&*arena, id, name) == 0 && (total as u32).wrapping_add(1) < 500 {
                    put_half(arena, out.wrapping_add((total as u32).wrapping_mul(2)), i);
                    put_half(arena, num, total.wrapping_add(1));
                }
                j = j.wrapping_add(1);
            }
            i = i.wrapping_add(1);
        }
    }
    0
}

/// Consumes one `RULE_STUB` script entry, replicating the C/ARM counter.
pub(crate) fn rule_entry(arena: &mut [u8], script: u32, size: u32) -> u32 {
    let n = word(arena, script);
    let idx = if n < 2 { n } else { 1 };
    put_word(arena, script, n.wrapping_add(1));
    script.wrapping_add(4).wrapping_add(idx.wrapping_mul(size))
}

/// fn_24a5d4, EU fn_24a790, KR fn_24a83c. Reads rule value 1, then stores
/// `val` through it.
pub fn rule_store_first(arena: &mut [u8], _obj: u32, _id: u32, _val: u32) -> u32 {
    let e = rule_entry(arena, 0x500, 8);
    let status = word(arena, e);
    let got = if status < 0x8000_0000 {
        word(arena, e.wrapping_add(4)) as u16
    } else {
        0
    };
    if status < 0x8000_0000 && got == 1 {
        let s = rule_entry(arena, 0x514, 8);
        let wrote = if word(arena, s) < 0x8000_0000 {
            word(arena, s.wrapping_add(4)) as u16
        } else {
            0
        };
        if wrote != 1 {
            return error_code(0);
        }
        return 0;
    }
    error_code(0)
}

/// fn_24a6bc, EU fn_24a878, KR fn_24a924. Reads rule value 2, then stores
/// `val` through it.
pub fn rule_store_second(arena: &mut [u8], _obj: u32, _id: u32, _val: u32) -> u32 {
    let e = rule_entry(arena, 0x500, 8);
    let status = word(arena, e);
    let got = if status < 0x8000_0000 {
        word(arena, e.wrapping_add(4)) as u16
    } else {
        0
    };
    if status < 0x8000_0000 && got == 1 {
        let s = rule_entry(arena, 0x514, 8);
        let wrote = if word(arena, s) < 0x8000_0000 {
            word(arena, s.wrapping_add(4)) as u16
        } else {
            0
        };
        if wrote != 1 {
            return error_code(0);
        }
        return 0;
    }
    error_code(0)
}

/// fn_24a7a4, EU fn_24a960, KR fn_24aa0c. Queries a rule, then fetches `b`
/// through it.
pub fn rule_store_query(arena: &mut [u8], _obj: u32, _a: u32, _b: u32) -> u32 {
    let e = rule_entry(arena, 0x500, 8);
    let status = word(arena, e);
    let got = if status < 0x8000_0000 {
        word(arena, e.wrapping_add(4)) as u16
    } else {
        0
    };
    if status < 0x8000_0000 && got == 1 {
        let s = rule_entry(arena, 0x514, 8);
        let wrote = if word(arena, s) < 0x8000_0000 {
            word(arena, s.wrapping_add(4)) as u16
        } else {
            0
        };
        if wrote != 1 {
            return error_code(0);
        }
    }
    0
}

/// fn_24c9fc, EU fn_24cbb8, KR fn_24cc64. Reads the sentence through the
/// tokenizer ops.
pub fn read_sentence(arena: &mut [u8], obj: u32) -> u32 {
    put_word(arena, obj.wrapping_add(0x5c), 0);
    let e = rule_entry(arena, 0x500, 8);
    let status = word(arena, e);
    if status >= 0x8000_0000 {
        return error_code(0);
    }
    let ready = word(arena, e.wrapping_add(4)) as u16;
    if ready != 0 {
        let c = rule_entry(arena, 0x500, 8);
        let check = word(arena, c);
        let single = if check < 0x8000_0000 {
            word(arena, c.wrapping_add(4)) as u16
        } else {
            0
        };
        if check < 0x8000_0000 && single == 1 {
            let x = rule_entry(arena, 0x528, 12);
            let exec = word(arena, x);
            if exec >= 0x8000_0000 {
                return error_code(0);
            }
            put_word(arena, obj.wrapping_add(0x5c), word(arena, x.wrapping_add(4)));
        }
    }
    status
}

/// fn_1b2128, EU fn_1b217c, KR fn_1b2378. Runs the completion callback.
pub fn run_callback(_arena: &[u8], _obj: u32, _arg: u32) {}

/// fn_19703c, EU fn_1970ec, KR fn_197128. Runs element destructors and
/// clears the count.
pub fn run_dtors(arena: &mut [u8], obj: u32) -> u32 {
    if obj == 0 {
        return 0;
    }
    put_word(arena, obj.wrapping_add(20), 0);
    1
}

/// fn_1733a0, EU fn_173450, KR fn_1733fc. Closes the stream or handle side.
pub fn close_stream_handle(arena: &mut [u8], obj: u32) -> u32 {
    if obj == 0 {
        return 0x8060_2007;
    }
    let stream = word(arena, obj.wrapping_add(12));
    if stream != 0 {
        let ops = word(arena, word(arena, obj).wrapping_add(8));
        if word(arena, ops.wrapping_add(0x38)) != 0 {
            let e = rule_entry(arena, 0x548, 4);
            return word(arena, e);
        }
        return 0;
    }
    let handle = word(arena, obj.wrapping_add(16));
    if handle == 0 {
        return 0;
    }
    let e = rule_entry(arena, 0x548, 4);
    let rc = word(arena, e);
    put_word(arena, obj.wrapping_add(16), 0);
    rc
}

/// fn_1b2140, EU fn_1b2194, KR fn_1b2390. Weak default or callback result
/// flag.
pub fn weak_or_callback(arena: &mut [u8], _obj: u32, flag: u32, out: u32) -> u32 {
    if flag == 0 {
        arena[out as usize] = 9;
        return 0;
    }
    let e = rule_entry(arena, 0x548, 4);
    let err = word(arena, e);
    arena[out as usize] = u8::from(err == 0);
    err
}

/// fn_1b218c, EU fn_1b21e0, KR fn_1b23dc. Weak default or 3-arg callback
/// result flag.
pub fn weak_or_callback3(
    arena: &mut [u8],
    _obj: u32,
    _arg: u32,
    flag: u32,
    out: u32,
) -> u32 {
    if flag == 0 {
        arena[out as usize] = 9;
        return 0;
    }
    let e = rule_entry(arena, 0x548, 4);
    let err = word(arena, e);
    arena[out as usize] = u8::from(err == 0);
    err
}

/// fn_17c9d8, EU fn_17ca88, KR fn_17ca34. Allocates and copies a stream
/// span.
pub fn stream_copy(
    arena: &mut [u8],
    obj: u32,
    _a: u32,
    _b: u32,
    c: u32,
    out: u32,
    host: &mut impl NormalizationHost,
) -> u32 {
    let heap = word(arena, word(arena, obj).wrapping_add(4));
    let Some(buf) = host.alloc(arena, heap, c.wrapping_add(1)) else {
        return 0x8c10_200a;
    };
    let read = rule_entry(arena, 0x548, 4);
    if word(arena, read) != 0 {
        host.free(arena, heap, buf);
        return 0x8c10_2004;
    }
    let copy = rule_entry(arena, 0x548, 4);
    if word(arena, copy) != 0 {
        host.free(arena, heap, buf);
        return 0x8c10_2004;
    }
    put_word(arena, out, buf);
    0
}

/// fn_24b540, EU fn_24b6fc, KR fn_24b7a8. Saves an expansion into the rule
/// store.
pub fn save_expansion(
    arena: &mut [u8],
    obj: u32,
    name: u32,
    text: u32,
    offset: u32,
    key: u32,
    token: u32,
) -> u32 {
    if key == 0 {
        let e = rule_entry(arena, 0x500, 8);
        let rc = word(arena, e);
        if rc >= 0x8000_0000 {
            return rc;
        }
        let _first = word(arena, e.wrapping_add(4));
        let f = rule_entry(arena, 0x500, 8);
        let rc = word(arena, f);
        if rc >= 0x8000_0000 {
            return rc;
        }
        put_half(arena, token, word(arena, f.wrapping_add(4)) as u16);
    } else {
        let l = rule_entry(arena, 0x560, 8);
        let rc = word(arena, l);
        if rc >= 0x8000_0000 {
            return rc;
        }
        put_half(arena, token, word(arena, l.wrapping_add(4)) as u16);
    }
    let store = |arena: &mut [u8], want: u16| -> Result<u32, u32> {
        let s = rule_entry(arena, 0x514, 8);
        let rc = word(arena, s);
        if rc >= 0x8000_0000 {
            return Err(rc);
        }
        if word(arena, s.wrapping_add(4)) as u16 != want {
            return Err(error_code(0));
        }
        Ok(rc)
    };
    for want in [1, 1, 1] {
        if let Err(e) = store(arena, want) {
            return e;
        }
    }
    let want = strlen8(arena, name).wrapping_add(1) as u16;
    if let Err(e) = store(arena, want) {
        return e;
    }
    let want = strlen8(arena, text.wrapping_add(offset)).wrapping_add(1) as u16;
    let mut rc = match store(arena, want) {
        Ok(rc) => rc,
        Err(e) => return e,
    };
    let entry_count = half(arena, obj.wrapping_add(0xc4));
    if entry_count > 1 {
        match store(arena, entry_count) {
            Ok(v) => rc = v,
            Err(e) => return e,
        }
    }
    if half(arena, obj.wrapping_add(0x9a)) == 0 {
        return rc;
    }
    let extra = word(arena, obj.wrapping_add(0x94));
    let length = strlen8(arena, extra.wrapping_add(offset));
    if length == 0 {
        return rc;
    }
    match store(arena, length.wrapping_add(1) as u16) {
        Ok(v) => v,
        Err(e) => e,
    }
}

/// fn_173000, EU fn_1730b0, KR fn_17305c. Opens a mapped source object.
pub fn open_mapped(
    arena: &mut [u8],
    src: u32,
    ctx: u32,
    c: u32,
    out: u32,
    host: &mut impl NormalizationHost,
) -> u32 {
    if src == 0 || ctx == 0 || out == 0 || c == 0 {
        return 0x8060_2007;
    }
    let Some(obj) = host.calloc(arena, ctx, 1, 0x20) else {
        put_word(arena, out, 0);
        return 0x8060_200a;
    };
    put_word(arena, out, obj);
    put_word(arena, obj, src);
    put_word(arena, obj.wrapping_add(4), ctx);
    let e = rule_entry(arena, 0x574, 8);
    let mut rc = word(arena, e);
    if rc < 0x8000_0000 {
        put_word(arena, obj.wrapping_add(12), word(arena, e.wrapping_add(4)));
    }
    if rc & 0x1fff == 1 {
        let o = rule_entry(arena, 0x588, 8);
        rc = word(arena, o);
        if rc < 0x8000_0000 {
            put_word(arena, obj.wrapping_add(16), word(arena, o.wrapping_add(4)));
        }
    }
    if rc >= 0x8000_0000 {
        host.free(arena, ctx, obj);
        put_word(arena, out, 0);
    }
    rc
}

/// fn_196b00, EU fn_196bb0, KR fn_196bec. Clones a callback table, running
/// the per-row mapper.
pub fn clone_table(
    arena: &mut [u8],
    tbl: u32,
    out: u32,
    host: &mut impl NormalizationHost,
) -> u32 {
    if tbl == 0 || out == 0 {
        return 0;
    }
    let heap = word(arena, tbl.wrapping_add(28));
    let Some(clone) = host.calloc(arena, heap, 1, 32) else {
        put_word(arena, out, 0);
        return 0;
    };
    put_word(arena, out, clone);
    memcpy_at(arena, clone, tbl, 32);
    let stride = half(arena, clone.wrapping_add(4)) as u32;
    let size = word(arena, clone.wrapping_add(16)).wrapping_mul(stride);
    let buf = host.alloc(arena, heap, size).unwrap_or(0);
    put_word(arena, clone, buf);
    if buf == 0 {
        host.free(arena, heap, clone);
        put_word(arena, out, 0);
        return 0;
    }
    let base = word(arena, tbl);
    let count = word(arena, clone.wrapping_add(20));
    memcpy_at(arena, buf, base, count.wrapping_mul(stride));
    let mut res: u32 = 1;
    if word(arena, clone.wrapping_add(8)) != 0 {
        let mut i: u32 = 0;
        loop {
            if count <= i {
                break;
            }
            let e = rule_entry(arena, 0x548, 4);
            let r = word(arena, e);
            res = r;
            if r != 1 {
                break;
            }
            i = i.wrapping_add(1);
        }
        if res != 0 {
            return res;
        }
    } else {
        return 1;
    }
    host.free(arena, heap, buf);
    host.free(arena, heap, clone);
    put_word(arena, out, 0);
    res
}

/// fn_1731c4, EU fn_173274, KR fn_173220. Fetches a source span into the
/// buffer.
pub fn fetch_span(
    arena: &mut [u8],
    obj: u32,
    want: u32,
    out_len: u32,
    out_buf: u32,
    host: &mut impl NormalizationHost,
) -> u32 {
    if obj == 0 || out_len == 0 || out_buf == 0 {
        return 0x8060_2007;
    }
    if arena[obj.wrapping_add(8) as usize] != 0 {
        return 0x8060_2011;
    }
    put_word(arena, out_buf, 0);
    let res;
    if word(arena, obj.wrapping_add(12)) != 0 {
        let e = rule_entry(arena, 0x59c, 12);
        res = word(arena, e);
        if res >= 0x8000_0000 {
            return res;
        }
        put_word(arena, out_len, word(arena, e.wrapping_add(4)));
        put_word(arena, out_buf, word(arena, e.wrapping_add(8)));
    } else {
        if word(arena, out_len) == 0 {
            let e = rule_entry(arena, 0x548, 4);
            let n = word(arena, e);
            put_word(arena, out_len, n);
            if n <= want {
                return 0x8060_2004;
            }
            put_word(arena, out_len, n.wrapping_sub(want));
        }
        let e = rule_entry(arena, 0x548, 4);
        res = word(arena, e);
        if res >= 0x8000_0000 {
            return res;
        }
        let need = word(arena, out_len);
        if word(arena, obj.wrapping_add(24)) < need {
            let alloc = word(arena, obj.wrapping_add(4));
            let buf = word(arena, obj.wrapping_add(20));
            match host.realloc(arena, alloc, buf, need) {
                Some(t) => {
                    put_word(arena, obj.wrapping_add(20), t);
                    put_word(arena, obj.wrapping_add(24), need);
                }
                None => return 0x8060_200a,
            }
        }
        let e = rule_entry(arena, 0x548, 4);
        let n = word(arena, e);
        if n == 0 || n == 0xffff_ffff {
            return 0x8060_2004;
        }
        put_word(arena, out_len, n);
        put_word(arena, out_buf, word(arena, obj.wrapping_add(20)));
    }
    arena[obj.wrapping_add(8) as usize] = 1;
    res
}

/// fn_17331c, EU fn_1733cc, KR fn_173378. Runs the release method or frees
/// an oversized buffer.
pub fn release_mapped(
    arena: &mut [u8],
    obj: u32,
    arg: u32,
    host: &mut impl NormalizationHost,
) -> u32 {
    if obj == 0 || arg == 0 {
        return 0x8060_2007;
    }
    if arena[obj.wrapping_add(8) as usize] == 0 {
        return 0x8060_2011;
    }
    let handle = word(arena, obj.wrapping_add(12));
    if handle != 0 {
        let e = rule_entry(arena, 0x548, 4);
        let r = word(arena, e);
        if r >= 0x8000_0000 {
            return r;
        }
        arena[obj.wrapping_add(8) as usize] = 0;
        return r;
    }
    if word(arena, obj.wrapping_add(24)) > 0x400 {
        let heap = word(arena, obj.wrapping_add(4));
        host.free(arena, heap, word(arena, obj.wrapping_add(20)));
        put_word(arena, obj.wrapping_add(20), 0);
        put_word(arena, obj.wrapping_add(24), 0);
    }
    arena[obj.wrapping_add(8) as usize] = 0;
    0
}

/// fn_1730fc, EU fn_1731ac, KR fn_173158. Runs the close method, then
/// releases the object.
pub fn close_mapped(
    arena: &mut [u8],
    obj: u32,
    host: &mut impl NormalizationHost,
) -> u32 {
    if obj == 0 {
        return 0x8060_2007;
    }
    let arg_c = word(arena, obj.wrapping_add(12));
    let r = if arg_c != 0 {
        let e = rule_entry(arena, 0x548, 4);
        word(arena, e)
    } else {
        let buf = word(arena, obj.wrapping_add(20));
        if buf != 0 {
            host.free(arena, word(arena, obj.wrapping_add(4)), buf);
        }
        if word(arena, obj.wrapping_add(16)) == 0 {
            0
        } else {
            let e = rule_entry(arena, 0x548, 4);
            word(arena, e)
        }
    };
    if r < 0x8000_0000 {
        host.free(arena, word(arena, obj.wrapping_add(4)), obj);
    }
    r
}

/// fn_17a1f4, EU fn_17a2a4, KR fn_17a250. Freezes broker data: closes the
/// entry handle unless already frozen. Log calls are no-ops.
pub fn data_freeze(arena: &mut [u8], broker: u32, entry: u32) -> u32 {
    if broker == 0 || entry == 0 {
        return 0x8150_6007;
    }
    let mut rc = 0;
    if word(arena, entry.wrapping_add(4)) == 0 {
        rc = close_stream_handle(arena, word(arena, entry));
    }
    rc
}

/// fn_1ba85c, EU fn_1ba90c, KR fn_1baaac. Appends `val` to the u32 vector,
/// growing through `fn_1b218c` (inlined: its callback is the `rule_ptrA`
/// script in this context, returning the new buffer or 0).
pub fn vec_push(arena: &mut [u8], v: u32, val: u32) -> u32 {
    let count = word(arena, v.wrapping_add(8));
    let cap = word(arena, v.wrapping_add(12));
    let need = count.wrapping_add(1);
    let mut status: u8 = 0;
    if cap < need {
        let mut newcap = cap.wrapping_add(word(arena, v.wrapping_add(16)));
        if newcap < need {
            newcap = need;
        }
        if newcap.wrapping_mul(4) == 0 {
            status = 9;
        } else {
            let e = rule_entry(arena, 0x5b0, 4);
            let off = word(arena, e);
            if off == 0 {
                status = 1;
            } else {
                put_word(arena, v.wrapping_add(4), off);
                put_word(arena, v.wrapping_add(12), newcap);
            }
        }
    }
    if status != 0 {
        return status as u32;
    }
    let i = word(arena, v.wrapping_add(8));
    let base = word(arena, v.wrapping_add(4));
    put_word(arena, v.wrapping_add(8), i.wrapping_add(1));
    put_word(arena, base.wrapping_add(i.wrapping_mul(4)), val);
    0
}

/// fn_24aa08, EU fn_24abc4, KR fn_24ac70. Checks whether `range` bisects a
/// basic token. `start`/`end`/`kind` stay 0: the real `fn_24a5d4`/`fn_24a6bc`
/// take their value argument by value and never write it back, so the
/// bisect test can never fire and this always returns 0. The ops calls and
/// rule scripts are still consumed in order.
pub fn range_bisect_check(arena: &mut [u8], p: u32, range: u32) -> u32 {
    if p == 0 {
        return 0;
    }
    let e = rule_entry(arena, 0x500, 8);
    if word(arena, e) >= 0x8000_0000 {
        return 0;
    }
    let e = rule_entry(arena, 0x500, 8);
    if word(arena, e) != 0 {
        return 0;
    }
    let mut token = word(arena, e.wrapping_add(4)) as u16;
    if token == 0 {
        return 0;
    }
    let range1 = half(arena, range.wrapping_add(2)) as i16;
    loop {
        if rule_store_first(arena, p, token as u32, 0) >= 0x8000_0000 {
            break;
        }
        if range1 < 0 {
            break;
        }
        if rule_store_second(arena, p, token as u32, 0) >= 0x8000_0000 {
            break;
        }
        if rule_store_query(arena, p, token as u32, 0) >= 0x8000_0000 {
            break;
        }
        let e = rule_entry(arena, 0x560, 8);
        if word(arena, e) != 0 {
            break;
        }
        token = word(arena, e.wrapping_add(4)) as u16;
        if token == 0 {
            break;
        }
    }
    0
}

/// fn_24e1a0, EU fn_24e35c, KR fn_24e408. Frees runtime match entries,
/// zeroes the table header, and runs destructors when `mode` is 1. Log and
/// release calls are no-ops.
pub fn free_matches(arena: &mut [u8], p: u32, mode: u32) -> u32 {
    let tbl = word(arena, p.wrapping_add(68));
    let ents = word(arena, tbl.wrapping_add(12));
    if ents != 0 {
        let count = half(arena, tbl.wrapping_add(16));
        let mut i: u16 = 1;
        while i < count {
            let slot = ents.wrapping_add((i as u32).wrapping_mul(12));
            if word(arena, slot) != 0 {
                put_word(arena, slot, 0);
            }
            i = i.wrapping_add(1);
        }
        if count > 1 {
            let start = ents.wrapping_add(12) as usize;
            let len = ((count as u32).wrapping_sub(1)).wrapping_mul(12) as usize;
            arena[start..start.wrapping_add(len)].fill(0);
            put_half(arena, tbl.wrapping_add(16), 1);
        }
        put_half(arena, tbl.wrapping_add(22), 0);
    }
    put_word(arena, tbl.wrapping_add(4), 0);
    put_half(arena, tbl.wrapping_add(8), 0);
    if mode == 1 {
        run_dtors(arena, word(arena, tbl));
    }
    0
}

/// fn_24b290, EU fn_24b44c, KR fn_24b4f8. Finds the token node for
/// (`position`, `index`). `nodeIndex`/`nodePosition`/`type` stay 0: the real
/// `fn_24a5d4`/`fn_24a6bc`/`fn_24a7a4` take their value argument by value and
/// never write it back.
pub fn find_token_node(
    arena: &mut [u8],
    text: u32,
    position: u32,
    index: u32,
    result: u32,
) -> u32 {
    if text == 0 {
        return error_code(7);
    }
    put_half(arena, result, 0);
    let e = rule_entry(arena, 0x500, 8);
    let mut rc = word(arena, e);
    if rc >= 0x8000_0000 {
        return rc;
    }
    let e = rule_entry(arena, 0x500, 8);
    rc = word(arena, e);
    if rc != 0 {
        return rc;
    }
    let mut node = word(arena, e.wrapping_add(4)) as u16;
    while node != 0 {
        rc = rule_store_second(arena, text, node as u32, 0);
        if rc >= 0x8000_0000 {
            return rc;
        }
        rc = rule_store_first(arena, text, node as u32, 0);
        if rc >= 0x8000_0000 {
            return rc;
        }
        rc = rule_store_query(arena, text, node as u32, 0);
        if rc >= 0x8000_0000 {
            return rc;
        }
        if index == 0 {
            put_half(arena, result, node);
            loop {
                let e = rule_entry(arena, 0x560, 8);
                if word(arena, e) != 0 {
                    break;
                }
                node = word(arena, e.wrapping_add(4)) as u16;
                if node == 0 {
                    break;
                }
                rc = rule_store_second(arena, text, node as u32, 0);
                if rc >= 0x8000_0000 {
                    return rc;
                }
                rc = rule_store_first(arena, text, node as u32, 0);
                if rc >= 0x8000_0000 {
                    return rc;
                }
                rc = rule_store_query(arena, text, node as u32, 0);
                if rc >= 0x8000_0000 {
                    return rc;
                }
                if position > 0 {
                    put_half(arena, result, node);
                    return rc;
                }
            }
            break;
        }
        if position == 0 {
            loop {
                put_half(arena, result, node);
                let e = rule_entry(arena, 0x560, 8);
                if word(arena, e) != 0 {
                    break;
                }
                node = word(arena, e.wrapping_add(4)) as u16;
                if node == 0 {
                    break;
                }
                rc = rule_store_second(arena, text, node as u32, 0);
                if rc >= 0x8000_0000 {
                    return rc;
                }
                rc = rule_store_first(arena, text, node as u32, 0);
                if rc >= 0x8000_0000 {
                    return rc;
                }
                if position != 0 {
                    break;
                }
                if index == 0 {
                    break;
                }
            }
            break;
        }
        let e = rule_entry(arena, 0x560, 8);
        if word(arena, e) != 0 {
            break;
        }
        node = word(arena, e.wrapping_add(4)) as u16;
    }
    rc
}

/// fn_244324, EU fn_2444e0, KR fn_24458c. Closes the owner handle pair,
/// releasing the map through either handle. The broker calls are platform
/// stubs returning 0.
pub fn close_owner_pair(arena: &mut [u8], p: u32, host: &mut impl NormalizationHost) -> u32 {
    if arena[p.wrapping_add(4) as usize] != 0 {
        put_word(arena, p.wrapping_add(8), 0);
        put_word(arena, p.wrapping_add(12), 0);
        put_word(arena, p.wrapping_add(16), 0);
        put_word(arena, p.wrapping_add(20), 0);
        put_word(arena, p.wrapping_add(24), 0);
        arena[p.wrapping_add(4) as usize] = 0;
        return 0;
    }
    let mut rc = 0;
    if word(arena, p.wrapping_add(16)) != 0 {
        let h1 = word(arena, p.wrapping_add(8));
        let h2 = word(arena, p.wrapping_add(12));
        let err = if h1 != 0 {
            0
        } else if h2 != 0 {
            release_mapped(arena, h2, word(arena, p.wrapping_add(16)), host)
        } else {
            0
        };
        if h1 != 0 || h2 != 0 {
            put_word(arena, p.wrapping_add(16), 0);
            put_word(arena, p.wrapping_add(24), 0);
            put_word(arena, p.wrapping_add(20), 0);
        }
        put_word(arena, p.wrapping_add(16), 0);
        rc = err;
        if rc >= 0x8000_0000 {
            return err;
        }
    }
    if word(arena, p.wrapping_add(8)) != 0 {
        rc = 0;
        put_word(arena, p.wrapping_add(8), 0);
    }
    if rc >= 0x8000_0000 {
        return rc;
    }
    let h2 = word(arena, p.wrapping_add(12));
    if h2 == 0 {
        return rc;
    }
    rc = close_mapped(arena, h2, host);
    put_word(arena, p.wrapping_add(12), 0);
    rc
}

/// fn_24b8d0, EU fn_24ba8c, KR fn_24bb38. Deletes tokens subsumed by `range`.
/// `start`/`end`/`kind` stay 0: the real `fn_24a5d4`/`fn_24a6bc`/`fn_24a7a4`
/// take their value argument by value and never write it back.
pub fn delete_subsumed_tokens(arena: &mut [u8], ctx: u32, range: u32) -> u32 {
    if ctx == 0 {
        return error_code(7);
    }
    let e = rule_entry(arena, 0x500, 8);
    let mut rc = word(arena, e);
    if rc >= 0x8000_0000 {
        return rc;
    }
    let e = rule_entry(arena, 0x500, 8);
    rc = word(arena, e);
    if rc != 0 {
        return rc;
    }
    let mut current = word(arena, e.wrapping_add(4)) as u16;
    if current == 0 {
        return rc;
    }
    let mut previous: u32 = 0;
    let r0 = half(arena, range) as i16;
    let r1 = half(arena, range.wrapping_add(2)) as i16;
    loop {
        rc = rule_store_first(arena, ctx, current as u32, 0);
        if rc >= 0x8000_0000 || r1 <= 0 {
            break;
        }
        rc = rule_store_second(arena, ctx, current as u32, 0);
        if rc >= 0x8000_0000 {
            break;
        }
        rc = rule_store_query(arena, ctx, current as u32, 0);
        if rc >= 0x8000_0000 {
            break;
        }
        if r0 <= 0 && r1 >= 0 {
            let e = rule_entry(arena, 0x548, 4);
            if word(arena, e) != 0 {
                break;
            }
            if previous == 0 {
                let e = rule_entry(arena, 0x500, 8);
                rc = word(arena, e);
                if rc != 0 {
                    break;
                }
                current = word(arena, e.wrapping_add(4)) as u16;
                continue;
            }
            current = previous as u16;
        } else {
            previous = current as u32;
        }
        let e = rule_entry(arena, 0x560, 8);
        if word(arena, e) != 0 {
            break;
        }
        current = word(arena, e.wrapping_add(4)) as u16;
        if current == 0 {
            break;
        }
    }
    rc
}

/// fn_2b89bc, EU fn_2b8b78, KR fn_2b8c24. Resolves the winning match by
/// domain priority, then by priority lists. Log calls are no-ops; the final
/// conflict scan only logs. `out` doubles as scratch for the `domain_item`
/// / `domain_priority` out-words (only its low half is observed).
pub fn resolve_priority(
    arena: &mut [u8],
    out: u32,
    this: u32,
    host: &mut impl NormalizationHost,
) -> u32 {
    let mut rc = grow_slots_first(arena, this, half(arena, this.wrapping_add(0x86)) as u32, host);
    if rc >= 0x8000_0000 {
        return rc;
    }
    let mut highest: u8 = 0;
    let mut i: u16 = 0;
    while i < half(arena, this.wrapping_add(0x86)) {
        let matches = word(arena, this.wrapping_add(0x80));
        let m = word(arena, matches.wrapping_add((i as u32).wrapping_mul(4)));
        let mut domain = 0;
        rc = domain_value(arena, this, half(arena, m.wrapping_add(0x58)) as u32, &mut domain);
        if rc >= 0x8000_0000 {
            return rc;
        }
        let resolutions = word(arena, this.wrapping_add(0x78));
        let pri = arena[domain.wrapping_add(4) as usize];
        arena[resolutions.wrapping_add((i as u32).wrapping_mul(4)).wrapping_add(2) as usize] = pri;
        if highest < pri {
            highest = pri;
        }
        i = i.wrapping_add(1);
    }
    let mut nwin: u16 = 0;
    let mut domain_winner: u16 = 999;
    i = 0;
    while i < half(arena, this.wrapping_add(0x86)) {
        let resolutions = word(arena, this.wrapping_add(0x78));
        if arena[resolutions.wrapping_add((i as u32).wrapping_mul(4)).wrapping_add(2) as usize]
            == highest
        {
            nwin = nwin.wrapping_add(1);
            domain_winner = i;
        }
        i = i.wrapping_add(1);
    }
    if nwin == 1 {
        put_half(arena, out, domain_winner);
        return 0;
    }
    let mut winner: u16 = 999;
    let mut previous: u32 = 0;
    i = 0;
    while i < half(arena, this.wrapping_add(0x86)) {
        let resolutions = word(arena, this.wrapping_add(0x78));
        put_half(arena, resolutions.wrapping_add((i as u32).wrapping_mul(4)), 999);
        let matches = word(arena, this.wrapping_add(0x80));
        let m = word(arena, matches.wrapping_add((i as u32).wrapping_mul(4)));
        domain_item(
            arena,
            this,
            half(arena, m.wrapping_add(0x58)) as u32,
            half(arena, m.wrapping_add(0x5a)) as u32,
            out,
        );
        let info = word(arena, out);
        if arena[info.wrapping_add(6) as usize] == 1 {
            domain_priority(
                arena,
                this,
                half(arena, m.wrapping_add(0x58)) as u32,
                half(arena, info.wrapping_add(8)) as u32,
                out,
            );
            let list = word(arena, out);
            if list != 0 && list != previous {
                let mut best = half(arena, list.wrapping_add(4));
                let mut j: u16 = 0;
                while j < half(arena, this.wrapping_add(0x86)) {
                    let mj = word(arena, matches.wrapping_add((j as u32).wrapping_mul(4)));
                    let tok = word(arena, mj.wrapping_add(0x54));
                    let jt = half(arena, tok.wrapping_add(2));
                    let jv = half(arena, tok.wrapping_add(4));
                    let tokens = word(arena, list);
                    let lcount = half(arena, list.wrapping_add(4));
                    let mut k: u16 = 0;
                    while k < lcount {
                        let tk = tokens.wrapping_add((k as u32).wrapping_mul(6));
                        if half(arena, tk.wrapping_add(2)) == jt
                            && half(arena, tk.wrapping_add(4)) == jv
                        {
                            if k < best {
                                let resolutions = word(arena, this.wrapping_add(0x78));
                                put_half(
                                    arena,
                                    resolutions.wrapping_add((i as u32).wrapping_mul(4)),
                                    j,
                                );
                                winner = j;
                                best = k;
                            }
                            break;
                        }
                        k = k.wrapping_add(1);
                    }
                    j = j.wrapping_add(1);
                }
            }
            previous = list;
        }
        i = i.wrapping_add(1);
    }
    if winner != 999 {
        put_half(arena, out, winner);
    } else {
        put_half(arena, out, 0);
    }
    rc
}

/// fn_197168, EU fn_197218, KR fn_197254. Writes the address of the last
/// element (`data + (count - 1) * stride`) through `output`; returns 1,
/// or 0 when the array, output, or count is null.
pub fn chunk_last(arena: &mut [u8], array: u32, output: u32) -> u32 {
    if array == 0 || output == 0 {
        return 0;
    }
    let count = word(arena, array.wrapping_add(0x14));
    if count == 0 {
        return 0;
    }
    let data = word(arena, array);
    let stride = half(arena, array.wrapping_add(4)) as u32;
    put_word(
        arena,
        output,
        data.wrapping_add((count - 1).wrapping_mul(stride)),
    );
    1
}

/// fn_1b3c00, EU fn_1b3c54, KR fn_1b3e50. Always returns 1.
pub fn code_1(_value: u32) -> u32 {
    1
}

/// fn_1b4248, EU fn_1b429c, KR fn_1b4498. Always returns 4.
pub fn code_4(_msg: u32) -> u32 {
    4
}

/// fn_1b4250, EU fn_1b42a4, KR fn_1b44a0. Always returns 6.
pub fn code_6(_msg: u32) -> u32 {
    6
}

/// fn_1b4258, EU fn_1b42ac, KR fn_1b44a8. Always returns 7.
pub fn code_7(_msg: u32) -> u32 {
    7
}

/// fn_1a8684, EU fn_1a86d8, KR fn_1a8770. Returns 1 when `x` is 0, 1, or 2.
pub fn range_0_2(x: u32) -> u32 {
    let x = x as i32;
    if x < 0 {
        return 0;
    }
    if x <= 2 {
        return 1;
    }
    0
}

/// fn_1b3580, EU fn_1b35d4, KR fn_1b37d0. Returns 1 when the words behind
/// the two double-indirected pointers differ.
pub fn dbl_deref_ne(arena: &[u8], a: u32, b: u32) -> u32 {
    let x = word(arena, word(arena, a));
    let y = word(arena, word(arena, b));
    u32::from(x != y)
}

/// fn_1ba3e0, EU fn_1ba490, KR fn_1ba630. Adler-32 over `len` bytes at
/// `buf`, seeded with `adler`; returns 1 for a null buffer.
pub fn adler32(arena: &[u8], adler: u32, buf: u32, len: u32) -> u32 {
    const BASE: u32 = 65521;
    const NMAX: u32 = 5552;
    if buf == 0 {
        return 1;
    }
    let mut s1 = adler & 0xffff;
    let mut s2 = (adler >> 16) & 0xffff;
    let mut rest = len;
    let mut at = buf;
    if rest < 16 {
        while rest != 0 {
            rest -= 1;
            s1 = s1.wrapping_add(arena[at as usize] as u32);
            s2 = s2.wrapping_add(s1);
            at = at.wrapping_add(1);
        }
        if s1 >= BASE {
            s1 -= BASE;
        }
        s2 %= BASE;
        return (s2 << 16) | s1;
    }
    while rest >= NMAX {
        rest -= NMAX;
        let mut n = NMAX / 16;
        while n != 0 {
            n -= 1;
            for _ in 0..16 {
                s1 = s1.wrapping_add(arena[at as usize] as u32);
                s2 = s2.wrapping_add(s1);
                at = at.wrapping_add(1);
            }
        }
        s1 %= BASE;
        s2 %= BASE;
    }
    if rest != 0 {
        while rest >= 16 {
            rest -= 16;
            for _ in 0..16 {
                s1 = s1.wrapping_add(arena[at as usize] as u32);
                s2 = s2.wrapping_add(s1);
                at = at.wrapping_add(1);
            }
        }
        while rest != 0 {
            rest -= 1;
            s1 = s1.wrapping_add(arena[at as usize] as u32);
            s2 = s2.wrapping_add(s1);
            at = at.wrapping_add(1);
        }
        s1 %= BASE;
        s2 %= BASE;
    }
    (s2 << 16) | s1
}

/// fn_1970dc, EU fn_19718c, KR fn_1971c8. Pops the last chunk: decrements
/// the count at `array + 0x14` and returns 1, or 0 when the array or its
/// count is null. The release callback is the `rule_void` no-op in this
/// context, and the tail move length is always zero (`count - index - 1`
/// with `index = count - 1`), so neither has an observable effect.
pub fn chunk_pop(arena: &mut [u8], array: u32) -> u32 {
    if array == 0 {
        return 0;
    }
    let count = word(arena, array.wrapping_add(0x14));
    if count == 0 {
        return 0;
    }
    put_word(arena, array.wrapping_add(0x14), count - 1);
    1
}

/// fn_1ba8e0, EU fn_1ba990, KR fn_1bab30. Linear search over the entry
/// table: returns the first entry for which the comparator replies 0, or
/// 0 when none matches. The comparator is the `rule_stA` script here,
/// popped once per entry in order.
pub fn vec_find(arena: &mut [u8], v: u32, _key: u32, _cmp: u32) -> u32 {
    let entries = word(arena, v.wrapping_add(4));
    let count = word(arena, v.wrapping_add(8));
    let mut i = 0;
    while i < count {
        let val = word(arena, entries.wrapping_add(i.wrapping_mul(4)));
        let e = rule_entry(arena, 0x548, 4);
        if word(arena, e) == 0 {
            return val;
        }
        i = i.wrapping_add(1);
    }
    0
}

/// fn_176fc0, EU fn_177070, KR fn_17701c. Returns 1 when both halves match.
pub fn pair_eq(al: u32, ah: u32, bl: u32, bh: u32) -> u32 {
    u32::from(ah == bh && al == bl)
}

/// fn_1970a0, EU fn_197150, KR fn_19718c. Reads word index 5, or 0 for
/// a null pointer.
pub fn idx5(arena: &[u8], p: u32) -> u32 {
    if p == 0 {
        return 0;
    }
    word(arena, p.wrapping_add(20))
}

/// fn_1970ac, EU fn_19715c, KR fn_197198. Writes `base + idx * size`
/// through `out`; 1 on success, 0 when a pointer is null or `idx` is out
/// of range.
pub fn span_at(arena: &mut [u8], s: u32, idx: u32, out: u32) -> u32 {
    if s == 0 || out == 0 {
        return 0;
    }
    if idx >= word(arena, s.wrapping_add(0x14)) {
        return 0;
    }
    let base = word(arena, s);
    let size = half(arena, s.wrapping_add(4)) as u32;
    put_word(arena, out, base.wrapping_add(idx.wrapping_mul(size)));
    1
}

/// fn_198fa8, EU fn_199058, KR fn_199094. Sets the flag byte at `p + 0x28`.
pub fn flag_set(arena: &mut [u8], p: u32) {
    arena[p.wrapping_add(0x28) as usize] = 1;
}

/// fn_196394, EU fn_196444, KR fn_196480. Calls the slot-0 vtable method
/// with (`obj`, `x`); 0 when either pointer is null. The method is the
/// `rule_ptrA` script here, returning an arena offset or 0.
pub fn vt_call0(arena: &mut [u8], s: u32, x: u32) -> u32 {
    if s == 0 || x == 0 {
        return 0;
    }
    let e = rule_entry(arena, 0x5b0, 4);
    word(arena, e)
}

/// fn_1963b8, EU fn_196468, KR fn_1964a4. Calls the slot-1 vtable method
/// with (`obj`, `a`, `b`); 0 when any pointer is null. The method is the
/// `rule_stA` script here.
pub fn vt_call1(arena: &mut [u8], p: u32, a: u32, b: u32) -> u32 {
    if p == 0 || a == 0 || b == 0 {
        return 0;
    }
    let e = rule_entry(arena, 0x548, 4);
    word(arena, e)
}

/// fn_1b20a4, EU fn_1b20f8, KR fn_1b22f4. Copies the 16-byte quad, then
/// the trailing word; always returns 0.
pub fn quad_copy(arena: &mut [u8], dst: u32, src: u32, e: u32) -> u32 {
    let (s, d) = (src as usize, dst as usize);
    arena.copy_within(s..s + 16, d);
    put_word(arena, dst.wrapping_add(16), e);
    0
}

/// fn_1963e0, EU fn_196490, KR fn_1964cc. Dispatches slot 2 with
/// (`self`, `a`, `n`) when `a` is non-null, else slot 0 with (`self`,
/// `n`); 0 when `o` is null or `n` is 0. Both slots are the `rule_stA`
/// script here.
pub fn vt_call2(arena: &mut [u8], o: u32, a: u32, n: u32) -> u32 {
    if o == 0 || n == 0 {
        return 0;
    }
    if a == 0 {
        let e = rule_entry(arena, 0x548, 4);
        return word(arena, e);
    }
    let e = rule_entry(arena, 0x548, 4);
    word(arena, e)
}

/// fn_198f74, EU fn_199024, KR fn_199060. Returns the last node of the
/// circular list, or 0 when the head links to itself.
pub fn list_last(arena: &[u8], head: u32) -> u32 {
    let mut p = word(arena, head);
    if p != head {
        let mut q = word(arena, p);
        if q != head {
            loop {
                p = q;
                q = word(arena, q);
                if q == head {
                    break;
                }
            }
        }
    }
    if p == head {
        0
    } else {
        p
    }
}

/// fn_176f2c, EU fn_176fdc, KR fn_176f88. Calls the inner-table slot-2
/// method with the target; `0x81402007` for a null `p`, 0 for a null
/// slot. The slot is the `rule_stA` script here.
pub fn inner_call_2(arena: &mut [u8], p: u32) -> u32 {
    if p == 0 {
        return 0x8140_2007;
    }
    let mid = word(arena, word(arena, p).wrapping_add(8));
    if word(arena, mid.wrapping_add(8)) == 0 {
        return 0;
    }
    let e = rule_entry(arena, 0x548, 4);
    word(arena, e)
}

/// fn_176f5c, EU fn_17700c, KR fn_176fb8. Calls the inner-table slot-3
/// method with the target; `0x81402007` for a null `p`, 0 for a null
/// slot. The slot is the `rule_stA` script here.
pub fn inner_call_3(arena: &mut [u8], p: u32) -> u32 {
    if p == 0 {
        return 0x8140_2007;
    }
    let mid = word(arena, word(arena, p).wrapping_add(8));
    if word(arena, mid.wrapping_add(12)) == 0 {
        return 0;
    }
    let e = rule_entry(arena, 0x548, 4);
    word(arena, e)
}

/// fn_2403f8, EU fn_2405b4, KR fn_240660. Maps a status code to its
/// error id.
pub fn err_code(code: u32) -> u32 {
    match code as i32 {
        -2 => 0x8a50_210d,
        -5 | -4 => 0x8a50_3e66,
        -6 => 0x8a50_200a,
        -7 | -1 => 0x8a50_2014,
        -8 => 0x8a50_3e60,
        -9 => 0x8a50_3e61,
        -10 => 0x8a50_2105,
        -11 => 0x8a50_3e65,
        -12 => 0x8a50_3e62,
        -13 => 0x8a50_3e63,
        -14 => 0x8a50_3e64,
        -15 | -3 => 0x8a50_2007,
        -16 => 0x8a50_2008,
        _ => 0x8a50_3e67,
    }
}

/// fn_198728, EU fn_1987d8, KR fn_198814. Initializes the node: clears
/// the accumulators, sets the flag, stores `a`/`b`/`c`, and links all
/// three list pointers to itself.
pub fn node_init(arena: &mut [u8], p: u32, a: u32, b: u32, c: u32) {
    put_word(arena, p.wrapping_add(24), 0);
    arena[p.wrapping_add(12) as usize] = 1;
    arena[p.wrapping_add(40) as usize] = 0;
    put_word(arena, p.wrapping_add(36), b);
    put_word(arena, p, p);
    put_word(arena, p.wrapping_add(28), a);
    put_word(arena, p.wrapping_add(32), c);
    put_word(arena, p.wrapping_add(4), p);
    put_word(arena, p.wrapping_add(8), p);
}

/// fn_195e38, EU fn_195ee8, KR fn_195f24. Copies at most `n` bytes,
/// stopping after the first NUL and zero-filling the tail; returns `dst`.
pub fn strn_copy(arena: &mut [u8], dst: u32, src: u32, n: u32) -> u32 {
    let mut d = dst;
    let mut s = src;
    let mut n = n;
    loop {
        if n == 0 {
            return dst;
        }
        let b = arena[s as usize];
        arena[d as usize] = b;
        d = d.wrapping_add(1);
        s = s.wrapping_add(1);
        if b == 0 {
            break;
        }
        n -= 1;
    }
    if n > 0x7fff_ffff {
        n -= 1;
        while n != 0 {
            arena[d as usize] = 0;
            d = d.wrapping_add(1);
            n -= 1;
        }
    } else {
        let mut i = (n as i32).wrapping_sub(1);
        if i > 0 {
            while i != 0 {
                arena[d as usize] = 0;
                d = d.wrapping_add(1);
                i -= 1;
            }
        }
    }
    dst
}

/// fn_196e28, EU fn_196ed8, KR fn_196f14. Removes element `idx`,
/// shifting the tail down and decrementing the count; 1 on success, 0
/// when the array is null or `idx` is out of range. The destructor is
/// the `rule_void` no-op in this context.
pub fn array_remove(arena: &mut [u8], a: u32, idx: u32) -> u32 {
    let count = if a == 0 {
        0
    } else {
        word(arena, a.wrapping_add(0x14))
    };
    if a == 0 || count <= idx {
        return 0;
    }
    let rest = (count - idx - 1) as usize;
    if rest != 0 {
        let elem = half(arena, a.wrapping_add(4)) as usize;
        let data = word(arena, a) as usize;
        let dst = data + (idx as usize) * elem;
        let src = data + ((idx as usize) + 1) * elem;
        arena.copy_within(src..src + elem * rest, dst);
    }
    put_word(arena, a.wrapping_add(0x14), count - 1);
    1
}

/// fn_1b3b3c, EU fn_1b3b90, KR fn_1b3d8c. Searches the vector for `key`
/// with the `fn_1b3580` comparator and returns the found cell's indirect
/// word, or its direct word when the indirection is null.
pub fn vec_key_get(arena: &[u8], p: u32, key: u32) -> u32 {
    let vec = word(arena, p.wrapping_add(8));
    let entries = word(arena, vec.wrapping_add(4));
    let count = word(arena, vec.wrapping_add(8));
    let mut i = 0;
    while i < count {
        let found = word(arena, entries.wrapping_add(i.wrapping_mul(4)));
        if word(arena, found) == key {
            let q = word(arena, found.wrapping_add(12));
            if q != 0 {
                return word(arena, q);
            }
            return word(arena, found.wrapping_add(8));
        }
        i = i.wrapping_add(1);
    }
    0
}

/// fn_1b3fe8, EU fn_1b403c, KR fn_1b4238. Searches the vector for `key`
/// with the `fn_1b3580` comparator and stores `value` through the found
/// cell's indirection, or into its direct word when the indirection is
/// null.
pub fn vec_key_set(arena: &mut [u8], p: u32, key: u32, value: u32) {
    let vec = word(arena, p.wrapping_add(8));
    let entries = word(arena, vec.wrapping_add(4));
    let count = word(arena, vec.wrapping_add(8));
    let mut i = 0;
    while i < count {
        let found = word(arena, entries.wrapping_add(i.wrapping_mul(4)));
        if word(arena, found) == key {
            let f12 = word(arena, found.wrapping_add(12));
            if f12 != 0 {
                put_word(arena, f12, value);
            } else {
                put_word(arena, found.wrapping_add(8), value);
            }
            return;
        }
        i = i.wrapping_add(1);
    }
}

/// fn_176cd8, EU fn_176d88, KR fn_176d34. Parses a decimal integer with
/// leading blank skipping and an optional `+`; returns 0 for a null
/// pointer, garbage, or an empty digit run.
pub fn atoi_scan(arena: &[u8], s: u32) -> u32 {
    if s == 0 {
        return 0;
    }
    let s = s as usize;
    let mut len: i32 = 0;
    while arena[s + len as usize] != 0 {
        len += 1;
    }
    let last = len - 1;
    let mut start: i32 = 0;
    while start <= last
        && (arena[s + start as usize] == b' ' || arena[s + start as usize] == b'\t')
    {
        start += 1;
    }
    let mut i = start;
    if start < last && arena[s + i as usize] == b'+' {
        i = start + 1;
    }
    while i <= last && arena[s + i as usize] >= b'0' && arena[s + i as usize] <= b'9' {
        i += 1;
    }
    i -= 1;
    let n = i - start + 2;
    let mut k = 1;
    let mut val: i32 = 0;
    let mut mul: i32 = 1;
    if n > 1 {
        loop {
            let d = (arena[s + i as usize] as u32).wrapping_sub(0x30);
            if d <= 9 {
                val = val.wrapping_add((d as i32).wrapping_mul(mul));
                mul = mul.wrapping_mul(10);
            } else if i != start || arena[s + i as usize] != b'+' {
                return 0;
            }
            k += 1;
            i -= 1;
            if n <= k {
                break;
            }
        }
    }
    val as u32
}

/// fn_1987fc, EU fn_1988ac, KR fn_1988e8. Walks the keyed tree with the
/// comparator, returning the match, the last lesser node, or 0. The
/// comparator is the `rule_stA` script here, read as a signed result.
pub fn tree_search(arena: &mut [u8], a: u32, _b: u32) -> u32 {
    let mut cur = word(arena, a);
    let mut prev = 0;
    let flag = arena[a.wrapping_add(0x28) as usize];
    while cur != a {
        let e = rule_entry(arena, 0x548, 4);
        let r = word(arena, e) as i32;
        if r > 0 {
            cur = word(arena, cur.wrapping_add(4));
        } else if r < 0 || flag != 0 {
            prev = cur;
            cur = word(arena, cur);
        } else {
            return cur;
        }
    }
    prev
}

/// fn_173fd8, EU fn_174088, KR fn_174034. Returns the maximum reply over
/// the five slot callbacks and the trailing callback (floor 1, clamped
/// to 4 when the clamp byte is set); 0 for a null `p`. Every live slot
/// is the `rule_stA` script here.
pub fn slot_max(arena: &mut [u8], p: u32) -> u32 {
    if p == 0 {
        return 0;
    }
    let mut max = 1u32;
    let mut i = 0u32;
    loop {
        let vt = word(arena, p.wrapping_add(8 + 8 * i).wrapping_add(4));
        if vt != 0 && word(arena, vt.wrapping_add(0x28)) != 0 {
            let e = rule_entry(arena, 0x548, 4);
            let v = word(arena, e);
            if !(v <= max) {
                max = v;
            }
        }
        i = (i + 1) & 0xff;
        if i >= 5 {
            break;
        }
    }
    let vt38 = word(arena, p.wrapping_add(56));
    if vt38 != 0 && word(arena, vt38.wrapping_add(0x28)) != 0 {
        let e = rule_entry(arena, 0x548, 4);
        let v = word(arena, e);
        if !(v <= max) {
            max = v;
        }
    }
    if arena[p.wrapping_add(49) as usize] != 0 && max > 4 {
        max = 4;
    }
    max
}

/// fn_198a80, EU fn_198b30, KR fn_198b6c. Returns the in-order successor
/// of `n` under the `a`/`b` links against `sentinel`, or 0 past the end.
pub fn tree_next(arena: &[u8], sentinel: u32, n: u32) -> u32 {
    let end = sentinel;
    if word(arena, n.wrapping_add(4)) != end {
        let mut prev = word(arena, n.wrapping_add(4));
        let mut nx = word(arena, prev);
        while nx != end {
            prev = nx;
            nx = word(arena, nx);
        }
        return prev;
    }
    let mut n = n;
    let mut cur = word(arena, n.wrapping_add(8));
    while cur != end && word(arena, cur.wrapping_add(4)) == n {
        n = cur;
        cur = word(arena, cur.wrapping_add(8));
    }
    if cur == end {
        0
    } else {
        cur
    }
}

/// fn_176fd4, EU fn_177084, KR fn_177030. Copies the 8-byte `D_8d7660`
/// global (all zeros on every region) to `out`. The global is mirrored
/// at arena `0x5c0`.
pub fn glob_pair(arena: &mut [u8], out: u32) {
    let (o, g) = (out as usize, 0x5c0);
    arena.copy_within(g..g + 8, o);
}

/// fn_240854, EU fn_2405b4, KR fn_240660. Scans at most `len` bytes for
/// the first UTF-8 error, returning its offset or -1; a negative `len`
/// scans to the NUL. Lead lengths come from the `D_9147f4` table,
/// mirrored at arena `0x5c8`.
pub fn utf8_scan(arena: &[u8], s: u32, len: u32) -> u32 {
    let s = s as usize;
    let mut len = len as i32;
    if len < 0 {
        let mut p = s;
        while arena[p] != 0 {
            p += 1;
        }
        len = (p - s) as i32;
    }
    let mut cur = s;
    while len > 0 {
        len -= 1;
        let b = arena[cur] as i32;
        if b >= 0x80 {
            if ((!b) & 0xc0) != 0 {
                return (cur - s) as u32;
            }
            let n = arena[0x5c8 + (b & 0x3f) as usize] as i32;
            if n > len {
                return (cur - s) as u32;
            }
            cur += 1;
            let b2 = arena[cur] as i32;
            len -= n;
            if (b2 & 0xc0) != 0x80 {
                return (cur - s) as u32;
            }
            match n {
                1 => {
                    if (b & 0x3e) == 0 {
                        return (cur - s) as u32;
                    }
                }
                2 => {
                    if b == 0xe0 && (b2 & 0x20) == 0 {
                        return (cur - s) as u32;
                    }
                }
                3 => {
                    if b == 0xf0 && (b2 & 0x30) == 0 {
                        return (cur - s) as u32;
                    }
                }
                4 => {
                    if b == 0xf8 && (b2 & 0x38) == 0 {
                        return (cur - s) as u32;
                    }
                }
                5 => {
                    if b == 0xfe || b == 0xff || (b == 0xfc && (b2 & 0x3c) == 0) {
                        return (cur - s) as u32;
                    }
                }
                _ => {}
            }
            for _ in 0..n - 1 {
                cur += 1;
                if ((arena[cur] as i32) & 0xc0) != 0x80 {
                    return (cur - s) as u32;
                }
            }
        }
        cur += 1;
    }
    (-1i32) as u32
}

/// Variable-length integer reader for [`charset_match`], following the
/// ROM's inlined `get6` (US `0x237dc4` and its two copies): the multibyte
/// path runs when `b >= 0xc0`, and the continuation decode always runs
/// there (the ARM gate is `cmp n, #0; ble`, which never fires because
/// the counts are 1..=5). Advances `*pp` past the consumed bytes.
///
/// NOTE (C defect, reported): `src/fn_237d48.c:18` gates the decode on
/// `if (v > 0)`, so the C takes the single path for the six
/// zero-top-field headers (`0xc0 0xe0 0xf0 0xf8 0xfc 0xfe`) where the
/// ROM decodes multibyte. (The old `b < 0x40` condition was fixed in
/// src commit `dd73888`.)
fn get6_237d48(arena: &[u8], pp: &mut usize) -> i32 {
    let mut p = *pp;
    let b = arena[p] as i32;
    p += 1;
    let mut v = b;
    if (b & 0xc0) == 0xc0 {
        let n = arena[0x5c8 + (b & 63) as usize] as i32;
        let sh = n * 6;
        let mask = word(arena, (0x608 + n * 4) as u32) as i32;
        v = ((b & mask) as u32).wrapping_shl(sh as u32) as i32;
        let mut q = p - 1;
        let mut sh = sh - 6;
        if (n & 1) != 0 {
            q += 1;
            v |= (((arena[q] & 63) as u32) << (sh as u32)) as i32;
            sh -= 6;
        }
        let mut k = n >> 1;
        while k != 0 {
            k -= 1;
            v |= ((((arena[q + 1]) & 63) as u32) << (sh as u32)) as i32;
            sh -= 6;
            v |= ((((arena[q + 2]) & 63) as u32) << (sh as u32)) as i32;
            sh -= 6;
            q += 2;
        }
        p += n as usize;
    }
    *pp = p;
    v
}

/// fn_237d48, EU fn_237f04, KR fn_237fb0. Tests `c` against the bitmap
/// and bytecode charset at `p`, honoring the negate flag. The get6
/// bytecode values decode per [`get6_237d48`] (see the C-defect note
/// there).
pub fn charset_match(arena: &[u8], c: u32, p: u32) -> u32 {
    let c = c as i32;
    let p = p as usize;
    let flags = arena[p];
    let neg = (flags & 1) as u32;
    if c < 256
        && (flags & 2) != 0
        && ((arena[p + 1 + (c / 8) as usize] as i32) & (1 << (c & 7))) != 0
    {
        return u32::from(neg == 0);
    }
    let mut q = p + 1;
    if (flags & 2) != 0 {
        q += 32;
    }
    loop {
        let b = arena[q];
        q += 1;
        if b == 0 {
            return neg;
        }
        if b == 1 {
            if get6_237d48(arena, &mut q) == c {
                return u32::from(neg == 0);
            }
        } else if b == 2 {
            let lo = get6_237d48(arena, &mut q);
            let hi = get6_237d48(arena, &mut q);
            if lo <= c && c <= hi {
                return u32::from(neg == 0);
            }
        }
    }
}

/// fn_1735e0, EU fn_173690, KR fn_17363c. Copies the `D_8d75bc` params
/// and calls `fn_17cbb4`; `0x80602007` when `p` or `q` is null. The
/// callee is the `rule_stA` script here, which ignores the copied
/// params, so only its status is observed.
pub fn params_call(arena: &mut [u8], p: u32, q: u32) -> u32 {
    if p == 0 || q == 0 {
        return 0x8060_2007;
    }
    let e = rule_entry(arena, 0x548, 4);
    word(arena, e)
}

/// fn_17c5bc, EU fn_17c66c, KR fn_17c618. Forwards `(s, end, base, 0)`
/// to `fn_17c5cc` and returns its result. The callee pops two
/// `rule_ptrA` script entries here: the first is returned, the second
/// is stored through `end` when non-null.
pub fn strtonum_wrap(arena: &mut [u8], s: u32, end: u32, base: u32) -> u32 {
    let _ = (s, base);
    let e1 = rule_entry(arena, 0x5b0, 4);
    let off1 = word(arena, e1);
    let e2 = rule_entry(arena, 0x5b0, 4);
    if end != 0 {
        put_word(arena, end, word(arena, e2));
    }
    off1
}

/// fn_176ac8, EU fn_176b78, KR fn_176b24. Formats `val` in `base` into
/// `buf` (sign only for base 10, digits above 9 as `W` + offset) and
/// returns `buf`. The `fn_30209c` divmod here is real: quotient feeds
/// the loop, remainder feeds the digit.
pub fn itoa(arena: &mut [u8], val: u32, buf: u32, base: u32) -> u32 {
    let base = base as i32;
    let mut v = val as i32;
    if v == 0 {
        arena[buf as usize] = b'0';
        arena[buf as usize + 1] = 0;
        return buf;
    }
    let mut p = buf;
    let mut s = buf;
    if v < 0 && base == 10 {
        arena[buf as usize] = b'-';
        s = buf.wrapping_add(1);
        p = p.wrapping_add(1);
        v = v.wrapping_neg();
    }
    while v != 0 {
        let d = v % base;
        v /= base;
        let ch = if d < 10 { d + 0x30 } else { d + 0x57 };
        arena[p as usize] = ch as u8;
        p = p.wrapping_add(1);
    }
    arena[p as usize] = 0;
    let mut e = p.wrapping_sub(1);
    while e > s {
        let u = arena[e as usize];
        let t = arena[s as usize];
        arena[e as usize] = t;
        arena[s as usize] = u;
        e = e.wrapping_sub(1);
        s = s.wrapping_add(1);
    }
    buf
}

/// fn_1b35a0, EU fn_1b35f4, KR fn_1b37f0. Allocates an entry through the
/// heap callback, fills it from the arguments (storing `value` through
/// `owner` when non-null, else into the link field), and appends it to
/// the vector; returns 1 when allocation fails, else the append status.
/// The opening search ignores its result and has no observable effect.
/// The heap callback is the `rule_ptrA` script here, and the vector
/// always has free capacity, so no growth call happens.
#[allow(clippy::too_many_arguments)]
pub fn vec_entry_add(
    arena: &mut [u8],
    p: u32,
    key: u32,
    value: u32,
    flags: u32,
    f: u32,
    owner: u32,
    tag: u32,
) -> u32 {
    let e = rule_entry(arena, 0x5b0, 4);
    let e = word(arena, e);
    if e == 0 {
        return 1;
    }
    put_word(arena, e, key);
    put_word(arena, e.wrapping_add(4), value);
    put_word(arena, e.wrapping_add(12), owner);
    if owner != 0 {
        put_word(arena, owner, value);
    } else {
        put_word(arena, e.wrapping_add(8), value);
    }
    put_word(arena, e.wrapping_add(16), flags);
    put_word(arena, e.wrapping_add(24), f);
    arena[e.wrapping_add(20) as usize] = tag as u8;
    arena[e.wrapping_add(21) as usize] = 0;
    let vec = word(arena, p.wrapping_add(8));
    let count = word(arena, vec.wrapping_add(8));
    let base = word(arena, vec.wrapping_add(4));
    put_word(arena, vec.wrapping_add(8), count.wrapping_add(1));
    put_word(arena, base.wrapping_add(count.wrapping_mul(4)), e);
    0
}

/// fn_1738e8, EU fn_173998, KR fn_173944. Fans a log record out to the
/// five targets and the extra target, returning the last error status
/// (`>= 0x80000000`) or 0; gated on level, the category, and the format.
/// Every live log method is the `rule_stA` script here.
pub fn log_fanout(
    arena: &mut [u8],
    p: u32,
    category: u32,
    level: u32,
    _arg: u32,
    fmt: u32,
) -> u32 {
    if p == 0 {
        return 0;
    }
    let top = arena[p.wrapping_add(0x30) as usize] as u32;
    let limited = arena[p.wrapping_add(0x31) as usize];
    if level > top || (limited != 0 && level > 4) || fmt == 0 {
        return 0;
    }
    if category == 0 {
        return 0x84b0_2006;
    }
    let mut result = 0;
    let mut i = 0u32;
    while i < 5 {
        // LogTarget targets[5] sits at +8 (after unused[2]); methods is
        // its second word: p + 8 + 8 * i + 4.
        let methods = word(arena, p.wrapping_add(8 + 8 * i).wrapping_add(4));
        if methods != 0 && word(arena, methods.wrapping_add(24)) != 0 && (i != 1 || level <= 1)
        {
            let e = rule_entry(arena, 0x548, 4);
            let status = word(arena, e);
            if status >= 0x8000_0000 {
                result = status;
            }
        }
        i += 1;
    }
    let methods = word(arena, p.wrapping_add(0x38));
    if methods != 0 && word(arena, methods.wrapping_add(24)) != 0 {
        let e = rule_entry(arena, 0x548, 4);
        let status = word(arena, e);
        if status >= 0x8000_0000 {
            result = status;
        }
    }
    result
}

/// Red-black rebalance for [`rb_insert`], mirroring the static helper.
fn rb_rebalance(arena: &mut [u8], t: u32, mut n: u32, mut p: u32) {
    while arena[p.wrapping_add(12) as usize] == 0 {
        let g = word(arena, p.wrapping_add(8));
        if p == word(arena, g) {
            let mut c = word(arena, g.wrapping_add(4));
            if arena[c.wrapping_add(12) as usize] == 0 {
                arena[p.wrapping_add(12) as usize] = 1;
                arena[c.wrapping_add(12) as usize] = 1;
                arena[g.wrapping_add(12) as usize] = 0;
                n = g;
                p = word(arena, g.wrapping_add(8));
            } else {
                if n == word(arena, p.wrapping_add(4)) {
                    c = word(arena, p.wrapping_add(4));
                    let y = word(arena, c);
                    put_word(arena, p.wrapping_add(4), y);
                    put_word(arena, y.wrapping_add(8), p);
                    let q = word(arena, p.wrapping_add(8));
                    put_word(arena, c.wrapping_add(8), q);
                    if word(arena, q) == p {
                        put_word(arena, q, c);
                    } else {
                        put_word(arena, q.wrapping_add(4), c);
                    }
                    put_word(arena, c, p);
                    put_word(arena, p.wrapping_add(8), c);
                    p = n;
                }
                arena[p.wrapping_add(12) as usize] = 1;
                arena[g.wrapping_add(12) as usize] = 0;
                c = word(arena, g);
                let y = word(arena, c.wrapping_add(4));
                put_word(arena, g, y);
                put_word(arena, y.wrapping_add(8), g);
                let q = word(arena, g.wrapping_add(8));
                put_word(arena, c.wrapping_add(8), q);
                if word(arena, q.wrapping_add(4)) == g {
                    put_word(arena, q.wrapping_add(4), c);
                } else {
                    put_word(arena, q, c);
                }
                put_word(arena, c.wrapping_add(4), g);
                put_word(arena, g.wrapping_add(8), c);
                break;
            }
        } else {
            let mut c = word(arena, g);
            if arena[c.wrapping_add(12) as usize] == 0 {
                arena[p.wrapping_add(12) as usize] = 1;
                arena[c.wrapping_add(12) as usize] = 1;
                arena[g.wrapping_add(12) as usize] = 0;
                n = g;
                p = word(arena, g.wrapping_add(8));
            } else {
                if n == word(arena, p) {
                    c = word(arena, p);
                    let y = word(arena, c.wrapping_add(4));
                    put_word(arena, p, y);
                    put_word(arena, y.wrapping_add(8), p);
                    let q = word(arena, p.wrapping_add(8));
                    put_word(arena, c.wrapping_add(8), q);
                    if word(arena, q.wrapping_add(4)) == p {
                        put_word(arena, q.wrapping_add(4), c);
                    } else {
                        put_word(arena, q, c);
                    }
                    put_word(arena, c.wrapping_add(4), p);
                    put_word(arena, p.wrapping_add(8), c);
                    p = n;
                }
                arena[p.wrapping_add(12) as usize] = 1;
                arena[g.wrapping_add(12) as usize] = 0;
                c = word(arena, g.wrapping_add(4));
                let y = word(arena, c);
                put_word(arena, g.wrapping_add(4), y);
                put_word(arena, y.wrapping_add(8), g);
                let q = word(arena, g.wrapping_add(8));
                put_word(arena, c.wrapping_add(8), q);
                if word(arena, q) == g {
                    put_word(arena, q, c);
                } else {
                    put_word(arena, q.wrapping_add(4), c);
                }
                put_word(arena, c, g);
                put_word(arena, g.wrapping_add(8), c);
                break;
            }
        }
    }
    let left = word(arena, t);
    arena[left.wrapping_add(12) as usize] = 1;
}

/// fn_198868, EU fn_198918, KR fn_198954. Inserts `n` under `key` with
/// red-black rebalancing; `0x89002002` on a duplicate in a non-unique
/// tree, else the init status. The comparator is the `rule_stA` script
/// here (read signed); init pops a `rule_stA` status and, on success, a
/// `rule_ptrA` entry for the node key.
pub fn rb_insert(arena: &mut [u8], t: u32, n: u32, key: u32) -> u32 {
    let mut p = t;
    let mut x = word(arena, t);
    let mut cmp = -1i32;
    while x != t {
        p = x;
        let e = rule_entry(arena, 0x548, 4);
        cmp = word(arena, e) as i32;
        if arena[t.wrapping_add(40) as usize] == 0 && cmp == 0 {
            return 0x8900_2002;
        }
        x = if cmp < 0 {
            word(arena, x)
        } else {
            word(arena, x.wrapping_add(4))
        };
    }
    let e = rule_entry(arena, 0x548, 4);
    let rc = word(arena, e);
    if rc >= 0x8000_0000 {
        return rc;
    }
    let f = rule_entry(arena, 0x5b0, 4);
    put_word(arena, n.wrapping_add(16), word(arena, f));
    let _ = key;
    if cmp < 0 {
        put_word(arena, p, n);
    } else {
        put_word(arena, p.wrapping_add(4), n);
    }
    put_word(arena, n, t);
    put_word(arena, n.wrapping_add(4), t);
    put_word(arena, n.wrapping_add(8), p);
    put_word(arena, t.wrapping_add(24), word(arena, t.wrapping_add(24)).wrapping_add(1));
    arena[n.wrapping_add(12) as usize] = 0;
    rb_rebalance(arena, t, n, p);
    rc
}

/// fn_1ba3c0, EU fn_1ba470, KR fn_1ba610. Stores the address of the
/// `D_917e80` global through `p`; always returns 0. The address is
/// represented by the `0x80000000` global token on every side.
pub fn glob_addr(arena: &mut [u8], p: u32) -> u32 {
    put_word(arena, p, 0x8000_0000);
    0
}

/// fn_243a94, EU fn_243c50, KR fn_243cfc. Forwards a resource open to
/// fn_242fd0 (the `r_init_stk` script here); passes through its status and,
/// on success, writes the resource pointer through `out`. The handle
/// couple (`obj`, `key`), `mode`, `path`, `type` and the packed version
/// word traverse untouched.
#[allow(clippy::too_many_arguments)]
pub fn handle_open(
    arena: &mut [u8],
    obj: u32,
    key: u32,
    mode: u32,
    path: u32,
    typ: u32,
    ver: u32,
    out: u32,
) -> u32 {
    let _ = (obj, key, mode, path, typ, ver);
    let e = rule_entry(arena, 0x548, 4);
    let rc = word(arena, e);
    if rc >= 0x8000_0000 {
        return rc;
    }
    let f = rule_entry(arena, 0x5b0, 4);
    if out != 0 {
        put_word(arena, out, word(arena, f));
    }
    rc
}

/// fn_17497c, EU fn_174a2c, KR fn_1749d8. Reads the flag byte at `p + 0x40`;
/// null yields 0.
pub fn flag_byte(arena: &[u8], p: u32) -> u32 {
    if p == 0 { 0 } else { byte(arena, p.wrapping_add(0x40)) as u32 }
}

/// fn_1757a8 (EU fn_175858, KR fn_175804) and fn_19861c (EU fn_1986cc,
/// KR fn_198708). Null guard: non-null yields 0, else the row's error code.
pub fn null_guard(p: u32, err: u32) -> u32 {
    if p != 0 { 0 } else { err }
}

/// fn_2028f0 (EU fn_202aac, KR fn_202b58) and fn_2028f8 (EU fn_202ab4,
/// KR fn_202b60). Nullary stubs; always return 0.
pub fn const_zero() -> u32 {
    0
}

/// fn_2fb820, EU fn_2fb9dc, KR fn_2fba88. Returns `*p + v + 4` (wrapping).
pub fn add4_deref(arena: &[u8], p: u32, v: u32) -> u32 {
    word(arena, p).wrapping_add(v).wrapping_add(4)
}

/// fn_2fb7b0, EU fn_2fb96c, KR fn_2fba18. Stores `v` through `p`; returns 0.
pub fn store_word(arena: &mut [u8], p: u32, v: u32) -> u32 {
    put_word(arena, p, v);
    0
}

/// fn_17eaf4, EU fn_17eba4, KR fn_17eb50. Writes `requested` through
/// `available` when non-null; returns 0.
pub fn avail_store(arena: &mut [u8], ctx: u32, feature: u32, requested: u32, avail: u32) -> u32 {
    let _ = (ctx, feature);
    if avail != 0 {
        put_word(arena, avail, requested);
    }
    0
}

/// fn_2ef004, EU fn_2ef1c0, KR fn_2ef26c. Clears the byte at `p + 0x4a`;
/// null yields `0x80002007`.
pub fn byte_clear_4a(arena: &mut [u8], p: u32) -> u32 {
    if p == 0 {
        return 0x8000_2007;
    }
    arena[p.wrapping_add(0x4a) as usize] = 0;
    0
}

/// fn_17bff4, EU fn_17c0a4, KR fn_17c050. UTF-8 length of a u16 string plus
/// one for the terminator.
pub fn utf8_len_u16(arena: &[u8], p: u32) -> u32 {
    let mut s = p;
    let mut n: u32 = 1;
    loop {
        let c = half(arena, s) as u32;
        s = s.wrapping_add(2);
        if c == 0 {
            break;
        }
        n += if c < 0x80 {
            1
        } else if c < 0x800 {
            2
        } else if c < 0x1_0000 {
            3
        } else if c < 0x11_0000 {
            4
        } else {
            5
        };
    }
    n
}

/// fn_1bab60, EU fn_1bac10, KR fn_1badb0. Zeroes the word at `p + 8`.
pub fn ctx_zero_08(arena: &mut [u8], p: u32) {
    put_word(arena, p.wrapping_add(8), 0);
}

/// fn_2c242c, EU fn_2c25e8, KR fn_2c2694. Zeroes three words at `o + 0x54`;
/// null yields `0x81102007`.
pub fn triple_zero(arena: &mut [u8], o: u32) -> u32 {
    if o == 0 {
        return 0x8110_2007;
    }
    put_word(arena, o.wrapping_add(0x54), 0);
    put_word(arena, o.wrapping_add(0x5c), 0);
    put_word(arena, o.wrapping_add(0x58), 0);
    0
}

/// fn_2c4768, EU fn_2c4924, KR fn_2c49d0. Copies the value at `p + 0x10`
/// through `out`; returns 0.
pub fn value_fetch(arena: &mut [u8], p: u32, out: u32) -> u32 {
    put_word(arena, out, word(arena, p.wrapping_add(0x10)));
    0
}

/// fn_1c0170, EU fn_1c0220, KR fn_1c03a4. Swaps `n` bytes between `a` and
/// `b`; aliased pointers are a no-op.
pub fn memswap(arena: &mut [u8], a: u32, b: u32, n: u32) {
    if a == b {
        return;
    }
    let mut x = a;
    let mut y = b;
    let mut left = n;
    while left != 0 {
        left -= 1;
        let t = arena[x as usize];
        arena[x as usize] = arena[y as usize];
        arena[y as usize] = t;
        x = x.wrapping_add(1);
        y = y.wrapping_add(1);
    }
}

/// fn_176f94, EU fn_177044, KR fn_176ff0. Returns 0 when `b == c` and
/// (`d == 0` or `a` is non-null), else `0x81706008`.
pub fn quad_check(a: u32, b: u32, c: u32, d: u32) -> u32 {
    if b == c && (d == 0 || a != 0) { 0 } else { 0x8170_6008 }
}

/// fn_176214, EU fn_1762c4, KR fn_176270. u16 string copy; returns `dst`.
pub fn u16_copy(arena: &mut [u8], dst: u32, src: u32) -> u32 {
    let mut d = dst;
    let mut s = src;
    loop {
        let v = half(arena, s);
        put_half(arena, d, v);
        if v == 0 {
            break;
        }
        d = d.wrapping_add(2);
        s = s.wrapping_add(2);
    }
    dst
}

/// fn_1763e4, EU fn_176494, KR fn_176440. Zero-extending u8 to u16 string
/// copy; returns `dst`.
pub fn u8_to_u16_copy(arena: &mut [u8], dst: u32, src: u32) -> u32 {
    let mut d = dst;
    let mut s = src;
    loop {
        let v = byte(arena, s) as u16;
        put_half(arena, d, v);
        if v == 0 {
            break;
        }
        d = d.wrapping_add(2);
        s = s.wrapping_add(1);
    }
    dst
}

/// fn_176514, EU fn_1765c4, KR fn_176570. u16 `strchr`; null when absent.
pub fn u16_chr(arena: &[u8], s: u32, c: u32) -> u32 {
    let mut p = s;
    while half(arena, p) as u32 != 0 && half(arena, p) as u32 != c {
        p = p.wrapping_add(2);
    }
    if half(arena, p) as u32 != c { 0 } else { p }
}

/// fn_176460, EU fn_176510, KR fn_1764bc. u16 to char copy, mapping units
/// above 0xff to `?`; returns `dst`.
pub fn u16_to_char_lossy(arena: &mut [u8], dst: u32, src: u32) -> u32 {
    let mut d = dst;
    let mut s = src;
    loop {
        let c = half(arena, s) as u32;
        if c == 0 {
            break;
        }
        arena[d as usize] = if c & 0xff00 != 0 { 0x3f } else { c as u8 };
        d = d.wrapping_add(1);
        s = s.wrapping_add(2);
    }
    arena[d as usize] = 0;
    dst
}

/// fn_19600c, EU fn_1960bc, KR fn_1960f8. u8 `strpbrk` over a 32-byte
/// bitmap; null when no byte matches.
pub fn u8_pbrk(arena: &[u8], s: u32, set: u32) -> u32 {
    let mut bits = [0u8; 32];
    let mut t = set;
    while byte(arena, t) != 0 {
        let v = byte(arena, t);
        bits[(v >> 3) as usize] |= 1 << (v & 7);
        t = t.wrapping_add(1);
    }
    let mut p = s;
    while byte(arena, p) != 0 {
        let v = byte(arena, p);
        if bits[(v >> 3) as usize] & (1 << (v & 7)) != 0 {
            return p;
        }
        p = p.wrapping_add(1);
    }
    0
}

/// fn_1765ac, EU fn_17665c, KR fn_176608. u16 `strcmp`.
pub fn u16_cmp(arena: &[u8], a: u32, b: u32) -> u32 {
    let mut x = a;
    let mut y = b;
    loop {
        let c = half(arena, x);
        if c == 0 {
            break;
        }
        if c != half(arena, y) {
            break;
        }
        x = x.wrapping_add(2);
        y = y.wrapping_add(2);
    }
    (half(arena, x) as i32 - half(arena, y) as i32) as u32
}

/// fn_1765dc, EU fn_17668c, KR fn_176638. u16 `strncmp` over `n` units.
pub fn u16_ncmp(arena: &[u8], a: u32, b: u32, n: u32) -> u32 {
    let mut x = a;
    let mut y = b;
    let mut left = n;
    while left != 0 {
        if half(arena, x) == 0 || half(arena, x) != half(arena, y) {
            return (half(arena, x) as i32 - half(arena, y) as i32) as u32;
        }
        left -= 1;
        x = x.wrapping_add(2);
        y = y.wrapping_add(2);
    }
    0
}

/// fn_17649c, EU fn_17654c, KR fn_1764f8. u16 to u8 copy capped at `n`
/// bytes, NUL-padded; returns the end pointer.
pub fn u16_to_u8_ncopy(arena: &mut [u8], dst: u32, src: u32, n: u32) -> u32 {
    let mut d = dst;
    let mut s = src;
    let mut left = n;
    loop {
        if left == 0 {
            return d;
        }
        let c = half(arena, s) as u32 & 255;
        s = s.wrapping_add(2);
        arena[d as usize] = c as u8;
        d = d.wrapping_add(1);
        if c == 0 {
            break;
        }
        left -= 1;
    }
    left -= 1;
    while left != 0 {
        arena[d as usize] = 0;
        d = d.wrapping_add(1);
        left -= 1;
    }
    d
}

/// fn_176644, EU fn_1766f4, KR fn_1766a0. u16 `strncpy` over `n` units;
/// returns `dst`.
pub fn u16_ncopy(arena: &mut [u8], dst: u32, src: u32, n: u32) -> u32 {
    let r = dst;
    let mut d = dst;
    let mut s = src;
    let mut left = n;
    loop {
        if left == 0 {
            return r;
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
    left -= 1;
    while left != 0 {
        put_half(arena, d, 0);
        d = d.wrapping_add(2);
        left -= 1;
    }
    r
}

/// fn_1b43ec, EU fn_1b4440, KR fn_1b463c. Takes one from the count at
/// `q + 0x10`; empty yields 3.
pub fn count_take(arena: &mut [u8], q: u32) -> u32 {
    let count = word(arena, q.wrapping_add(0x10));
    if count == 0 {
        return 3;
    }
    put_word(arena, q.wrapping_add(0x10), count - 1);
    0
}

/// fn_240b28, EU fn_240ce4, KR fn_240d90. CP437 lowercase over A-Z plus
/// accented Latin letters.
pub fn cp437_tolower(c: u32) -> u32 {
    if c.wrapping_sub(0x41) <= 0x19 {
        return (c.wrapping_add(0x20)) & 0xff;
    }
    match c {
        0x80 => 0x87,
        0x8e => 0x84,
        0x8f => 0x86,
        0x90 => 0x82,
        0x92 => 0x91,
        0x99 => 0x94,
        0x9a => 0x81,
        0xa5 => 0xa4,
        _ => c,
    }
}

/// fn_2e1ce8, EU fn_2e1ea4, KR fn_2e1f50. Tests whether the flag at
/// `p->f4->g8` is zero.
pub fn flag_is_zero(arena: &[u8], p: u32) -> u32 {
    u32::from(word(arena, word(arena, p.wrapping_add(4)).wrapping_add(8)) == 0)
}

/// fn_1baae0, EU fn_1bab90, KR fn_1bad30. Removes the first item equal to
/// `val`, shifting the tail down.
pub fn list_remove(arena: &mut [u8], p: u32, val: u32) {
    let items = word(arena, p.wrapping_add(4));
    let mut i: u32 = 0;
    while word(arena, p.wrapping_add(8)) > i {
        if word(arena, items.wrapping_add(i.wrapping_mul(4))) == val {
            let mut j = i;
            while word(arena, p.wrapping_add(8)).wrapping_sub(1) > j {
                let next = word(arena, items.wrapping_add(j.wrapping_add(1).wrapping_mul(4)));
                put_word(arena, items.wrapping_add(j.wrapping_mul(4)), next);
                j += 1;
            }
            put_word(arena, p.wrapping_add(8), word(arena, p.wrapping_add(8)) - 1);
            return;
        }
        i += 1;
    }
}

/// fn_17c800, EU fn_17c8b0, KR fn_17c85c. Space class through the
/// `D_8dc30c` table bit 3 (exactly tab/newline/vtab/formfeed/return/space,
/// verified from the ROM); returns 8 or 0.
pub fn space_flag(c: u32) -> u32 {
    if matches!(c as u8, 0x09..=0x0d | 0x20) { 8 } else { 0 }
}

/// fn_176624, EU fn_1766d4, KR fn_176680. u16 string length.
pub fn u16_len(arena: &[u8], s: u32) -> u32 {
    let mut p = s;
    while half(arena, p) != 0 {
        p = p.wrapping_add(2);
    }
    p.wrapping_sub(s) >> 1
}

/// fn_2832d8, EU fn_283494, KR fn_283540. Punctuation-class predicate over
/// ASCII, Latin-1/General-Punctuation and CJK/Compatibility ranges.
pub fn punct_class(c: u32) -> u32 {
    if c.wrapping_sub(0x21) <= 14 || c.wrapping_sub(0x3a) <= 6 {
        return 0;
    }
    if c.wrapping_sub(0x5b) <= 5 || c.wrapping_sub(0x7b) <= 0x44 {
        return 0;
    }
    if c.wrapping_sub(0x2000) <= 0x6f || c.wrapping_sub(0x3000) <= 0x3f {
        return 0;
    }
    if c.wrapping_sub(0xfe30) < 0x120 || c.wrapping_sub(0xfe50) <= 0x1f {
        return 0;
    }
    if c.wrapping_sub(0xff00) > 0xef { 1 } else { 0 }
}

/// fn_202900, EU fn_202abc, KR fn_202b68. Range accumulator selected by
/// `sel`; returns whether the total exceeds the limit. Fields are
/// struct-relative (f168/f172/f176/f180 at +0xa8/+0xac/+0xb0/+0xb4).
pub fn range_step(arena: &mut [u8], s: u32, a: u32, sel: u32) -> u32 {
    let lim = word(arena, s.wrapping_add(0xb0));
    match sel {
        0 => {
            put_word(arena, s.wrapping_add(0xb4), a);
            put_word(
                arena,
                s.wrapping_add(0xac),
                a.wrapping_add(word(arena, s.wrapping_add(0xa8))),
            );
        }
        1 => {
            put_word(
                arena,
                s.wrapping_add(0xac),
                word(arena, s.wrapping_add(0xac)).wrapping_add(a),
            );
            put_word(
                arena,
                s.wrapping_add(0xb4),
                a.wrapping_add(word(arena, s.wrapping_add(0xb4))),
            );
        }
        _ => {
            put_word(
                arena,
                s.wrapping_add(0xac),
                word(arena, s.wrapping_add(0xa8))
                    .wrapping_add(lim)
                    .wrapping_add(a),
            );
            put_word(arena, s.wrapping_add(0xb4), lim.wrapping_add(a));
        }
    }
    u32::from(word(arena, s.wrapping_add(0xb4)) > lim)
}

/// fn_176538, EU fn_1765e8, KR fn_176594. u16 `strstr`; null when absent.
pub fn u16_strstr(arena: &[u8], a: u32, b: u32) -> u32 {
    if half(arena, b) == 0 {
        return a;
    }
    if half(arena, a) == 0 {
        return 0;
    }
    let mut s = a;
    loop {
        let mut p = s;
        let mut q = b;
        loop {
            if half(arena, p) == 0 {
                break;
            }
            if half(arena, q) == 0 {
                return s;
            }
            if half(arena, p) != half(arena, q) {
                break;
            }
            p = p.wrapping_add(2);
            q = q.wrapping_add(2);
        }
        if half(arena, q) == 0 {
            return s;
        }
        s = s.wrapping_add(2);
        if half(arena, s) == 0 {
            return 0;
        }
    }
}

/// fn_283348, EU fn_283504, KR fn_2835b0. Unicode lowercase offset for
/// Latin, Latin-Extended, Greek and Cyrillic capitals.
pub fn tolower_offset(c: u32) -> u32 {
    let s = c as i32;
    let mut d: i32;
    if (0x41..=0x5a).contains(&s) || (0xc0..=0xd6).contains(&s) || (0xd8..=0xde).contains(&s) {
        d = 0x20;
    } else if ((0x100..=0x136).contains(&s) && c & 1 == 0)
        || ((0x139..=0x147).contains(&s) && c & 0x8000_0001 != 0)
        || ((0x14a..=0x178).contains(&s) && c & 1 == 0)
        || ((0x179..=0x17d).contains(&s) && c & 0x8000_0001 != 0)
    {
        if c == 0x130 {
            d = -199;
        } else if c == 0x178 {
            d = -121;
        } else {
            d = 1;
        }
    } else if c == 0x386
        || (0x388..=0x38a).contains(&c)
        || c == 0x38c
        || (0x38e..=0x38f).contains(&c)
        || (0x391..=0x3a1).contains(&c)
        || (0x3a3..=0x3ab).contains(&c)
    {
        d = match c {
            0x386 => 0x26,
            0x388 | 0x389 | 0x38a => 0x25,
            0x38c => 0x40,
            0x38e | 0x38f => 0x3f,
            _ => 0x20,
        };
    } else {
        let t = c.wrapping_sub(0x400);
        if t <= 0x2f
            || ((0x460..=0x480).contains(&c) && c & 1 == 0)
            || ((0x48c..=0x4be).contains(&c) && c & 1 == 0)
            || c == 0x4c1
            || c == 0x4c3
            || c == 0x4c7
            || c == 0x4cb
            || ((0x4d0..=0x4f4).contains(&c) && c & 1 == 0)
            || c == 0x4f8
        {
            if t <= 0xf {
                d = 0x50;
            } else if (0x410..=0x42f).contains(&c) {
                d = 0x20;
            } else {
                d = 1;
            }
        } else {
            d = 0;
        }
    }
    s.wrapping_add(d) as u16 as u32
}

/// fn_1734e8, EU fn_173598, KR fn_173544. Calls the doubly-indirect `m4c`
/// method (the `rule_stA` script here).
pub fn call_m4c(arena: &mut [u8], s: u32, arg: u32) -> u32 {
    if s == 0 || arg == 0 {
        return 0x8060_2007;
    }
    let e = rule_entry(arena, 0x548, 4);
    word(arena, e)
}

/// fn_196428, EU fn_1964d8, KR fn_196514. Releases `object` through the
/// allocator's `release` method (the `rule_stA` script here).
pub fn call_release(arena: &mut [u8], alloc: u32, obj: u32) {
    if alloc != 0 && obj != 0 {
        let e = rule_entry(arena, 0x548, 4);
        let _ = word(arena, e);
    }
}

/// fn_172fac, EU fn_17305c, KR fn_173008. Calls vtable slot 23 (the
/// `rule_stA` script here).
pub fn call_vt23(arena: &mut [u8], p: u32, x: u32, y: u32) -> u32 {
    if p == 0 || x == 0 || y == 0 {
        return 0x8060_2007;
    }
    let e = rule_entry(arena, 0x548, 4);
    word(arena, e)
}

/// fn_173180, EU fn_173230, KR fn_1731dc. Calls the `f3c` store method (the
/// `rule_stA` script here). The gate word at `0x50` proxies the patched
/// cell's nullness, which the replay image cannot show.
pub fn call_f3c(arena: &mut [u8], dst: u32, src: u32, path: u32, size: u32, flags: u32) -> u32 {
    let _ = (src, path, size, flags);
    if dst == 0 || src == 0 || path == 0 {
        return 0x8060_2007;
    }
    if word(arena, 0x50) == 0 {
        return 0x8060_2001;
    }
    let e = rule_entry(arena, 0x548, 4);
    word(arena, e)
}

/// fn_199134, EU fn_1991e4, KR fn_199220. Reads through the file `read`
/// method (the `rule_stA` script here). The gate word at `0x50` proxies
/// the patched cell's nullness.
pub fn file_read(arena: &mut [u8], dest: u32, size: u32, count: u32, file: u32) -> u32 {
    if file != 0 && dest != 0 && size != 0 && count != 0 {
        if word(arena, 0x50) == 0 {
            return 0x8060_2001;
        }
        let e = rule_entry(arena, 0x548, 4);
        return word(arena, e);
    }
    0x8060_2007
}

/// fn_1991d0, EU fn_199280, KR fn_1992bc. Invokes the triply-indirect
/// context method (the `rule_stA` script here). The gate word at `0x50`
/// proxies the patched cell's nullness.
pub fn ctx_invoke(arena: &mut [u8], p: u32) -> u32 {
    if p == 0 {
        return 0x8060_2000;
    }
    if word(arena, 0x50) == 0 {
        return 0x8060_2000;
    }
    let e = rule_entry(arena, 0x548, 4);
    word(arena, e)
}

/// fn_1748a0, EU fn_174950, KR fn_1748fc. Fans out over five entries plus a
/// tail call (the `rule_stA` script here), keeping the last failure. Gate
/// words at `0x90..0xa8` proxy the patched cells' nullness.
pub fn fanout_call(arena: &mut [u8], o: u32, a: u32, b: u32, c: u32, n: u32, f: u32) -> u32 {
    let _ = (a, b, c, f);
    let mut v = if o != 0 { byte(arena, o.wrapping_add(64)) as u32 } else { 0 };
    if o != 0 && v != 0 {
        if n == 0 {
            v = 0x84b0_2006;
        } else {
            let mut r = 0;
            let mut i: u32 = 0;
            loop {
                let vt = word(arena, o.wrapping_add(12).wrapping_add(8 * i));
                if vt != 0 && word(arena, 0x90 + 4 * i) != 0 {
                    let e = rule_entry(arena, 0x548, 4);
                    let t = word(arena, e);
                    if t >= 0x8000_0000 {
                        r = t;
                    }
                }
                i = i.wrapping_add(1) & 0xff;
                if i >= 5 {
                    break;
                }
            }
            if word(arena, o.wrapping_add(56)) != 0 && word(arena, 0xa4) != 0 {
                let e = rule_entry(arena, 0x548, 4);
                let t = word(arena, e);
                if t >= 0x8000_0000 {
                    r = t;
                }
            }
            v = r;
        }
    }
    v
}

/// fn_176fec, EU fn_17709c, KR fn_177048. Fetches `key` through fn_1966ac
/// (the `r_init` script here); maps lookup failure to `0x81506014`.
pub fn db_fetch(arena: &mut [u8], db: u32, key: u32, out: u32) -> u32 {
    let _ = (db, key);
    let e = rule_entry(arena, 0x548, 4);
    let rc = word(arena, e);
    if rc >= 0x8000_0000 {
        return 0x8150_6014;
    }
    let f = rule_entry(arena, 0x5b0, 4);
    let off = word(arena, f);
    if off != 0 {
        put_word(arena, out, word(arena, off));
    }
    0
}

/// fn_1964e0, EU fn_196590, KR fn_1965cc. Allocates a ticket through
/// fn_1963b8 (the `r_ptrA` script here); fn_198728/fn_198fa8 are void
/// stubs, so the `sub` block keeps its fill.
#[allow(clippy::too_many_arguments)]
pub fn ticket_new(
    arena: &mut [u8],
    ctx: u32,
    flag: u32,
    a: u32,
    b: u32,
    c: u32,
    d: u32,
    e: u32,
    out: u32,
) -> u32 {
    let _ = flag;
    if a == 0 || b == 0 || d == 0 || e == 0 || c == 0 || out == 0 {
        return 0x8900_2007;
    }
    let f = rule_entry(arena, 0x5b0, 4);
    let o = word(arena, f);
    put_word(arena, out, o);
    if o == 0 {
        return 0x8900_200a;
    }
    put_word(arena, o.wrapping_add(8), c);
    put_word(arena, o.wrapping_add(12), d);
    put_word(arena, o.wrapping_add(16), e);
    put_word(arena, o, a);
    put_word(arena, o.wrapping_add(4), b);
    put_word(arena, o.wrapping_add(20), ctx);
    0
}

/// fn_196a64, EU fn_196b14, KR fn_196b50. Allocates and fills a record
/// through fn_1963b8 (the `r_ptrA` script here).
pub fn obj_new(arena: &mut [u8], a: u32, b: u32, c: u32, d: u32, e: u32, out: u32) -> u32 {
    if c != 0 {
        if d == 0 {
            return 0;
        }
    } else if d != 0 {
        return 0;
    }
    if out == 0 {
        return 0;
    }
    let f = rule_entry(arena, 0x5b0, 4);
    let p = word(arena, f);
    put_word(arena, out, p);
    if p != 0 {
        put_word(arena, p, 0);
        put_word(arena, p.wrapping_add(16), 0);
        put_word(arena, p.wrapping_add(20), 0);
        put_word(arena, p.wrapping_add(4), b);
        put_word(arena, p.wrapping_add(8), c);
        put_word(arena, p.wrapping_add(12), d);
        put_half(arena, p.wrapping_add(24), e as u16);
        put_word(arena, p.wrapping_add(28), a);
        return 1;
    }
    0
}

/// fn_196794, EU fn_196844, KR fn_196880. Looks up through fn_198760 /
/// fn_1987fc / fn_198f74 (the `r_stA` script here), then allocates a
/// handle through fn_1963b8 (the `r_ptrA` script here).
pub fn handle_new(arena: &mut [u8], p: u32, a: u32, b: u32, out: u32) -> u32 {
    if p == 0 || out == 0 {
        return 0x8900_2007;
    }
    let mut rc = 0;
    let v;
    if a != 0 {
        let e = rule_entry(arena, 0x548, 4);
        v = word(arena, e);
        let _ = b;
        if v == 0 {
            rc = 0x8900_2014;
        }
    } else {
        let e = rule_entry(arena, 0x548, 4);
        v = word(arena, e);
    }
    if rc == 0 {
        let f = rule_entry(arena, 0x5b0, 4);
        let o = word(arena, f);
        put_word(arena, out, o);
        if o == 0 {
            return 0x8900_200a;
        }
        put_word(arena, o, p);
        put_word(arena, o.wrapping_add(4), 0);
        put_word(arena, o.wrapping_add(8), v);
    }
    rc
}

/// fn_196cfc, EU fn_196dac, KR fn_196de8. Vector push with growth through
/// the unsigned fn_1994d0 divmod and fn_1963e0/fn_196394 (the `r_ptrA`
/// script here). `grow` is constrained nonzero (divisor).
pub fn vec_grow(arena: &mut [u8], a: u32, out: u32) -> u32 {
    if a == 0 || out == 0 {
        return 0;
    }
    let cap = word(arena, a.wrapping_add(16));
    let count = word(arena, a.wrapping_add(20));
    if cap < count.wrapping_add(1) {
        let g = half(arena, a.wrapping_add(24)) as u32;
        let n = (count / g).wrapping_add(1).wrapping_mul(g);
        let data = word(arena, a);
        if data != 0 {
            let f = rule_entry(arena, 0x5b0, 4);
            let d = word(arena, f);
            if d != 0 {
                put_word(arena, a, d);
            } else {
                return 0;
            }
        } else {
            let f = rule_entry(arena, 0x5b0, 4);
            let d = word(arena, f);
            put_word(arena, a, d);
            if d == 0 {
                return 0;
            }
        }
        put_word(arena, a.wrapping_add(16), n);
    }
    let elem = half(arena, a.wrapping_add(4)) as u32;
    let data = word(arena, a);
    let count = word(arena, a.wrapping_add(20));
    put_word(arena, out, count.wrapping_mul(elem).wrapping_add(data));
    put_word(arena, a.wrapping_add(20), count.wrapping_add(1));
    1
}

/// fn_176c58, EU fn_176d08, KR fn_176cb4. Integer to string in `base`
/// through the unsigned fn_1994d0 divmod; `base` is constrained to 2..36.
pub fn udiv_itoa(arena: &mut [u8], value: u32, buf: u32, base: u32) -> u32 {
    if value == 0 {
        arena[buf as usize] = b'0';
        arena[buf.wrapping_add(1) as usize] = 0;
        return buf;
    }
    let mut v = value;
    let mut p = buf;
    while v != 0 {
        let d = v % base;
        v /= base;
        arena[p as usize] = if d < 10 { b'0' + d as u8 } else { b'W' + d as u8 };
        p = p.wrapping_add(1);
    }
    arena[p as usize] = 0;
    let mut q = buf;
    p = p.wrapping_sub(1);
    while p > q {
        let t = arena[q as usize];
        arena[q as usize] = arena[p as usize];
        arena[p as usize] = t;
        p = p.wrapping_sub(1);
        q = q.wrapping_add(1);
    }
    buf
}

/// fn_175944, EU fn_1759f4, KR fn_1759a0. Splits through fn_1756d0 (the
/// `r_1756d0` script here) and scans the headword with `atoi_scan` when
/// the split yields more than one piece.
pub fn pair_scan(arena: &mut [u8], a: u32, b: u32, out: u32) -> u32 {
    let _ = (a, b);
    let e1 = rule_entry(arena, 0x548, 4);
    let err = word(arena, e1);
    let f = rule_entry(arena, 0x5b0, 4);
    let off = word(arena, f);
    let e2 = rule_entry(arena, 0x548, 4);
    let v1 = word(arena, e2);
    if err < 0x8000_0000 && v1 > 1 {
        put_word(arena, out, atoi_scan(arena, off));
    } else {
        put_word(arena, out, 0);
    }
    err
}

/// fn_176104, EU fn_1761b4, KR fn_176160. Notifies two handler lists; each
/// entry call is the `rule_stA` script here. The fetch count is the word at
/// +20 (after base, size/pad, x, y, z), the same cell the length comes
/// from, so the fetch-error branch is unreachable here, matching `src`.
pub fn each_notify(arena: &mut [u8], p: u32, q: u32, o: u32, r: u32) -> u32 {
    let _ = (q, r);
    if p != 0 && q != 0 && o != 0 && r != 0 {
        for pass in 1..=2u32 {
            let l = if pass == 1 {
                word(arena, o.wrapping_add(8))
            } else {
                word(arena, p.wrapping_add(12))
            };
            if l != 0 {
                let n = idx5(arena, l);
                if n != 0 {
                    let mut i = 0;
                    while i < n {
                        if i >= word(arena, l.wrapping_add(20)) {
                            return 0x8b70_2100;
                        }
                        let size = half(arena, l.wrapping_add(4)) as u32;
                        let _e = word(arena, l).wrapping_add(i.wrapping_mul(size));
                        let e = rule_entry(arena, 0x548, 4);
                        let rc = word(arena, e);
                        if rc >= 0x8000_0000 {
                            return rc;
                        }
                        i += 1;
                    }
                }
            }
        }
    }
    0
}

/// Lead-byte masks from `D_8dc1ec` (verified from the ROM image).
const UTF8_LEAD: [u8; 8] = [0x00, 0x00, 0xc0, 0xe0, 0xf0, 0xf8, 0xfc, 0x00];

/// fn_17bdc4, EU fn_17be74, KR fn_17be20. Transcodes u16 to UTF-8 with an
/// optional source-position map; lengths come only in 1..3 from u16 input.
#[allow(clippy::too_many_arguments)]
pub fn transcode_u16(
    arena: &mut [u8],
    src: u32,
    srclen: u32,
    src_used: u32,
    dst: u32,
    dstcap: u32,
    dst_used: u32,
    mut posmap: u32,
) -> u32 {
    let mut result = 0;
    let mut si = 0;
    let mut di = 0;
    loop {
        if si >= srclen || di >= dstcap {
            break;
        }
        let c = half(arena, src.wrapping_add(2 * si)) as u32;
        let len = if c < 0x80 {
            1
        } else if c < 0x800 {
            2
        } else if c < 0x1_0000 {
            3
        } else if c < 0x11_0000 {
            4
        } else {
            5
        };
        if di.wrapping_add(len) > dstcap {
            result = 0x8bc0_2009;
            arena[dst.wrapping_add(di) as usize] = 0;
            break;
        }
        if posmap != 0 && len != 0 {
            for i in 0..len {
                put_word(arena, posmap.wrapping_add(4 * i), si << 1);
            }
            posmap = posmap.wrapping_add(len.wrapping_mul(4));
        }
        let mut v = c;
        si += 1;
        let mut k = len;
        while k > 1 {
            arena[dst.wrapping_add(di).wrapping_add(k).wrapping_sub(1) as usize] =
                ((v & 0xbf) | 0x80) as u8;
            v >>= 6;
            k -= 1;
        }
        arena[dst.wrapping_add(di) as usize] = v as u8 | UTF8_LEAD[len as usize];
        di += len;
    }
    if src_used != 0 {
        put_word(arena, src_used, si);
    }
    if dst_used != 0 {
        put_word(arena, dst_used, di);
    }
    if result < 0x8000_0000 && si != srclen {
        result = 0x8bc0_2009;
    }
    result
}

fn param_find(arena: &[u8], params: u32, key: u32) -> u32 {
    let entries = word(arena, params.wrapping_add(4));
    let count = word(arena, params.wrapping_add(8));
    let mut i = 0;
    while i < count {
        let e = word(arena, entries.wrapping_add(i.wrapping_mul(4)));
        if word(arena, e) == key {
            return e;
        }
        i += 1;
    }
    0
}

fn param_validate(arena: &mut [u8], entry: u32, value: u32) -> u32 {
    if entry == 0x400 {
        // Created entries carry the flags word here; the ROM calls through
        // to fn_1b3c00, which always accepts.
        return code_1(value);
    }
    let gate = word(
        arena,
        0x210 + (entry.wrapping_sub(0x140) / 0x20).wrapping_mul(4),
    );
    if gate == 0 {
        if byte(arena, entry.wrapping_add(21)) != 0
            && word(arena, entry.wrapping_add(4)) != value
        {
            return 0;
        }
        return 1;
    }
    let e = rule_entry(arena, 0x548, 4);
    word(arena, e)
}

/// fn_1b3c34, EU fn_1b3c88, KR fn_1b3e84. Loads ordinary and persistent
/// parameter pairs: found entries go through the `rule_stA` validate
/// script here (gated per entry), missing ones through `vec_entry_add`
/// when `add_missing` is set, and an empty persistent set re-syncs
/// defaults. New entries skip the region-specific flags word in the
/// observation.
pub fn param_db_load(arena: &mut [u8], settings: u32, data: u32, add_missing: u32) -> u32 {
    let params = word(arena, settings.wrapping_add(8));
    let count = half(arena, data) as u32;
    let persistent_count = half(arena, data.wrapping_add(2)) as u32;
    let ordinary = data.wrapping_add(4);
    let persistent = ordinary.wrapping_add(count.wrapping_mul(8));
    let mut status = 0;
    let mut pair = persistent;
    let mut left = persistent_count;
    while left != 0 {
        left -= 1;
        let key = word(arena, pair);
        let value = word(arena, pair.wrapping_add(4));
        pair = pair.wrapping_add(8);
        let entry = param_find(arena, params, key);
        if entry != 0 {
            if param_validate(arena, entry, value) == 0 {
                return code_4(0);
            }
            let output = word(arena, entry.wrapping_add(12));
            if output != 0 {
                put_word(arena, output, value);
            } else {
                put_word(arena, entry.wrapping_add(8), value);
            }
            let entry = param_find(arena, params, key);
            put_word(arena, entry.wrapping_add(4), value);
            arena[entry.wrapping_add(21) as usize] = 1;
        } else if add_missing != 0 {
            status = vec_entry_add(arena, settings, key, value, 0, 0, 0, 0);
            if status != 0 {
                return status;
            }
        }
    }
    let mut pair = ordinary;
    let mut left = count;
    while left != 0 {
        left -= 1;
        let key = word(arena, pair);
        let value = word(arena, pair.wrapping_add(4));
        pair = pair.wrapping_add(8);
        let entry = param_find(arena, params, key);
        if entry != 0 {
            if param_validate(arena, entry, value) == 0 {
                return code_4(0);
            }
            let output = word(arena, entry.wrapping_add(12));
            if output != 0 {
                put_word(arena, output, value);
            } else {
                put_word(arena, entry.wrapping_add(8), value);
            }
        } else if add_missing != 0 {
            status = vec_entry_add(arena, settings, key, value, 0, 0, 0, 0);
            if status != 0 {
                return status;
            }
        }
    }
    if persistent_count == 0 {
        let mut i = 0;
        while word(arena, params.wrapping_add(8)) > i {
            let entry = table_entry(arena, params, i);
            if byte(arena, entry.wrapping_add(20)) == 0 {
                put_word(arena, entry.wrapping_add(4), word(arena, entry.wrapping_add(8)));
            }
            i = i.wrapping_add(1);
        }
    }
    status
}

fn rb_rotate_left(arena: &mut [u8], node: u32) {
    let right = word(arena, node.wrapping_add(4));
    let left = word(arena, right);
    put_word(arena, node.wrapping_add(4), left);
    put_word(arena, left.wrapping_add(8), node);
    let parent = word(arena, node.wrapping_add(8));
    put_word(arena, right.wrapping_add(8), parent);
    if word(arena, parent) == node {
        put_word(arena, parent, right);
    } else {
        put_word(arena, parent.wrapping_add(4), right);
    }
    put_word(arena, right, node);
    put_word(arena, node.wrapping_add(8), right);
}

fn rb_rotate_right(arena: &mut [u8], node: u32) {
    let left = word(arena, node);
    let right = word(arena, left.wrapping_add(4));
    put_word(arena, node, right);
    put_word(arena, right.wrapping_add(8), node);
    let parent = word(arena, node.wrapping_add(8));
    put_word(arena, left.wrapping_add(8), parent);
    if word(arena, parent.wrapping_add(4)) == node {
        put_word(arena, parent.wrapping_add(4), left);
    } else {
        put_word(arena, parent, left);
    }
    put_word(arena, left.wrapping_add(4), node);
    put_word(arena, node.wrapping_add(8), left);
}

/// fn_198ae0, EU fn_198b90, KR fn_198bcc. Red-black tree delete with the
/// two-child successor splice and the full recolor fixup.
pub fn rb_erase(arena: &mut [u8], t: u32, z: u32) -> u32 {
    let mut y = z;
    let p = word(arena, z.wrapping_add(8));
    let x;
    if word(arena, z) != t && word(arena, z.wrapping_add(4)) != t {
        y = word(arena, z.wrapping_add(4));
        while word(arena, y) != t {
            y = word(arena, y);
        }
        let tmp = byte(arena, y.wrapping_add(12));
        x = word(arena, y.wrapping_add(4));
        let parent = word(arena, y.wrapping_add(8));
        put_word(arena, x.wrapping_add(8), parent);
        if word(arena, parent) == y {
            put_word(arena, parent, x);
        } else {
            put_word(arena, parent.wrapping_add(4), x);
        }
        put_word(arena, y.wrapping_add(8), p);
        let left = word(arena, z);
        put_word(arena, y, left);
        put_word(arena, y.wrapping_add(4), word(arena, z.wrapping_add(4)));
        put_word(arena, left.wrapping_add(8), y);
        let yr = word(arena, y.wrapping_add(4));
        put_word(arena, yr.wrapping_add(8), y);
        arena[y.wrapping_add(12) as usize] = byte(arena, z.wrapping_add(12));
        arena[z.wrapping_add(12) as usize] = tmp;
        if word(arena, p) == z {
            put_word(arena, p, y);
        } else {
            put_word(arena, p.wrapping_add(4), y);
        }
        y = z;
    } else {
        x = {
            let l = word(arena, z);
            if l == t { word(arena, z.wrapping_add(4)) } else { l }
        };
        put_word(arena, x.wrapping_add(8), p);
        if word(arena, p) == z {
            put_word(arena, p, x);
        } else {
            put_word(arena, p.wrapping_add(4), x);
        }
    }
    put_word(arena, z.wrapping_add(8), 0);
    put_word(arena, z.wrapping_add(4), 0);
    put_word(arena, z, 0);
    put_word(arena, t.wrapping_add(24), word(arena, t.wrapping_add(24)).wrapping_sub(1));
    if byte(arena, z.wrapping_add(12)) == 1 {
        let root = word(arena, t);
        arena[root.wrapping_add(12) as usize] = 0;
        let mut x = x;
        while byte(arena, x.wrapping_add(12)) == 1 {
            let p = word(arena, x.wrapping_add(8));
            if x == word(arena, p) {
                let mut w = word(arena, p.wrapping_add(4));
                if byte(arena, w.wrapping_add(12)) == 0 {
                    arena[w.wrapping_add(12) as usize] = 1;
                    arena[p.wrapping_add(12) as usize] = 0;
                    rb_rotate_left(arena, p);
                    w = word(arena, p.wrapping_add(4));
                }
                if byte(arena, word(arena, w).wrapping_add(12)) == 1 {
                    if byte(arena, word(arena, w.wrapping_add(4)).wrapping_add(12)) == 1 {
                        arena[w.wrapping_add(12) as usize] = 0;
                        x = p;
                        continue;
                    }
                } else if byte(arena, word(arena, w.wrapping_add(4)).wrapping_add(12)) == 1 {
                    let wl = word(arena, w);
                    arena[wl.wrapping_add(12) as usize] = 1;
                    arena[w.wrapping_add(12) as usize] = 0;
                    rb_rotate_right(arena, w);
                    w = word(arena, p.wrapping_add(4));
                }
                arena[w.wrapping_add(12) as usize] = byte(arena, p.wrapping_add(12));
                let wr = word(arena, w.wrapping_add(4));
                arena[wr.wrapping_add(12) as usize] = 1;
                arena[p.wrapping_add(12) as usize] = 1;
                rb_rotate_left(arena, p);
                break;
            } else {
                let mut w = word(arena, p);
                if byte(arena, w.wrapping_add(12)) == 0 {
                    arena[w.wrapping_add(12) as usize] = 1;
                    arena[p.wrapping_add(12) as usize] = 0;
                    rb_rotate_right(arena, p);
                    w = word(arena, p);
                }
                if byte(arena, word(arena, w.wrapping_add(4)).wrapping_add(12)) == 1 {
                    if byte(arena, word(arena, w).wrapping_add(12)) == 1 {
                        arena[w.wrapping_add(12) as usize] = 0;
                        x = p;
                        continue;
                    }
                } else if byte(arena, word(arena, w).wrapping_add(12)) == 1 {
                    let wr = word(arena, w.wrapping_add(4));
                    arena[wr.wrapping_add(12) as usize] = 1;
                    arena[w.wrapping_add(12) as usize] = 0;
                    rb_rotate_left(arena, w);
                    w = word(arena, p);
                }
                arena[w.wrapping_add(12) as usize] = byte(arena, p.wrapping_add(12));
                let wl = word(arena, w);
                arena[wl.wrapping_add(12) as usize] = 1;
                arena[p.wrapping_add(12) as usize] = 1;
                rb_rotate_right(arena, p);
                break;
            }
        }
        arena[x.wrapping_add(12) as usize] = 1;
        let root = word(arena, t);
        arena[root.wrapping_add(12) as usize] = 1;
    }
    y
}

/// fn_1b3ecc, EU fn_1b3f20, KR fn_1b411c. Applies the parameter database
/// with missing-key insertion disabled (forwards to [`param_db_load`]
/// with `add_missing` forced to 0).
pub fn param_db_load_no_add(arena: &mut [u8], obj: u32, data: u32) -> u32 {
    param_db_load(arena, obj, data, 0)
}

/// fn_19611c, EU fn_1961cc, KR fn_196208. Resolves `out` to the default
/// parameter address (the `0x80000000` global token here) unless the
/// scripted `fn_176fc0` check passes and `fn_176f94` succeeds, in which
/// case `out` takes the handle pointer (verified against `0x1e61`).
/// The `fn_176fd4` output is ignored, matching the stubbed row.
pub fn handle_word_or_default(arena: &mut [u8], ptr: u32, tag: u32, out: u32) -> u32 {
    let _ = tag;
    let e = rule_entry(arena, 0x548, 4);
    if word(arena, e) != 0 {
        put_word(arena, out, 0x8000_0000);
        return 0;
    }
    let e = rule_entry(arena, 0x548, 4);
    let r = word(arena, e);
    if r >= 0x8000_0000 {
        let r = if r & 0x1fff == 8 { 0x8160_2008 } else { r };
        put_word(arena, out, 0x8000_0000);
        return r;
    }
    put_word(arena, out, ptr);
    if ptr == 0 || half(arena, ptr) != 0x1e61 {
        put_word(arena, out, 0x8000_0000);
        return 0x8160_2008;
    }
    r
}

/// fn_196860, EU fn_196910, KR fn_19694c. Advances the iterator: on a
/// live `next` node copies `a`/`b` (at +16/+20) through `outa`/`outb`
/// and steps to the scripted `fn_198a80` successor (the `r_ptrA`
/// script here); otherwise zeroes the outs and reports exhaustion.
pub fn iter_next_pair(arena: &mut [u8], it: u32, outa: u32, outb: u32) -> u32 {
    if it == 0 || outa == 0 || outb == 0 {
        return 0x8900_2007;
    }
    let next = word(arena, it.wrapping_add(8));
    if next != 0 {
        put_word(arena, it.wrapping_add(4), next);
        put_word(arena, outa, word(arena, next.wrapping_add(16)));
        put_word(arena, outb, word(arena, next.wrapping_add(20)));
        let e = rule_entry(arena, 0x5b0, 4);
        put_word(arena, it.wrapping_add(8), word(arena, e));
        0
    } else {
        put_word(arena, outa, 0);
        put_word(arena, outb, 0);
        0x8900_2017
    }
}

/// fn_172f64, EU fn_173014, KR fn_172fc0. Calls vtable slot 0x58 (the
/// `rule_stA` script here); on success counts the `fn_196428` release
/// call in the arena word at 0x60 (the `r_inc` script here).
pub fn call_f58_release(arena: &mut [u8], h: u32) -> u32 {
    if h == 0 {
        return 0x8060_2007;
    }
    let e = rule_entry(arena, 0x548, 4);
    let rc = word(arena, e);
    if rc < 0x8000_0000 {
        put_word(arena, 0x60, word(arena, 0x60).wrapping_add(1));
    }
    rc
}

/// fn_173510, EU fn_1735c0, KR fn_17356c. Calls vtable slot 0x50 (the
/// `rule_stA` script here); on success counts the `fn_196428` release
/// call in the arena word at 0x60 (the `r_inc` script here).
pub fn call_f50_release(arena: &mut [u8], h: u32) -> u32 {
    if h == 0 {
        return 0x8060_2007;
    }
    let e = rule_entry(arena, 0x548, 4);
    let rc = word(arena, e);
    if rc < 0x8000_0000 {
        put_word(arena, 0x60, word(arena, 0x60).wrapping_add(1));
    }
    rc
}

/// fn_17c550, EU fn_17c600, KR fn_17c5ac. Parses an optional sign
/// followed by scripted digits (the `rule_stA` script here stands in
/// for `fn_17c7d0`; leading whitespace follows the ROM `D_8dc30c`
/// bit-8 set 9, 10, 11, 12, 13, 32 via the real `fn_17c800`).
/// At most one nonzero script pop is furnished, so the digit loop runs
/// at most once.
pub fn parse_int_scripted(arena: &mut [u8], s: u32) -> u32 {
    let mut p = s;
    while matches!(byte(arena, p), 9 | 10 | 11 | 12 | 13 | 32) {
        p = p.wrapping_add(1);
    }
    let mut n = 0u32;
    let mut c = byte(arena, p);
    p = p.wrapping_add(1);
    let sign = c;
    if c == b'-' || c == b'+' {
        c = byte(arena, p);
        p = p.wrapping_add(1);
    }
    loop {
        let e = rule_entry(arena, 0x548, 4);
        if word(arena, e) == 0 {
            break;
        }
        n = (c as u32)
            .wrapping_add(n.wrapping_mul(10))
            .wrapping_sub(b'0' as u32);
        c = byte(arena, p);
        p = p.wrapping_add(1);
    }
    if sign == b'-' {
        0u32.wrapping_sub(n)
    } else {
        n
    }
}

/// fn_17340c, EU fn_1734bc, KR fn_173468. Allocates a 12-byte block
/// (the `r_ptrA` script here), stores the object and allocator words,
/// and calls vtable slot 0x48 (the `rule_stA` script here, gated by
/// the word at 0x64). On a failing status the block is released
/// (counted at 0x60 via `r_inc`) and `out` cleared.
pub fn call_m72_alloc(
    arena: &mut [u8],
    o: u32,
    al: u32,
    a: u32,
    b: u32,
    c: u32,
    out: u32,
) -> u32 {
    let _ = b;
    alloc_call_free(arena, o, al, out, true, a, c, false, 0x8060_0000)
}

/// fn_1990ec, EU fn_19919c, KR fn_1991d8. Calls the inner `f4` method
/// (the `rule_stA` script here); on success counts the `fn_196428`
/// release call in the arena word at 0x60 (the `r_inc` script here).
pub fn call_f8_release(arena: &mut [u8], o: u32) -> u32 {
    if o == 0 {
        return 0x8060_2007;
    }
    let e = rule_entry(arena, 0x548, 4);
    let rc = word(arena, e);
    if rc < 0x8000_0000 {
        put_word(arena, 0x60, word(arena, 0x60).wrapping_add(1));
    }
    rc
}

/// fn_196c7c, EU fn_196d2c, KR fn_196d68. Destroys the array: runs the
/// optional element destructor over the live elements, logging each
/// `(element, context)` pair to the log at 0x180 (count at 0x280, the
/// `r_logA` script here, gated by the word at 0x64), then counts the
/// two `fn_196428` release calls at 0x60 (the `r_inc` script here).
pub fn release_array(arena: &mut [u8], a: u32) -> u32 {
    if a == 0 {
        return 0;
    }
    let data = word(arena, a);
    if data != 0 {
        if word(arena, 0x64) != 0 {
            let elem = half(arena, a.wrapping_add(4)) as u32;
            let count = word(arena, a.wrapping_add(20));
            let ctx = word(arena, a.wrapping_add(28));
            let mut i = 0u32;
            while i < count {
                let n = word(arena, 0x280);
                if n < 16 {
                    put_word(arena, 0x180 + n * 8, data.wrapping_add(i.wrapping_mul(elem)));
                    put_word(arena, 0x180 + n * 8 + 4, ctx);
                }
                put_word(arena, 0x280, n.wrapping_add(1));
                i = i.wrapping_add(1);
            }
        }
        put_word(arena, 0x60, word(arena, 0x60).wrapping_add(1));
    }
    put_word(arena, 0x60, word(arena, 0x60).wrapping_add(1));
    1
}

/// Post-order helper for [`release_tree`]: the row has no NULL check,
/// only the `n == end` stop (callers pass a NULL end for whole-tree
/// destroy), so the replay furnishes NULL-terminated trees.
fn release_tree_rec(arena: &mut [u8], heap: u32, n: u32, end: u32) {
    if n == end {
        return;
    }
    release_tree_rec(arena, heap, word(arena, n), end);
    release_tree_rec(arena, heap, word(arena, n.wrapping_add(4)), end);
    let a = word(arena, n.wrapping_add(16));
    if a != 0 {
        let m = word(arena, 0x280);
        if m < 16 {
            put_word(arena, 0x180 + m * 8, a);
            put_word(arena, 0x180 + m * 8 + 4, heap);
        }
        put_word(arena, 0x280, m.wrapping_add(1));
    }
    let b = word(arena, n.wrapping_add(20));
    if b != 0 {
        let m = word(arena, 0x284);
        if m < 16 {
            put_word(arena, 0x200 + m * 8, b);
            put_word(arena, 0x200 + m * 8 + 4, heap);
        }
        put_word(arena, 0x284, m.wrapping_add(1));
    }
    put_word(arena, 0x60, word(arena, 0x60).wrapping_add(1));
}

/// fn_198fb4, EU fn_199064, KR fn_1990a0. Destroys the tree in
/// post-order: nonzero `a`/`b` (at +16/+20) invoke the vtable
/// callbacks, logged to the pair-logs at 0x180/0x200 (counts at
/// 0x280/0x284, the `r_logA`/`r_logB` scripts here), and every
/// visited node is released (counted at 0x60 via `r_inc`).
pub fn release_tree(arena: &mut [u8], c: u32, n: u32, end: u32) {
    let heap = word(arena, c.wrapping_add(32));
    release_tree_rec(arena, heap, n, end);
}

/// fn_1b449c, EU fn_1b44f0, KR fn_1b46ec. Opens the stream state:
/// stores the `D_917e80` global through the stream (the `0x80000000`
/// token here, via [`glob_addr`]), clears the allocator word, then
/// stores the `D_917e8c` vtable (the `0x80000001` token) and `state`.
/// The linked `fn_1ba3c0` always succeeds, so the stores always run.
pub fn stream_open_state(arena: &mut [u8], stream: u32, state: u32) -> u32 {
    glob_addr(arena, stream);
    put_word(arena, stream.wrapping_add(8), 0);
    put_word(arena, stream, 0x8000_0001);
    put_word(arena, stream.wrapping_add(4), state);
    0
}

/// Alloc-call-free family shared by `call_m72_alloc`, `call_f54_alloc`,
/// `call_f0_alloc` and `call_create_alloc`: allocates a 12-byte block
/// (the `r_ptrA` script here), stores the two link words, and runs the
/// gated method (the `rule_stA` script here, gated by the word at
/// 0x64). On a failing status the block is released (counted at 0x60
/// via `r_inc`) and `out` cleared. `check_extra` selects the
/// `fn_17340c` variant, which also null-checks the two integer
/// arguments; `create_null_ok` selects the `fn_176e40` variant, where
/// a missing method skips the call and keeps the block.
fn alloc_call_free(
    arena: &mut [u8],
    o: u32,
    al: u32,
    out: u32,
    check_extra: bool,
    a: u32,
    c: u32,
    create_null_ok: bool,
    err_base: u32,
) -> u32 {
    if o == 0 || al == 0 || out == 0 || (check_extra && (a == 0 || c == 0)) {
        return err_base | 0x2007;
    }
    if word(arena, 0x64) == 0 {
        if create_null_ok {
            let e = rule_entry(arena, 0x5b0, 4);
            let blk = word(arena, e);
            put_word(arena, out, blk);
            if blk == 0 {
                return err_base | 0x200a;
            }
            put_word(arena, blk, o);
            put_word(arena, blk.wrapping_add(4), al);
            return 0;
        }
        return err_base | 0x2001;
    }
    let e = rule_entry(arena, 0x5b0, 4);
    let blk = word(arena, e);
    put_word(arena, out, blk);
    if blk == 0 {
        return err_base | 0x200a;
    }
    put_word(arena, blk, o);
    put_word(arena, blk.wrapping_add(4), al);
    let e = rule_entry(arena, 0x548, 4);
    let rc = word(arena, e);
    if rc >= 0x8000_0000 {
        put_word(arena, 0x60, word(arena, 0x60).wrapping_add(1));
        put_word(arena, out, 0);
    }
    rc
}

/// fn_172eac, EU fn_172f5c, KR fn_172f08. Allocates a 12-byte block
/// and calls vtable slot 0x54 (the `rule_stA` script here, gated by
/// the word at 0x64), freeing the block on failure. The integer
/// arguments are ignored by the stub.
pub fn call_f54_alloc(
    arena: &mut [u8],
    o: u32,
    al: u32,
    c: u32,
    d: u32,
    out: u32,
) -> u32 {
    let _ = (c, d);
    alloc_call_free(arena, o, al, out, false, 0, 0, false, 0x8060_0000)
}

/// fn_199034, EU fn_1990e4, KR fn_199120. Allocates a 12-byte block
/// and calls vtable slot 0 (the `rule_stA` script here, gated by the
/// word at 0x64), freeing the block on failure. The integer arguments
/// are ignored by the stub.
pub fn call_f0_alloc(
    arena: &mut [u8],
    o: u32,
    al: u32,
    c: u32,
    d: u32,
    out: u32,
) -> u32 {
    let _ = (c, d);
    alloc_call_free(arena, o, al, out, false, 0, 0, false, 0x8060_0000)
}

/// fn_176e40, EU fn_176ef0, KR fn_176e9c. Allocates a 12-byte block
/// and, when the create slot is present (gated by the word at 0x64),
/// calls it (the `rule_stA` script here), freeing the block on
/// failure. A missing method keeps the block and reports success.
pub fn call_create_alloc(arena: &mut [u8], o: u32, alloc: u32, out: u32) -> u32 {
    alloc_call_free(arena, o, alloc, out, false, 0, 0, true, 0x8140_0000)
}

/// fn_198f1c, EU fn_198fcc, KR fn_199008. Runs the notify pair: the
/// nonzero `f10`/`f14` argument words invoke the vtable callbacks,
/// logged to the pair-logs at 0x180/0x200 (counts at 0x280/0x284, the
/// `r_logA`/`r_logB` scripts here), then the owner object is released
/// (counted at 0x60 via `r_inc`). The leading `fn_198ae0` call is a
/// no-op stub.
pub fn call_f4f10_release(arena: &mut [u8], o: u32, a: u32) {
    let heap = word(arena, o.wrapping_add(32));
    let f10 = word(arena, a.wrapping_add(16));
    if f10 != 0 {
        let m = word(arena, 0x280);
        if m < 16 {
            put_word(arena, 0x180 + m * 8, f10);
            put_word(arena, 0x180 + m * 8 + 4, heap);
        }
        put_word(arena, 0x280, m.wrapping_add(1));
    }
    let f14 = word(arena, a.wrapping_add(20));
    if f14 != 0 {
        let m = word(arena, 0x284);
        if m < 16 {
            put_word(arena, 0x200 + m * 8, f14);
            put_word(arena, 0x200 + m * 8 + 4, heap);
        }
        put_word(arena, 0x284, m.wrapping_add(1));
    }
    put_word(arena, 0x60, word(arena, 0x60).wrapping_add(1));
}

/// fn_176edc, EU fn_176f8c, KR fn_176f38. Calls the doubly-indirect
/// method (the `rule_stA` script here, gated by the word at 0x64) and
/// releases the object on success (counted at 0x60 via `r_inc`).
pub fn call_ops_release(arena: &mut [u8], p: u32) -> u32 {
    if p == 0 {
        return 0x8140_2007;
    }
    let mut rc = 0u32;
    if word(arena, 0x64) != 0 {
        let e = rule_entry(arena, 0x548, 4);
        rc = word(arena, e);
    }
    if rc < 0x8000_0000 {
        put_word(arena, 0x60, word(arena, 0x60).wrapping_add(1));
    }
    rc
}

/// fn_198e34, EU fn_198ee4, KR fn_198f20. Allocates a node (the
/// `r_ptrA` script here); with a nonzero `b` runs the allocator
/// method (the `r_o32r2` script here, sharing the O32R3 cells) and
/// bails to the release path on failure. Otherwise zeroes the node,
/// stores the handle, and runs the stubbed `fn_198868` (the `r_stA`
/// script here). A failing insert logs the `free10` call (the `r_logA`
/// script here; `free4` is dead since `f10` is always 0 there) and
/// releases the node (counted at 0x60 via `r_inc`).
pub fn alloc_node_insert(arena: &mut [u8], s: u32, a: u32, b: u32, c: u32) -> u32 {
    let _ = (a, c);
    let heap = word(arena, s.wrapping_add(32));
    let e = rule_entry(arena, 0x5b0, 4);
    let n = word(arena, e);
    if n == 0 {
        return 0x8900_200a;
    }
    let mut tmp = 0u32;
    if b != 0 {
        let e = rule_entry(arena, 0x574, 8);
        let r = word(arena, e);
        if r >= 0x8000_0000 {
            put_word(arena, 0x60, word(arena, 0x60).wrapping_add(1));
            return r;
        }
        tmp = word(arena, e.wrapping_add(4));
    }
    put_word(arena, n, 0);
    put_word(arena, n.wrapping_add(4), 0);
    put_word(arena, n.wrapping_add(8), 0);
    arena[n.wrapping_add(12) as usize] = 0;
    put_word(arena, n.wrapping_add(16), 0);
    put_word(arena, n.wrapping_add(20), tmp);
    let e = rule_entry(arena, 0x548, 4);
    let r = word(arena, e);
    if r < 0x8000_0000 {
        return 0;
    }
    if tmp != 0 {
        let m = word(arena, 0x280);
        if m < 16 {
            put_word(arena, 0x180 + m * 8, tmp);
            put_word(arena, 0x180 + m * 8 + 4, heap);
        }
        put_word(arena, 0x280, m.wrapping_add(1));
    }
    put_word(arena, 0x60, word(arena, 0x60).wrapping_add(1));
    r
}

/// ROM `D_8dc30c` classifier bits verified from the US image:
/// bit 8 marks whitespace (9, 10, 11, 12, 13, 32), bit 4 marks
/// digits, bits 0-1 mark letters (A-Z, a-z), bit 2 marks lowercase.
fn ctype_8dc30c(c: u32) -> u16 {
    // Low nibble of the ROM `D_8dc30c` table (verified from the US image):
    // space = 8, digit = 4, upper = 1, lower = 2 (exclusive bits).
    let b = (c & 0xff) as u8;
    let mut t = 0u16;
    if matches!(b, 9 | 10 | 11 | 12 | 13 | 32) {
        t |= 8;
    }
    if b.is_ascii_digit() {
        t |= 4;
    }
    if b.is_ascii_uppercase() {
        t |= 1;
    }
    if b.is_ascii_lowercase() {
        t |= 2;
    }
    t
}

/// fn_17c5cc, EU fn_17c67c, KR fn_17c628. Parses an unsigned value in
/// `base` (with 0-prefix detection) through `end`, saturating on
/// overflow when `flags & 1` is clear. Classification and folding
/// mirror the real `fn_17c800`/`fn_17c7d0`/`fn_17c7b8`/`fn_17c8dc`
/// (`D_8dc30c` bits above); the quotient/remainder limit mirrors the
/// real `fn_1994d0` unsigned divmod.
pub fn strtoul_scan(arena: &mut [u8], text: u32, end: u32, base: u32, flags: u32) -> u32 {
    // `c` always holds the byte at `p` here (src keeps it one behind with
    // `p = text + 1` loads); `t` stays at the original text for the
    // no-conversion end pointer.
    let mut p = text;
    let mut c = byte(arena, p) as u32;
    while ctype_8dc30c(c) & 8 != 0 {
        p = p.wrapping_add(1);
        c = byte(arena, p) as u32;
    }
    let t = text;
    let mut f = flags;
    if c == b'-' as u32 {
        p = p.wrapping_add(1);
        c = byte(arena, p) as u32;
        f |= 2;
    } else if c == b'+' as u32 {
        p = p.wrapping_add(1);
        c = byte(arena, p) as u32;
    }
    let b = base as i32;
    if b < 0 || b == 1 || b > 36 {
        if end != 0 {
            put_word(arena, end, t);
        }
        return 0;
    }
    let mut b = b as u32;
    if b == 0 {
        if c != b'0' as u32 {
            b = 10;
        } else if byte(arena, p.wrapping_add(1)) != b'x'
            && byte(arena, p.wrapping_add(1)) != b'X'
        {
            b = 8;
        } else {
            b = 16;
        }
        if b == 16 {
            // Skip both prefix chars, then load the first digit char.
            p = p.wrapping_add(2);
            c = byte(arena, p) as u32;
        }
        return strtoul_digits(arena, t, end, p, c, b, f);
    }
    if b != 16 {
        return strtoul_digits(arena, t, end, p, c, b, f);
    }
    if c != b'0' as u32 {
        return strtoul_digits(arena, t, end, p, c, b, f);
    }
    if byte(arena, p.wrapping_add(1)) == b'x' {
        p = p.wrapping_add(2);
        c = byte(arena, p) as u32;
        return strtoul_digits(arena, t, end, p, c, b, f);
    }
    if byte(arena, p.wrapping_add(1)) != b'X' {
        return strtoul_digits(arena, t, end, p, c, b, f);
    }
    p = p.wrapping_add(2);
    c = byte(arena, p) as u32;
    strtoul_digits(arena, t, end, p, c, b, f)
}

/// Digit-loop core for [`strtoul_scan`]: `p`/`c` are the current
/// cursor and lookahead, `t` the token start for the no-digit reset.
fn strtoul_digits(
    arena: &mut [u8],
    t: u32,
    end: u32,
    mut p: u32,
    mut c: u32,
    b: u32,
    mut f: u32,
) -> u32 {
    let cutoff = 0xffff_ffffu32 / b;
    let cutlim = 0xffff_ffffu32 % b;
    let mut value = 0u32;
    loop {
        let digit = if ctype_8dc30c(c) & 4 != 0 {
            c.wrapping_sub(b'0' as u32)
        } else {
            if ctype_8dc30c(c) & 3 == 0 {
                break;
            }
            let mut up = c;
            if ctype_8dc30c(c) & 2 != 0 {
                up = up.wrapping_sub(32);
            }
            up.wrapping_sub(b'A' as u32).wrapping_add(10)
        };
        if digit >= b {
            break;
        }
        f |= 8;
        if value < cutoff || (value == cutoff && cutlim >= digit) {
            value = value.wrapping_mul(b).wrapping_add(digit);
        } else {
            f |= 4;
        }
        p = p.wrapping_add(1);
        c = byte(arena, p) as u32;
    }
    // No `--p` here: `p` already points at the first non-digit (`c` is
    // the byte at `p`, unlike src where it lags one behind).
    if f & 8 != 0 {
        if f & 4 != 0 {
            // Overflow with digits: `flags & 1` keeps 0xffffffff, else the
            // saturate blocks below overwrite with the signed limit.
            if f & 1 != 0 {
                value = 0xffff_ffff;
            }
        }
    } else {
        value = 0;
        if end == 0 {
            if f & 2 != 0 {
                value = value.wrapping_neg();
            }
            return value;
        }
        p = t;
    }
    if f & 1 == 0 && f & 4 == 0 {
        if f & 2 != 0 {
            if value > 0x8000_0000 {
                value = 0x8000_0000;
            }
        } else if value > 0x7fff_ffff {
            value = 0x7fff_ffff;
        }
    }
    if f & 4 != 0 && f & 1 == 0 {
        if f & 2 != 0 {
            value = 0x8000_0000;
        } else {
            value = 0x7fff_ffff;
        }
    }
    if end != 0 {
        put_word(arena, end, p);
    }
    if f & 2 != 0 {
        value = value.wrapping_neg();
    }
    value
}

/// fn_1babcc, EU fn_1bac7c, KR fn_1bae1c. Builds the object: the
/// scripted `fn_1b2140` (the `r_2140` script here) yields the object
/// or a nonzero error byte; the real `fn_1ba3c0` then stamps the
/// `D_917e80` global (the `0x80000000` token here). With `count >= 1`
/// the real `fn_1b218c` runs (its method is the `rule_stA` script
/// here; the flag-0 weak call is an empty stub) and a nonzero vote
/// stores the result. On error the `fn_1b2128` release call is counted
/// at 0x60 and `out` cleared; the `D_9c8954` vtable is the
/// `0x80000001` token here.
pub fn obj_build_or_free(
    arena: &mut [u8],
    heap: u32,
    x: u32,
    count: u32,
    y: u32,
    out: u32,
) -> u32 {
    let _ = heap;
    let e = rule_entry(arena, 0x5b0, 4);
    let o = word(arena, e);
    if o == 0 {
        return 1;
    }
    glob_addr(arena, o);
    put_word(arena, o.wrapping_add(4), 0);
    put_word(arena, o.wrapping_add(8), 0);
    put_word(arena, o, 0x8000_0001);
    put_word(arena, o.wrapping_add(12), 0);
    put_word(arena, o.wrapping_add(16), y);
    put_word(arena, o.wrapping_add(20), x);
    let mut e1 = 0u32;
    if count >= 1 {
        let yy = if y < count { count } else { y };
        let flag = yy.wrapping_mul(4);
        if flag == 0 {
            e1 = 9;
        } else {
            let e = rule_entry(arena, 0x548, 4);
            let r = word(arena, e);
            if r == 0 {
                e1 = 1;
            } else {
                put_word(arena, o.wrapping_add(4), r);
                put_word(arena, o.wrapping_add(12), yy);
            }
        }
    }
    if e1 == 0 {
        put_word(arena, out, o);
        0
    } else {
        put_word(arena, 0x60, word(arena, 0x60).wrapping_add(1));
        put_word(arena, out, 0);
        e1
    }
}

/// fn_17ad10, EU fn_17adc0, KR fn_17ad6c. Releases the optional `f8`,
/// each of the `h12` array elements plus the array itself, the
/// optional `f14`, and the object; every `fn_196428` call is counted
/// in the arena word at 0x60 (the `r_inc` script here).
pub fn release_parts(arena: &mut [u8], o: u32, p: u32) {
    let _ = o;
    if p == 0 {
        return;
    }
    let mut count = 0u32;
    if word(arena, p.wrapping_add(8)) != 0 {
        count = count.wrapping_add(1);
    }
    if word(arena, p.wrapping_add(16)) != 0 {
        let mut i = 0u32;
        let h12 = half(arena, p.wrapping_add(12)) as u32;
        while i < h12 {
            count = count.wrapping_add(1);
            i = i.wrapping_add(1);
        }
        count = count.wrapping_add(1);
    }
    if word(arena, p.wrapping_add(20)) != 0 {
        count = count.wrapping_add(1);
    }
    count = count.wrapping_add(1);
    put_word(arena, 0x60, word(arena, 0x60).wrapping_add(count));
}
