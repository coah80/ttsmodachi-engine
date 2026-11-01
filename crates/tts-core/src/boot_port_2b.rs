

use crate::boot::Memory;

pub use crate::boot_port_2::fn_252d6c;

pub mod owner {
    pub const HEAP: u32 = 0x04;
    pub const RESOURCES: u32 = 0x0c;
    pub const LOGGER: u32 = 0x10;
    pub const FIELD14: u32 = 0x14;
    pub const FIELD18: u32 = 0x18;
}

pub mod state {
    pub const BUFFER: u32 = 0x08;
    pub const TABLES: u32 = 0x0c;
    pub const H0: u32 = 0x10;
    pub const H1: u32 = 0x14;
    pub const OUTPUT: u32 = 0x18;
}

pub mod ctx {
    pub const OWNER: u32 = 0x00;
    pub const MAIN: u32 = 0x04;
    pub const SYSTEM: u32 = 0x24;
    pub const LANGUAGE: u32 = 0x44;
    pub const FIELD64: u32 = 0x64;
    pub const KEYS: u32 = 0x68;
    pub const TABLES: u32 = 0x6c;
    pub const RESULTS: u32 = 0x70;
    pub const QUERY: u32 = 0x74;
    pub const RESOURCES: u32 = 0x78;
    pub const HANDLE7C: u32 = 0x7c;
    pub const HANDLE80: u32 = 0x80;
    pub const RESERVED84: u32 = 0x84;
    pub const DICTIONARY_COUNT: u32 = 0x88;
    pub const RESULT_BLOCKS: u32 = 0x8a;
    pub const RESULT_CAPACITY: u32 = 0x8c;
    pub const QUERY_CAPACITY: u32 = 0x8e;
    pub const TABLE90: u32 = 0x90;
    pub const TOKEN_COUNT: u32 = 0x94;
    pub const TOKEN_ENTRIES: u32 = 0x98;
    pub const MODE9C: u32 = 0x9c;
    pub const NAME: u32 = 0x9e;
    pub const NAME2: u32 = 0x19e;
    pub const FLAG29E: u32 = 0x29e;
    pub const TABLE2A0: u32 = 0x2a0;
}

pub trait Host: crate::boot_part_e::Host {

    fn context(&mut self, m: &mut Memory<'_>, h0: u32, h1: u32, out: &mut u32) -> u32;

    fn resource_open(&mut self, m: &mut Memory<'_>, resources: u32, name: &[u8], out: &mut u32)
        -> u32;

    fn open_named(&mut self, m: &mut Memory<'_>, resources: u32, name: &[u8], out: &mut u32)
        -> u32;

    fn resource_close(&mut self, m: &mut Memory<'_>, resources: u32, name: &[u8]);

    fn parameter(&mut self, m: &mut Memory<'_>, params: u32, name: &[u8], out: &mut u32) -> u32;

    fn setting(&mut self, m: &mut Memory<'_>, settings: u32, name: &[u8]) -> u32;

    fn setting_value(
        &mut self,
        m: &mut Memory<'_>,
        settings: u32,
        name: &[u8],
        out: &mut u32,
    ) -> u32 {
        *out = 0;
        self.setting(m, settings, name)
    }

    fn close_runtime_data(
        &mut self,
        _m: &mut Memory<'_>,
        _o: u32,
        _e: &crate::boot_port_4::Environment,
    ) -> u32 {
        panic!("the runtime-data teardown is only bound by the token rows")
    }
}

pub trait FetchHost {

    fn fetch_table(&mut self, m: &mut Memory<'_>, h0: u32, h1: u32, out: &mut u32) -> u32;

    fn fetch_entry(&mut self, m: &mut Memory<'_>, table: u32, key: u32, a: u32, b: u32) -> u32;

    fn synth_sequence(&mut self, m: &mut Memory<'_>, h0: u32, h1: u32, vt: u32, out: &mut u32)
        -> u32;

    fn release_table(&mut self, m: &mut Memory<'_>, e: u32);

    fn release_sequence(&mut self, m: &mut Memory<'_>, o: u32);
}

pub mod synth {
    pub const H0: u32 = 0x00;
    pub const H1: u32 = 0x04;
    pub const VT: u32 = 0x08;
    pub const FE: u32 = 0x24;
    pub const BE: u32 = 0x28;
    pub const PIPELINE: u32 = 0x2c;
    pub const SIZE: u32 = 0x30;
}

pub mod synth_ctx {
    pub const HEAP: u32 = 0x04;
    pub const PARAMS: u32 = 0x14;
    pub const RESOURCES: u32 = 0x18;
}

pub trait Dictionaries {

    fn open(&mut self, m: &mut Memory<'_>, st: u32, name: u32, size: &mut u32) -> u32;

    fn load(&mut self, m: &mut Memory<'_>, st: u32, key: u32, size: u32, resource: u32) -> u32;

    fn lookup(
        &mut self,
        m: &mut Memory<'_>,
        st: u32,
        text: u32,
        start: &mut i16,
        end: &mut i16,
        fold: i32,
        length: &mut i16,
    ) -> i32;
}

fn fn_26bb44(m: &Memory<'_>, st: u32) -> u16 {
    let tables = m.word(st + state::TABLES);
    let header = m.word(tables);
    u16::from_le_bytes([m.byte(header + 0x30), m.byte(header + 0x31)])
}

fn fn_26bb54(m: &Memory<'_>, st: u32) -> u8 {
    let tables = m.word(st + state::TABLES);
    let header = m.word(tables);
    m.byte(header + 9)
}

fn fn_26bb28(m: &mut Memory<'_>, st: u32, resource: u32, h: &mut impl Host) {
    let tables = m.word(st + state::TABLES);
    let header = m.word(tables);
    h.call(m, 0x179208, &[m.word(st + state::H0), resource, header]);
}

fn fn_26b6fc(m: &mut Memory<'_>, st: u32, h: &mut impl Host) {
    let h1 = m.word(st + state::H1);
    if h1 != 0 {
        h.call(m, 0x179f3c, &[m.word(st + state::H0), h1]);
    }
}

fn strlen(m: &Memory<'_>, p: u32) -> u32 {
    crate::fe_text::strlen_195d90(m.bytes, p - m.base) as u32
}

fn strcpy(m: &mut Memory<'_>, dst: u32, src: u32) {
    crate::normalization::strcpy_at(m.bytes, dst - m.base, src - m.base);
}

fn append_guest(m: &mut Memory<'_>, dst: u32, src: u32) {
    let mut at = dst + strlen(m, dst);
    let mut s = src;
    loop {
        let b = m.byte(s);
        let o = m.offset(at);
        m.bytes[o] = b;
        at += 1;
        if b == 0 {
            break;
        }
        s += 1;
    }
}

fn strcat(m: &mut Memory<'_>, dst: u32, src: u32) {
    crate::fe_text::strcat_195c90(m.bytes, dst - m.base, src - m.base);
}

fn append(m: &mut Memory<'_>, dst: u32, src: &[u8]) {
    let mut at = dst + strlen(m, dst);
    for b in src {
        let o = m.offset(at);
        m.bytes[o] = *b;
        at += 1;
    }
    let o = m.offset(at);
    m.bytes[o] = 0;
}

fn strcmp(m: &Memory<'_>, a: u32, b: u32) -> i32 {
    crate::normalization::strcmp_at(m.bytes, a - m.base, b - m.base)
}

fn strcmp_lit(m: &Memory<'_>, a: u32, b: &[u8]) -> i32 {
    let mut x = a;
    let mut i = 0usize;
    loop {
        let p = m.byte(x);
        let q = if i < b.len() { b[i] } else { 0 };
        if p != q {
            return p as i32 - q as i32;
        }
        if q == 0 {
            return 0;
        }
        x += 1;
        i += 1;
    }
}

fn strncmp_lit(m: &Memory<'_>, a: u32, b: &[u8], n: u32) -> i32 {
    if n == 0 {
        return 0;
    }
    let mut x = a;
    let mut i = 0usize;
    let mut left = n;
    loop {
        left -= 1;
        if left == 0 {
            break;
        }
        let p = m.byte(x);
        if p == 0 {
            break;
        }
        let q = if i < b.len() { b[i] } else { 0 };
        if p != q {
            break;
        }
        x += 1;
        i += 1;
    }
    let p = m.byte(x);
    let q = if i < b.len() { b[i] } else { 0 };
    p as i32 - q as i32
}

fn strchr(m: &mut Memory<'_>, s: u32, c: u8) -> u32 {
    let at = crate::fe_text::strchr_195d30(m.bytes, s - m.base, c as u32);
    if at == 0 {
        0
    } else {
        at + m.base
    }
}

fn memset(m: &mut Memory<'_>, dst: u32, value: u8, n: u32) {
    crate::fe_text::memset_195bec(m.bytes, dst - m.base, value, n);
}

fn copy_bounded(m: &mut Memory<'_>, dst: u32, src: u32, n: u32) {
    let mut d = dst;
    let mut s = src;
    let mut left = n;
    loop {
        if left == 0 {
            return;
        }
        let b = m.byte(s);
        s += 1;
        let at = m.offset(d);
        m.bytes[at] = b;
        d += 1;
        if b == 0 {
            break;
        }
        left -= 1;
    }
    if left > 0 {
        left -= 1;
        while left > 0 {
            let at = m.offset(d);
            m.bytes[at] = 0;
            d += 1;
            left -= 1;
        }
    }
}

fn itoa10(value: u32, buf: &mut [u8; 8]) {
    if value == 0 {
        buf[0] = b'0';
        buf[1] = 0;
        return;
    }
    let mut p = 0usize;
    let mut v = value;
    while v != 0 {
        buf[p] = b'0' + (v % 10) as u8;
        p += 1;
        v /= 10;
    }
    buf[p] = 0;
    let mut i = 0usize;
    let mut j = p - 1;
    while i < j {
        buf.swap(i, j);
        i += 1;
        j -= 1;
    }
}

fn alloc(m: &mut Memory<'_>, heap: u32, size: u32, h: &mut impl Host) -> u32 {
    crate::boot_part_e::alloc(m, heap, size, h)
}

fn calloc(m: &mut Memory<'_>, heap: u32, count: u32, size: u32, h: &mut impl Host) -> u32 {
    crate::boot_part_e::calloc(m, heap, count, size, h)
}

fn realloc(m: &mut Memory<'_>, heap: u32, old: u32, size: u32, h: &mut impl Host) -> u32 {
    crate::boot_part_e::fn_1963e0(m, heap, old, size, h)
}

fn release(m: &mut Memory<'_>, heap: u32, p: u32, h: &mut impl Host) {
    crate::boot_part_e::release(m, heap, p, h);
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Which {
    Main,
    System,
    Language,
}

impl Which {
    fn offset(self) -> u32 {
        match self {
            Which::Main => ctx::MAIN,
            Which::System => ctx::SYSTEM,
            Which::Language => ctx::LANGUAGE,
        }
    }
}

fn find_dictionary(m: &Memory<'_>, ctx: u32, key: u32) -> i16 {
    let count = m.half(ctx + ctx::DICTIONARY_COUNT);
    let keys = m.word(ctx + ctx::KEYS);
    for i in 0..count {
        let name = m.word(keys + i as u32 * 4);
        if strcmp(m, key, name) == 0 {
            return i as i16;
        }
    }
    -1
}

pub fn fn_1ea4e0(
    m: &mut Memory<'_>,
    ctx: u32,
    handle: u32,
    key: u32,
    text: u32,
    out: u32,
    count: u32,
    separator: u32,
    h: &mut impl Host,
    d: &mut impl Dictionaries,
) -> u32 {
    let mut rc = crate::frontend::validate_handle(ctx != 0, handle, 0xf37f, 0x2a4);
    if rc >= 0x80000000 {
        return 0x89802008;
    }
    let owner = m.word(ctx + ctx::OWNER);
    let heap = m.word(owner + owner::HEAP);
    let logger = m.word(owner + owner::LOGGER);
    h.call(m, 0x1738e8, &[logger, 0x8f8f44, 5, 0, 0]);
    if key == 0 || text == 0 || out == 0 || count == 0 || separator == 0 {
        h.call(m, 0x173dc0, &[logger, 0x8f8f44, 0xa029, 0]);
        return done(m, logger, h, 0x89802007);
    }
    let mut lookup_key = 0u32;
    let mut dictionary = Which::Main;
    let mut pending = false;
    let mut maximum = 0u32;
    let first = m.half(count);
    if first == 1 {
        lookup_key = alloc(m, heap, strlen(m, key) + 2, h);
        if lookup_key == 0 {
            h.call(m, 0x173dc0, &[logger, 0x8f8f44, 0xa028, 0]);
            return done(m, logger, h, 0x8980200a);
        }
        strcpy(m, lookup_key, key);
        append(m, lookup_key, b"C");
        if strcmp_lit(m, key, b"lidcfg") == 0
            || strncmp_lit(m, key, b"mertfrq", 7) == 0
            || strncmp_lit(m, key, b"merwfrq", 7) == 0
        {
            if m.word(ctx + ctx::LANGUAGE) == 0 {
                return cleanup(
                    m, ctx, logger, h, d, dictionary, false, lookup_key, key, out, count,
                    0x89802007,
                );
            }
            dictionary = Which::Language;
        } else {
            if m.word(ctx + ctx::SYSTEM) == 0 {
                return cleanup(
                    m, ctx, logger, h, d, dictionary, false, lookup_key, key, out, count,
                    0x89802007,
                );
            }
            dictionary = Which::System;
        }
    } else {
        lookup_key = key;
    }
    let mut multiple = true;
    if first == 0xffff || first == 1 {
        multiple = false;
        m.put_half(count, 0);
    }
    let index = find_dictionary(m, ctx, lookup_key);
    if index >= 0 {
        let tables = m.word(ctx + ctx::TABLES);
        let slot = tables + (index as u32) * 4;
        m.put_word(ctx + dictionary.offset() + 4 + state::TABLES, m.word(slot));
    } else {
        pending = true;
        let count_dictionaries = m.half(ctx + ctx::DICTIONARY_COUNT);
        let p = realloc(
            m,
            heap,
            m.word(ctx + ctx::RESOURCES),
            (count_dictionaries as u32 + 2) * 4,
            h,
        );
        if p == 0 {
            h.call(m, 0x173dc0, &[logger, 0x8f8f44, 0xa028, 0]);
            return cleanup(
                m, ctx, logger, h, d, dictionary, true, lookup_key, key, out, count, 0x8980200a,
            );
        }
        m.put_word(ctx + ctx::RESOURCES, p);
        m.put_word(p + count_dictionaries as u32 * 4, 0);
        m.put_word(p + (count_dictionaries as u32 + 1) * 4, 0);
        let mut namebuf = [0u8; 256];
        let resource_name: &[u8] = match dictionary {
            Which::System => b"sysdct",
            Which::Language => b"lid",
            Which::Main => {
                let mut n = 0usize;
                while n < 256 {
                    let b = m.byte(ctx + ctx::NAME + n as u32);
                    namebuf[n] = b;
                    if b == 0 {
                        break;
                    }
                    n += 1;
                }
                &namebuf[..n]
            }
        };
        let mut resource = 0u32;
        rc = h.resource_open(m, m.word(owner + owner::RESOURCES), resource_name, &mut resource);
        m.put_word(p + count_dictionaries as u32 * 4, resource);
        if rc >= 0x80000000 {
            return cleanup(
                m, ctx, logger, h, d, dictionary, true, lookup_key, key, out, count, rc,
            );
        }
        let tables = realloc(
            m,
            heap,
            m.word(ctx + ctx::TABLES),
            (count_dictionaries as u32 + 2) * 4,
            h,
        );
        if tables == 0 {
            h.call(m, 0x173dc0, &[logger, 0x8f8f44, 0xa028, 0]);
            return cleanup(
                m, ctx, logger, h, d, dictionary, true, lookup_key, key, out, count, 0x8980200a,
            );
        }
        m.put_word(ctx + ctx::TABLES, tables);
        m.put_word(tables + count_dictionaries as u32 * 4, 0);
        m.put_word(tables + (count_dictionaries as u32 + 1) * 4, 0);
        let block = calloc(m, heap, 1, 0x4c, h);
        if block == 0 {
            h.call(m, 0x173dc0, &[logger, 0x8f8f44, 0xa028, 0]);
            return cleanup(
                m, ctx, logger, h, d, dictionary, true, lookup_key, key, out, count, 0x8980200a,
            );
        }
        m.put_word(tables + count_dictionaries as u32 * 4, block);
        let st = ctx + dictionary.offset() + 4;
        m.put_word(st + state::TABLES, block);
        let resource = m.word(m.word(ctx + ctx::RESOURCES) + count_dictionaries as u32 * 4);
        let size = m.word(ctx + dictionary.offset());
        rc = d.load(m, st, key, size, resource);
        if rc >= 0x80000000 {
            if rc == 0x89802014 {
                let tables = m.word(ctx + ctx::TABLES);
                let block = m.word(tables + count_dictionaries as u32 * 4);
                release(m, heap, block, h);
                m.put_word(tables + count_dictionaries as u32 * 4, 0);
                let resources = m.word(ctx + ctx::RESOURCES);
                let handle = m.word(resources + count_dictionaries as u32 * 4);
                h.call(m, 0x179f3c, &[m.word(owner + owner::RESOURCES), handle]);
                m.put_word(resources + count_dictionaries as u32 * 4, 0);
                rc = 0;
            }
            return cleanup(m, ctx, logger, h, d, dictionary, true, lookup_key, key, out, count, rc);
        }
        let keys = realloc(
            m,
            heap,
            m.word(ctx + ctx::KEYS),
            (count_dictionaries as u32 + 2) * 4,
            h,
        );
        if keys == 0 {
            h.call(m, 0x173dc0, &[logger, 0x8f8f44, 0xa028, 0]);
            return cleanup(
                m, ctx, logger, h, d, dictionary, true, lookup_key, key, out, count, 0x8980200a,
            );
        }
        m.put_word(ctx + ctx::KEYS, keys);
        m.put_word(keys + count_dictionaries as u32 * 4, 0);
        m.put_word(keys + (count_dictionaries as u32 + 1) * 4, 0);
        let name = alloc(m, heap, strlen(m, lookup_key) + 1, h);
        if name == 0 {
            h.call(m, 0x173dc0, &[logger, 0x8f8f44, 0xa028, 0]);
            return cleanup(
                m, ctx, logger, h, d, dictionary, true, lookup_key, key, out, count, 0x8980200a,
            );
        }
        m.put_word(keys + count_dictionaries as u32 * 4, name);
        strcpy(m, name, lookup_key);
        let st = ctx + dictionary.offset() + 4;
        maximum = fn_26bb44(m, st) as u32;
        if maximum == 0 {
            h.call(m, 0x173dc0, &[logger, 0x8f8f44, 0xa029, 0]);
            return cleanup(
                m, ctx, logger, h, d, dictionary, true, lookup_key, key, out, count, 0x89802007,
            );
        }
        if (m.half(ctx + ctx::RESULT_CAPACITY) as u32) < maximum {
            let p = realloc(
                m,
                heap,
                m.word(ctx + dictionary.offset() + 4 + state::OUTPUT),
                maximum + 1,
                h,
            );
            if p == 0 {
                h.call(m, 0x173dc0, &[logger, 0x8f8f44, 0xa028, 0]);
                return cleanup(
                    m, ctx, logger, h, d, dictionary, true, lookup_key, key, out, count, 0x8980200a,
                );
            }
            m.put_word(ctx + ctx::MAIN + 4 + state::OUTPUT, p);
            m.put_word(ctx + ctx::SYSTEM + 4 + state::OUTPUT, p);
            m.put_word(ctx + ctx::LANGUAGE + 4 + state::OUTPUT, p);
        }
        pending = false;
        m.put_half(ctx + ctx::DICTIONARY_COUNT, count_dictionaries + 1);
    }
    let st = ctx + dictionary.offset() + 4;
    let sep = fn_26bb54(m, st);
    let at = m.offset(separator);
    m.bytes[at] = sep;
    let needed = strlen(m, text) + 5;
    if (m.half(ctx + ctx::QUERY_CAPACITY) as i16) < (needed as u16 as i16) {
        let p = realloc(m, heap, m.word(ctx + ctx::QUERY), needed, h);
        if p == 0 {
            h.call(m, 0x173dc0, &[logger, 0x8f8f44, 0xa028, 0]);
            return cleanup(
                m, ctx, logger, h, d, dictionary, true, lookup_key, key, out, count, 0x8980200a,
            );
        }
        m.put_half(ctx + ctx::QUERY_CAPACITY, needed as u16);
        m.put_word(ctx + ctx::QUERY, p);
    }
    m.put_half(count, 0);
    m.put_word(out, m.word(ctx + ctx::RESULTS));
    if (m.half(ctx + ctx::RESULT_CAPACITY) as u32) < maximum {
        let blocks = m.half(ctx + ctx::RESULT_BLOCKS) as i32;
        let mut i = 0i32;
        while i < blocks * 5 - 1 {
            let results = m.word(ctx + ctx::RESULTS);
            let p = realloc(m, heap, m.word(results + (i as u32) * 4), maximum + 1, h);
            if p == 0 {
                h.call(m, 0x173dc0, &[logger, 0x8f8f44, 0xa028, 0]);
                return cleanup(
                    m, ctx, logger, h, d, dictionary, true, lookup_key, key, out, count, 0x8980200a,
                );
            }
            let results = m.word(ctx + ctx::RESULTS);
            m.put_word(results + (i as u32) * 4, p);
            i += 1;
        }
        m.put_half(ctx + ctx::RESULT_CAPACITY, maximum as u16);
    }
    loop {
        if m.half(count) != 0 && !multiple {
            break;
        }
        let mut start = 0i16;
        strcpy(m, m.word(ctx + ctx::QUERY), text);
        if m.half(count) != 0 {
            append(m, m.word(ctx + ctx::QUERY), &[0x17]);
            let mut number = [0u8; 8];
            itoa10(m.half(count) as u32, &mut number);
            let mut len = 0usize;
            while number[len] != 0 {
                len += 1;
            }
            append(m, m.word(ctx + ctx::QUERY), &number[..len]);
        }
        let mut end = (strlen(m, m.word(ctx + ctx::QUERY)) as i32 - 1) as i16;
        let mut length = 0i16;
        let found = d.lookup(
            m,
            st,
            m.word(ctx + ctx::QUERY),
            &mut start,
            &mut end,
            0,
            &mut length,
        );
        if found == 0 {
            break;
        }
        let blocks = m.half(ctx + ctx::RESULT_BLOCKS) as u32;
        if m.half(count) as u32 + 2 > blocks * 5 {
            let p = realloc(m, heap, m.word(ctx + ctx::RESULTS), (blocks + 1) * 20, h);
            if p == 0 {
                h.call(m, 0x173dc0, &[logger, 0x8f8f44, 0xa028, 0]);
                return cleanup(
                    m, ctx, logger, h, d, dictionary, true, lookup_key, key, out, count, 0x8980200a,
                );
            }
            m.put_word(ctx + ctx::RESULTS, p);
            let mut i: u16 = if blocks == 0 {
                0
            } else {
                (blocks * 5 - 1) as u16
            };
            let results = m.word(ctx + ctx::RESULTS);
            let span = (((blocks + 1) * 5) as u16 as i32 - i as i32) as u32 * 4;
            memset(m, results + i as u32 * 4, 0, span);
            while i < (blocks * 5 + 4) as u16 {
                let p = alloc(m, heap, m.half(ctx + ctx::RESULT_CAPACITY) as u32 + 1, h);
                if p == 0 {
                    h.call(m, 0x173dc0, &[logger, 0x8f8f44, 0xa028, 0]);
                    return cleanup(
                        m, ctx, logger, h, d, dictionary, true, lookup_key, key, out, count, 0x8980200a,
                    );
                }
                let results = m.word(ctx + ctx::RESULTS);
                m.put_word(results + i as u32 * 4, p);
                i += 1;
            }
            let results = m.word(ctx + ctx::RESULTS);
            m.put_word(results + i as u32 * 4, 0);
            m.put_half(ctx + ctx::RESULT_BLOCKS, (blocks + 1) as u16);
        }
        let output = m.word(st + state::OUTPUT);
        let results = m.word(ctx + ctx::RESULTS);
        let slot = m.word(results + m.half(count) as u32 * 4);
        let mut j = 0i16;
        while j < length {
            let b = m.byte(output + j as u32);
            let at = m.offset(slot + j as u32);
            m.bytes[at] = b;
            j += 1;
        }
        let at = m.offset(slot + j as u32);
        m.bytes[at] = 0;
        m.put_half(count, m.half(count) + 1);
    }
    cleanup(m, ctx, logger, h, d, dictionary, false, lookup_key, key, out, count, 0)
}

#[allow(clippy::too_many_arguments)]
fn cleanup(
    m: &mut Memory<'_>,
    ctx: u32,
    logger: u32,
    h: &mut impl Host,
    d: &mut impl Dictionaries,
    dictionary: Which,
    pending: bool,
    lookup_key: u32,
    key: u32,
    out: u32,
    count: u32,
    rc: u32,
) -> u32 {
    let owner = m.word(ctx + ctx::OWNER);
    let heap = m.word(owner + owner::HEAP);
    if lookup_key != 0 && lookup_key != key {
        release(m, heap, lookup_key, h);
    }
    if rc < 0x80000000 {
        if m.half(count) != 0 {
            m.put_word(out, m.word(ctx + ctx::RESULTS));
        }
        return done(m, logger, h, rc);
    }
    if !pending {
        return done(m, logger, h, rc);
    }
    let count_dictionaries = m.half(ctx + ctx::DICTIONARY_COUNT);
    let keys = m.word(ctx + ctx::KEYS);
    if keys != 0 {
        let key_block = m.word(keys + count_dictionaries as u32 * 4);
        if key_block != 0 {
            release(m, heap, key_block, h);
            m.put_word(keys + count_dictionaries as u32 * 4, 0);
        }
    }
    let tables = m.word(ctx + ctx::TABLES);
    if tables != 0 {
        let block = m.word(tables + count_dictionaries as u32 * 4);
        if block != 0 {
            let resources = m.word(ctx + ctx::RESOURCES);
            let resource = if resources != 0 {
                m.word(resources + count_dictionaries as u32 * 4)
            } else {
                0
            };
            if resource != 0 {
                let st = ctx + dictionary.offset() + 4;
                m.put_word(st + state::TABLES, block);
                fn_26bb28(m, st, resource, h);
            }
            release(m, heap, block, h);
            m.put_word(tables + count_dictionaries as u32 * 4, 0);
        }
    }
    let resources = m.word(ctx + ctx::RESOURCES);
    if resources != 0 {
        let handle = m.word(resources + count_dictionaries as u32 * 4);
        if handle != 0 {
            h.call(m, 0x179f3c, &[m.word(owner + owner::RESOURCES), handle]);
            m.put_word(resources + count_dictionaries as u32 * 4, 0);
        }
    }
    done(m, logger, h, rc)
}

fn done(m: &mut Memory<'_>, logger: u32, h: &mut impl Host, rc: u32) -> u32 {
    h.call(m, 0x1738e8, &[logger, 0x8f8f44, 4, 0, 0]);
    rc
}

pub struct Scratch {
    pub base: u32,
}

impl Scratch {
    pub fn out0(&self) -> u32 {
        self.base
    }
    pub fn count0(&self) -> u32 {
        self.base + 4
    }
    pub fn sep0(&self) -> u32 {
        self.base + 6
    }
    pub fn out1(&self) -> u32 {
        self.base + 8
    }
    pub fn count1(&self) -> u32 {
        self.base + 12
    }
    pub fn sep1(&self) -> u32 {
        self.base + 14
    }
    pub fn fecfg(&self) -> u32 {
        self.base + 16
    }
    pub fn domdef(&self) -> u32 {
        self.base + 24
    }
    pub fn punc(&self) -> u32 {
        self.base + 32
    }
    pub fn canonmapdomain(&self) -> u32 {
        self.base + 40
    }
    pub fn normal(&self) -> u32 {
        self.base + 56
    }
    pub fn sysdct(&self) -> u32 {
        self.base + 64
    }
    pub fn lid(&self) -> u32 {
        self.base + 72
    }
    pub fn text(&self) -> u32 {
        self.base + 80
    }

    pub fn token_name(&self) -> u32 {
        self.base + 88
    }

    pub fn keybuf(&self) -> u32 {
        self.base + 336
    }

    pub fn slot(&self) -> u32 {
        self.base + 592
    }

    pub fn map_key(&self) -> u32 {
        self.base + 600
    }

    pub fn host_out(&self) -> u32 {
        self.base + 664
    }

    pub fn host_out2(&self) -> u32 {
        self.base + 668
    }

    pub fn setting_slot(&self) -> u32 {
        self.base + 672
    }

    pub fn env_scratch(&self) -> u32 {
        self.base + 680
    }

    pub const SIZE: u32 = 720;
}

fn l_len(b: &[u8]) -> usize {
    b.iter().position(|&c| c == 0).unwrap_or(b.len())
}

fn l_copy(dst: &mut [u8], src: &[u8]) {
    let mut i = 0usize;
    while i < dst.len() {
        let c = if i < src.len() { src[i] } else { 0 };
        dst[i] = c;
        if c == 0 {
            break;
        }
        i += 1;
    }
}

fn l_append(dst: &mut [u8], src: &[u8]) {
    let mut at = l_len(dst);
    let mut i = 0usize;
    while at < dst.len() {
        let c = if i < src.len() { src[i] } else { 0 };
        dst[at] = c;
        if c == 0 {
            break;
        }
        at += 1;
        i += 1;
    }
}

fn l_chr(b: &[u8], c: u8) -> Option<usize> {
    b.iter().position(|&x| x == c)
}

fn l_copy_bounded(dst: &mut [u8], src: &[u8], n: usize) {
    let mut i = 0usize;
    let mut left = n;
    while i < dst.len() {
        if left == 0 {
            return;
        }
        let c = if i < src.len() { src[i] } else { 0 };
        dst[i] = c;
        if c == 0 {
            break;
        }
        i += 1;
        left -= 1;
    }
    while i < dst.len() && left > 0 {
        dst[i] = 0;
        i += 1;
        left -= 1;
    }
}

fn fn_1763e4(m: &mut Memory<'_>, dst: u32, src: &[u8]) {
    let mut at = dst;
    let mut i = 0usize;
    loop {
        let c = if i < src.len() { src[i] } else { 0 } as u16;
        m.put_half(at, c);
        if c == 0 {
            break;
        }
        at += 2;
        i += 1;
    }
}

pub fn fn_1eb56c(m: &mut Memory<'_>, ctx: u32, h: &mut impl Host) -> u32 {
    if ctx == 0 {
        return 0x89802007;
    }
    let owner = m.word(ctx + ctx::OWNER);
    let heap = m.word(owner + owner::HEAP);
    let resources = m.word(owner + owner::RESOURCES);
    if m.word(ctx + ctx::QUERY) != 0 {
        release(m, heap, m.word(ctx + ctx::QUERY), h);
        m.put_word(ctx + ctx::QUERY, 0);
    }
    if m.word(ctx + ctx::MAIN + 4 + state::OUTPUT) != 0 {
        release(m, heap, m.word(ctx + ctx::MAIN + 4 + state::OUTPUT), h);
        m.put_word(ctx + ctx::MAIN + 4 + state::OUTPUT, 0);
    }
    if m.word(ctx + ctx::RESULTS) != 0 {
        let results = m.word(ctx + ctx::RESULTS);
        if m.word(results) != 0 {
            let mut i = 0u32;
            loop {
                release(m, heap, m.word(results + i * 4), h);
                m.put_word(results + i * 4, 0);
                i += 1;
                if m.word(results + i * 4) == 0 {
                    break;
                }
            }
        }
        release(m, heap, results, h);
        m.put_word(ctx + ctx::RESULTS, 0);
    }
    if m.word(ctx + ctx::KEYS) != 0 {
        let keys = m.word(ctx + ctx::KEYS);
        if m.word(keys) != 0 {
            let mut i = 0u32;
            loop {
                release(m, heap, m.word(keys + i * 4), h);
                m.put_word(keys + i * 4, 0);
                i += 1;
                if m.word(keys + i * 4) == 0 {
                    break;
                }
            }
        }
        release(m, heap, keys, h);
        m.put_word(ctx + ctx::KEYS, 0);
    }
    if m.word(ctx + ctx::TABLES) != 0 {
        let tables = m.word(ctx + ctx::TABLES);
        if m.word(tables) != 0 {
            let mut i = 0u32;
            loop {
                let block = m.word(tables + i * 4);
                m.put_word(ctx + ctx::MAIN + 4 + state::TABLES, block);
                fn_26bb28(m, ctx + ctx::MAIN + 4, m.word(m.word(ctx + ctx::RESOURCES) + i * 4), h);
                release(m, heap, block, h);
                m.put_word(tables + i * 4, 0);
                i += 1;
                if m.word(tables + i * 4) == 0 {
                    break;
                }
            }
        }
        release(m, heap, tables, h);
        m.put_word(ctx + ctx::TABLES, 0);
    }
    if m.word(ctx + ctx::RESOURCES) != 0 {
        let list = m.word(ctx + ctx::RESOURCES);
        if m.word(list) != 0 {
            let mut i = 0u32;
            loop {
                h.call(m, 0x179f3c, &[resources, m.word(list + i * 4)]);
                m.put_word(list + i * 4, 0);
                i += 1;
                if m.word(list + i * 4) == 0 {
                    break;
                }
            }
        }
        release(m, heap, list, h);
        m.put_word(ctx + ctx::RESOURCES, 0);
    }
    if m.word(ctx + ctx::HANDLE7C) != 0 {
        if m.word(ctx + ctx::HANDLE80) != 0 {
            h.call(
                m,
                0x179208,
                &[resources, m.word(ctx + ctx::HANDLE7C), m.word(ctx + ctx::HANDLE80)],
            );
            m.put_word(ctx + ctx::HANDLE80, 0);
        }
        h.call(m, 0x179f3c, &[resources, m.word(ctx + ctx::HANDLE7C)]);
        m.put_word(ctx + ctx::HANDLE7C, 0);
    }
    m.put_half(ctx + ctx::DICTIONARY_COUNT, 0);
    m.put_half(ctx + ctx::RESULT_BLOCKS, 0);
    m.put_half(ctx + ctx::RESULT_CAPACITY, 0);
    m.put_half(ctx + ctx::QUERY_CAPACITY, 0);
    0
}

pub fn fn_1ee6f4(m: &mut Memory<'_>, ctx: u32, h: &mut impl Host) -> u32 {
    let count = m.half(ctx + ctx::TOKEN_COUNT);
    if count == 0 {
        return 0;
    }
    let owner = m.word(ctx + ctx::OWNER);
    let heap = m.word(owner + owner::HEAP);
    let arr = m.word(ctx + ctx::TOKEN_ENTRIES);
    for i in 0..count as u32 {
        let at = arr + i * 16;
        for off in [0u32, 4, 8, 12] {
            let v = m.word(at + off);
            if v != 0 {
                release(m, heap, v, h);
                m.put_word(at + off, 0);
            }
        }
    }
    release(m, heap, arr, h);
    m.put_word(ctx + ctx::TOKEN_ENTRIES, 0);
    m.put_half(ctx + ctx::TOKEN_COUNT, 0);
    0
}

pub fn fn_1eb7b8(m: &mut Memory<'_>, ctx: u32, handle: u32, h: &mut impl Host) -> u32 {
    let rc = crate::frontend::validate_handle(ctx != 0, handle, 0xf37f, 0x2a4);
    if rc >= 0x80000000 {
        return 0x89802008;
    }
    if ctx == 0 {
        return 0x89802008;
    }
    let owner = m.word(ctx + ctx::OWNER);
    let heap = m.word(owner + owner::HEAP);
    let logger = m.word(owner + owner::LOGGER);
    h.call(m, 0x1738e8, &[logger, 0x8f8f44, 4, 0, 0]);
    h.resource_close(m, m.word(owner + owner::FIELD18), b"LINGDB");
    let rc = fn_1eb56c(m, ctx, h);
    if rc >= 0x80000000 {
        return rc;
    }
    for which in [Which::Main, Which::System, Which::Language] {
        fn_26b6fc(m, ctx + which.offset() + 4, h);
    }
    if m.word(ctx + ctx::FIELD64) != 0 {
        release(m, heap, m.word(ctx + ctx::FIELD64), h);
    }
    let rc = fn_1ee6f4(m, ctx, h);
    if rc >= 0x80000000 {
        return rc;
    }
    for off in [ctx::TABLE90, ctx::TABLE2A0] {
        if m.word(ctx + off) != 0 {
            let table = m.word(ctx + off);
            for i in 0..176u32 {
                let v = m.word(table + i * 4);
                if v != 0 {
                    release(m, heap, v, h);
                }
            }
            release(m, heap, table, h);
        }
    }
    h.call(m, 0x1738e8, &[logger, 0x8f8f44, 4, 0, 0]);
    release(m, heap, ctx, h);
    0
}

pub fn fn_1eae98(
    m: &mut Memory<'_>,
    ctx: u32,
    handle: u32,
    aux: u32,
    s: &Scratch,
    h: &mut impl Host,
    d: &mut impl Dictionaries,
) -> u32 {
    let aux_owner = m.word(aux + ctx::OWNER);
    let aux_heap = m.word(aux_owner + owner::HEAP);
    let aux_logger = m.word(aux_owner + owner::LOGGER);
    m.put_half(aux + ctx::TOKEN_COUNT, 0);
    let mut out = s.out0();
    let mut count = s.count0();
    let mut sep = s.sep0();

    m.put_half(count, 0xffff);
    let mut rc = fn_1ea4e0(
        m,
        ctx,
        handle,
        s.fecfg(),
        s.domdef(),
        out,
        count,
        sep,
        h,
        d,
    );
    let first_rc = rc;
    if rc >= 0x80000000 {
        return rc;
    }
    if m.half(count) != 1 {
        return first_rc;
    }
    let mut buf = [0u8; 256];
    let result = m.word(out);
    let answer = m.word(result);
    let (bytes, len) = guest_bytes(m, answer);
    l_copy(&mut buf, &bytes[..len]);
    let hit = l_chr(&buf, m.byte(sep));
    if let Some(i) = hit {
        buf[i] = 0;
    }
    if strlen(m, answer) >= 256 {
        h.call(m, 0x173dc0, &[aux_logger, 0x8f8f44, 0xa02b, 0]);
        return 0x89802000;
    }
    if buf[0] == 0 {
        return first_rc;
    }
    let mut pos = 0usize;
    let mut start = 0usize;
    loop {
        while pos < buf.len() && buf[pos] != 0 && buf[pos] != b',' {
            pos += 1;
        }
        if pos != start {
            let mut token = [0u8; 256];
            l_copy_bounded(&mut token, &buf[start..pos], pos - start);
            token[pos - start] = 0;
            let index = m.half(aux + ctx::TOKEN_COUNT);
            if index != 0 {
                let p = realloc(
                    m,
                    aux_heap,
                    m.word(aux + ctx::TOKEN_ENTRIES),
                    index as u32 * 16 + 16,
                    h,
                );
                if p == 0 {
                    return 0x8980200a;
                }
                m.put_word(aux + ctx::TOKEN_ENTRIES, p);
            } else {
                let p = calloc(m, aux_heap, 16, 1, h);
                if p == 0 {
                    return 0x8980200a;
                }
                m.put_word(aux + ctx::TOKEN_ENTRIES, p);
            }
            let entries = m.word(aux + ctx::TOKEN_ENTRIES);
            let at = entries + index as u32 * 16;
            m.put_word(at, 0);
            m.put_word(at + 4, 0);
            m.put_word(at + 8, 0);
            m.put_word(at + 12, 0);
            let p = calloc(m, aux_heap, ((l_len(&token) + 1) * 2) as u32, 1, h);
            if p == 0 {
                m.put_half(aux + ctx::TOKEN_COUNT, index + 1);
                return 0x8980200a;
            }
            m.put_word(at, p);
            fn_1763e4(m, p, &token);

            let text = s.text();
            l_copy_to(m, text, &token);
            append_bytes(m, text, b"_alpha");
            out = s.out1();
            count = s.count1();
            sep = s.sep1();

            m.put_half(count, 0);
            rc = fn_1ea4e0(m, ctx, handle, s.fecfg(), text, out, count, sep, h, d);
            if rc >= 0x80000000 {
                m.put_half(aux + ctx::TOKEN_COUNT, index + 1);
                return rc;
            }
            if m.half(count) == 1 {
                let value = m.word(m.word(out));
                let (mut text_bytes, len) = guest_bytes(m, value);
                let hit = l_chr(&text_bytes[..len], m.byte(sep));
                if let Some(i) = hit {
                    text_bytes[i] = 0;
                }
                if strlen(m, value) >= 256 {
                    h.call(m, 0x173dc0, &[aux_logger, 0x8f8f44, 0xa02b, 0]);
                    m.put_half(aux + ctx::TOKEN_COUNT, index + 1);
                    return 0x89802000;
                }
                let p = calloc(m, aux_heap, strlen(m, value) + 1, 1, h);
                if p == 0 {
                    m.put_half(aux + ctx::TOKEN_COUNT, index + 1);
                    return 0x8980200a;
                }
                m.put_word(at + 4, p);
                l_copy_to(m, p, &text_bytes[..len]);
            }

            l_copy_to(m, text, &token);
            append_bytes(m, text, b"_digit");
            out = s.out1();
            count = s.count1();
            sep = s.sep1();

            m.put_half(count, 0);
            rc = fn_1ea4e0(m, ctx, handle, s.fecfg(), text, out, count, sep, h, d);
            if rc >= 0x80000000 {
                m.put_half(aux + ctx::TOKEN_COUNT, index + 1);
                return rc;
            }
            if m.half(count) == 1 {
                let value = m.word(m.word(out));
                let (mut text_bytes, len) = guest_bytes(m, value);
                let hit = l_chr(&text_bytes[..len], m.byte(sep));
                if let Some(i) = hit {
                    text_bytes[i] = 0;
                }
                if strlen(m, value) >= 256 {
                    h.call(m, 0x173dc0, &[aux_logger, 0x8f8f44, 0xa02b, 0]);
                    m.put_half(aux + ctx::TOKEN_COUNT, index + 1);
                    return 0x89802000;
                }
                let p = calloc(m, aux_heap, strlen(m, value) + 1, 1, h);
                if p == 0 {
                    m.put_half(aux + ctx::TOKEN_COUNT, index + 1);
                    return 0x89802000;
                }
                m.put_word(at + 8, p);
                l_copy_to(m, p, &text_bytes[..len]);
            }

            l_copy_to(m, text, &token);
            append_bytes(m, text, b"_punc");
            out = s.out1();
            count = s.count1();
            sep = s.sep1();

            m.put_half(count, 0);
            rc = fn_1ea4e0(m, ctx, handle, s.fecfg(), text, out, count, sep, h, d);
            if rc >= 0x80000000 {
                m.put_half(aux + ctx::TOKEN_COUNT, index + 1);
                return rc;
            }
            if m.half(count) == 1 {
                let value = m.word(m.word(out));
                let (mut text_bytes, len) = guest_bytes(m, value);
                let hit = l_chr(&text_bytes[..len], m.byte(sep));
                if let Some(i) = hit {
                    text_bytes[i] = 0;
                }
                if strlen(m, value) >= 256 {
                    h.call(m, 0x173dc0, &[aux_logger, 0x8f8f44, 0xa02b, 0]);
                    m.put_half(aux + ctx::TOKEN_COUNT, index + 1);
                    return 0x89802000;
                }
                let p = calloc(m, aux_heap, strlen(m, value) + 1, 1, h);
                if p == 0 {
                    m.put_half(aux + ctx::TOKEN_COUNT, index + 1);
                    return 0x8980200a;
                }
                m.put_word(at + 12, p);
                l_copy_to(m, p, &text_bytes[..len]);
            }
            if m.word(at + 4) == 0 && m.word(at + 8) == 0 && m.word(at + 12) == 0 {
                h.call(m, 0x173dc0, &[aux_logger, 0x8f8f44, 0xa02c, 0]);
                m.put_half(aux + ctx::TOKEN_COUNT, index + 1);
                return 0x89802000;
            }
            pos += 1;
            m.put_half(aux + ctx::TOKEN_COUNT, index + 1);
            start = pos;
        }
        if pos < buf.len() && buf[pos] == 0 {
            return first_rc;
        }
    }
}

fn guest_bytes(m: &Memory<'_>, p: u32) -> ([u8; 256], usize) {
    let mut at = p as usize;
    let mut out = [0u8; 256];
    let mut n = 0usize;
    while n < 256 && m.bytes[at] != 0 {
        out[n] = m.bytes[at];
        at += 1;
        n += 1;
    }
    (out, n)
}

fn l_copy_to(m: &mut Memory<'_>, dst: u32, src: &[u8]) {
    let mut at = dst;
    for c in src {
        let o = m.offset(at);
        m.bytes[o] = *c;
        at += 1;
    }
    let o = m.offset(at);
    m.bytes[o] = 0;
}

fn append_bytes(m: &mut Memory<'_>, dst: u32, src: &[u8]) {
    let mut at = dst + strlen(m, dst);
    for c in src {
        let o = m.offset(at);
        m.bytes[o] = *c;
        at += 1;
    }
    let o = m.offset(at);
    m.bytes[o] = 0;
}

pub fn fn_1eb978(
    m: &mut Memory<'_>,
    ctx: u32,
    handle: u32,
    name: u32,
    s: &Scratch,
    h: &mut impl Host,
    d: &mut impl Dictionaries,
) -> u32 {
    if ctx == 0 {
        return 0x89802008;
    }
    let owner = m.word(ctx + ctx::OWNER);
    let heap = m.word(owner + owner::HEAP);
    let logger = m.word(owner + owner::LOGGER);
    let key = s.keybuf();
    let mut out = s.out0();
    let mut count = s.count0();
    let mut sep = s.sep0();

    m.put_half(count, 0);

    for phase in 0..2u32 {
        let (prefix, plain, table): (&[u8], &[u8], u32) = if phase == 0 {
            (b"canonmap_", b"canonmap", ctx::TABLE90)
        } else {
            (b"canonmapg2pdct_", b"canonmapg2pdct", ctx::TABLE2A0)
        };
        l_copy_to(m, key, prefix);
        append_bytes(m, key, &guest_bytes(m, name).0[..guest_bytes(m, name).1]);
        let mut rc = fn_1ea4e0(m, ctx, handle, s.fecfg(), key, out, count, sep, h, d);
        if rc >= 0x80000000 {
            return 0x89802007;
        }
        if m.half(count) == 0 {
            l_copy_to(m, key, plain);
            rc = fn_1ea4e0(m, ctx, handle, s.fecfg(), key, out, count, sep, h, d);
            if rc >= 0x80000000 {
                return 0x89802007;
            }
            if m.half(count) == 0 {
                continue;
            }
        }
        let mut cache = m.word(ctx + table);
        if cache == 0 {
            cache = alloc(m, heap, 704, h);
            if cache == 0 {
                h.call(m, 0x173dc0, &[logger, 0x8f8f44, 0xa028, 0]);
                return 0x8980200a;
            }
            m.put_word(ctx + table, cache);
            for k in 0..176u32 {
                m.put_word(cache + k * 4, 0);
            }
        }
        let n = m.half(count);
        for i in 0..n {
            let entry = m.word(m.word(out) + i as u32 * 4);
            let b = m.byte(entry);
            let text = entry + 2;
            if b >= 176 {
                h.call(m, 0x173dc0, &[logger, 0x8f8f44, 0xa029, 0]);
                return 0x89802007;
            }
            if m.word(cache + b as u32 * 4) == 0 {
                let p = alloc(m, heap, strlen(m, text) + 1, h);
                if p == 0 {
                    h.call(m, 0x173dc0, &[logger, 0x8f8f44, 0xa028, 0]);
                    return 0x8980200a;
                }
                m.put_word(cache + b as u32 * 4, p);
                strcpy(m, p, text);
            }
        }
    }

    m.put_half(count, 0xffff);
    l_copy_to(m, key, b"canonmapusage_");
    append_bytes(m, key, &guest_bytes(m, name).0[..guest_bytes(m, name).1]);
    let mut rc = fn_1ea4e0(m, ctx, handle, s.fecfg(), key, out, count, sep, h, d);
    if rc >= 0x80000000 {
        return 0x89802007;
    }
    if m.half(count) == 0 {
        l_copy_to(m, key, b"canonmapusage");
        m.put_half(count, 0xffff);
        rc = fn_1ea4e0(m, ctx, handle, s.fecfg(), key, out, count, sep, h, d);
        if rc >= 0x80000000 {
            return 0x89802007;
        }
    }
    if m.half(count) == 1 {
        let value = m.word(m.word(out));
        let hit = crate::fe_text::strchr_195d30(m.bytes, value - m.base, m.byte(sep) as u32);
        if hit != 0 {
            let at = m.offset(hit + m.base);
            m.bytes[at] = 0;
        }
        if strcmp_lit(m, value, b"all") == 0 {
            m.put_half(ctx + ctx::MODE9C, 0);
        }
        if strcmp_lit(m, value, b"nocaseconv") == 0 {
            m.put_half(ctx + ctx::MODE9C, 2);
        }
        if strcmp_lit(m, value, b"g2pdct") == 0 {
            m.put_half(ctx + ctx::MODE9C, 1);
        }
    }
    rc
}

#[allow(clippy::too_many_arguments)]
pub fn fn_1ec070(
    m: &mut Memory<'_>,
    _a0: u32,
    _a1: u32,
    h0: u32,
    h1: u32,
    slot: u32,
    s: &Scratch,
    k: &crate::boot_port_2::ConfigStrings,
    h: &mut impl Host,
    d: &mut impl Dictionaries,
) -> u32 {
    if slot == 0 {
        return 0x89802007;
    }
    m.put_word(slot, 0);
    m.put_word(slot + 4, 0);
    let mut owner = 0u32;
    let mut rc = h.context(m, h0, h1, &mut owner);
    if rc >= 0x80000000 {
        return rc;
    }
    let logger = m.word(owner + owner::LOGGER);
    let heap = m.word(owner + owner::HEAP);
    let resources = m.word(owner + owner::RESOURCES);
    h.call(m, 0x1738e8, &[logger, 0x8f8f44, 4, 0, 0]);
    let mut lk = 0u32;
    rc = h.open_named(m, m.word(owner + owner::FIELD18), b"LINGDB", &mut lk);
    if rc >= 0x80000000 {
        return rc;
    }
    let ctx = calloc(m, heap, 1, 0x2a4, h);
    if ctx == 0 {
        h.call(m, 0x173dc0, &[logger, 0x8f8f44, 0xa028, 0]);
        h.resource_close(m, m.word(owner + owner::FIELD18), b"LINGDB");
        return 0x8980200a;
    }
    m.put_word(slot + 4, 0xf37f);
    m.put_word(slot, ctx);
    m.put_word(ctx + ctx::RESERVED84, m.word(lk + 4));
    m.put_word(ctx + ctx::OWNER, owner);
    m.put_half(ctx + ctx::MODE9C, 1);
    m.put_byte(ctx + ctx::FLAG29E, 1);
    for which in [Which::Main, Which::System, Which::Language] {
        m.put_word(ctx + which.offset() + 4 + state::H0, resources);
    }
    let mut out = s.out0();
    let mut count = s.count0();
    let mut sep = s.sep0();
    rc = fn_1ebe84(m, owner, ctx + ctx::NAME, 0, k, h);
    if rc < 0x80000000 {
        let mut size = 0u32;
        rc = d.open(m, ctx + ctx::MAIN + 4, ctx + ctx::NAME, &mut size);
        if rc < 0x80000000 {
            m.put_word(ctx + ctx::MAIN, size);
        } else {
            rc = fn_1ebe84(m, owner, ctx + ctx::NAME, 1, k, h);
            if rc >= 0x80000000 {
                return cleanup_open(m, slot, logger, h, rc);
            }
            let mut size = 0u32;
            rc = d.open(m, ctx + ctx::MAIN + 4, ctx + ctx::NAME, &mut size);
            if rc >= 0x80000000 {
                return cleanup_open(m, slot, logger, h, rc);
            }
            m.put_word(ctx + ctx::MAIN, size);
        }
    } else {
        rc = fn_1ebe84(m, owner, ctx + ctx::NAME, 1, k, h);
        if rc >= 0x80000000 {
            return cleanup_open(m, slot, logger, h, rc);
        }
        let mut size = 0u32;
        rc = d.open(m, ctx + ctx::MAIN + 4, ctx + ctx::NAME, &mut size);
        if rc >= 0x80000000 {
            return cleanup_open(m, slot, logger, h, rc);
        }
        m.put_word(ctx + ctx::MAIN, size);
    }
    rc = open_system(m, ctx, s, h, d);
    if rc >= 0x80000000 {
        return cleanup_open(m, slot, logger, h, rc);
    }
    finish_open(m, ctx, slot, s, k, h, d, rc)
}

fn fn_1ebe84(
    m: &mut Memory<'_>,
    owner: u32,
    dst: u32,
    flag: u32,
    k: &crate::boot_port_2::ConfigStrings,
    h: &mut impl Host,
) -> u32 {
    let res = m.word(owner + owner::FIELD14);
    let mut lang = 0u32;
    let mut rc = h.parameter(m, res, b"langcode", &mut lang);
    if rc >= 0x80000000 {
        return rc;
    }
    let mut dir = 0u32;
    if flag == 0 {
        rc = h.parameter(m, res, b"fecfg", &mut dir);
        if rc >= 0x80000000 {
            return rc;
        }
    }
    let at = m.offset(dst);
    m.bytes[at] = 0;
    let mut prefix = 0u32;
    if h.parameter(m, res, b"fedataprefix", &mut prefix) < 0x80000000 {
        if prefix != 0 && m.byte(prefix) != 0 {
            strcpy(m, dst, prefix);
        }
    }
    let mut packaging = 0u32;
    if h.parameter(m, res, b"fedatapackaging", &mut packaging) < 0x80000000 {
        if packaging != 0 && strcmp(m, packaging, k.clc) == 0 {
            append_guest(m, dst, k.clc_slash);
            append_guest(m, dst, lang);
            append_guest(m, dst, k.slash);
            if dir != 0 {
                append_guest(m, dst, dir);
            }
            append_guest(m, dst, k.colon);
        }
    }
    append_guest(m, dst, k.sysdct_slash);
    append_guest(m, dst, lang);
    if flag == 0 {
        append_guest(m, dst, k.slash);
        if dir != 0 {
            append_guest(m, dst, dir);
        }
    }
    let n = strlen(m, dst);
    for i in 0..n {
        let at = m.offset(dst + i);
        m.bytes[at] = crate::normalization::tolower_240ba0(m.bytes[at] as u32) as u8;
    }
    rc
}

fn open_system(
    m: &mut Memory<'_>,
    ctx: u32,
    s: &Scratch,
    h: &mut impl Host,
    d: &mut impl Dictionaries,
) -> u32 {
    let mut size = 0u32;
    let rc = d.open(m, ctx + ctx::SYSTEM + 4, s.sysdct(), &mut size);
    if rc >= 0x80000000 {
        m.put_word(ctx + ctx::SYSTEM, 0);
    } else {
        m.put_word(ctx + ctx::SYSTEM, size);
    }
    let mut size = 0u32;
    let rc = d.open(m, ctx + ctx::LANGUAGE + 4, s.lid(), &mut size);
    if rc >= 0x80000000 {
        m.put_word(ctx + ctx::LANGUAGE, 0);
    } else {
        m.put_word(ctx + ctx::LANGUAGE, size);
    }
    0
}

fn finish_open(
    m: &mut Memory<'_>,
    ctx: u32,
    slot: u32,
    s: &Scratch,
    k: &crate::boot_port_2::ConfigStrings,
    h: &mut impl Host,
    d: &mut impl Dictionaries,
    _rc: u32,
) -> u32 {
    let owner = m.word(ctx + ctx::OWNER);
    let heap = m.word(owner + owner::HEAP);
    let logger = m.word(owner + owner::LOGGER);
    let handle = m.word(slot + 4);
    let mut out = s.out0();
    let mut count = s.count0();
    let mut sep = s.sep0();

    m.put_half(count, 0xffff);
    let rc = fn_1ea4e0(m, ctx, handle, s.fecfg(), s.punc(), out, count, sep, h, d);
    if rc >= 0x80000000 {
        return cleanup_open(m, slot, logger, h, rc);
    }
    if m.word(out) == 0 {
        return cleanup_open(m, slot, logger, h, 0x8980200a);
    }
    let value = m.word(m.word(out));
    let hit = crate::fe_text::strchr_195d30(m.bytes, value - m.base, m.byte(sep) as u32);
    if hit != 0 {
        let at = m.offset(hit + m.base);
        m.bytes[at] = 0;
    }
    let saved = alloc(m, heap, strlen(m, value) + 1, h);
    m.put_word(ctx + ctx::FIELD64, saved);
    if saved == 0 {
        h.call(m, 0x173dc0, &[logger, 0x8f8f44, 0xa028, 0]);
        return cleanup_open(m, slot, logger, h, 0x8980200a);
    }
    strcpy(m, saved, value);
    m.put_half(count, 0xffff);
    let rc = fn_1ea4e0(
        m,
        ctx,
        handle,
        s.fecfg(),
        s.canonmapdomain(),
        out,
        count,
        sep,
        h,
        d,
    );
    if rc >= 0x80000000 {
        return cleanup_open(m, slot, logger, h, rc);
    }
    if m.half(count) != 0 {
        m.put_byte(ctx + ctx::FLAG29E, 0);
    }
    let rc = fn_1eae98(m, ctx, handle, ctx, s, h, d);
    if rc >= 0x80000000 {
        return cleanup_open(m, slot, logger, h, rc);
    }
    l_copy_to(m, ctx + ctx::NAME2, b"normal");
    m.put_word(ctx + ctx::TABLE90, 0);
    m.put_word(ctx + ctx::TABLE2A0, 0);
    let rc = fn_1eb978(m, ctx, handle, s.normal(), s, h, d);
    if rc >= 0x80000000 {
        return cleanup_open(m, slot, logger, h, rc);
    }
    let _ = k;
    h.call(m, 0x1738e8, &[logger, 0x8f8f44, 4, 0, 0]);
    rc
}

fn cleanup_open(m: &mut Memory<'_>, slot: u32, logger: u32, h: &mut impl Host, rc: u32) -> u32 {
    let ctx = m.word(slot);
    let handle = m.word(slot + 4);
    if ctx != 0 {
        fn_1eb7b8(m, ctx, handle, h);
    }
    m.put_word(slot, 0);
    m.put_word(slot + 4, 0);
    h.call(m, 0x1738e8, &[logger, 0x8f8f44, 4, 0, 0]);
    rc
}

pub fn fn_1e555c(
    m: &mut Memory<'_>,
    h0: u32,
    h1: u32,
    out: u32,
    h: &mut impl Host,
    f: &mut impl FetchHost,
    tbl: u32,
) -> u32 {
    if out == 0 {
        return 0x81102007;
    }
    let mut ctx = 0u32;
    if h.context(m, h0, h1, &mut ctx) >= 0x80000000 {
        return 0x81102007;
    }
    m.put_word(out, 0);
    m.put_word(out + 4, 0);
    let heap = m.word(ctx + synth_ctx::HEAP);
    let params = m.word(ctx + synth_ctx::PARAMS);
    let resources = m.word(ctx + synth_ctx::RESOURCES);
    let o = alloc(m, heap, synth::SIZE, h);
    if o == 0 {
        return 0x8110200a;
    }
    memset(m, o, 0, synth::SIZE);
    m.put_word(o + synth::H0, h0);
    m.put_word(o + synth::H1, h1);
    let mut res = 0u32;
    let rc = h.open_named(m, resources, b"SYNTHSTREAM", &mut res);
    if rc >= 0x80000000 {
        return synth_fail(m, o, rc, h, f);
    }
    let vt = m.word(res + 4);
    m.put_word(o + synth::VT, vt);
    let mut be = 0u32;
    let rc = f.synth_sequence(m, h0, h1, vt, &mut be);
    if rc >= 0x80000000 {
        return synth_fail(m, o, rc, h, f);
    }
    m.put_word(o + synth::BE, be);
    let mut fe = 0u32;
    let rc = f.fetch_table(m, h0, h1, &mut fe);
    if rc >= 0x80000000 {
        return synth_fail(m, o, rc, h, f);
    }
    m.put_word(o + synth::FE, fe);
    for i in 0..7 {
        let at = tbl + i * 6;
        let id = u16::from_le_bytes([m.byte(at), m.byte(at + 1)]) as u32;
        let a = u16::from_le_bytes([m.byte(at + 2), m.byte(at + 3)]) as u32;
        let b = u16::from_le_bytes([m.byte(at + 4), m.byte(at + 5)]) as u32;
        let rc = f.fetch_entry(m, fe, id, a, b);
        if rc >= 0x80000000 {
            return synth_fail(m, o, rc, h, f);
        }
    }
    let mut mode = 0u32;
    let pipeline = u8::from(
        h.parameter(m, params, b"clcpppipelinemode", &mut mode) < 0x80000000
            && mode != 0
            && m.byte(mode) != 0,
    );
    m.put_byte(o + synth::PIPELINE, pipeline);
    m.put_word(out, o);
    m.put_word(out + 4, 0x3815);
    0
}

fn synth_fail(
    m: &mut Memory<'_>,
    o: u32,
    rc: u32,
    h: &mut impl Host,
    f: &mut impl FetchHost,
) -> u32 {
    if crate::boot_port_1::fn_176f94(m, o, 0x3815, 0x3815, synth::SIZE) >= 0x80000000 {
        return rc;
    }
    if o == 0 {
        return rc;
    }
    let mut c2 = 0u32;
    if h.context(m, m.word(o + synth::H0), m.word(o + synth::H1), &mut c2) >= 0x80000000 {
        return rc;
    }
    let fe = m.word(o + synth::FE);
    if fe != 0 {
        f.release_table(m, fe);
    }
    let be = m.word(o + synth::BE);
    if be != 0 {
        f.release_sequence(m, be);
    }
    let vt = m.word(o + synth::VT);
    if vt != 0 {
        h.resource_close(m, m.word(c2 + synth_ctx::RESOURCES), b"SYNTHSTREAM");
    }
    release(m, m.word(c2 + synth_ctx::HEAP), o, h);
    rc
}

pub mod ectx {
    pub const MAGIC: u32 = 0x00;
    pub const HEAP: u32 = 0x04;
    pub const LOG: u32 = 0x10;
    pub const SETTINGS: u32 = 0x14;
    pub const RESOURCES: u32 = 0x18;
}

pub mod tok {
    pub const CONTEXT: u32 = 0x00;
    pub const SOURCE_TYPE: u32 = 0x04;
    pub const SOURCE_NAME: u32 = 0x08;
    pub const LINGDB: u32 = 0x0c;
    pub const CTLKP: u32 = 0x18;
    pub const CTLKP_DATA: u32 = 0x1c;
    pub const EPES: u32 = 0x24;
    pub const EPES_DATA: u32 = 0x28;
    pub const SELF: u32 = 0x30;
    pub const F38: u32 = 0x38;
    pub const F3C: u32 = 0x3c;
    pub const F40: u32 = 0x40;
    pub const F44: u32 = 0x44;
    pub const TAG: u32 = 0x48;
    pub const SRT_LEN: u32 = 0x4c;
    pub const MAX_BLOCKS: u32 = 0x4e;
    pub const FRAMES: u32 = 0x50;
    pub const F54: u32 = 0x54;
    pub const UNUSED58: u32 = 0x58;
    pub const COUNT: u32 = 0x6c;
    pub const TOKENS: u32 = 0x70;
    pub const F74: u32 = 0x74;
    pub const FB8: u32 = 0xb8;
    pub const FBC: u32 = 0xbc;
    pub const FC0: u32 = 0xc0;
    pub const FC4: u32 = 0xc4;
    pub const FC6: u32 = 0xc6;
    pub const FC8: u32 = 0xc8;
    pub const FF0: u32 = 0xf0;
    pub const FF1: u32 = 0xf1;
}

pub const MODULE_TAG: u32 = 0x10300;

pub const EMPTY_PAIR: u32 = 0x102c0;

struct TwoD<'a, H> {
    host: &'a mut H,
    base: u32,
}

impl<H: Host> crate::boot_part_e::Host for TwoD<'_, H> {
    fn call(&mut self, m: &mut Memory<'_>, f: u32, a: &[u32]) -> u32 {
        self.host.call(m, f, a)
    }
}

impl<H: Host> crate::boot_part_b::Host for TwoD<'_, H> {
    fn call(&mut self, m: &mut Memory<'_>, f: u32, a: &[u32]) -> u32 {
        self.host.call(m, f, a)
    }
    fn thread_id(&mut self) -> u32 {
        0
    }
}

impl<H: Host> crate::boot_part_b::FormatHost for TwoD<'_, H> {}

impl<H: Host> crate::frontend::BlockHeap for TwoD<'_, H> {
    fn alloc_block(&mut self, _heap: u32, _size: u32) -> u32 {
        0
    }
    fn realloc_block(&mut self, _heap: u32, _data: u32, _size: u32) -> u32 {
        0
    }
    fn alloc_cells(&mut self, _heap: u32, _count: u32, _size: u32) -> u32 {
        0
    }
}

impl<H: Host> crate::normalization::NormalizationHost for TwoD<'_, H> {
    fn calloc(&mut self, _arena: &mut [u8], _heap: u32, _count: u32, _size: u32) -> Option<u32> {
        None
    }
    fn alloc(&mut self, _arena: &mut [u8], _heap: u32, _size: u32) -> Option<u32> {
        None
    }
    fn realloc(
        &mut self,
        _arena: &mut [u8],
        _heap: u32,
        _ptr: u32,
        _size: u32,
    ) -> Option<u32> {
        None
    }
    fn free(&mut self, arena: &mut [u8], heap: u32, ptr: u32) {
        let base = self.base;
        release(&mut Memory { base, bytes: arena }, heap, ptr, &mut *self.host);
    }
    fn resolve(&mut self, _arena: &mut [u8], _h0: u32, _h1: u32) -> (u32, u32) {
        (0, 0)
    }
}

impl<H: Host> crate::render_part_a::ReaderHost for TwoD<'_, H> {
    fn array_release_method(&mut self, _context: u32, _pointer: u32) {}
    fn array_destructor(&mut self, _method: u32, _element: u32, _context: u32) {}
    fn array_initializer(
        &mut self,
        _memory: &mut [u8],
        _method: u32,
        _source: u32,
        _context: u32,
        _destination: u32,
    ) -> u32 {
        0
    }
}

fn two_d_environment(scratch: u32) -> crate::boot_port_2d::Environment {
    crate::boot_port_2d::Environment {
        begin: 0,
        end: 0,
        data_begin: 0,
        data_end: 0,
        block_begin: 0,
        block_end: 0,
        scratch,
        reader: crate::boot_port_2d::ReaderFields {
            category: 0,
            messages: [0; 16],
            pair_format: 0,
            triple_format: 0,
            four_format: 0,
            file_key: 0,
            chunk_key: 0,
            parent_key: 0,
            offset_key: 0,
            scratch,
        },
        tags: [0; 4],
        node_tags: [0; 12],
        sub_tag: 0,
        detach_messages: [0; 2],
    }
}

pub fn fn_1e4008(
    m: &mut Memory<'_>,
    ctx: u32,
    handle: u32,
    e: &crate::boot_port_4::Environment,
    h: &mut impl Host,
) -> u32 {
    if handle == 0 {
        return 0;
    }
    let logger = m.word(ctx + ectx::LOG);
    let heap = m.word(ctx + ectx::HEAP);
    h.call(m, 0x1738e8, &[logger, MODULE_TAG, 4, 0, 0]);
    let empty = (m.word(EMPTY_PAIR), m.word(EMPTY_PAIR + 4));
    let object = m.word(handle);
    let tag = m.word(handle + 4);
    let mut rc = 0u32;
    if crate::frontend::match_halves(object, tag, empty.0, empty.1) == 0 {
        let mut r = crate::boot_port_1::fn_176f94(m, object, tag, 0xe58d, 0xf4);
        if r >= 0x80000000 {
            return 0x89602008;
        }
        let o = object;
        let two_d = two_d_environment(e.scratch);
        r = crate::boot_port_2d::fn_252480(m, o, &two_d, &mut TwoD { host: &mut *h, base: m.base });
        if r >= 0x80000000 {
            rc = r;
        }
        r = crate::normalization::obj_close(m.bytes, o);
        if r >= 0x80000000 {
            rc = r;
        }
        let self_object = m.word(o + tok::SELF);
        let self_tag = m.word(o + tok::SELF + 4);
        if crate::frontend::match_halves(self_object, self_tag, empty.0, empty.1) == 0 {
            r = pcre_close(m, self_object, self_tag, h);
            if r >= 0x80000000 {
                rc = r;
            }
        }
        r = crate::boot_port_2d::fn_24ee5c(m, o, &two_d, &mut TwoD { host: &mut *h, base: m.base });
        if r >= 0x80000000 {
            rc = r;
        }
        m.put_word(o + tok::F38, 0);
        r = h.close_runtime_data(m, o, e);
        if r >= 0x80000000 {
            rc = r;
        }
        m.put_word(o + tok::F44, 0);
        r = crate::boot_port_2d::fn_24d748(m, o, &two_d, &mut TwoD { host: &mut *h, base: m.base });
        if r >= 0x80000000 {
            rc = r;
        }
        m.put_word(o + tok::F3C, 0);
        let resources = m.word(ctx + ectx::RESOURCES);
        if m.word(o + tok::CTLKP) != 0 {
            h.resource_close(m, resources, b"FE_DCTLKP");
        }
        if m.word(o + tok::EPES) != 0 {
            h.resource_close(m, resources, b"FE_DEPES");
        }
        if m.word(o + tok::LINGDB) != 0 {
            h.resource_close(m, resources, b"LINGDB");
        }
        let tokens = m.word(o + tok::TOKENS);
        if tokens != 0 {
            let count = m.half(o + tok::COUNT);
            for i in 0..count {
                let at = tokens + u32::from(i) * 4;
                release(m, heap, m.word(at), h);
                m.put_word(at, 0);
            }
            release(m, heap, tokens, h);
        }
        if m.word(o + tok::F54) != 0 {
            release(m, heap, m.word(o + tok::F54), h);
        }
        release(m, heap, o, h);
    }
    release(m, heap, handle, h);
    h.call(m, 0x1738e8, &[logger, MODULE_TAG, 4, 0, 0]);
    rc
}

fn pcre_close(m: &mut Memory<'_>, object: u32, key: u32, h: &mut impl Host) -> u32 {
    struct Pcre<'a, 'b, H> {
        m: &'a mut Memory<'b>,
        h: &'a mut H,
    }
    impl<H: Host> crate::regex::PcreHost for Pcre<'_, '_, H> {
        fn validate(&mut self, object: u32, key: u32, tag: u32, size: u32) -> u32 {
            crate::frontend::validate_handle(object != 0, key, tag, size)
        }
        fn resolve_owner(&mut self, _handle: u32, _key: i32) -> (u32, u32) {
            (0, 0)
        }
        fn heap_of(&mut self, _owner: u32) -> u32 {
            0
        }
        fn alloc(&mut self, _heap: u32, _count: u32, _size: u32) -> Option<u32> {
            None
        }
        fn free(&mut self, heap: u32, id: u32) {
            release(self.m, heap, id, self.h);
        }
    }
    let owner = m.word(object);
    let heap = m.word(owner + ectx::HEAP);
    let mut pcre = Pcre { m, h };
    crate::regex::pcre_close(&mut pcre, heap, object, key)
}

pub struct Format<'a, H>(pub &'a mut H);

impl<H: crate::boot_part_b::Host> crate::boot_part_b::Host for Format<'_, H> {
    fn call(&mut self, m: &mut Memory<'_>, f: u32, a: &[u32]) -> u32 {
        self.0.call(m, f, a)
    }
    fn thread_id(&mut self) -> u32 {
        self.0.thread_id()
    }
}

impl<H: crate::boot_part_b::Host> crate::boot_part_b::FormatHost for Format<'_, H> {}

pub fn fn_1e4270(
    m: &mut Memory<'_>,
    source_type: u32,
    source_name: u32,
    out_handle: u32,
    s: &Scratch,
    e: &crate::boot_port_4::Environment,
    h: &mut impl Host,
) -> u32 {
    if out_handle == 0 {
        return 0x89602007;
    }
    m.put_word(out_handle, 0);
    m.put_word(out_handle + 4, 0);
    let mut ctx = 0u32;
    let mut rc = h.context(m, source_type, source_name, &mut ctx);
    if rc >= 0x80000000 {
        return rc;
    }
    let logger = m.word(ctx + ectx::LOG);
    let heap = m.word(ctx + ectx::HEAP);
    h.call(m, 0x1738e8, &[logger, MODULE_TAG, 4, 0, 0]);
    let inner = calloc(m, heap, 1, 8, h);
    if inner == 0 {
        h.call(m, 0x173dc0, &[logger, MODULE_TAG, 0x5208, 0]);
        rc = 0x8960200a;
        h.call(m, 0x1738e8, &[logger, MODULE_TAG, 4, 0, 0]);
        return rc;
    }
    m.put_word(inner, 0);
    m.put_word(inner + 4, 0);
    let o = calloc(m, heap, 1, 0xf4, h);
    if o == 0 {
        rc = 0x8960200a;
        fn_1e4008(m, ctx, inner, e, h);
        h.call(m, 0x1738e8, &[logger, MODULE_TAG, 4, 0, 0]);
        return rc;
    }
    m.put_word(inner + 4, 0xe58d);
    m.put_word(inner, o);
    m.put_word(o + tok::CTLKP_DATA, 0);
    m.put_word(o + tok::CTLKP_DATA + 4, 0);
    m.put_word(o + tok::EPES_DATA, 0);
    m.put_word(o + tok::EPES_DATA + 4, 0);
    m.put_word(o + tok::F38, 0);
    m.put_word(o + tok::F44, 0);
    m.put_word(o + tok::F3C, 0);
    m.put_word(o + tok::CONTEXT, ctx);
    m.put_word(o + tok::SOURCE_TYPE, source_type);
    m.put_word(o + tok::SOURCE_NAME, source_name);
    m.put_word(o + tok::TAG, MODULE_TAG);
    m.put_half(o + tok::COUNT, 0);
    m.put_word(o + tok::TOKENS, 0);
    m.put_byte(o + tok::F74, 0);
    m.put_byte(o + tok::FC6, 0);
    m.put_word(o + tok::FC8, 0);
    m.put_word(o + tok::FB8, 0);
    m.put_half(o + tok::FBC, 0);
    m.put_word(o + tok::FC0, 0);
    m.put_half(o + tok::FC4, 0);
    m.put_word(o + tok::F54, 0);
    m.put_half(o + tok::UNUSED58, 0);
    m.put_half(o + tok::UNUSED58 + 2, 0);
    m.put_byte(o + tok::FF0, 1);
    m.put_byte(o + tok::FF1, 1);
    m.put_half(o + tok::SRT_LEN, 32);
    let mut setting = 0u32;
    if h.setting_value(
        m,
        m.word(ctx + ectx::SETTINGS),
        b"tokentninitstrlen",
        &mut setting,
    ) < 0x80000000
    {
        m.put_half(o + tok::SRT_LEN, setting as u16);
    }
    m.put_half(o + tok::MAX_BLOCKS, 5);
    if h.setting_value(
        m,
        m.word(ctx + ectx::SETTINGS),
        b"tokentnmaxpcreframeblocks",
        &mut setting,
    ) < 0x80000000
    {
        m.put_half(o + tok::MAX_BLOCKS, setting as u16);
    }
    m.put_half(o + tok::FRAMES, 20);
    if h.setting_value(
        m,
        m.word(ctx + ectx::SETTINGS),
        b"tokentnnumframesinblock",
        &mut setting,
    ) < 0x80000000
    {
        m.put_half(o + tok::FRAMES, setting as u16);
    }
    rc = pcre_open(m, source_type, source_name, o + tok::SELF, h);
    if rc >= 0x80000000 {
        h.call(m, 0x173dc0, &[logger, MODULE_TAG, 0x5dd5, 0]);
        fn_1e4008(m, ctx, inner, e, h);
        h.call(m, 0x1738e8, &[logger, MODULE_TAG, 4, 0, 0]);
        return rc;
    }
    rc = crate::boot_port_2::fn_1e3ef8(m, o, s.token_name(), h);
    if rc >= 0x80000000 {
        fn_1e4008(m, ctx, inner, e, h);
        h.call(m, 0x1738e8, &[logger, MODULE_TAG, 4, 0, 0]);
        return rc;
    }
    let resources = m.word(ctx + ectx::RESOURCES);
    let mut entry = 0u32;
    let status = h.open_named(m, resources, b"FE_DCTLKP", &mut entry);
    if status >= 0x80000000 {
        h.call(m, 0x173dc0, &[logger, MODULE_TAG, 0x5dd6, 0]);
        m.put_half(out_handle + 4, 0xe58c);
        m.put_word(out_handle, inner);
        h.call(m, 0x1738e8, &[logger, MODULE_TAG, 4, 0, 0]);
        return rc;
    }
    m.put_word(o + tok::CTLKP, m.word(entry + 4));
    m.put_word(o + tok::CTLKP_DATA, m.word(entry + 8));
    m.put_word(o + tok::CTLKP_DATA + 4, m.word(entry + 12));
    let status = h.open_named(m, resources, b"FE_DEPES", &mut entry);
    if status >= 0x80000000 {
        h.call(m, 0x173dc0, &[logger, MODULE_TAG, 0x5dd7, 0]);
        m.put_half(out_handle + 4, 0xe58c);
        m.put_word(out_handle, inner);
        h.call(m, 0x1738e8, &[logger, MODULE_TAG, 4, 0, 0]);
        return rc;
    }
    m.put_word(o + tok::EPES, m.word(entry + 4));
    m.put_word(o + tok::EPES_DATA, m.word(entry + 8));
    m.put_word(o + tok::EPES_DATA + 4, m.word(entry + 12));
    rc = h.open_named(m, resources, b"LINGDB", &mut entry);
    if rc >= 0x80000000 {
        fn_1e4008(m, ctx, inner, e, h);
        h.call(m, 0x1738e8, &[logger, MODULE_TAG, 4, 0, 0]);
        return rc;
    }
    m.put_word(o + tok::LINGDB, m.word(entry + 4));

    panic!("the token open stops at the LINGDB open");
}

fn pcre_open(
    m: &mut Memory<'_>,
    handle_ptr: u32,
    handle_key: u32,
    out: u32,
    h: &mut impl Host,
) -> u32 {
    struct Pcre<'a, 'b, H> {
        m: &'a mut Memory<'b>,
        h: &'a mut H,
        owner: u32,
    }
    impl<H: Host> crate::regex::PcreHost for Pcre<'_, '_, H> {
        fn validate(&mut self, object: u32, key: u32, tag: u32, size: u32) -> u32 {
            crate::frontend::validate_handle(object != 0, key, tag, size)
        }
        fn resolve_owner(&mut self, handle: u32, key: i32) -> (u32, u32) {
            let mut out = 0u32;
            let rc = self.h.context(self.m, handle, key as u32, &mut out);
            self.owner = out;
            (rc, out)
        }
        fn heap_of(&mut self, owner: u32) -> u32 {
            self.m.word(owner + ectx::HEAP)
        }
        fn alloc(&mut self, heap: u32, count: u32, size: u32) -> Option<u32> {
            let p = calloc(self.m, heap, count, size, self.h);
            if p == 0 {
                None
            } else {
                Some(p)
            }
        }
        fn free(&mut self, heap: u32, id: u32) {
            release(self.m, heap, id, self.h);
        }
    }
    let mut slot = crate::regex::PcreHandleOut::default();
    let mut pcre = Pcre { m, h, owner: 0 };
    let rc = crate::regex::pcre_open(
        &mut pcre,
        handle_ptr,
        handle_key as i32,
        Some(&mut slot),
    );
    let object = slot.object.unwrap_or(0);
    if object != 0 {
        pcre.m.put_word(object, pcre.owner);
    }
    pcre.m.put_word(out, object);
    pcre.m.put_word(out + 4, slot.key);
    rc
}

pub const SYS_DCT_IMAGE: u32 = 0x10800;

pub const SYS_DCT_SIZE: u32 = 0x200;

pub const SYS_DICT_FILE: u32 = 0x1234;

const SYS_DCT_SHORT: u32 = 0x81506004;

pub struct DictionaryImage<'a> {
    image: &'a [u8],
}

impl<'a> crate::morphology::SystemDictionaryIo<'a> for DictionaryImage<'a> {
    fn open(&mut self, _owner: u32, _name: &[u8], _flags: u32, file: &mut u32) -> u32 {
        *file = SYS_DICT_FILE;
        0
    }
    fn read(&mut self, _owner: u32, _file: u32, offset: u32, size: u32) -> Result<&'a [u8], u32> {
        let image = self.image;
        let start = offset as usize;
        let end = start.checked_add(size as usize).ok_or(SYS_DCT_SHORT)?;
        image.get(start..end).ok_or(SYS_DCT_SHORT)
    }
    fn release(&mut self, _owner: u32, _file: u32, _data: &'a [u8]) {}
    fn close(&mut self, _owner: u32, _file: u32) {}
}

struct ImagePages<'a> {
    image: &'a [u8],
}

impl crate::morphology::DictionaryPages for ImagePages<'_> {
    fn read(
        &mut self,
        offset: u32,
        length: u32,
    ) -> Result<&[u8], crate::frontend_types::Error> {
        let image = self.image;
        let start = offset as usize;
        let end = start
            .checked_add(length as usize)
            .ok_or(crate::frontend_types::Error::InvalidResource)?;
        image
            .get(start..end)
            .ok_or(crate::frontend_types::Error::InvalidResource)
    }
    fn release(&mut self) {}
}

fn sysdct_window<'a>(m: &'a Memory<'_>) -> &'a [u8] {
    let at = (SYS_DCT_IMAGE - m.base) as usize;
    &m.bytes[at..at + SYS_DCT_SIZE as usize]
}

fn sysdct_window_and_output<'a>(
    m: &'a mut Memory<'_>,
    out: u32,
    len: usize,
) -> (&'a [u8], &'a mut [u8]) {
    let base = m.base;
    let bytes = &mut *m.bytes;
    let image_at = (SYS_DCT_IMAGE - base) as usize;
    let image_end = image_at + SYS_DCT_SIZE as usize;
    let out_at = (out - base) as usize;
    let out_end = out_at + len;
    if out_at >= image_end {
        let (head, tail) = bytes.split_at_mut(out_at);
        (&head[image_at..image_end], &mut tail[..len])
    } else if out_end <= image_at {
        let (head, tail) = bytes.split_at_mut(image_at);
        (&tail[..SYS_DCT_SIZE as usize], &mut head[out_at..out_end])
    } else {
        panic!("the dictionary image and the lookup output overlap")
    }
}

pub fn fn_26b5f0(m: &mut Memory<'_>, st: u32, name: &[u8], size: &mut u32) -> u32 {
    let owner = m.word(st + state::H0);
    let (rc, handle) = {
        let mut io = DictionaryImage {
            image: sysdct_window(m),
        };
        let mut file = crate::morphology::SystemDictionaryFile { owner, file: 0 };
        let rc = match file.open(name, size, &mut io) {
            Ok(rc) => rc,
            Err(_) => SYS_DCT_SHORT,
        };
        (rc, file.file)
    };
    m.put_word(st + state::H1, handle);
    rc
}

pub fn fn_26b7ec(m: &mut Memory<'_>, st: u32, key: &[u8], size: u32, heap: u32) -> u32 {
    let owner = m.word(st + state::H0);
    let file = m.word(st + state::H1);
    let tables = m.word(st + state::TABLES);
    let (rc, handle, data, offsets) = {
        let image = sysdct_window(m);
        let base = image.as_ptr() as usize;
        let mut io = DictionaryImage { image };
        let mut file = crate::morphology::SystemDictionaryFile { owner, file };
        let mut sections = None;
        let rc = match file.load(key, size, heap, &mut sections, &mut io) {
            Ok(rc) => rc,
            Err(_) => SYS_DCT_SHORT,
        };
        let (data, offsets) = match &sections {
            Some(sections) => (
                SYS_DCT_IMAGE + (sections.data.as_ptr() as usize - base) as u32,
                sections.offsets,
            ),
            None => (0, [0u32; 19]),
        };
        (rc, file.file, data, offsets)
    };
    m.put_word(st + state::H1, handle);
    if data != 0 {
        m.put_word(tables, data);

        for index in 1..16 {
            m.put_word(tables + index as u32 * 4, data.wrapping_add(offsets[index]));
        }
        for index in 16..19 {
            m.put_word(tables + index as u32 * 4, offsets[index]);
        }
    }
    rc
}

pub fn fn_26bb6c(
    m: &mut Memory<'_>,
    st: u32,
    text: &[u8],
    start: &mut i16,
    end: &mut i16,
    fold: bool,
    length: &mut i16,
) -> i32 {
    let tables = m.word(st + state::TABLES);
    let section = m.word(tables);
    let output = m.word(st + state::OUTPUT);
    let section_end = m.word(tables + 16 * 4);
    let records = m.word(tables + 17 * 4);
    let offsets = m.word(tables + 18 * 4);

    let mut header = [0u8; 72];
    {
        let image = sysdct_window(m);
        let at = ((section - SYS_DCT_IMAGE) as usize).min(image.len());
        let end = (section_end as usize).min(image.len()).max(at);
        let slice = &image[at..end];
        let count = slice.len().min(header.len());
        header[..count].copy_from_slice(&slice[..count]);
    }
    let byte = |offset: usize| -> u8 { header.get(offset).copied().unwrap_or(0) };
    let half = |offset: usize| -> u16 {
        u16::from_le_bytes([byte(offset), byte(offset + 1)])
    };
    let word = |offset: usize| -> u32 {
        u32::from_le_bytes([byte(offset), byte(offset + 1), byte(offset + 2), byte(offset + 3)])
    };
    let entry = crate::morphology::EntryTables {
        trie: crate::morphology::TrieTables {
            alphabet: &[],
            first_character: byte(4),
            initial_characters: &[],
            initial_offsets: &[],
            initial_indices: &[],
            initial_position: word(56),
            initial_bit: byte(17),
            direct_count: half(24),
            count_tree: &[],
            count_root: half(26),
            counts: &[],
            character_tree: &[],
            character_root: half(28),
            characters: &[],
            delta_tree: &[],
            delta_root: half(30),
            deltas: &[],
            skipped_tree: &[],
            skipped_root: half(32),
            skipped_values: &[],
            skipped_width: byte(13),
            offsets: crate::morphology::OffsetTable {
                tree: &[],
                root: half(34),
                values: &[],
                width: byte(12),
                inline_limit: byte(18),
                shift: byte(14),
            },
            terminator: byte(16),
            inline_limit: byte(18),
            alignment_mask: byte(15),
            data_offset: section_end,
            page_size: half(50),
            fallback_bits: byte(12),
            fallback_shift: byte(14),
        },
        pairs: &[],
        pair_base: byte(6),
        input_min: byte(4),
        input_max: byte(5),
        separator: byte(8),
        prefix_min: byte(10),
        prefix_max: byte(11),
        max_length: half(22) as i16,
        offset_bits: byte(19),
        offset_size: half(54),
        offset_bytes: word(68),
        offsets_offset: offsets,
        record_size: half(52),
        records_offset: records,
    };
    let capacity = half(30) as usize + 1;
    let (image, out) = sysdct_window_and_output(m, output, capacity);
    let mut pages = ImagePages { image };
    let mut cursor = crate::morphology::TrieCursor {
        bits: crate::morphology::BitCursor {
            position: 0,
            bit: 0,
        },
        page_offset: None,
    };
    match cursor.lookup_entry(text, *start, *end, fold, entry, &mut pages, out, length) {
        Ok(true) => 1,
        Ok(false) => 0,
        Err(_) => panic!("the byte pair search would have to walk the trie"),
    }
}
