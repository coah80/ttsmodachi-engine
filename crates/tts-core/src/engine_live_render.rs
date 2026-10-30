//! Live (arena-threaded) forms of the render chain, from the backend progress
//! call 2645b4 to the pcm sink.
//!
//! The render ports in `render_part_*` take their allocator and their method
//! calls through `frontend::heap_alloc`, `frontend::heap_release`,
//! `frontend::heap_realloc` and `frontend::call_method`. Those read a shared
//! `&[u8]` arena and hand the host a callback with no arena, so a live host
//! cannot reenter the arena inside those callbacks: the blanket
//! `impl<T: HeapVtable + ?Sized> HeapVtableM for T` drops the arena for every
//! host that also satisfies `render_part_c::Host`.
//!
//! These forms dispatch the same engine entries with the caller's arena. The
//! guest heap's own method table is read from the arena inside the callback,
//! and the method is called with the arena, exactly as `boot_part_e::Heap`
//! does for the boot heap. The original ports are left unchanged.
use crate::boot::Memory;
use crate::frontend;
use crate::normalization;
use crate::render_part_c;
use crate::render_part_c2::{ContextHost, ParserEnvironment, ParserRuntime, ResourceHost, SegmentCallbackHost, SegmentEnvironment};

fn nz(p: u32) -> Option<u32> {
    if p == 0 {
        None
    } else {
        Some(p)
    }
}

fn half(m: &Memory<'_>, p: u32) -> u16 {
    let at = m.offset(p);
    u16::from_le_bytes(m.bytes[at..at + 2].try_into().unwrap())
}

/// The guest heap's four methods, dispatched with the caller's arena.
///
/// The heap handle is the one `frontend::heap_alloc_m` was given; the method
/// table is read from the arena at dispatch time, so the heap's own vtable
/// is the one the ARM would call.
pub struct LiveHeap<'a, H> {
    pub heap: u32,
    pub host: &'a mut H,
}

/// The live parser allocator: the guest heap's methods with the caller's
/// arena, plus the counted form the fetch port uses. The heap is fixed per
/// parse; the object check fails loud on a second heap.
impl<H: crate::render_part_c::Host> crate::render_part_c2::ParserAllocatorM for LiveHeap<'_, H> {
    fn calloc_m(&mut self, object: u32, count: u32, size: u32, memory: &mut [u8]) -> u32 {
        assert_eq!(object, crate::frontend::mem_word(memory, self.heap + 4), "live heap object changed heaps");
        let method = crate::frontend::mem_word(memory, crate::frontend::mem_word(memory, self.heap) + 4);
        crate::render_part_c::Host::call(
            self.host,
            &mut Memory { base: 0, bytes: memory },
            method,
            &[object, count, size],
        )
    }
}

/// The live allocator over a host.
pub fn live_heap<'a, H: crate::render_part_c::Host>(heap: u32, host: &'a mut H) -> LiveHeap<'a, H> {
    LiveHeap { heap, host }
}

/// What a host callback saw: the entry, its arguments and a byte of the
/// arena, so a gate can prove the arena reached the callback.
#[derive(Clone, Copy)]
pub struct CallRecord {
    pub entry: u32,
    pub args: [u32; 4],
    pub count: u8,
    pub marker: u8,
}

impl<H: crate::render_part_c::Host> crate::frontend::HeapVtableM for LiveHeap<'_, H> {
    fn alloc_at_m(&mut self, object: u32, size: u32, mem: &mut [u8]) -> u32 {
        // Every memory-passing call derives its object from the heap it
        // names; a foreign object means a second heap this binding does
        // not serve.
        assert_eq!(object, crate::frontend::mem_word(mem, self.heap + 4), "live heap object changed heaps");
        let method = crate::frontend::mem_word(mem, crate::frontend::mem_word(mem, self.heap));
        crate::render_part_c::Host::call(
            self.host,
            &mut Memory { base: 0, bytes: mem },
            method,
            &[object, size],
        )
    }
    fn realloc_at_m(&mut self, object: u32, old: u32, size: u32, mem: &mut [u8]) -> u32 {
        assert_eq!(object, crate::frontend::mem_word(mem, self.heap + 4), "live heap object changed heaps");
        let method =
            crate::frontend::mem_word(mem, crate::frontend::mem_word(mem, self.heap) + 8);
        crate::render_part_c::Host::call(
            self.host,
            &mut Memory { base: 0, bytes: mem },
            method,
            &[object, old, size],
        )
    }
    fn release_at_m(&mut self, object: u32, pointer: u32, mem: &mut [u8]) {
        assert_eq!(object, crate::frontend::mem_word(mem, self.heap + 4), "live heap object changed heaps");
        let method =
            crate::frontend::mem_word(mem, crate::frontend::mem_word(mem, self.heap) + 12);
        crate::render_part_c::Host::call(
            self.host,
            &mut Memory { base: 0, bytes: mem },
            method,
            &[object, pointer],
        );
    }
}

/// fn_1963b8 over the method table, with the caller's arena.
///
/// `frontend::call_method` reads the method table through a shared arena and
/// calls the host with no arena; this form reads the same table and calls the
/// same method with the arena the caller owns.
pub fn call_method_m(
    mem: &mut [u8],
    heap: Option<u32>,
    count: u32,
    size: u32,
    host: &mut impl crate::render_part_c::Host,
) -> u32 {
    let Some(heap) = heap else { return 0 };
    if count == 0 || size == 0 {
        return 0;
    }
    let object = crate::frontend::mem_word(mem, heap + 4);
    let method = crate::frontend::mem_word(mem, crate::frontend::mem_word(mem, heap) + 4);
    crate::render_part_c::Host::call(
        host,
        &mut Memory { base: 0, bytes: mem },
        method,
        &[object, count, size],
    )
}

/// The live normalization host of render part c.
///
/// `normalization::NormalizationHost` already passes the arena to every
/// callback; the render port's own adapter drops it again on the way to
/// `frontend::heap_alloc`. This adapter keeps it.
pub struct LiveNormal<'a, H> {
    pub environment: &'a crate::render_part_c::Environment,
    pub host: &'a mut H,
}

impl<H: crate::render_part_c::Host> crate::normalization::NormalizationHost for LiveNormal<'_, H> {
    fn resolve(&mut self, bytes: &mut [u8], a: u32, b: u32) -> (u32, u32) {
        crate::render_part_c::resolve(&Memory { base: 0, bytes }, [a, b], self.environment)
    }
    fn calloc(&mut self, bytes: &mut [u8], heap: u32, count: u32, size: u32) -> Option<u32> {
        let p = call_method_m(bytes, nz(heap), count, size, self.host);
        if p == 0 {
            None
        } else {
            Some(p)
        }
    }
    fn alloc(&mut self, bytes: &mut [u8], heap: u32, size: u32) -> Option<u32> {
        let mut v = LiveHeap {
            heap,
            host: &mut *self.host,
        };
        let p = crate::frontend::heap_alloc_m(bytes, nz(heap), size, &mut v);
        if p == 0 {
            None
        } else {
            Some(p)
        }
    }
    fn realloc(&mut self, bytes: &mut [u8], heap: u32, old: u32, size: u32) -> Option<u32> {
        let mut v = LiveHeap {
            heap,
            host: &mut *self.host,
        };
        let p = crate::frontend::heap_realloc_m(bytes, nz(heap), old, size, &mut v);
        if p == 0 {
            None
        } else {
            Some(p)
        }
    }
    fn free(&mut self, bytes: &mut [u8], heap: u32, pointer: u32) {
        let mut v = LiveHeap {
            heap,
            host: &mut *self.host,
        };
        crate::frontend::heap_release_m(bytes, nz(heap), nz(pointer), &mut v);
    }
}

/// The live normalization host of render part c2.
pub struct LiveFetch<'a, H> {
    pub host: &'a mut H,
    pub env: &'a crate::render_part_c::Environment,
}

impl<H: crate::render_part_c::Host> crate::normalization::NormalizationHost for LiveFetch<'_, H> {
    fn resolve(&mut self, bytes: &mut [u8], a: u32, b: u32) -> (u32, u32) {
        crate::render_part_c::resolve(&Memory { base: 0, bytes }, [a, b], self.env)
    }
    fn calloc(&mut self, bytes: &mut [u8], heap: u32, count: u32, size: u32) -> Option<u32> {
        let p = call_method_m(bytes, nz(heap), count, size, self.host);
        if p == 0 {
            None
        } else {
            Some(p)
        }
    }
    fn alloc(&mut self, bytes: &mut [u8], heap: u32, size: u32) -> Option<u32> {
        let mut v = LiveHeap {
            heap,
            host: &mut *self.host,
        };
        let p = crate::frontend::heap_alloc_m(bytes, nz(heap), size, &mut v);
        if p == 0 {
            None
        } else {
            Some(p)
        }
    }
    fn realloc(&mut self, bytes: &mut [u8], heap: u32, old: u32, size: u32) -> Option<u32> {
        let mut v = LiveHeap {
            heap,
            host: &mut *self.host,
        };
        let p = crate::frontend::heap_realloc_m(bytes, nz(heap), old, size, &mut v);
        if p == 0 {
            None
        } else {
            Some(p)
        }
    }
    fn free(&mut self, bytes: &mut [u8], heap: u32, pointer: u32) {
        let mut v = LiveHeap {
            heap,
            host: &mut *self.host,
        };
        crate::frontend::heap_release_m(bytes, nz(heap), nz(pointer), &mut v);
    }
}

/// The sequence destructor, fn_263a6c, at its true entry 0x263a64.
///
/// The delinked symbol starts eight bytes into the function, so the target
/// address is 0x263a64 and the body is the one the C carries.
pub fn fn_263a64(
    m: &mut Memory<'_>,
    o: u32,
    e: &crate::render_part_c::Environment,
    h: &mut impl crate::render_part_c::Host,
) -> u32 {
    if o == 0 {
        return 0;
    }
    let buffer = m.word(o + 0x70);
    if buffer == 0 {
        return 0;
    }
    let cached = [
        u32::from_le_bytes(e.cached_handle[..4].try_into().unwrap()),
        u32::from_le_bytes(e.cached_handle[4..].try_into().unwrap()),
    ];
    let pair = [m.word(o), m.word(o + 4)];
    let pointed = if pair != cached && pair[0] != 0 && pair[1] == 0x1e61 {
        Some(half(m, pair[0]))
    } else {
        None
    };
    let mut ctx = 0u32;
    let rc = crate::frontend::resolve_engine_context(
        pair[0],
        pointed,
        pair[1],
        &e.cached_handle,
        e.default_context,
        &mut ctx,
    );
    if rc >= 0x80000000 {
        return 0x81102007;
    }
    let heap = m.word(ctx + 4);
    let mut v = LiveHeap {
        heap,
        host: h,
    };
    crate::frontend::heap_release_m(m.bytes, nz(heap), Some(buffer), &mut v);
    m.put_word(o + 0x70, 0);
    0
}


// ---- the helpers the copied render bodies use ---------------------------

use crate::render_part_b::RenderArgument;

fn byte(m: &mut Memory<'_>, p: u32, v: u8) {
    let at = m.offset(p);
    m.bytes[at] = v;
}

fn move_bytes(m: &mut Memory<'_>, d: u32, s: u32, n: u32) {
    crate::fe_text::memmove_195b38(m.bytes, d - m.base, s - m.base, n);
}

fn copy_bytes(m: &mut Memory<'_>, d: u32, s: u32, n: u32) {
    crate::fe_text::memcpy_195ae8(m.bytes, d - m.base, s - m.base, n);
}

fn narrow(m: &mut Memory<'_>, d: u32, s: u32, n: u32) {
    crate::fe_text::narrow_n_17649c(m.bytes, d, s, n);
}

fn add_word(m: &mut Memory<'_>, p: u32, v: u32) {
    m.put_word(p, m.word(p).wrapping_add(v));
}

/// The render log, over the live host. The gate and the fan-out are the ones
/// `render_part_b` uses; the target callback receives the caller's arena.
fn render_log(
    m: &mut Memory<'_>,
    ctx: u32,
    e: &crate::render_part_b::RenderEnvironment,
    fmt: u32,
    args: &[RenderArgument<'_>],
    h: &mut impl crate::render_part_b::RenderLog,
) {
    let logger = m.word(ctx + 16);
    let mut view = [0u8; 128];
    let p = if logger == 0 { 0 } else { 64 };
    if logger != 0 {
        view[64 + 0x30] = m.byte(logger + 0x30);
        view[64 + 0x31] = m.byte(logger + 0x31);
    }
    let rc = crate::normalization::log_fanout(&mut view, p, e.category, 5, 0, fmt);
    if logger == 0
        || rc >= 0x80000000
        || 5 > view[64 + 0x30] as u32
        || (view[64 + 0x31] != 0 && 5 > 4)
        || fmt == 0
    {
        return;
    }
    for i in 0..6 {
        let slot = if i == 5 { 0x34 } else { 8 + i * 8 };
        let methods = m.word(logger + slot + 4);
        if methods != 0 && i != 1 {
            let method = m.word(methods + 24);
            if method != 0 {
                h.log_target(
                    m,
                    method,
                    m.word(logger + slot),
                    e.category,
                    5,
                    0,
                    fmt,
                    args,
                );
            }
        }
    }
}

/// fn_196428 with the caller's arena.
fn live_free(m: &mut Memory<'_>, heap: u32, p: u32, h: &mut impl crate::render_part_c::Host) {
    let mut v = LiveHeap {
        heap,
        host: h,
    };
    crate::frontend::heap_release_m(m.bytes, nz(heap), nz(p), &mut v);
}

/// The lifecycle calls of `fn_2b92d8`, with the caller's arena.
pub(crate) struct LiveLifecycleCalls<'a, H> {
    pub host: &'a mut H,
    pub scratch: u32,
}

impl<H: crate::render_part_b::LifecycleHost> crate::frontend::HeapVtable
    for LiveLifecycleCalls<'_, H>
{
    fn alloc_at(&mut self, o: u32, n: u32) -> u32 {
        self.host.alloc_at(o, n)
    }
    fn realloc_at(&mut self, o: u32, p: u32, n: u32) -> u32 {
        self.host.realloc_at(o, p, n)
    }
    fn release_at(&mut self, o: u32, p: u32) {
        self.host.release_at(o, p)
    }
}

impl<H: crate::render_part_b::LifecycleHost> crate::frontend::MethodTable
    for LiveLifecycleCalls<'_, H>
{
    fn call_at(&mut self, o: u32, n: u32, s: u32) -> u32 {
        self.host.call_at(o, n, s)
    }
}

impl<H: crate::render_part_b::LifecycleHost> crate::render_part_c::Host
    for LiveLifecycleCalls<'_, H>
{
    fn call(&mut self, m: &mut Memory<'_>, entry: u32, a: &[u32]) -> u32 {
        if entry == 0x175a24 {
            crate::render_part_b::fn_175a24(m, a[0], a[1], self.scratch, self.host)
        } else {
            self.host.invoke(entry, a)
        }
    }
    fn output(&mut self, _: &mut Memory<'_>, entry: u32, _: &[u32]) -> (u32, u32) {
        panic!("unrequested output method {entry:x}")
    }
}

// ---- the live render environment ---------------------------------------

/// The ROM constants of the render chain. Every address is a ROM address or
/// a ROM record, read out of the captured image.
pub mod rom {
    /// D_914e4c: the three tags the active sequence clears.
    pub const CLEAR_ACTIVE: u32 = 0x914e4c;
    /// D_9150a4: the six tags every sequence clears.
    pub const CLEAR_ALL: u32 = 0x9150a4;
    /// D_914ee0: the marker name table.
    pub const MARKER_NAMES: u32 = 0x914ee0;
    /// D_915062: the render log's category.
    pub const LOG_CATEGORY: u32 = 0x915062;
    /// D_9151b0: the render log's marker format.
    pub const MARKER_LOG: u32 = 0x9151b0;
    /// "readmode", the literal of fn_1756d0 and the key of fn_1758ac.
    pub const READ_MODE: u32 = 0x264bdc;
    /// "ppresetlen", the literal of fn_175944 and the key of fn_1758ac.
    pub const RESET_LEN: u32 = 0x264c44;
    /// "extraesctn", the literal of fn_175a1c in fn_2bfa90.
    pub const EXTRA_KEY: u32 = 0x2c0008;
    /// D_9c5630: the parser's global record; the event record is at +0xc4.
    pub const PARSER_GLOBALS: u32 = 0x9c5630;
    /// D_9c5650: the parser log's category.
    pub const PARSER_CATEGORY: u32 = 0x9c5650;
    /// D_9c5830 and D_9c5888: the parser's two log formats.
    pub const PARSER_FORMATS: [u32; 2] = [0x9c5830, 0x9c5888];
    /// The format strings of fn_2645b4, in the order the ARM loads them.
    ///
    /// Every address is the PC-relative literal the ARM's own code resolves:
    /// `add rN, pc, #imm` at the call site, computed as `site + 8 + imm`, read
    /// out of the captured image with capstone. The order is the order of
    /// first appearance in the ARM, which is also the order of the C's
    /// fn_2645b4. Twenty-eight call sites use twenty-four distinct strings:
    /// "AFTER esc loop", "AFTER sparser_GetTextElement",
    /// "BEFORE seq_loc_DoExtendedTOIWords", "BEFORE tolhp_RewritePhonTextInLhp"
    /// and "AFTER tolhp_RewritePhonTextInLhp" are each used twice.
    pub const LABELS: [u32; 24] = [
        0x264be4, // ""
        0x264bec, // "text/plain;charset=utf-16"
        0x264c08, // "[SEQ] pfReadDirectStart returns cTextInStream %u bytes"
        0x264c50, // "[SEQ] SSFT_PARAMC_PP_RESETLEN len is %u bytes"
        0x264c80, // "[SEQ] pfGetPosition is %u bytes - sync marker position"
        0x264edc, // "AFTER seq_GetTextBlock"
        0x264ef4, // "BEFORE escseqs_Process TAG_ESCSEQS_OUT_MARKER"
        0x264f24, // "BEFORE escseqs_Process TAG_ESCSEQS_OUT_TEXT"
        0x264f50, // "AFTER escseqs_Process TAG_ESCSEQS_OUT_MARKER"
        0x264f80, // "AFTER escseqs_Process TAG_ESCSEQS_OUT_TEXT"
        0x264fac, // "AFTER esc loop"
        0x264fbc, // "AFTER esc loop (out marker)"
        0x264fdc, // "AFTER sparser_ScanTextElement"
        0x265350, // "BEFORE sparser_GetTextElement"
        0x265378, // "AFTER sparser_GetTextElement"
        0x265398, // "AFTER sparser_GetTextElement (out marker)"
        0x2657a8, // "[SEQ](Txt) (text processed) (tag=%d) len=%d (w_char *) \"%s\""
        0x2657e4, // "[SEQ] bImmediateVoiceSwitch - immediate return"
        0x265814, // "[SEQ] bEndOnVoiceSwitch - continue processing"
        0x265844, // "AFTER seq_loc_AlignMarkers"
        0x265860, // "AFTER wparser_NormalizeWords"
        0x265880, // "BEFORE seq_loc_DoExtendedTOIWords"
        0x265a80, // "BEFORE tolhp_RewritePhonTextInLhp"
        0x265aa4, // "AFTER tolhp_RewritePhonTextInLhp"
    ];
}

fn word(m: &Memory<'_>, p: u32) -> u32 {
    let at = m.offset(p);
    u32::from_le_bytes(m.bytes[at..at + 4].try_into().unwrap())
}

/// The engine's cached handle, the eight bytes at 0x8d7660.
pub fn cached_pair(m: &Memory<'_>) -> [u8; 8] {
    core::array::from_fn(|i| m.byte(0x8d7660 + i as u32))
}

/// The word at the guest address.
pub fn guest_word(m: &Memory<'_>, p: u32) -> u32 {
    word(m, p)
}

/// fn_2bfa90, live: the allocator keeps the caller's arena. Choose a parser mode, constrain marker batches and insert EOS.
///
/// The comparison receives the caller's bytes with every call: a comparison
/// that captured the arena would freeze the arena for the whole call.
pub fn fn_2bfa90_m(
    m: &mut Memory<'_>,
    p: u32,
    text: u32,
    out: u32,
    mode: u32,
    e: &crate::render_part_c::Environment,
    data: &crate::render_part_c::ParserData<'_>,
    host: &mut (impl crate::render_part_c::BoundaryHost + crate::log_live::LogMethodsM),
    compare: &mut impl FnMut(&[u8], u32, u32) -> i32,
) -> u32 {
    assert_eq!(m.base, 0);
    if p == 0 || text == 0 || out == 0 {
        return 0x81102007;
    }
    let (status, engine) = crate::render_part_c::resolve(m, [m.word(p), m.word(p + 4)], e);
    if status >= 0x80000000 {
        return 0x81102007;
    }
    let s = data.scratch;
    let mut saved = [0u8; 1200];
    saved.copy_from_slice(&m.bytes[s as usize..s as usize + 1200]);
    m.bytes[s as usize..s as usize + 40].copy_from_slice(data.eos);
    let mut bridge = LiveNormal {
        environment: e,
        host,
    };
    let mut timeline = 0;
    let mut list = 0;
    let mut strings = 0;
    let mut edit = 0;
    let rc = (|| {
        if half(m, p + 48) == 0 {
            let mut work = [0; 20];
            work.copy_from_slice(&m.bytes[s as usize + 40..s as usize + 60]);
            crate::render_part_a::error_log(
                m,
                bridge.host,
                m.word(engine + 16),
                data.category,
                0x710,
                0,
                0,
                s + 40,
            );
            m.bytes[s as usize + 40..s as usize + 60].copy_from_slice(&work);
            return 0x81102007;
        }
        let rc = crate::normalization::fetch_entry(
            m.bytes,
            text,
            0x3f2,
            1,
            Some(&mut timeline),
            &mut bridge,
        );
        if rc >= 0x80000000 {
            m.bytes[out as usize] = 0;
            return 0;
        }
        let rc = crate::normalization::fetch_entry(
            m.bytes,
            text,
            0x3f3,
            1,
            Some(&mut list),
            &mut bridge,
        );
        if rc >= 0x80000000 {
            return rc;
        }
        let events = m.word(list + 4);
        let count = half(m, list + 8) as u32;
        let rc = crate::normalization::fetch_entry(
            m.bytes,
            text,
            0x3f4,
            1,
            Some(&mut strings),
            &mut bridge,
        );
        if rc >= 0x80000000 {
            return rc;
        }
        let rc = crate::normalization::open_iterator(
            m.bytes,
            m.word(p),
            m.word(p + 4),
            text,
            0x3f3,
            Some(&mut edit),
            &mut bridge,
        );
        if rc >= 0x80000000 {
            return rc;
        }
        // After opening an iterator the ROM funnels every result through done,
        // which releases it and returns zero, including failed callees.
        let _result = (|| {
            let mut boundary = 0;
            let mut scratch = 0;
            let rc = crate::normalization::scan_table(
                m.bytes,
                timeline,
                Some(&mut boundary),
                list,
                Some(&mut scratch),
                1,
                data.separators,
            );
            if rc >= 0x80000000 {
                return rc;
            }
            let scale = half(m, timeline + 2) as u32;
            let mut i = 1;
            let mut markers = 0u16;
            while i < count
                && m.word(events + i * 40 + 20)
                    .wrapping_sub(m.word(events + 20))
                    <= (boundary as u32 * scale).wrapping_add(m.word(events + 20))
            {
                let t = half(m, events + i * 40 + 10);
                if t != 0x4000 && t != 3 {
                    markers = markers.wrapping_add(1);
                }
                i += 1;
            }
            if markers >= half(m, p + 100) {
                markers = 0;
                i = 1;
                let (rc, table) = crate::render_part_c::iterator_table(m, edit);
                if rc >= 0x80000000 {
                    return rc;
                }
                list = table;
                while i < count
                    && m.word(events + i * 40 + 20)
                        .wrapping_sub(m.word(events + 20))
                        <= scale * boundary as u32
                {
                    let event = events + i * 40;
                    let t = half(m, event + 10);
                    if t != 0x4000 && t != 3 {
                        markers = markers.wrapping_add(1);
                    }
                    if markers == half(m, p + 100) {
                        let rc = crate::render_part_c::set_iterator_index(m, edit, i as i32);
                        if rc >= 0x80000000 {
                            return rc;
                        }
                        m.put_word(s + 12, m.word(event + 12));
                        m.put_word(s + 20, m.word(event + 20));
                        let rc = crate::text_input::insert_span(m.bytes, edit, s, 0, &mut bridge);
                        if rc >= 0x80000000 {
                            return rc;
                        }
                        crate::render_part_c::parser_log(
                            m,
                            m.word(m.word(p + 8) + 16),
                            data,
                            data.formats[0],
                            &[i],
                            bridge.host,
                        );
                        m.bytes[out as usize] = 1;
                        return 0;
                    }
                    i += 1;
                }
            } else if markers == 0 && boundary > 0x400 {
                let (rc, table) = crate::render_part_c::iterator_table(m, edit);
                if rc >= 0x80000000 {
                    return rc;
                }
                list = table;
                boundary = 0x400;
                m.put_word(s + 12, m.word(events + 12));
                m.put_word(s + 20, m.word(events + 20));
                i = 0;
                while i < count
                    && half(m, events + i * 40 + 10) == 0x4000
                    && m.word(events + i * 40 + 20)
                        .wrapping_sub(m.word(events + 20))
                        <= scale * boundary as u32
                {
                    m.put_word(s + 12, m.word(events + i * 40 + 12));
                    m.put_word(s + 20, m.word(events + i * 40 + 20));
                    i += 1;
                }
                let rc = crate::render_part_c::set_iterator_index(m, edit, i as i32);
                if rc >= 0x80000000 {
                    return rc;
                }
                let rc = crate::text_input::insert_span(m.bytes, edit, s, 0, &mut bridge);
                if rc >= 0x80000000 {
                    return rc;
                }
                crate::render_part_c::parser_log(
                    m,
                    m.word(m.word(p + 8) + 16),
                    data,
                    data.formats[1],
                    &[i],
                    bridge.host,
                );
                m.bytes[out as usize] = 1;
                return 0;
            }
            let rc = crate::normalization::scan_table(
                m.bytes,
                timeline,
                Some(&mut boundary),
                list,
                Some(&mut scratch),
                2,
                data.separators,
            );
            if rc >= 0x80000000 {
                return rc;
            }
            let count = half(m, list + 8) as u32;
            for i in 1..count {
                let event = events + i * 40;
                if m.word(event + 20) > (boundary as u32 * 2).wrapping_add(m.word(events + 20)) {
                    break;
                }
                if half(m, event + 10) == 20 {
                    m.put_half(mode, m.word(event + 28) as u16);
                }
            }
            let selected = half(m, mode) as i16;
            let rc = if (2..=4).contains(&selected) {
                crate::render_part_c::parser_boundary_callback(
                    m,
                    timeline,
                    boundary as u32,
                    out,
                    selected,
                    crate::render_part_c::pd_separators(data),
                )
            } else {
                let mut name = 0;
                // The ARM's fn_175a1c holds the params object's lock across
                // the lookup (fn_176f2c/fn_176f5c on the lock word). The
                // live form runs the real lock path through the dispatcher
                // and hands the lookup the already-held lock.
                let params = m.word(engine + 20);
                let arc = if params == 0 {
                    0
                } else {
                    <_ as crate::render_part_c::Host>::call(bridge.host, m, 0x176f2c, &[m.word(params + 4)])
                };
                if arc >= 0x80000000 {
                    return arc;
                }
                frontend::fetch_param(
                    m.bytes,
                    if params == 0 {
                        None
                    } else {
                        Some(params)
                    },
                    data.extra_key,
                    Some(&mut name),
                    &mut HeldLock,
                    &mut |x: u32, y: u32| compare(m.bytes, x, y),
                );
                let rrc = if params == 0 {
                    0
                } else {
                    <_ as crate::render_part_c::Host>::call(bridge.host, m, 0x176f5c, &[m.word(params + 4)])
                };
                if rrc >= 0x80000000 {
                    return rrc;
                }
                let mut value = m.byte(out);
                let rc = crate::text_input::check_boundary(
                    m.bytes,
                    timeline,
                    list,
                    strings,
                    boundary as u32,
                    name,
                    &mut value,
                    s + 64,
                    data.separators,
                );
                m.bytes[out as usize] = value;
                rc
            };
            if rc >= 0x80000000 || m.byte(out) != 0 || m.byte(timeline + 12) == 0 {
                return rc;
            }
            let mut found = false;
            for i in 0..half(m, list + 8) as u32 {
                if matches!(half(m, m.word(list + 4) + i * 40 + 10), 26 | 29 | 40 | 8) {
                    found = true;
                    break;
                }
            }
            m.bytes[out as usize] = found as u8;
            rc
        })();
        crate::normalization::release_iterator(m.bytes, edit, &mut bridge);
        edit = 0;
        0
    })();
    m.bytes[s as usize..s as usize + 1200].copy_from_slice(&saved);
    rc
}

pub struct SyncData {
    pub category: u32,
    pub formats: [u32; 2],
    pub stack: u32,
}

/// fn_2c0014. Select parser-mode windows, emit segments and add chunk splits.
/// stack is 160 disjoint caller-owned bytes, restored on return.
///
/// The live form carries the host and the parser environment directly: the
/// shared `ParserRuntime` would have to own the host, and the host is the
/// engine's dispatcher. The one path the shared runtime served here, the
/// chunk split, is not taken by the request this binding serves.
/// The sentence splitter for a sent segment: the ARM's split_input.
/// Builds the parser runtime the shared segment_split port needs: the
/// lock verdict is precomputed from the arena (see lock_verdict) and the
/// comparator reads the immutable keys from the snapshot.
fn segment_split_live<H>(
    m: &mut Memory<'_>,
    ctx: u32,
    timeline: u32,
    base: u32,
    end: u32,
    it: u32,
    index: u16,
    stack: u32,
    seg: &SegmentEnvironment<'_>,
    env: &crate::render_part_c2::Environment<'_>,
    host: &mut H,
    snap: &[u8],
) -> u32
where
    H: ResourceHost + SegmentCallbackHost + crate::render_part_b::ParameterHost,
{
    let (rrc, owner) = crate::render_part_c2::resolve(m, [m.word(ctx), m.word(ctx + 4)], &env.engine);
    let mut lock = if rrc >= 0x80000000 {
        LiveLock { rc: 0, entry: 0, target: 0, release_rc: 0, release_entry: 0, release_target: 0 }
    } else {
        let props = m.word(owner + 20);
        lock_verdict(m, if props == 0 { 0 } else { m.word(props + 4) })
    };
    let parameters = parameters_of(m);
    let separators = separators_of(m);
    let mut inserted = [0u8; 40];
    let at = parser_rom::DEFAULTS as usize + 0x8c;
    inserted.copy_from_slice(&m.bytes[at..at + 40]);
    let parser = crate::render_part_c2::ParserEnvironment {
        separators: &separators,
        boundary: None,
        default_name: 0,
        parameters,
        voice_key: 0,
        inserted_image: inserted,
    };
    let mut ops = LiveTreeOps;
    let mut before = LiveBefore;
    let mut callback = LiveElement;
    let mut compare = move |x: u32, y: u32| -> i32 { crate::normalization::strcmp_at(snap, x, y) };
    let mut rt = ParserRuntime {
        env,
        parser: &parser,
        host,
        lock: &mut lock,
        ops: &mut ops,
        before: &mut before,
        callback: &mut callback,
        compare: &mut compare,
        scratch: scratch::RUNTIME,
    };
    crate::render_part_c2::segment_split(m, ctx, timeline, base, end, it, index, stack, seg, &mut rt)
}

pub fn fn_2c0014<H>(
    m: &mut Memory<'_>,
    ctx: u32,
    handle: u32,
    flag3: i32,
    flag4arg: u32,
    stack: u32,
    seg: &SegmentEnvironment<'_>,
    env: &crate::render_part_c2::Environment<'_>,
    host: &mut H,
    snap: &[u8],
) -> u32
where
    H: ResourceHost + SegmentCallbackHost + crate::render_part_b::ParameterHost,
{
    if ctx == 0 || handle == 0 {
        return 0x81102007;
    }
    if half(m, ctx + 48) == 0 {
        return 0x81102007;
    }
    let (rc, timeline) = fetch_parser_live(m, handle, 0x3f2, 1, env, host);
    if rc >= 0x80000000 {
        return if rc & 0x1fff == 20 { 0 } else { rc };
    }
    let text_bytes = (u32::from(half(m, timeline + 8)) * u32::from(half(m, timeline + 2))) as u16;
    let pair = [m.word(ctx), m.word(ctx + 4)];
    let mut it = 0;
    let rc = normalization::open_iterator(
        m.bytes,
        pair[0],
        pair[1],
        handle,
        0x3f3,
        Some(&mut it),
        &mut LiveFetch {
            host: &mut *host,
            env: &env.engine,
        },
    );
    if rc >= 0x80000000 {
        return rc;
    }
    let mut list = 0;
    let rc = normalization::field_value(Some(&m.bytes[it as usize..]), Some(&mut list));
    if rc >= 0x80000000 {
        return rc;
    }
    let mut base = m.word(m.word(list + 4) + 20);
    let limit = base.wrapping_add(u32::from(text_bytes));
    let mut saved = [0u8; 160];
    saved.copy_from_slice(&m.bytes[stack as usize..stack as usize + 160]);
    let mut d4 = 0;
    let mut bc = 0;
    let mut index = 0;
    let mut early = false;
    let rc = (|| {
        let rc = crate::render_part_c2::iterator_seek(m, it, 0);
        if rc >= 0x80000000 {
            return rc;
        }
        let arr = m.word(it + 12);
        let data = m.word(arr + 4);
        let mut idx = half(m, it + 16);
        let rc = normalization::scan_forward(
            &m.bytes[data as usize..],
            half(m, arr + 2),
            half(m, arr + 8),
            &mut idx,
            &mut |event| crate::text_input::keys_equal(Some(event), Some(&seg.first)) as i32,
        );
        m.put_half(it + 16, idx);
        if rc >= 0x80000000 {
            return rc;
        }
        let (rc, first) = crate::render_part_c2::iterator_get(m, it, 0xffff);
        if rc >= 0x80000000 {
            return rc;
        }
        let field = m.word(first + 20);
        let pos = m.word(ctx + 84).max(base);
        m.put_word(ctx + 84, pos);
        m.put_word(ctx + 92, pos);
        d4 = frontend::divmod_u32(pos.wrapping_sub(base), u32::from(half(m, timeline + 2))).0;
        bc = d4;
        let mut rc =
            crate::text_input::timeline_window(m.bytes, timeline, it, &mut bc, &mut d4, &mut index);
        if rc >= 0x80000000 {
            return rc;
        }
        if d4 != 0 && d4 <= u32::from(half(m, timeline + 8)) {
            let mut c = d4 as u16;
            if half(m, timeline + 8) == c {
                c = c.wrapping_sub(1);
            }
            m.put_half(stack + 112, c);
            normalization::skip_matched_back(
                m.bytes,
                m.word(timeline + 4),
                stack + 112,
                seg.separators,
            );
            if half(m, stack + 112) == 0 {
                early = true;
                m.put_word(
                    ctx + 84,
                    d4.wrapping_mul(u32::from(half(m, timeline + 2)))
                        .wrapping_add(base),
                );
                return rc;
            }
        }
        if d4 <= bc {
            return rc;
        }
        let mut b4 = bc as u16;
        let mut b8 = d4 as u16;
        let mut sent = false;
        loop {
            if b4 != b8 {
                let mut mode = flag4arg as u16;
                rc = crate::render_part_c2::segment_scan(m, ctx, list, &mut b4, &mut b8, &mut mode);
                if rc >= 0x80000000 {
                    return rc;
                }
                if b4 != b8 && mode == 1 && flag3 != 0 {
                    sent = true;
                    rc = crate::render_part_c2::segment_send(
                        m,
                        ctx,
                        timeline,
                        field,
                        u32::from(b4),
                        u32::from(b8),
                        it,
                        host,
                    );
                    if rc >= 0x80000000 {
                        return rc;
                    }
                    b4 = b8;
                    b8 = d4 as u16;
                    if b4 != b8 {
                        continue;
                    }
                }
            }
            b4 = bc as u16;
            b8 = d4 as u16;
            rc = crate::text_input::timeline_window(
                m.bytes, timeline, it, &mut bc, &mut d4, &mut index,
            );
            if rc >= 0x80000000 {
                return rc;
            }
            if u32::from(b4) >= bc || u32::from(b8) >= d4 {
                bc = d4;
            }
            if d4
                .wrapping_mul(u32::from(half(m, timeline + 2)))
                .wrapping_add(base)
                > limit
            {
                bc = d4;
            }
            if d4 > bc {
                b4 = bc as u16;
                b8 = d4 as u16;
                continue;
            }
            if sent {
                // The ARM calls split_input when a send happened (C
                // fn_2c0014: `if (flag78 == 1) rc = split_input(...)`).
                let split_rc = segment_split_live(m, ctx, timeline, base, d4, it, index as u16, stack, seg, env, host, snap);
                if split_rc >= 0x80000000 {
                    return split_rc;
                }
                rc = split_rc;
            }
            return rc;
        }
    })();
    if !early {
        let mut c = d4 as u16;
        if c != 0 {
            if half(m, timeline + 8) == c {
                c = c.wrapping_sub(1);
            }
            if c <= half(m, timeline + 8) {
                m.put_half(stack + 112, c);
                normalization::skip_matched_back(
                    m.bytes,
                    m.word(timeline + 4),
                    stack + 112,
                    seg.separators,
                );
                c = half(m, stack + 112);
            }
        }
        if u32::from(c) != d4 {
            m.put_word(
                ctx + 88,
                u32::from(c)
                    .wrapping_mul(u32::from(half(m, timeline + 2)))
                    .wrapping_add(base),
            );
        }
        if d4 > 4 {
            base = base.wrapping_sub(4);
        }
        m.put_word(
            ctx + 84,
            d4.wrapping_mul(u32::from(half(m, timeline + 2)))
                .wrapping_add(base),
        );
    }
    if it != 0 {
        normalization::release_iterator(
            m.bytes,
            it,
            &mut LiveFetch {
                host: &mut *host,
                env: &env.engine,
            },
        );
    }
    m.bytes[stack as usize..stack as usize + 160].copy_from_slice(&saved);
    rc
}

/// Memory-passing allocator for sentence splitting. The allocator must not
/// capture the arena. All callbacks receive the caller's current bytes last.
pub trait ParserAllocatorM: frontend::HeapVtableM {
    fn calloc_m(&mut self, object: u32, count: u32, size: u32, memory: &mut [u8]) -> u32;
}
struct FetchHostM<'a, A> {
    allocator: &'a mut A,
    env: &'a crate::render_part_c::Environment,
}
impl<A: ParserAllocatorM> normalization::NormalizationHost for FetchHostM<'_, A> {
    fn resolve(&mut self, bytes: &mut [u8], a: u32, b: u32) -> (u32, u32) {
        crate::render_part_c2::resolve(&Memory { base: 0, bytes }, [a, b], self.env)
    }
    fn calloc(&mut self, bytes: &mut [u8], heap: u32, n: u32, size: u32) -> Option<u32> {
        if heap == 0 || n == 0 || size == 0 {
            return None;
        }
        let object = Memory { base: 0, bytes }.word(heap + 4);
        nz(self.allocator.calloc_m(object, n, size, bytes))
    }
    fn alloc(&mut self, bytes: &mut [u8], heap: u32, size: u32) -> Option<u32> {
        nz(frontend::heap_alloc_m(
            bytes,
            nz(heap),
            size,
            self.allocator,
        ))
    }
    fn realloc(&mut self, bytes: &mut [u8], heap: u32, p: u32, size: u32) -> Option<u32> {
        nz(frontend::heap_realloc_m(
            bytes,
            nz(heap),
            p,
            size,
            self.allocator,
        ))
    }
    fn free(&mut self, bytes: &mut [u8], heap: u32, p: u32) {
        frontend::heap_release_m(bytes, nz(heap), nz(p), self.allocator)
    }
}

fn fetch_parser_live(
    m: &mut Memory<'_>,
    handle: u32,
    kind: u32,
    count: u32,
    env: &crate::render_part_c2::Environment<'_>,
    host: &mut impl crate::render_part_c2::ContextHost,
) -> (u32, u32) {
    let mut array = 0;
    let rc = normalization::fetch_entry(
        m.bytes,
        handle,
        kind,
        count,
        Some(&mut array),
        &mut LiveFetch {
            host,
            env: &env.engine,
        },
    );
    (rc, array)
}

/// The ARM's parameter lock around a live lookup: acquire through the
/// dispatcher's lock path, run the lookup with the held lock, then release
/// through the same path. This is the live form of the `fe_lock`/`Locked`
/// pair the boot ports use around `lookup_param`: the ARM's fn_1756d0 and
/// fn_175944 call fn_176f2c/fn_176f5c on the params object's lock word, and
/// those entries call nothing when the inner dispatch slot is zero.
fn live_lookup_lock(m: &mut Memory<'_>, owner: u32, release: bool, h: &mut impl LiveRenderHost) -> u32 {
    let mut selected = None;
    let rc = if release {
        crate::frontend::release_lock(m.bytes, nz(owner), |arg| { selected = Some(arg); 0 })
    } else {
        crate::frontend::acquire_lock(m.bytes, nz(owner), |arg| { selected = Some(arg); 0 })
    };
    if let Some(arg) = selected {
        let off = if release { 12 } else { 8 };
        h.existing_port(m, m.word(m.word(m.word(owner) + 8) + off), &[arg])
    } else {
        rc
    }
}

/// The already-held parameter lock: the real acquire/release runs around
/// the lookup through `live_lookup_lock`, so the lookup itself is a no-op.
struct HeldLock;
impl crate::frontend::StateLock for HeldLock {
    fn acquire(&mut self) -> u32 { 0 }
    fn release(&mut self) -> u32 { 0 }
}

fn block_call_m(m:&mut Memory<'_>,f:u32,a:&[u32],e:&crate::render_part_b::BlockEnvironment<'_>,scratch:u32,h:&mut impl LiveRenderHost)->u32 {
    match f {
        0x2c0f74=>h.parse(m,a),
        0x2c322c=>h.pronounce(m,a),
        0x2bfa90=>h.parser_mode(m,a),
        0x2c4914=>h.escape(m,a),
        0x2c0014=>h.segment(m,a),
        0x252f88=>{
            let mut p=0;
            let rc=crate::normalization::fetch_entry(m.bytes,a[0],a[1],a[2],Some(&mut p),
                &mut LiveFetch{host:&mut *h,env:&e.lifecycle.engine});
            if rc<0x80000000 {m.put_word(a[3],p);}rc
        }
        0x263384=>crate::render_part_b::fn_263384(m,a[0] as i32,a[1],a[2],a[3],a[4],e.log,h),
        0x2634c0=>crate::render_part_b::fn_2634c0(m,a[0] as i32,a[1],a[2],a[3],a[4],a[5] as i32,a[6],a[7],e.log,h),
        0x2c47d0=>crate::render_part_c::fn_2c47d0(m,a[0],a[1],&e.lifecycle.engine,h),
        0x2c4728=>crate::render_part_c::fn_2c4728(m,a[0],a[1]),
        0x2c4750=>{
            if a[0]==0 {crate::text_input::mark_flag(None)}
            else {let at=m.offset(a[0]);crate::text_input::mark_flag(Some(&mut m.bytes[at..]))}
        }
        0x2c2408=>{
            let at=m.offset(a[0]);crate::text_input::normalize_window(&mut m.bytes[at..])
        }
        0x2c242c=>crate::normalization::triple_zero(m.bytes,a[0]),
        0x2c4768=>crate::normalization::value_fetch(m.bytes,a[0],a[1]),
        0x1756d0=>{
            let mut p=0;
            let owner=if a[0]==0 {0} else {m.word(a[0]+4)};
            let arc=if a[0]==0 {0} else {live_lookup_lock(m,owner,false,h)};
            if arc>=0x80000000 {arc} else {
                let rc=crate::frontend::lookup_param(m.bytes,nz(a[0]),a[1],Some(&mut p),None,&mut HeldLock,|x,y|crate::normalization::strcmp_at(m.bytes,x,y));
                if a[2]!=0 {m.put_word(a[2],p);}
                let rc2=if a[0]==0 {0} else {live_lookup_lock(m,owner,true,h)};
                if rc2>=0x80000000 && rc<0x80000000 {rc2} else {rc}
            }
        }
        0x175944=>{
            let mut p=0;let mut size=0;
            let owner=if a[0]==0 {0} else {m.word(a[0]+4)};
            let arc=if a[0]==0 {0} else {live_lookup_lock(m,owner,false,h)};
            if arc>=0x80000000 {arc} else {
                let rc=crate::frontend::lookup_param(m.bytes,nz(a[0]),a[1],Some(&mut p),Some(&mut size),&mut HeldLock,|x,y|crate::normalization::strcmp_at(m.bytes,x,y));
                let value=if rc<0x80000000 && size>1 {crate::frontend::parse_uint(m.bytes,nz(p)) as u32} else {0};
                m.put_word(a[2],value);
                let rc2=if a[0]==0 {0} else {live_lookup_lock(m,owner,true,h)};
                if rc2>=0x80000000 && rc<0x80000000 {rc2} else {rc}
            }
        }
        0x17497c=>crate::normalization::flag_byte(m.bytes,a[0]),
        0x17486c=>{
            if crate::normalization::flag_byte(m.bytes,a[0])!=0 {
                crate::render_part_b::fn_1741dc(m,a[0],a[1],a[2],scratch, scratch+64,h)
            } else {0}
        }
        0x26a610=>crate::render_part_b2::fn_26a610(m,a[0],a[1],e.marker,h),
        0x2c3e78=>crate::text_input::split_spans(m.bytes,a[0],a[1],a[2],a[3],scratch,e.normalizer_defaults,
            &mut LiveFetch{host:&mut *h,env:&e.lifecycle.engine}),
        0x2bb4c8=>crate::render_part_c::fn_2bb4c8(m,a[0],a[1],h),
        _=>h.existing_port(m,f,a),
    }
}

fn block_text(m:&mut Memory<'_>,o:u32,ctx:u32,sp:u32,tag:u32,label:usize,e:&crate::render_part_b::BlockEnvironment<'_>,h:&mut impl crate::render_part_b::BlockHost) {
    crate::render_part_b::fn_263384(m,m.byte(o+0x78) as i32,ctx,sp,tag,e.labels[label],e.log,h);
}
fn block_mark(m:&mut Memory<'_>,o:u32,ctx:u32,sp:u32,tag:u32,label:usize,e:&crate::render_part_b::BlockEnvironment<'_>,h:&mut impl crate::render_part_b::BlockHost) {
    crate::render_part_b::fn_2634c0(m,m.byte(o+0x78) as i32,ctx,sp,tag,e.labels[label],0,0,0,e.log,h);
}
/// fn_2645b4. Pull, escape-process, parse, normalize and rewrite one block.
/// scratch is 320 caller-owned guest bytes, including formatter locals.
pub fn fn_2645b4_m(m:&mut Memory<'_>,o:u32,sp:u32,param3:u32,e:&crate::render_part_b::BlockEnvironment<'_>,scratch:u32,h:&mut impl LiveRenderHost)->u32 {
    let mut saved=[0;320];let at=m.offset(scratch);saved.copy_from_slice(&m.bytes[at..at+320]);
    let result=(|| {
    if o==0 || sp==0 {return 0x81102007;}
    let (rc,ctx)=crate::render_part_b::lifecycle_context(m,o,e.lifecycle);
    if rc>=0x80000000 {return 0x81102007;}
    let s=scratch;let flag50=s;let v48=s+4;let v44=s+8;let b4c=s+12;let s70=s+16;
    let v64=s+20;let v54=s+24;let mrk=s+32;let v8=s+64;let out=s+68;
    byte(m,flag50,1);byte(m,b4c,0);m.put_half(mrk+10,0);
    for at in [v48,v44,s70,v64,v54,v8] {m.put_word(at,0);}
    macro_rules! call {($f:expr,$a:expr)=>{block_call_m(m,$f,$a,e,s+96,h)};}
    if m.byte(o+0x81)!=0 {
        byte(m,o+0x18,0);for off in [0x1c,0x20,0x24] {m.put_word(o+off,0);}
        byte(m,o+0x28,0);byte(m,o+0x29,0);m.put_word(out,0);
        let rc=call!(0x2c47d0,&[m.word(o+0x34),out]);if rc>=0x80000000 {return rc;}
    }
    let active=m.byte(o+0x18) as u32!=0;
    if !active {
        for tag in [0x3f3,0x3fd] {
            m.put_word(out,0);
            let rc=call!(0x252f88,&[sp,tag,1,out]);
            if rc<0x80000000 {
                let el=m.word(out);let mk=m.word(el+4);let mut i=1u16;
                while i<half(m,el+8) {
                    let p=mk+i as u32*40+32;
                    if m.word(p)!=0 {live_free(m,m.word(ctx+4),m.word(p),h);m.put_word(p,0);}
                    i=i.wrapping_add(1);
                }
            }
        }
    }
    let tags=if active {&e.clear_active[..]} else {&e.clear_all[..]};
    for &tag in tags {
        m.put_word(out,0);
        let rc=call!(0x252f88,&[sp,tag as u32,1,out]);
        if rc<0x80000000 && tag!=0x3f4 {m.put_half(m.word(out)+8,0);}
    }
    byte(m,s+72,0);
    let method=m.word(m.word(o+8)+0x48);
    let rc=crate::boot_part_e::Host::call(h,m,method,&[m.word(o+12),m.word(o+16),v8,s+72]);
    if rc>=0x80000000 {return rc;}
    if m.byte(s+72)!=0 && m.word(v8)==0 {return 0x81102017;}
    if half(m,o+0x30)==0 && m.word(ctx+20)!=0 {
        if call!(0x1756d0,&[m.word(ctx+20),e.readmode,s70,0])<0x80000000 {
            m.put_half(o+0x30,(m.byte(m.word(s70)) as i32-48) as u16);
        }
    }
    let mut rc=call!(0x2bfa90,&[m.word(o+0x38),sp,flag50,o+0x30]);
    if rc>=0x80000000 {return rc;}
    while m.byte(flag50)==0 {
        let mut rc=0;
        let step=(||->u32 {
            if m.byte(o+0x18) as u32==0 || (m.byte(o+0x29)==0 && m.byte(o+0x28)==1) {
                let old=m.word(o+0x1c);let method=m.word(m.word(o+8)+0x2c);
                rc=crate::boot_part_e::Host::call(h,m,method,&[m.word(o+12),m.word(o+16),o+0x14,o+0x1c]);
                if rc>=0x80000000 {return rc;}
                if m.byte(o+0x18) as u32==0 {call!(0x1748a0,&[m.word(ctx+16),0x20,e.labels[0],e.labels[1],m.word(o+0x14),m.word(o+0x1c)]);}
                byte(m,o+0x18,1);
                render_log(m,ctx,e.log,e.labels[2],&[RenderArgument::Word(m.word(o+0x1c))],h);
                m.put_word(out,0);
                let rc2=call!(0x175944,&[m.word(ctx+20),e.resetlen,out]);
                if rc2&0x1fff==0x14 {m.put_word(out,0);} else if rc2>=0x80000000 {return rc;}
                render_log(m,ctx,e.log,e.labels[3],&[RenderArgument::Word(m.word(out))],h);
                if m.word(out)!=0 {
                    m.put_word(s+76,0);
                    let method=m.word(m.word(o+8)+0x44);
                    rc=crate::boot_part_e::Host::call(h,m,method,&[m.word(o+12),m.word(o+16),s+76]);
                    if rc>=0x80000000 {return rc;}
                    render_log(m,ctx,e.log,e.labels[4],&[RenderArgument::Word(m.word(s+76))],h);
                    if m.byte(o+0x28)==0 {
                        rc=call!(0x2c4728,&[m.word(o+0x34),m.word(s+76)]);if rc>=0x80000000 {return rc;}
                    }
                } else {
                    rc=call!(0x2c4750,&[m.word(o+0x34)]);if rc>=0x80000000 {return rc;}
                    if half(m,o+0x30)==0 && m.word(ctx+20)!=0 && call!(0x1756d0,&[m.word(ctx+20),e.readmode,s70,0])<0x80000000 {
                        m.put_half(o+0x30,(m.byte(m.word(s70)) as i32-48) as u16);
                    }
                }
                if m.byte(o+0x28)==1 && old!=0 && m.word(o+0x1c)==old {byte(m,o+0x29,1);}else{byte(m,o+0x28,0);}
            }
            rc=call!(0x252f88,&[sp,0x3e8,1,out]);if rc>=0x80000000 {return rc;}
            let el=m.word(out);
            if m.byte(el+12)!=0 {
                if m.word(o+0x1c)!=0 {
                    let method=m.word(m.word(o+8)+0x30);
                    rc=crate::boot_part_e::Host::call(h,m,method,&[m.word(o+12),m.word(o+16),m.word(o+0x1c)]);
                    if rc>=0x80000000 {return rc;}
                }
                let rc=crate::render_part_c::fn_2b92d8(m,ctx,e.lifecycle.parameter_pairs,e.lifecycle.parameter_triple,&mut LiveLifecycleCalls{host:h,scratch:s+96});
                if rc>=0x80000000 {return rc;}
                return 0x81102017;
            }
            let diff=m.word(o+0x1c).wrapping_sub(m.word(o+0x20).wrapping_mul(2));
            if diff<=1 {
                if m.byte(o+0x29)!=0 || m.byte(o+0x28)!=0 {byte(m,el+12,1);}
                else {
                    byte(m,o+0x28,1);
                    if m.byte(o+0x81)!=0 && m.word(o+0x1c)!=0 {
                        let method=m.word(m.word(o+8)+0x30);
                        rc=crate::boot_part_e::Host::call(h,m,method,&[m.word(o+12),m.word(o+16),m.word(o+0x1c)]);
                        if rc>=0x80000000 {return rc;}
                    }
                    return 0x81102800;
                }
            } else {
                let n=if m.byte(o+0x81)!=0 {diff as u16 as u32} else {0x80};
                rc=call!(0x252f88,&[sp,0x3e8,n,out]);if rc>=0x80000000 {return rc;}
                if half(m,m.word(out)+8)!=0 {
                    let count=half(m,m.word(out)+8) as u32;
                    rc=call!(0x252f88,&[sp,0x3e8,(count+n) as u16 as u32,out]);if rc>=0x80000000 {return rc;}
                }
                let el=m.word(out);let count=half(m,el+8) as u32;
                copy_bytes(m,m.word(el+4)+count*2,m.word(o+0x14)+m.word(o+0x20)*2,diff.min(n*2));
                let used=if diff>n*2 {n} else {diff/2};
                add_word(m,o+0x20,used);m.put_half(el+8,(count+used) as u16);
                rc=call!(0x2c2408,&[m.word(o+0x38)]);
            }
            rc
        })();
        if step>=0x80000000 {return step;}
        block_text(m,o,ctx,sp,0x3e8,5,e,h);block_mark(m,o,ctx,sp,0x3f3,6,e,h);block_text(m,o,ctx,sp,0x3f2,7,e,h);
        let r4=call!(0x2c4914,&[m.word(o+0x34),sp]);
        block_mark(m,o,ctx,sp,0x3f3,8,e,h);block_text(m,o,ctx,sp,0x3f2,9,e,h);
        if r4&0x1fff==9 {continue;}if r4>=0x80000000 {return r4;}
        block_mark(m,o,ctx,sp,0x3f3,10,e,h);block_mark(m,o,ctx,sp,0x3fd,11,e,h);
        block_text(m,o,ctx,sp,0x3f2,10,e,h);block_text(m,o,ctx,sp,0x3fc,10,e,h);
        let rc=call!(0x2c0014,&[m.word(o+0x38),sp,m.byte(o+0x76) as u32,half(m,o+0x30) as i16 as i32 as u32]);
        if rc>=0x80000000 {return rc;}
        block_mark(m,o,ctx,sp,0x3f3,12,e,h);
        if m.byte(o+0x81)!=0 {byte(m,flag50,1);break;}
        let rc=call!(0x2bfa90,&[m.word(o+0x38),sp,flag50,o+0x30]);if rc>=0x80000000 {return rc;}
    }
    block_mark(m,o,ctx,sp,0x3f3,13,e,h);m.put_word(v44,0);m.put_word(v48,0);
    rc=call!(0x2c0f74,&[m.word(o+0x38),sp,v44,v48,b4c,param3,o+0x30,v64,v54,mrk]);
    if rc>=0x80000000 {return rc;}
    let mut name=m.word(e.log.marker_names+4);
    for k in 0..40 {if half(m,e.log.marker_names+k*8)==half(m,mrk+10) {name=m.word(e.log.marker_names+k*8+4);break;}}
    render_log(m,ctx,e.log,e.marker_log,&[RenderArgument::Word(half(m,o+0x30) as u32),RenderArgument::Word(m.word(v64)),RenderArgument::Word(m.word(v54)),RenderArgument::Guest(name)],h);
    if call!(0x17497c,&[m.word(ctx+16)])!=0 {call!(0x17486c,&[m.word(ctx+16),0x21,0]);}
    block_mark(m,o,ctx,sp,0x3f3,14,e,h);block_text(m,o,ctx,sp,0x3fc,14,e,h);block_mark(m,o,ctx,sp,0x3fd,15,e,h);
    let save0=m.word(v44);let save1=m.word(v48);let reset=m.byte(b4c);
    rc=call!(0x252f88,&[sp,0x3fc,1,out]);
    if rc<0x80000000 {
        let el=m.word(out);let count=half(m,el+8) as u32;
        if count!=0 && m.byte(o+0x78)==1 {
            let n=count.min(0x400);let buf=call_method_m(m.bytes,nz(m.word(ctx+4)),1,n*2+2,h);
            if buf==0 {return 0x8110200a;}
            narrow(m,buf,m.word(el+4),n);byte(m,buf+n,0);
            render_log(m,ctx,e.log,e.labels[16],&[RenderArgument::Word(0x3fc),RenderArgument::Word(count),RenderArgument::Guest(buf)],h);
            live_free(m,m.word(ctx+4),buf,h);
        }
        add_word(m,o+0x24,save0.wrapping_add(save1).wrapping_mul(half(m,el+2) as u32));
        if reset==1 || m.word(o+0x24)>m.word(o+0x2c) {
            let method=m.word(m.word(o+8)+0x30);
            rc=crate::boot_part_e::Host::call(h,m,method,&[m.word(o+12),m.word(o+16),m.word(o+0x24)]);
            if rc<0x80000000 {
                rc=call!(0x2c4768,&[m.word(o+0x34),v8]);
                if rc<0x80000000 {
                    rc=call!(0x2c1e2c,&[m.word(o+0x38),sp]);
                    if rc<0x80000000 && half(m,o+0x30)!=1 {rc=call!(0x1758ac,&[m.word(ctx+20),e.mode_key,half(m,o+0x30) as i16 as i32 as u32]);}
                    if rc<0x80000000 {rc=call!(0x2c242c,&[m.word(o+0x38)]);}
                    if rc<0x80000000 {rc=call!(0x1758ac,&[m.word(ctx+20),e.position_key,m.word(o+0x24)]);}
                    if rc<0x80000000 {
                        byte(m,o+0x18,0);for off in [0x1c,0x20,0x14,0x24] {m.put_word(o+off,0);}
                        byte(m,o+0x28,0);byte(m,o+0x29,0);
                        rc=call!(0x252f88,&[sp,0x3e8,1,out]);if rc<0x80000000 {byte(m,m.word(out)+12,0);}
                    }
                }
            }
        }
    }
    if rc>=0x80000000 {return rc;}
    if m.byte(param3)==1 {render_log(m,ctx,e.log,e.labels[17],&[],h);return rc;}
    if m.byte(b4c)==1 {render_log(m,ctx,e.log,e.labels[18],&[],h);}
    if m.byte(o+0x77)!=0 {
        let rc2=call!(0x252f88,&[sp,0x3f4,1,out]);if rc2<0x80000000 {m.put_half(o+0x74,half(m,m.word(out)+8));}
        rc=call!(0x26a610,&[sp,0x3f3]);if rc>=0x80000000 {return rc;}
        block_mark(m,o,ctx,sp,0x3fd,19,e,h);
        let r4=call!(0x2baf40,&[m.word(o+0x3c),sp,m.word(v64),m.word(v54),mrk]);
        block_mark(m,o,ctx,sp,0x3fd,20,e,h);if r4>=0x80000000 {return r4;}
    } else {
        rc=call!(0x2c3e78,&[m.word(o+0x3c),sp,m.word(v64),m.word(v54)]);if rc>=0x80000000 {return rc;}
    }
    block_text(m,o,ctx,sp,0x3fc,21,e,h);block_mark(m,o,ctx,sp,0x3fd,21,e,h);
    let rc2=call!(0x252f88,&[sp,0x3fd,1,out]);
    if rc2>=0x80000000 {return rc2;}
    let el=m.word(out);let mk=m.word(el+4);let mut i=1u16;
    while i<half(m,el+8) {
        let at=mk+i as u32*40;
        if half(m,at+10)==7 && m.word(at+28)>=0x46 {
            let cur=m.word(at+20);i=i.wrapping_add(1);
            while i<half(m,el+8) && half(m,mk+i as u32*40+10)!=7 {i=i.wrapping_add(1);}
            let end=if i>=half(m,el+8) {(m.word(mk+20)+m.word(mk+24)) as u16 as u32}else{m.word(mk+i as u32*40+20)};
            let mut lo=0u16;let mut hi=0u16;let mut k=1u16;
            while k<half(m,el+8) && m.word(mk+k as u32*40+20)<=end {
                let p=mk+k as u32*40;let pos=m.word(p+20);
                if half(m,p+10)==1 && pos>=cur && pos<end {if lo==0 {lo=k;}hi=k;}
                k=k.wrapping_add(1);
            }
            if lo!=0 && hi!=0 && lo!=hi {
                let first=mk+lo as u32*40;let last=mk+hi as u32*40;
                m.put_word(first+24,m.word(last+20).wrapping_add(m.word(last+24)).wrapping_sub(m.word(first+20)));
                m.put_word(first+16,m.word(last+12).wrapping_add(m.word(last+16)).wrapping_sub(m.word(first+12)));
                while hi>lo {
                    let p=mk+hi as u32*40;let pos=m.word(p+20);
                    if half(m,p+10)==1 && pos>=cur && pos<end {
                        let n=half(m,el+8);
                        if hi.wrapping_add(1)!=n {move_bytes(m,p,p+40,(n-hi) as u32*40);}
                        m.put_half(el+8,n.wrapping_sub(1));
                    }
                    hi-=1;
                }
            }
        } else {i=i.wrapping_add(1);}
    }
    block_text(m,o,ctx,sp,0x3fc,22,e,h);block_mark(m,o,ctx,sp,0x3fd,22,e,h);
    rc=call!(0x2c322c,&[m.word(o+0x40),sp]);if rc>=0x80000000 {return rc;}
    block_text(m,o,ctx,sp,0x3fc,23,e,h);block_mark(m,o,ctx,sp,0x3fd,23,e,h);
    if m.byte(o+0x77)!=0 {rc=call!(0x2bb4c8,&[m.word(o+0x3c),sp]);if rc>=0x80000000 {return rc;}}
    rc=call!(0x252f88,&[sp,0x3fd,1,out]);if rc>=0x80000000 {return rc;}
    let el=m.word(out);let mk=m.word(el+4);let mut w=1u16;
    for k in 1..half(m,el+8) {
        if half(m,mk+k as u32*40+10)!=0x4000 {move_bytes(m,mk+w as u32*40,mk+k as u32*40,40);w=w.wrapping_add(1);}
    }
    m.put_half(el+8,w);
    if m.byte(o+0x81)!=0 && m.word(o+0x1c)!=0 {
        let method=m.word(m.word(o+8)+0x30);
        rc=crate::boot_part_e::Host::call(h,m,method,&[m.word(o+12),m.word(o+16),m.word(o+0x1c)]);
        if rc>=0x80000000 {return rc;}
    }
    0
    })();
    m.bytes[at..at+320].copy_from_slice(&saved);result
}

// ---- the live render host ------------------------------------------------

/// The engine's guest dispatcher, reached with the caller's arena.
///
/// The render chain calls the engine's own entries: the text stream's four
/// methods, the guest heap's four methods and the engine's property table.
/// The live host dispatches them through the engine's own dispatcher, so the
/// guest sees the caller's arena and the guest's own code runs.
pub trait LiveDispatch {
    fn dispatch(&mut self, m: &mut Memory<'_>, entry: u32, args: &[u32]) -> u32;
    fn thread_id(&mut self) -> u32;
}

impl<D: LiveDispatch + ?Sized> LiveDispatch for &mut D {
    fn dispatch(&mut self, m: &mut Memory<'_>, entry: u32, args: &[u32]) -> u32 {
        (**self).dispatch(m, entry, args)
    }
    fn thread_id(&mut self) -> u32 {
        (**self).thread_id()
    }
}

/// The live render host: the engine's dispatcher and the render chain's own
/// constants. The constants are read out of the caller's arena.
pub struct LiveRender<'s, D> {
    pub dispatch: D,
    /// The arena image taken before the request ran. Property keys are
    /// immutable, so the escape comparator reads them from here without
    /// aliasing the live arena the ports mutate.
    pub snap: &'s [u8],
    pub engine: crate::render_part_c::Environment,
    pub log: crate::render_part_b::RenderEnvironment,
    pub marker: crate::render_part_b2::Environment,
    pub lifecycle: crate::render_part_b::LifecycleEnvironment,
    /// The lock owners the render chain selected. The ports name the lock
    /// token through `select_lock` before acquiring; the live acquire is a
    /// no-op and `run_block` audits every selected owner afterwards, so a
    /// set inner dispatch slot panics instead of diverging silently.
    pub lock_owners: [u32; 8],
    pub lock_used: [bool; 8],
    pub lock_count: u32,
    pub lock_current: u32,
    pub lock_current_set: bool,
    pub lock_untracked: bool,
    /// Dispatch nesting, for the runaway guard in `call`.
    pub depth: u32,
}

/// The block function's host: the engine's dispatcher and the four parser
/// ports the block function reaches.
pub trait LiveRenderHost: crate::render_part_b::BlockHost {
    /// fn_2c0f74, the parser.
    fn parse(&mut self, m: &mut Memory<'_>, args: &[u32]) -> u32;
    /// fn_2bfa90, the parser-mode chooser.
    fn parser_mode(&mut self, m: &mut Memory<'_>, args: &[u32]) -> u32;
    /// fn_2c4914, the escape-sequence processor.
    fn escape(&mut self, m: &mut Memory<'_>, args: &[u32]) -> u32;
    /// fn_2c0014, the parser-mode window scan.
    fn segment(&mut self, m: &mut Memory<'_>, args: &[u32]) -> u32;
    /// fn_2c322c, the pronunciation conversion.
    fn pronounce(&mut self, m: &mut Memory<'_>, args: &[u32]) -> u32;
}

/// The ROM constants of the parser chain. Every address is a ROM address or
/// a ROM record, read out of the captured image.
pub mod parser_rom {
    /// D_9e0990: the separator set, tab, line feed, carriage return, space
    /// and the 0xfeff marker. The engine copies it at run time; the live
    /// ports take a copy so the arena stays writable.
    pub const SEPARATORS: u32 = 0x9e0990;
    /// D_9c5aa6: the space set the escape scanner skips.
    pub const SPACES: u32 = 0x9c5aa6;
    /// D_9c5a28: the parser defaults, the span template at +0 and the
    /// following event at +0x8c.
    pub const DEFAULTS: u32 = 0x9c5a28;
    /// D_9c59f4: the span defaults fn_2c3e78 splits on: six separator
    /// units (tab, line feed, carriage return, space, 0xfeff, zero) then
    /// the 40-byte template event. Distinct from the 0x9c5a28 parser
    /// defaults (whose separators are "__NUAN").
    pub const SPAN_DEFAULTS: u32 = 0x9c59f4;
    /// D_9c5a44: the four parameter defaults, a u16 type and a name each.
    pub const PARAMETERS: u32 = 0x9c5a44;
    /// D_2be484: "maxinputlength", the segment length parameter.
    pub const MAX_LENGTH: u32 = 0x2be484;
    /// "audio", the escape command the ARM's own code computes with
    /// `add r1, pc, #0x3a8` at 0x2c4f88.
    pub const AUDIO: u32 = 0x2c5338;
    /// "mrk", the escape command the ARM's own code computes with
    /// `add r1, pc, #0x398` at 0x2c4fa0.
    pub const MARKER: u32 = 0x2c5340;
    /// D_8dc30c: the character class table the integer parser uses.
    pub const NUMBER_TABLE: u32 = 0x8dc30c;
}

/// The ROM constants of the pronunciation chain. Every address is proven by
/// the ARM's own literal loads in build/base/fn_2c3450.o (or fn_2eca2c.o
/// for the registry name); the field offsets are proven by the loads'
/// displacements. See rust/evidence/rust-wire-a-round15.progress.txt.
pub mod pron_rom {
    /// D_9c59c0: the 12-byte nts prefix (`strcpy` source at fn_2c3450.o:0x168).
    pub const NTS_PREFIX: u32 = 0x9c59c0;
    /// D_9c59c5: the ipa prefix (`strcpy` source at fn_2c3450.o:0x1a8).
    pub const IPA_PREFIX: u32 = 0x9c59c5;
    /// D_9c59c0+12: the language filter, 8 bytes = 4 u16 (`ldrd [r6,#12]`).
    pub const LANGUAGE_FILTER: u32 = 0x9c59cc;
    /// D_9c59c0+36: the default language, 16 bytes = 8 u16
    /// (`add r3,r1,#36` + `ldm` + 2x `ldr`).
    pub const DEFAULT_LANGUAGE: u32 = 0x9c59e4;
    /// D_9c59d4: the stop filter, 8 bytes = 4 u16 (`ldm r0,{r1,r2}`).
    pub const STOP_FILTER: u32 = 0x9c59d4;
    /// D_9c59da: the trim set, a NUL-terminated u16 string scanned by
    /// fn_176514 (`u16chr`). Copied out to a NUL-terminated local.
    pub const TRIM: u32 = 0x9c59da;
    /// D_2c31e8: the property name `fetch_param` looks up in `setLanguage`
    /// (fn_2c3450.o:0x21c loads the literal for fn_1756d0).
    pub const LANGCODE_KEY: u32 = 0x2c31e8;
    /// The dictionary registry name. fn_2ecaac passes "FE_DCTLKP" from a
    /// literal at section+0x74 (fn_2eca2c.o:0x138 `sub r1,pc,#204`),
    /// linked at 0x2ecaa0.
    pub const REGISTRY_NAME: u32 = 0x2ecaa0;
    /// The UTF-8 decoder tables (same windows slice2b and engine.rs use).
    pub const DECODER_EXTRA: u32 = 0x8dc1f3;
    pub const DECODER_MASK: u32 = 0x8dc1e0;
    pub const DECODER_BASE: u32 = 0x8dc2f4;
}

/// The disjoint scratch the live render chain uses. Every one of these areas
/// is restored before the call that uses it returns.
pub mod scratch {
    /// fn_2645b4's own 320 bytes, including the formatter's locals.
    pub const BLOCK: u32 = 0x6000;
    /// fn_2bfa90's 1200 bytes.
    pub const PARSER: u32 = 0x6200;
    /// fn_2c0014's 160 bytes.
    pub const SEGMENT: u32 = 0x6b00;
    /// fn_2c4914's 256 bytes.
    pub const ESCAPE: u32 = 0x6c00;
    /// The parser runtime's 64 bytes.
    pub const RUNTIME: u32 = 0x6d00;
    /// fn_2be984's 1200 bytes. Zero in the fixture, save/restored by the port.
    pub const SENTENCE: u32 = 0x6d40;
    /// The sentence PCRE's guest stack, 256 bytes. Restored by the port.
    pub const PCRE_STACK: u32 = 0x7200;
    /// fn_2c322c's 64 bytes. Zero in the fixture, save/restored by the port.
    pub const PRONOUNCE: u32 = 0x7300;
    /// fn_2c322c's dictionary-marshal words inside [`PRONOUNCE`]: the match
    /// result, length and flag, the 9-byte match prefix and the open flag.
    /// `fn_2c3450_m` saves and restores the whole 64 bytes, and conversion
    /// uses +8/+12/+16/+24, so the marshal area is +32..+56.
    pub const PRONOUNCE_MATCH_RESULT: u32 = 0x7320;
    pub const PRONOUNCE_MATCH_LENGTH: u32 = 0x7324;
    pub const PRONOUNCE_MATCH_FLAG: u32 = 0x7326;
    pub const PRONOUNCE_MATCH_PREFIX: u32 = 0x7328;
    pub const PRONOUNCE_OPEN_FLAG: u32 = 0x7334;
    /// `dictionary_new`'s temporary word, inside [`PRONOUNCE`]. The port
    /// saves and restores it around the acquire, and `fn_2c3450_m` saves
    /// and restores the whole area around the run.
    pub const PRONOUNCE_TEMP_WORD: u32 = 0x733c;
    /// fn_26a3e4's 128 bytes: the position out-word, then the 64-byte
    /// scratch `RenderWriterEnvironment.scratch` names (restored on return).
    /// The byte-text branch that owns the scratch is not taken by the own
    /// request, but the slot is still caller-owned and restored.
    pub const WRITER: u32 = 0x7400;
}

/// The ROM constants of the render writer (fn_26a3e4 -> fn_269ee4).
///
/// Content proof: scripts/wire_a_r17_labels.py + wire_a_r17_verify.py read
/// code.bin. labels[0] (D_269198), labels[2] (D_269e60), padding (D_914e52),
/// separators (D_9150b0) and markers.normal (D_915158) contents are verified
/// in the image. labels[1] is the single in-image occurrence of
/// "after seq_loc_HandleExtendedToi1()". The checked-in fn_269ee4.o is
/// stale versus code.bin (pool offset +0x14), so the add-pc targets do not
/// resolve from it; labels[3] ("at end()") occurs twice (0x269ea4 and
/// D_26a3d8, whose content is "at end()\\0") and uses D_26a3d8. All four
/// labels, before_write, log_record, content_type and empty_string are
/// UNREAD on the taken write-text path: mode 0 returns from fn_263384 and
/// fn_2634c0 before they touch a label, and the port's fanout_call ignores
/// its record/content/data addresses. The READ literals are padding,
/// separators and markers.normal, all verified above.
pub mod writer_rom {
    /// D_269198: the write-text log label.
    pub const LABEL_WRITE_TEXT: u32 = 0x269198;
    /// The "after seq_loc_HandleExtendedToi1()" label (single occurrence).
    pub const LABEL_AFTER_EXTENDED: u32 = 0x26a344;
    /// D_269e60: the marker-position log label.
    pub const LABEL_MARKER_POS: u32 = 0x269e60;
    /// The "at end()" label. Ambiguous 0x269ea4/D_26a3d8 (identical
    /// content); uses D_26a3d8. Unread on the taken branch.
    pub const LABEL_AT_END: u32 = 0x26a3d8;
    /// D_264bec: "text/plain;charset=utf-1..." (unread by the port).
    pub const TEXT_CONTENT_TYPE: u32 = 0x264bec;
    /// A ROM NUL byte (padding after the labels[1] string; unread by port).
    pub const EMPTY_STRING: u32 = 0x26a367;
    /// D_914e52: the u16 padding string.
    pub const PADDING: u32 = 0x914e52;
    /// D_9150b0: the u16 separator set.
    pub const SEPARATORS: u32 = 0x9150b0;
    /// D_915158: the "normal" u16 string the extended pass matches.
    pub const NORMAL: u32 = 0x915158;
    /// D_26a3d8: the pack-markers log label (unread, mode-gated).
    pub const BEFORE_WRITE: u32 = 0x26a3d8;
    /// D_26a3e0: the marker record template (unread by the port).
    pub const LOG_RECORD: u32 = 0x26a3e0;
    /// "application/x-realspeak-markers-pp;version=4.0" (unread by port).
    pub const CONTENT_TYPE: u32 = 0x14f85c;
}

/// The live state lock. The ARM's fn_176f2c/fn_176f5c read the lock word's
/// inner dispatch slot and reach the guest only when it is set. The live
/// binding precomputes the ARM's verdict from the arena, so the memory-free
/// trait reports exactly what the entries would: `rc` is the entry status
/// (0, or 0x81402007 when the lock word itself is null) and `entry` names
/// the guest call the inner slot selects, which the trait cannot reach, so
/// that case stays a loud blocker naming the exact entry and target.
pub struct LiveLock {
    pub rc: u32,
    pub entry: u32,
    pub target: u32,
    pub release_rc: u32,
    pub release_entry: u32,
    pub release_target: u32,
}
impl crate::frontend::StateLock for LiveLock {
    fn acquire(&mut self) -> u32 {
        if self.entry != 0 {
            panic!("the live render reached the guest state lock entry {:#x} with {:#x}", self.entry, self.target);
        }
        self.rc
    }
    fn release(&mut self) -> u32 {
        if self.release_entry != 0 {
            panic!("the live render reached the guest state lock entry {:#x} with {:#x}", self.release_entry, self.release_target);
        }
        self.release_rc
    }
}

/// Read the ARM's lock verdict for `owner` (the params object's lock word,
/// `word(params + 4)`, the argument the ARM passes to fn_176f2c/fn_176f5c)
/// out of the arena: the status the entry would report, plus the guest
/// entry and target it would call when the inner dispatch slot is set.
/// This mirrors `frontend::acquire_lock`/`release_lock` and the live
/// 0x176f2c/0x176f5c arm of `engine_live::Host`, which reenters through
/// the same table.
/// Audit the lock owners a live render selected. The ARM's fn_176f2c
/// and fn_176f5c reach the guest only when the owner's inner dispatch
/// slot is set (see lock_verdict), so every used owner must read zero
/// in both slots, or the live no-op diverged from the ARM.
pub(crate) fn audit_locks(
    m: &Memory<'_>,
    owners: &[u32; 8],
    used: &[bool; 8],
    count: u32,
    untracked: bool,
) {
    if untracked {
        panic!("the live render acquired a lock no select_lock named");
    }
    for i in 0..count as usize {
        if !used[i] {
            continue;
        }
        let owner = owners[i];
        if owner == 0 {
            panic!("the live render locked a null owner");
        }
        let table = m.word(m.word(owner) + 8);
        if m.word(table + 8) != 0 || m.word(table + 12) != 0 {
            panic!("the live render skipped the guest state lock owner {owner:#x}");
        }
    }
}

fn lock_verdict(m: &Memory<'_>, owner: u32) -> LiveLock {
    let one = |release: bool| {
        if owner == 0 {
            return (0x81402007, 0, 0);
        }
        let table = m.word(m.word(owner) + 8);
        let entry = m.word(table + if release { 12 } else { 8 });
        if entry == 0 {
            return (0, 0, 0);
        }
        (0, entry, m.word(owner + 8))
    };
    // Escape lookups pair one acquire with one release on the same owner,
    // so the verdict carries both halves.
    let (rc, entry, target) = one(false);
    let (release_rc, release_entry, release_target) = one(true);
    LiveLock { rc, entry, target, release_rc, release_entry, release_target }
}

/// The live tree operations. The property tree is not written by the render
/// chain this binding serves.
pub struct LiveTreeOps;
impl crate::frontend::TreeOps for LiveTreeOps {
    fn tree_cmp(&mut self, _a: u32, _b: u32) -> i32 {
        panic!("the live render reached the property tree");
    }
    fn tree_init(&mut self, _a: u32, _b: u32, _c: u32) -> (u32, Option<u32>) {
        panic!("the live render reached the property tree");
    }
    fn tree_alloc(&mut self, _a: u32, _b: u32) -> (u32, u32) {
        panic!("the live render reached the property tree");
    }
    fn tree_free_node(&mut self, _a: u32, _b: u32) {
        panic!("the live render reached the property tree");
    }
    fn tree_free_handle(&mut self, _a: u32, _b: u32) {
        panic!("the live render reached the property tree");
    }
}

/// The live before-change callback.
pub struct LiveBefore;
impl crate::frontend::BeforeChange for LiveBefore {
    fn before(&mut self, _c: u32, _k: u32, _d: u32, _a: &mut u8) -> u32 {
        panic!("the live render reached the before-change callback");
    }
}

/// The live element callback.
pub struct LiveElement;
impl crate::frontend::ElementCallback for LiveElement {
    fn call_element(&mut self, _a: u32, _q: u32, _r: u32) -> u32 {
        panic!("the live render reached the element callback");
    }
}

impl<'s, D: LiveDispatch> crate::render_part_c::Host for LiveRender<'s, D> {
    fn call(&mut self, m: &mut Memory<'_>, entry: u32, args: &[u32]) -> u32 {
        // caller-owned like Host's guard: engines share a process, and a
        // caught panic drops this with the request
        let d = self.depth;
        if d > 40 {
            panic!("render dispatch loop at {entry:#x}");
        }
        self.depth = d + 1;
        let r = self.dispatch.dispatch(m, entry, args);
        self.depth = d;
        r
    }
    fn output(&mut self, _m: &mut Memory<'_>, entry: u32, _args: &[u32]) -> (u32, u32) {
        panic!("the live render reached the output method {entry:#x}");
    }
}

impl<'s, D: LiveDispatch> crate::boot_part_e::Host for LiveRender<'s, D> {
    fn call(&mut self, m: &mut Memory<'_>, method: u32, arguments: &[u32]) -> u32 {
        self.dispatch.dispatch(m, method, arguments)
    }
}

impl<'s, D: LiveDispatch> crate::boot_part_b::Host for LiveRender<'s, D> {
    fn call(&mut self, m: &mut Memory<'_>, function: u32, args: &[u32]) -> u32 {
        self.dispatch.dispatch(m, function, args)
    }
    fn thread_id(&mut self) -> u32 {
        self.dispatch.thread_id()
    }
}

impl<'s, D: LiveDispatch> crate::boot_part_b::FormatHost for LiveRender<'s, D> {}

impl<'s, D: LiveDispatch> crate::log_live::LogMethodsM for LiveRender<'s, D> {
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
        self.dispatch
            .dispatch(mem, method, &[object, category, level, arg, format, varargs])
    }
}

/// The arena-blind allocator. The live render chain reaches the allocator
/// only through the memory-passing forms, so a call here means a path that
/// the own request does not take has been entered.
impl<'s, D: LiveDispatch> crate::frontend::HeapVtable for LiveRender<'s, D> {
    fn alloc_at(&mut self, _o: u32, _n: u32) -> u32 {
        panic!("the live render reached the arena-blind allocator");
    }
    fn realloc_at(&mut self, _o: u32, _p: u32, _n: u32) -> u32 {
        panic!("the live render reached the arena-blind reallocator");
    }
    fn release_at(&mut self, _o: u32, _p: u32) {
        panic!("the live render reached the arena-blind releaser");
    }
}

/// The arena-blind method table. See `HeapVtable` above.
impl<'s, D: LiveDispatch> crate::frontend::MethodTable for LiveRender<'s, D> {
    fn call_at(&mut self, _o: u32, _a: u32, _b: u32) -> u32 {
        panic!("the live render reached the arena-blind method table");
    }
}

impl<'s, D: LiveDispatch> crate::frontend::StateLock for LiveRender<'s, D> {
    fn acquire(&mut self) -> u32 {
        if !self.lock_current_set {
            self.lock_untracked = true;
            return 0;
        }
        for i in 0..self.lock_count as usize {
            if self.lock_owners[i] == self.lock_current {
                self.lock_used[i] = true;
            }
        }
        0
    }
    fn release(&mut self) -> u32 {
        if !self.lock_current_set {
            self.lock_untracked = true;
            return 0;
        }
        for i in 0..self.lock_count as usize {
            if self.lock_owners[i] == self.lock_current {
                self.lock_used[i] = true;
            }
        }
        0
    }
}

impl<'s, D: LiveDispatch> crate::render_part_b::ParameterHost for LiveRender<'s, D> {
    fn invoke(&mut self, function: u32, _args: &[u32]) -> u32 {
        panic!("the live render reached the arena-blind invoke {function:#x}");
    }
    fn compare_key(&mut self, memory: &[u8], a: u32, b: u32) -> i32 {
        // 1:1 with src/fn_196980.c (the tree comparator the ARM lookup
        // uses): null guards, then unsigned-char strcmp over guest memory
        // (c3 r30: the second 1e5bbc call's lifecycle close reaches here).
        if a == 0 && b == 0 {
            return 0;
        }
        if a == 0 {
            return -1;
        }
        if b == 0 {
            return 1;
        }
        crate::normalization::strcmp_at(memory, a, b)
    }
}

/// The live property tree. The render chain this binding serves does not
/// write the tree.
impl<'s, D: LiveDispatch> crate::frontend::TreeOps for LiveRender<'s, D> {
    fn tree_cmp(&mut self, _a: u32, _b: u32) -> i32 {
        panic!("the live render reached the property tree");
    }
    fn tree_init(&mut self, _a: u32, _b: u32, _c: u32) -> (u32, Option<u32>) {
        panic!("the live render reached the property tree");
    }
    fn tree_alloc(&mut self, _a: u32, _b: u32) -> (u32, u32) {
        panic!("the live render reached the property tree");
    }
    fn tree_free_node(&mut self, _a: u32, _b: u32) {
        panic!("the live render reached the property tree");
    }
    fn tree_free_handle(&mut self, _a: u32, _b: u32) {
        panic!("the live render reached the property tree");
    }
}

/// The live sequence constants: the engine's cached handle and the default
/// context, both read out of the arena.
impl<'s, D: LiveDispatch> crate::render_part_b::SequenceHost for LiveRender<'s, D> {
    fn cached_pair(&self) -> [u8; 8] {
        self.engine.cached_handle
    }
    fn default_context(&self) -> u32 {
        self.engine.default_context
    }
}

/// The live render part b2 host.
impl<'s, D: LiveDispatch> crate::render_part_b2::Host for LiveRender<'s, D> {}

/// The live writer host: the stream's own write entry with the caller's arena.
impl<'s, D: LiveDispatch> crate::render_part_b2::WriterHost for LiveRender<'s, D> {
    fn write(
        &mut self,
        m: &mut Memory<'_>,
        entry: u32,
        handle: [u32; 2],
        data: u32,
        size: u32,
    ) -> u32 {
        self.dispatch.dispatch(m, entry, &[handle[0], handle[1], data, size])
    }
}

/// The live sequence-writer host: the stream's own position entry, whose
/// out-word lives in the caller-owned [`scratch::WRITER`] slot.
impl<'s, D: LiveDispatch> crate::render_part_b2::SequenceWriterHost for LiveRender<'s, D> {
    fn position(&mut self, m: &mut Memory<'_>, entry: u32, handle: [u32; 2]) -> (u32, u32) {
        let rc = self.dispatch.dispatch(m, entry, &[handle[0], handle[1], scratch::WRITER]);
        (rc, m.word(scratch::WRITER))
    }
}

/// The live writer stream host. The write-text path (fn_269ee4) never
/// reaches the stream API; a call here names the exact entry as a blocker.
impl<'s, D: LiveDispatch> crate::render_part_b2::StreamHost for LiveRender<'s, D> {
    fn stream_call(
        &mut self,
        _m: &mut Memory<'_>,
        entry: u32,
        _args: &[u32],
        _output: &mut [u32; 2],
    ) -> u32 {
        panic!("the live render reached the writer stream call {entry:#x}");
    }
    fn stream_parameter(
        &mut self,
        _m: &mut Memory<'_>,
        entry: u32,
        _handle: [u32; 2],
        _text: &[u16],
    ) -> u32 {
        panic!("the live render reached the writer stream parameter {entry:#x}");
    }
}

/// The live lifecycle host.
impl<'s, D: LiveDispatch> crate::render_part_b::LifecycleHost for LiveRender<'s, D> {
    fn cleanup_environment(&self) -> crate::render_part_b2::CleanupEnvironment {
        crate::render_part_b2::CleanupEnvironment {
            engine: crate::render_part_c::Environment {
                cached_handle: self.engine.cached_handle,
                default_context: self.engine.default_context,
            },
            user_names: [0; 2],
            dictionary_name: 0,
            scratch: 0x6e00,
        }
    }
}

/// The render's log target. The ARM's fn_1738e8 gate suppresses every level
/// five message of this chain when the logger's own level byte is below five,
/// so the live render never reaches a target.
impl<'s, D: LiveDispatch> crate::render_part_b::RenderLog for LiveRender<'s, D> {
    fn log_target(
        &mut self,
        _m: &mut Memory<'_>,
        method: u32,
        _object: u32,
        _category: u32,
        _level: u32,
        _arg: u32,
        _format: u32,
        _args: &[crate::render_part_b::RenderArgument<'_>],
    ) -> u32 {
        panic!("the live render reached the log target {method:#x}");
    }
}

impl<'s, D: LiveDispatch> crate::render_part_b2::CleanupHost for LiveRender<'s, D> {
    fn cleanup_dependency(&mut self, m: &mut Memory<'_>, entry: u32, args: &[u32]) -> u32 {
        self.dispatch.dispatch(m, entry, args)
    }
}

/// The dictionary host. The render chain this binding serves does not open
/// or close a dictionary.
impl<'s, D: LiveDispatch> crate::render_part_d::DictionaryHost for LiveRender<'s, D> {
    fn heap_method(&mut self, m: &mut [u8], method: u32, object: u32, args: &[u32]) -> u32 {
        // The dictionary heap methods are guest entries reached with the
        // caller's arena, like the render heap methods.
        let mut full = [0u32; 8];
        full[0] = object;
        let n = 1 + args.len().min(7);
        full[1..n].copy_from_slice(&args[..n - 1]);
        self.dispatch.dispatch(&mut Memory { base: 0, bytes: m }, method, &full[..n])
    }
    fn lock_method(&mut self, m: &mut [u8], method: u32, target: u32) -> u32 {
        // The dictionary lock resolves to fn_176f2c/fn_176f5c; the ARM's
        // verdict for a clear inner slot is zero with no guest call.
        self.dispatch.dispatch(&mut Memory { base: 0, bytes: m }, method, &[target])
    }
    fn compare_key(&mut self, m: &[u8], _method: u32, key: u32, stored: u32) -> i32 {
        crate::normalization::strcmp_at(m, key, stored)
    }
    fn remove_node(&mut self, _m: &mut [u8], _tree: u32, _node: u32) {
        panic!("the live render reached the dictionary tree");
    }
    fn release_callback(&mut self, _m: &mut [u8], _owner: [u32; 2], _entry: &mut [u8; 32]) {
        panic!("the live render reached the dictionary release");
    }
    fn open_dictionary(
        &mut self,
        _m: &mut [u8],
        method: u32,
        _owner: [u32; 2],
        _name: u32,
        _flag: &mut u8,
    ) -> u32 {
        panic!("the live render reached the dictionary open {method:#x}");
    }
    fn match_dictionary(
        &mut self,
        _m: &mut [u8],
        method: u32,
        _owner: [u32; 2],
        _name: u32,
        _prefix: &[u8],
        _result: &mut u32,
        _length: &mut u16,
        _flag: &mut u8,
    ) -> u32 {
        panic!("the live render reached the dictionary match {method:#x}");
    }
}

impl<'s, D: LiveDispatch> crate::render_part_c2::ContextHost for LiveRender<'s, D> {
    fn select_lock(&mut self, lock: u32) {
        self.lock_current = lock;
        self.lock_current_set = true;
        for i in 0..self.lock_count as usize {
            if self.lock_owners[i] == lock {
                return;
            }
        }
        if self.lock_count as usize >= self.lock_owners.len() {
            panic!("the live render selected more than eight lock owners");
        }
        self.lock_owners[self.lock_count as usize] = lock;
        self.lock_used[self.lock_count as usize] = false;
        self.lock_count += 1;
    }
}

impl<'s, D: LiveDispatch> crate::render_part_c2::ResourceHost for LiveRender<'s, D> {
    fn resource_call(&mut self, m: &mut Memory<'_>, entry: u32, args: &[u32]) -> u32 {
        self.dispatch.dispatch(m, entry, args)
    }
}

impl<'s, D: LiveDispatch> crate::render_part_c2::SegmentCallbackHost for LiveRender<'s, D> {
    fn previous_match(&mut self, event: &[u8], target: &[u8]) -> i32 {
        let _ = target;
        crate::normalization::strcmp_at(event, 0, 0)
    }
}

impl<'s, D: LiveDispatch> crate::render_part_b::BlockHost for LiveRender<'s, D> {
    fn existing_port(&mut self, m: &mut Memory<'_>, entry: u32, args: &[u32]) -> u32 {
        self.dispatch.dispatch(m, entry, args)
    }
}

/// The separator set, copied out of the caller's arena.
pub(crate) fn separators_of(m: &Memory<'_>) -> [u8; 16] {
    let mut out = [0u8; 16];
    let at = parser_rom::SEPARATORS as usize;
    out.copy_from_slice(&m.bytes[at..at + 16]);
    out
}

/// The four parameter defaults, copied out of the caller's arena.
fn parameters_of(m: &Memory<'_>) -> [(u16, u32); 4] {
    let mut out = [(0u16, 0u32); 4];
    for (i, slot) in out.iter_mut().enumerate() {
        let at = parser_rom::PARAMETERS as usize + i * 8;
        slot.0 = u16::from_le_bytes([m.bytes[at], m.bytes[at + 1]]);
        slot.1 = u32::from_le_bytes(m.bytes[at + 4..at + 8].try_into().unwrap());
    }
    out
}

impl<'s, D: LiveDispatch> LiveRender<'s, D> {
    /// The parser, fn_2c0f74. Runs the live parser over the caller's arena:
    /// the lock verdict is precomputed from the arena, the comparator reads
    /// the snapshot keys and the heap methods dispatch with the arena.
    /// The engine heap is fixed per parse; a foreign heap object panics.
    pub fn parse(&mut self, m: &mut Memory<'_>, args: &[u32]) -> u32 {
        let engine = crate::render_part_c::Environment {
            cached_handle: self.engine.cached_handle,
            default_context: self.engine.default_context,
        };
        let (rrc, context) = crate::render_part_c2::resolve(m, [m.word(args[0]), m.word(args[0] + 4)], &engine);
        if rrc >= 0x80000000 {
            return 0x81102007;
        }
        let heap = m.word(context + 4);
        // The parse lookups lock the engine params; pre-select the owner
        // so the audit verifies every acquire it serves. The owner is
        // stable for one parse (same engine, same params object).
        let props = m.word(context + 20);
        let owner = if props == 0 { 0 } else { m.word(props + 4) };
        if props != 0 {
            crate::render_part_c2::ContextHost::select_lock(self, owner);
        }
        let lock = lock_verdict(m, owner);
        let snap = self.snap;
        let parameters = parameters_of(m);
        let separators = separators_of(m);
        let mut inserted = [0u8; 40];
        let at = parser_rom::DEFAULTS as usize + 0x8c;
        inserted.copy_from_slice(&m.bytes[at..at + 40]);
        let parser = crate::render_part_c2::ParserEnvironment {
            separators: &separators,
            boundary: None,
            default_name: 0,
            parameters,
            voice_key: 0,
            inserted_image: inserted,
        };
        let env = crate::render_part_c2::Environment {
            engine: crate::render_part_c::Environment {
                cached_handle: self.engine.cached_handle,
                default_context: self.engine.default_context,
            },
            max_markers_key: 0,
            langcode_key: 0,
            parser_category: rom::PARSER_CATEGORY,
            nts_prefix: b"",
            ipa_prefix: b"",
        };
        let cell = core::cell::RefCell::new(&mut self.dispatch);
        let mut ops = LiveTreeOps;
        let mut before = LiveBefore;
        let mut callback = LiveElement;
        let mut compare = move |x: u32, y: u32| -> i32 { crate::normalization::strcmp_at(snap, x, y) };
        let mut lock = lock;
        let mut host = ParseHost {
            dispatch: &cell,
            lock_owners: &mut self.lock_owners,
            lock_used: &mut self.lock_used,
            lock_count: &mut self.lock_count,
            lock_current: &mut self.lock_current,
            lock_current_set: &mut self.lock_current_set,
            lock_untracked: &mut self.lock_untracked,
        };
        let mut allocator = ParseAlloc { heap, dispatch: &cell };
        let mut rt = ParserRuntime {
            env: &env,
            parser: &parser,
            host: &mut host,
            lock: &mut lock,
            ops: &mut ops,
            before: &mut before,
            callback: &mut callback,
            compare: &mut compare,
            scratch: scratch::RUNTIME,
        };
        crate::render_part_c2::fn_2c0f74_m(m, args[0], args[1], args[2], args[3], args[4], args[5], args[6], args[7], args[8], args[9], &mut rt, &mut allocator)
    }
    /// The parser-mode chooser, fn_2bfa90.
    pub fn parser_mode(&mut self, m: &mut Memory<'_>, args: &[u32]) -> u32 {
        let mut eos = [0u8; 40];
        let at = rom::PARSER_GLOBALS as usize + 0xc4;
        eos.copy_from_slice(&m.bytes[at..at + 40]);
        let separators = separators_of(m);
        let engine = crate::render_part_c::Environment {
            cached_handle: self.engine.cached_handle,
            default_context: self.engine.default_context,
        };
        let data = crate::render_part_c::ParserData {
            eos: &eos,
            separators: &separators,
            category: rom::PARSER_CATEGORY,
            formats: [rom::PARSER_FORMATS[0], rom::PARSER_FORMATS[1], 0, 0, 0],
            extra_key: rom::EXTRA_KEY,
            scratch: scratch::PARSER,
        };
        let mut compare = |bytes: &[u8], x: u32, y: u32| crate::normalization::strcmp_at(bytes, x, y);
        fn_2bfa90_m(
            m,
            args[0],
            args[1],
            args[2],
            args[3],
            &engine,
            &data,
            self,
            &mut compare,
        )
    }

    /// The escape-sequence processor, fn_2c4914.
    pub fn escape(&mut self, m: &mut Memory<'_>, args: &[u32]) -> u32 {
        let parameters = parameters_of(m);
        let mut spaces = [0u8; 12];
        let at = parser_rom::SPACES as usize;
        spaces.copy_from_slice(&m.bytes[at..at + 12]);
        let mut following = [0u8; 40];
        let at = parser_rom::DEFAULTS as usize + 0x8c;
        following.copy_from_slice(&m.bytes[at..at + 40]);
        // Caller-owned PCRE work buffers for the escape regex commands.
        // Singing markup reaches escape sequences the own request never
        // takes; the match inputs and tables are ROM/snapshot-immutable,
        // so no live-arena borrow is held. Sizes match the certified
        // sentence-pattern arm (0x2be984).
        let snap_tables = self.snap;
        let mut escapes = [0i16; 75];
        for (i, slot) in escapes.iter_mut().enumerate() {
            let at = 0x9140e4 + i * 2;
            *slot = i16::from_le_bytes(snap_tables[at..at + 2].try_into().unwrap());
        }
        let mut utf8_limits = [0i32; 6];
        for (i, slot) in utf8_limits.iter_mut().enumerate() {
            let at = 0x9147ac + i * 4;
            *slot = i32::from_le_bytes(snap_tables[at..at + 4].try_into().unwrap());
        }
        let mut utf8_first = [0i32; 6];
        for (i, slot) in utf8_first.iter_mut().enumerate() {
            let at = 0x9147c4 + i * 4;
            *slot = i32::from_le_bytes(snap_tables[at..at + 4].try_into().unwrap());
        }
        let mut utf8_mask = [0u32; 6];
        for (i, slot) in utf8_mask.iter_mut().enumerate() {
            let at = 0x9147dc + i * 4;
            *slot = u32::from_le_bytes(snap_tables[at..at + 4].try_into().unwrap());
        }
        let tables = crate::regex::PcreTables {
            op_lengths: snap_tables[0x914092..0x914192].try_into().unwrap(),
            ctypes: snap_tables[0x91426c..0x91436c].try_into().unwrap(),
            default_tables: snap_tables[0x91436c..0x9147ac].try_into().unwrap(),
            escapes: &escapes,
            utf8_limits: &utf8_limits,
            utf8_first: &utf8_first,
            utf8_cont: snap_tables[0x9147f4..0x914834].try_into().unwrap(),
            utf8_mask: &utf8_mask,
            rep_min: snap_tables[0x914044..0x91404a].try_into().unwrap(),
            rep_max: snap_tables[0x91404a..0x914050].try_into().unwrap(),
            posix_lengths: &[],
            posix_names: &[],
            posix_maps: &[],
            ctypes_overspill: &snap_tables[0x9147ac..0x9147ac + 4096],
        };
        let mut program = [0u8; 2048];
        let mut subject = [0u8; 2048];
        let mut frames = [crate::regex::MatchFrame::default(); 2048];
        let mut tags = [0u8; 2048];
        let mut blocks = [crate::regex::FrameBlockState::default(); 64];
        let escape_scratch = crate::render_part_d::EscapeScratch {
            program: &mut program,
            subject: &mut subject,
            frames: &mut frames,
            tags: &mut tags,
            blocks: &mut blocks,
            stack: scratch::PCRE_STACK,
            fallback: 0,
            toi_names: [0xa09390, 0xa09394, 0xa093da],
        };
        let execution = crate::render_part_c2::EscapeExecution {
            scratch: core::cell::RefCell::new(escape_scratch),
            tables: &tables,
        };
        let escape = crate::render_part_c2::EscapeEnvironment {
            execution: Some(execution),
            parameters,
            spaces: &spaces,
            following,
            reset_name: parser_rom::DEFAULTS,
            fallback: 0,
            audio: parser_rom::AUDIO,
            marker: parser_rom::MARKER,
        };
        let separators = separators_of(m);
        let mut inserted = [0u8; 40];
        let at = parser_rom::DEFAULTS as usize + 0x8c;
        inserted.copy_from_slice(&m.bytes[at..at + 40]);
        let parser = crate::render_part_c2::ParserEnvironment {
            separators: &separators,
            boundary: None,
            default_name: 0,
            parameters,
            voice_key: 0,
            inserted_image: inserted,
        };
        let env = crate::render_part_c2::Environment {
            engine: crate::render_part_c::Environment {
                cached_handle: self.engine.cached_handle,
                default_context: self.engine.default_context,
            },
            max_markers_key: 0,
            langcode_key: 0,
            parser_category: rom::PARSER_CATEGORY,
            nts_prefix: b"",
            ipa_prefix: b"",
        };
        // The escape lookups lock the parser params through fn_175a1c
        // on word(context + 20). Replicate fn_2c4914's prologue to name
        // the object, then precompute the ARM's lock verdict from the
        // arena (see lock_verdict). fn_2c4914 returns before any lock use
        // when the state or buffers are null, so the verdict is unused
        // there; the heap stays unnamed on that path too.
        let (rrc, context) = if args[0] == 0 || args[1] == 0 {
            (0x80002007, 0)
        } else {
            crate::render_part_c2::resolve(m, [m.word(args[0]), m.word(args[0] + 4)], &env.engine)
        };
        let mut lock = if rrc >= 0x80000000 {
            LiveLock { rc: 0, entry: 0, target: 0, release_rc: 0, release_entry: 0, release_target: 0 }
        } else {
            let props = m.word(context + 20);
            lock_verdict(m, if props == 0 { 0 } else { m.word(props + 4) })
        };
        let mut ops = LiveTreeOps;
        let mut before = LiveBefore;
        let mut callback = LiveElement;
        // The escape comparator reads the immutable property keys from the
        // arena snapshot, so it holds no borrow of the live arena the
        // ports mutate through `m`.
        let snap = self.snap;
        let mut compare = move |x: u32, y: u32| -> i32 {
            crate::normalization::strcmp_at(snap, x, y)
        };
        // The escape heap traffic runs through the engine heap on the
        // caller's arena via the shared dispatch cell, exactly like the
        // pronunciation path's allocator. The cell borrow is released
        // before any host call (heap methods run pure arena ops plus
        // locks, never back into this host), so borrows cannot nest.
        let heap = if context == 0 { 0 } else { m.word(context + 4) };
        let cell = core::cell::RefCell::new(&mut self.dispatch);
        let mut host = ParseHost {
            dispatch: &cell,
            lock_owners: &mut self.lock_owners,
            lock_used: &mut self.lock_used,
            lock_count: &mut self.lock_count,
            lock_current: &mut self.lock_current,
            lock_current_set: &mut self.lock_current_set,
            lock_untracked: &mut self.lock_untracked,
        };
        let mut allocator = ParseAlloc { heap, dispatch: &cell };
        let mut rt = ParserRuntime {
            env: &env,
            parser: &parser,
            host: &mut host,
            lock: &mut lock,
            ops: &mut ops,
            before: &mut before,
            callback: &mut callback,
            compare: &mut compare,
            scratch: scratch::RUNTIME,
        };
        crate::render_part_c2::fn_2c4914_m(m, args[0], args[1], scratch::ESCAPE, &escape, &mut rt, &mut allocator)
    }

    /// The parser-mode window scan, fn_2c0014.
    pub fn segment(&mut self, m: &mut Memory<'_>, args: &[u32]) -> u32 {
        let separators = separators_of(m);
        let mut first = [0u8; 40];
        let at = rom::PARSER_GLOBALS as usize + 0x74;
        first.copy_from_slice(&m.bytes[at..at + 40]);
        let mut split = [0u8; 40];
        let at = rom::PARSER_GLOBALS as usize + 0x9c;
        split.copy_from_slice(&m.bytes[at..at + 40]);
        let mut number_table = [0u16; 256];
        for (i, slot) in number_table.iter_mut().enumerate() {
            *slot = m.half(parser_rom::NUMBER_TABLE + i as u32 * 2);
        }
        let seg = crate::render_part_c2::SegmentEnvironment {
            first,
            split,
            separators: &separators,
            max_length_key: parser_rom::MAX_LENGTH,
            number_table: &number_table,
        };
        let env = crate::render_part_c2::Environment {
            engine: crate::render_part_c::Environment {
                cached_handle: self.engine.cached_handle,
                default_context: self.engine.default_context,
            },
            max_markers_key: 0,
            langcode_key: 0,
            parser_category: rom::PARSER_CATEGORY,
            nts_prefix: b"",
            ipa_prefix: b"",
        };
        let snap = self.snap;
        fn_2c0014(
            m,
            args[0],
            args[1],
            args[2] as i32,
            args[3],
            scratch::SEGMENT,
            &seg,
            &env,
            self,
            snap,
        )
    }
    /// The pronunciation conversion, fn_2c322c. The language filter, stop
    /// filter, prefixes, default language, trim set and decoder tables are
    /// copied out of the ROM image in the caller's arena; the dictionary
    /// registry name is the ROM literal fn_2ecaac passes. The iterator and
    /// conversion heap traffic runs through the guest heap on the arena via
    /// the shared dispatch cell, exactly like the parser's allocator.
    pub fn pronounce(&mut self, m: &mut Memory<'_>, args: &[u32]) -> u32 {
        let engine = crate::render_part_c::Environment {
            cached_handle: self.engine.cached_handle,
            default_context: self.engine.default_context,
        };
        let (rrc, context) =
            crate::render_part_c2::resolve(m, [m.word(args[0]), m.word(args[0] + 4)], &engine);
        if rrc >= 0x80000000 {
            return 0x81102007;
        }
        let heap = m.word(context + 4);
        let mut nts = [0u8; 12];
        nts.copy_from_slice(&m.bytes[pron_rom::NTS_PREFIX as usize..pron_rom::NTS_PREFIX as usize + 12]);
        let mut ipa = [0u8; 12];
        ipa.copy_from_slice(&m.bytes[pron_rom::IPA_PREFIX as usize..pron_rom::IPA_PREFIX as usize + 12]);
        let mut language_filter = [0u16; 4];
        for (i, slot) in language_filter.iter_mut().enumerate() {
            let at = pron_rom::LANGUAGE_FILTER as usize + i * 2;
            *slot = u16::from_le_bytes([m.bytes[at], m.bytes[at + 1]]);
        }
        let mut stop_filter = [0u16; 4];
        for (i, slot) in stop_filter.iter_mut().enumerate() {
            let at = pron_rom::STOP_FILTER as usize + i * 2;
            *slot = u16::from_le_bytes([m.bytes[at], m.bytes[at + 1]]);
        }
        let mut trim = [0u8; 130];
        let mut units = 0usize;
        loop {
            let at = pron_rom::TRIM as usize + units * 2;
            let unit = u16::from_le_bytes([m.bytes[at], m.bytes[at + 1]]);
            if unit == 0 || units == 64 {
                break;
            }
            trim[units * 2..units * 2 + 2].copy_from_slice(&unit.to_le_bytes());
            units += 1;
        }
        let trim_bytes = (units + 1) * 2;
        let mut extra = [0u8; 256];
        extra.copy_from_slice(&m.bytes[pron_rom::DECODER_EXTRA as usize..pron_rom::DECODER_EXTRA as usize + 256]);
        let mut mask = [0u8; 24];
        mask.copy_from_slice(&m.bytes[pron_rom::DECODER_MASK as usize..pron_rom::DECODER_MASK as usize + 24]);
        let mut base = [0u8; 24];
        base.copy_from_slice(&m.bytes[pron_rom::DECODER_BASE as usize..pron_rom::DECODER_BASE as usize + 24]);
        let decoder = crate::normalization::DecoderTables {
            extra: &extra,
            mask: &mask,
            mask_base: 6,
            base: &base,
            base_base: 0,
        };
        let dictionary = crate::render_part_d::DictionaryEnvironment {
            registry_name: pron_rom::REGISTRY_NAME,
            temporary_word: scratch::PRONOUNCE_TEMP_WORD,
            cached_handle: self.engine.cached_handle,
            default_context: self.engine.default_context,
        };
        let env = crate::render_part_c2::Environment {
            engine,
            max_markers_key: 0,
            langcode_key: pron_rom::LANGCODE_KEY,
            parser_category: rom::PARSER_CATEGORY,
            nts_prefix: &nts,
            ipa_prefix: &ipa,
        };
        let pronunciation = crate::render_part_c2::PronunciationEnvironment {
            dictionary: &dictionary,
            default_language: pron_rom::DEFAULT_LANGUAGE,
            language_filter: &language_filter,
            stop_filter: &stop_filter,
            trim: &trim[..trim_bytes],
            decoder: &decoder,
            scratch: scratch::PRONOUNCE,
        };
        let cell = core::cell::RefCell::new(&mut self.dispatch);
        let mut host = ParseHost {
            dispatch: &cell,
            lock_owners: &mut self.lock_owners,
            lock_used: &mut self.lock_used,
            lock_count: &mut self.lock_count,
            lock_current: &mut self.lock_current,
            lock_current_set: &mut self.lock_current_set,
            lock_untracked: &mut self.lock_untracked,
        };
        let mut allocator = ParseAlloc { heap, dispatch: &cell };
        crate::render_part_c2::fn_2c322c_m(
            m,
            args[0],
            args[1],
            &env,
            &pronunciation,
            &mut host,
            &mut allocator,
        )
    }
}

/// The parse host: field borrows of the live render for the parser port,
/// which holds its host and its allocator as separate `&mut`s. The guest
/// heap methods dispatch through the shared cell; the cell borrow is
/// released before any host call (heap methods run pure arena ops plus
/// locks, never back into this host), so borrows cannot nest.
pub struct ParseHost<'a, 'x, D> {
    pub dispatch: &'a core::cell::RefCell<&'a mut D>,
    pub lock_owners: &'x mut [u32; 8],
    pub lock_used: &'x mut [bool; 8],
    pub lock_count: &'x mut u32,
    pub lock_current: &'x mut u32,
    pub lock_current_set: &'x mut bool,
    pub lock_untracked: &'x mut bool,
}
/// The parse allocator: the engine heap fixed per parse, dispatched
/// through the shared cell. The object check fails loud on a second heap.
pub struct ParseAlloc<'a, D> {
    pub heap: u32,
    pub dispatch: &'a core::cell::RefCell<&'a mut D>,
}
impl<D: LiveDispatch> crate::frontend::HeapVtableM for ParseAlloc<'_, D> {
    fn alloc_at_m(&mut self, object: u32, size: u32, mem: &mut [u8]) -> u32 {
        if self.heap == 0 {
            return 0;
        }
        assert_eq!(object, crate::frontend::mem_word(mem, self.heap + 4), "live heap object changed heaps");
        let method = crate::frontend::mem_word(mem, crate::frontend::mem_word(mem, self.heap));
        self.dispatch.borrow_mut().dispatch(&mut Memory { base: 0, bytes: mem }, method, &[object, size])
    }
    fn realloc_at_m(&mut self, object: u32, old: u32, size: u32, mem: &mut [u8]) -> u32 {
        if self.heap == 0 {
            return 0;
        }
        assert_eq!(object, crate::frontend::mem_word(mem, self.heap + 4), "live heap object changed heaps");
        let method = crate::frontend::mem_word(mem, crate::frontend::mem_word(mem, self.heap) + 8);
        self.dispatch.borrow_mut().dispatch(&mut Memory { base: 0, bytes: mem }, method, &[object, old, size])
    }
    fn release_at_m(&mut self, object: u32, pointer: u32, mem: &mut [u8]) {
        if self.heap == 0 {
            return;
        }
        assert_eq!(object, crate::frontend::mem_word(mem, self.heap + 4), "live heap object changed heaps");
        let method = crate::frontend::mem_word(mem, crate::frontend::mem_word(mem, self.heap) + 12);
        self.dispatch.borrow_mut().dispatch(&mut Memory { base: 0, bytes: mem }, method, &[object, pointer]);
    }
}
impl<D: LiveDispatch> crate::render_part_c2::ParserAllocatorM for ParseAlloc<'_, D> {
    fn calloc_m(&mut self, object: u32, count: u32, size: u32, memory: &mut [u8]) -> u32 {
        if self.heap == 0 || count == 0 || size == 0 {
            return 0;
        }
        assert_eq!(object, crate::frontend::mem_word(memory, self.heap + 4), "live heap object changed heaps");
        let method = crate::frontend::mem_word(memory, crate::frontend::mem_word(memory, self.heap) + 4);
        self.dispatch.borrow_mut().dispatch(&mut Memory { base: 0, bytes: memory }, method, &[object, count, size])
    }
}
impl<D: LiveDispatch> crate::boot_part_b::Host for ParseHost<'_, '_, D> {
    fn call(&mut self, m: &mut Memory<'_>, function: u32, args: &[u32]) -> u32 {
        self.dispatch.borrow_mut().dispatch(m, function, args)
    }
    fn thread_id(&mut self) -> u32 {
        self.dispatch.borrow_mut().thread_id()
    }
}
impl<D: LiveDispatch> crate::frontend::TreeOps for ParseHost<'_, '_, D> {
    fn tree_cmp(&mut self, _a: u32, _b: u32) -> i32 {
        panic!("the live parse reached the property tree");
    }
    fn tree_init(&mut self, _a: u32, _b: u32, _c: u32) -> (u32, Option<u32>) {
        panic!("the live parse reached the property tree");
    }
    fn tree_alloc(&mut self, _a: u32, _b: u32) -> (u32, u32) {
        panic!("the live parse reached the property tree");
    }
    fn tree_free_node(&mut self, _a: u32, _b: u32) {
        panic!("the live parse reached the property tree");
    }
    fn tree_free_handle(&mut self, _a: u32, _b: u32) {
        panic!("the live parse reached the property tree");
    }
}
impl<D: LiveDispatch> crate::render_part_c2::ResourceHost for ParseHost<'_, '_, D> {
    fn resource_call(&mut self, m: &mut Memory<'_>, entry: u32, args: &[u32]) -> u32 {
        self.dispatch.borrow_mut().dispatch(m, entry, args)
    }
}
/// The pronunciation path's dictionary calls, dispatched with the caller's
/// arena through the shared cell. The open/match guest entries marshal
/// their out-words through the [`scratch::PRONOUNCE`] area, which
/// `fn_2c3450_m` saves and restores around the run.
impl<D: LiveDispatch> crate::render_part_d::DictionaryHost for ParseHost<'_, '_, D> {
    fn heap_method(&mut self, m: &mut [u8], method: u32, object: u32, args: &[u32]) -> u32 {
        let mut full = [0u32; 8];
        full[0] = object;
        let n = 1 + args.len().min(7);
        full[1..n].copy_from_slice(&args[..n - 1]);
        self.dispatch
            .borrow_mut()
            .dispatch(&mut Memory { base: 0, bytes: m }, method, &full[..n])
    }
    fn lock_method(&mut self, m: &mut [u8], method: u32, target: u32) -> u32 {
        self.dispatch
            .borrow_mut()
            .dispatch(&mut Memory { base: 0, bytes: m }, method, &[target])
    }
    fn compare_key(&mut self, m: &[u8], _method: u32, key: u32, stored: u32) -> i32 {
        crate::normalization::strcmp_at(m, key, stored)
    }
    fn remove_node(&mut self, _m: &mut [u8], _tree: u32, _node: u32) {
        panic!("the live parse reached the dictionary tree");
    }
    fn release_callback(&mut self, _m: &mut [u8], _owner: [u32; 2], _entry: &mut [u8; 32]) {
        panic!("the live parse reached the dictionary release");
    }
    fn open_dictionary(
        &mut self,
        m: &mut [u8],
        method: u32,
        owner: [u32; 2],
        name: u32,
        flag: &mut u8,
    ) -> u32 {
        let at = scratch::PRONOUNCE_OPEN_FLAG as usize;
        m[at] = 0;
        let rc = self.dispatch.borrow_mut().dispatch(
            &mut Memory { base: 0, bytes: m },
            method,
            &[owner[0], owner[1], name, scratch::PRONOUNCE_OPEN_FLAG],
        );
        *flag = m[at];
        rc
    }
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
    ) -> u32 {
        assert!(prefix.len() <= 9, "dictionary prefix exceeds marshal");
        let pat = scratch::PRONOUNCE_MATCH_PREFIX as usize;
        m[pat..pat + 9].fill(0);
        m[pat..pat + prefix.len()].copy_from_slice(prefix);
        let rat = scratch::PRONOUNCE_MATCH_RESULT as usize;
        let lat = scratch::PRONOUNCE_MATCH_LENGTH as usize;
        let fat = scratch::PRONOUNCE_MATCH_FLAG as usize;
        for b in &mut m[rat..rat + 4] {
            *b = 0;
        }
        m[lat..lat + 2].copy_from_slice(&[0, 0]);
        m[fat] = 0;
        let rc = self.dispatch.borrow_mut().dispatch(
            &mut Memory { base: 0, bytes: m },
            method,
            &[
                owner[0],
                owner[1],
                name,
                scratch::PRONOUNCE_MATCH_PREFIX,
                scratch::PRONOUNCE_MATCH_RESULT,
                scratch::PRONOUNCE_MATCH_LENGTH,
                scratch::PRONOUNCE_MATCH_FLAG,
            ],
        );
        *result = u32::from_le_bytes(m[rat..rat + 4].try_into().unwrap());
        *length = u16::from_le_bytes([m[lat], m[lat + 1]]);
        *flag = m[fat];
        rc
    }
}
impl<D: LiveDispatch> crate::render_part_c2::SegmentCallbackHost for ParseHost<'_, '_, D> {
    fn previous_match(&mut self, event: &[u8], target: &[u8]) -> i32 {
        let _ = target;
        crate::normalization::strcmp_at(event, 0, 0)
    }
}
impl<D: LiveDispatch> crate::render_part_b::ParameterHost for ParseHost<'_, '_, D> {
    fn invoke(&mut self, function: u32, _args: &[u32]) -> u32 {
        panic!("the live parse reached the arena-blind invoke {function:#x}");
    }
}
impl<D: LiveDispatch> crate::render_part_c2::ContextHost for ParseHost<'_, '_, D> {
    fn select_lock(&mut self, lock: u32) {
        *self.lock_current = lock;
        *self.lock_current_set = true;
        for i in 0..*self.lock_count as usize {
            if self.lock_owners[i] == lock {
                return;
            }
        }
        if *self.lock_count as usize >= self.lock_owners.len() {
            panic!("the live parse selected more than eight lock owners");
        }
        self.lock_owners[*self.lock_count as usize] = lock;
        self.lock_used[*self.lock_count as usize] = false;
        *self.lock_count += 1;
    }
}
impl<D: LiveDispatch> crate::frontend::StateLock for ParseHost<'_, '_, D> {
    fn acquire(&mut self) -> u32 {
        if !*self.lock_current_set {
            *self.lock_untracked = true;
            return 0;
        }
        for i in 0..*self.lock_count as usize {
            if self.lock_owners[i] == *self.lock_current {
                self.lock_used[i] = true;
            }
        }
        0
    }
    fn release(&mut self) -> u32 {
        if !*self.lock_current_set {
            *self.lock_untracked = true;
            return 0;
        }
        for i in 0..*self.lock_count as usize {
            if self.lock_owners[i] == *self.lock_current {
                self.lock_used[i] = true;
            }
        }
        0
    }
}
impl<D: LiveDispatch> crate::render_part_c::Host for ParseHost<'_, '_, D> {
    fn call(&mut self, m: &mut Memory<'_>, entry: u32, args: &[u32]) -> u32 {
        self.dispatch.borrow_mut().dispatch(m, entry, args)
    }
    fn output(&mut self, _m: &mut Memory<'_>, entry: u32, _args: &[u32]) -> (u32, u32) {
        panic!("the live parse reached the output method {entry:#x}");
    }
}
impl<D: LiveDispatch> crate::boot_part_b::FormatHost for ParseHost<'_, '_, D> {}
impl<D: LiveDispatch> crate::log_live::LogMethodsM for ParseHost<'_, '_, D> {
    fn log_m(&mut self, method: u32, object: u32, category: u32, level: u32, arg: u32, format: u32, varargs: u32, mem: &mut Memory<'_>) -> u32 {
        self.dispatch.borrow_mut().dispatch(mem, method, &[object, category, level, arg, format, varargs])
    }
}
impl<D: LiveDispatch> crate::frontend::HeapVtable for ParseHost<'_, '_, D> {
    fn alloc_at(&mut self, _o: u32, _n: u32) -> u32 {
        panic!("the live parse reached the arena-blind allocator");
    }
    fn realloc_at(&mut self, _o: u32, _p: u32, _n: u32) -> u32 {
        panic!("the live parse reached the arena-blind reallocator");
    }
    fn release_at(&mut self, _o: u32, _p: u32) {
        panic!("the live parse reached the arena-blind releaser");
    }
}
impl<D: LiveDispatch> crate::frontend::MethodTable for ParseHost<'_, '_, D> {
    fn call_at(&mut self, _o: u32, _a: u32, _b: u32) -> u32 {
        panic!("the live parse reached the arena-blind method table");
    }
}

impl<'s, D: LiveDispatch> LiveRenderHost for LiveRender<'s, D> {
    fn parse(&mut self, m: &mut Memory<'_>, args: &[u32]) -> u32 {
        LiveRender::parse(self, m, args)
    }
    fn parser_mode(&mut self, m: &mut Memory<'_>, args: &[u32]) -> u32 {
        LiveRender::parser_mode(self, m, args)
    }
    fn escape(&mut self, m: &mut Memory<'_>, args: &[u32]) -> u32 {
        LiveRender::escape(self, m, args)
    }
    fn segment(&mut self, m: &mut Memory<'_>, args: &[u32]) -> u32 {
        LiveRender::segment(self, m, args)
    }
    fn pronounce(&mut self, m: &mut Memory<'_>, args: &[u32]) -> u32 {
        LiveRender::pronounce(self, m, args)
    }
}

/// Run the backend progress call, fn_2645b4, over the caller's arena.
///
/// The block environment is built from the arena: the label strings are the
/// ROM literals the ARM's own code resolves, the clear tags are the ROM
/// tables, the normalizer's defaults are the ROM record at D_9c5a28 and the
/// separator set is copied out of D_9e0990.
pub fn run_block<D: LiveDispatch>(
    dispatch: D,
    m: &mut Memory<'_>,
    o: u32,
    sp: u32,
    param3: u32,
    snap: &[u8],
) -> u32 {
    assert!(!snap.is_empty(), "run_block needs the arena key snapshot");
    let engine = crate::render_part_c::Environment {
        cached_handle: core::array::from_fn(|i| m.byte(0x8d7660 + i as u32)),
        default_context: 0x8d7640,
    };
    let log = || crate::render_part_b::RenderEnvironment {
        category: rom::LOG_CATEGORY,
        marker_format: 0,
        header_format: 0,
        offsets_format: 0,
        text_format: 0,
        marker_names: rom::MARKER_NAMES,
        tag_names: 0,
        special_tag_name: 0,
    };
    let marker = || crate::render_part_b2::Environment {
        normal: 0,
        suffix: 0,
        boundary_tags: [0; 6],
        cached_handle: engine.cached_handle,
        default_context: 0x8d7640,
    };
    let lifecycle = || crate::render_part_b::LifecycleEnvironment {
        engine: crate::render_part_c::Environment {
            cached_handle: engine.cached_handle,
            default_context: 0x8d7640,
        },
        // 1:1 with src/fn_2b92d8.c: the pair table D_9b25e8 and the
        // key triple D_9b2608 are ROM tables the lifecycle close reads
        // directly (c3 r30: the 0x0 placeholders made every non-Latin
        // second 1e5bbc call stop with 0x8b702007 from 0x175a24/key==0).
        parameter_pairs: 0x9b25e8,
        parameter_triple: 0x9b2608,
    };
    let mut defaults = [0u8; 52];
    let at = parser_rom::SPAN_DEFAULTS as usize;
    defaults.copy_from_slice(&m.bytes[at..at + 52]);
    let mut labels = [0u32; 24];
    labels.copy_from_slice(&rom::LABELS);
    let mut clear_active = [0u16; 3];
    let mut clear_all = [0u16; 6];
    for (i, slot) in clear_active.iter_mut().enumerate() {
        *slot = m.half(rom::CLEAR_ACTIVE + i as u32 * 2);
    }
    for (i, slot) in clear_all.iter_mut().enumerate() {
        *slot = m.half(rom::CLEAR_ALL + i as u32 * 2);
    }
    let log_v = log();
    let marker_v = marker();
    let lifecycle_v = lifecycle();
    let block = crate::render_part_b::BlockEnvironment {
        lifecycle: &lifecycle_v,
        log: &log_v,
        marker: &marker_v,
        clear_active,
        clear_all,
        readmode: rom::READ_MODE,
        resetlen: rom::RESET_LEN,
        mode_key: rom::READ_MODE,
        position_key: rom::RESET_LEN,
        labels,
        marker_log: rom::MARKER_LOG,
        normalizer_defaults: &defaults,
    };
    // No lock slot is baked in: the ports name the lock token through
    // `select_lock` before acquiring, and the owners are audited below.
    let mut host = LiveRender {
        dispatch,
        snap,
        engine: crate::render_part_c::Environment {
            cached_handle: engine.cached_handle,
            default_context: engine.default_context,
        },
        log: log(),
        marker: marker(),
        lifecycle: lifecycle(),
        lock_owners: [0; 8],
        lock_used: [false; 8],
        lock_count: 0,
        lock_current: 0,
        lock_current_set: false,
        lock_untracked: false,
        depth: 0,
    };
    let rc = fn_2645b4_m(m, o, sp, param3, &block, scratch::BLOCK, &mut host);
    // Audit the lock owners the render chain selected (see audit_locks).
    crate::engine_live_render::audit_locks(m, &host.lock_owners, &host.lock_used, host.lock_count, host.lock_untracked);
    rc
}

/// Run the render writer, fn_26a3e4, over the caller's arena.
///
/// The writer environment is built from the arena: the label strings are
/// the ROM literals in [`writer_rom`], the extended set is D_914e52 /
/// D_9150b0 / D_915158 and the 128-byte [`scratch::WRITER`] slot holds the
/// position out-word plus the writer scratch. The byte-text branch
/// (state+119 != 0) is not taken by the own request; its environment
/// fields (latin, rewrite, labels, formats) are zero placeholders and the
/// port panics naming fn_268cbc if it is ever reached.
pub fn run_writer<D: LiveDispatch>(
    dispatch: D,
    m: &mut Memory<'_>,
    context: u32,
    state: u32,
    sequence: u32,
    first: [u32; 2],
    second: [u32; 2],
    snap: &[u8],
) -> u32 {
    assert!(!snap.is_empty(), "run_writer needs the arena key snapshot");
    let engine = crate::render_part_c::Environment {
        cached_handle: core::array::from_fn(|i| m.byte(0x8d7660 + i as u32)),
        default_context: 0x8d7640,
    };
    let log = crate::render_part_b::RenderEnvironment {
        category: rom::LOG_CATEGORY,
        marker_format: 0,
        header_format: 0,
        offsets_format: 0,
        text_format: 0,
        marker_names: rom::MARKER_NAMES,
        tag_names: 0,
        special_tag_name: 0,
    };
    let markers = crate::render_part_b2::Environment {
        normal: writer_rom::NORMAL,
        suffix: 0,
        boundary_tags: [0; 6],
        cached_handle: engine.cached_handle,
        default_context: engine.default_context,
    };
    let extended = crate::render_part_b2::ExtendedEnvironment {
        markers,
        padding: writer_rom::PADDING,
        separators: writer_rom::SEPARATORS,
    };
    let writer = crate::render_part_b2::WriterEnvironment {
        before_write: writer_rom::BEFORE_WRITE,
        diagnostics: crate::render_part_b::RenderEnvironment {
            category: rom::LOG_CATEGORY,
            marker_format: 0,
            header_format: 0,
            offsets_format: 0,
            text_format: 0,
            marker_names: rom::MARKER_NAMES,
            tag_names: 0,
            special_tag_name: 0,
        },
        log_record: writer_rom::LOG_RECORD,
        content_type: writer_rom::CONTENT_TYPE,
    };
    let sequence_env = crate::render_part_b2::SequenceWriterEnvironment {
        extended,
        writer,
        labels: [
            writer_rom::LABEL_WRITE_TEXT,
            writer_rom::LABEL_AFTER_EXTENDED,
            writer_rom::LABEL_MARKER_POS,
            writer_rom::LABEL_AT_END,
        ],
        text_content_type: writer_rom::TEXT_CONTENT_TYPE,
        empty_string: writer_rom::EMPTY_STRING,
    };
    let env = crate::render_part_b2::RenderWriterEnvironment {
        sequence: sequence_env,
        latin: 0,
        rewrite: crate::render_part_b::RewriteEnvironment {
            patterns: 0,
            wide0: 0,
            wide1: 0,
            category: 0,
            empty: 0,
            space: 0,
        },
        labels: [0; 11],
        content_type: 0,
        log_category: 0,
        input_format: 0,
        output_format: 0,
        scratch: scratch::WRITER + 64,
    };
    let mut host = LiveRender {
        dispatch,
        snap,
        engine: crate::render_part_c::Environment {
            cached_handle: engine.cached_handle,
            default_context: engine.default_context,
        },
        log,
        marker: crate::render_part_b2::Environment {
            normal: 0,
            suffix: 0,
            boundary_tags: [0; 6],
            cached_handle: engine.cached_handle,
            default_context: engine.default_context,
        },
        lifecycle: crate::render_part_b::LifecycleEnvironment {
            engine: crate::render_part_c::Environment {
                cached_handle: engine.cached_handle,
                default_context: engine.default_context,
            },
            parameter_pairs: 0,
            parameter_triple: 0,
        },
        lock_owners: [0; 8],
        lock_used: [false; 8],
        lock_count: 0,
        lock_current: 0,
        lock_current_set: false,
        lock_untracked: false,
        depth: 0,
    };
    let at = m.offset(scratch::WRITER);
    let mut saved = [0u8; 128];
    saved.copy_from_slice(&m.bytes[at..at + 128]);
    let rc = crate::render_part_b2::fn_26a3e4_m(
        m, context, state, sequence, first, second, &env, &mut host,
    );
    m.bytes[at..at + 128].copy_from_slice(&saved);
    crate::engine_live_render::audit_locks(m, &host.lock_owners, &host.lock_used, host.lock_count, host.lock_untracked);
    rc
}
