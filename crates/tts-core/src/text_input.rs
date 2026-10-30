//! Text- and marker-parsing leaves reached by the enu corpus.
//!
//! These are the small members of slice 2's text/marker parsing section.
//! Pointer fields are byte offsets into the caller's borrowed arena, with 0
//! meaning null.

use crate::depes::HandleValidator;
use crate::normalization::{
    entry_offset, error_code, fetch_entry, field_value, inner_field, insert_elements,
    insert_ordered, memcmp_at, memcpy_at, memset_at, narrow_at, open_iterator, release_iterator,
    set_index, strcat_at, strlen8, strnappend, strstr_at, u16chr, u16cmp, u16len, udiv,
    NormalizationHost,
};

/// Host operations for dictionary chunk attachment (`fn_243ad4`,
/// `fn_243d50`), owned by another slice. Arena offsets with 0 meaning null.
pub trait ChunkHost {
    /// `fn_243ad4`: detach a chunk record for `reader`.
    fn detach(
        &mut self,
        arena: &mut [u8],
        reader: u32,
        out_record: &mut u32,
        out_data: &mut u32,
    ) -> u32;
    /// `fn_243d50`: release a chunk record.
    fn release(&mut self, record: u32) -> u32;
}

fn byte(data: &[u8], offset: u32) -> u8 {
    data.get(offset as usize).copied().unwrap_or(0)
}

fn half(data: &[u8], offset: u32) -> u16 {
    let low = data.get(offset as usize).copied().unwrap_or(0) as u16;
    let high = data
        .get(offset.wrapping_add(1) as usize)
        .copied()
        .unwrap_or(0) as u16;
    low | high << 8
}

fn word(data: &[u8], offset: u32) -> u32 {
    half(data, offset) as u32 | (half(data, offset.wrapping_add(2)) as u32) << 16
}

fn put_byte(data: &mut [u8], offset: u32, value: u8) {
    if let Some(slot) = data.get_mut(offset as usize) {
        *slot = value;
    }
}

fn put_half(data: &mut [u8], offset: u32, value: u16) {
    put_byte(data, offset, value as u8);
    put_byte(data, offset.wrapping_add(1), (value >> 8) as u8);
}

fn put_word(data: &mut [u8], offset: u32, value: u32) {
    for (index, byte) in value.to_le_bytes().iter().enumerate() {
        if let Some(slot) = data.get_mut(offset as usize + index) {
            *slot = *byte;
        }
    }
}

/// fn_2c4750, EU fn_2c490c, KR fn_2c49b8. Sets the +0x15 marker byte.
pub fn mark_flag(object: Option<&mut [u8]>) -> u32 {
    match object {
        Some(object) => {
            if let Some(slot) = object.get_mut(21) {
                *slot = 1;
            }
            0
        }
        None => 0x80002007,
    }
}

/// fn_2c2408, EU fn_2c25c4, KR fn_2c2670. Collapses the three window words when
/// the middle one is the smallest.
pub fn normalize_window(object: &mut [u8]) -> u32 {
    let first = word(object, 0x54);
    let middle = word(object, 0x58);
    let last = word(object, 0x5c);
    if middle < first && last < middle {
        put_word(object, 0x54, middle);
        put_word(object, 0x5c, middle);
    }
    0
}

/// fn_2bd7d8, EU fn_2bd994, KR fn_2bda40. Equality of the +0x0a keys.
pub fn keys_equal(a: Option<&[u8]>, b: Option<&[u8]>) -> u32 {
    match (a, b) {
        (Some(a), Some(b)) if half(a, 10) == half(b, 10) => 1,
        _ => 0,
    }
}

/// fn_2bd800, EU fn_2bd9bc, KR fn_2bda68. Less-than of the +0x14 keys.
pub fn key_less(a: Option<&[u8]>, b: Option<&[u8]>) -> u32 {
    match (a, b) {
        (Some(a), Some(b)) if word(a, 20) < word(b, 20) => 1,
        _ => 0,
    }
}

/// fn_2bd828, EU fn_2bd9e4, KR fn_2bda90. Greater-than of the +0x14 keys.
pub fn key_greater(a: Option<&[u8]>, b: Option<&[u8]>) -> u32 {
    match (a, b) {
        (Some(a), Some(b)) if word(a, 20) > word(b, 20) => 1,
        _ => 0,
    }
}

/// Table cursor shared by the segment and marker iterators: the table header
/// at `+0x0c`, the element size at header `+0x02`, the base at header `+0x04`,
/// the count at header `+0x08` and the cursor at object `+0x10`.
fn table_params(arena: &[u8], obj: u32) -> (u16, i32, u16, u16) {
    let tab = word(arena, obj.wrapping_add(12));
    (
        half(arena, tab.wrapping_add(2)),
        word(arena, tab.wrapping_add(4)) as i32,
        half(arena, tab.wrapping_add(8)),
        half(arena, obj.wrapping_add(16)),
    )
}

/// fn_2c2450, EU fn_2c260c, KR fn_2c26b8. Maps an input position through the
/// segment table.
pub fn map_position(arena: &[u8], owner: u32, position: u32, out: &mut u32) -> u32 {
    if owner == 0 {
        return 0x8110_2007;
    }
    *out = position;
    let (size, base, count, index) = table_params(arena, owner);
    let mut segments = 0i32;
    let status = entry_offset(size, base, count, index, 0, &mut segments);
    if status >= 0x8000_0000 {
        if status & 0x1fff != 15 {
            return status;
        }
    } else {
        let inner = word(arena, owner.wrapping_add(12));
        let inner_slice = &arena[inner as usize..];
        let mut total = 0u16;
        let status = if inner == 0 {
            inner_field(None, Some(&mut total))
        } else {
            inner_field(Some(inner_slice), Some(&mut total))
        };
        if status >= 0x8000_0000 {
            return status;
        }
        let mut best = 0u32;
        let mut i = 0u16;
        while i < total
            && position
                >= word(
                    arena,
                    (segments as u32)
                        .wrapping_add(i as u32 * 40)
                        .wrapping_add(20),
                )
        {
            let segment = (segments as u32).wrapping_add(i as u32 * 40);
            let end = word(arena, segment.wrapping_add(20))
                .wrapping_add(word(arena, segment.wrapping_add(24)));
            if byte(arena, segment) == 0
                && end <= position
                && (best == 0
                    || word(arena, best.wrapping_add(20))
                        .wrapping_add(word(arena, best.wrapping_add(24)))
                        <= end)
            {
                best = segment;
            }
            i = i.wrapping_add(1);
        }
        if best != 0 {
            let offset = word(arena, best.wrapping_add(12))
                .wrapping_add(word(arena, best.wrapping_add(16)))
                .wrapping_sub(
                    word(arena, best.wrapping_add(20))
                        .wrapping_add(word(arena, best.wrapping_add(24))),
                );
            *out = out.wrapping_add(offset);
        }
    }
    0
}

/// fn_2bdca4, EU fn_2bde60, KR fn_2bdf0c. Normalizes a marker span and inserts
/// it unless an identical span is already stored.
pub fn insert_span(
    arena: &mut [u8],
    obj: u32,
    span: u32,
    flag: u32,
    host: &mut impl NormalizationHost,
) -> u32 {
    if obj == 0 || span == 0 {
        return 0x8110_2007;
    }
    let off = word(arena, span.wrapping_add(0x14));
    let len = word(arena, span.wrapping_add(0x18));
    let mut start = word(arena, span.wrapping_add(0x0c));
    let mut status = map_position(arena, obj, off, &mut start);
    if status >= 0x8000_0000 {
        return status;
    }
    put_word(arena, span.wrapping_add(0x0c), start);
    let mut end = word(arena, span.wrapping_add(0x10));
    status = map_position(arena, obj, off.wrapping_add(len), &mut end);
    if status >= 0x8000_0000 {
        return status;
    }
    put_word(arena, span.wrapping_add(0x10), end);
    let start = word(arena, span.wrapping_add(0x0c));
    let end = word(arena, span.wrapping_add(0x10));
    if end >= start {
        put_word(arena, span.wrapping_add(0x10), end.wrapping_sub(start));
    }
    if flag == 1 {
        // Skip the ordered scan.
    } else {
        let (size, base, count, index) = table_params(arena, obj);
        let base_addr = base as u32 as usize;
        let span_slice = &arena[span as usize..];
        let mut index = index;
        let mut accept = |element: &[u8]| key_less(Some(span_slice), Some(element)) as i32;
        status = crate::normalization::scan_forward(
            &arena[base_addr..],
            size,
            count,
            &mut index,
            &mut accept,
        );
        put_half(arena, obj.wrapping_add(16), index);
        if status >= 0x8000_0000 && status & 0x1fff != 0x14 {
            return status;
        }
    }
    let inner = word(arena, obj.wrapping_add(12));
    let mut total = 0u16;
    status = if inner == 0 {
        inner_field(None, Some(&mut total))
    } else {
        inner_field(Some(&arena[inner as usize..]), Some(&mut total))
    };
    if status >= 0x8000_0000 {
        return status;
    }
    let (size, base, count, index) = table_params(arena, obj);
    let mut tab_addr = 0i32;
    status = entry_offset(size, base, count, index, 0, &mut tab_addr);
    if status >= 0x8000_0000 {
        return status;
    }
    let mut i = 0u16;
    while i < total {
        let entry = (tab_addr as u32).wrapping_add(i as u32 * 0x28);
        if word(arena, span.wrapping_add(0x14)) < word(arena, entry.wrapping_add(0x14)) {
            break;
        }
        if memcmp_at(arena, entry, span, 0x28) == 0 {
            return 0;
        }
        i = i.wrapping_add(1);
    }
    status = insert_elements(arena, obj, span, 1, host);
    if status < 0x8000_0000 {
        0
    } else {
        status
    }
}

fn is_boundary(arena: &[u8], event: u32) -> bool {
    if event == 0 {
        return false;
    }
    let kind = half(arena, event.wrapping_add(10));
    let value = word(arena, event.wrapping_add(28)) as i32;
    ((kind == 2 || kind == 3) && value == 1) || kind == 12 || kind == 20
}

/// fn_2bdecc, EU fn_2be088, KR fn_2be134. Checks whether a boundary decision
/// changes at `position`. `scratch` is 1026 caller-owned bytes: a 1024-byte
/// narrow buffer plus two halfword slots the stop scan uses.
pub fn check_boundary(
    arena: &mut [u8],
    timeline: u32,
    list: u32,
    strings: u32,
    position: u32,
    name: u32,
    out: &mut u8,
    scratch: u32,
    separators: &[u8],
) -> u32 {
    let mut different = 0;
    if byte(arena, timeline.wrapping_add(12)) != 0 {
        *out = if (half(arena, timeline.wrapping_add(8)) as u32) > position {
            1
        } else {
            0
        };
        return 0;
    }
    let events = word(arena, list.wrapping_add(4));
    let mut i = 0u16;
    if position != 0 {
        let scale = half(arena, timeline.wrapping_add(2)) as u32;
        while i < half(arena, list.wrapping_add(8))
            && (word(arena, events.wrapping_add(i as u32 * 0x28).wrapping_add(20)) as u32)
                <= position
                    .wrapping_mul(scale)
                    .wrapping_add(word(arena, events.wrapping_add(20)))
        {
            let event = events.wrapping_add(i as u32 * 0x28);
            if half(arena, event.wrapping_add(10)) == 21 && name != 0 {
                let text = word(arena, strings.wrapping_add(4));
                let value = word(arena, event.wrapping_add(28));
                narrow_at(arena, scratch, text.wrapping_add(value.wrapping_mul(2)));
                if strstr_at(arena, name, scratch).is_some() {
                    different = 1;
                } else {
                    different = 0;
                }
            }
            i = i.wrapping_add(1);
        }
    }
    // The stop scan borrows its two halfword slots from the scratch tail.
    let scan_base = scratch.wrapping_add(1024);
    if i < half(arena, list.wrapping_add(8)) {
        loop {
            let event = events.wrapping_add(i as u32 * 0x28);
            if word(arena, event.wrapping_add(20)) == word(arena, events.wrapping_add(20))
                || !is_boundary(arena, event)
            {
                if half(arena, event.wrapping_add(10)) == 21 && name != 0 {
                    let text = word(arena, strings.wrapping_add(4));
                    let value = word(arena, event.wrapping_add(28));
                    narrow_at(arena, scratch, text.wrapping_add(value.wrapping_mul(2)));
                    different = if strstr_at(arena, name, scratch).is_some() {
                        1
                    } else {
                        0
                    };
                }
                i = i.wrapping_add(1);
            } else {
                if different != 1 || half(arena, event.wrapping_add(10)) == 3 {
                    let middle = udiv(
                        word(arena, event.wrapping_add(20))
                            .wrapping_sub(word(arena, events.wrapping_add(20))),
                        2,
                    ) as u16;
                    let stop =
                        contains_stop_scan(arena, list, timeline, middle, separators, scan_base);
                    if !stop {
                        break;
                    }
                }
                put_word(arena, event.wrapping_add(28), 0);
                i = i.wrapping_add(1);
            }
            if !(i < half(arena, list.wrapping_add(8))) {
                break;
            }
        }
    }
    *out = if i < half(arena, list.wrapping_add(8)) {
        1
    } else {
        0
    };
    0
}

fn event_has_text(kind: u16) -> bool {
    matches!(
        kind,
        6 | 12 | 21 | 50 | 36 | 39 | 37 | 34 | 22 | 40 | 26 | 29
    )
}

/// fn_2c4328, EU fn_2c44e4, KR fn_2c4590. Appends an event, interning its
/// text when the type carries one.
pub fn append_event(
    arena: &mut [u8],
    obj: u32,
    event: u32,
    host: &mut impl NormalizationHost,
) -> u32 {
    if obj == 0 || event == 0 {
        return 0x8000_2007;
    }
    let mut array = 0u32;
    let mut status = fetch_entry(arena, obj, 0x3f4, 1, Some(&mut array), host);
    if status >= 0x8000_0000 {
        return status;
    }
    let mut staged = [0u8; 0x28];
    for (index, slot) in staged.iter_mut().enumerate() {
        *slot = byte(arena, event.wrapping_add(index as u32));
    }
    status = fetch_entry(arena, obj, 0x3f3, 1, Some(&mut array), host);
    if status >= 0x8000_0000 {
        return status;
    }
    let used = half(arena, array.wrapping_add(8));
    let next = used.wrapping_add(1);
    if half(arena, array.wrapping_add(10)) < next {
        status = fetch_entry(arena, obj, 0x3f3, next as u32, Some(&mut array), host);
        if status >= 0x8000_0000 {
            return status;
        }
    }
    let mut texts = 0u32;
    status = fetch_entry(arena, obj, 0x3f4, 1, Some(&mut texts), host);
    if status >= 0x8000_0000 {
        return status;
    }
    let staged_kind = u16::from_le_bytes([staged[10], staged[11]]);
    let staged_text = u32::from_le_bytes([staged[28], staged[29], staged[30], staged[31]]);
    if event_has_text(staged_kind) && staged_text != 0 {
        let mut index = 0u32;
        let mut cursor = word(arena, texts.wrapping_add(4));
        let texts_used = half(arena, texts.wrapping_add(8)) as u32;
        while cursor != 0 && index < texts_used {
            let len = u16len(arena, cursor);
            if u16cmp(arena, staged_text, cursor) == 0 {
                staged[28..32].copy_from_slice(&index.to_le_bytes());
                let slot = word(arena, array.wrapping_add(4))
                    .wrapping_add(half(arena, array.wrapping_add(8)) as u32 * 0x28);
                for (i, b) in staged.iter().enumerate() {
                    put_byte(arena, slot.wrapping_add(i as u32), *b);
                }
                put_half(arena, array.wrapping_add(8), next);
                return 0;
            }
            index = index.wrapping_add(len.wrapping_add(1));
            cursor = cursor.wrapping_add(len.wrapping_add(1).wrapping_mul(2));
        }
        if event_has_text(half(arena, event.wrapping_add(10))) {
            let interned = if word(arena, event.wrapping_add(28)) == 0 {
                0xffff_ffff
            } else {
                half(arena, texts.wrapping_add(8)) as u32
            };
            staged[28..32].copy_from_slice(&interned.to_le_bytes());
        }
    }
    let slot = word(arena, array.wrapping_add(4))
        .wrapping_add(half(arena, array.wrapping_add(8)) as u32 * 0x28);
    for (i, b) in staged.iter().enumerate() {
        put_byte(arena, slot.wrapping_add(i as u32), *b);
    }
    put_half(arena, array.wrapping_add(8), next);
    if event_has_text(half(arena, event.wrapping_add(10)))
        && word(arena, event.wrapping_add(28)) != 0
    {
        let len = u16len(arena, word(arena, event.wrapping_add(28)));
        let need = (len.wrapping_add(1) & 0xffff) as u16;
        let grown = half(arena, texts.wrapping_add(8)).wrapping_add(need);
        if half(arena, texts.wrapping_add(10)) < grown {
            status = fetch_entry(arena, obj, 0x3f4, grown as u32, Some(&mut texts), host);
            if status >= 0x8000_0000 {
                return status;
            }
        }
        let dst = word(arena, texts.wrapping_add(4))
            .wrapping_add(half(arena, texts.wrapping_add(8)) as u32 * 2);
        memcpy_at(
            arena,
            dst,
            word(arena, event.wrapping_add(28)),
            (need as u32).wrapping_mul(2),
        );
        put_half(arena, texts.wrapping_add(8), grown);
    }
    0
}

/// fn_2c3e78, EU fn_2c4034, KR fn_2c40e0. Splits the leading span into
/// per-word events. `defaults` is the borrowed 52-byte span-default struct
/// (six separator units, then the template event); `scratch` is 40
/// caller-owned bytes holding the event under construction.
pub fn split_spans(
    arena: &mut [u8],
    handles: u32,
    context: u32,
    offset: u32,
    prefix: u32,
    scratch: u32,
    defaults: &[u8],
    host: &mut impl NormalizationHost,
) -> u32 {
    let separators = &defaults[..12];
    for i in 0..0x28u32 {
        put_byte(
            arena,
            scratch.wrapping_add(i),
            defaults.get(12 + i as usize).copied().unwrap_or(0),
        );
    }
    if handles == 0 || context == 0 {
        return 0x8110_2007;
    }
    let mut texts = 0u32;
    let mut status = fetch_entry(arena, context, 0x3fc, 1, Some(&mut texts), host);
    if status >= 0x8000_0000 {
        if status & 0x1fff == 0x14 {
            status = 0;
        }
        return status;
    }
    if half(arena, texts.wrapping_add(8)) == 0 {
        return 0;
    }
    let mut iterator = 0u32;
    status = open_iterator(
        arena,
        word(arena, handles),
        word(arena, handles.wrapping_add(4)),
        context,
        0x3fd,
        Some(&mut iterator),
        host,
    );
    if status >= 0x8000_0000 {
        return status;
    }
    let text = word(arena, texts.wrapping_add(4));
    let text_count = half(arena, texts.wrapping_add(8)) as u32;
    let (size, base, count, index) = table_params(arena, iterator);
    let mut first = 0i32;
    status = entry_offset(size, base, count, index, 0, &mut first);
    if status >= 0x8000_0000 {
        return finish_spans(arena, iterator, status, host);
    }
    let mut saved = [0u8; 0x28];
    for (i, slot) in saved.iter_mut().enumerate() {
        *slot = byte(arena, (first as u32).wrapping_add(i as u32));
    }
    let inner = word(arena, iterator.wrapping_add(12));
    let mut total = 0u16;
    status = inner_field(Some(&arena[inner as usize..]), Some(&mut total));
    if status >= 0x8000_0000 {
        return finish_spans(arena, iterator, status, host);
    }
    let (size, base, count, index) = table_params(arena, iterator);
    let mut events_addr = 0i32;
    status = entry_offset(size, base, count, index, 0, &mut events_addr);
    if status >= 0x8000_0000 {
        return finish_spans(arena, iterator, status, host);
    }
    let mut events = events_addr as u32;
    let first_addr = first as u32;
    let mut cursor = 1u16;
    while cursor < total
        && word(
            arena,
            events.wrapping_add(cursor as u32 * 0x28).wrapping_add(20),
        ) == word(arena, events.wrapping_add(20))
    {
        cursor = cursor.wrapping_add(1);
    }
    put_word(arena, first_addr.wrapping_add(16), 0);
    put_word(arena, first_addr.wrapping_add(24), 0);
    let tab = word(arena, iterator.wrapping_add(12));
    let mut idx = half(arena, iterator.wrapping_add(16));
    status = set_index(half(arena, tab.wrapping_add(8)), 1, &mut idx);
    put_half(arena, iterator.wrapping_add(16), idx);
    if status >= 0x8000_0000 {
        return finish_spans(arena, iterator, status, host);
    }
    let saved_pos = u32::from_le_bytes([saved[12], saved[13], saved[14], saved[15]]);
    let saved_len = u32::from_le_bytes([saved[16], saved[17], saved[18], saved[19]]);
    let saved_text = u32::from_le_bytes([saved[20], saved[21], saved[22], saved[23]]);
    let mut end = 0u16;
    let mut added = 0u32;
    let mut index = cursor;
    while (end as u32) < text_count {
        while (end as u32) < text_count
            && u16chr(
                separators,
                0,
                half(arena, text.wrapping_add(end as u32 * 2)),
            )
            .is_some()
        {
            end = end.wrapping_add(1);
        }
        let start = end;
        while (end as u32) < text_count
            && u16chr(
                separators,
                0,
                half(arena, text.wrapping_add(end as u32 * 2)),
            )
            .is_none()
        {
            end = end.wrapping_add(1);
        }
        if start < end {
            let mut span_added = 0u32;
            let mut has_break = 0;
            let mut marker = 0u16;
            let mut i = 1u16;
            while i < total {
                let distance = word(arena, events.wrapping_add(i as u32 * 0x28).wrapping_add(20))
                    .wrapping_sub(word(arena, events.wrapping_add(20)));
                if (start as u32) * 2 <= distance
                    && distance <= (end as u32) * 2
                    && half(arena, events.wrapping_add(i as u32 * 0x28).wrapping_add(10)) == 7
                {
                    let value = word(arena, events.wrapping_add(i as u32 * 0x28).wrapping_add(28));
                    if value == 0x2b {
                        has_break = 1;
                    } else if value == 0x0f {
                        marker = i;
                    }
                }
                if distance > (end as u32) * 2 {
                    break;
                }
                i = i.wrapping_add(1);
            }
            if start != end && has_break == 1 && marker != 0 {
                let distance = word(
                    arena,
                    events.wrapping_add(marker as u32 * 0x28).wrapping_add(20),
                )
                .wrapping_sub(word(arena, events.wrapping_add(20)));
                if distance <= (end as u32) * 2 && start < (distance / 2) as u16 {
                    end = (distance / 2) as u16;
                }
            }
            while index < total
                && start != 0
                && word(
                    arena,
                    events.wrapping_add(index as u32 * 0x28).wrapping_add(20),
                )
                .wrapping_sub(word(arena, events.wrapping_add(20)))
                    <= (start as u32) * 2
            {
                if half(
                    arena,
                    events.wrapping_add(index as u32 * 0x28).wrapping_add(10),
                ) != 1
                {
                    added = added.wrapping_add(word(
                        arena,
                        events.wrapping_add(index as u32 * 0x28).wrapping_add(16),
                    ));
                }
                index = index.wrapping_add(1);
            }
            let mut i = index;
            while i < total
                && word(arena, events.wrapping_add(i as u32 * 0x28).wrapping_add(20))
                    .wrapping_sub(word(arena, events.wrapping_add(20)))
                    > (start as u32) * 2
                && word(arena, events.wrapping_add(i as u32 * 0x28).wrapping_add(20))
                    .wrapping_sub(word(arena, events.wrapping_add(20)))
                    < (end as u32) * 2
            {
                if half(arena, events.wrapping_add(i as u32 * 0x28).wrapping_add(10)) != 1 {
                    span_added = span_added.wrapping_add(word(
                        arena,
                        events.wrapping_add(i as u32 * 0x28).wrapping_add(16),
                    ));
                }
                i = i.wrapping_add(1);
            }
            let text_len = (end.wrapping_sub(start) as u32).wrapping_mul(2);
            put_word(
                arena,
                scratch.wrapping_add(12),
                saved_pos
                    .wrapping_add(offset)
                    .wrapping_add(added.wrapping_add(start as u32 * 2)),
            );
            put_word(
                arena,
                scratch.wrapping_add(16),
                text_len.wrapping_add(span_added),
            );
            put_word(
                arena,
                scratch.wrapping_add(20),
                saved_text.wrapping_add(start as u32 * 2),
            );
            put_word(arena, scratch.wrapping_add(24), text_len);
            if saved_pos.wrapping_add(saved_len)
                < word(arena, scratch.wrapping_add(12))
                    .wrapping_add(word(arena, scratch.wrapping_add(16)))
            {
                put_word(
                    arena,
                    scratch.wrapping_add(16),
                    saved_pos
                        .wrapping_add(saved_len)
                        .wrapping_sub(word(arena, scratch.wrapping_add(12))),
                );
            }
            status = insert_ordered(arena, iterator, scratch, host);
            if status >= 0x8000_0000 {
                return finish_spans(arena, iterator, status, host);
            }
            let inner = word(arena, iterator.wrapping_add(12));
            status = inner_field(Some(&arena[inner as usize..]), Some(&mut total));
            if status >= 0x8000_0000 {
                return finish_spans(arena, iterator, status, host);
            }
            let (size, base, count, index) = table_params(arena, iterator);
            let mut bumped = 0i32;
            status = entry_offset(size, base, count, index, 0, &mut bumped);
            if status >= 0x8000_0000 {
                return finish_spans(arena, iterator, status, host);
            }
            events = bumped as u32;
        }
    }
    let (size, base, count, index) = table_params(arena, iterator);
    let mut first_addr2 = 0i32;
    status = entry_offset(size, base, count, index, 0, &mut first_addr2);
    if status >= 0x8000_0000 {
        return finish_spans(arena, iterator, status, host);
    }
    let first_addr2 = first_addr2 as u32;
    saved[16..20].copy_from_slice(&saved_len.wrapping_add(prefix).to_le_bytes());
    saved[12..16].copy_from_slice(&saved_pos.wrapping_sub(prefix).to_le_bytes());
    for (i, b) in saved.iter().enumerate() {
        put_byte(arena, first_addr2.wrapping_add(i as u32), *b);
    }
    finish_spans(arena, iterator, status, host)
}

fn finish_spans(
    arena: &mut [u8],
    iterator: u32,
    status: u32,
    host: &mut impl NormalizationHost,
) -> u32 {
    if iterator != 0 {
        release_iterator(arena, iterator, host);
    }
    status
}

fn next_marker(arena: &[u8], list: u32, index: &mut u16) -> u32 {
    if list == 0 {
        return 0x8110_2007;
    }
    let events = word(arena, list.wrapping_add(4));
    let count = half(arena, list.wrapping_add(8));
    let mut i = *index;
    if i < count {
        loop {
            *index = i.wrapping_add(1);
            i = *index;
            if !(i < count
                && half(arena, events.wrapping_add(i as u32 * 0x28).wrapping_add(10)) != 7)
            {
                break;
            }
        }
    }
    0
}

/// fn_2bd880, EU fn_2bda3c, KR fn_2bdae8. Finds the timeline window covering
/// the indexed marker. The timeline scale must be nonzero.
pub fn timeline_window(
    arena: &[u8],
    timeline: u32,
    handle: u32,
    start: &mut u32,
    end: &mut u32,
    index: &mut u16,
) -> u32 {
    if timeline == 0 || handle == 0 {
        return 0x8110_2007;
    }
    let old_start = *start;
    let old_end = *end;
    let old_index = *index;
    *end = half(arena, timeline.wrapping_add(8)) as u32;
    *start = half(arena, timeline.wrapping_add(8)) as u32;
    let mut context = 0u32;
    let status = field_value(Some(&arena[handle as usize..]), Some(&mut context));
    if status >= 0x8000_0000 {
        return status;
    }
    let list = context;
    let events = word(arena, list.wrapping_add(4));
    let count = half(arena, list.wrapping_add(8));
    if *index < count {
        let scale = half(arena, timeline.wrapping_add(2)) as u32;
        let mut i = 0u16;
        let mut active = 0;
        while i < count
            && word(arena, events.wrapping_add(i as u32 * 0x28).wrapping_add(20))
                <= old_end
                    .wrapping_mul(scale)
                    .wrapping_add(word(arena, events.wrapping_add(20)))
        {
            let event = events.wrapping_add(i as u32 * 0x28);
            *index = i;
            if half(arena, event.wrapping_add(10)) == 7 {
                active = if word(arena, event.wrapping_add(28)) == 43 && active == 0 {
                    1
                } else {
                    0
                };
            }
            i = i.wrapping_add(1);
        }
        if active != 0 {
            let status = next_marker(arena, list, index);
            if status >= 0x8000_0000 {
                return status;
            }
            if *index >= count {
                return 0;
            }
            if half(
                arena,
                events.wrapping_add(*index as u32 * 0x28).wrapping_add(10),
            ) != 7
            {
                return 0;
            }
        }
        let first = *index;
        let status = next_marker(arena, list, index);
        if status >= 0x8000_0000 {
            return status;
        }
        let base = word(arena, events.wrapping_add(20));
        let mut lower = udiv(
            word(
                arena,
                events.wrapping_add(first as u32 * 0x28).wrapping_add(20),
            )
            .wrapping_sub(base),
            scale,
        );
        let next = *index;
        let mut upper = if next < count {
            udiv(
                word(
                    arena,
                    events.wrapping_add(next as u32 * 0x28).wrapping_add(20),
                )
                .wrapping_sub(base),
                scale,
            )
        } else {
            half(arena, timeline.wrapping_add(8)) as u32
        };
        if lower != upper {
            if next < count {
                *end = upper;
            }
            *start = lower;
            if lower == old_start && *end == old_end && *index == old_index {
                *start = half(arena, timeline.wrapping_add(8)) as u32;
                *end = half(arena, timeline.wrapping_add(8)) as u32;
            }
            return 0;
        }
        let mut current: u32;
        while {
            current = *index as u32;
            current < count as u32 && lower == upper
        } {
            if half(arena, events.wrapping_add(current * 0x28).wrapping_add(10)) == 7
                && word(arena, events.wrapping_add(i as u32 * 0x28).wrapping_add(28)) == 43
            {
                if list == 0 {
                    return 0x8110_2007;
                }
                if *index < count {
                    loop {
                        *index = index.wrapping_add(1);
                        if !(*index < count
                            && half(
                                arena,
                                events.wrapping_add(*index as u32 * 0x28).wrapping_add(10),
                            ) != 7)
                        {
                            break;
                        }
                    }
                    if half(
                        arena,
                        events.wrapping_add(*index as u32 * 0x28).wrapping_add(10),
                    ) != 7
                    {
                        return 0;
                    }
                }
            }
            let first = *index;
            let status = next_marker(arena, list, index);
            if status >= 0x8000_0000 {
                return status;
            }
            lower = udiv(
                word(
                    arena,
                    events.wrapping_add(first as u32 * 0x28).wrapping_add(20),
                )
                .wrapping_sub(base),
                scale,
            );
            upper = if (*index) < count {
                udiv(
                    word(
                        arena,
                        events.wrapping_add(*index as u32 * 0x28).wrapping_add(20),
                    )
                    .wrapping_sub(base),
                    scale,
                )
            } else {
                udiv(base, scale)
            };
        }
        *start = lower;
        if (current as u16) < count {
            *end = upper;
        }
    }
    if *start == old_start && *end == old_end {
        *start = half(arena, timeline.wrapping_add(8)) as u32;
        *end = half(arena, timeline.wrapping_add(8)) as u32;
    }
    0
}

/// fn_2bf244, EU fn_2bf400, KR fn_2bf4ac. Reads the dictionary header tables
/// into heap arrays and attaches the data chunk.
pub fn read_dictionary(
    arena: &mut [u8],
    object: u32,
    reader: u32,
    data: u32,
    mode: u32,
    alloc: &mut impl NormalizationHost,
    chunks: &mut impl ChunkHost,
) -> u32 {
    if half(arena, object.wrapping_add(48)) != 0 {
        return 0;
    }
    let env = word(arena, object.wrapping_add(8));
    let heap = word(arena, env.wrapping_add(4));
    let first_size = word(arena, data);
    let header_size = word(arena, data.wrapping_add(4));
    let first = 8u32.wrapping_add(header_size);
    let mut position = first.wrapping_add(first_size);
    let second_size = word(arena, data.wrapping_add(position));
    position = position.wrapping_add(4);
    let header_size = word(arena, data.wrapping_add(position));
    let second = position.wrapping_add(4).wrapping_add(header_size);
    position = second.wrapping_add(second_size);
    let raw_count = half(arena, data.wrapping_add(position));
    position = position.wrapping_add(2);
    put_half(arena, object.wrapping_add(48), raw_count >> 1);
    let count = half(arena, object.wrapping_add(48));
    // The original stores the calloc result before testing it.
    let entries = alloc
        .calloc(
            arena,
            heap,
            1,
            (count as u32).wrapping_mul(8).wrapping_add(1),
        )
        .unwrap_or(0);
    put_word(arena, object.wrapping_add(52), entries);
    if entries == 0 {
        return cleanup_dictionary(arena, object, heap, alloc, chunks, 0x8110_200a);
    }
    let mut i = 0u16;
    let mut output = 0u32;
    while (i as u32) < (count as u32).wrapping_mul(2) {
        let kind = half(arena, data.wrapping_add(position));
        position = position.wrapping_add(2);
        let value = word(arena, data.wrapping_add(position));
        position = position.wrapping_add(4);
        if (mode == 1 && (i as u32) < count as u32) || (mode == 0 && (i as u32) >= count as u32) {
            let slot = entries.wrapping_add(output.wrapping_mul(8));
            put_word(arena, slot, value);
            put_byte(arena, slot.wrapping_add(4), kind as u8);
            output = output.wrapping_add(1);
        }
        i = i.wrapping_add(1);
    }
    let dictionary_count = half(arena, data.wrapping_add(position));
    position = position.wrapping_add(2);
    put_half(arena, object.wrapping_add(64), dictionary_count);
    let dictionary = alloc
        .calloc(
            arena,
            heap,
            1,
            (dictionary_count as u32).wrapping_mul(2).wrapping_add(1),
        )
        .unwrap_or(0);
    put_word(arena, object.wrapping_add(60), dictionary);
    if dictionary == 0 {
        return cleanup_dictionary(arena, object, heap, alloc, chunks, 0x8110_200a);
    }
    let mut i = 0u16;
    while i < dictionary_count {
        put_half(
            arena,
            dictionary.wrapping_add(i as u32 * 2),
            half(arena, data.wrapping_add(position)),
        );
        position = position.wrapping_add(2);
        i = i.wrapping_add(1);
    }
    put_half(arena, object.wrapping_add(72), 0);
    put_half(
        arena,
        object.wrapping_add(72),
        half(arena, data.wrapping_add(position)),
    );
    position = position.wrapping_add(2);
    if half(arena, object.wrapping_add(72)) != 0 {
        let range_count = half(arena, object.wrapping_add(72));
        let ranges = alloc
            .calloc(
                arena,
                heap,
                1,
                (range_count as u32).wrapping_mul(2).wrapping_add(1),
            )
            .unwrap_or(0);
        put_word(arena, object.wrapping_add(68), ranges);
        if ranges == 0 {
            return cleanup_dictionary(arena, object, heap, alloc, chunks, 0x8110_200a);
        }
        let mut i = 0u16;
        while i < range_count {
            put_half(
                arena,
                ranges.wrapping_add(i as u32 * 2),
                half(arena, data.wrapping_add(position)),
            );
            position = position.wrapping_add(2);
            i = i.wrapping_add(1);
        }
    }
    let extra_count = half(arena, data.wrapping_add(position));
    position = position.wrapping_add(2);
    let extra = alloc
        .calloc(
            arena,
            heap,
            1,
            (extra_count as u32).wrapping_mul(2).wrapping_add(1),
        )
        .unwrap_or(0);
    put_word(arena, object.wrapping_add(76), extra);
    if extra == 0 {
        return cleanup_dictionary(arena, object, heap, alloc, chunks, 0x8110_200a);
    }
    let mut i = 0u16;
    while i < extra_count {
        put_half(
            arena,
            extra.wrapping_add(i as u32 * 2),
            half(arena, data.wrapping_add(position)),
        );
        position = position.wrapping_add(2);
        i = i.wrapping_add(1);
    }
    put_byte(
        arena,
        object.wrapping_add(80),
        half(arena, data.wrapping_add(position)) as u8,
    );
    if byte(arena, object.wrapping_add(80)) == 0 {
        return cleanup_dictionary(arena, object, heap, alloc, chunks, 0x8110_2007);
    }
    let mut record = word(arena, object.wrapping_add(44));
    let mut chunk_data = data;
    let status = chunks.detach(arena, reader, &mut record, &mut chunk_data);
    put_word(arena, object.wrapping_add(44), record);
    if status >= 0x8000_0000 {
        return status;
    }
    put_word(
        arena,
        object.wrapping_add(56),
        chunk_data.wrapping_add(if mode == 1 { first } else { second }),
    );
    0
}

fn cleanup_dictionary(
    arena: &mut [u8],
    object: u32,
    heap: u32,
    alloc: &mut impl NormalizationHost,
    chunks: &mut impl ChunkHost,
    status: u32,
) -> u32 {
    if half(arena, object.wrapping_add(48)) != 0 {
        if word(arena, object.wrapping_add(52)) != 0 {
            alloc.free(arena, heap, word(arena, object.wrapping_add(52)));
            put_word(arena, object.wrapping_add(52), 0);
        }
        put_word(arena, object.wrapping_add(56), 0);
        put_half(arena, object.wrapping_add(48), 0);
        if half(arena, object.wrapping_add(64)) != 0 {
            if word(arena, object.wrapping_add(60)) != 0 {
                alloc.free(arena, heap, word(arena, object.wrapping_add(60)));
                put_word(arena, object.wrapping_add(60), 0);
            }
            put_half(arena, object.wrapping_add(64), 0);
        }
        if half(arena, object.wrapping_add(72)) != 0 {
            if word(arena, object.wrapping_add(68)) != 0 {
                alloc.free(arena, heap, word(arena, object.wrapping_add(68)));
                put_word(arena, object.wrapping_add(68), 0);
            }
            put_half(arena, object.wrapping_add(72), 0);
        }
        if word(arena, object.wrapping_add(76)) != 0 {
            alloc.free(arena, heap, word(arena, object.wrapping_add(76)));
            put_word(arena, object.wrapping_add(76), 0);
        }
        if word(arena, object.wrapping_add(44)) != 0
            && chunks.release(word(arena, object.wrapping_add(44))) < 0x8000_0000
        {
            put_word(arena, object.wrapping_add(44), 0);
        }
    }
    status
}

fn contains_stop_scan(
    arena: &mut [u8],
    list: u32,
    timeline: u32,
    middle: u16,
    separators: &[u8],
    scan_base: u32,
) -> bool {
    if list == 0 || timeline == 0 {
        return false;
    }
    let events = word(arena, list.wrapping_add(4));
    let scale = half(arena, timeline.wrapping_add(2));
    let table = word(arena, timeline.wrapping_add(4));
    put_half(arena, scan_base, middle);
    put_half(arena, scan_base.wrapping_add(2), middle);
    crate::normalization::skip_matched_back(arena, table, scan_base, separators);
    crate::normalization::skip_matched(
        arena,
        table,
        scan_base.wrapping_add(2),
        half(arena, timeline.wrapping_add(8)) as u32,
        separators,
    );
    let lower = half(arena, scan_base).wrapping_mul(scale);
    let upper = half(arena, scan_base.wrapping_add(2)).wrapping_mul(scale);
    let mut found = false;
    let mut j = 0u16;
    while j < half(arena, list.wrapping_add(8)) && !found {
        let event = events.wrapping_add(j as u32 * 0x28);
        let relative = word(arena, event.wrapping_add(20))
            .wrapping_sub(word(arena, events.wrapping_add(20))) as u16;
        if relative > upper {
            return false;
        }
        if half(arena, event.wrapping_add(10)) == 3
            && word(arena, event.wrapping_add(28)) == 0
            && relative >= lower
        {
            found = true;
        }
        j = j.wrapping_add(1);
    }
    found
}

/// fn_2c3db8, EU fn_2c3f74, KR fn_2c4020. Allocates an 8-byte handle item for
/// the `(h0, h1)` pair.
pub fn alloc_handle_item(
    arena: &mut [u8],
    h0: u32,
    h1: u32,
    out: u32,
    host: &mut impl NormalizationHost,
) -> u32 {
    if out == 0 {
        return 0x8110_2007;
    }
    let (status, ctx) = host.resolve(arena, h0, h1);
    if status >= 0x8000_0000 {
        return 0x8110_2007;
    }
    put_word(arena, out, 0);
    let heap = word(arena, ctx.wrapping_add(4));
    let Some(item) = host.alloc(arena, heap, 8) else {
        return 0x8110_200a;
    };
    put_word(arena, item, h0);
    put_word(arena, item.wrapping_add(4), h1);
    put_word(arena, out, item);
    0
}

/// fn_2c6264, EU fn_2c6420, KR fn_2c64cc. Allocates a 20-byte break config.
pub fn alloc_break_config(
    arena: &mut [u8],
    object: u32,
    key: u32,
    out: u32,
    host: &mut impl NormalizationHost,
) -> u32 {
    if out == 0 {
        return 0x8110_2007;
    }
    let (status, ctx) = host.resolve(arena, object, key);
    if status >= 0x8000_0000 {
        return 0x8110_2007;
    }
    put_word(arena, out, 0);
    let heap = word(arena, ctx.wrapping_add(4));
    let Some(config) = host.alloc(arena, heap, 20) else {
        return 0x8110_200a;
    };
    for index in 0..20 {
        put_byte(arena, config.wrapping_add(index as u32), 0);
    }
    put_word(arena, config, object);
    put_word(arena, config.wrapping_add(4), key);
    put_word(arena, config.wrapping_add(8), ctx);
    put_half(arena, config.wrapping_add(16), 0);
    put_word(arena, out, config);
    0
}

/// Scripted token-store vtable calls for the TOKENTN tokenizer
/// (`fn_283528` unit): `find`/`get`/`next`/`create`/`set`/`getString`.
/// Handles are passed as their two words; all pointers are arena offsets.
pub trait TokenStoreHost {
    /// `tokens->find(handle, kind, arg, out)`: writes the token half.
    fn token_find(
        &mut self,
        arena: &mut [u8],
        h0: u32,
        h1: u32,
        kind: u32,
        arg: u32,
        out: u32,
    ) -> u32;
    /// `tokens->get(handle, rec, field, len, out, out_count)`: the stored
    /// width follows the field (field 0 reads a byte, the rest halves).
    fn token_get(
        &mut self,
        arena: &mut [u8],
        h0: u32,
        h1: u32,
        rec: u32,
        field: u32,
        len: u32,
        out: u32,
        out_count: u32,
    ) -> u32;
    /// `tokens->next(handle, tok, out)`: writes the next token half.
    fn token_next(&mut self, arena: &mut [u8], h0: u32, h1: u32, tok: u32, out: u32) -> u32;
    /// `tokens->create(handle, token, out)`: writes the record half.
    fn token_create(&mut self, arena: &mut [u8], h0: u32, h1: u32, tok: u32, out: u32) -> u32;
    /// `tokens->set(handle, rec, field, len, data, out_written)`.
    fn token_set(
        &mut self,
        arena: &mut [u8],
        h0: u32,
        h1: u32,
        rec: u32,
        field: u32,
        len: u32,
        data: u32,
        out_written: u32,
    ) -> u32;
    /// `tokens->getString(handle, rec, field, out_text, out_count)`.
    fn token_get_string(
        &mut self,
        arena: &mut [u8],
        h0: u32,
        h1: u32,
        rec: u32,
        field: u32,
        out_text: u32,
        out_count: u32,
    ) -> u32;
}

/// Scripted dictionary vtable calls for the TOKENTN tokenizer:
/// `lookupRules`/`lookup`/`normalize`/`fold`/`isSpace`.
pub trait TokenDictHost {
    /// `lookupRules(dictionary, table, key, out_rules, out_count, sep, mode)`.
    fn dict_lookup_rules(
        &mut self,
        arena: &mut [u8],
        h0: u32,
        h1: u32,
        table: u32,
        key: u32,
        out_rules: u32,
        out_count: u32,
        out_sep: u32,
        mode: u32,
    ) -> u32;
    /// `lookup(dictionary, text, kind, domain, out_entries, out_count)`.
    #[allow(clippy::too_many_arguments)]
    fn dict_lookup(
        &mut self,
        arena: &mut [u8],
        h0: u32,
        h1: u32,
        text: u32,
        kind: u32,
        domain: u32,
        out_entries: u32,
        out_count: u32,
    ) -> u32;
    /// `normalize(dictionary, domain, text)`: rewrites the text in place.
    fn dict_normalize(&mut self, arena: &mut [u8], h0: u32, h1: u32, domain: u32, text: u32) -> u32;
    /// `fold(dictionary, domain, text)`: rewrites the text in place.
    fn dict_fold(&mut self, arena: &mut [u8], h0: u32, h1: u32, domain: u32, text: u32) -> u32;
    /// `isSpace(dictionary, c)`: nonzero when `c` is a space character.
    fn dict_is_space(&mut self, arena: &[u8], h0: u32, h1: u32, c: u8) -> u32;
}

/// Scripted context vtable calls for the TOKENTN tokenizer:
/// `setText`/`run`/`getText`.
pub trait TokenContextHost {
    /// `setText(context, layer, text, len)`.
    fn context_set_text(
        &mut self,
        arena: &mut [u8],
        h0: u32,
        h1: u32,
        layer: u32,
        text: u32,
        len: u32,
    ) -> u32;
    /// `run(context, name)`.
    fn context_run(&mut self, arena: &mut [u8], h0: u32, h1: u32, name: u32) -> u32;
    /// `getText(context, layer, out_text, out_len)`.
    fn context_get_text(
        &mut self,
        arena: &mut [u8],
        h0: u32,
        h1: u32,
        layer: u32,
        out_text: u32,
        out_len: u32,
    ) -> u32;
}

/// fn_195d58. Compares two NUL-terminated byte strings, like strcmp:
/// negative, zero, or positive.
fn strcmp8(data: &[u8], a: u32, b: u32) -> i32 {
    let mut i = 0u32;
    loop {
        let x = byte(data, a.wrapping_add(i));
        let y = byte(data, b.wrapping_add(i));
        if x != y {
            return (x as i32) - (y as i32);
        }
        if x == 0 {
            return 0;
        }
        i = i.wrapping_add(1);
    }
}

/// fn_195d30. Finds the first occurrence of byte `c` in the string at
/// `base`, returning its offset, or 0 when absent.
fn strchr8(data: &[u8], base: u32, c: u8) -> u32 {
    if base == 0 {
        return 0;
    }
    let mut p = base;
    loop {
        let x = byte(data, p);
        if x == c {
            return p;
        }
        if x == 0 {
            return 0;
        }
        p = p.wrapping_add(1);
    }
}

/// fn_195cd0. Copies the NUL-terminated string at `src` to `dst`.
fn strcpy_at(arena: &mut [u8], dst: u32, src: u32) {
    memcpy_at(arena, dst, src, strlen8(arena, src).wrapping_add(1));
}

/// fn_1960a8. Finds the `needle` string in `haystack`, returning the match
/// offset, or 0 when absent. Both are NUL-terminated arena strings.
fn strstr8(data: &[u8], haystack: u32, needle: u32) -> u32 {
    strstr_at(data, haystack, needle).unwrap_or(0)
}

/// fn_176cd8. Parses an optionally signed, space-padded decimal prefix:
/// leading blanks/tabs skipped, one optional `+`, then digits evaluated
/// right to left; any other character yields 0.
fn parse_counted(data: &[u8], base: u32) -> i32 {
    if base == 0 {
        return 0;
    }
    let len = strlen8(data, base) as i32;
    let mut start = 0i32;
    while start <= len - 1
        && (byte(data, base.wrapping_add(start as u32)) == b' '
            || byte(data, base.wrapping_add(start as u32)) == b'\t')
    {
        start += 1;
    }
    let mut i = start;
    if start < len - 1 && byte(data, base.wrapping_add(i as u32)) == b'+' {
        i = start + 1;
    }
    while i <= len - 1 {
        let c = byte(data, base.wrapping_add(i as u32));
        if !(b'0'..=b'9').contains(&c) {
            break;
        }
        i += 1;
    }
    i -= 1;
    let n = i - start + 2;
    let mut k = 1i32;
    let mut val = 0i32;
    let mut mul = 1i32;
    if n > 1 {
        loop {
            let d = byte(data, base.wrapping_add(i as u32)).wrapping_sub(b'0') as u32;
            if d <= 9 {
                val = val.wrapping_add((d as i32).wrapping_mul(mul));
                mul = mul.wrapping_mul(10);
            } else if !(i == start && byte(data, base.wrapping_add(i as u32)) == b'+') {
                return 0;
            }
            k += 1;
            i -= 1;
            if n <= k {
                break;
            }
        }
    }
    val
}

/// Scratch frame layout for `tokenize_span` (`fn_284bc0`): word,
/// lookupText, normalizedPhrase, normalizedWord, foldedText, tokenText,
/// wordCounts, then the scalar outs. 0x480 bytes.
pub const TOKENIZE_FRAME: u32 = 0x480;
/// Offsets within the `tokenize_span` scratch frame.
pub const TOKENIZE_WORD: u32 = 0;
/// Offset of `lookupText[256]` in the frame.
pub const TOKENIZE_LOOKUP: u32 = 128;
/// Offset of `normalizedPhrase[256]` in the frame.
pub const TOKENIZE_PHRASE: u32 = 384;
/// Offset of `normalizedWord[128]` in the frame.
pub const TOKENIZE_NORMWORD: u32 = 640;
/// Offset of `foldedText[128]` in the frame.
pub const TOKENIZE_FOLDED: u32 = 768;
/// Offset of `tokenText[128]` in the frame.
pub const TOKENIZE_TOKENTEXT: u32 = 896;
/// Offset of `wordCounts[64]` in the frame.
pub const TOKENIZE_COUNTS: u32 = 1024;
/// Offset of the scalar outs (`readCount`, `nextToken`, `kind`,
/// `nextKind`, `tokenEnd`, `matchedEnd`, `entries`, `entryCount`,
/// field-1 `position`) in the frame.
pub const TOKENIZE_OUTS: u32 = 1088;
/// Offset of the `entries` pointer cell in the frame.
pub const TOKENIZE_ENTRIES: u32 = 1100;
/// Offset of the `entryCount` cell in the frame.
pub const TOKENIZE_ENTRY_COUNT: u32 = 1104;
/// Offset of the field-1 `position` cell in the frame.
pub const TOKENIZE_POSITION: u32 = 1106;
/// Offset of the `kind` string scratch (`"femwtn"`) in the frame.
pub const TOKENIZE_KIND: u32 = 1108;
/// Offset of the `table` string scratch (`"tn"`) in the frame.
pub const TOKENIZE_TABLE: u32 = 1116;

/// Scratch frame layout for `match_rules` (`fn_283528`): the 128-byte
/// context name, the 12-byte separator, then the scalar out cells.
/// 0xB0 bytes.
pub const MATCH_FRAME: u32 = 0xB0;
/// Offset of `contextName[128]` in the frame.
pub const MATCH_CONTEXT_NAME: u32 = 0;
/// Offset of `separator[12]` in the frame.
pub const MATCH_SEPARATOR: u32 = 128;
/// Offset of the `rules` pointer cell in the frame.
pub const MATCH_RULES_PTR: u32 = 140;
/// Offset of the `ruleCount` cell in the frame.
pub const MATCH_RULE_COUNT: u32 = 144;
/// Offset of the `readCount` cell in the frame.
pub const MATCH_READ_COUNT: u32 = 146;
/// Offset of the `writeCount` cell in the frame.
pub const MATCH_WRITE_COUNT: u32 = 148;
/// Offset of the `firstToken` cell in the frame.
pub const MATCH_FIRST_TOKEN: u32 = 150;
/// Offset of the `record` cell in the frame.
pub const MATCH_RECORD: u32 = 152;
/// Offset of the `recordType` cell in the frame.
pub const MATCH_RECORD_TYPE: u32 = 154;
/// Offset of the `recordText` pointer cell in the frame.
pub const MATCH_RECORD_TEXT: u32 = 156;
/// Offset of the `recordStart` cell in the frame.
pub const MATCH_RECORD_START: u32 = 160;
/// Offset of the `recordEnd` cell in the frame.
pub const MATCH_RECORD_END: u32 = 162;
/// Offset of the `output` pointer cell in the frame.
pub const MATCH_OUTPUT: u32 = 164;
/// Offset of the `outputLength` cell in the frame.
pub const MATCH_OUTPUT_LEN: u32 = 168;
/// Offset of the `start` value cell in the frame.
pub const MATCH_START: u32 = 170;
/// Offset of the `tokenEnd` value cell in the frame.
pub const MATCH_TOKEN_END: u32 = 172;

/// fn_2409d4. Quicksort of `n` elements of `size` bytes at `base` with the
/// C comparator `cmp(a, b)` (an integer like `fn_284ba8`'s 0/1), mirroring
/// the C partition with pivot tracking exactly, including `< 0` / `> 0`
/// loop tests and recursion structure.
fn quicksort_by(arena: &mut [u8], base: u32, n: u32, size: u32, cmp: fn(u8, u8) -> i32) {
    if n <= 1 {
        return;
    }
    let mut mid = n >> 1;
    let mut pivot = base.wrapping_add(mid.wrapping_mul(size));
    let mut i = 0u32;
    let mut j = n.wrapping_sub(1);
    while i < j {
        while cmp(
            byte(arena, base.wrapping_add(i.wrapping_mul(size))),
            byte(arena, pivot),
        ) < 0
        {
            i = i.wrapping_add(1);
        }
        while cmp(
            byte(arena, pivot),
            byte(arena, base.wrapping_add(j.wrapping_mul(size))),
        ) > 0
        {
            j = j.wrapping_sub(1);
        }
        if i < j {
            let pi = base.wrapping_add(i.wrapping_mul(size));
            let pj = base.wrapping_add(j.wrapping_mul(size));
            let mut k = 0u32;
            while k < size {
                let t = byte(arena, pi.wrapping_add(k));
                put_byte(arena, pi.wrapping_add(k), byte(arena, pj.wrapping_add(k)));
                put_byte(arena, pj.wrapping_add(k), t);
                k = k.wrapping_add(1);
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
        quicksort_by(arena, base, j.wrapping_add(1), size, cmp);
    }
    if i < n.wrapping_sub(1) {
        quicksort_by(
            arena,
            base.wrapping_add(i.wrapping_mul(size)),
            n.wrapping_sub(i),
            size,
            cmp,
        );
    }
}

fn is_tn_space(c: u8) -> bool {
    c == b' ' || c == b'\t' || c == b'\n' || c == b'\r' || c == b'_'
}

/// fn_195e38. Copies the string at `src` to `dst`, padding with NULs to
/// exactly `n` bytes; when no NUL appears in the first `n` bytes it copies
/// `n` bytes flat, like strncpy.
pub(crate) fn strncpy_at(arena: &mut [u8], dst: u32, src: u32, n: u32) {
    let mut d = dst;
    let mut s = src;
    let mut remaining = n;
    loop {
        if remaining == 0 {
            return;
        }
        let c = byte(arena, s);
        put_byte(arena, d, c);
        d = d.wrapping_add(1);
        s = s.wrapping_add(1);
        if c == 0 {
            break;
        }
        remaining = remaining.wrapping_sub(1);
    }
    // The NUL is copied without consuming `remaining`: pad the rest.
    let mut pad = remaining.wrapping_sub(1);
    while pad != 0 {
        put_byte(arena, d, 0);
        d = d.wrapping_add(1);
        pad = pad.wrapping_sub(1);
    }
}

/// fn_17c550, EU fn_17c600, KR fn_17c5ac. Parses the signed decimal value
/// of the string at `base` with wrapping arithmetic. The skip/digit
/// classes are the `D_8dc30c` masks verified from the ROM (bit 8 set
/// exactly on 9, 10, 11, 12, 13, 32; bit 4 exactly on digits).
fn atoi8(data: &[u8], base: u32) -> i32 {
    let mut p = base;
    while matches!(byte(data, p), 9 | 10 | 11 | 12 | 13 | 32) {
        p = p.wrapping_add(1);
    }
    let mut n = 0i32;
    let mut c = byte(data, p);
    p = p.wrapping_add(1);
    let sign = c;
    if c == b'-' || c == b'+' {
        c = byte(data, p);
        p = p.wrapping_add(1);
    }
    while c.is_ascii_digit() {
        n = (c as i32)
            .wrapping_add(n.wrapping_mul(10))
            .wrapping_sub(b'0' as i32);
        c = byte(data, p);
        p = p.wrapping_add(1);
    }
    if sign == b'-' {
        n.wrapping_neg()
    } else {
        n
    }
}

/// Copies a static NUL-terminated literal into the arena: stands in for a
/// C string literal with no arena address.
fn strcpy_static(arena: &mut [u8], dst: u32, lit: &[u8]) {
    let mut i = 0u32;
    loop {
        let c = lit[i as usize];
        put_byte(arena, dst.wrapping_add(i), c);
        if c == 0 {
            break;
        }
        i = i.wrapping_add(1);
    }
}

/// Appends one byte to the arena string at `dst`, like strcat of a
/// one-character literal.
fn strcat_char(arena: &mut [u8], dst: u32, c: u8) {
    let mut d = dst;
    while byte(arena, d) != 0 {
        d = d.wrapping_add(1);
    }
    put_byte(arena, d, c);
    put_byte(arena, d.wrapping_add(1), 0);
}

fn is_empty_str(arena: &[u8], p: u32) -> bool {
    byte(arena, p) == 0
}

fn streq(arena: &[u8], a: u32, lit: &[u8]) -> bool {
    let mut i = 0u32;
    loop {
        let c = lit[i as usize];
        if byte(arena, a.wrapping_add(i)) != c {
            return false;
        }
        if c == 0 {
            return true;
        }
        i = i.wrapping_add(1);
    }
}

/// fn_283528, EU fn_2836e4, KR fn_283790. Matches one word against the
/// dictionary rule chain for `table`: splits each candidate rule on the
/// separator, validates domain/case/punctuation/context, keeps the
/// highest-priority match (at most two), and on a single match emits a DCT
/// token record and advances `matched_end`. `start`/`token` arrive
/// full-width like the ARM and are narrowed to the C's u16 locals.
/// `scratch` is a `MATCH_FRAME`-byte region.
#[allow(clippy::too_many_arguments)]
pub fn match_rules(
    arena: &mut [u8],
    ctx: u32,
    key: u32,
    text: u32,
    table: u32,
    domain: u32,
    source: u32,
    start: u32,
    token: u32,
    h0: u32,
    h1: u32,
    matched_end: u32,
    scratch: u32,
    host: &mut (impl NormalizationHost + TokenStoreHost + TokenDictHost + TokenContextHost),
) -> u32 {
    let context_name = scratch.wrapping_add(MATCH_CONTEXT_NAME);
    let separator = scratch.wrapping_add(MATCH_SEPARATOR);
    let rules_cell = scratch.wrapping_add(MATCH_RULES_PTR);
    let rule_count_cell = scratch.wrapping_add(MATCH_RULE_COUNT);
    let read_count_cell = scratch.wrapping_add(MATCH_READ_COUNT);
    let write_count_cell = scratch.wrapping_add(MATCH_WRITE_COUNT);
    let first_token_cell = scratch.wrapping_add(MATCH_FIRST_TOKEN);
    let record_cell = scratch.wrapping_add(MATCH_RECORD);
    let record_type_cell = scratch.wrapping_add(MATCH_RECORD_TYPE);
    let record_text_cell = scratch.wrapping_add(MATCH_RECORD_TEXT);
    let record_start_cell = scratch.wrapping_add(MATCH_RECORD_START);
    let record_end_cell = scratch.wrapping_add(MATCH_RECORD_END);
    let output_cell = scratch.wrapping_add(MATCH_OUTPUT);
    let output_len_cell = scratch.wrapping_add(MATCH_OUTPUT_LEN);
    let start_cell = scratch.wrapping_add(MATCH_START);
    let token_end_cell = scratch.wrapping_add(MATCH_TOKEN_END);
    let dict_h0 = word(arena, ctx.wrapping_add(8));
    let dict_h1 = word(arena, ctx.wrapping_add(12));
    let context_h0 = word(arena, ctx.wrapping_add(20));
    let context_h1 = word(arena, ctx.wrapping_add(24));
    let lookup_mode = byte(arena, ctx.wrapping_add(40));
    // The ROM compares `record == token` with the unnarrowed register
    // (target fn_283528+0xa14: cmp r2, sl; sl holds the wide arg), so the
    // wide value threads through to context_check while the u16 copy feeds
    // the narrowed uses (record init, create id).
    let token_wide = token;
    let token = (token & 0xffff) as u16;
    let mut start = (start & 0xffff) as u16;
    let mut priority = 1u16;
    let mut record_type = 4u8;
    let mut record = token;
    let mut selected = 0u32;
    let mut rule_count = 0u16;
    let mut matches = 0u16;
    let mut token_end =
        (start as u32).wrapping_add(strlen8(arena, text)) as u16;
    let original_end = token_end;
    let mut status = host.token_find(arena, h0, h1, 1, 0, first_token_cell);
    if status >= 0x8000_0000 {
        return status;
    }
    let first_token = half(arena, first_token_cell);
    status = host.dict_lookup_rules(
        arena, dict_h0, dict_h1, table, key, rules_cell, rule_count_cell, separator, lookup_mode as u32,
    );
    if status >= 0x8000_0000 {
        return status;
    }
    rule_count = half(arena, rule_count_cell);
    let mut rule_index = 0u16;
    while rule_index < rule_count && matches < 2 {
        let rules = word(arena, rules_cell);
        let expansion = word(arena, rules.wrapping_add((rule_index as u32) * 4));
        let mut replacement = 0u32;
        let mut domains = 0u32;
        let mut pattern = 0u32;
        let mut condition = 0u32;
        let mut priority_text = 0u32;
        let sep0 = byte(arena, separator);
        let mut p = strchr8(arena, expansion, sep0);
        if p != 0 {
            replacement = p.wrapping_add(1);
            put_byte(arena, p, 0);
            p = strchr8(arena, replacement, sep0);
            if p != 0 {
                put_byte(arena, p, 0);
                domains = p.wrapping_add(1);
                p = strchr8(arena, domains, sep0);
                if p != 0 {
                    put_byte(arena, p, 0);
                    pattern = p.wrapping_add(1);
                    p = strchr8(arena, pattern, sep0);
                    if p != 0 {
                        put_byte(arena, p, 0);
                        condition = p.wrapping_add(1);
                        p = strchr8(arena, condition, sep0);
                        if p != 0 {
                            priority_text = p.wrapping_add(1);
                            put_byte(arena, p, 0);
                            p = strchr8(arena, priority_text, sep0);
                            if p != 0 {
                                put_byte(arena, p, 0);
                            }
                        }
                    }
                }
            }
        }
        if expansion == 0 {
            return 0x89b0_2007;
        }
        if domains != 0 && !is_empty_str(arena, domains) {
            let found = strstr8(arena, domains, domain);
            let mut ok = false;
            if found != 0 {
                let domain_end = found.wrapping_add(strlen8(arena, domain));
                if (found == domains || byte(arena, found.wrapping_sub(1)) == b',')
                    && (byte(arena, domain_end) == 0 || byte(arena, domain_end) == b',')
                {
                    ok = true;
                }
            }
            if !ok {
                rule_index = rule_index.wrapping_add(1);
                continue;
            }
        }
        if condition == 0 {
            // accept (below)
        } else if streq(arena, condition, b"8\0") {
            rule_index = rule_index.wrapping_add(1);
            continue;
        } else {
            if streq(arena, condition, b"1\0")
                || streq(arena, condition, b"4\0")
                || streq(arena, condition, b"5\0")
                || streq(arena, condition, b"7\0")
            {
                let mut valid = 1;
                if pattern == 0 || is_empty_str(arena, pattern) {
                    let mut p = text;
                    while byte(arena, p) != 0 && valid != 0 {
                        let c = byte(arena, p);
                        if c.wrapping_sub(b'a') <= 25
                            || matches!(
                                c,
                                0x80 | 0x8e | 0x8f | 0x90 | 0x92 | 0x99 | 0x9a | 0xa5
                            )
                        {
                            valid = 0;
                        }
                        p = p.wrapping_add(1);
                    }
                } else {
                    let text_len = strlen8(arena, text);
                    let mut last = text.wrapping_add(text_len).wrapping_sub(1);
                    while host.dict_is_space(arena, dict_h0, dict_h1, byte(arena, last)) != 0 {
                        last = last.wrapping_sub(1);
                    }
                    let mut p = text;
                    while host.dict_is_space(arena, dict_h0, dict_h1, byte(arena, p)) != 0 {
                        p = p.wrapping_add(1);
                    }
                    let mut pat = pattern;
                    while host.dict_is_space(arena, dict_h0, dict_h1, byte(arena, pat)) != 0 {
                        pat = pat.wrapping_add(1);
                    }
                    while p <= last && valid != 0 {
                        let a = byte(arena, p);
                        let b = byte(arena, pat);
                        p = p.wrapping_add(1);
                        pat = pat.wrapping_add(1);
                        if a != b {
                            valid = 0;
                        }
                    }
                    while byte(arena, pat) != 0 && valid != 0 {
                        if host.dict_is_space(arena, dict_h0, dict_h1, byte(arena, pat)) == 0 {
                            valid = 0;
                        }
                        pat = pat.wrapping_add(1);
                    }
                }
                if valid == 0 {
                    rule_index = rule_index.wrapping_add(1);
                    continue;
                }
            }
            if streq(arena, condition, b"3\0")
                || streq(arena, condition, b"5\0")
                || streq(arena, condition, b"6\0")
                || streq(arena, condition, b"7\0")
            {
                let text_len = strlen8(arena, text);
                let mut i = text_len.wrapping_sub(1) as u16;
                while host.dict_is_space(
                    arena, dict_h0,
                    dict_h1,
                    byte(arena, text.wrapping_add(i as u32)),
                ) != 0
                {
                    i = i.wrapping_sub(1);
                }
                if (i as u32) != text_len.wrapping_sub(1) {
                    i = i.wrapping_add(1);
                }
                if pattern != 0 && !is_empty_str(arena, pattern) {
                    let pattern_len = strlen8(arena, pattern);
                    if byte(arena, pattern.wrapping_add(pattern_len).wrapping_sub(1))
                        != byte(arena, text.wrapping_add(i as u32))
                    {
                        rule_index = rule_index.wrapping_add(1);
                        continue;
                    }
                } else if byte(arena, text.wrapping_add(i as u32)) != b'.' {
                    rule_index = rule_index.wrapping_add(1);
                    continue;
                }
            }
            if streq(arena, condition, b"2\0")
                || streq(arena, condition, b"4\0")
                || streq(arena, condition, b"6\0")
                || streq(arena, condition, b"7\0")
            {
                // Context disambiguation; on success falls to accept.
                let mut accepted_flag = false;
                status = context_check(
                    arena, ctx, source, &mut start, &mut token_end,
                    expansion, replacement, domain, token_wide, first_token,
                    h0, h1, dict_h0, dict_h1, context_h0, context_h1,
                    context_name, record_cell, record_type_cell, record_text_cell,
                    record_start_cell, record_end_cell, output_cell, output_len_cell,
                    read_count_cell, status, &mut accepted_flag,
                    host,
                );
                if status >= 0x8000_0000 {
                    return status;
                }
                if !accepted_flag {
                    rule_index = rule_index.wrapping_add(1);
                    continue;
                }
            }
        }
        // accept:
        if priority_text != 0 && !is_empty_str(arena, priority_text) {
            let i = (atoi8(arena, priority_text) as u32) as u16;
            if i < priority {
                rule_index = rule_index.wrapping_add(1);
                continue;
            }
            if i > priority {
                matches = 0;
                priority = i;
            }
        }
        selected = expansion;
        matches = matches.wrapping_add(1);
        rule_index = rule_index.wrapping_add(1);
    }
    if matches == 1 {
        put_half(arena, matched_end, original_end);
        if strcmp8(arena, key, text) != 0 {
            while host.dict_is_space(
                arena, dict_h0,
                dict_h1,
                byte(arena, source.wrapping_add(start as u32)),
            ) != 0
            {
                start = start.wrapping_add(1);
            }
            token_end = token_end.wrapping_sub(1);
            while host.dict_is_space(
                arena, dict_h0,
                dict_h1,
                byte(arena, source.wrapping_add(token_end as u32)),
            ) != 0
            {
                token_end = token_end.wrapping_sub(1);
            }
            token_end = token_end.wrapping_add(1);
        }
        record_type = 4;
        status = host.token_create(arena, h0, h1, token as u32, record_cell);
        if status >= 0x8000_0000 {
            return status;
        }
        record = half(arena, record_cell);
        put_byte(arena, record_type_cell, record_type);
        status = host.token_set(
            arena, h0, h1, record as u32, 0, 1, record_type_cell, write_count_cell,
        );
        if status >= 0x8000_0000 {
            return status;
        }
        put_half(arena, start_cell, start);
        status = host.token_set(
            arena, h0, h1, record as u32, 1, 1, start_cell, write_count_cell,
        );
        if status >= 0x8000_0000 {
            return status;
        }
        put_half(arena, token_end_cell, token_end);
        status = host.token_set(
            arena, h0, h1, record as u32, 2, 1, token_end_cell, write_count_cell,
        );
        if status >= 0x8000_0000 {
            return status;
        }
        let selected_len = strlen8(arena, selected).wrapping_add(1) as u16;
        status = host.token_set(
            arena,
            h0,
            h1,
            record as u32,
            3,
            selected_len as u32,
            selected,
            write_count_cell,
        );
        if status >= 0x8000_0000 {
            return status;
        }
    }
    status
}

/// Context disambiguation for one `fn_283528` rule (condition 2/4/6/7 in
/// `src/fn_283528.c`): builds the input/expansion pair in reallocated
/// `ctx->input`/`ctx->expansion`, runs the `disambiguate_abbreviations`
/// context, and reports through `accepted` whether the rule is accepted.
/// Returns the C's `status` (an error status returns immediately).
#[allow(clippy::too_many_arguments)]
fn context_check(
    arena: &mut [u8],
    ctx: u32,
    source: u32,
    start: &mut u16,
    token_end: &mut u16,
    expansion: u32,
    replacement: u32,
    domain: u32,
    token_wide: u32,
    first_token: u16,
    h0: u32,
    h1: u32,
    dict_h0: u32,
    dict_h1: u32,
    context_h0: u32,
    context_h1: u32,
    context_name: u32,
    record_cell: u32,
    record_type_cell: u32,
    record_text_cell: u32,
    record_start_cell: u32,
    record_end_cell: u32,
    output_cell: u32,
    output_len_cell: u32,
    read_count_cell: u32,
    mut status: u32,
    accepted: &mut bool,
    host: &mut (impl NormalizationHost + TokenStoreHost + TokenDictHost + TokenContextHost),
) -> u32 {
    let mut left = *start;
    let mut right = *start;
    let length = strlen8(arena, source).wrapping_sub(1) as u16;
    let mut words = 0u8;
    let mut padding = 0u8;
    strcpy_static(arena, context_name, b"disambiguate_abbreviations\0");
    let context_name_length = strlen8(arena, context_name) as u16;
    while words < 4 && left > 0 {
        left = left.wrapping_sub(1);
        if is_tn_space(byte(arena, source.wrapping_add(left as u32)))
            && !is_tn_space(byte(arena, source.wrapping_add(left as u32).wrapping_add(1)))
        {
            words = words.wrapping_add(1);
        }
    }
    if left != 0 {
        left = left.wrapping_add(1);
    }
    let mut other_words = 0u8;
    while other_words < 4 && (right as u32) <= (length as u32) {
        right = right.wrapping_add(1);
        if is_tn_space(byte(arena, source.wrapping_add(right as u32)))
            && !is_tn_space(byte(arena, source.wrapping_add((right as u32).wrapping_sub(1))))
        {
            other_words = other_words.wrapping_add(1);
        }
    }
    let delta = (right as i32).wrapping_sub(left as i32);
    let heap = word(arena, word(arena, ctx).wrapping_add(4));
    let size = strlen8(arena, expansion)
        .wrapping_add(delta as u32)
        .wrapping_add(24);
    let input = match host.realloc(arena, heap, word(arena, ctx.wrapping_add(32)), size) {
        Some(p) => p,
        None => return 0x89b0_200a,
    };
    put_word(arena, ctx.wrapping_add(32), input);
    put_byte(arena, input, 0);
    let expansion_buf = match host.realloc(arena, heap, word(arena, ctx.wrapping_add(36)), size) {
        Some(p) => p,
        None => return 0x89b0_200a,
    };
    put_word(arena, ctx.wrapping_add(36), expansion_buf);
    put_byte(arena, expansion_buf, 0);
    let mut result = host.token_find(arena, h0, h1, 3, first_token as u32, record_cell);
    if result >= 0x8000_0000 {
        return result;
    }
    let mut record = half(arena, record_cell);
    while record != 0 {
        if record as u32 == token_wide {
            while host.dict_is_space(
                arena, dict_h0,
                dict_h1,
                byte(arena, source.wrapping_add(*start as u32)),
            ) != 0
            {
                *start = start.wrapping_add(1);
            }
            *token_end = token_end.wrapping_sub(1);
            while host.dict_is_space(
                arena, dict_h0,
                dict_h1,
                byte(arena, source.wrapping_add(*token_end as u32)),
            ) != 0
            {
                *token_end = token_end.wrapping_sub(1);
            }
            *token_end = token_end.wrapping_add(1);
            strnappend(
                arena,
                input,
                source.wrapping_add(left as u32),
                (*start as i32 - left as i32) as u32,
            );
            let mut i = 0u16;
            while (i as i32) < (*start as i32 - left as i32) {
                strcat_char(arena, expansion_buf, b' ');
                i = i.wrapping_add(1);
            }
            strcat_char(arena, input, 0x15);
            strcat_at(arena, input, expansion);
            let replacement_len = strlen8(arena, replacement);
            let expansion_len = strlen8(arena, expansion);
            if replacement_len > expansion_len {
                let mut i = 0u16;
                while (i as u32) < replacement_len.wrapping_sub(expansion_len) && padding < 20 {
                    strcat_char(arena, input, b'~');
                    padding = padding.wrapping_add(1);
                    i = i.wrapping_add(1);
                }
            }
            strcat_char(arena, input, 0x15);
            strcat_char(arena, expansion_buf, 0x15);
            strcat_at(arena, expansion_buf, replacement);
            if expansion_len > replacement_len {
                let mut i = 0u16;
                while (i as u32) < expansion_len.wrapping_sub(replacement_len) {
                    strcat_char(arena, expansion_buf, b'~');
                    i = i.wrapping_add(1);
                }
            }
            strcat_char(arena, expansion_buf, 0x15);
            left = *token_end;
        } else {
            result = host.token_get(
                arena, h0, h1, record as u32, 0, 1, record_type_cell, read_count_cell,
            );
            if result >= 0x8000_0000 {
                return result;
            }
            if byte(arena, record_type_cell) == 6 {
                result = host.token_get(
                    arena, h0, h1, record as u32, 1, 1, record_start_cell, read_count_cell,
                );
                if result >= 0x8000_0000 {
                    return result;
                }
                result = host.token_get(
                    arena, h0, h1, record as u32, 2, 1, record_end_cell, read_count_cell,
                );
                // Target checks the fresh get result (fn_283528+0xd88: cmp
                // r0 / bcs return); src/fn_283528.c:341 reads stale `status`
                // instead (claimed C defect, slice 2b).
                if result >= 0x8000_0000 {
                    return result;
                }
                status = host.token_get_string(
                    arena, h0, h1, record as u32, 4, record_text_cell, read_count_cell,
                );
                if status >= 0x8000_0000 {
                    return status;
                }
                let mut record_start = half(arena, record_start_cell);
                let record_end = half(arena, record_end_cell);
                let record_text = word(arena, record_text_cell);
                if (left < record_end && record_start < *start && record_end <= *start)
                    || (record_start < right
                        && *token_end <= record_start
                        && *token_end < record_end)
                {
                    let mut span_end = record_end;
                    if span_end > right {
                        span_end = right;
                    }
                    strnappend(
                        arena,
                        input,
                        source.wrapping_add(left as u32),
                        (span_end as i32 - left as i32) as u32,
                    );
                    if left < record_start {
                        let mut i = 0u16;
                        while (i as i32) < (record_start as i32 - left as i32) {
                            strcat_char(arena, expansion_buf, b' ');
                            i = i.wrapping_add(1);
                        }
                    } else {
                        record_start = left;
                    }
                    let record_len = strlen8(arena, record_text);
                    strnappend(
                        arena,
                        expansion_buf,
                        record_text.wrapping_add(1),
                        record_len.wrapping_sub(2),
                    );
                    if (span_end.wrapping_sub(record_start) as i16)
                        < (record_len.wrapping_sub(2) as u16 as i16)
                    {
                        let mut i = 0u16;
                        while (i as u32)
                            < record_len
                                .wrapping_sub(span_end as u32)
                                .wrapping_add(record_start as u32)
                                .wrapping_sub(2)
                            && padding < 20
                        {
                            strcat_char(arena, input, b'~');
                            padding = padding.wrapping_add(1);
                            i = i.wrapping_add(1);
                        }
                        if padding >= 20 {
                            break;
                        }
                    } else {
                        let mut i = 0u16;
                        let bound = ((span_end as i32 - record_start as i32) as u32)
                            .wrapping_sub(record_len)
                            .wrapping_add(2);
                        while (i as u32) < bound {
                            strcat_char(arena, expansion_buf, b'~');
                            i = i.wrapping_add(1);
                        }
                    }
                    left = span_end;
                }
            }
        }
        status = host.token_next(arena, h0, h1, record as u32, record_cell);
        if status >= 0x8000_0000 {
            return status;
        }
        record = half(arena, record_cell);
    }
    if left < right && padding < 20 {
        strnappend(
            arena,
            input,
            source.wrapping_add(left as u32),
            (right as i32 - left as i32) as u32,
        );
        let mut i = 0u16;
        while (i as i32) < (right as i32 - left as i32) {
            strcat_char(arena, expansion_buf, b' ');
            i = i.wrapping_add(1);
        }
    }
    if padding < 20 {
        result = host.context_set_text(
            arena,
            context_h0,
            context_h1,
            0,
            input,
            strlen8(arena, input) & 0xffff,
        );
        if result >= 0x8000_0000 {
            return result;
        }
        result = host.context_set_text(
            arena,
            context_h0,
            context_h1,
            1,
            expansion_buf,
            strlen8(arena, expansion_buf) & 0xffff,
        );
        if result >= 0x8000_0000 {
            return result;
        }
        if !streq(arena, domain, b"normal\0")
            && strlen8(arena, domain)
                .wrapping_add(context_name_length as u32)
                .wrapping_add(2)
                < 128
        {
            strcat_char(arena, context_name, b'_');
            strcat_at(arena, context_name, domain);
        }
        result = host.context_run(arena, context_h0, context_h1, context_name);
        if result >= 0x8000_0000 {
            put_byte(
                arena,
                context_name.wrapping_add(context_name_length as u32),
                0,
            );
            result = host.context_run(arena, context_h0, context_h1, context_name);
            // Target checks the fresh run result (fn_283528+0x114c: cmp
            // r0 / bcs return); src/fn_283528.c:400 reads stale `status`
            // instead (claimed C defect, slice 2b).
            if result >= 0x8000_0000 {
                return result;
            }
        }
        status = host.context_get_text(
            arena, context_h0, context_h1, 0, output_cell, output_len_cell,
        );
        if status >= 0x8000_0000 {
            return status;
        }
        let output = word(arena, output_cell);
        put_byte(
            arena,
            output.wrapping_add(half(arena, output_len_cell) as u32),
            0,
        );
        let mut valid = 0;
        let mut p = strchr8(arena, output, 0x15);
        while p != 0 && valid == 0 {
            let q = strchr8(arena, p.wrapping_add(1), 0x15);
            if q != 0 && byte(arena, p.wrapping_add(1)) != b' ' {
                valid = 1;
            }
            p = q;
        }
        if valid == 0 {
            *accepted = true;
            return status;
        }
    }
    *accepted = false;
    status
}

/// fn_284bc0, EU fn_284d7c, KR fn_284e28. Tokenizes one span: walks the
/// token chain from `*token_ptr`, collects dictionary word counts for each
/// word start, and recursively normalizes through `match_rules` with the
/// `"tn"` table. `position`/`end` arrive full-width like the ARM and are
/// narrowed to the C's u16 locals. `scratch` is a `TOKENIZE_FRAME`-byte
/// region; `match_scratch` a `MATCH_FRAME`-byte region for the recursive
/// calls.
#[allow(clippy::too_many_arguments)]
pub fn tokenize_span(
    arena: &mut [u8],
    ctx: u32,
    source: u32,
    position: u32,
    end: u32,
    domain: u32,
    h0: u32,
    h1: u32,
    token_ptr: u32,
    scratch: u32,
    match_scratch: u32,
    host: &mut (impl NormalizationHost + TokenStoreHost + TokenDictHost + TokenContextHost),
) -> u32 {
    let word_buf = scratch.wrapping_add(TOKENIZE_WORD);
    let lookup_text = scratch.wrapping_add(TOKENIZE_LOOKUP);
    let phrase = scratch.wrapping_add(TOKENIZE_PHRASE);
    let norm_word = scratch.wrapping_add(TOKENIZE_NORMWORD);
    let folded = scratch.wrapping_add(TOKENIZE_FOLDED);
    let token_text = scratch.wrapping_add(TOKENIZE_TOKENTEXT);
    let counts = scratch.wrapping_add(TOKENIZE_COUNTS);
    let read_count_cell = scratch.wrapping_add(TOKENIZE_OUTS);
    let next_token_cell = scratch.wrapping_add(TOKENIZE_OUTS + 2);
    let kind_cell = scratch.wrapping_add(TOKENIZE_OUTS + 4);
    let next_kind_cell = scratch.wrapping_add(TOKENIZE_OUTS + 5);
    let token_end_cell = scratch.wrapping_add(TOKENIZE_OUTS + 6);
    let matched_end_cell = scratch.wrapping_add(TOKENIZE_OUTS + 8);
    let entries_cell = scratch.wrapping_add(TOKENIZE_ENTRIES);
    let entry_count_cell = scratch.wrapping_add(TOKENIZE_ENTRY_COUNT);
    let position_cell = scratch.wrapping_add(TOKENIZE_POSITION);
    let kind_str = scratch.wrapping_add(TOKENIZE_KIND);
    let table_str = scratch.wrapping_add(TOKENIZE_TABLE);
    let dict_h0 = word(arena, ctx.wrapping_add(8));
    let dict_h1 = word(arena, ctx.wrapping_add(12));
    strcpy_static(arena, kind_str, b"femwtn\0");
    strcpy_static(arena, table_str, b"tn\0");
    // The C holds `position`/`end` as narrowed u16 locals; the differential
    // fuzz passes full-width values and any ARM-raw site is adjusted when
    // its C-vs-ARM defects identify it.
    let end = (end & 0xffff) as u16;
    put_half(arena, read_count_cell, 0);
    put_half(arena, next_token_cell, 0);
    put_byte(arena, kind_cell, 0);
    put_byte(arena, next_kind_cell, 3);
    put_half(arena, token_end_cell, 0);
    let mut position = (position & 0xffff) as u16;
    put_half(arena, matched_end_cell, position.wrapping_sub(1));
    let mut previous_kind = 3u32;
    let mut word_count = 0u8;
    put_byte(arena, word_buf, 0);
    strcpy_at(arena, lookup_text, word_buf);
    put_byte(arena, token_text, 0);
    put_byte(arena, norm_word, 0);
    put_byte(arena, phrase, 0);
    put_byte(arena, folded, 0);
    let mut token = half(arena, token_ptr);
    let mut status = host.token_get(
        arena, h0, h1, token as u32, 0, 1, kind_cell, read_count_cell,
    );
    if status >= 0x8000_0000 {
        return status;
    }
    while byte(arena, kind_cell) == 5 {
        status = host.token_next(arena, h0, h1, token as u32, token_ptr);
        if status >= 0x8000_0000 {
            return status;
        }
        token = half(arena, token_ptr);
        if token == 0 {
            return 0x89b0_2000;
        }
        status = host.token_get(
            arena, h0, h1, token as u32, 0, 1, kind_cell, read_count_cell,
        );
        if status >= 0x8000_0000 {
            return status;
        }
    }
    while token != 0 && (end as i16) > (half(arena, token_end_cell) as i16) {
        status = host.token_next(arena, h0, h1, token as u32, next_token_cell);
        if status >= 0x8000_0000 {
            return status;
        }
        let mut next = half(arena, next_token_cell);
        if next != 0 {
            status = host.token_get(
                arena, h0, h1, next as u32, 0, 1, next_kind_cell, read_count_cell,
            );
            if status >= 0x8000_0000 {
                return status;
            }
            while byte(arena, next_kind_cell) == 5 {
                status = host.token_next(arena, h0, h1, next as u32, next_token_cell);
                if status >= 0x8000_0000 {
                    return status;
                }
                next = half(arena, next_token_cell);
                if next == 0 {
                    break;
                }
                status = host.token_get(
                    arena, h0, h1, next as u32, 0, 1, next_kind_cell, read_count_cell,
                );
                if status >= 0x8000_0000 {
                    return status;
                }
            }
        }
        if next == 0 {
            put_byte(arena, next_kind_cell, 3);
        }
        status = host.token_get(
            arena, h0, h1, token as u32, 1, 1, position_cell, read_count_cell,
        );
        if status >= 0x8000_0000 {
            return status;
        }
        position = half(arena, position_cell);
        if (position as i16) > (half(arena, matched_end_cell) as i16) {
            let kind = byte(arena, kind_cell);
            if kind != 3 && previous_kind == 3 {
                // Split the source word at the first blank boundary.
                let mut index = position;
                let mut i = 0u16;
                let word_ready = loop {
                    let c = byte(arena, source.wrapping_add(index as u32));
                    if c == b' ' || c == b'\t' || c == b'\n' || c == b'\r' {
                        put_byte(arena, word_buf.wrapping_add(i as u32), 0);
                        break true;
                    }
                    if c == b'_' || c == 0 {
                        put_byte(arena, word_buf.wrapping_add(i as u32), 0);
                        break true;
                    }
                    index = index.wrapping_add(1);
                    put_byte(arena, word_buf.wrapping_add(i as u32), c);
                    i = i.wrapping_add(1);
                    if i == 128 {
                        break false;
                    }
                };
                if word_ready {
                    strcpy_at(arena, lookup_text, word_buf);
                    put_half(arena, entry_count_cell, 0);
                    put_word(arena, entries_cell, 0);
                    memset_at(arena, counts, 0, 64);
                    status = host.dict_lookup(
                        arena, dict_h0, dict_h1, lookup_text, kind_str, domain,
                        entries_cell, entry_count_cell,
                    );
                    if status < 0x8000_0000 {
                        word_count = 0;
                        let entries = word(arena, entries_cell);
                        let entry_count = half(arena, entry_count_cell);
                        let mut i = 0u16;
                        while (i as u32) < entry_count as u32 {
                            let entry =
                                word(arena, entries.wrapping_add((i as u32) * 4));
                            let value = parse_counted(arena, entry) as u32;
                            if value < 64 {
                                put_byte(
                                    arena,
                                    counts.wrapping_add(word_count as u32),
                                    value as u8,
                                );
                                word_count = word_count.wrapping_add(1);
                            }
                            i = i.wrapping_add(1);
                        }
                        // The C comparator returns 0/1, never negative.
                        quicksort_by(arena, counts, word_count as u32, 1, |a, b| {
                            (a < b) as i32
                        });
                    }
                    if status >= 0x8000_0000 {
                        return status;
                    }
                    let mut candidate = 0u8;
                    while candidate < word_count
                        && (position as i16) > (half(arena, matched_end_cell) as i16)
                    {
                        let expected = byte(arena, counts.wrapping_add(candidate as u32));
                        index = position;
                        let mut length = 0u16;
                        let mut count = 0u8;
                        let mut hit_limit = false;
                        while count < expected
                            && byte(arena, source.wrapping_add(index as u32)) != 0
                        {
                            put_byte(
                                arena,
                                lookup_text.wrapping_add(length as u32),
                                byte(arena, source.wrapping_add(index as u32)),
                            );
                            index = index.wrapping_add(1);
                            length = length.wrapping_add(1);
                            if length == 256 {
                                hit_limit = true;
                                break;
                            }
                            if is_tn_space(byte(arena, source.wrapping_add(index as u32)))
                                && !is_tn_space(byte(
                                    arena,
                                    source.wrapping_add(index as u32).wrapping_sub(1),
                                ))
                            {
                                count = count.wrapping_add(1);
                            }
                        }
                        if !hit_limit {
                            put_byte(arena, lookup_text.wrapping_add(length as u32), 0);
                            if byte(arena, source.wrapping_add(index as u32)) == 0
                                && !is_tn_space(byte(
                                    arena,
                                    source.wrapping_add(index as u32).wrapping_sub(1),
                                ))
                            {
                                count = count.wrapping_add(1);
                            }
                            if count == expected {
                                strcpy_at(arena, phrase, lookup_text);
                                status = host.dict_normalize(arena, dict_h0, dict_h1, domain, phrase);
                                if status >= 0x8000_0000 {
                                    return status;
                                }
                                status = match_rules(
                                    arena, ctx, phrase, lookup_text, table_str, domain,
                                    source, position as u32, token as u32, h0, h1,
                                    matched_end_cell, match_scratch, host,
                                );
                                if status >= 0x8000_0000 {
                                    return status;
                                }
                            }
                        }
                        candidate = candidate.wrapping_add(1);
                    }
                    if (position as i16) > (half(arena, matched_end_cell) as i16) {
                        status = match_rules(
                            arena, ctx, word_buf, word_buf, table_str, domain,
                            source, position as u32, token as u32, h0, h1,
                            matched_end_cell, match_scratch, host,
                        );
                        if status >= 0x8000_0000 {
                            return status;
                        }
                    }
                    if (position as i16) > (half(arena, matched_end_cell) as i16) {
                        strcpy_at(arena, norm_word, word_buf);
                        status =
                            host.dict_normalize(arena, dict_h0, dict_h1, domain, norm_word);
                        if status >= 0x8000_0000 {
                            return status;
                        }
                        if strcmp8(arena, word_buf, norm_word) != 0 {
                            status = match_rules(
                                arena, ctx, norm_word, word_buf, table_str, domain,
                                source, position as u32, token as u32, h0, h1,
                                matched_end_cell, match_scratch, host,
                            );
                            if status >= 0x8000_0000 {
                                return status;
                            }
                        }
                    }
                }
            }
            let kind = byte(arena, kind_cell);
            if (position as i16) > (half(arena, matched_end_cell) as i16)
                && kind == 0
                && (previous_kind != 3 || byte(arena, next_kind_cell) != 3)
            {
                status = host.token_get(
                    arena, h0, h1, token as u32, 2, 1, token_end_cell, read_count_cell,
                );
                if status >= 0x8000_0000 {
                    return status;
                }
                let token_end = half(arena, token_end_cell);
                let delta = (token_end as i32).wrapping_sub(position as i32);
                if delta < 128 {
                    strncpy_at(
                        arena,
                        token_text,
                        source.wrapping_add(position as u32),
                        delta as u32,
                    );
                    put_byte(arena, token_text.wrapping_add(delta as u32), 0);
                    strcpy_at(arena, folded, token_text);
                    status = host.dict_fold(arena, dict_h0, dict_h1, domain, folded);
                    if status >= 0x8000_0000 {
                        return status;
                    }
                    if strcmp8(arena, norm_word, folded) != 0 {
                        status = match_rules(
                            arena, ctx, folded, token_text, table_str, domain,
                            source, position as u32, token as u32, h0, h1,
                            matched_end_cell, match_scratch, host,
                        );
                        if status >= 0x8000_0000 {
                            return status;
                        }
                    }
                }
            }
        }
        status = host.token_get(
            arena, h0, h1, token as u32, 2, 1, token_end_cell, read_count_cell,
        );
        if status >= 0x8000_0000 {
            return status;
        }
        put_half(arena, token_ptr, next);
        token = next;
        previous_kind = byte(arena, kind_cell) as u32;
        put_byte(arena, kind_cell, byte(arena, next_kind_cell));
    }
    status
}

/// Host operations for the TOKENTN Process entry (`fn_1e4b7c`): the
/// depth-1 callees are summarized from the recording, while the entry's
/// own control flow, sentence scan, and scratch/work lifecycle run in
/// the port. Arena offsets with 0 meaning null.
pub trait TokentnHost: NormalizationHost + HandleValidator {
    /// `fn_173fd8`: log verbosity for `log`.
    fn log_level(&mut self, arena: &mut [u8], log: u32) -> u32;
    /// `fn_24c9fc`: read the sentence into `p+0x5c`. Returns the status.
    fn read_sentence(&mut self, arena: &mut [u8], p: u32) -> u32;
    /// `fn_24cea0`: allocate destination buffer `buf` for process `p`.
    fn alloc_dest(&mut self, arena: &mut [u8], p: u32, buf: u32) -> u32;
    /// `fn_24cdac`: allocate the two tables.
    fn alloc_tables(&mut self, arena: &mut [u8], p: u32) -> u32;
    /// `fn_24cc90`: allocate the second record.
    fn alloc_rec2(&mut self, arena: &mut [u8], p: u32) -> u32;
    /// `fn_24cb74`: allocate the first record.
    fn alloc_rec(&mut self, arena: &mut [u8], p: u32) -> u32;
    /// `fn_249cec`: run the token rules over the prepared buffers.
    fn run_rules(&mut self, arena: &mut [u8], p: u32) -> u32;
    /// `fn_24ac00`: end-of-processing log pass over `name` with `mode`.
    fn log_end(&mut self, arena: &mut [u8], p: u32, name: u32, mode: u32) -> u32;
    /// `fn_24d030`: release buffer `buf` owned by context `ctx`.
    fn release_buf(&mut self, arena: &mut [u8], ctx: u32, buf: u32);
    /// `fn_24ce40`: release the owned pair.
    fn release_pair(&mut self, arena: &mut [u8], p: u32);
    /// `fn_24ccf0`: release the second record.
    fn release2(&mut self, arena: &mut [u8], p: u32);
    /// `fn_24cbd4`: release the first record.
    fn release1(&mut self, arena: &mut [u8], p: u32);
}

/// Buffer field offsets within `TokenProcess`, in `prepare`/`cleanup`
/// order: first, second, third, fifth, sixth, seventh, fourth.
const TOKENTN_BUFS: [u32; 7] = [0x88, 0x94, 0xa0, 0xcc, 0xd8, 0xe4, 0xac];

fn tokentn_prepare(
    arena: &mut [u8],
    p: u32,
    host: &mut impl TokentnHost,
) -> u32 {
    for buf in TOKENTN_BUFS {
        let result = host.alloc_dest(arena, p, p.wrapping_add(buf));
        if result >= 0x8000_0000 {
            return result;
        }
    }
    let result = host.alloc_tables(arena, p);
    if result >= 0x8000_0000 {
        return result;
    }
    let result = host.alloc_rec2(arena, p);
    if result >= 0x8000_0000 {
        return result;
    }
    let result = host.alloc_rec(arena, p);
    if result >= 0x8000_0000 {
        return result;
    }
    let heap = word(arena, word(arena, p).wrapping_add(4));
    let work20 = host.calloc(arena, heap, 1, 20).unwrap_or(0);
    put_word(arena, p.wrapping_add(0x64), work20);
    if work20 == 0 {
        return error_code(10);
    }
    let heap = word(arena, word(arena, p).wrapping_add(4));
    let work24 = host.calloc(arena, heap, 1, 24).unwrap_or(0);
    put_word(arena, p.wrapping_add(0x68), work24);
    if work24 == 0 {
        return error_code(10);
    }
    0
}

fn tokentn_cleanup(
    arena: &mut [u8],
    p: u32,
    host: &mut impl TokentnHost,
) {
    let ctx = word(arena, p);
    let heap = word(arena, ctx.wrapping_add(4));
    let scratch = word(arena, p.wrapping_add(0xc8));
    if scratch != 0 {
        host.free(arena, heap, scratch);
        put_word(arena, p.wrapping_add(0xc8), 0);
    }
    for buf in TOKENTN_BUFS {
        host.release_buf(arena, ctx, p.wrapping_add(buf));
    }
    host.release_pair(arena, p);
    host.release2(arena, p);
    host.release1(arena, p);
    let ctx = word(arena, p);
    let heap = word(arena, ctx.wrapping_add(4));
    let work20 = word(arena, p.wrapping_add(0x64));
    if work20 != 0 {
        host.free(arena, heap, work20);
        put_word(arena, p.wrapping_add(0x64), 0);
    }
    let work24 = word(arena, p.wrapping_add(0x68));
    if work24 != 0 {
        host.free(arena, heap, work24);
        put_word(arena, p.wrapping_add(0x68), 0);
    }
}

/// `fn_1e4b7c`, EU `fn_1e4b7c`, KR `fn_1e4de4` (TOKENTN Process):
/// validate both handles, read the sentence, count its words, prepare
/// buffers and tables, run the rules, and always clean up.
/// `fn_1738e8`/`fn_173dc0` calls are log-only. TokenProcess layout per
/// `src/fn_1e4b7c.c`: context@0 arg0@0x10 arg1@0x14 module@0x48
/// sentence@0x5c sentenceLength@0x60 wordCount@0x62 work20@0x64
/// work24@0x68 first@0x88 second@0x94 third@0xa0 fourth@0xac
/// detailedLog@0xc6 scratch@0xc8 fifth@0xcc sixth@0xd8 seventh@0xe4
/// f0@0xf0. TokenBuffer stride is 0xc (data@0 capacity@4 length@6
/// limit@8). The `fn_24ac00` name literal is staged in scratch at
/// `p+0x100`, past the 0xf4 struct. Note the `*processed` gate: only
/// the scratch/prepare/rules failures skip it; a `fn_24ac00` failure
/// still sets it.
pub fn tokentn_process_1e4b7c(
    arena: &mut [u8],
    handle: u32,
    check: u32,
    arg0: u32,
    arg1: u32,
    processed: u32,
    host: &mut impl TokentnHost,
) -> u32 {
    if host.validate(check, 0xe58c, 8) >= 0x8000_0000 {
        return error_code(8);
    }
    let p = word(arena, handle);
    let p_check = word(arena, handle.wrapping_add(4));
    if host.validate(p_check, 0xe58d, 0xf4) >= 0x8000_0000 {
        return error_code(8);
    }
    let ctx = word(arena, p);
    let log = word(arena, ctx.wrapping_add(0x10));
    put_word(arena, p.wrapping_add(0x14), arg1);
    put_word(arena, p.wrapping_add(0x10), arg0);
    let detailed = u8::from(host.log_level(arena, log) >= 5);
    put_byte(arena, p.wrapping_add(0xc6), detailed);
    let mut result = host.read_sentence(arena, p);
    if result >= 0x8000_0000 {
        return result;
    }
    let mut set_processed = true;
    if word(arena, p.wrapping_add(0x5c)) != 0 {
        let sentence = word(arena, p.wrapping_add(0x5c));
        let len = strlen8(arena, sentence) as u16;
        put_half(arena, p.wrapping_add(0x60), len);
        let heap = word(arena, word(arena, p).wrapping_add(4));
        let scratch = host
            .calloc(arena, heap, 1, u32::from(len).wrapping_add(1))
            .unwrap_or(0);
        put_word(arena, p.wrapping_add(0xc8), scratch);
        if scratch == 0 {
            result = error_code(10);
            set_processed = false;
        } else {
            put_half(arena, p.wrapping_add(0x62), 1);
            let mut i = 0u16;
            while i < len && byte(arena, sentence.wrapping_add(u32::from(i))) == b' ' {
                i = i.wrapping_add(1);
            }
            while i < len {
                if byte(arena, sentence.wrapping_add(u32::from(i))) == b' ' {
                    let words = half(arena, p.wrapping_add(0x62));
                    put_half(arena, p.wrapping_add(0x62), words.wrapping_add(1));
                    i = i.wrapping_add(1);
                    while i < len && byte(arena, sentence.wrapping_add(u32::from(i))) == b' ' {
                        i = i.wrapping_add(1);
                    }
                } else {
                    i = i.wrapping_add(1);
                }
            }
            result = tokentn_prepare(arena, p, host);
            if result >= 0x8000_0000 {
                set_processed = false;
            } else {
                result = host.run_rules(arena, p);
                if result >= 0x8000_0000 {
                    set_processed = false;
                } else if byte(arena, p.wrapping_add(0xc6)) == 1 {
                    let base = p.wrapping_add(0x100);
                    for (j, b) in b"At End of Processing\0".iter().enumerate() {
                        put_byte(arena, base.wrapping_add(j as u32), *b);
                    }
                    result = host.log_end(arena, p, base, 1);
                }
            }
        }
    }
    if set_processed {
        put_byte(arena, processed, 1);
    }
    tokentn_cleanup(arena, p, host);
    result
}
