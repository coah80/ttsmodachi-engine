use crate::{math, pitch::Language, tables::NumericTables, voice::VoiceQueue};

use crate::synthesis::{SynthesisBuffers, SynthesisResult, SynthesisState};
impl SynthesisState {
    fn pitch_korean(&self, modulation: &[f32; 1024], tables: &NumericTables<'_>) -> f32 {
        if !self.voice.enabled {
            return 1.0;
        }
        let mut pitch = self.voice.value6c
            * self.voice.base8c
            * math::powf(
                self.period as f32 / self.smoothed_period,
                self.voice.value84,
                tables,
            )
            + modulation[self.pitch_modulation_phase as usize] * self.voice.valuea4;
        if self.minimum_pitch > pitch {
            pitch = self.minimum_pitch;
        }
        if pitch.to_bits() as i32 > 0x4124cccd {
            pitch = 10.3;
        }
        pitch
    }

    /// KR fn_1bd540 / US fn_1bd2f0 / EU fn_1bd3a0: resample grains and overlap-add.
    /// All sample storage, queues, tracker state and EU scratch belong to the caller.
    /// Buffer lengths are the C capacities. Input must include interpolation padding;
    /// valid engine states require positive periods, resampling and output spacing.
    pub fn synthesize_korean(
        &mut self,
        buffers: &mut SynthesisBuffers<'_>,
        incoming: &[i16],
        finish: bool,
        input_events: &mut VoiceQueue,
        commands: &mut VoiceQueue,
        language: Language,
        scratch: &mut [f32],
        frequencies: &[f32; 5],
        tables: &NumericTables<'_>,
    ) -> SynthesisResult {
        let accepted = incoming
            .len()
            .min(buffers.input.len() - self.voice.time as usize);
        let mut length = self.voice.time as usize;
        buffers.input[length..length + accepted].copy_from_slice(&incoming[..accepted]);
        length += accepted;
        if finish && self.input_window > length {
            buffers.input[length..self.input_window].fill(0);
            length = self.input_window;
        }
        self.voice.time = length as i32;
        let start = self.output_position as i32;
        let end = (self.output_remainder + start).min(buffers.output.len() as i32);
        if end > start {
            buffers.output.copy_within(start as usize..end as usize, 0);
        }
        if end > self.output_remainder {
            buffers.output[self.output_remainder as usize..end as usize].fill(0);
        }
        self.output_remainder = 0;
        self.output_position = 0.0;
        self.voice.advance_timers(0);
        self.voice.consume(input_events, Some(commands), 0, tables);
        self.voice.consume(commands, None, 0, tables);
        let mut consumed = 0usize;
        let mut extent = 0i32;
        let mut excess = 0.0;
        while length - consumed >= self.input_window {
            if self.voice.offsetd8 > 0 {
                self.voice.offsetd8 = (self.voice.offsetd8 as f32 + self.voice.position) as i32;
                self.voice.position = 0.0;
                let count = self
                    .voice
                    .offsetd8
                    .min(buffers.output.len() as i32 - self.output_position as i32);
                self.voice.offsetd8 -= count;
                self.output_position += count as f32;
                extent = extent.max(self.output_position as i32);
            } else if self.voice.offsetdc > 0 {
                let count = (self.voice.offsetdc as usize).min(length - consumed);
                consumed += count;
                self.voice.offsetdc -= count as i32;
                self.voice.advance_timers(count as i32);
                self.voice
                    .consume(input_events, Some(commands), count as i32, tables);
            } else {
                let period = self.tracker.track(
                    language,
                    buffers.input,
                    buffers.input,
                    scratch,
                    consumed,
                    length,
                    frequencies,
                    tables,
                );
                self.period = period;
                let grain_length = ((period * 2) as f32 / self.voice.value70) as usize;
                let table_index = ((self.voice.value6c * 8.0) as usize).min(31);
                let normalization = if self.normalize {
                    buffers.normalization[table_index]
                } else {
                    1.0
                };
                let window_step = buffers.window.len() as f32 / grain_length as f32;
                let mut position = 0.0f32;
                let mut window_position = 0.0f32;
                for sample in &mut buffers.grain[..grain_length] {
                    let index = position as usize;
                    let fraction = position - index as f32;
                    *sample = (((1.0 - fraction) * buffers.input[consumed + index] as f32
                        + fraction * buffers.input[consumed + index + 1] as f32)
                        * buffers.window[window_position as usize]
                        * normalization) as i32;
                    position += self.voice.value70;
                    window_position += window_step;
                }
                self.grain_length = grain_length;
                consumed += period as usize;
                let mut stretch = self.voice.gain * self.voice.rate * self.voice.base98
                    + buffers.modulation[self.stretch_modulation_phase as usize]
                        * self.voice.valueb0;
                let mut spacing = period as f32 / self.pitch_korean(buffers.modulation, tables);
                let room =
                    (buffers.output.len() as f32 - self.output_position) / grain_length as f32;
                if stretch > room {
                    excess += period as f32 * (stretch - room);
                    stretch = room;
                }
                self.voice.position += period as f32 * stretch;
                self.stretch_modulation_phase = (self.stretch_modulation_phase
                    + crate::runtime::signed_division(
                        self.voice.durationb4.wrapping_mul(period),
                        self.control_period,
                    )
                    .expect("zero control period")
                    .quotient)
                    % 1024;
                let mut harmony_time = [0.0; 3];
                for (h, time) in harmony_time.iter_mut().enumerate() {
                    *time = self.voice.position + self.voice.phases[h];
                    if self.voice.active[h] && !self.voice.flag {
                        let harmony_spacing = spacing / self.voice.targets[h];
                        let mut harmony_position = self.output_position - self.voice.phases[h];
                        while *time > 0.0 {
                            overlap(
                                buffers.output,
                                &buffers.grain[..grain_length],
                                harmony_position,
                                harmony_spacing,
                                &mut extent,
                            );
                            harmony_position += harmony_spacing;
                            *time -= harmony_spacing;
                        }
                    }
                }
                while spacing <= self.voice.position {
                    if self.voice.flag {
                        let mut count = spacing as i32;
                        let index = self.output_position as usize;
                        if self.output_position + count as f32 >= buffers.output.len() as f32 {
                            count = (buffers.output.len() as f32 - self.output_position) as i32;
                        }
                        for i in 0..count as usize {
                            buffers.output[index + i] =
                                (math::sine(self.tone_phase * 40.7436637878418, tables) * 8192.0)
                                    as i32;
                            self.tone_phase += 6283.185546875 / self.voice.sample_rate;
                            if self.tone_phase.to_bits() as i32 > 0x40c90fdb {
                                self.tone_phase -= 6.2831854820251465;
                            }
                        }
                        extent = extent.max((self.output_position + count as f32) as i32);
                        self.voice.position -= count as f32;
                        self.output_position += count as f32;
                    } else {
                        overlap(
                            buffers.output,
                            &buffers.grain[..grain_length],
                            self.output_position,
                            spacing,
                            &mut extent,
                        );
                        self.output_position += spacing;
                        self.voice.position -= spacing;
                    }
                    self.control_time = self.control_time.wrapping_add(spacing as i32);
                    self.voice.consume(commands, None, spacing as i32, tables);
                    while self.control_time >= self.control_period {
                        if self.voice.coefficient90.to_bits() != 0x3f800000 {
                            self.voice.base8c *= self.voice.coefficient90;
                            if self.voice.base8c > self.voice.target94 {
                                self.voice.base8c = self.voice.target94;
                            }
                        }
                        if self.voice.coefficient9c != 1.0 {
                            self.voice.base98 *= self.voice.coefficient9c;
                            if if self.voice.coefficient9c.to_bits() as i32 > 0x3f800000 {
                                self.voice.base98 >= self.voice.targeta0
                            } else {
                                self.voice.base98 <= self.voice.targeta0
                            } {
                                self.voice.coefficient9c = 1.0;
                            }
                        }
                        self.pitch_modulation_phase =
                            (self.pitch_modulation_phase + self.voice.durationa8) % 1024;
                        self.smoothed_period = (1.0 - self.voice.value88) * self.voice.period
                            + self.voice.value88 * self.smoothed_period;
                        self.control_time -= self.control_period;
                        spacing = period as f32 / self.pitch_korean(buffers.modulation, tables);
                    }
                }
                for (h, time) in harmony_time.iter().enumerate() {
                    if self.voice.active[h] {
                        self.voice.phases[h] = time - self.voice.position;
                    }
                }
                self.voice.advance_timers(period);
                self.voice
                    .consume(input_events, Some(commands), period, tables);
            }
        }
        if self.output_position > buffers.output.len() as f32 {
            let count = (buffers.output.len() as f32 - self.output_position) as i32;
            self.output_position = buffers.output.len() as f32;
            extent -= count;
            excess += count as f32;
        }
        if excess > 0.0 {
            self.voice.offsetd8 = (self.voice.offsetd8 as f32 + excess) as i32;
        }
        let remaining = self.voice.time - self.input_window as i32;
        let produced = self.output_position as usize;
        self.output_remainder = extent - produced as i32;
        if consumed > 0 {
            self.history
                .copy_from_slice(&buffers.input[consumed - 2..consumed]);
            buffers.input.copy_within(consumed..length, 0);
            self.voice.time = (length - consumed) as i32;
        }
        SynthesisResult {
            accepted,
            remaining,
            produced,
        }
    }
}

fn overlap(output: &mut [i32], grain: &[i32], position: f32, spacing: f32, extent: &mut i32) {
    if grain.len() as f32 + position < output.len() as f32 {
        let index = position as i32;
        assert!(index >= 0, "negative harmony output index");
        for (dest, sample) in output[index as usize..index as usize + grain.len()]
            .iter_mut()
            .zip(grain)
        {
            *dest = dest.wrapping_add(*sample);
        }
        *extent = (*extent).max(
            (position
                + if (grain.len() as f32) < spacing {
                    spacing
                } else {
                    grain.len() as f32
                }) as i32,
        );
    }
}
