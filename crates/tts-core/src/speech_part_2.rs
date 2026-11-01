
use crate::{boot::Memory, frontend};

pub trait Host {
    fn call(&mut self, memory: &mut Memory<'_>, method: u32, args: &[u32]) -> u32;
}

pub struct Environment {
    pub cached_handle: [u8; 8],
    pub default_context: u32,
    pub input_types: [u32; 2],
    pub output_types: [u32; 2],
}
fn close_slots(
    m: &mut Memory<'_>,
    p: u32,
    vt: u32,
    start: u32,
    n: u32,
    mut rc: u32,
    h: &mut impl Host,
) -> u32 {
    for i in 0..n {
        let at = p + start + i * 8;
        if m.word(at) != 0 {
            rc = h.call(
                m,
                m.word(m.word(p + vt) + 0x24),
                &[m.word(at), m.word(at + 4)],
            );
            m.put_word(at, 0);
            m.put_word(at + 4, 0);
        }
    }
    rc
}

pub fn fn_1f1138(m: &mut Memory<'_>, p: u32, tag: u32, h: &mut impl Host) -> u32 {
    let rc = frontend::validate_handle(p != 0, tag, 0xf488, 0x54);
    if rc >= 0x80000000 {
        return 0x8c902008;
    }
    close_slots(m, p, 12, 0x28, 4, rc, h)
}

pub fn fn_205e58(m: &mut Memory<'_>, p: u32, tag: u32, h: &mut impl Host) -> u32 {
    let rc = frontend::validate_handle(p != 0, tag, 0xf38a, 0xb8);
    if rc >= 0x80000000 {
        return 0x8a802008;
    }
    close_slots(m, p, 24, 28, 4, rc, h)
}

pub fn fn_21cd54(m: &mut Memory<'_>, p: u32, tag: u32, h: &mut impl Host) -> u32 {
    let rc = frontend::validate_handle(p != 0, tag, 0xf389, 0x4c);
    if rc >= 0x80000000 {
        return 0x8a602008;
    }
    close_slots(m, p, 28, 32, 4, rc, h)
}

pub fn fn_219c8c(
    m: &mut Memory<'_>,
    p: u32,
    tag: u32,
    input: [u32; 2],
    e: &Environment,
    h: &mut impl Host,
) -> u32 {
    if frontend::validate_handle(p != 0, tag, 0xf389, 0x4c) >= 0x80000000 {
        return 0x8a602008;
    }
    let mut i = 0;
    let mut j = 0;
    let mut rc;
    loop {
        rc = h.call(
            m,
            m.word(m.word(p + 28) + 32),
            &[
                input[0],
                input[1],
                0xa6,
                e.input_types[i as usize],
                0,
                0,
                p + 32 + i * 8,
            ],
        );
        if rc >= 0x80000000 {
            break;
        }
        i += 1;
        if i == 2 {
            loop {
                let mode = if j == 1 {
                    h.call(m, m.word(m.word(p + 28) + 76), &[])
                } else {
                    0
                };
                rc = h.call(
                    m,
                    m.word(m.word(p + 28) + 32),
                    &[
                        input[0],
                        input[1],
                        0xa6,
                        e.output_types[j as usize],
                        mode,
                        1,
                        p + 48 + j * 8,
                    ],
                );
                if rc >= 0x80000000 {
                    break;
                }
                j += 1;
                if j == 2 {
                    rc = h.call(
                        m,
                        m.word(m.word(p + 24) + 56),
                        &[m.word(p + 16), m.word(p + 20), 0, 1],
                    );
                    if rc < 0x80000000 {
                        rc = h.call(
                            m,
                            m.word(m.word(p + 24) + 56),
                            &[m.word(p + 16), m.word(p + 20), 0, 0],
                        );
                    }
                    if rc < 0x80000000 {
                        return rc;
                    }
                    break;
                }
            }
            break;
        }
    }
    close_slots(m, p, 28, 32, i, 0, h);
    close_slots(m, p, 28, 48, j, 0, h);
    rc
}
fn context(m: &Memory<'_>, p: u32, tag: u32, expected: u32, size: u32, e: &Environment) -> u32 {
    let rc = frontend::validate_handle(p != 0, tag, expected, size);
    if rc >= 0x80000000 {
        return if rc & 0x1fff == 8 { 0x88102008 } else { rc };
    }
    let a = m.word(p);
    let b = m.word(p + 4);
    let cached_a = u32::from_le_bytes(e.cached_handle[..4].try_into().unwrap());
    let cached_b = u32::from_le_bytes(e.cached_handle[4..].try_into().unwrap());
    let pointed = if [a, b] != [cached_a, cached_b] && a != 0 && b == 0x1e61 {
        Some(u16::from_le_bytes([m.byte(a), m.byte(a + 1)]))
    } else {
        None
    };
    let mut out = 0;
    frontend::resolve_engine_context(a, pointed, b, &e.cached_handle, e.default_context, &mut out)
}
fn release_blobs(
    m: &mut Memory<'_>,
    p: u32,
    vt: u32,
    start: u32,
    e: &Environment,
    h: &mut impl Host,
) {
    for i in 0..3 {
        let at = p + start + i * 8;
        if m.word(at) != 0 {
            h.call(
                m,
                m.word(m.word(p + vt) + 36),
                &[m.word(at), m.word(at + 4)],
            );
            let mut pair = [0u8; 8];
            frontend::load_pair(&mut pair, &e.cached_handle);
            let off = m.offset(at);
            m.bytes[off..off + 8].copy_from_slice(&pair);
        }
    }
}

pub fn fn_2318f0(m: &mut Memory<'_>, p: u32, tag: u32, e: &Environment, h: &mut impl Host) -> u32 {
    let rc = context(m, p, tag, 0x354, 300, e);
    if rc < 0x80000000 {
        release_blobs(m, p, 0x110, 0x114, e, h);
    }
    rc
}

pub fn fn_23220c(m: &mut Memory<'_>, p: u32, tag: u32, e: &Environment, h: &mut impl Host) -> u32 {
    let rc = context(m, p, tag, 0x356, 0x24, e);
    if rc < 0x80000000 {
        release_blobs(m, p, 8, 12, e, h);
    }
    rc
}

pub fn fn_1ef464(
    m: &mut Memory<'_>,
    p: u32,
    tag: u32,
    input: [u32; 2],
    e: &Environment,
    h: &mut impl Host,
) -> u32 {
    if frontend::validate_handle(p != 0, tag, 0xf488, 0x54) >= 0x80000000 {
        return 0x8c902008;
    }
    let mut i = 0;
    let mut j = 0;
    let mut rc;
    loop {
        rc = h.call(
            m,
            m.word(m.word(p + 12) + 32),
            &[
                input[0],
                input[1],
                0xc9,
                e.input_types[i as usize],
                0,
                0,
                p + 0x28 + i * 8,
            ],
        );
        if rc >= 0x80000000 {
            break;
        }
        i += 1;
        if i == 2 {
            loop {
                let mode = if j == 1 {
                    h.call(m, m.word(m.word(p + 12) + 76), &[])
                } else {
                    0
                };
                rc = h.call(
                    m,
                    m.word(m.word(p + 12) + 32),
                    &[
                        input[0],
                        input[1],
                        0xc9,
                        e.output_types[j as usize],
                        mode,
                        1,
                        p + 0x38 + j * 8,
                    ],
                );
                if rc >= 0x80000000 {
                    break;
                }
                j += 1;
                if j == 2 {
                    return rc;
                }
            }
            break;
        }
    }
    close_slots(m, p, 12, 0x28, i, 0, h);
    close_slots(m, p, 12, 0x38, j, 0, h);
    rc
}

pub fn fn_226ef8(m: &mut Memory<'_>, _a: u32, _b: u32, p: u32) -> u32 {
    if p != 0 && m.word(p) != 0 {
        let at = m.word(p) + 0x1c;
        let mut refs = u16::from_le_bytes([m.byte(at), m.byte(at + 1)]);
        crate::selection::retain_selection(&mut refs);
        m.put_half(at, refs);
    }
    1
}

pub fn fn_24eac4(
    m: &mut Memory<'_>,
    p: u32,
    list: u32,
    count: u32,
    a: i32,
    b: i32,
    all: u32,
) -> u32 {
    let container = m.word(p + 0x44);
    let mut i = 0u16;
    loop {
        let table = m.word(container);
        let mut offset = 0;
        let desc = if table == 0 {
            None
        } else {
            Some(frontend::ElementTable {
                data: &[],
                elem_size: u16::from_le_bytes([m.byte(table + 4), m.byte(table + 5)]),
                count: m.word(table + 20),
            })
        };
        if frontend::element_at(desc.as_ref(), i as u32, Some(&mut offset)) == 0 {
            break;
        }
        let range = m.word(table).wrapping_add(offset);
        if all != 0
            || ((u16::from_le_bytes([m.byte(range), m.byte(range + 1)]) as i16 as i32) <= a
                && b <= (u16::from_le_bytes([m.byte(range + 2), m.byte(range + 3)]) as i16 as i32))
        {
            let n = u16::from_le_bytes([m.byte(count), m.byte(count + 1)]);
            if n as u32 + 1 < 0x1f5 {
                m.put_half(list + n as u32 * 2, i);
                let n = u16::from_le_bytes([m.byte(count), m.byte(count + 1)]);
                m.put_half(count, n.wrapping_add(1));
            }
        }
        i = i.wrapping_add(1);
    }
    0
}

pub fn fn_201eb0(m: &mut Memory<'_>, p: u32, tag: u32, types: [&[u8]; 3]) -> u32 {
    if frontend::validate_handle(p != 0, tag, 0x2f0, 0x30) >= 0x80000000 {
        return 0;
    }
    let mut node = m.word(p + 28);
    while node != 0 {

        let mut nodes = [crate::frontend_resources::ResourceListNode {
            key: m.word(node),
            value: m.word(node + 4),
            next: None,
        }];
        let view = crate::frontend_resources::ResourceList {
            nodes: &mut nodes,
            head: Some(0),
            tail: Some(0),
        };
        let entry = view.next(&mut None);
        if entry == 0 {
            break;
        }
        let s = m.word(entry + 8);
        let off = m.offset(s);
        if types
            .iter()
            .any(|t| frontend::compare_str(&m.bytes[off..], t) == 0)
            && m.byte(entry + 16) == 1
        {
            return 1;
        }
        node = m.word(node + 8);
    }
    u32::from(m.word(p + 40) != 0)
}

pub fn fn_1e5af0(m: &mut Memory<'_>, p: u32, tag: u32, h: &mut impl Host) -> u32 {
    if frontend::validate_handle(p != 0, tag, 0x3815, 0x30) >= 0x80000000 {
        return 0x81102008;
    }
    if p == 0 {
        return 0x81102007;
    }
    let rc = h.call(m, 0x264578, &[m.word(p + 40)]);
    if rc >= 0x80000000 {
        return rc;
    }
    for start in [28, 20, 12] {
        if m.word(p + start) != 0 {
            let rc = h.call(
                m,
                m.word(m.word(p + 8) + 36),
                &[m.word(p + start), m.word(p + start + 4)],
            );
            if rc >= 0x80000000 {
                return rc;
            }
            m.put_word(p + start, 0);
            m.put_word(p + start + 4, 0);
        }
    }
    0
}

pub struct MarkerTypes {
    pub sync: u32,
    pub markers: u32,
    pub category: u32,
    pub content_type: u32,
    pub format: u32,
}

pub fn fn_2319f0(
    m: &mut Memory<'_>,
    p: u32,
    tag: u32,
    content: [u32; 2],
    e: &Environment,
    t: &MarkerTypes,
    scratch: u32,
    h: &mut (impl Host + crate::boot_part_b::FormatHost),
) -> u32 {
    let rc = context(m, p, tag, 0x356, 0x24, e);
    if rc >= 0x80000000 {
        return rc;
    }
    let mut ctx = 0;
    let a = m.word(p);
    let b = m.word(p + 4);
    let cached = [
        u32::from_le_bytes(e.cached_handle[..4].try_into().unwrap()),
        u32::from_le_bytes(e.cached_handle[4..].try_into().unwrap()),
    ];
    let pointed = if [a, b] != cached && a != 0 && b == 0x1e61 {
        Some(u16::from_le_bytes([m.byte(a), m.byte(a + 1)]))
    } else {
        None
    };
    frontend::resolve_engine_context(a, pointed, b, &e.cached_handle, e.default_context, &mut ctx);
    let rc = Host::call(
        h,
        m,
        m.word(m.word(p + 8) + 32),
        &[content[0], content[1], 0x81, t.sync, 0, 0, p + 12],
    );
    if rc >= 0x80000000 {
        return 0;
    }
    for (offset, mode, line) in [(20, 0, 0x3ab2), (28, 1, 0x3ab3)] {
        let rc = Host::call(
            h,
            m,
            m.word(m.word(p + 8) + 32),
            &[content[0], content[1], 0x81, t.markers, 0, mode, p + offset],
        );
        if rc >= 0x80000000 {
            let off = m.offset(scratch);
            let mut saved = [0u8; 28];
            saved.copy_from_slice(&m.bytes[off..off + 28]);
            m.put_word(scratch, t.content_type);
            m.put_word(scratch + 4, t.markers);
            crate::render_part_a::error_log(
                m,
                h,
                m.word(ctx + 16),
                t.category,
                line,
                t.format,
                scratch,
                scratch + 8,
            );
            m.bytes[off..off + 28].copy_from_slice(&saved);
            return rc;
        }
        if mode == 1 {
            return rc;
        }
    }
    unreachable!()
}

use crate::{log_live, normalization as norm};

pub struct SpeechLiterals {
    pub text: [u32; 16],
    pub end: u32,

    pub errors: [u32; 5],
}

pub trait SpeechHost: Host + log_live::LogMethodsM + crate::boot_part_b::FormatHost {}
impl<T: Host + log_live::LogMethodsM + crate::boot_part_b::FormatHost> SpeechHost for T {}
fn half(m: &Memory<'_>, p: u32) -> u16 {
    u16::from_le_bytes([m.byte(p), m.byte(p + 1)])
}
fn live_log(
    m: &mut Memory<'_>,
    p: u32,
    category: u32,
    fmt: u32,
    args: &[u32],
    s: u32,
    h: &mut impl SpeechHost,
) {
    let mut saved = [0u8; 32];
    let off = m.offset(s);
    saved.copy_from_slice(&m.bytes[off..off + 32]);
    for (i, v) in args.iter().enumerate() {
        m.put_word(s + i as u32 * 4, *v);
    }
    log_live::fn_1738e8(m, m.word(m.word(p) + 16), category, 4, 0, fmt, s, h);
    m.bytes[off..off + 32].copy_from_slice(&saved);
}
fn diagnostic(
    m: &mut Memory<'_>,
    p: u32,
    category: u32,
    code: u32,
    fmt: u32,
    args: &[u32],
    s: u32,
    h: &mut impl SpeechHost,
) {
    let mut saved = [0u8; 52];
    let off = m.offset(s);
    saved.copy_from_slice(&m.bytes[off..off + 52]);
    for (i, v) in args.iter().enumerate() {
        m.put_word(s + i as u32 * 4, *v);
    }
    crate::render_part_a::error_log(m, h, m.word(m.word(p) + 16), category, code, fmt, s, s + 32);
    m.bytes[off..off + 52].copy_from_slice(&saved);
}

pub fn fn_24eb78(
    m: &mut Memory<'_>,
    p: u32,
    t: &SpeechLiterals,
    s: u32,
    h: &mut impl SpeechHost,
) -> u32 {
    assert_eq!(m.base, 0);
    let domains = m.word(p + 0x44);
    let mut i = 0u16;
    loop {
        let Some(r) = norm::table_row(m.bytes, m.word(domains), i as u32) else {
            break;
        };
        live_log(
            m,
            p,
            m.word(p + 0x48),
            t.text[0],
            &[
                half(m, r) as i16 as i32 as u32,
                half(m, r + 2) as i16 as i32 as u32,
                t.text[if m.byte(r + 8) == 1 { 1 } else { 2 }],
            ],
            s,
            h,
        );
        if m.byte(r + 8) != 0 {
            i = i.wrapping_add(1);
        } else {
            let a = m.word(domains);

            let dtor = m.word(a + 12);
            if dtor != 0 {
                Host::call(
                    h,
                    m,
                    dtor,
                    &[
                        m.word(a) + (i as u32) * (half(m, a + 4) as u32),
                        m.word(a + 28),
                    ],
                );
            }

            if norm::array_remove(m.bytes, a, i as u32) == 0 {
                diagnostic(
                    m,
                    p,
                    m.word(p + 0x48),
                    0x5de6,
                    t.text[3],
                    &[t.text[4], i as u32],
                    s,
                    h,
                );
                return 0;
            }
        }
    }
    let count = norm::idx5(m.bytes, m.word(domains));
    if count != 0 {
        live_log(m, p, m.word(p + 0x48), t.text[5], &[count], s, h);
        i = 0;
        while let Some(r) = norm::table_row(m.bytes, m.word(domains), i as u32) {
            live_log(
                m,
                p,
                m.word(p + 0x48),
                t.text[6],
                &[
                    half(m, r + 6) as u32,
                    half(m, r + 4) as u32,
                    half(m, r) as i16 as i32 as u32,
                    half(m, r + 2) as i16 as i32 as u32,
                ],
                s,
                h,
            );
            i = i.wrapping_add(1);
        }
    } else {
        live_log(m, p, m.word(p + 0x48), t.text[7], &[], s, h);
    }
    0
}
fn space(c: u8) -> bool {
    matches!(c, b' ' | b'\t' | b'\n' | b'\r' | b'_')
}
fn token_digit(m: &Memory<'_>, p: u32, d: u16, c: u8) -> bool {
    let set = if d == 999 {
        0
    } else {
        m.word(m.word(p + 0x98) + d as u32 * 16 + 8)
    };
    if set == 0 {
        (c as u32).wrapping_sub(48) <= 9
    } else {
        c != 0 && crate::fe_text::strchr_195d30(m.bytes, set, c as u32) != 0
    }
}
fn token_punct(m: &Memory<'_>, p: u32, d: u16, c: u8) -> bool {
    let mut set = if d == 999 {
        0
    } else {
        m.word(m.word(p + 0x98) + d as u32 * 16 + 12)
    };
    if set == 0 {
        set = m.word(p + 0x64);
    }
    c != 0 && crate::fe_text::strchr_195d30(m.bytes, set, c as u32) != 0
}
fn token_alpha(m: &Memory<'_>, p: u32, d: u16, c: u8) -> bool {
    let set = if d == 999 {
        0
    } else {
        m.word(m.word(p + 0x98) + d as u32 * 16 + 4)
    };
    if set == 0 {
        !space(c) && !token_punct(m, p, d, c) && !token_digit(m, p, d, c)
    } else {
        c != 0 && crate::fe_text::strchr_195d30(m.bytes, set, c as u32) != 0
    }
}
fn token_domain(
    m: &Memory<'_>,
    p: u32,
    a: u32,
    n: u32,
    pos: u16,
    index: &mut u16,
    domain: &mut u16,
) {
    let mut selected = n;
    while (*index as u32) < n
        && m.word(a + *index as u32 * 24 + 12)
            .wrapping_sub(m.word(a + 12))
            <= pos as u32
    {
        if half(m, a + *index as u32 * 24) == 36 {
            selected = *index as u32;
        }
        *index = index.wrapping_add(1);
    }
    if selected < n {
        *domain = 999;
        for i in 0..half(m, p + 0x94) {
            let name = m.word(a + selected * 24 + 20);
            if name != 0
                && crate::fe_text::strcmp16_1765ac(
                    m.bytes,
                    name,
                    m.word(m.word(p + 0x98) + i as u32 * 16),
                ) == 0
            {
                *domain = i;
                break;
            }
        }
    }
}

pub fn fn_1ede44(
    m: &mut Memory<'_>,
    p: u32,
    tag: u32,
    text_handle: [u32; 2],
    annotations: u32,
    count: u32,
    t: &SpeechLiterals,
    s: u32,
    h: &mut impl SpeechHost,
) -> u32 {
    assert_eq!(m.base, 0);
    if frontend::validate_handle(p != 0, tag, 0xf37f, 0x2a4) >= 0x80000000 {
        return 0x89802008;
    }
    let mut saved = [0u8; 84];
    let off = m.offset(s);
    saved.copy_from_slice(&m.bytes[off..off + 84]);
    m.put_half(s, 0);
    m.put_half(s + 8, 0);
    m.put_half(s + 10, 0);
    m.put_half(s + 12, 0);
    let mut result = 0;
    live_log(m, p, t.text[8], t.text[9], &[], s + 32, h);
    'work: {
        result = Host::call(
            h,
            m,
            m.word(m.word(p + 0x84) + 0x34),
            &[text_handle[0], text_handle[1], 1, 0, s],
        );
        if result >= 0x80000000 {
            break 'work;
        }
        result = Host::call(
            h,
            m,
            m.word(m.word(p + 0x84) + 0x58),
            &[
                text_handle[0],
                text_handle[1],
                half(m, s) as u32,
                0,
                s + 4,
                s + 8,
            ],
        );
        if result >= 0x80000000 {
            break 'work;
        }
        let mut pos = 0u16;
        let mut ai = 0u16;
        let mut d = 999u16;
        while pos < (crate::fe_text::strlen_195d90(m.bytes, m.word(s + 4)) as u16) {
            let text = m.word(s + 4);
            let start = pos;
            token_domain(m, p, annotations, count, start, &mut ai, &mut d);
            let typ;
            if token_alpha(m, p, d, m.byte(text + pos as u32)) {
                typ = 0;
                let mut len = 0u16;
                loop {
                    len = len.wrapping_add(1);
                    pos = pos.wrapping_add(1);
                    if !(token_alpha(m, p, d, m.byte(text + pos as u32))
                        && len < 128
                        && pos < (crate::fe_text::strlen_195d90(m.bytes, text) as u16))
                    {
                        break;
                    }
                }
            } else if token_digit(m, p, d, m.byte(text + pos as u32)) {
                typ = 1;
                loop {
                    pos = pos.wrapping_add(1);
                    if !(token_digit(m, p, d, m.byte(text + pos as u32))
                        && pos < (crate::fe_text::strlen_195d90(m.bytes, text) as u16))
                    {
                        break;
                    }
                }
            } else if token_punct(m, p, d, m.byte(text + pos as u32)) {
                typ = 2;
                pos = pos.wrapping_add(1);
            } else if space(m.byte(text + pos as u32)) {
                typ = 3;
                loop {
                    pos = pos.wrapping_add(1);
                    token_domain(m, p, annotations, count, start, &mut ai, &mut d);
                    if !(space(m.byte(text + pos as u32))
                        && pos < (crate::fe_text::strlen_195d90(m.bytes, text) as u16))
                    {
                        break;
                    }
                }
            } else {
                live_log(
                    m,
                    p,
                    t.text[8],
                    t.text[11],
                    &[m.byte(text + pos as u32) as u32],
                    s + 32,
                    h,
                );
                typ = 0;
                pos = pos.wrapping_add(1);
            }
            result = if start == 0 {
                Host::call(
                    h,
                    m,
                    m.word(m.word(p + 0x84) + 0x24),
                    &[text_handle[0], text_handle[1], 3, half(m, s) as u32, s + 10],
                )
            } else {
                Host::call(
                    h,
                    m,
                    m.word(m.word(p + 0x84) + 0x28),
                    &[
                        text_handle[0],
                        text_handle[1],
                        half(m, s + 10) as u32,
                        s + 10,
                    ],
                )
            };
            if result >= 0x80000000 {
                break 'work;
            }
            m.put_half(s + 14, typ);
            m.put_half(s + 16, start);
            m.put_half(s + 18, pos);
            for (field, at) in [(0, s + 14), (1, s + 16), (2, s + 18)] {
                result = Host::call(
                    h,
                    m,
                    m.word(m.word(p + 0x84) + 0x50),
                    &[
                        text_handle[0],
                        text_handle[1],
                        half(m, s + 10) as u32,
                        field,
                        1,
                        at,
                        s + 12,
                    ],
                );
                if result >= 0x80000000 {
                    diagnostic(m, p, t.text[8], 0xa02a, 0, &[], s + 32, h);
                    break 'work;
                }
            }
        }
    }
    live_log(m, p, t.text[8], t.text[10], &[], s + 32, h);
    m.bytes[off..off + 84].copy_from_slice(&saved);
    result
}
fn domain_row_live(
    m: &mut Memory<'_>,
    p: u32,
    i: u32,
    out: &mut u32,
    t: &SpeechLiterals,
    s: u32,
    h: &mut impl SpeechHost,
) -> u32 {
    let rc = norm::domain_row(m.bytes, p, i, Some(out));
    if rc >= 0x80000000 {
        diagnostic(
            m,
            p,
            m.word(p + 0x48),
            0x5de5,
            t.text[3],
            &[t.errors[0], i],
            s,
            h,
        );
    }
    rc
}
fn domain_entry_live(
    m: &mut Memory<'_>,
    p: u32,
    i: u32,
    a: &mut u16,
    b: &mut u16,
    t: &SpeechLiterals,
    s: u32,
    h: &mut impl SpeechHost,
) -> u32 {
    let rc = norm::domain_entry(m.bytes, p, i, a, b);
    if rc >= 0x80000000 {
        diagnostic(
            m,
            p,
            m.word(p + 0x48),
            0x5de6,
            t.text[3],
            &[t.errors[1], i],
            s,
            h,
        );
    }
    rc
}
fn domain_value_live(
    m: &mut Memory<'_>,
    p: u32,
    i: u32,
    out: &mut u32,
    t: &SpeechLiterals,
    s: u32,
    h: &mut impl SpeechHost,
) -> u32 {
    let list = m.word(m.word(p + 0x38) + 8);
    let rc = norm::domain_value(m.bytes, p, i, out);
    if rc >= 0x80000000 {
        if list == 0 {
            diagnostic(m, p, m.word(p + 0x48), 0x5df1, 0, &[], s, h);
        } else {
            diagnostic(
                m,
                p,
                m.word(p + 0x48),
                0x5de9,
                t.errors[4],
                &[t.errors[2], i, t.errors[3], half(m, list) as u32],
                s,
                h,
            );
        }
    }
    rc
}

fn domain_end_live(
    m: &mut Memory<'_>,
    p: u32,
    i: u32,
    end: u16,
    t: &SpeechLiterals,
    s: u32,
    h: &mut impl SpeechHost,
) -> u32 {
    if let Some(row) = norm::table_row(m.bytes, m.word(m.word(p + 0x44)), i) {
        m.put_half(row + 2, end);
        0
    } else {
        diagnostic(
            m,
            p,
            m.word(p + 0x48),
            0x5de5,
            t.text[3],
            &[t.errors[0], i],
            s,
            h,
        );
        norm::forward_error(0)
    }
}

pub fn fn_24c470(
    m: &mut Memory<'_>,
    p: u32,
    t: &SpeechLiterals,
    s: u32,
    h: &mut impl SpeechHost,
) -> u32 {
    assert_eq!(m.base, 0);
    if p == 0 {
        return norm::forward_error(7);
    }
    let mut saved = [0u8; 1120];
    let off = m.offset(s);
    saved.copy_from_slice(&m.bytes[off..off + 1120]);
    let logs = s + 1064;
    live_log(m, p, m.word(p + 0x48), t.text[12], &[], logs, h);
    m.put_half(s + 1000, 0);
    fn_24eac4(m, p, s, s + 1000, 0, half(m, p + 0x60) as i32, 1);
    let count = half(m, s + 1000);
    let mut rc = 0;
    'work: {
        let mut r = 0;
        let mut other = 0;
        let mut domain = 0;
        let mut record = 0;
        let mut od = 0;
        let mut or = 0;
        for i in 0..count {
            domain_row_live(m, p, half(m, s + i as u32 * 2) as u32, &mut r, t, logs, h);
            rc = domain_entry_live(
                m,
                p,
                half(m, s + i as u32 * 2) as u32,
                &mut domain,
                &mut record,
                t,
                logs,
                h,
            );
            for j in i + 1..count {
                domain_row_live(
                    m,
                    p,
                    half(m, s + j as u32 * 2) as u32,
                    &mut other,
                    t,
                    logs,
                    h,
                );
                rc = domain_entry_live(
                    m,
                    p,
                    half(m, s + j as u32 * 2) as u32,
                    &mut od,
                    &mut or,
                    t,
                    logs,
                    h,
                );
                if m.byte(other + 8) == 1
                    && od == domain
                    && or == record
                    && half(m, r) == half(m, other)
                    && half(m, r + 2) == half(m, other + 2)
                {
                    m.put_byte(other + 8, 0);
                    live_log(
                        m,
                        p,
                        m.word(p + 0x48),
                        t.text[13],
                        &[
                            1,
                            or as u32,
                            half(m, r) as i16 as i32 as u32,
                            half(m, r + 2) as i16 as i32 as u32,
                        ],
                        logs,
                        h,
                    );
                }
                if rc >= 0x80000000 {
                    break 'work;
                }
            }
        }
        for i in 0..count {
            domain_row_live(m, p, half(m, s + i as u32 * 2) as u32, &mut r, t, logs, h);
            rc = domain_entry_live(
                m,
                p,
                half(m, s + i as u32 * 2) as u32,
                &mut domain,
                &mut record,
                t,
                logs,
                h,
            );
            if rc >= 0x80000000 {
                break 'work;
            }
            let mut table = 0;
            rc = domain_value_live(m, p, domain as u32, &mut table, t, logs, h);
            if rc >= 0x80000000 {
                break 'work;
            }
            let entry = m.word(table + 8) + record as u32 * 28;
            if half(m, m.word(table + 16) + half(m, entry + 8) as u32 * 8 + 4) != 0
                && m.byte(r + 8) == 1
            {
                let mut end = half(m, r + 2);
                live_log(
                    m,
                    p,
                    m.word(p + 0x48),
                    t.text[14],
                    &[
                        m.word(entry + 20) + half(m, entry) as u32,
                        half(m, r) as i16 as i32 as u32,
                        end as i16 as i32 as u32,
                    ],
                    logs,
                    h,
                );
                for j in i..count {
                    domain_row_live(
                        m,
                        p,
                        half(m, s + j as u32 * 2) as u32,
                        &mut other,
                        t,
                        logs,
                        h,
                    );
                    rc = domain_entry_live(
                        m,
                        p,
                        half(m, s + j as u32 * 2) as u32,
                        &mut od,
                        &mut or,
                        t,
                        logs,
                        h,
                    );
                    if rc >= 0x80000000 {
                        break 'work;
                    }
                    let mut ot = 0;
                    rc = domain_value_live(m, p, od as u32, &mut ot, t, logs, h);
                    if rc >= 0x80000000 {
                        break 'work;
                    }
                    let oe = m.word(ot + 8) + or as u32 * 28;
                    if other != r
                        && m.byte(other + 8) == 1
                        && (half(m, other) == end
                            || (half(m, other) == half(m, r)
                                && half(m, other + 2) == half(m, r + 2)))
                        && frontend::compare_str(
                            &m.bytes[(m.word(entry + 20) + half(m, entry) as u32) as usize..],
                            &m.bytes[(m.word(oe + 20) + half(m, oe) as u32) as usize..],
                        ) == 0
                    {
                        end = half(m, other + 2);
                        live_log(
                            m,
                            p,
                            m.word(p + 0x48),
                            t.text[15],
                            &[
                                m.word(entry + 20) + half(m, entry) as u32,
                                half(m, r) as i16 as i32 as u32,
                                half(m, r + 2) as i16 as i32 as u32,
                                half(m, r) as i16 as i32 as u32,
                                end as u32,
                                m.word(oe + 20) + half(m, oe) as u32,
                            ],
                            logs,
                            h,
                        );
                        m.put_byte(other + 8, 0);
                    }
                }
                if half(m, r + 2) != end {

                    rc = domain_end_live(m, p, i as u32, end, t, logs, h);
                    if rc >= 0x80000000 {
                        break 'work;
                    }
                }
            }
        }
        if rc < 0x80000000 {
            rc = fn_24eb78(m, p, t, logs, h);
        }
    }

    live_log(m, p, m.word(p + 0x48), t.end, &[rc], logs, h);
    m.bytes[off..off + 1120].copy_from_slice(&saved);
    rc
}

pub trait Round3Host: SpeechHost + crate::reentrant::HooksM {}
impl<T: SpeechHost + crate::reentrant::HooksM> Round3Host for T {}
struct SelectionHost<'a, H>(&'a mut H);
impl<H: Host> crate::selection::arena::Host for SelectionHost<'_, H> {
    fn call(
        &mut self,
        a: &mut crate::selection::arena::Arena<'_>,
        f: u32,
        args: &[u32],
    ) -> (u32, u32) {
        let base = a.base();
        let mut m = Memory {
            base,
            bytes: a.bytes,
        };
        (Host::call(self.0, &mut m, f, args), 0)
    }
}
fn release_lookup(m: &mut Memory<'_>, p: u32, h: &mut impl Host) {
    let mut a = crate::selection::arena::Arena::new(m.base, m.bytes);
    crate::selection::arena_mid::release_frame_lookup(&mut a, &mut SelectionHost(h), p);
}

pub fn fn_225138(
    m: &mut Memory<'_>,
    e: u32,
    name: u32,
    value: u32,
    scratch: u32,
    h: &mut impl Round3Host,
) -> u32 {
    let is_freq = {
        let base = m.offset(name);
        let len = crate::frontend::strlen_bytes(&m.bytes[base..]) as usize;
        m.bytes[base..base + len] == *b"frequencyhz"
    };
    let is_rate;
    let is_wait;
    if is_freq {
        is_rate = false;
        is_wait = false;
    } else {
        let base = m.offset(name);
        let len = crate::frontend::strlen_bytes(&m.bytes[base..]) as usize;
        let bytes = &m.bytes[base..base + len];
        is_rate = bytes == *b"rate";
        is_wait = bytes == *b"waitfactor";
    }
    if is_freq {
        if e != 0 && (m.word(e + 0x7a0) != 0 || m.word(e + 0x7b0) != 0) {
            return 0x84e02011;
        }
        return 0;
    }
    if !is_rate && !is_wait {
        return 0;
    }
    let base = m.offset(value);
    let len = crate::frontend::strlen_bytes(&m.bytes[base..]) as usize;
    let v = crate::pronunciation::parse_int(&m.bytes[base..base + len]);
    if is_rate {
        let mut rate = 0i32;
        let rc = crate::selection::inverse_percent(v, &mut rate);
        if rc >= 0x80000000 {
            return rc;
        }
        m.put_word(e + 0x740, rate as u32);
        let log = m.word(m.word(e + 8) + 16);
        m.put_word(scratch, v as u32);
        m.put_word(scratch + 4, rate as u32);
        return crate::speech_part_2::Host::call(h, m, 0x1738e8, &[log, 0x8fa510, 3, 0, 0x225280, scratch]);
    }
    if v < 0 {
        return 0x84e0200f;
    }
    m.put_word(e + 0x744, v as u32);
    let log = m.word(m.word(e + 8) + 16);
    m.put_word(scratch, v as u32);
    crate::speech_part_2::Host::call(h, m, 0x1738e8, &[log, 0x8fa510, 3, 0, 0x22529c, scratch])
}

pub fn fn_228aac(
    m: &mut Memory<'_>,
    p: u32,
    name: u32,
    value: u32,
    scratch: u32,
    h: &mut impl Round3Host,
) -> u32 {

    let kind = {
        let base = m.offset(name);
        let len = crate::frontend::strlen_bytes(&m.bytes[base..]) as usize;
        let n = &m.bytes[base..base + len];
        if n == b"audiooutputbufsamples" {
            0
        } else if n == b"volume" {
            1
        } else if n == b"rate" {
            2
        } else if n == b"ratewpm" {
            3
        } else if n == b"pitch" {
            4
        } else {
            255
        }
    };
    if kind == 255 {
        return 0;
    }
    let vbase = m.offset(value);
    let vlen = crate::frontend::strlen_bytes(&m.bytes[vbase..]) as usize;
    let parsed = crate::pronunciation::parse_int(&m.bytes[vbase..vbase + vlen]);
    let name: &[u8] = match kind {
        0 => b"audiooutputbufsamples",
        1 => b"volume",
        2 => b"rate",
        3 => b"ratewpm",
        _ => b"pitch",
    };
    fn_228780(m, p, name, parsed, scratch, h)
}

fn fn_228780(
    m: &mut Memory<'_>,
    p: u32,
    name: &[u8],
    value: i32,
    scratch: u32,
    h: &mut impl Round3Host,
) -> u32 {
    if name != b"audiooutputbufsamples"
        && name != b"volume"
        && name != b"rate"
        && name != b"ratewpm"
        && name != b"pitch"
    {
        return 0;
    }
    if name == b"audiooutputbufsamples" {
        let frame = m.word(p + 0x18);
        let mut buffer = if (value as u32) > 1 { value as u32 } else { 1 };
        if frame != 0 {
            let remainder = (value as u32) % frame;
            if remainder != 0 && buffer > frame {
                buffer = buffer.wrapping_sub(remainder);
            }
        }
        if buffer > frame {
            m.put_word(p + 0x14, buffer);
        } else {
            m.put_word(p + 0x14, frame);
        }
        return 0;
    }
    let mut result = crate::speech_part_2::Host::call(h, m, 0x176f2c, &[m.word(p + 12)]);
    if result >= 0x80000000 {
        return result;
    }
    if name == b"volume" {
        if (value as u32) < 101 {
            m.put_half(p + 0x24, value as u16);
        } else {
            result = 0x84e0200f;
        }
    } else if name == b"rate" {
        let mut converted = 0i32;
        result = crate::selection::inverse_percent(value, &mut converted);
        if result < 0x80000000 {
            let log = m.word(m.word(p + 8) + 16);
            m.put_word(scratch, value as u32);
            m.put_word(scratch + 4, converted as u32);
            result = crate::speech_part_2::Host::call(
                h,
                m,
                0x1738e8,
                &[log, 0x8fa510, 3, 0, 0x225280, scratch],
            );
            if result < 0x80000000 {
                let engine = m.word(p + 0x28);
                if (converted as u32).wrapping_sub(11) <= 189 {
                    let scale = if converted == 100 {
                        0x4000
                    } else {
                        crate::selection::rounded_product(0xa3d70a, converted, 16)
                    };
                    m.put_word(engine + 36, scale as u32);
                } else {
                    result = 0x84e0200f;
                }
            }
        }
    } else if name == b"ratewpm" {
        result = 0x84e02001;
    } else if name == b"pitch" {
        if (value as u32).wrapping_sub(1) >= 100 {
            result = 0x84e0200f;
        } else {
            let pitch = (value - 50) * 5;
            let log = m.word(m.word(p + 8) + 16);
            m.put_word(scratch, value as u32);
            m.put_word(scratch + 4, pitch as u32);
            result = crate::speech_part_2::Host::call(
                h,
                m,
                0x1738e8,
                &[log, 0x8fa510, 3, 0, 0x228998, scratch],
            );
            if result < 0x80000000 {

                let engine = m.word(p + 0x28);
                m.put_half(engine + 32, ((pitch << 6) as i16) as u16);
            }
        }
    }
    let unlock = crate::speech_part_2::Host::call(h, m, 0x176f5c, &[m.word(p + 12)]);
    if unlock >= 0x80000000 && result < 0x80000000 {
        unlock
    } else {
        result
    }
}

pub fn fn_22508c(m: &mut Memory<'_>, e: u32, name: u32, value: u32, handled: u32) -> u32 {
    let is_freq = {
        let base = m.offset(name);
        let len = crate::frontend::strlen_bytes(&m.bytes[base..]) as usize;
        m.bytes[base..base + len] == *b"frequencyhz"
    };
    let is_wait = if is_freq {
        false
    } else {
        let base = m.offset(name);
        let len = crate::frontend::strlen_bytes(&m.bytes[base..]) as usize;
        m.bytes[base..base + len] == *b"waitfactor"
    };
    m.put_byte(handled, 1);
    if is_freq {
        if e != 0 && (m.word(e + 0x7a0) != 0 || m.word(e + 0x7b0) != 0) {
            m.put_byte(handled, 0);
            return 0x84e02011;
        }
    } else if is_wait {
        let v = {
            let base = m.offset(value);
            let len = crate::frontend::strlen_bytes(&m.bytes[base..]) as usize;
            crate::pronunciation::parse_int(&m.bytes[base..base + len])
        };
        if v < 0 {
            m.put_byte(handled, 0);
            return 0x84e0200f;
        }
    }
    0
}

pub fn fn_226f14(m: &mut Memory<'_>, p: u32, heap: u32, h: &mut impl Round3Host) {
    if p != 0 && m.word(p) != 0 {
        crate::reentrant::release_shared_stream_m(m, h, m.word(p));
        release_lookup(m, m.word(m.word(p) + 0x20), h);
        m.put_word(m.word(p) + 0x20, 0);
        if half(m, m.word(p) + 0x1c) == 0 {
            Host::call(h, m, 0x196428, &[heap, m.word(p)]);
        }
    }
}

pub fn fn_24e2f8(
    m: &mut Memory<'_>,
    p: u32,
    out: u32,
    scratch: u32,
    h: &mut impl Round3Host,
) -> u32 {
    let old = m.word(scratch);
    let ok = crate::reentrant::vec_grow_m(m, h, m.word(m.word(p + 0x44)), out);
    m.put_word(scratch, old);
    if ok != 0 {
        let data = m.word(out);
        let mut arena = crate::selection::arena::Arena::new(m.base, m.bytes);
        crate::selection::arena_leaf::run(&mut arena, 0x195bec, &[data, 0, 10]);
        return 0;
    }
    diagnostic(m, p, m.word(p + 0x48), 0x5df0, 0, &[], scratch, h);
    norm::error_code(10)
}

pub fn fn_2276dc(
    m: &mut Memory<'_>,
    p: u32,
    tag: u32,
    e: &Environment,
    h: &mut impl Round3Host,
) -> u32 {
    let mut rc = frontend::validate_handle(p != 0, tag, 0x8e84, 0x7f8);
    if rc >= 0x80000000 {
        return 0x84e02008;
    }
    if m.word(p + 0x7d8) != 0 {
        let mut arena = crate::selection::arena::Arena::new(m.base, m.bytes);
        let synth = arena.u32(p + 0x7d8);
        let r = crate::selection::arena_mid::finish_selection(
            &mut arena,
            &mut SelectionHost(h),
            synth,
            p + 0x94,
        );
        if r >= 0x80000000 {
            rc = r;
        }
        crate::reentrant::release_shared_stream_m(m, h, m.word(p + 0x7d8));
        release_lookup(m, m.word(m.word(p + 0x7d8) + 0x20), h);
        m.put_word(m.word(p + 0x7d8) + 0x20, 0);
        if half(m, m.word(p + 0x7d8) + 0x1c) == 0 {
            Host::call(
                h,
                m,
                0x196428,
                &[m.word(m.word(p + 8) + 4), m.word(p + 0x7d8)],
            );
        }
        m.put_word(p + 0x7d8, 0);
    }
    if m.word(p + 0x7dc) != 0 {
        Host::call(
            h,
            m,
            0x196428,
            &[m.word(m.word(p + 8) + 4), m.word(p + 0x7dc)],
        );
        for o in [0x7dc, 0x7e4, 0x7e0, 0x7e8] {
            m.put_word(p + o, 0);
        }
    }
    m.put_word(p + 0x7f0, 0);
    m.put_word(p + 0x7ec, 0);
    for o in [0x7a0, 0x7a8, 0x7c8, 0x7d0, 0x7b0, 0x7b8] {
        if m.word(p + o) != 0 {
            let r = Host::call(
                h,
                m,
                m.word(m.word(p + 12) + 36),
                &[m.word(p + o), m.word(p + o + 4)],
            );
            let at = m.offset(p + o);
            let out: &mut [u8; 8] = (&mut m.bytes[at..at + 8]).try_into().unwrap();
            frontend::load_pair(out, &e.cached_handle);
            if r >= 0x80000000 && rc < 0x80000000 {
                rc = r;
            }
        }
    }
    m.put_byte(p + 0x7f4, if rc < 0x80000000 { 0 } else { 255 });
    if rc < 0x80000000 && m.word(p + 0x7c0) != 0 {
        frontend::end_trace_hook(Some(m.word(p + 0x7c0)), Some(m.word(m.word(p + 8) + 16)));
    }
    rc
}

pub struct SelectionOpenLiterals {
    pub mime: [u32; 5],
    pub category: u32,
    pub format: u32,
    pub content_type: u32,
    pub callbacks: [u32; 3],
}

pub fn fn_2279b4(
    m: &mut Memory<'_>,
    p: u32,
    tag: u32,
    stream: [u32; 2],
    e: &Environment,
    t: &SelectionOpenLiterals,
    scratch: u32,
    h: &mut impl Round3Host,
) -> u32 {
    if frontend::validate_handle(p != 0, tag, 0x8e84, 0x7f8) >= 0x80000000 {
        return 0x84e02008;
    }
    let off = m.offset(scratch);
    let mut saved = [0u8; 64];
    saved.copy_from_slice(&m.bytes[off..off + 64]);
    for (i, v) in t.callbacks.iter().enumerate() {
        m.put_word(scratch + i as u32 * 4, *v);
    }
    m.put_byte(p + 0x7f4, 0);
    if m.word(p + 0x7dc) != 0 {
        Host::call(
            h,
            m,
            0x196428,
            &[m.word(m.word(p + 8) + 4), m.word(p + 0x7dc)],
        );
        for o in [0x7dc, 0x7e4, 0x7e0, 0x7e8] {
            m.put_word(p + o, 0);
        }
    }
    m.put_word(p + 0x7f0, 0);
    m.put_word(p + 0x7ec, 0);
    let mut rc = 0;
    for (o, mi, mode, rw, required, code) in [
        (0x7a0, 0, 0, 0, true, 25003),
        (0x7a8, 1, 0, 0, true, 25003),
        (0x7c8, 2, 0, 0, false, 0),
        (0x7d0, 3, 0, 0, false, 0),
        (0x7b0, 4, scratch, 1, true, 25004),
        (0x7b8, 1, u32::MAX, 1, true, 25004),
    ] {
        let mode = if mode == u32::MAX {
            Host::call(h, m, m.word(m.word(p + 12) + 76), &[])
        } else {
            mode
        };
        let r = Host::call(
            h,
            m,
            m.word(m.word(p + 12) + 32),
            &[stream[0], stream[1], 78, t.mime[mi], mode, rw, p + o],
        );
        if required {
            rc = r;
            if rc >= 0x80000000 {

                m.put_word(scratch + 12, t.content_type);
                m.put_word(scratch + 16, t.mime[mi]);
                crate::render_part_a::error_log(
                    m,
                    h,
                    m.word(m.word(p + 8) + 16),
                    t.category,
                    code,
                    t.format,
                    scratch + 12,
                    scratch + 24,
                );
                fn_2276dc(m, p, tag, e, h);
                m.bytes[off..off + 64].copy_from_slice(&saved);
                return rc;
            }
        }
    }
    if m.word(p + 0x7c0) != 0 {
        frontend::begin_trace_hook(Some(m.word(p + 0x7c0)));
    }
    m.bytes[off..off + 64].copy_from_slice(&saved);
    rc
}

pub fn selection_cleanup(m: &mut Memory<'_>, f: u32, p: u32, h: &mut impl Host) -> u32 {
    let mut arena = crate::selection::arena::Arena::new(m.base, m.bytes);
    let mut host = SelectionHost(h);
    match f {
        0x2a3740 => crate::selection::arena_mid::release_filter(&mut arena, &mut host, p),
        0x2a3414 => crate::selection::arena_mid::release_selector(&mut arena, &mut host, p),
        _ => panic!("not a selection cleanup entry"),
    }
}

pub struct QueryLiterals {
    pub category: u32,
    pub enter: u32,
    pub lid: u32,
}

pub fn fn_26b718(m:&mut Memory<'_>,p:u32,key:u32,size:u32,found:u32,scratch:u32,h:&mut impl Round3Host)->u32 {
 let off=m.offset(scratch);let mut saved=[0u8;80];saved.copy_from_slice(&m.bytes[off..off+80]);
 let rc=crate::reentrant::contains_m(m,&mut QueryMaps {h,scratch:scratch+4},p,key,size,found,scratch);
 m.bytes[off..off+80].copy_from_slice(&saved);rc
}
struct QueryMaps<'a,H>{h:&'a mut H,scratch:u32}
impl<H:Round3Host> crate::reentrant::HooksM for QueryMaps<'_,H>{
 fn call_m(&mut self,f:u32,a:&[u32],m:&mut Memory<'_>)->u32 {
  if f==0x17a1a4 {
   let e=crate::boot_part_e::UnmapEnvironment {category:0,begin_format:0,end_format:0,error_format:0,labels:[0;3],scratch:self.scratch+4};
   crate::leftovers_1::fn_17a1a4(m,a[0],a[1],a[2],a[3],a[4],self.scratch,&e,self.h)
  }else{crate::reentrant::HooksM::call_m(self.h,f,a,m)}
 }
 fn literal_m(&self,l:crate::reentrant::LiteralM)->u32{crate::reentrant::HooksM::literal_m(self.h,l)}
}
pub fn fn_1ecf68(
    m: &mut Memory<'_>,
    p: u32,
    tag: u32,
    key: u32,
    lid: u32,
    found: u32,
    t: &QueryLiterals,
    scratch: u32,
    h: &mut (impl Round3Host + crate::boot_part_b::DataHost),
) -> u32 {
    if frontend::validate_handle(p != 0, tag, 0xf37f, 0x2a4) >= 0x80000000 {
        return 0x89802008;
    }
    m.put_byte(found, 0);
    if key == 0 {
        diagnostic(m, p, t.category, 0xa029, 0, &[], scratch, h);
        return 0x89802007;
    }
    live_log(m, p, t.category, t.enter, &[key], scratch, h);
    let off = m.offset(scratch);
    let mut saved = [0u8; 80];
    saved.copy_from_slice(&m.bytes[off..off + 80]);
    let rc = 'query: {
        let r = if lid != 0 {
            Host::call(h, m, 0x1795f0, &[m.word(m.word(p) + 12), t.lid, 1, scratch])
        } else {
            crate::leftovers_1::fn_179f30(m,m.word(m.word(p)+12),p+0x9e,scratch,&crate::boot_part_b::DataOpenGlobals {module:0,allocators:[0;4],scratch:scratch+8,fullpath_key:0,add_item_format:0,broker_label:0,ctype_table:0,brokerstring_key:0},h)
        };
        if r >= 0x80000000 {
            break 'query if lid != 0 { 0 } else { r };
        }
        let base = p + if lid != 0 { 0x48 } else { 8 };
        let size = m.word(p + if lid != 0 { 0x44 } else { 4 });
        let r = crate::reentrant::contains_m(m, &mut QueryMaps {h,scratch:scratch+8}, base, key, size, found, scratch + 4);
        if r >= 0x80000000 {
            break 'query r;
        }
        Host::call(h, m, 0x179f3c, &[m.word(m.word(p) + 12), m.word(scratch)]);
        r
    };
    m.bytes[off..off + 80].copy_from_slice(&saved);
    rc
}

pub struct DictionaryLiterals<'a> {
    pub types: [&'a [u8]; 3],
    pub disabled_mode: &'a [u8],
    pub langcode: u32,
    pub mlset: u32,
    pub category: u32,
    pub invalid_format: u32,
    pub name: u32,
    pub empty: u32,
    pub type_label: u32,

    pub trace_formats: [u32; 4],
    pub labels: [u32; 7],
}
fn string_slice<'a>(m: &'a Memory<'_>, p: u32) -> &'a [u8] {
    &m.bytes[m.offset(p)..]
}
fn dictionary_trace(m: &mut Memory<'_>, p: u32, fmt: u32, args: &[u32], h: &mut impl Host) {
    let mut a = [0u32; 15];
    a[0] = m.word(m.word(p + 8) + 16);
    a[1] = 0x6c;
    a[2] = fmt;
    a[3..3 + args.len()].copy_from_slice(args);
    Host::call(h, m, 0x17486c, &a[..3 + args.len()]);
}

pub fn fn_202348(
    m: &mut Memory<'_>,
    p: u32,
    _tag: u32,
    text: u32,
    len: u32,
    out: u32,
    arg5: u32,
    lang: u32,
    classes: &[u16; 256],
    t: &DictionaryLiterals<'_>,
    scratch: u32,
    h: &mut impl Round3Host,
) -> u32 {
    if p == 0 {
        return 0x85002008;
    }
    assert_eq!(m.base, 0);
    let n = Host::call(h, m, 0x17bff4, &[text]);
    let buf = Host::call(h, m, 0x196394, &[m.word(m.word(p + 8) + 4), n]);
    if buf == 0 {
        return 0x8500200a;
    }
    let off = m.offset(scratch);
    let mut saved = [0u8; 128];
    saved.copy_from_slice(&m.bytes[off..off + 128]);
    let mut found = 0;
    let mut iterator = 0;
    let mut saved_language = false;
    let mut err;
    if m.word(lang) != 0 {
        let mut i = 0u8;
        while frontend::strlen_bytes(string_slice(m, m.word(lang))) as i32 > i as i32 {
            let at = m.word(lang) + i as u32;
            let b = m.byte(at);
            m.put_byte(at, frontend::ascii_fold(classes, b as u32) as u8);
            i = i.wrapping_add(1);
        }
    }
    m.put_byte(scratch, 0);
    if m.word(lang) != 0 && frontend::strlen_bytes(string_slice(m, m.word(lang))) < 8 {
        frontend::copy_str(m.bytes, scratch, m.word(lang));
        saved_language = true;
    }
    m.put_word(scratch + 8, 0);
    m.put_word(scratch + 12, 0);
    err = Host::call(
        h,
        m,
        0x175a1c,
        &[m.word(m.word(p + 8) + 20), t.langcode, scratch + 8],
    );
    if err < 0x80000000 {
        err = Host::call(
            h,
            m,
            0x175a1c,
            &[m.word(m.word(p + 8) + 20), t.mlset, scratch + 12],
        );
        if err >= 0x80000000 {
            m.put_word(scratch + 12, 0);
        }
        let langcode = m.word(scratch + 8);
        let mut i = 0u8;
        while frontend::strlen_bytes(string_slice(m, langcode)) as i32 > i as i32 {
            let at = langcode + i as u32;
            let b = m.byte(at);
            m.put_byte(at, frontend::ascii_fold(classes, b as u32) as u8);
            i = i.wrapping_add(1);
        }
        'entries: loop {
            m.put_word(scratch + 16, iterator);
            let entry = Host::call(h, m, 0x281aa4, &[p + 24, scratch + 16]);
            iterator = m.word(scratch + 16);
            if entry == 0 {
                break;
            }
            let kind = m.word(entry + 8);
            if t.types
                .iter()
                .all(|s| frontend::compare_str(string_slice(m, kind), s) != 0)
            {
                m.put_word(scratch + 64, t.name);
                m.put_word(
                    scratch + 68,
                    if m.word(entry + 4) != 0 {
                        m.word(entry + 4)
                    } else {
                        t.empty
                    },
                );
                m.put_word(scratch + 72, t.type_label);
                m.put_word(scratch + 76, kind);
                crate::render_part_a::error_log(
                    m,
                    h,
                    m.word(m.word(p + 8) + 16),
                    t.category,
                    0x113d,
                    t.invalid_format,
                    scratch + 64,
                    scratch + 80,
                );
                err = 0x85002007;
                break;
            }
            if frontend::compare_str(string_slice(m, m.word(entry + 12)), t.disabled_mode) == 0 {
                found = 0;
            } else {
                if saved_language {
                    frontend::copy_str(m.bytes, m.word(lang), scratch);
                }
                let mut lid = false;
                if m.word(lang) != 0 {
                    m.bytes[off + 120..off + 125].copy_from_slice(b"_lid\0");
                    lid = frontend::find_substr(m.bytes, m.word(lang), scratch + 120) != 0;
                }
                if lid {
                    let at = m.word(lang);
                    m.put_byte(at + 3, 0);
                    found = at;
                    m.bytes[off + 120..off + 122].copy_from_slice(b"_\0");
                    frontend::append_str(m.bytes, found, scratch + 120);
                    frontend::append_str(m.bytes, found, langcode);
                } else if m.word(lang) == 0 {
                    found = langcode;
                } else {
                    found = m.word(lang);
                    m.put_byte(found + 3, 0);
                }
            }
            if m.byte(entry + 16) == 1 {
                let status = Host::call(h, m, 0x17bf70, &[text, buf]);
                if status < 0x80000000 {
                    dictionary_trace(
                        m,
                        p,
                        t.trace_formats[0],
                        &[
                            t.labels[0],
                            0,
                            t.labels[1],
                            m.word(entry + 4),
                            t.labels[2],
                            buf,
                        ],
                        h,
                    );
                    m.put_word(scratch + 20, found);
                    err = crate::reentrant::lookup_transcriptions_m(
                        m,
                        h,
                        p,
                        entry,
                        scratch + 20,
                        buf,
                        out,
                        len,
                        arg5,
                        scratch + 24,
                    );
                    found = m.word(scratch + 20);
                    if err < 0x80000000 {
                        if m.word(lang) != 0 {
                            m.put_byte(m.word(lang) + 3, 0);
                            let v = if frontend::compare_str(
                                string_slice(m, m.word(lang)),
                                string_slice(m, found),
                            ) == 0
                            {
                                0
                            } else {
                                found
                            };
                            m.put_word(lang, v);
                        } else {
                            let v = if frontend::compare_str(
                                string_slice(m, found),
                                string_slice(m, langcode),
                            ) == 0
                            {
                                0
                            } else {
                                found
                            };
                            m.put_word(lang, v);
                        }
                        dictionary_trace(
                            m,
                            p,
                            t.trace_formats[1],
                            &[
                                t.labels[3],
                                0,
                                t.labels[4],
                                m.word(lang),
                                t.labels[5],
                                err,
                                t.labels[6],
                                out,
                            ],
                            h,
                        );
                        break 'entries;
                    }
                    dictionary_trace(
                        m,
                        p,
                        t.trace_formats[2],
                        &[t.labels[3], 0, t.labels[5], err, t.labels[6], out],
                        h,
                    );
                } else {
                    dictionary_trace(
                        m,
                        p,
                        t.trace_formats[3],
                        &[t.labels[3], 0, t.labels[5], err],
                        h,
                    );
                }
            }
        }

        if iterator == 0 {
            if m.word(p + 40) != 0 {
                err = Host::call(h, m, m.word(p + 40), &[m.word(p + 44), text, len, out]);
            } else if err < 0x80000000 {
                err = 0x85002014;
            }
        }
    }
    if found != 0 {
        frontend::check_handle(m.word(m.word(p + 8) + 20) != 0);
    }
    Host::call(h, m, 0x196428, &[m.word(m.word(p + 8) + 4), buf]);
    m.bytes[off..off + 128].copy_from_slice(&saved);
    err
}

pub fn fn_2031b0(
    m: &mut Memory<'_>,
    p: u32,
    tag: u32,
    a: u32,
    b: u32,
    done: u32,
    h: &mut impl crate::fe_text::FeTextHost,
) -> u32 {
    assert_eq!(m.base, 0);
    let mut saved = [0u8; 0x58];
    saved.copy_from_slice(&m.bytes[0x770..0x7c8]);
    let rc = crate::fe_text::fe_depes_2031b0(m.bytes, p, tag, a, b, done, h);
    m.bytes[0x770..0x7c8].copy_from_slice(&saved);
    rc
}

pub struct XcoderLiterals {
    pub input_key: u32,
    pub native_key: u32,
    pub utf16: u32,
    pub sync: u32,
    pub category: u32,
    pub format: u32,
    pub content_type: u32,
}
fn resolved_context(m: &Memory<'_>, p: u32, tag: u32, e: &Environment) -> (u32, u32) {
    let mut rc = frontend::validate_handle(p != 0, tag, 0x354, 300);
    if rc >= 0x80000000 {
        if rc & 0x1fff == 8 {
            rc = 0x88102008;
        }
        return (rc, 0);
    }
    let a = m.word(p);
    let b = m.word(p + 4);
    let cached = [
        u32::from_le_bytes(e.cached_handle[..4].try_into().unwrap()),
        u32::from_le_bytes(e.cached_handle[4..].try_into().unwrap()),
    ];
    let pointed = if [a, b] != cached && a != 0 && b == 0x1e61 {
        Some(half(m, a))
    } else {
        None
    };
    let mut ctx = 0;
    rc = frontend::resolve_engine_context(
        a,
        pointed,
        b,
        &e.cached_handle,
        e.default_context,
        &mut ctx,
    );
    (rc, ctx)
}
fn parameter(m: &mut Memory<'_>, context: u32, key: u32, out: u32, h: &mut impl Host) -> u32 {
    m.put_word(out, 0);
    let rc = Host::call(h, m, 0x175a1c, &[m.word(context + 20), key, out]);
    if rc >= 0x80000000 { 0 } else { m.word(out) }
}

pub fn fn_230e1c(
    m: &mut Memory<'_>,
    p: u32,
    tag: u32,
    source: [u32; 2],
    e: &Environment,
    t: &XcoderLiterals,
    classes: &[u16; 256],
    scratch: u32,
    h: &mut impl Round3Host,
) -> u32 {
    let (mut rc, ctx) = resolved_context(m, p, tag, e);
    if rc >= 0x80000000 {
        return rc;
    }
    assert_eq!(m.base, 0);
    let off = m.offset(scratch);
    let mut saved = [0u8; 512];
    saved.copy_from_slice(&m.bytes[off..off + 512]);
    let result = 'open: {
        let charset = scratch;
        let text = scratch + 132;
        let cell = scratch + 400;
        m.put_byte(charset, 0);
        let mut mime = parameter(m, ctx, t.input_key, cell, h);
        if mime == 0 || m.byte(mime) == 0 {
            let native = parameter(m, ctx, t.native_key, cell, h);
            if native == 0 || m.byte(native) == 0 {
                crate::render_part_a::error_log(
                    m,
                    h,
                    m.word(ctx + 16),
                    t.category,
                    0x3ab1,
                    0,
                    scratch + 420,
                    scratch + 440,
                );
                break 'open 0x88102000;
            }
            frontend::copy_str(m.bytes, charset, native);
            m.bytes[off + 272..off + 292].copy_from_slice(b"text/plain;charset=\0");
            frontend::copy_str(m.bytes, text, scratch + 272);
            frontend::append_str(m.bytes, text, native);
            mime = text;
        } else if frontend::compare_str(string_slice(m, mime), b"text/plain\0") == 0 {
            let native = parameter(m, ctx, t.native_key, cell, h);
            if native == 0 || m.byte(native) == 0 {
                crate::render_part_a::error_log(
                    m,
                    h,
                    m.word(ctx + 16),
                    t.category,
                    0x3ab1,
                    0,
                    scratch + 420,
                    scratch + 440,
                );
                break 'open 0x88102000;
            }
            frontend::copy_str(m.bytes, charset, native);
        } else {
            if frontend::compare_str_n(string_slice(m, mime), b"text/plain;\0", 11) != 0 {
                break 'open 0;
            }
            m.bytes[off + 272..off + 282].copy_from_slice(b";charset=\0");
            let value = frontend::find_substr(m.bytes, mime, scratch + 272);
            if value == 0 {
                break 'open 0;
            }
            let value = value + 9;
            let mut i = 0;
            while m.byte(value + i) != 0 && m.byte(value + i) != b';' && i < 127 {
                m.put_byte(charset + i, m.byte(value + i));
                i += 1;
            }
            m.put_byte(charset + i, 0);
        }
        if frontend::compare_folded(classes, string_slice(m, charset), b"UTF-16\0") == 0
            || frontend::compare_folded(classes, string_slice(m, charset), b"UCS-2\0") == 0
        {
            break 'open rc;
        }
        if m.word(p + 264) != 0
            || frontend::compare_folded(classes, string_slice(m, charset), string_slice(m, p + 8))
                != 0
        {
            frontend::copy_str(m.bytes, p + 8, charset);
            rc = Host::call(h, m, 0x2b0c04, &[p]);
            if rc & 0x1fff == 1 {
                break 'open 0;
            }
            if rc >= 0x80000000 {
                break 'open rc;
            }
        }
        for (literal, rw, out, code) in [
            (mime, 0, p + 276, 0x3ab2),
            (t.utf16, 1, p + 284, 0x3ab3),
            (t.sync, 1, p + 292, 0x3ab3),
        ] {
            rc = Host::call(
                h,
                m,
                m.word(m.word(p + 272) + 32),
                &[source[0], source[1], 0x81, literal, 0, rw, out],
            );
            if rc >= 0x80000000 {
                m.put_word(scratch + 420, t.content_type);
                m.put_word(scratch + 424, literal);
                crate::render_part_a::error_log(
                    m,
                    h,
                    m.word(ctx + 16),
                    t.category,
                    code,
                    t.format,
                    scratch + 420,
                    scratch + 440,
                );
                break 'open rc;
            }
        }
        rc
    };
    m.bytes[off..off + 512].copy_from_slice(&saved);
    result
}

pub struct G2pLiterals {
    pub key: u32,
    pub category: u32,
    pub formats: [u32; 9],
}
fn g2p_log(
    m: &mut Memory<'_>,
    ctx: u32,
    t: &G2pLiterals,
    index: usize,
    args: &[u32],
    s: u32,
    h: &mut impl Round3Host,
) {
    for (i, v) in args.iter().enumerate() {
        m.put_word(s + i as u32 * 4, *v);
    }
    log_live::fn_1738e8(
        m,
        m.word(m.word(ctx) + 16),
        t.category,
        5,
        0,
        t.formats[index],
        s,
        h,
    );
}

pub fn fn_26ff18(
    m: &mut Memory<'_>,
    ctx: u32,
    text: u32,
    word: u32,
    orig: u32,
    dict: u32,
    domain: u32,
    tail: [u32; 7],
    classes: &[u16; 256],
    t: &G2pLiterals,
    scratch: u32,
    h: &mut impl Round3Host,
) -> u32 {
    assert_eq!(m.base, 0);
    let off = m.offset(scratch);
    let mut saved = [0u8; 256];
    saved.copy_from_slice(&m.bytes[off..off + 256]);
    let start = tail[0] as u16;
    let mut record = tail[1] as u16;
    let object = tail[2];
    let data = tail[3];
    let end = tail[4];
    let token_check = tail[5] != 0;
    let lang_only = tail[6] != 0;
    m.put_half(scratch + 28, 0);
    m.put_half(scratch + 30, start);
    m.put_half(scratch + 32, record);
    m.put_half(scratch + 34, 1);
    m.put_byte(scratch + 36, 4);
    m.put_half(scratch + 44, 0);
    m.put_word(scratch + 48, 0);
    let result = 'lookup: {
        let status = Host::call(
            h,
            m,
            0x175a1c,
            &[m.word(m.word(ctx) + 20), t.key, scratch + 48],
        );
        if status < 0x80000000
            && frontend::compare_str(string_slice(m, m.word(scratch + 48)), b"no\0") == 0
        {
            break 'lookup 0;
        }
        g2p_log(m, ctx, t, 0, &[word], scratch + 160, h);
        let lex = m.word(ctx + 24);
        let mut ret;
        if m.byte(ctx + 0x2b4) != 0 {
            m.bytes[off + 80..off + 84].copy_from_slice(b"g2p\0");
            let language = ctx + 0x2b5 + m.byte(word) as u32 * 3;
            frontend::append_str(m.bytes, scratch + 80, language);
            ret = Host::call(
                h,
                m,
                m.word(lex + 28),
                &[
                    m.word(ctx + 16),
                    m.word(ctx + 20),
                    scratch + 80,
                    word,
                    scratch + 40,
                    scratch + 44,
                    scratch + 38,
                ],
            );
        } else {
            ret = Host::call(
                h,
                m,
                m.word(lex + 72),
                &[
                    m.word(ctx + 16),
                    m.word(ctx + 20),
                    dict,
                    word,
                    scratch + 40,
                    scratch + 44,
                    scratch + 38,
                    m.byte(ctx + 0x172) as u32,
                ],
            );
        }
        if ret >= 0x80000000 {
            break 'lookup ret;
        }
        let count = half(m, scratch + 44);
        for i in 0..count {
            let w = m.word(m.word(scratch + 40) + i as u32 * 4);
            let sep = m.byte(scratch + 38) as u32;
            let mut fields = [0u32; 7];
            let mut cursor = w;
            for slot in &mut fields {
                let split = frontend::find_byte(m.bytes, cursor, sep);
                if split == 0 {
                    break;
                }
                *slot = split + 1;
                m.put_byte(split, 0);
                cursor = split + 1;
            }

            let pos = fields[0];
            let dom = fields[1];
            let mut phon = fields[2];
            let gpos = fields[3];
            let priority = fields[4];
            let lang = fields[6];
            if lang != 0 {
                let p = frontend::find_byte(m.bytes, lang, sep);
                if p != 0 {
                    m.put_byte(p, 0);
                }
            }
            if w == 0 {
                crate::render_part_a::error_log(
                    m,
                    h,
                    m.word(m.word(ctx) + 16),
                    t.category,
                    0x84d1,
                    0,
                    scratch + 160,
                    scratch + 192,
                );
                break 'lookup 0x89b02007;
            }
            g2p_log(m, ctx, t, 1, &[w], scratch + 160, h);
            if dom != 0
                && frontend::compare_str(string_slice(m, dom), b"\0") != 0
                && frontend::find_substr(m.bytes, dom, domain) == 0
            {
                g2p_log(m, ctx, t, 2, &[], scratch + 160, h);
                continue;
            }
            if m.byte(ctx + 0x2a8) != 0 {
                if lang == 0 || frontend::find_byte(m.bytes, lang, b'!' as u32) != 0 {
                    m.put_byte(lang + 3, 0);
                } else {
                    let found = frontend::find_substr(m.bytes, domain, lang);
                    let length = frontend::strlen_bytes(string_slice(m, lang));
                    let absent = found == 0 || (length != 0 && m.byte(found + length) != 0);
                    if lang_only {
                        if !token_check
                            && absent
                            && frontend::compare_folded(
                                classes,
                                string_slice(m, lang),
                                string_slice(m, m.word(ctx + 0x2ac)),
                            ) != 0
                        {
                            g2p_log(m, ctx, t, 3, &[lang], scratch + 160, h);
                            continue;
                        }
                    } else if !token_check && absent {
                        let user = frontend::find_substr(m.bytes, m.word(ctx + 0x2b0), domain);
                        if frontend::compare_folded(
                            classes,
                            string_slice(m, lang),
                            string_slice(m, m.word(ctx + 0x2ac)),
                        ) != 0
                            || user == 0
                            || m.byte(user + frontend::strlen_bytes(string_slice(m, domain)))
                                != b'|'
                        {
                            g2p_log(m, ctx, t, 4, &[lang], scratch + 160, h);
                            continue;
                        }
                    }
                }
                if token_check && lang != 0 {
                    let found = frontend::find_substr(m.bytes, domain, lang);
                    let n = frontend::strlen_bytes(string_slice(m, lang));
                    if found == 0 || (n != 0 && m.byte(found + n) != 0) {
                        let user = frontend::find_substr(m.bytes, m.word(ctx + 0x2b0), domain);
                        if frontend::compare_folded(
                            classes,
                            string_slice(m, lang),
                            string_slice(m, m.word(ctx + 0x2ac)),
                        ) != 0
                            || user == 0
                            || m.byte(user + frontend::strlen_bytes(string_slice(m, domain)))
                                != b'|'
                        {
                            g2p_log(m, ctx, t, 5, &[lang], scratch + 160, h);
                            continue;
                        }
                    }
                }
            }
            m.put_half(
                end,
                start.wrapping_add(frontend::strlen_bytes(string_slice(m, orig)) as u16),
            );
            m.put_half(scratch + 32, record);
            ret = Host::call(
                h,
                m,
                m.word(m.word(ctx + 12) + 44),
                &[object, data, record as u32, scratch + 32],
            );
            record = half(m, scratch + 32);
            if ret >= 0x80000000 {
                break 'lookup ret;
            }
            for (field, value) in [(0, scratch + 36), (1, scratch + 30), (2, end)] {
                ret = Host::call(
                    h,
                    m,
                    m.word(m.word(ctx + 12) + 80),
                    &[object, data, record as u32, field, 1, value, scratch + 28],
                );
                if ret >= 0x80000000 {
                    break 'lookup ret;
                }
            }
            let mut p = m
                .word(text)
                .wrapping_add(half(m, ctx + 80) as i16 as i32 as u32)
                .wrapping_add(start as u32);
            let limit = m
                .word(text)
                .wrapping_add(half(m, end) as u32)
                .wrapping_add(half(m, ctx + 80) as i16 as i32 as u32);
            let mut q = frontend::find_byte(m.bytes, p, b' ' as u32);
            if q == 0 || q >= limit {
                q = frontend::find_byte(m.bytes, p, b'_' as u32);
            }
            while q != 0 && q < limit {
                p = q;
                m.put_byte(q, b'-');
                q = frontend::find_byte(m.bytes, q, b' ' as u32);
                if q == 0 || q >= limit {
                    q = frontend::find_byte(m.bytes, p, b'_' as u32);
                }
            }
            let p = frontend::find_byte(m.bytes, w, 0x12);
            if p != 0 {
                m.put_byte(p, 0);
                ret = Host::call(
                    h,
                    m,
                    m.word(m.word(ctx + 12) + 80),
                    &[
                        object,
                        data,
                        record as u32,
                        11,
                        frontend::strlen_bytes(string_slice(m, p + 1)) + 1,
                        p + 1,
                        scratch + 28,
                    ],
                );
                if ret >= 0x80000000 {
                    break 'lookup ret;
                }
            }
            ret = Host::call(
                h,
                m,
                m.word(m.word(ctx + 12) + 80),
                &[
                    object,
                    data,
                    record as u32,
                    3,
                    frontend::strlen_bytes(string_slice(m, w)) + 1,
                    w,
                    scratch + 28,
                ],
            );
            if ret >= 0x80000000 {
                break 'lookup ret;
            }
            for (field, ptr) in [(5, phon), (6, gpos), (4, pos)] {
                if ptr == 0 {
                    continue;
                }
                let mut value = ptr;
                let mut n = frontend::strlen_bytes(string_slice(m, ptr)) + 1;
                if field != 4 && n == 1 {
                    m.bytes[off + 96..off + 98].copy_from_slice(b"*\0");
                    value = scratch + 96;
                    n = 2;
                }
                ret = Host::call(
                    h,
                    m,
                    m.word(m.word(ctx + 12) + 80),
                    &[object, data, record as u32, field, n, value, scratch + 28],
                );
                if ret >= 0x80000000 {
                    break 'lookup ret;
                }
            }
            let mut prio = if priority != 0 {
                frontend::parse_int(classes, m.bytes, priority) as u16
            } else {
                1
            };
            let langc;
            if lang != 0 {
                phon = frontend::find_substr(m.bytes, domain, lang);
                langc = m.byte(lang);
                ret = Host::call(
                    h,
                    m,
                    m.word(m.word(ctx + 12) + 80),
                    &[
                        object,
                        data,
                        record as u32,
                        9,
                        frontend::strlen_bytes(string_slice(m, lang)) + 1,
                        lang,
                        scratch + 28,
                    ],
                );
                g2p_log(m, ctx, t, 6, &[lang], scratch + 160, h);
                let user = if m.word(ctx + 0x2b0) != 0 {
                    frontend::find_substr(m.bytes, m.word(ctx + 0x2b0), domain)
                } else {
                    0
                };
                let n = frontend::strlen_bytes(string_slice(m, lang));
                if m.byte(ctx + 0x2a8) != 0
                    && ((phon != 0 && (n == 0 || m.byte(phon + n) == 0))
                        || (frontend::compare_folded(
                            classes,
                            string_slice(m, lang),
                            string_slice(m, m.word(ctx + 0x2ac)),
                        ) == 0
                            && user != 0
                            && m.byte(user + frontend::strlen_bytes(string_slice(m, domain)))
                                == b'|'))
                {
                    prio = prio.wrapping_add(1);
                    g2p_log(m, ctx, t, 7, &[prio as u32], scratch + 160, h);
                }
            } else {
                langc = 0;
                m.put_byte(scratch + 96, 0);
                ret = Host::call(
                    h,
                    m,
                    m.word(m.word(ctx + 12) + 80),
                    &[
                        object,
                        data,
                        record as u32,
                        9,
                        1,
                        scratch + 96,
                        scratch + 28,
                    ],
                );
            }
            if ret >= 0x80000000 {
                break 'lookup ret;
            }
            m.put_half(scratch + 34, prio);
            ret = Host::call(
                h,
                m,
                m.word(m.word(ctx + 12) + 80),
                &[
                    object,
                    data,
                    record as u32,
                    7,
                    1,
                    scratch + 34,
                    scratch + 28,
                ],
            );
            if ret >= 0x80000000 {
                break 'lookup ret;
            }
            if count == 1 {
                if langc == b'e' {
                    m.put_byte(ctx + 0x1a5, 1);
                } else if langc != 0 {
                    m.put_byte(ctx + 0x1a5, 0);
                }
            }
        }
        ret
    };
    m.bytes[off..off + 256].copy_from_slice(&saved);
    result
}

pub struct DomainOpenLiterals {
    pub formats: [u32; 12],
    pub trace: u32,
    pub remapped: u32,
    pub empty: u32,
    pub virtual_domain: u32,
}
fn rule_live(
    m: &mut Memory<'_>,
    p: u32,
    id: u32,
    value: u32,
    field: u32,
    s: u32,
    h: &mut impl Round3Host,
) -> u32 {
    m.put_half(s, 0);
    m.put_half(s + 4, 0);
    let status = Host::call(
        h,
        m,
        m.word(m.word(p + 12) + 92),
        &[m.word(p + 16), m.word(p + 20), id, field, s],
    );
    let got = half(m, s);
    let mut wrote = 0u16;
    if status < 0x80000000 && got == 1 {
        Host::call(
            h,
            m,
            m.word(m.word(p + 12) + 84),
            &[m.word(p + 16), m.word(p + 20), id, field, 1, value, s + 4],
        );
        wrote = half(m, s + 4);
    }

    let mut frame = [0u8; 0x530];
    for (at, v) in [
        (0x504, status),
        (0x508, got as u32),
        (0x518, 0),
        (0x51c, wrote as u32),
    ] {
        frame[at..at + 4].copy_from_slice(&v.to_le_bytes());
    }
    let rc = match field {
        1 => norm::rule_store_first(&mut frame, 0, 0, 0),
        2 => norm::rule_store_second(&mut frame, 0, 0, 0),
        _ => norm::rule_store_query(&mut frame, 0, 0, 0),
    };
    if rc >= 0x80000000 {
        let code = if field == 0 {
            0x5de0
        } else if (status < 0x80000000 && got == 1) == (field == 1) {
            0x5dde
        } else {
            0x5ddf
        };
        diagnostic(m, p, m.word(p + 0x48), code, 0, &[], s + 8, h);
    }
    rc
}
fn set_active(
    m: &mut Memory<'_>,
    p: u32,
    block: u32,
    domain: u32,
    begin: u16,
    end: u16,
    out: u32,
    s: u32,
    h: &mut impl Round3Host,
) -> u32 {
    let rc = fn_24e2f8(m, p, out, s, h);
    if rc >= 0x80000000 {
        return rc;
    }
    let a = m.word(out);
    m.put_half(a + 6, block as u16);
    m.put_half(a + 4, domain as u16);
    m.put_half(a, begin);
    m.put_half(a + 2, end);
    m.put_byte(a + 8, 1);
    rc
}
fn domain_open_log(
    m: &mut Memory<'_>,
    p: u32,
    t: &DomainOpenLiterals,
    index: usize,
    args: &[u32],
    scratch: u32,
    h: &mut impl Round3Host,
) {
    let off = m.offset(scratch);
    let mut saved = [0u8; 32];
    saved.copy_from_slice(&m.bytes[off..off + 32]);
    for (i, v) in args.iter().enumerate() {
        m.put_word(scratch + i as u32 * 4, *v);
    }
    let level = if matches!(index, 3 | 4 | 5 | 8 | 9) {
        5
    } else {
        4
    };
    log_live::fn_1738e8(
        m,
        m.word(m.word(p) + 16),
        m.word(p + 0x48),
        level,
        0,
        t.formats[index],
        scratch,
        h,
    );
    m.bytes[off..off + 32].copy_from_slice(&saved);
}

pub fn fn_24bbc8(
    m: &mut Memory<'_>,
    p: u32,
    shared: &SpeechLiterals,
    t: &DomainOpenLiterals,
    scratch: u32,
    h: &mut impl Round3Host,
) -> u32 {
    if p == 0 {
        return norm::error_code(7);
    }
    assert_eq!(m.base, 0);
    let off = m.offset(scratch);
    let mut saved = [0u8; 1600];
    saved.copy_from_slice(&m.bytes[off..off + 1600]);
    let blocks = scratch;
    let count = scratch + 1000;
    let name = scratch + 1004;
    let out = scratch + 1040;
    let begin = scratch + 1048;
    let end = scratch + 1050;
    let token = scratch + 1052;
    let tag = scratch + 1054;
    let kind = scratch + 1056;
    for at in [count, begin, end, token, tag] {
        m.put_half(at, 0);
    }
    m.put_byte(kind, 0);
    m.put_word(out, 0);
    let logs = scratch + 1120;
    let call_scratch = scratch + 1200;
    let result = 'domains: {
        domain_open_log(m, p, t, 0, &[], logs, h);
        let mut rc = Host::call(h, m, 0x24ac00, &[p, t.trace, 1]);
        if rc >= 0x80000000 {
            break 'domains rc;
        }
        norm::match_ids(
            m.bytes,
            p,
            half(m, p + 0x6c) as u32,
            m.word(p + 0x70),
            blocks,
            count,
            0,
        );
        let n = half(m, count);
        domain_open_log(m, p, t, 1, &[n as u32], logs, h);
        if n == 0 {
            break 'domains 0;
        }
        let mut block = 0;
        for i in 0..n {
            domain_value_live(
                m,
                p,
                half(m, blocks + i as u32 * 2) as u32,
                &mut block,
                shared,
                logs,
                h,
            );
            if rc >= 0x80000000 {
                break 'domains rc;
            }
            for j in 0..half(m, block + 12) {
                let d = m.word(block + 8) + j as u32 * 28;
                let property = m.word(block + 16) + half(m, d + 8) as u32 * 8;
                domain_open_log(
                    m,
                    p,
                    t,
                    2,
                    &[
                        i as u32,
                        j as u32,
                        m.word(block),
                        m.byte(block + 4) as u32,
                        m.word(d + 20) + half(m, d) as u32,
                        if m.byte(d + 26) == 1 {
                            t.remapped
                        } else {
                            t.empty
                        },
                        if half(m, property + 4) == 0 {
                            t.virtual_domain
                        } else {
                            t.empty
                        },
                    ],
                    logs,
                    h,
                );
            }
        }
        rc = Host::call(
            h,
            m,
            m.word(m.word(p + 12) + 52),
            &[m.word(p + 16), m.word(p + 20), 1, 0, tag],
        );
        if rc >= 0x80000000 {
            break 'domains rc;
        }
        rc = Host::call(
            h,
            m,
            m.word(m.word(p + 12) + 52),
            &[
                m.word(p + 16),
                m.word(p + 20),
                3,
                half(m, tag) as u32,
                token,
            ],
        );
        if rc != 0 {
            break 'domains rc;
        }
        while half(m, token) != 0 {
            rc = rule_live(m, p, half(m, token) as u32, kind, 0, call_scratch, h);
            if rc >= 0x80000000 {
                break 'domains rc;
            }
            if m.byte(kind) == 5 {
                let mut mapped_domain = 0u16;
                let mut mapped_block = 0u16;
                let mut selected = 0u16;
                let mut matched_id = 0u32;
                let mut mapped_name = 0u32;
                rc = rule_live(m, p, half(m, token) as u32, begin, 1, call_scratch, h);
                if rc >= 0x80000000 {
                    break 'domains rc;
                }
                rc = rule_live(m, p, half(m, token) as u32, end, 2, call_scratch, h);
                if rc >= 0x80000000 {
                    break 'domains rc;
                }
                rc = 0;
                m.put_byte(name, 0);
                m.put_half(call_scratch, 0);
                m.put_half(call_scratch + 4, 0);
                let status = Host::call(
                    h,
                    m,
                    m.word(m.word(p + 12) + 92),
                    &[
                        m.word(p + 16),
                        m.word(p + 20),
                        half(m, token) as u32,
                        4,
                        call_scratch,
                    ],
                );
                if status < 0x80000000 && half(m, call_scratch) == 1 {
                    Host::call(
                        h,
                        m,
                        m.word(m.word(p + 12) + 84),
                        &[
                            m.word(p + 16),
                            m.word(p + 20),
                            half(m, token) as u32,
                            4,
                            32,
                            name,
                            call_scratch + 4,
                        ],
                    );
                }
                domain_open_log(
                    m,
                    p,
                    t,
                    3,
                    &[name, half(m, begin) as u32, half(m, end) as u32],
                    logs,
                    h,
                );
                for i in 0..n {
                    domain_value_live(
                        m,
                        p,
                        half(m, blocks + i as u32 * 2) as u32,
                        &mut block,
                        shared,
                        logs,
                        h,
                    );
                    if rc >= 0x80000000 {
                        break 'domains rc;
                    }
                    let remappable = norm::find_named(
                        m.bytes,
                        p,
                        name,
                        &mut mapped_name,
                        &mut mapped_domain,
                        &mut mapped_block,
                    );
                    let mut matched = 0;
                    if remappable != 1 {
                        for search in 0..half(m, block + 12) {
                            let d = m.word(block + 8) + search as u32 * 28;
                            if frontend::compare_str(
                                string_slice(m, m.word(d + 20) + half(m, d) as u32),
                                string_slice(m, name),
                            ) == 0
                            {
                                matched = block;
                                matched_id = half(m, blocks + i as u32 * 2) as u32;
                                selected = search;
                                break;
                            }
                        }
                    } else {
                        domain_open_log(m, p, t, 4, &[name, mapped_name], logs, h);
                        matched_id = mapped_block as u32;
                        selected = mapped_domain;
                        rc = domain_value_live(m, p, matched_id, &mut matched, shared, logs, h);
                        if rc >= 0x80000000 {
                            break 'domains rc;
                        }
                    }
                    let mut additional = false;
                    if matched != 0 {
                        let d = m.word(matched + 8) + selected as u32 * 28;
                        rc = set_active(
                            m,
                            p,
                            matched_id,
                            selected as u32,
                            half(m, begin),
                            half(m, end),
                            out,
                            call_scratch,
                            h,
                        );
                        if rc >= 0x80000000 {
                            break 'domains rc;
                        }
                        let a = m.word(out);
                        domain_open_log(
                            m,
                            p,
                            t,
                            5,
                            &[
                                m.word(matched),
                                m.word(d + 20) + half(m, d) as u32,
                                half(m, a) as i16 as i32 as u32,
                                half(m, a + 2) as i16 as i32 as u32,
                            ],
                            logs,
                            h,
                        );
                        additional = frontend::compare_str(
                            string_slice(m, m.word(d + 20) + half(m, d) as u32),
                            string_slice(m, m.word(p + 0x54)),
                        ) == 0
                            || m.byte(d + 26) == 1;
                    } else if remappable == 1 {
                        diagnostic(
                            m,
                            p,
                            m.word(p + 0x48),
                            0x5de9,
                            t.formats[6],
                            &[t.formats[7], mapped_name],
                            logs,
                            h,
                        );
                    }
                    if additional {
                        domain_open_log(m, p, t, 8, &[], logs, h);
                        for k in 0..half(m, block + 12) {
                            let d = m.word(block + 8) + k as u32 * 28;
                            if k != selected
                                && (frontend::compare_str(
                                    string_slice(m, m.word(d + 20) + half(m, d) as u32),
                                    string_slice(m, m.word(p + 0x54)),
                                ) == 0
                                    || m.byte(d + 26) == 1)
                            {
                                rc = set_active(
                                    m,
                                    p,
                                    half(m, blocks + i as u32 * 2) as u32,
                                    k as u32,
                                    half(m, begin),
                                    half(m, end),
                                    out,
                                    call_scratch,
                                    h,
                                );
                                if rc >= 0x80000000 {
                                    break 'domains rc;
                                }
                                let a = m.word(out);
                                domain_open_log(
                                    m,
                                    p,
                                    t,
                                    9,
                                    &[
                                        m.word(block),
                                        m.word(d + 20) + half(m, d) as u32,
                                        half(m, a) as i16 as i32 as u32,
                                        half(m, a + 2) as i16 as i32 as u32,
                                    ],
                                    logs,
                                    h,
                                );
                            }
                        }
                    }
                }
            }
            if Host::call(
                h,
                m,
                m.word(m.word(p + 12) + 60),
                &[m.word(p + 16), m.word(p + 20), half(m, token) as u32, token],
            ) != 0
            {
                break;
            }
        }
        domain_open_log(m, p, t, 10, &[rc], logs, h);
        rc
    };
    m.bytes[off..off + 1600].copy_from_slice(&saved);
    result
}

fn lattice_at(p: u32, i: i32) -> u32 {
    p.wrapping_add((i as u32).wrapping_mul(4))
}
fn lattice_get(m: &Memory<'_>, p: u32, i: i32) -> i32 {
    m.word(lattice_at(p, i)) as i32
}
fn lattice_elem(m: &Memory<'_>, table: u32, j: i32, i: i32) -> u32 {
    lattice_at(m.word(lattice_at(table, j)), i)
}
fn lattice_asr(v: i32, n: u32) -> i32 {
    v >> ((n & 255).min(31))
}
fn lattice_lsl(v: i32, n: u32) -> i32 {
    if n & 255 >= 32 {
        0
    } else {
        v.wrapping_shl(n & 255)
    }
}
fn lattice_div(v: i32, d: i32) -> i32 {
    crate::runtime::signed_division(v, d)
        .expect("ARM division by zero")
        .quotient
}

fn band_m(m: &mut Memory<'_>, p: u32, which: u32, a: u32, b: u32, c: u32) {
    let n = m.word(p + 32) as i32;
    let width = m.word(p + 36) as i32;
    if which == 0x2a3c44 {
        let input = m.word(p + 64);
        let rows = m.word(p + 60);
        let out = m.word(p + 56);
        let q = lattice_asr(lattice_get(m, m.word(rows), 0), b.wrapping_sub(a >> 1));
        m.put_word(
            out,
            lattice_lsl(
                lattice_div(lattice_get(m, input, 0), q),
                c.wrapping_sub(a >> 1),
            ) as u32,
        );
        for i in 1..n {
            let mut acc = 0i32;
            for j in 1..width {
                if i - j >= 0 {
                    let coefficient = lattice_get(m, m.word(lattice_at(m.word(p + 60), i - j)), j);
                    if coefficient != 0 {
                        acc = acc.wrapping_add(lattice_asr(coefficient, c >> 1).wrapping_mul(
                            lattice_asr(lattice_get(m, m.word(p + 56), i - j), c >> 1),
                        ));
                    }
                }
            }
            let d = a.wrapping_sub(b) as i32;
            let v = if d >= 0 {
                lattice_asr(lattice_get(m, m.word(p + 64), i), d as u32)
            } else {
                lattice_lsl(lattice_get(m, m.word(p + 64), i), b.wrapping_sub(a))
            };
            let q = lattice_asr(
                lattice_get(m, m.word(lattice_at(m.word(p + 60), i)), 0),
                c >> 1,
            );
            let v = lattice_lsl(lattice_div(v.wrapping_sub(acc), q), c >> 1);
            m.put_word(lattice_at(m.word(p + 56), i), v as u32);
        }
    } else {
        for i in (0..n).rev() {
            let row = m.word(lattice_at(m.word(p + 60), i));
            let mut acc = 0i32;
            let shift = if which == 0x2a3fe4 {
                9
            } else {
                ((c.wrapping_add(a).wrapping_sub(b)) as i32 >> 1) as u32
            };
            for j in 1..if which == 0x2a3fe4 { 3 } else { width } {
                if i + j < n {
                    let coefficient = lattice_get(m, row, j);
                    if which == 0x2a3fe4 || coefficient != 0 {
                        acc = acc.wrapping_add(lattice_asr(coefficient, shift).wrapping_mul(
                            lattice_asr(lattice_get(m, m.word(p + 44), i + j), shift),
                        ));
                    }
                }
            }
            let v = lattice_get(m, m.word(p + 56), i).wrapping_sub(acc);
            let div_shift = if which == 0x2a3fe4 {
                6
            } else {
                c.wrapping_sub(b >> 1)
            };
            let q = lattice_asr(lattice_get(m, row, 0), div_shift);
            let v = if which == 0x2a3fe4 && q == 0 && i + 2 >= n {
                v
            } else {
                lattice_div(v, q)
            };
            let result = lattice_lsl(
                v,
                if which == 0x2a3fe4 {
                    12
                } else {
                    a.wrapping_sub(b >> 1)
                },
            );
            m.put_word(lattice_at(m.word(p + 44), i), result as u32);
        }
    }
}

pub fn fn_2a24f8<H: Round3Host + crate::speech_part_5::Host>(
    m: &mut Memory<'_>,
    context: u32,
    p: u32,
    h: &mut H,
) {
    m.put_word(p + 56, m.word(m.word(p + 48)));
    let mut output = m.word(p + 44);
    m.put_word(p + 64, output);
    if m.word(p + 36) == 3 {
        for i in 0..m.word(p + 32) as i32 {
            let scale = lattice_get(m, m.word(m.word(p + 52)), i);
            let sample = lattice_get(m, m.word(m.word(p + 48)), i) >> 8;
            m.put_word(output, (scale >> 8).wrapping_mul(sample) as u32);
            let row = m.word(lattice_at(m.word(p + 60), i));
            m.put_word(row + 8, 0);
            m.put_word(row, scale as u32);
            m.put_word(row + 4, 0);
            let mut j = 1;
            while j < (m.word(m.word(p + 40) + 24) as i32) {
                let bank = m.word(p + 40);
                let weight = m.word(lattice_at(m.word(bank + 32), j));
                for (tap, k) in [(1, i - 1), (0, i), (-1, i + 1)] {
                    let w = lattice_get(m, weight, tap);
                    if w == 0 || k < 0 || k >= m.word(p + 32) as i32 {
                        continue;
                    }
                    let value = (lattice_get(m, lattice_elem(m, m.word(p + 52), j, k), 0) >> 8)
                        .wrapping_mul(w)
                        >> 8;
                    let sample = lattice_get(m, lattice_elem(m, m.word(p + 48), j, k), 0) >> 8;
                    m.put_word(
                        output,
                        lattice_get(m, output, 0).wrapping_add(value.wrapping_mul(sample)) as u32,
                    );
                    let coeff = m.word(m.word(p + 40) + 36);
                    let c0 = lattice_get(m, lattice_elem(m, coeff, j, tap), 0);
                    m.put_word(
                        row,
                        lattice_get(m, row, 0).wrapping_add(value.wrapping_mul(c0)) as u32,
                    );
                    if tap <= 0 && i + 1 < m.word(p + 32) as i32 {
                        let c1 = lattice_get(m, lattice_elem(m, coeff, j, tap + 1), 0);
                        m.put_word(
                            row + 4,
                            lattice_get(m, row, 1).wrapping_add(value.wrapping_mul(c1)) as u32,
                        );
                    }
                    if tap == -1 && i + 2 < m.word(p + 32) as i32 {
                        let c2 = lattice_get(m, lattice_elem(m, coeff, j, 1), 0);
                        m.put_word(
                            row + 8,
                            lattice_get(m, row, 2).wrapping_add(value.wrapping_mul(c2)) as u32,
                        );
                    }
                }
                j += 1;
            }
            output = output.wrapping_add(4);
        }
    } else {
        crate::speech_part_5::fn_2a178c(m, p, 16, 16, output);
    }
    if m.word(p + 36) == 3 {
        crate::reentrant::factor_lattice_m(m, h, context, p);
    } else {
        crate::speech_part_5::fn_2a19ac(m, context, p, 16, h);
    }
    if m.word(p + 36) == 3 {
        crate::reentrant::forward_lattice_m(m, h, p, 16, 16, 20);
        band_m(m, p, 0x2a3fe4, 0, 0, 0);
    } else {
        band_m(m, p, 0x2a3c44, 16, 16, 20);
        band_m(m, p, 0x2a3ec4, 16, 20, 16);
    }
}

pub fn fetch_parameter_m(
    m: &mut Memory<'_>,
    registry: u32,
    key: u32,
    out: u32,
    lock: &mut impl crate::reentrant2::StateLockM,
    cmp: impl FnMut(u32, u32, &mut [u8]) -> i32,
) -> u32 {
    assert_eq!(m.base, 0);
    let mut value = m.word(out);
    let rc = crate::reentrant2::fetch_param_m(
        m.bytes,
        if registry == 0 { None } else { Some(registry) },
        key,
        Some(&mut value),
        lock,
        cmp,
    );
    m.put_word(out, value);
    rc
}

pub fn parameter_lock_m(m: &mut Memory<'_>, lock: u32, release: bool, h: &mut impl Host) -> u32 {
    if lock == 0 {
        return if release {
            frontend::release_lock(&[], None, |_| unreachable!())
        } else {
            frontend::acquire_lock(&[], None, |_| unreachable!())
        };
    }
    let inner = m.word(m.word(lock) + 8);
    let method = m.word(inner + if release { 12 } else { 8 });
    let target = m.word(lock + 8);
    let mut frame = [0u8; 44];
    for (at, value) in [
        (4, 16u32),
        (12, target),
        (24, 28),
        (36, method),
        (40, method),
    ] {
        frame[at..at + 4].copy_from_slice(&value.to_le_bytes());
    }
    if release {
        frontend::release_lock(&frame, Some(4), |obj| Host::call(h, m, method, &[obj]))
    } else {
        frontend::acquire_lock(&frame, Some(4), |obj| Host::call(h, m, method, &[obj]))
    }
}
