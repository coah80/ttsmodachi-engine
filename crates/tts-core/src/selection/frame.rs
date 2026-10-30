//! Arena-based translation of the synthesis frame driver.

use super::arena::{Arena, Host};

const SAMPLE_RATE: u32 = 0x04;
const MIN_PERIOD: u32 = 0x10;
const MAX_PERIOD: u32 = 0x14;
const PITCH_OFFSET: u32 = 0x20;
const FRAME_SCALE: u32 = 0x24;
const MODE: u32 = 0x32;
const COEFFICIENTS: u32 = 0x44;
const PARAMETERS: u32 = 0x54;
const ORDER: u32 = 0x60;
const COEFFICIENT_STEP: u32 = 0x6c;
const LEFTOVER: u32 = 0x7c;
const BUFFER_TAG: u32 = 0x88;
const EXCITATION: u32 = 0x8c;
const OUTPUT: u32 = 0xac;
const FRAME_LENGTH: u32 = 0xd8;
const RECIPROCAL_LENGTH: u32 = 0xdc;
const PITCH_THRESHOLD: u32 = 0xfc;
const DEFAULT_PITCH: u32 = 0x100;
const FREQUENCY: u32 = 0x104;
const PREVIOUS_FREQUENCY: u32 = 0x106;
const CHANGED: u32 = 0x11c;
const VOICED: u32 = 0x120;
const PITCH: u32 = 0x124;
const ENERGY: u32 = 0x126;
const NEXT_COEFFICIENTS: u32 = 0x12c;
const PREVIOUS_PITCH: u32 = 0x138;
const PREVIOUS_ENERGY: u32 = 0x13a;
const PREVIOUS_COEFFICIENTS: u32 = 0x140;
const POSITION: u32 = 0x14c;
const CONVERTED_COEFFICIENTS: u32 = 0x19c;
const SUPPRESS_OUTPUT: u32 = 0x1f0;

/// fn_2a977c / EU fn_2a9938 / KR fn_2a99e4.
///
/// Drives one synthesis frame: pitch selection, coefficient blending, the
/// excitation loop and the final lattice filter. The seven callees are host
/// calls here; the frame's own state is written through the arena.
#[allow(clippy::too_many_arguments)]
pub fn render_frame(
    arena: &mut Arena<'_>,
    host: &mut impl Host,
    state: u32,
    pitch_argument: u32,
    coefficients: u32,
    energy: u32,
    length: u32,
) -> u32 {
    let mut pitch = pitch_argument;
    arena.set_u32(state.wrapping_add(CHANGED), 0);
    let voiced = arena.u32(state.wrapping_add(VOICED));
    if voiced == 1 {
        if pitch == 0 {
            arena.set_u32(state.wrapping_add(CHANGED), 1);
        }
    } else if voiced == 0 && pitch != 0 {
        arena.set_u32(state.wrapping_add(CHANGED), 1);
    }
    if pitch != 0 {
        let mut value = ((arena.u16(state.wrapping_add(PITCH_OFFSET)) as i16 as i32) as u32)
            .wrapping_add(pitch) as i32;
        if value < 0xa00 {
            value = 0xa00;
        }
        pitch = value as u16 as u32;
    }
    arena.set_u32(state.wrapping_add(VOICED), u32::from(pitch != 0));
    let mut frame_length = arena.u32(state.wrapping_add(FRAME_LENGTH)) as i32;
    let mut reciprocal = arena.u32(state.wrapping_add(RECIPROCAL_LENGTH));
    arena.set_u32(length, frame_length as u32);
    if arena.u32(state.wrapping_add(FRAME_SCALE)) as i32 != 0x4000 {
        frame_length = (arena.u32(state.wrapping_add(FRAME_LENGTH)) as i32)
            .wrapping_mul(arena.u32(state.wrapping_add(FRAME_SCALE)) as i32)
            .wrapping_add(0x2000)
            >> 14;
        reciprocal = host
            .call(arena, 0x1994d0, &[0x10000, frame_length as u16 as u32])
            .0
            & 0xffff;
        arena.set_u32(length, frame_length as u32);
    }
    let adjusted_pitch = if pitch == 0 {
        0
    } else {
        let value =
            host.call(
                arena,
                0x1994d0,
                &[arena.u32(state.wrapping_add(SAMPLE_RATE)) << 12, pitch],
            )
            .0 & 0xffff;
        if value < 0xffff {
            value
        } else {
            0xffff
        }
    };
    arena.set_u16(state.wrapping_add(PITCH), adjusted_pitch as u16);
    if adjusted_pitch == 0 {
        arena.set_u16(
            state.wrapping_add(PITCH),
            arena.u16(state.wrapping_add(DEFAULT_PITCH)),
        );
    }
    let order = arena.u32(state.wrapping_add(ORDER)) as i32;
    let count = order.wrapping_add(1) as u32;
    host.call(
        arena,
        0x195ae8,
        &[
            arena.u32(state.wrapping_add(NEXT_COEFFICIENTS)),
            coefficients,
            count.wrapping_mul(4),
        ],
    );
    arena.set_u16(state.wrapping_add(ENERGY), energy as u16);
    host.call(
        arena,
        0x2dbb2c,
        &[
            state,
            arena.u32(state.wrapping_add(NEXT_COEFFICIENTS)),
            arena.u32(state.wrapping_add(CONVERTED_COEFFICIENTS)),
        ],
    );
    host.call(
        arena,
        0x195ae8,
        &[
            arena.u32(state.wrapping_add(NEXT_COEFFICIENTS)),
            arena.u32(state.wrapping_add(CONVERTED_COEFFICIENTS)),
            count.wrapping_mul(4),
        ],
    );
    host.call(
        arena,
        0x2dca78,
        &[state, arena.u16(state.wrapping_add(ENERGY)) as u32],
    );
    let mut update_previous_pitch = true;
    if arena.u32(state.wrapping_add(CHANGED)) == 0 {
        let pitch = arena.u16(state.wrapping_add(PITCH)) as i32;
        let previous = arena.u16(state.wrapping_add(PREVIOUS_PITCH)) as i32;
        let mut delta = pitch.wrapping_sub(previous);
        if delta <= 0 {
            delta = previous.wrapping_sub(pitch);
        }
        if delta < arena.u32(state.wrapping_add(PITCH_THRESHOLD)) as i32 {
            update_previous_pitch = false;
        }
    }
    if update_previous_pitch {
        arena.set_u16(
            state.wrapping_add(PREVIOUS_PITCH),
            arena.u16(state.wrapping_add(PITCH)),
        );
    }
    let next = arena.u32(state.wrapping_add(NEXT_COEFFICIENTS));
    let previous = arena.u32(state.wrapping_add(PREVIOUS_COEFFICIENTS));
    let step = arena.u32(state.wrapping_add(COEFFICIENT_STEP));
    let mut i = 0;
    while i < count {
        let difference = (arena.u32(next.wrapping_add(i.wrapping_mul(4))) as i32 as i64)
            .wrapping_sub(arena.u32(previous.wrapping_add(i.wrapping_mul(4))) as i32 as i64)
            as u64;
        let value = (difference.wrapping_mul(reciprocal as u64) >> 16) as u32;
        arena.set_u32(step.wrapping_add(i.wrapping_mul(4)), value);
        i += 1;
    }
    host.call(
        arena,
        0x195ae8,
        &[
            arena.u32(state.wrapping_add(COEFFICIENTS)),
            previous,
            count.wrapping_mul(4),
        ],
    );
    let parameters = arena.u32(state.wrapping_add(PARAMETERS));
    for i in 0..5 {
        let value = (arena.u32(previous.wrapping_add(i * 4)) as i32 >> 10) as u16;
        arena.set_u16(parameters.wrapping_add(i * 2), value);
    }
    for i in 0..6 {
        let value = (arena.u32(previous.wrapping_add((i + 5) * 4)) as i32 >> 9) as u16;
        arena.set_u16(parameters.wrapping_add((i + 5) * 2), value);
    }
    let mut i = 0;
    while i < order - 10 {
        let value = (arena.u32(previous.wrapping_add((i as u32 + 11) * 4)) as i32 >> 8) as u16;
        arena.set_u16(parameters.wrapping_add((i as u32 + 11) * 2), value);
        i += 1;
    }
    let position = arena.u32(state.wrapping_add(POSITION)) as i32;
    if position > 0 {
        let excitation = arena.u32(state.wrapping_add(EXCITATION));
        let leftover = arena.u32(state.wrapping_add(LEFTOVER));
        let output = arena.u32(state.wrapping_add(OUTPUT));
        if frame_length < position {
            host.call(
                arena,
                0x195ae8,
                &[excitation, leftover, (frame_length as u32) << 1],
            );
            host.call(
                arena,
                0x2dc4cc,
                &[state, excitation, output, frame_length as u32],
            );
            host.call(
                arena,
                0x195ae8,
                &[
                    leftover,
                    leftover.wrapping_add((frame_length as u32) << 1),
                    ((position - frame_length) as u32) << 1,
                ],
            );
        } else {
            host.call(
                arena,
                0x195ae8,
                &[excitation, leftover, (position as u32) << 1],
            );
            host.call(
                arena,
                0x2dc4cc,
                &[state, excitation, output, position as u32],
            );
        }
    }
    while (arena.u32(state.wrapping_add(POSITION)) as i32) < frame_length {
        let position = arena.u32(state.wrapping_add(POSITION)) as i32;
        let weight = (position as u32).wrapping_mul(reciprocal);
        let pitch = arena.u16(state.wrapping_add(PITCH)) as u32;
        let previous_pitch = arena.u16(state.wrapping_add(PREVIOUS_PITCH)) as u32;
        let current_pitch = if (arena.u8(state.wrapping_add(MODE)) as u32) < 3 {
            pitch
        } else {
            weight
                .wrapping_mul(pitch)
                .wrapping_add(0x10000u32.wrapping_sub(weight).wrapping_mul(previous_pitch))
                >> 16
        };
        let mut period = (current_pitch >> 6) as i32;
        let mut frequency = arena.u16(state.wrapping_add(FREQUENCY)) as u32;
        let min_period = arena.u32(state.wrapping_add(MIN_PERIOD)) as i32;
        let max_period = arena.u32(state.wrapping_add(MAX_PERIOD)) as i32;
        if period < min_period {
            period = min_period;
        }
        if period > max_period {
            period = max_period;
        }
        let previous_frequency = arena.u16(state.wrapping_add(PREVIOUS_FREQUENCY)) as u32;
        if frequency != 0 && previous_frequency != 0 {
            frequency = weight.wrapping_mul(frequency).wrapping_add(
                0x10000u32
                    .wrapping_sub(weight)
                    .wrapping_mul(previous_frequency),
            ) >> 16;
        }
        host.call(
            arena,
            0x2dcb84,
            &[
                state,
                period as u32,
                frequency,
                state.wrapping_add(BUFFER_TAG),
            ],
        );
        let mut end = position.wrapping_add(period);
        if end >= frame_length {
            end = frame_length;
        }
        host.call(
            arena,
            0x2dc4cc,
            &[
                state,
                arena.u32(state.wrapping_add(EXCITATION)),
                arena
                    .u32(state.wrapping_add(OUTPUT))
                    .wrapping_add((position as u32) << 1),
                (end - position) as u32,
            ],
        );
        if position.wrapping_add(period) > frame_length {
            let remaining = frame_length - position;
            host.call(
                arena,
                0x195ae8,
                &[
                    arena.u32(state.wrapping_add(LEFTOVER)),
                    arena
                        .u32(state.wrapping_add(EXCITATION))
                        .wrapping_add((remaining as u32) << 1),
                    ((period - remaining) as u32) << 1,
                ],
            );
        }
        arena.set_u32(
            state.wrapping_add(POSITION),
            position.wrapping_add(period) as u32,
        );
    }
    arena.set_u16(
        state.wrapping_add(PREVIOUS_FREQUENCY),
        arena.u16(state.wrapping_add(FREQUENCY)),
    );
    arena.set_u16(
        state.wrapping_add(PREVIOUS_ENERGY),
        arena.u16(state.wrapping_add(ENERGY)),
    );
    arena.set_u16(
        state.wrapping_add(PREVIOUS_PITCH),
        arena.u16(state.wrapping_add(PITCH)),
    );
    host.call(
        arena,
        0x195ae8,
        &[
            arena.u32(state.wrapping_add(PREVIOUS_COEFFICIENTS)),
            arena.u32(state.wrapping_add(NEXT_COEFFICIENTS)),
            count.wrapping_mul(4),
        ],
    );
    let position = arena.u32(state.wrapping_add(POSITION)) as i32;
    arena.set_u32(
        state.wrapping_add(POSITION),
        position.wrapping_sub(frame_length) as u32,
    );
    if arena.u32(state.wrapping_add(SUPPRESS_OUTPUT)) == 0 {
        let done = host.call(
            arena,
            0x2dda54,
            &[
                state,
                arena.u32(state.wrapping_add(OUTPUT)),
                frame_length as u32,
            ],
        );
        if done.0 != 0 {
            return (-1i32) as u32;
        }
    }
    0
}
