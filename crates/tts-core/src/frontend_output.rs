use crate::frames_boundary::{SynthesisBoundary, OUTPUT_SAMPLES};
use crate::frontend_types::Error;

pub mod handoff;

pub use handoff::{
    close_synth, open_streams, open_synth, pump_synthesis, rebuild_synth, EngineWorkshop,
    EqualizerView, PumpBuffers, PumpHost, RebuildHost, Synth, SynthEnvironment, SynthHost,
    SynthMarker,
};

pub fn synthesis_interface<'a, T: ?Sized>(
    index: u32,
    table: &'a T,
    output: &mut Option<&'a T>,
) -> u32 {
    if index > 1 {
        return 0x84e02001;
    }
    *output = Some(table);
    0
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct OutputLoan {
    pub byte_count: u32,
    pub audio_offset: u32,
    pub marker_capacity: u32,
    pub marker_buffer: u32,
}

pub struct SynthesisBatch<'a> {
    pub audio: &'a [u8],
    pub markers: &'a [u8],
}

#[derive(Clone, Copy, Debug, Default)]
pub struct SynthesisCallback {
    pub boundary: SynthesisBoundary,
    pub active: u32,
    pub running: u8,
    pub count: u8,
    pub loan_bytes: u32,
    pub marker_buffer: u32,
    pub stop: u8,
    pub finished: u8,
    pub requested: u8,
    pub pending: u8,
    pub started: u8,
    pub delay: u64,
}

pub trait CallbackHost {
    fn event(&mut self, state: &mut SynthesisCallback, event: u32);
    fn reset(&mut self, state: &mut SynthesisCallback);
    fn finish(&mut self, state: &mut SynthesisCallback);
    fn available(&mut self, state: &mut SynthesisCallback) -> bool;
    fn render(&mut self, state: &mut SynthesisCallback);
    fn lock(&mut self, state: &mut SynthesisCallback);
    fn ready(&mut self, state: &mut SynthesisCallback) -> bool;
    fn unlock(&mut self, state: &mut SynthesisCallback);
    fn sleep(&mut self, state: &mut SynthesisCallback, nanoseconds: u64);
}

impl SynthesisCallback {

    pub fn handle(
        &mut self,
        event: u32,
        loan: Option<&mut OutputLoan>,
        host: &mut impl CallbackHost,
    ) -> Result<u32, Error> {
        host.event(self, event);
        match event {
            1 => host.reset(self),
            2 => {
                host.finish(self);
                self.started = 1;
            }
            8 => {
                let loan = loan.ok_or(Error::InvalidResource)?;
                loan.audio_offset = self.loan_bytes & !1;
                loan.byte_count = self.loan_bytes;
                loan.marker_buffer = self.marker_buffer;
                loan.marker_capacity = 4000;
            }
            16 => {
                loop {
                    if host.available(self)
                        && (self.running == 0 || self.boundary.state <= 1 || self.active != 0)
                    {
                        break;
                    }
                    if self.stop != 0 {
                        return Ok(0);
                    }
                    host.sleep(self, 10_000_000);
                }
                host.render(self);
                host.lock(self);
                if host.ready(self) && self.pending == 0 && self.running == 0 && self.finished == 0
                {
                    self.finished = 1;
                }
                host.unlock(self);
                if self.count < 16 {
                    host.sleep(self, self.delay.wrapping_mul(1000));
                }
            }
            32 => self.requested = 1,
            _ => (),
        }
        Ok(0)
    }

    pub fn render_stream(
        &mut self,
        output: [&mut [i16]; 2],
        mut synthesize: impl FnMut([&mut [i16]; 2]) -> [usize; 2],
        mut callback: impl FnMut(&[i16]),
        mut publish: impl FnMut(&mut SynthesisBoundary),
    ) -> Result<[usize; 2], Error> {
        let [first, second] = output;
        let offset = self.boundary.offset as usize;
        let second_capacity = if self.boundary.state < 2 {
            offset
        } else {
            OUTPUT_SAMPLES
        };
        if offset > OUTPUT_SAMPLES {
            return Err(Error::InvalidResource);
        }
        if first.len() < OUTPUT_SAMPLES {
            return Err(Error::Capacity {
                required: OUTPUT_SAMPLES,
                available: first.len(),
            });
        }
        if second.len() < second_capacity {
            return Err(Error::Capacity {
                required: second_capacity,
                available: second.len(),
            });
        }
        let counts = synthesize([
            &mut first[offset..OUTPUT_SAMPLES],
            &mut second[..second_capacity],
        ]);
        if counts[0] > OUTPUT_SAMPLES - offset || counts[1] > second_capacity {
            return Err(Error::InvalidResource);
        }
        self.boundary.total_bytes = self.boundary.total_bytes.wrapping_add(
            (counts[0] as u32)
                .wrapping_add(counts[1] as u32)
                .wrapping_mul(2),
        );
        callback(&first[offset..offset + counts[0]]);
        callback(&second[..counts[1]]);
        publish(&mut self.boundary);
        Ok(counts)
    }
}

pub struct DecodedBatch<'a> {
    pub samples: &'a [i16],
    pub events: &'a [crate::queue::SpeechEvent],
}

impl SynthesisBatch<'_> {

    pub fn decode<'a>(
        &self,
        samples: &'a mut [i16],
        events: &'a mut [crate::queue::SpeechEvent],
    ) -> Result<DecodedBatch<'a>, Error> {
        if self.markers.len() % 40 != 0 {
            return Err(Error::InvalidResource);
        }
        let sample_count = self.audio.len() / 2;
        let event_count = self.markers.len() / 40;
        if samples.len() < sample_count {
            return Err(Error::Capacity {
                required: sample_count,
                available: samples.len(),
            });
        }
        if events.len() < event_count {
            return Err(Error::Capacity {
                required: event_count,
                available: events.len(),
            });
        }
        for (output, bytes) in samples[..sample_count]
            .iter_mut()
            .zip(self.audio.chunks_exact(2))
        {
            *output = i16::from_le_bytes([bytes[0], bytes[1]]);
        }
        for (output, bytes) in events[..event_count]
            .iter_mut()
            .zip(self.markers.chunks_exact(40))
        {
            let half = |offset| u16::from_le_bytes([bytes[offset], bytes[offset + 1]]);
            let word = |offset| {
                u32::from_le_bytes([
                    bytes[offset],
                    bytes[offset + 1],
                    bytes[offset + 2],
                    bytes[offset + 3],
                ])
            };
            *output = crate::queue::SpeechEvent {
                kind: half(4),
                value: word(12) as i32,
                position: word(16),
                character: half(24),
                command: word(28),
            };
        }
        Ok(DecodedBatch {
            samples: &samples[..sample_count],
            events: &events[..event_count],
        })
    }
}

impl DecodedBatch<'_> {

    pub fn render(
        &self,
        state: &mut SynthesisCallback,
        backend: &mut impl crate::output::SpeechBackend,
        output: [&mut [i16]; 2],
        callback: impl FnMut(&[i16]),
        publish: impl FnMut(&mut SynthesisBoundary),
    ) -> Result<[usize; 2], Error> {
        state.render_stream(
            output,
            |spans| {
                crate::output::synthesize_stream(
                    backend,
                    self.samples,
                    self.events,
                    spans,
                    0,
                    false,
                )
            },
            callback,
            publish,
        )
    }
}

pub fn validate_parameter(name: &[u8], value: &[u8], valid: &mut u8) -> u32 {
    let name = &name[..name.iter().position(|&b| b == 0).unwrap_or(name.len())];
    let value = crate::prosody_rules::signed_decimal(value);
    let status = match name {
        b"audiooutputbufsamples" if value < 1 => 0x84e0200f,
        b"volume" if value as u32 >= 101 => 0x84e0200f,
        b"rate" if (value as u32).wrapping_sub(50) >= 351 => 0x84e0200f,
        b"ratewpm" => 0x84e02001,
        b"pitch" if (value as u32).wrapping_sub(1) >= 100 => 0x84e0200f,
        _ => 0,
    };
    *valid = (status < 0x80000000) as u8;
    status
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct SynthesisParameters {
    pub buffer_samples: u32,
    pub frame_samples: u32,
    pub volume: u16,
}

pub trait ParameterHost {
    fn lock(&mut self) -> u32;
    fn unlock(&mut self);
    fn duration(&mut self, percent: i32) -> i32;
    fn pitch(&mut self, pitch: i32) -> i32;
}

impl SynthesisParameters {

    pub fn set(&mut self, name: &[u8], value: u32, host: &mut impl ParameterHost) -> u32 {
        let name = &name[..name.iter().position(|&b| b == 0).unwrap_or(name.len())];
        if name == b"audiooutputbufsamples" {
            let mut value = value.max(1);
            if self.frame_samples != 0 {
                let remainder = value % self.frame_samples;
                if remainder != 0 && value > self.frame_samples {
                    value -= remainder;
                }
            }
            self.buffer_samples = value.max(self.frame_samples);
            return 0;
        }
        let mut status = host.lock();
        if status >= 0x80000000 {
            return status;
        }
        match name {
            b"volume" => {
                if value < 101 {
                    self.volume = value as u16;
                } else {
                    status = 0x84e0200f;
                }
            }
            b"rate" => {
                let mut converted = 0;
                status = crate::selection::inverse_percent(value as i32, &mut converted);
                if status >= 0x80000000 {
                    return status;
                }
                if host.duration(converted) != 0 {
                    status = 0x84e0200f;
                }
            }
            b"ratewpm" => status = 0x84e02001,
            b"pitch" => {
                if value.wrapping_sub(1) >= 100 {
                    status = 0x84e0200f;
                } else if host.pitch(value.wrapping_sub(50).wrapping_mul(5) as i32) != 0 {
                    status = 0x84e0200f;
                }
            }
            _ => (),
        }
        host.unlock();
        status
    }

    pub fn set_text(&mut self, name: &[u8], value: &[u8], host: &mut impl ParameterHost) -> u32 {
        let name = &name[..name.iter().position(|&b| b == 0).unwrap_or(name.len())];
        if matches!(
            name,
            b"audiooutputbufsamples" | b"volume" | b"rate" | b"ratewpm" | b"pitch"
        ) {
            self.set(
                name,
                crate::prosody_rules::signed_decimal(value) as u32,
                host,
            )
        } else {
            0
        }
    }
}

pub trait ParameterRegistry {
    fn get(&mut self, name: &[u8], value: &mut i32) -> u32;
    fn set(&mut self, name: &[u8], value: i32) -> u32;
    fn subscribe(&mut self, name: &[u8]) -> u32;
}

pub fn register_parameter(
    name: &[u8],
    minimum: i32,
    maximum: i32,
    fallback: i32,
    value: &mut i32,
    registry: &mut impl ParameterRegistry,
) -> u32 {
    let status = registry.get(name, value);
    let replacement = if status >= 0x80000000 {
        Some(fallback)
    } else if *value < minimum {
        Some(minimum)
    } else if *value > maximum {
        Some(maximum)
    } else {
        None
    };
    if let Some(replacement) = replacement {
        let status = registry.set(name, replacement);
        *value = replacement;
        if status >= 0x80000000 {
            return status;
        }
    }
    registry.subscribe(name)
}

pub fn reopen_synthesis(present: bool, tag: u32, configure: impl FnOnce() -> u32) -> u32 {
    if !present || tag != 0x4e7 {
        return 0x84e02008;
    }
    configure()
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct SynthesisStreams {
    pub speech_input: [u32; 2],
    pub audio_output: [u32; 2],
    pub marker_input: [u32; 2],
    pub marker_output: [u32; 2],
    pub frames_processed: u32,
    pub samples_processed: u32,
}

impl SynthesisStreams {

    pub fn close(
        &mut self,
        present: bool,
        tag: u32,
        mut close: impl FnMut([u32; 2]) -> u32,
    ) -> u32 {
        if !present || tag != 0x4e7 {
            return 0x84e02008;
        }
        self.samples_processed = 0;
        self.frames_processed = 0;
        let mut status = 0;
        for (index, handle) in [
            &mut self.speech_input,
            &mut self.marker_input,
            &mut self.audio_output,
            &mut self.marker_output,
        ]
        .into_iter()
        .enumerate()
        {
            if handle[0] != 0 {
                let result = close(*handle);
                if index == 0 || (status < 0x80000000 && result >= 0x80000000) {
                    status = result;
                }
                *handle = [0; 2];
            }
        }
        status
    }
}
