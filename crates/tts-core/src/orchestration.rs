use crate::{
    buffers::{SampleFilter, VoiceFilter},
    controls::FilterControls,
    math,
    tables::NumericTables,
    voice::{VoiceQueue, VoiceState},
};

#[derive(Clone, Debug, Default)]
pub struct SetupState {
    pub sample_rate: f32,
    pub active: bool,
    pub polarity: f32,
    pub voice_enabled: bool,
    pub control_enabled: bool,
    pub state: i32,
    pub gain: f32,
    pub pitch: f32,
    pub resampling: f32,
    pub effects: [f32; 3],
    pub smoothing: f32,
    pub blend: f32,
    pub frequency: f32,
    pub voice_gain: f32,
    pub value4c08: f32,
    pub timing_blend: f32,
    pub duration_scale: f32,
    pub value4c14: f32,
    pub value4c18: f32,
    pub tempo: f32,
    pub modulation: [f32; 2],
    pub stretch: [f32; 2],
    pub enabled: bool,
    pub mode: i32,
    pub reset_mode: i32,
    pub reset_count: i32,
}

pub struct Setup<'a, 'b> {
    pub state: &'a mut SetupState,
    pub voice_active: bool,
    pub voice: &'a mut VoiceState,
    pub input: &'a mut VoiceQueue,
    pub commands: &'a mut VoiceQueue,
    pub filter: &'a mut SampleFilter<'b>,
    pub voice_filter: &'a mut VoiceFilter<'b>,
    pub controls: &'a mut FilterControls,
}

impl Setup<'_, '_> {

    pub fn reset(&mut self, tables: &NumericTables<'_>) {
        let s = &mut self.state;
        s.active = false;
        s.polarity = 1.0;
        s.voice_enabled = false;
        s.control_enabled = false;
        s.state = 0;
        s.gain = 1.0;
        s.pitch = 1.0;
        s.resampling = 1.0;
        s.effects = [1.0; 3];
        s.smoothing = 0.05;
        s.blend = 1.0;
        s.frequency = 185.0;
        s.voice_gain = 1.0;
        s.value4c08 = 1.0;
        s.timing_blend = 1.0;
        s.duration_scale = 1.0;
        s.value4c14 = 1.0;
        s.value4c18 = 1.0;
        s.tempo = 120.0;
        s.modulation = [0.0; 2];
        s.stretch = [0.0; 2];
        s.enabled = false;
        s.mode = 1;
        self.voice
            .drain_events(self.voice_active, self.input, self.commands, tables);
        let base = self.voice.time;
        self.input.enable_voicing(base, 0);
        self.input.set_pitch_blend(base, 0, s.polarity * s.blend);
        self.input.set_pitch(base, 0, s.pitch);
        self.input.set_gain(base, 0, s.voice_gain);
        self.input.set_resampling(base, 0, s.resampling);
        self.input.set_frequency(base, 0, s.frequency);
        for i in 0..3 {
            self.input.set_harmony(
                base,
                s.effects[i].to_bits() != 1.0f32.to_bits(),
                i as i32,
                0,
                s.effects[i],
            );
        }
        self.input.set_smoothing(base, 0, s.smoothing);
        self.input
            .enqueue_duration(base, 0, s.modulation[0], s.modulation[1]);
        self.input
            .set_stretch_modulation(base, 0, s.stretch[0], s.stretch[1]);
        self.filter.reset();
        self.voice_filter.reset();
    }

    pub fn apply_commands(&mut self, commands: &[u32], delay: i32, tables: &NumericTables<'_>) {
        for &command in commands {
            if command == 1 {
                self.reset(tables);
                continue;
            }
            let s = &mut self.state;
            let base = self.voice.time;
            if command < 10_000_000 {
                match command {
                    2 | 3 => {
                        s.polarity = if command == 2 { 1.0 } else { -1.0 };
                        self.input
                            .set_pitch_blend(base, delay, s.polarity * s.blend);
                    }
                    4 => s.voice_enabled = true,
                    5 => {
                        self.voice_filter.reset();
                        s.voice_enabled = false;
                    }
                    6 | 7 => {
                        s.enabled = command == 6;
                        self.input.enqueue_flag(base, s.enabled as u32, delay);
                    }
                    8..=11 => s.mode = command as i32 - 8,
                    20..=22 => {
                        s.reset_mode = command as i32 - 20;
                        s.reset_count = 0;
                    }
                    _ => {}
                }
                continue;
            }
            let value = (command % 10_000_000) as i32;
            match command / 10_000_000 {
                1 => {
                    s.pitch = value as f32 * 0.0001;
                    self.input.set_pitch(base, delay, s.pitch);
                }
                2 | 3 => {
                    let duration = value / 10000;
                    let mut note = value - duration * 10000;
                    if command / 10_000_000 == 2 {
                        s.frequency = note as f32;
                    } else {
                        let negative = note / 1000;
                        note -= negative * 1000;
                        if negative != 0 {
                            note = -note;
                        }
                        s.frequency =
                            math::powf(2.0, note as f32 * (1.0 / 12.0), tables) * s.frequency;
                    }
                    let samples =
                        ((60.0 / s.tempo) * (duration as f32 / 48.0) * s.sample_rate) as i32;
                    self.input.schedule_frequency(base, samples, s.frequency);
                }
                20 | 22 | 23 => {
                    let index = match command / 10_000_000 {
                        20 => 0,
                        22 => 1,
                        _ => 2,
                    };
                    s.effects[index] = value as f32 * 0.0001;
                    self.input.set_harmony(
                        base,
                        value != 10000,
                        index as i32,
                        delay,
                        s.effects[index],
                    );
                }
                4 => {
                    s.voice_gain = value as f32 * 0.0001;
                    self.input.set_gain(base, delay, s.voice_gain);
                }
                5 => {
                    s.resampling = value as f32 * 0.0001;
                    self.input.set_resampling(base, delay, s.resampling);
                }
                6 => {
                    s.blend = value as f32 * 0.0001;
                    self.input
                        .set_pitch_blend(base, delay, s.blend * s.polarity);
                }
                7 => {
                    s.smoothing = value as f32 * 0.001;
                    self.input.set_smoothing(base, delay, s.smoothing);
                }
                8 => s.timing_blend = value as f32 * 0.0001,
                9 => s.duration_scale = value as f32 * 0.0001,
                10 => s.tempo = value as f32 * 0.01,
                11 | 12 => {
                    let index = (command / 10_000_000 - 11) as usize;
                    s.modulation[index] = value as f32 * if index == 0 { 0.0001 } else { 0.001 };
                    self.input
                        .enqueue_duration(base, delay, s.modulation[0], s.modulation[1]);
                }
                13 | 14 => {
                    let index = (command / 10_000_000 - 13) as usize;
                    s.stretch[index] = value as f32 * if index == 0 { 0.0001 } else { 0.001 };
                    self.input
                        .set_stretch_modulation(base, delay, s.stretch[0], s.stretch[1]);
                }
                18 => {
                    s.control_enabled = value != 0;
                    self.controls.set(2, value);
                }
                19 => self.controls.set(3, value),
                24 => s.state = value,
                25 => s.gain = value.clamp(0, 10000) as f32 * 0.0001,
                _ => {}
            }
        }
    }
}
