use crate::{
    tables::NumericTables,
    voice::{VoiceQueue, VoiceState},
};

impl VoiceQueue {
    /// fn_1bd26c / EU fn_1bd31c / KR fn_1bd4bc.
    pub fn set_gain(&mut self, base: i32, delay: i32, value: f32) -> u32 {
        self.enqueue(base, 2, delay, value)
    }
    /// fn_1bd27c / EU fn_1bd32c / KR fn_1bd4cc.
    pub fn set_pitch(&mut self, base: i32, delay: i32, value: f32) -> u32 {
        self.enqueue(base, 0, delay, value)
    }
    /// fn_1bd28c / EU fn_1bd33c / KR fn_1bd4dc.
    pub fn set_resampling(&mut self, base: i32, delay: i32, value: f32) -> u32 {
        self.enqueue(base, 1, delay, value)
    }
    /// fn_1bd29c / EU fn_1bd34c / KR fn_1bd4ec.
    pub fn set_pitch_blend(&mut self, base: i32, delay: i32, value: f32) -> u32 {
        self.enqueue(base, 8, delay, value)
    }
    /// fn_1bd2ac / EU fn_1bd35c / KR fn_1bd4fc.
    pub fn set_smoothing(&mut self, base: i32, delay: i32, value: f32) -> u32 {
        self.enqueue(base, 7, delay, value)
    }
    /// fn_1bd2bc / EU fn_1bd36c / KR fn_1bd50c.
    pub fn set_stretch_modulation(
        &mut self,
        base: i32,
        delay: i32,
        depth: f32,
        frequency: f32,
    ) -> i32 {
        self.enqueue_pair(base, 12, delay, depth, frequency)
    }
    /// fn_1bdff0 / EU fn_1be0a0 / KR fn_1be224.
    pub fn set_harmony(&mut self, base: i32, enabled: bool, index: i32, delay: i32, ratio: f32) {
        self.enqueue_pair(
            base,
            index.wrapping_add(4),
            delay,
            if enabled { 1.0 } else { 0.0 },
            ratio,
        );
    }
    /// fn_1c16c4 / EU fn_1c16e0 / KR fn_1c192c.
    pub fn enable_voicing(&mut self, base: i32, delay: i32) -> u32 {
        self.enqueue(base, 14, delay, 0.0)
    }
    /// fn_1c18c8 / EU fn_1c18e4 / KR fn_1c1b30.
    pub fn set_frequency(&mut self, base: i32, delay: i32, value: f32) -> u32 {
        self.enqueue(base, 3, delay, value)
    }
    /// fn_1b78f8 / EU fn_1b79a8 / KR fn_1b7b48.
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
    /// fn_1bccc0 / EU fn_1bcd70 / KR fn_1bcf10.
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
    /// fn_1b7908 / EU fn_1b79b8 / KR fn_1b7b58.
    pub fn insert_silence(&mut self, base: i32, samples: i32, delay: i32) {
        self.enqueue(base, 18, delay, samples as f32);
    }
    /// fn_1b7920 / EU fn_1b79d0 / KR fn_1b7b70.
    pub fn skip_samples(&mut self, base: i32, samples: i32, delay: i32) {
        if samples > 0 {
            self.enqueue(base, 19, delay, samples as f32);
        }
    }
}

impl VoiceState {
    /// fn_1c16dc / EU fn_1c16f8 / KR fn_1c1944: drain both queues when active.
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
