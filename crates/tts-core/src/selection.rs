pub use crate::frontend_types::Error;

pub mod arena;
pub mod arena_leaf;
pub mod arena_mid;
pub mod frame;
pub mod stage;

/// fn_240ba0 / EU fn_240d5c / KR fn_240e08.
pub fn lowercase_voice_byte(value: u32) -> u32 {
    match value {
        0x9f => 0xff,
        0x8a | 0x8c => (value + 0x10) & 255,
        _ if value.wrapping_sub(0x41) <= 0x19
            || value.wrapping_sub(0xc0) <= 0x16
            || value.wrapping_sub(0xd8) <= 6
            || value == 0xdf =>
        {
            (value + 0x20) & 255
        }
        _ => value,
    }
}

/// fn_240b28 / EU fn_240ce4 / KR fn_240d90.
pub fn lowercase_dos_byte(value: u32) -> u32 {
    if value.wrapping_sub(0x41) <= 0x19 {
        return (value + 0x20) & 255;
    }
    match value {
        0x80 => 0x87,
        0x8e => 0x84,
        0x8f => 0x86,
        0x90 => 0x82,
        0x92 => 0x91,
        0x99 => 0x94,
        0x9a => 0x81,
        0xa5 => 0xa4,
        _ => value,
    }
}

/// fn_2f7bd4 / EU fn_2f7d90 / KR fn_2f7e3c.
pub fn mix_fixed(a: i32, mut b: i32, b_bits: u32, a_bits: u32, gain: i32, output_bits: u32) -> i32 {
    let product = if a_bits < output_bits.wrapping_add(16) {
        rounded_product(
            left(a, output_bits.wrapping_sub(a_bits).wrapping_add(16)),
            gain,
            16,
        )
    } else {
        rounded_product(a, gain, a_bits.wrapping_sub(output_bits) & 65535)
    };
    if b_bits > output_bits {
        b = right(b, b_bits.wrapping_sub(output_bits));
    } else {
        let shift = output_bits.wrapping_sub(b_bits) & 255;
        let limit = if shift >= 32 {
            0
        } else {
            0x7fff_ffffu32 >> shift
        };
        if limit as i32 <= b {
            return i32::MAX;
        }
        b = left(b, shift);
    }
    product.wrapping_add(b)
}

/// fn_2fbd28 / EU fn_2fbee4 / KR fn_2fbf90.
pub fn search_records<T>(
    entries: &[T],
    mut compare: impl FnMut(&T) -> core::cmp::Ordering,
    index: &mut usize,
) -> Option<usize> {
    use core::cmp::Ordering;
    let mut low = 0;
    let mut high = entries.len();
    while low < high {
        let middle = low + (high - low - 1) / 2;
        match compare(&entries[middle]) {
            Ordering::Less => high = middle,
            Ordering::Greater => low = middle + 1,
            Ordering::Equal => {
                *index = middle;
                return Some(middle);
            }
        }
    }
    *index = low;
    None
}

/// fn_2fb288 / EU fn_2fb444 / KR fn_2fb4f0.
pub fn lookup_byte_mapping(entries: &[[u8; 2]], key: u8, output: &mut u8) -> bool {
    let mut index = 0;
    if let Some(found) = search_records(entries, |entry| key.cmp(&entry[0]), &mut index) {
        *output = entries[found][1];
        true
    } else {
        false
    }
}

/// fn_2a3c2c / EU fn_2a3de8 / KR fn_2a3e94.
pub fn feature_format() -> u32 {
    2
}

/// fn_2a3c34 / EU fn_2a3df0 / KR fn_2a3e9c.
pub fn feature_input_format() -> u32 {
    2
}

/// fn_2a3c3c / EU fn_2a3df8 / KR fn_2a3ea4.
pub fn feature_output_format() -> u32 {
    2
}

/// fn_2a496c / EU fn_2a4b28 / KR fn_2a4bd4.
pub fn finish_selection() {}

/// fn_2a47ac / EU fn_2a4968 / KR fn_2a4a14.
pub fn retain_selection(references: &mut u16) {
    *references = references.wrapping_add(1);
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct FeatureCursor {
    pub input: u32,
    pub input_length: u32,
    pub current_input: u32,
    pub output: u32,
    pub current_output: u32,
}

impl FeatureCursor {
    /// fn_2a24e0 / EU fn_2a269c / KR fn_2a2748.
    pub fn rewind(&mut self, first_input: u32) {
        self.current_input = first_input;
        self.current_output = self.output;
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct FeatureSource {
    pub output: u32,
    pub loaned: u32,
    pub input: u32,
    pub input_length: u32,
}

/// fn_2a3458 / EU fn_2a3614 / KR fn_2a36c0.
pub fn loan_features(source: &mut FeatureSource, destination: &mut FeatureCursor) -> u32 {
    *destination = FeatureCursor {
        input: source.input,
        input_length: source.input_length,
        current_input: 0,
        output: source.output,
        current_output: 0,
    };
    source.loaned = 1;
    0
}

pub struct PrivateHeap<'a> {
    pub owner: u32,
    pub name: &'a [u8],
    pub first_block: Option<usize>,
    pub expected: u32,
    pub pool: u32,
    pub allocated: u32,
}

impl<'a> PrivateHeap<'a> {
    /// fn_2db19c / EU fn_2db358 / KR fn_2db404.
    pub fn initialize(owner: u32, name: &'a [u8]) -> Self {
        Self {
            owner,
            name,
            first_block: None,
            expected: 0xc800,
            pool: 0,
            allocated: 0,
        }
    }
}

/// fn_2a93dc / EU fn_2a9598 / KR fn_2a9644.
pub fn align_feature_cursor(position: &mut u32, limit: u32, alignment: u32) -> Result<(), Error> {
    if alignment == 0 {
        return Err(Error::InvalidResource);
    }
    let padding = (alignment - *position % alignment) % alignment;
    *position = position.wrapping_add(padding).min(limit);
    Ok(())
}

/// fn_2a9608 / EU fn_2a97c4 / KR fn_2a9870.
pub fn inverse_percent(value: i32, output: &mut i32) -> u32 {
    *output = 100;
    if value != 0 {
        let percent = value.wrapping_add(9999).wrapping_div(value);
        if (percent as u32).wrapping_sub(25) < 176 {
            *output = percent;
            return 0;
        }
    }
    0x84e0200f
}

/// fn_2f9818 / EU fn_2f99d4 / KR fn_2f9a80.
pub fn voice_model_interface<T: ?Sized>(table: &T) -> &T {
    table
}

/// fn_1e33d0 / EU fn_1e358c / KR fn_1e3638.
pub fn phoneme_map_interface<'a, T: ?Sized>(
    mode: u32,
    table: &'a T,
    output: &mut Option<&'a T>,
) -> u32 {
    if mode > 2 {
        return 0x8a702001;
    }
    *output = Some(table);
    0
}

/// fn_29b214 / EU fn_29b3d0 / KR fn_29b47c.
pub fn divide_fixed(numerator: i32, denominator: i32) -> Result<i32, Error> {
    if denominator == 0 {
        return Err(Error::InvalidResource);
    }
    let quotient = numerator.wrapping_div(denominator);
    let remainder = numerator.wrapping_sub(quotient.wrapping_mul(denominator));
    Ok(remainder
        .wrapping_shl(15)
        .wrapping_div(denominator)
        .wrapping_add(quotient.wrapping_shl(15)))
}

/// fn_244318 / EU fn_2444d4 / KR fn_244580.
pub fn reader_position(position: Option<&u32>) -> u32 {
    position.copied().unwrap_or(0)
}

/// fn_2fb7b0 / EU fn_2fb96c / KR fn_2fba18.
pub fn set_record_value(destination: &mut u32, value: u32) -> u32 {
    *destination = value;
    0
}

/// fn_2fb820 / EU fn_2fb9dc / KR fn_2fba88.
pub fn record_extent(length: u32, additional: u32) -> u32 {
    length.wrapping_add(additional).wrapping_add(4)
}

/// fn_2fbe50 / EU fn_2fc00c / KR fn_2fc0b8.
pub fn finish_record() -> u32 {
    0
}

/// fn_2fbcf0 / EU fn_2fbeac / KR fn_2fbf58.
pub fn combine_log_weights(a: i32, b: i32, table: &[i8], origin: usize) -> Result<i32, Error> {
    let mut difference = a.wrapping_sub(b) as i16;
    let mut base = b;
    if difference > 254 {
        difference = 254;
    } else if difference < 0 {
        difference = difference.wrapping_neg();
        base = a;
    }
    let index = origin
        .checked_add_signed(difference as isize)
        .ok_or(Error::InvalidResource)?;
    let correction = *table.get(index).ok_or(Error::Capacity {
        required: index.saturating_add(1),
        available: table.len(),
    })?;
    Ok(base.wrapping_add(correction as i32))
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct PredictionHistory {
    pub sample: i32,
    pub first_length: i32,
    pub second_length: i32,
}

impl PredictionHistory {
    /// fn_2dc784 / EU fn_2dc940 / KR fn_2dc9ec.
    pub fn reset(&mut self) {
        self.first_length = 0;
        self.second_length = 0;
        self.sample = 0;
    }
}

fn left(value: i32, shift: u32) -> i32 {
    let shift = shift & 255;
    if shift >= 32 {
        0
    } else {
        value.wrapping_shl(shift)
    }
}

fn right(value: i32, shift: u32) -> i32 {
    value >> (shift & 255).min(31)
}

/// fn_29b0dc / EU fn_29b298 / KR fn_29b344.
pub fn add_i16(a: i32, b: i32) -> i16 {
    a.wrapping_add(b).clamp(i16::MIN as i32, i16::MAX as i32) as i16
}

/// fn_29b0e8 / EU fn_29b2a4 / KR fn_29b350.
pub fn add_i32(a: i32, b: i32) -> i32 {
    a.saturating_add(b)
}

/// fn_29b118 / EU fn_29b2d4 / KR fn_29b380.
pub fn subtract_i16(a: i32, b: i32) -> i16 {
    a.wrapping_sub(b).clamp(i16::MIN as i32, i16::MAX as i32) as i16
}

/// fn_29b154 / EU fn_29b310 / KR fn_29b3bc.
pub fn doubled_product(a: i32, b: i32) -> i32 {
    let product = a.wrapping_mul(b);
    if product <= 0x40000000 {
        product.wrapping_mul(2)
    } else {
        i32::MAX
    }
}

/// fn_29b168 / EU fn_29b324 / KR fn_29b3d0.
pub fn rounded_product(a: i32, b: i32, fractional_bits: u32) -> i32 {
    let shift = fractional_bits.wrapping_sub(16) as u16 as u32;
    let low = (a as u16 as u32)
        .wrapping_mul(b as u16 as u32)
        .wrapping_add(0x8000);
    let middle = (a >> 16)
        .wrapping_mul(b as u16 as i32)
        .wrapping_add((a as u16 as i32).wrapping_mul(b >> 16))
        .wrapping_add((low >> 16) as i32);
    let middle = if shift == 0 {
        middle
    } else {
        right(middle.wrapping_add(left(1, shift.wrapping_sub(1))), shift)
    };
    middle.wrapping_add(left(
        (a >> 16).wrapping_mul(b >> 16),
        16u32.wrapping_sub(shift),
    ))
}

/// fn_29b1c4 / EU fn_29b380 / KR fn_29b42c.
pub fn rounded_short_product(a: i32, b: i16, fractional_bits: u32) -> i32 {
    let low = (a as u16 as i32).wrapping_mul(b as i32);
    let high = (a >> 16).wrapping_mul(b as i32);
    left(high, 16u32.wrapping_sub(fractional_bits)).wrapping_add(right(
        low.wrapping_add(left(1, fractional_bits.wrapping_sub(1))),
        fractional_bits,
    ))
}

/// fn_29b374 / EU fn_29b530 / KR fn_29b5dc.
pub fn signed_table(x: i32, table: &[i16]) -> Result<i16, Error> {
    let index = x >> 5;
    if index == 1023 {
        return Ok(0);
    }
    let index = usize::try_from(index).map_err(|_| Error::InvalidResource)?;
    let pair = table.get(index..index + 2).ok_or(Error::Capacity {
        required: index + 2,
        available: table.len(),
    })?;
    let fraction = x.wrapping_sub((index as i32).wrapping_shl(5));
    let delta = (pair[1] as i32 - pair[0] as i32)
        .wrapping_mul(fraction)
        .wrapping_mul(2)
        .wrapping_add(0x8000)
        >> 24;
    Ok((pair[0] as i32 + delta) as i16)
}

/// fn_29b3b4 / EU fn_29b570 / KR fn_29b61c.
pub fn unsigned_table(x: u32, table: &[u16]) -> Result<u16, Error> {
    let index = if x >> 5 > 1024 {
        1023
    } else {
        (x >> 5) as usize
    };
    let pair = table.get(index..index + 2).ok_or(Error::Capacity {
        required: index + 2,
        available: table.len(),
    })?;
    let fraction = x.wrapping_sub((index as u32) << 5) as i32;
    let delta = (pair[1] as i32 - pair[0] as i32)
        .wrapping_mul(fraction)
        .wrapping_add(16)
        >> 5;
    Ok((pair[0] as i32).wrapping_add(delta) as u16)
}

/// fn_29b3e8 / EU fn_29b5a4 / KR fn_29b650.
pub fn combine_fixed(high: i32, low: i32) -> i32 {
    high.wrapping_shl(16).wrapping_add(low.wrapping_shl(1))
}

/// fn_29b3f4 / EU fn_29b5b0 / KR fn_29b65c.
pub fn split_fixed(value: i32) -> (i16, i16) {
    let high = value >> 16;
    (
        high as i16,
        (value >> 1).wrapping_sub(high.wrapping_shl(15)) as i16,
    )
}

/// fn_29b40c / EU fn_29b5c8 / KR fn_29b674.
pub fn half_square_root(value: u32) -> u16 {
    let mut root = 0u16;
    let mut step = 0x8000u16;
    for _ in 0..16 {
        let candidate = root.wrapping_add(step);
        let square = (candidate as u32)
            .wrapping_mul(candidate as u32)
            .wrapping_mul(2);
        if value >= square {
            root = candidate;
        }
        if value == square {
            break;
        }
        step >>= 1;
    }
    root
}

/// fn_29b458 / EU fn_29b614 / KR fn_29b6c0.
pub fn rounded_square_root(value: u32) -> u32 {
    match value {
        0xdc0 => 59,
        0x800 => 45,
        0x400 => 32,
        0x580 => 38,
        0xa00 => 51,
        0xb00 => 53,
        0x1000 => 64,
        0x1400 => 72,
        0x1600 => 75,
        0x1b80 => 84,
        _ => square_root(value) as u32,
    }
}

/// fn_29b528 / EU fn_29b6e4 / KR fn_29b790.
pub fn square_root(value: u32) -> u16 {
    let mut root = 0u16;
    let mut step = 0x8000u16;
    for _ in 0..16 {
        let candidate = root.wrapping_add(step);
        let square = (candidate as u32) * candidate as u32;
        if value >= square {
            root = candidate;
        }
        if value == square {
            break;
        }
        step >>= 1;
    }
    root
}

/// fn_29b570 / EU fn_29b72c / KR fn_29b7d8.
pub fn prediction_square_root(value: u32) -> i32 {
    square_root(value) as i32
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AcousticControls {
    pub rate: i32,
    pub combined_rate: i32,
    pub pitch: i16,
    pub pitch_scale: i32,
    pub duration: i32,
    pub frequency: u32,
}

impl AcousticControls {
    /// fn_2a9d5c / EU fn_2a9f18 / KR fn_2a9fc4.
    pub fn set_rate(&mut self, value: u32) -> i32 {
        if value > 200 {
            return -1;
        }
        self.rate = if value == 100 {
            0x4000
        } else {
            value as i32 * 164
        };
        self.combined_rate = rounded_product(self.rate, self.duration.wrapping_shl(2), 16);
        0
    }

    /// fn_2a9da0 / EU fn_2a9f5c / KR fn_2aa008.
    pub fn set_duration(&mut self, value: u32) -> i32 {
        if value > 200 {
            return -1;
        }
        self.duration = if value == 100 {
            0x4000
        } else {
            value as i32 * 164
        };
        let rate = (self.rate as u32).wrapping_mul(100) >> 14;
        if rate <= 200 {
            self.rate = if rate == 100 {
                0x4000
            } else {
                rate as i32 * 164
            };
            self.combined_rate = rounded_product(self.rate, self.duration.wrapping_shl(2), 16);
        }
        0
    }

    /// fn_2a9e20 / EU fn_2a9fdc / KR fn_2aa088.
    pub fn set_frequency(&mut self, value: u32) -> i32 {
        if value > 200 {
            return -1;
        }
        self.frequency = if value == 100 { 0x4000 } else { value * 164 };
        0
    }

    /// fn_2a9e48 / EU fn_2aa004 / KR fn_2aa0b0.
    pub fn set_pitch(&mut self, value: i32) -> i32 {
        if value.wrapping_add(250) as u32 > 500 {
            return -1;
        }
        self.pitch = value.wrapping_shl(6) as i16;
        0
    }

    /// fn_2a9e64 / EU fn_2aa020 / KR fn_2aa0cc.
    pub fn set_pitch_scale(&mut self, value: u32) -> i32 {
        if value.wrapping_sub(11) > 189 {
            return -1;
        }
        self.pitch_scale = if value == 100 {
            0x4000
        } else {
            rounded_product(0xa3d70a, value as i32, 16)
        };
        0
    }

    /// fn_2a9ea0 / EU fn_2aa05c / KR fn_2aa108.
    pub fn rate(&self) -> i32 {
        (self.rate >> 14).wrapping_mul(100)
    }

    /// fn_2a9ebc / EU fn_2aa078 / KR fn_2aa124.
    pub fn duration(&self) -> i32 {
        (self.duration >> 14).wrapping_mul(100)
    }

    /// fn_2a9ed8 / EU fn_2aa094 / KR fn_2aa140.
    pub fn frequency(&self) -> u32 {
        (self.frequency >> 14).wrapping_mul(100)
    }

    /// fn_2a9ef4 / EU fn_2aa0b0 / KR fn_2aa15c.
    pub fn pitch(&self) -> i32 {
        (self.pitch >> 6) as i32
    }

    /// fn_2a9f08 / EU fn_2aa0c4 / KR fn_2aa170.
    pub fn pitch_scale(&self) -> i32 {
        (self.pitch_scale >> 14).wrapping_mul(100)
    }
}

/// fn_2db63c / EU fn_2db7f8 / KR fn_2db8a4.
pub fn fill_samples(output: &mut [i16], value: i16, count: i32) -> Result<(), Error> {
    let count = count.max(0) as usize;
    let available = output.len();
    output
        .get_mut(..count)
        .ok_or(Error::Capacity {
            required: count,
            available,
        })?
        .fill(value);
    Ok(())
}

/// fn_2db574 / EU fn_2db730 / KR fn_2db7dc.
pub fn convolve(
    signal: &[i16],
    history: usize,
    coefficients: &[i16],
    output: &mut [i16],
) -> Result<(), Error> {
    if coefficients.is_empty() {
        return Err(Error::InvalidResource);
    }
    if history < coefficients.len() - 1 {
        return Err(Error::InvalidResource);
    }
    let required = history
        .checked_add(output.len())
        .ok_or(Error::InvalidResource)?;
    if signal.len() < required {
        return Err(Error::Capacity {
            required,
            available: signal.len(),
        });
    }
    for (index, destination) in output.iter_mut().enumerate().rev() {
        let mut sum = 0i16;
        for (tap, &coefficient) in coefficients.iter().enumerate() {
            sum = sum.wrapping_add(
                ((coefficient as i32 * signal[history + index - tap] as i32) >> 16) as i16,
            );
        }
        *destination = sum;
    }
    Ok(())
}

/// fn_2dbb2c / EU fn_2dbce8 / KR fn_2dbd94.
pub fn prediction_coefficients(
    input: &[i32],
    output: &mut [i32],
    order: usize,
    factor: i32,
) -> Result<(), Error> {
    if order == 0 {
        return Err(Error::InvalidResource);
    }
    let required = order.checked_add(1).ok_or(Error::InvalidResource)?;
    if input.len() < required {
        return Err(Error::Capacity {
            required,
            available: input.len(),
        });
    }
    if output.len() < required {
        return Err(Error::Capacity {
            required,
            available: output.len(),
        });
    }
    output[order] = input[order];
    for index in (0..order).rev() {
        let product = output[index + 1].wrapping_neg() as i64 * factor as i64;
        output[index] = ((product >> 24) as i32).wrapping_add(input[index]);
    }
    output[0] = output[0].clamp(0, 0x1f9999a);
    output[1] = output[1].clamp(-0xf66666, 0xf66666);
    Ok(())
}

/// fn_2a3760 / EU fn_2a391c / KR fn_2a39c8.
pub fn scale_features(input: &[i32], output: &mut [i32], count: i32) -> Result<(), Error> {
    let required = count.max(0) as usize;
    if input.len() < required {
        return Err(Error::Capacity {
            required,
            available: input.len(),
        });
    }
    if output.len() < required {
        return Err(Error::Capacity {
            required,
            available: output.len(),
        });
    }
    for (destination, &source) in output[..required].iter_mut().zip(input) {
        *destination = source.wrapping_shl(6);
    }
    Ok(())
}

/// fn_2a37f4 / EU fn_2a39b0 / KR fn_2a3a5c.
pub fn feature_delta(
    enabled: bool,
    input: &[i32],
    output: &mut [i32],
    coefficients: &[i32; 5],
) -> Result<(), Error> {
    if !enabled || input.is_empty() {
        return Ok(());
    }
    if output.len() < input.len() {
        return Err(Error::Capacity {
            required: input.len(),
            available: output.len(),
        });
    }
    for (index, destination) in output[..input.len()].iter_mut().enumerate() {
        let mut sum = 0i32;
        for (tap, &coefficient) in coefficients.iter().enumerate() {
            let position =
                (index as isize + tap as isize - 2).clamp(0, input.len() as isize - 1) as usize;
            sum = sum.wrapping_add(input[position].wrapping_mul(coefficient));
        }
        *destination = sum;
    }
    Ok(())
}

/// fn_2a38d8 / EU fn_2a3a94 / KR fn_2a3b40.
pub fn smooth_features(
    mode: i32,
    samples: &mut [i32],
    middle: &[i32],
    count: usize,
    output: &mut [i32],
) -> Result<(), Error> {
    if mode != 0 || count == 0 {
        return Ok(());
    }
    let required = count.checked_add(4).ok_or(Error::InvalidResource)?;
    if samples.len() < required {
        return Err(Error::Capacity {
            required,
            available: samples.len(),
        });
    }
    if count > 1 && middle.len() < count {
        return Err(Error::Capacity {
            required: count,
            available: middle.len(),
        });
    }
    if output.len() < count {
        return Err(Error::Capacity {
            required: count,
            available: output.len(),
        });
    }
    if count > 1 {
        let start = if middle[1] < 0 { 6 } else { 1 };
        for index in start..count - 1 {
            samples[index + 3] = samples[index + 1]
                .wrapping_add(samples[index + 3])
                .wrapping_add(middle[index].wrapping_mul(2))
                >> 1;
        }
    }
    samples[0] = samples[2];
    samples[1] = samples[2];
    samples[count + 2] = samples[count + 1];
    samples[count + 3] = samples[count + 1];
    {
        for (index, destination) in output[..count].iter_mut().enumerate() {
            let coefficients = [2i32, 16, 28, 16, 2];
            *destination = samples[index..index + 5]
                .iter()
                .zip(coefficients)
                .fold(0i32, |sum, (&value, coefficient)| {
                    sum.wrapping_add(value.wrapping_mul(coefficient))
                });
        }
    }
    Ok(())
}

/// fn_2db574 / EU fn_2db730 / KR fn_2db7dc, with overlapping source and destination.
pub fn convolve_in_place(
    storage: &mut [i16],
    source: usize,
    destination: usize,
    count: usize,
    coefficients: &[i16],
) -> Result<(), Error> {
    if coefficients.is_empty() || source < coefficients.len() - 1 {
        return Err(Error::InvalidResource);
    }
    let required = source
        .max(destination)
        .checked_add(count)
        .ok_or(Error::InvalidResource)?;
    if storage.len() < required {
        return Err(Error::Capacity {
            required,
            available: storage.len(),
        });
    }
    for index in (0..count).rev() {
        let mut sum = 0i16;
        for (tap, &coefficient) in coefficients.iter().enumerate() {
            sum = sum.wrapping_add(
                ((coefficient as i32 * storage[source + index - tap] as i32) >> 16) as i16,
            );
        }
        storage[destination + index] = sum;
    }
    Ok(())
}

#[derive(Clone, Copy)]
pub enum DirectoryUnits<'a> {
    Bytes(&'a [u8]),
    Wide(&'a [u16]),
}

impl DirectoryUnits<'_> {
    fn get(self, index: usize) -> Result<u16, Error> {
        let (value, available) = match self {
            Self::Bytes(bytes) => (bytes.get(index).map(|&value| value as u16), bytes.len()),
            Self::Wide(words) => (words.get(index).copied(), words.len()),
        };
        value.ok_or(Error::Capacity {
            required: index.saturating_add(1),
            available,
        })
    }

    fn wide(self) -> bool {
        matches!(self, Self::Wide(_))
    }
}

pub struct VoiceDirectory<'a> {
    pub entries: DirectoryUnits<'a>,
    pub count: i32,
    pub key_width: u16,
    pub value_width: u16,
}

pub struct DirectoryHandle<'a> {
    pub mode: u8,
    pub first_width: u16,
    pub second_width: u16,
    pub count: u16,
    pub extended_count: u16,
    pub primary: Option<DirectoryUnits<'a>>,
    pub secondary: Option<DirectoryUnits<'a>>,
}

impl DirectoryHandle<'_> {
    fn directory(&self, extended: bool, secondary: bool) -> Option<VoiceDirectory<'_>> {
        Some(VoiceDirectory {
            entries: if secondary {
                self.secondary?
            } else {
                self.primary?
            },
            count: if extended {
                self.extended_count
            } else {
                self.count
            } as i32,
            key_width: if extended || secondary {
                self.second_width
            } else {
                self.first_width
            },
            value_width: if extended || secondary {
                self.first_width
            } else {
                self.second_width
            },
        })
    }
}

fn lookup_handle(
    handle: Option<&DirectoryHandle<'_>>,
    key: Option<DirectoryUnits<'_>>,
    output: Option<&mut Option<usize>>,
    extended: bool,
) -> Result<(), Error> {
    let (handle, key, output) = match (handle, key, output) {
        (Some(handle), Some(key), Some(output)) => (handle, key, output),
        _ => return Err(Error::Engine(0x8a202007)),
    };
    *output = None;
    let directory = handle
        .directory(extended, false)
        .filter(|_| handle.mode == 1)
        .ok_or(Error::Engine(0x8a202001))?;
    if !directory.entries.wide() {
        return Err(Error::InvalidResource);
    }
    directory.lookup(key, output)
}

/// fn_241a5c / EU fn_241c18 / KR fn_241cc4.
pub fn lookup_primary_wide(
    handle: Option<&DirectoryHandle<'_>>,
    key: Option<DirectoryUnits<'_>>,
    output: Option<&mut Option<usize>>,
) -> Result<(), Error> {
    lookup_handle(handle, key, output, false)
}

/// fn_241acc / EU fn_241c88 / KR fn_241d34.
pub fn lookup_extended_wide(
    handle: Option<&DirectoryHandle<'_>>,
    key: Option<DirectoryUnits<'_>>,
    output: Option<&mut Option<usize>>,
) -> Result<(), Error> {
    lookup_handle(handle, key, output, true)
}

fn prefix_handle(
    handle: Option<&DirectoryHandle<'_>>,
    key: Option<DirectoryUnits<'_>>,
    prefix: Option<&mut u16>,
    value: Option<&mut Option<usize>>,
    wide: bool,
    secondary: bool,
) -> Result<(), Error> {
    let (handle, key, prefix, value) = match (handle, key, prefix, value) {
        (Some(handle), Some(key), Some(prefix), Some(value)) => (handle, key, prefix, value),
        _ => return Err(Error::Engine(0x8a202007)),
    };
    *prefix = 0;
    *value = None;
    if handle.primary.is_none() || handle.mode != wide as u8 {
        return Err(Error::Engine(0x8a202001));
    }
    let directory = handle
        .directory(false, secondary)
        .ok_or(Error::InvalidResource)?;
    if directory.entries.wide() != wide {
        return Err(Error::InvalidResource);
    }
    let mut result = DirectoryMatch::default();
    let status = directory.prefix_lookup(key, &mut result);
    *prefix = result.prefix;
    *value = result.value;
    status
}

/// fn_241b3c / EU fn_241cf8 / KR fn_241da4.
pub fn prefix_primary_byte(
    handle: Option<&DirectoryHandle<'_>>,
    key: Option<DirectoryUnits<'_>>,
    prefix: Option<&mut u16>,
    value: Option<&mut Option<usize>>,
) -> Result<(), Error> {
    prefix_handle(handle, key, prefix, value, false, false)
}

/// fn_241bc4 / EU fn_241d80 / KR fn_241e2c.
pub fn prefix_primary_wide(
    handle: Option<&DirectoryHandle<'_>>,
    key: Option<DirectoryUnits<'_>>,
    prefix: Option<&mut u16>,
    value: Option<&mut Option<usize>>,
) -> Result<(), Error> {
    prefix_handle(handle, key, prefix, value, true, false)
}

/// fn_241c4c / EU fn_241e08 / KR fn_241eb4.
pub fn prefix_secondary_byte(
    handle: Option<&DirectoryHandle<'_>>,
    key: Option<DirectoryUnits<'_>>,
    prefix: Option<&mut u16>,
    value: Option<&mut Option<usize>>,
) -> Result<(), Error> {
    prefix_handle(handle, key, prefix, value, false, true)
}

/// fn_241ccc / EU fn_241e88 / KR fn_241f34.
pub fn prefix_secondary_wide(
    handle: Option<&DirectoryHandle<'_>>,
    key: Option<DirectoryUnits<'_>>,
    prefix: Option<&mut u16>,
    value: Option<&mut Option<usize>>,
) -> Result<(), Error> {
    prefix_handle(handle, key, prefix, value, true, true)
}

/// fn_241d4c / EU fn_241f08 / KR fn_241fb4.
pub fn describe_primary_bytes<'a>(
    handle: Option<&DirectoryHandle<'a>>,
    length: Option<&mut u32>,
    data: Option<&mut Option<DirectoryUnits<'a>>>,
) -> Result<(), Error> {
    let (handle, length, data) = match (handle, length, data) {
        (Some(handle), Some(length), Some(data)) => (handle, length, data),
        _ => return Err(Error::Engine(0x8a202007)),
    };
    *length = 0;
    *data = None;
    if handle.primary.is_none() || handle.mode != 0 {
        return Err(Error::Engine(0x8a202001));
    }
    *length =
        (handle.first_width as u32 + handle.second_width as u32).wrapping_mul(handle.count as u32);
    *data = handle.primary;
    Ok(())
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct DirectoryMatch {
    pub prefix: u16,
    /// Index into the byte or UTF-16 entries slice.
    pub value: Option<usize>,
}

impl VoiceDirectory<'_> {
    fn entry(&self, index: i32) -> usize {
        self.key_width.wrapping_add(self.value_width) as usize * index as usize
    }

    fn compare(
        &self,
        start: usize,
        key: DirectoryUnits<'_>,
        key_start: usize,
    ) -> Result<(i32, u16), Error> {
        let mut length = 0usize;
        loop {
            let left = self.entries.get(start + length)?;
            let right = key.get(key_start + length)?;
            let difference = left as i32 - right as i32;
            if difference != 0 || left == 0 {
                return Ok((difference, if left == 0 { length as u16 } else { 0 }));
            }
            length += 1;
        }
    }

    /// fn_241dbc / EU fn_241f78 / KR fn_242024.
    pub fn lookup(&self, key: DirectoryUnits<'_>, output: &mut Option<usize>) -> Result<(), Error> {
        if self.entries.wide() != key.wide() || self.count == i32::MIN {
            return Err(Error::InvalidResource);
        }
        let mut low = 0i32;
        let mut high = self.count - 1;
        while low <= high {
            let middle = low.wrapping_add(high) / 2;
            if middle < 0 {
                return Err(Error::InvalidResource);
            }
            let (difference, _) = self.compare(self.entry(middle), key, 0)?;
            match difference.cmp(&0) {
                core::cmp::Ordering::Less => low = middle + 1,
                core::cmp::Ordering::Greater => high = middle - 1,
                core::cmp::Ordering::Equal => {
                    *output = Some(self.entry(middle) + self.key_width as usize);
                    break;
                }
            }
        }
        output.map_or(Err(Error::Engine(0x8a202014)), |_| Ok(()))
    }

    /// fn_240dcc / EU fn_240f88 / KR fn_241034.
    pub fn prefix_lookup(
        &self,
        key: DirectoryUnits<'_>,
        output: &mut DirectoryMatch,
    ) -> Result<(), Error> {
        if self.entries.wide() != key.wide() || self.count == i32::MIN {
            return Err(Error::InvalidResource);
        }
        if self.count > 0 {
            (self.key_width.wrapping_add(self.value_width) as usize)
                .checked_mul(self.count as usize)
                .ok_or(Error::InvalidResource)?;
        }
        let mut low = 0i32;
        let mut high = self.count - 1;
        let mut found = None;
        while low <= high {
            let middle = (low + high) / 2;
            let (difference, prefix) = self.compare(self.entry(middle), key, 0)?;
            if prefix != 0 {
                found = Some((middle, prefix));
                break;
            }
            if difference > 0 {
                high = middle - 1;
            } else if difference < 0 {
                low = middle + 1;
            } else {
                return Err(Error::InvalidResource);
            }
        }
        if found.is_none() {
            let mut middle = low;
            while middle > 0 {
                if key.get(0)? != self.entries.get(self.entry(middle - 1))? {
                    break;
                }
                middle -= 1;
                let (_, prefix) = self.compare(self.entry(middle), key, 0)?;
                if prefix != 0 {
                    found = Some((middle, prefix));
                    break;
                }
            }
        }
        let (mut best, mut prefix) = found.ok_or(Error::Engine(0x8a202014))?;
        let saved = best;
        let mut middle = best;
        while middle > 0 {
            let (_, common) =
                self.compare(self.entry(best), self.entries, self.entry(middle - 1))?;
            if common == 0 {
                break;
            }
            middle -= 1;
            let (_, candidate) = self.compare(self.entry(middle), key, 0)?;
            if candidate > prefix {
                best = middle;
                prefix = candidate;
            }
        }
        middle = saved;
        while middle < self.count - 1 {
            let (_, common) =
                self.compare(self.entry(best), self.entries, self.entry(middle + 1))?;
            if common == 0 {
                break;
            }
            middle += 1;
            let (_, candidate) = self.compare(self.entry(middle), key, 0)?;
            if candidate > prefix {
                best = middle;
                prefix = candidate;
            }
        }
        *output = DirectoryMatch {
            prefix,
            value: Some(
                self.entry(best)
                    .checked_add(self.key_width as usize)
                    .ok_or(Error::InvalidResource)?,
            ),
        };
        Ok(())
    }
}

/// fn_2f7b2c / EU fn_2f7ce8 / KR fn_2f7d94.
pub fn mix_nonnegative_fixed(
    a: i32,
    mut b: u32,
    b_bits: u32,
    a_bits: u32,
    gain: i32,
    mut output_bits: u32,
) -> i32 {
    let mut restore = 0;
    if output_bits.wrapping_add(2) > a_bits {
        restore = output_bits.wrapping_sub(a_bits).wrapping_add(2) as u16 as u32;
        output_bits = output_bits.wrapping_sub(restore) as u16 as u32;
    }
    let value = if a_bits < output_bits.wrapping_add(16) {
        rounded_product(
            left(a, output_bits.wrapping_sub(a_bits).wrapping_add(16)),
            gain,
            16,
        )
    } else {
        rounded_product(a, gain, a_bits.wrapping_sub(output_bits) as u16 as u32)
    };
    if b_bits > output_bits {
        b = logical_right(b, b_bits.wrapping_sub(output_bits));
    } else {
        let shift = output_bits.wrapping_sub(b_bits);
        if b >= logical_right(i32::MAX as u32, shift) {
            return i32::MAX;
        }
        b = left(b as i32, shift) as u32;
    }
    let sum = (value as u32).wrapping_add(b);
    if sum >= logical_right(i32::MAX as u32, restore) {
        i32::MAX
    } else {
        left(sum as i32, restore)
    }
}

fn logical_right(value: u32, shift: u32) -> u32 {
    let shift = shift & 255;
    if shift >= 32 {
        0
    } else {
        value >> shift
    }
}

fn smooth_feature_window(
    mode: i32,
    samples: &mut [i32],
    middle: &[i32],
    count: usize,
    output: &mut [i32],
    taps: [i32; 5],
) -> Result<(), Error> {
    if mode != 0 || count == 0 {
        return Ok(());
    }
    let required = count.checked_add(4).ok_or(Error::InvalidResource)?;
    if samples.len() < required {
        return Err(Error::Capacity {
            required,
            available: samples.len(),
        });
    }
    if count > 1 && middle.len() < count {
        return Err(Error::Capacity {
            required: count,
            available: middle.len(),
        });
    }
    if output.len() < count {
        return Err(Error::Capacity {
            required: count,
            available: output.len(),
        });
    }
    if count > 1 {
        let start = if middle[1] < 0 { 6 } else { 1 };
        for i in start..count - 1 {
            samples[i + 3] = samples[i + 1]
                .wrapping_add(samples[i + 3])
                .wrapping_add(middle[i].wrapping_mul(2))
                >> 1;
        }
    }
    samples[0] = samples[2];
    samples[1] = samples[2];
    samples[count + 2] = samples[count + 1];
    samples[count + 3] = samples[count + 1];
    for (i, out) in output[..count].iter_mut().enumerate() {
        *out = samples[i..i + 5]
            .iter()
            .zip(taps)
            .fold(0i32, |sum, (&x, k)| sum.wrapping_add(x.wrapping_mul(k)));
    }
    Ok(())
}

/// fn_2a39ec / EU fn_2a3ba8 / KR fn_2a3c54.
pub fn smooth_features_wide(
    mode: i32,
    samples: &mut [i32],
    middle: &[i32],
    count: usize,
    output: &mut [i32],
) -> Result<(), Error> {
    smooth_feature_window(mode, samples, middle, count, output, [9, 62, 114, 62, 9])
}

/// fn_2a3b18 / EU fn_2a3cd4 / KR fn_2a3d80.
pub fn smooth_features_narrow(
    mode: i32,
    samples: &mut [i32],
    middle: &[i32],
    count: usize,
    output: &mut [i32],
) -> Result<(), Error> {
    smooth_feature_window(mode, samples, middle, count, output, [2, 16, 28, 16, 2])
}

pub struct BandSystem<'a> {
    pub coefficients: &'a [i32],
    pub stride: usize,
    pub width: usize,
    pub input: &'a [i32],
}

impl BandSystem<'_> {
    fn validate(&self, output: &[i32], width: usize) -> Result<(), Error> {
        if self.input.is_empty() || self.stride < width {
            return Err(Error::InvalidResource);
        }
        let required = self
            .input
            .len()
            .checked_mul(self.stride)
            .ok_or(Error::InvalidResource)?;
        if self.coefficients.len() < required {
            return Err(Error::Capacity {
                required,
                available: self.coefficients.len(),
            });
        }
        if output.len() < self.input.len() {
            return Err(Error::Capacity {
                required: self.input.len(),
                available: output.len(),
            });
        }
        Ok(())
    }
    fn divide(a: i32, b: i32) -> Result<i32, Error> {
        crate::runtime::signed_division(a, b)
            .map(|v| v.quotient)
            .map_err(|_| Error::InvalidResource)
    }
    /// fn_2a3c44 / EU fn_2a3e00 / KR fn_2a3eac.
    pub fn forward(&self, a: u32, b: u32, c: u32, output: &mut [i32]) -> Result<(), Error> {
        self.validate(output, self.width.max(1))?;
        output[0] = left(
            Self::divide(
                self.input[0],
                right(self.coefficients[0], b.wrapping_sub(a >> 1)),
            )?,
            c.wrapping_sub(a >> 1),
        );
        for i in 1..self.input.len() {
            let mut sum = 0i32;
            for j in 1..self.width.min(i + 1) {
                let coefficient = self.coefficients[(i - j) * self.stride + j];
                if coefficient != 0 {
                    sum = sum.wrapping_add(
                        right(coefficient, c >> 1).wrapping_mul(right(output[i - j], c >> 1)),
                    );
                }
            }
            let d = a.wrapping_sub(b) as i32;
            let x = if d >= 0 {
                right(self.input[i], d as u32)
            } else {
                left(self.input[i], b.wrapping_sub(a))
            };
            output[i] = left(
                Self::divide(
                    x.wrapping_sub(sum),
                    right(self.coefficients[i * self.stride], c >> 1),
                )?,
                c >> 1,
            );
        }
        Ok(())
    }
    /// fn_2a3d84 / EU fn_2a3f40 / KR fn_2a3fec.
    pub fn forward_three(&self, output: &mut [i32]) -> Result<(), Error> {
        self.validate(output, 3)?;
        let x = self.input[0];
        let d = self.coefficients[0] >> 8;
        output[0] = if x > 0x1fffffff || x.wrapping_neg() > 0x1fffffff {
            left(Self::divide(x, d)?, 12)
        } else {
            left(Self::divide(left(x, 2), d)?, 10)
        };
        for i in 1..self.input.len() {
            let mut sum = (self.coefficients[(i - 1) * self.stride + 1] >> 9)
                .wrapping_mul(output[i - 1] >> 11);
            if i > 1 {
                sum = sum.wrapping_add(
                    (self.coefficients[(i - 2) * self.stride + 2] >> 9)
                        .wrapping_mul(output[i - 2] >> 11),
                );
            }
            output[i] = left(
                Self::divide(
                    self.input[i].wrapping_sub(sum),
                    self.coefficients[i * self.stride] >> 11,
                )?,
                9,
            );
        }
        Ok(())
    }
    /// fn_2a3ec4 / EU fn_2a4080 / KR fn_2a412c.
    pub fn backward(&self, a: u32, b: u32, c: u32, output: &mut [i32]) -> Result<(), Error> {
        self.validate(output, self.width.max(1))?;
        let n = self.input.len();
        let half = b >> 1;
        output[n - 1] = left(
            Self::divide(
                self.input[n - 1],
                right(
                    self.coefficients[(n - 1) * self.stride],
                    c.wrapping_sub(half),
                ),
            )?,
            a.wrapping_sub(half),
        );
        let shift = (c.wrapping_add(a).wrapping_sub(b) as i32 >> 1) as u32;
        for i in (0..n - 1).rev() {
            let mut sum = 0i32;
            for j in 1..self.width.min(n - i) {
                let coefficient = self.coefficients[i * self.stride + j];
                if coefficient != 0 {
                    sum = sum.wrapping_add(
                        right(coefficient, shift).wrapping_mul(right(output[i + j], shift)),
                    );
                }
            }
            output[i] = left(
                Self::divide(
                    self.input[i].wrapping_sub(sum),
                    right(self.coefficients[i * self.stride], c.wrapping_sub(half)),
                )?,
                a.wrapping_sub(half),
            );
        }
        Ok(())
    }
    /// fn_2a3fe4 / EU fn_2a41a0 / KR fn_2a424c.
    pub fn backward_three(&self, output: &mut [i32]) -> Result<(), Error> {
        self.validate(output, 3)?;
        let n = self.input.len();
        for i in (0..n).rev() {
            let mut sum = 0i32;
            if i + 1 < n {
                sum =
                    (self.coefficients[i * self.stride + 1] >> 9).wrapping_mul(output[i + 1] >> 9);
            }
            if i + 2 < n {
                sum = sum.wrapping_add(
                    (self.coefficients[i * self.stride + 2] >> 9).wrapping_mul(output[i + 2] >> 9),
                );
            }
            let x = self.input[i].wrapping_sub(sum);
            let d = self.coefficients[i * self.stride] >> 6;
            output[i] = left(
                if d == 0 && i + 2 >= n {
                    x
                } else {
                    Self::divide(x, d)?
                },
                12,
            );
        }
        Ok(())
    }
}

/// fn_2a4510 / EU fn_2a46cc / KR fn_2a4778.
pub fn validate_window_counts(
    f0_count: i32,
    expected_f0: i32,
    mcp_count: i32,
    expected_mcp: i32,
) -> Result<u32, Error> {
    if f0_count != expected_f0 {
        return Ok(0x84e02004);
    }
    let division = crate::runtime::signed_division(expected_mcp, mcp_count)
        .map_err(|_| Error::InvalidResource)?;
    Ok(if division.remainder == 0 {
        0
    } else {
        0x84e02004
    })
}

/// fn_2db66c / EU fn_2db828 / KR fn_2db8d4.
pub fn window_table_index(
    kind: i32,
    index: u32,
    width: i32,
    output: &mut Option<(usize, usize)>,
) -> i32 {
    if kind == 0 {
        if index.wrapping_sub(3) > 2 {
            return -1;
        }
        *output = Some((0, index as usize));
    } else if kind == 1 {
        if index.wrapping_sub(2) > 5 {
            return 0;
        }
        let table = match width {
            2 => 1,
            3 => 2,
            4 => 3,
            5 => 4,
            6 => 5,
            8 => 6,
            _ => return -1,
        };
        *output = Some((table, index as usize));
    } else {
        return -1;
    }
    0
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct WindowSettings {
    pub value: u32,
    pub parameters: [u8; 6],
}

/// fn_2db744 / EU fn_2db900 / KR fn_2db9ac.
pub fn parse_window_settings(
    index: usize,
    value: u32,
    text: &[u8],
    output: &mut WindowSettings,
) -> Result<i32, Error> {
    let end = text
        .iter()
        .position(|&x| x == 0)
        .ok_or(Error::InvalidResource)?;
    if end > 103 || index >= 5 {
        return Err(Error::InvalidResource);
    }
    output.value = value;
    if end != 80 {
        return Ok(-1);
    }
    for row in text[..80].chunks_exact(16) {
        if row[0] == b"XLMHU"[index] {
            for (out, offset) in output.parameters.iter_mut().zip([3, 9, 7, 13, 5, 11]) {
                let mut n = 0u32;
                let mut at = offset;
                while at < row.len() && !at.is_multiple_of(2) && row[at].is_ascii_whitespace() {
                    at += 1;
                }
                let negative = row.get(at) == Some(&b'-');
                if negative || row.get(at) == Some(&b'+') {
                    at += 1;
                }
                while at < row.len() && !at.is_multiple_of(2) && row[at].is_ascii_digit() {
                    n = n.wrapping_mul(10).wrapping_add((row[at] - b'0') as u32);
                    at += 1;
                }
                *out = if negative { n.wrapping_neg() } else { n } as u8;
            }
            break;
        }
    }
    Ok(0)
}

/// fn_2db87c / EU fn_2dba38 / KR fn_2dbae4.
pub fn window_settings(
    available: bool,
    index: i32,
    rate: i32,
    value: u32,
    text: Option<&[u8]>,
    output: &mut WindowSettings,
) -> Result<i32, Error> {
    if !available {
        return Ok(-1);
    }
    if let Some(text) = text.filter(|s| s.first().is_some_and(|&b| b != 0)) {
        return parse_window_settings(index as usize, value, text, output);
    }
    let last = if rate == 8000 { 4 } else { 5 };
    output.value = value;
    output.parameters = match index {
        0 => [3, 2, 1, 1, 4, last],
        2 => [3, 4, 0, 0, 8, 8],
        3 => [3, 5, 0, 0, 8, 8],
        4 => [3, 7, 1, 1, 4, 5],
        _ => [3, 3, 1, 1, 4, last],
    };
    Ok(0)
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct PredictionSmoother {
    pub limit: u32,
    pub scale: u32,
    pub minimum_count: u8,
    pub maximum_count: u8,
    pub result: u16,
}
impl PredictionSmoother {
    /// fn_2dca78 / EU fn_2dcc34 / KR fn_2dcce0.
    pub fn update(
        &mut self,
        mut value: i32,
        history: &mut [i16],
        length: usize,
        clear_count: usize,
        weights: &[i32; 8],
    ) -> Result<(), Error> {
        if self.minimum_count >= self.maximum_count {
            if value != 0 {
                if length < 4 || length > history.len() {
                    return Err(Error::InvalidResource);
                }
                history[length - 1] = value as i16;
                history.copy_within(1..length, 0);
                let mut sum = 0i32;
                let mut index = 0;
                for (offset, weight) in [(2, 4), (3, 2), (4, 1)] {
                    let x = history[length - offset] as i32;
                    if x > 0 {
                        sum = sum.wrapping_add(x.wrapping_mul(weight));
                        index += weight as usize;
                    }
                }
                value = ((weights[index].wrapping_mul(sum) as u32) >> 16) as i32;
            } else {
                if clear_count > history.len() {
                    return Err(Error::Capacity {
                        required: clear_count,
                        available: history.len(),
                    });
                }
                history[..clear_count].fill(0);
            }
        }
        self.result = self.limit.min(self.scale.wrapping_mul(value as u32) >> 14) as u16;
        Ok(())
    }
}

#[derive(Clone, Copy)]
pub struct AcousticRecord<'a> {
    pub status: u32,
    pub bytes: &'a [u8],
}

pub struct AcousticTable<'a> {
    pub width: u32,
    pub record_size: u16,
    pub shifts: [[u16; 3]; 3],
    pub scales: [[u16; 3]; 3],
    pub offsets: [[i32; 3]; 3],
    pub offset_shift: u16,
    pub data: Option<&'a [u8]>,
    pub data_origin: u32,
    pub blocks: &'a [u32],
}
impl<'a> AcousticTable<'a> {
    fn row(
        &self,
        block: u32,
        row: u32,
        load: &mut impl FnMut(u32, u32) -> AcousticRecord<'a>,
    ) -> Result<AcousticRecord<'a>, Error> {
        let base = *self
            .blocks
            .get(block as usize)
            .ok_or(Error::InvalidResource)?;
        let offset = base.wrapping_add(row.wrapping_mul(self.record_size as u32));
        if let Some(data) = self.data {
            Ok(AcousticRecord {
                status: 0,
                bytes: data
                    .get(offset.wrapping_sub(self.data_origin) as usize..)
                    .ok_or(Error::InvalidResource)?,
            })
        } else {
            Ok(load(offset, self.record_size as u32))
        }
    }
    fn samples(&self, data: &[u8], column: usize) -> Result<[u32; 3], Error> {
        let base = column
            .checked_mul(self.width as usize)
            .and_then(|n| n.checked_mul(3))
            .ok_or(Error::InvalidResource)?;
        let mut samples = [0; 3];
        for (i, sample) in samples.iter_mut().enumerate() {
            let offset = base + i * if self.width == 1 { 1 } else { 2 };
            *sample = if self.width == 1 {
                *data.get(offset).ok_or(Error::InvalidResource)? as u32
            } else {
                u16::from_le_bytes(
                    data.get(offset..offset + 2)
                        .ok_or(Error::InvalidResource)?
                        .try_into()
                        .unwrap(),
                ) as u32
            };
        }
        Ok(samples)
    }
    /// fn_2f850c / EU fn_2f86c8 / KR fn_2f8774.
    pub fn read_three(
        &self,
        block: u32,
        row: u32,
        column: usize,
        precision: u32,
        output: &mut [i32; 3],
        mut load: impl FnMut(u32, u32) -> AcousticRecord<'a>,
    ) -> Result<u32, Error> {
        if column >= 3 {
            return Err(Error::InvalidResource);
        }
        let record = self.row(block, row, &mut load)?;
        if record.status >= 0x80000000 {
            return Ok(record.status);
        }
        let samples = self.samples(record.bytes, column)?;
        for i in 0..2 {
            output[i] = mix_fixed(
                samples[i] as i32,
                self.offsets[column][i],
                self.offset_shift as u32,
                self.shifts[column][i] as u32,
                self.scales[column][i] as i32,
                precision,
            );
        }
        output[2] = mix_nonnegative_fixed(
            samples[2] as i32,
            self.offsets[column][2] as u32,
            self.offset_shift as u32,
            self.shifts[column][2] as u32,
            self.scales[column][2] as i32,
            precision,
        );
        Ok(record.status)
    }
    /// fn_2f86e0 / EU fn_2f889c / KR fn_2f8948.
    pub fn read_outer(
        &self,
        block: u32,
        row: u32,
        column: usize,
        precision: u32,
        output: &mut [i32; 2],
        mut load: impl FnMut(u32, u32) -> AcousticRecord<'a>,
    ) -> Result<u32, Error> {
        if column >= 3 {
            return Err(Error::InvalidResource);
        }
        let record = self.row(block, row, &mut load)?;
        if record.status >= 0x80000000 {
            return Ok(record.status);
        }
        let samples = self.samples(record.bytes, column)?;
        output[0] = mix_fixed(
            samples[0] as i32,
            self.offsets[column][0],
            self.offset_shift as u32,
            self.shifts[column][0] as u32,
            self.scales[column][0] as i32,
            precision,
        );
        output[1] = mix_nonnegative_fixed(
            samples[2] as i32,
            self.offsets[column][2] as u32,
            self.offset_shift as u32,
            self.shifts[column][2] as u32,
            self.scales[column][2] as i32,
            precision,
        );
        Ok(record.status)
    }
}

pub struct DurationTable<'a> {
    pub width: u32,
    pub count: u32,
    pub offsets: [u32; 14],
    pub offset_shift: u16,
    pub shifts: [u16; 14],
    pub scales: [u16; 14],
    pub data: Option<&'a [u8]>,
    pub data_origin: u32,
    pub offset: u32,
}
impl<'a> DurationTable<'a> {
    /// fn_2f7f90 / EU fn_2f814c / KR fn_2f81f8.
    pub fn read_pair(
        &self,
        row: u32,
        column: u32,
        precision: u32,
        output: &mut [i32; 2],
        mut load: impl FnMut(u32, u32) -> AcousticRecord<'a>,
    ) -> Result<u32, Error> {
        let size = self.count.wrapping_mul(self.width).wrapping_mul(2);
        let offset = self.offset.wrapping_add(size.wrapping_mul(row));
        let record = if let Some(data) = self.data {
            AcousticRecord {
                status: 0,
                bytes: data
                    .get(offset.wrapping_sub(self.data_origin) as usize..)
                    .ok_or(Error::InvalidResource)?,
            }
        } else {
            load(offset, size)
        };
        if record.status >= 0x80000000 {
            return Ok(record.status);
        }
        for (i, out) in output.iter_mut().enumerate() {
            let index = column.wrapping_add((i as u32).wrapping_mul(self.count)) as usize;
            if index >= 14 {
                return Err(Error::InvalidResource);
            }
            let sample = if self.width == 1 {
                *record.bytes.get(index).ok_or(Error::InvalidResource)? as i32
            } else {
                u16::from_le_bytes(
                    record
                        .bytes
                        .get(index * 2..index * 2 + 2)
                        .ok_or(Error::InvalidResource)?
                        .try_into()
                        .unwrap(),
                ) as i32
            };
            *out = mix_nonnegative_fixed(
                sample,
                self.offsets[index],
                self.offset_shift as u32,
                self.shifts[index] as u32,
                self.scales[index] as i32,
                precision,
            );
        }
        Ok(record.status)
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct PoolBlock {
    pub next: Option<usize>,
    pub base: u32,
    pub current: u32,
    pub end: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PoolAllocation {
    pub block: usize,
    pub address: u32,
}

pub trait PoolHost {
    fn block(&self, index: usize) -> PoolBlock;
    fn set_block(&mut self, index: usize, block: PoolBlock);
    fn allocate(&mut self, owner: u32, bytes: u32) -> Option<(usize, u32)>;
    fn free(&mut self, owner: u32, index: usize);
    fn clear(&mut self, address: u32, bytes: u32);
    fn report(&mut self, heap: &PrivateHeap<'_>, label: &[u8], blocks: u32);
}
impl PrivateHeap<'_> {
    /// fn_2db308 / EU fn_2db4c4 / KR fn_2db570.
    pub fn report(&self, label: &[u8], host: &mut impl PoolHost) {
        if self.owner == 0 {
            return;
        }
        let mut count = 0u32;
        let mut block = self.first_block;
        while let Some(index) = block {
            count = count.wrapping_add(1);
            block = host.block(index).next;
        }
        host.report(self, label, count);
    }
    /// fn_2db1d0 / EU fn_2db38c / KR fn_2db438.
    pub fn allocate(&mut self, size: u32, host: &mut impl PoolHost) -> Option<PoolAllocation> {
        let aligned = size.wrapping_add(3) & !3;
        let mut block = self.first_block;
        let mut previous = None;
        while let Some(index) = block {
            let b = host.block(index);
            if b.current.wrapping_add(aligned) <= b.end {
                host.set_block(
                    index,
                    PoolBlock {
                        current: b.current.wrapping_add(aligned),
                        ..b
                    },
                );
                self.allocated = self.allocated.wrapping_add(aligned);
                return Some(PoolAllocation {
                    block: index,
                    address: b.current,
                });
            }
            previous = block;
            block = b.next;
        }
        let mut growth = aligned;
        if self.expected > self.pool && self.expected - self.pool > growth {
            growth = self.expected - self.pool;
        } else if growth < self.expected >> 3 {
            growth = self.expected >> 3;
        }
        growth = growth.max(0x800);
        let (index, address) = host.allocate(self.owner, growth.wrapping_add(16))?;
        let base = address.wrapping_add(16);
        host.set_block(
            index,
            PoolBlock {
                next: None,
                base,
                current: base.wrapping_add(aligned),
                end: base.wrapping_add(growth),
            },
        );
        self.pool = self.pool.wrapping_add(growth);
        if let Some(previous) = previous {
            let mut p = host.block(previous);
            p.next = Some(index);
            host.set_block(previous, p);
        } else {
            self.first_block = Some(index);
        }
        self.allocated = self.allocated.wrapping_add(aligned);
        Some(PoolAllocation {
            block: index,
            address: base,
        })
    }
    /// fn_2db3c8 / EU fn_2db584 / KR fn_2db630.
    pub fn reset(&mut self, host: &mut impl PoolHost) {
        if self.owner == 0 {
            return;
        }
        self.report(b"before ResetPool", host);
        let mut block = self.first_block;
        while let Some(index) = block {
            let mut b = host.block(index);
            b.current = b.base;
            host.set_block(index, b);
            host.clear(b.base, b.end.wrapping_sub(b.base));
            block = b.next;
        }
        self.allocated = 0;
    }
    /// fn_2db4a0 / EU fn_2db65c / KR fn_2db708.
    pub fn free(&mut self, host: &mut impl PoolHost) {
        if self.owner == 0 {
            return;
        }
        self.report(b"before FreePool", host);
        let mut block = self.first_block;
        while let Some(index) = block {
            block = host.block(index).next;
            host.free(self.owner, index);
        }
        self.first_block = None;
        self.allocated = 0;
        self.pool = 0;
    }
}

impl BandSystem<'_> {
    /// fn_2a2224 / EU fn_2a23e0 / KR fn_2a248c.
    pub fn forward_lattice(
        &self,
        input_bits: u32,
        coefficient_bits: u32,
        output_bits: u32,
        output: &mut [i32],
    ) -> Result<(), Error> {
        self.validate(output, 3)?;
        let shift = output_bits.wrapping_sub(input_bits);
        let divisor = right(
            self.coefficients[0],
            coefficient_bits.wrapping_sub(input_bits >> 1),
        );
        let x = self.input[0];
        output[0] =
            if x > logical_right(i32::MAX as u32, shift) as i32 || x < right(i32::MIN, shift) {
                left(
                    Self::divide(left(x, shift.wrapping_sub(2)), divisor)?,
                    2u32.wrapping_add(input_bits >> 1),
                )
            } else {
                left(Self::divide(left(x, shift), divisor)?, input_bits >> 1)
            };
        let half = output_bits >> 1;
        for i in 1..self.input.len() {
            let mut sum = right(self.coefficients[(i - 1) * self.stride + 1], half)
                .wrapping_mul(right(output[i - 1], half));
            if i > 1 {
                sum = sum.wrapping_add(
                    right(self.coefficients[(i - 2) * self.stride + 2], half)
                        .wrapping_mul(right(output[i - 2], half)),
                );
            }
            let value =
                right(self.input[i], input_bits.wrapping_sub(coefficient_bits)).wrapping_sub(sum);
            let divisor = right(self.coefficients[i * self.stride], half);
            output[i] = left(
                if i > 1 && divisor == 0 {
                    value
                } else {
                    Self::divide(value, divisor)?
                },
                half,
            );
        }
        Ok(())
    }
    /// fn_2a23c4 / EU fn_2a2580 / KR fn_2a262c.
    pub fn backward_lattice(&self, bits: i32, output: &mut [i32]) -> Result<(), Error> {
        self.validate(output, 3)?;
        let shift = bits.wrapping_sub(11) as u32;
        let half = (bits.wrapping_sub(4) >> 1) as u32;
        for i in (0..self.input.len()).rev() {
            let mut sum = 0i32;
            if i + 1 < self.input.len() {
                sum = right(self.coefficients[i * self.stride + 1], half)
                    .wrapping_mul(right(output[i + 1], half));
            }
            if i + 2 < self.input.len() {
                sum = sum.wrapping_add(
                    right(self.coefficients[i * self.stride + 2], half)
                        .wrapping_mul(right(output[i + 2], half)),
                );
            }
            output[i] = left(
                Self::divide(
                    self.input[i].wrapping_sub(sum),
                    self.coefficients[i * self.stride] >> 7,
                )?,
                shift,
            );
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DecisionNode {
    pub question: u16,
    pub miss: i16,
    pub hit: i16,
}

/// Parses `raw` little-endian triplets into `out`, returning the node count.
pub fn parse_decision_nodes(raw: &[u8], out: &mut [DecisionNode]) -> Result<usize, Error> {
    if raw.len() % 6 != 0 || out.len() * 6 < raw.len() {
        return Err(Error::InvalidResource);
    }
    let count = raw.len() / 6;
    for (i, node) in out[..count].iter_mut().enumerate() {
        let base = i * 6;
        node.question = u16::from_le_bytes([raw[base], raw[base + 1]]);
        node.miss = i16::from_le_bytes([raw[base + 2], raw[base + 3]]);
        node.hit = i16::from_le_bytes([raw[base + 4], raw[base + 5]]);
    }
    Ok(count)
}

#[derive(Clone, Copy)]
pub struct SelectionFeatures<'a> {
    pub count: usize,
    pub columns: [Option<&'a [u8]>; 60],
}

pub struct DecisionTree<'a> {
    pub questions: &'a [u8],
    pub nodes: &'a [DecisionNode],
}

impl DecisionTree<'_> {
    /// fn_2d9dc0 / EU fn_2d9f7c / KR fn_2da028.
    pub fn select(&self, features: SelectionFeatures<'_>, position: usize) -> Result<u16, Error> {
        if position >= features.count || features.count > i16::MAX as usize {
            return Err(Error::InvalidResource);
        }
        let read = |column: usize, index: usize| -> Result<u8, Error> {
            features
                .columns
                .get(column)
                .and_then(|c| *c)
                .and_then(|c| c.get(index))
                .copied()
                .ok_or(Error::InvalidResource)
        };
        let mut index = 0usize;
        for _ in 0..self.nodes.len() {
            let node = self.nodes.get(index).ok_or(Error::InvalidResource)?;
            let question = self
                .questions
                .get(node.question as usize..)
                .ok_or(Error::InvalidResource)?;
            let kind = *question.first().ok_or(Error::InvalidResource)?;
            let selector = *question.get(1).ok_or(Error::InvalidResource)?;
            let value = match kind {
                29 => match features.columns[29] {
                    Some(column) => match *column.get(position).ok_or(Error::InvalidResource)? {
                        0 => Some(255),
                        value => Some(value),
                    },
                    None => Some(255),
                },
                22 => {
                    if read(2, position)? == 1 {
                        Some(1)
                    } else {
                        let value = read(22, position)? as u16 * 2;
                        Some(if value < 255 { value as u8 } else { 255 })
                    }
                }
                252..=255 => {
                    let neighbor = match kind {
                        252 => position.checked_sub(2),
                        253 => position.checked_sub(1),
                        254 => position.checked_add(1).filter(|&p| p < features.count),
                        _ => position.checked_add(2).filter(|&p| p < features.count),
                    };
                    Some(match neighbor {
                        Some(p) => read(2, p)?,
                        None => 255,
                    })
                }
                _ => match features
                    .columns
                    .get(kind as usize)
                    .ok_or(Error::InvalidResource)?
                {
                    Some(_) => Some(read(kind as usize, position)?),
                    None => None,
                },
            };
            let hit = if let Some(value) = value {
                let payload = &question[2..];
                match selector {
                    1 => *payload.first().ok_or(Error::InvalidResource)? == value,
                    255 => {
                        let range = payload.get(..2).ok_or(Error::InvalidResource)?;
                        range[0] <= value && value <= range[1]
                    }
                    _ => {
                        let values = payload
                            .get(..selector as usize)
                            .ok_or(Error::InvalidResource)?;
                        let mut low = 0;
                        let mut high = values.len();
                        let mut found = false;
                        while low < high {
                            let middle = low + (high - low - 1) / 2;
                            match values[middle].cmp(&value) {
                                core::cmp::Ordering::Less => low = middle + 1,
                                core::cmp::Ordering::Greater => high = middle,
                                core::cmp::Ordering::Equal => {
                                    found = true;
                                    break;
                                }
                            }
                        }
                        found
                    }
                }
            } else {
                false
            };
            let next = if hit { node.hit } else { node.miss };
            if next >= 0 {
                return Ok(next as u16);
            }
            index = -(next as i32) as usize;
        }
        Err(Error::InvalidResource)
    }
}

pub struct BandFactorization<'a> {
    pub coefficients: &'a mut [i32],
    pub count: usize,
    pub stride: usize,
}

impl BandFactorization<'_> {
    fn divide(value: i32, divisor: i32) -> i32 {
        crate::runtime::signed_division(value, divisor).map_or(0, |result| result.quotient)
    }

    fn normalize(value: i32, precision: u32) -> Result<i32, Error> {
        if value <= 0 {
            return Err(Error::InvalidResource);
        }
        let mut value = value;
        let mut shift = 0u32;
        while value < 0x1fff_ffff {
            value = value.wrapping_mul(4);
            shift += 2;
        }
        Ok(left(
            prediction_square_root(value as u32),
            precision.wrapping_sub(shift) >> 1,
        ))
    }

    fn factor(&mut self, precision: u32, lattice: bool) -> Result<bool, Error> {
        if self.count == 0 || self.stride < 3 || lattice && self.count < 2 {
            return Err(Error::InvalidResource);
        }
        let required = self
            .count
            .checked_mul(self.stride)
            .ok_or(Error::InvalidResource)?;
        if self.coefficients.len() < required {
            return Err(Error::Capacity {
                required,
                available: self.coefficients.len(),
            });
        }
        let half = precision >> 1;
        if lattice || self.coefficients[0] != 0 {
            self.coefficients[0] =
                Self::normalize(self.coefficients[0], if lattice { 16 } else { 18 })?;
        }
        for column in 1..3 {
            self.coefficients[column] = left(
                Self::divide(self.coefficients[column], right(self.coefficients[0], half)),
                half,
            );
        }
        for row in 1..self.count {
            let previous = (row - 1) * self.stride;
            let current = row * self.stride;
            let x = right(self.coefficients[previous + 1], half);
            let mut value = self.coefficients[current].wrapping_sub(x.wrapping_mul(x));
            self.coefficients[current] = value;
            if row > 1 {
                let x = right(self.coefficients[(row - 2) * self.stride + 2], half);
                value = value.wrapping_sub(x.wrapping_mul(x));
                self.coefficients[current] = value;
            }
            if value <= 0 {
                return Ok(false);
            }
            let normalized = Self::normalize(value, precision)?;
            self.coefficients[current] = normalized;
            let divisor = right(normalized, half);
            let x = right(self.coefficients[previous + 1], half);
            let y = right(self.coefficients[previous + 2], half);
            value = self.coefficients[current + 1].wrapping_sub(x.wrapping_mul(y));
            self.coefficients[current + 1] = value;
            self.coefficients[current + 1] = left(Self::divide(value, divisor), half);
            self.coefficients[current + 2] =
                left(Self::divide(self.coefficients[current + 2], divisor), half);
        }
        Ok(true)
    }

    /// fn_2a1c54 / EU fn_2a1e10 / KR fn_2a1ebc.
    pub fn factor_three(&mut self, precision: u32) -> Result<bool, Error> {
        self.factor(precision, false)
    }

    /// fn_2a1f50 / EU fn_2a210c / KR fn_2a21b8.
    pub fn factor_lattice(&mut self) -> Result<bool, Error> {
        self.factor(16, true)
    }
}

#[derive(Clone, Copy)]
pub struct PdfParameter {
    pub bias: i32,
    pub bias_bits: u16,
    pub sample_bits: u16,
    pub multiplier: u16,
}

impl PdfParameter {
    fn decode(self, value: i32, precision: u32) -> i32 {
        mix_fixed(
            value,
            self.bias,
            self.bias_bits as u32,
            self.sample_bits as u32,
            self.multiplier as i32,
            precision,
        )
    }
}

#[derive(Clone, Copy)]
pub struct VoiceBlock<'a> {
    pub data: Option<&'a [u8]>,
    pub data_origin: u32,
    pub offset: u32,
}

impl<'a> VoiceBlock<'a> {
    fn record(
        &self,
        frame: u32,
        size: u32,
        load: &mut impl FnMut(u32, u32) -> AcousticRecord<'a>,
    ) -> Result<AcousticRecord<'a>, Error> {
        let offset = self
            .offset
            .wrapping_add(frame.wrapping_sub(1).wrapping_mul(size));
        match self.data {
            Some(data) => Ok(AcousticRecord {
                status: 0,
                bytes: data
                    .get(offset.wrapping_sub(self.data_origin) as usize..)
                    .ok_or(Error::InvalidResource)?,
            }),
            None => Ok(load(offset, size)),
        }
    }
}

fn pdf_sample(data: &[u8], index: usize, width: u16) -> Result<i32, Error> {
    if width == 1 {
        Ok(*data.get(index).ok_or(Error::InvalidResource)? as i32)
    } else {
        let offset = index.checked_mul(2).ok_or(Error::InvalidResource)?;
        Ok(u16::from_le_bytes(
            data.get(offset..offset + 2)
                .ok_or(Error::InvalidResource)?
                .try_into()
                .unwrap(),
        ) as i32)
    }
}

pub struct PdfTable<'a> {
    pub width: u16,
    pub extra: u16,
    pub dimensions: u16,
    pub parameters: &'a [PdfParameter],
    pub blocks: &'a [VoiceBlock<'a>],
}

impl<'a> PdfTable<'a> {
    /// fn_2f6f94 / EU fn_2f7150 / KR fn_2f71fc.
    pub fn read_pair(
        &self,
        block: u32,
        frame: u32,
        column: u32,
        precision: u32,
        first: &mut i32,
        second: Option<&mut i32>,
        mut load: impl FnMut(u32, u32) -> AcousticRecord<'a>,
    ) -> Result<u32, Error> {
        let size = (self.dimensions as u32)
            .wrapping_mul(self.width as u32)
            .wrapping_mul(2)
            .wrapping_add(self.extra as u32);
        let record = self
            .blocks
            .get(block as usize)
            .ok_or(Error::InvalidResource)?
            .record(frame, size, &mut load)?;
        if record.status >= 0x80000000 {
            return Ok(record.status);
        }
        let index = column.wrapping_mul(2) as u16 as usize;
        *first = self
            .parameters
            .get(index)
            .ok_or(Error::InvalidResource)?
            .decode(pdf_sample(record.bytes, index, self.width)?, precision);
        if let Some(second) = second {
            let index = (index as u16).wrapping_add(1) as usize;
            *second = self
                .parameters
                .get(index)
                .ok_or(Error::InvalidResource)?
                .decode(pdf_sample(record.bytes, index, self.width)?, precision);
        }
        Ok(record.status)
    }
}

pub struct Codebook<'a> {
    pub dimensions: u16,
    pub width: u16,
    pub parameters: &'a [PdfParameter],
    pub data: &'a [u8],
    pub data_origin: u32,
}

impl Codebook<'_> {
    fn decode(&self, row: u8, element: usize, precision: u32) -> Result<i32, Error> {
        let offset = (row as u32)
            .wrapping_mul(self.dimensions as u32)
            .wrapping_mul(self.width as u32);
        let data = self
            .data
            .get(offset.wrapping_sub(self.data_origin) as usize..)
            .ok_or(Error::InvalidResource)?;
        Ok(self
            .parameters
            .get(element)
            .ok_or(Error::InvalidResource)?
            .decode(pdf_sample(data, element, self.width)?, precision))
    }
}

pub struct QuantizedPdfTable<'a> {
    pub dimensions: u16,
    pub extra: u16,
    pub channels: u16,
    pub stride: u16,
    pub blocks: &'a [VoiceBlock<'a>],
    pub codebooks: &'a [Codebook<'a>],
}

impl<'a> QuantizedPdfTable<'a> {
    fn record(
        &self,
        block: u32,
        frame: u32,
        load: &mut impl FnMut(u32, u32) -> AcousticRecord<'a>,
    ) -> Result<AcousticRecord<'a>, Error> {
        let size = (self.dimensions as u32)
            .wrapping_mul(self.channels as u32)
            .wrapping_mul(2)
            .wrapping_add(self.extra as u32);
        self.blocks
            .get(block as usize)
            .ok_or(Error::InvalidResource)?
            .record(frame, size, load)
    }

    fn decode(
        &self,
        record: &[u8],
        index: usize,
        element: usize,
        precision: u32,
    ) -> Result<i32, Error> {
        let prefix = (self.dimensions as usize)
            .checked_mul(self.channels as usize)
            .and_then(|n| n.checked_mul(2))
            .ok_or(Error::InvalidResource)?;
        let row = *record
            .get(prefix.checked_add(index).ok_or(Error::InvalidResource)?)
            .ok_or(Error::InvalidResource)?;
        self.codebooks
            .get(index)
            .ok_or(Error::InvalidResource)?
            .decode(row, element, precision)
    }

    /// fn_2f7420 / EU fn_2f75dc / KR fn_2f7688.
    pub fn read_pair(
        &self,
        block: u32,
        frame: u32,
        channel: u32,
        element: usize,
        pair: u32,
        precision: u32,
        output: &mut [i32; 2],
        mut load: impl FnMut(u32, u32) -> AcousticRecord<'a>,
    ) -> Result<u32, Error> {
        let record = self.record(block, frame, &mut load)?;
        if record.status >= 0x80000000 {
            return Ok(record.status);
        }
        let first = (self.stride as u32).wrapping_mul(pair) as u16 as u32;
        let second = (self.channels as u32)
            .wrapping_mul(self.stride as u32)
            .wrapping_add(first) as u16 as u32;
        output[0] = self.decode(
            record.bytes,
            first.wrapping_add(channel) as usize,
            element,
            precision,
        )?;
        output[1] = self.decode(
            record.bytes,
            second.wrapping_add(channel) as usize,
            element,
            precision,
        )?;
        Ok(0)
    }

    /// fn_2f760c / EU fn_2f77c8 / KR fn_2f7874.
    pub fn read_first(
        &self,
        block: u32,
        frame: u32,
        channel: u32,
        element: usize,
        pair: u32,
        precision: u32,
        output: &mut i32,
        mut load: impl FnMut(u32, u32) -> AcousticRecord<'a>,
    ) -> Result<u32, Error> {
        let record = self.record(block, frame, &mut load)?;
        if record.status >= 0x80000000 {
            return Ok(record.status);
        }
        let index = ((self.stride as u32).wrapping_mul(pair) as u16 as u32).wrapping_add(channel);
        *output = self.decode(record.bytes, index as usize, element, precision)?;
        Ok(0)
    }
}

/// fn_2a90c4 / EU fn_2a9280 / KR fn_2a932c.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct BlockCache<'a> {
    /// The buffer of the most recent load, or `None` when nothing is held.
    pub data: Option<&'a [u8]>,
    /// Offset the held buffer starts at.
    pub position: u32,
    /// Length of the held buffer.
    pub length: u32,
}

impl<'a> BlockCache<'a> {
    /// Returns the requested window, releasing a held buffer that does not cover it.
    pub fn read(
        &mut self,
        offset: u32,
        length: u32,
        release: impl FnOnce(&'a [u8]) -> u32,
        load: impl FnOnce(u32, u32) -> Result<&'a [u8], Error>,
    ) -> Result<&'a [u8], Error> {
        if let Some(data) = self.data {
            if self.position <= offset
                && offset.wrapping_add(length) <= self.length.wrapping_add(self.position)
            {
                let start = (offset - self.position) as usize;
                let end = start
                    .checked_add(length as usize)
                    .ok_or(Error::InvalidResource)?;
                return data.get(start..end).ok_or(Error::InvalidResource);
            }
            let status = release(data);
            self.data = None;
            self.position = 0;
            self.length = 0;
            if status >= 0x80000000 {
                return Err(Error::Engine(status));
            }
        }
        let data = load(offset, length)?;
        self.data = Some(data);
        self.position = offset;
        self.length = length;
        Ok(data)
    }
}

/// The zeroed synthesis frame `fn_2a4740` builds. The five block spans are
/// caller-owned; the original stores pointers to them, so Rust keeps slices.
pub struct SynthesisFrame<'a> {
    pub version: u16,
    pub owner: u32,
    pub history: &'a [u8],
    pub previous: &'a [u8],
    pub work: &'a [u8],
    pub input: &'a [u8],
    pub output: &'a [u8],
    pub f434: i32,
    pub f438: i32,
    pub f43c: i32,
    pub f564: i32,
    pub f568: i32,
}

/// fn_2a4740 / EU fn_2a48fc / KR fn_2a49a8.
///
/// Zeroes the synthesis frame and links it to its five caller-owned blocks in
/// the original's order. `blocks` holds five 0x34-byte entries; the frame's
/// remaining storage is zero in the original and zero here.
pub fn initialize_frame<'a>(owner: u32, blocks: &'a [u8; 0x34 * 5]) -> SynthesisFrame<'a> {
    SynthesisFrame {
        version: 1,
        owner,
        history: &blocks[..0x34],
        previous: &blocks[0x34..0x68],
        work: &blocks[0x68..0x9c],
        input: &blocks[0x9c..0xd0],
        output: &blocks[0xd0..0x104],
        f434: 0,
        f438: 0,
        f43c: 0,
        f564: 0,
        f568: 0,
    }
}

/// The persistent reader `fn_2a9420` keeps across calls: the block `base` and
/// `limit`, the current `pos`, the last load's `origin` and byte count, and the
/// window that load returned. `window` is `None` until a successful load.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct WindowState<'a> {
    pub kind: u8,
    pub base: u32,
    pub limit: u32,
    pub pos: u32,
    pub origin: u32,
    pub window: Option<&'a [u8]>,
    pub lastlen: u32,
}

/// fn_2a9420 / EU fn_2a95dc / KR fn_2a9688.
///
/// Copies `count` records of `size` bytes into `destination` from the caller's
/// cached block. `size` also selects the position alignment the original
/// applies first: 1, 2, 4 for 3..7, and 8. A `kind` of 2 reloads the window
/// through `cache`; otherwise the previous window is reused. `release` and
/// `load` stand in for `fn_179208` and `fn_17a1a4`. A failed reload leaves the
/// window cleared and returns 0, like the original.
pub fn read_window<'a>(
    destination: &mut [u8],
    size: u32,
    count: u32,
    state: &mut WindowState<'a>,
    cache: &mut BlockCache<'a>,
    release: impl FnOnce(&'a [u8]) -> u32,
    load: impl FnOnce(u32, u32) -> Result<&'a [u8], Error>,
) -> Result<u32, Error> {
    let alignment = match size {
        1 => 1,
        2 => 2,
        3..=7 => 4,
        8 => 8,
        _ => 4,
    };
    if alignment > 1 {
        let next = state
            .pos
            .wrapping_add((alignment - state.pos % alignment) % alignment);
        state.pos = next.min(state.limit);
    }
    let mut count = count;
    if size.wrapping_mul(count).wrapping_add(state.pos) > state.limit {
        if size == 0 {
            return Err(Error::InvalidResource);
        }
        count = state.limit.wrapping_sub(state.pos) / size;
    }
    if count == 0 {
        return Ok(count);
    }
    let mut total = size.wrapping_mul(count);
    if state.kind == 2 {
        if state.pos == 0 && total <= state.limit {
            if cache.data.is_none() || cache.position != state.base {
                total = state.limit;
            }
        }
        let window = match cache.read(state.base.wrapping_add(state.pos), total, release, load) {
            Ok(window) => window,
            Err(_) => {
                state.window = None;
                return Ok(0);
            }
        };
        state.origin = state.base.wrapping_add(state.pos);
        state.lastlen = total;
        state.window = Some(window);
    }
    let length = size.wrapping_mul(count) as usize;
    let skip = state
        .base
        .wrapping_add(state.pos)
        .wrapping_sub(state.origin) as usize;
    let end = skip.checked_add(length).ok_or(Error::InvalidResource)?;
    let window = state.window.ok_or(Error::InvalidResource)?;
    let source = window.get(skip..end).ok_or(Error::InvalidResource)?;
    let available = destination.len();
    destination
        .get_mut(..length)
        .ok_or(Error::Capacity {
            required: length,
            available,
        })?
        .copy_from_slice(source);
    state.pos = state.pos.wrapping_add(size.wrapping_mul(count));
    Ok(count)
}

/// fn_2a9690 / EU fn_2a984c / KR fn_2a98f8.
///
/// The table is 320 entries; the caller supplies the one loaded for the voice.
/// Indices outside it are rejected rather than reading neighbouring data.
pub fn dequantize(
    flags: u8,
    stereo: bool,
    left: &[i32],
    left_position: &mut usize,
    right: &[i32],
    right_position: &mut usize,
    table: &[i32],
    output: &mut [u16; 2],
) -> Result<(), Error> {
    fn interpolate(table: &[i32], value: u32) -> Result<u32, Error> {
        let index = (value >> 12) as usize;
        let low = *table.get(index).ok_or(Error::InvalidResource)? as u32;
        let high = (*table.get(160 + index).ok_or(Error::InvalidResource)? >> 6) as u32;
        Ok(high.wrapping_mul((value >> 6) & 0x3f).wrapping_add(low))
    }
    output[0] = if flags & 2 == 0 {
        0
    } else {
        let sample = *left.get(*left_position).ok_or(Error::InvalidResource)?;
        *left_position += 1;
        let value = sample.wrapping_shr(12).wrapping_add(-5056) as u32;
        (interpolate(table, value)? << 6 >> 16) as u16
    };
    if stereo {
        output[1] = if flags & 4 == 0 {
            0
        } else {
            let sample = *right.get(*right_position).ok_or(Error::InvalidResource)?;
            *right_position += 1;
            let value = sample.wrapping_shr(8) as u32;
            (interpolate(table, value)? >> 16) as u16
        };
    }
    Ok(())
}

/// fn_2dc4cc / EU fn_2dc688 / KR fn_2dc734.
///
/// Advances the SELECT_BET2 parameter ramp one sample at a time and filters
/// each sample through `fn_2dbc3c`. `current` and `step` are the caller's ramp
/// state, `parameters` is the frame the filter reads and this rewrites, `last`
/// bounds the ramp, `table` is the borrowed gain table and `input`/`output`
/// are the sample spans.
pub fn advance_prediction(
    filter: &mut PredictionFilter<'_>,
    current: &mut [i32],
    step: &[i32],
    parameters: &mut [i16],
    last: i32,
    table: &[i32],
    input: &[i16],
    output: &mut [i16],
) -> Result<(), Error> {
    let limit = last.max(11) as usize;
    if current.len() <= limit || step.len() <= limit || parameters.len() <= limit {
        return Err(Error::Capacity {
            required: limit + 1,
            available: current.len().min(step.len()).min(parameters.len()),
        });
    }
    let count = input.len().min(output.len());
    for i in 0..count {
        let index = (parameters[0] as i32 >> 7) as usize;
        let gain = *table.get(index).ok_or(Error::Capacity {
            required: index.saturating_add(1),
            available: table.len(),
        })?;
        let sample = filter.filter(((gain as i64 * input[i] as i64) >> 26) as i32, parameters)?;
        current[0] = current[0].wrapping_add(step[0]);
        parameters[0] = (current[0] >> 10) as i16;
        for j in 1..5 {
            current[j] = current[j].wrapping_add(step[j]);
            parameters[j] = (current[j] >> 10) as i16;
        }
        if parameters[0] < 0 {
            parameters[0] = 0;
        }
        for j in 5..11 {
            current[j] = current[j].wrapping_add(step[j]);
            parameters[j] = (current[j] >> 9) as i16;
        }
        let mut j = 11;
        loop {
            current[j] = current[j].wrapping_add(step[j]);
            parameters[j] = (current[j] >> 8) as i16;
            j += 1;
            if last < j as i32 {
                break;
            }
        }
        let clipped = if (sample.wrapping_add(0xf332) as u32) < 0x1e665 {
            sample
        } else if sample > 0xe8f6 {
            let mut value = sample - 0xe8f6;
            value = 0xe8f6 + ((value.wrapping_add(value >> 2)) >> 1);
            if value > 0xf333 {
                let delta = value - 0xf333;
                value = 0xf333 + ((delta.wrapping_add(delta >> 2)) >> 2);
            }
            value.min(0xffff)
        } else if sample < -0xe8f6 {
            let mut value = sample + 0xe8f6;
            value = -0xe8f6 + ((value.wrapping_add(value >> 2)) >> 1);
            if value < -0xf333 {
                let delta = value + 0xf333;
                value = -0xf333 + ((delta.wrapping_add(delta >> 2)) >> 2);
            }
            value.max(-0x10000)
        } else {
            continue;
        };
        output[i] = (clipped >> 1) as i16;
    }
    Ok(())
}

/// The three trailing passes and the copy-back shared by every filter stage
/// of `fn_2dbc3c`. `value` is the running sample at `buffer + 5`.
fn prediction_tail(
    memory: &mut [i32],
    buffer: usize,
    order: usize,
    alpha: i32,
    parameters: &[i16],
    mut value: i32,
    mut acc: i32,
) -> i32 {
    let mut destination = 5;
    let mut old = 4;
    let mut source = 6;
    let mut coefficient = 5;
    loop {
        value = value.wrapping_add(
            memory[buffer + source]
                .wrapping_sub(memory[buffer + old])
                .wrapping_mul(alpha)
                >> 14,
        );
        source += 1;
        old += 1;
        memory[buffer + destination] = value;
        destination += 1;
        acc = acc.wrapping_add((parameters[coefficient] as i32).wrapping_mul(value) >> 13);
        coefficient += 1;
        value = memory[buffer + destination];
        if destination > 10 {
            break;
        }
    }
    loop {
        value = value.wrapping_add(
            memory[buffer + source]
                .wrapping_sub(memory[buffer + old])
                .wrapping_mul(alpha)
                >> 14,
        );
        source += 1;
        old += 1;
        memory[buffer + destination] = value;
        destination += 1;
        acc = acc.wrapping_add((parameters[coefficient] as i32).wrapping_mul(value) >> 14);
        coefficient += 1;
        value = memory[buffer + destination];
        if destination > order {
            break;
        }
    }
    loop {
        let first = memory[buffer + old];
        memory[buffer + destination] = first;
        let second = memory[buffer + old - 1];
        memory[buffer + destination - 1] = second;
        old -= 2;
        destination -= 2;
        if destination <= 1 {
            break;
        }
    }
    acc
}

/// One whole-band pass of `fn_2dbc3c`: the seed pair, the three weighted
/// passes and the copy-back, returning the accumulated excitation.
fn prediction_block(
    memory: &mut [i32],
    buffer: usize,
    order: usize,
    alpha: i32,
    beta: i32,
    parameters: &[i16],
    first: i32,
) -> i32 {
    memory[buffer] = first;
    memory[buffer + 1] = (first.wrapping_mul(beta) >> 14)
        .wrapping_add((memory[buffer + 1].wrapping_mul(alpha)) >> 14);
    let mut value = memory[buffer + 2];
    let mut destination = 2;
    let mut old = 1;
    let mut source = 3;
    let mut coefficient = 2;
    let mut acc = 0i32;
    loop {
        value = value.wrapping_add(
            memory[buffer + source]
                .wrapping_sub(memory[buffer + old])
                .wrapping_mul(alpha)
                >> 14,
        );
        source += 1;
        old += 1;
        memory[buffer + destination] = value;
        destination += 1;
        acc = acc.wrapping_add((parameters[coefficient] as i32).wrapping_mul(value) >> 12);
        coefficient += 1;
        value = memory[buffer + destination];
        if destination > 4 {
            break;
        }
    }
    prediction_tail(memory, buffer, order, alpha, parameters, value, acc)
}

/// fn_2dbc3c / EU fn_2dbdf8 / KR fn_2dbea4.
///
/// One frame of the SELECT_BET2 prediction filter. `memory` is the caller's
/// interleaved history, `coefficients` and `gains` are borrowed voice tables
/// and `parameters` is the parameter frame being filtered.
pub struct PredictionFilter<'a> {
    pub order: i32,
    pub memory: &'a mut [i32],
    pub coefficients: &'a [i16],
    pub gains: &'a [i16],
    pub alpha: i16,
    pub beta: i16,
    pub count: u8,
    pub index: u8,
    pub offsets: [u16; 11],
}

impl PredictionFilter<'_> {
    fn required(&self) -> usize {
        let index = self.index as usize;
        let count = self.count as usize;
        let order = (self.order.max(0) as usize).max(10);
        let base = self.offsets[10] as usize;
        let mut required = 2 * index + 2;
        required = required.max(base + self.offsets[count + 1] as usize + count + 1);
        for k in 1..=count {
            required = required.max(base + self.offsets[k] as usize + order + 3);
        }
        required
    }

    /// fn_2dbc3c / EU fn_2dbdf8 / KR fn_2dbea4.
    pub fn filter(&mut self, input: i32, parameters: &[i16]) -> Result<i32, Error> {
        let required = self.required();
        if self.memory.len() < required {
            return Err(Error::Capacity {
                required,
                available: self.memory.len(),
            });
        }
        let index = self.index as usize;
        let order = self.order.max(0) as usize;
        let count = self.count as usize;
        let parameters_required = order.max(11) + 1;
        if parameters.len() < parameters_required
            || self.coefficients.len() < index.max(3) + 1
            || self.gains.len() < 4
        {
            return Err(Error::Capacity {
                required: parameters_required,
                available: parameters.len(),
            });
        }
        let alpha = self.alpha as i32;
        let beta = self.beta as i32;
        let offsets = self.offsets;
        let coefficients = self.coefficients;
        let gains = self.gains;
        let memory = &mut self.memory[..];
        let mq = index + 1;
        let coefficient = |m: usize| coefficients.get(m).copied().unwrap_or(0) as i32;

        let mut sum = ((memory[index] as i64 * alpha as i64) >> 14)
            + ((memory[mq + index - 1] as i64 * beta as i64) >> 14);
        memory[index] = sum as i32;
        let mut product = (sum * parameters[1] as i64) >> 12;
        memory[mq + index] = product as i32;
        let mut weight = ((product * coefficient(index) as i64) >> 18) as i32;
        if index == 3 {
            weight = memory[mq + 3].wrapping_mul(coefficient(3)) >> 16;
        }
        let mut acc = weight;
        let mut value = input.wrapping_add(
            if index & 1 != 0 {
                weight
            } else {
                weight.wrapping_neg()
            }
            .wrapping_mul(2),
        );
        if index > 3 {
            for j in 0..index - 3 {
                let m = index - 1 - j;
                sum = ((memory[mq + m - 1] as i64 * beta as i64) >> 14)
                    + ((memory[m] as i64 * alpha as i64) >> 14);
                memory[m] = sum as i32;
                product = (sum * parameters[1] as i64) >> 12;
                memory[mq + m] = product as i32;
                weight = ((product * coefficient(m) as i64) >> 18) as i32;
                if m == 3 {
                    weight = memory[mq + 3].wrapping_mul(coefficient(3)) >> 16;
                }
                let signed = if (j.wrapping_neg() ^ (index - 1)) & 1 != 0 {
                    weight
                } else {
                    weight.wrapping_neg()
                };
                value = value.wrapping_add(signed.wrapping_mul(2));
                acc = acc.wrapping_add(weight);
            }
        }
        sum = ((memory[2] as i64 * alpha as i64) + (memory[mq + 1] as i64 * beta as i64)) >> 14;
        memory[2] = sum as i32;
        product = (sum * parameters[1] as i64) >> 12;
        memory[mq + 2] = product as i32;
        sum = ((memory[1] as i64 * alpha as i64) >> 14) + ((memory[mq] as i64 * beta as i64) >> 14);
        memory[1] = sum as i32;
        let second = (sum * parameters[1] as i64) >> 12;
        let w2 = ((product * coefficient(2) as i64) >> 15) as i32;
        value = value.wrapping_sub(w2.wrapping_mul(2));
        memory[mq + 1] = second as i32;
        let h = rounded_short_product(second as i32, coefficients[1], 15);
        let current = (value >> 1).wrapping_add(h);
        memory[mq] = current;
        let mut acc2 = current.wrapping_add(h).wrapping_add(acc).wrapping_add(w2);
        let mut acc3 = 0i32;

        let base = offsets[10] as usize;
        let pv = base + offsets[count + 1] as usize;
        let mut k = count;
        loop {
            let buffer = base + offsets[k] as usize;
            let mut acc = 0i32;
            let mut wide = memory[pv + k - 1];
            memory[buffer] = wide;
            memory[buffer + 1] = (((wide as i64 * beta as i64) >> 14)
                + ((memory[buffer + 1] as i64 * alpha as i64) >> 14))
                as i32;
            wide = memory[buffer + 2];
            let mut destination = 2;
            let mut old = 1;
            let mut source = 3;
            let mut parameter = 2;
            loop {
                let v = wide as i64
                    + (((memory[buffer + source] as i64 - memory[buffer + old] as i64)
                        * alpha as i64)
                        >> 14);
                source += 1;
                old += 1;
                memory[buffer + destination] = v as i32;
                acc = acc.wrapping_add(((parameters[parameter] as i64 * v) >> 12) as i32);
                parameter += 1;
                destination += 1;
                wide = memory[buffer + destination];
                if destination > 4 {
                    break;
                }
            }
            let acc = prediction_tail(memory, buffer, order, alpha, parameters, wide, acc);
            memory[pv + k] = acc;
            let g = memory[pv + 3] as i64 * gains[3] as i64;
            let weight = if k == 3 {
                (g >> 16) as i32
            } else {
                (g >> 18) as i32
            };
            acc2 = acc2.wrapping_add(if k & 1 != 0 {
                weight
            } else {
                weight.wrapping_neg()
            });
            acc3 = acc3.wrapping_add(weight);
            k -= 1;
            if k < 3 {
                break;
            }
        }

        let acc = prediction_block(
            memory,
            base + offsets[2] as usize,
            order,
            alpha,
            beta,
            parameters,
            memory[pv + 1],
        );
        memory[pv + 2] = acc;
        let v2 = ((gains[2] as i64 * acc as i64) >> 15) as i32;
        acc2 = acc2.wrapping_sub(v2);
        acc3 = acc3.wrapping_add(v2);
        let acc = prediction_block(
            memory,
            base + offsets[1] as usize,
            order,
            alpha,
            beta,
            parameters,
            memory[pv],
        );
        memory[pv + 1] = acc;
        let v1 = ((gains[1] as i64 * acc as i64) >> 15) as i32;
        memory[pv] = acc2.wrapping_add(v1);
        Ok(v1.wrapping_add(acc3).wrapping_add(memory[pv]))
    }
}

/// fn_2dde50 / EU fn_2de00c / KR fn_2de0b8.
///
/// Lehmer noise generator for the SELECT_BET2 band synthesis. `seed` is the
/// caller's state word, advanced in place; `output` is the caller-owned
/// sample span, written for `count` entries; `gain` is the band noise gain.
pub fn generate_noise(
    seed: &mut u32,
    output: &mut [i16],
    gain: i32,
    count: i32,
) -> Result<(), Error> {
    let count = count.max(0) as usize;
    if output.len() < count {
        return Err(Error::Capacity {
            required: count,
            available: output.len(),
        });
    }
    let scale = (gain >> 2) as i16 as i32;
    let mut state = *seed;
    for slot in &mut output[..count] {
        let high = state >> 16;
        let scaled = high.wrapping_mul(0x41a7);
        let mut value = (state & 0xffff)
            .wrapping_mul(0x41a7)
            .wrapping_add((scaled & 0xffff_7fff).wrapping_mul(0x1_0000))
            .wrapping_add(scaled >> 15);
        if value > 0x7fff_ffff {
            value = value.wrapping_add(0x8000_0001);
        }
        value &= 0x7fff_ffff;
        state = value;
        let centred = (0x4000_0000u32.wrapping_sub(value) >> 16) as u16 as i16;
        *slot = ((centred as i32 * scale) >> 14) as i16;
    }
    *seed = state;
    Ok(())
}

/// One band of the caller's voice state read by `fn_2dcb84`: the band sample
/// and filter tables, the frequency threshold that selects it and the two
/// band gains.
#[derive(Clone, Copy)]
pub struct VoiceBand<'a> {
    pub samples: &'a [i16],
    pub filter: &'a [i16],
    pub threshold: u32,
    pub gain: u16,
    pub noise_gain: u16,
}

/// fn_2dcb84 / EU fn_2dcd40 / KR fn_2dcdec.
///
/// One band-synthesis frame. Every field is caller-owned state borrowed for
/// the call: `seed` is the noise generator's word, `bands` the voice's band
/// table, and `cached_samples`, `noise`, `overlap` and `history` are the four
/// work buffers. `cached_gain`, `cached_length`, `cached_band`, `position`,
/// `step` and `overlap_count` are updated in place.
pub struct VoiceSynthesis<'a> {
    pub frequency_scale: u32,
    pub gain: i32,
    pub noise_gain: i32,
    pub seed: &'a mut u32,
    pub sample_count: i16,
    pub filter_length: i16,
    pub cached_gain: i16,
    pub cached_length: i32,
    pub cached_band: i32,
    pub position: i32,
    pub step: i32,
    pub overlap_count: i32,
    pub overlap_capacity: i32,
    pub bands: &'a [VoiceBand<'a>],
    pub cached_samples: &'a mut [i16],
    pub noise: &'a mut [i16],
    pub overlap: &'a mut [i16],
    pub history: &'a mut [i16],
}

impl VoiceSynthesis<'_> {
    /// fn_2dcb84 / EU fn_2dcd40 / KR fn_2dcdec.
    pub fn synthesize(
        &mut self,
        length: i32,
        frequency: i32,
        output: &mut [i16],
        output_count: i32,
    ) -> Result<(), Error> {
        let samples = self.sample_count.max(0) as usize;
        let filter_length = self.filter_length.max(0) as usize;
        let span = length.max(0) as usize;
        let capacity = self.overlap_capacity.max(0) as usize;
        let noise_required = filter_length
            .checked_add(span)
            .ok_or(Error::InvalidResource)?;
        if self.noise.len() < noise_required {
            return Err(Error::Capacity {
                required: noise_required,
                available: self.noise.len(),
            });
        }
        if self.cached_samples.len() < samples
            || self.overlap.len() < capacity
            || output.len() < samples.max(span)
            || output_count.max(0) as usize > output.len()
        {
            return Err(Error::Capacity {
                required: samples.max(capacity),
                available: self.cached_samples.len().min(self.overlap.len()),
            });
        }
        let scaled = self.frequency_scale.wrapping_mul(frequency as u32);
        if scaled > 0x7a_e148 {
            let mut band = 0usize;
            for (index, candidate) in self.bands.iter().enumerate() {
                if candidate.threshold >= scaled {
                    band = index;
                    break;
                }
            }
            let gain = if self.cached_length == length {
                self.cached_gain
            } else {
                rounded_square_root(length.wrapping_shl(6) as u32) as i16
            };
            self.cached_length = length;
            let band_entry = self.bands.get(band).ok_or(Error::InvalidResource)?;
            if self.cached_gain == gain && self.cached_band == band as i32 {
                for index in 0..samples {
                    output[index] = self.cached_samples[index];
                }
            } else {
                let product = (band_entry.gain as i64).wrapping_mul(self.gain as i64);
                let amplitude = product.wrapping_mul(gain as i64) as u64 >> 28;
                for index in 0..samples {
                    let value = rounded_short_product(
                        amplitude as i32,
                        band_entry.samples.get(index).copied().unwrap_or(0),
                        7,
                    )
                    .clamp(i16::MIN as i32, i16::MAX as i32);
                    output[index] = value as i16;
                    self.cached_samples[index] = value as i16;
                }
            }
            self.cached_gain = gain;
            if self.overlap_count > 0 {
                let count =
                    (self.overlap_count.min(length) as usize).min(output_count.max(0) as usize);
                for index in 0..count {
                    output[index] = add_i16(output[index] as i32, self.overlap[index] as i32);
                }
                if self.overlap_count > count as i32 {
                    let remaining = self.overlap_count - count as i32;
                    self.overlap
                        .copy_within(count..count + remaining as usize, 0);
                    self.overlap_count = remaining.max(0);
                }
            }
            self.overlap[..capacity].fill(0);
            if (self.sample_count as i32) > length {
                let extra = (self.sample_count as i32 - length) as usize;
                self.overlap[..extra].copy_from_slice(&output[span..span + extra]);
                self.overlap_count = self.overlap_count.max(extra as i32);
            }
            let noise_gain = (band_entry.noise_gain as u32).wrapping_mul(self.noise_gain as u32);
            generate_noise(
                self.seed,
                &mut self.noise[filter_length..],
                noise_gain as i32 >> 14,
                length,
            )?;
            self.noise[..filter_length].copy_from_slice(&self.history[..filter_length]);
            self.history[..filter_length].copy_from_slice(&self.noise[span..span + filter_length]);
            convolve_in_place(
                self.noise,
                filter_length,
                filter_length,
                span,
                &band_entry.filter[..filter_length],
            )?;
            self.noise
                .copy_within(filter_length..filter_length + span, 0);
            if self.position > self.step {
                self.position -= self.step;
            }
            self.cached_band = band as i32;
        } else {
            let end = output_count.max(0) as usize;
            output[..end].fill(0);
            generate_noise(self.seed, self.noise, self.noise_gain, length)?;
            self.cached_band = -1;
            self.position = 0;
        }
        for index in 0..span {
            output[index] = add_i16(output[index] as i32, self.noise[index] as i32);
        }
        Ok(())
    }
}

/// fn_2dda54 / EU fn_2ddc10 / KR fn_2ddcbc.
///
/// The SELECT_BET2 all-pole lattice filter. `buffer` is the caller's work
/// span, `history` the previous frame's `order` tail, `coefficients` the
/// `taps` Q14 prediction coefficients. A frame longer than `cap` reallocates
/// through `replacement` (the caller's substitute for `fn_1963b8`) and
/// returns -1 so the caller can retry.
pub struct LatticeFilter<'a> {
    pub bypass: i32,
    pub cap: i32,
    pub order: i32,
    pub taps: i32,
    pub bufsize: i32,
    pub coefficients: &'a [i32],
    pub buffer: &'a mut [i32],
    pub history: &'a mut [i32],
}

impl<'a> LatticeFilter<'a> {
    /// fn_2dda54 / EU fn_2ddc10 / KR fn_2ddcbc.
    pub fn apply(
        &mut self,
        samples: &mut [i16],
        count: i32,
        replacement: Option<&'a mut [i32]>,
    ) -> Result<i32, Error> {
        if self.bypass == 1 {
            return Ok(0);
        }
        let span = count.max(0) as usize;
        let order = self.order.max(0) as usize;
        let taps = self.taps.max(0) as usize;
        if count > self.cap {
            if self.bypass != 0 {
                return Ok(-1);
            }
            self.cap = count;
            let needed = (count as usize)
                .checked_add(order * 2)
                .ok_or(Error::InvalidResource)?;
            if needed as i32 == self.bufsize {
                return Ok(-1);
            }
            if let Some(buffer) = replacement {
                self.buffer = buffer;
                self.bufsize = needed as i32;
                return Ok(-1);
            }
        }
        let required = (taps + span).max(span + order);
        if self.buffer.len() < required
            || self.history.len() < order
            || self.coefficients.len() < taps
            || samples.len() < span
        {
            return Err(Error::Capacity {
                required,
                available: self.buffer.len(),
            });
        }
        self.buffer[..order].copy_from_slice(&self.history[..order]);
        for index in 0..span {
            self.buffer[taps + index] = samples[index] as i32;
        }
        self.history[..order].copy_from_slice(&self.buffer[span..span + order]);
        for index in 0..span {
            let rem = span - index;
            let mut acc = 0i32;
            let mut sample = taps + rem - 1;
            let mut coefficient = 0usize;
            if taps & 1 != 0 {
                acc = acc.wrapping_add(
                    self.buffer[sample].wrapping_mul(self.coefficients[coefficient]) >> 14,
                );
                sample -= 1;
                coefficient += 1;
            }
            for _ in 0..taps >> 1 {
                acc = acc.wrapping_add(
                    self.buffer[sample].wrapping_mul(self.coefficients[coefficient]) >> 14,
                );
                sample -= 1;
                coefficient += 1;
                acc = acc.wrapping_add(
                    self.buffer[sample].wrapping_mul(self.coefficients[coefficient]) >> 14,
                );
                sample -= 1;
                coefficient += 1;
            }
            self.buffer[taps + rem - 1] = acc.clamp(i16::MIN as i32, i16::MAX as i32);
        }
        for index in 0..span {
            samples[index] = self.buffer[taps + index] as i16;
        }
        Ok(0)
    }
}
