//! Voice resource loading on the boot path. Guest state is caller-owned.
//! Entry-isolation replay passes; whole-call-tree certification is incomplete.
//! Host still contains existing callee ports that need real adapters.
use crate::boot::Memory;
use crate::frontend as f;
use crate::leftovers_1::OpenRuntime;

/// Foreign calls must receive the current arena, including local out parameters.
pub trait Host {
    fn call(&mut self, m: &mut Memory<'_>, address: u32, args: &[u32]) -> u32;
}
/// Reconstructed string literals in the same order as the source's lookups.
/// The caller places their NUL-terminated concatenation at `literal_base`.
pub const TEXT: &[&str] = &[
    "select",
    "/",
    "voice",
    "voicemodel",
    "frequencyhz",
    "f",
    "bet2voicemapatstartup",
    "HMPG",
    "VCFG",
    "bet2voiceoverride.",
    "FS",
    "ALPHA",
    "BETA",
    "VUV_REL_AMP",
    "VOL",
    "MVF_BOOST",
    "FRAME_SIZE",
    "LOGLEVEL",
    "BREAKERSTR",
    "BREAKMAXDUR",
    "SPLIT",
    "UV",
    "MVF_UV",
    "LT_STREAMS",
    "ORDER",
    "RHO",
    "DATATYPE",
    "VSYNTH_INIT",
    "ORDER_BEGIN_FAST",
    "PGEN_COMPLEX_LF0",
    "PGEN_COMPLEX_MVF",
    "PGEN_COMPLEX_CEP_ST",
    "PGEN_COMPLEX_CEP_LT",
    "high",
    "default",
    "low",
    "medium",
    "FEEX",
    "TREE",
    "PDF ",
    "CBK ",
    "DWIN",
    "MCP ",
    "LF0 ",
    "DUR ",
    "MVF ",
    "VCF ",
    "DYN ",
    "ACC ",
    "bet2voiceexcitetype",
    "bet2voicealpha",
    "bet2voicebeta",
    "bet2voicevol",
    "bet2voicevuvrelamp",
    "bet2voicemvfboost",
    "bet2voiceframesize",
    "bet2voiceorder",
    "bet2voicevsynthinit",
    "FVPG",
    "%s%s%s%s",
    "message",
    "Corrupt voice configuration chunk",
    "file",
    "parameter",
    "value",
    "Specified MCEP order %d is higher than order of the data (%d)",
    "%s%s%s%x",
    "lhError",
];
/// Scratch must be disjoint from all live objects and literal data. Its 1024
/// bytes model ABI-local storage and are restored on return, as in lane 3.
pub struct Environment<'a> {
    pub scratch: u32,
    pub literal_base: u32,
    pub version: u32,
    pub module: u32,
    pub character_classes: &'a [u16; 256],
    /// ROM addresses of the literals, parallel to `TEXT`. A zero entry keeps
    /// the literal in the caller's pool. The ARM passes the ROM address of
    /// every literal, and a parameter key ends up stored in the arena, so a
    /// live replay has to use the same addresses.
    pub literal_addresses: Option<&'a [u32]>,
}
fn lit(e: &Environment<'_>, name: &str) -> u32 {
    let mut p = e.literal_base;
    for (i, s) in TEXT.iter().enumerate() {
        if *s == name {
            if let Some(table) = e.literal_addresses {
                if table[i] != 0 {
                    return table[i];
                }
            }
            return p;
        }
        p += s.len() as u32 + 1;
    }
    panic!("missing voice-loader literal");
}
fn bad(r: u32) -> bool {
    r >= 0x80000000
}
fn code(r: u32) -> bool {
    r.wrapping_shl(19) == 0x14u32.wrapping_shl(19)
}
fn half(m: &Memory<'_>, p: u32) -> u16 {
    u16::from_le_bytes([m.byte(p), m.byte(p + 1)])
}
fn eq(m: &Memory<'_>, p: u32, s: &str) -> bool {
    s.as_bytes()
        .iter()
        .enumerate()
        .all(|(i, b)| m.byte(p + i as u32) == *b)
        && m.byte(p + s.len() as u32) == 0
}
fn leaf(m: &mut Memory<'_>, f: u32, a: &[u32]) -> u32 {
    crate::selection::arena_leaf::run(
        &mut crate::selection::arena::Arena::new(m.base, m.bytes),
        f,
        a,
    )
}
fn copy(m: &mut Memory<'_>, e: &Environment<'_>, to: u32, s: &str) {
    leaf(m, 0x195cd0, &[to, lit(e, s)]);
}
fn append(m: &mut Memory<'_>, to: u32, from: u32) {
    leaf(m, 0x195c90, &[to, from]);
}
fn number(m: &mut Memory<'_>, to: u32, v: i32) {
    let at = m.offset(to);
    crate::frontend::render_signed(v, &mut m.bytes[at..at + 36], 10).expect("decimal buffer");
}
fn integer(m: &Memory<'_>, e: &Environment<'_>, p: u32) -> i32 {
    crate::frontend::parse_int(e.character_classes, m.bytes, m.offset(p) as u32)
}
fn div(a: i32, b: i32) -> i32 {
    crate::runtime::signed_division(a, b)
        .expect("voice frame size is nonzero")
        .quotient
}
fn query(m: &mut Memory<'_>, h: &mut impl Host, params: u32, key: u32, out: u32) -> u32 {
    h.call(m, 0x175a1c, &[params, key, out])
}
fn valid(m: &Memory<'_>, rc: u32, out: u32) -> bool {
    !bad(rc) && m.word(out) != 0 && m.byte(m.word(out)) != 0
}
fn override_key(m: &mut Memory<'_>, e: &Environment<'_>, p: u32, key: &str) {
    copy(m, e, p, "bet2voiceoverride.");
    append(m, p, lit(e, key));
}
struct Sel<'a, H>(&'a mut H);
impl<H: Host> crate::selection::arena::Host for Sel<'_, H> {
    fn call(
        &mut self,
        a: &mut crate::selection::arena::Arena<'_>,
        f: u32,
        args: &[u32],
    ) -> (u32, u32) {
        (
            self.0.call(
                &mut Memory {
                    base: a.base,
                    bytes: a.bytes,
                },
                f,
                args,
            ),
            0,
        )
    }
}
/// fn_242444. The chunk string read is a host call, so the replay compares
/// its arguments and its result.
fn read_string(
    m: &mut Memory<'_>,
    h: &mut impl Host,
    r: u32,
    src: u32,
    size: u32,
    pos: u32,
    dst: u32,
    len: u32,
) -> u32 {
    h.call(m, 0x242444, &[r, src, size, pos, dst, len])
}
/// fn_2a4864. The default voice configuration is a host call, so the replay
/// compares its argument and its result.
fn defaults(m: &mut Memory<'_>, h: &mut impl Host, p: u32) -> u32 {
    h.call(m, 0x2a4864, &[p])
}
fn drop_voice(m: &mut Memory<'_>, h: &mut impl Host, p: u32) {
    crate::selection::stage::teardown_voice_object(
        &mut crate::selection::arena::Arena::new(m.base, m.bytes),
        &mut Sel(h),
        p,
    );
}
/// fn_2a9090. The frame release is a host call, so the replay compares its
/// argument and its result.
fn release_frame(m: &mut Memory<'_>, h: &mut impl Host, p: u32) -> u32 {
    h.call(m, 0x2a9090, &[p])
}
fn release_block(m: &mut Memory<'_>, h: &mut impl Host, c: u32, p: u32) {
    crate::selection::arena_mid::release_workspace(
        &mut crate::selection::arena::Arena::new(m.base, m.bytes),
        &mut Sel(h),
        c,
        p,
    );
}
fn open(
    m: &mut Memory<'_>,
    h: &mut impl Host,
    e: &Environment<'_>,
    p: u32,
    path: u32,
    out: u32,
) -> u32 {
    let obj = m.word(p);
    let key = m.word(p + 4);
    crate::selection::arena_mid::open_voice_resource(
        &mut crate::selection::arena::Arena::new(m.base, m.bytes),
        &mut Sel(h),
        obj,
        key,
        0,
        path,
        lit(e, "HMPG"),
        e.version,
        out,
    )
}
fn clear_blocks(m: &mut Memory<'_>, h: &mut impl Host, p: u32, ctx: u32) {
    for i in 0..5 {
        let block = p + 0x638 + i * 0x34;
        if m.word(block + 0x1c) != 0 {
            release_block(m, h, ctx, block);
        }
    }
    leaf(m, 0x195bec, &[p + 0x638, 0, 5 * 0x34]);
}
fn clear_handles(m: &mut Memory<'_>, h: &mut impl Host, p: u32) {
    let mut i = 0;
    while i < half(m, p + 0x788) as u32 {
        h.call(m, 0x243d50, &[m.word(p + 0x748 + i * 4)]);
        i += 1;
    }
    m.put_half(p + 0x788, 0);
}
/// fn_2252b0. The complete load, override, model/window and cleanup state machine.
/// Missing foreign adapters are explicit Host calls, never successful defaults.
pub fn fn_2252b0(m: &mut Memory<'_>, p: u32, e: &Environment<'_>, h: &mut impl Host) -> u32 {
    let saved: [u8; 1024] = core::array::from_fn(|i| m.byte(e.scratch + i as u32));
    let rc = load(m, p, e, h);
    let at = m.offset(e.scratch);
    m.bytes[at..at + 1024].copy_from_slice(&saved);
    rc
}
fn load(m: &mut Memory<'_>, p: u32, e: &Environment<'_>, h: &mut impl Host) -> u32 {
    let ctx = m.word(p + 8);
    let cfg = p + 0x94;
    let model = p + 0x1ac;
    let s = e.scratch;
    // The frame model mirrors the ARM's frame of fn_2252b0: the same offset
    // from the frame base holds the same slot, so an ABI-local pointer names
    // the same slot in both and the bytes written through it line up.
    let path = s + 0x20;
    let key = s + 0xb0;
    let win = s + 0xb0;
    let value = s + 0xf0;
    let pname = s + 0x1f0;
    let tag = s + 0x230;
    let num = s + 0x240;
    let out1 = s + 0x10;
    let int1 = s + 0x26c;
    let int2 = s + 0x270;
    let size = s + 0x1c;
    let data = s + 0xac;
    let rd_slot = s + 0xa0;
    let val_slot = s + 0x14;
    // The ROM's fn_2252b0 reads the chunk strings into the frame's own
    // length slot: the callee writes the length there.
    let len_slot = s + 0x238;
    // The ROM reuses the value buffer as the window pointer array: its
    // fn_2252b0 passes the frame slot +0xf0 to fn_2a2f04 as the array, and
    // the same slot holds the override value while the configuration chunk
    // is read.
    let wins = s + 0xf0;
    m.put_word(out1, 0);
    m.put_word(size, 0);
    m.put_word(data, 0);
    m.put_word(rd_slot, 0);
    m.put_word(val_slot, 0);
    for i in 0..8 {
        m.put_byte(tag + i, 0);
    }
    let mut stage = 0u16;
    copy(m, e, path, "select");
    let r = query(m, h, m.word(ctx + 20), lit(e, "voice"), out1);
    if !valid(m, r, out1) {
        return 0x84e02000;
    }
    append(m, path, lit(e, "/"));
    append(m, path, m.word(out1));
    leaf(m, 0x1757a8, &[m.word(ctx + 20), m.word(out1)]);
    m.put_word(out1, 0);
    let r = query(m, h, m.word(ctx + 20), lit(e, "voicemodel"), out1);
    if !valid(m, r, out1) {
        return 0x84e02000;
    }
    append(m, path, lit(e, "/"));
    append(m, path, m.word(out1));
    leaf(m, 0x1757a8, &[m.word(ctx + 20), m.word(out1)]);
    m.put_word(int1, 0);
    let r = h.call(m, 0x175850, &[m.word(ctx + 20), lit(e, "frequencyhz"), int1]);
    let frequency = m.word(int1) as i32;
    if bad(r) || frequency <= 0 {
        return 0x84e02000;
    }
    number(m, num, div(frequency, 1000));
    append(m, path, lit(e, "f"));
    append(m, path, num);
    if leaf(m, 0x195d58, &[p + 20, path]) == 0 {
        return 0;
    }
    m.put_word(int2, 1);
    let r = h.call(
        m,
        0x175850,
        &[m.word(ctx + 20), lit(e, "bet2voicemapatstartup"), int2],
    );
    let map_at_start = if bad(r) { 1 } else { m.word(int2) };
    let r = open(m, h, e, p, path, rd_slot);
    if bad(r) {
        return r;
    }
    let rd = m.word(rd_slot);
    if [0u32, 1, 3, 4]
        .iter()
        .any(|i| m.word(p + 0x638 + i * 0x34 + 0x1c) != 0)
    {
        if m.word(p + 0x790) != 0 {
            release_frame(m, h, p + 0x78c);
            h.call(m, 0x179f3c, &[m.word(ctx + 12), m.word(p + 0x790)]);
            m.put_word(p + 0x790, 0);
        }
        clear_blocks(m, h, p, ctx);
        h.call(m, 0x2a41b8, &[model]);
        // fn_weak_f94 is unresolved in the ROM; RVCT emits no call.
        drop_voice(m, h, m.word(p + 16));
        m.put_word(p + 16, 0);
        clear_handles(m, h, p);
    }
    let mut rc = 'run: {
        let r = h.call(m, 0x179f30, &[m.word(ctx + 12), path, p + 0x790]);
        if bad(r) {
            break 'run r;
        }
        m.put_word(p + 0x78c, m.word(ctx + 12));
        defaults(m, h, cfg);
        stage = stage.wrapping_add(1);
        loop {
            let r = h.call(m, 0x24277c, &[rd, tag, size, 0]);
            if bad(r) {
                break 'run r;
            }
            if eq(m, tag, "VCFG") {
                break;
            }
            let r = h.call(m, 0x2422ac, &[rd]);
            if bad(r) {
                break 'run r;
            }
        }
        let r = h.call(m, 0x242590, &[rd, m.word(size), data]);
        if bad(r) {
            break 'run r;
        }
        let mut r = r;
        let mut pos = 0u32;
        let mut break_max = (m.word(cfg + 0x4c) as i32).wrapping_mul(m.word(cfg + 56) as i32);
        while pos < m.word(size) {
            m.put_byte(value, 0);
            m.put_byte(key, 0);
            m.put_word(len_slot, 64);
            r = read_string(m, h, rd, m.word(data), m.word(size), pos, key, len_slot);
            if bad(r) {
                break;
            }
            if pos < m.word(size) {
                pos = pos.wrapping_add(m.word(len_slot));
                m.put_word(len_slot, 256);
                r = read_string(m, h, rd, m.word(data), m.word(size), pos, value, len_slot);
                    if bad(r) {
                    break;
                }
            }
            pos = pos.wrapping_add(m.word(len_slot));
            copy(m, e, pname, "bet2voiceoverride.");
            append(m, pname, key);
            let q = query(m, h, m.word(ctx + 20), pname, val_slot);
            let val = if !valid(m, q, val_slot) {
                if m.word(val_slot) != 0 {
                    leaf(m, 0x1757a8, &[m.word(ctx + 20), m.word(val_slot)]);
                }
                value
            } else {
                m.word(val_slot)
            };
            for (name, off) in [
                ("FS", 52),
                ("ALPHA", 4),
                ("BETA", 20),
                ("VUV_REL_AMP", 24),
                ("VOL", 48),
                ("MVF_BOOST", 28),
                ("FRAME_SIZE", 56),
                ("LOGLEVEL", 44),
                ("UV", 32),
                ("MVF_UV", 36),
                ("LT_STREAMS", 60),
                ("ORDER", 8),
            ] {
                if eq(m, key, name) {
                    m.put_word(cfg + off, integer(m, e, val) as u32);
                }
            }
            if eq(m, key, "BREAKERSTR") {
                m.put_byte(cfg + 0x41, m.byte(val));
            }
            if eq(m, key, "BREAKMAXDUR") {
                break_max = integer(m, e, val);
            }
            if eq(m, key, "SPLIT") {
                m.put_byte(cfg + 0x40, integer(m, e, val) as u8);
            }
            if eq(m, key, "RHO") {
                m.put_half(cfg, integer(m, e, val) as u16);
            }
            if eq(m, key, "DATATYPE") {
                m.put_byte(cfg + 0x42, m.byte(val));
            }
            if eq(m, key, "VSYNTH_INIT") {
                leaf(m, 0x195cd0, &[cfg + 0x5c, val]);
            }
            if val != value {
                leaf(m, 0x1757a8, &[m.word(ctx + 20), val]);
            }
        }
        override_key(m, e, pname, "BREAKMAXDUR");
        if query(m, h, m.word(ctx + 20), pname, val_slot) == 0 && m.word(val_slot) != 0 {
            let val = m.word(val_slot);
            if m.byte(val) != 0 {
                break_max = integer(m, e, val);
            }
            leaf(m, 0x1757a8, &[m.word(ctx + 20), val]);
        }
        break_max = break_max.max(160);
        let frame = m.word(cfg + 56) as i32;
        let nbreak = div(break_max, frame);
        m.put_word(cfg + 0x4c, nbreak as u32);
        let x = div(
            (if break_max >= 1000 { 40i32 } else { 32 }).wrapping_add(frame >> 1),
            frame,
        );
        let f50 = x
            .min(nbreak >> 2)
            .min(if break_max >= 1000 { 10 } else { 8 });
        m.put_word(cfg + 0x50, f50 as u32);
        m.put_word(
            cfg + 0x54,
            div(20i32.wrapping_add(frame >> 1), frame).min(f50) as u32,
        );
        m.put_word(
            cfg + 0x58,
            div(12i32.wrapping_add(frame >> 1), frame).min(f50) as u32,
        );
        override_key(m, e, pname, "ORDER_BEGIN_FAST");
        if query(m, h, m.word(ctx + 20), pname, val_slot) == 0 && m.word(val_slot) != 0 {
            let val = m.word(val_slot);
            if m.byte(val) != 0 {
                m.put_word(cfg + 12, integer(m, e, val) as u32);
            }
            leaf(m, 0x1757a8, &[m.word(ctx + 20), val]);
        }
        m.put_word(
            p + 0x73c,
            div((m.word(cfg + 56) as i32).wrapping_mul(frequency), 1000) as u32,
        );
        override_key(m, e, pname, "ORDER");
        let q = query(m, h, m.word(ctx + 20), pname, val_slot);
        if !valid(m, q, val_slot) && m.word(cfg + 52) == 16000 {
            m.put_word(cfg + 8, (m.word(cfg + 8) as i32).min(20) as u32);
        }
        override_key(m, e, pname, "BREAKERSTR");
        let q = query(m, h, m.word(ctx + 20), pname, val_slot);
        if !valid(m, q, val_slot) {
            m.put_byte(cfg + 0x41, b'2');
        }
        if pos != m.word(size) {
            h.call(
                m,
                0x173dc0,
                &[
                    m.word(m.word(p + 8) + 16),
                    e.module,
                    25012,
                    lit(e, "%s%s%s%s"),
                    lit(e, "message"),
                    lit(e, "Corrupt voice configuration chunk"),
                    lit(e, "file"),
                    path,
                ],
            );
            r = 0x84e02004;
        }
        m.put_word(val_slot, 0);
        for (name, off, default) in [
            ("PGEN_COMPLEX_LF0", 0x111, 3),
            ("PGEN_COMPLEX_MVF", 0x112, 2),
            ("PGEN_COMPLEX_CEP_ST", 0x114, 3),
            ("PGEN_COMPLEX_CEP_LT", 0x113, 3),
        ] {
            override_key(m, e, key, name);
            if query(m, h, m.word(ctx + 20), key, val_slot) == 0 && m.word(val_slot) != 0 {
                let val = m.word(val_slot);
                let level = if eq(m, val, "high") {
                    Some(3)
                } else if eq(m, val, "default") {
                    Some(default)
                } else if name == "PGEN_COMPLEX_MVF" && eq(m, val, "low") {
                    Some(1)
                } else if (name == "PGEN_COMPLEX_MVF" || name == "PGEN_COMPLEX_CEP_ST")
                    && eq(m, val, "medium")
                {
                    Some(2)
                } else {
                    None
                };
                if let Some(level) = level {
                    m.put_byte(cfg + off, level);
                } else {
                    h.call(
                        m,
                        0x173dc0,
                        &[
                            m.word(ctx + 16),
                            e.module,
                            25017,
                            lit(e, "%s%s%s%s"),
                            lit(e, "parameter"),
                            key,
                            lit(e, "value"),
                            val,
                        ],
                    );
                    if name == "PGEN_COMPLEX_CEP_ST" && eq(m, val, "low") {
                        m.put_byte(cfg + off, 2);
                    }
                }
                leaf(m, 0x1757a8, &[m.word(ctx + 20), val]);
            }
        }
        m.put_word(data, 0);
        let q = h.call(m, 0x2422ac, &[rd]);
        if !bad(r) && bad(q) {
            r = q;
        }
        if bad(r) {
            break 'run r;
        }
        loop {
            let r = h.call(m, 0x24277c, &[rd, tag, size, 0]);
            if bad(r) {
                break 'run r;
            }
            if eq(m, tag, "FEEX") {
                break;
            }
            let r = h.call(m, 0x2422ac, &[rd]);
            if bad(r) {
                break 'run r;
            }
        }
        r = h.call(
            m,
            0x2a0178,
            &[m.word(p), m.word(p + 4), m.word(p + 12), rd, path, p + 16],
        );
        stage = stage.wrapping_add(1);
        let q = h.call(m, 0x2422ac, &[rd]);
        if !bad(r) && bad(q) {
            r = q;
        }
        if bad(r) {
            break 'run r;
        }
        h.call(m, 0x2a4180, &[ctx, model]);
        loop {
            let r = h.call(m, 0x24277c, &[rd, tag, size, 0]);
            if bad(r) {
                break 'run r;
            }
            if eq(m, tag, "DWIN") {
                break;
            }
            let t = if eq(m, tag, "TREE") {
                Some(1)
            } else if eq(m, tag, "PDF ") {
                Some(0)
            } else if eq(m, tag, "CBK ") {
                Some(2)
            } else {
                None
            };
            let mut r = r;
            if let Some(t) = t {
                loop {
                    r = h.call(m, 0x24277c, &[rd, tag, size, 0]);
                    if bad(r) {
                        break;
                    }
                    let stream = ["MCP ", "LF0 ", "DUR ", "MVF ", "VCF "]
                        .iter()
                        .position(|n| eq(m, tag, n));
                    if let Some(stream) = stream {
                        let item = model + (t * 5 + stream as u32) * 32;
                        if t == 1 || (t == 0 && map_at_start != 0) {
                            let slot = p + 0x748 + half(m, p + 0x788) as u32 * 4;
                            r = h.call(m, 0x243ad4, &[rd, slot, data]);
                            if bad(r) {
                                break 'run r;
                            }
                            m.put_half(p + 0x788, half(m, p + 0x788).wrapping_add(1));
                            m.put_byte(item, 0);
                            m.put_word(item + 8, m.word(size));
                            m.put_word(item + 24, m.word(size));
                            m.put_word(item + 20, m.word(data));
                            m.put_word(data, 0);
                        } else {
                            m.put_byte(item, 2);
                            let off = h.call(m, 0x244318, &[rd]);
                            m.put_word(item + 4, off);
                            m.put_word(item + 8, m.word(size));
                            m.put_word(item + 28, p + 0x78c);
                        }
                    }
                    let q = h.call(m, 0x2422ac, &[rd]);
                    if bad(q) {
                        r = q;
                    }
                }
            }
            if code(r) {
                r = 0;
            }
            let _ = r;
            h.call(m, 0x2422ac, &[rd]);
        }
        r = h.call(m, 0x2a41e0, &[ctx, model, cfg]);
        if bad(r) {
            break 'run r;
        }
        r = h.call(m, 0x2a4620, &[ctx, model, cfg]);
        if bad(r) {
            break 'run r;
        }
        stage = stage.wrapping_add(1);
        release_frame(m, h, p + 0x78c);
        while !bad(r) {
            r = h.call(m, 0x24277c, &[rd, tag, size, 0]);
            if bad(r) {
                break;
            }
            leaf(m, 0x195bec, &[win, 0, 64]);
            let spec = [
                ("MCP ", 16, 0, 16),
                ("LF0 ", 20, 1, 18),
                ("MVF ", 18, 3, 18),
                ("VCF ", 16, 4, 16),
                ("DUR ", 0, 2, 0),
            ]
            .into_iter()
            .find(|x| eq(m, tag, x.0));
            if let Some((_, dim, stream, n)) = spec {
                while m.word(win + 20) == 0 || m.word(win + 52) == 0 {
                    r = h.call(m, 0x24277c, &[rd, tag, size, 0]);
                    if bad(r) {
                        break;
                    }
                    let w = if eq(m, tag, "DYN ") {
                        Some(0)
                    } else if eq(m, tag, "ACC ") {
                        Some(1)
                    } else {
                        None
                    };
                    if let Some(w) = w {
                        let slot = p + 0x748 + half(m, p + 0x788) as u32 * 4;
                        r = h.call(m, 0x243ad4, &[rd, slot, data]);
                        if bad(r) {
                            break 'run r;
                        }
                        m.put_half(p + 0x788, half(m, p + 0x788).wrapping_add(1));
                        let item = win + w * 32;
                        m.put_byte(item, 0);
                        m.put_word(item + 8, m.word(size));
                        m.put_word(item + 24, m.word(size));
                        m.put_word(item + 20, m.word(data));
                        m.put_word(wins + w * 4, item);
                        m.put_word(data, 0);
                    }
                    let q = h.call(m, 0x2422ac, &[rd]);
                    if bad(q) {
                        r = q;
                    }
                }
                if code(r) {
                    r = 0;
                }
                if !bad(r) && m.word(win + 20) != 0 && m.word(win + 52) != 0 {
                    // The ROM passes eight words: the window array and the
                    // reader name are the frame slots +0xf0 and +0xa0, and
                    // the last word repeats the dimension.
                    r = h.call(
                        m,
                        0x2a2f04,
                        &[
                            ctx,
                            p + 0x638 + stream * 0x34,
                            2,
                            dim,
                            n,
                            wins,
                            rd_slot,
                            dim,
                        ],
                    );
                }
            } else if code(r) {
                r = 0;
            }
            let q = h.call(m, 0x2422ac, &[rd]);
            if !bad(r) && bad(q) {
                r = q;
            }
        }
        if code(r) {
            r = 0;
        }
        stage = stage.wrapping_add(1);
        let q = h.call(m, 0x2422ac, &[rd]);
        if !bad(r) && bad(q) {
            r = q;
        }
        if bad(r) {
            break 'run r;
        }
        let excite = if m.byte(cfg + 0x46) != 0 {
            0
        } else if m.byte(cfg + 0x47) != 0 {
            2
        } else if m.byte(cfg + 0x48) != 0 {
            1
        } else {
            0
        };
        if m.word(cfg + 8) as i32 > half(m, model + 0x392) as i32 {
            h.call(
                m,
                0x1738e8,
                &[
                    m.word(m.word(p + 8) + 16),
                    e.module,
                    3,
                    0,
                    lit(
                        e,
                        "Specified MCEP order %d is higher than order of the data (%d)",
                    ),
                    m.word(cfg + 8),
                    half(m, model + 0x392) as u32,
                ],
            );
            m.put_word(cfg + 8, half(m, model + 0x38c) as u32);
        }
        for (key, kind, off) in [
            ("bet2voiceexcitetype", 0, excite),
            ("bet2voicealpha", 1, 4),
            ("bet2voicebeta", 1, 20),
            ("bet2voicevol", 1, 48),
            ("bet2voicevuvrelamp", 1, 24),
            ("bet2voicemvfboost", 1, 28),
            ("bet2voiceframesize", 0, m.word(cfg + 56)),
            ("bet2voiceorder", 0, m.word(cfg + 8)),
            ("bet2voicevsynthinit", 2, 0x5c),
        ] {
            r = if kind == 0 {
                h.call(m, 0x1757b8, &[m.word(ctx + 20), lit(e, key), off])
            } else {
                let v = if kind == 1 {
                    number(m, num, m.word(cfg + off) as i32);
                    num
                } else {
                    cfg + off
                };
                h.call(m, 0x1759a0, &[m.word(ctx + 20), lit(e, key), v])
            };
            if bad(r) {
                h.call(
                    m,
                    0x173dc0,
                    &[
                        m.word(m.word(p + 8) + 16),
                        e.module,
                        25009,
                        lit(e, "%s%s%s%x"),
                        lit(e, "parameter"),
                        lit(e, key),
                        lit(e, "lhError"),
                        r,
                    ],
                );
                break 'run r;
            }
        }
        m.put_word(model + 0x484, 0);
        m.put_word(model + 0x488, 0);
        if bad(h.call(m, 0x24277c, &[rd, tag, size, 0])) {
            r = 0;
        } else {
            if !eq(m, tag, "FVPG") {
                break 'run 0x84e02004;
            }
            r = h.call(
                m,
                0x243ad4,
                &[rd, p + 0x748 + half(m, p + 0x788) as u32 * 4, data],
            );
            if bad(r) {
                break 'run r;
            }
            let d = m.word(data);
            m.put_word(model + 0x484, m.word(d));
            m.put_word(model + 0x488, d + 4);
            m.put_half(p + 0x788, half(m, p + 0x788).wrapping_add(1));
            h.call(m, 0x2422ac, &[rd]);
        }
        leaf(m, 0x195cd0, &[p + 20, path]);
        r
    };
    if rd != 0 {
        h.call(m, 0x2421b0, &[rd]);
    }
    if !bad(rc) {
        return rc;
    }
    if ((rc >> 20) << 21) == 0x14400000 && code(rc) {
        rc = 0x84e02004;
    }
    if m.word(p + 0x790) != 0 {
        release_frame(m, h, p + 0x78c);
        h.call(m, 0x179f3c, &[m.word(ctx + 12), m.word(p + 0x790)]);
        m.put_word(p + 0x790, 0);
    }
    clear_blocks(m, h, p, ctx);
    if stage >= 3 {
        h.call(m, 0x2a41b8, &[model]);
    }
    // The unresolved weak configuration destructor emits no ROM call.
    if m.word(p + 16) != 0 {
        drop_voice(m, h, m.word(p + 16));
        m.put_word(p + 16, 0);
    }
    clear_handles(m, h, p);
    rc
}

/// Regional diagnostic literals and disjoint ABI-local storage for pool teardown.
pub struct DestructionEnvironment {
    pub pool_file: u32,
    pub pool_format: u32,
    pub free_pool_label: u32,
    /// Six words, restored on return. Must not overlap callback-owned state.
    pub scratch: u32,
}
struct DestructorCalls<'a, H> {
    host: &'a mut H,
    environment: &'a DestructionEnvironment,
}
struct ReleaseMethods<'a, H> {
    host: &'a mut H,
    base: u32,
    method: u32,
}
impl<H: Host> crate::frontend::HeapVtableM for ReleaseMethods<'_, H> {
    fn alloc_at_m(&mut self, _: u32, _: u32, _: &mut [u8]) -> u32 {
        unreachable!("destructor does not allocate")
    }
    fn realloc_at_m(&mut self, _: u32, _: u32, _: u32, _: &mut [u8]) -> u32 {
        unreachable!("destructor does not reallocate")
    }
    fn release_at_m(&mut self, context: u32, object: u32, bytes: &mut [u8]) {
        self.host.call(
            &mut Memory {
                base: self.base,
                bytes,
            },
            self.method,
            &[context, object],
        );
    }
}
/// The arena-aware release of one heap object: the ROM's release method is
/// dispatched through the host, so the ARM's release calls are compared.
pub fn release_object(m: &mut Memory<'_>, h: &mut impl Host, heap: u32, object: u32) {
    if heap == 0 || object == 0 {
        return;
    }
    let method = m.word(m.word(heap) + 12);
    let heap_offset = m.offset(heap) as u32;
    crate::frontend::heap_release_m(
        m.bytes,
        Some(heap_offset),
        Some(object),
        &mut ReleaseMethods {
            host: h,
            base: m.base,
            method,
        },
    );
}
impl<H: Host> crate::log_live::LogMethodsM for DestructorCalls<'_, H> {
    fn log_m(
        &mut self,
        method: u32,
        object: u32,
        category: u32,
        level: u32,
        arg: u32,
        format: u32,
        varargs: u32,
        m: &mut Memory<'_>,
    ) -> u32 {
        self.host
            .call(m, method, &[object, category, level, arg, format, varargs])
    }
}
impl<H: Host> crate::reentrant::HooksM for DestructorCalls<'_, H> {
    fn literal_m(&self, literal: crate::reentrant::LiteralM) -> u32 {
        use crate::reentrant::LiteralM::*;
        match literal {
            PoolFile => self.environment.pool_file,
            PoolReportFormat => self.environment.pool_format,
            FreePoolLabel => self.environment.free_pool_label,
            _ => panic!("unexpected destructor literal"),
        }
    }
    fn call_m(&mut self, f: u32, a: &[u32], m: &mut Memory<'_>) -> u32 {
        match f {
            0x196428 => {
                release_object(m, self.host, a[0], a[1]);
                0
            }
            0x1738e8 => {
                let s = self.environment.scratch;
                let saved: [u8; 24] = core::array::from_fn(|i| m.byte(s + i as u32));
                for (i, &value) in a[5..].iter().enumerate() {
                    m.put_word(s + i as u32 * 4, value);
                }
                let rc = crate::log_live::fn_1738e8(m, a[0], a[1], a[2], a[3], a[4], s, self);
                let at = m.offset(s);
                m.bytes[at..at + 24].copy_from_slice(&saved);
                rc
            }
            _ => panic!("unexpected source-backed pool teardown call"),
        }
    }
}
/// fn_2d9bb0. Each slot is read after the preceding release callback.
pub fn fn_2d9bb0(m: &mut Memory<'_>, h: &mut impl Host, p: u32) -> u32 {
    if p == 0 {
        return 0x84e02006;
    }
    for i in 0..5 {
        let object = m.word(p + 36 + i * 4);
        if object != 0 {
            release_object(m, h, m.word(m.word(p) + 4), object);
        }
    }
    leaf(m, 0x195bec, &[p, 0, 0x44]);
    0
}
/// fn_2a9090. The frame release the live replay binds through the host.
pub fn fn_2a9090(m: &mut Memory<'_>, h: &mut impl Host, p: u32) -> u32 {
    crate::selection::arena_mid::release_frame_lookup(
        &mut crate::selection::arena::Arena::new(m.base, m.bytes),
        &mut Sel(h),
        p,
    )
}

/// fn_2db4a0. The pool release the live replay binds through the host.
pub fn fn_2db4a0(
    m: &mut Memory<'_>,
    h: &mut impl Host,
    p: u32,
    k: &DestructionEnvironment,
) -> u32 {
    crate::reentrant::free_pool_m(
        m,
        &mut DestructorCalls {
            host: h,
            environment: k,
        },
        p,
    );
    0
}

/// fn_2a41b8. fn_2da9d4 is a tail wrapper around the certified free_pool_m.
/// Pool and tree teardown execute real Rust bodies, including heap/log dispatch.
pub fn fn_2a41b8(m: &mut Memory<'_>, h: &mut impl Host, p: u32, k: &DestructionEnvironment) {
    leaf(m, 0x195bec, &[p, 0, 0x1e0]);
    // The pool release stays a direct call: the round-two destructor gate
    // requires the pool teardown to be real, not a host call.  The ARM does
    // not call fn_2db4a0 in the live scene.
    crate::reentrant::free_pool_m(
        m,
        &mut DestructorCalls {
            host: h,
            environment: k,
        },
        p + 0x1e0,
    );
    fn_2d9bb0(m, h, p + 0x440);
}

// ----------------------------------------------------------------------
// The arena-aware resource open.
//
// leftovers_1::fn_242fd0 is the certified port of the ROM's resource
// open, but its host traits (frontend::HeapVtable, frontend::MethodTable
// and frontend::BlockHeap) carry no arena while the engine heap they
// reach lives in the arena, so the shared port cannot run against live
// memory. The body below is that port's body with the four heap call
// sites routed through the arena-aware stage ports: leftovers_1's
// stage_allocate / stage_calloc / stage_release reach the ROM's own
// allocator methods (fn_4a779c / fn_4a7758 / fn_4a774c) through the
// host's call boundary, with the arena in hand.
// ----------------------------------------------------------------------

/// The host the arena-aware resource open needs. The heap and method
/// tables arrive through `render_part_c::Host::call`, which carries the
/// arena; the arena-less traits the shared ports use stay unimplemented.
pub trait OpenHostM:
    crate::leftovers_1::CloseHost
    + crate::render_part_a::ReaderHost
    + crate::boot_part_b::DataHost
    + crate::log_live::LogMethodsM
    + crate::render_part_c::Host
{
    /// fn_196394: `size` bytes from the heap at `heap`.
    fn alloc_block_m(&mut self, m: &mut Memory<'_>, heap: u32, size: u32) -> u32;
    /// fn_1963e0: regrow `data` to `size` bytes in the heap at `heap`.
    fn realloc_block_m(&mut self, m: &mut Memory<'_>, heap: u32, data: u32, size: u32) -> u32;
    /// fn_1963b8: `count` cells of `size` bytes from the heap at `heap`.
    fn alloc_cells_m(&mut self, m: &mut Memory<'_>, heap: u32, count: u32, size: u32) -> u32;
}

/// The close-path adapter of `leftovers_1`, with the arena-aware heap
/// ports in place of the arena-less ones.
struct CallsM<'a, H>(&'a mut H);
impl<H: OpenHostM> crate::boot_part_b::Host for CallsM<'_, H> {
    fn call(&mut self, m: &mut Memory<'_>, f: u32, a: &[u32]) -> u32 {
        if f == 0x196428 {
            crate::leftovers_1::stage_release(m, a[0], a[1], self.0);
            0
        } else {
            self.0.indirect(m, f, a)
        }
    }
    fn thread_id(&mut self) -> u32 {
        panic!("archive close does not request thread id")
    }
}
impl<H: OpenHostM> crate::boot_part_e::Host for CallsM<'_, H> {
    fn call(&mut self, m: &mut Memory<'_>, entry: u32, a: &[u32]) -> u32 {
        match entry {
            0x196394 => crate::leftovers_1::stage_allocate(m, a[0], a[1], self.0),
            0x1963b8 => crate::leftovers_1::stage_calloc(m, a[0], a[1], a[2], self.0),
            0x1963e0 => crate::leftovers_1::stage_resize(m, a[0], a[1], a[2], self.0),
            0x196428 => {
                crate::leftovers_1::stage_release(m, a[0], a[1], self.0);
                0
            }
            _ => self.0.indirect(m, entry, a),
        }
    }
}
impl<H: OpenHostM> crate::boot_part_d::Host for CallsM<'_, H> {
    fn call_m(&mut self, f: u32, a: &[u32], m: &mut Memory<'_>) -> u32 {
        self.0.indirect(m, f, a)
    }
    fn params_call_m(
        &mut self,
        _: u32,
        _: &mut [u32; 4],
        _: u32,
        _: &mut u32,
        _: u32,
        _: &mut Memory<'_>,
    ) -> u32 {
        panic!("close does not open archive")
    }
}

/// fn_196a64 / fn_196cfc. The block creator of the shared
/// `frontend::create_block`, with the arena-less cell allocation routed
/// through the arena-aware calloc.
fn create_block_m(
    m: &mut Memory<'_>,
    h: &mut impl OpenHostM,
    a: u32,
    b: u32,
    c: u32,
    d: u32,
    e: u32,
    out: Option<&mut u32>,
) -> u32 {
    if c != 0 {
        if d == 0 {
            return 0;
        }
    } else if d != 0 {
        return 0;
    }
    let Some(slot) = out else {
        return 0;
    };
    let at = crate::leftovers_1::stage_calloc(m, a, 1, 32, h);
    *slot = at;
    if at == 0 {
        return 0;
    }
    let at = at as usize;
    m.bytes[at..at + 4].copy_from_slice(&0u32.to_le_bytes());
    m.bytes[at + 4..at + 8].copy_from_slice(&b.to_le_bytes());
    m.bytes[at + 8..at + 12].copy_from_slice(&c.to_le_bytes());
    m.bytes[at + 12..at + 16].copy_from_slice(&d.to_le_bytes());
    m.bytes[at + 16..at + 20].copy_from_slice(&0u32.to_le_bytes());
    m.bytes[at + 20..at + 24].copy_from_slice(&0u32.to_le_bytes());
    m.bytes[at + 24..at + 26].copy_from_slice(&(e as u16).to_le_bytes());
    m.bytes[at + 28..at + 32].copy_from_slice(&a.to_le_bytes());
    1
}

fn literal_m(m: &mut Memory<'_>, p: u32, b: &[u8]) {
    for (i, &v) in b.iter().chain(core::iter::once(&0)).enumerate() {
        m.put_byte(p + i as u32, v);
    }
}
fn equals_m(m: &Memory<'_>, p: u32, b: &[u8]) -> bool {
    b.iter()
        .enumerate()
        .all(|(i, v)| m.byte(p + i as u32) == *v)
        && m.byte(p + b.len() as u32) == 0
}
fn open_log_m(
    m: &mut Memory<'_>,
    h: &mut impl OpenHostM,
    env: u32,
    k: &OpenRuntime,
    fmt: u32,
    args: &[u32],
) {
    for (i, &v) in args.iter().enumerate() {
        m.put_word(k.scratch + 0x180 + i as u32 * 4, v);
    }
    crate::log_live::fn_1738e8(
        m,
        m.word(env + 16),
        k.reader.category,
        4,
        0,
        fmt,
        k.scratch + 0x180,
        h,
    );
}
fn open_error_m(
    m: &mut Memory<'_>,
    h: &mut impl OpenHostM,
    env: u32,
    k: &OpenRuntime,
    code: u32,
    fmt: u32,
    args: &[u32],
) {
    for (i, &v) in args.iter().enumerate() {
        m.put_word(k.scratch + 0x180 + i as u32 * 4, v);
    }
    crate::render_part_a::error_log(
        m,
        h,
        m.word(env + 16),
        k.reader.category,
        code,
        fmt,
        k.scratch + 0x180,
        k.scratch + 0x1c0,
    );
}
/// fn_242fd0, arena-aware. Open a RIFF resource, read its type and
/// optional INFO/VERS, compare the packed version and leave the reader at
/// its first data chunk. The body is leftovers_1::fn_242fd0's; only the
/// heap call sites differ.
pub fn fn_242fd0_m(
    m: &mut Memory<'_>,
    args: [u32; 9],
    k: &OpenRuntime,
    h: &mut impl OpenHostM,
) -> u32 {
    assert_eq!(m.base, 0);
    let mut saved = [0u8; 1024];
    saved.copy_from_slice(&m.bytes[k.scratch as usize..k.scratch as usize + 1024]);
    let rc = open_inner_m(m, args, k, h);
    m.bytes[k.scratch as usize..k.scratch as usize + 1024].copy_from_slice(&saved);
    rc
}
fn close_open_record_m(m: &mut Memory<'_>, rec: u32, k: &OpenRuntime, h: &mut impl OpenHostM) -> u32 {
    // The older normalization helper omits the broker-backed branches.
    // Execute both broker and mapped-file branches through the live ports.
    if m.byte(rec + 4) != 0 {
        return crate::normalization::close_owner_pair(m.bytes, rec, h);
    }
    let broker = m.word(m.word(rec) + 12);
    let handle = m.word(rec + 8);
    if handle != 0 && m.word(rec + 16) != 0 {
        let rc = crate::boot_part_e::fn_179208(
            m,
            broker,
            handle,
            m.word(rec + 16),
            &k.unmap,
            &mut CallsM(h),
        );
        for off in [16, 20, 24] {
            m.put_word(rec + off, 0);
        }
        if rc >= 0x80000000 {
            return rc;
        }
    }
    if handle == 0 && m.word(rec + 12) != 0 && m.word(rec + 16) != 0 {
        let rc =
            crate::boot_part_d::fn_17331c(m, m.word(rec + 12), m.word(rec + 16), &mut CallsM(h));
        for off in [16, 20, 24] {
            m.put_word(rec + off, 0);
        }
        if rc >= 0x80000000 {
            return rc;
        }
    }
    let handle = m.word(rec + 8);
    if handle != 0 {
        let rc = crate::leftovers_1::fn_179f3c(m, m.word(m.word(rec) + 12), handle, &k.close, h);
        m.put_word(rec + 8, 0);
        if rc >= 0x80000000 {
            return rc;
        }
    }
    if m.word(rec + 12) != 0 {
        let rc = crate::boot_part_d::fn_1730fc(m, m.word(rec + 12), &mut CallsM(h));
        m.put_word(rec + 12, 0);
        return rc;
    }
    crate::normalization::close_owner_pair(m.bytes, rec, h)
}
fn close_open_reader_m(m: &mut Memory<'_>, p: u32, k: &OpenRuntime, h: &mut impl OpenHostM) {
    let env = m.word(p + 8);
    open_log_m(m, h, env, k, k.reader.messages[2], &[p]);
    let rc = close_open_record_m(m, p + 24, k, h);
    let name = m.word(p + 16);
    if name != 0 {
        crate::leftovers_1::stage_release(m, m.word(m.word(p + 8) + 4), name, h);
    }
    if m.word(p + 20) != 0 {
        reader_release_array_m(m, h, m.word(p + 20));
    }
    if rc < 0x80000000 {
        crate::leftovers_1::stage_release(m, m.word(m.word(p + 8) + 4), p, h);
    }
    open_log_m(m, h, env, k, k.reader.messages[3], &[rc]);
}
fn open_inner_m(m: &mut Memory<'_>, a: [u32; 9], k: &OpenRuntime, h: &mut impl OpenHostM) -> u32 {
    use crate::{normalization as n, render_part_a as r};
    let [owner, tag, mode, filename, mapped, mapped_len, kind, version, out] = a;
    let (typ, quiet) = match mode {
        0 => (0, 0),
        1 => (1, 0),
        2 => (0, 1),
        3 => (1, 1),
        4 => (4, 0),
        _ => return 0x8a202007,
    };
    if filename == 0 || m.byte(filename) == 0 || kind == 0 {
        return 0x8a202007;
    }
    if typ == 0 && m.byte(kind) == 0 || typ == 4 && (mapped == 0 || mapped_len == 0) || out == 0 {
        return 0x8a202007;
    }
    m.put_word(out, 0);
    let cached = [
        u32::from_le_bytes(k.context.cached_handle[..4].try_into().unwrap()),
        u32::from_le_bytes(k.context.cached_handle[4..].try_into().unwrap()),
    ];
    let pointed = if [owner, tag] != cached && owner != 0 && tag == 0x1e61 {
        Some(u16::from_le_bytes([m.byte(owner), m.byte(owner + 1)]))
    } else {
        None
    };
    let mut env = 0;
    let mut rc = f::resolve_engine_context(
        owner,
        pointed,
        tag,
        &k.context.cached_handle,
        k.context.default_context,
        &mut env,
    );
    if rc >= 0x80000000 {
        return rc;
    }
    let major = version & 255;
    let minor = version >> 8 & 255;
    let patch = version >> 16 & 255;
    open_log_m(
        m,
        h,
        env,
        k,
        k.begin,
        &[mode, filename, kind, major, minor, patch],
    );
    let s = k.scratch;
    let rec = s;
    f::fill_bytes(m.bytes, rec, 0, 28);
    m.put_word(rec, env);
    let heap = m.word(env + 4);
    match typ {
        0 => {
            rc = crate::boot_part_b::fn_1795f0(
                m,
                m.word(env + 12),
                filename,
                quiet,
                rec + 8,
                &k.broker,
                h,
            );
            if rc >= 0x80000000 {
                return rc;
            }
        }
        1 => {
            rc = crate::boot_part_d::fn_173000(
                m,
                m.word(env + 28),
                heap,
                filename,
                rec + 12,
                k.rb,
                &mut CallsM(h),
            );
            if rc >= 0x80000000 {
                if quiet == 1 {
                    open_log_m(m, h, env, k, k.begin, &[filename, rc]);
                } else {
                    open_error_m(
                        m,
                        h,
                        env,
                        k,
                        0x697a,
                        k.error_format,
                        &[k.file_key, filename, k.mode_key, rc],
                    );
                }
                return rc;
            }
        }
        4 => {
            m.put_word(rec + 16, mapped);
            m.put_word(rec + 24, mapped_len);
        }
        _ => unreachable!(),
    }
    let p = crate::leftovers_1::stage_calloc(m, heap, 1, 56, h);
    if p == 0 {
        open_error_m(m, h, env, k, 0x6978, 0, &[]);
        close_open_record_m(m, rec, k, h);
        return 0x8a20200a;
    }
    m.put_word(p, owner);
    m.put_word(p + 4, tag);
    m.put_word(p + 8, env);
    m.put_byte(p + 12, typ as u8);
    for off in (0..28).step_by(4) {
        m.put_word(p + 24 + off, m.word(rec + off));
    }
    let name = crate::leftovers_1::stage_allocate(
        m,
        heap,
        f::strlen_bytes(&m.bytes[filename as usize..]) + 1,
        h,
    );
    m.put_word(p + 16, name);
    let id = s + 32;
    let len = s + 48;
    let data = s + 52;
    let chunk = s + 56;
    let buf = s + 64;
    let cap = s + 80;
    let tmp = s + 96;
    let num = s + 128;
    let end = s + 164;
    literal_m(m, buf, b"");
    m.put_word(cap, 16);
    let mut success = false;
    'operation: {
        if name == 0 {
            open_error_m(m, h, env, k, 0x6978, 0, &[]);
            rc = 0x8a20200a;
            break 'operation;
        }
        f::copy_str(m.bytes, name, filename);
        let mut array = 0;
        let ok = create_block_m(
            m,
            h,
            heap,
            k.array_methods[0],
            k.array_methods[1],
            k.array_methods[2],
            4,
            Some(&mut array),
        );
        m.put_word(p + 20, array);
        if ok == 0 {
            open_error_m(m, h, env, k, 0x6978, 0, &[]);
            rc = 0x8a20200a;
            break 'operation;
        }
        rc = rp_open_chunk(m, h, p, chunk, len, 0, &k.reader);
        if rc >= 0x80000000 {
            break 'operation;
        }
        rc = r::reader_chunk_data(m, h, p, 4, data, &k.reader);
        if rc >= 0x80000000 {
            break 'operation;
        }
        let be = k.big_endian;
        let alignment = if equals_m(m, chunk, if be { b"IFF " } else { b"RIFF" })
            || be && equals_m(m, chunk, b"RIFX")
        {
            2
        } else if equals_m(m, chunk, if be { b"IFF4" } else { b"RIF4" }) {
            4
        } else if equals_m(m, chunk, if be { b"IFF8" } else { b"RIF8" }) {
            8
        } else {
            u16::from_le_bytes([m.byte(p + 14), m.byte(p + 15)])
        };
        if alignment != 0 {
            m.put_half(p + 14, alignment);
        }
        if alignment == 0 || m.word(data) == 0 || m.word(len) < 4 {
            open_error_m(
                m,
                h,
                env,
                k,
                0x697c,
                k.error_format,
                &[k.file_key, m.word(p + 16)],
            );
            rc = 0x8a202004;
            break 'operation;
        }
        let src = m.word(data);
        crate::selection::arena_leaf::copy_bytes(
            &mut crate::selection::arena::Arena::new(0, m.bytes),
            id,
            src,
            4,
        );
        m.put_byte(id + 4, 0);
        if m.byte(p + 13) == 1 {
            for i in 0..4 {
                m.put_byte(id + i, m.byte(id + i) ^ 0xa9);
            }
        }
        if m.byte(kind) != 0 {
            if f::compare_str(&m.bytes[kind as usize..], &m.bytes[id as usize..]) != 0 {
                open_error_m(
                    m,
                    h,
                    env,
                    k,
                    0x697d,
                    k.error_format,
                    &[k.file_key, m.word(p + 16)],
                );
                rc = 0x8a202004;
                break 'operation;
            }
        } else {
            f::copy_str(m.bytes, kind, id);
        }
        m.put_word(p + 52, m.word(p + 52).wrapping_add(4));
        if typ != 0 && major == 0 {
            m.put_word(out, p);
            success = true;
            break 'operation;
        }
        rc = rp_open_chunk(m, h, p, chunk, len, data, &k.reader);
        if rc >= 0x80000000 || !equals_m(m, chunk, b"LIST") || m.word(len) <= 4 || m.word(data) == 0 {
            open_error_m(
                m,
                h,
                env,
                k,
                0x697e,
                k.error_format,
                &[k.file_key, m.word(p + 16)],
            );
            rc = 0x8a202004;
            break 'operation;
        }
        let src = m.word(data);
        crate::selection::arena_leaf::copy_bytes(
            &mut crate::selection::arena::Arena::new(0, m.bytes),
            id,
            src,
            4,
        );
        m.put_byte(id + 4, 0);
        if m.byte(p + 13) == 1 {
            for i in 0..4 {
                m.put_byte(id + i, m.byte(id + i) ^ 0xa9);
            }
        }
        if !equals_m(m, id, b"INFO") || (m.word(p + 52).wrapping_add(4) as i32) < 0 {
            open_error_m(
                m,
                h,
                env,
                k,
                0x697e,
                k.error_format,
                &[k.file_key, m.word(p + 16)],
            );
            rc = 0x8a202004;
            break 'operation;
        }
        m.put_word(p + 52, m.word(p + 52).wrapping_add(4));
        literal_m(m, id, b"VERS");
        rc = find_reader_chunk_m(m, h, p, id, 3, len, data, &k.reader);
        if rc < 0x80000000 {
            let src = m.word(data);
            let length = m.word(len);
            rc = r::read_chunk_string(
                &mut crate::selection::arena::Arena::new(0, m.bytes),
                p,
                src,
                length,
                0,
                buf,
                cap,
            );
        }
        if rc >= 0x80000000 {
            open_error_m(
                m,
                h,
                env,
                k,
                0x697e,
                k.error_format,
                &[k.file_key, m.word(p + 16)],
            );
            rc = 0x8a202004;
            break 'operation;
        }
        let fm = n::strtoul_scan(m.bytes, buf, end, 10, 0) as i32;
        let ep = m.word(end);
        let mut valid = fm >= 1 && ep != 0 && m.byte(ep) == b'.' && m.byte(ep + 1) != 0;
        let mut fn_ = 0i32;
        let mut fp = 0i32;
        if valid {
            fn_ = n::strtoul_scan(m.bytes, ep + 1, end, 10, 0) as i32;
            let ep = m.word(end);
            valid = fn_ >= 0 && ep != 0 && m.byte(ep) == b'.' && m.byte(ep + 1) != 0;
            if valid {
                fp = n::strtoul_scan(m.bytes, ep + 1, end, 10, 0) as i32;
                let ep = m.word(end);
                valid = fp >= 0 && ep != 0 && (m.byte(ep) == 0 || m.byte(ep) == b'.');
            }
        }
        if !valid {
            open_error_m(
                m,
                h,
                env,
                k,
                0x6980,
                k.error_format,
                &[k.version_key, m.word(p + 16)],
            );
            rc = 0x8a202004;
            break 'operation;
        }
        if major < fm as u32
            || major == fm as u32
                && (minor < fn_ as u32 || minor == fn_ as u32 && patch < fp as u32)
        {
            n::itoa(m.bytes, major, num, 10);
            f::copy_str(m.bytes, tmp, num);
            for v in [minor, patch] {
                literal_m(m, id, b".");
                f::append_str(m.bytes, tmp, id);
                n::itoa(m.bytes, v, num, 10);
                f::append_str(m.bytes, tmp, num);
            }
            open_error_m(
                m,
                h,
                env,
                k,
                0x6981,
                k.version_format,
                &[k.product_key, tmp, k.version_key, m.word(p + 16)],
            );
            rc = 0x8a202004;
            break 'operation;
        }
        rc = r::close_reader_chunk(m, h, p, &k.reader);
        if rc >= 0x80000000 {
            break 'operation;
        }
        rc = r::close_reader_chunk(m, h, p, &k.reader);
        if rc >= 0x80000000 {
            break 'operation;
        }
        m.put_word(out, p);
        success = true;
    }
    if !success {
        close_open_reader_m(m, p, k, h);
    }
    open_log_m(m, h, env, k, k.end, &[rc, m.word(out)]);
    rc
}

/// The reader's trace line, arena-aware.
fn rp_info(m:&mut crate::boot::Memory<'_>,r:u32,k:&crate::render_part_a::ReaderRuntime,level:u32,slot:usize){
    let logger=m.word(m.word(r+8)+16);
    crate::normalization::log_fanout(m.bytes,logger,k.category,level,0,k.messages[slot]);
}
/// The reader's error line, arena-aware.
fn rp_error(m:&mut crate::boot::Memory<'_>,h:&mut impl OpenHostM,r:u32,k:&crate::render_part_a::ReaderRuntime,code:u32,format:u32,args:&[u32]){
    for (i,&v) in args.iter().enumerate(){m.put_word(k.scratch+0x100+i as u32*4,v);}
    crate::render_part_a::error_log(m,h,m.word(m.word(r+8)+16),k.category,code,format,k.scratch+0x100,k.scratch);
}
/// The reader's descriptor append (fn_196db0 / fn_196cfc), arena-aware.
/// The block growth reaches the engine heap through the host, with the
/// arena in hand.
fn rp_append(m:&mut crate::boot::Memory<'_>,h:&mut impl OpenHostM,array:u32,source:u32)->u32 {
    let mut bytes=[0u8;16];bytes.copy_from_slice(&m.bytes[source as usize..source as usize+16]);
    let mut block=crate::frontend::GrowBlock{
        data:m.word(array),elem_size:u16::from_le_bytes(m.bytes[array as usize+4..array as usize+6].try_into().unwrap()),
        callback:m.word(array+8),cap:m.word(array+16),count:m.word(array+20),
        grow:u16::from_le_bytes(m.bytes[array as usize+24..array as usize+26].try_into().unwrap()),heap:m.word(array+28),
    };
    let callback=block.callback;
    let mut slot=0u32;
    if rp_slot(Some(&mut block),m,h,Some(&mut slot))==0 {return 0;}
    let size=block.elem_size as usize;
    let at=slot as usize;
    crate::selection::arena_leaf::copy_bytes(
        &mut crate::selection::arena::Arena::new(0,m.bytes),
        slot,source,block.elem_size as u32,
    );
    m.put_word(array,block.data);m.put_word(array+16,block.cap);m.put_word(array+20,block.count);
    if callback!=0 {
        let destination=block.data.wrapping_add(block.count.wrapping_sub(1).wrapping_mul(block.elem_size as u32));
        // The record is published. Run the caller's initializer with live
        // memory, then apply its veto exactly as fn_196db0 does.
        if h.array_initializer(m.bytes,callback,source,block.heap,destination)==0 {
            m.put_word(array+20,m.word(array+20).wrapping_sub(1));return 0;
        }
    }
    1
}
/// fn_196cfc, arena-aware: take the next slot, growing the block first.
fn rp_slot(
    block: Option<&mut crate::frontend::GrowBlock>,
    m: &mut crate::boot::Memory<'_>,
    h: &mut impl OpenHostM,
    out: Option<&mut u32>,
) -> u32 {
    let (Some(block),Some(out))=(block,out) else {return 0;};
    if block.cap<block.count.wrapping_add(1) {
        let quantum=block.grow as u32;
        if quantum==0 {return 0;}
        let slots=(block.count/quantum).wrapping_add(1).wrapping_mul(quantum);
        let bytes=(block.elem_size as u32).wrapping_mul(slots);
        if block.data!=0 {
            let grown=h.realloc_block_m(m,block.heap,block.data,bytes);
            if grown!=0 {block.data=grown;} else {return 0;}
        } else {
            let fresh=h.alloc_block_m(m,block.heap,bytes);
            block.data=fresh;
            if fresh==0 {return 0;}
        }
        block.cap=slots;
    }
    *out=block.count.wrapping_mul(block.elem_size as u32).wrapping_add(block.data);
    block.count=block.count.wrapping_add(1);
    1
}
/// fn_24277c. Read a chunk's name and length, validate its containment, and
/// push its descriptor. Invalid names clear only name[0], not the whole buffer.
pub fn rp_open_chunk(m:&mut crate::boot::Memory<'_>,h:&mut impl OpenHostM,r:u32,name:u32,out_len:u32,out_data:u32,k:&crate::render_part_a::ReaderRuntime)->u32 {
    assert_eq!(m.base,0);if r==0||out_len==0{return 0x8a202007;}
    rp_info(m,r,k,4,8);let slot=k.scratch+0x80;m.put_word(slot,0);
    let array=m.word(r+20);let last=crate::normalization::chunk_last(m.bytes,array,slot);let parent=m.word(slot);
    let mut rc=0;
    if last!=0 {
        if m.word(r+52).wrapping_add(8)>m.word(parent+12){rp_info(m,r,k,3,9);rc=0x8a202014;}
        else if m.word(r+40)==0||m.word(r+44)>m.word(r+52)||m.word(r+52).wrapping_add(8)>m.word(r+48){
            rc=crate::render_part_a::map_reader_window(m,h,r,m.word(r+52),8,r+24,k);
        }
    } else {
        rc=crate::render_part_a::map_reader_window(m,h,r,0,8,r+24,k);m.put_word(r+52,0);
        if rc<0x80000000 {
            let c=m.byte(m.word(r+40));if matches!(c,0xe0|0xfb){let at=m.offset(r+13);m.bytes[at]=1;}
        }
    }
    if rc<0x80000000 {
        let off=m.word(r+52).wrapping_sub(m.word(r+44));let source=m.word(r+40);
        if source!=0&&off.wrapping_add(4)<=m.word(r+48)&&name!=0 {
            let mut a=crate::selection::arena::Arena::new(0,m.bytes);crate::selection::arena_leaf::copy_bytes(&mut a,name,source.wrapping_add(off),4);a.set_u8(name+4,0);
            if a.u8(r+13)==1 {for i in 0..4 {a.set_u8(name+i,a.u8(name+i)^0xa9);}}
        }
        let mut a=crate::selection::arena::Arena::new(0,m.bytes);crate::selection::arena_leaf::copy_bytes(&mut a,out_len,source.wrapping_add(off).wrapping_add(4),4);
        a.set_u32(r+52,a.u32(r+52).wrapping_add(8));
        for i in 0..4 {
            let c=a.u8(name+i);
            if c.is_ascii_alphanumeric()||c==b' ' {continue;}
            if parent!=0 {
                rp_error(m,h,r,k,0x6983,k.triple_format,&[k.file_key,m.word(r+16),k.parent_key,parent,k.offset_key,m.word(r+52).wrapping_sub(8)]);
            } else {rp_error(m,h,r,k,0x697c,k.pair_format,&[k.file_key,m.word(r+16)]);}
            rc=0x8a202004;break;
        }
        if rc<0x80000000&&parent!=0 {
            let len=m.word(out_len);let end=m.word(parent+12);
            if len>end||(m.word(r+52).wrapping_add(len)as i32)>(end as i32){
                rp_error(m,h,r,k,0x6984,k.four_format,&[k.file_key,m.word(r+16),k.chunk_key,name]);rc=0x8a202004;
            }
        }
        if rc<0x80000000&&out_data!=0 {
            let pos=m.word(r+52);let len=m.word(out_len);
            if m.word(r+40)==0||m.word(r+44)>pos||pos.wrapping_add(len)>m.word(r+48) {
                rc=crate::render_part_a::map_reader_window(m,h,r,pos,len,r+24,k);
            }
        }
        if rc<0x80000000 {
            let element=k.scratch+0x40;for i in 0..16 {m.bytes[(element+i)as usize]=0;}
            let mut a=crate::selection::arena::Arena::new(0,m.bytes);
            // ROM sp+4 held the Begin-log reader argument before strcpy.
            // Its upper bytes survive in the descriptor name padding.
            a.set_u32(element+4,r);crate::selection::arena_leaf::copy_string(&mut a,element,name);
            a.set_u32(element+8,a.u32(r+52));a.set_u32(element+12,a.u32(r+52).wrapping_add(a.u32(out_len)));
            if rp_append(m,h,m.word(r+20),element)!=0 {
                if out_data!=0 {m.put_word(out_data,m.word(r+40).wrapping_add(m.word(r+52).wrapping_sub(m.word(r+44))));}
            } else {rp_error(m,h,r,k,0x6978,0,&[]);rc=0x8a20200a;}
        }
    }
    if rc>=0x80000000 {m.bytes[name as usize]=0;m.put_word(out_len,0);if out_data!=0 {m.put_word(out_data,0);}}
    rp_info(m,r,k,4,10);rc
}

/// fn_2a0178, arena-aware: open the voice object. The selection stage ports
/// work on the arena, so the borrow of the arena stays inside this file.
pub fn open_voice_object_m(
    m: &mut Memory<'_>,
    h: &mut impl Host,
    handle_p: u32,
    handle_tag: u32,
    param: u32,
    obj: u32,
    name: u32,
    out: u32,
    scratch: u32,
) -> u32 {
    let mut arena = crate::selection::arena::Arena::new(m.base, m.bytes);
    crate::selection::stage::open_voice_object(
        &mut arena,
        &mut Sel(h),
        handle_p,
        handle_tag,
        param,
        obj,
        name,
        out,
        scratch,
    )
}

/// fn_196c7c, arena-aware: the reader's chunk-table release.
///
/// The shared `render_part_a::reader_release_array` frees the table through
/// the arena-less `frontend::HeapVtable`, whose release hook cannot reach the
/// engine heap that lives in the arena. `reentrant2::destroy_array_m` reaches
/// the same heap release through `fn_196428`, so the block goes back to the
/// engine heap the ARM freed it from.
pub fn reader_release_array_m(m: &mut Memory<'_>, h: &mut impl OpenHostM, array: u32) -> u32 {
    struct Hooks<'a, H>(&'a mut H);
    impl<H: OpenHostM> crate::reentrant2::HooksM for Hooks<'_, H> {
        fn call_m(&mut self, f: u32, a: &[u32], m: &mut Memory<'_>) -> u32 {
            if f == 0x196428 {
                crate::leftovers_1::stage_release(m, a[0], a[1], self.0);
                0
            } else {
                self.0.array_destructor_m(m.bytes, f, a[0], a[1]);
                0
            }
        }
    }
    crate::reentrant2::destroy_array_m(m, array, &mut Hooks(h))
}

/// The clone hooks of fn_196b00 with the arena in hand: the copied table and
/// its buffer are allocated and released through the engine heap.
struct RpCloneHooks<'a, H>(&'a mut H);
impl<H: OpenHostM> crate::reentrant2::HooksM for RpCloneHooks<'_, H> {
    fn call_m(&mut self, f: u32, a: &[u32], m: &mut Memory<'_>) -> u32 {
        match f {
            0x1963b8 => {
                struct Methods<'a, H>(&'a mut H);
                impl<H: OpenHostM> crate::reentrant2::HooksM for Methods<'_, H> {
                    fn call_m(&mut self, f: u32, a: &[u32], m: &mut Memory<'_>) -> u32 {
                        crate::render_part_c::Host::call(self.0, m, f, a)
                    }
                }
                crate::reentrant2::call_method_m(m, a[0], a[1], a[2], &mut Methods(self.0))
            }
            0x196394 => crate::leftovers_1::stage_allocate(m, a[0], a[1], self.0),
            0x196428 => {
                crate::leftovers_1::stage_release(m, a[0], a[1], self.0);
                0
            }
            _ if a.len() == 3 => self.0.array_initializer(m.bytes, f, a[0], a[1], a[2]),
            _ => {
                self.0.array_destructor_m(m.bytes, f, a[0], a[1]);
                0
            }
        }
    }
}

/// fn_242c9c, arena-aware: find a named chunk, restoring a cloned descriptor
/// table and the saved reader position on any unsuccessful search.
///
/// The body is `render_part_a::find_reader_chunk`'s; the clone, the chunk
/// open and the table release reach the engine heap through the arena.
pub fn find_reader_chunk_m(
    m: &mut Memory<'_>,
    h: &mut impl OpenHostM,
    r: u32,
    name: u32,
    mode: u32,
    out_len: u32,
    out_data: u32,
    k: &crate::render_part_a::ReaderRuntime,
) -> u32 {
    assert_eq!(m.base, 0);
    if r == 0 || out_len == 0 {
        return 0x8a202007;
    }
    let count = crate::normalization::idx5(m.bytes, m.word(r + 20));
    if count == 0 {
        return 0x8a202007;
    }
    rp_info(m, r, k, 4, 11);
    let slot = k.scratch + 0x84;
    let source = m.word(r + 20);
    m.put_word(slot, 0);
    if crate::reentrant2::clone_table_m(m, source, slot, &mut RpCloneHooks(h)) == 0 {
        rp_error(m, h, r, k, 0x6978, 0, &[]);
        return 0x8a20200a;
    }
    let clone = m.word(slot);
    let saved = m.word(r + 52);
    let chunk_slot = k.scratch + 0x88;
    m.put_word(chunk_slot, 0);
    let mut cleanup = false;
    let mut rc = 0;
    match mode {
        0 => {
            m.put_word(r + 52, 12);
            for _ in 1..count {
                let array = m.word(r + 20);
                crate::normalization::chunk_pop(m.bytes, array);
            }
        }
        1 => {
            if count != 1 {
                let array = m.word(r + 20);
                if crate::normalization::span_at(m.bytes, array, 1, chunk_slot) == 0 {
                    cleanup = true;
                } else {
                    m.put_word(r + 52, m.word(m.word(chunk_slot) + 12));
                    for _ in 1..count {
                        let array = m.word(r + 20);
                        crate::normalization::chunk_pop(m.bytes, array);
                    }
                }
            }
        }
        2 => {
            let array = m.word(r + 20);
            if crate::normalization::chunk_last(m.bytes, array, chunk_slot) == 0 {
                cleanup = true;
            } else {
                m.put_word(r + 52, m.word(m.word(chunk_slot) + 8));
            }
        }
        3 => {}
        _ => {
            rp_error(m, h, r, k, 0x6986, k.pair_format, &[k.chunk_key, name]);
            cleanup = true;
        }
    }
    if cleanup && matches!(mode, 1 | 2) {
        rp_error(m, h, r, k, 0x6985, k.pair_format, &[k.chunk_key, name]);
    }
    if !cleanup {
        loop {
            let id = k.scratch + 0xc0;
            rc = rp_open_chunk(m, h, r, id, out_len, 0, k);
            if rc >= 0x80000000 {
                cleanup = true;
                break;
            }
            let mut a = crate::selection::arena::Arena::new(0, m.bytes);
            if crate::selection::arena_leaf::compare_strings(&mut a, id, name) == 0 {
                break;
            }
            rc = crate::render_part_a::close_reader_chunk(m, h, r, k);
            if rc >= 0x80000000 {
                cleanup = true;
                break;
            }
        }
        if !cleanup && out_data != 0 {
            rc = crate::render_part_a::reader_chunk_data(m, h, r, m.word(out_len), out_data, k);
            if rc >= 0x80000000 {
                cleanup = true;
            }
        }
    }
    if cleanup {
        rc = 0x8a202014;
        if clone != 0 {
            reader_release_array_m(m, h, m.word(r + 20));
            m.put_word(r + 20, clone);
            m.put_word(r + 52, saved);
        }
        m.put_word(out_len, 0);
        if out_data != 0 {
            m.put_word(out_data, 0);
        }
    } else {
        reader_release_array_m(m, h, clone);
    }
    rp_info(m, r, k, 4, 12);
    rc
}

/// fn_2421b0, arena-aware: close the owner, the name and the chunk table,
/// releasing the reader only on a non-error owner close.
pub fn close_reader_m(
    m: &mut Memory<'_>,
    h: &mut impl OpenHostM,
    r: u32,
    k: &crate::render_part_a::ReaderRuntime,
) -> u32 {
    assert_eq!(m.base, 0);
    if r == 0 {
        return 0x8a202007;
    }
    let env = m.word(r + 8);
    rp_info(m, r, k, 4, 2);
    let rc = crate::normalization::close_owner_pair(m.bytes, r + 24, h);
    if m.word(r + 16) != 0 {
        crate::render_part_c::Host::call(
            h,
            m,
            0x196428,
            &[m.word(m.word(r + 8) + 4), m.word(r + 16)],
        );
    }
    if m.word(r + 20) != 0 {
        reader_release_array_m(m, h, m.word(r + 20));
    }
    if rc < 0x80000000 {
        crate::render_part_c::Host::call(h, m, 0x196428, &[m.word(m.word(r + 8) + 4), r]);
    }
    let logger = m.word(env + 16);
    crate::normalization::log_fanout(m.bytes, logger, k.category, 4, 0, k.messages[3]);
    rc
}

/// fn_179208. The broker's data unmap, with the arena-aware close adapter.
pub fn unmap_m(
    m: &mut Memory<'_>,
    args: [u32; 3],
    k: &crate::boot_part_e::UnmapEnvironment,
    h: &mut impl OpenHostM,
) -> u32 {
    crate::boot_part_e::fn_179208(m, args[0], args[1], args[2], k, &mut CallsM(h))
}
