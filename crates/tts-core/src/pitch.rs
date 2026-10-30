use crate::{math::cosine, tables::NumericTables};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PeriodTracker {
    pub min_period: i32,
    pub max_period: i32,
    pub period: i32,
    pub period_delta: i32,
    pub reset: i8,
    pub stable_count: i32,
    pub polarity: i32,
}

#[derive(Clone, Copy, Debug)]
pub struct EuTracker {
    pub period: PeriodTracker,
    pub previous: f32,
    pub older: f32,
    pub oldest: f32,
    pub coefficient: f32,
    pub sample_rate: f32,
}

/// fn_1c1218's inline window_score: split accumulators preserve VFP rounding.
/// Both windows must fit in `samples`.
pub fn window_score(samples: &[i16], start: usize, candidate: usize, length: usize) -> f32 {
    let mut sum = 0.0;
    let mut second_sum = 0.0;
    if length > 1 {
        let offset = usize::from(length & 1 == 0);
        if offset != 0 {
            let difference = (samples[start] as i32 - samples[candidate] as i32) as f32;
            sum += difference * difference;
        }
        for pair in 0..(length - 1) / 2 {
            let index = offset + pair * 2;
            let a = (samples[start + index] as i32 - samples[candidate + index] as i32) as f32;
            let b =
                (samples[start + index + 1] as i32 - samples[candidate + index + 1] as i32) as f32;
            sum += a * a;
            second_sum += b * b;
        }
        sum += second_sum;
    }
    if length > 0 {
        let difference =
            (samples[start + length - 1] as i32 - samples[candidate + length - 1] as i32) as f32;
        sum += difference * difference;
    }
    if sum == 0.0 {
        10000000.0
    } else {
        1.0 / sum
    }
}

/// fn_1c1218, bound as native_us_pitch: US zero-crossing pitch tracker.
/// Samples need readable padding through each candidate plus max_period / 2.
/// `start + min_period` must remain readable even for an empty search.
pub fn track_us(tracker: &mut PeriodTracker, samples: &[i16], start: usize, end: usize) -> i32 {
    track_crossings(tracker, samples, start, end, false)
}

/// KR fn_1c144c, counterpart of US fn_1c1218.
/// Uses the US sample padding contract and reverses selection after stability 2.
pub fn track_kr(tracker: &mut PeriodTracker, samples: &[i16], start: usize, end: usize) -> i32 {
    track_crossings(tracker, samples, start, end, true)
}

fn track_crossings(
    tracker: &mut PeriodTracker,
    samples: &[i16],
    start: usize,
    end: usize,
    korean: bool,
) -> i32 {
    assert!(tracker.min_period >= 0 && tracker.max_period >= tracker.min_period);
    assert!(start <= end && end <= samples.len());
    let min_period = tracker.min_period;
    let max_period = tracker.max_period;
    let stable = tracker.stable_count;
    let span = if stable > 3 {
        ((tracker.period >> 1) * 3).min(max_period)
    } else {
        max_period
    };
    let span = span.min((end - start) as i32);
    let mut selected = min_period;
    let mut previous = 0;
    let mut count = 0;
    let mut candidates = [0usize; 8];
    let mut peak = samples[start + min_period as usize];
    let mut index = start + min_period as usize;
    while index < (start as i32 + span) as usize && count < 8 {
        let sample = if tracker.polarity < 0 {
            samples[index].wrapping_neg()
        } else {
            samples[index]
        };
        let magnitude = sample.wrapping_abs();
        if peak < magnitude {
            peak = magnitude;
        }
        if previous < 0 && sample > 0 {
            if peak <= 128 {
                peak = 0;
            } else {
                if end <= min_period as usize + index {
                    break;
                }
                peak = 0;
                candidates[count] = index;
                count += 1;
            }
        }
        previous = sample;
        index += 1;
    }
    if count == 0 {
        tracker.reset = 1;
        tracker.period = selected;
        tracker.stable_count = 0;
        return selected;
    }
    let mut scores = [0.0; 8];
    let mut best = 0.0;
    let length = (end - start).min((max_period >> 1) as usize);
    for index in 0..count {
        let candidate = candidates[index];
        let distance = ((candidate - start) as i32 - tracker.period).abs();
        let penalty = 1.0 + distance as f32 * 0.04 * stable as f32;
        let divisor = if penalty.to_bits() as i32 > 0x40400000 {
            10.0
        } else {
            1.0
        };
        scores[index] = (window_score(samples, start, candidate, length)
            - window_score(samples, start, start + (candidate - start) / 2, length))
            / divisor;
        if scores[index] > best {
            best = scores[index];
        }
    }
    best *= 0.8;
    for step in 0..count {
        let index = if korean && stable > 2 {
            count - 1 - step
        } else {
            step
        };
        if scores[index] >= best {
            selected = (candidates[index] - start) as i32;
            break;
        }
    }
    tracker.period_delta = selected - tracker.period;
    tracker.period = selected;
    if tracker.period_delta.wrapping_add(25) as u32 >= 51 {
        tracker.reset = 1;
        tracker.stable_count = 0;
        tracker.period_delta = 0;
    } else {
        tracker.stable_count = stable + 1;
    }
    selected
}

/// EU fn_1c7164: select a peak, then prefer the nearest qualifying crossing.
/// `lo` must be readable even when `hi <= lo`; otherwise `lo..hi` must fit.
pub fn select_eu_peak(
    tracker: &mut PeriodTracker,
    peaks: &[i16],
    lo: usize,
    hi: usize,
    stable: i32,
) -> usize {
    let mut result = lo;
    let mut peak = -32768;
    let polarity = tracker.polarity;
    for index in lo..hi {
        let sample = peaks[index];
        let value = if polarity == 1 || (polarity == 0 && sample < 0) {
            sample.wrapping_neg()
        } else {
            sample
        };
        if value as i32 > peak {
            peak = value as i32;
            result = index;
        }
    }
    let sample = peaks[result];
    let sign = if sample > 0 {
        tracker.polarity = -1;
        1
    } else if sample < 0 {
        tracker.polarity = 1;
        -1
    } else {
        return result;
    };
    let level = peak / 20;
    let threshold = peak * 66 / 100;
    let mut best = i32::MAX;
    let mut start = lo;
    let mut b = 0;
    let mut c = 0;
    let mut previous = 0;
    for index in lo..hi {
        let value = peaks[index].wrapping_mul(sign) as i32;
        if value > level {
            if previous <= level {
                start = index;
                c = 0;
            }
            if value < previous && previous >= b && c < previous {
                c = previous;
                start = index - 1;
            }
        } else if level < previous && level < c {
            b = threshold;
            if b < c {
                b = (index - lo) as i32 + tracker.min_period - tracker.period;
                b = b.abs();
                if best > b {
                    best = b;
                    result = start;
                    if stable <= 3 {
                        return result;
                    }
                }
            }
        }
        b = previous;
        previous = value;
    }
    result
}

/// EU fn_1c12c8: resonator-guided pitch tracker, with caller-owned float scratch.
/// `frequencies` is the external D_8bf7dc table. `scratch` needs max_period entries.
/// Samples need max_period entries after start. Peaks need padding through the
/// selected search offset, even when the logical end precedes that offset.
pub fn track_eu(
    tracker: &mut EuTracker,
    samples: &[i16],
    peaks: &[i16],
    scratch: &mut [f32],
    start: usize,
    end: usize,
    frequencies: &[f32; 5],
    tables: &NumericTables<'_>,
) -> i32 {
    assert!(tracker.period.min_period >= 0 && tracker.period.max_period >= 0);
    assert!(start <= end && end <= samples.len() && end <= peaks.len());
    let mut max_offset =
        (tracker.period.period + (tracker.period.period >> 1)).min(tracker.period.max_period);
    let mut min_offset = tracker.period.min_period;
    if tracker.period.stable_count < 3 {
        let length = tracker.period.max_period as usize;
        for index in 0..length {
            scratch[index] = samples[start + index] as f32 * 0.000030517578125;
        }
        let mut scores = [0.0; 5];
        let mut best = 0;
        for index in 0..5 {
            tracker.previous = 0.0;
            tracker.older = 0.0;
            tracker.oldest = 0.0;
            tracker.sample_rate = 16000.0;
            tracker.coefficient =
                cosine(frequencies[index] * 6.2832 / 16000.0 * 40.743664, tables) * 2.0;
            for &sample in &scratch[..length] {
                let a = tracker.previous;
                let b = tracker.older;
                tracker.oldest = b;
                tracker.older = a;
                let energy = a * a + b * b - a * b * tracker.coefficient;
                tracker.previous = tracker.coefficient * a - b + sample;
                if energy > scores[index] {
                    scores[index] = energy;
                }
            }
            if scores[index] > scores[best] {
                best = index;
            }
        }
        let threshold = scores[best] * 0.8;
        let index = (0..5)
            .rev()
            .find(|&i| scores[i] >= threshold)
            .unwrap_or(best);
        max_offset = if index >= 2 {
            (16000.0 / frequencies[index - 2]) as i32
        } else {
            266
        };
        min_offset = if index == 4 {
            34
        } else {
            (16000.0 / frequencies[index + 1]) as i32
        };
    }
    max_offset = max_offset.min((end - start) as i32);
    let polarity = tracker.period.polarity;
    let stable = tracker.period.stable_count;
    let pos = select_eu_peak(
        &mut tracker.period,
        peaks,
        start + min_offset as usize,
        start + max_offset as usize,
        stable,
    );
    let mut reset = tracker.period.reset;
    let amplitude = (peaks[pos] as i32).abs();
    let threshold = amplitude / 20;
    if reset != 0 {
        if amplitude > 512 {
            reset = 0;
        } else {
            return reset_eu(tracker, reset);
        }
    } else {
        if amplitude < 64 {
            return reset_eu(tracker, 1);
        }
        tracker.period.polarity = polarity;
    }
    let count = start as i32 + max_offset - pos as i32;
    let mut distance = count;
    if count > 1 {
        let polarity = tracker.period.polarity;
        for offset in 1..count as usize {
            let index = pos + offset;
            if polarity < 1
                && samples[index - 1] as i32 > threshold
                && samples[index] as i32 <= threshold
            {
                distance = offset as i32;
                tracker.period.polarity = -1;
                break;
            }
            if polarity > -1
                && (samples[index - 1] as i32) < threshold
                && samples[index] as i32 >= threshold
            {
                distance = offset as i32;
                tracker.period.polarity = 1;
                break;
            }
        }
    }
    let period = pos as i32 + distance - start as i32;
    let delta = period - tracker.period.period;
    tracker.period.period_delta = delta;
    tracker.period.stable_count += 1;
    if delta.wrapping_add(25) as u32 >= 51 {
        tracker.period.reset = 1;
        tracker.period.stable_count = 0;
        tracker.period.period_delta = 0;
    }
    tracker.period.reset = reset;
    tracker.period.period = period;
    period
}

fn reset_eu(tracker: &mut EuTracker, reset: i8) -> i32 {
    tracker.period.reset = reset;
    tracker.period.period = tracker.period.min_period;
    tracker.period.period_delta = 0;
    tracker.period.polarity = -1;
    tracker.period.period
}

/// Language selection for the shared engine. Japanese uses a separate pitch path.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Language {
    UsEnglish,
    UkEnglish,
    French,
    German,
    Italian,
    Spanish,
    Korean,
    Japanese,
}

/// fn_1c1218 / EU fn_1c12c8 / KR fn_1c144c state owned by one engine instance.
#[derive(Clone, Copy, Debug)]
pub enum PitchTracker {
    Crossings(PeriodTracker),
    Resonator(EuTracker),
}

impl PitchTracker {
    /// Dispatch the regional tracker without global language or scratch state.
    /// EU languages require Resonator; US English and Korean require Crossings.
    pub fn track(
        &mut self,
        language: Language,
        samples: &[i16],
        peaks: &[i16],
        scratch: &mut [f32],
        start: usize,
        end: usize,
        frequencies: &[f32; 5],
        tables: &NumericTables<'_>,
    ) -> i32 {
        match (self, language) {
            (Self::Crossings(state), Language::UsEnglish) => track_us(state, samples, start, end),
            (Self::Crossings(state), Language::Korean) => track_kr(state, samples, start, end),
            (
                Self::Resonator(state),
                Language::UkEnglish
                | Language::French
                | Language::German
                | Language::Italian
                | Language::Spanish,
            ) => track_eu(
                state,
                samples,
                peaks,
                scratch,
                start,
                end,
                frequencies,
                tables,
            ),
            _ => panic!("language and pitch state do not match"),
        }
    }
}
