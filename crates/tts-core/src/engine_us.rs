//! The US English engine end to end: request intake (seg2) -> scheduler segments -> render (back).
//!
//! See prompts/rust_us_engine_r1.md. This module composes one host that layers the
//! certified seg hosts (seg2 -> seg7, back) over `engine_live` by forwarding.
//! `engine_live::Host` is private to `engine_live.rs` (wire-a owns that file), so
//! this lane routes through the public live entry points, never by editing
//! `engine_live.rs` or any `engine_live_seg*` file.
//!
//! Scheduler order on speech_1 (ticks from wire-a-r21c-entries.json):
//! 1d5ba8 (wire-a) -> 200e64 (wire-a) -> 1e4b7c (wire-a) -> 206be4 (seg2) ->
//! 1dd914 (seg2) -> 1f98c8 (seg3) -> 1f1440 (seg3) -> 1f3248 (seg3) ->
//! 1df780 (seg3) -> 1e7908 (seg4) -> 1f3ea0 (seg4) -> 1f51f8 (seg4) ->
//! 2104b0 (seg5) -> 227d64 x3 (seg5) -> 22a0a8 (seg6) -> 22d6dc (seg7) ->
//! 1e3b14 (seg7) -> 231ba0 (seg7) -> 1530d0 (seg7) -> 1a18c4 render (back).
//!
//! Entry disagreements (round 1 audit): none on the speech_1 path. Every seg
//! root rides the same certified port the live dispatcher already binds
//! (206be4 via component_live_c4, 1dd914 via component_live_c, 1e7908/1f3ea0
//! via component_live_b LiveHost, 227d64/2104b0 via component_live_a/c,
//! 22a0a8 via component_live_a2, 22d6dc/231ba0 via the live arms, 1e3b14 via
//! engine_components, 1530d0 via emit_output). Where a seg file shadows a live
//! arm with an identical port, the certified seg arm is taken; the live arm is
//! the fallback. Wire-a's three roots (1d5ba8/200e64/1e4b7c) are live-only and
//! currently open (the live gate engine_live_pcm is red from wire-a's in-flight
//! edit).
//!
//! Sequencer glue: fn_15429c bumps two counters between a step return and the
//! next root entry (halfword at synth+0xd8 via strhhi at pc 0x155524, word at
//! synth+0xd4 via str at pc 0x154304; on this request synth is 0x14c40310 so
//! the stores land at 0x14c403e8/0x14c403e4). [`scheduler_advance`] ports that
//! as real scheduler code driven by the loop below, not a test replica. When
//! running through the real scheduler ([`speak`] uses `process_text_m`), the
//! certified frontend port already publishes pending/stage; `scheduler_advance`
//! is the explicit form used by manual back-to-back runs and documents the PCs.
//!
//! Boot state: `speak` starts from the post-voice-load job-loop WAITING image,
//! i.e. seg2's 0x191E40-entry template (reqZ-191e4b.wire: pristine entry bytes
//! after boot_port_3b::fn_2252b0, before the 0x191E40 request). The template
//! already holds the booted engine (voice wrapper 0x8668E00, engine 0x14c3fc08,
//! numeric tables 0xa21fb0/0x8cbfe0, text buffer 0xB27DAA mapped). What boot-a
//! must deliver for a Rust-booted start is exactly that stop state: job loop
//! WAITING with the voice loaded, the text buffer mapped, and the RequestArgs
//! below (engine/magic/request/sp/voice/s16/s17/tick) reading live from the
//! loop. The request intake (six voice setters + parameter_reset + the
//! 0x191e4b prefix chain) then runs from text+params, so no job bytes leak
//! from the template.
//!
//! Region: rom_region only (US). No globals or statics, no unsafe. C locals
//! live in Rust locals, never at fixed guest addresses.

use crate::{boot, engine, engine_backend, engine_live, engine_live_seg2};
use crate::reentrant::HooksM;

pub use crate::engine::Error;
pub use crate::engine_live_seg2::RequestArgs as BootArgs;
pub use crate::engine_live_seg2::TableBuffers;
pub use crate::engine_live_seg2::{TEXT_ADDR, VOICE_WRAPPER};

/// Voice params for the US engine. The six voice bytes match the bot job
/// (each 0-100; intonation 0-4 on US); `language` must be 1 (US English).
#[derive(Clone, Copy, Debug)]
pub struct VoiceParams {
    pub pitch: u8,
    pub speed: u8,
    pub quality: u8,
    pub tone: u8,
    pub accent: u8,
    pub intonation: u8,
    pub language: u8,
}

impl VoiceParams {
    /// speech_1 defaults: 50s with intonation 1, language 1 (US).
    pub fn speech_1() -> Self {
        Self { pitch: 50, speed: 50, quality: 50, tone: 50, accent: 50, intonation: 1, language: 1 }
    }
    fn to_seg2(self) -> engine_live_seg2::VoiceParams {
        engine_live_seg2::VoiceParams {
            pitch: self.pitch,
            speed: self.speed,
            quality: self.quality,
            tone: self.tone,
            accent: self.accent,
            intonation: self.intonation,
        }
    }
}

/// Caller-owned engine state. Every buffer is caller-allocated: `arena` is the
/// flat base-0 guest arena (at least as large as the highest mapped byte),
/// `snap` is the reference snapshot for runtime services, `workspace`/`scratch`
/// back the prefix chain (each at least the arena size). `platform`/`runtime`
/// are the caller's heap and entry-local machine services. No allocation inside
/// `speak`; no globals.
/// Caller-owned finish-pump scratch (~930 KB: input+output moved off the
/// `speak` stack, 1:1 with the ARM's per-speak `Speech` + backend
/// workspace; no_std, alloc-free, caller owns the storage via `UsEngine`).
pub struct FinishBuffers {
    pub input: [i16; 65536],
    pub output: [i32; 200000],
}

impl FinishBuffers {
    pub fn new() -> Self {
        Self {
            input: [0; 65536],
            output: [0; 200000],
        }
    }
}

pub struct UsEngine<'a> {
    pub boot: BootArgs,
    pub arena: &'a mut [u8],
    pub snap: &'a [u8],
    pub workspace: &'a mut [u8],
    pub scratch: &'a mut [u8],
    pub tables: TableBuffers,
    pub platform: &'a mut dyn engine::HeapPlatform,
    pub runtime: &'a mut dyn engine_live::RuntimeServicesM,
    pub finish: FinishBuffers,
}

impl<'a> UsEngine<'a> {
    pub fn new(
        boot: BootArgs,
        arena: &'a mut [u8],
        snap: &'a [u8],
        workspace: &'a mut [u8],
        scratch: &'a mut [u8],
        platform: &'a mut dyn engine::HeapPlatform,
        runtime: &'a mut dyn engine_live::RuntimeServicesM,
    ) -> Self {
        Self { boot, arena, snap, workspace, scratch, tables: TableBuffers::new(), platform, runtime, finish: FinishBuffers::new() }
    }
}

/// Scheduler root order on speech_1 (certified boundaries; wire-a's three first
/// are currently open).
pub const SCHEDULER_ORDER: &[u32] = &[
    0x1d5ba8, 0x200e64, 0x1e4b7c, 0x206be4, 0x1dd914, 0x1f98c8, 0x1f1440, 0x1f3248,
    0x1df780, 0x1e7908, 0x1f3ea0, 0x1f51f8, 0x2104b0, 0x227d64, 0x227d64, 0x227d64,
    0x22a0a8, 0x22d6dc, 0x1e3b14, 0x231ba0, 0x1530d0,
];

/// The fn_15429c per-component advance between a step return and the next root
/// entry: pending word at synth+0xd4 (ARM str at pc 0x154304) then stage
/// halfword at synth+0xd8 (ARM strhhi at pc 0x155524), each +1 wrapping. Real
/// scheduler code: driven by the loop (manual back-to-back runs call this
/// between roots; the live `process_text_m` path publishes the same counters
/// through its frontend state).
pub fn scheduler_advance(m: &mut boot::Memory<'_>, synth: u32) {
    let pending = m.word(synth.wrapping_add(0xd4));
    m.put_word(synth.wrapping_add(0xd4), pending.wrapping_add(1));
    let at = synth.wrapping_add(0xd8);
    let off = (at - m.base) as usize;
    let half = u16::from_le_bytes([m.bytes[off], m.bytes[off + 1]]);
    m.bytes[off..off + 2].copy_from_slice(&half.wrapping_add(1).to_le_bytes());
}

/// The fn_15429c per-component 5th (stack) argument, computed from live
/// scheduler state — never copied from a capture.
///
/// C derivation (`src/fn_151914.c`): line 1440 in the `fn_15429c` body
/// passes `(u8 *)p->out.extra + p->out.anStage` as the `active` pointer to
/// `ac->vt->process`; each component's `process` forwarder hands that same
/// byte address to its root as the trailing `u8 *` (which the root sets to
/// 1): `fn_1d5ba8` (`src/fn_1d49b4.c:580`, `u8 *pending`), `fn_200e64`
/// (`src/fn_200e64.c:63`, `u8 *active`), `fn_1e4b7c`
/// (`src/fn_1e4b7c.c:140`, `u8 *processed`), `fn_206be4`
/// (`src/fn_206be4.c:41`, `u8 *changed`), and the same pattern for every
/// later root. Layout: `TtsOut.extra` is at `out+0x58` and `TtsOut.anStage`
/// (u16) at `out+0x54` (`src/fn_151914.c:170-172`); `TtsSynth.out` starts at
/// `synth+0x84`, so `extra` lives at `synth+0xdc` and `anStage` at
/// `synth+0xd8`. The array itself is one flag byte per component, allocated
/// in `fn_15668c` (`src/fn_151914.c:2193`:
/// `out->extra = fn_1963b8(ctx->heap, p->cs->count, 1)`), so byte indexing
/// by the stage is exact. `0x1cca64` steps are not roots: they take a plain
/// `u32 flag` (`src/fn_1cca64.c:14`), always 1 on this path.
///
/// Verified against the live state at every root entry on speech_1
/// (`extra` base `0x14c428a4`, `ncomp` 26): `1d5ba8:stage 8->ac`,
/// `200e64:9->ad`, `1e4b7c:a->ae`, `206be4:b->af`, `1dd914:c->b0`,
/// `1f98c8:d->b1`, `1f1440:e->b2`, `1f3248:f->b3`, `1df780:10->b4`,
/// `1e7908:11->b5`, `1f3ea0:12->b6`, `1f51f8:13->b7`, `2104b0:14->b8`,
/// `227d64:15->b9` (x3, same stage — the component re-runs without an
/// advance), `22a0a8:16->ba`, `22d6dc:17->bb`, `1e3b14:18->bc`,
/// `231ba0:19->bd`. `1530d0`/`1a18c4` are not component roots (synth/pipe
/// args, stack `0x1551c4`/`0x14b58fb4`).
///
/// `synth` is the live `TtsSynth*`: `speak` takes it from the scheduler
/// boundary args (`boundary.args[0]`), never a constant. No guest
/// addresses are hardcoded here — only the `TtsOut` field offsets
/// (`extra` at `synth+0xdc`, `anStage` at `synth+0xd8`), the same class
/// of constant as `scheduler_advance`'s `0xd4`/`0xd8`.
pub fn scheduler_stack_arg(m: &boot::Memory<'_>, synth: u32) -> u32 {
    let extra = m.word(synth.wrapping_add(0xdc));
    let at = synth.wrapping_add(0xd8);
    let off = (at - m.base) as usize;
    let stage = u16::from_le_bytes([m.bytes[off], m.bytes[off + 1]]) as u32;
    extra.wrapping_add(stage)
}

/// Runtime wrapper that threads pump PCM to speak's caller-owned `out`.
/// Every entry-local service forwards to the caller's runtime unchanged;
/// the pump's post-render emit appends to `out`, capacity-checked, never
/// truncated. `written` is the sample count speak returns.
/// Sleep platform for the end-of-speak finish pump: the finish render
/// never sleeps on the ARM (no 0x166054 on the finish path), so this is
/// an infallible no-op like the seg7 pump's own sleeper.
struct FinishNap;
impl engine_live::SleepPlatform for FinishNap {
    fn sleep_ns(&mut self, _: u64) -> Result<(), engine::Error> {
        Ok(())
    }
}
struct ForwardSink<'o, 'r> {
    outer: &'r mut dyn engine_live::RuntimeServicesM,
    out: &'o mut [i16],
    written: usize,
    /// The first emit failure, if the pump outgrew `out`. The scheduler
    /// hook interface only carries a u32 status, so speak reads this back
    /// to report Capacity instead of a misleading open boundary.
    emit_err: Option<engine::Error>,
}

impl engine_live::RuntimeServicesM for ForwardSink<'_, '_> {
    fn invoke_m(
        &mut self,
        m: &mut boot::Memory<'_>,
        entry: u32,
        args: &[u32],
    ) -> Result<u32, engine::Error> {
        self.outer.invoke_m(m, entry, args)
    }
    fn emit_pcm_m(&mut self, samples: &[i16]) -> Result<(), engine::Error> {
        if self.written + samples.len() > self.out.len() {
            let e = engine::Error::Capacity {
                required: self.written + samples.len(),
                available: self.out.len(),
            };
            self.emit_err.get_or_insert(e);
            return Err(e);
        }
        for (i, s) in samples.iter().enumerate() {
            self.out[self.written + i] = *s;
        }
        self.written += samples.len();
        Ok(())
    }
}

/// Host layering the seg dispatches over the live dispatcher. Every nested call
/// rides the live engine; the first entry the live engine cannot serve is
/// recorded instead of replayed.
struct UsHooks<'p, 'r> {
    platform: &'p mut dyn engine::HeapPlatform,
    runtime: &'r mut dyn engine_live::RuntimeServicesM,
    snap: &'r [u8],
    first_unbound: Option<u32>,
    /// The first error a dispatch returned, kept so a heap-exhaustion or
    /// crash fault is reported as itself rather than as its entry.
    first_error: Option<Error>,
}

impl crate::reentrant::HooksM for UsHooks<'_, '_> {
    fn call_m(&mut self, address: u32, args: &[u32], mem: &mut boot::Memory<'_>) -> u32 {
        match engine_live::call_live_with_runtime_m(
            mem,
            address,
            args,
            &mut *self.platform,
            &mut *self.runtime,
            self.snap,
        ) {
            Ok(rc) => {
                // The live dispatcher reports some open boundaries as an error
                // status (e.g. wire-a's in-flight 0x8b702007) rather than Err:
                // record the failing entry so speak() names it.
                if rc >= 0x80000000 && self.first_unbound.is_none() {
                    self.first_unbound = Some(address);
                }
                rc
            }
            Err(engine::Error::UnsupportedStage { entry }) |
            Err(engine::Error::MissingStreamAdapter { entry }) => {
                if self.first_unbound.is_none() {
                    self.first_unbound = Some(entry);
                }
                0x80002001
            }
            Err(e) => {
                if self.first_unbound.is_none() {
                    self.first_unbound = Some(address);
                }
                self.first_error.get_or_insert(e);
                0x80002001
            }
        }
    }
}

impl engine_live::SchedulerHooksM for UsHooks<'_, '_> {
    fn switch_m(
        &mut self,
        _: crate::frontend::PipelineCall,
        _: u32,
        _: &mut boot::Memory<'_>,
    ) -> crate::frontend::PipelineReply {
        // Voice-switch bodies are not on the speech_1 path; fail closed naming
        // the scheduler, exactly like the live Host.
        if self.first_unbound.is_none() {
            self.first_unbound = Some(0x15429c);
        }
        crate::frontend::PipelineReply { status: 0x80002001, value: 0, data: 0 }
    }
}

/// Dispatcher host for the rate attr-propagation loop: nested parameter
/// calls ride the live dispatcher through `UsHooks` (single-region
/// prologue memory re-wrapped as the boot memory the hooks take).
struct DispatchHost<'a, 'p, 'r> {
    hooks: &'a mut UsHooks<'p, 'r>,
}

impl crate::engine_backend::AttrHost for DispatchHost<'_, '_, '_> {
    fn call_attr(
        &mut self,
        m: &mut crate::engine::Memory<'_, '_>,
        f: u32,
        a: &[u32],
    ) -> u32 {
        if m.regions.len() != 1 {
            return 0x80002001;
        }
        let (base, bytes) = (m.regions[0].base, &mut *m.regions[0].bytes);
        let mut bm = boot::Memory { base, bytes };
        self.hooks.call_m(f, a, &mut bm)
    }
    fn attr_unbound(&self) -> Option<u32> {
        self.hooks.first_unbound
    }
}

/// Run one US request end to end: request intake (seg2 certified) from the
/// boot template plus text+params to the 0x15429c scheduler entry, then the
/// live scheduler (all roots with live-derived args). Each output pump
/// threads its nested render's PCM to `out` (16 kHz s16le mono) through
/// the merged Host; the return is the sample count.
///
/// Returns the samples written. Until every boundary is certified this
/// returns the first open boundary as `UnsupportedStage{entry}` (expect
/// seg5's 2104b0 until that lane lands): no recorded results, no replayed
/// PCM, every output byte computed live. If `out` is too small for the
/// run's samples this returns `Capacity`, never silently truncating.
/// Patch-faithful intake state between the pre-chain and the chain.
/// `tts` is the game-heap `ttsInput` struct (`tmalloc(12)`); `text` is the
/// `utfRecv` heap copy. Both are freed by [`speak_release`] in
/// callTTS/mainLoopF order once the whole request has run.
pub struct Intake {
    pub request: engine_live_seg2::RequestArgs,
    pub tts: u32,
    pub text: u32,
}

/// Dispatch one game-heap call through the live dispatcher (the same
/// binding the scheduler uses: tmalloc 0x129778, tfree 0x11da30).
/// Heap failures surface the entry instead of trapping.
fn heap_dispatch(
    hooks: &mut UsHooks<'_, '_>,
    arena: &mut [u8],
    entry: u32,
    args: &[u32],
) -> Result<u32, Error> {
    let mut bm = boot::Memory { base: 0, bytes: arena };
    let rc = hooks.call_m(entry, args, &mut bm);
    if rc >= 0x80000000 {
        return Err(Error::UnsupportedStage {
            entry: hooks.first_unbound.unwrap_or(entry),
        });
    }
    Ok(rc)
}

/// Speak phase 1a (gamePatch `mainLoopF` + `callTTS` up to `ttsFunc`):
/// `utfRecv` first (tmalloc `inLen*2`, copy `0..=inLen` with the NUL
/// overflowing 2 bytes past the block, byte for byte), then
/// `RESET_TTS(effects)`, the six setters, abandon `audioJob->audioData`
/// (wire-a r48: the ARM zeroes size/data without freeing), `setupFunc`,
/// `tts = tmalloc(12)` filled `{0, len*2, text}` with the engine request
/// linked from it. Ends at the chain-entry state.
pub fn speak_pre_chain(
    state: &mut UsEngine<'_>,
    text: &[u16],
    params: &VoiceParams,
) -> Result<Intake, Error> {
    let seg_params = params.to_seg2();
    let request = engine_live_seg2::RequestArgs {
        engine: state.boot.engine,
        magic: state.boot.magic,
        request: state.boot.request,
        request_sp: state.boot.request_sp,
        voice: state.boot.voice,
        s16: state.boot.s16,
        s17: state.boot.s17,
        tick: state.boot.tick,
    };
    let mut hooks = UsHooks {
        platform: &mut *state.platform,
        runtime: &mut *state.runtime,
        snap: &*state.snap,
        first_unbound: None,
        first_error: None,
    };
    // utfRecv: wcslen of the bot-written TEXT_ADDR, tmalloc(inLen*2),
    // copy i = 0..=inLen (NUL included, 2 bytes past the block end).
    // No length cap: the game scans to the NUL whatever the length (the
    // arena bound below is defensive and unreachable on valid input).
    let mut units = 0usize;
    {
        let arena: &[u8] = &mut *state.arena;
        let ta = engine_live_seg2::TEXT_ADDR as usize;
        loop {
            let o = ta + units * 2;
            if o + 2 > arena.len() {
                return Err(Error::Capacity { required: o + 2, available: arena.len() });
            }
            let w = u16::from_le_bytes([
                arena[o],
                arena[o + 1],
            ]);
            if w == 0 {
                break;
            }
            units += 1;
        }
    }
    let nbytes = (units as u32).wrapping_mul(2);
    let text_copy = heap_dispatch(&mut hooks, &mut *state.arena, 0x129778, &[nbytes])?;
    if text_copy == 0 {
        return Err(Error::Capacity { required: nbytes as usize, available: 0 });
    }
    {
        let arena: &mut [u8] = &mut *state.arena;
        let ta = engine_live_seg2::TEXT_ADDR as usize;
        let to = text_copy as usize;
        for i in 0..=units {
            arena[to + i * 2] = arena[ta + i * 2];
            arena[to + i * 2 + 1] = arena[ta + i * 2 + 1];
        }
    }
    // RESET_TTS(effects): the Nintendo backend behind the voice wrapper
    // (ttsGlob->mainTtsClass->effects), before the setters.
    {
        let arena: &mut [u8] = &mut *state.arena;
        let mut region = [engine::Region { base: 0, bytes: arena }];
        let mut mem = engine::Memory::new(&mut region)?;
        mem.read(
            crate::rom_region::addr(crate::rom_region::Rom::Us, 0xa21fb0),
            &mut state.tables.constants,
        )?;
        mem.read(
            crate::rom_region::addr(crate::rom_region::Rom::Us, 0x8cbfe0),
            &mut state.tables.cosine,
        )?;
        let numeric =
            crate::tables::NumericTables::from_bytes(&state.tables.constants, &state.tables.cosine)
                .ok_or(Error::Capacity {
                    required: crate::tables::NumericTables::CONSTANT_BYTES,
                    available: state.tables.constants.len(),
                })?;
        let o = mem.word(request.voice + 8)?;
        let effects = mem.word(o + 0x61b8)?;
        engine_backend::reset(&mut mem, effects, &numeric)?;
    }
    // CallTTS body in game order (main.c): six setters, abandon(audioData),
    // then setupFunc 0x3914e8. Split so the previous render's audioData
    // frees between the setters and setup, byte for byte with the game.
    // Round 10: the rate setter's attr-propagation loop (fn_174b38 body)
    // rides the live dispatcher 1:1 through DispatchHost.
    {
        let arena: &mut [u8] = &mut *state.arena;
        let mut region = [engine::Region { base: 0, bytes: arena }];
        let mut mem = engine::Memory::new(&mut region)?;
        let mut host = DispatchHost { hooks: &mut hooks };
        engine_live_seg2::request_prologue_setters_rom(
            &mut mem,
            text,
            &seg_params,
            &request,
            &mut state.tables,
            crate::rom_region::Rom::Us,
            Some(&mut host),
        )?;
    }
    // wire-a r48: the ARM setup abandons the previous render's audio
    // buffer: it zeroes audioSize/audioData WITHOUT freeing (unicorn
    // march: audioData advances ~283KB every render, never freed or
    // reused; the 6 tfree/render always hit the same capture-chain
    // addresses). Zero both words and leak like the ARM.
    // (The capture state is double-indirect: slot 0xc72688 -> static
    // 0xc72864 -> word0 = job; the static link is baked in the snapshot.)
    // Game position: after the setters, before setupFunc.
    {
        let arena: &mut [u8] = &mut *state.arena;
        let at = engine_live_seg2::JOB_ADDR as usize + 12;
        arena[at..at + 8].copy_from_slice(&0u64.to_le_bytes());
    }
    // setupFunc (0x3914e8 parameter_reset) after the audioData abandon.
    {
        let arena: &mut [u8] = &mut *state.arena;
        let mut region = [engine::Region { base: 0, bytes: arena }];
        let mut mem = engine::Memory::new(&mut region)?;
        engine_live_seg2::request_setup_rom(
            &mut mem,
            &request,
            crate::rom_region::Rom::Us,
        )?;
    }
    // tts = tmalloc(12) {0, len*2, text}; engine request linked from it.
    let tts = heap_dispatch(&mut hooks, &mut *state.arena, 0x129778, &[12])?;
    if tts == 0 {
        return Err(Error::Capacity { required: 12, available: 0 });
    }
    {
        let arena: &mut [u8] = &mut *state.arena;
        let mut bm = boot::Memory { base: 0, bytes: arena };
        bm.put_word(tts, 0);
        bm.put_word(tts.wrapping_add(4), nbytes);
        bm.put_word(tts.wrapping_add(8), text_copy);
        bm.put_word(request.request.wrapping_add(4), nbytes);
        bm.put_word(request.request.wrapping_add(8), text_copy);
    }
    drop(hooks);
    Ok(Intake { request, tts, text: text_copy })
}

/// Free one guest heap block through the live tfree binding (0x11da30).
pub fn guest_free(state: &mut UsEngine<'_>, block: u32) -> Result<(), Error> {
    let mut hooks = UsHooks {
        platform: &mut *state.platform,
        runtime: &mut *state.runtime,
        snap: &*state.snap,
        first_unbound: None,
        first_error: None,
    };
    heap_dispatch(&mut hooks, &mut *state.arena, 0x11da30, &[block])?;
    Ok(())
}

/// Clear the voice-prepared flag (voice+0x61a0) after the first request
/// served from a fresh reqZ template.
///
/// A cold boot's voice is inactive, so its first request goes through
/// fn_35a958's start path, which leaves the flag clear, and the second one
/// still resets the voice through fn_35a9b0 (Speech.total, finish pad)
/// before the flag sticks. The template is a cold boot plus the warm-up
/// render, so its first request is already that second, resetting one.
/// Clearing the flag afterwards gives the next request the reset a cold
/// boot's second request gets. fn_35a9b0 rewrites every other field it
/// touches before anything reads it.
pub fn unprepare_voice(state: &mut UsEngine<'_>) {
    let arena: &mut [u8] = &mut *state.arena;
    let w = state.boot.voice as usize;
    let obj = u32::from_le_bytes(arena[w + 8..w + 12].try_into().unwrap()) as usize;
    if let Some(flag) = arena.get_mut(obj + 0x61a0) {
        *flag = 0;
    }
}

/// Speak phase 1b (the `ttsFunc` chain up to the scheduler). Returns the
/// 0x15429c scheduler boundary; the intake blocks stay live until
/// [`speak_release`], as on the ARM at that boundary.
pub fn speak_post_chain(
    state: &mut UsEngine<'_>,
    intake: &Intake,
) -> Result<engine::VoiceBoundary, Error> {
    let boundary = {
        let arena: &mut [u8] = &mut *state.arena;
        let workspace: &mut [u8] = &mut *state.workspace;
        let scratch: &mut [u8] = &mut *state.scratch;
        let mut region = [engine::Region { base: 0, bytes: arena }];
        let mut mem = engine::Memory::new(&mut region)?;
        engine_live_seg2::request_chain_rom(
            &mut mem,
            &intake.request,
            workspace,
            scratch,
            crate::rom_region::Rom::Us,
        )?
    };
    Ok(boundary)
}

/// `tfree(tts)` (callTTS) then `tfree(text)` (mainLoopF) after ttsFunc
/// returns: on the ARM both follow the capture buffer's last realloc.
pub fn speak_release(state: &mut UsEngine<'_>, intake: &Intake) -> Result<(), Error> {
    let mut hooks = UsHooks {
        platform: &mut *state.platform,
        runtime: &mut *state.runtime,
        snap: &*state.snap,
        first_unbound: None,
        first_error: None,
    };
    heap_dispatch(&mut hooks, &mut *state.arena, 0x11da30, &[intake.tts])?;
    heap_dispatch(&mut hooks, &mut *state.arena, 0x11da30, &[intake.text])?;
    Ok(())
}

/// Request-mutex host for the fn_191e40 tail restore: the same no-op
/// binding the request prefix uses (an uncontended single-threaded run
/// never takes the lock slots; a contended heap is as unbound here as
/// it is on the request entry path).
struct TailMutex;
impl engine::MutexPlatform for TailMutex {
    fn acquire_mutex(&mut self, _: u32) -> u32 {
        0x81402001
    }
    fn release_mutex(&mut self, _: u32) -> u32 {
        0x81402001
    }
}
/// fn_15961c tail restore through the live image: the segment pipeline
/// state (seg+0x30) back to `previous` (skipped when zero, 1:1 with the
/// ARM guard). `seg` is the voice handle the 0x15961c boundary carries.
fn synthesis_tail_restore(state: &mut UsEngine<'_>, seg: u32, previous: u8) -> Result<(), Error> {
    let mut host = TailMutex;
    let arena: &mut [u8] = &mut *state.arena;
    let mut region = [engine::Region { base: 0, bytes: arena }];
    let mut mem = engine::Memory::new(&mut region)?;
    engine::synthesis_restore_rom(&mut mem, seg, previous as u32, &mut host)
}
/// fn_191e40 tail restore through the live image: the engine+40 request
/// state back to `previous` (skipped when zero, 1:1 with the ARM guard).
fn request_tail_restore(state: &mut UsEngine<'_>, previous: u8) -> Result<(), Error> {
    let mut host = TailMutex;
    let engine_addr = state.boot.engine;
    let arena: &mut [u8] = &mut *state.arena;
    let mut region = [engine::Region { base: 0, bytes: arena }];
    let mut mem = engine::Memory::new(&mut region)?;
    engine::request_restore_rom(&mut mem, engine_addr, previous as u32, &mut host)
}

/// Scheduler setup prologue (r16 part 1): the ARM's pre-root driver calls
/// between the 0x15429c scheduler entry and the first component root
/// (1d5ba8), 1:1 with the ARM order (wire harness from mybad-15429c.wire:
/// two 0x22b518 calls, lr 0x154ef8, the same Process site the roots ride).
/// The 15017c/878/c0c stream setup is NESTED inside the driver on ARM (RET
/// order proves it), so it must ride the driver's genuine port, never as
/// siblings (standalone 15017c fails on live while ARM returns 0).
/// The 5th arg is extra+stage, computed live via scheduler_stack_arg
/// (reproduces the ARM's 0x14c428a5 on the my-bad sched-entry state).
/// Every call rides the live dispatcher so nested heap/compaction calls
/// run through the genuine ports. Returns the first failing entry;
/// never seeds state. Remaining: the driver's nested 150xxx effects and
/// the post-driver top-level stream calls (150878/150c0c/15017c with lrs
/// 0x1d312c/0x1d443c/0x205a30/...) once their parents are identified.
fn scheduler_setup(
    hooks: &mut UsHooks<'_, '_>,
    bm: &mut boot::Memory<'_>,
    synth: u32,
) -> Result<(), Error> {
    // 5-arg 22b518 arrays need an owned backing (call_m takes a slice).
    // NOTE (r16 triage): the 15017c/878/c0c calls are NESTED inside the
    // 22b518 driver on ARM (RET order proves it), so wiring them as
    // siblings double-drives and fails (standalone 15017c -> error while
    // ARM returns 0). The driver must run through its genuine port so its
    // nested calls ride the dispatcher in its frame. 150xxx siblings are
    // NOT wired until the driver's nesting is confirmed per call.
    // The 5th arg is extra+stage, computed live (never copied): on the
    // my-bad sched-entry state this reproduces the ARM's 0x14c428a5.
    let fifth = scheduler_stack_arg(bm, synth);
    let drv: [u32; 5] = [0x14c80218, 0x10d, 0x14c42964, 0xb2c2, fifth];
    let seq: &[(u32, &[u32])] = &[(0x22b518, &drv), (0x22b518, &drv)];
    for (entry, args) in seq {
        let rc = hooks.call_m(*entry, args, bm);
        if rc >= 0x80000000 {
            return Err(Error::UnsupportedStage {
                entry: hooks.first_unbound.unwrap_or(*entry),
            });
        }
    }
    Ok(())
}

/// Speak `text` through the live engine, gamePatch `mainLoopF` + `callTTS`
/// order (utfRecv first, then RESET, six setters, abandon(audioData), setup,
/// tmalloc(12) ttsInput, chain, tfrees), and render to `out`.
///
/// Caller-side preprocessing (NOT in the engine): the bot's `sendText`
/// (ttsmodachi `api/tts.py`) rewrites `<bleep>` to
/// `\x1b\\mrk=6\\`, `</bleep>` to `\x1b\\mrk=7\\`, `<echo>` to
/// `\x1b\\mrk=4\\` and `</echo>` to `\x1b\\mrk=5\\`, appends the
/// NUL, and writes the whole UTF-16LE text plus NUL to `textDataLoc`
/// (`TEXT_ADDR`) with no length cap before setting the job params and
/// status 5. `speak` takes that already-preprocessed text as-is and
/// performs the same unbounded write; marker interpretation lives
/// downstream in the engine, never here.
pub fn speak(
    state: &mut UsEngine<'_>,
    text: &[u16],
    params: &VoiceParams,
    out: &mut [i16],
) -> Result<usize, Error> {
    if params.language != 1 {
        return Err(Error::InvalidVoice);
    }
    if out.is_empty() {
        return Err(Error::Capacity { required: 1, available: 0 });
    }
    // Round 29: bot text delivery BEFORE intake (tts.py sendText writes
    // UTF-16LE to job+0x3499D = TEXT_ADDR). No app-copy to request+8 target
    // (ARM captures show it stays stale warmup; scheduler text source is a
    // separate blocker). Request+4/+8 untouched.
    // seg7 r13: shared app-boundary status (gamePatch mainLoopF). The bot
    // sets TEXT_READY before calling; mainLoopF answers PROCESSING here.
    {
        let arena: &mut [u8] = &mut *state.arena;
        let mut bm = boot::Memory { base: 0, bytes: arena };
        crate::engine_live_seg2::job_mark_processing(&mut bm, crate::engine_live_seg2::JOB_ADDR);
    }
    // sendText emulation (gamePatch app side): the whole caller text plus
    // NUL, no length cap. The write is arena-bounded only (the live arena
    // is finite; the game buffer is not the limit).
    {
        let arena: &mut [u8] = &mut *state.arena;
        let text_addr = engine_live_seg2::TEXT_ADDR as usize;
        let mut at = text_addr;
        for unit in text {
            if at + 2 > arena.len() {
                return Err(Error::Capacity { required: at + 2, available: arena.len() });
            }
            arena[at..at + 2].copy_from_slice(&unit.to_le_bytes());
            at += 2;
        }
        if at + 2 > arena.len() {
            return Err(Error::Capacity { required: at + 2, available: arena.len() });
        }
        arena[at..at + 2].copy_from_slice(&0u16.to_le_bytes());
    }
    // Phase 1: patch-faithful intake (gamePatch main.c: utfRecv first,
    // then callTTS RESET -> six setters -> abandon(audioData) -> setupFunc
    // 0x3914e8 -> tmalloc(12) ttsInput {0, len*2, text} -> ttsFunc ->
    // tfree(tts), then mainLoopF tfree(text)). utfRecv copies TEXT_ADDR to
    // the guest heap and points the ttsInput struct (args.request) at the
    // fresh copy: the ARM reads the text exclusively through that pointer
    // (0x195ae8).
    // seg7 r13: shared app-boundary status (gamePatch mainLoopF). Kept
    // verbatim: mainLoopF answers PROCESSING here (no-op after the first
    // mark above).
    {
        let arena: &mut [u8] = &mut *state.arena;
        let mut bm = boot::Memory { base: 0, bytes: arena };
        crate::engine_live_seg2::job_mark_processing(&mut bm, crate::engine_live_seg2::JOB_ADDR);
    }
    // wire-a r48: the voice-prepared flag at setup entry (voice+0x61a0).
    // fn_35a958 early-returns (and fn_35a9b0 never runs) when it is set,
    // so the per-request total reset below must also be skipped then (1:1).
    let setup_entry_prepared = {
        let arena: &[u8] = &*state.arena;
        let w = state.boot.voice as usize;
        let obj = u32::from_le_bytes(arena[w + 8..w + 12].try_into().unwrap());
        arena.get(obj as usize + 0x61a0).copied().unwrap_or(0) != 0
    };
    let intake = speak_pre_chain(state, text, params)?;
    // ttsFunc runs the whole request (chain, scheduler, finish, drain);
    // callTTS then frees the ttsInput and mainLoopF the text copy, on every
    // return: the ARM's capture reallocs all precede both frees
    let result = speak_intake(state, &intake, setup_entry_prepared, out);
    let released = speak_release(state, &intake);
    let samples = result?;
    released?;
    Ok(samples)
}

/// Everything ttsFunc runs for one intake, up to the served capture.
fn speak_intake(
    state: &mut UsEngine<'_>,
    intake: &Intake,
    setup_entry_prepared: bool,
    out: &mut [i16],
) -> Result<usize, Error> {
    // fn_191e40 locks `previous` (the engine+40 state at enter) in its
    // frame across the voice/scheduler run and restores it in the tail.
    // Capture it before the post chain enters (1 -> 3). The fn_15961c
    // tail is the same shape for the segment pipeline state (seg+0x30):
    // seg is the voice handle the 0x15961c boundary will carry (h0).
    let previous = *state
        .arena
        .get(state.boot.engine as usize + 40)
        .ok_or(Error::InvalidSnapshot)?;
    let seg = u32::from_le_bytes(
        state
            .arena
            .get(state.boot.engine as usize + 0x2c..state.boot.engine as usize + 0x30)
            .ok_or(Error::InvalidSnapshot)?
            .try_into()
            .map_err(|_| Error::InvalidSnapshot)?,
    );
    let previous_pipe = *state
        .arena
        .get(seg as usize + 0x30)
        .ok_or(Error::InvalidSnapshot)?;
    // fn_191e40 + fn_15961c tails run even when the intake chain fails:
    // the chain enters both states, so a failure exit restores them, the
    // same pairing the scheduler/drain failure paths below keep (1:1).
    // Without it the request state stays 3 and the next speak's enter
    // refuses with 0x80000011 (session wedge).
    let boundary = match speak_post_chain(state, intake) {
        Ok(b) => b,
        Err(e) => {
            request_tail_restore(state, previous)?;
            synthesis_tail_restore(state, seg, previous_pipe)?;
            return Err(e);
        }
    };
    if boundary.entry != 0x15429c {
        return Err(Error::UnsupportedStage { entry: boundary.entry });
    }
    // Per-request Speech.total reset (src/fn_35a9b0.c `p->f80 = 0`): the
    // ARM zeroes it in setup before the sched run, every request (warm
    // 193 after c3 restarts from 0 and accumulates to 2128 on the ARM,
    // never carrying c3's 40956). The snapshot already holds 0 for the
    // first request, but warm requests must reset explicitly. Sched
    // renders then accumulate into it 1:1 (`p->total +=
    // (first+second)*2`), so the finish pad gate reads the run's own
    // total. It must not be re-zeroed after sched (that wipes the
    // accumulation and misfires the pad on long requests).
    {
        let arena: &mut [u8] = &mut *state.arena;
        let mut bm = boot::Memory {
            base: 0,
            bytes: arena,
        };
        let user = bm.word(boundary.args[0].wrapping_add(0x28));
        if user != 0 && !setup_entry_prepared {
            let pspeech = bm.word(user.wrapping_add(0x184));
            if pspeech != 0 {
                bm.put_word(pspeech.wrapping_add(0x80), 0);
            }
        }
    }
    // Scheduler setup prologue (lane us r16 part 1): the ARM's pre-root
    // 22b518 driver pair between 0x15429c entry and the first root.
    // Fail closed on the first error.
    {
        let arena: &mut [u8] = &mut *state.arena;
        let mut bm = boot::Memory { base: 0, bytes: arena };
        let mut hooks_tmp = UsHooks {
            platform: &mut *state.platform,
            runtime: &mut *state.runtime,
            snap: &*state.snap,
            first_unbound: None,
            first_error: None,
        };
        // Same drain + tail pairing on a setup failure (1:1): the setup
        // driver runs inside the scheduler invocation, whose C caller
        // (fn_15b404) drains unconditionally before returning.
        if let Err(e) = scheduler_setup(&mut hooks_tmp, &mut bm, boundary.args[0]) {
            drop(hooks_tmp);
            drop(bm);
            {
                let mut drain_sink = ForwardSink {
                    outer: &mut *state.runtime,
                    out,
                    written: 0,
                    emit_err: None,
                };
                let arena: &mut [u8] = &mut *state.arena;
                let mut region = [engine::Region { base: 0, bytes: arena }];
                if let Ok(mut mem) = engine::Memory::new(&mut region) {
                    let _ = engine_live::synthesis_drain_m(
                        &mut mem,
                        boundary.args[0],
                        &mut *state.workspace,
                        &mut *state.platform,
                        &mut drain_sink,
                        &state.snap,
                    );
                }
            }
            request_tail_restore(state, previous)?;
            synthesis_tail_restore(state, seg, previous_pipe)?;
            return Err(e);
        }
    }
    // Phase 2: the live scheduler with live-derived args. Activity storage is
    // a disjoint caller-owned (stack) buffer; the scheduler scratch is the
    // live 0x1e00 slot. Pump PCM threads out through the sink below: each
    // 0x1530d0 pump's post-render emit appends its samples to `out`.
    let synth = boundary.args[0];
    let flush = boundary.args[1];
    let mut activity = [0u8; 256];
    let mut sink = ForwardSink { outer: &mut *state.runtime, out, written: 0, emit_err: None };
    let mut hooks = UsHooks {
        platform: &mut *state.platform,
        runtime: &mut sink,
        snap: &*state.snap,
        first_unbound: None,
        first_error: None,
    };
    let rc = {
        let arena: &mut [u8] = &mut *state.arena;
        let mut bm = boot::Memory { base: 0, bytes: arena };
        engine_live::process_text_m(&mut bm, synth, flush, &mut activity, 0x1e00, &mut hooks)
    };
    if rc >= 0x80000000 {
        // A failed emit means `out` was too small: report Capacity, never
        // a boundary and never a silent truncation.
        let first_unbound = hooks.first_unbound;
        let first_error = hooks.first_error;
        drop(hooks);
        let emit_err = sink.emit_err;
        let sched_written = sink.written;
        drop(sink);
        // 1:1 with fn_15b404.c: fn_1561b8 runs even when the scheduler
        // fails (no error gate around it). The drain closes the pcm/mk
        // streams, flushes/frees the lex queue and frees arr/extra, so a
        // failed speak leaves the heap as the ARM does. Its rc only wins
        // when the scheduler succeeded; here the scheduler rc stands.
        {
            let mut drain_sink = ForwardSink {
                outer: &mut *state.runtime,
                out: &mut out[sched_written..],
                written: 0,
                emit_err: None,
            };
            let arena: &mut [u8] = &mut *state.arena;
            let mut region = [engine::Region { base: 0, bytes: arena }];
            if let Ok(mut mem) = engine::Memory::new(&mut region) {
                let _ = engine_live::synthesis_drain_m(
                    &mut mem,
                    synth,
                    &mut *state.workspace,
                    &mut *state.platform,
                    &mut drain_sink,
                    &state.snap,
                );
            }
        }
        // fn_191e40 + fn_15961c tails run even when the voice run fails:
        // restore both states before classifying (1:1, incl. error path).
        request_tail_restore(state, previous)?;
        synthesis_tail_restore(state, seg, previous_pipe)?;
        if let Some(e) = emit_err {
            return Err(e);
        }
        if let Some(e @ (Error::HeapExhausted { .. } | Error::CodeOverwritten { .. })) = first_error {
            return Err(e);
        }
        return Err(Error::UnsupportedStage {
            entry: first_unbound.unwrap_or(0x15429c),
        });
    }
    drop(hooks);
    let sched_written = sink.written;
    drop(sink);
    // The app's event-2 finish pump (fn_183398, 1:1 with src/fn_183398.c):
    // runs on every speak with no outer gate. The ARM's 0x163b24 post
    // always dispatches event 2 to fn_183398 (src/fn_163a98.c:79,
    // unconditional in the `case 2` arm); the only early-outs live inside
    // fn_183398 itself (state<2 selects the second capacity,
    // src/fn_183398.c `if (p->state < 2)`, and total<0x400 gates the pad,
    // src/fn_183398.c `if (p->total < 0x400)` with its inner
    // `if (padFirst > 0x400)` clamp). The port's finish_m already ports
    // those inners 1:1, so no outer `sched_written` gate is kept.
    // Speech/user derive live exactly like the 0x1530d0 pump's own path
    // (user = [synth+0x28], speech = [user+0x184]); b is the 0x684f424a
    // request magic the port already carries. The per-speak total is the
    // snapshot's post-setup value (src/fn_35a9b0.c `p->f80 = 0` runs at
    // voice load, before the 0x191e40 template, so the arena already
    // holds 0): sched renders accumulate into it 1:1 with the C
    // (`p->total += (first+second)*2` in renderSamples and fn_183398),
    // and this block must not re-zero it after the sched run, or the
    // finish pad gate (`p->total < 0x400`) misfires on long requests.
    // The null guard mirrors the pump's
    // own early-out (src/fn_151914.c fn_1530d0 `if (p->putOutput == 0
    // || out->state == 2) return 0` / `if (p->status == 2) return 0`):
    // with no app object there is no finish render to run.
    let mut finish_written = 0usize;
    {
        let (user, pspeech) = {
            let arena: &mut [u8] = &mut *state.arena;
            let bm = boot::Memory {
                base: 0,
                bytes: arena,
            };
            let u = bm.word(synth.wrapping_add(0x28));
            let ps = if u == 0 {
                0
            } else {
                bm.word(u.wrapping_add(0x184))
            };
            (u, ps)
        };
        if user != 0 && pspeech != 0 {
            let crate::engine_us::FinishBuffers { input, output } = &mut state.finish;
            let mut grain = [0i32; 16384];
            let mut window = [0f32; 16384];
            let mut modulation = [0f32; 1024];
            let mut normalization = [0f32; 32];
            let mut wave = [0i16; 1024];
            let mut vr0 = [0f32; 16384];
            let mut vr1 = [0f32; 16384];
            let mut vr2 = [0f32; 16384];
            let mut delay = [0u16; 65536];
            let mut audio = [0i16; crate::queue::AUDIO_CAPACITY];
            let mut samples = [0i16; 65536];
            let mut events = [crate::queue::SpeechEvent::default(); 16384];
            let mut first = [0i16; 0x25800];
            let mut second = [0i16; 0x25800];
            let mut workspace = engine_live::BackendWorkspace {
                input,
                output,
                grain: &mut grain,
                window: &mut window,
                modulation: &mut modulation,
                normalization: &mut normalization,
                wave: &mut wave,
                voice_delays: [&mut vr0, &mut vr1, &mut vr2],
                sample_delay: &mut delay,
                audio: (&mut audio[..]).try_into().unwrap(),
                samples: &mut samples,
                events: &mut events,
                first: &mut first,
                second: &mut second,
                parameter_values: None,
            };
            let tail: &mut [i16] = &mut out[sched_written..];
            let mut nap = FinishNap;
            let mut inner = engine_live::RuntimeM {
                backend: Some(&mut workspace),
                pcm: tail,
                written: 0,
                tick: state.boot.tick,
                registers: engine::NotificationRegisters {
                    s16: state.boot.s16,
                    s17: state.boot.s17,
                },
                sleeper: &mut nap,
            };
            let mut hooks2 = UsHooks {
                platform: &mut *state.platform,
                runtime: &mut inner,
                snap: &*state.snap,
                first_unbound: None,
                first_error: None,
            };
            let arena: &mut [u8] = &mut *state.arena;
            let mut bm = boot::Memory {
                base: 0,
                bytes: arena,
            };
            engine_live::finish_m(&mut bm, pspeech, user, 0x684f424a, 0x1c40, &mut hooks2);
            finish_written = inner.written;
        }
    }
    let written = sched_written + finish_written;
    // fn_15b404 tail -> fn_1561b8 drain (round 17, 1:1 with ARM): SendDone,
    // CloseOutStreams (final pump through the sink below), pipe/aux/lex
    // teardown, arr/extra frees. Runs only on scheduler success, before
    // the state tails (ARM order: drain, then 15961c/191e40 tails).
    let mut drain_sink = ForwardSink {
        outer: &mut *state.runtime,
        out: &mut out[written..],
        written: 0,
        emit_err: None,
    };
    // Same tail pairing on a drain fault (1:1): the drain runs after the
    // chain's enters, so its failure exit restores them too.
    let drain_rc = match {
        let arena: &mut [u8] = &mut *state.arena;
        let mut region = [engine::Region { base: 0, bytes: arena }];
        let mut mem = engine::Memory::new(&mut region)?;
        engine_live::synthesis_drain_m(
            &mut mem,
            synth,
            &mut *state.workspace,
            &mut *state.platform,
            &mut drain_sink,
            &state.snap,
        )
    } {
        Ok(rc) => rc,
        Err(e) => {
            drop(drain_sink);
            request_tail_restore(state, previous)?;
            synthesis_tail_restore(state, seg, previous_pipe)?;
            return Err(e);
        }
    };
    let written = written + drain_sink.written;
    if let Some(e) = drain_sink.emit_err {
        drop(drain_sink);
        request_tail_restore(state, previous)?;
        synthesis_tail_restore(state, seg, previous_pipe)?;
        return Err(e);
    }
    drop(drain_sink);
    if drain_rc >= 0x80000000 {
        request_tail_restore(state, previous)?;
        synthesis_tail_restore(state, seg, previous_pipe)?;
        return Err(Error::EngineStatus(drain_rc));
    }
    // fn_191e40 + fn_15961c tails (1:1): restore the request and pipeline
    // states the enters saved, so the next render's enters see idle again.
    // State the ARM keeps (heap blocks, synth objects, PCM) is left
    // exactly as rendered.
    request_tail_restore(state, previous)?;
    synthesis_tail_restore(state, seg, previous_pipe)?;
    // Phase 3: every scheduler pump rendered through the merged Host and
    // appended its samples to `out` via the sink. The count is the PCM
    // the complete run produced.
    // seg7 r13: shared app-boundary status (gamePatch callTTS end / c722f0:
    // FINISHED unless ERROR).
    {
        let arena: &mut [u8] = &mut *state.arena;
        let mut bm = boot::Memory { base: 0, bytes: arena };
        crate::engine_live_seg2::job_mark_finished(&mut bm, crate::engine_live_seg2::JOB_ADDR);
    }
    // wire-a r48: serve the capture buffer (job audioData semantics: this
    // is what the citra worker serves), not the sink. Fresh renders are
    // byte-identical to the sink (proven); at exhaustion the capture is
    // the citra-short while the sink stays full. NULL buffer serves zeros
    // (citra reads zeros through NULL). Caller-owned `out` only.
    let served = {
        let arena: &[u8] = &*state.arena;
        let rd = |o: usize| -> u32 {
            u32::from_le_bytes(arena.get(o..o + 4).unwrap_or(&[0, 0, 0, 0]).try_into().unwrap())
        };
        let slot = rd(0xc72688) as usize;
        let st = rd(slot) as usize;
        let (cbuf, clen) = if st == 0 || st.wrapping_add(24) > arena.len() {
            (0, 0)
        } else {
            (rd(st.wrapping_add(16)), rd(st.wrapping_add(12)))
        };
        let nsamples = (clen as usize) / 2;
        if nsamples > out.len() {
            request_tail_restore(state, previous)?;
            synthesis_tail_restore(state, seg, previous_pipe)?;
            return Err(Error::Capacity { required: nsamples, available: out.len() });
        }
        if cbuf == 0 {
            for s in &mut out[..nsamples] {
                *s = 0;
            }
        } else if (cbuf as usize).wrapping_add(clen as usize) > arena.len() {
            request_tail_restore(state, previous)?;
            synthesis_tail_restore(state, seg, previous_pipe)?;
            return Err(Error::Capacity { required: clen as usize, available: 0 });
        } else {
            for (i, s) in out[..nsamples].iter_mut().enumerate() {
                let o = (cbuf as usize) + i * 2;
                *s = i16::from_le_bytes([arena[o], arena[o + 1]]);
            }
        }
        // Publish the served buffer as the job audio (1:1 with the ARM job
        // completion the worker reads: audioData/audioSize).
        {
            let arena: &mut [u8] = &mut *state.arena;
            let at = crate::engine_live_seg2::JOB_ADDR as usize + 12;
            arena[at..at + 4].copy_from_slice(&(clen).to_le_bytes());
            arena[at + 4..at + 8].copy_from_slice(&(cbuf).to_le_bytes());
        }
        nsamples
    };
    Ok(served)
}
