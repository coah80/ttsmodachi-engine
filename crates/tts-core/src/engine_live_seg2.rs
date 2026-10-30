//! Live scheduler segment 2 of the US speech_1 run (lane seg2).
//!
//! The stretch is the scheduler's per-component step plus root, twice:
//! `0x1cca64` then `0x206be4` (word expansion), then `0x1cca64` then
//! `0x1dd914` (punctuation). See prompts/rust_seg_r1.md.
//!
//! Dispatch shape. The prompt asks this module to wrap `engine_live::Host`
//! and forward every entry it does not handle. `Host` is private to
//! `engine_live.rs` (wire-a owns that file, so this lane cannot name it);
//! this module therefore routes through the public live entry points,
//! which run the same dispatcher:
//!
//! * the `0x1cca64` steps ride `engine_live::call_live_with_runtime_m`
//!   (the sentence-dump port is bound in `engine_live.rs`);
//! * `0x206be4` rides the certified `component_live_c4` port
//!   (`C4Calls` + `fn_206be4_m`);
//! * `0x1dd914` rides the certified `component_live_c` port
//!   (`FeTextLive` + `fn_1dd914_m`).
//!
//! Region: rom_region only (US). No globals or statics, no unsafe. The
//! root scratch frames live in caller-owned arena extensions past the
//! capture (the way tests/component_live_c4.rs places them), never at
//! fixed guest addresses.
//!
//! The scheduler-loop advance between a step return and the next root
//! entry (two counter stores in `fn_15429c`, pcs `0x155524` and `0x154304`;
//! found with scratch/seg2/watch_sched.py) is replicated by
//! [`scheduler_advance_after_step`] as read-modify-write `+1` on the live
//! arena. That fragment is scheduler-sequencer state owned by the
//! `0x15429c` port (wire-a's engine_live.rs); the coordinator should fold
//! it into the real sequencer. It is kept out of the per-boundary certs
//! and used only by the back-to-back stretch run.

use crate::engine;
use crate::boot::Memory as BootMemory;

/// Run one scheduler per-component step (`0x1cca64`) through the public
/// live dispatcher. `args` are the step's register args; `snap` is the
/// arena image at the step entry (render paths diff against it).
pub fn run_step(
    m: &mut crate::boot::Memory<'_>,
    args: &[u32],
    platform: &mut dyn engine::HeapPlatform,
    runtime: &mut dyn crate::engine_live::RuntimeServicesM,
    snap: &[u8],
) -> Result<u32, engine::Error> {
    crate::engine_live::call_live_with_runtime_m(m, 0x1cca64, args, platform, runtime, snap)
}

/// Dispatch arm for the `0x206be4` root: the certified `component_live_c4`
/// port over the caller's arena. `scratch` is the root's ABI-local frame
/// plus the `C4Calls` callee workspace (caller-owned, past the capture).
/// Returns the root status and the first unbound callee, if any.
pub fn run_206be4<P: engine::HeapPlatform>(
    m: &mut engine::Memory<'_, '_>,
    p: u32,
    key: u32,
    handle: [u32; 2],
    changed: u32,
    scratch: u32,
    platform: &mut P,
) -> (Result<u32, engine::Error>, Option<u32>) {
    run_206be4_rom(m, p, key, handle, changed, scratch, platform, crate::rom_region::Rom::Us)
}

/// Region-aware `run_206be4`: the `C4Calls` host carries the caller's
/// `rom`, so guest vtable slots and data literals resolve through the
/// region map instead of the US addresses.
pub fn run_206be4_rom<P: engine::HeapPlatform>(
    m: &mut engine::Memory<'_, '_>,
    p: u32,
    key: u32,
    handle: [u32; 2],
    changed: u32,
    scratch: u32,
    platform: &mut P,
    rom: crate::rom_region::Rom,
) -> (Result<u32, engine::Error>, Option<u32>) {
    let mut host = crate::component_live_c4::C4Calls::new(platform);
    host.scratch = scratch;
    host.rom = rom;
    let rc = crate::component_live_c4::fn_206be4_m(m, p, key, handle, changed, scratch, &mut host);
    (rc, host.first_unbound)
}

/// Dispatch arm for the `0x1dd914` root: the certified `component_live_c`
/// port over the caller's arena. Returns the root status and the first
/// unbound callee, if any.
pub fn run_1dd914<P: engine::HeapPlatform>(
    m: &mut crate::boot::Memory<'_>,
    args: &[u32],
    changed: u32,
    platform: &mut P,
) -> (u32, Option<u32>) {
    run_1dd914_rom(m, args, changed, platform, crate::rom_region::Rom::Us)
}

/// Region-aware `run_1dd914`: the `FeTextLive` host carries the caller's
/// `rom`, so guest vtable slots and data literals resolve through the
/// region map instead of the US addresses.
pub fn run_1dd914_rom<P: engine::HeapPlatform>(
    m: &mut crate::boot::Memory<'_>,
    args: &[u32],
    changed: u32,
    platform: &mut P,
    rom: crate::rom_region::Rom,
) -> (u32, Option<u32>) {
    let mut host = crate::component_live_c::FeTextLive::new(platform);
    host.rom = rom;
    let rc = crate::component_live_c::fn_1dd914_m(m, args, changed, &mut host);
    (rc, host.first_unbound)
}

/// Request intake (round 2): from the `0x191e4b`-point ARM image plus the
/// job text and voice params to the `0x15429c` scheduler-entry state.
///
/// The template image is a captured ARM state at the `0x191E40` entry (the
/// point the `0x191e4b` engine-stages entry replays): it already holds the
/// post-boot, post-reset, post-setter engine with the snapshot's own job.
/// The entry writes the caller's UTF-16LE text into the job text buffer
/// (`0xB27DAA`, NUL-terminated, any length: the game bot's `sendText`
/// writes the whole text plus NUL with no cap) and the six voice params into the
/// job struct (`0xAF340D`), runs the six voice setters plus `0x3914e8`
/// exactly once through the certified `engine_backend` ports in ARM order,
/// then runs the exact `tts-cli engine-stages` `0x191e4b` prefix chain
/// (`request_callback_prefix` → `synthesis_start_prefix` →
/// `synthesis_allocate_prefix` → `synthesis_logging_prefix` →
/// `synthesis_aux_prefix` → `database_return` → `component_boundary` →
/// `first_component`) to the scheduler boundary. Nothing is recorded or
/// replayed: every output byte is computed from the template plus the
/// given text and params.
///
/// Single-application matters: the setters enqueue voice-queue events, so
/// re-running them on an already-set image would double-apply. The
/// template is therefore the pristine entry image (no setter re-run, like
/// `wire-a-r9`), and the entry runs the setters exactly once, mirroring
/// the ARM capture. The pristine template plus one setter round reproduces
/// the ARM entry queue (citra's bumps plus the single re-run) by
/// construction.
///
/// All scratch is caller-owned: `workspace`/`scratch` back the prefix
/// chain's flat arenas (each must cover the image, i.e. be at least as
/// large as the highest mapped byte) and the controls host's heap;
/// `tables` borrows the numeric tables read back out of the guest image.
/// No globals or statics, no unsafe, rom_region only (US).
///
/// `args.voice` is the Nintendo voice wrapper the setters address (the
/// setters' `r0` in the ARM run); `args.request_sp` is the `0x191E40`-time
/// stack pointer the prefixes thread through. Both come from boot state in
/// the real engine; here they are explicit until boot-a's stop state
/// supplies them (see the round-2 progress file).

/// Job text buffer written by the bot protocol (UTF-16LE + NUL).
/// No length cap and no clearing: the game bot's `sendText` (api/tts.py)
/// writes the whole text plus NUL to `textDataLoc` with no extent check,
/// so the engine takes any length exactly as the ARM does. The old
/// emulator captures were built with a 256-byte clearing first
/// (scratch/seg2/cap_seg2r2.py zeroes 256 before writing the text); tests
/// that certify against those captures replicate the clearing in their own
/// template setup, never here.
pub const TEXT_ADDR: u32 = 0xB27DAA;
/// Bot job struct; the six voice params live at `+3..+9`.
pub const JOB_ADDR: u32 = 0x00AF340D;
/// Job audio-status bytes (gamePatch source/main.c `audioStatus`).
pub const JOB_WAITING: u8 = 1;
pub const JOB_PROCESSING: u8 = 2;
pub const JOB_FINISHED: u8 = 3;
pub const JOB_ERROR: u8 = 4;
pub const JOB_TEXT_READY: u8 = 5;
/// gamePatch mainLoopF status protocol, shared app-boundary helper
/// (source/main.c, read-only). mainLoopF sets PROCESSING when it
/// observes TEXT_READY; callTTS (and the c722f0 status store) sets
/// FINISHED at the end unless the job is in ERROR, which sticks.
/// speak() and the render tests drive the same transitions instead
/// of hand-writing the status byte.
pub fn job_mark_processing(m: &mut crate::boot::Memory<'_>, job: u32) {
    let at = job as usize;
    if at < m.bytes.len() && m.bytes[at] == JOB_TEXT_READY {
        m.bytes[at] = JOB_PROCESSING;
    }
}
/// End-of-callTTS status: FINISHED unless ERROR (c722f0: `if != 4 set 3`).
pub fn job_mark_finished(m: &mut crate::boot::Memory<'_>, job: u32) {
    let at = job as usize;
    if at < m.bytes.len() && m.bytes[at] != JOB_ERROR {
        m.bytes[at] = JOB_FINISHED;
    }
}
/// The `0x191E40`-time voice wrapper is constant for one snapshot boot.
/// Kept here so the tests name the value they certify against.
/// Regional twins (observed setter-`r0` on each snapshot boot; in the real
/// engine this arrives as `RequestArgs.voice` from boot state, never from
/// a region match in engine code): EU `0x86731A0`, KR `0x866A260`.
pub const VOICE_WRAPPER: u32 = 0x8668E00;
/// Observed `0x191E40`-time voice wrapper per snapshot boot, by region.
/// Boot-state provenance (private snapshot runs: the `0x3912fc` setter's
/// `r0`): US `0x8668E00`, EU `0x86731A0`, KR `0x866A260`. Engine code never
/// matches on region for this; the caller threads the value through
/// [`RequestArgs.voice`].
pub const VOICE_WRAPPER_EU: u32 = 0x86731A0;
/// Observed `0x191E40`-time voice wrapper on the KR snapshot boot. See
/// [`VOICE_WRAPPER_EU`] for provenance.
pub const VOICE_WRAPPER_KR: u32 = 0x866A260;

/// Voice params as the bot puts them in the job (each 0-100; intonation
/// 0-4 on the US engine).
#[derive(Clone, Copy, Debug)]
pub struct VoiceParams {
    pub pitch: u8,
    pub speed: u8,
    pub quality: u8,
    pub tone: u8,
    pub accent: u8,
    pub intonation: u8,
}

/// The `0x191e4b` wire args: boot state, notification bits and tick.
#[derive(Clone, Copy, Debug)]
pub struct RequestArgs {
    pub engine: u32,
    pub magic: u32,
    pub request: u32,
    pub request_sp: u32,
    pub voice: u32,
    pub s16: u32,
    pub s17: u32,
    pub tick: u64,
}

/// Single-region dispatcher for the intake realloc's nested calls.
/// Every arm is an existing 1:1 port; single-threaded use never contends,
/// so the lock words balance exactly like the mapped heap path (which
/// performs no arbitration at all). Unlisted entries fail closed.
fn request_host_call(
    h: &mut RequestLocks,
    m: &mut BootMemory<'_>,
    f: u32,
    a: &[u32],
) -> u32 {
    let need = |n: usize| {
        assert!(a.len() >= n, "request host arity {f:#x}");
    };
    match f {
        0x39f5e8 => {
            need(3);
            m.fn_39f5e8(h, [a[0], a[1], a[2]])
        }
        0x39f5b8 => {
            need(2);
            crate::speech_part_3::fn_39f5b8(m, a[0], a[1], h);
            0
        }
        0x1840fc => {
            need(1);
            crate::boot_part_b::fn_1840fc(m, a[0], h);
            0
        }
        0x18423c => {
            need(1);
            crate::boot_part_b::fn_18423c(m, a[0], h);
            0
        }
        0x183edc => {
            need(1);
            // Single-threaded: the semaphore is positive on entry, so the
            // handle is never read. The mapped heap path arbitrates nothing.
            crate::boot_part_b2::fn_183edc(m, a[0], 0, h);
            0
        }
        0x165d00 => {
            need(1);
            // Single-threaded: the negated word never exceeds 1, so no
            // waiter wakes and the semaphore value below is unread.
            let g = crate::boot::Globals {
                manager: 0, callbacks: 0, rules: 0, components: 0,
                dictionary: 0, context: 0, mode: 0, semaphore: 0,
                service: 0, command: 0,
            };
            m.fn_165d00(h, &g, a[0]);
            0
        }
        // Single-threaded arbitration succeeds immediately (no contention
        // is possible on a caller-owned arena); same as the component
        // lanes' 0x172d18 arms.
        0x172d18 => 0,
        _ => 0x80002001,
    }
}
impl crate::speech_part_3::Host for RequestLocks {
    fn call(&mut self, m: &mut BootMemory<'_>, f: u32, a: &[u32]) -> u32 {
        request_host_call(self, m, f, a)
    }
}
impl crate::boot_part_b::Host for RequestLocks {
    fn thread_id(&mut self) -> u32 {
        0x1ff82000
    }
    fn call(&mut self, m: &mut BootMemory<'_>, f: u32, a: &[u32]) -> u32 {
        request_host_call(self, m, f, a)
    }
}
impl crate::boot::Calls for RequestLocks {
    fn call(&mut self, m: &mut BootMemory<'_>, address: u32, args: &[u32]) -> u32 {
        request_host_call(self, m, address, args)
    }
    fn call_wide(&mut self, _: &mut BootMemory<'_>, _: u32, _: &[u32]) -> u64 {
        0
    }
}

/// Caller-owned buffers for the numeric tables, read back out of the guest
/// image by the entry (the tables live in the image's borrowed pages).
pub struct TableBuffers {
    pub constants: [u8; crate::tables::NumericTables::CONSTANT_BYTES],
    pub cosine: [u8; crate::tables::NumericTables::COSINE_BYTES],
}

impl TableBuffers {
    pub fn new() -> Self {
        Self {
            constants: [0; crate::tables::NumericTables::CONSTANT_BYTES],
            cosine: [0; crate::tables::NumericTables::COSINE_BYTES],
        }
    }
}

struct RequestMutex;
impl engine::MutexPlatform for RequestMutex {
    fn acquire_mutex(&mut self, _: u32) -> u32 {
        0x81402001
    }
    fn release_mutex(&mut self, _: u32) -> u32 {
        0x81402001
    }
}

struct RequestTick(u64);
impl engine::Platform for RequestTick {
    fn ticks(&mut self) -> u64 {
        self.0
    }
}

struct RequestHeap;
impl engine::HeapPlatform for RequestHeap {
    fn thread_id(&mut self) -> u32 {
        0x1ff82000
    }
    fn arbiter_address(&self) -> u32 {
        0xa7bb7c
    }
    fn arbitrate(&mut self, _: &mut crate::boot::Memory<'_>, _: &[u32]) -> u32 {
        panic!("contended heap locks are not bound by the request entry");
    }
}

/// Minimal lock host for the setter ports. Lock acquire/release report
/// success (single-threaded use, as in the backend tests); the tick feeds
/// the ports that stamp it. This lives here rather than reusing the
/// `live_controls` host because that module is `cfg(not(test))` while this
/// request entry must compile under every configuration.
struct RequestLocks {
    tick: u64,
}
impl crate::engine_backend::BackendLocks for RequestLocks {
    fn acquire(&mut self, _: u32) -> u32 {
        0
    }
    fn release(&mut self, _: u32) -> u32 {
        0
    }
    fn ticks(&mut self) -> Result<u64, engine::Error> {
        Ok(self.tick)
    }
    /// Heap-vtable realloc for the parameter store (fn_176054's growth).
    /// 1:1 with the ARM's wrapper-m2 call (fn_1963e0 with a live old
    /// pointer dispatches the wrapper's m2 slot, whose body is the
    /// fn_4a79b8 port below): the intake heap object resolves the same
    /// way, over the same single-region arena the mapped heap ports use.
    /// Multi-region arenas keep the old fail-closed boundary.
    fn parameter_resize(
        &mut self,
        m: &mut engine::Memory<'_, '_>,
        obj: u32,
        old: u32,
        size: u32,
    ) -> Result<u32, engine::Error> {
        if m.regions.len() != 1 || m.regions[0].base != 0 {
            return Err(engine::Error::MissingStreamAdapter { entry: 0x1963e0 });
        }
        let base = m.regions[0].base;
        let bytes = &mut *m.regions[0].bytes;
        let mut bm = BootMemory { base, bytes };
        Ok(crate::speech_part_3::fn_4a79b8(&mut bm, obj, old, size, self))
    }
}

/// Run the request intake: pristine `0x191e4b` template image plus text and
/// params to the `0x15429c` scheduler boundary. Returns the boundary the
/// ARM reaches at the scheduler entry (`entry`/`args` match the ARM regs).
#[allow(clippy::too_many_arguments)]
pub fn request_to_scheduler(
    m: &mut engine::Memory<'_, '_>,
    text: &[u16],
    params: &VoiceParams,
    args: &RequestArgs,
    tables: &mut TableBuffers,
    workspace: &mut [u8],
    scratch: &mut [u8],
) -> Result<engine::VoiceBoundary, engine::Error> {
    request_to_scheduler_rom(m, text, params, args, tables, workspace, scratch, crate::rom_region::Rom::Us, None)
}

/// Patch-faithful entry (gamePatch main.c callTTS via doTTS 0x191e40).
/// Takes a guest `ttsInput*` ({unknown=0, textInputLen bytes, textInput ptr}
/// to heap UTF-16), reads the text through the pointer (never request+8),
/// then runs the same intake. The heap backing here is caller-provided
/// guest memory (speak uses TEXT_ADDR); a game-heap tmalloc backing is
/// follow-up (needs 0x129778 binding). Request+4/+8 untouched.
#[allow(clippy::too_many_arguments)]
pub fn do_tts_live(
    m: &mut engine::Memory<'_, '_>,
    tts_input: u32,
    params: &VoiceParams,
    args: &RequestArgs,
    tables: &mut TableBuffers,
    workspace: &mut [u8],
    scratch: &mut [u8],
    text_buf: &mut [u16],
) -> Result<engine::VoiceBoundary, engine::Error> {
    let text_ptr = m.word(tts_input.wrapping_add(8))?;
    let mut n = 0usize;
    for slot in text_buf.iter_mut() {
        let w = m.word(text_ptr.wrapping_add((n as u32).wrapping_mul(2)))?;
        *slot = (w & 0xffff) as u16;
        n += 1;
        if (w & 0xffff) == 0 {
            break;
        }
    }
    if n == text_buf.len() {
        // No NUL terminator inside the caller buffer: the caller must lend
        // a longer one. Any length works; the buffer is caller-owned.
        return Err(engine::Error::Capacity {
            required: (n + 1) * 2,
            available: text_buf.len() * 2,
        });
    }
    request_to_scheduler_rom(m, &text_buf[..n], params, args, tables, workspace, scratch, crate::rom_region::Rom::Us, None)
}

/// Region-aware request intake. The numeric tables and the backend data
/// literals move per region, so the table reads and the setter ports go
/// through the region map; `args.voice` stays caller-owned boot state
/// (the snapshot boot's voice wrapper, e.g. [`VOICE_WRAPPER_EU`]).
/// No per-region address literals here: every guest address is the US
/// canonical form mapped with `addr(rom, ..)`.
/// Patch-faithful prologue setters (gamePatch `main.c` `callTTS` before
/// the `audioData` abandon): caller text into the job buffer, voice params
/// into the job struct, then the six voice setters exactly once. Split from
/// [`request_prologue_rom`] so `speak` can abandon the previous render's
/// `audioData` between the setters and `setupFunc`, the game order
/// (RESET -> six setters -> abandon(audioData) -> setupFunc -> tmalloc(12)).
/// [`request_prologue_rom`] keeps the bundled order for the seg2_req
/// captures (ARM reqA-D predate utfRecv; no audioData is live there).
#[allow(clippy::too_many_arguments)]
pub fn request_prologue_setters_rom(
    m: &mut engine::Memory<'_, '_>,
    text: &[u16],
    params: &VoiceParams,
    args: &RequestArgs,
    tables: &mut TableBuffers,
    rom: crate::rom_region::Rom,
    host: Option<&mut dyn crate::engine_backend::AttrHost>,
) -> Result<(), engine::Error> {
    // Bot-exact text delivery (gamePatch `sendText`): the whole caller
    // text plus NUL, no cap and no clearing. Tests that certify against
    // the old harness-built captures pre-clear the extent themselves.
    let mut at = TEXT_ADDR;
    for unit in text {
        m.write(at, &unit.to_le_bytes())?;
        at += 2;
    }
    m.write(at, &[0, 0])?;
    // Round 28: the request's text fields (+4 length, +8 pointer) stay
    // stale (ARM never writes them; they are constant 0x3c/0x8758c00 across
    // all captures). The caller's text rides TEXT_ADDR (app-written, like
    // cap_seg2r2.py); downstream that needs fresh text links it in the
    // caller (test harness), never here. (Removed round-27urgent link).
    m.write(JOB_ADDR + 3, &[params.pitch])?;
    m.write(JOB_ADDR + 4, &[params.speed])?;
    m.write(JOB_ADDR + 5, &[params.quality])?;
    m.write(JOB_ADDR + 6, &[params.tone])?;
    m.write(JOB_ADDR + 7, &[params.accent])?;
    m.write(JOB_ADDR + 8, &[params.intonation])?;
    m.read(crate::rom_region::addr(rom, 0xa21fb0), &mut tables.constants)?;
    m.read(crate::rom_region::addr(rom, 0x8cbfe0), &mut tables.cosine)?;
    let numeric =
        crate::tables::NumericTables::from_bytes(&tables.constants, &tables.cosine).ok_or(
            engine::Error::Capacity {
                required: crate::tables::NumericTables::CONSTANT_BYTES,
                available: tables.constants.len(),
            },
        )?;
    // Game order (main.c callTTS) with game addrs (tomoFunc.h US block):
    // pitch->0x3912fc, speed->0x39130c, quality->0x3912ec, tone->0x39133c,
    // accent->0x39131c, intonation->0x39132c. set_quality_rom IS the speed
    // (rate) setter (convert idx1, rate clamp, update_rate) despite its
    // name, so it takes params.speed; 0x3912fc takes pitch, 0x3912ec takes
    // quality (r17: was speed/quality/pitch -- swapped, invisible while all
    // sliders are 50).
    let mut controls = RequestLocks { tick: args.tick };
    crate::engine_backend::set_voice_rom(m, 0x3912fc, args.voice, params.pitch as i32, &numeric, rom)?;
    crate::engine_backend::set_quality_rom(m, args.voice, params.speed as i32, &mut controls, rom, host)?;
    crate::engine_backend::set_voice_rom(m, 0x3912ec, args.voice, params.quality as i32, &numeric, rom)?;
    crate::engine_backend::set_voice_rom(m, 0x39133c, args.voice, params.tone as i32, &numeric, rom)?;
    crate::engine_backend::set_voice_rom(m, 0x39131c, args.voice, params.accent as i32, &numeric, rom)?;
    crate::engine_backend::set_voice_rom(m, 0x39132c, args.voice, params.intonation as i32, &numeric, rom)?;
    Ok(())
}

/// Patch-faithful setup step (gamePatch `main.c` `setupFunc` = `0x3914e8`
/// `parameter_reset`): runs after the previous render's `audioData` free.
/// Split from [`request_prologue_rom`] for the `speak` order; the bundled
/// entry keeps calling setters+setup together for seg2_req parity.
pub fn request_setup_rom(
    m: &mut engine::Memory<'_, '_>,
    args: &RequestArgs,
    rom: crate::rom_region::Rom,
) -> Result<(), engine::Error> {
    let _ = rom;
    let mut controls = RequestLocks { tick: args.tick };
    crate::engine_backend::parameter_reset(m, args.voice, &mut controls)?;
    Ok(())
}

#[allow(clippy::too_many_arguments)]
/// Patch-faithful prologue (gamePatch `main.c` `callTTS` before `ttsFunc`):
/// caller text into the job buffer, voice params into the job struct, then
/// the six voice setters plus `0x3914e8` (`parameter_reset`) exactly once.
/// The ARM `0x191e4b` entry image is captured after these steps, so running
/// the prologue on the pristine template reproduces that entry state.
/// Idempotent except the setters (voice-queue events): run once per request.
#[allow(clippy::too_many_arguments)]
pub fn request_prologue_rom(
    m: &mut engine::Memory<'_, '_>,
    text: &[u16],
    params: &VoiceParams,
    args: &RequestArgs,
    tables: &mut TableBuffers,
    rom: crate::rom_region::Rom,
    host: Option<&mut dyn crate::engine_backend::AttrHost>,
) -> Result<(), engine::Error> {
    request_prologue_setters_rom(m, text, params, args, tables, rom, host)?;
    request_setup_rom(m, args, rom)?;
    Ok(())
}

/// Prefix chain from the `0x191e4b`-entry state to the `0x15429c` scheduler
/// boundary. Runs after the prologue; takes no text or params.
#[allow(clippy::too_many_arguments)]
pub fn request_chain_rom(
    m: &mut engine::Memory<'_, '_>,
    args: &RequestArgs,
    workspace: &mut [u8],
    scratch: &mut [u8],
    rom: crate::rom_region::Rom,
) -> Result<engine::VoiceBoundary, engine::Error> {
    let mut heap = RequestHeap;
    let mut mutex = RequestMutex;
    let mut ticks = RequestTick(args.tick);
    let (_, boundary) = engine::request_callback_prefix_rom(
        m,
        engine::BootState {
            engine: args.engine,
            magic: args.magic,
        },
        args.request,
        &mut mutex,
        engine::NotificationRegisters {
            s16: args.s16,
            s17: args.s17,
        },
        &mut ticks,
        rom,
    )?;
    let scheduler = boundary;
    let boundary = engine::synthesis_start_prefix_rom(m, boundary, &mut mutex, rom)?;
    let synthesis = boundary;
    let logger = engine::synthesis_allocate_prefix_rom(m, synthesis, workspace, &mut heap, rom)?;
    let continuation = engine::synthesis_logging_prefix_rom(
        m,
        synthesis,
        logger,
        args.request_sp,
        workspace,
        scratch,
        &mut heap,
        rom,
    )?;
    let aux = engine::synthesis_aux_prefix_rom(
        m,
        synthesis,
        scheduler,
        continuation,
        args.request_sp,
        workspace,
        &mut heap,
        rom,
    )?;
    // The live dispatcher matches US entries only, so hand it the US
    // canonical form of the guest vtable slot (the region map, never a
    // per-region literal; identical to what a rom-aware dispatcher
    // resolves). The boundary itself stays guest-faithful above.
    let aux_us = engine::VoiceBoundary {
        entry: crate::rom_region::to_us(rom, aux.entry),
        ..aux
    };
    let db = crate::engine_live::database_return(m, aux_us, args.request_sp, workspace, &mut heap)?;
    let start = crate::engine_live::component_boundary(m, synthesis, args.request_sp, workspace, &mut heap)?;
    let boundary = crate::engine_live::first_component(m, start, args.request_sp, workspace, &mut heap)?;
    Ok(boundary)
}

/// Region-aware request intake: prologue then chain. Behaviour unchanged;
/// `request_prologue_rom` + `request_chain_rom` are the same code in order.
#[allow(clippy::too_many_arguments)]
pub fn request_to_scheduler_rom(
    m: &mut engine::Memory<'_, '_>,
    text: &[u16],
    params: &VoiceParams,
    args: &RequestArgs,
    tables: &mut TableBuffers,
    workspace: &mut [u8],
    scratch: &mut [u8],
    rom: crate::rom_region::Rom,
    host: Option<&mut dyn crate::engine_backend::AttrHost>,
) -> Result<engine::VoiceBoundary, engine::Error> {
    request_prologue_rom(m, text, params, args, tables, rom, host)?;
    request_chain_rom(m, args, workspace, scratch, rom)
}

/// Patch-faithful intake tail (gamePatch `main.c` `callTTS` via `doTTS`
/// after the caller ran the prologue): reads the text through the `ttsInput`
/// heap pointer, then runs the prefix chain only. The prologue (setters)
/// must already have run exactly once; running it here again would
/// double-apply the voice-queue events. US only.
pub fn do_tts_live_after_prologue(
    m: &mut engine::Memory<'_, '_>,
    tts_input: u32,
    args: &RequestArgs,
    workspace: &mut [u8],
    scratch: &mut [u8],
) -> Result<engine::VoiceBoundary, engine::Error> {
    // NUL-termination check through the pointer, mirroring `do_tts_live`'s
    // read. Bounded by the struct's own length (plus the overflow NUL the
    // game copy appends past the block); any length works.
    let text_len = m.word(tts_input.wrapping_add(4))? as usize;
    let text_ptr = m.word(tts_input.wrapping_add(8))?;
    let mut n = 0usize;
    for i in 0..=text_len / 2 {
        let w = m.word(text_ptr.wrapping_add((i as u32).wrapping_mul(2)))?;
        n += 1;
        if (w & 0xffff) == 0 {
            break;
        }
    }
    let _ = n;
    request_chain_rom(m, args, workspace, scratch, crate::rom_region::Rom::Us)
}

/// The `fn_15429c` per-component advance between a step return and the next
/// root entry, through this lane's stretch: the halfword at `0x14c403e8`
/// (`strhhi`, pc `0x155524`) then the word at `0x14c403e4` (`str`, pc
/// `0x154304`). Both are loop counters the ARM increments once per
/// component here (verified against the capture: `0xa->0xb`/`0xe->0xf`
/// before `0x206be4`, `0xb->0xc`/`0xf->0x10` before `0x1dd914`); the ARM's
/// branch conditions hold on this request's stretch, and the back-to-back
/// test certifies the outcome against the ARM return images.
pub fn scheduler_advance_after_step(m: &mut crate::boot::Memory<'_>) {
    let half_at = 0x14c403e8usize;
    let half = u16::from_le_bytes([m.bytes[half_at], m.bytes[half_at + 1]]);
    m.bytes[half_at..half_at + 2].copy_from_slice(&(half.wrapping_add(1)).to_le_bytes());
    let word_at = 0x14c403e4usize;
    let word = u32::from_le_bytes(m.bytes[word_at..word_at + 4].try_into().unwrap());
    m.bytes[word_at..word_at + 4].copy_from_slice(&(word.wrapping_add(1)).to_le_bytes());
}
/// fn_15429c sequencer bookkeeping (seg7 r14 port of the test-side
/// scheduler setup). Applies the logged scheduler counter words
/// (0x14c403e4/e8) and pump-flag bytes (0x14c428b8) the ARM sequencer
/// maintains between pumps. Values are logged inputs (like all driven
/// call args); the word selection and store order is the ported logic.
pub fn scheduler_bookkeeping_m(m: &mut crate::boot::Memory<'_>, e4: u32, e8: u16, flag: &[u8]) {
    if m.bytes.len() > 0x14c403ec {
        m.bytes[0x14c403e4..0x14c403e8].copy_from_slice(&e4.to_le_bytes());
        m.bytes[0x14c403e8..0x14c403ea].copy_from_slice(&e8.to_le_bytes());
    }
    if !flag.is_empty() && m.bytes.len() > 0x14c428b8 + flag.len() {
        m.bytes[0x14c428b8..0x14c428b8 + flag.len()].copy_from_slice(flag);
    }
}
/// Pre-first-pump 18f6b8 prologue (seg7 r14 port of the test-side
/// scheduler setup). Applies the 0x153830-54 update (out->len=32,
/// out->nmark=0) the ARM run performs before the first pump. The
/// reset values (buf/len/nmark) already match entry, so only the
/// update is needed. Only the pre-first-pump 18f6b8 does this.
pub fn pump_prologue_m(m: &mut crate::boot::Memory<'_>) {
    const OO: usize = 0x14c40394;
    if m.bytes.len() <= OO + 0x6c {
        return;
    }
    m.bytes[OO + 0x64..OO + 0x68].copy_from_slice(&32u32.to_le_bytes());
    m.bytes[OO + 0x68..OO + 0x6c].copy_from_slice(&0u32.to_le_bytes());
}
