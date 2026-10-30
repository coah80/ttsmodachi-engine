use crate::{
    frames::{LipSync, PhonemeFrame, PlannedSegments},
    orchestration::{Setup, SetupState},
    resources::{EngineResources, LipSyncInfo},
    tables::NumericTables,
};

pub struct SpeechFrame<'a> {
    pub reset_pitch: bool,
    pub pitch_duration: i32,
    pub elapsed_pitch: i32,
    pub commands: &'a [u32],
    pub samples: &'a [i16],
    pub phonemes: PhonemeFrame<'a>,
    pub voiced_starts: &'a [i32],
    pub voiced_lengths: &'a [i32],
    pub group_starts: &'a [i32],
    pub group_lengths: &'a [i32],
    pub leading_ticks: i32,
    pub trailing_ticks: i32,
    pub ticks: &'a [i32],
    pub duration_ticks: i32,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct SpeechTiming {
    pub phoneme: u16,
    pub pitch_duration: i32,
}

fn ticks_to_samples(ticks: i32, state: &SetupState) -> f32 {
    (ticks as f32 / 480.0) * (60.0 / state.tempo) * state.sample_rate
}

pub struct KoreanTiming {
    pub scales: [[f32; 4]; 2],
    pub limits: [i32; 2],
}

pub struct FrameSynthesis<'a, 'b, 'c> {
    pub setup: &'a mut Setup<'b, 'c>,
    pub timing: &'a mut SpeechTiming,
    pub segments: &'a mut PlannedSegments<'b>,
    pub lip_sync: &'a mut LipSync<'b>,
}

impl FrameSynthesis<'_, '_, '_> {
    /// fn_1b0a64 / EU fn_1b0ab8. KR fn_1b0aac uses render_korean.
    pub fn render(
        &mut self,
        resources: &mut EngineResources<'_>,
        tag: u32,
        frame: &SpeechFrame<'_>,
        outputs: [&mut [i16]; 2],
        flush: bool,
        tables: &NumericTables<'_>,
        synthesize: impl FnMut(&mut Setup<'_, '_>, &[i16], [&mut [i16]; 2], bool) -> [usize; 2],
    ) -> [usize; 2] {
        self.render_region(
            resources, tag, frame, outputs, flush, tables, None, synthesize,
        )
    }

    /// KR fn_1b0aac / US fn_1b0a64 / EU fn_1b0ab8.
    pub fn render_korean(
        &mut self,
        resources: &mut EngineResources<'_>,
        tag: u32,
        frame: &SpeechFrame<'_>,
        outputs: [&mut [i16]; 2],
        flush: bool,
        tables: &NumericTables<'_>,
        korean: &KoreanTiming,
        synthesize: impl FnMut(&mut Setup<'_, '_>, &[i16], [&mut [i16]; 2], bool) -> [usize; 2],
    ) -> [usize; 2] {
        self.render_region(
            resources,
            tag,
            frame,
            outputs,
            flush,
            tables,
            Some(korean),
            synthesize,
        )
    }

    fn render_region(
        &mut self,
        resources: &mut EngineResources<'_>,
        tag: u32,
        frame: &SpeechFrame<'_>,
        outputs: [&mut [i16]; 2],
        flush: bool,
        tables: &NumericTables<'_>,
        korean: Option<&KoreanTiming>,
        mut synthesize: impl FnMut(&mut Setup<'_, '_>, &[i16], [&mut [i16]; 2], bool) -> [usize; 2],
    ) -> [usize; 2] {
        let mut rate = 0;
        let status = resources.rate(tag, &mut rate);
        assert!(
            status < 0x80000000,
            "synthesis requires a readable rate parameter"
        );
        let pitch_scale = 100.0 / rate as f32;
        let initial = if frame.duration_ticks > 0 {
            frame.phonemes.leading_sample()
        } else {
            0
        };
        if frame.duration_ticks > 0 {
            self.setup
                .input
                .skip_samples(self.setup.voice.time, initial, 0);
        }
        self.setup.apply_commands(frame.commands, 0, tables);
        let mut total = frame.leading_ticks;
        let sample_count = frame.samples.len() as i32;
        if sample_count == 0 {
            total = total.wrapping_add(frame.duration_ticks);
        }
        let leading = if total > 0 {
            ticks_to_samples(total, self.setup.state) as i32
        } else {
            0
        };
        if total > 0 {
            self.setup
                .input
                .insert_silence(self.setup.voice.time, leading, 0);
        }
        if frame.reset_pitch {
            self.timing.pitch_duration = frame.pitch_duration;
            let state = &mut self.setup.state;
            let scale = state.voice_gain
                * self.timing.pitch_duration as f32
                * (pitch_scale
                    * if korean.is_some() {
                        1.0 / 18.0
                    } else {
                        f32::from_bits(0x3cba2e8c)
                    });
            self.setup.input.pitch_transition(
                self.setup.voice.time,
                0,
                if korean.is_some() {
                    state.value4c14
                } else {
                    state.timing_blend
                },
                if korean.is_some() {
                    state.timing_blend
                } else {
                    state.value4c14
                },
                scale,
            );
            self.setup.input.gain_transition(
                self.setup.voice.time,
                0,
                state.duration_scale,
                state.value4c18,
                scale,
            );
            state.reset_count = 0;
        }
        if korean.is_some() && frame.phonemes.symbols[0] == 35 && self.timing.pitch_duration > 0 {
            let state = &mut self.setup.state;
            let scale =
                state.voice_gain * self.timing.pitch_duration as f32 * (pitch_scale * (1.0 / 18.0));
            self.setup.input.pitch_transition(
                self.setup.voice.time,
                frame.phonemes.leading_sample(),
                state.value4c14,
                state.timing_blend,
                scale,
            );
            state.reset_count = 0;
        }
        let mut segments = 0usize;
        let mut duration = (sample_count as f32 * self.setup.state.voice_gain) as i32;
        if frame.duration_ticks > 0 && sample_count > 0 {
            let state = &self.setup.state;
            let desired = (state.sample_rate * (state.state as f32 * 0.0001)) as i32;
            duration = (ticks_to_samples(frame.duration_ticks, state) * state.voice_gain) as i32;
            let first = frame.phonemes.leading_sample();
            let voiced_count = frame.voiced_starts.len();
            if frame.ticks.len() > 1 && voiced_count > 0 && voiced_count <= frame.ticks.len() {
                let mut remaining = duration;
                let skip = if voiced_count < frame.ticks.len() && state.mode == 2 {
                    frame.ticks.len() - voiced_count
                } else {
                    0
                };
                let mut accumulated = frame.ticks[..skip]
                    .iter()
                    .fold(0i32, |sum, &x| sum.wrapping_add(x));
                let mut length = 0;
                for i in 0..frame.group_starts.len() {
                    if i < frame.ticks.len() {
                        length = ticks_to_samples(
                            frame.ticks[skip + i].wrapping_add(accumulated),
                            state,
                        ) as i32;
                        accumulated = 0;
                    }
                    if i + 1 == frame.group_starts.len() {
                        length = remaining;
                    }
                    let start = frame.voiced_starts[i]
                        .wrapping_add(first)
                        .wrapping_sub(frame.group_starts[i]);
                    segments += self.segments.plan(
                        self.setup.input,
                        self.setup.voice.time,
                        frame.group_starts[i],
                        frame.group_lengths[i],
                        &[start],
                        &frame.voiced_lengths[i..],
                        1,
                        state.mode,
                        desired,
                        segments,
                        length as f32,
                        state.gain,
                    ) as usize;
                    remaining = remaining.wrapping_sub(length);
                }
            } else if voiced_count > 0 && sample_count.wrapping_sub(first) < duration {
                segments = self.segments.plan(
                    self.setup.input,
                    self.setup.voice.time,
                    first,
                    sample_count.wrapping_sub(first),
                    frame.voiced_starts,
                    frame.voiced_lengths,
                    voiced_count as i32,
                    state.mode,
                    desired,
                    0,
                    duration as f32,
                    state.gain,
                ) as usize;
            } else {
                segments = self.segments.output.add_simple(
                    self.setup.input,
                    self.setup.voice.time,
                    duration,
                    first,
                    sample_count.wrapping_sub(first),
                    0,
                ) as usize;
            }
        }
        if let Some(korean) = korean {
            let state = &mut self.setup.state;
            if frame.duration_ticks <= 0
                && state.reset_mode != 0
                && sample_count > 0
                && !frame.voiced_starts.is_empty()
            {
                let base = frame.phonemes.leading_sample();
                let mode = (state.reset_mode - 1) as usize;
                for i in 0..frame.group_starts.len() {
                    let start = frame.voiced_starts[i]
                        .wrapping_add(base)
                        .wrapping_sub(frame.group_starts[i]);
                    let scale = korean.scales[mode][state.reset_count as usize] * state.voice_gain;
                    let length = (frame.group_lengths[i] as f32 * scale) as i32;
                    segments += self.segments.plan(
                        self.setup.input,
                        self.setup.voice.time,
                        frame.group_starts[i],
                        frame.group_lengths[i],
                        &[start],
                        &frame.voiced_lengths[i..],
                        1,
                        1,
                        0,
                        segments,
                        length as f32,
                        0.0,
                    ) as usize;
                    state.reset_count = crate::runtime::signed_division(
                        state.reset_count.wrapping_add(1),
                        korean.limits[mode],
                    )
                    .expect("zero Korean reset limit")
                    .remainder;
                }
            }
        }
        let trailing = if frame.trailing_ticks > 0 {
            let length = ticks_to_samples(frame.trailing_ticks, self.setup.state) as i32;
            self.setup
                .input
                .insert_silence(self.setup.voice.time, length, sample_count);
            length
        } else {
            0
        };
        let count = frame.phonemes.count as usize;
        if korean.is_some()
            && count > 0
            && frame.phonemes.symbols[count - 1] == 35
            && self.timing.pitch_duration > 0
        {
            let state = &self.setup.state;
            let scale =
                state.voice_gain * self.timing.pitch_duration as f32 * (pitch_scale * (1.0 / 18.0));
            self.setup.input.pitch_transition(
                self.setup.voice.time,
                sample_count,
                state.value4c14,
                state.timing_blend,
                scale,
            );
        }
        let made = synthesize(self.setup, frame.samples, outputs, flush);
        if frame.elapsed_pitch > 0 {
            self.timing.pitch_duration = self
                .timing
                .pitch_duration
                .wrapping_sub(frame.elapsed_pitch)
                .wrapping_sub(2)
                .max(0);
        }
        if leading > 0 {
            self.lip_sync.add_duration(leading, 0);
        }
        let mut ratio = duration as f32 / sample_count as f32;
        let count = frame.phonemes.count as usize;
        let prefix = if count > 0 {
            frame.phonemes.starts[0]
        } else {
            sample_count
        };
        let mut lip = LipSyncInfo {
            values: [0; 8],
            symbol: [0; 8],
        };
        if prefix > 0 {
            let length = (prefix as f32 * ratio) as i32;
            assert!(
                resources.lip_sync(tag, (self.timing.phoneme & 255) as i32, &mut lip) < 0x80000000,
                "synthesis requires a mapped phoneme"
            );
            self.lip_sync
                .add_duration(length, lip.values[0] as i16 as i32);
        }
        if initial > 0 {
            ratio = 0.0;
        } else if segments > 0 {
            ratio = 1.0;
        }
        let mut segment = 0;
        for i in 0..count {
            let start = frame.phonemes.starts[i];
            if segment < segments && start >= self.segments.output.positions[segment] {
                ratio = self.segments.output.scales[segment];
                segment += 1;
            }
            let end = if i + 1 < count {
                frame.phonemes.starts[i + 1]
            } else {
                sample_count
            };
            self.timing.phoneme = frame.phonemes.symbols[i];
            let length = (end.wrapping_sub(start) as f32 * ratio) as i32;
            if length > 0 {
                assert!(
                    resources.lip_sync(tag, (self.timing.phoneme & 255) as i32, &mut lip)
                        < 0x80000000,
                    "synthesis requires a mapped phoneme"
                );
                self.lip_sync
                    .add_duration(length, lip.values[0] as i16 as i32);
            }
        }
        if trailing > 0 {
            self.lip_sync.add_duration(trailing, 0);
        }
        made
    }
}

pub struct AssemblyStep {
    pub samples: usize,
    pub events: usize,
    pub ready: bool,
}

pub trait SpeechBackend {
    fn assemble(
        &mut self,
        samples: &[i16],
        events: &[crate::queue::SpeechEvent],
        initial: bool,
        flush: u32,
    ) -> AssemblyStep;
    fn render(&mut self, output: [&mut [i16]; 2], flush: bool) -> [usize; 2];
    fn reset_frame(&mut self);
    fn state(&self) -> i32;
    fn position(&self) -> i32;
    fn advance(&mut self, samples: usize);
    fn pending(&self) -> i32;
    fn add_consumed(&mut self, samples: i32);
    fn filter(&mut self, samples: &mut [i16]);
}

/// fn_1a18c4 / EU fn_1a1918 / KR fn_1a19b0.
pub fn synthesize_stream(
    backend: &mut impl SpeechBackend,
    mut samples: &[i16],
    mut events: &[crate::queue::SpeechEvent],
    output: [&mut [i16]; 2],
    flush: u32,
    flush_silence: bool,
) -> [usize; 2] {
    let [first, second] = output;
    let mut made = [0usize; 2];
    while !samples.is_empty()
        || !events.is_empty()
        || flush & !((backend.state() == -1) as u32) != 0
    {
        let step = backend.assemble(samples, events, backend.position() == 0, flush);
        backend.advance(step.samples);
        samples = &samples[step.samples..];
        events = &events[step.events..];
        if step.ready {
            let generated = backend.render(
                [&mut first[made[0]..], &mut second[made[1]..]],
                samples.is_empty() && flush != 0,
            );
            made[0] += generated[0];
            made[1] += generated[1];
            let consumed = backend.pending();
            backend.reset_frame();
            backend.add_consumed(consumed);
        }
    }
    if flush_silence && flush != 0 {
        let length = (first.len() - made[0]).min(0x3200);
        let silence = &mut first[made[0]..made[0] + length];
        silence.fill(0);
        backend.filter(silence);
        backend.filter(&mut second[made[1]..made[1]]);
        made[0] += length;
    }
    made
}

/// fn_199404 / EU fn_1994b4 / KR fn_1994f0, bytes written by the clear helper.
pub fn clear_output(output: &mut [u8]) {
    output.fill(0);
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct OutputNode {
    pub buffer: u32,
    pub samples: u32,
    pub fields: [u32; 2],
    pub complete: u8,
    pub state: u8,
    pub reserved: [u8; 2],
    pub next: Option<usize>,
}

pub struct OutputList<'a> {
    pub nodes: &'a mut [OutputNode],
    pub head: Option<usize>,
    pub flags: u8,
}

impl OutputList<'_> {
    /// fn_1b5b8c / EU fn_1b5be0 / KR fn_1b5ddc.
    pub fn notify(&mut self, index: usize) {
        let mut current = self.head;
        while let Some(node) = current {
            if node == index {
                if self.nodes[node].state != 3 {
                    self.flags |= 2;
                }
                break;
            }
            current = self.nodes[node].next;
        }
    }
}

/// fn_1994d0 / EU fn_199580 / KR fn_1995bc, including RVCT unsigned-division leaves.
pub fn unsigned_division(
    numerator: u32,
    denominator: u32,
) -> Result<[u32; 2], crate::runtime::DivisionByZero> {
    if denominator == 0 {
        return Err(crate::runtime::DivisionByZero);
    }
    Ok([numerator / denominator, numerator % denominator])
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct OutputState {
    pub current: u32,
    pub next: u32,
    pub rotations: u32,
    pub offset: u32,
    pub first: u32,
    pub second: u32,
    pub total_bytes: u32,
    pub samples_per_buffer: u32,
    pub previous_quotient: u32,
}

impl OutputState {
    /// fn_1a1b68 / EU fn_1a1bbc / KR fn_1a1c54.
    /// The host holds the engine/output-list locks while borrowing this state.
    pub fn publish(
        &mut self,
        buffers: &[u32],
        list: &mut OutputList<'_>,
    ) -> Result<(), crate::runtime::DivisionByZero> {
        let [quotient, remainder] =
            unsigned_division(self.total_bytes >> 1, self.samples_per_buffer)?;
        if self.previous_quotient < quotient {
            if buffers.len() >= 2 {
                list.nodes[self.current as usize].samples = self.samples_per_buffer;
                list.notify(self.current as usize);
                let next = self.next as usize;
                self.current = next as u32;
                self.next = self.next.wrapping_add(1) % buffers.len() as u32;
                self.first = buffers[next];
                self.second = buffers[self.next as usize];
                self.offset = remainder;
                list.nodes[next].buffer = buffers[next];
                list.nodes[next].samples = remainder;
                list.nodes[next].complete = 0;
                list.notify(next);
                self.rotations = self.rotations.wrapping_add(1);
            }
        } else {
            self.offset = remainder;
            list.nodes[self.current as usize].samples = remainder;
            list.notify(self.current as usize);
        }
        self.previous_quotient = quotient;
        Ok(())
    }
}

impl SpeechFrame<'_> {
    pub fn from_assembled<'a>(
        frame: &'a crate::queue::SpeechFrame,
        samples: &'a [i16],
    ) -> SpeechFrame<'a> {
        SpeechFrame {
            reset_pitch: frame.started,
            pitch_duration: frame.start_value,
            elapsed_pitch: frame.boundary_value,
            commands: &frame.commands[..frame.command_count],
            samples,
            phonemes: frame.phonemes(),
            voiced_starts: &frame.group_starts[..frame.group_count],
            voiced_lengths: &frame.group_lengths[..frame.group_count],
            group_starts: &frame.output_starts[..frame.output_count],
            group_lengths: &frame.output_lengths[..frame.output_count],
            leading_ticks: frame.current_time,
            trailing_ticks: frame.before_time,
            ticks: &frame.pauses[..frame.pause_count],
            duration_ticks: frame.pause_total,
        }
    }
}

impl crate::queue::SpeechFrame {
    /// fn_1a16a4 / EU fn_1a16f8 / KR fn_1a1790.
    pub fn reset_for_output(&mut self) {
        self.started = false;
        self.start_value = 0;
        self.boundary_value = 0;
        self.character_count = 0;
        self.group_count = 0;
        self.output_count = 0;
        self.current_time = 0;
        self.before_time = 0;
        self.after_time = 0;
        self.pauses.fill(0);
        self.pause_count = 0;
        self.pause_total = 0;
        self.position = u32::MAX;
        self.command_count = 0;
        self.audio.count = 0;
        self.audio.written = 0;
        self.audio.pending = 0;
        self.audio.offsets.fill(0);
        self.audio.lengths.fill(0);
    }
}

pub struct StreamSynthesis<'a, R, F> {
    pub frame: &'a mut crate::queue::SpeechFrame,
    pub audio: &'a mut [i16; crate::queue::AUDIO_CAPACITY],
    pub position: i32,
    pub voice: crate::queue::VoiceBoundary,
    pub unvoiced: &'a [u8; 256],
    pub korean: bool,
    pub renderer: R,
    pub filter: F,
}

impl<R, F> SpeechBackend for StreamSynthesis<'_, R, F>
where
    R: FnMut(&crate::queue::SpeechFrame, &[i16], [&mut [i16]; 2], bool) -> [usize; 2],
    F: FnMut(&mut [i16]),
{
    fn assemble(
        &mut self,
        samples: &[i16],
        events: &[crate::queue::SpeechEvent],
        initial: bool,
        flush: u32,
    ) -> AssemblyStep {
        let result = if self.korean {
            self.frame.assemble_korean(
                self.audio,
                self.position as u32,
                samples,
                events,
                initial,
                flush != 0,
                self.voice,
                self.unvoiced,
            )
        } else {
            self.frame.assemble(
                self.audio,
                self.position as u32,
                samples,
                events,
                initial,
                flush != 0,
                self.voice,
                self.unvoiced,
            )
        };
        AssemblyStep {
            samples: result.samples as usize,
            events: result.events,
            ready: result.ready,
        }
    }
    fn render(&mut self, output: [&mut [i16]; 2], flush: bool) -> [usize; 2] {
        (self.renderer)(
            self.frame,
            &self.audio[..self.frame.audio.written as usize],
            output,
            flush,
        )
    }
    fn reset_frame(&mut self) {
        self.frame.reset_for_output();
    }
    fn state(&self) -> i32 {
        self.frame.position as i32
    }
    fn position(&self) -> i32 {
        self.position
    }
    fn advance(&mut self, samples: usize) {
        self.position = self.position.wrapping_add(samples as i32);
    }
    fn pending(&self) -> i32 {
        self.frame.after_time
    }
    fn add_consumed(&mut self, samples: i32) {
        self.frame.before_time = self.frame.before_time.wrapping_add(samples);
    }
    fn filter(&mut self, samples: &mut [i16]) {
        (self.filter)(samples);
    }
}
