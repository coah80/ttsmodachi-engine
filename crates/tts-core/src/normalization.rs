

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

pub fn clear_markers(object: &mut [u8]) {
    put_half(object, 0xbc, 0);
    put_half(object, 0xc4, 0);
}

pub fn set_item(items: &mut [i32], index: u32, value: i32) -> i32 {
    if let Some(slot) = items.get_mut(index as usize) {
        *slot = value;
    }
    0
}

pub fn byte_compare_at(bytes: &[u8], a: u32, b: u32) -> u32 {
    compare_byte(bytes[a as usize], bytes[b as usize]) as u32
}

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

pub fn field_value(object: Option<&[u8]>, out: Option<&mut u32>) -> u32 {
    match (object, out) {
        (Some(object), Some(out)) => {
            *out = word(object, 0xc);
            0
        }
        _ => 0x81102007,
    }
}

pub fn inner_field(inner: Option<&[u8]>, out: Option<&mut u16>) -> u32 {
    match (inner, out) {
        (Some(inner), Some(out)) => {
            *out = half(inner, 8);
            0
        }
        _ => 0x81102007,
    }
}

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

pub fn key_le(a_hi: u32, a_lo: u32, b_hi: u32, b_lo: u32) -> i32 {
    if a_hi == b_hi {
        (a_lo <= b_lo) as i32
    } else {
        (a_hi <= b_hi) as i32
    }
}

pub fn id_in_list(list: &[u16], id: u16) -> i32 {
    let mut index = 0;
    while list.get(index).copied().unwrap_or(0) != 0 && id != list[index] {
        index += 1;
    }
    (id == list.get(index).copied().unwrap_or(0)) as i32
}

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

pub fn token_position_order(a: &[u8], b: &[u8]) -> i32 {
    let (a_token, b_token) = (word(a, 12) as i32, word(b, 12) as i32);
    if a_token == b_token {
        (word(a, 4) as i32).wrapping_sub(word(b, 4) as i32)
    } else {
        a_token.wrapping_sub(b_token)
    }
}

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

pub fn byte_less(a: u8, b: u8) -> i32 {
    (a < b) as i32
}

pub fn check_object(handle: u32, host: &mut impl HandleValidator) -> u32 {
    let status = host.validate(handle, 0xf383, 56);
    if status >= 0x8000_0000 {
        0x89c02008
    } else {
        status
    }
}

pub fn check_normalizer(handle: u32, host: &mut impl HandleValidator) -> u32 {
    let status = host.validate(handle, 0xf3b3, 0x3c);
    if status >= 0x8000_0000 {
        0x8b802008
    } else {
        status
    }
}

pub trait NormalizationHost {

    fn calloc(&mut self, arena: &mut [u8], heap: u32, count: u32, size: u32) -> Option<u32>;

    fn alloc(&mut self, arena: &mut [u8], heap: u32, size: u32) -> Option<u32>;

    fn realloc(&mut self, arena: &mut [u8], heap: u32, ptr: u32, size: u32) -> Option<u32>;

    fn free(&mut self, arena: &mut [u8], heap: u32, ptr: u32);

    fn resolve(&mut self, arena: &mut [u8], h0: u32, h1: u32) -> (u32, u32);
}

pub fn error_code(code: u32) -> u32 {
    code | 0x8960_2000
}

pub fn forward_error(code: u32) -> u32 {
    error_code(code)
}

pub(crate) fn udiv(value: u32, divisor: u32) -> u32 {
    value / divisor
}

pub(crate) fn strlen8(data: &[u8], base: u32) -> u32 {
    let mut length = 0u32;
    while byte(data, base.wrapping_add(length)) != 0 {
        length = length.wrapping_add(1);
    }
    length
}

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

pub(crate) fn memset_at(arena: &mut [u8], dst: u32, value: u8, n: u32) {
    let mut index = 0u32;
    while index != n {
        if let Some(slot) = arena.get_mut(dst.wrapping_add(index) as usize) {
            *slot = value;
        }
        index = index.wrapping_add(1);
    }
}

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

pub(crate) fn u16len(data: &[u8], base: u32) -> u32 {
    let mut length = 0u32;
    while half(data, base.wrapping_add(length * 2)) != 0 {
        length = length.wrapping_add(1);
    }
    length
}

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

pub(crate) fn table_row(data: &[u8], table: u32, idx: u32) -> Option<u32> {
    if table == 0 || idx >= word(data, table.wrapping_add(20)) {
        return None;
    }
    Some(word(data, table).wrapping_add(idx.wrapping_mul(half(data, table.wrapping_add(4)) as u32)))
}

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

pub fn release_owned(arena: &mut [u8], obj: u32, host: &mut impl NormalizationHost) {
    release_owned_at(arena, obj, 0x78, host);
}

pub fn release_owned_second(arena: &mut [u8], obj: u32, host: &mut impl NormalizationHost) {
    release_owned_at(arena, obj, 0x80, host);
}

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

pub fn item_value(arena: &[u8], obj: u32) -> u16 {
    let item = word(arena, obj.wrapping_add(0x44));
    if item == 0 {
        return 0;
    }
    half(arena, item.wrapping_add(8))
}

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

pub fn alloc_record(arena: &mut [u8], obj: u32, host: &mut impl NormalizationHost) -> u32 {
    calloc_field(arena, obj, 0x78, 0x7c, 1, 16, host)
}

pub fn alloc_record_second(arena: &mut [u8], obj: u32, host: &mut impl NormalizationHost) -> u32 {
    calloc_field(arena, obj, 0x80, 0x84, 1, 16, host)
}

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

pub fn trim_pattern(arena: &mut [u8], pattern: u32, string: u32, len: u32, separators: &[u8]) {
    if string == 0 || len == 0 {
        return;
    }
    let at = |arena: &[u8]| half(arena, string.wrapping_add(half(arena, len) as u32 * 2));

    loop {
        if half(arena, len) == 0 {
            break;
        }
        if u16chr(separators, 0, at(arena)).is_none() {

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

pub struct DecoderTables<'a> {
    pub extra: &'a [u8],
    pub mask: &'a [u8],
    pub mask_base: u32,
    pub base: &'a [u8],
    pub base_base: u32,
}

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

pub fn check_punctuation(handle: u32, host: &mut impl HandleValidator) -> u32 {
    check_tagged(handle, 0xf383, 56, 0x89c0_2008, host)
}

pub fn check_dctlkp(handle: u32, host: &mut impl HandleValidator) -> u32 {
    check_tagged(handle, 0xf3b3, 60, 0x8b80_2008, host)
}

pub fn check_packaging(handle: u32, host: &mut impl HandleValidator) -> u32 {
    check_tagged(handle, 0xf382, 44, 0x89b0_2008, host)
}

pub fn check_packaging_second(handle: u32, host: &mut impl HandleValidator) -> u32 {
    check_tagged(handle, 0xf382, 44, 0x89b0_2008, host)
}

pub fn publish_object(arena: &mut [u8], object: u32, key: u32, out: u32) -> u32 {
    if object == 0 || key == 0 || out == 0 {
        error_code(7)
    } else {
        put_word(arena, out, object);
        0
    }
}

pub fn publish_object_second(arena: &mut [u8], object: u32, key: u32, out: u32) -> u32 {
    if object == 0 || key == 0 || out == 0 {
        error_code(7)
    } else {
        put_word(arena, out, object);
        0
    }
}

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

pub fn grow_slots_first(
    arena: &mut [u8],
    obj: u32,
    n: u32,
    host: &mut impl NormalizationHost,
) -> u32 {
    grow_slots(arena, obj, n, 0x78, 0x7c, 0x7e, host)
}

pub fn grow_slots_second(
    arena: &mut [u8],
    obj: u32,
    n: u32,
    host: &mut impl NormalizationHost,
) -> u32 {
    grow_slots(arena, obj, n, 0x80, 0x84, 0x86, host)
}

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

pub trait LookupHost {

    fn lookup(&mut self, obj: u32, arg: u32) -> u32;
}

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

pub trait PcreDeinitHost {

    fn deinit(&mut self, a: u32, b: u32) -> u32;
}

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

pub trait NormOutHost: NormalizationHost {

    fn acquire(&mut self, arena: &mut [u8], resources: u32, name: u32) -> (u32, u32);

    fn release_named(&mut self, resources: u32, name: u32);

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

pub const NAME_LINGDB: u32 = 0;

pub const NAME_OUT_FE_DEPES: u32 = 7;

pub const NAME_OUT_FE_DCTLKP: u32 = 16;

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

pub const WORD_RECORD_LEN: u32 = 12;

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

pub fn compare_packed(a: u32, b: u32) -> u32 {
    let high_a = a >> 16;
    let high_b = b >> 16;
    let low_a = (a << 16) >> 24;
    let low_b = (b << 16) >> 24;
    u32::from(high_a == high_b && low_a <= low_b)
}

pub fn table_entry(arena: &[u8], tbl: u32, index: u32) -> u32 {
    let base = word(arena, tbl.wrapping_add(4));
    if base == 0 {
        return 0;
    }
    word(arena, base.wrapping_add(index.wrapping_mul(4)))
}

pub fn obj_close(_arena: &[u8], _obj: u32) -> u32 {
    0
}

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

pub(crate) fn rule_entry(arena: &mut [u8], script: u32, size: u32) -> u32 {
    let n = word(arena, script);
    let idx = if n < 2 { n } else { 1 };
    put_word(arena, script, n.wrapping_add(1));
    script.wrapping_add(4).wrapping_add(idx.wrapping_mul(size))
}

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

pub fn run_callback(_arena: &[u8], _obj: u32, _arg: u32) {}

pub fn run_dtors(arena: &mut [u8], obj: u32) -> u32 {
    if obj == 0 {
        return 0;
    }
    put_word(arena, obj.wrapping_add(20), 0);
    1
}

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

pub fn code_1(_value: u32) -> u32 {
    1
}

pub fn code_4(_msg: u32) -> u32 {
    4
}

pub fn code_6(_msg: u32) -> u32 {
    6
}

pub fn code_7(_msg: u32) -> u32 {
    7
}

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

pub fn dbl_deref_ne(arena: &[u8], a: u32, b: u32) -> u32 {
    let x = word(arena, word(arena, a));
    let y = word(arena, word(arena, b));
    u32::from(x != y)
}

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

pub fn pair_eq(al: u32, ah: u32, bl: u32, bh: u32) -> u32 {
    u32::from(ah == bh && al == bl)
}

pub fn idx5(arena: &[u8], p: u32) -> u32 {
    if p == 0 {
        return 0;
    }
    word(arena, p.wrapping_add(20))
}

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

pub fn flag_set(arena: &mut [u8], p: u32) {
    arena[p.wrapping_add(0x28) as usize] = 1;
}

pub fn vt_call0(arena: &mut [u8], s: u32, x: u32) -> u32 {
    if s == 0 || x == 0 {
        return 0;
    }
    let e = rule_entry(arena, 0x5b0, 4);
    word(arena, e)
}

pub fn vt_call1(arena: &mut [u8], p: u32, a: u32, b: u32) -> u32 {
    if p == 0 || a == 0 || b == 0 {
        return 0;
    }
    let e = rule_entry(arena, 0x548, 4);
    word(arena, e)
}

pub fn quad_copy(arena: &mut [u8], dst: u32, src: u32, e: u32) -> u32 {
    let (s, d) = (src as usize, dst as usize);
    arena.copy_within(s..s + 16, d);
    put_word(arena, dst.wrapping_add(16), e);
    0
}

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

pub fn glob_pair(arena: &mut [u8], out: u32) {
    let (o, g) = (out as usize, 0x5c0);
    arena.copy_within(g..g + 8, o);
}

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

pub fn params_call(arena: &mut [u8], p: u32, q: u32) -> u32 {
    if p == 0 || q == 0 {
        return 0x8060_2007;
    }
    let e = rule_entry(arena, 0x548, 4);
    word(arena, e)
}

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

pub fn glob_addr(arena: &mut [u8], p: u32) -> u32 {
    put_word(arena, p, 0x8000_0000);
    0
}

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

pub fn flag_byte(arena: &[u8], p: u32) -> u32 {
    if p == 0 { 0 } else { byte(arena, p.wrapping_add(0x40)) as u32 }
}

pub fn null_guard(p: u32, err: u32) -> u32 {
    if p != 0 { 0 } else { err }
}

pub fn const_zero() -> u32 {
    0
}

pub fn add4_deref(arena: &[u8], p: u32, v: u32) -> u32 {
    word(arena, p).wrapping_add(v).wrapping_add(4)
}

pub fn store_word(arena: &mut [u8], p: u32, v: u32) -> u32 {
    put_word(arena, p, v);
    0
}

pub fn avail_store(arena: &mut [u8], ctx: u32, feature: u32, requested: u32, avail: u32) -> u32 {
    let _ = (ctx, feature);
    if avail != 0 {
        put_word(arena, avail, requested);
    }
    0
}

pub fn byte_clear_4a(arena: &mut [u8], p: u32) -> u32 {
    if p == 0 {
        return 0x8000_2007;
    }
    arena[p.wrapping_add(0x4a) as usize] = 0;
    0
}

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

pub fn ctx_zero_08(arena: &mut [u8], p: u32) {
    put_word(arena, p.wrapping_add(8), 0);
}

pub fn triple_zero(arena: &mut [u8], o: u32) -> u32 {
    if o == 0 {
        return 0x8110_2007;
    }
    put_word(arena, o.wrapping_add(0x54), 0);
    put_word(arena, o.wrapping_add(0x5c), 0);
    put_word(arena, o.wrapping_add(0x58), 0);
    0
}

pub fn value_fetch(arena: &mut [u8], p: u32, out: u32) -> u32 {
    put_word(arena, out, word(arena, p.wrapping_add(0x10)));
    0
}

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

pub fn quad_check(a: u32, b: u32, c: u32, d: u32) -> u32 {
    if b == c && (d == 0 || a != 0) { 0 } else { 0x8170_6008 }
}

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

pub fn u16_chr(arena: &[u8], s: u32, c: u32) -> u32 {
    let mut p = s;
    while half(arena, p) as u32 != 0 && half(arena, p) as u32 != c {
        p = p.wrapping_add(2);
    }
    if half(arena, p) as u32 != c { 0 } else { p }
}

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

pub fn count_take(arena: &mut [u8], q: u32) -> u32 {
    let count = word(arena, q.wrapping_add(0x10));
    if count == 0 {
        return 3;
    }
    put_word(arena, q.wrapping_add(0x10), count - 1);
    0
}

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

pub fn flag_is_zero(arena: &[u8], p: u32) -> u32 {
    u32::from(word(arena, word(arena, p.wrapping_add(4)).wrapping_add(8)) == 0)
}

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

pub fn space_flag(c: u32) -> u32 {
    if matches!(c as u8, 0x09..=0x0d | 0x20) { 8 } else { 0 }
}

pub fn u16_len(arena: &[u8], s: u32) -> u32 {
    let mut p = s;
    while half(arena, p) != 0 {
        p = p.wrapping_add(2);
    }
    p.wrapping_sub(s) >> 1
}

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

pub fn call_m4c(arena: &mut [u8], s: u32, arg: u32) -> u32 {
    if s == 0 || arg == 0 {
        return 0x8060_2007;
    }
    let e = rule_entry(arena, 0x548, 4);
    word(arena, e)
}

pub fn call_release(arena: &mut [u8], alloc: u32, obj: u32) {
    if alloc != 0 && obj != 0 {
        let e = rule_entry(arena, 0x548, 4);
        let _ = word(arena, e);
    }
}

pub fn call_vt23(arena: &mut [u8], p: u32, x: u32, y: u32) -> u32 {
    if p == 0 || x == 0 || y == 0 {
        return 0x8060_2007;
    }
    let e = rule_entry(arena, 0x548, 4);
    word(arena, e)
}

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



#[allow(clippy::too_many_arguments)]
pub fn transcode_u16(
    arena: &mut [u8],
    src: u32,
    srclen: u32,
    src_used: u32,
    dst: u32,
    dstcap: u32,
    dst_used: u32,
    posmap: u32,
) -> u32 {
    transcode_u16_with_lead(arena, src, srclen, src_used, dst, dstcap, dst_used, posmap, 0x8dc1ec)
}

#[allow(clippy::too_many_arguments)]
pub fn transcode_u16_with_lead(
    arena: &mut [u8],
    src: u32,
    srclen: u32,
    src_used: u32,
    dst: u32,
    dstcap: u32,
    dst_used: u32,
    mut posmap: u32,
    lead: u32,
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
        arena[dst.wrapping_add(di) as usize] = v as u8 | arena[lead as usize + len as usize];
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

pub fn param_db_load_no_add(arena: &mut [u8], obj: u32, data: u32) -> u32 {
    param_db_load(arena, obj, data, 0)
}

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

pub fn release_tree(arena: &mut [u8], c: u32, n: u32, end: u32) {
    let heap = word(arena, c.wrapping_add(32));
    release_tree_rec(arena, heap, n, end);
}

pub fn stream_open_state(arena: &mut [u8], stream: u32, state: u32) -> u32 {
    glob_addr(arena, stream);
    put_word(arena, stream.wrapping_add(8), 0);
    put_word(arena, stream, 0x8000_0001);
    put_word(arena, stream.wrapping_add(4), state);
    0
}

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

pub fn call_create_alloc(arena: &mut [u8], o: u32, alloc: u32, out: u32) -> u32 {
    alloc_call_free(arena, o, alloc, out, false, 0, 0, true, 0x8140_0000)
}

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

fn ctype_8dc30c(c: u32) -> u16 {

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

pub fn strtoul_scan(arena: &mut [u8], text: u32, end: u32, base: u32, flags: u32) -> u32 {

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

    if f & 8 != 0 {
        if f & 4 != 0 {

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
