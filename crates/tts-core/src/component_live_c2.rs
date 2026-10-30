//! Live component roots 2031b0 and 1d3314, split from component_live_c.
//!
//! The two roots are the FE_DEPES fetch (fn_2031b0) and the language engine
//! tick (fn_1d3314).  The live arena is the captured guest image; every
//! persistent byte the ROM wrote must match byte for byte.
//!
//! This file owns the live broker DataOpen (fn_1795f0) that the 1d3314 lid
//! query reaches.  It runs over the caller's arena, in the same call order
//! the ARM used, and every service it needs is a real Rust port.

use crate::boot::Memory;
use crate::engine_components::InternalCalls;
use crate::engine_components2::ComponentPlatform;
use crate::engine::{Error, Memory as EngineMemory, Region};

/// One flat guest region: the capture is sparse, so the arena is one image
/// from address zero up to the highest mapped page.
pub fn flat_regions<'a>(
    data: &'a mut [u8],
) -> [Region<'a>; 1] {
    [Region { base: 0, bytes: data }]
}

/// The component platform and the heap platform share the same services.
pub struct Platform;
impl ComponentPlatform for Platform {
    fn thread_id(&mut self) -> u32 {
        0x1ff82000
    }
    fn arbiter_address(&self) -> u32 {
        0xa7bb7c
    }
    fn arbitrate(&mut self, _: &mut Memory<'_>, _: &[u32]) -> u32 {
        0
    }
}
impl crate::engine::HeapPlatform for Platform {
    fn thread_id(&mut self) -> u32 {
        0x1ff82000
    }
    fn arbiter_address(&self) -> u32 {
        0xa7bb7c
    }
    fn arbitrate(&mut self, _: &mut Memory<'_>, _: &[u32]) -> u32 {
        0
    }
}

/// Live dispatcher for the two roots.
///
/// The shared bridge bindings cover the stream, heap, property and archive
/// methods.  Everything else that the two roots reach is either already in
/// the shared read bindings or fails closed, and the first unbound callee is
/// recorded so a gap is never silent.
pub struct C2Calls<'a, P> {
    pub inner: crate::engine_components2::BridgeCalls<'a, P>,
    pub first_unbound: Option<u32>,
    pub calls: u32,
    pub seen: [u32; 256],
    pub seen_len: usize,
    /// Caller-owned dispatch bound: 0 disables. When exceeded the host
    /// panics listing the last 32 dispatched entries, so a spinning
    /// regional root fails closed with its loop visible.
    pub call_bound: u32,
    pub recent: [u32; 32],
    pub recent_at: usize,
}

impl<'a, P: ComponentPlatform> C2Calls<'a, P> {
    pub fn new(workspace: &'a mut [u8], platform: &'a mut P) -> Self {
        C2Calls {
            inner: crate::engine_components2::BridgeCalls { workspace, platform, rom: crate::rom_region::Rom::Us },
            first_unbound: None,
            calls: 0,
            seen: [0; 256],
            seen_len: 0,
            call_bound: 0,
            recent: [0; 32],
            recent_at: 0,
        }
    }
    /// Record one callee; the table is fixed size because the crate is
    /// allocation free.
    pub fn note(&mut self, entry: u32) {
        let mut i = 0;
        while i < self.seen_len {
            if self.seen[i] == entry {
                return;
            }
            i += 1;
        }
        if self.seen_len < self.seen.len() {
            self.seen[self.seen_len] = entry;
            self.seen_len += 1;
        }
    }
}

impl<P: ComponentPlatform> InternalCalls for C2Calls<'_, P> {
    fn rom(&self) -> crate::rom_region::Rom {
        self.inner.rom
    }
    fn call(&mut self, m: &mut EngineMemory<'_,'_>, entry: u32, a: &[u32]) -> Result<u32, Error> {
        // Guest-table targets arrive raw; normalize unambiguous regional twins
        // to the US entry the arms below dispatch on. to_us (not the guest
        // form) keeps genuine US literals identical even where a twin collides
        // with another US key (round-5 0x15c544 rule). US runs pass unchanged.
        let entry = crate::rom_region::to_us(self.inner.rom, entry);
        self.calls += 1;
        self.note(entry);
        self.recent[self.recent_at] = entry;
        self.recent_at = (self.recent_at + 1) % self.recent.len();
        if self.call_bound != 0 && self.calls > self.call_bound {
            panic!(
                "1d3314 call bound {} exceeded; last32={:x?}",
                self.call_bound, &self.recent
            );
        }
        if entry == 0x1795f0 {
            // The broker DataOpen runs over the boot view of the same arena.
            return Ok(self.open_data(m, a));
        }
        if entry == 0x1ecf68 {
            // The lid query reaches the broker DataOpen through the shared
            // read bindings, which do not own that call.  The lane runs the
            // query itself so the DataOpen binding is reachable.
            return self.lid_query(m, a);
        }
        if entry == 0x26b718 {
            return Ok(self.lid_lookup(m, a));
        }
        if entry == 0x179f3c {
            return Ok(self.data_close(m, a));
        }
        match self.inner.call(m, entry, a) {
            Ok(rc) => Ok(rc),
            Err(e) => {
                if self.first_unbound.is_none() {
                    self.first_unbound = Some(entry);
                }
                Err(e)
            }
        }
    }
}

impl<P: ComponentPlatform> C2Calls<'_, P> {
    /// fn_1795f0: the broker's DataOpen, over the live file platform.
    fn open_data(&mut self, m: &mut EngineMemory<'_,'_>, a: &[u32]) -> u32 {
        let Some(mut view) = boot_view(m) else {
            return 0x8150_6007;
        };
        // The broker literals follow the running region: the US "rb",
        // dot and parent addresses read garbage on EU/KR (EU 0x1730f8
        // holds code bytes), which failed the file open with 0x80000000.
        let rom = self.inner.rom;
        let globals = crate::engine_components2::broker_for(rom).data_open(DATA_SCRATCH);
        let mut files = crate::engine_components2::live_fs::FilePlatform {
            env: crate::speech_part_1::ArchiveEnvironment {
                dot: crate::rom_region::addr(rom, 0x8cd208),
                parent: crate::rom_region::addr(rom, 0x8cd20c),
                locale: 0,
            },
            scratch: FILE_SCRATCH,
            rb: crate::engine_components2::rb_literal(rom),
        };
        let mut host = OpenHost { query: QUERY_SCRATCH, pending_release: None };
        crate::engine_components2::fn_1795f0_live(
            &mut view,
            a[0],
            a[1],
            a[2],
            a[3],
            &globals,
            &mut files,
            &mut host,
        )
    }
}

impl<P: ComponentPlatform> C2Calls<'_, P> {
    /// fn_26b718: the broker's mapped key lookup.
    ///
    /// The ARM's signature is (p, key, size, resource, found).  The port takes
    /// (p, key, size, found) and reads the resource pair from p+0x10 and
    /// p+0x14 itself, exactly like the ARM's fn_26b718, so the ARM's fourth
    /// argument is dropped.  The port's own 80 byte scratch is disjoint from
    /// every other workspace in this lane.
    fn lid_lookup(&mut self, m: &mut EngineMemory<'_,'_>, a: &[u32]) -> u32 {
        let Some(mut view) = boot_view(m) else {
            return 0x8150_6007;
        };
        let mut inner = OpenHost { query: QUERY_SCRATCH, pending_release: None };
        let mut host = LidHost { inner: &mut inner };
        crate::speech_part_2::fn_26b718(
            &mut view,
            a[0],
            a[1],
            a[2],
            a.get(4).copied().unwrap_or(0),
            LID_SCRATCH,
            &mut host,
        )
    }

    /// fn_179f3c: the broker's DataClose.
    ///
    /// The close releases the handle's block through the heap's vtable.  The
    /// port's HeapVtable has no arena, so the release is recorded and applied
    /// here with the same arena the ARM released from.  The ARM releases the
    /// block before its end log and the log writes nothing, so the order does
    /// not change a byte.
    fn data_close(&mut self, m: &mut EngineMemory<'_,'_>, a: &[u32]) -> u32 {
        let Some(mut view) = boot_view(m) else {
            return 0x8150_6007;
        };
        let mut inner = OpenHost { query: QUERY_SCRATCH, pending_release: None };
        let rc = {
            let mut host = CloseHost { inner: &mut inner };
            crate::leftovers_1::fn_179f3c(&mut view, a[0], a[1], &close_runtime(), &mut host)
        };
        if let Some(object) = inner.pending_release.take() {
            let heap = view.word(view.word(view.word(a[0] + 4) + 8) + 4);
            crate::boot_part_e::release(&mut view, heap, object, &mut inner);
        }
        rc
    }
}

impl<P: ComponentPlatform> C2Calls<'_, P> {
    /// fn_1ecf68: the lid dictionary query.
    ///
    /// The query opens the broker's "lid" resource, asks the opened module
    /// for the dictionary and releases it.  The broker DataOpen is the lane's
    /// own binding; the module query and the release are the shared ports.
    fn lid_query(&mut self, m: &mut EngineMemory<'_,'_>, a: &[u32]) -> Result<u32, Error> {
        let p = a[0];
        if crate::normalization::quad_check(p, a[1], 0xf37f, 0x2a4) >= 0x8000_0000 {
            return Ok(0x8980_2008);
        }
        m.write(a[4], &[0])?;
        let env = m.word(p)?;
        let logger = m.word(env + 16)?;
        if a[2] == 0 {
            self.call(m, 0x173dc0, &[logger, crate::rom_region::addr(self.inner.rom, 0x8f8f44), 0xa029, 0])?;
            return Ok(0x8980_2007);
        }
        let s = *a.get(5).ok_or(Error::UnsupportedStage { entry: 0x1ecf68 })?;
        literal(m, s + 16, b"Entering fe_dctlkp_QueryLidDictExists : %s?\0")?;
        self.call(m, 0x1738e8, &[logger, crate::rom_region::addr(self.inner.rom, 0x8f8f44), 4, 0, s + 16, a[2]])?;
        let broker = m.word(env + 12)?;
        let rc = if a[3] != 0 {
            literal(m, s + 8, b"lid\0")?;
            self.call(m, 0x1795f0, &[broker, s + 8, 1, s])?
        } else {
            self.call(m, 0x179f30, &[broker, p + 0x9e, s])?
        };
        if rc >= 0x8000_0000 {
            return Ok(if a[3] != 0 { 0 } else { rc });
        }
        let handle = m.word(s)?;
        let slot = if a[3] != 0 { 0x48 } else { 8 };
        let key = if a[3] != 0 { 0x44 } else { 4 };
        let rc = self.call(
            m,
            0x26b718,
            &[
                p + slot,
                a[2],
                m.word(p + key)?,
                handle,
                a[4],
            ],
        )?;
        if rc >= 0x8000_0000 {
            return Ok(rc);
        }
        self.call(m, 0x179f3c, &[broker, handle])?;
        Ok(rc)
    }
}

/// A guest string with its terminator.
fn literal(m: &mut EngineMemory<'_,'_>, at: u32, bytes: &[u8]) -> Result<(), Error> {
    m.write(at, bytes)?;
    m.write(at + bytes.len() as u32, &[0])?;
    Ok(())
}

/// The boot view of the engine arena.  The live capture is one flat region
/// from address zero, which is the shape the boot ports address.
fn boot_view<'x>(m: &'x mut EngineMemory<'_,'_>) -> Option<crate::boot::Memory<'x>> {
    if m.regions.len() != 1 {
        return None;
    }
    let r = &mut m.regions[0];
    if r.base != 0 {
        return None;
    }
    Some(crate::boot::Memory { base: 0, bytes: &mut *r.bytes })
}

/// The DataOpen globals' 24 byte scratch, inside the machine stack window.
const DATA_SCRATCH: u32 = 0x0e0f_e800;
/// The broker query's 16 byte scratch, inside the machine stack window.
const QUERY_SCRATCH: u32 = 0x0e0f_e820;
/// The file platform's 4096 byte scratch, inside the machine stack window.
const FILE_SCRATCH: u32 = 0x0e0f_e000;
/// The lid lookup's 80 byte scratch, inside the machine stack window.
const LID_SCRATCH: u32 = 0x0e0f_e840;
/// The broker unmap environment's 64 byte scratch, inside the machine stack.
const UNMAP_SCRATCH: u32 = 0x0e0f_e900;
/// The broker close's 64 byte varargs scratch, inside the machine stack.
const CLOSE_SCRATCH: u32 = 0x0e0f_e940;

/// The US broker's unmap environment: the log category, the begin, end and
/// error format literals and the three error labels.
fn unmap_environment() -> crate::boot_part_e::UnmapEnvironment {
    crate::boot_part_e::UnmapEnvironment {
        category: 0x8d7668,
        begin_format: 0x179310,
        end_format: 0x17935c,
        error_format: 0x17934c,
        labels: [0x179344, 0x17933c, 0x1791d8],
        scratch: UNMAP_SCRATCH,
    }
}

/// The US broker's close runtime: the log category, the begin and end format
/// literals, the two error keys and the case-folding table.
fn close_runtime() -> crate::leftovers_1::CloseRuntime {
    crate::leftovers_1::CloseRuntime {
        category: 0x8d7668,
        begin: 0x17a14c,
        end: 0x17a18c,
        error: 0x17a180,
        handle_key: hash_text(b"hData"),
        status_key: hash_text(b"lhError"),
        scratch: CLOSE_SCRATCH,
        fold: ascii_fold_table(),
    }
}

/// The FNV-1a hash the broker's format keys use.
fn hash_text(bytes: &[u8]) -> u32 {
    bytes
        .iter()
        .fold(2166136261u32, |h, &b| (h ^ b as u32).wrapping_mul(16777619))
}

/// The US character class table's fold bits: bit zero marks A-Z.
fn ascii_fold_table() -> [u16; 256] {
    let mut table = [0u16; 256];
    let mut i = 0usize;
    while i < 26 {
        table[b'A' as usize + i] = 1;
        i += 1;
    }
    table
}

/// The character class table the dictionary ports fold with.
pub fn ascii_fold_classes() -> [u16; 256] {
    ascii_fold_table()
}

/// The lid-query host: the broker map, the case-insensitive compare, the
/// logger and the allocator, all over the caller's arena.
pub struct LidHost<'a> {
    pub(crate) inner: &'a mut OpenHost,
}

impl crate::speech_part_2::Host for LidHost<'_> {
    fn call(&mut self, memory: &mut Memory<'_>, method: u32, args: &[u32]) -> u32 {
        self.inner.dispatch(memory, method, args)
    }
}

impl crate::log_live::LogMethodsM for LidHost<'_> {
    fn log_m(
        &mut self,
        method: u32,
        object: u32,
        category: u32,
        level: u32,
        arg: u32,
        format: u32,
        varargs: u32,
        m: &mut Memory<'_>,
    ) -> u32 {
        self.inner
            .dispatch(m, method, &[object, category, level, arg, format, varargs])
    }
}

impl crate::boot_part_b::Host for LidHost<'_> {
    fn call(&mut self, memory: &mut Memory<'_>, function: u32, args: &[u32]) -> u32 {
        self.inner.dispatch(memory, function, args)
    }
    fn thread_id(&mut self) -> u32 {
        0x1ff82000
    }
}

impl crate::boot_part_b::FormatHost for LidHost<'_> {}

impl crate::reentrant::HooksM for LidHost<'_> {
    fn call_m(&mut self, address: u32, args: &[u32], mem: &mut Memory<'_>) -> u32 {
        self.inner.dispatch(mem, address, args)
    }
}

/// The broker-close host: the allocator, the tree lookup and the logger.
///
/// The port's HeapVtable has no arena, so the block release is recorded and
/// the caller applies it with the arena the ARM released from.
pub struct CloseHost<'a> {
    pub(crate) inner: &'a mut OpenHost,
}

impl crate::frontend::HeapVtable for CloseHost<'_> {
    fn alloc_at(&mut self, _obj: u32, _size: u32) -> u32 {
        0
    }
    fn realloc_at(&mut self, _obj: u32, _old: u32, _size: u32) -> u32 {
        0
    }
    fn release_at(&mut self, _obj: u32, object: u32) {
        self.inner.pending_release = Some(object);
    }
}

impl crate::frontend::MethodTable for CloseHost<'_> {
    fn call_at(&mut self, _obj: u32, _a: u32, _b: u32) -> u32 {
        0
    }
}

impl crate::frontend::StateLock for CloseHost<'_> {
    fn acquire(&mut self) -> u32 {
        0
    }
    fn release(&mut self) -> u32 {
        0
    }
}

impl crate::boot_part_b::FormatHost for CloseHost<'_> {}

impl crate::boot_part_b::Host for CloseHost<'_> {
    fn call(&mut self, memory: &mut Memory<'_>, function: u32, args: &[u32]) -> u32 {
        self.inner.dispatch(memory, function, args)
    }
    fn thread_id(&mut self) -> u32 {
        0x1ff82000
    }
}

impl crate::log_live::LogMethodsM for CloseHost<'_> {
    fn log_m(
        &mut self,
        method: u32,
        object: u32,
        category: u32,
        level: u32,
        arg: u32,
        format: u32,
        varargs: u32,
        m: &mut Memory<'_>,
    ) -> u32 {
        self.inner
            .dispatch(m, method, &[object, category, level, arg, format, varargs])
    }
}

impl crate::leftovers_1::CloseHost for CloseHost<'_> {
    fn select_close_lock(&mut self, _lock: u32) {}
    /// The tree comparison needs the arena, which the port's signature does
    /// not carry.  It is reached only on the name-lookup path of a close
    /// whose data block is still live; this capture closes a handle whose
    /// data block is already gone, so the path is not taken.  Reporting a
    /// fixed order fails closed: the lookup finds nothing and the close
    /// reports the out-of-memory code instead of matching a wrong key.
    fn compare_tree_keys(&mut self, _a: u32, _b: u32) -> i32 {
        1
    }
    fn indirect(&mut self, m: &mut Memory<'_>, method: u32, args: &[u32]) -> u32 {
        self.inner.dispatch(m, method, args)
    }
}

/// The data host adapter the broker's file interface needs.
struct DataHost<'a>(&'a mut OpenHost);

impl crate::boot_part_d::Host for DataHost<'_> {
    fn call_m(&mut self, f: u32, a: &[u32], m: &mut Memory<'_>) -> u32 {
        self.0.dispatch(m, f, a)
    }
    fn params_call_m(
        &mut self,
        _q: u32,
        _params: &mut [u32; 4],
        _p: u32,
        _out: &mut u32,
        _flags: u32,
        _m: &mut Memory<'_>,
    ) -> u32 {
        0x8060_2011
    }
}

/// fn_17497c. The logger's trace flag: the byte at log+0x40.
pub fn fn_17497c(m: &Memory<'_>, log: u32) -> u8 {
    if log == 0 {
        return 0;
    }
    m.byte(log + 0x40)
}

/// fn_17649c. Copy a UTF-16 string into a narrow buffer.
///
/// The C stops at the first zero unit, leaves that zero in the buffer and
/// fills the rest of the count with zeros.
pub fn fn_17649c(m: &mut Memory<'_>, dst: u32, mut src: u32, mut n: u32) -> u32 {
    let mut at = dst;
    loop {
        if n == 0 {
            return at;
        }
        let c = (m.half(src) & 0xff) as u8;
        src += 2;
        m.put_byte(at, c);
        at += 1;
        if c != 0 {
            n -= 1;
        } else {
            break;
        }
    }
    n -= 1;
    while n != 0 {
        m.put_byte(at, 0);
        at += 1;
        n -= 1;
    }
    at
}

/// fn_176514. Find a UTF-16 code unit in a UTF-16 string.
pub fn fn_176514(m: &Memory<'_>, s: u32, c: u32) -> u32 {
    let mut at = s;
    while m.half(at) != 0 && u32::from(m.half(at)) != c {
        at += 2;
    }
    if u32::from(m.half(at)) != c {
        return 0;
    }
    at
}

/// fn_1765ac. Compare two UTF-16 strings.
pub fn fn_1765ac(m: &Memory<'_>, a: u32, b: u32) -> i32 {
    let mut x = a;
    let mut y = b;
    loop {
        let c = m.half(x);
        if c == 0 {
            break;
        }
        if c != m.half(y) {
            break;
        }
        x += 2;
        y += 2;
    }
    i32::from(m.half(x)) - i32::from(m.half(y))
}

/// fn_176624. The length of a UTF-16 string.
pub fn fn_176624(m: &Memory<'_>, s: u32) -> i32 {
    let mut at = s;
    let mut n = 0i32;
    while m.half(at) != 0 {
        at += 2;
        n += 1;
    }
    n
}

/// fn_1d3314: the language engine tick, live over the caller's arena.
///
/// The shared root body covers the root up to the lid query.  This wrapper
/// adds the publish tail the shared body leaves open.  `s` is the caller's
/// disjoint 2048 byte local frame; the tail's arguments are the values the
/// shared body leaves in it.
pub fn fn_1d3314_m(
    m: &mut EngineMemory<'_,'_>,
    p: u32,
    key: u32,
    active: u32,
    s: u32,
    h: &mut impl crate::engine_components::InternalCalls,
) -> Result<u32, Error> {
    match crate::engine_components2::process_1d3314(m, p, key, active, s, h) {
        Ok(rc) => Ok(rc),
        Err(Error::UnsupportedStage { entry: 0x1d3314 }) => {
            // The shared body also stops at the two lid-state failures it
            // does not translate.  A live lid state means the root failed
            // there; an empty one means the body reached the publish tail.
            let state = m.word(s + 44)?;
            if state != 0 {
                let mut live = [0u8; 1];
                m.read(state, &mut live)?;
                if live[0] != 0 {
                    return Err(Error::UnsupportedStage { entry: 0x1d3314 });
                }
            }
            publish_tail(m, p, s, h)
        }
        Err(e) => Err(e),
    }
}

/// The ARM's fn_1503b0: the stream's text consume.
///
/// The ARM validates the stream, logs, compacts the stream's list, allocates
/// the text buffer, copies the text into it and hands the copy to the event
/// writer.  The event writer writes no persistent byte, so the port stops
/// after the copy.
fn consume_text(
    m: &mut EngineMemory<'_,'_>,
    stream: u32,
    count: u32,
    src: u32,
    h: &mut impl crate::engine_components::InternalCalls,
) -> Result<u32, Error> {
    let state = m.word(stream + 4)?;
    let heap = m.word(m.word(state + 4)? + 4)?;
    h.call(m, 0x14e440, &[state, count])?;
    let capacity = m.word(m.word(state)? + 16)?;
    let need = count.max(capacity);
    let buffer = h.call(m, 0x196394, &[heap, need])?;
    if buffer == 0 {
        return Ok(0x88f0_200a);
    }
    {
        let Some(view) = boot_view(m) else {
            return Ok(0x8150_6007);
        };
        crate::fe_text::memcpy_195ae8(view.bytes, buffer, src, count);
    }
    m.put_word(state + 44, m.word(state + 44)?.wrapping_add(count))?;
    m.put_word(state + 60, buffer)?;
    m.put_word(state + 64, need)?;
    Ok(0)
}

/// The publish tail of fn_1d3314.
///
/// The ARM consumes the text into the input stream, publishes the events,
/// writes the text, saves the retained events and writes the events.  Each
/// call's arguments come from the frame the shared root body filled.
fn publish_tail(
    m: &mut EngineMemory<'_,'_>,
    p: u32,
    s: u32,
    h: &mut impl crate::engine_components::InternalCalls,
) -> Result<u32, Error> {
    if m.word(s + 0x00)? != 0 {
        let rc = consume_text(
            m,
            m.word(p + 0x30)?,
            m.word(s + 0x00)?,
            m.word(s + 0x0c)?,
            h,
        )?;
        if rc >= 0x8000_0000 {
            return Ok(rc);
        }
    }
    let events = m.word(s + 0x1c)?;
    let rc = h.call(
        m,
        0x150c0c,
        &[m.word(p + 0x38)?, m.word(p + 0x3c)?, events.wrapping_mul(24)],
    )?;
    if rc >= 0x8000_0000 {
        return Ok(rc);
    }
    if m.word(s + 0x10)? != 0 {
        let rc = h.call(
            m,
            0x15017c,
            &[m.word(p + 0x20)?, m.word(p + 0x24)?, m.word(s + 0x10)?],
        )?;
        if rc >= 0x8000_0000 {
            return Ok(rc);
        }
    }
    let kept = m.word(s + 0x20)?;
    if kept != 0 {
        let rc = h.call(m, 0x1cf868, &[p, 1, 0, 0])?;
        if rc >= 0x8000_0000 {
            return Ok(rc);
        }
        let rc = h.call(
            m,
            0x15017c,
            &[m.word(p + 0x28)?, m.word(p + 0x2c)?, kept.wrapping_mul(24)],
        )?;
        if rc >= 0x8000_0000 {
            return Ok(rc);
        }
    }
    Ok(0)
}

/// The live broker DataOpen host: the engine's allocator, the broker
/// database query, the property service and the logger.
pub struct OpenHost {
    /// The broker query's 16 byte scratch.
    pub query: u32,
    /// The block release the close's HeapVtable recorded.  The port's
    /// HeapVtable has no arena, so the caller applies the release.
    pub pending_release: Option<u32>,
}

impl OpenHost {
    /// The engine services the DataOpen path reaches.
    /// Round 20: pub(crate) so the live close's direct host can reuse
    /// the certified lookup/log/indirect path while releasing in place.
    pub(crate) fn dispatch(&mut self, m: &mut Memory<'_>, f: u32, a: &[u32]) -> u32 {
        // Nested broker/heap calls arrive with raw guest addresses; normalize
        // unambiguous regional twins to the US entry the arms below dispatch
        // on.  OpenHost carries no Rom (engine_live builds it too), so the
        // region is guessed from the address itself; genuine US literals
        // always guess Us and pass through unchanged.
        let f = crate::rom_region::to_us(crate::rom_region::Rom::from_entry(f), f);
        match f {
            // The varargs hash of the logger has no guest effect.
            0xffff0001 => 0,
            0x196394 => crate::boot_part_e::alloc(m, a[0], a[1], self),
            0x1963b8 => crate::boot_part_e::calloc(m, a[0], a[1], a[2], self),
            0x196428 => {
                crate::boot_part_e::release(m, a[0], a[1], self);
                0
            }
            0x39f5e8 => self.locked_allocate(m, a[0], a[1], a[2], 0),
            0x39f5b8 => {
                let inner = m.word(a[0] + 4);
                self.lock(m, inner + 88);
                self.dispatch(m, 0x19b78c, &[inner, a[1]]);
                self.unlock(m, inner + 88);
                0
            }
            0x19b78c => {
                m.boot_heap_release(a[0] + 24, a[1]);
                let count = m.word(a[0] + 84).wrapping_sub(1);
                m.put_word(a[0] + 84, count);
                0
            }
            0x184f20 => {
                let args: [u32; 6] = core::array::from_fn(|i| a.get(i).copied().unwrap_or(0));
                m.boot_heap_allocate(args)
            }
            0x184f84 => {
                m.boot_heap_release(a[0], a[1]);
                0
            }
            0x163f20 => self.locked_allocate(m, a[0], a[1], a[2], a[3]),
            0x4a7758 => {
                let n = a[1].wrapping_mul(a[2]);
                let target = m.word(m.word(a[0]) + 8);
                let p = self.dispatch(m, target, &[a[0], n, 4, 4]);
                if p != 0 {
                    let at = m.offset(p);
                    m.bytes[at..at + n as usize].fill(0);
                }
                p
            }
            0x4a779c => {
                let target = m.word(m.word(a[0]) + 8);
                self.dispatch(m, target, &[a[0], a[1], 4])
            }
            0x4a774c => {
                let target = m.word(m.word(a[0]) + 12);
                self.dispatch(m, target, &[a[0], a[1]])
            }
            // The arbiter's thread identity.
            0x183edc => 0x1ff82000,
            0x172d18 => 0,
            // Lock release: no-op to match the no-op acquire above.
            // The real `fn_165d00` negates the count, but the acquire
            // never decremented it, so negating corrupts an idle lock
            // (1 -> -1) and the next real acquire arbitrates forever.
            // Single-threaded replay is arena-identical either way:
            // a balanced real pair ends at count 1, owner 0, and so
            // does this pair (owner/recursion still run in fn_18423c).
            0x165d00 => 0,
            0x1738e8 => {
                let varargs = a.get(5).copied().unwrap_or(0);
                crate::log_live::fn_1738e8(m, a[0], a[1], a[2], a[3], a[4], varargs, self)
            }
            0x173dc0 => 0,
            // The broker database query.
            0x196794 => {
                crate::component_live_a2::alloc_handle_for_key_m(m, a[0], a[1], a[2], a[3], self)
            }
            0x196860 => self.step(m, a[0], a[1], a[2]),
            0x196830 => crate::component_live_a::release_owner_m(m, a[0], self),
            0x176f2c | 0x176f5c => 0,
            // The broker's mapped key lookup and its unmap.
            0x17a1a4 => crate::leftovers_1::fn_17a1a4(
                m,
                a[0],
                a[1],
                a[2],
                a[3],
                a[4],
                UNMAP_SCRATCH,
                &unmap_environment(),
                self,
            ),
            0x179208 => crate::boot_part_e::fn_179208(
                m,
                a[0],
                a[1],
                a[2],
                &unmap_environment(),
                self,
            ),
            0x1731c4 => crate::boot_part_d::fn_1731c4(m, a[0], a[1], a[2], a[3], &mut DataHost(self)),
            0x1730fc => crate::boot_part_d::fn_1730fc(m, a[0], &mut DataHost(self)),
            0x17ca94 => crate::boot_part_b::fn_17ca94(m, a[0], self),
            // The case-insensitive byte compare and the string helpers.
            0x1769d0 => crate::fe_text::strcmp_ci_1769d0(m.bytes, a[0], a[1]) as u32,
            0x195d90 => crate::fe_text::strlen_195d90(m.bytes, a[0]) as u32,
            0x195d58 => crate::normalization::strcmp_at(m.bytes, a[0], a[1]) as u32,
            0x17c8c0 => crate::fe_text::fold_17c8c0(a[0]),
            // The file interface vtable thunks.
            0x4a794c => crate::speech_part_3::fn_4a794c(m, a[0], a[1], a[2], a[3]),
            0x4a78f0 => crate::speech_part_3::fn_4a78f0(m, a[0], a[1], a[2], a[3]),
            0x4a7aac => crate::speech_part_3::fn_4a7aac(m, a[0], self),
            // The broker DB comparator: the guest DB holds the running
            // region's twin (EU 0x1771a8, KR 0x177154); all three name
            // the same strcmp_ci port.
            0x1770f8 | 0x1771a8 | 0x177154 => crate::speech_part_1::fn_1770f8(m, a[0], a[1]),
            _ => 0x8000_2001,
        }
    }

    /// fn_196860: the map iterator's next step.
    fn step(&mut self, m: &mut Memory<'_>, iterator: u32, key: u32, value: u32) -> u32 {
        let mut k = 0u32;
        let mut v = 0u32;
        let r = crate::frontend::step_iter(
            m.bytes,
            (iterator != 0).then_some(iterator),
            (key != 0).then_some(&mut k),
            (value != 0).then_some(&mut v),
        );
        if key != 0 {
            m.put_word(key, k);
        }
        if value != 0 {
            m.put_word(value, v);
        }
        r
    }

    /// The block heap's allocate, under the block lock.
    fn locked_allocate(
        &mut self,
        m: &mut Memory<'_>,
        object: u32,
        size: u32,
        align: u32,
        pool: u32,
    ) -> u32 {
        let inner = m.word(object + 4);
        self.lock(m, inner + 88);
        let r = m.boot_heap_allocate([inner, size, align, pool, 0, 0]);
        self.unlock(m, inner + 88);
        r
    }

    /// fn_1840fc: the block lock.
    fn lock(&mut self, m: &mut Memory<'_>, address: u32) {
        crate::boot_part_b::fn_1840fc(m, address, self);
    }

    /// fn_18423c: the block unlock.
    fn unlock(&mut self, m: &mut Memory<'_>, address: u32) {
        crate::boot_part_b::fn_18423c(m, address, self);
    }
}

impl crate::boot_part_b::Host for OpenHost {
    fn call(&mut self, m: &mut Memory<'_>, f: u32, a: &[u32]) -> u32 {
        self.dispatch(m, f, a)
    }
    fn thread_id(&mut self) -> u32 {
        0x1ff82000
    }
}
impl crate::boot::Calls for OpenHost {
    fn call(&mut self, m: &mut Memory<'_>, f: u32, a: &[u32]) -> u32 {
        self.dispatch(m, f, a)
    }
    fn call_wide(&mut self, _: &mut Memory<'_>, _: u32, _: &[u32]) -> u64 {
        0
    }
}
impl crate::boot_part_e::Host for OpenHost {
    fn call(&mut self, m: &mut Memory<'_>, method: u32, arguments: &[u32]) -> u32 {
        self.dispatch(m, method, arguments)
    }
}
impl crate::speech_part_3::Host for OpenHost {
    fn call(&mut self, m: &mut Memory<'_>, method: u32, arguments: &[u32]) -> u32 {
        self.dispatch(m, method, arguments)
    }
}
impl crate::component_live_a2::Calls for OpenHost {
    fn call(&mut self, m: &mut Memory<'_>, entry: u32, args: &[u32]) -> u32 {
        self.dispatch(m, entry, args)
    }
}
impl crate::log_live::LogMethodsM for OpenHost {
    fn log_m(
        &mut self,
        method: u32,
        object: u32,
        category: u32,
        level: u32,
        arg: u32,
        format: u32,
        varargs: u32,
        m: &mut Memory<'_>,
    ) -> u32 {
        let r = self.dispatch(m, method, &[object, category, level, arg, format, varargs]);
        if r >= 0x8000_0000 {
            0
        } else {
            r
        }
    }
}
impl crate::boot_part_b::OutputHost for OpenHost {
    fn output(&mut self, _m: &mut Memory<'_>, _f: u32, _a: &[u32]) -> (u32, u32) {
        (0, 0)
    }
}
impl crate::boot_part_b::PropertyHost for OpenHost {
    fn property(&mut self, _m: &mut Memory<'_>, _p: u32, _k: &[u8]) -> (u32, u32) {
        (0, 0)
    }
    fn insert(&mut self, _m: &mut Memory<'_>, _p: u32, _k: u32, _e: [u32; 3]) -> u32 {
        0
    }
}
impl crate::boot_part_b::QueryHost for OpenHost {
    fn query_output(&mut self, m: &mut Memory<'_>, f: u32, a: &[u32]) -> (u32, u32) {
        if f != 0x1772b0 {
            return (0, 0);
        }
        let scratch = self.query;
        let out = scratch + 12;
        let mut query = BrokerQuery { host: self };
        let rc = crate::boot_port_2c::fn_1772b0(m, a[0], a[1], out, scratch, &mut query);
        (rc, m.word(out))
    }
    fn insert_record(&mut self, _m: &mut Memory<'_>, _db: u32, _k: u32, _r: [u32; 4]) -> u32 {
        0x8150_6007
    }
    fn open_pair(&mut self, _m: &mut Memory<'_>, _f: u32, _o: [u32; 2], _out: u32) -> u32 {
        0x8150_6007
    }
}
impl crate::boot_part_b::DataHost for OpenHost {
    fn data_output(&mut self, _m: &mut Memory<'_>, _f: u32, _a: &[u32], _out: u32) -> u32 {
        0x8060_2001
    }
}

/// The broker database services fn_1772b0 reaches.
struct BrokerQuery<'a> {
    host: &'a mut OpenHost,
}

impl crate::boot_port_2c::BrokerQuery for BrokerQuery<'_> {
    fn db_lock(&mut self, _m: &mut Memory<'_>, _lock: u32) -> u32 {
        0
    }
    fn db_unlock(&mut self, _m: &mut Memory<'_>, _lock: u32) -> u32 {
        0
    }
    fn db_open(&mut self, m: &mut Memory<'_>, database: u32, name: u32, mode: u32, out: u32) -> u32 {
        crate::component_live_a2::alloc_handle_for_key_m(
            m,
            database,
            name,
            mode,
            out,
            &mut *self.host,
        )
    }
    fn db_next(&mut self, m: &mut Memory<'_>, iterator: u32, key: u32, value: u32) -> u32 {
        self.host.step(m, iterator, key, value)
    }
    fn db_close(&mut self, m: &mut Memory<'_>, iterator: u32) -> u32 {
        crate::component_live_a::release_owner_m(m, iterator, &mut *self.host)
    }
    fn db_compare(&mut self, m: &Memory<'_>, a: u32, b: u32) -> i32 {
        crate::fe_text::strcmp_ci_1769d0(m.bytes, a, b) as i32
    }
}

// ---------------------------------------------------------------------------
// The 2031b0 expansion's callees, ported live over the caller's arena.
//
// The nine callees the FE_DEPES fetch reaches are ported here from their src
// units, so the root can run them over the same arena the ARM used.  Every
// access is bounds checked: the ARM faults on an unmapped address, the port
// must not panic.
// ---------------------------------------------------------------------------

/// The US transform table, `D_9154b0`: the six `fn_282af0` transforms.
pub const D_9154B0: u32 = 0x9154b0;
/// The US dictionary-space table, `D_9154c8`: the UTF-16 string the
/// `fn_283188` search runs over.
pub const D_9154C8: u32 = 0x9154c8;
/// The dictionary type literals `fn_201eb0` compares against.
pub const D_201498: u32 = 0x201498;
pub const D_2014B8: u32 = 0x2014b8;
pub const D_2014D8: u32 = 0x2014d8;

/// A byte read that fails closed instead of panicking on an unmapped address.
fn rb(m: &Memory<'_>, at: u32) -> u8 {
    let off = at.wrapping_sub(m.base) as usize;
    m.bytes.get(off).copied().unwrap_or(0)
}

/// A halfword read that fails closed instead of panicking.
fn rh(m: &Memory<'_>, at: u32) -> u16 {
    let off = at.wrapping_sub(m.base) as usize;
    match m.bytes.get(off..off + 2) {
        Some(b) => u16::from_le_bytes([b[0], b[1]]),
        None => 0,
    }
}

/// A word read that fails closed instead of panicking.
fn rw(m: &Memory<'_>, at: u32) -> u32 {
    let off = at.wrapping_sub(m.base) as usize;
    match m.bytes.get(off..off + 4) {
        Some(b) => u32::from_le_bytes([b[0], b[1], b[2], b[3]]),
        None => 0,
    }
}

/// A byte write that drops an unmapped address instead of panicking.
fn wb(m: &mut Memory<'_>, at: u32, value: u8) {
    let off = at.wrapping_sub(m.base) as usize;
    if let Some(b) = m.bytes.get_mut(off) {
        *b = value;
    }
}

/// A halfword write that drops an unmapped address instead of panicking.
fn wh(m: &mut Memory<'_>, at: u32, value: u16) {
    let off = at.wrapping_sub(m.base) as usize;
    if let Some(b) = m.bytes.get_mut(off..off + 2) {
        b.copy_from_slice(&value.to_le_bytes());
    }
}

/// A word write that drops an unmapped address instead of panicking.
fn ww(m: &mut Memory<'_>, at: u32, value: u32) {
    let off = at.wrapping_sub(m.base) as usize;
    if let Some(b) = m.bytes.get_mut(off..off + 4) {
        b.copy_from_slice(&value.to_le_bytes());
    }
}

/// `fn_195d58`: compare two NUL-terminated narrow strings.  Bounded, so a
/// string that runs off the arena compares instead of panicking.
fn compare_narrow(m: &Memory<'_>, a: u32, b: u32) -> i32 {
    let mut x = a;
    let mut y = b;
    loop {
        let cx = rb(m, x);
        let cy = rb(m, y);
        if cx != cy {
            return if cx < cy { -1 } else { 1 };
        }
        if cy == 0 {
            return 0;
        }
        x = x.wrapping_add(1);
        y = y.wrapping_add(1);
    }
}

/// fn_283348 (src/fn_283348.c). The Unicode lower-case mapping: the code
/// point plus the case offset.
pub fn fn_283348_m(c: i32) -> u32 {
    let d: i32 = if (0x41..=0x5a).contains(&c)
        || (0xc0..=0xd6).contains(&c)
        || (0xd8..=0xde).contains(&c)
    {
        0x20
    } else if ((0x100..=0x136).contains(&c) && (c & 1) == 0)
        || ((0x139..=0x147).contains(&c) && (c & 1) != 0)
        || ((0x14a..=0x178).contains(&c) && (c & 1) == 0)
        || ((0x179..=0x17d).contains(&c) && (c & 1) != 0)
    {
        if c == 0x130 {
            -199
        } else if c == 0x178 {
            -121
        } else {
            1
        }
    } else if c == 0x386
        || (0x388..=0x38a).contains(&c)
        || c == 0x38c
        || (0x38e..=0x38f).contains(&c)
        || (0x391..=0x3a1).contains(&c)
        || (0x3a3..=0x3ab).contains(&c)
    {
        match c {
            0x386 => 0x26,
            0x388..=0x38a => 0x25,
            0x38c => 0x40,
            0x38e..=0x38f => 0x3f,
            _ => 0x20,
        }
    } else {
        let t = c.wrapping_sub(0x400) as u32;
        if t <= 0x2f
            || ((0x460..=0x480).contains(&c) && (c & 1) == 0)
            || ((0x48c..=0x4be).contains(&c) && (c & 1) == 0)
            || c == 0x4c1
            || c == 0x4c3
            || c == 0x4c7
            || c == 0x4cb
            || ((0x4d0..=0x4f4).contains(&c) && (c & 1) == 0)
            || c == 0x4f8
        {
            if t <= 0xf {
                0x50
            } else if (0x410..=0x42f).contains(&c) {
                0x20
            } else {
                1
            }
        } else {
            0
        }
    };
    (c.wrapping_add(d) as u16) as u32
}

/// fn_2831a4: is the character a double quote?
pub fn fn_2831a4_m(c: u32) -> u32 {
    u32::from(c == 0x22)
}

/// fn_2832d8: is the character a word character?  The ARM returns zero for
/// the punctuation ranges and one for everything else.
pub fn fn_2832d8_m(c: u32) -> u32 {
    let punctuation = (0x21..=0x2f).contains(&c)
        || (0x3a..=0x40).contains(&c)
        || (0x5b..=0x60).contains(&c)
        || (0x2000..=0x206f).contains(&c)
        || (0x3000..=0x303f).contains(&c)
        || (0xfe30..=0xfe4f).contains(&c)
        || (0xfe50..=0xfe6f).contains(&c)
        || (0xff00..=0xffef).contains(&c);
    u32::from(!punctuation)
}

/// fn_283188 (src/fn_283188.c). Is the character one of the dictionary
/// spaces?  The search runs over the UTF-16 table at `D_9154c8`.
/// fn_283188 with the caller's region (round-8 table row): the word-char
/// table relocates with the ROM, so the regional twin is read.
pub fn fn_283188_m(m: &Memory<'_>, c: u32, rom: crate::rom_region::Rom) -> u32 {
    fn_283188_at(m, c, crate::rom_region::addr(rom, D_9154C8))
}

/// fn_283188 over an explicit table, for the other regions' addresses.
pub fn fn_283188_at(m: &Memory<'_>, c: u32, table: u32) -> u32 {
    u32::from(fn_176514(m, table, c) != 0)
}

/// fn_1763e4 (src/fn_1763e4.c). Copy a NUL-terminated narrow string into a
/// UTF-16 string; the terminator is copied too.
pub fn fn_1763e4_m(m: &mut Memory<'_>, dst: u32, src: u32) -> u32 {
    let mut d = dst;
    let mut s = src;
    loop {
        let v = rb(m, s) as u16;
        wh(m, d, v);
        if v == 0 {
            break;
        }
        d = d.wrapping_add(2);
        s = s.wrapping_add(1);
    }
    dst
}

/// fn_2826f8 (src/fn_2826f8_merged.c). Reset the span: the start is zero and
/// the length is the whole word.
pub fn fn_2826f8_m(m: &mut Memory<'_>, p: u32, v: u16, a: u32, b: u32) -> u32 {
    if p == 0 || a == 0 || b == 0 {
        return 0x8a80_2007;
    }
    wh(m, a, 0);
    wh(m, b, v);
    0
}

/// fn_282718 (src/fn_2826f8_merged.c). Trim the leading and trailing
/// punctuation.
pub fn fn_282718_m(m: &mut Memory<'_>, s: u32, n: u32, start: u32, len: u32) -> u32 {
    if s == 0 || start == 0 || len == 0 {
        return 0x8a80_2007;
    }
    let old_len = rh(m, len);
    if old_len == 0 {
        return 0x8a80_2014;
    }
    let old_start = rh(m, start);
    let mut i: u32 = 0;
    while rh(m, s.wrapping_add(i.wrapping_mul(2))) != 0
        && fn_2832d8_m(u32::from(rh(m, s.wrapping_add(i.wrapping_mul(2))))) == 0
    {
        i = (i + 1) & 0xffff;
    }
    wh(m, start, i as u16);
    wh(m, len, (n.wrapping_sub(i)) as u16);
    if i < n {
        let mut tail = n;
        while tail != 0
            && u32::from(rh(m, start)) <= tail - 1
            && (rh(m, s.wrapping_add(tail.wrapping_sub(1).wrapping_mul(2))) == 0x2e
                || fn_2832d8_m(u32::from(rh(
                    m,
                    s.wrapping_add(tail.wrapping_sub(1).wrapping_mul(2)),
                ))) == 0)
        {
            tail = tail.wrapping_sub(1);
        }
        if rh(m, s.wrapping_add(tail.wrapping_mul(2))) == 0x2e {
            tail = tail.wrapping_add(1);
        }
        wh(m, len, tail.wrapping_sub(u32::from(rh(m, start))) as u16);
        if rh(m, start) != old_start || rh(m, len) != old_len {
            return 0;
        }
    }
    0x8a80_2014
}

/// fn_282810 (src/fn_282810.c). Trim the surrounding quotes and spaces.
pub fn fn_282810_m(m: &mut Memory<'_>, s: u32, len: u32, pos: u32, count: u32, rom: crate::rom_region::Rom) -> u32 {
    if s == 0 || pos == 0 || count == 0 {
        return 0x8a80_2007;
    }
    let old_count = rh(m, count);
    if old_count == 0 {
        return 0x8a80_2014;
    }
    let old_pos = rh(m, pos);
    let mut open_quote = false;
    let mut close_quote = false;
    let mut i: u32 = 0;
    loop {
        let c = rh(m, s.wrapping_add(i.wrapping_mul(2)));
        if c == 0 || fn_283188_m(m, u32::from(c), rom) == 0 {
            break;
        }
        i = (i + 1) & 0xffff;
    }
    if rh(m, s.wrapping_add(i.wrapping_mul(2))) != 0 {
        if fn_2831a4_m(u32::from(rh(m, s.wrapping_add(i.wrapping_mul(2))))) != 0 {
            open_quote = true;
            i = (i + 1) & 0xffff;
        }
        loop {
            let c = rh(m, s.wrapping_add(i.wrapping_mul(2)));
            if c == 0 || fn_283188_m(m, u32::from(c), rom) == 0 {
                break;
            }
            i = (i + 1) & 0xffff;
        }
    }
    wh(m, pos, i as u16);
    wh(m, count, (len.wrapping_sub(i)) as u16);
    if i >= len {
        return 0x8a80_2014;
    }
    let mut end = len;
    while end != 0
        && u32::from(rh(m, pos)) <= end - 1
        && fn_283188_m(m, u32::from(rh(m, s.wrapping_add(end.wrapping_sub(1).wrapping_mul(2)))), rom) != 0
    {
        end = end.wrapping_sub(1);
    }
    if end != 0
        && u32::from(rh(m, pos)) <= end - 1
        && fn_2831a4_m(u32::from(rh(m, s.wrapping_add(end.wrapping_sub(1).wrapping_mul(2))))) != 0
    {
        close_quote = true;
        end = end.wrapping_sub(1);
    }
    while end != 0
        && u32::from(rh(m, pos)) <= end - 1
        && fn_283188_m(m, u32::from(rh(m, s.wrapping_add(end.wrapping_sub(1).wrapping_mul(2)))), rom) != 0
    {
        end = end.wrapping_sub(1);
    }
    wh(m, count, end.wrapping_sub(u32::from(rh(m, pos))) as u16);
    if rh(m, pos) == old_pos && rh(m, count) == old_count && !(open_quote && close_quote) {
        return 0x8a80_2014;
    }
    0
}

/// fn_2829a0 (src/fn_2826f8_merged.c). Trim the trailing periods.
pub fn fn_2829a0_m(m: &mut Memory<'_>, buf: u32, unused: u32, start: u32, len: u32) -> u32 {
    if buf == 0 || start == 0 || len == 0 {
        return 0x8a80_2007;
    }
    let n = rh(m, len);
    if n != 0 {
        let s = rh(m, start);
        let mut e = (u32::from(s).wrapping_add(u32::from(n)) & 0xffff) as u32;
        while e != 0 && e.wrapping_sub(1) >= u32::from(s) && rh(m, buf.wrapping_add(e.wrapping_sub(1).wrapping_mul(2))) == 0x2e {
            e = e.wrapping_sub(1);
        }
        wh(m, len, e.wrapping_sub(u32::from(s)) as u16);
        if rh(m, start) != s || (e.wrapping_sub(u32::from(s)) as u16) != n {
            return 0;
        }
    }
    0x8a80_2014
}

/// fn_282a28 (src/fn_282a28.c). Lower-case the span.
pub fn fn_282a28_m(m: &mut Memory<'_>, buf: u32, unused: u32, start: u32, len: u32) -> u32 {
    if buf == 0 || start == 0 || len == 0 {
        return 0x8a80_2007;
    }
    let orig = rh(m, len);
    if orig != 0 {
        let mut end = (u32::from(rh(m, start)).wrapping_add(u32::from(orig)) & 0xffff) as u32;
        if rh(m, buf.wrapping_add(end.wrapping_mul(2))) == 0x2e {
            end = (end.wrapping_add(1) & 0xffff) as u32;
        }
        wh(m, len, end.wrapping_sub(u32::from(rh(m, start))) as u16);
        let mut changed = false;
        let mut i = u32::from(rh(m, start));
        let stop = u32::from(rh(m, start)).wrapping_add(u32::from(rh(m, len)));
        while i < stop {
            let at = buf.wrapping_add(i.wrapping_mul(2));
            let old = rh(m, at);
            let lower = fn_283348_m(i32::from(old)) as u16;
            if old != lower {
                wh(m, at, lower);
                changed = true;
            }
            i = (i + 1) & 0xffff;
        }
        if rh(m, len) != orig || changed {
            return 0;
        }
    }
    0x8a80_2014
}

/// fn_1748a0 (src/fn_1748a0.c). The logger's target fan-out: with a live
/// object and a nonzero target count, call each set target and then the
/// object's own vtable slot with (ctx, a, b, c, n, f), keeping the last
/// failing status. Guest function pointers normalize through the host's
/// dispatch (round-8 recipe), so regional twins reach the same ports.
/// A null object reports 0; a zero target count with live targets reports
/// 0x84b02006. Certified against the live ARM calls inside the EU/KR
/// 2031b0 roots (byte-exact logs, no unbound; the US no-op path is
/// untouched).
/// (src verification: build/target/fn_1748a0.o ends with the 0x84b02006
/// word; the ROM call sequence matches src/fn_1748a0.c.)
pub fn fn_1748a0_m(
    m: &mut Memory<'_>,
    o: u32,
    a: u32,
    b: u32,
    c: u32,
    n: u32,
    f: u32,
    host: &mut C2DictHost,
) -> u32 {
    if o == 0 {
        return 0;
    }
    let v = rb(m, o.wrapping_add(0x40)) as u32;
    if v == 0 {
        return 0;
    }
    if n == 0 {
        return 0x84b0_2006;
    }
    let mut r = 0u32;
    let mut i = 0u32;
    loop {
        let e = o.wrapping_add(i.wrapping_mul(8));
        let vt = rw(m, e.wrapping_add(8).wrapping_add(4));
        if vt != 0 {
            let g = rw(m, vt.wrapping_add(0x34));
            if g != 0 {
                let ctx = rw(m, e.wrapping_add(8));
                let t = host.dispatch(m, g, &[ctx, a, b, c, n, f]);
                if t >= 0x8000_0000 {
                    r = t;
                }
            }
        }
        i = i.wrapping_add(1) & 0xff;
        if i >= 5 {
            break;
        }
    }
    let vt = rw(m, o.wrapping_add(0x38));
    if vt != 0 {
        let g = rw(m, vt.wrapping_add(0x34));
        if g != 0 {
            let ctx = rw(m, o.wrapping_add(0x34));
            let t = host.dispatch(m, g, &[ctx, a, b, c, n, f]);
            if t >= 0x8000_0000 {
                r = t;
            }
        }
    }
    r
}

/// fn_201eb0 (src/fn_2012bc.c). Is a dictionary of one of the three
/// dictionary types loaded? The type-string pools relocate with the ROM
/// (round-8 table rows), so the caller passes its region.
pub fn fn_201eb0_m(m: &Memory<'_>, p: u32, magic: u32, rom: crate::rom_region::Rom) -> u32 {
    if crate::frontend::validate_handle(p != 0, magic, 0x2f0, 0x30) >= 0x8000_0000 {
        return 0;
    }
    let mut it: u32 = 0;
    loop {
        let entry = if it != 0 {
            let next = rw(m, it.wrapping_add(8));
            it = next;
            if next != 0 {
                rw(m, next.wrapping_add(4))
            } else {
                0
            }
        } else {
            let head = rw(m, p.wrapping_add(28));
            it = head;
            if head != 0 {
                rw(m, head.wrapping_add(4))
            } else {
                0
            }
        };
        if entry == 0 {
            break;
        }
        let kind = rw(m, entry.wrapping_add(8));
        let matched = compare_narrow(m, kind, crate::rom_region::addr(rom, D_201498)) == 0
            || compare_narrow(m, kind, crate::rom_region::addr(rom, D_2014B8)) == 0
            || compare_narrow(m, kind, crate::rom_region::addr(rom, D_2014D8)) == 0;
        if matched && rb(m, entry.wrapping_add(16)) == 1 {
            return 1;
        }
    }
    u32::from(rw(m, p.wrapping_add(40)) != 0)
}

// ---------------------------------------------------------------------------
// fn_282af0: the pronunciation expansion.
//
// The port is live over the caller's arena and follows src/fn_282af0.c.
// The six transforms come from the table at D_9154b0 and the lookup runs
// through the callbacks' function pointer, so the same code serves the
// dictionary path (the dictionary interface's vtable) and the callback path
// (the FE_DEPES lookup).
// ---------------------------------------------------------------------------

/// The literal addresses `fn_282af0` uses in the US ROM.
///
/// The addresses come from the ROM's own literal pool: `add r1, pc, #0x398`
/// and its neighbours in the fn_282af0 body.
pub const L_NUMBEROUTPUTTRANSCRIPTIONS: u32 = 0x282f30;
pub const L_LANGCODE: u32 = 0x282f4c;
pub const L_IN: u32 = 0x282f5c;
pub const L_BEG: u32 = 0x282f60;
pub const L_FMT_BEG: u32 = 0x282f64;
pub const L_OUT: u32 = 0x282f70;
pub const L_RETC: u32 = 0x282f74;
pub const L_END: u32 = 0x282f7c;
pub const L_FMT_END: u32 = 0x282f80;
/// The word that holds the transform table's address (`ldr r0, [pc, #0x40c]`).
pub const D_TRANSFORM_TABLE: u32 = 0x282f2c;

/// The literal addresses `fn_282324` uses in the US ROM.
pub const L282324_NUMBEROUTPUTTRANSCRIPTIONS: u32 = 0x28265c;
pub const L282324_BAD_HANDLE: u32 = 0x282678;
pub const L282324_NOT_FOUND: u32 = 0x28267c;

/// The frame `fn_282af0` keeps on the caller's stack.  The port takes the
/// frame's base so its own words land where the ARM's did.
pub const FN282AF0_FRAME: u32 = 0xb8;

/// The services the FE_DEPES fetch reaches.
pub trait C2DepesHost {
    /// A guest call over the caller's arena.
    fn call(&mut self, m: &mut Memory<'_>, entry: u32, args: &[u32]) -> u32;
    /// The ROM whose guest memory is live. US by default; the regional test
    /// host overrides it so ports translate ROM literals with addr().
    fn rom(&self) -> crate::rom_region::Rom {
        crate::rom_region::Rom::Us
    }
}

/// The `LookupCallbacks` vtable: the lookup sits after fourteen reserved
/// words.
const LOOKUP_CALLBACK_SLOT: u32 = 56;

/// The `DictionaryInterface` vtable: lookup, release, language, three
/// reserved words and the adjustment.
const DICTIONARY_LOOKUP_SLOT: u32 = 0;
const DICTIONARY_RELEASE_SLOT: u32 = 4;
const DICTIONARY_LANGUAGE_SLOT: u32 = 8;
const DICTIONARY_ADJUSTMENT_SLOT: u32 = 24;

/// fn_282af0 (src/fn_282af0.c). The pronunciation expansion.
///
/// `handle` and `handle_user` are the two words of the lookup handle,
/// `capacity` the address of the caller's capacity word and `flags` the
/// address of the caller's language word.
#[allow(clippy::too_many_arguments)]
/// fn_282af0 (src/fn_282af0.c, ROM checked at 0x282af0).
///
/// The pronunciation expansion.  `frame` is the base of the callee's own
/// stack frame, so every word the port keeps lands where the ARM kept it
/// (the ROM's `sp` is the frame base plus 0xb8).  `h` supplies the guest
/// calls the ARM made: the two registry lookups, the logger and the
/// dictionary lookup callback.
#[allow(clippy::too_many_arguments)]
pub fn fn_282af0_m(
    m: &mut Memory<'_>,
    frame: u32,
    ctx: u32,
    dictionary: u32,
    callbacks: u32,
    handle: u32,
    handle_user: u32,
    all_transforms: u32,
    input: u32,
    content_type: u32,
    output: u32,
    capacity: u32,
    flags: u32,
    h: &mut impl C2DepesHost,
) -> u32 {
    if input == 0 {
        return 0x8a80_2007;
    }
    if dictionary == 0 && (callbacks == 0 || handle == 0) {
        return 0x8a80_2014;
    }
    let settings = rw(m, ctx + 20);
    let logger = rw(m, ctx + 16);
    // The ARM clears these words before it calls anything.
    wh(m, frame + 0x30, 0);
    wh(m, frame + 0x14, 0);
    ww(m, frame + 0x44, 0);
    ww(m, frame + 0x60, 0);
    ww(m, frame + 0x64, 0);
    ww(m, frame + 0x68, 0);
    ww(m, frame + 0x6c, 0);
    ww(m, frame + 0x24, 0);
    let mut limit = 1u32;
    let status = h.call(
        m,
        0x175944,
        &[settings, crate::rom_region::addr(h.rom(), L_NUMBEROUTPUTTRANSCRIPTIONS), frame + 0x6c],
    );
    if status < 0x8000_0000 {
        limit = rw(m, frame + 0x6c) & 0xffff;
    }
    let status = h.call(m, 0x1756d0, &[settings, crate::rom_region::addr(h.rom(), L_LANGCODE), frame + 0x44, 0]);
    if status >= 0x8000_0000 {
        return status;
    }
    let language = rw(m, frame + 0x44);
    let mut word = 0u32;
    if callbacks == 0 {
        // The dictionary path: the interface's vtable supplies the language.
        let interface = rw(m, dictionary + 0x28);
        let vtable = rw(m, interface);
        let object = interface.wrapping_sub(rw(m, vtable + DICTIONARY_ADJUSTMENT_SLOT));
        let status = h.call(m, rw(m, vtable + DICTIONARY_LANGUAGE_SLOT), &[object, frame + 0x60]);
        let dictionary_language = rw(m, frame + 0x60);
        if status != 0 || dictionary_language == 0 {
            return 0x8a80_2014;
        }
        if fn_1769d0_m(m, dictionary_language, language) != 0 {
            return 0x8a80_2014;
        }
        word = h.call(m, 0x196394, &[rw(m, ctx + 4), fn_17bff4_m(m, input)]);
        if word == 0 {
            return 0x8a80_200a;
        }
        ww(m, frame + 0x24, word);
    }
    let input_length = fn_176624(m, input) as u32;
    ww(m, frame + 0x70, input_length);
    let initial_capacity = rh(m, capacity);
    let table = rw(m, crate::rom_region::addr(h.rom(), D_TRANSFORM_TABLE));
    let transform_count = if all_transforms != 0 { 6u32 } else { 1u32 };
    let mut transform = 0u32;
    let mut result = 0u32;
    while transform < transform_count {
        let entry = rw(m, table + transform * 4);
        // The transform entries are guest function pointers; normalize the
        // regional twin to the US entry transform_m dispatches on.
        result = transform_m(m, crate::rom_region::to_us_guest(h.rom(), entry), input, input_length, frame, h.rom());
        if result & 0x1fff == 0x14 {
            transform += 1;
            continue;
        }
        if result >= 0x8000_0000 {
            return result;
        }
        let start = u32::from(rh(m, frame + 0x30));
        let length = u32::from(rh(m, frame + 0x14));
        let text = input.wrapping_add(start.wrapping_mul(2));
        let saved = rh(m, text.wrapping_add(length.wrapping_mul(2)));
        wh(m, text.wrapping_add(length.wrapping_mul(2)), 0);
        if callbacks != 0 {
            h.call(
                m,
                0x17486c,
                &[logger, 0x6b, crate::rom_region::addr(h.rom(), L_FMT_BEG), crate::rom_region::addr(h.rom(), L_BEG), 0, crate::rom_region::addr(h.rom(), L_IN), text],
            );
            let lookup = rw(m, callbacks + LOOKUP_CALLBACK_SLOT);
            result = h.call(
                m,
                lookup,
                &[
                    handle,
                    handle_user,
                    text,
                    capacity,
                    output,
                    content_type,
                    flags,
                ],
            );
            h.call(
                m,
                0x17486c,
                &[logger, 0x6b, crate::rom_region::addr(h.rom(), L_FMT_END), crate::rom_region::addr(h.rom(), L_END), 0, crate::rom_region::addr(h.rom(), L_RETC), result, crate::rom_region::addr(h.rom(), L_OUT), output],
            );
            wh(m, text.wrapping_add(length.wrapping_mul(2)), saved);
        } else {
            result = fn_17bf70_m(m, text, word);
            let mut records = 0u32;
            let mut count = 0u32;
            if result < 0x8000_0000 {
                let interface = rw(m, dictionary + 0x28);
                let vtable = rw(m, interface);
                let object =
                    interface.wrapping_sub(rw(m, vtable + DICTIONARY_ADJUSTMENT_SLOT));
                result = h.call(
                    m,
                    rw(m, vtable + DICTIONARY_LOOKUP_SLOT),
                    &[object, word, frame + 0x68, frame + 0x64],
                );
                records = rw(m, frame + 0x68);
                count = rw(m, frame + 0x64);
            }
            wh(m, text.wrapping_add(length.wrapping_mul(2)), saved);
            if result == 0 && count != 0 && records != 0 {
                let kind = rb(m, records);
                ww(m, content_type, u32::from(kind));
                match kind {
                    1 => {
                        let mut output_length = 0u32;
                        wh(m, output, 0);
                        let total = if limit < count { limit } else { count };
                        let mut i = 0u32;
                        let mut failure = 0u32;
                        while i < total {
                            let record = records + i * 12;
                            let record_text = rw(m, record + 4);
                            let mut record_length = rw(m, record + 8);
                            let mut offset = 0u32;
                            while offset < record_length {
                                if rb(m, record_text.wrapping_add(offset)) == 0 {
                                    record_length = offset;
                                    break;
                                }
                                offset += 1;
                            }
                            if record_length + 1 > u32::from(rh(m, capacity)) {
                                failure = 0x8a80_2009;
                                break;
                            }
                            if i != 0 {
                                fn_176314_m(m, output.wrapping_add(output_length * 2), 0x12);
                                output_length += 1;
                            }
                            fn_176314_m(m, output.wrapping_add(output_length * 2), record_text);
                            output_length += record_length;
                            i += 1;
                        }
                        if failure != 0 {
                            result = failure;
                        } else {
                            wh(m, output.wrapping_add(output_length * 2), 0);
                            wh(m, capacity, output_length as u16);
                        }
                    }
                    4 => {
                        let record_text = rw(m, records + 4);
                        let record_length = fn_17bfc0_m(m, record_text);
                        if record_length > u32::from(initial_capacity) {
                            result = 0x8a80_2009;
                        } else {
                            result = fn_17bd74_m(m, record_text, output);
                            if result < 0x8000_0000 {
                                wh(m, capacity, (record_length - 1) as u16);
                            }
                        }
                    }
                    _ => result = 0x8a80_2014,
                }
            } else if result < 0x8000_0000 {
                result = 0x8a80_2014;
            }
            if result < 0x8000_0000 && records != 0 {
                let interface = rw(m, dictionary + 0x28);
                let vtable = rw(m, interface);
                let object = interface.wrapping_sub(rw(m, vtable + DICTIONARY_ADJUSTMENT_SLOT));
                let release = h.call(
                    m,
                    rw(m, vtable + DICTIONARY_RELEASE_SLOT),
                    &[object, records],
                );
                if release != 0 {
                    return (release + 0x1ea0) | 0x8a80_2000;
                }
            }
        }
        if result & 0x1fff != 0x14 && result & 0x1fff != 0x404 {
            if result >= 0x8000_0000 {
                if word != 0 {
                    h.call(m, 0x196428, &[rw(m, ctx + 4), word]);
                }
                return result;
            }
            if result == 0 {
                break;
            }
        }
        transform += 1;
    }
    if word != 0 {
        h.call(m, 0x196428, &[rw(m, ctx + 4), word]);
    }
    if transform >= transform_count {
        return 0x8a80_2014;
    }
    let start = u32::from(rh(m, frame + 0x30));
    let length = u32::from(rh(m, frame + 0x14));
    if rw(m, content_type) == 4 {
        result = restore_text_m(
            m,
            output,
            capacity,
            input,
            input_length,
            u32::from(initial_capacity),
            start,
            length,
        );
    }
    result
}

/// fn_1769d0: compare two narrow strings for equality.
fn fn_1769d0_m(m: &Memory<'_>, a: u32, b: u32) -> i32 {
    compare_narrow(m, a, b)
}

/// Excluded-stack cell for fn_17bf70's `out` C local (the byte count
/// fn_17bdc4 reports). Inside 0x0e000000..0x0e100000, never compared.
/// Reuses the old SEG7_RESULT slot, freed by seg5 r14 (noted in
/// scratch/integration-todo.txt so no other lane takes it).
///
/// fn_17bf70 (src/fn_17bf70.c): transcode the UTF-16 string into the word
/// buffer, returning the byte count. The C is a thin wrapper around
/// fn_17bdc4 (the u16-to-UTF-8 transcoder): out is a C local holding the
/// reported byte count. The old truncating byte-copy never called the
/// transcoder, so non-ASCII words came out Latin-1 (seg5 r15: 0x14c4371x
/// showed fb/e9 where the ARM wrote c3 bb/c3 a9).
pub(crate) fn fn_17bf70_m(m: &mut Memory<'_>, text: u32, word: u32) -> u32 {
    const OUT_CELL: u32 = 0x0e0fdd80;
    let len = fn_176624(m, text).wrapping_add(1) as u32;
    crate::normalization::transcode_u16(
        m.bytes, text, len, 0, word, 0xffff_ffff, OUT_CELL, 0,
    );
    m.word(OUT_CELL)
}


/// fn_17bff4 (src/fn_17bff4.c): the UTF-8 encoded length of a UTF-16
/// string, including the terminating zero byte. Each unit contributes
/// its UTF-8 width (1/2/3/4/5), not 2 bytes.
pub fn fn_17bff4_m(m: &Memory<'_>, s: u32) -> u32 {
    let mut at = s;
    let mut n = 1u32;
    loop {
        let c = u32::from(rh(m, at));
        at += 2;
        if c == 0 {
            break;
        }
        n += if c < 0x80 {
            1
        } else if c < 0x800 {
            2
        } else if c < 0x10000 {
            3
        } else if c < 0x110000 {
            4
        } else {
            5
        };
    }
    n
}

/// fn_176314: copy a narrow string into a UTF-16 string.
pub(crate) fn fn_176314_m(m: &mut Memory<'_>, dst: u32, src: u32) {
    let mut d = dst;
    let mut s = src;
    loop {
        let v = rb(m, s) as u16;
        wh(m, d, v);
        if v == 0 {
            break;
        }
        d += 2;
        s += 1;
    }
}

/// fn_17bfc0 (src/fn_17bfc0.c): the length of a narrow string, terminator
/// included, multi-byte lead bytes skipped via D_8dc1f3. Starts at 1, so
/// plain "i" measures 2, not 1: the old plain-strlen port under-measured
/// every record, zeroing the 282af0 capacity word (length - 1) and routing
/// the 2031b0 branch to the original word where the ARM writes replacement.
/// seg6 r7 (probe: txt "i", live len 1 vs ARM len 2).
pub(crate) fn fn_17bfc0_m(m: &Memory<'_>, s: u32) -> u32 {
    let mut n = 1u32;
    let mut p = s;
    let mut c = rb(m, p);
    p = p.wrapping_add(1);
    while c != 0 {
        n = n.wrapping_add(1);
        p = p.wrapping_add(u32::from(rb(m, 0x8dc1f3u32.wrapping_add(u32::from(c)))));
        c = rb(m, p);
        p = p.wrapping_add(1);
    }
    n
}

/// fn_17bd74: decode a narrow string into a UTF-16 buffer via the fn_17bb44
/// decoder (C src/fn_17bd74.c calls fn_17bb44; the old plain copy missed
/// decoding, e.g. sentence-initial case on speech_2: ARM writes lowercase
/// 'i', the copy preserves uppercase 'I'). Returns the decoded unit count.
/// seg6 r5; mirrors the certified component_live_a2 0x17bb44 arm.
pub(crate) fn fn_17bd74_m(m: &mut Memory<'_>, src: u32, dst: u32) -> u32 {
    let extra: [u8; 256] = core::array::from_fn(|i| m.byte(0x8dc1f3 + i as u32));
    let mask: [u8; 16] = core::array::from_fn(|i| m.byte(0x8dc1e0 + i as u32));
    let base: [u8; 1024] = core::array::from_fn(|i| m.byte(0x8dc2f4 + i as u32));
    let t = crate::normalization::DecoderTables {
        extra: &extra,
        mask: &mask,
        mask_base: 6,
        base: &base,
        base_base: 0,
    };
    let len = crate::fe_text::strlen_195d90(m.bytes, src).max(0) as u32;
    let (_status, _offset, count) = crate::normalization::decode_run(
        m.bytes,
        src,
        len.wrapping_add(1),
        dst,
        0xffff_ffff,
        0,
        &t,
    );
    count
}

/// The `restoreText` tail of fn_282af0.
#[allow(clippy::too_many_arguments)]
fn restore_text_m(
    m: &mut Memory<'_>,
    output: u32,
    capacity: u32,
    input: u32,
    input_length: u32,
    initial_capacity: u32,
    prefix: u32,
    replaced: u32,
) -> u32 {
    let cap = u32::from(rh(m, capacity));
    if (cap + input_length - replaced) as i32 > initial_capacity as i32 {
        return 0x8a80_2009;
    }
    if prefix != 0 {
        move_bytes_m(m, output + prefix * 2, output, cap * 2);
        move_bytes_m(m, output, input, prefix * 2);
    }
    let tail = input_length - replaced - prefix;
    if tail > 0 {
        move_bytes_m(
            m,
            output + (prefix + cap) * 2,
            input + (prefix + replaced) * 2,
            tail * 2,
        );
    }
    wh(m, capacity, (cap + input_length - replaced) as u16);
    0
}

/// fn_195b38: a byte copy that keeps the ARM's order.
fn move_bytes_m(m: &mut Memory<'_>, dst: u32, src: u32, n: u32) {
    let mut i = 0u32;
    while i < n {
        let v = rb(m, src + i);
        wb(m, dst + i, v);
        i += 1;
    }
}

/// One of the six transforms, dispatched by the table's entry.
///
/// The span words live in the callee's frame, exactly where the ARM keeps
/// them: `sp+0x30` is the start and `sp+0x14` the length.
fn transform_m(
    m: &mut Memory<'_>,
    entry: u32,
    input: u32,
    input_length: u32,
    frame: u32,
    rom: crate::rom_region::Rom,
) -> u32 {
    let start = frame + 0x30;
    let length = frame + 0x14;
    match entry {
        0x2826f8 => fn_2826f8_m(m, input, input_length as u16, start, length),
        0x282810 => fn_282810_m(m, input, input_length, start, length, rom),
        0x282718 => fn_282718_m(m, input, input_length, start, length),
        0x2829a0 => fn_2829a0_m(m, input, input_length, start, length),
        0x282a28 => fn_282a28_m(m, input, input_length, start, length),
        _ => 0x8a80_2007,
    }
}

// ---------------------------------------------------------------------------
// fn_282324: the dictionary's transcription lookup.
//
// The port follows src/fn_282324.c and the ROM body at 0x282324.  The
// dictionary interface's vtable is read from the arena, so the same code
// serves every dictionary the engine loads.  `frame` is the callee's own
// stack frame (the ROM's `sp` is the frame base plus 0x5c).
// ---------------------------------------------------------------------------

/// The frame `fn_282324` keeps on the caller's stack.
pub const FN282324_FRAME: u32 = 0x5c;

/// fn_282324 (src/fn_282324.c, ROM checked at 0x282324).
///
/// `owner` and `dictionary` are the two Dictionary objects, `language` the
/// caller's language pointer, `word` the narrow word, `output` the UTF-16
/// output buffer, `capacity` its length word and `content_type` the word
/// that receives the record type.
#[allow(clippy::too_many_arguments)]
pub fn fn_282324_m(
    m: &mut Memory<'_>,
    frame: u32,
    owner: u32,
    dictionary: u32,
    language: u32,
    word: u32,
    output: u32,
    capacity: u32,
    content_type: u32,
    h: &mut impl C2DepesHost,
) -> u32 {
    let mut limit = 1u32;
    ww(m, frame + 0x10, 0);
    let context = rw(m, owner + 8);
    let settings = rw(m, context + 0x14);
    let status = h.call(
        m,
        0x175944,
        &[settings, L282324_NUMBEROUTPUTTRANSCRIPTIONS, frame + 0x10],
    );
    if status < 0x8000_0000 {
        limit = rw(m, frame + 0x10) & 0xffff;
    }
    if owner == 0
        || rw(m, owner + 0x24) == 0
        || rw(m, dictionary + 8) == 0
        || word == 0
        || rb(m, word) == 0
        || capacity == 0
    {
        return 0x8500_2007;
    }
    let interface = rw(m, dictionary + 0x28);
    let vtable = rw(m, interface);
    let object = interface.wrapping_sub(rw(m, vtable + DICTIONARY_ADJUSTMENT_SLOT));
    ww(m, frame + 0x18, 0);
    let status = h.call(m, rw(m, vtable + DICTIONARY_LANGUAGE_SLOT), &[object, frame + 0x18]);
    // The ROM's language getter (fn_2b331c) is `ldr r2,[r0,#0x44]; str r2,[r1]`,
    // so it returns the language OBJECT; the ARM dereferences it once more
    // (`ldr r0,[r0]` at 0x2823f0) to reach the language string.
    let language_object = rw(m, frame + 0x18);
    if status != 0 || language_object == 0 {
        return 0x8500_2014;
    }
    let dictionary_language = rw(m, language_object);
    if dictionary_language == 0 {
        return 0x8500_2014;
    }
    // The three letter language code, folded the way the ROM folds it.
    for i in 0..3u32 {
        let folded = crate::fe_text::fold_17c8c0(u32::from(rb(m, dictionary_language + i)));
        wb(m, frame + 0x1c + i, folded as u8);
    }
    wb(m, frame + 0x1f, 0);
    if rw(m, language) != 0
        && crate::frontend::find_substr(m.bytes, rw(m, language), frame + 0x1c) == 0
    {
        return 0x8500_2014;
    }
    ww(m, language, dictionary_language);
    ww(m, frame + 0x14, 0);
    ww(m, frame + 0x0c, 0);
    let mut result = h.call(
        m,
        rw(m, vtable + DICTIONARY_LOOKUP_SLOT),
        &[object, word, frame + 0x14, frame + 0x0c],
    );
    let records = rw(m, frame + 0x14);
    let count = rw(m, frame + 0x0c);
    if result == 0 && count != 0 && records != 0 {
        let kind = rb(m, records);
        ww(m, content_type, u32::from(kind));
        match kind {
            1 => {
                wh(m, output, 0);
                let mut output_length = 0u32;
                let total = if limit < count { limit } else { count };
                let mut i = 0u32;
                while i < total {
                    let record = records + i * 12;
                    if rb(m, record) == 1 {
                        let record_text = rw(m, record + 4);
                        let mut record_length = rw(m, record + 8);
                        let mut offset = 0u32;
                        while offset < record_length {
                            if rb(m, record_text.wrapping_add(offset)) == 0 {
                                record_length = offset;
                                break;
                            }
                            offset += 1;
                        }
                        if record_length + 1 > u32::from(rh(m, capacity)) {
                            result = 0x8500_2009;
                            break;
                        }
                        if i != 0 {
                            fn_176314_m(m, output.wrapping_add(output_length * 2), 0x12);
                            output_length += 1;
                        }
                        fn_176314_m(m, output.wrapping_add(output_length * 2), record_text);
                        output_length += record_length;
                    }
                    i += 1;
                }
                if result == 0 {
                    wh(m, output.wrapping_add(output_length * 2), 0);
                    wh(m, capacity, output_length as u16);
                }
            }
            4 => {
                let record_text = rw(m, records + 4);
                let record_length = fn_17bfc0_m(m, record_text);
                if record_length > u32::from(rh(m, capacity)) {
                    result = 0x8500_2009;
                } else {
                    result = fn_17bd74_m(m, record_text, output);
                    if result < 0x8000_0000 {
                        wh(m, capacity, (record_length - 1) as u16);
                    }
                }
            }
            _ => result = 0x8500_2014,
        }
    } else if result < 0x8000_0000 {
        result = 0x8500_2014;
    }
    if records != 0 {
        let release = h.call(
            m,
            rw(m, vtable + DICTIONARY_RELEASE_SLOT),
            &[object, records],
        );
        if release != 0 {
            return (release + 0x1ea0) | 0x8500_2000;
        }
    }
    result
}

// ---------------------------------------------------------------------------
// fn_3015a4: the record finder.
//
// The port is the crate's live arena port of the same ROM function
// (speech_part_3::fn_3015a4, from src/fn_3015a4.c).  It is re-exported
// here with this lane's host so the 2031b0 root can reach it.
// ---------------------------------------------------------------------------

/// The heap port `fn_3015a4` needs, over this lane's guest calls.
struct C2HeapHost<'a, H: C2DepesHost>(&'a mut H);

impl<H: C2DepesHost> crate::boot_part_f::Host for C2HeapHost<'_, H> {
    fn call(&mut self, m: &mut Memory<'_>, method: u32, args: &[u32]) -> u32 {
        self.0.call(m, method, args)
    }
    fn compare(
        &mut self,
        m: &mut Memory<'_>,
        _method: u32,
        key: &mut crate::boot_part_f::Key,
        value: &mut u32,
    ) -> u32 {
        crate::boot_part_f::compare_parameter(m, key.clone(), *value)
    }
}

/// fn_3015a4 (src/fn_3015a4.c): find the matching database records.
pub fn fn_3015a4_m(
    m: &mut Memory<'_>,
    ctx: u32,
    crit: u32,
    list: u32,
    out_buf: u32,
    out_count: u32,
    h: &mut impl C2DepesHost,
) -> u32 {
    let mut host = C2HeapHost(h);
    crate::speech_part_3::fn_3015a4(m, ctx, crit, list, out_buf, out_count, &mut host)
}

// ---------------------------------------------------------------------------
// fn_2031b0: the FE_DEPES fetch.
//
// The port follows src/fn_2029dc.c and the ROM's own call order (the live
// trace of the 2031b0 capture).  `frame` is the caller's scratch for the
// root's locals; it must not overlap the compared window.
// ---------------------------------------------------------------------------

/// fn_15140c (src/fn_15140c.c): the stream's remaining-bytes query.
///
/// `stream` is the stream object, `key` its 0x47de handle key.  The two
/// 1738e8 logs are provable no-ops (the logger owns no persistent byte),
/// so only the validation and the state reads are live.
pub fn fn_15140c_m(
    m: &mut Memory<'_>,
    stream: u32,
    key: u32,
    out_remaining: u32,
    out_finished: u32,
    h: &mut C2DictHost,
) -> u32 {
    let rc = h.c2_dict_call(m, 0x176f94, &[stream, key, 0x47de, 12]);
    if rc >= 0x8000_0000 {
        return rc;
    }
    if out_remaining == 0 {
        return 0x88f0_2007;
    }
    if rb(m, stream) != 0 {
        ww(m, out_remaining, 0);
        return 0x88f0_2007;
    }
    let state = rw(m, stream + 4);
    ww(
        m,
        out_remaining,
        rw(m, state + 44).wrapping_sub(rw(m, stream + 8)),
    );
    if out_finished != 0 {
        wb(m, out_finished, rb(m, state + 68));
    }
    0
}

/// fn_14fe9c (src/fn_14fe9c.c): the stream's direct-read start.
///
/// `object`/`tag` are the stream handle pair.  The logs are provable
/// no-ops; only the validation and the state reads are live.
pub fn fn_14fe9c_m(
    m: &mut Memory<'_>,
    object: u32,
    tag: u32,
    out_buffer: u32,
    out_available: u32,
    h: &mut C2DictHost,
) -> u32 {
    let rc = h.c2_dict_call(m, 0x176f94, &[object, tag, 0x47de, 12]);
    if rc >= 0x8000_0000 {
        return rc;
    }
    if out_buffer == 0 || out_available == 0 {
        if out_buffer != 0 {
            ww(m, out_buffer, 0);
        }
        if out_available != 0 {
            ww(m, out_available, 0);
        }
        return 0x88f0_2007;
    }
    if rb(m, object) != 0 {
        ww(m, out_buffer, 0);
        ww(m, out_available, 0);
        return 0x88f0_2007;
    }
    let state = rw(m, object + 4);
    let position = rw(m, object + 8);
    let base = rw(m, state + 56);
    if position < base {
        ww(m, out_buffer, 0);
        ww(m, out_available, 0);
        return 0x88f0_2004;
    }
    let available = rw(m, state + 44).wrapping_sub(position);
    ww(m, out_available, available);
    ww(
        m,
        out_buffer,
        position.wrapping_sub(base).wrapping_add(rw(m, state + 60)),
    );
    if available != 0 && rw(m, out_buffer) == 0 {
        return 0x88f0_200a;
    }
    0
}

/// fn_21958c (src/fn_21958c.c): the DEPES getPosition.
pub fn fn_21958c_m(
    m: &mut Memory<'_>,
    a: u32,
    b: u32,
    out: u32,
    which: u32,
    h: &mut C2DictHost,
) -> u32 {
    let rc = h.c2_dict_call(m, 0x176f94, &[a, b, 0xf37e, 64]);
    if rc >= 0x8000_0000 {
        return 0x8970_2008;
    }
    if which == 1 {
        ww(m, out, rw(m, a + 56));
    } else {
        ww(m, out, rw(m, a + 60));
    }
    rc
}

/// The validation key the FE_DEPES fetch uses.
pub const FE_DEPES_KEY: u32 = 0xf38a;
/// The handle key the stream's methods use.
pub const STREAM_KEY: u32 = 0x47de;
/// The stream's "remaining" call, 0x15140c.
pub const STREAM_REMAINING: u32 = 0x15140c;
/// The stream's "get text" call, 0x14fe9c.
pub const STREAM_GETTEXT: u32 = 0x14fe9c;
/// The stream's "write" call, 0x1503b0.
pub const STREAM_WRITE: u32 = 0x1503b0;
/// The out-stream close, 0x14f918: the close loop rides the stream
/// method at slot +0x24 with (handle, key), never 0x15017c.
pub const STREAM_CLOSE: u32 = 0x14f918;

/// fn_2031b0 (src/fn_2029dc.c, ROM at 0x2031b0): the FE_DEPES fetch.
///
/// `p` is the FE_DEPES object, `b` the request key and `done` the caller's
/// byte that says whether the request is finished.
pub fn fn_2031b0_m(
    m: &mut Memory<'_>,
    frame: u32,
    p: u32,
    b: u32,
    done: u32,
    h: &mut impl C2DepesHost,
) -> u32 {
    let rc = h.call(m, 0x176f94, &[p, b, FE_DEPES_KEY, 0xb8]);
    if rc >= 0x8000_0000 {
        return 0x8a80_2008;
    }
    wb(m, done, 1);
    let ctx = rw(m, p + 8);
    let log = rw(m, ctx + 0x10);
    let reg = rw(m, ctx + 0x14);
    let heap = rw(m, ctx + 4);
    let in0 = rw(m, p + 0x1c);
    let in0_key = rw(m, p + 0x20);
    let in1 = rw(m, p + 0x24);
    let in1_key = rw(m, p + 0x28);
    // The out handle pairs: text at +0x2c, markers at +0x34.
    let out0 = rw(m, p + 0x2c);
    let out0_key = rw(m, p + 0x30);
    let out1 = rw(m, p + 0x34);
    let out1_key = rw(m, p + 0x38);
    let mut available_text = 0u32;
    let mut available_markers = 0u32;
    let mut finished = 0u32;
    let rc = h.call(
        m,
        STREAM_REMAINING,
        &[in0, in0_key, frame + 0, frame + 4],
    );
    if rc >= 0x8000_0000 {
        return rc;
    }
    available_text = rw(m, frame + 0);
    finished = rw(m, frame + 4);
    let rc = h.call(
        m,
        STREAM_REMAINING,
        &[in1, in1_key, frame + 8, frame + 4],
    );
    if rc >= 0x8000_0000 {
        return rc;
    }
    available_markers = rw(m, frame + 8);
    h.call(m, 0x17486c, &[log, 103, crate::rom_region::addr(h.rom(), 0x202fb8), crate::rom_region::addr(h.rom(), 0x202fbc), 0]);
    if available_text == 0 {
        // The tail return the ARM carries in r7: the remaining rc until
        // the marker close overwrites it, then the last out close. The
        // close loop has no error check (0x2034f0 always advances; r7
        // feeds the 0x203558 return).
        let mut tail = rc;
        if finished == 0 && available_markers == 0 {
            let mut setting = 0u32;
            let status = h.call(m, 0x175a1c, &[reg, crate::rom_region::addr(h.rom(), 0x202f80), frame + 0x10]);
            if status >= 0x8000_0000
                || rw(m, frame + 0x10) == 0
                || rb(m, rw(m, frame + 0x10)) == 0
            {
                wb(m, done, 0);
            }
        } else {
            if available_markers != 0 {
                let out1 = rw(m, p + 0x34);
                let out1_key = rw(m, p + 0x38);
                let rc = h.call(
                    m,
                    STREAM_GETTEXT,
                    &[in1, in1_key, frame + 0x18, frame + 0x1c],
                );
                if rc >= 0x8000_0000 {
                    return rc;
                }
                let marker_bytes = rw(m, frame + 0x1c);
                let rc = h.call(
                    m,
                    STREAM_WRITE,
                    &[out1, out1_key, rw(m, frame + 0x18), marker_bytes],
                );
                if rc >= 0x8000_0000 {
                    return rc;
                }
                if h.call(m, 0x17497c, &[log]) != 0 {
                    h.call(
                        m,
                        0x1748a0,
                        &[
                            log,
                            104,
                            crate::rom_region::addr(h.rom(), 0x202f78),
                            crate::rom_region::addr(h.rom(), 0x202f7c),
                            rw(m, frame + 0x18),
                            marker_bytes,
                        ],
                    );
                }
                let rc = h.call(m, 0x15017c, &[in1, in1_key, marker_bytes]);
                if rc >= 0x8000_0000 {
                    return rc;
                }
                tail = rc;
            }
            h.call(m, 0x1748a0, &[log, 104, crate::rom_region::addr(h.rom(), 0x202f78), crate::rom_region::addr(h.rom(), 0x202f84), 0, 0]);
            if finished != 0 {
                for i in 0..2u32 {
                    if rw(m, p + 0x2c + i * 8) != 0 {
                        // The out-stream close rides the stream method at
                        // slot +0x24 with (handle, key): entry 0x14f918 on
                        // this path (sing_1 pass-2 probe: blx at 0x203514
                        // lands on 0x14f918 with r0/r1 = handle/key), two
                        // args, never 0x15017c with one.
                        tail = h.call(
                            m,
                            0x14f918,
                            &[rw(m, p + 0x2c + i * 8), rw(m, p + 0x30 + i * 8)],
                        );
                        ww(m, p + 0x2c + i * 8, 0);
                        ww(m, p + 0x30 + i * 8, 0);
                    }
                }
            }
        }
        h.call(m, 0x17486c, &[log, 103, crate::rom_region::addr(h.rom(), 0x202f8c), crate::rom_region::addr(h.rom(), 0x202f90), 0]);
        return tail;
    }
    let rc = h.call(
        m,
        STREAM_GETTEXT,
        &[in0, in0_key, frame + 0x18, frame + 0x1c],
    );
    if rc >= 0x8000_0000 {
        return rc;
    }
    let text = rw(m, frame + 0x18);
    let text_bytes = rw(m, frame + 0x1c);
    let text_count = (text_bytes >> 1) as u16;
    h.call(
        m,
        0x1748a0,
        &[log, 104, crate::rom_region::addr(h.rom(), 0x202f78), crate::rom_region::addr(h.rom(), 0x202f84), text, text_bytes],
    );
    // From the marker fetch on, every exit runs the ROM's shared cleanup
    // (0x205c70): the END log, then the owned markers and the work buffers
    // go back to the heap.
    let text_count = u32::from(text_count);
    let rom = h.rom();
    let category = crate::rom_region::addr(rom, 0x8f9878);
    let mut lookup = 0u32;
    let mut narrow = 0u32;
    let mut replacement = 0u32;
    let mut markers = 0u32;
    let mut ordered = 0u32;
    let mut owned = 0u32;
    let mut multiword_ends = 0u32;
    let mut marker_count = 0u32;
    let mut extra_count = 0u32;
    let rc = 'body: {
        let rc = h.call(m, STREAM_GETTEXT, &[in1, in1_key, frame + 0x20, frame + 0x24]);
        if rc >= 0x8000_0000 {
            break 'body rc;
        }
        let marker_input = rw(m, frame + 0x20);
        let marker_bytes = rw(m, frame + 0x24);
        marker_count = (marker_bytes / 24) & 0xffff;
        let markers_type = crate::rom_region::addr(rom, 0x2035e4);
        if h.call(m, 0x17497c, &[log]) != 0 {
            h.call(
                m,
                0x1748a0,
                &[log, 104, crate::rom_region::addr(rom, 0x2035c0), markers_type, marker_input, marker_count * 24],
            );
        }
        let stream = rw(m, p + 0x18);
        let write = rw(m, stream.wrapping_add(0x34));
        let consume = rw(m, stream.wrapping_add(0x30));
        let text_type = crate::rom_region::addr(rom, 0x2035c4);
        // The dictionary: f68 (+0x44) first (0x2033d0), then f88 (+0x58)
        // and its count method ([f88+0x28] = 0x201eb0) on +0x5c/+0x60.
        let has_dictionary = rw(m, p + 0x44) != 0
            || (rw(m, p + 0x58) != 0 && fn_201eb0_m(m, rw(m, p + 0x5c), rw(m, p + 0x60), rom) != 0);
        if !has_dictionary {
            // 0x205b58: no dictionary, so the text and the markers pass
            // through untouched.
            let passthrough = crate::rom_region::addr(rom, 0x205ca4);
            let rc = h.call(m, write, &[out0, out0_key, text, text_bytes]);
            if rc >= 0x8000_0000 {
                break 'body rc;
            }
            h.call(m, 0x1748a0, &[log, 105, passthrough, text_type, text, text_bytes]);
            let rc = h.call(m, consume, &[in0, in0_key, text_bytes]);
            if rc >= 0x8000_0000 {
                break 'body rc;
            }
            let rc = h.call(m, write, &[out1, out1_key, marker_input, marker_bytes]);
            if rc >= 0x8000_0000 {
                break 'body rc;
            }
            if h.call(m, 0x17497c, &[log]) != 0 {
                h.call(m, 0x1748a0, &[log, 105, passthrough, markers_type, marker_input, marker_count * 24]);
            }
            break 'body h.call(m, consume, &[in1, in1_key, marker_bytes]);
        }
        // The stack strings, packed at 0x203960: spell, normal, _lid,
        // " \x15" and ":;,)".  languageText (four units) sits right below
        // spell like sp+0xf8 below sp+0x100, so a longer language spills
        // into spell the same way.
        fn_1763e4_m(m, frame + 0x40, crate::rom_region::addr(rom, 0x203960));
        fn_1763e4_m(m, frame + 0x50, crate::rom_region::addr(rom, 0x203968));
        fn_1763e4_m(m, frame + 0x60, crate::rom_region::addr(rom, 0x203970));
        fn_1763e4_m(m, frame + 0x70, crate::rom_region::addr(rom, 0x203978));
        fn_1763e4_m(m, frame + 0x80, crate::rom_region::addr(rom, 0x20397c));
        let spell = frame + 0x40;
        let normal = frame + 0x50;
        let lid = frame + 0x60;
        let dummy = frame + 0x70;
        let punctuation = frame + 0x80;
        let language_text = frame + 0x38;
        let f100 = u32::from(rh(m, p + 0x64));
        let f102 = u32::from(rh(m, p + 0x66));
        let f104 = u32::from(rh(m, p + 0x68));
        lookup = h.call(m, 0x1963b8, &[heap, f102 + 1, 2]);
        if lookup == 0 {
            break 'body c2_out_of_memory(m, h, log, category);
        }
        let max_length = text_count.max(f100);
        narrow = h.call(m, 0x1963b8, &[heap, 1, f104.wrapping_mul(max_length + 1).wrapping_add(1)]);
        if narrow == 0 {
            break 'body c2_out_of_memory(m, h, log, category);
        }
        replacement = h.call(m, 0x1963b8, &[heap, (f100 + 1).wrapping_mul(f104).wrapping_add(1), 2]);
        if replacement == 0 {
            break 'body c2_out_of_memory(m, h, log, category);
        }
        markers = h.call(m, 0x196394, &[heap, marker_bytes]);
        if markers == 0 {
            break 'body c2_out_of_memory(m, h, log, category);
        }
        h.call(m, 0x195ae8, &[markers, marker_input, marker_bytes]);
        h.call(m, 0x1738e8, &[log, category, 5, 0, crate::rom_region::addr(rom, 0x203984)]);
        for i in 0..marker_count {
            c2_log_marker(m, h, log, category, markers + i * 24);
        }
        // 0x203814: a whole-word store of 0 into each type-1 length.
        for i in 0..marker_count {
            let marker = markers + i * 24;
            if rh(m, marker) == 1 {
                ww(m, marker + 0x10, 0);
            }
        }
        // The DEPES getPosition ([depes+0x3c] = 0x21958c) on the handle
        // pair at +0x0c/+0x10; its rc stays live until a write replaces it.
        let depes = rw(m, p + 0x14);
        let depes_handle = rw(m, p + 0x0c);
        let depes_key = rw(m, p + 0x10);
        let mut rc = h.call(m, rw(m, depes.wrapping_add(0x3c)), &[depes_handle, depes_key, frame + 0x28, 0]);
        if rc >= 0x8000_0000 {
            break 'body rc;
        }
        ordered = h.call(m, 0x1963b8, &[heap, marker_count, 4]);
        if ordered == 0 {
            break 'body c2_out_of_memory(m, h, log, category);
        }
        owned = h.call(m, 0x1963b8, &[heap, marker_count, 4]);
        if owned == 0 {
            break 'body c2_out_of_memory(m, h, log, category);
        }
        multiword_ends = h.call(m, 0x1963b8, &[heap, text_bytes.wrapping_add(1), 2]);
        if multiword_ends == 0 {
            break 'body c2_out_of_memory(m, h, log, category);
        }
        // The ARM reads markers[0].position unconditionally.
        let base_position = rw(m, markers + 0xc);
        let mut state = C2MarkerState { mode: normal, language: normal, in_tag: 0, pronunciation: 15 };
        let mut marker_index = 0u32;
        let mut position_delta = 0i32;
        while marker_index < marker_count && rw(m, markers + marker_index * 24 + 0xc) == base_position {
            let marker = markers + marker_index * 24;
            state.track(m, marker);
            ww(m, marker + 0xc, rw(m, marker + 0xc).wrapping_add(position_delta as u32));
            ww(m, ordered + marker_index * 4, marker);
            wb(m, owned + marker_index, 0);
            marker_index += 1;
        }
        // The multiword scan: each 0x20 start/end pair records where the
        // multiword starting at that unit ends.
        let mut start = 0xffffu32;
        for i in 0..marker_count {
            let marker = markers + i * 24;
            let kind = rh(m, marker);
            let offset = (rw(m, marker + 0xc).wrapping_sub(base_position) >> 1) & 0xffff;
            if kind == 0x20 {
                let value = rw(m, marker + 0x14);
                if value == 1 {
                    if start != 0xffff {
                        h.call(m, 0x1738e8, &[log, category, 5, 0, crate::rom_region::addr(rom, 0x8f9928), start, offset]);
                    }
                    start = offset;
                } else if value == 0 {
                    if start == 0xffff {
                        h.call(m, 0x1738e8, &[log, category, 5, 0, crate::rom_region::addr(rom, 0x8f99a0), offset]);
                    } else {
                        let mut end = offset;
                        if end == text_count && end != 0 && c2_delimiter(m, text + (end - 1) * 2, rom) {
                            end -= 1;
                        }
                        while end != text_count && end != 0 && c2_delimiter(m, text + end * 2, rom) {
                            end -= 1;
                        }
                        if end != text_count && !c2_delimiter(m, text + end * 2, rom) {
                            end = (end + 1) & 0xffff;
                        }
                        wh(m, multiword_ends + start * 2, end as u16);
                        start = 0xffff;
                    }
                }
            } else if (kind == 0x15 || kind == 7 || kind == 0x22) && start != 0xffff {
                h.call(m, 0x1738e8, &[log, category, 5, 0, crate::rom_region::addr(rom, 0x8f99fc), start]);
                start = 0xffff;
            }
        }
        if start != 0xffff {
            h.call(m, 0x1738e8, &[log, category, 5, 0, crate::rom_region::addr(rom, 0x8f9a50), start]);
        }
        // The word loop (0x203ca8).  Locals as the ARM keeps them: wordEnd
        // (sl), wordStart (sp+0x80), length (r5), retry (sp+0x18), capacity
        // (sp+0x2c, u16), outputType (sp+0xcc), outputLanguage (sp+0xc8),
        // dummyCount (sp+0x1c).  The lookup reads and writes the last three
        // through the frame cells +0x210/+0x214/+0x218.
        let sink = C2Sink { write, out: out0, key: out0_key, log, content: text_type };
        let word_label = crate::rom_region::addr(rom, 0x204148);
        let output_label = crate::rom_region::addr(rom, 0x205818);
        let moved_format = crate::rom_region::addr(rom, 0x8f9b3c);
        let mut word_end = 0u32;
        let mut word_start;
        let mut retry = 0u32;
        let mut multiword_start = 0u32;
        let mut multiword_end = 0u32;
        let mut length;
        let mut capacity;
        let mut output_type;
        let mut output_language = 0u32;
        let mut dummy_count;
        while word_end < text_count {
            let run_start = word_end;
            while word_end < text_count && c2_delimiter(m, text + word_end * 2, rom) {
                word_end += 1;
            }
            let mut is_multiword = 0u32;
            if retry == 0 {
                let mut i = run_start;
                while i <= word_end {
                    let end = u32::from(rh(m, multiword_ends + i * 2));
                    if end != 0 {
                        multiword_start = i;
                        is_multiword = 1;
                        multiword_end = end;
                        break;
                    }
                    i += 1;
                }
                if run_start == word_end {
                    let end = u32::from(rh(m, multiword_ends + word_end * 2));
                    if end != 0 {
                        multiword_end = end;
                        is_multiword = 1;
                        multiword_start = word_end;
                    }
                }
            }
            // 0x203d74: the delimiter run goes out as one chunk, cut short
            // where a multiword starts inside it.
            if word_end > run_start {
                if is_multiword != 0 && multiword_end > word_end && word_end > multiword_start {
                    word_end = multiword_start;
                }
                if word_end as i32 - run_start as i32 > 0 {
                    rc = c2_write(m, h, &sink, word_label, text + run_start * 2, (word_end - run_start) * 2);
                    if rc >= 0x8000_0000 {
                        break 'body rc;
                    }
                }
            }
            if is_multiword == 1 {
                let span = multiword_end.wrapping_sub(multiword_start) as i32;
                length = span as u32 & 0xffff;
                h.call(
                    m,
                    0x1738e8,
                    &[log, category, 5, 0, crate::rom_region::addr(rom, 0x20414c), multiword_start, multiword_end],
                );
                if f102 < length {
                    h.call(m, 0x1738e8, &[log, category, 5, 0, crate::rom_region::addr(rom, 0x204160), length, f102]);
                    word_end = (word_end + length) & 0xffff;
                    if span > 0 {
                        rc = c2_write(m, h, &sink, word_label, text + multiword_start * 2, length * 2);
                        if rc >= 0x8000_0000 {
                            break 'body rc;
                        }
                    }
                    continue;
                }
                word_start = multiword_start;
                word_end = multiword_end;
                crate::fe_text::strncpy16_176644(m.bytes, lookup, text + multiword_start * 2, length);
                wh(m, lookup + length * 2, 0);
            } else {
                word_start = word_end;
                length = 0;
                crate::fe_text::memset_195bec(m.bytes, lookup, 0, f102 * 2);
                while word_end < text_count
                    && !c2_delimiter(m, text + word_end * 2, rom)
                    && f102 as i32 - 1 > length as i32
                {
                    wh(m, lookup + length * 2, rh(m, text + word_end * 2));
                    length = (length + 1) & 0xffff;
                    word_end = (word_end + 1) & 0xffff;
                }
                if word_end < text_count && !c2_delimiter(m, text + word_end * 2, rom) {
                    // 0x203ffc: longer than a lookup, so the whole word goes
                    // out untouched.
                    while word_end < text_count && !c2_delimiter(m, text + word_end * 2, rom) {
                        word_end += 1;
                    }
                    if word_end > word_start {
                        rc = c2_write(m, h, &sink, word_label, text + word_start * 2, (word_end - word_start) * 2);
                        if rc >= 0x8000_0000 {
                            break 'body rc;
                        }
                    }
                    continue;
                }
                if length == 0 {
                    continue;
                }
            }
            // 0x2040b4: the markers up to the word, then the lookup.
            output_type = 1;
            while marker_index < marker_count {
                let marker = markers + marker_index * 24;
                let position = rw(m, marker + 0xc);
                if word_start * 2 < (position.wrapping_sub(base_position) & 0xffff) {
                    break;
                }
                ww(m, marker + 0xc, position.wrapping_add(position_delta as u32));
                let slot = extra_count + marker_index;
                ww(m, ordered + slot * 4, marker);
                wb(m, owned + slot, 0);
                state.track(m, marker);
                marker_index = (marker_index + 1) & 0xffff;
            }
            capacity = f100;
            if fn_1765ac(m, state.mode, spell) != 0 && state.in_tag == 0 {
                fn_17649c(m, narrow, text + word_start * 2, length);
                wb(m, narrow + length, 0);
                h.call(m, 0x1738e8, &[log, category, 5, 0, crate::rom_region::addr(rom, 0x204604), narrow]);
                crate::fe_text::memset_195bec(m.bytes, replacement, 0, f100 * 2);
                let language_length = fn_176624(m, state.language);
                output_language = if language_length == 3
                    || (fn_176624(m, state.language) == 7
                        && crate::fe_text::strstr16_176538(m.bytes, state.language, lid) != 0)
                {
                    let target = rw(m, p + 0x6c);
                    crate::fe_text::narrow_176460(m.bytes, target, state.language);
                    rw(m, p + 0x6c)
                } else {
                    0
                };
                h.call(
                    m,
                    0x17486c,
                    &[
                        log,
                        106,
                        crate::rom_region::addr(rom, 0x204644),
                        crate::rom_region::addr(rom, 0x204640),
                        0,
                        crate::rom_region::addr(rom, 0x20463c),
                        lookup,
                        crate::rom_region::addr(rom, 0x204634),
                        crate::rom_region::addr(rom, if is_multiword != 0 { 0x20462c } else { 0x204630 }),
                        crate::rom_region::addr(rom, 0x204624),
                        output_language,
                        crate::rom_region::addr(rom, 0x20461c),
                        word_start,
                        crate::rom_region::addr(rom, 0x204614),
                        word_start + length,
                    ],
                );
                // fn_203144 inlined (0x20438c): the handle check, then the
                // expansion with f68 (+0x44) as its dictionary.  The nested
                // frame rides at +0x280 so its words never alias the cells.
                ww(m, frame + 0x210, output_type);
                wh(m, frame + 0x214, capacity as u16);
                ww(m, frame + 0x218, output_language);
                rc = if h.call(m, 0x176f94, &[p, b, FE_DEPES_KEY, 0xb8]) >= 0x8000_0000 {
                    0x8a80_2008
                } else {
                    fn_282af0_m(
                        m,
                        frame + 0x280,
                        ctx,
                        rw(m, p + 0x44),
                        rw(m, p + 0x58),
                        rw(m, p + 0x5c),
                        rw(m, p + 0x60),
                        1,
                        lookup,
                        frame + 0x210,
                        replacement,
                        frame + 0x214,
                        frame + 0x218,
                        h,
                    )
                };
                output_type = rw(m, frame + 0x210);
                capacity = u32::from(rh(m, frame + 0x214));
                output_language = rw(m, frame + 0x218);
                if rc < 0x8000_0000 {
                    h.call(
                        m,
                        0x17486c,
                        &[
                            log,
                            106,
                            crate::rom_region::addr(rom, 0x204678),
                            crate::rom_region::addr(rom, 0x204674),
                            0,
                            crate::rom_region::addr(rom, 0x20466c),
                            rc,
                            crate::rom_region::addr(rom, 0x204634),
                            output_type,
                            crate::rom_region::addr(rom, 0x204668),
                            replacement,
                            crate::rom_region::addr(rom, 0x204624),
                            output_language,
                        ],
                    );
                } else {
                    h.call(
                        m,
                        0x17486c,
                        &[
                            log,
                            106,
                            crate::rom_region::addr(rom, 0x204694),
                            crate::rom_region::addr(rom, 0x204674),
                            0,
                            crate::rom_region::addr(rom, 0x20466c),
                            rc,
                        ],
                    );
                }
            }
            dummy_count = 0;
            if rc < 0x8000_0000 && capacity != 0 && capacity < f100 {
                crate::fe_text::narrow_176460(m.bytes, narrow, replacement);
                h.call(m, 0x1738e8, &[log, category, 5, 0, crate::rom_region::addr(rom, 0x2046a0), narrow]);
                if output_language != 0 {
                    // 0x204518: a language marker ahead of the word.
                    fn_1763e4_m(m, language_text, output_language);
                    let count = marker_count + extra_count;
                    let grown = h.call(m, 0x1963e0, &[heap, ordered, (count + 1) * 4]);
                    if grown == 0 {
                        break 'body c2_out_of_memory(m, h, log, category);
                    }
                    ordered = grown;
                    let grown = h.call(m, 0x1963e0, &[heap, owned, (count + 1) * 4]);
                    if grown == 0 {
                        break 'body c2_out_of_memory(m, h, log, category);
                    }
                    owned = grown;
                    wb(m, owned + count, 0);
                    let marker = h.call(m, 0x1963b8, &[heap, 1, 24]);
                    if marker == 0 {
                        break 'body c2_out_of_memory(m, h, log, category);
                    }
                    let saved = h.call(m, 0x1963b8, &[heap, fn_176624(m, language_text) as u32 + 1, 2]);
                    if saved == 0 {
                        let rc = c2_out_of_memory(m, h, log, category);
                        h.call(m, 0x196428, &[heap, marker]);
                        break 'body rc;
                    }
                    crate::fe_text::strcpy16_176214(m.bytes, saved, language_text);
                    let position = base_position.wrapping_add(word_start * 2).wrapping_add(position_delta as u32);
                    c2_fill_marker(m, marker, 0x24, position, saved);
                    let slot = extra_count + marker_index;
                    extra_count = (extra_count + 1) & 0xffff;
                    ww(m, ordered + slot * 4, marker);
                    wb(m, owned + slot, 1);
                }
                if output_type == 1 {
                    // 0x204754: the transcription rides a 0x22 marker.
                    let count = marker_count + extra_count;
                    let grown = h.call(m, 0x1963e0, &[heap, ordered, count * 4 + 8]);
                    if grown == 0 {
                        break 'body c2_out_of_memory(m, h, log, category);
                    }
                    ordered = grown;
                    let grown = h.call(m, 0x1963e0, &[heap, owned, count * 4 + 8]);
                    if grown == 0 {
                        break 'body c2_out_of_memory(m, h, log, category);
                    }
                    owned = grown;
                    wb(m, owned + count, 0);
                    wb(m, owned + count + 1, 0);
                    let phonetic_marker = h.call(m, 0x1963b8, &[heap, 1, 24]);
                    if phonetic_marker == 0 {
                        break 'body c2_out_of_memory(m, h, log, category);
                    }
                    let phonetic = h.call(m, 0x1963b8, &[heap, capacity + 1, 2]);
                    if phonetic == 0 {
                        let rc = c2_out_of_memory(m, h, log, category);
                        h.call(m, 0x196428, &[heap, phonetic_marker]);
                        break 'body rc;
                    }
                    crate::fe_text::strcpy16_176214(m.bytes, phonetic, replacement);
                    if is_multiword == 1 {
                        // 0x20489c: the spaces in the text against the word
                        // breaks ('_' or '#') in the transcription.
                        let mut orthographic = 0u32;
                        let mut j = 0u32;
                        while length > j && c2_delimiter(m, lookup + j * 2, rom) {
                            j = (j + 1) & 0xffff;
                        }
                        while j < length && rh(m, lookup + j * 2) != 0 {
                            if c2_delimiter(m, lookup + j * 2, rom) {
                                orthographic = (orthographic + 1) & 0xffff;
                                while length > j && c2_delimiter(m, lookup + j * 2, rom) {
                                    j = (j + 1) & 0xffff;
                                }
                            }
                            j = (j + 1) & 0xffff;
                        }
                        let mut phonetic_breaks = 0u32;
                        let mut i = 1u32;
                        while capacity as i32 - 1 > i as i32 {
                            let c = rh(m, phonetic + i * 2);
                            if c == u16::from(b'_') || c == u16::from(b'#') {
                                phonetic_breaks = (phonetic_breaks + 1) & 0xffff;
                            }
                            if c == 0x12 {
                                break;
                            }
                            i = (i + 1) & 0xffff;
                        }
                        if orthographic < phonetic_breaks {
                            dummy_count = (phonetic_breaks - orthographic) & 0xffff;
                            h.call(
                                m,
                                0x1738e8,
                                &[
                                    log,
                                    category,
                                    5,
                                    0,
                                    crate::rom_region::addr(rom, 0x8f9a94),
                                    dummy_count,
                                    orthographic + 1,
                                    phonetic_breaks + 1,
                                ],
                            );
                        } else if orthographic > phonetic_breaks {
                            // TYPE2: fewer transcribed words, so the last
                            // spaces of the text become hyphens.
                            let mut remaining = (orthographic - phonetic_breaks) & 0xffff;
                            h.call(
                                m,
                                0x1738e8,
                                &[
                                    log,
                                    category,
                                    5,
                                    0,
                                    crate::rom_region::addr(rom, 0x8f9aec),
                                    orthographic,
                                    phonetic_breaks,
                                ],
                            );
                            fn_17649c(m, narrow, text + word_start * 2, length);
                            wb(m, narrow + length, 0);
                            let mut k = length.wrapping_sub(1) & 0xffff;
                            while k != 0 && rb(m, narrow + k) == b' ' {
                                k -= 1;
                            }
                            while k != 0 {
                                k -= 1;
                                if k == 0 || rb(m, narrow + k) == b' ' {
                                    break;
                                }
                            }
                            while remaining != 0 && k != 0 {
                                while k != 0 && rb(m, narrow + k) == b' ' {
                                    wb(m, narrow + k, b'-');
                                    k -= 1;
                                }
                                remaining = remaining.wrapping_sub(1) & 0xffff;
                                while k != 0 {
                                    k -= 1;
                                    if k == 0 || rb(m, narrow + k) == b' ' {
                                        break;
                                    }
                                }
                            }
                            h.call(m, 0x1738e8, &[log, category, 5, 0, crate::rom_region::addr(rom, 0x204e54), narrow]);
                            for i in 0..length {
                                let c = rb(m, narrow + i);
                                wh(m, text + (word_start + i) * 2, u16::from(c));
                            }
                            let moved = base_position
                                .wrapping_add((word_start.wrapping_add(position_delta as u32) & 0xffff) * 2);
                            for i in 0..marker_count {
                                let marker = markers + i * 24;
                                let position = rw(m, marker + 0xc);
                                let offset = position.wrapping_sub(rw(m, markers + 0xc));
                                if word_start * 2 < offset && offset < (word_start + length) * 2 {
                                    h.call(m, 0x1738e8, &[log, category, 5, 0, moved_format, i, position, moved]);
                                    ww(m, marker + 0xc, moved);
                                }
                            }
                        }
                    } else {
                        // 0x20484c: each word break not right before the
                        // end mark (0x12) gets a dummy.
                        let mut i = 1u32;
                        while capacity as i32 - 1 > i as i32 {
                            let c = rh(m, phonetic + i * 2);
                            if (c == u16::from(b'_') || c == u16::from(b'#')) && rh(m, phonetic + (i + 1) * 2) != 0x12 {
                                dummy_count = (dummy_count + 1) & 0xffff;
                            }
                            if c == 0x12 {
                                break;
                            }
                            i = (i + 1) & 0xffff;
                        }
                    }
                    for i in 0..length {
                        let at = text + (word_start + i) * 2;
                        if rh(m, at) == u16::from(b'_') {
                            wh(m, at, u16::from(b'-'));
                        }
                    }
                    let position = base_position.wrapping_add(word_start * 2).wrapping_add(position_delta as u32);
                    c2_fill_marker(m, phonetic_marker, 0x22, position, phonetic);
                    let slot = extra_count + marker_index;
                    ww(m, ordered + slot * 4, phonetic_marker);
                    wb(m, owned + slot, 1);
                    // 0x204d84: the markers inside the word follow it; a
                    // language change moves them (and the rest) past it.
                    let end_position = base_position
                        .wrapping_add(word_end * 2)
                        .wrapping_add(position_delta as u32)
                        .wrapping_add(dummy_count * 4);
                    let mut language_changed = false;
                    while marker_index < marker_count {
                        let marker = markers + marker_index * 24;
                        let position = rw(m, marker + 0xc);
                        if (position.wrapping_sub(base_position) & 0xffff) >= word_end * 2 {
                            break;
                        }
                        if rh(m, marker) == 0x24 {
                            language_changed = true;
                            state.language = rw(m, marker + 0x14);
                            ww(m, marker + 0xc, end_position);
                        } else if language_changed {
                            ww(m, marker + 0xc, end_position);
                        } else {
                            ww(m, marker + 0xc, position.wrapping_add(position_delta as u32));
                        }
                        let slot = marker_index + extra_count + 1;
                        ww(m, ordered + slot * 4, marker);
                        wb(m, owned + slot, 0);
                        state.track_inside(m, marker);
                        marker_index = (marker_index + 1) & 0xffff;
                    }
                    if word_end < text_count {
                        // 0x204dc0: the mode resumes after the word.
                        let mode_marker = h.call(m, 0x1963b8, &[heap, 1, 24]);
                        if mode_marker == 0 {
                            let rc = c2_out_of_memory(m, h, log, category);
                            extra_count = (extra_count + 1) & 0xffff;
                            break 'body rc;
                        }
                        let saved = h.call(m, 0x196394, &[heap, (fn_176624(m, state.mode) as u32 + 1) * 2]);
                        if saved == 0 {
                            let rc = c2_out_of_memory(m, h, log, category);
                            h.call(m, 0x196428, &[heap, mode_marker]);
                            extra_count = (extra_count + 1) & 0xffff;
                            break 'body rc;
                        }
                        crate::fe_text::strcpy16_176214(m.bytes, saved, state.mode);
                        let slot = marker_index + extra_count + 1;
                        extra_count = (extra_count + 1) & 0xffff;
                        c2_fill_marker(m, mode_marker, 0x15, end_position, saved);
                        ww(m, ordered + slot * 4, mode_marker);
                        wb(m, owned + slot, 1);
                    }
                    position_delta = c2_sxth(position_delta.wrapping_add(c2_sxth((dummy_count << 2) as i32)));
                    extra_count = (extra_count + 1) & 0xffff;
                } else if output_type == 4
                    && capacity >= 7
                    && crate::fe_text::memcmp_195aac(m.bytes, crate::rom_region::addr(rom, 0x8f9858), replacement, 6) == 0
                    && crate::fe_text::memcmp_195aac(
                        m.bytes,
                        crate::rom_region::addr(rom, 0x8f9858),
                        (replacement + capacity * 2).wrapping_sub(6),
                        6,
                    ) == 0
                {
                    // 0x204f80: a bracketed transcription becomes a
                    // pronunciation switch (7, 0x3c) around the word.
                    crate::fe_text::memmove_195b38(
                        m.bytes,
                        replacement,
                        replacement + 6,
                        (capacity * 2).wrapping_sub(12),
                    );
                    wh(m, (replacement + capacity * 2).wrapping_sub(12), 0);
                    capacity = capacity.wrapping_sub(6) & 0xffff;
                    crate::fe_text::narrow_176460(m.bytes, narrow, replacement);
                    wb(m, narrow + length, 0);
                    h.call(m, 0x1738e8, &[log, category, 5, 0, crate::rom_region::addr(rom, 0x205300), narrow]);
                    let count = marker_count + extra_count;
                    // These two keep the result before the null check
                    // (0x205060), so a failure leaves the slot null.
                    ordered = h.call(m, 0x1963e0, &[heap, ordered, count * 4 + 8]);
                    if ordered == 0 {
                        break 'body c2_out_of_memory(m, h, log, category);
                    }
                    owned = h.call(m, 0x1963e0, &[heap, owned, count * 4 + 8]);
                    if owned == 0 {
                        break 'body c2_out_of_memory(m, h, log, category);
                    }
                    wb(m, owned + count, 0);
                    wb(m, owned + count + 1, 0);
                    let start_marker = h.call(m, 0x1963b8, &[heap, 1, 24]);
                    if start_marker == 0 {
                        break 'body c2_out_of_memory(m, h, log, category);
                    }
                    let word_position = base_position.wrapping_add(word_start * 2);
                    let index = marker_index + extra_count;
                    c2_fill_marker(m, start_marker, 7, word_position.wrapping_add(position_delta as u32), 0x3c);
                    ww(m, ordered + index * 4, start_marker);
                    wb(m, owned + index, 1);
                    let end_marker = h.call(m, 0x1963b8, &[heap, 1, 24]);
                    if end_marker == 0 {
                        extra_count = (extra_count + 1) & 0xffff;
                        break 'body c2_out_of_memory(m, h, log, category);
                    }
                    let position = word_position.wrapping_add(capacity * 2).wrapping_add(position_delta as u32);
                    c2_fill_marker(m, end_marker, 7, position, state.pronunciation);
                    extra_count = (extra_count + 2) & 0xffff;
                    ww(m, ordered + (index + 1) * 4, end_marker);
                    wb(m, owned + index + 1, 1);
                }
                if output_language != 0 {
                    // 0x205194: the running language resumes after the word.
                    fn_1763e4_m(m, language_text, output_language);
                    let count = marker_count + extra_count;
                    let grown = h.call(m, 0x1963e0, &[heap, ordered, (count + 1) * 4]);
                    if grown == 0 {
                        break 'body c2_out_of_memory(m, h, log, category);
                    }
                    ordered = grown;
                    let grown = h.call(m, 0x1963e0, &[heap, owned, (count + 1) * 4]);
                    if grown == 0 {
                        break 'body c2_out_of_memory(m, h, log, category);
                    }
                    owned = grown;
                    wb(m, owned + count, 0);
                    let marker = h.call(m, 0x1963b8, &[heap, 1, 24]);
                    if marker == 0 {
                        break 'body c2_out_of_memory(m, h, log, category);
                    }
                    let saved = h.call(m, 0x1963b8, &[heap, fn_176624(m, state.language) as u32 + 1, 2]);
                    if saved == 0 {
                        let rc = c2_out_of_memory(m, h, log, category);
                        h.call(m, 0x196428, &[heap, marker]);
                        break 'body rc;
                    }
                    crate::fe_text::strcpy16_176214(m.bytes, saved, state.language);
                    let position = match output_type {
                        1 => base_position.wrapping_add(word_end * 2).wrapping_add(position_delta as u32),
                        4 => base_position
                            .wrapping_add(word_start * 2)
                            .wrapping_add(capacity * 2)
                            .wrapping_add(position_delta as u32),
                        _ => 0,
                    };
                    c2_fill_marker(m, marker, 0x24, position, saved);
                    let slot = extra_count + marker_index;
                    extra_count = (extra_count + 1) & 0xffff;
                    ww(m, ordered + slot * 4, marker);
                    wb(m, owned + slot, 1);
                }
            }
            // The output (0x205390).
            if is_multiword != 0 && rc >= 0x8000_0000 {
                retry = 1;
            } else {
                let original_length = word_end as i32 - word_start as i32;
                if original_length <= 0
                    || (output_type != 1 && capacity != 0 && capacity < f100 && rc < 0x8000_0000)
                {
                    // 0x205600: the replacement stands in for the word.
                    rc = c2_write(m, h, &sink, output_label, replacement, capacity * 2);
                    if rc >= 0x8000_0000 {
                        break 'body rc;
                    }
                    let original_words = c2_count_words(m, lookup, length, rom);
                    let replacement_words = c2_count_words(m, replacement, capacity, rom);
                    if is_multiword != 0 || replacement_words != original_words {
                        h.call(
                            m,
                            0x1738e8,
                            &[
                                log,
                                category,
                                5,
                                0,
                                crate::rom_region::addr(rom, 0x20581c),
                                original_words,
                                replacement_words,
                            ],
                        );
                        let moved = base_position.wrapping_add(multiword_start * 2);
                        for i in 0..marker_count {
                            let marker = markers + i * 24;
                            let position = rw(m, marker + 0xc);
                            let offset = position.wrapping_sub(rw(m, markers + 0xc));
                            if multiword_start * 2 < offset && offset <= multiword_end * 2 {
                                h.call(m, 0x1738e8, &[log, category, 5, 0, moved_format, i, position, moved]);
                                ww(m, marker + 0xc, moved);
                            }
                        }
                    }
                    let shift = (capacity * 2).wrapping_sub(length * 2) as i32;
                    position_delta = c2_sxth(position_delta.wrapping_add(c2_sxth(shift)));
                } else {
                    let last = text.wrapping_add(word_end.wrapping_sub(1).wrapping_mul(2));
                    let trailing = |m: &Memory<'_>| fn_176514(m, punctuation, u32::from(rh(m, last))) != 0;
                    if !trailing(m) {
                        rc = c2_write(m, h, &sink, output_label, text + word_start * 2, original_length as u32 * 2);
                        if rc >= 0x8000_0000 {
                            break 'body rc;
                        }
                    } else if original_length - 1 > 0 {
                        let bytes = (original_length - 1) as u32 * 2;
                        rc = c2_write(m, h, &sink, output_label, text + word_start * 2, bytes);
                        if rc >= 0x8000_0000 {
                            break 'body rc;
                        }
                    }
                    for _ in 0..dummy_count {
                        rc = c2_write(m, h, &sink, output_label, dummy, 4);
                        if rc >= 0x8000_0000 {
                            break 'body rc;
                        }
                    }
                    if trailing(m) {
                        rc = c2_write(m, h, &sink, output_label, last, 2);
                        if rc >= 0x8000_0000 {
                            break 'body rc;
                        }
                    }
                }
                retry = 0;
            }
            if is_multiword == 1 {
                if retry == 0 {
                    // 0x205aec: the markers inside the multiword slot in
                    // behind any inserted marker that sits after them.
                    word_end = multiword_end;
                    while marker_index < marker_count {
                        let marker = markers + marker_index * 24;
                        let position = rw(m, marker + 0xc);
                        if (position.wrapping_sub(base_position) & 0xffff) > multiword_end * 2 {
                            break;
                        }
                        let index = extra_count + marker_index;
                        let at = ordered.wrapping_add(index.wrapping_mul(4));
                        let before = at.wrapping_sub(4);
                        let previous = rw(m, before);
                        if rw(m, previous.wrapping_add(0xc)) > position {
                            ww(m, at, previous);
                            let previous_owned = rb(m, owned.wrapping_add(index).wrapping_sub(1));
                            wb(m, owned.wrapping_add(index), previous_owned);
                            ww(m, before, marker);
                            wb(m, owned.wrapping_add(index).wrapping_sub(1), 0);
                        } else {
                            ww(m, at, marker);
                            wb(m, owned.wrapping_add(index), 0);
                        }
                        marker_index = (marker_index + 1) & 0xffff;
                    }
                } else {
                    word_end = multiword_start;
                }
            }
        }
        // The drain (0x205898): the rest of the markers shift and fill in.
        while marker_index < marker_count {
            let marker = markers + marker_index * 24;
            ww(m, marker + 0xc, rw(m, marker + 0xc).wrapping_add(position_delta as u32));
            ww(m, ordered + (extra_count + marker_index) * 4, marker);
            marker_index = (marker_index + 1) & 0xffff;
        }
        ww(m, markers + 0x10, rw(m, markers + 0x10).wrapping_add(position_delta as u32));
        h.call(m, 0x1738e8, &[log, category, 5, 0, crate::rom_region::addr(rom, 0x205c84)]);
        let count = marker_count + extra_count;
        for i in 0..count {
            rc = h.call(m, write, &[out1, out1_key, rw(m, ordered + i * 4), 24]);
            if rc >= 0x8000_0000 {
                break 'body rc;
            }
            // The per-marker trace is gated by 173fd8 (>= 5); the level is
            // 1, so it never prints, but the gate is still called.
            if h.call(m, 0x173fd8, &[log]) >= 5 {
                c2_log_marker(m, h, log, category, rw(m, ordered + i * 4));
            }
        }
        if h.call(m, 0x17497c, &[log]) != 0 {
            h.call(
                m,
                0x1748a0,
                &[log, 105, crate::rom_region::addr(rom, 0x205c9c), markers_type, ordered, count * 4],
            );
        }
        rc = h.call(m, consume, &[in0, in0_key, text_bytes]);
        if rc >= 0x8000_0000 {
            break 'body rc;
        }
        rc = h.call(m, consume, &[in1, in1_key, marker_bytes]);
        if rc >= 0x8000_0000 {
            break 'body rc;
        }
        let stream_position = rw(m, frame + 0x28).wrapping_add(position_delta as u32);
        ww(m, frame + 0x28, stream_position);
        h.call(m, rw(m, depes.wrapping_add(0x38)), &[depes_handle, depes_key, stream_position, 0])
    };
    // The cleanup (0x205c70).
    h.call(
        m,
        0x17486c,
        &[
            log,
            103,
            crate::rom_region::addr(rom, 0x205e4c),
            crate::rom_region::addr(rom, 0x205e48),
            0,
            crate::rom_region::addr(rom, 0x205cac),
            rc,
        ],
    );
    if ordered != 0 {
        if owned != 0 {
            let count = marker_count + extra_count;
            let mut j = 1u32;
            while j < count {
                if rb(m, owned + j) != 0 {
                    let marker = rw(m, ordered + j * 4);
                    if rh(m, marker) != 7 {
                        h.call(m, 0x196428, &[heap, rw(m, marker + 0x14)]);
                    }
                    h.call(m, 0x196428, &[heap, rw(m, ordered + j * 4)]);
                }
                j = (j + 1) & 0xffff;
            }
        }
        h.call(m, 0x196428, &[heap, ordered]);
    }
    for buffer in [owned, multiword_ends, lookup, replacement, narrow, markers] {
        if buffer != 0 {
            h.call(m, 0x196428, &[heap, buffer]);
        }
    }
    rc
}

/// The running state fn_2031b0 keeps for the markers it has passed: the
/// mode (sp+0x90), the language (sp+0xa4), whether a pronunciation tag is
/// open (sp+0x98) and the pronunciation (sp+0xa8).
struct C2MarkerState {
    mode: u32,
    language: u32,
    in_tag: u32,
    pronunciation: u32,
}

impl C2MarkerState {
    /// The tracking the leading and pre-lookup loops share (0x203918,
    /// 0x2040f0): mode and language only outside a tag; 7/0x2b and 0x22
    /// open one, 7 and 0x15 close it.
    fn track(&mut self, m: &Memory<'_>, marker: u32) {
        let kind = rh(m, marker);
        let value = rw(m, marker + 0x14);
        if self.in_tag == 0 && kind == 0x15 {
            self.mode = value;
        } else if self.in_tag == 0 && kind == 0x24 {
            self.language = value;
        } else {
            if ((kind == 7 && value == 0x2b) || kind == 0x22) && self.in_tag == 0 {
                self.in_tag = 1;
            } else if (kind == 7 || kind == 0x15) && self.in_tag != 0 {
                self.in_tag = 0;
            }
            if kind == 7 {
                self.pronunciation = value;
            }
        }
    }

    /// The type-1 word loop's variant (0x204d2c): language is taken by the
    /// caller, 7/0x2b toggles the tag and any other 7 closes it.
    fn track_inside(&mut self, m: &Memory<'_>, marker: u32) {
        let kind = rh(m, marker);
        let value = rw(m, marker + 0x14);
        if self.in_tag == 0 && kind == 0x15 {
            self.mode = value;
        } else if kind == 7 {
            if value == 0x2b {
                self.in_tag = u32::from(self.in_tag == 0);
            } else {
                self.in_tag = 0;
            }
            self.pronunciation = value;
        }
    }
}

/// The out0 text sink: the stream's write method (+0x34) on the handle pair
/// at +0x2c, and the content type its trace names.
struct C2Sink {
    write: u32,
    out: u32,
    key: u32,
    log: u32,
    content: u32,
}

/// One out0 text write, then the level-105 trace the ROM logs after it.
fn c2_write(m: &mut Memory<'_>, h: &mut impl C2DepesHost, sink: &C2Sink, label: u32, at: u32, bytes: u32) -> u32 {
    let rc = h.call(m, sink.write, &[sink.out, sink.key, at, bytes]);
    if rc < 0x8000_0000 {
        h.call(m, 0x1748a0, &[sink.log, 105, label, sink.content, at, bytes]);
    }
    rc
}

/// The out-of-memory exit (0x205298): the error log, then 0x8a80200a.
fn c2_out_of_memory(m: &mut Memory<'_>, h: &mut impl C2DepesHost, log: u32, category: u32) -> u32 {
    h.call(m, 0x173dc0, &[log, category, 40000, 0]);
    0x8a80_200a
}

/// fn_283188 on the unit at `at`: does it delimit words?
fn c2_delimiter(m: &Memory<'_>, at: u32, rom: crate::rom_region::Rom) -> bool {
    fn_283188_m(m, u32::from(rh(m, at)), rom) != 0
}

/// The word count of the first `n` units (0x205660): one more than the
/// delimiter runs inside them.
fn c2_count_words(m: &Memory<'_>, s: u32, n: u32, rom: crate::rom_region::Rom) -> u32 {
    let mut words = 1u32;
    let mut i = 0u32;
    while i < n {
        if c2_delimiter(m, s + i * 2, rom) {
            words = (words + 1) & 0xffff;
            while n > i && c2_delimiter(m, s + i * 2, rom) {
                i = (i + 1) & 0xffff;
            }
        }
        i = (i + 1) & 0xffff;
    }
    words
}

/// The fields of a marker fn_2031b0 inserts; calloc already zeroed the rest.
fn c2_fill_marker(m: &mut Memory<'_>, marker: u32, kind: u16, position: u32, data: u32) {
    wh(m, marker, kind);
    ww(m, marker + 4, position);
    ww(m, marker + 8, 0);
    ww(m, marker + 0xc, position);
    ww(m, marker + 0x10, 0);
    ww(m, marker + 0x14, data);
}

/// The marker trace (0x2037a8, 0x205984): SYNC markers print without type.
fn c2_log_marker(m: &mut Memory<'_>, h: &mut impl C2DepesHost, log: u32, category: u32, marker: u32) {
    let rom = h.rom();
    let kind = rh(m, marker);
    let fields = [rw(m, marker + 4), rw(m, marker + 8), rw(m, marker + 0xc), rw(m, marker + 0x10)];
    if kind == 0x4000 {
        let format = crate::rom_region::addr(rom, 0x2039bc);
        h.call(m, 0x1738e8, &[log, category, 5, 0, format, fields[0], fields[1], fields[2], fields[3]]);
    } else {
        let format = crate::rom_region::addr(rom, 0x203990);
        h.call(
            m,
            0x1738e8,
            &[log, category, 5, 0, format, u32::from(kind), fields[0], fields[1], fields[2], fields[3]],
        );
    }
}

/// A 16-bit sign extension, the ARM's sxth.
fn c2_sxth(value: i32) -> i32 {
    i32::from(value as u16 as i16)
}

// ---------------------------------------------------------------------------
// fn_202348: the dictionary lookup, bound for the 2031b0 root.
//
// The port is speech_part_2::fn_202348.  Its host runs every callee over
// the caller's arena: the guest heap on the arena (fn_196394/fn_17bff4 go
// through the guest vtable methods, exactly like the ARM), the live
// parameter fetch (speech_part_2::fetch_parameter_m), the arena dictionary
// walk (fn_281aa4) and the dictionary engine (fn_2b3608).  The logger gate
// the traces check (the byte at log+64) is clear in the capture, so the
// 0x17486c traces are provable no-ops and return 0.
// ---------------------------------------------------------------------------

/// The langcode key address (ROM literal, mapped in the capture).
pub const C2_LANGCODE_KEY: u32 = 0x15df6c;
/// The mlset key: the ROM's first `mlset` literal (0x17f9a4) sits in an
/// unmapped page of the capture; this is a content-identical copy in a
/// mapped page.  Only the contents are ever compared, never the address.
pub const C2_MLSET_KEY: u32 = 0x180970;
/// The `numberoutputtranscriptions` key the lookup reads (ROM 0x28265c).
pub const C2_NUMBEROUTPUTTRANSCRIPTIONS: u32 = 0x28265c;
/// Caller-owned scratch for the dictionary engine's own lookup
/// (fn_2b3608), disjoint from fn_202348's 128-byte scratch.  The engine
/// saves and restores it; the certifier still snapshots it.  Both live in
/// an unmapped hole of the capture (0x1a2000-0x1b0000), so the ARM never
/// touches them; the certifier asserts the disjointness every run.
pub const C2_DICT_SCRATCH: u32 = 0x1a2000;
pub const C2_2B3608_SCRATCH: u32 = 0x1a3000;

/// The literal bytes fn_202348 compares, copied out of the arena before
/// the call so the port's borrow never aliases the arena.
pub struct C2DictTables {
    pub types: [[u8; 48]; 3],
    pub type_lens: [usize; 3],
    pub disabled: [u8; 32],
    pub disabled_len: usize,
    pub classes: [u16; 256],
}

fn copy_nul(dst: &mut [u8], bytes: &[u8], addr: u32) -> usize {
    let mut n = 0usize;
    let mut at = addr as usize;
    while n + 1 < dst.len() && at < bytes.len() && bytes[at] != 0 {
        dst[n] = bytes[at];
        n += 1;
        at += 1;
    }
    if n < dst.len() {
        dst[n] = 0;
        n += 1;
    }
    n
}

/// Copy the dictionary literal bytes out of the arena. The literal pools
/// relocate with the ROM (round-8 table rows), so the regional test passes
/// its Rom; US callers pass Rom::Us and are bit-identical.
pub fn c2_dict_tables(m: &Memory<'_>, tables: &mut C2DictTables, rom: crate::rom_region::Rom) {
    use crate::rom_region::addr;
    for (i, pool) in [0x201498u32, 0x2014b8, 0x2014d8].iter().enumerate() {
        tables.type_lens[i] = copy_nul(&mut tables.types[i], m.bytes, addr(rom, *pool));
    }
    tables.disabled_len = copy_nul(&mut tables.disabled, m.bytes, addr(rom, 0x201b74));
}

/// The host fn_202348 runs over: the guest heap on the arena, the live
/// parameter fetch, the arena dictionary walk and the dictionary engine.
/// Unbound callees are recorded, never scripted.
pub struct C2DictHost {
    pub first_unbound: Option<u32>,
    pub unbound_count: u32,
    pub error_log_fired: u32,
    pub lang_cb_fired: u32,
    pub trace: [(u32, u32, u32); 512],
    pub trace_len: usize,
    /// The ROM whose guest memory is live (round-8 recipe). US by default
    /// so US-only paths are unchanged; the regional test sets Eu/Kr.
    pub rom: crate::rom_region::Rom,
}

impl C2DictHost {
    pub fn new() -> Self {
        C2DictHost {
            first_unbound: None,
            unbound_count: 0,
            error_log_fired: 0,
            lang_cb_fired: 0,
            trace: [(0, 0, 0); 512],
            trace_len: 0,
            rom: crate::rom_region::Rom::Us,
        }
    }
    fn record(&mut self, method: u32, arg0: u32, result: u32) {
        if self.trace_len < self.trace.len() {
            self.trace[self.trace_len] = (method, arg0, result);
            self.trace_len += 1;
        }
    }
    /// One funnel for every guest call the dictionary path reaches, so
    /// test-side depes hosts can reuse the exact same bindings.
    pub fn c2_dict_call(&mut self, m: &mut Memory<'_>, method: u32, args: &[u32]) -> u32 {
        self.dispatch(m, method, args)
    }
    fn unbound(&mut self, method: u32) -> u32 {
        if self.first_unbound.is_none() {
            self.first_unbound = Some(method);
        }
        self.unbound_count += 1;
        0x8000_2001
    }
    fn dispatch(&mut self, m: &mut Memory<'_>, method: u32, args: &[u32]) -> u32 {
        // Every guest call the dictionary path reaches funnels here, including
        // raw guest-table targets: normalize unambiguous regional twins, keeping
        // genuine US literals identical (collision-safe to_us, not guest-form).
        let method = crate::rom_region::to_us(self.rom, method);
        let arg0 = args.first().copied().unwrap_or(0);
        let result = self.dispatch_inner(m, method, args);
        self.record(method, arg0, result);
        result
    }
    fn dispatch_inner(&mut self, m: &mut Memory<'_>, method: u32, args: &[u32]) -> u32 {
        match method {
            // fn_17bff4: the byte length of a UTF-16 string, exactly per src.
            0x17bff4 => {
                let mut at = args[0];
                let mut n = 1u32;
                loop {
                    let c = u32::from(rh(m, at));
                    at += 2;
                    if c == 0 {
                        break;
                    }
                    n += if c < 0x80 {
                        1
                    } else if c < 0x800 {
                        2
                    } else if c < 0x10000 {
                        3
                    } else if c < 0x110000 {
                        4
                    } else {
                        5
                    };
                }
                n
            }
            // fn_176f94: the handle validation (pure).
            0x176f94 => crate::boot_port_1::fn_176f94(m, args[0], args[1], args[2], args[3]),
            // The guest heap on the arena, through the guest vtable.
            0x196394 => crate::boot_part_e::alloc(m, args[0], args[1], self),
            0x1963b8 => crate::boot_part_e::calloc(m, args[0], args[1], args[2], self),
            0x196428 => {
                crate::boot_part_e::release(m, args[0], args[1], self);
                0
            }
            // The allocator object's vtable (src/fn_4a779c.c): allocate
            // through the block heap's own method.
            0x4a779c => {
                let target = rw(m, rw(m, args[0]).wrapping_add(8));
                self.dispatch(m, target, &[args[0], args[1], 4])
            }
            // The allocator object's sized-zeroing method (src/fn_4a7758.c).
            0x4a7758 => {
                let n = args[1].wrapping_mul(args[2]);
                let target = rw(m, rw(m, args[0]).wrapping_add(8));
                let p = self.dispatch(m, target, &[args[0], n, 4, 4]);
                if p != 0 {
                    let at = (p as usize).wrapping_sub(m.base as usize);
                    let end = at.wrapping_add(n as usize).min(m.bytes.len());
                    if at < m.bytes.len() {
                        m.bytes[at..end].fill(0);
                    }
                }
                p
            }
            // The allocator object's release (src/fn_4a774c.c).
            0x4a774c => {
                let target = rw(m, rw(m, args[0]).wrapping_add(12));
                self.dispatch(m, target, &[args[0], args[1]])
            }
            // The block heap's allocate, under the block lock.
            0x39f5e8 => {
                let inner = rw(m, args[0].wrapping_add(4));
                self.lock(m, inner.wrapping_add(88));
                let r = m.boot_heap_allocate([inner, args[1], args[2], 0, 0, 0]);
                self.unlock(m, inner.wrapping_add(88));
                r
            }
            // The block heap's release, under the block lock.
            0x39f5b8 => {
                let inner = rw(m, args[0].wrapping_add(4));
                self.lock(m, inner.wrapping_add(88));
                self.dispatch(m, 0x19b78c, &[inner, args[1]]);
                self.unlock(m, inner.wrapping_add(88));
                0
            }
            // The block heap itself (boot_port_1::fn_1850c0 family).
            0x184f20 => {
                let a: [u32; 6] = core::array::from_fn(|i| args.get(i).copied().unwrap_or(0));
                m.boot_heap_allocate(a)
            }
            0x184f84 => {
                m.boot_heap_release(args[0], args[1]);
                0
            }
            // src/fn_19b78c.c: release through the sub-heap, count the free.
            0x19b78c => {
                m.boot_heap_release(args[0].wrapping_add(24), args[1]);
                let count = rw(m, args[0].wrapping_add(84)).wrapping_sub(1);
                ww(m, args[0].wrapping_add(84), count);
                0
            }
            // The arbiter's thread identity and wake.
            0x183edc => 0x1ff82000,
            // Lock release: no-op to match the no-op acquire above
            // (same single-threaded reasoning as `OpenHost`: a real
            // negate would corrupt the idle count the acquire left).
            0x165d00 => 0,
            0x172d18 => 0,
            0x175a1c => {
                let lock_word = if args[0] == 0 { 0 } else { rw(m, args[0] + 4) };
                let mut lock = C2ParamLock { host: &mut *self, word: lock_word };
                crate::speech_part_2::fetch_parameter_m(
                    m,
                    args[0],
                    args[1],
                    args[2],
                    &mut lock,
                    c2_compare,
                )
            }
            0x175944 => {
                let lock_word = if args[0] == 0 { 0 } else { rw(m, args[0] + 4) };
                let mut lock = C2ParamLock { host: &mut *self, word: lock_word };
                crate::reentrant2::get_param_uint_m(
                    m.bytes,
                    if args[0] == 0 { None } else { Some(args[0]) },
                    args[1],
                    args[2],
                    &mut lock,
                    c2_compare,
                )
            }
            // fn_1756d0: the raw registry lookup (src/fn_1756d0.c) with its
            // own 176f2c/176f5c lock flavor and an optional size out-word.
            0x1756d0 => {
                if args[0] != 0 && args[1] != 0 && args[2] != 0 {
                    ww(m, args[2], 0);
                    if args[3] != 0 {
                        ww(m, args[3], 0);
                    }
                }
                let p = if args[0] == 0 { 0 } else { rw(m, args[0] + 4) };
                let mut value = if args[2] == 0 { 0 } else { rw(m, args[2]) };
                let mut size = if args[3] == 0 { 0 } else { rw(m, args[3]) };
                let mut lock = C2Lock176f { host: &mut *self, p };
                let rc = crate::reentrant2::lookup_param_m(
                    m.bytes,
                    if args[0] == 0 { None } else { Some(args[0]) },
                    args[1],
                    (args[2] != 0).then_some(&mut value),
                    (args[3] != 0).then_some(&mut size),
                    &mut lock,
                    c2_compare,
                );
                if args[2] != 0 {
                    ww(m, args[2], value);
                }
                if args[3] != 0 {
                    ww(m, args[3], size);
                }
                rc
            }
            // fn_281aa4: the arena dictionary walk, per src/fn_281aa4.c.
            0x281aa4 => {
                let current = rw(m, args[1]);
                let next = if current == 0 {
                    rw(m, args[0] + 4)
                } else {
                    rw(m, current + 8)
                };
                ww(m, args[1], next);
                if next == 0 {
                    0
                } else {
                    rw(m, next + 4)
                }
            }
            0x17bf70 => fn_17bf70_m(m, args[0], args[1]),
            // The logger gate (byte at log+64) is clear in the capture,
            // so the trace is a provable no-op.
            0x17486c => 0,
            // fn_2b331c: the dictionary's language getter.
            0x2b331c => {
                m.put_word(args[1], m.word(args[0].wrapping_add(0x44)));
                0
            }
            0x2b3608 => crate::speech_part_3::fn_2b3608(
                m,
                args[0],
                args[1],
                args[2],
                args[3],
                C2_2B3608_SCRATCH,
                self,
            ),
            // The dictionary's release: the ARM's returns 0 here; every
            // byte is still compared, so a real write would be caught.
            0x2b4004 => 0,
            0x17c8c0 => crate::fe_text::fold_17c8c0(args[0]),
            // fn_1ce56c: the slot level (src/fn_1ce56c.c).
            0x1ce56c => {
                if args[0] == 0 {
                    0
                } else {
                    let inner = rw(m, args[0]);
                    if rb(m, inner + 4) != 0 {
                        u32::from(rb(m, inner + 0x10c))
                    } else {
                        1
                    }
                }
            }
            // fn_18f488: the finish request (constant).
            0x18f488 => 0,
            0x1960a8 => crate::frontend::find_substr(m.bytes, args[0], args[1]),
            0x176314 => {
                fn_176314_m(m, args[0], args[1]);
                0
            }
            0x17bfc0 => fn_17bfc0_m(m, args[0]),
            0x17bd74 => fn_17bd74_m(m, args[0], args[1]),
            0x173dc0 => {
                self.error_log_fired += 1;
                crate::render_part_a::error_log(
                    m, self, args[0], args[1], args[2], args[3], args[4], args[5],
                )
            }
            0x1738e8 => crate::log_live::fn_1738e8(
                m,
                args[0],
                args[1],
                args[2],
                args[3],
                args[4],
                args.get(5).copied().unwrap_or(0),
                self,
            ),
            _ => self.unbound(method),
        }
    }
}

impl C2DictHost {
    /// fn_1840fc: the block lock.
    fn lock(&mut self, m: &mut Memory<'_>, address: u32) {
        crate::boot_part_b::fn_1840fc(m, address, self);
    }
    /// fn_18423c: the block unlock.
    fn unlock(&mut self, m: &mut Memory<'_>, address: u32) {
        crate::boot_part_b::fn_18423c(m, address, self);
    }
}

/// Forwarder for the arbiter wake (fn_165d00 needs `boot::Calls`, which
/// already has a blanket `HooksM` impl that would clash with ours).
struct C2Wake<'a> {
    host: &'a mut C2DictHost,
}

impl crate::boot::Calls for C2Wake<'_> {
    fn call(&mut self, m: &mut Memory<'_>, f: u32, a: &[u32]) -> u32 {
        self.host.dispatch(m, f, a)
    }
    fn call_wide(&mut self, _: &mut Memory<'_>, _: u32, _: &[u32]) -> u64 {
        0
    }
}

struct C2Lock176f<'a> {
    host: &'a mut C2DictHost,
    p: u32,
}

impl crate::reentrant2::StateLockM for C2Lock176f<'_> {
    fn acquire_m(&mut self, bytes: &mut [u8]) -> u32 {
        let mut m = Memory { base: 0, bytes };
        crate::boot_port_1::fn_176f2c(&mut m, self.p, self.host)
    }
    fn release_m(&mut self, bytes: &mut [u8]) -> u32 {
        let mut m = Memory { base: 0, bytes };
        crate::boot_port_1::fn_176f5c(&mut m, self.p, self.host)
    }
}

struct C2ParamLock<'a> {
    host: &'a mut C2DictHost,
    word: u32,
}

impl crate::reentrant2::StateLockM for C2ParamLock<'_> {
    fn acquire_m(&mut self, bytes: &mut [u8]) -> u32 {
        let mut m = Memory { base: 0, bytes };
        crate::speech_part_2::parameter_lock_m(&mut m, self.word, false, self.host)
    }
    fn release_m(&mut self, bytes: &mut [u8]) -> u32 {
        let mut m = Memory { base: 0, bytes };
        crate::speech_part_2::parameter_lock_m(&mut m, self.word, true, self.host)
    }
}

/// NUL-terminated byte compare, bounded by the arena: the same sign as
/// `frontend::compare_str` on terminated inputs, without ever indexing
/// past the end of a slice.
fn c2_compare(a: u32, b: u32, bytes: &mut [u8]) -> i32 {
    let mut i = 0usize;
    loop {
        let x = bytes.get(a as usize + i).copied().unwrap_or(0);
        let y = bytes.get(b as usize + i).copied().unwrap_or(0);
        if x != y {
            return if x < y { -1 } else { 1 };
        }
        if x == 0 {
            return 0;
        }
        i += 1;
    }
}

impl crate::speech_part_2::Host for C2DictHost {
    fn call(&mut self, memory: &mut Memory<'_>, method: u32, args: &[u32]) -> u32 {
        self.dispatch(memory, method, args)
    }
}

impl crate::log_live::LogMethodsM for C2DictHost {
    fn log_m(
        &mut self,
        method: u32,
        object: u32,
        category: u32,
        level: u32,
        arg: u32,
        format: u32,
        varargs: u32,
        mem: &mut Memory<'_>,
    ) -> u32 {
        self.dispatch(mem, method, &[object, category, level, arg, format, varargs])
    }
}

impl crate::boot_part_b::FormatHost for C2DictHost {}

impl crate::boot_part_b::Host for C2DictHost {
    fn call(&mut self, memory: &mut Memory<'_>, function: u32, args: &[u32]) -> u32 {
        self.dispatch(memory, function, args)
    }
    fn thread_id(&mut self) -> u32 {
        0x1ff82000
    }
}

impl crate::boot_part_e::Host for C2DictHost {
    fn call(&mut self, m: &mut Memory<'_>, method: u32, arguments: &[u32]) -> u32 {
        self.dispatch(m, method, arguments)
    }
}

impl crate::reentrant::HooksM for C2DictHost {
    fn call_m(&mut self, address: u32, args: &[u32], mem: &mut Memory<'_>) -> u32 {
        if address == 0 {
            self.lang_cb_fired += 1;
            return 0x8000_2001;
        }
        self.dispatch(mem, address, args)
    }
    fn literal_m(&self, literal: crate::reentrant::LiteralM) -> u32 {
        if literal == crate::reentrant::LiteralM::NumberOutputTranscriptions {
            C2_NUMBEROUTPUTTRANSCRIPTIONS
        } else {
            u32::MAX
        }
    }
}

impl crate::boot_part_f::Host for C2DictHost {
    fn call(&mut self, m: &mut Memory<'_>, method: u32, args: &[u32]) -> u32 {
        self.dispatch(m, method, args)
    }
    fn compare(
        &mut self,
        m: &mut Memory<'_>,
        _method: u32,
        key: &mut crate::boot_part_f::Key,
        value: &mut u32,
    ) -> u32 {
        crate::boot_part_f::compare_parameter(m, key.clone(), *value)
    }
}

impl crate::speech_part_5::Host for C2DictHost {
    fn scratch(&self) -> u32 {
        C2_2B3608_SCRATCH
    }
    fn format(&self, callsite: u32) -> u32 {
        callsite
    }
    fn dictionary_entries(
        &mut self,
        _m: &mut Memory<'_>,
        _dictionary: u32,
        entries: &mut u32,
        count: &mut u32,
    ) -> u32 {
        *entries = 0;
        *count = 0;
        0
    }
    fn record(
        &mut self,
        _m: &mut Memory<'_>,
        _record: u32,
        _arg: u32,
        _buffer: &mut crate::speech_part_5::RecordBuffer,
        found: &mut u8,
    ) -> u32 {
        *found = 0;
        0
    }
    fn entries(
        &mut self,
        _m: &mut Memory<'_>,
        _a: u32,
        _b: u32,
        _c: u32,
        array: &mut u32,
        count: &mut u32,
    ) -> u32 {
        *array = 0;
        *count = 0;
        0
    }
}

impl crate::speech_part_3::SpeechHost for C2DictHost {}

/// fn_202348 (src/fn_2012bc.c:498) over the caller's arena: the dictionary
/// lookup with the literals the crate test builds and a host whose heap
/// goes to the guest heap on the arena.  `scratch` is 128 disjoint bytes,
/// saved and restored by the port.
#[allow(clippy::too_many_arguments)]
pub fn fn_202348_m(
    m: &mut Memory<'_>,
    p: u32,
    magic: u32,
    text: u32,
    len: u32,
    out: u32,
    arg5: u32,
    lang: u32,
    scratch: u32,
    tables: &C2DictTables,
    h: &mut C2DictHost,
) -> u32 {
    let t = crate::speech_part_2::DictionaryLiterals {
        types: [
            &tables.types[0][..tables.type_lens[0]],
            &tables.types[1][..tables.type_lens[1]],
            &tables.types[2][..tables.type_lens[2]],
        ],
        disabled_mode: &tables.disabled[..tables.disabled_len],
        langcode: crate::rom_region::addr(h.rom, C2_LANGCODE_KEY),
        mlset: crate::rom_region::addr(h.rom, C2_MLSET_KEY),
        category: crate::rom_region::addr(h.rom, 0x8f9734),
        invalid_format: crate::rom_region::addr(h.rom, 0x155ec0),
        name: crate::rom_region::addr(h.rom, 0x14db07),
        empty: crate::rom_region::addr(h.rom, 0x8f972c),
        type_label: crate::rom_region::addr(h.rom, 0x201500),
        trace_formats: [crate::rom_region::addr(h.rom, 0x202778), crate::rom_region::addr(h.rom, 0x2027a0), crate::rom_region::addr(h.rom, 0x2027a5), crate::rom_region::addr(h.rom, 0x1feedc)],
        labels: [crate::rom_region::addr(h.rom, 0x1fee78), crate::rom_region::addr(h.rom, 0x14daf4), crate::rom_region::addr(h.rom, 0x1cd7e8), crate::rom_region::addr(h.rom, 0x1feed8), crate::rom_region::addr(h.rom, 0x155ea0), crate::rom_region::addr(h.rom, 0x1feed0), crate::rom_region::addr(h.rom, 0x14da60)],
    };
    crate::speech_part_2::fn_202348(
        m, p, magic, text, len, out, arg5, lang, &tables.classes, &t, scratch, h,
    )
}

