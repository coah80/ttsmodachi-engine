

use crate::text;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    Capacity { required: usize, available: usize },
    InvalidSnapshot,
    UnmappedMemory { address: u32, bytes: usize },
    UnsupportedStage { entry: u32 },
    AliasedEncodingBuffers,
    EngineStatus(u32),
    MissingTextStreamRuntime,
    MissingStreamAdapter { entry: u32 },
    InvalidVoice,
    InputTooLong,
    MissingBoot11cc0c,
    MissingInitializedRuntimeAdapters191e40,

    CodeOverwritten { end: u32 },

    HeapExhausted { size: u32 },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Settings {

    pub parameters: [u8; 6],
    pub bpm: u32,
    pub stretch: u32,
}
impl Default for Settings {
    fn default() -> Self {
        Self {
            parameters: [50, 50, 50, 50, 50, 1],
            bpm: 120,
            stretch: 50,
        }
    }
}

pub trait Platform {
    fn ticks(&mut self) -> u64;
}

pub struct Snapshot<'a> {
    pub code: &'a [u8],
    pub heap: &'a [u8],
    pub stack: &'a [u8],
    pub linear: &'a [u8],
    pub sysmem: &'a [u8],
}

impl Snapshot<'_> {
    fn word(&self, address: u32) -> Option<u32> {
        for (base, bytes) in [
            (0x0010_0000, self.code),
            (0x0800_0000, self.heap),
            (0x0fe0_0000, self.stack),
            (0x1400_0000, self.linear),
            (0x1ff8_0000, self.sysmem),
        ] {
            if let Some(at) = address.checked_sub(base) {
                let at = at as usize;
                if let Some(value) = bytes.get(at..at.checked_add(4)?) {
                    return Some(u32::from_le_bytes(value.try_into().ok()?));
                }
            }
        }
        None
    }

    pub fn engine_object(&self) -> Result<u32, Error> {
        let global = self
            .word(0x00ac_b54c)
            .filter(|&p| p != 0)
            .ok_or(Error::InvalidSnapshot)?;
        let engine = self
            .word(global.checked_add(8).ok_or(Error::InvalidSnapshot)?)
            .filter(|&p| p != 0)
            .ok_or(Error::InvalidSnapshot)?;
        self.word(engine).ok_or(Error::InvalidSnapshot)?;
        Ok(engine)
    }
}

pub struct Voice<'a> {
    pub components: &'a [u8],
    pub dictionary: &'a [u8],
    pub ruleset: &'a [u8],
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Start {
    Initialized,
    Cold,
}

pub struct Request<'a> {

    pub text: &'a [u16],
    pub settings: Settings,

    pub song: &'a [u8],
}

#[derive(Default, Debug)]
pub struct State {
    pub encoded_bytes: usize,
    pub pcm_samples: usize,
    pub started_at: u64,
    pub inspected_engine: Option<u32>,
}

pub fn missing_boot_11cc0c() -> Result<usize, Error> {
    Err(Error::MissingBoot11cc0c)
}

pub fn missing_initialized_runtime_adapters_191e40() -> Result<usize, Error> {
    Err(Error::MissingInitializedRuntimeAdapters191e40)
}

pub fn render(
    state: &mut State,
    request: Request<'_>,
    snapshot: &Snapshot<'_>,
    voice: &Voice<'_>,
    start: Start,
    encoded: &mut [u8],
    _pcm: &mut [i16],
    platform: &mut impl Platform,
) -> Result<usize, Error> {
    *state = State::default();
    if request.text.len() > u32::MAX as usize / 3 {
        return Err(Error::InputTooLong);
    }
    let required = request
        .text
        .iter()
        .map(|&c| match c {
            0..=0x7f => 1,
            0x80..=0x7ff => 2,
            _ => 3,
        })
        .sum();
    if encoded.len() < required {
        return Err(Error::Capacity {
            required,
            available: encoded.len(),
        });
    }
    let conversion = text::encode(request.text, &mut encoded[..required], None);
    state.encoded_bytes = conversion.written as usize;
    if voice.components.is_empty() || voice.dictionary.is_empty() || voice.ruleset.is_empty() {
        return Err(Error::InvalidVoice);
    }
    state.inspected_engine = Some(snapshot.engine_object()?);
    state.started_at = platform.ticks();
    match start {
        Start::Cold => missing_boot_11cc0c(),
        Start::Initialized => missing_initialized_runtime_adapters_191e40(),
    }
}

#[derive(Clone, Copy, Debug)]
pub struct BootState {
    pub engine: u32,
    pub magic: u32,
}

pub struct Region<'a> {
    pub base: u32,
    pub bytes: &'a mut [u8],
}

pub struct Memory<'a, 'b> {
    pub(crate) regions: &'a mut [Region<'b>],
}

impl<'a, 'b> Memory<'a, 'b> {
    pub fn new(regions: &'a mut [Region<'b>]) -> Result<Self, Error> {
        for (i, region) in regions.iter().enumerate() {
            let end = u64::from(region.base) + region.bytes.len() as u64;
            if end > 0x1_0000_0000 {
                return Err(Error::InvalidSnapshot);
            }
            for other in &regions[..i] {
                if u64::from(region.base) < u64::from(other.base) + other.bytes.len() as u64
                    && u64::from(other.base) < end
                {
                    return Err(Error::InvalidSnapshot);
                }
            }
        }
        Ok(Self { regions })
    }

    fn locate(&self, address: u32) -> Option<(usize, usize)> {
        self.regions.iter().enumerate().find_map(|(i, region)| {
            let offset = address.checked_sub(region.base)? as usize;
            (offset < region.bytes.len()).then_some((i, offset))
        })
    }

    pub fn check(&self, address: u32, bytes: usize) -> Result<(), Error> {
        let mut done = 0usize;
        while done < bytes {
            let at = address
                .checked_add(u32::try_from(done).map_err(|_| Error::InputTooLong)?)
                .ok_or(Error::InputTooLong)?;
            let (i, offset) = self.locate(at).ok_or(Error::UnmappedMemory {
                address: at,
                bytes: bytes - done,
            })?;
            done += (self.regions[i].bytes.len() - offset).min(bytes - done);
        }
        Ok(())
    }

    pub fn read(&self, address: u32, output: &mut [u8]) -> Result<(), Error> {
        self.check(address, output.len())?;
        let mut done = 0;
        while done < output.len() {
            let (i, offset) = self.locate(address + done as u32).unwrap();
            let count = (self.regions[i].bytes.len() - offset).min(output.len() - done);
            output[done..done + count]
                .copy_from_slice(&self.regions[i].bytes[offset..offset + count]);
            done += count;
        }
        Ok(())
    }

    pub fn write(&mut self, address: u32, input: &[u8]) -> Result<(), Error> {
        self.check(address, input.len())?;
        let mut done = 0;
        while done < input.len() {
            let (i, offset) = self.locate(address + done as u32).unwrap();
            let count = (self.regions[i].bytes.len() - offset).min(input.len() - done);
            self.regions[i].bytes[offset..offset + count]
                .copy_from_slice(&input[done..done + count]);
            done += count;
        }
        Ok(())
    }

    pub fn word(&self, address: u32) -> Result<u32, Error> {
        let mut bytes = [0; 4];
        self.read(address, &mut bytes)?;
        Ok(u32::from_le_bytes(bytes))
    }

    fn unit(&self, address: u32) -> Result<u16, Error> {
        let mut bytes = [0; 2];
        self.read(address, &mut bytes)?;
        Ok(u16::from_le_bytes(bytes))
    }

    fn byte(&self, address: u32) -> Result<u8, Error> {
        let mut byte = [0];
        self.read(address, &mut byte)?;
        Ok(byte[0])
    }

    pub fn put_word(&mut self, address: u32, value: u32) -> Result<(), Error> {
        self.write(address, &value.to_le_bytes())
    }
}

fn offset(base: u32, bytes: usize) -> Result<u32, Error> {
    base.checked_add(u32::try_from(bytes).map_err(|_| Error::InputTooLong)?)
        .ok_or(Error::InputTooLong)
}

fn overlaps(a: u32, an: usize, b: u32, bn: usize) -> bool {
    an != 0
        && bn != 0
        && u64::from(a) < u64::from(b) + bn as u64
        && u64::from(b) < u64::from(a) + an as u64
}

pub trait MutexPlatform {
    fn acquire_mutex(&mut self, target: u32) -> u32;
    fn release_mutex(&mut self, target: u32) -> u32;
}

struct Lock<'a> {
    target: u32,
    acquire_slot: bool,
    release_slot: bool,
    present: bool,
    host: &'a mut dyn MutexPlatform,
}

impl crate::frontend::StateLock for Lock<'_> {
    fn acquire(&mut self) -> u32 {
        if !self.present {
            0x81402007
        } else if self.acquire_slot {
            self.host.acquire_mutex(self.target)
        } else {
            0
        }
    }

    fn release(&mut self) -> u32 {
        if !self.present {
            0x81402007
        } else if self.release_slot {
            self.host.release_mutex(self.target)
        } else {
            0
        }
    }
}

fn lock<'a>(
    memory: &Memory<'_, '_>,
    object: u32,
    field: usize,
    host: &'a mut dyn MutexPlatform,
) -> Result<Lock<'a>, Error> {
    let wrapper = memory.word(offset(object, field)?)?;
    if wrapper == 0 {
        return Ok(Lock {
            target: 0,
            acquire_slot: false,
            release_slot: false,
            present: false,
            host,
        });
    }
    let table = memory.word(offset(memory.word(wrapper)?, 8)?)?;
    Ok(Lock {
        target: memory.word(offset(wrapper, 8)?)?,
        acquire_slot: memory.word(offset(table, 8)?)? != 0,
        release_slot: memory.word(offset(table, 12)?)? != 0,
        present: true,
        host,
    })
}

#[derive(Clone, Copy, Debug)]
pub struct StageCall {
    pub entry: u32,
    pub args: [u32; 7],
}

pub struct EncodingScratch<'a> {
    pub units: &'a mut [u16],
    pub bytes: &'a mut [u8],
    pub positions: &'a mut [u32],
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct StageReturn {

    pub r0: Option<u32>,
}

fn capacity(required: usize, available: usize) -> Result<(), Error> {
    if required > available {
        Err(Error::Capacity {
            required,
            available,
        })
    } else {
        Ok(())
    }
}

pub fn encoding_stage(
    memory: &mut Memory<'_, '_>,
    call: StageCall,
    scratch: &mut EncodingScratch<'_>,
) -> Result<StageReturn, Error> {
    let a = call.args;
    let terminated = match call.entry {
        0x17bdc4 => false,
        0x17bff4 | 0x17bf70 => true,
        entry => return Err(Error::UnsupportedStage { entry }),
    };
    let count = if terminated {
        let mut count = 0;
        loop {
            capacity(count + 1, scratch.units.len())?;
            let unit = memory.unit(offset(a[0], count * 2)?)?;
            scratch.units[count] = unit;
            count += 1;
            if unit == 0 {
                break count;
            }
        }
    } else {
        let count = a[1] as usize;
        capacity(count, scratch.units.len())?;
        for (i, unit) in scratch.units[..count].iter_mut().enumerate() {
            *unit = memory.unit(offset(a[0], i * 2)?)?;
        }
        count
    };
    let input = &scratch.units[..count];
    if call.entry == 0x17bff4 {
        return Ok(StageReturn {
            r0: Some(text::encoded_len(input)),
        });
    }
    let (dst, cap, src_used, dst_used, map) = if terminated {
        (a[1], text::encoded_len(input) as usize, 0, 0, 0)
    } else {
        (
            a[3],
            (a[4] as usize).min(count.checked_mul(3).ok_or(Error::InputTooLong)?),
            a[2],
            a[5],
            a[6],
        )
    };
    capacity(cap, scratch.bytes.len())?;
    if map != 0 {
        capacity(cap, scratch.positions.len())?;
    }
    let ranges = [
        (a[0], count * 2),
        (dst, cap),
        (
            map,
            if map == 0 {
                0
            } else {
                cap.checked_mul(4).ok_or(Error::InputTooLong)?
            },
        ),
    ];
    for i in 0..ranges.len() {
        for j in 0..i {
            if overlaps(ranges[i].0, ranges[i].1, ranges[j].0, ranges[j].1) {
                return Err(Error::AliasedEncodingBuffers);
            }
        }
    }
    for counter in [src_used, dst_used] {
        if counter != 0 {
            memory.check(counter, 4)?;
            for &(address, bytes) in &ranges {
                if overlaps(counter, 4, address, bytes) {
                    return Err(Error::AliasedEncodingBuffers);
                }
            }
        }
    }
    let conversion = if terminated {

        let written = text::encode_terminated(input, &mut scratch.bytes[..cap]);
        text::Conversion {
            status: 0,
            consumed: count as u32,
            written,
        }
    } else {
        text::encode(
            input,
            &mut scratch.bytes[..cap],
            if map == 0 {
                None
            } else {
                Some(&mut scratch.positions[..cap])
            },
        )
    };
    let written = conversion.written as usize;
    let writes = written + usize::from(conversion.status != 0 && written < cap);
    memory.check(dst, writes)?;
    if map != 0 {
        memory.check(map, written * 4)?;
    }

    memory.write(dst, &scratch.bytes[..writes])?;
    if map != 0 {
        for (i, &value) in scratch.positions[..written].iter().enumerate() {
            memory.put_word(offset(map, i * 4)?, value)?;
        }
    }
    if src_used != 0 {
        memory.put_word(src_used, conversion.consumed)?;
    }
    if dst_used != 0 {
        memory.put_word(dst_used, conversion.written)?;
    }
    Ok(StageReturn {
        r0: Some(if terminated {
            conversion.written
        } else {
            conversion.status
        }),
    })
}

pub fn control_stage(
    memory: &mut Memory<'_, '_>,
    call: StageCall,
    host: &mut dyn MutexPlatform,
) -> Result<StageReturn, Error> {
    let a = call.args;
    match call.entry {
        0x18f24c | 0x15ae7c => {
            let pipeline = call.entry == 0x15ae7c;
            let (object, value, out, field) = if pipeline {
                (a[1], a[3], a[4], 44)
            } else {
                (a[0], a[2], a[3], 36)
            };
            let at = offset(object, field + 4)?;
            let mut state = memory.byte(at)?;
            let mut previous = 0;
            let mut lock = lock(memory, object, field, host)?;
            let rc = if call.entry == 0x18f24c {
                crate::frontend::enter_request(&mut state, value as u8, &mut previous, &mut lock)
            } else {
                crate::frontend::enter_pipeline(&mut state, value as u8, &mut previous, &mut lock)
            };
            memory.write(at, &[state])?;
            if previous != 0 {
                memory.write(out, &[previous])?;
            }
            Ok(StageReturn { r0: Some(rc) })
        }
        0x1959dc | 0x15af34 => {

            let (object, previous, field) = if call.entry == 0x15af34 {
                (a[1], a[2], 44)
            } else {
                (a[0], a[1], 36)
            };
            if previous != 0 {
                let at = offset(object, field + 4)?;
                let mut state = memory.byte(at)?;
                let mut lock = lock(memory, object, field, host)?;
                if call.entry == 0x1959dc {
                    crate::frontend::restore_request(&mut state, previous, &mut lock);
                } else {
                    crate::frontend::restore_pipeline(&mut state, previous, &mut lock);
                }
                memory.write(at, &[state])?;
            }
            Ok(StageReturn { r0: None })
        }
        0x1756d0 | 0x175944 => {
            if call.entry == 0x175944 && (a[0] == 0 || a[1] == 0) {
                memory.put_word(a[2], 0)?;
                return Ok(StageReturn { r0: Some(0x8b702007) });
            }
            if call.entry == 0x1756d0 && (a[0] == 0 || a[1] == 0 || a[2] == 0) {
                return Ok(StageReturn { r0: Some(0x8b702007) });
            }
            let mut key = [0u8; 128];
            let mut n = 0;
            while n < key.len() && memory.byte(a[1] + n as u32)? != 0 {
                key[n] = memory.byte(a[1] + n as u32)?; n += 1;
            }
            if n == key.len() { return Err(Error::InputTooLong); }
            let rc = if call.entry == 0x175944 {
                let (rc, value) = numeric_property(memory, a[0], &key[..n], false)?;
                memory.put_word(a[2], value)?; rc
            } else {
                memory.put_word(a[2], 0)?;
                if a[3] != 0 { memory.put_word(a[3], 0)?; }
                let (rc, value, len) = property(memory, a[0], &key[..n])?;
                if rc < 0x80000000 {
                    memory.put_word(a[2], value)?;
                    if a[3] != 0 { memory.put_word(a[3], len)?; }
                }
                rc
            };
            Ok(StageReturn { r0: Some(rc) })
        }
        0x1970a0 => {
            let mut words = [0; 6];
            if a[0] != 0 { for (i, w) in words.iter_mut().enumerate() { *w = memory.word(a[0] + i as u32 * 4)?; } }
            Ok(StageReturn { r0: Some(crate::frontend::word_at5(if a[0] == 0 { None } else { Some(&words) })) })
        }
        0x176fc0 => Ok(StageReturn { r0: Some(crate::frontend::match_halves(a[0], a[1], a[2], a[3])) }),
        0x176f94 => Ok(StageReturn { r0: Some(crate::normalization::quad_check(a[0], a[1], a[2], a[3])) }),
        0x176fd4 => {
            let mut value = [0; 8];
            memory.read(memory.word(0x176fe8)?, &mut value)?;
            let mut out = [0; 8];
            crate::frontend::load_pair(&mut out, &value);
            memory.write(a[0], &out)?;
            Ok(StageReturn { r0: None })
        }
        0x19611c => {
            let (rc, env) = resolve_environment(memory, [a[0], a[1]])?;
            memory.put_word(a[2], env)?;
            Ok(StageReturn { r0: Some(rc) })
        }
        entry => Err(Error::UnsupportedStage { entry }),
    }
}

pub(crate) fn resolve_environment(
    memory: &Memory<'_, '_>,
    handle: [u32; 2],
) -> Result<(u32, u32), Error> {
    resolve_environment_rom(memory, handle, crate::rom_region::Rom::Us)
}

pub(crate) fn resolve_environment_rom(
    memory: &Memory<'_, '_>,
    handle: [u32; 2],
    rom: crate::rom_region::Rom,
) -> Result<(u32, u32), Error> {
    use crate::rom_region::addr;
    let null = [memory.word(addr(rom, 0x8d7660))?, memory.word(addr(rom, 0x8d7664))?];
    let def = addr(rom, 0x8d7640);
    if handle == null {
        return Ok((0, def));
    }
    let rc = crate::frontend::validate_handle(handle[0] != 0, handle[1], 0x1e61, 32);
    if rc >= 0x80000000 {
        return Ok((if rc & 0x1fff == 8 { 0x81602008 } else { rc }, def));
    }
    if memory.unit(handle[0])? != 0x1e61 {
        return Ok((0x81602008, def));
    }
    Ok((0, handle[0]))
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RequestPrefix {
    pub environment: u32,
    pub stream: [u32; 2],
    pub previous_state: u8,
    pub blocked_at: Error,
}

pub fn request_prefix(
    memory: &mut Memory<'_, '_>,
    boot: BootState,
    request: u32,
    host: &mut dyn MutexPlatform,
) -> Result<RequestPrefix, Error> {
    request_prefix_rom(memory, boot, request, host, crate::rom_region::Rom::Us)
}

pub fn request_prefix_rom(
    memory: &mut Memory<'_, '_>,
    boot: BootState,
    request: u32,
    host: &mut dyn MutexPlatform,
    rom: crate::rom_region::Rom,
) -> Result<RequestPrefix, Error> {
    if boot.magic != 0x684f424a || boot.engine == 0 || memory.word(boot.engine)? != 0x734f424a {
        return Err(Error::EngineStatus(0x80000008));
    }
    let mut state = memory.byte(offset(boot.engine, 40)?)?;
    let mut previous = 0;
    let rc = crate::frontend::enter_request(
        &mut state,
        3,
        &mut previous,
        &mut lock(memory, boot.engine, 36, host)?,
    );
    memory.write(offset(boot.engine, 40)?, &[state])?;
    if rc >= 0x80000000 {
        return Err(Error::EngineStatus(crate::frontend::classify_code(rc)));
    }
    let status = if request == 0
        || memory.word(offset(request, 4)?)? == 0
        || memory.word(offset(request, 8)?)? == 0
    {
        0x80002007
    } else if memory.byte(request)? != 0 {
        0x80002300
    } else if memory.word(offset(boot.engine, 368)?)? == 0 {
        0x80000880
    } else {
        0
    };
    if status != 0 {
        crate::frontend::restore_request(
            &mut state,
            previous as u32,
            &mut lock(memory, boot.engine, 36, host)?,
        );
        memory.write(offset(boot.engine, 40)?, &[state])?;
        return Err(Error::EngineStatus(crate::frontend::classify_code(status)));
    }
    let stream = [
        memory.word(crate::rom_region::addr(rom, 0x8d7660))?,
        memory.word(crate::rom_region::addr(rom, 0x8d7664))?,
    ];
    let handle = [
        memory.word(offset(boot.engine, 8)?)?,
        memory.word(offset(boot.engine, 12)?)?,
    ];
    let (rc, environment) = resolve_environment_rom(memory, handle, rom)?;
    if rc >= 0x80000000 {
        crate::frontend::restore_request(
            &mut state,
            previous as u32,
            &mut lock(memory, boot.engine, 36, host)?,
        );
        memory.write(offset(boot.engine, 40)?, &[state])?;
        return Err(Error::EngineStatus(crate::frontend::classify_code(rc)));
    }
    Ok(RequestPrefix {
        environment,
        stream,
        previous_state: previous,
        blocked_at: Error::MissingTextStreamRuntime,
    })
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SchedulerBoundary {
    pub call: crate::frontend::PipelineCall,
    pub entry: u32,
    pub args: [u32; 5],
}

struct DispatchPrefix<'a, 'b, 'c> {
    memory: &'a Memory<'b, 'c>,
    object: u32,
    boundary: Option<SchedulerBoundary>,
    fault: Option<Error>,
}
impl crate::frontend::PipelineStages for DispatchPrefix<'_, '_, '_> {
    fn call(
        &mut self,
        call: crate::frontend::PipelineCall,
        _: &mut crate::frontend::FrontendState<'_>,
        _: &mut impl crate::frontend::SynthesisSink,
    ) -> crate::frontend::PipelineReply {
        use crate::frontend::{PipelineCall as C, PipelineReply};

        let blocked = PipelineReply {
            status: 0x80002001,
            value: 0,
            data: 0,
        };
        if self.boundary.is_some() || self.fault.is_some() {
            return blocked;
        }
        match call {
            C::BeginStage(_) | C::EndStage(_) => {
                let result = (|| {
                    let cs = self.memory.word(offset(self.object, 0x7c)?)?;
                    Ok::<_, Error>(self.memory.word(offset(cs, 16)?)?)
                })();
                match result {
                    Ok(handle) => {
                        let status = if matches!(call, C::BeginStage(_)) {
                            crate::frontend::begin_stage_hook((handle != 0).then_some(handle))
                        } else {
                            crate::frontend::end_stage_hook((handle != 0).then_some(handle))
                        };
                        return PipelineReply {
                            status,
                            value: 0,
                            data: 0,
                        };
                    }
                    Err(error) => self.fault = Some(error),
                }
            }
            C::Process(stage) => {
                let result = (|| {
                    let m = self.memory;
                    let cs = m.word(offset(self.object, 0x7c)?)?;
                    let components = m.word(offset(cs, 8)?)?;
                    let component = m.word(offset(components, stage as usize * 4)?)?;
                    let methods = m.word(offset(component, 8)?)?;
                    Ok::<_, Error>(SchedulerBoundary {
                        call,
                        entry: m.word(offset(methods, 0x34)?)?,
                        args: [
                            m.word(offset(component, 12)?)?,
                            m.word(offset(component, 16)?)?,
                            m.word(offset(self.object, 0xa0)?)?,
                            m.word(offset(self.object, 0xa4)?)?,
                            offset(m.word(offset(self.object, 0xdc)?)?, stage as usize)?,
                        ],
                    })
                })();
                match result {
                    Ok(boundary) => self.boundary = Some(boundary),
                    Err(error) => self.fault = Some(error),
                }
            }
            _ => {
                self.boundary = Some(SchedulerBoundary {
                    call,
                    entry: 0,
                    args: [0; 5],
                })
            }
        }
        blocked
    }
}
struct NoSynthesis;
impl crate::frontend::SynthesisSink for NoSynthesis {
    fn emit(&mut self, _: crate::frontend::SynthesisPayload<'_>) -> u32 {
        0x80002001
    }
}

pub fn scheduler_prefix(
    memory: &mut Memory<'_, '_>,
    object: u32,
    flush: u32,
    activity: &mut [u8],
) -> Result<SchedulerBoundary, Error> {
    use crate::frontend::{process_text, FrontendState};
    let cs = memory.word(offset(object, 0x7c)?)?;
    let count = memory.unit(offset(cs, 12)?)? as usize;
    capacity(count, activity.len())?;
    if count == 0 {
        return Err(Error::InvalidSnapshot);
    }
    let act = memory.word(offset(object, 0xdc)?)?;
    memory.read(act, &mut activity[..count])?;
    let stage = memory.unit(offset(object, 0xd8)?)?;
    if stage as usize >= count {
        return Err(Error::InvalidSnapshot);
    }
    let vt2 = memory.word(offset(object, 0x54)?)?;
    let mut state = FrontendState {
        activity: &mut activity[..count],
        stage,
        pending: memory.word(offset(object, 0xd4)?)?,
        loop_limit: memory.word(offset(object, 0x38)?)?,
        request_state: memory.byte(offset(object, 0x30)?)?,
        prepared_bytes: memory.word(offset(object, 0xf0)?)?,
        input_open: memory.word(offset(object, 0xac)?)? != 0,
        output_open: memory.word(offset(object, 0xc0)?)? != 0,
        instrumented: memory.word(offset(cs, 16)?)? != 0,
        observer: memory.word(offset(object, 0x20)?)? != 0,
        auxiliary_notifications: memory.word(offset(vt2, 0x64)?)? != 0,
    };
    let mut dispatch = DispatchPrefix {
        memory,
        object,
        boundary: None,
        fault: None,
    };
    let status = process_text(&mut state, flush, &mut dispatch, &mut NoSynthesis);
    if let Some(error) = dispatch.fault {
        return Err(error);
    }
    let boundary = dispatch.boundary.ok_or(Error::EngineStatus(status))?;
    if !matches!(boundary.call, crate::frontend::PipelineCall::Process(_)) {
        return Err(Error::UnsupportedStage {
            entry: boundary.entry,
        });
    }

    memory.put_word(offset(object, 0xd4)?, state.pending)?;
    memory.write(offset(object, 0xd8)?, &state.stage.to_le_bytes())?;
    memory.write(offset(object, 0x30)?, &[state.request_state])?;
    memory.put_word(offset(object, 0xf0)?, state.prepared_bytes)?;
    memory.write(act, state.activity)?;
    Ok(boundary)
}

fn pair(m: &Memory<'_, '_>, p: u32) -> Result<[u32; 2], Error> {
    Ok([m.word(p)?, m.word(p + 4)?])
}
fn put_half(m: &mut Memory<'_, '_>, p: u32, v: u16) -> Result<(), Error> {
    m.write(p, &v.to_le_bytes())
}
fn fill(m: &mut Memory<'_, '_>, p: u32, n: u32, v: u8) -> Result<(), Error> {
    m.check(p, n as usize)?;
    for i in 0..n {
        m.write(p + i, &[v])?;
    }
    Ok(())
}
fn move_guest(m: &mut Memory<'_, '_>, dst: u32, src: u32, n: u32) -> Result<(), Error> {
    m.check(dst, n as usize)?;
    m.check(src, n as usize)?;
    if dst > src {
        for i in (0..n).rev() {
            m.write(dst + i, &[m.byte(src + i)?])?;
        }
    } else {
        for i in 0..n {
            m.write(dst + i, &[m.byte(src + i)?])?;
        }
    }
    Ok(())
}
fn block_init(m: &mut Memory<'_, '_>, b: u32, tag: u16, n: u32) -> Result<(), Error> {
    put_half(m, b, tag)?;
    put_half(m, b + 2, 0)?;
    m.put_word(b + 4, n)?;
    m.put_word(b + 8, 0)?;
    m.put_word(b + 12, 0)
}
fn block_unlink(m: &mut Memory<'_, '_>, list: u32, b: u32) -> Result<(), Error> {
    let prev = m.word(b + 8)?;
    let next = m.word(b + 12)?;
    m.put_word(if prev == 0 { list } else { prev + 12 }, next)?;
    m.put_word(if next == 0 { list + 4 } else { next + 8 }, prev)
}
fn block_link(m: &mut Memory<'_, '_>, list: u32, prev: u32, b: u32) -> Result<(), Error> {
    let slot = if prev == 0 { list } else { prev + 12 };
    let next = m.word(slot)?;
    m.put_word(slot, b)?;
    m.put_word(b + 8, prev)?;
    m.put_word(b + 12, next)?;
    m.put_word(if next == 0 { list + 4 } else { next + 8 }, b)
}

fn norm_entry(raw: u32) -> u32 {
    crate::rom_region::to_us(crate::rom_region::Rom::from_entry(raw), raw)
}

fn mapped_allocate(
    m: &mut Memory<'_, '_>,
    wrapper: u32,
    size: u32,
    zero: bool,
) -> Result<u32, Error> {
    if wrapper == 0 || size == 0 {
        return Ok(0);
    }
    let vt = m.word(wrapper)?;
    let target = m.word(vt + if zero { 4 } else { 0 })?;
    if norm_entry(target) != if zero { 0x4a7758 } else { 0x4a779c } {
        return Err(Error::MissingStreamAdapter { entry: target });
    }
    let object = m.word(wrapper + 4)?;
    let methods = m.word(object)?;
    let target = m.word(methods + 8)?;
    if norm_entry(target) != 0x39f5e8 {
        return Err(Error::MissingStreamAdapter { entry: target });
    }
    let heap = m.word(object + 4)?;

    let owner = m.word(heap + 0x5c)?;
    if owner != 0 && owner != 0x1ff82000 {
        return Err(Error::MissingStreamAdapter { entry: 0x1840fc });
    }

    put_half(m, heap + 0x4c, 0)?;
    put_half(m, heap + 0x4e, m.unit(heap + 0x4e)? & !1)?;
    m.write(heap + 0x50, &[0])?;
    let list = heap + 0x3c;
    let n = size.wrapping_add(3) & !3;
    let mut block = m.word(list)?;
    let mut best = 0;
    let mut address = 0;
    let mut best_size = u32::MAX;
    while block != 0 {
        let available = m.word(block + 4)?;
        let start = block + 16;
        let aligned = start.wrapping_add(3) & !3;
        if available >= aligned - start + n && available < best_size {
            best = block;
            address = aligned;
            best_size = available;
            if m.unit(heap + 0x4e)? & 1 == 0 || available == n {
                break;
            }
        }
        block = m.word(block + 12)?;
    }
    if best == 0 {
        return Ok(0);
    }
    let start = best - (u32::from(m.unit(best + 2)?) << 17 >> 25);
    let end = best + 16 + m.word(best + 4)?;
    let prev = m.word(best + 8)?;
    block_unlink(m, list, best)?;
    let header = address - 16;
    let mut allocation_end = address + n;
    if end - allocation_end >= 20 {
        block_init(
            m,
            allocation_end,
            crate::heap::FREE,
            end - allocation_end - 16,
        )?;
        block_link(m, list, prev, allocation_end)?;
    } else {
        allocation_end = end;
    }
    if m.word(heap + 0x38)? & 1 != 0 {
        fill(m, start, (allocation_end - start) / 4 * 4, 0)?;
    }
    block_init(m, header, crate::heap::USED, allocation_end - header - 16)?;
    put_half(m, header + 2, ((header - start) as u16 & 0x7f) << 8)?;
    block_link(m, list + 8, m.word(list + 12)?, header)?;
    m.put_word(heap + 0x54, m.word(heap + 0x54)?.wrapping_add(1))?;
    if zero {
        fill(m, address, size, 0)?;
    }
    Ok(address)
}

fn mapped_free(m: &mut Memory<'_, '_>, wrapper: u32, pointer: u32) -> Result<(), Error> {
    if wrapper == 0 || pointer == 0 {
        return Ok(());
    }
    let target = m.word(m.word(wrapper)? + 12)?;
    if norm_entry(target) != 0x4a774c {
        return Err(Error::MissingStreamAdapter { entry: target });
    }
    let obj = m.word(wrapper + 4)?;
    let target = m.word(m.word(obj)? + 12)?;
    if norm_entry(target) != 0x39f5b8 {
        return Err(Error::MissingStreamAdapter { entry: target });
    }
    let heap = m.word(obj + 4)?;

    let owner = m.word(heap + 0x5c)?;
    if owner != 0 && owner != 0x1ff82000 {
        return Err(Error::MissingStreamAdapter { entry: 0x1840fc });
    }
    let list = heap + 0x3c;
    let b = pointer - 16;
    let mut end = pointer + m.word(b + 4)?;
    let mut start = b - (u32::from(m.unit(b + 2)?) << 17 >> 25);
    block_unlink(m, list + 8, b)?;
    let mut prev = 0;
    let mut current = m.word(list)?;
    while current != 0 && start > current {
        prev = current;
        current = m.word(current + 12)?;
    }
    if current != 0 && end == current {
        end = current + 16 + m.word(current + 4)?;
        block_unlink(m, list, current)?;
    }
    if prev != 0 && prev + 16 + m.word(prev + 4)? == start {
        start = prev;
        prev = m.word(prev + 8)?;
        block_unlink(m, list, start)?;
    }
    if end - start >= 16 {
        block_init(m, start, crate::heap::FREE, end - start - 16)?;
        block_link(m, list, prev, start)?;
    }
    m.put_word(heap + 0x54, m.word(heap + 0x54)?.wrapping_sub(1))
}

fn string_equal(m: &Memory<'_, '_>, p: u32, s: &[u8]) -> Result<bool, Error> {
    for (i, &b) in s.iter().enumerate() {
        if m.byte(p + i as u32)? != b {
            return Ok(false);
        }
    }
    Ok(m.byte(p + s.len() as u32)? == 0)
}
fn compare_key(m: &Memory<'_, '_>, key: &[u8], p: u32) -> Result<i32, Error> {
    for i in 0.. {
        let a = key.get(i).copied().unwrap_or(0);
        let b = m.byte(p + i as u32)?;
        if a != b {
            return Ok(i32::from(a) - i32::from(b));
        }
        if a == 0 {
            return Ok(0);
        }
    }
    unreachable!()
}

fn property(m: &Memory<'_, '_>, props: u32, key: &[u8]) -> Result<(u32, u32, u32), Error> {
    property_rom(m, props, key, crate::rom_region::Rom::Us)
}

fn property_rom(
    m: &Memory<'_, '_>,
    props: u32,
    key: &[u8],
    rom: crate::rom_region::Rom,
) -> Result<(u32, u32, u32), Error> {
    if props == 0 {
        return Ok((0x8b702007, 0, 0));
    }

    let lock = m.word(props + 4)?;
    if lock == 0 {
        return Ok((0x81402007, 0, 0));
    }
    let table = m.word(m.word(lock)? + 8)?;
    for i in [8, 12] {
        let entry = m.word(table + i)?;
        if entry != 0 {
            return Err(Error::MissingStreamAdapter { entry });
        }
    }
    let map = m.word(props + 8)?;
    let tree = map + 24;
    let comparator = m.word(m.word(tree + 36)? + 8)?;
    if crate::rom_region::to_us(rom, comparator) != 0x196980 {
        return Err(Error::MissingStreamAdapter { entry: comparator });
    }
    let mut node = m.word(tree)?;
    while node != tree {
        let c = compare_key(m, key, m.word(node + 16)?)?;
        if c == 0 {
            let ent = m.word(node + 20)?;
            let len = m.word(ent + 4)?;
            return Ok((0, if len == 0 { 0 } else { m.word(ent)? }, len));
        }
        node = m.word(node + if c < 0 { 0 } else { 4 })?;
    }
    Ok((0x8b702014, 0, 0))
}
fn numeric_property(
    m: &Memory<'_, '_>,
    props: u32,
    key: &[u8],
    signed: bool,
) -> Result<(u32, u32), Error> {
    numeric_property_rom(m, props, key, signed, crate::rom_region::Rom::Us)
}

fn numeric_property_rom(
    m: &Memory<'_, '_>,
    props: u32,
    key: &[u8],
    signed: bool,
    rom: crate::rom_region::Rom,
) -> Result<(u32, u32), Error> {
    let (rc, p, n) = property_rom(m, props, key, rom)?;
    if rc >= 0x80000000 || n <= 1 {
        return Ok((rc, 0));
    }
    let mut at = p;
    let mut v = 0u32;
    while matches!(m.byte(at)?, b' ' | b'\t' | b'\n' | b'\r') {
        at += 1;
    }
    let negative = signed && m.byte(at)? == b'-';
    if m.byte(at)? == b'+' || negative {
        at += 1;
    }
    while let b'0'..=b'9' = m.byte(at)? {
        v = v
            .wrapping_mul(10)
            .wrapping_add(u32::from(m.byte(at)? - b'0'));
        at += 1;
    }
    Ok((rc, if negative { v.wrapping_neg() } else { v }))
}

struct CreateStream<'a, 'b, 'c> {
    memory: &'a mut Memory<'b, 'c>,
    cfg: u32,
    fault: Option<Error>,
    rom: crate::rom_region::Rom,
}
impl CreateStream<'_, '_, '_> {
    fn result<T: Default>(&mut self, r: Result<T, Error>) -> T {
        match r {
            Ok(v) => v,
            Err(e) => {
                self.fault = Some(e);
                T::default()
            }
        }
    }
}
impl crate::frontend::StreamOpenHooks for CreateStream<'_, '_, '_> {
    fn resolve_ctx(&mut self, a: u32, b: u32, out: &mut u32) -> u32 {
        let r = resolve_environment(self.memory, [a, b]);
        let (rc, _) = self.result(r);
        *out = 16;
        rc
    }
    fn log_begin(&mut self, _: u32) {}
    fn log_error(&mut self, _: u32, _: u32) {}
    fn log_end(&mut self, _: u32, _: u32) {}
    fn alloc_cfg(&mut self, heap: u32) -> Option<u32> {
        let r = mapped_allocate(self.memory, heap, 32, true);
        self.cfg = self.result(r);
        (self.cfg != 0).then_some(48)
    }
    fn free_cfg(&mut self, heap: u32, _: u32) {
        let r = mapped_free(self.memory, heap, self.cfg);
        self.result(r);
    }
    fn query_u32(&mut self, p: u32, which: u32, out: &mut u32) -> u32 {
        let key: &[u8] = match which {
            0 => b"synthstreamminmemalloc",
            1 => b"synthstreamminmemmove",
            _ => b"synthstreamminmemfree",
        };
        let r = numeric_property(self.memory, p, key, false);
        let (rc, v) = self.result(r);
        *out = v;
        rc
    }
    fn query_log_input(&mut self, p: u32, out: &mut u32) {
        let r = numeric_property(self.memory, p, b"loginputtext", true);
        *out = self.result(r).1;
    }
    fn create_map(&mut self, heap: u32, out: &mut u32) -> u32 {
        let r = (|| {
            let map = mapped_allocate(self.memory, heap, 0x44, true)?;
            *out = map;
            if map == 0 {
                return Ok(0x8900200a);
            }
            for i in 0..5 {
                let value = self.memory.word(crate::rom_region::addr(self.rom, 0x8d7808) + i * 4)?;
                self.memory.put_word(map + i * 4, value)?;
            }
            self.memory.put_word(map + 20, heap)?;

            let t = map + 24;
            for field in [0, 4, 8] {
                self.memory.put_word(t + field, t)?;
            }
            self.memory.write(t + 12, &[1])?;
            self.memory.put_word(t + 24, 0)?;
            self.memory.put_word(t + 28, u32::MAX)?;
            self.memory.put_word(t + 32, heap)?;
            self.memory.put_word(t + 36, map)?;
            self.memory.write(t + 40, &[0])?;
            Ok(0)
        })();
        self.result(r)
    }
}

pub fn stream_create_stage(m: &mut Memory<'_, '_>, call: StageCall) -> Result<StageReturn, Error> {
    let a = call.args;
    if a[4] == 0 {
        return Ok(StageReturn {
            r0: Some(0x88f02006),
        });
    }
    let (rc, env) = resolve_environment(m, [a[2], a[3]])?;
    if rc >= 0x80000000 {
        m.put_word(a[4], 0)?;
        m.put_word(a[4] + 4, 0)?;
        return Ok(StageReturn { r0: Some(rc) });
    }
    let mut local = [0u8; 88];
    for i in [4, 16, 20] {
        local[16 + i..20 + i].copy_from_slice(&m.word(env + i as u32)?.to_le_bytes());
    }
    let mut hooks = CreateStream {
        memory: m,
        cfg: 0,
        fault: None,
        rom: crate::rom_region::Rom::Us,
    };
    let rc = crate::frontend::open_synth_stream(
        &mut hooks,
        &mut local,
        a[0],
        a[1],
        a[2],
        a[3],
        Some(80),
    );
    if let Some(e) = hooks.fault {
        return Err(e);
    }
    let cfg = hooks.cfg;
    if cfg != 0 && rc < 0x80000000 {
        hooks.memory.write(cfg, &local[48..80])?;
    }
    hooks
        .memory
        .put_word(a[4], if rc < 0x80000000 { cfg } else { 0 })?;
    hooks.memory.put_word(
        a[4] + 4,
        u32::from_le_bytes(local[84..88].try_into().unwrap()),
    )?;
    Ok(StageReturn { r0: Some(rc) })
}

fn new_writer(
    m: &mut Memory<'_, '_>,
    config: [u32; 2],
    id: u32,
    mime: u32,
    mode: u32,
    rom: crate::rom_region::Rom,
) -> Result<(u32, [u32; 2]), Error> {
    let rc = crate::frontend::validate_handle(config[0] != 0, config[1], 0x47dd, 32);
    if rc >= 0x80000000 {
        return Ok((0x81706008, [0, 0]));
    }
    let cfg = config[0];
    let (rc, env) = resolve_environment(m, pair(m, cfg)?)?;
    if rc >= 0x80000000 {
        return Ok((rc, [0, 0]));
    }
    let mut content = [0u8; 128];
    let mut len = 0;
    while len < 128 {
        let b = m.byte(mime + len as u32)?;
        if b == 0 {
            break;
        }
        content[len] = b.to_ascii_lowercase();
        len += 1;
    }
    if len == 0 || len >= 128 || !(mode == 1 || mode == 2) {
        return Ok((0x88f02007, [0, 0]));
    }
    let map = m.word(cfg + 12)?;
    let tree = map + 24;
    if m.word(tree)? != tree {
        return Err(Error::MissingStreamAdapter { entry: 0x196794 });
    }
    let heap = m.word(env + 4)?;
    let state = mapped_allocate(m, heap, 72, true)?;
    if state == 0 {
        return Ok((0x88f0200a, [0, 0]));
    }
    let readers = mapped_allocate(m, heap, 32, true)?;
    if readers == 0 {
        return Err(Error::EngineStatus(0x88f0200a));
    }
    for i in 0..3 {
        m.put_word(readers + 4 + i * 4, m.word(crate::rom_region::addr(rom, 0x8d781c) + i * 4)?)?;
    }
    put_half(m, readers + 24, 1)?;
    m.put_word(readers + 28, heap)?;
    let ct = mapped_allocate(m, heap, len as u32 + 1, false)?;
    if ct == 0 {
        return Err(Error::EngineStatus(0x88f0200a));
    }
    m.write(ct, &content[..=len])?;
    m.put_word(state, cfg)?;
    m.put_word(state + 4, env)?;
    m.put_word(state + 12, readers)?;
    m.put_word(state + 16, ct)?;
    put_half(m, state + 20, m.unit(cfg + 8)?)?;
    put_half(m, state + 22, 1)?;
    put_half(m, state + 24, id as u16)?;
    if &content[..len] == b"application/x-realspeak-markers-pp;version=4.0" {
        move_guest(m, state + 28, crate::rom_region::addr(rom, 0x8d77a8), 12)?;
    }
    m.write(state + 40, &[u8::from(mode == 2)])?;
    m.write(
        state + 41,
        &[u8::from(
            content[..len].starts_with(b"text/")
                && !content[..len].windows(15).any(|v| v == b"charset=wchar_t"),
        )],
    )?;
    if m.word(cfg + 28)? != 0
        && [
            b"text/plain;charset=utf-16".as_slice(),
            b"application/synthesis+ssml",
            b"text/xml",
        ]
        .contains(&&content[..len])
    {
        m.put_word(cfg + 28, 0)?;
        m.write(state + 40, &[1])?;
        m.write(state + 42, &[1])?;
    }

    let node = mapped_allocate(m, heap, 24, false)?;
    if node == 0 {
        return Err(Error::EngineStatus(0x8900200a));
    }
    put_half(m, state + 22, m.unit(state + 22)?.wrapping_add(1))?;
    for field in [0, 4, 8, 16] {
        m.put_word(node + field, 0)?;
    }
    m.write(node + 12, &[0])?;
    m.put_word(node + 20, state)?;
    let key = mapped_allocate(m, heap, 8, true)?;
    if key == 0 {
        return Err(Error::EngineStatus(0x88f0200a));
    }
    let key_ct = mapped_allocate(m, heap, len as u32 + 1, false)?;
    if key_ct == 0 {
        return Err(Error::EngineStatus(0x8900200a));
    }
    m.write(key_ct, &content[..=len])?;
    m.put_word(key, key_ct)?;
    put_half(m, key + 4, 0)?;
    m.put_word(node + 16, key)?;
    m.put_word(tree, node)?;
    for field in [0, 4, 8] {
        m.put_word(node + field, tree)?;
    }
    m.put_word(tree + 24, m.word(tree + 24)?.wrapping_add(1))?;
    m.write(node + 12, &[1])?;
    put_half(m, state + 22, m.unit(state + 22)?.wrapping_sub(1))?;
    let writer = mapped_allocate(m, heap, 12, true)?;
    if writer == 0 {
        return Err(Error::EngineStatus(0x88f0200a));
    }
    m.write(writer, &[mode as u8])?;
    put_half(m, writer + 2, id as u16)?;
    m.put_word(writer + 4, state)?;
    m.put_word(writer + 8, m.word(state + 48)?)?;
    put_half(m, state + 22, m.unit(state + 22)?.wrapping_add(1))?;
    m.put_word(state + 8, writer)?;
    Ok((0, [writer, 0x47de]))
}
fn stream_state(m: &Memory<'_, '_>, h: [u32; 2]) -> Result<u32, Error> {
    let rc = crate::frontend::validate_handle(h[0] != 0, h[1], 0x47de, 12);
    if rc >= 0x80000000 {
        return Err(Error::EngineStatus(rc));
    }
    m.word(h[0] + 4)
}
fn reserve_guest(m: &mut Memory<'_, '_>, h: [u32; 2], count: u32) -> Result<(u32, u32), Error> {
    let state = stream_state(m, h)?;
    if count == 0 || !matches!(m.byte(h[0])?, 1 | 2) {
        return Ok((0x88f02007, 0));
    }
    let write = m.word(state + 44)?;
    let base = m.word(state + 56)?;
    if write < base {
        return Ok((0x88f02005, 0));
    }
    let need = write - base + count;
    if m.word(state + 64)? < need {
        if m.word(state + 60)? != 0 {
            return Err(Error::MissingStreamAdapter { entry: 0x1963e0 });
        }
        let size = need.max(m.word(m.word(state)? + 16)?);
        let heap = m.word(m.word(state + 4)? + 4)?;
        let buf = mapped_allocate(m, heap, size, false)?;
        if buf == 0 {
            return Ok((0x88f0200a, 0));
        }
        m.put_word(state + 60, buf)?;
        m.put_word(state + 64, size)?;
    }
    let buf = m.word(state + 60)? + write - base;
    fill(m, buf, count, 0)?;
    Ok((0, buf))
}
fn commit_guest(m: &mut Memory<'_, '_>, h: [u32; 2], count: u32) -> Result<u32, Error> {
    let state = stream_state(m, h)?;
    if !matches!(m.byte(h[0])?, 1 | 2) {
        return Ok(0x88f02007);
    }
    let written = m.word(state + 44)?;
    let base = m.word(state + 56)?;
    if written < base {
        return Ok(0x88f02005);
    }
    let need = written - base + count;
    if m.word(state + 64)? < need {
        return Ok(0x88f02007);
    }
    if m.word(state + 32)? != 0 {
        return Err(Error::MissingStreamAdapter {
            entry: m.word(state + 32)?,
        });
    }
    m.put_word(state + 44, written.wrapping_add(count))?;
    if m.word(m.word(state)? + 24)? <= m.word(state + 64)?.wrapping_sub(need) {
        return Err(Error::MissingStreamAdapter { entry: 0x1963e0 });
    }
    compact_guest(m, state, 0)?;
    Ok(0)
}
fn compact_guest(m: &mut Memory<'_, '_>, state: u32, required: u32) -> Result<(), Error> {
    if m.word(state + 60)? == 0 || m.byte(state + 40)? != 0 {
        return Ok(());
    }
    let write = m.word(state + 44)?;
    let readers = m.word(state + 12)?;
    let mut minimum = write;
    if readers != 0 {
        let data = m.word(readers)?;
        let stride = u32::from(m.unit(readers + 4)?);
        for i in 0..m.word(readers + 20)? {
            minimum = minimum.min(m.word(m.word(data + i * stride)? + 8)?);
        }
    }
    let old = m.word(state + 56)?;
    let mut discarded = minimum.wrapping_sub(old);
    let stride = u32::from(m.unit(state + 28)?);
    if stride > 1 {
        discarded -= discarded % stride;
    }

    if m.word(state + 36)? != 0 && discarded != 0 {
        return Err(Error::MissingStreamAdapter {
            entry: m.word(state + 36)?,
        });
    }
    let cfg = m.word(state)?;
    if required == 0 {
        if m.word(cfg + 24)? > discarded {
            return Ok(());
        }
    } else if discarded < required && discarded < m.word(cfg + 20)? {
        return Ok(());
    }
    let buf = m.word(state + 60)?;
    let remaining = write.wrapping_sub(minimum);
    move_guest(m, buf, buf + discarded, remaining)?;
    m.put_word(state + 56, old.wrapping_add(discarded))?;
    if required == 0 {
        if m.word(state + 64)? == discarded || write == minimum {
            let heap = m.word(m.word(state + 4)? + 4)?;
            mapped_free(m, heap, buf)?;
            m.put_word(state + 60, 0)?;
            m.put_word(state + 64, 0)?;
        } else {
            return Err(Error::MissingStreamAdapter { entry: 0x1963e0 });
        }
    }
    Ok(())
}
fn close_guest(m: &mut Memory<'_, '_>, h: [u32; 2]) -> Result<u32, Error> {
    let state = stream_state(m, h)?;
    if m.byte(h[0])? == 0 {
        let list = m.word(state + 12)?;
        let data = m.word(list)?;
        let size = u32::from(m.unit(list + 4)?);
        let count = m.word(list + 20)?;
        for i in 0..count {
            if m.word(data + i * size)? == h[0] {
                if m.word(list + 12)? != 0 {
                    return Err(Error::MissingStreamAdapter {
                        entry: m.word(list + 12)?,
                    });
                }
                move_guest(
                    m,
                    data + i * size,
                    data + (i + 1) * size,
                    (count - i - 1) * size,
                )?;
                m.put_word(list + 20, count - 1)?;
                break;
            }
        }
    } else {
        m.put_word(state + 8, 0)?;
        m.write(state + 68, &[1])?;
    }
    if m.unit(state + 22)? <= 1 {
        return Err(Error::MissingStreamAdapter { entry: 0x14e0b4 });
    }
    put_half(m, state + 22, m.unit(state + 22)? - 1)?;
    let heap = m.word(m.word(state + 4)? + 4)?;
    mapped_free(m, heap, h[0])?;
    Ok(0)
}
fn create_live(m: &mut Memory<'_, '_>, h: [u32; 2]) -> Result<(u32, [u32; 2]), Error> {
    create_live_rom(m, h, crate::rom_region::Rom::Us)
}
fn create_live_rom(
    m: &mut Memory<'_, '_>,
    h: [u32; 2],
    rom: crate::rom_region::Rom,
) -> Result<(u32, [u32; 2]), Error> {
    let (rc, env) = resolve_environment(m, h)?;
    if rc >= 0x80000000 {
        return Ok((rc, [0, 0]));
    }
    let mut local = [0u8; 88];
    for i in [4usize, 16, 20] {
        local[16 + i..20 + i].copy_from_slice(&m.word(env + i as u32)?.to_le_bytes());
    }
    let mut hooks = CreateStream {
        memory: m,
        cfg: 0,
        fault: None,
        rom,
    };
    let rc = crate::frontend::open_synth_stream(&mut hooks, &mut local, 0, 0, h[0], h[1], Some(80));
    if let Some(e) = hooks.fault {
        return Err(e);
    }
    let cfg = hooks.cfg;
    if cfg != 0 && rc < 0x80000000 {
        hooks.memory.write(cfg, &local[48..80])?;
    }
    Ok((
        rc,
        [
            if rc < 0x80000000 { cfg } else { 0 },
            u32::from_le_bytes(local[84..88].try_into().unwrap()),
        ],
    ))
}
fn open_text_live(
    m: &mut Memory<'_, '_>,
    engine: u32,
    request: u32,
) -> Result<(u32, [u32; 2]), Error> {
    open_text_live_rom(m, engine, request, crate::rom_region::Rom::Us)
}

pub(crate) fn open_text_live_rom(
    m: &mut Memory<'_, '_>,
    engine: u32,
    request: u32,
    rom: crate::rom_region::Rom,
) -> Result<(u32, [u32; 2]), Error> {
    let interface = m.word(m.word(engine + 4)? + 0xd4)?;
    for (slot, expected) in [
        (16, 0x14e7a0),
        (20, 0x14ea44),
        (32, 0x14f010),
        (36, 0x14f918),
        (56, 0x150878),
        (60, 0x150c0c),
    ] {
        let entry = m.word(interface + slot)?;
        if crate::rom_region::to_us(rom, entry) != expected {
            return Err(Error::MissingStreamAdapter { entry });
        }
    }
    let handle = pair(m, engine + 8)?;
    let (rc, env) = resolve_environment(m, handle)?;
    if rc >= 0x80000000 {
        return Ok((rc, [0, 0]));
    }

    let (mut rc, config) = create_live_rom(m, handle, rom)?;

    if rc >= 0x80000000 {
        return Ok((rc, config));
    }
    let (mrc, mime, _) = property_rom(m, m.word(env + 20)?, b"inputmimetype", rom)?;
    if mrc >= 0x80000000 {
        return Err(Error::MissingStreamAdapter { entry: 0x175a1c });
    }
    let utf8 = string_equal(m, mime, b"text/plain;charset=utf-8")?;
    let (r, stream) = new_writer(m, config, 0xc0, mime, 2, rom)?;
    rc = r;
    if rc < 0x80000000 {
        let prefix = if m.byte(engine + 0x35)? == 2 {
            if utf8 {
                10
            } else {
                20
            }
        } else {
            0
        };
        let length = m.word(request + 4)?;
        let (r, buffer) = reserve_guest(m, stream, length.wrapping_add(prefix))?;
        rc = r;
        if rc < 0x80000000 {
            if prefix != 0 {
                move_guest(m, buffer, if utf8 { crate::rom_region::addr(rom, 0x8d2dc0) } else { crate::rom_region::addr(rom, 0x8d2dac) }, prefix)?;
            }
            move_guest(m, buffer + prefix, m.word(request + 8)?, length)?;
            m.put_word(engine + 0x14c, prefix)?;
            rc = commit_guest(m, stream, length.wrapping_add(prefix))?;
        }
    }
    if stream[0] != 0 {
        close_guest(m, stream)?;
    }
    if rc >= 0x80000000 {
        return Err(Error::EngineStatus(rc));
    }
    Ok((rc, config))
}

pub fn open_text_stage(m: &mut Memory<'_, '_>, call: StageCall) -> Result<StageReturn, Error> {
    let a = call.args;
    if a[0] == 0 || a[2] == 0 {
        return Ok(StageReturn {
            r0: Some(0x80000007),
        });
    }
    m.put_word(a[2], 0)?;
    m.put_word(a[2] + 4, 0)?;
    let (rc, h) = open_text_live(m, a[0], a[1])?;
    m.put_word(a[2], h[0])?;
    m.put_word(a[2] + 4, h[1])?;
    Ok(StageReturn { r0: Some(rc) })
}

fn available_guest(m: &Memory<'_, '_>, h: [u32; 2]) -> Result<(u32, u32, u8), Error> {
    let state = stream_state(m, h)?;
    if m.byte(h[0])? != 0 {
        return Ok((0x88f02007, 0, 1));
    }
    Ok((
        0,
        m.word(state + 44)?.wrapping_sub(m.word(h[0] + 8)?),
        m.byte(state + 68)?,
    ))
}
fn position_guest(m: &Memory<'_, '_>, h: [u32; 2]) -> Result<u32, Error> {
    let state = stream_state(m, h)?;
    m.word(if m.byte(h[0])? == 0 {
        h[0] + 8
    } else {
        state + 44
    })
}
fn read_guest(m: &Memory<'_, '_>, h: [u32; 2]) -> Result<(u32, u32, u32), Error> {
    let state = stream_state(m, h)?;
    if m.byte(h[0])? != 0 {
        return Ok((0x88f02007, 0, 0));
    }
    let pos = m.word(h[0] + 8)?;
    let base = m.word(state + 56)?;
    if pos < base {
        return Ok((0x88f02004, 0, 0));
    }
    let count = m.word(state + 44)?.wrapping_sub(pos);
    let buf = m.word(state + 60)?.wrapping_add(pos - base);
    if count != 0 && buf == 0 {
        return Ok((0x88f0200a, 0, 0));
    }
    Ok((0, buf, count))
}
fn consume_guest(m: &mut Memory<'_, '_>, h: [u32; 2], count: u32) -> Result<u32, Error> {
    let state = stream_state(m, h)?;
    if m.byte(h[0])? != 0 {
        return Ok(0x88f02007);
    }
    let pos = m.word(h[0] + 8)?;
    let available = m.word(state + 44)?.wrapping_sub(pos);
    let used = count.min(available);
    let next = pos.wrapping_add(used);
    m.put_word(h[0] + 8, next)?;
    m.put_word(state + 48, next.max(m.word(state + 48)?))?;
    compact_guest(m, state, 0)?;
    Ok(if count > available { 0x88f02007 } else { 0 })
}
fn merge_status(rc: &mut u32, next: u32) {
    if *rc < 0x80000000 && next >= 0x80000000 {
        *rc = next;
    }
}
fn xcoder_validate(m: &Memory<'_, '_>, h: [u32; 2], tag: u32, size: u32) -> Result<u32, Error> {
    let rc = crate::frontend::validate_handle(h[0] != 0, h[1], tag, size);
    if rc >= 0x80000000 {
        return Ok(if rc & 0x1fff == 8 { 0x88102008 } else { rc });
    }
    Ok(resolve_environment(m, pair(m, h[0])?)?.0)
}

pub fn xcoder_text_stage(
    m: &mut Memory<'_, '_>,
    call: StageCall,
    scratch: &mut [u8],
) -> Result<StageReturn, Error> {
    let a = call.args;
    let o = a[0];
    let mut rc = xcoder_validate(m, [o, a[1]], 0x354, 300)?;
    if rc >= 0x80000000 {
        return Ok(StageReturn { r0: Some(rc) });
    }
    m.write(a[4], &[1])?;
    let output = pair(m, o + 284)?;
    if output[0] == 0 {
        return Ok(StageReturn { r0: Some(rc) });
    }
    let input = pair(m, o + 276)?;
    let sync = pair(m, o + 292)?;
    let (r, count, final_) = available_guest(m, input)?;
    rc = r;
    if rc >= 0x80000000 {
        return Ok(StageReturn { r0: Some(rc) });
    }
    if count == 0 {
        if final_ == 0 {
            m.write(a[4], &[0])?;
        } else {
            rc = close_guest(m, output)?;
            let null = pair(m, 0x8d7660)?;
            m.put_word(o + 284, null[0])?;
            m.put_word(o + 288, null[1])?;
            merge_status(&mut rc, close_guest(m, sync)?);
            m.put_word(o + 292, null[0])?;
            m.put_word(o + 296, null[1])?;
        }
        return Ok(StageReturn { r0: Some(rc) });
    }
    let offset = position_guest(m, input)?;
    let (r, src, count) = read_guest(m, input)?;
    rc = r;
    if rc >= 0x80000000 {
        return Ok(StageReturn { r0: Some(rc) });
    }
    let (r, dst) = reserve_guest(m, output, count.wrapping_mul(2))?;
    rc = r;
    if rc >= 0x80000000 {
        return Ok(StageReturn { r0: Some(rc) });
    }
    let (r, map) = reserve_guest(m, sync, count.wrapping_mul(8))?;
    rc = r;
    if rc >= 0x80000000 {
        consume_guest(m, input, 0)?;
        return Ok(StageReturn { r0: Some(rc) });
    }
    if offset == 0
        && count >= 2
        && (string_equal(m, o + 8, b"UTF-16LE")? || string_equal(m, o + 8, b"UTF-16BE")?)
    {
        if m.byte(src)? == 0xfe && m.byte(src + 1)? == 0xff {
            m.write(o + 8, b"UTF-16BE\0")?;
            m.write(m.word(o + 268)?, &[0])?;
            m.put_word(o + 264, 0x88102001)?;
            return Ok(StageReturn {
                r0: Some(0x88102001),
            });
        }
        m.write(o + 8, b"UTF-16LE\0")?;
    }
    fill(m, map, count.wrapping_mul(8), 0xff)?;
    let flag = m.word(o + 268)?;
    let mut consumed = 0;
    let mut produced = 0;
    if m.byte(flag)? == 1 {
        let n = count as usize;
        let out = 4 + n + 4;
        let positions = out + n * 4;
        let used = positions + n * 8;
        let written = used + 4;
        capacity(written + 4, scratch.len())?;
        m.read(src, &mut scratch[4..4 + n + 1])?;

        m.read(dst, &mut scratch[out..out + n * 2])?;
        scratch[positions..positions + n * 8].fill(0xff);
        let mut extra = [0; 256];
        let mut mask = [0; 12];
        let mut base = [0; 24];
        m.read(0x8dc1f3, &mut extra)?;
        m.read(0x8dc1e0, &mut mask)?;
        m.read(0x8dc2f4, &mut base)?;
        rc = crate::normalization::decode_units(
            scratch,
            4,
            count,
            used as u32,
            out as u32,
            count * 2,
            written as u32,
            positions as u32,
            &crate::normalization::DecoderTables {
                extra: &extra,
                mask: &mask,
                mask_base: 6,
                base: &base,
                base_base: 0,
            },
        );
        consumed = u32::from_le_bytes(scratch[used..used + 4].try_into().unwrap());
        produced = u32::from_le_bytes(scratch[written..written + 4].try_into().unwrap()) * 2;

        m.write(dst, &scratch[out..out + n * 2])?;
        m.write(map, &scratch[positions..positions + n * 8])?;
    } else {
        rc = 0x8810200e;
    }
    if rc >= 0x80000000 && (final_ != 0 || rc & 0x1fff != 7) {
        produced = 0;
        consumed = 0;
    } else {
        rc = 0;
        if offset != 0 {
            for i in 0..produced {
                let p = map + i * 4;
                m.put_word(p, m.word(p)?.wrapping_add(offset))?;
            }
        }
    }
    merge_status(&mut rc, commit_guest(m, sync, produced * 4)?);
    merge_status(&mut rc, commit_guest(m, output, produced)?);
    merge_status(&mut rc, consume_guest(m, input, consumed)?);
    Ok(StageReturn { r0: Some(rc) })
}
fn translated(
    m: &Memory<'_, '_>,
    pos: u32,
    base: u32,
    sync: u32,
    count: u32,
) -> Result<u32, Error> {
    if base > pos {
        m.word(sync)
    } else if base.wrapping_add(count) <= pos {
        Ok(m.word(sync + count.wrapping_sub(1) * 4)?.wrapping_add(1))
    } else {
        m.word(sync + pos.wrapping_sub(base) * 4)
    }
}

pub fn xcoder_markers_stage(m: &mut Memory<'_, '_>, call: StageCall) -> Result<StageReturn, Error> {
    let a = call.args;
    let o = a[0];
    let mut rc = xcoder_validate(m, [o, a[1]], 0x356, 36)?;
    if rc >= 0x80000000 {
        return Ok(StageReturn { r0: Some(rc) });
    }
    m.write(a[4], &[1])?;
    let output = pair(m, o + 28)?;
    if output[0] == 0 {
        return Ok(StageReturn { r0: Some(0) });
    }
    let input = pair(m, o + 20)?;
    let sync_h = pair(m, o + 12)?;
    let (r, bytes, final_) = available_guest(m, input)?;
    rc = r;
    if rc >= 0x80000000 {
        return Ok(StageReturn { r0: Some(rc) });
    }
    if bytes == 0 {
        if final_ != 0 {
            rc = close_guest(m, output)?;
            let null = pair(m, 0x8d7660)?;
            m.put_word(o + 28, null[0])?;
            m.put_word(o + 32, null[1])?;
        }
        return Ok(StageReturn { r0: Some(rc) });
    }
    let (r, src, bytes) = read_guest(m, input)?;
    rc = r;
    if rc >= 0x80000000 {
        return Ok(StageReturn { r0: Some(rc) });
    }
    let base = position_guest(m, sync_h)? >> 2;
    let (r, sync, count) = read_guest(m, sync_h)?;
    rc = r;
    if rc >= 0x80000000 {
        return Ok(StageReturn { r0: Some(rc) });
    }
    let count = count >> 2;
    let (r, dst) = reserve_guest(m, output, bytes)?;
    rc = r;
    if rc >= 0x80000000 {
        consume_guest(m, input, 0)?;
        return Ok(StageReturn { r0: Some(rc) });
    }
    move_guest(m, dst, src, bytes)?;
    let mut max_position = 0;
    for i in 0..bytes / 24 {
        let at = src + i * 24;
        let to = dst + i * 24;
        if matches!(
            m.unit(at)?,
            1 | 4 | 8 | 0x10 | 0x12 | 0x13 | 0x14 | 0x16 | 0x18 | 0x1d | 0x4000
        ) {
            let pos = m.word(at + 4)?;
            let length = m.word(at + 8)?;
            let start = translated(m, pos, base, sync, count)?;
            let end = translated(m, pos.wrapping_add(length), base, sync, count)?;
            m.put_word(to + 4, start)?;
            m.put_word(to + 8, end.wrapping_sub(start))?;
            max_position = max_position.max(pos);
            rc = 0;
        }
    }
    if max_position >= count {
        max_position = count.wrapping_sub(1);
    }
    merge_status(&mut rc, commit_guest(m, output, bytes)?);
    merge_status(&mut rc, consume_guest(m, input, bytes)?);
    merge_status(
        &mut rc,
        consume_guest(
            m,
            sync_h,
            if max_position > base {
                (max_position - base) * 4
            } else {
                0
            },
        )?,
    );
    Ok(StageReturn { r0: Some(rc) })
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct VoiceBoundary {
    pub entry: u32,
    pub args: [u32; 7],
    pub event: Option<[u32; 3]>,
}

struct VoicePrefix<'a, 'b, 'c, 'd> {
    memory: &'a mut Memory<'b, 'c>,
    voice: u32,
    notification: Option<NotificationRegisters>,
    platform: Option<&'d mut dyn Platform>,
    host: &'d mut dyn MutexPlatform,
    boundary: Option<VoiceBoundary>,
    fault: Option<Error>,
    stamped: bool,
    rom: crate::rom_region::Rom,
}

impl VoicePrefix<'_, '_, '_, '_> {
    fn suspend(&mut self, entry: u32, args: [u32; 7], event: Option<[u32; 3]>) -> u32 {
        if self.boundary.is_none() && self.fault.is_none() {
            self.boundary = Some(VoiceBoundary { entry, args, event });
        }

        0x80002001
    }
    fn mutex(&mut self, acquire: bool) -> u32 {
        if self.boundary.is_some() || self.fault.is_some() {
            return 0x80002001;
        }
        match lock(self.memory, self.voice, 0x24, self.host) {
            Ok(mut l) => {
                use crate::frontend::StateLock;
                if acquire {
                    l.acquire()
                } else {
                    l.release()
                }
            }
            Err(e) => {
                self.fault = Some(e);
                0x80002001
            }
        }
    }
}
impl crate::frontend::StopVoiceHooks for VoicePrefix<'_, '_, '_, '_> {
    fn apply_mode(&mut self, _: u32) -> u32 {
        self.suspend(
            0x1900f8,
            [self.voice, 1, self.voice + 0x36, 0, 0, 0, 0],
            None,
        )
    }
    fn configure(&mut self, h0: u32, h1: u32, buf: u32, mode: u32) -> u32 {
        if self.boundary.is_some() || self.fault.is_some() {
            return 0x80002001;
        }
        let target = (|| {
            let interfaces = self.memory.word(self.voice + 4)?;
            self.memory
                .word(self.memory.word(interfaces + 0xbc)? + 0x60)
        })();
        match target {
            Ok(entry) => self.suspend(
                entry,
                [
                    h0,
                    h1,
                    if buf == 0 { 0 } else { self.voice + 0xc8 },
                    mode,
                    0,
                    0,
                    0,
                ],
                None,
            ),
            Err(e) => {
                self.fault = Some(e);
                0x80002001
            }
        }
    }
    fn timestamp(&mut self) -> (u32, u32) {
        self.stamped = true;

        match pair(self.memory, crate::rom_region::addr(self.rom, 0x8d7660)) {
            Ok([a, b]) => (a, b),
            Err(e) => {
                self.fault = Some(e);
                (0, 0)
            }
        }
    }
    fn start_voice(&mut self, _: u32, flag: u32) -> u32 {
        if self.boundary.is_some() || self.fault.is_some() {
            return 0x80002001;
        }
        let result = (|| {
            let callback = self.memory.word(self.voice + 0x170)?;
            if callback == 0 {
                return Ok(None);
            }
            let value = self.memory.word(self.voice + 0x16c)?;
            let user = self.memory.word(self.voice + 0x184)?;

            let callback_us = crate::rom_region::to_us(
                crate::rom_region::Rom::from_entry(callback),
                callback,
            );
            Ok(Some(if callback_us == 0x132124 {
                VoiceBoundary {
                    entry: 0x163a98,
                    args: [user, self.voice, 0x684f424a, value, 0, 0, 0],
                    event: Some([flag, 0, 0]),
                }
            } else {
                VoiceBoundary {
                    entry: callback,
                    args: [self.voice, 0x684f424a, value, 0, user, 0, 0],
                    event: Some([flag, 0, 0]),
                }
            }))
        })();
        match result {
            Ok(Some(b)) => {
                if b.entry == 0x163a98 && flag == 1 && self.notification.is_some() {
                    let rc = start_callback_live(
                        self.memory,
                        b.args[0],
                        b.args[1],
                        b.args[2],
                        self.notification.unwrap(),
                        self.platform.as_deref_mut().unwrap(),
                        self.rom,
                    );
                    match rc {
                        Ok(()) => 0,
                        Err(Error::MissingStreamAdapter { entry }) => {
                            self.suspend(entry, [0; 7], None)
                        }
                        Err(error) => {
                            self.fault = Some(error);
                            0x80002001
                        }
                    }
                } else {
                    self.suspend(b.entry, b.args, b.event)
                }
            }
            Ok(None) => 0,
            Err(e) => {
                self.fault = Some(e);
                0x80002001
            }
        }
    }
    fn acquire(&mut self, _: u32) -> u32 {
        self.mutex(true)
    }
    fn release_lock(&mut self, _: u32) -> u32 {
        self.mutex(false)
    }
    fn log_lock_error(&mut self, _: u32, _: u32) {}
    fn render(&mut self, h0: u32, h1: u32, b0: u32, b1: u32, n0: u32, n1: u32) -> u32 {
        let target = (|| {
            let interfaces = self.memory.word(self.voice + 4)?;
            self.memory
                .word(self.memory.word(interfaces + 0xbc)? + 0x34)
        })();
        match target {
            Ok(entry) => self.suspend(entry, [h0, h1, b0, b1, n0, n1, 0], None),
            Err(e) => {
                self.fault = Some(e);
                0x80002001
            }
        }
    }
    fn release_buffer(&mut self, b0: u32, b1: u32) -> u32 {
        if self.boundary.is_some() || self.fault.is_some() {
            return 0x80002001;
        }
        let target = (|| {
            let interfaces = self.memory.word(self.voice + 4)?;
            self.memory
                .word(self.memory.word(interfaces + 0xd4)? + 0x14)
        })();
        match target {
            Ok(entry) => self.suspend(entry, [b0, b1, 0, 0, 0, 0, 0], None),
            Err(e) => {
                self.fault = Some(e);
                0x80002001
            }
        }
    }
}

pub fn voice_start_prefix(
    memory: &mut Memory<'_, '_>,
    call: StageCall,
    host: &mut dyn MutexPlatform,
) -> Result<VoiceBoundary, Error> {
    voice_start_inner(memory, call, host, None, None, crate::rom_region::Rom::Us)
}

pub(crate) fn voice_start_inner<'h>(
    memory: &mut Memory<'_, '_>,
    call: StageCall,
    host: &'h mut dyn MutexPlatform,
    notification: Option<NotificationRegisters>,
    platform: Option<&'h mut dyn Platform>,
    rom: crate::rom_region::Rom,
) -> Result<VoiceBoundary, Error> {
    if call.entry != 0x190410 {
        return Err(Error::UnsupportedStage { entry: call.entry });
    }
    let [voice, mode, b0, b1, ..] = call.args;
    let mut local = [0u8; 0x1a0];
    memory.read(voice, &mut local[..0x16c])?;
    let context = memory.word(voice + 0x10)?;
    let log = memory.word(context + 0x10)?;
    local[0x10..0x14].copy_from_slice(&0x180u32.to_le_bytes());
    local[0x190..0x194].copy_from_slice(&log.to_le_bytes());
    let mut hooks = VoicePrefix {
        memory,
        voice,
        host,
        notification,
        platform,
        boundary: None,
        fault: None,
        stamped: false,
        rom,
    };
    let status = crate::frontend::stop_voice(&mut hooks, &mut local, 0, mode, b0, b1);
    let boundary = hooks.boundary;
    let stamped = hooks.stamped;
    if let Some(error) = hooks.fault {
        return Err(error);
    }
    let boundary = boundary.ok_or(Error::EngineStatus(status))?;
    if boundary.entry == 0x1900f8 {

        memory.write(voice + 0x148, &[mode as u8])?;
    }
    if stamped {
        memory.write(voice + 0x154, &local[0x154..0x16c])?;
    }
    Ok(boundary)
}

pub fn request_voice_prefix(
    memory: &mut Memory<'_, '_>,
    boot: BootState,
    request: u32,
    host: &mut dyn MutexPlatform,
) -> Result<(RequestPrefix, VoiceBoundary), Error> {
    let mut prefix = request_stream_prefix(memory, boot, request, host)?;
    let boundary = voice_start_prefix(
        memory,
        StageCall {
            entry: 0x190410,
            args: [boot.engine, 0, prefix.stream[0], prefix.stream[1], 0, 0, 0],
        },
        host,
    )?;
    prefix.blocked_at = Error::MissingStreamAdapter {
        entry: boundary.entry,
    };
    Ok((prefix, boundary))
}

pub fn request_stream_prefix(
    m: &mut Memory<'_, '_>,
    boot: BootState,
    request: u32,
    host: &mut dyn MutexPlatform,
) -> Result<RequestPrefix, Error> {
    request_stream_prefix_rom(m, boot, request, host, crate::rom_region::Rom::Us)
}

pub fn request_stream_prefix_rom(
    m: &mut Memory<'_, '_>,
    boot: BootState,
    request: u32,
    host: &mut dyn MutexPlatform,
    rom: crate::rom_region::Rom,
) -> Result<RequestPrefix, Error> {
    let mut prefix = request_prefix_rom(m, boot, request, host, rom)?;
    let (rc, h) = open_text_live_rom(m, boot.engine, request, rom)?;
    if rc >= 0x80000000 {
        return Err(Error::EngineStatus(rc));
    }
    prefix.stream = h;
    prefix.blocked_at = Error::MissingStreamAdapter { entry: 0x190410 };
    Ok(prefix)
}

pub fn request_restore_rom(
    memory: &mut Memory<'_, '_>,
    engine_addr: u32,
    previous: u32,
    host: &mut dyn MutexPlatform,
) -> Result<(), Error> {
    if previous != 0 {
        let at = offset(engine_addr, 40)?;
        let mut state = memory.byte(at)?;
        let mut guard = lock(memory, engine_addr, 36, host)?;
        crate::frontend::restore_request(&mut state, previous, &mut guard);
        memory.write(at, &[state])?;
    }
    Ok(())
}

pub fn synthesis_restore_rom(
    memory: &mut Memory<'_, '_>,
    seg: u32,
    previous: u32,
    host: &mut dyn MutexPlatform,
) -> Result<(), Error> {
    if previous != 0 {
        let mut state = memory.byte(seg.wrapping_add(0x30))?;
        let mut guard = lock(memory, seg, 44, host)?;
        crate::frontend::restore_pipeline(&mut state, previous, &mut guard);
        memory.write(seg.wrapping_add(0x30), &[state])?;
    }
    Ok(())
}
fn release_state_guest(m: &mut Memory<'_, '_>, state: u32) -> Result<(), Error> {
    if state == 0 {
        return Ok(());
    }
    let refs = m.unit(state + 22)?;
    if refs > 1 {
        return put_half(m, state + 22, refs - 1);
    }
    if m.byte(state + 42)? != 0 {
        return Err(Error::MissingStreamAdapter { entry: 0x173b00 });
    }
    let heap = m.word(m.word(state + 4)? + 4)?;
    mapped_free(m, heap, m.word(state + 16)?)?;
    let readers = m.word(state + 12)?;
    if readers != 0 {
        if m.word(readers + 12)? != 0 && m.word(readers + 20)? != 0 {
            return Err(Error::MissingStreamAdapter {
                entry: m.word(readers + 12)?,
            });
        }
        mapped_free(m, heap, m.word(readers)?)?;
        mapped_free(m, heap, readers)?;
    }
    let buffer = m.word(state + 60)?;
    if buffer != 0 {
        if m.word(state + 36)? != 0 {
            return Err(Error::MissingStreamAdapter {
                entry: m.word(state + 36)?,
            });
        }
        mapped_free(m, heap, buffer)?;
    }
    mapped_free(m, heap, state)
}
fn tree_first(m: &Memory<'_, '_>, tree: u32) -> Result<u32, Error> {
    let mut n = m.word(tree)?;
    if n == tree {
        return Ok(0);
    }
    while m.word(n)? != tree {
        n = m.word(n)?;
    }
    Ok(n)
}
fn tree_next(m: &Memory<'_, '_>, tree: u32, mut n: u32) -> Result<u32, Error> {
    if m.word(n + 4)? != tree {
        n = m.word(n + 4)?;
        while m.word(n)? != tree {
            n = m.word(n)?;
        }
        return Ok(n);
    }
    let mut parent = m.word(n + 8)?;
    while parent != tree && m.word(parent + 4)? == n {
        n = parent;
        parent = m.word(parent + 8)?;
    }
    Ok(if parent == tree { 0 } else { parent })
}

pub fn stream_destroy_stage(m: &mut Memory<'_, '_>, call: StageCall) -> Result<StageReturn, Error> {
    let a = call.args;
    let rc = crate::frontend::validate_handle(a[0] != 0, a[1], 0x47dd, 32);
    if rc >= 0x80000000 {
        return Ok(StageReturn { r0: Some(rc) });
    }
    let cfg = a[0];
    let (rc, env) = resolve_environment(m, pair(m, cfg)?)?;
    if rc >= 0x80000000 {
        return Ok(StageReturn { r0: Some(rc) });
    }
    let heap = m.word(env + 4)?;
    let map = m.word(cfg + 12)?;
    let tree = map + 24;
    let iterator = mapped_allocate(m, heap, 12, true)?;
    if iterator == 0 {
        return Ok(StageReturn {
            r0: Some(0x8900200a),
        });
    }
    m.put_word(iterator, map)?;
    m.put_word(iterator + 4, 0)?;
    m.put_word(iterator + 8, tree_first(m, tree)?)?;
    loop {
        let node = m.word(iterator + 8)?;
        if node == 0 {
            break;
        }
        m.put_word(iterator + 4, node)?;
        m.put_word(iterator + 8, tree_next(m, tree, node)?)?;
        let state = m.word(node + 20)?;
        let writer = m.word(state + 8)?;
        if writer != 0 {
            mapped_free(m, heap, writer)?;
            m.put_word(state + 8, 0)?;
        }
        let readers = m.word(state + 12)?;
        for i in 0..m.word(readers + 20)? {
            mapped_free(
                m,
                heap,
                m.word(m.word(readers)? + i * u32::from(m.unit(readers + 4)?))?,
            )?;
        }
        if m.unit(state + 22)? > 1 {
            put_half(m, state + 22, 1)?;
        }
    }
    mapped_free(m, heap, iterator)?;

    let mut stack = [(0u32, false); 128];
    let mut used = 0;
    let root = m.word(tree)?;
    if root != tree {
        stack[0] = (root, false);
        used = 1;
    }
    while used != 0 {
        used -= 1;
        let (node, visited) = stack[used];
        if !visited {
            capacity(used + 3, stack.len())?;
            stack[used] = (node, true);
            used += 1;
            for field in [4, 0] {
                let child = m.word(node + field)?;
                if child != tree {
                    stack[used] = (child, false);
                    used += 1;
                }
            }
        } else {
            let key = m.word(node + 16)?;
            if key != 0 {
                mapped_free(m, heap, m.word(key)?)?;
                mapped_free(m, heap, key)?;
            }
            release_state_guest(m, m.word(node + 20)?)?;
            mapped_free(m, heap, node)?;
        }
    }
    m.put_word(tree + 24, 0)?;
    m.put_word(tree, tree)?;
    m.put_word(tree + 4, tree)?;
    mapped_free(m, heap, map)?;
    mapped_free(m, heap, cfg)?;
    Ok(StageReturn { r0: Some(0) })
}

pub fn text_lifecycle_stage(m: &mut Memory<'_, '_>, call: StageCall) -> Result<StageReturn, Error> {
    let result = open_text_stage(m, call)?;
    if result.r0.unwrap_or(0) >= 0x80000000 {
        return Ok(result);
    }
    let h = pair(m, call.args[2])?;
    stream_destroy_stage(
        m,
        StageCall {
            entry: 0x14ea44,
            args: [h[0], h[1], 0, 0, 0, 0, 0],
        },
    )
}

pub fn backend_stage(
    m: &mut Memory<'_, '_>,
    call: StageCall,
    tables: &crate::tables::NumericTables<'_>,
) -> Result<StageReturn, Error> {
    match call.entry {
        0x1be020 => crate::engine_backend::reset(m, call.args[0], tables)?,
        0x3912fc | 0x3912ec | 0x39133c | 0x39131c | 0x39132c => crate::engine_backend::set_voice(
            m,
            call.entry,
            call.args[0],
            call.args[1] as i32,
            tables,
        )?,
        entry => return Err(Error::UnsupportedStage { entry }),
    }
    Ok(StageReturn { r0: None })
}

pub(crate) struct LiveComponentCalls;
impl crate::engine_components::InternalCalls for LiveComponentCalls {
    fn call(&mut self, m: &mut Memory<'_, '_>, entry: u32, a: &[u32]) -> Result<u32, Error> {
        match entry {
            0x19611c => {
                let (rc, env) = resolve_environment(m, [a[0], a[1]])?;
                m.put_word(a[2], env)?;
                Ok(rc)
            }
            0x196394 => mapped_allocate(m, a[0], a[1], false),
            0x1963b8 => mapped_allocate(m, a[0], a[1].wrapping_mul(a[2]), true),
            0x196428 => {
                mapped_free(m, a[0], a[1])?;
                Ok(0)
            }
            0x176fd4 => {
                let null = pair(m, 0x8d7660)?;
                m.put_word(a[0], null[0])?;
                m.put_word(a[0] + 4, null[1])?;
                Ok(0)
            }
            0x15140c => {
                let (rc, n, final_) = available_guest(m, [a[0], a[1]])?;
                m.put_word(a[2], n)?;
                if a[3] != 0 {
                    m.write(a[3], &[final_])?;
                }
                Ok(rc)
            }
            0x151294 => {
                let p = position_guest(m, [a[0], a[1]])?;
                m.put_word(a[2], p)?;
                Ok(0)
            }
            0x14fe9c => {
                let (rc, p, n) = read_guest(m, [a[0], a[1]])?;
                m.put_word(a[2], p)?;
                m.put_word(a[3], n)?;
                Ok(rc)
            }
            0x15017c => consume_guest(m, [a[0], a[1]], a[2]),
            0x150878 => {
                let (rc, p) = reserve_guest(m, [a[0], a[1]], a[3])?;
                m.put_word(a[2], p)?;
                Ok(rc)
            }
            0x150c0c => commit_guest(m, [a[0], a[1]], a[2]),
            0x14f918 => close_guest(m, [a[0], a[1]]),
            _ => crate::engine_components::InternalCalls::call(
                &mut crate::engine_components2::ReadCalls,
                m,
                entry,
                a,
            ),
        }
    }
}

pub fn component_stage(
    m: &mut Memory<'_, '_>,
    call: StageCall,
    scratch: u32,
) -> Result<StageReturn, Error> {
    let a = call.args;
    let args = [a[0], a[1], a[2], a[3], a[4]];
    let result = if matches!(call.entry, 0x1d3314 | 0x21a25c) {
        crate::engine_components2::process(m, call.entry, args, scratch, &mut LiveComponentCalls)
    } else {
        crate::engine_components::process(m, call.entry, args, scratch, &mut LiveComponentCalls)
    };
    result.map(|rc| StageReturn { r0: Some(rc) })
}

use crate::voice::{VoiceEvent, VoiceQueue, VoiceState, VoiceTransition};
fn live_word(b: &[u8], at: usize) -> u32 {
    u32::from_le_bytes(b[at..at + 4].try_into().unwrap())
}
fn live_put(b: &mut [u8], at: usize, v: u32) {
    b[at..at + 4].copy_from_slice(&v.to_le_bytes());
}
const LIVE_VOICE_FLOAT_OFFSETS: [usize; 22] = [
    8, 0x5c, 0x6c, 0x70, 0x74, 0x78, 0x80, 0x84, 0x88, 0x8c, 0x90, 0x94, 0x98, 0x9c, 0xa0, 0xa4,
    0xb0, 0xc0, 0xc4, 0xc8, 0xcc, 0xd0,
];
fn live_queue(bytes: &[u8]) -> VoiceQueue {
    assert!(live_word(bytes, 0) < 256 && live_word(bytes, 4) < 256);
    VoiceQueue {
        read: live_word(bytes, 0) as u8,
        write: live_word(bytes, 4) as u8,
        events: core::array::from_fn(|i| VoiceEvent {
            kind: live_word(bytes, 8 + i * 20) as i32,
            value: f32::from_bits(live_word(bytes, 12 + i * 20)),
            target: f32::from_bits(live_word(bytes, 16 + i * 20)),
            duration: f32::from_bits(live_word(bytes, 20 + i * 20)),
            delay: live_word(bytes, 24 + i * 20) as i32,
        }),
    }
}
fn live_write_queue(q: &VoiceQueue, bytes: &mut [u8]) {
    live_put(bytes, 0, q.read as u32);
    live_put(bytes, 4, q.write as u32);
    for (i, e) in q.events.iter().enumerate() {
        for (j, value) in [
            e.kind as u32,
            e.value.to_bits(),
            e.target.to_bits(),
            e.duration.to_bits(),
            e.delay as u32,
        ]
        .iter()
        .enumerate()
        {
            live_put(bytes, 8 + i * 20 + j * 4, *value);
        }
    }
}
fn live_voice(bytes: &[u8]) -> VoiceState {
    let f = |o| f32::from_bits(live_word(bytes, o));
    VoiceState {
        sample_rate: f(8),
        enabled: bytes[0x48] != 0,
        position: f(0x5c),
        time: live_word(bytes, 0x64) as i32,
        value6c: f(0x6c),
        value70: f(0x70),
        gain: f(0x74),
        rate: f(0x78),
        period: f(0x80),
        value84: f(0x84),
        value88: f(0x88),
        base8c: f(0x8c),
        coefficient90: f(0x90),
        target94: f(0x94),
        base98: f(0x98),
        coefficient9c: f(0x9c),
        targeta0: f(0xa0),
        valuea4: f(0xa4),
        durationa8: live_word(bytes, 0xa8) as i32,
        valueb0: f(0xb0),
        durationb4: live_word(bytes, 0xb4) as i32,
        active: core::array::from_fn(|i| bytes[0xbc + i] != 0),
        targets: core::array::from_fn(|i| f(0xc0 + i * 4)),
        phases: core::array::from_fn(|i| f(0xcc + i * 4)),
        offsetd8: live_word(bytes, 0xd8) as i32,
        offsetdc: live_word(bytes, 0xdc) as i32,
        flag: bytes[0xe0] != 0,
        transitions: core::array::from_fn(|i| VoiceTransition {
            kind: live_word(bytes, 0x28f8 + i * 20) as i32,
            reciprocal: f(0x28fc + i * 20),
            remaining: live_word(bytes, 0x2908 + i * 20) as i32,
        }),
    }
}
fn live_write_voice(s: &VoiceState, bytes: &mut [u8]) {
    let floats = [
        s.sample_rate,
        s.position,
        s.value6c,
        s.value70,
        s.gain,
        s.rate,
        s.period,
        s.value84,
        s.value88,
        s.base8c,
        s.coefficient90,
        s.target94,
        s.base98,
        s.coefficient9c,
        s.targeta0,
        s.valuea4,
        s.valueb0,
        s.targets[0],
        s.targets[1],
        s.targets[2],
        s.phases[0],
        s.phases[1],
    ];
    for (o, f) in LIVE_VOICE_FLOAT_OFFSETS.into_iter().zip(floats) {
        live_put(bytes, o, f.to_bits());
    }
    live_put(bytes, 0xd4, s.phases[2].to_bits());
    for (o, v) in [
        (0x64, s.time),
        (0xa8, s.durationa8),
        (0xb4, s.durationb4),
        (0xd8, s.offsetd8),
        (0xdc, s.offsetdc),
    ] {
        live_put(bytes, o, v as u32);
    }
    bytes[0x48] = s.enabled as u8;
    bytes[0xe0] = s.flag as u8;
    for i in 0..3 {
        bytes[0xbc + i] = s.active[i] as u8;
    }
    for (i, t) in s.transitions.iter().enumerate() {
        live_put(bytes, 0x28f8 + i * 20, t.kind as u32);
        live_put(bytes, 0x28fc + i * 20, t.reciprocal.to_bits());
        live_put(bytes, 0x2908 + i * 20, t.remaining as u32);
    }
}

pub fn reset_synthesis_live(
    m: &mut Memory<'_, '_>,
    engine: u32,
    rom: crate::rom_region::Rom,
) -> Result<(), Error> {
    let mut raw = [0u8; 0x2920];
    m.read(engine, &mut raw)?;
    for at in [0xe8, 0x14f0] {
        if live_word(&raw, at) >= 256 || live_word(&raw, at + 4) >= 256 {
            return Err(Error::InvalidSnapshot);
        }
    }
    let mut voice = live_voice(&raw);
    let mut input = live_queue(&raw[0xe8..0x14f0]);
    let mut commands = live_queue(&raw[0x14f0..0x28f8]);
    if raw[4] != 0 {
        let mut constants = [0; crate::tables::NumericTables::CONSTANT_BYTES];
        let mut cosine = [0; crate::tables::NumericTables::COSINE_BYTES];
        m.read(crate::rom_region::addr(rom, 0xa21fb0), &mut constants)?;
        m.read(crate::rom_region::addr(rom, 0x8cbfe0), &mut cosine)?;
        let tables = crate::tables::NumericTables::from_bytes(&constants, &cosine).unwrap();
        while input.read != input.write {
            let elapsed = input.events[input.read as usize].delay;
            voice.consume(&mut input, Some(&mut commands), elapsed, &tables);
        }
        while commands.read != commands.write {
            let elapsed = commands.events[commands.read as usize].delay;
            voice.consume(&mut commands, None, elapsed, &tables);
        }
        live_write_voice(&voice, &mut raw);
        live_write_queue(&input, &mut raw[0xe8..0x14f0]);
        live_write_queue(&commands, &mut raw[0x14f0..0x28f8]);
        m.write(engine, &raw)?;

        let amount =
            (f32::from_bits(m.word(engine + 0x58)?) + m.word(engine + 0x68)? as i32 as f32) as i32;
        let amount = amount.min(m.word(engine + 0x40)? as i32).max(0) as u32;
        fill(
            m,
            m.word(engine + 0x3c)?,
            amount.checked_mul(4).ok_or(Error::InvalidSnapshot)?,
            0,
        )?;
        for at in [
            0x54, 0x58, 0x5c, 0x60, 0x64, 0x68, 0xcc, 0xd0, 0xd4, 0xd8, 0xdc, 0xac, 0xb8,
        ] {
            m.put_word(engine + at, 0)?;
        }
        m.put_word(engine + 0x78, 1.0f32.to_bits())?;
        m.put_word(engine + 0x2908, 0x80000000)?;
        m.put_word(engine + 0x291c, 0x80000000)?;
        let cursor = m.word(engine + 0x44)?;
        let mut local = [0u8; 28];
        m.read(cursor, &mut local)?;
        crate::boot::Memory {
            base: cursor,
            bytes: &mut local,
        }
        .reset_cursor(cursor);
        m.write(cursor, &local)?;
    }
    for at in [0x4bc8, 0x4bcc, 0x381c, 0x3820, 0x3824] {
        m.put_word(engine + at, 0)?;
    }
    m.write(engine + 0x4c30, &[0])?;

    let mut queue_bytes = [0u8; 0x1408];
    m.read(engine + 0xe8, &mut queue_bytes)?;
    let mut queue = live_queue(&queue_bytes);
    queue.enqueue_flag(m.word(engine + 0x64)? as i32, 0, 0);
    live_write_queue(&queue, &mut queue_bytes);
    m.write(engine + 0xe8, &queue_bytes)?;

    let mut frame = crate::queue::SpeechFrame::default();
    frame.reset_for_output();
    let p = engine + 0x3828;
    m.write(p + 4, &[frame.started as u8])?;
    for (at, value) in [
        (8, frame.start_value as u32),
        (12, frame.boundary_value as u32),
        (20, frame.command_count as u32),
        (2076, frame.audio.count as u32),
        (2080, frame.audio.written as u32),
        (2084, frame.audio.pending as u32),
        (2088, frame.character_count as u32),
        (2860, frame.group_count as u32),
        (3888, frame.output_count as u32),
        (4916, frame.current_time as u32),
        (4920, frame.before_time as u32),
        (4924, frame.after_time as u32),
        (4992, frame.pause_count as u32),
        (4996, frame.pause_total as u32),
        (5000, frame.position),
    ] {
        m.put_word(p + at, value)?;
    }
    for i in 0..128 {
        m.put_word(p + 1052 + i as u32 * 4, frame.audio.offsets[i] as u32)?;
        m.put_word(p + 1564 + i as u32 * 4, frame.audio.lengths[i] as u32)?;
    }
    for i in 0..frame.pauses.len() {
        m.put_word(p + 4928 + i as u32 * 4, frame.pauses[i] as u32)?;
    }
    m.put_word(engine + 0x4bb4, u32::MAX)?;
    m.put_word(engine + 0x4bb8, u32::MAX)?;
    m.write(engine + 0x4bbc, &[0])?;
    put_half(m, engine + 0x4c38, 0x23)?;
    m.put_word(engine + 0x4c6c, 0)?;

    let mut floats = [[0f32; crate::buffers::VOICE_DELAY_CAPACITY]; 3];
    let mut headers = [0u32; 3];
    let mut sizes = [0usize; 3];
    for i in 0..3 {
        headers[i] = m.word(engine + 0x31d8 + i as u32 * 4)?;
        sizes[i] = m.word(headers[i] + 8)? as usize;
        capacity(sizes[i], floats[i].len())?;
    }
    let [a, b, c] = &mut floats;
    let mut filter = crate::buffers::VoiceFilter {
        rings: [
            crate::buffers::RingBuffer {
                samples: &mut a[..sizes[0]],
                position: 0,
            },
            crate::buffers::RingBuffer {
                samples: &mut b[..sizes[1]],
                position: 0,
            },
            crate::buffers::RingBuffer {
                samples: &mut c[..sizes[2]],
                position: 0,
            },
        ],
        history: 0.0,
    };
    filter.reset();
    for i in 0..3 {
        let data = m.word(headers[i] + 4)?;
        for (j, v) in filter.rings[i].samples.iter().enumerate() {
            m.put_word(data + j as u32 * 4, v.to_bits())?;
        }
        m.put_word(headers[i] + 12, filter.rings[i].position)?;
    }
    m.put_word(engine + 0x31f0, filter.history.to_bits())?;
    let header = m.word(engine + 0x3210)?;
    let size = m.word(header + 8)? as usize;
    let mut samples = [0u16; crate::buffers::SAMPLE_DELAY_CAPACITY];
    capacity(size, samples.len())?;
    let mut filter = crate::buffers::SampleFilter {
        ring: crate::buffers::RingBuffer {
            samples: &mut samples[..size],
            position: 0,
        },
        history: 0.0,
    };
    filter.reset();
    let data = m.word(header + 4)?;
    for (i, v) in filter.ring.samples.iter().enumerate() {
        put_half(m, data + i as u32 * 2, *v)?;
    }
    m.put_word(header + 12, filter.ring.position)?;
    m.put_word(engine + 0x3214, filter.history.to_bits())?;
    Ok(())
}

#[derive(Clone, Copy, Debug, Default)]
pub struct NotificationRegisters {
    pub s16: u32,
    pub s17: u32,
}

pub fn notify_synthesis_live(
    m: &mut Memory<'_, '_>,
    speech: u32,
    event: u32,
    registers: NotificationRegisters,
    host: &mut (impl Platform + ?Sized),
) -> Result<(), Error> {
    let tick = host.ticks();
    let vector = speech + 0x61cc;
    let begin = m.word(vector + 4)?;
    let end = m.word(vector + 8)?;
    let capacity = m.word(vector + 12)?;
    let count = end.wrapping_sub(begin) as i32 >> 5;
    let reserved = capacity & 0x3fffffff;
    if reserved as i32 <= count {
        let grown = if reserved == 0 {
            1
        } else {
            reserved.wrapping_mul(2)
        };
        if (reserved as i32) < grown as i32 {
            if m.word(vector)? == 0 || capacity & 0xc0000000 != 0x40000000 {
                return Ok(());
            }
            let allocator = m.word(vector)?;
            return Err(Error::MissingStreamAdapter {
                entry: m.word(m.word(allocator)? + 8)?,
            });
        }
    }
    if end != 0 {
        for (i, value) in [
            event,
            registers.s16,
            tick as u32,
            (tick >> 32) as u32,
            0,
            0,
            0,
            registers.s17,
        ]
        .into_iter()
        .enumerate()
        {
            m.put_word(end + i as u32 * 4, value)?;
        }
    }
    m.put_word(vector + 8, end.wrapping_add(32))
}

pub fn request_callback_prefix(
    m: &mut Memory<'_, '_>,
    boot: BootState,
    request: u32,
    host: &mut dyn MutexPlatform,
    registers: NotificationRegisters,
    platform: &mut impl Platform,
) -> Result<(RequestPrefix, VoiceBoundary), Error> {
    request_callback_prefix_rom(m, boot, request, host, registers, platform, crate::rom_region::Rom::Us)
}

#[allow(clippy::too_many_arguments)]
pub fn request_callback_prefix_rom(
    m: &mut Memory<'_, '_>,
    boot: BootState,
    request: u32,
    host: &mut dyn MutexPlatform,
    registers: NotificationRegisters,
    platform: &mut impl Platform,
    rom: crate::rom_region::Rom,
) -> Result<(RequestPrefix, VoiceBoundary), Error> {
    let mut prefix = request_stream_prefix_rom(m, boot, request, host, rom)?;
    let boundary = voice_start_inner(
        m,
        StageCall {
            entry: 0x190410,
            args: [boot.engine, 0, prefix.stream[0], prefix.stream[1], 0, 0, 0],
        },
        host,
        Some(registers),
        Some(platform),
        rom,
    )?;
    prefix.blocked_at = Error::MissingStreamAdapter {
        entry: boundary.entry,
    };
    Ok((prefix, boundary))
}

fn start_callback_live(
    m: &mut Memory<'_, '_>,
    speech: u32,
    a: u32,
    b: u32,
    registers: NotificationRegisters,
    platform: &mut dyn Platform,
    rom: crate::rom_region::Rom,
) -> Result<(), Error> {
    use crate::frontend_output::handoff as cb;
    struct Host<'a, 'b, 'c, 'd> {
        memory: &'a mut Memory<'b, 'c>,
        platform: &'d mut dyn Platform,
        registers: NotificationRegisters,
        fault: Option<Error>,
        rom: crate::rom_region::Rom,
    }
    impl cb::CallbackHost for Host<'_, '_, '_, '_> {
        fn notify(&mut self, speech: u32, event: i32, _: &mut [u8]) {
            if self.fault.is_none() {
                self.fault = notify_synthesis_live(
                    self.memory,
                    speech,
                    event as u32,
                    self.registers,
                    self.platform,
                )
                .err();
            }
        }
        fn reset(&mut self, engine: u32) {
            if self.fault.is_none() {
                self.fault = reset_synthesis_live(self.memory, engine, self.rom).err();
            }
        }

        fn finalize(&mut self, _: u32, _: u32, _: u32) {
            self.fault = Some(Error::MissingStreamAdapter { entry: 0x183398 });
        }
        fn poll_ready(&mut self, _: u32) -> bool {
            unreachable!("event-1 host")
        }
        fn sleep(&mut self, _: u32, _: u32) {
            unreachable!("event-1 host")
        }
        fn synthesize(
            &mut self,
            _: u32,
            _: u32,
            _: u32,
            _: u32,
            _: &mut cb::SynthResult,
            _: u32,
            _: &mut [i16],
            _: u32,
            _: &mut [i16],
        ) -> (u32, u32) {
            unreachable!("event-1 host")
        }
        fn publish(&mut self, _: &[i16]) {
            unreachable!("event-1 host")
        }
        fn update(&mut self, _: u32) -> u32 {
            unreachable!("event-1 host")
        }
        fn lock(&mut self, _: u32) {
            unreachable!("event-1 host")
        }
        fn unlock(&mut self, _: u32) {
            unreachable!("event-1 host")
        }
        fn ready(&mut self, _: u32) -> bool {
            unreachable!("event-1 host")
        }
    }
    let mut view = cb::CallbackSpeech {
        address: speech,
        tail: cb::CallbackTail {
            engine: m.word(speech + 0x61b8)?,
            ..Default::default()
        },
        ..Default::default()
    };
    let mut host = Host {
        memory: m,
        platform,
        registers,
        fault: None,
        rom,
    };
    cb::handle_callback(
        &mut view,
        a,
        b,
        10_000_000,
        &cb::CallbackEvent { kind: 1, result: 0 },
        &mut cb::SynthResult::default(),
        &mut [0; 64],
        &mut [],
        &mut [],
        &mut host,
    );
    if let Some(error) = host.fault {
        Err(error)
    } else {
        Ok(())
    }
}

pub fn synthesis_start_prefix(
    m: &mut Memory<'_, '_>,
    call: VoiceBoundary,
    platform: &mut dyn MutexPlatform,
) -> Result<VoiceBoundary, Error> {
    synthesis_start_prefix_rom(m, call, platform, crate::rom_region::Rom::Us)
}

#[allow(clippy::too_many_arguments)]
pub fn synthesis_start_prefix_rom(
    m: &mut Memory<'_, '_>,
    call: VoiceBoundary,
    platform: &mut dyn MutexPlatform,
    rom: crate::rom_region::Rom,
) -> Result<VoiceBoundary, Error> {
    if crate::rom_region::to_us(rom, call.entry) != 0x15961c {
        return Err(Error::UnsupportedStage { entry: call.entry });
    }
    struct Hooks<'a, 'b, 'c, 'd> {
        m: &'a mut Memory<'b, 'c>,
        platform: &'d mut dyn MutexPlatform,
        env: u32,
        seg: u32,
        fault: Option<Error>,
        boundary: Option<VoiceBoundary>,
    }
    impl Hooks<'_, '_, '_, '_> {
        fn fail(&mut self, entry: u32, args: [u32; 7]) -> u32 {
            if self.boundary.is_none() && self.fault.is_none() {
                self.boundary = Some(VoiceBoundary {
                    entry,
                    args,
                    event: None,
                });
            }
            0x80002001
        }
    }
    impl crate::frontend::SegmentScoreHooks for Hooks<'_, '_, '_, '_> {
        fn score_segment(
            &mut self,
            _: u32,
            c: u32,
            d: u32,
            e: u32,
            f: u32,
            g: u32,
            _: u32,
            _: u32,
            _: u32,
            _: u32,
        ) -> u32 {
            self.fail(0x15668c, [self.env, self.seg, c, d, e, f, g])
        }
        fn refresh_object(&mut self) -> u32 {
            unreachable!("suspended scorer")
        }
        fn resolve_count(&mut self, _: u32, _: u32, _: &mut u32) {
            unreachable!("suspended scorer")
        }
        fn combine_outputs(&mut self, _: u32, _: u32) -> u32 {
            unreachable!("suspended scorer")
        }
    }
    impl crate::frontend::SynthesizeHooks for Hooks<'_, '_, '_, '_> {
        fn resolve_engine(&mut self, a: u32, b: u32, ctx: &mut u32, seg: &mut u32) -> u32 {
            let r = (|| {
                let rc = crate::normalization::quad_check(a, b, 0xcdac, 332);
                if rc >= 0x80000000 {
                    return Ok(if rc & 0x1fff == 8 { 0x81202008 } else { rc });
                }
                let (rc, env) = resolve_environment(self.m, pair(self.m, a)?)?;
                if rc < 0x80000000 {
                    self.env = env;
                    self.seg = a;
                    *ctx = 16;
                    *seg = 64;
                }
                Ok(rc)
            })();
            match r {
                Ok(rc) => rc,
                Err(error) => {
                    self.fault = Some(error);
                    0x80002001
                }
            }
        }
        fn log_begin(&mut self, _: u32) {}
        fn log_error(&mut self, _: u32, _: u32) {}
        fn log_end(&mut self, _: u32, _: u32) {}
        fn begin_synth(&mut self, previous: &mut u8) -> u32 {
            let r = (|| {
                let mut state = self.m.byte(self.seg + 0x30)?;
                let rc = crate::frontend::enter_pipeline(
                    &mut state,
                    3,
                    previous,
                    &mut lock(self.m, self.seg, 44, self.platform)?,
                );
                self.m.write(self.seg + 0x30, &[state])?;
                Ok(rc)
            })();
            match r {
                Ok(rc) => rc,
                Err(error) => {
                    self.fault = Some(error);
                    0x80002001
                }
            }
        }
        fn end_synth(&mut self, _: u8) {

        }
        fn teardown_handle(&mut self, a: u32, b: u32) {
            self.fail(0x19862c, [a, b, 0, 0, 0, 0, 0]);
        }
        fn reset_handle(&mut self, _: u32, _: u32) {}
        fn release_handle(&mut self, _: u32, _: u32) {}
        fn report_synthesis(&mut self, _: u32, _: u32, _: u32, _: u32, _: u32, _: u32) {}
        fn score(
            &mut self,
            a: u32,
            c: u32,
            d: u32,
            e: u32,
            f: u32,
            g: u32,
            i0: u32,
            i1: u32,
            o0: u32,
            o1: u32,
            _: &mut u32,
            _: &mut u32,
        ) -> u32 {
            if self.boundary.is_some() || self.fault.is_some() {
                return 0x80002001;
            }
            crate::frontend::score_segment(
                self,
                &mut [0; 8],
                0,
                a,
                c,
                d,
                e,
                f,
                g,
                i0,
                i1,
                o0,
                o1,
                0,
                4,
            )
        }
    }
    let seg = call.args[0];
    let (rc, env) = resolve_environment(m, pair(m, seg)?)?;
    if rc >= 0x80000000 {
        return Err(Error::EngineStatus(rc));
    }
    let mut local = [0u8; 0x240];
    live_put(&mut local, 16 + 0x10, m.word(env + 0x10)?);
    for at in [0x14, 0x1c, 0x28] {
        live_put(&mut local, 64 + at, m.word(seg + at as u32)?);
    }
    let trace = m.word(seg + 0x7c)?;
    if trace != 0 {
        live_put(&mut local, 64 + 0x7c, 0x200);
        for at in [0x10, 0x14, 0x18] {
            live_put(&mut local, 0x200 + at, m.word(trace + at as u32)?);
        }
    }
    let mut hooks = Hooks {
        m,
        platform,
        env: 0,
        seg: 0,
        fault: None,
        boundary: None,
    };
    let status = crate::frontend::synthesize_ttseg(
        &mut hooks,
        &mut local,
        call.args[0],
        call.args[1],
        call.args[2],
        call.args[3],
        call.args[4],
        call.args[5],
    );
    if let Some(error) = hooks.fault {
        return Err(error);
    }
    hooks.boundary.ok_or(Error::EngineStatus(status))
}

pub trait HeapPlatform {
    fn thread_id(&mut self) -> u32;
    fn arbiter_address(&self) -> u32;
    fn arbitrate(&mut self, memory: &mut crate::boot::Memory<'_>, args: &[u32]) -> u32;
}
struct BridgePlatform<'a>(&'a mut dyn HeapPlatform);
impl crate::bridge::Platform for BridgePlatform<'_> {
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

pub fn synthesis_allocate_prefix(
    m: &mut Memory<'_, '_>,
    call: VoiceBoundary,
    workspace: &mut [u8],
    platform: &mut dyn HeapPlatform,
) -> Result<VoiceBoundary, Error> {
    synthesis_allocate_prefix_rom(m, call, workspace, platform, crate::rom_region::Rom::Us)
}

#[allow(clippy::too_many_arguments)]
pub fn synthesis_allocate_prefix_rom(
    m: &mut Memory<'_, '_>,
    call: VoiceBoundary,
    workspace: &mut [u8],
    platform: &mut dyn HeapPlatform,
    rom: crate::rom_region::Rom,
) -> Result<VoiceBoundary, Error> {
    if call.entry != 0x15668c {
        return Err(Error::UnsupportedStage { entry: call.entry });
    }
    let [ctx, synth, src0, src1, _, _, _] = call.args;
    let kind = m.byte(synth + 0x69)?;
    if kind > 2 || (kind <= 1 && !matches!(m.byte(synth + 0x6a)?, 1 | 2)) {
        return Err(Error::EngineStatus(0x81203d06));
    }
    let out = synth + 0x84;
    fill(m, out, 188, 0)?;
    m.put_word(out, ctx)?;
    m.put_word(out + 52, src0)?;
    m.put_word(out + 56, src1)?;
    m.write(out + 76, &[1])?;
    let count = u32::from(m.unit(m.word(synth + 0x7c)? + 12)?);
    let heap = m.word(ctx + 4)?;
    let mut host = BridgePlatform(platform);
    let mut flat = crate::bridge::Flat::new(m, workspace, &mut host, heap)?;
    let extra = flat.fn_1963b8(heap, count, 1);
    flat.finish()?;
    m.put_word(out + 88, extra)?;
    if extra == 0 {
        return Err(Error::EngineStatus(0x8120200a));
    }
    put_half(m, out + 92, count as u16)?;
    let params = m.word(ctx + 20)?;
    let mut key = [0; 128];
    let mut len = 0;
    let key_base = crate::rom_region::addr(rom, 0x155598);
    while len < key.len() {
        let b = m.byte(key_base + len as u32)?;
        if b == 0 {
            break;
        }
        key[len] = b;
        len += 1;
    }
    if len == key.len() {
        return Err(Error::InputTooLong);
    }
    let (rc, frequency) = numeric_property_rom(m, params, &key[..len], false, rom)?;
    if rc >= 0x80000000 {
        m.put_word(out + 4, 0)?;
    } else {
        m.put_word(out + 4, frequency)?;
        let (rc, bits) = numeric_property_rom(m, params, b"audiooutputsamplebits", false, rom)?;
        m.put_word(out + 8, if rc >= 0x80000000 { 16 } else { bits })?;
    }
    Ok(VoiceBoundary {
        entry: 0x173fb4,
        args: [m.word(ctx + 16)?, 3, crate::rom_region::addr(rom, 0x155ea8), 0, 0, 0, 0],
        event: None,
    })
}

pub fn logging_live(
    m: &mut Memory<'_, '_>,
    a: [u32; 7],
    workspace: &mut [u8],
    allocator_workspace: &mut [u8],
    platform: &mut dyn HeapPlatform,
) -> Result<(), Error> {
    if a[0] == 0 {
        return Ok(());
    }
    m.check(a[4], 4)?;
    m.check(a[5], 20)?;
    let required = m
        .regions
        .iter()
        .map(|r| r.base as usize + r.bytes.len())
        .max()
        .unwrap_or(0);
    capacity(required, workspace.len())?;
    capacity(required, allocator_workspace.len())?;
    struct Logger<'a> {
        scratch: &'a mut [u8],
        platform: &'a mut dyn HeapPlatform,
        fault: Option<Error>,
    }
    impl crate::boot_part_b::Host for Logger<'_> {
        fn thread_id(&mut self) -> u32 {
            self.platform.thread_id()
        }
        fn call(&mut self, b: &mut crate::boot::Memory<'_>, entry: u32, a: &[u32]) -> u32 {
            if self.fault.is_some() {
                return 0;
            }
            let run = (|| {
                if !matches!(entry, 0x196394 | 0x1963b8 | 0x196428) {
                    return Err(Error::MissingStreamAdapter { entry });
                }
                let mut regions = [Region {
                    base: b.base,
                    bytes: &mut *b.bytes,
                }];
                let mut memory = Memory::new(&mut regions)?;
                let mut os = BridgePlatform(self.platform);
                let mut heap = crate::bridge::Flat::new(&mut memory, self.scratch, &mut os, a[0])?;
                let value = match entry {
                    0x196394 => heap.fn_196394(a[0], a[1]),
                    0x1963b8 => heap.fn_1963b8(a[0], a[1], a[2]),
                    _ => {
                        heap.fn_196428(a[0], a[1]);
                        0
                    }
                };
                heap.finish()?;
                Ok(value)
            })();
            match run {
                Ok(v) => v,
                Err(e) => {
                    self.fault = Some(e);
                    0
                }
            }
        }
    }
    impl crate::boot_part_b::FormatHost for Logger<'_> {}

    if m.regions.len() == 1 && m.regions[0].base == 0 && !m.regions[0].bytes.is_empty() {
        let mut logger = Logger {
            scratch: allocator_workspace,
            platform,
            fault: None,
        };
        let mut view = crate::boot::Memory {
            base: 0,
            bytes: &mut *m.regions[0].bytes,
        };

        view.put_word(a[4], a[3]);
        crate::boot_part_b::fn_173fb4(&mut view, a[0], a[1], a[2], a[4], a[5], a[6], &mut logger);
        if let Some(e) = logger.fault {
            return Err(e);
        }
        return Ok(());
    }
    let flat = &mut workspace[..required];
    flat.fill(0);
    for r in m.regions.iter() {
        flat[r.base as usize..r.base as usize + r.bytes.len()].copy_from_slice(r.bytes);
    }
    let mut logger = Logger {
        scratch: allocator_workspace,
        platform,
        fault: None,
    };
    let mut view = crate::boot::Memory {
        base: 0,
        bytes: flat,
    };

    view.put_word(a[4], a[3]);
    crate::boot_part_b::fn_173fb4(&mut view, a[0], a[1], a[2], a[4], a[5], a[6], &mut logger);
    if let Some(e) = logger.fault {
        return Err(e);
    }

    let mut end = 0usize;
    loop {
        let next = m
            .regions
            .iter()
            .filter(|r| !r.bytes.is_empty() && r.base as usize >= end)
            .min_by_key(|r| r.base);
        let Some(r) = next else { break };
        if let Some(i) = view.bytes[end..r.base as usize]
            .iter()
            .position(|b| *b != 0)
        {
            return Err(Error::UnmappedMemory {
                address: (end + i) as u32,
                bytes: 1,
            });
        }
        end = r.base as usize + r.bytes.len();
    }
    for r in m.regions.iter_mut() {
        r.bytes
            .copy_from_slice(&view.bytes[r.base as usize..r.base as usize + r.bytes.len()]);
    }
    Ok(())
}

pub fn synthesis_logging_prefix(
    m: &mut Memory<'_, '_>,
    synthesis: VoiceBoundary,
    logger: VoiceBoundary,
    request_sp: u32,
    workspace: &mut [u8],
    scratch: &mut [u8],
    platform: &mut dyn HeapPlatform,
) -> Result<VoiceBoundary, Error> {
    synthesis_logging_prefix_rom(m, synthesis, logger, request_sp, workspace, scratch, platform, crate::rom_region::Rom::Us)
}

#[allow(clippy::too_many_arguments)]
pub fn synthesis_logging_prefix_rom(
    m: &mut Memory<'_, '_>,
    synthesis: VoiceBoundary,
    logger: VoiceBoundary,
    request_sp: u32,
    workspace: &mut [u8],
    scratch: &mut [u8],
    platform: &mut dyn HeapPlatform,
    rom: crate::rom_region::Rom,
) -> Result<VoiceBoundary, Error> {
    if synthesis.entry != 0x15668c || logger.entry != 0x173fb4 {
        return Err(Error::UnsupportedStage {
            entry: logger.entry,
        });
    }
    let ctx = synthesis.args[0];
    let synth = synthesis.args[1];
    let props = m.word(ctx + 20)?;
    let alloc_base = crate::rom_region::addr(rom, 0x15668c);
    let empty = m.word(alloc_base + 0x988)?;
    let mut values = [0; 6];
    let keys: [&[u8]; 6] = [b"", b"", b"", b"voicemodel", b"voice", b"language"];
    for i in 0..6 {
        let mut key = [0; 128];
        let mut n = 0;
        let bytes = if i < 3 {
            let address = crate::rom_region::addr(rom, [0x155a5c, 0x155e3c, 0x155e4c][i]);
            while n < key.len() && m.byte(address + n as u32)? != 0 {
                key[n] = m.byte(address + n as u32)?;
                n += 1;
            }
            if n == key.len() {
                return Err(Error::InputTooLong);
            }
            &key[..n]
        } else {
            keys[i]
        };
        let (rc, p, _) = property_rom(m, props, bytes, rom)?;
        values[i] = if rc >= 0x80000000 || p == 0 { empty } else { p };
    }
    let sp = request_sp
        .checked_sub(0x1f8)
        .ok_or(Error::InvalidSnapshot)?;
    let va = sp.checked_sub(4).ok_or(Error::InvalidSnapshot)?;
    let params = sp.checked_sub(76).ok_or(Error::InvalidSnapshot)?;
    m.check(va, 64)?;
    m.check(params, 20)?;
    let required = m.regions.iter().map(|r| r.base as usize + r.bytes.len()).max().unwrap_or(0);
    capacity(required, workspace.len())?;
    capacity(required, scratch.len())?;
    let base = crate::rom_region::addr(rom, 0x15668c);
    let args = [
        base + 0x4ac,
        values[5],
        base + 0x4a4,
        values[4],
        base + 0x49c,
        values[3],
        base + 0x494,
        m.word(synth + 0x88)?,
        base + 0x48c,
        m.word(synth + 16)? + 30,
        base + 0x484,
        values[2],
        base + 0x47c,
        values[1],
        base + 0x474,
        values[0],
    ];
    for (i, v) in args.iter().enumerate() {
        m.put_word(va + i as u32 * 4, *v)?;
    }
    logging_live(
        m,
        [
            logger.args[0],
            3,
            crate::rom_region::addr(rom, 0x155ea8),
            args[0],
            va,
            params,
            crate::rom_region::addr(rom, 0x173df4) + 0x1b4,
        ],
        workspace,
        scratch,
        platform,
    )?;
    Ok(VoiceBoundary {
        entry: 0x156a30,
        args: logger.args,
        event: None,
    })
}

pub fn synthesis_aux_prefix(
    m: &mut Memory<'_, '_>,
    synthesis: VoiceBoundary,
    scheduler: VoiceBoundary,
    continuation: VoiceBoundary,
    request_sp: u32,
    workspace: &mut [u8],
    platform: &mut dyn HeapPlatform,
) -> Result<VoiceBoundary, Error> {
    synthesis_aux_prefix_rom(m, synthesis, scheduler, continuation, request_sp, workspace, platform, crate::rom_region::Rom::Us)
}

#[allow(clippy::too_many_arguments)]
pub fn synthesis_aux_prefix_rom(
    m: &mut Memory<'_, '_>,
    synthesis: VoiceBoundary,
    scheduler: VoiceBoundary,
    continuation: VoiceBoundary,
    request_sp: u32,
    workspace: &mut [u8],
    platform: &mut dyn HeapPlatform,
    rom: crate::rom_region::Rom,
) -> Result<VoiceBoundary, Error> {
    if continuation.entry != 0x156a30
        || crate::rom_region::to_us(rom, scheduler.entry) != 0x15961c
    {
        return Err(Error::UnsupportedStage { entry: continuation.entry });
    }
    let ctx = synthesis.args[0];
    let synth = synthesis.args[1];
    let out = synth + 0x84;
    let sp = request_sp.checked_sub(0x1f8).ok_or(Error::InvalidSnapshot)?;

    let fmt = crate::rom_region::addr(rom, 0x156a30) + 8 + 0x10c;
    m.put_word(sp, fmt)?;
    let args = [m.word(ctx + 16)?, m.word(crate::rom_region::addr(rom, 0x156624))?, 99, 0, fmt, sp + 4, 0];
    speech1_boot_stage(m, StageCall { entry: 0x1738e8, args }, workspace, platform)?;

    m.put_word(sp - 4, crate::rom_region::addr(rom, 0x156a4c))?;
    let pipe = [scheduler.args[2], scheduler.args[3]];
    let aux = [scheduler.args[4], scheduler.args[5]];
    if pipe[0] == 0 {
        return Err(Error::MissingStreamAdapter { entry: 0x156b6c });
    }
    m.put_word(out + 16, pipe[0])?;
    m.put_word(out + 20, pipe[1])?;
    if aux[0] != 0 {
        return Err(Error::MissingStreamAdapter { entry: 0x175c40 });
    }
    let mut value = [0; 8];
    m.read(m.word(crate::rom_region::addr(rom, 0x176fe8))?, &mut value)?;
    let mut pair = [0; 8];
    crate::frontend::load_pair(&mut pair, &value);
    m.write(sp + 8, &pair)?;
    m.put_word(sp, out + 28)?;
    let vtable = m.word(synth + 0x54)?;
    let entry = m.word(vtable + 16)?;
    Ok(VoiceBoundary {
        entry,
        args: [
            u32::from_le_bytes(pair[..4].try_into().unwrap()),
            u32::from_le_bytes(pair[4..].try_into().unwrap()),
            m.word(synth)?,
            m.word(synth + 4)?,
            out + 28,
            0,
            0,
        ],
        event: None,
    })
}

pub fn speech1_heap_stage(
    m: &mut Memory<'_, '_>,
    call: StageCall,
    workspace: &mut [u8],
    platform: &mut dyn HeapPlatform,
) -> Result<StageReturn, Error> {
    let a = call.args;
    let mut os = BridgePlatform(platform);
    let mut flat = crate::bridge::Flat::new(m, workspace, &mut os, a[0])?;
    let value = match call.entry {
        0x1963b8 => flat.fn_1963b8(a[0], a[1], a[2]),
        0x196428 => { flat.fn_196428(a[0], a[1]); 0 },
        0x1963e0 => flat.fn_1963e0(a[0], a[1], a[2]),
        0x198728 => { crate::frontend::init_link(flat.memory().bytes, a[0], a[1], a[2], a[3]); a[0] },
        0x184f20 => flat.fn_184f20([a[0], a[1], a[2], a[3], a[4], a[5]]),
        0x4a7758 | 0x39f5e8 => flat.allocator_method(call.entry, &a[..3]),
        0x195d58 if a[0] == a[1] => 0,
        0x195d58 => {
            let v = flat.memory();
            crate::frontend::compare_str(&v.bytes[a[0] as usize..], &v.bytes[a[1] as usize..]) as u32
        }
        0x195bec => crate::frontend::fill_bytes(flat.memory().bytes, a[0], a[1], a[2]),
        0x195ae8 => { flat.memory().rvct_copy(a[0], a[1], a[2]); a[0] },
        0x195cd0 => crate::frontend::copy_str(flat.memory().bytes, a[0], a[1]),
        0x195d90 => crate::frontend::strlen_bytes(&flat.memory().bytes[a[0] as usize..]),
        0x2ffb34 => crate::speech_part_4::fn_2ffb34(&mut flat.memory(), a[0]),
        0x2ffdc2 => crate::speech_part_4::fn_2ffdc2(&mut flat.memory(), a[0], a[1]),
        0x30208c => crate::speech_part_4::fn_30208c(&mut flat.memory(), a[0]),
        0x1012cc => crate::speech_part_4::fn_1012cc(&mut flat.memory(), a[0], a[1]),
        0x301ea0 => {
            let global = flat.memory().word(0x301ea8);
            crate::speech_part_3::fn_301ea0(&mut flat.memory(), global)
        }
        0x7e715c => crate::speech_part_3::fn_7e715c(&mut flat.memory(), a[0], a[1]),
        0x7f6820 => crate::speech_part_3::fn_7f6820(&mut flat.memory(), a[0]),
        0x4a794c => crate::speech_part_3::fn_4a794c(&mut flat.memory(), a[0], a[1], a[2], a[3]),
        0x4a78f0 => crate::speech_part_3::fn_4a78f0(&mut flat.memory(), a[0], a[1], a[2], a[3]),
        entry => return Err(Error::UnsupportedStage { entry }),
    };
    flat.finish()?;
    Ok(StageReturn { r0: if call.entry == 0x196428 { None } else { Some(value) } })
}

pub(crate) struct SpeechBootHost<'a> {
    pub(crate) platform: &'a mut dyn HeapPlatform,
    pub(crate) fault: Option<Error>,
}
impl SpeechBootHost<'_> {
    pub(crate) fn invoke(&mut self, m: &mut crate::boot::Memory<'_>, f: u32, a: &[u32]) -> u32 {
        if self.fault.is_some() { return 0x80002001; }

        let f = crate::rom_region::to_us(crate::rom_region::Rom::from_entry(f), f);
        match f {

            0x18f3fc => {

                let target = crate::rom_region::to_us(crate::rom_region::Rom::from_entry(m.word(a[0])), m.word(a[0]));
                if target != 0x4a7adc {
                    self.fault = Some(Error::UnsupportedStage { entry: target });
                    return 0x80002001;
                }

                crate::frontend::notify([a[2], a[3], a[4], a[5]], |_| 0)
            }
            0x1ce428 => {
                struct Disabled;
                impl crate::frontend_part_b::Host for Disabled {
                    fn call(&mut self,_:crate::frontend_part_b::Boundary,_:&[u32])->[u32;3] {unreachable!()}
                }
                impl crate::frontend_part_b::LogHost for Disabled {
                    fn time(&mut self,_:u32)->u32 {unreachable!()}
                    fn format_time(&mut self,_:u32,_:u32,_:&mut[u8;128]) {unreachable!()}
                    fn serialize(&mut self,_:&crate::frontend_part_b::LogRecord,_:&mut[u8;3072])->u32 {unreachable!()}
                    fn write(&mut self,_:u32,_:&[u8])->u32 {unreachable!()}
                }
                if a[0]!=0 && a[1]!=0 && m.byte(m.word(a[0])+4)!=0 {
                    self.fault=Some(Error::UnsupportedStage {entry:0x232354});return 0x80002001;
                }
                crate::frontend_part_b::fn_1ce428(m.bytes,a[0],a[1],a[2],a[3],a[4],a[5],&mut Disabled)
            }
            0x1738e8 | 0x173dc0 => {

                let mut v = [0u32; 6];
                let n = a.len().min(6);
                v[..n].copy_from_slice(&a[..n]);
                if f == 0x1738e8 {
                    crate::log_live::fn_1738e8(m, v[0], v[1], v[2], v[3], v[4], v[5], self)
                } else {
                    crate::render_part_a::error_log(m, self, v[0], v[1], v[2], v[3], v[4], v[5])
                }
            }

            0x1ce56c => crate::frontend::streaming_mode(
                (a[0] != 0).then(|| (m.byte(m.word(a[0]) + 4), m.byte(m.word(a[0]) + 0x10c))),
            ),

            0x18f488 => crate::frontend::finish_request(),
            0x184f20 => m.boot_heap_allocate(a.try_into().expect("heap ABI")),
            0x4a7758 => m.fn_4a7758(self, [a[0], a[1], a[2]]),
            0x4a779c => m.fn_4a779c(self, a[0], a[1]),
            0x39f5e8 => m.fn_39f5e8(self, [a[0], a[1], a[2]]),
            0x39f5b8 => { crate::speech_part_3::fn_39f5b8(m, a[0], a[1], self); 0 }
            0x4a774c => crate::speech_part_3::fn_4a774c(m, a[0], a[1], self),
            0x196394 => crate::boot_part_e::alloc(m, a[0], a[1], self),
            0x1963b8 => crate::boot_part_e::calloc(m, a[0], a[1], a[2], self),
            0x1963e0 => crate::boot_part_e::fn_1963e0(m, a[0], a[1], a[2], self),
            0x196428 => { crate::boot_part_e::release(m, a[0], a[1], self); 0 }

            0x1994d0 => crate::frontend::divmod_u32(a[0], a[1]).0,
            0x1840fc => { crate::boot_part_b::fn_1840fc(m, a[0], self); 0 }
            0x18423c => { crate::boot_part_b::fn_18423c(m, a[0], self); 0 }
            0x183edc => {
                let handle = m.word(self.platform.arbiter_address());
                crate::boot_part_b2::fn_183edc(m, a[0], handle, self); 0
            }
            0x165d00 => {
                let g = crate::boot::Globals { manager: 0, callbacks: 0, rules: 0, components: 0,
                    dictionary: 0, context: 0, mode: 0, semaphore: self.platform.arbiter_address(),
                    service: 0, command: 0 };
                m.fn_165d00(self, &g, a[0]); 0
            }
            0x172d18 => { if a.get(2) == Some(&0) { 0 } else { self.platform.arbitrate(m, a) } },
            0x17db60 => crate::boot_part_e::fn_17db60(m, a[0], a[1], a[2], a[3], a[4], self),
            0x1970ac => crate::boot_part_e::fn_1970ac(m, a[0], a[1], a[2]),
            0x17b7f0 => crate::speech_part_1::fn_17b7f0(m, a[0]),
            0x302078 => {
                let global = m.word(0x301ea8);
                crate::speech_part_3::fn_302078(m, global)
            }
            0x1770f8 => crate::speech_part_1::fn_1770f8(m, a[0], a[1]),
            0x196980 => crate::speech_part_1::fn_196980(m, a[0], a[1]),

            0x176cd8 => match a.first() {
                Some(&s) => crate::normalization::atoi_scan(m.bytes, s),
                None => {
                    self.fault = Some(Error::MissingStreamAdapter { entry: f });
                    0x80002001
                }
            },
            _ => {
                self.fault = Some(Error::MissingStreamAdapter { entry: f });
                0x80002001
            }
        }
    }
}
impl crate::log_live::LogMethodsM for SpeechBootHost<'_> {
    fn log_m(&mut self, method: u32, object: u32, category: u32, level: u32, arg: u32,
        format: u32, varargs: u32, m: &mut crate::boot::Memory<'_>) -> u32 {
        self.invoke(m, method, &[object, category, level, arg, format, varargs])
    }
}
impl crate::boot_part_b::FormatHost for SpeechBootHost<'_> {}
impl crate::boot_part_e::Host for SpeechBootHost<'_> {
    fn call(&mut self, m: &mut crate::boot::Memory<'_>, f: u32, a: &[u32]) -> u32 { self.invoke(m, f, a) }
}
impl crate::speech_part_3::Host for SpeechBootHost<'_> {
    fn call(&mut self, m: &mut crate::boot::Memory<'_>, f: u32, a: &[u32]) -> u32 { self.invoke(m, f, a) }
}
impl crate::boot_part_b::Host for SpeechBootHost<'_> {
    fn thread_id(&mut self) -> u32 { self.platform.thread_id() }
    fn call(&mut self, m: &mut crate::boot::Memory<'_>, f: u32, a: &[u32]) -> u32 { self.invoke(m, f, a) }
}
impl crate::boot::Calls for SpeechBootHost<'_> {
    fn call(&mut self, m: &mut crate::boot::Memory<'_>, f: u32, a: &[u32]) -> u32 { self.invoke(m, f, a) }
    fn call_wide(&mut self, _: &mut crate::boot::Memory<'_>, f: u32, _: &[u32]) -> u64 {
        self.fault = Some(Error::MissingStreamAdapter { entry: f }); 0
    }
}

pub fn speech1_boot_stage(
    m: &mut Memory<'_, '_>, call: StageCall, workspace: &mut [u8],
    platform: &mut dyn HeapPlatform,
) -> Result<StageReturn, Error> {
    struct ViewOnly;
    impl crate::bridge::Platform for ViewOnly {
        fn thread_id(&mut self) -> u32 { unreachable!("view-only staging") }
        fn arbiter_address(&self) -> u32 { unreachable!("view-only staging") }
        fn arbitrate(&mut self, _: &mut crate::boot::Memory<'_>, _: &[u32]) -> u32 { unreachable!("view-only staging") }
    }
    let mut os = ViewOnly;
    let mut flat = crate::bridge::Flat::new(m, workspace, &mut os, 0)?;
    let mut host = SpeechBootHost { platform, fault: None };
    let value = host.invoke(&mut flat.memory(), call.entry, &call.args);
    flat.finish()?;
    if let Some(e) = host.fault { return Err(e); }
    Ok(StageReturn { r0: Some(value) })
}
