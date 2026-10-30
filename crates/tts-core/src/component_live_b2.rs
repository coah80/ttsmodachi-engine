//! Live component roots 1e5bbc and 1e4b7c, split from component_live_b.
use crate::{boot::Memory, component_live_b};

/// 4096-byte frame for the round-four token ports. The port restores it.
const FRAME: u32 = 0x1000;
/// Host scratch for the round-four logger, resolver and callout.
const HOST_SCRATCH: u32 = 0x3000;

/// PCRE host for the live token root. The captured PCRE state holds one block
/// and no back-reference, so the exec performs no heap traffic and the host
/// needs no arena of its own.
pub struct PcreGuest;

impl crate::regex::PcreHost for PcreGuest {
    fn validate(&mut self, object: u32, key: u32, tag: u32, size: u32) -> u32 {
        crate::frontend::validate_handle(object != 0, key, tag, size)
    }
    fn resolve_owner(&mut self, _handle: u32, _key: i32) -> (u32, u32) {
        (0x8110_2007, 0)
    }
    fn heap_of(&mut self, _owner: u32) -> u32 {
        0
    }
    fn alloc(&mut self, _heap: u32, _count: u32, _size: u32) -> Option<u32> {
        None
    }
    fn free(&mut self, _heap: u32, _id: u32) {}
}

/// PCRE host that serves the matcher's spill/overflow blocks from the guest
/// heap via fn_1963b8/fn_196428 (the same allocator the ARM uses). Number,
/// currency and date patterns take the matcher's heap-traffic paths
/// (recursion spill, block growth) that PcreGuest refuses with -6; the ARM
/// allocates them and keeps going. Validate/resolve/heap_of match
/// PcreGuest: the exec path only reaches alloc/free/validate.
struct PcreLive<'a, 'b, 'c> {
    m: &'a mut Memory<'b>,
    h: &'a mut TokenHost<'c>,
    /// Guest address of the match subject (the `subject` root argument).
    guest_subject: u32,
    /// Owner heap for guest-side temporaries.
    heap: u32,
    /// Game-level callout the current root installed (fn_2b6afc for the
    /// token root): materialized per OP_CALLOUT hit. `None` keeps the
    /// historical continue-the-match behaviour.
    guest_callout:
        Option<for<'x, 'm> fn(&'x mut Memory<'m>, u32, &'x mut TokenHost<'c>) -> u32>,
}

impl crate::regex::PcreHost for PcreLive<'_, '_, '_> {
    fn guest_callout(&mut self, block: &crate::regex::CalloutBlock) -> i32 {
        let f = match self.guest_callout {
            Some(f) => f,
            None => return 0,
        };
        // ARM 1:1: the PCRE engine materializes the callout block (48B) and
        // the offsets array (n words) on its own stack, at 0xe0ff108 and
        // 0xe0ff2e0 on every wl callout of every workload probed. The old
        // heap-allocated copies shifted every downstream heap address.
        // Both areas are fully rewritten per callout, so like the ARM
        // frame they keep leftovers without save/restore.
        let offp = 0xe0ff2e0u32;
        for (i, v) in block.offsets.iter().enumerate() {
            self.m.put_word(offp + i as u32 * 4, *v as u32);
        }
        let bp = 0xe0ff108u32;
        self.m.put_word(bp, 1);
        self.m.put_word(bp + 4, block.number as u32);
        self.m.put_word(bp + 8, offp);
        self.m.put_word(bp + 12, self.guest_subject);
        self.m.put_word(bp + 16, block.subject.len() as u32);
        self.m.put_word(bp + 20, block.start_match as u32);
        self.m.put_word(bp + 24, block.current_position as u32);
        self.m.put_word(bp + 28, block.capture_top as u32);
        self.m.put_word(bp + 32, block.capture_last as u32);
        self.m.put_word(bp + 36, block.callout_data);
        self.m.put_word(bp + 40, block.pattern_position as u32);
        self.m.put_word(bp + 44, block.next_item_length as u32);
        let rc = f(&mut *self.m, bp, &mut *self.h);
        rc as i32
    }
    fn validate(&mut self, object: u32, key: u32, tag: u32, size: u32) -> u32 {
        crate::frontend::validate_handle(object != 0, key, tag, size)
    }
    fn resolve_owner(&mut self, _handle: u32, _key: i32) -> (u32, u32) {
        (0x8110_2007, 0)
    }
    fn heap_of(&mut self, _owner: u32) -> u32 {
        0
    }
    fn alloc(&mut self, heap: u32, count: u32, size: u32) -> Option<u32> {
        // The ARM's spill path allocates from the store heap via
        // fn_1963b8 (zeroed); the a2 lane's PcreAdapter does the same.
        let total = count.wrapping_mul(size);
        let p = self.h.dispatch(self.m, 0x1963b8, &[heap, count, size]);
        if p == 0 { None } else { Some(p) }
    }
    fn free(&mut self, heap: u32, id: u32) {
        self.h.dispatch(self.m, 0x196428, &[heap, id]);
    }
}

/// Live host for the 1e4b7c token root. The shared dispatcher keeps every
/// bound platform entry; this one adds the PCRE callout and the round-four
/// token ports that the token root reaches.
pub struct TokenHost<'a> {
    pub inner: component_live_b::LiveHost<'a>,
    /// Owner heap of the token root, used for the allocator vtable.
    pub heap: u32,
}

impl TokenHost<'_> {
    fn dispatch(&mut self, m: &mut Memory<'_>, f: u32, a: &[u32]) -> u32 {
        match f {
            0x249cec => crate::speech_part_1::fn_249cec(m, a[0], 0x4200, self),
            0x2b7118 => crate::speech_part_3::fn_2b7118(m, a[0], a[1], FRAME, self),
            0x2b8f08 => crate::speech_part_3::fn_2b8f08(m, a[0], FRAME, self),
            0x24e1a0 => fn_24e1a0_m(m, a[0], a[1], self),
            0x2e564c => {
                crate::speech_part_3::fn_2e564c(m, a[0], a[1], a[2], a[3], a[4] as u8, FRAME, self)
            }
            0x24dcf0 => crate::speech_part_3::fn_24dcf0(m, a[0], a[1], a[2], a[3], FRAME, self),
            // fn_24d5e4: unsigned byte compare of the bytes at two guest
            // addresses. Reached as the lookup comparator (sp5_lookup)
            // from fn_2e5adc's rule-function search.
            0x24d5e4 => crate::normalization::byte_compare_at(m.bytes, a[0], a[1]),
            // ISTOKEN rule subtree (seg1 r5): the stub, the rule body, the
            // section search, the section slots and the dictionary scan.
            // Heap temporaries use this TokenHost's owner heap.
            0x2b8504 => crate::component_live_2b7d::fn_2b8504_m(m, a[0], a[1], self),
            0x2b7fdc => crate::component_live_2b7d::fn_2b7fdc_m(m, a[0], a[1], a[2], self),
            // fn_179f30 (seg1 r4's shared broker open): the dictionary open
            // on the number/date validation path. Same live broker port.
            0x179f30 => crate::component_live_b3::broker_179f30_live(m, a[0], a[1], a[2], self),
            // Mapped-dictionary loads (seg1 r4 ports, shared): the 1ea4e0
            // expansion path maps sections and reads the result maximum.
            0x26b7ec => crate::component_live_b3::fn_26b7ec_m(m, self, a[0], a[1], a[2], a[3]),
            0x26bb28 => crate::component_live_b3::fn_26bb28_m(m, self, a[0], a[1]),
            0x26bb44 => crate::component_live_b3::fn_26bb44_m(m, a[0]),
            // DoExpansions driver (seg1 r5).
            0x2b4968 => crate::component_live_2b7d::fn_2b4968_m(m, a[0], a[1], self),
            // DoExpansions text buffers (seg1 r5/r8): the +0x78 record
            // alloc/free pair and the slot grow, 1:1 from src/fn_*.c.
            0x24cb74 => crate::component_live_2b7d::fn_24cb74_m(m, a[0], self),
            0x24cbd4 => crate::component_live_2b7d::fn_24cbd4_m(m, a[0], self),
            0x24cc08 => crate::component_live_2b7d::fn_24cc08_m(m, a[0], a[1], self),
            0x24cdac => crate::component_live_2b7d::fn_24cdac_m(m, a[0], self),
            0x24ce40 => crate::component_live_2b7d::fn_24ce40_m(m, a[0], self),
            // DoExpansions callees (seg1 r5): bound as reached; unported
            // ones fail closed until their 1:1 ports land.
            // DoExpansions subtree (seg1 r7): 1:1 ports in
            // component_live_2b7d, checked against the ROM disassembly.
            // 1966ac runs here (not inner) so its vtable comparator stays bound.
            0x1966ac => crate::speech_part_1::parameter_call_m(m, f, a, 0x1800, self),
            0x2b4260 => crate::component_live_2b7d::fn_2b4260_m(m, a[0], a[1], a[2], a[3], self, 0),
            0x24b290 => crate::component_live_2b7d::fn_24b290_m(m, a[0], a[1], a[2], a[3], self),
            0x24b540 => crate::component_live_2b7d::fn_24b540_m(
                m, a[0], a[1], a[2], a[3], a[4], a[5], a[6], self,
            ),
            0x24b8d0 => crate::component_live_2b7d::fn_24b8d0_m(m, a[0], a[1], self),
            0x24a5d4 => crate::component_live_2b7d::fn_24a5d4_m(m, a[0], a[1], a[2], self),
            0x24a6bc => crate::component_live_2b7d::fn_24a6bc_m(m, a[0], a[1], a[2], self),
            0x24a7a4 => crate::component_live_2b7d::fn_24a7a4_m(m, a[0], a[1], a[2], self),
            0x2e4e80 => crate::component_live_2b7d::fn_2e4e80_m(
                m, a[0], a[1], a[2], a[3], a[4], a[5], self,
            ),
            0x2b570c => crate::component_live_2b7d::fn_2b570c_m(m, a[0], a[1], self),
            0x2190b4 => crate::component_live_2b7d::fn_2190b4_m(m, a[0], a[1], a[2], a[3], self),
            0x2192c0 => crate::component_live_2b7d::fn_2192c0_m(m, a[0], a[1], a[2], a[3], self),
            0x24ac00 => crate::component_live_b::Host::call_m(&mut self.inner, f, a, m),
            // _AND_/_OR_ rule combinators (seg1 r5).
            0x2b73d0 => crate::component_live_2b7d::fn_2b73d0_m(m, a[0], a[1], self),
            // _STRING_TTS check: 1:1 port in component_live_2b7d.
            0x2b5c18 => crate::component_live_2b7d::fn_2b5c18_m(m, a[0], a[1], self),
            0x2b8868 => crate::component_live_2b7d::fn_2b8868_m(m, a[0], a[1], self),
            // c3 r31: route 0x2193a4 through the working depes expansion
            // (b3 expanded_m with LexExpansion) instead of LiveHost's
            // NoExpansion stub, which errors on URL/email expansions.
            // 1:1 with ARM fn_2195e4 (grammar methods expand via 1ea4e0).
            0x2193a4 => {
                if a.len() < 3 {
                    return crate::component_live_b::Host::call_m(&mut self.inner, f, a, m);
                }
                let mut ws = crate::component_live_c::VmWorkspace::default();
                crate::component_live_b3::depes_exec_module_expanded_m(
                    m, a[0], a[1], a[2], &mut ws, self,
                )
            }
            _ => crate::component_live_b::Host::call_m(&mut self.inner, f, a, m),
        }
    }
    /// The allocator vtable entry of the owner heap.
    fn heap_method(&self, m: &Memory<'_>, slot: u32) -> u32 {
        if self.heap == 0 {
            return 0;
        }
        m.word(m.word(self.heap) + slot)
    }
}

impl component_live_b::Host for TokenHost<'_> {
    fn call_m(&mut self, entry: u32, args: &[u32], m: &mut Memory<'_>) -> u32 {
        self.dispatch(m, entry, args)
    }
}

impl crate::speech_part_1::Host for TokenHost<'_> {
    fn call(&mut self, m: &mut Memory<'_>, f: u32, a: &[u32]) -> u32 {
        self.dispatch(m, f, a)
    }
    fn get(
        &mut self,
        m: &mut Memory<'_>,
        f: u32,
        handle: u32,
        pos: u32,
        len: u32,
    ) -> (u32, u32, u32) {
        crate::speech_part_1::Host::get(&mut self.inner, m, f, handle, pos, len)
    }
}

impl crate::speech_part_6::Host for TokenHost<'_> {
    fn call(&mut self, m: &mut Memory<'_>, f: u32, a: &[u32]) -> u32 {
        self.dispatch(m, f, a)
    }
    fn log(
        &mut self,
        m: &mut Memory<'_>,
        logger: u32,
        category: u32,
        level: u32,
        format: &[u8],
        values: &[u32],
    ) {
        crate::speech_part_6::Host::log(&mut self.inner, m, logger, category, level, format, values)
    }
    fn log_guest(
        &mut self,
        m: &mut Memory<'_>,
        logger: u32,
        category: u32,
        level: u32,
        fmt: u32,
        values: &[u32],
    ) {
        crate::speech_part_6::Host::log_guest(
            &mut self.inner,
            m,
            logger,
            category,
            level,
            fmt,
            values,
        )
    }
    fn error(&mut self, m: &mut Memory<'_>, logger: u32, category: u32, code: u32) {
        crate::speech_part_6::Host::error(&mut self.inner, m, logger, category, code)
    }
}

impl crate::speech_part_3::Host for TokenHost<'_> {
    fn call(&mut self, m: &mut Memory<'_>, f: u32, a: &[u32]) -> u32 {
        self.dispatch(m, f, a)
    }
}

impl crate::log_live::LogMethodsM for TokenHost<'_> {
    fn log_m(
        &mut self,
        f: u32,
        p: u32,
        category: u32,
        level: u32,
        arg: u32,
        fmt: u32,
        va: u32,
        m: &mut Memory<'_>,
    ) -> u32 {
        crate::log_live::LogMethodsM::log_m(&mut self.inner, f, p, category, level, arg, fmt, va, m)
    }
}

impl crate::boot_part_f::Host for TokenHost<'_> {
    fn call(&mut self, m: &mut Memory<'_>, f: u32, a: &[u32]) -> u32 {
        self.dispatch(m, f, a)
    }
    fn compare(
        &mut self,
        m: &mut Memory<'_>,
        _method: u32,
        key: &mut crate::boot_part_f::Key,
        value: &mut u32,
    ) -> u32 {
        crate::boot_part_f::compare_parameter(m, *key, *value)
    }
}

impl crate::speech_part_5::Host for TokenHost<'_> {
    fn scratch(&self) -> u32 {
        HOST_SCRATCH
    }
    fn format(&self, callsite: u32) -> u32 {
        format_address(callsite)
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

/// US ROM format strings for the round-four diagnostic sites that the live
/// token root reaches. The logger's level filter drops the rest.
fn format_address(callsite: u32) -> u32 {
    match callsite {
        0x2b13 | 0x2afa => 0x2b73a0,
        // The 2e5adc TRUE literal (ROM pc-rel add at 2e5adc+0x168).
        0x2e5bfc => 0x2e5cdc,
        _ => 0,
    }
}

impl crate::frontend::HeapVtableM for TokenHost<'_> {
    fn alloc_at_m(&mut self, obj: u32, n: u32, mem: &mut [u8]) -> u32 {
        let mut m = Memory {
            base: 0,
            bytes: mem,
        };
        let f = self.heap_method(&m, 0);
        if f == 0 {
            return 0;
        }
        crate::component_live_b::Host::call_m(&mut self.inner, f, &[obj, n], &mut m)
    }
    fn realloc_at_m(&mut self, obj: u32, old: u32, n: u32, mem: &mut [u8]) -> u32 {
        let mut m = Memory {
            base: 0,
            bytes: mem,
        };
        let f = self.heap_method(&m, 8);
        if f == 0 {
            return 0;
        }
        crate::component_live_b::Host::call_m(&mut self.inner, f, &[obj, old, n], &mut m)
    }
    fn release_at_m(&mut self, obj: u32, object: u32, mem: &mut [u8]) {
        let mut m = Memory {
            base: 0,
            bytes: mem,
        };
        let f = self.heap_method(&m, 12);
        if f == 0 {
            return;
        }
        crate::component_live_b::Host::call_m(&mut self.inner, f, &[obj, object], &mut m);
    }
}

impl crate::speech_part_3::SpeechHost for TokenHost<'_> {}

impl crate::speech_part_3::SpeechRound4Host for TokenHost<'_> {
    type RegexHost = PcreGuest;
    fn literal(&self, address: u32) -> u32 {
        match address {
            0x2b7120 => 0x2b7398,
            0x2b7124 => 0x2b7390,
            _ => 0,
        }
    }
    fn pool_name(&self, _pool: u32) -> u32 {
        0
    }
    fn regex_exec(
        &mut self,
        m: &mut Memory<'_>,
        a: &[u32],
        offsets: &mut [i32; 100],
        _exec: crate::speech_part_3::RegexEntry<Self::RegexHost>,
        callout: fn(&mut Memory<'_>, u32, &mut Self) -> u32,
    ) -> i32 {
        // The caller passes the pure pcre_exec forwarder; run it here with
        // the heap-capable host instead of the no-heap PcreGuest. The
        // argument mapping is identical (verified by the wl 0-diff certs).
        // The installed game-level callout (fn_2b6afc) is threaded through
        // so OP_CALLOUT runs it instead of continuing the match.
        regex_exec_live(m, a, offsets, self, callout)
    }
}

/// fn_23f598 over the live arena. The PCRE state, the compiled pattern, the
/// extra block and the subject all come from the guest image; the frame store
/// is the guest's own frame area.
fn regex_exec_live<'c>(
    m: &mut Memory<'_>,
    a: &[u32],
    offsets: &mut [i32; 100],
    host: &mut TokenHost<'c>,
    callout: for<'x, 'm> fn(&'x mut Memory<'m>, u32, &'x mut TokenHost<'c>) -> u32,
) -> i32 {
    use crate::regex::*;
    let (object, key, pattern, extra_ptr, subject, length, start, options) =
        (a[0], a[1], a[2], a[3], a[4], a[5], a[6], a[7]);
    let count = m.half(object + 8);
    let per = m.half(object + 10);
    let size = m.half(object + 12);
    let maximum = m.half(object + 14);
    let entries = m.word(object + 4);
    let plen = m.word(pattern + 4) as usize;
    let top_backref = m.half(pattern + 18);
    // The ARM clears the store on entry (clear_store_entries in
    // pcre_exec does the same), so a leftover count from the previous
    // call is routine, not an error. The remaining guards cover states
    // the workloads never reach.
    if top_backref != 0 || per == 0 || size == 0 {
        return -6;
    }
    let mut blocks: [FrameBlockState; 10] = core::array::from_fn(|i| {
        if i >= count as usize {
            return FrameBlockState::default();
        }
        let b = m.word(entries + i as u32 * 4);
        if b == 0 {
            return FrameBlockState::default();
        }
        FrameBlockState {
            present: true,
            live: true,
            used: m.word(b + 8),
            count: m.byte(b + 16),
            code_id: m.word(b),
            extra_id: m.word(b + 12),
            entry_id: b,
        }
    });
    let mut tags = [0u8; 100];
    for i in 0..count as usize {
        for j in 0..per as usize {
            tags[i * per as usize + j] = m.byte(blocks[i].extra_id + j as u32);
        }
    }
    let code = pattern
        + m.half(pattern + 24) as u32
        + (m.half(pattern + 26) as u32) * (m.half(pattern + 28) as u32);
    let delta = 0u32.wrapping_sub(subject);
    let mut frames: [MatchFrame; 100] = core::array::from_fn(|i| {
        if i >= count as usize * per as usize {
            return MatchFrame::default();
        }
        let ptr = blocks[i / per as usize].code_id + (i % per as usize) as u32 * size as u32;
        crate::render_part_d::load_match_frame(
            m.bytes,
            ptr,
            code,
            delta,
            &blocks,
            per as u32 * size as u32,
        )
    });
    let mut store = FrameStore {
        frames: &mut frames,
        tags: &mut tags,
        blocks: &mut blocks,
        frames_per_block: per,
        frame_size: size,
        max_blocks: maximum,
        block_count: count,
        current_block: m.half(object + 16),
        highest_block: m.half(object + 18),
        max_frames: m.half(object + 20),
        fresh: crate::regex::MatchFrame::default(),
        heap: m.word(m.word(object) + 4),
        blocks_id: Some(entries),
    };
    let mut program = [0u8; 8192];
    if plen > program.len() {
        return -6;
    }
    program[..plen].copy_from_slice(&m.bytes[m.offset(pattern)..m.offset(pattern) + plen]);
    let extra = ExecExtra {
        flags: m.word(extra_ptr),
        study: None,
        match_limit: m.word(extra_ptr + 8),
        callout_data: m.word(extra_ptr + 12),
        tables: None,
    };
    let op = core::array::from_fn::<u8, 256, _>(|i| m.byte(0x914092 + i as u32));
    let ctypes = core::array::from_fn::<u8, 256, _>(|i| m.byte(0x91426c + i as u32));
    let mut default_tables = [0u8; 1088];
    for (i, b) in default_tables.iter_mut().enumerate() {
        *b = m.byte(0x91436c + i as u32);
    }
    let escapes = load_i16(m, 0x9140e4);
    let limits = load_i32(m, 0x9147ac);
    let first = load_i32(m, 0x9147c4);
    let cont = core::array::from_fn::<u8, 64, _>(|i| m.byte(0x9147f4 + i as u32));
    let mask = core::array::from_fn::<u32, 6, _>(|i| m.word(0x9147dc + i as u32 * 4));
    let min = core::array::from_fn::<u8, 6, _>(|i| m.byte(0x914044 + i as u32));
    let max = core::array::from_fn::<u8, 6, _>(|i| m.byte(0x91404a + i as u32));
    let mut overspill = [0u8; 4096];
    for (i, b) in overspill.iter_mut().enumerate() {
        *b = m.byte(0x9147ac + i as u32);
    }
    let tables = PcreTables {
        op_lengths: &op,
        ctypes: &ctypes,
        default_tables: &default_tables,
        escapes: &escapes,
        utf8_limits: &limits,
        utf8_first: &first,
        utf8_cont: &cont,
        utf8_mask: &mask,
        rep_min: &min,
        rep_max: &max,
        posix_lengths: &[],
        posix_names: &[],
        posix_maps: &[],
        ctypes_overspill: &overspill,
    };
    // The PCRE host borrows the arena for spill/overflow blocks, so the
    // subject is copied first (the a2 lane's pcre_exec_live does the same).
    // `length` is a u16, so the 64 KiB buffer always fits.
    let len = length as usize;
    let mut subject_copy = [0u8; 65536];
    subject_copy[..len].copy_from_slice(&m.bytes[m.offset(subject)..m.offset(subject) + len]);
    let mut live = PcreLive {
        m: &mut *m,
        h: &mut *host,
        guest_subject: subject,
        heap: 0,
        guest_callout: Some(callout),
    };
    live.heap = store.heap;
    let result = crate::regex::pcre_exec(
        &mut live,
        object,
        key,
        &mut store,
        &program[..plen],
        Some(&extra),
        &subject_copy[..len],
        0,
        len,
        start as i32,
        options,
        Some(offsets),
        100,
        &tables,
        None,
    );
    for (off, v) in [
        (8, store.block_count),
        (10, store.frames_per_block),
        (12, store.frame_size),
        (14, store.max_blocks),
        (16, store.current_block),
        (18, store.highest_block),
        (20, store.max_frames),
    ] {
        m.put_half(object + off, v);
    }
    for i in 0..store.block_count as usize {
        let b = &store.blocks[i];
        m.put_word(entries + i as u32 * 4, b.entry_id);
        m.put_word(b.entry_id, b.code_id);
        m.put_word(b.entry_id + 4, size as u32 * per as u32 + 1);
        m.put_word(b.entry_id + 8, b.used);
        m.put_word(b.entry_id + 12, b.extra_id);
        m.put_byte(b.entry_id + 16, b.count);
        for j in 0..per as usize {
            let idx = i * per as usize + j;
            m.put_byte(b.extra_id + j as u32, store.tags[idx]);
            crate::render_part_d::save_match_frame(
                m.bytes,
                b.code_id + j as u32 * size as u32,
                &store.frames[idx],
                code,
                delta,
                store.blocks,
            );
        }
    }
    result
}

/// fn_24e1a0. Release every match entry of the runtime table and clear the
/// table head. The dtor list of fn_19703c is empty in the captured state.
fn fn_24e1a0_m(m: &mut Memory<'_>, p: u32, mode: u32, h: &mut TokenHost<'_>) -> u32 {
    let log = m.word(m.word(p) + 0x10);
    let id = m.word(p + 0x48);
    crate::log_live::fn_1738e8(m, log, id, 4, 0, 0x24e2ac, 0, h);
    let tbl = m.word(p + 0x44);
    if m.word(tbl + 0xc) != 0 {
        let ents = m.word(tbl + 0xc);
        let count = m.half(tbl + 0x10);
        let heap = m.word(m.word(p) + 4);
        let mut i = 1u16;
        while i < count {
            let e = m.word(ents + i as u32 * 12);
            if e != 0 {
                h.dispatch(m, 0x196428, &[heap, e]);
                m.put_word(ents + i as u32 * 12, 0);
            }
            i = i.wrapping_add(1);
        }
        if count > 1 {
            let at = m.offset(ents + 12);
            for k in 0..(count as u32 - 1) * 12 {
                m.bytes[at + k as usize] = 0;
            }
            m.put_half(tbl + 0x10, 1);
        }
        m.put_half(tbl + 0x16, 0);
    }
    m.put_word(tbl + 4, 0);
    m.put_half(tbl + 8, 0);
    if mode <= 1 {
        let arg = if mode == 1 { m.word(tbl) } else { 0 };
        fn_19703c_m(m, arg);
    }
    m.put_word(HOST_SCRATCH, 0);
    crate::log_live::fn_1738e8(m, log, id, 4, 0, 0x24e2d0, HOST_SCRATCH, h);
    0
}

/// Live entry for the 1e4b7c token root. The caller owns the arena and the
/// host; `end_name` is the mapped "At End of Processing" literal and
/// `scratch` is the root's own eight restored bytes.
pub fn fn_1e4b7c_live(
    m: &mut Memory<'_>,
    args: [u32; 5],
    end_name: u32,
    scratch: u32,
    host: &mut TokenHost<'_>,
) -> u32 {
    component_live_b::fn_1e4b7c_m(m, args, end_name, scratch, host)
}

/// fn_19703c. Run the destructor list of a runtime table and clear its count.
fn fn_19703c_m(m: &mut Memory<'_>, p: u32) -> u32 {
    if p == 0 {
        return 0;
    }
    let dtor = m.word(p + 0xc);
    let count = m.word(p + 0x14);
    if dtor != 0 && count != 0 {
        // A guest destructor is a code pointer; the live port cannot enter it.
        m.put_word(p + 0x14, 0);
        return 1;
    }
    m.put_word(p + 0x14, 0);
    1
}

fn load_i16(m: &Memory<'_>, at: u32) -> [i16; 75] {
    core::array::from_fn(|i| {
        let p = at + i as u32 * 2;
        let o = m.offset(p);
        i16::from_le_bytes(m.bytes[o..o + 2].try_into().unwrap())
    })
}

fn load_i32(m: &Memory<'_>, at: u32) -> [i32; 6] {
    core::array::from_fn(|i| {
        let p = at + i as u32 * 4;
        let o = m.offset(p);
        i32::from_le_bytes(m.bytes[o..o + 4].try_into().unwrap())
    })
}
