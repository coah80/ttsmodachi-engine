//! Boot-port-2d: the three unowned closers fn_24d748, fn_252480 and
//! fn_24ee5c.
//!
//! The engine logger is inert in this lane: every method pointer in the
//! guest logger is null, so a log call returns without touching the arena
//! and the format pointer is never dereferenced. The heap and the tree
//! destructor vtable are real: every release and every destructor reaches
//! the host, which records it in the caller's event log.
use crate::boot::Memory;

/// The platform window the harness maps at the same guest address on every
/// side. It holds the guest allocator, the tree destructor vtable and the
/// engine logger. It is never part of the compared arena.
pub mod platform {
    /// The first byte of the compared arena.
    pub const BASE: u32 = 0x1_0000;
    /// The size of the compared arena.
    pub const SIZE: u32 = 0x1000;
    /// The first byte of the platform window.
    pub const PLAT: u32 = 0x1_1000;
    /// The size of the platform window.
    pub const PLAT_SIZE: u32 = 0x1000;
    /// The guest allocator: methods at zero, context at four.
    pub const ALLOC: u32 = PLAT;
    /// The allocator method table. The release method sits at slot three.
    pub const ALLOC_METH: u32 = PLAT + 0x40;
    /// The tree destructor vtable. Slot one and slot four are destructors.
    pub const TREE_VT: u32 = PLAT + 0x80;
    /// The engine logger. Level four, every method pointer null.
    pub const LOGGER: u32 = PLAT + 0x100;
    /// The heap release method.
    pub const HOOK_REL: u32 = PLAT + 0x200;
    /// The first tree destructor.
    pub const HOOK_DA: u32 = PLAT + 0x240;
    /// The second tree destructor.
    pub const HOOK_DB: u32 = PLAT + 0x280;
    /// The caller's scratch window.
    pub const SCRATCH: u32 = PLAT + 0x400;
    /// The block data teardown's disjoint staging area.
    pub const BLOCK_SCRATCH: u32 = PLAT + 0x800;
    /// The event log inside the compared arena.
    pub const EVLOG: u32 = BASE;
    /// The number of event records the log holds.
    pub const EVMAX: usize = 64;
}

/// The platform boundary the three closers use. The block data teardown
/// reaches the reader facility, so the host is the reader host.
pub trait Host: crate::render_part_a::ReaderHost {}
impl<T: crate::render_part_a::ReaderHost> Host for T {}

/// The reader runtime fields the block data teardown reads. The runtime
/// itself is not copyable, so the lane carries its fields and builds one.
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
    /// The reader runtime the block data teardown expects.
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

/// The regional data the three closers read.
#[derive(Clone, Copy)]
pub struct Environment {
    /// The "Begin" format pointer of the tokenizer and the function map.
    pub begin: u32,
    /// The "End" format pointer of the tokenizer and the function map.
    pub end: u32,
    /// The "Begin" format pointer of the function data.
    pub data_begin: u32,
    /// The "End" format pointer of the function data.
    pub data_end: u32,
    /// The "Begin" format pointer of the block data.
    pub block_begin: u32,
    /// The "End" format pointer of the block data.
    pub block_end: u32,
    /// The caller's scratch window. The block data teardown needs a disjoint
    /// 1024-byte staging area of its own.
    pub scratch: u32,
    /// The reader runtime fields the block data teardown reads.
    pub reader: ReaderFields,
    /// The reader tags the block data teardown reads.
    pub tags: [u32; 4],
    /// The node tags the block data teardown reads.
    pub node_tags: [u32; 12],
    /// The sub tag the block data teardown writes.
    pub sub_tag: u32,
    /// The detach messages the block data teardown logs.
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

/// `fn_196428`. Release a heap block through the owner's vtable.
fn release(m: &mut Memory<'_>, heap: u32, p: u32, h: &mut impl Host) {
    struct Heap<'a, H>(&'a mut H);
    impl<H: Host> crate::boot_part_e::Host for Heap<'_, H> {
        fn call(&mut self, m: &mut Memory<'_>, f: u32, a: &[u32]) -> u32 {
            self.0.call(m, f, a)
        }
    }
    crate::boot_part_e::release(m, heap, p, &mut Heap(h));
}

/// fn_24d748, EU fn_24d904, KR fn_24d9b0.
///
/// Closes the function map: it empties both owned maps and releases their
/// array. The null array is the only error path.
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

/// fn_252480, EU fn_25263c, KR fn_2526e8.
///
/// Closes the tokenizer. The token tree is released through the owner's
/// manager; the null token is the only error path.
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

/// fn_24ee5c, EU fn_24f018, KR fn_24f0c4.
///
/// Closes the front end data: the function data, the block data and the
/// domain definitions in that order, then the owner's block.
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
