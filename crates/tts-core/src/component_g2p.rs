//! Live G2P dictionary lookup `fn_2c9ba0`, over the caller's arena.
//!
//! The exact ARM call is
//! `fn_2c9ba0(engine, normalized, text, lookup_type, domain, position,
//! token, handle_obj, handle_data, end)` (ten words: `r0`-`r3` plus six
//! stack words). `engine` is the word-engine object: `+0x00` the services
//! (`services+0x04` the allocator, `services+0x10` the logger), `+0x0c`
//! the token vtable (`+0x2c` insert-after, `+0x50` set), `+0x10`/`+0x14`
//! the dictionary handle, `+0x18` the dictionary vtable (`+0x48` lookup),
//! `+0x1c`/`+0x20` the MDE handle, `+0x24` the MDE vtable (`+0x24` input,
//! `+0x28` output, `+0x2c` select), `+0x58` the pronunciation buffer and
//! `+0x172` the lookup flags. Every guest call is issued through the host;
//! `sp` is the ARM entry SP (the caller's body SP). The frame hangs off
//! `sp` at the disassembled offsets, exactly where the ARM keeps it; the
//! `mode` buffer and the address-taken cells all live there. The pointer
//! tables the C keeps
//! on its stack live in Rust locals. Guest allocation (the pronunciation
//! resize) goes through the live heap (`0x1963e0`).
//!
//! The ROM's literals live in the ROM image the caller owns: `dctbased_mde`
//! at 0x2c9eb0, `normal` at 0x2c9b2c, `_` at 0x2c9b34, the log formats at
//! 0x2c9b1c/0x2c9ec0/0x2c9b38/0x2ca360/0x2ca370/0x2ca6a8/0x2ca6c8, `0` at
//! 0x2c9eac, the empty strings at 0x2c9ebc/0x2ca6c4, `\x11` at 0x2ca69c,
//! `*` at 0x2ca6f0 and the logger module at 0x8f9532.

#![allow(dead_code)]

use crate::{boot::Memory, component_live_b::Host};

/// `fn_2c9ba0` frame slots as ARM SP-relative offsets. The ARM keeps
/// `push {r0-r3, r4-r8, sb, sl, fp, lr}` (0x34) plus `sub sp, #0x184`, so
/// the body SP is the entry SP minus 0x1B8 and every slot below hangs off
/// it; the incoming stack words (domain at +0x1B8, position at +0x1BC,
/// token at +0x1C0, handle at +0x1C4/+0x1C8, end at +0x1CC) live in the
/// caller's frame, exactly like the ARM's. No fixed window: the caller
/// passes the live entry SP and the frame lands where the ARM puts it.
pub const G2P9B_SUB: u32 = 0x1b8;
const G_MODE: u32 = 0x20;
const G_RESULT_COUNT: u32 = 0x10;
const G_LENGTH: u32 = 0x1c;
const G_WRITTEN: u32 = 0x14;
const G_NUMBER: u32 = 0x18;
const G_TOKEN: u32 = 0x1c0;
const G_POSITION: u32 = 0x1bc;
const G_SEPARATOR: u32 = 0xf4;
const G_KIND: u32 = 0x110;
const G_RESULTS: u32 = 0x10c;
const G_MDE_RESULT: u32 = 0x114;
const G_LOOKUP: u32 = 0xf8;

const CAT: u32 = 0x8f9532;
const LIT_MODE: u32 = 0x2c9eb0;
const LIT_NORMAL: u32 = 0x2c9b2c;
const LIT_UNDER: u32 = 0x2c9b34;
const FMT_L1: u32 = 0x2c9b1c;
const FMT_L2: u32 = 0x2c9ec0;
const FMT_O1: u32 = 0x2c9b38;
const FMT_LOOKUP: u32 = 0x2ca360;
const FMT_VALID: u32 = 0x2ca370;
const FMT_DOMAIN: u32 = 0x2ca6a8;
const FMT_USAGE: u32 = 0x2ca6c8;
const ZERO_S: u32 = 0x2c9eac;
const EMPTY_S: u32 = 0x2c9ebc;
const EMPTY2_S: u32 = 0x2ca6c4;
const ONE_S: u32 = 0x2ca69c;
const STAR_S: u32 = 0x2ca6f0;

const FAIL_MEM: u32 = 0x89b0200a;
const FAIL_HANDLE: u32 = 0x89b02007;

fn g_word(m: &Memory<'_>, p: u32) -> Option<u32> {
    let at = p.checked_sub(m.base)? as usize;
    let b = m.bytes.get(at..at.checked_add(4)?)?;
    Some(u32::from_le_bytes(b.try_into().ok()?))
}

fn g_half(m: &Memory<'_>, p: u32) -> Option<u16> {
    let at = p.checked_sub(m.base)? as usize;
    let b = m.bytes.get(at..at.checked_add(2)?)?;
    Some(u16::from_le_bytes(b.try_into().ok()?))
}

fn g_byte(m: &Memory<'_>, p: u32) -> Option<u8> {
    let at = p.checked_sub(m.base)? as usize;
    Some(*m.bytes.get(at)?)
}

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

fn p_byte(m: &mut Memory<'_>, p: u32, v: u8) -> bool {
    if !g_range(m, p, 1) {
        return false;
    }
    let at = (p - m.base) as usize;
    m.bytes[at] = v;
    true
}

fn p_half(m: &mut Memory<'_>, p: u32, v: u16) -> bool {
    if !g_range(m, p, 2) {
        return false;
    }
    let at = (p - m.base) as usize;
    m.bytes[at..at + 2].copy_from_slice(&v.to_le_bytes());
    true
}

fn p_word(m: &mut Memory<'_>, p: u32, v: u32) -> bool {
    if !g_range(m, p, 4) {
        return false;
    }
    let at = (p - m.base) as usize;
    m.bytes[at..at + 4].copy_from_slice(&v.to_le_bytes());
    true
}

/// The `fn_2c9ba0` G2P lookup, live over the caller's arena.
///
/// Ports `src/fn_2c9748.c:150` 1:1. `position`/`token_in` are the `u16`
/// stack words the ARM receives; `end` is the `u16 *` the ARM fills with
/// `position + strlen(text)`. Returns the last callee status, `0` on the
/// select-fallback path, `0x89b02007` on a null result word and
/// `0x89b0200a` when the pronunciation resize fails.
#[allow(clippy::too_many_arguments)]
pub fn fn_2c9ba0_m(
    m: &mut Memory<'_>,
    h: &mut impl Host,
    sp: u32,
    engine: u32,
    normalized: u32,
    text: u32,
    lookup_type: u32,
    domain: u32,
    position: u16,
    token_in: u16,
    handle_obj: u32,
    handle_data: u32,
    end: u32,
) -> u32 {
    // The ARM body SP: entry SP minus the 0x34 push minus the 0x184
    // frame. Every slot below hangs off it at the disassembled offset,
    // so the frame sits exactly where the ARM keeps it.
    let body = sp.wrapping_sub(G2P9B_SUB);
    let mode = body.wrapping_add(G_MODE);
    let a_count = body.wrapping_add(G_RESULT_COUNT);
    let a_length = body.wrapping_add(G_LENGTH);
    let a_written = body.wrapping_add(G_WRITTEN);
    let a_number = body.wrapping_add(G_NUMBER);
    let a_token = body.wrapping_add(G_TOKEN);
    let a_sep = body.wrapping_add(G_SEPARATOR);
    let a_kind = body.wrapping_add(G_KIND);
    let a_results = body.wrapping_add(G_RESULTS);
    let a_mde_result = body.wrapping_add(G_MDE_RESULT);
    let a_lookup = body.wrapping_add(G_LOOKUP);

    // The scratch frame holds the mode buffer and the address-taken cells.
    if !g_range(m, mode, 128)
        || !g_range(m, a_count, 2)
        || !g_range(m, a_length, 2)
        || !g_range(m, a_written, 2)
        || !g_range(m, a_number, 2)
        || !g_range(m, a_token, 2)
        || !g_range(m, a_sep, 1)
        || !g_range(m, a_kind, 1)
        || !g_range(m, a_results, 4)
        || !g_range(m, a_mde_result, 4)
        || !g_range(m, a_lookup, 4)
        || !p_half(m, a_count, 0)
        || !p_half(m, a_written, 0)
        || !p_half(m, a_number, 1)
        || !p_half(m, a_token, token_in)
        || !p_byte(m, a_kind, 8)
    {
        return FAIL_MEM;
    }

    let services = g_word(m, engine).unwrap_or(0);
    let token_api = g_word(m, engine.wrapping_add(0x0c)).unwrap_or(0);
    let dict_obj = g_word(m, engine.wrapping_add(0x10)).unwrap_or(0);
    let dict_data = g_word(m, engine.wrapping_add(0x14)).unwrap_or(0);
    let dict_api = g_word(m, engine.wrapping_add(0x18)).unwrap_or(0);
    let mde_obj = g_word(m, engine.wrapping_add(0x1c)).unwrap_or(0);
    let mde_data = g_word(m, engine.wrapping_add(0x20)).unwrap_or(0);
    let mde_api = g_word(m, engine.wrapping_add(0x24)).unwrap_or(0);
    let logger = g_word(m, services.wrapping_add(0x10)).unwrap_or(0);
    let allocator = g_word(m, services.wrapping_add(4)).unwrap_or(0);
    let lookup_flags = g_byte(m, engine.wrapping_add(0x172)).unwrap_or(0);

    let f_insert_after = g_word(m, token_api.wrapping_add(0x2c)).unwrap_or(0);
    let f_set = g_word(m, token_api.wrapping_add(0x50)).unwrap_or(0);
    let f_dict_lookup = g_word(m, dict_api.wrapping_add(0x48)).unwrap_or(0);
    let f_mde_input = g_word(m, mde_api.wrapping_add(0x24)).unwrap_or(0);
    let f_mde_output = g_word(m, mde_api.wrapping_add(0x28)).unwrap_or(0);
    let f_mde_select = g_word(m, mde_api.wrapping_add(0x2c)).unwrap_or(0);

    h.call_m(0x195cd0, &[mode, LIT_MODE], m);
    let mut status: u32;
    let mode_length = h.call_m(0x195d90, &[mode], m) as u16;
    h.call_m(0x1738e8, &[logger, CAT, 5, 0, FMT_L1, mode, normalized], m);
    let normalized_len = h.call_m(0x195d90, &[normalized], m);
    status = h.call_m(f_mde_input, &[mde_obj, mde_data, 0, normalized, normalized_len], m);
    if status >= 0x80000000 {
        return status;
    }
    h.call_m(0x1738e8, &[logger, CAT, 5, 0, FMT_L2, mode, text], m);
    let text_len = h.call_m(0x195d90, &[text], m);
    status = h.call_m(f_mde_input, &[mde_obj, mde_data, 1, text, text_len], m);
    if status >= 0x80000000 {
        return status;
    }
    if h.call_m(0x195d58, &[domain, LIT_NORMAL], m) != 0
        && h.call_m(0x195d90, &[domain], m).wrapping_add(u32::from(mode_length)).wrapping_add(2)
            < 128
    {
        h.call_m(0x195c90, &[mode, LIT_UNDER], m);
        h.call_m(0x195c90, &[mode, domain], m);
    }
    status = h.call_m(f_mde_select, &[mde_obj, mde_data, mode], m);
    if status >= 0x80000000 {
        if !p_byte(m, mode.wrapping_add(u32::from(mode_length)), 0) {
            return FAIL_MEM;
        }
        status = h.call_m(f_mde_select, &[mde_obj, mde_data, mode], m);
        if status >= 0x80000000 {
            return 0;
        }
    }
    status = h.call_m(f_mde_output, &[mde_obj, mde_data, 0, a_mde_result, a_length], m);
    if status >= 0x80000000 {
        return status;
    }
    let mde_result = g_word(m, a_mde_result).unwrap_or(0);
    let length = g_half(m, a_length).unwrap_or(0);
    if !p_byte(m, mde_result.wrapping_add(u32::from(length)), 0) {
        return FAIL_MEM;
    }
    h.call_m(0x1738e8, &[logger, CAT, 5, 0, FMT_O1, mode, mde_result], m);
    status = h.call_m(f_mde_output, &[mde_obj, mde_data, 1, a_lookup, a_length], m);
    if status >= 0x80000000 {
        return status;
    }
    let lookup = g_word(m, a_lookup).unwrap_or(0);

    let mut prefixes = [0u32; 10];
    let mut suffixes = [0u32; 10];
    let mut contexts = [0u32; 10];
    let mut infixes = [0u32; 10];
    let context_count: u32;
    let mut infix_count = 0u8;
    let base: u32;
    let mut cursor: u32;
    status = 0;

    if g_byte(m, mde_result).unwrap_or(0) == b'{' {
        context_count = 2;
        prefixes[0] = mde_result.wrapping_add(1);
        prefixes[1] = ZERO_S;
        contexts[0] = ZERO_S;
        contexts[1] = mde_result.wrapping_add(1);
        if !p_byte(m, mde_result, 0) {
            return FAIL_MEM;
        }
        let p = h.call_m(0x195d30, &[mde_result.wrapping_add(1), u32::from(b'}')], m);
        if p == 0 {
            return status;
        }
        base = p.wrapping_add(1);
        if !p_byte(m, p, 0) {
            return FAIL_MEM;
        }
    } else {
        base = mde_result;
        context_count = 1;
        prefixes[0] = ZERO_S;
        contexts[0] = ZERO_S;
    }
    cursor = h.call_m(0x195d30, &[base, u32::from(b'{')], m);
    if cursor == 0 && context_count == 1 {
        return status;
    }
    while cursor != 0 && g_byte(m, cursor).unwrap_or(0) != b'}' {
        if (infix_count as usize) >= infixes.len() {
            return FAIL_MEM;
        }
        infixes[infix_count as usize] = cursor.wrapping_add(1);
        if !p_byte(m, cursor, 0) {
            return FAIL_MEM;
        }
        let p = h.call_m(0x195d30, &[cursor.wrapping_add(1), u32::from(b'+')], m);
        if p == 0 {
            return status;
        }
        if (infix_count as usize) >= suffixes.len() {
            return FAIL_MEM;
        }
        suffixes[infix_count as usize] = p.wrapping_add(1);
        infix_count = infix_count.wrapping_add(1);
        if !p_byte(m, p, 0) {
            return FAIL_MEM;
        }
        cursor = h.call_m(0x195d30, &[p.wrapping_add(1), u32::from(b',')], m);
        if cursor == 0 {
            cursor = h.call_m(
                0x195d30,
                &[suffixes[infix_count.wrapping_sub(1) as usize], u32::from(b'}')],
                m,
            );
        }
    }
    if cursor != 0 && !p_byte(m, cursor, 0) {
        return FAIL_MEM;
    }

    let mut i = 0u16;
    while u32::from(i) < context_count {
        let mut j = 0u16;
        while u32::from(j) < u32::from(infix_count) {
            if h.call_m(0x195d58, &[contexts[i as usize], ZERO_S], m) == 0
                && h.call_m(0x195d58, &[suffixes[j as usize], ZERO_S], m) == 0
            {
                j = j.wrapping_add(1);
                continue;
            }
            if h.call_m(0x195d58, &[prefixes[i as usize], ZERO_S], m) != 0 {
                h.call_m(0x195cd0, &[lookup, prefixes[i as usize]], m);
                h.call_m(0x195c90, &[lookup, base], m);
            } else {
                h.call_m(0x195cd0, &[lookup, base], m);
            }
            if h.call_m(0x195d58, &[infixes[j as usize], ZERO_S], m) != 0 {
                h.call_m(0x195c90, &[lookup, infixes[j as usize]], m);
            }
            h.call_m(0x1738e8, &[logger, CAT, 5, 0, FMT_LOOKUP, lookup], m);
            status = h.call_m(
                f_dict_lookup,
                &[
                    dict_obj,
                    dict_data,
                    lookup_type,
                    lookup,
                    a_results,
                    a_count,
                    a_sep,
                    u32::from(lookup_flags),
                ],
                m,
            );
            if status >= 0x80000000 {
                return status;
            }
            let result_count = g_half(m, a_count).unwrap_or(0);
            if result_count == 0 {
                j = j.wrapping_add(1);
                continue;
            }
            let results = g_word(m, a_results).unwrap_or(0);
            let separator = u32::from(g_byte(m, a_sep).unwrap_or(0));
            let mut extra = 0u32;
            let mut k = 0u16;
            while u32::from(k) < u32::from(result_count) {
                let word = g_word(m, results.wrapping_add(u32::from(k) * 4)).unwrap_or(0);
                // The record fields, split in place at the separator.
                let mut p = h.call_m(0x195d30, &[word, separator], m);
                let mut entry_type = 0u32;
                let mut result_domain = 0u32;
                let mut feature5 = 0u32;
                let mut feature6 = 0u32;
                let mut number_text = 0u32;
                let mut usage = 0u32;
                if p != 0 {
                    if !p_byte(m, p, 0) {
                        return FAIL_MEM;
                    }
                    entry_type = p.wrapping_add(1);
                    p = h.call_m(0x195d30, &[entry_type, separator], m);
                    if p != 0 {
                        if !p_byte(m, p, 0) {
                            return FAIL_MEM;
                        }
                        result_domain = p.wrapping_add(1);
                        p = h.call_m(0x195d30, &[result_domain, separator], m);
                        if p != 0 {
                            if !p_byte(m, p, 0) {
                                return FAIL_MEM;
                            }
                            feature5 = p.wrapping_add(1);
                            p = h.call_m(0x195d30, &[feature5, separator], m);
                            if p != 0 {
                                if !p_byte(m, p, 0) {
                                    return FAIL_MEM;
                                }
                                feature6 = p.wrapping_add(1);
                                p = h.call_m(0x195d30, &[feature6, separator], m);
                                if p != 0 {
                                    if !p_byte(m, p, 0) {
                                        return FAIL_MEM;
                                    }
                                    number_text = p.wrapping_add(1);
                                    p = h.call_m(0x195d30, &[number_text, separator], m);
                                    if p != 0 {
                                        if !p_byte(m, p, 0) {
                                            return FAIL_MEM;
                                        }
                                        usage = p.wrapping_add(1);
                                        p = h.call_m(0x195d30, &[usage, separator], m);
                                        if p != 0 {
                                            if !p_byte(m, p, 0) {
                                                return FAIL_MEM;
                                            }
                                            extra = p.wrapping_add(1);
                                            p = h.call_m(0x195d30, &[extra, separator], m);
                                            if p != 0 && !p_byte(m, p, 0) {
                                                return FAIL_MEM;
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
                let _ = entry_type;
                if word == 0 {
                    h.call_m(0x1738e8, &[logger, CAT, 0x84d1, 0], m);
                    return FAIL_HANDLE;
                }
                h.call_m(0x1738e8, &[logger, CAT, 5, 0, FMT_VALID, word], m);
                if result_domain != 0
                    && h.call_m(0x195d58, &[result_domain, EMPTY_S], m) != 0
                    && h.call_m(0x1960a8, &[result_domain, domain], m) == 0
                {
                    h.call_m(0x1738e8, &[logger, CAT, 5, 0, FMT_DOMAIN], m);
                    k = k.wrapping_add(1);
                    continue;
                }
                if usage != 0 && h.call_m(0x195d58, &[usage, ZERO_S], m) == 0 {
                    h.call_m(0x1738e8, &[logger, CAT, 5, 0, FMT_USAGE], m);
                    k = k.wrapping_add(1);
                    continue;
                }
                let end_len = h.call_m(0x195d90, &[text], m);
                if !p_half(m, end, position.wrapping_add(end_len as u16)) {
                    return FAIL_MEM;
                }
                let token = g_half(m, a_token).unwrap_or(0);
                status = h.call_m(
                    f_insert_after,
                    &[handle_obj, handle_data, u32::from(token), a_token],
                    m,
                );
                if status >= 0x80000000 {
                    return status;
                }
                let token = g_half(m, a_token).unwrap_or(0);
                status = h.call_m(
                    f_set,
                    &[handle_obj, handle_data, u32::from(token), 0, 1, a_kind, a_written],
                    m,
                );
                if status >= 0x80000000 {
                    return status;
                }
                // `&position` is the incoming stack word the ARM passes;
                // materialize it exactly like the ARM's own call setup.
                if !p_half(m, body.wrapping_add(G_POSITION), position) {
                    return FAIL_MEM;
                }
                status = h.call_m(
                    f_set,
                    &[handle_obj, handle_data, u32::from(token), 1, 1, body.wrapping_add(G_POSITION), a_written],
                    m,
                );
                if status >= 0x80000000 {
                    return status;
                }
                status = h.call_m(
                    f_set,
                    &[handle_obj, handle_data, u32::from(token), 2, 1, end, a_written],
                    m,
                );
                if status >= 0x80000000 {
                    return status;
                }
                let word_len = h.call_m(0x195d90, &[word], m);
                let cursor_len = h.call_m(0x195d90, &[cursor], m);
                if cursor != 0 && word_len.wrapping_add(cursor_len).wrapping_add(3) > 128 {
                    let pronunciation = g_word(m, engine.wrapping_add(0x58)).unwrap_or(0);
                    let grown = word_len.wrapping_add(cursor_len).wrapping_add(3);
                    let resized = h.call_m(0x1963e0, &[allocator, pronunciation, grown], m);
                    if resized == 0 {
                        h.call_m(0x1738e8, &[logger, CAT, 0x84d0, 0], m);
                        return FAIL_MEM;
                    }
                    if !p_word(m, engine.wrapping_add(0x58), resized) {
                        return FAIL_MEM;
                    }
                }
                let pronunciation = g_word(m, engine.wrapping_add(0x58)).unwrap_or(0);
                if h.call_m(0x195d58, &[contexts[i as usize], ZERO_S], m) != 0 {
                    h.call_m(0x195cd0, &[pronunciation, contexts[i as usize]], m);
                    h.call_m(0x195c90, &[pronunciation, ONE_S], m);
                } else {
                    h.call_m(0x195cd0, &[pronunciation, ONE_S], m);
                }
                h.call_m(0x195c90, &[pronunciation, word], m);
                h.call_m(0x195c90, &[pronunciation, ONE_S], m);
                if h.call_m(0x195d58, &[suffixes[j as usize], ZERO_S], m) != 0 {
                    h.call_m(0x195c90, &[pronunciation, suffixes[j as usize]], m);
                }
                let pron_len = h.call_m(0x195d90, &[pronunciation], m);
                status = h.call_m(
                    f_set,
                    &[
                        handle_obj,
                        handle_data,
                        u32::from(token),
                        3,
                        pron_len.wrapping_add(1) & 0xffff,
                        pronunciation,
                        a_written,
                    ],
                    m,
                );
                if status >= 0x80000000 {
                    return status;
                }
                if feature5 != 0 {
                    if h.call_m(0x195d58, &[feature5, EMPTY2_S], m) != 0 {
                        let f5_len = h.call_m(0x195d90, &[feature5], m);
                        status = h.call_m(
                            f_set,
                            &[
                                handle_obj,
                                handle_data,
                                u32::from(token),
                                5,
                                f5_len.wrapping_add(1) & 0xffff,
                                feature5,
                                a_written,
                            ],
                            m,
                        );
                    } else {
                        status = h.call_m(
                            f_set,
                            &[handle_obj, handle_data, u32::from(token), 5, 2, STAR_S, a_written],
                            m,
                        );
                    }
                    if status >= 0x80000000 {
                        return status;
                    }
                }
                if feature6 != 0 {
                    if h.call_m(0x195d58, &[feature6, EMPTY2_S], m) != 0 {
                        let f6_len = h.call_m(0x195d90, &[feature6], m);
                        status = h.call_m(
                            f_set,
                            &[
                                handle_obj,
                                handle_data,
                                u32::from(token),
                                6,
                                f6_len.wrapping_add(1) & 0xffff,
                                feature6,
                                a_written,
                            ],
                            m,
                        );
                    } else {
                        status = h.call_m(
                            f_set,
                            &[handle_obj, handle_data, u32::from(token), 6, 2, STAR_S, a_written],
                            m,
                        );
                    }
                    if status >= 0x80000000 {
                        return status;
                    }
                }
                let number_val = if number_text != 0 {
                    h.call_m(0x176cd8, &[number_text], m) as u16
                } else {
                    1
                };
                if !p_half(m, a_number, number_val) {
                    return FAIL_MEM;
                }
                status = h.call_m(
                    f_set,
                    &[handle_obj, handle_data, u32::from(token), 7, 1, a_number, a_written],
                    m,
                );
                if status >= 0x80000000 {
                    return status;
                }
                if extra != 0 {
                    let extra_len = h.call_m(0x195d90, &[extra], m);
                    status = h.call_m(
                        f_set,
                        &[
                            handle_obj,
                            handle_data,
                            u32::from(token),
                            9,
                            extra_len.wrapping_add(1) & 0xffff,
                            extra,
                            a_written,
                        ],
                        m,
                    );
                } else {
                    status = h.call_m(
                        f_set,
                        &[handle_obj, handle_data, u32::from(token), 9, 1, EMPTY2_S, a_written],
                        m,
                    );
                }
                if status >= 0x80000000 {
                    return status;
                }
                k = k.wrapping_add(1);
            }
            return status;
        }
        i = i.wrapping_add(1);
    }
    status
}

/// `fn_2c93f0` frame slots as ARM SP-relative offsets. The ARM keeps
/// `push {r0-r3, r4-r8, sb, sl, fp, lr}` (0x34) plus `sub sp, #0x1c`, so the
/// body SP is the entry SP minus 0x50; the token home is the incoming
/// stack word at +0x54 (written by select, like the ARM), while `&start`
/// is only ever read, so the port keeps the value in a Rust local and
/// never needs the incoming word's address.
pub const G2P93F0_SUB: u32 = 0x50;
const F93_APPLIED: u32 = 0x0c;
const F93_TOKEN: u32 = 0x54;
const F93_KIND: u32 = 0x10;
const F93_COUNT: u32 = 0x14;
const F93_ENABLED: u32 = 0x18;

/// `fn_2c8554` recursion levels as ARM SP-relative frames. The ARM keeps
/// `push {r4-r8, sb, sl, fp, lr}` (0x24) plus `sub sp, #0x134` per call, so
/// level N's body SP is its entry SP minus 0x158, and each deeper level
/// hangs 0x158 below the previous body. No fixed window and no depth cap:
/// the caller passes the level-0 entry SP (the `fn_2c93f0` body SP) and
/// each level is computed arithmetically, exactly like the ARM's own
/// stack. Slot offsets below hang off each level's body SP; `buf` spans
/// +0x14..+0x114 like the ARM's (writes past +0x84 would clobber the
/// later slots on the ARM too, so the layout matches by construction).
pub const G2P85_SUB: u32 = 0x158;
const R_BUF: u32 = 0x14;
const R_TABLE: u32 = 0x120;
const R_FLAG: u32 = 0x10;
const R_DELIM: u32 = 0x118;
const R_G: u32 = 0x98;
const R_GLEN: u32 = 0x9c;
const R_C1: u32 = 0xb8;
const R_C2: u32 = 0xbc;

const CAT93: u32 = 0x8f9532;
const SKIP_FMT: u32 = 0x2c9724;
const E93_S: u32 = 0x2c93c0;
const FMT_CHK: u32 = 0x2c8878;
const LIT_FECFG: u32 = 0x2c8884;
const FMT_SUCCEEDS: u32 = 0x2c888c;
const FMT_FAILS: u32 = 0x2c889c;
const LIT_MDE: u32 = 0x2c88a8;
const LIT_USCORE: u32 = 0x2c8874;
const E93_2: u32 = 0x2c8870;
const LIT_PLUS: u32 = 0x2c8ca4;
const LIT_SPACE: u32 = 0x2c8ca8;
const LIT_ONE: u32 = 0x2c8cac;
const LIT_YES: u32 = 0x2c8cb0;
const LIT_ONEWORD: u32 = 0x2c8cb4;
const FMT85_L1: u32 = 0x2c8cc0;
const FMT85_O1: u32 = 0x2c910c;
const LIT_G2P: u32 = 0x2c9118;
const FMT85_VALID: u32 = 0x2c911c;
const ZERO93: u32 = 0x2c9130;
const E93_3: u32 = 0x2c9134;
const E93_4: u32 = 0x2c8cbc;
const FMT85_DOMAIN: u32 = 0x2c93a4;
const FMT85_USAGE: u32 = 0x2c93c4;
const STAR93: u32 = 0x2c8868;
const LIT_MDEU: u32 = 0x2c886c;

/// The `fn_2c84ac` heap grower, live over the caller's arena.
///
/// Ports `src/fn_2c84ac.c:12`: when `n` exceeds 0x80 the three buffers at
/// `p+0x58/0x5c/0x60` are reallocated to `n` through the live heap
/// (`0x1963e0` over the allocator at `ctx+4`). A null grow logs `0x84d0`
/// and reports `0x89b0200a`; otherwise the status is 0.
pub fn fn_2c84ac_m(m: &mut Memory<'_>, h: &mut impl Host, p: u32, n: u32) -> u32 {
    if n <= 0x80 {
        return 0;
    }
    let ctx = g_word(m, p).unwrap_or(0);
    let allocator = g_word(m, ctx.wrapping_add(4)).unwrap_or(0);
    let logger = g_word(m, ctx.wrapping_add(16)).unwrap_or(0);
    for off in [0x58u32, 0x5cu32, 0x60u32] {
        let old = g_word(m, p.wrapping_add(off)).unwrap_or(0);
        let grown = h.call_m(0x1963e0, &[allocator, old, n], m);
        if grown == 0 {
            h.call_m(0x173dc0, &[logger, CAT93, 0x84d0, 0], m);
            return FAIL_MEM;
        }
        if !p_word(m, p.wrapping_add(off), grown) {
            return FAIL_MEM;
        }
    }
    0
}

/// The `fn_2c8554` MDE/g2p row engine, live over the caller's arena.
///
/// Ports `src/fn_2c84ac.c:76` 1:1. `recsp` is the ARM entry SP of level 0
/// (the `fn_2c93f0` body SP); each level's frame hangs 0x158 below the
/// previous body SP, exactly like the ARM's own nesting, with no depth
/// cap. `applied_ptr` is the caller's `u8` cell the engine
/// sets once a row applies. `word`/`opts` are guest string addresses,
/// `limit`/`start` plain words. Every guest call is issued through the
/// host except the certified leaf lookup `fn_1ea4e0_m`, which nests
/// directly the way `component_live_b3::fn_1ec7cc_m` nests it. Returns the
/// last callee status, or `0x89b02007` on a null result word.
#[allow(clippy::too_many_arguments)]
pub fn fn_2c8554_m(
    m: &mut Memory<'_>,
    h: &mut impl Host,
    recsp: u32,
    level: u32,
    p: u32,
    word: u32,
    opts: u32,
    limit: u32,
    applied_ptr: u32,
    start: u32,
) -> u32 {
    // This level's body SP: the level-0 entry SP minus one 0x24 push plus
    // 0x134 frame per level, exactly like the ARM's own nesting. No depth
    // cap: the ARM has none either (its bound is the stack guard), and the
    // guest range check below fails closed if the arithmetic ever leaves
    // the arena.
    let base = recsp.wrapping_sub(level.wrapping_add(1).wrapping_mul(G2P85_SUB));
    let buf = base.wrapping_add(R_BUF);
    let a_table = base.wrapping_add(R_TABLE);
    let a_flag = base.wrapping_add(R_FLAG);
    let a_delim = base.wrapping_add(R_DELIM);
    let a_g = base.wrapping_add(R_G);
    let a_glen = base.wrapping_add(R_GLEN);
    let a_c1 = base.wrapping_add(R_C1);
    let a_c2 = base.wrapping_add(R_C2);
    if !g_range(m, buf, 256)
        || !g_range(m, a_table, 4)
        || !g_range(m, a_flag, 2)
        || !g_range(m, a_delim, 1)
        || !g_range(m, a_g, 4)
        || !g_range(m, a_glen, 2)
        || !g_range(m, a_c1, 4)
        || !g_range(m, a_c2, 4)
    {
        return FAIL_MEM;
    }

    let ctx = g_word(m, p).unwrap_or(0);
    let f10 = g_word(m, p.wrapping_add(0x10)).unwrap_or(0);
    let f14 = g_word(m, p.wrapping_add(0x14)).unwrap_or(0);
    let vt = g_word(m, p.wrapping_add(0x18)).unwrap_or(0);
    let f_fec = g_word(m, vt.wrapping_add(0x1c)).unwrap_or(0);
    let f_g2p = g_word(m, vt.wrapping_add(0x48)).unwrap_or(0);
    let vt2 = g_word(m, p.wrapping_add(0x24)).unwrap_or(0);
    let f_m36 = g_word(m, vt2.wrapping_add(0x24)).unwrap_or(0);
    let f_m40 = g_word(m, vt2.wrapping_add(0x28)).unwrap_or(0);
    let f_m44 = g_word(m, vt2.wrapping_add(0x2c)).unwrap_or(0);
    let logger = g_word(m, ctx.wrapping_add(0x10)).unwrap_or(0);
    let f58 = g_word(m, p.wrapping_add(0x58)).unwrap_or(0);
    let f5c = g_word(m, p.wrapping_add(0x5c)).unwrap_or(0);
    let f60 = g_word(m, p.wrapping_add(0x60)).unwrap_or(0);

    let mut rc = 0u32;
    let mut kind = 0u32;
    // The row loop index and the pad-loop counter below are 16-bit on the
    // ARM (`uxth`); the counts and the `rem` window stay 32-bit.
    if h.call_m(0x195d90, &[word], m) == limit {
        h.call_m(0x195cd0, &[buf, LIT_MDEU], m);
        h.call_m(0x195c90, &[buf, opts], m);
        h.call_m(0x195c90, &[buf, LIT_USCORE], m);
        let mut n = h.call_m(0x195d90, &[buf], m) as u16;
        let mut k = 0u16;
        while u32::from(k) < h.call_m(0x195d90, &[f5c], m) && u32::from(n) < 256 {
            let ch = g_byte(m, f5c.wrapping_add(u32::from(k))).unwrap_or(0);
            if ch != b' ' {
                if !p_byte(m, buf.wrapping_add(u32::from(n)), ch) {
                    return FAIL_MEM;
                }
                n = n.wrapping_add(1);
            }
            k = k.wrapping_add(1);
        }
        if !p_byte(m, buf.wrapping_add(u32::from(n)), 0) {
            return FAIL_MEM;
        }
        h.call_m(0x1738e8, &[logger, CAT93, 5, 0, FMT_CHK, buf], m);
        if !p_half(m, a_flag, 0xffff) {
            return FAIL_MEM;
        }
        rc = h.call_m(f_fec, &[f10, f14, LIT_FECFG, buf, a_table, a_flag, a_delim], m);
        if rc >= 0x80000000 {
            return rc;
        }
        if g_half(m, a_flag).unwrap_or(0) == 0 {
            h.call_m(0x195cd0, &[buf, LIT_MDEU], m);
            n = 4;
            k = 0;
            while u32::from(k) < h.call_m(0x195d90, &[f5c], m) && u32::from(n) < 256 {
                let ch = g_byte(m, f5c.wrapping_add(u32::from(k))).unwrap_or(0);
                if ch != b' ' {
                    if !p_byte(m, buf.wrapping_add(u32::from(n)), ch) {
                        return FAIL_MEM;
                    }
                    n = n.wrapping_add(1);
                }
                k = k.wrapping_add(1);
            }
            if !p_byte(m, buf.wrapping_add(u32::from(n)), 0) {
                return FAIL_MEM;
            }
            h.call_m(0x1738e8, &[logger, CAT93, 5, 0, FMT_CHK, buf], m);
            if !p_half(m, a_flag, 0xffff) {
                return FAIL_MEM;
            }
            rc = h.call_m(f_fec, &[f10, f14, LIT_FECFG, buf, a_table, a_flag, a_delim], m);
            if rc >= 0x80000000 {
                return rc;
            }
            if g_half(m, a_flag).unwrap_or(0) == 0 {
                h.call_m(0x1738e8, &[logger, CAT93, 5, 0, FMT_FAILS], m);
            } else {
                if !p_byte(m, applied_ptr, 1) {
                    return FAIL_MEM;
                }
                h.call_m(0x1738e8, &[logger, CAT93, 5, 0, FMT_SUCCEEDS], m);
            }
        } else {
            if !p_byte(m, applied_ptr, 1) {
                return FAIL_MEM;
            }
            h.call_m(0x1738e8, &[logger, CAT93, 5, 0, FMT_SUCCEEDS], m);
        }
    }
    // `after`: the row engine runs only when the generation differs.
    if g_byte(m, p.wrapping_add(0x6d)).unwrap_or(0) == start as u8 {
        return rc;
    }
    let start = start.wrapping_add(1) & 0xff;
    let mut i = h.call_m(0x195d90, &[word], m) as u16;
    let base_w = word.wrapping_add(limit);
    loop {
        if !(u32::from(i) > limit) {
            return rc;
        }
        if g_byte(m, applied_ptr).unwrap_or(0) != 0 {
            return rc;
        }
        let save = g_byte(m, word.wrapping_add(u32::from(i))).unwrap_or(0);
        if !p_byte(m, word.wrapping_add(u32::from(i)), 0) {
            return FAIL_MEM;
        }
        if !p_half(m, a_flag, 0) {
            return FAIL_MEM;
        }
        rc = h.call_m(f_fec, &[f10, f14, LIT_MDE, base_w, a_table, a_flag, a_delim], m);
        if rc >= 0x80000000 {
            return rc;
        }
        if !p_byte(m, word.wrapping_add(u32::from(i)), save) {
            return FAIL_MEM;
        }
        let rem = u32::from(i).wrapping_sub(limit);
        let rem2 = rem.wrapping_sub(1);
        let m_count = rem2 as u16;
        let mut k = 0u16;
        let flag = g_half(m, a_flag).unwrap_or(0);
        while u32::from(k) < u32::from(flag) {
            if g_byte(m, applied_ptr).unwrap_or(0) != 0 {
                break;
            }
            let table = g_word(m, a_table).unwrap_or(0);
            let f1 = g_word(m, table.wrapping_add(u32::from(k) * 4)).unwrap_or(0);
            let mut f3 = 0u32;
            let mut f4 = 0u32;
            let delim = u32::from(g_byte(m, a_delim).unwrap_or(0));
            let mut q = h.call_m(0x195d30, &[f1, delim], m);
            if q == 0 {
                // nf1
            } else {
                kind = q.wrapping_add(1);
                if !p_byte(m, q, 0) {
                    return FAIL_MEM;
                }
                q = h.call_m(0x195d30, &[kind, delim], m);
                if q != 0 {
                    if !p_byte(m, q, 0) {
                        return FAIL_MEM;
                    }
                    f3 = q.wrapping_add(1);
                    q = h.call_m(0x195d30, &[f3, delim], m);
                    if q != 0 {
                        if !p_byte(m, q, 0) {
                            return FAIL_MEM;
                        }
                        f4 = q.wrapping_add(1);
                        q = h.call_m(0x195d30, &[f4, delim], m);
                        if q != 0 && !p_byte(m, q, 0) {
                            return FAIL_MEM;
                        }
                    }
                }
            }
            let n = h.call_m(0x195d90, &[f58], m) as u16;
            let f1v = if f1 == 0 || h.call_m(0x195d58, &[f1, E93_2], m) == 0 {
                STAR93
            } else {
                f1
            };
            let f4v = if f4 == 0 || h.call_m(0x195d58, &[f4, E93_2], m) == 0 {
                STAR93
            } else {
                f4
            };
            if n != 0 {
                h.call_m(0x195c90, &[f58, LIT_PLUS], m);
                h.call_m(0x195c90, &[f5c, LIT_SPACE], m);
                h.call_m(0x195c90, &[f60, LIT_SPACE], m);
            }
            if f3 == 0 || h.call_m(0x195d58, &[f3, E93_2], m) == 0 {
                // K: the affix/decode path.
                if kind != 0 && h.call_m(0x195d58, &[kind, LIT_YES], m) == 0 {
                    // M: the one-word MDE path.
                    h.call_m(0x195cd0, &[buf.wrapping_add(4), LIT_ONEWORD], m);
                    if !p_byte(m, word.wrapping_add(u32::from(i)), 0) {
                        return FAIL_MEM;
                    }
                    if !p_byte(m, a_c1, g_byte(m, f1v).unwrap_or(0))
                        || !p_byte(m, a_c1.wrapping_add(1), 0)
                        || !p_byte(m, a_c2, g_byte(m, f4v).unwrap_or(0))
                        || !p_byte(m, a_c2.wrapping_add(1), 0)
                    {
                        return FAIL_MEM;
                    }
                    h.call_m(0x1738e8, &[logger, CAT93, 5, 0, FMT85_L1, buf.wrapping_add(4), base_w], m);
                    let base_len = h.call_m(0x195d90, &[base_w], m);
                    rc = h.call_m(f_m36, &[f10, f14, 0, base_w, base_len], m);
                    if rc >= 0x80000000 {
                        return rc;
                    }
                    if !p_byte(m, word.wrapping_add(u32::from(i)), save) {
                        return FAIL_MEM;
                    }
                    rc = h.call_m(f_m44, &[f10, f14, buf.wrapping_add(4)], m);
                    if rc >= 0x80000000 {
                        return rc;
                    }
                    rc = h.call_m(f_m40, &[f10, f14, 0, a_g, a_glen], m);
                    if rc >= 0x80000000 {
                        return rc;
                    }
                    let mut jj = 0u16;
                    loop {
                        let glen = g_half(m, a_glen).unwrap_or(0);
                        if !(u32::from(jj) < u32::from(glen)) {
                            break;
                        }
                        let gptr = g_word(m, a_g).unwrap_or(0);
                        let gj = g_byte(m, gptr.wrapping_add(u32::from(jj))).unwrap_or(0);
                        if gj == 126 {
                            let mut kk = jj;
                            loop {
                                let glen2 = g_half(m, a_glen).unwrap_or(0);
                                if !(u32::from(kk) < u32::from(glen2)) {
                                    break;
                                }
                                let nxt = g_byte(m, gptr.wrapping_add(u32::from(kk) + 1)).unwrap_or(0);
                                if !p_byte(m, gptr.wrapping_add(u32::from(kk)), nxt) {
                                    return FAIL_MEM;
                                }
                                kk = kk.wrapping_add(1);
                            }
                            let glen3 = g_half(m, a_glen).unwrap_or(0);
                            if !p_half(m, a_glen, glen3.wrapping_sub(1)) {
                                return FAIL_MEM;
                            }
                            jj = jj.wrapping_sub(1);
                        } else if gj == 0xad {
                            if !p_byte(m, gptr.wrapping_add(u32::from(jj)), 126) {
                                return FAIL_MEM;
                            }
                        }
                        jj = jj.wrapping_add(1);
                    }
                    let gptr = g_word(m, a_g).unwrap_or(0);
                    let glen = g_half(m, a_glen).unwrap_or(0);
                    if !p_byte(m, gptr.wrapping_add(u32::from(glen)), 0) {
                        return FAIL_MEM;
                    }
                    h.call_m(0x1738e8, &[logger, CAT93, 5, 0, FMT85_O1, buf.wrapping_add(4), gptr], m);
                    let grow = h.call_m(0x195d90, &[gptr], m).wrapping_add(u32::from(n)).wrapping_add(4);
                    rc = fn_2c84ac_m(m, h, p, grow);
                    if rc >= 0x80000000 {
                        return rc;
                    }
                    h.call_m(0x195c90, &[f58, LIT_ONE], m);
                    h.call_m(0x195c90, &[f5c, LIT_SPACE], m);
                    h.call_m(0x195c90, &[f60, LIT_SPACE], m);
                    h.call_m(0x195c90, &[f58, gptr], m);
                    h.call_m(0x195c90, &[f5c, a_c1], m);
                    h.call_m(0x195c90, &[f60, a_c2], m);
                    let mut j = 0u16;
                    let bound = h.call_m(0x195d90, &[gptr], m).wrapping_sub(1) & 0xffff;
                    if bound > 0 {
                        loop {
                            h.call_m(0x195c90, &[f5c, LIT_SPACE], m);
                            h.call_m(0x195c90, &[f60, LIT_SPACE], m);
                            j = j.wrapping_add(1);
                            if !(bound > u32::from(j)) {
                                break;
                            }
                        }
                    }
                } else {
                    let grow = u32::from(i)
                        .wrapping_add(u32::from(n))
                        .wrapping_sub(limit)
                        .wrapping_add(2);
                    rc = fn_2c84ac_m(m, h, p, grow);
                    if rc >= 0x80000000 {
                        return rc;
                    }
                    rc = h.call_m(0x195dac, &[f58, base_w, rem], m);
                    if rc >= 0x80000000 {
                        return rc;
                    }
                    h.call_m(0x195c90, &[f5c, f1v], m);
                    h.call_m(0x195c90, &[f60, f4v], m);
                    let mut j = 0u16;
                    if (rem2 as u16) > 0 {
                        loop {
                            h.call_m(0x195c90, &[f5c, LIT_SPACE], m);
                            h.call_m(0x195c90, &[f60, LIT_SPACE], m);
                            j = j.wrapping_add(1);
                            if !(u32::from(m_count) > u32::from(j)) {
                                break;
                            }
                        }
                    }
                }
            } else {
                let f3len = h.call_m(0x195d90, &[f3], m);
                let grow = f3len.wrapping_add(u32::from(n)).wrapping_add(4);
                rc = fn_2c84ac_m(m, h, p, grow);
                if rc >= 0x80000000 {
                    return rc;
                }
                h.call_m(0x195c90, &[f58, LIT_ONE], m);
                h.call_m(0x195c90, &[f5c, LIT_SPACE], m);
                h.call_m(0x195c90, &[f60, LIT_SPACE], m);
                h.call_m(0x195c90, &[f58, f3], m);
                h.call_m(0x195c90, &[f5c, f1v], m);
                h.call_m(0x195c90, &[f60, f4v], m);
                let mut j = 0u16;
                let bound = f3len.wrapping_sub(1) & 0xffff;
                if bound > 0 {
                    loop {
                        h.call_m(0x195c90, &[f5c, LIT_SPACE], m);
                        h.call_m(0x195c90, &[f60, LIT_SPACE], m);
                        j = j.wrapping_add(1);
                        if !(bound > u32::from(j)) {
                            break;
                        }
                    }
                }
            }
            // call:
            rc = fn_2c8554_m(m, h, recsp, level.wrapping_add(1), p, word, opts, u32::from(i), applied_ptr, start);
            if rc >= 0x80000000 {
                return rc;
            }
            if g_byte(m, applied_ptr).unwrap_or(0) == 0 {
                if !p_byte(m, f58.wrapping_add(u32::from(n)), 0)
                    || !p_byte(m, f5c.wrapping_add(u32::from(n)), 0)
                    || !p_byte(m, f60.wrapping_add(u32::from(n)), 0)
                {
                    return FAIL_MEM;
                }
            }
            k = k.wrapping_add(1);
        }
        // `goto cont` with `applied` set always reaches `return rc`: the
        // flag is never cleared, so the next head check returns it.
        if g_byte(m, applied_ptr).unwrap_or(0) != 0 {
            return rc;
        }
        let save = g_byte(m, word.wrapping_add(u32::from(i))).unwrap_or(0);
        if !p_byte(m, word.wrapping_add(u32::from(i)), 0) {
            return FAIL_MEM;
        }
        if !p_half(m, a_flag, 0) {
            return FAIL_MEM;
        }
        let f172 = u32::from(g_byte(m, p.wrapping_add(0x172)).unwrap_or(0));
        rc = h.call_m(f_g2p, &[f10, f14, LIT_G2P, base_w, a_table, a_flag, a_delim, f172], m);
        if rc >= 0x80000000 {
            return rc;
        }
        if !p_byte(m, word.wrapping_add(u32::from(i)), save) {
            return FAIL_MEM;
        }
        let mut k = 0u16;
        let flag = g_half(m, a_flag).unwrap_or(0);
        while u32::from(k) < u32::from(flag) {
            if g_byte(m, applied_ptr).unwrap_or(0) != 0 {
                break;
            }
            let table = g_word(m, a_table).unwrap_or(0);
            let f1 = g_word(m, table.wrapping_add(u32::from(k) * 4)).unwrap_or(0);
            let mut f2 = 0u32;
            let mut f3 = 0u32;
            let mut f4 = 0u32;
            let mut f5 = 0u32;
            let mut f6 = 0u32;
            let delim = u32::from(g_byte(m, a_delim).unwrap_or(0));
            let mut q = h.call_m(0x195d30, &[f1, delim], m);
            if q == 0 {
                // nf2
            } else {
                if !p_byte(m, q, 0) {
                    return FAIL_MEM;
                }
                q = h.call_m(0x195d30, &[q.wrapping_add(1), delim], m);
                if q == 0 {
                    // nf2
                } else {
                    if !p_byte(m, q, 0) {
                        return FAIL_MEM;
                    }
                    f2 = q.wrapping_add(1);
                    q = h.call_m(0x195d30, &[f2, delim], m);
                    if q == 0 {
                        // nf2
                    } else {
                        if !p_byte(m, q, 0) {
                            return FAIL_MEM;
                        }
                        f3 = q.wrapping_add(1);
                        q = h.call_m(0x195d30, &[f3, delim], m);
                        if q != 0 {
                            f4 = q.wrapping_add(1);
                            if !p_byte(m, q, 0) {
                                return FAIL_MEM;
                            }
                            q = h.call_m(0x195d30, &[f4, delim], m);
                            if q == 0 {
                                // nf2
                            } else {
                                if !p_byte(m, q, 0) {
                                    return FAIL_MEM;
                                }
                                f5 = q.wrapping_add(1);
                                q = h.call_m(0x195d30, &[f5, delim], m);
                                if q == 0 {
                                    // nf2
                                } else {
                                    if !p_byte(m, q, 0) {
                                        return FAIL_MEM;
                                    }
                                    f6 = q.wrapping_add(1);
                                    q = h.call_m(0x195d30, &[f6, delim], m);
                                    if q != 0 && !p_byte(m, q, 0) {
                                        return FAIL_MEM;
                                    }
                                }
                            }
                        }
                    }
                }
            }
            if f1 == 0 {
                h.call_m(0x1738e8, &[logger, CAT93, 0x84d1, 0], m);
                return FAIL_HANDLE;
            }
            h.call_m(0x1738e8, &[logger, CAT93, 5, 0, FMT85_VALID, f1], m);
            if f2 != 0 && h.call_m(0x195d58, &[f2, E93_4], m) != 0 {
                if h.call_m(0x1960a8, &[f2, opts], m) == 0 {
                    h.call_m(0x1738e8, &[logger, CAT93, 5, 0, FMT85_DOMAIN], m);
                    k = k.wrapping_add(1);
                    continue;
                }
            }
            if f6 != 0 && h.call_m(0x195d58, &[f6, ZERO93], m) == 0 {
                h.call_m(0x1738e8, &[logger, CAT93, 5, 0, FMT85_USAGE], m);
                k = k.wrapping_add(1);
                continue;
            }
            let n = h.call_m(0x195d90, &[f58], m) as u16;
            if h.call_m(0x195d58, &[f1, E93_3], m) == 0 {
                // call2:
                rc = fn_2c8554_m(m, h, recsp, level.wrapping_add(1), p, word, opts, u32::from(i), applied_ptr, start);
                if rc >= 0x80000000 {
                    return rc;
                }
                if g_byte(m, applied_ptr).unwrap_or(0) == 0 {
                    if !p_byte(m, f58.wrapping_add(u32::from(n)), 0)
                        || !p_byte(m, f5c.wrapping_add(u32::from(n)), 0)
                        || !p_byte(m, f60.wrapping_add(u32::from(n)), 0)
                    {
                        return FAIL_MEM;
                    }
                }
                k = k.wrapping_add(1);
                continue;
            }
            if f3 == 0 || h.call_m(0x195d58, &[f3, E93_3], m) == 0 {
                f3 = STAR93;
            }
            let f4v = if f4 != 0 && h.call_m(0x195d58, &[f4, E93_3], m) != 0 {
                f4
            } else {
                STAR93
            };
            let grow = h.call_m(0x195d90, &[f1], m).wrapping_add(u32::from(n)).wrapping_add(4);
            rc = fn_2c84ac_m(m, h, p, grow);
            if rc >= 0x80000000 {
                return rc;
            }
            if n != 0 {
                h.call_m(0x195c90, &[f58, LIT_PLUS], m);
                h.call_m(0x195c90, &[f5c, LIT_SPACE], m);
                h.call_m(0x195c90, &[f60, LIT_SPACE], m);
            }
            h.call_m(0x195c90, &[f58, LIT_ONE], m);
            h.call_m(0x195c90, &[f5c, LIT_SPACE], m);
            h.call_m(0x195c90, &[f60, LIT_SPACE], m);
            h.call_m(0x195c90, &[f58, f1], m);
            h.call_m(0x195c90, &[f5c, f3], m);
            h.call_m(0x195c90, &[f60, f4v], m);
            let mut j = 0u16;
            let bound = h.call_m(0x195d90, &[f1], m).wrapping_sub(1) & 0xffff;
            if bound > 0 {
                loop {
                    h.call_m(0x195c90, &[f5c, LIT_SPACE], m);
                    h.call_m(0x195c90, &[f60, LIT_SPACE], m);
                    j = j.wrapping_add(1);
                    if !(bound > u32::from(j)) {
                        break;
                    }
                }
            }
            h.call_m(0x195c90, &[f58, LIT_ONE], m);
            h.call_m(0x195c90, &[f5c, LIT_SPACE], m);
            h.call_m(0x195c90, &[f60, LIT_SPACE], m);
            // call2:
            rc = fn_2c8554_m(m, h, recsp, level.wrapping_add(1), p, word, opts, u32::from(i), applied_ptr, start);
            if rc >= 0x80000000 {
                return rc;
            }
            if g_byte(m, applied_ptr).unwrap_or(0) == 0 {
                if !p_byte(m, f58.wrapping_add(u32::from(n)), 0)
                    || !p_byte(m, f5c.wrapping_add(u32::from(n)), 0)
                    || !p_byte(m, f60.wrapping_add(u32::from(n)), 0)
                {
                    return FAIL_MEM;
                }
            }
            k = k.wrapping_add(1);
        }
        i = i.wrapping_sub(1);
    }
}

/// The `fn_2c93f0` MDE-config fallback, live over the caller's arena.
///
/// Ports `src/fn_2c93f0.c:32` 1:1. `p` is the word-engine object (the same
/// `fe` the G2P lookup takes): `+0x00` the services, `+0x0c` the token
/// vtable (`+0x2c` select, `+0x50` set), `+0x58/0x5c/0x60` the text,
/// alternate and extra buffers, `+0x6a/0x6b/0x6c/0x6e` the length limits,
/// capital-only and disabled flags. `start`/`token_in` are the `u16` stack
/// words the ARM receives; `end` is the `u16 *` the ARM fills.
/// Level 0 of the nested `fn_2c8554` recursion enters at this body SP.
/// Returns the last callee status,
/// or `0x89b02007` on a null result word.
#[allow(clippy::too_many_arguments)]
pub fn fn_2c93f0_m(
    m: &mut Memory<'_>,
    h: &mut impl Host,
    sp: u32,
    p: u32,
    word: u32,
    text: u32,
    opts: u32,
    start: u16,
    token_in: u16,
    handle_obj: u32,
    handle_data: u32,
    end: u32,
) -> u32 {
    // The ARM body SP: entry SP minus the 0x34 push minus the 0x1c
    // frame. The token home is the incoming stack word, written by
    // select exactly like the ARM; `&start` is only ever read, so the
    // port keeps the value in a Rust local.
    let body = sp.wrapping_sub(G2P93F0_SUB);
    let a_applied = body.wrapping_add(F93_APPLIED);
    let a_token = body.wrapping_add(F93_TOKEN);
    let a_kind = body.wrapping_add(F93_KIND);
    let a_count = body.wrapping_add(F93_COUNT);
    let a_enabled = body.wrapping_add(F93_ENABLED);
    if !g_range(m, a_applied, 1)
        || !g_range(m, a_token, 2)
        || !g_range(m, a_kind, 1)
        || !g_range(m, a_count, 2)
        || !g_range(m, a_enabled, 2)
        || !p_byte(m, a_applied, 0)
        || !p_half(m, a_token, token_in)
        || !p_byte(m, a_kind, 8)
        || !p_half(m, a_count, 0)
        || !p_half(m, a_enabled, 1)
    {
        return FAIL_MEM;
    }

    let services = g_word(m, p).unwrap_or(0);
    let methods = g_word(m, p.wrapping_add(0x0c)).unwrap_or(0);
    let logger = g_word(m, services.wrapping_add(0x10)).unwrap_or(0);
    let f_select = g_word(m, methods.wrapping_add(0x2c)).unwrap_or(0);
    let f_set = g_word(m, methods.wrapping_add(0x50)).unwrap_or(0);
    let text_buf = g_word(m, p.wrapping_add(0x58)).unwrap_or(0);
    let alt_buf = g_word(m, p.wrapping_add(0x5c)).unwrap_or(0);
    let extra_buf = g_word(m, p.wrapping_add(0x60)).unwrap_or(0);

    let mut rc = 0u32;
    let skip = g_byte(m, p.wrapping_add(0x6e)).unwrap_or(0) == 1
        || h.call_m(0x195d90, &[text], m) < u32::from(g_byte(m, p.wrapping_add(0x6a)).unwrap_or(0))
        || h.call_m(0x195d90, &[text], m) > u32::from(g_byte(m, p.wrapping_add(0x6b)).unwrap_or(0));
    if skip {
        h.call_m(0x1738e8, &[logger, CAT93, 5, 0, SKIP_FMT, word], m);
        return rc;
    }
    if g_byte(m, p.wrapping_add(0x6c)).unwrap_or(0) == 1 {
        let c = g_byte(m, text).unwrap_or(0);
        let upper = (c.wrapping_sub(b'A') as u32) <= 25;
        let listed = matches!(c, 0x80 | 0x8e | 0x8f | 0x90 | 0x92 | 0x99 | 0x9a | 0xa5);
        if !upper && !listed {
            h.call_m(0x1738e8, &[logger, CAT93, 5, 0, SKIP_FMT, word], m);
            return rc;
        }
    }
    h.call_m(0x195cd0, &[text_buf, E93_S], m);
    h.call_m(0x195cd0, &[alt_buf, E93_S], m);
    h.call_m(0x195cd0, &[extra_buf, E93_S], m);
    rc = fn_2c8554_m(m, h, body, 0, p, word, opts, 0, a_applied, 0);
    if rc >= 0x80000000 || g_byte(m, a_applied).unwrap_or(0) == 0 {
        return rc;
    }
    let end_len = h.call_m(0x195d90, &[text], m);
    if !p_half(m, end, (end_len as u16).wrapping_add(start)) {
        return FAIL_MEM;
    }
    let token = g_half(m, a_token).unwrap_or(0);
    rc = h.call_m(f_select, &[handle_obj, handle_data, u32::from(token), a_token], m);
    if rc >= 0x80000000 {
        return rc;
    }
    let token = g_half(m, a_token).unwrap_or(0);
    rc = h.call_m(
        f_set,
        &[handle_obj, handle_data, u32::from(token), 0, 1, a_kind, a_count],
        m,
    );
    if rc >= 0x80000000 {
        return rc;
    }
    // The ARM caller pushes the stack words; `&start` is the incoming
    // slot, materialized here exactly like the ARM's own call setup.
    if !p_half(m, body.wrapping_add(0x50), start) {
        return FAIL_MEM;
    }
    rc = h.call_m(
        f_set,
        &[handle_obj, handle_data, u32::from(token), 1, 1, body.wrapping_add(0x50), a_count],
        m,
    );
    if rc >= 0x80000000 {
        return rc;
    }
    rc = h.call_m(
        f_set,
        &[handle_obj, handle_data, u32::from(token), 2, 1, end, a_count],
        m,
    );
    if rc >= 0x80000000 {
        return rc;
    }
    let tlen = h.call_m(0x195d90, &[text_buf], m);
    rc = h.call_m(
        f_set,
        &[
            handle_obj,
            handle_data,
            u32::from(token),
            3,
            tlen.wrapping_add(1) & 0xffff,
            text_buf,
            a_count,
        ],
        m,
    );
    if rc >= 0x80000000 {
        return rc;
    }
    let alen = h.call_m(0x195d90, &[alt_buf], m);
    rc = h.call_m(
        f_set,
        &[
            handle_obj,
            handle_data,
            u32::from(token),
            5,
            alen.wrapping_add(1) & 0xffff,
            alt_buf,
            a_count,
        ],
        m,
    );
    if rc >= 0x80000000 {
        return rc;
    }
    let elen = h.call_m(0x195d90, &[extra_buf], m);
    rc = h.call_m(
        f_set,
        &[
            handle_obj,
            handle_data,
            u32::from(token),
            6,
            elen.wrapping_add(1) & 0xffff,
            extra_buf,
            a_count,
        ],
        m,
    );
    if rc >= 0x80000000 {
        return rc;
    }
    rc = h.call_m(
        f_set,
        &[handle_obj, handle_data, u32::from(token), 7, 1, a_enabled, a_count],
        m,
    );
    if rc >= 0x80000000 {
        return rc;
    }
    h.call_m(
        f_set,
        &[handle_obj, handle_data, u32::from(token), 9, 1, E93_S, a_count],
        m,
    )
}
