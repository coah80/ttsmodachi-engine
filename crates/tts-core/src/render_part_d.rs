//! Render-path command and resource helpers. Pointers are caller-arena offsets.
use crate::normalization::{self as n, NormalizationHost};

fn word(m: &[u8], p: u32) -> u32 {
    u32::from_le_bytes(m[p as usize..p as usize + 4].try_into().unwrap())
}
fn half(m: &[u8], p: u32) -> u16 {
    u16::from_le_bytes(m[p as usize..p as usize + 2].try_into().unwrap())
}
fn put(m: &mut [u8], p: u32, v: u32) {
    m[p as usize..p as usize + 4].copy_from_slice(&v.to_le_bytes());
}
fn put_half(m: &mut [u8], p: u32, v: u16) {
    m[p as usize..p as usize + 2].copy_from_slice(&v.to_le_bytes());
}

/// fn_2ffb64. RVCT unbounded byte printf output callback.
pub fn write_byte(m: &mut [u8], value: u32, state: u32) -> u32 {
    let dst = word(m, state);
    m[dst as usize] = value as u8;
    put(m, state, dst.wrapping_add(1));
    value
}

/// fn_2ffba0. RVCT bounded wide printf output callback. The bound is
/// checked before the two-byte write, just as in the ROM.
pub fn write_wide_bounded(m: &mut [u8], value: u32, state: u32) -> u32 {
    let dst = word(m, state);
    if dst < word(m, state + 4) {
        put(m, state, dst.wrapping_add(2));
        put_half(m, dst, value as u16);
    }
    value
}

/// fn_2e66cc, EU fn_2e6888, KR fn_2e6934. Adjust the caller's byte end
/// after trimming a pattern. `scratch` is a disjoint caller-owned two-byte
/// temporary, restored before return. `separators` maps D_9e0990.
#[allow(clippy::too_many_arguments)]
pub fn trim_end(
    m: &mut [u8],
    pattern: u32,
    text: u32,
    offset: u32,
    position: u32,
    end: u32,
    scratch: u32,
    separators: &[u8],
) {
    if text == 0 || end == 0 || word(m, end) <= position {
        return;
    }
    let saved = half(m, scratch);
    let count = word(m, end).wrapping_sub(position) as u16 / 2;
    put_half(m, scratch, count);
    n::trim_pattern(m, pattern, text.wrapping_add(offset), scratch, separators);
    let count = half(m, scratch);
    put_half(m, scratch, saved);
    if count != 0 {
        put(
            m,
            end,
            position.wrapping_add(count as u32 * 2).wrapping_add(2),
        );
    }
}

/// fn_2ef3c4, EU fn_2ef580, KR fn_2ef62c. Find the next command backslash,
/// respecting escaped characters and the command's quote rules.
pub fn command_end(m: &mut [u8], text: u32, end: u32, start: u32, out: u32, mode: u32) -> u32 {
    let limit = end >> 1;
    let mut i = (start >> 1) as u16;
    let mut in_quote = false;
    let mut closed = false;
    while (i as u32) < limit {
        let c = half(m, text + i as u32 * 2);
        if c == 0 || c == 27 {
            break;
        }
        let escaped = c == 92
            && i as u32 + 1 < limit
            && matches!(half(m, text + (i as u32 + 1) * 2), 92 | 34);
        if c == 92 {
            let advance = if escaped {
                mode == 29 || ((mode == 26 || mode == 7) && in_quote)
            } else {
                matches!(mode, 29 | 26 | 7) && in_quote
            };
            if !advance {
                put(m, out, i as u32 * 2);
                return 1;
            }
        }
        if escaped {
            i = i.wrapping_add(1);
        } else if matches!(mode, 26 | 7) && c == 34 {
            if in_quote {
                in_quote = false;
                closed = true;
            } else if !closed {
                in_quote = true;
            } else {
                return 0;
            }
        }
        i = i.wrapping_add(1);
    }
    0
}

/// fn_2ed010, EU fn_2ed1cc, KR fn_2ed278. Copy and unescape a command value.
#[allow(clippy::too_many_arguments)]
pub fn copy_value(
    m: &mut [u8],
    ctx: u32,
    message: u32,
    src: u32,
    start: u32,
    end: u32,
    done: u32,
    flag: u32,
    host: &mut impl NormalizationHost,
) -> u32 {
    let bytes = end.wrapping_sub(start);
    let mut chars = (bytes >> 1) as u16;
    let heap = word(m, word(m, ctx) + 4);
    let Some(buf) = host.calloc(m, heap, 1, (bytes / 2 + 1).wrapping_mul(2)) else {
        return 0x8000200a;
    };
    n::memcpy_at(m, buf, src.wrapping_add(start), bytes);
    let mut i = 0u16;
    while i < chars {
        if i != 0
            && half(m, buf + (i as u32 - 1) * 2) == 92
            && matches!(half(m, buf + i as u32 * 2), 92 | 34)
        {
            n::memmove_at(
                m,
                buf + (i as u32 - 1) * 2,
                buf + i as u32 * 2,
                (chars - i) as u32 * 2,
            );
            put_half(m, buf + (chars as u32 - 1) * 2, 0);
            chars -= 1;
        }
        i = i.wrapping_add(1);
    }
    let rec = word(m, message + 4);
    if half(m, rec + 10) == 26 || flag == 1 {
        if half(m, buf) == 34 {
            let len = n::u16_len(m, buf);
            if half(m, buf + len.wrapping_sub(1).wrapping_mul(2)) == 34 && len != 1 {
                let len = len as u16;
                n::memmove_at(m, buf, buf + 2, (len as u32 * 2).wrapping_sub(2));
                put_half(m, buf + (len as u32).wrapping_sub(2).wrapping_mul(2), 0);
            }
        }
        if flag == 1 {
            put(m, rec + 32, buf);
            m[done as usize] = 1;
            return 0;
        }
    }
    put(m, rec + 28, buf);
    m[done as usize] = 1;
    0
}

/// fn_2ed188, EU fn_2ed344, KR fn_2ed3f0. Resolve a default string.
/// `fallback` is the mapped, terminated UTF-16 D_a093e6 constant.
pub fn default_value(
    m: &mut [u8],
    ctx: u32,
    message: u32,
    found: u32,
    fallback: u32,
    host: &mut impl NormalizationHost,
) -> u32 {
    m[found as usize] = 0;
    let count = half(m, ctx + 64);
    let rec = word(m, message + 4);
    let key = half(m, rec + 10);
    let mut selected = None;
    if count != 0 {
        'outer: for i in 0..count as u32 {
            let item = word(m, ctx + 68) + i * 12;
            for j in 0..half(m, item + 4) as u32 {
                if half(m, word(m, item + 8) + j * 2) == key {
                    selected = Some(word(m, item));
                    break 'outer;
                }
            }
        }
    } else if matches!(key, 12 | 21 | 36) {
        selected = Some(fallback);
    }
    let heap = word(m, word(m, ctx) + 4);
    if let Some(src) = selected {
        let size = (n::u16_len(m, src) + 1).wrapping_mul(2);
        let buf = host.calloc(m, heap, 1, size).unwrap_or(0);
        put(m, rec + 28, buf);
        if buf == 0 {
            return 0x8000200a;
        }
        n::u16_copy(m, buf, src);
        m[found as usize] = 1;
    } else {
        let node = word(m, rec + 28);
        if node != 0 {
            host.free(m, heap, node);
            put(m, rec + 28, 0);
        }
        put_half(m, rec + 10, 0);
    }
    0
}

/// fn_2ed314, EU fn_2ed4d0, KR fn_2ed57c. Replace a matching choice
/// string with its numeric value, releasing the old allocation.
pub fn choice_value(
    m: &mut [u8],
    ctx: u32,
    message: u32,
    host: &mut impl NormalizationHost,
) -> u32 {
    let rec = word(m, message + 4);
    let mut i = 0;
    while i < half(m, ctx + 56) as u32 {
        let row = word(m, ctx + 60) + i * 8;
        if half(m, row) == half(m, rec + 10) {
            for j in 0..half(m, row + 2) as u32 {
                let item = word(m, row + 4) + j * 8;
                let node = word(m, rec + 28);
                if node != 0 && n::u16_cmp(m, word(m, item), node) == 0 {
                    host.free(m, word(m, word(m, ctx) + 4), node);
                    put(m, rec + 28, 0);
                    put(m, rec + 28, half(m, item + 4) as u32);
                    return 0;
                }
            }
            host.free(m, word(m, word(m, ctx) + 4), word(m, rec + 28));
            put(m, rec + 28, 0);
            put_half(m, rec + 10, 0);
        }
        i += 1;
    }
    0
}

/// fn_2ee264, EU fn_2ee420, KR fn_2ee4cc. Decode the string/choice table.
pub fn read_choices(m: &mut [u8], ctx: u32, src: u32, host: &mut impl NormalizationHost) -> u32 {
    put_half(m, ctx + 56, half(m, src));
    let heap = word(m, word(m, ctx) + 4);
    let arr = host
        .calloc(m, heap, 1, half(m, ctx + 56) as u32 * 8 + 1)
        .unwrap_or(0);
    put(m, ctx + 60, arr);
    if arr == 0 {
        put_half(m, ctx + 56, 0);
        return 0x8000200a;
    }
    let mut off = src + 2;
    for i in 0..half(m, ctx + 56) as u32 {
        let row = arr + i * 8;
        put_half(m, row, half(m, off));
        off += 2;
        put_half(m, row + 2, half(m, off));
        off += 2;
        let subs = host
            .calloc(m, heap, 1, half(m, row + 2) as u32 * 8 + 1)
            .unwrap_or(0);
        put(m, row + 4, subs);
        if subs == 0 {
            put_half(m, row + 2, 0);
            return 0x8000200a;
        }
        for j in 0..half(m, row + 2) as u32 {
            let item = subs + j * 8;
            let v = half(m, off) as u32;
            off += 2;
            let Some(tmp) = host.calloc(m, heap, 1, v * 2 + 1) else {
                return 0x8000200a;
            };
            n::memcpy_at(m, tmp, off, v);
            off += v;
            let str = host.calloc(m, heap, 1, v * 2 + 2).unwrap_or(0);
            put(m, item, str);
            if str == 0 {
                return 0x8000200a;
            }
            n::u8_to_u16_copy(m, str, tmp);
            host.free(m, heap, tmp);
            put_half(m, item + 4, half(m, off));
            off += 2;
        }
    }
    0
}

/// fn_2ee49c, EU fn_2ee658, KR fn_2ee704. Decode default strings and keys.
pub fn read_defaults(m: &mut [u8], ctx: u32, src: u32, host: &mut impl NormalizationHost) -> u32 {
    put_half(m, ctx + 64, half(m, src));
    let heap = word(m, word(m, ctx) + 4);
    let arr = host
        .calloc(m, heap, 1, half(m, ctx + 64) as u32 * 12 + 1)
        .unwrap_or(0);
    put(m, ctx + 68, arr);
    if arr == 0 {
        put_half(m, ctx + 64, 0);
        return 0x8000200a;
    }
    let mut off = src + 2;
    for i in 0..half(m, ctx + 64) as u32 {
        let row = arr + i * 12;
        let v = half(m, off) as u32;
        off += 2;
        let Some(tmp) = host.calloc(m, heap, 1, v + 1) else {
            return 0x8000200a;
        };
        n::memcpy_at(m, tmp, off, v);
        off += v;
        let str = host.calloc(m, heap, 1, v * 2 + 3).unwrap_or(0);
        put(m, row, str);
        if str == 0 {
            return 0x8000200a;
        }
        n::u8_to_u16_copy(m, str, tmp);
        host.free(m, heap, tmp);
        put_half(m, row + 4, half(m, off));
        off += 2;
        let vals = host
            .calloc(m, heap, 1, half(m, row + 4) as u32 * 4 + 1)
            .unwrap_or(0);
        put(m, row + 8, vals);
        if vals == 0 {
            put_half(m, row + 4, 0);
            return 0x8000200a;
        }
        for j in 0..half(m, row + 4) as u32 {
            put_half(m, vals + j * 2, half(m, off));
            off += 2;
        }
    }
    0
}

/// fn_2ef034, EU fn_2ef1f0, KR fn_2ef29c. Build the initial 40-record
/// value table. `scratch` names nine disjoint caller-owned bytes for the
/// temporary message and found flag, restored before return.
pub fn initial_values(
    m: &mut [u8],
    ctx: u32,
    out: u32,
    scratch: u32,
    fallback: u32,
    host: &mut impl NormalizationHost,
) -> u32 {
    if ctx == 0 {
        return 0x80002007;
    }
    let heap = word(m, word(m, ctx) + 4);
    let Some(table) = host.calloc(m, heap, 1, 0x641) else {
        return 0x8000200a;
    };
    let mut saved = [0u8; 9];
    saved.copy_from_slice(&m[scratch as usize..scratch as usize + 9]);
    m[(scratch + 8) as usize] = 0;
    for key in 1u16..40 {
        let rec = table + key as u32 * 40;
        put_half(m, rec + 10, key);
        put(m, scratch + 4, rec);
        m[rec as usize] = 1;
        put(m, rec + 4, 0);
        if matches!(key, 4 | 2 | 9 | 8 | 13) {
            put_half(m, rec + 10, 0);
        } else {
            default_value(m, ctx, scratch, scratch + 8, fallback, host);
            let found = m[(scratch + 8) as usize] != 0;
            if matches!(
                key,
                6 | 12 | 21 | 50 | 36 | 39 | 37 | 34 | 22 | 40 | 26 | 29
            ) {
                if !found {
                    put_half(m, rec + 10, 0);
                }
            } else if !found {
                put_half(m, rec + 10, key);
                let mut matched = false;
                for i in 0..half(m, ctx + 22) as u32 {
                    let option = word(m, ctx + 24) + i * 12;
                    if half(m, option + 6) == half(m, rec + 10) && m[(option + 4) as usize] == 1 {
                        let value = word(m, option + 8);
                        if value != 9999 {
                            put(m, rec + 28, value);
                            m[scratch as usize] = 1;
                            matched = true;
                        }
                        break;
                    }
                }
                if !matched {
                    m[scratch as usize] = 0;
                    put_half(m, rec + 10, 0);
                }
            } else if half(m, ctx + 56) != 0 {
                choice_value(m, ctx, scratch, host);
            }
        }
    }
    m[scratch as usize..scratch as usize + 9].copy_from_slice(&saved);
    put(m, out, table);
    0
}

/// Chunk resource callbacks. ResourceAdapter supplies the real fn_243d50.
pub trait ResourceHost: NormalizationHost {
    /// fn_243d50. Use ResourceAdapter for guest chunk records.
    fn release_chunk(&mut self, m: &mut [u8], record: u32) -> u32;
}

/// fn_2edbf0, EU fn_2eddac, KR fn_2ede58. Release all configured resources.
/// A chunk-release error retains the handle and skips the final state clear.
pub fn clear_resources(m: &mut [u8], ctx: u32, host: &mut impl ResourceHost) -> u32 {
    if m[(ctx + 20) as usize] != 1 {
        return 0;
    }
    let heap = word(m, word(m, ctx) + 4);
    for (count_off, ptr_off, stride, nested) in [
        (22, 24, 12, 0),
        (28, 32, 0, 0),
        (44, 48, 12, 8),
        (56, 60, 8, 4),
        (64, 68, 12, 0),
    ] {
        let count = half(m, ctx + count_off);
        if count != 0 {
            let arr = word(m, ctx + ptr_off);
            if stride != 0 {
                for i in 0..count as u32 {
                    let row = arr + i * stride;
                    if count_off == 56 {
                        let children = word(m, row + 4);
                        for j in 0..half(m, row + 2) as u32 {
                            let slot = children + j * 8;
                            let node = word(m, slot);
                            if node != 0 {
                                host.free(m, heap, node);
                            }
                            put(m, slot, 0);
                        }
                        if children != 0 {
                            host.free(m, heap, children);
                        }
                        put(m, row + 4, 0);
                    } else {
                        for off in [nested, if count_off == 64 { 8 } else { nested }] {
                            let node = word(m, row + off);
                            if node != 0 {
                                host.free(m, heap, node);
                            }
                            put(m, row + off, 0);
                            if count_off != 64 {
                                break;
                            }
                        }
                    }
                }
            }
            if arr != 0 {
                host.free(m, heap, arr);
            }
            put(m, ctx + ptr_off, 0);
            if count_off == 22 || count_off == 28 {
                put_half(m, ctx + count_off, 0);
            }
        }
        if count_off == 28 {
            let node = word(m, ctx + 40);
            if node != 0 {
                host.free(m, heap, node);
                put(m, ctx + 40, 0);
            }
        }
    }
    let handle = word(m, ctx + 8);
    let rc = if handle != 0 {
        let rc = host.release_chunk(m, handle);
        if rc >= 0x80000000 {
            return rc;
        }
        put(m, ctx + 8, 0);
        rc
    } else {
        0
    };
    n::memset_at(m, ctx + 20, 0, 52);
    rc
}

/// Indirect methods only. Context, heap, lock and registry dispatchers run
/// their existing Rust ports; callers route the concrete guest method address.
pub trait DictionaryHost {
    fn heap_method(&mut self, m: &mut [u8], method: u32, object: u32, args: &[u32]) -> u32;
    fn lock_method(&mut self, m: &mut [u8], method: u32, target: u32) -> u32;
    fn compare_key(&mut self, m: &[u8], method: u32, key: u32, stored: u32) -> i32;
    /// Adapter for fn_198f1c after the real fn_19672c lookup.
    fn remove_node(&mut self, m: &mut [u8], tree: u32, node: u32);
    /// Invoke an indirect key/value destructor from fn_198f1c.
    fn tree_callback(&mut self, _m: &mut [u8], _method: u32, _args: &[u32]) {
        panic!("missing tree destructor callback host");
    }
    fn release_callback(&mut self, m: &mut [u8], owner: [u32; 2], entry: &mut [u8; 32]);
    fn open_dictionary(
        &mut self,
        m: &mut [u8],
        method: u32,
        owner: [u32; 2],
        name: u32,
        flag: &mut u8,
    ) -> u32;
    fn match_dictionary(
        &mut self,
        m: &mut [u8],
        method: u32,
        owner: [u32; 2],
        name: u32,
        prefix: &[u8],
        result: &mut u32,
        length: &mut u16,
        flag: &mut u8,
    ) -> u32;
}

/// Registry name and a disjoint, writable word for the acquired-entry output.
/// The word is restored before return. The cached handle and default context
/// correspond to fn_19611c's immutable globals.
pub struct DictionaryEnvironment {
    pub registry_name: u32,
    pub temporary_word: u32,
    pub cached_handle: [u8; 8],
    pub default_context: u32,
}

struct DictionaryHeap<'a, H> {
    memory: &'a mut [u8],
    host: &'a mut H,
    method: u32,
}
impl<H: DictionaryHost> crate::frontend::HeapVtable for DictionaryHeap<'_, H> {
    fn alloc_at(&mut self, object: u32, size: u32) -> u32 {
        self.host
            .heap_method(self.memory, self.method, object, &[size])
    }
    fn realloc_at(&mut self, object: u32, old: u32, size: u32) -> u32 {
        self.host
            .heap_method(self.memory, self.method, object, &[old, size])
    }
    fn release_at(&mut self, object: u32, p: u32) {
        self.host
            .heap_method(self.memory, self.method, object, &[p]);
    }
}
fn dictionary_heap(
    m: &mut [u8],
    heap: u32,
    pointer: u32,
    size: u32,
    operation: u32,
    host: &mut impl DictionaryHost,
) -> u32 {
    // The existing dispatcher port forwards the context through HeapVtable.
    // Resolve its actual guest method before borrowing mutable arena memory.
    let slot = if operation == 2 && pointer == 0 {
        0
    } else {
        operation
    };
    let active = heap != 0
        && if operation == 3 {
            pointer != 0
        } else {
            size != 0
        };
    let method = if active {
        word(m, word(m, heap) + slot * 4)
    } else {
        0
    };
    let mut descriptor = [0u8; 8];
    if heap != 0 {
        descriptor[4..].copy_from_slice(&word(m, heap + 4).to_le_bytes());
    }
    let heap = if heap == 0 { None } else { Some(0) };
    let mut dispatch = DictionaryHeap {
        memory: m,
        host,
        method,
    };
    match operation {
        0 => crate::frontend::heap_alloc(&descriptor, heap, size, &mut dispatch),
        2 => crate::frontend::heap_realloc(&descriptor, heap, pointer, size, &mut dispatch),
        3 => {
            crate::frontend::heap_release(
                &descriptor,
                heap,
                if pointer == 0 { None } else { Some(pointer) },
                &mut dispatch,
            );
            0
        }
        _ => unreachable!(),
    }
}

pub(crate) fn dictionary_lock(
    m: &mut [u8],
    registry: u32,
    acquire: bool,
    host: &mut impl DictionaryHost,
) -> u32 {
    let object = word(m, registry + 12);
    let object = if object == 0 { None } else { Some(object) };
    let mut target = None;
    let rc = if acquire {
        crate::frontend::acquire_lock(m, object, |p| {
            target = Some(p);
            0
        })
    } else {
        crate::frontend::release_lock(m, object, |p| {
            target = Some(p);
            0
        })
    };
    if let Some(target) = target {
        let inner = word(m, word(m, object.unwrap()) + 8);
        let method = word(m, inner + if acquire { 8 } else { 12 });
        host.lock_method(m, method, target)
    } else {
        rc
    }
}
pub(crate) fn dictionary_registry(
    m: &mut [u8],
    registry: u32,
    name: u32,
    host: &mut impl DictionaryHost,
) -> (u32, u32, Option<u32>) {
    if registry == 0 || name == 0 {
        return (0, 0, None);
    }
    let lock = dictionary_lock(m, registry, true, host);
    if lock >= 0x80000000 {
        return (lock, 0, None);
    }
    let map = word(m, registry + 16);
    let mut found = 0;
    let find = crate::frontend::lookup_value(
        m,
        if map == 0 { None } else { Some(map) },
        name,
        Some(&mut found),
        |key, stored| {
            let table = word(m, map + 24 + 36);
            host.compare_key(m, word(m, table + 8), key, stored)
        },
    );
    (
        lock,
        find,
        if find < 0x80000000 { Some(found) } else { None },
    )
}

/// fn_198f1c adapter. The balancing and heap dispatchers are existing ports;
/// only guest key/value function pointers cross the callback host.
pub(crate) fn dictionary_release_node(m: &mut [u8], tree: u32, node: u32, host: &mut impl DictionaryHost) {
    crate::normalization::rb_erase(m, tree, node);
    let table = word(m, tree + 36);
    let key = word(m, node + 16);
    if key != 0 {
        host.tree_callback(m, word(m, table + 4), &[key, word(m, tree + 32)]);
    }
    let value = word(m, node + 20);
    if value != 0 {
        host.tree_callback(m, word(m, table + 16), &[value, word(m, tree + 32)]);
    }
    dictionary_heap(m, word(m, tree + 32), node, 0, 3, host);
}

/// fn_2eca2c. Release a dictionary session and its owned name.
pub fn dictionary_free(
    m: &mut [u8],
    session: u32,
    env: &DictionaryEnvironment,
    host: &mut impl DictionaryHost,
) -> u32 {
    if session == 0 || word(m, session) == 0 {
        return 0x81102007;
    }
    if word(m, session + 8) != 0 {
        let registry = word(m, word(m, session) + 24);
        let (lock, find, found) = dictionary_registry(m, registry, env.registry_name, host);
        let scripts = crate::frontend::ReleaseScripts {
            lock,
            find,
            found,
            remove: 0,
            unlock: 0,
        };
        let has_callback = scripts.found.is_some_and(|p| word(m, p + 24) != 0);
        let mut event = None;
        let callback = if has_callback {
            Some(|h0, h1, snapshot: &mut [u8; 32]| {
                event = Some(([h0, h1], *snapshot));
            })
        } else {
            None
        };
        crate::frontend::release_ling_entry(
            m,
            if registry == 0 { None } else { Some(registry) },
            if env.registry_name == 0 {
                None
            } else {
                Some(env.registry_name)
            },
            &scripts,
            callback,
        );
        if let Some((owner, mut snapshot)) = event {
            let map = word(m, registry + 16);
            let mut removed = None;
            crate::frontend::remove_tree_value(
                m,
                if map == 0 { None } else { Some(map) },
                env.registry_name,
                |key, stored| {
                    let table = word(m, map + 24 + 36);
                    host.compare_key(m, word(m, table + 8), key, stored)
                },
                |tree, node| {
                    removed = Some((tree, node));
                },
            );
            if let Some((tree, node)) = removed {
                dictionary_release_node(m, tree, node, host);
            }
            host.release_callback(m, owner, &mut snapshot);
        }
        if registry != 0 && env.registry_name != 0 && lock < 0x80000000 {
            dictionary_lock(m, registry, false, host);
        }
        put(m, session + 8, 0);
        put(m, session + 12, 0);
        put(m, session + 16, 0);
    }
    let heap = word(m, word(m, session) + 4);
    let name = word(m, session + 4);
    if name != 0 {
        dictionary_heap(m, heap, name, 0, 3, host);
    }
    dictionary_heap(m, word(m, word(m, session) + 4), session, 0, 3, host);
    0
}

/// fn_2ecaac. Allocate a session and open its named dictionary.
pub fn dictionary_new(
    m: &mut [u8],
    name: u32,
    handle: [u32; 2],
    out: u32,
    env: &DictionaryEnvironment,
    host: &mut impl DictionaryHost,
) -> u32 {
    if out == 0 {
        return 0x81102007;
    }
    let mut context = 0;
    let cached = [
        u32::from_le_bytes(env.cached_handle[..4].try_into().unwrap()),
        u32::from_le_bytes(env.cached_handle[4..].try_into().unwrap()),
    ];
    let pointed = if handle != cached && handle[0] != 0 && handle[1] == 0x1e61 {
        Some(half(m, handle[0]))
    } else {
        None
    };
    let rc = crate::frontend::resolve_engine_context(
        handle[0],
        pointed,
        handle[1],
        &env.cached_handle,
        env.default_context,
        &mut context,
    );
    if rc >= 0x80000000 {
        return 0x81102007;
    }
    put(m, out, 0);
    let heap = word(m, context + 4);
    let session = dictionary_heap(m, heap, 0, 20, 0, host);
    if session == 0 {
        return 0x8110200a;
    }
    n::memset_at(m, session, 0, 20);
    put(m, session, context);
    let size = n::strlen8(m, name).wrapping_add(1);
    let copy = dictionary_heap(m, word(m, context + 4), 0, size, 0, host);
    put(m, session + 4, copy);
    let mut result = 0x8110200a;
    if copy != 0 {
        crate::frontend::copy_str(m, copy, name);
        result = 0x81102007;
        if word(m, session) != 0 {
            let registry = word(m, word(m, session) + 24);
            let (lock, find, found) = dictionary_registry(m, registry, env.registry_name, host);
            let scripts = crate::frontend::AcquireScripts {
                lock,
                find,
                found,
                unlock: 0,
            };
            let saved = word(m, env.temporary_word);
            result = crate::frontend::acquire_ling_entry(
                m,
                if registry == 0 { None } else { Some(registry) },
                if env.registry_name == 0 {
                    None
                } else {
                    Some(env.registry_name)
                },
                Some(env.temporary_word),
                &scripts,
            );
            let entry = word(m, env.temporary_word);
            put(m, env.temporary_word, saved);
            if registry != 0 && env.registry_name != 0 && lock < 0x80000000 {
                let unlock = dictionary_lock(m, registry, false, host);
                if unlock >= 0x80000000 && result < 0x80000000 {
                    result = unlock;
                }
            }
            if result < 0x80000000 {
                put(m, session + 8, word(m, entry + 4));
                put(m, session + 16, word(m, entry + 12));
                put(m, session + 12, word(m, entry + 8));
                let mut flag = 0;
                let method = word(m, word(m, session + 8) + 64);
                result = host.open_dictionary(
                    m,
                    method,
                    [word(m, session + 12), word(m, session + 16)],
                    name,
                    &mut flag,
                );
                if result < 0x80000000 {
                    if flag != 0 {
                        put(m, out, session);
                        return result;
                    }
                    result = 0x81102001;
                }
            }
        }
    }
    dictionary_free(m, session, env, host);
    result
}

/// fn_2ecc3c. Resize and replace a dictionary session's name.
pub fn dictionary_rename(
    m: &mut [u8],
    session: u32,
    name: u32,
    host: &mut impl DictionaryHost,
) -> u32 {
    if session == 0 || name == 0 {
        return 0x81102007;
    }
    let old = word(m, session + 4);
    let old_length = n::strlen8(m, old);
    let destination = if n::strlen8(m, name) > old_length {
        let size = n::strlen8(m, name).wrapping_add(1);
        let heap = word(m, word(m, session) + 4);
        let new = dictionary_heap(m, heap, old, size, 2, host);
        if new == 0 {
            return 0x8110200a;
        }
        put(m, session + 4, new);
        new
    } else {
        old
    };
    crate::frontend::copy_str(m, destination, name);
    0
}

/// fn_2eccb4. Try successively shorter dictionary prefixes, at most 8 bytes.
/// The callback's returned halfword is a presence flag, not the consumed size.
/// The consumed size is the successful prefix's length.
pub fn dictionary_match(
    m: &mut [u8],
    session: u32,
    name: u32,
    out: u32,
    host: &mut impl DictionaryHost,
) -> u32 {
    if session == 0 || name == 0 || out == 0 {
        return 0;
    }
    put(m, out, 0);
    let count = n::strlen8(m, name) as u16;
    if count == 0 {
        return 0;
    }
    let mut prefix = [0u8; 9];
    let mut count = core::cmp::min(count, 8) as usize;
    prefix[..count].copy_from_slice(&m[name as usize..name as usize + count]);
    let mut result = 0;
    let mut flag = 0;
    while count != 0 {
        prefix[count] = 0;
        let mut length = 0;
        let method = word(m, word(m, session + 8) + 28);
        let rc = host.match_dictionary(
            m,
            method,
            [word(m, session + 12), word(m, session + 16)],
            word(m, session + 4),
            &prefix[..count + 1],
            &mut result,
            &mut length,
            &mut flag,
        );
        if rc >= 0x80000000 {
            return 0;
        }
        if length != 0 {
            put(m, out, word(m, result));
            return count as u32;
        }
        count -= 1;
    }
    0
}

/// External reader calls in fn_2ee920 and fn_2ee6a8.
pub trait EscapeReaderHost: ResourceHost {
    /// fn_24277c, returning status, tag bytes, size and data.
    fn next_chunk(&mut self, m: &mut [u8], reader: u32, size: u32, data: u32)
        -> (u32, [u8; 8], u32, u32);
    /// fn_2422ac.
    fn advance_chunk(&mut self, m: &mut [u8], reader: u32) -> u32;
    /// fn_243ad4, returning status, owned chunk and data.
    fn detach_chunk(&mut self, m: &mut [u8], reader: u32, chunk: u32, data: u32)
        -> (u32, u32, u32);
}

/// fn_2ee6a8. Decode the two halves of the EPAT table and retain its data.
pub fn read_patterns(
    m: &mut [u8], ctx: u32, reader: u32, src: u32, mode: u32,
    host: &mut impl EscapeReaderHost,
) -> u32 {
    let first_data = word(m, src + 4).wrapping_add(8);
    let second_header = first_data.wrapping_add(word(m, src));
    let second_data = second_header.wrapping_add(8).wrapping_add(word(m, src + second_header + 4));
    let mut off = second_data.wrapping_add(word(m, src + second_header));
    let count = half(m, src + off) >> 1;
    put_half(m, ctx + 44, count);
    off = off.wrapping_add(2);
    let heap = word(m, word(m, ctx) + 4);
    let table = host.calloc(m, heap, 1, count as u32 * 12 + 1).unwrap_or(0);
    put(m, ctx + 48, table);
    if table == 0 {
        put_half(m, ctx + 44, 0);
        return 0x8000200a;
    }
    let mut j = 0u16;
    let mut i = 0u16;
    while (i as u32) < count as u32 * 2 {
        let len = half(m, src + off) as u32;
        let selected = (mode == 1 && i < count) || (mode == 0 && i >= count);
        if selected {
            let buf = host.calloc(m, heap, 1, len + 1).unwrap_or(0);
            put(m, table + j as u32 * 12 + 8, buf);
            if buf == 0 { return 0x8000200a; }
            n::memcpy_at(m, buf, src + off + 2, len);
        }
        off = off.wrapping_add(len + 2);
        if selected {
            put_half(m, table + j as u32 * 12 + 4, half(m, src + off));
            put(m, table + j as u32 * 12, word(m, src + off + 2));
            j = j.wrapping_add(1);
        }
        off = off.wrapping_add(6);
        i = i.wrapping_add(1);
    }
    let (rc, chunk, data) = host.detach_chunk(m, reader, word(m, ctx + 8), src);
    put(m, ctx + 8, chunk);
    if rc < 0x80000000 {
        put(m, ctx + 52, data.wrapping_add(if mode == 1 { first_data } else { second_data }));
    }
    rc
}

/// fn_2ee920. ESCD resource callback, using the shared table decoders.
pub fn read_escape_chunk(
    m: &mut [u8], ctx: u32, reader: u32, mut size: u32, mut data: u32,
    mode: u32, host: &mut impl EscapeReaderHost,
) -> u32 {
    if m[(ctx + 20) as usize] == 1 { return 0; }
    let rc = loop {
        let (mut rc, tag, next_size, next_data) = host.next_chunk(m, reader, size, data);
        size = next_size;
        data = next_data;
        if rc >= 0x80000000 {
            if rc & 0x1fff == 0x14 { return 0; }
            break rc;
        }
        let kind = if tag[..5] == *b"EMAP\0" { 1 }
            else if tag[..5] == *b"ENRG\0" { 2 }
            else if tag[..5] == *b"EVDN\0" { 3 }
            else if tag[..5] == *b"EPAT\0" { 4 }
            else if tag[..5] == *b"EENU\0" { 5 }
            else if tag[..5] == *b"ESTR\0" { 6 } else { 0 };
        if kind != 0 {
            m[(ctx + 20) as usize] = 1;
            if kind <= 3 {
                let count_at = ctx + match kind { 1 => 22, 2 => 28, _ => 36 };
                put_half(m, count_at, half(m, data));
                let count = half(m, count_at);
                let heap = word(m, word(m, ctx) + 4);
                let table = host.calloc(m, heap, 1, count as u32 * 12 + 1).unwrap_or(0);
                let table_at = ctx + match kind { 1 => 24, 2 => 32, _ => 40 };
                put(m, table_at, table);
                if table == 0 {
                    if kind == 3 { put(m, count_at, 0); } else { put_half(m, count_at, 0); }
                    break 0x8000200a;
                }
                let mut off = data + 2;
                for i in 0..count as u32 {
                    let dst = table + i * 12;
                    if kind == 1 {
                        m[(dst + 4) as usize] = half(m, off) as u8;
                        let len = half(m, off + 2) as u32;
                        off += 4;
                        let buf = host.calloc(m, heap, 1, len * 2 + 2).unwrap_or(0);
                        put(m, dst, buf);
                        if buf == 0 { rc = 0x8000200a; break; }
                        n::memcpy_at(m, buf, off, len * 2);
                        off += len * 2;
                        put_half(m, dst + 6, half(m, off));
                        put(m, dst + 8, word(m, off + 2));
                        off += 6;
                    } else {
                        put_half(m, dst, half(m, off));
                        put(m, dst + 4, word(m, off + 2));
                        if kind == 2 {
                            put(m, dst + 8, word(m, off + 6));
                            off += 10;
                        } else {
                            m[(dst + 8) as usize] = half(m, off + 6) as u8;
                            off += 8;
                        }
                    }
                    rc = 0;
                }
                if rc >= 0x80000000 { break rc; }
            } else {
                rc = match kind {
                    4 => read_patterns(m, ctx, reader, data, mode, host),
                    5 => read_choices(m, ctx, data, host),
                    _ => read_defaults(m, ctx, data, host),
                };
                if rc >= 0x80000000 { break rc; }
            }
        }
        rc = host.advance_chunk(m, reader);
        if rc >= 0x80000000 { break rc; }
    };
    clear_resources(m, ctx, host);
    rc
}

/// Caller-owned constants and disjoint temporary storage for fn_2eee7c.
pub struct EscapeEnvironment<'a> {
    pub cached_handle: [u8; 8],
    pub default_context: u32,
    pub descriptor: u32,
    pub parameter_output: u32,
    pub tag: u32,
    pub callback: u32,
    pub destructor: u32,
    pub parameter_name: u32,
    /// Arena-sized caller-owned immutable comparison view.
    pub snapshot: &'a mut [u8],
}

/// fn_2eee7c. Allocate an escape context and register its ESCD callback.
/// Runs the existing context resolver, descriptor and parameter lookup ports.
#[allow(clippy::too_many_arguments)]
pub fn escape_new<H>(
    m: &mut [u8], handle: [u32; 2], a: u32, b: u32, out: u32, source: u32,
    env: &mut EscapeEnvironment<'_>, host: &mut H,
) -> u32
where H: NormalizationHost + crate::render_part_c::Host + crate::frontend::StateLock {
    if out == 0 { return 0x80002007; }
    let mut owner = 0;
    let cached = [u32::from_le_bytes(env.cached_handle[..4].try_into().unwrap()),
        u32::from_le_bytes(env.cached_handle[4..].try_into().unwrap())];
    let pointed = if handle != cached && handle[0] != 0 && handle[1] == 0x1e61 {
        Some(half(m, handle[0]))
    } else { None };
    let rc = crate::frontend::resolve_engine_context(
        handle[0], pointed, handle[1], &env.cached_handle, env.default_context, &mut owner);
    if rc >= 0x80000000 { return 0x80002007; }
    put(m, out, 0);
    let heap = word(m, owner + 4);
    let ctx = host.calloc(m, heap, 1, 0x4c).unwrap_or(0);
    if ctx == 0 { return 0x8000200a; }
    put(m, ctx, owner);
    m[(ctx + 74) as usize] = 0;
    n::memset_at(m, ctx + 20, 0, 52);
    put(m, ctx + 4, source);
    put(m, ctx + 8, 0);
    put(m, ctx + 12, a);
    put(m, ctx + 16, b);
    let mut saved = [0; 16];
    let d = env.descriptor as usize;
    saved.copy_from_slice(&m[d..d + 16]);
    put(m, env.descriptor, env.tag);
    put(m, env.descriptor + 4, env.callback);
    put(m, env.descriptor + 8, env.destructor);
    put(m, env.descriptor + 12, ctx);
    let rc = crate::render_part_c::fn_2c61a0(
        &mut crate::boot::Memory { base: 0, bytes: m }, source, env.descriptor, 1, host);
    m[d..d + 16].copy_from_slice(&saved);
    if rc >= 0x80000000 {
        host.free(m, heap, ctx);
        return rc;
    }
    put_half(m, ctx + 72, 0x200);
    let saved = word(m, env.parameter_output);
    let property = word(m, owner + 20);
    env.snapshot[..m.len()].copy_from_slice(m);
    let snapshot = &env.snapshot[..m.len()];
    let rc = crate::frontend::get_param_uint(
        m, if property == 0 { None } else { Some(property) },
        env.parameter_name, env.parameter_output, host,
        |a, b| crate::frontend::compare_str(&snapshot[a as usize..], &snapshot[b as usize..]));
    let value = word(m, env.parameter_output);
    put(m, env.parameter_output, saved);
    if rc < 0x80000000 {
        put_half(m, ctx + 72, (value as u16).wrapping_shl(1).wrapping_add(64));
    }
    put(m, out, ctx);
    0
}

/// fn_2ffb28. The locale table address is an immutable caller ROM mapping.
pub fn locale_table(address: u32) -> u32 { address }

/// Escape matching calls the real regex::pcre_match, including bridge-4's
/// failure-side subject-pointer writes. Strict replays compare every arena byte.
/// Caller-owned representation buffers for the guest PCRE frame arena.
/// Program and subject buffers hold only immutable match inputs, never an
/// arena clone. Allocator hooks receive the live arena on every call.
pub struct EscapeScratch<'a> {
    pub program: &'a mut [u8],
    pub subject: &'a mut [u8],
    pub frames: &'a mut [crate::regex::MatchFrame],
    pub tags: &'a mut [u8],
    pub blocks: &'a mut [crate::regex::FrameBlockState],
    /// Disjoint guest stack workspace, at least 128 bytes; restored on return.
    pub stack: u32,
    pub fallback: u32,
    /// D_a09390, D_a09394 and D_a093da, terminated byte strings.
    pub toi_names: [u32; 3],
}

pub(crate) struct MatchHost<'a, H> {
    pub(crate) memory: &'a mut [u8],
    pub(crate) host: &'a mut H,
}
impl<H: NormalizationHost> crate::regex::PcreHost for MatchHost<'_, H> {
    fn validate(&mut self, object: u32, key: u32, tag: u32, size: u32) -> u32 {
        crate::frontend::validate_handle(object != 0, key, tag, size)
    }
    fn resolve_owner(&mut self, _: u32, _: i32) -> (u32, u32) {
        unreachable!("exec does not resolve an owner")
    }
    fn heap_of(&mut self, owner: u32) -> u32 { word(self.memory, owner + 4) }
    fn alloc(&mut self, heap: u32, count: u32, size: u32) -> Option<u32> {
        self.host.calloc(self.memory, heap, count, size)
    }
    fn free(&mut self, heap: u32, id: u32) { self.host.free(self.memory, heap, id); }
}

fn guest_ref(blocks: &[crate::regex::FrameBlockState], p: u32, bias: u32, span:u32) -> Option<crate::regex::FrameRef> {
    if p == 0 { return None; }
    for (i,b) in blocks.iter().enumerate() {
        let offset = p.wrapping_sub(b.code_id).wrapping_sub(bias);
        if b.present && offset < span {
            return Some(crate::regex::FrameRef { block:i as u16, offset });
        }
    }
    None
}
fn frame_pointer(blocks: &[crate::regex::FrameBlockState], r: Option<crate::regex::FrameRef>, bias:u32) -> u32 {
    r.map_or(0, |r| blocks[r.block as usize].code_id.wrapping_add(r.offset).wrapping_add(bias))
}

/// Bidirectional frame conversion preserves untouched fields, including stale
/// code/subject pointers. Guest pointers and Rust offsets differ by a fixed
/// wrapping displacement, so arbitrary stale words round-trip unchanged.
pub(crate) fn load_match_frame(m:&[u8], p:u32, code:u32, delta:u32, blocks:&[crate::regex::FrameBlockState],span:u32) -> crate::regex::MatchFrame {
    let mut f=crate::regex::MatchFrame::default();
    let w=|i:u32| word(m,p+i*4);
    let cp=|v:u32| v.wrapping_sub(code) as usize;
    let sp=|v:u32| v.wrapping_add(delta) as i32 as i64;
    f.prev=guest_ref(blocks,w(0),0,span); f.eptr=sp(w(1)); f.ecode=cp(w(2));
    f.offset_top=w(3) as i32; f.ims=w(4); f.eptrb=guest_ref(blocks,w(5),272,span); f.flags=w(6);
    f.callpat=cp(w(7)); f.charptr=cp(w(8)); f.data=cp(w(9)); f.next=cp(w(10));
    f.pp=sp(w(11)); f.prev_code=cp(w(12)); f.saved_eptr=sp(w(13));
    f.rec_prev=guest_ref(blocks,w(14),56,span); f.rec_group=w(15) as i32; f.rec_after=cp(w(16));
    f.rec_start=sp(w(17)); f.rec_saved_max=w(19) as i32;
    f.rec_scratch=w(18)==p+152; f.rec_scratch_base=w(18) as usize;
    f.cur_is_word=w(20) as i32; f.condition=w(21) as i32; f.minimize=w(22) as i32; f.prev_is_word=w(23) as i32;
    f.original_ims=w(24); f.ctype=w(25) as i32; f.fc=w(26) as i32; f.fi=w(27) as i32;
    f.length=w(28) as i32; f.max=w(29) as i32; f.min=w(30) as i32; f.number=w(31) as i32;
    f.offset=w(32) as i32; f.op=w(33) as i32; f.save_capture_last=w(34) as i32;
    f.save_offset1=w(35) as i32; f.save_offset2=w(36) as i32; f.save_offset3=w(37) as i32;
    for i in 0..30 { f.stacksave[i]=w(38+i as u32) as i32; }
    f.block_eptr=sp(w(69)); f.block_prev=guest_ref(blocks,w(68),272,span); f.result=w(70) as i32;
    if !f.rec_scratch && f.rec_scratch_base!=0 && f.rec_saved_max>30 {
        for i in 0..f.rec_saved_max as usize { f.rec_spill[i]=word(m,f.rec_scratch_base as u32+i as u32*4) as i32; }
    }
    f
}
pub(crate) fn save_match_frame(m:&mut [u8], p:u32, f:&crate::regex::MatchFrame, code:u32, delta:u32, blocks:&[crate::regex::FrameBlockState]) {
    let cp=|v:usize| if v==usize::MAX {0} else {(v as u32).wrapping_add(code)};
    let sp=|v:i64| (v as u32).wrapping_sub(delta);
    let vals=[
        frame_pointer(blocks,f.prev,0), sp(f.eptr),cp(f.ecode),f.offset_top as u32,f.ims,
        frame_pointer(blocks,f.eptrb,272),f.flags,cp(f.callpat),cp(f.charptr),cp(f.data),cp(f.next),
        sp(f.pp),cp(f.prev_code),sp(f.saved_eptr),frame_pointer(blocks,f.rec_prev,56),f.rec_group as u32,
        cp(f.rec_after),sp(f.rec_start),if f.rec_scratch {p+152} else {f.rec_scratch_base as u32}, f.rec_saved_max as u32,
        f.cur_is_word as u32,f.condition as u32,f.minimize as u32,f.prev_is_word as u32,f.original_ims,
        f.ctype as u32,f.fc as u32,f.fi as u32,f.length as u32,f.max as u32,f.min as u32,
        f.number as u32,f.offset as u32,f.op as u32,f.save_capture_last as u32,f.save_offset1 as u32,
        f.save_offset2 as u32,f.save_offset3 as u32,
    ];
    for (i,v) in vals.iter().enumerate() {put(m,p+i as u32*4,*v);}
    for i in 0..30 {put(m,p+152+i as u32*4,f.stacksave[i] as u32);}
    put(m,p+272,frame_pointer(blocks,f.block_prev,272)); put(m,p+276,sp(f.block_eptr)); put(m,p+280,f.result as u32);
    if !f.rec_scratch && f.rec_scratch_base!=0 && f.rec_saved_max>30 {
        for i in 0..f.rec_saved_max as usize {put(m,f.rec_scratch_base as u32+i as u32*4,f.rec_spill[i] as u32);}
    }
}

/// Guest fn_23fe54 adapter. The caller supplies capacity for every configured
/// frame block and immutable input. Heap callbacks reenter through MatchHost.
#[allow(clippy::too_many_arguments)]
pub fn exec_escape_pattern(m:&mut [u8], ctx:u32, pattern:u32, text:u32, len:u32, start:u32, options:u32,
    offsets:&mut [i32;100], scratch:&mut EscapeScratch<'_>, tables:&crate::regex::PcreTables<'_>,
    host:&mut impl NormalizationHost) -> i32 {
    let object=word(m,ctx+12); let key=word(m,ctx+16);
    // Invalid handles are rejected before dereferencing the frame store.
    if crate::frontend::validate_handle(object!=0,key,0xdd57,24)>=0x80000000 {return -16;}
    let entries=word(m,object+4);
    let count=half(m,object+8); let per=half(m,object+10); let size=half(m,object+12); let max=half(m,object+14);
    assert!(scratch.blocks.len()>=max as usize);
    assert!(scratch.frames.len()>=max as usize*per as usize && scratch.tags.len()>=max as usize*per as usize);
    for b in scratch.blocks.iter_mut() {*b=crate::regex::FrameBlockState::default();}
    for i in 0..count as usize {
        let p=word(m,entries+i as u32*4);
        scratch.blocks[i]=crate::regex::FrameBlockState {
            present:p!=0,live:p!=0,used:if p!=0 {word(m,p+8)}else{0},count:if p!=0 {m[(p+16) as usize]}else{0},
            code_id:if p!=0 {word(m,p)}else{0},extra_id:if p!=0 {word(m,p+12)}else{0},entry_id:p,
        };
    }
    let plen=if pattern==0 {0} else {word(m,pattern+4) as usize};
    assert!(plen<=scratch.program.len() && len as usize+512<=scratch.subject.len());
    scratch.program[..plen].copy_from_slice(&m[pattern as usize..pattern as usize+plen]);
    scratch.subject[..len as usize+512].fill(0);
    scratch.subject[256..256+len as usize].copy_from_slice(&m[text as usize..text as usize+len as usize]);
    let code=pattern+if plen>=40 {half(m,pattern+24) as u32+half(m,pattern+26) as u32*half(m,pattern+28) as u32}else{0};
    let delta=256u32.wrapping_sub(text);
    let zero=[0u8;284];
    let fresh=load_match_frame(&zero,0,code,delta,scratch.blocks,per as u32*size as u32);
    for f in scratch.frames.iter_mut() { *f=fresh; }
    for i in 0..count as usize {
        let b=scratch.blocks[i];
        if !b.present {continue;}
        for j in 0..per as usize {
            let slot=i*per as usize+j;
            scratch.tags[slot]=m[b.extra_id as usize+j];
            scratch.frames[slot]=load_match_frame(m,b.code_id+j as u32*size as u32,code,delta,scratch.blocks,per as u32*size as u32);
        }
    }
    let heap=word(m,word(m,object)+4);
    let mut store=crate::regex::FrameStore { frames:scratch.frames,tags:scratch.tags,blocks:scratch.blocks,
        frames_per_block:per,frame_size:size,max_blocks:max,block_count:count,current_block:half(m,object+16),
        highest_block:half(m,object+18),max_frames:half(m,object+20),fresh,heap,blocks_id:Some(entries) };
    let rc=crate::regex::pcre_exec_simple(&mut MatchHost{memory:m,host},object,key,&mut store,
        &scratch.program[..plen],None,&scratch.subject[..len as usize+512],256,len as usize,start as i32,options,
        Some(offsets),100,tables);
    put_half(m,object+8,store.block_count);put_half(m,object+16,store.current_block);
    put_half(m,object+18,store.highest_block);put_half(m,object+20,store.max_frames);
    for (i,b) in store.blocks.iter().enumerate() {
        if i<count as usize || b.entry_id!=0 {
            let old=word(m,entries+i as u32*4);
            if i!=0 && i<count as usize && old!=0 {put(m,old,0);put(m,old+12,0);}
            put(m,entries+i as u32*4,b.entry_id);
        }
        if !b.present {continue;}
        put(m,b.entry_id,b.code_id);put(m,b.entry_id+4,per as u32*size as u32+1);
        put(m,b.entry_id+8,b.used);put(m,b.entry_id+12,b.extra_id);m[(b.entry_id+16) as usize]=b.count;
        if b.code_id==0 || b.extra_id==0 {continue;}
        for j in 0..per as usize {
            let slot=i*per as usize+j;
            m[b.extra_id as usize+j]=store.tags[slot];
            save_match_frame(m,b.code_id+j as u32*size as u32,&store.frames[slot],code,delta,store.blocks);
        }
    }
    rc
}

// The compatibility parser API has no configured output-method dispatcher.
// Null loggers still execute the real fn_173dc0 port, as in the ROM. Reject
// non-null loggers instead of silently suppressing a live callback.
struct EscapeNullOutput;
impl crate::boot_part_b::Host for EscapeNullOutput {
    fn thread_id(&mut self)->u32 { unreachable!("null logger needs no thread ID") }
    fn call(&mut self,_:&mut crate::boot::Memory<'_>,_:u32,_:&[u32])->u32 {
        unreachable!("escape parser requires a configured logger output dispatcher")
    }
}
impl crate::boot_part_b::FormatHost for EscapeNullOutput {}
fn escape_error(m:&mut [u8],ctx:u32,code:u32) {
    let logger=word(m,word(m,ctx)+16);
    assert_eq!(logger,0,"escape parser requires a configured logger output dispatcher");
    crate::render_part_a::error_log(&mut crate::boot::Memory{base:0,bytes:m},
        &mut EscapeNullOutput,logger,1,code,0,0,0);
}
/// fn_2ecdb8, EU fn_2ecf74, KR fn_2ed020.
#[allow(clippy::too_many_arguments)]
pub fn match_escape(m:&mut [u8], name:&[u8], ctx:u32,text:u32,mut count:u32,start:u32,options:u32,
    found:u32,out:u32,fallback:u32,index:u32,scratch:&mut EscapeScratch<'_>,
    tables:&crate::regex::PcreTables<'_>,host:&mut impl NormalizationHost)->u32 {
    m[found as usize]=0;
    let base=word(m,ctx+52);
    let record=if index==999 {
        let mut selected=None;
        for i in 0..half(m,ctx+44) as u32 {
            let p=word(m,ctx+48)+i*12;
            let s=word(m,p+8);
            if name.iter().enumerate().all(|(i,b)| m[s as usize+i]==*b) && m[s as usize+name.len()]==0 {
                selected=Some(word(m,p).wrapping_add(base));break;
            }
        }
        let Some(p)=selected else {return 0;};p
    } else { word(m,word(m,ctx+40)+index*12+4).wrapping_add(base) };
    if record==0 {return 0;}
    count=count.min(half(m,ctx+72) as u32);
    let mut results=[-1i32;100];
    let n=exec_escape_pattern(m,ctx,record,text,count,start,options,&mut results,scratch,tables,host);
    if n<0 {
        let rc=crate::regex::error_status(n);
        if rc>=0x80000000 && ((rc>>20)&!0x800)==0xa5 {
            if rc&0x1fff==20 {return 0;}
            escape_error(m,ctx,if rc&0x1fff==10{0x2afa}else{0x2b13});
        }
        return rc;
    }
    if n>0 {
        for i in 0..6 {put(m,out+i*4,results[i as usize] as u32);}
        if results[4]==-1 && results[5]==-1 && results[6]!=-1 && results[7]!=-1 {
            m[fallback as usize]=1;put(m,out+16,results[6] as u32);put(m,out+20,results[7] as u32);
        }
        m[found as usize]=1;
    }
    0
}

fn string_kind(k:u16)->bool {matches!(k,6|12|21|50|36|39|37|34|22|40|26|29|0)}
fn default_command(m:&mut [u8],ctx:u32,out:u32)->bool {
    let rec=word(m,out+4);
    for i in 0..half(m,ctx+22) as u32 {
        let p=word(m,ctx+24)+i*12;
        if half(m,p+6)==half(m,rec+10) && m[(p+4) as usize]==1 {
            let v=word(m,p+8);
            if v!=9999 {put(m,rec+28,v);m[out as usize]=1;return true;}
            return false;
        }
    }
    m[out as usize]=0;false
}

/// fn_2ef4f4. Parse a command's optional string, numeric or pattern value.
#[allow(clippy::too_many_arguments)]
fn parse_escape_value_inner(m:&mut [u8],ctx:u32,out:u32,text:u32,size:u32,start:u32,end:u32,
    handled:u32,incomplete:u32,next:u32,scratch:&mut EscapeScratch<'_>,tables:&crate::regex::PcreTables<'_>,
    host:&mut impl NormalizationHost)->u32 {
    let rec=word(m,out+4);
    let kind=half(m,rec+10);
    if command_end(m,text,size,word(m,start),end,kind as u32)==0 {
        m[handled as usize]=0;m[incomplete as usize]=1;return 0;
    }
    put(m,next,word(m,end)+2);
    let mut n=word(m,end)>>1;
    while n!=0 && matches!(half(m,text+(n-1)*2),32|9|10|13) {n-=1;}
    put(m,end,n*2);
    let finish=word(m,end);let begin=word(m,start);
    if finish<=begin {
        if string_kind(kind) {return default_value(m,ctx,out,handled,scratch.fallback,host);}
        let have=default_command(m,ctx,out);
        m[handled as usize]=(have||kind==13) as u8;return 0;
    }
    for i in 0..half(m,ctx+36) as u32 {
        let p=word(m,ctx+40)+i*12;
        if half(m,p)!=kind {continue;}
        let s=scratch.stack+64;
        m[s as usize]=0;m[(s+1) as usize]=0;
        let mut rc=match_escape(m,b"dummy",ctx,text+begin,(finish-begin) as u16 as u32,0,16,s,s+4,s+1,i,scratch,tables,host);
        if rc>=0x80000000 {return rc;}
        let mut pattern_begin=begin;
        let pattern_end=finish;
        let length=word(m,s+8).wrapping_sub(word(m,s+4));
        if m[s as usize]==1 && (length==finish-begin ||
            (kind==7 && length+begin+2<finish && half(m,text+begin+(word(m,s+8)>>1)*2)==58 && {
                put(m,end,length+begin);pattern_begin=length+begin+2;
                while pattern_begin<finish && half(m,text+pattern_begin)==32 {pattern_begin+=2;}
                finish-pattern_begin>=2
            })) {m[handled as usize]=1;}
        else {m[handled as usize]=0;m[incomplete as usize]=1;}
        if m[handled as usize]==0 {
            if m[out as usize]==1 {m[handled as usize]=1;m[incomplete as usize]=0;}
            return rc;
        }
        match m[(p+8) as usize] {
            0=>put(m,rec+28,m[(text+word(m,start)) as usize] as u32),
            1=>{
                rc=copy_value(m,ctx,out,text,word(m,start),word(m,end),handled,0,host);
                if rc>=0x80000000 {return rc;}
            },_=>{}
        }
        if word(m,start)!=pattern_begin && word(m,end)!=pattern_end {
            rc=copy_value(m,ctx,out,text,pattern_begin,pattern_end,handled,1,host);
        }
        return rc;
    }
    if string_kind(kind) {
        let rc=copy_value(m,ctx,out,text,begin,finish,handled,0,host);
        if rc>=0x80000000 {return rc;}
        if kind==0 {host.free(m,word(m,word(m,ctx)+4),word(m,rec+28));put(m,rec+28,0);}
        return rc;
    }
    let mut number=None;
    for i in 0..half(m,ctx+28) as u32 {
        let p=word(m,ctx+32)+i*12;
        if half(m,p)==kind {number=Some(p);break;}
    }
    if kind==2 || number.is_some() {
        m[incomplete as usize]=0;m[handled as usize]=0;
        if finish-begin>21 {m[incomplete as usize]=1;return 0;}
        for i in 0..(finish-begin)/2 {
            let c=half(m,text+begin+i*2);
            if c<48||c>57 {m[incomplete as usize]=1;return 0;}
        }
        let buf=scratch.stack+96;
        n::memset_at(m,buf,0,24);n::memcpy_at(m,buf,text+begin,finish-begin);
        let v=crate::render_part_b::fn_1767c4(&mut crate::boot::Memory{base:0,bytes:m},buf,buf+24,10);
        let (min,max)=if kind==2 {(0,1)}else{number.map_or((0,1),|p|(word(m,p+4),word(m,p+8)))};
        put(m,rec+28,if v<min {min}else if v>max {max}else{v});
        m[handled as usize]=1;return 0;
    }
    if kind==13 {m[handled as usize]=1;m[incomplete as usize]=0;}
    else {m[handled as usize]=0;m[incomplete as usize]=0;}
    0
}

fn set_escape_range(m:&mut [u8],rec:u32,start:u32,end:u32) {
    put(m,rec+12,start);put(m,rec+16,end.wrapping_sub(start));
    put(m,rec+20,start);put(m,rec+24,start);
}
fn command_name_equal(m:&[u8],a:u32,b:u32,n:u32)->bool {
    let mut i=0;
    while i<n && half(m,b+i*2)!=0 {
        let x=half(m,a+i*2);let y=half(m,b+i*2);
        if x!=y && !(x.wrapping_sub(1)<127 && crate::fe_text::fold_17c8c0((y&255) as u32) as u8==x as u8) {return false;}
        i+=1;
    }
    true
}

/// fn_2ed440. Dispatch enhanced and text-output-interface escape commands.
fn parse_escape_inner(m:&mut [u8],ctx:u32,text:u32,size:u32,out:u32,state:u32,
    scratch:&mut EscapeScratch<'_>,tables:&crate::regex::PcreTables<'_>,host:&mut impl NormalizationHost)->u32 {
    let s=scratch.stack;
    m[state as usize]=0;m[s as usize]=0;m[(s+1) as usize]=0;
    let mut rc=match_escape(m,b"esc",ctx,text,size,0,0,s,s+4,s+1,999,scratch,tables,host);
    if rc>=0x80000000 || m[s as usize]==0 {return rc;}
    let start=word(m,s+4);let position=word(m,s+8);
    rc=match_escape(m,b"enhancedescseq",ctx,text,size,position,16,s,s+4,s+1,999,scratch,tables,host);
    if rc>=0x80000000 {return rc;}
    let rec=word(m,out+4);
    if m[s as usize]==1 {
        let name_begin=word(m,s+12);let name_end=word(m,s+16);
        put_half(m,rec+10,0);
        for i in 0..half(m,ctx+22) as u32 {
            let p=word(m,ctx+24)+i*12;let name=word(m,p);
            if n::u16_len(m,name)==(name_end-name_begin)/2 && m[(p+4) as usize]==1 &&
                command_name_equal(m,name,text+name_begin,(name_end-name_begin)/2) {
                put_half(m,rec+10,half(m,p+6));m[out as usize]=0;
                let v=word(m,p+8);
                if v!=9999 {put(m,rec+28,v);m[out as usize]=1;}break;
            }
        }
        let kind=half(m,rec+10);
        set_escape_range(m,rec,start,word(m,s+8));
        let handled=s+32;let incomplete=s+33;let end=s+36;
        m[handled as usize]=0;m[incomplete as usize]=0;put(m,end,0);
        if kind==32 {
            if command_end(m,text,size,word(m,s+20),s+24,32)!=0 {
                let finish=word(m,s+24)+2;put(m,end,finish);set_escape_range(m,rec,start,finish);m[state as usize]=2;
                if word(m,s+20)+2!=finish {put_half(m,rec+10,0);}
                else {let toggle=(m[(ctx+74) as usize]==0) as u8;m[(ctx+74) as usize]=toggle;put(m,rec+28,toggle as u32);}
            }
        } else if word(m,s+20)==word(m,s+24) {
            let mut has_type=kind==2;
            for i in 0..half(m,ctx+28) as u32 {has_type|=half(m,word(m,ctx+32)+i*12)==kind;}
            if has_type {put(m,rec+28,0);default_command(m,ctx,out);}
            else if string_kind(kind)&&kind!=0 {rc=default_value(m,ctx,out,handled,scratch.fallback,host);}
            m[state as usize]=2;
        } else {
            rc=parse_escape_value_inner(m,ctx,out,text,size,s+20,s+24,handled,incomplete,end,scratch,tables,host);
            if rc<0x80000000 {
                set_escape_range(m,rec,start,word(m,end));
                if m[handled as usize]!=0 {m[state as usize]=2;}
                else {put_half(m,rec+10,0);if m[incomplete as usize]==0 {m[state as usize]=2;}}
            }
        }
        if rc>=0x80000000 {return rc;}
        if half(m,rec+10)==2 {put_half(m,rec+10,3);}
        return rc;
    }
    rc=match_escape(m,b"esctoi",ctx,text,size,position,16,s,s+4,s+1,999,scratch,tables,host);
    if rc>=0x80000000 || m[s as usize]!=1 {return rc;}
    put_half(m,rec+10,7);set_escape_range(m,rec,start,word(m,s+8));
    let name=word(m,s+12);let k=match m[(text.wrapping_add(name)) as usize] {43=>0,62=>1,94=>2,_=>3};
    if half(m,ctx+56)==0 {
        if k==3 {put_half(m,rec+10,0);return rc;}
        put(m,rec+28,[43,16,60][k]);
    } else {
        put(m,rec+28,0);
        if name!=u32::MAX || word(m,s+16)!=u32::MAX {
            if k==3 {put_half(m,rec+10,0);return rc;}
            let src=scratch.toi_names[k];let mut len=0;
            while m[(src+len) as usize]!=0 {len+=1;}
            let v=host.calloc(m,word(m,word(m,ctx)+4),1,(len+1)*2).unwrap_or(0);
            put(m,rec+28,v);if v==0 {return 0x8000200a;}
            crate::fe_text::strcpy_widen_1763e4(m,v,src);
        }
    }
    m[state as usize]=2;rc
}

/// fn_2ef300. Public escape parser. Internal scratch bytes are restored even
/// when a callee returns an error, so only guest-visible state survives.
pub fn escape_parse(m:&mut [u8],ctx:u32,text:u32,size:u32,record:u32,scratch:&mut EscapeScratch<'_>,
    tables:&crate::regex::PcreTables<'_>,host:&mut impl NormalizationHost)->u32 {
    if ctx==0||text==0||size==0||record==0 {return 0x80002007;}
    if m[(ctx+20) as usize]==0 {escape_error(m,ctx,0x70d);return 0x80002007;}
    let s=scratch.stack;
    let mut saved=[0u8;128];saved.copy_from_slice(&m[s as usize..s as usize+128]);
    put(m,s+124,record);
    parse_escape_inner(m,ctx,text,size,s+120,s+119,scratch,tables,host);
    let rc=if m[(s+119) as usize]==2 {
        if half(m,ctx+56)!=0 {choice_value(m,ctx,s+120,host);}
        0
    } else {0x80002000};
    m[s as usize..s as usize+128].copy_from_slice(&saved);
    rc
}

/// fn_243d50. Close and release a detached chunk through the existing owner
/// close port. Free still runs when close returns an error.
/// STUB: not live-certified. Bridge-4 owns fn_244324's missing broker/logger
/// hooks. Retain this legacy adapter until its memory-last entry is published
/// in E2E.md; the full replay explicitly reports this one stub.
pub fn release_chunk(m:&mut [u8],record:u32,host:&mut impl NormalizationHost)->u32 {
    if record==0 || word(m,record+16)==0 {return 0x8a202007;}
    let rc=n::close_owner_pair(m,record,host);
    host.free(m,word(m,word(m,record)+4),record);
    rc
}

/// Resource callback adapter for clear_resources/read_escape_chunk. Other
/// host methods preserve the live memory-passing allocator interface.
pub struct ResourceAdapter<'a,H>(pub &'a mut H);
impl<H:NormalizationHost> NormalizationHost for ResourceAdapter<'_,H> {
    fn calloc(&mut self,m:&mut [u8],h:u32,n:u32,s:u32)->Option<u32>{self.0.calloc(m,h,n,s)}
    fn free(&mut self,m:&mut [u8],h:u32,p:u32){self.0.free(m,h,p)}
    fn alloc(&mut self,m:&mut [u8],h:u32,s:u32)->Option<u32>{self.0.alloc(m,h,s)}
    fn realloc(&mut self,m:&mut [u8],h:u32,p:u32,s:u32)->Option<u32>{self.0.realloc(m,h,p,s)}
    fn resolve(&mut self,m:&mut [u8],h:u32,k:u32)->(u32,u32){self.0.resolve(m,h,k)}
}
impl<H:NormalizationHost> ResourceHost for ResourceAdapter<'_,H> {
    fn release_chunk(&mut self,m:&mut [u8],p:u32)->u32{release_chunk(m,p,self.0)}
}
impl<H:EscapeReaderHost> EscapeReaderHost for ResourceAdapter<'_,H> {
    fn next_chunk(&mut self,m:&mut [u8],r:u32,s:u32,d:u32)->(u32,[u8;8],u32,u32) {
        self.0.next_chunk(m,r,s,d)
    }
    fn advance_chunk(&mut self,m:&mut [u8],r:u32)->u32 {self.0.advance_chunk(m,r)}
    fn detach_chunk(&mut self,m:&mut [u8],r:u32,c:u32,d:u32)->(u32,u32,u32) {
        self.0.detach_chunk(m,r,c,d)
    }
}

#[allow(clippy::too_many_arguments)]
pub fn parse_escape_value(m:&mut [u8],ctx:u32,out:u32,text:u32,size:u32,start:u32,end:u32,
    handled:u32,incomplete:u32,next:u32,scratch:&mut EscapeScratch<'_>,tables:&crate::regex::PcreTables<'_>,
    host:&mut impl NormalizationHost)->u32 {
    let s=scratch.stack as usize; let mut saved=[0u8;128];saved.copy_from_slice(&m[s..s+128]);
    let rc=parse_escape_value_inner(m,ctx,out,text,size,start,end,handled,incomplete,next,scratch,tables,host);
    m[s..s+128].copy_from_slice(&saved);rc
}
pub fn parse_escape(m:&mut [u8],ctx:u32,text:u32,size:u32,out:u32,state:u32,
    scratch:&mut EscapeScratch<'_>,tables:&crate::regex::PcreTables<'_>,host:&mut impl NormalizationHost)->u32 {
    let s=scratch.stack as usize;let mut saved=[0u8;128];saved.copy_from_slice(&m[s..s+128]);
    let rc=parse_escape_inner(m,ctx,text,size,out,state,scratch,tables,host);
    m[s..s+128].copy_from_slice(&saved);rc
}
