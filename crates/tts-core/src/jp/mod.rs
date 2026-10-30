//! Japanese kana engine (Nintendo backend, `fn_19xx`/`fn_1a_xx`/`fn_6b_xx`).
//!
//! Caller-owned state: the engine is a borrowed 0x14000-byte block, voice
//! tables are borrowed. No globals, no allocation, no unsafe.
//!
//! Stored guest pointers (ring words at 0x8584..0x8594, voice-context words)
//! hold engine-relative offsets here (engine base = 0). Native/ARM hold
//! absolute addresses; the test normalizes by subtracting the engine base.

pub mod libc;
pub mod sjis;
pub mod sync;
pub mod voice;

/// Engine block size from `fn_19bc70` (`fn_301674(engine, 0x14000)`).
pub const ENGINE_SIZE: usize = 0x14000;

/// Former exclusion consts, removed in lane B round 3: every word is now
/// reproduced bit-exactly, so state compare is a pure link rebase with no
/// carve-outs. Mutex handles at 0xE6F8/0xE6FC are caller-lent entry-R1
/// echoes ([`sync::GuestSync`]); the job object at 0xE6D8..0xE6E0 and the
/// scheduler request struct at 0x13564..0x13578 plus `*r7` at 0x1357C are
/// built by [`sync::job_submit`] from the submit arguments.

/// Borrowed engine memory.
pub struct Engine<'a> {
    pub mem: &'a mut [u8; ENGINE_SIZE],
}

impl<'a> Engine<'a> {
    pub fn new(mem: &'a mut [u8; ENGINE_SIZE]) -> Self {
        Engine { mem }
    }
    #[inline]
    pub fn rd8(&self, off: usize) -> u8 {
        self.mem[off]
    }
    #[inline]
    pub fn rd16(&self, off: usize) -> u16 {
        u16::from_le_bytes([self.mem[off], self.mem[off + 1]])
    }
    #[inline]
    pub fn rd32(&self, off: usize) -> u32 {
        u32::from_le_bytes([
            self.mem[off],
            self.mem[off + 1],
            self.mem[off + 2],
            self.mem[off + 3],
        ])
    }
    #[inline]
    pub fn rds8(&self, off: usize) -> i8 {
        self.mem[off] as i8
    }

    pub fn rds16(&self, off: usize) -> i16 {
        i16::from_le_bytes([self.mem[off], self.mem[off + 1]])
    }
    #[inline]
    pub fn rds32(&self, off: usize) -> i32 {
        i32::from_le_bytes([
            self.mem[off],
            self.mem[off + 1],
            self.mem[off + 2],
            self.mem[off + 3],
        ])
    }
    #[inline]
    pub fn wr8(&mut self, off: usize, v: u8) {
        self.mem[off] = v;
    }
    #[inline]
    pub fn wr16(&mut self, off: usize, v: u16) {
        let b = v.to_le_bytes();
        self.mem[off] = b[0];
        self.mem[off + 1] = b[1];
    }
    #[inline]
    pub fn wr32(&mut self, off: usize, v: u32) {
        let b = v.to_le_bytes();
        self.mem[off] = b[0];
        self.mem[off + 1] = b[1];
        self.mem[off + 2] = b[2];
        self.mem[off + 3] = b[3];
    }
    #[inline]
    pub fn zero_range(&mut self, off: usize, len: usize) {
        self.mem[off..off + len].fill(0);
    }
}

/// fn_19bc70: constructor. Zeroes the block, writes defaults, creates two
/// mutexes (caller-lent entry-R1 echoes, 0 under the harness) and submits
/// the deferred job object (stubbed-wait out-slot), then runs the param
/// reset. `sync` carries the guest facts the engine cannot derive.
pub fn construct(eng: &mut Engine, sync: sync::GuestSync) -> i32 {
    eng.mem.fill(0);
    eng.wr32(0x0000, 0x2AF8);
    eng.wr16(0x0004, 55);
    eng.wr8(0xE730, 0);
    eng.wr16(0xE732, 0x13C2);
    eng.wr16(0xE734, 100);
    eng.wr16(0xE736, 100);
    eng.wr8(0x1B21, 0);
    eng.wr16(0xE73A, 120);
    eng.wr16(0xE73C, 30);
    eng.wr16(0xE738, 0);
    eng.wr16(0xE73E, 0);
    eng.wr16(0x85A0, 0);
    eng.wr32(0xE748, 0);
    eng.wr32(0xE74C, 0);
    eng.wr32(0xE750, 0);
    eng.wr32(0xAFB8, 0);
    eng.wr16(0x81C2, 0);
    eng.wr16(0x8598, 480);
    eng.wr16(0x859A, 240);
    eng.wr32(0x859C, 0);
    eng.wr32(0xAFDC, 0);
    eng.wr16(0x7FDE, 0);
    eng.wr8(0xAFE4, 0);
    eng.wr8(0xAFE5, 0);
    eng.wr8(0xAFE6, 1);
    eng.wr8(0xAFE7, 1);
    eng.wr32(0xAFE8, 0);
    eng.wr32(0xAFEC, 0);
    eng.wr32(0xAFF0, 0);
    eng.wr32(0xAFF4, 0);
    eng.wr32(0xDEE0, 0);
    eng.wr32(0xE6BC, 0);
    eng.wr32(0xE6C0, 0);
    eng.wr32(0xE6C4, 0);
    eng.wr16(0xE6C8, 0);
    eng.wr8(0xE6CB, 0);
    eng.wr8(0xE6CC, 0);
    eng.wr32(0xE6D0, 0);
    eng.wr32(0xE754, 100);
    eng.wr16(0xE758, 1024);
    eng.wr16(0xE75A, 10);
    eng.wr32(0xE6D4, 1);
    // fn_6b1760 x2: the harness echoes entry R1 into the handle word
    // (both call sites pass flags 0, so 0); the caller lends the echoes.
    eng.wr32(0xE6F8, sync.mutex_a);
    eng.wr32(0xE6FC, sync.mutex_b);
    eng.wr8(0xE760, 0);
    eng.wr8(0xAFE9, 0);
    reset_params(eng);
    // fn_6b18b0(engine+0xE6D8, &D_6aebe8, 0, engine+0x13584, tmo 2/-1
    // on dead stack): SyncObj words plus the scheduler request struct.
    sync::job_submit(
        &mut eng.mem[..],
        0xE6D8,
        sync,
        sync::JobSubmit {
            cb: 0x6AEBE8,
            arg: 0,
            out_off: 0x13584,
        },
    );
    0
}

/// fn_1a2c90: param-block reset at engine+0xE700.
pub fn reset_params(eng: &mut Engine) {
    eng.wr8(0xE700, 0);
    eng.wr16(0xE702, 0x13C2);
    eng.wr16(0xE704, 100);
    eng.wr16(0xE706, 100);
    eng.wr16(0xE70A, 100);
    eng.wr16(0xE708, 0);
    eng.wr32(0xE724, 100);
    eng.wr16(0xE728, 1024);
    eng.wr16(0xE72A, 10);
    eng.wr16(0xE714, 0);
    eng.wr32(0xE710, 0);
    eng.wr16(0xE70E, 0);
    eng.wr16(0xE70C, 0);
    eng.wr32(0xE71C, 0);
    eng.wr16(0xE716, 0);
    eng.wr32(0xE720, 0);
    eng.wr32(0xE718, 0);
    eng.wr16(0xE72C, 0);
}

/// fn_19c2c8: reset entry, runs the param reset.
pub fn reset(eng: &mut Engine) -> i32 {
    reset_params(eng);
    0
}

/// fn_19cd9c: clears the 0xDEE0 flag and the 0xAFFC block.
pub fn clear_affc(eng: &mut Engine) {
    eng.wr32(0xDEE0, 0);
    eng.zero_range(0xAFFC, 0x2EE4);
}

/// fn_19cdc0: clears the 0xE6BC flag and the 0xDEE8 block.
pub fn clear_dee8(eng: &mut Engine) {
    eng.wr32(0xE6BC, 0);
    eng.zero_range(0xDEE8, 0x7D2);
}

/// fn_19de90: clamp with s16 narrowing on the in-range path only.
pub fn clamp_s16(v: i32, lo: i32, hi: i32) -> i32 {
    if v < lo {
        return lo;
    }
    if v > hi {
        return hi;
    }
    (v as i16) as i32
}

/// fn_19e26c: s16 at engine+0x7FD8.
pub fn peek_7fd8(eng: &Engine) -> i16 {
    eng.rds16(0x7FD8)
}

/// fn_1a4e1c: busy flag at 0xE6CB (lock/unlock elided single-threaded).
pub fn flag_busy(eng: &Engine) -> u8 {
    eng.rd8(0xE6CB)
}

/// fn_1a4e50: done flag at 0xE6CC (lock/unlock elided single-threaded).
pub fn flag_done(eng: &Engine) -> u8 {
    eng.rd8(0xE6CC)
}

/// fn_19e1e8: 16-step fixed-point ratio with s16 saturation.
///
/// Takes the full r1 register: ARM compares the unnarrowed word (no `sxth`
/// on entry; narrowing happens only after negation at +0x2c). Real callers
/// pass sign-extended s16, where narrowed and raw agree; fuzz passes garbage
/// upper bits (ABI difference, Rust follows ARM).
pub fn fixed_ratio(a: i32, r1: u32) -> i16 {
    let mut sign = 1i32;
    let mut x = a;
    let mut y = r1 as i32;
    if y == -0x8000 {
        y = sign - 0x8000;
    }
    if x < 0 {
        x = x.wrapping_neg();
        sign = -1;
    }
    if y < 0 {
        y = (y.wrapping_neg()) as i16 as i32;
        sign = -sign;
    }
    let t = if sign > 0 {
        y.wrapping_add((y as u32 >> 31) as i32)
    } else {
        let u = y.wrapping_sub(1);
        u.wrapping_add((u as u32 >> 31) as i32)
    };
    let d = y.wrapping_shl(15);
    x = x.wrapping_add(t >> 1);
    for _ in 0..16 {
        let diff = x.wrapping_sub(d);
        if diff >= 0 {
            x = 1i32.wrapping_add(diff.wrapping_shl(1));
        } else {
            x = x.wrapping_shl(1);
        }
    }
    if (x & 0x8000) == 0 {
        ((x as u16 as i32).wrapping_mul(sign)) as i16
    } else {
        (sign.wrapping_mul(0x7fff)) as i16
    }
}

/// fn_19dfd8: vowel-letter count over a NUL-terminated byte string.
pub fn vowel_count(s: &[u8]) -> i16 {
    let mut n: i16 = 0;
    let mut i = 0usize;
    if s.is_empty() || s[0] == 0 {
        return 0;
    }
    loop {
        let c = s[i] as i8;
        let hit = if c == b'Q' as i8 {
            true
        } else if c <= b'Q' as i8 {
            if c == b'I' as i8 {
                true
            } else if c <= b'I' as i8 {
                c == b'-' as i8 || c == b'A' as i8 || c == b'E' as i8
            } else {
                c == b'N' as i8 || c == b'O' as i8
            }
        } else if c == b'i' as i8 {
            true
        } else if c <= b'i' as i8 {
            c == b'U' as i8 || c == b'a' as i8 || c == b'e' as i8
        } else {
            c == b'o' as i8 || c == b'u' as i8
        };
        if hit {
            n = n.wrapping_add(1);
        }
        i += 1;
        if i >= s.len() || s[i] == 0 {
            break;
        }
    }
    n
}

fn strcmp_eq(a: &[u8], b: &[u8]) -> bool {
    let mut i = 0usize;
    loop {
        let ca = *a.get(i).unwrap_or(&0);
        let cb = *b.get(i).unwrap_or(&0);
        if ca != cb {
            return false;
        }
        if ca == 0 {
            return true;
        }
        i += 1;
    }
}

/// fn_19cde4: phoneme-class lookup over 38 borrowed ROM strings.
pub fn class_lookup(s: &[u8], table: [&[u8]; 38]) -> u8 {
    let mut i: i16 = 0;
    loop {
        if strcmp_eq(s, table[i as usize]) {
            return i as u8;
        }
        if strcmp_eq(s, table[(i + 1) as usize]) {
            return (i + 1) as u8;
        }
        i += 2;
        if i >= 38 {
            break;
        }
    }
    if strcmp_eq(s, b"pau\0") {
        0
    } else {
        7
    }
}

/// fn_19fea4: fill n bytes with `(u8)(s8)val`; returns end offset.
///
/// ARM returns `dst` unchanged for n <= 0 (ROM `ble` to return); the C
/// `return dst + n` differs for negative n (defect filed).
pub fn memfill(mem: &mut [u8], dst: usize, val: i32, n: i32) -> usize {
    if n <= 0 {
        return dst;
    }
    let c = (val as i8) as u8;
    for i in 0..n as usize {
        mem[dst + i] = c;
    }
    dst.wrapping_add(n as usize)
}

/// fn_1a4e84 single-element step (also used for overlapping buffers).
#[inline]
pub fn sat_scale_one(v: i16, scale: i32) -> i16 {
    let mut x = (v as i32).wrapping_mul(scale) >> 12;
    if x >= 0x8000 {
        x = 0x7fff;
    } else if x <= -0x8000 {
        x = -0x7fff;
    }
    x as i16
}

/// fn_1a4e84: saturating scale of s16 vectors over slices.
pub fn sat_scale(dst: &mut [i16], src: &[i16], n: i32, scale: i32) {
    let mut k = n;
    let mut i = 0usize;
    while {
        let cont = k > 0;
        k = k.wrapping_sub(1);
        cont
    } {
        dst[i] = sat_scale_one(src[i], scale);
        i += 1;
    }
}

/// fn_1a4e84 on engine buffers.
pub fn sat_scale_engine(eng: &mut Engine, dst: usize, src: usize, n: i32, scale: i32) {
    let mut k = n;
    let mut i = 0usize;
    while {
        let cont = k > 0;
        k = k.wrapping_sub(1);
        cont
    } {
        let v = eng.rds16(src + i * 2);
        let o = sat_scale_one(v, scale);
        let b = o.to_le_bytes();
        eng.mem[dst + i * 2] = b[0];
        eng.mem[dst + i * 2 + 1] = b[1];
        i += 1;
    }
}

/// fn_19df84: rate setup at base+0x200 with clamping divide.
pub fn rate_setup(eng: &mut Engine, base: usize, n: i32) {
    let d = if n <= 0 { 1 } else { n };
    let q = 76800i32.wrapping_div(d);
    let mut v = q.wrapping_shl(5).wrapping_div(256);
    if v == 0 {
        v = 1;
    }
    eng.wr32(base + 0x200, v as u32);
    eng.wr32(base + 0x204, 0);
    eng.wr32(base + 0x208, 0);
    eng.wr32(base + 0x20C, 0);
    eng.zero_range(base, 0x200);
}

/// fn_19fedc: strsep-like tokenizer over a caller buffer.
/// `ctx` is an offset into `buf`; `acc` holds the separator bytes.
/// Returns the token offset into `buf`.
pub fn tokenize(buf: &mut [u8], ctx: &mut usize, acc: &[u8]) -> Option<usize> {
    let mut pos = *ctx;
    if buf[pos] == 0 {
        return None;
    }
    // Phase 1: skip leading acc bytes. ARM reloads the token base from *ctx
    // after the skip (r5 reload at +0x4); the C keeps the entry pos (defect).
    let mut tok = pos;
    loop {
        let c = buf[pos];
        let mut a = 0usize;
        loop {
            if acc[a] == 0 {
                // Phase 2: scan the token.
                tok = pos;
                if buf[pos] == 0 {
                    return None;
                }
                loop {
                    pos = *ctx;
                    let d = buf[pos];
                    let mut b = 0usize;
                    loop {
                        let e = acc[b];
                        if e == 0 {
                            break;
                        }
                        if e == d {
                            if buf[pos] == 0 {
                                return Some(tok);
                            }
                            *ctx = pos + 1;
                            buf[pos] = 0;
                            return Some(tok);
                        }
                        b += 1;
                    }
                    *ctx = pos + 1;
                    if buf[pos + 1] == 0 {
                        return Some(tok);
                    }
                }
            }
            if acc[a] == c {
                break;
            }
            a += 1;
        }
        pos += 1;
        *ctx = pos;
        if buf[pos] == 0 {
            return None;
        }
    }
}

/// fn_19deac delay-line state (528 bytes in C order).
#[derive(Clone, Copy)]
pub struct DeacState {
    pub table: [i16; 256],
    pub limit: i32,
    pub idx: i32,
    pub n2: i32,
    pub pos: i32,
}

impl DeacState {
    pub fn load(mem: &[u8], off: usize) -> Self {
        let mut table = [0i16; 256];
        for (i, slot) in table.iter_mut().enumerate() {
            *slot = i16::from_le_bytes([mem[off + i * 2], mem[off + i * 2 + 1]]);
        }
        let rd = |o: usize| i32::from_le_bytes([mem[o], mem[o + 1], mem[o + 2], mem[o + 3]]);
        DeacState {
            table,
            limit: rd(off + 512),
            idx: rd(off + 516),
            n2: rd(off + 520),
            pos: rd(off + 524),
        }
    }
    pub fn store(&self, mem: &mut [u8], off: usize) {
        for (i, v) in self.table.iter().enumerate() {
            let b = v.to_le_bytes();
            mem[off + i * 2] = b[0];
            mem[off + i * 2 + 1] = b[1];
        }
        let wr = |mem: &mut [u8], o: usize, v: i32| {
            let b = v.to_le_bytes();
            mem[o] = b[0];
            mem[o + 1] = b[1];
            mem[o + 2] = b[2];
            mem[o + 3] = b[3];
        };
        wr(mem, off + 512, self.limit);
        wr(mem, off + 516, self.idx);
        wr(mem, off + 520, self.n2);
        wr(mem, off + 524, self.pos);
    }
}

/// One fn_19deac sample step.
pub fn deac_sample(state: &mut DeacState, x: i16) -> i16 {
    let mut k = state.n2;
    state.table[state.idx as usize] = x;
    if k >= 256 {
        k = 511 - k;
    }
    let t0 = ((state.idx.wrapping_sub(k)) as usize) & 255;
    let mut v = (state.table[t0] >> 1) as i32;
    let t1 = ((state.idx.wrapping_sub(255 - k)) as usize) & 255;
    v += (state.table[t1] >> 1) as i32;
    let j = state.idx.wrapping_add(1);
    state.idx = if j >= 256 { 0 } else { j };
    let p = state.pos.wrapping_add(32);
    state.pos = p;
    if p >= state.limit {
        let m = state.n2.wrapping_add(1);
        state.n2 = if m >= 511 { 0 } else { m };
        state.pos = 0;
    }
    v as i16
}

/// fn_19deac over slices.
pub fn deac_run(state: &mut DeacState, input: &[i16], output: &mut [i16], n: i32) {
    let mut i = 0i32;
    while i < n {
        output[i as usize] = deac_sample(state, input[i as usize]);
        i += 1;
    }
}

/// fn_19deac on engine buffers (state/in/out are engine offsets).
pub fn deac_engine(eng: &mut Engine, state: usize, inp: usize, out: usize, n: i32) {
    let mut st = DeacState::load(&eng.mem[..], state);
    let mut i = 0i32;
    while i < n {
        let x = i16::from_le_bytes([
            eng.mem[inp + i as usize * 2],
            eng.mem[inp + i as usize * 2 + 1],
        ]);
        let y = deac_sample(&mut st, x);
        let b = y.to_le_bytes();
        eng.mem[out + i as usize * 2] = b[0];
        eng.mem[out + i as usize * 2 + 1] = b[1];
        i += 1;
    }
    st.store(&mut eng.mem[..], state);
}

/// fn_6aeb50: ring silence with wrap at the 0x8594 limit.
pub fn ring_silence(eng: &mut Engine, count: u32, dst: u32) {
    let mut c = count as i32;
    let mut d = dst;
    let end = eng.rd32(0x8594);
    if d.wrapping_add((c.wrapping_mul(2)) as u32) >= end {
        while d < end && c > 0 {
            eng.wr16(d as usize, 0);
            d = d.wrapping_add(2);
            c -= 1;
        }
        d = eng.rd32(0x8590);
    }
    let mut i = 0i32;
    while i < c {
        let a = d.wrapping_add((i.wrapping_mul(2)) as u32) as usize;
        eng.wr16(a, 0);
        i += 1;
    }
}

/// fn_19efb0: ring setup; pointer words hold engine-relative offsets.
pub fn ring_init(eng: &mut Engine) {
    eng.wr32(0x8590, 0x81C4);
    eng.wr32(0x8594, 0x8584);
    eng.wr16(0x8598, 480);
    eng.wr16(0x859A, 240);
    eng.wr32(0x858C, 0x81C4);
    eng.wr32(0x8584, 0x81C4);
    eng.wr32(0x8588, 0x81C4);
    eng.wr32(0x859C, 0);
    ring_silence(eng, 480, 0x81C4);
}

/// fn_1a2bf0: start-path state init.
pub fn start_init(eng: &mut Engine) -> i32 {
    eng.wr16(0x41F2, 0);
    eng.wr16(0xE6C8, 0);
    eng.wr8(0xE6CA, 0);
    ring_init(eng);
    eng.wr32(0xAFBC, 0);
    memfill(&mut eng.mem[..], 0xAFC0, 0, 24);
    eng.wr16(0x7FDE, 0);
    eng.wr32(0xE6C0, 0);
    eng.wr32(0xE6C4, 0);
    eng.wr16(0x7FDE + 242 * 2, 0);
    eng.wr32(0xAFB8, 0);
    eng.wr32(0xAC00 + 251 * 4, 0);
    eng.wr32(0xAC00 + 252 * 4, 0);
    eng.wr32(0xAC00 + 253 * 4, 0);
    eng.wr32(0xAC00 + 254 * 4, 0);
    0
}

/// fn_1a2d28: shadow-copy of the 0xE700 params with conditional clears.
pub fn shadow_copy(eng: &mut Engine) {
    let b0 = eng.rd8(0xE700);
    eng.wr8(0xE730, b0);
    eng.wr16(0xE732, eng.rd16(0xE702));
    eng.wr16(0xE734, eng.rd16(0xE704));
    eng.wr16(0xE736, eng.rd16(0xE706));
    eng.wr16(0xE73A, eng.rd16(0xE70A));
    eng.wr16(0xE738, eng.rd16(0xE708));
    eng.wr32(0xE754, eng.rd32(0xE724));
    eng.wr16(0xE758, eng.rd16(0xE728));
    eng.wr16(0xE75A, eng.rd16(0xE72A));
    eng.wr16(0xE714 + 0x30, eng.rd16(0xE714));
    eng.wr32(0xE740, eng.rd32(0xE710));
    let a = eng.rds16(0xE70E);
    eng.wr16(0xE73E, a as u16);
    if a != 0 {
        clear_affc(eng);
    }
    eng.wr16(0xE73C, eng.rd16(0xE70C));
    let w = eng.rd32(0xE71C);
    eng.wr32(0xE74C, w);
    let v = eng.rds16(0xE716);
    eng.wr16(0xE716 + 0x30, v as u16);
    if v != 0 || w != 0 {
        clear_dee8(eng);
    }
    eng.wr32(0xE750, eng.rd32(0xE720));
    eng.wr32(0xE748, eng.rd32(0xE718));
    eng.wr16(0xE72C + 0x30, eng.rd16(0xE72C));
}

/// fn_1a2e38: quality setter into the selected param slot.
pub fn quality_set(eng: &mut Engine, v: i32, sel: i32) {
    let mut x = v;
    if x < 80 {
        x = 80;
    } else if x > 200 {
        x = 200;
    }
    let scaled = (x.wrapping_mul(0xA3C00) >> 16) as i16;
    let ratio = fixed_ratio(1000, x as u32);
    if sel != 0 {
        eng.wr32(0xE754, x as u32);
        eng.wr16(0xE758, scaled as u16);
        eng.wr16(0xE75A, ratio as u16);
        return;
    }
    eng.wr32(0xE724, x as u32);
    eng.wr16(0xE728, scaled as u16);
    eng.wr16(0xE72A, ratio as u16);
}

#[inline]
fn mix_sat(a: i16, b: i16) -> i16 {
    let v = (a as i32).wrapping_add(b as i32);
    if v >= 32768 {
        0x7fff
    } else if v <= -32768 {
        -0x7fff
    } else {
        v as i16
    }
}

/// fn_1a19c4: PCM ring mixer with s16 saturation and wrap.
///
/// Offset-pointer constraint: `(b as u32 & 0x7FFFFFFF) < 0x78000000`, so
/// `w + (b << 1)` keeps its order against the limit in both offset and
/// absolute space. Real calls use b < 480; only the fuzz window near 2^31
/// is excluded.
pub fn mix_pcm(eng: &mut Engine, a: i32, b: i32, pcm: &mut PcmLog) {
    let total = eng.rds16(0x7FE0) as i32;
    let orig_w = eng.rd32(0x8584);
    let mut w = orig_w;
    let base = eng.rd32(0x8590);
    let lim = eng.rd32(0x8594);
    if a > 0 && b > 0 {
        let half = total >> 1;
        w = w.wrapping_sub((half << 1) as u32);
    }
    let mut unit = 0x7FE2usize;
    if base > w {
        w = lim.wrapping_sub(base.wrapping_sub(w) & !1);
    }
    let mut ring = w;
    let mut n: i16 = 0;
    if w.wrapping_add((total.wrapping_mul(2)) as u32) >= lim {
        if lim > w {
            loop {
                let r = eng.rds16(ring as usize);
                let u = eng.rds16(unit);
                let m = mix_sat(r, u);
                eng.wr16(ring as usize, m as u16);
                pcm.push(m as i16);
                n = n.wrapping_add(1);
                unit += 2;
                ring = ring.wrapping_add(2);
                if eng.rd32(0x8594) <= ring {
                    break;
                }
            }
        }
        ring = base;
    }
    let rem = total.wrapping_sub(n as i32);
    if rem > 0 {
        let mut n2 = rem;
        loop {
            let r = eng.rds16(ring as usize);
            let u = eng.rds16(unit);
            let m = mix_sat(r, u);
            eng.wr16(ring as usize, m as u16);
            pcm.push(m as i16);
            unit += 2;
            n2 -= 1;
            ring = ring.wrapping_add(2);
            if n2 == 0 {
                break;
            }
        }
    }
    let v: u32;
    if a == 0 {
        v = orig_w.wrapping_add((b.wrapping_shl(1)) as u32);
        eng.wr32(0x8588, v);
    } else {
        eng.wr32(0x8588, ring);
        v = ring;
    }
    let lim2 = eng.rd32(0x8594);
    if v >= lim2 {
        let b3 = eng.rd32(0x8590);
        let nv = v.wrapping_sub(lim2) & !1;
        eng.wr32(0x8588, nv.wrapping_add(b3));
    }
}

#[inline]
fn fp_step(coef: i32, x: i32) -> i32 {
    let prod = (coef as i64).wrapping_mul(x as i64);
    let hi = (prod >> 32) as i32;
    (hi >> 8) - (hi >> 31)
}

#[inline]
fn div100(x: i32) -> i32 {
    let m = ((x as i64).wrapping_mul(0x51EB851F) >> 32) as i32;
    (m >> 5) - (m >> 31)
}

fn table_s16(eng: &Engine, idx: i32) -> i16 {
    let addr = 0xDEE8i32.wrapping_add(idx.wrapping_mul(2));
    eng.rds16(addr as usize)
}

fn table_store(eng: &mut Engine, idx: i32, v: i16) {
    let addr = 0xDEE8i32.wrapping_add(idx.wrapping_mul(2));
    eng.wr16(addr as usize, v as u16);
}

/// fn_19ffa0: fixed-point resample/filter loop over engine buffers.
pub fn resample_filter(eng: &mut Engine, n: i32, x: usize, y: usize) {
    let coef = 0x4178749Fi32;
    let e74c = eng.rd32(0xE74C);
    let mult = div100((e74c << 8) as i32);
    if n <= 0 {
        return;
    }
    let mut ip = 0i32;
    loop {
        let c = eng.rds32(0xE6BC);
        let thr = eng.rds16(0xE746);
        let sample = eng.rds16(x.wrapping_add((ip as usize).wrapping_mul(2)));
        let v: i32;
        if (thr as i32) <= c {
            let t = c.wrapping_sub(thr as i32);
            let fp = fp_step(coef, t);
            let tt = t.wrapping_add(fp.wrapping_mul(-1001));
            let acc = table_s16(eng, tt) as i32;
            let mut vv = (sample as i32).wrapping_add((acc.wrapping_mul(mult)) >> 8);
            if vv >= 0x8000 {
                vv = 0x7fff;
            } else if vv <= -0x8000 {
                vv = -0x8000;
            }
            v = vv;
        } else {
            v = sample as i32;
        }
        eng.wr16(
            y.wrapping_add((ip as usize).wrapping_mul(2)),
            v as i16 as u16,
        );
        let c2 = eng.rds32(0xE6BC);
        let wi = c2.wrapping_add(fp_step(coef, c2).wrapping_mul(-1001));
        table_store(eng, wi, sample);
        eng.wr32(0xE6BC, (c2.wrapping_add(1)) as u32);
        ip += 1;
        if ip >= n {
            return;
        }
    }
}


fn s16_at(mem: &[u8], base: usize, idx: i32) -> i16 {
    let o = (base as i64 + idx as i64 * 2) as usize;
    i16::from_le_bytes([mem[o], mem[o + 1]])
}

fn put_s16(mem: &mut [u8], base: usize, idx: i32, v: i16) {
    let o = (base as i64 + idx as i64 * 2) as usize;
    mem[o..o + 2].copy_from_slice(&v.to_le_bytes());
}

#[inline]
fn u16_at(mem: &[u8], off: usize) -> u16 {
    u16::from_le_bytes([mem[off], mem[off + 1]])
}

#[inline]
fn put_u16(mem: &mut [u8], off: usize, v: u16) {
    mem[off..off + 2].copy_from_slice(&v.to_le_bytes());
}

#[inline]
fn s8_at(mem: &[u8], base: usize, off: i64) -> i8 {
    mem[(base as i64 + off) as usize] as i8
}

/// fn_1a07e0 (cont, same TU as fn_1a0744): ramp writer. `row` is the s16
/// index of the target row from `p`. Returns `q` for `idx <= 0`, else the
/// ARM falloff R0 (`r6 - 1` at loop exit: 0 unless `idx ≡ 1 mod 0x10000`).
fn interp_ramp(
    mem: &mut [u8],
    p: usize,
    a: i32,
    max: i32,
    b: i32,
    idx: i32,
    row: i32,
) -> i32 {
    let max = max as i16 as i32;
    let mut step =
        fixed_ratio(b.wrapping_sub(a).wrapping_shl(11), idx.wrapping_sub(1) as u32) as i32;
    step = step.wrapping_shl(5);
    let q = step.wrapping_add(0x640000);
    let b16 = b.wrapping_shl(16);
    let mut ramp = b16;
    let base = b16.wrapping_sub(step);
    if (q as u32) > 0xc80000 {
        step = if step <= 0 { 0xff9c0000u32 as i32 } else { 0x640000 };
    }
    if idx <= 0 {
        return q;
    }
    let mut w = row.wrapping_sub(5);
    let mut rc = row as i64 * 2 - 8;
    let mut ip = 0i32;
    let mut r6 = idx;
    loop {
        let ctrl = s8_at(mem, p, rc) as i32;
        let mut v = s16_at(mem, p, w) as i32;
        rc -= 10;
        if ctrl <= 0 {
            v += ramp >> 16;
        } else {
            let t = b16.wrapping_sub(step.wrapping_mul(ip));
            v += t >> 16;
        }
        v = v as i16 as i32;
        if v < 0 {
            v = 0;
        }
        if v > max {
            v = max;
        }
        put_s16(mem, p, w, v as i16);
        if ctrl > 0 {
            ramp = base.wrapping_sub(step.wrapping_mul(ip));
        }
        w = w.wrapping_sub(5);
        ip = ip.wrapping_add(1);
        let r0 = r6.wrapping_sub(1);
        r6 = (r0 as i16) as i32;
        if r6 == 0 {
            return r0;
        }
    }
}

/// fn_1a0780: clamp `a`/`b` into ±400, then run the ramp writer.
/// `p` is the byte offset of the s16 table; caller keeps `idx` small and
/// five guard rows addressable below `p + idx*5`.
pub fn interp_clamp(mem: &mut [u8], p: usize, idx: i32, a: i32, b: i32) -> i32 {
    let mut a = a;
    let mut b = b;
    if (a.wrapping_add(400) as u32) > 800 {
        a = if a > 0 { 400 } else { -400 };
    }
    if (b.wrapping_add(400) as u32) > 800 {
        b = if b > 0 { 400 } else { -400 };
    }
    let row = idx.wrapping_mul(5);
    let x = s16_at(mem, p, row.wrapping_sub(5)) as i32 + b;
    let y = s16_at(mem, p, 0) as i32 + a;
    let max = if y > x { y } else { x };
    interp_ramp(mem, p, a, max, b, idx, row)
}

/// fn_1a0744: normalize `a`/`b` against the table, then `fn_1a0780`.
pub fn interp_setup(mem: &mut [u8], p: usize, idx: i32, a: i32, b: i32) -> i32 {
    let a = if a >= 0 {
        a.wrapping_sub(s16_at(mem, p, 0) as i32) as i16 as i32
    } else {
        0
    };
    let row = idx.wrapping_mul(5);
    let b = if b < 0 {
        a
    } else {
        b.wrapping_sub(s16_at(mem, p, row.wrapping_sub(5)) as i32) as i16 as i32
    };
    interp_clamp(mem, p, idx, a, b)
}

/// fn_1a1c44: block shuffle between 0x7D00/0x7F00 windows; returns `p`.
/// Loop-A copies 0x7DF6+2k into 0x7FE0+2k (bound: s16 at 0x7FDC),
/// loop-B the same with the bound at 0x7DF6. Caller keeps the bounds
/// small: the ROM faults past the mapping for huge counts just the same.
pub fn block_shuffle(mem: &mut [u8], p: usize) -> usize {
    let b1 = p + 0x7f00;
    let b2 = p + 0x7d00;
    let n = i16::from_le_bytes([mem[b1 + 220], mem[b1 + 221]]);
    if n > 0 {
        let m = i16::from_le_bytes([mem[b1 + 216], mem[b1 + 217]]);
        if m > 0 {
            if (n as i32) + 1 <= 0 {
                return p;
            }
            let mut c = 1u32 & !(n as u32);
            let mut k = 0u32;
            if c == 1 {
                put_u16(mem, b1 + 224, u16_at(mem, b2 + 246));
                k = 1;
            }
            if (n as i32) + 1 > c as i32 {
                loop {
                    let q = p + (k.wrapping_mul(2)) as usize;
                    put_u16(mem, q + 0x7fe0, u16_at(mem, q + 0x7df6));
                    k = k.wrapping_add(1);
                    let q = p + (k.wrapping_mul(2)) as usize;
                    c = c.wrapping_add(2);
                    put_u16(mem, q + 0x7fe0, u16_at(mem, q + 0x7df6));
                    k = k.wrapping_add(1);
                    if i16::from_le_bytes([mem[b1 + 220], mem[b1 + 221]]) as i32 + 1
                        <= c as i32
                    {
                        break;
                    }
                }
            }
            return p;
        }
        if m != 0 {
            put_u16(mem, b1 + 224, 0);
            return p;
        }
    } else if n == 0 {
        if i16::from_le_bytes([mem[b1 + 216], mem[b1 + 217]]) != 0 {
            put_u16(mem, b1 + 224, 0);
            return p;
        }
    }
    let kk = i16::from_le_bytes([mem[b2 + 246], mem[b2 + 247]]);
    if (kk as i32) + 1 <= 0 {
        return p;
    }
    let mut c = 1u32 & !(kk as u32);
    let mut k = 0u32;
    if c == 1 {
        k = 1;
        put_u16(mem, b1 + 224, kk as u16);
    }
    if (kk as i32) + 1 <= c as i32 {
        return p;
    }
    loop {
        let q = p + (k.wrapping_mul(2)) as usize;
        put_u16(mem, q + 0x7fe0, u16_at(mem, q + 0x7df6));
        k = k.wrapping_add(1);
        let q = p + (k.wrapping_mul(2)) as usize;
        c = c.wrapping_add(2);
        put_u16(mem, q + 0x7fe0, u16_at(mem, q + 0x7df6));
        k = k.wrapping_add(1);
        if i16::from_le_bytes([mem[b2 + 246], mem[b2 + 247]]) as i32 + 1 <= c as i32 {
            break;
        }
    }
    p
}

/// fn_19ca00: expose engine for render; `tid` is the borrowed thread word.
/// When the busy flag was set and the owner differs, the ROM spins on the
/// flag (nothing in the loop clears it), so that shape hangs on ARM too
/// and the harness only fuzzes shapes that return.
pub fn engine_expose(eng: &mut Engine, tid: u32) -> i32 {
    let mut changed = 0;
    if eng.mem[0xE6CB] == 1 {
        eng.mem[0xE6CC] = 1;
        changed = 1;
    }
    if changed != 1 {
        return 0;
    }
    if sync::owner_query(eng.mem, 0xE6D8) == tid {
        return 0;
    }
    loop {
        if flag_busy(eng) != 1 {
            return 0;
        }
        sync::sleep_ticks(0x989680, 0);
    }
}

/// fn_19c9e0: voice detach guard (no observable effect under stubbed SVCs;
/// verified by the row), flag the voice, expose its engine at +0xA0.
/// The test resolves the engine pointer to `eng`.
pub fn voice_expose(mem: &mut [u8], voice: usize, eng: &mut Engine, tid: u32) -> i32 {
    mem[voice + 0xB5] = 1;
    engine_expose(eng, tid)
}

/// fn_19caa8: post-render sync; `tid` is the borrowed thread word.
/// Same spin-or-return shape as [`engine_expose`], then a zero-compare of
/// the job object that re-arms the event and clears the owner words.
pub fn render_sync(eng: &mut Engine, tid: u32) -> i32 {
    if flag_busy(eng) == 1 {
        let mut r5 = 0;
        if eng.mem[0xE6CB] == 1 {
            eng.mem[0xE6CC] = 1;
            r5 = 1;
        }
        if r5 == 1 && sync::owner_query(eng.mem, 0xE400 + 728) != tid {
            loop {
                if flag_busy(eng) != 1 {
                    break;
                }
                sync::sleep_ticks(0x989680, 0);
            }
        }
    }
    let zero = [0u8; 8];
    if libc::memcmp0(&eng.mem[0xE6D8..0xE6E0], &zero, 8) != 0 {
        sync::event_wait(&mut eng.mem[..], 0xE6D8, 0);
        sync::event_wait_signal(eng, 0xE6D8, 0);
        eng.wr32(0xE400 + 728, 0);
        eng.wr32(0xE400 + 732, 0);
    }
    0
}

// --- batch B: voice-render chain (ROM dots, filters, per-mora) ---

/// fn_1933a4/fn_193408: N-tap smlawb dot over delay words `x` (u32 LE at
/// `xo`) and s16 coeffs `h`: `acc += (x[i]*h[i]) >> 16` with u32 wrap.
pub fn smlawb_dot(xb: &[u8], xo: usize, h: &[i16], taps: usize) -> i32 {
    let mut acc = 0u32;
    for i in 0..taps {
        let o = xo + i * 4;
        let xi = i32::from_le_bytes([xb[o], xb[o + 1], xb[o + 2], xb[o + 3]]);
        let p = ((xi as i64 * h[i] as i64) >> 16) as i32 as u32;
        acc = acc.wrapping_add(p);
    }
    acc as i32
}

/// fn_193330: 5-tap dot with `v` spliced at tap 2:
/// taps are `(x0,h0) (x1,h1) (v,h2) (x2,h3) (x3,h4)`, u32 wrap.
pub fn dot5(xb: &[u8], xo: usize, h: &[i16], v: i32) -> i32 {
    let mut acc = 0u32;
    for i in 0..5 {
        let s = if i == 2 {
            v
        } else {
            let o = xo + (if i < 2 { i } else { i - 1 }) * 4;
            i32::from_le_bytes([xb[o], xb[o + 1], xb[o + 2], xb[o + 3]])
        };
        let p = ((s as i64 * h[i] as i64) >> 16) as i32 as u32;
        acc = acc.wrapping_add(p);
    }
    acc as i32
}

/// fn_193384: `((a * (s16)b) >> 16) << 1` with u32 wrap. The ROM `smulwb`
/// uses the low 16 bits of `b` signed, replicated by the `as i16`.
pub fn smulwb2(a: i32, b: i32) -> i32 {
    let t = ((a as i64 * (b as i16 as i64)) >> 16) as i32;
    t.wrapping_shl(1)
}

/// fn_1a11b0: sine-table modulator. `table` is the borrowed 256-entry ROM
/// table `D_8468d0`; `phase` is the caller's u32 (engine+0xAFB8).
pub fn sine_mod(
    phase: &mut u32,
    n: i32,
    factor: i32,
    inb: &[u8],
    ino: usize,
    outb: &mut [u8],
    ofo: usize,
    table: &[i16],
) {
    let prod = 0x5f533929i64 * factor.wrapping_shl(16) as i64;
    let hi = (prod >> 32) as i32;
    let step = (hi >> 12).wrapping_sub(hi >> 31);
    if n <= 0 {
        return;
    }
    let mut n = n;
    let mut ii = ino;
    let mut oi = ofo;
    let mut one = |phase: &mut u32, ii: &mut usize, oi: &mut usize| {
        let p = phase.wrapping_add(1);
        *phase = p;
        let m = (p as i32).wrapping_mul(step) as u32;
        let s = i16::from_le_bytes([inb[*ii], inb[*ii + 1]]) as i32;
        let v = ((table[(m.wrapping_shl(16) >> 24) as usize] as i32 * s) >> 15) as i16;
        outb[*oi..*oi + 2].copy_from_slice(&v.to_le_bytes());
        *ii += 2;
        *oi += 2;
    };
    if n & 1 != 0 {
        one(phase, &mut ii, &mut oi);
    }
    n >>= 1;
    if n == 0 {
        return;
    }
    loop {
        one(phase, &mut ii, &mut oi);
        one(phase, &mut ii, &mut oi);
        n -= 1;
        if n == 0 {
            return;
        }
    }
}

/// fn_19edc0: lattice filter. `x` holds the coeff bytes (s16 at `x[2+2i]`,
/// 14 bytes for order 6, 22 otherwise); `dl` the u32 delay line of
/// `order` words. Returns the clip peak. Orders `< 1` with `n > 0` walk
/// off on ARM, so the harness only fuzzes that shape with `n <= 0`.
pub fn lattice_filter(
    n: i32,
    order: i32,
    x: &[u8],
    dl: &mut [u8],
    dlo: usize,
    input: &[u8],
    ino: usize,
    output: &mut [u8],
    ofo: usize,
) -> i32 {
    let ord = order.max(0) as usize;
    let taps = if order == 6 { 6 } else { 10 };
    let mut h = [0i16; 10];
    for (i, slot) in h.iter_mut().enumerate().take(taps) {
        *slot = i16::from_le_bytes([x[2 + i * 2], x[2 + i * 2 + 1]]);
    }
    let mut peak = 0i32;
    let mut idx = 0i32;
    while idx < n {
        let f = smlawb_dot(dl, dlo, &h, taps);
        for i in (1..ord).rev() {
            let v = u32::from_le_bytes([
                dl[dlo + (i - 1) * 4],
                dl[dlo + (i - 1) * 4 + 1],
                dl[dlo + (i - 1) * 4 + 2],
                dl[dlo + (i - 1) * 4 + 3],
            ]);
            dl[dlo + i * 4..dlo + i * 4 + 4].copy_from_slice(&v.to_le_bytes());
        }
        let io = ino + idx as usize * 2;
        let v = i16::from_le_bytes([input[io], input[io + 1]]) as i32;
        let mut t = (v << 7).wrapping_sub(f);
        dl[dlo..dlo + 4].copy_from_slice(&t.wrapping_shl(5).to_le_bytes());
        if order == 6 {
            t = (t >> 7).wrapping_add(t >> 8);
            if t > 0x6978 {
                t = 0x6978 + ((t - 0x6978) >> 1);
                if t >= 32768 {
                    if peak < t {
                        peak = t;
                    }
                    t = 0x7FFF;
                }
            } else if t < -27000 {
                t = -27000 + ((t + 27000) >> 1);
                if t < -32768 {
                    t = t.wrapping_neg();
                    if peak < t {
                        peak = t;
                    }
                    t = -32768;
                }
            }
        } else {
            t >>= 7;
            if t >= 32768 {
                if peak < t {
                    peak = t;
                }
                t = 0x7FFF;
            } else if t < -32768 {
                t = t.wrapping_neg();
                if peak < t {
                    peak = t;
                }
                t = -32768;
            }
        }
        let oo = ofo + idx as usize * 2;
        output[oo..oo + 2].copy_from_slice(&(t as i16).to_le_bytes());
        idx += 1;
    }
    peak
}

/// fn_19cbc8: per-mora unit synthesis over engine tables. `ino` is the
/// engine offset of the s16 input; `out`/`ofo` the output bytes; `tab`
/// the borrowed 5-entry ROM table `D_743608`. Table addresses wrap mod
/// 2^32 exactly like the ARM; the fixture keeps them in-engine.
pub fn per_mora(
    eng: &mut Engine,
    count: i32,
    ino: usize,
    out: &mut [u8],
    ofo: usize,
    tab: &[i16],
) {
    let c = ((eng.rd16(0xE73C) as i32) << 8) as i16 as i32;
    let th = eng.rds16(0xE73E);
    if count <= 0 {
        return;
    }
    let aff = |k: u32| 0xaffcu32.wrapping_add(k) as usize;
    for i in 0..count as usize {
        let ini = eng.rds16(ino + i * 2) as i32;
        let r = dot5(&eng.mem[..], 0xafec, tab, ini << 13).wrapping_shl(1);
        let t4 = eng.rd32(0xaff4);
        eng.wr32(0xaff8, t4);
        eng.wr32(0xaff4, (ini << 13) as u32);
        let ta = eng.rd32(0xafec);
        eng.wr32(0xaff0, ta);
        eng.wr32(0xafec, r.wrapping_shl(1) as u32);
        let cnt = eng.rd32(0xdee0);
        let vv: i32;
        if (cnt as i32) < th as i32 {
            eng.wr32(aff(cnt.wrapping_shl(2)), smulwb2(r, c) as u32);
            vv = ini;
        } else {
            let diff = (cnt.wrapping_sub(th as u32)) as i32;
            let idx_of = |p: i64, base: i32| {
                let hi = (p >> 32) as i32;
                (hi >> 10)
                    .wrapping_sub(hi >> 31)
                    .wrapping_mul(0x3ffff447i32)
                    .wrapping_add(base)
            };
            let idx1 = idx_of(0x575a2ad3i64 * diff as i64, diff);
            let w1 = eng.rds32(aff((idx1 as u32).wrapping_shl(2)));
            let rr = smulwb2(r.wrapping_add(w1), c);
            let idx2 = idx_of(0x575a2ad3i64 * cnt as i32 as i64, cnt as i32);
            eng.wr32(aff((idx2 as u32).wrapping_shl(2)), rr as u32);
            let w3 = eng.rds32(aff((idx1 as u32).wrapping_shl(2)));
            vv = ini.wrapping_add(w3 >> 12);
        }
        let vc = if vv >= 32768 {
            0x7fff
        } else if vv < -32768 {
            -32767
        } else {
            vv
        };
        out[ofo + i * 2..ofo + i * 2 + 2].copy_from_slice(&(vc as i16).to_le_bytes());
        eng.wr32(0xdee0, cnt.wrapping_add(1));
    }
}

/// fn_19ffa0 variant writing `y` to exterior bytes (the `fn_1a524c` call
/// passes stack `tmp`); table logic identical to [`resample_filter`].
pub fn resample_filter_to_mem(eng: &mut Engine, n: i32, x: usize, out: &mut [u8], ofo: usize) {
    let coef = 0x4178749Fi32;
    let e74c = eng.rd32(0xE74C);
    let mult = div100((e74c << 8) as i32);
    if n <= 0 {
        return;
    }
    let mut ip = 0i32;
    loop {
        let c = eng.rds32(0xE6BC);
        let thr = eng.rds16(0xE746);
        let sample = eng.rds16(x.wrapping_add((ip as usize).wrapping_mul(2)));
        let v: i32;
        if (thr as i32) <= c {
            let t = c.wrapping_sub(thr as i32);
            let fp = fp_step(coef, t);
            let tt = t.wrapping_add(fp.wrapping_mul(-1001));
            let acc = table_s16(eng, tt) as i32;
            let mut vv = (sample as i32).wrapping_add((acc.wrapping_mul(mult)) >> 8);
            if vv >= 0x8000 {
                vv = 0x7fff;
            } else if vv <= -0x8000 {
                vv = -0x8000;
            }
            v = vv;
        } else {
            v = sample as i32;
        }
        let oo = ofo.wrapping_add((ip as usize).wrapping_mul(2));
        out[oo..oo + 2].copy_from_slice(&(v as i16).to_le_bytes());
        let c2 = eng.rds32(0xE6BC);
        let wi = c2.wrapping_add(fp_step(coef, c2).wrapping_mul(-1001));
        table_store(eng, wi, sample);
        eng.wr32(0xE6BC, (c2.wrapping_add(1)) as u32);
        ip += 1;
        if ip >= n {
            return;
        }
    }
}

/// The five identical copy blocks in `fn_1a524c`: `dst[0..r5] = src[0..r5]`
/// through the even/odd pre-copy dance. Shapes with `r5 <= 0` and
/// `(r5-1)>>1 != 0` spin ~2^32 times on ARM; the harness excludes them.
fn copy_block(dst: &mut [u8], do_: usize, src: &[u8], r5: i32) {
    let mut pi = -1i32;
    let mut qi = -1i32;
    if r5 & 1 == 0 {
        pi += 1;
        qi += 1;
        put_u16(dst, do_.wrapping_add((qi as i64 * 2) as usize), u16_at(src, (pi as i64 * 2) as usize));
    }
    let mut s = s16_at(src, 0, pi.wrapping_add(1));
    let mut c = r5.wrapping_sub(1) >> 1;
    if c != 0 {
        loop {
            pi = pi.wrapping_add(2);
            let u = u16_at(src, (pi as i64 * 2) as usize);
            put_u16(dst, do_.wrapping_add((qi.wrapping_add(1) as i64 * 2) as usize), s as u16);
            s = s16_at(src, 0, pi.wrapping_add(1));
            qi = qi.wrapping_add(2);
            put_u16(dst, do_.wrapping_add((qi as i64 * 2) as usize), u);
            c -= 1;
            if c == 0 {
                break;
            }
        }
    }
    if r5.wrapping_sub(1) >= 0 {
        put_u16(
            dst,
            do_.wrapping_add((r5 as i64 * 2 - 2) as usize),
            u16_at(src, (r5 as i64 * 2 - 2) as usize),
        );
    }
}

// --- batch B: voice render + ring advance ---

/// Borrowed ROM tables shared by the voice-render chain. Slices borrow
/// snapshot windows; offsets are documented per field.
pub struct Tables<'a> {
    /// `D_790ba8`, 16 bytes (lattice coeffs).
    pub t790: &'a [u8],
    /// `D_8468d0`, 256 s16 (sine table).
    pub t8468: &'a [i16],
    /// `D_743608`, 5 s16 (per-mora taps).
    pub t7436: &'a [i16],
    /// `D_82e2b2` window, `[0]` = entry -128 (pitch curve).
    pub t82e2: &'a [i16],
    /// `D_82e0fc`, 91 s16 (duration curve).
    pub t82e0: &'a [i16],
    /// `D_8460c0` window (nibble tables).
    pub t8460: &'a [u8],
    /// Table at `*D_72273c` = `D_74249C` (expander).
    pub tw: &'a [u8],
    /// `D_74391c`, 256 bytes (voicing flags).
    pub t7439: &'a [u8],
    /// Window with `[0]` = `D_846200` (lattice drive tables).
    pub t846w: &'a [u8],
    /// `D_743908`, 6 s16 (mixer brackets).
    pub t908: &'a [i16],
    /// `D_7438FE`, 6 s16 (mixer values).
    pub t89e: &'a [i16],
    /// `D_743912`, 5 s16 (mixer divisors).
    pub t912: &'a [i16],
    /// Window around `D_7873a6` (phrase tables).
    pub t787: &'a [u8],
    /// Offset of `t787[0]` from `D_7873a6` (may be negative).
    pub w787: i32,
    /// 128 ROM bytes just before `D_74391c` (`mora_align` indexes `s88`).
    pub t7439pre: &'a [u8],
    /// ROM word `D_743944` value (pad-slot fill).
    pub k743944: u32,
    /// `D_743948` to end of code (note articulation bytes; the corpus
    /// indexes up to ~130K, still mapped ROM, so no tighter bound).
    pub t743948: &'a [u8],
    /// `D_74269c`, 144 bytes (phone class ids, all < 28 in ROM).
    pub t74269c: &'a [u8],
    /// `D_74272c`, 144 bytes (phone group ids, all < 27 in ROM).
    pub t74272c: &'a [u8],
    /// `D_7427bc`, 144 bytes (prosody ids, all <= 40 in ROM).
    pub t7427bc: &'a [u8],
    /// `D_74284c` window, 2400 bytes read as LE u16 (7x154 + 92).
    pub t74284c: &'a [u8],
    /// `D_7431ec` window, 216 bytes read as LE s16 (108 entries).
    pub t7431ec: &'a [u8],
    /// `D_743380` window, 200 bytes read as LE s16 (100 entries).
    pub t743380: &'a [u8],
    /// `D_7434be` window, 260 bytes read as LE s16 (130 entries).
    pub t7434be: &'a [u8],
    /// `D_74362e`, 576 + 64 trailing bytes (144 kana-name entries;
    /// trailing bytes keep `strcmp0` overruns bit-exact).
    pub t74362e: &'a [u8],
    pub t7435f4: &'a [u8],
    /// `D_7868c4` kana table, `(k7873a4+1)*12` bytes (12B entries).
    pub t7868c4: &'a [u8],
    /// ROM s16 `D_7873a4` (kana entry count; 232 in ROM).
    pub k7873a4: i16,
    /// `D_743614` window, 16 bytes (three stride-4 2-char entries).
    pub t743614: &'a [u8],
    /// `D_743620` window, 16 bytes (three stride-4 1-char entries).
    pub t743620: &'a [u8],
    /// `D_74362c` string + NUL (8 bytes).
    pub t74362c: &'a [u8],
    /// `D_722727`, 16 bytes (five stride-3 2-char entries + NULs).
    pub t722727: &'a [u8],
    /// 38 phoneme-name strings (`D_722740` pointer table resolved).
    pub ph38: [&'a [u8]; 38],
    /// `D_740bc4`, 53 tag entries of 120 bytes for `tag_parse`.
    pub tag53: &'a [u8],
    /// `D_7227d8`, 8 bytes compared against the job object in `dispatch`.
    pub t7227d8: &'a [u8],
    /// Full JP `code.bin` (base 0x100000). Mora-unit pointers in the
    /// alignment array are engine offsets for small values but absolute
    /// ROM addresses when they point at corpus-resident units (e.g.
    /// 0x7703e4 on the `long` phrase's join15), so `mora_render`,
    /// `mora_dsp` and `synth_vocoder` read unit bytes through
    /// `unit_byte`/`unit_s16`, which select engine or ROM by address.
    pub code: &'a [u8],
}

/// One unit byte from engine memory or ROM: unit pointers below
/// 0x14000 are engine-relative, 0x100000.. are absolute ROM addresses
/// into `code` (base 0x100000). Anything else is out of model and
/// panics loudly.
#[inline]
fn unit_byte(eng: &Engine, code: &[u8], uo: usize, off: usize) -> u8 {
    let a = uo.wrapping_add(off);
    if a < 0x14000 {
        eng.mem[a]
    } else {
        code[a - 0x100000]
    }
}

/// Little-endian s16 at a unit field via [`unit_byte`].
#[inline]
fn unit_s16(eng: &Engine, code: &[u8], uo: usize, off: usize) -> i16 {
    let lo = unit_byte(eng, code, uo, off) as u16;
    let hi = unit_byte(eng, code, uo, off + 1) as u16;
    (lo | (hi << 8)) as i16
}

/// Append-only PCM capture fed by `mix_pcm`, one s16 per mixed sample in
/// write order (the stream the ARM reference drains from the ring and
/// `mixer_pcm` logs natively). Rows pass a scratch log; the finish line
/// sizes it for the longest phrase. Pushing past capacity panics loudly.
pub struct PcmLog<'a> {
    pub buf: &'a mut [i16],
    pub len: usize,
}

impl<'a> PcmLog<'a> {
    pub fn new(buf: &'a mut [i16]) -> Self {
        PcmLog { buf, len: 0 }
    }
    #[inline]
    pub fn push(&mut self, v: i16) {
        assert!(self.len < self.buf.len(), "PcmLog full");
        self.buf[self.len] = v;
        self.len += 1;
    }
}

/// Null-link rule for node code: links are mem offsets and null is 0.
/// Structs never sit at offset 0 (the harness lays them out past a guard
/// window, and composition views start before the first node), so plain
/// `== 0` checks match the ARM exactly, byte for byte.


/// fn_1a524c: engine voice render into ring `buf` (engine offset).
/// `n` stays within the 480-word stack tmp (avg block needs `n <= 240`);
/// the harness matches real shapes.
pub fn voice_render(eng: &mut Engine, n: i32, buf: usize, tab: &Tables) -> i32 {
    let mut tmp = [0u8; 960];
    let r5 = n;
    let r8 = r5.wrapping_sub(1);
    eng.wr32(0xe6d4, 0);
    if r5 > 0 {
        let mut p = buf;
        let mut c = r5;
        loop {
            let mut v = eng.rds16(p) as i32 * 2;
            if v > 0x6978 {
                v -= 0x6978;
                v = 0x6978 + (v >> 1);
                if v >= 32768 {
                    v = 0x7FFF;
                }
            } else if v < -0x6978 {
                v += 0x6978;
                v = -0x6978 + (v >> 1);
                if v < -32768 {
                    v = -32768;
                }
            }
            eng.wr16(p, v as i16 as u16);
            p += 2;
            c -= 1;
            if c == 0 {
                break;
            }
        }
    }
    if eng.rd32(0xe6d4) == 1 {
        // Dynamically dead for ring bufs with n <= 240 (the clip loop
        // cannot reach 0xE6D4 without faulting first); kept faithful.
        let (lo, hi) = eng.mem.split_at_mut(0xafc0);
        lattice_filter(r5, 6, tab.t790, hi, 0, lo, buf, &mut tmp[..], 0);
        copy_block(lo, buf, &tmp, r5);
    }
    if eng.rd32(0xe74c) > 0 {
        libc::bzero0(&mut tmp[..], 512);
        resample_filter_to_mem(eng, r5, buf, &mut tmp[..], 0);
        copy_block(&mut eng.mem[..], buf, &tmp, r5);
    }
    let sl = eng.rd32(0xe750);
    if sl != 0 {
        libc::bzero0(&mut tmp[..], 512);
        let mut phase = eng.rd32(0xafb8);
        sine_mod(&mut phase, r5, sl as i32, &eng.mem[..], buf, &mut tmp[..], 0, tab.t8468);
        eng.wr32(0xafb8, phase);
        copy_block(&mut eng.mem[..], buf, &tmp, r5);
    }
    if eng.rd32(0xe748) != 0 {
        let mut st = DeacState::load(&eng.mem[..], 0xada4);
        let mut istg = [0i16; 256];
        let mut ostg = [0i16; 256];
        for i in 0..r5.max(0) as usize {
            istg[i] = eng.rds16(buf + i * 2);
        }
        deac_run(&mut st, &istg, &mut ostg, r5);
        st.store(&mut eng.mem[..], 0xada4);
        for i in 0..r5.max(0) as usize {
            tmp[i * 2..i * 2 + 2].copy_from_slice(&ostg[i].to_le_bytes());
        }
        copy_block(&mut eng.mem[..], buf, &tmp, r5);
    }
    if eng.rds16(0xe73e) > 0 {
        per_mora(eng, r5, buf, &mut tmp[..], 0, tab.t7436);
        copy_block(&mut eng.mem[..], buf, &tmp, r5);
    }
    {
        let w = eng.rd32(0xafbc);
        let buf0 = eng.rds16(buf);
        let mut t = (buf0 as i32 as u32).wrapping_add(w);
        t = t.wrapping_add(t >> 31);
        let t = (t as i32) >> 1;
        tmp[0..2].copy_from_slice(&(t as i16).to_le_bytes());
        tmp[2..4].copy_from_slice(&buf0.to_le_bytes());
        if r8 > 0 {
            let mut acc = buf0 as i32;
            for i in 0..r8 as usize {
                let u = eng.rd16(buf + 2 + i * 2) as i32;
                acc = acc.wrapping_add(u);
                acc = ((acc as u32).wrapping_add((acc as u32) >> 31)) as i32;
                acc >>= 1;
                tmp[4 + i * 4..6 + i * 4].copy_from_slice(&(acc as i16).to_le_bytes());
                tmp[6 + i * 4..8 + i * 4].copy_from_slice(&(u as u16).to_le_bytes());
                acc = u;
            }
        }
    }
    {
        let lastp = (buf as i64 + n as i64 * 2 - 2) as usize;
        let last = eng.rds16(lastp) as i32;
        let n2 = n.wrapping_mul(2);
        eng.wr32(0xafbc, last as u32);
        let fnp = eng.rd32(0xafd8);
        if fnp == 0 {
            return n2;
        }
        // Voice-queue push (ROM 0x607B10): writes stay inside the voice
        // struct page, engine untouched (vq probe: 0 engine diffs). The
        // ROM splits on `bgt`: nbytes > 0 returns cookie + 88 after the
        // queue copy; nbytes <= 0 tails to the unlock at 0x160204 (r0 0).
        // The lock-bail (`beq 0x607ba8`) never fires in renders.
        let cookie = eng.rd32(0xafe0);
        let _ = fnp;
        if (n2 as i16) as i32 > 0 {
            cookie.wrapping_add(88) as i32
        } else {
            0
        }
    }
}

/// fn_1a51e0: ring wrap check; renders + silences when behind, then parks
/// both positions on the slot. `n`/positions are offsets here.
pub fn ring_wrap(eng: &mut Engine, tab: &Tables) {
    let w = eng.rd32(0x858C);
    let r = eng.rd32(0x8584);
    if w < r {
        let s = eng.rds16(0x859A) as i32;
        voice_render(eng, s, w as usize, tab);
        ring_silence(eng, s as u32, w);
        let mut slot = 0x83A4u32;
        if w == slot {
            slot = 0x81C4;
        }
        eng.wr32(0x858C, slot);
        eng.wr32(0x8584, slot);
    }
}

/// fn_1a567c: ring advance by `n` samples with wrap/swap handling.
/// Returns the (possibly rewound) read position; `(u32)n` for `n < 0`.
pub fn ring_advance(eng: &mut Engine, n: i32, tab: &Tables) -> u32 {
    if n < 0 {
        return n as u32;
    }
    let cur = eng.rd32(0x8584).wrapping_add(n.wrapping_mul(2) as u32);
    eng.wr32(0x8584, cur);
    eng.wr32(0x859C, (n as u32).wrapping_add(eng.rd32(0x859C)));
    let act = eng.rd32(0x858C);
    if act == 0x81C4 && cur >= 0x83A4 {
        let s = eng.rds16(0x859A) as i32;
        voice_render(eng, s, 0x81C4, tab);
        let s = eng.rds16(0x859A) as i32;
        ring_silence(eng, s as u32, 0x81C4);
        eng.wr32(0x858C, 0x83A4);
    } else if act == 0x83A4 {
        // Goto lattice: cur < buf0 reaches `wrap` where &A (0x8584) is
        // above cur, so no swap; buf0 <= cur < buf1 swaps; cur >= buf1
        // swaps iff cur passes &A.
        if (0x81C4 <= cur && cur < 0x83A4) || cur > 0x8584 {
            let s = eng.rds16(0x859A) as i32;
            voice_render(eng, s, 0x83A4, tab);
            let s = eng.rds16(0x859A) as i32;
            ring_silence(eng, s as u32, 0x83A4);
            eng.wr32(0x858C, 0x81C4);
        }
    }
    let mut a = eng.rd32(0x8584);
    if eng.rd32(0x8594) <= a {
        a = a.wrapping_sub(960);
        eng.wr32(0x8584, a);
    }
    a
}

// --- batch B: classifier, envelope, energy norm, pitch ramp ---

/// fn_6b21ac: classify the 1-2 bytes before cursor `p` against borrowed
/// ROM string tables (`s1`: three 1-char entries at stride 4 from
/// `D_743620`; `s2`: three 2-byte entries at stride 4 from `D_743614`).
/// Returns 5/10/15 for code 0/1/2, 10 when nothing matches.
pub fn num_class(mem: &[u8], p: usize, s1: &[u8], s2: &[u8]) -> u16 {
    let mut tmp = [0u8; 4];
    libc::strncpy0(&mut tmp, &mem[p.wrapping_sub(1)..], 1);
    let mut code = -1i32;
    if libc::strcmp0(&tmp, &s1[0..]) == 0 {
        code = 0;
    } else if libc::strcmp0(&tmp, &s1[4..]) == 0 {
        code = 1;
    } else if libc::strcmp0(&tmp, &s1[8..]) == 0 {
        code = 2;
    } else {
        libc::strncpy0(&mut tmp, &mem[p.wrapping_sub(2)..], 2);
        if libc::strcmp0(&tmp, &s2[0..]) == 0 {
            code = 0;
        } else if libc::strcmp0(&tmp, &s2[4..]) == 0 {
            code = 1;
        } else if libc::strcmp0(&tmp, &s2[8..]) == 0 {
            code = 2;
        }
    }
    match code {
        0 => 5,
        2 => 15,
        _ => 10,
    }
}

/// One envelope stage of `fn_1a4ed0`: odd head sample plus `half >> 1`
/// pairs, each sample scaled by `(v * (e*k + acc)) >> 16`.
fn env_stage(mem: &mut [u8], dst: usize, e: i32, acc: i32, half: i32) {
    let mut k = 0i32;
    let mut pb = dst.wrapping_sub(2);
    if half & 1 != 0 {
        let cf = e.wrapping_mul(k).wrapping_add(acc);
        let v = s16_at(mem, pb, 1);
        k = 1;
        put_s16(mem, pb, 1, (((v as i32).wrapping_mul(cf) >> 16) as i16));
        pb = pb.wrapping_add(2);
    }
    let mut cnt = (half >> 1) as i16;
    if cnt == 0 {
        return;
    }
    loop {
        let i2 = k.wrapping_add(1);
        let c1 = e.wrapping_mul(k).wrapping_add(acc);
        let v1 = s16_at(mem, pb, 1);
        let c2 = e.wrapping_mul(i2).wrapping_add(acc);
        let v2 = s16_at(mem, pb, 2);
        let c1 = ((v1 as i32).wrapping_mul(c1) >> 16) as i16;
        cnt = cnt.wrapping_sub(1);
        let c2 = ((v2 as i32).wrapping_mul(c2) >> 16) as i16;
        put_s16(mem, pb, 1, c1);
        k = k.wrapping_add(2);
        put_s16(mem, pb, 2, c2);
        pb = pb.wrapping_add(4);
        if cnt == 0 {
            break;
        }
    }
}

/// fn_1a4ed0: envelope windowing over the 0x7FE0 window. `val > 0` runs
/// stage A (guarded by `diff`/`half`), `val == 0` runs stages B then C,
/// `val < 0` returns. Always returns 0.
pub fn env_window(mem: &mut [u8], base: usize, val: i32) -> i32 {
    let t = s16_at(mem, base + 0x7fe0, 0) as i32;
    let diff = t.wrapping_sub(val) as i16 as i32;
    if val > 0 {
        let half = t >> 1;
        let rem = t.wrapping_sub(half);
        if diff <= 0 {
            return 0;
        }
        if half <= 0 {
            return 0;
        }
        let e = fixed_ratio(0xffff0000u32 as i32, (half as i16 as i32) as u32) as i32;
        let acc = e.wrapping_mul(half.wrapping_sub(1)).wrapping_neg();
        env_stage(mem, base + rem as usize * 2 + 0x7fe2, e, acc, half);
        return 0;
    }
    if val != 0 {
        return 0;
    }
    let b4 = u16_at(mem, base + 4) as i32;
    let amp = t.wrapping_sub(b4) as i16 as i32;
    if amp <= 0 {
        return 0;
    }
    let e = fixed_ratio(0x10000, (amp as i16 as i32) as u32) as i32;
    env_stage(mem, base + 0x7fe2, e, 0, amp);
    let o = s16_at(mem, base + 4, 0) as i32;
    if amp <= 0 {
        return 0;
    }
    let e = fixed_ratio(0xffff0000u32 as i32, (amp as i16 as i32) as u32) as i32;
    let acc = e.wrapping_mul(amp.wrapping_sub(1)).wrapping_neg();
    env_stage(mem, (base as i64 + o as i64 * 2 + 0x7fe0) as usize, e, acc, amp);
    0
}

/// fn_6b1998: energy-normalize `n` samples (`src` u16) into `dst` (s16)
/// with an isqrt-ish gain, returning the gain. `factor` is the full R0
/// (the ROM `smulbb` narrows it); the pair-summed energy equals the plain
/// sum mod 2^32, ported as such.
pub fn energy_norm(
    factor: i32,
    srcb: &[u8],
    srco: usize,
    dstb: &mut [u8],
    dstoff: usize,
    n: i32,
) -> i32 {
    let mut r1: i32;
    let ip: i16;
    if n == 55 {
        ip = 59;
    } else if n == 60 {
        ip = 62;
    } else if n == 120 {
        ip = 88;
    } else if n == 240 {
        ip = 124;
    } else {
        let mut r1l = 15i32;
        let mut r2 = 0x40000000i32;
        loop {
            if n & r2 != 0 {
                break;
            }
            r1l = (r1l - 1) as i16 as i32;
            r2 >>= 1;
            if r2 <= 1 {
                break;
            }
        }
        if r1l & 1 != 0 {
            r1l = (r1l + 1) as i16 as i32;
        }
        let h1 = r1l;
        if r1l > 0 {
            r2 = n >> r1l;
        } else {
            r2 = n.wrapping_shl((-r1l) as u32);
        }
        let mut r3 = 0x4767i32;
        if r1l <= 0 {
            r1l = -r1l;
        }
        r1l += ((r1l as u32) >> 31) as i32;
        r1l >>= 1;
        r3 = r3.wrapping_mul(r2);
        r3 >>= 16;
        r3 = r3.wrapping_neg();
        r3 = r3.wrapping_add(650);
        r3 = r3.wrapping_mul(r2);
        r2 = r2.wrapping_add(r3 >> 16);
        r2 = r2.wrapping_add(17632);
        r1l = if h1 > 0 {
            r2.wrapping_shl(r1l as u32)
        } else {
            r2 >> r1l
        };
        r1l >>= 5;
        ip = r1l as i16;
    }
    r1 = 0;
    if n > 0 {
        for i in 0..n as usize {
            let u = u16_at(srcb, srco + i * 2) as i16 as i32;
            r1 = r1.wrapping_add((u * u) >> 6);
        }
    }
    let f: i16;
    if r1 == 0 {
        f = 1;
    } else {
        if (r1 as u32) > 0x3fff0001u32 {
            r1 = 0x3fff0001u32 as i32;
        }
        let mut r2 = 15i32;
        let mut r3 = 0x40000000i32;
        loop {
            if r1 & r3 != 0 {
                break;
            }
            r2 = (r2 - 1) as i16 as i32;
            r3 >>= 1;
            if r3 <= 1 {
                break;
            }
        }
        if r2 & 1 != 0 {
            r2 = (r2 + 1) as i16 as i32;
        }
        let prod = (factor as i16 as i32).wrapping_mul(ip as i32);
        let h2 = r2;
        if r2 > 0 {
            r1 >>= r2;
        } else {
            let t = -r2;
            r2 = -r2;
            r1 = r1.wrapping_shl(t as u32);
        }
        let mut r3 = 0x4767i32;
        r2 += ((r2 as u32) >> 31) as i32;
        r2 >>= 1;
        r3 = r3.wrapping_mul(r1);
        r3 >>= 16;
        r3 = r3.wrapping_neg();
        r3 = r3.wrapping_add(650);
        r3 = r3.wrapping_mul(r1);
        r1 = r1.wrapping_add(r3 >> 16);
        r1 = r1.wrapping_add(17632);
        r1 = if h2 > 0 {
            r1.wrapping_shl(r2 as u32)
        } else {
            r1 >> r2
        };
        r1 >>= 8;
        r1 = r1 as i16 as i32;
        f = fixed_ratio(prod, r1 as u32);
    }
    if n <= 0 {
        return f as i32;
    }
    let mut si = srco;
    let mut di = dstoff;
    let mut nn = n;
    loop {
        let s = u16_at(srcb, si) as i16 as i32;
        let mut v = (s.wrapping_mul(f as i32)) >> 6;
        if v < -32768 {
            v = -32768;
        } else if v >= 32768 {
            v = 0x7FFF;
        }
        put_u16(dstb, di, v as i16 as u16);
        si += 2;
        di += 2;
        nn -= 1;
        if nn == 0 {
            break;
        }
    }
    f as i32
}

/// fn_6b1fc4: pitch ramp of `n` s16 at engine `doff`, starting from the
/// sample before it, split by the 0xE75C threshold into flat start,
/// slope, and flat end runs.
pub fn pitch_ramp(eng: &mut Engine, doff: usize, n: i32, endv: i32) {
    let mut startv = eng.rds16(doff.wrapping_sub(2)) as i32;
    let mut endv = endv;
    let mut tval = eng.rds16(0xE75C) as i32;
    if startv > 0x1b58 {
        startv = 0x1b58;
    }
    if endv > 0x1b58 {
        endv = 0x1b58;
    }
    if tval < 0 {
        tval = 1;
    }
    if tval != 0 {
        // Target-true split: the flat/slope/flat division applies only when
        // n exceeds tval; otherwise the whole note is one slope run with
        // the n-based step (ARM `cmp n,tval` selects; tval comes from E75C,
        // nonzero only via id-16 tuning). Counts stay nonnegative, so the
        // write pointer only advances.
        let (r8, r9, nn) = if n > tval {
            let r8 = (n.wrapping_sub(tval)).wrapping_mul(4) / 10;
            let r9 = (n.wrapping_sub(tval)).wrapping_sub(r8);
            (r8, r9, tval)
        } else {
            (0, 0, n)
        };
        let step = if nn <= 0 {
            0
        } else {
            fixed_ratio((endv.wrapping_sub(startv)).wrapping_shl(6), nn as u32) as i32
        };
        let mut di = doff;
        for _ in 0..r8 {
            eng.wr16(di, startv as i16 as u16);
            di += 2;
        }
        for i in 0..nn.max(0) {
            let v = startv.wrapping_add(((step.wrapping_mul(i) >> 6) as i16) as i32);
            eng.wr16(di, v as i16 as u16);
            di += 2;
        }
        for _ in 0..r9 {
            eng.wr16(di, endv as i16 as u16);
            di += 2;
        }
    } else {
        if n <= 0 {
            return;
        }
        let step = fixed_ratio((endv.wrapping_sub(startv)).wrapping_shl(6), n as u32) as i32;
        let mut di = doff;
        for i in 0..n {
            let v = startv.wrapping_add(((step.wrapping_mul(i + 1) >> 6) as i16) as i32);
            eng.wr16(di, v as i16 as u16);
            di += 2;
        }
    }
}

// --- batch B: idiv + large mixer ---

/// ROM fn_300dd4 (__aeabi_idiv): signed quotient and remainder.
/// Divide-by-zero yields `(0, a)`; INT_MIN/-1 wraps to (INT_MIN, 0).
/// Callers that only need `r0` use `.0`.
pub fn idiv_full(a: u32, b: u32) -> (i32, i32) {
    let a = a as i32;
    let b = b as i32;
    if b == 0 {
        (0, a)
    } else if a == i32::MIN && b == -1 {
        (i32::MIN, 0)
    } else {
        (a / b, a % b)
    }
}

/// Piecewise-linear map shared by `mixer_6b`'s loop and tail: bracket
/// `prod` in `t908[1..5]`, interpolating `t89e` over `t912`.
fn pwlin(prod: i32, t908: &[i16], t89e: &[i16], t912: &[i16]) -> i32 {
    let mut prod = prod;
    let mut i = 1usize;
    loop {
        if prod >= t908[i] as i32 {
            i += 1;
            if i >= 5 {
                return 0;
            }
            continue;
        }
        let c3 = t908[i - 1] as i32;
        let lrv = t89e[i - 1] as i32;
        let ipv = t89e[i] as i32;
        prod = prod.wrapping_sub(c3);
        let d2 = ipv.wrapping_sub(lrv);
        let rv0 = t912[i] as i32;
        let mut tt = (d2.wrapping_mul(prod)) >> 4;
        tt = fixed_ratio(tt, rv0 as u32) as i32;
        tt += (t89e[i - 1] as i32) >> 4;
        return tt as i16 as i32;
    }
}

/// fn_6b1bd0: large mixer over a node chain in the engine (`fp` head, `end`
/// terminator, offsets) with engine head words. Always returns 0.
pub fn mixer_6b(
    fp: usize,
    p1: i32,
    end: usize,
    d: i32,
    e: i32,
    eng: &mut Engine,
    tab: &Tables,
) -> i32 {
    let r8: i16;
    if d <= 0 {
        if p1 <= 0x438 {
            r8 = 0xA66;
        } else {
            r8 = idiv_full(0x35E800, p1.wrapping_add(250) as u32).0 as i16;
        }
    } else if p1 <= 400 {
        r8 = 0x189D;
    } else {
        r8 = idiv_full(0x3E8000, p1.wrapping_add(250) as u32).0 as i16;
    }
    let sp0 = eng.rd32(fp + 4) as usize;
    let v0in = eng.mem[fp + 1] as i8 as i32;
    let mut v0 = v0in;
    let mut v7: i32;
    if e != 1 {
        v0 = v0.wrapping_mul(15);
        v7 = v0.wrapping_mul(7);
    } else if v0 != 1 {
        let t1 = eng.rd32(sp0 + 4) as usize;
        let t = eng.rd32(t1 + 24) as usize;
        v0 = v0.wrapping_mul(15);
        v0 = v0.wrapping_mul(7);
        v0 += u16_at(eng.mem, 0xE700 + 50) as i32 - eng.rd16(t + 12) as i32;
        v7 = v0 as i16 as i32;
    } else {
        v0 = eng.rds32(0xE6C0);
        if v0 == 1 {
            v0 = eng.rds32(0xE6C4);
            v7 = if v0 == 1 { 60 } else { 200 };
        } else {
            v7 = 200;
        }
    }
    eng.wr32(0xE6C0, e as u32);
    if v7 < 0 {
        v7 = 0;
    }
    eng.wr32(0xE6C4, v0in as u32);
    let mut r5 = e;
    let mut acc: i32;
    let sl: i16;
    {
        let env = (fixed_ratio(0xFA000, (r8 as i32) as u32) as i32)
            .wrapping_add(0xFFFFFFADu32 as i32)
            .wrapping_sub({
                let vc0 = eng.rd32(eng.rd32(sp0 + 4) as usize + 24) as usize;
                let e2 = s16_at(&eng.mem[..], vc0 + 8, 0) as i32 + 1;
                let e3 = eng.rd16(vc0 + 6) as i32;
                e3 + (e2 >> 1)
            });
        sl = env as i16;
        acc = 0;
        let mut vc = eng.rd32(eng.rd32(sp0 + 4) as usize + 24) as usize;
        if vc != end {
            loop {
                let x2 = s16_at(&eng.mem[..], vc + 8, 0) as i32 + 1;
                let x1 = s16_at(&eng.mem[..], vc + 6, 0) as i32;
                acc = acc.wrapping_add(x2 >> 1);
                r5 = x1.wrapping_add(acc);
                acc = r5.wrapping_add(sl as i32);
                let prod = ((r8 as i32).wrapping_mul(acc)) >> 8;
                acc = pwlin(prod, tab.t908, tab.t89e, tab.t912);
                let x1b = s16_at(&eng.mem[..], vc + 12, 0) as i32;
                let mut x2b = acc.wrapping_mul(v7);
                let acca = s16_at(&eng.mem[..], vc + 8, 0) as i32;
                x2b = x2b.wrapping_mul(3).wrapping_add(1024);
                acc = r5.wrapping_add(acca >> 1);
                x2b >>= 11;
                put_s16(&mut eng.mem[..], vc + 12, 0, (x1b.wrapping_add(x2b)) as i16);
                vc = eng.rd32(vc + 16) as usize;
                if vc == end {
                    break;
                }
            }
        }
        {
            let x2 = s16_at(&eng.mem[..], vc + 8, 0) as i32 + 1;
            let x1 = s16_at(&eng.mem[..], vc + 6, 0) as i32;
            acc = acc.wrapping_add(x2 >> 1);
            acc = acc.wrapping_add(x1);
            acc = acc.wrapping_add(sl as i32);
            let prod = ((r8 as i32).wrapping_mul(acc)) >> 8;
            acc = pwlin(prod, tab.t908, tab.t89e, tab.t912);
        }
        {
            let ipv = s16_at(&eng.mem[..], vc + 12, 0) as i32;
            let m = acc.wrapping_mul(v7);
            let mut o = m.wrapping_mul(3).wrapping_add(1024);
            o >>= 11;
            put_s16(&mut eng.mem[..], vc + 12, 0, (ipv.wrapping_add(o)) as i16);
        }
    }
    let f0 = eng.mem[fp] as i8;
    let s0a = (sp0 as u32)
        .wrapping_add(((f0 as i32) as u32).wrapping_shl(4))
        .wrapping_sub(8) as usize;
    let s0 = eng.rd32(s0a) as usize;
    if d == 1 {
        let o0 = s16_at(&eng.mem[..], s0 + 18, 0) as i32;
        let o1 = s16_at(eng.mem, 0xE700 + 58, 0) as i32;
        let o1 = o1.wrapping_mul(5);
        if (-10 + o1 * 2) < o0 && o0 < (10 + o1 * 2) {
            let f1 = eng.mem[fp + 1] as i8 as i32;
            let o = eng.rd16(end + 12) as i32;
            let mut p = (f1 * 103) >> 10;
            p -= (f1 * 103) >> 31;
            p *= -85;
            put_s16(&mut eng.mem[..], end + 12, 0, (o + p * 2) as i16);
        }
    } else if d == 0 {
        let o0 = s16_at(&eng.mem[..], s0 + 18, 0) as i32;
        let o1 = s16_at(eng.mem, 0xE700 + 58, 0) as i32;
        let ip = o1 * 3;
        if (ip - 10) < o0 && o0 < (ip + 10) {
            let f1 = eng.mem[fp + 1] as i8 as i32;
            let o = eng.rd16(end + 12) as i32;
            let mut p = (f1 * 103) >> 10;
            p -= (f1 * 103) >> 31;
            p = p - p * 16;
            put_s16(&mut eng.mem[..], end + 12, 0, (o + p * 8) as i16);
        } else if ((-10 + o1 * 2) < o0 && o0 < (10 + o1 * 2)) {
            let f1 = eng.mem[fp + 1] as i8 as i32;
            let o = eng.rd16(end + 12) as i32;
            let mut p = (f1 * 103) >> 10;
            p -= (f1 * 103) >> 31;
            p = p - p * 8;
            p = p + p * 4;
            put_s16(&mut eng.mem[..], end + 12, 0, (o + p * 2) as i16);
        }
    }
    let _ = r5;
    0
}

#[inline]
fn u32_at(mem: &[u8], off: usize) -> u32 {
    u32::from_le_bytes([mem[off], mem[off + 1], mem[off + 2], mem[off + 3]])
}

// --- batch B: tag parser + phrase emitter ---

/// fn_19ce6c: text tag parser, in place. Copies `text` through a 0x400
/// frame buffer, expanding `<\name` tags via the borrowed ROM table
/// (`tab`, 53 entries of 120 bytes at `D_740bc4`), then copies back.
/// `text` needs a 64-zero margin past the NUL (tag scans overrun it).
pub fn tag_parse(text: &mut [u8], toff: usize, tab: &[u8]) {
    let mut buf = [0u8; 0x400];
    let mut a = [0u8; 32];
    let mut c = [0u8; 12];
    let mut tobj = [0u8; 12];
    let count = libc::strlen0(&text[toff..]) as i32;
    let mut r4 = 0i32;
    let mut r7 = 0i32;
    let mut sl = 0i32;
    let mut app = |s: &[u8]| {
        let dl = libc::strlen0(&buf);
        libc::strcat0(&mut buf, dl, s);
    };
    if count > 0 {
        'main: loop {
            a.fill(0);
            c.fill(0);
            let cc = text[toff + r4 as usize];
            if cc == b'<' && text[toff + r4 as usize + 1] == b'\\' {
                let saved = r4 + 1;
                if sl == 1 {
                    r7 += 1;
                    if r7 >= 0x400 {
                        break 'main;
                    }
                    app(b"\n\0");
                    sl = 0;
                }
                let mut r6 = 0i32;
                loop {
                    let cc2 = text[toff + r4 as usize];
                    if cc2 == b'>' || cc2 == b'=' {
                        break;
                    }
                    a[r6 as usize] = cc2;
                    if r6 > 20 || count - 1 == r4 {
                        r4 = saved;
                        a.fill(0);
                        break;
                    }
                    if text[toff + r4 as usize] == b'<'
                        && text[toff + r4 as usize + 1] == b'\\'
                        && r6 != 0
                    {
                        r4 = saved;
                        a.fill(0);
                        break;
                    }
                    r6 += 1;
                    r4 += 1;
                }
                r6 = 0;
                if text[toff + r4 as usize] == b'=' {
                    let mut r6e = 0i32;
                    loop {
                        c[r6e as usize] = text[toff + r4 as usize];
                        if r6e > 7
                            || count - 1 == r4
                            || (text[toff + r4 as usize] == b'<'
                                && text[toff + r4 as usize + 1] == b'\\')
                        {
                            r4 = saved;
                            a.fill(0);
                            break;
                        }
                        r4 += 1;
                        r6e += 1;
                        if text[toff + r4 as usize] != b'>' {
                            continue;
                        }
                        break;
                    }
                }
                let mut found = None;
                for i in 0..53usize {
                    if libc::strcmp0(&a, &tab[i * 120..]) == 0 {
                        found = Some(i);
                        break;
                    }
                }
                if let Some(i) = found {
                    let e1 = libc::strlen0(&a[20..]) as i32;
                    let ent = i * 120;
                    let e2 = libc::strlen0(&c[1..]) as i32;
                    r6 = e2 + e1 + r7;
                    if r6 >= 0x400 {
                        break 'main;
                    }
                    app(&tab[ent + 20..]);
                    if c[1] == b'+' {
                        app(&c[2..]);
                    } else {
                        app(&c[1..]);
                    }
                    if libc::strcmp0(&tab[ent + 20..], b"#p\0") == 0 {
                        r6 += 1;
                        if r6 >= 0x400 {
                            break 'main;
                        }
                        app(b"\\\0");
                    } else {
                        r6 += 3;
                        if r6 >= 0x400 {
                            break 'main;
                        }
                        app(b"\n#q\0");
                    }
                    r7 = r6 + 1;
                    if r7 >= 0x400 {
                        break 'main;
                    }
                    app(b"\n\0");
                }
            } else {
                libc::strncpy0(&mut tobj, &text[toff + r4 as usize..], 1);
                r7 += libc::strlen0(&tobj) as i32;
                if r7 >= 0x400 {
                    break 'main;
                }
                app(&tobj);
                sl = 1;
            }
            r4 += 1;
            if r4 >= count {
                break 'main;
            }
        }
    }
    libc::strcpy0(&mut text[toff..], &buf);
}

/// s16 read across 1a15ac's `out[10]`/`stage[32]` frame pair (`out` sits
/// at sp+32, `stage` at sp+52): indices past `out` read `stage` bytes.
fn frame_s16(out: &[i16; 10], stage: &[u8; 32], oi: i32) -> i16 {
    if oi < 10 {
        out[oi as usize]
    } else {
        let o = (oi - 10) as usize * 2;
        i16::from_le_bytes([stage[o], stage[o + 1]])
    }
}


/// emit_1a1814: node emitter. `p` walks engine nodes, `q`/`st` engineine, `tab`
/// the ROM window with the selected table at `ti`. Always returns 0.
fn emit_nodes(
    mut p: usize,
    eng: &mut Engine,
    mut q: usize,
    mut idx: i32,
    st: usize,
    tab: &[u8],
    ti: usize,
    mut lim: i32,
    cmp: usize,
    acclo: i32,
    saved: i32,
    factor: i32,
    acchi: i32,
    out: &[i16; 10],
    stage: &[u8; 32],
) -> i32 {
    loop {
        if eng.rd16(st + 48) as i32 >= 384 {
            return 0;
        }
        let b0t = tab[ti + idx as usize + 4];
        if b0t == tab[ti + idx as usize + 5] {
            idx = (idx + 1) as i16 as i32;
        }
        let mut d = s16_at(&eng.mem[..], p + 8, 0) as i32;
        if d == 0 || p != cmp || saved == -1 {
            d = s16_at(&eng.mem[..], p + 6, 0) as i32;
        }
        if d == 0 && eng.mem[p.wrapping_sub(14)] != 0 {
            let v1 = s16_at(&eng.mem[..], p.wrapping_sub(12), 0) as i32;
            let v2 = s16_at(&eng.mem[..], p + 2, 0) as i32;
            let mut v3 = s16_at(&eng.mem[..], p.wrapping_sub(8), 0) as i32;
            let v4 = s16_at(&eng.mem[..], p + 20, 0) as i32;
            let v5 = s16_at(&eng.mem[..], p + 16, 0) as i32;
            let lr_ = v1 + v2;
            v3 -= v4;
            let mut v1 = v1 + v5;
            let m = lr_.wrapping_mul(v3);
            v1 += v2 << 1;
            let m = idiv_full(m as u32, v1 as u32).0;
            d = v4.wrapping_add(m) as i16 as i32;
        }
        let b0 = eng.mem[p];
        let t = frame_s16(out, stage, idx + 1) as u16;
        eng.mem[q] = b0;
        let j = idx + 1;
        eng.wr16(q + 2, t);
        let mut b0 = (b0 as i32).wrapping_sub(1);
        lim = lim.wrapping_add(1);
        if (b0 as u32) <= 5 {
            b0 = eng.mem[p + 13] as i32;
            if b0 == 0x55 {
                d = d.wrapping_neg();
            }
        }
        eng.wr16(q + 4, d as i16 as u16);
        eng.wr16(q + 6, eng.rd16(p + 4));
        let b0b = tab[ti + idx as usize + 4];
        let b1 = tab[ti + idx as usize + 5];
        idx = j as i16 as i32;
        eng.mem[q + 12] = b0b;
        let mut b0 = b1 as i32;
        let acc = ((acchi as i64) << 32 | acclo as u32 as i64)
            + (0x92492493u32 as i32 as i64) * factor as i64;
        let hi = (acc >> 32) as i32;
        b0 -= 1;
        eng.mem[q + 13] = b0 as u8;
        eng.wr16(q + 10, u16_at(tab, ti));
        b0 = hi >> 3;
        let r0n = b0 - (hi >> 31) + 2;
        eng.wr16(q + 8, eng.rd16(p + 10));
        p = p.wrapping_add(14);
        q = q.wrapping_add(14);
        eng.wr16(st + 48, eng.rd16(st + 48).wrapping_add(1));
        if r0n > lim {
            continue;
        }
        break;
    }
    let b0 = eng.mem[p];
    if b0 != 0 || saved != -2 {
        return 0;
    }
    if eng.rd16(st + 48) as i32 >= 384 {
        return 0;
    }
    eng.mem[q] = 0;
    let v = s16_at(&eng.mem[..], p + 2, 0) as i32 + 2;
    eng.wr16(q + 2, ((v * 0x199a) >> 15) as i16 as u16);
    eng.wr16(st + 48, eng.rd16(st + 48).wrapping_add(1));
    0
}

/// fn_1a15ac: phrase emitter over 14-byte engine nodes `b..=c` into
/// engine tables (`d` = engine). Always returns 0.
pub fn phrase_emit(
    mode: i32,
    b: usize,
    c: usize,
    eng: &mut Engine,
    tables: &Tables,
) -> i32 {
    let tab = tables.t787;
    let wlo = tables.w787;
    let (v0, v1) = match mode {
        -2 | 1 => (2, 3),
        -1 | 2 => (1, 2),
        _ => (2, 2),
    };
    let mut out = [0i16; 10];
    let mut stage = [7u8; 32];
    if mode == -1 {
        stage[0] = 0;
    }
    let qd = c.wrapping_sub(b) as i32;
    let mut idx = (mode == -1) as u32;
    let fdiv12 = |q: i32| q / 14 + 2;
    let mut cnt = fdiv12(qd);
    let b0 = b;
    let mut bb = b;
    if cnt > 1 {
        let mut it = 1i32;
        loop {
            let fb = eng.mem[bb];
            bb = bb.wrapping_add(14);
            stage[idx as usize] = fb;
            cnt = fdiv12(qd);
            it += 1;
            idx = idx.wrapping_add(1) as i16 as i32 as u32;
            if cnt <= it {
                break;
            }
        }
    }
    if mode == -2 {
        stage[idx as usize] = 0;
    }
    let mut bb = b0;
    let mut idx = 0u32;
    loop {
        if bb > c {
            break;
        }
        let s = s16_at(&eng.mem[..], bb + 2, 0) as i32;
        let oi = idx as usize + 1;
        if bb == b0 && v0 == 2 {
            out[oi] = (((s + 7) * 0xccd) >> 15) as i16;
        } else if bb == c && v1 == 2 {
            out[oi] = (((s + 3) * 0xccd) >> 15) as i16;
        } else {
            out[oi] = (((s + 2) * 0x199a) >> 15) as i16;
        }
        idx += 1;
        bb = bb.wrapping_add(14);
        idx = idx as i16 as i32 as u32;
        if (idx as i32) >= 9 {
            break;
        }
    }
    out[0] = idx as i16;
    let db48 = eng.mem[0xE700 + 48] as i8 as i32;
    let off = (db48 * 19) * 1024 + (stage[0] as i32 * 19) * 128 + stage[1] as i32 * 64
        + stage[2] as i32 * 8;
    let ti = (off - wlo) as usize;
    let vv = u16_at(tab, ti + 2) as i32;
    // ARM (0x1a1728-0x1a1738) returns -1 both when the entry is missing
    // (vv==0) and when its table pointer is null (r8==0); only a present
    // entry continues to emit.
    if vv == 0 || 0x7873A6u32.wrapping_add(off as u32) == 0 {
        return -1;
    }
    let c = if mode == 1 { c.wrapping_sub(14) } else { c };
    let st = 0x3e00usize;
    let cf = b0;
    let cd = 0x2930u32
        .wrapping_add((eng.rds16(0x3e30) as i32 as u32).wrapping_mul(14))
        as usize;
    if fdiv12(c.wrapping_sub(b0) as i32) > 1 {
        let span = c.wrapping_sub(b0) as i32;
        return emit_nodes(b0, eng, cd, 0, st, tab, ti, 1, b0, 0, mode, span, span, &out, &stage,
        );
    }
    if eng.mem[cf] != 0 || mode != 2 {
        return 0;
    }
    if eng.rd16(st + 48) as i32 >= 384 {
        return 0;
    }
    eng.mem[cd] = eng.mem[cf];
    let s = s16_at(&eng.mem[..], cf + 2, 0) as i32;
    eng.wr16(cd + 2, (((s + 2) * 0x199a) >> 15) as i16 as u16);
    eng.wr16(st + 48, eng.rd16(st + 48).wrapping_add(1));
    0
}

// --- batch C: vocoder synth + mora render ---

/// One lattice multiply step of `fn_19f8c0`, exactly as the asm: magnitude
/// product of the s16 sample and the 32-bit state (16-bit halves with
/// 0x8000 rounding), sign reapplied afterwards.
fn lat_prod(v: i32, b: i32) -> i32 {
    let mut u7 = ((v as i16 as i32) << 16) as u32;
    let mut sgn = 1i32;
    if (u7 as i32) < 0 {
        u7 = 0u32.wrapping_sub(u7);
        sgn = -1;
    }
    let mut um = b as u32;
    if (um as i32) < 0 {
        um = 0u32.wrapping_sub(um);
        sgn = -sgn;
    }
    let is7 = u7 as i32;
    let r4 = is7 >> 16;
    let lo = (r4 as u32).wrapping_mul(um & 0xFFFF);
    let hi = um >> 16;
    let t7 = lo.wrapping_add((u7 & 0xFFFF).wrapping_mul(hi));
    let slo = (r4 as i16 as i32).wrapping_mul(hi as i16 as i32);
    let t5 = (t7.wrapping_add(0x8000)) as i32;
    let tot = slo.wrapping_add(t5 >> 16);
    tot.wrapping_mul(sgn)
}

/// fn_19f8c0: lattice vocoder synth for one mora. `ioff` addresses the
/// mora descriptor (engine offset); `ooff` the engine out word (0x7DF6).
/// `c0raw` stays in 0..=10 (wider shapes smash the frame pairs on ARM);
/// the harness matches real shapes.
pub fn synth_vocoder(eng: &mut Engine, ioff: usize, ooff: usize, tab: &Tables) -> i32 {
    if unit_byte(eng, tab.code, ioff, 3) == 0 {
        eng.wr16(ooff, 0);
        return 0;
    }
    let mut ptmp = [0i16; 10];
    let mut obuf = [0u8; 512];
    let mut dlbuf = [0u8; 40];
    let mut b_d = [0i32; 6];
    let mut pairs = [0i16; 10];
    let mut b_c = [0i32; 6];
    let mut b_a = [0i32; 6];
    let mut b_b = [0i32; 6];
    let mut b_e = [0i32; 6];
    let mut b_f = [0i32; 6];
    let mut tail = [0u8; 40];
    let mut samp = [0i16; 12];
    for k in 0..10usize {
        let b = unit_byte(eng, tab.code, ioff, k / 2 + 5) as u32;
        let nib = if k & 1 == 1 { b & 15 } else { (b << 24) >> 28 };
        let to = k * 32 + nib as usize * 2;
        let v = i16::from_le_bytes([tab.t8460[to], tab.t8460[to + 1]]) as i32;
        ptmp[k] = (if k == 0 { v } else { v + ptmp[k - 1] as i32 }) as i16;
    }
    let p7 = 0xE700usize;
    let mut c0raw = 0i32;
    if eng.rd16(p7 + 0x58) != 0x400 {
        let w0 = eng.rd32(0xE754);
        if (w0 as i32) > 100 {
            c0raw = 10 - eng.rd16(p7 + 0x5A) as i32;
            for q in ptmp.iter_mut() {
                let t0 = *q as u16 as u32;
                let m2 = eng.rd16(p7 + 0x58) as u32;
                let pr = t0.wrapping_mul(m2) as i32;
                let bound = 0x8000u32 + (t0 >> 1);
                let nv = pr >> 10;
                *q = (if (nv as u32) > bound { bound as i16 } else { nv as i16 });
            }
        } else if (w0 as i32) < 100 {
            let first = ptmp[0] as u16 as u32;
            let ip0 = first >> 2;
            let m2 = eng.rd16(p7 + 0x58) as u32;
            for q in ptmp.iter_mut() {
                let t0 = *q as u16 as u32;
                let d = (t0 as i32) - (ip0 as i32);
                let lim = 0xFFFF0000u32.wrapping_add(t0.wrapping_mul(2)) as i32;
                let pr = (d as u32).wrapping_mul(m2) as i32;
                let nv = (ip0 as i32) + (pr >> 10);
                if t0 < 0x7000 {
                    *q = (if nv < lim { lim } else { nv }) as i16;
                }
            }
        }
    }
    let c0v = 10 - c0raw;
    if c0v > 0 {
        for i in 0..c0v as usize {
            let v = ptmp[i] as u16 as u32;
            let t = v.wrapping_shl(6) as i32;
            let u = (t >> 15) as usize;
            let e0 = i16::from_le_bytes([tab.tw[u * 2], tab.tw[u * 2 + 1]]) as i32;
            let e1 = u16::from_le_bytes([tab.tw[u * 2 + 2], tab.tw[u * 2 + 3]]) as i32;
            let dd = e1 - e0;
            let f = (t & !0x3F8000) as i16 as i32;
            let pr = (dd as i16 as i32) * f;
            let sh = pr >> 15;
            let res = e0 + (sh as i16 as i32);
            pairs[i] = (-res) as i16;
        }
    }
    let nn = ((c0v >> 1) as i16 as i32) + 1;
    if nn > 0 {
        for i in 0..nn as usize {
            b_f[i] = 0;
            b_d[i] = 0;
            b_b[i] = 0;
            b_a[i] = 0;
        }
    }
    let r3b = (c0v >> 1) as i16 as i32;
    if c0v >= 0 {
        let mut r = 0x2000000i32;
        let mut sl = 0i32;
        let mut r6 = 0i32;
        loop {
            let old = sl;
            b_e[0] = r - old;
            sl = r;
            b_c[0] = r + old;
            let mut kk = 0i32;
            if r3b > 0 {
                while kk < r3b {
                    let k = kk as usize;
                    let v0 = pairs[k * 2] as i32;
                    let v1 = pairs[k * 2 + 1] as i32;
                    let ak = b_a[k];
                    let dk = b_d[k];
                    let prod1 = lat_prod(v0, ak);
                    let cold = b_c[k];
                    let bold = b_b[k];
                    let prod2 = lat_prod(v1, dk);
                    let fold = b_f[k];
                    let eold = b_e[k];
                    b_c[k + 1] = bold.wrapping_add(cold).wrapping_add(prod1.wrapping_mul(4));
                    b_b[k] = ak;
                    b_a[k] = cold;
                    b_e[k + 1] = fold.wrapping_add(eold).wrapping_add(prod2.wrapping_mul(4));
                    b_f[k] = dk;
                    b_d[k] = eold;
                    kk += 1;
                }
            }
            let sum = b_c[r3b as usize].wrapping_add(b_e[r3b as usize]);
            samp[r6 as usize] = (sum >> 15) as i16;
            r6 = (r6 + 1) as i16 as i32;
            r = 0;
            if r6 > c0v {
                break;
            }
        }
    }
    if c0v - 1 > 0 {
        let (mut pi, mut qi) = (0usize, 0usize);
        if c0v & 1 == 0 {
            put_u16(&mut tail, 2, samp[1] as u16);
            pi = 1;
            qi = 1;
        }
        let mut s = samp[pi + 1];
        let mut cc = (c0v - 1) >> 1;
        if cc != 0 {
            loop {
                let u = samp[pi + 2] as u16;
                pi += 2;
                put_u16(&mut tail, 2 + qi * 2, s as u16);
                s = samp[pi + 1];
                cc -= 1;
                qi += 2;
                put_u16(&mut tail, 2 + (qi - 1) * 2, u);
                if cc == 0 {
                    break;
                }
            }
        }
    }
    let t0 = c0v - 1;
    if t0 >= 0 {
        put_u16(&mut tail, 2 + t0 as usize * 2, samp[t0 as usize + 1] as u16);
    }
    let bo = (10 - c0raw) as usize;
    libc::bzero0(&mut tail[2 + bo * 2..], (c0raw * 2) as usize);
    let t2 = eng.mem[ioff + 2] as i32;
    if t2 != 0 {
        let t4 = eng.mem[ioff + 4] as usize;
        let fl = tab.t7439[t4];
        dlbuf.fill(0);
        let tbl = if fl & 0x84 != 0 { 512usize } else { 0usize };
        lattice_filter(256, 10, &tail, &mut dlbuf, 0, &tab.t846w[tbl..], 0, &mut obuf, 0);
        energy_norm(0x61C, &obuf, 0, &mut eng.mem[..], ooff + 2, 0xF0);
        eng.wr16(ooff, 0xF0);
        eng.mem[ioff + 3] = 0xF0;
        return 0xF0;
    }
    dlbuf.fill(0);
    let v = eng.rds16(4) as i32;
    let w = eng.rds16(0x7FDE) as i32;
    let p = v * w;
    lattice_filter(
        v, 10, &tail, &mut dlbuf, 0,
        &tab.t846w[1024 + (p * 2) as usize..], 0, &mut obuf, 0,
    );
    let w2 = eng.rd16(0x7FDE) as i32;
    let nw = (w2 + 1) as i16 as i32;
    eng.wr16(0x7FDE, nw as i16 as u16);
    if nw >= 6 {
        eng.wr16(0x7FDE, 0);
    }
    energy_norm(0x800, &obuf, 0, &mut eng.mem[..], ooff + 2, v as i16 as i32);
    eng.wr16(ooff, v as i16 as u16);
    eng.mem[ioff + 3] = v as u8;
    v
}

/// fn_19e078: mora DSP chain (synth + pitch/duration shaping). `moff`
/// addresses the mora descriptor (engine offset); shaping tables come
/// from `tab`. Always returns 1.
pub fn mora_dsp(eng: &mut Engine, moff: usize, pitch: i32, tab: &Tables) -> i32 {
    let synth = synth_vocoder(eng, moff, 0x7DF6, tab);
    eng.wr16(0x7F00 + 220, synth as i16 as u16);
    let a = unit_byte(eng, tab.code, moff, 3) as i32;
    let b = eng.rd16(4) as u8 as i8 as i32;
    let mut p = pitch;
    if a == b || a == 0 {
        p = 0;
    }
    let h = unit_s16(eng, tab.code, moff, 0) as i32;
    let t = (h * 0xCCD + 0x4000) >> 15;
    eng.wr16(0x7F00 + 218, t as i16 as u16);
    if t > 90 {
        eng.wr16(0x7F00 + 218, 90);
    }
    if eng.rds16(0x7F00 + 218) < 0 {
        eng.wr16(0x7F00 + 218, 0);
    }
    if p > 0 {
        let mut idx = (p >> 7) - 28;
        if idx >= 50 {
            idx = 49;
        } else if idx < 0 {
            idx = 0;
        }
        let v1 = tab.t82e2[idx as usize + 128] as i32;
        let v2 = tab.t82e2[(p & 127) as usize] as i32;
        let w = eng.rd32(0) as i32;
        let mut r = v1 * v2;
        r >>= 16;
        r = w.wrapping_mul(r);
        r = ((r >> 16) * 0x3EE2) >> 16;
        eng.wr16(0x7F00 + 216, r as i16 as u16);
    } else {
        eng.wr16(0x7F00 + 216, p as i16 as u16);
    }
    let dd = eng.rds16(0x7F00 + 216);
    if dd < 0 {
        eng.wr16(0x7F00 + 216, 0);
    } else if dd >= 240 {
        eng.wr16(0x7F00 + 216, 239);
    }
    let synth = if synth < 0 { -synth } else { synth };
    let da = eng.rds16(0x7F00 + 218) as i32;
    let tv = tab.t82e0[da as usize] as i32;
    if synth > 0 {
        sat_scale_engine(eng, 0x7DF8, 0x7DF8, synth, tv * 2);
    }
    eng.wr16(0x7DF6, synth as i16 as u16);
    1
}

/// fn_1a1b14: render-all loop over `count` moras. The unit pointer
/// array at `uoff`, the s16 note array at `noff`, and the mora
/// descriptors all live in the engine; unit entries are engine offsets
/// (the test converts guest absolutes).
pub fn mora_render(
    eng: &mut Engine,
    uoff: usize,
    noff: usize,
    count: i32,
    tab: &Tables,
    pcm: &mut PcmLog,
) {



    let mut i = 0i16;
    loop {
        if (i as i32) >= count {
            break;
        }
        let uo = eng.rd32(uoff + (i as usize) * 4) as usize;
        let kind = unit_byte(eng, tab.code, uo, 4);
        if kind != eng.mem[0xE760] && kind != 0 {
            eng.mem[0xE760] = kind;
        }
        let note = s16_at(&eng.mem[..], noff, i as i32);
        mora_dsp(eng, uo, note as i32, tab);
        let eb = eng.rds16(0x81C2).wrapping_add(eng.rds16(4));
        eng.wr16(0x81C2, eb as u16);
        let step0 = peek_7fd8(eng) as i32;
        let dur = step0;
        let step = if dur == 0 { eng.rds16(4) as i32 } else { step0 };
        if eng.rds16(0x81C2) > 0 {
            loop {
                block_shuffle(&mut eng.mem[..], 0);
                if dur == 0 {
                    env_window(&mut eng.mem[..], 0, 0);
                    mix_pcm(eng, 0, step, pcm);
                } else {
                    env_window(&mut eng.mem[..], 0, dur);
                    mix_pcm(eng, 0, dur, pcm);
                }
                let eb = (eng.rds16(0x81C2) as i32) - step;
                eng.wr16(0x81C2, eb as i16 as u16);
                ring_advance(eng, step, tab);
                if eng.rds16(0x81C2) <= 0 {
                    break;
                }
            }
        }
        if flag_done(eng) == 1 {
            break;
        }
        i = i.wrapping_add(1);
    }
}

// --- batch C: prosody walker ---

/// strlen-scan helper for `prosody_walk`: `(length, last byte)` over a
/// NUL-terminated run, length counted like the ROM `(s16` counter).
fn scan_len(mem: &[u8], mut o: usize) -> (i32, i32) {
    if mem[o] == 0 {
        return (0, 0);
    }
    let mut s0 = 0i16;
    let mut ch = 0i32;
    loop {
        let nn = s0.wrapping_add(1);
        ch = mem[o + s0 as usize] as i8 as i32;
        s0 = nn;
        if mem[o + s0 as usize] == 0 {
            break;
        }
    }
    (s0 as i32, ch)
}

/// fn_1a00ac: prosody walker over node lists in the engine (offsets, null is
/// 0; structs never sit at offset 0), then `mixer_6b` calls. `fp0` is
/// the head link. Always returns 0.
pub fn prosody_walk(eng: &mut Engine, fp0: u32, tab: &Tables) -> i32 {
    if fp0 == 0 {
        return 0;
    }
    let mut fp = fp0;
    // Phase 1.
    loop {
        let c0 = eng.mem[fp as usize] as i8 as i32;
        let mut r7 = eng.rd32(fp as usize + 4);
        let mut sl = 0i32;
        if c0 > 0 {
            // inner (also re-entered from l4a0):
            loop {
                let r4 = eng.rd32(r7 as usize + 4);
                let r6 = eng.rd32(r4 as usize + 24);
                let r1v = eng.mem[r7 as usize + 2] as i8 as i32;
                let mut r0v: i32;
                if eng.mem[r6 as usize + 5] == 76
                    || eng.mem[r6 as usize + 24] == 81
                    || eng.mem[r6 as usize + 20] == 78
                {
                    r0v = if eng.mem[r7 as usize + 1] == 1 { 0x9375 } else { 0x84b6 };
                } else {
                    r0v = 0x9375;
                }
                let mut r9 = r1v.wrapping_mul(r0v);
                {
                    let r0b = eng.rd32(r7 as usize + 8) as usize;
                    let r1v = s16_at(&eng.mem[..], r0b + 18, 0) as i32;
                    let r2v = s16_at(eng.mem, 0xE700 + 58, 0) as i32;
                    let r2v = r2v + (r2v << 1);
                    if r1v > r2v {
                        let t1 = eng.mem[r0b + 6] as i8 as i32;
                        if t1 == 0 {
                            r9 = (r9.wrapping_mul(0x666)) >> 12;
                        } else {
                            let t0 = eng.mem[r0b + 4] as i8 as i32;
                            if t1 == 1 {
                                if t0 < 3 {
                                    r9 >>= 2;
                                }
                            } else {
                                let t0 = t0 - t1;
                                if t0 < 2 {
                                    r9 >>= 1;
                                }
                            }
                        }
                    }
                }
                // inner_tail:
                let t0 = eng.mem[r7 as usize] as i8 as i32;
                let mut r8 = 0i32;
                let mut r5 = 0i32;
                if t0 > 0 {
                    let mut r4 = r4;
                    loop {
                        let w = eng.rd32(r4 as usize);
                        let s: &[u8] = if w == 0 { &[] } else { &eng.mem[w as usize..] };
                        let tx = vowel_count(s) as i32;
                        let t1 = eng.mem[r7 as usize] as i8 as i32;
                        r4 = eng.rd32(r4 as usize + 28);
                        let tx = tx + r8;
                        r5 = (r5 + 1) as i16 as i32;
                        r8 = tx as i16 as i32;
                        if t1 <= r5 {
                            break;
                        }
                    }
                }
                let mut ip = eng.mem[r7 as usize + 1] as i8 as i32;
                // f8dispatch + A-chain: decide redispatch r2v or jump.
                enum Route {
                    Redisp(i32),
                    D274,
                }
                let route: Route;
                {
                    let mut r0v = ip * 137;
                    let r1v = r0v >> 12;
                    r0v = r1v - (r0v >> 31);
                    if r0v == 1 {
                        eng.mem[r7 as usize + 1] = (ip - 30) as u8;
                        route = Route::Redisp(0);
                    } else if r0v == 2 {
                        eng.mem[r7 as usize + 1] = (ip - 60) as u8;
                        route = Route::Redisp(1);
                    } else if r0v == 3 {
                        eng.mem[r7 as usize + 1] = (ip - 90) as u8;
                        route = Route::Redisp(0);
                    } else {
                        let t = s16_at(eng.mem, 0xE700 + 68, 0) as i32;
                        if t == 1 {
                            // do_19c chain
                            if r8 == 2 {
                                // do_1b4
                                if ip == 0 {
                                    eng.mem[r7 as usize + 1] = 1;
                                    route = Route::Redisp(-1);
                                } else if ip != 1 {
                                    // do_24c
                                    if ip == 2 {
                                        route = Route::D274;
                                    } else if r8 > ip
                                        && r8 != 2
                                        && ((r8 as u32 & 0x80000001) == 1)
                                    {
                                        route = Route::Redisp(-1);
                                    } else {
                                        route = Route::Redisp(1);
                                    }
                                } else {
                                    eng.mem[r7 as usize + 1] = 0;
                                    route = Route::Redisp(-1);
                                }
                            } else if r8 < 2 || ip != 1 {
                                // do_24c
                                if ip == 2 {
                                    route = Route::D274;
                                } else if r8 > ip && r8 != 2 && ((r8 as u32 & 0x80000001) == 1)
                                {
                                    route = Route::Redisp(-1);
                                } else {
                                    route = Route::Redisp(1);
                                }
                            } else {
                                // do_1d4
                                if eng.mem[r6 as usize + 5] == 76 {
                                    // do_254 -> do_24c_mid
                                    if r8 > ip && r8 != 2 && ((r8 as u32 & 0x80000001) == 1) {
                                        route = Route::Redisp(-1);
                                    } else {
                                        route = Route::Redisp(1);
                                    }
                                } else {
                                    let (s0len, ch) = scan_len(&eng.mem[..], r6 as usize);
                                    let r2vs = ch;
                                    let str2 = eng.rd32(r6 as usize + 16) as usize;
                                    let (s0b, last3) = scan_len(&eng.mem[..], str2);
                                    let _ = (s0len, s0b);
                                    if r2vs != last3 {
                                        let q0 = eng.mem[str2 + 4] as i32;
                                        if q0 != 81 {
                                            eng.mem[r7 as usize + 1] = 2;
                                            route = Route::D274;
                                        } else if ip == 2 {
                                            route = Route::D274;
                                        } else if r8 > ip
                                            && r8 != 2
                                            && ((r8 as u32 & 0x80000001) == 1)
                                        {
                                            route = Route::Redisp(-1);
                                        } else {
                                            route = Route::Redisp(1);
                                        }
                                    } else if ip == 2 {
                                        route = Route::D274;
                                    } else if r8 > ip
                                        && r8 != 2
                                        && ((r8 as u32 & 0x80000001) == 1)
                                    {
                                        route = Route::Redisp(-1);
                                    } else {
                                        route = Route::Redisp(1);
                                    }
                                }
                            }
                        } else if t != 2 {
                            route = Route::Redisp(0);
                        } else {
                            // do_308
                            if r8 <= 3 {
                                eng.mem[r7 as usize + 1] = r8 as u8;
                            } else {
                                let mut tt = r8 + (r8 >> 31);
                                tt &= !1;
                                eng.mem[r7 as usize + 1] = (r8 - tt + r8 - 1) as u8;
                            }
                            route = Route::Redisp(-1);
                        }
                    }
                }
                let mut to_out458 = false;
                let mut r2v = 0i32;
                let mut r1v = 0i32;
                let mut r3v = 3i32;
                let mut r0v = 0u32;
                let rid = match route { Route::D274 => 274, Route::Redisp(v) => 1000 + v };
                match route {
                    Route::D274 => {
                        r2v = -1;
                        let t0 = eng.rd16(r6 as usize + 12) as i32;
                        let t1 = r9.wrapping_shl(5).wrapping_add(32768);
                        put_s16(&mut eng.mem[..], r6 as usize + 12, 0, (t0 - (t1 >> 16)) as i16);
                    }
                    Route::Redisp(v) => {
                        r2v = v;
                        let b0 = eng.mem[r7 as usize + 1];
                        if b0 == 1 {
                            // do_334 -> out458
                            r9 -= r9 >> 2;
                            let mut t0 = r9 + 512;
                            t0 >>= 10;
                            t0 = s16_at(&eng.mem[..], r6 as usize + 12, 0) as i32 + t0;
                            put_s16(&mut eng.mem[..], r6 as usize + 12, 0, t0 as i16);
                            r0v = eng.rd32(r6 as usize + 16);
                            r1v = 1;
                            to_out458 = true;
                        } else {
                            if r2v == 1 {
                                let t1 = r9 + 512;
                                let t1 = t1 >> 10;
                                let t0 = s16_at(&eng.mem[..], r6 as usize + 12, 0) as i32 + t1;
                                put_s16(&mut eng.mem[..], r6 as usize + 12, 0, t0 as i16);
                            } else if r2v == -1 {
                                let t0 = eng.rd16(r6 as usize + 12) as i32;
                                let t1 = r9.wrapping_shl(5).wrapping_add(32768);
                                put_s16(&mut eng.mem[..], r6 as usize + 12, 0, (t0 - (t1 >> 16)) as i16);
                            }
                        }
                    }
                }
                if !to_out458 {
                    // do_28c:
                    ip = eng.mem[r7 as usize + 1] as i8 as i32;
                    r0v = eng.rd32(r6 as usize + 16);
                    r3v = 3;
                    if ip == 0 || ip > r8 {
                        r1v = r8;
                    } else if ip > 0 {
                        r1v = ip;
                    } else {
                        r0v = r6;
                        r3v = 2;
                        if ip != -1 {
                            r1v = -ip;
                        } else {
                            r1v = r8;
                        }
                    }
                    // do_2c8:
                    if r0v == 0 {
                        to_out458 = true;
                    } else {
                        let b4 = eng.mem[r0v as usize + 4];
                        if b4 == 81 {
                            r3v -= 1;
                            r3v = r3v as i16 as i32;
                            // ARM beqs from the b4==81 check straight to l3c8
                            // (0x1a0474), skipping the do_3c4 advance below.
                        } else if r8 > 1 {
                            if r2v == 0 {
                                let mut t4 = r9 + (r9 << 1);
                                t4 += 0x800;
                                let tip = s16_at(&eng.mem[..], r0v as usize + 12, 0) as i32;
                                t4 >>= 12;
                                put_s16(&mut eng.mem[..], r0v as usize + 12, 0, (tip + t4) as i16);
                            } else if r2v == -1 {
                                if ip == 2 || (r8 == 2 && ip == 0) {
                                    let t4 = r9 + 512;
                                    let tip = s16_at(&eng.mem[..], r0v as usize + 12, 0) as i32;
                                    put_s16(&mut eng.mem[..],
                                        r0v as usize + 12,
                                        0,
                                        (tip + (t4 >> 10)) as i16,
                                    );
                                }
                            } else {
                                let t4 = r9 + 512;
                                let tip = s16_at(&eng.mem[..], r0v as usize + 12, 0) as i32;
                                put_s16(&mut eng.mem[..], r0v as usize + 12, 0, (tip + (t4 >> 10)) as i16);
                            }
                        }
                        if !to_out458 {
                            // do_3c4 advance, skipped on the b4==81 path
                            // (ARM beqs to l3c8 instead).
                            if b4 != 81 {
                                r0v = eng.rd32(r0v as usize + 16);
                            }
                            // l3c8:
                            if r3v <= r1v {
                                ip = (r9 + 512) >> 10;
                                // l3d8 loop:
                                loop {
                                    let skip_l3f4 = r2v != 0
                                        && r3v != r1v
                                        && (r2v != 1 || r3v > r1v);
                                    if !skip_l3f4 {
                                        let t4 =
                                            s16_at(&eng.mem[..], r0v as usize + 12, 0) as i32 + ip;
                                        put_s16(&mut eng.mem[..], r0v as usize + 12, 0, t4 as i16);
                                    }
                                    // do_400:
                                    let b4 = eng.mem[r0v as usize + 4];
                                    if b4 == 81 && r3v >= 2 {
                                        let t4 = eng.mem[r7 as usize + 1] as i8 as i32;
                                        if t4 != 0 && t4 <= r8 {
                                            r3v += 1;
                                            r3v = r3v as i16 as i32;
                                        }
                                    }
                                    // l42c:
                                    r3v += 1;
                                    r0v = eng.rd32(r0v as usize + 16);
                                    r3v = r3v as i16 as i32;
                                    if r3v > r1v {
                                        break;
                                    }
                                }
                            }
                            // do_440:
                            if r0v == 0 {
                                to_out458 = true;
                            } else {
                                let t3 = r8 - 1;
                                if t3 > r1v {
                                    if r2v != 0 {
                                        to_out458 = true;
                                    } else {
                                        let t1 = (r9 + 1024) >> 11;
                                        let t2 = s16_at(&eng.mem[..], r0v as usize + 12, 0) as i32;
                                        put_s16(&mut eng.mem[..], r0v as usize + 12, 0, (t2 + t1) as i16);
                                    }
                                } else {
                                    let r7b8 = eng.rd32(r7 as usize + 8) as usize;
                                    let t1 = eng.rd16(r7b8 + 18) as i32;
                                    // ARM skips the store unless all three hold
                                    // (sliceJ-3: the C stores unconditionally).
                                    if t1 == 0 && r8 > 1 && r2v == 0 {
                                        r2v = (r9 + 0x800) >> 12;
                                        let t1 = s16_at(&eng.mem[..], r0v as usize + 12, 0) as i32;
                                        let t1 = (t1 as i16 as i32) + r2v;
                                        put_s16(&mut eng.mem[..], r0v as usize + 12, 0, t1 as i16);
                                    }
                                }
                            }
                        }
                    }
                }
                // l4a0:
                let t1 = eng.mem[fp as usize] as i8 as i32;
                let t0 = sl + 1;
                r7 = eng.rd32(r7 as usize + 12);
                sl = t0 as i16 as i32;
                if t1 <= sl {
                    break;
                }
            }
        }
        // fp_next:
        fp = eng.rd32(fp as usize + 8);
        if fp == 0 {
            break;
        }
    }
    // Phase 2.
    let mut sp8 = fp0;
    let mut endp = 0;
    let mut e5 = 0i32;
    let mut sl = 0i32;
    'outer2: loop {
        if sp8 == 0 {
            break;
        }
        let mut r4b = sp8;
        let mut r1b = 0i32;
        // l4ec:
        loop {
            let cc = eng.mem[r4b as usize] as i8 as i32;
            let mut fpc = 0i32;
            let sp4 = cc;
            let mut r9p = eng.rd32(r4b as usize + 4);
            if cc > 0 {
                // l504:
                loop {
                    let lr = eng.mem[r9p as usize] as i8 as i32;
                    let mut ipmap = eng.rd32(r9p as usize + 4);
                    sl = 0;
                    if lr > 0 {
                        // l518:
                        loop {
                            let b3 = eng.mem[ipmap as usize + 4] as i8 as i32;
                            let mut dl = eng.rd32(ipmap as usize + 24);
                            let tt = b3 - 1;
                            if tt > 0 {
                                if (b3 & 1) == 0 {
                                    let s3 = s16_at(&eng.mem[..], dl as usize + 6, 0) as i32;
                                    let s5 = s16_at(&eng.mem[..], dl as usize + 8, 0) as i32;
                                    dl = eng.rd32(dl as usize + 16);
                                    r1b = r1b.wrapping_add(s3 + s5);
                                }
                                let mut tt = tt >> 1;
                                let mut r3v = 0i32;
                                if tt != 0 {
                                    loop {
                                        let s5 = s16_at(&eng.mem[..], dl as usize + 6, 0) as i32;
                                        let s6 = s16_at(&eng.mem[..], dl as usize + 8, 0) as i32;
                                        dl = eng.rd32(dl as usize + 16);
                                        tt -= 1;
                                        r1b = r1b.wrapping_add(s5 + s6);
                                        let s7v = s16_at(&eng.mem[..], dl as usize + 6, 0) as i32;
                                        let s8v = s16_at(&eng.mem[..], dl as usize + 8, 0) as i32;
                                        dl = eng.rd32(dl as usize + 16);
                                        r3v = r3v.wrapping_add(s7v + s8v);
                                        if tt == 0 {
                                            break;
                                        }
                                    }
                                }
                                r1b = r1b.wrapping_add(r3v);
                            }
                            let s0 = s16_at(&eng.mem[..], dl as usize + 6, 0) as i32;
                            let s3 = s16_at(&eng.mem[..], dl as usize + 8, 0) as i32;
                            let u5 = eng.rd16(ipmap as usize + 18) as i32;
                            r1b = r1b.wrapping_add(s0 + s3);
                            let mut jumped = false;
                            // l5a4: reached when u5 != 0 (fall-through) or when
                            // u5 == 0 with the link conditions (goto).
                            let mut do_l5a4 = u5 != 0;
                            if u5 == 0 {
                                let t0 = eng.rd32(r4b as usize + 8);
                                if t0 == 0 {
                                    let t1 = eng.rd32(r9p as usize + 12);
                                    if t1 == 0 {
                                        let t2 = eng.rd32(r9p as usize + 8);
                                        if t2 == ipmap {
                                            do_l5a4 = true;
                                        }
                                    }
                                }
                            }
                            if do_l5a4 {
                                let ta = eng.rd32(sp8 as usize + 8);
                                endp = dl;
                                if ta != 0 {
                                    mixer_6b(sp8 as usize, r1b, dl as usize, 0, e5,
                                        eng, tab,
                                    );
                                } else {
                                    let t7 = eng.mem[ipmap as usize + 7];
                                    mixer_6b(sp8 as usize,
                                        r1b,
                                        endp as usize,
                                        if t7 == 0 { 1 } else { 2 },
                                        e5,
                                        eng,
                                        tab,
                                    );
                                }
                                e5 = (fp0 != r4b) as i32;
                                sp8 = eng.rd32(sp8 as usize + 8);
                                jumped = true;
                            }
                            if jumped {
                                continue 'outer2;
                            }
                            sl = (sl + 1) as i16 as i32;
                            ipmap = eng.rd32(ipmap as usize + 28);
                            if lr <= sl {
                                break;
                            }
                        }
                    }
                    // l61c:
                    let nn = fpc + 1;
                    r9p = eng.rd32(r9p as usize + 12);
                    fpc = nn as i16 as i32;
                    if sp4 <= fpc {
                        break;
                    }
                }
            }
            // next_r4:
            r4b = eng.rd32(r4b as usize + 8);
            if r4b == 0 {
                break 'outer2;
            }
            r1b = 0;
        }
    }
    0
}

// --- batch C: list builder ---

/// fn_1a50f8: phrase list builder over the node chain at engine `r0`
/// (0 = empty), emitting cursor lists at `r1`/`r2`. Stored links are
/// engine offsets (the test re-adds the base on pointer words).
pub fn list_build(eng: &mut Engine, r0: u32, r1: usize, r2: usize) {
    eng.mem[r1] = 0;
    if r0 == 0 {
        eng.wr32(r1 + 12, 0);
        eng.wr32(r2 + 8, 0);
        return;
    }
    let first = r0;
    let flag = 1u8;
    let mut r0 = r0 as usize;
    let (mut r1, mut r2) = (r1, r2);
    loop {
        let mut c = eng.mem[r0 + 8] as i32;
        if c == 0 {
            c = eng.mem[r0 + 9] as i32;
        }
        let cond = c != 0
            || eng.mem[r1] as i8 == 0
            || eng.rd16(r0.wrapping_sub(14)) != 0
            || r0 as u32 == first;
        if cond {
            eng.wr32(r1 + 12, (r1 + 16) as u32);
            r1 += 12;
            eng.wr32(r1 + 8, r0 as u32);
            eng.wr32(r1 + 12, r0 as u32);
            r1 += 4;
            eng.mem[r1] = flag;
            eng.mem[r1 + 2] = eng.mem[r0 + 8];
            eng.mem[r1 + 1] = eng.mem[r0 + 6];
            if eng.mem[r0 + 9] != 0 || eng.rd16(r0.wrapping_sub(14)) != 0 || r0 as u32 == first
            {
                eng.wr32(r2 + 8, (r2 + 12) as u32);
                r2 += 8;
                eng.wr32(r2 + 8, r1 as u32);
                r2 += 4;
                eng.mem[r2] = flag;
                eng.mem[r2 + 1] = eng.mem[r0 + 9];
            } else {
                eng.mem[r2] = eng.mem[r2].wrapping_add(1);
            }
        } else {
            let s = (eng.mem[r1] as i8 as i32) + 1;
            eng.wr32(r1 + 8, r0 as u32);
            eng.mem[r1] = s as u8;
        }
        r0 = eng.rd32(r0 + 28) as usize;
        if r0 == 0 {
            break;
        }
    }
    eng.wr32(r1 + 12, 0);
    eng.wr32(r2 + 8, 0);
}

// --- batch C: phrase driver ---

/// fn_1a1298: phrase-range driver over 14-byte engine nodes at `p` in voice
/// memory, dispatching `phrase_emit` per range with merge handling.
pub fn phrase_drive(eng: &mut Engine, p: usize, tab: &Tables) {
    let mut cnt = 0i16;
    let mut cur = p;
    let mut mark = p;
    let mut anchor;
    let mut outer = 0i16;
    let mut state = -1i32;
    if eng.mem[p] == 7 {
        return;
    }
    loop {
        let mut t = eng.mem[cur] as u32;
        anchor = mark;
        while t > 6 {
            if t == 7 {
                return;
            }
            if cnt >= 256 {
                return;
            }
            cur += 14;
            t = eng.mem[cur] as u32;
            cnt = cnt.wrapping_add(1);
        }
        if cnt >= 256 {
            return;
        }
        if t == 0 {
            mark = cur.wrapping_sub(14);
            cnt = cnt.wrapping_add(1);
            state = -2;
            cur += 14;
        } else {
            mark = cur;
            cnt = cnt.wrapping_add(1);
            cur += 14;
        }
        let to_merge = eng.mem[mark + 12] == 0x51
            && anchor != mark
            && matches!(
                eng.mem[mark.wrapping_sub(14)],
                26 | 25 | 21 | 19 | 18 | 15 | 20 | 23 | 24
            );
        if !to_merge {
            let mut mode = state;
            let mut aa = anchor;
            let span = (mark as u32).wrapping_sub(anchor as u32) as i32 / 14;
            if eng.mem[0x1b00 + 33] == 1 && state == 0 && span == 2 {
                let k = eng.mem[anchor + 14];
                if matches!(k, 27 | 12 | 8 | 36 | 28 | 11 | 13 | 37) {
                    let tmp = k;
                    eng.mem[anchor + 14] = 0;
                    phrase_emit(1, anchor, anchor + 14, eng, tab);
                    eng.mem[anchor + 14] = tmp;
                    mode = -1;
                    aa = anchor + 14;
                }
            }
            let rc = phrase_emit(mode, aa, mark, eng, tab);
            if rc >= 0 {
                // update
                if state != -2 {
                    state = 0;
                } else {
                    mark = cur;
                }
                outer = outer.wrapping_add(1);
                if eng.mem[cur] == 7 {
                    break;
                }
                continue;
            }
        }
        // merge:
        if state >= 0 {
            if eng.mem[anchor] == eng.mem[mark] && anchor + 14 == mark {
                let mut s = s16_at(&eng.mem[..], mark + 2, 0) as i32 + s16_at(&eng.mem[..], anchor + 2, 0) as i32;
                if s >= 0x8000 {
                    s = 0x7fff;
                }
                put_s16(&mut eng.mem[..], mark + 2, 0, s as i16);
                phrase_emit(state, anchor + 14, mark, eng, tab);
            } else {
                let pm = mark.wrapping_sub(14);
                let rc1 = phrase_emit(1, anchor, pm, eng, tab);
                if rc1 < 0 {
                    let b = eng.mem[pm];
                    if b == 18 {
                        eng.mem[pm] = 2;
                    } else if b == 20 {
                        eng.mem[pm] = 3;
                    } else if b == 16 {
                        eng.mem[pm] = 15;
                    }
                    if phrase_emit(1, anchor, pm, eng, tab) < 0 {
                        // ARM orig 0x1A153C passes -2 here, not -1.
                        phrase_emit(-2, anchor, anchor, eng, tab);
                    }
                    eng.mem[pm] = b;
                }
                mark = pm + 14;
                phrase_emit(-1, pm, mark, eng, tab);
                state = 0;
            }
        }
        // update:
        if state != -2 {
            state = 0;
        } else {
            mark = cur;
        }
        outer = outer.wrapping_add(1);
        if eng.mem[cur] == 7 {
            break;
        }
    }
    if eng.mem[mark] != 7 {
        // ARM orig 0x1A1598 passes -2 here, not -1.
        phrase_emit(-2, mark, mark, eng, tab);
    }
}

/// fn_19f01c: `#` command parser over the 0xE700 param block.
/// Reached once per render with p1=1 and non-`#` romaji text, which
/// returns 0 with no writes (ARM-verified: `ldrb/cmp #35/movne/bne`).
/// The `#` switch body never fires on the corpus; translated from C.
/// fn_19f01c: `#` command parser over the 0xE700 param block.
///
/// ARM-verified command map (snapshot runner): plain form SETs the slot,
/// `+` form AD identical (`-` counts as `+`, `+-N` is `-N` via text[3..]);
/// p1 selects the voice slot (p1==0) or its +0x30 shadow (p1==1).
/// Letters: A E704/34 (0..400), B E702/32 warp (0xDAC..0xDAC*2),
/// C enum (1/2 -> E710, other nonzero -> E714, 0 -> nothing),
/// D/E mirror +/-100/5/5/10/10, F resets warp, G E71C/4C (0..100),
/// I E70E/3E, J E70C/3C (0..100), M warp base, P E706/36,
/// R E716/46 (0..1000, negatives keep), S E70A/3A (floor 20),
/// T E72C/5C (plain only), V E708/38 (cap 360), X quality floor/computed,
/// Z copies E70A/E70C to E73A/E73C, Q and anything else are no-ops.
pub fn tag_command(eng: &mut Engine, text: &[u8], p1: i32) -> i32 {
    use crate::jp::libc::strtol10;
    if text[0] != b'#' {
        return 0;
    }
    let c1 = text[1] as i8;
    if c1 == b'M' as i8 {
        eng.wr16(0xE700 + 50, 0x1200);
        // ARM orig returns -3 here (slot456=-3 path in token_units).
        return -3;
    }
    if c1 == b'r' as i8 {
        let v = strtol10(&text[2..]);
        if p1 == 0 {
            eng.wr16(0xE700 + 32, v as u16);
        } else {
            eng.wr16(0xE700 + 80, v as u16);
        }
        return -3;
    }
    if c1 > b'M' as i8 {
        if c1 == b'V' as i8 {
            let slot = if p1 == 0 { 0xE700 + 8 } else { 0xE700 + 56 };
            if text[2] == b'+' {
                let v = strtol10(&text[3..]);
                let s = (eng.rds16(slot) as i32).wrapping_add(v);
                eng.wr16(slot, s.min(360) as u16);
            } else {
                let v = strtol10(&text[2..]);
                eng.wr16(slot, v.min(360) as u16);
            }
            return -3;
        }
        if c1 == b'P' as i8 {
            let slot = if p1 == 0 { 0xE700 + 6 } else { 0xE700 + 54 };
            if text[2] == b'+' {
                let v = strtol10(&text[3..]);
                let s = (eng.rds16(slot) as i32).wrapping_add(v);
                eng.wr16(slot, s.max(0) as u16);
            } else {
                let v = strtol10(&text[2..]);
                eng.wr16(slot, v.max(0) as u16);
            }
            return -3;
        }
        if c1 == b'R' as i8 {
            let v = strtol10(&text[2..]);
            if v < 0 {
                return -3;
            }
            let cl = if v > 1000 { 1000 } else { v as i16 };
            if p1 == 0 {
                eng.wr16(0xE700 + 22, cl as u16);
                return -3;
            }
            clear_dee8(eng);
            eng.wr16(0xE700 + 70, cl as u16);
            return -3;
        }
        if c1 == b'S' as i8 {
            let slot = if p1 == 0 { 0xE700 + 10 } else { 0xE700 + 58 };
            if text[2] == b'+' {
                let v = strtol10(&text[3..]);
                let s = (eng.rds16(slot) as i32).wrapping_add(v);
                eng.wr16(slot, s.max(20) as u16);
            } else {
                let v = strtol10(&text[2..]);
                eng.wr16(slot, v.max(20) as u16);
            }
            return -3;
        }
        if c1 == b'T' as i8 {
            if text[2] == b'+' {
                return -3;
            }
            let v = strtol10(&text[2..]);
            if p1 == 0 {
                eng.wr16(0xE700 + 44, v as u16);
            } else {
                eng.wr16(0xE700 + 92, v as u16);
            }
            return -3;
        }
        if c1 == b'X' as i8 {
            let v = strtol10(&text[2..]);
            // ARM orig clamps to 80..130 and calls quality_set (0x1a2e38),
            // then returns -3; the old hardcoded w1=810 mismatched its
            // computed 818.
            let x = v.clamp(80, 130);
            quality_set(eng, x, p1);
            return -3;
        }
        if c1 == b'Z' as i8 {
            eng.wr16(0xE700 + 58, eng.rd16(0xE700 + 10));
            eng.wr16(0xE700 + 60, eng.rd16(0xE700 + 12));
            return -3;
        }
        return -3;
    }
    let v0 = c1 as i32 - b'A' as i32;
    if (v0 as u32) < 10 {
        match v0 {
            0 => {
                // do_A
                let v = strtol10(&text[2..]);
                let r1 = if v < 0 { 0 } else if v > 400 { 400 } else { v as i16 };
                if p1 == 0 {
                    eng.wr16(0xE700 + 4, r1 as u16);
                } else {
                    eng.wr16(0xE700 + 52, r1 as u16);
                }
                return -3;
            }
            1 => {
                // do_B: warp current (p1==0) or base (p1==1).
                let slot = if p1 == 0 { 0xE700 + 2 } else { 0xE700 + 50 };
                if text[2] == b'+' {
                    let v = strtol10(&text[3..]);
                    let s = (eng.rds16(slot) as i32).wrapping_add(v);
                    eng.wr16(slot, clamp_dac(s) as u16);
                } else {
                    let v = strtol10(&text[2..]);
                    eng.wr16(slot, clamp_dac(v) as u16);
                }
                return -3;
            }
            2 => {
                // do_C: phrase enum; 0 keeps, 1/2 select, else flag.
                let v = strtol10(&text[2..]);
                let (w10, w14) = if p1 == 0 {
                    (0xE700 + 16, 0xE700 + 20)
                } else {
                    (0xE700 + 64, 0xE700 + 68)
                };
                if v == 1 {
                    eng.wr16(w10, 1);
                } else if v == 2 {
                    eng.wr16(w10, 2);
                } else if v != 0 {
                    eng.wr16(w14, 1);
                }
                return -3;
            }
            3 => {
                // do_D
                eng.wr16(0xE700 + 50, clamp_dac(eng.rds16(0xE700 + 50) as i32 - 100) as u16);
                eng.wr16(0xE700 + 52, clamp_u(eng.rds16(0xE700 + 52) as i32 - 5) as u16);
                eng.wr16(0xE700 + 54, clamp_u(eng.rds16(0xE700 + 54) as i32 - 5) as u16);
                eng.wr16(0xE700 + 56, eng.rds16(0xE700 + 56).wrapping_sub(10) as u16);
                eng.wr16(0xE700 + 58, eng.rds16(0xE700 + 58).wrapping_add(10) as u16);
                return -3;
            }
            4 => {
                // do_E
                eng.wr16(0xE700 + 50, clamp_dac(eng.rds16(0xE700 + 50) as i32 + 100) as u16);
                eng.wr16(0xE700 + 52, clamp_u(eng.rds16(0xE700 + 52) as i32 + 5) as u16);
                eng.wr16(0xE700 + 54, clamp_u(eng.rds16(0xE700 + 54) as i32 + 5) as u16);
                eng.wr16(0xE700 + 56, eng.rds16(0xE700 + 56).wrapping_add(10) as u16);
                eng.wr16(0xE700 + 58, eng.rds16(0xE700 + 58).wrapping_sub(10) as u16);
                return -3;
            }
            5 => {
                // do_F: warp reset.
                eng.wr16(0xE700 + 50, 0x13C2);
                eng.wr8(0xE730, 0);
                return -3;
            }
            6 => {
                // do_G
                let v = strtol10(&text[2..]);
                let r1 = v.clamp(0, 100);
                if p1 == 0 {
                    eng.wr16(0xE700 + 28, r1 as u16);
                } else {
                    eng.wr16(0xE700 + 76, r1 as u16);
                }
                return -3;
            }
            8 => {
                // do_I
                let v = strtol10(&text[2..]);
                if p1 == 0 {
                    eng.wr16(0xE700 + 14, v as u16);
                } else {
                    eng.wr16(0xE700 + 62, v as u16);
                }
                return -3;
            }
            9 => {
                // do_J
                let v = strtol10(&text[2..]);
                let r1 = v.clamp(0, 100);
                if p1 == 0 {
                    eng.wr16(0xE700 + 12, r1 as u16);
                } else {
                    eng.wr16(0xE700 + 60, r1 as u16);
                }
                return -3;
            }
            _ => return -3,
        }
    }
    // Anything unhandled (K, L, N, O, Q, non-letters like SJIS lead bytes):
    // ARM falls through to its -3 return (slot456=-3 path in token_units).
    -3
}

/// Warp clamp shared by the `#B` setter: 0xDAC..0xDAC*2.
#[inline]
fn clamp_dac(v: i32) -> i16 {
    if v < 0xDAC {
        0xDAC
    } else if v > 0xDAC * 2 {
        0xDAC * 2
    } else {
        v as i16
    }
}

/// 0..400 clamp shared by the `#D`/`#E` slot trims.
#[inline]
fn clamp_u(v: i32) -> i16 {
    if v < 0 {
        0
    } else if v > 400 {
        400
    } else {
        v as i16
    }
}

/// fn_19c2e8 case 1/default: voice preset lookup + `#` replay.
fn preset_voice(eng: &mut Engine, tab: &[u8], val: i32) -> i32 {
    use crate::jp::libc::{strchr0, strcmp0};
    let preset: &[u8] = match val {
        0x101 => b"<\\Spk_male",
        0x102 => b"<\\Spk_female",
        0x103 => b"<\\Spk_admale",
        0x104 => b"<\\Spk_adfemale",
        0x105 => b"<\\Spk_emale",
        0x106 => b"<\\Spk_efemale",
        0x107 => b"<\\Spk_chmale",
        0x108 => b"<\\Spk_chfemale",
        0x109 => b"<\\Spk_sisters",
        0x10a => b"<\\Spk_meimu",
        0x10b => b"<\\Spk_space",
        0x10c => b"<\\Spk_giant",
        _ => return -1,
    };
    let mut entry = None;
    if strcmp0(preset, &tab[0..]) == 0 {
        entry = Some(20);
    } else {
        let mut idx = 1;
        loop {
            if strcmp0(preset, &tab[idx * 120..]) == 0 {
                entry = Some(idx * 120 + 20);
                break;
            }
            if strcmp0(preset, &tab[idx * 120 + 120..]) == 0 {
                entry = Some(idx * 120 + 140);
                break;
            }
            idx += 2;
            if idx >= 53 {
                return 0;
            }
        }
    }
    let mut q = entry.unwrap_or(0);
    if q == 0 {
        return 0;
    }
    match strchr0(&tab[q..], 0x23) {
        Some(o) => q += o,
        None => return 0,
    }
    loop {
        tag_command(eng, &tab[q..], 0);
        match strchr0(&tab[q + 1..], 0x23) {
            Some(o) => q = q + 1 + o,
            None => return 0,
        }
    }
}

/// fn_19c2e8: setParam. Reached ids: 3, 4, 6, 7, 8, 10, 11, 13, 14
/// (range-check + store; 6 routes to `quality_set`). `tab` borrows the
/// ROM preset table at D_715514. Ids 1/2 use presets; 0 and 17+ fail
/// (ARM-verified: only ids 1..16 are valid).
pub fn set_param(eng: &mut Engine, tab: &[u8], id: i32, val: i32) -> i32 {
    use crate::jp::libc::{strchr0, strcmp0};
    if flag_busy(eng) != 0 {
        return -1;
    }
    match id {
        0 => -1,
        1 => preset_voice(eng, tab, val),
        2 => {
            let preset: &[u8] = match val {
                0x201 => b"<\\angry",
                0x202 => b"<\\business",
                0x203 => b"<\\calm",
                0x204 => b"<\\depressed",
                0x205 => b"<\\excited",
                0x206 => b"<\\falsetto",
                0x207 => b"<\\happy",
                0x208 => b"<\\loud",
                0x209 => b"<\\monotone",
                0x20a => b"<\\perky",
                0x20b => b"<\\quiet",
                0x20c => b"<\\sarcastic",
                0x20d => b"<\\scared",
                0x20e => b"<\\shout",
                0x20f => b"<\\tence",
                0x210 => b"<\\whisper",
                0x211 => b"<\\sing",
                _ => return -1,
            };
            let mut entry = None;
            if strcmp0(preset, &tab[0..]) == 0 {
                entry = Some(20);
            } else {
                let mut idx = 1;
                loop {
                    if strcmp0(preset, &tab[idx * 120..]) == 0 {
                        entry = Some(idx * 120 + 20);
                        break;
                    }
                    if strcmp0(preset, &tab[idx * 120 + 120..]) == 0 {
                        entry = Some(idx * 120 + 140);
                        break;
                    }
                    idx += 2;
                    if idx >= 53 {
                        return 0;
                    }
                }
            }
            let mut q = entry.unwrap_or(0);
            if q == 0 {
                return 0;
            }
            match strchr0(&tab[q..], 0x23) {
                Some(o) => q += o,
                None => return 0,
            }
            loop {
                tag_command(eng, &tab[q..], 0);
                match strchr0(&tab[q + 1..], 0x23) {
                    Some(o) => q = q + 1 + o,
                    None => return 0,
                }
            }
        }
        3 => {
            if (val.wrapping_sub(20) as u32) >= 0x885 {
                return -1;
            }
            eng.wr16(0xE70A, val as u16);
            0
        }
        4 => {
            if (val.wrapping_sub(3500) as u32) >= 0xDAD {
                return -1;
            }
            eng.wr16(0xE702, val as u16);
            0
        }
        5 => {
            if (val.wrapping_add(0x2710) as u32) >= 0x2879 {
                return -1;
            }
            eng.wr16(0xE708, val as u16);
            0
        }
        6 => {
            if (val.wrapping_sub(80) as u32) >= 51 {
                return -1;
            }
            quality_set(eng, val, 0);
            0
        }
        7 => {
            if (val as u32) >= 0x1389 {
                return -1;
            }
            eng.wr32(0xE720, val as u32);
            0
        }
        8 => {
            if (val as u32) >= 0xBB9 {
                return -1;
            }
            eng.wr16(0xE70E, val as u16);
            0
        }
        9 => {
            if (val as u32) >= 101 {
                return -1;
            }
            eng.wr16(0xE70C, val as u16);
            0
        }
        10 => {
            if (val as u32) > 1000 {
                return -1;
            }
            eng.wr16(0xE716, val as u16);
            0
        }
        11 => {
            if (val as u32) >= 101 {
                return -1;
            }
            eng.wr32(0xE71C, val as u32);
            0
        }
        12 => {
            if (val as u32) >= 4 {
                return -1;
            }
            eng.wr16(0xE714, 0);
            eng.wr32(0xE710, 0);
            if val == 1 {
                eng.wr32(0xE710, 1);
                return 0;
            }
            if val == 2 {
                eng.wr32(0xE710, 2);
                return 0;
            }
            if val == 3 {
                eng.wr16(0xE714, 1);
            }
            0
        }
        13 => {
            if (val as u32) >= 101 {
                return -1;
            }
            eng.wr32(0xE718, val as u32);
            0
        }
        14 => {
            if (val as u32) > 400 {
                return -1;
            }
            eng.wr16(0xE704, val as u16);
            0
        }
        15 => {
            if (val as u32) > 400 {
                return -1;
            }
            eng.wr16(0xE706, val as u16);
            0
        }
        16 => {
            if (val as u32) > 1000 {
                return -1;
            }
            eng.wr16(0xE72C, val as u16);
            0
        }
        _ => -1,
    }
}

/// fn_19bb58: getParam. Ids 3..16 read the param fields back (s16 reads
/// sign-extend; id 12 rebuilds the enum: nonzero E714 means 3, else the
/// E710 word); every other id yields INT_MIN.
pub fn get_param(eng: &Engine, id: i32) -> i32 {
    match id {
        3 => eng.rds16(0xE70A) as i32,
        4 => eng.rds16(0xE702) as i32,
        5 => eng.rds16(0xE708) as i32,
        6 => eng.rd32(0xE724) as i32,
        7 => eng.rd32(0xE720) as i32,
        8 => eng.rds16(0xE70E) as i32,
        9 => eng.rds16(0xE70C) as i32,
        10 => eng.rds16(0xE716) as i32,
        11 => eng.rd32(0xE71C) as i32,
        12 => {
            if eng.rds16(0xE714) != 0 {
                3
            } else {
                eng.rd32(0xE710) as i32
            }
        }
        13 => eng.rd32(0xE718) as i32,
        14 => eng.rds16(0xE704) as i32,
        15 => eng.rds16(0xE706) as i32,
        16 => eng.rds16(0xE72C) as i32,
        _ => -2147483647 - 1,
    }
}

/// 32-bit wrapping address add for guest pointers held as offsets.
#[inline]
fn aadd(a: usize, d: i32) -> usize {
    (a as u32).wrapping_add(d as u32) as usize
}

/// Odd + paired stride slot fill: `n` slots of `w124 + i*10` stored at
/// `w128 + (i<<2)` (`mora_align` shared tail; `w128` not advanced).
fn fill_stride(eng: &mut Engine, n: i32, w124: usize, w128: usize) {
    let mut f2 = n & 1;
    let mut f3 = 0;
    if f2 == 1 {
        f3 = 1;
        eng.wr32(w128, w124 as u32);
    }
    if f2 < n {
        loop {
            let t = f3 + (f3 << 2);
            f2 += 2;
            eng.wr32(aadd(w128, f3 << 2), aadd(w124, t << 1) as u32);
            f3 += 1;
            f2 = f2 as i16 as i32;
            let t = f3 + (f3 << 2);
            eng.wr32(aadd(w128, f3 << 2), aadd(w124, t << 1) as u32);
            f3 += 1;
            if f2 >= n {
                break;
            }
        }
    }
}

/// Odd + paired const slot fill: `n` slots of `v` at advancing `w128`.
/// Returns the advanced `w128`.
fn fill_const(eng: &mut Engine, n: i32, v: u32, mut w128: usize) -> usize {
    if (n & 1) != 0 {
        eng.wr32(w128, v);
        w128 += 4;
    }
    let mut t = n >> 1;
    if t != 0 {
        loop {
            eng.wr32(w128, v);
            w128 += 4;
            t -= 1;
            eng.wr32(w128, v);
            w128 += 4;
            if t == 0 {
                break;
            }
        }
    }
    w128
}

/// fn_19d1f0: mora alignment pass. `a0`/`a1` bound the 14-byte mora
/// list, `a2`/`a3` the frame/slot tables being built (all engine
/// offsets; slots store engine-relative offsets). Returns the s16
/// frame total. The row harness keeps `a1 - a0` realistic so the
/// `s60` loop and slot walks stay in-engine, as on ARM.
pub fn mora_align(
    eng: &mut Engine,
    tab: &Tables,
    a0: usize,
    a1: usize,
    a2: usize,
    a3: usize,
) -> i32 {
    let t7439 = tab.t7439;
    let tpre = tab.t7439pre;
    let k944 = tab.k743944;
    let mut r4 = a0;
    let diff = a1.wrapping_sub(a0) as u32 as i32;
    let r6: i32 = 0x92492493u32 as i32;
    let sp50 = 58880usize;
    let sp4c = 0xafe9usize;
    let mut s124 = a2;
    let mut s128 = a3;
    let mut s48: i32 = 0;
    let mut acc =
        ((diff as u32 as u64) << 32).wrapping_add((r6 as i64).wrapping_mul(diff as i64) as u64);
    let _ = acc as u32 as i32;
    let mut ip = (acc >> 32) as u32 as i32;
    if eng.rds8(sp50 + 202) as i32 == -1 || eng.rds8(sp50 + 202) as i32 == 2 {
        eng.wr8(sp4c, 255);
    } else {
        eng.wr8(sp4c, 1);
    }
    let mut r5 = ip >> 3;
    ip = (r5 - (ip >> 31)) + 1;
    let mut lr: i32 = 0;
    if ip <= 0 {
        return 0;
    }
    r5 = 0;
    ip = diff;
    acc = ((ip as u32 as u64) << 32).wrapping_add((r6 as i64).wrapping_mul(diff as i64) as u64);
    r5 = acc as u32 as i32;
    ip = (acc >> 32) as u32 as i32;
    r5 = diff;
    let mut r3b = aadd(r4, -12);
    let r7i = ip;
    ip >>= 3;
    ip -= r7i >> 31;
    let mut r7: i32 = lr;
    acc = ((((r5 as u32 as u64) << 32) | (r7 as u32 as u64)))
        .wrapping_add((r6 as i64).wrapping_mul(diff as i64) as u64);
    r7 = acc as u32 as i32;
    r5 = (acc >> 32) as u32 as i32;
    let mut r2: i32 = 0;
    if (ip & 1) == 0 {
        r2 = eng.rds16(r3b + 14) as i32;
        r3b += 14;
    }
    let r8full = r5;
    r5 >>= 3;
    r5 -= r8full >> 31;
    r5 += 1;
    ip = 0;
    let mut pairs = r5 >> 1;
    if pairs != 0 {
        loop {
            let h1 = eng.rd16(r3b + 14) as i32;
            let h2 = eng.rd16(r3b + 28) as i32;
            r3b += 28;
            pairs -= 1;
            r2 += h1;
            ip += h2;
            r2 = r2 as i16 as i32;
            ip = ip as i16 as i32;
            if pairs == 0 {
                break;
            }
        }
    }
    let mut r3 = diff;
    acc = ((((r3 as u32 as u64) << 32) | (lr as u32 as u64)))
        .wrapping_add((r6 as i64).wrapping_mul(diff as i64) as u64);
    lr = acc as u32 as i32;
    r3 = (acc >> 32) as u32 as i32;
    r2 += ip;
    r2 = r2 as i16 as i32;
    let s44 = r2;
    r2 = r3 >> 3;
    r2 -= r3 >> 31;
    r2 += 1;
    r2 = r2 as i16 as i32;
    if r2 == 0 {
        return 0;
    }
    let s60 = r2;
    let w54 = 32000usize;
    let mut c1 = eng.rd8(a1 + 13) as i32 - eng.rd8(r4 + 12) as i32;
    let mut s64v: i32 = 0;
    let mut s56: i32 = 0;
    c1 += 1;
    r5 = 0;
    eng.wr16(w54 + 244, 0);
    let s72 = c1;
    let s88 = eng.rds8(r4 + 14) as i32;
    if s60 <= 0 {
        return s64v;
    }
    let w5c = 0xafe6usize;
    let w60a = 0xafe4usize;
    let w64 = 0xafe5usize;
    let w68 = 0xafe8usize;
    let w6c = 0xafe7usize;
    let w70 = 44800usize;
    let _ = lr;
    loop {
        // mora:
        r7 = eng.rds16(r4 + 2) as i32;
        if r7 < 0 {
            // next:
            r5 = (r5 + 1) as i16 as i32;
            r4 += 14;
            if r5 < s60 {
                continue;
            }
            break;
        }
        if !(s60 <= r5 + 1) {
            let b26 = eng.rd8(r4 + 26) as i32;
            let b27 = eng.rd8(r4 + 27) as i32;
            if b26 == b27 && (t7439[eng.rd8(r4) as usize] as i32 & 0x80) != 0 {
                r4 += 14;
                r5 = (r5 + 1) as i16 as i32;
                r7 = (eng.rd16(r4 + 2) as i32 + r7) as i16 as i32;
            }
        }
        // spcheck:
        if eng.rds8(sp50 + 202) as i32 == 0 {
            let s88b = if s88 >= 0 {
                t7439[s88 as usize] as i32
            } else {
                tpre[(s88 + 128) as usize] as i32
            };
            if (s88b & 0x56) == 0x40 {
                // pdiff/pjoin:
                let p26 = eng.rd8(r4 + 26) as i32;
                let p27 = eng.rd8(r4 + 27) as i32;
                let mut r0v: i32;
                if p26 != p27 {
                    r0v = p27 - p26 + 1;
                } else {
                    let p40 = eng.rd8(r4 + 40) as i32;
                    let p41 = eng.rd8(r4 + 41) as i32;
                    if p40 != p41 {
                        r0v = p41 - p40 + 1;
                    } else {
                        let p54 = eng.rd8(r4 + 54) as i32;
                        let p55 = eng.rd8(r4 + 55) as i32;
                        r0v = if p54 == p55 { 0 } else { p55 - p54 + 1 };
                    }
                }
                if s72 <= s44 {
                    // c340 side:
                    if r5 == 0 {
                        // c384:
                        if s60 > 1 {
                            eng.wr8(sp4c, 1);
                            if (t7439[eng.rd8(r4 + 14) as usize] as i32 & 0x40) != 0 {
                                let d16 = eng.rds16(r4 + 16) as i32;
                                let u2 = eng.rd16(r4 + 2) as i32;
                                r0v = d16 - r0v;
                                r0v = u2 + (r0v >> 1);
                                r7 = r0v as i16 as i32;
                                s48 = r7;
                            }
                        }
                    } else if r5 < 0 {
                        // emit
                    } else if r5 <= 1 || s56 == 0 {
                        // c3cc:
                        if (t7439[eng.rd8(r4) as usize] as i32 & 0x40) != 0 {
                            eng.wr8(sp4c, 0);
                            r7 = eng.rd8(r4 + 13) as i32 - eng.rd8(r4 + 12) as i32 + 1;
                            s56 = r7;
                        } else {
                            // c400:
                            r7 = (s44 - s48) as i16 as i32;
                            eng.wr8(sp4c, r0v as u8);
                        }
                    } else {
                        // c340 tail:
                        r7 = (s44 - s56 - s48) as i16 as i32;
                        eng.wr8(sp4c, 255);
                    }
                } else if r5 == 0 {
                    // c2b8:
                    if s60 > 1 {
                        eng.wr8(sp4c, 1);
                        let dd = eng.rd8(r4 + 13) as i32 - eng.rd8(r4 + 12) as i32;
                        let rr3 = eng.rds16(r4 + 16) as i32 + r7;
                        let num = (dd + 1).wrapping_mul(rr3);
                        r0v += dd;
                        let q = idiv_full(num as u32, (r0v + 1) as u32).0;
                        r0v = q.wrapping_add(r7).wrapping_add(1) >> 1;
                        r7 = r0v as i16 as i32;
                        s48 = r7;
                        if r7 < 0 {
                            r7 = eng.rds16(r4 + 2) as i32;
                        }
                    }
                } else if r5 < 0 {
                    // emit
                } else if r5 <= 1 || s56 == 0 {
                    // c31c:
                    eng.wr8(sp4c, 0);
                    let t = eng.rd16(r4.wrapping_sub(12)) as i32 + r7;
                    r0v = t - s48;
                    r7 = r0v as i16 as i32;
                    s56 = r7;
                } else {
                    r7 = (s44 - s48 - s56) as i16 as i32;
                    eng.wr8(sp4c, 255);
                }
            }
        }
        // emit:
        let emit_r2v: i32 = 'emit: {
            let dd = eng.rd8(r4 + 13) as i32 - eng.rd8(r4 + 12) as i32;
            let mut s68 = r7;
            let mut w124 = s124;
            let mut w128 = s128;
            ip = dd + 1;
            if ip < 1 {
                break 'emit 0; // c4cc -> c20
            }
            lr = 1;
            // flag bytes from voicing table:
            let f0 = t7439[eng.rd8(r4) as usize] as i32;
            eng.wr8(w5c, ((lr as u32) & !((f0 as i8 as i32 as u32) >> 4)) as u8);
            eng.wr8(w60a, (f0 & 4) as u8);
            eng.wr8(w64, (f0 & 8) as u8);
            eng.wr8(w68, (f0 & 2) as u8);
            eng.wr8(w6c, (f0 & 128) as u8);
            let acc16 = eng.rds16(w54 + 244) as i32;
            lr = s68 + acc16;
            let r3v: i32;
            if 0x9ff <= lr {
                r3v = 0x9ff - acc16;
                s68 = r3v as i16 as i32;
            } else {
                r3v = s68;
            }
            if r3v <= 0 {
                break 'emit 0; // -> c20
            }
            eng.wr16(w54 + 244, (acc16 + r3v) as i16 as u16);
            let mut e1 = s68; // r1v
            if e1 == ip {
                fill_stride(eng, e1, w124, w128);
                break 'emit s68; // c1c
            }
            // c538:
            let r9b = w70;
            let c231 = eng.rds8(r9b + 231) as i32;
            if c231 == 0
                && eng.rd8(r9b + 228) == 0
                && eng.rd8(r9b + 229) == 0
            {
                // c824: scan frames for a zero +2 byte:
                let (e2, e7);
                let mut e3 = 0;
                let found = loop {
                    let at = aadd(w124, (e3 << 1) + (e3 << 3));
                    if eng.rd8(at + 2) == 0 {
                        break Some(e3);
                    }
                    e3 += 1;
                    e3 = e3 as i16 as i32;
                    if ip <= e3 {
                        break None;
                    }
                };
                match found {
                    None => {
                        e2 = -1;
                        e7 = 0;
                    }
                    Some(f3) => {
                        // c858:
                        let mut e3 = f3;
                        let e2v = f3;
                        let mut e7v;
                        let mut el = -e2v;
                        e7v = ip + (el << 1);
                        if e7v > 0 {
                            el = (((ip as u32) ^ ((el as u32) << 1)) & 1) as i32;
                        } else {
                            el = 0;
                        }
                        el += e2v;
                        let direct = if el > e3 {
                            // c8a0:
                            loop {
                                let at = aadd(w124, (e3 << 1) + (e3 << 3));
                                e7v = eng.rds8(at + 2) as i32;
                                if e7v > 0 {
                                    break true;
                                }
                                e3 += 1;
                                e3 = e3 as i16 as i32;
                                if el <= e3 {
                                    break false;
                                }
                            }
                        } else {
                            false
                        };
                        if !direct {
                            // c8b0:
                            e3 = el as i16 as i32;
                            let e8v = ip - e2v;
                            if e8v > e3 {
                                loop {
                                    let at = aadd(w124, (e3 << 1) + (e3 << 3));
                                    e7v = eng.rds8(at + 2) as i32;
                                    if e7v > 0 {
                                        break;
                                    }
                                    if eng.rds8(at + 12) as i32 > 0 {
                                        e3 += 1;
                                        e3 = e3 as i16 as i32;
                                        break;
                                    }
                                    e3 += 2;
                                    e3 = e3 as i16 as i32;
                                    if e3 >= e8v {
                                        break;
                                    }
                                }
                            }
                        }
                        // c8fc:
                        e2 = e2v;
                        e7 = (e3 - e2v) as i16 as i32;
                    }
                }
                // c904:
                let e8 = (e1 - ip) as i16 as i32;
                if e8 < 0 {
                    if e7 != 0 && e2 > 0 {
                        fill_stride(eng, e2, w124, w128);
                        let t3 = e2 + (e2 << 2);
                        w128 = aadd(w128, e2 << 2);
                        w124 = aadd(w124, t3 << 1);
                    }
                    // c978:
                    let n3 = -e8;
                    if n3 > 0 {
                        let t2 = n3 + (n3 << 2);
                        w124 = aadd(w124, t2 << 1);
                    }
                    // cbc8:
                    if e1 > 0 {
                        fill_stride(eng, e1, w124, w128);
                    }
                    break 'emit s68;
                }
                if e8 == 0 {
                    // cbc8:
                    if e1 > 0 {
                        fill_stride(eng, e1, w124, w128);
                    }
                    break 'emit s68;
                }
                // c994 (e8 > 0):
                if eng.rd8(r9b + 230) == 0 && eng.rd8(r9b + 232) != 0 {
                    if e7 != 0 && e2 > 0 {
                        fill_stride(eng, e2, w124, w128);
                        let t3 = e2 + (e2 << 2);
                        w128 = aadd(w128, e2 << 2);
                        w124 = aadd(w124, t3 << 1);
                    }
                    // ca20:
                    w128 = fill_const(eng, e8, k944, w128);
                    // ca58:
                    e1 = (e1 - e8) as i16 as i32;
                    if e7 >= 6 {
                        let mut r2a = w124;
                        let mut ipa = 6;
                        let r7a = -240;
                        let mut r3a = 0;
                        loop {
                            let mut x8 = 6 - r3a;
                            let lr = eng.rds16(r2a) as i32;
                            x8 = x8 + (x8 << 2);
                            x8 <<= 3;
                            x8 = x8 as i16 as i32;
                            if lr > x8 {
                                let mut y8 = r3a + (r3a << 2);
                                y8 = r7a + (y8 << 3);
                                eng.wr16(r2a, (lr + y8) as i16 as u16);
                            } else {
                                eng.wr16(r2a, 0);
                            }
                            ipa -= 1;
                            r2a += 10;
                            ipa = ipa as i16 as i32;
                            if ipa == 0 {
                                break;
                            }
                            r3a += 1;
                        }
                    }
                    // cbc8:
                    if e1 > 0 {
                        fill_stride(eng, e1, w124, w128);
                    }
                    break 'emit s68;
                }
                // cac8:
                let mut c_r2 = 0;
                if e7 > 0 {
                    let mut c_r3 = e7;
                    loop {
                        let at = aadd(w124, (c_r2 << 1) + (c_r2 << 3));
                        c_r3 -= 1;
                        c_r3 = c_r3 as i16 as i32;
                        eng.wr32(aadd(w128, c_r2 << 2), at as u32);
                        if c_r3 == 0 {
                            break;
                        }
                        c_r2 += 1;
                    }
                    c_r2 = e7;
                    let t3 = e7 + (e7 << 2);
                    w128 = aadd(w128, c_r2 << 2);
                    w124 = aadd(w124, t3 << 1);
                }
                if e7 != 0 {
                    // cb10 (e7 != 0; e7<0 skips the fill above):
                    e1 = (e1 - c_r2) as i16 as i32;
                    ip = (ip - c_r2) as i16 as i32;
                }
                // cb20:
                let c_r2 = (e1 - ip) as i16 as i32;
                if c_r2 > ip {
                    e1 = c_r2 - ip;
                    if e1 > 0 {
                        w128 = fill_const(eng, e1, w124 as u32, w128);
                    }
                    e1 = (ip << 1) as i16 as i32;
                }
                // cb64:
                let mut tlr = (e1 - ip) as i16 as i32;
                if tlr > 0 {
                    let mut t8 = 0;
                    let mut t7 = 0;
                    let s16t = tlr;
                    loop {
                        let tt3 = t8 + (t8 << 2);
                        let tt2 = tlr - 1;
                        let r9t = aadd(w124, tt3 << 1);
                        let fpv = t7 + 2;
                        let slv = aadd(w128, t7 << 2);
                        eng.wr32(aadd(w128, t7 << 2), r9t as u32);
                        tlr = tt2 as i16 as i32;
                        t7 = fpv as i16 as i32;
                        t8 += 1;
                        eng.wr32(slv + 4, r9t as u32);
                        if tlr == 0 {
                            break;
                        }
                    }
                    let tt2 = s16t + (s16t << 2);
                    w128 = aadd(w128, s16t << 3);
                    w124 = aadd(w124, tt2 << 1);
                }
                // cbc0:
                e1 = ((ip << 1) - e1) as i16 as i32;
                // cbc8:
                if e1 > 0 {
                    fill_stride(eng, e1, w124, w128);
                }
                break 'emit s68;
            }
            // c55c:
            let e2 = (e1 - ip) as i16 as i32;
            let elr = eng.rds8(r9b + 233) as i32;
            if e2 < 0 {
                if elr == 1 {
                    // c584:
                    if c231 != 0 && eng.rds8(sp50 + 202) as i32 == -1 {
                        let n3 = -e2;
                        if n3 <= e1 {
                            let mut c3 = e1;
                            let mut c1 = 0;
                            let mut c2 = e2;
                            loop {
                                eng.wr32(aadd(w128, c1 << 2), w124 as u32);
                                if c2 < 0 {
                                    c2 += 1;
                                    w124 += 10;
                                    c2 = c2 as i16 as i32;
                                    if c2 < 0 {
                                        c2 += 1;
                                        w124 += 10;
                                        c2 = c2 as i16 as i32;
                                    }
                                }
                                c3 -= 1;
                                c1 += 1;
                                if c3 == 0 {
                                    break;
                                }
                            }
                            break 'emit s68;
                        }
                    }
                } else if elr == -1 {
                    // c5f0:
                    if c231 != 0 && eng.rds8(sp50 + 202) as i32 == -1 {
                        let n3 = -e2;
                        if n3 <= e1 {
                            let t3 = ip + (ip << 2);
                            w128 = aadd(w128, e1 << 2);
                            let mut r0v = aadd(w124, t3 << 1);
                            r0v = aadd(r0v, -10);
                            let ipa = aadd(w128, -4);
                            let c3 = e1;
                            let mut c1 = 0;
                            let mut c2 = e2;
                            let mut left = c3;
                            loop {
                                eng.wr32(aadd(ipa, c1 << 2), r0v as u32);
                                r0v = aadd(r0v, -10);
                                if c2 < 0 {
                                    c2 += 1;
                                    r0v = aadd(r0v, -10);
                                    c2 = c2 as i16 as i32;
                                    if c2 < 0 {
                                        c2 += 1;
                                        r0v = aadd(r0v, -10);
                                        c2 = c2 as i16 as i32;
                                    }
                                }
                                left -= 1;
                                c1 -= 1;
                                if left == 0 {
                                    break;
                                }
                            }
                            break 'emit s68;
                        }
                    }
                }
                // c670:
                let mut g2 = 0;
                let mut g3 = 1;
                loop {
                    loop {
                        let t7 = (g2 as i16 as i32).wrapping_mul(ip as i16 as i32);
                        let tl = (g3 as i16 as i32).wrapping_mul(e1 as i16 as i32);
                        if tl > t7 || g3 >= ip {
                            break;
                        }
                        g3 += 1;
                        w124 += 10;
                        g3 = g3 as i16 as i32;
                    }
                    g2 += 1;
                    eng.wr32(w128, w124 as u32);
                    w128 += 4;
                    g2 = g2 as i16 as i32;
                    if g2 >= e1 {
                        break;
                    }
                }
                break 'emit s68;
            }
            // c6b4 (e2 >= 0):
            if elr == 1 && c231 != 0 {
                // c6c8:
                if e2 > 0 {
                    w128 = fill_const(eng, e2, w124 as u32, w128);
                }
                // c6f8:
                if ip > 0 {
                    if (ip & 1) == 1 {
                        eng.wr32(w128, w124 as u32);
                        w128 += 4;
                        if ip > 1 {
                            // stride slots 1..ip at w128-4 base:
                            let wb = aadd(w128, -4);
                            let mut c1 = 1;
                            let mut c2 = 1;
                            loop {
                                let t = c2 + (c2 << 2);
                                c1 += 2;
                                eng.wr32(aadd(wb, c2 << 2), aadd(w124, t << 1) as u32);
                                c2 += 1;
                                c1 = c1 as i16 as i32;
                                let t = c2 + (c2 << 2);
                                eng.wr32(aadd(wb, c2 << 2), aadd(w124, t << 1) as u32);
                                c2 += 1;
                                if c1 >= ip {
                                    break;
                                }
                            }
                        }
                    } else {
                        fill_stride(eng, ip, w124, w128);
                    }
                }
                break 'emit s68;
            }
            if elr == -1 && c231 != 0 {
                // c750:
                if (ip & 1) == 1 {
                    eng.wr32(w128, w124 as u32);
                    w128 += 4;
                    if ip > 1 {
                        let wb = aadd(w128, -4);
                        let mut c1 = 1;
                        let mut c3 = 1;
                        loop {
                            let t = c3 + (c3 << 2);
                            c1 += 2;
                            eng.wr32(aadd(wb, c3 << 2), aadd(w124, t << 1) as u32);
                            c3 += 1;
                            c1 = c1 as i16 as i32;
                            let t = c3 + (c3 << 2);
                            eng.wr32(aadd(wb, c3 << 2), aadd(w124, t << 1) as u32);
                            c3 += 1;
                            if c1 >= ip {
                                break;
                            }
                        }
                    }
                    // c7a4:
                    let t1 = ip + (ip << 2);
                    let mut last = aadd(w124, t1 << 1);
                    w128 = aadd(w128, (ip - 1) << 2);
                    last = aadd(last, -10);
                    if (last as u32) > 0 {
                        w128 = fill_const(eng, e2, last as u32, w128);
                    }
                    break 'emit s68;
                }
                // ip even: stride loop then c7a4:
                {
                    let mut c1 = 0;
                    let mut c3 = 0;
                    if c1 < ip {
                        loop {
                            let t = c3 + (c3 << 2);
                            c1 += 2;
                            eng.wr32(aadd(w128, c3 << 2), aadd(w124, t << 1) as u32);
                            c3 += 1;
                            c1 = c1 as i16 as i32;
                            let t = c3 + (c3 << 2);
                            eng.wr32(aadd(w128, c3 << 2), aadd(w124, t << 1) as u32);
                            c3 += 1;
                            if c1 >= ip {
                                break;
                            }
                        }
                    }
                    let _ = c3;
                }
                // c7a4 (fallthrough, w128 unadvanced):
                let t1 = ip + (ip << 2);
                let mut last = aadd(w124, t1 << 1);
                w128 = aadd(w128, ip << 2);
                last = aadd(last, -10);
                if (last as u32) > 0 {
                    w128 = fill_const(eng, e2, last as u32, w128);
                }
                break 'emit s68;
            }
            // c7e0:
            {
                let mut h_lr = 1;
                let mut h3 = e1;
                let mut h2 = 0i32;
                loop {
                    let t7 = (h_lr as i16 as i32).wrapping_mul(e1 as i16 as i32);
                    let t8 = h2.wrapping_mul(ip);
                    if t7 <= t8 {
                        h_lr += 1;
                        w124 += 10;
                        h_lr = h_lr as i16 as i32;
                    }
                    h3 -= 1;
                    eng.wr32(aadd(w128, h2 << 2), w124 as u32);
                    h3 = h3 as i16 as i32;
                    if h3 == 0 {
                        break;
                    }
                    h2 += 1;
                }
            }
            break 'emit s68;
        };
        // c20:
        eng.wr16(r4 + 2, emit_r2v as i16 as u16);
        s128 = aadd(s128, emit_r2v << 2);
        let dd2 = eng.rd8(r4 + 13) as i32 - eng.rd8(r4 + 12) as i32;
        s64v = (emit_r2v + s64v) as i16 as i32;
        let adv = dd2 + (dd2 << 2);
        s124 = aadd(s124, (adv << 1) + 10);
        eng.wr8(sp4c, 255);
        r5 = (r5 + 1) as i16 as i32;
        r4 += 14;
        if r5 < s60 {
            continue;
        }
        break;
    }
    s64v
}

/// fn_1a089c: phrase-level mora driver. Walks the 14-byte mora list at
/// 0x2930, renders each run via `mora_align` + pitch ramps + `mora_render`.
/// Row constraints (as on ARM): note index `u16(list+10)` + run length
/// stays inside `t743948`; `mora_align`'s return `r6v` stays >= 0 with
/// `r6v*2 + 0x41F4` and `r6v*4 + 0x55F8` in-engine (negative/overlong
/// `r6v` reads engine-external heap on ARM, out of model here).
pub fn phrase_driver(eng: &mut Engine, tab: &Tables, pcm: &mut PcmLog) {
    let t7439 = tab.t7439;
    let t948 = tab.t743948;
    let mut pu = 0x2930usize;
    let mut prev = 0x2930usize;
    let mut i = 0i32;
    let lim = (eng.rd16(0x3E30) as i32 - 1) as i16 as i32;
    let mut r6v = 0i32;
    let mut did = 0i32;
    eng.wr32(0x55F4, tab.k743944);
    eng.wr8(0xE6CA, 0);
    if lim < 0 {
        return;
    }
    loop {
        if eng.rd8(pu) == 0 {
            eng.wr8(0xE760, 0);
            r6v = 0;
            let s2 = eng.rds16(pu + 2) as i32;
            if s2 > 0 {
                let target = s2;
                loop {
                    ring_advance(eng, eng.rds16(4) as i32, tab);
                    if flag_done(eng) == 1 {
                        return;
                    }
                    r6v = (r6v + 1) as i16 as i32;
                    if target <= r6v {
                        break;
                    }
                }
            }
            if flag_done(eng) == 1 {
                return;
            }
            eng.wr16(0xE600 + 200, eng.rd16(pu + 2));
        }
        pu += 14;
        i = (i + 1) as i16 as i32;
        if i > lim {
            return;
        }
        // merge gate: render on discontinuity; skip (merge) on ==20,
        // small span, or unvoiced; fall through to render otherwise.
        let mut render = true;
        if eng.rd16(pu + 10) == eng.rd16(pu - 4)
            && eng.rd8(pu - 1) as i32 + 1 == eng.rd8(pu + 12) as i32
        {
            if eng.rd8(pu - 14) == 20
                || (pu as i64 - prev as i64) <= 27
                || (t7439[eng.rd8(pu - 14) as usize] & 0x80) == 0
            {
                render = false;
            }
        }
        if !render {
            continue;
        }
        // render:
        let pm14 = pu - 14;
        let cc = eng.rd8(pm14 + 13) as i32 - eng.rd8(prev + 12) as i32 + 1;
        let cnt = cc;
        if cc > 0 {
            let mut k = 0i32;
            let mut r3 = prev;
            r6v = eng.rd8(prev) as i32;
            let mut rb = eng.rd8(prev + 13) as i32 - eng.rd8(prev + 12) as i32 + 1;
            let n10 = eng.rd16(prev + 10) as i32;
            loop {
                let e = (k * 5) * 2;
                let t = t948[(((n10 + k) * 3) * 2) as usize] as i32;
                eng.wr16(e as usize + 0x3E32, (((t & 0x7F) * 5) * 2) as u16);
                eng.wr8(e as usize + 0x3E34, (((t as u32) << 24) >> 31) as u8);
                if k >= rb {
                    let t8 = eng.rd8(prev + 12) as i32;
                    rb = eng.rd8(r3 + 27) as i32 - t8 + 1;
                    r6v = eng.rd8(r3 + 14) as i32;
                    r3 += 14;
                }
                eng.wr8(e as usize + 0x3E36, eng.rd8(r3));
                if eng.rds16(r3 + 4) < 0 {
                    eng.wr8(e as usize + 0x3E34, 0);
                }
                if eng.rd8(e as usize + 0x3E34) == 0 {
                    eng.wr8(e as usize + 0x3E35, eng.rd8(4));
                } else {
                    eng.wr8(e as usize + 0x3E35, 120);
                }
                let fp = n10 + k;
                eng.wr8(e as usize + 0x3E37, t948[((fp * 3) * 2 + 1) as usize]);
                eng.wr8(e as usize + 0x3E38, t948[(((n10 + k) * 3) * 2 + 2) as usize]);
                eng.wr8(e as usize + 0x3E39, t948[(((n10 + k) * 3) * 2 + 3) as usize]);
                eng.wr8(e as usize + 0x3E3A, t948[(((n10 + k) * 3) * 2 + 4) as usize]);
                eng.wr8(e as usize + 0x3E3B, t948[(((n10 + k) * 3) * 2 + 5) as usize]);
                k = (k + 1) as i16 as i32;
                if k >= cnt {
                    break;
                }
            }
        }
        // abs pass:
        let q0 = pm14.wrapping_sub(prev);
        let mut c9 = (q0 / 14 + 1) as i32;
        let mut e2 = prev + 4;
        if c9 > 0 {
            loop {
                let av = eng.rds16(e2) as i32;
                if av < 0 {
                    eng.wr16(e2, (-av) as i16 as u16);
                }
                e2 += 14;
                c9 -= 1;
                if c9 == 0 {
                    break;
                }
            }
        }
        if prev == 0x2930 {
            let d =
                eng.rds16(pm14 + 6) as i32 - eng.rd16(aadd(0x3E28, cnt * 10)) as i32;
            interp_clamp(&mut eng.mem[..], 0x3E32, cnt, d, d);
            eng.wr8(0xE6CA, 0xFF);
        } else if i == lim {
            let d = eng.rds16(prev + 6) as i32 - eng.rd16(0x3E32) as i32;
            interp_clamp(&mut eng.mem[..], 0x3E32, cnt, d, d);
            eng.wr8(0xE6CA, 0xFE);
        } else {
            let b0a = eng.rd8(prev);
            let c1 = (t7439[b0a as usize] & 0x80) == 0 || b0a == 20;
            let mut c2 = false;
            if !c1 {
                let b0b = eng.rd8(pm14);
                c2 = (t7439[b0b as usize] & 0x80) == 0 || b0b == 20 || prev == pm14;
            }
            if c1 || c2 {
                if eng.rd8(0xE6CA) == 1 {
                    let d = eng.rds16(pm14 + 6) as i32
                        - eng.rd16(aadd(0x3E28, cnt * 10)) as i32;
                    interp_clamp(&mut eng.mem[..], 0x3E32, cnt, d, d);
                    eng.wr8(0xE6CA, 2);
                } else {
                    let d = eng.rds16(prev + 6) as i32 - eng.rd16(0x3E32) as i32;
                    interp_clamp(&mut eng.mem[..], 0x3E32, cnt, d, d);
                    eng.wr8(0xE6CA, 1);
                }
            } else {
                let d0 = eng.rds16(prev + 6) as i32;
                let d1 = eng.rds16(pm14 + 6) as i32;
                interp_setup(&mut eng.mem[..], 0x3E32, cnt, d0, d1);
                eng.wr8(0xE6CA, 0);
            }
        }
        r6v = mora_align(eng, tab, prev, pm14, 0x3E32, 0x55F8);
        if did > 0 && eng.rd8(0xE6CA) == 0 {
            let h = (eng.rds16(prev + 2) as i32 + 1) / 2;
            pitch_ramp(eng, 0x41F4, h as i16 as i32, eng.rds16(prev + 4) as i32);
            // ARM masks the unhalved (v+1) for the offset (0x1a0df8 bic)
            // but uses (v+1)>>1 for the count (0x1a0df4 asr).
            let hraw = eng.rds16(prev + 2) as i32 + 1;
            let hraw = hraw + ((hraw < 0) as i32);
            pitch_ramp(
                eng,
                aadd(0x41F4, hraw & !1),
                (r6v - (hraw >> 1)) as i16 as i32,
                eng.rds16(pm14 + 4) as i32,
            );
            did = 0;
        } else {
            let e4 = eng.rds16(pm14 + 4) as i32;
            let e18 = eng.rds16(pm14 + 18) as i32;
            if e4 == e18 {
                // bypass:
                bypass_pitch(eng, tab, prev, pm14, r6v);
            } else {
                let mode = eng.rds8(0xE600 + 202) as i32;
                if mode == -2 {
                    // setpm:
                    if eng.rd16(0xE700 + 54) == 0 {
                        eng.wr16(pm14 + 4, eng.rd16(0x4100 + 242));
                    } else {
                        eng.wr16(
                            pm14 + 4,
                            (eng.rd16(0x4100 + 242) as i32 - 10 * r6v) as i16 as u16,
                        );
                    }
                    pitch_ramp(eng, 0x41F4, r6v, eng.rds16(pm14 + 4) as i32);
                } else if eng.rd8(pm14) == eng.rd8(pm14 + 14) && mode == -1 {
                    let c200 = eng.rds16(0xE600 + 200) as i32;
                    if c200 == 0 {
                        if (t7439[eng.rd8(prev) as usize] & 1) != 0 {
                            eng.wr16(0x4100 + 242, e4 as u16);
                        } else {
                            let x = eng.rds16(pm14 + 2) as i32;
                            eng.wr16(0x4100 + 242, (e4 - 15 * (r6v - x / 2)) as i16 as u16);
                        }
                    } else {
                        let old = eng.rds16(0x4100 + 242) as i32;
                        let x = eng.rds16(pm14 + 2) as i32;
                        let a1 = c200 + r6v - x / 2;
                        let v = fixed_ratio(
                            (e4 - old).wrapping_mul(c200),
                            a1 as i16 as i32 as u32,
                        );
                        eng.wr16(0x4100 + 242, (v as i32 + old) as i16 as u16);
                    }
                    // tail2:
                    tail2_pitch(eng, pm14, r6v);
                    did = 1;
                } else if eng.rd8(pm14) == eng.rd8(pm14 + 14) {
                    // mode != -1, bytes equal: skip to tail2
                    tail2_pitch(eng, pm14, r6v);
                    did = 1;
                } else {
                    // bypass:
                    bypass_pitch(eng, tab, prev, pm14, r6v);
                }
            }
        }
        // join:
        mora_render(eng, 0x55F8, 0x41F4, r6v, tab, pcm);
        if flag_done(eng) == 1 {
            return;
        }
        let f2 = eng.rd16((0x41F2i32 + r6v * 2) as usize);
        prev = pu;
        eng.wr16(0x4100 + 242, f2);
        let pp = eng.rd32((0x55F4i32 + r6v * 4) as usize) as usize;
        eng.wr16(0xE600 + 200, eng.rd8(pp + 3) as u16);
    }
}

/// fn_1a089c tail2 block: interpolated pitch ramp pair.
fn tail2_pitch(eng: &mut Engine, pm14: usize, r6v: i32) {
    let e2 = eng.rds16(pm14 + 2) as i32;
    let e4b = eng.rds16(pm14 + 4) as i32;
    let dd = (eng.rds16(pm14 + 18) as i32 - e4b) as i16;
    let mm = e2 as i16;
    let v = fixed_ratio(
        (dd as i32 * mm as i32) as i16 as i32,
        (e2 + eng.rds16(pm14 + 16) as i32) as i16 as i32 as u32,
    );
    let sat = (v as i32 + e4b).clamp(-32768, 32767);
    let d2 = (e2 + ((e2 as u32 >> 31) as i32)) >> 1;
    pitch_ramp(eng, 0x41F4, (r6v - d2) as i16 as i32, e4b);
    let q = e2 + ((e2 as u32 >> 31) as i32);
    pitch_ramp(
        eng,
        aadd(0x41F4, r6v * 2 - (q & !1)),
        (q >> 1) as i16 as i32,
        sat,
    );
}

/// fn_1a089c bypass block: mode-dependent pitch base + ramp.
fn bypass_pitch(eng: &mut Engine, tab: &Tables, prev: usize, pm14: usize, r6v: i32) {
    let t7439 = tab.t7439;
    let mode = eng.rds8(0xE600 + 202) as i32;
    if mode == -1 {
        let c200 = eng.rds16(0xE600 + 200) as i32;
        if c200 != 0 && eng.rd16(0xE700 + 92) == 0 {
            let old = eng.rds16(0x4100 + 242) as i32;
            let a0 = c200 + r6v;
            let e4 = eng.rds16(pm14 + 4) as i32;
            // ARM muls full 32-bit here (0x1a1084 mul, no narrowing);
            // only the divisor is sxth'd (0x1a1080).
            let v = fixed_ratio((e4 - old).wrapping_mul(c200), a0 as i16 as i32 as u32);
            eng.wr16(0x4100 + 242, (v as i32 + old) as i16 as u16);
        } else if (t7439[eng.rd8(prev) as usize] & 1) != 0 {
            eng.wr16(0x4100 + 242, eng.rd16(pm14 + 4));
        } else if eng.rd16(0xE700 + 54) == 0 {
            eng.wr16(0x4100 + 242, eng.rd16(pm14 + 4));
        } else {
            let e4 = eng.rds16(pm14 + 4) as i32;
            eng.wr16(0x4100 + 242, (e4 - 15 * r6v) as i16 as u16);
        }
    } else if mode == -2 {
        if eng.rd16(0xE700 + 54) == 0 {
            eng.wr16(pm14 + 4, eng.rd16(0x4100 + 242));
        } else {
            eng.wr16(
                pm14 + 4,
                (eng.rd16(0x4100 + 242) as i32 - 10 * r6v) as i16 as u16,
            );
        }
    } else {
        if eng.rds16(0x4100 + 242) <= 0 {
            eng.wr16(0x4100 + 242, eng.rd16(prev + 4));
        }
    }
    pitch_ramp(eng, 0x41F4, r6v, eng.rds16(pm14 + 4) as i32);
}

/// fn_1a2ebc: segment prosody/duration assignment. Walks the Seg list
/// at `p0` (16-byte structs, engine offsets, null is 0), scores each
/// mora's kana chain against the ROM phone tables, and writes f6/f8/f10.
/// Row constraints (as on ARM/native): `mora->count` in 0..=96; every
/// kana name matches a `D_74362e` entry (unmatched names leave C stack
/// garbage in v8/v536/v664, nondeterministic and out of model); node
/// links never null-deref mid-walk.
pub fn seg_prosody(eng: &mut Engine, tab: &Tables, p0: usize) {
    use crate::jp::libc::strcmp0;
    if p0 == 0 {
        return;
    }
    let mut p = p0;
    let mut q = p0;
    let mut prev: i8 = 26;
    let mut v8 = [0i8; 132];
    let mut v536 = [0i8; 128];
    let mut v664 = [0i8; 128];
    let mut arr = [None; 98];
    loop {
        // accumulate vowel counts over p's moras, then q's while gated:
        let mut acc2 = 0i32;
        let mut mora = eng.rd32(p + 4) as usize;
        let mut cnt = 0i32;
        while cnt < eng.mem[p] as i8 as i32 {
            let nm = eng.rd32(mora) as usize;
            acc2 = (acc2 + vowel_count(&eng.mem[nm..]) as i32) as i16 as i32;
            mora = eng.rd32(mora + 28) as usize;
            cnt += 1;
        }
        while eng.rd8(q) != 0 && eng.rd16(eng.rd32(q + 8) as usize + 18) == 0 {
            q = eng.rd32(q + 12) as usize;
            cnt = 0;
            while cnt < eng.mem[q] as i8 as i32 {
                let nm = eng.rd32(mora) as usize;
                acc2 = (acc2 + vowel_count(&eng.mem[nm..]) as i32) as i16 as i32;
                mora = eng.rd32(mora + 28) as usize;
                cnt += 1;
            }
        }
        let mut r = p;
        while r <= q {
            let mut i2 = 0i32;
            let mut rmora = eng.rd32(r + 4) as usize;
            let rcount = eng.mem[r] as i8 as i32;
            if rcount > 0 {
                loop {
                    let count = eng.mem[rmora + 4] as i8 as i32;
                    let mut k = eng.rd32(rmora + 24) as usize;
                    let mut j = 0i32;
                    while j < count {
                        arr[(j + 1) as usize] = Some(k);
                        k = eng.rd32(k + 16) as usize;
                        j += 1;
                    }
                    // C: arr[count+1] is the lookahead mora (next unit's
                    // chain) or the D_7435f4 default; only its name is
                    // ever read (the i-loop runs to count+1 for nxt).
                    let lookahead = {
                        let nx = eng.rd32(rmora + 28) as usize;
                        if nx != 0 && eng.rd32(nx + 24) != 0 {
                            eng.rd32(nx + 24) as usize
                        } else {
                            0
                        }
                    };
                    // ARM orig 0x1a3060: r == p (first segment of the call),
                    // not r == q; the old port ended phrases instead of
                    // starting them on multi-segment input.
                    let f868 = r == p && i2 == 0;
                    let f872 = r == q && rcount - 1 == i2;
                    let mut state =
                        if r == p { 0 } else if eng.rd32(q + 12) != 0 { 1 } else { 2 };
                    eng.wr16(0xE73A, eng.rd16(rmora + 10));
                    let total = acc2;
                    let mut mode = 0i32;
                    let mut flag = 0i32;
                    v664[0] = prev;
                    v8[0] = prev;
                    let n2 = count + 2;
                    let mut i = 1i32;
                    while i < n2 {
                        // i == count+1 reads the lookahead mora (C arr[count+1]);
                        // koff 0 there means the D_7435f4 default name.
                        let tail = i == count + 1 && lookahead == 0;
                        let koff = if i == count + 1 && lookahead != 0 {
                            lookahead
                        } else {
                            arr[i as usize].unwrap_or(0)
                        };
                        let dash = if tail {
                            tab.t7435f4[0] == b'-'
                        } else {
                            eng.rd8(koff) == b'-'
                        };
                        if dash {
                            v536[i as usize] = 27;
                            v8[i as usize] = v8[(i - 1) as usize];
                            v664[i as usize] = v664[(i - 1) as usize];
                        } else if tail {
                            let mut j = 0usize;
                            while j < 144 {
                                if strcmp0(&tab.t7435f4, &tab.t74362e[j * 4..]) == 0 {
                                    v536[i as usize] = tab.t74269c[j] as i8;
                                    v8[i as usize] = tab.t74272c[j] as i8;
                                    v664[i as usize] = tab.t7427bc[j] as i8;
                                    break;
                                }
                                j += 1;
                            }
                        } else {
                            let mut j = 0usize;
                            while j < 144 {
                                if strcmp0(&eng.mem[koff..], &tab.t74362e[j * 4..]) == 0 {
                                    v536[i as usize] = tab.t74269c[j] as i8;
                                    v8[i as usize] = tab.t74272c[j] as i8;
                                    v664[i as usize] = tab.t7427bc[j] as i8;
                                    break;
                                }
                                j += 1;
                            }
                        }
                        i += 1;
                    }
                    prev = v8[count as usize];
                    let idx = if total <= 3 {
                        0
                    } else if total - 4 < 5 {
                        total - 3
                    } else if total == 9 || total == 10 {
                        6
                    } else if total - 11 < 3 {
                        7
                    } else if total - 14 < 3 {
                        8
                    } else if total - 17 < 9 {
                        9
                    } else if total > 25 {
                        10
                    } else {
                        -1
                    };
                    let mut i = 1i32;
                    while i <= count {
                        let koff = arr[i as usize].unwrap_or(0);
                        let last = if f872 && i == count {
                            if state != 2 {
                                state = 1;
                            }
                            2
                        } else {
                            let l = if f868 && i == 1 { 0 } else { 1 };
                            state = if (state | l) != 0 { 1 } else { 0 };
                            l
                        };
                        let v880 = v536[i as usize] as i32;
                        if v880 >= 26 {
                            eng.wr16(koff + 6, 0);
                        } else {
                            let mut ph = v664[i as usize] as i32;
                            match ph {
                                6 | 7 | 8 => {
                                    mode = 0;
                                    flag = 0;
                                }
                                9 | 10 | 11 => {
                                    mode = 1;
                                    flag = 0;
                                }
                                12 | 13 => {
                                    mode = 2;
                                    flag = 0;
                                }
                                14 | 15 | 16 | 24 => {
                                    mode = 3;
                                    flag = 0;
                                }
                                17 | 18 => {
                                    mode = 4;
                                    flag = 0;
                                }
                                19 => {
                                    mode = 5;
                                    flag = 0;
                                }
                                20 | 21 => {
                                    mode = 6;
                                    flag = 0;
                                }
                                23 | 27 => {
                                    mode = 7;
                                    flag = 0;
                                }
                                26 | 28 => {
                                    ph -= 20;
                                    mode = 0;
                                    flag = 1;
                                }
                                29 | 31 => {
                                    ph -= 20;
                                    mode = 1;
                                    flag = 1;
                                }
                                32 | 33 => {
                                    ph -= 20;
                                    mode = 2;
                                    flag = 1;
                                }
                                25 => {
                                    ph = 15;
                                    mode = 3;
                                    flag = 1;
                                }
                                34 => {
                                    ph = 19;
                                    mode = 5;
                                    flag = 1;
                                }
                                40 => {
                                    ph = 10;
                                    mode = 1;
                                    flag = 1;
                                }
                                _ => {}
                            }
                            let t0 = mode * 154;
                            let tprev = if last == 0 { 26 } else { v8[(i - 1) as usize] as i32 };
                            let first = if eng.rd8(koff + 4) == 81 { 1 } else { 0 };
                            let v8i = v8[i as usize] as i32;
                            let mut sum = (u16_at(tab.t74284c, (t0 * 2) as usize) as i32
                                + u16_at(tab.t74284c, ((t0 + ph + 1) * 2) as usize) as i32
                                + u16_at(tab.t74284c, ((t0 + tprev + 29) * 2) as usize) as i32
                                + u16_at(tab.t74284c, ((t0 + v8i + 56) * 2) as usize) as i32
                                + u16_at(tab.t74284c, ((t0 + first + 83) * 2) as usize) as i32
                                + u16_at(tab.t74284c, ((t0 + state + 85) * 2) as usize) as i32
                                + u16_at(tab.t74284c, ((t0 + last + 89) * 2) as usize) as i32)
                                as i16 as i32;
                            if flag != 0 {
                                sum = (match ph {
                                    9 => sum + 0xE40,
                                    10 => sum + 0xA00,
                                    11 => sum + 0x8C6,
                                    15 => sum + 0x6E0,
                                    8 => sum + 0xA0D,
                                    12 => sum + 0x880,
                                    13 => sum + 0xBB3,
                                    6 => sum + 0x673,
                                    19 => sum + 0xA40,
                                    _ => sum,
                                }) as i16 as i32;
                            }
                            let mut f6 = (sum >> 6) as i16 as i32;
                            if f6 < 25 {
                                f6 = 25;
                            }
                            eng.wr16(koff + 6, f6 as u16);
                        }
                        let mut cur2 = v880;
                        if cur2 == 27 {
                            cur2 = if last != 0 { v8[(i - 1) as usize] as i32 } else { 26 };
                        }
                        let mut nxt = v536[(i + 1) as usize] as i32;
                        if nxt == 27 {
                            nxt = v8[(i + 1) as usize] as i32;
                        }
                        if last == 2 {
                            nxt = 12;
                        }
                        let v8i = v8[i as usize] as i32;
                        if v8i == 5 {
                            let f8 = (s16_at(tab.t743380, 0, cur2 + 1) as i32
                                + s16_at(tab.t743380, 0, state + 92) as i32
                                + s16_at(tab.t743380, 0, nxt + 28) as i32
                                + s16_at(tab.t743380, 0, last + 97) as i32
                                + 0x1E00)
                                >> 6;
                            eng.wr16(koff + 8, f8 as i16 as u16);
                            let f10 = 600
                                + ((s16_at(tab.t7434be, 0, idx + 80) as i32
                                    + s16_at(tab.t7434be, 0, nxt + 33) as i32
                                    + s16_at(tab.t7434be, 0, state + 108) as i32
                                    + s16_at(tab.t7434be, 0, cur2 + 6) as i32
                                    + s16_at(tab.t7434be, 0, last + 117) as i32)
                                    >> 6);
                            eng.wr16(koff + 10, f10 as i16 as u16);
                            eng.wr16(koff + 6, 0);
                        } else {
                            let mut f8 = (s16_at(tab.t7431ec, 0, cur2 + 6) as i32
                                + s16_at(tab.t7431ec, 0, v8i + 1) as i32
                                + s16_at(tab.t7431ec, 0, nxt + 33) as i32
                                + s16_at(tab.t7431ec, 0, state + 101) as i32
                                + s16_at(tab.t7431ec, 0, last + 105) as i32
                                + 0x12FC)
                                >> 6;
                            f8 = f8 as i16 as i32;
                            if f8 < 70 {
                                f8 = 70;
                            }
                            eng.wr16(koff + 8, f8 as u16);
                            let f10 = 600
                                + ((s16_at(tab.t7434be, 0, idx + 80) as i32
                                    + s16_at(tab.t7434be, 0, nxt + 33) as i32
                                    + s16_at(tab.t7434be, 0, v8i + 1) as i32
                                    + s16_at(tab.t7434be, 0, cur2 + 6) as i32
                                    + s16_at(tab.t7434be, 0, state + 108) as i32
                                    + s16_at(tab.t7434be, 0, last + 117) as i32)
                                    >> 6);
                            eng.wr16(koff + 10, f10 as i16 as u16);
                            if eng.rd8(koff + 4) == 76 {
                                let cur = eng.rds16(koff + 8) as i32;
                                let add = (s16_at(tab.t7431ec, 0, v8i + 1) as i32
                                    + s16_at(tab.t7431ec, 0, v8i + 33) as i32
                                    + s16_at(tab.t7431ec, 0, v8i + 6) as i32
                                    + 0x129F)
                                    >> 6;
                                eng.wr16(koff + 8, (cur + add) as i16 as u16);
                            }
                        }
                        if eng.rd8(koff + 5) == 85 {
                            let f8 = eng.rds16(koff + 8) as i32;
                            eng.wr16(koff + 8, ((f8 * 0xB334) >> 16) as i16 as u16);
                            let f10 = eng.rds16(koff + 10) as i32;
                            eng.wr16(koff + 10, (f10 - 30) as i16 as u16);
                        }
                        i += 1;
                    }
                    // scale + f22 pass:
                    let scale = eng.rd16(0xE73A) as i32;
                    let f22 = eng.rds16(rmora + 22) as i32;
                    let mut i = 1i32;
                    while i <= count {
                        let koff = arr[i as usize].unwrap_or(0);
                        match v664[i as usize] as i32 {
                            6 | 7 | 8 | 14 | 23 | 24 | 26 | 27 | 28 => {
                                if eng.rd8(koff + 4) == 81 {
                                    let f6 = eng.rds16(koff + 6) as i32;
                                    eng.wr16(koff + 6, ((scale * f6 + 50) / 100) as i16 as u16);
                                }
                            }
                            _ => {
                                let f6 = eng.rds16(koff + 6) as i32;
                                eng.wr16(koff + 6, ((scale * f6 + 50) / 100) as i16 as u16);
                            }
                        }
                        let f8 = eng.rds16(koff + 8) as i32;
                        eng.wr16(koff + 8, ((scale * f8 + 50) / 100) as i16 as u16);
                        i += 1;
                    }
                    let mut i = 1i32;
                    while i <= count {
                        let koff = arr[i as usize].unwrap_or(0);
                        let f10 = eng.rds16(koff + 10) as i32;
                        eng.wr16(koff + 10, (f10 + f22) as i16 as u16);
                        i += 1;
                    }
                    i2 += 1;
                    rmora = eng.rd32(rmora + 28) as usize;
                    if !(rcount > i2) {
                        break;
                    }
                }
            }
            r += 16;
        }
        q = eng.rd32(q + 12) as usize;
        p = q;
        if p == 0 {
            break;
        }
    }
}

/// fn_1a3a54: kana-to-unit converter. Scans Shift-JIS `text`, emits the
/// unit string to `out` (`size` bytes at `out`), returns 0 on success
/// or the `f536` text offset on overflow error (callers ignore it).
/// `nbuf` is zero-init here; the C reads uninit stack there on the `#P`
/// path, which never fires on the corpus (no `#` in tag output).
pub fn kana_map(eng: &mut Engine, tab: &Tables, text: usize, out: usize, size: i32) -> usize {
    use crate::jp::libc::{log_format, memcmp0, strcat0, strcmp0, strcpy0, strlen0, strchr0,
        strncat0, strncpy0, strtol10};
    let mut p = text;
    let mut f12 = 10i32;
    let mut buf0 = [0u8; 100];
    let mut f120 = 10i32;
    let mut f124 = 0i32;
    let mut buf1 = [0u8; 196];
    let mut buf2 = [0u8; 196];
    let mut f520 = 0i32;
    let mut nbuf = [0u8; 8];
    let mut f536 = p;
    let mut mode = 0i32;
    let mut fp = 0i32;
    let mut f540 = 0i32;
    let mut f544 = 0i32;
    let mut f548 = 0i32;
    eng.wr8(out, 0);
    macro_rules! rd8 {
        ($o:expr) => {
            eng.rd8($o)
        };
    }
    macro_rules! rds8 {
        ($o:expr) => {
            eng.rds8($o) as i32
        };
    }
    // emit helper: strcpy buf1->buf2, append D_74362c, size-check, strcat out.
    macro_rules! emit12 {
        () => {{
            strcpy0(&mut buf2, &buf1);
            let bl2 = strlen0(&buf2);
            strcat0(&mut buf2, bl2, tab.t74362c);
            let h1 = strlen0(&buf2) as i32;
            let h2 = strlen0(&eng.mem[out..]) as i32;
            if h1 + h2 >= size {
                return f536;
            }
            let olen = strlen0(&eng.mem[out..]);
            strcat0(&mut eng.mem[out..], olen, &buf2);
            fp = h1 as i16 as i32;
            if fp < 0 {
                return f536;
            }
        }};
    }
    loop {
        if rd8!(p) == 0 {
            break;
        }
        // dispatch:
        let c0 = rds8!(p);
        if c0 == b'#' as i32 {
            // hash:
            let c1 = rds8!(p + 1);
            if c1 == b'Q' as i32 {
                // hashQ:
                p += 2;
                let mut e2 = 0i32;
                let mut d = 0usize;
                fp = 0;
                if rd8!(p) != 0 {
                    loop {
                        let cc = rds8!(p);
                        if cc == b'#' as i32 && rd8!(p + 1) == b'q' {
                            p += 2;
                            break;
                        }
                        if cc == b'#' as i32 {
                            // qbyte with '#':
                            if e2 < 190 {
                                e2 = (e2 + 1) as i16 as i32;
                                p += 1;
                                buf1[d] = cc as u8;
                                d += 1;
                            } else {
                                p += 1;
                            }
                        } else if cc == b'/' as i32 || cc == 13 || cc == 10 {
                            buf1[d] = 0;
                            p += 1;
                            if buf1[0] != 0 {
                                emit12!();
                                f536 = p;
                            }
                            d = 0;
                            e2 = 0;
                        } else {
                            // qbyte:
                            if e2 < 190 {
                                e2 = (e2 + 1) as i16 as i32;
                                p += 1;
                                buf1[d] = cc as u8;
                                d += 1;
                            } else {
                                p += 1;
                            }
                        }
                        if rd8!(p) == 0 {
                            break;
                        }
                    }
                }
                if fp < 0 {
                    return f536;
                }
                // emit2:
                buf1[d] = 0;
                if buf1[0] != 0 {
                    emit12!();
                }
                f536 = p;
                continue;
            }
            if c1 == b'p' as i32 {
                // hashP:
                let mut e2 = 0i32;
                loop {
                    let h1 = (e2 + 1) as i16 as i32;
                    if h1 > 5 {
                        e2 = 0;
                        break;
                    }
                    let cc = rds8!(p + e2 as usize + 2);
                    if cc == 0 || cc == 92 {
                        break;
                    }
                    e2 = h1;
                }
                let nl = strlen0(&nbuf);
                strncat0(&mut nbuf, nl, &eng.mem[p + 2..], e2 as usize);
                let h1 = strtol10(&nbuf);
                f520 = clamp_s16(h1, 0, 30000);
                p = p + e2 as usize + 3;
                f536 = p;
                continue;
            }
            if c1 == b'q' as i32 {
                // h_b90:
                p += 2;
                f536 = p;
                continue;
            }
            // plain hash: scan to delimiter:
            let mut e2 = 0i32;
            let mut d = 0usize;
            loop {
                if memcmp0(&eng.mem[p..], b"\x81\x5e", 2) == 0
                    || memcmp0(&eng.mem[p..], b"\x81\x62", 2) == 0
                    || memcmp0(&eng.mem[p..], b"\x81\x49", 2) == 0
                    || memcmp0(&eng.mem[p..], b"\x81\x48", 2) == 0
                    || memcmp0(&eng.mem[p..], b"\x81\x44", 2) == 0
                    || memcmp0(&eng.mem[p..], b"\x81\x51", 2) == 0
                {
                    // hadv2:
                    p += 2;
                    break;
                }
                let cc = rds8!(p);
                if cc == b'/' as i32
                    || cc == 13
                    || cc == 10
                    || cc == b'|' as i32
                    || cc == b'!' as i32
                    || cc == b'?' as i32
                    || cc == b'.' as i32
                    || cc == b'_' as i32
                {
                    // hadv1:
                    p += 1;
                    break;
                }
                if e2 >= 190 {
                    p += 1;
                } else {
                    e2 = (e2 + 1) as i16 as i32;
                    p += 1;
                    buf1[d] = cc as u8;
                    d += 1;
                }
                // hashcopy:
                if rd8!(p) == 0 {
                    break;
                }
            }
            // hemit:
            buf1[d] = 0;
            if buf1[0] != 0 {
                emit12!();
            }
            f536 = p;
            continue;
        }
        if c0 == b'.' as i32 || memcmp0(&eng.mem[p..], b"\x81\x44", 2) == 0 {
            // h_594:
            if buf0[0] != 0 {
                let bl0 = strlen0(&buf0);
                strcat0(&mut buf0, bl0, b".");
                if f520 == 0 {
                    f520 = 1000;
                }
                if mode == 0 && f12 == 10 {
                    f12 = num_class(&eng.mem[..], p, tab.t743620, tab.t743614) as i16 as i32;
                }
                let h1 = clamp_s16(f520 + f124, 0, 30000);
                fp = log_format(&mut eng.mem[out..], size, &buf0, mode, f12, f120, h1);
                f520 = 0;
                if fp < 0 {
                    return f536;
                }
                buf0[0] = 0;
                f120 = 10;
                f124 = 0;
                mode = 0;
                f12 = 10;
                f548 = 0;
                f544 = 0;
                f540 = 0;
            }
            if rd8!(p) == b'.' {
                // h_11e4:
                p += 1;
                f536 = p;
                continue;
            }
            // h_b90:
            p += 2;
            f536 = p;
            continue;
        }
        if c0 == b'?' as i32
            || memcmp0(&eng.mem[p..], b"\x81\x48", 2) == 0
            || c0 == b'!' as i32
            || memcmp0(&eng.mem[p..], b"\x81\x49", 2) == 0
        {
            // h_660:
            if buf0[0] == 0 {
                if c0 == b'?' as i32 || c0 == b'!' as i32 {
                    p += 1;
                    f536 = p;
                    continue;
                }
                p += 2;
                f536 = p;
                continue;
            }
            if mode == 0 && f12 == 10 {
                f12 = num_class(&eng.mem[..], p, tab.t743620, tab.t743614) as i16 as i32;
            }
            let mut h2 = 0i32;
            loop {
                if strlen0(&buf0) + 1 >= 96 {
                    break;
                }
                let c = rds8!(p);
                if c == b'?' as i32 {
                    let bl0 = strlen0(&buf0);
                    strcat0(&mut buf0, bl0, b"?");
                    p += 1;
                } else if c == b'!' as i32 {
                    let bl0 = strlen0(&buf0);
                    strcat0(&mut buf0, bl0, b"!");
                    p += 1;
                } else if memcmp0(&eng.mem[p..], b"\x81\x48", 2) == 0 {
                    let bl0 = strlen0(&buf0);
                    strcat0(&mut buf0, bl0, b"?");
                    p += 2;
                } else if memcmp0(&eng.mem[p..], b"\x81\x49", 2) == 0 {
                    let bl0 = strlen0(&buf0);
                    strcat0(&mut buf0, bl0, b"!");
                    p += 2;
                } else {
                    break;
                }
                h2 += 1;
                if h2 >= 6 {
                    break;
                }
            }
            // emit660:
            if f520 == 0 {
                f520 = 1000;
            }
            let h1 = clamp_s16(f520 + f124, 0, 30000);
            fp = log_format(&mut eng.mem[out..], size, &buf0, mode, f12, f120, h1);
            f520 = 0;
            if fp < 0 {
                return f536;
            }
            buf0[0] = 0;
            f120 = 10;
            f124 = 0;
            mode = 0;
            f12 = 10;
            f548 = 0;
            f544 = 0;
            f540 = 0;
            f536 = p;
            continue;
        }
        if c0 == 10 || c0 == 13 {
            // h_7f4:
            if buf0[0] == 0 {
                p += 1;
                f536 = p;
                continue;
            }
            if f124 != 0 {
                if mode == 0 && f12 == 10 {
                    f12 = num_class(&eng.mem[..], p, tab.t743620, tab.t743614) as i16 as i32;
                }
                if f520 != 0 {
                    fp = log_format(&mut eng.mem[out..], size, &buf0, mode, f12, f120, f520);
                    f520 = 0;
                    f540 = 0;
                } else {
                    fp = log_format(&mut eng.mem[out..], size, &buf0, mode, f12, f120, f124);
                    f540 = 0;
                }
            } else {
                if mode == 0 && f12 == 10 {
                    f12 = num_class(&eng.mem[..], p, tab.t743620, tab.t743614) as i16 as i32;
                }
                fp = log_format(&mut eng.mem[out..], size, &buf0, mode, f12, f120, f124);
            }
            if fp < 0 {
                return f536;
            }
            f544 = 0;
            mode = 0;
            buf0[0] = 0;
            // ARM orig 0x1a4378: f12 is 10 unconditionally here; the old
            // conditional (0 when f124 == 0) starved the next line's f12.
            if f124 != 0 {
                f120 = 10;
            } else {
                f120 = 0;
            }
            f12 = 10;
            f124 = 0;
            f548 = 0;
            p += 1;
            f536 = p;
            continue;
        }
        if c0 == b'|' as i32 || memcmp0(&eng.mem[p..], b"\x81\x62", 2) == 0 {
            // h_950:
            if buf0[0] != 0 {
                if mode == 0 && f12 == 10 {
                    f12 = num_class(&eng.mem[..], p, tab.t743620, tab.t743614) as i16 as i32;
                }
                if f124 != 0 && f520 != 0 {
                    fp = log_format(&mut eng.mem[out..], size, &buf0, mode, f12, f120, f520);
                    f520 = 0;
                    f540 = 0;
                } else {
                    fp = log_format(&mut eng.mem[out..], size, &buf0, mode, f12, f120, f124);
                    if f124 != 0 {
                        f540 = 0;
                    }
                }
                f544 = 0;
                if fp < 0 {
                    return f536;
                }
                f12 = 10;
                buf0[0] = 0;
                mode = 0;
                f120 = 5;
                f124 = 0;
                f548 = 0;
            }
            if rd8!(p) == b'|' {
                p += 1;
                f536 = p;
                continue;
            }
            p += 2;
            f536 = p;
            continue;
        }
        if c0 == b'/' as i32 || memcmp0(&eng.mem[p..], b"\x81\x5e", 2) == 0 {
            // h_a3c:
            if buf0[0] != 0 {
                if f124 != 0 {
                    if mode == 0 && f12 == 10 {
                        f12 = num_class(&eng.mem[..], p, tab.t743620, tab.t743614) as i16 as i32;
                    }
                    if f520 != 0 {
                        fp = log_format(&mut eng.mem[out..], size, &buf0, mode, f12, f120, f520);
                        f520 = 0;
                        f540 = 0;
                    } else {
                        fp = log_format(&mut eng.mem[out..], size, &buf0, mode, f12, f120, f124);
                        f540 = 0;
                    }
                } else {
                    if mode == 0 && f12 == 10 {
                        f12 = num_class(&eng.mem[..], p, tab.t743620, tab.t743614) as i16 as i32;
                    }
                    fp = log_format(&mut eng.mem[out..], size, &buf0, mode, f12, f120, f124);
                }
                if fp < 0 {
                    return f536;
                }
                mode = 0;
                buf0[0] = 0;
                if f124 != 0 {
                    f120 = 10;
                } else {
                    f120 = 0;
                }
                f124 = 0;
                f12 = 10;
                f548 = 0;
                f544 = 0;
            }
            if rd8!(p) == b'/' {
                p += 1;
                f536 = p;
                continue;
            }
            p += 2;
            f536 = p;
            continue;
        }
        if c0 == b'_' as i32 || memcmp0(&eng.mem[p..], b"\x81\x51", 2) == 0 {
            // h_b98:
            if buf0[0] != 0 {
                if f124 == 0 && mode == 0 && f12 == 10 {
                    f12 = num_class(&eng.mem[..], p, tab.t743620, tab.t743614) as i16 as i32;
                }
                if rd8!(p) == b'_' {
                    p += 1;
                } else {
                    p += 2;
                }
                if rd8!(p) == b'0' {
                    f124 = (f124 + 100) as i16 as i32;
                    p += 1;
                } else if memcmp0(&eng.mem[p..], b"\x82\x4f", 2) == 0 {
                    f124 = (f124 + 100) as i16 as i32;
                    p += 2;
                } else {
                    if rd8!(p) == b'1' {
                        p += 1;
                    }
                    f124 = (f124 + 200) as i16 as i32;
                }
            } else if rd8!(p) == b'_' {
                p += 1;
            } else {
                p += 2;
            }
            // h_b98clamp:
            if f124 > 30000 {
                f124 = 30000;
            }
            continue;
        }
        if c0 == b'\'' as i32 || memcmp0(&eng.mem[p..], b"\x81\x66", 2) == 0 {
            // h_c74:
            mode = 0;
            buf2[0] = 0;
            if buf0[0] != 0 {
                let mut di = 0usize;
                loop {
                    if strchr0(b"AIUEOaiueoN-Q", buf0[di]).is_some() {
                        mode = (mode + 1) as i16 as i32;
                    }
                    di += 1;
                    if buf0[di] == 0 {
                        break;
                    }
                }
            }
            let d = if rd8!(p) == b'\'' { p + 1 } else { p + 2 };
            strncpy0(&mut buf2, &eng.mem[d..], 1);
            let mut h1 = strcmp0(&buf2, &tab.t743620[0..]);
            if h1 != 0 {
                h1 = strcmp0(&buf2, &tab.t743620[4..]);
                if h1 == 0 {
                    h1 = 1;
                } else {
                    h1 = strcmp0(&buf2, &tab.t743620[8..]);
                    h1 = if h1 == 0 { 2 } else { -1 };
                }
            }
            let mut h2v = 1i32;
            h1 = h1 as i16 as i32;
            if h1 == -1 {
                strncpy0(&mut buf2, &eng.mem[d..], 2);
                h1 = strcmp0(&buf2, &tab.t743614[0..]);
                if h1 != 0 {
                    h1 = strcmp0(&buf2, &tab.t743614[4..]);
                    if h1 == 0 {
                        h1 = 1;
                    } else {
                        h1 = strcmp0(&buf2, &tab.t743614[8..]);
                        h1 = if h1 == 0 { 2 } else { -1 };
                    }
                }
                h2v = 2;
                h1 = h1 as i16 as i32;
                if h1 == -1 {
                    f12 = 10;
                    p = d;
                    continue;
                }
            }
            f12 = match h1 {
                0 => 5,
                1 => 10,
                2 => 15,
                _ => 10,
            };
            p = d + h2v as usize;
            continue;
        }
        if c0 == b'%' as i32 || memcmp0(&eng.mem[p..], b"\x81\x93", 2) == 0 {
            // h_dd0:
            let h1 = strlen0(&buf0) as i16 as i32;
            if h1 >= 2 {
                let dl = h1 as usize;
                if strchr0(b"aiueo", buf0[dl - 1]).is_some()
                    && strchr0(b"AIUEOaiueoN-Q", buf0[dl - 2]).is_none()
                {
                    buf0[dl - 1] = buf0[dl - 1].wrapping_sub(32);
                }
            }
            if rd8!(p) == b'%' {
                p += 1;
            } else {
                p += 2;
            }
            continue;
        }
        // kana gate (kh1 = 0 with the ROM count, so the kana scan is
        // dead and nokana always runs; kept general like the C):
        let kh2 = tab.k7873a4 as i32;
        let kh1 = if kh2 > 0 { kh2 & 1 } else { 0 };
        let mut e2 = kh1;
        let mut found = false;
        if kh1 > 0 {
            // kana: scan 0..kh1, fall through to nokana on miss:
            let mut e = 0i32;
            while e < kh1 {
                let kl = strlen0(&tab.t7868c4[(e * 12) as usize..]) as i32;
                if memcmp0(&eng.mem[p..], &tab.t7868c4[(e * 12) as usize..], kl as usize) == 0 {
                    e2 = e;
                    found = true;
                    break;
                }
                e = (e + 1) as i16 as i32;
            }
        }
        if !found {
            // nokana: stepped scan from kh1:
            e2 = kh1;
            if kh1 < kh2 {
                let mut e = e2;
                loop {
                    if e >= kh2 {
                        break;
                    }
                    let kl = strlen0(&tab.t7868c4[(e * 12) as usize..]) as i32;
                    if memcmp0(&eng.mem[p..], &tab.t7868c4[(e * 12) as usize..], kl as usize) == 0
                    {
                        e2 = e;
                        break;
                    }
                    let kl1 = strlen0(&tab.t7868c4[((e + 1) * 12) as usize..]) as i32;
                    if memcmp0(
                        &eng.mem[p..],
                        &tab.t7868c4[((e + 1) * 12) as usize..],
                        kl1 as usize,
                    ) == 0
                    {
                        e2 = (e + 1) as i16 as i32;
                        break;
                    }
                    e = (e + 2) as i16 as i32;
                    e2 = e;
                }
            }
        }
        // found:
        if e2 >= kh2 || e2 < 0 {
            // ascii:
            let mut c = rds8!(p);
            p += 1;
            if ((c + 127) as u32) > 30 {
                c += 32;
                if (c as u32) > 28 {
                    f536 = p;
                    continue;
                }
            }
            let mut h1 = rds8!(p);
            if ((h1 - 64) as u32) > 62 {
                h1 += 128;
                if (h1 as u32) > 124 {
                    f536 = p;
                    continue;
                }
            }
            p += 1;
            f536 = p;
            continue;
        }
        if f124 != 0 && buf0[0] != 0 {
            if f520 == 0 {
                if mode == 0 && f12 == 10 {
                    f12 = num_class(&eng.mem[..], p, tab.t743620, tab.t743614) as i16 as i32;
                }
                fp = log_format(&mut eng.mem[out..], size, &buf0, mode, f12, f120, f124);
            } else {
                if mode == 0 && f12 == 10 {
                    f12 = num_class(&eng.mem[..], p, tab.t743620, tab.t743614) as i16 as i32;
                }
                fp = log_format(&mut eng.mem[out..], size, &buf0, mode, f12, f120, f520);
                f520 = 0;
            }
            f544 = 0;
            f540 = 0;
            if fp < 0 {
                return f536;
            }
            buf0[0] = 0;
            f120 = 10;
            f124 = 0;
            f12 = 10;
            f548 = 0;
            f536 = p;
        } else if f124 != 0 {
            f536 = p;
        }
        // matchcont:
        if f548 == 0 {
            let h2len = strlen0(&buf0) as i32;
            let base = (e2 * 12) as usize;
            let h1 = strlen0(&tab.t7868c4[base + 8..]) as i32;
            if f544 + h1 >= 64 || f540 + h1 >= 256 {
                // emit100:
                fp = log_format(&mut eng.mem[out..], size, &buf0, mode, f12, f120, 100);
                buf0[0] = 0;
                if fp >= 0 {
                    mode = 0;
                }
                f536 = p;
                f544 = 0;
                f540 = 0;
                if fp < 0 {
                    return f536;
                }
                // checklen with h2 = 0:
                let h1 = strlen0(&tab.t7868c4[base + 8..]) as i32;
                if h1 < 96 {
                    let n0 = tab.t7868c4[base + 8];
                    if n0 != b'-' && n0 != b'Q' {
                        let blen = strlen0(&buf0);
                        strcat0(&mut buf0, blen, &tab.t7868c4[base + 8..]);
                    }
                } else {
                    f548 = 1;
                }
            } else {
                // checklen:
                if h2len + h1 >= 96 {
                    f548 = 1;
                } else if h2len != 0 {
                    if !(buf0[(h2len - 1) as usize] == b'Q' && tab.t7868c4[base + 8] == b'Q') {
                        let blen = strlen0(&buf0);
                        strcat0(&mut buf0, blen, &tab.t7868c4[base + 8..]);
                    }
                } else if tab.t7868c4[base + 8] != b'-' && tab.t7868c4[base + 8] != b'Q' {
                    let blen = strlen0(&buf0);
                    strcat0(&mut buf0, blen, &tab.t7868c4[base + 8..]);
                }
            }
        }
        // advance:
        let base = (e2 * 12) as usize;
        let h1 = strlen0(&tab.t7868c4[base..]) as i32;
        p += h1 as usize;
        f540 = (f540 + strlen0(&tab.t7868c4[base..]) as i32) as i16 as i32;
        f544 = (f544 + strlen0(&tab.t7868c4[base..]) as i32) as i16 as i32;
        continue;
    }
    // final:
    if buf0[0] == 0 {
        return 0;
    }
    if f124 == 0 {
        f124 = 1000;
    }
    if f520 != 0 {
        if mode == 0 && f12 == 10 {
            f12 = num_class(&eng.mem[..], p, tab.t743620, tab.t743614) as i16 as i32;
        }
        fp = log_format(&mut eng.mem[out..], size, &buf0, mode, f12, f120, f520);
    } else {
        if mode == 0 && f12 == 10 {
            f12 = num_class(&eng.mem[..], p, tab.t743620, tab.t743614) as i16 as i32;
        }
        fp = log_format(&mut eng.mem[out..], size, &buf0, mode, f12, f120, f124);
    }
    if fp < 0 {
        return f536;
    }
    0
}

/// fn_19e1cc (unreached on corpus; only called when `Z == -1`, which
/// faults on ARM): vowel count minus 2, floored at 0.
fn e1cc_count(s: &[u8]) -> i32 {
    let v = (vowel_count(s) as i32 - 2) as i16 as i32;
    if v < 0 {
        0
    } else {
        v
    }
}

/// fn_19ecf8 (unreached on corpus; `Z == -1` path): binary search over
/// the word table at `ctx` (table u32 @8, count s16 @12; 8-byte entries:
/// name u32 @0, val s8 @4) with I/U case folding. On match copies the
/// entry name to `buf` and returns val, else -1.
fn lookup_word(buf: &mut [u8], mem: &[u8], ctx: usize) -> i32 {
    use crate::jp::libc::strcpy0;
    let mut lo = 0i32;
    let mut hi = i16::from_le_bytes([mem[ctx + 12], mem[ctx + 13]]) as i32 - 1;
    if hi < 0 {
        return -1;
    }
    let table = u32::from_le_bytes([mem[ctx + 8], mem[ctx + 9], mem[ctx + 10], mem[ctx + 11]])
        as usize;
    loop {
        let mid = (lo + hi) >> 1;
        let eo = table + (mid as usize) * 8;
        let mut e = u32::from_le_bytes([mem[eo], mem[eo + 1], mem[eo + 2], mem[eo + 3]]) as usize;
        let mut si = 0usize;
        let dd = loop {
            let a = buf[si] as i8 as i32;
            if a != 0 || mem[e] != 0 {
                let b = mem[e] as i8 as i32;
                if a != b {
                    let mut aa = a;
                    let mut bb = b;
                    if aa == b'I' as i32 || aa == b'U' as i32 {
                        aa += 32;
                    }
                    if bb == b'I' as i32 || bb == b'U' as i32 {
                        bb += 32;
                    }
                    if aa != bb {
                        break aa - bb;
                    }
                }
                si += 1;
                e += 1;
            } else {
                break 0;
            }
        };
        let d = dd as i16 as i32;
        if d < 0 {
            hi = mid - 1;
        } else if d > 0 {
            lo = mid + 1;
        } else {
            let nm = u32::from_le_bytes([mem[eo], mem[eo + 1], mem[eo + 2], mem[eo + 3]]) as usize;
            strcpy0(buf, &mem[nm..]);
            return mem[eo + 4] as i8 as i32;
        }
        if lo > hi {
            return -1;
        }
    }
}

/// Static word table compiled into `fn_19e278` (decompiler-provided
/// constants, LE bytes): "S", SJIS 0x4281, ".", "!", "?", SJIS 0x4181,
/// SJIS 0x4381, ",", trailing word.
const DTAB_19E278: [u8; 36] = [
    0x53, 0x00, 0x00, 0x00, 0x81, 0x42, 0x00, 0x00, 0x2e, 0x00, 0x00, 0x00, 0x21, 0x00, 0x00,
    0x00, 0x3f, 0x00, 0x00, 0x00, 0x81, 0x41, 0x00, 0x00, 0x81, 0x43, 0x00, 0x00, 0x2c, 0x00,
    0x00, 0x00, 0x32, 0xe7, 0x00, 0x00,
];

/// fn_19e278 do_9c0: link the current kana node and start a fresh one.
/// Increments the mora kana count at `r6+4` (wrapping u8).
fn emit_kana(eng: &mut Engine, z: usize, r6: usize, r7: &mut usize, r4p: &mut usize) {
    let qv = eng.rd16(z + 0xE732);
    let old = *r7;
    let nw = old + 20;
    eng.wr16(old + 12, qv);
    eng.wr32(old + 16, nw as u32);
    eng.wr8(nw, 0);
    eng.wr8(nw + 5, 45);
    eng.wr8(nw + 4, 45);
    eng.wr16(nw + 8, 0);
    eng.wr16(nw + 6, 0);
    eng.wr32(nw + 16, 0);
    eng.wr16(nw + 12, qv);
    *r4p = nw;
    *r7 = nw;
    eng.wr8(r6 + 4, eng.rd8(r6 + 4).wrapping_add(1));
}

/// fn_19e278 do_a44/do_764: reset the current kana node in place.
fn reset_kana(eng: &mut Engine, z: usize, r7: usize) {
    eng.wr8(r7, 0);
    eng.wr8(r7 + 5, 45);
    eng.wr8(r7 + 4, 45);
    eng.wr16(r7 + 8, 0);
    eng.wr16(r7 + 6, 0);
    eng.wr32(r7 + 16, 0);
    eng.wr16(r7 + 12, eng.rd16(z + 0xE732));
}

/// fn_19e278: phrase tokenizer/unit-list builder. Reads `NUL`/CR/LF lines
/// via the cursor at `Z+0xDEE4`, parses `word r7 r8 r4 r9…` fields, and
/// appends 32-byte Mora units at `X+32` (names pooled from `Z+0x1960),
/// chaining 20-byte kana nodes from `Y`. Returns the unit count.
/// Row constraints: no empty input lines (the C reads `stok[-1]` for
/// those); `spbuf` tokens contain no `#` (else C/ARM disagree on the
/// `tag_command` return, a filed defect); node links never null-deref.
pub fn token_units(eng: &mut Engine, tab: &Tables, x: usize, y: usize, z: usize) -> i32 {
    use crate::jp::libc::{memcmp0, strcat0, strcmp0, strcpy0, strlen0, strtol10};
    let mut r6 = x + 32;
    let mut r5c = 0i32;
    let mut slot420 = 0i32;
    let mut slot436 = 0i32;
    let mut slot416 = z + 0x1960;
    let mut slot400 = 0i32;
    let mut slot404 = 0i32;
    let mut stok = [0u8; 192];
    let mut spbuf = [0u8; 192];
    // Kana-chain head spill (ARM orig 0x19e8a4/0x19ea78): reloaded per
    // line and advanced by this line's emit count * 20, so later lines
    // append after earlier lines' nodes instead of overwriting them.
    let mut chain = y;
    eng.wr8(y, 0);
    eng.wr8(y + 5, 45);
    eng.wr8(y + 4, 45);
    eng.wr16(y + 8, 0);
    eng.wr16(y + 6, 0);
    eng.wr32(y + 16, 0);
    eng.wr16(y + 12, eng.rd16(z + 0xE732));
    eng.wr16(x + 18, 0);
    eng.wr32(x + 28, (x + 32) as u32);
    loop {
        // copy loop c8: one line into stok (strip CR/LF, cap 191):
        let mut fedc_ctx = 0usize;
        {
            let ipchain = z + 0xDEE4;
            if ipchain == 0 {
                break;
            }
            let mut t1i = eng.rd32(ipchain) as usize;
            if t1i == 0 || eng.rd8(t1i) == 0 {
                break;
            }
            let mut t1 = 0i32;
            let mut cursor = 0usize;
            loop {
                t1 = t1 as i16 as i32;
                let c2 = eng.rd32(ipchain) as usize;
                let c3 = eng.rds8(c2) as i32;
                if c3 == 10 || c3 == 13 {
                    eng.wr32(ipchain, (c2 + 1) as u32);
                    break;
                }
                eng.wr32(ipchain, (c2 + 1) as u32);
                let c2b = eng.rd8(c2) as i32;
                t1 += 1;
                stok[cursor] = c2b as u8;
                cursor += 1;
                let nx = eng.rd32(ipchain) as usize;
                if eng.rd8(nx) == 0 {
                    break;
                }
                if t1 >= 191 {
                    break;
                }
            }
            stok[cursor] = 0;
            if t1 < 0 {
                break;
            }
        }
        // strip one trailing control byte (panics on empty lines, like the
        // C reads stok[-1] there — out of model, never on corpus):
        let mut len = strlen0(&stok) as i32;
        len = len as i16 as i32;
        let last = stok[(len - 1) as usize] as i8 as i32;
        if last < 32 {
            stok[(len - 1) as usize] = 0;
        }
        eng.wr16(r6 + 20, 0);
        eng.wr16(r6 + 14, 0);
        // Tokens are optional (C defaults); short lines still parse.
        spbuf[0] = 0;
        let mut r7 = 0i32;
        let mut r8 = 10i32;
        let mut r4 = 10i32;
        let mut r9 = 200i32;
        if let Some(o) = tokenize(&mut stok, &mut fedc_ctx, b" \0") {
            strcpy0(&mut spbuf, &stok[o..]);
        }
        if let Some(o) = tokenize(&mut stok, &mut fedc_ctx, b" \0") {
            r7 = strtol10(&stok[o..]);
        }
        if let Some(o) = tokenize(&mut stok, &mut fedc_ctx, b" \0") {
            r8 = strtol10(&stok[o..]);
        }
        if let Some(o) = tokenize(&mut stok, &mut fedc_ctx, b" \0") {
            r4 = strtol10(&stok[o..]);
        }
        if let Some(o) = tokenize(&mut stok, &mut fedc_ctx, b" \0") {
            r9 = strtol10(&stok[o..]);
        }
        if let Some(o) = tokenize(&mut stok, &mut fedc_ctx, b" \0") {
            eng.wr16(r6 + 14, strtol10(&stok[o..]) as u16);
        }
        if let Some(o) = tokenize(&mut stok, &mut fedc_ctx, b" \0") {
            eng.wr16(r6 + 20, strtol10(&stok[o..]) as u16);
        }
        // filter: drop leading/repeated Q and control bytes:
        let mut fi = 0i32;
        let mut flen = 0i32;
        loop {
            let ch = spbuf[fi as usize] as i8 as i32;
            if ch == 0 {
                break;
            }
            if ch != b'Q' as i32 {
                if ch >= 32 {
                    spbuf[flen as usize] = ch as u8;
                    flen = (flen + 1) as i16 as i32;
                }
            } else if fi != 0 && spbuf[(fi - 1) as usize] != b'Q' {
                spbuf[flen as usize] = ch as u8;
                flen = (flen + 1) as i16 as i32;
            }
            fi = (fi + 1) as i16 as i32;
            if fi >= 192 {
                break;
            }
        }
        spbuf[flen as usize] = 0;
        if flen == 0 {
            continue;
        }

        len = strlen0(&spbuf) as i32;
        if len > 96 {
            spbuf[96] = 0;
        }
        // per-unit DSP params from the numeric fields:
        eng.wr8(r6 + 5, r7 as u8);
        eng.wr8(r6 + 6, r7 as u8);
        let q2 = r8.wrapping_mul(655);
        let mut qq1 = eng.rds16(z + 0xE734) as i32;
        qq1 = qq1.wrapping_mul(q2);
        qq1 <<= 8;
        qq1 = qq1.wrapping_add(0x800000);
        qq1 >>= 24;
        eng.wr8(r6 + 8, qq1 as u8);
        let q2 = r4.wrapping_mul(655);
        let mut qq1 = eng.rds16(z + 0xE736) as i32;
        qq1 = qq1.wrapping_mul(q2);
        qq1 <<= 8;
        qq1 = qq1.wrapping_add(0x800000);
        qq1 >>= 24;
        eng.wr8(r6 + 9, qq1 as u8);
        let s1 = (eng.rd16(z + 0xE73A) as i16 as i32 * 655 + 128) >> 8;
        let s2 = (s1.wrapping_mul(r9).wrapping_add(128)) >> 8;
        let s2r = s2.clamp(0, 30000);
        eng.wr16(r6 + 18, s2r as u16);
        eng.wr8(r6 + 12, eng.rd8(z + 0xE730));
        eng.wr16(r6 + 10, eng.rd16(z + 0xE73A));
        eng.wr16(r6 + 22, eng.rd16(z + 0xE738));
        eng.wr8(r6 + 7, 0);
        if z as u32 == 0xFFFF_FFFFu32 {
            // dead on ARM (Z is the engine; Z == -1 would fault):
            let e8r = lookup_word(&mut spbuf, &eng.mem[..], z) as i8 as i32;
            eng.wr8(r6 + 6, e8r as u8);
            if e8r == -1 {
                eng.wr8(r6 + 6, e1cc_count(&spbuf) as u8);
            }
        }
        stok[0] = 0;
        let mut slot456: i32;
        if tag_command(eng, &spbuf, 1) != 0 {
            slot456 = -3;
        } else {
            // lr table lookup: dash-mark substring hits in spbuf. ARM
            // resets r8 per table entry (mov at 0x19e674); the re-search
            // after a mark keeps the advanced r8 (branch to 0x19e67c).
            let mut lk9 = 0i32;
            loop {
                let mut r8 = 0i32;
                let t7 = (lk9 * 3) as usize;
                // strstr-prefix search for lrtab[t7..] from r8:
                let tlen = strlen0(&tab.t722727[t7..]);
                loop {
                    let mut pp = r8;
                    let mut hit: Option<i32> = None;
                    while spbuf[pp as usize] != 0 {
                        if memcmp0(&spbuf[pp as usize..], &tab.t722727[t7..], tlen) == 0 {
                            hit = Some(pp - r8);
                            break;
                        }
                        pp += 1;
                    }
                    if let Some(r5v) = hit {
                        r5c = r5v;
                        let o0 = (r8 + r5c + 1) as i16 as i32;
                        spbuf[o0 as usize] = 45;
                        r8 = (o0 + 1) as i16 as i32;
                        if r8 < 96 {
                            continue;
                        }
                    }
                    break;
                }
                lk9 = (lk9 + 1) as i16 as i32;
                if lk9 >= 5 {
                    break;
                }
            }
            // delimiter chain:
            if strcmp0(&spbuf, &DTAB_19E278[0..]) == 0 {
                slot456 = -1;
            } else if strcmp0(&spbuf, &DTAB_19E278[4..]) == 0 {
                slot456 = -1;
            } else if strcmp0(&spbuf, &DTAB_19E278[8..]) == 0 {
                slot456 = -1;
            } else if strcmp0(&spbuf, &DTAB_19E278[12..]) == 0 {
                slot456 = -1;
            } else if strcmp0(&spbuf, &DTAB_19E278[16..]) == 0 {
                slot456 = -1;
            } else if strcmp0(&spbuf, &DTAB_19E278[20..]) == 0 {
                slot456 = -2;
            } else if strcmp0(&spbuf, &DTAB_19E278[24..]) == 0 {
                slot456 = -2;
            } else if strcmp0(&spbuf, &DTAB_19E278[28..]) == 0 {
                slot456 = -2;
            } else {
                strcpy0(&mut stok, &spbuf);
                let slen = strlen0(&stok) as i32;
                let lastc = spbuf[(slen - 1) as usize] as i8 as i32;
                if lastc == b'.' as i32 || lastc == b'!' as i32 || lastc == b'?' as i32 {
                    slot456 = -1;
                } else if lastc == b',' as i32 {
                    slot456 = -2;
                } else {
                    slot456 = 0;
                }
            }
        }
        // do_5cc: pool the name, then classify:
        let tlen = strlen0(&stok) as i32;
        if tlen + slot404 + 1 > 448 {
            stok[(448 - slot404 - 1) as usize] = 0;
        }
        strcpy0(&mut eng.mem[slot416..], &stok);
        eng.wr32(r6, slot416 as u32);
        let llen = strlen0(&eng.mem[slot416..]) as i32;
        slot400 = llen;
        if llen > 0 {
            let mut r7 = chain;
            let mut r4p = chain;
            let mut r4c = 0i32;
            eng.wr32(r6 + 24, chain as u32);
            let r9 = 128 - slot420;
            eng.wr8(r6 + 4, 0);
            let slen2 = strlen0(&eng.mem[eng.rd32(r6) as usize..]) as i16 as i32;
            if slen2 > 0 {
                let name0 = eng.rd32(r6) as usize;
                let mut r8 = 0i32;
                loop {
                    // classifier:
                    if eng.rds8(r6 + 4) as i32 >= r9 {
                        eng.wr8(name0 + r8 as usize, 0);
                        break;
                    }
                    let cc = eng.rd8(name0 + r8 as usize) as i32;
                    let mut ccm = cc;
                    let mut advanced = false;
                    if cc == 81 {
                        eng.wr8(r7 + 4, 81);
                    } else if cc == 69 || cc == 73 || cc == 79 || cc == 65 {
                        // do_930:
                        ccm = if cc - 65 <= 25 { cc + 32 } else { cc };
                        eng.wr8(r7 + 5, 85);
                        advanced = true;
                    } else if cc == 78 || cc == 45 || cc == 97 || cc == 101 || cc == 105
                        || cc == 111 || cc == 117
                    {
                        // do_948:
                        advanced = true;
                    } else if cc == 85 {
                        // do_930:
                        ccm = if cc - 65 <= 25 { cc + 32 } else { cc };
                        eng.wr8(r7 + 5, 85);
                        advanced = true;
                    } else if cc == 126 {
                        // do_754 directly
                    } else if cc == 33 {
                        // do_914:
                        let mut t0 = eng.rds8(r6 + 7) as i32;
                        if 3u32 > (((t0 & 0x70) >> 4) as u32) {
                            t0 += 16;
                            eng.wr8(r6 + 7, t0 as u8);
                        }
                    } else if cc == 63 {
                        // do_8f8:
                        let mut t0 = eng.rds8(r6 + 7) as i32;
                        if ((t0 as u32) & 7) < 3 {
                            t0 += 1;
                            eng.wr8(r6 + 7, t0 as u8);
                        }
                    } else {
                        // do_748:
                        if r4p.wrapping_sub(r7) < 2 {
                            eng.wr8(r4p, ccm as u8);
                            r4p += 1;
                        }
                    }
                    if advanced {
                        // do_948 tail (do_a14 falls into do_950retry): ARM
                        // skips the 76-write when r7 == z+16 (beq to the
                        // do_754 tail) and otherwise writes it when the
                        // node name is still empty.
                        let mut retry = ccm != 45;
                        if ccm == 45 && r7 != z + 16 && eng.rd8(r7) == 0 {
                            eng.wr8(r7 - 15, 76);
                            retry = true;
                        }
                        if retry {
                            // do_950retry:
                            eng.wr8(r4p, ccm as u8);
                            eng.wr8(r4p + 1, 0);
                            if strcmp0(&eng.mem[r7..], b"-\0") == 0 {
                                emit_kana(eng, z, r6, &mut r7, &mut r4p);
                            } else {
                                r4c = 0;
                                loop {
                                    let e1 = (r4c * 4) as usize;
                                    if strcmp0(&eng.mem[r7..], &tab.t74362e[e1..]) == 0 {
                                        break;
                                    }
                                    if strcmp0(&eng.mem[r7..], &tab.t74362e[e1 + 4..]) == 0 {
                                        r4c = (r4c + 1) as i16 as i32;
                                        break;
                                    }
                                    r4c = (r4c + 2) as i16 as i32;
                                    if r4c >= 144 {
                                        break;
                                    }
                                }
                                if r4c == 144 {
                                    reset_kana(eng, z, r7);
                                    r4p = r7;
                                } else {
                                    emit_kana(eng, z, r6, &mut r7, &mut r4p);
                                }
                            }
                        }
                    }
                    // do_754:
                    r8 = (r8 + 1) as i16 as i32;
                    if r8 >= slen2 {
                        break;
                    }
                }
            }
            // do_764: rebuild the pooled name from the kana chain:
            eng.wr8(r7, 0);
            eng.wr8(r7 + 5, 45);
            eng.wr8(r7 + 4, 45);
            eng.wr16(r7 + 8, 0);
            eng.wr16(r7 + 6, 0);
            eng.wr32(r7 + 16, 0);
            eng.wr16(r7 + 12, eng.rd16(z + 0xE732));
            let namep = eng.rd32(r6) as usize;
            eng.wr8(namep, 0);
            let r0v = eng.rds8(r6 + 4) as i32;
            let mut node = eng.rd32(r6 + 24) as usize;
            if r0v > 0 {
                let mut cnt = 0i32;
                loop {
                    let nl = strlen0(&eng.mem[node..]);
                    let mut nm = [0u8; 8];
                    nm[..nl + 1].copy_from_slice(&eng.mem[node..node + nl + 1]);
                    let dl = strlen0(&eng.mem[namep..]);
                    strcat0(&mut eng.mem[namep..], dl, &nm);
                    cnt = (cnt + 1) as i16 as i32;
                    node = eng.rd32(node + 16) as usize;
                    if !(r0v > cnt) {
                        break;
                    }
                }
            }
            let r0v = eng.rds8(r6 + 4) as i32;
            if r0v == 0 {
                continue;
            }
            chain = chain.wrapping_add(r0v as usize * 20);
            slot420 += r0v;
            let t1 = eng.rds8(r6 + 7) as i32;
            if (t1 & 0x70) != 0 {
                let t2b = (t1 as u32 >> 31) as i32;
                let mut t0 = 2 + (t2b >> 4);
                t0 >>= 1;
                let t1b = eng.rd8(r6 + 8) as i32;
                t0 = (t1b as i16 as i32) * (t0 as i16 as i32);
                eng.wr8(r6 + 8, t0 as u8);
            }
            // do_824:
            slot436 = (slot436 + 1) as i16 as i32;
            eng.wr32(r6 + 28, (r6 + 32) as u32);
            r6 += 32;
        }
        // do_840:
        slot416 += slot400 as usize + 1;
        slot404 = slot400 + slot404 + 1;
        if slot456 == -1 {
            break;
        }
        if eng.rds16(r6 - 14) > 0 {
            break;
        }
        if slot420 < 128 && slot436 < 64 && slot404 < 0x1BF {
            continue;
        }
        break;
    }
    // bail8c8:
    if slot436 > 0 {
        let fv = eng.rds16(r6 - 14) as i32;
        if fv == 0 {
            eng.wr16(r6 - 14, 100);
        }
    }
    eng.wr32(r6 - 4, 0);
    slot436
}

/// fn_1a1d80: kana-synth task driver. Re-tokenizes the kana cursor
/// (`token_units`), rebuilds the phrase lists (`list_build`, `seg_prosody`,
/// `prosody_walk`), packs 14-byte phoneme records at 0x1B22 from the table
/// chain at 0x1660, then drains each table through mode/post shaping and
/// `phrase_drive`/`rate_setup`/`phrase_driver` until the done flag latches.
/// Returns 0 after the AFD8 completion callback (ROM 0x607B10, voice-page
/// only) and the 6b17b8/6b17ec unlock pair.
/// Row constraints (as on ARM): 0xE6D0 == 0 (the config branch needs ROM
/// file IO fn_100db6/fn_100b30, unreached and unported; e6d0 is
/// construct-zeroed with no writer on the render path); node names
/// non-empty with strlen <= 31 when node+6 != 0 (`buf[32]`); table count
/// bytes >= 0; node/sl/t1/table links never null-deref mid-walk (the
/// S-record reads node[4] post-walk like ARM, which faults on null).
pub fn task_driver(eng: &mut Engine, tab: &Tables<'_>, pcm: &mut PcmLog) -> u32 {
    use crate::jp::libc::{strlen0, strncpy0};
    ring_init(eng);
    eng.wr32(0xDEE4, 0x85A2);
    let mut buf = [0u8; 32];
    'outer: loop {
        // e6d0 == 1 config branch dead: see row constraints.
        if token_units(eng, tab, 0xA24, 16, 0) == 0 {
            break 'outer;
        }
        list_build(eng, 0xA44, 0x1244, 0x1654);
        seg_prosody(eng, tab, 0x1254);
        let _ = prosody_walk(eng, 0x1660, tab);
        let mut table = 0x1660usize;
        let mut idx = 0i32;
        'table: while table != 0 {
            let mut nph = eng.rds8(table) as i32;
            let mut t1 = eng.rd32(table + 4) as usize;
            let mut rp = (0x1B22 + idx * 14 + 14) as usize;
            let mut count = 0i32;
            let limit = 255 - idx;
            'nph: while nph > 0 {
                let mut c0 = eng.rds8(t1) as i32;
                let mut sl = eng.rd32(t1 + 4) as usize;
                while c0 > 0 {
                    let mut n8 = eng.rds8(sl + 4) as i32;
                    let mut node = eng.rd32(sl + 24) as usize;
                    while n8 > 0 {
                        if eng.rd8(node + 5) == 0x55 {
                            let v = eng.rds16(node + 8) as i32;
                            if v > 20 {
                                let w = eng.rd16(node + 6) as i32;
                                eng.wr16(node + 8, 20);
                                eng.wr16(node + 6, (v + w - 20) as u16);
                            }
                        }
                        let len: i32;
                        if eng.rd16(node + 6) == 0 {
                            len = 1;
                        } else {
                            len = (strlen0(&eng.mem[node..]) as i32) as i16 as i32;
                            strncpy0(&mut buf, &eng.mem[node..], (len - 1) as usize);
                            buf[(len - 1) as usize] = 0;
                            let c = class_lookup(&buf, tab.ph38);
                            eng.wr8(rp, c);
                            eng.wr16(rp + 2, eng.rd16(node + 6));
                            count += 1;
                            if limit <= count {
                                break 'nph;
                            }
                            eng.wr16(rp + 6, 0);
                            eng.wr16(rp + 4, 0);
                            eng.wr16(rp + 10, 0);
                            eng.wr16(rp + 8, 0);
                            eng.wr8(rp + 12, eng.rd8(node + 4));
                            eng.wr8(rp + 13, eng.rd8(node + 5));
                            rp += 14;
                        }
                        let lb = node.wrapping_add((len - 1) as usize);
                        if eng.rd8(lb) != 0x2d {
                            let c = class_lookup(&eng.mem[lb..], tab.ph38);
                            eng.wr8(rp, c);
                            eng.wr16(rp + 2, eng.rd16(node + 8));
                            count += 1;
                            let d1 = eng.rds16(node + 12);
                            let d2 = eng.rds16(node + 10);
                            eng.wr16(rp + 6, d1 as u16);
                            eng.wr16(rp + 4, d2 as u16);
                            eng.wr16(rp + 10, 0);
                            eng.wr16(rp + 8, 0);
                            eng.wr8(rp + 12, eng.rd8(node + 4));
                            eng.wr8(rp + 13, eng.rd8(node + 5));
                            if d1 <= 0 {
                                eng.wr16(rp + 6, 0);
                            }
                            if d2 <= 0 {
                                eng.wr16(rp + 4, 0);
                            }
                            rp += 14;
                            if count >= limit {
                                break 'nph;
                            }
                        } else {
                            let s = eng.rds16(rp - 12) as i32
                                + eng.rds16(node + 8) as i32;
                            let p = eng.rds16(rp - 6) as i32;
                            if s < 0x8000
                                && (p == 0 || p == eng.rds16(node + 12) as i32)
                            {
                                eng.wr16(rp - 12, s as u16);
                                eng.wr16(rp - 6, eng.rd16(node + 12));
                            } else {
                                eng.wr8(rp, eng.rd8(rp - 14));
                                eng.wr16(rp + 2, eng.rd16(node + 8));
                                count += 1;
                                let d1 = eng.rds16(node + 12);
                                let d2 = eng.rds16(node + 10);
                                eng.wr16(rp + 6, d1 as u16);
                                eng.wr16(rp + 4, d2 as u16);
                                eng.wr16(rp + 10, 0);
                                eng.wr16(rp + 8, 0);
                                eng.wr8(rp + 12, eng.rd8(node + 4));
                                eng.wr8(rp + 13, eng.rd8(node + 5));
                                if d1 <= 0 {
                                    eng.wr16(rp + 6, 0);
                                }
                                if d2 <= 0 {
                                    eng.wr16(rp + 4, 0);
                                }
                                rp += 14;
                                if count >= limit {
                                    break 'nph;
                                }
                            }
                        }
                        // adv:
                        node = eng.rd32(node + 16) as usize;
                        n8 -= 1;
                    }
                    if eng.rd16(sl + 18) != 0 {
                        let c = class_lookup(b"S", tab.ph38);
                        eng.wr8(rp, c);
                        eng.wr16(rp + 2, eng.rd16(sl + 18));
                        count += 1;
                        if limit <= count {
                            break 'nph;
                        }
                        eng.wr16(rp + 6, 0);
                        eng.wr16(rp + 4, 0);
                        eng.wr16(rp + 10, 0);
                        eng.wr16(rp + 8, 0);
                        eng.wr8(rp + 12, eng.rd8(node + 4));
                        rp += 14;
                    }
                    c0 -= 1;
                    sl = eng.rd32(sl + 28) as usize;
                }
                nph -= 1;
                t1 = eng.rd32(t1 + 12) as usize;
            }
            // phrase:
            eng.wr8(rp, 7);
            let nt = eng.rd32(table + 8) as usize;
            idx += count;
            if nt != 0 && (count == 0 || eng.rd8((0x1B22 + idx * 14) as usize) != 0)
            {
                table = nt;
                continue 'table;
            }
            // render:
            let t0 = eng.rds8(table) as i32;
            let t1init = eng.rd32(table + 4) as usize;
            let desc =
                eng.rd32(t1init.wrapping_add((t0 * 16 - 8) as usize)) as usize;
            let mode = eng.rd32(0xE740) as i32;
            let sfl = eng.rds8(desc + 7) as i32;
            rp = (0x1B22 + idx * 14) as usize;
            let mut skip_post = false;
            match mode {
                1 => {
                    if idx <= 4 || idx >= 253 || eng.rd8(rp) != 0 {
                        skip_post = true;
                    } else {
                        if eng.rds16(rp - 22) != 0 {
                            let v = eng.rds16(rp - 22) as i32;
                            let w = eng.rd16(rp - 26) as i32;
                            eng.wr16(rp - 22, (v + 50) as u16);
                            eng.wr16(rp - 26, (w + 50) as u16);
                        } else if eng.rds16(rp - 36) != 0 {
                            let v = eng.rds16(rp - 36) as i32;
                            let w = eng.rd16(rp - 40) as i32;
                            eng.wr16(rp - 36, (v + 50) as u16);
                            eng.wr16(rp - 40, (w + 50) as u16);
                        }
                        let v = eng.rd16(rp - 8) as i32;
                        let w = eng.rd16(rp - 12) as i32;
                        let b = eng.rd8(rp);
                        eng.wr16(rp - 8, (v + 0x1c2) as u16);
                        eng.wr8(rp + 14, b);
                        eng.wr16(rp + 16, eng.rd16(rp + 2));
                        eng.wr16(rp - 12, (w + 200) as u16);
                        eng.wr8(rp, eng.rd8(rp - 14));
                        let h = (((v + 0x1c2) as u16) as i32 - 40) as i16;
                        eng.wr16(rp + 6, h as u16);
                        if h < 0 {
                            eng.wr16(rp + 6, 0);
                        }
                        let h = (eng.rd16(rp - 10) as i32 - 200) as i16;
                        eng.wr16(rp + 4, h as u16);
                        if h < 0 {
                            eng.wr16(rp + 4, 0);
                        }
                        eng.wr16(rp + 2, 10);
                        idx += 1;
                        eng.wr16(rp - 6, 0);
                        eng.wr8(rp + 13, 45);
                        eng.wr16(rp + 8, 0);
                        rp += 14;
                    }
                }
                2 => {
                    if idx <= 4 {
                        skip_post = true;
                    } else {
                        let base = 0x1B22 + 14;
                        let span = rp.wrapping_sub(base) as i32;
                        if span > 13 {
                            let cnt = span / 14;
                            let mut p0 = base + 6;
                            let mut p1 = base + 2;
                            let mut p2 = base + 8;
                            let mut started = 0;
                            for _ in 0..cnt {
                                if eng.rd16(p0) != 0 {
                                    if started == 1 {
                                        eng.wr16(p2, 0);
                                        let w = eng.rd16(p0);
                                        eng.wr16(p0, w.wrapping_add(300));
                                        let w = eng.rd16(p1);
                                        eng.wr16(p1, w.wrapping_add(150));
                                        started = 0;
                                    } else {
                                        started = 1;
                                    }
                                }
                                p0 += 14;
                                p1 += 14;
                                p2 += 14;
                            }
                            rp = base + (cnt as usize) * 14;
                        }
                    }
                }
                10 => {
                    if idx <= 4 || idx >= 251 {
                        skip_post = true;
                    } else if eng.rd8(rp) != 0 || eng.rd8(rp - 14) != 6 {
                        skip_post = true;
                    } else {
                        let mut v8 = eng.rds16(rp - 12) as i32;
                        let sp28: i32;
                        if v8 <= 120 {
                            sp28 = eng.rds16(rp - 8) as i32;
                            v8 = 120;
                        } else {
                            let v = eng.rds16(rp - 6) as i32;
                            if v == 0 {
                                sp28 = eng.rds16(rp - 8) as i32 - 50;
                            } else {
                                v8 = v8.wrapping_add(((v8 as u32) >> 31) as i32)
                                    >> 1;
                                sp28 = v - 50;
                            }
                        }
                        if eng.rds16(rp + 2) as i32 >= 800 && idx >= 8 {
                            eng.wr16(rp + 44, eng.rd16(rp + 2));
                            let c = class_lookup(b"S", tab.ph38);
                            eng.wr8(rp + 42, c);
                            let c = class_lookup(b"n", tab.ph38);
                            eng.wr8(rp, c);
                            let half = v8.wrapping_add(((v8 as u32) >> 31) as i32)
                                >> 1;
                            eng.wr16(rp + 2, half as u16);
                            eng.wr16(rp + 4, 0);
                            eng.wr16(rp + 6, 0);
                            rp += 14;
                            let c = class_lookup(b"e", tab.ph38);
                            eng.wr8(rp, c);
                            eng.wr16(rp + 2, v8 as u16);
                            let w9 = sp28 as i16;
                            eng.wr16(rp + 6, w9 as u16);
                            if w9 < 0 {
                                eng.wr16(rp + 6, 0);
                            }
                            eng.wr16(rp + 8, 0);
                            eng.wr16(rp + 4, eng.rd16(rp - 24));
                            rp += 14;
                            let c = class_lookup(b"N", tab.ph38);
                            eng.wr8(rp, c);
                            eng.wr16(rp + 2, ((v8 as f64 * 1.4) as i32) as u16);
                            eng.wr16(rp + 6, w9 as u16);
                            if w9 < 0 {
                                eng.wr16(rp + 6, 0);
                            }
                            eng.wr16(rp + 8, 0);
                            eng.wr16(rp + 4, eng.rd16(rp - 38));
                            rp += 14;
                            idx += 3;
                        } else {
                            eng.wr16(rp + 30, eng.rd16(rp + 2));
                            let c = class_lookup(b"S", tab.ph38);
                            eng.wr8(rp + 28, c);
                            let c = class_lookup(b"n", tab.ph38);
                            eng.wr8(rp, c);
                            let half = v8.wrapping_add(((v8 as u32) >> 31) as i32)
                                >> 1;
                            eng.wr16(rp + 2, half as u16);
                            eng.wr16(rp + 4, 0);
                            eng.wr16(rp + 6, 0);
                            let np = rp + 14;
                            let c = class_lookup(b"a", tab.ph38);
                            eng.wr8(np, c);
                            eng.wr16(np + 2, v8 as u16);
                            idx += 2;
                            eng.wr16(np + 6, (sp28 + 50) as u16);
                            eng.wr16(np + 8, 0);
                            eng.wr16(np + 4, eng.rd16(np - 24));
                            rp += 28;
                        }
                    }
                }
                _ => {}
            }
            // post:
            if !skip_post {
                eng.wr8(rp + 14, 7);
                if !(idx <= 0 || sfl == 0 || idx >= 253) {
                    let mut mx = eng.rds16(0x1B22 + 20) as i32;
                    let mut mn = mx;
                    let mut k = 1;
                    while k < idx {
                        let ip = (0x1B22 + k * 14) as usize;
                        let mut v = eng.rds16(ip + 6) as i32;
                        if v != 0 {
                            if v > mx {
                                mx = v;
                            }
                            if v < mn {
                                mn = v;
                            }
                        }
                        v = eng.rds16(ip + 8) as i32;
                        if v != 0 {
                            if v > mx {
                                mx = v;
                            }
                            if v < mn {
                                mn = v;
                            }
                        }
                        k += 1;
                    }
                    let e = (0x1B22 + idx * 14) as usize;
                    let r5p = if eng.rd16(e - 6) != 0 { e - 6 } else { e - 8 };
                    if (sfl & 112) != 0 {
                        let f2 = (sfl & 112) >> 4;
                        let w = eng.rd16(e - 10);
                        eng.wr16(e - 10, w.wrapping_add((f2 * 30) as u16));
                        let mut v3 = ((eng.rds16(e - 12) as f64 * 0.8) as i32)
                            as i16;
                        if v3 < 10 {
                            v3 = 10;
                        }
                        eng.wr16(e - 12, v3 as u16);
                        let w = eng.rd16(r5p);
                        eng.wr16(r5p, w.wrapping_add((f2 * 100) as u16));
                    }
                    if (sfl & 7) != 0 {
                        let f6 = sfl & 7;
                        let sc = f6 * 220;
                        let e8 = eng.rds16(e - 8) as i32;
                        let acc = (mx - e8) * f6;
                        let mut lo = acc / 3 + e8;
                        lo = lo.clamp(0, 0x7FFF);
                        lo += sc >> 1;
                        lo = lo as i16 as i32;
                        let mut d = (lo - sc) as i16;
                        if d < 0 {
                            d = 0;
                        }
                        let e0 = eng.rd8(e);
                        eng.wr8(e + 14, e0);
                        let e2 = eng.rd16(e + 2);
                        eng.wr16(e + 16, e2);
                        eng.wr8(e + 28, 7);
                        eng.wr8(e, eng.rd8(e - 14));
                        let h = eng.rds16(e - 12) >> 1;
                        eng.wr16(e + 2, h as u16);
                        if h < 10 {
                            eng.wr16(e + 2, 10);
                        }
                        let h = (eng.rd16(e - 10) as i32 - 90) as i16;
                        eng.wr16(e + 4, h as u16);
                        if h < 0 {
                            eng.wr16(e + 4, 0);
                        }
                        eng.wr16(e + 6, lo as u16);
                        eng.wr16(e + 8, 0);
                        eng.wr16(e - 8, d as u16);
                        if eng.rd16(e - 6) != 0 {
                            eng.wr16(e - 6, d as u16);
                        }
                        let mut h =
                            ((eng.rds16(e - 12) as f64 * 0.8) as i32) as i16;
                        if h < 10 {
                            h = 10;
                        }
                        eng.wr16(e - 12, h as u16);
                    }
                }
            }
            // fin:
            eng.wr16(0x3E30, 0);
            idx = 0;
            phrase_drive(eng, 0x1B30, tab);
            let e748 = eng.rd32(0xE748) as i32;
            rate_setup(eng, 0xADA4, e748);
            phrase_driver(eng, tab, pcm);
            if flag_done(eng) == 1 {
                break 'table;
            }
            table = eng.rd32(table + 8) as usize;
        }
        // stage:
        if flag_done(eng) == 1 {
            break 'outer;
        }
        ring_wrap(eng, tab);
        ring_init(eng);
    }
    // cleanup:
    if eng.rd32(0xAFD8) != 0 {
        // Completion signal via ROM 0x607B10: consumes the done flag and
        // the AFE0 cookie inside the voice page only (vq probe: 0 engine
        // diffs); the C discards its return.
        let _ = flag_done(eng);
        let _ = eng.rd32(0xAFE0);
    }
    eng.wr8(0xE760, 0);
    sync::mutex_lock(eng, 0);
    eng.wr8(0xE6CB, 0);
    sync::mutex_unlock(eng);
    0
}


/// fn_1a4cdc: render dispatch closure. Seals the record header, refreshes
/// start state, parses tags and converts kana to units at 0x85A2, then
/// either runs `task_driver` inline (0xE6CD != 0) or submits it as a
/// deferred job (`job_submit`, stubbed wait). Returns the submit code.
/// `sync` carries the guest facts the engine cannot derive.
pub fn dispatch(eng: &mut Engine, tab: &Tables<'_>, pcm: &mut PcmLog, sync: sync::GuestSync) -> u32 {
    eng.wr32(0, 0x2AF8);
    eng.wr8(0x1B21, 0);
    eng.wr16(4, 55);
    let _ = start_init(eng);
    let e730 = eng.rd8(0xE730);
    shadow_copy(eng);
    if e730 == 1 {
        eng.wr8(0xE730, 1);
    }
    eng.wr16(0x85A0, 1);
    tag_parse(&mut eng.mem[..], 0xA9A2, tab.tag53);
    let _ = kana_map(eng, tab, 0xA9A2, 0x85A2, 0x2400);
    if eng.rd8(0xE6CD) != 0 {
        return task_driver(eng, tab, pcm);
    }
    if libc::memcmp0(&eng.mem[0xE6D8..0xE6E0], tab.t7227d8, 8) != 0 {
        sync::event_wait(&mut eng.mem[..], 0xE6D8, 0);
        sync::event_wait_signal(eng, 0xE6D8, 0);
    }
    eng.wr32(0xE6D8, 0);
    eng.wr32(0xE6DC, 0);
    // t.hi = -1, t.lo = E6E0: consumed by the submit path with no further
    // engine effect under stubbed SVCs (see `job_submit`).
    let _ = eng.rd32(0xE6E0);
    // fn_6b18b0(q, fn_1a1d80, p, p+0x13584, t): SyncObj words plus the
    // scheduler request struct; arg is the guest engine pointer.
    sync::job_submit(
        &mut eng.mem[..],
        0xE6D8,
        sync,
        sync::JobSubmit {
            cb: 0x1A1D80,
            arg: sync.engine_base,
            out_off: 0x13584,
        },
    )
}

/// Shared submit tail of `render_entry`: start init, text copy, trailing
/// lead-byte trim, then `dispatch`. `sync` carries the guest facts the
/// engine cannot derive (forwarded to `dispatch`).
fn submit_text(eng: &mut Engine, text: &[u8]) {
    let _ = start_init(eng);
    libc::bzero0(&mut eng.mem[0xA9A2..], 0x400);
    libc::strncpy0(&mut eng.mem[0xA9A2..], text, 0x400);
    if eng.rd8(0xADA1) != 0 {
        eng.wr8(0xADA1, 0);
        if eng.rd8(0xADA0) & 0x80 != 0 {
            eng.wr8(0xADA0, 0);
        }
    }
}

/// fn_19be8c: engine submit/start (Speak path). Validates mode/fmt, takes
/// the E6FC lock, dispatches per mode, copies the Shift-JIS text to 0xA9A2
/// and runs `dispatch`. `tid` is the borrowed thread word for the mode-0
/// busy owner check; `text` needs the 0x400 window readable.
/// Row constraints (as on ARM, single-threaded): modes 0/1 enter with
/// 0xE6CB == 0 (a set busy flag spins on another thread's render, which
/// cannot complete here); the C's final `else` (mode > 2) is dead via the
/// early return, as is its address-test `pada0` variant.
pub fn render_entry(
    eng: &mut Engine,
    tab: &Tables<'_>,
    text: &[u8],
    mode: u32,
    fmt: u32,
    snd: u32,
    voice: u32,
    tid: u32,
    pcm: &mut PcmLog,
    sync: sync::GuestSync,
) -> i32 {
    // weak_19be8c: empty weak stub (no-op).
    if mode >= 3 {
        return -1;
    }
    if fmt > 31 {
        return -1;
    }
    eng.wr32(0xE6E0, fmt);
    sync::mutex_lock_at(eng.mem, 0xE6FC, 0);
    if mode == 2 {
        sync::mutex_lock(eng, 0);
        if eng.rd8(0xE6CB) != 0 {
            sync::mutex_unlock(eng);
            sync::mutex_unlock_at(eng.mem, 0xE6FC);
            return 0;
        }
        eng.wr8(0xE6CB, 1);
        eng.wr8(0xE6CC, 0);
        sync::mutex_unlock(eng);
        eng.wr8(0xE6CD, 0);
        eng.wr32(0xAFD8, snd);
        eng.wr32(0xAFE0, voice);
        submit_text(eng, text);
        let _ = dispatch(eng, tab, pcm, sync);
        sync::mutex_unlock_at(eng.mem, 0xE6FC);
        return 0;
    } else if mode == 1 {
        loop {
            if flag_busy(eng) == 0 {
                break;
            }
            sync::sleep_ticks(0x5F5E100, 0);
        }
        sync::mutex_lock(eng, 0);
        eng.wr8(0xE6CB, 1);
        eng.wr8(0xE6CC, 0);
        sync::mutex_unlock(eng);
        eng.wr8(0xE6CD, 0);
        eng.wr32(0xAFD8, snd);
        eng.wr32(0xAFE0, voice);
        submit_text(eng, text);
        let _ = dispatch(eng, tab, pcm, sync);
        sync::mutex_unlock_at(eng.mem, 0xE6FC);
        return 0;
    } else if mode == 0 {
        if flag_busy(eng) == 1 {
            sync::mutex_lock(eng, 0);
            let mut f = 0;
            if eng.rd8(0xE6CB) == 1 {
                eng.wr8(0xE6CC, 1);
                f = 1;
            }
            sync::mutex_unlock(eng);
            let v = sync::current_tid(tid);
            if f == 1 && sync::owner_query(eng.mem, 0xE6D8) != v {
                loop {
                    if flag_busy(eng) != 1 {
                        break;
                    }
                    sync::sleep_ticks(0x989680, 0);
                }
            }
        }
        // common:
        sync::mutex_lock(eng, 0);
        eng.wr8(0xE6CB, 1);
        eng.wr8(0xE6CC, 0);
        sync::mutex_unlock(eng);
        eng.wr8(0xE6CD, 0);
        eng.wr32(0xAFD8, snd);
        eng.wr32(0xAFE0, voice);
        submit_text(eng, text);
        let _ = dispatch(eng, tab, pcm, sync);
        sync::mutex_unlock_at(eng.mem, 0xE6FC);
        return 0;
    }
    // Dead: mode > 2 returned early.
    sync::mutex_unlock_at(eng.mem, 0xE6FC);
    0
}
