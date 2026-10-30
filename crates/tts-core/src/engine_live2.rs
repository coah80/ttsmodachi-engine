//! Live component roots owned by lane wire-a2.
//!
//! The eight processStart entries between fn_1deb04 and fn_2319f0 in the ARM
//! component order read at the call site 0x153eb0 of fn_153c5c. Every callee
//! reenters the current caller-owned arena; the ROM literals are the mapped
//! addresses of the US build, the same ones the C decomp names.
//!
//! The ports are certified from their own ARM boundary capture: the entry
//! image of the root is the input, the exit image of the same root is the
//! comparison target, and the persistent guest state plus the return value
//! must match byte for byte.

use crate::{boot::Memory, frontend, speech_part_1, speech_part_2};

/// D_8f9328. The two output MIME names of the selection component.
const OUTPUT_NAMES: u32 = 0x8f9328;
/// D_8f9e7c. The three resource names of the resource component.
const RESOURCE_NAMES: u32 = 0x8f9e7c;
/// The u16 class table the integer parameter parser reads.
const CLASS_TABLE: u32 = 0x8dc30c;
/// The cached engine handle pair.
const CACHED_HANDLE: u32 = 0x8d7660;
/// The default engine context.
const DEFAULT_CONTEXT: u32 = 0x8d7640;
/// D_8fa504. The stream callbacks the selection open hands to its last open.
const STREAM_CALLBACKS: u32 = 0x8fa504;
/// D_8fa510. The SELECT_BET2 log category.
const SELECT_CATEGORY: u32 = 0x8fa510;
/// The five selection MIME names, in the order the ARM opens them.
const SELECT_MIME: [u32; 5] = [0x227bb8, 0x227bfc, 0x227c2c, 0x227c5c, 0x227c8c];
/// "contentType" beside the selection MIME names.
const SELECT_CONTENT_NAME: u32 = 0x227be4;
/// "%s%s" beside the selection MIME names.
const SELECT_FORMAT: u32 = 0x227bf0;
/// D_8f8b6e. The PHONMAP log category.
const PHONMAP_CATEGORY: u32 = 0x8f8b6e;
/// "markermode".
const MARKER_MODE: u32 = 0x1e3aa8;
/// "phonmapmrkenabled".
const MARKER_ENABLED: u32 = 0x1e3ab4;
/// The marker MIME name beside the phonmap literals.
const MARKER_MIME: u32 = 0x1e3ac8;
/// "contentType" beside the phonmap literals.
const MARKER_CONTENT_NAME: u32 = 0x1e3af8;
/// "%s%s" beside the phonmap literals.
const MARKER_FORMAT: u32 = 0x1e3b04;
/// D_2312cc. The xcoder sync content type.
const XCODER_SYNC: u32 = 0x2312cc;
/// D_23122c. The xcoder "contentType" name.
const XCODER_NAME: u32 = 0x23122c;
/// The marker MIME name beside the xcoder literals.
const XCODER_MIME: u32 = 0x231b64;
/// "%s%s" beside the xcoder literals.
const XCODER_FORMAT: u32 = 0x231b98;
/// D_8fa93c. The XCODER log category.
const XCODER_CATEGORY: u32 = 0x8fa93c;
/// fn_176fd4. The default handle pair the stream close leaves behind.
const DEFAULT_HANDLE: [u32; 2] = [0x8d7660, 0xe92d4038];

/// The engine context the selection and xcoder ports resolve.
fn engine_environment(m: &Memory<'_>) -> speech_part_2::Environment {
    speech_part_2::Environment {
        cached_handle: core::array::from_fn(|i| m.byte(CACHED_HANDLE + i as u32)),
        default_context: DEFAULT_CONTEXT,
        input_types: [0; 2],
        output_types: [0; 2],
    }
}

/// fn_1deb04. The component validates its own handle and changes nothing.
///
/// The handle tag must be 0xf387 and the object must be present, so the
/// component only fails when the caller passed the wrong component key.
pub fn fn_1deb04_m(_m: &mut Memory<'_>, p: u32, key: u32) -> u32 {
    let rc = frontend::validate_handle(p != 0, key, 0xf387, 40);
    if rc >= 0x80000000 {
        0x8a002008
    } else {
        rc
    }
}

/// fn_1e772c. The component validates its own handle and changes nothing.
pub fn fn_1e772c_m(_m: &mut Memory<'_>, p: u32, key: u32) -> u32 {
    let rc = frontend::validate_handle(p != 0, key, 0xf3b3, 0x3c);
    if rc >= 0x80000000 {
        0x8b802008
    } else {
        rc
    }
}

/// fn_1f51dc. The component validates its own handle and changes nothing.
pub fn fn_1f51dc_m(_m: &mut Memory<'_>, p: u32, key: u32) -> u32 {
    let rc = frontend::validate_handle(p != 0, key, 0xf388, 0x40);
    if rc >= 0x80000000 {
        0x8a102008
    } else {
        rc
    }
}

/// fn_1f3dc0. Opens the two output streams of the selection component.
///
/// The stream interface is the one the component object holds, so the create
/// and release methods are read from the live vtable. A failed open closes the
/// streams that were already opened and leaves the default handle behind.
pub fn fn_1f3dc0_m(
    m: &mut Memory<'_>,
    p: u32,
    tag: u32,
    engine: [u32; 2],
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
        let mime = m.word(OUTPUT_NAMES + i * 4);
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
                m.put_word(out, DEFAULT_HANDLE[0]);
                m.put_word(out + 4, DEFAULT_HANDLE[1]);
            }
            j += 1;
        }
    }
    result
}

/// fn_210398. Opens the three resources of the resource component.
///
/// The middle resource takes the interface's current value as its element
/// count. A failed open destroys the resources that were already created.
pub fn fn_210398_m(
    m: &mut Memory<'_>,
    owner: u32,
    key: u32,
    first: u32,
    second: u32,
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
    let names = RESOURCE_NAMES;
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

/// fn_2279b4. Live selection open.
///
/// The five MIME names, the log literals and the stream callbacks are the
/// mapped ROM addresses of the US build. scratch is 64 disjoint bytes that the
/// port restores before it returns.
pub fn fn_2279b4_m<H: speech_part_2::Round3Host>(
    m: &mut Memory<'_>,
    p: u32,
    tag: u32,
    stream: [u32; 2],
    scratch: u32,
    h: &mut H,
) -> u32 {
    let e = engine_environment(m);
    let t = speech_part_2::SelectionOpenLiterals {
        mime: SELECT_MIME,
        category: SELECT_CATEGORY,
        format: SELECT_FORMAT,
        content_type: SELECT_CONTENT_NAME,
        callbacks: core::array::from_fn(|i| m.word(STREAM_CALLBACKS + 0xc0 + i as u32 * 4)),
    };
    speech_part_2::fn_2279b4(m, p, tag, stream, &e, &t, scratch, h)
}

/// fn_1e3938. Live phonmap marker open.
///
/// The parameter keys, the MIME name and the log literals are the mapped ROM
/// addresses of the US build. scratch is 136 disjoint bytes that the port
/// restores before it returns.
pub fn fn_1e3938_m<H: speech_part_1::Host + crate::speech_part_6::Host>(
    m: &mut Memory<'_>,
    p: u32,
    tag: u32,
    source: [u32; 2],
    scratch: u32,
    h: &mut H,
) -> u32 {
    let env = speech_part_1::MarkerEnvironment {
        mode: MARKER_MODE,
        enabled: MARKER_ENABLED,
        mime: MARKER_MIME,
        category: PHONMAP_CATEGORY,
        format: MARKER_FORMAT,
        content_type: MARKER_CONTENT_NAME,
        ctype: core::array::from_fn(|i| m.half(CLASS_TABLE + i as u32 * 2)),
    };
    speech_part_1::fn_1e3938(m, p, tag, source, &env, scratch, h)
}

/// fn_2319f0. Live xcoder marker open.
///
/// The sync and marker content types, the log literals and the "contentType"
/// name are the mapped ROM addresses of the US build. scratch is 28 disjoint
/// bytes that the port restores before it returns.
pub fn fn_2319f0_m<H: speech_part_2::Host + crate::boot_part_b::FormatHost>(
    m: &mut Memory<'_>,
    p: u32,
    tag: u32,
    content: [u32; 2],
    scratch: u32,
    h: &mut H,
) -> u32 {
    let e = engine_environment(m);
    let t = speech_part_2::MarkerTypes {
        sync: XCODER_SYNC,
        markers: XCODER_MIME,
        category: XCODER_CATEGORY,
        content_type: XCODER_NAME,
        format: XCODER_FORMAT,
    };
    speech_part_2::fn_2319f0(m, p, tag, content, &e, &t, scratch, h)
}
