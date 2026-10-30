/// fn_1f1430, EU fn_1f15ec, KR fn_1f1698.
pub fn compare_priority(left: u16, right: u16) -> i32 {
    i32::from(right) - i32::from(left)
}

fn validate(check: impl FnOnce(u32, u32) -> u32, tag: u32, size: u32, error: u32) -> u32 {
    let status = check(tag, size);
    if status >= 0x80000000 {
        error
    } else {
        status
    }
}

/// fn_1f13f8, EU fn_1f15b4, KR fn_1f1660.
pub fn reset_pos(check: impl FnOnce(u32, u32) -> u32) -> u32 {
    validate(check, 0xf385, 32, 0x89e02008)
}

/// fn_1f1414, EU fn_1f15d0, KR fn_1f167c.
pub fn prepare_pos(check: impl FnOnce(u32, u32) -> u32) -> u32 {
    validate(check, 0xf385, 32, 0x89e02008)
}

/// fn_1f27ec, EU fn_1f29a8, KR fn_1f2a54.
pub fn finish_pos(check: impl FnOnce(u32, u32) -> u32) -> u32 {
    validate(check, 0xf385, 32, 0x89e02008)
}

/// fn_1f2bf0, EU fn_1f2dac, KR fn_1f2e58.
pub fn prepare_homographs(check: impl FnOnce(u32, u32) -> u32) -> u32 {
    validate(check, 0xf386, 48, 0x89f02008)
}

/// fn_1f3a8c, EU fn_1f3c48, KR fn_1f3cf4.
pub fn finish_homographs(check: impl FnOnce(u32, u32) -> u32) -> u32 {
    validate(check, 0xf386, 48, 0x89f02008)
}

/// fn_1f2808, EU fn_1f29c4, KR fn_1f2a70.
pub fn pos_interface(selector: u32, table: &[u8]) -> Result<&[u8], u32> {
    if selector <= 1 {
        Ok(table)
    } else {
        Err(0x89e02001)
    }
}

/// fn_1f3aa8, EU fn_1f3c64, KR fn_1f3d10.
pub fn homograph_interface(selector: u32, table: &[u8]) -> Result<&[u8], u32> {
    if selector <= 1 {
        Ok(table)
    } else {
        Err(0x89f02001)
    }
}

use crate::frontend_types::{Error, TokenId};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u32)]
pub enum StageOperation {
    Validate,
    First,
    Next,
    Value,
    Read,
    Text,
    Set,
    Remove,
    SetRule,
    GetRule,
    ProcessRule,
    Classify,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct StageReply {
    pub status: u32,
    pub count: u16,
    pub bytes: usize,
}

pub trait StageIo {
    fn call(
        &mut self,
        operation: StageOperation,
        arguments: [u32; 3],
        input: &[u8],
        output: &mut [u8],
    ) -> Result<StageReply, Error>;
    fn truncate_text(&mut self, node: TokenId, field: u16) -> Result<(), Error>;
}

pub struct StageScratch<'a> {
    pub text: &'a mut [u8],
    pub markers: &'a mut [u8],
    pub lines: [&'a mut [u8]; 3],
    pub rule_output: [&'a mut [u8]; 2],
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct HomographState {
    pub capacity: u16,
    pub limit: u16,
    pub preserve: bool,
}

/// Caller-owned FE_POS / FE_HMOGRPH environment view, resolved through
/// `fn_19611c`. `id` is the opaque address the original stored in its objects.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct PosEnv {
    pub id: u32,
    pub heap: u32,
    pub log: u32,
    pub params: u32,
    pub resources: u32,
}

/// One acquired component: its interface pointer and two-word handle.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct PosResource {
    pub interface: u32,
    pub handle: [u32; 2],
}

/// Two-word component handle published by an open entry point.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct PosHandle {
    pub module: u32,
    pub tag: u32,
}

/// FE_POS module instance (`fn_1f1224`), 32 bytes in the original. `id` is the
/// caller-owned object address and is not part of the original field layout.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct PosModule {
    pub id: u32,
    pub env: PosEnv,
    pub ling: u32,
    pub depes: [u32; 2],
    pub methods: u32,
    pub line1: u32,
    pub line2: u32,
    pub line3: u32,
}

impl PosModule {
    /// Original 32-byte field order, for differential comparison.
    pub fn words(&self) -> [u32; 8] {
        [
            self.env.id,
            self.ling,
            self.depes[0],
            self.depes[1],
            self.methods,
            self.line1,
            self.line2,
            self.line3,
        ]
    }
}

/// FE_HMOGRPH module instance (`fn_1f2830`), 48 bytes in the original. `id` is
/// the caller-owned object address and is not part of the original layout.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct HomographModule {
    pub id: u32,
    pub env: PosEnv,
    pub tokens: u32,
    pub depes: [u32; 2],
    pub depes_iface: u32,
    pub dctl: [u32; 2],
    pub dctl_iface: u32,
    pub text_buffer: u32,
    pub record_buffer: u32,
    pub state: HomographState,
}

impl HomographModule {
    /// Original 48-byte field order, for differential comparison.
    pub fn words(&self) -> [u32; 12] {
        [
            self.env.id,
            self.tokens,
            self.depes[0],
            self.depes[1],
            self.depes_iface,
            self.dctl[0],
            self.dctl[1],
            self.dctl_iface,
            self.text_buffer,
            self.record_buffer,
            u32::from(self.state.capacity) | (u32::from(self.state.limit) << 16),
            u32::from(self.state.preserve),
        ]
    }
}

/// Engine services the FE_POS and FE_HMOGRPH components borrow from their host.
///
/// The host owns every allocation, every loaded resource and the engine
/// context, so these components keep only the opaque addresses the original
/// stored in their objects. Logging is host work and is modelled here only so
/// the differential adapters can compare the call.
pub trait PosHost {
    /// fn_19611c: resolve the engine context a component request names.
    fn resolve_env(&mut self, a: u32, b: u32) -> (u32, Option<PosEnv>);
    /// fn_17b058: acquire the named resource and fill its interface and handle.
    fn acquire(&mut self, resources: u32, name: &[u8]) -> (u32, Option<PosResource>);
    /// fn_17b1c4: release the named resource.
    fn release(&mut self, resources: u32, name: &[u8]);
    /// fn_1963b8: `count` items of `size` bytes. Zero when the allocator refuses.
    fn allocate(&mut self, heap: u32, count: u32, size: u32) -> u32;
    /// fn_196394: `size` bytes. Zero when the allocator refuses.
    fn allocate_block(&mut self, heap: u32, size: u32) -> u32;
    /// fn_196428: release an allocation.
    fn free(&mut self, heap: u32, address: u32);
    /// fn_173dc0: report a component error.
    fn log_error(&mut self, log: u32, tag: &[u8], line: u32);
    /// fn_1756d0: read the named configuration string. `None` is a null result.
    fn get_string(&mut self, params: u32, key: &[u8]) -> (u32, Option<u32>);
    /// fn_17c550: parse the decimal value of a configuration string.
    fn parse_number(&mut self, string: u32) -> u32;
    /// fn_195d90: the length of a NUL-terminated string.
    fn string_length(&mut self, string: u32) -> u32;
    /// fn_176f94: validate a component handle against `expected` and `size`.
    fn validate(&mut self, object: u32, tag: u32, expected: u32, size: u32) -> u32;
    /// Publish a freshly built FE_POS module into the caller-owned object.
    fn install_pos(&mut self, object: u32, module: PosModule);
    /// Publish a freshly built FE_HMOGRPH module into the caller-owned object.
    fn install_homographs(&mut self, object: u32, module: HomographModule);
}

/// The `numberoutputtranscriptions` and `mlset` reads `fn_1f2830` and
/// `fn_1f2b3c` share. Returns the status the original returns.
fn read_homograph_config(env: PosEnv, host: &mut impl PosHost, state: &mut HomographState) -> u32 {
    let (status, string) = host.get_string(env.params, b"numberoutputtranscriptions");
    state.limit = if status < 0x80000000 {
        string.map_or(0, |string| host.parse_number(string) as u16)
    } else {
        0
    };
    let (status, string) = host.get_string(env.params, b"mlset");
    if status < 0x80000000 && host.string_length(string.unwrap_or(0)) > 3 {
        state.preserve = true;
        status
    } else {
        state.preserve = false;
        0
    }
}

/// fn_1f1224 / EU fn_1f13e0 / KR fn_1f148c.
pub fn open_pos(a: u32, b: u32, host: &mut impl PosHost, out: Option<&mut PosHandle>) -> u32 {
    let Some(out) = out else {
        return 0x89e02007;
    };
    *out = PosHandle::default();
    let (status, env) = host.resolve_env(a, b);
    let Some(env) = env else { return status };
    let (status, ling) = host.acquire(env.resources, b"LINGDB");
    let Some(ling) = ling else { return status };
    let (status, depes) = host.acquire(env.resources, b"FE_DEPES");
    let Some(depes) = depes else {
        host.release(env.resources, b"LINGDB");
        return status;
    };
    let object = host.allocate(env.heap, 1, 32);
    if object == 0 {
        host.log_error(env.log, b"FE_POS", 0x88b8);
        host.release(env.resources, b"LINGDB");
        host.release(env.resources, b"FE_DEPES");
        return 0x89e0200a;
    }
    host.install_pos(
        object,
        PosModule {
            id: object,
            env,
            ling: ling.interface,
            depes: depes.handle,
            methods: depes.interface,
            line1: 0,
            line2: 0,
            line3: 0,
        },
    );
    out.module = object;
    out.tag = 0xf385;
    status
}

/// fn_1f1398 / EU fn_1f1554 / KR fn_1f1600.
pub fn close_pos(module: Option<&PosModule>, tag: u32, host: &mut impl PosHost) -> u32 {
    let object = module.map_or(0, |module| module.id);
    if host.validate(object, tag, 0xf385, 32) >= 0x80000000 {
        return 0x89e02008;
    }
    let Some(module) = module else { return 0 };
    host.release(module.env.resources, b"LINGDB");
    host.release(module.env.resources, b"FE_DEPES");
    host.free(module.env.heap, module.id);
    0
}

/// fn_1f2830 / EU fn_1f29ec / KR fn_1f2a98.
pub fn open_homographs(
    a: u32,
    b: u32,
    host: &mut impl PosHost,
    out: Option<&mut PosHandle>,
) -> u32 {
    let Some(out) = out else {
        return 0x89f02007;
    };
    *out = PosHandle::default();
    let (status, env) = host.resolve_env(a, b);
    let Some(env) = env else { return status };
    let (status, ling) = host.acquire(env.resources, b"LINGDB");
    let Some(ling) = ling else { return status };
    let (status, dctl) = host.acquire(env.resources, b"FE_DCTLKP");
    let Some(dctl) = dctl else {
        host.release(env.resources, b"LINGDB");
        return status;
    };
    let (status, depes) = host.acquire(env.resources, b"FE_DEPES");
    let Some(depes) = depes else {
        host.release(env.resources, b"LINGDB");
        host.release(env.resources, b"FE_DCTLKP");
        return status;
    };
    let object = host.allocate_block(env.heap, 48);
    if object == 0 {
        host.log_error(env.log, b"FE_HMOGRPH", 0x8ca0);
        host.release(env.resources, b"LINGDB");
        host.release(env.resources, b"FE_DEPES");
        host.release(env.resources, b"FE_DCTLKP");
        return 0x89f0200a;
    }
    let mut module = HomographModule {
        id: object,
        env,
        tokens: ling.interface,
        depes: depes.handle,
        depes_iface: depes.interface,
        dctl: dctl.handle,
        dctl_iface: dctl.interface,
        ..HomographModule::default()
    };
    let status = read_homograph_config(env, host, &mut module.state);
    host.install_homographs(object, module);
    out.module = object;
    out.tag = 0xf386;
    status
}

/// fn_1f2acc / EU fn_1f2c88 / KR fn_1f2d34.
pub fn close_homographs(
    module: Option<&HomographModule>,
    tag: u32,
    host: &mut impl PosHost,
) -> u32 {
    let object = module.map_or(0, |module| module.id);
    if host.validate(object, tag, 0xf386, 48) >= 0x80000000 {
        return 0x89f02008;
    }
    let Some(module) = module else { return 0 };
    host.release(module.env.resources, b"LINGDB");
    host.release(module.env.resources, b"FE_DEPES");
    host.release(module.env.resources, b"FE_DCTLKP");
    host.free(module.env.heap, module.id);
    0
}

/// fn_1f2b3c / EU fn_1f2cf8 / KR fn_1f2da4.
pub fn reset_homographs(
    module: Option<&mut HomographModule>,
    tag: u32,
    host: &mut impl PosHost,
) -> u32 {
    let object = module.as_ref().map_or(0, |module| module.id);
    if host.validate(object, tag, 0xf386, 48) >= 0x80000000 {
        return 0x89f02008;
    }
    let Some(module) = module else { return 0 };
    let env = module.env;
    read_homograph_config(env, host, &mut module.state)
}

fn invoke(
    io: &mut impl StageIo,
    operation: StageOperation,
    arguments: [u32; 3],
    input: &[u8],
    output: &mut [u8],
) -> Result<StageReply, Error> {
    let reply = io.call(operation, arguments, input, output)?;
    if reply.status >= 0x80000000 {
        Err(Error::Engine(reply.status))
    } else {
        Ok(reply)
    }
}

fn scalar(
    io: &mut impl StageIo,
    operation: StageOperation,
    a: u32,
    b: u32,
) -> Result<(u32, u16), Error> {
    let mut bytes = [0; 2];
    let reply = invoke(
        io,
        operation,
        [a, b, u32::from(operation == StageOperation::Read)],
        &[],
        &mut bytes,
    )?;
    Ok((reply.status, u16::from_le_bytes(bytes)))
}

fn field(io: &mut impl StageIo, node: u16, index: u16) -> Result<u16, Error> {
    scalar(io, StageOperation::Read, u32::from(node), u32::from(index)).map(|(_, value)| value)
}

fn ctext(bytes: &[u8]) -> &[u8] {
    &bytes[..bytes.iter().position(|&b| b == 0).unwrap_or(bytes.len())]
}

fn append(buffer: &mut [u8], bytes: &[u8]) -> Result<(), Error> {
    let start = ctext(buffer).len();
    let required = start + bytes.len() + 1;
    if required > buffer.len() {
        return Err(Error::Capacity {
            required,
            available: buffer.len(),
        });
    }
    buffer[start..start + bytes.len()].copy_from_slice(bytes);
    buffer[start + bytes.len()] = 0;
    Ok(())
}

fn repeat(buffer: &mut [u8], byte: u8, count: usize) -> Result<(), Error> {
    for _ in 0..count {
        append(buffer, &[byte])?;
    }
    Ok(())
}

fn text(
    io: &mut impl StageIo,
    node: u16,
    index: u16,
    buffer: &mut [u8],
) -> Result<StageReply, Error> {
    invoke(
        io,
        StageOperation::Text,
        [u32::from(node), u32::from(index), 0],
        &[],
        buffer,
    )
}

fn set(
    io: &mut impl StageIo,
    node: u16,
    index: u16,
    count: u16,
    bytes: &[u8],
) -> Result<u32, Error> {
    invoke(
        io,
        StageOperation::Set,
        [u32::from(node), u32::from(index), u32::from(count)],
        bytes,
        &mut [],
    )
    .map(|r| r.status)
}

fn set_rule(io: &mut impl StageIo, line: u32, buffer: &[u8]) -> Result<u32, Error> {
    let bytes = ctext(buffer);
    let count = usize::from(bytes.len() as u16);
    invoke(
        io,
        StageOperation::SetRule,
        [line, count as u32, 0],
        &bytes[..count],
        &mut [],
    )
    .map(|r| r.status)
}

/// fn_1f2c0c, EU fn_1f2dc8, KR fn_1f2e74.
pub fn validate_homograph(
    io: &mut impl StageIo,
    state: &mut HomographState,
    input: &[u8],
    chosen: TokenId,
    bounds: [u16; 4],
    lines: &mut [&mut [u8]; 3],
    output: &mut [u8],
) -> Result<u32, Error> {
    let [lower, upper, mut start, mut end] = bounds;
    let (mut status, root) = scalar(io, StageOperation::First, 1, 0)?;
    let mut count = [0; 2];
    let reply = io.call(
        StageOperation::Value,
        [u32::from(chosen.0), 4, 0],
        &[],
        &mut count,
    )?;
    if reply.status >= 0x80000000 || u16::from_le_bytes(count) != 1 {
        return Ok(status);
    }
    let [line0, line1, transcription_buffer] = lines;
    status = text(io, chosen.0, 4, transcription_buffer)?.status;
    let transcription = ctext(transcription_buffer);
    if transcription.is_empty() {
        return Ok(status);
    }
    let required = usize::from(upper.wrapping_sub(lower)) + 12;
    for available in [line0.len(), line1.len()] {
        if required > available {
            return Err(Error::Capacity {
                required,
                available,
            });
        }
    }
    if required > usize::from(state.capacity) {
        state.capacity = required as u16;
    }
    line0[0] = 0;
    line1[0] = 0;
    let (_, mut node) = scalar(io, StageOperation::First, 2, u32::from(root))?;
    let mut position = lower;
    let mut padding = 0u8;
    while node != 0 {
        if node == chosen.0 {
            while io
                .call(
                    StageOperation::Classify,
                    [u32::from(input[usize::from(start)]), 0, 0],
                    &[],
                    &mut [],
                )?
                .status
                != 0
            {
                start = start.wrapping_add(1);
            }
            let mut last = end.wrapping_sub(1);
            while io
                .call(
                    StageOperation::Classify,
                    [u32::from(input[usize::from(last)]), 0, 0],
                    &[],
                    &mut [],
                )?
                .status
                != 0
            {
                last = last.wrapping_sub(1);
            }
            end = last.wrapping_add(1);
            let prefix = start.wrapping_sub(position);
            append(line0, &input[usize::from(position)..usize::from(start)])?;
            repeat(line1, b' ', usize::from(prefix))?;
            append(line0, &[21])?;
            append(line0, &input[usize::from(start)..usize::from(end)])?;
            let span = end.wrapping_sub(start);
            if (transcription.len() as i16) > span as i16 {
                let extra = (transcription.len() as u16).wrapping_sub(span);
                while u16::from(padding) < extra && padding < 10 {
                    append(line0, b"~")?;
                    padding = padding.wrapping_add(1);
                }
            }
            append(line0, &[21])?;
            append(line1, &[21])?;
            append(line1, transcription)?;
            if (span as i16) > transcription.len() as i16 {
                repeat(
                    line1,
                    b'~',
                    usize::from(span.wrapping_sub(transcription.len() as u16)),
                )?;
            }
            append(line1, &[21])?;
            position = end;
        }
        node = scalar(io, StageOperation::Next, u32::from(node), 0)?.1;
    }
    if position < upper {
        append(line0, &input[usize::from(position)..usize::from(upper)])?;
        repeat(line1, b' ', usize::from(upper - position))?;
    }
    if padding < 10 {
        set_rule(io, 0, line0)?;
        set_rule(io, 1, line1)?;
        invoke(
            io,
            StageOperation::ProcessRule,
            [0; 3],
            b"disambiguate_homographs\0",
            &mut [],
        )?;
        let reply = invoke(io, StageOperation::GetRule, [0; 3], &[], output)?;
        status = reply.status;
        output[usize::from(reply.count)] = 0;
        if !ctext(output).contains(&21) {
            return Ok(status);
        }
    }
    set(io, chosen.0, 7, 1, &[0, 0])
}

fn whitespace(byte: u8) -> bool {
    matches!(byte, b' ' | b'\t' | b'\n' | b'\r' | b'_')
}

/// fn_1f3248, EU fn_1f3404, KR fn_1f34b0.
pub fn process_homographs(
    io: &mut impl StageIo,
    state: &mut HomographState,
    applied: &mut u8,
    scratch: &mut StageScratch<'_>,
) -> Result<u32, Error> {
    let checked = io.call(StageOperation::Validate, [0xf386, 48, 0], &[], &mut [])?;
    if checked.status >= 0x80000000 {
        return Err(Error::Engine(0x89f02008));
    }
    *applied = 1;
    let (status, root) = scalar(io, StageOperation::First, 1, 0)?;
    let mut attributes = [0; 2];
    let reply = io.call(
        StageOperation::Value,
        [u32::from(root), 0, 0],
        &[],
        &mut attributes,
    )?;
    if reply.status >= 0x80000000 || u16::from_le_bytes(attributes) != 1 {
        return Ok(status);
    }
    let reply = text(io, root, 0, scratch.text)?;
    if reply.count <= 1 {
        return Ok(reply.status);
    }
    let text_last = reply.count - 1;
    state.capacity = 0;
    let result = (|| {
        let markers = text(io, root, 1, scratch.markers)?.count;
        let base_offset = i32::from_le_bytes(scratch.markers[12..16].try_into().unwrap());
        let (_, mut token) = scalar(io, StageOperation::First, 2, u32::from(root))?;
        let mut previous = None;
        let mut chosen = 0;
        while token != 0 {
            if field(io, token, 0)? == 4 {
                let start = field(io, token, 1)?;
                if previous == Some(start) {
                    let end = field(io, token, 2)?;
                    let mut lower = start;
                    let mut boundaries = 0u8;
                    while lower != 0 {
                        lower -= 1;
                        let i = usize::from(lower);
                        if whitespace(scratch.text[i]) && !whitespace(scratch.text[i + 1]) {
                            boundaries = boundaries.wrapping_add(1);
                        }
                        if boundaries >= 4 {
                            break;
                        }
                    }
                    if lower != 0 {
                        lower += 1;
                    }
                    let mut upper = end;
                    boundaries = 0;
                    while upper < text_last {
                        upper += 1;
                        let i = usize::from(upper);
                        if whitespace(scratch.text[i]) && !whitespace(scratch.text[i - 1]) {
                            boundaries = boundaries.wrapping_add(1);
                        }
                        if boundaries >= 3 {
                            break;
                        }
                    }
                    for node in [chosen, token] {
                        if node != 0 {
                            validate_homograph(
                                io,
                                state,
                                scratch.text,
                                TokenId(node),
                                [lower, upper, start, end],
                                &mut scratch.lines,
                                scratch.rule_output[0],
                            )?;
                        }
                    }
                    chosen = 0;
                } else {
                    chosen = token;
                }
                previous = Some(start);
            }
            token = scalar(io, StageOperation::Next, u32::from(token), 0)?.1;
        }
        let (mut status, mut token) = scalar(io, StageOperation::First, 2, u32::from(root))?;
        previous = None;
        chosen = 0;
        let mut best = 0;
        let mut protected = false;
        let mut base_voice = 1;
        let mut same_voice = false;
        let mut last_voice = 1;
        let mut voice = 0;
        let mut record = 0usize;
        while token != 0 {
            let kind = field(io, token, 0)?;
            let start = field(io, token, 1)?;
            if previous != Some(start) {
                protected = false;
            }
            while record < usize::from(markers) {
                let offset = record * 24;
                let position = i32::from_le_bytes(
                    scratch.markers[offset + 12..offset + 16]
                        .try_into()
                        .unwrap(),
                );
                if position.wrapping_sub(base_offset) as i16 >= start.wrapping_add(1) as i16 {
                    break;
                }
                let kind =
                    u16::from_le_bytes(scratch.markers[offset..offset + 2].try_into().unwrap());
                let value = u32::from_le_bytes(
                    scratch.markers[offset + 20..offset + 24]
                        .try_into()
                        .unwrap(),
                );
                if kind == 14 && value == 64 {
                    protected = true;
                }
                record += 1;
            }
            let mut discard = 0;
            if kind == 4 {
                let score = field(io, token, 7)?;
                let mut attributes = [0; 2];
                let reply = io.call(
                    StageOperation::Value,
                    [u32::from(token), 9, 0],
                    &[],
                    &mut attributes,
                )?;
                if reply.status < 0x80000000 && u16::from_le_bytes(attributes) == 1 {
                    let mut data = [0; 256];
                    text(io, token, 9, &mut data)?;
                    voice = u32::from(data[0]);
                }
                if previous == Some(start) {
                    let better = score > best
                        || (score == best && !same_voice && (voice == base_voice || voice != 101));
                    if better != protected {
                        best = score;
                        discard = chosen;
                        chosen = token;
                        same_voice = voice == base_voice;
                        last_voice = if voice == 0 { 1 } else { voice };
                    } else {
                        discard = token;
                    }
                } else {
                    base_voice = last_voice;
                    best = score;
                    chosen = token;
                    same_voice = voice == last_voice;
                    if voice != 0 {
                        last_voice = voice;
                    }
                }
                previous = Some(start);
            }
            (status, token) = scalar(io, StageOperation::Next, u32::from(token), 0)?;
            if discard != 0 {
                status = if state.limit > 1 && state.preserve {
                    set(io, discard, 0, 1, &[15])?
                } else {
                    invoke(
                        io,
                        StageOperation::Remove,
                        [u32::from(discard), 0, 0],
                        &[],
                        &mut [],
                    )?
                    .status
                };
            }
        }
        Ok(status)
    })();
    result
}

#[derive(Clone, Copy, Default)]
struct PosRecord {
    left: [u8; 10],
    right: [u8; 10],
    priority: u16,
}

fn sort_records(records: &mut [PosRecord]) {
    let n = records.len();
    if n <= 1 {
        return;
    }
    let mut mid = n / 2;
    let mut i = 0usize;
    let mut j = n - 1;
    while i < j {
        while records[i].priority > records[mid].priority {
            i += 1;
        }
        while records[j].priority < records[mid].priority {
            j -= 1;
        }
        if i < j {
            records.swap(i, j);
            if mid == i {
                mid = j;
            } else if mid == j {
                mid = i;
            }
            i += 1;
            j = j.wrapping_sub(1);
        } else if i == j {
            i += 1;
            j = j.wrapping_sub(1);
            break;
        }
    }
    if j != 0 {
        sort_records(&mut records[..j + 1]);
    }
    if i < n - 1 {
        sort_records(&mut records[i..]);
    }
}

fn flush_records(
    records: &mut [PosRecord],
    lines: &mut [&mut [u8]; 3],
    final_group: bool,
) -> Result<(), Error> {
    sort_records(records);
    for (i, record) in records.iter().enumerate() {
        for (column, bytes) in [(1, ctext(&record.left)), (2, ctext(&record.right))] {
            let existing = ctext(lines[column]);
            let separator = if final_group {
                column == 1 || !existing.is_empty()
            } else {
                !existing.is_empty() && existing.last() != Some(&b' ')
            };
            if i != 0 && !bytes.is_empty() && separator {
                append(lines[column], b"\\")?;
            }
            append(lines[column], bytes)?;
        }
    }
    Ok(())
}

fn pad_lines(lines: &mut [&mut [u8]; 3], length: [u16; 2], span: u16) -> Result<(), Error> {
    if length[0] > span || length[1] > span {
        repeat(lines[0], b'~', usize::from(length[0].max(length[1]) - span))?;
        if length[1] > length[0] {
            repeat(lines[1], b'~', usize::from(length[1] - length[0]))?;
        } else {
            repeat(lines[2], b'~', usize::from(length[0] - length[1]))?;
        }
    } else {
        repeat(lines[1], b'~', usize::from(span - length[0]))?;
        repeat(lines[2], b'~', usize::from(span - length[1]))?;
    }
    Ok(())
}

fn next_rule_token(bytes: &mut [u8], cursor: &mut usize) -> (usize, usize) {
    while *cursor < bytes.len() && matches!(bytes[*cursor], b'~' | b' ') {
        *cursor += 1;
    }
    let start = *cursor;
    while *cursor < bytes.len() && !matches!(bytes[*cursor], 0 | b'~' | b' ') {
        *cursor += 1;
    }
    let end = *cursor;
    if *cursor < bytes.len() && bytes[*cursor] != 0 {
        // The ROM NUL-terminates the token in the rule buffer (the
        // `*current = 0` / `outputRight[-1] = 0` stores of fn_1f1440).
        bytes[*cursor] = 0;
        *cursor += 1;
    }
    (start, end)
}

fn flush_pending(
    io: &mut impl StageIo,
    nodes: &[u16],
    matched: bool,
) -> Result<Option<u32>, Error> {
    let mut status = None;
    for &node in nodes {
        if matched {
            status = Some(
                invoke(
                    io,
                    StageOperation::Remove,
                    [u32::from(node), 0, 0],
                    &[],
                    &mut [],
                )?
                .status,
            );
        } else {
            for index in [5, 6] {
                let mut data = [0; 256];
                status = Some(text(io, node, index, &mut data)?.status);
                io.truncate_text(TokenId(node), index)?;
            }
        }
    }
    Ok(status)
}

/// fn_1f1440, EU fn_1f15fc, KR fn_1f16a8.
pub fn process_pos(
    io: &mut impl StageIo,
    applied: &mut u8,
    scratch: &mut StageScratch<'_>,
) -> Result<u32, Error> {
    let checked = io.call(StageOperation::Validate, [0xf385, 32, 0], &[], &mut [])?;
    if checked.status >= 0x80000000 {
        return Err(Error::Engine(0x89e02008));
    }
    *applied = 1;
    let (status, root) = scalar(io, StageOperation::First, 1, 0)?;
    let mut enabled = [0; 2];
    let reply = io.call(
        StageOperation::Value,
        [u32::from(root), 0, 0],
        &[],
        &mut enabled,
    )?;
    if reply.status >= 0x80000000 || u16::from_le_bytes(enabled) != 1 {
        return Ok(status);
    }
    let reply = text(io, root, 0, scratch.text)?;
    if reply.count <= 1 {
        return Ok(reply.status);
    }
    for line in &mut scratch.lines {
        if line.is_empty() {
            return Err(Error::Capacity {
                required: 1,
                available: 0,
            });
        }
        line[0] = 0;
    }
    let (_, mut node) = scalar(io, StageOperation::First, 2, u32::from(root))?;
    let mut records = [PosRecord::default(); 10];
    let mut count = 0usize;
    let mut previous = None;
    let mut lengths = [0u16; 2];
    let mut span = 0;
    while node != 0 {
        let kind = field(io, node, 0)?;
        if matches!(kind, 4 | 9 | 10) {
            let start = field(io, node, 1)?;
            let end = field(io, node, 2)?;
            let mut left = [0; 10];
            let mut right = [0; 10];
            text(io, node, 5, &mut left)?;
            text(io, node, 6, &mut right)?;
            let priority = field(io, node, 7)?;
            if previous != Some(start) {
                if previous.is_some() {
                    flush_records(&mut records[..count], &mut scratch.lines, false)?;
                    count = 0;
                }
                pad_lines(&mut scratch.lines, lengths, span)?;
                if start > end {
                    return Err(Error::Engine(0x89e03eb1));
                }
                span = end - start;
                lengths = [0; 2];
                if previous.is_some() {
                    append(
                        scratch.lines[0],
                        &scratch.text[usize::from(start) - 1..usize::from(start)],
                    )?;
                    append(scratch.lines[1], b" ")?;
                    append(scratch.lines[2], b" ")?;
                }
                append(
                    scratch.lines[0],
                    &scratch.text[usize::from(start)..usize::from(end)],
                )?;
            }
            let left_seen = records[..count]
                .iter()
                .any(|r| ctext(&r.left) == ctext(&left));
            let right_seen = records[..count]
                .iter()
                .any(|r| ctext(&r.right) == ctext(&right));
            if !left_seen || !right_seen {
                if count == records.len() {
                    return Err(Error::Capacity {
                        required: count + 1,
                        available: records.len(),
                    });
                }
                for (column, seen, bytes) in [(0, left_seen, &left), (1, right_seen, &right)] {
                    let target = if column == 0 {
                        &mut records[count].left
                    } else {
                        &mut records[count].right
                    };
                    if seen {
                        target[0] = 0;
                    } else {
                        *target = *bytes;
                        if lengths[column] != 0 {
                            lengths[column] = lengths[column].wrapping_add(1);
                        }
                        lengths[column] = lengths[column].wrapping_add(ctext(bytes).len() as u16);
                    }
                }
                records[count].priority = priority;
                count += 1;
            }
            previous = Some(start);
        }
        node = scalar(io, StageOperation::Next, u32::from(node), 0)?.1;
    }
    if previous.is_some() {
        flush_records(&mut records[..count], &mut scratch.lines, true)?;
    }
    pad_lines(&mut scratch.lines, lengths, span)?;
    for line in 0..3 {
        set_rule(io, line as u32, scratch.lines[line])?;
    }
    invoke(
        io,
        StageOperation::ProcessRule,
        [0; 3],
        b"partofspeech\0",
        &mut [],
    )?;
    for (i, output) in scratch.rule_output.iter_mut().enumerate() {
        let reply = invoke(
            io,
            StageOperation::GetRule,
            [i as u32 + 1, 0, 0],
            &[],
            output,
        )?;
        if usize::from(reply.count) >= output.len() {
            return Err(Error::Capacity {
                required: usize::from(reply.count) + 1,
                available: output.len(),
            });
        }
        output[usize::from(reply.count)] = 0;
    }
    let (mut status, mut node) = scalar(io, StageOperation::First, 2, u32::from(root))?;
    previous = None;
    let mut matched = false;
    let mut pending = [0u16; 128];
    let mut count = 0usize;
    let mut cursors = [0; 2];
    let mut tokens = [(0, 0); 2];
    while node != 0 {
        let kind = field(io, node, 0)?;
        if matches!(kind, 4 | 9 | 10) {
            let start = field(io, node, 1)?;
            let mut left = [0; 256];
            text(io, node, 5, &mut left)?;
            if previous != Some(start) {
                for i in 0..2 {
                    tokens[i] = next_rule_token(scratch.rule_output[i], &mut cursors[i]);
                }
            }
            if previous.unwrap_or(0) != start {
                flush_pending(io, &pending[..count], matched)?;
                count = 0;
                matched = false;
            }
            let (begin, end) = tokens[0];
            let token_left = &scratch.rule_output[0][begin..end];
            let (begin, end) = tokens[1];
            let token_right = &scratch.rule_output[1][begin..end];
            let left = ctext(&left);
            let contains =
                token_left.is_empty() || left.windows(token_left.len()).any(|w| w == token_left);
            if contains || token_left == b"*" || left == b"*" {
                for (index, token) in [(5, token_left), (6, token_right)] {
                    let mut value = [0; 256];
                    if token.len() >= value.len() {
                        return Err(Error::Capacity {
                            required: token.len() + 1,
                            available: value.len(),
                        });
                    }
                    value[..token.len()].copy_from_slice(token);
                    set(
                        io,
                        node,
                        index,
                        (token.len() + 1) as u16,
                        &value[..token.len() + 1],
                    )?;
                }
                matched = true;
            } else {
                if count == pending.len() {
                    return Err(Error::Capacity {
                        required: count + 1,
                        available: pending.len(),
                    });
                }
                pending[count] = node;
                count = usize::from((count as u8).wrapping_add(1));
            }
            previous = Some(start);
        }
        (status, node) = scalar(io, StageOperation::Next, u32::from(node), 0)?;
    }
    if let Some(last) = flush_pending(io, &pending[..count], matched)? {
        status = last;
    }
    Ok(status)
}
