//! live fe lookup-domain and tokenize entries (1ea4e0, 26bb6c, 1ede44) over the caller's arena, for the 1f98c8 root.
//!
//! Every guest pointer stays a scalar address inside the caller's arena and
//! every guest call is issued through the caller's host, so the ports leave
//! the arena exactly as the ROM leaves it.
//!
//! The offsets come from the ROM objects (`build/target/fn_*.o`) and match
//! the C units: `record_size`/`offset_size`/`offset_bytes` at header +0x34,
//! +0x36, +0x44 (`ldr` +0x34/+0x36/+0x44 in `fn_26bb6c.o`), the separator
//! the `fn_26bb6c` expansion breaks on at +0x08 (`ldrb` +0x08) and the one
//! `fn_26bb54` reports at +0x09 (`src/fn_26bb54.c`, the ARM's 0x16 byte).
//!
//! `lookup_entry` (and through it `fn_26bb6c_m`) calls lane b's
//! `component_live_b::fn_26c1c0_m` (exact ARM call
//! `fn_26c1c0(reader, text, start, end, fold)`); this file holds no trie
//! copy. Lane b round 8 carried an extra `fn_26b37c` read before the char
//! walk that missed this capture (1/4 archive reads); lane b has since
//! removed it and certified the port, so the binding is back.

#![allow(dead_code)]
#![allow(non_upper_case_globals)]

use crate::{boot::Memory, component_live_b::Host};

/// The `DictionaryState` (the ROM's `fn_26c1c0` reader) field offsets.
const ST_POS: u32 = 0x00;
const ST_BIT: u32 = 0x04;
const ST_BUFFER: u32 = 0x08;
const ST_TABLES: u32 = 0x0c;
const ST_H0: u32 = 0x10;
const ST_H1: u32 = 0x14;
const ST_OUTPUT: u32 = 0x18;

/// The `DictionaryTables` field offsets.
const TB_HEADER: u32 = 0x00;
const TB_ALPHABET: u32 = 0x04;
const TB_PAIRS: u32 = 0x08;
const TB_INITIAL_CHARS: u32 = 0x0c;
const TB_INITIAL_OFFSETS: u32 = 0x10;
const TB_INITIAL_INDICES: u32 = 0x14;
const TB_COUNT_TREE: u32 = 0x18;
const TB_CHAR_TREE: u32 = 0x1c;
const TB_DELTA_TREE: u32 = 0x20;
const TB_SKIPPED_TREE: u32 = 0x24;
const TB_OFFSET_TREE: u32 = 0x28;
const TB_COUNTS: u32 = 0x2c;
const TB_CHARS: u32 = 0x30;
const TB_DELTAS: u32 = 0x34;
const TB_SKIPPED_VALUES: u32 = 0x38;
const TB_OFFSET_VALUES: u32 = 0x3c;
const TB_DATA_OFFSET: u32 = 0x40;
const TB_RECORDS: u32 = 0x44;
const TB_OFFSETS: u32 = 0x48;

/// The `DictionaryHeader` field offsets.
const HD_FIRST_CHAR: u32 = 0x04;
const HD_INPUT_MIN: u32 = 0x04;
const HD_INPUT_MAX: u32 = 0x05;
const HD_PAIR_BASE: u32 = 0x06;
const HD_SEPARATOR: u32 = 0x08;
const HD_PREFIX_MIN: u32 = 0x0a;
const HD_PREFIX_MAX: u32 = 0x0b;
const HD_WIDTH: u32 = 0x0c;
const HD_BITS: u32 = 0x0d;
const HD_SHIFT: u32 = 0x0e;
const HD_ALIGN_MASK: u32 = 0x0f;
const HD_TERMINATOR: u32 = 0x10;
const HD_INITIAL_BIT: u32 = 0x11;
const HD_INLINE_LIMIT: u32 = 0x12;
const HD_OFFSET_BITS: u32 = 0x13;
const HD_MAX_LENGTH: u32 = 0x16;
const HD_DIRECT_COUNT: u32 = 0x18;
const HD_COUNT_ROOT: u32 = 0x1a;
const HD_CHAR_ROOT: u32 = 0x1c;
const HD_DELTA_ROOT: u32 = 0x1e;
const HD_SKIPPED_ROOT: u32 = 0x20;
const HD_OFFSET_ROOT: u32 = 0x22;
const HD_INITIAL_COUNT: u32 = 0x2e;
const HD_PAGE_SIZE: u32 = 0x32;
const HD_RECORD_SIZE: u32 = 0x34;
const HD_OFFSET_SIZE: u32 = 0x36;
const HD_INITIAL_POS: u32 = 0x38;
const HD_OFFSET_BYTES: u32 = 0x44;

/// Read a guest word. `None` when the address is outside the caller's arena.
fn g_word(m: &Memory<'_>, p: u32) -> Option<u32> {
    let at = p.checked_sub(m.base)? as usize;
    let b = m.bytes.get(at..at.checked_add(4)?)?;
    Some(u32::from_le_bytes(b.try_into().ok()?))
}

/// Read a guest half word. `None` when the address is outside the arena.
fn g_half(m: &Memory<'_>, p: u32) -> Option<u16> {
    let at = p.checked_sub(m.base)? as usize;
    let b = m.bytes.get(at..at.checked_add(2)?)?;
    Some(u16::from_le_bytes(b.try_into().ok()?))
}

/// Read a guest byte. `None` when the address is outside the arena.
fn g_byte(m: &Memory<'_>, p: u32) -> Option<u8> {
    let at = p.checked_sub(m.base)? as usize;
    Some(*m.bytes.get(at)?)
}

/// True when `n` bytes at `p` are inside the caller's arena.
fn g_range(m: &Memory<'_>, p: u32, n: u32) -> bool {
    let at = match p.checked_sub(m.base) {
        Some(v) => v as usize,
        None => return false,
    };
    match at.checked_add(n as usize) {
        Some(end) => end <= m.bytes.len(),
        None => false,
    }
}

/// Write a guest word.
fn p_word(m: &mut Memory<'_>, p: u32, v: u32) -> bool {
    if !g_range(m, p, 4) {
        return false;
    }
    let at = (p - m.base) as usize;
    m.bytes[at..at + 4].copy_from_slice(&v.to_le_bytes());
    true
}

/// Write a guest half word.
fn p_half(m: &mut Memory<'_>, p: u32, v: u16) -> bool {
    if !g_range(m, p, 2) {
        return false;
    }
    let at = (p - m.base) as usize;
    m.bytes[at..at + 2].copy_from_slice(&v.to_le_bytes());
    true
}

/// Write a guest byte.
fn p_byte(m: &mut Memory<'_>, p: u32, v: u8) -> bool {
    if !g_range(m, p, 1) {
        return false;
    }
    let at = (p - m.base) as usize;
    m.bytes[at] = v;
    true
}

/// The length of the guest string at `p`, terminator excluded.
fn g_strlen(m: &Memory<'_>, p: u32) -> Option<u32> {
    let mut i = 0u32;
    loop {
        match g_byte(m, p.checked_add(i)?) {
            Some(0) => return Some(i),
            Some(_) => i = i.checked_add(1)?,
            None => return None,
        }
    }
}

/// Copy the guest string `src` to `dst`, terminator included.
fn g_strcpy(m: &mut Memory<'_>, dst: u32, src: u32) -> bool {
    let len = match g_strlen(m, src) {
        Some(v) => v,
        None => return false,
    };
    if !g_range(m, dst, len + 1) || !g_range(m, src, len + 1) {
        return false;
    }
    let s = (src - m.base) as usize;
    let d = (dst - m.base) as usize;
    m.bytes.copy_within(s..s + len as usize, d);
    let d = (dst - m.base) as usize;
    m.bytes[d + len as usize] = 0;
    true
}

/// Case-insensitive guest strcmp for the dictionary index search. The
/// C compares with `fn_195d58` and the ARM matches `ACRON1` against
/// `acron1` via `fn_1769d0`; a byte compare misses those entries.
fn g_strcmp_ci(m: &Memory<'_>, a: u32, b: u32) -> i32 {
    fn fold(c: u8) -> u8 {
        if c >= b'a' && c <= b'z' {
            c.wrapping_sub(b'a').wrapping_add(b'A')
        } else {
            c
        }
    }
    let mut i = 0u32;
    loop {
        let x = fold(g_byte(m, a.wrapping_add(i)).unwrap_or(0));
        let y = fold(g_byte(m, b.wrapping_add(i)).unwrap_or(0));
        if x != y {
            return i32::from(x) - i32::from(y);
        }
        if y == 0 {
            return 0;
        }
        i = i.wrapping_add(1);
    }
}

/// Find `c` in the guest string `s`. Zero when it is absent.
fn g_strchr(m: &Memory<'_>, s: u32, c: u32) -> u32 {
    let mut i = 0u32;
    loop {
        match g_byte(m, s.wrapping_add(i)) {
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

/// The `fn_240b28` case fold, issued through the caller's host.
fn fold_byte(h: &mut impl Host, m: &mut Memory<'_>, c: u8) -> u8 {
    (h.call_m(0x240b28, &[u32::from(c)], m) & 0xff) as u8
}


/// The `fn_26bb6c` dictionary lookup: the decoded entry of `text`.
///
/// The exact ARM call is `fn_26bb6c(state, text, start, end, fold, length)`:
/// `state` is the `DictionaryState`, `start` and `end` point at the halfword
/// bounds of the query inside `text` and `length` points at the halfword that
/// receives the length of the decoded entry. The decoded bytes are written to
/// `state->output`.
///
/// The ROM's dictionary header offsets are the ones above; `src/fn_26bb6c.c`
/// puts `record_size`, `offset_size` and `offset_bytes` four bytes too high.
/// The `fn_26bb6c` decode with native bounds: the entry of `text` between
/// the `s16` bounds `first` and `last`, or `None` when the ROM returns 0.
///
/// `fn_26bb6c_m` (the exact ARM call) and `fn_1ea4e0_m` (whose `start`,
/// `end` and `length` are native locals) share this core, so the decode
/// runs once for both entries.
fn lookup_entry(
    m: &mut Memory<'_>,
    h: &mut impl Host,
    state: u32,
    text: u32,
    first: i16,
    last: i16,
    fold: u32,
) -> Option<u16> {
    let tables = match g_word(m, state.wrapping_add(ST_TABLES)) {
        Some(v) => v,
        None => return None,
    };
    let header = match g_word(m, tables.wrapping_add(TB_HEADER)) {
        Some(v) => v,
        None => return None,
    };
    let span = (i32::from(last) - i32::from(first)) + 1;
    let max_length = match g_half(m, header.wrapping_add(HD_MAX_LENGTH)) {
        Some(v) => v,
        None => return None,
    };
    if span <= 0 || span > i32::from(max_length) {
        return None;
    }
    let input_min = match g_byte(m, header.wrapping_add(HD_INPUT_MIN)) {
        Some(v) => v,
        None => return None,
    };
    let input_max = match g_byte(m, header.wrapping_add(HD_INPUT_MAX)) {
        Some(v) => v,
        None => return None,
    };
    let mut i = first;
    while i <= last {
        let raw = match g_byte(m, text.wrapping_add(i as i32 as u32)) {
            Some(v) => v,
            None => return None,
        };
        let c = if fold != 0 {
            fold_byte(h, m, raw)
        } else {
            raw
        };
        // The ROM only rejects a character above the range: the lower bound
        // never reaches the branch (`ldrbls` leaves the tables pointer in the
        // register it masks).
        if c >= input_min && c > input_max {
            return None;
        }
        i = i.wrapping_add(1);
    }
    if last < first {
        return None;
    }
    // Lane b owns the trie lookup: `first`/`last` are the `s16` bounds the
    // ARM passes by value to `fn_26c1c0`.
    let found = crate::component_live_b::fn_26c1c0_m(
        m,
        h,
        state,
        text,
        i32::from(first),
        i32::from(last),
        fold as i32,
    );
    if found < 0 {
        return None;
    }
    // The offset of the entry, read out of the packed offset table.
    let offset_bits = match g_byte(m, header.wrapping_add(HD_OFFSET_BITS)) {
        Some(v) => v,
        None => return None,
    };
    let mut offset_size = match g_half(m, header.wrapping_add(HD_OFFSET_SIZE)) {
        Some(v) => v,
        None => return None,
    };
    let offset_bytes = match g_word(m, header.wrapping_add(HD_OFFSET_BYTES)) {
        Some(v) => v,
        None => return None,
    };
    let offsets = match g_word(m, tables.wrapping_add(TB_OFFSETS)) {
        Some(v) => v,
        None => return None,
    };
    let position = (found as u32).wrapping_mul(u32::from(offset_bits));
    let bit = position & 7;
    let offset = position >> 3;
    if offset.wrapping_add(u32::from(offset_size)) > offset_bytes {
        offset_size = (offset_bytes.wrapping_sub(offset) & 0xffff) as u16;
    }
    let h0 = match g_word(m, state.wrapping_add(ST_H0)) {
        Some(v) => v,
        None => return None,
    };
    let h1 = match g_word(m, state.wrapping_add(ST_H1)) {
        Some(v) => v,
        None => return None,
    };
    let rc = h.call_m(
        0x17a1a4,
        &[
            h0,
            h1,
            offsets.wrapping_add(offset),
            u32::from(offset_size),
            state.wrapping_add(ST_BUFFER),
        ],
        m,
    );
    if rc != 0 {
        return None;
    }
    let width = u32::from(offset_bits);
    let total_bits = bit + width;
    let buffer = match g_word(m, state.wrapping_add(ST_BUFFER)) {
        Some(v) => v,
        None => return None,
    };
    let value = if total_bits <= 8 {
        let first_byte = match g_byte(m, buffer) {
            Some(v) => v,
            None => return None,
        };
        u32::from(first_byte >> (8 - width - bit)) & (0xffff_ffffu32 >> (32 - width))
    } else {
        let first_byte = match g_byte(m, buffer) {
            Some(v) => v,
            None => return None,
        };
        let mut value = (u32::from(first_byte & (0xff >> bit))) << (total_bits - 8);
        let mut byte = 1u32;
        if total_bits > 16 {
            loop {
                let next = match g_byte(m, buffer.wrapping_add(byte)) {
                    Some(v) => v,
                    None => return None,
                };
                value = value.wrapping_add(u32::from(next) << (total_bits - byte * 8 - 8));
                byte += 1;
                if total_bits <= (byte + 1) * 8 {
                    break;
                }
            }
        }
        let next = match g_byte(m, buffer.wrapping_add(byte)) {
            Some(v) => v,
            None => return None,
        };
        value.wrapping_add(u32::from(next) >> ((byte + 1) * 8 - width - bit))
    };
    h.call_m(0x179208, &[h0, h1, buffer], m);
    if (value as i32) < 0 {
        return None;
    }
    // The record of the entry.
    let records = match g_word(m, tables.wrapping_add(TB_RECORDS)) {
        Some(v) => v,
        None => return None,
    };
    let record_size = match g_half(m, header.wrapping_add(HD_RECORD_SIZE)) {
        Some(v) => v,
        None => return None,
    };
    let rc = h.call_m(
        0x17a1a4,
        &[
            h0,
            h1,
            records.wrapping_add(value),
            u32::from(record_size),
            state.wrapping_add(ST_BUFFER),
        ],
        m,
    );
    if rc != 0 {
        return None;
    }
    let buffer = match g_word(m, state.wrapping_add(ST_BUFFER)) {
        Some(v) => v,
        None => return None,
    };
    let output = match g_word(m, state.wrapping_add(ST_OUTPUT)) {
        Some(v) => v,
        None => return None,
    };
    let pair_base = match g_byte(m, header.wrapping_add(HD_PAIR_BASE)) {
        Some(v) => v,
        None => return None,
    };
    let pairs = match g_word(m, tables.wrapping_add(TB_PAIRS)) {
        Some(v) => v,
        None => return None,
    };
    let separator = match g_byte(m, header.wrapping_add(HD_SEPARATOR)) {
        Some(v) => v,
        None => return None,
    };
    let prefix_min = match g_byte(m, header.wrapping_add(HD_PREFIX_MIN)) {
        Some(v) => v,
        None => return None,
    };
    let prefix_max = match g_byte(m, header.wrapping_add(HD_PREFIX_MAX)) {
        Some(v) => v,
        None => return None,
    };
    // The byte-pair expansion of the record.
    let mut temporary = [0u8; 44];
    let mut count = 0u16;
    let mut expanded = 0u16;
    let mut pos = 0u32;
    let mut c = match g_byte(m, buffer.wrapping_add(pos)) {
        Some(v) => v,
        None => return None,
    };
    loop {
        pos += 1;
        if !p_byte(m, output.wrapping_add(u32::from(count)), c) {
            return None;
        }
        count += 1;
        loop {
            let current = match g_byte(m, output.wrapping_add(u32::from(expanded))) {
                Some(v) => v,
                None => return None,
            };
            let index = u32::from(current.wrapping_sub(pair_base));
            let mut replacement = match g_byte(m, pairs.wrapping_add(index * 2)) {
                Some(v) => v,
                None => return None,
            };
            while replacement != 0 {
                let span = count.wrapping_sub(1).wrapping_sub(expanded);
                let mut j = 0u16;
                while j < span {
                    let value = match g_byte(m, output.wrapping_add(u32::from(count - j - 1))) {
                        Some(v) => v,
                        None => return None,
                    };
                    if !p_byte(m, output.wrapping_add(u32::from(count - j)), value) {
                        return None;
                    }
                    j += 1;
                }
                // the rom re-reads output[expanded] each pass; it holds the last replacement
                let head = match g_byte(m, output.wrapping_add(u32::from(expanded))) {
                    Some(v) => v,
                    None => return None,
                };
                let head_index = u32::from(head.wrapping_sub(pair_base));
                let second = match g_byte(m, pairs.wrapping_add(head_index * 2 + 1)) {
                    Some(v) => v,
                    None => return None,
                };
                if !p_byte(m, output.wrapping_add(u32::from(expanded) + 1), second) {
                    return None;
                }
                if !p_byte(m, output.wrapping_add(u32::from(expanded)), replacement) {
                    return None;
                }
                let next_index = u32::from(replacement.wrapping_sub(pair_base));
                replacement = match g_byte(m, pairs.wrapping_add(next_index * 2)) {
                    Some(v) => v,
                    None => return None,
                };
                count += 1;
            }
            expanded += 1;
            if expanded >= count {
                break;
            }
        }
        c = match g_byte(m, buffer.wrapping_add(pos)) {
            Some(v) => v,
            None => return None,
        };
        if c == separator || (c >= prefix_min && c <= prefix_max) {
            break;
        }
    }
    // The prefix chain of the entry.
    let mut prefix = match g_byte(m, output) {
        Some(v) => v,
        None => return None,
    };
    while prefix >= prefix_min && prefix <= prefix_max {
        let mut c = match g_byte(m, buffer.wrapping_add(pos)) {
            Some(v) => v,
            None => return None,
        };
        while c != separator && (c < prefix_min || c > prefix_max) {
            pos += 1;
            c = match g_byte(m, buffer.wrapping_add(pos)) {
                Some(v) => v,
                None => return None,
            };
        }
        if c == separator {
            pos += 1;
        }
        let mut next_prefix = c;
        if next_prefix < prefix_min || next_prefix > prefix_max {
            next_prefix = prefix_min.wrapping_sub(1);
        }
        if next_prefix >= prefix {
            pos += 1;
            continue;
        }
        let mut span = 0u16;
        let mut expanded = 0u16;
        let mut remaining = prefix;
        loop {
            let byte = match g_byte(m, buffer.wrapping_add(pos)) {
                Some(v) => v,
                None => return None,
            };
            pos += 1;
            if span as usize >= temporary.len() {
                return None;
            }
            temporary[span as usize] = byte;
            span += 1;
            loop {
                let current = temporary[expanded as usize];
                let index = u32::from(current.wrapping_sub(pair_base));
                let mut replacement = match g_byte(m, pairs.wrapping_add(index * 2)) {
                    Some(v) => v,
                    None => return None,
                };
                while replacement != 0 {
                    let mut j = span.wrapping_sub(1);
                    while j > expanded {
                        temporary[(j + 1) as usize] = temporary[j as usize];
                        j -= 1;
                    }
                    span += 1;
                    // the rom re-indexes by temporary[expanded], which holds the last replacement
                    let head_index = u32::from(temporary[expanded as usize].wrapping_sub(pair_base));
                    let second = match g_byte(m, pairs.wrapping_add(head_index * 2 + 1)) {
                        Some(v) => v,
                        None => return None,
                    };
                    temporary[(expanded + 1) as usize] = second;
                    temporary[expanded as usize] = replacement;
                    let next_index = u32::from(replacement.wrapping_sub(pair_base));
                    replacement = match g_byte(m, pairs.wrapping_add(next_index * 2)) {
                        Some(v) => v,
                        None => return None,
                    };
                }
                expanded += 1;
                remaining = remaining.wrapping_sub(1);
                if !(expanded < span && remaining >= next_prefix) {
                    break;
                }
            }
            if remaining < next_prefix {
                break;
            }
        }
        let index = u32::from(prefix.wrapping_sub(next_prefix));
        let new_length = count.wrapping_add(index as u16);
        let mut j = 0u16;
        while j < count.wrapping_sub(1) {
            let value = match g_byte(m, output.wrapping_add(u32::from(count - j - 1))) {
                Some(v) => v,
                None => return None,
            };
            if !p_byte(m, output.wrapping_add(u32::from(new_length - j - 1)), value) {
                return None;
            }
            j += 1;
        }
        let mut j = 0u16;
        while j <= index as u16 {
            if !p_byte(m, output.wrapping_add(u32::from(j)), temporary[j as usize]) {
                return None;
            }
            j += 1;
        }
        count = new_length;
        prefix = next_prefix;
    }
    h.call_m(0x179208, &[h0, h1, buffer], m);
    Some(count)
}

/// The `fn_26bb6c` dictionary lookup: the decoded entry of `text`.
///
/// The exact ARM call is `fn_26bb6c(state, text, start, end, fold, length)`:
/// `state` is the `DictionaryState`, `start` and `end` point at the halfword
/// bounds of the query inside `text` and `length` points at the halfword that
/// receives the length of the decoded entry. The decoded bytes are written to
/// `state->output`.
///
/// The ROM's dictionary header offsets are the ones above (see the file
/// header). The trie lookup is lane b's `component_live_b::fn_26c1c0_m`.
pub fn fn_26bb6c_m(
    m: &mut Memory<'_>,
    h: &mut impl Host,
    state: u32,
    text: u32,
    start: u32,
    end: u32,
    fold: u32,
    out_length: u32,
) -> u32 {
    let first = match g_half(m, start) {
        Some(v) => v as i16,
        None => return 0,
    };
    let last = match g_half(m, end) {
        Some(v) => v as i16,
        None => return 0,
    };
    match lookup_entry(m, h, state, text, first, last, fold) {
        Some(count) => {
            if !p_half(m, out_length, count) {
                return 0;
            }
            1
        }
        None => 0,
    }
}
/// The `LookupOwner` field offsets (`src/fn_1ea4e0.c`).
const OW_HEAP: u32 = 0x04;
const OW_RESOURCES: u32 = 0x0c;
const OW_LOGGER: u32 = 0x10;

/// The `LookupDictionary` field offsets: the size word, then the state.
const LD_SIZE: u32 = 0x00;
const LD_STATE: u32 = 0x04;

/// The `LookupContext` field offsets (`src/fn_1ea4e0.c`).
const LC_MAIN: u32 = 0x04;
const LC_SYSTEM: u32 = 0x24;
const LC_LANGUAGE: u32 = 0x44;
const LC_KEYS: u32 = 0x68;
const LC_TABLES: u32 = 0x6c;
const LC_RESULTS: u32 = 0x70;
const LC_QUERY: u32 = 0x74;
const LC_RESOURCES: u32 = 0x78;
const LC_DICT_COUNT: u32 = 0x88;
const LC_RESULT_BLOCKS: u32 = 0x8a;
const LC_RESULT_CAP: u32 = 0x8c;
const LC_QUERY_CAP: u32 = 0x8e;
const LC_RESOURCE_NAME: u32 = 0x9e;

/// The `fn_176f94` gate tag and size of the lookup entry.
const LOOKUP_TAG: u32 = 0xf37f;
const LOOKUP_SIZE: u32 = 0x2a4;

/// The header offset `fn_26bb54` reads: the separator byte the lookup
/// entry reports (`src/fn_26bb54.c` reads the header word and then + 9).
const HD_SEPARATOR_OUT: u32 = 0x09;

/// `D_8f8f44`, the log module of the lookup entry.
const LOOKUP_MODULE: u32 = 0x8f8f44;

/// The error codes of the lookup entry.
const LOOKUP_GATE: u32 = 0x89802008;
const LOOKUP_BAD: u32 = 0x89802007;
const LOOKUP_MEMORY: u32 = 0x8980200a;
const LOOKUP_INVALID: u32 = 0x89802014;

/// Append the guest string `src` to the guest string `dst`.
fn g_strcat(m: &mut Memory<'_>, dst: u32, src: u32) -> bool {
    let base = match g_strlen(m, dst) {
        Some(v) => v,
        None => return false,
    };
    let len = match g_strlen(m, src) {
        Some(v) => v,
        None => return false,
    };
    if !g_range(m, dst.wrapping_add(base), len + 1) || !g_range(m, src, len + 1) {
        return false;
    }
    let s = (src - m.base) as usize;
    let d = (dst - m.base) as usize;
    m.bytes.copy_within(s..s + len as usize + 1, d + base as usize);
    true
}

/// Write zeroes to `n` bytes at `dst`.
fn g_memset(m: &mut Memory<'_>, dst: u32, n: u32) -> bool {
    if !g_range(m, dst, n) {
        return false;
    }
    let d = (dst - m.base) as usize;
    for b in &mut m.bytes[d..d + n as usize] {
        *b = 0;
    }
    true
}

/// Append the literal C string `lit` (terminator included) to `dst`.
fn g_strcat_lit(m: &mut Memory<'_>, dst: u32, lit: &[u8]) -> bool {
    let base = match g_strlen(m, dst) {
        Some(v) => v,
        None => return false,
    };
    let len = lit.len().wrapping_sub(1) as u32;
    if !g_range(m, dst.wrapping_add(base), len + 1) {
        return false;
    }
    let d = (dst - m.base) as usize + base as usize;
    m.bytes[d..d + len as usize + 1].copy_from_slice(lit);
    true
}

/// True when the guest string at `p` equals the literal C string `lit`.
fn g_lit_eq(m: &Memory<'_>, p: u32, lit: &[u8]) -> bool {
    let mut i = 0u32;
    for &want in lit {
        if g_byte(m, p.wrapping_add(i)).unwrap_or(0) != want {
            return false;
        }
        i = i.wrapping_add(1);
    }
    true
}

/// True when the first `n` guest bytes at `p` equal the literal's first `n`.
fn g_lit_prefix_eq(m: &Memory<'_>, p: u32, lit: &[u8], n: u32) -> bool {
    let mut i = 0u32;
    while i < n {
        if g_byte(m, p.wrapping_add(i)).unwrap_or(0) != lit[i as usize] {
            return false;
        }
        i = i.wrapping_add(1);
    }
    true
}

/// Copy the literal `lit` (terminator included) onto the heap.
///
/// The `sysdct`/`lid` resource names are static literals with no arena
/// home; the resource loader keeps the pointer, so the port never frees it.
fn alloc_lit(m: &mut Memory<'_>, h: &mut impl Host, heap: u32, lit: &[u8]) -> u32 {
    let p = h.call_m(0x196394, &[heap, lit.len() as u32], m);
    if p == 0 {
        return 0;
    }
    if !g_range(m, p, lit.len() as u32) {
        return 0;
    }
    let d = (p - m.base) as usize;
    m.bytes[d..d + lit.len()].copy_from_slice(lit);
    p
}

/// Render `value` in base 10 into `buffer`, terminator included.
///
/// The ROM's `fn_176ac8` digit table is the engine's lowercase digits; base
/// 10 uses plain decimal digits either way, so the lookup entry formats the
/// numbered-query suffix inline.
fn render_decimal(mut value: u32, buffer: &mut [u8]) {
    let mut digits = [0u8; 10];
    let mut count = 0usize;
    loop {
        digits[count] = b'0' + (value % 10) as u8;
        count += 1;
        value /= 10;
        if value == 0 {
            break;
        }
    }
    for (i, &d) in digits[..count].iter().rev().enumerate() {
        buffer[i] = d;
    }
    buffer[count] = 0;
}

/// The shared tail of the lookup entry: the ROM's `cleanup`, `failure` and
/// `done` labels.
///
/// Frees the lookup key when the entry allocated one, stores the result
/// vector on success, unwinds the half-built dictionary on failure, issues
/// the leaving log and returns `rc`.
#[allow(clippy::too_many_arguments)]
fn lookup_tail(
    m: &mut Memory<'_>,
    h: &mut impl Host,
    ctx: u32,
    heap: u32,
    key: u32,
    lookup_key: u32,
    allocated: bool,
    count_addr: u32,
    out_addr: u32,
    dictionary: u32,
    pending: bool,
    rc: u32,
) -> u32 {
    if allocated && lookup_key != 0 && lookup_key != key {
        h.call_m(0x196428, &[heap, lookup_key], m);
    }
    if rc < 0x80000000 {
        if g_half(m, count_addr).unwrap_or(0) != 0 {
            let results = g_word(m, ctx.wrapping_add(LC_RESULTS)).unwrap_or(0);
            p_word(m, out_addr, results);
        }
    } else if pending {
        let dict_count = g_half(m, ctx.wrapping_add(LC_DICT_COUNT)).unwrap_or(0);
        let keys = g_word(m, ctx.wrapping_add(LC_KEYS)).unwrap_or(0);
        let tables = g_word(m, ctx.wrapping_add(LC_TABLES)).unwrap_or(0);
        let res = g_word(m, ctx.wrapping_add(LC_RESOURCES)).unwrap_or(0);
        if keys != 0 && g_word(m, keys.wrapping_add(u32::from(dict_count) * 4)).unwrap_or(0) != 0
        {
            let slot = keys.wrapping_add(u32::from(dict_count) * 4);
            h.call_m(0x196428, &[heap, g_word(m, slot).unwrap_or(0)], m);
            p_word(m, slot, 0);
        }
        if tables != 0 && g_word(m, tables.wrapping_add(u32::from(dict_count) * 4)).unwrap_or(0) != 0
        {
            let slot = tables.wrapping_add(u32::from(dict_count) * 4);
            let table = g_word(m, slot).unwrap_or(0);
            let state = dictionary.wrapping_add(LD_STATE);
            if res != 0 && g_word(m, res.wrapping_add(u32::from(dict_count) * 4)).unwrap_or(0) != 0
            {
                p_word(m, state.wrapping_add(ST_TABLES), table);
                h.call_m(
                    0x26bb28,
                    &[state, g_word(m, res.wrapping_add(u32::from(dict_count) * 4)).unwrap_or(0)],
                    m,
                );
            }
            h.call_m(0x196428, &[heap, table], m);
            p_word(m, slot, 0);
        }
        if res != 0 && g_word(m, res.wrapping_add(u32::from(dict_count) * 4)).unwrap_or(0) != 0 {
            let owner = g_word(m, ctx).unwrap_or(0);
            let owner_res = g_word(m, owner.wrapping_add(OW_RESOURCES)).unwrap_or(0);
            let slot = res.wrapping_add(u32::from(dict_count) * 4);
            h.call_m(0x179f3c, &[owner_res, g_word(m, slot).unwrap_or(0)], m);
            p_word(m, slot, 0);
        }
        let _ = dict_count;
    }
    let owner = g_word(m, ctx).unwrap_or(0);
    let logger = g_word(m, owner.wrapping_add(OW_LOGGER)).unwrap_or(0);
    h.call_m(0x1738e8, &[logger, LOOKUP_MODULE, 4, 0], m);
    rc
}

/// The `fn_1ea4e0` dictionary lookup (fe_dctlkp_Lookup, cfg idx 7,
/// getString/lookupDomain).
///
/// The exact ARM call is `fn_1ea4e0(ctx, handle, key, text, out, count,
/// separator)`: `ctx` is the `LookupContext`, `key` names the dictionary,
/// `text` is the query, `out` receives the result vector, `count` points at
/// the halfword that enters as `0xffff`/`1` (single lookup) or a running
/// count and leaves as the result count, and `separator` receives the
/// dictionary separator byte. The `+0x69c` call is the file-local
/// `lookup_entry` (the `fn_26bb6c` decode); every other guest call goes
/// through the caller's host, so the port leaves the arena exactly as the
/// ROM leaves it.
///
/// Pure string and memory helpers run inline on the arena; the heap, the
/// resources, the log, the entry gate and the dictionary setup family go
/// through the host.
#[allow(clippy::too_many_arguments)]
pub fn fn_1ea4e0_m(
    m: &mut Memory<'_>,
    h: &mut impl Host,
    ctx: u32,
    handle: u32,
    key: u32,
    text: u32,
    out: u32,
    count: u32,
    separator: u32,
) -> u32 {
    let owner = match g_word(m, ctx) {
        Some(v) => v,
        None => return LOOKUP_GATE,
    };
    let heap = match g_word(m, owner.wrapping_add(OW_HEAP)) {
        Some(v) => v,
        None => return LOOKUP_GATE,
    };
    let resources = match g_word(m, owner.wrapping_add(OW_RESOURCES)) {
        Some(v) => v,
        None => return LOOKUP_GATE,
    };
    let logger = match g_word(m, owner.wrapping_add(OW_LOGGER)) {
        Some(v) => v,
        None => return LOOKUP_GATE,
    };
    let rc = h.call_m(0x176f94, &[ctx, handle, LOOKUP_TAG, LOOKUP_SIZE], m);
    if rc >= 0x80000000 {
        return LOOKUP_GATE;
    }
    h.call_m(0x1738e8, &[logger, LOOKUP_MODULE, 5, 0, text, key], m);
    if key == 0 || text == 0 || out == 0 || count == 0 || separator == 0 {
        h.call_m(0x173dc0, &[logger, LOOKUP_MODULE, 0xa029, 0], m);
        return lookup_tail(m, h, ctx, heap, key, 0, false, count, out, 0, false, LOOKUP_BAD);
    }
    let entry_count = g_half(m, count).unwrap_or(0);
    let mut lookup_key = key;
    let mut allocated = false;
    // The dictionary the key selects: the single lookup (`*count == 1`)
    // copies the key with the `C` marker and picks the language or the
    // system table; every other entry uses the main table as is.
    let dictionary = if entry_count == 1 {
        let len = g_strlen(m, key).unwrap_or(0);
        let p = h.call_m(0x196394, &[heap, len.wrapping_add(2)], m);
        if p == 0 {
            h.call_m(0x173dc0, &[logger, LOOKUP_MODULE, 0xa028, 0], m);
            return lookup_tail(m, h, ctx, heap, key, 0, false, count, out, 0, false, LOOKUP_MEMORY);
        }
        if !g_strcpy(m, p, key) {
            h.call_m(0x173dc0, &[logger, LOOKUP_MODULE, 0xa028, 0], m);
            return lookup_tail(m, h, ctx, heap, key, p, true, count, out, 0, false, LOOKUP_MEMORY);
        }
        let base = g_strlen(m, p).unwrap_or(0);
        if !p_byte(m, p.wrapping_add(base), b'C') || !p_byte(m, p.wrapping_add(base + 1), 0) {
            h.call_m(0x173dc0, &[logger, LOOKUP_MODULE, 0xa028, 0], m);
            return lookup_tail(m, h, ctx, heap, key, p, true, count, out, 0, false, LOOKUP_MEMORY);
        }
        lookup_key = p;
        allocated = true;
        if g_lit_eq(m, key, b"lidcfg\0")
            || g_lit_prefix_eq(m, key, b"mertfrq\0", 7)
            || g_lit_prefix_eq(m, key, b"merwfrq\0", 7)
        {
            let size = g_word(m, ctx.wrapping_add(LC_LANGUAGE).wrapping_add(LD_SIZE)).unwrap_or(0);
            if size == 0 {
                return lookup_tail(
                    m, h, ctx, heap, key, p, true, count, out, 0, false, LOOKUP_BAD,
                );
            }
            ctx.wrapping_add(LC_LANGUAGE)
        } else {
            let size = g_word(m, ctx.wrapping_add(LC_SYSTEM).wrapping_add(LD_SIZE)).unwrap_or(0);
            if size == 0 {
                return lookup_tail(
                    m, h, ctx, heap, key, p, true, count, out, 0, false, LOOKUP_BAD,
                );
            }
            ctx.wrapping_add(LC_SYSTEM)
        }
    } else {
        ctx.wrapping_add(LC_MAIN)
    };
    let single = entry_count == 0xffff || entry_count == 1;
    // The ROM zeroes the running count before the first query.
    if !p_half(m, count, 0) {
        return lookup_tail(
            m, h, ctx, heap, key, lookup_key, allocated, count, out, dictionary, false,
            LOOKUP_MEMORY,
        );
    }
    // The dictionary index of the lookup key.
    let dict_count = g_half(m, ctx.wrapping_add(LC_DICT_COUNT)).unwrap_or(0);
    let keys = g_word(m, ctx.wrapping_add(LC_KEYS)).unwrap_or(0);
    let mut index: i32 = -1;
    {
        let mut i = 0u16;
        while i < dict_count {
            let candidate = g_word(m, keys.wrapping_add(u32::from(i) * 4)).unwrap_or(0);
            if g_strcmp_ci(m, lookup_key, candidate) == 0 {
                index = i32::from(i);
                break;
            }
            i = i.wrapping_add(1);
        }
    }
    let state = dictionary.wrapping_add(LD_STATE);
    let mut maximum = 0u32;
    let mut pending = false;
    if index >= 0 {
        let tables = g_word(m, ctx.wrapping_add(LC_TABLES)).unwrap_or(0);
        let table = g_word(m, tables.wrapping_add(index as u32 * 4)).unwrap_or(0);
        if !p_word(m, state.wrapping_add(ST_TABLES), table) {
            return lookup_tail(
                m, h, ctx, heap, key, lookup_key, allocated, count, out, dictionary, false,
                LOOKUP_MEMORY,
            );
        }
    } else {
        // The dictionary is not loaded yet: grow the registries, load the
        // resource and build the tables through the host. The capture path
        // never reaches this branch (the `unixlit` table is already
        // loaded), so the test host rejects these calls.
        pending = true;
        let dc = u32::from(g_half(m, ctx.wrapping_add(LC_DICT_COUNT)).unwrap_or(0));
        let old_res = g_word(m, ctx.wrapping_add(LC_RESOURCES)).unwrap_or(0);
        let grown = h.call_m(0x1963e0, &[heap, old_res, dc.wrapping_add(2).wrapping_mul(4)], m);
        if grown == 0 {
            h.call_m(0x173dc0, &[logger, LOOKUP_MODULE, 0xa028, 0], m);
            return lookup_tail(
                m, h, ctx, heap, key, lookup_key, allocated, count, out, dictionary, pending,
                LOOKUP_MEMORY,
            );
        }
        if !p_word(m, ctx.wrapping_add(LC_RESOURCES), grown)
            || !p_word(m, grown.wrapping_add(dc * 4), 0)
            || !p_word(m, grown.wrapping_add(dc * 4 + 4), 0)
        {
            return lookup_tail(
                m, h, ctx, heap, key, lookup_key, allocated, count, out, dictionary, pending,
                LOOKUP_MEMORY,
            );
        }
        // The resource name: the ROM passes the static `sysdct`/`lid`
        // literals or the context's own name. Literals have no arena home,
        // so the port copies the selected name onto the heap once; the
        // resource loader keeps the pointer.
        let name = if dictionary == ctx.wrapping_add(LC_SYSTEM) {
            alloc_lit(m, h, heap, b"sysdct\0")
        } else if dictionary == ctx.wrapping_add(LC_LANGUAGE) {
            alloc_lit(m, h, heap, b"lid\0")
        } else {
            ctx.wrapping_add(LC_RESOURCE_NAME)
        };
        if name == 0 {
            h.call_m(0x173dc0, &[logger, LOOKUP_MODULE, 0xa028, 0], m);
            return lookup_tail(
                m, h, ctx, heap, key, lookup_key, allocated, count, out, dictionary, pending,
                LOOKUP_MEMORY,
            );
        }
        let slot = grown.wrapping_add(dc * 4);
        let rc = h.call_m(0x179f30, &[resources, name, slot], m);
        if rc >= 0x80000000 {
            return lookup_tail(
                m, h, ctx, heap, key, lookup_key, allocated, count, out, dictionary, pending, rc,
            );
        }
        let old_tables = g_word(m, ctx.wrapping_add(LC_TABLES)).unwrap_or(0);
        let grown_tables =
            h.call_m(0x1963e0, &[heap, old_tables, dc.wrapping_add(2).wrapping_mul(4)], m);
        if grown_tables == 0 {
            h.call_m(0x173dc0, &[logger, LOOKUP_MODULE, 0xa028, 0], m);
            return lookup_tail(
                m, h, ctx, heap, key, lookup_key, allocated, count, out, dictionary, pending,
                LOOKUP_MEMORY,
            );
        }
        if !p_word(m, ctx.wrapping_add(LC_TABLES), grown_tables)
            || !p_word(m, grown_tables.wrapping_add(dc * 4), 0)
            || !p_word(m, grown_tables.wrapping_add(dc * 4 + 4), 0)
        {
            return lookup_tail(
                m, h, ctx, heap, key, lookup_key, allocated, count, out, dictionary, pending,
                LOOKUP_MEMORY,
            );
        }
        let table = h.call_m(0x1963b8, &[heap, 1, 0x4c], m);
        if table == 0 {
            h.call_m(0x173dc0, &[logger, LOOKUP_MODULE, 0xa028, 0], m);
            return lookup_tail(
                m, h, ctx, heap, key, lookup_key, allocated, count, out, dictionary, pending,
                LOOKUP_MEMORY,
            );
        }
        if !p_word(m, grown_tables.wrapping_add(dc * 4), table)
            || !p_word(m, state.wrapping_add(ST_TABLES), table)
        {
            return lookup_tail(
                m, h, ctx, heap, key, lookup_key, allocated, count, out, dictionary, pending,
                LOOKUP_MEMORY,
            );
        }
        let size = g_word(m, dictionary.wrapping_add(LD_SIZE)).unwrap_or(0);
        let slot_value = g_word(m, grown.wrapping_add(dc * 4)).unwrap_or(0);
        let rc = h.call_m(0x26b7ec, &[state, lookup_key, size, slot_value], m);
        if rc >= 0x80000000 {
            if rc == LOOKUP_INVALID {
                h.call_m(0x196428, &[heap, table], m);
                p_word(m, grown_tables.wrapping_add(dc * 4), 0);
                h.call_m(0x179f3c, &[resources, slot_value], m);
                p_word(m, grown.wrapping_add(dc * 4), 0);
                return lookup_tail(
                    m, h, ctx, heap, key, lookup_key, allocated, count, out, dictionary, pending,
                    0,
                );
            }
            return lookup_tail(
                m, h, ctx, heap, key, lookup_key, allocated, count, out, dictionary, pending, rc,
            );
        }
        let old_keys = g_word(m, ctx.wrapping_add(LC_KEYS)).unwrap_or(0);
        let grown_keys =
            h.call_m(0x1963e0, &[heap, old_keys, dc.wrapping_add(2).wrapping_mul(4)], m);
        if grown_keys == 0 {
            h.call_m(0x173dc0, &[logger, LOOKUP_MODULE, 0xa028, 0], m);
            return lookup_tail(
                m, h, ctx, heap, key, lookup_key, allocated, count, out, dictionary, pending,
                LOOKUP_MEMORY,
            );
        }
        if !p_word(m, ctx.wrapping_add(LC_KEYS), grown_keys)
            || !p_word(m, grown_keys.wrapping_add(dc * 4), 0)
            || !p_word(m, grown_keys.wrapping_add(dc * 4 + 4), 0)
        {
            return lookup_tail(
                m, h, ctx, heap, key, lookup_key, allocated, count, out, dictionary, pending,
                LOOKUP_MEMORY,
            );
        }
        let len = g_strlen(m, lookup_key).unwrap_or(0);
        let key_copy = h.call_m(0x196394, &[heap, len.wrapping_add(1)], m);
        if key_copy == 0 || !g_strcpy(m, key_copy, lookup_key) {
            h.call_m(0x173dc0, &[logger, LOOKUP_MODULE, 0xa028, 0], m);
            return lookup_tail(
                m, h, ctx, heap, key, lookup_key, allocated, count, out, dictionary, pending,
                LOOKUP_MEMORY,
            );
        }
        if !p_word(m, grown_keys.wrapping_add(dc * 4), key_copy) {
            return lookup_tail(
                m, h, ctx, heap, key, lookup_key, allocated, count, out, dictionary, pending,
                LOOKUP_MEMORY,
            );
        }
        maximum = h.call_m(0x26bb44, &[state], m);
        if maximum == 0 {
            h.call_m(0x173dc0, &[logger, LOOKUP_MODULE, 0xa029, 0], m);
            return lookup_tail(
                m, h, ctx, heap, key, lookup_key, allocated, count, out, dictionary, pending,
                LOOKUP_BAD,
            );
        }
        let result_cap = g_half(m, ctx.wrapping_add(LC_RESULT_CAP)).unwrap_or(0);
        if u32::from(result_cap) < maximum {
            let output = g_word(m, state.wrapping_add(ST_OUTPUT)).unwrap_or(0);
            let grown_output = h.call_m(0x1963e0, &[heap, output, maximum.wrapping_add(1)], m);
            if grown_output == 0 {
                h.call_m(0x173dc0, &[logger, LOOKUP_MODULE, 0xa028, 0], m);
                return lookup_tail(
                    m, h, ctx, heap, key, lookup_key, allocated, count, out, dictionary, pending,
                    LOOKUP_MEMORY,
                );
            }
            if !p_word(m, ctx.wrapping_add(LC_MAIN).wrapping_add(LD_STATE).wrapping_add(ST_OUTPUT), grown_output)
                || !p_word(m, ctx.wrapping_add(LC_SYSTEM).wrapping_add(LD_STATE).wrapping_add(ST_OUTPUT), grown_output)
                || !p_word(m, ctx.wrapping_add(LC_LANGUAGE).wrapping_add(LD_STATE).wrapping_add(ST_OUTPUT), grown_output)
            {
                return lookup_tail(
                    m, h, ctx, heap, key, lookup_key, allocated, count, out, dictionary,
                    true, LOOKUP_MEMORY,
                );
            }
        }
        pending = false;
        if !p_half(m, ctx.wrapping_add(LC_DICT_COUNT), dc.wrapping_add(1) as u16) {
            return lookup_tail(
                m, h, ctx, heap, key, lookup_key, allocated, count, out, dictionary, false,
                LOOKUP_MEMORY,
            );
        }
    }
    // The separator byte of the dictionary header.
    {
        let tables = match g_word(m, state.wrapping_add(ST_TABLES)) {
            Some(v) => v,
            None => {
                return lookup_tail(
                    m, h, ctx, heap, key, lookup_key, allocated, count, out, dictionary,
                    pending, LOOKUP_BAD,
                );
            }
        };
        let header = match g_word(m, tables.wrapping_add(TB_HEADER)) {
            Some(v) => v,
            None => {
                return lookup_tail(
                    m, h, ctx, heap, key, lookup_key, allocated, count, out, dictionary,
                    pending, LOOKUP_BAD,
                );
            }
        };
        // The separator the ROM's `fn_26bb54` returns: the byte at header
        // + 9 (`src/fn_26bb54.c`, `component_live_c::fn_26bb54_m`), not the
        // header + 8 byte the `fn_26bb6c` expansion loop breaks on.
        let value = match g_byte(m, header.wrapping_add(HD_SEPARATOR_OUT)) {
            Some(v) => v,
            None => {
                return lookup_tail(
                    m, h, ctx, heap, key, lookup_key, allocated, count, out, dictionary,
                    pending, LOOKUP_BAD,
                );
            }
        };
        if !p_byte(m, separator, value) {
            return lookup_tail(
                m, h, ctx, heap, key, lookup_key, allocated, count, out, dictionary,
                pending, LOOKUP_MEMORY,
            );
        }
    }
    // Size the query buffer for the text.
    let mut query = g_word(m, ctx.wrapping_add(LC_QUERY)).unwrap_or(0);
    {
        let len = g_strlen(m, text).unwrap_or(0);
        let want = (len.wrapping_add(5) & 0xffff) as u16;
        let capacity = g_half(m, ctx.wrapping_add(LC_QUERY_CAP)).unwrap_or(0) as i16;
        if capacity < want as i16 {
            let grown = h.call_m(0x1963e0, &[heap, query, u32::from(want)], m);
            if grown == 0 {
                h.call_m(0x173dc0, &[logger, LOOKUP_MODULE, 0xa028, 0], m);
                return lookup_tail(
                    m, h, ctx, heap, key, lookup_key, allocated, count, out, dictionary,
                    pending, LOOKUP_MEMORY,
                );
            }
            if !p_word(m, ctx.wrapping_add(LC_QUERY), grown)
                || !p_half(m, ctx.wrapping_add(LC_QUERY_CAP), want)
            {
                return lookup_tail(
                    m, h, ctx, heap, key, lookup_key, allocated, count, out, dictionary,
                    pending, LOOKUP_MEMORY,
                );
            }
            query = grown;
        }
    }
    if !p_word(m, out, g_word(m, ctx.wrapping_add(LC_RESULTS)).unwrap_or(0)) {
        return lookup_tail(
            m, h, ctx, heap, key, lookup_key, allocated, count, out, dictionary, pending,
            LOOKUP_MEMORY,
        );
    }
    {
        let result_cap = g_half(m, ctx.wrapping_add(LC_RESULT_CAP)).unwrap_or(0);
        if u32::from(result_cap) < maximum {
            let blocks = u32::from(g_half(m, ctx.wrapping_add(LC_RESULT_BLOCKS)).unwrap_or(0));
            let results = g_word(m, ctx.wrapping_add(LC_RESULTS)).unwrap_or(0);
            let mut i = 0u32;
            let mut bad = false;
            while i < blocks.wrapping_mul(5).wrapping_sub(1) {
                let slot = g_word(m, results.wrapping_add(i * 4)).unwrap_or(0);
                let grown = h.call_m(0x1963e0, &[heap, slot, maximum.wrapping_add(1)], m);
                if grown == 0 {
                    bad = true;
                    break;
                }
                if !p_word(m, results.wrapping_add(i * 4), grown) {
                    bad = true;
                    break;
                }
                i += 1;
            }
            if bad || !p_half(m, ctx.wrapping_add(LC_RESULT_CAP), maximum as u16) {
                h.call_m(0x173dc0, &[logger, LOOKUP_MODULE, 0xa028, 0], m);
                return lookup_tail(
                    m, h, ctx, heap, key, lookup_key, allocated, count, out, dictionary,
                    pending, LOOKUP_MEMORY,
                );
            }
        }
    }
    let multiple = if single { 0u32 } else { 1u32 };
    loop {
        if g_half(m, count).unwrap_or(0) != 0 && multiple == 0 {
            break;
        }
        if !g_strcpy(m, query, text) {
            return lookup_tail(
                m, h, ctx, heap, key, lookup_key, allocated, count, out, dictionary,
                pending, LOOKUP_MEMORY,
            );
        }
        let running = g_half(m, count).unwrap_or(0);
        if running != 0 {
            if !g_strcat_lit(m, query, b"\x17\0") {
                return lookup_tail(
                    m, h, ctx, heap, key, lookup_key, allocated, count, out, dictionary,
                    pending, LOOKUP_MEMORY,
                );
            }
            let mut number = [0u8; 8];
            render_decimal(u32::from(running), &mut number);
            let base = g_strlen(m, query).unwrap_or(0);
            let mut len = 0u32;
            while len < number.len() as u32 && number[len as usize] != 0 {
                len += 1;
            }
            if !g_range(m, query.wrapping_add(base), len + 1) {
                return lookup_tail(
                    m, h, ctx, heap, key, lookup_key, allocated, count, out, dictionary,
                    pending, LOOKUP_MEMORY,
                );
            }
            let d = (query - m.base) as usize + base as usize;
            m.bytes[d..d + len as usize + 1].copy_from_slice(&number[..len as usize + 1]);
        }
        let len = g_strlen(m, query).unwrap_or(0);
        let end = (len.wrapping_sub(1) & 0xffff) as u16 as i16;
        let entry = match lookup_entry(m, h, state, query, 0, end, 0) {
            Some(v) => v,
            None => break,
        };
        let blocks = g_half(m, ctx.wrapping_add(LC_RESULT_BLOCKS)).unwrap_or(0);
        let running = g_half(m, count).unwrap_or(0);
        if u32::from(running).wrapping_add(2) > u32::from(blocks).wrapping_mul(5) {
            let results = g_word(m, ctx.wrapping_add(LC_RESULTS)).unwrap_or(0);
            let grown = h.call_m(
                0x1963e0,
                &[heap, results, u32::from(blocks).wrapping_add(1).wrapping_mul(20)],
                m,
            );
            if grown == 0 {
                h.call_m(0x173dc0, &[logger, LOOKUP_MODULE, 0xa028, 0], m);
                return lookup_tail(
                    m, h, ctx, heap, key, lookup_key, allocated, count, out, dictionary,
                    pending, LOOKUP_MEMORY,
                );
            }
            if !p_word(m, ctx.wrapping_add(LC_RESULTS), grown) {
                return lookup_tail(
                    m, h, ctx, heap, key, lookup_key, allocated, count, out, dictionary,
                    pending, LOOKUP_MEMORY,
                );
            }
            let results = grown;
            let mut i = if blocks == 0 {
                0u32
            } else {
                u32::from(blocks).wrapping_mul(5).wrapping_sub(1)
            };
            let capacity = g_half(m, ctx.wrapping_add(LC_RESULT_CAP)).unwrap_or(0);
            let mut bad = !g_memset(
                m,
                results.wrapping_add(i * 4),
                u32::from(blocks).wrapping_add(1).wrapping_mul(5).wrapping_sub(i).wrapping_mul(4),
            );
            while !bad && i < u32::from(blocks).wrapping_mul(5).wrapping_add(4) {
                let slot = h.call_m(0x196394, &[heap, u32::from(capacity).wrapping_add(1)], m);
                if slot == 0 || !p_word(m, results.wrapping_add(i * 4), slot) {
                    bad = true;
                    break;
                }
                i += 1;
            }
            if bad || !p_word(m, results.wrapping_add(i * 4), 0) {
                if bad {
                    h.call_m(0x173dc0, &[logger, LOOKUP_MODULE, 0xa028, 0], m);
                }
                return lookup_tail(
                    m, h, ctx, heap, key, lookup_key, allocated, count, out, dictionary,
                    pending, LOOKUP_MEMORY,
                );
            }
            if !p_half(m, ctx.wrapping_add(LC_RESULT_BLOCKS), blocks.wrapping_add(1)) {
                return lookup_tail(
                    m, h, ctx, heap, key, lookup_key, allocated, count, out, dictionary,
                    pending, LOOKUP_MEMORY,
                );
            }
        }
        let results = g_word(m, ctx.wrapping_add(LC_RESULTS)).unwrap_or(0);
        let slot = g_word(m, results.wrapping_add(u32::from(running) * 4)).unwrap_or(0);
        let output = g_word(m, state.wrapping_add(ST_OUTPUT)).unwrap_or(0);
        let mut j = 0u16;
        let mut bad = false;
        while j < entry {
            match g_byte(m, output.wrapping_add(u32::from(j))) {
                Some(v) => {
                    if !p_byte(m, slot.wrapping_add(u32::from(j)), v) {
                        bad = true;
                        break;
                    }
                }
                None => {
                    bad = true;
                    break;
                }
            }
            j = j.wrapping_add(1);
        }
        if bad || !p_byte(m, slot.wrapping_add(u32::from(entry)), 0) {
            return lookup_tail(
                m, h, ctx, heap, key, lookup_key, allocated, count, out, dictionary,
                pending, LOOKUP_MEMORY,
            );
        }
        if !p_half(m, count, running.wrapping_add(1)) {
            return lookup_tail(
                m, h, ctx, heap, key, lookup_key, allocated, count, out, dictionary,
                pending, LOOKUP_MEMORY,
            );
        }
    }
    lookup_tail(m, h, ctx, heap, key, lookup_key, allocated, count, out, dictionary, pending, 0)
}

/// The `TokenContext` field offsets (`src/fn_1ede44.c`).
const TC_PUNCT: u32 = 0x64;
const TC_IFACE: u32 = 0x84;
const TC_DOM_COUNT: u32 = 0x94;
const TC_DOMAINS: u32 = 0x98;

/// The `TokenInterface` slot offsets: the ROM calls each entry through the
/// pointer the arena holds (`blx ip`), so the port reads the slot and
/// issues the same entry through the caller's host.
const IF_FIRST: u32 = 0x24;
const IF_NEXT: u32 = 0x28;
const IF_FIND: u32 = 0x34;
const IF_SET: u32 = 0x50;
const IF_GET: u32 = 0x58;

/// The `TokenAnnotation` field offsets: the type halfword at +0, the
/// position word at +12 and the domain pointer at +20.
const TA_POSITION: u32 = 0x0c;
const TA_DOMAIN: u32 = 0x14;

/// The `TokenDomain` field offsets.
const TD_NAME: u32 = 0x00;
const TD_ALPHA: u32 = 0x04;
const TD_DIGITS: u32 = 0x08;
const TD_PUNCT: u32 = 0x0c;

/// The error codes of the tokenize entry.
const TOKEN_GATE: u32 = 0x89802008;
const TOKEN_MEMORY: u32 = 0x8980200a;
const TOKEN_ERROR: u32 = 0xa02a;

/// Caller stack-scratch frame of the `fn_1ede44` tokenize entry: the ROM
/// keeps these locals on its machine stack (`push {r0-r11, lr}; sub sp,
/// sp, #52` in the objdump of `build/target/fn_1ede44.o`) and passes
/// their addresses to the token interface. The caller passes the base that
/// plays the role of the ROM's `sp`; the slots sit at the ROM's exact
/// sp-relative offsets so the addresses the port hands out match:
/// position `u16` @+12 (`strh/ldrh [sp, #12]`; set field-2 value),
/// item `u16` @+16 (`add r0, sp, #16` into `find`; `first` reloads it),
/// text `*mut u8` @+20 (`add r0, sp, #20` into `get`),
/// token `u16` @+24 (`add r3, sp, #24` into `first`/`next`; set id),
/// type `u8` @+28 (set field-0 value),
/// length `u16` @+32 (`add r1, sp, #32` into `get`, never read back),
/// written `u16` @+36 (`add r6, sp, #36` into every `set`, never read),
/// start `u16` @+48 (`ldrh [sp, #48]` picks `first` vs `next`;
/// set field-1 value). The ROM's remaining two locals stay Rust locals:
/// domain @+40 and the annotation cursor @+44 are only ever touched by
/// the inlined domain search, never handed to the guest.
/// The ROM zeroes position/item/token/type/length/written at entry
/// (objdump +0x20..+0x3c); text and start take values below instead.
const S_POSITION: u32 = 12;
const S_ITEM: u32 = 16;
const S_TEXT: u32 = 20;
const S_TOKEN: u32 = 24;
const S_TYPE: u32 = 28;
const S_LENGTH: u32 = 32;
const S_WRITTEN: u32 = 36;
const S_START: u32 = 48;
const SCRATCH_SIZE: u32 = 50;

/// True when `c` is token whitespace: space, tab, newline, carriage return
/// or underscore.
fn token_space(c: u8) -> bool {
    c == b' ' || c == b'\t' || c == b'\n' || c == b'\r' || c == b'_'
}

/// The `fn_1ede44` digit test for the domain's digit set.
fn token_digit(m: &Memory<'_>, ctx: u32, domain: u16, c: u8) -> bool {
    if domain == 999 {
        return (c.wrapping_sub(b'0') as u32) <= 9;
    }
    let domains = g_word(m, ctx.wrapping_add(TC_DOMAINS)).unwrap_or(0);
    let digits = g_word(
        m,
        domains.wrapping_add(u32::from(domain) * 16).wrapping_add(TD_DIGITS),
    )
    .unwrap_or(0);
    if digits == 0 {
        return (c.wrapping_sub(b'0') as u32) <= 9;
    }
    c != 0 && g_strchr(m, digits, u32::from(c)) != 0
}

/// The `fn_1ede44` punctuation test for the domain's punctuation set.
fn token_punctuation(m: &Memory<'_>, ctx: u32, domain: u16, c: u8) -> bool {
    if domain == 999 {
        let punct = g_word(m, ctx.wrapping_add(TC_PUNCT)).unwrap_or(0);
        return c != 0 && g_strchr(m, punct, u32::from(c)) != 0;
    }
    let domains = g_word(m, ctx.wrapping_add(TC_DOMAINS)).unwrap_or(0);
    let punct = g_word(
        m,
        domains.wrapping_add(u32::from(domain) * 16).wrapping_add(TD_PUNCT),
    )
    .unwrap_or(0);
    c != 0 && g_strchr(m, punct, u32::from(c)) != 0
}

/// The `fn_1ede44` alpha test for the domain's alpha set.
fn token_alpha(m: &Memory<'_>, ctx: u32, domain: u16, c: u8) -> bool {
    if domain == 999 {
        return !token_space(c)
            && !token_punctuation(m, ctx, domain, c)
            && !token_digit(m, ctx, domain, c);
    }
    let domains = g_word(m, ctx.wrapping_add(TC_DOMAINS)).unwrap_or(0);
    let alpha = g_word(
        m,
        domains.wrapping_add(u32::from(domain) * 16).wrapping_add(TD_ALPHA),
    )
    .unwrap_or(0);
    c != 0 && g_strchr(m, alpha, u32::from(c)) != 0
}

/// The `fn_1ede44` domain search: the domain of the annotation covering
/// `position`, or 999 when no annotation names one.
///
/// `index` is the caller's scan cursor over the annotations. Domain names
/// compare through lane c2's `fn_1765ac`, guarded to the arena like every
/// other guest read on a port path.
fn token_domain(
    m: &Memory<'_>,
    ctx: u32,
    annotations: u32,
    count: u16,
    position: u16,
    index: &mut u16,
    domain: &mut u16,
) {
    let first = g_word(m, annotations.wrapping_add(TA_POSITION)).unwrap_or(0);
    let mut selected = u32::from(count);
    let mut guard = u32::from(count).wrapping_add(1);
    while *index < count && guard > 0 {
        guard -= 1;
        let at = annotations.wrapping_add(u32::from(*index) * 16);
        let pos = g_word(m, at.wrapping_add(TA_POSITION)).unwrap_or(0);
        if pos.wrapping_sub(first) > u32::from(position) {
            break;
        }
        if g_half(m, at).unwrap_or(0) == b'$' as u16 {
            selected = u32::from(*index);
        }
        *index = index.wrapping_add(1);
    }
    if selected >= u32::from(count) {
        return;
    }
    let which = g_word(
        m,
        annotations.wrapping_add(selected * 16).wrapping_add(TA_DOMAIN),
    )
    .unwrap_or(0);
    if which == 0 {
        return;
    }
    let n = g_half(m, ctx.wrapping_add(TC_DOM_COUNT)).unwrap_or(0);
    let domains = g_word(m, ctx.wrapping_add(TC_DOMAINS)).unwrap_or(0);
    let mut d = 0u16;
    let mut found = 999u16;
    while d < n {
        let name = g_word(
            m,
            domains.wrapping_add(u32::from(d) * 16).wrapping_add(TD_NAME),
        )
        .unwrap_or(0);
        if name != 0
            && g_range(m, which, g_strlen(m, which).unwrap_or(0) + 1)
            && g_range(m, name, g_strlen(m, name).unwrap_or(0) + 1)
            && crate::component_live_c2::fn_1765ac(m, which, name) == 0
        {
            found = d;
            break;
        }
        d = d.wrapping_add(1);
    }
    *domain = found;
}

/// The shared tail of the tokenize entry: the ROM's `leave` label.
///
/// Issues the leaving log and returns `rc`. The `error` label (the `0xa02a`
/// log) is issued by the caller first. The frame lives in caller scratch,
/// so there is nothing to free, exactly like the ROM's stack frame.
fn token_tail(m: &mut Memory<'_>, h: &mut impl Host, logger: u32, rc: u32) -> u32 {
    h.call_m(0x1738e8, &[logger, LOOKUP_MODULE, 4, 0], m);
    rc
}

/// The `fn_1ede44` tokenize entry (fe_dctlkp_Tokenize, cfg idx 11).
///
/// The exact ARM call is `fn_1ede44(p, handle, text_handle, annotations,
/// count)`: `p` is the `TokenContext`, `text_handle` is the two-word text
/// handle, `annotations` points at `count` `TokenAnnotation`s, and the port
/// emits one token record per run of alpha, digit, punctuation or space
/// through the `TokenInterface` the arena holds (`first` at +0x24, `next`
/// at +0x28, `find` at +0x34, `set` at +0x50, `get` at +0x58). Every
/// interface call goes through the caller's host with the arena's own entry
/// address (the entry gates its own handle, exactly like the ROM's callee
/// does); the pure character tests run inline.
///
/// The ROM keeps `item`, `token`, `text`, `length`, `written`, `type`,
/// `start` and `position` on its stack and passes their addresses down.
/// The port keeps the same 20 bytes in caller stack scratch at the ROM's
/// exact sp-relative offsets (`S_*` above, from `build/target/fn_1ede44.o`),
/// so no heap block is allocated or freed and no caller slot is clobbered.
#[allow(clippy::too_many_arguments)]
pub fn fn_1ede44_m(
    m: &mut Memory<'_>,
    h: &mut impl Host,
    scratch: u32,
    ctx: u32,
    handle: u32,
    th0: u32,
    th1: u32,
    annotations: u32,
    count: u16,
) -> u32 {
    let owner = match g_word(m, ctx) {
        Some(v) => v,
        None => return TOKEN_GATE,
    };
    let logger = match g_word(m, owner.wrapping_add(0x10)) {
        Some(v) => v,
        None => return TOKEN_GATE,
    };
    let rc = h.call_m(0x176f94, &[ctx, handle, LOOKUP_TAG, LOOKUP_SIZE], m);
    if rc >= 0x80000000 {
        return TOKEN_GATE;
    }
    h.call_m(0x1738e8, &[logger, LOOKUP_MODULE, 4, 0], m);
    let iface = match g_word(m, ctx.wrapping_add(TC_IFACE)) {
        Some(v) => v,
        None => return token_tail(m, h, logger, TOKEN_MEMORY),
    };
    // The tokenize frame lives in caller stack scratch at the ROM's
    // sp-relative offsets; fail closed when the scratch is not mapped.
    if !g_range(m, scratch, SCRATCH_SIZE) {
        h.call_m(0x173dc0, &[logger, LOOKUP_MODULE, TOKEN_ERROR, 0], m);
        return token_tail(m, h, logger, TOKEN_MEMORY);
    }
    let item = scratch.wrapping_add(S_ITEM);
    let token = scratch.wrapping_add(S_TOKEN);
    let text_slot = scratch.wrapping_add(S_TEXT);
    let length_slot = scratch.wrapping_add(S_LENGTH);
    let written = scratch.wrapping_add(S_WRITTEN);
    let type_slot = scratch.wrapping_add(S_TYPE);
    let start_slot = scratch.wrapping_add(S_START);
    let position_slot = scratch.wrapping_add(S_POSITION);
    // The ROM's entry zeroes (objdump +0x20..+0x3c).
    if !p_half(m, position_slot, 0)
        || !p_half(m, item, 0)
        || !p_half(m, token, 0)
        || !p_byte(m, type_slot, 0)
        || !p_half(m, length_slot, 0)
        || !p_half(m, written, 0)
    {
        h.call_m(0x173dc0, &[logger, LOOKUP_MODULE, TOKEN_ERROR, 0], m);
        return token_tail(m, h, logger, TOKEN_MEMORY);
    }
    let find = g_word(m, iface.wrapping_add(IF_FIND)).unwrap_or(0);
    let mut rc = h.call_m(find, &[th0, th1, 1, 0, item], m);
    if rc >= 0x80000000 {
        return token_tail(m, h, logger, rc);
    }
    let get = g_word(m, iface.wrapping_add(IF_GET)).unwrap_or(0);
    let item_val = u32::from(g_half(m, item).unwrap_or(0));
    rc = h.call_m(get, &[th0, th1, item_val, 0, text_slot, length_slot], m);
    if rc >= 0x80000000 {
        return token_tail(m, h, logger, rc);
    }
    let text = g_word(m, text_slot).unwrap_or(0);
    let mut position: u16 = 0;
    let mut annotation_index: u16 = 0;
    let mut token_val: u16 = 0;
    let mut error = false;
    loop {
        let text_len = g_strlen(m, text).unwrap_or(0);
        if u32::from(position) >= text_len {
            break;
        }
        let start: u16 = position;
        let mut domain: u16 = 999;
        token_domain(m, ctx, annotations, count, start, &mut annotation_index, &mut domain);
        let c = match g_byte(m, text.wrapping_add(u32::from(position))) {
            Some(v) => v,
            None => {
                rc = TOKEN_MEMORY;
                error = true;
                break;
            }
        };
        let type_val: u8;
        if token_alpha(m, ctx, domain, c) {
            let mut alpha_length: u16 = 0;
            type_val = 0;
            loop {
                alpha_length = alpha_length.wrapping_add(1);
                position = position.wrapping_add(1);
                if alpha_length >= 128 {
                    break;
                }
                if u32::from(position) >= g_strlen(m, text).unwrap_or(0) {
                    break;
                }
                let next = match g_byte(m, text.wrapping_add(u32::from(position))) {
                    Some(v) => v,
                    None => {
                        rc = TOKEN_MEMORY;
                        error = true;
                        break;
                    }
                };
                if !token_alpha(m, ctx, domain, next) {
                    break;
                }
            }
            if error {
                break;
            }
        } else if token_digit(m, ctx, domain, c) {
            type_val = 1;
            loop {
                position = position.wrapping_add(1);
                if u32::from(position) >= g_strlen(m, text).unwrap_or(0) {
                    break;
                }
                let next = match g_byte(m, text.wrapping_add(u32::from(position))) {
                    Some(v) => v,
                    None => {
                        rc = TOKEN_MEMORY;
                        error = true;
                        break;
                    }
                };
                if !token_digit(m, ctx, domain, next) {
                    break;
                }
            }
            if error {
                break;
            }
        } else if token_punctuation(m, ctx, domain, c) {
            type_val = 2;
            position = position.wrapping_add(1);
        } else if token_space(c) {
            type_val = 3;
            loop {
                position = position.wrapping_add(1);
                token_domain(
                    m,
                    ctx,
                    annotations,
                    count,
                    start,
                    &mut annotation_index,
                    &mut domain,
                );
                if u32::from(position) >= g_strlen(m, text).unwrap_or(0) {
                    break;
                }
                let next = match g_byte(m, text.wrapping_add(u32::from(position))) {
                    Some(v) => v,
                    None => {
                        rc = TOKEN_MEMORY;
                        error = true;
                        break;
                    }
                };
                if !token_space(next) {
                    break;
                }
            }
            if error {
                break;
            }
        } else {
            h.call_m(0x1738e8, &[logger, LOOKUP_MODULE, 4, 0, 0x8f8fd8, u32::from(c)], m);
            type_val = 0;
            position = position.wrapping_add(1);
        }
        if !p_byte(m, type_slot, type_val)
            || !p_half(m, start_slot, start)
            || !p_half(m, position_slot, position)
        {
            rc = TOKEN_MEMORY;
            error = true;
            break;
        }
        let first = g_word(m, iface.wrapping_add(IF_FIRST)).unwrap_or(0);
        let next = g_word(m, iface.wrapping_add(IF_NEXT)).unwrap_or(0);
        let set = g_word(m, iface.wrapping_add(IF_SET)).unwrap_or(0);
        if start == 0 {
            rc = h.call_m(first, &[th0, th1, 3, u32::from(item_val), token], m);
        } else {
            rc = h.call_m(next, &[th0, th1, u32::from(token_val), token], m);
        }
        if rc >= 0x80000000 {
            error = true;
            break;
        }
        token_val = g_half(m, token).unwrap_or(0);
        let fields = [(0u32, type_slot), (1u32, start_slot), (2u32, position_slot)];
        for (field, slot) in fields {
            rc = h.call_m(
                set,
                &[th0, th1, u32::from(token_val), field, 1, slot, written],
                m,
            );
            if rc >= 0x80000000 {
                error = true;
                break;
            }
        }
        if error {
            break;
        }
    }
    if error {
        h.call_m(0x173dc0, &[logger, LOOKUP_MODULE, TOKEN_ERROR, 0], m);
    }
    token_tail(m, h, logger, rc)
}
