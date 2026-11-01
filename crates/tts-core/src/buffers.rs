
pub struct RingBuffer<'a, T> {
    pub samples: &'a mut [T],
    pub position: u32,
}

pub struct SampleFilter<'a> {
    pub ring: RingBuffer<'a, u16>,
    pub history: f32,
}

impl SampleFilter<'_> {

    pub fn reset(&mut self) {
        self.ring.samples.fill(0);
        self.ring.position = 0;
        self.history = 0.0;
    }

    pub fn process(&mut self, gain: i32, delay: u32, samples: &mut [i16]) {
        let ring = &mut self.ring;
        let length = ring.samples.len() as u32;
        for sample in samples {
            let index = if ring.position > delay {
                ring.position - delay
            } else {
                ring.position.wrapping_sub(delay).wrapping_add(length) % length
            };
            let delayed = ring.samples[index as usize] as i16 as i32;
            let value = (*sample as i32).wrapping_add(delayed.wrapping_mul(gain) / 127);
            let value = value.clamp(-32768, 32767) as i16;
            ring.samples[ring.position as usize] = value as u16;
            ring.position += 1;
            if ring.position >= length {
                ring.position = 0;
            }
            *sample = value;
        }
    }
}

pub struct VoiceFilter<'a> {
    pub rings: [RingBuffer<'a, f32>; 3],
    pub history: f32,
}

impl VoiceFilter<'_> {

    pub fn reset(&mut self) {
        for ring in &mut self.rings {
            ring.samples.fill(0.0);
            ring.position = 0;
        }
        self.history = 0.0;
    }
}

pub const SAMPLE_DELAY_CAPACITY: usize = 8000;
pub const VOICE_DELAY_CAPACITY: usize = 1024;
