//! Caller-owned engine heap.
//!
//! `fn_1ea4e0` and its callees obtain storage through the engine's allocator
//! object. The TTS-side entry points are thin vtable dispatchers:
//!
//! | US address | dispatch |
//! | --- | --- |
//! | `fn_196394` | `vt[0](object, size)` |
//! | `fn_1963b8` | `vt[1](object, count, size)` |
//! | `fn_1963e0` | `object ? vt[2](object, pointer, size) : vt[0](object, size)` |
//! | `fn_196428` | `methods[3](object, pointer)` |
//!
//! The concrete methods those slots hold are the game's block allocator:
//!
//! | US address | role |
//! | --- | --- |
//! | `fn_184f20` | allocate entry: stores the heap configuration, then `fn_1850c0` |
//! | `fn_1850c0` | best-fit/first-fit block search with alignment |
//! | `fn_19b814` | carve a block, split the remainder into free blocks |
//! | `fn_184f84` | release and coalesce with the free list |
//! | `fn_1852d8` / `fn_1852e8` / `fn_1853c4` | configuration setters (`pool`, `flags`, `fill`) |
//! | `fn_6bab68` | grow or shrink a block in place, absorbing the following free block |
//! | `fn_6baddc` | block payload size |
//!
//! The block header is 16 bytes: `{u16 tag, u16 flags, u32 size, Block *prev,
//! Block *next}` with tag `0x4652` for free blocks and `0x5544` for live ones.
//! `flags` holds the header's own displacement in bits 8..14 and the carve
//! direction in bit 15, so the block start is `header - ((flags << 17) >> 25)`.
//! The free and used lists keep the original order: a carve inserts into the
//! free list before the original predecessor and appends the live block at the
//! used list's tail; release walks the free list forward from its head and stops
//! at the first block at or above the released start.
//!
//! `EngineHeap` owns a caller-supplied `&mut [u8]` arena and keeps the original
//! layout: the heap object starts at offset 0, the zero-on-allocate word is at
//! `0x38`, the lists and configuration at `0x3c`, and blocks are carved from the
//! rest. Every pointer is an arena offset, so pointer-derived values and
//! capacity checks downstream behave exactly as they do in the original. The
//! first free block is whatever the arena already holds at `first_block`.
//!
//! The heap is additive: other slices can take a `&mut impl Heap` and keep the
//! same offsets.

/// Free block tag.
pub const FREE: u16 = 0x4652;
/// Live block tag.
pub const USED: u16 = 0x5544;
/// Block header size.
pub const HEADER: u32 = 16;
/// Offset of the zero-on-allocate word.
pub const ZERO_FLAG: u32 = 0x38;
/// Offset of the free/used lists.
pub const LISTS: u32 = 0x3c;
/// Offset of the live allocation count `fn_184f20` keeps.
pub const COUNT: u32 = 0x54;

/// Storage for one call tree. Pointers are offsets into the caller's arena.
pub trait Heap {
    /// `fn_196394`. `None` when the allocator refuses.
    fn allocate(&mut self, size: u32) -> Option<u32>;
    /// `fn_1963b8`: `count * size` bytes, zeroed. `None` when it refuses.
    fn allocate_zeroed(&mut self, count: u32, size: u32) -> Option<u32>;
    /// `fn_1963e0`. `None` when it refuses.
    fn reallocate(&mut self, pointer: u32, size: u32) -> Option<u32>;
    /// `fn_196428`.
    fn release(&mut self, pointer: u32);
    /// The arena those offsets index.
    fn arena(&mut self) -> &mut [u8];
}

/// A heap that always refuses. Useful for the allocation-free paths.
pub struct NoHeap;

impl Heap for NoHeap {
    fn allocate(&mut self, _size: u32) -> Option<u32> {
        None
    }
    fn allocate_zeroed(&mut self, _count: u32, _size: u32) -> Option<u32> {
        None
    }
    fn reallocate(&mut self, _pointer: u32, _size: u32) -> Option<u32> {
        None
    }
    fn release(&mut self, _pointer: u32) {}
    fn arena(&mut self) -> &mut [u8] {
        &mut []
    }
}

fn word(arena: &[u8], offset: u32) -> u32 {
    let at = offset as usize;
    u32::from_le_bytes(arena[at..at + 4].try_into().unwrap())
}

fn set_word(arena: &mut [u8], offset: u32, value: u32) {
    let at = offset as usize;
    arena[at..at + 4].copy_from_slice(&value.to_le_bytes());
}

fn half(arena: &[u8], offset: u32) -> u16 {
    let at = offset as usize;
    u16::from_le_bytes(arena[at..at + 2].try_into().unwrap())
}

fn set_half(arena: &mut [u8], offset: u32, value: u16) {
    let at = offset as usize;
    arena[at..at + 2].copy_from_slice(&value.to_le_bytes());
}

fn byte(arena: &[u8], offset: u32) -> u8 {
    arena[offset as usize]
}

fn set_byte(arena: &mut [u8], offset: u32, value: u8) {
    arena[offset as usize] = value;
}

/// A block header at `offset`: `(tag, flags, size, previous, next)`.
#[derive(Clone, Copy)]
struct Block {
    tag: u16,
    flags: u16,
    size: u32,
    previous: u32,
    next: u32,
}

fn read_block(arena: &[u8], offset: u32) -> Block {
    Block {
        tag: half(arena, offset),
        flags: half(arena, offset + 2),
        size: word(arena, offset + 4),
        previous: word(arena, offset + 8),
        next: word(arena, offset + 12),
    }
}

fn write_block(arena: &mut [u8], offset: u32, block: Block) {
    set_half(arena, offset, block.tag);
    set_half(arena, offset + 2, block.flags);
    set_word(arena, offset + 4, block.size);
    set_word(arena, offset + 8, block.previous);
    set_word(arena, offset + 12, block.next);
}

fn init_block(arena: &mut [u8], offset: u32, tag: u16, size: u32) {
    write_block(
        arena,
        offset,
        Block {
            tag,
            flags: 0,
            size,
            previous: 0,
            next: 0,
        },
    );
}

/// Start of the block owning `header`, from its stored displacement.
fn block_start(header: u32, flags: u16) -> u32 {
    header.wrapping_sub(u32::from(flags) << 17 >> 25)
}

/// Insert `block` into `list` (at `list_offset`) before `before`.
fn link(arena: &mut [u8], list_offset: u32, before: u32, block: u32) {
    let next = if before != 0 {
        let following = word(arena, before + 12);
        set_word(arena, before + 12, block);
        following
    } else {
        let head = word(arena, list_offset);
        set_word(arena, list_offset, block);
        head
    };
    set_word(arena, block + 8, before);
    set_word(arena, block + 12, next);
    if next != 0 {
        set_word(arena, next + 8, block);
    } else {
        set_word(arena, list_offset + 4, block);
    }
}

/// Remove `block` from the list at `list_offset`.
fn unlink(arena: &mut [u8], list_offset: u32, block: u32) {
    let previous = word(arena, block + 8);
    let next = word(arena, block + 12);
    if previous != 0 {
        set_word(arena, previous + 12, next);
    } else {
        set_word(arena, list_offset, next);
    }
    if next != 0 {
        set_word(arena, next + 8, previous);
    } else {
        set_word(arena, list_offset + 4, previous);
    }
}

/// The block allocator over one caller-owned arena.
pub struct EngineHeap<'a> {
    arena: &'a mut [u8],
    pool: u16,
    flags: u16,
    fill: u8,
    zero_on_allocate: bool,
    alignment: u32,
}

impl<'a> EngineHeap<'a> {
    /// Wrap an arena whose first block already sits at `first_block` and whose
    /// region ends at `arena.len()`.
    pub fn new(arena: &'a mut [u8], first_block: u32) -> Self {
        EngineHeap {
            arena,
            pool: 0,
            flags: 0,
            fill: 0,
            zero_on_allocate: false,
            alignment: 4,
        }
        .with_first_block(first_block)
    }

    /// Wrap an arena that already holds a heap image.
    pub fn from_arena(arena: &'a mut [u8]) -> Self {
        EngineHeap {
            pool: half(arena, LISTS + 0x10),
            flags: half(arena, LISTS + 0x12),
            fill: byte(arena, LISTS + 0x14),
            zero_on_allocate: word(arena, ZERO_FLAG) & 1 != 0,
            alignment: 4,
            arena,
        }
    }

    /// The alignment the engine's allocator asks the block heap for.
    pub fn set_alignment(&mut self, alignment: u32) -> &mut Self {
        self.alignment = alignment;
        self
    }

    /// The zero-on-allocate word at `ZERO_FLAG`.
    pub fn set_zero_on_allocate(&mut self, zero: bool) -> &mut Self {
        self.zero_on_allocate = zero;
        set_word(self.arena, ZERO_FLAG, u32::from(zero));
        self
    }

    fn with_first_block(self, first_block: u32) -> Self {
        let end = self.arena.len() as u32;
        init_block(self.arena, first_block, FREE, end - first_block - HEADER);
        set_word(self.arena, LISTS, first_block);
        set_word(self.arena, LISTS + 4, first_block);
        set_word(self.arena, LISTS + 8, 0);
        set_word(self.arena, LISTS + 12, 0);
        set_half(self.arena, LISTS + 0x10, self.pool);
        set_half(self.arena, LISTS + 0x12, self.flags);
        set_byte(self.arena, LISTS + 0x14, self.fill);
        set_word(self.arena, ZERO_FLAG, u32::from(self.zero_on_allocate));
        self
    }

    /// `fn_1852d8`. Returns the previous pool.
    pub fn set_pool(&mut self, pool: u16) -> u16 {
        let old = half(self.arena, LISTS + 0x10);
        set_half(self.arena, LISTS + 0x10, pool);
        self.pool = pool;
        old
    }

    /// `fn_1852e8`. Returns the previous flag bit.
    pub fn set_flags(&mut self, flags: u16) -> u16 {
        let old = half(self.arena, LISTS + 0x12);
        set_half(self.arena, LISTS + 0x12, (old & !1) | (flags & 1));
        self.flags = flags;
        u32::from(old & 1) as u16
    }

    /// `fn_1853c4`. Returns the previous fill value.
    pub fn set_fill(&mut self, fill: u8) -> u8 {
        let old = byte(self.arena, LISTS + 0x14);
        set_byte(self.arena, LISTS + 0x14, fill);
        self.fill = fill;
        old
    }

    /// `fn_1850c0`. `alignment` is signed: negative searches from the tail.
    pub fn block_allocate(&mut self, requested: u32, alignment: i32) -> u32 {
        let pointer = self.search(requested, alignment);
        if pointer != 0 {
            let count = word(self.arena, COUNT).wrapping_add(1);
            set_word(self.arena, COUNT, count);
        }
        pointer
    }

    fn search(&mut self, requested: u32, alignment: i32) -> u32 {
        let requested = if requested == 0 { 1 } else { requested };
        let size = requested.wrapping_add(3) & !3;
        let first_fit = u32::from(self.flags) & 1 == 0;
        if alignment >= 0 {
            let alignment = alignment as u32;
            let mut block = word(self.arena, LISTS);
            let mut best = 0u32;
            let mut best_size = u32::MAX;
            let mut address = 0u32;
            while block != 0 {
                let header = read_block(self.arena, block);
                let start = block + HEADER;
                let aligned = (start.wrapping_add(alignment) - 1) & !(alignment - 1);
                let needed = aligned - start + size;
                if header.size >= needed && best_size > header.size {
                    best = block;
                    best_size = header.size;
                    address = aligned;
                    if first_fit || best_size == size {
                        break;
                    }
                }
                block = header.next;
            }
            if best != 0 {
                return self.carve(best, address, size, 0);
            }
        } else {
            let alignment = alignment.wrapping_neg() as u32;
            let mut block = word(self.arena, LISTS + 4);
            let mut best = 0u32;
            let mut best_size = u32::MAX;
            let mut address = 0u32;
            while block != 0 {
                let header = read_block(self.arena, block);
                let start = block + HEADER;
                let aligned = (header.size.wrapping_add(start) - size) & !(alignment - 1);
                if (aligned as i32) - (start as i32) >= 0 && header.size < best_size {
                    best = block;
                    best_size = header.size;
                    address = aligned;
                    if first_fit || best_size == size {
                        break;
                    }
                }
                block = header.previous;
            }
            if best != 0 {
                return self.carve(best, address, size, 1);
            }
        }
        0
    }

    /// `fn_19b814`.
    fn carve(&mut self, block: u32, address: u32, size: u32, which: u32) -> u32 {
        let header = read_block(self.arena, block);
        let header_at = address - HEADER;
        let mut effective = header_at;
        let block_end = block + HEADER + header.size;
        let mut allocation_end = address + size;
        let mut previous = header.previous;
        let start = block_start(block, header.flags);
        unlink(self.arena, LISTS, block);
        let gap = header_at.wrapping_sub(start);
        if gap >= 20 && (which != 0 || self.fill != 0) {
            init_block(self.arena, start, FREE, gap - HEADER);
            link(self.arena, LISTS, previous, start);
            previous = start;
        } else {
            effective = start;
        }
        let gap = block_end.wrapping_sub(allocation_end);
        if gap >= 20 && (which != 1 || self.fill != 0) {
            init_block(self.arena, allocation_end, FREE, gap - HEADER);
            link(self.arena, LISTS, previous, allocation_end);
        } else {
            allocation_end = block_end;
        }
        let gap = allocation_end.wrapping_sub(effective);
        if word(self.arena, ZERO_FLAG) & 1 != 0 {
            for offset in 0..gap / 4 {
                set_word(self.arena, effective + offset * 4, 0);
            }
        }
        init_block(
            self.arena,
            header_at,
            USED,
            allocation_end - header_at - HEADER,
        );
        let mut flags = u32::from(half(self.arena, header_at + 2));
        flags |= which << 15;
        flags = (flags & !0x7f00) | ((header_at.wrapping_sub(effective) & 0x7f) << 8);
        flags = (flags & !0xff) | (u32::from(self.pool) & 0xff);
        set_half(self.arena, header_at + 2, flags as u16);
        let tail = word(self.arena, LISTS + 12);
        link(self.arena, LISTS + 8, tail, header_at);
        address
    }

    /// `fn_184f84`.
    pub fn block_release(&mut self, pointer: u32) {
        if pointer == 0 {
            return;
        }
        let block = pointer - HEADER;
        let header = read_block(self.arena, block);
        let mut end = pointer + header.size;
        let mut start = block_start(block, header.flags);
        unlink(self.arena, LISTS + 8, block);
        let mut before = 0u32;
        let mut current = word(self.arena, LISTS);
        while current != 0 && start > current {
            before = current;
            current = read_block(self.arena, current).next;
        }
        if current != 0 && end == current {
            end = current + HEADER + read_block(self.arena, current).size;
            unlink(self.arena, LISTS, current);
        }
        if before != 0 && before + HEADER + read_block(self.arena, before).size == start {
            start = before;
            before = unlink_at(self.arena, LISTS, before);
        }
        if end.wrapping_sub(start) >= HEADER {
            init_block(self.arena, start, FREE, end - start - HEADER);
            link(self.arena, LISTS, before, start);
        }
    }

    /// `fn_6bab68`: grow or shrink `pointer` in place, absorbing the following
    /// free block. Returns the new payload size, or `None` when it cannot grow.
    pub fn block_resize(&mut self, pointer: u32, size: u32) -> Option<u32> {
        let want = size.wrapping_add(3) & !3;
        let block = pointer - HEADER;
        let old = read_block(self.arena, block).size;
        if old == want {
            return Some(want);
        }
        if old < want {
            let end = pointer + old;
            let mut current = word(self.arena, LISTS);
            while current != 0 && current != end {
                current = read_block(self.arena, current).next;
            }
            if current == 0 {
                return None;
            }
            let absorbed = read_block(self.arena, current);
            if want > old + absorbed.size + HEADER {
                return None;
            }
            let end = current + HEADER + absorbed.size;
            let base = block_start(current, absorbed.flags);
            let previous = absorbed.previous;
            unlink(self.arena, LISTS, current);
            let mut new_end = pointer + want;
            if end.wrapping_sub(new_end) < HEADER {
                new_end = end;
            }
            set_word(self.arena, block + 4, new_end - pointer);
            let left = end.wrapping_sub(new_end);
            if left >= HEADER {
                init_block(self.arena, new_end, FREE, left - HEADER);
                link(self.arena, LISTS, previous, new_end);
            }
            let span = new_end.wrapping_sub(base);
            if word(self.arena, ZERO_FLAG) & 1 != 0 && span > 3 {
                // 1:1 from C (fn_6bab68.c): zero [base, base+(span&~3)) with
                // the C's write pattern: word at base if bit2 set, then
                // 8-byte pairs. Same final state; kept 1:1 for write fidelity.
                let mut w = base.wrapping_sub(4);
                let mut n = span;
                if (n.wrapping_shl(29) as i32) < 0 {
                    w = w.wrapping_add(4);
                    set_word(self.arena, w, 0);
                }
                n >>= 3;
                while n != 0 {
                    n -= 1;
                    set_word(self.arena, w.wrapping_add(4), 0);
                    w = w.wrapping_add(8);
                    set_word(self.arena, w, 0);
                }
            }
            return Some(read_block(self.arena, block).size);
        }
        set_word(self.arena, block + 4, want);
        let end = pointer + old;
        let new_end = pointer + want;
        let mut before = 0u32;
        let mut current = word(self.arena, LISTS);
        while current != 0 && new_end > current {
            before = current;
            current = read_block(self.arena, current).next;
        }
        let mut end = end;
        if current != 0 && current == end {
            let absorbed = read_block(self.arena, current);
            end = current + HEADER + absorbed.size;
            unlink(self.arena, LISTS, current);
        }
        let mut new_end = new_end;
        if before != 0 && before + HEADER + read_block(self.arena, before).size == new_end {
            let previous = read_block(self.arena, before).previous;
            new_end = before;
            unlink(self.arena, LISTS, before);
            before = previous;
        }
        let left = end.wrapping_sub(new_end);
        if left < HEADER {
            set_word(self.arena, block + 4, old);
        } else {
            init_block(self.arena, new_end, FREE, left - HEADER);
            link(self.arena, LISTS, before, new_end);
        }
        Some(read_block(self.arena, block).size)
    }
}

/// `fn_184f84`'s inner merge step needs the removed predecessor.
fn unlink_at(arena: &mut [u8], list_offset: u32, block: u32) -> u32 {
    let previous = word(arena, block + 8);
    unlink(arena, list_offset, block);
    previous
}

impl Heap for EngineHeap<'_> {
    fn allocate(&mut self, size: u32) -> Option<u32> {
        let pointer = self.block_allocate(size, self.alignment as i32);
        (pointer != 0).then_some(pointer)
    }

    fn allocate_zeroed(&mut self, count: u32, size: u32) -> Option<u32> {
        let bytes = count.wrapping_mul(size);
        let pointer = self.block_allocate(bytes, self.alignment as i32);
        if pointer == 0 {
            return None;
        }
        for offset in 0..bytes {
            self.arena[(pointer + offset) as usize] = 0;
        }
        Some(pointer)
    }

    fn reallocate(&mut self, pointer: u32, size: u32) -> Option<u32> {
        if pointer == 0 {
            return self.allocate(size);
        }
        if self.block_resize(pointer, size).is_some() {
            return Some(pointer);
        }
        let fresh = self.block_allocate(size, self.alignment as i32);
        if fresh == 0 {
            return None;
        }
        for offset in 0..size {
            self.arena[(fresh + offset) as usize] = 0;
        }
        let old = self.block_size(pointer);
        let copy = old.min(size);
        for offset in 0..copy {
            let byte = self.arena[(pointer + offset) as usize];
            self.arena[(fresh + offset) as usize] = byte;
        }
        self.block_release(pointer);
        Some(fresh)
    }

    fn release(&mut self, pointer: u32) {
        self.block_release(pointer);
    }

    fn arena(&mut self) -> &mut [u8] {
        self.arena
    }
}

impl EngineHeap<'_> {
    /// `fn_6baddc`: the payload size of a live block.
    pub fn block_size(&self, pointer: u32) -> u32 {
        word(self.arena, pointer - HEADER + 4)
    }

    /// The arena, for tests that compare whole images.
    pub fn arena(&self) -> &[u8] {
        self.arena
    }

    /// The arena, mutably.
    pub fn arena_mut(&mut self) -> &mut [u8] {
        self.arena
    }
}
