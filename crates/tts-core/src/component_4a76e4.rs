

use crate::engine;

pub struct Src;

pub struct Dst;

fn word(mem: &[u8], at: u32) -> u32 {
    u32::from_le_bytes(mem[at as usize..at as usize + 4].try_into().unwrap_or([0; 4]))
}
fn put_word(mem: &mut [u8], at: u32, v: u32) {
    if (at as usize) + 4 <= mem.len() {
        mem[at as usize..at as usize + 4].copy_from_slice(&v.to_le_bytes());
    }
}

pub fn fn_1831ac_m(
    mem: &mut [u8],
    d: u32,
    s: u32,
    host: &mut dyn FnMut(u32, &[u32]) -> u32,
) {
    let f5 = mem.get(d as usize + 0xf5).copied().unwrap_or(0xff);
    if f5 != 0 && f5 != 1 {
        return;
    }
    let fc = word(mem, d.wrapping_add(0xfc));
    let tbl = word(mem, fc);
    let entry = word(mem, tbl.wrapping_add(0x10));
    let r0 = host(entry, &[fc]);
    for i in 0..3u32 {
        put_word(mem, r0.wrapping_add(8 + i * 4), word(mem, s.wrapping_add(i * 4)));
    }
}

pub fn fn_330fcc_m(
    mem: &mut [u8],
    d: u32,
    v: f32,
    host: &mut dyn FnMut(u32, &[u32]) -> u32,
) {
    let ctx = word(mem, d.wrapping_add(0xf8));
    let tbl = word(mem, ctx);
    let entry = word(mem, tbl.wrapping_add(0x10));
    let r0 = host(entry, &[ctx]);
    let bits = v.to_bits();
    put_word(mem, r0.wrapping_add(4), bits);
}

pub fn fn_330f14_m(
    mem: &mut [u8],
    d: u32,
    v: f32,
    host: &mut dyn FnMut(u32, &[u32]) -> u32,
) {
    let ctx = word(mem, d.wrapping_add(0xf8));
    let tbl = word(mem, ctx);
    let entry = word(mem, tbl.wrapping_add(0x10));
    let r0 = host(entry, &[ctx]);
    let bits = v.to_bits();
    put_word(mem, r0.wrapping_add(8), bits);
}

pub fn fn_4a76e4_m(
    mem: &mut [u8],
    s: u32,
    d: u32,
    host: &mut dyn FnMut(u32, &[u32]) -> u32,
) -> u32 {
    put_word(mem, s.wrapping_add(0x30), word(mem, s.wrapping_add(0x38)).wrapping_add(1));
    let vt = word(mem, d);
    let entry15 = word(mem, vt.wrapping_add(0x3c));
    host(entry15, &[d, s.wrapping_add(0x18)]);
    fn_1831ac_m(mem, d, s.wrapping_add(0x24), host);
    put_word(mem, d.wrapping_add(0x150), word(mem, s.wrapping_add(0x44)));
    put_word(mem, d.wrapping_add(0x154), word(mem, s.wrapping_add(0x4c)));
    let f54 = f32::from_bits(word(mem, s.wrapping_add(0x54)));
    fn_330fcc_m(mem, d, f54, host);
    let f5c = f32::from_bits(word(mem, s.wrapping_add(0x5c)));
    fn_330f14_m(mem, d, f5c, host);
    0
}

pub fn dispatch_4a76e4(
    mem: &mut [u8],
    f: u32,
    a: &[u32],
    call: &mut dyn FnMut(u32, &[u32]) -> u32,
) -> Option<u32> {
    if f != 0x4a76e4 || a.len() < 2 {
        return None;
    }
    Some(fn_4a76e4_m(mem, a[0], a[1], call))
}
