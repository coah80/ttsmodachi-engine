use crate::frontend_types::Error;

/// fn_1deae8 / EU fn_1deca4 / KR fn_1ded50.
pub fn validate_phrasing(present: bool, tag: u32) -> u32 {
    if present && tag == 0xf387 {
        0
    } else {
        0x8a002008
    }
}

/// fn_1deb04 / EU fn_1decc0 / KR fn_1ded6c.
pub fn close_phrasing(present: bool, tag: u32) -> u32 {
    validate_phrasing(present, tag)
}

/// fn_1e03ac / EU fn_1e0568 / KR fn_1e0614.
pub fn check_phrasing(present: bool, tag: u32) -> u32 {
    validate_phrasing(present, tag)
}

/// fn_1f51dc / EU fn_1f5398 / KR fn_1f5444.
pub fn validate_global(present: bool, tag: u32) -> u32 {
    if present && tag == 0xf388 {
        0
    } else {
        0x8a102008
    }
}

/// fn_1f7e8c / EU fn_1f8048 / KR fn_1f80f4.
pub fn check_global(present: bool, tag: u32) -> u32 {
    validate_global(present, tag)
}

/// fn_1e03c8 / EU fn_1e0584 / KR fn_1e0630.
pub fn phrasing_interface<'a, T: ?Sized>(
    index: u32,
    table: &'a T,
    output: &mut Option<&'a T>,
) -> u32 {
    if index > 1 {
        return 0x8a002001;
    }
    *output = Some(table);
    0
}

/// fn_1f7ea8 / EU fn_1f8064 / KR fn_1f8110.
pub fn global_interface<'a, T: ?Sized>(
    index: u32,
    table: &'a T,
    output: &mut Option<&'a T>,
) -> u32 {
    if index > 1 {
        return 0x8a102001;
    }
    *output = Some(table);
    0
}

/// fn_2943ec / EU fn_2945a8 / KR fn_294654.
pub fn count_phrase_boundaries(bytes: &[u8], count: u32) -> Result<u32, Error> {
    if count == 1 {
        return Ok(0);
    }
    if count == 0 {
        return Err(Error::InvalidResource);
    }
    if bytes.len() < count as usize {
        return Err(Error::Capacity {
            required: count as usize,
            available: bytes.len(),
        });
    }
    Ok(bytes[..count as usize]
        .windows(2)
        .filter(|pair| pair[0] == 0x10 && matches!(pair[1], 0x12 | 0x13))
        .count() as u32)
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct SynthesisWindow {
    pub rate: u16,
    pub period: u16,
    pub overlap: i16,
    pub duration: i32,
    pub samples: i16,
    pub cycles: i16,
    pub padding: i16,
    pub limit: i16,
    pub scaled: i16,
    pub phase: [i16; 2],
    pub length: u32,
    pub tail: i16,
    pub remaining: i16,
}

impl SynthesisWindow {
    /// fn_29556c / EU fn_295728 / KR fn_2957d4.
    pub fn set_percent(&mut self, percent: i32) -> Result<(), Error> {
        if percent <= 0 {
            return Ok(());
        }
        let scaled = (self.limit as i32 * percent as i16 as i32 / 100) as i16;
        self.scaled = scaled;
        let division = crate::runtime::signed_division(scaled as i32, self.limit as i32)
            .map_err(|_| Error::InvalidResource)?;
        self.cycles = division.quotient as i16;
        self.remaining = self.length.wrapping_sub(scaled as u32) as i16;
        Ok(())
    }

    /// fn_295444 / EU fn_295600 / KR fn_2956ac.
    pub fn set_duration(
        &mut self,
        duration: i32,
        buffer_length: Option<u32>,
        mut grow: impl FnMut(usize, u32) -> bool,
    ) -> u32 {
        self.duration = duration;
        let samples = (i32::from(self.rate).wrapping_mul(duration) / 1000) as i16;
        self.samples = samples;
        let tail = if samples < self.limit {
            samples
        } else {
            samples.wrapping_mul(2).wrapping_sub(self.limit)
        };
        self.tail = tail;
        self.length =
            (tail as i32 + (samples as i32 + self.padding as i32) * 2 - self.overlap as i32) as u32;
        if buffer_length.is_some_and(|length| self.length > length) {
            for index in 0..6 {
                if !grow(index, self.length >> (index / 2)) {
                    return 0x8100200a;
                }
            }
        }
        0
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ProsodyResource {
    Linguistic,
    Rules,
    Dictionary,
}

/// Caller-owned FE_PHRASING environment view, resolved through `fn_19611c`.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct PhrasingEnv {
    pub id: u32,
    pub heap: u32,
    pub log: u32,
    pub registry: u32,
}

/// One acquired FE_* component: its interface pointer and two-word handle.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct PhrasingResource {
    pub interface: u32,
    pub handle: [u32; 2],
}

/// Two-word engine handle published by an open/close entry point.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct FeHandle {
    pub obj: u32,
    pub tag: u32,
}

/// FE_PHRASING module instance (`fn_1de880`), 40 bytes in the original.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct Phrasing {
    pub env: u32,
    pub ling: u32,
    pub depes: [u32; 2],
    pub depes_if: u32,
    pub dct: [u32; 2],
    pub dct_if: u32,
    pub z32: u32,
    pub z36: u32,
}

impl Phrasing {
    /// Original 40-byte field order, for differential comparison.
    pub fn words(&self) -> [u32; 10] {
        [
            self.env,
            self.ling,
            self.depes[0],
            self.depes[1],
            self.depes_if,
            self.dct[0],
            self.dct[1],
            self.dct_if,
            self.z32,
            self.z36,
        ]
    }
}

pub trait PhrasingHost {
    fn resolve_env(&mut self, a: u32, b: u32) -> (u32, Option<PhrasingEnv>);
    fn acquire(&mut self, registry: u32, name: &[u8]) -> (u32, Option<PhrasingResource>);
    fn release(&mut self, registry: u32, name: &[u8]);
    fn allocate(&mut self, heap: u32, count: u32, size: u32) -> u32;
    fn log_error(&mut self, log: u32, line: u32);
    fn install(&mut self, object: u32, phrasing: Phrasing);
}

/// fn_1de880 / EU fn_1dea3c / KR fn_1deae8.
pub fn open_phrasing(
    a: u32,
    b: u32,
    host: &mut impl PhrasingHost,
    out: Option<&mut FeHandle>,
) -> u32 {
    let Some(out) = out else {
        return 0x8a002007;
    };
    *out = FeHandle::default();
    let (status, env) = host.resolve_env(a, b);
    if status >= 0x80000000 {
        return status;
    }
    let Some(env) = env else { return status };
    let (status, ling) = host.acquire(env.registry, b"LINGDB");
    if status >= 0x80000000 {
        return status;
    }
    let Some(ling) = ling else { return status };
    let (status, depes) = host.acquire(env.registry, b"FE_DEPES");
    if status >= 0x80000000 {
        host.release(env.registry, b"LINGDB");
        return status;
    }
    let Some(depes) = depes else { return status };
    let (status, dct) = host.acquire(env.registry, b"FE_DCTLKP");
    if status >= 0x80000000 {
        host.release(env.registry, b"FE_DEPES");
        host.release(env.registry, b"LINGDB");
        return status;
    }
    let Some(dct) = dct else { return status };
    let object = host.allocate(env.heap, 1, 40);
    if object == 0 {
        host.log_error(env.log, 0x9088);
        host.release(env.registry, b"LINGDB");
        host.release(env.registry, b"FE_DEPES");
        host.release(env.registry, b"FE_DCTLKP");
        return 0x8a00200a;
    }
    host.install(
        object,
        Phrasing {
            env: env.id,
            ling: ling.interface,
            depes_if: depes.interface,
            depes: depes.handle,
            dct_if: dct.interface,
            dct: dct.handle,
            z32: 0,
            z36: 0,
        },
    );
    out.obj = object;
    out.tag = 0xf387;
    status
}

/// Caller-owned FE_GLOBAL environment view, resolved through `fn_19611c`.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct GlobalEnv {
    pub id: u32,
    pub heap: u32,
    pub log: u32,
    pub params: u32,
    pub res: u32,
}

/// One `getCfg` result: value count, flag and first value byte.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct ConfigResult {
    pub count: u16,
    pub flag: u8,
    pub values_present: bool,
    pub first: u8,
}

/// FE_GLOBAL module instance (`fn_1f4e2c`), 64 bytes in the original.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct Global {
    pub id: u32,
    pub env: u32,
    pub ling_if: u32,
    pub depes: [u32; 2],
    pub depes_if: u32,
    pub dct: [u32; 2],
    pub dct_if: u32,
    pub f20: u32,
    pub f24: u32,
    pub f28: u32,
    pub f2c: u32,
    pub f30: u32,
    pub f34: u32,
    pub f38: u32,
    pub use_tn_tag: u8,
    pub clc_pipeline: u8,
    pub foreign: u8,
}

impl Global {
    /// Original 64-byte field order, for differential comparison.
    pub fn words(&self) -> [u32; 16] {
        [
            self.env,
            self.ling_if,
            self.depes[0],
            self.depes[1],
            self.depes_if,
            self.dct[0],
            self.dct[1],
            self.dct_if,
            self.f20,
            self.f24,
            self.f28,
            self.f2c,
            self.f30,
            self.f34,
            self.f38,
            u32::from(self.use_tn_tag)
                | (u32::from(self.clc_pipeline) << 8)
                | (u32::from(self.foreign) << 16),
        ]
    }
}

pub trait GlobalHost {
    fn resolve_env(&mut self, a: u32, b: u32) -> (u32, Option<GlobalEnv>);
    fn acquire(&mut self, res: u32, name: &[u8]) -> (u32, Option<PhrasingResource>);
    fn release(&mut self, res: u32, name: &[u8]);
    fn allocate(&mut self, heap: u32, size: u32) -> u32;
    fn log_error(&mut self, log: u32, line: u32);
    fn validate(&mut self, object: u32, b: u32, tag: u32, size: u32) -> u32;
    fn destroy(&mut self, object: u32, tag: u32);
    fn get_config(&mut self, dct: [u32; 2], section: &[u8], key: &[u8]) -> (u32, ConfigResult);
    fn get_string(&mut self, params: u32, key: &[u8]) -> (u32, Option<u8>);
    fn install(&mut self, object: u32, global: Global);
}

/// fn_1f4e2c / EU fn_1f4fe8 / KR fn_1f5094.
pub fn open_global(a: u32, b: u32, host: &mut impl GlobalHost, out: Option<&mut FeHandle>) -> u32 {
    let Some(out) = out else {
        return 0x8a102007;
    };
    *out = FeHandle::default();
    let (status, env) = host.resolve_env(a, b);
    if status >= 0x80000000 {
        return status;
    }
    let Some(env) = env else { return status };
    let (status, ling) = host.acquire(env.res, b"LINGDB");
    if status >= 0x80000000 {
        return status;
    }
    let Some(ling) = ling else { return status };
    let (status, depes) = host.acquire(env.res, b"FE_DEPES");
    if status >= 0x80000000 {
        host.release(env.res, b"LINGDB");
        return status;
    }
    let Some(depes) = depes else { return status };
    let (status, dct) = host.acquire(env.res, b"FE_DCTLKP");
    if status >= 0x80000000 {
        host.release(env.res, b"FE_DEPES");
        host.release(env.res, b"LINGDB");
        return status;
    }
    let Some(dct) = dct else { return status };
    let object = host.allocate(env.heap, 0x40);
    if object == 0 {
        host.log_error(env.log, 0x9470);
        host.release(env.res, b"LINGDB");
        host.release(env.res, b"FE_DEPES");
        host.release(env.res, b"FE_DCTLKP");
        return 0x8a10200a;
    }
    let mut global = Global {
        id: object,
        env: env.id,
        ling_if: ling.interface,
        depes: depes.handle,
        depes_if: depes.interface,
        dct: dct.handle,
        dct_if: dct.interface,
        ..Global::default()
    };
    let (status, config) = host.get_config(global.dct, b"fecfg", b"usetntag");
    if status >= 0x80000000 {
        host.destroy(out.obj, out.tag);
        *out = FeHandle::default();
    }
    if config.count == 1 && config.values_present && config.first == b'1' {
        global.use_tn_tag = 1;
    }
    let (_, mode) = host.get_string(env.params, b"clcpppipelinemode");
    if mode.is_some_and(|byte| byte != 0) {
        global.clc_pipeline = 1;
    }
    host.install(object, global);
    out.obj = object;
    out.tag = 0xf388;
    status
}

/// fn_1f5138 / EU fn_1f52f4 / KR fn_1f53a0.
pub fn refresh_global(global: Option<&mut Global>, b: u32, host: &mut impl GlobalHost) -> u32 {
    let status = host.validate(
        global.as_ref().map_or(0, |global| global.id),
        b,
        0xf388,
        0x40,
    );
    if status >= 0x80000000 {
        return 0x8a102008;
    }
    let Some(global) = global else { return status };
    global.use_tn_tag = 0;
    let (status, config) = host.get_config(global.dct, b"fecfg", b"usetntag");
    if status < 0x80000000 && config.count == 1 && config.values_present && config.first == b'1' {
        global.use_tn_tag = 1;
    }
    status
}

/// Caller-owned synthesis window buffers touched by `fn_2955c0`.
pub struct WindowBuffers<'a> {
    pub vectors: [&'a mut [u8]; 6],
    pub scratch: &'a mut [u8],
    pub table: &'a mut [u8],
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct WindowCursor {
    pub f16: i16,
    pub f18: i16,
    pub f2c: i16,
    pub f48: u32,
    pub f4c: u32,
    pub f50: u32,
    pub f_f0: i32,
    pub f_f4: i32,
    pub f_f8: u8,
    pub f_e5: u8,
}

pub trait WindowResetHost {
    fn reset_vector(&mut self, vector: &mut [u8]);
    fn clear_scratch(&mut self, scratch: &mut [u8]);
    fn fill_table(&mut self, table: &mut [u8]);
}

/// fn_2955c0 / EU fn_29577c / KR fn_295828.
pub fn reset_window(
    buffers: &mut WindowBuffers<'_>,
    cursor: &mut WindowCursor,
    host: &mut impl WindowResetHost,
) {
    for vector in &mut buffers.vectors {
        host.reset_vector(vector);
    }
    cursor.f_f4 = -1;
    cursor.f_f0 = -1;
    cursor.f_f8 = 1;
    cursor.f4c = 0;
    cursor.f50 = 0;
    cursor.f48 = 0;
    cursor.f_e5 = 0;
    host.clear_scratch(buffers.scratch);
    host.fill_table(buffers.table);
    cursor.f16 = 0;
    cursor.f18 = 0;
    cursor.f2c = 0;
}

/// Voice table consulted by `fn_28ff3c`: borrowed candidate names and the
/// voice-feature flag at +0x9e0.
pub struct VoiceSelector<'a> {
    pub names: &'a [Option<&'a [u8]>],
    pub voice_feature: i32,
}

impl VoiceSelector<'_> {
    fn name(&self, index: usize) -> Option<&[u8]> {
        self.names.get(index).copied().flatten()
    }
}

/// fn_2d0840 / EU fn_2d09fc / KR fn_2d0aa8.
///
/// Fixed-point product through the shared `fn_29b3f4`/`fn_29b0e8` helpers.
/// Only reached here through the synthesis-window table fill below.
pub fn fixed_product(first: i32, second: i32) -> i32 {
    let (first_high, first_low) = crate::selection::split_fixed(first);
    let (second_high, second_low) = crate::selection::split_fixed(second);
    let mut product = (first_low as i32)
        .wrapping_mul(second_low as i32)
        .wrapping_mul(2) as u32
        >> 15;
    product = crate::selection::add_i32(
        product as i32,
        (first_high as i32)
            .wrapping_mul(second_low as i32)
            .wrapping_mul(2),
    ) as u32;
    product = crate::selection::add_i32(
        product as i32,
        (first_low as i32)
            .wrapping_mul(second_high as i32)
            .wrapping_mul(2),
    ) as u32;
    crate::selection::add_i32(
        product as i32,
        (first_high as i32)
            .wrapping_mul(second_high as i32)
            .wrapping_shl(16),
    )
}

/// fn_2d08d8 / EU fn_2d0a94 / KR fn_2d0b40.
///
/// Window sine used by the `fn_29527c`/`fn_2956f4` table fill.
pub fn window_sine(angle: i32) -> i32 {
    let mut angle = angle;
    if angle < 0 {
        angle = angle.wrapping_neg();
    }
    loop {
        if angle <= 0x6487e {
            break;
        }
        angle = angle.wrapping_sub(0x6487e);
    }
    let mut sign = 1;
    if angle > 0x1921f && angle <= 0x3243f {
        sign = -1;
        angle = 0x3243f - angle;
    } else if angle > 0x3243f && angle <= 0x4b65e {
        sign = -1;
        angle -= 0x3243f;
    } else if angle > 0x4b65e {
        angle = 0x6487e - angle;
    }
    let square = fixed_product(angle, angle);
    fixed_product(fixed_product(0x918, square).wrapping_sub(0x7f27), square)
        .wrapping_add(0x10000)
        .wrapping_mul(sign)
}

/// fn_28ff3c / EU fn_2900f8 / KR fn_2901a4.
///
/// Scans `text` at `index` for a `name\` voice escape, resolves it against the
/// borrowed voice table, folds the escape out of `text`/`weights` and updates
/// `length`. `weights` starts four bytes into the caller's allocation so the
/// original `weights[index - 1]` read stays in bounds when `index` is zero.
/// Returns 1 when an escape was consumed, 0 otherwise.
pub fn select_voice(
    voice: &VoiceSelector<'_>,
    text: &mut [u8],
    mut weights: Option<&mut [u8]>,
    length: &mut i16,
    index: i32,
    voice_index: u32,
    current_voice: &mut u8,
    default_voice: &mut u8,
) -> i32 {
    use crate::resources::{compare_strings, string_length};
    let weights = &mut weights;
    let size = *length as i32;
    let mut name = [0u8; 12];
    let mut count = 0usize;
    let mut i = index;
    loop {
        if (i.wrapping_add(1) as u32) >= (size as u32) {
            break;
        }
        let byte = text[(i + 1) as usize];
        if byte == b'\\' {
            break;
        }
        name[count] = byte;
        count += 1;
        if count == 10 {
            i += 1;
            break;
        }
        i += 1;
    }
    if (size as u32) <= (i.wrapping_add(1) as u32) || text[(i + 1) as usize] != b'\\' {
        return 0;
    }
    let slash = u32::from(name[0] == b'/');
    let search = &name[slash as usize..];
    let search_len = string_length(search);
    if search_len == 0 {
        return 0;
    }
    // ARM checks the voice list only when it is non-empty, but the `T`
    // fallback applies whenever no entry matched, including an empty list.
    let mut matched = false;
    if voice.name(0).is_some() {
        let mut slot = 0usize;
        loop {
            let Some(candidate) = voice.name(slot) else {
                break;
            };
            if compare_strings(candidate, search) == 0 {
                matched = true;
                break;
            }
            slot += 1;
        }
    }
    if !matched && name[slash as usize] != b'T' {
        return 0;
    }
    if name[0] == b'/' {
        *current_voice = voice_index as u8;
        *default_voice = 2;
    } else if voice.voice_feature >= 0 {
        let mut slot = 0usize;
        for _ in 0..257 {
            match voice.names.get(slot & 0xff).copied().flatten() {
                Some(candidate) if compare_strings(candidate, &name[..]) == 0 => break,
                Some(_) => slot = (slot + 1) & 0xff,
                None => {
                    slot = voice_index as usize;
                    break;
                }
            }
        }
        *current_voice = slot as u8;
    } else {
        let mut kind = 2u8;
        if compare_strings(&name[..], b"TM\0") == 0 {
            kind = 0;
        } else if compare_strings(&name[..], b"TQ\0") == 0 {
            kind = 1;
        } else if compare_strings(&name[..], b"TS\0") != 0
            && compare_strings(&name[..], b"TW\0") == 0
        {
            kind = 3;
        }
        *default_voice = kind;
    }
    let name_len = string_length(&name[..]) as u32;
    // Logical weight `i` lives at `weights[i + 4]`; the four leading guard
    // bytes keep the original `weights[index - 1]` read in bounds.
    if let Some(weights) = weights.as_deref_mut() {
        let at = |i: i32| (i + 4) as usize;
        let mut pos = (index + 1) as i16;
        let limit = (index as u32).wrapping_add(name_len).wrapping_add(2) as i32;
        if (pos as i32) < limit {
            loop {
                let sum = weights[at(index)].wrapping_add(weights[at(pos as i32)]);
                weights[at(index)] = sum;
                pos = pos.wrapping_add(1);
                if !((index as u32).wrapping_add(name_len).wrapping_add(2) as u16 as i16 > pos) {
                    break;
                }
            }
        }
        if (size as u32).wrapping_sub(name_len).wrapping_sub(2) == index as u32 {
            let sum = weights[at(index - 1)].wrapping_add(weights[at(index)]);
            weights[at(index - 1)] = sum;
            *length = (size as u32).wrapping_sub(name_len).wrapping_sub(2) as i16;
            return 1;
        }
    }
    let mut i = index;
    if (i as u32) < (size as u32).wrapping_sub(name_len).wrapping_sub(2) {
        loop {
            text[i as usize] = text[(i as u32).wrapping_add(name_len).wrapping_add(2) as usize];
            if let Some(weights) = weights.as_deref_mut() {
                let at = |i: i32| (i + 4) as usize;
                let source = (i as u32).wrapping_add(name_len).wrapping_add(2);
                weights[at(i)] = if i != index {
                    weights[at(source as i32)]
                } else {
                    weights[at(source as i32)].wrapping_add(weights[at(i)])
                };
            }
            i += 1;
            if !((i as u32) < (size as u32).wrapping_sub(name_len).wrapping_sub(2)) {
                break;
            }
        }
    }
    *length = (size as u32).wrapping_sub(name_len).wrapping_sub(2) as i16;
    1
}

/// Caller-owned synthesis-window object shared by `fn_29527c`, `fn_29564c`
/// and `fn_2956f4` (260 bytes in the original).
///
/// `fn_295444` views the same bytes with overlapping fields: its `duration`
/// sits in the padding at +0x08, and its `samples` overwrites `limit` at
/// +0x0c. The ports below preserve the original call order, so the overlay
/// behaves identically. Pointer fields are host tokens; 0 is null. The sine
/// table itself is a separate caller slice passed alongside the object.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct WindowObject {
    pub rate: u16,
    pub period: u16,
    pub overlap: i16,
    pub duration: i32,
    pub limit: i16,
    pub divisor: i16,
    pub tail_count: i16,
    pub bound: i16,
    pub average: i16,
    pub cursor_a: i16,
    pub cursor_b: i16,
    pub length: u32,
    pub scaled: i16,
    pub spare: i16,
    pub table: u32,
    pub phase: i16,
    pub vecs: [u32; 6],
    pub vec_lens: [u32; 6],
    pub ready_a: u32,
    pub ready_b: u32,
    pub ready_c: u32,
    pub aux: u32,
    pub block: u32,
    pub table_bytes: [u8; 129],
    pub pending: u8,
    pub cursor_c: i32,
    pub cursor_d: i32,
    pub active: u8,
    pub owner: u32,
    pub heap: u32,
}

impl Default for WindowObject {
    fn default() -> Self {
        WindowObject {
            rate: 0,
            period: 0,
            overlap: 0,
            duration: 0,
            limit: 0,
            divisor: 0,
            tail_count: 0,
            bound: 0,
            average: 0,
            cursor_a: 0,
            cursor_b: 0,
            length: 0,
            scaled: 0,
            spare: 0,
            table: 0,
            phase: 0,
            vecs: [0; 6],
            vec_lens: [0; 6],
            ready_a: 0,
            ready_b: 0,
            ready_c: 0,
            aux: 0,
            block: 0,
            table_bytes: [0; 129],
            pending: 0,
            cursor_c: 0,
            cursor_d: 0,
            active: 0,
            owner: 0,
            heap: 0,
        }
    }
}

impl WindowObject {
    /// Field comparison with pointer tokens reduced to presence bits, in
    /// offset order: eleven half-words, length, three half-words, table and
    /// phase tokens, six vec tokens, three ready words, aux/block tokens, the
    /// pending byte, two cursors, the active byte and owner/heap tokens.
    /// `table_bytes` and `vec_lens` are compared separately.
    pub fn shape(&self) -> [u32; 33] {
        let present = |token: u32| u32::from(token != 0);
        [
            u32::from(self.rate),
            u32::from(self.period),
            self.overlap as u16 as u32,
            self.duration as u32,
            self.limit as u16 as u32,
            self.divisor as u16 as u32,
            self.tail_count as u16 as u32,
            self.bound as u16 as u32,
            self.average as u16 as u32,
            self.cursor_a as u16 as u32,
            self.cursor_b as u16 as u32,
            self.length,
            self.scaled as u16 as u32,
            self.spare as u16 as u32,
            present(self.table),
            self.phase as u16 as u32,
            present(self.vecs[0]),
            present(self.vecs[1]),
            present(self.vecs[2]),
            present(self.vecs[3]),
            present(self.vecs[4]),
            present(self.vecs[5]),
            self.ready_a,
            self.ready_b,
            self.ready_c,
            present(self.aux),
            present(self.block),
            u32::from(self.pending),
            self.cursor_c as u32,
            self.cursor_d as u32,
            u32::from(self.active),
            present(self.owner),
            present(self.heap),
        ]
    }
}

/// Engine calls reached by the synthesis-window lifecycle ports.
pub trait WindowHost {
    fn grow_buffer(&mut self, vec: u32, len: &mut u32, length: u32) -> bool;
    fn realloc(&mut self, heap: u32, ptr: u32, size: u32) -> u32;
    fn alloc(&mut self, heap: u32, count: u32, size: u32) -> u32;
    fn alloc_bytes(&mut self, heap: u32, size: u32) -> u32;
    fn free(&mut self, heap: u32, ptr: u32);
    fn create_buffer(&mut self, heap: u32, length: u32) -> u32;
    fn init_buffer(&mut self, vec: u32);
    fn reset_buffer(&mut self, vec: u32);
    fn destroy_buffer(&mut self, vec: u32);
    fn create_aux(&mut self, heap: u32, flag: i32) -> u32;
    fn destroy_aux(&mut self, aux: u32);
    fn fill(&mut self, ptr: u32, value: u8, count: u32);
    fn init_table(&mut self, table: &mut [u8]);
}

fn fill_sine_table(table: &mut [i16], overlap: i16) {
    let n = (overlap as i32).wrapping_mul(2);
    for (i, slot) in table.iter_mut().take(n.max(0) as usize).enumerate() {
        let phase =
            crate::runtime::signed_division((i as i32).wrapping_mul(0x3243f), overlap as i32);
        let sine = window_sine(phase.map(|division| division.quotient).unwrap_or(0));
        let mut half = 0x10000i32.wrapping_sub(sine) >> 1;
        if half >= 0x10000 {
            half = 0xffff;
        }
        *slot = half as i16;
    }
}

/// fn_29527c / EU fn_295438 / KR fn_2954e4.
///
/// Retunes the window for rate `v`, growing the six sample vectors and
/// refilling the sine `table` when present. A zero `table` token skips the
/// refill; the harness keeps the token in sync with the borrowed slice.
pub fn retune_window(
    obj: &mut WindowObject,
    v: i32,
    table: &mut [i16],
    host: &mut impl WindowHost,
) -> u32 {
    let a = obj.limit;
    let q = ((obj.rate as i32).wrapping_mul(v) / 1000) as i16;
    let q2 = q.wrapping_mul(2);
    obj.period = v as u16;
    let b = q2.wrapping_sub(q);
    obj.overlap = q;
    obj.tail_count = q2;
    obj.bound = b;
    let mut c = a;
    if a >= b {
        c = a.wrapping_mul(2).wrapping_sub(b);
    }
    let ip = (q2 as i32).wrapping_add(a as i32);
    obj.scaled = c;
    obj.length = (ip
        .wrapping_shl(1)
        .wrapping_sub(q as i32)
        .wrapping_add(c as i32)) as u32;
    if obj.vecs[0] != 0 && obj.length > obj.vec_lens[0] {
        let sizes = [
            obj.length,
            obj.length,
            obj.length >> 1,
            obj.length >> 1,
            obj.length >> 2,
            obj.length >> 2,
        ];
        for (index, size) in sizes.into_iter().enumerate() {
            if !host.grow_buffer(obj.vecs[index], &mut obj.vec_lens[index], size) {
                return 0x8100200a;
            }
        }
    }
    if obj.table == 0 {
        return 0;
    }
    let n = (obj.overlap as i32).wrapping_mul(2);
    let fresh = host.realloc(obj.heap, obj.table, n as u32);
    if fresh == 0 {
        return 0x8100200a;
    }
    obj.table = fresh;
    fill_sine_table(table, obj.overlap);
    0
}

/// fn_29564c / EU fn_295808 / KR fn_2958b4.
pub fn destroy_window(slot: &mut u32, obj: &mut WindowObject, host: &mut impl WindowHost) {
    for vec in obj.vecs {
        host.destroy_buffer(vec);
    }
    if obj.table != 0 {
        host.free(obj.heap, obj.table);
    }
    if obj.aux != 0 {
        host.destroy_aux(obj.aux);
    }
    if obj.block != 0 {
        host.free(obj.heap, obj.block);
    }
    let heap = obj.heap;
    let address = *slot;
    host.free(heap, address);
    *slot = 0;
    *obj = WindowObject::default();
}

/// fn_2956f4 / EU fn_2958b0 / KR fn_29595c.
///
/// Creates a window for rate `v`, replaying the `fn_295444` duration setup
/// inline on the same bytes. Returns the object token, or 0 on failure.
pub fn create_window(
    obj: &mut WindowObject,
    slot: &mut u32,
    owner: u32,
    heap: u32,
    v: i32,
    table: &mut [i16],
    host: &mut impl WindowHost,
) -> u32 {
    let address = host.alloc(heap, 260, 1);
    if address == 0 {
        return 0;
    }
    *obj = WindowObject::default();
    *slot = address;
    obj.owner = owner;
    obj.heap = heap;
    obj.active = 0;
    obj.rate = v as u16;
    if retune_window(obj, 10, table, host) >= 0x80000000 {
        destroy_window(slot, obj, host);
        return *slot;
    }
    if init_window_duration(obj, 4, host) >= 0x80000000 {
        destroy_window(slot, obj, host);
        return *slot;
    }
    let shifts = [0, 0, 1, 1, 2, 2];
    for (index, shift) in shifts.into_iter().enumerate() {
        let vec = host.create_buffer(heap, obj.length >> shift);
        obj.vecs[index] = vec;
        if vec == 0 {
            destroy_window(slot, obj, host);
            return *slot;
        }
    }
    for vec in obj.vecs {
        host.init_buffer(vec);
    }
    obj.vec_lens[0] = obj.length;
    obj.vec_lens[1] = obj.length;
    obj.vec_lens[2] = obj.length >> 1;
    obj.vec_lens[3] = obj.length >> 1;
    obj.vec_lens[4] = obj.length >> 2;
    obj.vec_lens[5] = obj.length >> 2;
    obj.aux = host.create_aux(heap, 0);
    if obj.aux == 0 {
        destroy_window(slot, obj, host);
        return *slot;
    }
    obj.block = host.alloc(heap, 128, 4);
    if obj.block == 0 {
        destroy_window(slot, obj, host);
        return *slot;
    }
    let n = (obj.overlap as i32).wrapping_mul(2);
    obj.table = host.alloc_bytes(heap, n.wrapping_mul(2) as u32);
    if obj.table == 0 {
        destroy_window(slot, obj, host);
        return *slot;
    }
    if n > 0 {
        fill_sine_table(table, obj.overlap);
    }
    // ARM stores the scaled quotient scratch at +0x14, not +0x20: the
    // companion duration value at +0x20 is preserved, and the spare field
    // subtracts the scratch value.
    obj.divisor = 100;
    obj.average = ((obj.bound as i32).wrapping_mul(obj.divisor as i32) / 100) as i16;
    let quotient = crate::runtime::signed_division(obj.average as i32, obj.bound as i32)
        .map(|division| division.quotient)
        .unwrap_or(0);
    obj.divisor = quotient as i16;
    obj.spare = (obj.length as i32).wrapping_sub(obj.average as i32) as i16;
    for vec in obj.vecs {
        host.reset_buffer(vec);
    }
    obj.cursor_d = -1;
    obj.cursor_c = -1;
    obj.active = 1;
    obj.ready_a = 0;
    obj.ready_c = 0;
    obj.ready_b = 0;
    obj.pending = 0;
    host.fill(obj.block, 0, 512);
    host.init_table(&mut obj.table_bytes[..128]);
    obj.cursor_a = 0;
    obj.cursor_b = 0;
    obj.phase = 0;
    address
}

/// `fn_295444` replayed on the shared window bytes for `create_window`.
/// EU fn_295600 / KR fn_2956ac.
fn init_window_duration(obj: &mut WindowObject, duration: i32, host: &mut impl WindowHost) -> u32 {
    obj.duration = duration;
    let samples = (obj.rate as i32).wrapping_mul(duration) / 1000;
    obj.limit = samples as i16;
    let mut tail = obj.bound as i32;
    if samples < tail {
        tail = samples;
    } else {
        tail = ((samples.wrapping_mul(2) as i16) as i32).wrapping_sub(tail);
    }
    obj.scaled = tail as i16;
    obj.length = (tail
        .wrapping_add((samples.wrapping_add(obj.tail_count as i32)).wrapping_mul(2))
        .wrapping_sub(obj.overlap as i32)) as u32;
    if obj.vecs[0] != 0 && obj.length > obj.vec_lens[0] {
        let sizes = [
            obj.length,
            obj.length,
            obj.length >> 1,
            obj.length >> 1,
            obj.length >> 2,
            obj.length >> 2,
        ];
        for (index, size) in sizes.into_iter().enumerate() {
            if !host.grow_buffer(obj.vecs[index], &mut obj.vec_lens[index], size) {
                return 0x8100200a;
            }
        }
    }
    0
}

/// Caller-owned phoneme-timing views for `fn_29442c`. All byte buffers are
/// NUL-terminated like the original inputs; the carrier copy scratch must
/// hold `carrier` plus its terminator.
pub struct PhonemeTiming<'a> {
    pub phonemes: &'a mut [u8],
    pub weights: Option<&'a mut [u8]>,
    pub attributes: &'a mut [u8],
    pub timings: &'a mut [i32],
    pub carrier_timings: &'a [i32],
    pub carrier: Option<&'a [u8]>,
    pub carrier_scratch: &'a mut [u8],
}

/// Engine calls reached by `fn_29442c`.
pub trait TimingHost {
    fn alloc(&mut self, heap: u32, size: u32) -> u32;
    fn free(&mut self, heap: u32, ptr: u32);
    fn logging(&mut self) -> bool;
    fn log(&mut self, code: u32);
    fn error(&mut self, request: u32, level: u32);
    fn log_error(&mut self, code: u32);
}

fn timing_strlen(bytes: &[u8]) -> usize {
    bytes.iter().position(|&b| b == 0).unwrap_or(bytes.len())
}

fn timing_strchr(bytes: &[u8], byte: u8) -> Option<usize> {
    let len = timing_strlen(bytes);
    bytes[..len].iter().position(|&b| b == byte)
}

fn timing_strstr(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    let haystack_len = timing_strlen(haystack);
    let needle_len = timing_strlen(needle);
    if needle_len == 0 {
        return Some(0);
    }
    let haystack = &haystack[..haystack_len];
    let needle = &needle[..needle_len];
    if needle.len() > haystack.len() {
        return None;
    }
    haystack
        .windows(needle.len())
        .position(|window| window == needle)
}

/// fn_29442c / EU fn_2945e8 / KR fn_294694.
///
/// Resolves tap carriers against the phoneme string, emits per-phoneme timing
/// pairs and folds consumed escapes out of the borrowed buffers.
pub fn align_timings(
    allocator: u32,
    log: u32,
    request: u32,
    classes: &[u8; 256],
    data: &mut PhonemeTiming<'_>,
    length: &mut i16,
    host: &mut impl TimingHost,
) -> u32 {
    let _ = log;
    let mut in_brackets = false;
    let mut timing_index: i16 = 0;
    let mut carrier_index: i16 = 0;
    let mut reverse_brackets = false;
    let has_carrier = data.carrier.is_some();
    let mut allocation = 0u32;
    if let Some(carrier) = data.carrier {
        let need = timing_strlen(carrier) + 1;
        allocation = host.alloc(allocator, need as u32);
        if allocation == 0 {
            host.log_error(0x4a38);
            return 0x84e0200a;
        }
        data.carrier_scratch[..need].copy_from_slice(&carrier[..need]);
    }
    if host.logging() {
        host.log(0x12d);
    }
    let mut car_start = 0usize;
    let mut i: i16 = 0;
    if *length > 0 {
        loop {
            let at = i as usize;
            if (data.phonemes[at] == 0x12 || data.phonemes[at] == 0x13)
                && data.phonemes[at + 1] == 0x10
            {
                *length -= 2;
                if let Some(weights) = data.weights.as_deref_mut() {
                    weights[at + 2] = weights[at + 2]
                        .wrapping_add(weights[at + 1])
                        .wrapping_add(weights[at]);
                }
                let remaining = (*length as i32).wrapping_sub(i as i32);
                if remaining > 0 {
                    for j in 0..remaining as usize {
                        data.phonemes[at + j] = data.phonemes[at + j + 2];
                        if let Some(weights) = data.weights.as_deref_mut() {
                            weights[at + j] = weights[at + j + 2];
                        }
                    }
                }
            }
            if data.phonemes[at] == 0x10 {
                let next = data.phonemes[at + 1];
                let skip = if next == 0x12 {
                    Some(0)
                } else if next == 0x13 {
                    Some(1)
                } else {
                    None
                };
                if let Some(skip) = skip {
                    *length -= 2;
                    if let Some(weights) = data.weights.as_deref_mut() {
                        weights[at + 2] = weights[at + 2]
                            .wrapping_add(weights[at + 1])
                            .wrapping_add(weights[at]);
                    }
                    let remaining = (*length as i32).wrapping_sub(i as i32);
                    if remaining > 0 {
                        for j in 0..remaining as usize {
                            data.phonemes[at + j] = data.phonemes[at + j + 2];
                            data.attributes[at + j] = data.attributes[at + j + 2];
                            if let Some(weights) = data.weights.as_deref_mut() {
                                weights[at + j] = weights[at + j + 2];
                            }
                        }
                    }
                    if skip == 0 {
                        if has_carrier {
                            let scratch = &mut data.carrier_scratch;
                            let found = timing_strchr(&scratch[car_start..], 0x1b);
                            let Some(rel) = found else {
                                i = i.wrapping_add(1);
                                if i < *length {
                                    continue;
                                } else {
                                    break;
                                }
                            };
                            let mut carrier_end = car_start + rel;
                            scratch[carrier_end] = 0;
                            carrier_end += 1;
                            let phoneme_end = timing_strchr(&data.phonemes[at..], 0x12);
                            let Some(end_rel) = phoneme_end else {
                                i = i.wrapping_add(1);
                                if i < *length {
                                    continue;
                                } else {
                                    break;
                                }
                            };
                            let phoneme_end = at + end_rel;
                            data.phonemes[phoneme_end] = 0;
                            if host.logging() {
                                host.log(0x12e);
                            }
                            let mut suffix_count: i16 = 0;
                            let mut prefix_count: i16 = 0;
                            let mut count: i16 = 0;
                            let carrier_len = timing_strlen(&scratch[car_start..]);
                            let mut k: u16 = 0;
                            let mut carrier_brackets = false;
                            while (k as i32) < (carrier_len as i16 as i32) {
                                let c = scratch[car_start + k as usize];
                                if c == b'[' {
                                    carrier_brackets = true;
                                } else {
                                    if c == b']' {
                                        carrier_brackets = false;
                                    }
                                    if !carrier_brackets && classes[c as usize] != 255 {
                                        count += 2;
                                    }
                                }
                                k += 1;
                            }
                            let mut matched =
                                timing_strstr(&data.phonemes[at..], &scratch[car_start..])
                                    .map(|rel| at + rel);
                            if matched.is_none() && count > 2 {
                                let original = car_start;
                                let mut cursor = car_start;
                                let first = scratch[cursor];
                                if first == b'[' {
                                    carrier_brackets = true;
                                } else if first == b']' {
                                    carrier_brackets = false;
                                }
                                while scratch[cursor] != 0
                                    && (carrier_brackets
                                        || classes[scratch[cursor] as usize] == 255)
                                {
                                    cursor += 1;
                                    if scratch[cursor] == b'[' {
                                        carrier_brackets = true;
                                    } else if scratch[cursor] == b']' {
                                        carrier_brackets = false;
                                    }
                                }
                                if scratch[cursor] != 0 && scratch[cursor + 1] != 0 {
                                    prefix_count = 2;
                                    matched =
                                        timing_strstr(&data.phonemes[at..], &scratch[cursor..])
                                            .map(|rel| at + rel);
                                    if matched.is_none() {
                                        car_start = original;
                                        prefix_count = 0;
                                        let end_byte = scratch[carrier_end];
                                        if end_byte == b']' {
                                            reverse_brackets = true;
                                        } else if end_byte == b'[' {
                                            reverse_brackets = false;
                                        }
                                        let mut tail = carrier_end;
                                        while tail != car_start
                                            && (reverse_brackets
                                                || classes[scratch[tail] as usize] == 255)
                                        {
                                            tail -= 1;
                                            if scratch[tail] == b']' {
                                                reverse_brackets = true;
                                            } else if scratch[tail] == b'[' {
                                                reverse_brackets = false;
                                            }
                                        }
                                        if tail != car_start
                                            && classes[scratch[tail] as usize] != 255
                                        {
                                            suffix_count = 2;
                                            scratch[tail] = 0;
                                            matched = timing_strstr(
                                                &data.phonemes[at..],
                                                &scratch[car_start..],
                                            )
                                            .map(|rel| at + rel);
                                            if matched.is_none() {
                                                car_start = cursor;
                                                prefix_count = 2;
                                                matched = timing_strstr(
                                                    &data.phonemes[at..],
                                                    &scratch[car_start..],
                                                )
                                                .map(|rel| at + rel);
                                            }
                                        }
                                    }
                                }
                            }
                            let matched = if let Some(matched) = matched {
                                if host.logging() {
                                    host.log(0x12e);
                                }
                                matched
                            } else {
                                host.error(request, 3);
                                host.error(request, 3);
                                host.error(request, 3);
                                prefix_count = count;
                                if host.logging() {
                                    host.log(0x12e);
                                }
                                phoneme_end
                            };
                            let mut phonemes = at;
                            let distance = (matched as i32).wrapping_sub(at as i32);
                            if distance > 0 {
                                let mut n = distance;
                                let mut scan = at + 1;
                                while n != 0 {
                                    if !in_brackets
                                        && classes[data.phonemes[phonemes] as usize] != 255
                                    {
                                        timing_index = timing_index.wrapping_add(2);
                                    }
                                    phonemes += 1;
                                    if data.phonemes[scan] == b'[' {
                                        in_brackets = true;
                                    } else if data.phonemes[scan] == b']' {
                                        in_brackets = false;
                                    }
                                    n -= 1;
                                    scan += 1;
                                }
                                i = i.wrapping_add(distance as i16);
                            }
                            let mut reference_index = carrier_index.wrapping_add(prefix_count);
                            if matched != phoneme_end {
                                let mut end_connected = false;
                                let mut start_connected = false;
                                let mut previous = i.wrapping_sub(1);
                                let mut scan = phoneme_end + 1;
                                while data.phonemes[scan] != 0 {
                                    if classes[data.phonemes[scan] as usize] != 255 {
                                        if data.phonemes[scan] != b'#' {
                                            end_connected = true;
                                        }
                                        break;
                                    }
                                    scan += 1;
                                }
                                let limit = (previous as i32).wrapping_add(2);
                                let mut n = 1;
                                while n < limit {
                                    if classes[data.phonemes[previous as usize] as usize] != 255 {
                                        if data.phonemes[previous as usize] != b'#' {
                                            start_connected = true;
                                        }
                                        break;
                                    }
                                    previous = previous.wrapping_sub(1);
                                    n += 1;
                                }
                                let copy_len = timing_strlen(&scratch[car_start..]);
                                let mut copied = 0;
                                while copied < copy_len as i16 {
                                    if !in_brackets
                                        && classes[data.phonemes[phonemes] as usize] != 255
                                    {
                                        if !start_connected {
                                            data.timings[timing_index as usize] =
                                                data.carrier_timings[reference_index as usize];
                                        } else {
                                            start_connected = false;
                                        }
                                        timing_index = timing_index.wrapping_add(1);
                                        reference_index = reference_index.wrapping_add(1);
                                        data.timings[timing_index as usize] =
                                            data.carrier_timings[reference_index as usize];
                                        timing_index = timing_index.wrapping_add(1);
                                        reference_index = reference_index.wrapping_add(1);
                                    }
                                    phonemes += 1;
                                    i = i.wrapping_add(1);
                                    if data.phonemes[i as usize] == b'[' {
                                        in_brackets = true;
                                    } else if data.phonemes[i as usize] == b']' {
                                        in_brackets = false;
                                    }
                                    copied += 1;
                                }
                                if end_connected {
                                    data.timings[timing_index.wrapping_sub(1) as usize] = -1;
                                }
                            }
                            let distance = (phoneme_end as i32).wrapping_sub(phonemes as i32);
                            if distance > 0 {
                                let mut n = distance;
                                let mut scan = (i as i32).wrapping_add(1) as usize;
                                while n != 0 {
                                    if !in_brackets
                                        && classes[data.phonemes[phonemes] as usize] != 255
                                    {
                                        timing_index = timing_index.wrapping_add(2);
                                    }
                                    phonemes += 1;
                                    if data.phonemes[scan] == b'[' {
                                        in_brackets = true;
                                    } else if data.phonemes[scan] == b']' {
                                        in_brackets = false;
                                    }
                                    n -= 1;
                                    scan += 1;
                                }
                                i = i.wrapping_add(distance as i16);
                            }
                            carrier_index = reference_index.wrapping_add(suffix_count);
                            car_start = carrier_end;
                            *length -= 2;
                            if let Some(weights) = data.weights.as_deref_mut() {
                                let at = i as usize;
                                weights[at + 2] = weights[at + 2]
                                    .wrapping_add(weights[at + 1])
                                    .wrapping_add(weights[at]);
                            }
                            let remaining = (*length as i32).wrapping_sub(i as i32);
                            if remaining > 0 {
                                for j in 0..remaining as usize {
                                    let at = i as usize;
                                    data.phonemes[at + j] = data.phonemes[at + j + 2];
                                    data.attributes[at + j] = data.attributes[at + j + 2];
                                    if let Some(weights) = data.weights.as_deref_mut() {
                                        weights[at + j] = weights[at + j + 2];
                                    }
                                }
                            }
                        }
                    } else {
                        while (i as i32) < (*length as i32) && data.phonemes[i as usize] != 0x13 {
                            if !in_brackets && classes[data.phonemes[i as usize] as usize] != 255 {
                                data.timings[timing_index as usize] = -2;
                                timing_index = timing_index.wrapping_add(1);
                                data.timings[timing_index as usize] = -2;
                                timing_index = timing_index.wrapping_add(1);
                            }
                            i = i.wrapping_add(1);
                            if data.phonemes[i as usize] == b'[' {
                                in_brackets = true;
                            } else if data.phonemes[i as usize] == b']' {
                                in_brackets = false;
                            }
                        }
                        if i != *length {
                            *length -= 2;
                            if let Some(weights) = data.weights.as_deref_mut() {
                                let at = i as usize;
                                weights[at + 2] = weights[at + 2]
                                    .wrapping_add(weights[at + 1])
                                    .wrapping_add(weights[at]);
                            }
                            let remaining = (*length as i32).wrapping_sub(i as i32);
                            if remaining > 0 {
                                for j in 0..remaining as usize {
                                    let at = i as usize;
                                    data.phonemes[at + j] = data.phonemes[at + j + 2];
                                    data.attributes[at + j] = data.attributes[at + j + 2];
                                    if let Some(weights) = data.weights.as_deref_mut() {
                                        weights[at + j] = weights[at + j + 2];
                                    }
                                }
                            }
                        }
                    }
                }
            }
            if data.phonemes[i as usize] == b'[' {
                in_brackets = true;
            } else {
                if data.phonemes[i as usize] == b']' {
                    in_brackets = false;
                }
                if !in_brackets && classes[data.phonemes[i as usize] as usize] != 255 {
                    timing_index = timing_index.wrapping_add(2);
                }
            }
            i = i.wrapping_add(1);
            if i < *length {
                continue;
            } else {
                break;
            }
        }
    }
    if allocation != 0 {
        host.free(allocator, allocation);
    }
    if host.logging() {
        host.log(0x12d);
    }
    0
}

/// fn_1dea78 / EU fn_1dec34 / KR fn_1dece0.
pub fn destroy_phrasing(
    present: bool,
    tag: u32,
    mut release: impl FnMut(ProsodyResource),
    destroy: impl FnOnce(),
) -> u32 {
    if !present || tag != 0xf387 {
        return 0x8a002008;
    }
    release(ProsodyResource::Linguistic);
    release(ProsodyResource::Rules);
    release(ProsodyResource::Dictionary);
    destroy();
    0
}

/// fn_1f4d94 / EU fn_1f4f50 / KR fn_1f4ffc.
pub fn destroy_global(
    present: bool,
    tag: u32,
    mut release: impl FnMut(ProsodyResource),
    destroy: impl FnOnce(),
) -> u32 {
    if !present || tag != 0xf388 {
        return 0x8a102008;
    }
    release(ProsodyResource::Linguistic);
    release(ProsodyResource::Rules);
    release(ProsodyResource::Dictionary);
    destroy();
    0
}

/// One pronunciation-view mark consumed by `process_phrasing`.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct PhrasingMark {
    pub r#type: u16,
    pub position: u32,
    pub value: u32,
}

/// Caller-owned scratch buffers for `process_phrasing` (`fn_1deb20`) and
/// `process_phrasing_stage` (`fn_1df780`).
///
/// Every slice stands in for one C heap/LINGDB buffer; the harness sizes each
/// one to the recorded case: `text`/`map` hold `capacity` bytes, `input` the
/// pronunciation buffer, `marks` at least one slot (slot 0 is read even when
/// the recorded count is 0, matching the C), `mode`/`next_mode`/merge slots
/// the recorded NUL-terminated strings, and `output`/`flags` the recorded
/// engine outputs plus one NUL byte. `input_prefix` is the byte before
/// `input[0]` (`input[-1]` in C, read when a word starts at offset 0).
pub struct PhrasingBuffers<'a> {
    pub text: &'a mut [u8],
    pub map: &'a mut [u8],
    pub input: &'a mut [u8],
    pub input_prefix: u8,
    pub marks: &'a mut [PhrasingMark],
    pub mode: &'a mut [u8],
    pub next_mode: &'a mut [u8],
    pub merge_next: &'a mut [u8],
    pub merge_current: &'a mut [u8],
    pub output: &'a mut [u8],
    pub flags: &'a mut [u8],
}

/// Node/engine/dictionary/config calls reached by `process_phrasing`
/// (`fn_1deb20`) and `process_phrasing_stage` (`fn_1df780`).
///
/// Field values cross as raw `u32`; the caller truncates to the C width.
/// `node_data` lends word strings for one loop iteration only; every longer
/// lived string is copied out into `PhrasingBuffers` (`root_text`,
/// `root_marks`, `mode_text`, `merge_param`) because the C keeps it across
/// later calls. String arguments cross without the NUL terminator. Lengths
/// that the C ignores are still returned for the harness.
/// The `env` argument is the raw environment word; the host resolves
/// heap/log/config from the environment block. Log stages are 0/1/2/3 for the
/// `L1`/`L2`/`O1`/`O3` calls; `merge_param` always queries section `fecfg`.
pub trait PhrasingProcessHost {
    fn next_node(&mut self, handle: [u32; 2], node: u16) -> (u32, u16);
    fn previous_node(&mut self, handle: [u32; 2], node: u16) -> (u32, u16);
    fn first_node(&mut self, handle: [u32; 2], kind: i32, sub: u32) -> (u32, u16);
    fn node_field(
        &mut self,
        handle: [u32; 2],
        node: u16,
        field: i32,
        count: i32,
    ) -> (u32, u32, u16);
    fn set_node_field(
        &mut self,
        handle: [u32; 2],
        node: u16,
        field: i32,
        count: u16,
        value: u32,
    ) -> (u32, u16);
    fn node_data(&mut self, handle: [u32; 2], node: u16, field: i32) -> (u32, &[u8], u16);
    fn node_count(&mut self, handle: [u32; 2], node: u16, field: i32) -> (u32, u16);
    fn root_text(&mut self, handle: [u32; 2], node: u16, out: &mut [u8]) -> (u32, u16);
    fn root_marks(&mut self, handle: [u32; 2], node: u16, out: &mut [PhrasingMark]) -> (u32, u16);
    fn mode_text(&mut self, handle: [u32; 2], node: u16, out: &mut [u8]) -> (u32, u16);
    fn merge_param(
        &mut self,
        dictionary: [u32; 2],
        key: &[u8],
        out: &mut [u8],
    ) -> (u32, u16, u8, usize);
    fn pause_weights(&mut self, env: u32, key: &[u8]) -> (u32, u32);
    fn set_engine_input(&mut self, handle: [u32; 2], index: i32, text: &[u8]) -> u32;
    fn engine_output(&mut self, handle: [u32; 2], index: i32, buf: &mut [u8]) -> (u32, usize);
    fn run_engine(&mut self, handle: [u32; 2], key: &[u8]) -> u32;
    fn prepare_dictionary(&mut self, handle: [u32; 2], mode: &[u8], text: &mut [u8]) -> u32;
    fn log_engine(&mut self, env: u32, stage: u8, key: &[u8], text: &[u8]);
    fn log_nomem(&mut self, env: u32);
    fn validate(&mut self, object: [u32; 2], tag: u32, size: u32) -> u32;
    fn alloc_text(&mut self, env: u32, capacity: u16) -> u32;
    fn free_text(&mut self, env: u32, addr: u32);
    /// Commit one input-text byte back to the guest arena (the ARM rewrites
    /// the lattice text in place, e.g. the phrasing-mode `?` -> `.`).
    /// Default no-op; the live lattice host overrides it.
    fn commit_input_byte(&mut self, _offset: u32, _value: u8) {}
}

fn c_strlen(bytes: &[u8]) -> usize {
    bytes.iter().position(|&b| b == 0).unwrap_or(bytes.len())
}

/// Length of the NUL-terminated string at the start of `bytes`.
///
/// Panics when unterminated: the C reads until NUL in real memory, so a
/// missing terminator is a harness contract violation, never valid input.
fn c_str_len(bytes: &[u8], what: &str) -> usize {
    let len = c_strlen(bytes);
    assert!(len < bytes.len(), "{what} is not NUL-terminated");
    len
}

/// `fn_195d58`: compare until the first difference or NUL, clamped to -1/0/1.
fn c_strcmp(a: &[u8], b: &[u8]) -> i32 {
    let mut i = 0;
    loop {
        let x = a.get(i).copied().unwrap_or(0);
        let y = b.get(i).copied().unwrap_or(0);
        let d = i32::from(x) - i32::from(y);
        if d != 0 || y == 0 {
            return d.clamp(-1, 1);
        }
        i += 1;
    }
}

/// `fn_1960a8` with a non-empty needle: true when `needle` occurs in `hay`.
fn c_strstr(hay: &[u8], needle: &[u8]) -> bool {
    let hlen = c_strlen(hay);
    let nlen = c_strlen(needle);
    if nlen == 0 {
        return true;
    }
    if nlen > hlen {
        return false;
    }
    (0..=hlen - nlen).any(|i| hay[i..i + nlen] == needle[..nlen])
}

/// `fn_195d30`: position of `c` in `bytes[..len]`; `c == 0` matches at `len`.
fn c_strchr(bytes: &[u8], len: usize, c: u8) -> Option<usize> {
    if c == 0 {
        return Some(len);
    }
    bytes[..len].iter().position(|&b| b == c)
}

/// `fn_195cf8`: last position of `c` in `bytes[..len]`.
fn c_strrchr(bytes: &[u8], len: usize, c: u8) -> Option<usize> {
    bytes[..len].iter().rposition(|&b| b == c)
}

/// `fn_17c550`: skip bytes 9-13 and 32, optional sign, then decimal digits.
///
/// The skip/digit classes are the `D_8dc30c` masks verified from the ROM
/// (bit 8 set exactly on 9, 10, 11, 12, 13, 32; bit 4 exactly on digits).
/// Overflow wraps like the ARM.
fn phrasing_atoi(bytes: &[u8]) -> i32 {
    let mut i = 0;
    while i < bytes.len() && matches!(bytes[i], 9 | 10 | 11 | 12 | 13 | 32) {
        i += 1;
    }
    let mut n: i32 = 0;
    let mut c = bytes.get(i).copied().unwrap_or(0);
    i += 1;
    let sign = c;
    if c == b'-' || c == b'+' {
        c = bytes.get(i).copied().unwrap_or(0);
        i += 1;
    }
    while c.is_ascii_digit() {
        n = (i32::from(c))
            .wrapping_add(n.wrapping_mul(10))
            .wrapping_sub(i32::from(b'0'));
        c = bytes.get(i).copied().unwrap_or(0);
        i += 1;
    }
    if sign == b'-' {
        n.wrapping_neg()
    } else {
        n
    }
}

fn push_byte(buf: &mut [u8], len: &mut usize, b: u8) {
    buf[*len] = b;
    *len += 1;
}

fn push_bytes(buf: &mut [u8], len: &mut usize, src: &[u8]) {
    buf[*len..*len + src.len()].copy_from_slice(src);
    *len += src.len();
}

fn out_byte(buf: &[u8], pos: usize) -> u8 {
    buf.get(pos).copied().unwrap_or(0)
}

/// Parses a `#...` pause at `output[*pos]`, consuming `[digits]` when present.
///
/// Mirrors both `fn_1deb20` pause sites: `pause_duration == 0` parses the
/// bracket text, otherwise the low 16 bits are the pause.
fn parse_pause(output: &[u8], pos: &mut usize, pause_duration: u32) -> i16 {
    *pos += 1;
    let mut text = [0u8; 12];
    if out_byte(output, *pos) == b'[' {
        *pos += 1;
        let mut i = 0;
        while i < 10 {
            let c = out_byte(output, *pos);
            if c == 0 || c == b']' {
                break;
            }
            text[i] = c;
            *pos += 1;
            i += 1;
        }
        text[i] = 0;
    }
    if pause_duration == 0 {
        phrasing_atoi(&text) as i16
    } else {
        (pause_duration & 0xffff) as u16 as i16
    }
}

/// fn_1deb20 / EU fn_1decdc / KR fn_1ded88.
///
/// Runs one FE_PHRASING mode span: builds the word text/map from the
/// pronunciation nodes, runs the phrasing engine, and writes phrase pause
/// fields back. `buffers.text`/`map` are the caller word buffers;
/// `buffers.mode` holds the NUL-terminated mode; `pause_duration` is the raw
/// `u32` weights word (`0` selects `[pause]` parsing, otherwise its low 16
/// bits are the pause). `node`/`mark_index` are in/out cursors; `last_node`
/// receives the final word node.
#[allow(clippy::too_many_arguments)]
pub fn process_phrasing(
    phrasing: &Phrasing,
    handle: [u32; 2],
    limit: u32,
    buffers: &mut PhrasingBuffers<'_>,
    node: &mut u16,
    mark_index: &mut u16,
    mark_count: u16,
    base: u32,
    pause_duration: u32,
    last_node: &mut u16,
    host: &mut impl PhrasingProcessHost,
) -> u32 {
    let mut key = [0u8; 128];
    key[..8].copy_from_slice(b"phrasing");
    let key_length = 8usize;
    let mut key_len = 8usize;
    let mode_len = c_str_len(buffers.mode, "mode");
    buffers.text[0] = 0;
    buffers.map[0] = 0;
    let mut text_len = 0usize;
    let mut map_len = 0usize;
    let (mut result, raw, _) = host.node_field(handle, *node, 1, 1);
    if result >= 0x80000000 {
        return result;
    }
    let mut cursor = *node;
    let mut start = raw as u16;
    let first_start = start;
    let mut first = true;
    while cursor != 0 && u32::from(start) < limit {
        let (status, raw, _) = host.node_field(handle, cursor, 0, 1);
        result = status;
        if result >= 0x80000000 {
            return result;
        }
        if raw as u8 == 4 || raw as u8 == 9 || raw as u8 == 10 {
            let (status, raw, _) = host.node_field(handle, cursor, 2, 1);
            result = status;
            if result >= 0x80000000 {
                return result;
            }
            let end = raw as u16;
            let (status, word, _) = host.node_data(handle, cursor, 5);
            result = status;
            if result >= 0x80000000 {
                return result;
            }
            let word_len = c_str_len(word, "word");
            if !first {
                let sep = if start == 0 {
                    buffers.input_prefix
                } else {
                    buffers.input[usize::from(start) - 1]
                };
                push_byte(buffers.text, &mut text_len, sep);
                push_byte(buffers.map, &mut map_len, b' ');
            }
            first = false;
            if start > end {
                return 0x8a003eb1;
            }
            push_bytes(
                buffers.text,
                &mut text_len,
                &buffers.input[usize::from(start)..usize::from(end)],
            );
            push_bytes(buffers.map, &mut map_len, &word[..word_len]);
            let span = (i32::from(end) - i32::from(start)) as i16;
            let want = (word_len as u32) as i16;
            if span < want {
                let mut bound = (word_len as u32)
                    .wrapping_sub(u32::from(end))
                    .wrapping_add(u32::from(start));
                while bound > 0 {
                    push_byte(buffers.text, &mut text_len, b'~');
                    bound -= 1;
                }
            } else if span > want {
                let mut bound =
                    ((i32::from(end) - i32::from(start)) as u32).wrapping_sub(word_len as u32);
                while bound > 0 {
                    push_byte(buffers.map, &mut map_len, b'~');
                    bound -= 1;
                }
            }
        }
        let (status, next) = host.next_node(handle, cursor);
        result = status;
        if result >= 0x80000000 {
            return result;
        }
        cursor = next;
        if cursor == 0 {
            break;
        }
        let (status, raw, _) = host.node_field(handle, cursor, 1, 1);
        result = status;
        if result >= 0x80000000 {
            return result;
        }
        start = raw as u16;
    }
    result = host.prepare_dictionary(phrasing.dct, &buffers.mode[..mode_len], buffers.text);
    if result >= 0x80000000 {
        return result;
    }
    text_len = c_str_len(buffers.text, "prepared text");
    host.log_engine(phrasing.env, 0, &key[..key_len], &buffers.text[..text_len]);
    result = host.set_engine_input(phrasing.depes, 0, &buffers.text[..text_len]);
    if result >= 0x80000000 {
        return result;
    }
    map_len = c_str_len(buffers.map, "map");
    host.log_engine(phrasing.env, 1, &key[..key_len], &buffers.map[..map_len]);
    result = host.set_engine_input(phrasing.depes, 1, &buffers.map[..map_len]);
    if result >= 0x80000000 {
        return result;
    }
    if c_strcmp(&buffers.mode[..mode_len], b"normal") != 0
        && (mode_len as u32)
            .wrapping_add(key_length as u32)
            .wrapping_add(2)
            < 128
    {
        key[key_len] = b'_';
        key_len += 1;
        key[key_len..key_len + mode_len].copy_from_slice(&buffers.mode[..mode_len]);
        key_len += mode_len;
        key[key_len] = 0;
    }
    result = host.run_engine(phrasing.depes, &key[..key_len]);
    if result >= 0x80000000 {
        key[key_length] = 0;
        key_len = key_length;
        let _ = host.run_engine(phrasing.depes, &key[..key_len]);
    }
    let (status, out_len) = host.engine_output(phrasing.depes, 0, buffers.output);
    result = status;
    if result >= 0x80000000 {
        return result;
    }
    buffers.output[out_len] = 0;
    let out_end = c_strlen(&buffers.output[..=out_len]);
    host.log_engine(phrasing.env, 2, &key[..key_len], &buffers.output[..out_end]);
    let (status, flag_len) = host.engine_output(phrasing.depes, 2, buffers.flags);
    result = status;
    if result >= 0x80000000 {
        return result;
    }
    buffers.flags[flag_len] = 0;
    let flag_end = c_strlen(&buffers.flags[..=flag_len]);
    host.log_engine(phrasing.env, 3, &key[..key_len], &buffers.flags[..flag_end]);
    if first_start == 0 && c_strchr(buffers.flags, flag_end, b'W').is_some() {
        let input_end = c_str_len(buffers.input, "input");
        if let Some(question) = c_strrchr(buffers.input, input_end, b'?') {
            if question != 0 && buffers.input[question - 1] != b' ' {
                buffers.input[question] = b'.';
                // The ARM rewrites the lattice text in place: commit the
                // byte to the guest arena (seg5 r15: the missing write
                // left '?' where the ARM has '.').
                host.commit_input_byte(question as u32, b'.');
            }
        }
    }
    let (status, raw, _) = host.node_field(handle, *node, 1, 1);
    result = status;
    if result >= 0x80000000 {
        return result;
    }
    start = raw as u16;
    let (status, mut previous) = host.previous_node(handle, *node);
    result = status;
    if result >= 0x80000000 {
        return result;
    }
    let mut node_type = 0u8;
    if previous != 0 {
        let (status, raw, _) = host.node_field(handle, previous, 0, 1);
        result = status;
        if result >= 0x80000000 {
            return result;
        }
        node_type = raw as u8;
    }
    while node_type != 4 && node_type != 10 && node_type != 9 && previous != 0 {
        let (status, prev) = host.previous_node(handle, previous);
        result = status;
        if result >= 0x80000000 {
            return result;
        }
        previous = prev;
        if previous != 0 {
            let (status, raw, _) = host.node_field(handle, previous, 0, 1);
            result = status;
            if result >= 0x80000000 {
                return result;
            }
            node_type = raw as u8;
        }
    }
    let mut o = 0usize;
    if out_byte(buffers.output, 0) == b'#' {
        let pause = parse_pause(buffers.output, &mut o, pause_duration);
        if previous != 0 {
            let (status, _) = host.set_node_field(handle, previous, 8, 1, pause as u16 as u32);
            result = status;
            if result >= 0x80000000 {
                return result;
            }
        }
    }
    loop {
        if *node == 0 || u32::from(start) >= limit {
            break;
        }
        let (status, raw, _) = host.node_field(handle, *node, 0, 1);
        result = status;
        if result >= 0x80000000 {
            return result;
        }
        node_type = raw as u8;
        if node_type == 4 || node_type == 9 || node_type == 10 {
            let (status, raw, _) = host.node_field(handle, *node, 1, 1);
            result = status;
            if result >= 0x80000000 {
                return result;
            }
            start = raw as u16;
            let (status, raw, _) = host.node_field(handle, *node, 2, 1);
            result = status;
            if result >= 0x80000000 {
                return result;
            }
            let end = raw as u16;
            while !matches!(out_byte(buffers.output, o), 0 | b'~' | b' ' | b'_' | b'#') {
                o += 1;
            }
            if out_byte(buffers.output, o) == b'#' {
                let pause = parse_pause(buffers.output, &mut o, pause_duration);
                let (status, _) = host.set_node_field(handle, *node, 8, 1, pause as u16 as u32);
                result = status;
                if result >= 0x80000000 {
                    return result;
                }
            } else {
                let mut pause = 0i16;
                let (status, n) = host.node_count(handle, *node, 8);
                result = status;
                if result < 0x80000000 && n == 1 {
                    let (status, raw, _) = host.node_field(handle, *node, 8, 1);
                    result = status;
                    if result >= 0x80000000 {
                        return result;
                    }
                    pause = raw as u16 as i16;
                }
                if pause == 0 {
                    let (status, _) = host.set_node_field(handle, *node, 8, 1, 0);
                    result = status;
                    if result >= 0x80000000 {
                        return result;
                    }
                }
            }
            'skip: loop {
                let c = out_byte(buffers.output, o);
                if c == 0 || !(c == b'~' || c == b' ' || c == b'_' || c == b']') {
                    break 'skip;
                }
                loop {
                    o += 1;
                    if out_byte(buffers.output, o) != b'#' {
                        break;
                    }
                    loop {
                        o += 1;
                        let d = out_byte(buffers.output, o);
                        if d == 0 {
                            break 'skip;
                        }
                        if d == b' ' || d == b'_' {
                            break;
                        }
                    }
                }
            }
            let adjustment = (((i32::from(end) - i32::from(start)) as i16) > 1) as i32;
            let mut pause = 0i16;
            while *mark_index < mark_count
                && ((buffers.marks[usize::from(*mark_index)]
                    .position
                    .wrapping_sub(base)) as i16)
                    < ((i32::from(end) - adjustment) as i16)
            {
                let mark = buffers.marks[usize::from(*mark_index)];
                if mark.r#type == 8 && previous != 0 {
                    pause = ((pause as i32 as u32).wrapping_add(mark.value)) as i16;
                    let (status, _) =
                        host.set_node_field(handle, previous, 8, 1, pause as u16 as u32);
                    result = status;
                    if result >= 0x80000000 {
                        return result;
                    }
                }
                *mark_index = mark_index.wrapping_add(1);
            }
            previous = *node;
        }
        let (status, next) = host.next_node(handle, *node);
        result = status;
        if result >= 0x80000000 {
            return result;
        }
        *node = next;
        if *node == 0 {
            break;
        }
        let (status, raw, _) = host.node_field(handle, *node, 1, 1);
        result = status;
        if result >= 0x80000000 {
            return result;
        }
        start = raw as u16;
    }
    *last_node = previous;
    result
}

/// Merges adjacent `fn_1df780` mode spans with compatible phrasing rules.
///
/// Walks the mode nodes after `start`, looking up `phrasing_<mode>` rules for
/// the next and current modes; while they agree, `out_end` advances to the
/// next span end. Both value truncations use the second lookup's separator (a
/// single C local). Over-long modes fall back to `normal` (`D_1df298`, read as
/// `normal` from all three ROMs).
#[allow(clippy::too_many_arguments)]
fn merge_modes(
    phrasing: &Phrasing,
    handle: [u32; 2],
    start: u16,
    current_mode: &[u8],
    next_mode_buf: &mut [u8],
    merge_next: &mut [u8],
    merge_current: &mut [u8],
    out_end: &mut u16,
    host: &mut impl PhrasingProcessHost,
) -> u32 {
    let mut key = [0u8; 128];
    let (mut result, mut next) = host.next_node(handle, start);
    if result < 0x80000000 && next != 0 {
        loop {
            let (status, raw, _) = host.node_field(handle, next, 0, 1);
            result = status;
            if result >= 0x80000000 {
                break;
            }
            if raw as u8 == 5 {
                let (status, _) = host.mode_text(handle, next, next_mode_buf);
                result = status;
                if result >= 0x80000000 {
                    break;
                }
                let next_len = c_str_len(next_mode_buf, "next mode");
                let mut key_len = 9usize;
                key[..9].copy_from_slice(b"phrasing_");
                let frag: &[u8] = if (next_len as u32).wrapping_add(11) < 128 {
                    &next_mode_buf[..next_len]
                } else {
                    b"normal"
                };
                key[key_len..key_len + frag.len()].copy_from_slice(frag);
                key_len += frag.len();
                key[key_len] = 0;
                let (status, next_found, _, next_value_len) =
                    host.merge_param(phrasing.dct, &key[..key_len], merge_next);
                result = status;
                if result >= 0x80000000 {
                    break;
                }
                assert!(
                    next_value_len < merge_next.len() && merge_next[next_value_len] == 0,
                    "merge value is not NUL-terminated"
                );
                key_len = 9;
                key[..9].copy_from_slice(b"phrasing_");
                let frag: &[u8] = if (current_mode.len() as u32).wrapping_add(11) < 128 {
                    current_mode
                } else {
                    b"normal"
                };
                key[key_len..key_len + frag.len()].copy_from_slice(frag);
                key_len += frag.len();
                key[key_len] = 0;
                let (status, current_found, separator, value_len) =
                    host.merge_param(phrasing.dct, &key[..key_len], merge_current);
                result = status;
                if result >= 0x80000000 {
                    break;
                }
                assert!(
                    value_len < merge_current.len() && merge_current[value_len] == 0,
                    "merge value is not NUL-terminated"
                );
                let mut next_end = next_value_len;
                if next_found != 0 {
                    if let Some(i) = c_strchr(merge_next, next_value_len, separator) {
                        next_end = i;
                    }
                }
                let mut current_end = value_len;
                if current_found != 0 {
                    if let Some(i) = c_strchr(merge_current, value_len, separator) {
                        current_end = i;
                    }
                }
                let next_mode = &next_mode_buf[..next_len];
                if c_strcmp(current_mode, next_mode) == 0
                    || (next_found == 1
                        && current_found == 0
                        && c_strcmp(current_mode, &merge_next[..next_end]) == 0)
                    || (current_found == 1
                        && next_found == 0
                        && c_strcmp(next_mode, &merge_current[..current_end]) == 0)
                    || (next_found == 1
                        && current_found == 1
                        && c_strcmp(&merge_next[..next_end], &merge_current[..current_end]) == 0)
                {
                    let (status, raw, _) = host.node_field(handle, next, 2, 1);
                    result = status;
                    if result >= 0x80000000 {
                        break;
                    }
                    *out_end = raw as u16;
                } else {
                    break;
                }
            }
            let (status, n) = host.next_node(handle, next);
            result = status;
            if result >= 0x80000000 {
                break;
            }
            next = n;
            if next == 0 {
                break;
            }
        }
    }
    result
}

/// fn_1df780 / EU fn_1df93c / KR fn_1df9e8.
///
/// FE_PHRASING stage Process entry: validates the module object, sizes the
/// word buffers, then walks the mode spans calling `process_phrasing` and
/// copying word pauses. `object` is the module handle, `handle` the LINGDB
/// handle; `active` is set to 1. Text/map addresses land in
/// `phrasing.z32`/`z36` (0 after cleanup; left allocated on the two direct
/// error returns, matching the C).
pub fn process_phrasing_stage(
    phrasing: &mut Phrasing,
    object: [u32; 2],
    handle: [u32; 2],
    active: &mut u8,
    buffers: &mut PhrasingBuffers<'_>,
    host: &mut impl PhrasingProcessHost,
) -> u32 {
    if host.validate(object, 0xf387, 40) >= 0x80000000 {
        return 0x8a002008;
    }
    *active = 1;
    let (status, root) = host.first_node(handle, 1, 0);
    if status >= 0x80000000 {
        return status;
    }
    let first_status = status;
    let (count_status, mut count) = host.node_count(handle, root, 0);
    if count_status >= 0x80000000 || count != 1 {
        return first_status;
    }
    let (status, input_len) = host.root_text(handle, root, buffers.input);
    if status >= 0x80000000 || input_len <= 1 {
        return status;
    }
    let (status, mark_count) = host.root_marks(handle, root, buffers.marks);
    if status >= 0x80000000 {
        return status;
    }
    let base = buffers.marks[0].position;
    let result = 'cleanup: loop {
        let (status, n) = host.first_node(handle, 2, u32::from(root));
        if status >= 0x80000000 {
            break 'cleanup status;
        }
        let mut node = n;
        let mut capacity = 1u16;
        let mut first = true;
        if node != 0 {
            loop {
                let (status, raw, _) = host.node_field(handle, node, 0, 1);
                if status >= 0x80000000 {
                    break 'cleanup status;
                }
                let node_type = raw as u8;
                if node_type == 4 || node_type == 9 || node_type == 10 {
                    if !first {
                        capacity = capacity.wrapping_add(1);
                    }
                    first = false;
                    let (status, raw, _) = host.node_field(handle, node, 1, 1);
                    if status >= 0x80000000 {
                        break 'cleanup status;
                    }
                    let start = raw as u16;
                    let (status, raw, _) = host.node_field(handle, node, 2, 1);
                    if status >= 0x80000000 {
                        break 'cleanup status;
                    }
                    let end = raw as u16;
                    let (status, word, _) = host.node_data(handle, node, 5);
                    if status >= 0x80000000 {
                        break 'cleanup status;
                    }
                    if end < start {
                        break 'cleanup 0x8a003eb1;
                    }
                    let word_len = c_str_len(word, "capacity word");
                    let span = (i32::from(end) - i32::from(start)) as i16;
                    let want = (word_len as u32) as i16;
                    let add = if span < want { want } else { span };
                    capacity = capacity.wrapping_add(add as u16);
                }
                let (status, n) = host.next_node(handle, node);
                if status >= 0x80000000 {
                    break 'cleanup status;
                }
                node = n;
                if node == 0 {
                    break;
                }
            }
        }
        let text_addr = host.alloc_text(phrasing.env, capacity);
        phrasing.z32 = text_addr;
        if text_addr == 0 {
            host.log_nomem(phrasing.env);
            break 'cleanup 0x8a00200a;
        }
        buffers.text[0] = 0;
        let map_addr = host.alloc_text(phrasing.env, capacity);
        phrasing.z36 = map_addr;
        if map_addr == 0 {
            host.log_nomem(phrasing.env);
            break 'cleanup 0x8a00200a;
        }
        buffers.map[0] = 0;
        let (status, n) = host.first_node(handle, 2, u32::from(root));
        if status >= 0x80000000 {
            break 'cleanup status;
        }
        node = n;
        let mut cursor = node;
        let mut result = status;
        let mut last_mode = 0u16;
        let mut mark_index = 0u16;
        let mut previous = 0u16;
        while node != 0 && cursor != 0 {
            let (status, raw, _) = host.node_field(handle, cursor, 0, 1);
            result = status;
            if result >= 0x80000000 {
                break 'cleanup result;
            }
            if raw as u8 == 5 {
                if cursor == last_mode {
                    let (status, c) = host.next_node(handle, cursor);
                    result = status;
                    if result >= 0x80000000 {
                        break 'cleanup result;
                    }
                    cursor = c;
                    continue;
                }
                last_mode = cursor;
                let (status, _) = host.mode_text(handle, cursor, buffers.mode);
                result = status;
                if result >= 0x80000000 {
                    break 'cleanup result;
                }
                let mode_len = c_str_len(buffers.mode, "mode");
                if c_strcmp(&buffers.mode[..mode_len], b"phon") == 0 {
                    let (status, c) = host.next_node(handle, cursor);
                    result = status;
                    if result >= 0x80000000 {
                        break 'cleanup result;
                    }
                    cursor = c;
                    continue;
                }
                let (status, _, _) = host.node_field(handle, cursor, 1, 1);
                result = status;
                if result >= 0x80000000 {
                    break 'cleanup result;
                }
                let (status, raw, _) = host.node_field(handle, cursor, 2, 1);
                result = status;
                if result >= 0x80000000 {
                    break 'cleanup result;
                }
                let mut end = raw as u16;
                result = merge_modes(
                    phrasing,
                    handle,
                    cursor,
                    &buffers.mode[..mode_len],
                    buffers.next_mode,
                    buffers.merge_next,
                    buffers.merge_current,
                    &mut end,
                    host,
                );
                if result >= 0x80000000 {
                    break 'cleanup result;
                }
                let mut weights = 0u32;
                let mut key = [0u8; 256];
                key[..10].copy_from_slice(b"tnpausedur");
                let mut key_len = 10usize;
                if (mode_len as u32) < 255u32.wrapping_sub(10) {
                    key[key_len..key_len + mode_len].copy_from_slice(&buffers.mode[..mode_len]);
                    key_len += mode_len;
                    key[key_len] = 0;
                    let (status, ptr) = host.pause_weights(phrasing.env, &key[..key_len]);
                    if status < 0x80000000 {
                        weights = ptr;
                    }
                }
                if c_strstr(&buffers.mode[..mode_len], b"spell") {
                    let mut i = 0u16;
                    while i < mark_count
                        && ((buffers.marks[usize::from(i)].position.wrapping_sub(base)) as i16)
                            < (end as i16)
                    {
                        if buffers.marks[usize::from(i)].r#type == 31 {
                            weights = buffers.marks[usize::from(i)].value;
                        }
                        i = i.wrapping_add(1);
                    }
                }
                result = process_phrasing(
                    phrasing,
                    handle,
                    u32::from(end),
                    buffers,
                    &mut node,
                    &mut mark_index,
                    mark_count,
                    base,
                    weights,
                    &mut previous,
                    host,
                );
                if result >= 0x80000000 {
                    break 'cleanup result;
                }
                cursor = node;
            } else {
                let node_type = raw as u8;
                if node_type == 9 || node_type == 4 || node_type == 10 {
                    count = 0;
                    let (status, raw, _) = host.node_field(handle, cursor, 1, 1);
                    result = status;
                    if result >= 0x80000000 {
                        break 'cleanup result;
                    }
                    let start = raw as u16;
                    let (status, raw, _) = host.node_field(handle, cursor, 2, 1);
                    result = status;
                    if result >= 0x80000000 {
                        break 'cleanup result;
                    }
                    let end = raw as u16;
                    let span = (i32::from(end) - i32::from(start)) as i16;
                    let mut adjustment = 0i32;
                    if span > 1 {
                        adjustment = 1;
                    }
                    let mut pause = 0i16;
                    while mark_index < mark_count
                        && ((buffers.marks[usize::from(mark_index)]
                            .position
                            .wrapping_sub(base)) as i16)
                            < ((i32::from(end) - adjustment) as i16)
                    {
                        let mark = buffers.marks[usize::from(mark_index)];
                        if mark.r#type == 8 && previous != 0 {
                            pause = ((pause as i32 as u32).wrapping_add(mark.value)) as i16;
                            let (status, _) =
                                host.set_node_field(handle, previous, 8, 1, pause as u16 as u32);
                            if status >= 0x80000000 {
                                return status;
                            }
                        }
                        mark_index = mark_index.wrapping_add(1);
                    }
                    pause = 0;
                    let (status, n) = host.node_count(handle, cursor, 8);
                    count = n;
                    if status >= 0x80000000 || count == 0 {
                        let (status, _) =
                            host.set_node_field(handle, cursor, 8, 1, pause as u16 as u32);
                        if status >= 0x80000000 {
                            return status;
                        }
                    }
                    previous = cursor;
                }
                let (status, c) = host.next_node(handle, cursor);
                result = status;
                if result >= 0x80000000 {
                    break 'cleanup result;
                }
                cursor = c;
            }
        }
        break 'cleanup result;
    };
    if phrasing.z32 != 0 {
        host.free_text(phrasing.env, phrasing.z32);
    }
    phrasing.z32 = 0;
    if phrasing.z36 != 0 {
        host.free_text(phrasing.env, phrasing.z36);
    }
    phrasing.z36 = 0;
    result
}

/// `fn_1769d0` compared against zero: ASCII case-insensitive match with
/// prefix semantics.
///
/// Mirrors the C exactly: scanning stops with success at `a`'s NUL (so both a
/// strict prefix and a superstring of `b` match); only a mismatching byte
/// fails. The uppercase class is bit 1 of `D_8dc30c`, verified from the ROM to
/// be exactly `A-Z`.
fn c_strcasecmp_prefix(a: &[u8], b: &[u8]) -> bool {
    fn lower(c: u8) -> u8 {
        if c.is_ascii_uppercase() {
            c + 32
        } else {
            c
        }
    }
    let mut i = 0;
    if lower(a.get(i).copied().unwrap_or(0)) == lower(b.get(i).copied().unwrap_or(0)) {
        loop {
            if a.get(i).copied().unwrap_or(0) == 0 {
                return true;
            }
            i += 1;
            if lower(a.get(i).copied().unwrap_or(0)) != lower(b.get(i).copied().unwrap_or(0)) {
                break;
            }
        }
    }
    lower(a.get(i).copied().unwrap_or(0)) == lower(b.get(i).copied().unwrap_or(0))
}

/// Phonmap resource table loaded by `load_phonmap` (`fn_1e0400`).
///
/// `f12`-`f40`/`f556`/`f560` are the ten section slots (`LANG`, `VOIC`,
/// `LEXA`, `VEXA`, `SLAN`, `SVOI`, `SLEX`, `SVEX`, `IPA`, `SAPI`);
/// `b44`/`b300` the two version strings.
pub struct PhonmapObj {
    pub h: [u32; 2],
    pub p8: u32,
    pub f12: u32,
    pub f16: u32,
    pub f20: u32,
    pub f24: u32,
    pub f28: u32,
    pub f32: u32,
    pub f36: u32,
    pub f40: u32,
    pub b44: [u8; 256],
    pub b300: [u8; 256],
    pub f556: u32,
    pub f560: u32,
}

impl PhonmapObj {
    /// Section slot by `fn_1e0400` order: 0-9 map to `f12`, `f20`, `f28`,
    /// `f36`, `f16`, `f24`, `f32`, `f40`, `f556`, `f560`.
    fn slot_mut(&mut self, slot: u8) -> &mut u32 {
        match slot {
            0 => &mut self.f12,
            1 => &mut self.f20,
            2 => &mut self.f28,
            3 => &mut self.f36,
            4 => &mut self.f16,
            5 => &mut self.f24,
            6 => &mut self.f32,
            7 => &mut self.f40,
            8 => &mut self.f556,
            _ => &mut self.f560,
        }
    }
}

/// Resource/config calls reached by `load_phonmap` (`fn_1e0400`).
///
/// String arguments cross without the NUL terminator. `param` lends config
/// values for immediate use only; the language code is copied out into the
/// caller buffer because the C keeps it across later queries. `params`
/// resolves the `p8` block to `(f12, f20)`; `version` reads `D_8f8ad4`.
pub trait PhonmapHost {
    fn version(&mut self) -> u32;
    fn params(&mut self, p8: u32) -> (u32, u32);
    fn release_resource(&mut self, addr: u32);
    fn param_lang(&mut self, prm: u32, out: &mut [u8]) -> (u32, usize);
    fn param(&mut self, prm: u32, name: &[u8]) -> (u32, Option<&[u8]>);
    fn open_phonmap(
        &mut self,
        h: [u32; 2],
        mode: u8,
        path: &[u8],
        ver_major: u16,
        ver_minor: u8,
    ) -> (u32, u32);
    fn next_tag(&mut self, h: u32, tag: &mut [u8; 8]) -> (u32, u32, u32);
    fn read_entry(&mut self, h: u32, en: u32, size: u32, off: u32, buf: &mut [u8]) -> (u32, u32);
    fn load_section(&mut self, h: [u32; 2], path: &[u8], handle: u32, slot: u8) -> (u32, u32);
    fn close_entry(&mut self, h: u32) -> u32;
    fn close_phonmap(&mut self, h: u32) -> u32;
}

fn section_slot(tag: &[u8]) -> Option<u8> {
    [
        (b"LANG", 0),
        (b"LEXA", 1),
        (b"SLAN", 2),
        (b"SLEX", 3),
        (b"VOIC", 4),
        (b"VEXA", 5),
        (b"SVOI", 6),
        (b"SVEX", 7),
        (b"IPA ", 8),
        (b"SAPI", 9),
    ]
    .iter()
    .find(|(name, _)| c_strcmp(tag, &name[..]) == 0)
    .map(|(_, slot)| *slot)
}

/// fn_1e0400 / EU fn_1e05bc / KR fn_1e0668.
///
/// Loads the phonmap resources: releases the old sections, builds the two
/// lookup paths from config, then opens each pack and installs the advertised
/// sections plus the `version` strings into `b44`/`b300`. `arg` is the
/// NUL-terminated voice argument; `lang` receives the language code.
/// A second-pack open failure returns 0, matching the C.
pub fn load_phonmap(
    obj: &mut PhonmapObj,
    arg: &[u8],
    lang: &mut [u8],
    host: &mut impl PhonmapHost,
) -> u32 {
    let tmp = host.version();
    for slot in 0..10u8 {
        let entry = obj.slot_mut(slot);
        if *entry != 0 {
            host.release_resource(*entry);
            *entry = 0;
        }
    }
    let arg_len = c_str_len(arg, "arg");
    let (dir_mode, prm) = host.params(obj.p8);
    let (status, lang_len) = host.param_lang(prm, lang);
    let lang_len = if status >= 0x80000000 {
        lang[0] = 0;
        0
    } else {
        assert!(
            lang_len < lang.len() && lang[lang_len] == 0,
            "language code is not NUL-terminated"
        );
        lang_len
    };
    let mut buf = [[0u8; 128]; 2];
    let mut len = [0usize; 2];
    let (status, prefix) = host.param(prm, b"fedataprefix");
    if status < 0x80000000 {
        if let Some(prefix) = prefix {
            if !prefix.is_empty() && prefix[0] != 0 {
                let n = c_str_len(prefix, "prefix");
                buf[0][..n].copy_from_slice(&prefix[..n]);
                buf[0][n] = 0;
                len[0] = n;
                buf[1][..n].copy_from_slice(&prefix[..n]);
                buf[1][n] = 0;
                len[1] = n;
            }
        }
    }
    let (status, pack) = host.param(prm, b"fedatapackaging");
    if status < 0x80000000 {
        if let Some(pack) = pack {
            if c_strcmp(pack, b"clc") == 0 {
                let (status, fecfg) = host.param(prm, b"fecfg");
                if status >= 0x80000000 {
                    return status;
                }
                let fecfg = fecfg.unwrap_or(&[]);
                let fecfg_len = c_str_len(fecfg, "fecfg");
                push_bytes(&mut buf[0], &mut len[0], b"clc/");
                push_bytes(&mut buf[0], &mut len[0], &lang[..lang_len]);
                if !c_strstr(&arg[..arg_len], b"uni") {
                    push_byte(&mut buf[0], &mut len[0], b'/');
                    push_bytes(&mut buf[0], &mut len[0], &arg[..arg_len]);
                }
                push_byte(&mut buf[0], &mut len[0], b'/');
                push_bytes(&mut buf[0], &mut len[0], &fecfg[..fecfg_len]);
                push_byte(&mut buf[0], &mut len[0], b':');
                push_bytes(&mut buf[1], &mut len[1], b"clc/");
                push_bytes(&mut buf[1], &mut len[1], &lang[..lang_len]);
                push_byte(&mut buf[1], &mut len[1], b'/');
                push_bytes(&mut buf[1], &mut len[1], &fecfg[..fecfg_len]);
                push_byte(&mut buf[1], &mut len[1], b':');
            }
        }
    }
    let mode: [u8; 2];
    if dir_mode != 0 {
        push_bytes(&mut buf[0], &mut len[0], b"phonmap/");
        push_bytes(&mut buf[1], &mut len[1], b"phonmap_ipa/");
        mode = [0, 2];
        push_bytes(&mut buf[0], &mut len[0], &lang[..lang_len]);
        push_byte(&mut buf[0], &mut len[0], b'/');
        push_bytes(&mut buf[0], &mut len[0], &arg[..arg_len]);
        push_bytes(&mut buf[1], &mut len[1], &lang[..lang_len]);
    } else {
        push_bytes(&mut buf[0], &mut len[0], b"phonmap_");
        push_bytes(&mut buf[1], &mut len[1], b"phonmap_ipa_");
        mode = [1, 3];
        push_bytes(&mut buf[0], &mut len[0], &lang[..lang_len]);
        push_byte(&mut buf[0], &mut len[0], b'_');
        push_bytes(&mut buf[0], &mut len[0], &arg[..arg_len]);
        push_bytes(&mut buf[0], &mut len[0], b".dat");
        push_bytes(&mut buf[1], &mut len[1], &lang[..lang_len]);
        push_bytes(&mut buf[1], &mut len[1], b".dat");
    }
    buf[0][len[0]] = 0;
    buf[1][len[1]] = 0;
    let ver_major = tmp as u16;
    let ver_minor = (tmp >> 16) as u8;
    let mut h = 0u32;
    let mut status = 0u32;
    for i7 in 0..2usize {
        let path = &buf[i7][..len[i7]];
        let (open_status, handle) = host.open_phonmap(obj.h, mode[i7], path, ver_major, ver_minor);
        status = open_status;
        h = handle;
        if status >= 0x80000000 {
            if i7 != 0 {
                status = 0;
            }
            return status;
        }
        let mut tag = [0u8; 8];
        loop {
            let (tag_status, sz, en) = host.next_tag(h, &mut tag);
            if tag_status >= 0x80000000 {
                break;
            }
            let tag_len = c_str_len(&tag, "tag");
            let tag_str = &tag[..tag_len];
            if c_strcmp(tag_str, b"LINF") == 0 || c_strcmp(tag_str, b"VINF") == 0 {
                let mut off = 0u32;
                if status < 0x80000000 {
                    let mut d0 = [0u8; 256];
                    let mut d1 = [0u8; 256];
                    while off < sz {
                        d0[0] = 0;
                        d1[0] = 0;
                        let (read_status, n) = host.read_entry(h, en, sz, off, &mut d1);
                        status = read_status;
                        if status >= 0x80000000 {
                            break;
                        }
                        assert!(
                            (n as usize) < d1.len() && d1[..n as usize].contains(&0),
                            "entry name is not NUL-terminated"
                        );
                        if off >= sz {
                            break;
                        }
                        let t = n;
                        off = off.wrapping_add(t);
                        let (read_status, n) = host.read_entry(h, en, sz, off, &mut d0);
                        status = read_status;
                        if status >= 0x80000000 {
                            break;
                        }
                        assert!(
                            (n as usize) < d0.len() && d0[..n as usize].contains(&0),
                            "entry value is not NUL-terminated"
                        );
                        if c_strcasecmp_prefix(&d1, b"version") {
                            let value_len = c_strlen(&d0);
                            let slot = if i7 == 0 { &mut obj.b44 } else { &mut obj.b300 };
                            slot[..value_len + 1].copy_from_slice(&d0[..value_len + 1]);
                        }
                    }
                }
            } else if let Some(slot) = section_slot(tag_str) {
                let (section_status, value) = host.load_section(obj.h, path, h, slot);
                status = section_status;
                *obj.slot_mut(slot) = value;
            }
            let rc = host.close_entry(h);
            if rc >= 0x80000000 && status < 0x80000000 {
                status = rc;
            }
        }
        if h != 0 {
            let rc = host.close_phonmap(h);
            h = 0;
            if rc >= 0x80000000 && status < 0x80000000 {
                status = rc;
            }
        }
    }
    status
}

/// Caller-owned scratch buffers for `process_global_stage` (`fn_1f51f8`).
///
/// `layers` are the seven layer buffers (`f20`/`f24`/`f28`/`f2c`/`f30`/`f34`/
/// `f38`), each `total` bytes; `input` is the pronunciation text (mutated:
/// `-` becomes `_`); `input_prefix` is the byte before `input[0]`; `word` and
/// `w80` are per-node scratch; `lastw` persists across nodes; `output` is the
/// engine output with 11 spare bytes for the `\TQ\` insertion.
pub struct GlobalBuffers<'a> {
    pub layers: [&'a mut [u8]; 7],
    pub input: &'a mut [u8],
    pub input_prefix: u8,
    pub word: &'a mut [u8],
    pub w80: &'a mut [u8],
    pub lastw: &'a mut [u8],
    pub output: &'a mut [u8],
}

/// The seven `fn_1f51f8` layer strings with tracked lengths.
///
/// Appends NUL-terminate like the C `strcat`/`strncat` calls; every length
/// below matches the C `strlen` at the same point.
struct GlobalLayers<'a, 'b> {
    layers: &'a mut [&'b mut [u8]; 7],
    lens: [usize; 7],
}

impl<'a, 'b> GlobalLayers<'a, 'b> {
    fn append(&mut self, idx: usize, bytes: &[u8]) {
        let len = self.lens[idx];
        self.layers[idx][len..len + bytes.len()].copy_from_slice(bytes);
        self.lens[idx] = len + bytes.len();
        self.layers[idx][self.lens[idx]] = 0;
    }

    fn append_byte(&mut self, idx: usize, b: u8) {
        let len = self.lens[idx];
        self.layers[idx][len] = b;
        self.lens[idx] = len + 1;
        self.layers[idx][self.lens[idx]] = 0;
    }

    /// Appends `first` to layer 0 and `rest` to layers 1-5 (never 6).
    fn append6(&mut self, first: &[u8], rest: &[u8]) {
        self.append(0, first);
        for idx in 1..6 {
            self.append(idx, rest);
        }
    }

    fn len(&self, idx: usize) -> usize {
        self.lens[idx]
    }

    fn slice(&self, idx: usize) -> &[u8] {
        &self.layers[idx][..self.lens[idx]]
    }

    fn full(&mut self, idx: usize) -> &mut [u8] {
        &mut self.layers[idx][..]
    }

    fn recompute(&mut self, idx: usize) -> usize {
        let len = c_str_len(self.full(idx), "layer");
        self.lens[idx] = len;
        len
    }
}

/// Node/layer/dictionary calls reached by `process_global_stage` (`fn_1f51f8`).
///
/// Mirrors the `LingIf`/`DepesIf`/`DctVt` methods used by the C: `first_node`
/// is `m34`, `next_node` is `m3c`, `set_pred` is `m50` (field 3, the 4-byte
/// out block is unobserved), `node_field` is `m54`, `node_count` is `m5c`;
/// `input_text`/`word_text`/`alt_text`/`tag_text` copy out `m58` modes
/// 0/3/10/4 because the C keeps them across later calls, while `mode_text`
/// lends mode-4 tag bytes for immediate use and `layer_byte` returns the
/// single bytes the C reads from `m58` modes 5/6/9; `set_layer_input` is
/// `m24`, `layer_output` is `m28`, `run_layers` is `m2c`,
/// `set_layer_input2` is `m40`, `prepare_layer` is the dictionary `m20`
/// (which may rewrite the layer, so the full buffer crosses); `foreign_param`
/// reads `foreignpron`. String arguments cross without the NUL terminator.
/// The `env` argument is the raw environment word; the host resolves
/// heap/log/params from the environment block. Log stages are 0-6 for the
/// `L1`-`L7` calls and 7 for `O1`.
#[allow(clippy::too_many_arguments)]
pub trait GlobalProcessHost {
    fn validate(&mut self, object: u32, b: u32, tag: u32, size: u32) -> u32;
    fn foreign_param(&mut self, env: u32) -> (u32, Option<&[u8]>);
    fn first_node(&mut self, handle: [u32; 2], kind: i32, sub: u32) -> (u32, u16);
    fn next_node(&mut self, handle: [u32; 2], node: u16) -> (u32, u16);
    fn node_field(
        &mut self,
        handle: [u32; 2],
        node: u16,
        field: i32,
        count: i32,
    ) -> (u32, u32, u16);
    fn node_count(&mut self, handle: [u32; 2], node: u16, field: i32) -> (u32, u16);
    fn input_text(&mut self, handle: [u32; 2], node: u16, out: &mut [u8]) -> (u32, u16);
    fn word_text(&mut self, handle: [u32; 2], node: u16, out: &mut [u8]) -> (u32, u16);
    fn alt_text(&mut self, handle: [u32; 2], node: u16, out: &mut [u8]) -> (u32, u16);
    fn tag_text(&mut self, handle: [u32; 2], node: u16, out: &mut [u8]) -> (u32, u16);
    fn layer_byte(&mut self, handle: [u32; 2], node: u16, field: i32) -> (u32, u8, u16);
    fn mode_text(&mut self, handle: [u32; 2], node: u16) -> (u32, &[u8], u16);
    fn set_pred(&mut self, handle: [u32; 2], node: u16, len: u32, src: &[u8]) -> u32;
    fn set_layer_input(&mut self, handle: [u32; 2], index: i32, text: &[u8]) -> u32;
    fn set_layer_input2(&mut self, handle: [u32; 2], index: i32, text: &[u8]) -> u32;
    fn layer_output(&mut self, handle: [u32; 2], index: i32, buf: &mut [u8]) -> (u32, u16);
    fn run_layers(&mut self, handle: [u32; 2], key: &[u8]) -> u32;
    fn prepare_layer(&mut self, handle: [u32; 2], tag: &[u8], text: &mut [u8]) -> u32;
    fn log_layer(&mut self, env: u32, stage: u8, key: &[u8], text: &[u8]);
    fn log_nomem(&mut self, env: u32);
    fn alloc_layer(&mut self, env: u32, size: u16) -> u32;
    fn free_layer(&mut self, env: u32, addr: u32);
    /// Guest-memory channel for the ROM's own in-guest writes. The C keeps
    /// token and layer text in guest buffers the port only borrows: the
    /// fill loop strcats into the f20.. staging buffers
    /// (pc 0x1f6298-0x1f62ac), the layer fetch NUL-terminates the guest
    /// layer text in place, and the token loop NUL-terminates each guest
    /// token at its delimiter (pc 0x1f7ce8). The port performs those writes
    /// through these primitives; the live host stores them in the guest
    /// arena, scripted hosts ignore them (their script asserts the port's
    /// own calls, which are unchanged).
    fn guest_read_byte(&self, _addr: u32) -> u8 {
        0
    }
    fn guest_write_byte(&mut self, _addr: u32, _val: u8) {}
    fn guest_write_bytes(&mut self, _addr: u32, _data: &[u8]) {}
    /// Guest cursor for the token loop's in-guest NUL writes: the guest
    /// address of the current token's first byte (0 when unknown).
    fn token_cursor(&self) -> u32 {
        0
    }
    fn set_token_cursor(&mut self, _cur: u32) {}
    /// Guest address of the layer staging buffer for `index` (0 unknown).
    fn staging_buffer(&self, _index: i32) -> u32 {
        0
    }
    /// NUL-terminate the guest layer text fetched by the last `layer_output`.
    fn terminate_layer_text(&mut self, _len: u16) {}
}
/// Mirror the C token loop's in-guest NUL termination: scan the guest token
/// from the cursor for the first ' ', '#' or NUL; NUL-terminate at a
/// delimiter and leave the cursor past it (plus any further delimiters),
/// exactly like the C (`*tail = 0`, `ostr[0] = fk`, skip loop; pc 0x1f7ce8).
/// The old code placed the NUL at `cursor + rust_len - 1`, which misses
/// whenever the Rust token and the guest token have different lengths
/// (seg5 r15: live NUL at 0x57, ARM at 0x53 on 'who won the game?').
/// Mirror the caller `output` buffer back over the guest layer-0 row
/// (`ostr[0]` from `m28`/GetLayer, whose base the host stashed as the
/// token cursor). The C rewrites that row in place (quote map,
/// compaction, tag insertion); without this the guest keeps the
/// pre-insertion bytes and every later guest scan drifts.
fn commit_layer_output<H: GlobalProcessHost>(host: &mut H, output: &[u8], cnt: u16) {
    let base = host.token_cursor();
    if base == 0 {
        return;
    }
    // Include the terminator at `[cnt]`: the C shift loop moves the old
    // `[old_cnt]` NUL there, and the final walk stops its guest scan on
    // it (ROM 0x1f7a58 writes down through the tag-shifted tail).
    let end = (cnt as usize).wrapping_add(1);
    if end <= output.len() {
        host.guest_write_bytes(base, &output[..end]);
    }
}

fn terminate_guest_token<H: GlobalProcessHost>(host: &mut H) {
    let mut cur = host.token_cursor();
    if cur == 0 {
        return;
    }
    loop {
        let b = host.guest_read_byte(cur);
        if b == 0 {
            break;
        }
        if b == b' ' || b == b'#' {
            host.guest_write_byte(cur, 0);
            cur = cur.wrapping_add(1);
            break;
        }
        cur = cur.wrapping_add(1);
    }
    let mut next = cur;
    loop {
        let b = host.guest_read_byte(next);
        if b != b' ' && b != b'#' {
            break;
        }
        next = next.wrapping_add(1);
    }
    host.set_token_cursor(next);
}
/// Mirror the C fill loop's strcat into the guest staging buffers
/// (pc 0x1f6298-0x1f62ac): copy the layer text to the guest staging
/// buffer, exactly as the depes call leaves it.
fn mirror_guest_staging<H: GlobalProcessHost>(host: &mut H, index: i32, text: &[u8]) {
    let base = host.staging_buffer(index);
    if base == 0 {
        return;
    }
    host.guest_write_bytes(base, text);
    host.guest_write_byte(base.wrapping_add(text.len() as u32), 0);
}

/// Appends `lastw[1..lw-1]` (`strncat(lastw + 1, lw - 2)`).
///
/// `lw == 1` copies only the NUL (a no-op); `lw == 0` would read past the
/// terminator into heap garbage, which no harness can reproduce, so it is a
/// loud contract violation.
fn append_lastw_inner(layers: &mut GlobalLayers<'_, '_>, lastw: &[u8], lw: usize) {
    if lw >= 2 {
        layers.append(6, &lastw[1..lw - 1]);
    } else if lw == 0 {
        panic!("empty tag word");
    }
}

/// Appends an input span (`strncat(f24, input + beg, n)`).
///
/// Copies until the input NUL at the latest: `n <= 0` copies nothing for 0
/// and everything up to the NUL for negative `n`, matching `fn_195dac`.
/// `beg` past the terminator would read heap garbage, so it is a loud
/// contract violation.
fn append_span(
    layers: &mut GlobalLayers<'_, '_>,
    idx: usize,
    input: &[u8],
    input_end: usize,
    beg: usize,
    n: i32,
) {
    assert!(beg <= input_end, "span starts past input end");
    let rest = input_end - beg;
    let k = if n <= 0 {
        if n == 0 {
            0
        } else {
            rest
        }
    } else {
        (n as usize).min(rest)
    };
    layers.append(idx, &input[beg..beg + k]);
}

/// fn_1f51f8 / EU fn_1f53b4 / KR fn_1f5460.
///
/// FE_GLOBAL stage Process entry: sizes and builds the six pronunciation
/// layers (plus the tag layer), runs them through the rule layers, and writes
/// the output tokens back to the word nodes. `done` is set to 1; layer
/// addresses land in `global.f20`-`f38` (0 after cleanup; left allocated on
/// the two direct engine-error returns, matching the C).
#[allow(clippy::too_many_arguments)]
pub fn process_global_stage(
    global: &mut Global,
    b: u32,
    handle: [u32; 2],
    done: &mut u8,
    buffers: &mut GlobalBuffers<'_>,
    host: &mut impl GlobalProcessHost,
) -> u32 {
    let mut tmp = [0u8; 128];
    tmp[..6].copy_from_slice(b"global");
    let tmplen = 6usize;
    let mut tmp_len = 6usize;
    let mut total = 1u32;
    if host.validate(global.id, b, 0xf388, 0x40) >= 0x80000000 {
        return 0x8a102008;
    }
    *done = 1;
    global.foreign = 0;
    let (status, mode) = host.foreign_param(global.env);
    if status < 0x80000000 {
        if let Some(mode) = mode {
            if c_strcmp(mode, b"yes") == 0 {
                global.foreign = 1;
            }
        }
    }
    let (status, id_a) = host.first_node(handle, 1, 0);
    if status >= 0x80000000 {
        return status;
    }
    let (status, has8) = host.node_count(handle, id_a, 0);
    if status >= 0x80000000 {
        return status;
    }
    if has8 != 1 {
        return status;
    }
    let (status, input_len) = host.input_text(handle, id_a, buffers.input);
    if status >= 0x80000000 {
        return status;
    }
    if input_len <= 1 {
        return status;
    }
    let input_end = c_str_len(buffers.input, "input");
    let mut layers = GlobalLayers {
        layers: &mut buffers.layers,
        lens: [0; 7],
    };
    let rc = 'fail: loop {
        let (status, mut id_b) = host.first_node(handle, 2, u32::from(id_a));
        if status >= 0x80000000 {
            break 'fail status;
        }
        // Size walk.
        let mut n64 = 0u32;
        let mut n60 = 0u32;
        let mut first = true;
        let mut has_t = 0u16;
        let mut w80_len = 0usize;
        // NOTE: the C `word` pointer is set by the size walk too, so kind-7
        // nodes before the fill walk's first word still see it.
        let mut word_seen = false;
        while id_b != 0 {
            let (status, raw, _) = host.node_field(handle, id_b, 0, 1);
            if status >= 0x80000000 {
                break 'fail status;
            }
            let kind = raw as u8;
            // ROM 0x1f5400-0x1f5498: kinds 4/9/10/11/13/12/14 share the word
            // arm. Kind 10 enters at the n60 flush (0x1f5450), skipping the
            // n64 flush, then takes the separator +1 and sets n64 (0x1f5490)
            // exactly like kind 13 sets n60 (0x1f54ac), before the shared
            // word-length tail. The old `kind == 10` divert above skipped the
            // word arm, so dictionary phonetic words (e.g. "mii") sized 1.
            if kind == 4
                || kind == 9
                || kind == 10
                || kind == 11
                || kind == 13
                || kind == 12
                || kind == 14
            {
                if kind != 10 && n64 != 0 {
                    total = (total.wrapping_add(3)) & 0xffff;
                    n64 = 0;
                }
                if kind != 11 && n60 != 0 {
                    total = (total.wrapping_add(2)) & 0xffff;
                    n60 = 0;
                }
                if !first {
                    total = (total.wrapping_add(1)) & 0xffff;
                }
                if kind == 10 {
                    if n64 == 0 {
                        total = (total.wrapping_add(3)) & 0xffff;
                        n64 = 1;
                    }
                } else if kind == 13 {
                    total = total.wrapping_add(2);
                    n60 = 1;
                    total &= 0xffff;
                }
                first = false;
                let (status, raw, _) = host.node_field(handle, id_b, 1, 1);
                if status >= 0x80000000 {
                    break 'fail status;
                }
                let beg = raw as u16;
                let (status, raw, _) = host.node_field(handle, id_b, 2, 1);
                if status >= 0x80000000 {
                    break 'fail status;
                }
                let end = raw as u16;
                let (status, _, _) = host.layer_byte(handle, id_b, 5);
                if status >= 0x80000000 {
                    break 'fail status;
                }
                let (status, _) = host.word_text(handle, id_b, buffers.word);
                if status >= 0x80000000 {
                    break 'fail status;
                }
                word_seen = true;
                let word_len = c_str_len(buffers.word, "word");
                let (status, count) = host.node_count(handle, id_b, 10);
                if status < 0x80000000 && count == 1 {
                    let (status, _) = host.alt_text(handle, id_b, buffers.w80);
                    if status >= 0x80000000 {
                        break 'fail status;
                    }
                    w80_len = c_str_len(buffers.w80, "alt word");
                } else {
                    w80_len = 0;
                }
                if word_len != 0 {
                    if has_t != 0 {
                        total = (total.wrapping_add(2)) & 0xffff;
                    }
                    let mark = c_strchr(buffers.word, word_len, 0x1e);
                    let (status, count) = host.node_count(handle, id_b, 8);
                    if status < 0x80000000 && count == 1 {
                        let (status, raw, _) = host.node_field(handle, id_b, 8, 1);
                        if status >= 0x80000000 {
                            break 'fail status;
                        }
                        has_t = raw as u16;
                    }
                    if let Some(mark_pos) = mark {
                        let dd = w80_len as i16;
                        let mut span = (i32::from(end) - i32::from(beg)) as i16;
                        if span < dd {
                            span = w80_len as i16;
                        }
                        let mut cut = (mark_pos as i16) as i32;
                        if cut > span as i32 {
                            span = (i32::from(end) - i32::from(beg)) as i16;
                            if (span as i32) < w80_len as i32 {
                                span = w80_len as i16;
                            }
                            cut = span as i32;
                        }
                        total = (((cut as u32).wrapping_add(total)) & 0xffff)
                            .wrapping_add((word_len - mark_pos) as u32);
                    } else {
                        let a = word_len as i16;
                        let dd = w80_len as i16;
                        let mut span = (i32::from(end) - i32::from(beg)) as i16;
                        if span < dd {
                            span = w80_len as i16;
                        }
                        let best = if span < a {
                            a as i32
                        } else {
                            let best = (i32::from(end) - i32::from(beg)) as i16;
                            if (best as i32) < w80_len as i32 {
                                (w80_len as i16) as i32
                            } else {
                                best as i32
                            }
                        };
                        total = total.wrapping_add(best as u32);
                    }
                    total &= 0xffff;
                }
            }
            if global.use_tn_tag != 0 && kind == 6 {
                let (status, mark, _) = host.mode_text(handle, id_b);
                if status >= 0x80000000 {
                    break 'fail status;
                }
                let mark_len = c_str_len(mark, "tag");
                total = total
                    .wrapping_add(((mark_len as i32).wrapping_mul(2).wrapping_sub(2)) as u32)
                    & 0xffff;
            }
            if kind == 7 {
                let (status, mark, _) = host.mode_text(handle, id_b);
                if status >= 0x80000000 {
                    break 'fail status;
                }
                let mark_len = c_str_len(mark, "mark");
                total = (mark_len as u32).wrapping_add(total).wrapping_add(2) & 0xffff;
            }
            let (status, next) = host.next_node(handle, id_b);
            if status >= 0x80000000 {
                break 'fail status;
            }
            id_b = next;
        }
        if n64 != 0 {
            total = (total.wrapping_add(3)) & 0xffff;
        }
        if n60 != 0 {
            total = (total.wrapping_add(2)) & 0xffff;
        }
        // Layer buffers.
        global.f20 = host.alloc_layer(global.env, total as u16);
        if global.f20 == 0 {
            host.log_nomem(global.env);
            break 'fail 0x8a10200a;
        }
        global.f24 = host.alloc_layer(global.env, total as u16);
        if global.f24 == 0 {
            host.log_nomem(global.env);
            break 'fail 0x8a10200a;
        }
        global.f28 = host.alloc_layer(global.env, total as u16);
        if global.f28 == 0 {
            host.log_nomem(global.env);
            break 'fail 0x8a10200a;
        }
        global.f2c = host.alloc_layer(global.env, total as u16);
        if global.f2c == 0 {
            host.log_nomem(global.env);
            break 'fail 0x8a10200a;
        }
        global.f30 = host.alloc_layer(global.env, total as u16);
        if global.f30 == 0 {
            host.log_nomem(global.env);
            break 'fail 0x8a10200a;
        }
        global.f34 = host.alloc_layer(global.env, total as u16);
        if global.f34 == 0 {
            host.log_nomem(global.env);
            break 'fail 0x8a10200a;
        }
        for idx in 0..6 {
            layers.layers[idx][0] = 0;
        }
        if global.use_tn_tag != 0 {
            let addr = host.alloc_layer(global.env, total as u16);
            global.f38 = addr;
            if addr == 0 {
                host.log_nomem(global.env);
                break 'fail 0x8a10200a;
            }
            layers.layers[6][0] = 0;
        }
        // Fill walk.
        first = true;
        has_t = 0;
        let (status, id) = host.first_node(handle, 2, u32::from(id_a));
        if status < 0x80000000 {
            id_b = id;
            n64 = 0;
            n60 = 0;
            let mut u_end = 0u32;
            let mut al1 = 0u32;
            let mut al2 = 0u32;
            let mut olen = 0u32;
            let mut wlen = 0i32;
            let mut id130 = 0u16;
            let mut id128 = 0u16;
            let mut lastw_len = 0usize;
            let mut lastw_valid = false;
            let mut fill_rc = status;
            while id_b != 0 {
                let (status, raw, _) = host.node_field(handle, id_b, 0, 1);
                fill_rc = status;
                if fill_rc >= 0x80000000 {
                    break 'fail fill_rc;
                }
                let kind = raw as u8;
                if kind == 4
                    || kind == 9
                    || kind == 10
                    || kind == 11
                    || kind == 13
                    || kind == 12
                    || kind == 14
                {
                    let (status, raw, _) = host.node_field(handle, id_b, 1, 1);
                    fill_rc = status;
                    if fill_rc >= 0x80000000 {
                        break 'fail fill_rc;
                    }
                    let beg = raw as u16;
                    let (status, raw, _) = host.node_field(handle, id_b, 2, 1);
                    fill_rc = status;
                    if fill_rc >= 0x80000000 {
                        break 'fail fill_rc;
                    }
                    let end = raw as u16;
                    u_end = u32::from(end);
                    let (status, w78, _) = host.layer_byte(handle, id_b, 5);
                    fill_rc = status;
                    if fill_rc >= 0x80000000 {
                        break 'fail fill_rc;
                    }
                    let (status, w74, _) = host.layer_byte(handle, id_b, 6);
                    fill_rc = status;
                    if fill_rc >= 0x80000000 {
                        break 'fail fill_rc;
                    }
                    let (status, count) = host.node_count(handle, id_b, 10);
                    if status < 0x80000000 && count == 1 {
                        let (status, _) = host.alt_text(handle, id_b, buffers.w80);
                        fill_rc = status;
                        if fill_rc >= 0x80000000 {
                            break 'fail fill_rc;
                        }
                        w80_len = c_str_len(buffers.w80, "alt word");
                    } else {
                        w80_len = 0;
                    }
                    let (status, count) = host.node_count(handle, id_b, 9);
                    let mut cbuf = b'~';
                    if status < 0x80000000 && count == 1 {
                        let (status, byte, _) = host.layer_byte(handle, id_b, 9);
                        fill_rc = status;
                        if fill_rc >= 0x80000000 {
                            break 'fail fill_rc;
                        }
                        if byte != 0 {
                            cbuf = byte;
                        }
                    }
                    let (status, _) = host.word_text(handle, id_b, buffers.word);
                    fill_rc = status;
                    if fill_rc >= 0x80000000 {
                        break 'fail fill_rc;
                    }
                    word_seen = true;
                    let word_len = c_str_len(buffers.word, "word");
                    wlen = (word_len & 0xffff) as i32;
                    if word_len != 0 {
                        for b in buffers.word[..word_len].iter_mut() {
                            if *b == b'~' {
                                *b = 0xad;
                            }
                        }
                        assert!((beg as usize) <= input_end, "word starts past input end");
                        if global.use_tn_tag != 0 && !first {
                            if !lastw_valid || beg == id130 {
                                let mut c = 0u32;
                                if al1 != 0 {
                                    loop {
                                        layers.append(6, b" ");
                                        c = c.wrapping_add(1) & 0xffff;
                                        if !(c < al1) {
                                            break;
                                        }
                                    }
                                }
                            } else if id128 < beg {
                                let lw = lastw_len;
                                if (((lw as i32).wrapping_mul(2).wrapping_sub(2)) as u32) < al1 {
                                    layers.append(6, b"X");
                                    let lw = c_str_len(buffers.lastw, "last tag");
                                    append_lastw_inner(&mut layers, buffers.lastw, lw);
                                    let lw = c_str_len(buffers.lastw, "last tag");
                                    al2 = (((lw as i32).wrapping_mul(2).wrapping_sub(2)) as u32)
                                        & 0xffff;
                                    while (al2 & 0xffff) < al1 {
                                        layers.append(6, b"~");
                                        al2 = (al2 & 0xffff).wrapping_add(1);
                                    }
                                    layers.append(6, b"Y");
                                    let lw = c_str_len(buffers.lastw, "last tag");
                                    append_lastw_inner(&mut layers, buffers.lastw, lw);
                                } else {
                                    layers.append(6, b"X");
                                    let lw = c_str_len(buffers.lastw, "last tag");
                                    append_lastw_inner(&mut layers, buffers.lastw, lw);
                                    layers.append(6, b"Y");
                                    let lw = c_str_len(buffers.lastw, "last tag");
                                    append_lastw_inner(&mut layers, buffers.lastw, lw);
                                    let lw = c_str_len(buffers.lastw, "last tag");
                                    if al1 < (((lw as i32).wrapping_mul(2).wrapping_sub(2)) as u32)
                                    {
                                        loop {
                                            for idx in 0..6 {
                                                layers.append(idx, b"~");
                                            }
                                            al1 = al1.wrapping_add(1) & 0xffff;
                                            let lw = c_str_len(buffers.lastw, "last tag");
                                            if !(al1
                                                < (((lw as i32).wrapping_mul(2).wrapping_sub(2))
                                                    as u32))
                                            {
                                                break;
                                            }
                                        }
                                    }
                                }
                                lastw_valid = false;
                            }
                        }
                        total = 0;
                        if kind == 10 {
                            if n60 != 0 {
                                layers.append6(b"\x12\x10", b"  ");
                                total = total.wrapping_add(2);
                                n60 = 0;
                            }
                        } else {
                            if n64 != 0 {
                                layers.append6(b"\x11/+", b"   ");
                                total = 3;
                                n64 = 0;
                            }
                            if kind != 11 && n60 != 0 {
                                layers.append6(b"\x12\x10", b"  ");
                                total = total.wrapping_add(2);
                                n60 = 0;
                            }
                        }
                        if has_t != 0 {
                            layers.append6(b" #", b"  ");
                            total = total.wrapping_add(2);
                        }
                        let (status, count) = host.node_count(handle, id_b, 8);
                        if status < 0x80000000 && count == 1 {
                            let (status, raw, _) = host.node_field(handle, id_b, 8, 1);
                            fill_rc = status;
                            if fill_rc >= 0x80000000 {
                                break 'fail fill_rc;
                            }
                            has_t = raw as u16;
                        } else {
                            has_t = 0;
                        }
                        if !first {
                            layers.append_byte(0, b' ');
                            let sep = if beg == 0 {
                                buffers.input_prefix
                            } else {
                                buffers.input[usize::from(beg) - 1]
                            };
                            if sep != 0 {
                                layers.append_byte(1, sep);
                            }
                            for idx in 2..6 {
                                layers.append_byte(idx, b' ');
                            }
                            total = total.wrapping_add(1);
                        }
                        // ROM 0x1f6188-0x1f6228: kind 10 with n64 == 0 appends
                        // "\x11/+" to layer 0 and "   " to layers 1-5, then
                        // sets n64 (r1 carries the "   " into the shared
                        // tail). It never takes the kind-13 pad path: the
                        // draft C's shared `after_pad` (and the old `pad`
                        // flag here) wrongly gave kind-10 words the
                        // "\x10\x12" marker, breaking every dictionary
                        // phonetic word (e.g. "mii" lost its transcription).
                        // Kind 13 alone appends "\x10\x12" / "  " and sets
                        // n60.
                        if kind == 10 && n64 == 0 {
                            layers.append6(b"\x11/+", b"   ");
                            total = total.wrapping_add(3);
                            n64 = 1;
                        } else if kind == 13 {
                            layers.append6(b"\x10\x12", b"  ");
                            total = total.wrapping_add(2);
                            n60 = 1;
                        }
                        let mark_pos = c_strchr(buffers.word, word_len, 0x1e);
                        if let Some(pos) = mark_pos {
                            buffers.word[pos] = 0;
                        }
                        let word_len_full = word_len;
                        let word_len = mark_pos.unwrap_or(word_len);
                        layers.append(0, &buffers.word[..word_len]);
                        append_span(
                            &mut layers,
                            1,
                            buffers.input,
                            input_end,
                            beg as usize,
                            end as i32 - beg as i32,
                        );
                        if w78 != 0 {
                            layers.append_byte(2, w78);
                        }
                        if w74 != 0 {
                            layers.append_byte(3, w74);
                        }
                        layers.append_byte(4, cbuf);
                        layers.append(5, &buffers.w80[..w80_len]);
                        // Three-way width alignment.
                        let span = ((i32::from(end) - i32::from(beg)) as i16) as i32;
                        let dw_path = (w80_len as i32) >= span && {
                            let dw4 = w80_len as i32;
                            let ww4 = word_len as i32;
                            dw4 >= ww4
                        };
                        if dw_path {
                            al1 = 0;
                            let bound = ((w80_len as i32 - i32::from(end)) as u32)
                                .wrapping_add(u32::from(beg));
                            if bound != 0 {
                                loop {
                                    layers.append(1, b"~");
                                    al1 = al1.wrapping_add(1) & 0xffff;
                                    let dw4 = w80_len as i32;
                                    if !(al1
                                        < ((dw4 - i32::from(end)) as u32)
                                            .wrapping_add(u32::from(beg)))
                                    {
                                        break;
                                    }
                                }
                            }
                            al1 = 0;
                            if (w80_len as i32) != word_len as i32 {
                                loop {
                                    layers.append(0, b"~");
                                    wlen = (wlen.wrapping_add(1)) & 0xffff;
                                    al1 = al1.wrapping_add(1) & 0xffff;
                                    let dw4 = w80_len as i32;
                                    let d = word_len as i32;
                                    if !(al1 < (dw4 - d) as u32) {
                                        break;
                                    }
                                }
                            }
                            al1 = 0;
                            if (w80_len as i32) != 1 {
                                loop {
                                    if w78 != 0 {
                                        layers.append_byte(2, w78);
                                    }
                                    if w74 != 0 {
                                        layers.append_byte(3, w74);
                                    }
                                    layers.append_byte(4, cbuf);
                                    al1 = al1.wrapping_add(1) & 0xffff;
                                    if !(al1 < (w80_len as u32).wrapping_sub(1)) {
                                        break;
                                    }
                                }
                            }
                            al1 = (w80_len as u32) & 0xffff;
                        } else {
                            let ww = word_len as i32;
                            let mut do_fit = false;
                            let mut do_ww = false;
                            if ww < span {
                                do_fit = true;
                            } else if (word_len as i32) < w80_len as i32 {
                                do_fit = true;
                            } else {
                                do_ww = true;
                            }
                            if do_ww {
                                al1 = 0;
                                let bound = ((word_len as i32 - i32::from(end)) as u32)
                                    .wrapping_add(u32::from(beg));
                                if bound != 0 {
                                    loop {
                                        layers.append(1, b"~");
                                        al1 = al1.wrapping_add(1) & 0xffff;
                                        let ww3 = word_len as i32;
                                        if !(al1
                                            < ((ww3 - i32::from(end)) as u32)
                                                .wrapping_add(u32::from(beg)))
                                        {
                                            break;
                                        }
                                    }
                                }
                                al1 = 0;
                                if (word_len as i32) != w80_len as i32 {
                                    loop {
                                        layers.append(5, b"~");
                                        al1 = al1.wrapping_add(1) & 0xffff;
                                        let ww3 = word_len as i32;
                                        let d = w80_len as i32;
                                        if !(al1 < (ww3 - d) as u32) {
                                            break;
                                        }
                                    }
                                }
                                al1 = 0;
                                if (word_len as i32) != 1 {
                                    loop {
                                        if w78 != 0 {
                                            layers.append_byte(2, w78);
                                        }
                                        if w74 != 0 {
                                            layers.append_byte(3, w74);
                                        }
                                        layers.append_byte(4, cbuf);
                                        al1 = al1.wrapping_add(1) & 0xffff;
                                        if !(al1 < (word_len as u32).wrapping_sub(1)) {
                                            break;
                                        }
                                    }
                                }
                                al1 = (word_len as u32) & 0xffff;
                            }
                            if do_fit {
                                let ww2 = word_len as i32;
                                let dw2 = w80_len as i32;
                                if ww2 <= span && dw2 <= span {
                                    al1 = 0;
                                    if span - 1 > 0 {
                                        loop {
                                            if w78 != 0 {
                                                layers.append_byte(2, w78);
                                            }
                                            if w74 != 0 {
                                                layers.append_byte(3, w74);
                                            }
                                            layers.append_byte(4, cbuf);
                                            al1 = al1.wrapping_add(1) & 0xffff;
                                            if !((al1 as i32) < span - 1) {
                                                break;
                                            }
                                        }
                                    }
                                    al1 = 0;
                                    if ((i32::from(end) - i32::from(beg)) as u32) != word_len as u32
                                    {
                                        loop {
                                            layers.append(0, b"~");
                                            wlen = (wlen.wrapping_add(1)) & 0xffff;
                                            al1 = al1.wrapping_add(1) & 0xffff;
                                            if !(al1
                                                < ((i32::from(end) - i32::from(beg)) as u32)
                                                    .wrapping_sub(word_len as u32))
                                            {
                                                break;
                                            }
                                        }
                                    }
                                    al2 = 0;
                                    al1 = u32::from(beg);
                                    if ((i32::from(end) - al1 as i32) as u32) != w80_len as u32 {
                                        loop {
                                            layers.append(5, b"~");
                                            al2 = al2.wrapping_add(1) & 0xffff;
                                            al1 = u32::from(beg);
                                            if !(al2
                                                < (u32::from(end).wrapping_sub(al1))
                                                    .wrapping_sub(w80_len as u32))
                                            {
                                                break;
                                            }
                                        }
                                    }
                                    al1 = u32::from(end).wrapping_sub(al1);
                                }
                            }
                        }
                        al2 = al1;
                        if let Some(pos) = mark_pos {
                            buffers.word[pos] = 0x1e;
                            let tail = word_len_full - pos;
                            layers.append(0, &buffers.word[pos..pos + tail]);
                            let d = tail;
                            al2 = (d as u32).wrapping_add(al2) & 0xffff;
                            if d != 0 {
                                for _ in 0..d {
                                    layers.append(1, b"~");
                                }
                                for _ in 0..d {
                                    if w78 != 0 {
                                        layers.append_byte(2, w78);
                                    }
                                }
                                for _ in 0..d {
                                    if w74 != 0 {
                                        layers.append_byte(3, w74);
                                    }
                                }
                                for _ in 0..d {
                                    layers.append_byte(4, cbuf);
                                }
                                for _ in 0..d {
                                    layers.append(5, b"~");
                                }
                            }
                        }
                        if global.use_tn_tag != 0 {
                            if !lastw_valid || beg == id130 {
                                if total != 0 {
                                    for _ in 0..total {
                                        layers.append(6, b" ");
                                    }
                                }
                                if lastw_valid {
                                    al1 = al2;
                                }
                            } else {
                                al1 = al2.wrapping_add(total).wrapping_add(al1) & 0xffff;
                            }
                        }
                        first = false;
                    }
                }
                if kind == 7 {
                    if n60 == 0 {
                        let (status, raw, _) = host.node_field(handle, id_b, 1, 1);
                        fill_rc = status;
                        if fill_rc >= 0x80000000 {
                            break 'fail fill_rc;
                        }
                        let beg = raw as u16;
                        let (status, mark, _) = host.mode_text(handle, id_b);
                        fill_rc = status;
                        if fill_rc >= 0x80000000 {
                            break 'fail fill_rc;
                        }
                        let mark_len = c_str_len(mark, "mark");
                        let word_len_now = c_str_len(buffers.word, "word");
                        let star = if word_seen {
                            c_strchr(buffers.word, word_len_now, b'*')
                        } else {
                            None
                        };
                        let single_star = star.is_some_and(|pos| {
                            !buffers.word[pos + 1..word_len_now].contains(&b'*')
                        });
                        if (beg as i32) < (u_end.wrapping_sub(1) as i32)
                            && star.is_some()
                            && single_star
                        {
                            let pos = star.unwrap();
                            let lb = (layers.len(0) & 0xffff) as i32;
                            let cut = (pos & 0xffff) as i32;
                            let adv = wlen - cut;
                            let dst = lb - wlen + cut;
                            total = ((mark_len as u32).wrapping_add(2)) & 0xffff;
                            let total_n = total as usize;
                            let dst_u = dst as usize;
                            layers.layers[0]
                                .copy_within(dst_u..dst_u + (adv as usize) + 1, dst_u + total_n);
                            layers.layers[2]
                                .copy_within(dst_u..dst_u + (adv as usize) + 1, dst_u + total_n);
                            layers.layers[3]
                                .copy_within(dst_u..dst_u + (adv as usize) + 1, dst_u + total_n);
                            layers.layers[4]
                                .copy_within(dst_u..dst_u + (adv as usize) + 1, dst_u + total_n);
                            for idx in [0usize, 2, 3, 4] {
                                layers.lens[idx] += total_n;
                            }
                            layers.layers[0][dst_u] = 0x1e;
                            layers.layers[2][dst_u] = 0x20;
                            layers.layers[3][dst_u] = 0x20;
                            layers.layers[4][dst_u] = 0x20;
                            for i2 in 0..mark_len {
                                layers.layers[0][dst_u + 1 + i2] = mark[i2];
                                layers.layers[2][dst_u + 1 + i2] = 0x20;
                                layers.layers[3][dst_u + 1 + i2] = 0x20;
                                layers.layers[4][dst_u + 1 + i2] = 0x20;
                            }
                            layers.layers[0][dst_u + total_n - 1] = 0x1f;
                            layers.layers[2][dst_u + total_n - 1] = 0x20;
                            layers.layers[3][dst_u + total_n - 1] = 0x20;
                            layers.layers[4][dst_u + total_n - 1] = 0x20;
                            al2 = 0;
                            if (mark_len as i32) != -2 {
                                loop {
                                    layers.append(1, b" ");
                                    layers.append(5, b" ");
                                    al2 = al2.wrapping_add(1) & 0xffff;
                                    if !(al2 < (mark_len as u32).wrapping_add(2)) {
                                        break;
                                    }
                                }
                            }
                        } else {
                            layers.append(0, b"\x1e");
                            layers.append(0, &mark[..mark_len]);
                            layers.append(0, b"\x1f");
                            al2 = 0;
                            if (mark_len as i32) != -2 {
                                loop {
                                    for idx in 1..6 {
                                        layers.append(idx, b" ");
                                    }
                                    al2 = al2.wrapping_add(1) & 0xffff;
                                    if !(al2 < (mark_len as u32).wrapping_add(2)) {
                                        break;
                                    }
                                }
                            }
                        }
                        if lastw_valid {
                            al2 = (mark_len as u32).wrapping_add(al2).wrapping_add(2) & 0xffff;
                        }
                        al1 = (mark_len as u32).wrapping_add(al1).wrapping_add(2) & 0xffff;
                    } else {
                        break 'fail fill_rc;
                    }
                }
                if kind == 6 {
                    let (status, raw, _) = host.node_field(handle, id_b, 1, 1);
                    fill_rc = status;
                    if fill_rc >= 0x80000000 {
                        break 'fail fill_rc;
                    }
                    id130 = raw as u16;
                    if global.use_tn_tag != 0 && lastw_valid && id128 < id130 {
                        let lw = lastw_len;
                        if (((lw as i32).wrapping_mul(2).wrapping_sub(2)) as u32) < al2 {
                            layers.append(6, b"X");
                            let lw = c_str_len(buffers.lastw, "last tag");
                            append_lastw_inner(&mut layers, buffers.lastw, lw);
                            let lw = c_str_len(buffers.lastw, "last tag");
                            al1 = (((lw as i32).wrapping_mul(2).wrapping_sub(2)) as u32) & 0xffff;
                            while (al1 & 0xffff) < al2 {
                                layers.append(6, b"~");
                                al1 = (al1 & 0xffff).wrapping_add(1);
                            }
                            layers.append(6, b"Y");
                            let lw = c_str_len(buffers.lastw, "last tag");
                            append_lastw_inner(&mut layers, buffers.lastw, lw);
                        } else {
                            layers.append(6, b"X");
                            let lw = c_str_len(buffers.lastw, "last tag");
                            append_lastw_inner(&mut layers, buffers.lastw, lw);
                            layers.append(6, b"Y");
                            let lw = c_str_len(buffers.lastw, "last tag");
                            append_lastw_inner(&mut layers, buffers.lastw, lw);
                            let lw = c_str_len(buffers.lastw, "last tag");
                            if al2 < (((lw as i32).wrapping_mul(2).wrapping_sub(2)) as u32) {
                                loop {
                                    for idx in 0..6 {
                                        layers.append(idx, b"~");
                                    }
                                    al2 = al2.wrapping_add(1) & 0xffff;
                                    let lw = c_str_len(buffers.lastw, "last tag");
                                    if !(al2
                                        < (((lw as i32).wrapping_mul(2).wrapping_sub(2)) as u32))
                                    {
                                        break;
                                    }
                                }
                            }
                        }
                        lastw_valid = false;
                        al2 = 0;
                    }
                    al1 = al2;
                    let (status, _) = host.tag_text(handle, id_b, buffers.lastw);
                    fill_rc = status;
                    if fill_rc >= 0x80000000 {
                        break 'fail fill_rc;
                    }
                    lastw_len = c_str_len(buffers.lastw, "last tag");
                    lastw_valid = true;
                    let (status, raw, _) = host.node_field(handle, id_b, 2, 1);
                    id128 = raw as u16;
                    al2 = al1;
                    fill_rc = status;
                    if fill_rc >= 0x80000000 {
                        break 'fail fill_rc;
                    }
                }
                let (status, next) = host.next_node(handle, id_b);
                fill_rc = status;
                if fill_rc >= 0x80000000 {
                    break 'fail fill_rc;
                }
                id_b = next;
            }
            // Trailing tag flush.
            if global.use_tn_tag != 0 && !first {
                if !lastw_valid {
                    al1 = 0;
                    if al2 != 0 {
                        loop {
                            layers.append(6, b" ");
                            al1 = al1.wrapping_add(1) & 0xffff;
                            if !(al1 < al2) {
                                break;
                            }
                        }
                    }
                } else {
                    let lw = lastw_len;
                    if (((lw as i32).wrapping_mul(2).wrapping_sub(2)) as u32) < al2 {
                        layers.append(6, b"X");
                        let lw = c_str_len(buffers.lastw, "last tag");
                        append_lastw_inner(&mut layers, buffers.lastw, lw);
                        let lw = c_str_len(buffers.lastw, "last tag");
                        al1 = (((lw as i32).wrapping_mul(2).wrapping_sub(2)) as u32) & 0xffff;
                        while (al1 & 0xffff) < al2 {
                            layers.append(6, b"~");
                            al1 = (al1 & 0xffff).wrapping_add(1);
                        }
                        layers.append(6, b"Y");
                        let lw = c_str_len(buffers.lastw, "last tag");
                        append_lastw_inner(&mut layers, buffers.lastw, lw);
                    } else {
                        layers.append(6, b"X");
                        let lw = c_str_len(buffers.lastw, "last tag");
                        append_lastw_inner(&mut layers, buffers.lastw, lw);
                        layers.append(6, b"Y");
                        let lw = c_str_len(buffers.lastw, "last tag");
                        append_lastw_inner(&mut layers, buffers.lastw, lw);
                        let lw = c_str_len(buffers.lastw, "last tag");
                        if al2 < (((lw as i32).wrapping_mul(2).wrapping_sub(2)) as u32) {
                            loop {
                                for idx in 0..6 {
                                    layers.append(idx, b"~");
                                }
                                al2 = al2.wrapping_add(1) & 0xffff;
                                let lw = c_str_len(buffers.lastw, "last tag");
                                if !(al2 < (((lw as i32).wrapping_mul(2).wrapping_sub(2)) as u32)) {
                                    break;
                                }
                            }
                        }
                    }
                }
            }
            olen = 0;
            if n64 != 0 {
                layers.append6(b"\x11/+", b"   ");
                olen = 3;
            }
            if n60 != 0 {
                layers.append6(b"\x12\x10", b"  ");
                olen = olen.wrapping_add(2);
            }
            if global.use_tn_tag != 0 && !lastw_valid && olen != 0 {
                let mut c = 0u32;
                loop {
                    layers.append(6, b" ");
                    c = c.wrapping_add(1);
                    if !(c < olen) {
                        break;
                    }
                }
            }
            host.log_layer(global.env, 0, &tmp[..tmp_len], layers.slice(0));
            mirror_guest_staging(host, 0, layers.slice(0));
            fill_rc = host.set_layer_input(global.depes, 0, layers.slice(0));
            if fill_rc >= 0x80000000 {
                break 'fail fill_rc;
            }
            fill_rc = host.prepare_layer(global.dct, b"normal", layers.full(1));
            if fill_rc >= 0x80000000 {
                break 'fail fill_rc;
            }
            layers.recompute(1);
            host.log_layer(global.env, 1, &tmp[..tmp_len], layers.slice(1));
            mirror_guest_staging(host, 1, layers.slice(1));
            fill_rc = host.set_layer_input(global.depes, 1, layers.slice(1));
            if fill_rc >= 0x80000000 {
                break 'fail fill_rc;
            }
            host.log_layer(global.env, 2, &tmp[..tmp_len], layers.slice(2));
            mirror_guest_staging(host, 2, layers.slice(2));
            fill_rc = host.set_layer_input(global.depes, 2, layers.slice(2));
            if fill_rc >= 0x80000000 {
                break 'fail fill_rc;
            }
            host.log_layer(global.env, 3, &tmp[..tmp_len], layers.slice(3));
            mirror_guest_staging(host, 3, layers.slice(3));
            fill_rc = host.set_layer_input(global.depes, 3, layers.slice(3));
            if fill_rc >= 0x80000000 {
                break 'fail fill_rc;
            }
            host.log_layer(global.env, 4, &tmp[..tmp_len], layers.slice(4));
            mirror_guest_staging(host, 4, layers.slice(4));
            fill_rc = host.set_layer_input(global.depes, 4, layers.slice(4));
            if fill_rc >= 0x80000000 {
                break 'fail fill_rc;
            }
            host.log_layer(global.env, 5, &tmp[..tmp_len], layers.slice(5));
            mirror_guest_staging(host, 5, layers.slice(5));
            fill_rc = host.set_layer_input2(global.depes, 5, layers.slice(5));
            if fill_rc >= 0x80000000 {
                break 'fail fill_rc;
            }
            if global.use_tn_tag != 0 {
                host.log_layer(global.env, 6, &tmp[..tmp_len], layers.slice(6));
                mirror_guest_staging(host, 6, layers.slice(6));
                fill_rc = host.set_layer_input2(global.depes, 6, layers.slice(6));
                if fill_rc >= 0x80000000 {
                    break 'fail fill_rc;
                }
            }
            if global.foreign == 0 {
                fill_rc = host.run_layers(global.depes, &tmp[..tmp_len]);
                if fill_rc >= 0x80000000 {
                    return fill_rc;
                }
            } else {
                tmp[tmp_len..tmp_len + 7].copy_from_slice(b"foreign");
                tmp_len += 7;
                tmp[tmp_len] = 0;
                fill_rc = host.run_layers(global.depes, &tmp[..tmp_len]);
                if fill_rc >= 0x80000000 {
                    tmp[tmplen] = 0;
                    tmp_len = tmplen;
                    fill_rc = host.run_layers(global.depes, &tmp[..tmp_len]);
                    if fill_rc >= 0x80000000 {
                        return fill_rc;
                    }
                }
            }
            let (status, out_len) = host.layer_output(global.depes, 0, buffers.output);
            host.terminate_layer_text(out_len);
            fill_rc = status;
            if fill_rc >= 0x80000000 {
                break 'fail fill_rc;
            }
            let mut cnt = out_len;
            buffers.output[out_len as usize] = 0;
            let out_str = c_strlen(&buffers.output[..=out_len as usize]);
            let has_quote = c_strchr(buffers.output, out_str, 0x22).is_some();
            let cbuf = if has_quote { 0x22 } else { 0x27 };
            for b in buffers.output[..out_len as usize].iter_mut() {
                if *b == 0 {
                    break;
                }
                if *b == 0x13 {
                    *b = cbuf;
                }
            }
            // Compaction: delete `~` bytes and `0x1e..0x1f` spans.
            al2 = 0;
            if cnt != 0 {
                loop {
                    if buffers.output[al2 as usize] == b'~' {
                        if (al2 as u32) < u32::from(cnt) {
                            let p = al2 as usize;
                            let c = cnt as usize;
                            buffers.output.copy_within(p + 1..c + 1, p);
                        }
                        al2 = al2.wrapping_sub(1) & 0xffff;
                        cnt = cnt.wrapping_sub(1);
                        al2 = al2.wrapping_add(1) & 0xffff;
                    } else if buffers.output[al2 as usize] == 0x1e {
                        let mut slim = 1u32;
                        loop {
                            slim = slim.wrapping_add(1);
                            al2 = al2.wrapping_add(1) & 0xffff;
                            assert!((al2 as usize) < buffers.output.len(), "unterminated span");
                            if buffers.output[al2 as usize] == 0x1f {
                                break;
                            }
                        }
                        al2 = al2.wrapping_sub(slim);
                        let rc_pos = (al2.wrapping_add(1) & 0xffff) as usize;
                        if (rc_pos as u32) < u32::from(cnt) {
                            let c = cnt as usize;
                            buffers
                                .output
                                .copy_within(rc_pos + slim as usize..c + 1, rc_pos);
                        }
                        cnt = cnt.wrapping_sub(slim as u16);
                    } else {
                        al2 = al2.wrapping_add(1) & 0xffff;
                    }
                    if !(al2 < u32::from(cnt)) {
                        break;
                    }
                }
            }
            buffers.output[cnt as usize] = 0;
            host.log_layer(
                global.env,
                7,
                &tmp[..tmp_len],
                &buffers.output[..c_strlen(&buffers.output[..=cnt as usize])],
            );
            if global.clc_pipeline != 0 && cnt != 0 {
                for b in buffers.output[..cnt as usize].iter_mut() {
                    if *b == 0xad {
                        *b = b'~';
                    }
                }
            }
            // Trailing `?` handling with `\TQ\` insertion.
            let input_len = c_strlen(buffers.input);
            let last = if input_len == 0 {
                buffers.input_prefix
            } else {
                buffers.input[input_len - 1]
            };
            let out_full = c_str_len(buffers.output, "output");
            if last == b'?' && !c_strstr(&buffers.output[..out_full], b"\\TQ\\") {
                let mut pos = u32::from(cnt);
                let mut lvl = 0i32;
                let mut tc = 0u8;
                if pos != 0 {
                    loop {
                        let ch = buffers.output[pos as usize];
                        if ch != b' ' && ch != 0 {
                            break;
                        }
                        pos = pos.wrapping_sub(1) & 0xffff;
                        if pos == 0 {
                            break;
                        }
                    }
                    if pos > 4 && buffers.output[pos as usize] == b'\\' {
                        let c1 = buffers.output[(pos - 1) as usize];
                        let is_ta = c1 == b'A';
                        let c1 = if is_ta {
                            buffers.output[(pos - 3) as usize]
                        } else {
                            c1
                        };
                        if is_ta && c1 == b'T' {
                            let c2 = buffers.output[(pos - 4) as usize];
                            let is_s = c2 == b'/';
                            let c2 = if is_s {
                                buffers.output[(pos - 5) as usize]
                            } else {
                                c2
                            };
                            if is_s && c2 == b'\\' {
                                tc = buffers.output[(pos - 2) as usize];
                                buffers.output[(pos - 1) as usize] = b'Q';
                            }
                        }
                    }
                    if pos != 0 {
                        let c0 = buffers.output[pos as usize];
                        if pos > 4 && c0 == b'\\' {
                            let c1 = buffers.output[(pos - 1) as usize];
                            let is_ta = c1 == b'A';
                            let c1 = if is_ta {
                                buffers.output[(pos - 3) as usize]
                            } else {
                                c1
                            };
                            if is_ta && c1 == b'T' {
                                let c2 = buffers.output[(pos - 4) as usize];
                                let is_s = c2 == b'/';
                                let c2 = if is_s {
                                    buffers.output[(pos - 5) as usize]
                                } else {
                                    c2
                                };
                                if is_s && c2 == b'\\' {
                                    tc = buffers.output[(pos - 2) as usize];
                                    buffers.output[(pos - 1) as usize] = b'Q';
                                }
                            }
                        }
                        loop {
                            if lvl > 2 {
                                // ROM 0x1f7924: this exit runs `pos++`
                                // before the space skip (like the `#`
                                // exit), it does not stop on the byte.
                                pos = pos.wrapping_add(1) & 0xffff;
                                break;
                            }
                            let at_ta5 = pos >= 5 && buffers.output[pos as usize] == b'\\';
                            let at_ta4 = pos >= 4 && buffers.output[pos as usize] == b'\\';
                            if at_ta5 {
                                let c0 = buffers.output[(pos - 1) as usize];
                                let not_ta = c0 != b'A';
                                let c0 = if !not_ta {
                                    buffers.output[(pos - 3) as usize]
                                } else {
                                    c0
                                };
                                if !not_ta && c0 == b'T' {
                                    let c0 = buffers.output[(pos - 4) as usize];
                                    let is_s = c0 == b'/';
                                    let c0 = if is_s {
                                        buffers.output[(pos - 5) as usize]
                                    } else {
                                        c0
                                    };
                                    if !is_s || c0 != b'\\' {
                                        let c0 = buffers.output[pos as usize];
                                        if c0 == b' ' || c0 == b'-' {
                                            lvl += 1;
                                        }
                                        if c0 == b'#' {
                                            if pos != 0 {
                                                break;
                                            }
                                            break;
                                        }
                                    } else {
                                        tc = buffers.output[(pos - 2) as usize];
                                        buffers.output[(pos - 1) as usize] = b'Q';
                                    }
                                } else if !at_ta4 {
                                    let c0 = buffers.output[pos as usize];
                                    if c0 == b' ' || c0 == b'-' {
                                        lvl += 1;
                                    }
                                    if c0 == b'#' {
                                        if pos != 0 {
                                            break;
                                        }
                                        break;
                                    }
                                } else {
                                    let c0 = buffers.output[(pos - 1) as usize];
                                    let is_ta = c0 == b'A';
                                    let c0 = if is_ta {
                                        buffers.output[(pos - 3) as usize]
                                    } else {
                                        c0
                                    };
                                    if is_ta
                                        && c0 == b'T'
                                        && buffers.output[(pos - 4) as usize] == b'\\'
                                    {
                                        buffers.output[(pos - 1) as usize] = b'Q';
                                    }
                                }
                            } else if !at_ta4 {
                                let c0 = buffers.output[pos as usize];
                                if c0 == b' ' || c0 == b'-' {
                                    lvl += 1;
                                }
                                if c0 == b'#' {
                                    if pos != 0 {
                                        break;
                                    }
                                    break;
                                }
                            } else {
                                let c0 = buffers.output[(pos - 1) as usize];
                                let is_ta = c0 == b'A';
                                let c0 = if is_ta {
                                    buffers.output[(pos - 3) as usize]
                                } else {
                                    c0
                                };
                                if is_ta
                                    && c0 == b'T'
                                    && buffers.output[(pos - 4) as usize] == b'\\'
                                {
                                    buffers.output[(pos - 1) as usize] = b'Q';
                                }
                            }
                            pos = pos.wrapping_sub(1) & 0xffff;
                            if pos == 0 {
                                break;
                            }
                        }
                    }
                }
                while buffers.output[pos as usize] == b' ' || buffers.output[pos as usize] == b'#' {
                    pos = pos.wrapping_add(1) & 0xffff;
                }
                let mut inserted = false;
                if tc == 0 {
                    inserted = true;
                    let mut al = (u32::from(cnt).wrapping_add(4)) & 0xffff;
                    loop {
                        al = al & 0xffff;
                        if !(pos < al && al > 3) {
                            break;
                        }
                        buffers.output[al as usize] = buffers.output[(al - 4) as usize];
                        al = al.wrapping_sub(1);
                    }
                    cnt = cnt.wrapping_add(4);
                    buffers.output[pos as usize] = b'\\';
                    buffers.output[(pos + 1) as usize] = b'T';
                    buffers.output[(pos + 2) as usize] = b'Q';
                    buffers.output[(pos + 3) as usize] = b'\\';
                } else if pos != 0 {
                    let mut al = (u32::from(cnt).wrapping_add(11)) & 0xffff;
                    loop {
                        al = al & 0xffff;
                        if !(pos < al && al > 10) {
                            break;
                        }
                        buffers.output[al as usize] = buffers.output[(al - 11) as usize];
                        al = al.wrapping_sub(1);
                    }
                    cnt = cnt.wrapping_add(11);
                    buffers.output[pos as usize] = b'\\';
                    buffers.output[(pos + 1) as usize] = b'/';
                    buffers.output[(pos + 2) as usize] = b'T';
                    buffers.output[(pos + 3) as usize] = tc;
                    buffers.output[(pos + 4) as usize] = b'A';
                    buffers.output[(pos + 5) as usize] = b'\\';
                    buffers.output[(pos + 6) as usize] = b'\\';
                    buffers.output[(pos + 7) as usize] = b'T';
                    buffers.output[(pos + 8) as usize] = tc;
                    buffers.output[(pos + 9) as usize] = b'Q';
                    buffers.output[(pos + 10) as usize] = b'\\';
                    inserted = true;
                }
                if inserted {
                    // Commit the tag insertion to the guest layer-0
                    // row the same way the C's in-place edit leaves
                    // it: the row holds the shifted bytes and the tag,
                    // and the final walk splits it into tokens from
                    // there.
                    commit_layer_output(host, buffers.output, cnt);
                }
            }
            // Final walk: write output tokens back.
            let (status, id) = host.first_node(handle, 2, u32::from(id_a));
            fill_rc = status;
            if fill_rc >= 0x80000000 || id == 0 {
                break 'fail fill_rc;
            }
            id_b = id;
            let mut opos = 0usize;
            loop {
                let (status, raw, _) = host.node_field(handle, id_b, 0, 1);
                fill_rc = status;
                if fill_rc >= 0x80000000 {
                    break;
                }
                let kind = raw as u8;
                if kind == 4
                    || kind == 9
                    || kind == 10
                    || kind == 11
                    || kind == 13
                    || kind == 12
                    || kind == 14
                {
                    let (status, _) = host.word_text(handle, id_b, buffers.word);
                    fill_rc = status;
                    if fill_rc >= 0x80000000 {
                        break;
                    }
                    let word_len = c_str_len(buffers.word, "word");
                    if word_len != 0 {
                        let has_dash_star = c_strstr(buffers.word, b"-*");
                        if has_dash_star {
                            let (status, raw, _) = host.node_field(handle, id_b, 1, 1);
                            fill_rc = status;
                            if fill_rc >= 0x80000000 {
                                break;
                            }
                            let beg = raw as u16;
                            let (status, raw, _) = host.node_field(handle, id_b, 2, 1);
                            fill_rc = status;
                            if fill_rc >= 0x80000000 {
                                break;
                            }
                            let end = raw as u16;
                            assert!(
                                (beg as usize) <= input_end,
                                "dash span starts past input end"
                            );
                            let mut from = beg as usize;
                            while let Some(rel) = c_strchr(
                                &buffers.input[from..=input_end],
                                input_end + 1 - from,
                                b'-',
                            ) {
                                let at = from + rel;
                                if at >= end as usize {
                                    break;
                                }
                                buffers.input[at] = b'_';
                                from = at + 1;
                            }
                        }
                        let mut tpos = opos;
                        let fk = loop {
                            let cc = buffers.output[tpos];
                            if cc == 0 {
                                break tpos;
                            }
                            if cc == b' ' || cc == b'#' {
                                buffers.output[tpos] = 0;
                                break tpos + 1;
                            }
                            tpos += 1;
                        };
                        if buffers.output[opos] == b'-' {
                            opos += 1;
                        }
                        let d = c_strlen(&buffers.output[opos..]);
                        olen = ((d + 1) & 0xffff) as u32;
                        if buffers.output[opos + (olen.wrapping_sub(1) as usize)] == b'-' {
                            olen = olen.wrapping_sub(1) & 0xffff;
                        }
                        let span = &buffers.output[opos..opos + olen as usize];
                        terminate_guest_token(host);
                        fill_rc = host.set_pred(handle, id_b, olen, span);
                        if fill_rc >= 0x80000000 {
                            break;
                        }
                        opos = fk;
                        while buffers.output[opos] == b' ' || buffers.output[opos] == b'#' {
                            opos += 1;
                        }
                    }
                }
                let (status, next) = host.next_node(handle, id_b);
                fill_rc = status;
                if fill_rc >= 0x80000000 {
                    break;
                }
                id_b = next;
                if id_b == 0 {
                    break;
                }
            }
            break 'fail fill_rc;
        }
        break 'fail status;
    };
    if global.f20 != 0 {
        host.free_layer(global.env, global.f20);
    }
    global.f20 = 0;
    if global.f24 != 0 {
        host.free_layer(global.env, global.f24);
    }
    global.f24 = 0;
    if global.f28 != 0 {
        host.free_layer(global.env, global.f28);
    }
    global.f28 = 0;
    if global.f2c != 0 {
        host.free_layer(global.env, global.f2c);
    }
    global.f2c = 0;
    if global.f30 != 0 {
        host.free_layer(global.env, global.f30);
    }
    global.f30 = 0;
    if global.f34 != 0 {
        host.free_layer(global.env, global.f34);
    }
    global.f34 = 0;
    if global.f38 != 0 {
        host.free_layer(global.env, global.f38);
    }
    global.f38 = 0;
    rc
}

/// One `FeatureRules` table from `VoiceFeatures`: `count` entries of `stride`
/// bytes at `entries`.
pub struct ProsodyRules<'a> {
    pub count: u16,
    pub stride: u16,
    pub entries: &'a [u8],
}

/// Borrowed `TtsMarker`: only kind/active/position are read by `fn_2902d8`.
#[derive(Clone, Copy)]
pub struct ProsodyMarker {
    pub kind: u16,
    pub active: u32,
    pub position: u32,
}

/// Borrowed `VoiceFeatures` tables read by `fn_2902d8`.
///
/// `modifiers`, the voice/set names and the rule strings are NUL-terminated
/// like the originals. `pitch_ranges` is a byte view of the `u16` table;
/// `modifier_pitch_rules`/`phrase_pitch_rules` are byte views of the 12/16
/// byte rule records.
pub struct ProsodyVoice<'a> {
    pub categories: &'a [u8],
    pub vowel_types: &'a [u8],
    pub language_classes: &'a [u8; 256],
    pub phoneme_classes: &'a [u8; 256],
    pub modifiers: &'a [u8],
    pub voice_names: &'a [Option<&'a [u8]>],
    pub markup: i32,
    pub context_enabled: i32,
    pub language: i32,
    pub feature_widths: &'a [u8; 60],
    pub acoustic_count: u16,
    pub acoustic_defaults: &'a [u8],
    pub category_feature: i32,
    pub phoneme_feature: i32,
    pub voice_feature: i32,
    pub modifier_feature: i32,
    pub number_feature: i32,
    pub alternate_number_feature: i32,
    pub stress_feature: i32,
    pub phrase_feature: i32,
    pub position_feature: i32,
    pub length_feature: i32,
    pub duration_count: u16,
    pub duration_defaults: &'a [u8],
    pub pitch_feature: i32,
    pub alternate_pitch_feature: i32,
    pub phoneme_sets: &'a [Option<&'a [u8]>],
    pub voice_sets: &'a [Option<&'a [u8]>],
    pub acoustic_rules: ProsodyRules<'a>,
    pub secondary_acoustic_rules: ProsodyRules<'a>,
    pub duration_rules: ProsodyRules<'a>,
    pub alternate_duration_rules: ProsodyRules<'a>,
    pub mode_rules: ProsodyRules<'a>,
    pub pitch_ranges: &'a [u8],
    pub modifier_pitch_rules: &'a [u8],
    pub phrase_pitch_rules: &'a [u8],
    pub category_context: &'a [u8],
    pub category_table: u32,
    pub phoneme_context: &'a [u8],
    pub phoneme_table: &'a [u32],
}

/// Caller-owned `TtsFeatures` planes for `fn_2902d8`.
///
/// `bytes[f]`/`words[f]` are `None` where the original pointer is null.
/// `pitch` holds 4 `u16` per entry, `contexts` 9 bytes per entry, `timings`
/// 2 `i32` per entry (flat, as `fn_29442c` indexes it). `bytes[59]` is the
/// tap carrier `fn_29442c` reads; `carrier_scratch` must hold the carrier
/// plus its terminator. `bytes[0]` starts with one guard byte ahead of
/// logical index 0 (a stale scan index can read `bytes[0][-1]`) and
/// `bytes[1]` with four (`fn_28ff3c` folds weights at `index - 1`).
pub struct ProsodyFeatures<'a> {
    pub bytes: [Option<&'a mut [u8]>; 60],
    pub words: [Option<&'a mut [u16]>; 60],
    pub markers: &'a [ProsodyMarker],
    pub marker_count: u32,
    pub count: i16,
    pub length: i16,
    pub timing_count: i16,
    pub pitch: Option<&'a mut [u16]>,
    pub contexts: Option<&'a mut [u8]>,
    pub acoustic: Option<&'a mut [i32]>,
    pub duration: Option<&'a mut [i32]>,
    pub alternate_duration: Option<&'a mut [i32]>,
    pub modes: Option<&'a mut [u8]>,
    pub timings: Option<&'a mut [i32]>,
    pub carrier_timings: &'a [i32],
    pub carrier_scratch: &'a mut [u8],
}

/// One dumped feature plane for `fn_173b00`.
///
/// `PitchBytes` covers the pitch plane dumped with a width other than 2:
/// the host serializes the first `len` little-endian bytes of the `u16`
/// plane, exactly the bytes the C `buffer`/`count` pair spans.
pub enum DumpBytes<'a> {
    B8(&'a [u8]),
    B16(&'a [u16]),
    B32(&'a [i32]),
    PitchBytes {
        pitch: &'a [u16],
        len: usize,
    },
    /// Null C `buffer` with its byte length, passed through untouched.
    Null(usize),
}

/// Engine calls reached by `fn_2902d8`.
///
/// `compute_features` runs the voice `compute_features` callback, which
/// writes one byte to each of `out0`/`out1`; `category` selects the category
/// (`kind` 0) versus phoneme (`kind` 1) call. `log_level` is `fn_173fd8` and
/// `write_output` is `fn_173b00`.
pub trait ProsodyHost: TimingHost {
    fn compute_features(
        &mut self,
        category: bool,
        context: &[u8],
        table: &[u32],
        index: i32,
        data: &ProsodyFeatures<'_>,
        out0: &mut u8,
        out1: &mut u8,
    );
    fn log_level(&mut self) -> u32;
    fn write_output(
        &mut self,
        request: u32,
        kind: u32,
        name: &[u8],
        mime: &[u8],
        bytes: DumpBytes<'_>,
    );
}

/// Plane base offsets: `bytes[0]` carries one guard byte ahead of index 0
/// (a stale scan index can read `bytes[0][-1]`), `bytes[1]` four (the
/// `fn_28ff3c` weights fold); every other plane starts at its index 0.
fn plane_base(f: usize) -> usize {
    match f {
        0 => 1,
        1 => 4,
        _ => 0,
    }
}

fn feat_get(data: &ProsodyFeatures<'_>, f: usize, k: usize) -> u8 {
    data.bytes[f].as_deref().unwrap()[plane_base(f).wrapping_add(k)]
}

fn feat_set(data: &mut ProsodyFeatures<'_>, f: usize, k: usize, v: u8) {
    data.bytes[f].as_deref_mut().unwrap()[plane_base(f).wrapping_add(k)] = v;
}

fn feat_present(data: &ProsodyFeatures<'_>, f: usize) -> bool {
    data.bytes[f].is_some()
}

fn word_get(data: &ProsodyFeatures<'_>, f: usize, k: usize) -> u16 {
    data.words[f].as_deref().unwrap()[k]
}

fn word_set(data: &mut ProsodyFeatures<'_>, f: usize, k: usize, v: u16) {
    data.words[f].as_deref_mut().unwrap()[k] = v;
}

fn read_u16(bytes: &[u8], at: usize) -> u16 {
    u16::from_le_bytes([bytes[at], bytes[at + 1]])
}

fn read_i16(bytes: &[u8], at: usize) -> i16 {
    i16::from_le_bytes([bytes[at], bytes[at + 1]])
}

fn read_i32(bytes: &[u8], at: usize) -> i32 {
    i32::from_le_bytes([bytes[at], bytes[at + 1], bytes[at + 2], bytes[at + 3]])
}

fn find_byte_in(text: &[u8], byte: u8) -> Option<usize> {
    crate::resources::find_byte(text, u32::from(byte))
}

fn prosody_strstr(haystack: &[u8], needle: &[u8]) -> bool {
    timing_strstr(haystack, needle).is_some()
}

/// `fn_2902d8` rule matcher shared by all five rule blocks.
///
/// Checks the current/next/previous/+2 phoneme sets and the two voice sets
/// exactly like the duration block; `output`/`previous`/`count` are the
/// precomputed neighbor indices.
fn prosody_rule_matches(
    voice: &ProsodyVoice<'_>,
    data: &ProsodyFeatures<'_>,
    rules: &ProsodyRules<'_>,
    entry: usize,
    output: usize,
    previous: i32,
    count: i32,
) -> bool {
    use crate::resources::find_byte;
    let base = entry * usize::from(rules.stride);
    let set = |slot: usize| -> &[u8] {
        let id = rules.entries[base + slot] as usize;
        voice.phoneme_sets[id].unwrap()
    };
    let vset = |slot: usize| -> Option<&[u8]> {
        let id = rules.entries[base + slot] as usize;
        voice.voice_sets.get(id).copied().flatten()
    };
    let phon = |s: &[u8], b: u8| -> bool { find_byte(s, u32::from(b)).is_some() };
    if !phon(set(0), feat_get(data, 2, output)) {
        return false;
    }
    if !phon(set(1), feat_get(data, 2, output + 1)) {
        return false;
    }
    if output == 0 {
        if !phon(set(2), 0x23) {
            return false;
        }
    } else if !phon(set(2), feat_get(data, 2, previous as usize)) {
        return false;
    }
    let tail_ok = if i32::from(data.count) - 2 != output as i32 {
        phon(set(3), feat_get(data, 2, count as usize))
    } else {
        phon(set(3), 0x23)
    };
    if !tail_ok {
        return false;
    }
    if voice.voice_feature < 0 {
        return true;
    }
    for (slot, at) in [(4, output), (5, output + 1)] {
        match vset(slot) {
            None => {}
            Some(s) => {
                let b = feat_get(data, 29, at).wrapping_add(0x30);
                if find_byte(s, u32::from(b)).is_none() {
                    return false;
                }
            }
        }
    }
    true
}

/// `fn_2902d8` multiply application shared by the duration, alternate
/// duration, acoustic and secondary acoustic blocks: `n == 0` scales by 5,
/// `n == 1` clears, else scales by `n`.
fn prosody_apply_scale(slot: &mut i32, n: u32) {
    if n == 0 {
        *slot = slot.wrapping_mul(5);
    } else if n == 1 {
        *slot = 0;
    } else {
        *slot = slot.wrapping_mul(n as i32);
    }
}

/// Logical `bytes[1]` read through the guard-prefixed plane.
fn w1(data: &ProsodyFeatures<'_>, k: usize) -> u8 {
    feat_get(data, 1, k)
}

fn w1set(data: &mut ProsodyFeatures<'_>, k: usize, v: u8) {
    feat_set(data, 1, k, v);
}

fn render_u32(mut value: u32, out: &mut [u8]) -> usize {
    let mut digits = [0u8; 10];
    let mut count = 0;
    loop {
        digits[count] = b'0' + (value % 10) as u8;
        count += 1;
        value /= 10;
        if value == 0 {
            break;
        }
    }
    for i in 0..count {
        out[i] = digits[count - 1 - i];
    }
    out[count] = 0;
    count
}

/// `fn_2902d8` backward boundary scan shared by the `bytes[11]`/`bytes[20]`
/// tail: walks back from `previous` for `*`/`%`/`#`, refines a `*`/`#`
/// hit with a forward class scan, then maps `bytes[11]` to `bytes[20]`
/// (1->3, 2->1, 3->0, 4->2, 5->4, anything else unwritten).
fn prosody_boundary_tail(
    data: &mut ProsodyFeatures<'_>,
    classes: &[u8; 256],
    o: usize,
    previous: i32,
) {
    if feat_get(data, 11, o) == 2 {
        let mut position = previous as i16;
        let (address, tag) = loop {
            let address = position as i32;
            let tag = feat_get(data, 0, address as usize);
            if address < 1 {
                break (address, tag);
            }
            if tag == 0x2a || tag == 0x23 {
                break (address, tag);
            }
            if tag == 0x25 {
                break (address, tag);
            }
            position = position.wrapping_sub(1);
        };
        if tag == 0x2a || tag == 0x25 || tag == 0x23 {
            if !(address == 0 && tag == 0x23) {
                if tag == 0x25 {
                    feat_set(data, 11, o, 3);
                } else {
                    position = position.wrapping_sub(1);
                    loop {
                        let j = position as i32;
                        if !(j > 0) {
                            break;
                        }
                        let b = feat_get(data, 0, j as usize);
                        if classes[usize::from(b)] != 255 {
                            break;
                        }
                        if j == 0 && feat_get(data, 0, 0) == 0x23 {
                            feat_set(data, 11, o, 1);
                        } else if b == 0x25 {
                            feat_set(data, 11, o, 3);
                        }
                        position = position.wrapping_add(1);
                    }
                }
            } else {
                feat_set(data, 11, o, 1);
                feat_set(data, 20, o, 3);
                return;
            }
        }
        match feat_get(data, 11, o) {
            3 => feat_set(data, 20, o, 0),
            2 => feat_set(data, 20, o, 1),
            4 => feat_set(data, 20, o, 2),
            1 => feat_set(data, 20, o, 3),
            5 => feat_set(data, 20, o, 4),
            _ => {}
        }
    } else {
        match feat_get(data, 11, o) {
            3 => feat_set(data, 20, o, 0),
            2 => feat_set(data, 20, o, 1),
            4 => feat_set(data, 20, o, 2),
            1 => feat_set(data, 20, o, 3),
            5 => feat_set(data, 20, o, 4),
            _ => {}
        }
    }
}

/// fn_2902d8 / EU fn_290494 / KR fn_290540.
///
/// Builds the per-phone prosody features: marker weights, voice escapes,
/// tap-carrier timing alignment, markup stripping, `#[pause]` folding, the
/// forward phone scan, the backward phrase scan, context windows, default
/// acoustic/duration fills, the five rule blocks, pitch/category/phoneme
/// features, `-2` boundary compaction and the optional feature dump.
/// Returns the `fn_29442c` status on the success path.
#[allow(clippy::too_many_lines)]
pub fn build_phone_features(
    allocator: u32,
    log: u32,
    request: u32,
    voice: &ProsodyVoice<'_>,
    data: &mut ProsodyFeatures<'_>,
    host: &mut impl ProsodyHost,
) -> u32 {
    let _ = log;
    let mut output: i32 = 0;
    let mut found = 0;
    let mut default_voice: u8 = 2;
    let mut current_voice: u8 = 0;
    let mut phrase_count: u8 = 0;
    let mut count: u32 = 0;
    let mut first_word = true;
    let mut first_sentence = true;
    let mut first_phrase = true;
    let mut pending_weight: u32 = 0;
    let mut stress_start = 1;
    let mut index: u32 = 0;
    let mut phrase_started = 0;
    let mut item: u32 = 0;
    let mut first_vowel: u32 = 1;
    let mut previous_vowel: i32 = 0;
    let mut j: i32 = 0;
    if data.length <= 0 {
        return 0x84e02015;
    }
    if feat_present(data, 1) {
        let mut marker_base: u32 = 0;
        let len = data.length as usize;
        data.bytes[1].as_deref_mut().unwrap()[4..4 + len + 1].fill(0);
        let mut first_idx: u32 = 0;
        for (n, marker) in data
            .markers
            .iter()
            .take(data.marker_count as usize)
            .enumerate()
        {
            if marker.kind == 0x4000 && marker.active != 0 {
                marker_base = marker.position;
                first_idx = n as u32;
            }
        }
        for _ in 0..first_idx {
            let v = w1(data, 0).wrapping_add(1);
            w1set(data, 0, v);
        }
        for marker in data
            .markers
            .iter()
            .take(data.marker_count as usize)
            .skip(first_idx as usize)
        {
            let mut offset = marker.position.wrapping_sub(marker_base);
            if offset > len as u32 {
                offset = len as u32;
            }
            let v = w1(data, offset as usize).wrapping_add(1);
            w1set(data, offset as usize, v);
        }
    }
    if voice.voice_feature >= 0 {
        let mut slot: u32 = 0;
        let mut matched = false;
        for _ in 0..256 {
            let name = voice
                .voice_names
                .get((slot & 0xff) as usize)
                .copied()
                .flatten();
            let Some(name) = name else { break };
            if crate::resources::compare_strings(name, b"TS\0") == 0 {
                matched = true;
                break;
            }
            slot = (slot + 1) & 0xff;
        }
        index = if matched { slot } else { 0 };
        current_voice = index as u8;
    }
    let selector = VoiceSelector {
        names: voice.voice_names,
        voice_feature: voice.voice_feature,
    };
    if feat_get(data, 0, 0) == b'\\' {
        let b0 = data.bytes[0].take();
        let b1 = data.bytes[1].take();
        let mut b0 = b0;
        let mut b1 = b1;
        select_voice(
            &selector,
            &mut b0.as_deref_mut().unwrap()[1..],
            b1.as_deref_mut(),
            &mut data.length,
            0,
            index,
            &mut current_voice,
            &mut default_voice,
        );
        data.bytes[0] = b0;
        data.bytes[1] = b1;
    }
    // Leading `#` strip loop; the markup `<` branch enters the body once
    // unconditionally like the C `goto`.
    let markup_entry = voice.markup != 0 && feat_get(data, 0, 0) == b'<';
    if markup_entry {
        found = 1;
    }
    let mut first_strip = markup_entry;
    loop {
        if !first_strip {
            if feat_get(data, 0, 0) == b'#' {
                break;
            }
        }
        first_strip = false;
        if feat_present(data, 1) {
            let v = w1(data, 0).wrapping_add(w1(data, 1));
            w1set(data, 0, v);
        }
        let mut position = data.length;
        if position > 0 {
            let mut i: i32 = 0;
            loop {
                let next = (i + 1) as usize;
                position = position.wrapping_sub(1);
                let v = feat_get(data, 0, next);
                feat_set(data, 0, i as usize, v);
                if feat_present(data, 1) && i != 0 {
                    let v = w1(data, next);
                    w1set(data, i as usize, v);
                }
                i += 1;
                if position == 0 {
                    break;
                }
            }
        }
        data.length = data.length.wrapping_sub(1);
    }
    let mut i: i32 = 0;
    if data.length > 0 {
        loop {
            let at = i as usize;
            if feat_get(data, 0, at) != b'\\' {
                feat_set(data, 29, at, current_voice);
                i = (i.wrapping_add(1) as i16) as i32;
            } else {
                let b0 = data.bytes[0].take();
                let b1 = data.bytes[1].take();
                let mut b0 = b0;
                let mut b1 = b1;
                let consumed = select_voice(
                    &selector,
                    &mut b0.as_deref_mut().unwrap()[1..],
                    b1.as_deref_mut(),
                    &mut data.length,
                    i,
                    index,
                    &mut current_voice,
                    &mut default_voice,
                );
                data.bytes[0] = b0;
                data.bytes[1] = b1;
                if consumed == 0 {
                    feat_set(data, 29, at, current_voice);
                    i = (i.wrapping_add(1) as i16) as i32;
                }
            }
            if !(i < i32::from(data.length)) {
                break;
            }
        }
    }
    let b0 = data.bytes[0].take();
    let b1 = data.bytes[1].take();
    let b29 = data.bytes[29].take();
    let b59 = data.bytes[59].take();
    let t = data.timings.take();
    let mut b0 = b0;
    let mut b1 = b1;
    let mut b29 = b29;
    let mut t = t;
    let mut timing = PhonemeTiming {
        phonemes: &mut b0.as_deref_mut().unwrap()[1..],
        weights: b1.as_deref_mut().map(|s| &mut s[4..]),
        attributes: b29.as_deref_mut().unwrap(),
        timings: t.as_deref_mut().unwrap(),
        carrier_timings: data.carrier_timings,
        carrier: b59.as_deref(),
        carrier_scratch: &mut *data.carrier_scratch,
    };
    let result = align_timings(
        allocator,
        log,
        request,
        voice.phoneme_classes,
        &mut timing,
        &mut data.length,
        host,
    );
    data.bytes[0] = b0;
    data.bytes[1] = b1;
    data.bytes[29] = b29;
    data.bytes[59] = b59;
    data.timings = t;
    if result >= 0x80000000 {
        return result;
    }
    if voice.markup != 0 {
        i = 0;
        output = 0;
        loop {
            if !(i < i32::from(data.length)) {
                break;
            }
            let at = i as usize;
            index = u32::from(feat_get(data, 0, at));
            if index == u32::from(b'<') {
                found = 1;
                let len = data.length as usize;
                for j in 0..=len - at {
                    let v = feat_get(data, 0, at + j + 1);
                    feat_set(data, 0, at + j, v);
                    let v = feat_get(data, 29, at + j + 1);
                    feat_set(data, 29, at + j, v);
                }
                i = (i.wrapping_sub(1) as i16) as i32;
                data.length = data.length.wrapping_sub(1);
            } else if index == u32::from(b'>') {
                if found == 0 {
                    return 0x84e02015;
                }
                found = 0;
                let len = data.length as usize;
                for j in 0..=len - at {
                    let v = feat_get(data, 29, at + j + 1);
                    feat_set(data, 29, at + j, v);
                    let v = feat_get(data, 0, at + j + 1);
                    feat_set(data, 0, at + j, v);
                }
                i = (i.wrapping_sub(1) as i16) as i32;
                data.length = data.length.wrapping_sub(1);
            } else if voice.phoneme_classes[index as usize] != 255 {
                feat_set(data, 55, output as usize, if found == 1 { 1 } else { 0 });
                output += 1;
            }
            i = (i.wrapping_add(1) as i16) as i32;
        }
    }
    i = 0;
    output = 1;
    word_set(data, 3, 0, 0);
    word_set(data, 3, 1, 0);
    if data.length > 1 {
        loop {
            let at = output as usize;
            let prev = feat_get(data, 0, at - 1);
            found = i32::from(prev == b'#');
            let mut cur = prev;
            if found != 0 {
                cur = feat_get(data, 0, at);
            }
            if found != 0 && cur == b'[' {
                let mut number = [0u8; 40];
                j = (output + 1) as i16 as i32;
                let mut address: i32 = 0;
                cur = feat_get(data, 0, j as usize);
                while cur != b']' {
                    let v = feat_get(data, 0, j as usize);
                    j = (j.wrapping_add(1) as i16) as i32;
                    number[address as usize] = v;
                    address = (address.wrapping_add(1) as i16) as i32;
                    if i32::from(data.length) == j || address == 10 {
                        return 0x84e02015;
                    }
                    cur = feat_get(data, 0, j as usize);
                }
                number[address as usize] = 0;
                let mut table = crate::pronunciation::parse_int(&number);
                if table < 0 {
                    table = 0;
                } else if table > 0xffff {
                    table = 65535;
                }
                let position = (address as i16).wrapping_add(2);
                let address_idx = (output - 1) as usize;
                index = position as u32;
                word_set(data, 3, i as usize, table as i16 as u16);
                // C L341: j = address (output-1); fold re-scans from same output.
                j = ((output - 1) as i16) as i32;
                if !feat_present(data, 1) {
                    let mut table_it = output;
                    if output < i32::from(data.length) - index as i32 {
                        loop {
                            let offset = (table_it + index as i32) as usize;
                            let v = feat_get(data, 0, offset);
                            feat_set(data, 0, table_it as usize, v);
                            if voice.voice_feature >= 0 {
                                let v = feat_get(data, 29, offset);
                                feat_set(data, 29, table_it as usize, v);
                            }
                            table_it += 1;
                            if !(table_it < i32::from(data.length) - index as i32) {
                                break;
                            }
                        }
                    }
                } else {
                    let mut scan: i16;
                    if (index as i32) < 2 {
                        scan = 0;
                    } else {
                        scan = (!index & 1) as i16;
                        if (!index & 1) == 1 {
                            let v = w1(data, at).wrapping_add(w1(data, at + 1));
                            w1set(data, at, v);
                        }
                    }
                    scan = scan.wrapping_add(output as i16).wrapping_add(1);
                    loop {
                        let table_it = scan as i32;
                        if !(table_it < output + index as i32) {
                            break;
                        }
                        let v = w1(data, at).wrapping_add(w1(data, table_it as usize));
                        w1set(data, at, v);
                        let v = v.wrapping_add(w1(data, table_it as usize + 1));
                        w1set(data, at, v);
                        scan = scan.wrapping_add(2);
                    }
                    if i32::from(data.length) - index as i32 != output {
                        let mut table_it = output;
                        if output < i32::from(data.length) - index as i32 {
                            loop {
                                let offset = (table_it + index as i32) as usize;
                                let v = feat_get(data, 0, offset);
                                feat_set(data, 0, table_it as usize, v);
                                let v = w1(data, offset);
                                if table_it == output {
                                    w1set(
                                        data,
                                        table_it as usize,
                                        v.wrapping_add(w1(data, table_it as usize)),
                                    );
                                } else {
                                    w1set(data, table_it as usize, v);
                                }
                                if voice.voice_feature >= 0 {
                                    let v = feat_get(data, 29, offset);
                                    feat_set(data, 29, table_it as usize, v);
                                }
                                table_it += 1;
                                if !(table_it < i32::from(data.length) - index as i32) {
                                    break;
                                }
                            }
                        }
                    } else {
                        let v = w1(data, address_idx).wrapping_add(w1(data, at));
                        w1set(data, address_idx, v);
                    }
                }
                data.length = data.length.wrapping_sub(position);
                output = (j.wrapping_add(1) as i16) as i32;
            } else {
                j = output;
                if voice.phoneme_classes[usize::from(feat_get(data, 0, at))] != 255 {
                    i = (i.wrapping_add(1) as i16) as i32;
                    word_set(data, 3, i as usize, 0);
                }
                output = (j.wrapping_add(1) as i16) as i32;
            }
            if output < i32::from(data.length) {
                continue;
            }
            break;
        }
    }
    word_set(data, 3, (i + 1) as usize, 0);
    if voice.category_feature >= 0 {
        feat_set(data, 26, (i + 1) as usize, 0);
    }
    if voice.phoneme_feature >= 0 {
        feat_set(data, 28, (i + 1) as usize, 0);
    }
    let mut value = feat_get(data, 0, (data.length - 1) as usize);
    while value != b'#' {
        if feat_present(data, 1) {
            let len = data.length as i32;
            let v = w1(data, (len - 2) as usize).wrapping_add(w1(data, (len - 1) as usize));
            w1set(data, (len - 2) as usize, v);
        }
        let position = data.length.wrapping_sub(1);
        data.length = position;
        value = feat_get(data, 0, (position as i32 - 1) as usize);
    }
    let mut position_out = i32::from(data.length);
    output = position_out;
    i = 0;
    while i < output
        && (voice.phoneme_classes[usize::from(feat_get(data, 0, i as usize))] == 255
            || feat_get(data, 0, i as usize) == 0x23)
    {
        i = (i.wrapping_add(1) as i16) as i32;
    }
    if output < 1 {
        return 0x84e02015;
    }
    j = position_out - 1;
    if j >= 0 {
        loop {
            let b = feat_get(data, 0, j as usize);
            if !(voice.phoneme_classes[usize::from(b)] == 255 || b == 0x23) {
                break;
            }
            position_out -= 1;
            j = position_out;
            if j < 0 {
                break;
            }
        }
    }
    found = 1;
    let mut flag = true;
    while i <= j && found == 1 {
        let b = feat_get(data, 0, i as usize);
        if b == b'*' || b == b'%' || b == b'#' {
            found = 0;
        }
        if b == b'*' || b == b'%' || b == b'#' || b == b'-' {
            flag = false;
        }
        i = (i.wrapping_add(1) as i16) as i32;
    }
    if voice.voice_feature < 0 && flag && default_voice == 2 {
        default_voice = 0;
    }
    if feat_get(data, 0, 0) != b'#' {
        return 0x84e02015;
    }
    if feat_get(data, 0, (output - 1) as usize) != b'#' {
        return 0x84e02015;
    }
    if voice.modifier_feature >= 0 {
        match find_byte_in(voice.modifiers, feat_get(data, 0, 1)) {
            None => feat_set(data, 31, 0, 0),
            Some(_) => {
                let at = find_byte_in(voice.modifiers, feat_get(data, 0, 1)).unwrap();
                feat_set(data, 31, 0, (at as u32 + 1) as u8);
            }
        }
        feat_set(data, 32, 0, 0);
    }
    if voice.number_feature >= 0 || voice.alternate_number_feature >= 0 {
        feat_set(data, 33, 0, 0);
    }
    if voice.stress_feature >= 0 {
        feat_set(data, 34, 0, 1);
    }
    if voice.phrase_feature >= 0 {
        feat_set(data, 35, 0, 1);
    }
    if voice.position_feature >= 0 {
        feat_set(data, 36, 0, 1);
    }
    if voice.length_feature >= 0 {
        feat_set(data, 37, 0, 1);
    }
    if feat_present(data, 1) {
        let v = w1(data, 0);
        feat_set(data, 5, 0, v);
    }
    let mut feature_value: u8 = if voice.language == 2 { 0x57 } else { 0x58 };
    feat_set(data, 4, 0, feature_value);
    let v = feat_get(data, 0, 0);
    feat_set(data, 2, 0, v);
    feat_set(data, 7, 0, 1);
    feat_set(data, 8, 0, 1);
    feat_set(data, 9, 0, 0);
    feat_set(data, 10, 0, 3);
    feat_set(data, 11, 0, 1);
    feat_set(data, 12, 0, 0);
    feat_set(data, 13, 0, 1);
    feat_set(data, 14, 0, 0);
    feat_set(data, 16, 0, 4);
    if voice.category_feature >= 0 {
        feat_set(data, 25, 0, 0);
        feat_set(data, 26, 0, 0);
    }
    if voice.phoneme_feature >= 0 {
        feat_set(data, 27, 0, 0);
        feat_set(data, 28, 0, 0);
    }
    if voice.pitch_feature >= 0 || voice.alternate_pitch_feature >= 0 {
        let pitch = data.pitch.as_deref_mut().unwrap();
        pitch[0] = 0xff38;
        pitch[1] = 0xff38;
        pitch[2] = 200;
        pitch[3] = 200;
    }
    if voice.voice_feature < 0 {
        feat_set(data, 29, 0, default_voice);
    }
    feat_set(data, 17, 0, 3);
    i = 1;
    feat_set(data, 18, 0, 0);
    output = 1;
    feat_set(data, 19, 0, 0);
    feat_set(data, 20, 0, 4);
    feat_set(data, 21, 0, 4);
    feat_set(data, 22, 0, 1);
    if 1 < i32::from(data.length) - 1 {
        loop {
            let at_i = i as usize;
            let at_o = output as usize;
            if voice.phoneme_classes[usize::from(feat_get(data, 0, at_i))] == 255 {
                if feat_present(data, 1) {
                    pending_weight = u32::from(w1(data, at_i)).wrapping_add(pending_weight) & 0xff;
                }
            } else {
                if feat_present(data, 1) {
                    let v = pending_weight as u8;
                    pending_weight = 0;
                    feat_set(data, 5, at_o, w1(data, at_i).wrapping_add(v));
                }
                let v = feat_get(data, 0, at_i);
                feat_set(data, 2, at_o, v);
                if voice.voice_feature < 0 {
                    feat_set(data, 29, at_o, default_voice);
                } else {
                    let v = feat_get(data, 29, at_i);
                    feat_set(data, 29, at_o, v);
                }
                if voice.modifier_feature >= 0 {
                    match find_byte_in(voice.modifiers, feat_get(data, 0, at_i + 1)) {
                        None => {
                            feat_set(data, 31, at_o, 0);
                        }
                        Some(at) => {
                            feat_set(data, 31, at_o, (at as u32 + 1) as u8);
                        }
                    }
                }
                if feat_get(data, 0, at_i) == b'#' {
                    if voice.number_feature >= 0 || voice.alternate_number_feature >= 0 {
                        feat_set(data, 33, at_o, 0);
                    }
                    if voice.modifier_feature >= 0 {
                        feat_set(data, 32, at_o, 0);
                    }
                    feat_set(data, 7, at_o, 1);
                    feat_set(data, 8, at_o, 1);
                    feat_set(data, 9, at_o, 0);
                    found = 0;
                    feat_set(data, 10, at_o, 3);
                    feat_set(data, 17, at_o, 3);
                    feat_set(data, 18, at_o, 0);
                    feat_set(data, 19, at_o, 0);
                    feat_set(data, 20, at_o, 4);
                    feat_set(data, 21, at_o, 4);
                    feat_set(data, 11, at_o, 2);
                    let mut position = i as i16;
                    loop {
                        position = position.wrapping_sub(1);
                        if position < 0 || found != 0 {
                            break;
                        }
                        let idx = u32::from(feat_get(data, 0, position as usize));
                        let is_class = voice.phoneme_classes[idx as usize] != 255;
                        if is_class {
                            found = 1;
                        }
                        if !is_class && idx == 0x25 {
                            found = 1;
                            feat_set(data, 11, at_o, 3);
                        }
                    }
                    if feat_get(data, 11, at_o) == 2 {
                        found = 0;
                        j = i;
                        loop {
                            j = (j.wrapping_add(1) as i16) as i32;
                            if !(j < i32::from(data.length) && found == 0) {
                                break;
                            }
                            let b = feat_get(data, 0, j as usize);
                            let is_class = voice.phoneme_classes[usize::from(b)] != 255;
                            if is_class {
                                found = 1;
                            }
                            if !is_class && b == 0x25 {
                                found = 1;
                                feat_set(data, 11, at_o, 4);
                            }
                        }
                    }
                } else {
                    let next_phoneme = i.wrapping_add(1);
                    found = 0;
                    let mut position = next_phoneme as i16;
                    index = 1;
                    while (position as i32) < i32::from(data.length) && found == 0 {
                        let n = u32::from(feat_get(data, 0, position as usize));
                        if n == 0x2d || n == 0x25 || n == 0x23 {
                            found = 1;
                            feat_set(data, 7, at_o, 1);
                        } else if voice.phoneme_classes[n as usize] != 255 {
                            found = 1;
                            feat_set(data, 7, at_o, 0);
                        }
                        position = position.wrapping_add(1);
                    }
                    let previous_phoneme = i.wrapping_sub(1);
                    let mut n: u32 = 0;
                    j = previous_phoneme;
                    loop {
                        j = (j as i16) as i32;
                        if !(j >= 0 && n == 0) {
                            break;
                        }
                        let limit = u32::from(feat_get(data, 0, j as usize));
                        if limit == 0x2d || limit == 0x25 || limit == 0x23 {
                            n = 1;
                            feat_set(data, 8, at_o, 1);
                            index = 1;
                        } else {
                            index = u32::from(voice.phoneme_classes[limit as usize]);
                            if index != 0xff {
                                n = 1;
                                feat_set(data, 8, at_o, 0);
                            }
                        }
                        j -= 1;
                    }
                    if voice.modifier_feature >= 0 {
                        if feat_get(data, 2, at_o - 1) == b'#' {
                            let v = crate::resources::string_length(voice.modifiers) as u8;
                            feat_set(data, 32, at_o, v.wrapping_add(1));
                        } else if feat_get(data, 8, at_o) == 1 {
                            let v = feat_get(data, 31, at_o - 1);
                            feat_set(data, 32, at_o, v);
                        } else {
                            let v = feat_get(data, 32, at_o - 1);
                            feat_set(data, 32, at_o, v);
                        }
                    }
                    let mut acute = false;
                    let mut grave = false;
                    let mut scan = next_phoneme as i16;
                    flag = false;
                    found = 0;
                    position = scan;
                    while (position as i32) < i32::from(data.length) && found == 0 {
                        let v = feat_get(data, 0, position as usize);
                        if v == b'-' || v == b'%' || v == b'#' {
                            found = 1;
                        }
                        if v != b'-' && v != b'%' && v != b'#' {
                            if v == b'\'' {
                                acute = true;
                            } else if v == b'`' {
                                grave = true;
                            } else if v == b'"' {
                                flag = true;
                            }
                        }
                        position = position.wrapping_add(1);
                    }
                    found = 0;
                    j = previous_phoneme;
                    loop {
                        j = (j as i16) as i32;
                        if !(j >= 0 && found == 0) {
                            break;
                        }
                        let v = feat_get(data, 0, j as usize);
                        if v == b'-' || v == b'%' || v == b'#' {
                            found = 1;
                        }
                        if v != b'-' && v != b'%' && v != b'#' {
                            if v == b'\'' {
                                acute = true;
                            } else if v == b'`' {
                                grave = true;
                            } else if v == b'"' {
                                flag = true;
                            }
                        }
                        j -= 1;
                    }
                    let stress: u8 = if flag {
                        3
                    } else if acute {
                        2
                    } else if grave {
                        1
                    } else {
                        0
                    };
                    feat_set(data, 9, at_o, stress);
                    if stress == 3 {
                        if first_phrase {
                            feat_set(data, 18, at_o, if first_sentence { 3 } else { 4 });
                        } else {
                            feat_set(data, 18, at_o, 2);
                        }
                        if feat_get(data, 7, at_o) == 1 {
                            first_phrase = false;
                        }
                    } else if stress == 0 {
                        feat_set(data, 18, at_o, 0);
                    } else {
                        feat_set(data, 18, at_o, if stress == 1 { 7 } else { 1 });
                    }
                    feat_set(data, 19, at_o, 0);
                    if feat_get(data, 9, at_o) == 3
                        && (feat_get(data, 12, at_o - 1) == 2 || feat_get(data, 19, at_o - 1) == 0)
                    {
                        feat_set(data, 19, at_o, 2);
                    } else if feat_get(data, 12, at_o - 1) == 1 {
                        let v = feat_get(data, 19, at_o - 1);
                        if v == 2 {
                            feat_set(data, 19, at_o, 3);
                        } else if v == 3 || v == 4 {
                            feat_set(data, 19, at_o, 4);
                        }
                    } else if feat_get(data, 12, at_o - 1) == 0 {
                        let v = feat_get(data, 19, at_o - 1);
                        feat_set(data, 19, at_o, v);
                    }
                    feat_set(data, 10, at_o, 2);
                    j = scan as i32;
                    while j < i32::from(data.length)
                        && (voice.phoneme_classes[usize::from(feat_get(data, 0, j as usize))]
                            != 255
                            || find_byte_in(voice.modifiers, feat_get(data, 0, j as usize))
                                .is_some())
                    {
                        scan = scan.wrapping_add(1);
                        j = scan as i32;
                    }
                    if i32::from(data.length) == j && feat_get(data, 0, (j - 1) as usize) == b'#' {
                        feat_set(data, 10, at_o, 3);
                    }
                    while j < i32::from(data.length)
                        && voice.phoneme_classes[usize::from(feat_get(data, 0, j as usize))] == 255
                        && {
                            let idx = feat_get(data, 0, j as usize);
                            idx != 0x2a && idx != 0x25 && idx != 0x23
                        }
                    {
                        j = (j.wrapping_add(1) as i16) as i32;
                    }
                    if j < i32::from(data.length) {
                        let v = feat_get(data, 0, j as usize);
                        if v == b'*' || v == b'%' || v == b'#' {
                            feat_set(data, 10, at_o, 3);
                        } else {
                            j = previous_phoneme;
                            if feat_get(data, 10, at_o) == 2 {
                                loop {
                                    j = (j as i16) as i32;
                                    if !(j > 0
                                        && (voice.phoneme_classes
                                            [usize::from(feat_get(data, 0, j as usize))]
                                            != 255
                                            || find_byte_in(
                                                voice.modifiers,
                                                feat_get(data, 0, j as usize),
                                            )
                                            .is_some()))
                                    {
                                        break;
                                    }
                                    j -= 1;
                                }
                                if j == -1 {
                                    if feat_get(data, 0, 0) == b'#' {
                                        feat_set(data, 10, at_o, 1);
                                    }
                                } else {
                                    while j > 0
                                        && voice.phoneme_classes
                                            [usize::from(feat_get(data, 0, j as usize))]
                                            == 255
                                    {
                                        let idx = feat_get(data, 0, j as usize);
                                        if idx == 0x2a || idx == 0x25 || idx == 0x23 {
                                            feat_set(data, 10, at_o, 1);
                                            break;
                                        }
                                        j = (j.wrapping_sub(1) as i16) as i32;
                                    }
                                }
                                let v = feat_get(data, 0, j as usize);
                                if v == b'*' || v == b'%' || v == b'#' {
                                    feat_set(data, 10, at_o, 1);
                                }
                            }
                        }
                    } else {
                        j = previous_phoneme;
                        if feat_get(data, 10, at_o) == 2 {
                            loop {
                                j = (j as i16) as i32;
                                if !(j > 0
                                    && (voice.phoneme_classes
                                        [usize::from(feat_get(data, 0, j as usize))]
                                        != 255
                                        || find_byte_in(
                                            voice.modifiers,
                                            feat_get(data, 0, j as usize),
                                        )
                                        .is_some()))
                                {
                                    break;
                                }
                                j -= 1;
                            }
                            if j == -1 {
                                if feat_get(data, 0, 0) == b'#' {
                                    feat_set(data, 10, at_o, 1);
                                }
                            } else {
                                while j > 0
                                    && voice.phoneme_classes
                                        [usize::from(feat_get(data, 0, j as usize))]
                                        == 255
                                {
                                    let idx = feat_get(data, 0, j as usize);
                                    if idx == 0x2a || idx == 0x25 || idx == 0x23 {
                                        feat_set(data, 10, at_o, 1);
                                        break;
                                    }
                                    j = (j.wrapping_sub(1) as i16) as i32;
                                }
                            }
                            let v = feat_get(data, 0, j as usize);
                            if v == b'*' || v == b'%' || v == b'#' {
                                feat_set(data, 10, at_o, 1);
                            }
                        }
                    }
                    feat_set(data, 11, at_o, 2);
                    position = next_phoneme as i16;
                    let nn: u32 = loop {
                        index = position as u32;
                        let n = u32::from(feat_get(data, 0, position as usize));
                        if i32::from(data.length) <= position as i32 {
                            break n;
                        }
                        if n == 0x2a || n == 0x25 || n == 0x23 {
                            break n;
                        }
                        position = position.wrapping_add(1);
                    };
                    if nn == 0x2a || nn == 0x25 || nn == 0x23 {
                        let end = (data.length as u32).wrapping_sub(1);
                        let mut limit = end;
                        if end == index {
                            limit = nn;
                        }
                        if end == index && limit == 0x23 {
                            feat_set(data, 11, at_o, 5);
                            feat_set(data, 20, at_o, 4);
                        } else if nn != 0x25 {
                            loop {
                                position = position.wrapping_add(1);
                                j = position as i32;
                                if i32::from(data.length) <= j
                                    || voice.phoneme_classes
                                        [usize::from(feat_get(data, 0, j as usize))]
                                        != 255
                                {
                                    break;
                                }
                                let idx = feat_get(data, 0, j as usize);
                                if i32::from(data.length) - 1 == j && idx == 0x23 {
                                    feat_set(data, 11, at_o, 5);
                                } else if idx == 0x25 {
                                    feat_set(data, 11, at_o, 4);
                                }
                            }
                            prosody_boundary_tail(
                                &mut *data,
                                voice.phoneme_classes,
                                at_o,
                                previous_phoneme,
                            );
                        } else {
                            feat_set(data, 11, at_o, 4);
                            feat_set(data, 20, at_o, 2);
                        }
                    } else {
                        prosody_boundary_tail(
                            &mut *data,
                            voice.phoneme_classes,
                            at_o,
                            previous_phoneme,
                        );
                    }
                    if voice.number_feature >= 0 || voice.alternate_number_feature >= 0 {
                        found = 0;
                        let mut scan_index = next_phoneme;
                        loop {
                            j = (scan_index as i16) as i32;
                            if !(j < i32::from(data.length) && found == 0) {
                                break;
                            }
                            let v = feat_get(data, 0, j as usize);
                            if v == b'-' || v == b'%' || v == b'#' {
                                feat_set(data, 33, at_o, 0);
                                found = 1;
                            } else if find_byte_in(b"1234\0", v).is_some() {
                                feat_set(data, 33, at_o, v.wrapping_sub(0x31));
                                found = 1;
                            }
                            scan_index = j.wrapping_add(1);
                        }
                    }
                }
                let boundary: u8 = if feat_get(data, 7, at_o) == 1 {
                    if feat_get(data, 10, at_o) == 3 {
                        if feat_get(data, 11, at_o) == 5 || feat_get(data, 11, at_o) == 4 {
                            3
                        } else {
                            2
                        }
                    } else {
                        1
                    }
                } else {
                    0
                };
                feat_set(data, 12, at_o, boundary);
                if feat_get(data, 2, at_o) != b'#' {
                    let previous_kind = feat_get(data, 12, at_o - 1);
                    if previous_kind == 2 || previous_kind == 3 || output == 1 {
                        feat_set(data, 21, at_o, 0);
                    } else if boundary == 2 || boundary == 3 {
                        feat_set(data, 21, at_o, 4);
                    } else if boundary == 1 {
                        feat_set(data, 21, at_o, 3);
                    } else if previous_kind == 1 {
                        feat_set(data, 21, at_o, 1);
                    } else {
                        feat_set(data, 21, at_o, 2);
                    }
                }
                feature_value = if first_word { 1 } else { 2 };
                feat_set(data, 13, at_o, feature_value);
                if feat_get(data, 7, at_o) == 1 {
                    first_word = false;
                }
                feat_set(data, 14, at_o, phrase_count);
                if feat_get(data, 12, at_o) == 3 {
                    phrase_count = 0;
                } else if feat_get(data, 2, at_o) != b'#'
                    && feat_get(data, 7, at_o) == 1
                    && phrase_count != 254
                {
                    phrase_count = phrase_count.wrapping_add(1);
                }
                if voice.phrase_feature >= 0 {
                    feat_set(data, 35, at_o, 2);
                    if phrase_started == 0 {
                        feat_set(data, 35, at_o, 1);
                        if (feat_get(data, 9, at_o) == 2 || feat_get(data, 9, at_o) == 3)
                            && feat_get(data, 7, at_o) == 1
                        {
                            phrase_started = 1;
                        }
                    }
                }
                output = (output.wrapping_add(1) as i16) as i32;
            }
            if feat_get(data, 0, i as usize) == b'%' {
                first_sentence = false;
                first_phrase = true;
            }
            i = (i.wrapping_add(1) as i16) as i32;
            if !(i < i32::from(data.length) - 1) {
                break;
            }
        }
    }
    if voice.context_enabled != 0 {
        let ctx = data.contexts.as_deref_mut().unwrap();
        ctx[(output * 9 + 7) as usize] = 0x23;
        ctx[(output * 9 + 8) as usize] = 0x23;
    }
    if voice.stress_feature >= 0 {
        feat_set(data, 34, output as usize, 1);
    }
    if voice.phrase_feature >= 0 {
        feat_set(data, 35, output as usize, 3);
    }
    if voice.position_feature >= 0 {
        feat_set(data, 36, output as usize, 1);
    }
    if voice.length_feature >= 0 {
        feat_set(data, 37, output as usize, 1);
    }
    if voice.modifier_feature >= 0 {
        feat_set(data, 31, output as usize, 0);
        feat_set(data, 32, output as usize, 0);
    }
    if voice.number_feature >= 0 || voice.alternate_number_feature >= 0 {
        feat_set(data, 33, output as usize, 0);
    }
    feature_value = if voice.language == 2 { 0x57 } else { 0x58 };
    feat_set(data, 4, output as usize, feature_value);
    let v = feat_get(data, 0, (data.length - 1) as usize);
    feat_set(data, 2, output as usize, v);
    if feat_present(data, 1) {
        let v = w1(data, (data.length - 1) as usize).wrapping_add(pending_weight as u8);
        feat_set(data, 5, output as usize, v);
    }
    feature_value = 0;
    feat_set(data, 7, output as usize, 1);
    feat_set(data, 8, output as usize, 1);
    feat_set(data, 9, output as usize, 0);
    feat_set(data, 10, output as usize, 3);
    feat_set(data, 11, output as usize, 5);
    feat_set(data, 12, output as usize, 0);
    feat_set(data, 13, output as usize, 3);
    feat_set(data, 14, output as usize, 0);
    feat_set(data, 15, output as usize, 0);
    feat_set(data, 16, output as usize, 4);
    if voice.category_feature >= 0 {
        feat_set(data, 25, output as usize, 0);
        feat_set(data, 26, output as usize, 0);
    }
    if voice.phoneme_feature >= 0 {
        feat_set(data, 27, output as usize, 0);
        feat_set(data, 28, output as usize, 0);
    }
    if voice.pitch_feature >= 0 || voice.alternate_pitch_feature >= 0 {
        let base = (output * 4) as usize;
        let pitch = data.pitch.as_deref_mut().unwrap();
        pitch[base] = 0xff38;
        pitch[base + 1] = 0xff38;
        pitch[base + 2] = 200;
        pitch[base + 3] = 200;
    }
    if voice.voice_feature < 0 {
        feat_set(data, 29, output as usize, default_voice);
    } else {
        let v = feat_get(data, 29, (output - 1) as usize);
        feat_set(data, 29, output as usize, v);
    }
    feat_set(data, 17, output as usize, 3);
    feat_set(data, 18, output as usize, 0);
    feat_set(data, 19, output as usize, 0);
    feat_set(data, 20, output as usize, 4);
    feat_set(data, 21, output as usize, 4);
    feat_set(data, 22, output as usize, 1);
    if voice.context_enabled != 0 {
        let base = (output * 9) as usize;
        let ctx = data.contexts.as_deref_mut().unwrap();
        ctx[base..base + 9].fill(0);
        let v = feat_get(data, 2, (output - 1) as usize);
        let ctx = data.contexts.as_deref_mut().unwrap();
        ctx[base + 5] = v;
        ctx[base + 6] = 0x23;
        ctx[base] = 0x23;
    }
    data.count = (output as i16).wrapping_add(1);
    found = 1;
    if voice.phrase_feature >= 0 {
        phrase_started = 0;
    }
    first_word = true;
    output = (output.wrapping_sub(1) as i16) as i32;
    i = ((i32::from(data.length).wrapping_sub(2)) as i16) as i32;
    if i > 0 {
        feature_value = 2;
    }
    first_phrase = true;
    loop {
        if !(i > 0) {
            break;
        }
        // ARM gates the whole body (through the markup block and
        // `output -= 1`) on the class check; only `i -= 1` is shared.
        if voice.phoneme_classes[usize::from(feat_get(data, 0, i as usize))] != 255 {
            phrase_count = count as u8;
            let at_o = output as usize;
            feat_set(data, 15, at_o, u8::from(found == 0));
            j = i;
            loop {
                j = (j.wrapping_sub(1) as i16) as i32;
                if j < 1 {
                    break;
                }
                if voice.phoneme_classes[usize::from(feat_get(data, 0, j as usize))] != 255 {
                    break;
                }
                if feat_get(data, 0, j as usize) == 0x25 {
                    break;
                }
            }
            if feat_get(data, 9, at_o) == 3 {
                if found != 0 {
                    if first_phrase {
                        feat_set(data, 18, at_o, 6);
                        if feat_get(data, 8, at_o) == 1 {
                            first_phrase = false;
                        }
                    }
                } else if first_phrase {
                    let v = feat_get(data, 18, at_o);
                    if v != 3 && v != 4 {
                        feat_set(data, 18, at_o, 5);
                        if feat_get(data, 8, at_o) == 1 {
                            first_phrase = false;
                        }
                    }
                }
            }
            if feat_get(data, 19, at_o) == 0 {
                if feat_get(data, 12, at_o) == 1 {
                    let v = feat_get(data, 19, at_o + 1);
                    if v == 2 || v == 1 {
                        feat_set(data, 19, at_o, 1);
                    }
                } else if feat_get(data, 12, at_o) == 0 && feat_get(data, 19, at_o + 1) == 1 {
                    feat_set(data, 19, at_o, 1);
                }
            }
            let address = i32::from(data.length) - 1;
            if address == i
                || (i < address
                    && (feat_get(data, 14, at_o) != 0 || feat_get(data, 12, at_o) == 3)
                    && feat_get(data, 14, at_o + 1) == 0)
            {
                count = u32::from(feat_get(data, 14, at_o));
            }
            feat_set(data, 14, at_o + 1, phrase_count.wrapping_add(1));
            index = u32::from(feat_get(data, 14, at_o));
            if index == count {
                phrase_count = 4;
            } else if index == count.wrapping_sub(1) {
                phrase_count = 6;
            } else if index == 1 {
                phrase_count = 1;
            } else if index == 0 {
                phrase_count = 0;
            } else {
                let n = (((count as i16) as i32).wrapping_mul(3) as i16 as i32)
                    .wrapping_mul(0xcd)
                    .wrapping_shr(11) as u32;
                if (count.wrapping_sub(n) as i32) < index as i32 {
                    phrase_count = 2;
                } else if index < n {
                    phrase_count = 3;
                } else {
                    phrase_count = 5;
                }
            }
            feat_set(data, 16, at_o, phrase_count);
            if feat_get(data, 0, i as usize) == b'#' {
                if feat_get(data, 17, at_o) == 2 && feat_get(data, 18, at_o) == 4 {
                    feat_set(data, 18, at_o, if found != 0 { 6 } else { 5 });
                }
            } else if phrase_count == 4 {
                if feat_get(data, 14, at_o) == 0 {
                    feat_set(data, 17, at_o, feature_value);
                    if feat_get(data, 18, at_o) == 4 {
                        feat_set(data, 18, at_o, if found != 0 { 6 } else { 5 });
                    }
                } else {
                    feat_set(data, 17, at_o, if found != 0 { 3 } else { 1 });
                }
            } else if feat_get(data, 14, at_o) == 0 {
                feat_set(data, 17, at_o, 0);
            } else {
                match feat_get(data, 10, at_o) {
                    1 => feat_set(data, 17, at_o, 4),
                    2 => feat_set(data, 17, at_o, 5),
                    3 => feat_set(data, 17, at_o, 6),
                    _ => {
                        if feat_get(data, 17, at_o) == 2 && feat_get(data, 18, at_o) == 4 {
                            feat_set(data, 18, at_o, if found != 0 { 6 } else { 5 });
                        }
                    }
                }
            }
            if feat_get(data, 0, j as usize) == b'%' {
                found = 0;
                first_phrase = true;
            }
            if first_word {
                feat_set(data, 13, at_o, 3);
            }
            if feat_get(data, 8, at_o) == 1 {
                first_word = false;
            }
            if voice.stress_feature >= 0 {
                feat_set(data, 34, at_o, 1);
                let boundary = feat_get(data, 2, at_o) == b'#'
                    || feat_get(data, 12, at_o) == 2
                    || feat_get(data, 12, at_o) == 3;
                if boundary {
                    stress_start = 1;
                }
                if boundary || stress_start == 1 {
                    if feat_get(data, 9, at_o) < 2 {
                        if feat_get(data, 2, at_o) != b'#' {
                            feat_set(data, 34, at_o, 3);
                        }
                    } else {
                        stress_start = 0;
                    }
                }
            }
            if voice.phrase_feature >= 0 && phrase_started == 0 {
                if i32::from(data.length) - 2 == i
                    || ((feat_get(data, 9, at_o + 1) != 2 && feat_get(data, 9, at_o + 1) != 3)
                        || feat_get(data, 7, at_o) != 1)
                {
                    feat_set(data, 35, at_o, 3);
                } else {
                    phrase_started = 1;
                }
            }
            if voice.position_feature >= 0 {
                if feat_get(data, 2, at_o) == b'#' {
                    feat_set(data, 36, at_o, 1);
                } else {
                    phrase_count = feat_get(data, 12, at_o);
                    if phrase_count == 2 || phrase_count == 3 {
                        feat_set(data, 36, at_o, 1);
                    } else {
                        let mut v = feat_get(data, 36, at_o + 1);
                        if phrase_count == 1 {
                            v = v.wrapping_add(1);
                        }
                        feat_set(data, 36, at_o, v);
                    }
                }
            }
            index = u32::from(voice.phoneme_classes[usize::from(feat_get(data, 0, i as usize))]);
            if voice.markup == 0 {
                if voice.language != 2 {
                    feat_set(data, 4, at_o, voice.language_classes[index as usize]);
                } else {
                    feat_set(data, 4, at_o, 0x57);
                }
            }
            output = (output.wrapping_sub(1) as i16) as i32;
        }
        i = (i.wrapping_sub(1) as i16) as i32;
    }
    output = 0;
    // ARM (US 0x292218/0x292220/0x292224/0x29222c) writes count+1 here, not
    // phrase_count+1 as src/fn_2902d8.c:1335-1337 does (C translation defect).
    let post_count = (count as u8).wrapping_add(1);
    feat_set(data, 14, 0, post_count);
    feat_set(data, 14, 1, post_count);
    let v = feat_get(data, 15, 1);
    feat_set(data, 15, 0, v);
    if voice.stress_feature >= 0 {
        stress_start = 1;
    }
    if 0 < i32::from(data.count) - 1 {
        loop {
            let at_o = output as usize;
            if feat_get(data, 8, at_o) == 1 {
                index = 1;
                if voice.context_enabled != 0 {
                    i = output * 9;
                    let base = i as usize;
                    data.contexts.as_deref_mut().unwrap()[base..base + 9].fill(0);
                    let prev = if output == 0 {
                        0x23
                    } else {
                        feat_get(data, 2, at_o - 1)
                    };
                    data.contexts.as_deref_mut().unwrap()[base + 5] = prev;
                    let v = feat_get(data, 2, at_o);
                    let ctx = data.contexts.as_deref_mut().unwrap();
                    ctx[base + 7] = v;
                    ctx[base] = v;
                }
                i = output;
                if feat_get(data, 7, at_o) != 1 {
                    j = output * 9;
                    loop {
                        if voice.context_enabled != 0 {
                            if index < 5 {
                                let v = feat_get(data, 2, (i + 1) as usize);
                                data.contexts.as_deref_mut().unwrap()
                                    [(j + index as i32) as usize] = v;
                            } else {
                                let base = j as usize;
                                data.contexts.as_deref_mut().unwrap()[base..base + 9].fill(0);
                            }
                        }
                        i = (i.wrapping_add(1) as i16) as i32;
                        index = index.wrapping_add(1) & 0xff;
                        if feat_get(data, 7, i as usize) == 1 {
                            break;
                        }
                    }
                }
                if voice.context_enabled != 0 {
                    let base = (output * 9) as usize;
                    let v6 = feat_get(data, 2, (i + 1) as usize);
                    let v8 = feat_get(data, 2, i as usize);
                    let ctx = data.contexts.as_deref_mut().unwrap();
                    ctx[base + 6] = v6;
                    ctx[base + 8] = v8;
                }
                feat_set(data, 22, at_o, index as u8);
            } else {
                let v = feat_get(data, 22, at_o - 1);
                feat_set(data, 22, at_o, v);
                if voice.context_enabled != 0 {
                    let base = (output * 9) as usize;
                    let mut row = [0u8; 9];
                    row.copy_from_slice(&data.contexts.as_deref().unwrap()[base - 9..base]);
                    data.contexts.as_deref_mut().unwrap()[base..base + 9].copy_from_slice(&row);
                }
            }
            if feat_get(data, 22, at_o) == 1
                && feat_get(data, 2, at_o) != b'#'
                && feat_get(data, 21, at_o) != 0
            {
                feat_set(data, 21, at_o, 1);
            }
            if voice.markup == 0 {
                let ac = usize::from(voice.acoustic_count);
                let dc = usize::from(voice.duration_count);
                for d in 0..ac {
                    let v = read_i32(voice.acoustic_defaults, d * 24 + 4);
                    data.acoustic.as_deref_mut().unwrap()[at_o * ac + d] = v;
                }
                for d in 0..dc {
                    let v = read_i32(voice.duration_defaults, d * 12 + 4);
                    data.duration.as_deref_mut().unwrap()[at_o * dc + d] = v;
                }
                for d in 0..dc {
                    let v = read_i32(voice.duration_defaults, d * 12 + 4);
                    data.alternate_duration.as_deref_mut().unwrap()[at_o * dc + d] = v;
                }
            }
            let dc = usize::from(voice.duration_count);
            let ac = usize::from(voice.acoustic_count);
            if voice.duration_rules.count != 0 {
                i = 0;
                loop {
                    let entry = i as usize;
                    let stride = usize::from(voice.duration_rules.stride);
                    if prosody_rule_matches(
                        voice,
                        data,
                        &voice.duration_rules,
                        entry,
                        at_o,
                        output - 1,
                        output + 2,
                    ) {
                        let base = entry * stride;
                        let f = voice.duration_rules.entries[base + stride - 2] as usize;
                        let n = u32::from(voice.duration_rules.entries[base + stride - 1]);
                        let slot = &mut data.duration.as_deref_mut().unwrap()[at_o * dc + f];
                        prosody_apply_scale(slot, n);
                    }
                    i = (i.wrapping_add(1) as i16) as i32;
                    if !(i < i32::from(voice.duration_rules.count)) {
                        break;
                    }
                }
            }
            if voice.alternate_duration_rules.count != 0 {
                i = 0;
                loop {
                    let entry = i as usize;
                    let stride = usize::from(voice.alternate_duration_rules.stride);
                    if prosody_rule_matches(
                        voice,
                        data,
                        &voice.alternate_duration_rules,
                        entry,
                        at_o,
                        output - 1,
                        output + 2,
                    ) {
                        let base = entry * stride;
                        let f = voice.alternate_duration_rules.entries[base + stride - 2] as usize;
                        let n =
                            u32::from(voice.alternate_duration_rules.entries[base + stride - 1]);
                        let slot =
                            &mut data.alternate_duration.as_deref_mut().unwrap()[at_o * dc + f];
                        prosody_apply_scale(slot, n);
                    }
                    i = (i.wrapping_add(1) as i16) as i32;
                    if !(i < i32::from(voice.alternate_duration_rules.count)) {
                        break;
                    }
                }
            }
            if voice.acoustic_rules.count != 0 {
                i = 0;
                loop {
                    let entry = i as usize;
                    let stride = usize::from(voice.acoustic_rules.stride);
                    if prosody_rule_matches(
                        voice,
                        data,
                        &voice.acoustic_rules,
                        entry,
                        at_o,
                        output - 1,
                        output + 2,
                    ) {
                        let base = entry * stride;
                        let f = voice.acoustic_rules.entries[base + stride - 2] as usize;
                        let n = u32::from(voice.acoustic_rules.entries[base + stride - 1]);
                        let slot = &mut data.acoustic.as_deref_mut().unwrap()[at_o * ac + f];
                        prosody_apply_scale(slot, n);
                    }
                    i = (i.wrapping_add(1) as i16) as i32;
                    if !(i < i32::from(voice.acoustic_rules.count)) {
                        break;
                    }
                }
            }
            if voice.secondary_acoustic_rules.count != 0 {
                i = 0;
                loop {
                    let entry = i as usize;
                    let stride = usize::from(voice.secondary_acoustic_rules.stride);
                    if prosody_rule_matches(
                        voice,
                        data,
                        &voice.secondary_acoustic_rules,
                        entry,
                        at_o,
                        output - 1,
                        output + 2,
                    ) {
                        let base = entry * stride;
                        let f = voice.secondary_acoustic_rules.entries[base + stride - 2] as usize;
                        let n =
                            u32::from(voice.secondary_acoustic_rules.entries[base + stride - 1]);
                        let slot = &mut data.acoustic.as_deref_mut().unwrap()[at_o * ac + f];
                        prosody_apply_scale(slot, n);
                    }
                    i = (i.wrapping_add(1) as i16) as i32;
                    if !(i < i32::from(voice.secondary_acoustic_rules.count)) {
                        break;
                    }
                }
            }
            if voice.markup == 0 {
                for d in 0..ac {
                    let v = voice.acoustic_defaults[d * 24 + 10];
                    data.modes.as_deref_mut().unwrap()[at_o * ac + d] = v;
                }
            }
            if voice.mode_rules.count != 0 {
                i = 0;
                loop {
                    let entry = i as usize;
                    let stride = usize::from(voice.mode_rules.stride);
                    if prosody_rule_matches(
                        voice,
                        data,
                        &voice.mode_rules,
                        entry,
                        at_o,
                        output - 1,
                        output + 2,
                    ) {
                        let base = entry * stride;
                        let f = voice.mode_rules.entries[base + stride - 2] as usize;
                        let v = voice.mode_rules.entries[base + stride - 1];
                        data.modes.as_deref_mut().unwrap()[at_o * ac + f] = v;
                    }
                    i = (i.wrapping_add(1) as i16) as i32;
                    if !(i < i32::from(voice.mode_rules.count)) {
                        break;
                    }
                }
            }
            if voice.stress_feature >= 0 {
                if stress_start == 1 {
                    if feat_get(data, 9, at_o) < 2 {
                        if feat_get(data, 2, at_o) != b'#' {
                            feat_set(data, 34, at_o, 1);
                        }
                        if feat_get(data, 9, at_o) != 0 {
                            feat_set(data, 34, at_o, 2);
                        }
                    } else {
                        stress_start = 0;
                        feat_set(data, 34, at_o, 2);
                    }
                } else if feat_get(data, 9, at_o) != 0 {
                    feat_set(data, 34, at_o, 2);
                }
                if feat_get(data, 2, at_o) == b'#'
                    || feat_get(data, 12, at_o) == 2
                    || feat_get(data, 12, at_o) == 3
                {
                    stress_start = 1;
                }
            }
            if voice.length_feature >= 0 {
                if feat_get(data, 2, at_o) == b'#' {
                    feature_value = 1;
                } else {
                    feature_value = item as u8;
                }
                feat_set(data, 37, at_o, feature_value);
                if feat_get(data, 2, at_o) == b'#'
                    || feat_get(data, 12, at_o) == 2
                    || feat_get(data, 12, at_o) == 3
                {
                    item = u32::from(feat_get(data, 36, at_o + 1));
                }
            }
            output = (output.wrapping_add(1) as i16) as i32;
            if !(output < i32::from(data.count) - 1) {
                break;
            }
        }
    }
    if voice.markup == 0 {
        let ac = usize::from(voice.acoustic_count);
        let dc = usize::from(voice.duration_count);
        let last = (i32::from(data.count) - 1) as usize;
        for d in 0..ac {
            data.modes.as_deref_mut().unwrap()[last * ac + d] =
                voice.acoustic_defaults[d * 24 + 10];
        }
        for d in 0..ac {
            let v = read_i32(voice.acoustic_defaults, d * 24 + 4);
            data.acoustic.as_deref_mut().unwrap()[last * ac + d] = v;
        }
        for d in 0..dc {
            let v = read_i32(voice.duration_defaults, d * 12 + 4);
            data.duration.as_deref_mut().unwrap()[last * dc + d] = v;
        }
        for d in 0..dc {
            let v = read_i32(voice.duration_defaults, d * 12 + 4);
            data.alternate_duration.as_deref_mut().unwrap()[last * dc + d] = v;
        }
    }
    if voice.pitch_feature >= 0
        || voice.alternate_pitch_feature >= 0
        || voice.category_feature >= 0
        || voice.phoneme_feature >= 0
    {
        i = 0;
        output = i32::from(data.count);
        if voice.phoneme_feature < 0 {
            output -= 1;
        }
        if output > 0 {
            loop {
                let at = i as usize;
                item = u32::from(voice.phoneme_classes[usize::from(feat_get(data, 2, at))]);
                if voice.category_feature >= 0 {
                    if voice.categories[item as usize] == b'v' {
                        let mut o0 = feat_get(data, 25, at);
                        let mut o1 = feat_get(data, 26, at);
                        host.compute_features(
                            true,
                            voice.category_context,
                            core::slice::from_ref(&voice.category_table),
                            i,
                            &*data,
                            &mut o0,
                            &mut o1,
                        );
                        feat_set(data, 25, at, o0);
                        feat_set(data, 26, at, o1);
                    } else {
                        feat_set(data, 25, at, 0);
                        feat_set(data, 26, at, 0);
                    }
                }
                if voice.phoneme_feature >= 0 {
                    let mut o0 = feat_get(data, 27, at);
                    let mut o1 = feat_get(data, 28, at);
                    host.compute_features(
                        false,
                        voice.phoneme_context,
                        &voice.phoneme_table[item as usize..],
                        i,
                        &*data,
                        &mut o0,
                        &mut o1,
                    );
                    feat_set(data, 27, at, o0);
                    feat_set(data, 28, at, o1);
                }
                j = previous_vowel;
                if voice.pitch_feature >= 0 || voice.alternate_pitch_feature >= 0 {
                    let tag = feat_get(data, 29, at);
                    let base = at * 4;
                    let pitch = data.pitch.as_deref_mut().unwrap();
                    pitch[base] = 0xff38;
                    pitch[base + 1] = 0xff38;
                    pitch[base + 2] = 200;
                    pitch[base + 3] = 200;
                    if feat_get(data, 8, at) == 1 {
                        first_vowel = 1;
                    }
                    let mut index = u32::from(read_u16(voice.pitch_ranges, usize::from(tag) * 2));
                    let n = u32::from(read_u16(voice.pitch_ranges, usize::from(tag) * 2 + 2));
                    if index < n && voice.vowel_types[item as usize] == b'V' && first_vowel == 1 {
                        first_vowel = 0;
                        j = i;
                        if previous_vowel != 0 {
                            let mut matched: Option<(usize, usize)> = None;
                            if voice.pitch_feature >= 0 {
                                while index < n {
                                    let rec = index as usize * 12;
                                    if read_i16(voice.modifier_pitch_rules, rec + 8) as i32
                                        == i32::from(feat_get(data, 31, previous_vowel as usize))
                                        && read_i16(voice.modifier_pitch_rules, rec + 10) as i32
                                            == i32::from(feat_get(data, 31, at))
                                    {
                                        matched = Some((rec, 12));
                                        break;
                                    }
                                    index = index.wrapping_add(1) & 0xffff;
                                }
                                if let Some((rec, _)) = matched {
                                    let p0 = read_u16(voice.modifier_pitch_rules, rec);
                                    let p1 = read_u16(voice.modifier_pitch_rules, rec + 2);
                                    let p2 = read_u16(voice.modifier_pitch_rules, rec + 4);
                                    let p3 = read_u16(voice.modifier_pitch_rules, rec + 6);
                                    let pitch = data.pitch.as_deref_mut().unwrap();
                                    pitch[base] = p0;
                                    pitch[base + 1] = p1;
                                    pitch[base + 2] = p2;
                                    pitch[base + 3] = p3;
                                }
                            } else {
                                while index < n {
                                    let rec = index as usize * 16;
                                    if read_i16(voice.phrase_pitch_rules, rec + 8) as i32
                                        == i32::from(feat_get(data, 18, previous_vowel as usize))
                                        && read_i16(voice.phrase_pitch_rules, rec + 10) as i32
                                            == i32::from(feat_get(
                                                data,
                                                17,
                                                previous_vowel as usize,
                                            ))
                                        && read_i16(voice.phrase_pitch_rules, rec + 12) as i32
                                            == i32::from(feat_get(data, 18, at))
                                        && read_i16(voice.phrase_pitch_rules, rec + 14) as i32
                                            == i32::from(feat_get(data, 17, at))
                                    {
                                        matched = Some((rec, 16));
                                        break;
                                    }
                                    index = index.wrapping_add(1) & 0xffff;
                                }
                                if let Some((rec, _)) = matched {
                                    let p0 = read_u16(voice.phrase_pitch_rules, rec);
                                    let p1 = read_u16(voice.phrase_pitch_rules, rec + 2);
                                    let p2 = read_u16(voice.phrase_pitch_rules, rec + 4);
                                    let p3 = read_u16(voice.phrase_pitch_rules, rec + 6);
                                    let pitch = data.pitch.as_deref_mut().unwrap();
                                    pitch[base] = p0;
                                    pitch[base + 1] = p1;
                                    pitch[base + 2] = p2;
                                    pitch[base + 3] = p3;
                                }
                            }
                        }
                    }
                }
                previous_vowel = j;
                i = (i.wrapping_add(1) as i16) as i32;
                if voice.phoneme_feature >= 0 {
                    if i32::from(data.count) <= i {
                        break;
                    }
                } else if !(i < i32::from(data.count) - 1) {
                    break;
                }
            }
        }
    }
    item = 0;
    if data.count > 0 {
        loop {
            output = data.timings.as_deref().unwrap()[(item * 2) as usize];
            index = item;
            while output == -2 {
                index = index.wrapping_add(1) & 0xffff;
                output = data.timings.as_deref().unwrap()[(index * 2) as usize];
            }
            if index != item {
                if item < index {
                    let mut n = (!index.wrapping_sub(item).wrapping_sub(1)) & 1;
                    if n == 1 {
                        let v = feat_get(data, 5, index as usize).wrapping_add(feat_get(
                            data,
                            5,
                            item as usize,
                        ));
                        feat_set(data, 5, index as usize, v);
                    }
                    n = n.wrapping_add(item);
                    loop {
                        n &= 0xffff;
                        if !(n < index) {
                            break;
                        }
                        let v = feat_get(data, 5, index as usize)
                            .wrapping_add(feat_get(data, 5, n as usize));
                        feat_set(data, 5, index as usize, v);
                        let v = v.wrapping_add(feat_get(data, 5, (n + 1) as usize));
                        feat_set(data, 5, index as usize, v);
                        n = n.wrapping_add(2);
                    }
                }
                let mut n: u32 = 0;
                if 0 < i32::from(data.count) - index as i32 {
                    loop {
                        let dst = item.wrapping_add(n) as usize;
                        let src = index.wrapping_add(n) as usize;
                        for f in 0..56 {
                            if data.bytes[f + 2].is_some() {
                                let v = feat_get(data, f + 2, src);
                                feat_set(data, f + 2, dst, v);
                            }
                            if data.words[f + 2].is_some() {
                                let v = word_get(data, f + 2, src);
                                word_set(data, f + 2, dst, v);
                            }
                        }
                        if data.pitch.is_some() {
                            let mut row = [0u16; 4];
                            row.copy_from_slice(
                                &data.pitch.as_deref().unwrap()[src * 4..src * 4 + 4],
                            );
                            data.pitch.as_deref_mut().unwrap()[dst * 4..dst * 4 + 4]
                                .copy_from_slice(&row);
                        }
                        if data.contexts.is_some() {
                            let mut row = [0u8; 9];
                            row.copy_from_slice(
                                &data.contexts.as_deref().unwrap()[src * 9..src * 9 + 9],
                            );
                            data.contexts.as_deref_mut().unwrap()[dst * 9..dst * 9 + 9]
                                .copy_from_slice(&row);
                        }
                        if data.timings.is_some() {
                            let mut row = [0i32; 2];
                            row.copy_from_slice(
                                &data.timings.as_deref().unwrap()[src * 2..src * 2 + 2],
                            );
                            data.timings.as_deref_mut().unwrap()[dst * 2..dst * 2 + 2]
                                .copy_from_slice(&row);
                        }
                        let ac = usize::from(voice.acoustic_count);
                        for d in 0..ac {
                            let v = data.acoustic.as_deref().unwrap()[src * ac + d];
                            data.acoustic.as_deref_mut().unwrap()[dst * ac + d] = v;
                            let v = data.modes.as_deref().unwrap()[src * ac + d];
                            data.modes.as_deref_mut().unwrap()[dst * ac + d] = v;
                        }
                        let dc = usize::from(voice.duration_count);
                        for f in 0..dc {
                            let v = data.duration.as_deref().unwrap()[src * dc + f];
                            data.duration.as_deref_mut().unwrap()[dst * dc + f] = v;
                            let v = data.alternate_duration.as_deref().unwrap()[src * dc + f];
                            data.alternate_duration.as_deref_mut().unwrap()[dst * dc + f] = v;
                        }
                        n = n.wrapping_add(1) & 0xffff;
                        if !((n as i32) < i32::from(data.count) - index as i32) {
                            break;
                        }
                    }
                }
                data.count =
                    (i32::from(data.count) - (index as i16) as i32 + (item as i16) as i32) as i16;
                index = item;
            }
            item = index.wrapping_add(1) & 0xffff;
            if !((item as i32) < i32::from(data.count)) {
                break;
            }
        }
    }
    data.timing_count = data.count.wrapping_mul(2);
    if data.marker_count != 0 && !data.markers.is_empty() && feat_present(data, 1) {
        let mut marker: u8 = 0;
        let mut has_kind_8 = 0;
        let total = feat_get(data, 5, 0);
        while marker < total {
            if data.markers[marker as usize].kind == 26 {
                let mut next = marker;
                while next < total {
                    if data.markers[next as usize].kind == 8 {
                        has_kind_8 = 1;
                    }
                    next = next.wrapping_add(1);
                }
                if has_kind_8 == 0 {
                    let v = feat_get(data, 5, 1)
                        .wrapping_add(feat_get(data, 5, 0).wrapping_sub(marker));
                    feat_set(data, 5, 1, v);
                    feat_set(data, 5, 0, marker);
                }
                break;
            }
            marker = marker.wrapping_add(1);
        }
    }
    if host.log_level() > 5 {
        let mut feature_name = [0u8; 64];
        let mut message_name = [0u8; 16];
        render_u32(78, &mut message_name);
        let message_len = crate::resources::string_length(&message_name);
        let mut feature: i16 = 0;
        while feature < 60 {
            let f = feature as usize;
            let mut width = voice.feature_widths[f];
            let mut count_d = data.count as u32;
            let mut kind = 99u32;
            let mut mime: &[u8] = &[];
            let mut name_len = 0usize;
            let mut is_marker_counts = false;
            match feature {
                0 | 2 | 3 | 4 | 5 => width = 0,
                1 => {
                    count_d = data.length as u32;
                    is_marker_counts = true;
                }
                _ => {
                    if width == 0 {
                        feature += 1;
                        continue;
                    }
                    let prefix: &[u8] = if width == 1 {
                        b"application/x-realspeak-u8-feat-"
                    } else if width == 2 {
                        b"application/x-realspeak-u16-feat-"
                    } else {
                        b"application/x-realspeak-u32-feat-"
                    };
                    feature_name[..prefix.len()].copy_from_slice(prefix);
                    let n = render_u32(feature as u32, &mut feature_name[prefix.len()..]);
                    name_len = prefix.len() + n;
                    kind = 100;
                }
            }
            if width == 0 {
                if feature != 1 || !feat_present(data, 1) {
                    feature += 1;
                    continue;
                }
            } else if width == 1 {
            } else if width == 2 {
                mime = if is_marker_counts {
                    b"application/x-realspeak-marker-counts;version=4.0"
                } else {
                    &feature_name[..name_len]
                };
                if f == 46 {
                    // `buffer` is the contexts plane: `count * 2` raw bytes.
                    let n = count_d.wrapping_mul(2) as usize;
                    let bytes = match data.contexts.as_deref() {
                        Some(p) => DumpBytes::B8(&p[..n]),
                        None => DumpBytes::Null(n),
                    };
                    host.write_output(request, kind, &message_name[..message_len], mime, bytes);
                } else {
                    let words = match f {
                        24 => match data.pitch.as_deref() {
                            Some(p) => DumpBytes::B16(&p[..count_d as usize]),
                            None => DumpBytes::Null(count_d.wrapping_mul(2) as usize),
                        },
                        _ => match data.words[f].as_deref() {
                            Some(p) => DumpBytes::B16(&p[..count_d as usize]),
                            None => DumpBytes::Null(count_d.wrapping_mul(2) as usize),
                        },
                    };
                    host.write_output(request, kind, &message_name[..message_len], mime, words);
                }
                feature += 1;
                continue;
            } else if width == 4 {
                mime = if is_marker_counts {
                    b"application/x-realspeak-marker-counts;version=4.0"
                } else {
                    &feature_name[..name_len]
                };
                let ac = usize::from(voice.acoustic_count);
                let dc = usize::from(voice.duration_count);
                if f == 24 {
                    // `buffer` is the pitch plane: `count * 4` raw bytes.
                    let n = count_d.wrapping_mul(4) as usize;
                    let bytes = match data.pitch.as_deref() {
                        Some(p) => DumpBytes::PitchBytes { pitch: p, len: n },
                        None => DumpBytes::Null(n),
                    };
                    host.write_output(request, kind, &message_name[..message_len], mime, bytes);
                } else if f == 46 {
                    // `buffer` is the contexts plane: `count * 4` raw bytes.
                    let n = count_d.wrapping_mul(4) as usize;
                    let bytes = match data.contexts.as_deref() {
                        Some(p) => DumpBytes::B8(&p[..n]),
                        None => DumpBytes::Null(n),
                    };
                    host.write_output(request, kind, &message_name[..message_len], mime, bytes);
                } else {
                    let bytes = match f {
                        52 => {
                            let elems = count_d.wrapping_mul(ac as u32) as usize;
                            match data.acoustic.as_deref() {
                                Some(p) => DumpBytes::B32(&p[..elems]),
                                None => DumpBytes::Null(elems * 4),
                            }
                        }
                        53 => {
                            let elems = count_d.wrapping_mul(dc as u32) as usize;
                            match data.duration.as_deref() {
                                Some(p) => DumpBytes::B32(&p[..elems]),
                                None => DumpBytes::Null(elems * 4),
                            }
                        }
                        54 => {
                            let elems = count_d.wrapping_mul(dc as u32) as usize;
                            match data.alternate_duration.as_deref() {
                                Some(p) => DumpBytes::B32(&p[..elems]),
                                None => DumpBytes::Null(elems * 4),
                            }
                        }
                        _ => DumpBytes::Null(count_d.wrapping_mul(4) as usize),
                    };
                    host.write_output(request, kind, &message_name[..message_len], mime, bytes);
                }
                feature += 1;
                continue;
            } else {
                feature += 1;
                continue;
            }
            mime = if is_marker_counts {
                b"application/x-realspeak-marker-counts;version=4.0"
            } else {
                &feature_name[..name_len]
            };
            if f == 24 {
                // `buffer` is the pitch plane: `count` raw bytes.
                let bytes = match data.pitch.as_deref() {
                    Some(p) => DumpBytes::PitchBytes {
                        pitch: p,
                        len: count_d as usize,
                    },
                    None => DumpBytes::Null(count_d as usize),
                };
                host.write_output(request, kind, &message_name[..message_len], mime, bytes);
            } else {
                let bytes = match f {
                    1 => match data.bytes[1].as_deref() {
                        Some(p) => DumpBytes::B8(&p[4..4 + count_d as usize]),
                        None => DumpBytes::Null(count_d as usize),
                    },
                    46 => match data.contexts.as_deref() {
                        Some(p) => DumpBytes::B8(&p[..count_d as usize]),
                        None => DumpBytes::Null(count_d as usize),
                    },
                    56 => {
                        let n = count_d.wrapping_mul(voice.acoustic_count as u32) as usize;
                        match data.modes.as_deref() {
                            Some(p) => DumpBytes::B8(&p[..n]),
                            None => DumpBytes::Null(n),
                        }
                    }
                    _ => match data.bytes[f].as_deref() {
                        Some(p) => DumpBytes::B8(&p[..count_d as usize]),
                        None => DumpBytes::Null(count_d as usize),
                    },
                };
                host.write_output(request, kind, &message_name[..message_len], mime, bytes);
            }
            feature += 1;
        }
    }
    result
}
