

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct SynthEnvironment {
    pub heap: u32,
    pub log: u32,
    pub pool: u32,
    pub parameters: u32,
    pub resources: u32,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct Synth {
    pub address: u32,
    pub session: [u32; 2],
    pub environment: u32,
    pub lock: u32,
    pub streams: u32,
    pub buffer_samples: u32,
    pub frame_samples: u32,
    pub sample_rate: i32,
    pub excitation: i32,
    pub volume: u16,
    pub quality: u8,
    pub reserved27: u8,
    pub engine: u32,
    pub equalizer_handle: u32,
    pub equalizer_data: u32,
    pub equalizer_count: u32,
    pub equalizer_rate: i32,
    pub reserved3c: u32,
    pub speech_input: [u32; 2],
    pub audio_output: [u32; 2],
    pub marker_input: [u32; 2],
    pub marker_output: [u32; 2],
    pub current: u32,
    pub frame_index: i32,
    pub frames_processed: u32,
    pub samples_processed: u32,
}

impl Synth {

    pub fn words(&self) -> [u32; 22] {
        [
            self.session[0],
            self.session[1],
            self.environment,
            self.lock,
            self.streams,
            self.buffer_samples,
            self.frame_samples,
            self.sample_rate as u32,
            self.excitation as u32,
            self.volume as u32
                | (u32::from(self.quality) << 16)
                | (u32::from(self.reserved27) << 24),
            self.engine,
            self.equalizer_handle,
            self.equalizer_data,
            self.equalizer_count,
            self.equalizer_rate as u32,
            self.reserved3c,
            self.speech_input[0],
            self.speech_input[1],
            self.audio_output[0],
            self.audio_output[1],
            self.marker_input[0],
            self.marker_input[1],
        ]
    }
}

pub const PARAM_BUFFER_SAMPLES: &[u8] = b"audiooutputbufsamples";

pub const ENV_TOKEN: u32 = 0x0e00_0200;

pub trait SynthHost {
    fn validate(&mut self, object: u32, tag: u32, kind: u32, size: u32) -> u32;
    fn resolve_env(&mut self, session: [u32; 2]) -> (u32, Option<SynthEnvironment>);
    fn environment(&mut self, token: u32) -> SynthEnvironment;
    fn log_module(&mut self, log: u32, code: u32);
    fn log_error(&mut self, log: u32, code: u32);
    fn release_parameter(&mut self, params: u32, name: &[u8], owner: u32) -> u32;
    fn release_resource(&mut self, res: u32, name: &[u8]) -> u32;
    fn destroy_engine(&mut self, engine: u32);
    fn destroy_equalizer(&mut self, handle: u32);
    fn destroy_lock(&mut self, lock: u32) -> u32;
    fn free_object(&mut self, heap: u32, object: u32);
    fn open_synth_object(&mut self, heap: u32, count: u32, size: u32) -> u32;
    fn open_stream(
        &mut self,
        handle: [u32; 2],
        kind: u32,
        mime: &[u8],
        extra: u32,
        mode: u32,
        out: &mut [u32; 2],
    ) -> u32;
    fn close_stream(&mut self, handle: [u32; 2]) -> u32;
    fn empty_handle(&mut self) -> [u32; 2];
    fn marker_format(&mut self) -> u32;
    fn create_synth_engine(&mut self, engine: u32) -> i32;
    fn acquire_resource(&mut self, res: u32, name: &[u8], out: &mut u32) -> u32;
    fn open_lock(&mut self, pool: u32, heap: u32, out: &mut u32) -> u32;
    fn register_parameter(
        &mut self,
        params: u32,
        name: &[u8],
        minimum: i32,
        maximum: i32,
        fallback: i32,
        owner: u32,
        value: &mut i32,
    ) -> u32;
    fn rebuild_engine(&mut self, synth: &mut Synth) -> u32;
}

fn write_decimal(buffer: &mut [u8], mut value: u32) {
    let mut digits = [0u8; 12];
    let mut count = 0;
    loop {
        digits[count] = b'0' + (value % 10) as u8;
        count += 1;
        value /= 10;
        if value == 0 {
            break;
        }
    }
    for index in 0..count {
        buffer[index] = digits[count - 1 - index];
    }
}

pub fn close_streams(synth: &mut Synth, tag: u32, host: &mut impl SynthHost) -> u32 {
    let result = host.validate(synth.address, tag, 0x4e7, 0x70);
    if result >= 0x80000000 {
        return 0x84e02008;
    }
    synth.samples_processed = 0;
    synth.frames_processed = 0;
    let mut status = 0;
    for (index, handle) in [
        &mut synth.speech_input,
        &mut synth.marker_input,
        &mut synth.audio_output,
        &mut synth.marker_output,
    ]
    .into_iter()
    .enumerate()
    {
        if handle[0] != 0 {
            let result = host.close_stream(*handle);
            if index == 0 || (status < 0x80000000 && result >= 0x80000000) {
                status = result;
            }
            *handle = host.empty_handle();
        }
    }
    status
}

pub fn close_synth(synth: &mut Synth, tag: u32, host: &mut impl SynthHost) -> u32 {
    let mut result = host.validate(synth.address, tag, 0x4e7, 0x70);
    if result >= 0x80000000 {
        return 0x84e02008;
    }
    let env = host.environment(synth.environment);
    host.log_module(env.log, 4);
    let owner = synth.address;
    for name in [
        b"volume".as_slice(),
        b"rate",
        b"pitch",
        PARAM_BUFFER_SAMPLES,
    ] {
        let status = host.release_parameter(env.parameters, name, owner);
        if result < 0x80000000 && status >= 0x80000000 {
            result = status;
        }
    }
    let status = host.release_resource(env.resources, b"SYNTHSTREAM");
    if result < 0x80000000 && status >= 0x80000000 {
        result = status;
    }
    if synth.engine != 0 {
        host.destroy_engine(synth.engine);
        synth.engine = 0;
    }
    if synth.equalizer_handle != 0 {
        host.destroy_equalizer(synth.equalizer_handle);
        synth.equalizer_handle = 0;
    }
    let status = host.destroy_lock(synth.lock);
    if result < 0x80000000 && status >= 0x80000000 {
        result = status;
    }
    host.free_object(env.heap, synth.address);
    host.log_module(env.log, 4);
    result
}

pub fn open_streams(
    synth: &mut Synth,
    tag: u32,
    stream: [u32; 2],
    host: &mut impl SynthHost,
) -> u32 {
    let mut result = host.validate(synth.address, tag, 0x4e7, 0x70);
    if result >= 0x80000000 {
        return 0x84e02008;
    }
    let env = host.environment(synth.environment);
    let mut content_type = [0u8; 64];
    let prefix = b"audio/L16;rate=";
    content_type[..prefix.len()].copy_from_slice(prefix);
    let mut digits = [0u8; 12];
    write_decimal(&mut digits, synth.sample_rate as u32);
    let count = digits.iter().position(|&byte| byte == 0).unwrap_or(12);
    content_type[prefix.len()..prefix.len() + count].copy_from_slice(&digits[..count]);
    let content_type = &content_type[..prefix.len() + count];
    result = host.open_stream(
        stream,
        0x10,
        b"application/x-realspeak-bet2-speech-frames;version=5.0",
        0,
        0,
        &mut synth.speech_input,
    );
    if result >= 0x80000000 {
        host.log_error(env.log, 0x6593);
        return finish(synth, tag, result, host);
    }
    result = host.open_stream(
        stream,
        0x10,
        b"application/x-realspeak-markers-pp;version=4.0",
        0,
        0,
        &mut synth.marker_input,
    );
    if result >= 0x80000000 {
        host.log_error(env.log, 0x6593);
        return finish(synth, tag, result, host);
    }
    result = host.open_stream(stream, 0x10, content_type, 0, 1, &mut synth.audio_output);
    if result >= 0x80000000 {
        host.log_error(env.log, 0x6594);
        return finish(synth, tag, result, host);
    }
    let format = host.marker_format();
    result = host.open_stream(
        stream,
        0x10,
        b"application/x-realspeak-markers-pp;version=4.0",
        format,
        1,
        &mut synth.marker_output,
    );
    if result >= 0x80000000 {
        host.log_error(env.log, 0x6594);
    } else {
        result = if host.create_synth_engine(synth.engine) != 0 {
            0x84e02000
        } else {
            0
        };
    }
    finish(synth, tag, result, host)
}

fn finish(synth: &mut Synth, tag: u32, result: u32, host: &mut impl SynthHost) -> u32 {
    synth.frame_index = 0;
    synth.current = 0;
    if result >= 0x80000000 {
        close_streams(synth, tag, host);
    }
    result
}

pub const VOICE_VERSION: u32 = 0x00030205;

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct EqualizerView {
    pub data: u32,
    pub sample_rate: i32,
    pub count: u32,
}

pub trait RebuildHost {
    fn get_int(&mut self, params: u32, name: &[u8], value: &mut i32) -> u32;
    fn get_string(&mut self, params: u32, name: &[u8], out: &mut [u8]) -> (u32, Option<usize>);
    fn subscribe(&mut self, params: u32, value: &[u8]);
    fn log_error(&mut self, log: u32, code: u32);
    fn open_voice(&mut self, session: [u32; 2], path: &[u8], version: u32) -> (u32, u32);
    fn destroy_engine(&mut self, engine: u32);
    fn destroy_resource(&mut self, handle: u32);
    fn parse_value(&mut self, text: &[u8]) -> i32;
    fn divide(&mut self, numerator: u32, denominator: u32) -> (u32, u32);
    fn read_entry(&mut self, handle: u32) -> (u32, [u8; 16], u32, i32, i32, u32);
    fn close_entry(&mut self, handle: u32) -> u32;
    fn acquire_equalizer(&mut self, handle: u32) -> (u32, u32, u32);
    fn release(&mut self, handle: u32) -> u32;
    fn create_engine(&mut self, workshop: &EngineWorkshop<'_>) -> i32;
    fn set_engine_int(&mut self, engine: u32, which: u32, value: i32) -> i32;
    fn set_synth_param(&mut self, synth: &mut Synth, name: &[u8], value: u32) -> u32;
}

pub struct EngineWorkshop<'a> {
    pub voice: u32,
    pub frame_size: i32,
    pub parameter: i32,
    pub excitation: i32,
    pub frequency: i32,
    pub cpu_power: u8,
    pub init: &'a [u8],
    pub has_equalizer: bool,
}

fn cstr<'a>(bytes: &'a [u8], len: usize) -> &'a [u8] {
    let len = len.min(bytes.len());
    &bytes[..len]
}

#[allow(clippy::too_many_arguments)]
pub fn rebuild_synth(
    synth: &mut Synth,
    eq: &mut EqualizerView,
    env: &SynthEnvironment,
    scratch: &mut [u8],
    vsynth: &mut [u8],
    host: &mut impl RebuildHost,
) -> u32 {
    let mut frequency = 0i32;
    let mut result = host.get_int(env.parameters, b"frequencyhz", &mut frequency);
    if result >= 0x80000000 {
        host.log_error(env.log, 0x659b);
        return result;
    }
    let mut path = [0u8; 64];
    path[..5].copy_from_slice(b"synth");
    let mut path_len = 5usize;
    let (status, quality) = host.get_string(env.parameters, b"bet2voicequality", scratch);
    if status < 0x80000000 {
        if let Some(len) = quality {
            let text = cstr(scratch, len);
            if !text.is_empty() && text[0] != 0 {
                path[path_len] = b'/';
                path_len += 1;
                let len = text.iter().position(|&b| b == 0).unwrap_or(text.len());
                path[path_len..path_len + len].copy_from_slice(&text[..len]);
                path_len += len;
                host.subscribe(env.parameters, text);
            } else {
                path[path_len..path_len + 4].copy_from_slice(b"/med");
                path_len += 4;
            }
        } else {
            path[path_len..path_len + 4].copy_from_slice(b"/med");
            path_len += 4;
        }
    } else {
        path[path_len..path_len + 4].copy_from_slice(b"/med");
        path_len += 4;
    }
    path[path_len..path_len + 4].copy_from_slice(b"/fxd");
    path_len += 4;
    path[path_len..path_len + 6].copy_from_slice(b"/bet2f");
    path_len += 6;
    let mut digits = [0u8; 12];
    write_decimal(&mut digits, (frequency / 1000) as u32);
    let count = digits.iter().position(|&byte| byte == 0).unwrap_or(12);
    path[path_len..path_len + count].copy_from_slice(&digits[..count]);
    path_len += count;
    let (status, voice) = host.open_voice(synth.session, &path[..path_len], VOICE_VERSION);
    result = status;
    if result >= 0x80000000 {
        return result;
    }
    let mut equalizer_data = 0u32;
    if synth.engine != 0 {
        host.destroy_engine(synth.engine);
        synth.engine = 0;
    }
    if synth.equalizer_handle != 0 {
        host.destroy_resource(synth.equalizer_handle);
        synth.equalizer_handle = 0;
    }
    synth.sample_rate = frequency;
    macro_rules! voice_param {
        ($key:expr) => {{
            let (status, len) = host.get_string(env.parameters, $key, scratch);
            result = status;
            if status >= 0x80000000 {
                host.log_error(env.log, 0x659b);
                return finish_rebuild(result, voice, equalizer_data, host);
            }
            let Some(len) = len else {
                host.log_error(env.log, 0x659b);
                return finish_rebuild(result, voice, equalizer_data, host);
            };
            let text = cstr(scratch, len);
            if text.is_empty() || text[0] == 0 {
                host.log_error(env.log, 0x659b);
                return finish_rebuild(result, voice, equalizer_data, host);
            }
            let parsed = host.parse_value(text);
            host.subscribe(env.parameters, text);
            parsed
        }};
    }
    let (status, excite_len) = host.get_string(env.parameters, b"bet2voiceexcitetype", scratch);
    result = status;
    let Some(excite_len) = excite_len else {
        host.log_error(env.log, 0x659b);
        return finish_rebuild(result, voice, equalizer_data, host);
    };
    let excite = cstr(scratch, excite_len);
    if excite.is_empty() || excite[0] == 0 {
        host.log_error(env.log, 0x659b);
        return finish_rebuild(result, voice, equalizer_data, host);
    }
    synth.quality = host.parse_value(excite) as u8;
    host.subscribe(env.parameters, excite);
    let excitation_type = voice_param!(b"bet2voicealpha");
    let mut frame_size = 0i32;
    result = host.get_int(env.parameters, b"bet2voiceframesize", &mut frame_size);
    if result >= 0x80000000 {
        host.log_error(env.log, 0x659b);
        return finish_rebuild(result, voice, equalizer_data, host);
    }

    synth.frame_samples =
        (synth.sample_rate as u32).wrapping_mul(frame_size as u32) / 1000;
    if synth.frame_samples != 0 {
        let (_, remainder) = host.divide(synth.buffer_samples, synth.frame_samples);
        if remainder != 0 {
            synth.buffer_samples = synth
                .buffer_samples
                .wrapping_add(synth.frame_samples)
                .wrapping_sub(remainder);
        }
    }
    synth.excitation = voice_param!(b"bet2voicevol");
    let vuv_ramp = voice_param!(b"bet2voicevuvrelamp");
    let mvf_boost = voice_param!(b"bet2voicemvfboost");
    let parameter = voice_param!(b"bet2voiceorder");
    let (_, vsynth_len) = host.get_string(env.parameters, b"bet2voicevsynthinit", vsynth);
    let (_, power_len) = host.get_string(env.parameters, b"bet2cpupower", scratch);
    let mut cpu_power = 1u8;
    if let Some(len) = power_len {
        let power = cstr(scratch, len);
        if !power.is_empty() && power[0] != 0 {
            let parsed = host.parse_value(power);
            if (0..=4).contains(&parsed) {
                cpu_power = parsed as u8;
            }
            host.subscribe(env.parameters, power);
        }
    }
    let mut equalizer_setting = 0i32;
    let mut has_equalizer = false;
    if host.get_int(
        env.parameters,
        b"bet2equalizersetting",
        &mut equalizer_setting,
    ) < 0x80000000
        && equalizer_setting > 0
    {
        let (status, data) = host.open_voice(synth.session, b"equalizer/bet2", VOICE_VERSION);
        result = status;
        if result >= 0x80000000 {
            return result;
        }
        equalizer_data = data;
        loop {
            let (status, name, size, setting, rate, token) = host.read_entry(equalizer_data);
            result = status;
            if result >= 0x80000000 {
                break;
            }
            let _ = token;
            let name_len = name.iter().position(|&b| b == 0).unwrap_or(name.len());
            if name[..name_len] == *b"EQUS"
                && setting == equalizer_setting
                && size > 8
                && rate == synth.sample_rate
            {
                let (status, handle, blob) = host.acquire_equalizer(equalizer_data);
                result = status;
                if result >= 0x80000000 {
                    break;
                }
                synth.equalizer_handle = handle;
                eq.data = blob.wrapping_add(8);
                eq.sample_rate = synth.sample_rate;
                eq.count = (size >> 2).wrapping_sub(2);
                synth.equalizer_data = eq.data;
                synth.equalizer_count = eq.count;
                synth.equalizer_rate = eq.sample_rate;
                has_equalizer = true;
                break;
            }
            result = host.close_entry(equalizer_data);
            if result >= 0x80000000 {
                break;
            }
        }
        if !has_equalizer {
            host.log_error(env.log, 0x659e);
            if result >= 0x80000000 {
                return finish_rebuild(result, voice, equalizer_data, host);
            }
        }
    }
    let init = match vsynth_len {
        Some(len) => cstr(vsynth, len),
        None => &[],
    };
    let workshop = EngineWorkshop {
        voice,
        frame_size,
        parameter,
        excitation: excitation_type,
        frequency,
        cpu_power,
        init,
        has_equalizer,
    };
    if host.create_engine(&workshop) != 0 {
        result = 0x84e02000;
        return finish_rebuild(result, voice, equalizer_data, host);
    }
    if host.set_engine_int(synth.engine, 0, vuv_ramp) != 0 {
        result = 0x84e02000;
    } else if host.set_engine_int(synth.engine, 1, mvf_boost) != 0 {
        result = 0x84e02000;
    }
    if host.set_engine_int(synth.engine, 2, synth.excitation) != 0 {
        result = 0x84e02000;
        return finish_rebuild(result, voice, equalizer_data, host);
    }
    if result >= 0x80000000 {
        return finish_rebuild(result, voice, equalizer_data, host);
    }
    let mut parameter_value = 0i32;
    result = host.get_int(env.parameters, b"volume", &mut parameter_value);
    if result >= 0x80000000 {
        return finish_rebuild(result, voice, equalizer_data, host);
    }
    result = host.set_synth_param(synth, b"volume", parameter_value as u32);
    if result >= 0x80000000 {
        return finish_rebuild(result, voice, equalizer_data, host);
    }
    result = host.get_int(env.parameters, b"rate", &mut parameter_value);
    if result >= 0x80000000 {
        return finish_rebuild(result, voice, equalizer_data, host);
    }
    result = host.set_synth_param(synth, b"rate", parameter_value as u32);
    if result >= 0x80000000 {
        return finish_rebuild(result, voice, equalizer_data, host);
    }
    result = host.get_int(env.parameters, b"pitch", &mut parameter_value);
    if result < 0x80000000 {
        result = host.set_synth_param(synth, b"pitch", parameter_value as u32);
    }
    finish_rebuild(result, voice, equalizer_data, host)
}

fn finish_rebuild(mut result: u32, voice: u32, equalizer: u32, host: &mut impl RebuildHost) -> u32 {
    if voice != 0 {
        let status = host.release(voice);
        if status >= 0x80000000 && result < 0x80000000 {
            result = status;
        }
    }
    if equalizer != 0 {
        let status = host.release(equalizer);
        if status >= 0x80000000 && result < 0x80000000 {
            result = status;
        }
    }
    result
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct SynthMarker {
    pub typ: u16,
    pub sample: u32,
    pub reserved: u32,
    pub value: i32,
}

impl SynthMarker {
    pub fn load(bytes: &[u8]) -> Self {
        SynthMarker {
            typ: u16::from_le_bytes([bytes[0], bytes[1]]),
            sample: u32::from_le_bytes([bytes[12], bytes[13], bytes[14], bytes[15]]),
            reserved: u32::from_le_bytes([bytes[16], bytes[17], bytes[18], bytes[19]]),
            value: i32::from_le_bytes([bytes[20], bytes[21], bytes[22], bytes[23]]),
        }
    }
    pub fn store(&self, bytes: &mut [u8]) {
        bytes[0..2].copy_from_slice(&self.typ.to_le_bytes());
        bytes[12..16].copy_from_slice(&self.sample.to_le_bytes());
        bytes[16..20].copy_from_slice(&self.reserved.to_le_bytes());
        bytes[20..24].copy_from_slice(&self.value.to_le_bytes());
    }
}

pub struct PumpBuffers<'a> {
    pub frames: &'a mut [u8],
    pub markers_in: &'a mut [u8],
    pub markers_out: &'a mut [u8],
    pub audio: &'a mut [i16],
    pub engine_out: &'a mut [i16],
    pub coefficients: &'a mut [i32],
    pub voice_table_16k: &'a [i32],
    pub voice_table_22k: &'a [i32],
    pub gain_table: &'a [u16],
}

pub trait PumpHost {
    fn validate(&mut self, object: u32, tag: u32) -> u32;
    fn available(&mut self, stream: [u32; 2]) -> (u32, u32, bool);

    fn read_frames(&mut self, stream: [u32; 2], frames: &mut [u8]) -> (u32, u32);
    fn read_markers(&mut self, stream: [u32; 2], markers: &mut [u8]) -> (u32, u32);
    fn write_audio(&mut self, stream: [u32; 2], samples: u32) -> (u32, u32);
    fn write_markers(&mut self, stream: [u32; 2], bytes: u32) -> (u32, u32);

    fn commit_audio(&mut self, stream: [u32; 2], samples: &[i16]) -> u32;

    fn commit_markers(&mut self, stream: [u32; 2], bytes: &[u8]) -> u32;
    fn consume(&mut self, stream: [u32; 2], bytes: u32) -> u32;
    fn close(&mut self, stream: [u32; 2]) -> u32;
    fn lock(&mut self, lock: u32) -> u32;
    fn unlock(&mut self, lock: u32) -> u32;
    fn set_param(&mut self, params: u32, name: &[u8], value: i32);
    fn log_error(&mut self, log: u32, code: u32);
    fn engine_info(&mut self, engine: u32) -> (i32, i32);
    fn frame_pitch(
        &mut self,
        engine: u32,
        quality_flag: i32,
        position: i32,
        pitch_position: i32,
        energy_position: i32,
    ) -> (u16, u16, i32, i32);

    fn read_coefficients(&mut self, table: u32, index: u32, out: &mut [i32]);
    fn synth_frame(
        &mut self,
        engine: u32,
        pitch: u16,
        coefficients: &[i32],
        energy: u16,
        output: &mut [i16],
    ) -> (bool, usize);
}

fn frame_u32(frames: &[u8], offset: usize) -> u32 {
    u32::from_le_bytes(frames[offset..offset + 4].try_into().unwrap_or([0; 4]))
}

fn frame_i32(frames: &[u8], offset: usize) -> i32 {
    frame_u32(frames, offset) as i32
}

fn frame_put_i32(frames: &mut [u8], offset: usize, value: i32) {
    frames[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
}

pub fn pump_synthesis(
    synth: &mut Synth,
    tag: u32,
    env_log: u32,
    env_params: u32,
    buffers: &mut PumpBuffers<'_>,
    state: &mut u8,
    host: &mut impl PumpHost,
) -> u32 {
    let mut result = host.validate(synth.address, tag);
    if result >= 0x80000000 {
        return 0x84e02008;
    }
    *state = 1;
    let (status, speech_bytes, ended) = host.available(synth.speech_input);
    result = status;
    if result >= 0x80000000 {
        host.log_error(env_log, 0x6595);
        return result;
    }
    let mut frames_ptr = 0u32;
    if speech_bytes != 0 {
        let (status, ptr) = host.read_frames(synth.speech_input, buffers.frames);
        result = status;
        if result >= 0x80000000 {
            host.log_error(env_log, 0x6596);
            return result;
        }
        frames_ptr = ptr;
    } else {
        if !ended {
            *state = 0;
            return result;
        }
        if synth.audio_output[0] != 0 {
            result = host.close(synth.audio_output);
            synth.audio_output = [0, 0];
            if result >= 0x80000000 {
                return result;
            }
        }
        if synth.marker_output[0] == 0 {
            return result;
        }
        let (status, marker_bytes) = host.read_markers(synth.marker_input, buffers.markers_in);
        result = status;
        if result >= 0x80000000 {
            host.log_error(env_log, 0xdac5);
            return result;
        }
        if marker_bytes != 0 {
            let (status, _) = host.write_markers(synth.marker_output, marker_bytes);
            result = status;
            if result >= 0x80000000 {
                host.log_error(env_log, 0xdac7);
                return result;
            }
            forward_markers(
                buffers.markers_in,
                buffers.markers_out,
                marker_bytes,
                synth.samples_processed,
            );
            let count = marker_bytes / 24 * 24;
            result =
                host.commit_markers(synth.marker_output, &buffers.markers_out[..count as usize]);
            if result >= 0x80000000 {
                host.log_error(env_log, 0xdac7);
                return result;
            }
            result = host.consume(synth.marker_input, count);
            if result >= 0x80000000 {
                host.log_error(env_log, 0xdac6);
                return result;
            }
        }
        result = host.close(synth.marker_output);
        synth.marker_output = [0, 0];
        return result;
    }
    let (status, marker_bytes) = host.read_markers(synth.marker_input, buffers.markers_in);
    result = status;
    if result >= 0x80000000 {
        host.log_error(env_log, 0xdac5);
        return result;
    }
    if speech_bytes < 4 {
        return result;
    }
    let (status, _) = host.write_audio(synth.audio_output, synth.buffer_samples * 2);
    result = status;
    if result >= 0x80000000 {
        host.log_error(env_log, 0x6599);
        return result;
    }
    if marker_bytes != 0 {
        let (status, _) = host.write_markers(synth.marker_output, marker_bytes);
        result = status;
        if result >= 0x80000000 {
            host.log_error(env_log, 0xdac7);
            return result;
        }
    }
    result = host.lock(synth.lock);
    if result >= 0x80000000 {
        return result;
    }
    if synth.current != frames_ptr {
        synth.current = frames_ptr;
        synth.frame_index = 0;
    }
    let mut marker_index = 0usize;
    let marker_count = marker_bytes as usize / 24;
    if marker_bytes != 0 {
        buffers.markers_out[..marker_bytes as usize]
            .copy_from_slice(&buffers.markers_in[..marker_bytes as usize]);
    }
    let frame_count = frame_i32(buffers.frames, 0x26c + 0x20);
    let mut position = frame_i32(buffers.frames, 0x434);
    let mut pitch_position = frame_i32(buffers.frames, 0x438);
    let mut energy_position = frame_i32(buffers.frames, 0x43c);
    let mut samples = 0i32;
    let mut full = false;
    let (order, frame_length) = host.engine_info(synth.engine);
    while (synth.frame_samples.wrapping_add(samples as u32) <= synth.buffer_samples)
        && synth.frame_index < frame_count
    {
        while marker_index < marker_count
            && ({
                let marker = SynthMarker::load(
                    &buffers.markers_out[marker_index * 24..marker_index * 24 + 24],
                );
                marker.typ == 0x1a
                    || marker.typ == 0x15
                    || marker.sample <= (frame_length as u32).wrapping_mul(synth.frames_processed)
            })
        {
            let mut marker =
                SynthMarker::load(&buffers.markers_out[marker_index * 24..marker_index * 24 + 24]);
            match marker.typ {
                0x18 => {
                    host.unlock(synth.lock);
                    host.set_param(env_params, b"volume", marker.value);
                    host.lock(synth.lock);
                }
                0x12 => {
                    host.unlock(synth.lock);
                    host.set_param(env_params, b"rate", marker.value);
                    host.lock(synth.lock);
                }
                0x10 => {
                    host.unlock(synth.lock);
                    host.set_param(env_params, b"pitch", marker.value);
                    host.lock(synth.lock);
                }
                _ => (),
            }
            marker.sample = synth.samples_processed;
            marker.reserved = 0;
            marker.store(&mut buffers.markers_out[marker_index * 24..marker_index * 24 + 24]);
            marker_index += 1;
        }
        let (pitch, energy, next_pp, next_ep) = host.frame_pitch(
            synth.engine,
            i32::from(synth.quality == 0),
            position,
            pitch_position,
            energy_position,
        );
        pitch_position = next_pp;
        energy_position = next_ep;

        let table = frame_u32(buffers.frames, 0x26c + 0x2c);
        host.read_coefficients(
            table,
            synth.frame_index as u32,
            &mut buffers.coefficients[..order as usize + 1],
        );
        if synth.sample_rate == 16000 {
            for (slot, adjust) in buffers.coefficients[..order as usize + 1]
                .iter_mut()
                .zip(buffers.voice_table_16k.iter())
            {
                *slot = slot.wrapping_add(*adjust);
            }
        } else if synth.sample_rate == 22050 {
            for (slot, adjust) in buffers.coefficients[..order as usize + 1]
                .iter_mut()
                .zip(buffers.voice_table_22k.iter())
            {
                *slot = slot.wrapping_add(*adjust);
            }
        }
        let (failed, produced) = host.synth_frame(
            synth.engine,
            pitch,
            &buffers.coefficients[..order as usize + 1],
            energy,
            &mut buffers.engine_out[..],
        );
        result = if failed { 0x84e02000 } else { 0 };
        if produced > 0 {
            synth.samples_processed = synth.samples_processed.wrapping_add(produced as u32);
            synth.frames_processed = synth.frames_processed.wrapping_add(1);
            if synth.buffer_samples < (samples as u32).wrapping_add(produced as u32) {
                if samples > 0 {
                    result =
                        host.commit_audio(synth.audio_output, &buffers.audio[..samples as usize]);
                }
                samples = 0;
                if result < 0x80000000 {
                    let (status, _) = host.write_audio(synth.audio_output, produced as u32 * 2);
                    result = status;
                    if result >= 0x80000000 {
                        host.log_error(env_log, 0x6599);
                    }
                }
                full = true;
            }
            if result < 0x80000000 {
                for i in 0..produced {
                    buffers.audio[samples as usize + i] = buffers.engine_out[i];
                }
                if crate::prosody_rules::apply_gain(
                    &mut buffers.audio[samples as usize..samples as usize + produced],
                    u32::from(synth.volume),
                    buffers.gain_table,
                )
                .is_err()
                {

                }
                samples += produced as i32;
            }
        }
        frame_put_i32(buffers.frames, 0x438, pitch_position);
        position += 1;
        frame_put_i32(buffers.frames, 0x43c, energy_position);
        frame_put_i32(buffers.frames, 0x434, position);
        synth.frame_index += 1;
        if result >= 0x80000000 || full {
            break;
        }
    }
    if samples > 0 {
        if result >= 0x80000000 {
            return unlock_pump(synth, result, host);
        }
        result = host.commit_audio(synth.audio_output, &buffers.audio[..samples as usize]);
        if result >= 0x80000000 {
            host.log_error(env_log, 0x6599);
            return unlock_pump(synth, result, host);
        }
    } else if result >= 0x80000000 {
        return unlock_pump(synth, result, host);
    }
    if marker_index != 0 {
        result = host.commit_markers(
            synth.marker_output,
            &buffers.markers_out[..marker_index as usize * 24],
        );
        if result >= 0x80000000 {
            host.log_error(env_log, 0xdac7);
            return unlock_pump(synth, result, host);
        }
    }
    if synth.frame_index < frame_count {
        *state = 2;
    } else {
        result = host.consume(synth.speech_input, 4);
        if result >= 0x80000000 {
            host.log_error(env_log, 0x6598);
            return unlock_pump(synth, result, host);
        }
        synth.current = 0;
        *state = if speech_bytes > 4 { 2 } else { 1 };
    }
    if marker_index != 0 {
        result = host.consume(synth.marker_input, marker_index as u32 * 24);
        if result >= 0x80000000 {
            host.log_error(env_log, 0xdac6);
        }
    }
    unlock_pump(synth, result, host)
}

fn unlock_pump(synth: &Synth, mut result: u32, host: &mut impl PumpHost) -> u32 {
    let status = host.unlock(synth.lock);
    if status >= 0x80000000 && result < 0x80000000 {
        result = status;
    }
    result
}

fn forward_markers(input: &[u8], output: &mut [u8], bytes: u32, processed: u32) {
    output[..bytes as usize].copy_from_slice(&input[..bytes as usize]);
    let mut count = bytes / 24;
    let mut index = 0usize;
    while count != 0 {
        let mut marker = SynthMarker::load(&output[index * 24..index * 24 + 24]);
        marker.sample = processed;
        marker.reserved = 0;
        marker.store(&mut output[index * 24..index * 24 + 24]);
        index += 1;
        count -= 1;
    }
}

pub fn open_synth(
    session: [u32; 2],
    out: Option<&mut [u32; 2]>,
    synth: &mut Synth,
    host: &mut impl SynthHost,
) -> u32 {
    let Some(out) = out else { return 0x84e02007 };
    let (status, env) = host.resolve_env(session);
    if status >= 0x80000000 {
        return status;
    }
    let Some(env) = env else { return status };
    *out = [0, 0];
    host.log_module(env.log, 4);
    let object = host.open_synth_object(env.heap, 1, 0x70);
    if object == 0 {
        host.log_error(env.log, 0x6590);
        host.log_module(env.log, 4);
        return 0x84e0200a;
    }
    *synth = Synth {
        address: object,
        session,
        environment: ENV_TOKEN,
        ..Synth::default()
    };
    let mut result = host.open_lock(env.pool, env.heap, &mut synth.lock);
    if result >= 0x80000000 {
        return fail(synth, env, result, host);
    }
    let mut value = 0;
    for (name, minimum, maximum, fallback) in [
        (b"volume".as_slice(), 0, 100, 80),
        (b"rate", 50, 400, 100),
        (b"pitch", 1, 100, 50),
        (PARAM_BUFFER_SAMPLES, 1, 0x7fffffff, 256),
    ] {
        result = host.register_parameter(
            env.parameters,
            name,
            minimum,
            maximum,
            fallback,
            object,
            &mut value,
        );
        if result >= 0x80000000 {
            return fail(synth, env, result, host);
        }
        if name == PARAM_BUFFER_SAMPLES {
            synth.buffer_samples = value as u32;
        }
    }
    result = host.rebuild_engine(synth);
    if result >= 0x80000000 {
        return fail(synth, env, result, host);
    }
    let mut resource = 0;
    result = host.acquire_resource(env.resources, b"SYNTHSTREAM", &mut resource);
    if result >= 0x80000000 {
        host.log_error(env.log, 0x6591);
        return fail(synth, env, result, host);
    }
    synth.streams = resource;
    *out = [object, 0x4e7];
    host.log_module(env.log, 4);
    0
}

fn fail(synth: &mut Synth, env: SynthEnvironment, result: u32, host: &mut impl SynthHost) -> u32 {
    if synth.streams != 0 {
        host.release_resource(env.resources, b"SYNTHSTREAM");
    }
    if synth.engine != 0 {
        host.destroy_engine(synth.engine);
        synth.engine = 0;
    }
    if synth.equalizer_handle != 0 {
        host.destroy_equalizer(synth.equalizer_handle);
        synth.equalizer_handle = 0;
    }
    for name in [
        b"volume".as_slice(),
        b"rate",
        b"pitch",
        PARAM_BUFFER_SAMPLES,
    ] {
        host.release_parameter(env.parameters, name, synth.address);
    }
    if synth.lock != 0 {
        host.destroy_lock(synth.lock);
    }
    host.free_object(env.heap, synth.address);
    host.log_module(env.log, 4);
    result
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct CallbackTail {
    pub data: u32,
    pub size: u32,
    pub metadata_buffer: u32,
    pub stop: u8,
    pub finished: u8,
    pub requested: u8,
    pub pending: u8,
    pub started: u8,
    pub engine: u32,
    pub delay: u64,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct CallbackSpeech {
    pub address: u32,
    pub state: u32,
    pub active: u32,
    pub offset: u32,
    pub first: u32,
    pub second: u32,
    pub total: u32,
    pub running: u8,
    pub count: u8,
    pub tail: CallbackTail,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct CallbackEvent {
    pub kind: i32,
    pub result: u32,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct SynthResult {
    pub field0: u32,
    pub size: u32,
    pub data: u32,
    pub metadata_capacity: u32,
    pub metadata_buffer: u32,
}

pub trait CallbackHost {
    fn notify(&mut self, speech: u32, event: i32, queue: &mut [u8]);
    fn reset(&mut self, engine: u32);
    fn finalize(&mut self, speech: u32, a: u32, b: u32);
    fn poll_ready(&mut self, speech: u32) -> bool;
    fn sleep(&mut self, lo: u32, hi: u32);
    #[allow(clippy::too_many_arguments)]
    fn synthesize(
        &mut self,
        engine: u32,
        a: u32,
        b: u32,
        result_addr: u32,
        result: &mut SynthResult,
        buffer: u32,
        first: &mut [i16],
        second_addr: u32,
        second: &mut [i16],
    ) -> (u32, u32);
    fn publish(&mut self, samples: &[i16]);

    fn update(&mut self, speech: u32) -> u32;
    fn lock(&mut self, lock: u32);
    fn unlock(&mut self, lock: u32);
    fn ready(&mut self, speech: u32) -> bool;
}

#[allow(clippy::too_many_arguments)]
pub fn handle_callback(
    speech: &mut CallbackSpeech,
    a: u32,
    b: u32,
    tick: u32,
    event: &CallbackEvent,
    result: &mut SynthResult,
    queue: &mut [u8],
    first: &mut [i16],
    second: &mut [i16],
    host: &mut impl CallbackHost,
) -> u32 {
    host.notify(speech.address, event.kind, queue);
    match event.kind {
        1 => host.reset(speech.tail.engine),
        2 => {
            host.finalize(speech.address, a, b);
            speech.tail.started = 1;
        }
        8 => {
            result.data = speech.tail.data.wrapping_add(speech.tail.size & !1);
            result.size = speech.tail.size;
            result.metadata_buffer = speech.tail.metadata_buffer;
            result.metadata_capacity = 4000;
        }
        16 => render_callback(speech, a, b, tick, event.result, result, first, second, host),
        32 => speech.tail.requested = 1,
        _ => {}
    }
    0
}

#[allow(clippy::too_many_arguments)]
fn render_callback(
    speech: &mut CallbackSpeech,
    a: u32,
    b: u32,
    tick: u32,
    result_addr: u32,
    result: &mut SynthResult,
    first: &mut [i16],
    second: &mut [i16],
    host: &mut impl CallbackHost,
) {
    loop {
        if host.poll_ready(speech.address) {
            if speech.running == 0 || speech.state <= 1 || speech.active != 0 {
                break;
            }
        }
        if speech.tail.stop != 0 {
            return;
        }
        host.sleep(tick, 0);
    }
    let offset = speech.offset.wrapping_mul(2);
    let buffer = speech.first.wrapping_add(offset);
    let first_cap = 0x4b000u32.wrapping_sub(offset) >> 1;
    let second_cap = if speech.state < 2 { offset >> 1 } else { 0x25800 };
    let (produced_first, produced_second) = host.synthesize(
        speech.tail.engine,
        a,
        b,
        result_addr,
        result,
        buffer,
        &mut first[..first_cap as usize],
        speech.second,
        &mut second[..second_cap as usize],
    );
    speech.total = speech
        .total
        .wrapping_add(produced_first.wrapping_add(produced_second).wrapping_mul(2));
    host.publish(&first[..produced_first as usize]);
    host.publish(&second[..produced_second as usize]);
    speech.offset = host.update(speech.address);
    let lock = speech.address.wrapping_add(0x110);
    host.lock(lock);
    if host.ready(speech.address) && speech.tail.pending == 0 {
        if speech.running == 0 && speech.tail.finished == 0 {
            speech.tail.finished = 1;
        }
    }
    host.unlock(lock);
    if speech.count < 16 {
        let delay = (speech.tail.delay as i64).wrapping_mul(1000);
        host.sleep(delay as u32, (delay >> 32) as u32);
    }
}
