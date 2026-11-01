

use super::arena::{Arena, Host};
use super::arena_leaf;
use super::arena_mid;
use super::{DecisionNode, DecisionTree, SelectionFeatures};

const ROM_SENTINEL: u32 = 0xffff_ffff;

const ENG_CTX: u32 = 0x08;
const ENG_PIPE: u32 = 0x0c;
const ENG_F10: u32 = 0x10;
const ENG_CFG: u32 = 0x94;
const ENG_MODEL: u32 = 0x1ac;
const ENG_BLOCKS: u32 = 0x638;
const ENG_F73C: u32 = 0x73c;
const ENG_WAIT: u32 = 0x744;
const ENG_F78C: u32 = 0x78c;
const ENG_IN: u32 = 0x7a0;
const ENG_MARKERS_IN: u32 = 0x7a8;
const ENG_FRAMES_OUT: u32 = 0x7b0;
const ENG_MARKERS_OUT: u32 = 0x7b8;
const ENG_PROF: u32 = 0x7c0;
const ENG_PROFH: u32 = 0x7c4;
const ENG_SPTCHK: u32 = 0x7c8;
const ENG_SGT: u32 = 0x7d0;
const ENG_SYNTH: u32 = 0x7d8;
const ENG_MARKERS: u32 = 0x7dc;
const ENG_MARKER_COUNT: u32 = 0x7e0;
const ENG_MARKER_DONE: u32 = 0x7e4;
const ENG_MARKER_BYTES: u32 = 0x7e8;
const ENG_F7EC: u32 = 0x7ec;
const ENG_F7F0: u32 = 0x7f0;
const ENG_STATE: u32 = 0x7f4;

const PIPE_CLOSE: u32 = 0x24;
const PIPE_READ: u32 = 0x2c;
const PIPE_CONSUME: u32 = 0x30;
const PIPE_WRITE: u32 = 0x34;
const PIPE_AVAIL: u32 = 0x48;

const SYN_F20: u32 = 0x20;
const SYN_ARENA: u32 = 0x24;
const SYN_FEATURES: u32 = 0x3c;
const SYN_CUR: u32 = 0x564;
const SYN_POS: u32 = 0x568;

pub fn teardown_voice_object(arena: &mut Arena<'_>, host: &mut impl Host, object: u32) -> u32 {
    let mut rc = 0;
    if object != 0 {
        let heap = arena.u32(arena.u32(object).wrapping_add(4));
        let array = arena.u32(object.wrapping_add(0x888));
        if array != 0 {
            let mut i = 0u16;
            while arena.u32(array.wrapping_add((i as u32).wrapping_mul(4))) != 0 {
                host.call(
                    arena,
                    0x196428,
                    &[
                        heap,
                        arena.u32(array.wrapping_add((i as u32).wrapping_mul(4))),
                    ],
                );
                i = i.wrapping_add(1);
            }
            host.call(arena, 0x196428, &[heap, array]);
            arena.set_u32(object.wrapping_add(0x888), 0);
        }
        let sub = arena.u32(object.wrapping_add(0x3b54));
        if sub != 0 {
            rc = host.call(arena, 0x2412a0, &[sub]).0;
            arena.set_u32(object.wrapping_add(0x3b54), 0);
        }
        let mut i = 0u16;
        while i < arena.u16(object.wrapping_add(0x140)) {
            host.call(
                arena,
                0x243d50,
                &[arena.u32(
                    object
                        .wrapping_add(0xe0)
                        .wrapping_add((i as u32).wrapping_mul(4)),
                )],
            );
            i = i.wrapping_add(1);
        }
        arena.set_u16(object.wrapping_add(0x140), 0);
        host.call(arena, 0x196428, &[heap, object]);
    }
    rc
}

pub fn step_models(
    arena: &mut Arena<'_>,
    host: &mut impl Host,
    context: u32,
    a: u32,
    b: u32,
    out: u32,
) -> u32 {
    let mut sum = 0i32;
    let mut count = 0i32;
    let list = context.wrapping_add(0x440);
    let mut previous = 0u32;
    let head = arena.u32(list);
    arena.set_u32(list.wrapping_add(4), arena.u32(context.wrapping_add(0x460)));
    let mut node = head;
    arena.set_u8(
        node.wrapping_add(0x14),
        arena.u8(node.wrapping_add(0x14)) & !1,
    );
    while arena.u8(node.wrapping_add(0x14)) & 1 == 0 && arena.u32(list.wrapping_add(4)) != node {
        count = count.wrapping_add(1);
        sum = sum.wrapping_add(arena.u32(node.wrapping_add(0x10)) as i32);
        previous = node;
        node = arena.u32(node.wrapping_add(0x18));
    }
    let sub = arena.u32(context.wrapping_add(0x18));
    host.call(
        arena,
        0x1738e8,
        &[
            arena.u32(sub.wrapping_add(0x10)),
            ROM_SENTINEL,
            4,
            0,
            ROM_SENTINEL,
            count as u32,
            sum as u32,
        ],
    );
    let mut result;
    if previous == 0 {
        result = 0;
    } else {
        arena.set_u8(
            out,
            u32::from(arena.u32(list.wrapping_add(4)) == node) as u8,
        );
        arena.set_u32(list.wrapping_add(20), sum as u32);
        arena.set_u32(list.wrapping_add(4), node);
        arena.set_u32(list.wrapping_add(8), count as u32);
        if arena.u32(context.wrapping_add(0x55c)) != 0 {
            host.call(
                arena,
                0x1985fc,
                &[
                    arena.u32(context.wrapping_add(0x55c)),
                    arena.u32(context.wrapping_add(0x560)),
                    4,
                ],
            );
        }
        result = host.call(arena, 0x2a8414, &[context, a, b]).0 as i32;
        if arena.u32(context.wrapping_add(0x55c)) != 0 {
            host.call(
                arena,
                0x19860c,
                &[
                    arena.u32(context.wrapping_add(0x55c)),
                    arena.u32(context.wrapping_add(0x560)),
                    4,
                ],
            );
        }
        if result == 0 {
            if arena.u32(context.wrapping_add(0x55c)) != 0 {
                host.call(
                    arena,
                    0x1985fc,
                    &[
                        arena.u32(context.wrapping_add(0x55c)),
                        arena.u32(context.wrapping_add(0x560)),
                        3,
                    ],
                );
            }
            result = host.call(arena, 0x2a5578, &[context, a, b]).0 as i32;
            if arena.u32(context.wrapping_add(0x55c)) != 0 {
                host.call(
                    arena,
                    0x19860c,
                    &[
                        arena.u32(context.wrapping_add(0x55c)),
                        arena.u32(context.wrapping_add(0x560)),
                        3,
                    ],
                );
            }
            if result == 0 {
                result = host.call(arena, 0x2a7740, &[context, a, b]).0 as i32;
                if result == 0 {
                    if arena.u32(list.wrapping_add(4)) == arena.u32(context.wrapping_add(0x460)) {
                        arena.set_u32(context.wrapping_add(0x3a9c), 0);
                    } else {
                        let accumulated = (arena.u32(list.wrapping_add(20)) as i32)
                            .wrapping_add(arena.u32(context.wrapping_add(0x3a9c)) as i32);
                        let max = arena.u32(context.wrapping_add(0x3a90)) as i32;
                        arena.set_u32(
                            context.wrapping_add(0x3a9c),
                            (if accumulated > max { max } else { accumulated }) as u32,
                        );
                    }
                }
            }
        }
    }
    arena.set_u32(context.wrapping_add(0x494), head);
    arena.set_u32(context.wrapping_add(0x434), 0);
    arena.set_u32(context.wrapping_add(0x438), 0);
    arena.set_u32(context.wrapping_add(0x440), node);
    arena.set_u32(context.wrapping_add(0x43c), 0);
    arena.set_u32(list.wrapping_add(4), node);
    result as u32
}

fn select_duration_row(
    arena: &Arena<'_>,
    state: u32,
    trees: u32,
    position: usize,
    nodes: &mut [DecisionNode],
) -> u16 {
    let env = state.wrapping_add(0x3c);
    let count = arena.u16(env.wrapping_add(6)) as usize;
    let table = arena.u32(trees.wrapping_add(8).wrapping_add(36));
    let raw_nodes = arena.u32(table.wrapping_add(4));
    let node_count = arena.u16(table.wrapping_add(2)) as usize;
    let raw = arena.slice(raw_nodes, node_count.wrapping_mul(6));
    let parsed = super::parse_decision_nodes(raw, nodes).expect("decision table fits");
    debug_assert_eq!(parsed, node_count);
    let questions_base = arena.u32(trees.wrapping_add(8).wrapping_add(16));
    let mut need = 0usize;
    for node in &nodes[..parsed] {
        let base = node.question as usize;
        let selector = arena.slice(questions_base, base.wrapping_add(2))[base.wrapping_add(1)];
        let extent = base.wrapping_add(2).wrapping_add(match selector {
            1 => 1,
            255 => 2,
            other => other as usize,
        });
        if extent > need {
            need = extent;
        }
    }
    let tree = DecisionTree {
        questions: arena.slice(questions_base, need),
        nodes: &nodes[..parsed],
    };
    let mut columns: [Option<&[u8]>; 60] = [None; 60];
    for (i, column) in columns.iter_mut().enumerate() {
        let pointer = arena.u32(
            env.wrapping_add(12)
                .wrapping_add((i as u32).wrapping_mul(4)),
        );
        if pointer != 0 {
            *column = Some(arena.slice(pointer, count));
        }
    }
    tree.select(SelectionFeatures { count, columns }, position)
        .expect("decision table resolves")
}

#[allow(clippy::too_many_arguments)]
pub fn build_models(
    arena: &mut Arena<'_>,
    host: &mut impl Host,
    state: u32,
    voice: u32,
    settings: u32,
    nodes: &mut [DecisionNode],
    scratch: u32,
) -> u32 {
    let heap = state;
    let mut status = 0u32;
    let mut index = 0i32;
    let list = state.wrapping_add(0x440);
    let mut node = state.wrapping_add(0x478);
    let context = arena.u32(state.wrapping_add(0x18));
    let log = arena.u32(context.wrapping_add(16));
    let engine = arena.u32(state.wrapping_add(0x20));
    let voice_states = arena.u32(voice.wrapping_add(0x1f8)) as i32;
    let verbosity = arena.u32(settings.wrapping_add(0x2c)) as i32;
    let frame_period = arena.u32(settings.wrapping_add(0x38)) as i32;
    let target_states = arena.u32(settings.wrapping_add(0x28)) as i32;
    host.call(arena, 0x2db19c, &[heap, context, ROM_SENTINEL]);
    arena.set_u32(state.wrapping_add(0xc), 6000);
    arena.set_u8(list.wrapping_add(24), 0);
    arena.set_u8(list.wrapping_add(25), 0);
    arena.set_u32(node, heap);
    arena.set_u32(list, node);
    arena.set_u32(list.wrapping_add(4), node);
    arena.set_u16(state.wrapping_add(0x268), arena.u16(settings));
    let mut last_timed = 0u8;
    let count = arena.u16(state.wrapping_add(0x42)) as i16 as i32;
    if count > 0 {
        loop {
            last_timed = 0;
            let timings = arena.u32(state.wrapping_add(0x144));
            arena.set_u16(node.wrapping_add(4), index as u16);
            let mut target = 0i32;
            if timings != 0 {
                target = arena.u16(timings.wrapping_add((index as u32).wrapping_mul(2))) as i32;
            }
            if timings != 0 && arena.u8(settings.wrapping_add(0x49)) != 0 && target > 0 {
                let mut total = 0u32;
                let mut preserve = 0i32;
                let mut fixed = 0u32;
                let row = select_duration_row(
                    arena,
                    state,
                    voice.wrapping_add(0x440),
                    index as usize,
                    nodes,
                );
                if arena.u32(list) == node {
                    arena.set_u8(list.wrapping_add(24), 1);
                }
                last_timed = 1;
                let starts = arena.u32(state.wrapping_add(0x50));
                if index == 0 || arena.u8(starts.wrapping_add(index as u32)) == 1 {
                    preserve = 1;
                }
                status = arena_mid::read_durations(
                    arena,
                    host,
                    engine,
                    node,
                    u32::from(row),
                    voice.wrapping_add(0x1e0),
                    arena.u16(state.wrapping_add(0x268)) as i16 as i32,
                    scratch,
                );
                if status >= 0x8000_0000 {
                    break;
                }
                let durations = arena.u32(node.wrapping_add(12));
                let mut i = 2i32;
                if preserve != 0 {
                    i = 3;
                    fixed = arena.u16(durations.wrapping_add(4)) as u32;
                }
                while i < voice_states.wrapping_add(1) {
                    total = total.wrapping_add(
                        arena.u16(durations.wrapping_add((i as u32).wrapping_mul(2))) as u32,
                    );
                    i = i.wrapping_add(1);
                }
                if preserve != 0 {
                    fixed = fixed.wrapping_add(
                        arena.u16(durations.wrapping_add((i as u32).wrapping_mul(2))) as u32,
                    );
                } else {
                    total = total.wrapping_add(
                        arena.u16(durations.wrapping_add((i as u32).wrapping_mul(2))) as u32,
                    );
                }
                if verbosity > 10 {
                    host.call(
                        arena,
                        0x1738e8,
                        &[
                            log,
                            ROM_SENTINEL,
                            4,
                            0,
                            ROM_SENTINEL,
                            total.wrapping_add(fixed),
                            target as u32,
                        ],
                    );
                }
                let scaled =
                    (frame_period as u32).wrapping_mul(fixed.wrapping_add(total).wrapping_mul(10));
                if (scaled as i32) > target {
                    preserve = 0;
                    total = total.wrapping_add(fixed);
                } else {
                    target = ((target as u32)
                        .wrapping_sub((frame_period as u32).wrapping_mul(fixed).wrapping_mul(10)))
                        as i32;
                }
                let mut stretch;
                if total == 0 {
                    stretch = 0u32;
                } else {
                    if target > 0x989680 {
                        target = 0x989680;
                    }
                    stretch = crate::runtime::signed_division(
                        target.wrapping_mul(205),
                        (frame_period as u32).wrapping_mul(total) as i32,
                    )
                    .expect("nonzero frame total")
                    .quotient as u32;
                    if stretch > 0x7fffff {
                        host.call(arena, 0x1738e8, &[log, ROM_SENTINEL, 3, 0, ROM_SENTINEL]);
                        stretch = 0x7fffff;
                    }
                }
                let mut i = 2i32;
                let mut node_duration = 0i32;
                if preserve != 0 {
                    i = 3;
                    node_duration = arena.u16(durations.wrapping_add(4)) as i32;
                }
                while i < voice_states.wrapping_add(2) {
                    if preserve == 0 || i < voice_states.wrapping_add(1) {
                        let mut duration =
                            (arena.u16(durations.wrapping_add((i as u32).wrapping_mul(2))) as u32)
                                .wrapping_mul(stretch) as i32;
                        if duration >= 0 {
                            duration = duration.wrapping_add(0x400) >> 11;
                            if duration == 0 {
                                duration = 1;
                            }
                        } else {
                            host.call(arena, 0x1738e8, &[log, ROM_SENTINEL, 3, 1, ROM_SENTINEL]);
                            duration = 0xfffff;
                        }
                        if verbosity >= 20 {
                            host.call(
                                arena,
                                0x1738e8,
                                &[
                                    log,
                                    ROM_SENTINEL,
                                    4,
                                    0,
                                    ROM_SENTINEL,
                                    i as u32,
                                    arena.u16(durations.wrapping_add((i as u32).wrapping_mul(2)))
                                        as u32,
                                    duration as u32,
                                ],
                            );
                        }
                        arena.set_u16(
                            durations.wrapping_add((i as u32).wrapping_mul(2)),
                            if duration < 0xffff {
                                duration as u16
                            } else {
                                0xffff
                            },
                        );
                    }
                    node_duration = node_duration.wrapping_add(
                        arena.u16(durations.wrapping_add((i as u32).wrapping_mul(2))) as i32,
                    );
                    i = i.wrapping_add(1);
                }
                if verbosity > 20 {
                    host.call(
                        arena,
                        0x1738e8,
                        &[log, ROM_SENTINEL, 4, 0, ROM_SENTINEL, node_duration as u32],
                    );
                }
                arena.set_u32(node.wrapping_add(16), node_duration as u32);
                arena.set_u32(
                    list.wrapping_add(16),
                    arena
                        .u32(list.wrapping_add(16))
                        .wrapping_add(arena.u32(node.wrapping_add(16))),
                );
            } else {
                let row = select_duration_row(
                    arena,
                    state,
                    voice.wrapping_add(0x440),
                    index as usize,
                    nodes,
                );
                if verbosity > 5 {
                    host.call(
                        arena,
                        0x1738e8,
                        &[log, ROM_SENTINEL, 4, 0, ROM_SENTINEL, u32::from(row)],
                    );
                }
                if target_states == 0 {
                    status = arena_mid::read_durations(
                        arena,
                        host,
                        engine,
                        node,
                        u32::from(row),
                        voice.wrapping_add(0x1e0),
                        arena.u16(state.wrapping_add(0x268)) as i16 as i32,
                        scratch,
                    );
                    if status >= 0x8000_0000 {
                        break;
                    }
                    if verbosity > 20 {
                        host.call(
                            arena,
                            0x1738e8,
                            &[
                                log,
                                ROM_SENTINEL,
                                4,
                                0,
                                ROM_SENTINEL,
                                arena.u32(node.wrapping_add(16)),
                            ],
                        );
                    }
                    arena.set_u32(
                        list.wrapping_add(16),
                        arena
                            .u32(list.wrapping_add(16))
                            .wrapping_add(arena.u32(node.wrapping_add(16))),
                    );
                }
            }
            let next = host.call(arena, 0x2db1d0, &[heap, 28]).0;
            if next == 0 {
                status = 0x84e0_200a;
                break;
            }
            arena.set_u32(next, heap);
            arena.set_u32(node.wrapping_add(24), next);
            node = next;
            arena.set_u32(list.wrapping_add(4), node);
            index = ((index.wrapping_add(1) as u16) as i16) as i32;
            arena.set_u32(
                list.wrapping_add(8),
                arena.u32(list.wrapping_add(8)).wrapping_add(1),
            );
            arena.set_u32(
                list.wrapping_add(12),
                arena
                    .u32(list.wrapping_add(12))
                    .wrapping_add(voice_states as u32),
            );
            if !(index < count) {
                break;
            }
        }
        if status >= 0x8000_0000 {
            host.call(arena, 0x2db4a0, &[heap]);
            return status;
        }
    }
    arena.set_u8(list.wrapping_add(25), last_timed);
    host.call(arena, 0x2db308, &[heap, ROM_SENTINEL]);
    if verbosity > 2 {
        host.call(
            arena,
            0x1738e8,
            &[
                log,
                ROM_SENTINEL,
                4,
                0,
                ROM_SENTINEL,
                arena.u32(list.wrapping_add(12)),
                arena.u32(list.wrapping_add(8)),
            ],
        );
    }
    if target_states > 0 && (arena.u32(list.wrapping_add(12)) as i32) > target_states {
        host.call(
            arena,
            0x1738e8,
            &[
                log,
                ROM_SENTINEL,
                0,
                0,
                ROM_SENTINEL,
                arena.u32(list.wrapping_add(12)),
            ],
        );
        host.call(
            arena,
            0x1738e8,
            &[
                log,
                ROM_SENTINEL,
                0,
                0,
                ROM_SENTINEL,
                (arena.u32(list.wrapping_add(12)) as i32).wrapping_mul(frame_period) as u32,
            ],
        );
        status = 0x84e0_2000;
        host.call(arena, 0x2db4a0, &[heap]);
        return status;
    }
    arena.set_u32(list.wrapping_add(20), 0);
    let first = arena.u32(list);
    let end = arena.u32(list.wrapping_add(4));
    let first_timed = arena.u8(list.wrapping_add(24));
    node = first;
    while node != end {
        let next = arena.u32(node.wrapping_add(24));
        if (node == first && first_timed == 0)
            || (next == end && arena.u8(list.wrapping_add(25)) == 0)
        {
            arena.set_u32(node.wrapping_add(16), voice_states as u32);
            let durations = arena.u32(node.wrapping_add(12));
            let mut i = 2i32;
            while i < voice_states.wrapping_add(2) {
                arena.set_u16(durations.wrapping_add((i as u32).wrapping_mul(2)), 1);
                arena.set_u32(
                    list.wrapping_add(20),
                    arena.u32(list.wrapping_add(20)).wrapping_add(1),
                );
                i = i.wrapping_add(1);
            }
        } else {
            let durations = arena.u32(node.wrapping_add(12));
            let mut i = 2i32;
            while i < voice_states.wrapping_add(2) {
                arena.set_u32(
                    list.wrapping_add(20),
                    arena.u32(list.wrapping_add(20)).wrapping_add(
                        arena.u16(durations.wrapping_add((i as u32).wrapping_mul(2))) as u32,
                    ),
                );
                i = i.wrapping_add(1);
            }
        }
        node = next;
    }
    if verbosity > 20 {
        host.call(arena, 0x2d9fbc, &[context, list]);
    }
    if verbosity > 1 {
        host.call(arena, 0x1738e8, &[log, ROM_SENTINEL, 4, 0, ROM_SENTINEL]);
    }
    let mut repeat = 0u8;
    let mode = arena.u8(settings.wrapping_add(0x41)) as u32;
    let maximum = arena.u32(settings.wrapping_add(0x4c)) as i32;
    let mut alternate = 0i32;
    let mut strong = 0u32;
    let mut weak = 0u32;
    let mut marked = 0i32;
    let mut duration = arena.u32(arena.u32(list).wrapping_add(16)) as i32;
    node = arena.u32(arena.u32(list).wrapping_add(24));
    let mut wanted = mode;
    let boundaries = arena.u32(state.wrapping_add(0x78));
    while node != end {
        duration = duration.wrapping_add(arena.u32(node.wrapping_add(16)) as i32);
        if duration > maximum {
            if strong != 0 {
                node = strong;
            } else if weak != 0 {
                node = weak;
            }
            strong = 0;
            weak = 0;
            marked = 0;
            arena.set_u8(node.wrapping_add(20), arena.u8(node.wrapping_add(20)) | 1);
            duration = arena.u32(node.wrapping_add(16)) as i32;
        }
        if arena.u32(node.wrapping_add(24)) != end {
            let boundary =
                arena.u8(boundaries.wrapping_add(arena.u16(node.wrapping_add(4)) as u32));
            if boundary == 3 {
                strong = arena.u32(node.wrapping_add(24));
            } else if boundary == 2 {
                weak = arena.u32(node.wrapping_add(24));
                marked = 1;
                node = arena.u32(node.wrapping_add(24));
                continue;
            }
            if marked == 0 && boundary == 1 && mode == 0x34 {
                marked = 1;
                let flagged = arena.u32(node.wrapping_add(24));
                arena.set_u8(
                    flagged.wrapping_add(20),
                    arena.u8(flagged.wrapping_add(20)) | 2,
                );
            }
        }
        node = arena.u32(node.wrapping_add(24));
    }
    if mode == 0x34 {
        wanted = 0x32;
    } else if mode == 0x64 {
        alternate = 1;
        wanted = 0x32;
    } else if mode == 0x74 {
        wanted = 0x32;
        repeat = 1;
    }
    node = arena.u32(arena.u32(list).wrapping_add(24));
    strong = end;
    if node != end && arena.u32(node.wrapping_add(24)) != end {
        loop {
            let boundary =
                arena.u8(boundaries.wrapping_add(arena.u16(node.wrapping_add(4)) as u32));
            if (boundary as u32).wrapping_add(0x30) == wanted {
                if mode == 0x74 && repeat == 1 {
                    repeat = 0;
                } else {
                    let next = arena.u32(node.wrapping_add(24));
                    if next != end {
                        arena.set_u8(next.wrapping_add(20), arena.u8(next.wrapping_add(20)) | 2);
                    }
                    if alternate == 1 {
                        alternate = 0;
                        wanted = 0x33;
                    } else if wanted == 0x32 {
                        repeat = repeat.wrapping_add(1);
                    }
                }
            }
            if boundary == 2 {
                strong = node;
            }
            node = arena.u32(node.wrapping_add(24));
            if arena.u32(node.wrapping_add(24)) == end {
                break;
            }
        }
    }
    while strong != end {
        arena.set_u8(
            strong.wrapping_add(20),
            arena.u8(strong.wrapping_add(20)) | 4,
        );
        strong = arena.u32(strong.wrapping_add(24));
    }
    arena.copy(state.wrapping_add(0x45c), list, 28);
    arena.set_u32(state.wrapping_add(0x568), 0);
    arena.set_u32(state.wrapping_add(0x564), arena.u32(list));
    arena.set_u32(state.wrapping_add(0x3a8c), 0);
    arena.set_u32(state.wrapping_add(0x3a90), 0);
    arena.set_u32(state.wrapping_add(0x3a98), 0);
    arena.set_u32(state.wrapping_add(0x3a9c), 0);
    if status < 0x8000_0000 {
        return status;
    }
    host.call(arena, 0x2db4a0, &[heap]);
    status
}

#[allow(clippy::too_many_arguments)]
pub fn synthesize_phones(
    arena: &mut Arena<'_>,
    host: &mut impl Host,
    synth: u32,
    model: u32,
    config: u32,
    done: u32,
    values: u32,
    blend_current: &[i32],
    blend_previous: &[i32],
) -> u32 {
    let mut pad = 0i32;
    let output = synth.wrapping_add(0x26c);
    let start = arena.u32(synth.wrapping_add(0x494));
    let mut extra = 0i32;
    let mut history_frames = 0i32;
    let mut orders = arena.u32(config.wrapping_add(8)) as i32;
    let queue = synth.wrapping_add(0x440);
    let work = synth.wrapping_add(0x498);
    let previous = synth.wrapping_add(0x4e0);
    let mut overlap = 0i32;
    let input = synth.wrapping_add(0x3d4);
    let context = arena.u32(synth.wrapping_add(0x18));
    let heap = arena.u32(context.wrapping_add(4));
    let database = arena.u32(synth.wrapping_add(0x20));
    let model_count = arena.u32(model.wrapping_add(0x1f8)) as i32;
    let first_order = arena.u32(config.wrapping_add(0x3c)) as i32;
    let mut fast_order = arena.u16(model.wrapping_add(0x394)) as i32;
    if (arena.u32(output.wrapping_add(0x1c)) as i32) <= orders {
        orders = arena.u32(output.wrapping_add(0x1c)) as i32;
    }
    orders = orders.wrapping_add(1);
    if arena.u8(config.wrapping_add(0x114)) == 2 {
        if fast_order >= first_order {
            fast_order = first_order;
        }
        let configured = arena.u32(config.wrapping_add(12)) as i32;
        if configured != 99 && fast_order <= configured {
            fast_order = configured;
        }
    }
    let mut frames = 0i32;
    let mut phones = 0i32;
    arena.set_u8(done, 0);
    arena.set_u8(
        start.wrapping_add(20),
        arena.u8(start.wrapping_add(20)) & !2,
    );
    let mut phone = start;
    while arena.u8(phone.wrapping_add(20)) & 2 == 0 && arena.u32(queue.wrapping_add(4)) != phone {
        phones = phones.wrapping_add(1);
        frames = frames.wrapping_add(arena.u32(phone.wrapping_add(16)) as i32);
        phone = arena.u32(phone.wrapping_add(24));
    }
    if arena.u32(queue.wrapping_add(4)) == phone {
        arena.set_u8(done, 1);
    }
    arena.set_u32(synth.wrapping_add(0x494), phone);
    host.call(
        arena,
        0x1738e8,
        &[
            arena.u32(context.wrapping_add(16)),
            ROM_SENTINEL,
            4,
            0,
            ROM_SENTINEL,
            phones as u32,
            frames as u32,
        ],
    );
    let model_ptr = arena.u32(output.wrapping_add(0x28));
    arena.set_u32(
        work.wrapping_add(0x18),
        arena.u32(model_ptr.wrapping_add(0x18)),
    );
    let mut output_frames = frames;
    arena.set_u32(output.wrapping_add(0x20), output_frames as u32);
    if arena.u32(queue.wrapping_add(4)) != phone && phone != 0 {
        extra = 1;
        phones = phones.wrapping_add(1);
        frames = frames.wrapping_add(arena.u32(config.wrapping_add(0x58)) as i32);
    }
    if arena.u32(synth.wrapping_add(0x45c)) != start {
        let saved_overlap = arena.u32(synth.wrapping_add(0x3a98)) as i32;
        history_frames = arena.u32(config.wrapping_add(0x54)) as i32;
        if saved_overlap > history_frames {
            history_frames = saved_overlap;
        }
        frames = frames.wrapping_add(history_frames);
        output_frames = output_frames.wrapping_add(saved_overlap);
        arena.set_u32(output.wrapping_add(0x20), output_frames as u32);
    }
    let cursor = arena.u32(synth.wrapping_add(0x434)) as i32;
    if arena.u32(queue.wrapping_add(4)) != arena.u32(synth.wrapping_add(0x460))
        && (arena.u32(previous.wrapping_add(0x20)) as i32) < cursor.wrapping_add(output_frames)
    {
        let limit = arena.u32(synth.wrapping_add(0x3a90)) as i32;
        overlap = if frames < limit { frames } else { limit };
        output_frames = output_frames.wrapping_sub(overlap);
        arena.set_u32(output.wrapping_add(0x20), output_frames as u32);
    }
    arena.set_u32(work.wrapping_add(0x20), frames as u32);
    let old_channels = arena.u32(output.wrapping_add(0x18)) as i32;
    arena.set_u32(output.wrapping_add(0x18), 0);
    arena_mid::release_filter(arena, host, output);
    let mut rc = arena_mid::prepare_filter_rows(arena, host, context, 0, output, ROM_SENTINEL);
    if rc >= 0x8000_0000 {
        return cleanup(
            arena, host, synth, input, work, phone, done, fast_order, orders, pad, 0, 0, rc,
        );
    }
    arena.set_u32(output.wrapping_add(0x18), old_channels as u32);
    if first_order >= fast_order {
        arena.set_u32(input.wrapping_add(0x50), 0);
        arena.set_u32(input.wrapping_add(0x54), 0);
        let mut work_channels = arena.u32(work.wrapping_add(0x18)) as i32;
        if work_channels >= 2 {
            work_channels = 2;
            arena.set_u32(work.wrapping_add(0x18), 2);
        }
        pad = 2;
        rc = host
            .call(
                arena,
                0x2a76a4,
                &[
                    heap,
                    work_channels as u32,
                    arena.u32(work.wrapping_add(0x20)),
                    pad as u32,
                    input.wrapping_add(0x50),
                ],
            )
            .0;
        if rc == 0 {
            arena.set_u32(input.wrapping_add(0x3c), arena.u32(work.wrapping_add(0x18)));
            arena.set_u32(input.wrapping_add(0x4c), 0);
            arena.set_u32(input.wrapping_add(0x38), arena.u32(work.wrapping_add(0x20)));
        } else if rc >= 0x8000_0000 {
            return cleanup(
                arena, host, synth, input, work, phone, done, fast_order, orders, pad, 0, 0, rc,
            );
        }
    } else {
        rc = arena_mid::prepare_input_tables(arena, host, context, work, input);
        if rc >= 0x8000_0000 {
            return cleanup(
                arena, host, synth, input, work, phone, done, fast_order, orders, pad, 0, 0, rc,
            );
        }
    }
    if first_order < fast_order {
        let radius = arena.u32(arena.u32(work.wrapping_add(0x28)).wrapping_add(0x30)) as i32;
        rc = arena_mid::prepare_gain_tables(
            arena,
            host,
            context,
            arena.u32(work.wrapping_add(0x20)) as i32,
            radius.wrapping_mul(2).wrapping_add(1),
            arena.u32(work.wrapping_add(0x18)) as i32,
            0,
            input,
        );
        if rc >= 0x8000_0000 {
            return cleanup(
                arena, host, synth, input, work, phone, done, fast_order, orders, pad, 0, 0, rc,
            );
        }
    }
    rc = arena_mid::prepare_filter_rows(arena, host, context, input, work, ROM_SENTINEL);
    if rc >= 0x8000_0000 {
        return cleanup(
            arena, host, synth, input, work, phone, done, fast_order, orders, pad, 0, 0, rc,
        );
    }
    let means = host
        .call(
            arena,
            0x1963b8,
            &[heap, arena.u32(work.wrapping_add(0x18)), 4],
        )
        .0;
    if means == 0 {
        return cleanup(
            arena,
            host,
            synth,
            input,
            work,
            phone,
            done,
            fast_order,
            orders,
            pad,
            0,
            0,
            0x84e0_200a,
        );
    }
    let variances = host
        .call(
            arena,
            0x1963b8,
            &[heap, arena.u32(work.wrapping_add(0x18)), 4],
        )
        .0;
    if variances == 0 {
        return cleanup(
            arena,
            host,
            synth,
            input,
            work,
            phone,
            done,
            fast_order,
            orders,
            pad,
            means,
            0,
            0x84e0_200a,
        );
    }
    let mut order = first_order;
    arena.set_u32(values.wrapping_add(32), first_order as u32);
    arena.set_u32(values.wrapping_add(36), orders as u32);
    arena.set_u32(values.wrapping_add(40), fast_order as u32);
    while order < orders {
        if order == fast_order && first_order != order {
            arena_mid::reset_workspace(arena, host, work, input);
            arena.set_u32(work.wrapping_add(0x34), 0);
            arena.set_u32(work.wrapping_add(0x30), 0);
            let mut work_channels = arena.u32(work.wrapping_add(0x18)) as i32;
            if work_channels >= 2 {
                work_channels = 2;
                arena.set_u32(work.wrapping_add(0x18), 2);
            }
            pad = 2;
            rc = allocate_rows(
                arena,
                host,
                heap,
                work_channels,
                arena.u32(work.wrapping_add(0x20)) as i32,
                pad,
                input.wrapping_add(0x50),
            );
            if rc >= 0x8000_0000 {
                break;
            }
            arena.set_u32(input.wrapping_add(0x3c), arena.u32(work.wrapping_add(0x18)));
            arena.set_u32(input.wrapping_add(0x4c), 0);
            arena.set_u32(input.wrapping_add(0x38), arena.u32(work.wrapping_add(0x20)));
            arena.set_u32(work.wrapping_add(0x30), arena.u32(input.wrapping_add(0x50)));
        }
        let mut frame = 0i32;
        let mut n = 0i32;
        phone = start;
        let work_channels = arena.u32(work.wrapping_add(0x18)) as i32;
        arena.set_u32(values.wrapping_add(44), work.wrapping_add(0x20));
        arena.set_u32(values.wrapping_add(48), arena.u32(work.wrapping_add(0x20)));
        if arena.u32(synth.wrapping_add(0x45c)) != start {
            while frame < history_frames {
                for i in 0..work_channels {
                    let row = arena.u32(
                        arena
                            .u32(work.wrapping_add(0x30))
                            .wrapping_add((i as u32).wrapping_mul(4)),
                    );
                    arena.set_u32(
                        row.wrapping_add((frame as u32).wrapping_mul(4)),
                        arena.u32(history_at(synth, history_frames - frame - 1, order, i)),
                    );
                    if order < fast_order {
                        let vrow = arena.u32(
                            arena
                                .u32(work.wrapping_add(0x34))
                                .wrapping_add((i as u32).wrapping_mul(4)),
                        );
                        arena.set_u32(
                            vrow.wrapping_add((frame as u32).wrapping_mul(4)),
                            arena.u32(history_at(synth, history_frames - frame - 1, order + 25, i)),
                        );
                    }
                }
                frame = frame.wrapping_add(1);
            }
        }
        let voice = model.wrapping_add(0x1e0);
        while n != phones {
            let models = arena.u32(phone.wrapping_add(8));
            let durations = arena.u32(phone.wrapping_add(12));
            let mut j = 2i32;
            while j < model_count.wrapping_add(2)
                && frame < arena.u32(work.wrapping_add(0x20)) as i32
            {
                arena.set_u32(values, 0);
                arena.set_u32(values.wrapping_add(4), means);
                arena.set_u32(values.wrapping_add(8), variances);
                if order >= fast_order {
                    rc = host
                        .call(
                            arena,
                            0x2da8c8,
                            &[
                                database,
                                j as u32,
                                arena.u16(models.wrapping_add((j as u32).wrapping_mul(2))) as u32,
                                order as u32,
                                work_channels as u32,
                                values,
                                voice,
                            ],
                        )
                        .0;
                } else {
                    rc = super::arena::read_voice_pair(
                        arena,
                        host,
                        database,
                        j as u32,
                        arena.u16(models.wrapping_add((j as u32).wrapping_mul(2))) as u32,
                        order as u32,
                        values,
                        voice,
                    );
                }
                if rc >= 0x8000_0000 {
                    break;
                }
                let mut k = 1i32;
                while k <= arena.u16(durations.wrapping_add((j as u32).wrapping_mul(2))) as i32
                    && frame < arena.u32(work.wrapping_add(0x20)) as i32
                {
                    for channel in 0..work_channels {
                        let row = arena.u32(
                            arena
                                .u32(work.wrapping_add(0x30))
                                .wrapping_add((channel as u32).wrapping_mul(4)),
                        );
                        let means_base = arena.u32(values.wrapping_add(4));
                        let value = arena
                            .u32(means_base.wrapping_add((channel as u32).wrapping_mul(4)))
                            as i32;
                        arena.set_u32(
                            row.wrapping_add((frame as u32).wrapping_mul(4)),
                            value as u32,
                        );
                        arena.set_u32(
                            row.wrapping_add((frame as u32).wrapping_mul(4)),
                            (if value > 0x7d00000 {
                                0x7d00000
                            } else if value <= -0x7d00000 {
                                -0x7d00000
                            } else {
                                value
                            }) as u32,
                        );
                        if order < fast_order {
                            let vrow = arena.u32(
                                arena
                                    .u32(work.wrapping_add(0x34))
                                    .wrapping_add((channel as u32).wrapping_mul(4)),
                            );
                            let variances_base = arena.u32(values.wrapping_add(8));
                            let mut v = arena
                                .u32(variances_base.wrapping_add((channel as u32).wrapping_mul(4)))
                                as i32;
                            arena.set_u32(
                                vrow.wrapping_add((frame as u32).wrapping_mul(4)),
                                v as u32,
                            );
                            if v > 0x7d00000 {
                                v = 0x7d00000;
                            }
                            arena.set_u32(
                                vrow.wrapping_add((frame as u32).wrapping_mul(4)),
                                v as u32,
                            );
                        }
                    }
                    frame = frame.wrapping_add(1);
                    k = k.wrapping_add(1);
                }
                j = j.wrapping_add(1);
            }
            if rc >= 0x8000_0000 {
                break;
            }
            n = n.wrapping_add(1);
            phone = arena.u32(phone.wrapping_add(24));
        }
        if rc >= 0x8000_0000 {
            break;
        }
        if extra != 0 {
            frame = frame.wrapping_sub(arena.u32(config.wrapping_add(0x58)) as i32);
        }
        let history_limit = arena.u32(config.wrapping_add(0x54)) as i32;
        let keep = if history_limit > overlap {
            history_limit
        } else {
            overlap
        };
        let mut i = 0i32;
        while i < keep {
            for j in 0..arena.u32(work.wrapping_add(0x18)) as i32 {
                let row = arena.u32(
                    arena
                        .u32(work.wrapping_add(0x30))
                        .wrapping_add((j as u32).wrapping_mul(4)),
                );
                let at = if i < frame { frame - i - 1 } else { 0 };
                arena.set_u32(
                    history_at(synth, i, order, j),
                    arena.u32(row.wrapping_add((at as u32).wrapping_mul(4))),
                );
                if order < fast_order {
                    let vrow = arena.u32(
                        arena
                            .u32(work.wrapping_add(0x34))
                            .wrapping_add((j as u32).wrapping_mul(4)),
                    );
                    arena.set_u32(
                        history_at(synth, i, order + 25, j),
                        arena.u32(vrow.wrapping_add((at as u32).wrapping_mul(4))),
                    );
                }
            }
            i = i.wrapping_add(1);
        }
        if order >= fast_order {
            host.call(arena, 0x2a3b18, &[context, work]);
        } else if arena.u32(model.wrapping_add(0x488)) == 0 {
            host.call(arena, 0x2a24f8, &[context, work]);
        } else if order < 3 {
            arena_mid::run_forward_stream(arena, host, context, work);
        } else {
            host.call(
                arena,
                0x2daf68,
                &[
                    work,
                    order as u32,
                    arena.u32(synth.wrapping_add(0x3aa0)),
                    model.wrapping_add(0x484),
                ],
            );
        }
        let linear = arena.u32(work.wrapping_add(0x2c));
        output_frames = arena.u32(output.wrapping_add(0x20)) as i32;

        let blend_bound = arena.u32(synth.wrapping_add(0x3a98)) as i32;
        let capped = if blend_bound > 7 { 7 } else { blend_bound };
        let mut i = 0i32;
        while i < capped {
            let line = arena.u32(linear.wrapping_add((i as u32).wrapping_mul(4))) as i32;
            let tail = arena.u32(tails_at(synth, order, i)) as i32;
            arena.set_u32(
                linear.wrapping_add((i as u32).wrapping_mul(4)),
                (((line.wrapping_add(4) >> 3).wrapping_mul(blend_value(blend_current, i)))
                    .wrapping_add(
                        (tail.wrapping_add(4) >> 3).wrapping_mul(blend_value(blend_previous, i)),
                    )) as u32,
            );
            i = i.wrapping_add(1);
        }
        if overlap != 0 {
            let mut offset = 0i32;
            if arena.u32(synth.wrapping_add(0x45c)) != start
                && (arena.u32(synth.wrapping_add(0x3a98)) as i32)
                    < (arena.u32(config.wrapping_add(0x54)) as i32)
            {
                offset = (arena.u32(config.wrapping_add(0x54)) as i32)
                    .wrapping_sub(arena.u32(synth.wrapping_add(0x3a98)) as i32);
            }
            arena_leaf::run(
                arena,
                0x195ae8,
                &[
                    tails_at(synth, order, 0),
                    linear
                        .wrapping_add((output_frames.wrapping_add(offset) as u32).wrapping_mul(4)),
                    (overlap as u32).wrapping_mul(4),
                ],
            );
        }
        let mut i = 0i32;
        if arena.u32(synth.wrapping_add(0x45c)) != start
            && (arena.u32(synth.wrapping_add(0x3a98)) as i32)
                < arena.u32(config.wrapping_add(0x54)) as i32
        {
            i = (arena.u32(config.wrapping_add(0x54)) as i32)
                .wrapping_sub(arena.u32(synth.wrapping_add(0x3a98)) as i32);
        }
        let mut j = 0i32;
        while j < output_frames {
            let row = arena.u32(
                arena
                    .u32(output.wrapping_add(0x2c))
                    .wrapping_add((j as u32).wrapping_mul(4)),
            );
            arena.set_u32(
                row.wrapping_add((order as u32).wrapping_mul(4)),
                arena.u32(linear.wrapping_add((i as u32).wrapping_mul(4))),
            );
            i = i.wrapping_add(1);
            j = j.wrapping_add(1);
        }
        order = order.wrapping_add(1);
    }

    if rc < 0x8000_0000 {
        arena.set_u32(synth.wrapping_add(0x3a98), overlap as u32);
    }
    let mut i = 0i32;
    while rc < 0x8000_0000 && i < first_order {
        let mut j = 0i32;
        while j < output_frames {
            let dst = arena.u32(
                arena
                    .u32(output.wrapping_add(0x2c))
                    .wrapping_add((j as u32).wrapping_mul(4)),
            );
            let src = arena.u32(
                arena
                    .u32(previous.wrapping_add(0x2c))
                    .wrapping_add(((cursor.wrapping_add(j)) as u32).wrapping_mul(4)),
            );
            arena.set_u32(
                dst.wrapping_add((i as u32).wrapping_mul(4)),
                arena.u32(src.wrapping_add((i as u32).wrapping_mul(4))),
            );
            j = j.wrapping_add(1);
        }
        i = i.wrapping_add(1);
    }
    cleanup(
        arena, host, synth, input, work, phone, done, fast_order, orders, pad, means, variances, rc,
    )
}

fn history_at(synth: u32, frame: i32, order: i32, channel: i32) -> u32 {
    synth.wrapping_add(0x56c).wrapping_add(
        (((frame as u32).wrapping_mul(50).wrapping_add(order as u32))
            .wrapping_mul(3)
            .wrapping_add(channel as u32))
        .wrapping_mul(4),
    )
}

fn tails_at(synth: u32, order: i32, index: i32) -> u32 {
    synth
        .wrapping_add(0x344c)
        .wrapping_add(((order as u32).wrapping_mul(10).wrapping_add(index as u32)).wrapping_mul(4))
}

fn blend_value(table: &[i32], index: i32) -> i32 {
    table[index as usize]
}

#[allow(clippy::too_many_arguments)]
fn cleanup(
    arena: &mut Arena<'_>,
    host: &mut impl Host,
    synth: u32,
    input: u32,
    work: u32,
    _phone: u32,
    done: u32,
    fast_order: i32,
    orders: i32,
    pad: i32,
    means: u32,
    variances: u32,
    rc: u32,
) -> u32 {
    let context = arena.u32(synth.wrapping_add(0x18));
    let heap = arena.u32(context.wrapping_add(4));
    if means != 0 {
        host.call(arena, 0x196428, &[heap, means]);
    }
    if variances != 0 {
        host.call(arena, 0x196428, &[heap, variances]);
    }
    if fast_order < orders {
        let data = arena.u32(input.wrapping_add(0x50));
        if data != 0 {
            host.call(
                arena,
                0x196428,
                &[
                    heap,
                    arena.u32(data).wrapping_sub((pad as u32).wrapping_mul(4)),
                ],
            );
            host.call(arena, 0x196428, &[heap, data]);
            arena.set_u32(input.wrapping_add(0x50), 0);
        }
        arena.set_u32(work.wrapping_add(0x30), 0);
    }
    arena_mid::reset_workspace(arena, host, work, input);
    arena_mid::release_filter(arena, host, work);
    if arena.u8(done) != 0 {
        arena_mid::release_filter(arena, host, synth.wrapping_add(0x4e0));
    }
    let model_ptr = arena.u32(synth.wrapping_add(0x26c).wrapping_add(0x28));
    arena.set_u32(
        work.wrapping_add(0x18),
        arena.u32(model_ptr.wrapping_add(0x18)),
    );
    rc
}

fn allocate_rows(
    arena: &mut Arena<'_>,
    host: &mut impl Host,
    heap: u32,
    rows: i32,
    columns: i32,
    pad: i32,
    out: u32,
) -> u32 {
    let array = host
        .call(arena, 0x196394, &[heap, (rows as u32).wrapping_shl(2)])
        .0;
    arena.set_u32(out, array);
    if array == 0 {
        return 0x84e0_200a;
    }
    let stride = columns.wrapping_add(pad.wrapping_mul(2));
    let mut data = host
        .call(
            arena,
            0x196394,
            &[heap, (stride.wrapping_mul(rows) as u32).wrapping_shl(2)],
        )
        .0;
    if data == 0 {
        host.call(arena, 0x196428, &[heap, arena.u32(out)]);
        arena.set_u32(out, 0);
        return 0x84e0_200a;
    }
    let mut i = 0i32;
    while i < rows {
        arena.set_u32(
            array.wrapping_add((i as u32).wrapping_mul(4)),
            data.wrapping_add((pad as u32).wrapping_mul(4)),
        );
        data = data.wrapping_add((stride as u32).wrapping_mul(4));
        i = i.wrapping_add(1);
    }
    0
}

#[allow(clippy::too_many_arguments)]
pub fn setup_features(
    arena: &mut Arena<'_>,
    host: &mut impl Host,
    engine: u32,
    text: u32,
    length: u32,
    markers: u32,
    marker_count: u32,
    text_p: u32,
    text_h: u32,
    count_p: u32,
    count_h: u32,
    mut wait_factor: i32,
    feat_arena: u32,
    features: u32,
    out_markers: u32,
    out_count: u32,
    scratch: u32,
) -> u32 {
    let context = arena.u32(engine);
    let heap = arena.u32(context.wrapping_add(4));
    let log = arena.u32(context.wrapping_add(16));
    let params = arena.u32(context.wrapping_add(20));
    let debug = host.call(arena, 0x173fd8, &[log]).0 >= 6;
    let mut extra = 0u32;
    let mut parser = 0u32;
    let number = scratch;
    let phone = scratch.wrapping_add(16);
    let parser_slot = scratch.wrapping_add(20);
    let name_slot = scratch.wrapping_add(24);
    let debug_name = scratch.wrapping_add(32);
    let not_set = scratch.wrapping_add(96);
    let special_set = scratch.wrapping_add(100);
    let prefix8 = scratch.wrapping_add(107);

    let prefix16 = scratch.wrapping_add(140);
    arena_leaf::run(arena, 0x195bec, &[number, 0, 16]);
    for (i, byte) in [0x21u8, 0x3c, 0x3e, 0].iter().enumerate() {
        arena.set_u8(not_set.wrapping_add(i as u32), *byte);
    }
    for (i, byte) in [0x2du8, 0x25, 0x27, 0x22, 0x2a, 0, 0].iter().enumerate() {
        arena.set_u8(special_set.wrapping_add(i as u32), *byte);
    }
    for (i, byte) in b"application/x-realspeak-u8-feat-\0".iter().enumerate() {
        arena.set_u8(prefix8.wrapping_add(i as u32), *byte);
    }
    for (i, byte) in b"application/x-realspeak-u16-feat-\0".iter().enumerate() {
        arena.set_u8(prefix16.wrapping_add(i as u32), *byte);
    }
    let mut rc: u32;
    if text == 0 || length == 0 || arena.u8(text.wrapping_add(length.wrapping_sub(1))) != 0 {
        return 0x84e0_2007;
    }
    if out_markers != 0 && out_count != 0 {
        arena.set_u32(out_markers, 0);
        arena.set_u32(out_count, 0);
    }
    host.call(arena, 0x2db19c, &[feat_arena, context, ROM_SENTINEL]);
    arena.set_u32(feat_arena.wrapping_add(12), 0x1000);
    arena_leaf::run(arena, 0x195bec, &[features, 0, 0x22c]);
    let mut cursor = text;
    while matches!(arena.u8(cursor), 0x3c | 0x27 | 0x22 | 0x60) {
        cursor = cursor.wrapping_add(1);
    }
    if arena.u8(cursor) != 0x23 {
        rc = host
            .call(arena, 0x17b704, &[heap, ROM_SENTINEL, parser_slot])
            .0;

        parser = arena.u32(parser_slot);
        if rc >= 0x8000_0000 {
            return finish_setup(
                arena,
                host,
                parser,
                rc,
                feat_arena,
                features,
                out_markers,
                out_count,
                heap,
            );
        }
    }
    if parser == 0 {
        rc = host.call(arena, 0x17b500, &[heap, parser_slot]).0;
        parser = arena.u32(parser_slot);
        if rc >= 0x8000_0000 {
            return finish_setup(
                arena,
                host,
                parser,
                rc,
                feat_arena,
                features,
                out_markers,
                out_count,
                heap,
            );
        }
    }
    rc = host.call(arena, 0x17b53c, &[parser, length]).0;
    if rc >= 0x8000_0000 {
        return finish_setup(
            arena,
            host,
            parser,
            rc,
            feat_arena,
            features,
            out_markers,
            out_count,
            heap,
        );
    }
    if arena_leaf::run(arena, 0x19600c, &[text, not_set]) == 0 {
        rc = host.call(arena, 0x17b9d4, &[parser, text]).0;
        if rc >= 0x8000_0000 {
            return finish_setup(
                arena,
                host,
                parser,
                rc,
                feat_arena,
                features,
                out_markers,
                out_count,
                heap,
            );
        }
    } else {

        cursor = text;
        while arena.u8(cursor) != 0 {
            let ch = arena.u8(cursor);
            if ch == 0x3c || ch == 0x3e || ch == 0x21 {
                cursor = cursor.wrapping_add(1);
            } else if ch == arena.u8(cursor.wrapping_add(1))
                && arena.u8(cursor.wrapping_add(2)) == 0x21
            {
                rc = host.call(arena, 0x17b908, &[parser, ch as u32]).0;
                cursor = cursor.wrapping_add(3);
            } else {
                let n = arena_leaf::run(arena, 0x195eb4, &[cursor.wrapping_add(1), special_set]);
                if n == 0
                    || ch != arena.u8(cursor.wrapping_add(n.wrapping_add(1)))
                    || arena.u8(cursor.wrapping_add(n.wrapping_add(2))) != 0x21
                {
                    rc = host.call(arena, 0x17b908, &[parser, ch as u32]).0;
                    cursor = cursor.wrapping_add(1);
                } else {
                    rc = host
                        .call(arena, 0x17b960, &[parser, cursor, n.wrapping_add(1)])
                        .0;
                }
            }
            if rc >= 0x8000_0000 {
                return finish_setup(
                    arena,
                    host,
                    parser,
                    rc,
                    feat_arena,
                    features,
                    out_markers,
                    out_count,
                    heap,
                );
            }
        }
    }
    let stream = arena.u32(engine.wrapping_add(4));
    let get = arena.u32(stream.wrapping_add(0x2c));
    let set = arena.u32(stream.wrapping_add(0x30));
    let bytes59 = features.wrapping_add(0xf8);
    let capacity = features.wrapping_add(0x21c);
    rc = host
        .call(arena, get, &[text_p, text_h, bytes59, capacity])
        .0;
    if rc >= 0x8000_0000 || arena.u32(capacity) == 0 {
        arena.set_u32(bytes59, 0);
        arena.set_u32(features.wrapping_add(0x218), 0);
        arena.set_u32(capacity, 0);
    } else {
        rc = host
            .call(
                arena,
                get,
                &[count_p, count_h, features.wrapping_add(0x218), capacity],
            )
            .0;
        if rc < 0x8000_0000 && arena.u32(capacity) != 0 {
            arena.set_u32(capacity, arena.u32(capacity) >> 2);
            extra = host
                .call(arena, 0x2943ec, &[text, length])
                .0
                .wrapping_mul(4);
        }
    }
    let phones = host.call(arena, 0x17b7f0, &[parser]).0;
    arena.set_u32(features.wrapping_add(12), phones);
    let mut raw_count = host.call(arena, 0x17b824, &[parser]).0 as i32;
    let mut count = 0u32;
    let mut at = phones;
    let mut n = 0u32;
    while n < raw_count as u32 {
        if arena.u8(engine.wrapping_add(0x748).wrapping_add(arena.u8(at) as u32)) != 255 {
            count = count.wrapping_add(1);
        }
        at = at.wrapping_add(1);
        n = n.wrapping_add(1);
    }
    count = count.wrapping_add(1).wrapping_add(extra);
    raw_count = (raw_count as u32).wrapping_add(extra) as i32;
    let spectral_order = arena.u16(engine.wrapping_add(0xa3a)) as u32;
    let model_order = arena.u16(engine.wrapping_add(0x994)) as u32;
    let bytes9 = count.wrapping_mul(9);
    let bytes8 = count.wrapping_mul(8);
    let mut i = 0u32;
    while i < 60 {
        match arena.u8(engine.wrapping_add(0x958).wrapping_add(i)) {
            0 => {}
            1 => {
                let mut n = count;
                if i == 29 {
                    n = (raw_count as u32).wrapping_add(1);
                }
                let pointer = host.call(arena, 0x2db1d0, &[feat_arena, n]).0;
                arena.set_u32(
                    features.wrapping_add(12).wrapping_add(i.wrapping_mul(4)),
                    pointer,
                );
                if pointer == 0 {
                    return setup_alloc_failed(
                        arena,
                        host,
                        parser,
                        feat_arena,
                        features,
                        out_markers,
                        out_count,
                        heap,
                        log,
                    );
                }
            }
            2 => {
                let mut n = count;
                if i == 3 {
                    n = n.wrapping_add(1);
                }
                let pointer = host
                    .call(arena, 0x2db1d0, &[feat_arena, n.wrapping_mul(2)])
                    .0;
                arena.set_u32(
                    features.wrapping_add(0xfc).wrapping_add(i.wrapping_mul(4)),
                    pointer,
                );
                if pointer == 0 {
                    return setup_alloc_failed(
                        arena,
                        host,
                        parser,
                        feat_arena,
                        features,
                        out_markers,
                        out_count,
                        heap,
                        log,
                    );
                }
            }
            _ => match i {
                53 => {
                    let pointer = host
                        .call(
                            arena,
                            0x2db1d0,
                            &[
                                feat_arena,
                                spectral_order.wrapping_mul(count).wrapping_mul(4),
                            ],
                        )
                        .0;
                    arena.set_u32(features.wrapping_add(0x208), pointer);
                    if pointer == 0 {
                        return setup_alloc_failed(
                            arena,
                            host,
                            parser,
                            feat_arena,
                            features,
                            out_markers,
                            out_count,
                            heap,
                            log,
                        );
                    }
                }
                54 => {
                    let pointer = host
                        .call(
                            arena,
                            0x2db1d0,
                            &[
                                feat_arena,
                                spectral_order.wrapping_mul(count).wrapping_mul(4),
                            ],
                        )
                        .0;
                    arena.set_u32(features.wrapping_add(0x20c), pointer);
                    if pointer == 0 {
                        return setup_alloc_failed(
                            arena,
                            host,
                            parser,
                            feat_arena,
                            features,
                            out_markers,
                            out_count,
                            heap,
                            log,
                        );
                    }
                }
                24 => {
                    let pointer = host.call(arena, 0x2db1d0, &[feat_arena, bytes8]).0;
                    arena.set_u32(features.wrapping_add(0x1fc), pointer);
                    if pointer == 0 {
                        return setup_alloc_failed(
                            arena,
                            host,
                            parser,
                            feat_arena,
                            features,
                            out_markers,
                            out_count,
                            heap,
                            log,
                        );
                    }
                }
                46 => {
                    let pointer = host.call(arena, 0x2db1d0, &[feat_arena, bytes9]).0;
                    arena.set_u32(features.wrapping_add(0x200), pointer);
                    if pointer == 0 {
                        return setup_alloc_failed(
                            arena,
                            host,
                            parser,
                            feat_arena,
                            features,
                            out_markers,
                            out_count,
                            heap,
                            log,
                        );
                    }
                }
                52 => {
                    let pointer = host
                        .call(
                            arena,
                            0x2db1d0,
                            &[feat_arena, model_order.wrapping_mul(count).wrapping_mul(4)],
                        )
                        .0;
                    arena.set_u32(features.wrapping_add(0x204), pointer);
                    if pointer == 0 {
                        return setup_alloc_failed(
                            arena,
                            host,
                            parser,
                            feat_arena,
                            features,
                            out_markers,
                            out_count,
                            heap,
                            log,
                        );
                    }
                }
                56 => {
                    let pointer = host
                        .call(
                            arena,
                            0x2db1d0,
                            &[feat_arena, model_order.wrapping_mul(count)],
                        )
                        .0;
                    arena.set_u32(features.wrapping_add(0x210), pointer);
                    if pointer == 0 {
                        return setup_alloc_failed(
                            arena,
                            host,
                            parser,
                            feat_arena,
                            features,
                            out_markers,
                            out_count,
                            heap,
                            log,
                        );
                    }
                }
                _ => {
                    host.call(
                        arena,
                        0x173dc0,
                        &[log, ROM_SENTINEL, 25014, ROM_SENTINEL, ROM_SENTINEL, i],
                    );
                }
            },
        }
        i = i.wrapping_add(1);
    }
    let states = host.call(arena, 0x2db1d0, &[feat_arena, bytes8]).0;
    arena.set_u32(features.wrapping_add(0x214), states);
    if states == 0 {
        return setup_alloc_failed(
            arena,
            host,
            parser,
            feat_arena,
            features,
            out_markers,
            out_count,
            heap,
            log,
        );
    }
    let mut i = 0u32;
    while i < count.wrapping_mul(2) {
        arena.set_u32(states.wrapping_add(i.wrapping_mul(4)), 0xffff_ffff);
        i = i.wrapping_add(1);
    }
    let bytes4 = arena.u32(features.wrapping_add(12).wrapping_add(16));
    if bytes4 != 0 {
        arena_leaf::run(arena, 0x195bec, &[bytes4, 32, count]);
    }
    if out_markers != 0 && out_count != 0 && arena.u32(engine.wrapping_add(0x8b0)) != 0 {
        let pointer = host
            .call(
                arena,
                0x2db1d0,
                &[feat_arena, (raw_count as u32).wrapping_add(1)],
            )
            .0;
        arena.set_u32(features.wrapping_add(16), pointer);
        if pointer == 0 {
            return setup_alloc_failed(
                arena,
                host,
                parser,
                feat_arena,
                features,
                out_markers,
                out_count,
                heap,
                log,
            );
        }
        arena.set_u32(features.wrapping_add(0x1ec), markers);
        arena.set_u32(features.wrapping_add(0x1f0), marker_count);
    }
    arena.set_u16(
        features.wrapping_add(8),
        (raw_count as u32).wrapping_sub(extra) as u16,
    );
    arena.set_u16(features.wrapping_add(6), count.wrapping_sub(extra) as u16);
    arena.set_u16(
        features.wrapping_add(0x1f4),
        count.wrapping_sub(extra).wrapping_mul(2) as u16,
    );
    arena.set_u32(features, engine.wrapping_add(8));
    rc = host
        .call(arena, 0x2902d8, &[context, ROM_SENTINEL, features])
        .0;
    if rc >= 0x8000_0000 {
        return finish_setup(
            arena,
            host,
            parser,
            rc,
            feat_arena,
            features,
            out_markers,
            out_count,
            heap,
        );
    }
    raw_count = arena.u16(features.wrapping_add(8)) as i16 as i32;
    let mut count_i = arena.u16(features.wrapping_add(6)) as i16 as i32;
    if raw_count <= 0 {
        raw_count = 0;
    }
    if count_i <= 0 {
        count_i = 0;
    }
    let count = count_i;
    if debug {
        host.call(arena, 0x176ac8, &[78, number, 10]);
        let mut i = 0u32;
        while i < 60 {
            let column_type = arena.u8(engine.wrapping_add(0x958).wrapping_add(i));
            let mut label = 0u32;
            let mut event = 99u32;
            let mut dump = count as u32;
            match i {
                2 => label = ROM_SENTINEL,
                4 => label = ROM_SENTINEL,
                1 => {
                    label = ROM_SENTINEL;
                    dump = raw_count as u32;
                }
                5 => label = ROM_SENTINEL,
                3 => label = ROM_SENTINEL,
                _ => {
                    if column_type != 0 {
                        label = debug_name;
                        event = 100;
                        arena_leaf::run(
                            arena,
                            0x195cd0,
                            &[
                                debug_name,
                                if column_type == 1 { prefix8 } else { prefix16 },
                            ],
                        );
                        let debug_len = arena_leaf::run(arena, 0x195d90, &[debug_name]);
                        host.call(
                            arena,
                            0x176c58,
                            &[i, debug_name.wrapping_add(debug_len), 10],
                        );
                        if i == 29 {
                            dump = raw_count as u32;
                        }
                    }
                }
            }
            match column_type {
                0 => {
                    if i == 1 && arena.u32(features.wrapping_add(16)) != 0 {
                        host.call(
                            arena,
                            0x173b00,
                            &[
                                log,
                                ROM_SENTINEL,
                                event,
                                0,
                                number,
                                label,
                                arena.u32(features.wrapping_add(16)),
                                dump,
                                0,
                            ],
                        );
                    }
                }
                1 => {
                    host.call(
                        arena,
                        0x173b00,
                        &[
                            log,
                            ROM_SENTINEL,
                            event,
                            0,
                            number,
                            label,
                            arena.u32(features.wrapping_add(12).wrapping_add(i.wrapping_mul(4))),
                            dump,
                            0,
                        ],
                    );
                }
                2 => {
                    host.call(
                        arena,
                        0x173b00,
                        &[
                            log,
                            ROM_SENTINEL,
                            event,
                            0,
                            number,
                            label,
                            arena.u32(features.wrapping_add(12).wrapping_add(i.wrapping_mul(4))),
                            dump.wrapping_mul(2),
                            0,
                        ],
                    );
                }
                _ => {}
            }
            i = i.wrapping_add(1);
        }
    }
    if out_markers != 0 && out_count != 0 {
        let total = (count as u32).wrapping_add(marker_count);
        arena.set_u32(out_count, total);
        if total != 0 {
            let mut phone_index = 0u32;
            let mut marker_index = 0u32;
            let table = host.call(arena, 0x1963b8, &[heap, total, 24]).0;
            arena.set_u32(out_markers, table);
            if table == 0 {
                return setup_alloc_failed(
                    arena,
                    host,
                    parser,
                    feat_arena,
                    features,
                    out_markers,
                    out_count,
                    heap,
                    log,
                );
            }
            let bytes5 = arena.u32(features.wrapping_add(12).wrapping_add(20));
            let bytes2 = arena.u32(features.wrapping_add(12).wrapping_add(8));
            let mut i = 0u32;
            while i < total {
                if marker_index < marker_count
                    && (phone_index >= count as u32
                        || arena.u8(bytes5.wrapping_add(phone_index)) != 0)
                {
                    arena.copy(
                        table.wrapping_add(i.wrapping_mul(24)),
                        markers.wrapping_add(marker_index.wrapping_mul(24)),
                        24,
                    );
                    marker_index = marker_index.wrapping_add(1);
                    if phone_index < count as u32 {
                        arena.set_u8(
                            bytes5.wrapping_add(phone_index),
                            arena.u8(bytes5.wrapping_add(phone_index)).wrapping_sub(1),
                        );
                    }
                } else {
                    arena.set_u16(table.wrapping_add(i.wrapping_mul(24)), 33);
                    arena.set_u32(
                        table.wrapping_add(i.wrapping_mul(24)).wrapping_add(20),
                        arena.u8(bytes2.wrapping_add(phone_index)) as u32,
                    );
                    phone_index = phone_index.wrapping_add(1);
                }
                i = i.wrapping_add(1);
            }
        }
    }
    if markers != 0 && marker_count != 0 {
        let mut wait = wait_factor;
        let mut i = 0u32;
        while i < marker_count {
            if arena.u16(markers.wrapping_add(i.wrapping_mul(24))) == 25 {
                let value = arena.u32(markers.wrapping_add(i.wrapping_mul(24)).wrapping_add(20));
                host.call(arena, 0x1758ac, &[params, ROM_SENTINEL, value]);
                wait = value as i32;
            }
            i = i.wrapping_add(1);
        }
        wait_factor = wait;
    }
    let bytes2 = arena.u32(features.wrapping_add(12).wrapping_add(8));
    let words3 = arena.u32(features.wrapping_add(0xfc).wrapping_add(12));
    let phone_map = arena.u32(engine.wrapping_add(0x3b54));
    let mut i = 0u32;
    while i < count as u32 {
        let mut id = arena.u8(bytes2.wrapping_add(i));
        arena.set_u8(phone, id);
        arena.set_u8(phone.wrapping_add(1), 0);
        let lookup = host.call(arena, 0x24197c, &[phone_map, phone, name_slot]).0;
        let mut name = arena.u32(name_slot);
        if lookup >= 0x8000_0000 {
            host.call(
                arena,
                0x173dc0,
                &[
                    log,
                    ROM_SENTINEL,
                    25012,
                    ROM_SENTINEL,
                    ROM_SENTINEL,
                    id as u32,
                ],
            );
            name = 0;
        }
        if name != 0 && arena.u8(name) == 0x41 {
            id = host.call(arena, 0x17c550, &[name.wrapping_add(1)]).0 as u8;
        } else {
            id = 255;
        }
        arena.set_u8(bytes2.wrapping_add(i), id);
        if words3 != 0 {
            let mut duration = arena.u16(words3.wrapping_add(i.wrapping_mul(2)));
            duration = if duration > 6553 {
                0xffff
            } else {
                duration.wrapping_mul(10)
            };
            arena.set_u16(words3.wrapping_add(i.wrapping_mul(2)), duration);
            if i.wrapping_add(1) == count as u32 {
                let mut adjusted = (duration as i32).wrapping_add(wait_factor.wrapping_mul(2000));
                if adjusted == 0 {
                    adjusted = 1;
                } else if adjusted >= 65535 {
                    adjusted = 65535;
                }
                arena.set_u16(words3.wrapping_add(i.wrapping_mul(2)), adjusted as u16);
            }
        }
        i = i.wrapping_add(1);
    }
    let out_bytes = arena.u32(bytes59);
    if out_bytes != 0 {
        let out_len = arena_leaf::run(arena, 0x195d90, &[out_bytes]);
        rc = host
            .call(arena, set, &[text_p, text_h, out_len.wrapping_add(1)])
            .0;
        if rc >= 0x8000_0000 {
            return finish_setup(
                arena,
                host,
                parser,
                rc,
                feat_arena,
                features,
                out_markers,
                out_count,
                heap,
            );
        }
    }
    if arena.u32(features.wrapping_add(0x218)) != 0 {
        rc = host
            .call(
                arena,
                set,
                &[count_p, count_h, arena.u32(capacity).wrapping_mul(4)],
            )
            .0;
    }
    finish_setup(
        arena,
        host,
        parser,
        rc,
        feat_arena,
        features,
        out_markers,
        out_count,
        heap,
    )
}

#[allow(clippy::too_many_arguments)]
fn finish_setup(
    arena: &mut Arena<'_>,
    host: &mut impl Host,
    parser: u32,
    rc: u32,
    feat_arena: u32,
    features: u32,
    out_markers: u32,
    out_count: u32,
    heap: u32,
) -> u32 {
    if parser != 0 {
        host.call(arena, 0x17b654, &[parser]);
    }
    if rc >= 0x8000_0000 {
        host.call(arena, 0x2db4a0, &[feat_arena]);
        arena_leaf::run(arena, 0x195bec, &[features, 0, 0x22c]);
        if out_markers != 0 && out_count != 0 && arena.u32(out_markers) != 0 {
            host.call(arena, 0x196428, &[heap, arena.u32(out_markers)]);
            arena.set_u32(out_markers, 0);
            arena.set_u32(out_count, 0);
        }
    }
    rc
}

#[allow(clippy::too_many_arguments)]
fn setup_alloc_failed(
    arena: &mut Arena<'_>,
    host: &mut impl Host,
    parser: u32,
    feat_arena: u32,
    features: u32,
    out_markers: u32,
    out_count: u32,
    heap: u32,
    log: u32,
) -> u32 {
    host.call(arena, 0x173dc0, &[log, ROM_SENTINEL, 25000, 0]);
    finish_setup(
        arena,
        host,
        parser,
        0x84e0_200a,
        feat_arena,
        features,
        out_markers,
        out_count,
        heap,
    )
}

#[allow(clippy::too_many_arguments)]
pub fn open_voice_object(
    arena: &mut Arena<'_>,
    host: &mut impl Host,
    handle_p: u32,
    handle_tag: u32,
    param: u32,
    obj: u32,
    name: u32,
    out: u32,
    scratch: u32,
) -> u32 {
    let tag = scratch;
    let wtmp = scratch.wrapping_add(8);
    let vlen = scratch.wrapping_add(12);
    let tags = scratch.wrapping_add(16);
    let context_slot = scratch.wrapping_add(72);
    let rem_slot = scratch.wrapping_add(76);
    let cur_slot = scratch.wrapping_add(80);
    for (t, literal) in [
        "VPHO\0", "VOIC\0", "COVO\0", "PLOS\0", "CLAS\0", "DIFF\0", "TAGA\0", "FXOP\0", "TARG\0",
        "TRAN\0", "PHMP\0",
    ]
    .iter()
    .enumerate()
    {
        for (i, byte) in literal.bytes().enumerate() {
            arena.set_u8(
                tags.wrapping_add((t as u32).wrapping_mul(5).wrapping_add(i as u32)),
                byte,
            );
        }
    }
    let tag_of = |index: usize| tags.wrapping_add((index as u32).wrapping_mul(5));
    arena.set_u32(out, 0);
    let mut rc = host
        .call(arena, 0x19611c, &[handle_p, handle_tag, context_slot])
        .0;
    if rc >= 0x8000_0000 {
        return rc;
    }
    let context = arena.u32(context_slot);
    let heap = arena.u32(context.wrapping_add(4));
    let log = arena.u32(context.wrapping_add(16));
    let object = host.call(arena, 0x1963b8, &[heap, 1, 0x3b58]).0;
    if object == 0 {
        host.call(arena, 0x173dc0, &[log, ROM_SENTINEL, 0x61a8, 0]);
        return 0x84e0_200a;
    }
    arena.set_u32(object, context);
    arena.set_u32(object.wrapping_add(4), param);
    arena_leaf::run(arena, 0x195bec, &[object.wrapping_add(0x748), 0xff, 0x100]);
    arena_leaf::run(arena, 0x195bec, &[object.wrapping_add(0x9c0), 0xff, 0x78]);
    arena_leaf::run(arena, 0x195bec, &[object.wrapping_add(0xa40), 0xff, 0x20]);
    arena.set_u32(object.wrapping_add(0x8b0), 1);
    let psub = object.wrapping_add(0x3b54);
    loop {
        rc = host
            .call(arena, 0x24277c, &[obj, tag, rem_slot, cur_slot])
            .0;
        if rc >= 0x8000_0000 {
            break;
        }
        let mut rem = arena.u32(rem_slot);
        let mut cur = arena.u32(cur_slot);
        if arena_leaf::run(arena, 0x195d58, &[tag, tag_of(0)]) == 0 {
            let n = if rem > 256 { 256 } else { rem };
            arena_leaf::run(arena, 0x195ae8, &[object.wrapping_add(0x748), cur, n]);
        } else if arena_leaf::run(arena, 0x195d58, &[tag, tag_of(1)]) == 0 {
            let n = if rem <= 0x101 { rem } else { 0x101 };
            arena_leaf::run(arena, 0x195ae8, &[object.wrapping_add(0x243), cur, n]);
        } else if arena_leaf::run(arena, 0x195d58, &[tag, tag_of(2)]) == 0 {
            let n = if rem > 0x101 { 0x101 } else { rem };
            arena_leaf::run(arena, 0x195ae8, &[object.wrapping_add(0x344), cur, n]);
        } else if arena_leaf::run(arena, 0x195d58, &[tag, tag_of(3)]) == 0 {
            let n = if rem > 0x101 { 0x101 } else { rem };
            arena_leaf::run(arena, 0x195ae8, &[object.wrapping_add(0x445), cur, n]);
        } else if arena_leaf::run(arena, 0x195d58, &[tag, tag_of(4)]) == 0 {
        } else if arena_leaf::run(arena, 0x195d58, &[tag, tag_of(5)]) == 0 {
            let n = if rem > 64 { 64 } else { rem };
            arena_leaf::run(arena, 0x195ae8, &[object.wrapping_add(0x848), cur, n]);
        } else if arena_leaf::run(arena, 0x195d58, &[tag, tag_of(6)]) == 0 {
            arena_leaf::run(arena, 0x195ae8, &[wtmp, cur, 2]);
            let count = arena.u16(wtmp) as u32;
            cur = cur.wrapping_add(2);
            rem = rem.wrapping_sub(2);
            if count != 0 {
                let mut k = 0u32;
                let array = host
                    .call(arena, 0x1963b8, &[heap, count.wrapping_add(1), 4])
                    .0;
                arena.set_u32(object.wrapping_add(0x888), array);
                if array == 0 {
                    host.call(arena, 0x173dc0, &[log, ROM_SENTINEL, 0x61a8, 0]);
                    rc = 0x84e0_200a;
                    break;
                }
                while rc < 0x8000_0000 {
                    if k >= count {
                        break;
                    }
                    arena.set_u32(vlen, 0);
                    rc = arena_mid::read_voice_entry(arena, obj, cur, rem, 0, 0, vlen);
                    if rc >= 0x8000_0000 && (rc & 0x1fff) != 9 {
                        k = k.wrapping_add(1);
                        break;
                    }
                    let text = host.call(arena, 0x196394, &[heap, arena.u32(vlen)]).0;
                    arena.set_u32(array.wrapping_add(k.wrapping_mul(4)), text);
                    if text == 0 {
                        host.call(arena, 0x173dc0, &[log, ROM_SENTINEL, 0x61a8, 0]);
                        rc = 0x84e0_200a;
                        break;
                    }
                    rc = arena_mid::read_voice_entry(arena, obj, cur, rem, 0, text, vlen);
                    cur = cur.wrapping_add(arena.u32(vlen).wrapping_add(1));
                    rem = rem.wrapping_sub(arena.u32(vlen).wrapping_add(1));
                    k = k.wrapping_add(1);
                }
                if rc >= 0x8000_0000 {
                    break;
                }
            }
        } else if arena_leaf::run(arena, 0x195d58, &[tag, tag_of(7)]) == 0 {
            let mut i = 0u32;
            while i < rem.wrapping_div(2) {
                let option = arena.u16(cur.wrapping_add(i.wrapping_mul(2)));
                match option {
                    0 => arena.set_u32(object.wrapping_add(0x900), 2),
                    3 => arena.set_u32(object.wrapping_add(0x9f8), 1),
                    4 => arena.set_u32(object.wrapping_add(0x9d8), 1),
                    11 => arena.set_u32(object.wrapping_add(0x9ec), 1),
                    15 => arena.set_u32(object.wrapping_add(0xa24), 1),
                    19 => arena.set_u32(object.wrapping_add(0x9d4), 1),
                    27 => arena.set_u32(object.wrapping_add(0xa50), 1),
                    29 => arena.set_u32(object.wrapping_add(0xa54), 1),
                    31 => arena.set_u32(object.wrapping_add(0x8b8), 1),
                    33 => arena.set_u32(object.wrapping_add(0x8a8), 1),
                    35 => arena.set_u32(object.wrapping_add(0xa1c), 1),
                    36 => arena.set_u32(object.wrapping_add(0xa20), 1),
                    37 => arena.set_u32(object.wrapping_add(0xa18), 1),
                    38 => arena.set_u32(object.wrapping_add(0x8d4), 1),
                    39 => arena.set_u32(object.wrapping_add(0xa08), 1),
                    41 => arena.set_u32(object.wrapping_add(0x8b0), 1),
                    1 | 2 | 13 | 14 | 16 | 40 => {}
                    _ => {
                        host.call(
                            arena,
                            0x173dc0,
                            &[
                                log,
                                ROM_SENTINEL,
                                0x61b4,
                                ROM_SENTINEL,
                                ROM_SENTINEL,
                                name,
                                ROM_SENTINEL,
                                ROM_SENTINEL,
                                ROM_SENTINEL,
                                option as u32,
                            ],
                        );
                        rc = 0x84e0_2004;
                    }
                }
                i = i.wrapping_add(1);
            }
        } else if arena_leaf::run(arena, 0x195d58, &[tag, tag_of(8)]) == 0 {
            arena_leaf::run(arena, 0x195ae8, &[object.wrapping_add(0x994), cur, 2]);
        } else if arena_leaf::run(arena, 0x195d58, &[tag, tag_of(9)]) == 0 {
            arena_leaf::run(arena, 0x195ae8, &[object.wrapping_add(0xa3a), cur, 2]);
        } else if arena_leaf::run(arena, 0x195d58, &[tag, tag_of(10)]) == 0 {
            rc = host
                .call(arena, 0x2413c4, &[handle_p, handle_tag, name, obj, psub])
                .0;
        }
        if rc >= 0x8000_0000 {
            break;
        }
        arena.set_u32(rem_slot, rem);
        arena.set_u32(cur_slot, cur);
        rc = host.call(arena, 0x2422ac, &[obj]).0;
        if rc >= 0x8000_0000 {
            break;
        }
    }
    if (rc & 0x1fff) == 0x14 {
        rc = 0;
    }
    if rc >= 0x8000_0000 {
        return teardown_load(arena, host, object, heap, rc);
    }
    if rc >= 0x8000_0000 {
        return teardown_load(arena, host, object, heap, rc);
    }
    if arena.u32(psub) == 0 {
        host.call(
            arena,
            0x173dc0,
            &[
                log,
                ROM_SENTINEL,
                0x61b4,
                ROM_SENTINEL,
                ROM_SENTINEL,
                name,
                ROM_SENTINEL,
                ROM_SENTINEL,
            ],
        );
        return teardown_load(arena, host, object, heap, 0x84e0_2004);
    }
    let keep_8a8 = arena.u32(object.wrapping_add(0x8a8));
    arena.set_u8(object.wrapping_add(0x95b), 2);
    arena.set_u8(object.wrapping_add(0x98c), 0xff);
    arena.set_u8(object.wrapping_add(0x990), 0xff);
    arena.set_u8(object.wrapping_add(0x98d), 0xff);
    arena.set_u8(object.wrapping_add(0x98e), 0xff);
    arena.set_u8(object.wrapping_add(0x95a), 1);
    arena.set_u8(object.wrapping_add(0x95c), 1);
    arena.set_u8(object.wrapping_add(0x95d), 1);
    for i in 0..16u32 {
        arena.set_u8(object.wrapping_add(0x95f).wrapping_add(i), 1);
    }
    arena.set_u8(object.wrapping_add(0x975), 1);
    if keep_8a8 != 0 {
        arena.set_u8(object.wrapping_add(0x98f), 1);
    }
    if arena.u32(object.wrapping_add(0x9f8)) as i32 >= 0 {
        arena.set_u8(object.wrapping_add(0x977), 1);
        arena.set_u8(object.wrapping_add(0x978), 1);
    }
    if arena.u32(object.wrapping_add(0xa08)) as i32 >= 0 {
        arena.set_u8(object.wrapping_add(0x979), 1);
    }
    if arena.u32(object.wrapping_add(0xa18)) as i32 >= 0 {
        arena.set_u8(object.wrapping_add(0x97a), 1);
    }
    if arena.u32(object.wrapping_add(0xa1c)) as i32 >= 0 {
        arena.set_u8(object.wrapping_add(0x97b), 1);
    }
    if arena.u32(object.wrapping_add(0xa20)) as i32 >= 0 {
        arena.set_u8(object.wrapping_add(0x97c), 1);
    }
    if arena.u32(object.wrapping_add(0xa24)) as i32 >= 0 {
        arena.set_u8(object.wrapping_add(0x97d), 1);
    }
    if arena.u32(object.wrapping_add(0x9d4)) as i32 >= 0 {
        arena.set_u8(object.wrapping_add(0x971), 1);
        arena.set_u8(object.wrapping_add(0x972), 1);
    }
    if arena.u32(object.wrapping_add(0xa50)) as i32 >= 0
        || arena.u32(object.wrapping_add(0xa54)) as i32 >= 0
    {
        arena.set_u8(object.wrapping_add(0x970), 8);
    }
    if arena.u32(object.wrapping_add(0x9d8)) as i32 >= 0 {
        arena.set_u8(object.wrapping_add(0x973), 1);
        arena.set_u8(object.wrapping_add(0x974), 1);
    }
    if arena.u32(object.wrapping_add(0x8d4)) != 0 {
        arena.set_u8(object.wrapping_add(0x986), 9);
    }
    arena.set_u32(out, object);
    rc
}

fn teardown_load(
    arena: &mut Arena<'_>,
    host: &mut impl Host,
    object: u32,
    heap: u32,
    rc: u32,
) -> u32 {
    if object != 0 {
        let array = arena.u32(object.wrapping_add(0x888));
        if array != 0 {
            let mut j = 0u16;
            while arena.u32(array.wrapping_add((j as u32).wrapping_mul(4))) != 0 {
                host.call(
                    arena,
                    0x196428,
                    &[
                        heap,
                        arena.u32(array.wrapping_add((j as u32).wrapping_mul(4))),
                    ],
                );
                j = j.wrapping_add(1);
            }
            host.call(arena, 0x196428, &[heap, array]);
            arena.set_u32(object.wrapping_add(0x888), 0);
        }
        if arena.u32(object.wrapping_add(0x3b54)) != 0 {
            host.call(arena, 0x2412a0, &[arena.u32(object.wrapping_add(0x3b54))]);
            arena.set_u32(object.wrapping_add(0x3b54), 0);
        }
        let mut j = 0u16;
        while (j as u32) < arena.u32(object.wrapping_add(0x140)) {
            host.call(
                arena,
                0x243d50,
                &[arena.u32(
                    object
                        .wrapping_add(0xe0)
                        .wrapping_add((j as u32).wrapping_mul(4)),
                )],
            );
            j = j.wrapping_add(1);
        }
        arena.set_u32(object.wrapping_add(0x140), 0);
        host.call(arena, 0x196428, &[heap, object]);
    }
    rc
}

#[allow(clippy::too_many_arguments)]
pub fn process_stage(
    arena: &mut Arena<'_>,
    host: &mut impl Host,
    engine: u32,
    handle: u32,
    _a: u32,
    _b: u32,
    status: u32,
    scratch: u32,
    nodes: &mut [DecisionNode],
    rom: crate::rom_region::Rom,
) -> u32 {
    let context = arena.u32(engine.wrapping_add(ENG_CTX));
    let heap = arena.u32(context.wrapping_add(4));
    let log = arena.u32(context.wrapping_add(16));
    let pipe = arena.u32(engine.wrapping_add(ENG_PIPE));
    let length_slot = scratch;
    let eos_slot = scratch.wrapping_add(4);
    let text_slot = scratch.wrapping_add(8);
    let marker_slot = scratch.wrapping_add(12);
    let marker_len_slot = scratch.wrapping_add(16);
    let more_slot = scratch.wrapping_add(20);
    let done_slot = scratch.wrapping_add(21);
    let setup_scratch = scratch.wrapping_add(32);
    let duration_scratch = scratch.wrapping_add(208);
    let mut rc = host
        .call(arena, 0x176f94, &[engine, handle, 0x8e84, 0x7f8])
        .0;
    if rc >= 0x8000_0000 {
        return 0x84e0_2008;
    }
    arena.set_u8(status, 1);
    if arena.u32(engine.wrapping_add(ENG_PROF)) != 0 {
        host.call(
            arena,
            0x1985fc,
            &[
                arena.u32(engine.wrapping_add(ENG_PROF)),
                arena.u32(engine.wrapping_add(ENG_PROFH)),
                0,
            ],
        );
    }
    let state = arena.u8(engine.wrapping_add(ENG_STATE)) as i8 as i32;
    if state == 0 {
        if arena.u32(engine.wrapping_add(ENG_SYNTH)) == 0 {
            rc = stage_state_zero(
                arena,
                host,
                engine,
                context,
                heap,
                log,
                pipe,
                status,
                length_slot,
                eos_slot,
                text_slot,
                marker_slot,
                marker_len_slot,
                setup_scratch,
                duration_scratch,
                nodes,
            );
        }
    } else if state == 1 {
        let synth = arena.u32(engine.wrapping_add(ENG_SYNTH));
        rc = step_models(
            arena,
            host,
            synth,
            engine.wrapping_add(ENG_MODEL),
            engine.wrapping_add(ENG_CFG),
            more_slot,
        );
        if rc >= 0x8000_0000 {
            arena.set_u8(engine.wrapping_add(ENG_STATE), 0xff);
        } else {
            arena.set_u8(
                engine.wrapping_add(ENG_STATE),
                if arena.u8(more_slot) == 1 { 3 } else { 2 },
            );
            arena.set_u8(status, 4);
        }
    } else if state == 2 || state == 3 {
        rc = stage_state_emit(
            arena, host, engine, heap, log, pipe, status, scratch, done_slot, state, rom,
        );
    }
    if arena.u32(engine.wrapping_add(ENG_PROF)) != 0 {
        host.call(
            arena,
            0x19860c,
            &[
                arena.u32(engine.wrapping_add(ENG_PROF)),
                arena.u32(engine.wrapping_add(ENG_PROFH)),
                0,
            ],
        );
    }
    rc
}

#[allow(clippy::too_many_arguments)]
fn stage_state_zero(
    arena: &mut Arena<'_>,
    host: &mut impl Host,
    engine: u32,
    context: u32,
    heap: u32,
    log: u32,
    pipe: u32,
    status: u32,
    length_slot: u32,
    eos_slot: u32,
    text_slot: u32,
    marker_slot: u32,
    marker_len_slot: u32,
    setup_scratch: u32,
    duration_scratch: u32,
    nodes: &mut [DecisionNode],
) -> u32 {
    let input = arena.u32(engine.wrapping_add(ENG_IN));
    let input_h = arena.u32(engine.wrapping_add(ENG_IN).wrapping_add(4));
    let mut rc = host
        .call(
            arena,
            arena.u32(pipe.wrapping_add(PIPE_AVAIL)),
            &[input, input_h, length_slot, eos_slot],
        )
        .0;
    if rc >= 0x8000_0000 {
        host.call(
            arena,
            0x173dc0,
            &[log, ROM_SENTINEL, 25005, ROM_SENTINEL, ROM_SENTINEL, rc],
        );
        return rc;
    }
    let length = arena.u32(length_slot);
    if length == 0 {
        if arena.u8(eos_slot) == 0 {
            arena.set_u8(status, 0);
            return rc;
        }
        let markers_in = arena.u32(engine.wrapping_add(ENG_MARKERS_IN));
        let markers_in_h = arena.u32(engine.wrapping_add(ENG_MARKERS_IN).wrapping_add(4));
        let r = host
            .call(
                arena,
                arena.u32(pipe.wrapping_add(PIPE_READ)),
                &[markers_in, markers_in_h, marker_slot, marker_len_slot],
            )
            .0;
        if r >= 0x8000_0000 {
            host.call(
                arena,
                0x173dc0,
                &[log, ROM_SENTINEL, 25005, ROM_SENTINEL, ROM_SENTINEL, r],
            );
        }
        let marker_len = arena.u32(marker_len_slot);
        if marker_len != 0 {
            let markers_out = arena.u32(engine.wrapping_add(ENG_MARKERS_OUT));
            let markers_out_h = arena.u32(engine.wrapping_add(ENG_MARKERS_OUT).wrapping_add(4));
            let r = host
                .call(
                    arena,
                    arena.u32(pipe.wrapping_add(PIPE_WRITE)),
                    &[
                        markers_out,
                        markers_out_h,
                        arena.u32(marker_slot),
                        marker_len,
                    ],
                )
                .0;
            if r >= 0x8000_0000 {
                host.call(
                    arena,
                    0x173dc0,
                    &[log, ROM_SENTINEL, 25005, ROM_SENTINEL, ROM_SENTINEL, r],
                );
            }
        }
        rc = host
            .call(
                arena,
                arena.u32(pipe.wrapping_add(PIPE_CONSUME)),
                &[markers_in, markers_in_h, marker_len],
            )
            .0;
        if rc >= 0x8000_0000 {
            host.call(
                arena,
                0x173dc0,
                &[log, ROM_SENTINEL, 25005, ROM_SENTINEL, ROM_SENTINEL, rc],
            );
        }
        if arena.u32(engine.wrapping_add(ENG_FRAMES_OUT)) != 0 {
            let frames_out = arena.u32(engine.wrapping_add(ENG_FRAMES_OUT));
            let frames_out_h = arena.u32(engine.wrapping_add(ENG_FRAMES_OUT).wrapping_add(4));
            rc = host
                .call(
                    arena,
                    arena.u32(pipe.wrapping_add(PIPE_CLOSE)),
                    &[frames_out, frames_out_h],
                )
                .0;
            let (p, h) = host.call(arena, 0x176fd4, &[]);
            arena.set_u32(engine.wrapping_add(ENG_FRAMES_OUT), p);
            arena.set_u32(engine.wrapping_add(ENG_FRAMES_OUT).wrapping_add(4), h);
        }
        if arena.u32(engine.wrapping_add(ENG_MARKERS_OUT)) != 0 {
            let markers_out = arena.u32(engine.wrapping_add(ENG_MARKERS_OUT));
            let markers_out_h = arena.u32(engine.wrapping_add(ENG_MARKERS_OUT).wrapping_add(4));
            rc = host
                .call(
                    arena,
                    arena.u32(pipe.wrapping_add(PIPE_CLOSE)),
                    &[markers_out, markers_out_h],
                )
                .0;
            let (p, h) = host.call(arena, 0x176fd4, &[]);
            arena.set_u32(engine.wrapping_add(ENG_MARKERS_OUT), p);
            arena.set_u32(engine.wrapping_add(ENG_MARKERS_OUT).wrapping_add(4), h);
        }
        arena.set_u8(status, 1);
        return rc;
    }
    rc = host
        .call(
            arena,
            arena.u32(pipe.wrapping_add(PIPE_READ)),
            &[input, input_h, text_slot, length_slot],
        )
        .0;
    if rc >= 0x8000_0000 {
        host.call(
            arena,
            0x173dc0,
            &[
                log,
                ROM_SENTINEL,
                25006,
                ROM_SENTINEL,
                ROM_SENTINEL,
                ROM_SENTINEL,
                ROM_SENTINEL,
                rc,
            ],
        );
        return rc;
    }
    let text = arena.u32(text_slot);
    let length = arena.u32(length_slot);
    let markers_in = arena.u32(engine.wrapping_add(ENG_MARKERS_IN));
    let markers_in_h = arena.u32(engine.wrapping_add(ENG_MARKERS_IN).wrapping_add(4));
    rc = host
        .call(
            arena,
            arena.u32(pipe.wrapping_add(PIPE_READ)),
            &[
                markers_in,
                markers_in_h,
                marker_slot,
                engine.wrapping_add(ENG_MARKER_BYTES),
            ],
        )
        .0;
    if rc >= 0x8000_0000 {
        host.call(
            arena,
            0x173dc0,
            &[
                log,
                ROM_SENTINEL,
                25006,
                ROM_SENTINEL,
                ROM_SENTINEL,
                ROM_SENTINEL,
                ROM_SENTINEL,
                rc,
            ],
        );
        return rc;
    }
    arena.set_u32(engine.wrapping_add(ENG_MARKER_DONE), 0);
    let new_synth = host.call(arena, 0x196394, &[heap, 0x3aa4]).0;
    arena.set_u32(engine.wrapping_add(ENG_SYNTH), new_synth);
    if new_synth == 0 {
        host.call(arena, 0x173dc0, &[log, ROM_SENTINEL, 25000, 0]);
        rc = 0x84e0_200a;
    } else {
        host.call(
            arena,
            0x2a4740,
            &[context, new_synth, engine.wrapping_add(ENG_BLOCKS)],
        );
        rc = host
            .call(
                arena,
                0x2a4510,
                &[context, new_synth, engine.wrapping_add(ENG_MODEL)],
            )
            .0;
        if rc >= 0x8000_0000 {
            return stage_fail(arena, engine, status, -1, rc);
        }
        if arena.u32(engine.wrapping_add(ENG_PROF)) != 0 {
            host.call(
                arena,
                0x1985fc,
                &[
                    arena.u32(engine.wrapping_add(ENG_PROF)),
                    arena.u32(engine.wrapping_add(ENG_PROFH)),
                    1,
                ],
            );
        }
        let sptchk = arena.u32(engine.wrapping_add(ENG_SPTCHK));
        let sptchk_h = arena.u32(engine.wrapping_add(ENG_SPTCHK).wrapping_add(4));
        let sgt = arena.u32(engine.wrapping_add(ENG_SGT));
        let sgt_h = arena.u32(engine.wrapping_add(ENG_SGT).wrapping_add(4));
        rc = setup_features(
            arena,
            host,
            arena.u32(engine.wrapping_add(ENG_F10)),
            text,
            length,
            arena.u32(marker_slot),
            arena
                .u32(engine.wrapping_add(ENG_MARKER_BYTES))
                .wrapping_div(24),
            sptchk,
            sptchk_h,
            sgt,
            sgt_h,
            arena.u32(engine.wrapping_add(ENG_WAIT)) as i32,
            new_synth.wrapping_add(SYN_ARENA),
            new_synth.wrapping_add(SYN_FEATURES),
            engine.wrapping_add(ENG_MARKERS),
            engine.wrapping_add(ENG_MARKER_COUNT),
            setup_scratch,
        );
        if arena.u32(engine.wrapping_add(ENG_PROF)) != 0 {
            host.call(
                arena,
                0x19860c,
                &[
                    arena.u32(engine.wrapping_add(ENG_PROF)),
                    arena.u32(engine.wrapping_add(ENG_PROFH)),
                    1,
                ],
            );
        }
        if rc >= 0x8000_0000 {
            return stage_fail(arena, engine, status, -1, rc);
        }
        arena.set_u32(
            new_synth.wrapping_add(SYN_F20),
            engine.wrapping_add(ENG_F78C),
        );
        arena.set_u32(
            new_synth.wrapping_add(0x55c),
            arena.u32(engine.wrapping_add(ENG_PROF)),
        );
        arena.set_u32(
            new_synth.wrapping_add(0x560),
            arena.u32(engine.wrapping_add(ENG_PROFH)),
        );
        if arena.u32(engine.wrapping_add(ENG_PROF)) != 0 {
            host.call(
                arena,
                0x1985fc,
                &[
                    arena.u32(engine.wrapping_add(ENG_PROF)),
                    arena.u32(engine.wrapping_add(ENG_PROFH)),
                    2,
                ],
            );
        }
        rc = build_models(
            arena,
            host,
            new_synth,
            engine.wrapping_add(ENG_MODEL),
            engine.wrapping_add(ENG_CFG),
            nodes,
            duration_scratch,
        );
        if arena.u32(engine.wrapping_add(ENG_PROF)) != 0 {
            host.call(
                arena,
                0x19860c,
                &[
                    arena.u32(engine.wrapping_add(ENG_PROF)),
                    arena.u32(engine.wrapping_add(ENG_PROFH)),
                    2,
                ],
            );
        }
        if rc >= 0x8000_0000 {
            return stage_fail(arena, engine, status, -1, rc);
        }
        rc = host
            .call(
                arena,
                arena.u32(pipe.wrapping_add(PIPE_CONSUME)),
                &[input, input_h, length],
            )
            .0;
        if rc >= 0x8000_0000 {
            host.call(
                arena,
                0x173dc0,
                &[
                    log,
                    ROM_SENTINEL,
                    25007,
                    ROM_SENTINEL,
                    ROM_SENTINEL,
                    ROM_SENTINEL,
                    ROM_SENTINEL,
                    rc,
                ],
            );
            return stage_fail(arena, engine, status, -1, rc);
        }
        if arena.u32(engine.wrapping_add(ENG_SYNTH)) != 0 {
            arena.set_u8(engine.wrapping_add(ENG_STATE), 1);
        } else {
            arena.set_u8(engine.wrapping_add(ENG_STATE), 0xff);
        }
        arena.set_u8(status, 4);
    }
    rc
}

fn stage_fail(arena: &mut Arena<'_>, engine: u32, status: u32, state: i8, rc: u32) -> u32 {
    arena.set_u8(engine.wrapping_add(ENG_STATE), state as u8);
    arena.set_u8(status, 4);
    rc
}

fn stage_state_emit(
    arena: &mut Arena<'_>,
    host: &mut impl Host,
    engine: u32,
    heap: u32,
    log: u32,
    pipe: u32,
    status: u32,
    scratch: u32,
    done_slot: u32,
    state: i32,
    rom: crate::rom_region::Rom,
) -> u32 {
    let synth = arena.u32(engine.wrapping_add(ENG_SYNTH));
    if arena.u32(engine.wrapping_add(ENG_PROF)) != 0 {
        host.call(
            arena,
            0x1985fc,
            &[
                arena.u32(engine.wrapping_add(ENG_PROF)),
                arena.u32(engine.wrapping_add(ENG_PROFH)),
                5,
            ],
        );
    }
    let values = scratch.wrapping_add(216);

    let wcur = crate::rom_region::addr(rom, 0x91728c);
    let wprev = crate::rom_region::addr(rom, 0x9172a8);
    let mut blend_current = [0i32; 7];
    let mut blend_previous = [0i32; 7];
    for i in 0..7 {
        blend_current[i] =
            arena.u32(wcur.wrapping_add((i as u32).wrapping_mul(4))) as i32;
        blend_previous[i] =
            arena.u32(wprev.wrapping_add((i as u32).wrapping_mul(4))) as i32;
    }
    let mut rc = synthesize_phones(
        arena,
        host,
        synth,
        engine.wrapping_add(ENG_MODEL),
        engine.wrapping_add(ENG_CFG),
        done_slot,
        values,
        &blend_current,
        &blend_previous,
    );
    if rc < 0x8000_0000 && arena.u8(done_slot) == 1 {
        if state == 2 {
            arena.set_u8(engine.wrapping_add(ENG_STATE), 1);
        } else if state == 3 {
            arena.set_u8(engine.wrapping_add(ENG_STATE), 4);
        }
    }
    if arena.u32(engine.wrapping_add(ENG_PROF)) != 0 {
        host.call(
            arena,
            0x19860c,
            &[
                arena.u32(engine.wrapping_add(ENG_PROF)),
                arena.u32(engine.wrapping_add(ENG_PROFH)),
                5,
            ],
        );
    }
    if rc >= 0x8000_0000 {
        arena.set_u8(status, 2);
        return emit_epilogue(arena, host, engine, rc);
    }
    if arena.u32(engine.wrapping_add(ENG_MARKERS)) != 0
        && arena.u32(engine.wrapping_add(ENG_MARKER_COUNT))
            > arena.u32(engine.wrapping_add(ENG_MARKER_DONE))
    {
        let mut marker_done = arena.u32(engine.wrapping_add(ENG_MARKER_DONE));
        let marker_count = arena.u32(engine.wrapping_add(ENG_MARKER_COUNT));
        let markers = arena.u32(engine.wrapping_add(ENG_MARKERS));
        let mut m = markers.wrapping_add(marker_done.wrapping_mul(24));
        let mut i = 0u32;
        let mut synth_cur = arena.u32(synth.wrapping_add(SYN_CUR));
        while synth_cur != 0 && marker_done.wrapping_add(i) < marker_count {
            if arena.u16(m) != 33 {
                arena.set_u32(
                    m.wrapping_add(0xc),
                    arena.u32(engine.wrapping_add(ENG_F7F0)),
                );
                arena.set_u32(m.wrapping_add(0x10), 0);
            } else {
                let pos = (arena.u32(synth.wrapping_add(SYN_POS)) as i32)
                    .wrapping_mul(arena.u32(engine.wrapping_add(ENG_F73C)) as i32)
                    .wrapping_add(arena.u32(engine.wrapping_add(ENG_F7EC)) as i32);
                let dur = arena.u32(synth_cur.wrapping_add(0x10)) as i32;
                let len = dur.wrapping_mul(arena.u32(engine.wrapping_add(ENG_F73C)) as i32);
                arena.set_u32(m.wrapping_add(0xc), pos as u32);
                arena.set_u32(m.wrapping_add(0x10), len as u32);
                arena.set_u32(
                    engine.wrapping_add(ENG_F7F0),
                    (pos.wrapping_add(len)) as u32,
                );
                arena.set_u32(
                    synth.wrapping_add(SYN_POS),
                    ((arena.u32(synth.wrapping_add(SYN_POS)) as i32).wrapping_add(dur)) as u32,
                );
                synth_cur = arena.u32(synth_cur.wrapping_add(0x18));
                arena.set_u32(synth.wrapping_add(SYN_CUR), synth_cur);
            }
            m = m.wrapping_add(24);
            i = i.wrapping_add(1);
        }
        if arena.u8(engine.wrapping_add(ENG_STATE)) == 4 {
            while marker_done.wrapping_add(i) < marker_count {
                arena.set_u32(
                    m.wrapping_add(0xc),
                    arena.u32(engine.wrapping_add(ENG_F7F0)),
                );
                arena.set_u32(m.wrapping_add(0x10), 0);
                i = i.wrapping_add(1);
                m = m.wrapping_add(24);
            }
        }
        if i != 0 {
            let frames_out = arena.u32(engine.wrapping_add(ENG_MARKERS_OUT));
            let frames_out_h = arena.u32(engine.wrapping_add(ENG_MARKERS_OUT).wrapping_add(4));
            rc = host
                .call(
                    arena,
                    arena.u32(pipe.wrapping_add(PIPE_WRITE)),
                    &[
                        frames_out,
                        frames_out_h,
                        markers.wrapping_add(marker_done.wrapping_mul(24)),
                        i.wrapping_mul(24),
                    ],
                )
                .0;
            if rc >= 0x8000_0000 {
                host.call(
                    arena,
                    0x173dc0,
                    &[
                        log,
                        ROM_SENTINEL,
                        25008,
                        ROM_SENTINEL,
                        ROM_SENTINEL,
                        ROM_SENTINEL,
                        ROM_SENTINEL,
                        rc,
                    ],
                );
            }
        }
        marker_done = marker_done.wrapping_add(i);
        arena.set_u32(engine.wrapping_add(ENG_MARKER_DONE), marker_done);
        if rc >= 0x8000_0000 {
            arena.set_u8(status, 2);
            return emit_epilogue(arena, host, engine, rc);
        }
    }
    let frames_out = arena.u32(engine.wrapping_add(ENG_FRAMES_OUT));
    let frames_out_h = arena.u32(engine.wrapping_add(ENG_FRAMES_OUT).wrapping_add(4));
    rc = host
        .call(
            arena,
            arena.u32(pipe.wrapping_add(PIPE_WRITE)),
            &[frames_out, frames_out_h, engine.wrapping_add(ENG_SYNTH), 4],
        )
        .0;
    if rc >= 0x8000_0000 {
        host.call(
            arena,
            0x173dc0,
            &[
                log,
                ROM_SENTINEL,
                25008,
                ROM_SENTINEL,
                ROM_SENTINEL,
                ROM_SENTINEL,
                ROM_SENTINEL,
                rc,
            ],
        );
        arena.set_u8(status, 2);
        return emit_epilogue(arena, host, engine, rc);
    }
    if arena.u8(engine.wrapping_add(ENG_STATE)) != 4 {
        arena.set_u8(status, 2);
        return emit_epilogue(arena, host, engine, rc);
    }
    let r = host
        .call(arena, 0x2a762c, &[synth, engine.wrapping_add(ENG_CFG)])
        .0;
    if r >= 0x8000_0000 {
        rc = r;
    }
    arena_mid::release_shared_stream(arena, host, synth);
    let lookup = arena.u32(synth.wrapping_add(SYN_F20));
    arena_mid::release_frame_lookup(arena, host, lookup);
    arena.set_u32(synth.wrapping_add(SYN_F20), 0);
    if arena.u16(synth.wrapping_add(0x1c)) == 0 {
        host.call(arena, 0x196428, &[heap, synth]);
    }
    arena.set_u32(engine.wrapping_add(ENG_SYNTH), 0);
    if rc < 0x8000_0000 && arena.u32(engine.wrapping_add(ENG_MARKER_BYTES)) != 0 {
        let markers_in = arena.u32(engine.wrapping_add(ENG_MARKERS_IN));
        let markers_in_h = arena.u32(engine.wrapping_add(ENG_MARKERS_IN).wrapping_add(4));
        rc = host
            .call(
                arena,
                arena.u32(pipe.wrapping_add(PIPE_CONSUME)),
                &[
                    markers_in,
                    markers_in_h,
                    arena.u32(engine.wrapping_add(ENG_MARKER_BYTES)),
                ],
            )
            .0;
        if rc >= 0x8000_0000 {
            host.call(
                arena,
                0x173dc0,
                &[
                    log,
                    ROM_SENTINEL,
                    25007,
                    ROM_SENTINEL,
                    ROM_SENTINEL,
                    ROM_SENTINEL,
                    ROM_SENTINEL,
                    rc,
                ],
            );
        }
    }
    if arena.u32(engine.wrapping_add(ENG_MARKERS)) != 0 {
        host.call(
            arena,
            0x196428,
            &[heap, arena.u32(engine.wrapping_add(ENG_MARKERS))],
        );
        arena.set_u32(engine.wrapping_add(ENG_MARKERS), 0);
        arena.set_u32(engine.wrapping_add(ENG_MARKER_DONE), 0);
        arena.set_u32(engine.wrapping_add(ENG_MARKER_COUNT), 0);
        arena.set_u32(engine.wrapping_add(ENG_MARKER_BYTES), 0);
        arena.set_u32(
            engine.wrapping_add(ENG_F7EC),
            arena.u32(engine.wrapping_add(ENG_F7F0)),
        );
    }
    arena.set_u8(status, 1);
    arena.set_u8(engine.wrapping_add(ENG_STATE), 0);
    if rc >= 0x8000_0000 {
        arena.set_u8(engine.wrapping_add(ENG_STATE), 0xff);
    }
    emit_epilogue(arena, host, engine, rc)
}

fn emit_epilogue(arena: &mut Arena<'_>, _host: &mut impl Host, engine: u32, rc: u32) -> u32 {
    arena_leaf::run(
        arena,
        0x19861c,
        &[
            arena.u32(engine.wrapping_add(ENG_PROF)),
            arena.u32(engine.wrapping_add(ENG_PROFH)),
            8,
        ],
    );
    rc
}
