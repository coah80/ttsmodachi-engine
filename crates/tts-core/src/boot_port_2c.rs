use crate::boot::Memory;

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

    pub const ADDRESSES: [u32; 21] = [
        0x17f9c0, 0x193bd4, 0x193be4, 0x17f9a4, 0x15df6c, 0x1d8dbc, 0x100006, 0x14f410, 0x155e60,
        0x193bc4, 0x154ce8, 0x159050, 0x1cfbe4, 0x101108, 0x1000aa, 0x10124e, 0x1003cc, 0x152718,
        0x1d8a08, 0x1cfbcc, 0x17f9b4,
    ];
}

pub mod token {
    pub const COPY: u32 = 0x90000000;
    pub const DESTROY: u32 = 0x90000001;
    pub const COMPARE: u32 = 0x90000002;
    pub const RETAIN: u32 = 0x90000003;
    pub const RELEASE: u32 = 0x90000004;

    pub const ALL: [u32; 5] = [COPY, DESTROY, COMPARE, RETAIN, RELEASE];
}

pub trait Host {
    fn parameter(&mut self, m: &mut Memory<'_>, params: u32, name: u32) -> (u32, u32);

    fn set_parameter(&mut self, m: &mut Memory<'_>, params: u32, name: u32, value: u32) -> u32;

    fn set_owner(&mut self, m: &mut Memory<'_>, params: u32, key: u32, owner: u32, b: u32, f: f32);

    fn context(&mut self, m: &mut Memory<'_>, h0: u32, h1: u32) -> (u32, u32);

    fn context_into(&mut self, m: &mut Memory<'_>, h0: u32, h1: u32, out: u32) -> u32 {
        let (rc, ctx) = self.context(m, h0, h1);
        if rc < 0x80000000 {
            m.put_word(out, ctx);
        }
        rc
    }

    fn resource_open(&mut self, m: &mut Memory<'_>, resources: u32, name: u32) -> (u32, u32);

    fn release_resource(&mut self, m: &mut Memory<'_>, resources: u32, name: u32);

    fn calloc(&mut self, m: &mut Memory<'_>, heap: u32, count: u32, size: u32) -> u32;

    fn allocate(&mut self, m: &mut Memory<'_>, heap: u32, size: u32) -> u32;

    fn release(&mut self, m: &mut Memory<'_>, heap: u32, pointer: u32);

    fn map_create(
        &mut self,
        m: &mut Memory<'_>,
        ctx: u32,
        flag: u32,
        cb: [u32; 5],
        out: u32,
    ) -> u32;

    fn map_lookup(&mut self, m: &mut Memory<'_>, map: u32, key: u32) -> (u32, u32);

    fn map_insert(&mut self, m: &mut Memory<'_>, map: u32, key: u32, value: u32) -> u32;

    fn map_remove(&mut self, m: &mut Memory<'_>, map: u32, key: u32) -> u32;

    fn clear_map(&mut self, m: &mut Memory<'_>, map: u32);

    fn register_owner(
        &mut self,
        m: &mut Memory<'_>,
        params: u32,
        key: u32,
        owner: u32,
        b: u32,
        f: f32,
    ) -> u32;

    fn cleanup(&mut self, m: &mut Memory<'_>, p: u32) -> u32;

    fn engine_call(&mut self, _m: &mut Memory<'_>, _f: u32, _args: &[u32]) -> u32 {
        0x80602007
    }

    fn secondary_parameters(&mut self, _m: &mut Memory<'_>, _a: [u32; 4]) -> u32 {
        0x80602007
    }

    fn secondary_linguistic(&mut self, _m: &mut Memory<'_>, _a: [u32; 4], _scratch: u32) -> u32 {
        0x80602007
    }

    fn broker_open(&mut self, _m: &mut Memory<'_>, _ctx: u32, _name: u32, _out: u32) -> u32 {
        0x80602007
    }

    fn log(
        &mut self,
        m: &mut Memory<'_>,
        log: u32,
        module: u32,
        code: u32,
        fmt: u32,
        a: u32,
        b: u32,
    );
}

pub struct Environment {
    pub d8: u32,

    pub d905a: u32,

    pub literals: [u32; 21],

    pub scratch: u32,
}

impl Environment {
    pub fn lit(&self, index: u32) -> u32 {
        self.literals[index as usize]
    }

    pub fn out8(&self) -> u32 {
        self.scratch
    }
    pub fn out264(&self) -> u32 {
        self.scratch + 0x100
    }
    pub fn out520(&self) -> u32 {
        self.scratch + 0x200
    }

    pub fn owner_default(&self, m: &Memory<'_>) -> u32 {
        m.word(self.d8 + 0xb0)
    }

    pub fn owner_float(&self, m: &Memory<'_>) -> f32 {
        f32::from_bits(m.word(self.d8 + 0xb4))
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

        if cstr_len(m, clang) < r4 {
            break;
        }
    }

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

    if cstr_eq(m, clang, env.lit(lit::EMPTY)) && cstr_eq(m, ctn, env.lit(lit::EMPTY)) {
        return 0;
    }
    let mut src = cfg;
    let mut r6 = r6;
    loop {
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
            if ctn == 0 || str_find(m, ctn, out520) == 0 {
                star2 = true;
            }
        }
        if star2 {
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

fn alloc_fail(m: &mut Memory<'_>, envp: u32, env: &Environment, h: &mut impl Host) -> u32 {
    h.log(m, m.word(envp + ctx::LOG), env.d905a, 0xea60, 0, 0, 0);
    0x8c90200a
}

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

fn fail(
    m: &mut Memory<'_>,
    o: u32,
    out: u32,
    rc: u32,
    env: &Environment,
    h: &mut impl Host,
) -> u32 {
    m.put_word(out, o);
    m.put_word(out + 4, 0xf488);
    if rc >= 0x80000000 {
        fn_1ee808(m, o, 0xf488, env, h);
        m.put_word(out, 0);
        m.put_word(out + 4, 0);
    }
    rc
}

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

pub fn fn_17b1c4(
    m: &mut Memory<'_>,
    resources: u32,
    name: u32,
    scratch: u32,
    h: &mut impl crate::render_part_c::ParserMethodsM,
) -> u32 {
    crate::render_part_c::registry_release_m(m, resources, name, scratch, h)
}

pub fn fn_26caec(
    m: &mut Memory<'_>,
    h0: u32,
    h1: u32,
    text: u32,
    h: &mut impl Host,
    env: &Environment,
) -> u32 {
    let (rc, ctx) = h.context(m, h0, h1);
    if rc >= 0x80000000 {
        return rc;
    }
    let heap = m.word(ctx + ctx::HEAP);

    let obj = h.calloc(m, heap, 1, 0x2d4);
    if obj == 0 {
        h.log(m, m.word(ctx + ctx::LOG), env.d905a, 60000, 0, 0, 0);
        return 0x8c90200a;
    }

    let rc = h.engine_call(m, 0x1961d0, &[heap, m.word(ctx + ctx::F08), 0, 0, obj + 4]);
    if rc >= 0x80000000 {
        h.cleanup(m, obj);
        return rc;
    }

    let rc = h.context_into(m, m.word(obj + 4), m.word(obj + 8), obj);
    if rc >= 0x80000000 {
        h.cleanup(m, obj);
        return rc;
    }
    let sub = m.word(obj);

    m.put_word(sub + 0x1c, m.word(ctx + ctx::F1C));
    m.put_word(sub + 0x0c, m.word(ctx + ctx::F0C));
    m.put_word(sub + 0x10, m.word(ctx + ctx::LOG));

    let rc = h.secondary_parameters(
        m,
        [0, m.word(obj + 4), m.word(obj + 8), m.word(obj + 4) + 0x14],
    );
    if rc >= 0x80000000 {
        h.cleanup(m, obj);
        return rc;
    }

    let rc = h.secondary_linguistic(
        m,
        [0, m.word(obj + 4), m.word(obj + 8), m.word(obj + 4) + 0x18],
        env.scratch + 0x700,
    );
    if rc >= 0x80000000 {
        h.cleanup(m, obj);
        return rc;
    }

    let _ = h.parameter(m, m.word(ctx + ctx::PARAMS), env.lit(lit::NOCLMSET));

    let buf = env.scratch + 0x300;
    copy_string(m, buf, text);
    let mut at = buf + cstr_len(m, buf);
    for byte in b":pipeline\0" {
        m.bytes[m.offset(at)] = *byte;
        at += 1;
    }

    let rc = h.broker_open(m, m.word(sub + 0x0c), buf, obj + 0x3c);
    if rc >= 0x80000000 {
        h.cleanup(m, obj);
        return rc;
    }
    rc
}

pub fn fn_174ca8_at(
    m: &mut Memory<'_>,
    h: &mut impl crate::boot::Calls,
    a: [u32; 4],
    desc: u32,
) -> u32 {
    m.fn_174ca8(h, a, desc)
}

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

    let acquire = h.call(m, 0x19611c, &[h0, h1, scratch]);
    if acquire >= 0x80000000 {
        return acquire;
    }
    let ctx = m.word(scratch);
    let heap = m.word(ctx + 4);
    let res = m.word(ctx + 8);

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

    let owner_rc = h.call(m, 0x176e40, &[res, heap, object + 12]);
    let owner = m.word(object + 12);

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

#[derive(Clone, Copy)]
pub struct BrokerGlobals {
    pub module: u32,

    pub brokerstring_key: u32,

    pub add_item_format: u32,

    pub broker_label: u32,

    pub fullpath_key: u32,

    pub ctype_table: u32,

    pub parameters_desc: u32,

    pub linguistic_desc: u32,
}

impl BrokerGlobals {
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

pub trait BrokerQuery {
    fn db_lock(&mut self, m: &mut Memory<'_>, lock: u32) -> u32;

    fn db_unlock(&mut self, m: &mut Memory<'_>, lock: u32) -> u32;

    fn db_open(&mut self, m: &mut Memory<'_>, database: u32, name: u32, mode: u32, out: u32)
        -> u32;

    fn db_next(&mut self, m: &mut Memory<'_>, iterator: u32, key: u32, value: u32) -> u32;

    fn db_close(&mut self, m: &mut Memory<'_>, iterator: u32) -> u32;

    fn db_compare(&mut self, m: &Memory<'_>, a: u32, b: u32) -> i32;
}

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

pub struct BrokerQueryHost<'a, H, Q> {
    pub inner: &'a mut H,
    pub database: &'a mut Q,
    pub scratch: u32,
}

impl<H: crate::boot_part_b::DataHost, Q: BrokerQuery> BrokerQueryHost<'_, H, Q> {
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
    fn insert_record(&mut self, m: &mut Memory<'_>, db: u32, key: u32, record: [u32; 4]) -> u32 {
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
