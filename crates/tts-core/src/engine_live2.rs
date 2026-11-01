

use crate::{boot::Memory, frontend, speech_part_1, speech_part_2};
use crate::rom_region::{addr, Rom};

const OUTPUT_NAMES: u32 = 0x8f9328;

const RESOURCE_NAMES: u32 = 0x8f9e7c;

const CLASS_TABLE: u32 = 0x8dc30c;

const CACHED_HANDLE: u32 = 0x8d7660;

const DEFAULT_CONTEXT: u32 = 0x8d7640;

const STREAM_CALLBACKS: u32 = 0x8fa504;

const SELECT_CATEGORY: u32 = 0x8fa510;

const SELECT_MIME: [u32; 5] = [0x227bb8, 0x227bfc, 0x227c2c, 0x227c5c, 0x227c8c];

const SELECT_CONTENT_NAME: u32 = 0x227be4;

const SELECT_FORMAT: u32 = 0x227bf0;

const PHONMAP_CATEGORY: u32 = 0x8f8b6e;

const MARKER_MODE: u32 = 0x1e3aa8;

const MARKER_ENABLED: u32 = 0x1e3ab4;

const MARKER_MIME: u32 = 0x1e3ac8;

const MARKER_CONTENT_NAME: u32 = 0x1e3af8;

const MARKER_FORMAT: u32 = 0x1e3b04;

const XCODER_SYNC: u32 = 0x2312cc;

const XCODER_NAME: u32 = 0x23122c;

const XCODER_MIME: u32 = 0x231b64;

const XCODER_FORMAT: u32 = 0x231b98;

const XCODER_CATEGORY: u32 = 0x8fa93c;



fn engine_environment(m: &Memory<'_>, rom: Rom) -> speech_part_2::Environment {
    speech_part_2::Environment {
        cached_handle: core::array::from_fn(|i| m.byte(addr(rom, CACHED_HANDLE) + i as u32)),
        default_context: addr(rom, DEFAULT_CONTEXT),
        input_types: [0; 2],
        output_types: [0; 2],
    }
}

pub fn fn_1deb04_m(_m: &mut Memory<'_>, p: u32, key: u32) -> u32 {
    let rc = frontend::validate_handle(p != 0, key, 0xf387, 40);
    if rc >= 0x80000000 {
        0x8a002008
    } else {
        rc
    }
}

pub fn fn_1e772c_m(_m: &mut Memory<'_>, p: u32, key: u32) -> u32 {
    let rc = frontend::validate_handle(p != 0, key, 0xf3b3, 0x3c);
    if rc >= 0x80000000 {
        0x8b802008
    } else {
        rc
    }
}

pub fn fn_1f51dc_m(_m: &mut Memory<'_>, p: u32, key: u32) -> u32 {
    let rc = frontend::validate_handle(p != 0, key, 0xf388, 0x40);
    if rc >= 0x80000000 {
        0x8a102008
    } else {
        rc
    }
}

pub fn fn_1f3dc0_m(
    m: &mut Memory<'_>,
    p: u32,
    tag: u32,
    engine: [u32; 2],
    rom: Rom,
    h: &mut impl speech_part_2::Host,
) -> u32 {
    let rc = frontend::validate_handle(p != 0, tag, 0xf382, 0x34);
    if rc >= 0x80000000 {
        return 0x8ba02008;
    }
    let streams = m.word(p + 8);
    let create = m.word(streams + 0x20);
    let release = m.word(streams + 0x24);
    let mut result = 0u32;
    let mut i = 0u32;
    while i < 2 {
        let mime = m.word(addr(rom, OUTPUT_NAMES) + i * 4);
        result = h.call(
            m,
            create,
            &[engine[0], engine[1], 0xba, mime, 0, 1, p + 0x24 + i * 8],
        );
        if result >= 0x80000000 {
            break;
        }
        i += 1;
    }
    if result >= 0x80000000 {
        let mut j = 0u32;
        while j < i {
            let out = p + 0x24 + j * 8;
            if m.word(out) != 0 {
                h.call(m, release, &[m.word(out), m.word(out + 4)]);
                m.put_word(out, m.word(addr(rom, 0x8d7660)));
                m.put_word(out + 4, m.word(addr(rom, 0x8d7664)));
            }
            j += 1;
        }
    }
    result
}

pub fn fn_210398_m(
    m: &mut Memory<'_>,
    owner: u32,
    key: u32,
    first: u32,
    second: u32,
    rom: Rom,
    h: &mut impl speech_part_2::Host,
) -> u32 {
    let rc = frontend::validate_handle(owner != 0, key, 0xf381, 52);
    if rc >= 0x80000000 {
        return 0x89a02008;
    }
    let interface = m.word(owner + 8);
    let create = m.word(interface + 0x20);
    let destroy = m.word(interface + 0x24);
    let get_value = m.word(interface + 0x4c);
    let names = addr(rom, RESOURCE_NAMES);
    let mut result = 0u32;
    let mut i = 0u32;
    while i < 3 {
        let name = m.word(names + i * 4);
        let elements = if i == 1 { h.call(m, get_value, &[]) } else { 0 };
        result = h.call(
            m,
            create,
            &[first, second, 0x9a, name, elements, 1, owner + 0x18 + i * 8],
        );
        if result >= 0x80000000 {
            break;
        }
        i += 1;
    }
    if result >= 0x80000000 {
        let mut j = 0u32;
        while j < i {
            let slot = owner + 0x18 + j * 8;
            if m.word(slot) != 0 {
                h.call(m, destroy, &[m.word(slot), m.word(slot + 4)]);
                m.put_word(slot, 0);
                m.put_word(slot + 4, 0);
            }
            j += 1;
        }
    }
    result
}

pub fn fn_2279b4_m<H: speech_part_2::Round3Host>(
    m: &mut Memory<'_>,
    p: u32,
    tag: u32,
    stream: [u32; 2],
    scratch: u32,
    h: &mut H,
    rom: Rom,
) -> u32 {
    let e = engine_environment(m, rom);
    let t = speech_part_2::SelectionOpenLiterals {
        mime: core::array::from_fn(|i| addr(rom, SELECT_MIME[i])),
        category: addr(rom, SELECT_CATEGORY),
        format: addr(rom, SELECT_FORMAT),
        content_type: addr(rom, SELECT_CONTENT_NAME),
        callbacks: core::array::from_fn(|i| m.word(addr(rom, STREAM_CALLBACKS) + 0xc0 + i as u32 * 4)),
    };
    speech_part_2::fn_2279b4(m, p, tag, stream, &e, &t, scratch, h)
}

pub fn fn_1e3938_m<H: speech_part_1::Host + crate::speech_part_6::Host>(
    m: &mut Memory<'_>,
    p: u32,
    tag: u32,
    source: [u32; 2],
    scratch: u32,
    h: &mut H,
    rom: Rom,
) -> u32 {
    let env = speech_part_1::MarkerEnvironment {
        mode: addr(rom, MARKER_MODE),
        enabled: addr(rom, MARKER_ENABLED),
        mime: addr(rom, MARKER_MIME),
        category: addr(rom, PHONMAP_CATEGORY),
        format: addr(rom, MARKER_FORMAT),
        content_type: addr(rom, MARKER_CONTENT_NAME),
        ctype: core::array::from_fn(|i| m.half(addr(rom, CLASS_TABLE) + i as u32 * 2)),
    };
    speech_part_1::fn_1e3938(m, p, tag, source, &env, scratch, h)
}

pub fn fn_2319f0_m<H: speech_part_2::Host + crate::boot_part_b::FormatHost>(
    m: &mut Memory<'_>,
    p: u32,
    tag: u32,
    content: [u32; 2],
    scratch: u32,
    h: &mut H,
    rom: Rom,
) -> u32 {
    let e = engine_environment(m, rom);
    let t = speech_part_2::MarkerTypes {
        sync: addr(rom, XCODER_SYNC),
        markers: addr(rom, XCODER_MIME),
        category: addr(rom, XCODER_CATEGORY),
        content_type: addr(rom, XCODER_NAME),
        format: addr(rom, XCODER_FORMAT),
    };
    speech_part_2::fn_2319f0(m, p, tag, content, &e, &t, scratch, h)
}
