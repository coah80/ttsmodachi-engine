pub const OUTPUT_SAMPLES: usize = 0x25800;

#[derive(Clone, Copy, Debug, Default)]
pub struct SynthesisBoundary {
    pub state: u32,
    pub offset: u32,
    pub total_bytes: u32,
}

impl SynthesisBoundary {
    /// fn_183398 / EU fn_183448 / KR fn_1833f4.
    /// The host owns output storage, PCM callbacks and publication locks.
    pub fn finish(
        &mut self,
        output: [&mut [i16]; 2],
        mut synthesize: impl FnMut([&mut [i16]; 2]) -> [usize; 2],
        mut callback: impl FnMut(&[i16]),
        mut publish: impl FnMut(&mut Self),
    ) -> [usize; 2] {
        let [first, second] = output;
        let offset = self.offset as usize;
        let second_capacity = if self.state < 2 {
            offset
        } else {
            OUTPUT_SAMPLES
        };
        let mut counts = synthesize([
            &mut first[offset..OUTPUT_SAMPLES],
            &mut second[..second_capacity],
        ]);
        self.total_bytes = self
            .total_bytes
            .wrapping_add(((counts[0] + counts[1]) as u32).wrapping_mul(2));
        if self.total_bytes < 0x400 {
            let mut pad_first = 0x4b000i32.wrapping_sub(counts[0] as i32);
            let mut pad_second = 0x4b000i32.wrapping_sub(counts[1] as i32);
            if pad_first > 0x400 {
                pad_first = 0x400;
                pad_second = 0;
            } else {
                let remaining = 0x400i32.wrapping_sub(pad_second);
                if pad_second > remaining {
                    pad_second = remaining;
                }
            }
            let pads = [pad_first as usize, pad_second as usize];
            first[offset + counts[0]..offset + counts[0] + pads[0]].fill(0);
            second[counts[1]..counts[1] + pads[1]].fill(0);
            counts[0] += pads[0];
            counts[1] += pads[1];
            self.total_bytes = self
                .total_bytes
                .wrapping_add(((pads[0] + pads[1]) as u32).wrapping_mul(2));
        }
        callback(&first[offset..offset + counts[0]]);
        callback(&second[..counts[1]]);
        publish(self);
        counts
    }
}
