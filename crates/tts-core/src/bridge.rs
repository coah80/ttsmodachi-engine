//! Memory bridge. Guest pointers always remain absolute u32 addresses.
//!
//! Call boot_view for a flat port whose complete working range is in one region.
//! For ports indexing slices by absolute address, Flat stages mapped bytes in
//! caller-owned, zero-based workspace. It never rebases pointers or payloads.
//! finish checks holes for nonzero bytes and copies mapped bytes back. The workspace must extend
//! through the highest mapped address; this is intentionally a capacity cost,
//! not a hidden allocation. No script results or recorded outputs are consumed.
//!
//! Address routing: fn_184f20/fn_184f84 are concrete block methods;
//! fn_196394/fn_1963b8/fn_1963e0/fn_196428 dispatch guest allocator slots.
//! Concrete realloc fn_4a79b8 stages only the heap header for block_resize,
//! runs real locking, and preserves the original fallback allocate/zero/copy/free.
//! A failed fallback followed by nonzero memcpy is an explicit bridge error:
//! the original faults at its null destination. No generic realloc policy is used.
//!
//! Round-1 signatures remain unchanged, including their intentional callback
//! refusals. Use *_with for live resource create, stream copy/close, nonempty
//! map destruction and LINGDB error logging. fn_19668c and fn_19126c are new.
//! These are bridge-side ports where the typed ports hold an exclusive slice
//! across callbacks, or replace dependencies with preselected scripts. The
//! callback-free red-black rebalance is shared with the existing frontend port.
//!
//! Implement Live on the engine's dependency dispatcher. call receives a fresh
//! &mut Flat, so it can allocate or invoke another adapter before returning.
//! memory() gives a temporary absolute-address boot view of those same staged
//! bytes. Never capture canonical Memory, retain the view, create another Flat
//! on it, or call finish in a callback. Finish once at the outermost boundary.
//! Internal allocator methods stay concrete; Platform supplies only thread ID
//! and the kernel arbitration boundary. A missing Live dependency must reject,
//! not return a fabricated success or consume OpenWorkerScripts.
//!
//! Reserve a disjoint mapped guest scratch word for fn_19668c and 64 bytes for
//! fn_19126c. Both restore it on return. Nested invocations need separate space.
//! WorkerData supplies the nine documented regional literal addresses. The
//! worker currently uses US callback entry addresses. Its c72000 notification
//! carries [original_r0 = 0x191938, output_pair_address]; the second value makes
//! the ARM hook's live r8 explicit. The matched C's one-argument handle hook is
//! not the dumped ARM ABI. All worker success, failure and cleanup calls are live.
//!
//! Logging is an explicit semantic boundary, not omitted formatting. Live::log
//! gets a site, logger and values. Sites 19126c/191860, 14f918/14f919 and
//! 1503b0/1503b1 mean begin/end; 1503b2 is invalid-input detail. Other sites
//! are the original error codes. Worker errors carry status, LINGDB 4e34 carries
//! record index and 4e30 carries [type,index,field]. Stream begin events carry
//! [id,stream,kind_or_count]; end carries status. Other stream error events carry
//! their original call arguments or [stream,written,base]. The logger dispatcher
//! owns module names, strings and formatting. fn_14e614 trace formatting,
//! fn_14e440 compaction, fn_196c7c reader-list teardown, fn_196e28 reader removal
//! and logInput fn_14e0b4 destruction are live call dependencies, not silent stubs.
//!
//! ARM tests compare all mapped bytes and a semantic callback trace, with real
//! heap/rotations/realloc and memory-mutating dependency implementations. They
//! do not certify the dependency implementations in the combined engine.
//! Flat also implements Heap. Call finish even after a refused allocation to
//! publish configuration writes. Guest graphs must be well formed and every
//! accessed byte mapped. Staging is not an MMU and cannot detect zero writes
//! into holes. The workspace capacity cost and US-only dispatch still apply.

use crate::{boot, engine::{Error, Memory}, frontend, heap::Heap, linguistic};

/// A callee address read from guest memory (allocator vtable and object
/// method slots). The regional ROMs relocate code, so the slot holds that
/// region's twin; normalize to the US entry the arms below dispatch on.
/// This is the ReadCalls pattern (engine_components2.rs): from_entry only
/// names an unambiguous twin, so genuine US entries pass through unchanged
/// and US runs are bit-identical. src/fn_4a7758.c dispatches the same way
/// (`o->m8(p, n, k, k)`); the EU twin of US 0x4a7758 is EU 0x4a7f30
/// (eu/data/map.json), which the raw dispatch failed closed on.
fn guest_entry(target: u32) -> u32 {
    let rom = crate::rom_region::Rom::from_entry(target);
    crate::rom_region::to_us(rom, target)
}

/// Borrow the entire region containing [address, address + length).
/// All addresses touched by the flat port must remain in this same region.
pub(crate) fn boot_view<'m>(
    memory: &'m mut Memory<'_, '_>, address: u32, length: usize,
) -> Result<boot::Memory<'m>, Error> {
    let end = u64::from(address).checked_add(length as u64)
        .filter(|end| *end <= 0x1_0000_0000).ok_or(Error::InputTooLong)?;
    for region in memory.regions.iter_mut() {
        if address >= region.base && end <= u64::from(region.base) + region.bytes.len() as u64 {
            return Ok(boot::Memory { base: region.base, bytes: &mut *region.bytes });
        }
    }
    Err(Error::UnmappedMemory { address, bytes: length })
}

/// CP15 identity and the kernel address-arbitration boundary.
pub(crate) trait Platform {
    fn thread_id(&mut self) -> u32;
    fn arbiter_address(&self) -> u32;
    fn arbitrate(&mut self, memory: &mut boot::Memory<'_>, args: &[u32]) -> u32;
}

struct LockHost<'a, P>(&'a mut P);
impl<P: Platform> crate::boot_part_b::Host for LockHost<'_, P> {
    fn thread_id(&mut self) -> u32 { self.0.thread_id() }
    fn call(&mut self, m: &mut boot::Memory<'_>, entry: u32, a: &[u32]) -> u32 {
        match entry {
            0x183edc => {
                let handle = m.word(self.0.arbiter_address());
                crate::boot_part_b2::fn_183edc(m, a[0], handle, self); 0
            }
            0x165d00 => {
                let g = boot::Globals {manager:0,callbacks:0,rules:0,components:0,
                    dictionary:0,context:0,mode:0,semaphore:self.0.arbiter_address(),
                    service:0,command:0};
                m.fn_165d00(self, &g, a[0]); 0
            }
            0x172d18 => self.0.arbitrate(m, a),
            _ => unreachable!("mutex port reached non-platform dependency"),
        }
    }
}
impl<P: Platform> boot::Calls for LockHost<'_, P> {
    fn call(&mut self, m: &mut boot::Memory<'_>, entry: u32, a: &[u32]) -> u32 {
        crate::boot_part_b::Host::call(self, m, entry, a)
    }
    fn call_wide(&mut self, _: &mut boot::Memory<'_>, _: u32, _: &[u32]) -> u64 {
        unreachable!("mutex has no wide call")
    }
}

/// Caller-owned absolute-index workspace over non-contiguous engine memory.
///
/// Fast path (`memory: None`): a single base-0 region already is the flat
/// view staging would build (fill(0), then copy each region in), so `bytes`
/// aliases the live region with no staging copy and `finish` publishes
/// nothing. The hole scan is vacuous (no gaps) and every guest read/write
/// lands at the same address, so success-path behavior is identical while
/// the workspace stays untouched. On a recorded `error` the live region
/// keeps the partial mutations (there is no staging copy to roll back to);
/// callers treat a failed request's arena as discarded, the same as after
/// any error. The workspace is still caller-owned and capacity-checked.
pub(crate) struct Flat<'m, 'r, 'b, 'p, P> {
    memory: Option<&'m mut Memory<'r, 'b>>,
    bytes: &'m mut [u8],
    platform: &'p mut P,
    allocator: u32,
    error: Option<Error>,
}

impl<'m, 'r, 'b, 'p, P: Platform> Flat<'m, 'r, 'b, 'p, P> {
    /// Stage guest mappings without normalizing any pointer-looking payload.
    pub(crate) fn new<'w>(memory: &'m mut Memory<'r, 'b>, workspace: &'w mut [u8],
        platform: &'p mut P, allocator: u32) -> Result<Self, Error>
    where 'w: 'm {
        let required = memory.regions.iter().map(|r| r.base as usize + r.bytes.len()).max().unwrap_or(0);
        if workspace.len() < required {
            return Err(Error::Capacity { required, available: workspace.len() });
        }
        if memory.regions.len() == 1
            && memory.regions[0].base == 0
            && !memory.regions[0].bytes.is_empty()
        {
            let bytes: &'m mut [u8] = &mut *memory.regions[0].bytes;
            return Ok(Self { memory: None, bytes, platform, allocator, error: None });
        }
        let bytes = &mut workspace[..required];
        bytes.fill(0);
        for r in memory.regions.iter() {
            bytes[r.base as usize..r.base as usize + r.bytes.len()].copy_from_slice(r.bytes);
        }
        Ok(Self { memory: Some(memory), bytes, platform, allocator, error: None })
    }

    /// Validate holes before publishing. An error leaves canonical memory unchanged.
    pub(crate) fn finish(mut self) -> Result<(), Error> {
        if let Some(error) = self.error { return Err(error); }
        let Some(memory) = self.memory.take() else { return Ok(()); };
        // Regions need not be sorted. Visit gaps in guest-address order.
        let mut end = 0usize;
        loop {
            let next = memory.regions.iter()
                .filter(|r| r.bytes.len() != 0 && r.base as usize >= end)
                .min_by_key(|r| r.base);
            let Some(r) = next else { break };
            if let Some(at) = self.bytes[end..r.base as usize].iter().position(|b| *b != 0) {
                return Err(Error::UnmappedMemory {address: (end + at) as u32, bytes: 1});
            }
            end = r.base as usize + r.bytes.len();
        }
        for r in memory.regions.iter_mut() {
            r.bytes.copy_from_slice(&self.bytes[r.base as usize..r.base as usize + r.bytes.len()]);
        }
        Ok(())
    }

    pub(crate) fn word(&self, address: u32) -> u32 {
        u32::from_le_bytes(self.bytes[address as usize..address as usize + 4].try_into().unwrap())
    }

    pub(crate) fn put_word(&mut self, address: u32, value: u32) {
        self.bytes[address as usize..address as usize + 4].copy_from_slice(&value.to_le_bytes());
    }

    fn fail(&mut self, entry: u32) -> u32 {
        self.error = Some(Error::UnsupportedStage { entry });
        0
    }

    fn method(&self, allocator: u32, slot: u32) -> (u32, u32) {
        (guest_entry(self.word(self.word(allocator) + slot * 4)), self.word(allocator + 4))
    }

    /// fn_184f20(object, size, signed_alignment, pool, flags, fill).
    pub(crate) fn fn_184f20(&mut self, a: [u32; 6]) -> u32 {
        // boot's adapter temporarily uses the first 88 bytes, and restores them.
        // Its heap object and every block must be disjoint from that scratch.
        if a[0] < 88 { return self.fail(0x184f20); }
        boot::Memory { base: 0, bytes: self.bytes }.boot_heap_allocate(a)
    }

    /// fn_184f84(subobject = heap + 24, pointer). Does not decrement count.
    pub(crate) fn fn_184f84(&mut self, subobject: u32, pointer: u32) {
        if subobject < 112 { self.fail(0x184f84); return; }
        boot::Memory {base: 0, bytes: self.bytes}.boot_heap_release(subobject, pointer);
    }

    fn lock(&mut self, address: u32) {
        crate::boot_part_b::fn_1840fc(&mut boot::Memory {base:0,bytes:self.bytes},
            address, &mut LockHost(self.platform));
    }
    fn unlock(&mut self, address: u32) {
        crate::boot_part_b::fn_18423c(&mut boot::Memory {base:0,bytes:self.bytes},
            address, &mut LockHost(self.platform));
    }
    fn locked_allocate(&mut self, object: u32, size: u32, alignment: u32, pool: u32) -> u32 {
        let inner = self.word(object + 4);
        self.lock(inner + 88);
        let result = self.fn_184f20([inner, size, alignment, pool, 0, 0]);
        self.unlock(inner + 88);
        result
    }

    /// Concrete methods currently bound by the boot allocator vtable.
    /// Unknown internal methods are rejected, never supplied by Platform.
    pub(crate) fn allocator_method(&mut self, entry: u32, args: &[u32]) -> u32 {
        match entry {
            0x184f20 => self.fn_184f20(args.try_into().expect("six heap arguments")),
            0x184f84 => { self.fn_184f84(args[0], args[1]); 0 }
            0x4a779c => {
                let target = guest_entry(self.word(self.word(args[0]) + 8));
                self.allocator_method(target, &[args[0], args[1], 4])
            }
            0x4a774c => {
                let target = guest_entry(self.word(self.word(args[0]) + 12));
                self.allocator_method(target, &[args[0], args[1]])
            }
            0x39f5b8 => {
                let inner = self.word(args[0] + 4);
                self.lock(inner + 88);
                self.allocator_method(0x19b78c, &[inner, args[1]]);
                self.unlock(inner + 88);
                0
            }
            0x163f20 => self.locked_allocate(args[0], args[1], args[2], args[3]),
            0x39f5e8 => self.locked_allocate(args[0], args[1], args[2], 0),
            0x4a7758 => {
                let n = args[1].wrapping_mul(args[2]);
                let target = guest_entry(self.word(self.word(args[0]) + 8));
                let p = self.allocator_method(target, &[args[0], n, 4, 4]);
                if p != 0 { self.bytes[p as usize..p as usize + n as usize].fill(0); }
                p
            }
            0x19b78c => {
                self.fn_184f84(args[0] + 24, args[1]);
                self.put_word(args[0] + 84, self.word(args[0] + 84).wrapping_sub(1));
                0
            }
            0x4a79b8 => self.concrete_reallocate(args[0], args[1], args[2]),
            _ => self.fail(entry),
        }
    }

    /// fn_196394(allocator, size), preserving the guest vtable and context.
    pub(crate) fn fn_196394(&mut self, allocator: u32, size: u32) -> u32 {
        if allocator == 0 || size == 0 { return 0; }
        let (target, object) = self.method(allocator, 0);
        self.allocator_method(target, &[object, size])
    }

    /// fn_1963b8(allocator, count, size), including the concrete zeroing callback.
    pub(crate) fn fn_1963b8(&mut self, allocator: u32, count: u32, size: u32) -> u32 {
        if allocator == 0 || count == 0 || size == 0 { return 0; }
        let (target, object) = self.method(allocator, 1);
        self.allocator_method(target, &[object, count, size])
    }

    /// fn_196428(allocator, pointer), dispatching slot three, not slot two.
    pub(crate) fn fn_196428(&mut self, allocator: u32, pointer: u32) {
        if allocator == 0 || pointer == 0 { return; }
        let (target, object) = self.method(allocator, 3);
        self.allocator_method(target, &[object, pointer]);
    }

    /// fn_1964e0(ctx, flag, a, b, c, d, e, out). Allocation precedes all writes,
    /// so a completed real allocation can be handed to the existing slice port.
    pub(crate) fn fn_1964e0(&mut self, a: [u32; 8]) -> u32 {
        let valid = a[7] != 0 && a[2..7].iter().all(|x| *x != 0);
        let value = if valid { self.fn_1963b8(a[0], 1, 68) } else { 0 };
        let mut method = CompletedAllocation { value };
        frontend::create_ctx_object(self.bytes, nz(a[0]), a[1], a[2], a[3], a[4],
            a[5], a[6], nz(a[7]), &mut method)
    }

    /// fn_1503b0(stream, tag, source, count), existing-capacity writes without
    /// element callbacks or logging. Growth and callback paths reject atomically.
    pub(crate) fn fn_1503b0(&mut self, a: [u32; 4]) -> u32 {
        if a[0] != 0 && a[1] == 0x47de && a[2] != 0 && a[3] != 0 {
            let kind = self.bytes[a[0] as usize];
            if kind == 1 || kind == 2 {
                let state = self.word(a[0] + 4);
                let written = self.word(state + 44);
                let base = self.word(state + 56);
                if written >= base {
                    let need = written.wrapping_sub(base).wrapping_add(a[3]);
                    let context = self.word(state + 4);
                    if self.word(state + 64) < need || self.word(state + 32) != 0
                        || self.word(context + 16) != 0 || self.bytes[state as usize + 42] != 0 {
                        return self.fail(0x1503b0);
                    }
                    let dst = self.word(state + 60).wrapping_add(written.wrapping_sub(base));
                    if u64::from(dst) < u64::from(a[2]) + u64::from(a[3])
                        && u64::from(a[2]) < u64::from(dst) + u64::from(a[3]) {
                        return self.fail(0x1503b0);
                    }
                }
            }
        }
        frontend::write_stream(self.bytes, nz(a[0]), a[1], nz(a[2]), a[3],
            &mut NoCallbacks, &mut |_,_,_| unreachable!(), &mut |_,_| unreachable!())
    }

    /// fn_19665c(map), empty-tree teardown only. The actual clear and release
    /// ports run; nonempty trees require reentrant destructors and reject first.
    pub(crate) fn fn_19665c(&mut self, object: u32) -> u32 {
        if object == 0 { return 0x89002007; }
        let tree = object + 24;
        if self.word(tree) != tree { return self.fail(0x19665c); }
        let allocator = self.word(object + 20);
        frontend::clear_tree(self.bytes, tree, &mut NoCallbacks, &mut |_,_| unreachable!());
        let mut release = OneRelease::default();
        let rc = frontend::release_cleared(self.bytes, Some(object), &mut |_| {}, &mut release);
        if let Some((_, p)) = release.0 { self.fn_196428(allocator, p); }
        rc
    }

    /// fn_14f918(stream, tag), writer close with a shared state and no logger.
    /// This path decrements references and frees the stream via real ports.
    /// Final-reference destruction, reader unlinking and logging are not bridged.
    pub(crate) fn fn_14f918(&mut self, a: [u32; 2]) -> u32 {
        if a[0] == 0 || a[1] != 0x47de { return 0x81706008; }
        let state = self.word(a[0] + 4);
        let context = self.word(state + 4);
        let refs = u16::from_le_bytes(self.bytes[state as usize+22..state as usize+24].try_into().unwrap());
        if self.bytes[a[0] as usize] == 0 || refs <= 1 || self.word(context + 16) != 0
            || self.bytes[state as usize + 42] != 0 {
            return self.fail(0x14f918);
        }
        let allocator = self.word(context + 4);
        let mut release = OneRelease::default();
        let rc = frontend::close_stream(self.bytes, Some(a[0]), a[1], 0, &mut release,
            &mut |_,_| unreachable!(), &mut |_,_| unreachable!());
        if let Some((_, p)) = release.0 { self.fn_196428(allocator, p); }
        rc
    }

    /// fn_176e40(resource, allocator, out), for the null create method observed
    /// in boot. Non-null creates are rejected before allocation or output writes.
    pub(crate) fn fn_176e40(&mut self, a: [u32; 3]) -> u32 {
        if a.iter().all(|x| *x != 0) && self.word(self.word(a[0] + 8)) != 0 {
            return self.fail(0x176e40);
        }
        let value = if a.iter().all(|x| *x != 0) { self.fn_1963b8(a[1], 1, 12) } else { 0 };
        frontend::create_node(self.bytes, nz(a[0]), nz(a[1]), nz(a[2]),
            &mut CompletedAllocation {value}, &mut NoCallbacks,
            |_, _| unreachable!("non-null create rejected before mutation"))
    }
}

fn nz(p: u32) -> Option<u32> { (p != 0).then_some(p) }

struct CompletedAllocation { value: u32 }
impl frontend::MethodTable for CompletedAllocation {
    fn call_at(&mut self, _: u32, a: u32, b: u32) -> u32 {
        assert_eq!(a, 1);
        assert!(b == 68 || b == 12);
        self.value
    }
}
#[derive(Default)]
struct OneRelease(Option<(u32, u32)>);
impl frontend::HeapVtable for OneRelease {
    fn alloc_at(&mut self, _: u32, _: u32) -> u32 { unreachable!() }
    fn realloc_at(&mut self, _: u32, _: u32, _: u32) -> u32 { unreachable!() }
    fn release_at(&mut self, object: u32, pointer: u32) {
        assert!(self.0.is_none());
        self.0 = Some((object, pointer));
    }
}
struct NoCallbacks;
impl frontend::HeapVtable for NoCallbacks {
    fn alloc_at(&mut self, _: u32, _: u32) -> u32 { unreachable!() }
    fn realloc_at(&mut self, _: u32, _: u32, _: u32) -> u32 { unreachable!() }
    fn release_at(&mut self, _: u32, _: u32) { unreachable!() }
}

impl<P: Platform> Heap for Flat<'_, '_, '_, '_, P> {
    fn allocate(&mut self, size: u32) -> Option<u32> { nz(self.fn_196394(self.allocator, size)) }
    fn allocate_zeroed(&mut self, count: u32, size: u32) -> Option<u32> {
        nz(self.fn_1963b8(self.allocator, count, size))
    }
    fn reallocate(&mut self, pointer: u32, size: u32) -> Option<u32> {
        nz(self.fn_1963e0(self.allocator, pointer, size))
    }
    fn release(&mut self, pointer: u32) { self.fn_196428(self.allocator, pointer); }
    fn arena(&mut self) -> &mut [u8] { self.bytes }
}

/// fn_15d594(database, tag, record_index, field_index, out_u16).
/// descriptors is the regional D_8d7ca0 table, with 272-byte entries.
/// Only the selected record/field is projected; unrelated records are not copied.
/// Error logging is not implemented: error paths return UnsupportedStage rather
/// than dropping observable logging calls.
pub(crate) fn fn_15d594(memory: &mut Memory<'_, '_>, a: [u32; 5],
    descriptors: u32) -> Result<u32, Error> {
    if frontend::validate_handle(a[0] != 0, a[1], 0xb2c2, 0x38) >= 0x80000000 {
        return Ok(0x89202008);
    }
    if a[2] >= memory.word(a[0] + 32)? {
        return Err(Error::UnsupportedStage {entry: 0x173dc0});
    }
    let record = memory.word(memory.word(a[0] + 48)?.wrapping_add(a[2].wrapping_mul(4)))?;
    if record == 0 { return Err(Error::UnsupportedStage {entry: 0x173dc0}); }
    let mut kind = [0];
    memory.read(record + 2, &mut kind)?;
    if kind[0] == 0 { return Err(Error::UnsupportedStage {entry: 0x173dc0}); }
    let desc = descriptors.wrapping_add(u32::from(kind[0]) * 272);
    let mut count = [0; 2];
    memory.read(desc + 4, &mut count)?;
    if a[3] >= u32::from(u16::from_le_bytes(count)) {
        return Err(Error::UnsupportedStage {entry: 0x173dc0});
    }
    let address = record.wrapping_add(memory.word(desc + 8)?.wrapping_sub(memory.word(desc)?))
        .wrapping_add(a[3]);
    let mut state = [0];
    memory.read(address, &mut state)?;
    let records = [Some(linguistic::Record {kind: kind[0], ..Default::default()})];
    let fields = [linguistic::Field {data: None, state: state[0], fixed_count: 0,
        count: None, element_size: 0}];
    let rows: [&[linguistic::Field<'_>]; 1] = [&fields];
    let database = linguistic::FieldDatabase {records: &records, fields: &rows, valid: true};
    let mut output = 0;
    let status = database.field_state(0, 0, &mut output);
    memory.write(a[4], &output.to_le_bytes())?;
    Ok(status)
}

#[cfg(test)]
#[path = "../tests/bridge.rs"]
#[macro_use]
mod test_source;
#[cfg(test)]
mod tests { bridge_tests!(); }


/// Reentrant guest calls. The callback borrows the *live* bridge, never captures
/// its memory. It may call another adapter or allocate before returning.
/// Unknown calls must return an error through reject(), not a scripted answer.
/// Logging uses semantic sites: begin/end addresses or the original error code.
/// A logger implementation owns formatting, not the bridge's control flow.
pub(crate) trait Live {
    fn call<P: Platform>(&mut self, bridge: &mut Flat<'_, '_, '_, '_, P>,
        entry: u32, args: &[u32]) -> u32;
    fn log<P: Platform>(&mut self, bridge: &mut Flat<'_, '_, '_, '_, P>,
        site: u32, logger: u32, args: &[u32]);
}

/// US string addresses needed by worker dependency calls. Kept explicit so
/// callers cannot accidentally use US literals with a different regional image.
/// Order: engine name, waitfactor, maxinputlength, D_18ffc4, D_18ffd4,
/// phonmapmrkenabled, D_191254, LINGDB, D_8d2e48.
pub(crate) struct WorkerData { pub(crate) text: [u32; 9] }

impl<P: Platform> Flat<'_, '_, '_, '_, P> {
    /// Temporary direct view of staged memory, including callback mutations.
    /// Call finish() once, only on the outermost bridge.
    pub(crate) fn memory(&mut self) -> boot::Memory<'_> {
        boot::Memory { base: 0, bytes: self.bytes }
    }
    pub(crate) fn reject(&mut self, entry: u32) -> u32 { self.fail(entry) }

    /// Full resource-create path, including callbacks which allocate reentrantly.
    pub(crate) fn fn_176e40_with(&mut self, a: [u32; 3], host: &mut impl Live) -> u32 {
        if a.iter().any(|x| *x == 0) { return 0x81402007; }
        let n = self.fn_1963b8(a[1], 1, 12);
        self.put_word(a[2], n);
        if n == 0 { return 0x8140200a; }
        self.put_word(n, a[0]); self.put_word(n + 4, a[1]);
        let entry = self.word(self.word(a[0] + 8));
        if entry == 0 { return 0; }
        let rc = host.call(self, entry, &[self.word(a[0]+12), self.word(a[0]+4),
            self.word(a[2])+8]);
        if rc >= 0x80000000 {
            self.fn_196428(a[1], self.word(a[2]));
            self.put_word(a[2], 0);
        }
        rc
    }

    /// fn_19668c(map,key,value). scratch is a disjoint mapped guest word
    /// reserved by the caller for this invocation; nested calls need another.
    /// A bridge-side port is necessary: TreeOps cannot reborrow its slice.
    pub(crate) fn fn_19668c(&mut self, a: [u32; 3], scratch: u32,
        host: &mut impl Live) -> u32 {
        if a.iter().any(|x| *x == 0) { return 0x89002007; }
        let saved = self.word(scratch);
        self.put_word(scratch, 0);
        let tree = a[0]+24;
        let heap = self.word(tree+32);
        let n = self.fn_196394(heap, 24);
        if n == 0 { self.put_word(scratch, saved); return 0x8900200a; }
        let vt = self.word(tree+36);
        let mut rc = host.call(self, self.word(vt+12), &[a[2], heap, scratch]);
        let tmp = self.word(scratch);
        if rc < 0x80000000 {
            for off in [0,4,8,16] { self.put_word(n+off, 0); }
            self.bytes[(n+12) as usize] = 0;
            self.put_word(n+20, tmp);
            rc = self.tree_insert_live(tree, n, a[1], host);
            if rc < 0x80000000 { self.put_word(scratch, saved); return 0; }
            if self.word(n+16) != 0 {
                host.call(self, self.word(self.word(tree+36)+4), &[self.word(n+16), heap]);
            }
            if self.word(n+20) != 0 {
                host.call(self, self.word(self.word(tree+36)+16), &[tmp, heap]);
            }
        }
        self.fn_196428(heap, n);
        self.put_word(scratch, saved);
        rc
    }

    fn tree_insert_live(&mut self, tree: u32, node: u32, key: u32,
        host: &mut impl Live) -> u32 {
        let mut parent = tree;
        let mut x = self.word(tree);
        let mut cmp = -1i32;
        while x != tree {
            parent = x;
            cmp = host.call(self, self.word(self.word(tree+36)+8),
                &[key, self.word(x+16)]) as i32;
            if self.bytes[(tree+40) as usize] == 0 && cmp == 0 { return 0x89002002; }
            x = self.word(x + if cmp < 0 {0} else {4});
        }
        let rc = host.call(self, self.word(self.word(tree+36)),
            &[key, self.word(tree+32), node+16]);
        if rc >= 0x80000000 { return rc; }
        self.put_word(parent + if cmp < 0 {0} else {4}, node);
        self.put_word(node, tree); self.put_word(node+4, tree);
        self.put_word(node+8, parent);
        self.put_word(tree+24, self.word(tree+24).wrapping_add(1));
        self.bytes[(node+12) as usize] = 0;
        frontend::rebalance(self.bytes, tree, node, parent);
        rc
    }

    fn destroy_tree_live(&mut self, tree: u32, n: u32, host: &mut impl Live) {
        if n == tree { return; }
        self.destroy_tree_live(tree, self.word(n), host);
        self.destroy_tree_live(tree, self.word(n+4), host);
        let heap = self.word(tree+32);
        if self.word(n+16) != 0 {
            host.call(self, self.word(self.word(tree+36)+4), &[self.word(n+16), heap]);
        }
        if self.word(n+20) != 0 {
            host.call(self, self.word(self.word(tree+36)+16), &[self.word(n+20), heap]);
        }
        self.fn_196428(self.word(tree+32), n);
    }

    /// Nonempty-tree postorder destruction through the live guest callbacks.
    pub(crate) fn fn_19665c_with(&mut self, object: u32, host: &mut impl Live) -> u32 {
        if object == 0 { return 0x89002007; }
        let tree = object+24;
        self.destroy_tree_live(tree, self.word(tree), host);
        self.put_word(tree+24, 0); self.put_word(tree, tree); self.put_word(tree+4, tree);
        self.fn_196428(self.word(object+20), object);
        0
    }

    fn worker_cleanup(&mut self, state: u32, owner: u32, user: u32, task: u32,
        status: u32, host: &mut impl Live) -> u32 {
        if self.word(state+16) != 0 {
            host.call(self, 0x190edc, &[state, 0x684f424a]);
        } else {
            host.call(self, self.word(owner+16), &[user, state]);
            self.put_word(owner+220, self.word(owner+220).wrapping_sub(1));
            if task != 0 { host.call(self, 0x196368, &[task]); }
        }
        let logger = self.word(self.word(owner+176)+16);
        host.log(self, 0x465c, logger, &[]);
        frontend::classify_code(status)
    }

    /// Full fn_19126c bridge-side port. OpenWorkerScripts is deliberately not
    /// used: every call can mutate memory before the next field is read.
    /// scratch reserves 64 mapped bytes, disjoint from all live objects, for
    /// task/stream outputs, setup args, properties and handle references.
    /// It is restored on return. Nested calls must use separate scratch.
    pub(crate) fn fn_19126c(&mut self, a: [u32; 6], scratch: u32,
        data: &WorkerData, host: &mut impl Live) -> u32 {
        if a[1] != 0x68434c53 { return 0x80000008; }
        // Like the ARM, a valid tag requires a readable owner even if zero.
        if self.word(a[0]) != 0x73434c53 { return 0x80000008; }
        let mut saved = [0u8;64];
        saved.copy_from_slice(&self.bytes[scratch as usize..scratch as usize+64]);
        self.bytes[scratch as usize..scratch as usize+64].fill(0);
        let rc = self.open_worker_live(a, scratch, data, host);
        self.bytes[scratch as usize..scratch as usize+64].copy_from_slice(&saved);
        rc
    }

    fn open_worker_live(&mut self, a: [u32;6], q: u32, d: &WorkerData,
        h: &mut impl Live) -> u32 {
        let [owner, _, user, argument, out, flags] = a;
        h.log(self, 0x19126c, self.word(self.word(owner+176)+16), &[]);
        let state = h.call(self, self.word(owner+8), &[user,1,0x1ac]);
        if state == 0 { return 0x8000000a; }
        self.put_word(state+20, user);
        self.put_word(state+24, self.word(owner+156));
        self.put_word(state+28, self.word(owner+160));
        self.put_word(state+32, argument);
        self.put_word(owner+220, self.word(owner+220).wrapping_add(1));
        let mut rc = h.call(self, 0x196318, &[owner+4,user,q]);
        if rc >= 0x80000000 {
            return self.worker_cleanup(state,owner,user,self.word(q),rc,h);
        }
        self.put_word(state+4, owner); self.put_word(state+0x184,flags);
        self.put_word(state+0x188,0); self.put_word(state+0x18c,0);
        self.put_word(state,0x734f424a); self.put_word(state+0x1a8,0);
        self.bytes[(state+52) as usize]=12; self.bytes[(state+53) as usize]=1;
        rc = h.call(self, 0x177b7c, &[d.text[0],self.word(owner+184),q+4]);
        let mut site = 0x4652;
        if rc < 0x80000000 {
            let manager = self.word(owner+176);
            rc = h.call(self,0x176e40,&[self.word(manager+8),self.word(q),state+36]);
            site = 0x4673;
            if rc >= 0x80000000 {
                h.log(self,site,self.word(self.word(owner+176)+16),&[rc]);
                h.call(self,0x177cc8,&[self.word(q+4)]);
                return self.worker_cleanup(state,owner,user,self.word(q),rc,h);
            }
            rc = h.call(self,0x1961d0,&[self.word(q),self.word(self.word(owner+176)+8),
                self.word(q+4),0,state+8]);
            if rc < 0x80000000 {
                rc = h.call(self,0x19611c,&[self.word(state+8),self.word(state+12),state+16]);
            }
            site = 0x4653;
            if rc >= 0x80000000 {
                h.log(self,site,self.word(self.word(owner+176)+16),&[rc]);
                h.call(self,0x177cc8,&[self.word(q+4)]);
                h.call(self,0x176edc,&[self.word(state+36)]);
                return self.worker_cleanup(state,owner,user,self.word(q),rc,h);
            }
        }
        if rc >= 0x80000000 {
            h.log(self,site,self.word(self.word(owner+176)+16),&[rc]);
            return self.worker_cleanup(state,owner,user,self.word(q),rc,h);
        }
        let ctx = self.word(state+16);
        self.put_word(ctx+28,self.word(self.word(owner+176)+28));
        self.bytes[(state+40) as usize]=1;
        if self.word(owner+180)!=0 {
            h.call(self,0x14dc94,&[self.word(owner+180),ctx+16]);
            if self.word(state+24)!=0 || self.word(state+28)!=0 {
                self.put_word(q+8,state+24); self.put_word(q+12,d.text[8]);
                h.call(self,0x14de40,&[self.word(ctx+16),self.word(q+8),self.word(q+12)]);
            }
        }
        rc = h.call(self,0x174ca8,&[0,self.word(state+8),self.word(state+12),ctx+20]);
        site = 0x4654;
        if rc < 0x80000000 {
            rc = h.call(self,0x17ae04,&[0,self.word(state+8),self.word(state+12),ctx+24]);
            site = 0x4655;
        }
        if rc < 0x80000000 {
            rc = h.call(self,0x1758ac,&[self.word(ctx+20),d.text[1],1]);
            if rc < 0x80000000 { rc=h.call(self,0x1758ac,&[self.word(ctx+20),d.text[2],250]); }
            if rc < 0x80000000 { rc=h.call(self,0x1759a0,&[self.word(ctx+20),d.text[3],d.text[4]]); }
            if rc < 0x80000000 { rc=h.call(self,0x1757b8,&[self.word(ctx+20),d.text[5],0]); }
            site=0xc74c;
        }
        if rc < 0x80000000 {
            for (field,name,code) in [(212,d.text[6],0x4656),(216,d.text[7],0x4657)] {
                self.bytes[(q+16) as usize..(q+36) as usize].fill(0);
                self.bytes[(q+16) as usize]=1;
                self.put_word(q+20,self.word(self.word(state+4)+field));
                rc=h.call(self,0x17b0f8,&[self.word(ctx+24),name,q+16]);
                site=code;
                if rc>=0x80000000 { break; }
            }
        }
        if rc < 0x80000000 {
            // SetupArgs at q+40, zero-initialized before any dependency calls.
            self.put_word(q+44,0x18f6b8); self.put_word(q+56,0x18fbb8);
            rc=h.call(self,self.word(self.word(owner+188)+16),&[
                self.word(owner+192),self.word(owner+196),self.word(state+8),
                self.word(state+12),q+40,state,0,state+44]);
            site=0x4658;
        }
        if rc < 0x80000000 {
            rc=h.call(self,0x160b0c,&[self.word(q),state+0x1a8]);
            site=0x465b;
        }
        if rc>=0x80000000 {
            h.log(self,site,self.word(self.word(state+16)+16),&[rc]);
            return self.worker_cleanup(state,owner,user,self.word(q),rc,h);
        }
        self.put_word(out,state); self.put_word(out+4,0x684f424a);
        h.call(self,0xc72000,&[0x191938,out]);
        h.log(self,0x191860,self.word(self.word(state+16)+16),&[]);
        0
    }
}


impl<P: Platform> Flat<'_, '_, '_, '_, P> {
    pub(crate) fn fn_1963e0(&mut self, allocator: u32, pointer: u32, size: u32) -> u32 {
        if allocator==0 || size==0 { return 0; }
        if pointer==0 { return self.fn_196394(allocator,size); }
        let (entry, object)=self.method(allocator,2);
        self.allocator_method(entry,&[object,pointer,size])
    }

    fn concrete_reallocate(&mut self, object: u32, old: u32, size: u32) -> u32 {
        let heap=self.word(object+4);
        if heap<88 { return self.fail(0x4a79b8); }
        self.lock(heap+88);
        let mut saved=[0u8;88];
        saved.copy_from_slice(&self.bytes[..88]);
        self.bytes.copy_within(heap as usize..heap as usize+88,0);
        let result=crate::heap::EngineHeap::from_arena(self.bytes).block_resize(old,size);
        self.bytes.copy_within(0..88,heap as usize);
        self.bytes[..88].copy_from_slice(&saved);
        self.unlock(heap+88);
        if result.unwrap_or(0)!=0 { return old; }
        let entry=guest_entry(self.word(self.word(object)+8));
        let n=self.allocator_method(entry,&[object,size,4]);
        if n!=0 { self.bytes[n as usize..n as usize+size as usize].fill(0); }
        let count=self.word(old-12).min(size);
        // The original memcpy faults if allocation fails and count != 0.
        // Preserve the refusal as an explicit bridge error instead of a host
        // slice panic or copying to a staged address-zero hole.
        if n==0 && count!=0 { return self.fail(0x30253c); }
        for i in 0..count { self.bytes[(n+i) as usize]=self.bytes[(old+i) as usize]; }
        let entry=guest_entry(self.word(self.word(object)+12));
        self.allocator_method(entry,&[object,old]);
        n
    }

    fn half(&self, address: u32) -> u16 {
        u16::from_le_bytes(self.bytes[address as usize..address as usize+2].try_into().unwrap())
    }
    fn put_half(&mut self, address: u32, value: u16) {
        self.bytes[address as usize..address as usize+2].copy_from_slice(&value.to_le_bytes());
    }

    /// fn_15d594 with observable error logging. No output is written on error.
    pub(crate) fn fn_15d594_with(&mut self, a: [u32;5], descriptors: u32,
        host: &mut impl Live) -> u32 {
        if frontend::validate_handle(a[0]!=0,a[1],0xb2c2,0x38)>=0x80000000 {
            return 0x89202008;
        }
        let record=if a[2]<self.word(a[0]+32) {
            self.word(self.word(a[0]+48).wrapping_add(a[2].wrapping_mul(4)))
        } else {0};
        let logger=self.word(self.word(a[0])+16);
        if record==0 {
            host.log(self,0x4e34,logger,&[a[2]]); return 0x89203e11;
        }
        let kind=self.bytes[(record+2) as usize];
        let desc=descriptors.wrapping_add(u32::from(kind)*272);
        if kind==0 || a[3]>=u32::from(self.half(desc+4)) {
            host.log(self,0x4e30,logger,&[u32::from(kind),a[2],a[3]]);
            return 0x89203e0d;
        }
        let address=record.wrapping_add(self.word(desc+8).wrapping_sub(self.word(desc)))
            .wrapping_add(a[3]);
        self.put_half(a[4],u16::from(self.bytes[address as usize]));
        0
    }

    fn release_state_live(&mut self, state: u32, host: &mut impl Live) {
        let refs=self.half(state+22);
        if refs>1 { self.put_half(state+22,refs-1); return; }
        let heap=self.word(self.word(state+4)+4);
        if self.bytes[(state+42) as usize]!=0 {
            // Input-text logging is a separate, memory-mutating subsystem.
            // The host must dispatch the real fn_14e0b4, never replay a result.
            host.call(self,0x14e0b4,&[state,heap]); return;
        }
        if self.word(state+16)!=0 { self.fn_196428(heap,self.word(state+16)); }
        if self.word(state+12)!=0 { host.call(self,0x196c7c,&[self.word(state+12)]); }
        let buffer=self.word(state+60);
        if buffer!=0 {
            if self.word(state+36)!=0 {
                let mut element=buffer.wrapping_add(self.word(state+52).wrapping_sub(self.word(state+56)));
                let end=buffer.wrapping_add(self.word(state+44).wrapping_sub(self.word(state+56)));
                while element<end {
                    host.call(self,self.word(state+36),&[element,self.word(self.word(state+4)+4)]);
                    let stride=u32::from(self.half(state+28));
                    if stride==0 { self.fail(0x14e0b4); return; }
                    self.put_word(state+52,self.word(state+52).wrapping_add(stride));
                    element=element.wrapping_add(stride);
                }
            }
            self.fn_196428(heap,buffer);
        }
        self.fn_196428(heap,state);
    }

    /// Writer close including last-reference destruction and live logging.
    /// Reader removal and logInput destruction are explicit live dependencies.
    pub(crate) fn fn_14f918_with(&mut self, a: [u32;2], host: &mut impl Live) -> u32 {
        if frontend::validate_handle(a[0]!=0,a[1],0x47de,12)>=0x80000000 {
            return 0x81706008;
        }
        let state=self.word(a[0]+4);
        let context=self.word(state+4);
        host.log(self,0x14f918,self.word(context+16),
            &[u32::from(self.half(a[0]+2)),a[0],u32::from(self.bytes[a[0] as usize])]);
        if self.bytes[a[0] as usize]==0 {
            let list=self.word(state+12);
            let base=self.word(list);
            let stride=u32::from(self.half(list+4));
            let count=self.word(list+20);
            let mut index=0;
            while index<count && self.word(base+index*stride)!=a[0] { index+=1; }
            if index<count {
                host.call(self,0x196e28,&[list,index]);
            } else { host.log(self,0x55f3,self.word(context+16),&[]); }
        } else {
            self.put_word(state+8,0); self.bytes[(state+68) as usize]=1;
            host.call(self,0x14e614,&[state,0,0]);
        }
        self.release_state_live(state,host);
        self.fn_196428(self.word(context+4),a[0]);
        host.log(self,0x14f919,self.word(context+16),&[0]);
        0
    }

    /// Full write control flow. Buffer compaction and formatted trace logging
    /// dispatch live fn_14e440/fn_14e614 dependencies with the same reborrow.
    pub(crate) fn fn_1503b0_with(&mut self, a: [u32;4], host: &mut impl Live) -> u32 {
        if frontend::validate_handle(a[0]!=0,a[1],0x47de,12)>=0x80000000 {
            return 0x81706008;
        }
        let context=self.word(self.word(a[0]+4)+4);
        host.log(self,0x1503b0,self.word(context+16),
            &[u32::from(self.half(a[0]+2)),a[0],a[3]]);
        let rc=self.write_live(a,host);
        host.log(self,0x1503b1,self.word(context+16),&[rc]);
        rc
    }
    fn write_live(&mut self, a: [u32;4], h: &mut impl Live) -> u32 {
        let state=self.word(a[0]+4);
        let context=self.word(state+4);
        if a[2]==0 || a[3]==0 {
            h.log(self,0x1503b2,self.word(context+16),&a); return 0x88f02007;
        }
        if ![1,2].contains(&self.bytes[a[0] as usize]) {
            h.log(self,0x55f8,self.word(context+16),&a); return 0x88f02007;
        }
        let written=self.word(state+44);
        let base=self.word(state+56);
        if written<base {
            h.log(self,0x55fc,self.word(context+16),&[a[0],written,base]);
            return 0x88f02005;
        }
        let mut need=written.wrapping_sub(base).wrapping_add(a[3]);
        let mut buffered=true;
        if self.word(state+64)<need {
            let readers=self.word(state+12);
            let count=if readers==0 {0} else {self.word(readers+20)};
            if self.bytes[(state+40) as usize]!=0 || count!=0 {
                h.call(self,0x14e440,&[state,need.wrapping_sub(self.word(state+64))]);
                need=self.word(state+44).wrapping_sub(self.word(state+56)).wrapping_add(a[3]);
                if self.word(state+64)<need {
                    need=need.max(self.word(self.word(state)+16));
                    let heap=self.word(self.word(state+4)+4);
                    let old=self.word(state+60);
                    let n=if old!=0 {self.fn_1963e0(heap,old,need)}
                        else {self.fn_196394(heap,need)};
                    if n==0 {
                        h.log(self,0x55f0,self.word(self.word(state+4)+16),&[]);
                        return 0x88f0200a;
                    }
                    self.put_word(state+60,n); self.put_word(state+64,need);
                }
            } else {buffered=false;}
        }
        if buffered {
            let position=self.word(self.word(a[0]+4)+44).wrapping_sub(self.word(self.word(a[0]+4)+56));
            let dst=self.word(self.word(a[0]+4)+60).wrapping_add(position);
            // Original memcpy requires disjoint source/destination.
            if u64::from(dst)<u64::from(a[2])+u64::from(a[3])
                && u64::from(a[2])<u64::from(dst)+u64::from(a[3]) {
                return self.fail(0x195ae8);
            }
            for i in 0..a[3] {self.bytes[(dst+i) as usize]=self.bytes[(a[2]+i) as usize];}
            if self.word(self.word(a[0]+4)+32)!=0 {
                let mut offset=0;
                while offset<a[3] {
                    let s=self.word(a[0]+4);
                    let ok=h.call(self,self.word(s+32),&[a[2]+offset,
                        self.word(self.word(s+4)+4),self.word(s+60)+position+offset]);
                    if ok==0 {
                        let s=self.word(a[0]+4);
                        if self.word(s+36)!=0 {
                            let mut done=0;
                            while done<offset {
                                let s=self.word(a[0]+4);
                                h.call(self,self.word(s+36),&[self.word(s+60)+position+done,
                                    self.word(self.word(s+4)+4)]);
                                let stride=u32::from(self.half(self.word(a[0]+4)+28));
                                if stride==0 {return self.fail(0x1503b0);}
                                done=done.wrapping_add(stride);
                            }
                        }
                        return 0x88f0200a;
                    }
                    let stride=u32::from(self.half(self.word(a[0]+4)+28));
                    if stride==0 {return self.fail(0x1503b0);}
                    offset=offset.wrapping_add(stride);
                }
            }
        } else {
            let s=self.word(a[0]+4);
            self.put_word(s+56,self.word(s+56).wrapping_add(a[3]));
        }
        let s=self.word(a[0]+4);
        self.put_word(s+44,self.word(s+44).wrapping_add(a[3]));
        h.call(self,0x14e614,&[self.word(a[0]+4),a[2],a[3]]);
        0
    }
}

impl<P: Platform> Flat<'_, '_, '_, '_, P> {
    /// Writer/reader close for shared state and a null or trace-filtered logger.
    /// Reader removal uses the existing list-compaction port, with no list
    /// destructor. Final-reference destruction still uses the live close API.
    pub(crate) fn fn_14f918_shared_quiet(&mut self, a:[u32;2])->u32 {
        if a[0]==0 || a[1]!=0x47de {return 0x81706008;}
        let state=self.word(a[0]+4);
        let context=self.word(state+4);
        let list=self.word(state+12);
        if self.half(state+22)<=1
            || !quiet_stream_logger(self.bytes,self.word(context+16)) || self.bytes[(state+42) as usize]!=0
            || (self.bytes[a[0] as usize]==0 && (list==0 || self.word(list+12)!=0)) {
            return self.fail(0x14f918);
        }
        let heap=self.word(context+4);
        let mut release=OneRelease::default();
        let rc=frontend::close_stream(self.bytes,Some(a[0]),a[1],0,&mut release,
            &mut |_,_| unreachable!(), &mut |_,_| unreachable!());
        if let Some((_,p))=release.0 {self.fn_196428(heap,p);}
        rc
    }

    /// Live fn_175a1c binding. Lock hooks receive the arena currently held by
    /// lookup; no snapshot of a lock result or value is retained.
    pub(crate) fn fn_175a1c(&mut self,a:[u32;3],rom:crate::rom_region::Rom)->u32 {
        struct Lock<'a,P> {object:u32,platform:&'a mut P,rom:crate::rom_region::Rom,error:Option<u32>}
        impl<P:Platform> Lock<'_,P> {
            fn run(&mut self,mem:&mut [u8],slot:u32)->u32 {
                let p=self.object;
                if p==0 {return 0x81402007;}
                let inner=frontend_word(mem,frontend_word(mem,p)+8);
                let entry=frontend_word(mem,inner+slot);
                if entry==0 {return 0;}
                let target=frontend_word(mem,p+8);
                let mut arena=boot::Memory{base:0,bytes:mem};
                // The lock methods are guest addresses: the regional ROMs
                // hold their own twins (EU 0x1841ac/0x1842ec). Normalize
                // to the US entry the ports below dispatch on.
                match crate::rom_region::to_us_guest(self.rom,entry) {
                    0x1840fc=>{crate::boot_part_b::fn_1840fc(&mut arena,target,&mut LockHost(self.platform));0}
                    0x18423c=>{crate::boot_part_b::fn_18423c(&mut arena,target,&mut LockHost(self.platform));0}
                    _=>{self.error=Some(entry);0x80000000}
                }
            }
        }
        impl<P:Platform> crate::reentrant2::StateLockM for Lock<'_,P> {
            fn acquire_m(&mut self,mem:&mut [u8])->u32 {self.run(mem,8)}
            fn release_m(&mut self,mem:&mut [u8])->u32 {self.run(mem,12)}
        }
        // fn_1756d0 leaves the caller's out slot untouched on the null-argument
        // exit, and clears it before the lock is taken.
        if a[0]==0 || a[1]==0 || a[2]==0 {return 0x8b702007;}
        let object=self.word(a[0]+4);
        // The comparator is stored in the guest property tree, so the
        // regional ROMs hold their own twin (EU 0x196a30, KR 0x196a6c).
        // Normalize to the US entry the port below dispatches on.
        let comparator=if object!=0 && self.word(a[0]+8)!=0 {
            crate::rom_region::to_us_guest(rom,self.word(self.word(self.word(a[0]+8)+60)+8))
        } else {0x196980};
        if comparator!=0x196980 {return self.fail(comparator);}
        let mut lock=Lock{object,platform:&mut *self.platform,rom,error:None};
        let mut value=0u32;
        let (rc,error)={
            let rc=crate::reentrant2::lookup_param_m(self.bytes,Some(a[0]),a[1],Some(&mut value),
                None,&mut lock,
                |x,y,mem| crate::speech_part_1::fn_196980(&mut boot::Memory{base:0,bytes:mem},x,y) as i32);
            (rc,lock.error)
        };
        self.bytes[a[2] as usize..a[2] as usize+4].copy_from_slice(&value.to_le_bytes());
        if let Some(entry)=error {return self.fail(entry);}
        rc
    }
}
fn frontend_word(mem:&[u8],p:u32)->u32 {
    u32::from_le_bytes(mem[p as usize..p as usize+4].try_into().unwrap())
}

impl<P:Platform> Flat<'_, '_, '_, '_,P> {
    pub(crate) fn fn_150878(&mut self,a:[u32;4])->u32 {
        if a[0]!=0 && a[1]==0x47de && a[2]!=0 && a[3]!=0 {
            let state=self.word(a[0]+4);
            let context=self.word(state+4);
            if !quiet_stream_logger(self.bytes,self.word(context+16)) {return self.fail(0x1738e8);}
            let written=self.word(state+44);
            let base=self.word(state+56);
            if [1,2].contains(&self.bytes[a[0] as usize]) && written>=base {
                let need=written.wrapping_sub(base).wrapping_add(a[3]);
                if self.word(state+64)<need {
                    if !self.reserve_stream(state,a[3]) {self.put_word(a[2],0);return 0x88f0200a;}
                }
            }
        }
        frontend::write_direct_start(self.bytes,nz(a[0]),a[1],nz(a[2]),a[3],
            &mut NoCallbacks,&mut |_,_| unreachable!())
    }

    /// Uses speech-part-3's certified non-null realloc, with every callback
    /// reentering the live staged arena. The outer Flat alone publishes.
    pub(crate) fn fn_4a79b8_speech(&mut self,a:[u32;3])->u32 {
        struct Host<'a,P> {platform:&'a mut P,error:Option<Error>}
        impl<P:Platform> crate::boot_part_b::Host for Host<'_,P> {
            fn thread_id(&mut self)->u32 {self.platform.thread_id()}
            fn call(&mut self,m:&mut boot::Memory<'_>,e:u32,a:&[u32])->u32 {
                crate::boot_part_b::Host::call(&mut LockHost(self.platform),m,e,a)
            }
        }
        impl<P:Platform> crate::speech_part_3::Host for Host<'_,P> {
            fn call(&mut self,m:&mut boot::Memory<'_>,e:u32,a:&[u32])->u32 {
                // fn_4a79b8 forwards the allocator method address it reads from
                // the guest object; normalize the regional twin the same way.
                arena_allocator(m,guest_entry(e),a,self.platform,&mut self.error)
            }
        }
        let mut h=Host{platform:self.platform,error:None};
        let rc=crate::speech_part_3::fn_4a79b8(&mut boot::Memory{base:0,bytes:self.bytes},a[0],a[1],a[2],&mut h);
        if let Some(error)=h.error {self.error=Some(error);}
        rc
    }
}

fn quiet_stream_logger(mem:&[u8],log:u32)->bool {log==0 || mem[log as usize+48]<4}

struct OneAllocation(u32);
impl frontend::HeapVtable for OneAllocation {
    fn alloc_at(&mut self,_:u32,_:u32)->u32 {self.0}
    fn realloc_at(&mut self,_:u32,_:u32,_:u32)->u32 {unreachable!()}
    fn release_at(&mut self,_:u32,_:u32) {unreachable!()}
}
impl<P:Platform> Flat<'_, '_, '_, '_,P> {
    fn stream_dtor(&mut self,entry:u32,p:u32,heap:u32) {
        if entry==0 {return;}
        // The dtor pointer comes from the guest stream state: normalize the
        // regional twin (EU 0x15178c / KR 0x1516ac) to US 0x1516c8 first.
        let entry=guest_entry(entry);
        if entry!=0x1516c8 {self.fail(entry);return;}
        let mut release=OneRelease::default();
        frontend::release_typed_buffer(self.bytes,nz(p),heap,&mut release);
        if let Some((_,pointer))=release.0 {self.fn_196428(heap,pointer);}
    }
    fn stream_copy(&mut self,entry:u32,src:u32,heap:u32,dst:u32)->u32 {
        // The copy pointer comes from the guest stream state: normalize the
        // regional twin (EU 0x1516d4 / KR 0x1515f4) to US 0x151610 first.
        let entry=guest_entry(entry);
        if entry!=0x151610 {return self.fail(entry);}
        let mut n=0;
        if src!=0 && dst!=0 && [6,12,21,50,36,39,37,34,22,40,26,29].contains(&self.half(src)) {
            let text=self.word(src+20);
            if text!=0 {while self.half(text+n*2)!=0 {n+=1;}}
            n=self.fn_196394(heap,n*2+2);
        }
        frontend::duplicate_typed_string(self.bytes,nz(src),nz(heap),nz(dst),&mut OneAllocation(n))
    }

    /// fn_14e440 with live destructor, release and realloc calls.
    pub(crate) fn fn_14e440(&mut self,state:u32,required:u32) {
        let buffer=self.word(state+60);
        if buffer==0 || self.bytes[(state+40) as usize]!=0 {return;}
        let write=self.word(state+44);
        let readers=self.word(state+12);
        let mut minimum=write;
        if readers!=0 {
            let base=self.word(readers);let stride=u32::from(self.half(readers+4));
            for i in 0..self.word(readers+20) {
                minimum=minimum.min(self.word(self.word(base+i*stride)+8));
            }
        }
        let old=self.word(state+56);
        let stride=u32::from(self.half(state+28));
        let mut discarded=minimum.wrapping_sub(old);
        if stride>1 {discarded=discarded.wrapping_sub(discarded%stride);}
        let dtor=self.word(state+36);
        if dtor!=0 && discarded!=0 {
            let heap=self.word(self.word(state+4)+4);
            let mut element=buffer.wrapping_add(self.word(state+52).wrapping_sub(old));
            let end=buffer.wrapping_add(discarded);
            let mut position=self.word(state+52);
            while element<end {
                self.stream_dtor(dtor,element,heap);
                element=element.wrapping_add(stride);position=position.wrapping_add(stride);
                if stride==0 {self.fail(0x14e440);return;}
            }
            self.put_word(state+52,position);
        }
        let config=self.word(state);
        if required==0 {
            if self.word(config+24)>discarded {return;}
        } else if discarded<required && discarded<self.word(config+20) {return;}
        let remaining=write.wrapping_sub(minimum);
        if remaining!=0 {
            self.bytes.copy_within(buffer.wrapping_add(discarded) as usize..buffer.wrapping_add(discarded).wrapping_add(remaining) as usize,buffer as usize);
        }
        self.put_word(state+56,old.wrapping_add(discarded));
        if required==0 {
            let capacity=self.word(state+64);
            let heap=self.word(self.word(state+4)+4);
            if capacity==discarded || write==minimum {
                self.fn_196428(heap,buffer);self.put_word(state+60,0);self.put_word(state+64,0);
            } else {
                let grown=self.fn_1963e0_speech(heap,buffer,capacity.wrapping_sub(discarded));
                if grown!=0 {self.put_word(state+60,grown);self.put_word(state+64,capacity.wrapping_sub(discarded));}
            }
        }
    }
    fn reserve_stream(&mut self,state:u32,count:u32)->bool {
        let mut need=self.word(state+44).wrapping_sub(self.word(state+56)).wrapping_add(count);
        if self.word(state+64)<need {
            self.fn_14e440(state,need.wrapping_sub(self.word(state+64)));
            need=self.word(state+44).wrapping_sub(self.word(state+56)).wrapping_add(count);
            if self.word(state+64)<need {
                need=need.max(self.word(self.word(state)+16));
                let heap=self.word(self.word(state+4)+4);
                let old=self.word(state+60);
                let new=if old==0 {self.fn_196394(heap,need)} else {self.fn_1963e0_speech(heap,old,need)};
                if new==0 {return false;}
                self.put_word(state+60,new);self.put_word(state+64,need);
            }
        }
        true
    }
    /// fn_1503b0 for trace-filtered streams. Copies and typed-string callbacks
    /// execute before advancing written, as on ARM.
    pub(crate) fn fn_1503b0_quiet(&mut self,a:[u32;4])->u32 {
        if frontend::validate_handle(a[0]!=0,a[1],0x47de,12)>=0x80000000 {return 0x81706008;}
        let state=self.word(a[0]+4);
        let context=self.word(state+4);
        if !quiet_stream_logger(self.bytes,self.word(context+16)) {return self.fail(0x1738e8);}
        if a[2]==0 || a[3]==0 || ![1,2].contains(&self.bytes[a[0] as usize]) {
            if self.word(context+16)!=0 {return self.fail(0x173dc0);}
            return 0x88f02007;
        }
        let written=self.word(state+44);
        let base=self.word(state+56);
        if written<base {return 0x88f02005;}
        let list=self.word(state+12);
        let buffered=self.word(state+64)>=written.wrapping_sub(base).wrapping_add(a[3])
            || self.bytes[(state+40) as usize]!=0 || (list!=0 && self.word(list+20)!=0);
        if buffered && !self.reserve_stream(state,a[3]) {return 0x88f0200a;}
        let buffer=self.word(state+60);
        let heap=self.word(context+4);
        if buffered {
            let position=written.wrapping_sub(self.word(state+56));
            let dst=buffer.wrapping_add(position);
            self.bytes.copy_within(a[2] as usize..a[2].wrapping_add(a[3]) as usize,dst as usize);
            let copy=self.word(state+32);
            if copy!=0 {
                let stride=u32::from(self.half(state+28));
                let mut offset=0;
                while offset<a[3] {
                    if self.stream_copy(copy,a[2]+offset,heap,dst+offset)==0 {
                        let dtor=self.word(state+36);
                        if dtor!=0 {let mut done=0;while done<offset {self.stream_dtor(dtor,dst+done,heap);done+=stride;}}
                        return 0x88f0200a;
                    }
                    offset=offset.wrapping_add(stride);
                    if stride==0 {return self.fail(copy);}
                }
            }
        } else {self.put_word(state+56,self.word(state+56).wrapping_add(a[3]));}
        self.put_word(state+44,self.word(state+44).wrapping_add(a[3]));
        0
    }
    pub(crate) fn fn_15017c(&mut self,a:[u32;3])->u32 {
        if frontend::validate_handle(a[0]!=0,a[1],0x47de,12)>=0x80000000 {return 0x81706008;}
        if self.bytes[a[0] as usize]!=0 {return 0x88f02007;}
        let state=self.word(a[0]+4);
        if !quiet_stream_logger(self.bytes,self.word(self.word(state+4)+16)) {return self.fail(0x1738e8);}
        let position=self.word(a[0]+8);
        let available=self.word(state+44).wrapping_sub(position);
        let next=position.wrapping_add(a[2].min(available));
        self.put_word(a[0]+8,next);
        if next>self.word(state+48) {self.put_word(state+48,next);}
        self.fn_14e440(state,0);
        if a[2]>available {0x88f02007} else {0}
    }
    pub(crate) fn fn_150c0c(&mut self,a:[u32;3])->u32 {
        if frontend::validate_handle(a[0]!=0,a[1],0x47de,12)>=0x80000000 {return 0x81706008;}
        if ![1,2].contains(&self.bytes[a[0] as usize]) {return 0x88f02007;}
        let state=self.word(a[0]+4);
        let context=self.word(state+4);
        if !quiet_stream_logger(self.bytes,self.word(context+16)) {return self.fail(0x1738e8);}
        let written=self.word(state+44);let base=self.word(state+56);
        if written<base {return 0x88f02005;}
        let heap=self.word(context+4);let buffer=self.word(state+60);
        let offset=written.wrapping_sub(base);let need=offset.wrapping_add(a[2]);
        if self.word(state+64)<need {return 0x88f02007;}
        let copy=self.word(state+32);
        if copy!=0 {
            let stride=u32::from(self.half(state+28));let mut i=0;
            while i<a[2] {
                if self.stream_copy(copy,buffer+offset+i,heap,buffer+offset+i)==0 {
                    let dtor=self.word(state+36);
                    if dtor!=0 {let mut j=0;while j<i {self.stream_dtor(dtor,buffer+offset+j,heap);j+=stride;}}
                    return 0x88f0200a;
                }
                i=i.wrapping_add(stride);
                if stride==0 {return self.fail(copy);}
            }
        }
        self.put_word(state+44,written.wrapping_add(a[2]));
        if self.word(self.word(state)+24)<=self.word(state+64).wrapping_sub(need) {
            self.put_word(state+64,need);
            if need==0 {self.fn_196428(heap,buffer);self.put_word(state+60,0);}
            else {let grown=self.fn_1963e0_speech(heap,buffer,need);if grown!=0 {self.put_word(state+60,grown);}}
        }
        self.fn_14e440(state,0);
        0
    }
}


fn arena_allocator<P:Platform>(m:&mut boot::Memory<'_>,e:u32,a:&[u32],platform:&mut P,error:&mut Option<Error>)->u32 {
    match e {
        0x184f20=>m.boot_heap_allocate(a.try_into().expect("heap ABI")),
        0x184f84=>{m.boot_heap_release(a[0],a[1]);0}
        0x163f20|0x39f5e8=>{
            let heap=m.word(a[0]+4);
            crate::boot_part_b::fn_1840fc(m,heap+88,&mut LockHost(platform));
            let out=m.boot_heap_allocate([heap,a[1],a[2],if e==0x39f5e8 {0} else {a[3]},0,0]);
            crate::boot_part_b::fn_18423c(m,heap+88,&mut LockHost(platform));
            out
        }
        0x39f5b8=>{
            crate::speech_part_3::fn_39f5b8(m,a[0],a[1],&mut LockHost(platform));0
        }
        0x19b78c=>{crate::boot_part_b2::fn_19b78c_guest(m,a[0],a[1]);0}
        _=>{*error=Some(Error::UnsupportedStage{entry:e});0}
    }
}
impl<P:Platform> Flat<'_, '_, '_, '_,P> {
    pub(crate) fn fn_1963e0_speech(&mut self,allocator:u32,old:u32,n:u32)->u32 {
        if allocator==0 || n==0 {return 0;}
        if old==0 {return self.fn_196394(allocator,n);}
        let (entry,object)=self.method(allocator,2);
        if entry==0x4a79b8 {self.fn_4a79b8_speech([object,old,n])}
        else {self.allocator_method(entry,&[object,old,n])}
    }
}

impl<P:Platform> Flat<'_, '_, '_, '_,P> {
    /// fn_17a1a4(broker, handle, offset, requested, out, scratch).
    /// The map read uses the resident archive mapping. A logger level the
    /// guest keeps is refused instead of formatted, and an unbound archive
    /// service fails closed at its own address.
    pub(crate) fn fn_17a1a4(&mut self,a:[u32;6])->u32 {
        struct MapHost<'a,P>{platform:&'a mut P,error:Option<u32>}
        impl<P:Platform> crate::boot_part_b::Host for MapHost<'_,P> {
            fn call(&mut self,m:&mut boot::Memory<'_>,f:u32,args:&[u32])->u32 {
                match f {
                    0x1738e8=>{
                        let logger=args[0];
                        let level=args[2];
                        if logger==0 || level>m.bytes[logger as usize+0x30] as u32 {return 0;}
                        self.error=Some(f);0x80000000
                    }
                    0xffff0001=>0,
                    0x1840fc=>{crate::boot_part_b::fn_1840fc(m,args[0],&mut LockHost(self.platform));0}
                    0x18423c=>{crate::boot_part_b::fn_18423c(m,args[0],&mut LockHost(self.platform));0}
                    _=>{self.error=Some(f);0x80000000}
                }
            }
            fn thread_id(&mut self)->u32 {self.platform.thread_id()}
        }
        let scratch=a[5];
        // The archive range is mapped on demand. Without the archive service
        // the map read cannot produce the guest's buffer, so stop at the
        // ROM instruction that performs the mapping. The caller clears the
        // buffer before this point, as the ROM's map read does.
        {
            let handle=a[1];
            let data=self.word(handle+4);
            if data!=0 {
                let rec=self.word(data+4).wrapping_add(self.word(handle+8).wrapping_mul(24));
                if self.word(rec+4)==0 {return self.fail(0x17c9d8);}
            }
        }
        let unmap=crate::boot_part_e::UnmapEnvironment{category:0,begin_format:0,end_format:0,
            error_format:0,labels:[0;3],scratch:scratch.wrapping_add(4)};
        let (rc,error)={
            let mut host=MapHost{platform:&mut *self.platform,error:None};
            let mut arena=boot::Memory{base:0,bytes:&mut *self.bytes};
            let rc=crate::leftovers_1::fn_17a1a4(&mut arena,a[0],a[1],a[2],a[3],a[4],
                scratch,&unmap,&mut host);
            (rc,host.error)
        };
        if let Some(entry)=error {return self.fail(entry);}
        rc
    }
}

impl<P:Platform> Flat<'_, '_, '_, '_,P> {
    pub(crate) fn fn_2409d4(&mut self,a:[u32;4])->u32 {
        if a[3]!=0x21a1fc {return self.fail(a[3]);}
        fn compare(mem:&[u8],a:u32,b:u32)->i32 {
            let mut bytes=[0u8;48];
            bytes[..24].copy_from_slice(&mem[a as usize..a as usize+24]);
            bytes[24..].copy_from_slice(&mem[b as usize..b as usize+24]);
            let mut regions=[crate::engine::Region{base:0,bytes:&mut bytes}];
            let m=Memory::new(&mut regions).expect("local comparator");
            crate::engine_components2::compare_events(&m,0,24).expect("mapped records") as i32
        }
        crate::fe_text::qsort_2409d4(self.bytes,a[0],a[1],a[2],compare);0
    }
}
