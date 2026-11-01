pub struct PhonemeFrame<'a> {
    pub count: i32,
    pub symbols: &'a [u16; 128],
    pub starts: &'a [i32; 128],
}

impl PhonemeFrame<'_> {

    pub fn leading_sample(&self) -> i32 {
        if self.count <= 1 {
            return 0;
        }
        let mut index = 0;
        while self.symbols[index] == 0x23 && self.starts[0] == 0 {
            index += 1;
        }
        self.starts[index]
    }
}

pub struct LipSync<'a> {
    pub samples: &'a mut [i16; 0x301],
    pub count: i32,
    pub limit: i32,
}

pub struct TimingSegments<'a> {
    pub scales: &'a mut [f32; 128],
    pub positions: &'a mut [i32; 128],
}

impl TimingSegments<'_> {

    pub fn add_simple(
        &mut self,
        queue: &mut crate::voice::VoiceQueue,
        base: i32,
        duration: i32,
        start: i32,
        denominator: i32,
        index: usize,
    ) -> bool {
        if denominator == 0 {
            return false;
        }
        let target = denominator as f32;
        let ratio = duration as f32 / target;
        queue.enqueue_pair(base, 15, start, ratio, target);
        if index >= 128 {
            return false;
        }
        self.positions[index] = start;
        self.scales[index] = ratio;
        true
    }
}

pub struct PlannedSegments<'a> {
    pub starts: &'a mut [i32; 128],
    pub lengths: &'a mut [i32; 128],
    pub voiced: &'a mut [u8; 128],
    pub output: TimingSegments<'a>,
}

impl PlannedSegments<'_> {

    pub fn plan(
        &mut self,
        queue: &mut crate::voice::VoiceQueue,
        base_position: i32,
        base: i32,
        length: i32,
        input_starts: &[i32],
        input_lengths: &[i32],
        limit: i32,
        mode: i32,
        desired: i32,
        offset: usize,
        total: f32,
        blend: f32,
    ) -> i32 {
        if length == 0 {
            return 0;
        }
        let fallback = |this: &mut Self, queue: &mut crate::voice::VoiceQueue| {
            this.output
                .add_simple(queue, base_position, total as i32, base, length, offset)
                as i32
        };
        if mode == 0 {
            return fallback(self, queue);
        }
        let mut first = 0;
        let mut last = limit;
        if mode == 2 {
            last = 1;
        } else if mode == 3 {
            first = limit - 1;
            if first < 0 {
                return fallback(self, queue);
            }
        }
        if last > limit {
            return fallback(self, queue);
        }
        let n = last - first + 1;
        let mut count = 0usize;
        let mut previous = 0;
        let mut end = 0;
        let mut i = 1;
        while n > i {
            let index = first as usize;
            let start = input_starts[index];
            let duration = input_lengths[index];
            end = start.wrapping_add(duration);
            if start == previous {
                if count > 0 && self.voiced[count - 1] != 0 {
                    self.lengths[count - 1] = self.lengths[count - 1].wrapping_add(duration);
                } else {
                    self.starts[count] = previous;
                    self.lengths[count] = duration;
                    self.voiced[count] = 1;
                    count += 1;
                }
            } else {
                self.starts[count] = previous;
                self.lengths[count] = start.wrapping_sub(previous);
                self.voiced[count] = 0;
                count += 1;
                previous = start;
                self.starts[count] = previous;
                self.lengths[count] = end.wrapping_sub(previous);
                self.voiced[count] = 1;
                count += 1;
            }
            previous = end;
            i += 1;
            first += 1;
        }
        if length > end {
            self.starts[count] = end;
            self.lengths[count] = length - end;
            self.voiced[count] = 0;
            count += 1;
        }
        let mut voiced_length = 0i32;
        let mut unvoiced_length = 0i32;
        for index in 0..count {
            if self.voiced[index] != 0 {
                voiced_length = voiced_length.wrapping_add(self.lengths[index]);
            } else {
                unvoiced_length = unvoiced_length.wrapping_add(self.lengths[index]);
            }
        }
        let mut adjusted = length.wrapping_sub(voiced_length);
        let mut unvoiced_scale = 1.0f32;
        if desired > 0 && unvoiced_length > 0 {
            adjusted = (desired as f32 * blend + unvoiced_length as f32 * (1.0 - blend)) as i32;
            unvoiced_scale = adjusted as f32 / unvoiced_length as f32;
        }
        if voiced_length == 0 {
            return fallback(self, queue);
        }
        let voiced_scale = (total - adjusted as f32) / voiced_length as f32;
        if voiced_scale <= 0.0 {
            return fallback(self, queue);
        }
        for index in 0..count {
            let position = self.starts[index].wrapping_add(base);
            let scale = if self.voiced[index] != 0 {
                voiced_scale
            } else {
                unvoiced_scale
            };
            queue.enqueue_pair(
                base_position,
                15,
                position,
                scale,
                self.lengths[index] as f32,
            );
            self.output.positions[index + offset] = position;
            self.output.scales[index + offset] = scale;
        }
        count as i32
    }
}

impl LipSync<'_> {

    pub fn add_duration(&mut self, mut delta: i32, mode: i32) {
        if mode <= 40 {
            delta = delta.wrapping_neg();
        }
        while delta != 0 {
            let count = self.count;
            let current = if count > 0 {
                self.samples[count as usize] as i32
            } else {
                0
            };
            if delta >> 31 == current >> 31
                && self.limit != count
                && current != 0x7fff
                && current != -0x8000
                && current != 0
            {
                let next = current.wrapping_add(delta).clamp(-0x8000, 0x7fff);
                self.samples[count as usize] = next as i16;
                delta = delta.wrapping_sub(next.wrapping_sub(current));
            } else {
                let mut used = delta;
                if count < 0x300 {
                    used = used.clamp(-0x8000, 0x7fff);
                    self.count += 1;
                    self.samples[self.count as usize] = used as i16;
                }
                delta = delta.wrapping_sub(used);
            }
        }
    }
}
