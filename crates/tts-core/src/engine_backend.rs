//! Live backend controls. Guest pointers address caller-owned engine mappings.
use crate::{
    engine::{Error, Memory},
    rom_region::Rom,
    tables::NumericTables,
    voice::{VoiceEvent, VoiceQueue, VoiceState, VoiceTransition},
};
fn word(b: &[u8], o: usize) -> u32 {
    u32::from_le_bytes(b[o..o + 4].try_into().unwrap())
}
fn put(b: &mut [u8], o: usize, v: u32) {
    b[o..o + 4].copy_from_slice(&v.to_le_bytes());
}
const FLOAT_OFFSETS: [usize; 22] = [
    8, 0x5c, 0x6c, 0x70, 0x74, 0x78, 0x80, 0x84, 0x88, 0x8c, 0x90, 0x94, 0x98, 0x9c, 0xa0, 0xa4,
    0xb0, 0xc0, 0xc4, 0xc8, 0xcc, 0xd0,
];
fn state(bytes: &[u8]) -> VoiceState {
    let f = |o| f32::from_bits(word(bytes, o));
    VoiceState {
        sample_rate: f(8),
        enabled: bytes[0x48] != 0,
        position: f(0x5c),
        time: word(bytes, 0x64) as i32,
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
        durationa8: word(bytes, 0xa8) as i32,
        valueb0: f(0xb0),
        durationb4: word(bytes, 0xb4) as i32,
        active: core::array::from_fn(|i| bytes[0xbc + i] != 0),
        targets: core::array::from_fn(|i| f(0xc0 + i * 4)),
        phases: core::array::from_fn(|i| f(0xcc + i * 4)),
        offsetd8: word(bytes, 0xd8) as i32,
        offsetdc: word(bytes, 0xdc) as i32,
        flag: bytes[0xe0] != 0,
        transitions: core::array::from_fn(|i| VoiceTransition {
            kind: word(bytes, 0x28f8 + i * 20) as i32,
            reciprocal: f(0x28fc + i * 20),
            remaining: word(bytes, 0x2908 + i * 20) as i32,
        }),
    }
}
fn write_state(s: &VoiceState, bytes: &mut [u8]) {
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
    for (o, f) in FLOAT_OFFSETS.into_iter().zip(floats) {
        put(bytes, o, f.to_bits());
    }
    put(bytes, 0xd4, s.phases[2].to_bits());
    for (o, v) in [
        (0x64, s.time),
        (0xa8, s.durationa8),
        (0xb4, s.durationb4),
        (0xd8, s.offsetd8),
        (0xdc, s.offsetdc),
    ] {
        put(bytes, o, v as u32);
    }
    bytes[0x48] = s.enabled as u8;
    bytes[0xe0] = s.flag as u8;
    for i in 0..3 {
        bytes[0xbc + i] = s.active[i] as u8;
    }
    for (i, t) in s.transitions.iter().enumerate() {
        put(bytes, 0x28f8 + i * 20, t.kind as u32);
        put(bytes, 0x28fc + i * 20, t.reciprocal.to_bits());
        put(bytes, 0x2908 + i * 20, t.remaining as u32);
    }
}

fn byte(m: &Memory<'_, '_>, a: u32) -> Result<u8, Error> {
    let mut b = [0];
    m.read(a, &mut b)?;
    Ok(b[0])
}
fn load_queue(m: &Memory<'_, '_>, a: u32, reserve: usize) -> Result<VoiceQueue, Error> {
    let r = m.word(a)?;
    let w = m.word(a + 4)?;
    if r >= 256 || w >= 256 {
        return Err(Error::InvalidSnapshot);
    }
    let mut q = VoiceQueue::default();
    q.read = r as u8;
    q.write = w as u8;
    let mut indices = [false; 256];
    let mut i = q.read;
    while i != q.write {
        indices[i as usize] = true;
        i = i.wrapping_add(1);
    }
    for n in 0..reserve {
        indices[q.write.wrapping_add(n as u8) as usize] = true;
    }
    for (i, needed) in indices.into_iter().enumerate() {
        if needed {
            let mut b = [0; 20];
            m.read(a + 8 + i as u32 * 20, &mut b)?;
            q.events[i] = VoiceEvent {
                kind: word(&b, 0) as i32,
                value: f32::from_bits(word(&b, 4)),
                target: f32::from_bits(word(&b, 8)),
                duration: f32::from_bits(word(&b, 12)),
                delay: word(&b, 16) as i32,
            };
        }
    }
    Ok(q)
}
fn save_queue(
    m: &mut Memory<'_, '_>,
    a: u32,
    before: &VoiceQueue,
    after: &VoiceQueue,
) -> Result<(), Error> {
    for (o, x, y) in [(0, before.read, after.read), (4, before.write, after.write)] {
        if x != y {
            m.put_word(a + o, y as u32)?;
        }
    }
    for i in 0..256 {
        let x = &before.events[i];
        let y = &after.events[i];
        let xs = [
            x.kind as u32,
            x.value.to_bits(),
            x.target.to_bits(),
            x.duration.to_bits(),
            x.delay as u32,
        ];
        let ys = [
            y.kind as u32,
            y.value.to_bits(),
            y.target.to_bits(),
            y.duration.to_bits(),
            y.delay as u32,
        ];
        for j in 0..5 {
            if xs[j] != ys[j] {
                m.put_word(a + 8 + i as u32 * 20 + j as u32 * 4, ys[j])?;
            }
        }
    }
    Ok(())
}
fn clear_ring(m: &mut Memory<'_, '_>, ring: u32, width: u32) -> Result<(), Error> {
    let data = m.word(ring + 4)?;
    let count = m.word(ring + 8)?;
    let bytes = count.checked_mul(width).ok_or(Error::InputTooLong)? as usize;
    m.check(data, bytes)?;
    let zero = [0; 256];
    let mut at = 0;
    while at < bytes {
        let n = (bytes - at).min(zero.len());
        m.write(data + at as u32, &zero[..n])?;
        at += n;
    }
    m.put_word(ring + 12, 0)
}
/// fn_1be020. Reset Nintendo customization, drain the two existing voice
/// queues with the Rust fn_1bcd84 port, enqueue defaults, and clear delay rings.
/// `object` is the Nintendo backend, not the outer Voice wrapper. `tables`
/// borrows the numeric tables from the same caller-owned voice image.
pub(crate) fn reset(
    m: &mut Memory<'_, '_>,
    object: u32,
    tables: &NumericTables<'_>,
) -> Result<(), Error> {
    let mut s = VoiceState::default();
    s.time = m.word(object + 0x64)? as i32;
    let mut input = load_queue(m, object + 0xe8, 12)?;
    let forwarded = input.events.iter().filter(|e| e.kind == 17).count();
    let mut commands = load_queue(m, object + 0x14f0, (forwarded + 1).min(256))?;
    let old_input = input.clone();
    let old_commands = commands.clone();
    if byte(m, object + 4)? != 0 && (input.read != input.write || commands.read != commands.write) {
        let mut b = [0; 0x2920];
        m.read(object, &mut b[..0xe8])?;
        m.read(object + 0x28f8, &mut b[0x28f8..])?;
        let before = b;
        s = state(&b);
        s.drain_events(true, &mut input, &mut commands, tables);
        write_state(&s, &mut b);
        for (i, (&x, &y)) in before.iter().zip(b.iter()).enumerate() {
            if x != y {
                m.write(object + i as u32, &[y])?;
            }
        }
    }
    let base = s.time;
    input.enable_voicing(base, 0);
    input.set_pitch_blend(base, 0, 1.0);
    input.set_pitch(base, 0, 1.0);
    input.set_gain(base, 0, 1.0);
    input.set_resampling(base, 0, 1.0);
    input.set_frequency(base, 0, 185.0);
    for i in 0..3 {
        input.set_harmony(base, false, i, 0, 1.0);
    }
    input.set_smoothing(base, 0, 0.05);
    input.enqueue_duration(base, 0, 0.0, 0.0);
    input.set_stretch_modulation(base, 0, 0.0, 0.0);
    save_queue(m, object + 0xe8, &old_input, &input)?;
    save_queue(m, object + 0x14f0, &old_commands, &commands)?;
    for o in [0x4bd0, 0x4bd8, 0x4bd9, 0x4c30] {
        m.write(object + o, &[0])?;
    }
    m.put_word(object + 0x4bdc, 0)?;
    m.put_word(object + 0x4c34, 1)?;
    for o in [
        0x4bd4, 0x4be0, 0x4be4, 0x4be8, 0x4bec, 0x4bf0, 0x4bf4, 0x4bfc, 0x4c04, 0x4c08, 0x4c0c,
        0x4c10, 0x4c14, 0x4c18,
    ] {
        m.put_word(object + o, 1.0f32.to_bits())?;
    }
    for (o, v) in [
        (0x4bf8, 0.05f32),
        (0x4c00, 185.0),
        (0x4c1c, 120.0),
        (0x4c20, 0.0),
        (0x4c24, 0.0),
        (0x4c28, 0.0),
        (0x4c2c, 0.0),
    ] {
        m.put_word(object + o, v.to_bits())?;
    }
    clear_ring(m, m.word(object + 0x31f4 + 0x1c)?, 2)?;
    m.put_word(object + 0x31f4 + 0x20, 0)?;
    for o in [0x34, 0x38, 0x3c] {
        clear_ring(m, m.word(object + 0x31a4 + o)?, 4)?;
    }
    m.put_word(object + 0x31a4 + 0x4c, 0)?;
    Ok(())
}
/// fn_743d70 numeric parameter dispatch. Mutates live fields and enqueues
/// real Rust VoiceQueue events. Unsupported IDs return the ROM's -1.
pub(crate) fn set_numeric(m: &mut Memory<'_, '_>, o: u32, id: u32, v: i32) -> Result<i32, Error> {
    let f = |m: &Memory<'_, '_>, at| m.word(o + at).map(f32::from_bits);
    let mut pair = None;
    let (field, value, kind) = match id {
        3 => (0x4bd4, if v != 0 { -1.0 } else { 1.0 }, Some(8)),
        4 => {
            m.write(o + 0x4bd8, &[(v != 0) as u8])?;
            if v == 0 {
                for at in [0x34, 0x38, 0x3c] {
                    clear_ring(m, m.word(o + 0x31a4 + at)?, 4)?;
                }
                m.put_word(o + 0x31a4 + 0x4c, 0)?;
            }
            return Ok(0);
        }
        200_000_000 | 220_000_000 | 230_000_000 => {
            let i = match id {
                200_000_000 => 0,
                220_000_000 => 1,
                _ => 2,
            };
            let ratio = v as f32 * 0.0001;
            m.put_word(o + 0x4bec + i * 4, ratio.to_bits())?;
            let old = load_queue(m, o + 0xe8, 1)?;
            let mut q = old.clone();
            q.set_harmony(m.word(o + 0x64)? as i32, v != 10000, i as i32, 0, ratio);
            save_queue(m, o + 0xe8, &old, &q)?;
            return Ok(0);
        }
        6 => {
            m.write(o + 0x4c30, &[(v != 0) as u8])?;
            (0, (v != 0) as u8 as f32, Some(20))
        }
        10_000_000 => (0x4be4, v as f32 * 0.0001, Some(0)),
        20_000_000 => (0x4c00, v as f32, Some(3)),
        40_000_000 => (0x4c04, v as f32 * 0.0001, Some(2)),
        50_000_000 => (0x4be8, v as f32 * 0.0001, Some(1)),
        60_000_000 => (0x4bfc, v as f32 * 0.0001, Some(8)),
        70_000_000 => (0x4bf8, v as f32 * 0.001, Some(7)),
        80_000_000 => (0x4c0c, v as f32 * 0.0001, None),
        90_000_000 => (0x4c10, v as f32 * 0.0001, None),
        100_000_000 => (0x4c1c, v as f32 * 0.01, None),
        110_000_000 => {
            pair = Some(f(m, 0x4c24)?);
            (0x4c20, v as f32 * 0.0001, Some(11))
        }
        120_000_000 => {
            pair = Some(v as f32 * 0.001);
            (0x4c24, v as f32 * 0.001, Some(11))
        }
        130_000_000 => {
            pair = Some(f(m, 0x4c2c)?);
            (0x4c28, v as f32 * 0.0001, Some(12))
        }
        140_000_000 => {
            pair = Some(v as f32 * 0.001);
            (0x4c2c, v as f32 * 0.001, Some(12))
        }
        240_000_000 => {
            m.put_word(o + 0x4bdc, v as u32)?;
            return Ok(0);
        }
        250_000_000 => (0x4be0, v.clamp(0, 10000) as f32 * 0.0001, None),
        _ => return Ok(-1),
    };
    if field != 0 {
        m.put_word(o + field, value.to_bits())?;
    }
    if let Some(kind) = kind {
        let old = load_queue(m, o + 0xe8, 1)?;
        let mut q = old.clone();
        let base = m.word(o + 0x64)? as i32;
        let x = match id {
            3 => value * f(m, 0x4bfc)?,
            60_000_000 => value * f(m, 0x4bd4)?,
            120_000_000 => f(m, 0x4c20)?,
            140_000_000 => f(m, 0x4c28)?,
            _ => value,
        };
        if let Some(y) = pair {
            q.enqueue_pair(base, kind, 0, x, y);
        } else {
            q.enqueue(base, kind, 0, x);
        }
        save_queue(m, o + 0xe8, &old, &q)?;
    }
    Ok(0)
}
fn signed_half(m: &Memory<'_, '_>, a: u32) -> Result<i32, Error> {
    let mut b = [0; 2];
    m.read(a, &mut b)?;
    Ok(i16::from_le_bytes(b) as i32)
}
/// fn_71efdc uses only index and offset; the mode register is overwritten.
/// The two ROM tables move per region, so the caller threads `rom` and the
/// reads go through the region map (US identity, EU/KR twins).
fn convert(m: &Memory<'_, '_>, index: u32, value: i32) -> Result<i32, Error> {
    convert_rom(m, index, value, Rom::Us)
}
fn convert_rom(m: &Memory<'_, '_>, index: u32, value: i32, rom: Rom) -> Result<i32, Error> {
    let k = index * 3;
    let base = crate::rom_region::addr(rom, 0x8bdc2c);
    let span = signed_half(m, base + (k + 1) * 2)? - signed_half(m, base + k * 2)?;
    let words = crate::rom_region::addr(rom, 0xa7b6c4);
    let lo = m.word(words + (k + 1) * 4)? as i32;
    let hi = m.word(words + (k + 2) * 4)? as i32;
    Ok((lo as f32 + (hi.wrapping_sub(lo) as f32) * (value as f32 / span as f32)) as i32)
}
/// fn_3912fc/3912ec/39133c/39131c/39132c, through their concrete Nintendo
/// vtable targets. `wrapper` owns the impl pointer at +8; value is signed.
/// Speed (fn_39130c, the rate setter: convert idx1, rate clamp 50-400,
/// update_rate propagation) uses `set_quality` instead: it needs the
/// parameter lock host as well as the Nintendo wrapper. (The old comment
/// mislabeled it quality; tomoFunc.h: speed=0x39130c, quality=0x3912ec.)
pub(crate) fn set_voice(
    m: &mut Memory<'_, '_>,
    entry: u32,
    wrapper: u32,
    value: i32,
    tables: &NumericTables<'_>,
) -> Result<(), Error> {
    set_voice_rom(m, entry, wrapper, value, tables, Rom::Us)
}
/// Region-aware `set_voice`: the per-voice-mode float tables move per
/// region, so the table read goes through the region map. `entry` names
/// the Nintendo setter (a dispatch key, not a guest address); `wrapper`
/// is the caller-owned voice object from boot state.
pub(crate) fn set_voice_rom(
    m: &mut Memory<'_, '_>,
    entry: u32,
    wrapper: u32,
    value: i32,
    tables: &NumericTables<'_>,
    rom: Rom,
) -> Result<(), Error> {
    let index = match entry {
        0x3912fc => 0,
        0x3912ec => 2,
        0x39133c => 3,
        0x39131c => 4,
        0x39132c => 5,
        _ => return Err(Error::UnsupportedStage { entry }),
    };
    let o = m.word(wrapper + 8)?;
    if index == 5 && !(0..=3).contains(&value) {
        m.write(o + 0x6a0a, &(value as i16).to_le_bytes())?;
        return Ok(());
    }
    let backend = m.word(o + 0x61b8)?;
    // Only speed and pitch index the per-voice-mode tables. The other
    // Nintendo setters do not read the mode byte in the ROM.
    let mode = if index == 0 || index == 2 {
        let mode = byte(m, o + 0x17a)? as i8 as i32;
        if !(0..8).contains(&mode) {
            return Err(Error::InvalidVoice);
        }
        mode
    } else {
        0
    };
    let mut mapped = if index == 5 {
        value
    } else {
        convert_rom(m, index, value, rom)?
    };
    let id = match index {
        0 | 2 => {
            let table = crate::rom_region::addr(rom, if index == 0 { 0x8bfbc4 } else { 0x8bfc04 });
            mapped = mapped.wrapping_add(m.word(table + mode as u32 * 4)? as i32);
            mapped = (crate::math::powf(2.0, mapped as f32 * 0.0008333333535119891, tables)
                * 10000.0) as i32;
            if index == 0 {
                10_000_000
            } else {
                50_000_000
            }
        }
        3 => 80_000_000,
        4 => 60_000_000,
        _ => {
            intonation(m, o, backend, value)?;
            m.write(o + 0x6a00 + index * 2, &(value as i16).to_le_bytes())?;
            return Ok(());
        }
    };
    if set_numeric(m, backend, id, mapped)? != 0 {
        return Err(Error::UnsupportedStage { entry: 0x743d70 });
    }
    m.write(o + 0x6a00 + index * 2, &(value as i16).to_le_bytes())?;
    Ok(())
}
fn intonation(m: &mut Memory<'_, '_>, o: u32, b: u32, v: i32) -> Result<(), Error> {
    if v == 0 {
        for id in [3, 110_000_000, 130_000_000] {
            set_numeric(m, b, id, 0)?;
        }
    } else if (1..=3).contains(&v) {
        let i = (v - 1) as u32 * 4;
        for (id, at) in [
            (3, 0xf0),
            (110_000_000, 0xb0),
            (120_000_000, 0xc0),
            (130_000_000, 0xd0),
        ] {
            let x = m.word(o + at + i)? as i32;
            set_numeric(m, b, id, x)?;
        }
        let mut h = [0; 2];
        m.read(o + 0x182, &mut h)?;
        let scale = u16::from_le_bytes(h) as f32 * 0.01;
        let x = ((m.word(o + 0xe0 + i)? as i32 as f32) * scale) as i32;
        set_numeric(m, b, 140_000_000, x)?;
    }
    Ok(())
}

/// Regional resources and caller-owned EU pitch scratch. Korean timing comes
/// from the live backend's regional timing table, not a US default.
/// EU voices need a Resonator tracker and max_period floats of scratch;
/// US/KR use Crossings and empty scratch. Retain scratch across calls.
pub(crate) struct BackendRegion<'a> {
    pub(crate) language: crate::pitch::Language,
    pub(crate) scratch: &'a mut [f32],
    pub(crate) frequencies: &'a [f32; 5],
    pub(crate) korean_timing: Option<&'a crate::output::KoreanTiming>,
}

/// Persistent typed binding of fn_1a18c4 to fn_1b0a64 and fn_1b7e6c.
/// Decode the first backend input once into caller-owned state and buffers.
/// Keep those objects across every event-16 call, then call with empty input
/// and `flush = true` for event 2. Only incoming samples/events are per-call.
/// No recorded synthesis replies or PCM enter this function.
///
/// Language, pitch scratch, frequencies and Korean timing are inputs.
/// Initialize synth.voice from the same live
/// voice held by setup. The binding swaps it across each PCM render so command
/// updates and timer consumption share one state. Retain resources.synth,
/// position, assembled frame/audio, all synthesis buffers, filter rings,
/// effects, segments and lip-sync storage between calls. Never reseed from
/// later ARM states.
///
/// Mapped Nintendo input: buffer pointers at +0x20/24/2c/3c/44/50, filter
/// headers at +0x31d8/31dc/31e0/3210, frame at +0x3828, position at +0x4bc8.
/// KR omits the word at +0x4bbc: subsequent guest offsets are four bytes
/// lower. Decode that regional layout before calling this typed binding.
/// Read rate from the initial parameter tree: sing_1 uses 74, not the 116
/// restored by request teardown. persistent_backend_pcm demonstrates complete
/// initialization and serialization without copying an ARM reply.
///
/// In engine.rs, use this call as the real synthesize closure for
/// SynthesisCallback::render_stream on event 16 and SynthesisBoundary::finish
/// on event 2. Those wrappers own publication and minimum output padding.
/// Pass their output spans unchanged.
#[allow(clippy::too_many_arguments)]
pub(crate) fn render_backend<'a, 'b: 'a>(
    setup: &mut crate::orchestration::Setup<'a, 'b>,
    synth: &mut crate::synthesis::SynthesisState,
    frame: &mut crate::queue::SpeechFrame,
    audio: &mut [i16; crate::queue::AUDIO_CAPACITY],
    position: &mut i32,
    voice_boundary: crate::queue::VoiceBoundary,
    classification: &[u8; 256],
    timing: &mut crate::output::SpeechTiming,
    segments: &mut crate::frames::PlannedSegments<'a>,
    lip: &mut crate::frames::LipSync<'a>,
    resources: &mut crate::resources::EngineResources<'_>,
    input: &mut [i16],
    out: &mut [i32],
    grain: &mut [i32],
    window: &[f32],
    modulation: &[f32; 1024],
    normalization: &[f32; 32],
    effect: &mut crate::effects::VoiceEffect,
    wave: &[i16; 1024],
    tables: &NumericTables<'_>,
    samples: &[i16],
    events: &[crate::queue::SpeechEvent],
    outputs: [&mut [i16]; 2],
    flush: bool,
    flush_silence: bool,
    region: &mut BackendRegion<'_>,
) -> [usize; 2] {
    use crate::{
        frames_pcm::FrameSynthesis, orchestration::Setup, output, synthesis::SynthesisBuffers,
    };
    let korean = region.language == crate::pitch::Language::Korean;
    let language = region.language;
    let setup_cell = core::cell::RefCell::new(setup);
    let mut backend = output::StreamSynthesis {
        frame: frame,
        audio: &mut *audio,
        position: *position,
        voice: voice_boundary,
        unvoiced: &classification,
        korean,
        renderer: |frame: &crate::queue::SpeechFrame,
                   audio: &[i16],
                   outputs: [&mut [i16]; 2],
                   flush: bool| {
            let view = output::SpeechFrame::from_assembled(frame, audio);
            let mut borrowed = setup_cell.borrow_mut();
            let setup = &mut **borrowed;
            let mut renderer = output::FrameSynthesis {
                setup,
                timing: &mut *timing,
                segments: &mut *segments,
                lip_sync: &mut *lip,
            };
            let mut render_pcm = |setup: &mut Setup<'_, '_>,
                                  samples: &[i16],
                                  outputs: [&mut [i16]; 2],
                                  finish: bool| {
                core::mem::swap(&mut synth.voice, setup.voice);
                let mut pcm = FrameSynthesis {
                    state: &mut *synth,
                    buffers: SynthesisBuffers {
                        input: &mut *input,
                        output: &mut *out,
                        grain: &mut *grain,
                        window: window,
                        modulation: modulation,
                        normalization: normalization,
                    },
                    input_events: setup.input,
                    commands: setup.commands,
                    language,
                    scratch: &mut *region.scratch,
                    frequencies: region.frequencies,
                    tables: tables,
                    bypass_effects: setup.state.enabled,
                    voice_effect: if setup.state.voice_enabled {
                        Some((&mut *effect, setup.voice_filter, wave))
                    } else {
                        None
                    },
                    sample_effect: if setup.state.control_enabled {
                        Some((
                            setup.filter,
                            setup.controls.values[2],
                            setup.controls.values[3] as u32,
                        ))
                    } else {
                        None
                    },
                };
                let [a, b] = outputs;
                let made = pcm.render(samples, a, b, finish);
                core::mem::swap(&mut synth.voice, setup.voice);
                made
            };
            let [first, second] = outputs;
            if korean {
                renderer.render_korean(
                    resources,
                    0x684f424a,
                    &view,
                    [first, second],
                    flush,
                    tables,
                    region.korean_timing.expect("Korean timing input"),
                    &mut render_pcm,
                )
            } else {
                renderer.render(
                    resources,
                    0x684f424a,
                    &view,
                    [first, second],
                    flush,
                    tables,
                    &mut render_pcm,
                )
            }
        },
        filter: |samples: &mut [i16]| {
            let mut borrowed = setup_cell.borrow_mut();
            let setup = &mut **borrowed;
            setup.filter.process(
                setup.controls.values[2],
                setup.controls.values[3] as u32,
                samples,
            );
        },
    };

    let made = output::synthesize_stream(
        &mut backend,
        samples,
        events,
        outputs,
        flush as u32,
        flush_silence,
    );
    *position = backend.position;
    made
}

/// OS locks are host operations. Internal stop/start paths are not delegated.
pub trait BackendLocks {
    /// Caller-owned allocator binding used only when the event vector grows.
    fn event_allocate(
        &mut self,
        _: &mut Memory<'_, '_>,
        _: u32,
        _: u32,
        _: u32,
    ) -> Result<u32, Error> {
        Err(Error::UnsupportedStage { entry: 0x183258 })
    }
    /// Release the old caller-owned event allocation after relocation.
    fn event_release(&mut self, _: &mut Memory<'_, '_>, _: u32, _: u32) -> Result<(), Error> {
        Err(Error::UnsupportedStage { entry: 0x183258 })
    }
    /// Heap-vtable binding for fn_176054. A null old pointer allocates;
    /// otherwise the host regrows the caller-owned block, returning zero on refusal.
    fn parameter_resize(
        &mut self,
        _: &mut Memory<'_, '_>,
        _: u32,
        _: u32,
        _: u32,
    ) -> Result<u32, Error> {
        Err(Error::UnsupportedStage { entry: 0x1963e0 })
    }
    /// Live fn_19668c insertion. The default refuses unbound callers.
    fn parameter_insert(&mut self, _: &mut Memory<'_, '_>, _: u32, _: &[u8]) -> Result<u32, Error> {
        Err(Error::UnsupportedStage { entry: 0x19668c })
    }
    /// Live fn_19310c begin/end and error trace, on the same request memory.
    fn parameter_trace(&mut self, _: &mut Memory<'_, '_>, _: u32, status: Option<u32>, _: bool) -> Result<(), Error> { if status.is_some() { Err(Error::UnsupportedStage { entry: 0x173dc0 }) } else { Ok(()) } }
    /// Inherited ARM s16/s17 bits. Required on newly allocated log storage.
    fn event_vfp(&mut self) -> Option<[u32; 2]> {
        None
    }
    fn acquire_m(&mut self, _: &mut Memory<'_, '_>, lock:u32)->Result<u32,Error> {Ok(self.acquire(lock))}
    fn release_m(&mut self, _: &mut Memory<'_, '_>, lock:u32)->Result<u32,Error> {Ok(self.release(lock))}
    fn acquire(&mut self, lock: u32) -> u32;
    fn release(&mut self, lock: u32) -> u32;
    fn ticks(&mut self) -> Result<u64, Error> {
        Err(Error::UnsupportedStage { entry: 0x183258 })
    }
}

/// fn_3914e8 -> fn_75b708 + fn_35a958 -> fn_35a9b0.
/// Covers active Nintendo voices and their running Nuance stop closure.
/// Inactive fn_35adf8 unconditionally waits in SDK fn_183dd4. That boot
/// dependency has no executable port yet, so ownership/restart fails closed.
pub(crate) fn parameter_reset(
    m: &mut Memory<'_, '_>,
    wrapper: u32,
    locks: &mut impl BackendLocks,
) -> Result<(), Error> {
    let object = m.word(wrapper + 8)?;
    m.put_word(object + 0x2c, 0)?;
    if byte(m, object + 0x61a0)? != 0 {
        return Ok(());
    }
    if byte(m, object + 0x9d)? == 0 {
        // Required by fn_35adf8 at 0x35ae90, after the game audio restart.
        // Do not run a partial restart or invent a platform success for it.
        return Err(Error::UnsupportedStage { entry: 0x183dd4 });
    }
    // fn_35a9b0 sets the pending flag before trying vauto_ttsStop.
    m.write(object + 0x61a1, &[1])?;
    let engine = m.word(object + 0x11c)?;
    let tag = m.word(object + 0x120)?;
    if tag == 0x684f424a && engine != 0 && m.word(engine)? == 0x734f424a {
        running_stop(m, engine, locks)?;
    }
    m.put_word(object + 0x61ac, 0)?;
    m.put_word(object + 0x61b0, 0)?;
    m.write(object + 0x61a0, &[0])?;
    m.put_word(object + 0x80, 0)?;
    m.write(object + 0x61a2, &[0])?;
    // fn_35a958 prepares the still-active voice after fn_35a9b0 returns.
    m.write(object + 0x61a0, &[1])?;
    Ok(())
}

/// fn_39130c -> fn_35980c -> fn_35b0ec. Set the live numeric rate and
/// notify its registered Rust callbacks. Existing values may allocate or grow
/// through the caller's live heap and map bindings.
pub fn set_quality(
    m: &mut Memory<'_, '_>,
    wrapper: u32,
    value: i32,
    locks: &mut impl BackendLocks,
    host: Option<&mut dyn AttrHost>,
) -> Result<(), Error> {
    set_quality_rom(m, wrapper, value, locks, Rom::Us, host)
}
/// Region-aware `set_quality`: the rate table moves per region, so the
/// read goes through the region map. `wrapper` is caller-owned boot state.
pub fn set_quality_rom(
    m: &mut Memory<'_, '_>,
    wrapper: u32,
    value: i32,
    locks: &mut impl BackendLocks,
    rom: Rom,
    host: Option<&mut dyn AttrHost>,
) -> Result<(), Error> {
    let object = m.word(wrapper + 8)?;
    let mode = byte(m, object + 0x17a)? as i8 as i32;
    if !(0..8).contains(&mode) {
        return Err(Error::InvalidVoice);
    }
    let mapped = convert_rom(m, 1, value, rom)?
        .wrapping_add(m.word(crate::rom_region::addr(rom, 0x8bfbe4) + mode as u32 * 4)? as i32);
    let rate = (mapped as u16).clamp(50, 400);
    m.write(object + 0x182, &rate.to_le_bytes())?;
    let engine = m.word(object + 0x11c)?;
    if m.word(object + 0x120)? == 0x684f424a && engine != 0 && m.word(engine)? == 0x734f424a {
        locks.parameter_trace(m, engine, None, false)?;
        let context = m.word(engine + 8)?;
        let params = m.word(context + 0x14)?;
        let lock = m.word(params + 4)?;
        if locks.acquire_m(m, lock)? < 0x80000000 {
            let result = update_rate(m, params, rate, locks, rom, host);
            locks.release_m(m, lock)?;
            // fn_35b0ec ignores vauto_ttsSetParamList's returned status.
            // Missing Rust closure bindings still fail explicitly.
            match result {
                Ok(()) => (),
                // The error path additionally formats and publishes an engine
                // trace. Do not report successful byte identity without it.
                Err(Error::EngineStatus(status)) => {
                    locks.parameter_trace(m, engine, Some(status), false)?;
                }
                Err(e) => return Err(e),
            }
        }
        locks.parameter_trace(m, engine, None, true)?;
    }
    m.write(object + 0x6a02, &(value as i16).to_le_bytes())
}

/// Dispatcher host for the rate attr-propagation loop (`fn_174b38` body).
/// Nested parameter calls (`0x19668c`/`0x1966ac`/`0x176054`) ride the live
/// dispatcher 1:1; the first unbound entry is reported so the next bind
/// lands precisely instead of failing at the whole setter.
pub trait AttrHost {
    fn call_attr(&mut self, m: &mut Memory<'_, '_>, f: u32, a: &[u32]) -> u32;
    fn attr_unbound(&self) -> Option<u32>;
}

/// `fn_174b38` on the rate key: for each attribute object, look the key up
/// in its dict; on a hit propagate the new value into the dependent slot.
/// Caller-owned 32-byte window at `0x2000` (saved/restored); the key rides
/// the tree's own name pointer, the value is copied into the window.
fn attr_propagate(
    m: &mut Memory<'_, '_>,
    attrs: u32,
    key: u32,
    text: &[u8],
    host: &mut dyn AttrHost,
) -> Result<(), Error> {
    const WIN: u32 = 0x2000;
    let mut saved = [0u8; 32];
    m.read(WIN, &mut saved)?;
    let r = (|| -> Result<(), Error> {
        m.write(WIN, text)?;
        m.put_word(WIN + 16, WIN)?;
        m.put_word(WIN + 20, text.len() as u32)?;
        m.put_word(WIN + 24, 0)?;
        m.put_word(WIN + 28, 0)?;
        let incoming = WIN + 16;
        let count = m.word(attrs + 20)?;
        let base = m.word(attrs)?;
        let stride = signed_half(m, attrs + 4)? as u32;
        let mut i = 0;
        let mut r = 0u32;
        while i < count {
            let ent = base.wrapping_add(i.wrapping_mul(stride));
            r = host.call_attr(m, 0x19668c, &[m.word(ent + 4)?, key, incoming]);
            if r & 0x1fff == 2 {
                m.put_word(WIN + 28, 0)?;
                r = host.call_attr(m, 0x1966ac, &[m.word(ent + 4)?, key, WIN + 28]);
                if r >= 0x80000000 {
                    r = (r & 0x1fff) | 0x8b702000;
                } else {
                    r = host.call_attr(
                        m,
                        0x176054,
                        &[m.word(WIN + 28)?, WIN, text.len() as u32, m.word(m.word(ent)?)?],
                    );
                }
            }
            i += 1;
            if r >= 0x80000000 {
                break;
            }
        }
        if r >= 0x80000000 {
            return Err(Error::UnsupportedStage {
                entry: host.attr_unbound().unwrap_or(0x174b38),
            });
        }
        Ok(())
    })();
    m.write(WIN, &saved)?;
    r
}

fn update_rate(
    m: &mut Memory<'_, '_>,
    params: u32,
    rate: u16,
    locks: &mut impl BackendLocks,
    rom: crate::rom_region::Rom,
    host: Option<&mut dyn AttrHost>,
) -> Result<(), Error> {
    let tree = m.word(params + 8)? + 24;
    let mut node = m.word(tree)?;
    let mut found = 0;
    let mut key_ptr = 0;
    while node != tree {
        let mut name = [0; 64];
        let name_ptr = m.word(node + 16)?;
        m.read(name_ptr, &mut name)?;
        let end = name
            .iter()
            .position(|&x| x == 0)
            .ok_or(Error::InvalidSnapshot)?;
        match b"rate".as_slice().cmp(&name[..end]) {
            core::cmp::Ordering::Equal => {
                found = m.word(node + 20)?;
                key_ptr = name_ptr;
                break;
            }
            core::cmp::Ordering::Less => node = m.word(node)?,
            core::cmp::Ordering::Greater => node = m.word(node + 4)?,
        }
    }
    let mut text = [0; 4];
    let size = if rate >= 100 {
        text[0] = b'0' + (rate / 100) as u8;
        text[1] = b'0' + (rate / 10 % 10) as u8;
        text[2] = b'0' + (rate % 10) as u8;
        4
    } else {
        text[0] = b'0' + (rate / 10) as u8;
        text[1] = b'0' + (rate % 10) as u8;
        3
    };
    let missing = found == 0;
    if missing {
        // Incoming Value has no per-value callbacks. Global callbacks still
        // run before insertion, and after insertion with the incoming value.
        found = 0;
    }
    let data = if missing { 0 } else { m.word(found)? };
    let old_size = if missing { 0 } else { m.word(found + 4)? as usize };
    let mut old = [0; 4];
    if old_size == size {
        m.read(data, &mut old[..size])?;
    }
    if old_size == size && old[..size] == text[..size] {
        return Ok(());
    }
    let attrs = m.word(params + 16)?;
    if attrs != 0 && m.word(attrs + 20)? != 0 {
        match host {
            Some(h) => attr_propagate(m, attrs, key_ptr, &text[..size], h)?,
            None => return Err(Error::UnsupportedStage { entry: 0x174b38 }),
        }
    }
    // fn_175170: value callbacks precede global callbacks, in both passes.
    for pass in 0..2 {
        for list in [if missing { 0 } else { m.word(found + 8)? }, m.word(params + 12)?] {
            if list == 0 {
                continue;
            }
            let count = m.word(list + 20)?;
            let base = m.word(list)?;
            let stride = signed_half(m, list + 4)? as u32;
            for i in 0..count {
                let entry = base + i * stride;
                let owner = m.word(entry)?;
                // Callback slots hold guest code pointers: regional twins
                // on EU/KR runs. Normalize US-first (direct US literals
                // from the ports keep working); the unbound report keeps
                // the raw guest entry.
                let raw = m.word(entry + 4 + pass * 4)?;
                let function = crate::rom_region::to_us(rom, raw);
                if pass == 0 {
                    match function {
                        0 | 0x22508c => (),
                        0x2289b0 => {
                            let mut valid = 1;
                            let rc = crate::frontend_output::validate_parameter(
                                b"rate",
                                &text[..size],
                                &mut valid,
                            );
                            if rc >= 0x80000000 {
                                return Err(Error::EngineStatus(rc));
                            }
                        }
                        0x22c5f0 => {
                            let p = crate::prosody_rules::InserterParameters::default();
                            let mut valid = 1;
                            let rc = p.validate(b"rate", &text[..size], &mut valid);
                            if rc >= 0x80000000 {
                                return Err(Error::EngineStatus(rc));
                            }
                        }
                        _ => return Err(Error::UnsupportedStage { entry: raw }),
                    }
                } else {
                    let mut converted = 0;
                    let rc = crate::selection::inverse_percent(rate as i32, &mut converted);
                    if rc >= 0x80000000 {
                        return Err(Error::EngineStatus(rc));
                    }
                    match function {
                        0x225138 => m.put_word(owner + 0x740, converted as u32)?,
                        0x228aac => {
                            let lock = m.word(owner + 12)?;
                            if locks.acquire_m(m, lock)? < 0x80000000 {
                                let engine = m.word(owner + 0x28)?;
                                // fn_2a9e64. The table-like absolute is an integer,
                                // not ROM bytes: the real callee is rounded_product.
                                if (converted as u32).wrapping_sub(11) <= 189 {
                                    let scale = if converted == 100 {
                                        0x4000
                                    } else {
                                        crate::selection::rounded_product(0xa3d70a, converted, 16)
                                    };
                                    m.put_word(engine + 36, scale as u32)?;
                                }
                                locks.release_m(m, lock)?;
                            }
                        }
                        0x22c6dc => {
                            let mut p = crate::prosody_rules::InserterParameters::default();
                            p.set(b"rate", &text[..size]);
                            m.write(owner + 0x62, &p.rate.to_le_bytes())?;
                        }
                        _ => return Err(Error::UnsupportedStage { entry: raw }),
                    }
                }
            }
        }
        if pass == 0 {
            if missing {
                let status = locks.parameter_insert(m, m.word(params + 8)?, &text[..size])?;
                if status >= 0x80000000 { return Err(Error::EngineStatus((status & 0x1fff) | 0x8b702000)); }
            } else {
                store_parameter(m, found, &text[..size], m.word(params)?, locks)?;
            }
        }
    }
    Ok(())
}

/// Mapped fn_163a98 stop-event arm. All event-log storage is caller owned.
fn stop_event(
    m: &mut Memory<'_, '_>,
    engine: u32,
    host: &mut impl BackendLocks,
) -> Result<u32, Error> {
    let callback = m.word(engine + 0x170)?;
    if callback == 0 {
        return Ok(0);
    }
    if callback != 0x132124 {
        return Err(Error::UnsupportedStage { entry: callback });
    }
    let speech = m.word(engine + 0x184)?;
    let vfp = if let Some(bits) = host.event_vfp() {
        bits
    } else {
        // Legacy boundary captures omit VFP registers. Preserve their two
        // slot words only for a preallocated slot, never for fresh storage.
        let begin = m.word(speech + 0x61d0)?;
        let end = m.word(speech + 0x61d4)?;
        let capacity = m.word(speech + 0x61d8)? & 0x3fffffff;
        if end.wrapping_sub(begin) / 32 >= capacity || end == 0 {
            return Err(Error::UnsupportedStage { entry: 0x183258 });
        }
        [m.word(end + 4)?, m.word(end + 28)?]
    };
    append_event(m, speech, 32, vfp, &mut EventWorkspace::new(), host)?;
    struct EventHost;
    impl crate::frontend_output::CallbackHost for EventHost {
        fn event(&mut self, _: &mut crate::frontend_output::SynthesisCallback, _: u32) {}
        fn reset(&mut self, _: &mut crate::frontend_output::SynthesisCallback) {
            unreachable!()
        }
        fn finish(&mut self, _: &mut crate::frontend_output::SynthesisCallback) {
            unreachable!()
        }
        fn available(&mut self, _: &mut crate::frontend_output::SynthesisCallback) -> bool {
            unreachable!()
        }
        fn render(&mut self, _: &mut crate::frontend_output::SynthesisCallback) {
            unreachable!()
        }
        fn lock(&mut self, _: &mut crate::frontend_output::SynthesisCallback) {
            unreachable!()
        }
        fn ready(&mut self, _: &mut crate::frontend_output::SynthesisCallback) -> bool {
            unreachable!()
        }
        fn unlock(&mut self, _: &mut crate::frontend_output::SynthesisCallback) {
            unreachable!()
        }
        fn sleep(&mut self, _: &mut crate::frontend_output::SynthesisCallback, _: u64) {
            unreachable!()
        }
    }
    // Logging was just executed above, not delegated to the event callback.
    let mut state = crate::frontend_output::SynthesisCallback::default();
    state.requested = byte(m, speech + 0x61a3)?;
    state
        .handle(32, None, &mut EventHost)
        .map_err(|_| Error::InvalidSnapshot)?;
    m.write(speech + 0x61a3, &[state.requested])?;
    Ok(0)
}

/// TTSEG break, with validated live environment and caller-owned local view.
fn scheduler_break(
    m: &mut Memory<'_, '_>,
    object: u32,
    tag: u32,
    host: &mut impl BackendLocks,
) -> Result<u32, Error> {
    let validation = crate::normalization::quad_check(object, tag, 0xcdac, 332);
    if validation >= 0x80000000 {
        return Ok(0x81202008);
    }
    let env = [m.word(object)?, m.word(object + 4)?];
    let null = [m.word(0x8d7660)?, m.word(0x8d7664)?];
    let status = if env == null {
        0
    } else {
        let rc = crate::frontend::validate_handle(env[0] != 0, env[1], 0x1e61, 32);
        if rc >= 0x80000000 {
            0x81602008
        } else {
            let mut magic = [0; 2];
            m.read(env[0], &mut magic)?;
            if u16::from_le_bytes(magic) == 0x1e61 {
                0
            } else {
                0x81602008
            }
        }
    };
    if status >= 0x80000000 {
        return Ok(status);
    }
    struct Hooks<'a, H> {
        host: &'a mut H,
        status: u32,
    }
    impl<H: BackendLocks> crate::frontend_part_a::SchedulerHooks for Hooks<'_, H> {
        fn resolve(&mut self, _: bool, _: [u32; 2]) -> (u32, u32, u32) {
            (self.status, 0, 64)
        }
        fn acquire(&mut self, p: u32) -> u32 {
            self.host.acquire(p)
        }
        fn release(&mut self, p: u32) -> u32 {
            self.host.release(p)
        }
        fn validate(&mut self, _: u32, _: u32, _: u32, _: u32) -> u32 {
            unreachable!()
        }
        fn close_channel(&mut self, _: u32, _: &mut u32) -> u32 {
            unreachable!()
        }
        fn enter(&mut self, _: &mut [u8], _: u32, _: u32, _: &[u8], _: &mut u8) -> u32 {
            unreachable!()
        }
        fn restore(&mut self, _: &mut [u8], _: u32, _: u32, _: u8) {
            unreachable!()
        }
        fn load(&mut self, _: &mut [u8], _: u32, _: &crate::frontend_part_a::ResourceLoad) -> u32 {
            unreachable!()
        }
        fn unload(&mut self, _: &mut [u8], _: u32, _: u32, _: u32) -> u32 {
            unreachable!()
        }
        fn resource_method(&mut self, _: &mut [u8], _: bool, _: [u32; 2], _: u32, _: u32) -> u32 {
            unreachable!()
        }
    }
    let mut local = [0; 0x200];
    m.read(object, &mut local[64..64 + 332])?;
    let before = local[64 + 0x30];
    let rc = crate::frontend_part_a::break_scheduler(
        &mut local,
        [object, tag],
        &mut Hooks { host, status },
    );
    if local[64 + 0x30] != before {
        m.write(object + 0x30, &local[64 + 0x30..64 + 0x31])?;
    }
    Ok(rc)
}

/// fn_194790 through the existing stop_synth port. Unknown methods and
/// unsupported scheduler closures remain explicit errors. Event-log growth
/// requires the allocator and inherited VFP bindings supplied by the caller.
fn running_stop(
    m: &mut Memory<'_, '_>,
    engine: u32,
    host: &mut impl BackendLocks,
) -> Result<(), Error> {
    struct Hooks<'a, 'b, 'c, H> {
        m: &'a mut Memory<'b, 'c>,
        engine: u32,
        host: &'a mut H,
        error: Option<Error>,
    }
    impl<H: BackendLocks> Hooks<'_, '_, '_, H> {
        fn result(&mut self, r: Result<u32, Error>) -> u32 {
            match r {
                Ok(v) => v,
                Err(e) => {
                    self.error = Some(e);
                    0x80002001
                }
            }
        }
    }
    impl<H: BackendLocks> crate::frontend::StopSynthHooks for Hooks<'_, '_, '_, H> {
        fn log_begin(&mut self, _: u32) {}
        fn log_end(&mut self, _: u32) {}
        fn log_error(&mut self, _: u32, _: u32, _: u32) {}
        fn acquire(&mut self, p: u32) -> u32 {
            self.host.acquire(p)
        }
        fn release(&mut self, p: u32) -> u32 {
            self.host.release(p)
        }
        fn flag_voice(&mut self, _: u32) -> u32 {
            let r = stop_event(self.m, self.engine, self.host);
            self.result(r)
        }
        fn call_stop(&mut self, a: u32, b: u32) -> u32 {
            let r = (|| {
                let target = self
                    .m
                    .word(self.m.word(self.m.word(self.engine + 4)? + 188)? + 64)?;
                if target != 0x15a4b8 {
                    return Err(Error::UnsupportedStage { entry: target });
                }
                scheduler_break(self.m, a, b, self.host)
            })();
            self.result(r)
        }
        fn stop_output(&mut self, _: u32) -> u32 {
            if self.error.is_some() {
                return 0x80002001;
            }
            let r = (|| {
                let lock = self.m.word(self.engine + 0x24)?;
                let mut rc = self.host.acquire(lock);
                if rc < 0x80000000 {
                    match byte(self.m, self.engine + 0x28)? {
                        3 | 4 => self.m.write(self.engine + 0x28, &[6])?,
                        5 | 6 => (),
                        _ => rc = 0x80002011,
                    }
                    self.host.release(lock);
                }
                Ok(rc)
            })();
            self.result(r)
        }
    }
    let mut local = [0; 0x200];
    m.read(engine, &mut local[64..64 + 0x188])?;
    // Only the logger parent needs rebasing for stop_synth's local reads.
    let logger = m.word(m.word(engine + 0x10)? + 16)?;
    put(&mut local, 64 + 0x10, 8);
    put(&mut local, 24, logger);
    let mut hooks = Hooks {
        m,
        engine,
        host,
        error: None,
    };
    crate::frontend::stop_synth(&mut hooks, &mut local, Some(64), 0x684f424a);
    match hooks.error {
        Some(e) => Err(e),
        None => Ok(()),
    }
}

/// Game song-parameter layout. Japanese requests reject singing upstream.
#[derive(Clone, Copy)]
pub(crate) enum SongRegion {
    Western,
    Korean,
}

/// Construct and parse the supported serialized game song tags, then resolve
/// note kinds through the regional ROM table. This composes fn_741654,
/// fn_425948/fn_425b74, fn_7442e4 and fn_744bd0, or their KR equivalents.
/// The reader's size prefix/newlines are not parameters. No captured output
/// enters this path. Output is caller-owned, and malformed input is rejected
/// before any write. This is conversion only, not the missing lyric generator.
pub(crate) fn convert_song(
    raw: &[u8],
    bpm: u8,
    stretch: u8,
    region: SongRegion,
    note_kinds: &[u16; 62],
    out: &mut [u8],
) -> Result<usize, Error> {
    let korean = matches!(region, SongRegion::Korean);
    let size = if korean { 0x240 } else { 0x234 };
    if raw.is_empty() || raw.len() > 254 || raw.len() % 2 != 0 || bpm == 0 {
        return Err(Error::InvalidSnapshot);
    }
    if out.len() < size {
        return Err(Error::InputTooLong);
    }
    // Validate the complete stream before clearing caller state.
    let mut at = 0;
    while at < raw.len() {
        let (group, index, bytes, _) = song_tag(raw, at)?;
        if !((group == 5 && index == 0 && bytes == 4)
            || (group == 12 && matches!(index, 14..=18 | 20..=21) && bytes == 2))
        {
            return Err(Error::InvalidSnapshot);
        }
        if group == 5 && raw[at + 10] == 0 {
            return Err(Error::InvalidSnapshot);
        }
        at += 10 + bytes;
    }
    out[..size].fill(0);
    put(out, 0x210, 1.0f32.to_bits());
    let voice = if korean { 0x224 } else { 0x218 };
    for o in [voice, voice + 4, voice + 8] {
        put(out, o, 10000);
    }
    out[voice + 14..voice + 16].copy_from_slice(&10000u16.to_le_bytes());
    // KR fn_170654 also zeroes its twelve-byte syllable array, already clear.
    let timing = if korean { 0x234 } else { 0x228 };
    let scale = if korean { 0x23c } else { 0x230 };
    let mut count = 0usize;
    let mut duration = 0i16;
    at = 0;
    while at < raw.len() {
        let (group, index, bytes, value) = song_tag(raw, at)?;
        if group == 12 && (15..=18).contains(&index) {
            let n = (value as i8 as i16).wrapping_mul(stretch as i16);
            let offset = timing + (index as usize - 15) * 2;
            out[offset..offset + 2].copy_from_slice(&n.to_le_bytes());
        } else {
            let (kind, value) = if group == 5 {
                let kind = raw[at + 9].min(54) as usize;
                let n = (raw[at + 10] as i32) * (stretch as i32);
                duration = duration.wrapping_add(n as i16);
                (kind, n)
            } else if index == 14 {
                let kind = if value <= 3 { 55 + value as usize } else { 0 };
                (kind, 0)
            } else {
                (if index == 20 { 60 } else { 61 }, value as i32)
            };
            if count >= 64 {
                return Err(Error::InputTooLong);
            }
            put(out, count * 8, note_kinds[kind] as u32);
            put(out, count * 8 + 4, value as u32);
            count += 1;
        }
        at += 10 + bytes;
    }
    out[0x200..0x202].copy_from_slice(&(count as u16).to_le_bytes());
    out[0x202..0x204].copy_from_slice(&duration.to_le_bytes());
    out[0x20c..0x20e].copy_from_slice(&((bpm as u16) * 2).to_le_bytes());
    out[scale] = stretch;
    Ok(size)
}

/// Validate one complete MSBT-style tag including its padding and newline.
fn song_tag(raw: &[u8], at: usize) -> Result<(u16, u16, usize, u8), Error> {
    if raw.len().saturating_sub(at) < 12 {
        return Err(Error::InvalidSnapshot);
    }
    let u16_at = |i| u16::from_le_bytes([raw[at + i], raw[at + i + 1]]);
    let bytes = u16_at(6) as usize;
    let end = at.checked_add(10 + bytes).ok_or(Error::InputTooLong)?;
    if u16_at(0) != 14
        || bytes == 0
        || end > raw.len()
        || raw[end - 3] != 0xcd
        || raw[end - 2..end] != [10, 0]
    {
        return Err(Error::InvalidSnapshot);
    }
    Ok((u16_at(2), u16_at(4), bytes, raw[at + 8]))
}

/// Caller-owned workspace for adapting the existing event-vector port to
/// discontiguous engine memory. Capacity is explicit, never a heap allocation.
pub struct EventWorkspace {
    bytes: [u8; 0xa400],
}
impl EventWorkspace {
    /// Initialize empty temporary storage; no persistent engine state lives here.
    pub const fn new() -> Self {
        Self { bytes: [0; 0xa400] }
    }
}

/// fn_183258 via the existing boot port. Supports up to 256 old and 256
/// grown entries. Allocator calls stay behind the host; copied old events,
/// capacity flags, inherited VFP fields and tick writes are all Rust work.
/// The workspace uses local addresses only; all published pointers are guest
/// addresses. Refused ownership/allocation and empty vectors follow the ROM.
pub fn append_event(
    m: &mut Memory<'_, '_>,
    speech: u32,
    kind: u32,
    vfp: [u32; 2],
    workspace: &mut EventWorkspace,
    host: &mut impl BackendLocks,
) -> Result<(), Error> {
    const OLD: u32 = 0x6400;
    const NEW: u32 = 0x8400;
    let begin = m.word(speech + 0x61d0)?;
    let end = m.word(speech + 0x61d4)?;
    let flags = m.word(speech + 0x61d8)?;
    let count = end.wrapping_sub(begin) / 32;
    let capacity = flags & 0x3fffffff;
    if count > 256 || (end.wrapping_sub(begin) % 32) != 0 {
        return Err(Error::InputTooLong);
    }
    let grow = count >= capacity;
    let grown = if capacity == 0 {
        1
    } else {
        capacity.wrapping_mul(2)
    };
    if grow && grown > 256 {
        return Err(Error::InputTooLong);
    }
    let allocator = m.word(speech + 0x61cc)?;
    let ticks = host.ticks()?;
    // fn_183258 returns without writing when growth is not owned.
    if grow && (allocator == 0 || flags & 0xc0000000 != 0x40000000) {
        return Ok(());
    }
    let b = &mut workspace.bytes;
    b.fill(0);
    put(b, 0x61cc, if allocator == 0 { 0 } else { 0x6200 });
    put(b, 0x61d0, if begin == 0 { 0 } else { OLD });
    put(b, 0x61d4, if begin == 0 { 0 } else { OLD + count * 32 });
    put(b, 0x61d8, flags);
    put(b, 0x6200, 0x6210);
    put(b, 0x6218, 1);
    put(b, 0x621c, 2);
    if count != 0 {
        m.read(
            begin,
            &mut b[OLD as usize..OLD as usize + count as usize * 32],
        )?;
    }
    // Preserve untouched tail bytes in the append slot.
    if !grow && end != 0 {
        m.read(
            end,
            &mut b[(OLD + count * 32) as usize..(OLD + count * 32 + 32) as usize],
        )?;
    }
    struct Bridge<'a, 'b, 'c, H> {
        memory: &'a mut Memory<'b, 'c>,
        host: &'a mut H,
        allocator: u32,
        old: u32,
        fresh: u32,
        ticks: u64,
        error: Option<Error>,
    }
    impl<H: BackendLocks> crate::boot_part_b::Host for Bridge<'_, '_, '_, H> {
        fn thread_id(&mut self) -> u32 {
            unreachable!("event append never reads thread ID")
        }
        fn call(
            &mut self,
            local: &mut crate::boot::Memory<'_>,
            function: u32,
            args: &[u32],
        ) -> u32 {
            let result = match function {
                1 => self
                    .host
                    .event_allocate(self.memory, self.allocator, args[1], args[2])
                    .and_then(|pointer| {
                        self.fresh = pointer;
                        // The ARM faults copying a nonempty old vector to a
                        // null allocation. Reject it rather than publish a
                        // successful event append to unmapped address zero.
                        if pointer == 0 {
                            return Err(Error::InvalidSnapshot);
                        }
                        self.memory
                            .read(pointer, &mut local.bytes[0x8400..0x8400 + args[1] as usize])?;
                        Ok(0x8400)
                    }),
                2 => self
                    .host
                    .event_release(self.memory, self.allocator, self.old)
                    .map(|()| 0),
                _ => Err(Error::InvalidSnapshot),
            };
            match result {
                Ok(x) => x,
                Err(e) => {
                    self.error = Some(e);
                    0
                }
            }
        }
    }
    impl<H: BackendLocks> crate::boot_part_b2::EventHost for Bridge<'_, '_, '_, H> {
        fn tick(&mut self, _: &mut crate::boot::Memory<'_>) -> u64 {
            self.ticks
        }
    }
    let mut bridge = Bridge {
        memory: m,
        host,
        allocator,
        old: begin,
        fresh: 0,
        ticks,
        error: None,
    };
    crate::boot_part_b2::fn_183258(
        &mut crate::boot::Memory { base: 0, bytes: b },
        0,
        kind,
        &crate::boot_part_b2::EventVfp {
            s16: vfp[0],
            s17: vfp[1],
        },
        &mut bridge,
    );
    if let Some(e) = bridge.error {
        return Err(e);
    }
    let fresh = bridge.fresh;
    let local_begin = word(b, 0x61d0);
    let local_end = word(b, 0x61d4);
    let result_begin = if grow { fresh } else { begin };
    let written = local_end.wrapping_sub(local_begin);
    if result_begin != 0 && written != 0 {
        m.write(
            result_begin,
            &b[local_begin as usize..(local_begin + written) as usize],
        )?;
    }
    m.put_word(speech + 0x61d0, result_begin)?;
    m.put_word(speech + 0x61d4, result_begin.wrapping_add(written))?;
    m.put_word(speech + 0x61d8, word(b, 0x61d8))?;
    Ok(())
}

/// Mapped fn_176054 using the existing store_sized port. Local offsets cover
/// only the value, source and heap descriptor; heap returns are translated
/// back to guest pointers. Nothing copies allocator results from recordings.
fn store_parameter(
    m: &mut Memory<'_, '_>,
    value: u32,
    text: &[u8],
    heap: u32,
    host: &mut impl BackendLocks,
) -> Result<(), Error> {
    let old = m.word(value)?;
    let old_size = m.word(value + 4)?;
    let mut local = [0u8; 128];
    put(&mut local, 8, 64);
    put(&mut local, 12, old_size);
    if heap != 0 {
        put(&mut local, 36, m.word(heap + 4)?);
    }
    local[48..48 + text.len()].copy_from_slice(text);
    struct Heap<'a, 'b, 'c, H> {
        m: &'a mut Memory<'b, 'c>,
        host: &'a mut H,
        old: u32,
        returned: u32,
        error: Option<Error>,
    }
    impl<H: BackendLocks> crate::frontend::HeapVtable for Heap<'_, '_, '_, H> {
        fn alloc_at(&mut self, context: u32, size: u32) -> u32 {
            match self.host.parameter_resize(self.m, context, 0, size) {
                Ok(p) => {
                    self.returned = p;
                    if p == 0 {
                        0
                    } else {
                        80
                    }
                }
                Err(e) => {
                    self.error = Some(e);
                    0
                }
            }
        }
        fn realloc_at(&mut self, context: u32, _: u32, size: u32) -> u32 {
            match self.host.parameter_resize(self.m, context, self.old, size) {
                Ok(p) => {
                    self.returned = p;
                    if p == 0 {
                        0
                    } else {
                        80
                    }
                }
                Err(e) => {
                    self.error = Some(e);
                    0
                }
            }
        }
        fn release_at(&mut self, _: u32, _: u32) {
            unreachable!("rate is never empty")
        }
    }
    let mut bridge = Heap {
        m,
        host,
        old,
        returned: old,
        error: None,
    };
    let rc = crate::frontend::store_sized(
        &mut local,
        Some(8),
        48,
        text.len() as u32,
        if heap == 0 { 0 } else { 32 },
        &mut bridge,
    );
    if let Some(e) = bridge.error {
        return Err(e);
    }
    let returned = bridge.returned;
    if old_size == 0 || rc < 0x80000000 {
        m.put_word(value, returned)?;
    }
    if rc >= 0x80000000 {
        return Err(Error::EngineStatus(rc));
    }
    let data = word(&local, 8) as usize;
    m.write(returned, &local[data..data + text.len()])?;
    m.put_word(value + 4, text.len() as u32)
}

// Historical song tests include this source as a module. Keep the production
// bridge in the normal library, which recorded_live_controls exercises directly.
#[cfg(not(test))]
pub use live_controls::LiveControls;
#[cfg(not(test))]
mod live_controls {
use super::*;
/// Production bindings for parked controls. All arenas, scratch, inherited
/// VFP registers and OS services belong to the caller. No recorded replies.
pub struct LiveControls<'a> {
    pub workspace: &'a mut [u8],
    /// 128 disjoint mapped bytes, restored after each synchronous invocation.
    pub scratch: u32,
    pub platform: &'a mut dyn crate::engine::HeapPlatform,
    pub tick: u64,
    pub vfp: [u32; 2],
}
struct ControlPlatform<'a>(&'a mut dyn crate::engine::HeapPlatform);
impl crate::bridge::Platform for ControlPlatform<'_> {
    fn thread_id(&mut self) -> u32 { self.0.thread_id() }
    fn arbiter_address(&self) -> u32 { self.0.arbiter_address() }
    fn arbitrate(&mut self, m: &mut crate::boot::Memory<'_>, a: &[u32]) -> u32 {
        self.0.arbitrate(m, a)
    }
}
/// Only map value/key construction lives here. Heap operations are the real
/// bridge ports; fn_176054 and strcmp use the existing frontend ports.
struct ParameterMap<'a> { host: crate::engine::SpeechBootHost<'a> }
impl crate::bridge::Live for ParameterMap<'_> {
    fn log<P: crate::bridge::Platform>(&mut self, f: &mut crate::bridge::Flat<'_, '_, '_, '_, P>,
        site: u32, _: u32, _: &[u32]) { f.reject(site); }
    fn call<P: crate::bridge::Platform>(&mut self, f: &mut crate::bridge::Flat<'_, '_, '_, '_, P>,
        entry: u32, a: &[u32]) -> u32 {
        match entry {
            0x196980 => {
                let m = f.memory();
                crate::frontend::compare_str(&m.bytes[a[0] as usize..], &m.bytes[a[1] as usize..]) as u32
            }
            0x19690c => {
                if a.iter().any(|v| *v == 0) { return 0x89002007; }
                let n = crate::frontend::strlen_bytes(&f.memory().bytes[a[0] as usize..]) + 1;
                let p = f.fn_196394(a[1], n);
                if p == 0 { return 0x8900200a; }
                crate::frontend::copy_str(f.memory().bytes, p, a[0]);
                f.put_word(a[2], p);
                0
            }
            0x174a04 => {
                let rc=f.memory().fn_174a04(&mut ValueCalls(&mut self.host),a[0],a[1],a[2]);
                if let Some(error)=self.host.fault.take() {
                    let entry=match error {Error::UnsupportedStage {entry}|Error::MissingStreamAdapter {entry}=>entry,_=>0x174a04};
                    return f.reject(entry);
                }
                rc
            }
            0x196964 => { f.fn_196428(a[1], a[0]); 0 }
            0x1749ac => {
                if a[0] != 0 {
                    if f.word(a[0]+8) != 0 {return f.reject(0x196c7c);}
                    let p = f.word(a[0]);
                    f.fn_196428(a[1], p);
                    f.put_word(a[0],0);f.put_word(a[0]+4,0);
                    f.fn_196428(a[1], a[0]);
                }
                0
            }
            _ => f.reject(entry),
        }
    }
}
impl LiveControls<'_> {
    fn lock(&mut self,m:&mut Memory<'_, '_>,p:u32,release:bool)->Result<u32,Error> {
        if p==0 {return Ok(0x81402007);}
        let method=m.word(m.word(m.word(p)?+8)?+if release {12}else{8})?;
        if method==0 {return Ok(0);}
        let target=m.word(p+8)?;
        crate::engine::speech1_boot_stage(m,crate::engine::StageCall {entry:method,args:[target,0,0,0,0,0,0]},self.workspace,self.platform).map(|r|r.r0.unwrap_or(0))
    }
    fn heap(&mut self, m: &mut Memory<'_, '_>, entry:u32, a:&[u32])->Result<u32,Error> {
        let mut os=ControlPlatform(self.platform);
        let mut f=crate::bridge::Flat::new(m,self.workspace,&mut os,0)?;
        let p=match entry {
            0x196394=>f.fn_196394(a[0],a[1]),
            0x1963e0=>f.fn_1963e0(a[0],a[1],a[2]),
            0x196428=>{f.fn_196428(a[0],a[1]);0},
            _=>f.allocator_method(entry,a),
        };
        f.finish()?;
        Ok(p)
    }
}
impl BackendLocks for LiveControls<'_> {
    fn acquire(&mut self, _:u32)->u32 {unreachable!("use memory-passing lock")}
    fn release(&mut self, _:u32)->u32 {unreachable!("use memory-passing lock")}
    fn acquire_m(&mut self,m:&mut Memory<'_, '_>,p:u32)->Result<u32,Error> {self.lock(m,p,false)}
    fn release_m(&mut self,m:&mut Memory<'_, '_>,p:u32)->Result<u32,Error> {self.lock(m,p,true)}
    fn ticks(&mut self)->Result<u64,Error> {Ok(self.tick)}
    fn event_vfp(&mut self)->Option<[u32;2]> {Some(self.vfp)}
    fn event_allocate(&mut self,m:&mut Memory<'_, '_>,a:u32,n:u32,align:u32)->Result<u32,Error> {
        let target=m.word(m.word(a)?+8)?;
        self.heap(m,target,&[a,n,align])
    }
    fn event_release(&mut self,m:&mut Memory<'_, '_>,a:u32,p:u32)->Result<(),Error> {
        let target=m.word(m.word(a)?+12)?;
        self.heap(m,target,&[a,p]).map(|_|())
    }
    fn parameter_resize(&mut self,m:&mut Memory<'_, '_>,a:u32,p:u32,n:u32)->Result<u32,Error> {
        if p==0 {let target=m.word(m.word(a)?+8)?;self.heap(m,target,&[a,n,4])}
        else {self.heap(m,0x4a79b8,&[a,p,n])}
    }
    fn parameter_insert(&mut self,m:&mut Memory<'_, '_>,map:u32,text:&[u8])->Result<u32,Error> {
        let s=self.scratch;
        let mut saved=[0;128];m.read(s,&mut saved)?;
        let result=(|| {
            m.write(s,b"rate\0")?;m.write(s+8,text)?;
            m.put_word(s+16,s+8)?;m.put_word(s+20,text.len() as u32)?;m.put_word(s+24,0)?;
            // The tree and its callbacks share only the OS dispatcher. Every
            // guest call reborrows the current Flat, never another staged arena.
            let cell=core::cell::RefCell::new(&mut *self.platform);
            let mut os=SharedPlatform(&cell);let mut callbacks=SharedPlatform(&cell);
            let mut f=crate::bridge::Flat::new(m,self.workspace,&mut os,0)?;
            let mut host=ParameterMap {host:crate::engine::SpeechBootHost {platform:&mut callbacks,fault:None}};
            let rc=f.fn_19668c([map,s,s+16],s+32,&mut host);
            f.finish()?;
            Ok(rc)
        })();
        m.write(s,&saved)?;
        result
    }
    fn parameter_trace(&mut self,m:&mut Memory<'_, '_>,engine:u32,status:Option<u32>,end:bool)->Result<(),Error> {
        let logger=m.word(m.word(engine+16)?+16)?;
        let s=self.scratch;
        let mut saved=[0;128];m.read(s,&mut saved)?;
        // US fn_19310c literals. These are addresses, never captured output.
        let call=if let Some(status)=status {
            m.put_word(s,0x193bf8)?;m.put_word(s+4,3)?;
            m.put_word(s+8,0x193bf0)?;m.put_word(s+12,status)?;
            crate::engine::StageCall {entry:0x173dc0,args:[logger,0x8d2e04,0x4663,0x193c04,s,s+32,0]}
        } else {
            crate::engine::StageCall {entry:0x1738e8,args:[logger,0x8d2e04,4,0,if end {0x193d70}else{0x193450},s,0]}
        };
        let result=(|| {
            let mut os=ViewOnly;
            let mut f=crate::bridge::Flat::new(m,self.workspace,&mut os,0)?;
            let mut host=ControlLog {inner:crate::engine::SpeechBootHost {platform:self.platform,fault:None}};
            let a=call.args;
            if call.entry==0x173dc0 {
                crate::render_part_a::error_log(&mut f.memory(),&mut host,a[0],a[1],a[2],a[3],a[4],a[5]);
            } else {
                crate::log_live::fn_1738e8(&mut f.memory(),a[0],a[1],a[2],a[3],a[4],a[5],&mut host);
            }
            f.finish()?;
            if let Some(e)=host.inner.fault {return Err(e);}
            Ok(())
        })();
        m.write(s,&saved)?;
        result
    }
}

struct ViewOnly;
impl crate::bridge::Platform for ViewOnly {
    fn thread_id(&mut self)->u32 {unreachable!()}
    fn arbiter_address(&self)->u32 {unreachable!()}
    fn arbitrate(&mut self,_:&mut crate::boot::Memory<'_>,_:&[u32])->u32 {unreachable!()}
}
struct ControlLog<'a> {inner:crate::engine::SpeechBootHost<'a>}
impl ControlLog<'_> {
    fn invoke(&mut self,m:&mut crate::boot::Memory<'_>,entry:u32,a:&[u32])->u32 {
        match entry {
            0x1ce428 => {
                // This capture's file logger is disabled in live state. Run
                // the real method, rejecting active unbound dependencies.
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
                    self.inner.fault=Some(Error::UnsupportedStage {entry:0x232354});return 0x80002001;
                }
                crate::frontend_part_b::fn_1ce428(m.bytes,a[0],a[1],a[2],a[3],a[4],a[5],&mut Disabled)
            }
            0x18f3fc => {
                let target=m.word(a[0]);
                if target!=0x4a7adc {
                    self.inner.fault=Some(Error::UnsupportedStage {entry:target});return 0x80002001;
                }
                // fn_4a7adc is exactly bx lr, src/fn_4a7adc.c.
                crate::frontend::notify([a[2],a[3],a[4],a[5]],|_|0)
            }
            _=>self.inner.invoke(m,entry,a),
        }
    }
}
impl crate::boot_part_b::Host for ControlLog<'_> {
    fn thread_id(&mut self)->u32 {crate::boot_part_b::Host::thread_id(&mut self.inner)}
    fn call(&mut self,m:&mut crate::boot::Memory<'_>,e:u32,a:&[u32])->u32 {self.invoke(m,e,a)}
}
impl crate::boot_part_b::FormatHost for ControlLog<'_> {}
impl crate::log_live::LogMethodsM for ControlLog<'_> {
    fn log_m(&mut self,e:u32,o:u32,c:u32,l:u32,a:u32,f:u32,v:u32,m:&mut crate::boot::Memory<'_>)->u32 {
        self.invoke(m,e,&[o,c,l,a,f,v])
    }
}

struct SharedPlatform<'a,'b,'c>(&'a core::cell::RefCell<&'b mut (dyn crate::engine::HeapPlatform + 'c)>);
impl crate::engine::HeapPlatform for SharedPlatform<'_, '_, '_> {
    fn thread_id(&mut self)->u32 {self.0.borrow_mut().thread_id()}
    fn arbiter_address(&self)->u32 {self.0.borrow().arbiter_address()}
    fn arbitrate(&mut self,m:&mut crate::boot::Memory<'_>,a:&[u32])->u32 {self.0.borrow_mut().arbitrate(m,a)}
}
impl crate::bridge::Platform for SharedPlatform<'_, '_, '_> {
    fn thread_id(&mut self)->u32 {crate::engine::HeapPlatform::thread_id(self)}
    fn arbiter_address(&self)->u32 {crate::engine::HeapPlatform::arbiter_address(self)}
    fn arbitrate(&mut self,m:&mut crate::boot::Memory<'_>,a:&[u32])->u32 {crate::engine::HeapPlatform::arbitrate(self,m,a)}
}
struct ValueCalls<'a,'b>(&'a mut crate::engine::SpeechBootHost<'b>);
impl crate::boot::Calls for ValueCalls<'_, '_> {
    fn call(&mut self,m:&mut crate::boot::Memory<'_>,entry:u32,a:&[u32])->u32 {
        if entry!=0x176054 {return self.0.invoke(m,entry,a);}
        // fn_174a04 has just zero-allocated this Value. Dispatch its real
        // data allocation before the existing fn_176054 port copies bytes.
        if m.word(a[0]+4)!=0 {
            self.0.fault=Some(Error::UnsupportedStage {entry:0x176054});return 0x8b702007;
        }
        let p=if a[2]==0 {0} else {self.0.invoke(m,0x196394,&[a[3],a[2]])};
        struct Alloc(u32);
        impl crate::frontend::HeapVtable for Alloc {
            fn alloc_at(&mut self,_:u32,_:u32)->u32 {self.0}
            fn realloc_at(&mut self,_:u32,_:u32,_:u32)->u32 {unreachable!("fresh clone")}
            fn release_at(&mut self,_:u32,_:u32) {unreachable!("fresh clone")}
        }
        crate::frontend::store_sized(m.bytes,Some(a[0]),a[1],a[2],a[3],&mut Alloc(p))
    }
    fn call_wide(&mut self,_:&mut crate::boot::Memory<'_>,e:u32,_:&[u32])->u64 {
        self.0.fault=Some(Error::UnsupportedStage {entry:e});0
    }
}

}
