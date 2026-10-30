use crate::{
    buffers::{SampleFilter, VoiceFilter},
    effects::VoiceEffect,
    pitch::Language,
    synthesis::{SynthesisBuffers, SynthesisState},
    tables::NumericTables,
    voice::VoiceQueue,
};

pub struct FrameSynthesis<'a, 'b, 'c> {
    pub state: &'a mut SynthesisState,
    pub buffers: SynthesisBuffers<'b>,
    pub input_events: &'a mut VoiceQueue,
    pub commands: &'a mut VoiceQueue,
    pub language: Language,
    pub scratch: &'a mut [f32],
    pub frequencies: &'a [f32; 5],
    pub tables: &'a NumericTables<'b>,
    pub bypass_effects: bool,
    pub voice_effect: Option<(
        &'a mut VoiceEffect,
        &'a mut VoiceFilter<'c>,
        &'a [i16; 1024],
    )>,
    pub sample_effect: Option<(&'a mut SampleFilter<'c>, i32, u32)>,
}

impl FrameSynthesis<'_, '_, '_> {
    fn effects(&mut self, samples: &mut [i16]) {
        if !self.bypass_effects {
            if let Some((effect, filter, wave)) = &mut self.voice_effect {
                effect.process(filter, wave, samples);
            }
            if let Some((filter, gain, delay)) = &mut self.sample_effect {
                filter.process(*gain, *delay, samples);
            }
        }
    }

    /// fn_1b7e6c / EU fn_1b7f1c / KR fn_1b80b8.
    /// Clip overlap-add output into two spans, applying effects in stream order.
    pub fn render(
        &mut self,
        mut incoming: &[i16],
        first: &mut [i16],
        second: &mut [i16],
        finish: bool,
    ) -> [usize; 2] {
        let mut counts = [0; 2];
        let mut initial = true;
        while !incoming.is_empty() || initial {
            let synthesize = if self.language == Language::Korean {
                SynthesisState::synthesize_korean
            } else {
                SynthesisState::synthesize
            };
            let result = synthesize(
                self.state,
                &mut self.buffers,
                incoming,
                finish,
                self.input_events,
                self.commands,
                self.language,
                self.scratch,
                self.frequencies,
                self.tables,
            );
            initial = false;
            incoming = &incoming[result.accepted..];
            let n1 = result.produced.min(first.len() - counts[0]);
            let n2 = (result.produced - n1).min(second.len() - counts[1]);
            let span1 = &mut first[counts[0]..counts[0] + n1];
            for (dest, source) in span1.iter_mut().zip(&self.buffers.output[..n1]) {
                *dest = (*source).clamp(-32768, 32767) as i16;
            }
            self.effects(span1);
            let span2 = &mut second[counts[1]..counts[1] + n2];
            for (dest, source) in span2.iter_mut().zip(&self.buffers.output[n1..n1 + n2]) {
                *dest = (*source).clamp(-32768, 32767) as i16;
            }
            self.effects(span2);
            counts[0] += n1;
            counts[1] += n2;
        }
        counts
    }
}
