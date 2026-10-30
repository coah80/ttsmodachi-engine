//! Live scheduler segment 1 of the US speech_1 run (lane seg1, prompts/rust_seg_r1.md).
//!
//! Roots `0x1d5ba8` (tick 1307764, the first scheduler root: no preceding
//! step), `0x200e64` (tick 1336059) and `0x1e4b7c` (tick 1342217, long:
//! until just before the 2895641 step), with the `0x1cca64` scheduler
//! step before `0x200e64` and `0x1e4b7c` (ticks 1335471 and 1341629).
//! Captures: `scratch/engine-stages/seg1-*.wire` / `*.expected` plus
//! `seg1-bounds.json` (see `scratch/seg1/cap_seg1.py`).
//!
//! Dispatch shape. The prompt asks this module to wrap
//! `engine_live::Host` and forward every entry it does not handle. `Host`
//! is private to `engine_live.rs` (wire-a owns that file, so this lane
//! cannot name it); the arms below are written to merge into
//! `Host::call_inner` directly, routing through the public live entry
//! points and the certified ports:
//!
//! * the `0x1cca64` steps ride `engine_live::call_live_with_runtime_m`
//!   (the sentence-dump port is bound in `engine_live.rs`);
//! * `0x1d5ba8` rides the certified `component_live_c::fn_1d5ba8_m`
//!   port with the component test's probe dispatch (exactly as
//!   `tests/component_live_c.rs` binds it: `process_environment_us`,
//!   scratch `0xe0ff080`);
//! * `0x200e64` rides the certified
//!   `component_live_c::prompt_orth_live_m` port with the component
//!   test's probe dispatch (scratch `0xe0ff700`);
//! * `0x1e4b7c` rides the certified `component_live_b::fn_1e4b7c_m`
//!   port over the component test's `TokenHost` (exactly as
//!   `tests/component_live_b2.rs` binds it: `LiveHost` scratch `0x500`,
//!   root scratch `0x600`, heap taken from the entry arena).
//!
//! The fifth (stack) argument of each root is the ARM's stack word from
//! the entry capture (`args[4]`: `0x14c428ac` / `0x14c428ad` /
//! `0x14c428ae`); it is passed explicitly, never re-read from the live
//! guest stack slot, so the back-to-back stretch (which chains arenas
//! instead of reloading entry images) hands each root the scheduler's
//! own word. On a fresh entry image this equals the image's stack slot,
//! the way the certified component tests read it.
//!
//! Region: rom_region via the caller's `Rom` (`run_*` wrappers stay US;
//! `run_*_rom` threads the region through the probe dispatch, the
//! environment/literal addresses and the nested host's `rom()`). No
//! globals or statics, no unsafe. C locals live in Rust locals; the
//! ports' scratch frames are the certified components' own
//! excluded-stack windows, never lane state.
//!
//! The scheduler-loop advance between a step return and the next root
//! entry (the word at `0x14c403e4` then the halfword at `0x14c403e8`;
//! ARM pcs `0x155524`/`0x154304`) is replicated by
//! [`scheduler_advance_after_step`]. That fragment is scheduler-sequencer
//! state owned by the `0x15429c` port (wire-a's engine_live.rs); the
//! coordinator should fold it into the real sequencer. It is kept out of
//! the per-boundary certs and used only by the back-to-back stretch run.

use crate::{
    boot::Memory,
    engine, engine_live,
    rom_region::{addr, to_us, Rom},
};

/// Run one scheduler per-component step (`0x1cca64`) through the public
/// live dispatcher. `args` are the step's register args; `snap` is the
/// arena image at the step entry (render paths diff against it).
pub fn run_step(
    m: &mut Memory<'_>,
    args: &[u32],
    platform: &mut dyn engine::HeapPlatform,
    runtime: &mut dyn engine_live::RuntimeServicesM,
    snap: &[u8],
) -> Result<u32, engine::Error> {
    engine_live::call_live_with_runtime_m(m, 0x1cca64, args, platform, runtime, snap)
}

/// Probe host for the `0x1d5ba8` root: the component test's dispatch
/// (`tests/component_live_c.rs` `live_1d5ba8`), in `no_std` shape (no
/// tracing, no diagnostics). Nested calls that fail to bind record the
/// entry in [`Probe1d5ba8::first_unbound`] instead of panicking, so the
/// test reports the first unbound function start.
pub struct Probe1d5ba8<'a> {
    pub first_unbound: Option<u32>,
    platform: &'a mut dyn engine::HeapPlatform,
    rom: Rom,
}

impl<'a> Probe1d5ba8<'a> {
    pub fn new(platform: &'a mut dyn engine::HeapPlatform) -> Self {
        Probe1d5ba8::new_rom(platform, Rom::Us)
    }
    pub fn new_rom(platform: &'a mut dyn engine::HeapPlatform, rom: Rom) -> Self {
        Probe1d5ba8 {
            first_unbound: None,
            platform,
            rom,
        }
    }
    fn unbound(&mut self, f: u32) -> u32 {
        if self.first_unbound.is_none() {
            self.first_unbound = Some(f);
        }
        0x80002001
    }
}
/// Regional process environment: explicit per-region literals derived
/// from each ROM's loader correspondence (see component_live_c), never
/// the region map (code-region strings are unlisted there and would
/// fall back to the US address). US stays bit-identical.
fn process_environment_rom(rom: Rom) -> crate::speech_part_1::ProcessEnvironment {
    match rom {
        Rom::Us => crate::component_live_c::process_environment_us(),
        Rom::Eu => crate::component_live_c::process_environment_eu(),
        Rom::Kr => crate::component_live_c::process_environment_kr(),
    }
}

/// Regional speech literals for the `0x1ede44` arm: the US canonical
/// with every address field through the region map.
fn speech_literals_rom(rom: Rom) -> crate::speech_part_2::SpeechLiterals {
    let u = crate::component_live_c::speech_literals_us();
    crate::speech_part_2::SpeechLiterals {
        text: core::array::from_fn(|i| addr(rom, u.text[i])),
        end: addr(rom, u.end),
        errors: core::array::from_fn(|i| addr(rom, u.errors[i])),
    }
}

/// The broker open (`0x179f30`) rides the seg1 lane's certified port,
/// which takes the `component_live_b` host: forward to this probe's own
/// dispatch so nested calls stay regional through the same probe.
impl crate::component_live_b::Host for Probe1d5ba8<'_> {
    fn call_m(&mut self, f: u32, a: &[u32], m: &mut Memory<'_>) -> u32 {
        <Self as crate::component_live_c::Host>::call_m(self, f, a, m)
    }
    fn rom(&self) -> Rom {
        self.rom
    }
}

impl crate::component_live_c::Host for Probe1d5ba8<'_> {
    fn rom(&self) -> Rom {
        self.rom
    }
    fn call_m(&mut self, f: u32, a: &[u32], m: &mut Memory<'_>) -> u32 {
        // Guest-sourced entries are regional twins (e.g. EU 0x15bb74 for
        // US 0x15bab0); normalize US-first so direct US-literal calls from
        // the ports keep working while guest twins bind their US arm.
        let f = to_us(self.rom, f);
        if f == 0x15017c {
            return crate::component_live_c::fn_15017c_m(m, a, self);
        }
        if f == 0x1516c8 {
            crate::component_live_c::release_typed_buffer_m(m, a, self);
            return 0;
        }
        if f == 0x15c1e4 {
            let Ok(abi) = a.try_into() else {
                return self.unbound(f);
            };
            return crate::component_live_c::clone_record_live_m(m, abi, 0xe0fef80, self);
        }
        if f == 0x15c1c8 && self.rom != Rom::Us {
            // KR twin of US 0x15c1e4 that collides with a US key
            // (mid-function address, never a genuine call target).
            // Route the same clone-record port with the probe's region.
            let Ok(abi) = a.try_into() else {
                return self.unbound(f);
            };
            return crate::component_live_c::clone_record_live_m(m, abi, 0xe0fef80, self);
        }
        // Boot-part formatter observation, not a guest callee; the return
        // is ignored by the caller.
        if f == 0xffff0001 {
            return 0;
        }
        if [
            0x17a1a4, 0x179374, 0x179208, 0x17c9d8, 0x1b2760, 0x1b254c, 0x1b2974, 0x172de0,
            0x172d30, 0x4a794c, 0x4a78f0,
        ]
        .contains(&f)
        {
            return crate::component_live_c::broker_range_live_m(m, f, a, 0xe0fcf00, self);
        }
        if f == 0x26bb6c {
            let Ok(abi) = a.try_into() else {
                return self.unbound(f);
            };
            let mut workspace = crate::component_live_c::DictionaryWorkspace::default();
            match crate::component_live_c::dictionary_lookup_entry_m(m, abi, &mut workspace, self)
            {
                Ok(value) => return value,
                Err(_) => return self.unbound(f),
            }
        }
        if f == 0x1ea4e0 {
            let Ok(abi): Result<[u32; 7], _> = a.try_into() else {
                return self.unbound(f);
            };
            match crate::component_live_c::lookup_dictionary_live_m(m, abi, 0xe0fd000, self) {
                Ok(rc) => return rc,
                Err(_) => return self.unbound(f),
            }
        }
        if f == 0x1963e0 {
            // Heap resize: the block-resize slot holds a regional twin, so
            // it goes through the region-aware heap call. Same port.
            match crate::component_live_c::heap_call_m_rom(m, f, a, &mut *self.platform, self.rom) {
                Ok(p) => return p,
                Err(_) => {
                    if self.first_unbound.is_none() {
                        self.first_unbound = Some(f);
                    }
                    return 0;
                }
            }
        }
        if f == 0x15d0c4 {
            let Ok(abi) = a.try_into() else {
                return self.unbound(f);
            };
            match crate::component_live_c::read_field_m(m, abi, addr(self.rom, 0x8d7ca0), 0xe0feb00, &mut [0u8; 64])
            {
                Ok(rc) => return rc,
                Err(_) => return self.unbound(f),
            }
        }
        if f == 0x15bf74 {
            let Ok(abi) = a.try_into() else {
                return self.unbound(f);
            };
            return crate::component_live_c::create_child_live_m(m, abi, 0xe0fef80, self);
        }
        if f == 0x15b758 {
            let Ok(abi) = a.try_into() else {
                return self.unbound(f);
            };
            return crate::component_live_c::create_record_live_m(m, abi, 0xe0fee00, self);
        }
        if f == 0x1ede44 {
            let Ok(abi) = a.try_into() else {
                return self.unbound(f);
            };
            return crate::component_live_c::fn_1ede44_m(
                m,
                abi,
                &speech_literals_rom(self.rom),
                0xe0fed00,
                self,
            );
        }
        if f == 0x15bab0 {
            if a.len() < 2 {
                return self.unbound(f);
            }
            return crate::component_live_c::ld_reset_live_m(m, a[0], a[1], 0xe0fef00, self);
        }
        if f == 0x15cdcc {
            let Ok(abi) = a.try_into() else {
                return self.unbound(f);
            };
            return crate::component_live_c::write_field_live_m(m, abi, self);
        }
        if f == 0x15d378 {
            // Field-data read: the descriptor table moves per region, so
            // it goes through the region map (the shared core_call arm
            // stays US). Same port, same ABI, regional descriptors.
            let Ok(abi) = a.try_into() else {
                return self.unbound(f);
            };
            match crate::component_live_c::field_data_m(m, abi, addr(self.rom, 0x8d7ca0)) {
                Ok(rc) => return rc,
                Err(_) => return self.unbound(f),
            }
        }
        if f == 0x15d6c4 {
            // Child enquiry takes the region explicitly; the shared
            // core_call arm stays US. Same port, same ABI.
            let Ok(abi) = a.try_into() else {
                return self.unbound(f);
            };
            match crate::component_live_c::enquire_child_m(m, abi, self.rom) {
                Ok(rc) => return rc,
                Err(_) => return self.unbound(f),
            }
        }
        if f == 0x176f2c || f == 0x176f5c {
            // Heap/archive lock acquire/release: the broker resource open
            // takes it around the archive work. Single-threaded harness,
            // so the real acquire/release runs; contention would route to
            // the owner's callback through this same probe. The comparator
            // never fires off the lock path (no sorted structures here).
            if a.is_empty() {
                return self.unbound(f);
            }
            struct LockHost<'x, 'y> {
                probe: &'x mut Probe1d5ba8<'y>,
            }
            impl crate::boot_part_f::Host for LockHost<'_, '_> {
                fn call(&mut self, m: &mut Memory<'_>, method: u32, args: &[u32]) -> u32 {
                    <Probe1d5ba8<'_> as crate::component_live_c::Host>::call_m(
                        self.probe, method, args, m,
                    )
                }
                fn compare(
                    &mut self,
                    _: &mut Memory<'_>,
                    _: u32,
                    _: &mut crate::boot_part_f::Key,
                    _: &mut u32,
                ) -> u32 {
                    // Unreachable off the lock path (see above).
                    0x80002001
                }
            }
            let mut host = LockHost { probe: self };
            return if f == 0x176f2c {
                crate::boot_port_1::fn_176f2c(m, a[0], &mut host)
            } else {
                crate::boot_port_1::fn_176f5c(m, a[0], &mut host)
            };
        }
        if f == 0x195d90 {
            // Guest strlen: pure port, no literals. Same as the heap
            // stage's binding.
            if a.is_empty() || (a[0] as usize) >= m.bytes.len() {
                return self.unbound(f);
            }
            return crate::frontend::strlen_bytes(&m.bytes[a[0] as usize..]);
        }
        if f == 0x1769d0 {
            // Case-insensitive guest-string compare: pure port, no
            // literals. Same binding as the boot ports.
            if a.len() < 2 {
                return self.unbound(f);
            }
            return crate::fe_text::strcmp_ci_1769d0(m.bytes, a[0], a[1]) as u32;
        }
        if f == 0x1770f8 || f == 0x1771a8 || f == 0x177154 {
            // Archive name compare: pure guest-string compare, no
            // literals. Same grouping as the component lanes.
            if a.len() < 2 {
                return self.unbound(f);
            }
            return crate::speech_part_1::fn_1770f8(m, a[0], a[1]);
        }
        // Dictionary section load/release (the mapped-dictionary load
        // the 1ea4e0 lookup needs): same certified ports as the seg1
        // lane's acronym path, through this probe's region-aware host.
        if f == 0x26b7ec && a.len() >= 4 {
            return crate::component_live_b3::fn_26b7ec_m(m, self, a[0], a[1], a[2], a[3]);
        }
        if f == 0x26bb28 && a.len() >= 2 {
            return crate::component_live_b3::fn_26bb28_m(m, self, a[0], a[1]);
        }
        if f == 0x26bb44 && a.len() >= 1 {
            return crate::component_live_b3::fn_26bb44_m(m, a[0]);
        }
        if f == 0x179f3c {
            // Broker resource close: the open above is always closed in
            // the same root. Same certified port as the component lane's
            // data_close, with region-aware log category.
            if a.len() < 2 {
                return self.unbound(f);
            }
            let mut inner = crate::component_live_c2::OpenHost {
                query: 0xe0fc800,
                pending_release: None,
            };
            let rc = {
                let mut host = crate::component_live_c2::CloseHost { inner: &mut inner };
                crate::leftovers_1::fn_179f3c(m, a[0], a[1], &close_runtime_rom(self.rom), &mut host)
            };
            if let Some(object) = inner.pending_release.take() {
                // Read the heap the same way the component lane does;
                // unmapped reads fail closed instead of panicking.
                let rd = |at: u32| -> Option<u32> {
                    m.bytes
                        .get(at as usize..at as usize + 4)
                        .map(|b| u32::from_le_bytes(b.try_into().unwrap()))
                };
                match rd(a[0] + 4).and_then(|p| rd(p + 8)).and_then(|b| rd(b + 4)) {
                    Some(heap) => crate::boot_part_e::release(m, heap, object, &mut inner),
                    None => return self.unbound(f),
                }
            }
            return rc;
        }
        if f == 0x179f30 {
            // Broker resource open: the EU/KR 1d5ba8 entry states open
            // their system dictionary here (the US entry state arrives
            // with it open). Same certified port as the seg1 lane's
            // acronym path, through this probe's region-aware host.
            if a.len() < 3 {
                return self.unbound(f);
            }
            return crate::component_live_b3::broker_179f30_live(m, a[0], a[1], a[2], self);
        }
        if f == 0x15c544 {
            // LD child lookup: the shared core_call arm guesses the region
            // from the normalized entry and reads the stale US parent table
            // on EU/KR (the colliding 0x15c544 twin). Route the same port
            // through the region-aware dispatcher with the probe's region.
            let mut host = crate::component_live_b::LiveHost {
                platform: &mut *self.platform,
                fault: None,
                scratch: 0xe0fcf00,
            };
            let rc = host.call_m_with(f, a, m, self.rom);
            if host.fault.is_some() {
                return self.unbound(f);
            }
            return rc;
        }
        let result = if [
            0x196394, 0x1963b8, 0x196428, 0x4a779c, 0x4a7758, 0x4a774c, 0x39f5e8, 0x39f5b8,
        ]
        .contains(&f)
        {
            crate::component_live_c::heap_call_m(m, f, a, &mut *self.platform)
        } else {
            crate::component_live_c::core_call_m_rom(m, f, a, self.rom)
        };
        match result {
            Ok(rc) => rc,
            Err(_) => {
                if self.first_unbound.is_none() {
                    self.first_unbound = Some(f);
                }
                if [0x196394, 0x1963b8, 0x1963e0].contains(&f) {
                    0
                } else {
                    0x80002001
                }
            }
        }
    }
}

/// Broker-close runtime with the region-aware log category (the format
/// addresses are unlisted, i.e. identical everywhere; the scratch window
/// is disjoint from this probe's other windows and the broker windows).
fn close_runtime_rom(rom: Rom) -> crate::leftovers_1::CloseRuntime {
    let addr = |us| crate::rom_region::addr(rom, us);
    let hash = |bytes: &[u8]| {
        bytes
            .iter()
            .fold(2166136261u32, |h, &b| (h ^ b as u32).wrapping_mul(16777619))
    };
    crate::leftovers_1::CloseRuntime {
        category: addr(0x8d7668),
        begin: addr(0x17a14c),
        end: addr(0x17a18c),
        error: addr(0x17a180),
        handle_key: hash(b"hData"),
        status_key: hash(b"lhError"),
        scratch: 0xe0fc840,
        fold: crate::component_live_c2::ascii_fold_classes(),
    }
}

/// Dispatch arm for the `0x1d5ba8` root: the certified lane-c port over
/// the caller's arena. `args` is the 7-word captured call vector
/// `[p, tag, h0, h1, pending, s1, s2]`; the fifth word is the ARM's
/// stack argument (the pending-flag address). Returns the root status
/// and the first unbound callee, if any.
pub fn run_1d5ba8(
    m: &mut Memory<'_>,
    args: &[u32],
    platform: &mut dyn engine::HeapPlatform,
) -> (u32, Option<u32>) {
    run_1d5ba8_rom(m, args, platform, Rom::Us)
}

/// Region-aware `run_1d5ba8`: the probe dispatch normalizes regional
/// guest entries US-first, and the environment literals go through the
/// region map. No per-region literals: every guest address is the US
/// canonical form.
pub fn run_1d5ba8_rom(
    m: &mut Memory<'_>,
    args: &[u32],
    platform: &mut dyn engine::HeapPlatform,
    rom: Rom,
) -> (u32, Option<u32>) {
    let mut host = Probe1d5ba8::new_rom(platform, rom);
    let env = process_environment_rom(rom);
    let rc = crate::component_live_c::fn_1d5ba8_m(
        m,
        args[0],
        args[1],
        [args[2], args[3]],
        args[4],
        &env,
        0xe0ff080,
        &mut host,
    );
    (rc, host.first_unbound)
}

/// Probe host for the `0x200e64` root: the component test's dispatch
/// (`tests/component_live_c.rs` `live_200e64`), in `no_std` shape.
pub struct Probe200e64<'a> {
    pub first_unbound: Option<u32>,
    platform: &'a mut dyn engine::HeapPlatform,
    rom: Rom,
}

impl<'a> Probe200e64<'a> {
    pub fn new(platform: &'a mut dyn engine::HeapPlatform) -> Self {
        Probe200e64::new_rom(platform, Rom::Us)
    }
    pub fn new_rom(platform: &'a mut dyn engine::HeapPlatform, rom: Rom) -> Self {
        Probe200e64 {
            first_unbound: None,
            platform,
            rom,
        }
    }
    fn unbound(&mut self, f: u32) -> u32 {
        if self.first_unbound.is_none() {
            self.first_unbound = Some(f);
        }
        0x80002001
    }
}

impl crate::component_live_c::Host for Probe200e64<'_> {
    fn rom(&self) -> Rom {
        self.rom
    }
    fn call_m(&mut self, f: u32, a: &[u32], m: &mut Memory<'_>) -> u32 {
        // Same US-first normalization as the 1d5ba8 probe: direct
        // US-literal calls keep working while regional guest twins bind.
        let f = to_us(self.rom, f);
        if f == 0x173dc0 {
            // Error log: the certified component test answers it with 0
            // (log-and-continue). Reached on the KR entry when the
            // records alloc fails; the port then frees and returns its
            // status, exactly like the ARM path.
            return 0;
        }
        if f == 0x15cdcc {
            let Ok(abi) = a.try_into() else {
                return self.unbound(f);
            };
            return crate::component_live_c::write_field_live_m(m, abi, self);
        }
        if f == 0x1ff4fc {
            let Ok(abi) = a.try_into() else {
                return self.unbound(f);
            };
            return crate::component_live_c::insert_prompt_live_m(m, abi, 0xe0ff600, self);
        }
        let result = if [0x196394, 0x1963b8, 0x196428].contains(&f) {
            crate::component_live_c::heap_call_m(m, f, a, &mut *self.platform)
        } else {
            crate::component_live_c::core_call_m_rom(m, f, a, self.rom)
        };
        match result {
            Ok(rc) => rc,
            Err(e) => {
                if self.first_unbound.is_none() {
                    self.first_unbound = Some(f);
                }
                let _ = e;
                0x80002001
            }
        }
    }
}

/// Dispatch arm for the `0x200e64` root: the certified prompt-orthography
/// port over the caller's arena. `args` is the 7-word captured call
/// vector `[p, tag, h0, h1, active, s1, s2]`; the fifth word is the ARM's
/// stack argument (the active-flag address). Returns the root status and
/// the first unbound callee, if any.
pub fn run_200e64(
    m: &mut Memory<'_>,
    args: &[u32],
    platform: &mut dyn engine::HeapPlatform,
) -> (u32, Option<u32>) {
    run_200e64_rom(m, args, platform, Rom::Us)
}

/// Region-aware `run_200e64`: the probe dispatch normalizes regional
/// guest entries US-first.
pub fn run_200e64_rom(
    m: &mut Memory<'_>,
    args: &[u32],
    platform: &mut dyn engine::HeapPlatform,
    rom: Rom,
) -> (u32, Option<u32>) {
    let mut host = Probe200e64::new_rom(platform, rom);
    let rc = crate::component_live_c::prompt_orth_live_m(
        m,
        args[0],
        args[1],
        [args[2], args[3]],
        args[4],
        0xe0ff700,
        &mut host,
    );
    (rc, host.first_unbound)
}

/// The mapped "At End of Processing" literal the `0x1e4b7c` root takes.
/// Same scan as the live dispatcher's `source_literal`: the first
/// NUL-terminated match in the mapped low segment.
fn source_literal(bytes: &[u8], text: &[u8]) -> u32 {
    let end = bytes.len().min(0x500000);
    if end <= 0x100000 {
        return 0;
    }
    (0x100000
        + bytes[0x100000..end]
            .windows(text.len() + 1)
            .position(|w| &w[..text.len()] == text && w[text.len()] == 0)
            .expect("mapped source literal")) as u32
}

/// Map a live-host fault to the entry that caused it, for the
/// first-unbound report (a real start in data/functions.json).
fn fault_entry(e: &engine::Error) -> u32 {
    match *e {
        engine::Error::UnsupportedStage { entry } => entry,
        engine::Error::MissingStreamAdapter { entry } => entry,
        _ => 0,
    }
}

/// Dispatch arm for the `0x1e4b7c` root: the certified lane-b/b2 token
/// port over the caller's arena, with the component test's host (exactly
/// as `tests/component_live_b2.rs` binds it: `TokenHost` over `LiveHost`
/// scratch `0x500`, root scratch `0x600`, owner heap from the entry
/// arena). `args` is the 7-word captured call vector
/// `[p, tag, h0, h1, flag, s1, s2]`; the fifth word is the ARM's stack
/// argument. Returns the root status and the first unbound callee, if
/// any (`LiveHost` fails closed by recording its fault; a `faulted`
/// panic names the unbound entry directly).
pub fn run_1e4b7c(
    m: &mut Memory<'_>,
    args: &[u32],
    platform: &mut dyn engine::HeapPlatform,
) -> (u32, Option<u32>) {
    run_1e4b7c_rom(m, args, platform, Rom::Us)
}

/// Region-aware `run_1e4b7c`.
///
/// The `TokenHost` dispatch (component_live_b2, owned by the seg1 lane)
/// matches the port's direct calls, which are US literals on every region,
/// and forwards the rest to `LiveHost`, which derives the region per entry
/// (`Rom::from_entry`) and maps its data literals through the region map.
/// Guest-sourced regional twins therefore bind their US arm with the right
/// region; no per-region literal is needed here. If a future regional run
/// reports a first-unbound inside the token subtree, the fix belongs in
/// `TokenHost::dispatch` (translate there and thread `rom` to the inner
/// host), not here.
pub fn run_1e4b7c_rom(
    m: &mut Memory<'_>,
    args: &[u32],
    platform: &mut dyn engine::HeapPlatform,
    _rom: Rom,
) -> (u32, Option<u32>) {
    let mut host = crate::component_live_b2::TokenHost {
        inner: crate::component_live_b::LiveHost {
            platform,
            fault: None,
            scratch: 0x500,
        },
        heap: 0,
    };
    // seg6 r22: p->context->allocator (src/fn_1e4b7c.c): handle [args0]
    // -> process -> context -> heap [context+4]. Same triple deref as
    // the live dispatch arm (fixed there in r21).
    host.heap = m.word(m.word(m.word(args[0])) + 4);
    let end = source_literal(m.bytes, b"At End of Processing");
    let a5 = [args[0], args[1], args[2], args[3], args[4]];
    let rc = crate::component_live_b::fn_1e4b7c_m(m, a5, end, 0x600, &mut host);
    let unbound = match &host.inner.fault {
        None => {
            if rc >= 0x80000000 {
                Some(0x1e4b7c)
            } else {
                None
            }
        }
        Some(e) => {
            let en = fault_entry(e);
            Some(if en == 0 { 0x1e4b7c } else { en })
        }
    };
    (rc, unbound)
}

/// The `fn_15429c` per-component advance between a step return and the
/// next root entry: the word at `0x14c403e4` then the halfword at
/// `0x14c403e8` (ARM pcs `0x155524` and `0x154304`). Scheduler-sequencer
/// state owned by the `0x15429c` port; replicated here only for the
/// back-to-back stretch run.
pub fn scheduler_advance_after_step(arena: &mut [u8]) {
    let w = u32::from_le_bytes(arena[0x14c403e4..0x14c403e8].try_into().unwrap());
    arena[0x14c403e4..0x14c403e8].copy_from_slice(&w.wrapping_add(1).to_le_bytes());
    let h = u16::from_le_bytes(arena[0x14c403e8..0x14c403ea].try_into().unwrap());
    arena[0x14c403e8..0x14c403ea].copy_from_slice(&h.wrapping_add(1).to_le_bytes());
}
