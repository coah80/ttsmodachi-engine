

use crate::engine;
use crate::boot::Memory as BootMemory;

pub fn run_step(
    m: &mut crate::boot::Memory<'_>,
    args: &[u32],
    platform: &mut dyn engine::HeapPlatform,
    runtime: &mut dyn crate::engine_live::RuntimeServicesM,
    snap: &[u8],
) -> Result<u32, engine::Error> {
    crate::engine_live::call_live_with_runtime_m(m, 0x1cca64, args, platform, runtime, snap)
}

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

pub fn run_1dd914<P: engine::HeapPlatform>(
    m: &mut crate::boot::Memory<'_>,
    args: &[u32],
    changed: u32,
    platform: &mut P,
) -> (u32, Option<u32>) {
    run_1dd914_rom(m, args, changed, platform, crate::rom_region::Rom::Us)
}

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

pub const TEXT_ADDR: u32 = 0xB27DAA;

pub const JOB_ADDR: u32 = 0x00AF340D;

pub const JOB_WAITING: u8 = 1;
pub const JOB_PROCESSING: u8 = 2;
pub const JOB_FINISHED: u8 = 3;
pub const JOB_ERROR: u8 = 4;
pub const JOB_TEXT_READY: u8 = 5;

pub fn job_mark_processing(m: &mut crate::boot::Memory<'_>, job: u32) {
    let at = job as usize;
    if at < m.bytes.len() && m.bytes[at] == JOB_TEXT_READY {
        m.bytes[at] = JOB_PROCESSING;
    }
}

pub fn job_mark_finished(m: &mut crate::boot::Memory<'_>, job: u32) {
    let at = job as usize;
    if at < m.bytes.len() && m.bytes[at] != JOB_ERROR {
        m.bytes[at] = JOB_FINISHED;
    }
}

pub const VOICE_WRAPPER: u32 = 0x8668E00;

pub const VOICE_WRAPPER_EU: u32 = 0x86731A0;

pub const VOICE_WRAPPER_KR: u32 = 0x866A260;

#[derive(Clone, Copy, Debug)]
pub struct VoiceParams {
    pub pitch: u8,
    pub speed: u8,
    pub quality: u8,
    pub tone: u8,
    pub accent: u8,
    pub intonation: u8,
}

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

            crate::boot_part_b2::fn_183edc(m, a[0], 0, h);
            0
        }
        0x165d00 => {
            need(1);

            let g = crate::boot::Globals {
                manager: 0, callbacks: 0, rules: 0, components: 0,
                dictionary: 0, context: 0, mode: 0, semaphore: 0,
                service: 0, command: 0,
            };
            m.fn_165d00(h, &g, a[0]);
            0
        }

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
    fn call_wide(&mut self, _: &mut BootMemory<'_>, entry: u32, _: &[u32]) -> u64 {
        panic!("unbound request lock wide call {entry:#x}")
    }
}

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

        return Err(engine::Error::Capacity {
            required: (n + 1) * 2,
            available: text_buf.len() * 2,
        });
    }
    request_to_scheduler_rom(m, &text_buf[..n], params, args, tables, workspace, scratch, crate::rom_region::Rom::Us, None)
}

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

    let mut at = TEXT_ADDR;
    for unit in text {
        m.write(at, &unit.to_le_bytes())?;
        at += 2;
    }
    m.write(at, &[0, 0])?;

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

    let mut controls = RequestLocks { tick: args.tick };
    crate::engine_backend::set_voice_rom(m, 0x3912fc, args.voice, params.pitch as i32, &numeric, rom)?;
    crate::engine_backend::set_quality_rom(m, args.voice, params.speed as i32, &mut controls, rom, host)?;
    crate::engine_backend::set_voice_rom(m, 0x3912ec, args.voice, params.quality as i32, &numeric, rom)?;
    crate::engine_backend::set_voice_rom(m, 0x39133c, args.voice, params.tone as i32, &numeric, rom)?;
    crate::engine_backend::set_voice_rom(m, 0x39131c, args.voice, params.accent as i32, &numeric, rom)?;
    crate::engine_backend::set_voice_rom(m, 0x39132c, args.voice, params.intonation as i32, &numeric, rom)?;
    Ok(())
}

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
    let boot = engine::BootState {
        engine: args.engine,
        magic: args.magic,
    };
    let mut prefix = engine::request_prefix_rom(m, boot, args.request, &mut mutex, rom)?;

    let (rc, h) = engine::open_text_live_rom(m, boot.engine, args.request, rom)?;

    if rc >= 0x80000000 {
        return Err(engine::Error::EngineStatus(rc));
    }
    prefix.stream = h;
    prefix.blocked_at = engine::Error::MissingStreamAdapter { entry: 0x190410 };

    let boundary = engine::voice_start_inner(
        m,
        engine::StageCall {
            entry: 0x190410,
            args: [args.engine, 0, prefix.stream[0], prefix.stream[1], 0, 0, 0],
        },
        &mut mutex,
        Some(engine::NotificationRegisters {
            s16: args.s16,
            s17: args.s17,
        }),
        Some(&mut ticks),
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

    let aux_us = engine::VoiceBoundary {
        entry: crate::rom_region::to_us(rom, aux.entry),
        ..aux
    };
    let db = crate::engine_live::database_return(m, aux_us, args.request_sp, workspace, &mut heap, rom)?;
    let start = crate::engine_live::component_boundary(m, synthesis, args.request_sp, workspace, &mut heap, rom)?;
    let boundary = crate::engine_live::first_component(m, start, args.request_sp, workspace, &mut heap, rom)?;
    Ok(boundary)
}

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

pub fn do_tts_live_after_prologue(
    m: &mut engine::Memory<'_, '_>,
    tts_input: u32,
    args: &RequestArgs,
    workspace: &mut [u8],
    scratch: &mut [u8],
) -> Result<engine::VoiceBoundary, engine::Error> {

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

pub fn scheduler_advance_after_step(m: &mut crate::boot::Memory<'_>) {
    let half_at = 0x14c403e8usize;
    let half = u16::from_le_bytes([m.bytes[half_at], m.bytes[half_at + 1]]);
    m.bytes[half_at..half_at + 2].copy_from_slice(&(half.wrapping_add(1)).to_le_bytes());
    let word_at = 0x14c403e4usize;
    let word = u32::from_le_bytes(m.bytes[word_at..word_at + 4].try_into().unwrap());
    m.bytes[word_at..word_at + 4].copy_from_slice(&(word.wrapping_add(1)).to_le_bytes());
}

pub fn scheduler_bookkeeping_m(m: &mut crate::boot::Memory<'_>, e4: u32, e8: u16, flag: &[u8]) {
    if m.bytes.len() > 0x14c403ec {
        m.bytes[0x14c403e4..0x14c403e8].copy_from_slice(&e4.to_le_bytes());
        m.bytes[0x14c403e8..0x14c403ea].copy_from_slice(&e8.to_le_bytes());
    }
    if !flag.is_empty() && m.bytes.len() > 0x14c428b8 + flag.len() {
        m.bytes[0x14c428b8..0x14c428b8 + flag.len()].copy_from_slice(flag);
    }
}

pub fn pump_prologue_m(m: &mut crate::boot::Memory<'_>) {
    const OO: usize = 0x14c40394;
    if m.bytes.len() <= OO + 0x6c {
        return;
    }
    m.bytes[OO + 0x64..OO + 0x68].copy_from_slice(&32u32.to_le_bytes());
    m.bytes[OO + 0x68..OO + 0x6c].copy_from_slice(&0u32.to_le_bytes());
}
