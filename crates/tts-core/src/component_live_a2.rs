//! Reentrant component group a2. Guest pointers refer to caller-owned memory.
//!
//! These bindings cover the four first-entry speech_1 captures, not every later
//! pump or rule configuration. LiveCalls requires a zero-based arena and 4096
//! disjoint scratch bytes. Each root takes separate restored scratch: 16 bytes
//! for 22b3bc, 64 for 229de4/22b518, 256 for 22a0a8. The host is HeapPlatform;
//! dependency calls run existing Rust ports, never captured outputs.
//!
//! Uncovered paths fail explicitly: active stream/rule logging, unordered
//! iterator lookup, typed rule application, drained-pump marker forwarding,
//! parameter markers, and empty-rule-list stream copying. Voice work buffers
//! and PCRE stores must satisfy the fixed capacities asserted below. The
//! integration test uses an 8 MiB stack for the typed-kernel projection arrays.
//! Only fn_2aa124_m also supports nonzero Memory.base.
use crate::{boot::Memory, frontend};
use crate::rom_region::{Rom, addr, to_us};
/// Internal calls receive the same arena. Implementors must dispatch real ports.
pub trait Calls {
    fn call(&mut self, m: &mut Memory<'_>, entry: u32, args: &[u32]) -> u32;
    /// The ROM whose guest memory is live. US by default; region-aware
    /// hosts (the b dispatch) carry Eu/Kr so data literals follow it.
    fn rom(&self) -> Rom {
        Rom::Us
    }
}
/// fn_22b3bc. Scratch is 16 disjoint guest bytes, restored on return.
pub fn fn_22b3bc_m(
    m: &mut Memory<'_>,
    p: u32,
    tag: u32,
    a: u32,
    b: u32,
    key: u32,
    mime: u32,
    s: u32,
    h: &mut impl Calls,
) -> u32 {
    let saved = core::array::from_fn::<_, 16, _>(|i| m.byte(s + i as u32));
    let rc = (|| {
        if frontend::validate_handle(p != 0, tag, 0x10d, 0x38) >= 0x80000000 {
            return 0x88602008;
        }
        m.put_word(s, 0);
        m.put_word(s + 4, 0);
        let mut rc = h.call(m, 0x19611c, &[m.word(p), m.word(p + 4), s]);
        if rc >= 0x80000000 {
            return 0x88602007;
        }
        if m.byte(p + 0x34) == 0
            && h.call(m, 0x1970a0, &[m.word(p + 0x18)]) == 0
            && m.byte(p + 0x1c) == 0
        {
            return 0;
        }
        let r = h.call(m, 0x175850, &[m.word(m.word(s) + 20), key, s + 4]);
        if r < 0x80000000 && m.word(s + 4) != 1 {
            m.put_word(p + 0x24, 0);
            m.put_word(p + 0x2c, 0);
            return rc;
        }
        let f = m.word(m.word(p + 0x20) + 0x20);
        let r = h.call(m, f, &[a, b, 0x86, mime, 0, 0, p + 0x24]);
        if r >= 0x80000000 {
            rc = r;
        } else {
            let f = m.word(m.word(p + 0x20) + 0x20);
            let r = h.call(m, f, &[a, b, 0x86, mime, 0, 1, p + 0x2c]);
            if r >= 0x80000000 {
                rc = r;
            }
        }
        if rc >= 0x80000000 {
            h.call(m, 0x22b238, &[p, tag]);
        }
        rc
    })();
    for (i, v) in saved.into_iter().enumerate() {
        m.put_byte(s + i as u32, v);
    }
    rc
}

/// Reentrant allocator and map host for the component adapters.
/// Scratch spans 4096 disjoint guest bytes and is restored by each entry.
fn terminated(m: &[u8], p: u32) -> &[u8] {
    let s = &m[p as usize..];
    &s[..s.iter().position(|v| *v == 0).unwrap() + 1]
}

pub struct LiveCalls<'a> {
    boot: crate::engine::SpeechBootHost<'a>,
    pub scratch: u32,
    rom: Rom,
}
impl<'a> LiveCalls<'a> {
    pub fn new(platform: &'a mut dyn crate::engine::HeapPlatform, scratch: u32) -> Self {
        Self {
            boot: crate::engine::SpeechBootHost {
                platform,
                fault: None,
            },
            scratch,
            rom: Rom::Us,
        }
    }
    /// Region-aware construction: data literals (D_8d7764 and friends)
    /// follow the running region through rom_region.
    pub fn with_rom(mut self, rom: Rom) -> Self {
        self.rom = rom;
        self
    }
    pub fn fault(&self) -> Option<crate::engine::Error> {
        self.boot.fault
    }
}
impl crate::boot_part_e::Host for LiveCalls<'_> {
    fn call(&mut self, m: &mut Memory<'_>, f: u32, a: &[u32]) -> u32 {
        Calls::call(self, m, f, a)
    }
}
impl crate::speech_part_1::Host for LiveCalls<'_> {
    fn get(&mut self, _: &mut Memory<'_>, f: u32, _: u32, _: u32, _: u32) -> (u32, u32, u32) {
        panic!("unbound get {f:x}")
    }
    fn call(&mut self, m: &mut Memory<'_>, f: u32, a: &[u32]) -> u32 {
        Calls::call(self, m, f, a)
    }
}
impl crate::speech_part_2::Host for LiveCalls<'_> {
    fn call(&mut self, m: &mut Memory<'_>, f: u32, a: &[u32]) -> u32 {
        Calls::call(self, m, f, a)
    }
}
impl crate::boot_part_b::FormatHost for LiveCalls<'_> {}
impl crate::log_live::LogMethodsM for LiveCalls<'_> {
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
        if method == 0x1738e8 {
            self.boot
                .invoke(m, method, &[object, category, level, arg, format, varargs])
        } else {
            Calls::call(self, m, method, &[object, category, level, arg, format, varargs])
        }
    }
}
impl crate::boot::Calls for LiveCalls<'_> {
    fn call(&mut self, m: &mut Memory<'_>, f: u32, a: &[u32]) -> u32 {
        Calls::call(self, m, f, a)
    }
    fn call_wide(&mut self, _: &mut Memory<'_>, f: u32, _: &[u32]) -> u64 {
        panic!("unbound wide call {f:x}")
    }
}
impl Calls for LiveCalls<'_> {
    fn rom(&self) -> Rom {
        self.rom
    }
    fn call(&mut self, m: &mut Memory<'_>, f: u32, a: &[u32]) -> u32 {
        // Guest-sourced code pointers (vtable slots, comparator callbacks
        // read from the live heap) arrive in the running region's numbering.
        // Map them back to the US entries the arms below dispatch on; the
        // mapping is US-identity, and unknown addresses pass through to
        // their own unbound entry, so US behavior is bit-identical. The
        // inferred region also selects data twins below.
        let rom = Rom::from_entry(f);
        let f = to_us(rom, f);
        if let Some(r) = m.boot_round10_call(f, a) {
            return r;
        }
        let s = self.scratch;
        match f {
            0x1994d0 => crate::frontend::divmod_u32(a[0], a[1]).0,
            0x14fe9c | 0x15140c | 0x15017c | 0x150878 | 0x150c0c => stream_call(m, f, a, self),
            0x2a977c => {
                let mut arena = crate::selection::arena::Arena::new(m.base, m.bytes);
                crate::selection::frame::render_frame(
                    &mut arena, self, a[0], a[1], a[2], a[3], a[4],
                )
            }
            0x2dbb2c | 0x2dca78 | 0x2dcb84 | 0x2dc4cc => numeric_call(m, f, a),
            0x195ae8 => frontend::move_bytes(m.bytes, a[0], a[1], a[2]),
            0x175a1c => m
                .boot_lookup_pure(f, a, 0x196980)
                .expect("reentrant string getter required"),
            0x17bdc4 => crate::normalization::transcode_u16(
                m.bytes, a[0], a[1], a[2], a[3], a[4], a[5], a[6],
            ),
            0x17bb44 => {
                let extra: [u8; 256] = core::array::from_fn(|i| m.byte(0x8dc1f3 + i as u32));
                let mask: [u8; 16] = core::array::from_fn(|i| m.byte(0x8dc1e0 + i as u32));
                let base: [u8; 1024] = core::array::from_fn(|i| m.byte(0x8dc2f4 + i as u32));
                let t = crate::normalization::DecoderTables {
                    extra: &extra,
                    mask: &mask,
                    mask_base: 6,
                    base: &base,
                    base_base: 0,
                };
                crate::normalization::decode_units(
                    m.bytes, a[0], a[1], a[2], a[3], a[4], a[5], a[6], &t,
                )
            }
            0x2ab68c => {
                let it = a[0];
                let lang = m.word(it + 24);
                let mut language = [0u8; 64];
                let n = term(m, lang).len();
                language[..n].copy_from_slice(term(m, lang));
                let mut input = [0; 8];
                let n2 = term(m, a[1]).len();
                input[..n2].copy_from_slice(term(m, a[1]));
                let r = crate::prosody_rules::matches_language(
                    m.byte(it + 20),
                    Some(&mut language[..n + 1]),
                    Some(&input),
                )
                .unwrap();
                for i in 0..=n {
                    m.put_byte(lang + i as u32, language[i]);
                }
                u32::from(r)
            }
            0x2ac0ac => apply_rules_live(m, a[0], a[1], a[2], s + 512, self),
            0x14f918 => close_stream_m(m, a[0], a[1], self),
            0x2312fc|0x231ba0 => {
                // fn_2312fc/fn_231ba0 text/marker xcoders (round 17: heap
                // teardown dispatches 0x231ba0 through here; the live Host
                // binds the same entries). 1:1 with engine_live's arm.
                let mut args = [0u32; 7];
                args[..a.len().min(7)].copy_from_slice(&a[..a.len().min(7)]);
                let call = crate::engine::StageCall { entry: f, args };
                let mut regions = [crate::engine::Region { base: m.base, bytes: &mut *m.bytes }];
                let mut sparse = match crate::engine::Memory::new(&mut regions) {
                    Ok(v) => v,
                    Err(e) => {
                        self.boot.fault = Some(e);
                        return 0x80002001;
                    }
                };
                let result = if f == 0x2312fc {
                    let mut scratch = [0u8; 8192];
                    crate::engine::xcoder_text_stage(&mut sparse, call, &mut scratch)
                } else {
                    crate::engine::xcoder_markers_stage(&mut sparse, call)
                };
                match result {
                    Ok(r) => r.r0.expect("XCODER result"),
                    Err(e) => {
                        self.boot.fault = Some(e);
                        0x80002001
                    }
                }
            }
            0x226f14 => {
                crate::speech_part_2::fn_226f14(m, a[0], a[1], self);
                0
            }
            0x22508c => crate::speech_part_2::fn_22508c(m, a[0], a[1], a[2], a[3]),
            0x1738e8 => {
                crate::log_live::fn_1738e8(m, a[0], a[1], a[2], a[3], a[4], a[5], self)
            }
            0x2a9608 => {
                let mut out = 0i32;
                let rc = crate::selection::inverse_percent(a[0] as i32, &mut out);
                m.put_word(a[1], out as u32);
                rc
            }
            0x225138 => crate::speech_part_2::fn_225138(m, a[0], a[1], a[2], s, self),
            0x2289b0 => {
                // fn_2289b0: the parameter check the global callbacks run
                // (mirrors engine_live's arm): validate name/value, report
                // through the valid flag.
                let mut valid = 0u8;
                let rc = crate::frontend_output::validate_parameter(
                    terminated(m.bytes, a[1]),
                    terminated(m.bytes, a[2]),
                    &mut valid,
                );
                m.put_byte(a[3], valid);
                rc
            }
            0x228aac => crate::speech_part_2::fn_228aac(m, a[0], a[1], a[2], s, self),
            0x22c6dc => {
                // fn_22c6dc: the inserter's parameter change (mirrors
                // engine_live's arm). Only the field the name selects is
                // written back.
                let mut p = crate::prosody_rules::InserterParameters {
                    input_active: m.word(a[0] + 0x10),
                    output_active: m.word(a[0] + 0x18),
                    frequency: m.word(a[0] + 0x3c),
                    buffer_samples: m.word(a[0] + 0x40),
                    volume: m.half(a[0] + 0x60),
                    rate: m.half(a[0] + 0x62),
                    adjust_volume: m.byte(a[0] + 0x64),
                    adjust_rate: m.byte(a[0] + 0x65),
                };
                let rc = p.set(terminated(m.bytes, a[1]), terminated(m.bytes, a[2]));
                if p.frequency != m.word(a[0] + 0x3c) {
                    m.put_word(a[0] + 0x3c, p.frequency);
                }
                if p.buffer_samples != m.word(a[0] + 0x40) {
                    m.put_word(a[0] + 0x40, p.buffer_samples);
                }
                if p.volume != m.half(a[0] + 0x60) {
                    m.put_half(a[0] + 0x60, p.volume);
                }
                if p.rate != m.half(a[0] + 0x62) {
                    m.put_half(a[0] + 0x62, p.rate);
                }
                if p.adjust_volume != m.byte(a[0] + 0x64) {
                    m.put_byte(a[0] + 0x64, p.adjust_volume);
                }
                if p.adjust_rate != m.byte(a[0] + 0x65) {
                    m.put_byte(a[0] + 0x65, p.adjust_rate);
                }
                rc
            }
            0x22c5f0 => {
                // fn_22c5f0: the inserter's parameter check (mirrors
                // engine_live's arm).
                let p = crate::prosody_rules::InserterParameters {
                    input_active: m.word(a[0] + 0x10),
                    output_active: m.word(a[0] + 0x18),
                    ..Default::default()
                };
                let mut valid = 1u8;
                let rc = p.validate(terminated(m.bytes, a[1]), terminated(m.bytes, a[2]), &mut valid);
                m.put_byte(a[3], valid);
                rc
            }
            0x196e28 => remove_element_m(m, a[0], a[1], self),
            0x2aea98 => replace_live(m, a.try_into().unwrap(), s + 256, self),
            0x4a79b8 => crate::speech_part_3::fn_4a79b8(m, a[0], a[1], a[2], self),
            0x19611c => {
                let pair = core::array::from_fn(|i| m.byte(0x8d7660 + i as u32));
                let half = if a[0] == 0 { None } else { Some(m.half(a[0])) };
                let mut ctx = 0;
                let r =
                    frontend::resolve_engine_context(a[0], half, a[1], &pair, 0x8d7640, &mut ctx);
                m.put_word(a[2], ctx);
                r
            }
            0x151df8 => {
                // fn_151df8: the parameter change callback (round 17:
                // the lex-teardown flush applies pending edits here).
                // 1:1 with src/fn_151914.c fn_151df8 and engine_live's arm:
                // log, then store the parsed number in the field the name
                // selects. Scratch is LiveCalls' own (saved/restored).
                // The __NUAN_alternative_esc_seq__ path stays loud
                // (unbound 0x15af9c, as in engine_live).
                let (p, name, value) = (a[0], a[1], a[2]);
                if value == 0 || m.byte(value) == 0 || p == 0 {
                    return 0;
                }
                let saved = m.word(s);
                m.put_word(s, 0);
                let rc = self.call(m, 0x19611c, &[m.word(p), m.word(p + 4), s]);
                if rc < 0x80000000 {
                    if self.call(m, 0x195d58, &[name, 0x151f24]) == 0 {
                        m.put_word(
                            p + 0x38,
                            crate::normalization::atoi_scan(m.bytes, value),
                        );
                    } else if self.call(m, 0x195d58, &[name, 0x151f3c]) == 0 {
                        m.put_word(
                            p + 0x34,
                            crate::normalization::atoi_scan(m.bytes, value),
                        );
                    } else if self.call(m, 0x195d58, &[name, 0x151f48]) == 0 {
                        m.put_word(
                            p + 0xe8,
                            crate::normalization::atoi_scan(m.bytes, value),
                        );
                    } else if self.call(m, 0x195d58, &[name, 0x151f60]) == 0 {
                        // fn_15af9c: the alternative escape sequence handler is
                        // not bound in the live path.
                        self.boot.fault =
                            Some(crate::engine::Error::MissingStreamAdapter { entry: 0x15af9c });
                        m.put_word(s, saved);
                        return 0x80002001;
                    }
                }
                m.put_word(s, saved);
                rc
            }
            0x195d58 => crate::frontend::compare_str(&m.bytes[a[0] as usize..], &m.bytes[a[1] as usize..]) as u32,
            0x151840 => {
                // fn_151840: the parameter check the value callbacks run
                // (round 17: unblocks the lex-teardown flush, whose
                // set_parameter path validates values here). 1:1 with
                // src/fn_151840.c; digit parse matches fn_17c550 (leading
                // whitespace 9-13/32, optional sign, decimal digits).
                let (p, name, s, out) = (a[0], a[1], a[2], a[3]);
                let mut ret = 0u32;
                let mut flag = 1u8;
                if s != 0 && m.byte(s) != 0 && p != 0 {
                    let h0 = m.word(p);
                    let h1 = m.word(p.wrapping_add(4));
                    let pair =
                        core::array::from_fn(|i| m.byte(0x8d7660 + i as u32));
                    let half = if h0 == 0 { None } else { Some(m.half(h0)) };
                    let mut ctx = 0u32;
                    let r = frontend::resolve_engine_context(
                        h0, half, h1, &pair, 0x8d7640, &mut ctx,
                    );
                    if r < 0x80000000 {
                        // NUL-terminated name vs literal: C fn_195d58
                        // returns 0 exactly on equality.
                        let nm = terminated(m.bytes, name);
                        let nm = &nm[..nm.len() - 1];
                        let parse = |mut at: u32| -> i32 {
                            while matches!(m.byte(at), 9 | 10 | 11 | 12 | 13 | 32) {
                                at = at.wrapping_add(1);
                            }
                            let mut n = 0i32;
                            let mut c = m.byte(at);
                            at = at.wrapping_add(1);
                            let sign = c;
                            if c == b'-' || c == b'+' {
                                c = m.byte(at);
                                at = at.wrapping_add(1);
                            }
                            while c.is_ascii_digit() {
                                n = (c as i32).wrapping_add(n.wrapping_mul(10)).wrapping_sub(b'0' as i32);
                                c = m.byte(at);
                                at = at.wrapping_add(1);
                            }
                            if sign == b'-' { -n } else { n }
                        };
                        if nm == b"readmode" {
                            if (parse(s).wrapping_sub(1) as u32) >= 4 {
                                ret = 0x8120200f;
                                flag = 0;
                            }
                        } else if nm == b"waitfactor" {
                            if (parse(s) as u32) >= 10 {
                                ret = 0x8120200f;
                                flag = 0;
                            }
                        }
                    }
                }
                m.put_byte(out, flag);
                ret
            }
            0x1758ac | 0x176f2c | 0x176f5c | 0x175850 | 0x19668c | 0x176054 | 0x175170
            | 0x196980 | 0x176104 | 0x1966ac | 0x174b38 => {
                crate::speech_part_1::parameter_call_m(m, f, a, s + 1024, self)
            }
            0x196830 => crate::component_live_a::release_owner_m(m, a[0], self),
            0x19690c => crate::component_live_a::clone_str_m(m, a[0], a[1], a[2], self),
            0x14df44 => crate::component_live_a::clone_labeled_word_m(m, a[0], a[1], a[2], self),
            0x151744 => addr(self.rom(), 0x8d77a8),
            0x151610 => {
                if a[0] == 0
                    || ![6, 12, 21, 50, 36, 39, 37, 34, 22, 40, 26, 29].contains(&m.half(a[0]))
                {
                    return 1;
                }
                let src = m.word(a[0] + 20);
                let mut n = 0;
                if src != 0 {
                    while m.half(src + n * 2) != 0 {
                        n += 1;
                    }
                }
                let dst = Calls::call(self, m, 0x196394, &[a[1], n * 2 + 2]);
                if dst == 0 {
                    return 0;
                }
                for i in 0..=n {
                    m.put_half(dst + i * 2, if n == 0 { 0 } else { m.half(src + i * 2) });
                }
                m.put_word(a[2] + 20, dst);
                1
            }
            0x1516c8 => {
                if a[0] != 0
                    && [6, 12, 21, 50, 36, 39, 37, 34, 22, 40, 26, 29].contains(&m.half(a[0]))
                    && m.word(a[0] + 20) != 0
                {
                    Calls::call(self, m, 0x196428, &[a[1], m.word(a[0] + 20)]);
                    m.put_word(a[0] + 20, 0);
                }
                0
            }
            0x2aa124 => fn_2aa124_m(m, a[0]),
            0x14dfc0 => crate::normalization::list_last(m.bytes, a[0]),
            0x14dff4 => {
                let r = frontend::compare_str(
                    &m.bytes[m.offset(m.word(a[0]))..],
                    &m.bytes[m.offset(m.word(a[1]))..],
                );
                if r != 0 {
                    r as u32
                } else {
                    (m.half(a[0] + 4) as i32 - m.half(a[1] + 4) as i32) as u32
                }
            }
            0x14e084 => {
                let p = a[0];
                if p == 0 {
                    return 0x88f02007;
                }
                let v = m.half(p + 22);
                m.put_half(p + 22, v.wrapping_add(1));
                m.put_word(a[2], p);
                0
            }
            0x14f010 => fn_14f010_m(m, a.try_into().unwrap(), s, self),
            0x196860 => {
                let mut x = 0;
                let mut y = 0;
                let r = frontend::step_iter(
                    m.bytes,
                    (a[0] != 0).then_some(a[0]),
                    (a[1] != 0).then_some(&mut x),
                    (a[2] != 0).then_some(&mut y),
                );
                if a[1] != 0 {
                    m.put_word(a[1], x);
                }
                if a[2] != 0 {
                    m.put_word(a[2], y);
                }
                r
            }
            0x196794 => alloc_handle_for_key_m(m, a[0], a[1], a[2], a[3], self),
            0x196a64 => create_block_m(m, a.try_into().unwrap(), self),
            0x196db0 => crate::db_live::append_element_m(self, m, a[0], a[1], s + 3072),
            0x174a04 => m.fn_174a04(self, a[0], a[1], a[2]),
            0x1749ac => {
                crate::speech_part_1::fn_1749ac(m, a[0], a[1], self);
                0
            }
            0x14e0b4 => {
                release_stream_state_m(m, a[0], a[1], self);
                0
            }
            0x196c7c => crate::reentrant2::destroy_array_m(m, a[0], self),
            0x184f84 => {
                m.boot_heap_release(a[0], a[1]);
                0
            }
            0x1963e0 => crate::boot_part_e::fn_1963e0(m, a[0], a[1], a[2], self),
            _ => {
                let r = self.boot.invoke(m, f, a);
                assert!(self.boot.fault.is_none(), "unbound {f:x} {a:x?}");
                r
            }
        }
    }
}
/// fn_196794 with memory-passing comparison and allocation.
pub fn alloc_handle_for_key_m(
    m: &mut Memory<'_>,
    p: u32,
    key: u32,
    ordered: u32,
    out: u32,
    h: &mut impl Calls,
) -> u32 {
    if p == 0 || out == 0 {
        return 0x89002007;
    }
    let node = if key == 0 {
        frontend::last_link(m.bytes, p + 24)
    } else if ordered != 0 {
        let head = m.word(p + 24);
        let flag = m.byte(p + 64);
        crate::reentrant2::find_node_m(m.bytes, p + 24, head, flag, key, |a, b, bytes| {
            let mut m = Memory { base: 0, bytes };
            let f = m.word(m.word(p + 60) + 8);
            h.call(&mut m, f, &[a, b]) as i32
        })
        .unwrap_or(0)
    } else {
        // Ordered-map clients here always request a lower-bound iterator.
        panic!("unordered fn_196794 needs reentrant search")
    };
    if key != 0 && node == 0 {
        return 0x89002014;
    }
    let object = h.call(m, 0x1963b8, &[m.word(p + 20), 1, 12]);
    m.put_word(out, object);
    if object == 0 {
        return 0x8900200a;
    }
    m.put_word(object, p);
    m.put_word(object + 4, 0);
    m.put_word(object + 8, node);
    0
}
/// fn_196a64. Reentrant allocation, original field stores unchanged.
pub fn create_block_m(m: &mut Memory<'_>, a: [u32; 6], h: &mut impl Calls) -> u32 {
    let [heap, b, c, d, e, out] = a;
    if (c == 0) != (d == 0) || out == 0 {
        return 0;
    }
    let p = h.call(m, 0x1963b8, &[heap, 1, 32]);
    m.put_word(out, p);
    if p == 0 {
        return 0;
    }
    for (off, v) in [
        (0, 0),
        (4, b),
        (8, c),
        (12, d),
        (16, 0),
        (20, 0),
        (28, heap),
    ] {
        m.put_word(p + off, v);
    }
    m.put_half(p + 24, e as u16);
    1
}
/// fn_14e0b4. All destruction callbacks receive the current arena.
pub fn release_stream_state_m(m: &mut Memory<'_>, p: u32, _heap: u32, h: &mut impl Calls) {
    if p == 0 {
        return;
    }
    let refs = m.half(p + 22);
    if refs > 1 {
        m.put_half(p + 22, refs - 1);
        return;
    }
    let heap = m.word(m.word(p + 4) + 4);
    if m.word(p + 16) != 0 {
        h.call(m, 0x196428, &[heap, m.word(p + 16)]);
    }
    if m.word(p + 12) != 0 {
        h.call(m, 0x196c7c, &[m.word(p + 12)]);
    }
    if m.word(p + 60) != 0 {
        if m.word(p + 36) != 0 {
            let buffer = m.word(p + 60);
            let old = m.word(p + 56);
            let stride = m.half(p + 28) as u32;
            let end = buffer.wrapping_add(m.word(p + 44).wrapping_sub(old));
            let mut element = buffer.wrapping_add(m.word(p + 52).wrapping_sub(old));
            let mut dpos = m.word(p + 52);
            while element < end {
                h.call(m, m.word(p + 36), &[element, heap]);
                element = element.wrapping_add(stride);
                dpos = dpos.wrapping_add(stride);
            }
            m.put_word(p + 52, dpos);
        }
        h.call(m, 0x196428, &[heap, m.word(p + 60)]);
    }
    h.call(m, 0x196428, &[heap, p]);
}
fn term<'a>(m: &'a Memory<'_>, p: u32) -> &'a [u8] {
    let s = &m.bytes[m.offset(p)..];
    &s[..s.iter().position(|&b| b == 0).unwrap()]
}
/// fn_14f010. Scratch spans 256 bytes and is restored, including early returns.
pub fn fn_14f010_m(m: &mut Memory<'_>, a: [u32; 7], s: u32, h: &mut impl Calls) -> u32 {
    let saved = core::array::from_fn::<_, 256, _>(|i| m.byte(s + i as u32));
    let r = open_stream_inner(m, a, s, h);
    for (i, b) in saved.into_iter().enumerate() {
        m.put_byte(s + i as u32, b);
    }
    r
}
fn open_stream_inner(m: &mut Memory<'_>, a: [u32; 7], s: u32, h: &mut impl Calls) -> u32 {
    let [p, tag, id, content, mut elements, mode, out] = a;
    let r = frontend::validate_handle(p != 0, tag, 0x47dd, 32);
    if r >= 0x80000000 {
        return 0x81706008;
    }
    let r = h.call(m, 0x19611c, &[m.word(p), m.word(p + 4), s]);
    if r >= 0x80000000 {
        return r;
    }
    let context = m.word(s);
    // The trace is filtered at the logger before it can read its format or arguments.
    assert!(
        m.word(context + 16) == 0 || m.byte(m.word(context + 16) + 48) < 3,
        "active StreamOpen logging needs format literals"
    );
    if content == 0 || m.byte(content) == 0 || out == 0 || term(m, content).len() >= 128 {
        return 0x88f02007;
    }
    m.put_word(out, 0);
    m.put_word(out + 4, 0);
    let len = term(m, content).len() as u32;
    for i in 0..len {
        m.put_byte(
            s + 32 + i,
            crate::fe_text::fold_17c8c0(m.byte(content + i) as u32) as u8,
        );
    }
    m.put_byte(s + 32 + len, 0);
    m.put_word(s + 8, s + 32);
    m.put_word(s + 12, 0);
    m.put_word(s + 4, 0);
    let mut rc = h.call(m, 0x196794, &[m.word(p + 12), s + 8, 1, s + 4]);
    let mut state = 0;
    let mut last_key = 0;
    if rc < 0x80000000 {
        loop {
            let r = h.call(m, 0x196860, &[m.word(s + 4), s + 16, s + 20]);
            if r >= 0x80000000 {
                break;
            }
            let key = m.word(s + 16);
            let found = m.word(s + 20);
            if frontend::compare_str(
                &m.bytes[m.offset(s + 32)..],
                &m.bytes[m.offset(m.word(key))..],
            ) != 0
            {
                break;
            }
            let pipe = m.half(p + 8);
            let fp = m.half(found + 20);
            if !(pipe > fp || (mode == 0 && pipe == fp)) {
                break;
            }
            state = found;
            last_key = key;
        }
        h.call(m, 0x196830, &[m.word(s + 4)]);
    }
    if rc & 0x1fff == 20 {
        rc = 0;
    }
    if rc >= 0x80000000 {
        return rc;
    }
    let heap = m.word(context + 4);
    if mode == 0 {
        if state == 0 {
            return 0x88f02014;
        }
    } else {
        if mode != 1 && mode != 2 {
            return 0x88f02007;
        }
        if state != 0 && m.half(state + 20) == m.half(p + 8) {
            return 0x88f02002;
        }
        m.put_half(
            s + 12,
            if last_key == 0 {
                0
            } else {
                m.half(last_key + 4).wrapping_add(1)
            },
        );
        state = h.call(m, 0x1963b8, &[heap, 1, 72]);
        if state == 0 {
            return 0x88f0200a;
        }
        let r = h.call(
            m,
            0x196a64,
            &[
                heap,
                m.word(addr(h.rom(), 0x8d7764) + 0xb8),
                m.word(addr(h.rom(), 0x8d7764) + 0xbc),
                m.word(addr(h.rom(), 0x8d7764) + 0xc0),
                1,
                state + 12,
            ],
        );
        if r != 1 {
            h.call(m, 0x196428, &[heap, state]);
            return 0x88f0200a;
        }
        let text = h.call(m, 0x196394, &[heap, len + 1]);
        m.put_word(state + 16, text);
        if text == 0 {
            h.call(m, 0x196c7c, &[m.word(state + 12)]);
            h.call(m, 0x196428, &[heap, state]);
            return 0x88f0200a;
        }
        let markers = term(m, s + 32) == b"application/x-realspeak-markers-pp;version=4.0";
        if elements == 0 && markers {
            // D_8d77a8 (the SYNTHSTREAM element callbacks) moves per
            // region; the table carries the twin (lane c3 round 11).
            elements = addr(h.rom(), 0x8d77a8);
        }
        m.put_word(state, p);
        m.put_word(state + 4, context);
        frontend::copy_str(m.bytes, text, s + 32);
        m.put_half(state + 20, m.half(p + 8));
        m.put_half(state + 22, 1);
        m.put_half(state + 24, id as u16);
        if elements != 0 && m.half(elements) != 0 {
            frontend::move_bytes(m.bytes, state + 28, elements, 12);
        }
        m.put_byte(state + 40, u8::from(mode == 2));
        let t = term(m, s + 32);
        let is_text = t.starts_with(b"text/") && !t.windows(14).any(|w| w == b"charset=wchar_t");
        m.put_byte(state + 41, u8::from(is_text));
        if m.word(p + 28) != 0
            && [
                b"text/plain;charset=utf-16".as_slice(),
                b"application/synthesis+ssml",
                b"text/xml",
            ]
            .contains(&term(m, s + 32))
        {
            m.put_word(p + 28, 0);
            m.put_byte(state + 40, 1);
            m.put_byte(state + 42, 1);
        }
        rc = h.call(m, 0x19668c, &[m.word(p + 12), s + 8, state]);
        h.call(m, 0x14e0b4, &[state, heap]);
        if rc >= 0x80000000 {
            return rc;
        }
    }
    let stream = h.call(m, 0x1963b8, &[heap, 1, 12]);
    if stream == 0 {
        return 0x88f0200a;
    }
    m.put_word(s + 24, stream);
    if mode == 0 && h.call(m, 0x196db0, &[m.word(state + 12), s + 24]) == 0 {
        h.call(m, 0x196428, &[heap, stream]);
        return 0x88f0200a;
    }
    m.put_byte(stream, mode as u8);
    m.put_half(stream + 2, id as u16);
    m.put_word(stream + 4, state);
    m.put_word(stream + 8, m.word(state + 48));
    m.put_half(state + 22, m.half(state + 22).wrapping_add(1));
    if mode == 1 || mode == 2 {
        m.put_word(state + 8, stream);
    }
    m.put_word(out, stream);
    m.put_word(out + 4, 0x47de);
    rc
}

impl crate::db_live::PoolHooksM for LiveCalls<'_> {
    fn log_pool_m(&mut self, _: u32, _: u32, _: &mut Memory<'_>) {
        panic!("unexpected pool log")
    }
}
impl crate::db_live::DbHooksM for LiveCalls<'_> {
    fn call_m(&mut self, f: u32, a: &[u32], m: &mut Memory<'_>) -> u32 {
        Calls::call(self, m, f, a)
    }
}

/// fn_2aa124. Reset the SELECT_BET2 engine's caller-owned work buffers.
pub fn fn_2aa124_m(m: &mut Memory<'_>, pp: u32) -> u32 {
    if pp == 0 || m.word(pp) == 0 {
        return u32::MAX;
    }
    let p = m.word(pp);
    let mut active = m.word(p + 0x34);
    crate::morphology::activate_dictionary(&mut active);
    m.put_word(p + 0x34, active);
    m.put_word(p + 0x14c, 0);
    m.put_word(p + 0x120, 0);
    m.put_half(p + 0x106, 0);
    fn clear(m: &mut Memory<'_>, p: u32, ptr: u32, count: u32, shift: u32) {
        let dst = m.word(p + ptr);
        let n = m.word(p + count).wrapping_shl(shift);
        frontend::fill_bytes(m.bytes, dst.wrapping_sub(m.base), 0, n);
    }
    clear(m, p, 0x18c, 0x194, 2);
    clear(m, p, 0x19c, 0x1a4, 2);
    m.put_half(p + 0x138, m.half(p + 0x100));
    m.put_half(p + 0x13a, 0);
    clear(m, p, 0x140, 0x144, 2);
    let n = m.word(p + 0x60) as usize;
    assert!(n < 42);
    let ptr = m.word(p + 0x140);
    let mut input = [0; 42];
    let mut output = [0; 42];
    for i in 0..=n {
        input[i] = m.word(ptr + i as u32 * 4) as i32;
    }
    crate::selection::prediction_coefficients(&input, &mut output, n, m.word(p + 0x1b0) as i32)
        .unwrap();
    for i in 0..=n {
        m.put_word(ptr + i as u32 * 4, output[i] as u32);
    }
    for (ptr, count) in [
        (0x8c, 0x94),
        (0x7c, 0x84),
        (0xbc, 0xc4),
        (0xcc, 0xd4),
        (0x9c, 0xa4),
        (0x17c, 0x184),
    ] {
        clear(m, p, ptr, count, 1);
    }
    m.put_word(p + 0x16c, u32::MAX);
    m.put_word(p + 0x168, 4);
    m.put_word(p + 0x170, 0);
    m.put_half(p + 0x164, 2);
    clear(m, p, 0x6c, 0x74, 2);
    let count = m.word(p + 0xec) as i32;
    let mut filled = [0i16; 2048];
    assert!(count <= 2048);
    crate::selection::fill_samples(&mut filled, m.half(p + 0x100) as i16, count).unwrap();
    for i in 0..count.max(0) as u32 {
        m.put_half(m.word(p + 0xe4) + i * 2, filled[i as usize] as u16);
    }
    if m.byte(p + 0x32) >= m.byte(p + 0x118) {
        clear(m, p, 0x10c, 0x114, 1);
    }
    0
}
/// fn_229de4. Scratch is 64 disjoint bytes, restored before return.
pub fn fn_229de4_m(
    m: &mut Memory<'_>,
    p: u32,
    tag: u32,
    stream: [u32; 2],
    speech_mime: u32,
    marker_mime: u32,
    s: u32,
    h: &mut impl Calls,
) -> u32 {
    if frontend::validate_handle(p != 0, tag, 0x4e7, 0x70) >= 0x80000000 {
        return 0x84e02008;
    }
    let saved = core::array::from_fn::<_, 64, _>(|i| m.byte(s + i as u32));
    let prefix = b"audio/L16;rate=";
    for (i, b) in prefix.iter().enumerate() {
        m.put_byte(s + i as u32, *b);
    }
    let rate = m.word(p + 0x1c) as i32;
    let off = m.offset(s) + prefix.len();
    frontend::render_signed(rate, &mut m.bytes[off..off + 16], 10).unwrap();
    let mut result = 0;
    for (mime, mode, out, format) in [
        (speech_mime, 0, p + 0x40, 0),
        (marker_mime, 0, p + 0x50, 0),
        (s, 1, p + 0x48, 0),
        (marker_mime, 1, p + 0x58, u32::MAX),
    ] {
        let format = if format == u32::MAX {
            h.call(m, m.word(m.word(p + 0x10) + 0x4c), &[])
        } else {
            format
        };
        let entry = m.word(m.word(p + 0x10) + 32);
        result = h.call(
            m,
            entry,
            &[stream[0], stream[1], 16, mime, format, mode, out],
        );
        if result >= 0x80000000 {
            break;
        }
    }
    if result < 0x80000000 {
        result = if h.call(m, 0x2aa124, &[p + 0x28]) != 0 {
            0x84e02000
        } else {
            0
        };
    }
    m.put_word(p + 0x64, 0);
    m.put_word(p + 0x60, 0);
    if result >= 0x80000000 {
        h.call(m, 0x229cc0, &[p, tag]);
    }
    for (i, b) in saved.into_iter().enumerate() {
        m.put_byte(s + i as u32, b);
    }
    result
}

pub fn compact_stream_buffer_m(
    mem: &mut [u8],
    state: u32,
    required: u32,
    vt: &mut impl frontend::HeapVtableM,
    dtor: &mut impl FnMut(&mut [u8], u32, u32),
) {
    let buffer = mem_word(mem, state + 60);
    if buffer == 0 || mem[(state + 40) as usize] != 0 {
        return;
    }
    let write = mem_word(mem, state + 44);
    let minimum = {
        let readers = mem_word(&*mem, state + 12);
        if readers == 0 {
            write
        } else {
            let base = mem_word(&*mem, readers);
            let table = ElementTable {
                data: &mem[base as usize..],
                elem_size: mem_half(&*mem, readers + 4),
                count: mem_word(&*mem, readers + 20),
            };
            let mut minimum = write;
            let mut index = 0u32;
            loop {
                let mut off = 0u32;
                if element_at(Some(&table), index, Some(&mut off)) != 1 {
                    break;
                }
                let stream = mem_word(&*mem, base.wrapping_add(off));
                let rp = mem_word(&*mem, stream.wrapping_add(8));
                if rp < minimum {
                    minimum = rp;
                }
                index = index.wrapping_add(1);
            }
            minimum
        }
    };
    let old = mem_word(mem, state + 56);
    let stride = mem_half(mem, state + 28) as u32;
    let mut discarded = minimum.wrapping_sub(old);
    if stride > 1 {
        let (_, rem) = divmod_u32(minimum.wrapping_sub(old), stride);
        discarded = discarded.wrapping_sub(rem);
    }
    if mem_word(mem, state + 36) != 0 && discarded != 0 {
        let heap = mem_word(mem, mem_word(mem, state + 4) + 4);
        let end = buffer.wrapping_add(discarded);
        let mut element = (mem_word(mem, state + 52).wrapping_sub(old)).wrapping_add(buffer);
        let mut dpos = mem_word(mem, state + 52);
        while element < end {
            dtor(mem, element, heap);
            element = element.wrapping_add(stride);
            dpos = dpos.wrapping_add(stride);
        }
        mem_store(mem, state + 52, dpos);
    }
    let config = mem_word(mem, state);
    let mut shrink = false;
    if required == 0 {
        if mem_word(mem, config + 24) <= discarded {
            shrink = true;
        } else {
            return;
        }
    } else if discarded < required && discarded < mem_word(mem, config + 20) {
        return;
    }
    let remaining = write.wrapping_sub(minimum);
    if remaining != 0 {
        move_bytes(mem, buffer, buffer.wrapping_add(discarded), remaining);
    }
    mem_store(mem, state + 56, old.wrapping_add(discarded));
    if shrink {
        let capacity = mem_word(mem, state + 64);
        let heap = mem_word(mem, mem_word(mem, state + 4) + 4);
        if capacity == discarded || write == minimum {
            heap_release_m(mem, nz(heap), Some(buffer), vt);
            mem_store(mem, state + 60, 0);
            mem_store(mem, state + 64, 0);
        } else {
            let newbuf =
                heap_realloc_m(mem, nz(heap), buffer, capacity.wrapping_sub(discarded), vt);
            if newbuf != 0 {
                mem_store(mem, state + 60, newbuf);
                mem_store(mem, state + 64, capacity.wrapping_sub(discarded));
            }
        }
    }
}

pub fn read_direct_end_m(
    mem: &mut [u8],
    stream: Option<u32>,
    b: u32,
    bytes_read: u32,
    vt: &mut impl frontend::HeapVtableM,
    dtor: &mut impl FnMut(&mut [u8], u32, u32),
) -> u32 {
    let rc = validate_handle(stream.is_some(), b, 0x47de, 12);
    if rc >= 0x80000000 {
        return rc;
    }
    let base = stream.unwrap();
    if mem[base as usize] != 0 {
        return 0x88f02007;
    }
    let state = mem_word(mem, base + 4);
    let position = mem_word(mem, base + 8);
    let available = mem_word(mem, state + 44).wrapping_sub(position);
    let (position, rc) = if bytes_read > available {
        (position.wrapping_add(available), 0x88f02007)
    } else {
        (position.wrapping_add(bytes_read), rc)
    };
    mem_store(mem, base + 8, position);
    if position > mem_word(mem, state + 48) {
        mem_store(mem, state + 48, position);
    }
    compact_stream_buffer_m(mem, state, 0, vt, dtor);
    rc
}

pub fn write_direct_start_m(
    mem: &mut [u8],
    object: Option<u32>,
    tag: u32,
    out: Option<u32>,
    count: u32,
    vt: &mut impl frontend::HeapVtableM,
    dtor: &mut impl FnMut(&mut [u8], u32, u32),
) -> u32 {
    let rc = validate_handle(object.is_some(), tag, 0x47de, 12);
    if rc >= 0x80000000 {
        return rc;
    }
    let stream = object.unwrap();
    let rc = 0x88f02007u32;
    let Some(slot) = out else {
        return rc;
    };
    if count == 0 {
        mem_store(mem, slot, 0);
        return rc;
    }
    let kind = mem[stream as usize];
    if kind != 1 && kind != 2 {
        mem_store(mem, slot, 0);
        return rc;
    }
    let state = mem_word(mem, stream + 4);
    let written = mem_word(mem, state + 44);
    let base = mem_word(mem, state + 56);
    if written < base {
        mem_store(mem, slot, 0);
        return 0x88f02005;
    }
    let heap = mem_word(mem, mem_word(mem, state + 4) + 4);
    let need0 = written.wrapping_sub(base).wrapping_add(count);
    if mem_word(mem, state + 64) < need0 {
        compact_stream_buffer_m(
            mem,
            state,
            need0.wrapping_sub(mem_word(mem, state + 64)),
            vt,
            dtor,
        );
        let need = mem_word(mem, state + 44)
            .wrapping_sub(mem_word(mem, state + 56))
            .wrapping_add(count);
        if mem_word(mem, state + 64) < need {
            let maximum = mem_word(mem, mem_word(mem, state) + 16);
            let need = if need > maximum { need } else { maximum };
            let buffer = mem_word(mem, state + 60);
            let grown = if buffer != 0 {
                heap_realloc_m(mem, nz(heap), buffer, need, vt)
            } else {
                heap_alloc_m(mem, nz(heap), need, vt)
            };
            if grown == 0 {
                mem_store(mem, slot, 0);
                return 0x88f0200a;
            }
            mem_store(mem, state + 60, grown);
            mem_store(mem, state + 64, need);
        }
    }
    let at = mem_word(mem, state + 60)
        .wrapping_add(mem_word(mem, state + 44).wrapping_sub(mem_word(mem, state + 56)));
    mem_store(mem, slot, at);
    fill_bytes(mem, at, 0, count);
    0
}

pub fn write_direct_end_m(
    mem: &mut [u8],
    object: Option<u32>,
    tag: u32,
    count: u32,
    vt: &mut impl frontend::HeapVtableM,
    copy: &mut impl FnMut(&mut [u8], u32, u32, u32) -> u32,
    dtor: &mut impl FnMut(&mut [u8], u32, u32),
) -> u32 {
    let rc = validate_handle(object.is_some(), tag, 0x47de, 12);
    if rc >= 0x80000000 {
        return rc;
    }
    let stream = object.unwrap();
    let kind = mem[stream as usize];
    if kind != 1 && kind != 2 {
        return 0x88f02007;
    }
    let state = mem_word(mem, stream + 4);
    let written = mem_word(mem, state + 44);
    let base_pos = mem_word(mem, state + 56);
    if written < base_pos {
        return 0x88f02005;
    }
    let heap = mem_word(mem, mem_word(mem, state + 4) + 4);
    let buffer = mem_word(mem, state + 60);
    let base = written.wrapping_sub(base_pos);
    let need = base.wrapping_add(count);
    if mem_word(mem, state + 64) < need {
        return 0x88f02007;
    }
    if mem_word(mem, state + 32) != 0 {
        let stride = mem_half(mem, state + 28) as u32;
        let mut offset = 0u32;
        loop {
            if count <= offset {
                break;
            }
            let at = buffer.wrapping_add(base.wrapping_add(offset));
            if copy(mem, at, heap, at) == 0 {
                if mem_word(mem, state + 36) != 0 {
                    let mut done = 0u32;
                    while done < offset {
                        dtor(mem, buffer.wrapping_add(base.wrapping_add(done)), heap);
                        done = done.wrapping_add(stride);
                    }
                }
                return 0x88f0200a;
            }
            offset = offset.wrapping_add(stride);
        }
    }
    mem_store(mem, state + 44, written.wrapping_add(count));
    let old_cap = mem_word(mem, state + 64);
    if mem_word(mem, mem_word(mem, state) + 24) <= old_cap.wrapping_sub(need) {
        mem_store(mem, state + 64, need);
        if need == 0 {
            heap_release_m(mem, nz(heap), nz(buffer), vt);
            mem_store(mem, state + 60, 0);
        } else {
            let grown = heap_realloc_m(mem, nz(heap), buffer, need, vt);
            if grown != 0 {
                mem_store(mem, state + 60, grown);
            }
        }
    }
    compact_stream_buffer_m(mem, state, 0, vt, dtor);
    0
}

use frontend::{
    divmod_u32, element_at, fill_bytes, heap_alloc_m, heap_realloc_m, heap_release_m, move_bytes,
    validate_handle, ElementTable,
};
fn mem_word(m: &[u8], p: u32) -> u32 {
    u32::from_le_bytes(m[p as usize..p as usize + 4].try_into().unwrap())
}
fn mem_half(m: &[u8], p: u32) -> u16 {
    u16::from_le_bytes(m[p as usize..p as usize + 2].try_into().unwrap())
}
fn mem_store(m: &mut [u8], p: u32, v: u32) {
    m[p as usize..p as usize + 4].copy_from_slice(&v.to_le_bytes());
}
fn nz(p: u32) -> Option<u32> {
    (p != 0).then_some(p)
}
struct StreamHeap<'a, 'h, H> {
    host: &'a core::cell::RefCell<&'h mut H>,
    heap: u32,
}
impl<H: Calls> frontend::HeapVtableM for StreamHeap<'_, '_, H> {
    fn alloc_at_m(&mut self, o: u32, n: u32, b: &mut [u8]) -> u32 {
        let mut m = Memory { base: 0, bytes: b };
        let f = m.word(m.word(self.heap));
        self.host.borrow_mut().call(&mut m, f, &[o, n])
    }
    fn realloc_at_m(&mut self, o: u32, p: u32, n: u32, b: &mut [u8]) -> u32 {
        let mut m = Memory { base: 0, bytes: b };
        let f = m.word(m.word(self.heap) + 8);
        self.host.borrow_mut().call(&mut m, f, &[o, p, n])
    }
    fn release_at_m(&mut self, o: u32, p: u32, b: &mut [u8]) {
        let mut m = Memory { base: 0, bytes: b };
        let f = m.word(m.word(self.heap) + 12);
        self.host.borrow_mut().call(&mut m, f, &[o, p]);
    }
}
fn stream_call(m: &mut Memory<'_>, f: u32, a: &[u32], h: &mut impl Calls) -> u32 {
    if f == 0x14fe9c {
        return frontend::read_direct_start(m.bytes, nz(a[0]), a[1], nz(a[2]), nz(a[3]));
    }
    if f == 0x15140c {
        let r = validate_handle(a[0] != 0, a[1], 0x47de, 12);
        if r >= 0x80000000 {
            return r;
        }
        let state = m.word(a[0] + 4);
        let v = frontend::StreamPosition {
            kind: m.byte(a[0]),
            read: m.word(a[0] + 8),
            written: m.word(state + 44),
            eof: m.byte(state + 68),
        };
        let (mut remaining, mut ended) = (0, 0);
        let r = v.remaining(
            r,
            nz(a[2]).map(|_| &mut remaining),
            nz(a[3]).map(|_| &mut ended),
        );
        if a[2] != 0 {
            m.put_word(a[2], remaining);
        }
        if a[3] != 0 {
            m.put_byte(a[3], ended);
        }
        return r;
    }
    let p = a[0];
    let state = m.word(p + 4);
    let heap = m.word(m.word(state + 4) + 4);
    let copy_entry = m.word(state + 32);
    let destroy_entry = m.word(state + 36);
    let cell = core::cell::RefCell::new(h);
    let mut vt = StreamHeap { host: &cell, heap };
    let mut dtor = |b: &mut [u8], a: u32, c: u32| {
        let mut m = Memory { base: 0, bytes: b };
        cell.borrow_mut().call(&mut m, destroy_entry, &[a, c]);
    };
    match f {
        0x15017c => read_direct_end_m(m.bytes, nz(p), a[1], a[2], &mut vt, &mut dtor),
        0x150878 => write_direct_start_m(m.bytes, nz(p), a[1], nz(a[2]), a[3], &mut vt, &mut dtor),
        0x150c0c => write_direct_end_m(
            m.bytes,
            nz(p),
            a[1],
            a[2],
            &mut vt,
            &mut |b, a, c, d| {
                let mut m = Memory { base: 0, bytes: b };
                cell.borrow_mut().call(&mut m, copy_entry, &[a, c, d])
            },
            &mut dtor,
        ),
        _ => panic!("stream adapter {f:x}"),
    }
}

impl crate::selection::arena::Host for LiveCalls<'_> {
    fn call(
        &mut self,
        a: &mut crate::selection::arena::Arena<'_>,
        f: u32,
        args: &[u32],
    ) -> (u32, u32) {
        let mut m = Memory {
            base: a.base,
            bytes: a.bytes,
        };
        (Calls::call(self, &mut m, f, args), 0)
    }
}
fn load_i16<const N: usize>(m: &Memory<'_>, p: u32) -> [i16; N] {
    core::array::from_fn(|i| m.half(p + i as u32 * 2) as i16)
}
fn load_i32<const N: usize>(m: &Memory<'_>, p: u32) -> [i32; N] {
    core::array::from_fn(|i| m.word(p + i as u32 * 4) as i32)
}
fn store_i16(m: &mut Memory<'_>, p: u32, x: &[i16]) {
    for (i, v) in x.iter().enumerate() {
        m.put_half(p + i as u32 * 2, *v as u16);
    }
}
fn store_i32(m: &mut Memory<'_>, p: u32, x: &[i32]) {
    for (i, v) in x.iter().enumerate() {
        m.put_word(p + i as u32 * 4, *v as u32);
    }
}
fn numeric_call(m: &mut Memory<'_>, f: u32, a: &[u32]) -> u32 {
    let p = a[0];
    match f {
        0x2dbb2c => {
            let n = m.word(p + 0x60) as usize;
            let input = load_i32::<42>(m, a[1]);
            let mut output = load_i32::<42>(m, a[2]);
            crate::selection::prediction_coefficients(
                &input,
                &mut output,
                n,
                m.word(p + 0x1b0) as i32,
            )
            .unwrap();
            store_i32(m, a[2], &output[..=n]);
        }
        0x2dca78 => {
            let ptr = m.word(p + 0x10c);
            let n = m.word(p + 0x110) as usize;
            let clear = m.word(p + 0x114) as usize;
            let mut history = load_i16::<64>(m, ptr);
            let weights = load_i32::<8>(m, 0x9c8830);
            let mut state = crate::selection::PredictionSmoother {
                limit: m.word(p + 8),
                scale: m.word(p + 0x2c),
                minimum_count: m.byte(p + 0x32),
                maximum_count: m.byte(p + 0x118),
                result: m.half(p + 0x104),
            };
            state
                .update(a[1] as i32, &mut history, n, clear, &weights)
                .unwrap();
            m.put_half(p + 0x104, state.result);
            store_i16(m, ptr, &history[..n.max(clear)]);
        }
        0x2dc4cc => {
            let order = m.word(p + 0x60) as i32;
            let n = a[3] as i32;
            assert!((0..=2048).contains(&n));
            let mut memory = load_i32::<2048>(m, m.word(p + 0x18c));
            let coefficients = load_i16::<32>(m, m.word(p + 0x1ac));
            let gains = load_i16::<4>(m, m.word(p + 0x1a8));
            let mut current = load_i32::<42>(m, m.word(p + 0x44));
            let step = load_i32::<42>(m, m.word(p + 0x6c));
            let mut params = load_i16::<42>(m, m.word(p + 0x54));
            let table = load_i32::<256>(m, 0x9c832c);
            let input = load_i16::<2048>(m, a[1]);
            let mut output = [0i16; 2048];
            let mut filter = crate::selection::PredictionFilter {
                order,
                memory: &mut memory,
                coefficients: &coefficients,
                gains: &gains,
                alpha: m.half(p + 0x1b8) as i16,
                beta: m.half(p + 0x1ba) as i16,
                count: m.byte(p + 0x1bc),
                index: m.byte(p + 0x1bd),
                offsets: core::array::from_fn(|i| m.half(p + 0x1be + i as u32 * 2)),
            };
            crate::selection::advance_prediction(
                &mut filter,
                &mut current,
                &step,
                &mut params,
                order,
                &table,
                &input[..n as usize],
                &mut output[..n as usize],
            )
            .unwrap();
            store_i32(m, m.word(p + 0x18c), &memory[..m.word(p + 0x194) as usize]);
            store_i32(m, m.word(p + 0x44), &current[..order.max(11) as usize + 1]);
            store_i16(m, m.word(p + 0x54), &params[..order.max(11) as usize + 1]);
            store_i16(m, a[2], &output[..n as usize]);
        }
        0x2dcb84 => {
            let band_count = m.word(p + 0x150) as usize;
            assert!(band_count <= 32);
            let samples: [[i16; 512]; 32] = core::array::from_fn(|i| {
                if i < band_count {
                    load_i16(m, m.word(m.word(p + 0x154) + i as u32 * 16))
                } else {
                    [0; 512]
                }
            });
            let filters: [[i16; 128]; 32] = core::array::from_fn(|i| {
                if i < band_count {
                    load_i16(m, m.word(m.word(p + 0x154) + i as u32 * 16 + 4))
                } else {
                    [0; 128]
                }
            });
            let bands: [crate::selection::VoiceBand<'_>; 32] = core::array::from_fn(|i| {
                let b = m.word(p + 0x154) + i as u32 * 16;
                crate::selection::VoiceBand {
                    samples: &samples[i],
                    filter: &filters[i],
                    threshold: if i < band_count { m.word(b + 8) } else { 0 },
                    gain: if i < band_count { m.half(b + 12) } else { 0 },
                    noise_gain: if i < band_count { m.half(b + 14) } else { 0 },
                }
            });
            let mut seed = m.word(p + 0x34);
            let mut cached_samples = load_i16::<2048>(m, m.word(p + 0x17c));
            let mut noise = load_i16::<2048>(m, m.word(p + 0x9c));
            let mut overlap = load_i16::<2048>(m, m.word(p + 0xbc));
            let mut history = load_i16::<2048>(m, m.word(p + 0xcc));
            let mut output = load_i16::<2048>(m, m.word(a[3] + 4));
            let mut v = crate::selection::VoiceSynthesis {
                frequency_scale: m.word(p + 0xc),
                gain: m.word(p + 0x18) as i32,
                noise_gain: m.word(p + 0x1c) as i32,
                seed: &mut seed,
                sample_count: m.half(p + 0x158) as i16,
                filter_length: m.half(p + 0x15c) as i16,
                cached_gain: m.half(p + 0x164) as i16,
                cached_length: m.word(p + 0x168) as i32,
                cached_band: m.word(p + 0x16c) as i32,
                position: m.word(p + 0x170) as i32,
                step: m.word(p + 0x174) as i32,
                overlap_count: m.word(p + 0xc0) as i32,
                overlap_capacity: m.word(p + 0xc4) as i32,
                bands: &bands[..band_count],
                cached_samples: &mut cached_samples,
                noise: &mut noise,
                overlap: &mut overlap,
                history: &mut history,
            };
            v.synthesize(
                a[1] as i32,
                a[2] as i32,
                &mut output,
                m.word(a[3] + 8) as i32,
            )
            .unwrap();
            m.put_half(p + 0x164, v.cached_gain as u16);
            m.put_word(p + 0x168, v.cached_length as u32);
            m.put_word(p + 0x16c, v.cached_band as u32);
            m.put_word(p + 0x170, v.position as u32);
            m.put_word(p + 0xc0, v.overlap_count as u32);
            drop(v);
            m.put_word(p + 0x34, seed);
            store_i16(
                m,
                m.word(p + 0x17c),
                &cached_samples[..m.word(p + 0x184) as usize],
            );
            store_i16(m, m.word(p + 0x9c), &noise[..m.word(p + 0xa4) as usize]);
            store_i16(m, m.word(p + 0xbc), &overlap[..m.word(p + 0xc4) as usize]);
            store_i16(m, m.word(p + 0xcc), &history[..m.word(p + 0xd4) as usize]);
            store_i16(
                m,
                m.word(a[3] + 4),
                &output[..(a[1] as usize)
                    .max(m.half(p + 0x158) as usize)
                    .max(m.word(a[3] + 8) as usize)],
            );
        }
        _ => unreachable!(),
    }
    0
}

fn pump_stream(
    m: &mut Memory<'_>,
    p: u32,
    slot: u32,
    method: u32,
    tail: &[u32],
    h: &mut impl Calls,
) -> u32 {
    let mut a = [0; 5];
    a[0] = m.word(p + slot);
    a[1] = m.word(p + slot + 4);
    a[2..2 + tail.len()].copy_from_slice(tail);
    h.call(m, m.word(m.word(p + 16) + method), &a[..2 + tail.len()])
}
/// fn_22a0a8, live synthesis pump. Scratch is 256 disjoint bytes, restored.
pub fn fn_22a0a8_m(
    m: &mut Memory<'_>,
    p: u32,
    tag: u32,
    state: u32,
    s: u32,
    h: &mut impl Calls,
) -> u32 {
    let saved = core::array::from_fn::<_, 256, _>(|i| m.byte(s + i as u32));
    let r = pump_inner(m, p, tag, state, s, h);
    for (i, b) in saved.into_iter().enumerate() {
        m.put_byte(s + i as u32, b);
    }
    r
}
fn pump_inner(m: &mut Memory<'_>, p: u32, tag: u32, state: u32, s: u32, h: &mut impl Calls) -> u32 {
    if validate_handle(p != 0, tag, 0x4e7, 0x70) >= 0x80000000 {
        return 0x84e02008;
    }
    m.put_byte(state, 1);
    m.put_word(s, 0);
    m.put_byte(s + 4, 0);
    let mut result = pump_stream(m, p, 0x40, 0x48, &[s, s + 4], h);
    if result >= 0x80000000 {
        return result;
    }
    if m.word(s) == 0 {
        if m.byte(s + 4) == 0 {
            m.put_byte(state, 0);
            return result;
        }
        // Drained with pending markers (last pump iteration): close the
        // audio output, clear its handles, read pending markers, copy
        // them to the marker output with the run's sample stamps, commit
        // and consume them if any, then close and clear the marker
        // output. Mirrors ARM 0x22a1d4-0x22a41c (src/fn_228780.c): the
        // marker-output-absent early-out precedes the read, and the
        // write is followed by the memcpy, the sample/reserved stamping,
        // the 0x3c commit and the 0x30 consume from the main path.
        if m.word(p + 0x48) != 0 {
            h.call(
                m,
                m.word(m.word(p + 16) + 0x24),
                &[m.word(p + 0x48), m.word(p + 0x4c)],
            );
            for i in 0..8 {
                m.put_byte(p + 0x48 + i as u32, m.byte(0x8d7660 + i as u32));
            }
        }
        if m.word(p + 0x58) == 0 {
            return 0;
        }
        m.put_word(s + 32, 0);
        m.put_word(s + 36, 0);
        result = h.call(
            m,
            m.word(m.word(p + 16) + 0x2c),
            &[m.word(p + 0x50), m.word(p + 0x54), s + 32, s + 36],
        );
        if result >= 0x80000000 {
            return result;
        }
        if m.word(s + 36) != 0 {
            let pending = m.word(s + 32);
            result = h.call(
                m,
                m.word(m.word(p + 16) + 0x38),
                &[m.word(p + 0x58), m.word(p + 0x5c), s + 32, s + 36],
            );
            if result >= 0x80000000 {
                return result;
            }
            let staged = m.word(s + 32);
            let nbytes = m.word(s + 36);
            frontend::move_bytes(m.bytes, staged, pending, nbytes);
            let mut i = 0u32;
            while i < nbytes {
                m.put_word(staged + i + 12, m.word(p + 0x6c));
                m.put_word(staged + i + 16, 0);
                i += 24;
            }
            result = pump_stream(m, p, 0x58, 0x3c, &[nbytes], h);
            if result >= 0x80000000 {
                return result;
            }
            result = pump_stream(m, p, 0x50, 0x30, &[nbytes], h);
            if result >= 0x80000000 {
                return result;
            }
        }
        if m.word(p + 0x58) != 0 {
            h.call(
                m,
                m.word(m.word(p + 16) + 0x24),
                &[m.word(p + 0x58), m.word(p + 0x5c)],
            );
            for i in 0..8 {
                m.put_byte(p + 0x58 + i as u32, m.byte(0x8d7660 + i as u32));
            }
        }
        return 0;
    }
    result = pump_stream(m, p, 0x40, 0x2c, &[s + 8, s], h);
    if result >= 0x80000000 {
        return result;
    }
    m.put_word(s + 12, 0);
    m.put_word(s + 16, 0);
    result = pump_stream(m, p, 0x50, 0x2c, &[s + 12, s + 16], h);
    if result >= 0x80000000 || m.word(s) < 4 {
        return result;
    }
    result = pump_stream(m, p, 0x48, 0x38, &[s + 20, m.word(p + 20) * 2], h);
    if result >= 0x80000000 {
        return result;
    }
    if m.word(s + 16) != 0 {
        result = pump_stream(m, p, 0x58, 0x38, &[s + 24, m.word(s + 16)], h);
        if result >= 0x80000000 {
            return result;
        }
    }
    result = h.call(m, 0x176f2c, &[m.word(p + 12)]);
    if result >= 0x80000000 {
        return result;
    }
    let frames = m.word(m.word(s + 8));
    let mut position = m.word(frames + 0x434);
    let mut pp = m.word(frames + 0x438) as usize;
    let mut ep = m.word(frames + 0x43c) as usize;
    let mut samples = 0u32;
    let mut full = false;
    let mut marker_index = 0u32;
    let mut marker_count = m.word(s + 16);
    if marker_count != 0 && m.word(s + 12) != 0 && m.word(s + 24) != 0 {
        frontend::move_bytes(m.bytes, m.word(s + 24), m.word(s + 12), marker_count);
        marker_count /= 24;
    }
    if m.word(p + 0x60) != frames {
        m.put_word(p + 0x60, frames);
        m.put_word(p + 0x64, 0);
    }
    let engine = m.word(p + 0x28);
    let order = m.word(engine + 0x60) as usize;
    while m.word(p + 0x18).wrapping_add(samples) <= m.word(p + 0x14)
        && (m.word(p + 0x64) as i32) < m.word(frames + 0x28c) as i32
    {
        while marker_index < marker_count {
            let mark = m.word(s + 24) + marker_index * 24;
            let typ = m.half(mark);
            if typ != 0x1a
                && typ != 0x15
                && m.word(mark + 12) > m.word(engine + 0xd8).wrapping_mul(m.word(p + 0x68))
            {
                break;
            }
            // Parameter markers (src/fn_228780.c pump loop): unlock, set
            // the named voice parameter to the marker value, relock. The
            // names are the pump's ROM literals (0x22a86c "volume" for
            // 0x18, 0x22a874 "rate" for 0x12, 0x22a87c "pitch" for
            // 0x10, each the target of the ARM `add r1, pc` at its call
            // site). Like the C code the call results are not consulted.
            match typ {
                0x18 | 0x12 | 0x10 => {
                    let name = match typ {
                        0x18 => 0x22a86c,
                        0x12 => 0x22a874,
                        _ => 0x22a87c,
                    };
                    h.call(m, 0x176f5c, &[m.word(p + 12)]);
                    h.call(
                        m,
                        0x1758ac,
                        &[m.word(m.word(p + 8) + 20), name, m.word(mark + 20)],
                    );
                    h.call(m, 0x176f2c, &[m.word(p + 12)]);
                }
                _ => {}
            }
            m.put_word(mark + 12, m.word(p + 0x6c));
            m.put_word(mark + 16, 0);
            marker_index += 1;
        }
        let flags = m.byte(m.word(frames + 0x430) + position);
        let stereo = m.byte(p + 0x26) == 0;
        let left = load_i32::<4096>(m, m.word(frames + 0x2e0));
        let right = load_i32::<4096>(m, m.word(frames + 0x370));
        let table = load_i32::<320>(m, 0x91736c);
        let mut pe = [0u16; 2];
        crate::selection::dequantize(
            flags, stereo, &left, &mut pp, &right, &mut ep, &table, &mut pe,
        )
        .unwrap();
        let source = m.word(m.word(frames + 0x298) + m.word(p + 0x64) * 4);
        for i in 0..=order {
            let mut c = m.word(source + i as u32 * 4);
            let adjust = match m.word(p + 0x1c) {
                16000 => 0x8fa5ec,
                22050 => 0x8fa650,
                _ => 0,
            };
            if adjust != 0 {
                c = c.wrapping_add(m.word(adjust + i as u32 * 4));
            }
            m.put_word(s + 64 + i as u32 * 4, c);
        }
        result = if h.call(
            m,
            0x2a977c,
            &[engine, pe[0] as u32, s + 64, pe[1] as u32, s + 28],
        ) != 0
        {
            0x84e02000
        } else {
            0
        };
        let produced = m.word(s + 28) as i32;
        if produced > 0 {
            m.put_word(p + 0x6c, m.word(p + 0x6c).wrapping_add(produced as u32));
            m.put_word(p + 0x68, m.word(p + 0x68).wrapping_add(1));
            if m.word(p + 0x14) < samples.wrapping_add(produced as u32) {
                if samples > 0 {
                    result = pump_stream(m, p, 0x48, 0x3c, &[samples * 2], h);
                }
                samples = 0;
                if result < 0x80000000 {
                    result = pump_stream(m, p, 0x48, 0x38, &[s + 20, produced as u32 * 2], h);
                }
                full = true;
            }
            if result < 0x80000000 {
                let dst = m.word(s + 20) + samples * 2;
                frontend::move_bytes(m.bytes, dst, m.word(engine + 0xac), produced as u32 * 2);
                let mut audio = load_i16::<2048>(m, dst);
                let gains: [u16; 101] = core::array::from_fn(|i| m.half(0x917ae6 + i as u32 * 2));
                crate::prosody_rules::apply_gain(
                    &mut audio[..produced as usize],
                    m.half(p + 0x24) as u32,
                    &gains,
                )
                .unwrap();
                store_i16(m, dst, &audio[..produced as usize]);
                samples += produced as u32;
            }
        }
        m.put_word(frames + 0x438, pp as u32);
        position += 1;
        m.put_word(frames + 0x43c, ep as u32);
        m.put_word(frames + 0x434, position);
        m.put_word(p + 0x64, m.word(p + 0x64) + 1);
        if result >= 0x80000000 || full {
            break;
        }
    }
    if samples > 0 && result < 0x80000000 {
        result = pump_stream(m, p, 0x48, 0x3c, &[samples * 2], h);
    }
    if result < 0x80000000 && marker_index != 0 {
        result = pump_stream(m, p, 0x58, 0x3c, &[marker_index * 24], h);
    }
    if result < 0x80000000 {
        if (m.word(p + 0x64) as i32) < m.word(frames + 0x28c) as i32 {
            m.put_byte(state, 2);
        } else {
            result = pump_stream(m, p, 0x40, 0x30, &[4], h);
            if result < 0x80000000 {
                m.put_word(p + 0x60, 0);
                m.put_byte(state, if m.word(s) > 4 { 2 } else { 1 });
            }
        }
        if result < 0x80000000 && marker_index != 0 {
            result = pump_stream(m, p, 0x50, 0x30, &[marker_index * 24], h);
        }
    }
    let status = h.call(m, 0x176f5c, &[m.word(p + 12)]);
    if status >= 0x80000000 && result < 0x80000000 {
        result = status;
    }
    result
}

/// fn_196e28 with live destructor.
pub fn remove_element_m(m: &mut Memory<'_>, p: u32, i: u32, h: &mut impl Calls) -> u32 {
    if p == 0 || i >= m.word(p + 20) {
        return 0;
    }
    let n = m.word(p + 20);
    let size = m.half(p + 4) as u32;
    let dst = m.word(p) + i * size;
    if m.word(p + 12) != 0 {
        h.call(m, m.word(p + 12), &[dst, m.word(p + 28)]);
    }
    if n - i - 1 != 0 {
        frontend::move_bytes(m.bytes, dst, dst + size, (n - i - 1) * size);
    }
    m.put_word(p + 20, n - 1);
    1
}
/// fn_14f918 with reentrant destruction.
pub fn close_stream_m(m: &mut Memory<'_>, p: u32, tag: u32, h: &mut impl Calls) -> u32 {
    if validate_handle(p != 0, tag, 0x47de, 12) >= 0x80000000 {
        return 0x81706008;
    }
    let state = m.word(p + 4);
    let ctx = m.word(state + 4);
    if m.byte(p) == 0 {
        let list = m.word(state + 12);
        let mut i = 0;
        while i < m.word(list + 20) {
            let item = m.word(list) + i * m.half(list + 4) as u32;
            if m.word(item) == p {
                remove_element_m(m, list, i, h);
                break;
            }
            i += 1;
        }
    } else {
        m.put_word(state + 8, 0);
        m.put_byte(state + 68, 1);
    }
    h.call(m, 0x14e0b4, &[state, 0]);
    h.call(m, 0x196428, &[m.word(ctx + 4), p]);
    0
}
fn rule_stream(
    m: &mut Memory<'_>,
    p: u32,
    slot: u32,
    method: u32,
    tail: &[u32],
    h: &mut impl Calls,
) -> u32 {
    let mut a = [0; 6];
    a[0] = m.word(p + slot);
    a[1] = m.word(p + slot + 4);
    a[2..2 + tail.len()].copy_from_slice(tail);
    h.call(m, m.word(m.word(p + 32) + method), &a[..2 + tail.len()])
}
/// fn_22b518. Scratch is 64 disjoint bytes, restored on return.
pub fn fn_22b518_m(
    m: &mut Memory<'_>,
    p: u32,
    tag: u32,
    out: u32,
    lang_key: u32,
    s: u32,
    h: &mut impl Calls,
) -> u32 {
    let saved = core::array::from_fn::<_, 64, _>(|i| m.byte(s + i as u32));
    let rc = process_inner(m, p, tag, out, lang_key, s, h);
    for (i, b) in saved.into_iter().enumerate() {
        m.put_byte(s + i as u32, b);
    }
    rc
}
fn process_inner(
    m: &mut Memory<'_>,
    p: u32,
    tag: u32,
    out: u32,
    lang_key: u32,
    s: u32,
    h: &mut impl Calls,
) -> u32 {
    let r = validate_handle(p != 0, tag, 0x10d, 0x38);
    if r >= 0x80000000 {
        return r;
    }
    m.put_word(s, 0);
    let r = h.call(m, 0x19611c, &[m.word(p), m.word(p + 4), s]);
    if r >= 0x80000000 {
        return 0x88602007;
    }
    let ctx = m.word(s);
    let heap = m.word(ctx + 4);
    assert!(
        m.word(ctx + 16) == 0 || m.byte(m.word(ctx + 16) + 64) == 0,
        "active rule logging"
    );
    if m.word(p + 36) == 0 || m.word(p + 44) == 0 {
        m.put_byte(out, 1);
        return 0;
    }
    m.put_byte(out, 0);
    m.put_word(s + 4, 0);
    m.put_byte(s + 8, 0);
    let mut rc = rule_stream(m, p, 36, 0x48, &[s + 4, s + 8], h);
    if rc >= 0x80000000 {
        return rc;
    }
    if m.byte(s + 8) == 1 && m.word(p + 44) == 0 {
        m.put_byte(out, 1);
        return 0;
    }
    if m.byte(s + 8) == 0 && m.byte(p + 52) == 0 {
        return 0;
    }
    assert!(
        m.word(m.word(p + 24) + 20) != 0,
        "empty rule list needs copy-stream binding"
    );
    rc = rule_stream(m, p, 36, 0x2c, &[s + 12, s + 4], h);
    if rc >= 0x80000000 {
        return rc;
    }
    let units = m.word(s + 4) >> 1;
    let mut cur = h.call(m, 0x196394, &[heap, units * 4 + 1]);
    if cur == 0 {
        return 0x8860200a;
    }
    let mut prev = 0;
    let mut extra = 0;
    rc = h.call(
        m,
        0x17bdc4,
        &[m.word(s + 12), units, 0, cur, units * 4, s + 16, 0],
    );
    if rc < 0x80000000 {
        m.put_byte(cur + m.word(s + 16), 0);
        rc = rule_stream(m, p, 36, 0x30, &[units * 2], h);
    }
    if rc < 0x80000000 {
        rc = h.call(m, 0x175a1c, &[m.word(ctx + 20), lang_key, s + 20]);
    }
    if rc < 0x80000000 {
        for pass in 0..3 {
            if pass == 1 {
                m.put_word(s + 24, 0);
                let d = 0x8fa74c;
                let ok = h.call(
                    m,
                    0x196a64,
                    &[
                        heap,
                        m.word(d + 0xa4),
                        m.word(d + 0xa8),
                        m.word(d + 0xac),
                        1,
                        s + 24,
                    ],
                );
                extra = m.word(s + 24);
                if ok == 0 {
                    break;
                }
                let list = m.word(p + 24);
                let mut k = 0;
                while k < m.word(list + 20) {
                    let item = m.word(list) + k * m.half(list + 4) as u32;
                    if m.byte(item + 28) != 0 && h.call(m, 0x2ab68c, &[item, m.word(s + 20)]) == 1 {
                        if h.call(m, 0x196db0, &[extra, item]) != 1 {
                            break;
                        }
                    }
                    k += 1;
                }
            }
            let list = if pass == 1 { extra } else { m.word(p + 24) };
            loop {
                let mut n = m.word(list + 20);
                let mut restart = false;
                while n != 0 {
                    let item = m.word(list) + (n - 1) * m.half(list + 4) as u32;
                    let eligible = if pass == 1 {
                        m.byte(item + 22) != 1
                    } else {
                        m.byte(item + 28) == 0
                            && m.byte(item + 21) == if pass == 0 { 1 } else { 0 }
                            && h.call(m, 0x2ab68c, &[item, m.word(s + 20)]) == 1
                    };
                    if eligible {
                        core::mem::swap(&mut cur, &mut prev);
                        m.put_word(s + 28, cur);
                        rc = h.call(m, 0x2ac0ac, &[item, prev, s + 28]);
                        cur = m.word(s + 28);
                        if rc >= 0x80000000 {
                            break;
                        }
                        if pass == 1
                            && ((prev != 0
                                && (cur == 0
                                    || frontend::compare_str(
                                        &m.bytes[m.offset(prev)..],
                                        &m.bytes[m.offset(cur)..],
                                    ) != 0))
                                || (prev == 0 && cur != 0))
                        {
                            m.put_byte(item + 22, 1);
                            restart = true;
                            break;
                        }
                    }
                    n -= 1;
                }
                if !restart || rc >= 0x80000000 {
                    break;
                }
            }
            if rc >= 0x80000000 {
                break;
            }
        }
    }
    let len = if cur == 0 {
        0
    } else {
        term(m, cur).len() as u32
    };
    if rc < 0x80000000 && len != 0 {
        rc = rule_stream(m, p, 44, 0x38, &[s + 32, len * 2], h);
        if rc < 0x80000000 {
            rc = h.call(m, 0x17bb44, &[cur, len, 0, m.word(s + 32), len, s + 16, 0]);
            let len = if rc >= 0x80000000 { 0 } else { m.word(s + 16) };
            let r = rule_stream(m, p, 44, 0x3c, &[len * 2], h);
            if r >= 0x80000000 && rc < 0x80000000 {
                rc = r;
            }
        }
    }
    if extra != 0 {
        while m.word(extra + 20) != 0 {
            h.call(m, 0x196e28, &[extra, m.word(extra + 20) - 1]);
        }
        h.call(m, 0x196c7c, &[extra]);
    }
    if prev != 0 {
        h.call(m, 0x196428, &[heap, prev]);
    }
    if cur != 0 {
        h.call(m, 0x196428, &[heap, cur]);
    }
    if m.byte(p + 52) == 0 {
        let r = rule_stream(m, p, 44, 0x24, &[], h);
        if r >= 0x80000000 {
            rc = r;
        }
        m.put_word(p + 44, 0);
        m.put_word(p + 48, 0);
    }
    m.put_byte(out, 1);
    rc
}

use crate::prosody_rules::{Arena, RuleContext, RuleItem};
pub trait ApplyRuleHostM {
    fn resolve(&mut self, arena: &mut Arena<'_>, h0: u32, h1: u32) -> (u32, Option<RuleContext>);
    fn alloc(&mut self, arena: &mut Arena<'_>, size: u32) -> u32;
    fn realloc(&mut self, arena: &mut Arena<'_>, ptr: u32, size: u32) -> u32;
    fn free(&mut self, arena: &mut Arena<'_>, ptr: u32);
    fn count_rules(&mut self, arena: &mut Arena<'_>, rules: u32) -> u32;
    fn get_rule(&mut self, arena: &mut Arena<'_>, rules: u32, index: u32) -> Option<[u32; 2]>;
    fn apply_indexed(
        &mut self,
        arena: &mut Arena<'_>,
        item: &RuleItem,
        index: u32,
        flag: u32,
        input: u32,
        output: &mut u32,
    ) -> u32;
    fn apply_segment(
        &mut self,
        arena: &mut Arena<'_>,
        engine: u32,
        rule: [u32; 2],
        input: u32,
        output: &mut u32,
    ) -> u32;
    fn logging(&mut self, arena: &mut Arena<'_>) -> bool;
    fn log(&mut self, arena: &mut Arena<'_>, code: u32);
    fn log_error(&mut self, arena: &mut Arena<'_>, code: u32);
}

fn tag_memcmp(arena: &Arena<'_>, offset: usize, tag: &[u8], count: usize) -> bool {
    let bytes = arena.bytes();
    count == 0
        || (offset + count <= bytes.len()
            && tag.len() >= count
            && bytes[offset..offset + count] == tag[..count])
}

fn arena_region_eq(arena: &Arena<'_>, first: usize, second: usize, count: usize) -> bool {
    let bytes = arena.bytes();
    count == 0
        || (first + count <= bytes.len()
            && second + count <= bytes.len()
            && bytes[first..first + count] == bytes[second..second + count])
}

fn arena_strstr(arena: &Arena<'_>, start: usize, needle: &[u8]) -> Option<usize> {
    let bytes = arena.bytes();
    let end = bytes[start..]
        .iter()
        .position(|&b| b == 0)
        .map(|len| start + len)
        .unwrap_or(bytes.len());
    let needle_len = needle.iter().position(|&b| b == 0).unwrap_or(needle.len());
    if needle_len == 0 {
        return Some(start);
    }
    (start..=end.saturating_sub(needle_len))
        .find(|&at| bytes[at..at + needle_len] == needle[..needle_len])
}

/// Tag prefix scanned by `fn_2ac0ac` (`D_91786f`).
const RULE_TAG: &[u8] = b"\x1b\\tn";

/// fn_2ac0ac / EU fn_2ac268 / KR fn_2ac314.
///
/// Applies the rule set to the arena string at `input`, writing the result
/// arena offset to `out`. On the typed path the underlying applier statuses
/// are discarded and `0x88602007` is returned, matching the original.
pub fn apply_rules_m(
    arena: &mut Arena<'_>,
    item: Option<&RuleItem>,
    input: u32,
    out: Option<&mut u32>,
    host: &mut impl ApplyRuleHostM,
) -> u32 {
    let (Some(item), Some(out)) = (item, out) else {
        return 0x88602007;
    };
    if input == 0 {
        return 0x88602007;
    }
    let (status, ctx) = host.resolve(arena, item.handle[0], item.handle[1]);
    if status >= 0x80000000 {
        return 0x88602007;
    }
    let Some(_) = ctx else { return 0x88602007 };
    if item.name == 0 {
        return 0x88602011;
    }
    let mut status = status;
    let mut matches = 1;
    let mut text = host.alloc(arena, arena.len(input) as u32 + 1);
    if text == 0 {
        host.log_error(arena, 0x2afa);
        return 0x8860200a;
    }
    arena.copy_str(text, input);
    let mut previous = 0u32;
    if item.kind != 0 {
        let kind_off = item.kind as usize;
        let kind_len = arena.len(item.kind);
        let kind_is_normal = kind_len == 6 && tag_memcmp(arena, kind_off, b"normal", 6);
        let mut scan = text as usize;
        matches = 0;
        if !tag_memcmp(arena, scan, RULE_TAG, 4) && kind_is_normal {
            matches = 1;
        } else {
            loop {
                let Some(found) = arena_strstr(arena, scan, RULE_TAG) else {
                    break;
                };
                scan = found;
                if tag_memcmp(arena, scan, RULE_TAG, 4) {
                    let mut value = scan + 4;
                    if arena.bytes().get(value) == Some(&b'=') {
                        value += 1;
                        if arena_region_eq(arena, value, kind_off, kind_len)
                            && arena.bytes().get(value + kind_len) == Some(&b'\\')
                        {
                            matches = 1;
                            break;
                        }
                    }
                }
                matches = 0;
                scan += 1;
            }
        }
    }
    if host.logging(arena) {
        host.log(arena, 8);
    }
    if matches != 0 {
        let count = host.count_rules(arena, item.rules);
        if item.kind != 0 {
            for i in 0..count {
                let swap = text;
                text = previous;
                previous = swap;
                if text != 0 {
                    host.free(arena, text);
                    text = 0;
                }
                host.apply_indexed(arena, item, i, 0, previous, &mut text);
            }
            if previous != 0 {
                host.free(arena, previous);
            }
            previous = text;
            text = 0;
            host.apply_indexed(arena, item, 0, 1, previous, &mut text);
            if previous != 0 {
                host.free(arena, previous);
                previous = 0;
            }
        } else {
            let count = host.count_rules(arena, item.rules);
            for i in 0..count {
                let swap = text;
                text = previous;
                previous = swap;
                let Some(rule) = host.get_rule(arena, item.rules, i) else {
                    host.log_error(arena, 0x2b11);
                    if previous != 0 {
                        host.free(arena, previous);
                    }
                    if text != 0 {
                        host.free(arena, text);
                    }
                    host.log(arena, 8);
                    return 0x88602006;
                };
                host.log(arena, 9);
                status = host.apply_segment(arena, item.engine, rule, previous, &mut text);
                host.log(arena, 9);
                if status >= 0x80000000 {
                    if previous != 0 {
                        host.free(arena, previous);
                    }
                    if text != 0 {
                        host.free(arena, text);
                    }
                    host.log(arena, 8);
                    return status;
                }
            }
        }
    }
    if text == 0 {
        host.log_error(arena, 0x2afa);
        status = 0x8860200a;
        if previous != 0 {
            host.free(arena, previous);
        }
        host.log(arena, 8);
        return status;
    }
    let result = if *out == 0 {
        host.alloc(arena, arena.len(text) as u32 + 1)
    } else {
        host.realloc(arena, *out, arena.len(text) as u32 + 1)
    };
    if result == 0 {
        host.log_error(arena, 0x2afa);
        status = 0x8860200a;
        if previous != 0 {
            host.free(arena, previous);
        }
        if text != 0 {
            host.free(arena, text);
        }
        host.log(arena, 8);
        return status;
    }
    *out = result;
    arena.copy_str(result, text);
    if previous != 0 {
        host.free(arena, previous);
    }
    if text != 0 {
        host.free(arena, text);
    }
    host.log(arena, 8);
    status
}

struct ApplyAdapter<'a, H> {
    h: &'a mut H,
    heap: u32,
    s: u32,
}
fn arena_call(h: &mut impl Calls, arena: &mut Arena<'_>, f: u32, a: &[u32]) -> u32 {
    let mut m = Memory {
        base: 0,
        bytes: arena.bytes_mut(),
    };
    h.call(&mut m, f, a)
}
impl<H: Calls> ApplyRuleHostM for ApplyAdapter<'_, H> {
    fn resolve(&mut self, arena: &mut Arena<'_>, a: u32, b: u32) -> (u32, Option<RuleContext>) {
        let r = arena_call(self.h, arena, 0x19611c, &[a, b, self.s]);
        let m = Memory {
            base: 0,
            bytes: arena.bytes_mut(),
        };
        let p = m.word(self.s);
        (
            r,
            if r >= 0x80000000 {
                None
            } else {
                Some(RuleContext {
                    heap: m.word(p + 4),
                    log: m.word(p + 16),
                    params: m.word(p + 20),
                    res: m.word(p + 24),
                })
            },
        )
    }
    fn alloc(&mut self, a: &mut Arena<'_>, n: u32) -> u32 {
        arena_call(self.h, a, 0x196394, &[self.heap, n])
    }
    fn realloc(&mut self, a: &mut Arena<'_>, p: u32, n: u32) -> u32 {
        arena_call(self.h, a, 0x1963e0, &[self.heap, p, n])
    }
    fn free(&mut self, a: &mut Arena<'_>, p: u32) {
        arena_call(self.h, a, 0x196428, &[self.heap, p]);
    }
    fn count_rules(&mut self, a: &mut Arena<'_>, p: u32) -> u32 {
        let m = Memory {
            base: 0,
            bytes: a.bytes_mut(),
        };
        if p == 0 {
            0
        } else {
            m.word(p + 20)
        }
    }
    fn get_rule(&mut self, a: &mut Arena<'_>, p: u32, i: u32) -> Option<[u32; 2]> {
        let m = Memory {
            base: 0,
            bytes: a.bytes_mut(),
        };
        if p == 0 || i >= m.word(p + 20) {
            None
        } else {
            let v = m.word(p) + i * m.half(p + 4) as u32;
            Some([m.word(v), m.word(v + 4)])
        }
    }
    fn apply_indexed(
        &mut self,
        _: &mut Arena<'_>,
        _: &RuleItem,
        _: u32,
        _: u32,
        _: u32,
        _: &mut u32,
    ) -> u32 {
        panic!("typed rule applier needs conversion")
    }
    fn apply_segment(
        &mut self,
        a: &mut Arena<'_>,
        e: u32,
        r: [u32; 2],
        input: u32,
        out: &mut u32,
    ) -> u32 {
        let saved = mem_word(a.bytes(), self.s + 4);
        mem_store(a.bytes_mut(), self.s + 4, *out);
        let rc = arena_call(self.h, a, 0x2aea98, &[e, r[0], r[1], input, self.s + 4]);
        *out = mem_word(a.bytes(), self.s + 4);
        mem_store(a.bytes_mut(), self.s + 4, saved);
        rc
    }
    fn logging(&mut self, _: &mut Arena<'_>) -> bool {
        false
    }
    fn log(&mut self, _: &mut Arena<'_>, _: u32) {}
    fn log_error(&mut self, _: &mut Arena<'_>, code: u32) {
        panic!("rule error {code:x}")
    }
}
fn apply_rules_live(
    m: &mut Memory<'_>,
    p: u32,
    input: u32,
    out: u32,
    s: u32,
    h: &mut impl Calls,
) -> u32 {
    let item = RuleItem {
        handle: [m.word(p), m.word(p + 4)],
        engine: m.word(p + 8),
        name: m.word(p + 12),
        alternate: m.word(p + 16),
        active: m.byte(p + 20),
        in_memory: m.byte(p + 21),
        language: m.word(p + 24),
        kind: m.word(p + 28),
        rules: m.word(p + 32),
        stream: [m.word(p + 36), m.word(p + 40)],
    };
    let mut result = m.word(out);
    let saved = core::array::from_fn::<_, 8, _>(|i| m.byte(s + i as u32));
    let heap = m.word(m.word(p) + 4);
    let mut arena = Arena::new(m.bytes);
    let mut adapter = ApplyAdapter { h, heap, s };
    let rc = apply_rules_m(
        &mut arena,
        Some(&item),
        input,
        Some(&mut result),
        &mut adapter,
    );
    m.put_word(out, result);
    for (i, b) in saved.into_iter().enumerate() {
        m.put_byte(s + i as u32, b);
    }
    rc
}

struct ReplacementAdapter<'a, 'b, H> {
    m: &'a mut Memory<'b>,
    h: &'a mut H,
    heap: u32,
    engine: [u32; 2],
    pattern: u32,
    input: u32,
    ptr: u32,
    s: u32,
}
impl<H: Calls> crate::prosody_rules::ReplacementMatcher for ReplacementAdapter<'_, '_, H> {
    fn resolve(&mut self) -> bool {
        true
    }
    fn reserve(&mut self, n: u32, _existing: bool) -> bool {
        let new = if self.ptr == 0 {
            self.h.call(self.m, 0x196394, &[self.heap, n])
        } else {
            self.h.call(self.m, 0x1963e0, &[self.heap, self.ptr, n])
        };
        if new == 0 {
            false
        } else {
            self.ptr = new;
            if !_existing {
                frontend::fill_bytes(self.m.bytes, new, 0, n);
            }
            true
        }
    }
    fn execute(&mut self, _: &[u8], offset: u32, captures: &mut [i32; 30]) -> i32 {
        pcre_exec_live(
            self.m,
            self.engine,
            self.pattern,
            self.input,
            offset,
            captures,
            self.s,
            self.h,
        )
    }
    fn error(&mut self, code: i32) -> u32 {
        crate::regex::error_status(code)
    }
}
fn replace_live(m: &mut Memory<'_>, a: [u32; 5], s: u32, h: &mut impl Calls) -> u32 {
    let [p, pattern, replacement, input, out] = a;
    if p == 0 || pattern == 0 || input == 0 || out == 0 {
        return 0x80002007;
    }
    let saved = m.word(s);
    let r = h.call(m, 0x19611c, &[m.word(p), m.word(p + 4), s]);
    if r >= 0x80000000 {
        m.put_word(s, saved);
        return 0x80002007;
    }
    let ctx = m.word(s);
    m.put_word(s, saved);
    let mut inp = [0u8; 65536];
    let n = term(m, input).len();
    inp[..n + 1].copy_from_slice(&m.bytes[m.offset(input)..m.offset(input) + n + 1]);
    let mut repl = [0u8; 4096];
    let rn = term(m, replacement).len();
    repl[..rn + 1].copy_from_slice(&m.bytes[m.offset(replacement)..m.offset(replacement) + rn + 1]);
    let old = m.word(out);
    let mut buffer = [0u8; 65536];
    let capacity = if old == 0 {
        0
    } else {
        term(m, old).len() as u32
    };
    if old != 0 {
        buffer[..capacity as usize + 1]
            .copy_from_slice(&m.bytes[m.offset(old)..m.offset(old) + capacity as usize + 1]);
        m.put_word(out, 0);
    }
    let lengths = core::array::from_fn::<_, 64, _>(|i| m.byte(0x917aa6 + i as u32));
    let engine = [m.word(p + 8), m.word(p + 12)];
    let heap = m.word(ctx + 4);
    let mut matcher = ReplacementAdapter {
        m,
        h,
        heap,
        engine,
        pattern,
        input,
        ptr: old,
        s: s + 16,
    };
    let mut result = crate::prosody_rules::ReplacementBuffer {
        bytes: &mut buffer,
        length: 0,
        capacity,
        limit: n as u32,
    };
    let r = crate::prosody_rules::replace_pattern_global(
        true,
        true,
        Some(&inp[..n + 1]),
        Some(&repl[..rn + 1]),
        Some(&mut result),
        old != 0,
        &lengths,
        &mut matcher,
    )
    .unwrap();
    let count = result.length as usize;
    let ptr = matcher.ptr;
    if ptr != 0 {
        let off = matcher.m.offset(ptr);
        matcher.m.bytes[off..off + count].copy_from_slice(&buffer[..count]);
        matcher.m.put_word(out, ptr);
    }
    r
}
struct PcreAdapter<'a, 'b, H> {
    m: &'a mut Memory<'b>,
    h: &'a mut H,
    s: u32,
}
impl<H: Calls> crate::regex::PcreHost for PcreAdapter<'_, '_, H> {
    fn validate(&mut self, p: u32, k: u32, t: u32, n: u32) -> u32 {
        validate_handle(p != 0, k, t, n)
    }
    fn resolve_owner(&mut self, p: u32, k: i32) -> (u32, u32) {
        let saved = self.m.word(self.s);
        let rc = self.h.call(self.m, 0x19611c, &[p, k as u32, self.s]);
        let ctx = self.m.word(self.s);
        self.m.put_word(self.s, saved);
        (rc, ctx)
    }
    fn heap_of(&mut self, p: u32) -> u32 {
        self.m.word(p + 4)
    }
    fn alloc(&mut self, p: u32, n: u32, size: u32) -> Option<u32> {
        nz(self.h.call(self.m, 0x1963b8, &[p, n, size]))
    }
    fn free(&mut self, p: u32, v: u32) {
        self.h.call(self.m, 0x196428, &[p, v]);
    }
}
fn pcre_exec_live(
    m: &mut Memory<'_>,
    engine: [u32; 2],
    pattern: u32,
    input: u32,
    offset: u32,
    captures: &mut [i32; 30],
    s: u32,
    h: &mut impl Calls,
) -> i32 {
    use crate::regex::*;
    let p = engine[0];
    let block_count = m.half(p + 8);
    let per = m.half(p + 10);
    let size = m.half(p + 12);
    let maximum = m.half(p + 14);
    assert!(block_count <= 10 && per <= 10);
    let mut blocks: [FrameBlockState; 10] = core::array::from_fn(|i| {
        if i >= block_count as usize {
            return FrameBlockState::default();
        }
        let b = m.word(m.word(p + 4) + i as u32 * 4);
        if b == 0 {
            return FrameBlockState::default();
        }
        FrameBlockState {
            present: true,
            live: true,
            used: m.word(b + 8),
            count: m.byte(b + 16),
            code_id: m.word(b),
            extra_id: m.word(b + 12),
            entry_id: b,
        }
    });
    let mut tags = [0u8; 100];
    for i in 0..block_count as usize {
        for j in 0..per as usize {
            tags[i * per as usize + j] = m.byte(blocks[i].extra_id + j as u32);
        }
    }
    let code = pattern
        + m.half(pattern + 24) as u32
        + (m.half(pattern + 26) as u32) * (m.half(pattern + 28) as u32);
    let subject_base = input.wrapping_sub(256);
    let mut frames: [MatchFrame; 100] = core::array::from_fn(|i| {
        if i >= block_count as usize * per as usize {
            return MatchFrame::default();
        }
        let ptr = blocks[i / per as usize].code_id + (i % per as usize) as u32 * size as u32;
        decode_frame(m, ptr, &blocks, code, subject_base, size)
    });
    let mut store = FrameStore {
        frames: &mut frames,
        tags: &mut tags,
        blocks: &mut blocks,
        frames_per_block: per,
        frame_size: size,
        max_blocks: maximum,
        block_count,
        current_block: m.half(p + 16),
        highest_block: m.half(p + 18),
        max_frames: m.half(p + 20),
        fresh: crate::regex::MatchFrame::default(),
        heap: m.word(m.word(p) + 4),
        blocks_id: Some(m.word(p + 4)),
    };
    let mut program = [0u8; 8192];
    let pn = m.word(pattern + 4) as usize;
    assert!(pn <= 8192);
    program[..pn].copy_from_slice(&m.bytes[m.offset(pattern)..m.offset(pattern) + pn]);
    let n = term(m, input).len();
    let mut subject = [0u8; 66048];
    subject[256..256 + n + 1].copy_from_slice(&m.bytes[m.offset(input)..m.offset(input) + n + 1]);
    let op = core::array::from_fn(|i| m.byte(0x914092 + i as u32));
    let ctypes = core::array::from_fn(|i| m.byte(0x91426c + i as u32));
    let defaults = core::array::from_fn(|i| m.byte(0x91436c + i as u32));
    let escapes: [i16; 75] = load_i16(m, 0x9140e4);
    let limits = load_i32(m, 0x9147ac);
    let first = load_i32(m, 0x9147c4);
    let cont = core::array::from_fn(|i| m.byte(0x9147f4 + i as u32));
    let mask = core::array::from_fn(|i| m.word(0x9147dc + i as u32 * 4));
    let min = core::array::from_fn(|i| m.byte(0x914044 + i as u32));
    let max = core::array::from_fn(|i| m.byte(0x91404a + i as u32));
    let overspill: [u8; 4096] = core::array::from_fn(|i| m.byte(0x91436c + 1088 + i as u32));
    let tables = PcreTables {
        op_lengths: &op,
        ctypes: &ctypes,
        default_tables: &defaults,
        escapes: &escapes,
        utf8_limits: &limits,
        utf8_first: &first,
        utf8_cont: &cont,
        utf8_mask: &mask,
        rep_min: &min,
        rep_max: &max,
        posix_lengths: &[],
        posix_names: &[],
        posix_maps: &[],
        ctypes_overspill: &overspill,
    };
    let mut host = PcreAdapter { m, h, s };
    let result = pcre_exec_simple(
        &mut host,
        p,
        engine[1],
        &mut store,
        &program[..pn],
        None,
        &subject,
        256,
        n,
        offset as i32,
        0,
        Some(captures),
        30,
        &tables,
    );
    for (off, v) in [
        (8, store.block_count),
        (10, store.frames_per_block),
        (12, store.frame_size),
        (14, store.max_blocks),
        (16, store.current_block),
        (18, store.highest_block),
        (20, store.max_frames),
    ] {
        host.m.put_half(p + off, v);
    }
    for i in 0..store.block_count as usize {
        let b = &store.blocks[i];
        host.m
            .put_word(m_word_addr(p, 4, host.m) + i as u32 * 4, b.entry_id);
        host.m.put_word(b.entry_id, b.code_id);
        host.m
            .put_word(b.entry_id + 4, size as u32 * per as u32 + 1);
        host.m.put_word(b.entry_id + 8, b.used);
        host.m.put_word(b.entry_id + 12, b.extra_id);
        host.m.put_byte(b.entry_id + 16, b.count);
        for j in 0..per as usize {
            let idx = i * per as usize + j;
            host.m.put_byte(b.extra_id + j as u32, store.tags[idx]);
            encode_frame(
                host.m,
                b.code_id + j as u32 * size as u32,
                &store.frames[idx],
                store.blocks,
                code,
                subject_base,
                size,
            );
        }
    }
    result
}
fn m_word_addr(p: u32, off: u32, m: &Memory<'_>) -> u32 {
    m.word(p + off)
}

fn frame_ref_at(
    ptr: u32,
    blocks: &[crate::regex::FrameBlockState],
    size: u16,
    extra: u32,
) -> Option<crate::regex::FrameRef> {
    if ptr == 0 {
        return None;
    }
    let ptr = ptr.wrapping_sub(extra);
    for (i, b) in blocks.iter().enumerate() {
        if ptr >= b.code_id && ptr < b.code_id + size as u32 * 10 {
            return Some(crate::regex::FrameRef {
                block: i as u16,
                offset: ptr - b.code_id,
            });
        }
    }
    panic!("unknown frame ptr {ptr:x}")
}
fn frame_addr(
    r: Option<crate::regex::FrameRef>,
    blocks: &[crate::regex::FrameBlockState],
    extra: u32,
) -> u32 {
    r.map(|f| blocks[f.block as usize].code_id + f.offset + extra)
        .unwrap_or(0)
}
fn decode_frame(
    m: &Memory<'_>,
    p: u32,
    b: &[crate::regex::FrameBlockState],
    code: u32,
    subject: u32,
    size: u16,
) -> crate::regex::MatchFrame {
    let mut f = crate::regex::MatchFrame::default();
    f.prev = frame_ref_at(m.word(p + 0), b, size, 0);
    f.eptr = m.word(p + 4).wrapping_sub(subject) as i32 as i64;
    f.ecode = if m.word(p + 8) == 0 {
        usize::MAX
    } else {
        m.word(p + 8).wrapping_sub(code) as usize
    };
    f.offset_top = m.word(p + 12) as i32;
    f.ims = m.word(p + 16);
    f.eptrb = frame_ref_at(m.word(p + 20), b, size, 272);
    f.flags = m.word(p + 24);
    f.callpat = if m.word(p + 28) == 0 {
        usize::MAX
    } else {
        m.word(p + 28).wrapping_sub(code) as usize
    };
    f.charptr = if m.word(p + 32) == 0 {
        usize::MAX
    } else {
        m.word(p + 32).wrapping_sub(code) as usize
    };
    f.data = if m.word(p + 36) == 0 {
        usize::MAX
    } else {
        m.word(p + 36).wrapping_sub(code) as usize
    };
    f.next = if m.word(p + 40) == 0 {
        usize::MAX
    } else {
        m.word(p + 40).wrapping_sub(code) as usize
    };
    f.pp = m.word(p + 44).wrapping_sub(subject) as i32 as i64;
    f.prev_code = if m.word(p + 48) == 0 {
        usize::MAX
    } else {
        m.word(p + 48).wrapping_sub(code) as usize
    };
    f.saved_eptr = m.word(p + 52).wrapping_sub(subject) as i32 as i64;
    f.rec_prev = frame_ref_at(m.word(p + 56), b, size, 56);
    f.rec_group = m.word(p + 60) as i32;
    f.rec_after = if m.word(p + 64) == 0 {
        usize::MAX
    } else {
        m.word(p + 64).wrapping_sub(code) as usize
    };
    f.rec_start = m.word(p + 68).wrapping_sub(subject) as i32 as i64;
    f.rec_saved_max = m.word(p + 76) as i32;
    f.cur_is_word = m.word(p + 80) as i32;
    f.condition = m.word(p + 84) as i32;
    f.minimize = m.word(p + 88) as i32;
    f.prev_is_word = m.word(p + 92) as i32;
    f.original_ims = m.word(p + 96);
    f.ctype = m.word(p + 100) as i32;
    f.fc = m.word(p + 104) as i32;
    f.fi = m.word(p + 108) as i32;
    f.length = m.word(p + 112) as i32;
    f.max = m.word(p + 116) as i32;
    f.min = m.word(p + 120) as i32;
    f.number = m.word(p + 124) as i32;
    f.offset = m.word(p + 128) as i32;
    f.op = m.word(p + 132) as i32;
    f.save_capture_last = m.word(p + 136) as i32;
    f.save_offset1 = m.word(p + 140) as i32;
    f.save_offset2 = m.word(p + 144) as i32;
    f.save_offset3 = m.word(p + 148) as i32;
    f.block_prev = frame_ref_at(m.word(p + 272), b, size, 272);
    f.block_eptr = m.word(p + 276).wrapping_sub(subject) as i32 as i64;
    f.result = m.word(p + 280) as i32;
    for i in 0..30 {
        f.stacksave[i] = m.word(p + 152 + i as u32 * 4) as i32;
    }
    f.rec_scratch = m.word(p + 72) == p + 152;
    f
}
fn encode_frame(
    m: &mut Memory<'_>,
    p: u32,
    f: &crate::regex::MatchFrame,
    b: &[crate::regex::FrameBlockState],
    code: u32,
    subject: u32,
    _size: u16,
) {
    m.put_word(p + 0, frame_addr(f.prev, b, 0));
    m.put_word(p + 4, subject.wrapping_add(f.eptr as u32));
    m.put_word(
        p + 8,
        if f.ecode == usize::MAX {
            0
        } else {
            code.wrapping_add(f.ecode as u32)
        },
    );
    m.put_word(p + 12, f.offset_top as u32);
    m.put_word(p + 16, f.ims);
    m.put_word(p + 20, frame_addr(f.eptrb, b, 272));
    m.put_word(p + 24, f.flags);
    m.put_word(
        p + 28,
        if f.callpat == usize::MAX {
            0
        } else {
            code.wrapping_add(f.callpat as u32)
        },
    );
    m.put_word(
        p + 32,
        if f.charptr == usize::MAX {
            0
        } else {
            code.wrapping_add(f.charptr as u32)
        },
    );
    m.put_word(
        p + 36,
        if f.data == usize::MAX {
            0
        } else {
            code.wrapping_add(f.data as u32)
        },
    );
    m.put_word(
        p + 40,
        if f.next == usize::MAX {
            0
        } else {
            code.wrapping_add(f.next as u32)
        },
    );
    m.put_word(p + 44, subject.wrapping_add(f.pp as u32));
    m.put_word(
        p + 48,
        if f.prev_code == usize::MAX {
            0
        } else {
            code.wrapping_add(f.prev_code as u32)
        },
    );
    m.put_word(p + 52, subject.wrapping_add(f.saved_eptr as u32));
    m.put_word(p + 56, frame_addr(f.rec_prev, b, 56));
    m.put_word(p + 60, f.rec_group as u32);
    m.put_word(
        p + 64,
        if f.rec_after == usize::MAX {
            0
        } else {
            code.wrapping_add(f.rec_after as u32)
        },
    );
    m.put_word(p + 68, subject.wrapping_add(f.rec_start as u32));
    m.put_word(p + 76, f.rec_saved_max as u32);
    m.put_word(p + 80, f.cur_is_word as u32);
    m.put_word(p + 84, f.condition as u32);
    m.put_word(p + 88, f.minimize as u32);
    m.put_word(p + 92, f.prev_is_word as u32);
    m.put_word(p + 96, f.original_ims);
    m.put_word(p + 100, f.ctype as u32);
    m.put_word(p + 104, f.fc as u32);
    m.put_word(p + 108, f.fi as u32);
    m.put_word(p + 112, f.length as u32);
    m.put_word(p + 116, f.max as u32);
    m.put_word(p + 120, f.min as u32);
    m.put_word(p + 124, f.number as u32);
    m.put_word(p + 128, f.offset as u32);
    m.put_word(p + 132, f.op as u32);
    m.put_word(p + 136, f.save_capture_last as u32);
    m.put_word(p + 140, f.save_offset1 as u32);
    m.put_word(p + 144, f.save_offset2 as u32);
    m.put_word(p + 148, f.save_offset3 as u32);
    m.put_word(p + 272, frame_addr(f.block_prev, b, 272));
    m.put_word(p + 276, subject.wrapping_add(f.block_eptr as u32));
    m.put_word(p + 280, f.result as u32);
    for i in 0..30 {
        m.put_word(p + 152 + i as u32 * 4, f.stacksave[i] as u32);
    }
    if f.rec_scratch {
        m.put_word(p + 72, p + 152);
    }
}

impl crate::speech_part_3::Host for LiveCalls<'_> {
    fn call(&mut self, m: &mut Memory<'_>, f: u32, a: &[u32]) -> u32 {
        Calls::call(self, m, f, a)
    }
}
impl crate::boot_part_b::Host for LiveCalls<'_> {
    fn thread_id(&mut self) -> u32 {
        self.boot.platform.thread_id()
    }
    fn call(&mut self, m: &mut Memory<'_>, f: u32, a: &[u32]) -> u32 {
        Calls::call(self, m, f, a)
    }
}
