use crate::{
    tables::NumericTables,
    voice::{VoiceQueue, VoiceState},
};

impl VoiceQueue {

    pub fn set_gain(&mut self, base: i32, delay: i32, value: f32) -> u32 {
        self.enqueue(base, 2, delay, value)
    }

    pub fn set_pitch(&mut self, base: i32, delay: i32, value: f32) -> u32 {
        self.enqueue(base, 0, delay, value)
    }

    pub fn set_resampling(&mut self, base: i32, delay: i32, value: f32) -> u32 {
        self.enqueue(base, 1, delay, value)
    }

    pub fn set_pitch_blend(&mut self, base: i32, delay: i32, value: f32) -> u32 {
        self.enqueue(base, 8, delay, value)
    }

    pub fn set_smoothing(&mut self, base: i32, delay: i32, value: f32) -> u32 {
        self.enqueue(base, 7, delay, value)
    }

    pub fn set_stretch_modulation(
        &mut self,
        base: i32,
        delay: i32,
        depth: f32,
        frequency: f32,
    ) -> i32 {
        self.enqueue_pair(base, 12, delay, depth, frequency)
    }

    pub fn set_harmony(&mut self, base: i32, enabled: bool, index: i32, delay: i32, ratio: f32) {
        self.enqueue_pair(
            base,
            index.wrapping_add(4),
            delay,
            if enabled { 1.0 } else { 0.0 },
            ratio,
        );
    }

    pub fn enable_voicing(&mut self, base: i32, delay: i32) -> u32 {
        self.enqueue(base, 14, delay, 0.0)
    }

    pub fn set_frequency(&mut self, base: i32, delay: i32, value: f32) -> u32 {
        self.enqueue(base, 3, delay, value)
    }

    pub fn pitch_transition(
        &mut self,
        base: i32,
        delay: i32,
        start: f32,
        target: f32,
        duration: f32,
    ) {
        self.enqueue_transition(base, 9, delay, start, target, duration);
    }

    pub fn gain_transition(
        &mut self,
        base: i32,
        delay: i32,
        start: f32,
        target: f32,
        duration: f32,
    ) {
        self.enqueue_transition(base, 10, delay, start, target, duration);
    }

    pub fn insert_silence(&mut self, base: i32, samples: i32, delay: i32) {
        self.enqueue(base, 18, delay, samples as f32);
    }

    pub fn skip_samples(&mut self, base: i32, samples: i32, delay: i32) {
        if samples > 0 {
            self.enqueue(base, 19, delay, samples as f32);
        }
    }
}

impl VoiceState {

    pub fn drain_events(
        &mut self,
        active: bool,
        input: &mut VoiceQueue,
        commands: &mut VoiceQueue,
        tables: &NumericTables<'_>,
    ) {
        if !active {
            return;
        }
        while input.read != input.write {
            self.consume(
                input,
                Some(commands),
                input.events[input.read as usize].delay,
                tables,
            );
        }
        while commands.read != commands.write {
            self.consume(
                commands,
                None,
                commands.events[commands.read as usize].delay,
                tables,
            );
        }
    }
}
