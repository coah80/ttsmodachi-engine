//! Live component root 21a25c, split from component_live_c.
//!
//! fn_21a25c is the language/spelling processor of the speech pipeline. The
//! root body itself is engine_components2::process_21a25c (wire-b2). This file
//! binds the families that no other lane binds for this root:
//! the property fetch (fn_175a1c) through speech_part_2::fetch_parameter_m,
//! the fe_text qsort comparators that the marker sort dispatches as code
//! pointers, the morphology decode family (fn_26bb6c over the certified
//! fn_26c1c0 trie) and the archive range read (fn_17c9d8) of the broker map
//! over the game's resource file in the 3DS linear memory.

use crate::engine::{Error, Memory};
use crate::engine_components::InternalCalls;

/// Canonical tokens for the fe_text qsort comparators.
/// The comparator is a code pointer, so the C model, the ARM model and Rust
/// record one token per comparator instead of a raw address.
pub const QSORT_TOKENS: [u32; 3] = [0x9000_0000, 0x9000_0001, 0x9000_0002];

/// fn_21a1fc is the marker-record comparator of the fn_2409d4 sort.
pub const MARKER_COMPARATOR: u32 = 0x21a1fc;

/// fn_196980 is the only string comparator the property tree may use.
pub const PROPERTY_COMPARATOR: u32 = 0x196980;

/// The live arena is one flat region. A flat view lets the ports that were
/// written against the flat boot arena run against the same bytes.
pub fn flat_view<'x, 'b>(m: &'x mut Memory<'_, 'b>) -> (u32, &'x mut [u8]) {
    let regions = &mut *m.regions;
    assert_eq!(regions.len(), 1, "the live arena must be one flat region");
    let first = &mut regions[0];
    (first.base, &mut first.bytes[..])
}

/// The component platform and the heap platform have the same three services.
struct PlatformAdapter<'a, P>(&'a mut P);
impl<P: crate::engine_components2::ComponentPlatform> crate::engine::HeapPlatform
    for PlatformAdapter<'_, P>
{
    fn thread_id(&mut self) -> u32 {
        self.0.thread_id()
    }
    fn arbiter_address(&self) -> u32 {
        self.0.arbiter_address()
    }
    fn arbitrate(&mut self, m: &mut crate::boot::Memory<'_>, a: &[u32]) -> u32 {
        self.0.arbitrate(m, a)
    }
}

/// The property registry lock, taken by fn_1756d0 around the tree lookup.
struct ParamLock {
    object: u32,
    error: Option<u32>,
}
impl ParamLock {
    /// The registry lock of fn_176f2c/fn_176f5c (frontend acquire_lock):
    /// a null slot is a no-op with no host call; a set slot dispatches the
    /// lock method, which has no bound port here and fails closed.
    fn run(&mut self, mem: &mut [u8], slot: u32) -> u32 {
        let p = self.object;
        if p == 0 {
            return 0x8140_2007;
        }
        let word = |a: u32| {
            u32::from_le_bytes(mem[a as usize..a as usize + 4].try_into().unwrap())
        };
        let inner = word(word(p).wrapping_add(8));
        let method = word(inner.wrapping_add(slot));
        if method == 0 {
            return 0;
        }
        self.error = Some(method);
        0x8000_0000
    }
}
impl crate::reentrant2::StateLockM for ParamLock {
    fn acquire_m(&mut self, mem: &mut [u8]) -> u32 {
        self.run(mem, 8)
    }
    fn release_m(&mut self, mem: &mut [u8]) -> u32 {
        self.run(mem, 12)
    }
}

/// The lock methods of the property registry object.
struct LockHost<'a, P>(&'a mut P);
impl<P: crate::engine_components2::ComponentPlatform> crate::boot_part_b::Host for LockHost<'_, P> {
    fn thread_id(&mut self) -> u32 {
        self.0.thread_id()
    }
    fn call(&mut self, m: &mut crate::boot::Memory<'_>, entry: u32, a: &[u32]) -> u32 {
        match entry {
            0x183edc => {
                let handle = m.word(self.0.arbiter_address());
                crate::boot_part_b2::fn_183edc(m, a[0], handle, self);
                0
            }
            0x165d00 => {
                let g = crate::boot::Globals {
                    manager: 0,
                    callbacks: 0,
                    rules: 0,
                    components: 0,
                    dictionary: 0,
                    context: 0,
                    mode: 0,
                    semaphore: self.0.arbiter_address(),
                    service: 0,
                    command: 0,
                };
                m.fn_165d00(self, &g, a[0]);
                0
            }
            0x172d18 => self.0.arbitrate(m, a),
            _ => 0x8000_2001,
        }
    }
}
impl<P: crate::engine_components2::ComponentPlatform> crate::boot::Calls for LockHost<'_, P> {
    fn call(&mut self, m: &mut crate::boot::Memory<'_>, entry: u32, a: &[u32]) -> u32 {
        crate::boot_part_b::Host::call(self, m, entry, a)
    }
    fn call_wide(&mut self, _: &mut crate::boot::Memory<'_>, _: u32, _: &[u32]) -> u64 {
        0
    }
}

/// The live host of the archive path: the broker map, the archive read, the
/// heap and the log.
///
/// The ROM's resource stream dispatches its methods through the code pointers
/// of its own vtable (0x1b2760 seeks, 0x1b254c reads), and the stream's seek
/// reaches the archive's own vtable (0x172de0 seeks, 0x172d30 reads, 0x4a78f0
/// copies). Every one of those addresses is a real ROM code address, so the
/// host dispatches them to the landed ports instead of guessing.
struct MorphHost<'a> {
    platform: &'a mut dyn crate::engine::HeapPlatform,
    workspace: &'a core::cell::RefCell<crate::component_live_c::DictionaryWorkspace>,
    scratch: u32,
    first: &'a mut Option<u32>,
    rom: crate::rom_region::Rom,
}
impl crate::component_live_c::Host for MorphHost<'_> {
    fn call_m(&mut self, f: u32, a: &[u32], m: &mut crate::boot::Memory<'_>) -> u32 {
        // Nested ports pass guest/nested callee addresses raw; normalize the
        // regional twin to the US entry the arms below dispatch on.
        let f = crate::rom_region::to_us_guest(self.rom, f);
        match f {
            0x1738e8 | 0x173dc0 => match crate::component_live_c::core_call_m(m, f, a) {
                Ok(rc) => rc,
                Err(_) => 0x8000_2001,
            },
            // The logger's variadic argument service, not a guest call.
            0xffff0001 => 0,
            0x196394 | 0x1963b8 | 0x196428 | 0x1963e0 | 0x4a779c | 0x4a7758 | 0x4a79b8
            | 0x4a774c | 0x39f5e8 | 0x39f5b8 | 0x184f20 | 0x1840fc | 0x18423c => {
                match crate::component_live_c::heap_call_m(m, f, a, self.platform) {
                    Ok(rc) => rc,
                    Err(crate::engine::Error::MissingStreamAdapter { entry }) => {
                        *self.first = Some(entry);
                        0x8000_2001
                    }
                    Err(_) => 0x8000_2001,
                }
            }
            0x17a1a4 | 0x179208 | 0x17c9d8 | 0x179374 | 0x1b2760 | 0x1b254c | 0x1b2974
            | 0x172d30 | 0x172de0 | 0x4a794c | 0x4a78f0 => {
                crate::component_live_c::broker_range_live_m(m, f, a, self.scratch, self)
            }
            _ => {
                *self.first = Some(f);
                0x8000_2001
            }
        }
    }
}

/// The host of the morphology ports (lane b's fn_26c1c0 and lane b4's
/// fn_26bb6c).  The ports reach the broker map and unmap of this lane's
/// archive binding and the ROM's case fold; every other callee is recorded
/// as unbound.
struct TrieHost<'a, 'h>(&'a mut MorphHost<'h>);
impl crate::component_live_b::Host for TrieHost<'_, '_> {
    fn call_m(&mut self, entry: u32, args: &[u32], m: &mut crate::boot::Memory<'_>) -> u32 {
        let entry = crate::rom_region::to_us_guest(self.0.rom, entry);
        match entry {
            0x17a1a4 | 0x179208 | 0x17c9d8 | 0x179374 | 0x1b2760 | 0x1b254c | 0x1b2974
            | 0x172d30 | 0x172de0 | 0x4a794c | 0x4a78f0 => {
                let scratch = self.0.scratch;
                crate::component_live_c::broker_range_live_m(m, entry, args, scratch, &mut *self.0)
            }
            0x240b28 => crate::component_live_c::fn_240b28(args[0]),
            _ => {
                *self.0.first = Some(entry);
                0x8000_2001
            }
        }
    }
}

/// Live bindings for the fn_21a25c subtree.
/// The bridge owns the guest heap, the stream ports and the property registry.
/// This type adds the property fetch, the marker qsort and the morphology
/// decode entry, and records the first callee it cannot serve.
pub struct C3Calls<'a, P> {
    pub inner: crate::engine_components2::BridgeCalls<'a, P>,
    pub first_unbound: Option<u32>,
    pub qsort_calls: u32,
    pub fetch_calls: u32,
    pub morphology_calls: u32,
    pub archive_calls: u32,
    pub scratch: u32,
    pub dictionary: core::cell::RefCell<crate::component_live_c::DictionaryWorkspace>,
}

impl<'a, P: crate::engine_components2::ComponentPlatform> C3Calls<'a, P> {
    pub fn new(workspace: &'a mut [u8], platform: &'a mut P) -> Self {
        C3Calls {
            inner: crate::engine_components2::BridgeCalls { workspace, platform, rom: crate::rom_region::Rom::Us },
            first_unbound: None,
            qsort_calls: 0,
            fetch_calls: 0,
            morphology_calls: 0,
            archive_calls: 0,
            scratch: 0,
            dictionary: core::cell::RefCell::new(
                crate::component_live_c::DictionaryWorkspace::default(),
            ),
        }
    }
    fn unbound(&mut self, entry: u32) -> u32 {
        if self.first_unbound.is_none() {
            self.first_unbound = Some(entry);
        }
        0x8000_2001
    }
    /// fn_175a1c. The property tree is read from the live arena and the
    /// caller-owned out word is written with the fetched value.
    fn fetch_parameter(&mut self, m: &mut Memory<'_, '_>, a: &[u32]) -> Result<u32, Error> {
        self.fetch_calls += 1;
        if a.len() < 3 || a[0] == 0 || a[1] == 0 || a[2] == 0 {
            return Ok(0x8b70_2007);
        }
        let (base, bytes) = flat_view(m);
        let mut mem = crate::boot::Memory { base, bytes };
        let tree = mem.word(a[0] + 8);
        // The comparator address is stored in the guest property tree, so the
        // regional ROMs hold their own twin (EU 0x196a30 / KR 0x196a6c);
        // normalize to the US entry the port below dispatches on.
        let comparator = if mem.word(a[0] + 4) != 0 && tree != 0 {
            let raw = mem.word(mem.word(tree + 60) + 8);
            crate::rom_region::to_us_guest(self.inner.rom, raw)
        } else {
            PROPERTY_COMPARATOR
        };
        if comparator != PROPERTY_COMPARATOR {
            let entry = comparator;
            return Ok(self.unbound(entry));
        }
        let object = mem.word(a[0] + 4);
        let mut lock = ParamLock { object, error: None };
        let cmp = |x: u32, y: u32, bytes: &mut [u8]| {
            let mut view = crate::boot::Memory { base: 0, bytes };
            crate::speech_part_1::fn_196980(&mut view, x, y) as i32
        };
        let rc = crate::speech_part_2::fetch_parameter_m(&mut mem, a[0], a[1], a[2], &mut lock, cmp);
        if let Some(entry) = lock.error {
            return Ok(self.unbound(entry));
        }
        Ok(rc)
    }
    /// fn_17a1a4 / fn_179208 / fn_17c9d8. The broker map over the live
    /// archive: the ROM's stream vtable and the archive's own vtable are read
    /// from the arena, so the ROM's code addresses drive the landed ports.
    fn archive_range(&mut self, m: &mut Memory<'_, '_>, entry: u32, a: &[u32]) -> u32 {
        self.archive_calls += 1;
        let scratch = self.scratch;
        let C3Calls {
            inner,
            first_unbound,
            dictionary,
            ..
        } = self;
        let (base, bytes) = flat_view(m);
        let mut mem = crate::boot::Memory { base, bytes };
        let mut adapter = PlatformAdapter(inner.platform);
        let mut host = MorphHost {
            platform: &mut adapter,
            workspace: dictionary,
            scratch,
            first: first_unbound,
            rom: inner.rom,
        };
        crate::component_live_c::broker_range_live_m(&mut mem, entry, a, scratch, &mut host)
    }
    /// fn_26bb6c. The morphology decode over the certified fn_26c1c0 trie.
    fn morphology_decode(&mut self, m: &mut Memory<'_, '_>, a: &[u32]) -> Result<u32, Error> {
        self.morphology_calls += 1;
        if a.len() < 6 {
            return Ok(self.unbound(0x26bb6c));
        }
        let scratch = self.scratch;
        let C3Calls {
            inner,
            first_unbound,
            dictionary,
            ..
        } = self;
        let (base, bytes) = flat_view(m);
        let mut mem = crate::boot::Memory { base, bytes };
        let mut adapter = PlatformAdapter(inner.platform);
        let mut host = MorphHost {
            platform: &mut adapter,
            workspace: dictionary,
            scratch: scratch + 512,
            first: first_unbound,
            rom: inner.rom,
        };
        let mut trie = TrieHost(&mut host);
        Ok(crate::component_live_b4::fn_26bb6c_m(
            &mut mem, &mut trie, a[0], a[1], a[2], a[3], a[4], a[5],
        ))
    }
    /// fn_26c1c0. The trie lookup of the morphology family.
    fn morphology_trie(&mut self, m: &mut Memory<'_, '_>, a: &[u32]) -> Result<u32, Error> {
        self.morphology_calls += 1;
        if a.len() < 5 {
            return Ok(self.unbound(0x26c1c0));
        }
        let scratch = self.scratch;
        let C3Calls {
            inner,
            first_unbound,
            dictionary,
            ..
        } = self;
        let (base, bytes) = flat_view(m);
        let mut mem = crate::boot::Memory { base, bytes };
        let mut adapter = PlatformAdapter(inner.platform);
        let mut host = MorphHost {
            platform: &mut adapter,
            workspace: dictionary,
            scratch: scratch + 512,
            first: first_unbound,
            rom: inner.rom,
        };
        let mut trie = TrieHost(&mut host);
        let rc = crate::component_live_b::fn_26c1c0_m(
            &mut mem, &mut trie, a[0], a[1], a[2] as i32, a[3] as i32, a[4] as i32,
        );
        Ok(rc as u32)
    }
    /// fn_1ea4e0. The dictionary lookup: the shared prefix owns the
    /// prologue, and the ROM's decode (fn_26bb6c) owns the entry.
    fn dictionary_lookup(&mut self, m: &mut Memory<'_, '_>, a: &[u32]) -> Result<u32, Error> {
        match crate::engine_components2::read_call(m, 0x1ea4e0, a, &mut *self) {
            Err(Error::UnsupportedStage { entry: 0x26c1c0 }) => {
                // The prefix probe maps one dictionary page and leaves it
                // mapped; the ROM's found-path never maps here, so release
                // the page before the decode. The alloc and this free are
                // adjacent, keeping the heap neutral.
                let ctx = a[0];
                let state = ctx + 8;
                let (buffer, broker, handle) = {
                    let (base, bytes) = flat_view(m);
                    let mem = crate::boot::Memory { base, bytes };
                    (mem.word(state + 8), mem.word(state + 16), mem.word(state + 20))
                };
                if buffer != 0 {
                    self.archive_range(m, 0x179208, &[broker, handle, buffer]);
                }
                self.dictionary_decode(m, a)
            }
            other => other,
        }
    }
    /// The decode step of fn_1ea4e0: the ROM calls fn_26bb6c with the query
    /// and the two halfword bounds (src/fn_1ea4e0.c).
    fn dictionary_decode(&mut self, m: &mut Memory<'_, '_>, a: &[u32]) -> Result<u32, Error> {
        if a.len() < 6 {
            return Ok(self.unbound(0x1ea4e0));
        }
        let scratch = self.scratch + 1024;
        let ctx = a[0];
        let state = ctx + 8;
        let query = {
            let (base, bytes) = flat_view(m);
            let mut mem = crate::boot::Memory { base, bytes };
            let query = mem.word(ctx + 0x74);
            let length = crate::fe_text::strlen_195d90(mem.bytes, query - base) as i16;
            mem.put_half(scratch, 0);
            mem.put_half(scratch + 2, (length - 1) as u16);
            mem.put_half(scratch + 4, 0);
            query
        };
        let found = self.morphology_decode(
            m,
            &[state, query, scratch, scratch + 2, 0, scratch + 4],
        )?;
        if found == 0 {
            return Ok(0);
        }
        // The ROM copies the decoded entry into the result slot and counts it.
        let (base, bytes) = flat_view(m);
        let mut mem = crate::boot::Memory { base, bytes };
        let decoded = mem.word(state + 24);
        let size = u16::from_le_bytes([mem.byte(scratch + 4), mem.byte(scratch + 5)]);
        let results = mem.word(ctx + 0x70);
        let count = u16::from_le_bytes([mem.byte(a[5]), mem.byte(a[5] + 1)]) as u32;
        let slot = mem.word(results + count * 4);
        for index in 0..size as usize {
            let byte = mem.byte(decoded + index as u32);
            mem.put_byte(slot + index as u32, byte);
        }
        mem.put_byte(slot + size as u32, 0);
        mem.put_half(a[5], (count + 1) as u16);
        Ok(0)
    }
}

/// The marker sort of fn_2409d4 with the fn_21a1fc comparator.
/// The comparator is a code pointer, so the call is only served for the
/// canonical comparator of this subtree.
pub fn qsort_markers(arena: &mut [u8], base: u32, count: u32, size: u32) -> u32 {
    if size != 24 {
        return 0x8000_2001;
    }
    fn compare(mem: &[u8], a: u32, b: u32) -> i32 {
        let mut bytes = [0u8; 48];
        bytes[..24].copy_from_slice(&mem[a as usize..a as usize + 24]);
        bytes[24..].copy_from_slice(&mem[b as usize..b as usize + 24]);
        let mut regions = [crate::engine::Region { base: 0, bytes: &mut bytes }];
        let m = crate::engine::Memory::new(&mut regions).expect("local comparator");
        crate::engine_components2::compare_events(&m, 0, 24).expect("mapped records") as i32
    }
    crate::fe_text::qsort_2409d4(arena, base, count, size, compare);
    0
}

impl<P: crate::engine_components2::ComponentPlatform> InternalCalls for C3Calls<'_, P> {
    fn call(&mut self, m: &mut Memory<'_, '_>, entry: u32, a: &[u32]) -> Result<u32, Error> {
        // process_21a25c dispatches through invoke(), which reads the callee
        // address raw from the guest tables: in EU/KR runs those are the
        // relocated twins. Normalize guest-sourced pointers to the US entry
        // the arms below dispatch on (round-8 recipe: to_us_guest). US runs
        // pass through unchanged (genuine US keys are identity).
        let entry = crate::rom_region::to_us_guest(self.inner.rom, entry);
        match entry {
            0x175a1c => self.fetch_parameter(m, a),
            0x2409d4 => {
                self.qsort_calls += 1;
                if a.len() < 4 || a[3] != MARKER_COMPARATOR {
                    return Ok(self.unbound(a.get(3).copied().unwrap_or(entry)));
                }
                let (_, bytes) = flat_view(m);
                Ok(qsort_markers(bytes, a[0], a[1], a[2]))
            }
            0x26bb6c => self.morphology_decode(m, a),
            0x26c1c0 => self.morphology_trie(m, a),
            0x17a1a4 | 0x179208 | 0x17c9d8 => Ok(self.archive_range(m, entry, a)),
            // The dictionary lookup runs over the live arena with this lane's
            // host, so its broker map reaches the archive binding above.
            0x1ea4e0 => self.dictionary_lookup(m, a),
            _ => self.inner.call(m, entry, a),
        }
    }
}
