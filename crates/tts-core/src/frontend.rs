use core::cell::RefCell;
use core::cmp::Ordering;

#[derive(Clone, Copy)]
pub struct StreamKey<'a> {
    pub content_type: &'a [u8],
    pub version: u16,
}

/// fn_14dff4 / EU fn_14e0b8 / KR fn_14dfd8.
pub fn compare_stream_keys(left: Option<StreamKey<'_>>, right: Option<StreamKey<'_>>) -> i32 {
    match (left, right) {
        (None, None) => 0,
        (None, Some(_)) => -1,
        (Some(_), None) => 1,
        (Some(left), Some(right)) => {
            let length = |text: &[u8]| {
                text.iter()
                    .position(|&byte| byte == 0)
                    .expect("unterminated stream key")
            };
            let order = left.content_type[..length(left.content_type)]
                .cmp(&right.content_type[..length(right.content_type)]);
            match order.then(left.version.cmp(&right.version)) {
                Ordering::Less => -1,
                Ordering::Equal => 0,
                Ordering::Greater => 1,
            }
        }
    }
}

/// fn_14e084 / EU fn_14e148 / KR fn_14e068.
pub fn retain_stream(references: Option<&mut u16>, context: bool, output: bool) -> u32 {
    match references {
        Some(references) if context && output => {
            *references = references.wrapping_add(1);
            0
        }
        _ => 0x88f02007,
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct StreamPosition {
    pub kind: u8,
    pub read: u32,
    pub written: u32,
    pub eof: u8,
}

impl StreamPosition {
    /// fn_151294 / EU fn_151358 / KR fn_151278.
    pub fn position(&self, validation: u32, output: &mut u32) -> u32 {
        if validation >= 0x80000000 {
            return validation;
        }
        *output = if self.kind == 0 {
            self.read
        } else {
            self.written
        };
        validation
    }

    /// fn_15140c / EU fn_1514d0 / KR fn_1513f0.
    pub fn remaining(
        &self,
        validation: u32,
        output: Option<&mut u32>,
        finished: Option<&mut u8>,
    ) -> u32 {
        if validation >= 0x80000000 {
            return validation;
        }
        let Some(output) = output else {
            if let Some(finished) = finished {
                *finished = 1;
            }
            return 0x88f02007;
        };
        if self.kind != 0 {
            *output = 0;
            if let Some(finished) = finished {
                *finished = 1;
            }
            return 0x88f02007;
        }
        *output = self.written.wrapping_sub(self.read);
        if let Some(finished) = finished {
            *finished = self.eof;
        }
        validation
    }
}

/// fn_195974 / EU fn_195a24 / KR fn_195a60.
pub fn initialize_version(output: Option<&mut [u8; 3]>) -> u32 {
    match output {
        Some(output) => {
            *output = [5, 6, 1];
            0
        }
        None => 0x80000006,
    }
}

/// fn_1959a0 / EU fn_195a50 / KR fn_195a8c.
pub fn initialize_build(output: Option<&mut [u16; 3]>) -> u32 {
    match output {
        Some(output) => {
            *output = [0x7dc, 0x050c, 0];
            0
        }
        None => 0x80000006,
    }
}

/// fn_190ea4 / EU fn_190f54 / KR fn_190f90.
pub fn unsupported_interface(object_tag: u32, tag: u32, output: &mut u32) -> u32 {
    if tag != 0x68434c53 || object_tag != 0x73434c53 {
        return 0x80000008;
    }
    *output = 0;
    0x80000001
}

/// fn_151744 / EU fn_151808 / KR fn_151728.
pub fn stream_element_callbacks(callbacks: &[u8]) -> &[u8] {
    callbacks
}

/// fn_15f410 / EU fn_15f4d4 / KR fn_15f3f4.
pub fn finish_component_query() -> u32 {
    0
}

/// fn_18f488 / EU fn_18f538 / KR fn_18f574.
pub fn finish_request() -> u32 {
    0
}

/// fn_18fb74 / EU fn_18fc24 / KR fn_18fc60.
pub fn has_utf16_bom(text: &[u8; 2]) -> u32 {
    u32::from(matches!(text, [0xff, 0xfe] | [0xfe, 0xff]))
}

/// fn_1ce56c / EU fn_1ce728 / KR fn_1ce7d4.
pub fn streaming_mode(state: Option<(u8, u8)>) -> u32 {
    state.map_or(
        0,
        |(enabled, value)| if enabled != 0 { value as u32 } else { 1 },
    )
}

/// fn_159238 / EU fn_1592fc / KR fn_15921c.
pub fn set_parameter_without_marker<T>(
    first: T,
    second: u32,
    value: u32,
    mut set: impl FnMut(T, u32, u32, u32) -> u32,
) -> u32 {
    set(first, second, value, 0)
}

pub trait StateLock {
    fn acquire(&mut self) -> u32;
    fn release(&mut self) -> u32;
}

fn enter_state(
    state: &mut u8,
    next: u8,
    previous: &mut u8,
    lock: &mut impl StateLock,
    busy: u32,
) -> u32 {
    let mut status = lock.acquire();
    if status >= 0x80000000 {
        return status;
    }
    if *state == 1 {
        *previous = *state;
        *state = next;
    } else {
        status = busy;
    }
    let release = lock.release();
    if release >= 0x80000000 && status < 0x80000000 {
        status = release;
    }
    status
}

fn restore_state(state: &mut u8, previous: u32, lock: &mut impl StateLock) {
    if previous != 0 && lock.acquire() < 0x80000000 {
        *state = previous as u8;
        lock.release();
    }
}

/// fn_18f24c / EU fn_18f2fc / KR fn_18f338.
pub fn enter_request(
    state: &mut u8,
    next: u8,
    previous: &mut u8,
    lock: &mut impl StateLock,
) -> u32 {
    enter_state(state, next, previous, lock, 0x80002011)
}

/// fn_15ae7c / EU fn_15af40 / KR fn_15ae60.
pub fn enter_pipeline(
    state: &mut u8,
    next: u8,
    previous: &mut u8,
    lock: &mut impl StateLock,
) -> u32 {
    enter_state(state, next, previous, lock, 0x81202011)
}

/// fn_15af34 / EU fn_15aff8 / KR fn_15af18.
pub fn restore_pipeline(state: &mut u8, previous: u32, lock: &mut impl StateLock) {
    restore_state(state, previous, lock);
}

/// fn_1959dc / EU fn_195a8c / KR fn_195ac8.
pub fn restore_request(state: &mut u8, previous: u32, lock: &mut impl StateLock) {
    restore_state(state, previous, lock);
}

/// fn_18f3fc / EU fn_18f4ac / KR fn_18f4e8.
pub fn notify(arguments: [u32; 4], mut callback: impl FnMut([u32; 4]) -> u32) -> u32 {
    callback(arguments);
    0
}

/// fn_1517e8 / EU fn_1518ac / KR fn_1517cc.
pub fn resolve_engine<T: Copy>(
    object: T,
    validation: u32,
    mut context: impl FnMut() -> u32,
    output: &mut Option<T>,
) -> u32 {
    *output = None;
    if validation >= 0x80000000 {
        return if validation & 0x1fff == 8 {
            0x81202008
        } else {
            validation
        };
    }
    let status = context();
    if status < 0x80000000 {
        *output = Some(object);
    }
    status
}

/// fn_15ae20 / EU fn_15aee4 / KR fn_15ae04.
pub fn resolve_channel<T: Copy>(
    object: T,
    validation: u32,
    context: Option<impl FnMut() -> u32>,
    output: &mut Option<T>,
) -> u32 {
    *output = None;
    if validation >= 0x80000000 {
        return if validation & 0x1fff == 8 {
            0x81202008
        } else {
            validation
        };
    }
    *output = Some(object);
    match context {
        Some(mut context) => context(),
        None => validation,
    }
}

/// fn_158a50 / EU fn_158b14 / KR fn_158a34.
pub fn allocate_license(present: bool, mut resolve: impl FnMut() -> u32) -> u32 {
    if !present {
        return 0x81202007;
    }
    let status = resolve();
    if status >= 0x80000000 {
        status
    } else {
        0x81203d09
    }
}

/// fn_158b58 / EU fn_158c1c / KR fn_158b3c.
pub fn free_license(present: bool, mut resolve: impl FnMut() -> u32) -> u32 {
    if !present {
        return 0x81202007;
    }
    let status = resolve();
    if status >= 0x80000000 {
        status
    } else {
        0x81203d09
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PipelineCall {
    BeginStage(u16),
    Process(u16),
    EndStage(u16),
    ObserveStage(u16),
    QueryAuxiliary,
    NotifyAuxiliary(u16),
    FlushOutput,
    PrepareVoiceSwitch(u16),
    ExecuteVoiceSwitch(u16),
    ReadInput,
    CommitInput { bytes: u32, data: u32 },
    CloseInput,
    Lock,
    Unlock,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct PipelineReply {
    pub status: u32,
    pub value: u32,
    pub data: u32,
}

pub struct FrontendState<'a> {
    pub activity: &'a mut [u8],
    pub stage: u16,
    pub pending: u32,
    pub loop_limit: u32,
    pub request_state: u8,
    pub prepared_bytes: u32,
    pub input_open: bool,
    pub output_open: bool,
    pub instrumented: bool,
    pub observer: bool,
    pub auxiliary_notifications: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SynthesisPayload<'a> {
    pub event: u32,
    pub bytes: &'a [u8],
    pub markers: &'a [u8],
}

pub trait SynthesisSink {
    fn emit(&mut self, payload: SynthesisPayload<'_>) -> u32;
}

pub trait PipelineStages {
    fn call(
        &mut self,
        call: PipelineCall,
        state: &mut FrontendState<'_>,
        sink: &mut impl SynthesisSink,
    ) -> PipelineReply;
}

/// Speech scheduling in fn_15429c / EU fn_154360 / KR fn_154280.
/// Text analysis and voice-switch lifecycle operations belong to the stage adapter.
pub fn process_text(
    state: &mut FrontendState<'_>,
    flush: u32,
    stages: &mut impl PipelineStages,
    sink: &mut impl SynthesisSink,
) -> u32 {
    assert!(!state.activity.is_empty() && state.activity.len() <= u16::MAX as usize);
    assert!((state.stage as usize) < state.activity.len());
    let saved = state.stage;
    let mut result = 0;
    loop {
        if state.request_state == 6 {
            result = 0x81202807;
            break;
        }
        state.pending = state.pending.wrapping_add(1);
        if state.loop_limit != 0 && state.loop_limit < state.pending {
            result = 0x8120280d;
            break;
        }
        if flush != 1 || state.stage != saved {
            state.activity[state.stage as usize] = 0;
            if state.instrumented {
                stages.call(PipelineCall::BeginStage(state.stage), state, sink);
            }
            result = stages
                .call(PipelineCall::Process(state.stage), state, sink)
                .status;
            if state.instrumented {
                stages.call(PipelineCall::EndStage(state.stage), state, sink);
            }
            if state.observer {
                stages.call(PipelineCall::ObserveStage(state.stage), state, sink);
            }
            if state.auxiliary_notifications {
                let reply = stages.call(PipelineCall::QueryAuxiliary, state, sink);
                if reply.status < 0x80000000 && reply.value != 0 {
                    stages.call(PipelineCall::NotifyAuxiliary(state.stage), state, sink);
                }
            }
        }
        let mut ready = false;
        if result < 0x80000000 {
            let mut activity = state.activity[state.stage as usize];
            if activity == 3 {
                result = stages
                    .call(PipelineCall::PrepareVoiceSwitch(state.stage), state, sink)
                    .status;
                activity = state.activity[state.stage as usize];
            }
            if activity != 4 {
                if activity != 0 && (state.stage as usize + 1) < state.activity.len() {
                    state.stage += 1;
                } else {
                    let mut rewind = false;
                    if state.stage as usize + 1 == state.activity.len() {
                        if state.activity[state.stage as usize] == 0 {
                            rewind = true;
                        } else if state.output_open {
                            result = stages.call(PipelineCall::FlushOutput, state, sink).status;
                            if result >= 0x80000000 {
                                break;
                            }
                        }
                    }
                    loop {
                        let activity = state.activity[state.stage as usize];
                        if !rewind && (activity == 2 || activity == 3) {
                            break;
                        }
                        rewind = false;
                        if state.stage == 0 || (flush == 1 && state.stage <= saved) {
                            break;
                        }
                        state.stage -= 1;
                    }
                    if flush == 1 && state.stage == saved {
                        return result;
                    }
                    if state.activity[state.stage as usize] == 3 {
                        result = stages
                            .call(PipelineCall::ExecuteVoiceSwitch(state.stage), state, sink)
                            .status;
                    }
                }
                if state.stage == 0 && state.activity[0] != 2 {
                    if state.input_open {
                        state.pending = 0;
                        if state.instrumented {
                            stages.call(PipelineCall::BeginStage(0), state, sink);
                        }
                        let input = stages.call(PipelineCall::ReadInput, state, sink);
                        if state.instrumented {
                            stages.call(PipelineCall::EndStage(0), state, sink);
                        }
                        result = input.status;
                        if result & 0x1fff == 0 {
                            if input.value == 0 {
                                stages.call(PipelineCall::CloseInput, state, sink);
                                state.input_open = false;
                                result = 0x01202003;
                            } else {
                                result = stages
                                    .call(
                                        PipelineCall::CommitInput {
                                            bytes: input.value,
                                            data: input.data,
                                        },
                                        state,
                                        sink,
                                    )
                                    .status;
                                state.prepared_bytes =
                                    state.prepared_bytes.wrapping_add(input.value);
                            }
                        } else {
                            stages.call(PipelineCall::CloseInput, state, sink);
                            state.input_open = false;
                        }
                        if result & 0x1fff != 3 {
                            if flush == 2 {
                                return result;
                            }
                            if result >= 0x80000000 {
                                break;
                            }
                            continue;
                        }
                        result = 0;
                    }
                    ready = state.activity.iter().all(|&activity| activity == 1);
                }
            }
        }
        if flush == 2 {
            if ready {
                let lock = stages.call(PipelineCall::Lock, state, sink).status;
                let lock = if lock < 0x80000000 {
                    if state.request_state == 4 {
                        state.request_state = 5;
                    }
                    stages.call(PipelineCall::Unlock, state, sink).status
                } else {
                    lock
                };
                if result < 0x80000000 && lock >= 0x80000000 {
                    return lock;
                }
            }
            return result;
        }
        if ready || result >= 0x80000000 {
            break;
        }
    }
    if flush == 1 {
        state.stage = saved;
    }
    result
}

// --- fn_2104b0 render-path text rewriting -----------------------------------
//
// The rewriter borrows every engine buffer from the caller. Text and event
// records the engine lends are copied into caller-owned scratch, mutated
// there, and appended from there, so no mutable aliasing or unsafe code is
// needed. Only the appended bytes, position updates, release calls and the
// ordered interface-call sequence are observable, and those match the
// original exactly. The engine is assumed to lend disjoint storage per
// pointer, as the real engine lends distinct record fields. Null text with a
// count above 1, and missing event storage with a nonzero count, crash the
// original; this API rejects those engine states instead.

use crate::frontend_types::Error;

/// fn_176f94 / EU fn_177044 / KR fn_176ff0.
///
/// Validates a component handle: `tag` must equal `expected`, and either
/// `size` is zero or an object is present. Only nullness is observed; the
/// object is never dereferenced.
pub fn validate_handle(present: bool, tag: u32, expected: u32, size: u32) -> u32 {
    if tag == expected && (size == 0 || present) {
        0
    } else {
        0x81706008
    }
}

/// fn_176c58 / EU fn_176d08 / KR fn_176cb4.
///
/// Renders unsigned `value` in `base` into `buffer` with a terminator, using
/// the engine's lowercase digits. Bases below 2 hang the original in its
/// divide loop; this API rejects them instead of hanging.
pub fn render_unsigned(value: u32, buffer: &mut [u8], base: u32) -> Result<(), Error> {
    if value == 0 {
        if buffer.len() < 2 {
            return Err(Error::Capacity {
                required: 2,
                available: buffer.len(),
            });
        }
        buffer[0] = b'0';
        buffer[1] = 0;
        return Ok(());
    }
    if base < 2 {
        return Err(Error::InvalidResource);
    }
    let mut digits = [0u8; 32];
    let mut count = 0usize;
    let mut rest = value;
    loop {
        let [quotient, digit] =
            crate::output::unsigned_division(rest, base).map_err(|_| Error::InvalidResource)?;
        digits[count] = if digit < 10 {
            digit as u8 + b'0'
        } else {
            digit as u8 + b'W'
        };
        count += 1;
        rest = quotient;
        if rest == 0 {
            break;
        }
    }
    if buffer.len() < count + 1 {
        return Err(Error::Capacity {
            required: count + 1,
            available: buffer.len(),
        });
    }
    for (index, &digit) in digits[..count].iter().rev().enumerate() {
        buffer[index] = digit;
    }
    buffer[count] = 0;
    Ok(())
}

/// fn_176ac8 / EU fn_176b78 / KR fn_176b24.
///
/// Renders signed `value` in `base` into `buffer` with a terminator. A
/// negative value in base 10 gets a leading `-`; other bases divide the
/// negative value directly with truncation toward zero, keeping the engine's
/// signed digit mapping. Bases below 2 hang the original in its divide loop;
/// this API rejects them instead of hanging.
pub fn render_signed(value: i32, buffer: &mut [u8], base: u32) -> Result<(), Error> {
    if value == 0 {
        if buffer.len() < 2 {
            return Err(Error::Capacity {
                required: 2,
                available: buffer.len(),
            });
        }
        buffer[0] = b'0';
        buffer[1] = 0;
        return Ok(());
    }
    if base < 2 {
        return Err(Error::InvalidResource);
    }
    let mut digits = [0u8; 32];
    let mut count = 0usize;
    let mut rest = value;
    let mut start = 0usize;
    if rest < 0 && base == 10 {
        start = 1;
        rest = rest.wrapping_neg();
    }
    loop {
        let division = crate::runtime::signed_division(rest, base as i32)
            .map_err(|_| Error::InvalidResource)?;
        let digit = division
            .remainder
            .wrapping_add(if division.remainder < 10 { 48 } else { 87 });
        digits[count] = digit as u8;
        count += 1;
        rest = division.quotient;
        if rest == 0 {
            break;
        }
    }
    if buffer.len() < start + count + 1 {
        return Err(Error::Capacity {
            required: start + count + 1,
            available: buffer.len(),
        });
    }
    if start == 1 {
        buffer[0] = b'-';
    }
    for (index, &digit) in digits[..count].iter().rev().enumerate() {
        buffer[start + index] = digit;
    }
    buffer[start + count] = 0;
    Ok(())
}

/// One 24-byte text event record, as `fn_2104b0` reads and updates it.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct TextEvent {
    pub kind: u16,
    pub flags: u16,
    pub data: [u32; 2],
    pub position: i32,
    pub length: i32,
    pub value: i32,
}

impl TextEvent {
    fn serialize(&self, out: &mut [u8]) {
        out[0..2].copy_from_slice(&self.kind.to_le_bytes());
        out[2..4].copy_from_slice(&self.flags.to_le_bytes());
        out[4..8].copy_from_slice(&self.data[0].to_le_bytes());
        out[8..12].copy_from_slice(&self.data[1].to_le_bytes());
        out[12..16].copy_from_slice(&self.position.to_le_bytes());
        out[16..20].copy_from_slice(&self.length.to_le_bytes());
        out[20..24].copy_from_slice(&self.value.to_le_bytes());
    }
}

/// The two-word handle the engine passes to its interfaces, by value.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct RewriterHandle {
    pub first: u32,
    pub second: u32,
}

/// The text side of `fn_2104b0`'s engine: record traversal and field access.
///
/// The `bool` in the `get_text`/`get_events` result reports whether the
/// engine lent storage. Byte counts are opaque engine values, except that a
/// text count of 1 or less selects the `"##"` fallback without dereference.
pub trait RewriterText {
    fn first(&mut self, input: RewriterHandle, table: u32, root: u16, out: &mut u16) -> u32;
    fn next(&mut self, input: RewriterHandle, node: u16, out: &mut u16) -> u32;
    fn has_children(&mut self, input: RewriterHandle, table: u32, node: u16, out: &mut u16) -> u32;
    fn has(&mut self, input: RewriterHandle, node: u16, field: u32, out: &mut u16) -> u32;
    fn get_u8(
        &mut self,
        input: RewriterHandle,
        node: u16,
        field: u32,
        index: u16,
        out: &mut u8,
        count: &mut u16,
    ) -> u32;
    fn get_u16(
        &mut self,
        input: RewriterHandle,
        node: u16,
        field: u32,
        index: u16,
        out: &mut u16,
        count: &mut u16,
    ) -> u32;
    fn get_text(
        &mut self,
        input: RewriterHandle,
        node: u16,
        field: u32,
        scratch: &mut [u8],
        count: &mut u16,
    ) -> Result<(u32, bool), Error>;
    fn get_events(
        &mut self,
        input: RewriterHandle,
        node: u16,
        field: u32,
        scratch: &mut [TextEvent],
        count: &mut u16,
    ) -> Result<(u32, bool), Error>;
    fn set_text(
        &mut self,
        input: RewriterHandle,
        node: u16,
        field: u32,
        value: u16,
        text: &[u8],
        out: &mut u16,
    ) -> u32;
    fn release(&mut self, input: RewriterHandle) -> u32;
}

/// The buffer side of `fn_2104b0`'s engine: ordered byte appends and output
/// disposal.
pub trait RewriterBuffer {
    fn append(&mut self, output: RewriterHandle, data: &[u8]) -> u32;
    fn release(&mut self, output: RewriterHandle) -> u32;
}

/// The position side of `fn_2104b0`'s engine: two `i32` words selected by
/// `which`.
pub trait RewriterPositions {
    fn get(&mut self, handle: RewriterHandle, out: &mut i32, which: u32) -> u32;
    fn set(&mut self, handle: RewriterHandle, value: i32, which: u32) -> u32;
}

/// The whole `fn_2104b0` engine: text, buffer and position access together.
pub trait RewriterEngine: RewriterText + RewriterBuffer + RewriterPositions {}

impl<T: RewriterText + RewriterBuffer + RewriterPositions> RewriterEngine for T {}

/// Caller-owned `fn_2104b0` owner state: three output handles, the position
/// handle and the star-preservation flag.
pub struct TextRewriter {
    pub outputs: [RewriterHandle; 3],
    pub position_handle: RewriterHandle,
    pub preserve_stars: bool,
}

fn serialize_events(events: &[TextEvent], out: &mut [u8]) -> Result<usize, Error> {
    let need = events.len() * 24;
    if out.len() < need {
        return Err(Error::Capacity {
            required: need,
            available: out.len(),
        });
    }
    for (index, event) in events.iter().enumerate() {
        event.serialize(&mut out[index * 24..(index + 1) * 24]);
    }
    Ok(need)
}

/// fn_2104b0 / EU fn_21066c / KR fn_210718.
///
/// Rewrites one text record into the three output buffers. `event_scratch`
/// holds the engine's event records, `text_scratch` the engine's text, and
/// `event_bytes` the serialized events for the whole-record appends; the
/// caller sizes all three for its workload. `active` is set to 1 once the
/// handle validates, even when the first output is missing.
#[allow(clippy::too_many_arguments)]
pub fn rewrite_text(
    owner: &mut TextRewriter,
    arg: u32,
    input: RewriterHandle,
    active: &mut u8,
    engine: &mut impl RewriterEngine,
    event_scratch: &mut [TextEvent],
    event_bytes: &mut [u8],
    text_scratch: &mut [u8],
) -> Result<u32, Error> {
    if validate_handle(true, arg, 0xf381, 0x34) != 0 {
        return Ok(0x89a02008);
    }
    *active = 1;
    if owner.outputs[0].first == 0 {
        return Ok(0);
    }
    let ok = |status: u32| status < 0x80000000;
    let mut number = [0u8; 32];
    let mut root = 0u16;
    let mut result = engine.first(input, 1, 0, &mut root);
    if !ok(result) {
        return finish(owner, input, engine, result, false);
    }
    let mut dispose = 0u8;
    let mut count = 0u16;
    result = engine.get_u8(input, root, 2, 1, &mut dispose, &mut count);
    if !ok(result) {
        return finish(owner, input, engine, result, dispose != 0);
    }
    let mut position = 0i32;
    result = engine.get(owner.position_handle, &mut position, 1);
    if !ok(result) {
        return finish(owner, input, engine, result, dispose != 0);
    }
    let mut pending = 0i32;
    result = engine.get(owner.position_handle, &mut pending, 0);
    if !ok(result) {
        return finish(owner, input, engine, result, dispose != 0);
    }
    let mut present = 0u16;
    let rc = engine.has(input, root, 0, &mut present);
    if !ok(rc) || present != 1 {
        result = rewrite_fallback(
            owner,
            input,
            root,
            engine,
            &mut number,
            event_scratch,
            event_bytes,
            &mut result,
        )?;
        return finish(owner, input, engine, result, dispose != 0);
    }
    let (status, present_text) = engine.get_text(input, root, 0, text_scratch, &mut count)?;
    result = status;
    if !ok(result) {
        return finish(owner, input, engine, result, dispose != 0);
    }
    if count <= 1 {
        result = engine.append(owner.outputs[0], b"##\0");
        return finish(owner, input, engine, result, dispose != 0);
    }
    if !present_text {
        return Err(Error::InvalidResource);
    }
    let original_length = crate::resources::string_length(text_scratch) as u32;
    result = engine.append(
        owner.outputs[0],
        &text_scratch[..original_length as usize + 1],
    );
    if !ok(result) {
        return finish(owner, input, engine, result, dispose != 0);
    }
    result = rewrite_long(
        owner,
        input,
        engine,
        &mut number,
        event_scratch,
        event_bytes,
        text_scratch,
        original_length,
        root,
        &mut position,
        &mut pending,
        &mut result,
    )?;
    finish(owner, input, engine, result, dispose != 0)
}

fn finish(
    owner: &mut TextRewriter,
    input: RewriterHandle,
    engine: &mut impl RewriterEngine,
    result: u32,
    dispose: bool,
) -> Result<u32, Error> {
    let mut result = result;
    if dispose {
        for output in owner.outputs.iter_mut() {
            if output.first != 0 {
                result = RewriterBuffer::release(engine, *output);
                output.first = 0;
                output.second = 0;
            }
        }
    }
    let rc = RewriterText::release(engine, input);
    if rc >= 0x80000000 && result < 0x80000000 {
        result = rc;
    }
    Ok(result)
}

#[allow(clippy::too_many_arguments)]
fn rewrite_fallback(
    owner: &mut TextRewriter,
    input: RewriterHandle,
    root: u16,
    engine: &mut impl RewriterEngine,
    number: &mut [u8; 32],
    event_scratch: &mut [TextEvent],
    event_bytes: &mut [u8],
    result: &mut u32,
) -> Result<u32, Error> {
    let ok = |status: u32| status < 0x80000000;
    let mut present = 0u16;
    let mut count = 0u16;
    let mut fallback_count = 0u16;
    let mut leading_pause = 0i32;
    let mut trailing_pause = 0i32;
    let rc = engine.has(input, root, 1, &mut present);
    if ok(rc) && present == 1 {
        let (status, _) = engine.get_events(input, root, 1, event_scratch, &mut count)?;
        *result = status;
        if !ok(*result) {
            return Ok(*result);
        }
        fallback_count = count;
        if fallback_count != 0 {
            let need = serialize_events(&event_scratch[..fallback_count as usize], event_bytes)?;
            *result = engine.append(owner.outputs[1], &event_bytes[..need]);
            if !ok(*result) {
                return Ok(*result);
            }
        }
    }
    *result = engine.append(owner.outputs[0], b"##\0");
    if fallback_count != 0 {
        let initial_position = event_scratch[0].position;
        let mut index = 0u16;
        loop {
            let event = event_scratch[index as usize];
            if event.kind == 8 {
                if event.position == initial_position {
                    leading_pause = leading_pause.wrapping_add(event.value);
                } else {
                    trailing_pause = trailing_pause.wrapping_add(event.value);
                }
            }
            index = index.wrapping_add(1);
            if index >= fallback_count {
                break;
            }
        }
    }
    if leading_pause != 0 || trailing_pause != 0 {
        let mut length: u32;
        if leading_pause != 0 {
            number[0] = b'#';
            number[1] = b'[';
            render_unsigned(leading_pause as u32, &mut number[2..], 10)?;
            let digits = crate::resources::string_length(&number[2..]) as u32;
            number[2 + digits as usize] = b']';
            number[2 + digits as usize + 1] = b'#';
            length = digits + 4;
        } else {
            number[0] = b'#';
            number[1] = b'#';
            length = 2;
        }
        *result = engine.append(owner.outputs[2], &number[..length as usize]);
        if !ok(*result) {
            return Ok(*result);
        }
        if trailing_pause != 0 {
            number[0] = b'[';
            render_unsigned(trailing_pause as u32, &mut number[1..], 10)?;
            let digits = crate::resources::string_length(&number[1..]) as u32;
            number[digits as usize + 1] = b']';
            number[digits as usize + 2] = 0;
            length = digits + 3;
        } else {
            number[0] = 0;
            length = 1;
        }
        *result = engine.append(owner.outputs[2], &number[..length as usize]);
    }
    Ok(*result)
}

#[allow(clippy::too_many_arguments)]
fn rewrite_long(
    owner: &mut TextRewriter,
    input: RewriterHandle,
    engine: &mut impl RewriterEngine,
    number: &mut [u8; 32],
    event_scratch: &mut [TextEvent],
    event_bytes: &mut [u8],
    text_scratch: &mut [u8],
    original_length: u32,
    root: u16,
    position: &mut i32,
    pending: &mut i32,
    result: &mut u32,
) -> Result<u32, Error> {
    let ok = |status: u32| status < 0x80000000;
    let mut count = 0u16;
    let (status, present_events) = engine.get_events(input, root, 1, event_scratch, &mut count)?;
    *result = status;
    if !ok(*result) {
        return Ok(*result);
    }
    if !present_events {
        return Err(Error::InvalidResource);
    }
    let event_count = count;
    let mut event_index = 0u16;
    let mut last_pause = 0u32;
    let mut end = 0u16;
    let base_position = if event_count != 0 {
        event_scratch[0].position
    } else {
        0
    };
    let mut duration = 0u16;
    while event_index < event_count && event_scratch[event_index as usize].position == base_position
    {
        if event_scratch[event_index as usize].kind == 8 {
            duration = duration.wrapping_add(event_scratch[event_index as usize].value as u16);
        }
        event_index = event_index.wrapping_add(1);
    }
    engine.append(owner.outputs[2], b"#");
    let mut offset = 1i16;
    if duration != 0 {
        *result = engine.append(owner.outputs[2], b"[");
        if !ok(*result) {
            return Ok(*result);
        }
        render_signed(i32::from(duration), &mut number[..], 10)?;
        let digits = crate::resources::string_length(&number[..]) as u32;
        *result = engine.append(owner.outputs[2], &number[..digits as usize]);
        if !ok(*result) {
            return Ok(*result);
        }
        *result = engine.append(owner.outputs[2], b"]");
        if !ok(*result) {
            return Ok(*result);
        }
        offset = offset.wrapping_add((digits + 2) as i16);
    }
    let mut present = 0u16;
    let rc = engine.has_children(input, 2, root, &mut present);
    let mut mode13 = false;
    let mut mode14 = false;
    if ok(rc) && present != 0 {
        let mut node = 0u16;
        *result = engine.first(input, 2, root, &mut node);
        if !ok(*result) {
            return Ok(*result);
        }
        if node != 0 {
            let mut previous_end = 0u32;
            let mut first_text = true;
            let mut pause = 0u16;
            loop {
                let mut kind = 0u8;
                *result = engine.get_u8(input, node, 0, 1, &mut kind, &mut count);
                if !ok(*result) {
                    return Ok(*result);
                }
                let body = matches!(kind, 4 | 9 | 10 | 11 | 12 | 13 | 14);
                if kind == 11 {
                    if mode14 {
                        *result = engine.append(owner.outputs[2], &[0x13, 0x10]);
                        if !ok(*result) {
                            return Ok(*result);
                        }
                        offset = offset.wrapping_add(2);
                        mode14 = false;
                    }
                } else if body {
                    if mode13 {
                        *result = engine.append(owner.outputs[2], &[0x12, 0x10]);
                        if !ok(*result) {
                            return Ok(*result);
                        }
                        offset = offset.wrapping_add(2);
                        mode13 = false;
                    }
                    if kind != 12 && mode14 {
                        *result = engine.append(owner.outputs[2], &[0x13, 0x10]);
                        if !ok(*result) {
                            return Ok(*result);
                        }
                        offset = offset.wrapping_add(2);
                        mode14 = false;
                    }
                }
                if body {
                    let mut start = 0u16;
                    *result = engine.get_u16(input, node, 1, 1, &mut start, &mut count);
                    if !ok(*result) {
                        return Ok(*result);
                    }
                    *result = engine.get_u16(input, node, 2, 1, &mut end, &mut count);
                    if !ok(*result) {
                        return Ok(*result);
                    }
                    duration = 0;
                    let mut pause_offset = 0i32;
                    offset =
                        previous_end.wrapping_add((offset as i32 - start as i32) as u32) as i16;
                    let (text_status, present_text) =
                        engine.get_text(input, node, 3, text_scratch, &mut count)?;
                    *result = text_status;
                    if !ok(*result) {
                        return Ok(*result);
                    }
                    if present_text && text_scratch[0] != 0 {
                        let text_length = crate::resources::string_length(text_scratch) as u32;
                        if !owner.preserve_stars {
                            let mut changed = false;
                            let mut remaining = text_length as u16;
                            if remaining != 0 {
                                let mut at = 0usize;
                                let mut index = 0i32;
                                loop {
                                    if text_scratch[at] == b'*'
                                        && index != 0
                                        && text_scratch[at - 1] != b'-'
                                    {
                                        changed = true;
                                        text_scratch[at] = b'-';
                                    }
                                    at += 1;
                                    index = index.wrapping_add(1);
                                    remaining = remaining.wrapping_sub(1);
                                    if remaining == 0 {
                                        break;
                                    }
                                }
                            }
                            if changed {
                                let mut changed_count = 0u16;
                                *result = engine.set_text(
                                    input,
                                    node,
                                    3,
                                    text_length.wrapping_add(1) as u16,
                                    &text_scratch[..text_length as usize + 1],
                                    &mut changed_count,
                                );
                                if !ok(*result) {
                                    return Ok(*result);
                                }
                            }
                        }
                        if !first_text {
                            if pause != 0 {
                                *result = engine.append(owner.outputs[2], b"%#[");
                                if !ok(*result) {
                                    return Ok(*result);
                                }
                                render_signed(i32::from(pause), &mut number[..], 10)?;
                                let digits = crate::resources::string_length(&number[..]);
                                *result = engine.append(owner.outputs[2], &number[..digits]);
                                if !ok(*result) {
                                    return Ok(*result);
                                }
                                *result = engine.append(owner.outputs[2], b"]");
                                if !ok(*result) {
                                    return Ok(*result);
                                }
                                pause_offset = offset as i32;
                                offset = offset.wrapping_add((digits as u32 + 4) as i16);
                            } else {
                                *result = engine.append(owner.outputs[2], b"-*");
                                if !ok(*result) {
                                    return Ok(*result);
                                }
                                offset = offset.wrapping_add(2);
                            }
                        }
                        if kind == 13 {
                            *result = engine.append(owner.outputs[2], &[0x10, 0x12]);
                            if !ok(*result) {
                                return Ok(*result);
                            }
                            offset = offset.wrapping_add(2);
                            mode13 = true;
                        }
                        if kind == 14 {
                            *result = engine.append(owner.outputs[2], &[0x10, 0x13]);
                            if !ok(*result) {
                                return Ok(*result);
                            }
                            offset = offset.wrapping_add(2);
                            mode14 = true;
                        }
                        let mut present8 = 0u16;
                        let rc = engine.has(input, node, 8, &mut present8);
                        if ok(rc) && present8 == 1 {
                            *result = engine.get_u16(input, node, 8, 1, &mut pause, &mut count);
                            if !ok(*result) {
                                return Ok(*result);
                            }
                        } else {
                            pause = 0;
                        }
                        first_text = false;
                    }
                    while event_index < event_count
                        && ((event_scratch[event_index as usize]
                            .position
                            .wrapping_sub(base_position)) as u16)
                            <= start
                    {
                        let fresh = base_position
                            .wrapping_add(start as i16 as i32)
                            .wrapping_add(offset as i32);
                        event_scratch[event_index as usize].position = fresh;
                        if event_scratch[event_index as usize].kind == 8 {
                            duration = duration
                                .wrapping_add(event_scratch[event_index as usize].value as u16);
                            last_pause = (fresh
                                .wrapping_sub(base_position)
                                .wrapping_sub(offset as i32)
                                as u16) as u32;
                            let shifted = base_position
                                .wrapping_add(start as i16 as i32)
                                .wrapping_add(pause_offset);
                            let mut junior = event_index;
                            while junior != 0 {
                                if event_scratch[(junior - 1) as usize].position == fresh {
                                    event_scratch[(junior - 1) as usize].position = shifted;
                                }
                                junior -= 1;
                            }
                            event_scratch[event_index as usize].position = shifted;
                        }
                        event_index = event_index.wrapping_add(1);
                    }
                    previous_end = end as u32;
                    offset = offset.wrapping_sub(previous_end.wrapping_sub(start as u32) as i16);
                    if present_text && text_scratch[0] != 0 {
                        let text_length = crate::resources::string_length(text_scratch) as u32;
                        *result =
                            engine.append(owner.outputs[2], &text_scratch[..text_length as usize]);
                        if !ok(*result) {
                            return Ok(*result);
                        }
                        offset = offset.wrapping_add(text_length as i16);
                    }
                }
                *result = engine.next(input, node, &mut node);
                if !ok(*result) {
                    return Ok(*result);
                }
                if node == 0 {
                    break;
                }
            }
            if mode13 {
                *result = engine.append(owner.outputs[2], &[0x12, 0x10]);
                if !ok(*result) {
                    return Ok(*result);
                }
                offset = offset.wrapping_add(2);
            }
        }
    }
    if mode14 {
        *result = engine.append(owner.outputs[2], &[0x13, 0x10]);
        if !ok(*result) {
            return Ok(*result);
        }
        offset = offset.wrapping_add(2);
    }
    if last_pause == 0 || end.wrapping_sub(1) as u32 != last_pause {
        duration = 0;
    }
    if event_index < event_count {
        loop {
            event_scratch[event_index as usize].position = base_position
                .wrapping_add(end as i16 as i32)
                .wrapping_add(offset as i32)
                .wrapping_add(1);
            if event_scratch[event_index as usize].kind == 8 {
                duration = duration.wrapping_add(event_scratch[event_index as usize].value as u16);
            }
            event_index = event_index.wrapping_add(1);
            if event_index >= event_count {
                break;
            }
        }
    }
    offset = offset.wrapping_sub(original_length.wrapping_sub(end as u32) as i16);
    engine.append(owner.outputs[2], b"#");
    offset = offset.wrapping_add(1);
    if duration != 0 {
        *result = engine.append(owner.outputs[2], b"[");
        if !ok(*result) {
            return Ok(*result);
        }
        render_signed(i32::from(duration), &mut number[..], 10)?;
        let digits = crate::resources::string_length(&number[..]) as u32;
        *result = engine.append(owner.outputs[2], &number[..digits as usize]);
        if !ok(*result) {
            return Ok(*result);
        }
        *result = engine.append(owner.outputs[2], b"]");
        if !ok(*result) {
            return Ok(*result);
        }
        offset = offset.wrapping_add((digits + 2) as i16);
    }
    engine.append(owner.outputs[2], &[0]);
    if event_count != 0 {
        event_scratch[0].length = event_scratch[0].length.wrapping_add(offset as i32);
    }
    *position = position.wrapping_add(*pending).wrapping_add(offset as i32);
    *pending = 0;
    *result = engine.set(owner.position_handle, *position, 1);
    if !ok(*result) {
        return Ok(*result);
    }
    *result = engine.set(owner.position_handle, *pending, 0);
    if !ok(*result) {
        return Ok(*result);
    }
    let need = serialize_events(&event_scratch[..event_count as usize], event_bytes)?;
    *result = engine.append(owner.outputs[1], &event_bytes[..need]);
    Ok(*result)
}

/// fn_176fd4 / EU fn_177084 / KR fn_177030.
///
/// Copies the caller's 8-byte pair to the output. The original reads a
/// global pair; the caller supplies its bytes.
pub fn load_pair(out: &mut [u8; 8], value: &[u8; 8]) {
    out.copy_from_slice(value);
}

/// fn_1757a8 / EU fn_175858 / KR fn_175804.
///
/// Null-checks a handle without dereferencing it.
pub fn check_handle(present: bool) -> u32 {
    if present {
        0
    } else {
        0x8b702007
    }
}

/// fn_17eb74 / EU fn_17ec24 / KR fn_17ebd0.
///
/// Reads the caller's license counter without changing it.
pub fn license_counter(counter: u32) -> u32 {
    counter
}

/// fn_17eb3c / EU fn_17ebec / KR fn_17eb98.
///
/// Increments the caller's license counter and scales the feature.
pub fn acquire_license(counter: &mut u32, feature: u32) -> u32 {
    *counter = counter.wrapping_add(1);
    feature.wrapping_mul(100000)
}

/// fn_17eb58 / EU fn_17ec08 / KR fn_17ebb4.
///
/// Decrements a nonzero caller's license counter.
pub fn release_license(counter: &mut u32) -> u32 {
    if *counter != 0 {
        *counter -= 1;
    }
    0
}

/// fn_17c7b8 / EU fn_17c868 / KR fn_17c814.
///
/// Low two bits of the caller's 256-entry class table at the low byte.
pub fn char_class(table: &[u16; 256], c: u32) -> u32 {
    u32::from(table[(c & 0xff) as usize] & 3)
}

/// fn_17c8c0 / EU fn_17c970 / KR fn_17c91c.
///
/// Folds to lowercase when the class table's bit 0 is set for the low byte.
pub fn ascii_fold(table: &[u16; 256], c: u32) -> u32 {
    if table[(c & 0xff) as usize] & 1 != 0 {
        c.wrapping_add(0x20)
    } else {
        c
    }
}

/// fn_1769d0 / EU fn_176a80 / KR fn_176a2c.
///
/// Case-insensitive byte comparison through the class table. Inputs must be
/// NUL-terminated within the borrowed slices; the original reads past the
/// terminator only on difference.
pub fn compare_folded(table: &[u16; 256], a: &[u8], b: &[u8]) -> i32 {
    let mut index = 0usize;
    if ascii_fold(table, a[0] as u32) == ascii_fold(table, b[0] as u32) {
        loop {
            if a[index] == 0 {
                return 0;
            }
            index += 1;
            if ascii_fold(table, a[index] as u32) != ascii_fold(table, b[index] as u32) {
                break;
            }
        }
    }
    ascii_fold(table, a[index] as u32) as i32 - ascii_fold(table, b[index] as u32) as i32
}

/// fn_176a48 / EU fn_176af8 / KR fn_176aa4.
///
/// Bounded case-insensitive byte comparison through the class table. The
/// limit uses the original unsigned comparison after the usual conversions,
/// so a negative limit behaves as unbounded; inputs must still be
/// NUL-terminated within the borrowed slices.
pub fn compare_folded_bounded(table: &[u16; 256], a: &[u8], b: &[u8], n: i32) -> i32 {
    let mut index = 0u32;
    while (n as u32) > index {
        if ascii_fold(table, a[index as usize] as u32)
            != ascii_fold(table, b[index as usize] as u32)
        {
            break;
        }
        if a[index as usize] == 0 {
            break;
        }
        if b[index as usize] == 0 {
            break;
        }
        index += 1;
    }
    if index == n as u32 {
        return 0;
    }
    ascii_fold(table, a[index as usize] as u32) as i32
        - ascii_fold(table, b[index as usize] as u32) as i32
}

/// Caller-owned array for `fn_196fbc` element removal.
pub struct ElementArray<'a> {
    pub data: &'a mut [u8],
    pub elem_size: u16,
    pub count: u32,
}

/// fn_196fbc / EU fn_19706c / KR fn_1970a8.
///
/// Removes the element at `elem`, shifting the tail down with overlap-safe
/// copy semantics and decrementing the count. `elem` is an element index;
/// `None` is the null pointer and reports 0. `dtor`, when present, runs
/// first on the element bytes, matching the engine's destructor ordering.
/// An out-of-range index, including the one-past-end slot the original
/// accepts before overrunning its copy length, reports 0 instead of
/// accessing outside the borrowed slice.
pub fn remove_element(
    array: &mut ElementArray<'_>,
    elem: Option<usize>,
    mut dtor: Option<impl FnMut(&mut [u8])>,
) -> u32 {
    let index = match elem {
        Some(index) => index,
        None => return 0,
    };
    let size = array.elem_size as usize;
    let count = array.count as usize;
    if index >= count {
        return 0;
    }
    let end = match count.checked_mul(size) {
        Some(end) if end <= array.data.len() => end,
        _ => return 0,
    };
    let at = match index.checked_mul(size) {
        Some(at) if at + size <= end => at,
        _ => return 0,
    };
    if let Some(dtor) = dtor.as_mut() {
        dtor(&mut array.data[at..at + size]);
    }
    array.data.copy_within(at + size..end, at);
    array.count = array.count.wrapping_sub(1);
    1
}

/// fn_175ef0 / EU fn_175fa0 / KR fn_175f4c.
///
/// Lock-guarded element removal. A missing owner or element reports
/// `0x8b702007` without touching the lock; otherwise the lock is acquired,
/// the element removed (a miss reports `0x8b702007`), and the lock
/// released, with a failing release overriding a successful removal.
pub fn remove_guarded(
    array: Option<&mut ElementArray<'_>>,
    elem: Option<usize>,
    lock: &mut impl StateLock,
    dtor: Option<impl FnMut(&mut [u8])>,
) -> u32 {
    let (Some(array), Some(index)) = (array, elem) else {
        return 0x8b702007;
    };
    let mut status = lock.acquire();
    if status < 0x80000000 {
        if remove_element(array, Some(index), dtor) == 0 {
            status = 0x8b702007;
        }
        let release = lock.release();
        if release >= 0x80000000 && status < 0x80000000 {
            status = release;
        }
    }
    status
}

/// fn_1970a0 / EU fn_197150 / KR fn_19718c.
///
/// Reads word 5 of the borrowed table; a missing table reports 0.
pub fn word_at5(input: Option<&[u32; 6]>) -> u32 {
    match input {
        Some(words) => words[5],
        None => 0,
    }
}

/// Caller-owned element table for `fn_1970ac` slot lookup.
pub struct ElementTable<'a> {
    pub data: &'a [u8],
    pub elem_size: u16,
    pub count: u32,
}

/// fn_1970ac / EU fn_19715c / KR fn_197198.
///
/// Writes the byte offset of slot `index` from the start of the table data.
/// A missing table/output or an out-of-range index reports 0 without
/// writing; the index/size product wraps mod 2^32 like the original.
pub fn element_at(table: Option<&ElementTable<'_>>, index: u32, out: Option<&mut u32>) -> u32 {
    let (Some(table), Some(out)) = (table, out) else {
        return 0;
    };
    if index >= table.count {
        return 0;
    }
    *out = index.wrapping_mul(table.elem_size as u32);
    1
}

/// fn_195ae8 / EU fn_195b98 / KR fn_195bd4.
///
/// Copies `n` bytes forward from `src` to `dst`. Both sides must lend at
/// least `n` bytes. The regions must not overlap: the original always
/// copies forward, which safe borrowing cannot express for aliased slices.
pub fn copy_bytes(dst: &mut [u8], src: &[u8], n: u32) {
    let n = n as usize;
    for i in 0..n {
        dst[i] = src[i];
    }
}

/// Caller-owned growable block shared by `fn_196cfc` and `fn_196db0`.
///
/// `data` is an offset into caller memory (`0` is null). Words +12 and the
/// high half of +24 are preserved bytes neither function reads. `callback`
/// is the `fn_196db0` view of word +8 (presence only); `heap` is word +28,
/// which `fn_196cfc` uses as the heap id and `fn_196db0` passes on as the
/// callback cookie.
pub struct GrowBlock {
    pub data: u32,
    pub elem_size: u16,
    pub callback: u32,
    pub cap: u32,
    pub count: u32,
    pub grow: u16,
    pub heap: u32,
}

/// Scripted heap behind `fn_196cfc`/`fn_196a64`: offsets into caller memory,
/// `0` fails.
pub trait BlockHeap {
    fn alloc_block(&mut self, heap: u32, size: u32) -> u32;
    fn realloc_block(&mut self, heap: u32, data: u32, size: u32) -> u32;
    fn alloc_cells(&mut self, heap: u32, count: u32, size: u32) -> u32;
}

/// fn_196cfc / EU fn_196dac / KR fn_196de8.
///
/// Appends a slot, growing the block to the next quantum multiple first.
/// Writes the new slot's offset and reports 1; a missing block/output, a
/// failed heap call, or a zero growth quantum with growth needed reports 0.
/// A zero quantum divides by zero on ARM (degenerate) and traps in C, so
/// the port totalizes it to failure without touching the block.
pub fn append_slot(
    block: Option<&mut GrowBlock>,
    heap: &mut impl BlockHeap,
    out: Option<&mut u32>,
) -> u32 {
    let (Some(block), Some(out)) = (block, out) else {
        return 0;
    };
    if block.cap < block.count.wrapping_add(1) {
        let quantum = block.grow as u32;
        if quantum == 0 {
            return 0;
        }
        let slots = (block.count / quantum)
            .wrapping_add(1)
            .wrapping_mul(quantum);
        let bytes = (block.elem_size as u32).wrapping_mul(slots);
        if block.data != 0 {
            let grown = heap.realloc_block(block.heap, block.data, bytes);
            if grown != 0 {
                block.data = grown;
            } else {
                return 0;
            }
        } else {
            let fresh = heap.alloc_block(block.heap, bytes);
            block.data = fresh;
            if fresh == 0 {
                return 0;
            }
        }
        block.cap = slots;
    }
    *out = block
        .count
        .wrapping_mul(block.elem_size as u32)
        .wrapping_add(block.data);
    block.count = block.count.wrapping_add(1);
    1
}

/// fn_196a64 / EU fn_196b14 / KR fn_196b50.
///
/// Allocates a 32-byte block and initializes it: zeros at words +0/+16/+20,
/// `b`/`c`/`d` at words +4/+8/+12, the low half of `e` at +24 and the heap
/// id `a` at +28. The half at +26 is preserved. Reports 1 with the block
/// offset; mismatched `c`/`d` presence or a missing output fails before
/// allocating, and heap failure writes a null output.
pub fn create_block(
    heap: &mut impl BlockHeap,
    a: u32,
    b: u32,
    c: u32,
    d: u32,
    e: u32,
    mem: &mut [u8],
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
    let at = heap.alloc_cells(a, 1, 32);
    *slot = at;
    if at == 0 {
        return 0;
    }
    let at = at as usize;
    mem[at..at + 4].copy_from_slice(&0u32.to_le_bytes());
    mem[at + 4..at + 8].copy_from_slice(&b.to_le_bytes());
    mem[at + 8..at + 12].copy_from_slice(&c.to_le_bytes());
    mem[at + 12..at + 16].copy_from_slice(&d.to_le_bytes());
    mem[at + 16..at + 20].copy_from_slice(&0u32.to_le_bytes());
    mem[at + 20..at + 24].copy_from_slice(&0u32.to_le_bytes());
    mem[at + 24..at + 26].copy_from_slice(&(e as u16).to_le_bytes());
    mem[at + 28..at + 32].copy_from_slice(&a.to_le_bytes());
    1
}

/// fn_196db0 / EU fn_196e60 / KR fn_196e9c.
///
/// Appends `src` as a new element: takes a slot, copies the element bytes
/// into caller memory at the slot offset, and runs the callback with the
/// source id, heap cookie and slot offset. A callback veto decrements the
/// count and reports 0. Callback presence comes from `cb`, not from the
/// block's address-valued word; callers pass `None` when it is null.
pub fn append_element(
    block: Option<&mut GrowBlock>,
    mem: &mut [u8],
    src_id: u32,
    src: &[u8],
    heap: &mut impl BlockHeap,
    cb: Option<impl FnMut(u32, u32, u32) -> u32>,
) -> u32 {
    let Some(block) = block else {
        return 0;
    };
    let mut slot = 0u32;
    if append_slot(Some(block), heap, Some(&mut slot)) == 0 {
        return 0;
    }
    let size = block.elem_size as usize;
    let at = slot as usize;
    copy_bytes(&mut mem[at..at + size], &src[..size], size as u32);
    if let Some(mut cb) = cb {
        if cb(src_id, block.heap, slot) == 0 {
            block.count = block.count.wrapping_sub(1);
            return 0;
        }
    }
    1
}

/// fn_175c40 / EU fn_175cf0 / KR fn_175c9c.
///
/// Lock-guarded owner append: appends `{owner_id, 0}` as a new element and
/// reports the new last slot's offset through `q`. Missing owner/output
/// reports `0x8b702007` without locking; lock failure returns without
/// unlocking; append/lookup failures report `0x8b70200a`/`0x8b702100`, with
/// a failing unlock overriding a success. The callback observes the owner
/// id as the source id: the transient `k` has no stable offset-space
/// address.
pub fn append_owner(
    block: Option<&mut GrowBlock>,
    mem: &mut [u8],
    owner_id: u32,
    q: Option<&mut u32>,
    lock: &mut impl StateLock,
    heap: &mut impl BlockHeap,
    cb: Option<impl FnMut(u32, u32, u32) -> u32>,
) -> u32 {
    let (Some(block), Some(q)) = (block, q) else {
        return 0x8b702007;
    };
    let mut status = lock.acquire();
    if status >= 0x80000000 {
        return status;
    }
    let mut pair = [0u8; 8];
    pair[..4].copy_from_slice(&owner_id.to_le_bytes());
    if append_element(Some(&mut *block), mem, owner_id, &pair, heap, cb) == 0 {
        status = 0x8b70200a;
    } else {
        let table = ElementTable {
            data: &mem[block.data as usize..],
            elem_size: block.elem_size,
            count: block.count,
        };
        if element_at(Some(&table), block.count.wrapping_sub(1), Some(q)) == 0 {
            status = 0x8b702100;
        }
    }
    let release = lock.release();
    if release >= 0x80000000 && status < 0x80000000 {
        status = release;
    }
    status
}

/// fn_198760 / EU fn_198810 / KR fn_19884c.
///
/// Searches the node tree for `key` with the caller's comparator. Nodes
/// are 24-byte records in `mem` (`next` at +0, `prev` at +4, key at +16);
/// links and `head` are offsets with `tree` as the sentinel. With `flag`
/// set, walks to the last consecutive match. Reports the found offset, or
/// `None` on a miss. The comparator must order like wrapping subtraction.
pub fn find_node(
    mem: &[u8],
    tree: u32,
    head: u32,
    flag: u8,
    key: u32,
    mut cmp: impl FnMut(u32, u32) -> i32,
) -> Option<u32> {
    fn word(mem: &[u8], at: u32) -> u32 {
        u32::from_le_bytes(mem[at as usize..at as usize + 4].try_into().unwrap())
    }
    let mut node = head;
    loop {
        if node == tree {
            return None;
        }
        let order = cmp(key, word(mem, node.wrapping_add(16)));
        if order < 0 {
            node = word(mem, node);
        } else if order > 0 {
            node = word(mem, node.wrapping_add(4));
        } else {
            break;
        }
    }
    if flag == 0 {
        return Some(node);
    }
    loop {
        let last = node;
        node = word(mem, last);
        while node != tree && cmp(key, word(mem, node.wrapping_add(16))) != 0 {
            node = word(mem, node.wrapping_add(4));
        }
        if node == tree {
            return Some(last);
        }
    }
}

/// fn_1966ac + fn_1966ec (identical bodies) / EU fn_19675c + fn_19679c /
/// KR fn_196798 + fn_1967d8.
///
/// Looks `key` up in the tree at `base` + 24 and reports the found node's
/// value word through `out`. A missing base/output or a zero key reports
/// `0x89002007` without searching; a miss reports `0x89002014`.
pub fn lookup_value(
    mem: &[u8],
    base: Option<u32>,
    key: u32,
    out: Option<&mut u32>,
    cmp: impl FnMut(u32, u32) -> i32,
) -> u32 {
    let (Some(base), Some(out)) = (base, out) else {
        return 0x89002007;
    };
    if key == 0 {
        return 0x89002007;
    }
    let tree = base.wrapping_add(24);
    let head = u32::from_le_bytes(mem[tree as usize..tree as usize + 4].try_into().unwrap());
    match find_node(
        mem,
        tree,
        head,
        mem[tree.wrapping_add(40) as usize],
        key,
        cmp,
    ) {
        Some(node) => {
            let at = node.wrapping_add(20) as usize;
            *out = u32::from_le_bytes(mem[at..at + 4].try_into().unwrap());
            0
        }
        None => 0x89002014,
    }
}

pub(crate) fn mem_word(mem: &[u8], at: u32) -> u32 {
    u32::from_le_bytes(mem[at as usize..at as usize + 4].try_into().unwrap())
}

pub(crate) fn mem_store(mem: &mut [u8], at: u32, value: u32) {
    mem[at as usize..at as usize + 4].copy_from_slice(&value.to_le_bytes());
}

fn block_at(mem: &[u8], at: u32) -> GrowBlock {
    let at = at as usize;
    GrowBlock {
        data: mem_word(mem, at as u32),
        elem_size: u16::from_le_bytes(mem[at + 4..at + 6].try_into().unwrap()),
        callback: mem_word(mem, at as u32 + 8),
        cap: mem_word(mem, at as u32 + 16),
        count: mem_word(mem, at as u32 + 20),
        grow: u16::from_le_bytes(mem[at + 24..at + 26].try_into().unwrap()),
        heap: mem_word(mem, at as u32 + 28),
    }
}

fn store_block(mem: &mut [u8], at: u32, block: &GrowBlock) {
    mem_store(mem, at, block.data);
    mem[at as usize + 4..at as usize + 6].copy_from_slice(&block.elem_size.to_le_bytes());
    mem_store(mem, at + 8, block.callback);
    mem_store(mem, at + 16, block.cap);
    mem_store(mem, at + 20, block.count);
    mem[at as usize + 24..at as usize + 26].copy_from_slice(&block.grow.to_le_bytes());
    mem_store(mem, at + 28, block.heap);
}

/// Caller-owned parameters for `fn_174e74`: heap id, optional tree base and
/// optional extra-block offset into `mem`.
pub struct RegisterParams {
    pub heap: u32,
    pub values: Option<u32>,
    pub extra: Option<u32>,
}

/// fn_174e74 / EU fn_174f24 / KR fn_174ed0.
///
/// Registers an owner: with a live key, looks up the values tree and
/// appends the 12 owner bytes to the found entry's callback block (reached
/// through the entry's value word plus 8), creating it from `def` when
/// missing; with a null or empty key, appends to the extra block instead.
/// A missing owner or a null `set` word reports
/// `0x8b702007` without locking; lookup failures remap `0x89002007` /
/// `0x89002014` to `0x8b702007` / `0x8b702014`; create/append failures
/// report `0x8b70200a`; a failing unlock overrides a success. Key identity
/// for the search is the second tuple element; the callback observes 0 as
/// the source id since the transient owner has no stable offset.
pub fn register_owner(
    params: Option<RegisterParams>,
    mem: &mut [u8],
    key: Option<(&[u8], u32)>,
    owner: [u32; 3],
    def: [u32; 3],
    lock: &mut impl StateLock,
    heap: &mut impl BlockHeap,
    cmp: impl FnMut(u32, u32) -> i32,
    cb: Option<impl FnMut(u32, u32, u32) -> u32>,
) -> u32 {
    const MISSING: u32 = 0x8b702007;
    const FAILED: u32 = 0x8b70200a;
    const ABSENT: u32 = 0x8b702014;
    let Some(params) = params else {
        return MISSING;
    };
    if owner[2] == 0 {
        return MISSING;
    }
    let mut status = lock.acquire();
    if status >= 0x80000000 {
        return status;
    }
    let mut cb = cb;
    let live = match key {
        Some((bytes, id)) if !bytes.is_empty() && bytes[0] != 0 => Some(id),
        _ => None,
    };
    if let Some(id) = live {
        let found = match params.values {
            None => {
                status = MISSING;
                None
            }
            Some(base) => {
                let tree = base.wrapping_add(24);
                let head = mem_word(mem, tree);
                let flag = mem[tree.wrapping_add(40) as usize];
                match find_node(mem, tree, head, flag, id, cmp) {
                    None => {
                        status = ABSENT;
                        None
                    }
                    Some(node) => {
                        status = 0;
                        Some(node)
                    }
                }
            }
        };
        if let Some(node) = found {
            let value_at = mem_word(mem, node.wrapping_add(20));
            let slot_at = value_at.wrapping_add(8);
            let mut target = mem_word(mem, slot_at);
            if target == 0 {
                let mut fresh = 0u32;
                if create_block(
                    heap,
                    params.heap,
                    def[0],
                    def[1],
                    def[2],
                    1,
                    mem,
                    Some(&mut fresh),
                ) == 0
                {
                    status = FAILED;
                } else {
                    mem_store(mem, slot_at, fresh);
                    target = fresh;
                }
            }
            if target != 0 {
                let mut block = block_at(mem, target);
                let mut bytes = [0u8; 12];
                bytes[0..4].copy_from_slice(&owner[0].to_le_bytes());
                bytes[4..8].copy_from_slice(&owner[1].to_le_bytes());
                bytes[8..12].copy_from_slice(&owner[2].to_le_bytes());
                if append_element(Some(&mut block), mem, 0, &bytes, heap, cb.as_mut()) == 0 {
                    status = FAILED;
                } else {
                    store_block(mem, target, &block);
                }
            }
        }
    } else {
        match params.extra {
            None => status = FAILED,
            Some(off) => {
                let mut block = block_at(mem, off);
                let mut bytes = [0u8; 12];
                bytes[0..4].copy_from_slice(&owner[0].to_le_bytes());
                bytes[4..8].copy_from_slice(&owner[1].to_le_bytes());
                bytes[8..12].copy_from_slice(&owner[2].to_le_bytes());
                if append_element(Some(&mut block), mem, 0, &bytes, heap, cb.as_mut()) == 0 {
                    status = FAILED;
                } else {
                    store_block(mem, off, &block);
                }
            }
        }
    }
    let release = lock.release();
    if release >= 0x80000000 && status < 0x80000000 {
        status = release;
    }
    status
}

fn mem_half(mem: &[u8], at: u32) -> u16 {
    u16::from_le_bytes(mem[at as usize..at as usize + 2].try_into().unwrap())
}

fn mem_store16(mem: &mut [u8], at: u32, value: u16) {
    mem[at as usize..at as usize + 2].copy_from_slice(&value.to_le_bytes());
}

fn nz(value: u32) -> Option<u32> {
    if value == 0 {
        None
    } else {
        Some(value)
    }
}

/// fn_195aac / EU fn_195b5c / KR fn_195b98.
///
/// Bounded byte comparison: compares up to `n` bytes and reports the
/// first differing pair as unsigned bytes. A zero bound reports 0 without
/// reading. Both sides must lend at least `n` bytes when `n` is nonzero.
pub fn compare_bytes(a: &[u8], b: &[u8], n: u32) -> i32 {
    if n == 0 {
        return 0;
    }
    let mut left = n - 1;
    let mut i = 0usize;
    while left != 0 && a[i] == b[i] {
        i += 1;
        left -= 1;
    }
    a[i] as i32 - b[i] as i32
}

/// fn_195d90 / EU fn_195e40 / KR fn_195e7c.
///
/// Reports the length of the NUL-terminated bytes at the start of `s`.
/// The terminator must be within the borrowed slice.
pub fn strlen_bytes(s: &[u8]) -> u32 {
    let mut len = 0u32;
    while s[len as usize] != 0 {
        len = len.wrapping_add(1);
    }
    len
}

/// Scripted heap vtable behind `fn_196394` / `fn_1963e0` / `fn_196428`:
/// `obj` is the heap object's context word (offset +4), the method words
/// themselves are never read. Offsets into caller memory, `0` is null.
pub trait HeapVtable {
    fn alloc_at(&mut self, obj: u32, size: u32) -> u32;
    fn realloc_at(&mut self, obj: u32, old: u32, size: u32) -> u32;
    fn release_at(&mut self, obj: u32, object: u32);
}

/// fn_196394 / EU fn_196444 / KR fn_196480.
///
/// Allocates `size` bytes through the heap's first method. A null heap or
/// a zero size reports 0 without calling.
pub fn heap_alloc(mem: &[u8], heap: Option<u32>, size: u32, vt: &mut impl HeapVtable) -> u32 {
    let Some(heap) = heap else {
        return 0;
    };
    if size == 0 {
        return 0;
    }
    vt.alloc_at(mem_word(mem, heap + 4), size)
}

/// fn_1963e0 / EU fn_196490 / KR fn_1964cc.
///
/// Regrows `old` to `size` bytes through the heap's third method, or
/// allocates through the first method when `old` is null. A null heap or
/// a zero size reports 0 without calling.
pub fn heap_realloc(
    mem: &[u8],
    heap: Option<u32>,
    old: u32,
    size: u32,
    vt: &mut impl HeapVtable,
) -> u32 {
    let Some(heap) = heap else {
        return 0;
    };
    if size == 0 {
        return 0;
    }
    let obj = mem_word(mem, heap + 4);
    if old != 0 {
        vt.realloc_at(obj, old, size)
    } else {
        vt.alloc_at(obj, size)
    }
}

/// fn_196428 / EU fn_1964d8 / KR fn_196514.
///
/// Releases `object` through the heap's fourth method. A null heap or a
/// null object returns without calling.
pub fn heap_release(mem: &[u8], heap: Option<u32>, object: Option<u32>, vt: &mut impl HeapVtable) {
    let Some(heap) = heap else {
        return;
    };
    let Some(object) = object else {
        return;
    };
    vt.release_at(mem_word(mem, heap + 4), object);
}

/// fn_180d3c / EU fn_180dec / KR fn_180d98.
///
/// Allocates `size` bytes (rounded up to 4) from the arena's chunk
/// list, growing it through `alloc` when no chunk fits. A fresh
/// arena (zero total) grows by at least its minimum size, otherwise
/// by at least the grow size; the new chunk links at the head or the
/// tail and the total advances, then the used count advances by the
/// rounded size. `alloc` takes the context heap and the chunk size
/// with header and reports the new chunk or 0, in which case the
/// arena is unchanged and 0 is returned.
pub fn arena_alloc(
    mem: &mut [u8],
    arena: u32,
    size: u32,
    alloc: &mut impl FnMut(u32, u32) -> u32,
) -> u32 {
    let n = size.wrapping_add(3) & !3;
    let mut prev = 0u32;
    let mut chunk = mem_word(mem, arena + 8);
    let fit = loop {
        if chunk == 0 {
            break None;
        }
        if mem_word(mem, chunk + 8).wrapping_add(n) <= mem_word(mem, chunk + 12) {
            break Some(chunk);
        }
        prev = chunk;
        chunk = mem_word(mem, chunk);
    };
    let chunk = match fit {
        Some(chunk) => chunk,
        None => {
            let mut grow = n;
            if mem_word(mem, arena + 20) == 0 && mem_word(mem, arena + 12) > grow {
                grow = mem_word(mem, arena + 12);
            } else if mem_word(mem, arena + 16) > grow {
                grow = mem_word(mem, arena + 16);
            }
            let heap = mem_word(mem, mem_word(mem, arena) + 4);
            let chunk = alloc(heap, grow.wrapping_add(16));
            if chunk == 0 {
                return 0;
            }
            mem_store(mem, chunk + 4, chunk.wrapping_add(16));
            mem_store(mem, chunk, 0);
            mem_store(mem, chunk + 8, chunk.wrapping_add(16));
            mem_store(
                mem,
                chunk + 12,
                chunk.wrapping_add(16).wrapping_add(grow),
            );
            mem_store(
                mem,
                arena + 20,
                mem_word(mem, arena + 20).wrapping_add(grow),
            );
            if prev == 0 {
                mem_store(mem, arena + 8, chunk);
            } else {
                mem_store(mem, prev, chunk);
            }
            chunk
        }
    };
    mem_store(
        mem,
        arena + 24,
        mem_word(mem, arena + 24).wrapping_add(n),
    );
    let result = mem_word(mem, chunk + 8);
    mem_store(mem, chunk + 8, result.wrapping_add(n));
    result
}

/// Scripted tree operations behind `fn_198868` / `fn_198e34`: comparison,
/// key creation, handle allocation and both releases. The `node` argument
/// to `tree_init` is the new node's offset so the caller can see the key
/// slot the original passes by pointer; a reported key is stored there
/// whether creation succeeds or not, since the callback runs first.
pub trait TreeOps {
    fn tree_cmp(&mut self, key: u32, node_key: u32) -> i32;
    fn tree_init(&mut self, key: u32, heap: u32, node: u32) -> (u32, Option<u32>);
    fn tree_alloc(&mut self, arg: u32, heap: u32) -> (u32, u32);
    fn tree_free_node(&mut self, p: u32, heap: u32);
    fn tree_free_handle(&mut self, p: u32, heap: u32);
}

pub(crate) fn rebalance(mem: &mut [u8], t: u32, mut n: u32, mut p: u32) {
    while mem[(p + 12) as usize] == 0 {
        let g = mem_word(mem, p + 8);
        if p == mem_word(mem, g) {
            let mut c = mem_word(mem, g + 4);
            if mem[(c + 12) as usize] == 0 {
                mem[(p + 12) as usize] = 1;
                mem[(c + 12) as usize] = 1;
                mem[(g + 12) as usize] = 0;
                n = g;
                p = mem_word(mem, g + 8);
            } else {
                if n == mem_word(mem, p + 4) {
                    c = mem_word(mem, p + 4);
                    let y = mem_word(mem, c);
                    mem_store(mem, p + 4, y);
                    mem_store(mem, y + 8, p);
                    let q = mem_word(mem, p + 8);
                    mem_store(mem, c + 8, q);
                    if mem_word(mem, q) == p {
                        mem_store(mem, q, c);
                    } else {
                        mem_store(mem, q + 4, c);
                    }
                    mem_store(mem, c, p);
                    mem_store(mem, p + 8, c);
                    p = n;
                }
                mem[(p + 12) as usize] = 1;
                mem[(g + 12) as usize] = 0;
                c = mem_word(mem, g);
                let y = mem_word(mem, c + 4);
                mem_store(mem, g, y);
                mem_store(mem, y + 8, g);
                let q = mem_word(mem, g + 8);
                mem_store(mem, c + 8, q);
                if mem_word(mem, q + 4) == g {
                    mem_store(mem, q + 4, c);
                } else {
                    mem_store(mem, q, c);
                }
                mem_store(mem, c + 4, g);
                mem_store(mem, g + 8, c);
                break;
            }
        } else {
            let mut c = mem_word(mem, g);
            if mem[(c + 12) as usize] == 0 {
                mem[(p + 12) as usize] = 1;
                mem[(c + 12) as usize] = 1;
                mem[(g + 12) as usize] = 0;
                n = g;
                p = mem_word(mem, g + 8);
            } else {
                if n == mem_word(mem, p) {
                    c = mem_word(mem, p);
                    let y = mem_word(mem, c + 4);
                    mem_store(mem, p, y);
                    mem_store(mem, y + 8, p);
                    let q = mem_word(mem, p + 8);
                    mem_store(mem, c + 8, q);
                    if mem_word(mem, q + 4) == p {
                        mem_store(mem, q + 4, c);
                    } else {
                        mem_store(mem, q, c);
                    }
                    mem_store(mem, c + 4, p);
                    mem_store(mem, p + 8, c);
                    p = n;
                }
                mem[(p + 12) as usize] = 1;
                mem[(g + 12) as usize] = 0;
                c = mem_word(mem, g + 4);
                let y = mem_word(mem, c);
                mem_store(mem, g + 4, y);
                mem_store(mem, y + 8, g);
                let q = mem_word(mem, g + 8);
                mem_store(mem, c + 8, q);
                if mem_word(mem, q) == g {
                    mem_store(mem, q, c);
                } else {
                    mem_store(mem, q + 4, c);
                }
                mem_store(mem, c, g);
                mem_store(mem, g + 8, c);
                break;
            }
        }
    }
    let root = mem_word(mem, t);
    mem[(root + 12) as usize] = 1;
}

/// fn_198868 / EU fn_198918 / KR fn_198954.
///
/// Inserts `node` under `key` into the red-black tree rooted at `tree`
/// (itself the sentinel header: root at +0, size at +24, heap id at +32,
/// unique flag at +40). Nodes are 44-byte records. A duplicate key in a
/// non-unique tree reports `0x89002002`; a failing key creation reports
/// its status; otherwise the node is linked, counted, painted red,
/// rebalanced and the creation status is reported.
pub fn tree_insert(mem: &mut [u8], tree: u32, node: u32, key: u32, ops: &mut impl TreeOps) -> u32 {
    let mut p = tree;
    let mut x = mem_word(mem, tree);
    let mut cmp = -1i32;
    while x != tree {
        p = x;
        cmp = ops.tree_cmp(key, mem_word(mem, x + 16));
        if mem[(tree + 40) as usize] == 0 && cmp == 0 {
            return 0x89002002;
        }
        x = if cmp < 0 {
            mem_word(mem, x)
        } else {
            mem_word(mem, x + 4)
        };
    }
    let (rc, out_key) = ops.tree_init(key, mem_word(mem, tree + 32), node);
    if let Some(out_key) = out_key {
        mem_store(mem, node + 16, out_key);
    }
    if rc >= 0x80000000 {
        return rc;
    }
    if cmp < 0 {
        mem_store(mem, p, node);
    } else {
        mem_store(mem, p + 4, node);
    }
    mem_store(mem, node, tree);
    mem_store(mem, node + 4, tree);
    mem_store(mem, node + 8, p);
    mem_store(mem, tree + 24, mem_word(mem, tree + 24).wrapping_add(1));
    mem[(node + 12) as usize] = 0;
    rebalance(mem, tree, node, p);
    rc
}

/// fn_198e34 / EU fn_198ee4 / KR fn_198f20.
///
/// Allocates a 24-byte node through the heap, takes an optional handle
/// for `b`, and inserts under `a`. Reports 0 on success; heap failure
/// reports `0x8900200a`, and a failed insert frees the handle, the node
/// key and the node before reporting the insert status. The original's
/// fourth argument is dead (never read by the ARM), so the port takes
/// three.
pub fn tree_alloc_insert(
    mem: &mut [u8],
    s: u32,
    a: u32,
    b: u32,
    heap_vt: &mut impl HeapVtable,
    ops: &mut impl TreeOps,
) -> u32 {
    let heap = mem_word(mem, s + 32);
    let n = heap_alloc(mem, nz(heap), 0x18, heap_vt);
    if n == 0 {
        return 0x8900200a;
    }
    let mut tmp = 0u32;
    if b != 0 {
        let (r, out) = ops.tree_alloc(b, heap);
        if r >= 0x80000000 {
            heap_release(mem, nz(heap), nz(n), heap_vt);
            return r;
        }
        tmp = out;
    }
    mem_store(mem, n + 16, 0);
    mem_store(mem, n + 8, 0);
    mem_store(mem, n, 0);
    mem_store(mem, n + 4, 0);
    mem[n as usize + 12] = 0;
    mem_store(mem, n + 20, tmp);
    let r = tree_insert(mem, s, n, a, ops);
    if r < 0x80000000 {
        return 0;
    }
    if mem_word(mem, n + 16) != 0 {
        ops.tree_free_node(mem_word(mem, n + 16), heap);
    }
    if mem_word(mem, n + 20) != 0 {
        ops.tree_free_handle(tmp, heap);
    }
    heap_release(mem, nz(heap), nz(n), heap_vt);
    r
}

/// fn_19668c / EU fn_19673c / KR fn_196778.
///
/// Inserts into the tree 24 bytes past `base` under `key`, carrying
/// `value` opaquely to the handle allocator. Any null argument reports
/// `0x89002007` without touching the tree.
pub fn insert_guarded(
    mem: &mut [u8],
    base: Option<u32>,
    key: u32,
    value: u32,
    heap_vt: &mut impl HeapVtable,
    ops: &mut impl TreeOps,
) -> u32 {
    let Some(base) = base else {
        return 0x89002007;
    };
    if key == 0 || value == 0 {
        return 0x89002007;
    }
    tree_alloc_insert(mem, base.wrapping_add(24), key, value, heap_vt, ops)
}

/// Scripted element callback behind `fn_176104`: `a` is the element's
/// first word, `q`/`r` the caller's opaque cookies.
pub trait ElementCallback {
    fn call_element(&mut self, a: u32, q: u32, r: u32) -> u32;
}

/// fn_176104 / EU fn_1761b4 / KR fn_176160.
///
/// Runs the element callbacks of the table at `o` + 8, then the table at
/// `p` + 12, each as `(first_word, q, r)`. Any null argument reports 0
/// without running; a failed slot lookup reports `0x8b702100`; the first
/// failing callback status is reported.
pub fn run_element_callbacks(
    mem: &[u8],
    p: Option<u32>,
    q: u32,
    o: Option<u32>,
    r: u32,
    cb: &mut impl ElementCallback,
) -> u32 {
    let (Some(p), Some(o)) = (p, o) else {
        return 0;
    };
    if q == 0 || r == 0 {
        return 0;
    }
    for pass in [1u32, 2] {
        let l = if pass == 1 {
            mem_word(mem, o + 8)
        } else {
            mem_word(mem, p + 12)
        };
        if l != 0 {
            let mut words = [0u32; 6];
            for (i, slot) in words.iter_mut().enumerate() {
                *slot = mem_word(mem, l.wrapping_add((i as u32).wrapping_mul(4)));
            }
            let n = word_at5(Some(&words));
            if n != 0 {
                let mut i = 0u32;
                while i < n {
                    let table = ElementTable {
                        data: &[],
                        elem_size: mem_half(mem, l + 4),
                        count: mem_word(mem, l + 20),
                    };
                    let mut h = 0u32;
                    if element_at(Some(&table), i, Some(&mut h)) == 0 {
                        return 0x8b702100;
                    }
                    let e = mem_word(mem, l).wrapping_add(h);
                    let rc = cb.call_element(mem_word(mem, e), q, r);
                    if rc >= 0x80000000 {
                        return rc;
                    }
                    i = i.wrapping_add(1);
                }
            }
        }
    }
    0
}

/// fn_176054 / EU fn_176104 / KR fn_1760b0.
///
/// Stores `n` bytes from `src_off` into the `{data, size}` slot at
/// `slot_at`, allocating when empty and regrowing when short. A zero
/// length releases a live buffer and stores the source offset as-is. A
/// null slot reports `0x8b702007`; heap failure reports `0x8b70200a`,
/// storing the null first on the allocate path but not on regrow.
/// The regrow call passes all three arguments (heap, data, length) as the
/// ARM does; `src/fn_176054.c` line 19 drops the length (C defect: the
/// callee then reads an unpassed third parameter).
pub fn store_sized(
    mem: &mut [u8],
    slot_at: Option<u32>,
    src_off: u32,
    n: u32,
    heap: u32,
    vt: &mut impl HeapVtable,
) -> u32 {
    let Some(slot_at) = slot_at else {
        return 0x8b702007;
    };
    if n == 0 {
        if mem_word(mem, slot_at) != 0 && mem_word(mem, slot_at + 4) != 0 {
            heap_release(mem, nz(heap), nz(mem_word(mem, slot_at)), vt);
        }
        mem_store(mem, slot_at + 4, 0);
        mem_store(mem, slot_at, src_off);
    } else {
        let size = mem_word(mem, slot_at + 4);
        if size == 0 {
            let fresh = heap_alloc(mem, nz(heap), n, vt);
            mem_store(mem, slot_at, fresh);
            if fresh == 0 {
                return 0x8b70200a;
            }
        } else if size < n {
            let grown = heap_realloc(mem, nz(heap), mem_word(mem, slot_at), n, vt);
            if grown == 0 {
                return 0x8b70200a;
            }
            mem_store(mem, slot_at, grown);
        }
        let data = mem_word(mem, slot_at);
        let mut i = 0u32;
        while i < n {
            let b = mem[src_off.wrapping_add(i) as usize];
            mem[data.wrapping_add(i) as usize] = b;
            i = i.wrapping_add(1);
        }
        mem_store(mem, slot_at + 4, n);
    }
    0
}

/// fn_174b38 / EU fn_174be8 / KR fn_174b94.
///
/// Walks the element table at `obj` + 16, offering `{data, size}` to each
/// entry's tree. A duplicate-key answer with `upd` set looks the entry up
/// and replaces its value bytes, allocating or regrowing through the
/// heap id at the entry's double-indirect word. `scratch` lends 12 bytes
/// for the transient offered value. Lookup failures remap `0x890020xx`
/// to `0x8b7020xx`; a null value slot reports `0x8b702007` and heap
/// failure reports `0x8b70200a`.
pub fn update_attr(
    mem: &mut [u8],
    obj: u32,
    attr: u32,
    data: u32,
    size: u32,
    upd: u32,
    scratch: u32,
    heap_vt: &mut impl HeapVtable,
    ops: &mut impl TreeOps,
    mut cmp: impl FnMut(u32, u32) -> i32,
) -> u32 {
    mem_store(mem, scratch, data);
    mem_store(mem, scratch + 4, size);
    mem_store(mem, scratch + 8, 0);
    let dict = mem_word(mem, obj + 16);
    let mut r = 0u32;
    let mut i = 0u32;
    loop {
        if dict == 0 {
            break;
        }
        if i >= mem_word(mem, dict + 20) {
            break;
        }
        let ent = mem_word(mem, dict).wrapping_add(i.wrapping_mul(mem_half(mem, dict + 4) as u32));
        r = insert_guarded(mem, nz(mem_word(mem, ent + 4)), attr, scratch, heap_vt, ops);
        if r & 0x1fff == 2 {
            if upd == 0 {
                r = 0;
            } else {
                let mut slot = 0u32;
                let ent1 = mem_word(mem, ent + 4);
                r = lookup_value(mem, nz(ent1), attr, Some(&mut slot), &mut cmp);
                if r >= 0x80000000 {
                    r = (r & 0x1fff) | 0x8b702000;
                } else {
                    let pi = slot;
                    let alloc = mem_word(mem, mem_word(mem, ent));
                    if pi == 0 {
                        r = 0x8b702007;
                        break;
                    }
                    if size == 0 {
                        if mem_word(mem, pi) != 0 && mem_word(mem, pi + 4) != 0 {
                            heap_release(mem, nz(alloc), nz(mem_word(mem, pi)), heap_vt);
                        }
                        mem_store(mem, pi, data);
                        mem_store(mem, pi + 4, 0);
                        r = 0;
                    } else if mem_word(mem, pi + 4) == 0 {
                        let fresh = heap_alloc(mem, nz(alloc), size, heap_vt);
                        mem_store(mem, pi, fresh);
                        if fresh == 0 {
                            r = 0x8b70200a;
                            break;
                        }
                        let mut k = 0u32;
                        while k < size {
                            let b = mem[data.wrapping_add(k) as usize];
                            mem[mem_word(mem, pi).wrapping_add(k) as usize] = b;
                            k = k.wrapping_add(1);
                        }
                        mem_store(mem, pi + 4, size);
                        r = 0;
                    } else if mem_word(mem, pi + 4) < size {
                        let m = heap_realloc(mem, nz(alloc), mem_word(mem, pi), size, heap_vt);
                        if m == 0 {
                            r = 0x8b70200a;
                            break;
                        }
                        mem_store(mem, pi, m);
                        let mut k = 0u32;
                        while k < size {
                            let b = mem[data.wrapping_add(k) as usize];
                            mem[mem_word(mem, pi).wrapping_add(k) as usize] = b;
                            k = k.wrapping_add(1);
                        }
                        mem_store(mem, pi + 4, size);
                        r = 0;
                    } else {
                        let mut k = 0u32;
                        while k < size {
                            let b = mem[data.wrapping_add(k) as usize];
                            mem[mem_word(mem, pi).wrapping_add(k) as usize] = b;
                            k = k.wrapping_add(1);
                        }
                        mem_store(mem, pi + 4, size);
                        r = 0;
                    }
                }
            }
        }
        i = i.wrapping_add(1);
        if r >= 0x80000000 {
            break;
        }
    }
    r
}

/// Scripted change gate behind `fn_175170`: observes the callback's
/// context with the key/data offsets and either reports a status or
/// clears `allow` to veto, which reports `0x8b70200f`.
pub trait BeforeChange {
    fn before(&mut self, context: u32, key: u32, data: u32, allow: &mut u8) -> u32;
}

pub(crate) fn before_change(
    mem: &[u8],
    callbacks: u32,
    v: u32,
    key: u32,
    data: u32,
    before: &mut impl BeforeChange,
) -> u32 {
    if v == 0 {
        return 0;
    }
    let mut rc = 0u32;
    let mut allow = 1u8;
    for pass in [1u32, 2] {
        let list = if pass == 1 {
            mem_word(mem, v + 8)
        } else {
            callbacks
        };
        if list != 0 {
            let count = mem_word(mem, list + 20);
            if count != 0 {
                let mut i = 0u32;
                while rc < 0x80000000 && i < count {
                    let base = mem_word(mem, list);
                    let size = mem_half(mem, list + 4);
                    if i >= mem_word(mem, list + 20) {
                        return 0x8b702100;
                    }
                    let c = base.wrapping_add(i.wrapping_mul(size as u32));
                    if mem_word(mem, c + 4) != 0 {
                        rc = before.before(mem_word(mem, c), key, data, &mut allow);
                        if rc < 0x80000000 && allow == 0 {
                            rc = 0x8b70200f;
                            break;
                        }
                    }
                    i = i.wrapping_add(1);
                }
            }
        }
    }
    rc
}

/// fn_175170 / EU fn_175220 / KR fn_1751cc.
///
/// Sets the `key` parameter to `size` bytes at `data` under the caller's
/// lock, gating on the value's then the parameters' change callbacks,
/// pushing the update through `update_attr`, storing into the looked-up
/// value slot, and notifying element callbacks. A lookup miss builds a
/// transient value in the first 12 bytes of `scratch` and inserts it
/// instead; the next 12 bytes serve `update_attr`. A null parameters
/// base, key or data reports `0x8b702007` without locking; insert and
/// notify failures on a miss remap `0x890020xx` to `0x8b7020xx`; a
/// failing unlock overrides a success. `scratch` must be nonzero with 24
/// writable bytes.
#[allow(clippy::too_many_arguments)]
pub fn set_param(
    mem: &mut [u8],
    params_at: Option<u32>,
    key: u32,
    data: u32,
    size: u32,
    update: u32,
    scratch: u32,
    lock: &mut impl StateLock,
    heap_vt: &mut impl HeapVtable,
    ops: &mut impl TreeOps,
    mut cmp: impl FnMut(u32, u32) -> i32,
    before: impl BeforeChange,
    elem_cb: impl ElementCallback,
) -> u32 {
    const MISSING: u32 = 0x8b702007;
    let Some(params_at) = params_at else {
        return MISSING;
    };
    if key == 0 || data == 0 {
        return MISSING;
    }
    let heap = mem_word(mem, params_at);
    let values = mem_word(mem, params_at + 8);
    let callbacks = mem_word(mem, params_at + 12);
    let mut result = lock.acquire();
    if result >= 0x80000000 {
        return result;
    }
    let mut before = before;
    let mut elem_cb = elem_cb;
    let mut value = 0u32;
    let rc = lookup_value(mem, nz(values), key, Some(&mut value), &mut cmp);
    if rc < 0x80000000 {
        let (old_data, old_size) = if value == 0 {
            (0, 0)
        } else {
            let n = mem_word(mem, value + 4);
            if n == 0 {
                (0, 0)
            } else {
                (mem_word(mem, value), n)
            }
        };
        let mut need = old_size != size;
        if !need {
            let end = old_data.wrapping_add(size) as usize;
            let dend = data.wrapping_add(size) as usize;
            need = compare_bytes(
                &mem[old_data as usize..end],
                &mem[data as usize..dend],
                size,
            ) != 0;
        }
        if need {
            result = before_change(mem, callbacks, value, key, data, &mut before);
            if result < 0x80000000 {
                let (upd_data, upd_size) = if update != 0 {
                    (data, size)
                } else {
                    (old_data, old_size)
                };
                result = update_attr(
                    mem,
                    params_at,
                    key,
                    upd_data,
                    upd_size,
                    update,
                    scratch.wrapping_add(12),
                    &mut *heap_vt,
                    &mut *ops,
                    &mut cmp,
                );
                if result < 0x80000000 {
                    result = store_sized(mem, nz(value), data, size, heap, &mut *heap_vt);
                    if result < 0x80000000 {
                        result = run_element_callbacks(
                            mem,
                            Some(params_at),
                            key,
                            nz(value),
                            data,
                            &mut elem_cb,
                        );
                    }
                }
            }
        }
    } else {
        mem_store(mem, scratch, data);
        mem_store(mem, scratch + 4, size);
        mem_store(mem, scratch + 8, 0);
        result = before_change(mem, callbacks, scratch, key, data, &mut before);
        if result < 0x80000000 {
            let (upd_data, upd_size) = if update != 0 { (data, size) } else { (0, 0) };
            result = update_attr(
                mem,
                params_at,
                key,
                upd_data,
                upd_size,
                update,
                scratch.wrapping_add(12),
                &mut *heap_vt,
                &mut *ops,
                &mut cmp,
            );
            if result < 0x80000000 {
                result = insert_guarded(mem, nz(values), key, scratch, &mut *heap_vt, &mut *ops);
                if result >= 0x80000000 {
                    result = (result & 0x1fff) | 0x8b702000;
                } else {
                    result = run_element_callbacks(
                        mem,
                        Some(params_at),
                        key,
                        Some(scratch),
                        data,
                        &mut elem_cb,
                    );
                    if result >= 0x80000000 {
                        result = (result & 0x1fff) | 0x8b702000;
                    }
                }
            }
        }
    }
    let rc = lock.release();
    if rc >= 0x80000000 && result < 0x80000000 {
        result = rc;
    }
    result
}

/// fn_1759a0 / EU fn_175a50 / KR fn_1759fc.
///
/// Sets parameter `key` to the NUL-terminated bytes at `s` (length plus
/// terminator) without the update flag. A null `s` uses the empty string
/// at `empty`.
#[allow(clippy::too_many_arguments)]
pub fn set_param_str(
    mem: &mut [u8],
    params_at: Option<u32>,
    key: u32,
    s: u32,
    empty: u32,
    scratch: u32,
    lock: &mut impl StateLock,
    heap_vt: &mut impl HeapVtable,
    ops: &mut impl TreeOps,
    cmp: impl FnMut(u32, u32) -> i32,
    before: impl BeforeChange,
    elem_cb: impl ElementCallback,
) -> u32 {
    let s = if s == 0 { empty } else { s };
    let n = strlen_bytes(&mem[s as usize..]).wrapping_add(1);
    set_param(
        mem, params_at, key, s, n, 0, scratch, lock, heap_vt, ops, cmp, before, elem_cb,
    )
}

/// fn_1759e0 / EU fn_175a90 / KR fn_175a3c.
///
/// Sets parameter `key` to the NUL-terminated bytes at `value` (length
/// plus terminator) with the update flag. A null `value` uses the empty
/// string at `empty`.
#[allow(clippy::too_many_arguments)]
pub fn set_param_kv(
    mem: &mut [u8],
    params_at: Option<u32>,
    key: u32,
    value: u32,
    empty: u32,
    scratch: u32,
    lock: &mut impl StateLock,
    heap_vt: &mut impl HeapVtable,
    ops: &mut impl TreeOps,
    cmp: impl FnMut(u32, u32) -> i32,
    before: impl BeforeChange,
    elem_cb: impl ElementCallback,
) -> u32 {
    let value = if value == 0 { empty } else { value };
    let n = strlen_bytes(&mem[value as usize..]).wrapping_add(1);
    set_param(
        mem, params_at, key, value, n, 1, scratch, lock, heap_vt, ops, cmp, before, elem_cb,
    )
}

/// fn_195bec / EU fn_195c9c / KR fn_195cd8.
///
/// Fills `n` bytes at `dst` with the low byte of `val`; returns `dst`.
pub fn fill_bytes(mem: &mut [u8], dst: u32, val: u32, n: u32) -> u32 {
    let value = val as u8;
    if n != 0 {
        let mut d = dst as usize;
        let mut left = n;
        loop {
            mem[d] = value;
            d += 1;
            left -= 1;
            if left == 0 {
                break;
            }
        }
    }
    dst
}

/// fn_195cd0 / EU fn_195d80 / KR fn_195dbc.
///
/// Copies the NUL-terminated bytes at `src` to `dst`, terminator
/// included; returns `dst`.
pub fn copy_str(mem: &mut [u8], dst: u32, src: u32) -> u32 {
    let mut d = dst as usize;
    let mut s = src as usize;
    loop {
        let c = mem[s];
        mem[d] = c;
        if c == 0 {
            break;
        }
        d += 1;
        s += 1;
    }
    dst
}

/// fn_195d58 / EU fn_195e08 / KR fn_195e44.
///
/// Compares two NUL-terminated strings, normalizing the first differing
/// byte difference to -1, 0 or 1.
pub fn compare_str(a: &[u8], b: &[u8]) -> i32 {
    let mut i = 0usize;
    let mut d = a[0] as i32 - b[0] as i32;
    while d == 0 && b[i] != 0 {
        i += 1;
        d = a[i] as i32 - b[i] as i32;
    }
    if d < 0 {
        -1
    } else if d > 0 {
        1
    } else {
        0
    }
}

/// fn_195b38 / EU fn_195be8 / KR fn_195c24.
///
/// Copies `n` bytes from `src` to `dst`, copying backwards when the
/// ranges overlap with `dst` above `src` (wrapping address arithmetic,
/// matching the ARM); returns `dst`.
pub fn move_bytes(mem: &mut [u8], dst: u32, src: u32, n: u32) -> u32 {
    if dst > src && src.wrapping_add(n) > dst {
        let mut d = dst.wrapping_add(n) as usize;
        let mut s = src.wrapping_add(n) as usize;
        let mut left = n;
        while left != 0 {
            left -= 1;
            d -= 1;
            s -= 1;
            let c = mem[s];
            mem[d] = c;
        }
    } else {
        let mut d = dst as usize;
        let mut s = src as usize;
        let mut left = n;
        while left != 0 {
            left -= 1;
            let c = mem[s];
            mem[d] = c;
            d += 1;
            s += 1;
        }
    }
    dst
}

/// fn_195c90 / EU fn_195d40 / KR fn_195d7c.
///
/// Appends the NUL-terminated bytes at `src` to the NUL-terminated
/// bytes at `dst`, terminator included; returns `dst`.
pub fn append_str(mem: &mut [u8], dst: u32, src: u32) -> u32 {
    let mut d = dst as usize;
    while mem[d] != 0 {
        d += 1;
    }
    let mut s = src as usize;
    loop {
        let c = mem[s];
        mem[d] = c;
        if c == 0 {
            break;
        }
        s += 1;
        d += 1;
    }
    dst
}

/// fn_195df8 / EU fn_195ea8 / KR fn_195ee4.
///
/// Compares at most `n` bytes of two NUL-terminated strings, reporting
/// the first differing byte difference (or the NUL-vs-byte difference
/// when `a` ends first). A zero `n` reports 0; a negative `n` behaves
/// as unbounded, matching the wrapping counter.
pub fn compare_str_n(a: &[u8], b: &[u8], n: i32) -> i32 {
    if n == 0 {
        return 0;
    }
    let mut left = n;
    let mut i = 0usize;
    loop {
        left = left.wrapping_sub(1);
        if left == 0 || a[i] == 0 || a[i] != b[i] {
            break;
        }
        i += 1;
    }
    a[i] as i32 - b[i] as i32
}

/// fn_1960a8 / EU fn_196158 / KR fn_196194.
///
/// Reports the offset of the first occurrence of the NUL-terminated
/// needle at `s2` in the haystack at `s1`, or 0 on a miss. An empty
/// needle matches at `s1`.
pub fn find_substr(mem: &[u8], s1: u32, s2: u32) -> u32 {
    if mem[s2 as usize] == 0 {
        return s1;
    }
    let mut h = s1 as usize;
    while mem[h] != 0 {
        let mut p1 = h;
        let mut p2 = s2 as usize;
        while mem[p1] != 0 && mem[p2] != 0 && mem[p1] == mem[p2] {
            p1 += 1;
            p2 += 1;
        }
        if mem[p2] == 0 {
            return h as u32;
        }
        h += 1;
    }
    0
}

/// fn_17c7d0 / EU fn_17c880 / KR fn_17c82c.
///
/// Reports bit 2 (value 4) of the class-table entry for the low byte of
/// `c`; the digit class.
pub fn char_is_digit(table: &[u16; 256], c: u32) -> u32 {
    (table[(c & 0xff) as usize] & 4) as u32
}

/// fn_17c800 / EU fn_17c8b0 / KR fn_17c85c.
///
/// Reports bit 3 (value 8) of the class-table entry for the low byte of
/// `c`; the whitespace class.
pub fn char_is_space(table: &[u16; 256], c: u32) -> u16 {
    table[(c & 0xff) as usize] & 8
}

/// fn_14e084 / EU fn_14e148 / KR fn_14e068.
///
/// Increments the reference count at `p` + 22 and reports `p` through
/// `out`. A null `p`, `b` or `out` reports `0x88f02007`; `b` is only
/// null-checked.
pub fn add_ref(mem: &mut [u8], p: Option<u32>, b: Option<u32>, out: Option<u32>) -> u32 {
    let (Some(p), Some(out)) = (p, out) else {
        return 0x88f02007;
    };
    if b.is_none() {
        return 0x88f02007;
    }
    let refs = mem_half(mem, p + 22).wrapping_add(1);
    mem[p as usize + 22..p as usize + 24].copy_from_slice(&refs.to_le_bytes());
    mem_store(mem, out, p);
    0
}

/// fn_160ac4 / EU fn_160b88 / KR fn_160aa8.
///
/// Null-tolerant string comparison: two live strings defer to the
/// folded comparison, otherwise a non-empty string against null reports
/// 1 and anything else reports 0.
pub fn compare_nullable(
    table: &[u16; 256],
    mem: &[u8],
    a: Option<u32>,
    b: Option<u32>,
) -> i32 {
    match (a, b) {
        (Some(a), Some(b)) => {
            let end = |o: u32| {
                let at = o as usize;
                at + mem[at..].iter().position(|&byte| byte == 0).unwrap() + 1
            };
            let ae = end(a);
            let be = end(b);
            compare_folded(table, &mem[a as usize..ae], &mem[b as usize..be])
        }
        (Some(a), None) => (mem[a as usize] != 0) as i32,
        (None, Some(b)) => (mem[b as usize] != 0) as i32,
        (None, None) => 0,
    }
}

/// fn_17b654 / EU fn_17b704 / KR fn_17b6b0.
///
/// Releases the extra block at `p` + 12 when live, then releases `p`
/// itself through the allocator at `p`; a null `p` reports
/// `0x8a402007`.
pub fn release_handle(mem: &[u8], p: Option<u32>, vt: &mut impl HeapVtable) -> u32 {
    let Some(p) = p else {
        return 0x8a402007;
    };
    let heap = mem_word(mem, p);
    let extra = mem_word(mem, p + 12);
    if extra != 0 {
        heap_release(mem, nz(heap), Some(extra), vt);
    }
    heap_release(mem, nz(heap), Some(p), vt);
    0
}

/// fn_14dfc0 / EU fn_14e084 / KR fn_14dfa4.
///
/// Releases the head block when live, then releases `p` itself through
/// `alloc`; a null `p` returns quietly.
pub fn release_list(
    mem: &[u8],
    p: Option<u32>,
    alloc: Option<u32>,
    vt: &mut impl HeapVtable,
) {
    let Some(p) = p else {
        return;
    };
    let head = mem_word(mem, p);
    if head != 0 {
        heap_release(mem, alloc, Some(head), vt);
    }
    heap_release(mem, alloc, Some(p), vt);
}

/// fn_196830 / EU fn_1968e0 / KR fn_19691c.
///
/// Releases `p` through the owner behind its vtable; a null `p`
/// reports `0x89002007`.
pub fn release_owner(mem: &[u8], p: Option<u32>, vt: &mut impl HeapVtable) -> u32 {
    let Some(p) = p else {
        return 0x89002007;
    };
    let vtable = mem_word(mem, p);
    heap_release(mem, nz(mem_word(mem, vtable + 20)), Some(p), vt);
    0
}

/// fn_160b0c / EU fn_160bd0 / KR fn_160af0.
///
/// Allocates an 8-byte object, stores the heap back-pointer, zeroes the
/// two halfwords and reports the object through `out`. A null `out`
/// reports `0x87f66006`; allocation failure reports `0x87f6600a`.
pub fn alloc_obj(
    mem: &mut [u8],
    heap: Option<u32>,
    out: Option<u32>,
    vt: &mut impl HeapVtable,
) -> u32 {
    let Some(out) = out else {
        return 0x87f66006;
    };
    let o = heap_alloc(mem, heap, 8, vt);
    if o == 0 {
        return 0x87f6600a;
    }
    mem_store(mem, o + 4, heap.unwrap_or(0));
    mem[o as usize..o as usize + 4].copy_from_slice(&[0, 0, 0, 0]);
    mem_store(mem, out, o);
    0
}

/// fn_19690c / EU fn_1969bc / KR fn_1969f8.
///
/// Duplicates the NUL-terminated string at `src` onto `heap` (length
/// plus terminator) and reports the copy through `out`. A null `src`,
/// `heap` or `out` reports `0x89002007`; allocation failure reports
/// `0x8900200a`.
pub fn clone_str(
    mem: &mut [u8],
    src: Option<u32>,
    heap: Option<u32>,
    out: Option<u32>,
    vt: &mut impl HeapVtable,
) -> u32 {
    let (Some(src), Some(out)) = (src, out) else {
        return 0x89002007;
    };
    if heap.is_none() {
        return 0x89002007;
    }
    let copy = heap_alloc(mem, heap, strlen_bytes(&mem[src as usize..]).wrapping_add(1), vt);
    if copy == 0 {
        return 0x8900200a;
    }
    copy_str(mem, copy, src);
    mem_store(mem, out, copy);
    0
}

/// fn_198a80 / EU fn_198b30 / KR fn_198b6c.
///
/// Reports the in-order successor of node `n` under the `end`
/// sentinel: down the `a` side it returns the last node of the
/// `next` chain, otherwise it climbs the `b` side while each node
/// still points back at the previous one, reporting 0 when the climb
/// reaches `end`.
pub fn successor_node(mem: &[u8], end: u32, n: u32) -> u32 {
    let a = mem_word(mem, n + 4);
    if a != end {
        let mut prev = a;
        let mut nx = mem_word(mem, prev);
        while nx != end {
            prev = nx;
            nx = mem_word(mem, nx);
        }
        prev
    } else {
        let mut nn = n;
        let mut cur = mem_word(mem, n + 8);
        while cur != end && mem_word(mem, cur + 4) == nn {
            nn = cur;
            cur = mem_word(mem, cur + 8);
        }
        if cur == end {
            0
        } else {
            cur
        }
    }
}

/// fn_196860 / EU fn_196910 / KR fn_19694c.
///
/// Advances the iterator at `it` to its pending node, reporting the
/// node's value pair through `outa`/`outb` and fetching the following
/// node; with no pending node it zeroes both outputs and reports
/// `0x89002017`. A null iterator or output reports `0x89002007`.
pub fn step_iter(
    mem: &mut [u8],
    it: Option<u32>,
    outa: Option<&mut u32>,
    outb: Option<&mut u32>,
) -> u32 {
    let (Some(it), Some(outa), Some(outb)) = (it, outa, outb) else {
        return 0x89002007;
    };
    let next = mem_word(mem, it + 8);
    if next != 0 {
        mem_store(mem, it + 4, next);
        *outa = mem_word(mem, next + 16);
        *outb = mem_word(mem, next + 20);
        mem_store(
            mem,
            it + 8,
            successor_node(mem, mem_word(mem, it).wrapping_add(24), next),
        );
        0
    } else {
        *outa = 0;
        *outb = 0;
        0x89002017
    }
}

/// Scripted two-argument method behind `fn_1963b8`: observes the
/// object word with both integer arguments and reports a status.
pub trait MethodTable {
    fn call_at(&mut self, obj: u32, a: u32, b: u32) -> u32;
}

/// fn_1963b8 / EU fn_196468 / KR fn_1964a4.
///
/// Dispatches `(a, b)` through the method table at `p`; a null `p`
/// or a null argument reports 0 without dispatching.
pub fn call_method(
    mem: &[u8],
    p: Option<u32>,
    a: u32,
    b: u32,
    vt: &mut impl MethodTable,
) -> u32 {
    let Some(p) = p else {
        return 0;
    };
    if a == 0 || b == 0 {
        return 0;
    }
    vt.call_at(mem_word(mem, p + 4), a, b)
}

/// fn_17b500 / EU fn_17b5b0 / KR fn_17b55c.
///
/// Allocates a handle cell through the method table at `a`, stores the
/// allocator back-pointer into the cell and reports the cell through
/// `out`. A null `out` reports `0x8a402007`; allocation failure
/// reports `0x8a40200a`.
pub fn create_handle(
    mem: &mut [u8],
    a: Option<u32>,
    out: Option<u32>,
    vt: &mut impl MethodTable,
) -> u32 {
    let Some(out) = out else {
        return 0x8a402007;
    };
    let p = call_method(mem, a, 1, 16, vt);
    mem_store(mem, out, p);
    if p != 0 {
        mem_store(mem, p, a.unwrap_or(0));
        0
    } else {
        0x8a40200a
    }
}

/// fn_1756d0 / EU fn_175780 / KR fn_17572c.
///
/// Looks `key` up in the tree at `o` + 8 under the caller's lock,
/// reporting the found entry's value pair through `p1`/`p2` (both
/// zeroed first, and zeroed again when the entry's size word is 0). A
/// null object, key or `p1` reports `0x8b702007`; lookup failures
/// remap `0x890020xx` to `0x8b7020xx`; a failing unlock overrides a
/// success.
pub fn lookup_param(
    mem: &[u8],
    obj: Option<u32>,
    key: u32,
    p1: Option<&mut u32>,
    p2: Option<&mut u32>,
    lock: &mut impl StateLock,
    cmp: impl FnMut(u32, u32) -> i32,
) -> u32 {
    let (Some(obj), Some(p1)) = (obj, p1) else {
        return 0x8b702007;
    };
    if key == 0 {
        return 0x8b702007;
    }
    let mut p2 = p2;
    *p1 = 0;
    if let Some(q) = p2.as_mut() {
        **q = 0;
    }
    let mut result = lock.acquire();
    if result >= 0x80000000 {
        return result;
    }
    let mut ent = 0u32;
    let rc = lookup_value(
        mem,
        nz(mem_word(mem, obj + 8)),
        key,
        Some(&mut ent),
        cmp,
    );
    if rc >= 0x80000000 {
        result = (rc & 0x1fff) | 0x8b702000;
    } else {
        result = 0;
        if ent != 0 {
            let e1 = mem_word(mem, ent + 4);
            if e1 != 0 {
                *p1 = mem_word(mem, ent);
                if let Some(q) = p2.as_mut() {
                    **q = e1;
                }
            } else {
                *p1 = 0;
                if let Some(q) = p2.as_mut() {
                    **q = 0;
                }
            }
        }
    }
    let t = lock.release();
    if t >= 0x80000000 && result < 0x80000000 {
        result = t;
    }
    result
}

/// fn_176cd8 / EU fn_176d88 / KR fn_176d34.
///
/// Parses an unsigned decimal integer, skipping leading blanks and
/// tabs and one optional `+`. Parsing walks the digit run from the
/// right with a decimal multiplier (wrapping); any other byte in the
/// scanned span, or no digits at all, reports 0, as does a null `s`.
pub fn parse_uint(mem: &[u8], s: Option<u32>) -> i32 {
    let Some(s) = s else {
        return 0;
    };
    let base = s as usize;
    let mut start = 0i32;
    let mut val = 0i32;
    let mut mul = 1i32;
    let last = (strlen_bytes(&mem[base..]) as i32).wrapping_sub(1);
    while start <= last
        && (mem[base + start as usize] == b' ' || mem[base + start as usize] == b'\t')
    {
        start += 1;
    }
    let mut i = start;
    if start < last && mem[base + i as usize] == b'+' {
        i = start + 1;
    }
    while i <= last && mem[base + i as usize] >= b'0' && mem[base + i as usize] <= b'9' {
        i += 1;
    }
    i -= 1;
    let n = i - start + 2;
    let mut k = 1i32;
    if n > 1 {
        loop {
            let d = (mem[base + i as usize] as i32).wrapping_sub(0x30) as u32;
            if d <= 9 {
                val = (val as u32)
                    .wrapping_add(d.wrapping_mul(mul as u32))
                    as i32;
                mul = mul.wrapping_mul(10);
            } else if i != start || mem[base + i as usize] != b'+' {
                return 0;
            }
            k += 1;
            i -= 1;
            if n <= k {
                break;
            }
        }
    }
    val
}

/// fn_17c550 / EU fn_17c600 / KR fn_17c5ac.
///
/// Parses a signed decimal integer, skipping whitespace, taking one
/// optional sign and accumulating the digit run (wrapping); no digits
/// parses as 0.
pub fn parse_int(table: &[u16; 256], mem: &[u8], s: u32) -> i32 {
    let mut p = s as usize;
    while char_is_space(table, mem[p] as u32) != 0 {
        p += 1;
    }
    let mut n = 0i32;
    let c0 = mem[p];
    p += 1;
    let mut c = c0;
    if c == b'-' || c == b'+' {
        c = mem[p];
        p += 1;
    }
    while char_is_digit(table, c as u32) != 0 {
        n = n
            .wrapping_mul(10)
            .wrapping_add(c as i32)
            .wrapping_sub(0x30);
        c = mem[p];
        p += 1;
    }
    if c0 == b'-' {
        n.wrapping_neg()
    } else {
        n
    }
}

/// fn_175944 / EU fn_1759f4 / KR fn_1759a0.
///
/// Looks up `(a, b)` and, on success with a size above 1, reports the
/// entry's value parsed as an unsigned integer through `out` (0
/// otherwise); returns the lookup status. `out` is always stored.
pub fn get_param_uint(
    mem: &mut [u8],
    a: Option<u32>,
    b: u32,
    out: u32,
    lock: &mut impl StateLock,
    cmp: impl FnMut(u32, u32) -> i32,
) -> u32 {
    let mut v0 = 0u32;
    let mut v1 = 0u32;
    let err = lookup_param(&*mem, a, b, Some(&mut v0), Some(&mut v1), lock, cmp);
    if err < 0x80000000 && v1 > 1 {
        let n = parse_uint(mem, nz(v0)) as u32;
        mem_store(mem, out, n);
    } else {
        mem_store(mem, out, 0);
    }
    err
}

/// fn_175850 / EU fn_175900 / KR fn_1758ac.
///
/// Looks up `(a, b)` and, on success with a size above 1, reports the
/// entry's value parsed as a signed integer through `out` (0
/// otherwise); returns the lookup status. `out` is always stored.
pub fn get_param_int(
    table: &[u16; 256],
    mem: &mut [u8],
    a: Option<u32>,
    b: u32,
    out: u32,
    lock: &mut impl StateLock,
    cmp: impl FnMut(u32, u32) -> i32,
) -> u32 {
    let mut v0 = 0u32;
    let mut v1 = 0u32;
    let err = lookup_param(&*mem, a, b, Some(&mut v0), Some(&mut v1), lock, cmp);
    if err < 0x80000000 && v1 > 1 {
        let n = parse_int(table, mem, v0) as u32;
        mem_store(mem, out, n);
    } else {
        mem_store(mem, out, 0);
    }
    err
}

/// fn_175a1c / EU fn_175acc / KR fn_175a78.
///
/// Looks up `(a, b)`, reporting the entry's value word through `c` and
/// the lookup status to the caller.
pub fn fetch_param(
    mem: &[u8],
    a: Option<u32>,
    b: u32,
    c: Option<&mut u32>,
    lock: &mut impl StateLock,
    cmp: impl FnMut(u32, u32) -> i32,
) -> u32 {
    lookup_param(mem, a, b, c, None, lock, cmp)
}

/// fn_196c7c / EU fn_196d2c / KR fn_196d68.
///
/// Destroys the element array at `a`: runs the destructor over the
/// live elements when present, releases the data block when live, then
/// releases the array itself; reports 1, or 0 for a null array.
pub fn destroy_array(
    mem: &[u8],
    a: Option<u32>,
    vt: &mut impl HeapVtable,
    mut dtor: impl FnMut(u32, u32),
) -> u32 {
    let Some(a) = a else {
        return 0;
    };
    let data = mem_word(mem, a);
    let ctx = mem_word(mem, a + 28);
    if data != 0 {
        if mem_word(mem, a + 12) != 0 {
            let size = mem_half(mem, a + 4) as u32;
            let count = mem_word(mem, a + 20);
            let mut i = 0u32;
            while i < count {
                dtor(data.wrapping_add(i.wrapping_mul(size)), ctx);
                i = i.wrapping_add(1);
            }
        }
        heap_release(mem, nz(ctx), Some(data), vt);
    }
    heap_release(mem, nz(ctx), Some(a), vt);
    1
}

/// fn_178818 / EU fn_1788c8 / KR fn_178874.
///
/// Releases the tag handle: a null registry or tag reports
/// `0x81506007`, otherwise the host-owned begin/end trace runs and 0
/// is reported.
pub fn release_tag(registry: Option<u32>, tag: Option<u32>) -> u32 {
    if registry.is_none() || tag.is_none() {
        return 0x81506007;
    }
    0
}

/// fn_176edc / EU fn_176f8c / KR fn_176f38.
///
/// Runs the object's `call(arg)` hook when present, then releases the
/// object through its heap when the hook reports success. A null `p`
/// reports `0x81402007`.
pub fn release_guarded(
    mem: &[u8],
    p: Option<u32>,
    vt: &mut impl HeapVtable,
    mut call: impl FnMut(u32) -> u32,
) -> u32 {
    let Some(p) = p else {
        return 0x81402007;
    };
    let sub = mem_word(mem, p);
    let ops = mem_word(mem, sub + 8);
    let mut rc = 0u32;
    if mem_word(mem, ops + 4) != 0 {
        rc = call(mem_word(mem, p + 8));
    }
    if rc < 0x80000000 {
        heap_release(mem, nz(mem_word(mem, p + 4)), Some(p), vt);
    }
    rc
}

/// fn_196e28 / EU fn_196ed8 / KR fn_196f14.
///
/// Removes element `idx` from the array at `a`: runs the destructor
/// over it when present, shifts the tail down with an overlapping
/// move and decrements the count. A null array or an out-of-range
/// index reports 0 without touching anything.
pub fn remove_at(
    mem: &mut [u8],
    a: Option<u32>,
    idx: u32,
    mut dtor: impl FnMut(u32, u32),
) -> u32 {
    let n = a.map_or(0, |a| mem_word(mem, a + 20));
    let Some(a) = a else { return 0 };
    if n <= idx {
        return 0;
    }
    let data = mem_word(mem, a);
    let size = mem_half(mem, a + 4) as u32;
    if mem_word(mem, a + 12) != 0 {
        dtor(
            data.wrapping_add(idx.wrapping_mul(size)),
            mem_word(mem, a + 28),
        );
    }
    let rest = n.wrapping_sub(idx).wrapping_sub(1) as i32;
    if rest != 0 {
        let elem = data.wrapping_add(idx.wrapping_mul(size));
        move_bytes(
            mem,
            elem,
            elem.wrapping_add(size),
            (size as i32).wrapping_mul(rest) as u32,
        );
    }
    mem_store(mem, a + 20, n.wrapping_sub(1));
    1
}

/// fn_190078 / EU fn_190128 / KR fn_190164.
///
/// Copies the NUL-terminated UTF-16 string at `src` to `dst` as
/// ASCII, replacing non-Latin1 units with `?`. A null source, or a
/// source (with terminator) longer than `cap`, clears `dst` and
/// reports `0x80000006` / `0x80000007`.
pub fn copy_utf16_as_ascii(
    mem: &mut [u8],
    src: Option<u32>,
    dst: u32,
    cap: u32,
) -> u32 {
    let Some(src) = src else {
        mem[dst as usize] = 0;
        return 0x80000006;
    };
    let mut len = 0u32;
    while mem_half(mem, src.wrapping_add(len.wrapping_mul(2))) != 0 {
        len = len.wrapping_add(1);
    }
    len = len.wrapping_add(1);
    if len > cap {
        mem[dst as usize] = 0;
        return 0x80000007;
    }
    let mut s = src as usize;
    let mut d = dst as usize;
    loop {
        let c = mem_half(mem, s as u32);
        if c == 0 {
            break;
        }
        mem[d] = if c & 0xff00 != 0 { 0x3f } else { c as u8 };
        s += 2;
        d += 1;
    }
    mem[d] = 0;
    0
}

/// fn_14de40 / EU fn_14df04 / KR fn_14de24.
///
/// Attaches the `(handle, type)` reference to the log object: stores
/// both words, raises the severity floor through the type's hook when
/// present, and marks the object ready once a fully-described type
/// lands. A null log reports 0; an already-typed log reports
/// `0x84b02000` without touching anything.
pub fn attach_log_handle(
    mem: &mut [u8],
    log: Option<u32>,
    handle: u32,
    typ: Option<u32>,
    mut severity: impl FnMut(u32) -> u32,
) -> u32 {
    let Some(log) = log else {
        return 0;
    };
    if mem_word(mem, log + 0x38) != 0 {
        return 0x84b02000;
    }
    mem_store(mem, log + 0x34, handle);
    mem_store(mem, log + 0x38, typ.unwrap_or(0));
    if let Some(typ) = typ {
        if mem_word(mem, typ + 0x28) != 0 {
            let v = severity(handle);
            if v > mem[log as usize + 0x30] as u32 {
                mem[log as usize + 0x30] = v as u8;
            }
        }
    }
    if mem[log as usize + 0x40] == 0 {
        if let Some(typ) = typ {
            if mem_word(mem, typ + 0x30) != 0 && mem_word(mem, typ + 0x34) != 0 {
                mem[log as usize + 0x40] = 1;
            }
        }
    }
    0
}

/// fn_173fd8 / EU fn_174088 / KR fn_174034.
///
/// Reports the maximum level over the five slots and the trailing
/// vtable entry, starting from 1 and clamped to 4 when the clamp
/// byte is set. A null `p` reports 0.
pub fn max_slot_level(
    mem: &[u8],
    p: Option<u32>,
    mut level: impl FnMut(u32) -> u32,
) -> u32 {
    let Some(p) = p else {
        return 0;
    };
    let mut max = 1u32;
    let mut i = 0u32;
    loop {
        let slot = p.wrapping_add(8).wrapping_add(i.wrapping_mul(8));
        let vt = mem_word(mem, slot + 4);
        if vt != 0 && mem_word(mem, vt + 40) != 0 {
            let v = level(mem_word(mem, slot));
            if v > max {
                max = v;
            }
        }
        i = i.wrapping_add(1) & 0xff;
        if i >= 5 {
            break;
        }
    }
    let vt = mem_word(mem, p + 0x38);
    if vt != 0 && mem_word(mem, vt + 40) != 0 {
        let v = level(mem_word(mem, p + 0x34));
        if v > max {
            max = v;
        }
    }
    if mem[p as usize + 0x31] != 0 && max > 4 {
        max = 4;
    }
    max
}

/// fn_14df44 / EU fn_14e008 / KR fn_14df28.
///
/// Makes an 8-byte heap word object through the context's method,
/// copies the input's half word into it and duplicates the input's
/// string onto it, reporting the object through `out`. A null input,
/// context or `out` reports `0x88f02007`; method failure reports
/// `0x88f0200a`; a failed duplication releases the object.
pub fn clone_labeled_word(
    mem: &mut [u8],
    input: Option<u32>,
    ctx: Option<u32>,
    out: Option<u32>,
    method_vt: &mut impl MethodTable,
    heap_vt: &mut impl HeapVtable,
) -> u32 {
    let (Some(input), Some(ctx), Some(out)) = (input, ctx, out) else {
        return 0x88f02007;
    };
    let o = call_method(mem, Some(ctx), 1, 8, method_vt);
    if o == 0 {
        return 0x88f0200a;
    }
    let second = mem_half(mem, input + 4);
    mem[o as usize + 4..o as usize + 6].copy_from_slice(&second.to_le_bytes());
    let r = clone_str(mem, nz(mem_word(mem, input)), Some(ctx), Some(o), heap_vt);
    if r < 0x80000000 {
        mem_store(mem, out, o);
    } else {
        heap_release(mem, Some(ctx), Some(o), heap_vt);
    }
    r
}

/// fn_176e40 / EU fn_176ef0 / KR fn_176e9c.
///
/// Makes a 12-byte node through the allocator's method, links it to
/// `o` and the allocator, then runs the object's create hook to fill
/// the payload word. A null argument reports `0x81402007`; method
/// failure reports `0x8140200a`; a failed create releases the node
/// and clears `out`.
pub fn create_node(
    mem: &mut [u8],
    o: Option<u32>,
    alloc: Option<u32>,
    out: Option<u32>,
    method_vt: &mut impl MethodTable,
    heap_vt: &mut impl HeapVtable,
    mut create: impl FnMut(u32, u32) -> (u32, u32),
) -> u32 {
    let (Some(o), Some(alloc), Some(out)) = (o, alloc, out) else {
        return 0x81402007;
    };
    let n = call_method(mem, Some(alloc), 1, 12, method_vt);
    mem_store(mem, out, n);
    if n == 0 {
        return 0x8140200a;
    }
    mem_store(mem, n, o);
    mem_store(mem, n + 4, alloc);
    let ops = mem_word(mem, o + 8);
    let mut rc = 0u32;
    if mem_word(mem, ops) != 0 {
        let (status, payload) = create(mem_word(mem, o + 12), mem_word(mem, o + 4));
        mem_store(mem, n + 8, payload);
        rc = status;
        if rc >= 0x80000000 {
            heap_release(mem, Some(alloc), Some(n), heap_vt);
            mem_store(mem, out, 0);
        }
    }
    rc
}

/// fn_17a2ec / EU fn_17a39c / KR fn_17a348.
///
/// Closes the tag iterator: releases the live entry and iterator,
/// then the iterator itself, all through the context heap. A null
/// iterator reports `0x81506007`.
pub fn close_tag_iter(
    mem: &[u8],
    p: Option<u32>,
    vt: &mut impl HeapVtable,
) -> u32 {
    let Some(p) = p else {
        return 0x81506007;
    };
    let ctx = mem_word(mem, p);
    let heap = nz(mem_word(mem, ctx + 4));
    let entry = mem_word(mem, p + 4);
    if entry != 0 {
        heap_release(mem, heap, Some(entry), vt);
    }
    let iterator = mem_word(mem, p + 8);
    if iterator != 0 {
        release_owner(mem, Some(iterator), vt);
    }
    heap_release(mem, heap, Some(p), vt);
    0
}

/// fn_1788c0 / EU fn_178970 / KR fn_17891c.
///
/// Closes the component-table iterator: releases the live key
/// (clearing it), the live iterator, then the iterator itself, all
/// through the context heap. A null iterator reports `0x81506007`.
pub fn close_component_iter(
    mem: &mut [u8],
    p: Option<u32>,
    vt: &mut impl HeapVtable,
) -> u32 {
    let Some(p) = p else {
        return 0x81506007;
    };
    let ctx = mem_word(mem, p);
    let heap = nz(mem_word(mem, ctx + 4));
    if mem_word(mem, p + 8) != 0 {
        heap_release(mem, heap, Some(mem_word(mem, p + 8)), vt);
        mem_store(mem, p + 8, 0);
    }
    let iterator = mem_word(mem, p + 12);
    if iterator != 0 {
        release_owner(mem, Some(iterator), vt);
    }
    heap_release(mem, heap, Some(p), vt);
    0
}

/// fn_177b7c / EU fn_177c2c / KR fn_177bd8.
///
/// Opens a broker object: clears `out`, makes a 12-byte object
/// through the context heap's method and links it to the context
/// logger and the broker. A null broker reports `0x81506007`;
/// method failure reports `0x8150600a`.
pub fn open_broker_object(
    mem: &mut [u8],
    _id: Option<u32>,
    broker: Option<u32>,
    out: u32,
    vt: &mut impl MethodTable,
) -> u32 {
    mem_store(mem, out, 0);
    let Some(broker) = broker else {
        return 0x81506007;
    };
    let ctx = mem_word(mem, broker + 8);
    let object = call_method(mem, nz(mem_word(mem, ctx + 4)), 1, 12, vt);
    if object == 0 {
        return 0x8150600a;
    }
    mem_store(mem, object, mem_word(mem, ctx + 16));
    mem_store(mem, object + 4, broker);
    mem_store(mem, out, object);
    0
}

/// fn_18f24c / EU fn_18f2fc / KR fn_18f338.
///
/// Sets the live-handle state byte under the handle lock: when the
/// current state is 1 it is reported through `out` and replaced by
/// `val`, otherwise `0x80002011` is reported. A failing lock
/// overrides only a success.
pub fn set_lh_state(
    mem: &mut [u8],
    p: u32,
    _name: u32,
    val: u32,
    out: u32,
    lock: &mut impl StateLock,
) -> u32 {
    let mut rc = lock.acquire();
    if rc >= 0x80000000 {
        return rc;
    }
    if mem[p as usize + 40] == 1 {
        mem[out as usize] = mem[p as usize + 40];
        mem[p as usize + 40] = val as u8;
    } else {
        rc = 0x80002011;
    }
    let rc2 = lock.release();
    if rc2 >= 0x80000000 && rc < 0x80000000 {
        rc = rc2;
    }
    rc
}

/// fn_1758f8 + fn_1758ac / EU fn_1759a8 + fn_17595c / KR fn_175954 +
/// fn_175908.
///
/// Formats the unsigned integer `c` into the first 16 bytes of the
/// 40-byte `scratch` region and stores it as the `key` parameter
/// with the given update flag. Mirrors the C wrappers, whose buffer
/// lives on the machine stack; `scratch` must sit outside the
/// observed ranges.
#[allow(clippy::too_many_arguments)]
pub fn set_param_uint(
    mem: &mut [u8],
    params_at: Option<u32>,
    key: u32,
    c: u32,
    update: u32,
    scratch: u32,
    lock: &mut impl StateLock,
    heap_vt: &mut impl HeapVtable,
    ops: &mut impl TreeOps,
    cmp: impl FnMut(u32, u32) -> i32,
    before: impl BeforeChange,
    elem_cb: impl ElementCallback,
) -> u32 {
    let at = scratch as usize;
    if render_unsigned(c, &mut mem[at..at + 16], 10).is_err() {
        mem[at] = 0;
    }
    let n = strlen_bytes(&mem[at..]).wrapping_add(1);
    set_param(
        mem,
        params_at,
        key,
        scratch,
        n,
        update,
        scratch.wrapping_add(16),
        lock,
        heap_vt,
        ops,
        cmp,
        before,
        elem_cb,
    )
}

/// fn_1757b8 / EU fn_175868 / KR fn_175814.
///
/// Formats the signed integer `val` into the first 16 bytes of
/// the 40-byte `scratch` region and stores it as the `key` parameter
/// without update. See `set_param_uint` for the scratch convention.
#[allow(clippy::too_many_arguments)]
pub fn set_param_int(
    mem: &mut [u8],
    params_at: Option<u32>,
    key: u32,
    val: u32,
    scratch: u32,
    lock: &mut impl StateLock,
    heap_vt: &mut impl HeapVtable,
    ops: &mut impl TreeOps,
    cmp: impl FnMut(u32, u32) -> i32,
    before: impl BeforeChange,
    elem_cb: impl ElementCallback,
) -> u32 {
    let at = scratch as usize;
    if render_signed(val as i32, &mut mem[at..at + 16], 10).is_err() {
        mem[at] = 0;
    }
    let n = strlen_bytes(&mem[at..]).wrapping_add(1);
    set_param(
        mem,
        params_at,
        key,
        scratch,
        n,
        0,
        scratch.wrapping_add(16),
        lock,
        heap_vt,
        ops,
        cmp,
        before,
        elem_cb,
    )
}

/// fn_176fc0 / EU fn_177070 / KR fn_17701c.
///
/// Reports whether the two 64-bit halves match: 1 when `ah == bh`
/// and `al == bl`, else 0.
pub fn match_halves(al: u32, ah: u32, bl: u32, bh: u32) -> u32 {
    u32::from(ah == bh && al == bl)
}

/// fn_1987fc / EU fn_1988ac / KR fn_1988e8.
///
/// Searches the tree at `a` for `b`: walks down the `hi` side while
/// the key compares greater, down the `lo` side while it compares
/// smaller (or whenever the flag is set, remembering the node),
/// reporting the node on an exact flagged-off match, the last
/// remembered node when the walk ends, or 0.
pub fn search_node(
    mem: &[u8],
    a: u32,
    b: u32,
    mut cmp: impl FnMut(u32, u32) -> i32,
) -> u32 {
    let mut cur = mem_word(mem, a);
    let mut prev = 0u32;
    let _ = mem_word(mem, a + 0x24);
    let flag = mem[a as usize + 0x28];
    while cur != a {
        let r = cmp(b, mem_word(mem, cur + 16));
        if r > 0 {
            cur = mem_word(mem, cur + 4);
        } else if r < 0 || flag != 0 {
            prev = cur;
            cur = mem_word(mem, cur);
        } else {
            return cur;
        }
    }
    prev
}

/// fn_198f74 / EU fn_199024 / KR fn_199060.
///
/// Reports the last node of the `head`-terminated link chain, or 0
/// when the chain holds fewer than two live nodes.
pub fn last_link(mem: &[u8], head: u32) -> u32 {
    let mut p = mem_word(mem, head);
    if p != head {
        let mut q = mem_word(mem, p);
        if q != head {
            loop {
                p = q;
                q = mem_word(mem, q);
                if q == head {
                    break;
                }
            }
        }
    }
    if p == head { 0 } else { p }
}

/// fn_198728 / EU fn_1987d8 / KR fn_198814.
///
/// Initializes the link node at `p` as a self-loop carrying
/// `(a, b, c)`: self links, zeroed zero/flag2 words, set flag.
pub fn init_link(mem: &mut [u8], p: u32, a: u32, b: u32, c: u32) {
    mem_store(mem, p + 24, 0);
    mem[p as usize + 12] = 1;
    mem[p as usize + 40] = 0;
    mem_store(mem, p + 36, b);
    mem_store(mem, p, p);
    mem_store(mem, p + 28, a);
    mem_store(mem, p + 32, c);
    mem_store(mem, p + 4, p);
    mem_store(mem, p + 8, p);
}

/// fn_198fa8 / EU fn_199058 / KR fn_199094.
///
/// Sets the flag byte at `p` + 0x28.
pub fn set_link_flag(mem: &mut [u8], p: u32) {
    mem[p as usize + 0x28] = 1;
}

/// fn_17b824 / EU fn_17b8d4 / KR fn_17b880.
///
/// Reports the word at `p` + 8, or 0 for a null `p`.
pub fn deref_slot2(mem: &[u8], p: Option<u32>) -> u32 {
    p.map_or(0, |p| mem_word(mem, p + 8))
}

/// fn_17a814 / EU fn_17a8c4 / KR fn_17a870.
///
/// Steps the tag iterator, reporting the entry name through `name`
/// and the entry value through `value` (null when the entry carries
/// attributes), plus the entry name through `attributes` when wanted.
/// A null iterator, `name` or `value` reports `0x81506007`; a name
/// that does not match the iterator filter reports `0x81506017`.
pub fn next_tag_entry(
    mem: &mut [u8],
    it: Option<u32>,
    name: Option<&mut u32>,
    value: Option<&mut u32>,
    attrs: Option<&mut u32>,
) -> u32 {
    let (Some(it), Some(name), Some(value)) = (it, name, value) else {
        return 0x81506007;
    };
    let mut got = 0u32;
    let mut entry_word = 0u32;
    let rc = step_iter(
        mem,
        nz(mem_word(mem, it + 8)),
        Some(&mut got),
        Some(&mut entry_word),
    );
    *name = got;
    if rc < 0x80000000 {
        let filter = mem_word(mem, it + 4);
        if filter != 0 {
            let a = &mem[filter as usize..];
            let b = &mem[got as usize..];
            let ae = a.iter().position(|&c| c == 0).unwrap();
            let be = b.iter().position(|&c| c == 0).unwrap();
            if compare_str(&a[..ae + 1], &b[..be + 1]) != 0 {
                return 0x81506017;
            }
        }
        let entry = entry_word;
        if mem_word(mem, entry + 8) == 0 {
            *value = mem_word(mem, entry);
        } else {
            *value = 0;
        }
        if let Some(attrs) = attrs {
            *attrs = mem_word(mem, entry + 4);
        }
    }
    rc
}

/// fn_160fec / EU fn_1610b0 / KR fn_160fd0.
///
/// Maps a raw status `code` to the reported error: codes below
/// `0x80000000` report 0, otherwise the `0x80001fff` masked code
/// reports itself when known, `0x8000010e` when the mask is 0, and
/// `0x80000000` when unknown.
pub fn classify_code(code: u32) -> u32 {
    if code < 0x80000000 {
        return 0;
    }
    let r = code & 0x80001fff;
    if r == 0 {
        return 0x8000010e;
    }
    match r {
        0x1..=0x6
        | 0x1fff
        | 0x80000000..=0x8000001b
        | 0x80000100..=0x80000113
        | 0x80000200
        | 0x80000300
        | 0x80000400
        | 0x8000040e
        | 0x80000416
        | 0x80000800..=0x8000080c
        | 0x80000880
        | 0x80000881 => r,
        _ => 0x80000000,
    }
}

/// fn_14fe9c / EU fn_14ff60 / KR fn_14fe80.
///
/// Starts a direct stream read: validates the `(object, tag)`
/// handle, then reports the readable window through `buffer` and
/// `available`. Failures clear both outputs (`available` only when
/// non-null) and report `0x88f02007` for a null output, a foreign
/// stream kind or a bad handle, `0x88f02004` when rewound past the
/// buffer base, and `0x88f0200a` for a null buffer with live data.
pub fn read_direct_start(
    mem: &mut [u8],
    obj: Option<u32>,
    tag: u32,
    buffer: Option<u32>,
    avail: Option<u32>,
) -> u32 {
    let rc = validate_handle(obj.is_some(), tag, 0x47de, 12);
    if rc >= 0x80000000 {
        return rc;
    }
    let stream = obj.unwrap_or(0);
    if buffer.is_none() || avail.is_none() {
        let rc = 0x88f02007;
        if buffer.is_none() {
            if let Some(avail) = avail {
                mem_store(mem, avail, 0);
            }
            return rc;
        }
        mem_store(mem, buffer.unwrap_or(0), 0);
        if let Some(avail) = avail {
            mem_store(mem, avail, 0);
        }
        return rc;
    }
    let state = mem_word(mem, stream + 4);
    if mem[stream as usize] != 0 {
        mem_store(mem, buffer.unwrap_or(0), 0);
        if let Some(avail) = avail {
            mem_store(mem, avail, 0);
        }
        return 0x88f02007;
    }
    let position = mem_word(mem, stream + 8);
    let base = mem_word(mem, state + 56);
    if position < base {
        mem_store(mem, buffer.unwrap_or(0), 0);
        if let Some(avail) = avail {
            mem_store(mem, avail, 0);
        }
        return 0x88f02004;
    }
    let available = mem_word(mem, state + 44).wrapping_sub(position);
    let bufp = position
        .wrapping_sub(base)
        .wrapping_add(mem_word(mem, state + 60));
    mem_store(mem, avail.unwrap_or(0), available);
    mem_store(mem, buffer.unwrap_or(0), bufp);
    if available != 0 && bufp == 0 {
        mem_store(mem, buffer.unwrap_or(0), 0);
        if let Some(avail) = avail {
            mem_store(mem, avail, 0);
        }
        return 0x88f0200a;
    }
    0
}

/// fn_198fb4 / EU fn_199064 / KR fn_1990a0.
///
/// Destroys the subtree at `n` down to the `end` sentinel:
/// recurses left and right, runs the context's pair hooks over the
/// live payload words, then releases the node through the context
/// heap.
pub fn destroy_subtree(
    mem: &[u8],
    c: u32,
    n: u32,
    end: u32,
    vt: &mut impl HeapVtable,
    dtor: &mut impl FnMut(u32, u32),
) {
    if n == end {
        return;
    }
    destroy_subtree(mem, c, mem_word(mem, n), end, vt, dtor);
    destroy_subtree(mem, c, mem_word(mem, n + 4), end, vt, dtor);
    let heap = mem_word(mem, c + 0x20);
    let vtab = mem_word(mem, c + 0x24);
    let a = mem_word(mem, n + 16);
    if a != 0 {
        let _ = mem_word(mem, vtab + 4);
        dtor(a, heap);
    }
    let b = mem_word(mem, n + 20);
    if b != 0 {
        let _ = mem_word(mem, vtab + 16);
        dtor(b, heap);
    }
    heap_release(mem, nz(heap), Some(n), vt);
}

/// fn_176fec / EU fn_17709c / KR fn_177048.
///
/// Fetches the database word for `key`, reporting it through `out`.
/// Lookup failure reports `0x81506014`; a missing entry reports 0
/// without touching `out`.
pub fn fetch_db_word(
    mem: &mut [u8],
    db: Option<u32>,
    key: u32,
    out: u32,
    cmp: impl FnMut(u32, u32) -> i32,
) -> u32 {
    let mut slot = 0u32;
    let rc = lookup_value(&*mem, db, key, Some(&mut slot), cmp);
    if rc >= 0x80000000 {
        return 0x81506014;
    }
    if slot != 0 {
        mem_store(mem, out, mem_word(mem, slot));
    }
    0
}

/// fn_195d30 / EU fn_195de0 / KR fn_195e1c.
///
/// Reports the offset of the first `ch` byte in the string at `s`,
/// or 0 when absent.
pub fn find_byte(mem: &[u8], s: u32, ch: u32) -> u32 {
    let c = ch as u8;
    let mut p = s as usize;
    while mem[p] != 0 && mem[p] != c {
        p += 1;
    }
    if mem[p] == c { p as u32 } else { 0 }
}

/// fn_195e38 / EU fn_195ee8 / KR fn_195f24.
///
/// Copies at most `n` bytes from `src` to `dst`, stopping after the
/// terminator and padding the remainder with zeros; reports `dst`.
pub fn copy_str_n(mem: &mut [u8], dst: u32, src: u32, n: u32) -> u32 {
    let mut d = dst as usize;
    let mut s = src as usize;
    let mut left = n;
    if left != 0 {
        loop {
            let c = mem[s];
            mem[d] = c;
            d += 1;
            s += 1;
            if c == 0 {
                break;
            }
            left -= 1;
            if left == 0 {
                return dst;
            }
        }
    } else {
        return dst;
    }
    if left > 0x7fffffff {
        left -= 1;
        while left != 0 {
            mem[d] = 0;
            d += 1;
            left -= 1;
        }
    } else {
        let mut i = left as i32 - 1;
        if i > 0 {
            while i != 0 {
                mem[d] = 0;
                d += 1;
                i -= 1;
            }
        }
    }
    dst
}

/// fn_1959dc / EU fn_195a8c / KR fn_195ac8.
///
/// Flags the live-handle error value: a zero `v` returns quietly,
/// otherwise the value lands in the flag byte under the object lock;
/// lock failure is traced through the host-owned diagnostic.
pub fn flag_lh_error(mem: &mut [u8], p: u32, v: u32, lock: &mut impl StateLock) {
    if v == 0 {
        return;
    }
    if lock.acquire() >= 0x80000000 {
        return;
    }
    mem[p as usize + 40] = v as u8;
    lock.release();
}

/// fn_196794 / EU fn_196844 / KR fn_196880.
///
/// Allocates a 12-byte handle for a tree hit: looks `a` up in the
/// table at `p` + 0x18 (exact search, or last-link scan when `a` is
/// 0), choosing the ordered or the hashed search by `b`. A null `p`
/// or `out` reports `0x89002007`, a miss `0x89002014`, method
/// failure `0x8900200a`.
pub fn alloc_handle_for_key(
    mem: &mut [u8],
    p: Option<u32>,
    a: u32,
    b: u32,
    out: Option<&mut u32>,
    vt: &mut impl MethodTable,
    mut cmp: impl FnMut(u32, u32) -> i32,
) -> u32 {
    let (Some(p), Some(out)) = (p, out) else {
        return 0x89002007;
    };
    let mut rc = 0u32;
    let v: u32;
    if a != 0 {
        if b != 0 {
            let tree = p.wrapping_add(0x18);
            v = find_node(
                &*mem,
                tree,
                mem_word(mem, tree),
                mem[tree as usize + 40],
                a,
                &mut cmp,
            )
            .unwrap_or(0);
        } else {
            v = search_node(&*mem, p.wrapping_add(0x18), a, &mut cmp);
        }
        if v == 0 {
            rc = 0x89002014;
        }
    } else {
        v = last_link(&*mem, p.wrapping_add(0x18));
    }
    if rc == 0 {
        let o = call_method(mem, nz(mem_word(mem, p + 0x14)), 1, 12, vt);
        *out = o;
        if o == 0 {
            return 0x8900200a;
        }
        mem_store(mem, o, p);
        mem_store(mem, o + 4, 0);
        mem_store(mem, o + 8, v);
    }
    rc
}

/// fn_1964e0 / EU fn_196590 / KR fn_1965cc.
///
/// Creates a 0x44-byte context object through the context's method,
/// stores the five words and links it to the context, initializing
/// the trailing sub-object (flagged when `flag` is 1). A null word
/// or `out` reports `0x89002007`; method failure reports
/// `0x8900200a`.
#[allow(clippy::too_many_arguments)]
pub fn create_ctx_object(
    mem: &mut [u8],
    ctx: Option<u32>,
    flag: u32,
    a: u32,
    b: u32,
    c: u32,
    d: u32,
    e: u32,
    out: Option<u32>,
    vt: &mut impl MethodTable,
) -> u32 {
    let Some(out) = out else {
        return 0x89002007;
    };
    if a == 0 || b == 0 || d == 0 || e == 0 || c == 0 {
        return 0x89002007;
    }
    let o = call_method(mem, ctx, 1, 0x44, vt);
    mem_store(mem, out, o);
    if o == 0 {
        return 0x8900200a;
    }
    mem_store(mem, o, a);
    mem_store(mem, o + 4, b);
    mem_store(mem, o + 8, c);
    mem_store(mem, o + 12, d);
    mem_store(mem, o + 16, e);
    mem_store(mem, o + 20, ctx.unwrap_or(0));
    init_link(mem, o.wrapping_add(24), 0xffffffff, o, ctx.unwrap_or(0));
    if flag == 1 {
        set_link_flag(mem, o.wrapping_add(24));
    }
    0
}

/// fn_19869c / EU fn_19874c / KR fn_198788.
///
/// Clears the tree at `t`: destroys both head subtrees, runs the
/// vtable pair hooks over the head's live words, releases the head,
/// then resets count, head and tail.
pub fn clear_tree(
    mem: &mut [u8],
    t: u32,
    vt: &mut impl HeapVtable,
    dtor: &mut impl FnMut(u32, u32),
) {
    let n = mem_word(mem, t);
    if n != t {
        destroy_subtree(&*mem, t, mem_word(mem, n), t, vt, dtor);
        destroy_subtree(&*mem, t, mem_word(mem, n + 4), t, vt, dtor);
        let heap = mem_word(mem, t + 32);
        let vtab = mem_word(mem, t + 36);
        let p = mem_word(mem, n + 16);
        if p != 0 {
            let _ = mem_word(mem, vtab + 4);
            dtor(p, heap);
        }
        let q = mem_word(mem, n + 20);
        if q != 0 {
            let _ = mem_word(mem, vtab + 16);
            dtor(q, heap);
        }
        heap_release(mem, nz(heap), Some(n), vt);
    }
    mem_store(mem, t + 24, 0);
    mem_store(mem, t, t);
    mem_store(mem, t + 4, t);
}

/// NUL-terminated string at `s`, terminator included. The caller
/// guarantees a terminator in bounds, matching the original's reads.
pub(crate) fn terminated(mem: &[u8], s: u32) -> &[u8] {
    let at = s as usize;
    let end = at + mem[at..].iter().position(|&c| c == 0).unwrap() + 1;
    &mem[at..end]
}

/// Shared `copyBrokerName` static behind `fn_1789b0`/`fn_17a3b8`:
/// duplicates `name` onto `heap`, reporting through `out`. A null
/// name reports `0x81506000` (leaving `*out` zeroed); allocation
/// failure reports `0x8150600a`. The original's null-`out` check is
/// dead: both callers pass a live word.
fn copy_heap_string(
    mem: &mut [u8],
    heap: Option<u32>,
    name: Option<u32>,
    out: &mut u32,
    vt: &mut impl HeapVtable,
) -> u32 {
    *out = 0;
    let Some(name) = name else {
        return 0x81506000;
    };
    let o = heap_alloc(mem, heap, strlen_bytes(&mem[name as usize..]) + 1, vt);
    *out = o;
    if o == 0 {
        return 0x8150600a;
    }
    copy_str(mem, o, name);
    0
}

/// `matchComponent` static behind `fn_178c24`: reports through
/// `matched` whether `key` satisfies the `/`-separated wildcard
/// `pattern`, allocating the segment scratch on `heap`.
fn match_wildcard(
    mem: &mut [u8],
    heap: Option<u32>,
    pattern: u32,
    key: u32,
    matched: &mut u8,
    vt: &mut impl HeapVtable,
) -> u32 {
    let copy = heap_alloc(mem, heap, strlen_bytes(&mem[pattern as usize..]) + 1, vt);
    if copy == 0 {
        return 0x8150600a;
    }
    *matched = 1;
    let mut cursor = pattern;
    let mut search = key;
    while strlen_bytes(&mem[pattern as usize..]) > cursor.wrapping_sub(pattern) {
        let star = find_byte(mem, cursor, '*' as u32);
        let length = if star != 0 {
            star.wrapping_sub(cursor)
        } else {
            strlen_bytes(&mem[cursor as usize..])
        };
        copy_str_n(mem, copy, cursor, length);
        mem[(copy + length) as usize] = 0;
        let found = find_substr(mem, search, copy);
        if found == 0 {
            *matched = 0;
            break;
        }
        cursor = cursor.wrapping_add(length).wrapping_add(1);
        search = found.wrapping_add(length);
        if length != 0
            && mem[(copy + length - 1) as usize] != b'/'
            && strlen_bytes(&mem[key as usize..]) != search.wrapping_sub(key)
        {
            *matched = 0;
            break;
        }
    }
    heap_release(mem, heap, nz(copy), vt);
    0
}

/// fn_1772b0 / EU fn_177360 / KR fn_17730c.
///
/// Looks `name` up in the state's database, stepping the result
/// handle while the folded comparison runs negative and reporting
/// the value on an exact hit. A null `p` or `name`, a handle miss,
/// a lock-release failure over success, or success without a value
/// all report `0x8150600d`; other failures pass through.
pub fn lookup_keyed_value(
    mem: &mut [u8],
    p: Option<u32>,
    name: Option<u32>,
    out: &mut u32,
    table: &[u16; 256],
    lock: &mut impl StateLock,
    hvt: &mut impl HeapVtable,
    mvt: &mut impl MethodTable,
    mut cmp: impl FnMut(u32, u32) -> i32,
) -> u32 {
    *out = 0;
    let (Some(p), Some(name)) = (p, name) else {
        return 0x8150600d;
    };
    let ctx = mem_word(mem, p + 4);
    let result = lock.acquire();
    if result >= 0x80000000 {
        return result;
    }
    let mut iterator = 0u32;
    let mut result = alloc_handle_for_key(
        mem,
        nz(mem_word(mem, ctx + 16)),
        name,
        1,
        Some(&mut iterator),
        &mut *mvt,
        &mut cmp,
    );
    if result < 0x80000000 {
        let mut key = 0u32;
        let mut value = 0u32;
        let mut status = step_iter(mem, nz(iterator), Some(&mut key), Some(&mut value));
        if status < 0x80000000 {
            loop {
                let order = compare_folded(table, terminated(mem, name), terminated(mem, key));
                if order == 0 {
                    *out = value;
                    break;
                }
                if order > 0 {
                    break;
                }
                status = step_iter(mem, nz(iterator), Some(&mut key), Some(&mut value));
                if status >= 0x80000000 {
                    break;
                }
            }
        }
        release_owner(mem, nz(iterator), &mut *hvt);
    } else if result & 0x1fff == 0x14 {
        result = 0x8150600d;
    }
    let status = lock.release();
    if status >= 0x80000000 {
        if result < 0x80000000 {
            result = status;
        } else {
            return result;
        }
    }
    if result < 0x80000000 && *out == 0 {
        return 0x8150600d;
    }
    result
}

/// fn_1785c8 / EU fn_178678 / KR fn_178624.
///
/// Queries the broker's `component` table for `key`, reporting the
/// word through `out`. Null inputs report `0x81506007`, a component
/// miss `0x8150600d`, a key miss `0x81506014`. `flags` selects the
/// host-owned diagnostic path only.
pub fn query_tag_word(
    mem: &mut [u8],
    broker: Option<u32>,
    component: Option<u32>,
    key: Option<u32>,
    flags: u32,
    out: Option<&mut u32>,
    table: &[u16; 256],
    lock: &mut impl StateLock,
    hvt: &mut impl HeapVtable,
    mvt: &mut impl MethodTable,
    mut cmp: impl FnMut(u32, u32) -> i32,
) -> u32 {
    let _ = flags;
    let (Some(broker), Some(component), Some(key), Some(out)) = (broker, component, key, out)
    else {
        return 0x81506007;
    };
    if mem_word(mem, broker + 4) == 0 {
        return 0x81506007;
    }
    *out = 0;
    let mut entry = 0u32;
    let rc = lookup_keyed_value(
        mem,
        Some(broker),
        Some(component),
        &mut entry,
        table,
        &mut *lock,
        &mut *hvt,
        &mut *mvt,
        &mut cmp,
    );
    if rc >= 0x80000000 {
        return 0x8150600d;
    }
    let mut value = 0u32;
    let rc = lookup_value(mem, nz(mem_word(mem, entry)), key, Some(&mut value), &mut cmp);
    if rc >= 0x80000000 {
        return 0x81506014;
    }
    if value != 0 {
        *out = mem_word(mem, value);
    }
    0
}

/// fn_1789b0 / EU fn_178a60 / KR fn_178a0c.
///
/// Opens a component-table iterator over `name`, reporting it
/// through `out`. Null inputs report `0x81506007`, allocation
/// failure `0x8150600a`; a key miss opens an empty iterator.
/// Failures after allocation close and clear the iterator.
pub fn open_component_iter(
    mem: &mut [u8],
    broker: Option<u32>,
    name: Option<u32>,
    out: Option<&mut u32>,
    lock: &mut impl StateLock,
    hvt: &mut impl HeapVtable,
    mvt: &mut impl MethodTable,
    mut cmp: impl FnMut(u32, u32) -> i32,
) -> u32 {
    let (Some(broker), Some(name), Some(out)) = (broker, name, out) else {
        return 0x81506007;
    };
    let context = mem_word(mem, broker + 4);
    if context == 0 {
        return 0x81506007;
    }
    let runtime = mem_word(mem, context + 8);
    let heap = nz(mem_word(mem, runtime + 4));
    let o = call_method(mem, heap, 1, 36, &mut *mvt);
    *out = o;
    if o == 0 {
        return 0x8150600a;
    }
    mem_store(mem, o, runtime);
    mem_store(mem, o + 4, broker);
    let mut oname = 0u32;
    let mut rc = copy_heap_string(mem, heap, Some(name), &mut oname, &mut *hvt);
    mem_store(mem, o + 8, oname);
    if rc < 0x80000000 {
        rc = lock.acquire();
        if rc < 0x80000000 {
            let mut it = mem_word(mem, o + 12);
            rc = alloc_handle_for_key(
                mem,
                nz(mem_word(mem, context + 16)),
                name,
                0,
                Some(&mut it),
                &mut *mvt,
                &mut cmp,
            );
            mem_store(mem, o + 12, it);
            if rc >= 0x80000000 && rc & 0x1fff == 0x14 {
                rc = 0;
            }
        }
        let status = lock.release();
        if status >= 0x80000000 && rc < 0x80000000 {
            rc = status;
        }
    }
    if rc >= 0x80000000 {
        let obj = *out;
        let ictx = mem_word(mem, obj);
        let key = mem_word(mem, obj + 8);
        if key != 0 {
            heap_release(mem, nz(mem_word(mem, ictx + 4)), nz(key), &mut *hvt);
            mem_store(mem, obj + 8, 0);
        }
        if mem_word(mem, obj + 12) != 0 {
            release_owner(mem, nz(mem_word(mem, obj + 12)), &mut *hvt);
        }
        heap_release(mem, nz(mem_word(mem, ictx + 4)), nz(obj), &mut *hvt);
        *out = 0;
    }
    rc
}

/// fn_178c24 / EU fn_178cd4 / KR fn_178c80.
///
/// Steps the component iterator to the next entry matching its
/// pattern, reporting the key and (when wanted) the attribute
/// block. A null iterator reports `0x81506007`; a pattern overrun
/// reports `0x81506017`. Attribute lookups use rodata literal keys,
/// which compare greater than any numeric node key on both sides, so the
/// row's comparator reports constant greater and only its call
/// count is observed.
pub fn next_component_entry(
    mem: &mut [u8],
    iterator: Option<u32>,
    mut key_out: Option<&mut u32>,
    mut attrs_out: Option<&mut u32>,
    table: &[u16; 256],
    hvt: &mut impl HeapVtable,
    mut cmp: impl FnMut(u32, u32) -> i32,
) -> u32 {
    const ATTR_KEY: u32 = 0x100000;
    let Some(iterator) = iterator else {
        return 0x81506007;
    };
    if let Some(k) = key_out.as_mut() {
        **k = 0;
    }
    if let Some(a) = attrs_out.as_mut() {
        **a = 0;
    }
    let runtime = mem_word(mem, iterator);
    let heap = nz(mem_word(mem, runtime + 4));
    let pattern = mem_word(mem, iterator + 8);
    let star = find_byte(mem, pattern, '*' as u32);
    let prefix = if star != 0 {
        star.wrapping_sub(pattern)
    } else {
        strlen_bytes(&mem[pattern as usize..])
    };
    let mut key = 0u32;
    let mut entry = 0u32;
    let mut rc = step_iter(
        mem,
        nz(mem_word(mem, iterator + 12)),
        Some(&mut key),
        Some(&mut entry),
    );
    while rc < 0x80000000 {
        if compare_folded_bounded(
            table,
            &mem[pattern as usize..],
            &mem[key as usize..],
            prefix as i32,
        ) < 0
        {
            rc = 0x81506017;
            break;
        }
        let mut matched = 0u8;
        rc = match_wildcard(mem, heap, pattern, key, &mut matched, &mut *hvt);
        if rc >= 0x80000000 {
            break;
        }
        if matched == 1 {
            if let Some(k) = key_out.as_mut() {
                **k = key;
            }
            if attrs_out.is_some() {
                mem_store(mem, iterator + 20, 0);
                mem_store(mem, iterator + 24, 0);
                mem_store(mem, iterator + 28, 0);
                mem_store(mem, iterator + 32, 0);
                mem_store(mem, iterator + 16, key);
                let base = mem_word(mem, entry);
                for off in [20, 28, 24] {
                    let mut attr = 0u32;
                    rc = lookup_value(mem, nz(base), ATTR_KEY, Some(&mut attr), &mut cmp);
                    if rc >= 0x80000000 {
                        rc = 0x81506014;
                    } else {
                        rc = 0;
                        if attr != 0 {
                            mem_store(mem, iterator + off, mem_word(mem, attr));
                        }
                    }
                    if rc >= 0x80000000 && rc != 0x81506014 {
                        break;
                    }
                }
                if rc >= 0x80000000 && rc != 0x81506014 {
                    break;
                }
                if let Some(a) = attrs_out.as_mut() {
                    **a = iterator + 16;
                }
                rc = 0;
            }
            break;
        }
        rc = step_iter(
            mem,
            nz(mem_word(mem, iterator + 12)),
            Some(&mut key),
            Some(&mut entry),
        );
    }
    rc
}

/// fn_17a3b8 / EU fn_17a468 / KR fn_17a414.
///
/// Opens a tag iterator over `component`/`query`, reporting it
/// through `out`. Null or empty inputs report `0x81506007`, lookup
/// failures `0x8150600d`, allocation failure `0x8150600a`. Failures
/// after allocation close and clear the iterator. `segment` lends
/// 128 bytes for the query-segment scratch.
pub fn open_tag_iter(
    mem: &mut [u8],
    broker: Option<u32>,
    component: Option<u32>,
    query: Option<u32>,
    flags: u32,
    out: Option<&mut u32>,
    segment: u32,
    table: &[u16; 256],
    lock: &mut impl StateLock,
    hvt: &mut impl HeapVtable,
    mvt: &mut impl MethodTable,
    mut cmp: impl FnMut(u32, u32) -> i32,
) -> u32 {
    let _ = flags;
    let (Some(broker), Some(component), Some(query), Some(out)) = (broker, component, query, out)
    else {
        return 0x81506007;
    };
    if mem[component as usize] == 0 || mem[query as usize] == 0 {
        return 0x81506007;
    }
    let context = mem_word(mem, broker + 4);
    let runtime = mem_word(mem, context + 8);
    *out = 0;
    let mut entry = 0u32;
    let mut rc = lookup_keyed_value(
        mem,
        Some(broker),
        Some(component),
        &mut entry,
        table,
        &mut *lock,
        &mut *hvt,
        &mut *mvt,
        &mut cmp,
    );
    let mut database = 0u32;
    if (if rc < 0x80000000 { entry } else { 0 }) == 0 && rc & 0x1fff == 0x0d {
        rc = 0x8150600d;
    } else if rc < 0x80000000 {
        database = mem_word(mem, entry);
    }
    if rc >= 0x80000000 {
        return failed_tag_iter(mem, &mut *out, &mut *hvt, rc);
    }
    let mut cursor = query;
    let mut node = 0u32;
    loop {
        let slash = find_byte(mem, cursor, '/' as u32);
        if slash == 0 && compare_str(terminated(mem, cursor), b"*\0") == 0 {
            break;
        }
        if node != 0 {
            database = mem_word(mem, node + 8);
        }
        if database == 0 {
            rc = 0x8150600d;
            return failed_tag_iter(mem, &mut *out, &mut *hvt, rc);
        }
        if slash != 0 && slash.wrapping_sub(cursor) < 128 {
            let length = slash.wrapping_sub(cursor);
            copy_str_n(mem, segment, cursor, length);
            mem[(segment + length) as usize] = 0;
            let mut found = 0u32;
            rc = lookup_value(mem, nz(database), segment, Some(&mut found), &mut cmp);
            node = found;
            if rc >= 0x80000000 {
                rc = 0x8150600d;
                return failed_tag_iter(mem, &mut *out, &mut *hvt, rc);
            }
            rc = 0;
        } else {
            let mut found = 0u32;
            rc = lookup_value(mem, nz(database), cursor, Some(&mut found), &mut cmp);
            node = found;
            if rc >= 0x80000000 {
                rc = 0x8150600d;
                return failed_tag_iter(mem, &mut *out, &mut *hvt, rc);
            }
            rc = 0;
            if slash == 0 {
                break;
            }
        }
        cursor = slash.wrapping_add(1);
    }
    if rc >= 0x80000000 {
        return failed_tag_iter(mem, &mut *out, &mut *hvt, rc);
    }
    let heap = nz(mem_word(mem, runtime + 4));
    let o = call_method(mem, heap, 1, 12, &mut *mvt);
    *out = o;
    if o == 0 {
        return 0x8150600a;
    }
    if compare_str(terminated(mem, cursor), b"*\0") == 0 {
        if node != 0 {
            database = mem_word(mem, node + 8);
        }
        let mut it = mem_word(mem, o + 8);
        rc = alloc_handle_for_key(
            mem,
            nz(database),
            0,
            1,
            Some(&mut it),
            &mut *mvt,
            &mut cmp,
        );
        mem_store(mem, o + 8, it);
    } else {
        let mut it = mem_word(mem, o + 8);
        rc = alloc_handle_for_key(
            mem,
            nz(database),
            cursor,
            1,
            Some(&mut it),
            &mut *mvt,
            &mut cmp,
        );
        mem_store(mem, o + 8, it);
        if rc >= 0x80000000 {
            return failed_tag_iter(mem, &mut *out, &mut *hvt, rc);
        }
        let mut oname = mem_word(mem, o + 4);
        rc = copy_heap_string(mem, heap, Some(cursor), &mut oname, &mut *hvt);
        mem_store(mem, o + 4, oname);
    }
    mem_store(mem, o, runtime);
    if rc < 0x80000000 {
        return rc;
    }
    failed_tag_iter(mem, &mut *out, &mut *hvt, rc)
}

/// Shared `failed` tail of [`open_tag_iter`]: closes a live
/// iterator, clears the caller's word, and reports `rc`. The
/// original's flag-selected diagnostic is host-owned.
fn failed_tag_iter(
    mem: &mut [u8],
    out: &mut u32,
    vt: &mut impl HeapVtable,
    rc: u32,
) -> u32 {
    let iterator = *out;
    if iterator != 0 {
        let owner = mem_word(mem, iterator);
        if mem_word(mem, iterator + 4) != 0 {
            heap_release(
                mem,
                nz(mem_word(mem, owner + 4)),
                nz(mem_word(mem, iterator + 4)),
                &mut *vt,
            );
        }
        if mem_word(mem, iterator + 8) != 0 {
            release_owner(mem, nz(mem_word(mem, iterator + 8)), &mut *vt);
        }
        heap_release(
            mem,
            nz(mem_word(mem, owner + 4)),
            nz(iterator),
            &mut *vt,
        );
    }
    *out = 0;
    rc
}

/// fn_192e5c / EU fn_192f0c / KR fn_192f48.
///
/// Resolves the voice matching the previous resolution, the voice
/// name, or the voice id, scanning the pipeline components'
/// parameter tags. Null `resolved`/`voice_id` reports `0x80000006`;
/// no match reports `0x80000014`; allocation failure `0x8000000a`.
/// `pipeline_query`/`params_query` locate the fixture copies of the
/// query literals; `segment` lends `open_tag_iter` its scratch.
/// `cmp` serves arena-addressed keys while `lcmp` serves the
/// rodata-literal query: literal addresses compare less than any
/// arena node on both sides, so that comparator reports constant
/// less (only its call count is observed).
#[allow(clippy::too_many_arguments)]
pub fn resolve_voice(
    mem: &mut [u8],
    manager: u32,
    resolved: Option<&mut u32>,
    voice_id: Option<&mut u16>,
    voice_name: u32,
    pipeline_query: u32,
    params_query: u32,
    segment: u32,
    table: &[u16; 256],
    lock: &mut impl StateLock,
    hvt: &mut impl HeapVtable,
    mvt: &mut impl MethodTable,
    mut cmp: impl FnMut(u32, u32) -> i32,
    lcmp: impl FnMut(u32, u32) -> i32,
) -> u32 {
    let (Some(resolved), Some(voice_id)) = (resolved, voice_id) else {
        return 0x80000006;
    };
    let registry = mem_word(mem, manager + 12);
    let allocator = nz(mem_word(mem, manager + 4));
    let mut reg_it = 0u32;
    let result = open_component_iter(
        mem,
        nz(registry),
        Some(pipeline_query),
        Some(&mut reg_it),
        &mut *lock,
        &mut *hvt,
        &mut *mvt,
        lcmp,
    );
    if result >= 0x80000000 {
        return result;
    }
    let mut result = 0x80000014u32;
    let mut item = 0u32;
    let mut attributes = 0u32;
    let mut rc = next_component_entry(
        mem,
        nz(reg_it),
        Some(&mut item),
        None,
        table,
        &mut *hvt,
        &mut cmp,
    );
    if rc < 0x80000000 {
        loop {
            let st = open_tag_iter(
                mem,
                nz(registry),
                Some(item),
                Some(params_query),
                1,
                Some(&mut attributes),
                segment,
                table,
                &mut *lock,
                &mut *hvt,
                &mut *mvt,
                &mut cmp,
            );
            if st < 0x80000000 {
                let mut language = 0u32;
                let mut language_code = 0u32;
                let mut candidate_id = 0u16;
                let mut aname = 0u32;
                let mut avalue = 0u32;
                loop {
                    let st = next_tag_entry(
                        mem,
                        nz(attributes),
                        Some(&mut aname),
                        Some(&mut avalue),
                        None,
                    );
                    if st >= 0x80000000 {
                        break;
                    }
                    if aname != 0 {
                        if compare_str(terminated(mem, aname), b"language\0") == 0 {
                            language = avalue;
                        } else if compare_str(terminated(mem, aname), b"langcode\0") == 0 {
                            language_code = avalue;
                        } else if compare_str(terminated(mem, aname), b"langid\0") == 0 {
                            candidate_id = parse_uint(mem, nz(avalue)) as u16;
                        }
                    }
                }
                close_tag_iter(mem, nz(attributes), &mut *hvt);
                attributes = 0;
                if language != 0 && language_code != 0 {
                    if candidate_id == 0 {
                        candidate_id = parse_uint(mem, nz(language)) as u16;
                    }
                    if *resolved != 0 {
                        if compare_folded(
                            table,
                            terminated(mem, language),
                            terminated(mem, *resolved),
                        ) == 0
                        {
                            copy_str(mem, voice_name, language_code);
                            *voice_id = candidate_id;
                            result = 0;
                            break;
                        }
                    } else if *voice_id == 0 && mem[voice_name as usize] != 0 {
                        if compare_folded(
                            table,
                            terminated(mem, language_code),
                            terminated(mem, voice_name),
                        ) == 0
                        {
                            let copy = heap_alloc(
                                mem,
                                allocator,
                                strlen_bytes(&mem[language as usize..]) + 1,
                                &mut *hvt,
                            );
                            *resolved = copy;
                            if copy == 0 {
                                result = 0x8000000a;
                                break;
                            }
                            copy_str(mem, copy, language);
                            *voice_id = candidate_id;
                            result = 0;
                            break;
                        }
                    } else if *voice_id == candidate_id {
                        copy_str(mem, voice_name, language_code);
                        let copy = heap_alloc(
                            mem,
                            allocator,
                            strlen_bytes(&mem[language as usize..]) + 1,
                            &mut *hvt,
                        );
                        *resolved = copy;
                        if copy == 0 {
                            result = 0x8000000a;
                            break;
                        }
                        copy_str(mem, copy, language);
                        result = 0;
                        break;
                    }
                }
            }
            rc = next_component_entry(
                mem,
                nz(reg_it),
                Some(&mut item),
                None,
                table,
                &mut *hvt,
                &mut cmp,
            );
            if rc >= 0x80000000 {
                break;
            }
        }
    }
    close_component_iter(mem, nz(reg_it), &mut *hvt);
    result
}

/// fn_15db94 / EU fn_15dc58 / KR fn_15db78.
///
/// Matches the voice record at `rec` against the query dictionary in
/// `hdr` (`iter` at +12, `dict` at +14): a `xx-yy` record is first
/// relocated, then the nine query words are fetched and compared
/// against the record fields directly; on any mismatch the pipeline
/// components are enumerated for the best scoring entry and the
/// winning tags fill the record. Reports 0, `0x8950200d` when no
/// pipeline matches, or the first unfiltered lookup error. `dict_keys`
/// locates the nine query strings (language, langcode, langietf,
/// langgroup, voice, gender, age, frequencyhz, voicemodel);
/// `pipeline`, `priority` and `params_query` locate
/// `"pipeline/*"`, `"PRIORITY"` and `"PARAMETERS/*"`; `es_us`/`es_mx`
/// locate the langietf exception tags. `segment` lends
/// `open_tag_iter` its 128-byte scratch; `scratch` lends 12 bytes
/// (uint out, query langgroup, candidate langgroup) emulating the
/// stack words the C aliases across loop iterations. `flags` is
/// ignored. Null `hdr`/`rec` is a caller bug (the oracles crash
/// there too).
#[allow(clippy::too_many_arguments)]
pub fn match_voice(
    mem: &mut [u8],
    hdr: u32,
    rec: u32,
    flags: u32,
    dict_keys: &[u32; 9],
    pipeline: u32,
    priority: u32,
    params_query: u32,
    es_us: u32,
    es_mx: u32,
    segment: u32,
    scratch: u32,
    table: &[u16; 256],
    lock: &mut impl StateLock,
    hvt: &mut impl HeapVtable,
    mvt: &mut impl MethodTable,
    mut cmp: impl FnMut(u32, u32) -> i32,
) -> u32 {
    let _ = flags;
    let iter = mem_word(mem, hdr + 12);
    let dict = mem_word(mem, hdr + 20);
    let mut qv = [0u32; 11];
    let mut cur = [0u32; 11];
    let mut best = [0u32; 11];
    if strlen_bytes(&mem[rec as usize..]) == 5
        && char_class(table, mem[rec as usize] as u32) != 0
        && char_class(table, mem[rec as usize + 1] as u32) != 0
        && mem[rec as usize + 2] == b'-'
        && char_class(table, mem[rec as usize + 3] as u32) != 0
        && char_class(table, mem[rec as usize + 4] as u32) != 0
    {
        copy_str(mem, rec + 0x48, rec);
        fill_bytes(mem, rec, 0, 0x40);
    }
    for (index, (slot, key)) in qv.iter_mut().take(9).zip(dict_keys.iter()).enumerate() {
        let rc = if index == 7 {
            let rc = get_param_uint(mem, nz(dict), *key, scratch, &mut *lock, &mut cmp);
            *slot = mem_word(mem, scratch);
            rc
        } else {
            fetch_param(mem, nz(dict), *key, Some(slot), &mut *lock, &mut cmp)
        };
        if dict != 0 && rc >= 0x80000000 && rc.wrapping_shl(19) != 0xa00000 {
            return rc;
        }
    }
    derive_langgroup(mem, table, &mut qv, scratch + 4);
    if rec_field_matches(mem, table, rec, &qv, es_us, es_mx) {
        best = qv;
    } else {
        let mut params = 0u32;
        let rc = open_component_iter(
            mem,
            nz(iter),
            Some(pipeline),
            Some(&mut params),
            &mut *lock,
            &mut *hvt,
            &mut *mvt,
            &mut cmp,
        );
        if rc >= 0x80000000 {
            return rc;
        }
        let mut best_flag = 0u32;
        let mut best_score = 0u32;
        let mut best_pri = 0u32;
        let mut key = 0u32;
        while next_component_entry(
            mem,
            nz(params),
            Some(&mut key),
            None,
            table,
            &mut *hvt,
            &mut cmp,
        ) < 0x80000000
        {
            cur = [0u32; 11];
            let mut it = 0u32;
            let rc = open_tag_iter(
                mem,
                nz(iter),
                nz(key),
                Some(params_query),
                1,
                Some(&mut it),
                segment,
                table,
                &mut *lock,
                &mut *hvt,
                &mut *mvt,
                &mut cmp,
            );
            if rc >= 0x80000000 {
                continue;
            }
            let mut name = 0u32;
            let mut val = 0u32;
            loop {
                if next_tag_entry(mem, nz(it), Some(&mut name), Some(&mut val), None)
                    >= 0x80000000
                {
                    break;
                }
                if name == 0 {
                    continue;
                }
                let tag = terminated(mem, name);
                if compare_str(tag, b"language\0") == 0 {
                    cur[0] = val;
                } else if compare_str(tag, b"langcode\0") == 0 {
                    cur[1] = val;
                } else if compare_str(tag, b"langietf\0") == 0 {
                    cur[2] = val;
                } else if compare_str(tag, b"langgroup\0") == 0 {
                    cur[3] = val;
                } else if compare_str(tag, b"voice\0") == 0 {
                    cur[4] = val;
                } else if compare_str(tag, b"gender\0") == 0 {
                    cur[5] = val;
                } else if compare_str(tag, b"age\0") == 0 {
                    cur[6] = val;
                } else if compare_str(tag, b"frequencyhz\0") == 0 {
                    cur[7] = parse_uint(mem, nz(val)) as u32;
                } else if compare_str(tag, b"voicemodel\0") == 0 {
                    cur[8] = val;
                }
            }
            close_tag_iter(mem, nz(it), &mut *hvt);
            derive_langgroup_candidate(mem, table, &mut cur, scratch + 8);
            if !candidate_matches(mem, table, rec, &cur, es_us, es_mx) {
                continue;
            }
            let score = score_candidate(mem, table, &qv, &cur);
            if best_flag != 0 && best_score > score {
                continue;
            }
            let mut bval = 0u32;
            let mut pri = 0u32;
            let rc = query_tag_word(
                mem,
                nz(iter),
                nz(key),
                Some(priority),
                1,
                Some(&mut bval),
                table,
                &mut *lock,
                &mut *hvt,
                &mut *mvt,
                &mut cmp,
            );
            if rc < 0x80000000 && bval != 0 {
                pri = parse_uint(mem, nz(bval)) as u32 & 0xffff;
                release_tag(nz(iter), nz(bval));
            }
            if best_flag == 0 || score > best_score || (pri as u16) > (best_pri as u16) {
                best = cur;
                best_flag = 1;
                best_score = score;
                best_pri = pri;
            }
        }
        close_component_iter(mem, nz(params), &mut *hvt);
        if best_flag == 0 {
            return 0x8950200d;
        }
    }
    copy_str_or_empty(mem, rec, best[0]);
    copy_str_or_empty(mem, rec + 0x40, best[1]);
    copy_str_or_empty(mem, rec + 0x48, best[2]);
    copy_str_or_empty(mem, rec + 0x120, best[3]);
    copy_str_or_empty(mem, rec + 0x88, best[4]);
    copy_str_or_empty(mem, rec + 0xa8, best[5]);
    copy_str_or_empty(mem, rec + 0x114, best[6]);
    mem_store(mem, rec + 0xb8, best[7]);
    copy_str_or_empty(mem, rec + 0xbc, best[8]);
    0
}

/// Derives the `langgroup` word of a query vector: a 3-letter
/// `langcode` folds to `xx` plus `x` in the scratch word.
pub(crate) fn derive_langgroup(mem: &mut [u8], table: &[u16; 256], qv: &mut [u32; 11], np: u32) {
    if qv[1] != 0 && strlen_bytes(&mem[qv[1] as usize..]) == 3 {
        let lc = qv[1] as usize;
        qv[3] = np;
        mem[np as usize] = ascii_fold(table, mem[lc] as u32) as u8;
        mem[np as usize + 1] = ascii_fold(table, mem[lc + 1] as u32) as u8;
        mem[np as usize + 2] = b'x';
        mem[np as usize + 3] = 0;
    }
}

/// Derives the `langgroup` word of a pipeline candidate, keeping an
/// explicit tag when one matched.
pub(crate) fn derive_langgroup_candidate(
    mem: &mut [u8],
    table: &[u16; 256],
    cur: &mut [u32; 11],
    np: u32,
) {
    if cur[3] == 0 && cur[1] != 0 && strlen_bytes(&mem[cur[1] as usize..]) == 3 {
        let lc = cur[1] as usize;
        cur[3] = np;
        mem[np as usize] = ascii_fold(table, mem[lc] as u32) as u8;
        mem[np as usize + 1] = ascii_fold(table, mem[lc + 1] as u32) as u8;
        mem[np as usize + 2] = b'x';
        mem[np as usize + 3] = 0;
    }
}

/// Reports whether the record fields all match the query vector
/// (null-tolerant compares plus the langietf 2-letter and es-us/es-mx
/// exceptions).
pub(crate) fn rec_field_matches(
    mem: &[u8],
    table: &[u16; 256],
    rec: u32,
    qv: &[u32; 11],
    es_us: u32,
    es_mx: u32,
) -> bool {
    if mem[rec as usize] != 0 && compare_nullable(table, mem, Some(rec), nz(qv[0])) != 0 {
        return false;
    }
    if mem[rec as usize + 0x40] != 0
        && compare_nullable(table, mem, Some(rec + 0x40), nz(qv[1])) != 0
    {
        return false;
    }
    if mem[rec as usize + 0x48] != 0 {
        let rl = rec + 0x48;
        let qw = qv[2];
        if qw == 0 {
            return false;
        }
        if !langietf_matches(mem, table, rl, qw, es_us, es_mx) {
            return false;
        }
    }
    if mem[rec as usize + 0x120] != 0
        && compare_nullable(table, mem, Some(rec + 0x120), nz(qv[3])) != 0
    {
        return false;
    }
    if mem[rec as usize + 0x88] != 0
        && compare_nullable(table, mem, Some(rec + 0x88), nz(qv[4])) != 0
    {
        return false;
    }
    if mem[rec as usize + 0xa8] != 0
        && compare_nullable(table, mem, Some(rec + 0xa8), nz(qv[5])) != 0
    {
        return false;
    }
    if mem[rec as usize + 0x114] != 0
        && compare_nullable(table, mem, Some(rec + 0x114), nz(qv[6])) != 0
    {
        return false;
    }
    if mem_word(mem, rec + 0xb8) != 0 && mem_word(mem, rec + 0xb8) != qv[7] {
        return false;
    }
    if mem[rec as usize + 0xbc] != 0
        && compare_nullable(table, mem, Some(rec + 0xbc), nz(qv[8])) != 0
    {
        return false;
    }
    true
}

/// Reports whether the record fields all match a pipeline candidate
/// (folded compares; a missing candidate word fails a set record
/// field).
pub(crate) fn candidate_matches(
    mem: &[u8],
    table: &[u16; 256],
    rec: u32,
    cur: &[u32; 11],
    es_us: u32,
    es_mx: u32,
) -> bool {
    if mem[rec as usize] != 0 {
        if cur[0] == 0
            || compare_folded(table, terminated(mem, rec), terminated(mem, cur[0])) != 0
        {
            return false;
        }
    }
    if mem[rec as usize + 0x40] != 0 {
        if cur[1] == 0
            || compare_folded(table, terminated(mem, rec + 0x40), terminated(mem, cur[1])) != 0
        {
            return false;
        }
    }
    if mem[rec as usize + 0x48] != 0 {
        if cur[2] == 0 || !langietf_matches(mem, table, rec + 0x48, cur[2], es_us, es_mx) {
            return false;
        }
    }
    if mem[rec as usize + 0x120] != 0 {
        if cur[3] == 0
            || compare_folded(table, terminated(mem, rec + 0x120), terminated(mem, cur[3])) != 0
        {
            return false;
        }
    }
    if mem[rec as usize + 0x88] != 0 {
        if cur[4] == 0
            || compare_folded(table, terminated(mem, rec + 0x88), terminated(mem, cur[4])) != 0
        {
            return false;
        }
    }
    if mem[rec as usize + 0xa8] != 0 {
        if cur[5] == 0
            || compare_folded(table, terminated(mem, rec + 0xa8), terminated(mem, cur[5])) != 0
        {
            return false;
        }
    }
    if mem[rec as usize + 0x114] != 0 {
        if cur[6] == 0
            || compare_folded(table, terminated(mem, rec + 0x114), terminated(mem, cur[6])) != 0
        {
            return false;
        }
    }
    if mem_word(mem, rec + 0xb8) != 0 && mem_word(mem, rec + 0xb8) != cur[7] {
        return false;
    }
    if mem[rec as usize + 0xbc] != 0 {
        if cur[8] == 0
            || compare_folded(table, terminated(mem, rec + 0xbc), terminated(mem, cur[8])) != 0
        {
            return false;
        }
    }
    true
}

/// The shared langietf matcher: 2-letter prefix rules, else a folded
/// compare with the es-us/es-mx exceptions.
fn langietf_matches(
    mem: &[u8],
    table: &[u16; 256],
    rl: u32,
    tw: u32,
    es_us: u32,
    es_mx: u32,
) -> bool {
    if strlen_bytes(&mem[rl as usize..]) == 2 || strlen_bytes(&mem[tw as usize..]) == 2 {
        if strlen_bytes(&mem[rl as usize..]) == 2 {
            if compare_folded_bounded(table, terminated(mem, rl), terminated(mem, tw), 2) != 0 {
                return false;
            }
            if mem[tw as usize + 2] != b'-' && mem[tw as usize + 2] != 0 {
                return false;
            }
        } else {
            if compare_folded_bounded(table, terminated(mem, rl), terminated(mem, tw), 2) != 0 {
                return false;
            }
            if mem[tw as usize + 1] != b'-' && mem[tw as usize + 1] != 0
                && compare_folded(table, terminated(mem, rl), terminated(mem, tw)) != 0
            {
                return langietf_exception(mem, table, rl, tw, es_us, es_mx);
            }
        }
    } else if compare_folded(table, terminated(mem, rl), terminated(mem, tw)) != 0 {
        return langietf_exception(mem, table, rl, tw, es_us, es_mx);
    }
    true
}

/// The es-us/es-mx exception tail of the langietf matcher.
fn langietf_exception(
    mem: &[u8],
    table: &[u16; 256],
    rl: u32,
    tw: u32,
    es_us: u32,
    es_mx: u32,
) -> bool {
    if compare_folded(table, terminated(mem, rl), terminated(mem, es_us)) == 0 {
        if compare_folded(table, terminated(mem, tw), terminated(mem, es_us)) != 0
            && compare_folded(table, terminated(mem, tw), terminated(mem, es_mx)) != 0
        {
            return false;
        }
    } else if compare_folded(table, terminated(mem, rl), terminated(mem, es_mx)) != 0 {
        return false;
    } else if compare_folded(table, terminated(mem, tw), terminated(mem, es_us)) != 0
        && compare_folded(table, terminated(mem, tw), terminated(mem, es_mx)) != 0
    {
        return false;
    }
    true
}

/// Scores a pipeline candidate against the query vector, bit by bit.
pub(crate) fn score_candidate(mem: &[u8], table: &[u16; 256], qv: &[u32; 11], cur: &[u32; 11]) -> u32 {
    let mut score = 0u32;
    let hit = |mem: &[u8], table: &[u16; 256], q: u32, c: u32| {
        q != 0 && mem[q as usize] != 0 && c != 0
            && compare_folded(table, terminated(mem, q), terminated(mem, c)) == 0
    };
    if hit(mem, table, qv[8], cur[8]) {
        score |= 4;
    }
    if qv[7] != 0 && qv[7] == cur[7] {
        score |= 16;
    }
    if hit(mem, table, qv[6], cur[6]) {
        score |= 32;
    }
    if hit(mem, table, qv[5], cur[5]) {
        score |= 64;
    }
    if hit(mem, table, qv[1], cur[1]) {
        score |= 128;
    }
    if hit(mem, table, qv[2], cur[2]) {
        score |= 256;
    }
    if hit(mem, table, qv[0], cur[0]) {
        score |= 512;
    }
    if hit(mem, table, qv[3], cur[3]) {
        score |= 1024;
    }
    if hit(mem, table, qv[4], cur[4]) {
        score |= 2048;
    }
    score
}

/// Copies a possibly-null word as a NUL-terminated string (null
/// reports the empty string).
pub(crate) fn copy_str_or_empty(mem: &mut [u8], dst: u32, src: u32) {
    if src == 0 {
        mem[dst as usize] = 0;
    } else {
        copy_str(mem, dst, src);
    }
}

/// fn_19611c / EU fn_1961cc / KR fn_196208.
///
/// Resolves the engine context for a voice handle: fast path when the
/// handle matches the cached pair, otherwise validates the tag and the
/// pointed-to word. The original falls back to a global default context
/// and reads a global pair; the caller supplies both (`default_ctx` is
/// the `&D_8d7640` token, `pair` the `D_8d7660` bytes). `p_addr` is the
/// handle pointer's address token (0 when null, otherwise the offset,
/// never 0) and `p_word` the word it points at.
pub fn resolve_engine_context(
    p_addr: u32,
    p_word: Option<u16>,
    tag: u32,
    pair: &[u8; 8],
    default_ctx: u32,
    out: &mut u32,
) -> u32 {
    let mut buf = [0u8; 8];
    load_pair(&mut buf, pair);
    let buf0 = u32::from_le_bytes([buf[0], buf[1], buf[2], buf[3]]);
    let buf1 = u32::from_le_bytes([buf[4], buf[5], buf[6], buf[7]]);
    if match_halves(p_addr, tag, buf0, buf1) != 0 {
        *out = default_ctx;
        return 0;
    }
    let r = validate_handle(p_word.is_some(), tag, 0x1e61, 32);
    if r >= 0x80000000 {
        let r = if r & 0x1fff == 8 { 0x81602008 } else { r };
        *out = default_ctx;
        return r;
    }
    *out = p_addr;
    if p_word.is_none() || p_word != Some(0x1e61) {
        *out = default_ctx;
        return 0x81602008;
    }
    r
}

/// fn_1900f8 / EU fn_1901a8 / KR fn_1901e4.
///
/// Configures a voice (mode 0) and assembles its pipeline specification
/// string. String literals live in caller memory (`lit_*`); `converted`
/// (0x128 bytes) and `freq_buf` (32 bytes) are caller scratch.
/// `configure` stands in for the voice method: it returns the status plus
/// the bytes the method leaves in the converted record. `query` stands in
/// for the broker-string lookup returning status and value; `notify`
/// observes the release call.
pub fn configure_voice(
    mem: &mut [u8],
    object: u32,
    mode: u32,
    voice: u32,
    converted: u32,
    freq_buf: u32,
    lit_pcm: u32,
    lit_pipeline: u32,
    lit_slash: u32,
    lit_format_pcm: u32,
    lit_phon: u32,
    pair: &[u8; 8],
    default_ctx: u32,
    configure: &mut impl FnMut(u32, u32, u32, u32) -> (u32, [u8; 0x128]),
    query: &mut impl FnMut(u32, u32) -> (u32, u32),
    notify: &mut impl FnMut(u32, u32),
) -> u32 {
    let spec = voice + 0x92;
    let engine = mem_word(mem, object + 8);
    let p_word = if engine == 0 {
        None
    } else {
        Some(mem_half(mem, engine))
    };
    let mut context = 0u32;
    let result = resolve_engine_context(
        engine,
        p_word,
        mem_word(mem, object + 12),
        pair,
        default_ctx,
        &mut context,
    );
    if result >= 0x80000000 {
        return result;
    }
    let resource = mem_word(mem, context + 0x0c);
    let synthesis = mem_word(mem, context + 0x14);
    if mode == 0 {
        fill_bytes(mem, converted, 0, 0x128);
        copy_str(mem, converted, voice);
        copy_str(mem, converted + 0x88, voice + 0x40);
        let freq = mem_half(mem, voice + 0x60);
        let converted_freq = match freq {
            11 => 0x2b11,
            22 => 0x5622,
            _ => u32::from(freq) * 1000,
        };
        mem_store(mem, converted + 0xb8, converted_freq);
        copy_str(mem, converted + 0xbc, voice + 0x62);
        // voice->format is an array, so it decays to a non-null pointer
        // and the C condition is always true; the "text" else is dead.
        copy_str(mem, converted + 0xf4, voice + 0x82);
        copy_str(mem, converted + 0x104, lit_pcm);
        let methods = mem_word(mem, object + 4);
        let (rc, script) = configure(
            mem_word(mem, methods + 0xcc),
            mem_word(mem, methods + 0xd0),
            resource,
            synthesis,
        );
        if rc >= 0x80000000 {
            return rc;
        }
        mem[converted as usize..converted as usize + 0x128].copy_from_slice(&script);
        copy_str(mem, voice, converted);
        copy_str(mem, voice + 0x40, converted + 0x88);
        let back = (mem_word(mem, converted + 0xb8) / 1000) as u16;
        mem[voice as usize + 0x60..voice as usize + 0x62]
            .copy_from_slice(&back.to_le_bytes());
        copy_str(mem, voice + 0x62, converted + 0xbc);
        copy_str(mem, voice + 0x82, converted + 0xf4);
    }
    let freq = mem_half(mem, voice + 0x60);
    let _ = render_unsigned(
        u32::from(freq),
        &mut mem[freq_buf as usize..freq_buf as usize + 32],
        10,
    );
    copy_str(mem, spec, lit_pipeline);
    append_str(mem, spec, voice);
    append_str(mem, spec, lit_slash);
    append_str(mem, spec, voice + 0x40);
    append_str(mem, spec, lit_slash);
    append_str(mem, spec, freq_buf);
    if mem[voice as usize + 0x62] != 0 {
        let v = &mem[voice as usize + 0x62..voice as usize + 0x67];
        let full = (v[0] == b'f' || v[0] == b'F')
            && (v[1] == b'u' || v[1] == b'U')
            && (v[2] == b'l' || v[2] == b'L')
            && (v[3] == b'l' || v[3] == b'L')
            && v[4] == 0;
        if !full {
            append_str(mem, spec, lit_slash);
            append_str(mem, spec, voice + 0x62);
        }
    }
    match mem[voice as usize + 0x112] {
        0 | 2 => {
            append_str(mem, spec, lit_slash);
            append_str(mem, spec, voice + 0x82);
            append_str(mem, spec, lit_format_pcm);
        }
        1 => {
            append_str(mem, spec, lit_phon);
        }
        _ => return 0x80002001,
    }
    let (qrc, value) = query(resource, spec);
    if qrc >= 0x80000000 {
        0x80002000
    } else {
        notify(resource, value);
        result
    }
}

/// fn_19310c / EU fn_1931bc / KR fn_1931f8.
///
/// Applies a parameter list (`vauto_ttsSetParamList`): validates the
/// object and tag, resolves the engine context, dispatches each parameter
/// to a numeric/string engine setting or a voice-field update, then
/// reconfigures the voice and hands the specification to the voice
/// method. `voice` (0x114 bytes) and `buffer` (128 bytes) are caller
/// scratch for the locals; `converted`/`freq_buf` and the `lit_*`
/// offsets serve the inner voice configuration as in `configure_voice`.
/// Engine settings go through `set_num`/`set_str`, voice resolution
/// through `resolve`/`release`, and the final hand-off through
/// `set_voice` (spec bytes or `None` for the detach call).
#[allow(clippy::too_many_arguments)]
pub fn set_param_list(
    mem: &mut [u8],
    handle: Option<u32>,
    tag: u32,
    params: Option<u32>,
    count: u32,
    voice: u32,
    buffer: u32,
    converted: u32,
    freq_buf: u32,
    lit_pcm: u32,
    lit_pipeline: u32,
    lit_slash: u32,
    lit_format_pcm: u32,
    lit_phon: u32,
    pair: &[u8; 8],
    default_ctx: u32,
    lock: &mut impl StateLock,
    configure: &mut impl FnMut(u32, u32, u32, u32) -> (u32, [u8; 0x128]),
    query: &mut impl FnMut(u32, u32) -> (u32, u32),
    notify: &mut impl FnMut(u32, u32),
    set_num: &mut impl FnMut(u32, &[u8], u32) -> u32,
    set_str: &mut impl FnMut(u32, &[u8], &[u8]) -> u32,
    resolve: &mut impl FnMut(u32, &[u8]) -> (u32, u32, u16),
    release: &mut impl FnMut(u32, u32),
    set_voice: &mut impl FnMut(u32, u32, Option<&[u8]>, u32) -> u32,
) -> u32 {
    let Some(object) = handle else {
        return 0x80000008;
    };
    if tag != 0x684f424a || mem_word(mem, object) != 0x734f424a {
        return 0x80000008;
    }
    let Some(params) = params else {
        return 0x80000006;
    };
    let engine_first = mem_word(mem, object + 8);
    let p_word = if engine_first == 0 {
        None
    } else {
        Some(mem_half(mem, engine_first))
    };
    let mut context = 0u32;
    let status = resolve_engine_context(
        engine_first,
        p_word,
        mem_word(mem, object + 12),
        pair,
        default_ctx,
        &mut context,
    );
    if status >= 0x80000000 {
        return classify_code(status);
    }
    let params_base = object + 0x36;
    {
        let (head, tail) = mem.split_at_mut(voice as usize);
        copy_bytes(
            &mut tail[..0x114],
            &head[params_base as usize..params_base as usize + 0x114],
            0x114,
        );
    }
    let mut input = mem[object as usize + 0x35];
    let mut mode = mem[object as usize + 0x34];
    let mut changed = 0u32;
    let mut frequency_set = 0u32;
    let mut variant_set = 0u32;
    let mut voice_id = 0u16;
    let mut resolved = 0u32;
    let mut acquired = 0u32;
    let mut locked = 0u8;
    let mut status = status;
    if !((mode & 8 != 0) && (mode & 4 != 0)) && (count > 1 || mem[params as usize] != 13) {
        return classify_code(0x80002011);
    }
    if mem[voice as usize + 0x92] == 0 {
        for i in 0..count {
            let id = mem[params as usize + i as usize * 0x102];
            if id == 1 || id == 7 || id == 15 || id == 8 || id == 19 {
                changed = 1;
                break;
            }
        }
        if changed == 0 {
            status = 0x80002011;
            return finish_param_list(
                mem, object, voice, mode, input, status, changed, acquired, locked, lock,
            );
        }
    }
    for i in 0..count {
        let id = mem[params as usize + i as usize * 0x102];
        if id != 3 && id != 4 && id != 2 && id != 5 {
            acquired = 1;
            status = enter_request(&mut mem[object as usize + 0x28], 2, &mut locked, lock);
            if status >= 0x80000000 {
                locked = 0;
                return finish_param_list(
                    mem, object, voice, mode, input, status, changed, acquired, locked, lock,
                );
            }
            break;
        }
    }
    let engine = mem_word(mem, context + 0x14);
    let manager = mem_word(mem, object + 0x10);
    let zero_key = [0u8; 4];
    for i in 0..count {
        let base = params + i * 0x102;
        let id = mem[base as usize];
        let number = mem_half(mem, base + 2);
        let text = base + 2;
        match id {
            3 => status = set_num(engine, b"rate", u32::from(number)),
            4 => status = set_num(engine, b"pitch", u32::from(number)),
            1 => {
                if number != 8 && number != 11 && number != 16 && number != 22 {
                    status = 0x80002007;
                    return finish_param_list(
                        mem, object, voice, mode, input, status, changed, acquired,
                        locked, lock,
                    );
                }
                changed = 1;
                mem[voice as usize + 0x60..voice as usize + 0x62]
                    .copy_from_slice(&number.to_le_bytes());
                frequency_set = 1;
            }
            12 => {
                if number == 0 {
                    status = set_num(engine, b"markermode", 0);
                } else if number == 1 {
                    status = set_num(engine, b"markermode", 0x4af);
                } else {
                    status = 0x80002007;
                    return finish_param_list(
                        mem, object, voice, mode, input, status, changed, acquired,
                        locked, lock,
                    );
                }
            }
            2 => status = set_num(engine, b"volume", u32::from(number)),
            5 => status = set_num(engine, b"waitfactor", u32::from(number)),
            6 => {
                status = set_num(engine, b"readmode", u32::from(number));
                if status >= 0x80000000 {
                    return finish_param_list(
                        mem, object, voice, mode, input, status, changed, acquired,
                        locked, lock,
                    );
                }
            }
            13 => {
                if number == 3 || number == 12 {
                    mode = number as u8;
                } else {
                    status = 0x8000200f;
                    return finish_param_list(
                        mem, object, voice, mode, input, status, changed, acquired,
                        locked, lock,
                    );
                }
            }
            14 => {
                if number == 1 || number == 2 {
                    input = number as u8;
                } else {
                    status = 0x8000200f;
                    return finish_param_list(
                        mem, object, voice, mode, input, status, changed, acquired,
                        locked, lock,
                    );
                }
            }
            7 => {
                copy_utf16_as_ascii(mem, Some(text), voice, 64);
                voice_id = 0;
                changed = 1;
                if strlen_bytes(&mem[voice as usize..]) == 3 {
                    let (rc, res, id) =
                        resolve(manager, &mem[voice as usize..voice as usize + 64]);
                    if rc < 0x80000000 {
                        resolved = res;
                        voice_id = id;
                        copy_str(mem, voice, resolved);
                        release(mem_word(mem, manager + 4), resolved);
                        resolved = 0;
                    }
                }
            }
            15 => {
                voice_id = number;
                let (rc, res, id) = resolve(manager, &zero_key);
                status = rc;
                if status >= 0x80000000 {
                    return finish_param_list(
                        mem, object, voice, mode, input, status, changed, acquired,
                        locked, lock,
                    );
                }
                resolved = res;
                voice_id = id;
                copy_str(mem, voice, resolved);
                release(mem_word(mem, manager + 4), resolved);
                resolved = 0;
                changed = 1;
                continue;
            }
            8 => {
                copy_utf16_as_ascii(mem, Some(text), voice + 0x40, 32);
                changed = 1;
            }
            9 => {
                copy_utf16_as_ascii(mem, Some(text), voice + 0x82, 16);
            }
            19 => {
                copy_utf16_as_ascii(mem, Some(text), voice + 0x62, 32);
                changed = 1;
                variant_set = 1;
            }
            16 => {
                if number > 1024 {
                    status = 0x8000200f;
                    return finish_param_list(
                        mem, object, voice, mode, input, status, changed, acquired,
                        locked, lock,
                    );
                }
                status = set_num(engine, b"promptlexsize", u32::from(number));
            }
            17 => status = set_num(engine, b"promptlexmsgs", u32::from(number)),
            18 => {
                if (i32::from(number).wrapping_sub(25) as u32) >= 0x9ac {
                    status = 0x8000200f;
                    return finish_param_list(
                        mem, object, voice, mode, input, status, changed, acquired,
                        locked, lock,
                    );
                }
                status = set_num(engine, b"maxinputlength", u32::from(number));
            }
            11 => {
                if number == 1 {
                    status = set_str(engine, b"inputmimetype", b"text/plain;charset=utf-16");
                } else if number == 2 {
                    status = set_str(engine, b"inputmimetype", b"text/plain;charset=utf-8");
                } else {
                    status = 0x8000200f;
                    return finish_param_list(
                        mem, object, voice, mode, input, status, changed, acquired,
                        locked, lock,
                    );
                }
            }
            20 => match number {
                0 => status = set_str(engine, b"lidscope", b"none"),
                1 => status = set_str(engine, b"lidscope", b"user-defined"),
                2 => status = set_str(engine, b"lidscope", b"message"),
                _ => {
                    status = 0x8000200f;
                    return finish_param_list(
                        mem, object, voice, mode, input, status, changed, acquired,
                        locked, lock,
                    );
                }
            },
            21 => {
                if number == 0 {
                    status = set_str(engine, b"lidvoiceswitch", b"no");
                } else if number == 1 {
                    status = set_str(engine, b"lidvoiceswitch", b"yes");
                } else {
                    status = 0x8000200f;
                    return finish_param_list(
                        mem, object, voice, mode, input, status, changed, acquired,
                        locked, lock,
                    );
                }
            }
            22 => {
                copy_utf16_as_ascii(mem, Some(text), buffer, 128);
                let end = nul_len(&mem[buffer as usize..buffer as usize + 128]);
                status = set_str(
                    engine,
                    b"extraesclang",
                    &mem[buffer as usize..buffer as usize + end],
                );
            }
            23 => {
                copy_utf16_as_ascii(mem, Some(text), buffer, 128);
                let end = nul_len(&mem[buffer as usize..buffer as usize + 128]);
                status = set_str(
                    engine,
                    b"extraesctn",
                    &mem[buffer as usize..buffer as usize + end],
                );
            }
            _ => {
                status = 0x80002007;
                return finish_param_list(
                    mem, object, voice, mode, input, status, changed, acquired, locked, lock,
                );
            }
        }
        if status >= 0x80000000 {
            return finish_param_list(
                mem, object, voice, mode, input, status, changed, acquired, locked, lock,
            );
        }
    }
    if changed != 0 {
        status = configure_voice(
            mem, object, 0, voice, converted, freq_buf, lit_pcm, lit_pipeline, lit_slash,
            lit_format_pcm, lit_phon, pair, default_ctx, configure, query, notify,
        );
        if status >= 0x80000000 {
            if mem[voice as usize] != 0 || mem[voice as usize + 0x40] != 0 {
                if frequency_set == 0 || variant_set == 0 {
                    if frequency_set == 0 {
                        mem[voice as usize + 0x60..voice as usize + 0x62]
                            .copy_from_slice(&0u16.to_le_bytes());
                    }
                    if variant_set == 0 {
                        mem[voice as usize + 0x62] = 0;
                    }
                    configure_voice(
                        mem, object, 0, voice, converted, freq_buf, lit_pcm, lit_pipeline,
                        lit_slash, lit_format_pcm, lit_phon, pair, default_ctx, configure,
                        query, notify,
                    );
                }
            }
            return finish_param_list(
                mem, object, voice, mode, input, status, changed, acquired, locked, lock,
            );
        }
    } else if status >= 0x80000000 {
        return finish_param_list(
            mem, object, voice, mode, input, status, changed, acquired, locked, lock,
        );
    }
    let mode_dual = (mode & 8 != 0) && (mode & 4 != 0);
    let object_dual = (mem[object as usize + 0x34] & 8 != 0) && (mem[object as usize + 0x34] & 4 != 0);
    if mode_dual != object_dual {
        if mode_dual {
            changed = 1;
        } else {
            status = set_voice(
                mem_word(mem, object + 0x2c),
                mem_word(mem, object + 0x30),
                None,
                0,
            );
            if status >= 0x80000000 {
                return finish_param_list(
                    mem, object, voice, mode, input, status, changed, acquired, locked, lock,
                );
            }
        }
    }
    if changed == 1 && mode_dual {
        let end = nul_len(&mem[voice as usize + 0x92..voice as usize + 0x92 + 128]);
        let spec = voice as usize + 0x92;
        status = set_voice(
            mem_word(mem, object + 0x2c),
            mem_word(mem, object + 0x30),
            Some(&mem[spec..spec + end]),
            0,
        );
        if status >= 0x80000000 {
            return finish_param_list(
                mem, object, voice, mode, input, status, changed, acquired, locked, lock,
            );
        }
    }
    let _ = resolved;
    let _ = voice_id;
    finish_param_list(
        mem, object, voice, mode, input, status, changed | 2, acquired, locked, lock,
    )
}

/// Tail of `set_param_list`: write the mode/input/voice back unless the
/// exit skipped it, release the request lock when held, and classify.
fn finish_param_list(
    mem: &mut [u8],
    object: u32,
    voice: u32,
    mode: u8,
    input: u8,
    status: u32,
    changed: u32,
    acquired: u32,
    locked: u8,
    lock: &mut impl StateLock,
) -> u32 {
    if changed & 2 != 0 {
        mem[object as usize + 0x34] = mode;
        mem[object as usize + 0x35] = input;
        if changed & 1 != 0 {
            let params_base = object + 0x36;
            let (head, tail) = mem.split_at_mut(voice as usize);
            copy_bytes(
                &mut head[params_base as usize..params_base as usize + 0x114],
                &tail[..0x114],
                0x114,
            );
        }
    }
    if acquired != 0 && locked != 0 {
        let state = mem[object as usize + 0x28];
        let mut state = state;
        restore_request(&mut state, u32::from(locked), lock);
        mem[object as usize + 0x28] = state;
    }
    classify_code(status)
}

/// Length of the NUL-terminated prefix, capped at the slice length.
fn nul_len(s: &[u8]) -> usize {
    let mut n = 0;
    while n < s.len() && s[n] != 0 {
        n += 1;
    }
    n
}

/// fn_19665c / EU fn_19670c / KR fn_196748.
///
/// Clears the owned subtree, then releases the object through its
/// owner heap. `clear` stands in for the tree teardown.
pub fn release_cleared(
    mem: &[u8],
    p: Option<u32>,
    clear: &mut impl FnMut(u32),
    vt: &mut impl HeapVtable,
) -> u32 {
    let Some(p) = p else {
        return 0x89002007;
    };
    clear(p + 24);
    heap_release(mem, nz(mem_word(mem, p + 20)), Some(p), vt);
    0
}

/// fn_196770 / EU fn_196820 / KR fn_19685c.
///
/// Clears the iterator's head subtree. `clear` stands in for the
/// tree teardown.
pub fn clear_iterator(p: Option<u32>, clear: &mut impl FnMut(u32)) -> u32 {
    let Some(p) = p else {
        return 0x89002007;
    };
    clear(p + 24);
    0
}

/// fn_160b54 / EU fn_160c18 / KR fn_160b38.
///
/// Releases the cell through its own heap. A null `p` is a caller
/// bug: the original dereferences it unconditionally.
pub fn release_self(mem: &[u8], p: u32, vt: &mut impl HeapVtable) -> u32 {
    heap_release(mem, nz(mem_word(mem, p + 4)), Some(p), vt);
    0
}

/// fn_196368 / EU fn_196418 / KR fn_196454.
///
/// Frees the node through its own interface.
pub fn free_node(mem: &[u8], node: Option<u32>, vt: &mut impl HeapVtable) -> u32 {
    let Some(node) = node else {
        return 0x80b02007;
    };
    vt.release_at(mem_word(mem, node + 4), node);
    0
}

/// fn_196318 / EU fn_1963c8 / KR fn_196404.
///
/// Allocates a node through the interface and links it back to the
/// interface and context.
pub fn alloc_node(
    mem: &mut [u8],
    iface: Option<u32>,
    ctx: u32,
    out: &mut u32,
    mvt: &mut impl MethodTable,
) -> u32 {
    let Some(iface) = iface else {
        return 0x80b02007;
    };
    let n = mvt.call_at(ctx, 1, 8);
    *out = n;
    if n == 0 {
        return 0x80b0200a;
    }
    mem_store(mem, n, iface);
    mem_store(mem, n + 4, ctx);
    0
}

/// fn_174e00 / EU fn_174eb0 / KR fn_174e5c.
///
/// Tears down the engine shell at `s`: destroys the two element arrays,
/// releases the cleared sub-object, runs the guarded teardown hook, then
/// releases the shell through its own allocator. `dtor` stands in for the
/// arrays' element destructor, `clear` for the sub-object teardown and
/// `call` for the guarded hook. A null `s` reports `0x8b702007`.
pub fn teardown_engine(
    mem: &mut [u8],
    s: Option<u32>,
    vt: &mut impl HeapVtable,
    clear: &mut impl FnMut(u32),
    mut dtor: impl FnMut(u32, u32),
    mut call: impl FnMut(u32) -> u32,
) -> u32 {
    let Some(s) = s else {
        return 0x8b702007;
    };
    if mem_word(mem, s + 0x10) != 0 {
        destroy_array(mem, nz(mem_word(mem, s + 0x10)), vt, &mut dtor);
        mem_store(mem, s + 0x10, 0);
    }
    if mem_word(mem, s + 0x0c) != 0 {
        destroy_array(mem, nz(mem_word(mem, s + 0x0c)), vt, &mut dtor);
        mem_store(mem, s + 0x0c, 0);
    }
    if mem_word(mem, s + 8) != 0 {
        release_cleared(mem, nz(mem_word(mem, s + 8)), clear, vt);
        mem_store(mem, s + 8, 0);
    }
    if mem_word(mem, s + 4) != 0 {
        release_guarded(mem, nz(mem_word(mem, s + 4)), vt, &mut call);
    }
    heap_release(mem, nz(mem_word(mem, s)), Some(s), vt);
    0
}

/// fn_177cc8 / EU fn_177d78 / KR fn_177d24.
///
/// Closes the broker object at `object`: when it is the context's active
/// object the database lock is acquired, the iterator cleared and the
/// lock released, folding a release failure into the report only when
/// the clear succeeded; then the object is released through the
/// context heap. `clear` stands in for the iterator teardown. A null
/// `object` reports `0x81506007`.
pub fn close_broker_object(
    mem: &[u8],
    object: Option<u32>,
    vt: &mut impl HeapVtable,
    clear: &mut impl FnMut(u32),
    lock: &mut impl StateLock,
) -> u32 {
    let Some(object) = object else {
        return 0x81506007;
    };
    let owner = mem_word(mem, object + 4);
    let context = mem_word(mem, owner + 8);
    let mut rc = 0u32;
    if mem_word(mem, context + 0x0c) == object {
        let status = lock.acquire();
        if status >= 0x80000000 {
            return status;
        }
        rc = clear_iterator(nz(mem_word(mem, owner + 0x14)), clear);
        let status = lock.release();
        if status >= 0x80000000 && rc < 0x80000000 {
            rc = status;
        }
    }
    heap_release(mem, nz(mem_word(mem, context + 4)), Some(object), vt);
    rc
}

/// fn_1985fc / EU fn_1986ac / KR fn_1986e8.
///
/// Stage-enter instrumentation hook: reports `0x88402007` for a null
/// handle and 0 otherwise. The tag word is ignored.
pub fn begin_stage_hook(handle: Option<u32>) -> u32 {
    if handle.is_some() {
        0
    } else {
        0x88402007
    }
}

/// fn_19860c / EU fn_1986bc / KR fn_1986f8.
///
/// Stage-exit instrumentation hook: reports `0x88402007` for a null
/// handle and 0 otherwise. The tag word is ignored.
pub fn end_stage_hook(handle: Option<u32>) -> u32 {
    if handle.is_some() {
        0
    } else {
        0x88402007
    }
}

/// fn_176624 / EU fn_1766d4 / KR fn_176680.
///
/// Length of the NUL-terminated u16 string at `s`, in code units.
pub fn u16_len(mem: &[u8], s: u32) -> u32 {
    let mut length = 0u32;
    while mem_half(mem, s.wrapping_add(length.wrapping_mul(2))) != 0 {
        length = length.wrapping_add(1);
    }
    length
}

/// fn_176214 / EU fn_1762c4 / KR fn_176270.
///
/// Copies the NUL-terminated u16 string at `src` to `dst`, terminator
/// included, and reports `dst`.
pub fn u16_copy(mem: &mut [u8], dst: u32, src: u32) -> u32 {
    let mut i = 0u32;
    loop {
        let unit = mem_half(&*mem, src.wrapping_add(i.wrapping_mul(2)));
        let at = dst.wrapping_add(i.wrapping_mul(2)) as usize;
        mem[at..at + 2].copy_from_slice(&unit.to_le_bytes());
        if unit == 0 {
            break;
        }
        i = i.wrapping_add(1);
    }
    dst
}

/// String-kind record types shared by `release_typed_buffer` and
/// `duplicate_typed_string` (fn_1516c8 / fn_151610).
const STRING_KINDS: [u16; 12] = [6, 12, 21, 50, 36, 39, 37, 34, 22, 40, 26, 29];

/// fn_1516c8 / EU fn_15178c / KR fn_1516ac.
///
/// Releases the buffer of the record at `p` through `heap` when its
/// type is a string kind and the buffer is live, clearing the slot.
/// A null record is a silent no-op.
pub fn release_typed_buffer(
    mem: &mut [u8],
    p: Option<u32>,
    heap: u32,
    vt: &mut impl HeapVtable,
) {
    let Some(p) = p else {
        return;
    };
    if STRING_KINDS.contains(&mem_half(mem, p)) && mem_word(mem, p + 20) != 0 {
        heap_release(&*mem, nz(heap), Some(mem_word(mem, p + 20)), vt);
        mem_store(mem, p + 20, 0);
    }
}

/// fn_151610 / EU fn_1516d4 / KR fn_1515f4.
///
/// Duplicates the NUL-terminated u16 string of the record at `p` into
/// `out` through `heap` when the type is a string kind. Reports 1, or
/// 0 when the allocation fails. Null records and other types are a
/// silent success.
pub fn duplicate_typed_string(
    mem: &mut [u8],
    p: Option<u32>,
    heap: Option<u32>,
    out: Option<u32>,
    vt: &mut impl HeapVtable,
) -> u32 {
    let (Some(p), Some(out)) = (p, out) else {
        return 1;
    };
    if !STRING_KINDS.contains(&mem_half(&*mem, p)) {
        return 1;
    }
    let s = mem_word(mem, p + 20);
    let n = if s == 0 { 0 } else { u16_len(mem, s) };
    let d = heap_alloc(mem, heap, n.wrapping_mul(2).wrapping_add(2), vt);
    if d == 0 {
        return 0;
    }
    if n == 0 {
        let at = d as usize;
        mem[at..at + 2].copy_from_slice(&0u16.to_le_bytes());
    } else {
        u16_copy(mem, d, s);
    }
    mem_store(mem, out + 20, d);
    1
}

/// fn_151840 / EU fn_151904 / KR fn_151824.
///
/// Validates the `name` parameter value `s`: readmode accepts 1 through
/// 4, waitfactor accepts 0 through 9, anything else reports `0x8120200f`.
/// The validity flag is always stored. Unrecognized names and inactive
/// engine contexts validate silently.
pub fn validate_param(
    mem: &[u8],
    table: &[u16; 256],
    p: Option<u32>,
    name: &[u8],
    s: Option<u32>,
    out: &mut u8,
    pair: &[u8; 8],
    default_ctx: u32,
) -> u32 {
    let mut ret = 0u32;
    let mut flag = 1u8;
    if let (Some(p), Some(s)) = (p, s) {
        if mem[s as usize] != 0 {
            let pa = mem_word(mem, p);
            let pb = mem_word(mem, p + 4);
            let pword = if pa == 0 {
                None
            } else {
                Some(mem_half(mem, pa))
            };
            let mut tmp = 0u32;
            if resolve_engine_context(pa, pword, pb, pair, default_ctx, &mut tmp) < 0x80000000
            {
                if compare_str(name, b"readmode\0") == 0 {
                    if parse_int(table, mem, s).wrapping_sub(1) as u32 >= 4 {
                        ret = 0x8120200f;
                        flag = 0;
                    }
                } else if compare_str(name, b"waitfactor\0") == 0
                    && parse_int(table, mem, s) as u32 >= 10
                {
                    ret = 0x8120200f;
                    flag = 0;
                }
            }
        }
    }
    *out = flag;
    ret
}

/// fn_15f068 / EU fn_15f12c / KR fn_15f04c.
///
/// Closes the component-query object: validates the handle, closes the
/// broker sub-object, runs the guarded teardown hook, then releases the
/// object through its heap. The first error wins; the teardown slot is
/// always cleared. `clear` stands in for the broker iterator teardown,
/// `call` for the guarded hook.
pub fn close_query_object(
    mem: &mut [u8],
    h_obj: Option<u32>,
    h_tag: u32,
    vt: &mut impl HeapVtable,
    clear: &mut impl FnMut(u32),
    lock: &mut impl StateLock,
    call: impl FnMut(u32) -> u32,
) -> u32 {
    let rc = validate_handle(h_obj.is_some(), h_tag, 0x15c, 32);
    let Some(o) = h_obj else {
        return rc;
    };
    if rc >= 0x80000000 {
        return rc;
    }
    let inner = mem_word(mem, o + 8);
    let mut rc = 0u32;
    if let Some(f16) = nz(mem_word(mem, o + 0x10)) {
        let rc2 = close_broker_object(mem, Some(f16), vt, clear, lock);
        if rc2 >= 0x80000000 {
            rc = rc2;
        }
    }
    if mem_word(mem, o + 0x1c) != 0 {
        let rc3 = release_guarded(mem, Some(mem_word(mem, o + 0x1c)), vt, call);
        mem_store(mem, o + 0x1c, 0);
        if rc3 >= 0x80000000 && rc < 0x80000000 {
            rc = rc3;
        }
    }
    heap_release(mem, nz(mem_word(mem, inner + 4)), Some(o), vt);
    rc
}

/// fn_1994d0 / EU fn_199580 / KR fn_1995bc.
///
/// Unsigned divide: reports `(quotient, remainder)` of `v / b`. The
/// divisor is never 0 (callers gate it; the fuzzer excludes it).
pub fn divmod_u32(v: u32, b: u32) -> (u32, u32) {
    (v / b, v % b)
}

/// fn_14e440 / EU fn_14e504 / KR fn_14e424.
///
/// Compacts the stream buffer at `state`: drops consumed bytes ahead
/// of the slowest reader, destroys discarded elements, memmoves the
/// tail, and shrinks or frees the buffer per the config minimums. A
/// nonzero `required` forces the compact. `dtor` stands in for the
/// element destructor. A null buffer or a kept buffer is a no-op.
pub fn compact_stream_buffer(
    mem: &mut [u8],
    state: u32,
    required: u32,
    vt: &mut impl HeapVtable,
    dtor: &mut impl FnMut(u32, u32),
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
        let mut element =
            (mem_word(mem, state + 52).wrapping_sub(old)).wrapping_add(buffer);
        let mut dpos = mem_word(mem, state + 52);
        while element < end {
            dtor(element, heap);
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
            heap_release(mem, nz(heap), Some(buffer), vt);
            mem_store(mem, state + 60, 0);
            mem_store(mem, state + 64, 0);
        } else {
            let newbuf = heap_realloc(
                mem,
                nz(heap),
                buffer,
                capacity.wrapping_sub(discarded),
                vt,
            );
            if newbuf != 0 {
                mem_store(mem, state + 60, newbuf);
                mem_store(mem, state + 64, capacity.wrapping_sub(discarded));
            }
        }
    }
}

/// fn_15017c / EU fn_150240 / KR fn_150160.
///
/// Ends a direct stream read of `bytes_read` bytes: validates the
/// `(stream, b)` handle pair, advances the stream position (clamped to
/// the available bytes with error `0x88f02007` on overrun), raises the
/// consumed high-water mark and compacts the state buffer. Non-direct
/// streams and bad handles return errors. The trace logging is
/// unobserved. `vt` and `dtor` serve the compaction step.
pub fn read_direct_end(
    mem: &mut [u8],
    stream: Option<u32>,
    b: u32,
    bytes_read: u32,
    vt: &mut impl HeapVtable,
    dtor: &mut impl FnMut(u32, u32),
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
    compact_stream_buffer(mem, state, 0, vt, dtor);
    rc
}

/// fn_14e614 / EU fn_14e6d8 / KR fn_14e5f8.
///
/// Trace-logs a stream write: below verbosity 6 it returns at once,
/// otherwise it logs the header shape for null or empty writes, the
/// binary shape for non-text states, or the text shape plus the
/// writer id. All logging is unobserved (void log stubs); `level` is
/// the scripted `fn_173fd8` answer. Every logged field is still read
/// so a wrong offset traps instead of passing silently.
pub fn log_write(mem: &[u8], state: u32, buffer: Option<u32>, count: u32, level: u32) {
    let context = mem_word(mem, state.wrapping_add(4));
    let _log = mem_word(mem, context.wrapping_add(16));
    if level < 6 {
        return;
    }
    let _writer = mem_half(mem, state.wrapping_add(24));
    let _content = mem_word(mem, state.wrapping_add(16));
    let _pipe = mem_half(mem, state.wrapping_add(20));
    let _refs = mem_half(mem, state.wrapping_add(22));
    let _write = mem_word(mem, state.wrapping_add(44));
    let _bufpos = mem_word(mem, state.wrapping_add(56));
    let _eos = mem[state.wrapping_add(68) as usize];
    if buffer.is_none() || count == 0 {
        return;
    }
    if mem[state.wrapping_add(41) as usize] == 0 {
        return;
    }
}

/// fn_1503b0 / EU fn_150474 / KR fn_150394.
///
/// Writes `count` bytes from `source` into the stream: validates the
/// `(object, tag)` handle pair, grows the state buffer through
/// compaction plus heap alloc/regrow (or takes the unbuffered path when
/// nothing is stored), copies the bytes, runs the per-element copy
/// callback with destroy rollback on failure, and advances the write
/// position. Null sources, empty writes and non-writable kinds report
/// `0x88f02007`; a backwards write position reports `0x88f02005`; an
/// allocation or element-copy failure reports `0x88f0200a`. The trace
/// logging (including the `fn_14e614` call) is unobserved. `source` and
/// the state buffer must not overlap. `vt`, `copy` and `dtor` serve the
/// heap and element steps.
pub fn write_stream(
    mem: &mut [u8],
    object: Option<u32>,
    tag: u32,
    source: Option<u32>,
    count: u32,
    vt: &mut impl HeapVtable,
    copy: &mut impl FnMut(u32, u32, u32) -> u32,
    dtor: &mut impl FnMut(u32, u32),
) -> u32 {
    if validate_handle(object.is_some(), tag, 0x47de, 12) >= 0x80000000 {
        return 0x81706008;
    }
    let stream = object.unwrap();
    let Some(src) = source else {
        return 0x88f02007;
    };
    if count == 0 {
        return 0x88f02007;
    }
    let kind = mem[stream as usize];
    if kind != 1 && kind != 2 {
        return 0x88f02007;
    }
    let state = mem_word(mem, stream + 4);
    let written = mem_word(mem, state + 44);
    let base = mem_word(mem, state + 56);
    if written < base {
        return 0x88f02005;
    }
    let heap = mem_word(mem, mem_word(mem, state + 4) + 4);
    let mut buffered = 1u32;
    let need0 = written.wrapping_sub(base).wrapping_add(count);
    if mem_word(mem, state + 64) < need0 {
        let storage = mem_word(mem, state + 12);
        let stored = if storage == 0 {
            word_at5(None)
        } else {
            let mut words = [0u32; 6];
            for (index, slot) in words.iter_mut().enumerate() {
                *slot = mem_word(mem, storage + index as u32 * 4);
            }
            word_at5(Some(&words))
        };
        if mem[(state + 40) as usize] != 0 || stored != 0 {
            let capacity = mem_word(mem, state + 64);
            compact_stream_buffer(mem, state, need0.wrapping_sub(capacity), vt, dtor);
            let need = mem_word(mem, state + 44)
                .wrapping_sub(mem_word(mem, state + 56))
                .wrapping_add(count);
            if mem_word(mem, state + 64) < need {
                let minimum = mem_word(mem, mem_word(mem, state) + 16);
                let need = if minimum >= need { minimum } else { need };
                let buffer = mem_word(mem, state + 60);
                let grown = if buffer != 0 {
                    heap_realloc(mem, nz(heap), buffer, need, vt)
                } else {
                    heap_alloc(mem, nz(heap), need, vt)
                };
                if grown == 0 {
                    return 0x88f0200a;
                }
                mem_store(mem, state + 60, grown);
                mem_store(mem, state + 64, need);
            }
        } else {
            buffered = 0;
        }
    }
    if buffered == 1 {
        let position = mem_word(mem, state + 44).wrapping_sub(mem_word(mem, state + 56));
        let buffer = mem_word(mem, state + 60);
        let dst = buffer.wrapping_add(position);
        let n = count as usize;
        let (src_us, dst_us) = (src as usize, dst as usize);
        if src_us < dst_us {
            let (head, tail) = mem.split_at_mut(dst_us);
            copy_bytes(&mut tail[..n], &head[src_us..src_us + n], count);
        } else if src_us > dst_us {
            let (head, tail) = mem.split_at_mut(src_us);
            copy_bytes(&mut head[dst_us..dst_us + n], &tail[..n], count);
        }
        if mem_word(mem, state + 32) != 0 {
            let stride = mem_half(mem, state + 28) as u32;
            let mut offset = 0u32;
            loop {
                if count <= offset {
                    break;
                }
                let at = position.wrapping_add(offset);
                if copy(src.wrapping_add(offset), heap, buffer.wrapping_add(at)) == 0 {
                    if mem_word(mem, state + 36) != 0 {
                        let mut done = 0u32;
                        while done < offset {
                            dtor(
                                buffer.wrapping_add(position.wrapping_add(done)),
                                heap,
                            );
                            done = done.wrapping_add(stride);
                        }
                    }
                    return 0x88f0200a;
                }
                offset = offset.wrapping_add(stride);
            }
        }
    } else {
        let at = mem_word(mem, state + 56);
        mem_store(mem, state + 56, at.wrapping_add(count));
    }
    let at = mem_word(mem, state + 44);
    mem_store(mem, state + 44, at.wrapping_add(count));
    0
}

/// fn_150878 / EU fn_15093c / KR fn_15085c.
///
/// Begins a direct stream write of `count` bytes: validates the
/// `(object, tag)` handle pair, reserves buffer space through
/// compaction plus heap alloc/regrow (at least the object maximum),
/// hands the write pointer back through `out`, and zeroes the
/// region. Null handles fail validation; a null `out` reports
/// `0x88f02007` without writing; empty writes, non-writable kinds
/// and a backwards write position clear `out` and report
/// `0x88f02007` (`0x88f02005` for the backwards position); an
/// allocation failure clears `out` and reports `0x88f0200a`. The
/// trace logging is unobserved. `vt` and `dtor` serve the compaction
/// and heap steps.
pub fn write_direct_start(
    mem: &mut [u8],
    object: Option<u32>,
    tag: u32,
    out: Option<u32>,
    count: u32,
    vt: &mut impl HeapVtable,
    dtor: &mut impl FnMut(u32, u32),
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
        compact_stream_buffer(mem, state, need0.wrapping_sub(mem_word(mem, state + 64)), vt, dtor);
        let need = mem_word(mem, state + 44)
            .wrapping_sub(mem_word(mem, state + 56))
            .wrapping_add(count);
        if mem_word(mem, state + 64) < need {
            let maximum = mem_word(mem, mem_word(mem, state) + 16);
            let need = if need > maximum { need } else { maximum };
            let buffer = mem_word(mem, state + 60);
            let grown = if buffer != 0 {
                heap_realloc(mem, nz(heap), buffer, need, vt)
            } else {
                heap_alloc(mem, nz(heap), need, vt)
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

/// fn_150c0c / EU fn_150cd0 / KR fn_150bf0.
///
/// Ends a direct stream write of `count` bytes: validates the
/// `(object, tag)` handle pair, runs the per-element copy callback
/// over the fresh bytes with destroy rollback on failure, advances
/// the write position, shrinks the buffer to the needed size past
/// the object limit, and compacts the state buffer. Non-writable
/// kinds and short capacities report `0x88f02007`; a backwards write
/// position reports `0x88f02005`; an element-copy failure reports
/// `0x88f0200a`. The trace logging (including the `fn_14e614` call)
/// is unobserved. `vt`, `copy` and `dtor` serve the heap and element
/// steps.
pub fn write_direct_end(
    mem: &mut [u8],
    object: Option<u32>,
    tag: u32,
    count: u32,
    vt: &mut impl HeapVtable,
    copy: &mut impl FnMut(u32, u32, u32) -> u32,
    dtor: &mut impl FnMut(u32, u32),
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
            if copy(at, heap, at) == 0 {
                if mem_word(mem, state + 36) != 0 {
                    let mut done = 0u32;
                    while done < offset {
                        dtor(buffer.wrapping_add(base.wrapping_add(done)), heap);
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
            heap_release(mem, nz(heap), nz(buffer), vt);
            mem_store(mem, state + 60, 0);
        } else {
            let grown = heap_realloc(mem, nz(heap), buffer, need, vt);
            if grown != 0 {
                mem_store(mem, state + 60, grown);
            }
        }
    }
    compact_stream_buffer(mem, state, 0, vt, dtor);
    0
}

/// fn_150fbc / EU fn_151080 / KR fn_150fa0.
///
/// Copies up to `count` bytes from the source stream into the
/// destination stream: validates the destination handle, peeks the
/// source's readable window, writes what fits, and consumes the
/// source bytes. Reports `0x88f02007` for an empty write, a
/// non-writable kind or a failed peek, and the write's own status
/// otherwise; `written` takes the copied count (0 on every early
/// path) and must be valid (the original faults on null). `data`
/// and `avail` are scratch words for the peek outputs. `vt`,
/// `copy` and `dtor` serve the write and consume steps.
#[allow(clippy::too_many_arguments)]
pub fn write_copy(
    mem: &mut [u8],
    object: Option<u32>,
    tag: u32,
    src_object: Option<u32>,
    src_tag: u32,
    count: u32,
    written: u32,
    data: u32,
    avail: u32,
    vt: &mut impl HeapVtable,
    copy: &mut impl FnMut(u32, u32, u32) -> u32,
    dtor: &mut impl FnMut(u32, u32),
) -> u32 {
    let rc = validate_handle(object.is_some(), tag, 0x47de, 12);
    if rc >= 0x80000000 {
        return rc;
    }
    if count == 0 {
        mem_store(mem, written, 0);
        return 0x88f02007;
    }
    let kind = mem[object.unwrap() as usize];
    if kind != 1 && kind != 2 {
        mem_store(mem, written, 0);
        return 0x88f02007;
    }
    mem_store(mem, written, 0);
    let rc = read_direct_start(mem, src_object, src_tag, Some(data), Some(avail));
    if rc >= 0x80000000 {
        return rc;
    }
    if mem_word(mem, avail) == 0 {
        return rc;
    }
    let available = mem_word(mem, avail);
    let count = if available < count { available } else { count };
    let rc = write_stream(
        mem,
        object,
        tag,
        Some(mem_word(mem, data)),
        count,
        vt,
        copy,
        dtor,
    );
    if rc < 0x80000000 {
        mem_store(mem, written, count);
        read_direct_end(mem, src_object, src_tag, count, vt, dtor);
    } else {
        read_direct_end(mem, src_object, src_tag, 0, vt, dtor);
    }
    rc
}

/// fn_281618 / EU fn_2817d4 / KR fn_281880.
///
/// Duplicates a NUL-terminated string. The allocation hook represents
/// fn_196394, with the caller's heap handle and requested byte count.
pub fn duplicate_voice_name(
    mem: &mut [u8],
    ctx: u32,
    source: u32,
    out: u32,
    heap: &mut impl HeapVtable,
) -> u32 {
    let mut len = 0u32;
    while mem[source.wrapping_add(len) as usize] != 0 {
        len = len.wrapping_add(1);
    }
    let size = len.wrapping_add(1);
    let dst = heap.alloc_at(ctx, size);
    mem_store(mem, out, dst);
    if dst == 0 {
        return 0x88e0200a;
    }
    // strcpy reads the source after the allocation result is stored.
    let mut i = 0u32;
    loop {
        let byte = mem[source.wrapping_add(i) as usize];
        mem[dst.wrapping_add(i) as usize] = byte;
        if byte == 0 {
            break;
        }
        i = i.wrapping_add(1);
    }
    0
}

/// Direct fn_1963b8 hook, preserving both full-width allocation arguments.
pub trait VoiceNodeAllocator: HeapVtable {
    fn alloc_items_at(&mut self, ctx: u32, count: u32, width: u32) -> u32;
}

/// fn_281224 / EU fn_2813e0 / KR fn_28148c.
///
/// Replaces a voice node's payload. Failed allocations leave the partial
/// replacement in the node, rather than releasing it as the constructor does.
/// `amount` carries the soft-float argument's unmodified bits.
pub fn update_voice_node(
    mem: &mut [u8],
    ctx: u32,
    node: u32,
    kind: u32,
    value: u32,
    amount: u32,
    name: Option<u32>,
    count: u32,
    items: Option<u32>,
    heap: &mut impl VoiceNodeAllocator,
) -> u32 {
    if (kind == 1 && (name.is_some() || items.is_some())) || name.is_some() != items.is_some() {
        return 0x88e02000;
    }
    let old_name = mem_word(mem, node.wrapping_add(12));
    if old_name != 0 {
        heap.release_at(ctx, old_name);
        mem_store(mem, node.wrapping_add(12), 0);
    }
    let old_items = mem_word(mem, node.wrapping_add(20));
    if old_items != 0 {
        heap.release_at(ctx, old_items);
        mem_store(mem, node.wrapping_add(20), 0);
        mem_store16(mem, node.wrapping_add(16), 0);
    }
    mem[node as usize] = kind as u8;
    mem_store(mem, node.wrapping_add(4), value);
    mem_store(mem, node.wrapping_add(8), amount);
    if let (Some(name), Some(items)) = (name, items) {
        duplicate_voice_name(mem, ctx, name, node.wrapping_add(12), heap);
        let block = heap.alloc_items_at(ctx, count, 4);
        mem_store(mem, node.wrapping_add(20), block);
        if block != 0 {
            move_bytes(mem, block, items, count.wrapping_mul(4));
            mem_store16(mem, node.wrapping_add(16), count as u16);
        }
        if mem_word(mem, node.wrapping_add(12)) == 0 || block == 0 {
            return 0x88e0200a;
        }
    }
    0
}

/// fn_28107c / EU fn_281238 / KR fn_2812e4.
///
/// Creates a voice node: kind 1 takes no payload, otherwise
/// `name` and `items` must both be null or both be present. The
/// node carries `value`/`value2` plus the low byte of `kind`;
/// with a payload the name duplicates through `dup` and
/// `count` words copy from `items` via [`move_bytes`]. A failed
/// duplicate or items allocation releases the partial node and
/// reports null.
pub fn create_voice_node(
    mem: &mut [u8],
    ctx: u32,
    kind: u32,
    value: u32,
    value2: u32,
    name: Option<u32>,
    count: u32,
    items: Option<u32>,
    heap: &mut impl HeapVtable,
    dup: &mut impl FnMut(u32, u32, &mut u32),
) -> u32 {
    if kind == 1 && (name.is_some() || items.is_some()) {
        return 0;
    }
    match (name, items) {
        (None, Some(_)) | (Some(_), None) => return 0,
        _ => {}
    }
    let node = heap.alloc_at(ctx, 28);
    if node == 0 {
        return 0;
    }
    mem_store(mem, node.wrapping_add(12), 0);
    mem_store16(mem, node.wrapping_add(16), 0);
    mem_store(mem, node.wrapping_add(20), 0);
    mem[node as usize] = kind as u8;
    mem_store(mem, node.wrapping_add(4), value);
    mem_store(mem, node.wrapping_add(8), value2);
    if let (Some(n), Some(it)) = (name, items) {
        let mut dst = 0u32;
        dup(ctx, n, &mut dst);
        mem_store(mem, node.wrapping_add(12), dst);
        let blk = heap.alloc_at(ctx, count.wrapping_mul(4));
        mem_store(mem, node.wrapping_add(20), blk);
        if blk != 0 {
            move_bytes(mem, blk, it, count.wrapping_mul(4));
            mem_store16(mem, node.wrapping_add(16), count as u16);
        }
        if dst == 0 || blk == 0 {
            let mut cur = node;
            loop {
                let next = mem_word(mem, cur.wrapping_add(24));
                let nm = mem_word(mem, cur.wrapping_add(12));
                if nm != 0 {
                    heap.release_at(ctx, nm);
                }
                let itw = mem_word(mem, cur.wrapping_add(20));
                if itw != 0 {
                    heap.release_at(ctx, itw);
                }
                heap.release_at(ctx, cur);
                if next == 0 {
                    break;
                }
                cur = next;
            }
            return 0;
        }
    }
    node
}

/// fn_281024 / EU fn_2811e0 / KR fn_28128c.
///
/// Releases a node chain: walks the next links from `n` and
/// releases each node's two payload words (when nonzero) plus the
/// node itself through the owner's heap. A null owner drops every
/// release.
pub fn release_node_chain(
    mem: &[u8],
    n: Option<u32>,
    owner: Option<u32>,
    vt: &mut impl HeapVtable,
) {
    let mut cur = n;
    while let Some(node) = cur {
        let fc = mem_word(mem, node.wrapping_add(12));
        let f14 = mem_word(mem, node.wrapping_add(20));
        cur = nz(mem_word(mem, node.wrapping_add(24)));
        if fc != 0 {
            heap_release(mem, owner, nz(fc), vt);
        }
        if f14 != 0 {
            heap_release(mem, owner, nz(f14), vt);
        }
        heap_release(mem, owner, Some(node), vt);
    }
}

/// fn_15cdcc / EU fn_15ce90 / KR fn_15cdb0.
///
/// Writes `count` elements from `input` into `field` of `record`:
/// variable-length fields allocate through the database arena (at
/// `object + 4`) and store the pointer, fixed-length fields copy in
/// place up to their length, and the count anchor, `written` and the
/// dirty flag update on success. Fields of the word list (whose data
/// anchor is `word_anchor`) additionally intern the strings of word
/// entries. `descs` holds the record layouts (272 bytes each, as in
/// the ROM table); a record type beyond it reports `0x89203e0d`
/// like a missing layout. A null `input` reads nothing and is only
/// valid with an empty write. `alloc` serves the arena grows.
pub fn write_field(
    mem: &mut [u8],
    object: Option<u32>,
    tag: u32,
    record: u32,
    field: u32,
    count: u32,
    input: Option<u32>,
    written: u32,
    descs: &[u8],
    word_anchor: u32,
    alloc: &mut impl FnMut(u32, u32) -> u32,
) -> u32 {
    if validate_handle(object.is_some(), tag, 0xb2c2, 0x38) >= 0x80000000 {
        return 0x89202008;
    }
    let db = object.unwrap();
    let table = mem_word(mem, db + 48);
    let data = if record < mem_word(mem, db + 32) {
        mem_word(mem, table.wrapping_add(record.wrapping_mul(4)))
    } else {
        0
    };
    if data == 0 {
        return 0x89203e11;
    }
    let ty = mem[data as usize + 2];
    let layout = match descs.get(ty as usize * 272..) {
        Some(layout) if layout.len() >= 272 => layout,
        _ => return 0x89203e0d,
    };
    let base = u32::from_le_bytes(layout[0..4].try_into().unwrap());
    let nfields = u16::from_le_bytes(layout[4..6].try_into().unwrap()) as u32;
    let states = u32::from_le_bytes(layout[8..12].try_into().unwrap());
    if ty == 0 || field >= nfields {
        return 0x89203e0d;
    }
    let field_off = 12 + field as usize * 20;
    let fdata = u32::from_le_bytes(layout[field_off..field_off + 4].try_into().unwrap());
    let fcount = u32::from_le_bytes(layout[field_off + 4..field_off + 8].try_into().unwrap());
    let flength = u16::from_le_bytes(layout[field_off + 8..field_off + 10].try_into().unwrap());
    let fsize = u16::from_le_bytes(layout[field_off + 10..field_off + 12].try_into().unwrap());
    let dest = data.wrapping_add(fdata.wrapping_sub(base));
    let cnt = if fcount != 0 {
        Some(data.wrapping_add(fcount.wrapping_sub(base)))
    } else {
        None
    };
    mem[data
        .wrapping_add(states.wrapping_sub(base))
        .wrapping_add(field) as usize] = 1;
    let src = input.unwrap_or(0);
    if flength == 0 {
        let bytes = (fsize as u32).wrapping_mul(count);
        let p = arena_alloc(mem, db + 4, bytes, alloc);
        mem_store(mem, dest, p);
        if p == 0 {
            return 0x8920200a;
        }
        move_bytes(mem, p, src, bytes);
        if fdata == word_anchor {
            let mut i = 0u32;
            while i < count {
                let entry = src.wrapping_add(i.wrapping_mul(24));
                let etype = mem_half(mem, entry);
                if matches!(
                    etype,
                    6 | 12 | 21 | 50 | 36 | 39 | 37 | 34 | 22 | 40 | 26 | 29
                ) {
                    let str_ptr = mem_word(mem, entry + 20);
                    let n = if str_ptr == 0 {
                        0
                    } else {
                        u16_len(mem, str_ptr)
                    };
                    let d = arena_alloc(
                        mem,
                        db + 4,
                        n.wrapping_mul(2).wrapping_add(2),
                        alloc,
                    );
                    if d == 0 {
                        return 0x8920200a;
                    }
                    if n == 0 {
                        mem_store16(mem, d, 0);
                    } else {
                        u16_copy(mem, d, str_ptr);
                    }
                    mem_store(mem, p.wrapping_add(i.wrapping_mul(24)).wrapping_add(20), d);
                }
                i = i.wrapping_add(1);
            }
        }
        if let Some(cnt) = cnt {
            mem_store16(mem, cnt, count as u16);
        }
        mem_store16(mem, written, count as u16);
    } else {
        let length = if flength as u32 > count {
            count
        } else {
            flength as u32
        };
        move_bytes(mem, dest, src, (fsize as u32).wrapping_mul(length));
        if let Some(cnt) = cnt {
            mem_store16(mem, cnt, length as u16);
        }
        mem_store16(mem, written, length as u16);
    }
    mem[db as usize + 52] = 1;
    0
}

/// fn_15b758 / EU fn_15b81c / KR fn_15b73c.
///
/// Creates a linguistic record of `rectype` (valid: 1..=4): regrows
/// the record list through the heap when full, carves the node from
/// the database arena with the `sizes` table, stamps id/type,
/// zeroes the kind-0 layout fields from `layouts`, and publishes
/// the node. `rectype` arrives full 32-bit and is gated unsigned.
/// `sizes`/`layouts` are the `D_8d8274`/`D_8d7ca0` windows. The
/// field offset comes from 20 bytes below the source's member
/// (kind at layout+20+20i, offset at layout+12+20i): the ROM loop
/// addresses it there on all three regions while
/// `src/fn_15b758.c` reads one slot higher, a reported C defect.
/// The trace logging is unobserved. Returns `0x89203e09` (bad
/// type), `0x89203e10` (empty/full count), `0x8920200a` (alloc
/// failure) or `0x89203e11` (missing node).
pub fn create_record(
    mem: &mut [u8],
    db: u32,
    rectype: u32,
    out: u32,
    sizes: &[u8],
    layouts: &[u8],
    vt: &mut impl HeapVtable,
    alloc: &mut impl FnMut(u32, u32) -> u32,
) -> u32 {
    if rectype.wrapping_sub(1) >= 4 {
        return 0x89203e09;
    }
    let count = mem_word(mem, db.wrapping_add(32));
    if mem_word(mem, db.wrapping_add(44)) < count || count == 0 {
        return 0x89203e10;
    }
    let capacity = mem_word(mem, db.wrapping_add(36));
    if capacity <= count {
        let new_capacity = capacity.wrapping_add(mem_word(mem, db.wrapping_add(40)));
        let heap = mem_word(mem, mem_word(mem, db).wrapping_add(4));
        let grown = heap_realloc(
            mem,
            nz(heap),
            mem_word(mem, db.wrapping_add(48)),
            new_capacity.wrapping_mul(4),
            vt,
        );
        if grown == 0 {
            return 0x8920200a;
        }
        mem_store(mem, db.wrapping_add(48), grown);
        mem_store(mem, db.wrapping_add(36), new_capacity);
        let at = grown.wrapping_add(count.wrapping_mul(4)) as usize;
        let len = new_capacity.wrapping_sub(count).wrapping_mul(4) as usize;
        mem[at..at + len].fill(0);
    }
    let base = rectype as usize * 4;
    let size = u32::from_le_bytes(sizes[base..base + 4].try_into().unwrap());
    let record = arena_alloc(mem, db.wrapping_add(4), size, alloc);
    mem_store(mem, out, record);
    if record == 0 {
        return 0x8920200a;
    }
    let records = mem_word(mem, db.wrapping_add(48));
    mem_store(mem, records.wrapping_add(count.wrapping_mul(4)), record);
    mem_store16(mem, record, count as u16);
    mem[record.wrapping_add(2) as usize] = rectype as u8;
    let new_count = count.wrapping_add(1);
    mem_store(mem, db.wrapping_add(32), new_count);
    let id = u32::from(mem_half(mem, record));
    if new_count > id {
        let found = mem_word(mem, records.wrapping_add(id.wrapping_mul(4)));
        if found != 0 {
            let ftype = mem[found.wrapping_add(2) as usize] as usize;
            let layout = ftype.wrapping_mul(0x110);
            let lbase = u32::from_le_bytes(layouts[layout..layout + 4].try_into().unwrap());
            let nfields =
                u16::from_le_bytes(layouts[layout + 4..layout + 6].try_into().unwrap()) as u32;
            let mut field = 0u8;
            while (field as u32) < nfields {
                let fbase = layout + (field as usize) * 20;
                let kind = u16::from_le_bytes(
                    layouts[fbase + 20..fbase + 22].try_into().unwrap(),
                );
                if kind == 0 {
                    let off = u32::from_le_bytes(
                        layouts[fbase + 12..fbase + 16].try_into().unwrap(),
                    );
                    mem_store(mem, found.wrapping_add(off.wrapping_sub(lbase)), 0);
                }
                field = field.wrapping_add(1);
            }
            mem[db as usize + 52] = 1;
            return 0;
        }
    }
    0x89203e11
}

/// fn_15c1e4 / EU fn_15c2a8 / KR fn_15c1c8.
///
/// Clones records[`index`] through [`create_record`] and splices
/// the copy after the source, reporting the copy's id at `out`.
/// `scratch` is a caller-owned word for the inner call's node
/// output (unobserved, like [`open_stream`]'s scratch). Returns
/// `0x89202008` (bad handle), `0x89203e11` (missing source),
/// `0x89203e04` (type-1 source) or the create status.
pub fn clone_record(
    mem: &mut [u8],
    db: Option<u32>,
    a: u32,
    index: u32,
    out: u32,
    scratch: u32,
    sizes: &[u8],
    layouts: &[u8],
    vt: &mut impl HeapVtable,
    alloc: &mut impl FnMut(u32, u32) -> u32,
) -> u32 {
    if validate_handle(db.is_some(), a, 0xb2c2, 0x38) >= 0x80000000 {
        return 0x89202008;
    }
    let base = db.unwrap();
    if index >= mem_word(mem, base.wrapping_add(32)) {
        return 0x89203e11;
    }
    let records = mem_word(mem, base.wrapping_add(48));
    let record = mem_word(mem, records.wrapping_add(index.wrapping_mul(4)));
    if record == 0 {
        return 0x89203e11;
    }
    let rtype = mem[record.wrapping_add(2) as usize];
    if rtype == 1 {
        return 0x89203e04;
    }
    let rc = create_record(
        mem,
        base,
        u32::from(rtype),
        scratch,
        sizes,
        layouts,
        vt,
        alloc,
    );
    if rc >= 0x80000000 {
        return rc;
    }
    let node = mem_word(mem, scratch);
    mem_store(mem, node.wrapping_add(4), mem_word(mem, record.wrapping_add(4)));
    mem_store(mem, node.wrapping_add(12), record);
    let next = mem_word(mem, record.wrapping_add(8));
    mem_store(mem, node.wrapping_add(8), next);
    mem_store(mem, record.wrapping_add(8), node);
    mem_store(mem, node.wrapping_add(16), 0);
    if next != 0 {
        mem_store(mem, next.wrapping_add(12), node);
    }
    mem_store16(mem, out, mem_half(mem, node));
    mem[base as usize + 52] = 1;
    rc
}

/// fn_15bf74 / EU fn_15c038 / KR fn_15bf58.
///
/// Creates a `child_type` record under the parent at `index:
/// validates the handle, checks the child type against the parent
/// type through `parent_table`, creates the record, links it in
/// (splicing before the matching child or taking the first free
/// child slot), and reports its id through `out`. A child type
/// below 2 reports `0x89203e02`, a parent mismatch `0x89203e03`,
/// and a full child pair `0x89203e1b`. `scratch` takes the fresh
/// node; `parent_table` must cover `child_type` (the row pins it
/// below 32: wider values read region-divergent bytes).
#[allow(clippy::too_many_arguments)]
pub fn create_child_record(
    mem: &mut [u8],
    object: Option<u32>,
    tag: u32,
    child_type: u32,
    index: u32,
    out: u32,
    scratch: u32,
    parent_table: &[u8],
    sizes: &[u8],
    layouts: &[u8],
    vt: &mut impl HeapVtable,
    alloc: &mut impl FnMut(u32, u32) -> u32,
) -> u32 {
    if validate_handle(object.is_some(), tag, 0xb2c2, 0x38) >= 0x80000000 {
        return 0x89202008;
    }
    let base = object.unwrap();
    if index >= mem_word(mem, base.wrapping_add(32)) {
        return 0x89203e11;
    }
    let records = mem_word(mem, base.wrapping_add(48));
    let parent = mem_word(mem, records.wrapping_add(index.wrapping_mul(4)));
    if parent == 0 {
        return 0x89203e11;
    }
    let parent_type = mem[parent.wrapping_add(2) as usize];
    if child_type < 2 {
        return 0x89203e02;
    }
    if parent_table[child_type as usize] != parent_type {
        return 0x89203e03;
    }
    let rc = create_record(
        mem, base, child_type, scratch, sizes, layouts, vt, alloc,
    );
    if rc >= 0x80000000 {
        return rc;
    }
    let node = mem_word(mem, scratch);
    mem_store(mem, node.wrapping_add(4), parent);
    mem_store(mem, node.wrapping_add(16), 0);
    mem_store(mem, node.wrapping_add(12), 0);
    let mut found = false;
    let mut i = 0u32;
    while i < 2 && !found {
        let slot = parent.wrapping_add(16).wrapping_add(i.wrapping_mul(4));
        let child = mem_word(mem, slot);
        if child != 0 && u32::from(mem[child.wrapping_add(2) as usize]) == child_type {
            found = true;
            mem_store(mem, node.wrapping_add(8), child);
            mem_store(mem, child.wrapping_add(12), node);
            mem_store(mem, slot, node);
        }
        i += 1;
    }
    if !found {
        let mut j = 0u32;
        while j < 2
            && mem_word(
                mem,
                parent.wrapping_add(16).wrapping_add(j.wrapping_mul(4)),
            ) != 0
        {
            j += 1;
        }
        if j < 2 {
            mem_store(
                mem,
                parent.wrapping_add(16).wrapping_add(j.wrapping_mul(4)),
                node,
            );
            mem_store(mem, node.wrapping_add(8), 0);
        } else {
            return 0x89203e1b;
        }
    }
    mem_store16(mem, out, mem_half(mem, node));
    mem[base.wrapping_add(52) as usize] = 1;
    rc
}

/// fn_15c2f0 / EU fn_15c3b4 / KR fn_15c2d4.
///
/// Creates the predecessor of the record at `index`: with a live
/// second reference it clones the referenced record, with only a
/// first reference it creates a same-typed child under the
/// referenced parent, and with neither it reports `0x89203e06`.
/// Type-1 records report `0x89203e05`. `result_slot` takes the
/// inner id, `scratch` the fresh node. `parent_table` serves the
/// child path (same under-32 envelope as [`create_child_record`]).
#[allow(clippy::too_many_arguments)]
pub fn create_prev_record(
    mem: &mut [u8],
    object: Option<u32>,
    tag: u32,
    index: u32,
    out: u32,
    result_slot: u32,
    scratch: u32,
    parent_table: &[u8],
    sizes: &[u8],
    layouts: &[u8],
    vt: &mut impl HeapVtable,
    alloc: &mut impl FnMut(u32, u32) -> u32,
) -> u32 {
    if validate_handle(object.is_some(), tag, 0xb2c2, 0x38) >= 0x80000000 {
        return 0x89202008;
    }
    let base = object.unwrap();
    if index >= mem_word(mem, base.wrapping_add(32)) {
        return 0x89203e11;
    }
    let records = mem_word(mem, base.wrapping_add(48));
    let record = mem_word(mem, records.wrapping_add(index.wrapping_mul(4)));
    if record == 0 {
        return 0x89203e11;
    }
    let rtype = mem[record.wrapping_add(2) as usize];
    if rtype == 1 {
        return 0x89203e05;
    }
    mem_store16(mem, result_slot, 0);
    let ref2 = mem_word(mem, record.wrapping_add(12));
    let refr = mem_word(mem, record.wrapping_add(4));
    let err = if ref2 != 0 {
        clone_record(
            mem,
            Some(base),
            tag,
            u32::from(mem_half(mem, ref2)),
            result_slot,
            scratch,
            sizes,
            layouts,
            vt,
            alloc,
        )
    } else if refr != 0 {
        create_child_record(
            mem,
            Some(base),
            tag,
            u32::from(rtype),
            u32::from(mem_half(mem, refr)),
            result_slot,
            scratch,
            parent_table,
            sizes,
            layouts,
            vt,
            alloc,
        )
    } else {
        return 0x89203e06;
    };
    if err >= 0x80000000 {
        return err;
    }
    mem_store16(mem, out, mem_half(mem, result_slot));
    mem[base.wrapping_add(52) as usize] = 1;
    err
}

/// fn_14e0b4 / EU fn_14e178 / KR fn_14e098.
///
/// Releases a stream state: a null state is a no-op, a multiply
/// referenced state just loses one reference, otherwise the content
/// type, readers, remaining buffer elements and the buffer and state
/// itself are destroyed and released through the context heap. The
/// `heap` argument is vestigial (the original ignores it) and the
/// input-logging block is unobserved, so this port covers the
/// `logInput == 0` behavior.
pub fn release_stream_state(
    mem: &mut [u8],
    state: Option<u32>,
    _heap: u32,
    vt: &mut impl HeapVtable,
    dtor: &mut impl FnMut(u32, u32),
) {
    let Some(base) = state else {
        return;
    };
    let references = mem_half(mem, base + 22);
    if references > 1 {
        let at = base as usize + 22;
        mem[at..at + 2].copy_from_slice(&(references - 1).to_le_bytes());
        return;
    }
    let heap = mem_word(mem, mem_word(mem, base + 4) + 4);
    let content = mem_word(mem, base + 16);
    if content != 0 {
        heap_release(mem, nz(heap), Some(content), vt);
    }
    let readers = mem_word(mem, base + 12);
    if readers != 0 {
        destroy_array(mem, Some(readers), vt, &mut *dtor);
    }
    let buffer = mem_word(mem, base + 60);
    if buffer != 0 {
        if mem_word(mem, base + 36) != 0 {
            let old = mem_word(mem, base + 56);
            let stride = mem_half(mem, base + 28) as u32;
            let end = buffer.wrapping_add(mem_word(mem, base + 44).wrapping_sub(old));
            let mut element =
                buffer.wrapping_add(mem_word(mem, base + 52).wrapping_sub(old));
            let mut dpos = mem_word(mem, base + 52);
            while element < end {
                dtor(element, heap);
                dpos = dpos.wrapping_add(stride);
                element = element.wrapping_add(stride);
            }
            mem_store(mem, base + 52, dpos);
        }
        heap_release(mem, nz(heap), Some(buffer), vt);
    }
    heap_release(mem, nz(heap), Some(base), vt);
}

/// fn_14f918 / EU fn_14f9dc / KR fn_14f8fc.
///
/// Closes a stream: validates the `(object, tag)` handle pair, then
/// either unlinks a kind-0 stream from its list (removing the first
/// matching entry through the `fn_1970ac` walk and the `fn_196e28`
/// compaction) or clears a writer stream's writer and eos and
/// trace-logs the write; finally releases the state and the stream
/// itself through the context allocator. `level` is the scripted
/// `fn_173fd8` answer for the inner [`log_write`]; all trace
/// logging is unobserved. `remove_dtor` serves a non-null list
/// destructor (the harness keeps it null and asserts it never
/// fires); `dtor` serves the state release. A kind-0 close needs a
/// non-null list with at least one entry: the original dereferences
/// the walk cursor afterwards, so an empty list crashes it too.
pub fn close_stream(
    mem: &mut [u8],
    object: Option<u32>,
    tag: u32,
    level: u32,
    vt: &mut impl HeapVtable,
    remove_dtor: &mut impl FnMut(u32, u32),
    dtor: &mut impl FnMut(u32, u32),
) -> u32 {
    if validate_handle(object.is_some(), tag, 0x47de, 12) >= 0x80000000 {
        return 0x81706008;
    }
    let stream = object.unwrap();
    let state = mem_word(mem, stream.wrapping_add(4));
    let context = mem_word(mem, state.wrapping_add(4));
    let _log = mem_word(mem, context.wrapping_add(16));
    if mem[stream as usize] == 0 {
        let list = mem_word(mem, state.wrapping_add(12));
        let base = mem_word(mem, list);
        let size = mem_half(mem, list.wrapping_add(4)) as u32;
        let count = mem_word(mem, list.wrapping_add(20));
        let mut index = 0u32;
        while index < count
            && mem_word(mem, base.wrapping_add(index.wrapping_mul(size))) != stream
        {
            index = index.wrapping_add(1);
        }
        if index < count {
            if mem_word(mem, list.wrapping_add(12)) != 0 {
                remove_dtor(
                    base.wrapping_add(index.wrapping_mul(size)),
                    mem_word(mem, list.wrapping_add(28)),
                );
            }
            let rest = count.wrapping_sub(index).wrapping_sub(1);
            if rest != 0 {
                let dst = base.wrapping_add(index.wrapping_mul(size));
                let src = base.wrapping_add(index.wrapping_add(1).wrapping_mul(size));
                move_bytes(mem, dst, src, size.wrapping_mul(rest));
            }
            mem_store(mem, list.wrapping_add(20), count.wrapping_sub(1));
        }
    } else {
        mem_store(mem, state.wrapping_add(8), 0);
        mem[state.wrapping_add(68) as usize] = 1;
        log_write(mem, state, None, 0, level);
    }
    release_stream_state(mem, Some(state), 0, vt, dtor);
    heap_release(
        mem,
        nz(mem_word(mem, context.wrapping_add(4))),
        Some(stream),
        vt,
    );
    0
}

/// Scripted answers for the stubbed map/list/context interfaces behind
/// [`open_stream`]: the map-find status, the map-insert status, the
/// reader-append status, and the context-resolve status plus context.
pub struct OpenScripts {
    /// `fn_196794` status (`(rc & 0x1fff) == 20` means absent-but-ok).
    pub rc794: u32,
    /// `fn_19668c` map-insert status.
    pub rc68c: u32,
    /// `fn_196db0` reader-append status.
    pub rc_db0: u32,
    /// `fn_19611c` context-resolve status.
    pub rc_611c: u32,
    /// `fn_19611c` context output.
    pub ctx_611c: u32,
}

/// fn_14ed4c / EU fn_14ee10 / KR fn_14ed30.
///
/// Sets the stream-config pipeline position: validates the `(config,
/// tag)` handle pair, resolves the context, then stores the low 16
/// bits of `position` at `config+8`. `position` arrives full 32-bit
/// and narrows on store. The context feeds only the trace logging,
/// which is unobserved (void log stubs, like [`open_stream`]).
/// Returns the validate status on failure, else the resolve status.
pub fn set_pipeline_pos(
    mem: &mut [u8],
    config: Option<u32>,
    tag: u32,
    position: u32,
    scripts: &OpenScripts,
) -> u32 {
    if validate_handle(config.is_some(), tag, 0x47dd, 32) >= 0x80000000 {
        return 0x81706008;
    }
    if scripts.rc_611c >= 0x80000000 {
        return scripts.rc_611c;
    }
    mem_store16(mem, config.unwrap() + 8, position as u16);
    scripts.rc_611c
}

/// fn_14f010 / EU fn_14f0d4 / KR fn_14eff4.
///
/// Opens a stream: validates the `(config, tag)` handle pair, rejects
/// empty or overlong content types, walks same-type states newest
/// first, then either opens the found state (mode 0) or creates one
/// (modes 1/2) through heap allocation, reader-list creation, the
/// map insert and a balancing release, and finally publishes the
/// stream handle. `id` arrives full 32-bit and narrows on store.
/// `synthdata`/`markers` are the `D_8d7764`/`D_8d77a8` windows,
/// `scratch` a 128-byte lowercase buffer. The trace logging is
/// unobserved, as are the 196794 iterator, the new key version and
/// the 196830 call. `D_14e3ac` is verified byte-identical across
/// regions, so its content is a constant.
pub fn open_stream(
    mem: &mut [u8],
    table: &[u16; 256],
    config: Option<u32>,
    tag: u32,
    id: u32,
    content: Option<u32>,
    elements: Option<u32>,
    mode: u32,
    out: Option<u32>,
    synthdata: u32,
    markers: u32,
    scratch: u32,
    scripts: &OpenScripts,
    heap: &mut (impl HeapVtable + MethodTable + BlockHeap),
    dtor: &mut impl FnMut(u32, u32),
    iter_next: &mut impl FnMut() -> (u32, Option<u32>, Option<u32>),
) -> u32 {
    const MARKERS: &[u8] = b"application/x-realspeak-markers-pp;version=4.0\0";
    const TEXT_PREFIX: &[u8] = b"text/";
    const CHARSET_TEXT: &[u8] = b"charset=wchar_t";
    const PLAIN16: &[u8] = b"text/plain;charset=utf-16\0";
    const SYNTH_SSML: &[u8] = b"application/synthesis+ssml\0";
    const XML: &[u8] = b"text/xml\0";
    if validate_handle(config.is_some(), tag, 0x47dd, 32) >= 0x80000000 {
        return 0x81706008;
    }
    if scripts.rc_611c >= 0x80000000 {
        return scripts.rc_611c;
    }
    let base = config.unwrap();
    let context = scripts.ctx_611c;
    let invalid = content.is_none()
        || mem[content.unwrap() as usize] == 0
        || out.is_none()
        || strlen_bytes(&mem[content.unwrap() as usize..]) >= 128;
    if invalid {
        return 0x88f02007;
    }
    let src = content.unwrap();
    let outbase = out.unwrap();
    mem_store(mem, outbase, 0);
    mem_store(mem, outbase + 4, 0);
    let len = strlen_bytes(&mem[src as usize..]);
    for i in 0..len as usize {
        mem[scratch as usize + i] = ascii_fold(table, mem[src as usize + i] as u32) as u8;
    }
    mem[scratch as usize + len as usize] = 0;
    let pipe = mem_half(mem, base + 8);
    let mut state = None;
    if scripts.rc794 < 0x80000000 {
        let (mut next, mut fkey, mut fstate) = iter_next();
        loop {
            if next >= 0x80000000 {
                break;
            }
            let key = fkey.unwrap();
            let found = fstate.unwrap();
            if compare_str(
                &mem[scratch as usize..],
                &mem[mem_word(mem, key) as usize..],
            ) != 0
            {
                break;
            }
            let fpipe = mem_half(mem, found + 20);
            if !(pipe > fpipe || (mode == 0 && fpipe == pipe)) {
                break;
            }
            state = Some(found);
            (next, fkey, fstate) = iter_next();
        }
    }
    let mut rc = scripts.rc794;
    if (rc & 0x1fff) == 20 {
        rc = 0;
    }
    if rc >= 0x80000000 {
        return rc;
    }
    let memory_error = 0x88f0200a;
    let state = if mode == 0 {
        match state {
            Some(found) => found,
            None => return 0x88f02014,
        }
    } else {
        if mode != 1 && mode != 2 {
            return 0x88f02007;
        }
        if let Some(found) = state {
            if mem_half(mem, found + 20) == pipe {
                return 0x88f02002;
            }
        }
        let heap_word = mem_word(mem, context + 4);
        let new = call_method(mem, nz(heap_word), 1, 72, heap);
        if new == 0 {
            return memory_error;
        }
        let mut readers = 0u32;
        let ok = create_block(
            heap,
            heap_word,
            mem_word(mem, synthdata + 0xb8),
            mem_word(mem, synthdata + 0xbc),
            mem_word(mem, synthdata + 0xc0),
            1,
            mem,
            Some(&mut readers),
        );
        if ok != 1 {
            heap_release(mem, nz(heap_word), Some(new), heap);
            return memory_error;
        }
        mem_store(mem, new + 12, readers);
        let content_new = heap_alloc(mem, nz(heap_word), len.wrapping_add(1), heap);
        mem_store(mem, new + 16, content_new);
        if content_new == 0 {
            destroy_array(mem, Some(readers), heap, &mut *dtor);
            heap_release(mem, nz(heap_word), Some(new), heap);
            return memory_error;
        }
        let use_markers = elements.is_none() && compare_str(&mem[scratch as usize..], MARKERS) == 0;
        mem_store(mem, new + 4, context);
        mem_store(mem, new, base);
        copy_str(mem, content_new, scratch);
        let at = new as usize;
        mem[at + 20..at + 22].copy_from_slice(&pipe.to_le_bytes());
        mem[at + 22..at + 24].copy_from_slice(&1u16.to_le_bytes());
        mem[at + 24..at + 26].copy_from_slice(&(id as u16).to_le_bytes());
        if use_markers {
            let from = markers as usize;
            if mem_half(mem, markers) != 0 {
                mem.copy_within(from..from + 12, at + 28);
            }
        } else if let Some(el) = elements {
            let from = el as usize;
            if mem_half(mem, el) != 0 {
                mem.copy_within(from..from + 12, at + 28);
            }
        }
        mem[at + 40] = u8::from(mode == 2);
        let text = compare_str_n(&mem[scratch as usize..], TEXT_PREFIX, 5) == 0
            && !(0..len as usize).any(|i| {
                len as usize - i >= CHARSET_TEXT.len()
                    && compare_str_n(
                        &mem[scratch as usize + i..],
                        CHARSET_TEXT,
                        CHARSET_TEXT.len() as i32,
                    ) == 0
            });
        mem[at + 41] = u8::from(text);
        if mem_word(mem, base + 28) != 0
            && (compare_str(&mem[scratch as usize..], PLAIN16) == 0
                || compare_str(&mem[scratch as usize..], SYNTH_SSML) == 0
                || compare_str(&mem[scratch as usize..], XML) == 0)
        {
            mem_store(mem, base + 28, 0);
            mem[at + 40] = 1;
            mem[at + 42] = 1;
        }
        let create_rc = scripts.rc68c;
        release_stream_state(mem, Some(new), heap_word, heap, dtor);
        if create_rc >= 0x80000000 {
            return create_rc;
        }
        rc = create_rc;
        new
    };
    let heap_word = mem_word(mem, context + 4);
    let stream = call_method(mem, nz(heap_word), 1, 12, heap);
    if stream == 0 {
        return memory_error;
    }
    if mode == 0 && scripts.rc_db0 == 0 {
        heap_release(mem, nz(heap_word), Some(stream), heap);
        return memory_error;
    }
    let at = stream as usize;
    mem[at] = mode as u8;
    mem[at + 2..at + 4].copy_from_slice(&(id as u16).to_le_bytes());
    mem_store(mem, stream + 4, state);
    mem_store(mem, stream + 8, mem_word(mem, state + 48));
    let refs = mem_half(mem, state + 22);
    let sat = state as usize;
    mem[sat + 22..sat + 24].copy_from_slice(&refs.wrapping_add(1).to_le_bytes());
    if mode == 1 || mode == 2 {
        mem_store(mem, state + 8, stream);
    }
    mem_store(mem, outbase, stream);
    mem_store(mem, outbase + 4, 0x47de);
    rc
}

/// Scripted answers for the stubbed map interfaces behind
/// [`insert_ling_entry`]: the lock/unlock, find and insert statuses
/// plus the found entry.
pub struct InsertScripts {
    /// `fn_176f2c` lock status.
    pub lock: u32,
    /// `fn_1966ac` find status.
    pub find: u32,
    /// `fn_1966ac` entry output (`None` inserts).
    pub found: Option<u32>,
    /// `fn_19668c` insert status.
    pub insert: u32,
    /// `fn_176f5c` unlock status.
    pub unlock: u32,
}

/// fn_17b0f8 / EU fn_17b1a8 / KR fn_17b154.
///
/// Inserts or updates a linguistic map entry: the three handles
/// null-check up front, then the lock runs, then a successful find
/// overwrites the found entry with 20 bytes from the pair while a
/// failed find inserts a fresh stack-built entry, and the unlock
/// status merges last. Matches `src/fn_17ae04.c:162` and the ARM,
/// which is identical across regions. The owner/map words flow only
/// into stubbed interfaces, and the fresh entry is built on the C
/// stack for the stubbed insert, so neither is reproduced here. A
/// successful find always yields an entry (real-map contract), which
/// the row's fixtures guarantee.
pub fn insert_ling_entry(
    mem: &mut [u8],
    o: Option<u32>,
    name: Option<u32>,
    pair: Option<u32>,
    scripts: &InsertScripts,
) -> u32 {
    if o.is_none() || name.is_none() || pair.is_none() {
        return 0x8b602007;
    }
    if scripts.lock >= 0x80000000 {
        return scripts.lock;
    }
    let mut rc = scripts.find;
    if scripts.find < 0x80000000 {
        let from = pair.unwrap() as usize;
        mem.copy_within(from..from + 20, scripts.found.unwrap() as usize);
    } else {
        rc = scripts.insert;
    }
    if scripts.unlock >= 0x80000000 && rc < 0x80000000 {
        rc = scripts.unlock;
    }
    rc
}

/// Scripted answers for the stubbed map interfaces behind
/// [`release_ling_entry`]: the lock/unlock, find and remove
/// statuses plus the found entry.
pub struct ReleaseScripts {
    /// `fn_176f2c` lock status.
    pub lock: u32,
    /// `fn_1966ac` find status.
    pub find: u32,
    /// `fn_1966ac` entry output (`None` removes nothing).
    pub found: Option<u32>,
    /// `fn_19672c` remove status.
    pub remove: u32,
    /// `fn_176f5c` unlock status.
    pub unlock: u32,
}

/// fn_17b1c4 / EU fn_17b274 / KR fn_17b220.
///
/// Releases one reference on a linguistic map entry: a failed find
/// remaps its status, a zero count reports `0x8b602011`, any other
/// count above one decrements, and the last reference snapshots the
/// 32-byte entry, removes it from the map, then runs the release
/// callback with the owner halves and the snapshot. Callback
/// presence comes from `cb`, not from the entry's address-valued
/// word; callers pass `None` when it is null. Matches
/// `src/fn_17ae04.c:186` and the ARM, which is identical across
/// regions. A successful find always yields an entry (real-map
/// contract), which the row's fixtures guarantee.
pub fn release_ling_entry(
    mem: &mut [u8],
    o: Option<u32>,
    name: Option<u32>,
    scripts: &ReleaseScripts,
    cb: Option<impl FnMut(u32, u32, &mut [u8; 32])>,
) -> u32 {
    if o.is_none() || name.is_none() {
        return 0x8b602007;
    }
    if scripts.lock >= 0x80000000 {
        return scripts.lock;
    }
    let mut rc = scripts.find;
    if scripts.find >= 0x80000000 {
        rc = (scripts.find & 0x1fff) | 0x8b602000;
    } else {
        let e = scripts.found.unwrap() as usize;
        let refs = u16::from_le_bytes(mem[e + 28..e + 30].try_into().unwrap());
        if refs == 1 {
            if let Some(mut cb) = cb {
                let mut tmp = [0u8; 32];
                tmp.copy_from_slice(&mem[e..e + 32]);
                rc = scripts.remove;
                let o = o.unwrap() as usize;
                let h0 = u32::from_le_bytes(mem[o..o + 4].try_into().unwrap());
                let h1 = u32::from_le_bytes(mem[o + 4..o + 8].try_into().unwrap());
                cb(h0, h1, &mut tmp);
            } else {
                mem[e + 28..e + 30].copy_from_slice(&0u16.to_le_bytes());
            }
        } else if refs == 0 {
            rc = 0x8b602011;
        } else {
            mem[e + 28..e + 30].copy_from_slice(&refs.wrapping_sub(1).to_le_bytes());
        }
    }
    if scripts.unlock >= 0x80000000 && rc < 0x80000000 {
        rc = scripts.unlock;
    }
    rc
}

/// fn_17b2b0 / EU fn_17b360 / KR fn_17b30c.
///
/// Drops an unreferenced linguistic map entry: with no map the
/// lock status flows through, a referenced entry reports
/// `0x8b602011`, and only a zero count reaches the remove. Reads
/// the map word and the entry's refcount; writes nothing. Matches
/// `src/fn_17ae04.c:222` and the ARM, which is identical across
/// regions. A successful find always yields an entry (real-map
/// contract), which the row's fixtures guarantee.
pub fn drop_ling_entry(
    mem: &[u8],
    o: Option<u32>,
    name: Option<u32>,
    scripts: &ReleaseScripts,
) -> u32 {
    if o.is_none() || name.is_none() {
        return 0x8b602007;
    }
    if scripts.lock >= 0x80000000 {
        return scripts.lock;
    }
    let o = o.unwrap() as usize;
    let map = u32::from_le_bytes(mem[o + 16..o + 20].try_into().unwrap());
    let mut rc = scripts.lock;
    if map != 0 {
        rc = scripts.find;
        if scripts.find < 0x80000000 {
            let e = scripts.found.unwrap() as usize;
            let refs = u16::from_le_bytes(mem[e + 28..e + 30].try_into().unwrap());
            rc = if refs != 0 {
                0x8b602011
            } else {
                scripts.remove
            };
        }
    }
    if scripts.unlock >= 0x80000000 && rc < 0x80000000 {
        rc = scripts.unlock;
    }
    rc
}

/// Scripted answers for the stubbed interfaces behind
/// [`create_ling_table`]: the acquire/alloc/owner-open/map-create
/// statuses plus their outputs.
pub struct CreateScripts {
    /// `fn_19611c` context-acquire status.
    pub acquire: u32,
    /// `fn_19611c` context output.
    pub ctx: Option<u32>,
    /// `fn_1963b8` allocation output (`None` reports `0x8b60200a`).
    pub alloc: Option<u32>,
    /// `fn_176e40` owner-open status.
    pub owner_rc: u32,
    /// `fn_176e40` owner output (written before the status check,
    /// like the stub).
    pub owner: u32,
    /// `fn_1964e0` map-create status.
    pub create: u32,
    /// `fn_1964e0` map output (written before the status check,
    /// like the stub).
    pub map: u32,
}

/// fn_17ae04 / EU fn_17aeb4 / KR fn_17ae60.
///
/// Creates a linguistic table object: acquires the context,
/// allocates and publishes the object, opens the owner, creates
/// the map. A failed owner open frees the object and reports the
/// status; a failed map create destroys the owner, frees the
/// object and reports `0x8b60200a`. The object handle re-reads
/// from `out` at each step like the source; the ROM's map-config
/// words flow only into the stubbed create. Matches
/// `src/fn_17ae04.c:71` and the ARM, which is identical across
/// regions. A successful acquire always yields a context
/// (real-context contract), which the row's fixtures guarantee.
pub fn create_ling_table(
    mem: &mut [u8],
    h: [u32; 2],
    out: Option<u32>,
    scripts: &CreateScripts,
    mut free: impl FnMut(),
    mut destroy_owner: impl FnMut(u32),
) -> u32 {
    if scripts.acquire >= 0x80000000 {
        return scripts.acquire;
    }
    let Some(out) = out else {
        return 0x8b602007;
    };
    let Some(o) = scripts.alloc else {
        mem_store(mem, out, 0);
        return 0x8b60200a;
    };
    mem_store(mem, out, o);
    mem_store(mem, o, h[0]);
    mem_store(mem, o + 4, h[1]);
    mem_store(mem, mem_word(mem, out) + 8, scripts.ctx.unwrap());
    mem_store(mem, mem_word(mem, out) + 12, scripts.owner);
    if scripts.owner_rc >= 0x80000000 {
        free();
        mem_store(mem, out, 0);
        return scripts.owner_rc;
    }
    mem_store(mem, mem_word(mem, out) + 16, scripts.map);
    if scripts.create >= 0x80000000 {
        destroy_owner(mem_word(mem, mem_word(mem, out) + 12));
        free();
        mem_store(mem, out, 0);
        return 0x8b60200a;
    }
    0
}

/// Scripted answers for the stubbed teardown interfaces behind
/// [`close_worker`]: the lock/unlock, context acquire and every
/// subsystem release status.
pub struct TeardownScripts {
    /// `fn_176f2c` lock status.
    pub lock: u32,
    /// `fn_176f5c` unlock status.
    pub unlock: u32,
    /// `fn_19611c` context-acquire status.
    pub acquire: u32,
    /// `fn_19611c` context output.
    pub ctx: Option<u32>,
    /// `fn_17b1c4` LINGDB release status (overwrites on failure).
    pub lingdb: u32,
    /// `fn_160b54` worker release status.
    pub worker: u32,
    /// `fn_14dc30` log release status.
    pub log: u32,
    /// `fn_17b2b0` PHONMAP drop status.
    pub phonmap: u32,
    /// `fn_17b2b0` SYNTHSTREAM drop status.
    pub synthstream: u32,
    /// `fn_17af38` linguistic teardown status.
    pub teardown: u32,
    /// `fn_174e00` parameters release status.
    pub params: u32,
    /// `fn_177cc8` synthesis release status.
    pub synthfin: u32,
    /// `fn_176edc` mutex destroy status.
    pub mutex: u32,
    /// `fn_196238` engine release status.
    pub engine: u32,
}

/// fn_190edc / EU fn_190f8c / KR fn_190fc8.
///
/// Closes a synthesis worker: validates the tag and magic, locks
/// to sample the state, then tears down the linguistic, worker,
/// voice, log, parameters, synthesis, mutex and engine pieces in
/// order, publishing the completion callback last. A LINGDB
/// failure overwrites the running status and the voice release
/// assigns it directly; every other failure merges first-wins.
/// Early exits report raw codes, late ones map through
/// [`classify_code`]. Matches `src/fn_190edc.c:73` and the ARM,
/// which is identical across regions. A successful acquire always
/// yields a context (real-context contract), which the row's
/// fixtures guarantee.
pub fn close_worker(
    mem: &mut [u8],
    handle: Option<u32>,
    tag: u32,
    scripts: &TeardownScripts,
    mut release: impl FnMut(u32, u32) -> u32,
    mut complete: impl FnMut(u32, u32),
) -> u32 {
    let Some(handle) = handle else {
        return 0x80000008;
    };
    if tag != 0x684f424a || mem_word(mem, handle) != 0x734f424a {
        return 0x80000008;
    }
    if scripts.lock >= 0x80000000 {
        return classify_code(scripts.lock);
    }
    let state = mem[(handle + 40) as usize];
    if scripts.unlock >= 0x80000000 {
        return classify_code(scripts.unlock);
    }
    if state != 1 {
        return 0x80000011;
    }
    mem_store(mem, handle, 0x8cb0bdb5);
    let mut status = scripts.acquire;
    if status >= 0x80000000 {
        return classify_code(status);
    }
    let ctx = scripts.ctx.unwrap();
    let parent = mem_word(mem, handle + 4);
    if mem_word(mem, handle + 0x1a0) != 0 {
        let result = scripts.lingdb;
        if result >= 0x80000000 {
            status = result;
        }
    }
    if mem_word(mem, handle + 0x1a8) != 0 {
        let result = scripts.worker;
        if status < 0x80000000 && result >= 0x80000000 {
            status = result;
        }
    }
    let voice_first = mem_word(mem, handle + 44);
    if voice_first != 0 {
        status = release(voice_first, mem_word(mem, handle + 48));
    }
    if mem_word(mem, ctx + 16) != 0 {
        let result = scripts.log;
        if status < 0x80000000 && result >= 0x80000000 {
            status = result;
        }
        mem_store(mem, ctx + 16, 0);
    }
    if mem_word(mem, ctx + 24) != 0 {
        if mem_word(mem, parent + 0xd8) != 0 {
            let result = scripts.phonmap;
            if status < 0x80000000 && result >= 0x80000000 {
                status = result;
            }
        }
        if mem_word(mem, parent + 0xd4) != 0 {
            let result = scripts.synthstream;
            if status < 0x80000000 && result >= 0x80000000 {
                status = result;
            }
        }
        let result = scripts.teardown;
        if status < 0x80000000 && result >= 0x80000000 {
            status = result;
        }
        mem_store(mem, ctx + 24, 0);
    }
    if mem_word(mem, ctx + 20) != 0 {
        let result = scripts.params;
        if status < 0x80000000 && result >= 0x80000000 {
            status = result;
        }
        mem_store(mem, ctx + 20, 0);
    }
    if mem_word(mem, ctx + 12) != 0 {
        let result = scripts.synthfin;
        if status < 0x80000000 && result >= 0x80000000 {
            status = result;
        }
        mem_store(mem, ctx + 12, 0);
    }
    if mem_word(mem, handle + 36) != 0 {
        let result = scripts.mutex;
        if status < 0x80000000 && result >= 0x80000000 {
            status = result;
        }
        mem_store(mem, handle + 36, 0);
    }
    if mem_word(mem, handle + 8) != 0 {
        let result = scripts.engine;
        if status < 0x80000000 && result >= 0x80000000 {
            status = result;
        }
    }
    let count = mem_word(mem, parent + 0xdc);
    mem_store(mem, parent + 0xdc, count.wrapping_sub(1));
    complete(mem_word(mem, handle + 20), handle);
    classify_code(status)
}

/// Scripted answers for the stubbed open interfaces behind
/// [`open_worker`]: the task/stream/mutex/engine/context
/// bring-up, every subsystem publish status and the allocation.
pub struct OpenWorkerScripts {
    /// `fn_196318` task status.
    pub task_rc: u32,
    /// `fn_177b7c` stream status.
    pub stream_rc: u32,
    /// `fn_176e40` mutex status.
    pub mutex_rc: u32,
    /// `fn_176e40` mutex output (written before the status check,
    /// like the stub).
    pub mutex: u32,
    /// `fn_1961d0` engine status.
    pub engine_rc: u32,
    /// `fn_1961d0` engine output (written before the status check,
    /// like the stub).
    pub engine: [u32; 2],
    /// `fn_19611c` context-acquire status.
    pub acquire: u32,
    /// `fn_19611c` context output.
    pub ctx: Option<u32>,
    /// `fn_14dc94` log output (its status is ignored).
    pub log: u32,
    /// `fn_174ca8` parameters status.
    pub params_rc: u32,
    /// `fn_174ca8` parameters output (written before the status
    /// check, like the stub).
    pub params: u32,
    /// `fn_17ae04` linguistic status.
    pub ling_rc: u32,
    /// `fn_17ae04` linguistic output (written before the status
    /// check, like the stub).
    pub ling: u32,
    /// `fn_1758ac` property status (both calls share it).
    pub prop: u32,
    /// `fn_1759a0` name status.
    pub name: u32,
    /// `fn_1757b8` marker status.
    pub mrk: u32,
    /// `fn_17b0f8` insert status (both calls share it).
    pub insert: u32,
    /// Voice `setup` status.
    pub setup_rc: u32,
    /// Voice `setup` voice-pair output (written before the status
    /// check, like the hook).
    pub voice: [u32; 2],
    /// `fn_160b0c` worker status.
    pub worker_rc: u32,
    /// `fn_160b0c` worker output (written before the status check,
    /// like the stub).
    pub worker: u32,
    /// Factory `allocate` output (`None` reports `0x8000000a`).
    pub alloc: Option<u32>,
}

fn open_cleanup(
    mem: &mut [u8],
    state: u32,
    owner: u32,
    status: u32,
    teardown: &mut impl FnMut(),
    release: &mut impl FnMut(),
) -> u32 {
    if mem_word(mem, state + 16) != 0 {
        teardown();
    } else {
        release();
        mem_store(
            mem,
            owner + 220,
            mem_word(mem, owner + 220).wrapping_sub(1),
        );
    }
    classify_code(status)
}

/// fn_19126c / EU fn_19131c / KR fn_191358.
///
/// Opens a synthesis worker: validates the factory handle,
/// allocates the lifecycle object, brings up the task, stream,
/// mutex, engine and context, publishes parameters, linguistic
/// data, properties and voice, then starts the worker and hands
/// the pair back. Any failure unwinds through the full teardown
/// when a context exists, else releases the object. Matches
/// `src/fn_19126c.c:135` and the ARM across regions (EU/KR
/// replace the US-only completion hook with a stack spill of
/// identical size). A successful acquire always yields a context
/// (real-context contract), which the row's fixtures guarantee.
#[allow(clippy::too_many_arguments)]
pub fn open_worker(
    mem: &mut [u8],
    owner: Option<u32>,
    tag: u32,
    user: u32,
    argument: u32,
    out: u32,
    flags: u32,
    scripts: &OpenWorkerScripts,
    mut teardown: impl FnMut(),
    mut release: impl FnMut(),
) -> u32 {
    if tag != 0x68434c53 {
        return 0x80000008;
    }
    // The source dereferences the owner before its null check, so a
    // null owner with a passing tag faults; fixtures never do that.
    let owner = owner.unwrap();
    if mem_word(mem, owner) != 0x73434c53 {
        return 0x80000008;
    }
    let Some(state) = scripts.alloc else {
        return 0x8000000a;
    };
    mem_store(mem, state + 20, user);
    mem_store(mem, state + 24, mem_word(mem, owner + 156));
    mem_store(mem, state + 28, mem_word(mem, owner + 160));
    mem_store(mem, state + 32, argument);
    mem_store(
        mem,
        owner + 220,
        mem_word(mem, owner + 220).wrapping_add(1),
    );
    let mut status = scripts.task_rc;
    if status >= 0x80000000 {
        return open_cleanup(mem, state, owner, status, &mut teardown, &mut release);
    }
    mem_store(mem, state + 4, owner);
    mem_store(mem, state + 0x184, flags);
    mem_store(mem, state + 0x188, 0);
    mem_store(mem, state + 0x18c, 0);
    mem_store(mem, state, 0x734f424a);
    mem_store(mem, state + 0x1a8, 0);
    mem[(state + 52) as usize] = 0xc;
    mem[(state + 53) as usize] = 1;
    status = scripts.stream_rc;
    if status >= 0x80000000 {
        return open_cleanup(mem, state, owner, status, &mut teardown, &mut release);
    }
    mem_store(mem, state + 36, scripts.mutex);
    status = scripts.mutex_rc;
    if status >= 0x80000000 {
        return open_cleanup(mem, state, owner, status, &mut teardown, &mut release);
    }
    mem_store(mem, state + 8, scripts.engine[0]);
    mem_store(mem, state + 12, scripts.engine[1]);
    status = scripts.engine_rc;
    if status < 0x80000000 {
        // The stub publishes its output before the status check, so
        // a failed acquire can still plant a null context.
        mem_store(mem, state + 16, scripts.ctx.unwrap_or(0));
        status = scripts.acquire;
    }
    if status >= 0x80000000 {
        return open_cleanup(mem, state, owner, status, &mut teardown, &mut release);
    }
    let ctx = mem_word(mem, state + 16);
    let manager = mem_word(mem, owner + 176);
    mem_store(mem, ctx + 28, mem_word(mem, manager + 28));
    mem[(state + 40) as usize] = 1;
    if mem_word(mem, owner + 180) != 0 {
        mem_store(mem, ctx + 16, scripts.log);
    }
    mem_store(mem, ctx + 20, scripts.params);
    status = scripts.params_rc;
    if status >= 0x80000000 {
        return open_cleanup(mem, state, owner, status, &mut teardown, &mut release);
    }
    mem_store(mem, ctx + 24, scripts.ling);
    status = scripts.ling_rc;
    if status >= 0x80000000 {
        return open_cleanup(mem, state, owner, status, &mut teardown, &mut release);
    }
    status = scripts.prop;
    if status < 0x80000000 {
        status = scripts.prop;
    }
    if status < 0x80000000 {
        status = scripts.name;
    }
    if status < 0x80000000 {
        status = scripts.mrk;
    }
    if status >= 0x80000000 {
        return open_cleanup(mem, state, owner, status, &mut teardown, &mut release);
    }
    status = scripts.insert;
    if status >= 0x80000000 {
        return open_cleanup(mem, state, owner, status, &mut teardown, &mut release);
    }
    status = scripts.insert;
    if status >= 0x80000000 {
        return open_cleanup(mem, state, owner, status, &mut teardown, &mut release);
    }
    mem_store(mem, state + 44, scripts.voice[0]);
    mem_store(mem, state + 48, scripts.voice[1]);
    status = scripts.setup_rc;
    if status >= 0x80000000 {
        return open_cleanup(mem, state, owner, status, &mut teardown, &mut release);
    }
    mem_store(mem, state + 0x1a8, scripts.worker);
    status = scripts.worker_rc;
    if status >= 0x80000000 {
        return open_cleanup(mem, state, owner, status, &mut teardown, &mut release);
    }
    mem_store(mem, out, state);
    mem_store(mem, out + 4, 0x684f424a);
    0
}

/// fn_154108 / EU fn_1541cc / KR fn_1540ec.
///
/// Ends components `start..end`: stamps the cursor, fires the begin
/// hook per component, closes the marker stream for the component
/// whose name matches the pending name while closing, ends it,
/// fires the end hook, and keeps the first component error. A live
/// scratch block is released and cleared. Matches
/// `src/fn_151914.c:1241` and the ARM, which is identical across
/// regions. The C takes a `u16` start but the ARM compares the full
/// register without narrowing, so the port takes `u32` (ARM width);
/// callers pass small component indices and the row's fixtures stay
/// within bounds. The log calls are dropped (unobserved); the
/// begin/end hooks stay as callbacks because the scheduler replay
/// records them.
#[allow(clippy::too_many_arguments)]
pub fn close_components(
    mem: &mut [u8],
    synth: u32,
    start: u32,
    end: u32,
    mut begin: impl FnMut(u32, u32, u32),
    mut close: impl FnMut(u32, u32),
    mut process_end: impl FnMut(u32, u32) -> u32,
    mut end_stage: impl FnMut(u32, u32, u32),
) -> u32 {
    let at = (synth + 0xd2) as usize;
    mem[at..at + 2].copy_from_slice(&(start as u16).to_le_bytes());
    let cs = mem_word(mem, synth + 0x7c);
    let comps = mem_word(mem, cs + 8);
    let obj = mem_word(mem, cs + 16);
    let arg = mem_word(mem, cs + 20);
    let status = mem[(synth + 0x80) as usize];
    let pending = terminated(mem, synth + 0x118);
    let mark_a = mem_word(mem, synth + 0x4c);
    let mark_b = mem_word(mem, synth + 0x50);
    let mut rc = 0u32;
    let mut s = start;
    while s < end {
        if obj != 0 {
            begin(obj, arg, s.wrapping_add(2));
        }
        let comp = mem_word(mem, comps.wrapping_add(s.wrapping_mul(4)));
        if status == 2 && compare_str(terminated(mem, mem_word(mem, comp + 4)), pending) == 0 {
            close(mark_a, mark_b);
        }
        let r2 = process_end(mem_word(mem, comp + 12), mem_word(mem, comp + 16));
        if obj != 0 {
            end_stage(obj, arg, s.wrapping_add(2));
        }
        if r2 >= 0x80000000 && rc < 0x80000000 {
            rc = r2;
        }
        s = s.wrapping_add(1);
    }
    if mem_word(mem, synth + 0x114) != 0 {
        mem_store(mem, synth + 0x114, 0);
    }
    rc
}

/// fn_153c5c / EU fn_153d20 / KR fn_153c40.
///
/// Starts components `start..end`: clears the pending name, scans
/// the pipeline for the post-processor window while closing,
/// then per component sets the stage, fires the begin hook, opens
/// the marker/text streams on name match while closing, starts
/// it, fires the end hook, keeps a start failure, else stamps the
/// cursor and reports through the callback. While closing the
/// scratch block is dropped and the marker list is reallocated
/// and filled. Matches `src/fn_151914.c:1149` and the ARM, which
/// is identical across regions. The C takes a `u16` start but the
/// ARM compares the full register without narrowing, so the port
/// takes `u32` (ARM width); callers pass small component indices
/// and the row's fixtures stay within bounds. The log calls are
/// dropped (unobserved); the list walk is skipped (its reads are
/// unobservable and its frees ignored) while the realloc keeps a
/// `None` fault like the C. The stage hooks stay as callbacks
/// because the scheduler replay records them.
#[allow(clippy::too_many_arguments)]
pub fn start_components(
    mem: &mut [u8],
    synth: u32,
    start: u32,
    end: u32,
    alloc: Option<u32>,
    cb_arg: u32,
    mut set_stage: impl FnMut(u32, u32, u32) -> u32,
    mut begin: impl FnMut(u32, u32, u32),
    mut open: impl FnMut(u32, u32, u32, &[u8], u32, u32) -> ([u32; 2], u32),
    mut process_start: impl FnMut(u32, u32, u32, u32) -> u32,
    mut end_stage: impl FnMut(u32, u32, u32),
    cb: Option<impl FnMut(u32, u32, u32, u32) -> u32>,
) -> u32 {
    const MARKERS: &[u8] = b"application/x-realspeak-markers-pp;version=4.0\0";
    const TEXT_UTF16: &[u8] = b"text/plain;charset=utf-16\0";
    const PP_TEXT: &[u8] = b"pp/text\0";
    const FE_LID: &[u8] = b"fe/fe_lid\0";
    let out = synth + 0x84;
    let pending = out + 148;
    let cs = mem_word(mem, synth + 0x7c);
    let comps = mem_word(mem, cs + 8);
    let count = mem_half(mem, cs + 12) as u32;
    let status = mem[(synth + 0x80) as usize];
    let obj = mem_word(mem, cs + 16);
    let arg = mem_word(mem, cs + 20);
    let pipe_a = mem_word(mem, out + 16);
    let pipe_b = mem_word(mem, out + 20);
    mem[pending as usize] = 0;
    mem_store16(mem, out + 180, 0);
    mem_store16(mem, out + 182, 0);
    let mut rc = 0u32;
    if status == 2 {
        let mut i = 0u32;
        while i < count {
            let comp = mem_word(mem, comps.wrapping_add(i.wrapping_mul(4)));
            if compare_str(terminated(mem, mem_word(mem, comp + 4)), PP_TEXT) == 0 {
                mem_store16(mem, out + 180, i as u16);
                mem_store16(mem, out + 182, i as u16);
                let comp1 =
                    mem_word(mem, comps.wrapping_add(i.wrapping_add(1).wrapping_mul(4)));
                if compare_str(terminated(mem, mem_word(mem, comp1 + 4)), FE_LID) == 0 {
                    mem_store16(mem, out + 182, i.wrapping_add(1) as u16);
                    let comp2 = mem_word(
                        mem,
                        comps.wrapping_add(i.wrapping_add(2).wrapping_mul(4)),
                    );
                    copy_str(mem, pending, mem_word(mem, comp2 + 4));
                } else {
                    copy_str(mem, pending, mem_word(mem, comp1 + 4));
                }
            }
            i += 1;
        }
    }
    let mut cb = cb;
    let mut s = start;
    while rc < 0x80000000 && s < end {
        let next = s.wrapping_add(1);
        // Both verdicts are discarded, matching the C; the
        // signatures still carry them for the row's slots.
        let _ = set_stage(pipe_a, pipe_b, next & 0xffff);
        let comp = mem_word(mem, comps.wrapping_add(s.wrapping_mul(4)));
        let name = mem_word(mem, comp + 4);
        if obj != 0 {
            begin(obj, arg, s.wrapping_add(2));
        }
        if status == 2 {
            if compare_str(terminated(mem, name), terminated(mem, pending)) == 0 {
                let (pair, _) = open(pipe_a, pipe_b, 18, MARKERS, 0, 0);
                mem_store(mem, synth + 0x4c, pair[0]);
                mem_store(mem, synth + 0x50, pair[1]);
            }
            if compare_str(terminated(mem, name), PP_TEXT) == 0 {
                let (pair, _) = open(pipe_a, pipe_b, 18, TEXT_UTF16, 0, 0);
                mem_store(mem, synth + 0x44, pair[0]);
                mem_store(mem, synth + 0x48, pair[1]);
                mem_store(mem, out + 124, 0);
            }
        }
        let r2 = process_start(
            mem_word(mem, comp + 12),
            mem_word(mem, comp + 16),
            pipe_a,
            pipe_b,
        );
        if obj != 0 {
            end_stage(obj, arg, s.wrapping_add(2));
        }
        rc = r2;
        if r2 >= 0x80000000 {
            // Error log dropped (unobserved).
        } else {
            mem_store16(mem, out + 78, next as u16);
            if let Some(cb) = cb.as_mut() {
                rc = cb(cb_arg, name, pipe_a, pipe_b);
            }
        }
        s = next;
    }
    if status == 2 {
        mem_store(mem, out + 144, 0);
        let a = alloc.unwrap();
        mem_store(mem, out + 188, a);
        fill_bytes(mem, a, 0, 0x84);
        for k in 0..5 {
            mem_store(mem, a + 4 + k * 4, 9999);
        }
    }
    rc
}

/// Scripted leaf values behind [`start_synthesis`]: the voice-pair
/// loader, independent frequency and sample-bits queries,
/// the string query, the lexicon bind, the
/// mime check, the scratch and list allocators, and the pipe
/// binder with its prep word.
pub struct StartSynthesisScripts {
    pub t_pair: [u32; 2],
    pub freq_rc: u32,
    pub freq_val: u32,
    pub bits_rc: u32,
    pub bits_val: u32,
    pub q_rc: u32,
    pub q_out: Option<u32>,
    pub lex_rc: u32,
    pub lex: u32,
    pub mime_rc: u32,
    pub extra: Option<u32>,
    pub alloc: Option<u32>,
    pub bind_rc: u32,
    pub prep: u32,
}

/// fn_15668c / EU fn_156750 / KR fn_156670.
///
/// Binds the synthesis output: license gate, output clear,
/// scratch alloc, rate setup, stats note, parameter log, pipe
/// and aux creation, mime check, post-pair open, component
/// start, output open, callback bind, then flush or return.
/// Matches `src/fn_151914.c:2157` and the ARM, which is
/// identical across regions. The ROM patches the license and
/// stats calls out (always-licensed, stats verdict 0), which
/// the port mirrors; the component starter runs through
/// [`start_components`]. Fail edges report through `fail` and
/// the done-path flusher through `flush`; the row's scripts
/// keep both unreached (a null scratch takes the early return,
/// and status != 2 forces the shared open pair null so the
/// flusher is skipped). The log calls are dropped
/// (unobserved); the `& 0x1fff == 2` open exemption only gates
/// a dropped log, so a failing post-pair open fails directly.
#[allow(clippy::too_many_arguments)]
pub fn start_synthesis(
    mem: &mut [u8],
    ctx: u32,
    synth: u32,
    src: [u32; 2],
    mime: Option<u32>,
    cb_arg: u32,
    pipe: [u32; 2],
    aux: [u32; 2],
    scripts: &StartSynthesisScripts,
    mut set_stage: impl FnMut(u32, u32, u32) -> u32,
    mut begin: impl FnMut(u32, u32, u32),
    mut open: impl FnMut(u32, u32, u32, &[u8], u32, u32) -> ([u32; 2], u32),
    mut create: impl FnMut(u32, u32, u32, u32) -> ([u32; 2], u32),
    mut process_start: impl FnMut(u32, u32, u32, u32) -> u32,
    mut end_stage: impl FnMut(u32, u32, u32),
    mut flush: impl FnMut(&mut [u8], u32, u32) -> u32,
    mut fail: impl FnMut(u32, u32, u32) -> u32,
    mut cb: Option<impl FnMut(u32, u32, u32, u32) -> u32>,
) -> u32 {
    const D154094: &[u8] = b"application/x-realspeak-markers-pp;version=4.0\0";
    let out = synth + 0x84;
    let cs = mem_word(mem, synth + 0x7c);
    let count = mem_half(mem, cs + 12) as u32;
    let status = mem[(synth + 0x80) as usize];
    let lic_kind = mem[(synth + 0x69) as usize];
    if lic_kind <= 1 {
        let lic_state = mem[(synth + 0x6a) as usize];
        if lic_state != 1 && lic_state != 2 {
            return 0x81203d06;
        }
    } else if lic_kind != 2 {
        return 0x81203d06;
    }
    fill_bytes(mem, out, 0, 188);
    mem_store(mem, out, ctx);
    mem_store(mem, out + 52, src[0]);
    mem_store(mem, out + 56, src[1]);
    mem[(out + 76) as usize] = 1;
    mem_store(mem, out + 88, scripts.extra.unwrap_or(0));
    if scripts.extra.is_none() {
        return 0x8120200a;
    }
    mem_store16(mem, out + 92, count as u16);
    let mut rc = 0x81203d06u32;
    if scripts.freq_rc >= 0x80000000 {
        mem_store(mem, out + 4, 0);
    } else {
        mem_store(mem, out + 4, scripts.freq_val);
        let bits = if scripts.bits_rc >= 0x80000000 {
            16
        } else {
            scripts.bits_val
        };
        mem_store(mem, out + 8, bits);
    }
    if pipe[0] != 0 {
        mem_store(mem, out + 16, pipe[0]);
        mem_store(mem, out + 20, pipe[1]);
    } else {
        let h0 = mem_word(mem, synth);
        let h1 = mem_word(mem, synth + 4);
        let (pair, crc) = create(scripts.t_pair[0], scripts.t_pair[1], h0, h1);
        mem_store(mem, out + 16, pair[0]);
        mem_store(mem, out + 20, pair[1]);
        rc = crc;
        if crc >= 0x80000000 {
            return fail(1, 0, 0);
        }
        mem[(out + 24) as usize] = 1;
    }
    if aux[0] != 0 {
        mem_store(mem, out + 28, aux[0]);
        mem_store(mem, out + 32, aux[1]);
    } else {
        let h0 = mem_word(mem, synth);
        let h1 = mem_word(mem, synth + 4);
        let (pair, crc) = create(scripts.t_pair[0], scripts.t_pair[1], h0, h1);
        mem_store(mem, out + 28, pair[0]);
        mem_store(mem, out + 32, pair[1]);
        rc = crc;
        if crc >= 0x80000000 {
            return fail(1, 0, 0);
        }
        mem[(out + 36) as usize] = 1;
    }
    if let Some(m) = mime {
        if mem[m as usize] != 0 {
            rc = scripts.mime_rc;
            if scripts.mime_rc >= 0x80000000 {
                return fail(1, 0, 0);
            }
        }
    }
    rc = scripts.lex_rc;
    if scripts.lex_rc >= 0x80000000 {
        return fail(1, 0, 0);
    }
    mem_store(mem, out + 48, scripts.lex);
    if src[0] != 0 {
        let mut buf = [0u8; 128];
        let mt: &[u8] = match mime {
            Some(m) if mem[m as usize] != 0 => terminated(mem, m),
            _ => match scripts.q_out {
                Some(q)
                    if scripts.q_rc < 0x80000000 && mem[q as usize] != 0 =>
                {
                    terminated(mem, q)
                }
                _ => match scripts.q_out {
                    Some(q)
                        if scripts.q_rc < 0x80000000 && mem[q as usize] != 0 =>
                    {
                        let nat = terminated(mem, q);
                        buf[..19].copy_from_slice(b"text/plain;charset=");
                        buf[19..19 + nat.len()].copy_from_slice(nat);
                        terminated(&buf, 0)
                    }
                    _ => {
                        buf[..11].copy_from_slice(b"text/plain\0");
                        terminated(&buf, 0)
                    }
                },
            },
        };
        let pa = mem_word(mem, out + 16);
        let pb = mem_word(mem, out + 20);
        let (pair, orc) = open(pa, pb, 18, mt, 0, 1);
        mem_store(mem, out + 40, pair[0]);
        mem_store(mem, out + 44, pair[1]);
        rc = orc;
        if orc >= 0x80000000 {
            return fail(1, 0, 0);
        }
    }
    let put_output = mem_word(mem, synth + 0x18);
    rc = start_components(
        mem,
        synth,
        0,
        count,
        scripts.alloc,
        cb_arg,
        |a, b, c| set_stage(a, b, c),
        |o, a, i| begin(o, a, i),
        |a, b, k, m, x, y| open(a, b, k, m, x, y),
        |a, h, p0, p1| process_start(a, h, p0, p1),
        |o, a, i| end_stage(o, a, i),
        cb.as_mut(),
    );
    if rc >= 0x80000000 {
        return fail(1, 0, 0);
    }
    if put_output != 0 {
        let pa = mem_word(mem, out + 16);
        let pb = mem_word(mem, out + 20);
        let ssrc = set_stage(pa, pb, count.wrapping_add(1) & 0xffff);
        rc = ssrc;
        if ssrc >= 0x80000000 {
            return fail(1, 0, 0);
        }
        if scripts.q_rc < 0x80000000 {
            if let Some(mq) = scripts.q_out {
                if mem[mq as usize] != 0 {
                    let (pair, porc) =
                        open(pa, pb, 18, terminated(mem, mq), 0, 0);
                    mem_store(mem, out + 60, pair[0]);
                    mem_store(mem, out + 64, pair[1]);
                    if porc < 0x80000000 {
                        let (pair, _) = open(pa, pb, 18, D154094, 0, 0);
                        mem_store(mem, out + 68, pair[0]);
                        mem_store(mem, out + 72, pair[1]);
                    }
                }
            }
        }
    } else {
        rc = 0;
    }
    if let Some(cb) = cb.as_mut() {
        let pa = mem_word(mem, out + 16);
        let pb = mem_word(mem, out + 20);
        let ssrc = set_stage(pa, pb, count.wrapping_add(1) & 0xffff);
        rc = ssrc;
        if ssrc >= 0x80000000 {
            return fail(1, 0, 0);
        }
        rc = cb(cb_arg, 0, pa, pb);
    }
    if rc < 0x80000000 && mem_word(mem, out + 40) != 0 {
        mem_store(mem, out + 108, scripts.prep);
        rc = scripts.bind_rc;
        if rc & 0x1fff == 3 {
            rc = 0;
        }
    }
    if status != 2 {
        if rc >= 0x80000000 {
            return fail(1, 0, 0);
        }
        if mem_word(mem, out + 60) == 0 {
            return rc;
        }
        rc = flush(mem, synth, 0);
        if rc < 0x80000000 {
            return rc;
        }
        return fail(1, 0, 0);
    }
    if rc < 0x80000000 {
        return rc;
    }
    fail(1, 0, 0)
}

/// ROM `D_` string bytes read by `fn_157a80` and its in-unit
/// callees (identical on US/EU/KR, verified from the images).
/// The log-only names stay host dummies in the harness and are
/// not needed here.
pub const CREATE_SYNTHSTREAM: &[u8] = b"SYNTHSTREAM";
pub const CREATE_SYNTHSTREAM_LO: &[u8] = b"synthstream";
pub const CREATE_LINGDB: &[u8] = b"LINGDB";
pub const CREATE_LINGDB_LO: &[u8] = b"lingdb";
pub const CREATE_TTSEG: &[u8] = b"TTSEG";
pub const CREATE_INET: &[u8] = b"INET";
pub const CREATE_COMPQUERY: &[u8] = b"COMPQUERY";
pub const CREATE_SINK: &[u8] = b"SYNTHOUTPUTSINK";
pub const CREATE_PITCH: &[u8] = b"pitch";
pub const CREATE_RATE: &[u8] = b"rate";
pub const CREATE_EMPTY: &[u8] = b"";
pub const CREATE_VOLUME: &[u8] = b"volume";
pub const CREATE_WAITFACTOR: &[u8] = b"waitfactor";
pub const CREATE_READMODE: &[u8] = b"readmode";
pub const CREATE_MARKERMODE: &[u8] = b"markermode";
pub const CREATE_LOOPLIMIT: &[u8] = b"infiniteloopdetectlimit";
pub const CREATE_INPUTMIME: &[u8] = b"inputmimetype";
pub const CREATE_AUDIOBUF: &[u8] = b"audiooutputbufsamples";
pub const CREATE_LICMODE: &[u8] = b"licensingmode";
pub const CREATE_DEFAULT: &[u8] = b"default";
pub const CREATE_NUANKEY: &[u8] = b"__NUAN_alternative_esc_seq__";
pub const CREATE_COMPANY: &[u8] = b"companyname";
pub const CREATE_APPNAME: &[u8] = b"applicationname";
pub const CREATE_VOICEMODEL: &[u8] = b"voicemodel";
/// Compare-only literals, NUL-terminated for the fold loops.
pub const CREATE_CPR_Z: &[u8] = b"cpr\0";
pub const CREATE_USELECT_Z: &[u8] = b"uselect/\0";
pub const CREATE_SELECT_Z: &[u8] = b"select/\0";
pub const CREATE_SELCPR_Z: &[u8] = b"select/cpr\0";
/// Callback-registration keys (`D_8d7974` order, ROM bytes).
pub const CREATE_CBKEYS: [&[u8]; 7] = [
    CREATE_LOOPLIMIT,
    CREATE_MARKERMODE,
    CREATE_READMODE,
    CREATE_WAITFACTOR,
    CREATE_AUDIOBUF,
    CREATE_NUANKEY,
    CREATE_COMPANY,
];
/// License-key query and expected descramble (in-unit literals).
pub const CREATE_TXMKEY: &[u8] = b"txm ypy hwb &53(1@+- _={[]?/>.<,";
pub const CREATE_TXMWANT: &[u8] = b"4:\x014CS5]@AJGW^CL\x1e\"\x168&9\x17\x0b\x18\x11\x0e\x1a\x04\x184\0";
/// Escape-sequence template head (in-unit literal).
pub const CREATE_ESCHEAD: &[u8] = b"[header]\nlanguage = *\n[data]\n/";
/// Escape-sequence template tail (ROM bytes: the `src/`
/// literal lacks the backslash, see the round-28 C defect).
pub const CREATE_ESCTAIL: &[u8] = b"/ --> \"\\x1b\" \n";
/// Voice-install MIME and key argument (in-unit literals).
pub const CREATE_VOICEMIME: &[u8] = b"application/x-vocalizer-rettt+text";
/// `D_8d7968` compare/copy/release (US ROM values; the row
/// never observes them).
pub const CREATE_CMP: u32 = 0x4;
pub const CREATE_CPY: u32 = 0x151774;
pub const CREATE_REL: u32 = 0x1517cc;
/// `&D_8d79d8` / `&D_8d7970` (US ROM addresses, stored into the
/// 17b0f8 buffers; the row skips those words).
pub const CREATE_D_79D8: u32 = 0x8d79d8;
pub const CREATE_D_7970: u32 = 0x8d7970;

/// Byte length before the NUL (`fn_195d90` on a slice).
fn cstrlen(s: &[u8]) -> u32 {
    s.iter().position(|&c| c == 0).unwrap() as u32
}

/// Case-insensitive 8-bit compare through the proven
/// `fn_17c8c0` fold (`fn_1769d0` on slices).
fn cmp_fold(a: &[u8], b: &[u8]) -> i32 {
    let mut i = 0;
    loop {
        let x = crate::fe_text::fold_17c8c0(a[i] as u32);
        let y = crate::fe_text::fold_17c8c0(b[i] as u32);
        if x != y || a[i] == 0 {
            return x.wrapping_sub(y) as i32;
        }
        i += 1;
    }
}

/// Bounded case-insensitive compare (`fn_176a48` on slices).
fn cmp_fold_n(a: &[u8], b: &[u8], n: u32) -> i32 {
    let mut i = 0;
    while n > i {
        let x = crate::fe_text::fold_17c8c0(a[i as usize] as u32);
        let y = crate::fe_text::fold_17c8c0(b[i as usize] as u32);
        if x != y || a[i as usize] == 0 || b[i as usize] == 0 {
            break;
        }
        i += 1;
    }
    if i == n {
        0
    } else {
        let x = crate::fe_text::fold_17c8c0(a[i as usize] as u32);
        let y = crate::fe_text::fold_17c8c0(b[i as usize] as u32);
        x.wrapping_sub(y) as i32
    }
}

/// External calls of the `fn_157a80` unit that live outside
/// `src/fn_151914_merged.c`. Offsets address the caller's
/// `mem`; strings are passed as bytes for logging. Methods
/// named after the caller's use; the doc names the callee.
pub trait CreateHooks {
    /// `fn_176f94`: validate the open pair.
    fn check_open(&mut self, obj: u32, b: u32) -> u32;
    /// `fn_19611c`: resolve `(a, b)` to `(status, context)`.
    fn resolve_ctx(&mut self, a: u32, b: u32) -> (u32, u32);
    /// `fn_1963b8`: allocate the object (0 is failure).
    fn alloc_object(&mut self, n: u32, size: u32) -> u32;
    /// `fn_176e40`: make the object mutex.
    fn make_mutex(&mut self, pad8: u32, heap: u32) -> (u32, u32);
    /// `fn_196a64`: make the parameter callback (0 fails).
    fn make_param_cb(&mut self, heap: u32, cmp: u32, cpy: u32, rel: u32) -> (u32, u32);
    /// `fn_17b058`: query the component registry.
    fn query_comp(&mut self, comp: u32, name: &[u8]) -> (u32, u32);
    /// `fn_17f278`: register a missing component.
    fn register_comp(&mut self, c: u32, d: u32, name: &[u8], lname: &[u8], param_cb: u32) -> u32;
    /// `fn_17b1c4`: release a registry handle.
    fn release_comp(&mut self, comp: u32, name: &[u8]);
    /// `fn_17b0f8`: insert a registry entry.
    fn insert_comp(&mut self, comp: u32, name: &[u8], buf: &[u8; 20]) -> u32;
    /// `fn_173fd8`: log verbosity.
    fn log_level(&mut self, log: u32) -> u32;
    /// `fn_174e74`: register a parameter callback.
    fn reg_param_cb(&mut self, params: u32, key: Option<&[u8]>, this: u32) -> u32;
    /// `fn_175a1c`: query a parameter string.
    fn query_param(&mut self, params: u32, key: &[u8]) -> (u32, u32);
    /// `fn_1757a8`: keep a present parameter.
    fn touch_param(&mut self, params: u32, value: &[u8]);
    /// `fn_1758ac`: set a numeric default.
    fn set_default_num(&mut self, params: u32, key: &[u8], value: u32);
    /// `fn_1759a0`: set a string default.
    fn set_default_str(&mut self, params: u32, key: &[u8], value: &[u8]);
    /// `fn_175944`: query a numeric parameter.
    fn query_num(&mut self, params: u32, key: &[u8]) -> (u32, u32);
    /// `fn_17eb90`: license token.
    fn lic_token(&mut self, c: u32, d: u32) -> (u32, u32);
    /// `fn_17f614`: trace the pipeline.
    fn trace_pipeline(&mut self, ha: u32, hb: u32, mime: &[u8], param_cb: u32) -> (u32, u32, u8);
    /// `fn_1529b4`: start the object.
    fn start_object(&mut self, ctx: u32, obj: u32) -> u32;
    /// `fn_151914`: install the escape mapping.
    #[allow(clippy::too_many_arguments)]
    fn install_voice(
        &mut self,
        h: (u32, u32),
        mime: &[u8],
        key: &[u8],
        text: &[u8],
        len: u32,
        blob: (u32, u32),
    ) -> (u32, u32, u32);
    /// `fn_176fd4`: the null pair.
    fn null_pair(&mut self) -> (u32, u32);
    /// `fn_196394`: allocate a temp string (never fails here).
    fn alloc_temp(&mut self, heap: u32, size: u32) -> u32;
    /// `fn_196428`: free a temp string.
    fn free_temp(&mut self, heap: u32, ptr: u32);
    /// `fn_1523cc`: tear down a failed create.
    fn teardown(&mut self, ctx: u32);
    /// `fn_151bf8`: release a replaced voice.
    fn release_voice(&mut self, h: (u32, u32), ptr: u32, n: u32);
}

/// fn_15ae20 / EU fn_15aee4 / KR fn_15ae04.
///
/// Validate the open pair and resolve its context. Returns the
/// check status (mapping the `0x1fff == 8` failure to
/// `0x81202008`); on success stores the object into `out` and,
/// when `slot` is present, resolves `(a, b)` into it.
pub fn open_object<H: CreateHooks>(
    hooks: &mut H,
    mem: &[u8],
    obj: u32,
    b: u32,
    slot: Option<&mut u32>,
    out: &mut u32,
) -> u32 {
    *out = 0;
    let rc = hooks.check_open(obj, b);
    if rc >= 0x80000000 {
        if rc & 0x1fff == 8 {
            return 0x81202008;
        }
        return rc;
    }
    *out = obj;
    if let Some(slot) = slot {
        let (rc, ctx) = hooks.resolve_ctx(mem_word(mem, obj), mem_word(mem, obj.wrapping_add(4)));
        *slot = ctx;
        rc
    } else {
        rc
    }
}

/// fn_15accc / EU fn_15ad90 / KR fn_15acb0.
///
/// Descramble the `txm` license key from `params` with the
/// 3-byte rotating key and report whether it matches. The
/// caller guarantees the queried string is shorter than the
/// 100-byte scratch, matching the original's stack buffer.
pub fn check_key<H: CreateHooks>(hooks: &mut H, mem: &[u8], params: u32) -> u32 {
    let (rc, src) = hooks.query_param(params, CREATE_TXMKEY);
    if rc >= 0x80000000 || src == 0 {
        return 0;
    }
    let src = terminated(mem, src);
    let mut buf = [0u8; 100];
    let key = b"key";
    let k = key.len() as u32;
    let mut n = cstrlen(src);
    let (mut i, mut j) = (0usize, 0u32);
    if n != 0 {
        if n & 1 != 0 {
            j = u32::from(k > 1);
            buf[0] = key[0] ^ src[0];
            i = 1;
        }
        n >>= 1;
        while n != 0 {
            buf[i] = key[j as usize] ^ src[i];
            j = if j < k - 1 { j + 1 } else { 0 };
            i += 1;
            buf[i] = key[j as usize] ^ src[i];
            j = if j < k - 1 { j + 1 } else { 0 };
            i += 1;
            n -= 1;
        }
    }
    u32::from(cmp_fold(&buf, CREATE_TXMWANT) == 0)
}

/// fn_15b128 / EU fn_15b1ec / KR fn_15b10c.
///
/// Classify the license state: 2 for a keyed license, 1 for a
/// CPR-voiced trace that survives the entry walk, 0 otherwise.
pub fn license_kind<H: CreateHooks>(hooks: &mut H, mem: &[u8], ctx: u32, obj: u32, trace: u32) -> u32 {
    let lic = mem_word(mem, obj.wrapping_add(0x10));
    let (rc, inst) = hooks.resolve_ctx(mem_word(mem, lic), mem_word(mem, lic.wrapping_add(4)));
    if rc < 0x80000000 && check_key(hooks, mem, mem_word(mem, inst.wrapping_add(0x14))) != 0 {
        return 2;
    }
    if trace == 0 {
        return 0;
    }
    if mem_word(mem, lic.wrapping_add(0x4c)) == 0 && mem_word(mem, lic.wrapping_add(0x58)) == 0 {
        return 0;
    }
    let (rc, voice) = hooks.query_param(mem_word(mem, ctx.wrapping_add(0x14)), CREATE_VOICEMODEL);
    if rc >= 0x80000000 || voice == 0 {
        return 0;
    }
    if cmp_fold(terminated(mem, voice), CREATE_CPR_Z) != 0 {
        return 0;
    }
    let count = mem_half(mem, trace.wrapping_add(0xc));
    if count == 0 {
        return 1;
    }
    let mut i = 0u32;
    loop {
        let entry = mem_word(mem, mem_word(mem, trace.wrapping_add(8)).wrapping_add(i.wrapping_mul(4)));
        let e = terminated(mem, mem_word(mem, entry.wrapping_add(4)));
        if cmp_fold_n(e, CREATE_USELECT_Z, 8) == 0 {
            return 0;
        }
        if cmp_fold_n(e, CREATE_SELECT_Z, 7) == 0 && cmp_fold(e, CREATE_SELCPR_Z) != 0 {
            return 0;
        }
        i = (i.wrapping_add(1)) & 0xffff;
        if i >= u32::from(count) {
            break;
        }
    }
    1
}

/// fn_15af9c / EU fn_15b060 / KR fn_15af80.
///
/// Install the alternate escape-sequence mapping for `value`
/// (0 skips the string build). Returns the install status, or
/// the replaced pair head, or the value's first byte.
pub fn install_esc_seq<H: CreateHooks>(
    hooks: &mut H,
    mem: &mut [u8],
    ctx: u32,
    obj: u32,
    value: u32,
) -> u32 {
    let heap = mem_word(mem, ctx.wrapping_add(4));
    let pair = hooks.null_pair();
    let mut rc = 0;
    if mem_word(mem, obj.wrapping_add(0x144)) != 0 {
        let h = (mem_word(mem, obj.wrapping_add(8)), mem_word(mem, obj.wrapping_add(12)));
        hooks.release_voice(
            h,
            mem_word(mem, obj.wrapping_add(0x144)),
            mem_word(mem, obj.wrapping_add(0x148)),
        );
        let pair = hooks.null_pair();
        mem_store(mem, obj.wrapping_add(0x144), pair.0);
        mem_store(mem, obj.wrapping_add(0x148), pair.1);
        rc = pair.0;
    }
    if value != 0 {
        rc = u32::from(mem[value as usize]);
    }
    if value != 0 && rc != 0 {
        let n1 = CREATE_ESCHEAD.len() as u32;
        let n2 = cstrlen(terminated(mem, value));
        let n3 = CREATE_ESCTAIL.len() as u32;
        let size = n3.wrapping_add(n1).wrapping_add(n2).wrapping_add(1);
        let q = hooks.alloc_temp(heap, size);
        let qz = q as usize;
        mem[qz..qz + n1 as usize].copy_from_slice(CREATE_ESCHEAD);
        mem[qz + n1 as usize] = 0;
        let at = qz + n1 as usize;
        mem.copy_within(value as usize..value as usize + n2 as usize, at);
        mem[at + n2 as usize] = 0;
        let at = at + n2 as usize;
        mem[at..at + n3 as usize].copy_from_slice(CREATE_ESCTAIL);
        mem[at + n3 as usize] = 0;
        let len = cstrlen(terminated(mem, q));
        let h = (mem_word(mem, obj.wrapping_add(8)), mem_word(mem, obj.wrapping_add(12)));
        let text = terminated(mem, q);
        let (status, p0, p1) = hooks.install_voice(
            h,
            CREATE_VOICEMIME,
            CREATE_NUANKEY,
            &text[..len as usize],
            len,
            pair,
        );
        mem_store(mem, obj.wrapping_add(0x144), p0);
        mem_store(mem, obj.wrapping_add(0x148), p1);
        hooks.free_temp(heap, q);
        rc = status;
    }
    rc
}

/// Shared takedown for the `fn_157a80` fail paths: release the
/// half-built object and zero the out pair.
fn fail_create<H: CreateHooks>(hooks: &mut H, mem: &mut [u8], ctx: u32, out: u32, rc: u32) -> u32 {
    hooks.teardown(ctx);
    mem_store(mem, out, 0);
    mem_store(mem, out.wrapping_add(4), 0);
    rc
}

/// fn_157a80 / EU fn_157b44 / KR fn_157a64.
///
/// Create a synthesis object for the `(obj, b)` open pair:
/// resolve the context, allocate and wire the object, bind the
/// stream/lingua/component/inet providers, install parameter
/// defaults and callbacks, license and start the pipeline, and
/// report the pair through `out`. The `D_7974` callback list,
/// the `D_8d7968` triple, and the `&D_` buffer words are the US
/// ROM values (the row leaves the addresses unobserved). Log
/// calls are elided (void, unobserved), as are the dead `inet`
/// buffer stores. `mime`/`out` are 0 for null; `opts` points
/// at 5 words.
#[allow(clippy::too_many_arguments)]
pub fn create_object<H: CreateHooks>(
    hooks: &mut H,
    mem: &mut [u8],
    obj: u32,
    b: u32,
    c: u32,
    d: u32,
    opts: u32,
    sink: u32,
    mime: u32,
    out: u32,
) -> u32 {
    let mut lic = 0;
    let mut ctx = 0;
    let rc = open_object(hooks, mem, obj, b, Some(&mut ctx), &mut lic);
    if rc >= 0x80000000 {
        return 0x81202008;
    }
    if out == 0 {
        return 0x81202006;
    }
    let (rc, fresh) = hooks.resolve_ctx(c, d);
    ctx = fresh;
    if rc >= 0x80000000 {
        return rc;
    }
    let heap = mem_word(mem, ctx.wrapping_add(4));
    let params = mem_word(mem, ctx.wrapping_add(20));
    if params == 0 {
        return 0x81202007;
    }
    let o = hooks.alloc_object(1, 0x14c);
    if o == 0 {
        return 0x8120200a;
    }
    mem_store(mem, out, o);
    mem_store(mem, out.wrapping_add(4), 0xcdac);
    for i in 0..5 {
        mem_store(
            mem,
            o.wrapping_add(0x14).wrapping_add(i * 4),
            mem_word(mem, opts.wrapping_add(i * 4)),
        );
    }
    mem_store(mem, o.wrapping_add(0x28), sink);
    mem[o as usize + 0x30] = 1;
    mem[o as usize + 0x69] = 3;
    mem_store(mem, o.wrapping_add(0x140), 0);
    mem_store(mem, o, c);
    mem_store(mem, o.wrapping_add(4), d);
    mem_store(mem, o.wrapping_add(8), o);
    mem_store(mem, o.wrapping_add(12), 0xcdac);
    mem_store(mem, o.wrapping_add(0x10), lic);
    let (rc, mutex) = hooks.make_mutex(mem_word(mem, ctx.wrapping_add(8)), heap);
    if rc >= 0x80000000 {
        return fail_create(hooks, mem, ctx, out, rc);
    }
    mem_store(mem, o.wrapping_add(0x2c), mutex);
    let (rc, param_cb) = hooks.make_param_cb(heap, CREATE_CMP, CREATE_CPY, CREATE_REL);
    if rc == 0 {
        return fail_create(hooks, mem, ctx, out, rc);
    }
    mem_store(mem, o.wrapping_add(0x3c), param_cb);
    let comp = mem_word(mem, ctx.wrapping_add(24));
    let (mut rc, mut q) = hooks.query_comp(comp, CREATE_SYNTHSTREAM);
    if rc >= 0x80000000 {
        rc = hooks.register_comp(c, d, CREATE_SYNTHSTREAM, CREATE_SYNTHSTREAM_LO, param_cb);
        if rc >= 0x80000000 {
            return fail_create(hooks, mem, ctx, out, rc);
        }
        (rc, q) = hooks.query_comp(comp, CREATE_SYNTHSTREAM);
        if rc >= 0x80000000 {
            return fail_create(hooks, mem, ctx, out, rc);
        }
    }
    mem_store(mem, o.wrapping_add(0x40), mem_word(mem, q.wrapping_add(4)));
    let (rc, q) = hooks.query_comp(comp, CREATE_LINGDB);
    let (mut rc, mut q) = (rc, q);
    if rc >= 0x80000000 {
        rc = hooks.register_comp(c, d, CREATE_LINGDB, CREATE_LINGDB_LO, param_cb);
        if rc >= 0x80000000 {
            return fail_create(hooks, mem, ctx, out, rc);
        }
        (rc, q) = hooks.query_comp(comp, CREATE_LINGDB);
        if rc >= 0x80000000 {
            return fail_create(hooks, mem, ctx, out, rc);
        }
    }
    mem_store(mem, o.wrapping_add(0x54), mem_word(mem, q.wrapping_add(4)));
    if mem_word(mem, lic.wrapping_add(16)) != 0 {
        let (rc, _) = hooks.query_comp(comp, CREATE_COMPQUERY);
        if rc >= 0x80000000 {
            let mut buf = [0u8; 20];
            buf[0..4].copy_from_slice(&1u32.to_le_bytes());
            buf[4..8].copy_from_slice(&mem_word(mem, lic.wrapping_add(16)).to_le_bytes());
            buf[8..12].copy_from_slice(&mem_word(mem, lic.wrapping_add(20)).to_le_bytes());
            buf[12..16].copy_from_slice(&mem_word(mem, lic.wrapping_add(24)).to_le_bytes());
            let rc = hooks.insert_comp(comp, CREATE_COMPQUERY, &buf);
            if rc >= 0x80000000 {
                return fail_create(hooks, mem, ctx, out, rc);
            }
            mem[o as usize + 0x58] = 1;
        } else {
            hooks.release_comp(comp, CREATE_COMPQUERY);
        }
    }
    let (rc, _) = hooks.query_comp(comp, CREATE_TTSEG);
    if rc >= 0x80000000 {
        let mut buf = [0u8; 20];
        buf[0..4].copy_from_slice(&3u32.to_le_bytes());
        buf[4..8].copy_from_slice(&CREATE_D_79D8.to_le_bytes());
        buf[8..12].copy_from_slice(&mem_word(mem, out).to_le_bytes());
        buf[12..16].copy_from_slice(&mem_word(mem, out.wrapping_add(4)).to_le_bytes());
        let rc = hooks.insert_comp(comp, CREATE_TTSEG, &buf);
        if rc >= 0x80000000 {
            return fail_create(hooks, mem, ctx, out, rc);
        }
        mem[o as usize + 0x59] = 1;
    } else {
        hooks.release_comp(comp, CREATE_TTSEG);
    }
    let (rc, q) = hooks.query_comp(comp, CREATE_INET);
    if rc < 0x80000000 {
        mem_store(mem, o.wrapping_add(0x5c), mem_word(mem, q.wrapping_add(4)));
        mem_store(mem, o.wrapping_add(0x60), mem_word(mem, q.wrapping_add(8)));
        mem_store(mem, o.wrapping_add(0x64), mem_word(mem, q.wrapping_add(12)));
    }
    let mut param_reg = mem[o as usize + 0x68];
    if hooks.log_level(mem_word(mem, ctx.wrapping_add(16))) > 3 {
        if hooks.reg_param_cb(params, None, o) < 0x80000000 {
            param_reg = 1;
            mem[o as usize + 0x68] = 1;
        }
    }
    let defaults: [(&[u8], u32, u32); 13] = [
        (CREATE_PITCH, 0, 0x32),
        (CREATE_RATE, 0, 100),
        (CREATE_VOLUME, 0, 0x50),
        (CREATE_WAITFACTOR, 0, 2),
        (CREATE_READMODE, 0, 1),
        (CREATE_MARKERMODE, 0, 0),
        (CREATE_LOOPLIMIT, 0, 0x2710),
        (CREATE_INPUTMIME, 1, 0),
        (CREATE_AUDIOBUF, 0, 0x1000),
        (CREATE_LICMODE, 2, 0),
        (CREATE_NUANKEY, 1, 0),
        (CREATE_COMPANY, 1, 0),
        (CREATE_APPNAME, 1, 0),
    ];
    for (key, kind, num) in defaults {
        let (rc, pv) = hooks.query_param(params, key);
        if rc < 0x80000000 && pv != 0 {
            hooks.touch_param(params, terminated(mem, pv));
        } else if kind == 0 {
            hooks.set_default_num(params, key, num);
        } else if kind == 2 {
            hooks.set_default_str(params, key, CREATE_DEFAULT);
        } else {
            hooks.set_default_str(params, key, CREATE_EMPTY);
        }
    }
    let (rc, limit) = hooks.query_num(params, CREATE_LOOPLIMIT);
    if rc >= 0x80000000 {
        return fail_create(hooks, mem, ctx, out, rc);
    }
    mem_store(mem, o.wrapping_add(0x38), limit);
    let (rc, mode) = hooks.query_num(params, CREATE_MARKERMODE);
    if rc >= 0x80000000 {
        return fail_create(hooks, mem, ctx, out, rc);
    }
    mem_store(mem, o.wrapping_add(0x34), mode);
    if param_reg == 0 {
        for key in CREATE_CBKEYS {
            let rc = hooks.reg_param_cb(params, Some(key), o);
            if rc < 0x80000000 {
                param_reg = 1;
                mem[o as usize + 0x68] = 1;
            } else if rc & 0x1fff != 0x14 {
                return fail_create(hooks, mem, ctx, out, rc);
            }
        }
    }
    let (rc, token) = hooks.lic_token(c, d);
    if rc >= 0x80000000 {
        return fail_create(hooks, mem, ctx, out, rc);
    }
    mem_store(mem, o.wrapping_add(0x74), token);
    mem[o as usize + 0x70] = mem[lic as usize + 148];
    let mut buf = [0u8; 20];
    buf[0..4].copy_from_slice(&1u32.to_le_bytes());
    buf[4..8].copy_from_slice(&CREATE_D_7970.to_le_bytes());
    buf[8..12].copy_from_slice(&mem_word(mem, out).to_le_bytes());
    buf[12..16].copy_from_slice(&mem_word(mem, out.wrapping_add(4)).to_le_bytes());
    let rc = hooks.insert_comp(comp, CREATE_SINK, &buf);
    if rc >= 0x80000000 {
        return fail_create(hooks, mem, ctx, out, rc);
    }
    mem_store(mem, o.wrapping_add(0x78), 1);
    let mut rc = rc;
    if mime != 0 && mem[mime as usize] != 0 {
        let mt = terminated(mem, mime);
        let (rc2, trace, _) =
            hooks.trace_pipeline(mem_word(mem, o), mem_word(mem, o.wrapping_add(4)), mt, param_cb);
        if rc2 >= 0x80000000 {
            return fail_create(hooks, mem, ctx, out, rc2);
        }
        mem_store(mem, o.wrapping_add(0x7c), trace);
        mem[o as usize + 0x69] = license_kind(hooks, mem, ctx, o, trace) as u8;
        rc = hooks.start_object(ctx, o);
        if rc >= 0x80000000 {
            return fail_create(hooks, mem, ctx, out, rc);
        }
    }
    let (rc2, tmp) = hooks.query_param(params, CREATE_NUANKEY);
    if rc2 < 0x80000000 {
        let status = install_esc_seq(hooks, mem, ctx, o, tmp);
        if status >= 0x80000000 {
            return fail_create(hooks, mem, ctx, out, status);
        }
        rc = status;
    }
    rc
}
/// License-feature map behind `fn_158c4c`'s kind guard
/// (`D_8d7969` bytes 0,3,0,6, identical on US/EU/KR).
pub const PIPE_KIND_MAP: [u8; 4] = [0, 3, 0, 6];
/// Parameter key for the pipeline-set analysis toggle.
pub const PIPE_ANALYSIS_KEY: &[u8] = b"textanalysison";
/// Toggle values (in-unit literals).
pub const PIPE_YES: &[u8] = b"yes";
pub const PIPE_NO: &[u8] = b"no";
/// Guard name for the pipeline enter/leave pair (in-unit literal).
pub const PIPE_GUARD_NAME: &[u8] = b"ttseg_PipelineSet";

/// Cross-TU calls made by `fn_158c4c` (void log calls elided).
pub trait PipeHooks {
    /// `fn_176f94`: open the `(obj, b)` handle (tag/size fixed by the caller).
    fn check_open(&mut self, obj: u32, b: u32) -> u32;
    /// `fn_19611c`: resolve `(a, b)` to the context.
    fn resolve_ctx(&mut self, a: u32, b: u32) -> (u32, u32);
    /// `fn_1759a0`: set the analysis toggle.
    fn set_textanalysis(&mut self, params: u32, on: bool);
    /// `fn_176f2c`: lock a mutex.
    fn lock(&mut self, mutex: u32) -> u32;
    /// `fn_176f5c`: unlock a mutex.
    fn unlock(&mut self, mutex: u32) -> u32;
    /// `fn_175c40`: open a pending parameter edit.
    fn query_pending(&mut self, params: u32) -> (u32, u32);
    /// `fn_17f614`: trace the pipeline for `name`; returns
    /// (status, pipeline, reactivate flag).
    fn trace_pipe(
        &mut self,
        ha: u32,
        hb: u32,
        name: &[u8],
        cs: u32,
        stream: u32,
    ) -> (u32, u32, u8);
    /// `fn_175a1c`: query a parameter string.
    fn query_param(&mut self, params: u32, key: &[u8]) -> (u32, u32);
    /// `fn_1529b4`: start the object.
    fn start_object(&mut self, ctx: u32, obj: u32) -> u32;
    /// `fn_17eb74`: licensed count for a feature.
    fn lic_count(&mut self, feat: u32) -> u32;
    /// `fn_17eb3c`: licensed feature probe.
    fn lic_feat(&mut self, feat: u32) -> u32;
    /// `fn_17eb58`: release the license token.
    fn release_token(&mut self, token: u32);
    /// `fn_151f80`: license-state notify.
    fn notify_lic(&mut self, ctx: u32, channel: u32, kind: u32, mode: u32);
    /// `fn_17ec38`: release a pipeline.
    fn release_pipe(&mut self, pipe: u32) -> u32;
    /// `fn_175ce0`: roll a pending edit back.
    fn rollback_pending(&mut self, params: u32, pending: u32) -> u32;
    /// `fn_175ef0`: release a pending edit.
    fn release_pending(&mut self, params: u32, pending: u32) -> u32;
    /// `fn_1805f0`: reactivate the current stream.
    fn reactivate(&mut self, cs: u32);
}

/// fn_1517e8 / EU fn_1518ac / KR fn_1517cc.
///
/// Open the `(a, b)` pipeline handle and resolve its context;
/// reports the object through `obj` (the handle itself).
pub fn open_pipeline_ctx<H: PipeHooks>(
    hooks: &mut H,
    mem: &[u8],
    a: u32,
    b: u32,
    ctx: &mut u32,
    obj: &mut u32,
) -> u32 {
    *obj = 0;
    let rc = hooks.check_open(a, b);
    if rc >= 0x80000000 {
        if rc & 0x1fff == 8 {
            return 0x81202008;
        }
        return rc;
    }
    let (rc, fresh) = hooks.resolve_ctx(mem_word(mem, a), mem_word(mem, a.wrapping_add(4)));
    *ctx = fresh;
    if rc < 0x80000000 {
        *obj = a;
    }
    rc
}

/// `StateLock` over a `PipeHooks` mutex cell.
struct PipeLock<'h, H: PipeHooks> {
    hooks: &'h mut H,
    mutex: u32,
}

impl<H: PipeHooks> StateLock for PipeLock<'_, H> {
    fn acquire(&mut self) -> u32 {
        self.hooks.lock(self.mutex)
    }
    fn release(&mut self) -> u32 {
        self.hooks.unlock(self.mutex)
    }
}

/// Sign-normalized byte compare (`fn_195d58`).
pub(crate) fn pipe_cmp(a: &[u8], b: &[u8]) -> i32 {
    let mut i = 0;
    loop {
        let d = i32::from(a[i]) - i32::from(b[i]);
        if d != 0 || b[i] == 0 {
            if d < 0 {
                return -1;
            }
            if d > 0 {
                return 1;
            }
            return 0;
        }
        i += 1;
    }
}

/// fn_15accc mirror for the pipeline path (same key/want pair).
fn pipe_check_key<H: PipeHooks>(hooks: &mut H, mem: &[u8], params: u32) -> u32 {
    let (rc, src) = hooks.query_param(params, CREATE_TXMKEY);
    if rc >= 0x80000000 || src == 0 {
        return 0;
    }
    let src = terminated(mem, src);
    let mut buf = [0u8; 100];
    let key = b"key";
    let k = key.len() as u32;
    let mut n = cstrlen(src);
    let (mut i, mut j) = (0usize, 0u32);
    if n != 0 {
        if n & 1 != 0 {
            j = u32::from(k > 1);
            buf[0] = key[0] ^ src[0];
            i = 1;
        }
        n >>= 1;
        while n != 0 {
            buf[i] = key[j as usize] ^ src[i];
            j = if j < k - 1 { j + 1 } else { 0 };
            i += 1;
            buf[i] = key[j as usize] ^ src[i];
            j = if j < k - 1 { j + 1 } else { 0 };
            i += 1;
            n -= 1;
        }
    }
    u32::from(cmp_fold(&buf, CREATE_TXMWANT) == 0)
}

/// fn_15b128 mirror for the pipeline path.
pub(crate) fn pipe_license_kind<H: PipeHooks>(
    hooks: &mut H,
    mem: &[u8],
    ctx: u32,
    obj: u32,
    trace: u32,
) -> u32 {
    let lic = mem_word(mem, obj.wrapping_add(0x10));
    let (rc, inst) = hooks.resolve_ctx(mem_word(mem, lic), mem_word(mem, lic.wrapping_add(4)));
    if rc < 0x80000000 && pipe_check_key(hooks, mem, mem_word(mem, inst.wrapping_add(0x14))) != 0
    {
        return 2;
    }
    if trace == 0 {
        return 0;
    }
    if mem_word(mem, lic.wrapping_add(0x4c)) == 0 && mem_word(mem, lic.wrapping_add(0x58)) == 0 {
        return 0;
    }
    let (rc, voice) = hooks.query_param(mem_word(mem, ctx.wrapping_add(0x14)), CREATE_VOICEMODEL);
    if rc >= 0x80000000 || voice == 0 {
        return 0;
    }
    if cmp_fold(terminated(mem, voice), CREATE_CPR_Z) != 0 {
        return 0;
    }
    let count = mem_half(mem, trace.wrapping_add(0xc));
    if count == 0 {
        return 1;
    }
    let mut i = 0u32;
    loop {
        let entry = mem_word(mem, mem_word(mem, trace.wrapping_add(8)).wrapping_add(i.wrapping_mul(4)));
        let e = terminated(mem, mem_word(mem, entry.wrapping_add(4)));
        if cmp_fold_n(e, CREATE_USELECT_Z, 8) == 0 {
            return 0;
        }
        if cmp_fold_n(e, CREATE_SELECT_Z, 7) == 0 && cmp_fold(e, CREATE_SELCPR_Z) != 0 {
            return 0;
        }
        i = (i.wrapping_add(1)) & 0xffff;
        if i >= u32::from(count) {
            break;
        }
    }
    1
}

/// Hooks for the TTSEG license allocate/free twins.
pub trait LicenseOpHooks {
    /// `fn_1517e8`: open the `(a, b)` handle; reports ctx and lic.
    fn open_handle(&mut self, a: u32, b: u32, ctx: &mut u32, lic: &mut u32) -> u32;
}

/// fn_158a50 / EU fn_158b14 / KR fn_158a34, twin fn_158b58 / EU fn_158c1c /
/// KR fn_158b3c.
///
/// TTSEG license allocate/free: both resolve the `(a, b)` handle and
/// report the fixed unlicensed status. Log calls are elided (void,
/// unobserved); `c` only feeds the null check (the mode string goes
/// to the log, which never reads it).
pub fn license_op<H: LicenseOpHooks>(hooks: &mut H, a: u32, b: u32, c: u32) -> u32 {
    if c == 0 {
        return 0x81202007;
    }
    let mut ctx = 0;
    let mut lic = 0;
    let rc = hooks.open_handle(a, b, &mut ctx, &mut lic);
    if rc >= 0x80000000 {
        return rc;
    }
    0x81203d09
}

/// fn_158c4c / EU fn_158d10 / KR fn_158c30.
///
/// Set the TTSEG pipeline: open the handle, store the status,
/// flip the analysis toggle, enter the guard, and either
/// install the traced pipeline (switching the license state
/// when the kind moves) or roll the pending edit back.
/// `name` is 0 for null; `d` feeds the status byte and the
/// toggle. Log calls are elided (void, unobserved).
pub fn set_pipeline<H: PipeHooks>(
    hooks: &mut H,
    mem: &mut [u8],
    a: u32,
    b: u32,
    name: u32,
    d: u32,
) -> u32 {
    let mut ctx = 0;
    let mut object = 0;
    let rc = open_pipeline_ctx(hooks, mem, a, b, &mut ctx, &mut object);
    if rc >= 0x80000000 {
        return rc;
    }
    let o = object;
    mem[o as usize + 0x80] = d as u8;
    let params = mem_word(mem, ctx.wrapping_add(20));
    hooks.set_textanalysis(params, d == 2);
    let mut state = 0;
    let omutex = mem_word(mem, o.wrapping_add(0x2c));
    let rc = enter_pipeline(
        &mut mem[o as usize + 0x30],
        2,
        &mut state,
        &mut PipeLock {
            hooks: &mut *hooks,
            mutex: omutex,
        },
    );
    if rc >= 0x80000000 {
        return rc;
    }
    let mut pipeline = 0;
    let mut reactivate = 0;
    let mut rc = rc;
    if name != 0 && mem[name as usize] != 0 {
        let cs = mem_word(mem, o.wrapping_add(0x7c));
        if cs != 0
            && pipe_cmp(
                terminated(mem, name),
                terminated(mem, mem_word(mem, cs.wrapping_add(4))),
            ) == 0
        {
            restore_pipeline(
                &mut mem[o as usize + 0x30],
                u32::from(state),
                &mut PipeLock {
                    hooks: &mut *hooks,
                    mutex: omutex,
                },
            );
            return rc;
        }
        let (rc2, pending) = hooks.query_pending(params);
        rc = rc2;
        if rc >= 0x80000000 {
            restore_pipeline(
                &mut mem[o as usize + 0x30],
                u32::from(state),
                &mut PipeLock {
                    hooks: &mut *hooks,
                    mutex: omutex,
                },
            );
            return rc;
        }
        let (rc2, fresh, flag) = hooks.trace_pipe(
            mem_word(mem, o),
            mem_word(mem, o.wrapping_add(4)),
            terminated(mem, name),
            cs,
            mem_word(mem, o.wrapping_add(0x3c)),
        );
        pipeline = fresh;
        reactivate = flag;
        rc = rc2;
        if rc < 0x80000000 {
            let kind = pipe_license_kind(hooks, mem, ctx, o, pipeline);
            if mem[o as usize + 0x6a] == 0 {
                mem[o as usize + 0x69] = kind as u8;
                rc = hooks.start_object(ctx, o);
            } else {
                let old = u32::from(mem[o as usize + 0x69]);
                if old != kind {
                    let channel = mem_word(mem, o.wrapping_add(0x10));
                    rc = hooks.lock(mem_word(mem, channel.wrapping_add(0x90)));
                    if rc < 0x80000000 {
                        let usage =
                            channel.wrapping_add(0x30).wrapping_add(kind.wrapping_mul(24));
                        let feat = if kind < 4 {
                            u32::from(PIPE_KIND_MAP[kind as usize])
                        } else {
                            6
                        };
                        if mem_word(mem, usage.wrapping_add(12))
                            < mem_word(mem, usage.wrapping_add(4))
                        {
                            mem_store(
                                mem,
                                usage.wrapping_add(12),
                                mem_word(mem, usage.wrapping_add(12)).wrapping_add(1),
                            );
                            pipe_switch(
                                hooks, mem, ctx, channel, o, old, kind, 1, usage, feat,
                            );
                        } else if hooks.lic_count(feat)
                            < mem_word(mem, usage.wrapping_add(16))
                            && hooks.lic_feat(feat) == 0
                        {
                            pipe_switch(
                                hooks, mem, ctx, channel, o, old, kind, 2, usage, feat,
                            );
                        } else {
                            rc = 0x81203d06;
                            hooks.notify_lic(ctx, channel, kind, 7);
                            if old == 0 {
                                rc = 0;
                            }
                            hooks.unlock(mem_word(mem, channel.wrapping_add(0x90)));
                        }
                    }
                }
            }
            if rc >= 0x80000000 {
                hooks.release_pipe(pipeline);
                hooks.rollback_pending(params, pending);
            }
        } else {
            hooks.rollback_pending(params, pending);
        }
        let status = hooks.release_pending(params, pending);
        if status >= 0x80000000 && rc < 0x80000000 {
            rc = status;
        }
        if rc >= 0x80000000 {
            if reactivate != 0 && mem_word(mem, o.wrapping_add(0x7c)) != 0 {
                hooks.reactivate(mem_word(mem, o.wrapping_add(0x7c)));
            }
            restore_pipeline(
                &mut mem[o as usize + 0x30],
                u32::from(state),
                &mut PipeLock {
                    hooks: &mut *hooks,
                    mutex: omutex,
                },
            );
            return rc;
        }
    }
    if mem_word(mem, o.wrapping_add(0x7c)) != 0 {
        rc = hooks.release_pipe(mem_word(mem, o.wrapping_add(0x7c)));
    }
    mem_store(mem, o.wrapping_add(0x7c), pipeline);
    restore_pipeline(
        &mut mem[o as usize + 0x30],
        u32::from(state),
        &mut PipeLock {
            hooks: &mut *hooks,
            mutex: omutex,
        },
    );
    rc
}

/// License-state switch shared by the count and feature paths
/// (previous-count drop or token release, notifies, stores).
#[allow(clippy::too_many_arguments)]
fn pipe_switch<H: PipeHooks>(
    hooks: &mut H,
    mem: &mut [u8],
    ctx: u32,
    channel: u32,
    o: u32,
    old: u32,
    kind: u32,
    license_state: u8,
    usage: u32,
    feat: u32,
) {
    if mem[o as usize + 0x6a] == 1 {
        let previous = channel.wrapping_add(0x30).wrapping_add(old.wrapping_mul(24));
        if mem_word(mem, previous.wrapping_add(12)) != 0 {
            mem_store(
                mem,
                previous.wrapping_add(12),
                mem_word(mem, previous.wrapping_add(12)).wrapping_sub(1),
            );
        }
    } else {
        hooks.release_token(mem_word(mem, o.wrapping_add(0x6c)));
    }
    hooks.notify_lic(ctx, channel, old, 6);
    mem[o as usize + 0x6a] = license_state;
    mem[o as usize + 0x69] = kind as u8;
    if hooks.lic_count(feat).wrapping_add(mem_word(mem, usage.wrapping_add(12)))
        > mem_word(mem, usage.wrapping_add(8))
    {
        hooks.lic_count(feat);
    }
    hooks.notify_lic(ctx, channel, kind, 5);
    hooks.unlock(mem_word(mem, channel.wrapping_add(0x90)));
}

/// Input-bind buffer behind `fn_15b2a0` (1028 stack bytes).
pub const BIND_BUF_LEN: usize = 1028;

/// Cross-TU calls made by `fn_15b2a0` (void log calls elided).
pub trait BindHooks {
    /// `fn_1985fc`: open the bind stage.
    fn begin_stage(&mut self, obj: u32, arg: u32);
    /// `fn_19860c`: close the bind stage.
    fn end_stage(&mut self, obj: u32, arg: u32);
    /// `src.a` callback: fill `buf` (zeroed by the row) and
    /// report the byte count through `size`.
    fn get_input(&mut self, b: u32, size: &mut u32, buf: &mut [u8; BIND_BUF_LEN]) -> u32;
    /// `m24` of the bind object: drop the pair.
    fn drop_pair(&mut self, a: u32, b: u32);
    /// `m34` of the bind object: push the filled buffer.
    fn push_input(&mut self, a: u32, b: u32, buf: &[u8; BIND_BUF_LEN]) -> u32;
    /// `fn_176fd4`: a fresh null pair.
    fn null_pair(&mut self) -> (u32, u32);
}

/// fn_15b2a0 / EU fn_15b364 / KR fn_15b284.
///
/// Pull one input buffer through the `src` callback and push
/// it into the bind object: a nonzero low 13 bits drops the
/// pair, a clean status with bytes pushes them and advances
/// `prep`, and a clean empty read drops the pair with
/// `0x01202003`. Log calls are elided (void, unobserved).
pub fn bind_input<H: BindHooks>(
    hooks: &mut H,
    mem: &mut [u8],
    src_b: u32,
    pp: u32,
    obj: u32,
    arg: u32,
    prep: u32,
) -> u32 {
    if obj != 0 {
        hooks.begin_stage(obj, arg);
    }
    let mut size = 0x400u32;
    let mut buf = [0u8; BIND_BUF_LEN];
    let mut r = hooks.get_input(src_b, &mut size, &mut buf);
    if obj != 0 {
        hooks.end_stage(obj, arg);
    }
    if r & 0x1fff != 0 {
        hooks.drop_pair(mem_word(mem, pp), mem_word(mem, pp.wrapping_add(4)));
        let pair = hooks.null_pair();
        mem_store(mem, pp, pair.0);
        mem_store(mem, pp.wrapping_add(4), pair.1);
    } else if size != 0 {
        r = hooks.push_input(mem_word(mem, pp), mem_word(mem, pp.wrapping_add(4)), &buf);
        mem_store(mem, prep, mem_word(mem, prep).wrapping_add(size));
    } else {
        hooks.drop_pair(mem_word(mem, pp), mem_word(mem, pp.wrapping_add(4)));
        let pair = hooks.null_pair();
        mem_store(mem, pp, pair.0);
        mem_store(mem, pp.wrapping_add(4), pair.1);
        r = 0x01202003;
    }
    r
}

/// Synthesis marker record size in bytes (`Marker` in fn_151914.c).
pub const MARKER_LEN: usize = 24;

/// fn_152ed4 / EU fn_152f98 / KR fn_152eb8.
///
/// Scan markers at `[start, end)` for the first one selected by
/// `flags`. With a null count the scan stops after the first hit;
/// otherwise it counts every hit whose position does not exceed the
/// first hit's. Reports the first hit's offset, or 0 for none.
pub fn filter_markers(
    mem: &[u8],
    flags: u32,
    start: u32,
    end: u32,
    count: Option<&mut u32>,
) -> u32 {
    fn marker_type(mem: &[u8], at: u32) -> u16 {
        u16::from_le_bytes(mem[at as usize..at as usize + 2].try_into().unwrap())
    }
    fn marker_arg(mem: &[u8], at: u32) -> u32 {
        mem_word(mem, at.wrapping_add(8))
    }
    fn marker_pos(mem: &[u8], at: u32) -> u32 {
        mem_word(mem, at.wrapping_add(12))
    }
    let mut count = count;
    if let Some(count) = count.as_deref_mut() {
        *count = 0;
    }
    let sync = (flags >> 4) & 1;
    let b2 = (flags >> 2) & 1;
    let b10 = (flags >> 10) & 1;
    let b6 = (flags >> 6) & 1;
    let mut first: Option<u32> = None;
    let mut p = start;
    loop {
        if p >= end {
            break;
        }
        if let Some(f) = first {
            if count.is_none() {
                break;
            }
            if marker_pos(mem, p) > marker_pos(mem, f) {
                break;
            }
        }
        let matched = match marker_type(mem, p) {
            33 | 38 => b2,
            1 => (flags >> 1) & 1,
            4 => (flags >> 9) & 1,
            8 => (flags >> 5) & 1,
            16 | 18 | 19 | 24 => sync,
            20 => (flags >> 8) & 1,
            25 => (flags >> 7) & 1,
            29 => (flags >> 3) & 1,
            36 => b6,
            40 => b10,
            0x4000 => {
                if marker_arg(mem, p) == 0 {
                    p = p.wrapping_add(MARKER_LEN as u32);
                    continue;
                }
                flags & 1
            }
            _ => 0,
        };
        if matched != 0 {
            if first.is_none() {
                first = Some(p);
            }
            if let Some(count) = count.as_deref_mut() {
                *count = count.wrapping_add(1);
            }
        }
        p = p.wrapping_add(MARKER_LEN as u32);
    }
    first.unwrap_or(0)
}

/// fn_19861c / EU fn_1986cc / KR fn_198708.
///
/// Resume instrumentation hook: reports `0x88402007` for a null
/// handle and 0 otherwise. The tag arguments are ignored.
pub fn resume_stage_hook(handle: Option<u32>) -> u32 {
    if handle.is_some() {
        0
    } else {
        0x88402007
    }
}

/// fn_176f2c / EU fn_176fdc / KR fn_176f88.
///
/// Acquire the state lock: dispatch through the vtable's lock slot
/// with the lock target. A null handle reports `0x81402007`; a null
/// slot reports 0 without calling.
pub fn acquire_lock(mem: &[u8], p: Option<u32>, mut call: impl FnMut(u32) -> u32) -> u32 {
    let Some(p) = p else {
        return 0x81402007;
    };
    let inner = mem_word(mem, mem_word(mem, p).wrapping_add(8));
    if mem_word(mem, inner.wrapping_add(8)) == 0 {
        return 0;
    }
    call(mem_word(mem, p.wrapping_add(8)))
}

/// fn_176f5c / EU fn_17700c / KR fn_176fb8.
///
/// Release the state lock: dispatch through the vtable's unlock slot
/// with the lock target. A null handle reports `0x81402007`; a null
/// slot reports 0 without calling.
pub fn release_lock(mem: &[u8], p: Option<u32>, mut call: impl FnMut(u32) -> u32) -> u32 {
    let Some(p) = p else {
        return 0x81402007;
    };
    let inner = mem_word(mem, mem_word(mem, p).wrapping_add(8));
    if mem_word(mem, inner.wrapping_add(12)) == 0 {
        return 0;
    }
    call(mem_word(mem, p.wrapping_add(8)))
}

/// fn_19672c / EU fn_1967dc / KR fn_196818.
///
/// Remove `key` from the tree at `p` + 24: a null base or zero key
/// reports `0x89002007`, a miss reports `0x89002014`, and a hit runs
/// `remove` over the tree base and found node and reports 0.
pub fn remove_tree_value(
    mem: &[u8],
    p: Option<u32>,
    key: u32,
    mut cmp: impl FnMut(u32, u32) -> i32,
    mut remove: impl FnMut(u32, u32),
) -> u32 {
    let Some(p) = p else {
        return 0x89002007;
    };
    if key == 0 {
        return 0x89002007;
    }
    let tree = p.wrapping_add(0x18);
    let head = mem_word(mem, tree);
    let flag = mem[tree.wrapping_add(40) as usize];
    match find_node(mem, tree, head, flag, key, &mut cmp) {
        Some(node) => {
            remove(tree, node);
            0
        }
        None => 0x89002014,
    }
}

/// Cross-TU calls made by `fn_175ce0` (queue flush + param removal).
pub trait FlushHooks {
    /// `fn_196794`: open the pending queue iterator.
    fn open_iter(&mut self) -> (u32, u32);
    /// `fn_196860`: fetch the next pending entry, if any.
    fn next_entry(&mut self) -> Option<(u32, u32, u32)>;
    /// `fn_196830`: close the pending queue iterator.
    fn close_iter(&mut self);
    /// `fn_175170` / `fn_174b38`: write a parameter value.
    fn write_param(&mut self, key: u32, data: u32, size: u32, remove: bool) -> u32;
    /// `fn_176f2c` / `fn_176f5c`: lock and unlock the parameter set.
    fn lock(&mut self) -> u32;
    /// `fn_176f5c`: unlock (see `lock`).
    fn unlock(&mut self) -> u32;
    /// `fn_1966ac` + `fn_1970a0`: resolve a live value for removal.
    fn resolve_for_remove(&mut self, key: u32) -> (u32, Option<(u32, u32)>);
    /// `fn_19672c`: drop the value from the table.
    fn drop_value(&mut self, key: u32) -> u32;
}

/// fn_175ce0 / EU fn_175d90 / KR fn_175d3c.
///
/// Flush the pending queue into the parameter set: nonzero entries
/// update, zero-size entries remove (a `0x11` miss updates with an
/// empty string, a `0x14` miss skips). Reports the first error.
pub fn flush_pending<H: FlushHooks>(hooks: &mut H, handle: u32, present: bool) -> u32 {
    if handle == 0 || !present {
        return 0x8b702007;
    }
    let (mut rc, _) = hooks.open_iter();
    if rc >= 0x80000000 {
        return rc;
    }
    while let Some((key, data, size)) = hooks.next_entry() {
        let t = if size != 0 {
            hooks.write_param(key, data, size, false)
        } else {
            let t = remove_pending_param(hooks, key);
            match t & 0x1fff {
                0x11 => hooks.write_param(key, 0, 1, false),
                0x14 => continue,
                _ => t,
            }
        };
        if t >= 0x80000000 && rc < 0x80000000 {
            rc = t;
        }
    }
    hooks.close_iter();
    rc
}

fn remove_pending_param<H: FlushHooks>(hooks: &mut H, key: u32) -> u32 {
    let r = hooks.lock();
    if r >= 0x80000000 {
        return r;
    }
    let mut r = match hooks.resolve_for_remove(key) {
        (_, Some((data, size))) => {
            let r2 = hooks.write_param(key, data, size, true);
            if r2 >= 0x80000000 {
                r2
            } else {
                let r3 = hooks.drop_value(key);
                if r3 < 0x80000000 {
                    r3
                } else {
                    (r3 & 0x1fff) | 0x8b702000
                }
            }
        }
        (r, None) => {
            if r < 0x80000000 {
                0x8b702011
            } else {
                (r & 0x1fff) | 0x8b702000
            }
        }
    };
    let t = hooks.unlock();
    if t >= 0x80000000 && r < 0x80000000 {
        r = t;
    }
    r
}

/// fn_178530 / EU fn_1785e0 / KR fn_17858c.
///
/// Release a broker interface: reports 0 when both registry and
/// component are present, `0x81506007` otherwise. Log calls are void.
pub fn release_broker_interface(reg: Option<u32>, comp: Option<u32>) -> u32 {
    if reg.is_some() && comp.is_some() {
        0
    } else {
        0x81506007
    }
}

/// fn_19862c / EU fn_1986dc / KR fn_198718.
///
/// Trace-open hook: reports 0 for a present handle,
/// `0x88402007` for null. Further arguments are ignored.
pub fn begin_trace_hook(handle: Option<u32>) -> u32 {
    if handle.is_some() {
        0
    } else {
        0x88402007
    }
}

/// fn_19863c / EU fn_1986ec / KR fn_198728.
///
/// Trace-close hook: reports 0 when both the handle and log are
/// present, `0x88402007` otherwise. Further arguments are ignored.
pub fn end_trace_hook(handle: Option<u32>, log: Option<u32>) -> u32 {
    if handle.is_some() && log.is_some() {
        0
    } else {
        0x88402007
    }
}

/// fn_1985c4 / EU fn_198674 / KR fn_1986b0.
///
/// Release a trace entry: validate the handle tag, freeing the entry
/// through `free` on success. Reports the validation status.
pub fn release_trace_entry(
    mem: &[u8],
    p: Option<u32>,
    tag: u32,
    mut free: impl FnMut(u32, u32),
) -> u32 {
    let present = p.is_some();
    let rc = validate_handle(present, tag, 0xeffb, 0x124);
    if rc < 0x80000000 {
        let e = p.unwrap();
        let ctx = mem_word(mem, e.wrapping_add(8));
        free(mem_word(mem, ctx.wrapping_add(4)), e);
    }
    rc
}

/// Scripted cross-TU statuses plus the found entry for `fn_17b058`.
pub struct AcquireScripts {
    /// `fn_176f2c` lock status.
    pub lock: u32,
    /// `fn_1966ac` find status.
    pub find: u32,
    /// `fn_1966ac` entry output.
    pub found: Option<u32>,
    /// `fn_176f5c` unlock status.
    pub unlock: u32,
}

/// fn_17b058 / EU fn_17b108 / KR fn_17b0b4.
///
/// Acquire a linguistic map entry: a failed find remaps its status,
/// otherwise the entry is stored to `out` and its reference count
/// increments. Reports the find status (or the unlock status when the
/// find succeeded and the unlock failed).
pub fn acquire_ling_entry(
    mem: &mut [u8],
    o: Option<u32>,
    name: Option<u32>,
    out: Option<u32>,
    scripts: &AcquireScripts,
) -> u32 {
    if o.is_none() || name.is_none() || out.is_none() {
        return 0x8b602007;
    }
    if scripts.lock >= 0x80000000 {
        return scripts.lock;
    }
    let mut rc = if scripts.find >= 0x80000000 {
        (scripts.find & 0x1fff) | 0x8b602000
    } else {
        let slot = scripts.found.unwrap();
        mem_store(mem, out.unwrap(), slot);
        let at = slot.wrapping_add(28) as usize;
        let refs = u16::from_le_bytes(mem[at..at + 2].try_into().unwrap());
        mem[at..at + 2].copy_from_slice(&refs.wrapping_add(1).to_le_bytes());
        scripts.find
    };
    if scripts.unlock >= 0x80000000 && rc < 0x80000000 {
        rc = scripts.unlock;
    }
    rc
}

/// Cross-TU calls made by `fn_17ec38` (pipeline teardown).
pub trait TeardownHooks {
    /// Trace open/close calls (returns ignored).
    fn trace(&mut self, _which: u32) {}
    /// Per-component trace brackets (returns ignored).
    fn trace_comp(&mut self, _index: u32, _closing: bool) {}
    /// `vt->close`: close one component handle; reports its status.
    fn close_comp(&mut self, h0: u32, h1: u32) -> u32;
    /// `fn_178530`: release one component vtable; reports its status.
    fn release_broker(&mut self) -> u32;
    /// `fn_1985c4`: release the trace entry (return ignored).
    fn release_trace(&mut self) {}
    /// `fn_196428`: free one heap block.
    fn free_block(&mut self, heap: u32, block: u32);
}

/// fn_17ec38 / EU fn_17ece8 / KR fn_17ec94.
///
/// Tear down a pipeline: in reverse, drop refcounted components,
/// close the rest whose handle differs from `null_pair`, release
/// each vtable, and free each name and struct; then release the
/// trace entry, free the extras array, field4, and pipeline.
/// Reports the first close/release error.
pub fn teardown_pipeline<H: TeardownHooks>(
    mem: &mut [u8],
    cs: Option<u32>,
    null_pair: &[u8; 8],
    hooks: &mut H,
) -> u32 {
    let Some(cs) = cs else {
        return 0x81202006;
    };
    let mut r = 0u32;
    let ctx = mem_word(mem, cs);
    let heap = mem_word(mem, ctx.wrapping_add(4));
    let obj = mem_word(mem, cs.wrapping_add(16));
    let comps = mem_word(mem, cs.wrapping_add(8));
    if comps != 0 {
        if obj != 0 {
            hooks.trace(0);
            hooks.trace(1);
        }
        let count = mem_half(mem, cs.wrapping_add(12)) as i32;
        if count != 0 {
            let mut i = count;
            while i != 0 {
                i -= 1;
                let c = mem_word(mem, comps.wrapping_add((i as u16 as u32) * 4));
                if c == 0 {
                    continue;
                }
                let refs = mem_half(mem, c);
                if refs > 1 {
                    let at = c as usize;
                    mem[at..at + 2]
                        .copy_from_slice(&refs.wrapping_sub(1).to_le_bytes());
                    continue;
                }
                let h0 = mem_word(mem, c.wrapping_add(12));
                let h1 = mem_word(mem, c.wrapping_add(16));
                let n0 = u32::from_le_bytes(null_pair[0..4].try_into().unwrap());
                let n1 = u32::from_le_bytes(null_pair[4..8].try_into().unwrap());
                if match_halves(h0, h1, n0, n1) != 0 {
                    let vt = mem_word(mem, c.wrapping_add(8));
                    if vt != 0 {
                        let rc = hooks.release_broker();
                        if r < 0x80000000 && rc >= 0x80000000 {
                            r = rc;
                        }
                    }
                    hooks.free_block(heap, mem_word(mem, c.wrapping_add(4)));
                    hooks.free_block(heap, c);
                    continue;
                }
                if obj != 0 {
                    hooks.trace_comp(i as u16 as u32, true);
                }
                let rc = hooks.close_comp(h0, h1);
                if obj != 0 {
                    hooks.trace_comp(i as u16 as u32, false);
                }
                if rc >= 0x80000000 && r < 0x80000000 {
                    r = rc;
                }
                let vt = mem_word(mem, c.wrapping_add(8));
                if vt != 0 {
                    let rc = hooks.release_broker();
                    if r < 0x80000000 && rc >= 0x80000000 {
                        r = rc;
                    }
                }
                hooks.free_block(heap, mem_word(mem, c.wrapping_add(4)));
                hooks.free_block(heap, c);
            }
        }
        if obj != 0 {
            hooks.trace(2);
            hooks.trace(3);
        }
        hooks.free_block(heap, comps);
    }
    if obj != 0 {
        hooks.release_trace();
    }
    let extra = mem_word(mem, cs.wrapping_add(24));
    if extra != 0 {
        let count = mem_half(mem, cs.wrapping_add(12));
        let mut n = 0u16;
        while n < count.wrapping_add(3) {
            let block = mem_word(mem, extra.wrapping_add(n as u32 * 4));
            if block != 0 {
                hooks.free_block(heap, block);
            }
            n = n.wrapping_add(1);
        }
        hooks.free_block(heap, extra);
    }
    let field4 = mem_word(mem, cs.wrapping_add(4));
    if field4 != 0 {
        hooks.free_block(heap, field4);
    }
    hooks.free_block(heap, cs);
    r
}

/// Cross-TU calls made by `fn_1805f0` (pipeline reopen).
pub trait ReopenHooks {
    /// `fn_1759a0`: write one named parameter (return ignored).
    fn write_named(&mut self, tag: &[u8], val: &[u8]);
    /// `fn_17a3b8`: open a tag iterator; reports `(status, live)`.
    fn open_tags(&mut self) -> (u32, bool);
    /// `fn_17a814`: fetch the next tag, if any.
    fn next_tag(&mut self) -> Option<(u32, Option<u32>)>;
    /// `fn_17a2ec`: close the tag iterator.
    fn close_tags(&mut self);
    /// `fn_17b058`: acquire the named object entry.
    fn acquire_entry(&mut self) -> (u32, Option<(u32, u32, u32)>);
    /// `o->reopen`: reopen one entry handle; reports its status.
    fn reopen_entry(&mut self, h0: u32, h1: u32) -> u32;
    /// `fn_17b1c4`: release one entry by tag.
    fn release_entry(&mut self, tag: u32);
    /// `vt->unload`: unload one component handle; reports its status.
    fn unload_comp(&mut self, h0: u32, h1: u32) -> u32;
}

/// fn_1805f0 / EU fn_1806a0 / KR fn_18064c.
///
/// Reopen a pipeline under a new voice name: seed the fixed
/// parameters, apply registry tags, reopen each listed object, then
/// unload every component. Reports the first object/unload error, or
/// `0x81202004` for a malformed object tag.
///
/// The unload slot follows the ROM (vt+0x18), not the decomp's
/// `c->vt->unload` at +0x24 (src/fn_17ec38.c:732,734; reported C
/// defect, all regions). Likewise the malformed-tag error returns
/// immediately: the ROM jumps to done while the decomp `break`s
/// into 17a2ec + the unload loop (src/fn_17ec38.c:706-711), and an
/// unload error also returns at once while the decomp continues
/// the loop (src/fn_17ec38.c:735-737).
pub fn reopen_pipeline<H: ReopenHooks>(
    mem: &[u8],
    cs: u32,
    filter: &[u8],
    hooks: &mut H,
) -> u32 {
    let mut rc = 0u32;
    let count = mem_half(mem, cs.wrapping_add(12));
    let name = mem_word(mem, cs.wrapping_add(4));
    // Fixed parameter seed (the last tag is D_17f9cc).
    hooks.write_named(b"pipeline", read_cstr(mem, name));
    for (tag, val) in [
        (&b"mlset"[..], &b""[..]),
        (&b"voiceml"[..], &b""[..]),
        (&b"noclmset"[..], &b""[..]),
        (&b"extclccfg"[..], &b""[..]),
        (&b"fedatapackaging"[..], &b""[..]),
    ] {
        hooks.write_named(tag, val);
    }
    let (_, live) = hooks.open_tags();
    if live {
        while let Some((tag, val)) = hooks.next_tag() {
            if tag == 0 {
                continue;
            }
            if mem[tag as usize] == 0 {
                continue;
            }
            if compare_tag_prefix(mem, tag, filter) {
                continue;
            }
            let val = val.unwrap_or(0);
            hooks.write_named(
                read_cstr(mem, tag),
                if val == 0 { b"" } else { read_cstr(mem, val) },
            );
        }
        hooks.close_tags();
    }
    let (_, live) = hooks.open_tags();
    if live {
        loop {
            let Some((tag, val)) = hooks.next_tag() else {
                break;
            };
            let val = match val {
                Some(v) if tag != 0 && mem[v as usize] != 0 => v,
                _ => {
                    // C defect: src/fn_17ec38.c:706-711 `break`s to
                    // 17a2ec + the unload loop, but the ROM jumps to
                    // done (US 0x180880 b 0x1809fc; all regions
                    // agree), skipping both. The port follows the ROM.
                    return 0x81202004;
                }
            };
            let (st, entry) = hooks.acquire_entry();
            if st >= 0x80000000 {
                continue;
            }
            let Some((reopen, h0, h1)) = entry else {
                continue;
            };
            if reopen != 0 {
                let r2 = hooks.reopen_entry(h0, h1);
                rc = r2;
                if r2 >= 0x80000000 {
                    hooks.release_entry(tag);
                    return rc;
                }
            }
            hooks.release_entry(tag);
            let _ = val;
        }
        hooks.close_tags();
    }
    let comps = mem_word(mem, cs.wrapping_add(8));
    let mut i = 0u32;
    while i < count as u32 {
        let c = mem_word(mem, comps.wrapping_add(i * 4));
        let vt = mem_word(mem, c.wrapping_add(8));
        // C defect: src/fn_17ec38.c:732,734 reads c->vt->unload
        // at +0x24, but the ROM loads vt+0x18 (US 0x1808c4 e5900018
        // null-check / 0x1808f4 e5902018 call; same EU/KR), so the
        // port follows the ROM. The r2/r3 the ROM passes (its own
        // fn pointer, log residue) are ignored by the callee.
        if mem_word(mem, vt.wrapping_add(0x18)) != 0 {
            let h0 = mem_word(mem, c.wrapping_add(12));
            let h1 = mem_word(mem, c.wrapping_add(16));
            let r2 = hooks.unload_comp(h0, h1);
            // C defect: src/fn_17ec38.c:735-737 logs an unload
            // error and continues, but the ROM jumps to done (US
            // 0x180940 b 0x1809fc; all regions agree). The port
            // follows the ROM.
            if r2 >= 0x80000000 {
                return r2;
            }
            rc = r2;
        }
        i += 1;
    }
    rc
}

fn compare_tag_prefix(mem: &[u8], tag: u32, filter: &[u8]) -> bool {
    // Mirrors fn_195df8(tag, filter, 11) == 0.
    let mut i = 0usize;
    while i < 11 {
        let a = mem[tag as usize + i];
        let b = if i < filter.len() { filter[i] } else { 0 };
        if a != b {
            return false;
        }
        if a == 0 {
            return true;
        }
        i += 1;
    }
    true
}

fn read_cstr(mem: &[u8], at: u32) -> &[u8] {
    let start = at as usize;
    let len = mem[start..].iter().position(|&c| c == 0).unwrap();
    &mem[start..start + len]
}

/// Cross-TU calls made by `fn_1530d0` (US 1530d0 / EU 153194 / KR 1530b4),
/// the output pump: the user's `putOutput` sink plus the pcm/marker
/// stream methods and the marker-array realloc.
pub trait EmitHooks {
    /// `p->putOutput(user, msg, &buf, &len, arr, &nmark)`: deliver one
    /// buffer. `markers` holds the `n_in` entries at `arr` (empty when
    /// `arr` is null). Reports `(status, buf, len, nmark)`.
    fn put_output(
        &mut self,
        user: u32,
        msg: u32,
        len_in: u32,
        n_in: u32,
        markers: &[u8],
    ) -> (u32, Option<u32>, u32, u32);
    /// `p->vt->getAvail`: reports `(status, avail, eos)`.
    fn get_avail(&mut self, a: u32, b: u32) -> (u32, u32, u8);
    /// `p->vt->getBuf`: reports `(status, buf, len)`.
    fn get_buf(&mut self, a: u32, b: u32) -> (u32, Option<u32>, u32);
    /// `p->vt->getPos`: reports `(status, pos)`.
    fn get_pos(&mut self, a: u32, b: u32) -> (u32, u32);
    /// `p->vt->consume`: consume `n` units; reports its status.
    fn consume(&mut self, a: u32, b: u32, n: u32) -> u32;
    /// `fn_1963e0`: grow the marker array; reports the new block, if any.
    fn realloc_markers(&mut self) -> Option<u32>;
}

/// Pump pending pcm/markers out through the user's `putOutput` sink.
///
/// `p` is the `TtsSynth`, `sent` the optional delivery flag. Returns the
/// terminal status: a hook error, `0x8120200a` when the marker array
/// cannot grow, `0x81202807` for an empty user buffer, else 0.
pub fn emit_output<H: EmitHooks>(
    mem: &mut [u8],
    p: u32,
    sent: Option<u32>,
    hooks: &mut H,
) -> u32 {
    const OUT: u32 = 0x84;
    let o = p.wrapping_add(OUT);
    if let Some(s) = sent {
        mem[s as usize] = 0;
    }
    let put_output = mem_word(mem, p.wrapping_add(0x18));
    if put_output == 0 || mem[o.wrapping_add(0x5e) as usize] == 2 {
        return 0;
    }
    if mem[p.wrapping_add(0x80) as usize] == 2 {
        return 0;
    }
    let user = mem_word(mem, p.wrapping_add(0x28));
    let mut msg_type = 64u32;
    let mut rc;
    let mut out_buf = mem_word(mem, o.wrapping_add(0x60));
    if out_buf == 0 {
        mem[o.wrapping_add(0xc) as usize] = 1;
        mem_store(mem, o.wrapping_add(0x64), 0);
        mem_store(mem, o.wrapping_add(0x68), 0);
        let (r, buf, len, nmark) = hooks.put_output(user, 1, 0, 0, &[]);
        rc = r;
        mem_store(mem, o.wrapping_add(0x60), buf.unwrap_or(0));
        mem_store(mem, o.wrapping_add(0x64), len);
        mem_store(mem, o.wrapping_add(0x68), nmark);
        out_buf = buf.unwrap_or(0);
        if rc >= 0x80000000 {
            return rc;
        }
        if out_buf == 0 || mem_word(mem, o.wrapping_add(0x64)) == 0 {
            return 0x81202807;
        }
    }
    let bits = mem_word(mem, o.wrapping_add(8));
    let pcm_a = mem_word(mem, o.wrapping_add(0x3c));
    let pcm_b = mem_word(mem, o.wrapping_add(0x40));
    let (r, mut pcm_avail, eos) = hooks.get_avail(pcm_a, pcm_b);
    rc = r;
    if rc >= 0x80000000 {
        return rc;
    }
    let mut pcm_buf = None;
    if pcm_avail != 0 {
        let (r, buf, len) = hooks.get_buf(pcm_a, pcm_b);
        rc = r;
        if rc >= 0x80000000 {
            return rc;
        }
        pcm_buf = buf;
        pcm_avail = len;
    }
    let mk_a = mem_word(mem, o.wrapping_add(0x44));
    let mk_b = mem_word(mem, o.wrapping_add(0x48));
    let (r, mk_buf, mk_len) = hooks.get_buf(mk_a, mk_b);
    rc = r;
    if rc >= 0x80000000 {
        return rc;
    }
    let marker_avail = mk_len / 24;
    let (_, pos) = hooks.get_pos(pcm_a, pcm_b);
    let mut min_pos = if bits == 16 {
        pos >> 1
    } else if bits == 8 {
        pos
    } else {
        divmod_u32(pos << 3, bits).0
    };
    let flags = mem_word(mem, p.wrapping_add(0x34));
    let mut pcm_used = 0u32;
    let mut marker_used = 0u32;
    while mem[o.wrapping_add(0x5e) as usize] != 2
        && (pcm_avail > pcm_used || marker_avail > marker_used)
    {
        let out_len = mem_word(mem, o.wrapping_add(0x64));
        let mut copy_len = pcm_avail.wrapping_sub(pcm_used);
        let cap = out_len.wrapping_mul(bits) >> 3;
        if copy_len >= cap {
            copy_len = cap;
        }
        let first = match mk_buf {
            Some(mk) => mk.wrapping_add(marker_used.wrapping_mul(24)),
            None => 0,
        };
        let nmark = mem_word(mem, o.wrapping_add(0x68));
        let mut n = 0u32;
        if nmark == 0 || flags == 0 || mk_buf.is_none() || marker_used == marker_avail {
            marker_used = marker_avail;
        } else {
            let mk = mk_buf.unwrap();
            let (_, base) = hooks.get_pos(pcm_a, pcm_b);
            let end = if bits == 16 {
                base.wrapping_add(pcm_used).wrapping_add(copy_len) >> 1
            } else if bits == 8 {
                base.wrapping_add(pcm_used).wrapping_add(copy_len)
            } else {
                divmod_u32(base.wrapping_add(pcm_used).wrapping_add(copy_len) << 3, bits).0
            };
            let mk_end = mk.wrapping_add(marker_avail.wrapping_mul(24));
            let mut cur = first;
            let mut max_pos = min_pos;
            let mut cnt = 0u32;
            let mut m = 0u32;
            let mut all = false;
            loop {
                m = filter_markers(mem, flags, cur, mk_end, Some(&mut cnt));
                if m == 0 {
                    all = true;
                    break;
                }
                if cnt.wrapping_add(n) > nmark
                    || (eos == 0 && mem_word(mem, m.wrapping_add(12)) >= end)
                {
                    break;
                }
                n = n.wrapping_add(1);
                cur = m.wrapping_add(24);
                let m_pos = mem_word(mem, m.wrapping_add(12));
                if m_pos > max_pos {
                    max_pos = m_pos;
                }
            }
            if !all {
                let m_pos = mem_word(mem, m.wrapping_add(12));
                if m_pos < end {
                    if m_pos >= max_pos {
                        max_pos = m_pos;
                    }
                    let t = if bits == 16 {
                        max_pos.wrapping_mul(2).wrapping_sub(base.wrapping_add(pcm_used)) as i32
                    } else if bits == 8 {
                        max_pos.wrapping_sub(base.wrapping_add(pcm_used)) as i32
                    } else {
                        (bits.wrapping_mul(max_pos) >> 3).wrapping_sub(base.wrapping_add(pcm_used))
                            as i32
                    };
                    copy_len = if t < 0 { 0 } else { t as u32 };
                    if t <= 0 && nmark < cnt {
                        loop {
                            m = filter_markers(mem, flags, cur, mk_end, None);
                            if m == 0 {
                                all = true;
                                break;
                            }
                            if nmark <= n {
                                break;
                            }
                            n = n.wrapping_add(1);
                            cur = m.wrapping_add(24);
                        }
                    }
                }
            }
            if all {
                marker_used = marker_avail;
            } else {
                marker_used = m.wrapping_sub(mk) / 24;
            }
        }
        if copy_len == 0 && n == 0 {
            if rc >= 0x80000000 {
                return rc;
            }
            break;
        }
        let cap_now = mem_word(mem, o.wrapping_add(0x74));
        if cap_now < n {
            match hooks.realloc_markers() {
                Some(arr) => {
                    mem_store(mem, o.wrapping_add(0x78), arr);
                    mem_store(mem, o.wrapping_add(0x74), n);
                }
                None => return 0x8120200a,
            }
        }
        if n != 0 {
            let arr = mem_word(mem, o.wrapping_add(0x78));
            let pos_offset = mem_word(mem, o.wrapping_add(0x70));
            let mk = mk_buf.unwrap();
            let mk_end = mk.wrapping_add(marker_avail.wrapping_mul(24));
            let mut cur = first;
            let mut i = 0u32;
            while i < n {
                let m = filter_markers(mem, flags, cur, mk_end, None);
                if m == 0 {
                    break;
                }
                let dst = arr.wrapping_add(i.wrapping_mul(24)) as usize;
                mem.copy_within(m as usize..m as usize + 24, dst);
                let at = dst.wrapping_add(12);
                let v = u32::from_le_bytes(mem[at..at + 4].try_into().unwrap());
                let wpos = v.wrapping_add(pos_offset);
                mem[at..at + 4].copy_from_slice(&wpos.to_le_bytes());
                if wpos < min_pos {
                    mem[at..at + 4].copy_from_slice(&min_pos.to_le_bytes());
                } else {
                    min_pos = wpos;
                }
                cur = m.wrapping_add(24);
                i = i.wrapping_add(1);
            }
        }
        mem[o.wrapping_add(0xc) as usize] = 1;
        if let Some(s) = sent {
            mem[s as usize] = 1;
        }
        mem_store(mem, o.wrapping_add(0x50), 0);
        if copy_len != 0 {
            let src_us = pcm_buf.unwrap() as usize + pcm_used as usize;
            let dst_us = out_buf as usize;
            let count = copy_len as usize;
            if src_us < dst_us {
                let (head, tail) = mem.split_at_mut(dst_us);
                copy_bytes(&mut tail[..count], &head[src_us..src_us + count], copy_len);
            } else if src_us > dst_us {
                let (head, tail) = mem.split_at_mut(src_us);
                copy_bytes(&mut head[dst_us..dst_us + count], &tail[..count], copy_len);
            }
            pcm_used = pcm_used.wrapping_add(copy_len);
        }
        let new_len = if bits == 16 {
            copy_len >> 1
        } else if bits == 8 {
            copy_len
        } else {
            divmod_u32(copy_len << 3, bits).0
        };
        mem_store(mem, o.wrapping_add(0x64), new_len);
        mem_store(mem, o.wrapping_add(0x68), n);
        let restart = mem[o.wrapping_add(0x4c) as usize];
        if restart == 1 {
            mem[o.wrapping_add(0x4c) as usize] = 0;
        }
        if let Some(mk) = mk_buf {
            let mut j = first;
            // Scan to &markers[markerAvail] (src/fn_151914.c:1093
            // matches the ROM since 300e7b8).
            let stop = mk.wrapping_add(marker_avail.wrapping_mul(24));
            while j != stop {
                let ty = u16::from_le_bytes(mem[j as usize..j as usize + 2].try_into().unwrap());
                let arg = mem_word(mem, j.wrapping_add(8));
                if ty == 0x4000 && arg != 0 {
                    msg_type = 32;
                    break;
                }
                j = j.wrapping_add(24);
            }
            if eos != 0 && pcm_used == pcm_avail {
                if msg_type == 64 {
                    msg_type = 128;
                } else {
                    msg_type |= 128;
                }
            }
        }
        let arr_now = mem_word(mem, o.wrapping_add(0x78));
        let n_now = mem_word(mem, o.wrapping_add(0x68));
        let take = (n_now as usize).saturating_mul(24);
        let (r, buf, len, nmark) = hooks.put_output(
            user,
            msg_type,
            mem_word(mem, o.wrapping_add(0x64)),
            n_now,
            &mem[arr_now as usize..arr_now as usize + take],
        );
        rc = r;
        mem_store(mem, o.wrapping_add(0x60), buf.unwrap_or(0));
        mem_store(mem, o.wrapping_add(0x64), len);
        mem_store(mem, o.wrapping_add(0x68), nmark);
        out_buf = buf.unwrap_or(0);
        if rc >= 0x80000000 {
            // Error is only logged; fall through to the state update.
        } else if out_buf == 0 || mem_word(mem, o.wrapping_add(0x64)) == 0 {
            rc = 0x81202807;
        }
        if mem[o.wrapping_add(0x5e) as usize] == 1 {
            mem[o.wrapping_add(0x5e) as usize] = 2;
            mem_store(mem, o.wrapping_add(0x60), 0);
            out_buf = 0;
        }
        if rc >= 0x80000000 {
            return rc;
        }
    }
    if pcm_used != 0 {
        rc = hooks.consume(pcm_a, pcm_b, pcm_used);
        if rc >= 0x80000000 {
            return rc;
        }
    }
    if marker_used != 0 {
        rc = hooks.consume(mk_a, mk_b, marker_used.wrapping_mul(24));
    }
    rc
}

/// Cross-TU calls made by `fn_1561b8` (US 1561b8 / EU 15627c / KR 15619c),
/// the output teardown: stream close/release, the null pair, the lex
/// close/free pair, and the pipeline swap pair. The inner
/// `fn_1530d0` pump arrives through [`EmitHooks`].
pub trait SynthTeardownHooks {
    /// `p->vt->close`: closes one stream; reports its status. Serves
    /// the pp/pcm/marker closes and the marker-stream close inside
    /// `fn_154108` (which ignores the status).
    fn close_stream(&mut self, a: u32, b: u32) -> u32;
    /// `c->vt->processEnd`: ends one component; reports its status.
    fn process_end(&mut self, h0: u32, h1: u32) -> u32;
    /// `p->vt->release` / `p->vt2->release`: reports its status.
    fn release_stream(&mut self, a: u32, b: u32) -> u32;
    /// `fn_176fd4`: the null pair (`D_8d7660`).
    fn null_pair(&mut self) -> (u32, u32);
    /// `fn_175ce0`: reports its status.
    fn lex_close(&mut self, params: u32, lex: u32) -> u32;
    /// `fn_175ef0`: reports its status.
    fn lex_free(&mut self, params: u32, lex: u32) -> u32;
    /// `fn_17ec38`: tears down the outgoing pipeline; reports its status.
    fn retire_pipeline(&mut self, cs: u32) -> u32;
    /// `fn_1805f0`: reopens the incoming pipeline; reports its status.
    fn reopen_pipeline_cs(&mut self, cs: u32) -> u32;
}

/// `fn_1561b8` / EU `fn_15627c` / KR `fn_15619c`.
///
/// Tears the output side down: ends components `0..curComp`, closes
/// the pp/pcm/marker streams (flushing the pump first for pcm),
/// sends DONE, frees the marker array, swaps in the pending
/// pipeline, releases the pipe/aux streams, and closes the lex
/// handle. Returns the first error in that order, else 0. `err`,
/// `in_len` and `dur` feed only the dropped trace call.
pub fn teardown_synth<H: SynthTeardownHooks + EmitHooks>(
    mem: &mut [u8],
    p: u32,
    _err: u32,
    _in_len: u32,
    _dur: u32,
    hooks: &mut H,
) -> u32 {
    const OUT: u32 = 0x84;
    let o = p.wrapping_add(OUT);
    let bits = mem_word(mem, o.wrapping_add(8));
    let user = mem_word(mem, p.wrapping_add(0x28));
    let ctx = mem_word(mem, o);
    let cur = mem_half(mem, o.wrapping_add(0x4e)) as u32;
    let cell = RefCell::new(hooks);
    let mut rc = close_components(
        mem,
        p,
        0,
        cur,
        |_, _, _| {},
        |a, b| {
            cell.borrow_mut().close_stream(a, b);
        },
        |h0, h1| cell.borrow_mut().process_end(h0, h1),
        |_, _, _| {},
    );
    let hooks = cell.into_inner();
    let pp_a = mem_word(mem, o.wrapping_add(0x28));
    let pp_b = mem_word(mem, o.wrapping_add(0x2c));
    if pp_a != 0 {
        let rc2 = hooks.close_stream(pp_a, pp_b);
        let (t0, t1) = hooks.null_pair();
        mem_store(mem, o.wrapping_add(0x28), t0);
        mem_store(mem, o.wrapping_add(0x2c), t1);
        if rc < 0x80000000 && rc2 >= 0x80000000 {
            rc = rc2;
        }
    }
    let pcm_a = mem_word(mem, o.wrapping_add(0x3c));
    let pcm_b = mem_word(mem, o.wrapping_add(0x40));
    let mk_a = mem_word(mem, o.wrapping_add(0x44));
    let mk_b = mem_word(mem, o.wrapping_add(0x48));
    let mut rc2 = 0u32;
    if pcm_a != 0 {
        let r = emit_output(mem, p, None, &mut *hooks);
        if r >= 0x80000000 {
            rc2 = r;
        }
        let (r, pos) = hooks.get_pos(pcm_a, pcm_b);
        if r < 0x80000000 {
            let off = mem_word(mem, o.wrapping_add(0x70));
            mem_store(
                mem,
                o.wrapping_add(0x70),
                off.wrapping_add(divmod_u32(pos << 3, bits).0),
            );
        } else if rc2 < 0x80000000 {
            rc2 = r;
        }
        let r = hooks.close_stream(pcm_a, pcm_b);
        let (t0, t1) = hooks.null_pair();
        mem_store(mem, o.wrapping_add(0x3c), t0);
        mem_store(mem, o.wrapping_add(0x40), t1);
        if rc2 < 0x80000000 && r >= 0x80000000 {
            rc2 = r;
        }
    }
    if mk_a != 0 {
        let r = hooks.close_stream(mk_a, mk_b);
        let (t0, t1) = hooks.null_pair();
        mem_store(mem, o.wrapping_add(0x44), t0);
        mem_store(mem, o.wrapping_add(0x48), t1);
        if rc2 < 0x80000000 && r >= 0x80000000 {
            rc2 = r;
        }
    }
    if rc < 0x80000000 && rc2 >= 0x80000000 {
        rc = rc2;
    }
    if rc < 0x80000000 || mem[o.wrapping_add(0xc) as usize] != 0 {
        mem_store(mem, o.wrapping_add(0x64), 0);
        mem_store(mem, o.wrapping_add(0x68), 0);
        let mut r = 0u32;
        if mem_word(mem, p.wrapping_add(0x18)) != 0 {
            let (rr, buf, len, nmark) = hooks.put_output(user, 2, 0, 0, &[]);
            r = rr;
            mem_store(mem, o.wrapping_add(0x60), buf.unwrap_or(0));
            mem_store(mem, o.wrapping_add(0x64), len);
            mem_store(mem, o.wrapping_add(0x68), nmark);
        }
        if r >= 0x80000000 && rc < 0x80000000 {
            rc = r;
        }
    }
    if mem_word(mem, o.wrapping_add(0x78)) != 0 {
        mem_store(mem, o.wrapping_add(0x78), 0);
    }
    let new_cs = mem_word(mem, o.wrapping_add(0xb8));
    if new_cs != 0 {
        let r = hooks.retire_pipeline(mem_word(mem, p.wrapping_add(0x7c)));
        if r >= 0x80000000 && rc < 0x80000000 {
            rc = r;
        }
        mem_store(mem, p.wrapping_add(0x7c), new_cs);
        mem_store(mem, o.wrapping_add(0xb8), 0);
        let r = hooks.reopen_pipeline_cs(new_cs);
        if r >= 0x80000000 && rc < 0x80000000 {
            rc = r;
        }
    }
    if mem[o.wrapping_add(0x18) as usize] != 0 {
        let r = hooks.release_stream(
            mem_word(mem, o.wrapping_add(0x10)),
            mem_word(mem, o.wrapping_add(0x14)),
        );
        if r >= 0x80000000 && rc < 0x80000000 {
            rc = r;
        }
    }
    if mem[o.wrapping_add(0x24) as usize] != 0 {
        let r = hooks.release_stream(
            mem_word(mem, o.wrapping_add(0x1c)),
            mem_word(mem, o.wrapping_add(0x20)),
        );
        if r >= 0x80000000 && rc < 0x80000000 {
            rc = r;
        }
    }
    let lex = mem_word(mem, o.wrapping_add(0x30));
    if lex != 0 {
        let params = mem_word(mem, ctx.wrapping_add(20));
        let r = hooks.lex_close(params, lex);
        if r >= 0x80000000 && rc < 0x80000000 {
            rc = r;
        }
        let r = hooks.lex_free(params, lex);
        if r >= 0x80000000 && rc < 0x80000000 {
            rc = r;
        }
    }
    rc
}

/// Audio event record moved by `fn_22d6dc` (24 bytes).
pub const INSERT_EVENT_LEN: u32 = 24;

/// Cross-TU calls made by `fn_22d6dc` (void log calls and the
/// log-gate probe elided: neither writes an observed byte).
pub trait InserterPumpHooks {
    /// Buffer `available`: report `count` bytes with end flag `end`.
    fn buf_available(&mut self, h0: u32, h1: u32, count: &mut u32, end: &mut u8) -> u32;
    /// Buffer `read`: expose `*count` input bytes at `*data`.
    fn buf_read(&mut self, h0: u32, h1: u32, data: &mut u32, count: &mut u32) -> u32;
    /// Buffer `write`: expose the output events at `*data`.
    fn buf_write(&mut self, h0: u32, h1: u32, data: &mut u32, count: u32) -> u32;
    /// Buffer `consume`: release `n` input bytes.
    fn buf_consume(&mut self, h0: u32, h1: u32, n: u32) -> u32;
    /// Buffer `close`.
    fn buf_close(&mut self, h0: u32, h1: u32) -> u32;
    /// Buffer `copy`: move `n` bytes, reporting `moved`.
    fn buf_copy(
        &mut self,
        d0: u32,
        d1: u32,
        s0: u32,
        s1: u32,
        n: u32,
        moved: &mut u32,
    ) -> u32;
    /// Buffer `position`: report the read position.
    fn buf_position(&mut self, h0: u32, h1: u32, pos: &mut u32) -> u32;
    /// Buffer `commit`: publish `n` output bytes.
    fn buf_commit(&mut self, h0: u32, h1: u32, n: u32) -> u32;
    /// Inserter `resolve`: map `uri` to a name.
    fn ins_resolve(
        &mut self,
        s0: u32,
        s1: u32,
        uri: u32,
        n0: u32,
        n1: u32,
        out: &mut u32,
    ) -> u32;
    /// Inserter `open`: start an insertion, writing position,
    /// remaining and handle.
    fn ins_open(
        &mut self,
        s0: u32,
        s1: u32,
        name: u32,
        pos: &mut u32,
        rem: &mut u32,
        ins: &mut (u32, u32),
    ) -> u32;
    /// Inserter `close`.
    fn ins_close(&mut self, h0: u32, h1: u32, flag: u32) -> u32;
    /// `fn_22c85c`: run one insertion step (stubbed in the row:
    /// the render never reaches it).
    fn pump_remote(&mut self, mem: &mut [u8], base: u32) -> u32;
    /// `fn_176fd4`: a fresh null pair.
    fn null_pair(&mut self) -> (u32, u32);
}

/// fn_22d6dc / EU fn_22d898 / KR fn_22d944.
///
/// Pump one events buffer through the audio inserter at `base`:
/// with an insertion already open, step it and tear it down when
/// it drains; with input events, rebase their positions, fold
/// type-8 timing and open type-0x1a insertions, then commit the
/// sorted outputs; with neither, close out ended streams or copy
/// the audio tail. Reports progress through `status` (1 open, 0
/// stalled, 2 drained).
pub fn pump_insertion<H: InserterPumpHooks>(
    hooks: &mut H,
    mem: &mut [u8],
    base: u32,
    check: u32,
    status: &mut u8,
) -> u32 {
    fn pair(mem: &[u8], at: u32) -> (u32, u32) {
        (mem_word(mem, at), mem_word(mem, at.wrapping_add(4)))
    }
    fn store_pair(mem: &mut [u8], at: u32, value: (u32, u32)) {
        mem_store(mem, at, value.0);
        mem_store(mem, at.wrapping_add(4), value.1);
    }
    if validate_handle(true, check, 0x34ea, 0x74) >= 0x80000000 {
        return 0x8b902008;
    }
    *status = 1;
    if mem_word(mem, base.wrapping_add(48)) == 0 {
        return 0;
    }
    if mem_word(mem, base.wrapping_add(80)) != 0 {
        let stepped = hooks.pump_remote(mem, base);
        if stepped < 0x80000000 && mem_word(mem, base.wrapping_add(92)) == 0 {
            let mut result = 0;
            if mem_word(mem, base.wrapping_add(80)) != 0 {
                let (i0, i1) = pair(mem, base.wrapping_add(80));
                result = hooks.ins_close(i0, i1, 1);
                store_pair(mem, base.wrapping_add(80), hooks.null_pair());
            }
            mem_store(mem, base.wrapping_add(88), 0);
            mem_store(mem, base.wrapping_add(92), 0);
            *status = 2;
            return result;
        }
        *status = 2;
        return stepped;
    }
    let (ein0, ein1) = pair(mem, base.wrapping_add(32));
    let mut count = 0u32;
    let mut events_end = 0u8;
    let mut rc = hooks.buf_available(ein0, ein1, &mut count, &mut events_end);
    if rc >= 0x80000000 {
        return rc;
    }
    let mut copy_audio = true;
    if count != 0 {
        let mut input = 0u32;
        rc = hooks.buf_read(ein0, ein1, &mut input, &mut count);
        if rc >= 0x80000000 {
            return rc;
        }
        let (eout0, eout1) = pair(mem, base.wrapping_add(40));
        let mut output = 0u32;
        rc = hooks.buf_write(eout0, eout1, &mut output, count);
        if rc >= 0x80000000 {
            hooks.buf_consume(ein0, ein1, 0);
            return rc;
        }
        let events = count / INSERT_EVENT_LEN;
        let mut i = 0u32;
        let mut processed = 0u32;
        while i < events {
            let ie = input.wrapping_add(i.wrapping_mul(INSERT_EVENT_LEN));
            let oe = output.wrapping_add(i.wrapping_mul(INSERT_EVENT_LEN));
            if mem_word(mem, ie.wrapping_add(12)) != mem_word(mem, base.wrapping_add(72)) {
                mem_store(mem, base.wrapping_add(76), 0);
            }
            let mut tmp = [0u8; 24];
            tmp.copy_from_slice(&mem[ie as usize..ie as usize + 24]);
            mem[oe as usize..oe as usize + 24].copy_from_slice(&tmp);
            let pos = mem_word(mem, oe.wrapping_add(12))
                .wrapping_add(mem_word(mem, base.wrapping_add(68)));
            mem_store(mem, oe.wrapping_add(12), pos);
            if mem_half(mem, oe) != 0x21 {
                let pos = mem_word(mem, oe.wrapping_add(12))
                    .wrapping_add(mem_word(mem, base.wrapping_add(76)));
                mem_store(mem, oe.wrapping_add(12), pos);
            }
            processed += 1;
            let itype = mem_half(mem, ie);
            if itype == 8 {
                let ipos = mem_word(mem, ie.wrapping_add(12));
                mem_store(mem, base.wrapping_add(72), ipos);
                let add = mem_word(mem, ie.wrapping_add(20))
                    .wrapping_mul(mem_word(mem, base.wrapping_add(60)))
                    / 1000;
                let acc = mem_word(mem, base.wrapping_add(76)).wrapping_add(add);
                mem_store(mem, base.wrapping_add(76), acc);
            } else if itype == 0x1a {
                let mut position = 0u32;
                let mut available = 0u32;
                let mut audio_end = 0u8;
                if mem_word(mem, base.wrapping_add(16)) != 0 {
                    let (a0, a1) = pair(mem, base.wrapping_add(16));
                    hooks.buf_position(a0, a1, &mut position);
                    rc = hooks.buf_available(a0, a1, &mut available, &mut audio_end);
                    position >>= 1;
                    available >>= 1;
                } else {
                    audio_end = 1;
                }
                let target = mem_word(mem, base.wrapping_add(76))
                    .wrapping_add(mem_word(mem, ie.wrapping_add(12)));
                if rc >= 0x80000000
                    || (audio_end == 0 && target >= position.wrapping_add(available))
                {
                    processed = processed.wrapping_sub(1);
                    break;
                }
                copy_audio = false;
                if mem_word(mem, base.wrapping_add(16)) != 0 && position < target {
                    let (d0, d1) = pair(mem, base.wrapping_add(24));
                    let (s0, s1) = pair(mem, base.wrapping_add(16));
                    let n = target.wrapping_sub(position).wrapping_mul(2);
                    rc = hooks.buf_copy(d0, d1, s0, s1, n, &mut available);
                }
                if audio_end != 0 {
                    let (a0, a1) = pair(mem, base.wrapping_add(24));
                    hooks.buf_position(a0, a1, &mut position);
                    position >>= 1;
                    if rc < 0x80000000 && position < mem_word(mem, oe.wrapping_add(12)) {
                        let samples =
                            mem_word(mem, oe.wrapping_add(12)).wrapping_sub(position);
                        let mut silence = 0u32;
                        rc = hooks.buf_write(a0, a1, &mut silence, samples.wrapping_mul(2));
                        if rc < 0x80000000 {
                            rc = hooks.buf_commit(a0, a1, samples.wrapping_mul(2));
                        }
                    }
                }
                if rc < 0x80000000 {
                    let mut attempt = 1u32;
                    let uri = mem_word(mem, ie.wrapping_add(20));
                    let mut name = uri;
                    rc = 0x8b902014;
                    while rc & 0x1fff == 0x14 && attempt <= 2 {
                        if attempt == 2 {
                            let (s0, s1) = pair(mem, base.wrapping_add(52));
                            let null = hooks.null_pair();
                            let mut resolved = 0u32;
                            rc = hooks.ins_resolve(s0, s1, uri, null.0, null.1, &mut resolved);
                            if rc >= 0x80000000 {
                                break;
                            }
                            name = resolved;
                        }
                        let (s0, s1) = pair(mem, base.wrapping_add(52));
                        let mut opos = 0u32;
                        let mut orem = 0u32;
                        let mut oins = (0u32, 0u32);
                        rc = hooks.ins_open(s0, s1, name, &mut opos, &mut orem, &mut oins);
                        mem_store(mem, base.wrapping_add(88), opos);
                        mem_store(mem, base.wrapping_add(92), orem);
                        store_pair(mem, base.wrapping_add(80), oins);
                        attempt += 1;
                    }
                    if rc < 0x80000000 {
                        rc = hooks.pump_remote(mem, base);
                        if rc < 0x80000000 && mem_word(mem, base.wrapping_add(92)) == 0 {
                            rc = 0;
                            if mem_word(mem, base.wrapping_add(80)) != 0 {
                                let (i0, i1) = pair(mem, base.wrapping_add(80));
                                rc = hooks.ins_close(i0, i1, 1);
                                store_pair(mem, base.wrapping_add(80), hooks.null_pair());
                            }
                            mem_store(mem, base.wrapping_add(88), 0);
                            mem_store(mem, base.wrapping_add(92), 0);
                        }
                    }
                }
                *status = 2;
                break;
            }
            i += 1;
        }
        if rc < 0x80000000 {
            if processed != 0 {
                loop {
                    let mut swapped = false;
                    let mut j = 0u32;
                    while j.wrapping_add(1) < processed {
                        let a = output.wrapping_add(j.wrapping_mul(INSERT_EVENT_LEN));
                        let b = output.wrapping_add(
                            j.wrapping_add(1).wrapping_mul(INSERT_EVENT_LEN),
                        );
                        if mem_word(mem, b.wrapping_add(12)) < mem_word(mem, a.wrapping_add(12))
                        {
                            let mut ta = [0u8; 24];
                            let mut tb = [0u8; 24];
                            ta.copy_from_slice(&mem[a as usize..a as usize + 24]);
                            tb.copy_from_slice(&mem[b as usize..b as usize + 24]);
                            mem[a as usize..a as usize + 24].copy_from_slice(&tb);
                            mem[b as usize..b as usize + 24].copy_from_slice(&ta);
                            swapped = true;
                        }
                        j += 1;
                    }
                    if !swapped {
                        break;
                    }
                }
            }
            let (e0, e1) = pair(mem, base.wrapping_add(40));
            rc = hooks.buf_commit(e0, e1, processed.wrapping_mul(INSERT_EVENT_LEN));
            if rc >= 0x80000000 {
                hooks.buf_consume(ein0, ein1, 0);
            } else {
                rc = hooks.buf_consume(ein0, ein1, processed.wrapping_mul(INSERT_EVENT_LEN));
            }
        }
        if processed == events && events_end != 0 {
            *status = 2;
        }
    } else {
        let mut available = 0u32;
        let mut audio_end = 0u8;
        let mut avail_err = false;
        if mem_word(mem, base.wrapping_add(16)) != 0 {
            let (a0, a1) = pair(mem, base.wrapping_add(16));
            rc = hooks.buf_available(a0, a1, &mut available, &mut audio_end);
            if rc >= 0x80000000 {
                avail_err = true;
            }
        } else {
            audio_end = 1;
        }
        if !avail_err && events_end != 0 && audio_end != 0 {
            if mem_word(mem, base.wrapping_add(40)) != 0 {
                let (a0, a1) = pair(mem, base.wrapping_add(24));
                hooks.buf_close(a0, a1);
                store_pair(mem, base.wrapping_add(24), hooks.null_pair());
                let (e0, e1) = pair(mem, base.wrapping_add(40));
                rc = hooks.buf_close(e0, e1);
                store_pair(mem, base.wrapping_add(40), hooks.null_pair());
            }
            copy_audio = false;
        } else if available == 0 {
            copy_audio = false;
            *status = 0;
        }
    }
    if rc < 0x80000000 && mem_word(mem, base.wrapping_add(16)) != 0 && copy_audio {
        let (d0, d1) = pair(mem, base.wrapping_add(24));
        let (s0, s1) = pair(mem, base.wrapping_add(16));
        let mut moved = 0u32;
        rc = hooks.buf_copy(d0, d1, s0, s1, 0xffffffff, &mut moved);
    }
    rc
}

/// Cross-TU calls made by `fn_1fe040` and `fn_1fe094`.
pub trait TextStateHooks {
    /// `fn_19611c`: resolve the state's handle to a context.
    fn resolve_context(&mut self, h0: u32, h1: u32, ctx: &mut u32) -> u32;
    /// `fn_1970a0`: read the source count.
    fn read_count(&mut self, src: u32) -> u32;
}

/// fn_1fe040 / EU fn_1fe1fc / KR fn_1fe2a8.
///
/// Refresh the text state's item count from its source once the
/// handle resolves.
pub fn refresh_text_count<H: TextStateHooks>(
    hooks: &mut H,
    mem: &mut [u8],
    base: u32,
    check: u32,
) -> u32 {
    let result = validate_handle(true, check, 0x11d, 0x48);
    if result >= 0x80000000 {
        return result;
    }
    let mut ctx = 0u32;
    let result = hooks.resolve_context(
        mem_word(mem, base),
        mem_word(mem, base.wrapping_add(4)),
        &mut ctx,
    );
    if result >= 0x80000000 {
        return result;
    }
    let count = hooks.read_count(mem_word(mem, base.wrapping_add(0x40)));
    mem_store(mem, base.wrapping_add(0x38), count);
    result
}

/// Element destructor behind `fn_19703c`.
pub trait ItemsDtorHooks {
    /// Run one element destructor over `elem` with `ctx`.
    fn destroy_elem(&mut self, elem: u32, ctx: u32);
}

/// fn_19703c / EU fn_1970ec / KR fn_197128.
///
/// Run the items destructor over every element, then clear the
/// count. A null block destroys nothing.
pub fn destroy_items<H: ItemsDtorHooks>(
    hooks: &mut H,
    mem: &mut [u8],
    base: Option<u32>,
) -> u32 {
    let Some(base) = base else {
        return 0;
    };
    if mem_word(mem, base.wrapping_add(12)) != 0 {
        let data = mem_word(mem, base);
        let stride = mem_half(mem, base.wrapping_add(4)) as u32;
        let count = mem_word(mem, base.wrapping_add(20));
        let ctx = mem_word(mem, base.wrapping_add(28));
        let mut i = 0u32;
        while i < count {
            hooks.destroy_elem(data.wrapping_add(i.wrapping_mul(stride)), ctx);
            i = i.wrapping_add(1);
        }
    }
    mem_store(mem, base.wrapping_add(20), 0);
    1
}

/// Cross-TU calls made by `fn_1fe094`.
pub trait TextResetHooks: ItemsDtorHooks {
    /// `fn_19611c`: resolve the state's handle to a context.
    fn resolve_context(&mut self, h0: u32, h1: u32, ctx: &mut u32) -> u32;
}

/// fn_1fe094 / EU fn_1fe250 / KR fn_1fe2fc.
///
/// Clear the text state's items once the handle resolves.
pub fn clear_text_items<H: TextResetHooks>(
    hooks: &mut H,
    mem: &mut [u8],
    base: u32,
    check: u32,
) -> u32 {
    let result = validate_handle(true, check, 0x11d, 0x48);
    if result >= 0x80000000 {
        return result;
    }
    let mut ctx = 0u32;
    let result = hooks.resolve_context(
        mem_word(mem, base),
        mem_word(mem, base.wrapping_add(4)),
        &mut ctx,
    );
    if result >= 0x80000000 {
        return result;
    }
    let items = mem_word(mem, base.wrapping_add(0x2c));
    destroy_items(hooks, mem, nz(items));
    result
}

/// Cross-TU calls made by `fn_230274` (the switch name is a
/// literal, so only the flag value crosses).
pub trait SwitchHooks {
    /// `fn_175944`: read the pipeline-switch flag into `val`.
    fn read_switch(&mut self, obj: u32, val: &mut u32) -> u32;
    /// `fn_196770`: close the pipes for `arg`.
    fn close_pipes(&mut self, arg: u32) -> u32;
}

/// fn_230274 / EU fn_230430 / KR fn_2304dc.
///
/// Report the pipeline-switch state: an executing switch
/// returns early, otherwise the pipes close and the slot is
/// marked done.
pub fn pipeline_switch<H: SwitchHooks>(
    hooks: &mut H,
    mem: &mut [u8],
    base: u32,
    check: u32,
) -> u32 {
    let mut val = 0u32;
    let mut r = validate_handle(true, check, 0x2be, 44);
    if r < 0x80000000 {
        let inner = mem_word(mem, base.wrapping_add(8));
        let obj = mem_word(mem, inner.wrapping_add(20));
        if obj != 0 {
            r = hooks.read_switch(obj, &mut val);
            if r < 0x80000000 {
                if val == 1 {
                    return r;
                }
            } else {
                val = 0;
            }
        }
        r = hooks.close_pipes(mem_word(mem, base.wrapping_add(0x14)));
        mem_store(mem, base.wrapping_add(0x10), 1);
    }
    r
}

/// Stream release behind `fn_1f4cb0`.
pub trait PromptReleaseHooks {
    /// Release one `(object, tag)` stream handle.
    fn release_stream(&mut self, h0: u32, h1: u32) -> u32;
}

/// fn_1f4cb0 / EU fn_1f4e6c / KR fn_1f4f18.
///
/// Release the prompt processor's input then output streams,
/// clearing each slot. Reports the last release status.
pub fn release_prompt<H: PromptReleaseHooks>(
    hooks: &mut H,
    mem: &mut [u8],
    base: u32,
    check: u32,
) -> u32 {
    let mut r = validate_handle(true, check, 0xf424, 0x40);
    if r >= 0x80000000 {
        return 0x8c802008;
    }
    for slot in [32u32, 40, 48, 56] {
        let at = base.wrapping_add(slot);
        if mem_word(mem, at) != 0 {
            r = hooks.release_stream(mem_word(mem, at), mem_word(mem, at.wrapping_add(4)));
            mem_store(mem, at, 0);
            mem_store(mem, at.wrapping_add(4), 0);
        }
    }
    r
}

/// Slot release behind `fn_211348`.
pub trait SlotReleaseHooks {
    /// Release one `(obj, arg)` slot.
    fn release_slot(&mut self, obj: u32, arg: u32) -> u32;
}

/// fn_211348 / EU fn_211504 / KR fn_2115b0.
///
/// Release the three resource slots in order, clearing each
/// one. Reports the last release status.
pub fn release_slots<H: SlotReleaseHooks>(hooks: &mut H, mem: &mut [u8], base: u32, b: u32) -> u32 {
    let mut r = validate_handle(true, b, 0xf381, 52);
    if r >= 0x80000000 {
        return 0x89a02008;
    }
    let mut i = 0u32;
    loop {
        let at = base.wrapping_add(24).wrapping_add(i.wrapping_mul(8));
        if mem_word(mem, at) != 0 {
            r = hooks.release_slot(mem_word(mem, at), mem_word(mem, at.wrapping_add(4)));
            mem_store(mem, at, 0);
            mem_store(mem, at.wrapping_add(4), 0);
        }
        i = (i.wrapping_add(1) & 0xffff) as u32;
        if i >= 3 {
            break;
        }
    }
    r
}

/// Stream close behind `fn_1f43f0`.
pub trait StreamCloseHooks {
    /// Close one `(h0, h1)` stream handle.
    fn close_stream(&mut self, h0: u32, h1: u32) -> u32;
    /// `fn_176fd4`: a fresh null pair.
    fn null_pair(&mut self) -> (u32, u32);
}

/// fn_1f43f0 / EU fn_1f45ac / KR fn_1f4658.
///
/// Close both stream handles, resetting each slot to a null
/// pair. Reports the last close status.
pub fn close_streams<H: StreamCloseHooks>(
    hooks: &mut H,
    mem: &mut [u8],
    base: u32,
    key: u32,
) -> u32 {
    let mut r = validate_handle(true, key, 0xf382, 0x34);
    if r >= 0x80000000 {
        return 0x8ba02008;
    }
    for k in 0..2u32 {
        let at = base.wrapping_add(36).wrapping_add(k.wrapping_mul(8));
        if mem_word(mem, at) != 0 {
            r = hooks.close_stream(mem_word(mem, at), mem_word(mem, at.wrapping_add(4)));
            let null = hooks.null_pair();
            mem_store(mem, at, null.0);
            mem_store(mem, at.wrapping_add(4), null.1);
        }
    }
    r
}

/// Vtable method call behind `fn_200e2c` / `fn_201264`.
pub trait VMethodHooks {
    /// Call the method with `(c, d)`.
    fn call_method(&mut self, c: u32, d: u32) -> u32;
}

/// fn_200e2c / EU fn_200fe8 / KR fn_201094.
///
/// Dispatch `(c, d)` through vtable method 7 once the handle
/// validates. The row plants the probe at slot +28.
pub fn call_method7<H: VMethodHooks>(hooks: &mut H, mem: &[u8], base: u32, q: u32) -> u32 {
    if validate_handle(true, q, 0xf383, 0x20) >= 0x80000000 {
        return 0x8bb02008;
    }
    hooks.call_method(
        mem_word(mem, base.wrapping_add(12)),
        mem_word(mem, base.wrapping_add(16)),
    )
}

/// fn_201264 / EU fn_201420 / KR fn_2014cc.
///
/// Dispatch `(c, d)` through vtable method 8 once the handle
/// validates. The row plants the probe at slot +32 (one past
/// the declared `m[8]`, which the engine sizes generously).
pub fn call_method8<H: VMethodHooks>(hooks: &mut H, mem: &[u8], base: u32, q: u32) -> u32 {
    if validate_handle(true, q, 0xf383, 0x20) >= 0x80000000 {
        return 0x8bb02008;
    }
    hooks.call_method(
        mem_word(mem, base.wrapping_add(12)),
        mem_word(mem, base.wrapping_add(16)),
    )
}

/// Word allocator behind `fn_1ce1bc`.
pub trait AllocRecHooks {
    /// Allocate `n` `size`-byte units from `heap`.
    fn alloc_word(&mut self, heap: u32, n: u32, size: u32) -> Option<u32>;
}

/// fn_1ce1bc / EU fn_1ce378 / KR fn_1ce424.
///
/// Allocate one back-reference cell holding the object address
/// and report it through `out`.
pub fn create_backref<H: AllocRecHooks>(
    hooks: &mut H,
    mem: &mut [u8],
    obj: Option<u32>,
    a: u32,
    out: Option<u32>,
) -> u32 {
    let rc = validate_handle(obj.is_some(), a, 0x73766, 0x198);
    if rc >= 0x80000000 {
        return 0x84b02000 | (rc & 0x1fff);
    }
    let Some(out) = out else {
        return 0x84b02006;
    };
    mem_store(mem, out, 0);
    let base = obj.unwrap();
    let heap = mem_word(mem, mem_word(mem, base).wrapping_add(4));
    let Some(rec) = hooks.alloc_word(heap, 1, 4) else {
        return 0x84b0200a;
    };
    mem_store(mem, rec, base);
    mem_store(mem, out, rec);
    rc
}

/// Context calls behind `fn_22e084`.
pub trait CtxResetHooks {
    /// Outer `release` of `(a, b)` with `flag`.
    fn release_pair(&mut self, a: u32, b: u32, flag: u32) -> u32;
    /// Inner `free` of `(a, b)`.
    fn free_pair(&mut self, a: u32, b: u32) -> u32;
    /// Outer `detach` of `(a, b)`.
    fn detach_pair(&mut self, a: u32, b: u32) -> u32;
    /// `fn_176fd4`: a fresh null pair.
    fn null_pair(&mut self) -> (u32, u32);
}

/// fn_22e084 / EU fn_22e240 / KR fn_22e2ec.
///
/// Tear down the context: release the trailing handle, clear the
/// counters, free the four live handles in order (first error
/// sticks), then detach the stale one.
pub fn reset_ctx<H: CtxResetHooks>(
    hooks: &mut H,
    mem: &mut [u8],
    obj: Option<u32>,
    tag: u32,
) -> u32 {
    let mut ret = validate_handle(obj.is_some(), tag, 0x34ea, 0x74);
    if ret >= 0x80000000 {
        return 0x8b902008;
    }
    let base = obj.unwrap();
    if mem_word(mem, base.wrapping_add(0x50)) != 0 {
        hooks.release_pair(
            mem_word(mem, base.wrapping_add(0x50)),
            mem_word(mem, base.wrapping_add(0x54)),
            1,
        );
        let null = hooks.null_pair();
        mem_store(mem, base.wrapping_add(0x50), null.0);
        mem_store(mem, base.wrapping_add(0x54), null.1);
    }
    mem_store(mem, base.wrapping_add(0x58), 0);
    mem_store(mem, base.wrapping_add(0x5c), 0);
    for (i, at) in [0x10u32, 0x18, 0x20, 0x28].iter().enumerate() {
        let at = base.wrapping_add(*at);
        if mem_word(mem, at) != 0 {
            let e = hooks.free_pair(mem_word(mem, at), mem_word(mem, at.wrapping_add(4)));
            if e >= 0x80000000 && (i == 0 || ret < 0x80000000) {
                ret = e;
            }
            let null = hooks.null_pair();
            mem_store(mem, at, null.0);
            mem_store(mem, at.wrapping_add(4), null.1);
        }
    }
    if mem_word(mem, base.wrapping_add(0x30)) != 0 {
        hooks.detach_pair(
            mem_word(mem, base.wrapping_add(0x34)),
            mem_word(mem, base.wrapping_add(0x38)),
        );
    }
    ret
}

/// Text-stream calls behind `fn_18fdc0`.
pub trait TextOpenHooks {
    /// `fn_19611c`: resolve the engine handle to an environment.
    fn resolve_env(&mut self, h0: u32, h1: u32, env: &mut u32) -> u32;
    /// `fn_175a1c`: report `(rc, mime-offset)` for the properties;
    /// the offset is the planted default on error.
    fn mime_type(&mut self, props: u32) -> (u32, u32);
    /// Planted prefix offset: the UTF-8 mark when `utf8`, the
    /// UTF-16 mark otherwise.
    fn prefix_off(&mut self, utf8: bool) -> u32;
    /// Interface `create`: open a stream for `(h0, h1)` into `out`.
    fn stream_create(&mut self, h0: u32, h1: u32, out: &mut (u32, u32)) -> u32;
    /// Interface `open`: bind `name` bytes to `out`.
    fn stream_open(
        &mut self,
        h0: u32,
        h1: u32,
        kind: u32,
        name: u32,
        x: u32,
        y: u32,
        out: &mut (u32, u32),
    ) -> u32;
    /// Interface `buffer`: expose `count` bytes at `*data`.
    fn stream_buffer(&mut self, h0: u32, h1: u32, data: &mut u32, count: u32) -> u32;
    /// Interface `commit`: publish `n` bytes.
    fn stream_commit(&mut self, h0: u32, h1: u32, n: u32) -> u32;
    /// Interface `close`.
    fn stream_close(&mut self, h0: u32, h1: u32) -> u32;
    /// Interface `destroy`.
    fn stream_destroy(&mut self, h0: u32, h1: u32) -> u32;
    /// `fn_176fd4`: a fresh null pair.
    fn null_pair(&mut self) -> (u32, u32);
}

/// fn_18fdc0 / EU fn_18fe70 / KR fn_18feac.
///
/// Open a text stream for the request: resolve the environment,
/// pick the prefix mark from the input MIME type, stage
/// prefix plus text into the stream buffer and commit. Reports
/// the first error, destroying a half-opened stream.
pub fn open_text<H: TextOpenHooks>(
    hooks: &mut H,
    mem: &mut [u8],
    base: Option<u32>,
    req: u32,
    out: Option<u32>,
) -> u32 {
    let mut stream = hooks.null_pair();
    if base.is_none() || out.is_none() {
        return 0x80000007;
    }
    let base = base.unwrap();
    let out = out.unwrap();
    let length = mem_word(mem, req.wrapping_add(4));
    let text = mem_word(mem, req.wrapping_add(8));
    let null = hooks.null_pair();
    mem_store(mem, out, null.0);
    mem_store(mem, out.wrapping_add(4), null.1);
    let mut env = 0u32;
    let mut rc = hooks.resolve_env(
        mem_word(mem, base.wrapping_add(8)),
        mem_word(mem, base.wrapping_add(12)),
        &mut env,
    );
    if rc >= 0x80000000 {
        return rc;
    }
    let null = hooks.null_pair();
    let mut opened = (0u32, 0u32);
    rc = hooks.stream_create(null.0, null.1, &mut opened);
    mem_store(mem, out, opened.0);
    mem_store(mem, out.wrapping_add(4), opened.1);
    if rc < 0x80000000 {
        let props = mem_word(mem, env.wrapping_add(0x14));
        let (mrc, mime) = hooks.mime_type(props);
        let utf8 = mrc < 0x80000000
            && compare_str(&mem[mime as usize..], b"text/plain;charset=utf-8\0") == 0;
        let (prefix, mut prefix_len) = if utf8 {
            (hooks.prefix_off(true), 10u32)
        } else {
            (hooks.prefix_off(false), 20u32)
        };
        let mut bound = stream;
        rc = hooks.stream_open(
            opened.0, opened.1, 0xc0, mime, 0, 2, &mut bound,
        );
        if mem[base.wrapping_add(0x35) as usize] != 2 {
            prefix_len = 0;
        }
        if rc < 0x80000000 {
            let total = length.wrapping_add(prefix_len);
            let mut buffer = 0u32;
            rc = hooks.stream_buffer(bound.0, bound.1, &mut buffer, total);
            if rc < 0x80000000 {
                let (lo, hi) = mem.split_at_mut(buffer as usize);
                if prefix_len != 0 {
                    copy_bytes(
                        &mut hi[..prefix_len as usize],
                        &lo[prefix as usize..prefix as usize + prefix_len as usize],
                        prefix_len,
                    );
                }
                copy_bytes(
                    &mut hi[prefix_len as usize..prefix_len as usize + length as usize],
                    &lo[text as usize..text as usize + length as usize],
                    length,
                );
                mem_store(mem, base.wrapping_add(0x14c), prefix_len);
                rc = hooks.stream_commit(bound.0, bound.1, total);
            }
        }
        stream = bound;
    }
    if stream.0 != 0 {
        hooks.stream_close(stream.0, stream.1);
    }
    let o0 = mem_word(mem, out);
    if rc >= 0x80000000 && o0 != 0 {
        hooks.stream_destroy(o0, mem_word(mem, out.wrapping_add(4)));
        let null = hooks.null_pair();
        mem_store(mem, out, null.0);
        mem_store(mem, out.wrapping_add(4), null.1);
    }
    rc
}

/// Language-database reset calls behind `fn_15bab0`.
pub trait LdResetHooks {
    /// `fn_1738e8`: reset-begin trace on the table logger.
    fn log_begin(&mut self, vt4: u32);
    /// `fn_173dc0`: reset-failure trace with `code`.
    fn log_error(&mut self, vt4: u32, code: u32);
    /// `fn_1738e8`: reset-end trace reporting `r`.
    fn log_end(&mut self, vt4: u32, r: u32);
    /// `fn_1963b8`: allocate the scratch buffer; 0 on failure.
    fn alloc_buf(&mut self, vt1: u32, n: u32) -> u32;
    /// `fn_180ea0`: reset the block pool at `at` (zero each
    /// block back to its base and clear the tally word).
    fn reset_pool(&mut self, mem: &mut [u8], at: u32);
    /// `fn_15b758`: loader reset into `x`.
    fn load_reset(&mut self, x: &mut u32) -> u32;
}

/// fn_15bab0 / EU fn_15bb74 / KR fn_15ba94.
///
/// Reset the language database: validate the handle, allocate the
/// scratch buffer on first use, reset the block pool and delegate
/// to the loader reset. Mode 2 skips straight to the tail; any
/// other non-1 mode is rejected.
pub fn ld_reset<H: LdResetHooks>(
    hooks: &mut H,
    mem: &mut [u8],
    base: Option<u32>,
    a2: u32,
) -> u32 {
    if a2 != 0xb2c2 || base.is_none() {
        return 0x89202008;
    }
    let base = base.unwrap();
    let vt = mem_word(mem, base);
    let vt4 = mem_word(mem, vt.wrapping_add(16));
    hooks.log_begin(vt4);
    let mut r = 0u32;
    let mode = mem_word(mem, base.wrapping_add(32));
    if mode == 1 {
        if mem_word(mem, base.wrapping_add(48)) == 0 {
            let vt1 = mem_word(mem, vt.wrapping_add(4));
            let buf = hooks.alloc_buf(vt1, mem_word(mem, base.wrapping_add(36)));
            mem_store(mem, base.wrapping_add(48), buf);
            if buf == 0 {
                hooks.log_error(vt4, 0x4e20);
                return 0x8920200a;
            }
        }
        hooks.reset_pool(mem, base.wrapping_add(4));
        let mut x = 0u32;
        r = hooks.load_reset(&mut x);
        if r >= 0x80000000 {
            return r;
        }
    } else if mode != 2 {
        hooks.log_error(vt4, 0x4e23);
        return 0x89203e00;
    }
    mem[base.wrapping_add(52) as usize] = 0;
    hooks.log_end(vt4, r);
    r
}

/// Dual-stream open calls behind `fn_1f3dc0`.
pub trait DualStreamHooks {
    /// Stream-name token for output `i` (the name bytes ride
    /// with the create call and are never dereferenced).
    fn name_off(&mut self, i: u32) -> u32;
    /// Interface `create`: open the named stream into `out`.
    fn stream_create(
        &mut self,
        h0: u32,
        h1: u32,
        name: u32,
        out: &mut (u32, u32),
    ) -> u32;
    /// Interface `release`.
    fn stream_release(&mut self, h0: u32, h1: u32);
    /// `fn_176fd4`: a fresh null pair.
    fn null_pair(&mut self) -> (u32, u32);
}

/// fn_1f3dc0 / EU fn_1f3f7c / KR fn_1f4028.
///
/// Open both processor output streams; a failed open releases
/// the earlier outputs and reports the failure.
pub fn open_dual<H: DualStreamHooks>(
    hooks: &mut H,
    mem: &mut [u8],
    base: Option<u32>,
    tag: u32,
    h0: u32,
    h1: u32,
) -> u32 {
    if tag != 0xf382 || base.is_none() {
        return 0x8ba02008;
    }
    let base = base.unwrap();
    let mut result = 0u32;
    let mut i = 0u32;
    while i < 2 {
        let mut opened = (0u32, 0u32);
        let name = hooks.name_off(i);
        result = hooks.stream_create(h0, h1, name, &mut opened);
        mem_store(mem, base.wrapping_add(36).wrapping_add(i * 8), opened.0);
        mem_store(
            mem,
            base.wrapping_add(40).wrapping_add(i * 8),
            opened.1,
        );
        if result >= 0x80000000 {
            break;
        }
        i += 1;
    }
    if result >= 0x80000000 {
        let mut j = 0u32;
        while j < i {
            let o = mem_word(mem, base.wrapping_add(36).wrapping_add(j * 8));
            if o != 0 {
                hooks.stream_release(o, mem_word(mem, base.wrapping_add(40).wrapping_add(j * 8)));
                let null = hooks.null_pair();
                mem_store(mem, base.wrapping_add(36).wrapping_add(j * 8), null.0);
                mem_store(mem, base.wrapping_add(40).wrapping_add(j * 8), null.1);
            }
            j += 1;
        }
    }
    result
}

/// Prompt-stream open calls behind `fn_1f4724`.
pub trait PromptStreamsHooks {
    /// Stream-name token for `table` (0 input, 1 output), entry `i`.
    fn name_off(&mut self, table: u32, i: u32) -> u32;
    /// Interface `create`: open the named stream into `out`.
    fn stream_create(
        &mut self,
        h0: u32,
        h1: u32,
        name: u32,
        x: u32,
        y: u32,
        out: &mut (u32, u32),
    ) -> u32;
    /// Interface `getMode`.
    fn query_mode(&mut self) -> u32;
    /// Interface `release`.
    fn stream_release(&mut self, h0: u32, h1: u32);
}

/// fn_1f4724 / EU fn_1f48e0 / KR fn_1f498c.
///
/// Open both prompt input streams then both output streams (the
/// second output carries the interface mode); a failed open
/// releases the earlier streams and reports the failure.
pub fn open_prompt<H: PromptStreamsHooks>(
    hooks: &mut H,
    mem: &mut [u8],
    base: Option<u32>,
    tag: u32,
    h0: u32,
    h1: u32,
) -> u32 {
    if tag != 0xf424 || base.is_none() {
        return 0x8c802008;
    }
    let base = base.unwrap();
    let mut result = 0u32;
    let mut input_count = 0u32;
    while input_count < 2 {
        let name = hooks.name_off(0, input_count);
        let mut opened = (0u32, 0u32);
        result = hooks.stream_create(h0, h1, name, 0, 0, &mut opened);
        mem_store(
            mem,
            base.wrapping_add(32).wrapping_add(input_count * 8),
            opened.0,
        );
        mem_store(
            mem,
            base.wrapping_add(36).wrapping_add(input_count * 8),
            opened.1,
        );
        if result >= 0x80000000 {
            break;
        }
        input_count += 1;
    }
    let mut output_count = 0u32;
    if result < 0x80000000 {
        while output_count < 2 {
            let name = hooks.name_off(1, output_count);
            let x = if output_count == 1 {
                hooks.query_mode()
            } else {
                0
            };
            let mut opened = (0u32, 0u32);
            result = hooks.stream_create(h0, h1, name, x, 1, &mut opened);
            mem_store(
                mem,
                base.wrapping_add(48).wrapping_add(output_count * 8),
                opened.0,
            );
            mem_store(
                mem,
                base.wrapping_add(52).wrapping_add(output_count * 8),
                opened.1,
            );
            if result >= 0x80000000 {
                break;
            }
            output_count += 1;
        }
    }
    if result >= 0x80000000 {
        let mut j = 0u32;
        while j < input_count {
            let o = mem_word(mem, base.wrapping_add(32).wrapping_add(j * 8));
            if o != 0 {
                hooks.stream_release(o, mem_word(mem, base.wrapping_add(36).wrapping_add(j * 8)));
                mem_store(mem, base.wrapping_add(32).wrapping_add(j * 8), 0);
                mem_store(mem, base.wrapping_add(36).wrapping_add(j * 8), 0);
            }
            j += 1;
        }
        j = 0;
        while j < output_count {
            let o = mem_word(mem, base.wrapping_add(48).wrapping_add(j * 8));
            if o != 0 {
                hooks.stream_release(o, mem_word(mem, base.wrapping_add(52).wrapping_add(j * 8)));
                mem_store(mem, base.wrapping_add(48).wrapping_add(j * 8), 0);
                mem_store(mem, base.wrapping_add(52).wrapping_add(j * 8), 0);
            }
            j += 1;
        }
    }
    result
}

/// Triple-stream open calls behind `fn_210398`.
pub trait TripleStreamsHooks {
    /// Stream-name token for output `i`.
    fn name_off(&mut self, i: u32) -> u32;
    /// Interface `create`: open the named stream into `out`.
    fn stream_create(
        &mut self,
        h0: u32,
        h1: u32,
        name: u32,
        x: u32,
        out: &mut (u32, u32),
    ) -> u32;
    /// Interface `get_value`.
    fn query_value(&mut self) -> u32;
    /// Interface `destroy`.
    fn stream_destroy(&mut self, h0: u32, h1: u32);
}

/// fn_210398 / EU fn_210554 / KR fn_210600.
///
/// Open the three resource streams (the middle one carries the
/// interface value); a failed open destroys the earlier
/// resources and reports the failure.
pub fn open_triple<H: TripleStreamsHooks>(
    hooks: &mut H,
    mem: &mut [u8],
    base: Option<u32>,
    tag: u32,
    h0: u32,
    h1: u32,
) -> u32 {
    if tag != 0xf381 || base.is_none() {
        return 0x89a02008;
    }
    let base = base.unwrap();
    let mut result = 0u32;
    let mut i = 0u32;
    while i < 3 && result < 0x80000000 {
        let name = hooks.name_off(i);
        let x = if i == 1 { hooks.query_value() } else { 0 };
        let mut opened = (0u32, 0u32);
        result = hooks.stream_create(h0, h1, name, x, &mut opened);
        mem_store(mem, base.wrapping_add(24).wrapping_add(i * 8), opened.0);
        mem_store(mem, base.wrapping_add(28).wrapping_add(i * 8), opened.1);
        if result >= 0x80000000 {
            break;
        }
        i += 1;
    }
    if result >= 0x80000000 {
        let mut j = 0u32;
        while j < i {
            let h = mem_word(mem, base.wrapping_add(24).wrapping_add(j * 8));
            if h != 0 {
                hooks.stream_destroy(h, mem_word(mem, base.wrapping_add(28).wrapping_add(j * 8)));
                mem_store(mem, base.wrapping_add(24).wrapping_add(j * 8), 0);
                mem_store(mem, base.wrapping_add(28).wrapping_add(j * 8), 0);
            }
            j += 1;
        }
    }
    result
}

/// Voice-query calls behind `fn_15eef0`.
pub trait VoiceQueryHooks {
    /// `fn_1738e8`: match-begin trace on the header logger.
    fn log_begin(&mut self, logger: u32);
    /// `fn_1738e8`: match-end trace reporting `rc`.
    fn log_end(&mut self, logger: u32, rc: u32);
    /// `fn_15db94`: match `vrec` against the header, updating
    /// the record in place.
    fn match_voice(&mut self, hdr: &[u8; 32], vrec: &mut [u8; 296], flags: u32) -> u32;
}

/// fn_15eef0 / EU fn_15efb4 / KR fn_15eed4.
///
/// Find the voice for the query: copy the header with the
/// caller's name/language overrides, match a scratch copy of
/// the record, fill "text"/"pcm" defaults for empty slots and
/// write the record back.
pub fn find_voice<H: VoiceQueryHooks>(
    hooks: &mut H,
    mem: &mut [u8],
    base: Option<u32>,
    a1: u32,
    name: Option<u32>,
    lang: u32,
    rec: Option<u32>,
    flags: u32,
) -> u32 {
    if a1 != 0x15c || base.is_none() {
        return 0x89502008;
    }
    let base = base.unwrap();
    if rec.is_none() {
        return 0x89502007;
    }
    let rec = rec.unwrap();
    let inner = mem_word(mem, base.wrapping_add(8));
    let mut hdr = [0u8; 32];
    hdr.copy_from_slice(&mem[inner as usize..inner as usize + 32]);
    if let Some(n) = name {
        hdr[12..16].copy_from_slice(&n.to_le_bytes());
    }
    if lang != 0 {
        hdr[20..24].copy_from_slice(&lang.to_le_bytes());
    }
    let mut vrec = [0u8; 296];
    vrec.copy_from_slice(&mem[rec as usize..rec as usize + 296]);
    let logger = u32::from_le_bytes(hdr[16..20].try_into().unwrap());
    hooks.log_begin(logger);
    let rc = hooks.match_voice(&hdr, &mut vrec, flags);
    if rc < 0x80000000 {
        if vrec[0xf4] == 0 {
            vrec[0xf4..0xf9].copy_from_slice(b"text\0");
        }
        if vrec[0x104] == 0 {
            vrec[0x104..0x108].copy_from_slice(b"pcm\0");
        }
        mem[rec as usize..rec as usize + 296].copy_from_slice(&vrec);
    }
    hooks.log_end(logger, rc);
    rc
}

/// Audio-stream open calls behind `fn_22d460`.
pub trait AudioOpenHooks {
    /// Interface `open`: open the named stream into `out`.
    fn stream_open(
        &mut self,
        h0: u32,
        h1: u32,
        name: &[u8],
        x: u32,
        y: u32,
        out: &mut (u32, u32),
    ) -> u32;
    /// Interface `markerMode`.
    fn query_marker_mode(&mut self) -> u32;
    /// Audio-open failure trace.
    fn audio_open_failed(&mut self, logger: u32);
    /// Content-open failure trace with `code`.
    fn content_open_failed(&mut self, logger: u32, code: u32);
}

/// fn_22d460 / EU fn_22d61c / KR fn_22d6c8.
///
/// Open the synthesiser audio streams: two L16 streams named
/// for the sample rate, then two marker streams. A failed
/// second or third open stops the chain; the first and last
/// failures only trace.
pub fn open_audio<H: AudioOpenHooks>(
    hooks: &mut H,
    mem: &mut [u8],
    base: Option<u32>,
    tag: u32,
    a: u32,
    b: u32,
) -> u32 {
    if tag != 0x34ea || base.is_none() {
        return 0x8b902008;
    }
    let base = base.unwrap();
    mem_store(mem, base.wrapping_add(68), 0);
    mem_store(mem, base.wrapping_add(72), 0);
    mem_store(mem, base.wrapping_add(76), 0);
    if mem_word(mem, base.wrapping_add(48)) == 0 {
        return 0;
    }
    let mut name = [0u8; 64];
    name[..15].copy_from_slice(b"audio/L16;rate=");
    let rate = mem_word(mem, base.wrapping_add(60));
    let _ = render_unsigned(rate, &mut name[15..], 10);
    let context = mem_word(mem, base.wrapping_add(8));
    let logger = mem_word(mem, context.wrapping_add(16));
    let mut h = (0u32, 0u32);
    let mut rc = hooks.stream_open(a, b, &name, 0, 0, &mut h);
    mem_store(mem, base.wrapping_add(16), h.0);
    mem_store(mem, base.wrapping_add(20), h.1);
    if rc >= 0x80000000 {
        hooks.audio_open_failed(logger);
    }
    rc = hooks.stream_open(a, b, &name, 0, 1, &mut h);
    mem_store(mem, base.wrapping_add(24), h.0);
    mem_store(mem, base.wrapping_add(28), h.1);
    if rc >= 0x80000000 {
        hooks.content_open_failed(logger, 0xa10);
        return rc;
    }
    const MARKERS: &[u8] = b"application/x-realspeak-markers-pp;version=4.0\0";
    rc = hooks.stream_open(a, b, MARKERS, 0, 0, &mut h);
    mem_store(mem, base.wrapping_add(32), h.0);
    mem_store(mem, base.wrapping_add(36), h.1);
    if rc >= 0x80000000 {
        hooks.content_open_failed(logger, 0xa0f);
        return rc;
    }
    let mode = hooks.query_marker_mode();
    rc = hooks.stream_open(a, b, MARKERS, mode, 1, &mut h);
    mem_store(mem, base.wrapping_add(40), h.0);
    mem_store(mem, base.wrapping_add(44), h.1);
    if rc >= 0x80000000 {
        hooks.content_open_failed(logger, 0xa10);
    }
    rc
}

/// Segment-score calls behind `fn_15b404`.
pub trait SegmentScoreHooks {
    /// `fn_15668c`: score the segment; the gate for the rest.
    #[allow(clippy::too_many_arguments)]
    fn score_segment(
        &mut self,
        a: u32,
        c: u32,
        d: u32,
        e: u32,
        f: u32,
        g: u32,
        i0: u32,
        i1: u32,
        o0: u32,
        o1: u32,
    ) -> u32;
    /// `fn_15429c`: refresh the object.
    fn refresh_object(&mut self) -> u32;
    /// Table method `m44`: resolve the handle count into `n`.
    fn resolve_count(&mut self, handle: u32, flag: u32, n: &mut u32);
    /// `fn_1561b8`: combine the outputs.
    fn combine_outputs(&mut self, o1: u32, o2: u32) -> u32;
}

/// fn_15b404 / EU fn_15b4c8 / KR fn_15b3e8.
///
/// Score the segment: gate on the segment scorer, refresh the
/// object, publish the tally word, scale the resolved handle
/// count by the sub-object divisors (taking the multiply-first
/// path below 0x418937) and report the combiner status when
/// the refresh succeeded.
pub fn score_segment<H: SegmentScoreHooks>(
    hooks: &mut H,
    mem: &mut [u8],
    base: u32,
    a: u32,
    c: u32,
    d: u32,
    e: u32,
    f: u32,
    g: u32,
    i0: u32,
    i1: u32,
    o0: u32,
    o1: u32,
    out1: u32,
    out2: u32,
) -> u32 {
    fn div(n: u32, d: u32) -> u32 {
        crate::output::unsigned_division(n, d)
            .map(|[q, _]| q)
            .unwrap_or(0)
    }
    let r = hooks.score_segment(a, c, d, e, f, g, i0, i1, o0, o1);
    if r < 0x80000000 {
        let mut r = hooks.refresh_object();
        let tall = mem_word(mem, base.wrapping_add(0xf0));
        mem_store(mem, out1, tall);
        let handle = mem_word(mem, base.wrapping_add(0xc0));
        if handle == 0 {
            mem_store(mem, out2, 0);
        } else {
            let mut n = 0u32;
            hooks.resolve_count(handle, mem_word(mem, base.wrapping_add(0xc4)), &mut n);
            let x = div(n.wrapping_shl(3), mem_word(mem, base.wrapping_add(0x8c)));
            mem_store(mem, out2, x);
            let x2 = if x < 0x418937 {
                div(x.wrapping_mul(1000), mem_word(mem, base.wrapping_add(0x88)))
            } else {
                div(x, mem_word(mem, base.wrapping_add(0x88))).wrapping_mul(1000)
            };
            mem_store(mem, out2, x2);
        }
        let x = hooks.combine_outputs(mem_word(mem, out1), mem_word(mem, out2));
        let mut y = 0u32;
        if x >= 0x80000000 {
            y = x;
        }
        if r < 0x80000000 && y >= 0x80000000 {
            r = y;
        }
        r
    } else {
        r
    }
}

/// TTSEG synthesise calls behind `fn_15961c`.
pub trait SynthesizeHooks {
    /// `fn_1517e8`: resolve the engine into `ctx`/`seg`.
    fn resolve_engine(&mut self, a: u32, b: u32, ctx: &mut u32, seg: &mut u32) -> u32;
    /// `fn_1738e8`: synthesise-begin trace.
    fn log_begin(&mut self, log: u32);
    /// `fn_173dc0`: synthesise-failure trace with `code`.
    fn log_error(&mut self, log: u32, code: u32);
    /// `fn_1738e8`: synthesise-end trace reporting `r`.
    fn log_end(&mut self, log: u32, r: u32);
    /// `fn_15ae7c`: open the synthesiser into `state`.
    fn begin_synth(&mut self, state: &mut u8) -> u32;
    /// `fn_15af34`: close the synthesiser.
    fn end_synth(&mut self, state: u8);
    /// `fn_19862c`: tear down the trace handle.
    fn teardown_handle(&mut self, h0: u32, h1: u32);
    /// `fn_1985fc`: reset the trace handle.
    fn reset_handle(&mut self, h0: u32, h1: u32);
    /// `fn_19860c`: release the trace handle.
    fn release_handle(&mut self, h0: u32, h1: u32);
    /// `fn_19863c`: report the synthesis output words.
    fn report_synthesis(&mut self, h0: u32, h1: u32, log: u32, c: u32, x: u32, y: u32);
    /// `fn_15b404`: score the segment into `x`/`y`.
    #[allow(clippy::too_many_arguments)]
    fn score(
        &mut self,
        a: u32,
        c: u32,
        d: u32,
        e: u32,
        f: u32,
        g: u32,
        i0: u32,
        i1: u32,
        o0: u32,
        o1: u32,
        x: &mut u32,
        y: &mut u32,
    ) -> u32;
}

/// fn_15961c / EU fn_1596e0 / KR fn_159600.
///
/// Synthesise the segment: resolve the engine, open the
/// synthesiser, score live traces through the segment scorer
/// and close up. Reports 0x81202011 for a missing trace.
///
/// Note: the ROM `fn_1517e8` writes only its second out-word,
/// so a successful resolve leaves `ctx` null and the trace
/// below faults; the row scripts both out-words to exercise
/// the orchestration the source describes.
pub fn synthesize_ttseg<H: SynthesizeHooks>(
    hooks: &mut H,
    mem: &mut [u8],
    a: u32,
    b: u32,
    i0: u32,
    i1: u32,
    o0: u32,
    o1: u32,
) -> u32 {
    let mut x = 0u32;
    let mut y = 0u32;
    let mut seg = 0u32;
    let mut ctx = 0u32;
    let mut state = 0u8;
    let mut r = hooks.resolve_engine(a, b, &mut ctx, &mut seg);
    if r >= 0x80000000 {
        return r;
    }
    let log = mem_word(mem, ctx.wrapping_add(0x10));
    hooks.log_begin(log);
    r = hooks.begin_synth(&mut state);
    if r >= 0x80000000 {
        return r;
    }
    let trace = mem_word(mem, seg.wrapping_add(0x7c));
    if trace == 0 {
        hooks.log_error(log, 0x3eab);
        r = 0x81202011;
    } else {
        let h0 = mem_word(mem, trace.wrapping_add(0x10));
        if h0 != 0 {
            let h1 = mem_word(mem, trace.wrapping_add(0x14));
            hooks.teardown_handle(h0, h1);
            hooks.reset_handle(h0, h1);
        }
        let sa = mem_word(mem, seg.wrapping_add(0x14));
        let count = mem_word(mem, seg.wrapping_add(0x28));
        let sb = mem_word(mem, seg.wrapping_add(0x1c));
        r = hooks.score(ctx, sa, count, 0, sb, count, i0, i1, o0, o1, &mut x, &mut y);
        let g0 = mem_word(mem, trace.wrapping_add(0x10));
        if g0 != 0 {
            let g1 = mem_word(mem, trace.wrapping_add(0x14));
            hooks.release_handle(g0, g1);
            hooks.report_synthesis(
                g0,
                g1,
                log,
                mem_word(mem, trace.wrapping_add(0x18)),
                x,
                y,
            );
        }
    }
    hooks.end_synth(state);
    hooks.log_end(log, r);
    r
}

/// Language-database open calls behind `fn_15bc18`.
pub trait DbOpenHooks {
    /// `fn_19611c`: resolve the environment into `env`.
    fn resolve_env(&mut self, p: Option<u32>, a: u32, env: &mut u32) -> u32;
    /// `fn_1738e8`: open-begin trace.
    fn log_begin(&mut self, log: u32);
    /// `fn_173dc0`: open-failure trace with `code`.
    fn log_error(&mut self, log: u32, code: u32);
    /// `fn_1738e8`: open-end trace reporting `r`.
    fn log_end(&mut self, log: u32, r: u32);
    /// `fn_1963b8`: allocate the object; 0 on failure.
    fn alloc_obj(&mut self) -> u32;
    /// `fn_175944`: query parameter `i` into `out`.
    fn query_param(&mut self, i: u32, out: &mut u32) -> u32;
    /// `fn_180d00`: initialise the record pool at `at`.
    fn init_pool(&mut self, mem: &mut [u8], at: u32, ctx: u32, initial: u32, grow: u32);
    /// `fn_15bab0`: reset the database.
    fn reset_db(&mut self, obj: u32) -> u32;
    /// `fn_15b5f0`: close the database.
    fn close_db(&mut self, h0: u32, h1: u32);
}

/// fn_15bc18 / EU fn_15bcdc / KR fn_15bbfc.
///
/// Open the language database: resolve the environment,
/// allocate the object, read the record/pool sizes with
/// built-in defaults, initialise the pool and reset. A
/// failed reset closes the handle and clears the outputs.
pub fn open_db<H: DbOpenHooks>(
    hooks: &mut H,
    mem: &mut [u8],
    _a1: u32,
    _a2: u32,
    p: Option<u32>,
    a: u32,
    out: u32,
) -> u32 {
    mem_store(mem, out, 0);
    mem_store(mem, out.wrapping_add(4), 0);
    let mut ctx = 0u32;
    let r = hooks.resolve_env(p, a, &mut ctx);
    if r >= 0x80000000 {
        return r;
    }
    let log = mem_word(mem, ctx.wrapping_add(0x10));
    hooks.log_begin(log);
    let obj = hooks.alloc_obj();
    if obj == 0 {
        hooks.log_error(log, 0x4e20);
        return 0x8920200a;
    }
    mem_store(mem, obj, ctx);
    mem_store(mem, obj.wrapping_add(32), 1);
    let mut f24 = 0u32;
    if hooks.query_param(0, &mut f24) >= 0x80000000 {
        f24 = 0x80;
    }
    mem_store(mem, obj.wrapping_add(36), f24);
    let mut f28 = 0u32;
    if hooks.query_param(1, &mut f28) >= 0x80000000 {
        f28 = 0x80;
    }
    mem_store(mem, obj.wrapping_add(40), f28);
    let mut f2c = 0u32;
    if hooks.query_param(2, &mut f2c) >= 0x80000000 {
        f2c = 0xffffffff;
    }
    mem_store(mem, obj.wrapping_add(44), f2c);
    let mut initial = 0u32;
    if hooks.query_param(3, &mut initial) >= 0x80000000 {
        initial = 0x2800;
    }
    let mut grow = 0u32;
    if hooks.query_param(4, &mut grow) >= 0x80000000 {
        grow = 0x1000;
    }
    hooks.init_pool(mem, obj.wrapping_add(4), ctx, initial, grow);
    mem_store(mem, out, obj);
    mem_store(mem, out.wrapping_add(4), 0xb2c2);
    let ret = hooks.reset_db(obj);
    if ret >= 0x80000000 {
        hooks.close_db(obj, 0xb2c2);
        mem_store(mem, out, 0);
        mem_store(mem, out.wrapping_add(4), 0);
    }
    hooks.log_end(log, ret);
    ret
}

/// Language-database clear calls behind `fn_15b518`.
pub trait DbClearHooks {
    /// `fn_1738e8`: clear-begin trace.
    fn log_begin(&mut self, log: u32);
    /// `fn_1738e8`: clear-end trace reporting `r`.
    fn log_end(&mut self, log: u32, r: u32);
    /// `fn_180ea0`: reset the block pool at `at` (zero each
    /// block back to its base and clear the tally word).
    fn reset_pool(&mut self, mem: &mut [u8], at: u32);
}

/// fn_15b518 / EU fn_15b5dc / KR fn_15b4fc.
///
/// Clear the language database: validate the handle, drop the
/// pending byte, reset the record pool for live objects and
/// mark the generation flag.
pub fn clear_db<H: DbClearHooks>(
    hooks: &mut H,
    mem: &mut [u8],
    base: Option<u32>,
    a: u32,
) -> u32 {
    if a != 0xb2c2 || base.is_none() {
        return 0x89202008;
    }
    let base = base.unwrap();
    let ctx = mem_word(mem, base);
    let log = mem_word(mem, ctx.wrapping_add(16));
    hooks.log_begin(log);
    mem[base.wrapping_add(52) as usize] = 0;
    if mem_word(mem, base.wrapping_add(48)) != 0 {
        hooks.reset_pool(mem, base.wrapping_add(4));
        mem_store(mem, base.wrapping_add(32), 1);
    }
    hooks.log_end(log, 0);
    0
}

/// Language-database close calls behind `fn_15b5f0`.
pub trait DbCloseHooks {
    /// `fn_1738e8`: close-begin trace.
    fn log_open_begin(&mut self, log: u32);
    /// `fn_1738e8`: close-end trace reporting `r`.
    fn log_open_end(&mut self, log: u32, r: u32);
    /// `fn_1738e8`: clear-begin trace.
    fn log_clear_begin(&mut self, log: u32);
    /// `fn_1738e8`: clear-end trace reporting `r`.
    fn log_clear_end(&mut self, log: u32, r: u32);
    /// `fn_180f8c`: free the block pool at `at`.
    fn free_pool(&mut self, mem: &mut [u8], at: u32);
    /// `fn_180ea0`: reset the block pool at `at`.
    fn reset_pool(&mut self, mem: &mut [u8], at: u32);
    /// `fn_196428`: release `obj` to `heap`.
    fn free_obj(&mut self, heap: u32, obj: u32);
}

/// fn_15b5f0 / EU fn_15b6b4 / KR fn_15b5d4.
///
/// Close the language database: validate the handle, free the
/// record pool, run the clear sequence and release the live
/// object and the object itself.
pub fn close_db<H: DbCloseHooks>(
    hooks: &mut H,
    mem: &mut [u8],
    object: Option<u32>,
    tag: u32,
) -> u32 {
    if tag != 0xb2c2 || object.is_none() {
        return 0x89202008;
    }
    let base = object.unwrap();
    let ctx = mem_word(mem, base);
    let log = mem_word(mem, ctx.wrapping_add(16));
    hooks.log_open_begin(log);
    hooks.free_pool(mem, base.wrapping_add(4));
    hooks.log_clear_begin(log);
    mem[base.wrapping_add(52) as usize] = 0;
    if mem_word(mem, base.wrapping_add(48)) != 0 {
        hooks.reset_pool(mem, base.wrapping_add(4));
        mem_store(mem, base.wrapping_add(32), 1);
    }
    hooks.log_clear_end(log, 0);
    if mem_word(mem, base.wrapping_add(48)) != 0 {
        hooks.free_obj(
            mem_word(mem, ctx.wrapping_add(4)),
            mem_word(mem, base.wrapping_add(48)),
        );
    }
    hooks.free_obj(mem_word(mem, ctx.wrapping_add(4)), base);
    hooks.log_open_end(log, 0);
    0
}

/// Output-device calls behind `fn_191948`.
pub trait SetOutDeviceHooks {
    /// `fn_1738e8`: set-device-begin trace.
    fn log_begin(&mut self, log: u32);
    /// `fn_19611c`: resolve the engine pair into `ctx`.
    fn resolve_ctx(&mut self, e0: u32, e1: u32, ctx: &mut u32) -> u32;
    /// `fn_18f24c`: lock `vauto_ttsSetOutDevice` into `locked`.
    fn set_state(&mut self, obj: u32, locked: &mut u8) -> u32;
    /// `fn_1959dc`: release the state.
    fn flag_state(&mut self, obj: u32, locked: u8);
    /// `fn_1738e8`: set-device-end trace.
    fn log_end(&mut self, log: u32);
}

/// fn_191948 / EU fn_1919f8 / KR fn_191a34.
///
/// Set the output device pair: validate the handle tag and
/// magic, resolve the engine, take the state lock and store
/// the device words. Missing handles report raw 0x80000008,
/// a missing device 0x80000006 and an empty device
/// 0x80000012; scripted failures classify through
/// `fn_160fec`.
pub fn set_out_device<H: SetOutDeviceHooks>(
    hooks: &mut H,
    mem: &mut [u8],
    handle: Option<u32>,
    tag: u32,
    device: Option<u32>,
) -> u32 {
    let Some(object) = handle else {
        return 0x80000008;
    };
    if tag != 0x684f424a || mem_word(mem, object) != 0x734f424a {
        return 0x80000008;
    }
    let manager = mem_word(mem, object.wrapping_add(0x10));
    hooks.log_begin(mem_word(mem, manager.wrapping_add(0x10)));
    let mut ctx = 0u32;
    let mut result = hooks.resolve_ctx(
        mem_word(mem, object.wrapping_add(8)),
        mem_word(mem, object.wrapping_add(0xc)),
        &mut ctx,
    );
    if result >= 0x80000000 {
        return classify_code(result);
    }
    let Some(dev) = device else {
        return 0x80000006;
    };
    let second = mem_word(mem, dev.wrapping_add(4));
    if second == 0 {
        return 0x80000012;
    }
    let mut locked = 0u8;
    result = hooks.set_state(object, &mut locked);
    if result >= 0x80000000 {
        return classify_code(result);
    }
    mem_store(mem, object.wrapping_add(0x170), second);
    mem_store(mem, object.wrapping_add(0x16c), mem_word(mem, dev));
    hooks.flag_state(object, locked);
    let manager = mem_word(mem, object.wrapping_add(0x10));
    hooks.log_end(mem_word(mem, manager.wrapping_add(0x10)));
    classify_code(result)
}

/// Resource-load calls behind `fn_194454`.
pub trait ResourceLoadHooks {
    /// `fn_176fd4`: reset a handle pair to the default.
    fn init_pair(&mut self, pair: &mut (u32, u32));
    /// `fn_1738e8`: load-begin trace.
    fn log_begin(&mut self, log: u32);
    /// `fn_18f24c`: lock `vauto_ttsResourceLoad` into `locked`.
    fn set_state(&mut self, obj: u32, locked: &mut u8) -> u32;
    /// `fn_190078`: copy the resource name into `name`.
    fn copy_name(&mut self, resource: Option<u32>, name: &mut [u8; 128]) -> u32;
    /// Resource `load` vtable call; writes `start` on success.
    #[allow(clippy::too_many_arguments)]
    fn load(
        &mut self,
        h0: u32,
        h1: u32,
        name: &[u8; 128],
        flags: u32,
        size: u32,
        g0: u32,
        g1: u32,
        start: &mut (u32, u32),
    ) -> u32;
    /// `fn_1959dc`: release the state.
    fn flag_state(&mut self, obj: u32, locked: u8);
    /// `fn_1738e8`: load-end trace.
    fn log_end(&mut self, log: u32);
}

/// fn_194454 / EU fn_194504 / KR fn_194540.
///
/// Load a named resource: validate the handle tag and magic,
/// take the state lock, render the UTF-16 resource name to
/// ASCII and call the resource `load` entry. A loaded handle
/// lands in `out`; failures clear it. Missing handles report
/// raw 0x80000008, missing names/handles 0x80000006 and an
/// empty size 0x80000007; later failures classify through
/// `fn_160fec`.
#[allow(clippy::too_many_arguments)]
pub fn load_resource<H: ResourceLoadHooks>(
    hooks: &mut H,
    mem: &mut [u8],
    object: Option<u32>,
    tag: u32,
    resource: Option<u32>,
    size: u32,
    flags: u32,
    out: Option<u32>,
) -> u32 {
    let mut start = (0u32, 0u32);
    hooks.init_pair(&mut start);
    let Some(obj) = object else {
        return 0x80000008;
    };
    if tag != 0x684f424a || mem_word(mem, obj) != 0x734f424a {
        return 0x80000008;
    }
    let manager = mem_word(mem, obj.wrapping_add(0x10));
    hooks.log_begin(mem_word(mem, manager.wrapping_add(0x10)));
    if resource.is_none() || flags == 0 {
        return 0x80000006;
    }
    let Some(out) = out else {
        return 0x80000006;
    };
    if size == 0 {
        return 0x80000007;
    }
    let mut locked = 0u8;
    let mut result = hooks.set_state(obj, &mut locked);
    if result >= 0x80000000 {
        return classify_code(result);
    }
    let mut name = [0u8; 128];
    result = hooks.copy_name(resource, &mut name);
    if result < 0x80000000 {
        let mut handle = (0u32, 0u32);
        hooks.init_pair(&mut handle);
        let h0 = mem_word(mem, obj.wrapping_add(0x2c));
        let h1 = mem_word(mem, obj.wrapping_add(0x30));
        result = hooks.load(
            h0,
            h1,
            &name,
            flags,
            size,
            handle.0,
            handle.1,
            &mut start,
        );
        if result < 0x80000000 {
            mem_store(mem, out, start.0);
            mem_store(mem, out.wrapping_add(4), start.1);
        }
    }
    if result >= 0x80000000 {
        mem_store(mem, out, 0);
        mem_store(mem, out.wrapping_add(4), 0);
    }
    hooks.flag_state(obj, locked);
    let manager = mem_word(mem, obj.wrapping_add(0x10));
    hooks.log_end(mem_word(mem, manager.wrapping_add(0x10)));
    classify_code(result)
}

/// Stop calls behind `fn_194790`.
pub trait StopSynthHooks {
    /// `fn_1738e8`: stop-begin trace.
    fn log_begin(&mut self, log: u32);
    /// `fn_176f2c`: acquire the lock.
    fn acquire(&mut self, lock: u32) -> u32;
    /// `fn_173dc0`: stop-failure trace with `code`/`status`.
    fn log_error(&mut self, log: u32, code: u32, status: u32);
    /// `fn_176f5c`: release the lock.
    fn release(&mut self, lock: u32) -> u32;
    /// `fn_195a44`: flag the voice.
    fn flag_voice(&mut self, obj: u32) -> u32;
    /// Methods-table stop call.
    fn call_stop(&mut self, a: u32, b: u32) -> u32;
    /// `fn_18f340`: stop the output state.
    fn stop_output(&mut self, obj: u32) -> u32;
    /// `fn_1738e8`: stop-end trace.
    fn log_end(&mut self, log: u32);
}

/// fn_194790 / EU fn_194840 / KR fn_19487c.
///
/// Stop synthesis: validate the handle tag and magic,
/// hold the lock across the type check, flag the voice,
/// run the methods-table stop call and stop the output
/// state. A stop status whose low 13 bits are 0x11
/// counts as success; a failed output stop replaces a
/// zero status. Missing handles report raw 0x80000008
/// and a wrong type raw 0x80000011; other failures
/// classify through `fn_160fec`.
pub fn stop_synth<H: StopSynthHooks>(
    hooks: &mut H,
    mem: &mut [u8],
    p: Option<u32>,
    tag: u32,
) -> u32 {
    let Some(q) = p else {
        return 0x80000008;
    };
    if tag != 0x684f424a || mem_word(mem, q) != 0x734f424a {
        return 0x80000008;
    }
    let f10 = mem_word(mem, q.wrapping_add(0x10));
    hooks.log_begin(mem_word(mem, f10.wrapping_add(16)));
    let lock = mem_word(mem, q.wrapping_add(0x24));
    let status = hooks.acquire(lock);
    let (status, ty) = if status >= 0x80000000 {
        hooks.log_error(mem_word(mem, f10.wrapping_add(16)), 0x4671, status);
        (status, 0u8)
    } else {
        let ty = mem[q.wrapping_add(0x28) as usize];
        (hooks.release(lock), ty)
    };
    if status >= 0x80000000 {
        return classify_code(status);
    }
    if ty != 3 && ty != 4 {
        return 0x80000011;
    }
    let mut r = hooks.flag_voice(q);
    if r >= 0x80000000 {
        return classify_code(r);
    }
    r = hooks.call_stop(
        mem_word(mem, q.wrapping_add(0x2c)),
        mem_word(mem, q.wrapping_add(0x30)),
    );
    if r & 0x1fff == 0x11 {
        r = 0;
    }
    let v = hooks.stop_output(q);
    if v >= 0x80000000 && r == 0 {
        r = v;
    }
    hooks.log_end(mem_word(mem, f10.wrapping_add(16)));
    classify_code(r)
}

/// Voice-stop calls behind `fn_190410`.
pub trait StopVoiceHooks {
    /// `fn_1900f8`: apply the mode change.
    fn apply_mode(&mut self, voice: u32) -> u32;
    /// Voice-table configure call.
    fn configure(&mut self, h0: u32, h1: u32, buf: u32, mode: u32) -> u32;
    /// `fn_176fd4`: read the timestamp pair.
    fn timestamp(&mut self) -> (u32, u32);
    /// `fn_195a44`: start or stop the voice with `flag`.
    fn start_voice(&mut self, voice: u32, flag: u32) -> u32;
    /// `fn_176f2c`: acquire the lock.
    fn acquire(&mut self, lock: u32) -> u32;
    /// `fn_176f5c`: release the lock.
    fn release_lock(&mut self, lock: u32) -> u32;
    /// `fn_173dc0`: lock-failure trace with `status`.
    fn log_lock_error(&mut self, log: u32, status: u32);
    /// Voice-table render call.
    fn render(
        &mut self,
        h0: u32,
        h1: u32,
        b0: u32,
        b1: u32,
        n0: u32,
        n1: u32,
    ) -> u32;
    /// Buffer-table release call.
    fn release_buffer(&mut self, b0: u32, b1: u32) -> u32;
}

/// Check the stopped flag behind `fn_190410`.
///
/// Acquires the lock, reports a lock failure through
/// the trace call, else reads `state == 6` under the
/// lock and releases it. Returns the flag and the
/// trailing status.
fn check_stopped<H: StopVoiceHooks>(
    hooks: &mut H,
    mem: &[u8],
    voice: u32,
) -> (bool, u32) {
    let qi = voice as usize;
    let lock = mem_word(mem, voice.wrapping_add(0x24));
    let log = mem_word(mem, mem_word(mem, voice.wrapping_add(0x10)).wrapping_add(0x10));
    let result = hooks.acquire(lock);
    if result >= 0x80000000 {
        hooks.log_lock_error(log, result);
        (false, result)
    } else {
        let stopped = mem[qi.wrapping_add(0x28)] == 6;
        (stopped, hooks.release_lock(lock))
    }
}

/// fn_190410 / EU fn_1904c0 / KR fn_1904fc.
///
/// Stop the voice: on a mode change apply the new
/// mode and reconfigure, else reconfigure only when
/// the flag pair is incomplete. Stamp the time, clear
/// the counters, start the voice and render the
/// buffer unless already stopped. Always release the
/// buffer, stop the started voice, reconfigure when
/// the flag pair is incomplete and re-check the
/// stopped flag; a stopped voice at the end reports
/// 0x80002807. A failed mode apply restores the old
/// mode.
pub fn stop_voice<H: StopVoiceHooks>(
    hooks: &mut H,
    mem: &mut [u8],
    voice: u32,
    mode: u32,
    b0: u32,
    b1: u32,
) -> u32 {
    let qi = voice as usize;
    let oldmode = mem[qi.wrapping_add(0x148)];
    let flags = mem[qi.wrapping_add(0x34)];
    let h0 = mem_word(mem, voice.wrapping_add(0x2c));
    let h1 = mem_word(mem, voice.wrapping_add(0x30));
    let buf = voice.wrapping_add(0xc8);
    let full_flags = flags & 8 != 0 && flags & 4 != 0;
    let mut result;
    let mut main = true;
    if u32::from(oldmode) != mode {
        mem[qi.wrapping_add(0x148)] = mode as u8;
        result = hooks.apply_mode(voice);
        if result >= 0x80000000 {
            mem[qi.wrapping_add(0x148)] = oldmode;
            main = false;
        } else {
            result = hooks.configure(h0, h1, buf, mode);
            if result >= 0x80000000 {
                main = false;
            }
        }
    } else if !full_flags {
        result = hooks.configure(h0, h1, buf, mode);
        if result >= 0x80000000 {
            main = false;
        }
    } else {
        result = 0;
    }
    let mut started = false;
    if main {
        let (t0, t1) = hooks.timestamp();
        mem_store(mem, voice.wrapping_add(0x154), t0);
        mem_store(mem, voice.wrapping_add(0x158), t1);
        mem_store(mem, voice.wrapping_add(0x15c), 0);
        mem_store(mem, voice.wrapping_add(0x160), 0);
        mem_store(mem, voice.wrapping_add(0x164), 0);
        mem_store(mem, voice.wrapping_add(0x168), 0);
        result = hooks.start_voice(voice, if mode == 2 { 0x10000 } else { 1 });
        if result < 0x80000000 {
            started = true;
            let (busy, status) = check_stopped(hooks, mem, voice);
            result = status;
            if !busy && result == 0 {
                let (n0, n1) = hooks.timestamp();
                result = hooks.render(h0, h1, b0, b1, n0, n1);
            }
        }
    }
    let rc = hooks.release_buffer(b0, b1);
    if result < 0x80000000 {
        result = rc;
    }
    if started {
        let rc = hooks.start_voice(voice, if mode == 2 { 0x20000 } else { 2 });
        if result < 0x80000000 {
            result = rc;
        }
    }
    if !full_flags {
        let rc = hooks.configure(h0, h1, 0, mode);
        if result < 0x80000000 {
            result = rc;
        } else {
            return result;
        }
    }
    if result < 0x80000000 {
        let (stopped, status) = check_stopped(hooks, mem, voice);
        result = status;
        if result < 0x80000000 && stopped {
            result = 0x80002807;
        }
    }
    result
}

/// Stream-close calls behind `fn_14ea44`.
pub trait CloseStreamHooks {
    /// `fn_176f94`: validate the config handle.
    fn check(&mut self, obj: u32, tag: u32) -> u32;
    /// `fn_19611c`: resolve the context pair into `ctx`.
    fn resolve_ctx(&mut self, obj: u32, tag: u32, ctx: &mut u32) -> u32;
    /// `fn_1738e8`: close-begin trace.
    fn log_begin(&mut self, log: u32);
    /// `fn_196794`: open the entry iterator into `iter`.
    fn open_iter(&mut self, streams: u32, iter: &mut u32) -> u32;
    /// `fn_196860`: read the next key/state pair.
    fn next_entry(&mut self, iter: u32, key: &mut u32, state: &mut u32) -> u32;
    /// `fn_173dc0`: unterminated-writer trace.
    fn log_writer(&mut self, log: u32, writer_id: u32, content_type: u32, version: u32);
    /// `fn_173dc0`: module-release trace.
    fn log_module(&mut self, log: u32, module_id: u32, content_type: u32, version: u32);
    /// `fn_196428`: free `obj` from `heap`.
    fn free_obj(&mut self, heap: u32, obj: u32);
    /// `fn_1970ac`: read the reader slot `index` into `reader`; 1 continues.
    fn next_reader(&mut self, readers: u32, index: u32, reader: &mut u32) -> u32;
    /// `fn_196830`: close the entry iterator.
    fn close_iter(&mut self, iter: u32);
    /// `fn_19665c`: close the stream map.
    fn close_streams(&mut self, streams: u32) -> u32;
    /// `fn_1738e8`: close-end trace with `rc`.
    fn log_end(&mut self, log: u32, rc: u32);
}

/// fn_14ea44 / EU fn_14eb08 / KR fn_14ea28.
///
/// Close the synth stream: resolve the context,
/// walk every map entry releasing its writer and
/// readers and clamping references above one down
/// to one, then close the map. A failed iterator
/// open still closes the map when the low 13 bits
/// of the status are 20; a closed map frees the
/// config. Handle and resolve failures return
/// their raw status.
pub fn close_synth_stream<H: CloseStreamHooks>(
    hooks: &mut H,
    mem: &mut [u8],
    p: u32,
    tag: u32,
) -> u32 {
    let mut rc = hooks.check(p, tag);
    if rc >= 0x80000000 {
        return rc;
    }
    let mut ctx = 0u32;
    rc = hooks.resolve_ctx(
        mem_word(mem, p),
        mem_word(mem, p.wrapping_add(4)),
        &mut ctx,
    );
    if rc >= 0x80000000 {
        return rc;
    }
    let log = mem_word(mem, ctx.wrapping_add(16));
    let heap = mem_word(mem, ctx.wrapping_add(4));
    hooks.log_begin(log);
    let streams = mem_word(mem, p.wrapping_add(12));
    let mut iter = 0u32;
    rc = hooks.open_iter(streams, &mut iter);
    if rc < 0x80000000 {
        let mut key = 0u32;
        let mut state = 0u32;
        while hooks.next_entry(iter, &mut key, &mut state) < 0x80000000 {
            let si = state as usize;
            let content_type = mem_word(mem, key);
            let version = mem_half(mem, key.wrapping_add(4)) as u32;
            let readers = mem_word(mem, state.wrapping_add(12));
            if mem[si.wrapping_add(68)] == 0 {
                hooks.log_writer(
                    log,
                    mem_half(mem, state.wrapping_add(24)) as u32,
                    content_type,
                    version,
                );
            }
            let writer = mem_word(mem, state.wrapping_add(8));
            if writer != 0 {
                hooks.log_module(
                    log,
                    mem_half(mem, writer.wrapping_add(2)) as u32,
                    content_type,
                    version,
                );
                hooks.free_obj(heap, writer);
                mem_store(mem, state.wrapping_add(8), 0);
            }
            let mut index = 0u32;
            let mut reader = 0u32;
            while hooks.next_reader(readers, index, &mut reader) == 1 {
                let stream = mem_word(mem, reader);
                hooks.log_module(
                    log,
                    mem_half(mem, stream.wrapping_add(2)) as u32,
                    content_type,
                    version,
                );
                hooks.free_obj(heap, stream);
                index = index.wrapping_add(1);
            }
            if mem_half(mem, state.wrapping_add(22)) > 1 {
                mem_store16(mem, state.wrapping_add(22), 1);
            }
        }
        hooks.close_iter(iter);
    }
    if rc & 0x1fff == 20 || rc < 0x80000000 {
        rc = hooks.close_streams(streams);
        if rc < 0x80000000 {
            hooks.free_obj(heap, p);
        }
    }
    hooks.log_end(log, rc);
    rc
}

/// User-prompt insertion calls behind `fn_1ff4fc`.
pub trait InsertPromptHooks {
    /// `fn_176f94`: validate the domain handle.
    fn check(&mut self, handle: u32, start: u32) -> u32;
    /// `fn_19611c`: resolve the key pair into `manager`.
    fn resolve(&mut self, a: u32, b: u32, manager: &mut u32) -> u32;
    /// `fn_196394`: allocate `size` bytes from `heap`; `None` on failure.
    fn alloc_text(&mut self, heap: u32, size: u32) -> Option<u32>;
    /// `fn_1fe418`: build the prompt for `text` into `prompt`.
    fn build_prompt(
        &mut self,
        handle: u32,
        text: u32,
        prompt: &mut [u32; 9],
    ) -> u32;
    /// `fn_1738e8`: insertion trace.
    fn log_insert(&mut self, log: u32);
    /// `fn_196428`: free `text` back to `heap`.
    fn free_text(&mut self, heap: u32, text: u32);
    /// `fn_2784c8`: rewrite the prompt at `pos` into the out words.
    #[allow(clippy::too_many_arguments)]
    fn rewrite_prompt(
        &mut self,
        prompt: &[u32; 9],
        pos: u32,
        start: u32,
        output: &mut u32,
        tokens: &mut u32,
        count: &mut u16,
        index: u32,
        args_a: u32,
        args_b: u32,
        c: u32,
    ) -> u32;
    /// `fn_1738e8`: rewrite trace.
    fn log_rewrite(&mut self, log: u32);
    /// `fn_173dc0`: allocation-failure trace.
    fn log_no_memory(&mut self, log: u32);
}

/// Recode a NUL-terminated u16 string at `src` into ASCII at `dst`.
///
/// `fn_176460`: units with a set high byte become `?`.
fn recode_176460(mem: &mut [u8], dst: u32, src: u32) {
    let mut s = src;
    let mut d = dst;
    loop {
        let unit = mem_half(mem, s);
        if unit == 0 {
            break;
        }
        mem[d as usize] = if unit & 0xff00 != 0 { b'?' } else { unit as u8 };
        s = s.wrapping_add(2);
        d = d.wrapping_add(1);
    }
    mem[d as usize] = 0;
}

/// fn_1ff4fc / EU fn_1ff6b8 / KR fn_1ff764.
///
/// Insert user prompts: for every type-40 token recode
/// its text, build the prompt and rewrite it into the
/// out words, which may move the token array and the
/// count under the loop. A failed allocation reports
/// 0x88e0200a; build and rewrite failures return their
/// raw status.
#[allow(clippy::too_many_arguments)]
pub fn insert_prompt<H: InsertPromptHooks>(
    hooks: &mut H,
    mem: &mut [u8],
    handle: u32,
    check: u32,
    output: u32,
    tokens: u32,
    count_ptr: u32,
    args_a: u32,
    args_b: u32,
    c: u32,
) -> u32 {
    let mut rc = hooks.check(handle, check);
    if rc >= 0x80000000 {
        return rc;
    }
    let mut manager = 0u32;
    rc = hooks.resolve(
        mem_word(mem, handle),
        mem_word(mem, handle.wrapping_add(4)),
        &mut manager,
    );
    if rc >= 0x80000000 {
        return rc;
    }
    let heap = mem_word(mem, manager.wrapping_add(4));
    let log = mem_word(mem, manager.wrapping_add(16));
    let mut tokp = mem_word(mem, tokens);
    let start = mem_word(mem, tokp.wrapping_add(12));
    let mut i = 0u32;
    loop {
        let count = mem_half(mem, count_ptr) as u32;
        if i >= count {
            break;
        }
        let tok = tokp.wrapping_add(i.wrapping_mul(24));
        if mem_half(mem, tok) == 40 {
            let text_ptr = mem_word(mem, tok.wrapping_add(20));
            let size = u16_len(mem, text_ptr).wrapping_add(1);
            let Some(text) = hooks.alloc_text(heap, size) else {
                hooks.log_no_memory(log);
                rc = 0x88e0200a;
                break;
            };
            recode_176460(mem, text, text_ptr);
            let mut prompt = [0u32; 9];
            rc = hooks.build_prompt(handle, text, &mut prompt);
            if rc >= 0x80000000 {
                hooks.free_text(heap, text);
                break;
            }
            hooks.log_insert(log);
            hooks.free_text(heap, text);
            let pos =
                mem_word(mem, tok.wrapping_add(12)).wrapping_sub(start) as u16 as u32;
            let mut out_w = mem_word(mem, output);
            let mut tok_w = mem_word(mem, tokens);
            let mut count_w = mem_half(mem, count_ptr);
            rc = hooks.rewrite_prompt(
                &prompt, pos, start, &mut out_w, &mut tok_w, &mut count_w, i,
                args_a, args_b, c,
            );
            mem_store(mem, output, out_w);
            mem_store(mem, tokens, tok_w);
            mem_store16(mem, count_ptr, count_w);
            if rc >= 0x80000000 {
                break;
            }
            hooks.log_rewrite(log);
            tokp = tok_w;
        }
        i = i.wrapping_add(1);
    }
    rc
}

/// Text-to-speech calls behind `fn_191e40`.
pub trait ProcessTextHooks {
    /// `fn_176fd4`: read the timestamp pair.
    fn timestamp(&mut self) -> (u32, u32);
    /// `fn_1738e8`: processing-begin trace.
    fn log_begin(&mut self, log: u32);
    /// `fn_18f24c`: lock the engine; returns the status and the lock byte.
    fn lock_engine(&mut self, engine: u32) -> (u32, u8);
    /// `fn_1959dc`: unlock the engine.
    fn unlock_engine(&mut self, engine: u32, locked: u8);
    /// `fn_18fdc0`: process the request into `state`.
    fn process_request(
        &mut self,
        engine: u32,
        request: u32,
        state: &mut (u32, u32),
    ) -> u32;
    /// `fn_190410`: stop the voice over `state`.
    fn stop_voice_call(&mut self, engine: u32, s0: u32, s1: u32) -> u32;
    /// `fn_1738e8`: processing-end trace.
    fn log_end(&mut self, log: u32);
}

/// fn_191e40 / EU fn_191ef0 / KR fn_191f2c.
///
/// Process text to speech: validate the engine
/// handle and lock it, validate the request,
/// process it unless the engine is not ready,
/// and stop the voice over the result. Engine
/// failures report raw 0x80000008, an unready
/// engine raw 0x80000880; lock, request and
/// processing failures classify through
/// `fn_160fec`. The engine unlocks whenever the
/// lock call set the lock byte.
pub fn process_text2speech<H: ProcessTextHooks>(
    hooks: &mut H,
    mem: &mut [u8],
    engine: Option<u32>,
    magic: u32,
    request: Option<u32>,
) -> u32 {
    let (mut s0, mut s1) = hooks.timestamp();
    if magic != 0x684f424a {
        return 0x80000008;
    }
    let Some(eng) = engine else {
        return 0x80000008;
    };
    if mem_word(mem, eng) != 0x734f424a {
        return 0x80000008;
    }
    let log = mem_word(mem, mem_word(mem, eng.wrapping_add(16)).wrapping_add(16));
    hooks.log_begin(log);
    let lock = hooks.lock_engine(eng);
    let locked = lock.1;
    if lock.0 >= 0x80000000 {
        return classify_code(lock.0);
    }
    let mut result = match request {
        None => 0x80002007,
        Some(r)
            if mem_word(mem, r.wrapping_add(8)) == 0
                || mem_word(mem, r.wrapping_add(4)) == 0 =>
        {
            0x80002007
        }
        Some(r) if mem[r as usize] != 0 => 0x80002300,
        _ => 0,
    };
    if result == 0 {
        if mem_word(mem, eng.wrapping_add(368)) == 0 {
            hooks.unlock_engine(eng, locked);
            return 0x80000880;
        }
        let req = request.unwrap();
        let mut state = (s0, s1);
        result = hooks.process_request(eng, req, &mut state);
        (s0, s1) = state;
        if result < 0x80000000 {
            result = hooks.stop_voice_call(eng, s0, s1);
        }
    }
    hooks.log_end(log);
    if locked != 0 {
        hooks.unlock_engine(eng, locked);
    }
    classify_code(result)
}

/// Prompt pre-processing calls behind `fn_1f48cc`.
pub trait PreprocessHooks {
    /// `fn_176f94`: validate the processor.
    fn check(&mut self, proc: u32, tag: u32) -> u32;
    /// Stream-table available call.
    fn available(&mut self, h0: u32, h1: u32, avail: &mut u32, ended: &mut u8) -> u32;
    /// Stream-table transfer call.
    fn transfer(
        &mut self,
        oh0: u32,
        oh1: u32,
        ih0: u32,
        ih1: u32,
        count: &mut u32,
    ) -> u32;
    /// Stream-table release call.
    fn release(&mut self, h0: u32, h1: u32) -> u32;
    /// Stream-table read call.
    fn read_stream(
        &mut self,
        h0: u32,
        h1: u32,
        ptr: &mut u32,
        len: &mut u32,
    ) -> u32;
    /// `fn_196394`: allocate `size` bytes from `heap`; `None` on failure.
    fn alloc_text(&mut self, heap: u32, size: u32) -> Option<u32>;
    /// `fn_1963b8`: allocate `size` bytes from `heap`; `None` on failure.
    fn alloc_events(&mut self, heap: u32, size: u32) -> Option<u32>;
    /// `fn_173dc0`: allocation-failure trace.
    fn log_no_memory(&mut self, log: u32);
    /// Position-table get call.
    fn get_position(&mut self, h0: u32, h1: u32, pos: &mut u32) -> u32;
    /// Transform-table transform call.
    fn transform(
        &mut self,
        h0: u32,
        h1: u32,
        text: &mut u32,
        events: &mut u32,
        count: &mut u32,
    ) -> u32;
    /// Stream-table write call.
    fn write_stream(&mut self, h0: u32, h1: u32, data: u32, len: u32) -> u32;
    /// Stream-table consume call.
    fn consume(&mut self, h0: u32, h1: u32, bytes: u32) -> u32;
    /// Position-table set call.
    fn set_position(&mut self, h0: u32, h1: u32, pos: u32) -> u32;
    /// `fn_196428`: free `ptr` back to `heap`.
    fn free_buf(&mut self, heap: u32, ptr: u32);
}

/// fn_1f48cc / EU fn_1f4a88 / KR fn_1f4b34.
///
/// Pre-process the prompt: without available
/// text forward pending events and release the
/// outputs at end, else read the text and
/// events, transform them, write both outputs,
/// consume both inputs and advance the
/// position by the text growth. Allocation
/// failures report 0x8c80200a; every other
/// failure returns its raw status.
pub fn preprocess_prompt<H: PreprocessHooks>(
    hooks: &mut H,
    mem: &mut [u8],
    proc: u32,
    tag: u32,
    _engine0: u32,
    _engine1: u32,
    active: u32,
) -> u32 {
    let mut result = hooks.check(proc, tag);
    if result >= 0x80000000 {
        return 0x8c802008;
    }
    mem[active as usize] = 1;
    let heap = mem_word(mem, mem_word(mem, proc).wrapping_add(4));
    let log = mem_word(mem, mem_word(mem, proc).wrapping_add(16));
    let (ih00, ih01) = (
        mem_word(mem, proc.wrapping_add(32)),
        mem_word(mem, proc.wrapping_add(36)),
    );
    let (ih10, ih11) = (
        mem_word(mem, proc.wrapping_add(40)),
        mem_word(mem, proc.wrapping_add(44)),
    );
    let mut available_text = 0u32;
    let mut ended = 0u8;
    result = hooks.available(ih00, ih01, &mut available_text, &mut ended);
    if result >= 0x80000000 {
        return result;
    }
    let mut available_events = 0u32;
    result = hooks.available(ih10, ih11, &mut available_events, &mut ended);
    if result >= 0x80000000 {
        return result;
    }
    if available_text == 0 {
        if available_events | ended as u32 == 0 {
            mem[active as usize] = 0;
        } else {
            if available_events != 0 {
                let mut event_count = 0u32;
                result = hooks.transfer(
                    mem_word(mem, proc.wrapping_add(56)),
                    mem_word(mem, proc.wrapping_add(60)),
                    ih10,
                    ih11,
                    &mut event_count,
                );
                if result >= 0x80000000 {
                    return result;
                }
            }
            if ended != 0 {
                for o in [48u32, 56u32] {
                    if mem_word(mem, proc.wrapping_add(o)) != 0 {
                        result = hooks.release(
                            mem_word(mem, proc.wrapping_add(o)),
                            mem_word(mem, proc.wrapping_add(o + 4)),
                        );
                        mem_store(mem, proc.wrapping_add(o), 0);
                        mem_store(mem, proc.wrapping_add(o + 4), 0);
                    }
                }
            }
        }
        return result;
    }
    let mut input_text = 0u32;
    let mut text_length = 0u32;
    result = hooks.read_stream(ih00, ih01, &mut input_text, &mut text_length);
    if result >= 0x80000000 {
        return result;
    }
    text_length >>= 1;
    let bytes = text_length.wrapping_mul(2);
    let Some(text) = hooks.alloc_text(heap, bytes.wrapping_add(2)) else {
        hooks.log_no_memory(log);
        return 0x8c80200a;
    };
    mem.copy_within(
        input_text as usize..(input_text.wrapping_add(bytes)) as usize,
        text as usize,
    );
    mem_store16(mem, text.wrapping_add(bytes), 0);
    let mut input_events = 0u32;
    let mut original_events = 0u32;
    result = hooks.read_stream(ih10, ih11, &mut input_events, &mut original_events);
    if result >= 0x80000000 {
        hooks.free_buf(heap, text);
        return result;
    }
    original_events /= 24;
    let event_bytes = original_events.wrapping_mul(24);
    let Some(events) = hooks.alloc_events(heap, event_bytes) else {
        hooks.log_no_memory(log);
        hooks.free_buf(heap, text);
        return 0x8c80200a;
    };
    mem.copy_within(
        input_events as usize..(input_events.wrapping_add(event_bytes)) as usize,
        events as usize,
    );
    let (ph0, ph1) = (
        mem_word(mem, proc.wrapping_add(20)),
        mem_word(mem, proc.wrapping_add(24)),
    );
    let mut position = 0u32;
    result = hooks.get_position(ph0, ph1, &mut position);
    if result >= 0x80000000 {
        hooks.free_buf(heap, text);
        hooks.free_buf(heap, events);
        return result;
    }
    let (th0, th1) = (
        mem_word(mem, proc.wrapping_add(12)),
        mem_word(mem, proc.wrapping_add(16)),
    );
    let mut ttext = text;
    let mut tevents = events;
    let mut event_count = original_events;
    result = hooks.transform(th0, th1, &mut ttext, &mut tevents, &mut event_count);
    if result >= 0x80000000 {
        hooks.free_buf(heap, text);
        hooks.free_buf(heap, events);
        return result;
    }
    let growth = u16_len(mem, ttext).wrapping_sub(text_length);
    let evpos = mem_word(mem, tevents.wrapping_add(16));
    mem_store(mem, tevents.wrapping_add(16), evpos.wrapping_add(growth.wrapping_mul(2)));
    if u16_len(mem, ttext) != 0 {
        result = hooks.write_stream(
            mem_word(mem, proc.wrapping_add(48)),
            mem_word(mem, proc.wrapping_add(52)),
            ttext,
            u16_len(mem, ttext).wrapping_mul(2),
        );
        if result >= 0x80000000 {
            hooks.free_buf(heap, text);
            hooks.free_buf(heap, events);
            return result;
        }
    }
    result = hooks.consume(ih00, ih01, bytes);
    if result >= 0x80000000 {
        hooks.free_buf(heap, text);
        hooks.free_buf(heap, events);
        return result;
    }
    result = hooks.write_stream(
        mem_word(mem, proc.wrapping_add(56)),
        mem_word(mem, proc.wrapping_add(60)),
        tevents,
        event_count.wrapping_mul(24),
    );
    if result >= 0x80000000 {
        hooks.free_buf(heap, text);
        hooks.free_buf(heap, events);
        return result;
    }
    result = hooks.consume(ih10, ih11, event_bytes);
    if result >= 0x80000000 {
        hooks.free_buf(heap, text);
        hooks.free_buf(heap, events);
        return result;
    }
    position = position.wrapping_add(growth.wrapping_mul(2));
    result = hooks.set_position(ph0, ph1, position);
    hooks.free_buf(heap, text);
    hooks.free_buf(heap, events);
    result
}

/// Prompt-table calls behind `fn_200e64`.
pub trait PromptOrthHooks {
    /// `fn_176f94`: validate the prompter.
    fn check(&mut self, p: u32, check: u32, f1: u32, f2: u32) -> u32;
    /// Data-table find call.
    fn find(&mut self, h0: u32, h1: u32, f1: u32, f2: u32, id: &mut u16) -> u32;
    /// Data-table type call.
    fn data_type(
        &mut self,
        h0: u32,
        h1: u32,
        id: u16,
        flag: u32,
        ty: &mut u16,
    ) -> u32;
    /// Data-table get call; `ptr` is an arena offset.
    fn get(
        &mut self,
        h0: u32,
        h1: u32,
        id: u16,
        flag: u32,
        ptr: &mut u32,
        len: &mut u16,
    ) -> u32;
    /// Data-table set call; `ptr` is an arena offset.
    #[allow(clippy::too_many_arguments)]
    fn set(
        &mut self,
        h0: u32,
        h1: u32,
        id: u16,
        flag: u32,
        count: u32,
        ptr: u32,
        written: &mut u16,
    ) -> u32;
    /// Transform-table process call; `None` is the null pointer.
    #[allow(clippy::too_many_arguments)]
    fn process(
        &mut self,
        t0: u32,
        t1: u32,
        orth: &mut Option<u32>,
        records: &mut Option<u32>,
        count: &mut u16,
        h0: u32,
        h1: u32,
        delta: &mut i16,
    ) -> u32;
    /// Position-table get call.
    fn get_position(&mut self, h0: u32, h1: u32, pos: &mut u32) -> u32;
    /// Position-table set call.
    fn set_position(&mut self, h0: u32, h1: u32, pos: u32) -> u32;
    /// `fn_196394`: allocate `size` bytes from `heap`; `None` on failure.
    fn alloc_orth(&mut self, heap: u32, size: u32) -> Option<u32>;
    /// `fn_1963b8`: allocate `size` bytes from `heap`; `None` on failure.
    fn alloc_records(&mut self, heap: u32, size: u32) -> Option<u32>;
    /// `fn_196428`: free `ptr` back to `heap`.
    fn free_buf(&mut self, heap: u32, ptr: u32);
    /// `fn_173dc0`: allocation-failure trace.
    fn log_no_memory(&mut self, log: u32, code: u32, flag: u32);
}

/// fn_200e64 / EU fn_201020 / KR fn_2010cc.
///
/// Resolve the prompt orthography and its
/// records, run the transform, store a
/// non-empty result back, shift the first
/// record and the stream position by the
/// returned delta and release both buffers.
/// A failing check reports 0x8bb02008 and an
/// allocation failure 0x8bb0200a; every other
/// failure returns its raw status.
pub fn prompt_orth<H: PromptOrthHooks>(
    hooks: &mut H,
    mem: &mut [u8],
    p: u32,
    check: u32,
    h0: u32,
    h1: u32,
    active: u32,
) -> u32 {
    let mut result = hooks.check(p, check, 0xf383, 0x20);
    if result >= 0x80000000 {
        return 0x8bb02008;
    }
    mem[active as usize] = 1;
    let ctx = mem_word(mem, p);
    let heap = mem_word(mem, ctx.wrapping_add(4));
    let log = mem_word(mem, ctx.wrapping_add(16));
    let mut id = 0u16;
    result = hooks.find(h0, h1, 1, 0, &mut id);
    if result >= 0x80000000 {
        return result;
    }
    let mut ty = 0u16;
    let mut source_orth = 0u32;
    let mut source_rec = 0u32;
    let mut length = 0u16;
    let mut have_source = false;
    if hooks.data_type(h0, h1, id, 0, &mut ty) < 0x80000000 && ty == 1 {
        result = hooks.get(h0, h1, id, 0, &mut source_orth, &mut length);
        if result >= 0x80000000 {
            return result;
        }
        if length > 1 {
            have_source = true;
        } else if hooks.data_type(h0, h1, id, 1, &mut ty) >= 0x80000000
            || ty != 1
        {
            return result;
        }
    } else if hooks.data_type(h0, h1, id, 1, &mut ty) >= 0x80000000
        || ty != 1
    {
        return result;
    }
    result = hooks.get(h0, h1, id, 1, &mut source_rec, &mut length);
    if result >= 0x80000000 {
        return result;
    }
    let mut count = length;
    // The empty-source path copies the empty orthography instead of
    // the fetched one; its record copy runs over the flag-1 source.
    let size = if have_source {
        strlen_bytes(&mem[source_orth as usize..]).wrapping_add(1)
    } else {
        1
    };
    let Some(buf) = hooks.alloc_orth(heap, size) else {
        hooks.log_no_memory(log, 0xafc8, 0);
        return 0x8bb0200a;
    };
    if have_source {
        copy_str(mem, buf, source_orth);
    } else {
        mem[buf as usize] = 0;
    }
    let mut orth = Some(buf);
    let bytes = (count as u32).wrapping_mul(24);
    let Some(rec) = hooks.alloc_records(heap, bytes) else {
        hooks.log_no_memory(log, 0xafc8, 0);
        hooks.free_buf(heap, buf);
        return 0x8bb0200a;
    };
    if bytes != 0 {
        mem.copy_within(
            source_rec as usize..(source_rec.wrapping_add(bytes)) as usize,
            rec as usize,
        );
    }
    let mut records = Some(rec);
    let mut delta = 0i16;
    result = hooks.process(
        mem_word(mem, p.wrapping_add(12)),
        mem_word(mem, p.wrapping_add(16)),
        &mut orth,
        &mut records,
        &mut count,
        h0,
        h1,
        &mut delta,
    );
    if result >= 0x80000000 {
        if let Some(buf) = orth {
            hooks.free_buf(heap, buf);
        }
        if let Some(rec) = records {
            hooks.free_buf(heap, rec);
        }
        return result;
    }
    let mut written = 0u16;
    if let Some(buf) = orth {
        if compare_str(&mem[buf as usize..], b"\0") != 0 {
            let n = strlen_bytes(&mem[buf as usize..]).wrapping_add(1) as u16 as u32;
            result = hooks.set(h0, h1, id, 0, n, buf, &mut written);
            if result >= 0x80000000 {
                hooks.free_buf(heap, buf);
                if let Some(rec) = records {
                    hooks.free_buf(heap, rec);
                }
                return result;
            }
        }
    }
    if let Some(rec) = records {
        if count != 0 {
            let pos = mem_word(mem, rec.wrapping_add(16));
            mem_store(mem, rec.wrapping_add(16), pos.wrapping_add(delta as u32));
            result = hooks.set(
                h0,
                h1,
                id,
                1,
                count as u32,
                rec,
                &mut written,
            );
            if result >= 0x80000000 {
                if let Some(buf) = orth {
                    hooks.free_buf(heap, buf);
                }
                hooks.free_buf(heap, rec);
                return result;
            }
        }
    }
    let mut position = 0u32;
    result = hooks.get_position(
        mem_word(mem, p.wrapping_add(24)),
        mem_word(mem, p.wrapping_add(28)),
        &mut position,
    );
    if result >= 0x80000000 {
        if let Some(buf) = orth {
            hooks.free_buf(heap, buf);
        }
        if let Some(rec) = records {
            hooks.free_buf(heap, rec);
        }
        return result;
    }
    position = position.wrapping_add(delta as u32);
    result = hooks.set_position(
        mem_word(mem, p.wrapping_add(24)),
        mem_word(mem, p.wrapping_add(28)),
        position,
    );
    if let Some(buf) = orth {
        hooks.free_buf(heap, buf);
    }
    if let Some(rec) = records {
        hooks.free_buf(heap, rec);
    }
    result
}

/// Token-table calls behind `fn_1f3ea0`.
pub trait PromptTokenHooks {
    /// `fn_176f94`: validate the prompter.
    fn check(&mut self, p: u32, check: u32, f1: u32, f2: u32) -> u32;
    /// Token-table first call.
    fn first(&mut self, h0: u32, h1: u32, a: u32, b: u32, out: &mut u16) -> u32;
    /// Token-table getCount call.
    fn get_count(
        &mut self,
        h0: u32,
        h1: u32,
        root: u16,
        flag: u32,
        attr: &mut u16,
    ) -> u32;
    /// Token-table getPointer call; `ptr` is an arena offset.
    fn get_pointer(
        &mut self,
        h0: u32,
        h1: u32,
        root: u16,
        flag: u32,
        ptr: &mut u32,
        len: &mut u16,
    ) -> u32;
    /// Prompt-table process call; `None` is the null pointer.
    #[allow(clippy::too_many_arguments)]
    fn process(
        &mut self,
        t0: u32,
        t1: u32,
        text: &mut Option<u32>,
        records: &mut Option<u32>,
        count: &mut u16,
        h0: u32,
        h1: u32,
        f0: u32,
        f1: u32,
        s0: u32,
        s1: u32,
        delta: &mut i16,
    ) -> u32;
    /// Token-table kind probe; shares the get script with `get_pause`.
    fn get_kind(
        &mut self,
        h0: u32,
        h1: u32,
        id: u16,
        f1: u32,
        f2: u32,
        kind: &mut u8,
        count: &mut u16,
    ) -> u32;
    /// Token-table pause probe; shares the get script with `get_kind`.
    fn get_pause(
        &mut self,
        h0: u32,
        h1: u32,
        id: u16,
        f1: u32,
        f2: u32,
        pause: &mut u16,
        count: &mut u16,
    ) -> u32;
    /// Token-table next call.
    fn next(&mut self, h0: u32, h1: u32, token: u16, out: &mut u16) -> u32;
    /// Token-table set call; `ptr` is an arena offset.
    #[allow(clippy::too_many_arguments)]
    fn set(
        &mut self,
        h0: u32,
        h1: u32,
        id: u16,
        flag: u32,
        count: u32,
        ptr: u32,
        written: &mut u16,
    ) -> u32;
    /// Token-table pause store; the C passes a stack slot, so the
    /// value crosses the hook instead of a pointer. Shares the set
    /// script with `set`.
    #[allow(clippy::too_many_arguments)]
    fn set_pause(
        &mut self,
        h0: u32,
        h1: u32,
        id: u16,
        flag: u32,
        count: u32,
        value: u16,
        written: &mut u16,
    ) -> u32;
    /// Position-table get call.
    fn get_position(&mut self, h0: u32, h1: u32, pos: &mut u32) -> u32;
    /// Position-table set call.
    fn set_position(&mut self, h0: u32, h1: u32, pos: u32) -> u32;
    /// `fn_196394`: allocate `size` bytes from `heap`; `None` on failure.
    fn alloc_text(&mut self, heap: u32, size: u32) -> Option<u32>;
    /// `fn_1963b8`: allocate `size` bytes from `heap`; `None` on failure.
    fn alloc_records(&mut self, heap: u32, size: u32) -> Option<u32>;
    /// `fn_196428`: free `ptr` back to `heap`.
    fn free_buf(&mut self, heap: u32, ptr: u32);
    /// `fn_173dc0`: allocation-failure trace.
    fn log_no_memory(&mut self, log: u32, code: u32, flag: u32);
    /// `fn_1738e8`: pause-store trace.
    fn log_pause(&mut self, log: u32, level: u32, flag: u32);
}

/// fn_1f3ea0 / EU fn_1f405c / KR fn_1f4108.
///
/// Resolve the prompt text and its records,
/// run the transform, walk the token chain
/// storing a pause of 5 across phrase-kind
/// boundaries, store a surviving result back,
/// shift the first record and the stream
/// position by the returned delta and release
/// both buffers. A failing check reports
/// 0x8ba02008 and an allocation failure
/// 0x8ba0200a; a rejected root returns the
/// root probe's status, and every other
/// failure returns its raw status.
pub fn prompt_token<H: PromptTokenHooks>(
    hooks: &mut H,
    mem: &mut [u8],
    p: u32,
    check: u32,
    h0: u32,
    h1: u32,
    applied: u32,
) -> u32 {
    let mut result = hooks.check(p, check, 0xf382, 0x34);
    if result >= 0x80000000 {
        return 0x8ba02008;
    }
    mem[applied as usize] = 1;
    let ctx = mem_word(mem, p);
    let heap = mem_word(mem, ctx.wrapping_add(4));
    let log = mem_word(mem, ctx.wrapping_add(16));
    let mut root = 0u16;
    result = hooks.first(h0, h1, 1, 0, &mut root);
    if result >= 0x80000000 {
        return result;
    }
    let mut attr = 0u16;
    let status = hooks.get_count(h0, h1, root, 0, &mut attr);
    if status >= 0x80000000 || attr != 1 {
        return result;
    }
    let mut source = 0u32;
    let mut length = 0u16;
    let status = hooks.get_pointer(h0, h1, root, 0, &mut source, &mut length);
    if status >= 0x80000000 || length <= 1 {
        return status;
    }
    let mut source_rec = 0u32;
    result = hooks.get_pointer(h0, h1, root, 1, &mut source_rec, &mut length);
    if result >= 0x80000000 {
        return result;
    }
    let mut record_count = length;
    let size = strlen_bytes(&mem[source as usize..]).wrapping_add(1);
    let Some(buf) = hooks.alloc_text(heap, size) else {
        hooks.log_no_memory(log, 0xb090, 0);
        return 0x8ba0200a;
    };
    copy_str(mem, buf, source);
    let mut text = Some(buf);
    let bytes = (record_count as u32).wrapping_mul(24);
    let Some(rec) = hooks.alloc_records(heap, bytes) else {
        hooks.log_no_memory(log, 0xb090, 0);
        hooks.free_buf(heap, buf);
        return 0x8ba0200a;
    };
    if bytes != 0 {
        mem.copy_within(
            source_rec as usize..(source_rec.wrapping_add(bytes)) as usize,
            rec as usize,
        );
    }
    let mut records = Some(rec);
    let mut delta = 0i16;
    result = hooks.process(
        mem_word(mem, p.wrapping_add(16)),
        mem_word(mem, p.wrapping_add(20)),
        &mut text,
        &mut records,
        &mut record_count,
        h0,
        h1,
        mem_word(mem, p.wrapping_add(36)),
        mem_word(mem, p.wrapping_add(40)),
        mem_word(mem, p.wrapping_add(44)),
        mem_word(mem, p.wrapping_add(48)),
        &mut delta,
    );
    if result >= 0x80000000 {
        if let Some(buf) = text {
            hooks.free_buf(heap, buf);
        }
        if let Some(rec) = records {
            hooks.free_buf(heap, rec);
        }
        return result;
    }
    let mut token = 0u16;
    result = hooks.first(h0, h1, 2, root as u32, &mut token);
    if result >= 0x80000000 {
        if let Some(buf) = text {
            hooks.free_buf(heap, buf);
        }
        if let Some(rec) = records {
            hooks.free_buf(heap, rec);
        }
        return result;
    }
    let mut previous = 0u16;
    let mut previous_kind = 4u8;
    let mut kind = 11u8;
    let mut count = 0u16;
    let mut written = 0u16;
    while token != 0 {
        result = hooks.get_kind(h0, h1, token, 0, 1, &mut kind, &mut count);
        if result >= 0x80000000 {
            if let Some(buf) = text {
                hooks.free_buf(heap, buf);
            }
            if let Some(rec) = records {
                hooks.free_buf(heap, rec);
            }
            return result;
        }
        if matches!(kind, 4 | 9 | 10 | 14 | 12 | 13 | 11) {
            if previous != 0 {
                let closing = matches!(previous_kind, 14 | 12)
                    && matches!(kind, 4 | 9 | 10 | 13);
                let opening = matches!(previous_kind, 4 | 9 | 10 | 13 | 11)
                    && kind == 14;
                if closing || opening {
                    let mut pause = 0u16;
                    result = hooks.get_pause(
                        h0, h1, previous, 8, 1, &mut pause, &mut count,
                    );
                    if result >= 0x80000000 {
                        if let Some(buf) = text {
                            hooks.free_buf(heap, buf);
                        }
                        if let Some(rec) = records {
                            hooks.free_buf(heap, rec);
                        }
                        return result;
                    }
                    if pause == 0 {
                        pause = 5;
                        result = hooks.set_pause(
                            h0, h1, previous, 8, 1, pause, &mut written,
                        );
                        if result >= 0x80000000 {
                            if let Some(buf) = text {
                                hooks.free_buf(heap, buf);
                            }
                            if let Some(rec) = records {
                                hooks.free_buf(heap, rec);
                            }
                            return result;
                        }
                        hooks.log_pause(log, 5, 0);
                    }
                }
            }
            previous = token;
            previous_kind = kind;
        }
        result = hooks.next(h0, h1, token, &mut token);
        if result >= 0x80000000 {
            if let Some(buf) = text {
                hooks.free_buf(heap, buf);
            }
            if let Some(rec) = records {
                hooks.free_buf(heap, rec);
            }
            return result;
        }
    }
    if let Some(buf) = text {
        let n = strlen_bytes(&mem[buf as usize..]).wrapping_add(1) as u16 as u32;
        result = hooks.set(h0, h1, root, 0, n, buf, &mut written);
        if result >= 0x80000000 {
            hooks.free_buf(heap, buf);
            if let Some(rec) = records {
                hooks.free_buf(heap, rec);
            }
            return result;
        }
    }
    if let Some(rec) = records {
        if record_count != 0 {
            let pos = mem_word(mem, rec.wrapping_add(16));
            mem_store(mem, rec.wrapping_add(16), pos.wrapping_add(delta as u32));
            result = hooks.set(
                h0,
                h1,
                root,
                1,
                record_count as u32,
                rec,
                &mut written,
            );
            if result >= 0x80000000 {
                if let Some(buf) = text {
                    hooks.free_buf(heap, buf);
                }
                hooks.free_buf(heap, rec);
                return result;
            }
        }
    }
    let mut position = 0u32;
    result = hooks.get_position(
        mem_word(mem, p.wrapping_add(28)),
        mem_word(mem, p.wrapping_add(32)),
        &mut position,
    );
    if result >= 0x80000000 {
        if let Some(buf) = text {
            hooks.free_buf(heap, buf);
        }
        if let Some(rec) = records {
            hooks.free_buf(heap, rec);
        }
        return result;
    }
    position = position.wrapping_add(delta as u32);
    result = hooks.set_position(
        mem_word(mem, p.wrapping_add(28)),
        mem_word(mem, p.wrapping_add(32)),
        position,
    );
    if let Some(buf) = text {
        hooks.free_buf(heap, buf);
    }
    if let Some(rec) = records {
        hooks.free_buf(heap, rec);
    }
    result
}

/// Ling-database open calls behind `fn_2100f0`.
pub trait LingOpenHooks {
    /// `fn_19611c`: resolve the context into `ctx`.
    fn resolve_ctx(&mut self, object: u32, tag: u32, ctx: &mut u32) -> u32;
    /// `fn_17b058`: acquire the next resource into `res`.
    fn acquire(&mut self, db: u32, res: &mut u32) -> u32;
    /// `fn_17b1c4`: release resource `which` (0 LINGDB, 1 SYNTHSTREAM, 2 FE_DEPES).
    fn release_name(&mut self, db: u32, which: u32);
    /// `fn_196394`: allocate the 52-byte object; `None` on failure.
    fn alloc_obj(&mut self, heap: u32) -> Option<u32>;
    /// `fn_173dc0`: open-failure trace with `code`.
    fn log_error(&mut self, log: u32, code: u32);
    /// `fn_175944`: query `usewordorthography` into `out`.
    fn query_ortho(&mut self, props: u32, out: &mut u32) -> u32;
}

/// fn_2100f0 / EU fn_2102ac / KR fn_210358.
///
/// Open the ling database: resolve the context, acquire
/// the LINGDB, SYNTHSTREAM and FE_DEPES resources,
/// allocate the object and wire the resource pointers.
/// A set `usewordorthography` property enables word
/// orthography. A missing `out` reports 0x89a02007 and a
/// failed allocation 0x89a0200a; resolve and acquire
/// failures return their raw status.
pub fn open_ling<H: LingOpenHooks>(
    hooks: &mut H,
    mem: &mut [u8],
    _a: u32,
    _b: u32,
    object: u32,
    tag: u32,
    out: Option<u32>,
) -> u32 {
    let Some(out) = out else {
        return 0x89a02007;
    };
    mem_store(mem, out, 0);
    mem_store(mem, out.wrapping_add(4), 0);
    let mut ctx = 0u32;
    let mut rc = hooks.resolve_ctx(object, tag, &mut ctx);
    if rc >= 0x80000000 {
        return rc;
    }
    let db = mem_word(mem, ctx.wrapping_add(0x18));
    let mut ling = 0u32;
    rc = hooks.acquire(db, &mut ling);
    if rc >= 0x80000000 {
        return rc;
    }
    let mut synth = 0u32;
    rc = hooks.acquire(db, &mut synth);
    if rc >= 0x80000000 {
        hooks.release_name(db, 0);
        return rc;
    }
    let mut deps = 0u32;
    rc = hooks.acquire(db, &mut deps);
    if rc >= 0x80000000 {
        hooks.release_name(db, 0);
        hooks.release_name(db, 1);
        return rc;
    }
    let heap = mem_word(mem, ctx.wrapping_add(4));
    let Some(o) = hooks.alloc_obj(heap) else {
        hooks.log_error(mem_word(mem, ctx.wrapping_add(0x10)), 0x7918);
        hooks.release_name(db, 0);
        hooks.release_name(db, 1);
        hooks.release_name(db, 2);
        return 0x89a0200a;
    };
    mem_store(mem, o, ctx);
    mem_store(mem, o.wrapping_add(4), mem_word(mem, ling.wrapping_add(4)));
    mem_store(mem, o.wrapping_add(8), mem_word(mem, synth.wrapping_add(4)));
    mem_store(mem, o.wrapping_add(0x14), mem_word(mem, deps.wrapping_add(4)));
    mem_store(mem, o.wrapping_add(0xc), mem_word(mem, deps.wrapping_add(8)));
    mem_store(mem, o.wrapping_add(0x10), mem_word(mem, deps.wrapping_add(0xc)));
    mem[o.wrapping_add(0x30) as usize] = 0;
    let mut orth = 0u32;
    let q = hooks.query_ortho(mem_word(mem, ctx.wrapping_add(0x14)), &mut orth);
    if q < 0x80000000 && orth == 1 {
        mem[o.wrapping_add(0x30) as usize] = 1;
    }
    mem_store(mem, out.wrapping_add(4), 0xf381);
    mem_store(mem, out, o);
    rc
}

/// Prompt open calls behind `fn_1f3ac8`.
pub trait PromptOpenHooks {
    /// `fn_19611c`: resolve the environment into `env`.
    fn resolve_env(&mut self, h0: u32, h1: u32, env: &mut u32) -> u32;
    /// `fn_17b058`: acquire the next resource into `res`.
    fn acquire(&mut self, res: u32, out: &mut u32) -> u32;
    /// `fn_17b1c4`: release resource `which` (0 LINGDB, 1 SYNTHSTREAM, 2 DOMAINMNGR, 3 FE_DEPES).
    fn release_name(&mut self, res: u32, which: u32);
    /// `fn_196394`: allocate the 0x34-byte object; `None` on failure.
    fn alloc_obj(&mut self, heap: u32) -> Option<u32>;
    /// `fn_173dc0`: open-failure trace with `code`.
    fn log_error(&mut self, log: u32, code: u32);
}

/// fn_1f3ac8 / EU fn_1f3c84 / KR fn_1f3d30.
///
/// Open the prompt processor: resolve the environment,
/// acquire the LINGDB, SYNTHSTREAM, DOMAINMNGR and
/// FE_DEPES resources, allocate the object and wire the
/// resource table and handle pairs. A missing `out`
/// reports 0x8ba02007 and a failed allocation
/// 0x8ba0200a; resolve and acquire failures return
/// their raw status.
pub fn open_prompt_proc<H: PromptOpenHooks>(
    hooks: &mut H,
    mem: &mut [u8],
    _a: u32,
    _b: u32,
    h0: u32,
    h1: u32,
    out: Option<u32>,
) -> u32 {
    let Some(out) = out else {
        return 0x8ba02007;
    };
    mem_store(mem, out, 0);
    mem_store(mem, out.wrapping_add(4), 0);
    let mut env = 0u32;
    let mut rc = hooks.resolve_env(h0, h1, &mut env);
    if rc >= 0x80000000 {
        return rc;
    }
    let res = mem_word(mem, env.wrapping_add(0x18));
    let mut ling = 0u32;
    rc = hooks.acquire(res, &mut ling);
    if rc >= 0x80000000 {
        return rc;
    }
    let mut synth = 0u32;
    rc = hooks.acquire(res, &mut synth);
    if rc >= 0x80000000 {
        hooks.release_name(res, 0);
        return rc;
    }
    let mut domain = 0u32;
    rc = hooks.acquire(res, &mut domain);
    if rc >= 0x80000000 {
        hooks.release_name(res, 0);
        hooks.release_name(res, 1);
        return rc;
    }
    let mut depes = 0u32;
    rc = hooks.acquire(res, &mut depes);
    if rc >= 0x80000000 {
        hooks.release_name(res, 0);
        hooks.release_name(res, 1);
        return rc;
    }
    let heap = mem_word(mem, env.wrapping_add(4));
    let Some(n) = hooks.alloc_obj(heap) else {
        hooks.log_error(mem_word(mem, env.wrapping_add(0x10)), 0xb090);
        hooks.release_name(res, 0);
        hooks.release_name(res, 1);
        hooks.release_name(res, 2);
        hooks.release_name(res, 3);
        return 0x8ba0200a;
    };
    mem_store(mem, n, env);
    mem_store(mem, n.wrapping_add(4), mem_word(mem, ling.wrapping_add(4)));
    mem_store(mem, n.wrapping_add(8), mem_word(mem, synth.wrapping_add(4)));
    mem_store(mem, n.wrapping_add(0xc), mem_word(mem, domain.wrapping_add(4)));
    mem_store(mem, n.wrapping_add(0x10), mem_word(mem, domain.wrapping_add(8)));
    mem_store(mem, n.wrapping_add(0x14), mem_word(mem, domain.wrapping_add(0xc)));
    mem_store(mem, n.wrapping_add(0x18), mem_word(mem, depes.wrapping_add(4)));
    mem_store(mem, n.wrapping_add(0x1c), mem_word(mem, depes.wrapping_add(8)));
    mem_store(mem, n.wrapping_add(0x20), mem_word(mem, depes.wrapping_add(0xc)));
    mem_store(mem, out, n);
    mem_store(mem, out.wrapping_add(4), 0xf382);
    rc
}

/// Parameter-set calls behind `fn_151df8`.
pub trait SetParamHooks {
    /// `fn_19611c`: resolve the handle into `ctx`.
    fn resolve_ctx(&mut self, h0: u32, h1: u32, ctx: &mut u32) -> u32;
    /// `fn_1738e8`: parameter-change trace.
    fn log_change(&mut self, log: u32);
    /// `fn_15af9c`: apply the escape-sequence parameter.
    fn apply_esc(&mut self, ctx: u32, p: u32, value: u32) -> u32;
}

/// fn_151df8 / EU fn_151ebc / KR fn_151ddc.
///
/// Set a synthesis parameter by name: resolve the
/// context, then store the parsed value into the loop
/// detector limit, marker mode or output buffer size,
/// or apply the escape-sequence parameter. Empty values
/// and failed resolves report 0.
pub fn set_synth_param<H: SetParamHooks>(
    hooks: &mut H,
    mem: &mut [u8],
    p: Option<u32>,
    name: Option<u32>,
    value: Option<u32>,
) -> u32 {
    let mut r = 0u32;
    let (Some(p), Some(v)) = (p, value) else {
        return r;
    };
    if mem[v as usize] == 0 {
        return r;
    }
    let mut ctx = 0u32;
    if hooks.resolve_ctx(mem_word(mem, p), mem_word(mem, p.wrapping_add(4)), &mut ctx)
        >= 0x80000000
    {
        return r;
    }
    hooks.log_change(mem_word(mem, ctx.wrapping_add(0x10)));
    let Some(name) = name else {
        return r;
    };
    if compare_str(&mem[name as usize..], b"infiniteloopdetectlimit\0") == 0 {
        mem_store(mem, p.wrapping_add(0x38), parse_uint(mem, Some(v)) as u32);
    } else if compare_str(&mem[name as usize..], b"markermode\0") == 0 {
        mem_store(mem, p.wrapping_add(0x34), parse_uint(mem, Some(v)) as u32);
    } else if compare_str(&mem[name as usize..], b"audiooutputbufsamples\0") == 0 {
        mem_store(mem, p.wrapping_add(0xe8), parse_uint(mem, Some(v)) as u32);
    } else if compare_str(&mem[name as usize..], b"__NUAN_alternative_esc_seq__\0") == 0 {
        r = hooks.apply_esc(ctx, p, v);
    }
    r
}

/// Domain-manager open calls behind `fn_200a2c`.
pub trait DomainOpenHooks {
    /// `fn_19611c`: resolve the handle into `env`.
    fn resolve_env(&mut self, h0: u32, h1: u32, env: &mut u32) -> u32;
    /// `fn_17b058`: acquire the next resource into `res`.
    fn acquire(&mut self, rm: u32, out: &mut u32) -> u32;
    /// `fn_17b1c4`: release resource `which` (0 LINGDB, 1 DOMAINMNGR, 2 FE_DEPES).
    fn release_name(&mut self, rm: u32, which: u32);
    /// `fn_196394`: allocate the 32-byte object; `None` on failure.
    fn alloc_obj(&mut self, heap: u32) -> Option<u32>;
    /// `fn_173dc0`: open-failure trace with `code`.
    fn log_error(&mut self, log: u32, code: u32);
}

/// fn_200a2c / EU fn_200be8 / KR fn_200c94.
///
/// Open the domain manager: resolve the environment,
/// acquire the LINGDB, DOMAINMNGR and FE_DEPES
/// resources, allocate the object and wire the table
/// and handle pairs. A missing `out` reports
/// 0x8bb02007 and a failed allocation 0x8bb0200a;
/// resolve and acquire failures return their raw
/// status.
pub fn open_domain<H: DomainOpenHooks>(
    hooks: &mut H,
    mem: &mut [u8],
    _a0: u32,
    _a1: u32,
    h0: u32,
    h1: u32,
    out: Option<u32>,
) -> u32 {
    let Some(out) = out else {
        return 0x8bb02007;
    };
    mem_store(mem, out, 0);
    mem_store(mem, out.wrapping_add(4), 0);
    let mut env = 0u32;
    let mut rc = hooks.resolve_env(h0, h1, &mut env);
    if rc >= 0x80000000 {
        return rc;
    }
    let rm = mem_word(mem, env.wrapping_add(0x18));
    let mut lk1 = 0u32;
    rc = hooks.acquire(rm, &mut lk1);
    if rc >= 0x80000000 {
        return rc;
    }
    let mut lk2 = 0u32;
    rc = hooks.acquire(rm, &mut lk2);
    if rc >= 0x80000000 {
        hooks.release_name(rm, 0);
        return rc;
    }
    let mut lk3 = 0u32;
    rc = hooks.acquire(rm, &mut lk3);
    if rc >= 0x80000000 {
        hooks.release_name(rm, 0);
        hooks.release_name(rm, 1);
        return rc;
    }
    let heap = mem_word(mem, env.wrapping_add(4));
    let Some(s) = hooks.alloc_obj(heap) else {
        hooks.log_error(mem_word(mem, env.wrapping_add(0x10)), 0xafc8);
        hooks.release_name(rm, 0);
        hooks.release_name(rm, 1);
        hooks.release_name(rm, 2);
        return 0x8bb0200a;
    };
    mem_store(mem, s, env);
    mem_store(mem, s.wrapping_add(4), mem_word(mem, lk1.wrapping_add(4)));
    mem_store(mem, s.wrapping_add(8), mem_word(mem, lk2.wrapping_add(4)));
    mem_store(mem, s.wrapping_add(0xc), mem_word(mem, lk2.wrapping_add(8)));
    mem_store(mem, s.wrapping_add(0x10), mem_word(mem, lk2.wrapping_add(0xc)));
    mem_store(mem, s.wrapping_add(0x14), mem_word(mem, lk3.wrapping_add(4)));
    mem_store(mem, s.wrapping_add(0x18), mem_word(mem, lk3.wrapping_add(8)));
    mem_store(mem, s.wrapping_add(0x1c), mem_word(mem, lk3.wrapping_add(0xc)));
    mem_store(mem, out, s);
    mem_store(mem, out.wrapping_add(4), 0xf383);
    rc
}

/// Stream open calls behind `fn_14e7a0`.
pub trait StreamOpenHooks {
    /// `fn_19611c`: resolve the handle into `ctx`.
    fn resolve_ctx(&mut self, h0: u32, h1: u32, ctx: &mut u32) -> u32;
    /// `fn_1738e8`: open-begin trace.
    fn log_begin(&mut self, log: u32);
    /// `fn_1963b8`: allocate the 32-byte config; `None` on failure.
    fn alloc_cfg(&mut self, heap: u32) -> Option<u32>;
    /// `fn_173dc0`: open-failure trace with `code`.
    fn log_error(&mut self, log: u32, code: u32);
    /// `fn_175944`: query tuning value `which` (0 allocation, 1 move, 2 free).
    fn query_u32(&mut self, props: u32, which: u32, out: &mut u32) -> u32;
    /// `fn_175850`: query the log-input flag.
    fn query_log_input(&mut self, props: u32, out: &mut u32);
    /// `fn_1964e0`: create the stream map into `streams`.
    fn create_map(&mut self, heap: u32, streams: &mut u32) -> u32;
    /// `fn_196428`: release the config.
    fn free_cfg(&mut self, heap: u32, cfg: u32);
    /// `fn_1738e8`: open-end trace reporting `rc`.
    fn log_end(&mut self, log: u32, rc: u32);
}

/// fn_14e7a0 / EU fn_14e864 / KR fn_14e784.
///
/// Open the synth stream: resolve the context,
/// allocate the config, read the allocation tuning
/// values with built-in defaults and create the
/// stream map. A missing `out` reports 0x88f02006
/// and a failed allocation 0x88f0200a; resolve and
/// map failures return their raw status.
pub fn open_synth_stream<H: StreamOpenHooks>(
    hooks: &mut H,
    mem: &mut [u8],
    _a: u32,
    _b: u32,
    h0: u32,
    h1: u32,
    out: Option<u32>,
) -> u32 {
    let Some(out) = out else {
        return 0x88f02006;
    };
    mem_store(mem, out, 0);
    mem_store(mem, out.wrapping_add(4), 0);
    let mut ctx = 0u32;
    let mut rc = hooks.resolve_ctx(h0, h1, &mut ctx);
    if rc >= 0x80000000 {
        return rc;
    }
    let log = mem_word(mem, ctx.wrapping_add(0x10));
    hooks.log_begin(log);
    let heap = mem_word(mem, ctx.wrapping_add(4));
    let Some(cfg) = hooks.alloc_cfg(heap) else {
        hooks.log_error(log, 0x55f0);
        rc = 0x88f0200a;
        hooks.log_end(log, rc);
        return rc;
    };
    mem_store(mem, cfg, h0);
    mem_store(mem, cfg.wrapping_add(4), h1);
    let props = mem_word(mem, ctx.wrapping_add(0x14));
    let mut v = 0u32;
    let q = hooks.query_u32(props, 0, &mut v);
    mem_store(
        mem,
        cfg.wrapping_add(0x10),
        if q >= 0x80000000 || v == 0 { 1024 } else { v },
    );
    let q = hooks.query_u32(props, 1, &mut v);
    mem_store(
        mem,
        cfg.wrapping_add(0x14),
        if q >= 0x80000000 || v == 0 { 128 } else { v },
    );
    let q = hooks.query_u32(props, 2, &mut v);
    mem_store(
        mem,
        cfg.wrapping_add(0x18),
        if q >= 0x80000000 || v == 0 { 4097 } else { v },
    );
    let mut li = 0u32;
    hooks.query_log_input(props, &mut li);
    mem_store(mem, cfg.wrapping_add(0x1c), li);
    let mut streams = 0u32;
    rc = hooks.create_map(heap, &mut streams);
    mem_store(mem, cfg.wrapping_add(0xc), streams);
    if rc < 0x80000000 {
        mem_store(mem, out.wrapping_add(4), 0x47dd);
        mem_store(mem, out, cfg);
    } else {
        hooks.free_cfg(heap, cfg);
    }
    hooks.log_end(log, rc);
    rc
}

/// Language-list calls behind `fn_15f594`.
pub trait LangListHooks {
    /// `fn_176f94`: validate the handle.
    fn check(&mut self, o: u32, a1: u32) -> u32;
    /// `fn_1738e8`: list-begin trace.
    fn log_begin(&mut self, log: u32);
    /// `fn_173dc0`: list-failure trace with `code`.
    fn log_error(&mut self, log: u32, code: u32);
    /// `fn_177b7c`: open the broker object into `comp`.
    fn open_broker(&mut self, name: u32, db: u32, comp: &mut u32) -> u32;
    /// `fn_1789b0`: open the component iterator into `iter`.
    fn open_comp_iter(&mut self, comp: u32, iter: &mut u32) -> u32;
    /// `fn_178c24`: fetch the next component key into `k`.
    fn next_comp(&mut self, iter: u32, k: &mut u32) -> u32;
    /// `fn_17a3b8`: open the tag iterator into `iter`.
    fn open_tag_iter(&mut self, db: u32, k: u32, iter: &mut u32) -> u32;
    /// `fn_17a814`: fetch the next tag pair.
    fn next_tag(
        &mut self,
        iter: u32,
        tag: &mut Option<u32>,
        value: &mut Option<u32>,
    ) -> u32;
    /// `fn_17a2ec`: close the tag iterator.
    fn close_tag(&mut self, iter: u32);
    /// `fn_1788c0`: close the component iterator.
    fn close_comp_iter(&mut self, iter: u32);
    /// `fn_177cc8`: close the broker object.
    fn close_comp(&mut self, comp: u32);
    /// `fn_1738e8`: list-end trace reporting `rc`/`count`.
    fn log_end(&mut self, log: u32, rc: u32, count: u32);
}

/// fn_15f594 / EU fn_15f658 / KR fn_15f578.
///
/// List the available languages: walk the broker's
/// components, collect each one's language tags into
/// an entry and store deduplicated entries into
/// `user` up to `*maxPtr`, which then reports the
/// count. String copies use the proven byte copy
/// with the measured length; dedup folds through
/// `fold`. A bad handle reports 0x89502008 and
/// missing inputs 0x89502007; a full list reports 9.
pub fn get_lang_list<H: LangListHooks>(
    hooks: &mut H,
    mem: &mut [u8],
    fold: &[u16; 256],
    o: Option<u32>,
    a1: u32,
    name: Option<u32>,
    user: Option<u32>,
    max_ptr: Option<u32>,
) -> u32 {
    let mut rc = hooks.check(o.unwrap_or(0), a1);
    if rc >= 0x80000000 {
        return 0x89502008;
    }
    let Some(p) = o else {
        return 0x89502008;
    };
    let inn = mem_word(mem, p.wrapping_add(8));
    let log = mem_word(mem, inn.wrapping_add(16));
    hooks.log_begin(log);
    let (Some(name), Some(max)) = (name, max_ptr) else {
        hooks.log_error(log, 0x59df);
        return 0x89502007;
    };
    let user = if mem_word(mem, max) == 0 {
        None
    } else {
        user
    };
    let mut comp = 0u32;
    rc = hooks.open_broker(name, mem_word(mem, p.wrapping_add(0xc)), &mut comp);
    if rc >= 0x80000000 {
        hooks.log_error(log, 0x59e0);
        return rc;
    }
    let mut comp_iter = 0u32;
    rc = hooks.open_comp_iter(comp, &mut comp_iter);
    if rc >= 0x80000000 {
        hooks.close_comp(comp);
        hooks.log_error(log, 0x59e1);
        return rc;
    }
    let mut count = 0u32;
    loop {
        let mut k = 0u32;
        if hooks.next_comp(comp_iter, &mut k) >= 0x80000000 {
            break;
        }
        let mut entry = [0u8; 86];
        fill_bytes(&mut entry, 0, 0, 86);
        let mut tag_iter = 0u32;
        if hooks.open_tag_iter(mem_word(mem, inn.wrapping_add(12)), k, &mut tag_iter)
            < 0x80000000
        {
            loop {
                let mut tag = None;
                let mut value = None;
                if hooks.next_tag(tag_iter, &mut tag, &mut value) >= 0x80000000 {
                    hooks.close_tag(tag_iter);
                    break;
                }
                let Some(t) = tag else {
                    continue;
                };
                // The C only tests the tag; a null value with a live
                // tag faults in `fn_195cd0`, which the engine never
                // produces. Read offset 0 there instead of faulting.
                let v = value.unwrap_or(0);
                if compare_str(&mem[t as usize..], b"language\0") == 0 {
                    let n = cstrlen(&mem[v as usize..]) + 1;
                    copy_bytes(&mut entry[..], &mem[v as usize..], n);
                } else if compare_str(&mem[t as usize..], b"langcode\0") == 0 {
                    let n = cstrlen(&mem[v as usize..]) + 1;
                    copy_bytes(&mut entry[64..], &mem[v as usize..], n);
                } else if compare_str(&mem[t as usize..], b"langversion\0") == 0 {
                    let n = cstrlen(&mem[v as usize..]) + 1;
                    copy_bytes(&mut entry[72..], &mem[v as usize..], n);
                } else if compare_str(&mem[t as usize..], b"langid\0") == 0 {
                    mem_store16(&mut entry, 84, parse_uint(mem, value) as u16);
                }
            }
        }
        if let Some(u) = user {
            let mut dup = false;
            let mut i = 0u32;
            while i < count {
                if compare_folded(fold, &entry, &mem[(u + i * 86) as usize..]) == 0 {
                    dup = true;
                    break;
                }
                i += 1;
            }
            if !dup {
                let maxv = mem_word(mem, max);
                if maxv != 0 {
                    if maxv <= count {
                        rc = 9;
                    } else {
                        copy_bytes(&mut mem[(u + count * 86) as usize..], &entry, 86);
                    }
                }
            }
            if dup {
                continue;
            }
        }
        count += 1;
    }
    mem_store(mem, max, count);
    hooks.close_comp_iter(comp_iter);
    hooks.close_comp(comp);
    hooks.log_end(log, rc, count);
    rc
}

/// Caller-owned calls used by the domain manager API.
pub trait DomainApi {
    fn resolve(&mut self, first: u32, second: u32, context: &mut u32) -> u32;
    fn find(&mut self, map: u32, name: u32, item: &mut u32) -> u32;
    fn append(&mut self, list: u32, item: u32) -> u32;
    fn pop(&mut self, list: u32);
    fn remove(&mut self, list: u32, index: u32);
    fn duplicate(&mut self, heap: u32, text: u32, copy: &mut u32) -> u32;
    fn free(&mut self, heap: u32, text: u32);
    fn slot(&mut self, list: u32, index: u32, item: &mut u32) -> u32;
    fn build(&mut self, mem: &mut [u8], database: u32, args: u32, output: u32) -> u32;
    /// Diagnostic-only fn_1738e8 and fn_173dc0 boundaries.
    fn trace(&mut self, log: u32, code: u32, name: u32, status: u32);
}

fn domain_context(mem: &[u8], handle: Option<u32>, tag: u32, api: &mut impl DomainApi)
    -> Result<(u32, u32), u32>
{
    let rc = validate_handle(handle.is_some(), tag, 0x11d, 0x48);
    if rc >= 0x80000000 { return Err(rc); }
    let handle = handle.unwrap();
    let mut context = 0;
    let rc = api.resolve(mem_word(mem, handle), mem_word(mem, handle.wrapping_add(4)), &mut context);
    if rc >= 0x80000000 { Err(rc) } else { Ok((context, rc)) }
}

/// fn_1fe0e4 / EU fn_1fe2a0 / KR fn_1fe34c.
/// Enable a named domain by appending its map value to the active list.
pub fn enable_domain(mem: &mut [u8], handle: Option<u32>, tag: u32, name: u32, api: &mut impl DomainApi) -> u32 {
    let (context, _) = match domain_context(mem, handle, tag, api) {
        Ok(v) => v, Err(rc) => return rc,
    };
    let handle = handle.unwrap();
    let log = mem_word(mem, context.wrapping_add(16));
    let mut item = 0;
    let mut rc = api.find(mem_word(mem, handle.wrapping_add(60)), name, &mut item);
    if rc < 0x80000000 {
        if api.append(mem_word(mem, handle.wrapping_add(64)), item) != 0 {
            api.trace(log, 0, name, rc);
            return rc;
        }
        api.trace(log, 0xb798, 0, 0);
        rc = 0x88e0200a;
    }
    api.trace(log, 0xb7a0, name, rc);
    rc
}

/// fn_1fe210 / EU fn_1fe3cc / KR fn_1fe478.
/// Remove the first matching domain from the mandatory prefix of the list.
pub fn disable_domain(mem: &mut [u8], handle: Option<u32>, tag: u32, name: u32, api: &mut impl DomainApi) -> u32 {
    let (context, _) = match domain_context(mem, handle, tag, api) {
        Ok(v) => v, Err(rc) => return rc,
    };
    let handle = handle.unwrap();
    let mut value = 0;
    let rc = api.find(mem_word(mem, handle.wrapping_add(60)), name, &mut value);
    if rc >= 0x80000000 { return rc; }
    let mut index = 0u32;
    let mut item = 0;
    while index < mem_word(mem, handle.wrapping_add(52)) {
        api.slot(mem_word(mem, handle.wrapping_add(64)), index, &mut item);
        if item != 0 && mem_word(mem, item) == value {
            api.remove(mem_word(mem, handle.wrapping_add(64)), index);
            let count = mem_word(mem, handle.wrapping_add(52));
            mem_store(mem, handle.wrapping_add(52), count.wrapping_sub(1));
            api.trace(mem_word(mem, context.wrapping_add(16)), 0, name, rc);
            break;
        }
        index = index.wrapping_add(1);
    }
    rc
}

/// fn_1fe328 / EU fn_1fe4e4 / KR fn_1fe590.
/// Pop enabled domains beyond the manager's minimum count.
pub fn disable_optional_domains(mem: &mut [u8], handle: Option<u32>, tag: u32, api: &mut impl DomainApi) -> u32 {
    let (context, rc) = match domain_context(mem, handle, tag, api) {
        Ok(v) => v, Err(rc) => return rc,
    };
    let handle = handle.unwrap();
    let list = mem_word(mem, handle.wrapping_add(64));
    let mut count = if list == 0 { 0 } else { mem_word(mem, list.wrapping_add(20)) };
    while mem_word(mem, handle.wrapping_add(52)) < count {
        api.pop(list);
        count = count.wrapping_sub(1);
    }
    api.trace(mem_word(mem, context.wrapping_add(16)), 0, 0, rc);
    rc
}

/// fn_1fe418 / EU fn_1fe5d4 / KR fn_1fe680.
/// Split "domain::arguments", try databases in reverse order, and free the copy.
pub fn build_domain_prompt(mem: &mut [u8], handle: u32, text: u32, output: u32, api: &mut impl DomainApi) -> u32 {
    let context = mem_word(mem, handle.wrapping_add(8));
    if output != 0 {
        mem[output as usize..output.wrapping_add(36) as usize].fill(0);
    }
    let heap = mem_word(mem, context.wrapping_add(4));
    let mut copy = 0;
    let mut rc = api.duplicate(heap, text, &mut copy);
    if rc < 0x80000000 {
        let mut separator = copy;
        while mem[separator as usize] != 0 {
            if mem[separator as usize] == b':' && mem[separator.wrapping_add(1) as usize] == b':' { break; }
            separator = separator.wrapping_add(1);
        }
        if mem[separator as usize] == 0 {
            rc = 0x88e02001;
        } else {
            mem[separator as usize] = 0;
            let args = separator.wrapping_add(2);
            let mut item = 0;
            rc = api.find(mem_word(mem, handle.wrapping_add(60)), copy, &mut item);
            if rc < 0x80000000 {
                rc = 0x88e02014;
                let list = mem_word(mem, item.wrapping_add(8));
                let mut index = if list == 0 { 0 } else { mem_word(mem, list.wrapping_add(20)) };
                let mut sub = 0;
                while index != 0 && api.slot(list, index.wrapping_sub(1), &mut sub) != 0 {
                    rc = api.build(mem, mem_word(mem, sub), args, output);
                    index = index.wrapping_sub(1);
                    if rc < 0x80000000 { break; }
                }
            }
        }
    }
    if context != 0 && copy != 0 { api.free(heap, copy); }
    if rc >= 0x80000000 {
        api.trace(mem_word(mem, context.wrapping_add(16)), 0xb7a8, text, rc);
    }
    rc
}

/// fn_1fe580 / EU fn_1fe73c / KR fn_1fe7e8.
/// Probe a domain prompt without retaining its decoded output.
pub fn domain_prompt_exists(handle: Option<u32>, tag: u32, text: u32, mut build: impl FnMut(u32, u32, u32) -> u32) -> u32 {
    if validate_handle(handle.is_some(), tag, 0x11d, 0x48) >= 0x80000000 { return 0; }
    u32::from(build(handle.unwrap(), text, 0) < 0x80000000)
}

/// fn_1fe5c4 / EU fn_1fe780 / KR fn_1fe82c.
/// Validate a domain handle and forward prompt decoding.
pub fn decode_domain_prompt(handle: Option<u32>, tag: u32, text: u32, output: u32, mut build: impl FnMut(u32, u32, u32) -> u32) -> u32 {
    let rc = validate_handle(handle.is_some(), tag, 0x11d, 0x48);
    if rc >= 0x80000000 { return rc; }
    build(handle.unwrap(), text, output)
}

/// fn_2817c0 / EU fn_28197c / KR fn_281a28.
/// Unlink the first node with the requested tag, preserving the list's tail.
pub fn remove_domain_key(mem: &mut [u8], list: u32, key: u32, mut release: impl FnMut(u32, u32)) -> u32 {
    let mut prev = 0u32;
    let mut cur = mem_word(mem, list.wrapping_add(4));
    while cur != 0 {
        let next = mem_word(mem, cur.wrapping_add(8));
        if mem_word(mem, cur.wrapping_add(4)) == key {
            if prev == 0 {
                mem_store(mem, list.wrapping_add(4), next);
                if next == 0 { mem_store(mem, list.wrapping_add(8), 0); }
                release(mem_word(mem, list), cur);
            } else {
                release(mem_word(mem, list), cur);
                mem_store(mem, prev.wrapping_add(8), next);
                if next == 0 { mem_store(mem, list.wrapping_add(8), prev); }
            }
            return 0;
        }
        prev = cur;
        cur = next;
    }
    0x85002014
}

/// fn_281864 / EU fn_281a20 / KR fn_281acc.
/// Drain the domain-key list and release every detached node.
pub fn clear_domain_keys(mem: &mut [u8], list: u32, mut release: impl FnMut(u32, u32)) -> u32 {
    loop {
        let cur = mem_word(mem, list.wrapping_add(4));
        if cur == 0 { return 0; }
        let next = mem_word(mem, cur.wrapping_add(8));
        mem_store(mem, list.wrapping_add(4), next);
        if next == 0 { mem_store(mem, list.wrapping_add(8), 0); }
        release(mem_word(mem, list), cur);
    }
}

/// fn_281948 / EU fn_281b04 / KR fn_281bb0.
/// Move a tagged key to its new ordered position without allocating.
pub fn reposition_domain_key(mem: &mut [u8], list: u32, tag: u32, key: u32) -> u32 {
    let mut node = mem_word(mem, list.wrapping_add(4));
    if node == 0 { return 0x85002014; }
    let mut cur = node;
    let mut prev = 0;
    let mut pred = 0;
    let mut ins = 0;
    loop {
        let current_tag = mem_word(mem, cur.wrapping_add(4));
        let current_key = mem_word(mem, cur);
        if current_tag == tag {
            if current_key == key { return 0; }
            pred = prev;
        }
        if current_key == key && key != u32::MAX { return 0x85002419; }
        if current_key <= key && current_tag != tag { ins = cur; }
        prev = cur;
        if pred != 0 && ins != 0 { break; }
        cur = mem_word(mem, cur.wrapping_add(8));
        if cur == 0 { break; }
    }
    if key == u32::MAX && ins != 0 && mem_word(mem, ins.wrapping_add(8)) != mem_word(mem, list.wrapping_add(8)) {
        ins = mem_word(mem, list.wrapping_add(8));
    }
    if pred == ins {
        let target = if pred != 0 { mem_word(mem, pred.wrapping_add(8)) } else { node };
        mem_store(mem, target, key);
        return 0;
    }
    if pred != 0 {
        node = mem_word(mem, pred.wrapping_add(8));
        let next = mem_word(mem, node.wrapping_add(8));
        mem_store(mem, pred.wrapping_add(8), next);
        if next == 0 { mem_store(mem, list.wrapping_add(8), pred); }
    } else {
        if mem_word(mem, node.wrapping_add(4)) != tag { return 0x85002014; }
        let next = mem_word(mem, node.wrapping_add(8));
        mem_store(mem, list.wrapping_add(4), next);
        if next == 0 { mem_store(mem, list.wrapping_add(8), 0); }
    }
    let next = mem_word(mem, if ins != 0 { ins.wrapping_add(8) } else { list.wrapping_add(4) });
    mem_store(mem, node.wrapping_add(8), next);
    if ins != 0 {
        mem_store(mem, ins.wrapping_add(8), node);
        if next == 0 { mem_store(mem, list.wrapping_add(8), node); }
    } else {
        mem_store(mem, list.wrapping_add(4), node);
    }
    mem_store(mem, node, key);
    0
}

/// fn_281338 / EU fn_2814f4 / KR fn_2815a0.
/// Resolve the resource context and release its optional database array.
pub fn release_domain_databases(mem: &mut [u8], object: Option<u32>, mut resolve: impl FnMut(u32, u32, &mut u32) -> u32,
    mut release: impl FnMut(u32)) -> u32 {
    let Some(object) = object else { return 0x88e02007; };
    let mut context = 0;
    let rc = resolve(mem_word(mem, object), mem_word(mem, object.wrapping_add(4)), &mut context);
    if rc >= 0x80000000 { return rc; }
    let array = mem_word(mem, object.wrapping_add(8));
    if array != 0 {
        release(array);
        mem_store(mem, object.wrapping_add(8), 0);
    }
    rc
}



/// Caller-owned property queries and diagnostic boundaries for APDB validation.
pub trait DomainValidation {
    fn property(&mut self, props: u32, name: &[u8], value: &mut u32) -> u32;
    fn mismatch(&mut self, log: u32, strict: bool, domain: &[u8], attribute: &[u8], voice: Option<&[u8]>, apdb: &[u8]);
    fn version_mismatch(&mut self, log: u32, reduction: bool, voice: Option<&[u8]>, apdb: &[u8]);
}

/// fn_27df20 / EU fn_27e0dc / KR fn_27e188.
pub fn match_domain_attribute(mem: &[u8], engine: u32, strict: u32, domain: &[u8], attribute: &[u8], apdb: &[u8], alternate: Option<&[u8]>, api: &mut impl DomainValidation) -> u32 {
    let mut current = 0;
    let rc = api.property(mem_word(mem, engine + 20), attribute, &mut current);
    let value = if current == 0 { None } else { Some(&mem[current as usize..]) };
    if rc < 0x80000000 {
        let value = value.expect("successful property query returns text");
        if cmp_fold(value, apdb) == 0 || alternate.is_some_and(|v| cmp_fold(value, v) == 0) { return 1; }
    }
    api.mismatch(mem_word(mem, engine + 16), strict != 0, domain, attribute, value, apdb);
    0
}

/// fn_27e0d4 / EU fn_27e290 / KR fn_27e33c.
/// Validate voice identity and raw/reduction versions, caching state 1 or 2.
pub fn validate_domain(mem: &mut [u8], ctx: u32, api: &mut impl DomainValidation) {
    let engine = mem_word(mem, mem_word(mem, ctx) + 8);
    let domain = mem_word(mem, ctx + 88);
    let check = |mem: &[u8], strict, name: &[u8], value: &[u8], alternate: Option<&[u8]>, api: &mut _| {
        match_domain_attribute(mem, engine, strict, &mem[domain as usize..], name, value, alternate, api) != 0
    };
    let mut ok = check(mem, 0, b"langcode\0", &mem[(ctx+36) as usize..], None, api);
    if ok && !check(mem, 0, b"voice\0", &mem[mem_word(mem, ctx+44) as usize..], None, api) {
        ok = check(mem, 0, b"fevoice\0", &mem[mem_word(mem, ctx+44) as usize..], None, api);
    }
    if ok && mem[(ctx+56) as usize] != 0 {
        ok = check(mem, 0, b"reduction\0", &mem[(ctx+56) as usize..], None, api);
    }
    if ok && mem[(ctx+76) as usize] != 0 {
        ok = if cmp_fold(&mem[(ctx+76) as usize..], b"dev\0") == 0 {
            check(mem, 1, b"reduction\0", b"devred\0", Some(b"devfull\0"), api)
        } else { check(mem, 1, b"reduction\0", &mem[(ctx+76) as usize..], None, api) };
    }
    let mut value = 0;
    for (off, name, reduction) in [(48, &b"uselectrawdataversion\0"[..], false), (68, &b"uselectreductionversion\0"[..], true)] {
        if ok && mem[(ctx+off) as usize] != 0 {
            let rc = api.property(mem_word(mem, engine+20), name, &mut value);
            let voice = if value == 0 { None } else { Some(&mem[value as usize..]) };
            if rc >= 0x80000000 || cmp_fold(voice.unwrap(), &mem[(ctx+off) as usize..]) != 0 {
                api.version_mismatch(mem_word(mem, engine+16), reduction, voice, &mem[(ctx+off) as usize..]);
                ok = false;
            }
        }
    }
    mem[(ctx+13) as usize] = if ok { 2 } else { 1 };
}

/// fn_27e400 / EU fn_27e5bc / KR fn_27e668.
/// Query each index once, append nonempty names and separators, and return unsupported.
pub fn list_domain_names(mem: &mut [u8], ctx: u32, output: u32, api: &mut impl DomainValidation, mut slot: impl FnMut(u32, u32, &mut u32) -> u32, mut append: impl FnMut(u32, Option<&[u8]>)) -> u32 {
    if mem[(ctx+13) as usize] == 0 { validate_domain(mem, ctx, api); }
    if mem[(ctx+13) as usize] == 2 {
        let mut item = 0;
        let mut index = 0u32;
        loop {
            if slot(mem_word(mem, ctx+116), index, &mut item) == 0 { break; }
            let name = mem_word(mem, item+4);
            if name != 0 && mem[name as usize] != 0 {
                append(output, Some(&mem[name as usize..]));
                append(output, None);
            }
            index = index.wrapping_add(1);
        }
    }
    0x88e02014
}


/// Remaining caller-owned database and iterator operations for consultation.
pub trait DomainConsultation: DomainApi + VoiceNodeAllocator {
    fn allocate_name(&mut self, heap: u32, size: u32) -> u32;
    fn map_iter(&mut self, map: u32, iterator: &mut u32) -> u32;
    fn map_next(&mut self, iterator: u32, key: &mut u32, value: &mut u32) -> u32;
    fn map_close(&mut self, iterator: u32);
    fn expand_original(&mut self, mem: &mut [u8], database: u32, text: u32, entries: u32, count: u32, head: u32) -> u32;
    /// The original optional begin/end text and marker diagnostics.
    fn consultation_log(&mut self, mem: &[u8], log: u32, text: u32, entries: u32, count: u32, status: Option<u32>);
}

fn domain_wide_equal(mem: &[u8], a: Option<u32>, b: Option<u32>) -> bool {
    let normal = [110u16,111,114,109,97,108,0];
    let mut i=0usize;
    loop {
        let x=a.map_or_else(||normal[i],|p|mem_half(mem,p.wrapping_add((i*2) as u32)));
        let y=b.map_or_else(||normal[i],|p|mem_half(mem,p.wrapping_add((i*2) as u32)));
        if x!=y {return false;}
        if x==0 {return true;}
        i+=1;
    }
}

fn consult_active_domains(mem: &mut [u8], manager:u32, text:u32, entries:u32, count:u32, head:u32, api:&mut impl DomainConsultation)->u32 {
    let ctx=mem_word(mem,manager+8);
    let log=mem_word(mem,ctx+16);
    api.consultation_log(mem,log,text,entries,count,None);
    let mut iterator=0;let mut key=0;let mut value=0;
    if api.map_iter(mem_word(mem,manager+60),&mut iterator)<0x80000000 {
        while api.map_next(iterator,&mut key,&mut value)<0x80000000 {
            if value!=0 {mem[(value+12) as usize]=0;}
        }
        api.map_close(iterator);
    }
    let list=mem_word(mem,manager+64);
    let mut i=if list==0 {0}else{mem_word(mem,list+20)};
    let mut domain=0;let mut database=0;let mut rc=0;
    'domains: while i!=0 && api.slot(list,i-1,&mut domain)!=0 {
        let d=mem_word(mem,domain);
        if d==0 || mem[(d+12) as usize]==0 {
            let databases=mem_word(mem,d+8);
            let mut j=if databases==0 {0}else{mem_word(mem,databases+20)};
            while j!=0 && api.slot(databases,j-1,&mut database)!=0 {
                rc=api.expand_original(mem,mem_word(mem,database),text,entries,count,head);
                if rc>=0x80000000 {break 'domains;}
                j-=1;
            }
            if d!=0 {mem[(d+12) as usize]=1;}
        }
        i-=1;
    }
    api.consultation_log(mem,log,text,entries,count,Some(rc));
    rc
}

/// fn_1fe600 / EU fn_1fe7bc / KR fn_1fe868.
/// Split original orthography into prompt spans, apply active databases, and
/// release every temporary node. All text/marker changes stay in caller memory.
pub fn consult_original_domains(mem:&mut [u8], handle:Option<u32>, tag:u32, text:u32, entries:u32, count:u32, api:&mut impl DomainConsultation)->u32 {
    let (ctx,mut rc)=match domain_context(mem,handle,tag,api){Ok(v)=>v,Err(rc)=>return rc};
    let manager=handle.unwrap();let heap=mem_word(mem,ctx+4);let log=mem_word(mem,ctx+16);
    api.trace(log,0,0,0);
    mem[(manager+68) as usize]=1;
    let entry=mem_word(mem,entries);
    let head=create_voice_node(mem,heap,1,mem_word(mem,entry+12),mem_word(mem,entry+16),None,0,None,api,&mut |_,_,_|unreachable!());
    if head==0 {api.trace(log,0xb798,0,0);return 0x88e0200a;}
    let mut node=head;let mut first=None;let mut second=None;let mut i=0u16;
    while i<mem_half(mem,count) {
        let e=mem_word(mem,entries).wrapping_add((i as u32)*24);
        let kind=mem_half(mem,e);let value=mem_word(mem,e+20);let position=mem_word(mem,e+12);
        i=i.wrapping_add(1);
        if kind==12 {
            if value!=0 && mem_half(mem,value)!=0 && !domain_wide_equal(mem,Some(value),None) {
                let name=api.allocate_name(heap,u16_len(mem,value).wrapping_add(1));
                if name==0 {api.trace(log,0xb798,0,0);rc=0x88e0200a;break;}
                recode_176460(mem,name,value);
                enable_domain(mem,handle,tag,name,api);
                api.free(heap,name);
            } else {
                let list=mem_word(mem,manager+64);
                let n=if list==0{0}else{mem_word(mem,list+20)};
                if n>mem_word(mem,manager+52){api.pop(list);}else{api.trace(log,0xb7a5,0,0);}
            }
            continue;
        }
        let boundary=matches!(kind,29|8|40|26);
        let style=(kind==21 && !domain_wide_equal(mem,first,Some(value))) || kind==36;
        if !boundary && !style {continue;}
        let mut new_kind=mem[node as usize] as u32;
        if !boundary {
            new_kind=if domain_wide_equal(mem,Some(value),None) && ((kind==21 && domain_wide_equal(mem,second,None)) || (kind==36 && domain_wide_equal(mem,first,None))) {1}else{0};
            if kind==21 {first=Some(value);}else{second=Some(value);}
        }
        let start=mem_word(mem,node+4);let length=mem_word(mem,node+8);
        if position>start && position.wrapping_sub(start)!=length {
            let amount=position.wrapping_sub(start);
            rc=update_voice_node(mem,heap,node,mem[node as usize] as u32,start,amount,None,0,None,api);
            if rc>=0x80000000 {break;}
            let rest=length.wrapping_sub(mem_word(mem,node+8));
            let next=create_voice_node(mem,heap,new_kind,position,rest,None,0,None,api,&mut |_,_,_|unreachable!());
            if next==0 {api.trace(log,0xb798,0,0);rc=0x88e0200a;break;}
            mem_store(mem,node+24,next);node=next;
        } else if !boundary && position==start {
            rc=update_voice_node(mem,heap,node,new_kind,start,length,None,0,None,api);
            if rc>=0x80000000 {break;}
        }
    }
    if i>=mem_half(mem,count) && rc<0x80000000 && u16_len(mem,mem_word(mem,text))!=0 {
        rc=consult_active_domains(mem,manager,text,entries,count,head,api);
    }
    release_node_chain(mem,Some(head),nz(heap),api);
    api.trace(log,0,0,rc);
    rc
}


/// fn_279c28 / EU fn_279de4 / KR fn_279e90.
/// Format the APDB character-class matching diagnostic into a caller buffer.
pub fn format_domain_match(mem:&mut [u8], flag:u32, kind:u32, ch:u32, n:u32, chars:u32, mark:u32, out:u32)->u32 {
    const NAMES:[&[u8];8]=[b"[*]",b"[S]",b"[P]",b"[S|P]",b"[...]",b"[p]",b"[P|...]",b"[PG]"];
    fn class(mem:&mut [u8],at:&mut usize,kind:u32,ch:u32){
        if kind==5 {mem[*at..*at+3].copy_from_slice(&[b'[',ch as u8,b']']);*at+=3;}else{let text=NAMES[kind as usize];mem[*at..*at+text.len()].copy_from_slice(text);*at+=text.len();}
        mem[*at]=0;
    }
    fn append(mem:&mut [u8],out:u32,text:&[u8]) {let at=out as usize+cstrlen(&mem[out as usize..]) as usize;mem[at..at+text.len()].copy_from_slice(text);mem[at+text.len()]=0;}
    let mut at=out as usize;class(mem,&mut at,kind,ch);
    append(mem,out,if flag==1{b" matches ("}else{b" does not match ("});
    for i in 0..n {if i!=0 {append(mem,out,b" ");}let mut at=out as usize+cstrlen(&mem[out as usize..]) as usize;class(mem,&mut at,mem[(chars+i) as usize] as u32,mark);}
    append(mem,out,b")");out
}

/// fn_27b678 / EU fn_27b834 / KR fn_27b8e0.
/// Find a normalized text span and count missing prompt separators.
/// Compare candidate positions with the selected record's position, not the first record.
pub fn domain_normalized_span(mem:&mut [u8],text:u32,recs:u32,n:u32,mut index:u32,counts:[u32;3],start_out:u32,end_out:u32)->u32 {
    if n==0 || index>=n || mem_half(mem,recs)!=0x4000 || mem_half(mem,recs+index*24)!=0x22 {return 0;}
    for p in counts {mem_store16(mem,p,0);}
    let selected=recs+index*24;let origin=mem_word(mem,recs+12);let selected_pos=mem_word(mem,selected+12);
    let name=mem_word(mem,selected+20);let length=u16_len(mem,name);let mut expected=1u16;
    let mut i=1u16;
    if length.wrapping_sub(1)>1 {loop {let c=mem_half(mem,name+(i as u32)*2);if c==95 || c==35 {expected=expected.wrapping_add(1);}i=i.wrapping_add(1);if i as u32>=length.wrapping_sub(1){break;}}}
    mem_store16(mem,counts[0],expected);
    let start=selected_pos.wrapping_sub(origin);mem_store(mem,start_out,start);
    let mut mark=0u32;
    loop {index=index.wrapping_add(1)&0xffff;if n<=index {break;}let e=recs+index*24;let t=mem_half(mem,e);if matches!(t,21|34) && mem_word(mem,e+12)!=selected_pos {mark=mem_word(mem,e+12).wrapping_sub(origin);break;}}
    if index==n {mark=cstrlen(&mem[text as usize..]);}
    let is_space=|c:u8|c==b' ' || c==b'_' || c==0;
    let mut end=mark.wrapping_sub(1);
    while start<end && (is_space(mem[(text+end) as usize]) || mem[(text+end) as usize]==0x15) {end=end.wrapping_sub(1);}
    while start<end {if is_space(mem[(text+end) as usize]) {end=end.wrapping_add(1);break;}end=end.wrapping_sub(1);}
    mem_store(mem,end_out,end);
    let mut actual=0u16;let mut j=start;while j<mark {if mem[(text+j) as usize]==0x15 {actual=actual.wrapping_add(1);}j=j.wrapping_add(1);}
    mem_store16(mem,counts[2],actual);
    if actual<=expected {mem_store16(mem,counts[1],expected.wrapping_sub(actual));1}else{0}
}


/// Caller-owned domain database-vector lifecycle operations.
pub trait DomainDatabaseList {
    fn resolve(&mut self, first:u32, second:u32, context:&mut u32)->u32;
    fn create(&mut self, heap:u32, config:[u32;3], single:u32, array:&mut u32)->u32;
    fn release(&mut self, array:u32);
    fn append(&mut self, array:u32, value:u32)->u32;
    fn no_memory(&mut self, log:u32);
}

/// fn_281384 / EU fn_281540 / KR fn_2815ec.
/// Initialize a domain's database vector, releasing partial allocation on error.
pub fn initialize_domain_databases(mem:&mut [u8],first:u32,second:u32,out:u32,config:[u32;3],api:&mut impl DomainDatabaseList)->u32 {
    let mut context=0;let mut rc=api.resolve(first,second,&mut context);
    if rc>=0x80000000 {return rc;}
    mem[out as usize..out as usize+16].fill(0);mem_store(mem,out,first);mem_store(mem,out+4,second);
    let mut array=0;
    let created=api.create(mem_word(mem,context+4),config,1,&mut array);mem_store(mem,out+8,array);
    if created==0 {
        api.no_memory(mem_word(mem,context+16));rc=0x88e0200a;
        let mut again=0;
        if api.resolve(mem_word(mem,out),mem_word(mem,out+4),&mut again)<0x80000000 && mem_word(mem,out+8)!=0 {
            api.release(mem_word(mem,out+8));mem_store(mem,out+8,0);
        }
    }
    rc
}

/// fn_281464 / EU fn_281620 / KR fn_2816cc.
/// Append one database handle to a domain's vector.
pub fn append_domain_database(mem:&[u8],domain:Option<u32>,value:u32,api:&mut impl DomainDatabaseList)->u32 {
    let Some(domain)=domain else{return 0x88e02007;};
    if api.append(mem_word(mem,domain+8),value)==0 {0x88e0200a}else{0}
}


// ---- Additive reentrant hook entries. Legacy traits/entries above are unchanged. ----
/// Live-memory counterpart of CreateHooks. Memory is the final argument so
/// scalar guest reads finish before its exclusive reborrow. Guest-owned
/// strings use u32 offsets for touch_param, trace_pipeline and install_voice;
/// constant/local byte slices keep their existing signatures. No arena slice
/// may remain borrowed across a callback that receives mutable memory.
/// Every existing CreateHooks implementation is adapted without modification.
pub trait CreateHooksM {
    #[allow(clippy::too_many_arguments)]
    fn check_open_m(&mut self, obj: u32, b: u32, mem: &mut [u8]) -> u32;
    #[allow(clippy::too_many_arguments)]
    fn resolve_ctx_m(&mut self, a: u32, b: u32, mem: &mut [u8]) -> (u32, u32);
    #[allow(clippy::too_many_arguments)]
    fn alloc_object_m(&mut self, n: u32, size: u32, mem: &mut [u8]) -> u32;
    #[allow(clippy::too_many_arguments)]
    fn make_mutex_m(&mut self, pad8: u32, heap: u32, mem: &mut [u8]) -> (u32, u32);
    #[allow(clippy::too_many_arguments)]
    fn make_param_cb_m(&mut self, heap: u32, cmp: u32, cpy: u32, rel: u32, mem: &mut [u8]) -> (u32, u32);
    #[allow(clippy::too_many_arguments)]
    fn query_comp_m(&mut self, comp: u32, name: &[u8], mem: &mut [u8]) -> (u32, u32);
    #[allow(clippy::too_many_arguments)]
    fn register_comp_m(&mut self, c: u32, d: u32, name: &[u8], lname: &[u8], param_cb: u32, mem: &mut [u8]) -> u32;
    #[allow(clippy::too_many_arguments)]
    fn release_comp_m(&mut self, comp: u32, name: &[u8], mem: &mut [u8]) ;
    #[allow(clippy::too_many_arguments)]
    fn insert_comp_m(&mut self, comp: u32, name: &[u8], buf: &[u8; 20], mem: &mut [u8]) -> u32;
    #[allow(clippy::too_many_arguments)]
    fn log_level_m(&mut self, log: u32, mem: &mut [u8]) -> u32;
    #[allow(clippy::too_many_arguments)]
    fn reg_param_cb_m(&mut self, params: u32, key: Option<&[u8]>, this: u32, mem: &mut [u8]) -> u32;
    #[allow(clippy::too_many_arguments)]
    fn query_param_m(&mut self, params: u32, key: &[u8], mem: &mut [u8]) -> (u32, u32);
    #[allow(clippy::too_many_arguments)]
    fn touch_param_m(&mut self, params: u32, value: u32, mem: &mut [u8]) ;
    #[allow(clippy::too_many_arguments)]
    fn set_default_num_m(&mut self, params: u32, key: &[u8], value: u32, mem: &mut [u8]) ;
    #[allow(clippy::too_many_arguments)]
    fn set_default_str_m(&mut self, params: u32, key: &[u8], value: &[u8], mem: &mut [u8]) ;
    #[allow(clippy::too_many_arguments)]
    fn query_num_m(&mut self, params: u32, key: &[u8], mem: &mut [u8]) -> (u32, u32);
    #[allow(clippy::too_many_arguments)]
    fn lic_token_m(&mut self, c: u32, d: u32, mem: &mut [u8]) -> (u32, u32);
    #[allow(clippy::too_many_arguments)]
    fn trace_pipeline_m(&mut self, ha: u32, hb: u32, mime: u32, param_cb: u32, mem: &mut [u8]) -> (u32, u32, u8);
    #[allow(clippy::too_many_arguments)]
    fn start_object_m(&mut self, ctx: u32, obj: u32, mem: &mut [u8]) -> u32;
    #[allow(clippy::too_many_arguments)]
    fn install_voice_m(
        &mut self,
        h: (u32, u32),
        mime: &[u8],
        key: &[u8],
        text: u32,
        len: u32,
        blob: (u32, u32), mem: &mut [u8]) -> (u32, u32, u32);
    #[allow(clippy::too_many_arguments)]
    fn null_pair_m(&mut self, mem: &mut [u8]) -> (u32, u32);
    #[allow(clippy::too_many_arguments)]
    fn alloc_temp_m(&mut self, heap: u32, size: u32, mem: &mut [u8]) -> u32;
    #[allow(clippy::too_many_arguments)]
    fn free_temp_m(&mut self, heap: u32, ptr: u32, mem: &mut [u8]) ;
    #[allow(clippy::too_many_arguments)]
    fn teardown_m(&mut self, ctx: u32, mem: &mut [u8]) ;
    #[allow(clippy::too_many_arguments)]
    fn release_voice_m(&mut self, h: (u32, u32), ptr: u32, n: u32, mem: &mut [u8]) ;
}
impl<T: CreateHooks + ?Sized> CreateHooksM for T {
    fn check_open_m(&mut self, obj: u32, b: u32, mem: &mut [u8]) -> u32 { let _ = &mem; CreateHooks::check_open(self, obj, b) }
    fn resolve_ctx_m(&mut self, a: u32, b: u32, mem: &mut [u8]) -> (u32, u32) { let _ = &mem; CreateHooks::resolve_ctx(self, a, b) }
    fn alloc_object_m(&mut self, n: u32, size: u32, mem: &mut [u8]) -> u32 { let _ = &mem; CreateHooks::alloc_object(self, n, size) }
    fn make_mutex_m(&mut self, pad8: u32, heap: u32, mem: &mut [u8]) -> (u32, u32) { let _ = &mem; CreateHooks::make_mutex(self, pad8, heap) }
    fn make_param_cb_m(&mut self, heap: u32, cmp: u32, cpy: u32, rel: u32, mem: &mut [u8]) -> (u32, u32) { let _ = &mem; CreateHooks::make_param_cb(self, heap, cmp, cpy, rel) }
    fn query_comp_m(&mut self, comp: u32, name: &[u8], mem: &mut [u8]) -> (u32, u32) { let _ = &mem; CreateHooks::query_comp(self, comp, name) }
    fn register_comp_m(&mut self, c: u32, d: u32, name: &[u8], lname: &[u8], param_cb: u32, mem: &mut [u8]) -> u32 { let _ = &mem; CreateHooks::register_comp(self, c, d, name, lname, param_cb) }
    fn release_comp_m(&mut self, comp: u32, name: &[u8], mem: &mut [u8])  { let _ = &mem; CreateHooks::release_comp(self, comp, name) }
    fn insert_comp_m(&mut self, comp: u32, name: &[u8], buf: &[u8; 20], mem: &mut [u8]) -> u32 { let _ = &mem; CreateHooks::insert_comp(self, comp, name, buf) }
    fn log_level_m(&mut self, log: u32, mem: &mut [u8]) -> u32 { let _ = &mem; CreateHooks::log_level(self, log) }
    fn reg_param_cb_m(&mut self, params: u32, key: Option<&[u8]>, this: u32, mem: &mut [u8]) -> u32 { let _ = &mem; CreateHooks::reg_param_cb(self, params, key, this) }
    fn query_param_m(&mut self, params: u32, key: &[u8], mem: &mut [u8]) -> (u32, u32) { let _ = &mem; CreateHooks::query_param(self, params, key) }
    fn touch_param_m(&mut self, params: u32, value: u32, mem: &mut [u8])  { CreateHooks::touch_param(self, params, terminated(mem, value)) }
    fn set_default_num_m(&mut self, params: u32, key: &[u8], value: u32, mem: &mut [u8])  { let _ = &mem; CreateHooks::set_default_num(self, params, key, value) }
    fn set_default_str_m(&mut self, params: u32, key: &[u8], value: &[u8], mem: &mut [u8])  { let _ = &mem; CreateHooks::set_default_str(self, params, key, value) }
    fn query_num_m(&mut self, params: u32, key: &[u8], mem: &mut [u8]) -> (u32, u32) { let _ = &mem; CreateHooks::query_num(self, params, key) }
    fn lic_token_m(&mut self, c: u32, d: u32, mem: &mut [u8]) -> (u32, u32) { let _ = &mem; CreateHooks::lic_token(self, c, d) }
    fn trace_pipeline_m(&mut self, ha: u32, hb: u32, mime: u32, param_cb: u32, mem: &mut [u8]) -> (u32, u32, u8) { CreateHooks::trace_pipeline(self, ha, hb, terminated(mem, mime), param_cb) }
    fn start_object_m(&mut self, ctx: u32, obj: u32, mem: &mut [u8]) -> u32 { let _ = &mem; CreateHooks::start_object(self, ctx, obj) }
    fn install_voice_m(
        &mut self,
        h: (u32, u32),
        mime: &[u8],
        key: &[u8],
        text: u32,
        len: u32,
        blob: (u32, u32), mem: &mut [u8]) -> (u32, u32, u32) { CreateHooks::install_voice(self, h, mime, key, &mem[text as usize..text as usize + len as usize], len, blob) }
    fn null_pair_m(&mut self, mem: &mut [u8]) -> (u32, u32) { let _ = &mem; CreateHooks::null_pair(self) }
    fn alloc_temp_m(&mut self, heap: u32, size: u32, mem: &mut [u8]) -> u32 { let _ = &mem; CreateHooks::alloc_temp(self, heap, size) }
    fn free_temp_m(&mut self, heap: u32, ptr: u32, mem: &mut [u8])  { let _ = &mem; CreateHooks::free_temp(self, heap, ptr) }
    fn teardown_m(&mut self, ctx: u32, mem: &mut [u8])  { let _ = &mem; CreateHooks::teardown(self, ctx) }
    fn release_voice_m(&mut self, h: (u32, u32), ptr: u32, n: u32, mem: &mut [u8])  { let _ = &mem; CreateHooks::release_voice(self, h, ptr, n) }
}

pub fn open_object_m<H: CreateHooksM>(
    hooks: &mut H,
    mem: &mut [u8],
    obj: u32,
    b: u32,
    slot: Option<&mut u32>,
    out: &mut u32,
) -> u32 {
    *out = 0;
    let rc = hooks.check_open_m(obj, b, mem);
    if rc >= 0x80000000 {
        if rc & 0x1fff == 8 {
            return 0x81202008;
        }
        return rc;
    }
    *out = obj;
    if let Some(slot) = slot {
        let (rc, ctx) = hooks.resolve_ctx_m(mem_word(mem, obj), mem_word(mem, obj.wrapping_add(4)), mem);
        *slot = ctx;
        rc
    } else {
        rc
    }
}

pub fn check_key_m<H: CreateHooksM>(hooks: &mut H, mem: &mut [u8], params: u32) -> u32 {
    let (rc, src) = hooks.query_param_m(params, CREATE_TXMKEY, mem);
    if rc >= 0x80000000 || src == 0 {
        return 0;
    }
    let src = terminated(mem, src);
    let mut buf = [0u8; 100];
    let key = b"key";
    let k = key.len() as u32;
    let mut n = cstrlen(src);
    let (mut i, mut j) = (0usize, 0u32);
    if n != 0 {
        if n & 1 != 0 {
            j = u32::from(k > 1);
            buf[0] = key[0] ^ src[0];
            i = 1;
        }
        n >>= 1;
        while n != 0 {
            buf[i] = key[j as usize] ^ src[i];
            j = if j < k - 1 { j + 1 } else { 0 };
            i += 1;
            buf[i] = key[j as usize] ^ src[i];
            j = if j < k - 1 { j + 1 } else { 0 };
            i += 1;
            n -= 1;
        }
    }
    u32::from(cmp_fold(&buf, CREATE_TXMWANT) == 0)
}

pub fn license_kind_m<H: CreateHooksM>(hooks: &mut H, mem: &mut [u8], ctx: u32, obj: u32, trace: u32) -> u32 {
    let lic = mem_word(mem, obj.wrapping_add(0x10));
    let (rc, inst) = hooks.resolve_ctx_m(mem_word(mem, lic), mem_word(mem, lic.wrapping_add(4)), mem);
    if rc < 0x80000000 && { let params = mem_word(mem, inst.wrapping_add(0x14)); check_key_m(hooks, mem, params) } != 0 {
        return 2;
    }
    if trace == 0 {
        return 0;
    }
    if mem_word(mem, lic.wrapping_add(0x4c)) == 0 && mem_word(mem, lic.wrapping_add(0x58)) == 0 {
        return 0;
    }
    let (rc, voice) = hooks.query_param_m(mem_word(mem, ctx.wrapping_add(0x14)), CREATE_VOICEMODEL, mem);
    if rc >= 0x80000000 || voice == 0 {
        return 0;
    }
    if cmp_fold(terminated(mem, voice), CREATE_CPR_Z) != 0 {
        return 0;
    }
    let count = mem_half(mem, trace.wrapping_add(0xc));
    if count == 0 {
        return 1;
    }
    let mut i = 0u32;
    loop {
        let entry = mem_word(mem, mem_word(mem, trace.wrapping_add(8)).wrapping_add(i.wrapping_mul(4)));
        let e = terminated(mem, mem_word(mem, entry.wrapping_add(4)));
        if cmp_fold_n(e, CREATE_USELECT_Z, 8) == 0 {
            return 0;
        }
        if cmp_fold_n(e, CREATE_SELECT_Z, 7) == 0 && cmp_fold(e, CREATE_SELCPR_Z) != 0 {
            return 0;
        }
        i = (i.wrapping_add(1)) & 0xffff;
        if i >= u32::from(count) {
            break;
        }
    }
    1
}

pub fn install_esc_seq_m<H: CreateHooksM>(
    hooks: &mut H,
    mem: &mut [u8],
    ctx: u32,
    obj: u32,
    value: u32,
) -> u32 {
    let heap = mem_word(mem, ctx.wrapping_add(4));
    let pair = hooks.null_pair_m(mem);
    let mut rc = 0;
    if mem_word(mem, obj.wrapping_add(0x144)) != 0 {
        let h = (mem_word(mem, obj.wrapping_add(8)), mem_word(mem, obj.wrapping_add(12)));
        hooks.release_voice_m(
            h,
            mem_word(mem, obj.wrapping_add(0x144)),
            mem_word(mem, obj.wrapping_add(0x148)), mem);
        let pair = hooks.null_pair_m(mem);
        mem_store(mem, obj.wrapping_add(0x144), pair.0);
        mem_store(mem, obj.wrapping_add(0x148), pair.1);
        rc = pair.0;
    }
    if value != 0 {
        rc = u32::from(mem[value as usize]);
    }
    if value != 0 && rc != 0 {
        let n1 = CREATE_ESCHEAD.len() as u32;
        let n2 = cstrlen(terminated(mem, value));
        let n3 = CREATE_ESCTAIL.len() as u32;
        let size = n3.wrapping_add(n1).wrapping_add(n2).wrapping_add(1);
        let q = hooks.alloc_temp_m(heap, size, mem);
        let qz = q as usize;
        mem[qz..qz + n1 as usize].copy_from_slice(CREATE_ESCHEAD);
        mem[qz + n1 as usize] = 0;
        let at = qz + n1 as usize;
        mem.copy_within(value as usize..value as usize + n2 as usize, at);
        mem[at + n2 as usize] = 0;
        let at = at + n2 as usize;
        mem[at..at + n3 as usize].copy_from_slice(CREATE_ESCTAIL);
        mem[at + n3 as usize] = 0;
        let len = cstrlen(terminated(mem, q));
        let h = (mem_word(mem, obj.wrapping_add(8)), mem_word(mem, obj.wrapping_add(12)));
        let text = q;
        let (status, p0, p1) = hooks.install_voice_m(
            h,
            CREATE_VOICEMIME,
            CREATE_NUANKEY,
            text,
            len,
            pair, mem);
        mem_store(mem, obj.wrapping_add(0x144), p0);
        mem_store(mem, obj.wrapping_add(0x148), p1);
        hooks.free_temp_m(heap, q, mem);
        rc = status;
    }
    rc
}

fn fail_create_m<H: CreateHooksM>(hooks: &mut H, mem: &mut [u8], ctx: u32, out: u32, rc: u32) -> u32 {
    hooks.teardown_m(ctx, mem);
    mem_store(mem, out, 0);
    mem_store(mem, out.wrapping_add(4), 0);
    rc
}

pub fn create_object_m<H: CreateHooksM>(
    hooks: &mut H,
    mem: &mut [u8],
    obj: u32,
    b: u32,
    c: u32,
    d: u32,
    opts: u32,
    sink: u32,
    mime: u32,
    out: u32,
) -> u32 {
    let mut lic = 0;
    let mut ctx = 0;
    let rc = open_object_m(hooks, mem, obj, b, Some(&mut ctx), &mut lic);
    if rc >= 0x80000000 {
        return 0x81202008;
    }
    if out == 0 {
        return 0x81202006;
    }
    let (rc, fresh) = hooks.resolve_ctx_m(c, d, mem);
    ctx = fresh;
    if rc >= 0x80000000 {
        return rc;
    }
    let heap = mem_word(mem, ctx.wrapping_add(4));
    let params = mem_word(mem, ctx.wrapping_add(20));
    if params == 0 {
        return 0x81202007;
    }
    let o = hooks.alloc_object_m(1, 0x14c, mem);
    if o == 0 {
        return 0x8120200a;
    }
    mem_store(mem, out, o);
    mem_store(mem, out.wrapping_add(4), 0xcdac);
    for i in 0..5 {
        mem_store(
            mem,
            o.wrapping_add(0x14).wrapping_add(i * 4),
            mem_word(mem, opts.wrapping_add(i * 4)),
        );
    }
    mem_store(mem, o.wrapping_add(0x28), sink);
    mem[o as usize + 0x30] = 1;
    mem[o as usize + 0x69] = 3;
    mem_store(mem, o.wrapping_add(0x140), 0);
    mem_store(mem, o, c);
    mem_store(mem, o.wrapping_add(4), d);
    mem_store(mem, o.wrapping_add(8), o);
    mem_store(mem, o.wrapping_add(12), 0xcdac);
    mem_store(mem, o.wrapping_add(0x10), lic);
    let (rc, mutex) = hooks.make_mutex_m(mem_word(mem, ctx.wrapping_add(8)), heap, mem);
    if rc >= 0x80000000 {
        return fail_create_m(hooks, mem, ctx, out, rc);
    }
    mem_store(mem, o.wrapping_add(0x2c), mutex);
    let (rc, param_cb) = hooks.make_param_cb_m(heap, CREATE_CMP, CREATE_CPY, CREATE_REL, mem);
    if rc == 0 {
        return fail_create_m(hooks, mem, ctx, out, rc);
    }
    mem_store(mem, o.wrapping_add(0x3c), param_cb);
    let comp = mem_word(mem, ctx.wrapping_add(24));
    let (mut rc, mut q) = hooks.query_comp_m(comp, CREATE_SYNTHSTREAM, mem);
    if rc >= 0x80000000 {
        rc = hooks.register_comp_m(c, d, CREATE_SYNTHSTREAM, CREATE_SYNTHSTREAM_LO, param_cb, mem);
        if rc >= 0x80000000 {
            return fail_create_m(hooks, mem, ctx, out, rc);
        }
        (rc, q) = hooks.query_comp_m(comp, CREATE_SYNTHSTREAM, mem);
        if rc >= 0x80000000 {
            return fail_create_m(hooks, mem, ctx, out, rc);
        }
    }
    mem_store(mem, o.wrapping_add(0x40), mem_word(mem, q.wrapping_add(4)));
    let (rc, q) = hooks.query_comp_m(comp, CREATE_LINGDB, mem);
    let (mut rc, mut q) = (rc, q);
    if rc >= 0x80000000 {
        rc = hooks.register_comp_m(c, d, CREATE_LINGDB, CREATE_LINGDB_LO, param_cb, mem);
        if rc >= 0x80000000 {
            return fail_create_m(hooks, mem, ctx, out, rc);
        }
        (rc, q) = hooks.query_comp_m(comp, CREATE_LINGDB, mem);
        if rc >= 0x80000000 {
            return fail_create_m(hooks, mem, ctx, out, rc);
        }
    }
    mem_store(mem, o.wrapping_add(0x54), mem_word(mem, q.wrapping_add(4)));
    if mem_word(mem, lic.wrapping_add(16)) != 0 {
        let (rc, _) = hooks.query_comp_m(comp, CREATE_COMPQUERY, mem);
        if rc >= 0x80000000 {
            let mut buf = [0u8; 20];
            buf[0..4].copy_from_slice(&1u32.to_le_bytes());
            buf[4..8].copy_from_slice(&mem_word(mem, lic.wrapping_add(16)).to_le_bytes());
            buf[8..12].copy_from_slice(&mem_word(mem, lic.wrapping_add(20)).to_le_bytes());
            buf[12..16].copy_from_slice(&mem_word(mem, lic.wrapping_add(24)).to_le_bytes());
            let rc = hooks.insert_comp_m(comp, CREATE_COMPQUERY, &buf, mem);
            if rc >= 0x80000000 {
                return fail_create_m(hooks, mem, ctx, out, rc);
            }
            mem[o as usize + 0x58] = 1;
        } else {
            hooks.release_comp_m(comp, CREATE_COMPQUERY, mem);
        }
    }
    let (rc, _) = hooks.query_comp_m(comp, CREATE_TTSEG, mem);
    if rc >= 0x80000000 {
        let mut buf = [0u8; 20];
        buf[0..4].copy_from_slice(&3u32.to_le_bytes());
        buf[4..8].copy_from_slice(&CREATE_D_79D8.to_le_bytes());
        buf[8..12].copy_from_slice(&mem_word(mem, out).to_le_bytes());
        buf[12..16].copy_from_slice(&mem_word(mem, out.wrapping_add(4)).to_le_bytes());
        let rc = hooks.insert_comp_m(comp, CREATE_TTSEG, &buf, mem);
        if rc >= 0x80000000 {
            return fail_create_m(hooks, mem, ctx, out, rc);
        }
        mem[o as usize + 0x59] = 1;
    } else {
        hooks.release_comp_m(comp, CREATE_TTSEG, mem);
    }
    let (rc, q) = hooks.query_comp_m(comp, CREATE_INET, mem);
    if rc < 0x80000000 {
        mem_store(mem, o.wrapping_add(0x5c), mem_word(mem, q.wrapping_add(4)));
        mem_store(mem, o.wrapping_add(0x60), mem_word(mem, q.wrapping_add(8)));
        mem_store(mem, o.wrapping_add(0x64), mem_word(mem, q.wrapping_add(12)));
    }
    let mut param_reg = mem[o as usize + 0x68];
    if hooks.log_level_m(mem_word(mem, ctx.wrapping_add(16)), mem) > 3 {
        if hooks.reg_param_cb_m(params, None, o, mem) < 0x80000000 {
            param_reg = 1;
            mem[o as usize + 0x68] = 1;
        }
    }
    let defaults: [(&[u8], u32, u32); 13] = [
        (CREATE_PITCH, 0, 0x32),
        (CREATE_RATE, 0, 100),
        (CREATE_VOLUME, 0, 0x50),
        (CREATE_WAITFACTOR, 0, 2),
        (CREATE_READMODE, 0, 1),
        (CREATE_MARKERMODE, 0, 0),
        (CREATE_LOOPLIMIT, 0, 0x2710),
        (CREATE_INPUTMIME, 1, 0),
        (CREATE_AUDIOBUF, 0, 0x1000),
        (CREATE_LICMODE, 2, 0),
        (CREATE_NUANKEY, 1, 0),
        (CREATE_COMPANY, 1, 0),
        (CREATE_APPNAME, 1, 0),
    ];
    for (key, kind, num) in defaults {
        let (rc, pv) = hooks.query_param_m(params, key, mem);
        if rc < 0x80000000 && pv != 0 {
            hooks.touch_param_m(params, pv, mem);
        } else if kind == 0 {
            hooks.set_default_num_m(params, key, num, mem);
        } else if kind == 2 {
            hooks.set_default_str_m(params, key, CREATE_DEFAULT, mem);
        } else {
            hooks.set_default_str_m(params, key, CREATE_EMPTY, mem);
        }
    }
    let (rc, limit) = hooks.query_num_m(params, CREATE_LOOPLIMIT, mem);
    if rc >= 0x80000000 {
        return fail_create_m(hooks, mem, ctx, out, rc);
    }
    mem_store(mem, o.wrapping_add(0x38), limit);
    let (rc, mode) = hooks.query_num_m(params, CREATE_MARKERMODE, mem);
    if rc >= 0x80000000 {
        return fail_create_m(hooks, mem, ctx, out, rc);
    }
    mem_store(mem, o.wrapping_add(0x34), mode);
    if param_reg == 0 {
        for key in CREATE_CBKEYS {
            let rc = hooks.reg_param_cb_m(params, Some(key), o, mem);
            if rc < 0x80000000 {
                param_reg = 1;
                mem[o as usize + 0x68] = 1;
            } else if rc & 0x1fff != 0x14 {
                return fail_create_m(hooks, mem, ctx, out, rc);
            }
        }
    }
    let (rc, token) = hooks.lic_token_m(c, d, mem);
    if rc >= 0x80000000 {
        return fail_create_m(hooks, mem, ctx, out, rc);
    }
    mem_store(mem, o.wrapping_add(0x74), token);
    mem[o as usize + 0x70] = mem[lic as usize + 148];
    let mut buf = [0u8; 20];
    buf[0..4].copy_from_slice(&1u32.to_le_bytes());
    buf[4..8].copy_from_slice(&CREATE_D_7970.to_le_bytes());
    buf[8..12].copy_from_slice(&mem_word(mem, out).to_le_bytes());
    buf[12..16].copy_from_slice(&mem_word(mem, out.wrapping_add(4)).to_le_bytes());
    let rc = hooks.insert_comp_m(comp, CREATE_SINK, &buf, mem);
    if rc >= 0x80000000 {
        return fail_create_m(hooks, mem, ctx, out, rc);
    }
    mem_store(mem, o.wrapping_add(0x78), 1);
    let mut rc = rc;
    if mime != 0 && mem[mime as usize] != 0 {
        let mt = mime;
        let (rc2, trace, _) =
            hooks.trace_pipeline_m(mem_word(mem, o), mem_word(mem, o.wrapping_add(4)), mt, param_cb, mem);
        if rc2 >= 0x80000000 {
            return fail_create_m(hooks, mem, ctx, out, rc2);
        }
        mem_store(mem, o.wrapping_add(0x7c), trace);
        mem[o as usize + 0x69] = license_kind_m(hooks, mem, ctx, o, trace) as u8;
        rc = hooks.start_object_m(ctx, o, mem);
        if rc >= 0x80000000 {
            return fail_create_m(hooks, mem, ctx, out, rc);
        }
    }
    let (rc2, tmp) = hooks.query_param_m(params, CREATE_NUANKEY, mem);
    if rc2 < 0x80000000 {
        let status = install_esc_seq_m(hooks, mem, ctx, o, tmp);
        if status >= 0x80000000 {
            return fail_create_m(hooks, mem, ctx, out, status);
        }
        rc = status;
    }
    rc
}

/// Live-memory heap and tree callbacks. Legacy implementations get a blanket
/// adapter. New dispatchers implement the M trait directly, not both traits.
pub trait HeapVtableM {
    #[allow(clippy::too_many_arguments)]
    fn alloc_at_m(&mut self, obj: u32, size: u32, mem: &mut [u8]) -> u32;
    #[allow(clippy::too_many_arguments)]
    fn realloc_at_m(&mut self, obj: u32, old: u32, size: u32, mem: &mut [u8]) -> u32;
    #[allow(clippy::too_many_arguments)]
    fn release_at_m(&mut self, obj: u32, object: u32, mem: &mut [u8]) ;
}
impl<T: HeapVtable + ?Sized> HeapVtableM for T {
    fn alloc_at_m(&mut self, obj: u32, size: u32, mem: &mut [u8]) -> u32 { let _ = &mem; HeapVtable::alloc_at(self, obj, size) }
    fn realloc_at_m(&mut self, obj: u32, old: u32, size: u32, mem: &mut [u8]) -> u32 { let _ = &mem; HeapVtable::realloc_at(self, obj, old, size) }
    fn release_at_m(&mut self, obj: u32, object: u32, mem: &mut [u8])  { let _ = &mem; HeapVtable::release_at(self, obj, object) }
}
pub trait TreeOpsM {
    #[allow(clippy::too_many_arguments)]
    fn tree_cmp_m(&mut self, key: u32, node_key: u32, mem: &mut [u8]) -> i32;
    #[allow(clippy::too_many_arguments)]
    fn tree_init_m(&mut self, key: u32, heap: u32, node: u32, mem: &mut [u8]) -> (u32, Option<u32>);
    #[allow(clippy::too_many_arguments)]
    fn tree_alloc_m(&mut self, arg: u32, heap: u32, mem: &mut [u8]) -> (u32, u32);
    #[allow(clippy::too_many_arguments)]
    fn tree_free_node_m(&mut self, p: u32, heap: u32, mem: &mut [u8]) ;
    #[allow(clippy::too_many_arguments)]
    fn tree_free_handle_m(&mut self, p: u32, heap: u32, mem: &mut [u8]) ;
}
impl<T: TreeOps + ?Sized> TreeOpsM for T {
    fn tree_cmp_m(&mut self, key: u32, node_key: u32, mem: &mut [u8]) -> i32 { let _ = &mem; TreeOps::tree_cmp(self, key, node_key) }
    fn tree_init_m(&mut self, key: u32, heap: u32, node: u32, mem: &mut [u8]) -> (u32, Option<u32>) { let _ = &mem; TreeOps::tree_init(self, key, heap, node) }
    fn tree_alloc_m(&mut self, arg: u32, heap: u32, mem: &mut [u8]) -> (u32, u32) { let _ = &mem; TreeOps::tree_alloc(self, arg, heap) }
    fn tree_free_node_m(&mut self, p: u32, heap: u32, mem: &mut [u8])  { let _ = &mem; TreeOps::tree_free_node(self, p, heap) }
    fn tree_free_handle_m(&mut self, p: u32, heap: u32, mem: &mut [u8])  { let _ = &mem; TreeOps::tree_free_handle(self, p, heap) }
}

pub fn heap_alloc_m(mem: &mut [u8], heap: Option<u32>, size: u32, vt: &mut impl HeapVtableM) -> u32 {
    let Some(heap) = heap else {
        return 0;
    };
    if size == 0 {
        return 0;
    }
    vt.alloc_at_m(mem_word(mem, heap + 4), size, mem)
}

pub fn heap_realloc_m(
    mem: &mut [u8],
    heap: Option<u32>,
    old: u32,
    size: u32,
    vt: &mut impl HeapVtableM,
) -> u32 {
    let Some(heap) = heap else {
        return 0;
    };
    if size == 0 {
        return 0;
    }
    let obj = mem_word(mem, heap + 4);
    if old != 0 {
        vt.realloc_at_m(obj, old, size, mem)
    } else {
        vt.alloc_at_m(obj, size, mem)
    }
}

pub fn heap_release_m(mem: &mut [u8], heap: Option<u32>, object: Option<u32>, vt: &mut impl HeapVtableM) {
    let Some(heap) = heap else {
        return;
    };
    let Some(object) = object else {
        return;
    };
    vt.release_at_m(mem_word(mem, heap + 4), object, mem);
}

pub fn tree_insert_m(mem: &mut [u8], tree: u32, node: u32, key: u32, ops: &mut impl TreeOpsM) -> u32 {
    let mut p = tree;
    let mut x = mem_word(mem, tree);
    let mut cmp = -1i32;
    while x != tree {
        p = x;
        cmp = ops.tree_cmp_m(key, mem_word(mem, x + 16), mem);
        if mem[(tree + 40) as usize] == 0 && cmp == 0 {
            return 0x89002002;
        }
        x = if cmp < 0 {
            mem_word(mem, x)
        } else {
            mem_word(mem, x + 4)
        };
    }
    let (rc, out_key) = ops.tree_init_m(key, mem_word(mem, tree + 32), node, mem);
    if let Some(out_key) = out_key {
        mem_store(mem, node + 16, out_key);
    }
    if rc >= 0x80000000 {
        return rc;
    }
    if cmp < 0 {
        mem_store(mem, p, node);
    } else {
        mem_store(mem, p + 4, node);
    }
    mem_store(mem, node, tree);
    mem_store(mem, node + 4, tree);
    mem_store(mem, node + 8, p);
    mem_store(mem, tree + 24, mem_word(mem, tree + 24).wrapping_add(1));
    mem[(node + 12) as usize] = 0;
    rebalance(mem, tree, node, p);
    rc
}

pub fn tree_alloc_insert_m(
    mem: &mut [u8],
    s: u32,
    a: u32,
    b: u32,
    heap_vt: &mut impl HeapVtableM,
    ops: &mut impl TreeOpsM,
) -> u32 {
    let heap = mem_word(mem, s + 32);
    let n = heap_alloc_m(mem, nz(heap), 0x18, heap_vt);
    if n == 0 {
        return 0x8900200a;
    }
    let mut tmp = 0u32;
    if b != 0 {
        let (r, out) = ops.tree_alloc_m(b, heap, mem);
        if r >= 0x80000000 {
            heap_release_m(mem, nz(heap), nz(n), heap_vt);
            return r;
        }
        tmp = out;
    }
    mem_store(mem, n + 16, 0);
    mem_store(mem, n + 8, 0);
    mem_store(mem, n, 0);
    mem_store(mem, n + 4, 0);
    mem[n as usize + 12] = 0;
    mem_store(mem, n + 20, tmp);
    let r = tree_insert_m(mem, s, n, a, ops);
    if r < 0x80000000 {
        return 0;
    }
    if mem_word(mem, n + 16) != 0 {
        ops.tree_free_node_m(mem_word(mem, n + 16), heap, mem);
    }
    if mem_word(mem, n + 20) != 0 {
        ops.tree_free_handle_m(tmp, heap, mem);
    }
    heap_release_m(mem, nz(heap), nz(n), heap_vt);
    r
}

pub fn insert_guarded_m(
    mem: &mut [u8],
    base: Option<u32>,
    key: u32,
    value: u32,
    heap_vt: &mut impl HeapVtableM,
    ops: &mut impl TreeOpsM,
) -> u32 {
    let Some(base) = base else {
        return 0x89002007;
    };
    if key == 0 || value == 0 {
        return 0x89002007;
    }
    tree_alloc_insert_m(mem, base.wrapping_add(24), key, value, heap_vt, ops)
}
