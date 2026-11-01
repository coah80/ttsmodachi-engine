

use crate::boot::Memory;

pub mod platform {

    pub const BASE: u32 = 0x1_0000;

    pub const SIZE: u32 = 0x1000;

    pub const PLAT: u32 = 0x1_1000;

    pub const PLAT_SIZE: u32 = 0x1000;

    pub const ALLOC: u32 = PLAT;

    pub const ALLOC_METH: u32 = PLAT + 0x40;

    pub const TREE_VT: u32 = PLAT + 0x80;

    pub const LOGGER: u32 = PLAT + 0x100;

    pub const HOOK_REL: u32 = PLAT + 0x200;

    pub const HOOK_DA: u32 = PLAT + 0x240;

    pub const HOOK_DB: u32 = PLAT + 0x280;

    pub const SCRATCH: u32 = PLAT + 0x400;

    pub const BLOCK_SCRATCH: u32 = PLAT + 0x800;

    pub const EVLOG: u32 = BASE;

    pub const EVMAX: usize = 64;
}

pub trait Host: crate::render_part_a::ReaderHost {}
impl<T: crate::render_part_a::ReaderHost> Host for T {}

#[derive(Clone, Copy)]
pub struct ReaderFields {
    pub category: u32,
    pub messages: [u32; 16],
    pub pair_format: u32,
    pub triple_format: u32,
    pub four_format: u32,
    pub file_key: u32,
    pub chunk_key: u32,
    pub parent_key: u32,
    pub offset_key: u32,
    pub scratch: u32,
}

impl ReaderFields {

    pub fn runtime(&self) -> crate::render_part_a::ReaderRuntime {
        crate::render_part_a::ReaderRuntime {
            category: self.category,
            messages: self.messages,
            pair_format: self.pair_format,
            triple_format: self.triple_format,
            four_format: self.four_format,
            file_key: self.file_key,
            chunk_key: self.chunk_key,
            parent_key: self.parent_key,
            offset_key: self.offset_key,
            scratch: self.scratch,
        }
    }
}

#[derive(Clone, Copy)]
pub struct Environment {

    pub begin: u32,

    pub end: u32,

    pub data_begin: u32,

    pub data_end: u32,

    pub block_begin: u32,

    pub block_end: u32,

    pub scratch: u32,

    pub reader: ReaderFields,

    pub tags: [u32; 4],

    pub node_tags: [u32; 12],

    pub sub_tag: u32,

    pub detach_messages: [u32; 2],
}

struct Log<'a, H>(&'a mut H);
impl<H: Host> crate::log_live::LogMethodsM for Log<'_, H> {
    fn log_m(
        &mut self,
        f: u32,
        o: u32,
        c: u32,
        l: u32,
        a: u32,
        fmt: u32,
        va: u32,
        m: &mut Memory<'_>,
    ) -> u32 {
        self.0.call(m, f, &[o, c, l, a, fmt, va])
    }
}

fn release(m: &mut Memory<'_>, heap: u32, p: u32, h: &mut impl Host) {
    struct Heap<'a, H>(&'a mut H);
    impl<H: Host> crate::boot_part_e::Host for Heap<'_, H> {
        fn call(&mut self, m: &mut Memory<'_>, f: u32, a: &[u32]) -> u32 {
            self.0.call(m, f, a)
        }
    }
    crate::boot_part_e::release(m, heap, p, &mut Heap(h));
}

pub fn fn_24d748(m: &mut Memory<'_>, o: u32, e: &Environment, h: &mut impl Host) -> u32 {
    let arr = m.word(o + 0x3c);
    if arr == 0 {
        return crate::normalization::error_code(8);
    }
    let ctx = m.word(o);
    let logger = m.word(ctx + 16);
    let module = m.word(o + 0x48);
    crate::log_live::fn_1738e8(m, logger, module, 4, 0, e.begin, 0, &mut Log(h));
    let mut i = 0u32;
    while i < 2 {
        let entry = m.word(arr + i * 4);
        if entry != 0 {
            crate::boot_part_b2::guest_19665c_live(m, entry, h);
            m.put_word(arr + i * 4, 0);
        }
        i += 1;
    }
    crate::log_live::fn_1738e8(m, logger, module, 4, 0, e.end, 0, &mut Log(h));
    release(m, m.word(ctx + 4), arr, h);
    0
}

pub fn fn_252480(m: &mut Memory<'_>, o: u32, e: &Environment, h: &mut impl Host) -> u32 {
    if m.word(o + 0x40) == 0 {
        return crate::normalization::error_code(8);
    }
    let owner = m.word(o);
    let logger = m.word(owner + 0x10);
    let cat = m.word(o + 0x48);
    crate::log_live::fn_1738e8(m, logger, cat, 4, 0, e.begin, 0, &mut Log(h));
    {
        assert_eq!(m.base, 0, "the token tree port indexes the arena absolutely");
        let arena = &mut m.bytes[..];
        crate::normalization::close_token_tree(arena, o, h);
    }
    crate::log_live::fn_1738e8(m, logger, cat, 4, 0, e.end, 0, &mut Log(h));
    0
}

pub fn fn_24ee5c(m: &mut Memory<'_>, o: u32, e: &Environment, h: &mut impl Host) -> u32 {
    let blk = m.word(o + 0x38);
    if blk == 0 {
        return crate::normalization::error_code(8);
    }
    let ctx = m.word(o);
    let logger = m.word(ctx + 16);
    let name = m.word(o + 0x48);
    crate::log_live::fn_1738e8(m, logger, name, 4, 0, e.data_begin, 0, &mut Log(h));
    let data = crate::boot_port_4::Environment {
        null_pair: [0; 8],
        default_context: 0,
        module: 0,
        synthstream: 0,
        utf16: 0,
        begin: e.data_begin,
        end: e.data_end,
        handlers: [0; 5],
        runtime_callbacks: [0; 3],
        scratch: e.scratch,
        block_scratch: e.scratch + 0x400,
    };
    let block = crate::boot_port_4b::Environment {
        reader: e.reader.runtime(),
        tags: e.tags,
        node_tags: e.node_tags,
        sub_tag: e.sub_tag,
        detach_messages: e.detach_messages,
        close_messages: [e.block_begin, e.block_end],
        scratch: e.scratch + 0x400,
    };
    let mut rc = 0;
    let r = crate::boot_port_4::fn_2b6594(m, o, &data, h);
    if r >= 0x8000_0000 {
        rc = r;
    }
    let r = crate::boot_port_4b::fn_250a3c(m, h, o, &block);
    if r >= 0x8000_0000 {
        rc = r;
    }
    let r = crate::boot_port_4::fn_2b5e0c(m, o, &data, h);
    if r >= 0x8000_0000 {
        rc = r;
    }
    release(m, m.word(ctx + 4), blk, h);
    m.put_word(o + 0x38, 0);
    crate::log_live::fn_1738e8(m, logger, name, 4, 0, e.data_end, 0, &mut Log(h));
    rc
}
