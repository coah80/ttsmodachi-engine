

use super::arena::{Arena, Host};

const POOL_ALLOC_ERROR: u32 = 0x84e0_200a;

const ROM_SENTINEL: u32 = 0xffff_ffff;

fn widened_product(value: i32, mode: u32, fractional_bits: u32) -> i32 {
    let low = (value as u16 as u32).wrapping_mul(mode);
    let high = (value >> 16).wrapping_mul((mode as u16 as i16) as i32);
    super::left(high, 16u32.wrapping_sub(fractional_bits)).wrapping_add(super::right(
        low.wrapping_add(super::left(1, fractional_bits.wrapping_sub(1)) as u32) as i32,
        fractional_bits,
    ))
}

pub fn allocate_table(
    arena: &mut Arena<'_>,
    host: &mut impl Host,
    heap: u32,
    rows: i32,
    columns: i32,
    out: u32,
) -> u32 {
    let table = host
        .call(arena, 0x2db1d0, &[heap, rows.wrapping_shl(2) as u32])
        .0;
    arena.set_u32(out, table);
    if table != 0 {
        let data = host
            .call(
                arena,
                0x2db1d0,
                &[heap, (rows.wrapping_mul(columns).wrapping_shl(2)) as u32],
            )
            .0;
        if data != 0 {
            let mut i = 0;
            while i < rows {
                arena.set_u32(
                    table.wrapping_add((i as u32).wrapping_mul(4)),
                    (data as i32).wrapping_add(i.wrapping_mul(columns) << 2) as u32,
                );
                i += 1;
            }
            return 0;
        }
    }
    POOL_ALLOC_ERROR
}

pub fn release_workspace(
    arena: &mut Arena<'_>,
    host: &mut impl Host,
    _context: u32,
    heap: u32,
) -> u32 {
    host.call(arena, 0x2db4a0, &[heap]).0
}

pub fn release_selector(arena: &mut Arena<'_>, host: &mut impl Host, state: u32) -> u32 {
    for word in [16u32, 17, 18, 20, 21, 19, 14, 13, 15] {
        arena.set_u32(state.wrapping_add(word.wrapping_mul(4)), 0);
    }
    host.call(arena, 0x2db4a0, &[state]);
    host.call(arena, 0x2db4a0, &[state.wrapping_add(24)]);
    0
}

pub fn release_filter(arena: &mut Arena<'_>, host: &mut impl Host, state: u32) -> u32 {
    let mut i = 0;
    while i < 6 {
        arena.set_u32(state.wrapping_add(0x2c).wrapping_add(i * 4), 0);
        i += 1;
    }
    host.call(arena, 0x2db4a0, &[state]);
    0
}

pub fn reset_workspace(
    arena: &mut Arena<'_>,
    host: &mut impl Host,
    first: u32,
    second: u32,
) -> u32 {
    if arena.u32(second.wrapping_add(0x50)) != 0 {
        host.call(arena, 0x2db4a0, &[second.wrapping_add(0x18)]);
        arena.set_u32(second.wrapping_add(0x4c), 0);
        arena.set_u32(second.wrapping_add(0x50), 0);
        arena.set_u32(second.wrapping_add(0x54), 0);
    }
    let mut i = 0;
    while i < 5 {
        arena.set_u32(first.wrapping_add(0x30).wrapping_add(i * 4), 0);
        i += 1;
    }
    for offset in [0x40u32, 0x48, 0x44, 0x38, 0x3c] {
        arena.set_u32(second.wrapping_add(offset), 0);
    }
    arena.set_u32(second.wrapping_add(0x34), 0);
    host.call(arena, 0x2db4a0, &[second]);
    0
}

#[allow(clippy::too_many_arguments)]
pub fn prepare_gain_tables(
    arena: &mut Arena<'_>,
    host: &mut impl Host,
    owner: u32,
    columns: i32,
    rows: i32,
    stride: i32,
    extra: u32,
    state: u32,
) -> u32 {
    if extra != 0 {
        arena.set_u32(state.wrapping_add(80), 0);
        arena.set_u32(state.wrapping_add(84), 0);
    }
    arena.set_u32(state.wrapping_add(64), 0);
    arena.set_u32(state.wrapping_add(72), 0);
    arena.set_u32(state.wrapping_add(68), 0);
    if arena.u32(state.wrapping_add(16)) != 0 {
        host.call(arena, 0x2db3c8, &[state]);
    } else {
        host.call(arena, 0x2db19c, &[state, owner, ROM_SENTINEL]);
    }
    let need = (columns
        .wrapping_mul(rows)
        .wrapping_add(columns)
        .wrapping_shl(2)) as u32;
    arena.set_u32(state.wrapping_add(12), need);
    arena.set_u32(state.wrapping_add(52), rows as u32);
    arena.set_u32(state.wrapping_add(56), columns as u32);
    arena.set_u32(state.wrapping_add(60), stride as u32);
    let mut status = host
        .call(
            arena,
            0x2a4104,
            &[state, columns as u32, rows as u32, state.wrapping_add(72)],
        )
        .0;
    if status < 0x8000_0000 && extra != 0 {
        if arena.u32(state.wrapping_add(40)) != 0 {
            host.call(arena, 0x2db3c8, &[state.wrapping_add(24)]);
        } else {
            host.call(
                arena,
                0x2db19c,
                &[state.wrapping_add(24), owner, ROM_SENTINEL],
            );
        }
        let need = (columns
            .wrapping_mul(stride)
            .wrapping_add(stride)
            .wrapping_shl(3)) as u32;
        arena.set_u32(state.wrapping_add(36), need);
        status = host
            .call(
                arena,
                0x2a4104,
                &[
                    state.wrapping_add(24),
                    stride as u32,
                    columns as u32,
                    state.wrapping_add(80),
                ],
            )
            .0;
        if status < 0x8000_0000 {
            status = host
                .call(
                    arena,
                    0x2a4104,
                    &[
                        state.wrapping_add(24),
                        stride as u32,
                        columns as u32,
                        state.wrapping_add(84),
                    ],
                )
                .0;
            if status < 0x8000_0000 {
                arena.set_u32(state.wrapping_add(76), 0);
            }
        }
    }
    status
}

pub fn prepare_input_tables(
    arena: &mut Arena<'_>,
    host: &mut impl Host,
    context: u32,
    source: u32,
    input: u32,
) -> u32 {
    let channels = arena.u32(source.wrapping_add(0x18)) as i32;
    let frames = arena.u32(source.wrapping_add(0x20)) as i32;
    arena.set_u32(source.wrapping_add(0x30), 0);
    arena.set_u32(source.wrapping_add(0x34), 0);
    if arena.u32(input.wrapping_add(0x28)) != 0 {
        host.call(arena, 0x2db3c8, &[input.wrapping_add(0x18)]);
    } else {
        host.call(
            arena,
            0x2db19c,
            &[input.wrapping_add(0x18), context, ROM_SENTINEL],
        );
    }
    let need = (frames
        .wrapping_mul(channels)
        .wrapping_add(channels)
        .wrapping_shl(3)) as u32;
    arena.set_u32(input.wrapping_add(0x24), need);
    let mut result = host
        .call(
            arena,
            0x2a4104,
            &[
                input.wrapping_add(0x18),
                channels as u32,
                frames as u32,
                input.wrapping_add(0x50),
            ],
        )
        .0;
    if result < 0x8000_0000 {
        result = host
            .call(
                arena,
                0x2a4104,
                &[
                    input.wrapping_add(0x18),
                    channels as u32,
                    frames as u32,
                    input.wrapping_add(0x54),
                ],
            )
            .0;
        if result < 0x8000_0000 {
            arena.set_u32(input.wrapping_add(0x3c), channels as u32);
            arena.set_u32(input.wrapping_add(0x38), frames as u32);
            arena.set_u32(input.wrapping_add(0x4c), 0);
            arena.set_u32(
                source.wrapping_add(0x30),
                arena.u32(input.wrapping_add(0x50)),
            );
            arena.set_u32(
                source.wrapping_add(0x34),
                arena.u32(input.wrapping_add(0x54)),
            );
        }
    }
    result
}

pub fn prepare_filter_rows(
    arena: &mut Arena<'_>,
    host: &mut impl Host,
    owner: u32,
    workspace: u32,
    state: u32,
    name: u32,
) -> u32 {
    arena.set_u32(state.wrapping_add(0x2c), 0);
    if arena.u32(state.wrapping_add(16)) != 0 {
        host.call(arena, 0x2db3c8, &[state]);
    } else {
        host.call(arena, 0x2db19c, &[state, owner, name]);
    }
    let bank = arena.u32(state.wrapping_add(0x28));
    arena.set_u32(
        state.wrapping_add(0x24),
        (arena.u32(bank.wrapping_add(0x30)) as i32)
            .wrapping_mul(2)
            .wrapping_add(1) as u32,
    );
    let streams = arena.u32(state.wrapping_add(0x18));
    arena.set_u32(state.wrapping_add(0x30), 0);
    arena.set_u32(state.wrapping_add(0x34), 0);
    arena.set_u32(state.wrapping_add(0x38), 0);
    arena.set_u32(state.wrapping_add(0x3c), 0);
    arena.set_u32(state.wrapping_add(0x40), 0);
    if streams != 0 && workspace != 0 {
        arena.set_u32(
            state.wrapping_add(0x30),
            arena.u32(workspace.wrapping_add(0x50)),
        );
        arena.set_u32(
            state.wrapping_add(0x34),
            arena.u32(workspace.wrapping_add(0x54)),
        );
        arena.set_u32(workspace.wrapping_add(0x4c), 1);
        arena.set_u32(
            state.wrapping_add(0x3c),
            arena.u32(workspace.wrapping_add(0x48)),
        );
    }
    let frames = arena.u32(state.wrapping_add(0x20)) as i32;
    let last = arena.u32(state.wrapping_add(0x1c)) as i32;
    if last != 0 {
        let stride = last.wrapping_add(1);
        arena.set_u32(
            state.wrapping_add(12),
            (frames.wrapping_mul(stride).wrapping_add(frames) << 2) as u32,
        );
        let rows = host.call(arena, 0x2db1d0, &[state, (frames << 2) as u32]).0;
        if rows == 0 {
            return POOL_ALLOC_ERROR;
        }
        let data = host
            .call(
                arena,
                0x2db1d0,
                &[state, (frames.wrapping_mul(stride) << 2) as u32],
            )
            .0;
        if data == 0 {
            return POOL_ALLOC_ERROR;
        }
        let mut i = 0;
        while i < frames {
            arena.set_u32(
                (rows as i32).wrapping_add(i << 2) as u32,
                (data as i32).wrapping_add(i.wrapping_mul(stride) << 2) as u32,
            );
            i += 1;
        }
        arena.set_u32(state.wrapping_add(0x2c), rows);
    } else {
        arena.set_u32(state.wrapping_add(12), (frames << 2) as u32);
        let data = host.call(arena, 0x2db1d0, &[state, (frames << 2) as u32]).0;
        if data == 0 {
            return POOL_ALLOC_ERROR;
        }
        arena.set_u32(state.wrapping_add(0x2c), data);
    }
    0
}

#[allow(clippy::too_many_arguments)]
pub fn select_voiced(
    arena: &mut Arena<'_>,
    host: &mut impl Host,
    engine: u32,
    block: u32,
    row: u32,
    flags: u32,
    model: u32,
    threshold: i32,
    scratch: u32,
) -> u32 {
    let mut i = 1;
    let mut result = 0;
    let count = arena.u32(model.wrapping_add(0xc0)) as i32;
    if 1 < count + 1 {
        loop {
            let frame = arena.u32(flags.wrapping_add(8));
            let start = arena.u32(flags.wrapping_add(4));
            result = host
                .call(
                    arena,
                    0x2f850c,
                    &[
                        engine,
                        model.wrapping_add(0xb4),
                        (block as i32).wrapping_sub(2) as u32,
                        (row as i32).wrapping_sub(1) as u32,
                        (i - 1) as u32,
                        (start as i32).wrapping_add(i << 2) as u32,
                        (frame as i32).wrapping_add(i << 2) as u32,
                        scratch,
                        20,
                    ],
                )
                .0;
            if result >= 0x8000_0000 {
                return result;
            }
            if i == 1 {
                let current = arena.u8(flags);
                if arena.u32(scratch) as i32 > threshold {
                    arena.set_u8(flags, current | 2);
                } else {
                    arena.set_u8(flags, current & !2);
                }
            }
            i += 1;
            if !(i < count + 1) {
                break;
            }
        }
    }
    result
}

#[allow(clippy::too_many_arguments)]
pub fn select_spectral(
    arena: &mut Arena<'_>,
    host: &mut impl Host,
    engine: u32,
    block: u32,
    row: u32,
    flags: u32,
    model: u32,
    threshold: i32,
    scratch: u32,
) -> u32 {
    let mut i = 1;
    let mut result = 0;
    let count = arena.u32(model.wrapping_add(0x20)) as i32;
    if count >= 1 {
        loop {
            let frame = arena.u32(flags.wrapping_add(8));
            let start = arena.u32(flags.wrapping_add(4));
            result = host
                .call(
                    arena,
                    0x2f850c,
                    &[
                        engine,
                        model.wrapping_add(0x12c),
                        (block as i32).wrapping_sub(2) as u32,
                        (row as i32).wrapping_sub(1) as u32,
                        (i - 1) as u32,
                        (start as i32).wrapping_add(i << 2) as u32,
                        (frame as i32).wrapping_add(i << 2) as u32,
                        scratch,
                        18,
                    ],
                )
                .0;
            if result >= 0x8000_0000 {
                return result;
            }
            if i == 1 {
                let current = arena.u8(flags);
                if arena.u32(scratch) as i32 > threshold {
                    arena.set_u8(flags, current | 4);
                } else {
                    arena.set_u8(flags, current & !4);
                }
            }
            i += 1;
            if !(i <= count) {
                break;
            }
        }
    }
    result
}

#[allow(clippy::too_many_arguments)]
pub fn select_band(
    arena: &mut Arena<'_>,
    host: &mut impl Host,
    engine: u32,
    block: u32,
    row: u32,
    count: i32,
    output: u32,
    directory: u32,
    threshold: i32,
    scratch: u32,
) -> u32 {
    let mut result = 0;
    let mut i = 1;
    if count >= 1 {
        loop {
            result = host
                .call(
                    arena,
                    0x2f86e0,
                    &[
                        engine,
                        directory.wrapping_add(300),
                        (block as i32).wrapping_sub(2) as u32,
                        (row as i32).wrapping_sub(1) as u32,
                        (i - 1) as u32,
                        arena
                            .u32(output.wrapping_add(4))
                            .wrapping_add((i as u32) << 2),
                        scratch,
                        18,
                    ],
                )
                .0;
            if result >= 0x8000_0000 {
                return result;
            }
            if i == 1 {
                let current = arena.u8(output);
                if arena.u32(scratch) as i32 > threshold {
                    arena.set_u8(output, current | 4);
                } else {
                    arena.set_u8(output, current & !4);
                }
            }
            i += 1;
            if !(i <= count) {
                break;
            }
        }
    }
    result
}

#[allow(clippy::too_many_arguments)]
pub fn read_durations(
    arena: &mut Arena<'_>,
    host: &mut impl Host,
    context: u32,
    state: u32,
    row: u32,
    source: u32,
    adjustment: i32,
    scratch: u32,
) -> u32 {
    let heap = arena.u32(state);
    let count = arena.u32(source.wrapping_add(24)) as i32;
    let values = host.call(arena, 0x2db1d0, &[heap, (count << 1) as u32]).0;
    arena.set_u32(state.wrapping_add(12), values);
    if values == 0 {
        return POOL_ALLOC_ERROR;
    }
    arena.set_u32(state.wrapping_add(12), values.wrapping_sub(4));
    arena.set_u32(state.wrapping_add(16), 0);
    let mut status = 0;
    let mut i = 2;
    while i < count + 2 {
        status = host
            .call(
                arena,
                0x2f7f90,
                &[
                    context,
                    source.wrapping_add(40),
                    (row as i32).wrapping_sub(1) as u32,
                    (i - 2) as u32,
                    scratch,
                    scratch.wrapping_add(4),
                    15,
                ],
            )
            .0;
        if status >= 0x8000_0000 {
            return status;
        }
        let first = arena.u32(scratch) as i32;
        let second = arena.u32(scratch.wrapping_add(4)) as i32;
        let mut value = first;
        if adjustment != 0 {
            value = super::add_i32(value, widened_product(second, adjustment as u32, 15));
        }
        if value < 0 {
            value = 0;
        } else if value > 0x7d0_0000 {
            value = 0x7d0_0000;
        }
        let rounded = (((value << 1) as u32).wrapping_add(0x8000) >> 16) as u32;
        let slot = values.wrapping_sub(4).wrapping_add((i as u32) << 1);
        arena.set_u16(slot, rounded as u16);
        let total = arena.u32(state.wrapping_add(16));
        arena.set_u32(state.wrapping_add(16), total.wrapping_add(rounded));
        i += 1;
    }
    status
}

#[allow(clippy::too_many_arguments)]
pub fn read_frame_params(
    arena: &mut Arena<'_>,
    host: &mut impl Host,
    engine: u32,
    position: u32,
    selected: u32,
    params: u32,
    model: u32,
    threshold: i32,
    mode: u32,
    scratch: u32,
) -> u32 {
    let mut i = 1;
    let mut result = 0;
    let count = arena.u32(model.wrapping_add(0xc0)) as i32;
    if 1 < count + 1 {
        loop {
            result = host
                .call(
                    arena,
                    0x2f850c,
                    &[
                        engine,
                        model.wrapping_add(0xb4),
                        (position as i32).wrapping_sub(2) as u32,
                        (selected as i32).wrapping_sub(1) as u32,
                        (i - 1) as u32,
                        scratch,
                        scratch.wrapping_add(4),
                        scratch.wrapping_add(8),
                        20,
                    ],
                )
                .0;
            if result >= 0x8000_0000 {
                return result;
            }
            let mean = arena.u32(scratch) as i32;
            let variance = arena.u32(scratch.wrapping_add(4)) as i32;
            let means = arena.u32(params.wrapping_add(4));
            let variances = arena.u32(params.wrapping_add(8));
            arena.set_u32(means.wrapping_add((i as u32) << 2), mean as u32);
            arena.set_u32(variances.wrapping_add((i as u32) << 2), variance as u32);
            let adjusted = super::add_i32(mean, widened_product(variance, mode, 13));
            arena.set_u32(means.wrapping_add((i as u32) << 2), adjusted as u32);
            if i == 1 {
                let current = arena.u8(params);
                if arena.u32(scratch.wrapping_add(8)) as i32 > threshold {
                    arena.set_u8(params, current | 2);
                } else {
                    arena.set_u8(params, current & !2);
                }
            }
            i += 1;
            if !(i < count + 1) {
                break;
            }
        }
    }
    result
}

pub fn run_forward_stream(
    arena: &mut Arena<'_>,
    host: &mut impl Host,
    context: u32,
    state: u32,
) -> u32 {
    arena.set_u32(
        state.wrapping_add(0x38),
        arena.u32(arena.u32(state.wrapping_add(0x30))),
    );
    arena.set_u32(
        state.wrapping_add(0x40),
        arena.u32(state.wrapping_add(0x2c)),
    );
    host.call(arena, 0x2dacb4, &[state]);
    host.call(arena, 0x2a1f50, &[context, state]);
    host.call(arena, 0x2a2224, &[state, 16, 16, 20]);
    host.call(arena, 0x2a3fe4, &[state]);
    0
}

pub fn run_backward_stream(
    arena: &mut Arena<'_>,
    host: &mut impl Host,
    context: u32,
    state: u32,
) -> u32 {
    arena.set_u32(
        state.wrapping_add(0x38),
        arena.u32(arena.u32(state.wrapping_add(0x30))),
    );
    arena.set_u32(
        state.wrapping_add(0x40),
        arena.u32(state.wrapping_add(0x2c)),
    );
    host.call(arena, 0x2da9e0, &[state]);
    host.call(arena, 0x2a1c54, &[context, state, 18]);
    host.call(arena, 0x2a3d84, &[state]);
    host.call(arena, 0x2a23c4, &[state, 28, 22, 18]);
    0
}

pub fn default_voice_config(arena: &mut Arena<'_>, config: u32) -> u32 {
    arena.set_u16(config, 0);
    arena.set_u16(config.wrapping_add(2), 0);
    arena.set_u32(config.wrapping_add(4), 0x6b8);
    arena.set_u32(config.wrapping_add(8), 20);
    arena.set_u32(config.wrapping_add(12), 99);
    arena.set_u32(config.wrapping_add(20), 0x99a);
    arena.set_u32(config.wrapping_add(24), 100);
    arena.set_u32(config.wrapping_add(28), 100);
    arena.set_u32(config.wrapping_add(32), 0x80000);
    arena.set_u32(config.wrapping_add(36), 0x20000);
    arena.set_u32(config.wrapping_add(40), 0);
    arena.set_u32(config.wrapping_add(44), 0);
    arena.set_u32(config.wrapping_add(48), 100);
    arena.set_u32(config.wrapping_add(52), 16000);
    arena.set_u32(config.wrapping_add(56), 4);
    arena.set_u32(config.wrapping_add(60), 6);
    arena.set_u8(config.wrapping_add(0x40), 0);
    arena.set_u8(config.wrapping_add(0x41), 50);
    arena.set_u8(config.wrapping_add(0x42), 115);
    arena.set_u8(config.wrapping_add(0x43), 0);
    arena.set_u8(config.wrapping_add(0x44), 0);
    arena.set_u8(config.wrapping_add(0x45), 0);
    arena.set_u8(config.wrapping_add(0x46), 0);
    arena.set_u8(config.wrapping_add(0x47), 0);
    arena.set_u8(config.wrapping_add(0x48), 0);
    arena.set_u8(config.wrapping_add(0x49), 1);
    arena.set_u32(config.wrapping_add(0x4c), 800);
    arena.set_u32(config.wrapping_add(0x50), 10);
    arena.set_u32(config.wrapping_add(0x54), 5);
    if arena.u32(config.wrapping_add(0x50)) < arena.u32(config.wrapping_add(0x54)) {
        arena.set_u32(
            config.wrapping_add(0x54),
            arena.u32(config.wrapping_add(0x50)),
        );
    }
    arena.set_u32(config.wrapping_add(0x58), 3);
    if arena.u32(config.wrapping_add(0x50)) < arena.u32(config.wrapping_add(0x58)) {
        arena.set_u32(
            config.wrapping_add(0x58),
            arena.u32(config.wrapping_add(0x50)),
        );
    }
    arena.set_u8(config.wrapping_add(0x5c), 0);
    arena.set_u8(config.wrapping_add(0x111), 3);
    arena.set_u8(config.wrapping_add(0x112), 2);
    arena.set_u8(config.wrapping_add(0x113), 3);
    arena.set_u8(config.wrapping_add(0x114), 3);
    0
}

pub fn release_frame_lookup(arena: &mut Arena<'_>, host: &mut impl Host, state: u32) -> u32 {
    let mut result = 0;
    if state != 0 && arena.u32(state.wrapping_add(12)) != 0 {
        result = host
            .call(
                arena,
                0x179208,
                &[
                    arena.u32(state),
                    arena.u32(state.wrapping_add(4)),
                    arena.u32(state.wrapping_add(12)),
                ],
            )
            .0;
        arena.set_u32(state.wrapping_add(8), 0);
        arena.set_u32(state.wrapping_add(12), 0);
        arena.set_u32(state.wrapping_add(16), 0);
    }
    result
}

pub fn read_voice_entry(
    arena: &mut Arena<'_>,
    reader: u32,
    data: u32,
    size: u32,
    offset: u32,
    destination: u32,
    capacity: u32,
) -> u32 {
    if reader == 0
        || data == 0
        || offset.wrapping_add(2) > size
        || capacity == 0
        || (arena.u32(capacity) != 0 && destination == 0)
    {
        return 0x8a20_2007;
    }
    let source = data.wrapping_add(offset);
    let length = arena.u16(source) as u32;
    if offset.wrapping_add(length).wrapping_add(2) > size {
        arena.set_u32(capacity, length.wrapping_add(1));
        return 0x8a20_2004;
    }
    if length.wrapping_add(1) > arena.u32(capacity) {
        if arena.u32(capacity) != 0 {
            arena.set_u8(destination, 0);
        }
        arena.set_u32(capacity, length.wrapping_add(1));
        return 0x8a20_2009;
    }
    let text = source.wrapping_add(2);
    let mut i = 0;
    while i < length {
        arena.set_u8(destination.wrapping_add(i), arena.u8(text.wrapping_add(i)));
        i = i.wrapping_add(1);
    }
    arena.set_u8(destination.wrapping_add(length), 0);
    if arena.u8(reader.wrapping_add(13)) == 1 {
        let mut i = 0;
        while i < length {
            let byte = arena.u8(destination.wrapping_add(i));
            arena.set_u8(destination.wrapping_add(i), byte ^ 0xa9);
            i = i.wrapping_add(1);
        }
    }
    arena.set_u32(capacity, length.wrapping_add(1));
    0
}

#[allow(clippy::too_many_arguments)]
pub fn open_voice_resource(
    arena: &mut Arena<'_>,
    host: &mut impl Host,
    object: u32,
    key: u32,
    mode: u32,
    path: u32,
    resource_type: u32,
    version: u32,
    output: u32,
) -> u32 {
    host.call(
        arena,
        0x242fd0,
        &[
            object,
            key,
            mode,
            path,
            0,
            0,
            resource_type,
            version,
            output,
        ],
    )
    .0
}

pub fn release_shared_stream(arena: &mut Arena<'_>, host: &mut impl Host, state: u32) -> u32 {
    if arena.u16(state.wrapping_add(0x1c)) != 0 {
        arena.set_u16(
            state.wrapping_add(0x1c),
            arena.u16(state.wrapping_add(0x1c)).wrapping_sub(1),
        );
        if arena.u16(state.wrapping_add(0x1c)) == 0 {
            let mut i = 0;
            while i < 5 {
                release_filter(
                    arena,
                    host,
                    state.wrapping_add(0x26c).wrapping_add(i * 0x48),
                );
                i += 1;
            }
            let allocator = arena.u32(state.wrapping_add(0x18));
            let heap = arena.u32(allocator.wrapping_add(4));
            for slot in [0x430u32, 0x42c] {
                if arena.u32(state.wrapping_add(slot)) != 0 {
                    host.call(
                        arena,
                        0x196428,
                        &[heap, arena.u32(state.wrapping_add(slot))],
                    );
                    arena.set_u32(state.wrapping_add(slot), 0);
                }
            }
            host.call(arena, 0x2db4a0, &[state]);
            host.call(arena, 0x2db4a0, &[state.wrapping_add(0x24)]);
        }
    }
    0
}

pub fn finish_selection(
    arena: &mut Arena<'_>,
    host: &mut impl Host,
    state: u32,
    argument: u32,
) -> u32 {
    host.call(arena, 0x2a3740, &[state.wrapping_add(0x498)]);
    host.call(arena, 0x2a3740, &[state.wrapping_add(0x4e0)]);
    host.call(arena, 0x2a3414, &[state.wrapping_add(0x3d4)]);
    host.call(arena, 0x2db4a0, &[state]);
    if arena.u32(argument.wrapping_add(0x2c)) > 1 {
        let context = arena.u32(state.wrapping_add(0x18));
        host.call(
            arena,
            0x1738e8,
            &[
                arena.u32(context.wrapping_add(0x10)),
                ROM_SENTINEL,
                4,
                0,
                ROM_SENTINEL,
            ],
        );
    }
    0
}
