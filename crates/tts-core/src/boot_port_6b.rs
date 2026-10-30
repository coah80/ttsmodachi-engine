//! Boot-path cluster for fn_1e0400 (US/KR voice boot), lane 6b.
//!
//! The six callees and fn_1e0400 itself. Five bodies delegate to the
//! already-certified arena ports; fn_2412a0_m is a full body (the only prior
//! Rust shape was a model that skipped fn_243d50); fn_1e0400_m is the new
//! orchestrator. Live allocations go through the engine heap on the arena;
//! platform calls (log, release, context, file open) run through the host.

use crate::boot::Memory;

/// Host surface for the fn_1e0400 cluster.
///
/// `call` is the engine-heap seam (vtable slots the scenes point at the
/// platform addresses). `release_chunk_243d50` binds fn_243d50, which has src
/// (src/fn_243ad4.c) but no live-certified Rust port; the test harness
/// emulates it with the certified owner-close port and a slot release, and
/// the owning lane is named on the emulation.
pub trait Host: crate::boot_part_e::Host {
    fn release_chunk_243d50(&mut self, m: &mut Memory<'_>, record: u32) -> u32;
    /// fn_243a94 seam. The wrapper forwards (object, key, mode, path, 0, 0,
    /// type, version, out) exactly like src/fn_243a94.c. The test harness
    /// scripts the file open per scene mode and logs (mode, path length,
    /// path byte-sum, version, type-ok) so the argument order is checked;
    /// boot-a binds this seam to the real fn_242fd0_m in its voice harness.
    fn open_token_243a94(&mut self, m: &mut Memory<'_>, args: [u32; 9]) -> u32;
    /// fn_175a1c seam: named-parameter lookup. No Rust port exists, so the
    /// harness answers from the scene table and boot-a answers from live
    /// voice parameters.
    fn parameter_175a1c(&mut self, m: &mut Memory<'_>, params: u32, key: u32) -> (u32, u32);
}

struct Log<'a, H>(&'a mut H);
impl<H: Host> crate::log_live::LogMethodsM for Log<'_, H> {
    fn log_m(
        &mut self,
        f: u32,
        o: u32,
        c: u32,
        l: u32,
        a: u32,
        fmt: u32,
        va: u32,
        m: &mut Memory<'_>,
    ) -> u32 {
        self.0.call(m, f, &[o, c, l, a, fmt, va])
    }
}

/// fn_2421b0. Close the chunk reader: owner, name and chunk table.
pub fn fn_2421b0_m<H: Host + crate::render_part_a::ReaderHost>(
    m: &mut Memory<'_>,
    h: &mut H,
    r: u32,
    k: &crate::render_part_a::ReaderRuntime,
) -> u32 {
    crate::render_part_a::close_reader(m, h, r, k)
}

/// fn_2422ac. Pop the current chunk and advance past its aligned end.
pub fn fn_2422ac_m<H: Host + crate::render_part_a::ReaderHost>(
    m: &mut Memory<'_>,
    h: &mut H,
    r: u32,
    k: &crate::render_part_a::ReaderRuntime,
) -> u32 {
    crate::render_part_a::close_reader_chunk(m, h, r, k)
}

/// fn_24277c. Read a chunk header, validate it and push its descriptor.
pub fn fn_24277c_m<H: Host + crate::render_part_a::ReaderHost>(
    m: &mut Memory<'_>,
    h: &mut H,
    r: u32,
    name: u32,
    out_len: u32,
    out_data: u32,
    k: &crate::render_part_a::ReaderRuntime,
) -> u32 {
    crate::render_part_a::open_reader_chunk(m, h, r, name, out_len, out_data, k)
}

/// fn_242444. Copy a chunk string, decoding the reader's XOR layer.
pub fn fn_242444_m(
    m: &mut Memory<'_>,
    r: u32,
    src: u32,
    len: u32,
    off: u32,
    dst: u32,
    out_len: u32,
) -> u32 {
    let mut arena = crate::selection::arena::Arena::new(m.base, m.bytes);
    crate::render_part_a::read_chunk_string(&mut arena, r, src, len, off, dst, out_len)
}

/// fn_2413c4. Open the string-map reader and publish it through `out`.
#[allow(clippy::too_many_arguments)]
pub fn fn_2413c4_m<H: Host + crate::render_part_a::ReaderHost>(
    m: &mut Memory<'_>,
    h: &mut H,
    object: u32,
    tag: u32,
    name: u32,
    reader: u32,
    out: u32,
    e: &crate::boot_port_4::Environment,
    k: &crate::render_part_a::ReaderRuntime,
) -> u32 {
    crate::boot_port_4::fn_2413c4(m, h, object, tag, name, reader, out, e, k)
}

const SM_MODULE: u32 = 0x914834;

/// fn_2412a0. Close the string-map reader: its chunk records, name and self.
/// Follows src/fn_240dcc.c: failing record closes stick, the object frees
/// only when no record close failed.
pub fn fn_2412a0_m<H: Host>(
    m: &mut Memory<'_>,
    h: &mut H,
    p: u32,
) -> u32 {
    if p == 0 {
        return 0x8a20_2007;
    }
    let mut rc = 0;
    let ctx = m.word(p + 8);
    let log = m.word(ctx + 0x10);
    let heap = m.word(ctx + 4);
    crate::log_live::fn_1738e8(m, log, SM_MODULE, 4, 0, 0, 0, &mut Log(h));
    if m.word(p + 0x1c) != 0 {
        let r = h.release_chunk_243d50(m, m.word(p + 0x1c));
        if r >= 0x8000_0000 {
            rc = r;
        }
    }
    if m.word(p + 0x28) != 0 {
        let r = h.release_chunk_243d50(m, m.word(p + 0x28));
        if r >= 0x8000_0000 && rc < 0x8000_0000 {
            rc = r;
        }
    }
    if m.word(p + 0x0c) != 0 {
        crate::boot_part_e::release(m, heap, m.word(p + 0x0c), h);
    }
    if rc < 0x8000_0000 {
        crate::boot_part_e::release(m, heap, p, h);
    }
    crate::log_live::fn_1738e8(m, log, SM_MODULE, 4, 0, 0, 0, &mut Log(h));
    rc
}

fn strlen(m: &Memory<'_>, p: u32) -> u32 {
    crate::fe_text::strlen_195d90(m.bytes, m.offset(p) as u32) as u32
}

fn strcpy(m: &mut Memory<'_>, d: u32, s: u32) {
    let (dst, src) = (m.offset(d) as u32, m.offset(s) as u32);
    crate::normalization::strcpy_at(m.bytes, dst, src);
}

fn strcat(m: &mut Memory<'_>, d: u32, s: u32) {
    let (dst, src) = (m.offset(d) as u32, m.offset(s) as u32);
    crate::fe_text::strcat_195c90(m.bytes, dst, src);
}

fn strcmp(m: &Memory<'_>, a: u32, b: u32) -> i32 {
    let (mut x, mut y) = (m.offset(a) as u32, m.offset(b) as u32);
    loop {
        let (p, q) = (m.bytes[x as usize], m.bytes[y as usize]);
        if p == q && q != 0 {
            x += 1;
            y += 1;
            continue;
        }
        let d = p as i32 - q as i32;
        return if d < 0 { -1 } else if d > 0 { 1 } else { 0 };
    }
}

/// Case-insensitive compare following src/fn_1769d0.c (ASCII fold).
fn casecmp(m: &Memory<'_>, a: u32, b: u32) -> i32 {
    fn fold(c: u8) -> u8 {
        if c >= b'A' && c <= b'Z' {
            c + 32
        } else {
            c
        }
    }
    let (x, y) = (m.offset(a) as u32, m.offset(b) as u32);
    let mut i = 0u32;
    if fold(m.bytes[(x) as usize]) == fold(m.bytes[(y) as usize]) {
        loop {
            if m.bytes[(x + i) as usize] == 0 {
                return 0;
            }
            i += 1;
            if fold(m.bytes[(x + i) as usize]) != fold(m.bytes[(y + i) as usize]) {
                break;
            }
        }
    }
    fold(m.bytes[(x + i) as usize]) as i32 - fold(m.bytes[(y + i) as usize]) as i32
}

fn strstr(m: &Memory<'_>, hay: u32, needle: u32) -> u32 {
    let nl = strlen(m, needle) as usize;
    if nl == 0 {
        return hay;
    }
    let hl = strlen(m, hay) as usize;
    if nl > hl {
        return 0;
    }
    for i in 0..=(hl - nl) {
        let mut ok = true;
        for j in 0..nl {
            if m.byte(hay + i as u32 + j as u32) != m.byte(needle + j as u32) {
                ok = false;
                break;
            }
        }
        if ok {
            return hay + i as u32;
        }
    }
    0
}

/// Environment for fn_1e0400_m: the string-map open inputs, the token-file
/// open inputs, and the caller-owned literal addresses.
///
/// The ROM keeps its key/tag strings in rodata; the port takes their
/// addresses from the caller so the harness can stage the same bytes in the
/// arena (the C and ARM sides read them from ROM; only the bytes matter).
/// `ver` is the D_8f8ad4 word (va in the low half, vb in byte 2): the C
/// reads the ROM global, the ARM reads its own ROM, and the caller passes
/// the same value in because the arena cannot see that address.
pub struct Env6b<'a> {
    pub stringmap: &'a crate::boot_port_4::Environment,
    pub reader: &'a crate::render_part_a::ReaderRuntime,
    /// Scratch word receiving the token-reader handle.
    pub open_out: u32,
    /// Caller-staged NUL-terminated literals, same bytes as the ROM rodata.
    pub lit_empty: u32,
    pub lit_langcode: u32,
    pub lit_fedataprefix: u32,
    pub lit_fedatapackaging: u32,
    pub lit_fecfg: u32,
    pub lit_clc: u32,
    pub lit_clc_cmp: u32,
    pub lit_uni: u32,
    pub lit_phmp: u32,
    pub lit_slash: u32,
    pub lit_colon: u32,
    pub lit_uscore: u32,
    pub lit_dotdat: u32,
    pub lit_phonmap: u32,
    pub lit_phonmap_ipa: u32,
    pub lit_phonmap_u: u32,
    pub lit_phonmap_ipa_u: u32,
    pub lit_version: u32,
    pub lit_linf: u32,
    pub lit_vinf: u32,
    pub lit_lang: u32,
    pub lit_lexa: u32,
    pub lit_slan: u32,
    pub lit_slex: u32,
    pub lit_voic: u32,
    pub lit_vexa: u32,
    pub lit_svoi: u32,
    pub lit_svex: u32,
    pub lit_ipa: u32,
    pub lit_sapi: u32,
}

/// fn_1e0400. Load the phonmap readers for one voice: close stale readers,
/// resolve the language/packaging parameters, build the two candidate paths,
/// open each PHMP token file and attach every chunk kind to its slot.
///
/// `p` is the Obj_fn_1e0400 phonmap object. `arg` selects the voice sub-path.
/// Follows src/fn_1e0400.c exactly, including the i7==0/i7!=0 error rule and
/// the version-string capture into b44/b300.
#[allow(clippy::too_many_lines)]
pub fn fn_1e0400_m<H: Host + crate::render_part_a::ReaderHost>(
    m: &mut Memory<'_>,
    h: &mut H,
    p: u32,
    arg: u32,
    ver: u32,
    e: &Env6b<'_>,
) -> u32 {
    // Caller-owned scratch: two 128-byte path buffers, two 256-byte line
    // buffers, tag/length cells and the version cell. The ROM keeps these on
    // the stack; the port keeps them in the reader scratch and restores the
    // whole frame before returning so the replay stays whole-arena strict.
    let base = e.reader.scratch + 0x200;
    let buf0 = base;
    let buf1 = base + 128;
    let d0 = base + 256;
    let d1 = base + 512;
    let tag = base + 768;
    let ncell = base + 776;
    let szcell = base + 780;
    let encell = base + 784;
    let frame = base;
    let frame_len = 788;
    let at = m.offset(frame);
    let mut saved = [0u8; 792];
    saved[..frame_len].copy_from_slice(&m.bytes[at..at + frame_len]);
    let saved_out = m.word(e.open_out);
    m.bytes[m.offset(buf0)] = 0;
    m.bytes[m.offset(buf1)] = 0;
    m.put_word(tag, 0);
    m.put_word(tag + 4, 0);
    m.put_word(ncell, 0);
    m.put_word(szcell, 0);
    m.put_word(encell, 0);

    let rc = fn_1e0400_inner(m, h, p, arg, ver, e, buf0, buf1, d0, d1, tag, ncell, szcell, encell);

    let at = m.offset(frame);
    m.bytes[at..at + frame_len].copy_from_slice(&saved[..frame_len]);
    m.put_word(e.open_out, saved_out);
    rc
}

#[allow(clippy::too_many_arguments)]
fn fn_1e0400_inner<H: Host + crate::render_part_a::ReaderHost>(
    m: &mut Memory<'_>,
    h: &mut H,
    p: u32,
    arg: u32,
    ver: u32,
    e: &Env6b<'_>,
    buf0: u32,
    buf1: u32,
    d0: u32,
    d1: u32,
    tag: u32,
    ncell: u32,
    szcell: u32,
    encell: u32,
) -> u32 {
    for off in [0x0c, 0x14, 0x1c, 0x24, 0x10, 0x18, 0x20, 0x28, 0x22c, 0x230] {
        let slot = p + off;
        if m.word(slot) != 0 {
            fn_2412a0_m(m, h, m.word(slot));
            m.put_word(slot, 0);
        }
    }

    let p8 = m.word(p + 8);
    let params = m.word(p8 + 0x14);
    let (rc, lang) = h.parameter_175a1c(m, params, e.lit_langcode);
    let lang = if rc >= 0x8000_0000 { e.lit_empty } else { lang };
    let (rc, prefix) = h.parameter_175a1c(m, params, e.lit_fedataprefix);
    if rc < 0x8000_0000 && prefix != 0 && m.byte(prefix) != 0 {
        strcpy(m, buf0, prefix);
        strcpy(m, buf1, prefix);
    }
    let (rc, pack) = h.parameter_175a1c(m, params, e.lit_fedatapackaging);
    if rc < 0x8000_0000 && pack != 0 && strcmp(m, pack, e.lit_clc_cmp) == 0 {
        let (rc, fecfg) = h.parameter_175a1c(m, params, e.lit_fecfg);
        if rc >= 0x8000_0000 {
            return rc;
        }
        strcat(m, buf0, e.lit_clc);
        strcat(m, buf0, lang);
        if strstr(m, arg, e.lit_uni) == 0 {
            strcat(m, buf0, e.lit_slash);
            strcat(m, buf0, arg);
        }
        strcat(m, buf0, e.lit_slash);
        strcat(m, buf0, fecfg);
        strcat(m, buf0, e.lit_colon);
        strcat(m, buf1, e.lit_clc);
        strcat(m, buf1, lang);
        strcat(m, buf1, e.lit_slash);
        strcat(m, buf1, fecfg);
        strcat(m, buf1, e.lit_colon);
    }

    let flat = m.word(p8 + 0x0c) != 0;
    let (mode0, mode1): (u32, u32);
    if flat {
        strcat(m, buf0, e.lit_phonmap);
        strcat(m, buf1, e.lit_phonmap_ipa);
        mode0 = 0;
        mode1 = 2;
        strcat(m, buf0, lang);
        strcat(m, buf0, e.lit_slash);
        strcat(m, buf0, arg);
        strcat(m, buf1, lang);
    } else {
        strcat(m, buf0, e.lit_phonmap_u);
        strcat(m, buf1, e.lit_phonmap_ipa_u);
        mode0 = 1;
        mode1 = 3;
        strcat(m, buf0, lang);
        strcat(m, buf0, e.lit_uscore);
        strcat(m, buf0, arg);
        strcat(m, buf0, e.lit_dotdat);
        strcat(m, buf1, lang);
        strcat(m, buf1, e.lit_dotdat);
    }

    let slots = [
        p + 0x0c,
        p + 0x14,
        p + 0x1c,
        p + 0x24,
        p + 0x10,
        p + 0x18,
        p + 0x20,
        p + 0x28,
        p + 0x22c,
        p + 0x230,
    ];
    let tags = [
        e.lit_lang, e.lit_lexa, e.lit_slan, e.lit_slex, e.lit_voic, e.lit_vexa, e.lit_svoi,
        e.lit_svex, e.lit_ipa, e.lit_sapi,
    ];
    let mut status = 0;
    for i7 in 0..2u32 {
        let path = if i7 == 0 { buf0 } else { buf1 };
        let mode = if i7 == 0 { mode0 } else { mode1 };
        let ha = m.word(p);
        let hb = m.word(p + 4);
        // fn_243a94 forwards (object, key, mode, path, 0, 0, type,
        // version, out); the seam keeps the order and the harness (then
        // boot-a's real fn_242fd0_m binding) runs the open.
        let args = [ha, hb, mode, path, 0, 0, e.lit_phmp, ver, e.open_out];
        status = h.open_token_243a94(m, args);
        let handle = m.word(e.open_out);
        if status >= 0x8000_0000 {
            if i7 != 0 {
                status = 0;
            }
            return status;
        }
        loop {
            m.put_word(szcell, 0);
            m.put_word(encell, 0);
            if fn_24277c_m(m, h, handle, tag, szcell, encell, e.reader) >= 0x8000_0000 {
                break;
            }
            let sz = m.word(szcell);
            let en = m.word(encell);
            if strcmp(m, tag, e.lit_linf) == 0 || strcmp(m, tag, e.lit_vinf) == 0 {
                let mut off = 0u32;
                if status < 0x8000_0000 {
                    while off < sz {
                        m.bytes[m.offset(d0)] = 0;
                        m.bytes[m.offset(d1)] = 0;
                        m.put_word(ncell, 256);
                        status = fn_242444_m(m, handle, en, sz, off, d1, ncell);
                        if status >= 0x8000_0000 {
                            break;
                        }
                        if off >= sz {
                            break;
                        }
                        let t = m.word(ncell);
                        m.put_word(ncell, 256);
                        off += t;
                        status = fn_242444_m(m, handle, en, sz, off, d0, ncell);
                        if status >= 0x8000_0000 {
                            break;
                        }
                        if casecmp(m, d1, e.lit_version) == 0 {
                            let dst = if i7 == 0 { p + 0x2c } else { p + 0x12c };
                            strcpy(m, dst, d0);
                        }
                    }
                }
            } else {
                for (idx, want) in tags.iter().enumerate() {
                    if strcmp(m, tag, *want) == 0 {
                        status = fn_2413c4_m(
                            m, h, ha, hb, path, handle, e.open_out, e.stringmap, e.reader,
                        );
                        // Route the published reader to the matching slot.
                        m.put_word(slots[idx], m.word(e.open_out));
                        break;
                    }
                }
            }
            let rc2 = fn_2422ac_m(m, h, handle, e.reader);
            if rc2 >= 0x8000_0000 && status < 0x8000_0000 {
                status = rc2;
            }
        }
        if handle != 0 {
            let rc2 = fn_2421b0_m(m, h, handle, e.reader);
            if rc2 >= 0x8000_0000 && status < 0x8000_0000 {
                status = rc2;
            }
        }
    }
    status
}
