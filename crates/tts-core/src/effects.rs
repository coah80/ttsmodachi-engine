use crate::{buffers::RingBuffer, math::floorf};

pub struct VoiceEffect {
    pub gain: i32,
    pub feedforward: [i32; 2],
    pub feedback: [i32; 2],
    pub delay: [i32; 2],
    pub rate: [i32; 2],
    pub depth: [i32; 2],
    pub phase: [i32; 2],
}

fn read(ring: &RingBuffer<'_, f32>, delay: f32, bounds: [f32; 2]) -> f32 {
    let sample = |index: u32| {
        let offset = if ring.position > index {
            ring.position - index
        } else {
            ring.position
                .wrapping_sub(index)
                .wrapping_add(ring.samples.len() as u32)
                % ring.samples.len() as u32
        };
        ring.samples[offset as usize]
    };
    let mut y0 = sample(delay as u32);
    let mut y1 = sample((delay + 1.0) as u32);
    let [mut x0, mut x1] = bounds;
    if x0 == x1 {
        return y0;
    }
    if x0 > x1 {
        core::mem::swap(&mut x0, &mut x1);
        core::mem::swap(&mut y0, &mut y1);
    }
    (delay - x0) * (y1 - y0) / (x1 - x0) + y0
}

fn write(ring: &mut RingBuffer<'_, f32>, value: f32) {
    ring.samples[ring.position as usize] = value;
    ring.position += 1;
    if ring.position as usize >= ring.samples.len() {
        ring.position = 0;
    }
}

impl VoiceEffect {

    pub fn process(
        &mut self,
        filter: &mut crate::buffers::VoiceFilter<'_>,
        wave: &[i16; 1024],
        samples: &mut [i16],
    ) {
        for sample in samples {
            let delay: [f32; 2] = core::array::from_fn(|i| {
                let index = self.phase[i].wrapping_mul(1024) / 65536;
                self.delay[i] as f32
                    + (wave[index as usize] as i32).wrapping_mul(self.depth[i]) as f32
                        * (1.0 / 32768.0)
            });
            let input = *sample as f32;
            let mut output = 0.0;
            for i in 0..2 {
                let bounds = [floorf(delay[i]), floorf(delay[i] + 1.0)];
                let delayed = read(&filter.rings[0], delay[i], bounds);
                let mut value = self.gain as f32 * input * (1.0 / 127.0)
                    - delayed * self.feedforward[i] as f32 * (1.0 / 127.0);
                let delayed = read(&filter.rings[i + 1], delay[i], bounds);
                value -= delayed * self.feedback[i] as f32 * (1.0 / 127.0);
                write(&mut filter.rings[i + 1], value);
                output += value;
                if i == 0 {
                    write(&mut filter.rings[0], input);
                }
            }
            filter.history = output;
            let mut clipped = output * 0.5;
            if clipped.to_bits() as i32 > 0x46fffe00 {
                clipped = 32767.0;
            } else if clipped.to_bits() > 0xc7000000 {
                clipped = -32768.0;
            }
            *sample = clipped as i32 as i16;
            for i in 0..2 {
                self.phase[i] = self.phase[i].wrapping_add(self.rate[i]) & 0xffff;
            }
        }
    }
}
