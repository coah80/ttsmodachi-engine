

use crate::engine::{Error, Memory};
use crate::engine_components::InternalCalls;
use crate::rom_region::{Rom, to_us, to_us_guest};

pub const UNBOUND: u32 = 0x8000_2001;

pub const TRACED: [u32; 8] = [
    0x15c2f0, 0x15c544, 0x15c7d8, 0x15cdcc, 0x15d0c4, 0x15d378, 0x15da64, 0x15c744,
];

pub fn trace_count_into(counts: &mut [u32; 8], entry: u32) {
    for (index, traced) in TRACED.iter().enumerate() {
        if *traced == entry {
            counts[index] += 1;
        }
    }
}

pub fn flat_view<'x, 'b>(m: &'x mut Memory<'_, 'b>) -> (u32, &'x mut [u8]) {
    let regions = &mut *m.regions;
    assert_eq!(regions.len(), 1, "the live arena must be one flat region");
    let first = &mut regions[0];
    (first.base, &mut first.bytes[..])
}

fn byte(m: &Memory<'_, '_>, a: u32) -> Result<u8, Error> {
    let mut v = [0u8; 1];
    m.read(a, &mut v)?;
    Ok(v[0])
}

fn half(m: &Memory<'_, '_>, a: u32) -> Result<u16, Error> {
    let mut v = [0u8; 2];
    m.read(a, &mut v)?;
    Ok(u16::from_le_bytes(v))
}

fn put_half(m: &mut Memory<'_, '_>, a: u32, v: u16) -> Result<(), Error> {
    m.write(a, &v.to_le_bytes())
}

pub fn fn_26b2f8_m(m: &mut Memory<'_, '_>, reader: u32, tree: u32, index: u32) -> Result<u32, Error> {
    let mut position = m.word(reader)?;
    let mut bit = byte(m, reader + 4)?;
    let data = m.word(reader + 12)?;
    let mut node = index;
    loop {
        let first = half(m, tree.wrapping_add(node.wrapping_mul(4)))?;
        if first == u16::MAX {
            break;
        }
        let second = half(m, tree.wrapping_add(node.wrapping_mul(4)).wrapping_add(2))?;
        let current = byte(m, data.wrapping_add(position))?;
        node = if current & (1u8 << bit) != 0 {
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
    m.put_word(reader, position)?;
    m.write(reader + 4, &[bit])?;
    Ok(node)
}

pub fn fn_26b444_m(m: &mut Memory<'_, '_>, obj: u32) -> Result<u32, Error> {
    let table = m.word(obj + 12)?;
    let header = m.word(table)?;
    let tree = m.word(table + 40)?;
    let values = m.word(table + 60)?;
    let width = byte(m, header + 12)? as u32;
    let shift = byte(m, header + 14)?;
    let limit = byte(m, header + 18)? as u32;
    let root = half(m, header + 34)? as u32;
    let index = fn_26b2f8_m(m, obj, tree, root)?;
    let bit_position = index.wrapping_mul(width);
    let sh = bit_position & 0xf;
    let remaining = 0x10 - sh;
    let word = (bit_position & 0xfffff) >> 4;
    let mut value;
    if remaining < width {
        let shift_bits = 0x20 - sh;
        value = read_packed(m, values, word, sh)?;
        if shift_bits < width {
            let middle = read_packed(m, values, word + 1, 0)?;
            value = value.wrapping_add(middle << remaining);
            let mask = 0xffffu32 >> ((0x30 - width) - sh);
            let last = read_packed(m, values, word + 2, 0)? & mask;
            value = value.wrapping_add(last << shift_bits);
        } else {
            let mask = 0xffffu32 >> ((0x20 - width) - sh);
            let next = read_packed(m, values, word + 1, 0)? & mask;
            value = value.wrapping_add(next << remaining);
        }
    } else {
        value = read_packed(m, values, word, sh)? & (0xffffu32 >> (0x10 - width));
    }
    if limit < value {
        value = value.wrapping_shl(shift.into());
    }
    Ok(value)
}

fn read_packed(m: &mut Memory<'_, '_>, values: u32, word: u32, sh: u32) -> Result<u32, Error> {
    let at = values.wrapping_add(word.wrapping_mul(2));
    Ok(u32::from(half(m, at)?) >> sh)
}

pub fn fn_26b37c_m(m: &mut Memory<'_, '_>, obj: u32) -> Result<u32, Error> {
    let table = m.word(obj + 12)?;
    let header = m.word(table)?;
    let tree = m.word(table + 36)?;
    let counts = m.word(table + 56)?;
    let bits = byte(m, header + 13)? as u32;
    let root = half(m, header + 32)? as u32;
    let index = fn_26b2f8_m(m, obj, tree, root)?;
    if bits <= 16 {
        return Ok(u32::from(half(m, counts.wrapping_add(index.wrapping_mul(2)))?));
    }
    let position = index.wrapping_mul(bits);
    let sh = position & 15;
    let word = (position << 12) >> 16;
    let lo = read_packed(m, counts, word, sh)?;
    let remaining = 32 - sh;
    if bits > remaining {
        let mid = read_packed(m, counts, word + 1, 0)? << (16 - sh);
        let mask = 0xffffu32 >> ((48 - bits) - sh);
        let hi = (read_packed(m, counts, word + 2, 0)? & mask) << remaining;
        return Ok(mid.wrapping_add(lo).wrapping_add(hi));
    }
    let mask = 0xffffu32 >> ((32 - bits) - sh);
    let next = read_packed(m, counts, word + 1, 0)? & mask;
    Ok(lo.wrapping_add(next << (16 - sh)))
}

pub fn fn_26b530_m(m: &mut Memory<'_, '_>, state: u32) -> Result<u32, Error> {
    let position = m.word(state)?;
    let bit = byte(m, state + 4)?;
    let base = m.word(state + 8)?;
    let format = m.word(state + 12)?;
    let bits = byte(m, format + 12)? as u32;
    let shift = byte(m, format + 14)? as u32;
    let end = bits + u32::from(bit);
    let mut value = u32::from(byte(m, base.wrapping_add(position))?) >> bit;
    let mut index = 1u32;
    if end > 8 {
        if end > 16 {
            loop {
                let next = byte(m, base.wrapping_add(position).wrapping_add(index))?;
                value = value.wrapping_add(u32::from(next) << (index * 8 - u32::from(bit)));
                index = (index + 1) & 0xff;
                if (index + 1) * 8 >= end {
                    break;
                }
            }
        }
        let shift_bits = index * 8;
        let mut remaining = shift_bits + 8;
        remaining -= u32::from(bit);
        remaining -= bits;
        let mask = 0xffu32 >> remaining;
        let next = byte(m, base.wrapping_add(position).wrapping_add(index))?;
        value = value.wrapping_add((u32::from(next) & mask).wrapping_shl(shift_bits - u32::from(bit)));
    } else {
        value &= 0xffu32 >> (8 - bits);
    }
    let position = position.wrapping_add(((u32::from(bit) + bits) & 0x38) >> 3);
    let bit = ((u32::from(bit) + bits) & 7) as u8;
    m.put_word(state, position)?;
    m.write(state + 4, &[bit])?;
    Ok(value.wrapping_shl(shift))
}

pub struct C4Calls<'a, P> {
    pub platform: &'a mut P,

    pub first_unbound: Option<u32>,

    pub calls: u32,

    pub rom: Rom,

    pub dictionary: core::cell::RefCell<crate::component_live_c::DictionaryWorkspace>,

    pub scratch: u32,

    pub trace_counts: [u32; 8],
}

impl<'a, P: crate::engine::HeapPlatform> C4Calls<'a, P> {
    pub fn new(platform: &'a mut P) -> Self {
        C4Calls {
            platform,
            first_unbound: None,
            calls: 0,
            rom: Rom::Us,
            dictionary: core::cell::RefCell::new(
                crate::component_live_c::DictionaryWorkspace::default(),
            ),
            scratch: 0,
            trace_counts: [0; 8],
        }
    }

    fn unbound(&mut self, entry: u32) -> u32 {
        if self.first_unbound.is_none() {
            self.first_unbound = Some(entry);
        }
        UNBOUND
    }
}

impl<'a, P: crate::engine::HeapPlatform> C4Calls<'a, P> {

    fn unrelocate(&self, entry: u32) -> u32 {
        to_us(self.rom, entry)
    }

    fn live(&mut self, entry: u32, args: &[u32], m: &mut Memory<'_, '_>) -> Result<u32, Error> {
        let rom = self.rom;
        trace_count_into(&mut self.trace_counts, to_us(rom, entry));
        let scratch = self.scratch;
        let (base, bytes) = flat_view(m);
        let mut host = crate::component_live_b::LiveHost {
            platform: &mut *self.platform,
            fault: None,
            scratch,
            rom,
        };
        let rc = host.call_m_with(
            entry,
            args,
            &mut crate::boot::Memory { base, bytes },
            rom,
        );
        match host.fault {
            Some(error) => Err(error),
            None => Ok(rc),
        }
    }
}

pub fn fn_206be4_m(
    m: &mut Memory<'_, '_>,
    p: u32,
    key: u32,
    handle: [u32; 2],
    changed: u32,
    scratch: u32,
    host: &mut impl InternalCalls,
) -> Result<u32, Error> {
    crate::engine_components::process_206be4(m, p, key, handle, changed, scratch, host)
}

impl<P: crate::engine::HeapPlatform> InternalCalls for C4Calls<'_, P> {
    fn rom(&self) -> crate::rom_region::Rom {
        self.rom
    }
    fn call(&mut self, m: &mut Memory<'_, '_>, entry: u32, args: &[u32]) -> Result<u32, Error> {
        self.calls += 1;
        let entry = self.unrelocate(entry);
        let rom = self.rom;
        match entry {
            0x284bc0 => {
                if args.len() < 8 {
                    return Ok(self.unbound(entry));
                }
                let scratch = self.scratch.wrapping_add(0x100);
                let match_scratch = self.scratch.wrapping_add(0x600);
                let host_scratch = self.scratch.wrapping_add(0x700);
                let first = &mut self.first_unbound;
                let counts = &mut self.trace_counts;
                fn_284bc0_m(
                    m,
                    args[0],
                    args[1],
                    args[2],
                    args[3],
                    args[4],
                    [args[5], args[6]],
                    args[7],
                    scratch,
                    match_scratch,
                    host_scratch,
                    &mut *self.platform,
                    first,
                    rom,
                    counts,
                )
            }
            _ => match self.live(entry, args, m) {
                Ok(rc) => Ok(rc),
                Err(error) => {
                    let missing = match error {
                        Error::MissingStreamAdapter { entry } => entry,
                        _ => entry,
                    };
                    let _ = error;
                    Ok(self.unbound(missing))
                }
            },
        }
    }
}

const TOKEN_CREATE: u32 = 44;
const TOKEN_FIND: u32 = 52;
const TOKEN_NEXT: u32 = 60;
const TOKEN_SET: u32 = 80;
const TOKEN_GET: u32 = 84;
const TOKEN_STRING: u32 = 88;

const DICT_FOLD: u32 = 32;
const DICT_NORMALIZE: u32 = 36;
const DICT_LOOKUP: u32 = 40;
const DICT_SPACE: u32 = 48;
const DICT_RULES: u32 = 72;

const CONTEXT_SET_TEXT: u32 = 36;
const CONTEXT_GET_TEXT: u32 = 40;
const CONTEXT_RUN: u32 = 44;

const DICT_CELLS: u32 = 0;
const DICT_BROKER: u32 = 16;

const DICT_QUERY: u32 = 0x200;

pub const DICT_FRAME: u32 = 0x600;

fn put_bytes(arena: &mut [u8], at: u32, data: &[u8]) -> bool {
    let Ok(at) = usize::try_from(at) else {
        return false;
    };
    match arena.get_mut(at..at.saturating_add(data.len())) {
        Some(slot) => {
            slot.copy_from_slice(data);
            true
        }
        None => false,
    }
}

fn get_byte(arena: &[u8], at: u32) -> Option<u8> {
    let at = usize::try_from(at).ok()?;
    arena.get(at).copied()
}

fn get_half(arena: &[u8], at: u32) -> Option<u16> {
    let at = usize::try_from(at).ok()?;
    let slot = arena.get(at..at.checked_add(2)?)?;
    Some(u16::from_le_bytes([slot[0], slot[1]]))
}

fn get_word(arena: &[u8], at: u32) -> Option<u32> {
    let at = usize::try_from(at).ok()?;
    let slot = arena.get(at..at.checked_add(4)?)?;
    Some(u32::from_le_bytes([slot[0], slot[1], slot[2], slot[3]]))
}

struct TokenHost<'a> {
    platform: &'a mut dyn crate::engine::HeapPlatform,

    rom: Rom,

    tokens: u32,

    dictionary: u32,

    context: u32,

    scratch: u32,

    first_unbound: &'a mut Option<u32>,

    counts: &'a mut [u32; 8],
}

impl TokenHost<'_> {

    fn guest(&mut self, entry: u32, args: &[u32], arena: &mut [u8]) -> u32 {
        let rom = self.rom;
        let entry = to_us(rom, entry);
        trace_count_into(self.counts, entry);
        if entry == 0x1ed6c4 || entry == 0x1ed1d0 {
            return self.canonical(entry, args, arena);
        }
        let scratch = self.scratch;
        let (rc, fault) = {
            let mut host = crate::component_live_b::LiveHost {
                platform: &mut *self.platform,
                fault: None,
                scratch,
                rom,
            };
            let rc = host.call_m_with(
                entry,
                args,
                &mut crate::boot::Memory {
                    base: 0,
                    bytes: arena,
                },
                rom,
            );
            (rc, host.fault)
        };
        if fault.is_some() && self.first_unbound.is_none() {
            *self.first_unbound = Some(entry);
        }
        rc
    }

    fn call_method(
        &mut self,
        table: u32,
        slot: u32,
        args: &[u32],
        arena: &mut [u8],
    ) -> u32 {
        let Some(entry) = get_word(arena, table.wrapping_add(slot)) else {
            return UNBOUND;
        };
        let entry = crate::rom_region::to_us_guest(self.rom, entry);
        self.guest(entry, args, arena)
    }

    fn canonical(&mut self, entry: u32, args: &[u32], arena: &mut [u8]) -> u32 {
        if args.len() < 4 {
            return UNBOUND;
        }
        let env = DictEnv {
            platform: core::cell::RefCell::new(&mut *self.platform),
            first_unbound: core::cell::RefCell::new(core::mem::take(self.first_unbound)),
            scratch: self.scratch.wrapping_add(0x100),
            rom: self.rom,
            resources: core::cell::Cell::new(0),
        };
        let rc = if entry == 0x1ed1d0 {

            let mut broker = DictBroker {
                env: &env,
                scratch: env.scratch.wrapping_add(DICT_BROKER),
            };
            let mut memory = crate::boot::Memory { base: 0, bytes: arena };
            crate::component_live_c::dctlkp_canonical_token_m(
                &mut memory,
                args[0],
                args[1],
                args[2],
                args[3],
                &mut broker,
            )
        } else {
            dctlkp_canonical_word(arena, args[0], args[1], args[2], args[3], &env)
        };
        *self.first_unbound = env.first_unbound.into_inner();
        rc
    }

    fn dictionary_lookup(&mut self, kind: u32, args: &[u32], arena: &mut [u8]) -> u32 {
        let env = DictEnv {
            platform: core::cell::RefCell::new(&mut *self.platform),
            first_unbound: core::cell::RefCell::new(core::mem::take(self.first_unbound)),
            scratch: self.scratch.wrapping_add(0x100),
            rom: self.rom,
            resources: core::cell::Cell::new(0),
        };
        let rc = dictionary_port(kind, args, arena, &env);
        *self.first_unbound = env.first_unbound.into_inner();
        rc
    }
}

fn cstr_len(arena: &[u8], at: u32) -> usize {
    let mut length = 0usize;
    while length < 4096 {
        match get_byte(arena, at.wrapping_add(length as u32)) {
            Some(0) => return length,
            Some(_) => length += 1,
            None => return length,
        }
    }
    length
}

fn copy_cstr(arena: &[u8], at: u32, out: &mut [u8; 1024]) -> usize {
    let length = cstr_len(arena, at).min(1023);
    let mut index = 0usize;
    while index < length {
        out[index] = get_byte(arena, at.wrapping_add(index as u32)).unwrap_or(0);
        index += 1;
    }
    out[length] = 0;
    length
}

fn dictionary_port<'p>(
    kind: u32,
    args: &[u32],
    arena: &mut [u8],
    env: &DictEnv<'p>,
) -> u32 {
    let needed = if kind == 0 { 7 } else { 8 };
    if args.len() < needed {
        return UNBOUND;
    }
    let ctx = args[0];
    let mut model = dctlkp_read(arena, ctx);
    env.resources.set(model.owner.resources);
    let states = [
        ctx.wrapping_add(8),
        ctx.wrapping_add(40),
        ctx.wrapping_add(72),
    ];
    let mut text = [0u8; 1024];
    let mut key = [0u8; 1024];
    let mut name = [0u8; 1024];
    let (text_len, key_len, name_len) = if kind == 0 {
        (
            copy_cstr(arena, args[2], &mut text),
            copy_cstr(arena, args[3], &mut key),
            copy_cstr(arena, args[4], &mut name),
        )
    } else {
        (
            copy_cstr(arena, args[3], &mut text),
            copy_cstr(arena, args[2], &mut key),
            0,
        )
    };
    let mut heap = ArenaHeap {
        arena,
        env,
        heap: model.owner.heap,
    };
    let mut host = DictHost { env };
    let mut dictionaries = DictDictionaries {
        env,
        workspace: core::cell::RefCell::new(
            crate::component_live_c::DictionaryWorkspace::default(),
        ),
        states,
        projected: [model.main, model.system, model.language],
    };
    let rc = if kind == 0 {

        let mut out = 0u32;
        let mut count = 0u16;
        let rc = crate::pronunciation::lookup_multiword(
            &mut model,
            &mut heap,
            &mut host,
            &mut dictionaries,
            ctx,
            &mut text[..text_len + 1],
            &key[..key_len + 1],
            &name[..name_len + 1],
            &mut out,
            &mut count,
        );
        let arena = crate::heap::Heap::arena(&mut heap);

        put_bytes(arena, args[2], &text[..text_len + 1]);
        put_bytes(arena, args[5], &out.to_le_bytes());
        put_bytes(arena, args[6], &count.to_le_bytes());
        rc
    } else {

        let mut out = 0u32;
        let mut count = 0u16;
        let mut separator = 0u8;
        let total = args[7];
        let rc = crate::pronunciation::lookup_dictionary_count(
            &mut model,
            &mut heap,
            &mut host,
            &mut dictionaries,
            ctx,
            &key[..key_len + 1],
            &text[..text_len + 1],
            &mut out,
            &mut count,
            &mut separator,
            total,
        );
        let arena = crate::heap::Heap::arena(&mut heap);
        put_bytes(arena, args[4], &out.to_le_bytes());
        put_bytes(arena, args[5], &count.to_le_bytes());
        put_bytes(arena, args[6], &[separator]);
        rc
    };
    let arena = crate::heap::Heap::arena(&mut heap);
    dctlkp_write(arena, ctx, &model);
    rc
}

pub fn fn_1edabc_m(
    m: &mut Memory<'_, '_>,
    a: [u32; 7],
    scratch: u32,
    platform: &mut dyn crate::engine::HeapPlatform,
    rom: Rom,
) -> Result<u32, Error> {
    let (_, arena) = flat_view(m);
    let env = DictEnv {
        platform: core::cell::RefCell::new(platform),
        first_unbound: core::cell::RefCell::new(None),
        scratch,
        rom,
        resources: core::cell::Cell::new(0),
    };
    let rc = dictionary_port(0, &a, arena, &env);
    match env.first_unbound.into_inner() {
        Some(entry) => Err(Error::UnsupportedStage { entry }),
        None => Ok(rc),
    }
}

pub fn fn_1ec7cc_m(
    m: &mut Memory<'_, '_>,
    a: [u32; 8],
    scratch: u32,
    platform: &mut dyn crate::engine::HeapPlatform,
    rom: Rom,
) -> Result<u32, Error> {
    let (_, arena) = flat_view(m);
    let env = DictEnv {
        platform: core::cell::RefCell::new(platform),
        first_unbound: core::cell::RefCell::new(None),
        scratch,
        rom,
        resources: core::cell::Cell::new(0),
    };
    let rc = dictionary_port(1, &a, arena, &env);
    match env.first_unbound.into_inner() {
        Some(entry) => Err(Error::UnsupportedStage { entry }),
        None => Ok(rc),
    }
}

struct DictHost<'e, 'p> {
    env: &'e DictEnv<'p>,
}

impl crate::pronunciation::LookupHost for DictHost<'_, '_> {
    fn check(&mut self, handle: u32, tag: u32, _size: u32) -> u32 {

        if handle == 0 || tag != 0xf37f {
            return 0x89802008;
        }
        0
    }

    fn resource_open(&mut self, heap: &mut impl crate::heap::Heap, name: &[u8], handle: &mut u32) -> u32 {
        let resources = self.env.resources.get();
        let arena = heap.arena();
        let mut buffer = [0u8; 1024];
        let length = name.len().min(1023);
        buffer[..length].copy_from_slice(&name[..length]);
        buffer[length] = 0;

        let name_address = self.env.scratch.wrapping_add(DICT_QUERY);
        put_bytes(arena, name_address, &buffer[..length + 1]);
        let out = self.env.scratch.wrapping_add(DICT_CELLS + 8);
        let rc = self.env.guest(
            0x179f30,
            &[resources, name_address, out],
            arena,
        );
        if rc < 0x8000_0000 {
            *handle = get_word(arena, out).unwrap_or(0);
        }
        rc
    }

    fn resource_close(&mut self, heap: &mut impl crate::heap::Heap, handle: u32) {
        let resources = self.env.resources.get();
        let arena = heap.arena();
        self.env.guest(0x179f3c, &[resources, handle], arena);
    }

    fn resource_release(&mut self, _handle: u32, _buffer: u32) { panic!("dictionary release requires live memory"); }
}

impl crate::text_input::TokenStoreHost for TokenHost<'_> {
    fn token_find(
        &mut self,
        arena: &mut [u8],
        h0: u32,
        h1: u32,
        kind: u32,
        arg: u32,
        out: u32,
    ) -> u32 {
        self.call_method(self.tokens, TOKEN_FIND, &[h0, h1, kind, arg, out], arena)
    }

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
    ) -> u32 {
        let buffer = self.scratch;
        let count = self.scratch.wrapping_add(8);
        let rc = self.call_method(
            self.tokens,
            TOKEN_GET,
            &[h0, h1, rec, field, len, buffer, count],
            arena,
        );
        if rc < 0x8000_0000 {
            if field == 0 {
                let byte = get_byte(arena, buffer).unwrap_or(0);
                put_bytes(arena, out, &[byte]);
            } else {
                let value = get_half(arena, buffer).unwrap_or(0);
                put_bytes(arena, out, &value.to_le_bytes());
            }
            let read = get_half(arena, count).unwrap_or(0);
            put_bytes(arena, out_count, &read.to_le_bytes());
        }
        rc
    }

    fn token_next(&mut self, arena: &mut [u8], h0: u32, h1: u32, tok: u32, out: u32) -> u32 {
        self.call_method(self.tokens, TOKEN_NEXT, &[h0, h1, tok, out], arena)
    }

    fn token_create(&mut self, arena: &mut [u8], h0: u32, h1: u32, tok: u32, out: u32) -> u32 {

        self.call_method(self.tokens, TOKEN_CREATE, &[h0, h1, tok, out], arena)
    }

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
    ) -> u32 {

        let count = self.scratch.wrapping_add(8);
        let rc = self.call_method(
            self.tokens,
            TOKEN_SET,
            &[h0, h1, rec, field, len, data, count],
            arena,
        );
        if rc < 0x8000_0000 {
            let written = get_half(arena, count).unwrap_or(0);
            put_bytes(arena, out_written, &written.to_le_bytes());
        }
        rc
    }

    fn token_get_string(
        &mut self,
        arena: &mut [u8],
        h0: u32,
        h1: u32,
        rec: u32,
        field: u32,
        out_text: u32,
        out_count: u32,
    ) -> u32 {
        self.call_method(
            self.tokens,
            TOKEN_STRING,
            &[h0, h1, rec, field, out_text, out_count],
            arena,
        )
    }
}

impl crate::text_input::TokenDictHost for TokenHost<'_> {
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
    ) -> u32 {
        self.dictionary_lookup(
            1,
            &[h0, h1, table, key, out_rules, out_count, out_sep, mode],
            arena,
        )
    }

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
    ) -> u32 {
        self.dictionary_lookup(
            0,
            &[h0, h1, text, kind, domain, out_entries, out_count],
            arena,
        )
    }

    fn dict_normalize(&mut self, arena: &mut [u8], h0: u32, h1: u32, domain: u32, text: u32) -> u32 {
        self.call_method(self.dictionary, DICT_NORMALIZE, &[h0, h1, domain, text], arena)
    }

    fn dict_fold(&mut self, arena: &mut [u8], h0: u32, h1: u32, domain: u32, text: u32) -> u32 {
        self.call_method(self.dictionary, DICT_FOLD, &[h0, h1, domain, text], arena)
    }

    fn dict_is_space(&mut self, arena: &[u8], h0: u32, h1: u32, c: u8) -> u32 {

        if h0 == 0 || h1 != 0xf37f {
            return 0;
        }
        let Some(class) = get_word(arena, h0.wrapping_add(100)) else {
            return 0;
        };
        if class == 0 {
            return 0;
        }
        let mut at = class;
        loop {
            let Some(byte) = get_byte(arena, at) else {
                return 0;
            };
            if byte == 0 {
                return 0;
            }
            if byte == c {
                return 1;
            }
            at = at.wrapping_add(1);
        }
    }
}

impl crate::text_input::TokenContextHost for TokenHost<'_> {
    fn context_set_text(
        &mut self,
        arena: &mut [u8],
        h0: u32,
        h1: u32,
        layer: u32,
        text: u32,
        len: u32,
    ) -> u32 {
        self.call_method(
            self.context,
            CONTEXT_SET_TEXT,
            &[h0, h1, layer, text, len],
            arena,
        )
    }

    fn context_run(&mut self, arena: &mut [u8], h0: u32, h1: u32, name: u32) -> u32 {
        self.call_method(self.context, CONTEXT_RUN, &[h0, h1, name], arena)
    }

    fn context_get_text(
        &mut self,
        arena: &mut [u8],
        h0: u32,
        h1: u32,
        layer: u32,
        out_text: u32,
        out_len: u32,
    ) -> u32 {
        self.call_method(
            self.context,
            CONTEXT_GET_TEXT,
            &[h0, h1, layer, out_text, out_len],
            arena,
        )
    }
}

impl crate::normalization::NormalizationHost for TokenHost<'_> {
    fn calloc(&mut self, arena: &mut [u8], heap: u32, count: u32, size: u32) -> Option<u32> {
        let rc = self.guest(0x1963b8, &[heap, count, size], arena);
        (rc < 0x8000_0000 && rc != 0).then_some(rc)
    }

    fn alloc(&mut self, arena: &mut [u8], heap: u32, size: u32) -> Option<u32> {
        let rc = self.guest(0x196394, &[heap, size], arena);
        (rc < 0x8000_0000 && rc != 0).then_some(rc)
    }

    fn realloc(&mut self, arena: &mut [u8], heap: u32, ptr: u32, size: u32) -> Option<u32> {
        let rc = self.guest(0x1963e0, &[heap, ptr, size], arena);
        (rc < 0x8000_0000 && rc != 0).then_some(rc)
    }

    fn free(&mut self, arena: &mut [u8], heap: u32, ptr: u32) {
        self.guest(0x196428, &[heap, ptr], arena);
    }

    fn resolve(&mut self, arena: &mut [u8], h0: u32, h1: u32) -> (u32, u32) {
        let anchor = crate::rom_region::addr(self.rom, 0x8d7660) as usize;
        let pair: [u8; 8] = arena[anchor..anchor + 8].try_into().unwrap();
        let pointed = (h0 != 0).then(|| u16::from_le_bytes(arena[h0 as usize..h0 as usize + 2].try_into().unwrap()));
        let mut context = 0;
        let status = crate::frontend::resolve_engine_context(h0, pointed, h1, &pair,
            crate::rom_region::addr(self.rom, 0x8d7640), &mut context);
        (status, context)
    }
}

#[allow(clippy::too_many_arguments)]
pub fn fn_284bc0_m(
    m: &mut Memory<'_, '_>,
    p: u32,
    source: u32,
    position: u32,
    end: u32,
    domain: u32,
    handle: [u32; 2],
    token: u32,
    scratch: u32,
    match_scratch: u32,
    host_scratch: u32,
    platform: &mut dyn crate::engine::HeapPlatform,
    first_unbound: &mut Option<u32>,
    rom: Rom,
    trace_counts: &mut [u32; 8],
) -> Result<u32, Error> {
    let tokens = m.word(p + 4)?;
    let dictionary = m.word(p + 16)?;
    let context = m.word(p + 28)?;
    let (base, bytes) = flat_view(m);
    let mut host = TokenHost {
        platform,
        rom,
        tokens,
        dictionary,
        context,
        scratch: host_scratch,
        first_unbound,
        counts: trace_counts,
    };
    let _ = base;
    Ok(crate::text_input::tokenize_span(
        bytes,
        p,
        source,
        position,
        end,
        domain,
        handle[0],
        handle[1],
        token,
        scratch,
        match_scratch,
        &mut host,
    ))
}

fn read_cstr(arena: &[u8], at: u32, out: &mut [u8; 256]) {
    let mut index = 0usize;
    while index < 256 {
        let byte = get_byte(arena, at.wrapping_add(index as u32)).unwrap_or(0);
        out[index] = byte;
        if byte == 0 {
            return;
        }
        index += 1;
    }
}

fn write_cstr(arena: &mut [u8], at: u32, value: &[u8; 256]) {
    let mut index = 0usize;
    while index < 256 && value[index] != 0 {
        put_bytes(arena, at.wrapping_add(index as u32), &[value[index]]);
        index += 1;
    }
    put_bytes(arena, at.wrapping_add(index as u32), &[0]);
}

fn dctlkp_read(arena: &[u8], ctx: u32) -> crate::pronunciation::LookupContext {
    use crate::pronunciation::{LookupContext, LookupDictionary, LookupDictionaryState, LookupOwner};
    let mut model = LookupContext::default();
    let owner = get_word(arena, ctx).unwrap_or(0);
    model.owner = LookupOwner {
        heap: get_word(arena, owner.wrapping_add(4)).unwrap_or(0),
        resources: get_word(arena, owner.wrapping_add(12)).unwrap_or(0),
        logger: get_word(arena, owner.wrapping_add(16)).unwrap_or(0),
    };
    let dictionary = |base: u32| LookupDictionary {
        size: get_word(arena, base).unwrap_or(0),
        state: LookupDictionaryState {
            bit_byte: get_word(arena, base.wrapping_add(4)).unwrap_or(0),
            bit_index: get_byte(arena, base.wrapping_add(8)).unwrap_or(0),
            buffer: get_word(arena, base.wrapping_add(12)).unwrap_or(0),
            tables: get_word(arena, base.wrapping_add(16)).unwrap_or(0),
            h0: get_word(arena, base.wrapping_add(20)).unwrap_or(0),
            h1: get_word(arena, base.wrapping_add(24)).unwrap_or(0),
            output: get_word(arena, base.wrapping_add(28)).unwrap_or(0),
        },
    };
    model.main = dictionary(ctx.wrapping_add(4));
    model.system = dictionary(ctx.wrapping_add(36));
    model.language = dictionary(ctx.wrapping_add(68));
    model.field64 = get_word(arena, ctx.wrapping_add(100)).unwrap_or(0);
    model.keys = get_word(arena, ctx.wrapping_add(104)).unwrap_or(0);
    model.tables = get_word(arena, ctx.wrapping_add(108)).unwrap_or(0);
    model.results = get_word(arena, ctx.wrapping_add(112)).unwrap_or(0);
    model.query = get_word(arena, ctx.wrapping_add(116)).unwrap_or(0);
    model.resources = get_word(arena, ctx.wrapping_add(120)).unwrap_or(0);
    model.dictionary_count = get_half(arena, ctx.wrapping_add(136)).unwrap_or(0);
    model.result_blocks = get_half(arena, ctx.wrapping_add(138)).unwrap_or(0);
    model.result_capacity = get_half(arena, ctx.wrapping_add(140)).unwrap_or(0);
    model.query_capacity = get_half(arena, ctx.wrapping_add(142)).unwrap_or(0) as i16;
    model.table90 = get_word(arena, ctx.wrapping_add(144)).unwrap_or(0);
    model.mode9c = get_half(arena, ctx.wrapping_add(156)).unwrap_or(0);
    model.flag29e = get_byte(arena, ctx.wrapping_add(670)).unwrap_or(0);
    model.table2a0 = get_word(arena, ctx.wrapping_add(672)).unwrap_or(0);
    read_cstr(arena, ctx.wrapping_add(158), &mut model.name);
    read_cstr(arena, ctx.wrapping_add(414), &mut model.name2);
    model
}

fn dctlkp_write(arena: &mut [u8], ctx: u32, model: &crate::pronunciation::LookupContext) {
    let mut dictionary = |base: u32, dict: &crate::pronunciation::LookupDictionary| {
        put_bytes(arena, base, &dict.size.to_le_bytes());
        let state = &dict.state;
        put_bytes(arena, base.wrapping_add(4), &state.bit_byte.to_le_bytes());
        put_bytes(arena, base.wrapping_add(8), &[state.bit_index]);
        put_bytes(arena, base.wrapping_add(12), &state.buffer.to_le_bytes());
        put_bytes(arena, base.wrapping_add(16), &state.tables.to_le_bytes());
        put_bytes(arena, base.wrapping_add(20), &state.h0.to_le_bytes());
        put_bytes(arena, base.wrapping_add(24), &state.h1.to_le_bytes());
        put_bytes(arena, base.wrapping_add(28), &state.output.to_le_bytes());
    };
    dictionary(ctx.wrapping_add(4), &model.main);
    dictionary(ctx.wrapping_add(36), &model.system);
    dictionary(ctx.wrapping_add(68), &model.language);
    put_bytes(arena, ctx.wrapping_add(104), &model.keys.to_le_bytes());
    put_bytes(arena, ctx.wrapping_add(108), &model.tables.to_le_bytes());
    put_bytes(arena, ctx.wrapping_add(112), &model.results.to_le_bytes());
    put_bytes(arena, ctx.wrapping_add(116), &model.query.to_le_bytes());
    put_bytes(arena, ctx.wrapping_add(120), &model.resources.to_le_bytes());
    put_bytes(arena, ctx.wrapping_add(136), &model.dictionary_count.to_le_bytes());
    put_bytes(arena, ctx.wrapping_add(138), &model.result_blocks.to_le_bytes());
    put_bytes(arena, ctx.wrapping_add(140), &model.result_capacity.to_le_bytes());
    put_bytes(arena, ctx.wrapping_add(142), &(model.query_capacity as u16).to_le_bytes());
    put_bytes(arena, ctx.wrapping_add(144), &model.table90.to_le_bytes());
    put_bytes(arena, ctx.wrapping_add(156), &model.mode9c.to_le_bytes());
    put_bytes(arena, ctx.wrapping_add(672), &model.table2a0.to_le_bytes());
    write_cstr(arena, ctx.wrapping_add(414), &model.name2);
}

fn dctlkp_canonical_word(
    arena: &mut [u8],
    ctx: u32,
    handle: u32,
    key: u32,
    text: u32,
    env: &DictEnv,
) -> u32 {
    if ctx == 0 || handle != 0xf37f {
        return 0x89802008;
    }
    let mut model = dctlkp_read(arena, ctx);
    env.resources.set(model.owner.resources);
    let mut key_buffer = [0u8; 1024];
    let key_length = copy_cstr(arena, key, &mut key_buffer);
    if key_length + 1 > 256 {

        return 0x89802007;
    }
    let mut text_buffer = [0u8; 1024];
    let text_length = copy_cstr(arena, text, &mut text_buffer);
    let mut cached = [0u8; 256];
    read_cstr(arena, ctx.wrapping_add(414), &mut cached);
    let cache_enabled = model.flag29e != 0;

    if cache_enabled && !same_bytes(&key_buffer[..key_length + 1], &cached) {
        let rc = canonical_map_refresh(arena, ctx, handle, &mut model, &key_buffer[..key_length + 1], env);
        if rc >= 0x8000_0000 {
            return rc;
        }
        cached[..key_length + 1].copy_from_slice(&key_buffer[..key_length + 1]);
    }

    let host = DictHost { env };
    let arena_ref: &[u8] = arena;
    let class = crate::pronunciation::text_at(arena_ref, model.field64);
    let mut storage: [Option<&[u8]>; 256] = [None; 256];
    if model.mode9c == 0 && model.table90 != 0 {
        for (slot, entry) in storage.iter_mut().enumerate() {
            let pointer =
                crate::pronunciation::read_offset(arena_ref, model.table90.wrapping_add(slot as u32 * 4));
            *entry = crate::pronunciation::LookupHost::resolve_replacement(
                &host,
                pointer,
                arena_ref,
            );
        }
    }
    let tables = crate::pronunciation::CanonicalTables {
        character_class: class,
        replacements: if model.table90 != 0 { &storage } else { &[] },
        mode: model.mode9c,
    };
    let mut loader = IdleLoader;
    let rc = match crate::pronunciation::canonical_word(
        &mut text_buffer[..text_length + 1],
        &key_buffer[..key_length + 1],
        &mut cached,
        cache_enabled,
        tables,
        &mut loader,
    ) {
        Ok(rc) => rc,
        Err(_) => return 0x89802007,
    };
    if rc >= 0x8000_0000 {
        return rc;
    }
    put_bytes(arena, text, &text_buffer[..text_length + 1]);
    write_cstr(arena, ctx.wrapping_add(414), &cached);
    dctlkp_write(arena, ctx, &model);
    rc
}

fn canonical_map_refresh(
    arena: &mut [u8],
    ctx: u32,
    handle: u32,
    model: &mut crate::pronunciation::LookupContext,
    name: &[u8],
    env: &DictEnv,
) -> u32 {
    let states = [
        ctx.wrapping_add(8),
        ctx.wrapping_add(40),
        ctx.wrapping_add(72),
    ];
    let projected = [model.main, model.system, model.language];
    let mut heap = ArenaHeap {
        arena,
        env,
        heap: model.owner.heap,
    };
    let mut host = DictHost { env };
    let mut dictionaries = DictDictionaries {
        env,
        workspace: core::cell::RefCell::new(
            crate::component_live_c::DictionaryWorkspace::default(),
        ),
        states,
        projected,
    };
    crate::pronunciation::canonical_map_refresh(
        model,
        &mut heap,
        &mut host,
        &mut dictionaries,
        handle,
        name,
    )
}

struct IdleLoader;

impl crate::pronunciation::CanonicalLoader for IdleLoader {
    fn refresh(&mut self, _key: &[u8]) -> u32 {
        0x89802008
    }
}

fn same_bytes(a: &[u8], b: &[u8]) -> bool {
    let length = a.iter().position(|&byte| byte == 0).unwrap_or(a.len());
    let other = b.iter().position(|&byte| byte == 0).unwrap_or(b.len());
    a[..length] == b[..other]
}

struct DictEnv<'p> {
    platform: core::cell::RefCell<&'p mut dyn crate::engine::HeapPlatform>,
    first_unbound: core::cell::RefCell<Option<u32>>,
    scratch: u32,

    rom: Rom,

    resources: core::cell::Cell<u32>,
}

impl DictEnv<'_> {

    fn guest(&self, entry: u32, args: &[u32], arena: &mut [u8]) -> u32 {
        if entry == 0xffff0001 {
            self.first_unbound.replace(Some(entry)); return 0x80002001;
        }
        let rom = self.rom;
        let entry = to_us(rom, entry);
        let scratch = self.scratch;
        let mut platform = self.platform.borrow_mut();
        let (rc, fault) = {
            let mut host = crate::component_live_b::LiveHost {
                platform: &mut **platform,
                fault: None,
                scratch,
                rom,
            };
            let rc = host.call_m_with(
                entry,
                args,
                &mut crate::boot::Memory {
                    base: 0,
                    bytes: arena,
                },
                rom,
            );
            (rc, host.fault)
        };
        if fault.is_some() {
            let mut first = self.first_unbound.borrow_mut();
            if first.is_none() {
                *first = Some(entry);
            }
        }
        rc
    }
}

struct ArenaHeap<'a, 'e, 'p> {
    arena: &'a mut [u8],
    env: &'e DictEnv<'p>,
    heap: u32,
}

impl crate::heap::Heap for ArenaHeap<'_, '_, '_> {
    fn allocate(&mut self, size: u32) -> Option<u32> {
        let rc = self.env.guest(0x196394, &[self.heap, size], self.arena);
        (rc < 0x8000_0000 && rc != 0).then_some(rc)
    }

    fn allocate_zeroed(&mut self, count: u32, size: u32) -> Option<u32> {
        let rc = self
            .env
            .guest(0x1963b8, &[self.heap, count, size], self.arena);
        (rc < 0x8000_0000 && rc != 0).then_some(rc)
    }

    fn reallocate(&mut self, pointer: u32, size: u32) -> Option<u32> {
        let rc = self.env.guest(0x1963e0, &[self.heap, pointer, size], self.arena);
        (rc < 0x8000_0000 && rc != 0).then_some(rc)
    }

    fn release(&mut self, pointer: u32) {
        self.env.guest(0x196428, &[self.heap, pointer], self.arena);
    }

    fn arena(&mut self) -> &mut [u8] {
        self.arena
    }
}

struct DictDictionaries<'e, 'p> {
    env: &'e DictEnv<'p>,
    workspace: core::cell::RefCell<crate::component_live_c::DictionaryWorkspace>,

    states: [u32; 3],

    projected: [crate::pronunciation::LookupDictionary; 3],
}

impl DictDictionaries<'_, '_> {

    fn state_address(&self, state: &crate::pronunciation::LookupDictionaryState) -> u32 {
        for (index, projected) in self.projected.iter().enumerate() {
            if projected.state.h0 == state.h0 && projected.state.h1 == state.h1 {
                return self.states[index];
            }
        }
        0
    }
}

impl crate::pronunciation::LookupDictionaries for DictDictionaries<'_, '_> {
    fn load(
        &mut self,
        arena: &mut [u8],
        state: &mut crate::pronunciation::LookupDictionaryState,
        key: &[u8],
        size: u32,
        resource: u32,
    ) -> u32 {
        let slot = self.env.scratch + DICT_CELLS;
        let rc = self.env.guest(0x17a1a4, &[state.h0, state.h1, 40, size, slot], arena);
        if rc != 0 {
            self.env.guest(0x179f3c, &[state.h0, state.h1], arena);
            state.h1 = 0;
            return rc;
        }
        let buffer = get_word(arena, slot).expect("dictionary index window");
        let mut position = 0;
        while position < size {
            let length = cstr_len(arena, buffer + position);
            let name = &arena[(buffer + position) as usize..(buffer + position) as usize + length];
            if name.eq_ignore_ascii_case(key) { break; }
            position = (position + length as u32 + 4) & !3;
            position += 4;
        }
        if position >= size {
            self.env.guest(0x179208, &[state.h0, state.h1, buffer], arena);
            return 0x89802014;
        }
        let position = (position + cstr_len(arena, buffer + position) as u32 + 4) & !3;
        let offset = get_word(arena, buffer + position).expect("dictionary section offset");
        self.env.guest(0x179208, &[state.h0, state.h1, buffer], arena);
        let rc = self.env.guest(0x17a1a4, &[state.h0, state.h1, offset, 4, slot], arena);
        if rc != 0 {
            self.env.guest(0x179f3c, &[state.h0, state.h1], arena);
            state.h1 = 0;
            return rc;
        }
        let header = get_word(arena, slot).expect("dictionary size window");
        let length = get_word(arena, header).expect("dictionary section size");
        self.env.guest(0x179208, &[state.h0, state.h1, header], arena);
        let rc = self.env.guest(0x17a1a4, &[state.h0, resource, offset, length, state.tables], arena);
        if rc != 0 { return rc; }
        let section = get_word(arena, state.tables).expect("dictionary mapped section");
        let byte = |at| get_byte(arena, section + at).expect("dictionary header byte") as u32;
        let half = |at| get_half(arena, section + at).expect("dictionary header half") as u32;
        let mut offsets = [0u32; 19];
        offsets[0] = section;
        let mut cursor = section + 72;
        offsets[1] = cursor;
        cursor = cursor.wrapping_add(byte(5).wrapping_sub(byte(4)).wrapping_add(4) & !3);
        offsets[2] = cursor;
        cursor = cursor.wrapping_add(byte(7).wrapping_sub(byte(6)).wrapping_mul(2).wrapping_add(5) & !3);
        offsets[3] = cursor;
        cursor = cursor.wrapping_add((half(46) + 3) & !3);
        offsets[4] = cursor;
        cursor = cursor.wrapping_add(half(46) * 4);
        offsets[5] = cursor;
        cursor = cursor.wrapping_add(half(46) * 4);
        offsets[6] = cursor;
        for (index, field) in (7..=11).zip((26..=34).step_by(2)) {
            cursor = cursor.wrapping_add(half(field) * 4 + 4);
            offsets[index] = cursor;
        }
        for (index, field, width) in [(12,36,2),(13,38,1),(14,40,1),(15,42,2)] {
            cursor = cursor.wrapping_add((half(field) * width + 3) & !3);
            offsets[index] = cursor;
        }
        offsets[16] = offset.wrapping_add(length);
        offsets[17] = offsets[16].wrapping_add((get_word(arena, section + 60).expect("dictionary tail size") + 3) & !3);
        offsets[18] = offsets[17].wrapping_add((get_word(arena, section + 64).expect("dictionary tail size") + 3) & !3);
        for (index, value) in offsets.into_iter().enumerate() {
            assert!(put_bytes(arena, state.tables + index as u32 * 4, &value.to_le_bytes()));
        }
        0
    }

    fn unload(
        &mut self,
        arena: &mut [u8],
        state: &mut crate::pronunciation::LookupDictionaryState,
        resource: u32,
    ) -> u32 {
        let buffer = get_word(arena, state.tables).expect("dictionary section release");
        self.env.guest(0x179208, &[state.h0, resource, buffer], arena);
        0
    }

    fn result_capacity(
        &mut self,
        arena: &[u8],
        state: &mut crate::pronunciation::LookupDictionaryState,
    ) -> u16 {
        let header = get_word(arena, state.tables).expect("dictionary section header");
        get_half(arena, header + 48).expect("dictionary result capacity")
    }

    fn separator(
        &mut self,
        arena: &[u8],
        state: &mut crate::pronunciation::LookupDictionaryState,
    ) -> u8 {

        let inner = state.tables;
        if inner == 0 {
            return 0;
        }
        let Some(inner) = get_word(arena, inner) else {
            return 0;
        };
        get_byte(arena, inner.wrapping_add(9)).unwrap_or(0)
    }

    fn lookup(
        &mut self,
        arena: &mut [u8],
        state: &mut crate::pronunciation::LookupDictionaryState,
        text: &[u8],
        start: &mut i16,
        end: &mut i16,
        fold: bool,
        length: &mut i16,
    ) -> bool {
        let address = self.state_address(state);
        if address == 0 {
            return false;
        }

        push_state(arena, address, state);

        let output_address = state.output;
        let mut stale = [0u8; 256];
        for (index, byte) in stale.iter_mut().enumerate() {
            *byte = get_byte(arena, output_address.wrapping_add(index as u32)).unwrap_or(0);
        }
        let scratch = self.env.scratch;
        let start_cell = scratch.wrapping_add(DICT_CELLS);
        let end_cell = scratch.wrapping_add(DICT_CELLS + 2);
        let length_cell = scratch.wrapping_add(DICT_CELLS + 4);
        put_bytes(arena, start_cell, &(*start as u16).to_le_bytes());
        put_bytes(arena, end_cell, &(*end as u16).to_le_bytes());
        put_bytes(arena, length_cell, &(*length as u16).to_le_bytes());

        let text_address = scratch.wrapping_add(DICT_QUERY);
        put_bytes(arena, text_address, text);
        let args = [
            address,
            text_address,
            start_cell,
            end_cell,
            u32::from(fold),
            length_cell,
        ];
        let mut workspace = self.workspace.borrow_mut();
        let mut broker = DictBroker {
            env: self.env,
            scratch: scratch.wrapping_add(DICT_BROKER),
        };
        let mut memory = crate::boot::Memory {
            base: 0,
            bytes: arena,
        };
        let rc = crate::component_live_c::dictionary_lookup_entry_m(
            &mut memory,
            args,
            &mut workspace,
            &mut broker,
        );
        *start = get_half(arena, start_cell).unwrap_or(0) as i16;
        *end = get_half(arena, end_cell).unwrap_or(0) as i16;
        *length = get_half(arena, length_cell).unwrap_or(0) as i16;

        let written = get_half(arena, length_cell).unwrap_or(0) as usize;
        if written < stale.len() {
            put_bytes(
                arena,
                output_address.wrapping_add(written as u32),
                &[stale[written]],
            );
        }

        pull_state(arena, address, state);
        rc.is_ok() && rc.unwrap_or(1) != 0
    }
}

fn push_state(
    arena: &mut [u8],
    address: u32,
    state: &crate::pronunciation::LookupDictionaryState,
) {
    put_bytes(arena, address, &state.bit_byte.to_le_bytes());
    put_bytes(arena, address + 4, &[state.bit_index]);
    put_bytes(arena, address + 8, &state.buffer.to_le_bytes());
    put_bytes(arena, address + 12, &state.tables.to_le_bytes());
    put_bytes(arena, address + 16, &state.h0.to_le_bytes());
    put_bytes(arena, address + 20, &state.h1.to_le_bytes());
    put_bytes(arena, address + 24, &state.output.to_le_bytes());
}

fn pull_state(
    arena: &[u8],
    address: u32,
    state: &mut crate::pronunciation::LookupDictionaryState,
) {
    state.bit_byte = get_word(arena, address).unwrap_or(0);
    state.bit_index = get_byte(arena, address + 4).unwrap_or(0);
    state.buffer = get_word(arena, address + 8).unwrap_or(0);
    state.tables = get_word(arena, address + 12).unwrap_or(0);
    state.h0 = get_word(arena, address + 16).unwrap_or(0);
    state.h1 = get_word(arena, address + 20).unwrap_or(0);
    state.output = get_word(arena, address + 24).unwrap_or(0);
}

struct DictBroker<'e, 'p> {
    env: &'e DictEnv<'p>,
    scratch: u32,
}

impl crate::component_live_c::Host for DictBroker<'_, '_> {
    fn call_m(&mut self, entry: u32, args: &[u32], m: &mut crate::boot::Memory<'_>) -> u32 {
        match entry {
            0x17a1a4 | 0x179208 => {
                crate::component_live_c::broker_range_live_m(m, entry, args, self.scratch, self)
            }
            _ => {
                let bytes = &mut *m.bytes;
                self.env.guest(entry, args, bytes)
            }
        }
    }

    fn thread_id_m(&mut self) -> u32 {
        0x1ff82000
    }
}
