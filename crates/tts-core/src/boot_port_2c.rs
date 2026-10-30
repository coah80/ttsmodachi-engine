//! boot-port-2c: fn_26caec, fn_1ee9f8 and fn_1ef0a0, certified from live ARM boot scenes.
//!
//! Guest pointers stay guest addresses; the caller owns the arena. The three
//! functions read engine objects that only a real boot builds, so the ports
//! take the engine services as host traits and never script a result.
//!
//! The bodies follow the ROM's ARM, not `src/`. The join test of fn_1ee9f8,
//! its `out520` comparison, the second `mlset` lookup and the 0xea62 log
//! arguments differ from the decompiled C. The evidence is in
//! `rust/evidence/boot-port-2c-round1.json`.
//!
//! The caller supplies the literals and the scratch windows. The literal
//! window holds the strings the ROM reads from its literal pool; the scratch
//! window holds the three 256-byte split buffers. Both live outside every
//! compared page, exactly like the ARM's stack.

use crate::boot::Memory;

/// `Obj_fn_1ee808` field offsets.
pub mod obj {
    pub const ENV: u32 = 0x00;
    pub const H0: u32 = 0x04;
    pub const H1: u32 = 0x08;
    pub const F12: u32 = 0x0c;
    pub const P16: u32 = 0x10;
    pub const F24: u32 = 0x18;
    pub const P28: u32 = 0x1c;
    pub const F36: u32 = 0x24;
    pub const BUF72: u32 = 0x48;
    pub const MAP76: u32 = 0x4c;
    pub const BUF80: u32 = 0x50;
    pub const SIZE: u32 = 0x54;
}

/// `Ctx_fn_1ee808` field offsets.
pub mod ctx {
    pub const TAG: u32 = 0x00;
    pub const HEAP: u32 = 0x04;
    pub const F08: u32 = 0x08;
    pub const F0C: u32 = 0x0c;
    pub const LOG: u32 = 0x10;
    pub const PARAMS: u32 = 0x14;
    pub const RES: u32 = 0x18;
    pub const F1C: u32 = 0x1c;
}

/// The literal indices of the strings the three functions read.
pub mod lit {
    pub const EXTCLCCFG: u32 = 0;
    pub const EXTRAESCLANG: u32 = 1;
    pub const EXTRAESCTN: u32 = 2;
    pub const MLSET: u32 = 3;
    pub const LANGCODE: u32 = 4;
    pub const MPTHREE: u32 = 5;
    pub const EMPTY: u32 = 6;
    pub const PERCENT_S_S: u32 = 7;
    pub const LANGUAGE: u32 = 8;
    pub const LIDVOICESWITCH: u32 = 9;
    pub const YES: u32 = 10;
    pub const NO: u32 = 11;
    pub const LIDLANGUAGES: u32 = 12;
    pub const COMMA: u32 = 13;
    pub const STAR: u32 = 14;
    pub const EQUALS: u32 = 15;
    pub const PLUS: u32 = 16;
    pub const SYNTHSTREAM: u32 = 17;
    pub const FE_DEPES: u32 = 18;
    pub const FE_DCTLKP: u32 = 19;
    pub const NOCLMSET: u32 = 20;
    /// The literal texts, each with its terminator.
    pub const TEXTS: [&[u8]; 21] = [
        b"extclccfg\0",
        b"extraesclang\0",
        b"extraesctn\0",
        b"mlset\0",
        b"langcode\0",
        b"mpthree\0",
        b"\0",
        b"%s%s\0",
        b"language\0",
        b"lidvoiceswitch\0",
        b"yes\0",
        b"no\0",
        b"lidlanguages\0",
        b",\0",
        b"*\0",
        b"=\0",
        b"+\0",
        b"SYNTHSTREAM\0",
        b"FE_DEPES\0",
        b"FE_DCTLKP\0",
        b"noclmset\0",
    ];
    const fn offsets() -> [u32; 21] {
        let mut out = [0u32; 21];
        let mut at = 0u32;
        let mut i = 0;
        while i < 21 {
            out[i] = at;
            at += TEXTS[i].len() as u32;
            i += 1;
        }
        out
    }
    /// The offset of every literal inside the window.
    pub const OFFSET: [u32; 21] = offsets();
    /// The size of the whole literal window.
    pub const SIZE: u32 = OFFSET[20] + TEXTS[20].len() as u32;
}

/// The canonical code-pointer tokens. The map object stores five callback
/// addresses; native and ROM addresses differ per region, so both sides use
/// these tokens.
pub mod token {
    pub const COPY: u32 = 0x90000000;
    pub const DESTROY: u32 = 0x90000001;
    pub const COMPARE: u32 = 0x90000002;
    pub const RETAIN: u32 = 0x90000003;
    pub const RELEASE: u32 = 0x90000004;
    /// The tokens in the order the map object stores its five callbacks.
    pub const ALL: [u32; 5] = [COPY, DESTROY, COMPARE, RETAIN, RELEASE];
}

/// The engine services the three functions need from the platform.
pub trait Host {
    /// `fn_175a1c`. A named parameter string: the status and the guest
    /// address of the value.
    fn parameter(&mut self, m: &mut Memory<'_>, params: u32, name: u32) -> (u32, u32);
    /// `fn_1759a0`. Set a named parameter string.
    fn set_parameter(&mut self, m: &mut Memory<'_>, params: u32, name: u32, value: u32) -> u32;
    /// `fn_174f9c`. Register the owner of a named parameter.
    fn set_owner(
        &mut self,
        m: &mut Memory<'_>,
        params: u32,
        key: u32,
        owner: u32,
        b: u32,
        f: f32,
    );
    /// `fn_19611c`. The engine context block for a handle pair: the status and
    /// the guest address of the block.
    fn context(&mut self, m: &mut Memory<'_>, h0: u32, h1: u32) -> (u32, u32);
    /// `fn_19611c` with a caller-chosen out slot: the status; the block
    /// lands at `out`. The default keeps the old behaviour. `fn_26caec`
    /// takes its sub context in the pipeline object itself.
    fn context_into(&mut self, m: &mut Memory<'_>, h0: u32, h1: u32, out: u32) -> u32 {
        let (rc, ctx) = self.context(m, h0, h1);
        if rc < 0x80000000 {
            m.put_word(out, ctx);
        }
        rc
    }
    /// `fn_17b058`. Open a named engine resource: the status and the guest
    /// address of the resource block.
    fn resource_open(&mut self, m: &mut Memory<'_>, resources: u32, name: u32) -> (u32, u32);
    /// `fn_17b1c4`. Release a named engine resource.
    fn release_resource(&mut self, m: &mut Memory<'_>, resources: u32, name: u32);
    /// `fn_1963b8`. Allocate a zeroed block from the heap.
    fn calloc(&mut self, m: &mut Memory<'_>, heap: u32, count: u32, size: u32) -> u32;
    /// `fn_196394`. Allocate a block from the heap.
    fn allocate(&mut self, m: &mut Memory<'_>, heap: u32, size: u32) -> u32;
    /// `fn_196428`. Release a block.
    fn release(&mut self, m: &mut Memory<'_>, heap: u32, pointer: u32);
    /// `fn_1964e0`. Create the map object with the five callback words.
    fn map_create(&mut self, m: &mut Memory<'_>, ctx: u32, flag: u32, cb: [u32; 5], out: u32)
        -> u32;
    /// `fn_1966ac`. Look a key up in the map: the status and the value.
    fn map_lookup(&mut self, m: &mut Memory<'_>, map: u32, key: u32) -> (u32, u32);
    /// `fn_19668c`. Insert a key into the map.
    fn map_insert(&mut self, m: &mut Memory<'_>, map: u32, key: u32, value: u32) -> u32;
    /// `fn_19672c`. Remove a key from the map.
    fn map_remove(&mut self, m: &mut Memory<'_>, map: u32, key: u32) -> u32;
    /// `fn_196830`, `fn_196860` and `fn_1968cc`. Empty the map and release
    /// every value it holds.
    fn clear_map(&mut self, m: &mut Memory<'_>, map: u32);
    /// `fn_174e74`. Register the owner under a parameter key.
    fn register_owner(
        &mut self,
        m: &mut Memory<'_>,
        params: u32,
        key: u32,
        owner: u32,
        b: u32,
        f: f32,
    ) -> u32;
    /// `fn_26c858`. Release a pipeline object.
    fn cleanup(&mut self, m: &mut Memory<'_>, p: u32) -> u32;
    /// `fn_1961d0`. The engine's second handle pair.
    ///
    /// The default reports the call as unbound. The lane that binds the
    /// engine's call boundary overrides it.
    fn engine_call(&mut self, _m: &mut Memory<'_>, _f: u32, _args: &[u32]) -> u32 {
        0x80602007
    }
    /// `fn_174ca8`. The parameter table of one language.
    ///
    /// The default reports the table as unbound.
    fn secondary_parameters(&mut self, _m: &mut Memory<'_>, _a: [u32; 4]) -> u32 {
        0x80602007
    }
    /// `fn_17ae04`. The linguistic table of one language.
    ///
    /// The default reports the table as unbound.
    fn secondary_linguistic(
        &mut self,
        _m: &mut Memory<'_>,
        _a: [u32; 4],
        _scratch: u32,
    ) -> u32 {
        0x80602007
    }
    /// `fn_179f30`. The broker's open of one file.
    ///
    /// The default reports the broker as unbound.
    fn broker_open(&mut self, _m: &mut Memory<'_>, _ctx: u32, _name: u32, _out: u32) -> u32 {
        0x80602007
    }
    /// `fn_173dc0`. The engine log. `module` is the region's D_8f905a.
    fn log(&mut self, m: &mut Memory<'_>, log: u32, module: u32, code: u32, fmt: u32, a: u32, b: u32);
}

/// The regional data the three functions read.
pub struct Environment {
    /// The region's D_8f902c block.
    pub d8: u32,
    /// The region's D_8f905a module name.
    pub d905a: u32,
    /// The caller's literal window.
    pub literals: u32,
    /// The caller's scratch window. The three split buffers live at
    /// `scratch`, `scratch + 0x100` and `scratch + 0x200`.
    pub scratch: u32,
}

impl Environment {
    /// The guest address of a literal.
    pub fn lit(&self, index: u32) -> u32 {
        self.literals + lit::OFFSET[index as usize]
    }
    /// The three split buffers.
    pub fn out8(&self) -> u32 {
        self.scratch
    }
    pub fn out264(&self) -> u32 {
        self.scratch + 0x100
    }
    pub fn out520(&self) -> u32 {
        self.scratch + 0x200
    }
    /// D_8f902c.fb0, the owner default of fn_174e74.
    pub fn owner_default(&self, m: &Memory<'_>) -> u32 {
        m.word(self.d8 + 0xb0)
    }
    /// D_8f902c.fb4, the owner default of fn_174e74.
    pub fn owner_float(&self, m: &Memory<'_>) -> f32 {
        f32::from_bits(m.word(self.d8 + 0xb4))
    }
    /// Write the literal window into the arena.
    pub fn write_literals(&self, m: &mut Memory<'_>) {
        let mut at = self.literals;
        for text in lit::TEXTS.iter() {
            for byte in *text {
                m.bytes[m.offset(at)] = *byte;
                at += 1;
            }
        }
    }
}

fn cstr_eq(m: &Memory<'_>, a: u32, b: u32) -> bool {
    crate::normalization::strcmp_at(m.bytes, a, b) == 0
}

fn cstr_len(m: &mut Memory<'_>, s: u32) -> u32 {
    crate::boot_port_1::fn_195d90(m, s)
}

fn str_find(m: &Memory<'_>, haystack: u32, needle: u32) -> u32 {
    crate::normalization::strstr_at(m.bytes, haystack, needle).unwrap_or(0)
}

fn copy_string(m: &mut Memory<'_>, dst: u32, src: u32) -> u32 {
    crate::frontend::copy_str(m.bytes, dst, src)
}

fn append_string(m: &mut Memory<'_>, dst: u32, src: u32) -> u32 {
    crate::frontend::append_str(m.bytes, dst, src)
}

fn lower(c: u32) -> u32 {
    crate::normalization::tolower_240ba0(c)
}

fn folded_eq(m: &Memory<'_>, a: u32, b: u32) -> bool {
    crate::fe_text::strcmp_ci_1769d0(m.bytes, a, b) == 0
}

/// `fn_1ee9f8`. Rebuild the extra language configuration of one owner.
///
/// The ROM returns zero only when both `extraesclang` and `extraesctn` are
/// empty. The decompiled C tests the opposite pair.
pub fn fn_1ee9f8(m: &mut Memory<'_>, p: u32, env: &Environment, h: &mut impl Host) -> u32 {
    let envp = m.word(p);
    let params = m.word(envp + ctx::PARAMS);
    let log = m.word(envp + ctx::LOG);
    let (r6, cfg) = h.parameter(m, params, env.lit(lit::EXTCLCCFG));
    if r6 >= 0x80000000 || cfg == 0 {
        return 0;
    }
    let (_, clang) = h.parameter(m, params, env.lit(lit::EXTRAESCLANG));
    let (_, ctn) = h.parameter(m, params, env.lit(lit::EXTRAESCTN));
    let (_, mlset) = h.parameter(m, params, env.lit(lit::MLSET));
    let (_, langcode) = h.parameter(m, params, env.lit(lit::LANGCODE));
    if !folded_eq(m, ctn, env.lit(lit::MPTHREE)) && !folded_eq(m, ctn, env.lit(lit::EMPTY)) {
        h.log(
            m,
            log,
            env.d905a,
            0xea62,
            env.lit(lit::PERCENT_S_S),
            env.lit(lit::EXTRAESCTN),
            ctn,
        );
    }
    if cstr_eq(m, clang, env.lit(lit::EMPTY)) {
        h.set_parameter(m, params, env.lit(lit::LIDVOICESWITCH), env.lit(lit::YES));
        return join(m, p, env, h, r6, cfg, clang, ctn, mlset, langcode);
    }
    // The parse loop: the clang list is cut into four-character groups.
    let out8 = env.out8();
    let mut r4 = 0u32;
    let mut r7 = 0u32;
    loop {
        let r8 = r4 + 1;
        let mut log_clang = false;
        if (r8 & 3) == 0 {
            m.bytes[m.offset(out8 + r7)] = 0;
            r7 = 0;
            let c = m.byte(clang + r4);
            if c != 44 && c != 0 {
                log_clang = true;
            } else if str_find(m, mlset, out8) == 0 || folded_eq(m, langcode, out8) {
                h.log(
                    m,
                    log,
                    env.d905a,
                    0xea64,
                    env.lit(lit::PERCENT_S_S),
                    env.lit(lit::LANGUAGE),
                    out8,
                );
            }
        } else {
            let c = m.byte(clang + r4);
            let mut d = (c as u32).wrapping_sub(97);
            if d >= 26 {
                d = (c as u32).wrapping_sub(65);
                if d >= 26 {
                    log_clang = true;
                }
            }
            if !log_clang {
                m.bytes[m.offset(out8 + r7)] = lower(c as u32) as u8;
                r7 = (r7 + 1) & 0xffff;
            }
        }
        if log_clang {
            h.log(
                m,
                log,
                env.d905a,
                0xea63,
                env.lit(lit::PERCENT_S_S),
                env.lit(lit::EXTRAESCLANG),
                clang,
            );
        }
        r4 = r8 & 0xffff;
        if r6 >= 0x80000000 {
            break;
        }
        // The ROM leaves the loop when the length is shorter than the index.
        if cstr_len(m, clang) < r4 {
            break;
        }
    }
    // The lidlanguages path.
    let len = cstr_len(m, clang);
    let heap = m.word(envp + ctx::HEAP);
    let buf = h.allocate(m, heap, len + 5);
    if buf == 0 {
        return alloc_fail(m, envp, env, h);
    }
    copy_string(m, buf, langcode);
    append_string(m, buf, env.lit(lit::COMMA));
    append_string(m, buf, clang);
    let rc = h.set_parameter(m, params, env.lit(lit::LIDLANGUAGES), buf);
    if rc >= 0x80000000 {
        return rc;
    }
    h.release(m, heap, buf);
    let r6 = h.set_parameter(m, params, env.lit(lit::LIDVOICESWITCH), env.lit(lit::NO));
    join(m, p, env, h, r6, cfg, clang, ctn, mlset, langcode)
}

/// The join block of `fn_1ee9f8`: the configuration string is split and every
/// language is created.
fn join(
    m: &mut Memory<'_>,
    p: u32,
    env: &Environment,
    h: &mut impl Host,
    r6: u32,
    cfg: u32,
    clang: u32,
    ctn: u32,
    mlset: u32,
    langcode: u32,
) -> u32 {
    let envp = m.word(p);
    let heap = m.word(envp + ctx::HEAP);
    let out8 = env.out8();
    let out264 = env.out264();
    let out520 = env.out520();
    let buf80 = m.word(p + obj::BUF80);
    if buf80 != 0 {
        copy_string(m, buf80, env.lit(lit::EMPTY));
    }
    // The ROM returns zero only when both names are empty.
    if cstr_eq(m, clang, env.lit(lit::EMPTY)) && cstr_eq(m, ctn, env.lit(lit::EMPTY)) {
        return 0;
    }
    let mut src = cfg;
    let mut r6 = r6;
    loop {
        // The three fields of one configuration entry.
        let mut idx = 0u32;
        loop {
            let c = m.byte(src);
            if c != 43 && c != 0 {
                if idx >= 255 {
                    return 0;
                }
                m.bytes[m.offset(out8 + idx)] = c;
                idx += 1;
                src += 1;
                continue;
            }
            break;
        }
        if idx >= 255 {
            return 0;
        }
        m.bytes[m.offset(out8 + idx)] = 0;
        if m.byte(src) == 43 {
            src += 1;
        }
        let mut idx = 0u32;
        loop {
            let c = m.byte(src);
            if c != 61 && c != 0 {
                if idx >= 255 {
                    return 0;
                }
                m.bytes[m.offset(out520 + idx)] = c;
                idx += 1;
                src += 1;
                continue;
            }
            break;
        }
        if idx >= 255 {
            return 0;
        }
        m.bytes[m.offset(out520 + idx)] = 0;
        if m.byte(src) == 61 {
            src += 1;
        }
        let mut idx = 0u32;
        loop {
            let c = m.byte(src);
            if c != 44 && c != 0 {
                if idx >= 255 {
                    return 0;
                }
                m.bytes[m.offset(out264 + idx)] = c;
                idx += 1;
                src += 1;
                continue;
            }
            break;
        }
        if idx >= 255 {
            return 0;
        }
        m.bytes[m.offset(out264 + idx)] = 0;
        if m.byte(src) == 44 {
            src += 1;
        }
        // The star decision.
        let mut star2 = false;
        if !cstr_eq(m, out8, env.lit(lit::STAR)) && !folded_eq(m, langcode, out8) {
            if str_find(m, mlset, out8) == 0 {
                star2 = true;
            } else if clang == 0 {
                star2 = true;
            } else if str_find(m, clang, out8) == 0 {
                star2 = true;
            }
        }
        if !star2 && !cstr_eq(m, out520, env.lit(lit::STAR)) {
            // The ROM searches the country name for the voice name.
            if ctn == 0 || str_find(m, ctn, out520) == 0 {
                star2 = true;
            }
        }
        if star2 {
            // Drop the language the map still holds and continue.
            let map = m.word(p + obj::MAP76);
            let (rc, value) = h.map_lookup(m, map, out264);
            if rc >= 0x80000000 {
                r6 = 0;
            } else {
                let rc = h.map_remove(m, map, out264);
                if rc >= 0x80000000 {
                    return rc;
                }
                let rc = h.cleanup(m, value);
                r6 = rc;
                if rc >= 0x80000000 {
                    return rc;
                }
            }
        } else {
            // The nostar path: remember the entry and create the language.
            let buf80 = m.word(p + obj::BUF80);
            if buf80 == 0 {
                let len = cstr_len(m, cfg);
                let buf = h.allocate(m, heap, len + 2);
                m.put_word(p + obj::BUF80, buf);
                if buf == 0 {
                    return alloc_fail(m, envp, env, h);
                }
                copy_string(m, buf, env.lit(lit::EMPTY));
            }
            let buf80 = m.word(p + obj::BUF80);
            append_string(m, buf80, out8);
            append_string(m, buf80, env.lit(lit::PLUS));
            append_string(m, buf80, out520);
            append_string(m, buf80, env.lit(lit::EQUALS));
            append_string(m, buf80, out264);
            append_string(m, buf80, env.lit(lit::COMMA));
            if m.word(p + obj::BUF72) == 0 {
                let buf = h.allocate(m, heap, 1024);
                m.put_word(p + obj::BUF72, buf);
                if buf == 0 {
                    return alloc_fail(m, envp, env, h);
                }
            }
            let map = m.word(p + obj::MAP76);
            let (rc, value) = h.map_lookup(m, map, out264);
            r6 = rc;
            if rc >= 0x80000000 {
                let rc = fn_26caec(m, m.word(p + obj::H0), m.word(p + obj::H1), out264, h, env);
                if rc >= 0x80000000 {
                    return rc;
                }
                let rc = h.map_insert(m, map, out264, value);
                r6 = rc;
                if rc >= 0x80000000 {
                    return rc;
                }
            }
        }
        if m.byte(src) == 0 {
            return r6;
        }
    }
}

/// The allocation failure path of `fn_1ee9f8` and `fn_1ef0a0`.
fn alloc_fail(m: &mut Memory<'_>, envp: u32, env: &Environment, h: &mut impl Host) -> u32 {
    h.log(m, m.word(envp + ctx::LOG), env.d905a, 0xea60, 0, 0, 0);
    0x8c90200a
}

/// `fn_1ef0a0`. Create the extra-language owner of an engine handle.
pub fn fn_1ef0a0(
    m: &mut Memory<'_>,
    h0: u32,
    h1: u32,
    out: u32,
    env: &Environment,
    h: &mut impl Host,
) -> u32 {
    if out == 0 {
        return 0x8c902007;
    }
    m.put_word(out, 0);
    m.put_word(out + 4, 0);
    let (rc, ctx) = h.context(m, h0, h1);
    if rc >= 0x80000000 {
        return rc;
    }
    let heap = m.word(ctx + ctx::HEAP);
    let o = h.calloc(m, heap, 1, obj::SIZE);
    if o == 0 {
        return alloc_fail(m, ctx, env, h);
    }
    m.put_word(o + obj::ENV, ctx);
    m.put_word(o + obj::H0, h0);
    m.put_word(o + obj::H1, h1);
    let res = m.word(ctx + ctx::RES);
    let params = m.word(ctx + ctx::PARAMS);
    let (rc, r40) = h.resource_open(m, res, env.lit(lit::SYNTHSTREAM));
    if rc >= 0x80000000 {
        return fail(m, o, out, rc, env, h);
    }
    let (rc, r44) = h.resource_open(m, res, env.lit(lit::FE_DEPES));
    if rc >= 0x80000000 {
        return fail(m, o, out, rc, env, h);
    }
    let (rc, r48) = h.resource_open(m, res, env.lit(lit::FE_DCTLKP));
    if rc >= 0x80000000 {
        return fail(m, o, out, rc, env, h);
    }
    m.put_word(o + obj::F12, m.word(r40 + 4));
    m.put_word(o + obj::F24, m.word(r44 + 4));
    m.put_word(o + obj::P16, m.word(r44 + 8));
    m.put_word(o + obj::P16 + 4, m.word(r44 + 12));
    m.put_word(o + obj::F36, m.word(r48 + 4));
    m.put_word(o + obj::P28, m.word(r48 + 8));
    m.put_word(o + obj::P28 + 4, m.word(r48 + 12));
    m.put_word(o + obj::MAP76, 0);
    m.put_word(o + obj::BUF80, 0);
    m.put_word(o + obj::BUF72, 0);
    let key_a = env.lit(lit::EXTRAESCLANG);
    let key_b = env.lit(lit::EXTRAESCTN);
    let (rc, _) = h.parameter(m, params, key_a);
    if rc >= 0x80000000 {
        let rc = h.set_parameter(m, params, key_a, env.lit(lit::EMPTY));
        if rc >= 0x80000000 {
            return fail(m, o, out, rc, env, h);
        }
    }
    let (rc, _) = h.parameter(m, params, key_b);
    if rc >= 0x80000000 {
        let rc = h.set_parameter(m, params, key_b, env.lit(lit::EMPTY));
        if rc >= 0x80000000 {
            return fail(m, o, out, rc, env, h);
        }
    }
    let b = env.owner_default(m);
    let f = env.owner_float(m);
    let rc = h.register_owner(m, params, key_a, o, b, f);
    if rc >= 0x80000000 {
        return fail(m, o, out, rc, env, h);
    }
    let rc = h.register_owner(m, params, key_b, o, b, f);
    if rc >= 0x80000000 {
        return fail(m, o, out, rc, env, h);
    }
    let cb = [
        token::COPY,
        token::DESTROY,
        token::COMPARE,
        token::RETAIN,
        token::RELEASE,
    ];
    let rc = h.map_create(m, heap, 0, cb, o + obj::MAP76);
    if rc >= 0x80000000 {
        return fail(m, o, out, rc, env, h);
    }
    let rc = fn_1ee9f8(m, o, env, h);
    fail(m, o, out, rc, env, h)
}

/// The exit block of `fn_1ef0a0`.
fn fail(m: &mut Memory<'_>, o: u32, out: u32, rc: u32, env: &Environment, h: &mut impl Host) -> u32 {
    m.put_word(out, o);
    m.put_word(out + 4, 0xf488);
    if rc >= 0x80000000 {
        fn_1ee808(m, o, 0xf488, env, h);
        m.put_word(out, 0);
        m.put_word(out + 4, 0);
    }
    rc
}

/// `fn_1ee808`. Release the extra-language owner and everything it holds.
pub fn fn_1ee808(
    m: &mut Memory<'_>,
    p: u32,
    key: u32,
    env: &Environment,
    h: &mut impl Host,
) -> u32 {
    let rc = crate::boot_port_1::fn_176f94(m, p, key, 0xf488, obj::SIZE);
    if rc >= 0x80000000 {
        return 0x8c902008;
    }
    if p == 0 {
        return rc;
    }
    let envp = m.word(p);
    let params = m.word(envp + ctx::PARAMS);
    let res = m.word(envp + ctx::RES);
    let heap = m.word(envp + ctx::HEAP);
    let b = m.word(env.d8 + 0xbc);
    let f = f32::from_bits(m.word(env.d8 + 0xc0));
    h.set_owner(m, params, env.lit(lit::EXTRAESCLANG), p, b, f);
    h.set_owner(m, params, env.lit(lit::EXTRAESCTN), p, b, f);
    if m.word(p + obj::MAP76) != 0 {
        h.clear_map(m, m.word(p + obj::MAP76));
    }
    if m.word(p + obj::F12) != 0 {
        h.release_resource(m, res, env.lit(lit::SYNTHSTREAM));
    }
    if m.word(p + obj::F24) != 0 {
        h.release_resource(m, res, env.lit(lit::FE_DEPES));
    }
    if m.word(p + obj::F36) != 0 {
        h.release_resource(m, res, env.lit(lit::FE_DCTLKP));
    }
    if m.word(p + obj::BUF72) != 0 {
        h.release(m, heap, m.word(p + obj::BUF72));
    }
    if m.word(p + obj::BUF80) != 0 {
        h.release(m, heap, m.word(p + obj::BUF80));
    }
    h.release(m, heap, p);
    0
}

/// `fn_17b1c4`. Release a named engine resource.
///
/// The resource registry releases the entry and calls the entry's release
/// callback. `scratch` is 32 caller-owned bytes.
pub fn fn_17b1c4(
    m: &mut Memory<'_>,
    resources: u32,
    name: u32,
    scratch: u32,
    h: &mut impl crate::render_part_c::ParserMethodsM,
) -> u32 {
    crate::render_part_c::registry_release_m(m, resources, name, scratch, h)
}

/// `fn_26caec`. Create the synthesis pipeline of one language.
///
/// The port follows the ROM's ARM up to and through the broker: the engine
/// context, the 0x2d4 byte pipeline object, the engine's second context,
/// the parameter table and the linguistic table, the "noclmset" parameter,
/// the ":pipeline" name and the broker's open. The fail path releases the
/// pipeline object and returns the status.
///
/// The ROM's object layout: the sub context at +0, the engine's handle pair
/// at +4 and +8, the parameter table at +0x18, the linguistic table at
/// +0x1c and the broker's file handle at +0x3c.
pub fn fn_26caec(
    m: &mut Memory<'_>,
    h0: u32,
    h1: u32,
    text: u32,
    h: &mut impl Host,
    env: &Environment,
) -> u32 {
    // fn_19611c: the engine context of the handle pair.
    let (rc, ctx) = h.context(m, h0, h1);
    if rc >= 0x80000000 {
        return rc;
    }
    let heap = m.word(ctx + ctx::HEAP);
    // fn_1963b8: the pipeline object.
    let obj = h.calloc(m, heap, 1, 0x2d4);
    if obj == 0 {
        h.log(m, m.word(ctx + ctx::LOG), env.d905a, 60000, 0, 0, 0);
        return 0x8c90200a;
    }
    // fn_1961d0: the engine's second handle pair, in the object at +4.
    // The ARM takes it through its stack slot and the stores land in the
    // call's own write window, so the port stores into the object inline.
    let rc = h.engine_call(m, 0x1961d0, &[heap, m.word(ctx + ctx::F08), 0, 0, obj + 4]);
    if rc >= 0x80000000 {
        h.cleanup(m, obj);
        return rc;
    }
    // fn_19611c: the sub context of the second handle pair, in the object
    // at +0. The ARM's acquire writes the block to the object itself.
    let rc = h.context_into(m, m.word(obj + 4), m.word(obj + 8), obj);
    if rc >= 0x80000000 {
        h.cleanup(m, obj);
        return rc;
    }
    let sub = m.word(obj);
    // The sub context takes the three words of the engine context.
    m.put_word(sub + 0x1c, m.word(ctx + ctx::F1C));
    m.put_word(sub + 0x0c, m.word(ctx + ctx::F0C));
    m.put_word(sub + 0x10, m.word(ctx + ctx::LOG));
    // fn_174ca8: the parameter table. The ARM's call is
    // fn_174ca8(0, obj[4], obj[8], obj[4]+0x14): the table goes into the sub
    // context's own +0x14 field, not into the pipeline object.
    let rc = h.secondary_parameters(
        m,
        [0, m.word(obj + 4), m.word(obj + 8), m.word(obj + 4) + 0x14],
    );
    if rc >= 0x80000000 {
        h.cleanup(m, obj);
        return rc;
    }
    // fn_17ae04: the linguistic table. The ARM's call is
    // fn_17ae04(0, obj[4], obj[8], obj[4]+0x18): the sub context's +0x18.
    let rc = h.secondary_linguistic(
        m,
        [0, m.word(obj + 4), m.word(obj + 8), m.word(obj + 4) + 0x18],
        env.scratch + 0x700,
    );
    if rc >= 0x80000000 {
        h.cleanup(m, obj);
        return rc;
    }
    // fn_175a1c: the "noclmset" parameter. The broker's name is the text
    // and ":pipeline"; the parameter's value is used after the broker.
    let _ = h.parameter(m, m.word(ctx + ctx::PARAMS), env.lit(lit::NOCLMSET));
    // fn_195cd0 and fn_195c90: the pipeline's name.
    let buf = env.scratch + 0x300;
    copy_string(m, buf, text);
    let mut at = buf + cstr_len(m, buf);
    for byte in b":pipeline\0" {
        m.bytes[m.offset(at)] = *byte;
        at += 1;
    }
    // fn_179f30: the broker's open.
    let rc = h.broker_open(m, m.word(sub + 0x0c), buf, obj + 0x3c);
    if rc >= 0x80000000 {
        // fn_26c858: the pipeline release.
        h.cleanup(m, obj);
        return rc;
    }
    rc
}

/// `fn_174ca8`: the parameter table of one language.
///
/// The certified `boot::Memory::fn_174ca8` port takes the region's
/// `D_8d7614` descriptor: the map's five callback words at +0, the first
/// list's three words at +20 and the second list's three words at +32.
/// `a` is the ROM's argument list: the flags, the handle pair and the
/// caller's object slot.
pub fn fn_174ca8_at(
    m: &mut Memory<'_>,
    h: &mut impl crate::boot::Calls,
    a: [u32; 4],
    desc: u32,
) -> u32 {
    m.fn_174ca8(h, a, desc)
}

/// `fn_17ae04`: the linguistic table of one language.
///
/// The certified `frontend::create_ling_table` port takes the answers of the
/// four calls behind it, so the adapter makes those calls itself and then
/// runs the certified port: the write order, the failure paths and the
/// released object are the certified ones.
///
/// The map's five callback words come from the region's `D_8d773c`
/// descriptor, which the ROM reads before the context acquire, so the
/// adapter reads them there too. `scratch` is one caller-owned word: the
/// context pointer the acquire writes.
pub fn fn_17ae04_at(
    m: &mut Memory<'_>,
    h: &mut impl crate::boot::Calls,
    a: [u32; 4],
    desc: u32,
    scratch: u32,
) -> u32 {
    let [_flags, h0, h1, out] = a;
    let cb = [
        m.word(desc),
        m.word(desc + 4),
        m.word(desc + 8),
        m.word(desc + 12),
        m.word(desc + 16),
    ];
    // fn_19611c: the engine context of the handle pair.
    let acquire = h.call(m, 0x19611c, &[h0, h1, scratch]);
    if acquire >= 0x80000000 {
        return acquire;
    }
    let ctx = m.word(scratch);
    let heap = m.word(ctx + 4);
    let res = m.word(ctx + 8);
    // fn_1963b8: the twenty byte object.
    let object = h.call(m, 0x1963b8, &[heap, 1, 20]);
    if object == 0 {
        let scripts = crate::frontend::CreateScripts {
            acquire,
            ctx: Some(ctx),
            alloc: None,
            owner_rc: 0,
            owner: 0,
            create: 0,
            map: 0,
        };
        return ling_table(m, h, [h0, h1], out, &scripts, heap, 0, 0);
    }
    // fn_176e40: the owner of the table.
    let owner_rc = h.call(m, 0x176e40, &[res, heap, object + 12]);
    let owner = m.word(object + 12);
    // fn_1964e0: the map of the table.
    let create = if owner_rc >= 0x80000000 {
        owner_rc
    } else {
        h.call(
            m,
            0x1964e0,
            &[heap, 0, cb[0], cb[1], cb[2], cb[3], cb[4], object + 16],
        )
    };
    let map = m.word(object + 16);
    let scripts = crate::frontend::CreateScripts {
        acquire,
        ctx: Some(ctx),
        alloc: Some(object),
        owner_rc,
        owner,
        create,
        map,
    };
    ling_table(m, h, [h0, h1], out, &scripts, heap, object, owner)
}

/// Run the certified linguistic table port and the teardown it asks for.
///
/// The certified port owns the arena while it runs, so the two teardown
/// calls it makes are recorded and issued afterwards. They touch the heap
/// and the owner, never the caller's object slot, so the arena ends in the
/// same state as the ROM's.
fn ling_table(
    m: &mut Memory<'_>,
    h: &mut impl crate::boot::Calls,
    handle: [u32; 2],
    out: u32,
    scripts: &crate::frontend::CreateScripts,
    heap: u32,
    object: u32,
    owner: u32,
) -> u32 {
    let action = core::cell::Cell::new(0u8);
    let rc = {
        let arena = &mut *m.bytes;
        crate::frontend::create_ling_table(
            arena,
            handle,
            Some(out),
            scripts,
            || action.set(1),
            |_| action.set(2),
        )
    };
    match action.get() {
        1 => {
            h.call(m, 0x196428, &[heap, object]);
        }
        2 => {
            h.call(m, 0x176edc, &[owner]);
            h.call(m, 0x196428, &[heap, object]);
        }
        _ => {}
    }
    rc
}

/// The broker's regional constants.
///
/// The broker's globals live in the data segment, whose address differs per
/// region, so the caller supplies them. The capture reads every address from
/// the region's own literal pool (the broker's fn_1795f0 holds the module,
/// the broker string key, the add item format and the broker label; the
/// parameter and linguistic descriptors live in their own functions' pools).
#[derive(Clone, Copy)]
pub struct BrokerGlobals {
    /// `D_8d7668`: the module block. Its name is the first 108 bytes and its
    /// five callbacks follow at +108.
    pub module: u32,
    /// `D_177264`: the broker string key.
    pub brokerstring_key: u32,
    /// `D_177278`: the add item format.
    pub add_item_format: u32,
    /// `D_17729c`: the broker label.
    pub broker_label: u32,
    /// `D_17823c`: the full path property key.
    pub fullpath_key: u32,
    /// `D_8dc30c`: the character class table the case fold uses.
    pub ctype_table: u32,
    /// `D_8d7614`: the parameter table's callback descriptor (fn_174ca8).
    pub parameters_desc: u32,
    /// `D_8d773c`: the linguistic table's descriptor (fn_17ae04).
    pub linguistic_desc: u32,
}

impl BrokerGlobals {
    /// The `DataOpenGlobals` of the broker's open. `scratch` is the
    /// caller's 24 byte area, disjoint from every allocation.
    pub fn data_open(&self, scratch: u32) -> crate::boot_part_b::DataOpenGlobals {
        crate::boot_part_b::DataOpenGlobals {
            module: self.module,
            allocators: [0; 4],
            scratch,
            fullpath_key: self.fullpath_key,
            add_item_format: self.add_item_format,
            broker_label: self.broker_label,
            ctype_table: self.ctype_table,
            brokerstring_key: self.brokerstring_key,
        }
    }
}

/// The broker database services `fn_1772b0` reaches.
///
/// The ROM walks the database through the map's own iterator functions
/// (`fn_196794`, `fn_196860`, `fn_196830`) and takes the database lock
/// (`fn_176f2c`, `fn_176f5c`). The port keeps those calls in the host, so the
/// certified tree and allocator ports stay in one place and the query's own
/// control flow stays here.
pub trait BrokerQuery {
    /// `fn_176f2c`. Acquire the database lock.
    fn db_lock(&mut self, m: &mut Memory<'_>, lock: u32) -> u32;
    /// `fn_176f5c`. Release the database lock.
    fn db_unlock(&mut self, m: &mut Memory<'_>, lock: u32) -> u32;
    /// `fn_196794`. Open the database iterator for `name`. `mode` is the
    /// ROM's search flag: 1 is the ordered search, 0 the last-link scan.
    fn db_open(
        &mut self,
        m: &mut Memory<'_>,
        database: u32,
        name: u32,
        mode: u32,
        out: u32,
    ) -> u32;
    /// `fn_196860`. Read the next key/value pair of the iterator.
    fn db_next(&mut self, m: &mut Memory<'_>, iterator: u32, key: u32, value: u32) -> u32;
    /// `fn_196830`. Close the database iterator.
    fn db_close(&mut self, m: &mut Memory<'_>, iterator: u32) -> u32;
    /// `fn_1769d0`. The case-insensitive key compare.
    fn db_compare(&mut self, m: &Memory<'_>, a: u32, b: u32) -> i32;
}

/// `fn_1772b0`. The broker's database query.
///
/// `broker` is the broker object, `name` the key and `out` the caller's
/// entry slot. `scratch` is 12 caller-owned bytes: the iterator word, the
/// key word and the value word, the ROM's three stack locals. The port
/// follows the ROM's ARM, which is the same as `src/fn_1772b0.c`: the out
/// slot is cleared before the null checks, the iterator is closed on every
/// path that opened one, and the not-found status `0x89002014` of the
/// iterator open becomes `0x8150600d`.
pub fn fn_1772b0(
    m: &mut Memory<'_>,
    broker: u32,
    name: u32,
    out: u32,
    scratch: u32,
    h: &mut impl BrokerQuery,
) -> u32 {
    m.put_word(scratch, 0);
    m.put_word(scratch + 4, 0);
    m.put_word(scratch + 8, 0);
    m.put_word(out, 0);
    if broker == 0 || name == 0 {
        return 0x8150600d;
    }
    let ctx = m.word(broker + 4);
    let lock = m.word(ctx + 0x1c);
    let database = m.word(ctx + 0x10);
    let mut result = h.db_lock(m, lock);
    if result >= 0x80000000 {
        return result;
    }
    result = h.db_open(m, database, name, 1, scratch);
    if result < 0x80000000 {
        let iterator = m.word(scratch);
        let mut status = h.db_next(m, iterator, scratch + 4, scratch + 8);
        if status < 0x80000000 {
            loop {
                let order = h.db_compare(m, name, m.word(scratch + 4));
                if order == 0 {
                    m.put_word(out, m.word(scratch + 8));
                    break;
                }
                if order > 0 {
                    break;
                }
                status = h.db_next(m, iterator, scratch + 4, scratch + 8);
                if status >= 0x80000000 {
                    break;
                }
            }
        }
        h.db_close(m, iterator);
    } else if result & 0x1fff == 0x14 {
        result = 0x8150600d;
    }
    let status = h.db_unlock(m, lock);
    if status >= 0x80000000 {
        if result < 0x80000000 {
            result = status;
        } else {
            return result;
        }
    }
    if result < 0x80000000 && m.word(out) == 0 {
        return 0x8150600d;
    }
    result
}

/// A `DataHost` adapter that answers the broker's database query with this
/// lane's `fn_1772b0` port and leaves every other query to the inner host.
///
/// `scratch` is 16 caller-owned bytes: `fn_1772b0` uses the first twelve for
/// its three stack locals, the last four receive the entry word.
pub struct BrokerQueryHost<'a, H, Q> {
    pub inner: &'a mut H,
    pub database: &'a mut Q,
    pub scratch: u32,
}

impl<H: crate::boot_part_b::DataHost, Q: BrokerQuery> BrokerQueryHost<'_, H, Q> {
    /// The broker query's answer: the status and the entry word.
    pub fn query(&mut self, m: &mut Memory<'_>, broker: u32, name: u32) -> (u32, u32) {
        let out = self.scratch + 12;
        let rc = fn_1772b0(m, broker, name, out, self.scratch, self.database);
        (rc, m.word(out))
    }
}

impl<H: crate::boot_part_b::DataHost, Q: BrokerQuery> crate::boot_part_b::Host
    for BrokerQueryHost<'_, H, Q>
{
    fn call(&mut self, m: &mut Memory<'_>, f: u32, args: &[u32]) -> u32 {
        self.inner.call(m, f, args)
    }
    fn thread_id(&mut self) -> u32 {
        self.inner.thread_id()
    }
}

impl<H: crate::boot_part_b::DataHost, Q: BrokerQuery> crate::boot_part_b::OutputHost
    for BrokerQueryHost<'_, H, Q>
{
    fn output(&mut self, m: &mut Memory<'_>, f: u32, args: &[u32]) -> (u32, u32) {
        self.inner.output(m, f, args)
    }
}

impl<H: crate::boot_part_b::DataHost, Q: BrokerQuery> crate::boot_part_b::QueryHost
    for BrokerQueryHost<'_, H, Q>
{
    fn query_output(&mut self, m: &mut Memory<'_>, f: u32, args: &[u32]) -> (u32, u32) {
        if f == 0x1772b0 {
            let broker = args.first().copied().unwrap_or(0);
            let name = args.get(1).copied().unwrap_or(0);
            return self.query(m, broker, name);
        }
        self.inner.query_output(m, f, args)
    }
    fn insert_record(
        &mut self,
        m: &mut Memory<'_>,
        db: u32,
        key: u32,
        record: [u32; 4],
    ) -> u32 {
        self.inner.insert_record(m, db, key, record)
    }
    fn open_pair(&mut self, m: &mut Memory<'_>, f: u32, owner: [u32; 2], out: u32) -> u32 {
        self.inner.open_pair(m, f, owner, out)
    }
}

impl<H: crate::boot_part_b::DataHost, Q: BrokerQuery> crate::boot_part_b::PropertyHost
    for BrokerQueryHost<'_, H, Q>
{
    fn property(&mut self, m: &mut Memory<'_>, p: u32, key: &[u8]) -> (u32, u32) {
        self.inner.property(m, p, key)
    }
    fn insert(&mut self, m: &mut Memory<'_>, p: u32, key: u32, e: [u32; 3]) -> u32 {
        self.inner.insert(m, p, key, e)
    }
}

impl<H: crate::boot_part_b::DataHost, Q: BrokerQuery> crate::boot_part_b::DataHost
    for BrokerQueryHost<'_, H, Q>
{
    fn data_output(&mut self, m: &mut Memory<'_>, f: u32, args: &[u32], out: u32) -> u32 {
        self.inner.data_output(m, f, args, out)
    }
}
