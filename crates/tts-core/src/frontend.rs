use core::cell::RefCell;
use core::cmp::Ordering;

#[derive(Clone, Copy)]
pub struct StreamKey<'a> {
    pub content_type: &'a [u8],
    pub version: u16,
}

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

pub fn initialize_version(output: Option<&mut [u8; 3]>) -> u32 {
    match output {
        Some(output) => {
            *output = [5, 6, 1];
            0
        }
        None => 0x80000006,
    }
}

pub fn initialize_build(output: Option<&mut [u16; 3]>) -> u32 {
    match output {
        Some(output) => {
            *output = [0x7dc, 0x050c, 0];
            0
        }
        None => 0x80000006,
    }
}

pub fn unsupported_interface(object_tag: u32, tag: u32, output: &mut u32) -> u32 {
    if tag != 0x68434c53 || object_tag != 0x73434c53 {
        return 0x80000008;
    }
    *output = 0;
    0x80000001
}

pub fn stream_element_callbacks(callbacks: &[u8]) -> &[u8] {
    callbacks
}

pub fn finish_component_query() -> u32 {
    0
}

pub fn finish_request() -> u32 {
    0
}

pub fn has_utf16_bom(text: &[u8; 2]) -> u32 {
    u32::from(matches!(text, [0xff, 0xfe] | [0xfe, 0xff]))
}

pub fn streaming_mode(state: Option<(u8, u8)>) -> u32 {
    state.map_or(
        0,
        |(enabled, value)| if enabled != 0 { value as u32 } else { 1 },
    )
}

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

pub fn enter_request(
    state: &mut u8,
    next: u8,
    previous: &mut u8,
    lock: &mut impl StateLock,
) -> u32 {
    enter_state(state, next, previous, lock, 0x80002011)
}

pub fn enter_pipeline(
    state: &mut u8,
    next: u8,
    previous: &mut u8,
    lock: &mut impl StateLock,
) -> u32 {
    enter_state(state, next, previous, lock, 0x81202011)
}

pub fn restore_pipeline(state: &mut u8, previous: u32, lock: &mut impl StateLock) {
    restore_state(state, previous, lock);
}

pub fn restore_request(state: &mut u8, previous: u32, lock: &mut impl StateLock) {
    restore_state(state, previous, lock);
}

pub fn notify(arguments: [u32; 4], mut callback: impl FnMut([u32; 4]) -> u32) -> u32 {
    callback(arguments);
    0
}

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

use crate::frontend_types::Error;

pub fn validate_handle(present: bool, tag: u32, expected: u32, size: u32) -> u32 {
    if tag == expected && (size == 0 || present) {
        0
    } else {
        0x81706008
    }
}

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

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct RewriterHandle {
    pub first: u32,
    pub second: u32,
}

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

pub trait RewriterBuffer {
    fn append(&mut self, output: RewriterHandle, data: &[u8]) -> u32;
    fn release(&mut self, output: RewriterHandle) -> u32;
}

pub trait RewriterPositions {
    fn get(&mut self, handle: RewriterHandle, out: &mut i32, which: u32) -> u32;
    fn set(&mut self, handle: RewriterHandle, value: i32, which: u32) -> u32;
}

pub trait RewriterEngine: RewriterText + RewriterBuffer + RewriterPositions {}

impl<T: RewriterText + RewriterBuffer + RewriterPositions> RewriterEngine for T {}

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

pub fn load_pair(out: &mut [u8; 8], value: &[u8; 8]) {
    out.copy_from_slice(value);
}

pub fn check_handle(present: bool) -> u32 {
    if present {
        0
    } else {
        0x8b702007
    }
}

pub fn license_counter(counter: u32) -> u32 {
    counter
}

pub fn acquire_license(counter: &mut u32, feature: u32) -> u32 {
    *counter = counter.wrapping_add(1);
    feature.wrapping_mul(100000)
}

pub fn release_license(counter: &mut u32) -> u32 {
    if *counter != 0 {
        *counter -= 1;
    }
    0
}

pub fn char_class(table: &[u16; 256], c: u32) -> u32 {
    u32::from(table[(c & 0xff) as usize] & 3)
}

pub fn ascii_fold(table: &[u16; 256], c: u32) -> u32 {
    if table[(c & 0xff) as usize] & 1 != 0 {
        c.wrapping_add(0x20)
    } else {
        c
    }
}

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

pub struct ElementArray<'a> {
    pub data: &'a mut [u8],
    pub elem_size: u16,
    pub count: u32,
}

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

pub fn word_at5(input: Option<&[u32; 6]>) -> u32 {
    match input {
        Some(words) => words[5],
        None => 0,
    }
}

pub struct ElementTable<'a> {
    pub data: &'a [u8],
    pub elem_size: u16,
    pub count: u32,
}

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

pub fn copy_bytes(dst: &mut [u8], src: &[u8], n: u32) {
    let n = n as usize;
    for i in 0..n {
        dst[i] = src[i];
    }
}

pub struct GrowBlock {
    pub data: u32,
    pub elem_size: u16,
    pub callback: u32,
    pub cap: u32,
    pub count: u32,
    pub grow: u16,
    pub heap: u32,
}

pub trait BlockHeap {
    fn alloc_block(&mut self, heap: u32, size: u32) -> u32;
    fn realloc_block(&mut self, heap: u32, data: u32, size: u32) -> u32;
    fn alloc_cells(&mut self, heap: u32, count: u32, size: u32) -> u32;
}

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

pub struct RegisterParams {
    pub heap: u32,
    pub values: Option<u32>,
    pub extra: Option<u32>,
}

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

pub fn strlen_bytes(s: &[u8]) -> u32 {
    let mut len = 0u32;
    while s[len as usize] != 0 {
        len = len.wrapping_add(1);
    }
    len
}

pub trait HeapVtable {
    fn alloc_at(&mut self, obj: u32, size: u32) -> u32;
    fn realloc_at(&mut self, obj: u32, old: u32, size: u32) -> u32;
    fn release_at(&mut self, obj: u32, object: u32);
}

pub fn heap_alloc(mem: &[u8], heap: Option<u32>, size: u32, vt: &mut impl HeapVtable) -> u32 {
    let Some(heap) = heap else {
        return 0;
    };
    if size == 0 {
        return 0;
    }
    vt.alloc_at(mem_word(mem, heap + 4), size)
}

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

pub fn heap_release(mem: &[u8], heap: Option<u32>, object: Option<u32>, vt: &mut impl HeapVtable) {
    let Some(heap) = heap else {
        return;
    };
    let Some(object) = object else {
        return;
    };
    vt.release_at(mem_word(mem, heap + 4), object);
}

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

pub trait ElementCallback {
    fn call_element(&mut self, a: u32, q: u32, r: u32) -> u32;
}

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

pub fn char_is_digit(table: &[u16; 256], c: u32) -> u32 {
    (table[(c & 0xff) as usize] & 4) as u32
}

pub fn char_is_space(table: &[u16; 256], c: u32) -> u16 {
    table[(c & 0xff) as usize] & 8
}

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

pub fn release_owner(mem: &[u8], p: Option<u32>, vt: &mut impl HeapVtable) -> u32 {
    let Some(p) = p else {
        return 0x89002007;
    };
    let vtable = mem_word(mem, p);
    heap_release(mem, nz(mem_word(mem, vtable + 20)), Some(p), vt);
    0
}

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

pub trait MethodTable {
    fn call_at(&mut self, obj: u32, a: u32, b: u32) -> u32;
}

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

pub fn release_tag(registry: Option<u32>, tag: Option<u32>) -> u32 {
    if registry.is_none() || tag.is_none() {
        return 0x81506007;
    }
    0
}

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

pub fn match_halves(al: u32, ah: u32, bl: u32, bh: u32) -> u32 {
    u32::from(ah == bh && al == bl)
}

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

pub fn set_link_flag(mem: &mut [u8], p: u32) {
    mem[p as usize + 0x28] = 1;
}

pub fn deref_slot2(mem: &[u8], p: Option<u32>) -> u32 {
    p.map_or(0, |p| mem_word(mem, p + 8))
}

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

pub fn find_byte(mem: &[u8], s: u32, ch: u32) -> u32 {
    let c = ch as u8;
    let mut p = s as usize;
    while mem[p] != 0 && mem[p] != c {
        p += 1;
    }
    if mem[p] == c { p as u32 } else { 0 }
}

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

pub(crate) fn terminated(mem: &[u8], s: u32) -> &[u8] {
    let at = s as usize;
    let end = at + mem[at..].iter().position(|&c| c == 0).unwrap() + 1;
    &mem[at..end]
}

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

pub(crate) fn copy_str_or_empty(mem: &mut [u8], dst: u32, src: u32) {
    if src == 0 {
        mem[dst as usize] = 0;
    } else {
        copy_str(mem, dst, src);
    }
}

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

fn nul_len(s: &[u8]) -> usize {
    let mut n = 0;
    while n < s.len() && s[n] != 0 {
        n += 1;
    }
    n
}

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

pub fn clear_iterator(p: Option<u32>, clear: &mut impl FnMut(u32)) -> u32 {
    let Some(p) = p else {
        return 0x89002007;
    };
    clear(p + 24);
    0
}

pub fn release_self(mem: &[u8], p: u32, vt: &mut impl HeapVtable) -> u32 {
    heap_release(mem, nz(mem_word(mem, p + 4)), Some(p), vt);
    0
}

pub fn free_node(mem: &[u8], node: Option<u32>, vt: &mut impl HeapVtable) -> u32 {
    let Some(node) = node else {
        return 0x80b02007;
    };
    vt.release_at(mem_word(mem, node + 4), node);
    0
}

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

pub fn begin_stage_hook(handle: Option<u32>) -> u32 {
    if handle.is_some() {
        0
    } else {
        0x88402007
    }
}

pub fn end_stage_hook(handle: Option<u32>) -> u32 {
    if handle.is_some() {
        0
    } else {
        0x88402007
    }
}

pub fn u16_len(mem: &[u8], s: u32) -> u32 {
    let mut length = 0u32;
    while mem_half(mem, s.wrapping_add(length.wrapping_mul(2))) != 0 {
        length = length.wrapping_add(1);
    }
    length
}

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

const STRING_KINDS: [u16; 12] = [6, 12, 21, 50, 36, 39, 37, 34, 22, 40, 26, 29];

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

pub fn divmod_u32(v: u32, b: u32) -> (u32, u32) {
    (v / b, v % b)
}

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

pub trait VoiceNodeAllocator: HeapVtable {
    fn alloc_items_at(&mut self, ctx: u32, count: u32, width: u32) -> u32;
}

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

pub struct OpenScripts {

    pub rc794: u32,

    pub rc68c: u32,

    pub rc_db0: u32,

    pub rc_611c: u32,

    pub ctx_611c: u32,
}

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

pub struct InsertScripts {

    pub lock: u32,

    pub find: u32,

    pub found: Option<u32>,

    pub insert: u32,

    pub unlock: u32,
}

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

pub struct ReleaseScripts {

    pub lock: u32,

    pub find: u32,

    pub found: Option<u32>,

    pub remove: u32,

    pub unlock: u32,
}

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

pub struct CreateScripts {

    pub acquire: u32,

    pub ctx: Option<u32>,

    pub alloc: Option<u32>,

    pub owner_rc: u32,

    pub owner: u32,

    pub create: u32,

    pub map: u32,
}

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

pub struct TeardownScripts {

    pub lock: u32,

    pub unlock: u32,

    pub acquire: u32,

    pub ctx: Option<u32>,

    pub lingdb: u32,

    pub worker: u32,

    pub log: u32,

    pub phonmap: u32,

    pub synthstream: u32,

    pub teardown: u32,

    pub params: u32,

    pub synthfin: u32,

    pub mutex: u32,

    pub engine: u32,
}

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

pub struct OpenWorkerScripts {

    pub task_rc: u32,

    pub stream_rc: u32,

    pub mutex_rc: u32,

    pub mutex: u32,

    pub engine_rc: u32,

    pub engine: [u32; 2],

    pub acquire: u32,

    pub ctx: Option<u32>,

    pub log: u32,

    pub params_rc: u32,

    pub params: u32,

    pub ling_rc: u32,

    pub ling: u32,

    pub prop: u32,

    pub name: u32,

    pub mrk: u32,

    pub insert: u32,

    pub setup_rc: u32,

    pub voice: [u32; 2],

    pub worker_rc: u32,

    pub worker: u32,

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

pub const CREATE_CPR_Z: &[u8] = b"cpr\0";
pub const CREATE_USELECT_Z: &[u8] = b"uselect/\0";
pub const CREATE_SELECT_Z: &[u8] = b"select/\0";
pub const CREATE_SELCPR_Z: &[u8] = b"select/cpr\0";

pub const CREATE_CBKEYS: [&[u8]; 7] = [
    CREATE_LOOPLIMIT,
    CREATE_MARKERMODE,
    CREATE_READMODE,
    CREATE_WAITFACTOR,
    CREATE_AUDIOBUF,
    CREATE_NUANKEY,
    CREATE_COMPANY,
];

pub const CREATE_TXMKEY: &[u8] = b"txm ypy hwb &53(1@+- _={[]?/>.<,";
pub const CREATE_TXMWANT: &[u8] = b"4:\x014CS5]@AJGW^CL\x1e\"\x168&9\x17\x0b\x18\x11\x0e\x1a\x04\x184\0";

pub const CREATE_ESCHEAD: &[u8] = b"[header]\nlanguage = *\n[data]\n/";

pub const CREATE_ESCTAIL: &[u8] = b"/ --> \"\\x1b\" \n";

pub const CREATE_VOICEMIME: &[u8] = b"application/x-vocalizer-rettt+text";

pub const CREATE_CMP: u32 = 0x4;
pub const CREATE_CPY: u32 = 0x151774;
pub const CREATE_REL: u32 = 0x1517cc;

pub const CREATE_D_79D8: u32 = 0x8d79d8;
pub const CREATE_D_7970: u32 = 0x8d7970;

fn cstrlen(s: &[u8]) -> u32 {
    s.iter().position(|&c| c == 0).unwrap() as u32
}

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

pub trait CreateHooks {

    fn check_open(&mut self, obj: u32, b: u32) -> u32;

    fn resolve_ctx(&mut self, a: u32, b: u32) -> (u32, u32);

    fn alloc_object(&mut self, n: u32, size: u32) -> u32;

    fn make_mutex(&mut self, pad8: u32, heap: u32) -> (u32, u32);

    fn make_param_cb(&mut self, heap: u32, cmp: u32, cpy: u32, rel: u32) -> (u32, u32);

    fn query_comp(&mut self, comp: u32, name: &[u8]) -> (u32, u32);

    fn register_comp(&mut self, c: u32, d: u32, name: &[u8], lname: &[u8], param_cb: u32) -> u32;

    fn release_comp(&mut self, comp: u32, name: &[u8]);

    fn insert_comp(&mut self, comp: u32, name: &[u8], buf: &[u8; 20]) -> u32;

    fn log_level(&mut self, log: u32) -> u32;

    fn reg_param_cb(&mut self, params: u32, key: Option<&[u8]>, this: u32) -> u32;

    fn query_param(&mut self, params: u32, key: &[u8]) -> (u32, u32);

    fn touch_param(&mut self, params: u32, value: &[u8]);

    fn set_default_num(&mut self, params: u32, key: &[u8], value: u32);

    fn set_default_str(&mut self, params: u32, key: &[u8], value: &[u8]);

    fn query_num(&mut self, params: u32, key: &[u8]) -> (u32, u32);

    fn lic_token(&mut self, c: u32, d: u32) -> (u32, u32);

    fn trace_pipeline(&mut self, ha: u32, hb: u32, mime: &[u8], param_cb: u32) -> (u32, u32, u8);

    fn start_object(&mut self, ctx: u32, obj: u32) -> u32;

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

    fn null_pair(&mut self) -> (u32, u32);

    fn alloc_temp(&mut self, heap: u32, size: u32) -> u32;

    fn free_temp(&mut self, heap: u32, ptr: u32);

    fn teardown(&mut self, ctx: u32);

    fn release_voice(&mut self, h: (u32, u32), ptr: u32, n: u32);
}

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

fn fail_create<H: CreateHooks>(hooks: &mut H, mem: &mut [u8], ctx: u32, out: u32, rc: u32) -> u32 {
    hooks.teardown(ctx);
    mem_store(mem, out, 0);
    mem_store(mem, out.wrapping_add(4), 0);
    rc
}

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

pub const PIPE_KIND_MAP: [u8; 4] = [0, 3, 0, 6];

pub const PIPE_ANALYSIS_KEY: &[u8] = b"textanalysison";

pub const PIPE_YES: &[u8] = b"yes";
pub const PIPE_NO: &[u8] = b"no";

pub const PIPE_GUARD_NAME: &[u8] = b"ttseg_PipelineSet";

pub trait PipeHooks {

    fn check_open(&mut self, obj: u32, b: u32) -> u32;

    fn resolve_ctx(&mut self, a: u32, b: u32) -> (u32, u32);

    fn set_textanalysis(&mut self, params: u32, on: bool);

    fn lock(&mut self, mutex: u32) -> u32;

    fn unlock(&mut self, mutex: u32) -> u32;

    fn query_pending(&mut self, params: u32) -> (u32, u32);

    fn trace_pipe(
        &mut self,
        ha: u32,
        hb: u32,
        name: &[u8],
        cs: u32,
        stream: u32,
    ) -> (u32, u32, u8);

    fn query_param(&mut self, params: u32, key: &[u8]) -> (u32, u32);

    fn start_object(&mut self, ctx: u32, obj: u32) -> u32;

    fn lic_count(&mut self, feat: u32) -> u32;

    fn lic_feat(&mut self, feat: u32) -> u32;

    fn release_token(&mut self, token: u32);

    fn notify_lic(&mut self, ctx: u32, channel: u32, kind: u32, mode: u32);

    fn release_pipe(&mut self, pipe: u32) -> u32;

    fn rollback_pending(&mut self, params: u32, pending: u32) -> u32;

    fn release_pending(&mut self, params: u32, pending: u32) -> u32;

    fn reactivate(&mut self, cs: u32);
}

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

pub trait LicenseOpHooks {

    fn open_handle(&mut self, a: u32, b: u32, ctx: &mut u32, lic: &mut u32) -> u32;
}

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

pub const BIND_BUF_LEN: usize = 1028;

pub trait BindHooks {

    fn begin_stage(&mut self, obj: u32, arg: u32);

    fn end_stage(&mut self, obj: u32, arg: u32);

    fn get_input(&mut self, b: u32, size: &mut u32, buf: &mut [u8; BIND_BUF_LEN]) -> u32;

    fn drop_pair(&mut self, a: u32, b: u32);

    fn push_input(&mut self, a: u32, b: u32, buf: &[u8; BIND_BUF_LEN]) -> u32;

    fn null_pair(&mut self) -> (u32, u32);
}

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

pub const MARKER_LEN: usize = 24;

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

pub fn resume_stage_hook(handle: Option<u32>) -> u32 {
    if handle.is_some() {
        0
    } else {
        0x88402007
    }
}

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

pub trait FlushHooks {

    fn open_iter(&mut self) -> (u32, u32);

    fn next_entry(&mut self) -> Option<(u32, u32, u32)>;

    fn close_iter(&mut self);

    fn write_param(&mut self, key: u32, data: u32, size: u32, remove: bool) -> u32;

    fn lock(&mut self) -> u32;

    fn unlock(&mut self) -> u32;

    fn resolve_for_remove(&mut self, key: u32) -> (u32, Option<(u32, u32)>);

    fn drop_value(&mut self, key: u32) -> u32;
}

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

pub fn release_broker_interface(reg: Option<u32>, comp: Option<u32>) -> u32 {
    if reg.is_some() && comp.is_some() {
        0
    } else {
        0x81506007
    }
}

pub fn begin_trace_hook(handle: Option<u32>) -> u32 {
    if handle.is_some() {
        0
    } else {
        0x88402007
    }
}

pub fn end_trace_hook(handle: Option<u32>, log: Option<u32>) -> u32 {
    if handle.is_some() && log.is_some() {
        0
    } else {
        0x88402007
    }
}

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

pub struct AcquireScripts {

    pub lock: u32,

    pub find: u32,

    pub found: Option<u32>,

    pub unlock: u32,
}

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

pub trait TeardownHooks {

    fn trace(&mut self, _which: u32) {}

    fn trace_comp(&mut self, _index: u32, _closing: bool) {}

    fn close_comp(&mut self, h0: u32, h1: u32) -> u32;

    fn release_broker(&mut self) -> u32;

    fn release_trace(&mut self) {}

    fn free_block(&mut self, heap: u32, block: u32);
}

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

pub trait ReopenHooks {

    fn write_named(&mut self, tag: &[u8], val: &[u8]);

    fn open_tags(&mut self) -> (u32, bool);

    fn next_tag(&mut self) -> Option<(u32, Option<u32>)>;

    fn close_tags(&mut self);

    fn acquire_entry(&mut self) -> (u32, Option<(u32, u32, u32)>);

    fn reopen_entry(&mut self, h0: u32, h1: u32) -> u32;

    fn release_entry(&mut self, tag: u32);

    fn unload_comp(&mut self, h0: u32, h1: u32) -> u32;
}

pub fn reopen_pipeline<H: ReopenHooks>(
    mem: &[u8],
    cs: u32,
    filter: &[u8],
    hooks: &mut H,
) -> u32 {
    let mut rc = 0u32;
    let count = mem_half(mem, cs.wrapping_add(12));
    let name = mem_word(mem, cs.wrapping_add(4));

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

        if mem_word(mem, vt.wrapping_add(0x18)) != 0 {
            let h0 = mem_word(mem, c.wrapping_add(12));
            let h1 = mem_word(mem, c.wrapping_add(16));
            let r2 = hooks.unload_comp(h0, h1);

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

pub trait EmitHooks {

    fn put_output(
        &mut self,
        user: u32,
        msg: u32,
        len_in: u32,
        n_in: u32,
        markers: &[u8],
    ) -> (u32, Option<u32>, u32, u32);

    fn get_avail(&mut self, a: u32, b: u32) -> (u32, u32, u8);

    fn get_buf(&mut self, a: u32, b: u32) -> (u32, Option<u32>, u32);

    fn get_pos(&mut self, a: u32, b: u32) -> (u32, u32);

    fn consume(&mut self, a: u32, b: u32, n: u32) -> u32;

    fn realloc_markers(&mut self) -> Option<u32>;
}

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

pub trait SynthTeardownHooks {

    fn close_stream(&mut self, a: u32, b: u32) -> u32;

    fn process_end(&mut self, h0: u32, h1: u32) -> u32;

    fn release_stream(&mut self, a: u32, b: u32) -> u32;

    fn null_pair(&mut self) -> (u32, u32);

    fn lex_close(&mut self, params: u32, lex: u32) -> u32;

    fn lex_free(&mut self, params: u32, lex: u32) -> u32;

    fn retire_pipeline(&mut self, cs: u32) -> u32;

    fn reopen_pipeline_cs(&mut self, cs: u32) -> u32;
}

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

pub const INSERT_EVENT_LEN: u32 = 24;

pub trait InserterPumpHooks {

    fn buf_available(&mut self, h0: u32, h1: u32, count: &mut u32, end: &mut u8) -> u32;

    fn buf_read(&mut self, h0: u32, h1: u32, data: &mut u32, count: &mut u32) -> u32;

    fn buf_write(&mut self, h0: u32, h1: u32, data: &mut u32, count: u32) -> u32;

    fn buf_consume(&mut self, h0: u32, h1: u32, n: u32) -> u32;

    fn buf_close(&mut self, h0: u32, h1: u32) -> u32;

    fn buf_copy(
        &mut self,
        d0: u32,
        d1: u32,
        s0: u32,
        s1: u32,
        n: u32,
        moved: &mut u32,
    ) -> u32;

    fn buf_position(&mut self, h0: u32, h1: u32, pos: &mut u32) -> u32;

    fn buf_commit(&mut self, h0: u32, h1: u32, n: u32) -> u32;

    fn ins_resolve(
        &mut self,
        s0: u32,
        s1: u32,
        uri: u32,
        n0: u32,
        n1: u32,
        out: &mut u32,
    ) -> u32;

    fn ins_open(
        &mut self,
        s0: u32,
        s1: u32,
        name: u32,
        pos: &mut u32,
        rem: &mut u32,
        ins: &mut (u32, u32),
    ) -> u32;

    fn ins_close(&mut self, h0: u32, h1: u32, flag: u32) -> u32;

    fn pump_remote(&mut self, mem: &mut [u8], base: u32) -> u32;

    fn null_pair(&mut self) -> (u32, u32);
}

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

pub trait TextStateHooks {

    fn resolve_context(&mut self, h0: u32, h1: u32, ctx: &mut u32) -> u32;

    fn read_count(&mut self, src: u32) -> u32;
}

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

pub trait ItemsDtorHooks {

    fn destroy_elem(&mut self, elem: u32, ctx: u32);
}

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

pub trait TextResetHooks: ItemsDtorHooks {

    fn resolve_context(&mut self, h0: u32, h1: u32, ctx: &mut u32) -> u32;
}

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

pub trait SwitchHooks {

    fn read_switch(&mut self, obj: u32, val: &mut u32) -> u32;

    fn close_pipes(&mut self, arg: u32) -> u32;
}

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

pub trait PromptReleaseHooks {

    fn release_stream(&mut self, h0: u32, h1: u32) -> u32;
}

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

pub trait SlotReleaseHooks {

    fn release_slot(&mut self, obj: u32, arg: u32) -> u32;
}

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

pub trait StreamCloseHooks {

    fn close_stream(&mut self, h0: u32, h1: u32) -> u32;

    fn null_pair(&mut self) -> (u32, u32);
}

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

pub trait VMethodHooks {

    fn call_method(&mut self, c: u32, d: u32) -> u32;
}

pub fn call_method7<H: VMethodHooks>(hooks: &mut H, mem: &[u8], base: u32, q: u32) -> u32 {
    if validate_handle(true, q, 0xf383, 0x20) >= 0x80000000 {
        return 0x8bb02008;
    }
    hooks.call_method(
        mem_word(mem, base.wrapping_add(12)),
        mem_word(mem, base.wrapping_add(16)),
    )
}

pub fn call_method8<H: VMethodHooks>(hooks: &mut H, mem: &[u8], base: u32, q: u32) -> u32 {
    if validate_handle(true, q, 0xf383, 0x20) >= 0x80000000 {
        return 0x8bb02008;
    }
    hooks.call_method(
        mem_word(mem, base.wrapping_add(12)),
        mem_word(mem, base.wrapping_add(16)),
    )
}

pub trait AllocRecHooks {

    fn alloc_word(&mut self, heap: u32, n: u32, size: u32) -> Option<u32>;
}

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

pub trait CtxResetHooks {

    fn release_pair(&mut self, a: u32, b: u32, flag: u32) -> u32;

    fn free_pair(&mut self, a: u32, b: u32) -> u32;

    fn detach_pair(&mut self, a: u32, b: u32) -> u32;

    fn null_pair(&mut self) -> (u32, u32);
}

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

pub trait TextOpenHooks {

    fn resolve_env(&mut self, h0: u32, h1: u32, env: &mut u32) -> u32;

    fn mime_type(&mut self, props: u32) -> (u32, u32);

    fn prefix_off(&mut self, utf8: bool) -> u32;

    fn stream_create(&mut self, h0: u32, h1: u32, out: &mut (u32, u32)) -> u32;

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

    fn stream_buffer(&mut self, h0: u32, h1: u32, data: &mut u32, count: u32) -> u32;

    fn stream_commit(&mut self, h0: u32, h1: u32, n: u32) -> u32;

    fn stream_close(&mut self, h0: u32, h1: u32) -> u32;

    fn stream_destroy(&mut self, h0: u32, h1: u32) -> u32;

    fn null_pair(&mut self) -> (u32, u32);
}

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

pub trait LdResetHooks {

    fn log_begin(&mut self, vt4: u32);

    fn log_error(&mut self, vt4: u32, code: u32);

    fn log_end(&mut self, vt4: u32, r: u32);

    fn alloc_buf(&mut self, vt1: u32, n: u32) -> u32;

    fn reset_pool(&mut self, mem: &mut [u8], at: u32);

    fn load_reset(&mut self, x: &mut u32) -> u32;
}

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

pub trait DualStreamHooks {

    fn name_off(&mut self, i: u32) -> u32;

    fn stream_create(
        &mut self,
        h0: u32,
        h1: u32,
        name: u32,
        out: &mut (u32, u32),
    ) -> u32;

    fn stream_release(&mut self, h0: u32, h1: u32);

    fn null_pair(&mut self) -> (u32, u32);
}

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

pub trait PromptStreamsHooks {

    fn name_off(&mut self, table: u32, i: u32) -> u32;

    fn stream_create(
        &mut self,
        h0: u32,
        h1: u32,
        name: u32,
        x: u32,
        y: u32,
        out: &mut (u32, u32),
    ) -> u32;

    fn query_mode(&mut self) -> u32;

    fn stream_release(&mut self, h0: u32, h1: u32);
}

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

pub trait TripleStreamsHooks {

    fn name_off(&mut self, i: u32) -> u32;

    fn stream_create(
        &mut self,
        h0: u32,
        h1: u32,
        name: u32,
        x: u32,
        out: &mut (u32, u32),
    ) -> u32;

    fn query_value(&mut self) -> u32;

    fn stream_destroy(&mut self, h0: u32, h1: u32);
}

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

pub trait VoiceQueryHooks {

    fn log_begin(&mut self, logger: u32);

    fn log_end(&mut self, logger: u32, rc: u32);

    fn match_voice(&mut self, hdr: &[u8; 32], vrec: &mut [u8; 296], flags: u32) -> u32;
}

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

pub trait AudioOpenHooks {

    fn stream_open(
        &mut self,
        h0: u32,
        h1: u32,
        name: &[u8],
        x: u32,
        y: u32,
        out: &mut (u32, u32),
    ) -> u32;

    fn query_marker_mode(&mut self) -> u32;

    fn audio_open_failed(&mut self, logger: u32);

    fn content_open_failed(&mut self, logger: u32, code: u32);
}

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

pub trait SegmentScoreHooks {

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

    fn refresh_object(&mut self) -> u32;

    fn resolve_count(&mut self, handle: u32, flag: u32, n: &mut u32);

    fn combine_outputs(&mut self, o1: u32, o2: u32) -> u32;
}

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

pub trait SynthesizeHooks {

    fn resolve_engine(&mut self, a: u32, b: u32, ctx: &mut u32, seg: &mut u32) -> u32;

    fn log_begin(&mut self, log: u32);

    fn log_error(&mut self, log: u32, code: u32);

    fn log_end(&mut self, log: u32, r: u32);

    fn begin_synth(&mut self, state: &mut u8) -> u32;

    fn end_synth(&mut self, state: u8);

    fn teardown_handle(&mut self, h0: u32, h1: u32);

    fn reset_handle(&mut self, h0: u32, h1: u32);

    fn release_handle(&mut self, h0: u32, h1: u32);

    fn report_synthesis(&mut self, h0: u32, h1: u32, log: u32, c: u32, x: u32, y: u32);

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

pub trait DbOpenHooks {

    fn resolve_env(&mut self, p: Option<u32>, a: u32, env: &mut u32) -> u32;

    fn log_begin(&mut self, log: u32);

    fn log_error(&mut self, log: u32, code: u32);

    fn log_end(&mut self, log: u32, r: u32);

    fn alloc_obj(&mut self) -> u32;

    fn query_param(&mut self, i: u32, out: &mut u32) -> u32;

    fn init_pool(&mut self, mem: &mut [u8], at: u32, ctx: u32, initial: u32, grow: u32);

    fn reset_db(&mut self, obj: u32) -> u32;

    fn close_db(&mut self, h0: u32, h1: u32);
}

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

pub trait DbClearHooks {

    fn log_begin(&mut self, log: u32);

    fn log_end(&mut self, log: u32, r: u32);

    fn reset_pool(&mut self, mem: &mut [u8], at: u32);
}

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

pub trait DbCloseHooks {

    fn log_open_begin(&mut self, log: u32);

    fn log_open_end(&mut self, log: u32, r: u32);

    fn log_clear_begin(&mut self, log: u32);

    fn log_clear_end(&mut self, log: u32, r: u32);

    fn free_pool(&mut self, mem: &mut [u8], at: u32);

    fn reset_pool(&mut self, mem: &mut [u8], at: u32);

    fn free_obj(&mut self, heap: u32, obj: u32);
}

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

pub trait SetOutDeviceHooks {

    fn log_begin(&mut self, log: u32);

    fn resolve_ctx(&mut self, e0: u32, e1: u32, ctx: &mut u32) -> u32;

    fn set_state(&mut self, obj: u32, locked: &mut u8) -> u32;

    fn flag_state(&mut self, obj: u32, locked: u8);

    fn log_end(&mut self, log: u32);
}

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

pub trait ResourceLoadHooks {

    fn init_pair(&mut self, pair: &mut (u32, u32));

    fn log_begin(&mut self, log: u32);

    fn set_state(&mut self, obj: u32, locked: &mut u8) -> u32;

    fn copy_name(&mut self, resource: Option<u32>, name: &mut [u8; 128]) -> u32;

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

    fn flag_state(&mut self, obj: u32, locked: u8);

    fn log_end(&mut self, log: u32);
}

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

pub trait StopSynthHooks {

    fn log_begin(&mut self, log: u32);

    fn acquire(&mut self, lock: u32) -> u32;

    fn log_error(&mut self, log: u32, code: u32, status: u32);

    fn release(&mut self, lock: u32) -> u32;

    fn flag_voice(&mut self, obj: u32) -> u32;

    fn call_stop(&mut self, a: u32, b: u32) -> u32;

    fn stop_output(&mut self, obj: u32) -> u32;

    fn log_end(&mut self, log: u32);
}

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

pub trait StopVoiceHooks {

    fn apply_mode(&mut self, voice: u32) -> u32;

    fn configure(&mut self, h0: u32, h1: u32, buf: u32, mode: u32) -> u32;

    fn timestamp(&mut self) -> (u32, u32);

    fn start_voice(&mut self, voice: u32, flag: u32) -> u32;

    fn acquire(&mut self, lock: u32) -> u32;

    fn release_lock(&mut self, lock: u32) -> u32;

    fn log_lock_error(&mut self, log: u32, status: u32);

    fn render(
        &mut self,
        h0: u32,
        h1: u32,
        b0: u32,
        b1: u32,
        n0: u32,
        n1: u32,
    ) -> u32;

    fn release_buffer(&mut self, b0: u32, b1: u32) -> u32;
}

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

pub trait CloseStreamHooks {

    fn check(&mut self, obj: u32, tag: u32) -> u32;

    fn resolve_ctx(&mut self, obj: u32, tag: u32, ctx: &mut u32) -> u32;

    fn log_begin(&mut self, log: u32);

    fn open_iter(&mut self, streams: u32, iter: &mut u32) -> u32;

    fn next_entry(&mut self, iter: u32, key: &mut u32, state: &mut u32) -> u32;

    fn log_writer(&mut self, log: u32, writer_id: u32, content_type: u32, version: u32);

    fn log_module(&mut self, log: u32, module_id: u32, content_type: u32, version: u32);

    fn free_obj(&mut self, heap: u32, obj: u32);

    fn next_reader(&mut self, readers: u32, index: u32, reader: &mut u32) -> u32;

    fn close_iter(&mut self, iter: u32);

    fn close_streams(&mut self, streams: u32) -> u32;

    fn log_end(&mut self, log: u32, rc: u32);
}

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

pub trait InsertPromptHooks {

    fn check(&mut self, handle: u32, start: u32) -> u32;

    fn resolve(&mut self, a: u32, b: u32, manager: &mut u32) -> u32;

    fn alloc_text(&mut self, heap: u32, size: u32) -> Option<u32>;

    fn build_prompt(
        &mut self,
        handle: u32,
        text: u32,
        prompt: &mut [u32; 9],
    ) -> u32;

    fn log_insert(&mut self, log: u32);

    fn free_text(&mut self, heap: u32, text: u32);

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

    fn log_rewrite(&mut self, log: u32);

    fn log_no_memory(&mut self, log: u32);
}

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

pub trait ProcessTextHooks {

    fn timestamp(&mut self) -> (u32, u32);

    fn log_begin(&mut self, log: u32);

    fn lock_engine(&mut self, engine: u32) -> (u32, u8);

    fn unlock_engine(&mut self, engine: u32, locked: u8);

    fn process_request(
        &mut self,
        engine: u32,
        request: u32,
        state: &mut (u32, u32),
    ) -> u32;

    fn stop_voice_call(&mut self, engine: u32, s0: u32, s1: u32) -> u32;

    fn log_end(&mut self, log: u32);
}

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

pub trait PreprocessHooks {

    fn check(&mut self, proc: u32, tag: u32) -> u32;

    fn available(&mut self, h0: u32, h1: u32, avail: &mut u32, ended: &mut u8) -> u32;

    fn transfer(
        &mut self,
        oh0: u32,
        oh1: u32,
        ih0: u32,
        ih1: u32,
        count: &mut u32,
    ) -> u32;

    fn release(&mut self, h0: u32, h1: u32) -> u32;

    fn read_stream(
        &mut self,
        h0: u32,
        h1: u32,
        ptr: &mut u32,
        len: &mut u32,
    ) -> u32;

    fn alloc_text(&mut self, heap: u32, size: u32) -> Option<u32>;

    fn alloc_events(&mut self, heap: u32, size: u32) -> Option<u32>;

    fn log_no_memory(&mut self, log: u32);

    fn get_position(&mut self, h0: u32, h1: u32, pos: &mut u32) -> u32;

    fn transform(
        &mut self,
        h0: u32,
        h1: u32,
        text: &mut u32,
        events: &mut u32,
        count: &mut u32,
    ) -> u32;

    fn write_stream(&mut self, h0: u32, h1: u32, data: u32, len: u32) -> u32;

    fn consume(&mut self, h0: u32, h1: u32, bytes: u32) -> u32;

    fn set_position(&mut self, h0: u32, h1: u32, pos: u32) -> u32;

    fn free_buf(&mut self, heap: u32, ptr: u32);
}

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

pub trait PromptOrthHooks {

    fn check(&mut self, p: u32, check: u32, f1: u32, f2: u32) -> u32;

    fn find(&mut self, h0: u32, h1: u32, f1: u32, f2: u32, id: &mut u16) -> u32;

    fn data_type(
        &mut self,
        h0: u32,
        h1: u32,
        id: u16,
        flag: u32,
        ty: &mut u16,
    ) -> u32;

    fn get(
        &mut self,
        h0: u32,
        h1: u32,
        id: u16,
        flag: u32,
        ptr: &mut u32,
        len: &mut u16,
    ) -> u32;

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

    fn get_position(&mut self, h0: u32, h1: u32, pos: &mut u32) -> u32;

    fn set_position(&mut self, h0: u32, h1: u32, pos: u32) -> u32;

    fn alloc_orth(&mut self, heap: u32, size: u32) -> Option<u32>;

    fn alloc_records(&mut self, heap: u32, size: u32) -> Option<u32>;

    fn free_buf(&mut self, heap: u32, ptr: u32);

    fn log_no_memory(&mut self, log: u32, code: u32, flag: u32);
}

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

pub trait PromptTokenHooks {

    fn check(&mut self, p: u32, check: u32, f1: u32, f2: u32) -> u32;

    fn first(&mut self, h0: u32, h1: u32, a: u32, b: u32, out: &mut u16) -> u32;

    fn get_count(
        &mut self,
        h0: u32,
        h1: u32,
        root: u16,
        flag: u32,
        attr: &mut u16,
    ) -> u32;

    fn get_pointer(
        &mut self,
        h0: u32,
        h1: u32,
        root: u16,
        flag: u32,
        ptr: &mut u32,
        len: &mut u16,
    ) -> u32;

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

    fn next(&mut self, h0: u32, h1: u32, token: u16, out: &mut u16) -> u32;

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

    fn get_position(&mut self, h0: u32, h1: u32, pos: &mut u32) -> u32;

    fn set_position(&mut self, h0: u32, h1: u32, pos: u32) -> u32;

    fn alloc_text(&mut self, heap: u32, size: u32) -> Option<u32>;

    fn alloc_records(&mut self, heap: u32, size: u32) -> Option<u32>;

    fn free_buf(&mut self, heap: u32, ptr: u32);

    fn log_no_memory(&mut self, log: u32, code: u32, flag: u32);

    fn log_pause(&mut self, log: u32, level: u32, flag: u32);
}

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

pub trait LingOpenHooks {

    fn resolve_ctx(&mut self, object: u32, tag: u32, ctx: &mut u32) -> u32;

    fn acquire(&mut self, db: u32, res: &mut u32) -> u32;

    fn release_name(&mut self, db: u32, which: u32);

    fn alloc_obj(&mut self, heap: u32) -> Option<u32>;

    fn log_error(&mut self, log: u32, code: u32);

    fn query_ortho(&mut self, props: u32, out: &mut u32) -> u32;
}

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

pub trait PromptOpenHooks {

    fn resolve_env(&mut self, h0: u32, h1: u32, env: &mut u32) -> u32;

    fn acquire(&mut self, res: u32, out: &mut u32) -> u32;

    fn release_name(&mut self, res: u32, which: u32);

    fn alloc_obj(&mut self, heap: u32) -> Option<u32>;

    fn log_error(&mut self, log: u32, code: u32);
}

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

pub trait SetParamHooks {

    fn resolve_ctx(&mut self, h0: u32, h1: u32, ctx: &mut u32) -> u32;

    fn log_change(&mut self, log: u32);

    fn apply_esc(&mut self, ctx: u32, p: u32, value: u32) -> u32;
}

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

pub trait DomainOpenHooks {

    fn resolve_env(&mut self, h0: u32, h1: u32, env: &mut u32) -> u32;

    fn acquire(&mut self, rm: u32, out: &mut u32) -> u32;

    fn release_name(&mut self, rm: u32, which: u32);

    fn alloc_obj(&mut self, heap: u32) -> Option<u32>;

    fn log_error(&mut self, log: u32, code: u32);
}

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

pub trait StreamOpenHooks {

    fn resolve_ctx(&mut self, h0: u32, h1: u32, ctx: &mut u32) -> u32;

    fn log_begin(&mut self, log: u32);

    fn alloc_cfg(&mut self, heap: u32) -> Option<u32>;

    fn log_error(&mut self, log: u32, code: u32);

    fn query_u32(&mut self, props: u32, which: u32, out: &mut u32) -> u32;

    fn query_log_input(&mut self, props: u32, out: &mut u32);

    fn create_map(&mut self, heap: u32, streams: &mut u32) -> u32;

    fn free_cfg(&mut self, heap: u32, cfg: u32);

    fn log_end(&mut self, log: u32, rc: u32);
}

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

pub trait LangListHooks {

    fn check(&mut self, o: u32, a1: u32) -> u32;

    fn log_begin(&mut self, log: u32);

    fn log_error(&mut self, log: u32, code: u32);

    fn open_broker(&mut self, name: u32, db: u32, comp: &mut u32) -> u32;

    fn open_comp_iter(&mut self, comp: u32, iter: &mut u32) -> u32;

    fn next_comp(&mut self, iter: u32, k: &mut u32) -> u32;

    fn open_tag_iter(&mut self, db: u32, k: u32, iter: &mut u32) -> u32;

    fn next_tag(
        &mut self,
        iter: u32,
        tag: &mut Option<u32>,
        value: &mut Option<u32>,
    ) -> u32;

    fn close_tag(&mut self, iter: u32);

    fn close_comp_iter(&mut self, iter: u32);

    fn close_comp(&mut self, comp: u32);

    fn log_end(&mut self, log: u32, rc: u32, count: u32);
}

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

pub fn domain_prompt_exists(handle: Option<u32>, tag: u32, text: u32, mut build: impl FnMut(u32, u32, u32) -> u32) -> u32 {
    if validate_handle(handle.is_some(), tag, 0x11d, 0x48) >= 0x80000000 { return 0; }
    u32::from(build(handle.unwrap(), text, 0) < 0x80000000)
}

pub fn decode_domain_prompt(handle: Option<u32>, tag: u32, text: u32, output: u32, mut build: impl FnMut(u32, u32, u32) -> u32) -> u32 {
    let rc = validate_handle(handle.is_some(), tag, 0x11d, 0x48);
    if rc >= 0x80000000 { return rc; }
    build(handle.unwrap(), text, output)
}

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

pub trait DomainValidation {
    fn property(&mut self, props: u32, name: &[u8], value: &mut u32) -> u32;
    fn mismatch(&mut self, log: u32, strict: bool, domain: &[u8], attribute: &[u8], voice: Option<&[u8]>, apdb: &[u8]);
    fn version_mismatch(&mut self, log: u32, reduction: bool, voice: Option<&[u8]>, apdb: &[u8]);
}

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

pub trait DomainConsultation: DomainApi + VoiceNodeAllocator {
    fn allocate_name(&mut self, heap: u32, size: u32) -> u32;
    fn map_iter(&mut self, map: u32, iterator: &mut u32) -> u32;
    fn map_next(&mut self, iterator: u32, key: &mut u32, value: &mut u32) -> u32;
    fn map_close(&mut self, iterator: u32);
    fn expand_original(&mut self, mem: &mut [u8], database: u32, text: u32, entries: u32, count: u32, head: u32) -> u32;

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

pub trait DomainDatabaseList {
    fn resolve(&mut self, first:u32, second:u32, context:&mut u32)->u32;
    fn create(&mut self, heap:u32, config:[u32;3], single:u32, array:&mut u32)->u32;
    fn release(&mut self, array:u32);
    fn append(&mut self, array:u32, value:u32)->u32;
    fn no_memory(&mut self, log:u32);
}

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

pub fn append_domain_database(mem:&[u8],domain:Option<u32>,value:u32,api:&mut impl DomainDatabaseList)->u32 {
    let Some(domain)=domain else{return 0x88e02007;};
    if api.append(mem_word(mem,domain+8),value)==0 {0x88e0200a}else{0}
}

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
