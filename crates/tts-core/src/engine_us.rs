

use crate::{boot, engine, engine_backend, engine_live, engine_live_seg2};
use crate::rom_region::{capture_slot, Rom};
use crate::reentrant::HooksM;

pub use crate::engine::Error;
pub use crate::engine_live_seg2::RequestArgs as BootArgs;
pub use crate::engine_live_seg2::TableBuffers;
pub use crate::engine_live_seg2::{TEXT_ADDR, VOICE_WRAPPER};

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
    pub rom: Rom,
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
        Self { boot, rom: Rom::Us, arena, snap, workspace, scratch, tables: TableBuffers::new(), platform, runtime, finish: FinishBuffers::new() }
    }
}

fn guest_entry(rom: Rom, us: u32) -> u32 {
    if rom == Rom::Us {
        return us;
    }
    crate::rom_region::addr(rom, crate::rom_region::to_us(rom, us))
}

pub const SCHEDULER_ORDER: &[u32] = &[
    0x1d5ba8, 0x200e64, 0x1e4b7c, 0x206be4, 0x1dd914, 0x1f98c8, 0x1f1440, 0x1f3248,
    0x1df780, 0x1e7908, 0x1f3ea0, 0x1f51f8, 0x2104b0, 0x227d64, 0x227d64, 0x227d64,
    0x22a0a8, 0x22d6dc, 0x1e3b14, 0x231ba0, 0x1530d0,
];

pub fn scheduler_advance(m: &mut boot::Memory<'_>, synth: u32) {
    let pending = m.word(synth.wrapping_add(0xd4));
    m.put_word(synth.wrapping_add(0xd4), pending.wrapping_add(1));
    let at = synth.wrapping_add(0xd8);
    let off = (at - m.base) as usize;
    let half = u16::from_le_bytes([m.bytes[off], m.bytes[off + 1]]);
    m.bytes[off..off + 2].copy_from_slice(&half.wrapping_add(1).to_le_bytes());
}

pub fn scheduler_stack_arg(m: &boot::Memory<'_>, synth: u32) -> u32 {
    let extra = m.word(synth.wrapping_add(0xdc));
    let at = synth.wrapping_add(0xd8);
    let off = (at - m.base) as usize;
    let stage = u16::from_le_bytes([m.bytes[off], m.bytes[off + 1]]) as u32;
    extra.wrapping_add(stage)
}

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

    emit_err: Option<engine::Error>,
}

impl engine_live::RuntimeServicesM for ForwardSink<'_, '_> {
    fn guest_capture(&self)->bool {self.outer.guest_capture()}

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

struct UsHooks<'p, 'r> {
    platform: &'p mut dyn engine::HeapPlatform,
    runtime: &'r mut dyn engine_live::RuntimeServicesM,
    snap: &'r [u8],
    rom: Rom,
    first_unbound: Option<u32>,

    first_error: Option<Error>,
}

impl crate::reentrant::HooksM for UsHooks<'_, '_> {
    fn call_m(&mut self, address: u32, args: &[u32], mem: &mut boot::Memory<'_>) -> u32 {
        let address = guest_entry(self.rom, address);

        let result = engine_live::call_live_with_runtime_m_rom(
                mem,
                address,
                args,
                &mut *self.platform,
                &mut *self.runtime,
                self.snap,
                self.rom,
            );

        match result {
            Ok(rc) => {

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

        if self.first_unbound.is_none() {
            self.first_unbound = Some(0x15429c);
        }
        crate::frontend::PipelineReply { status: 0x80002001, value: 0, data: 0 }
    }
    fn rom(&self) -> Rom {
        self.rom
    }

}

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

pub struct Intake {
    pub request: engine_live_seg2::RequestArgs,
    pub tts: u32,
    pub text: u32,
}

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
        rom: state.rom,
        first_unbound: None,
        first_error: None,
    };

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

    {
        let arena: &mut [u8] = &mut *state.arena;
        let mut region = [engine::Region { base: 0, bytes: arena }];
        let mut mem = engine::Memory::new(&mut region)?;
        mem.read(
            crate::rom_region::addr(state.rom, 0xa21fb0),
            &mut state.tables.constants,
        )?;
        mem.read(
            crate::rom_region::addr(state.rom, 0x8cbfe0),
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
            state.rom,
            Some(&mut host),
        )?;
    }

    {
        let arena: &mut [u8] = &mut *state.arena;
        let at = engine_live_seg2::JOB_ADDR as usize + 12;
        arena[at..at + 8].copy_from_slice(&0u64.to_le_bytes());
    }

    {
        let arena: &mut [u8] = &mut *state.arena;
        let mut region = [engine::Region { base: 0, bytes: arena }];
        let mut mem = engine::Memory::new(&mut region)?;
        engine_live_seg2::request_setup_rom(
            &mut mem,
            &request,
            state.rom,
        )?;
    }

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
    }
    drop(hooks);
    Ok(Intake { request: engine_live_seg2::RequestArgs { request: tts, ..request }, tts, text: text_copy })
}

pub fn guest_free(state: &mut UsEngine<'_>, block: u32) -> Result<(), Error> {
    let mut hooks = UsHooks {
        platform: &mut *state.platform,
        runtime: &mut *state.runtime,
        snap: &*state.snap,
        rom: state.rom,
        first_unbound: None,
        first_error: None,
    };
    heap_dispatch(&mut hooks, &mut *state.arena, 0x11da30, &[block])?;
    Ok(())
}

pub fn unprepare_voice(state: &mut UsEngine<'_>) {
    let arena: &mut [u8] = &mut *state.arena;
    let w = state.boot.voice as usize;
    let obj = u32::from_le_bytes(arena[w + 8..w + 12].try_into().unwrap()) as usize;
    if let Some(flag) = arena.get_mut(obj + 0x61a0) {
        *flag = 0;
    }
}

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
            state.rom,
        )?
    };
    Ok(boundary)
}

pub fn speak_release(state: &mut UsEngine<'_>, intake: &Intake) -> Result<(), Error> {
    let mut hooks = UsHooks {
        platform: &mut *state.platform,
        runtime: &mut *state.runtime,
        snap: &*state.snap,
        rom: state.rom,
        first_unbound: None,
        first_error: None,
    };
    heap_dispatch(&mut hooks, &mut *state.arena, 0x11da30, &[intake.tts])?;
    heap_dispatch(&mut hooks, &mut *state.arena, 0x11da30, &[intake.text])?;
    Ok(())
}

struct TailMutex;
impl engine::MutexPlatform for TailMutex {
    fn acquire_mutex(&mut self, _: u32) -> u32 {
        0x81402001
    }
    fn release_mutex(&mut self, _: u32) -> u32 {
        0x81402001
    }
}

fn synthesis_tail_restore(state: &mut UsEngine<'_>, seg: u32, previous: u8) -> Result<(), Error> {
    let mut host = TailMutex;
    let arena: &mut [u8] = &mut *state.arena;
    let mut region = [engine::Region { base: 0, bytes: arena }];
    let mut mem = engine::Memory::new(&mut region)?;
    engine::synthesis_restore_rom(&mut mem, seg, previous as u32, &mut host)
}

fn request_tail_restore(state: &mut UsEngine<'_>, previous: u8) -> Result<(), Error> {
    let mut host = TailMutex;
    let engine_addr = state.boot.engine;
    let arena: &mut [u8] = &mut *state.arena;
    let mut region = [engine::Region { base: 0, bytes: arena }];
    let mut mem = engine::Memory::new(&mut region)?;
    engine::request_restore_rom(&mut mem, engine_addr, previous as u32, &mut host)
}

fn scheduler_setup(
    hooks: &mut UsHooks<'_, '_>,
    bm: &mut boot::Memory<'_>,
    synth: u32,
) -> Result<(), Error> {

    let fifth = scheduler_stack_arg(bm, synth);
    let out = bm.word(synth.wrapping_add(0xa0));
    let r2obj = bm.word(synth.wrapping_add(0x7c));
    let table = bm.word(r2obj.wrapping_add(8));
    let r4 = bm.word(table.wrapping_add(4));
    let p = bm.word(r4.wrapping_add(0xc));
    let tag = bm.word(r4.wrapping_add(0x10));
    let tag3 = bm.word(synth.wrapping_add(0xa4));
    if tag != 0x10d || tag3 != 0xb2c2 {
        return Err(Error::UnsupportedStage { entry: guest_entry(hooks.rom, 0x22b518) });
    }
    let drv: [u32; 5] = [p, 0x10d, out, 0xb2c2, fifth];
    let seq: &[(u32, &[u32])] = &[(guest_entry(hooks.rom, 0x22b518), &drv), (guest_entry(hooks.rom, 0x22b518), &drv)];
    for (entry, args) in seq {
        let rc = hooks.call_m(*entry, args, bm);
        if rc >= 0x80000000 {
            if hooks.rom == crate::rom_region::Rom::Eu && hooks.first_unbound.is_none() {
                return Err(Error::EngineStatus(rc));
            }
            return Err(Error::UnsupportedStage {
                entry: hooks.first_unbound.unwrap_or(*entry),
            });
        }
    }
    Ok(())
}

pub fn speak(
    state: &mut UsEngine<'_>,
    text: &[u16],
    params: &VoiceParams,
    out: &mut [i16],
) -> Result<usize, Error> {
    match state.rom {
        Rom::Us if params.language != 1 => return Err(Error::InvalidVoice),
        Rom::Eu if !(1..=5).contains(&params.language) => return Err(Error::InvalidVoice),
        Rom::Kr => {
            return Err(Error::UnsupportedStage { entry: 0x15429c });
        }
        _ => {}
    }
    if out.is_empty() {
        return Err(Error::Capacity { required: 1, available: 0 });
    }

    {
        let arena: &mut [u8] = &mut *state.arena;
        let mut bm = boot::Memory { base: 0, bytes: arena };
        crate::engine_live_seg2::job_mark_processing(&mut bm, crate::engine_live_seg2::JOB_ADDR);
    }

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

    {
        let arena: &mut [u8] = &mut *state.arena;
        let mut bm = boot::Memory { base: 0, bytes: arena };
        crate::engine_live_seg2::job_mark_processing(&mut bm, crate::engine_live_seg2::JOB_ADDR);
    }

    let setup_entry_prepared = {
        let arena: &[u8] = &*state.arena;
        let w = state.boot.voice as usize;
        let obj = u32::from_le_bytes(arena[w + 8..w + 12].try_into().unwrap());
        arena.get(obj as usize + 0x61a0).copied().unwrap_or(0) != 0
    };
    let intake = speak_pre_chain(state, text, params)?;

    let result = speak_intake(state, &intake, setup_entry_prepared, out);
    let released = speak_release(state, &intake);
    let samples = result?;
    released?;
    Ok(samples)
}

fn speak_intake(
    state: &mut UsEngine<'_>,
    intake: &Intake,
    setup_entry_prepared: bool,
    out: &mut [i16],
) -> Result<usize, Error> {

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

    let boundary = match speak_post_chain(state, intake) {
        Ok(b) => {
            b
        }
        Err(e) => {
            request_tail_restore(state, previous)?;
            synthesis_tail_restore(state, seg, previous_pipe)?;
            return Err(e);
        }
    };
    if boundary.entry != 0x15429c {
        return Err(Error::UnsupportedStage { entry: boundary.entry });
    }

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

    {
        let arena: &mut [u8] = &mut *state.arena;
        let mut bm = boot::Memory { base: 0, bytes: arena };
        let mut hooks_tmp = UsHooks {
            platform: &mut *state.platform,
            runtime: &mut *state.runtime,
            snap: &*state.snap,
            rom: state.rom,
            first_unbound: None,
            first_error: None,
        };

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
                        state.rom,
                    );
                }
            }
            request_tail_restore(state, previous)?;
            synthesis_tail_restore(state, seg, previous_pipe)?;
            return Err(e);
        }
    }

    let synth = boundary.args[0];
    let flush = boundary.args[1];
    let mut activity = [0u8; 256];
    let mut sink = ForwardSink { outer: &mut *state.runtime, out, written: 0, emit_err: None };
    let mut hooks = UsHooks {
        platform: &mut *state.platform,
        runtime: &mut sink,
        snap: &*state.snap,
        rom: state.rom,
        first_unbound: None,
        first_error: None,
    };
    let rc = {
        let arena: &mut [u8] = &mut *state.arena;
        let mut bm = boot::Memory { base: 0, bytes: arena };
        engine_live::process_text_m(&mut bm, synth, flush, &mut activity, 0x1e00, &mut hooks)
    };
    if rc >= 0x80000000 {

        let first_unbound = hooks.first_unbound;
        let first_error = hooks.first_error;
        drop(hooks);
        let emit_err = sink.emit_err;
        let sched_written = sink.written;
        drop(sink);

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
                    state.rom,
                );
            }
        }

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
                rom: state.rom,
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
            let mut output_runtime=engine_live::OutputRuntime {inner:&mut inner,guest_capture:state.runtime.guest_capture()};
            let mut hooks2 = UsHooks {
                platform: &mut *state.platform,
                runtime: &mut output_runtime,
                snap: &*state.snap,
                rom: state.rom,
                first_unbound: None,
                first_error: None,
            };
            let arena: &mut [u8] = &mut *state.arena;
            let mut bm = boot::Memory {
                base: 0,
                bytes: arena,
            };
            engine_live::finish_m(&mut bm, pspeech, user, 0x684f424a, 0x1c40, &mut hooks2);
            if let Some(e)=hooks2.first_error {return Err(e);}
            if let Some(entry)=hooks2.first_unbound {return Err(Error::UnsupportedStage{entry});}
            finish_written = inner.written;
        }
    }
    let written = sched_written + finish_written;

    let mut drain_sink = ForwardSink {
        outer: &mut *state.runtime,
        out: &mut out[written..],
        written: 0,
        emit_err: None,
    };

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
            state.rom,
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

    request_tail_restore(state, previous)?;
    synthesis_tail_restore(state, seg, previous_pipe)?;

    {
        let arena: &mut [u8] = &mut *state.arena;
        let mut bm = boot::Memory { base: 0, bytes: arena };
        crate::engine_live_seg2::job_mark_finished(&mut bm, crate::engine_live_seg2::JOB_ADDR);
    }
    if !state.runtime.guest_capture() {return Ok(written);}

    let served = {
        let arena: &[u8] = &*state.arena;
        let rd = |o: usize| -> u32 {
            u32::from_le_bytes(arena.get(o..o + 4).unwrap_or(&[0, 0, 0, 0]).try_into().unwrap())
        };
        let slot = rd(capture_slot(state.rom) as usize) as usize;
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
