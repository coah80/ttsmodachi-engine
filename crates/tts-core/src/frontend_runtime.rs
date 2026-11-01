

use crate::frontend::StateLock;
use crate::frontend_types::Error;

pub trait FrontendLifetime {
    fn validate(&mut self, object: u32, tag: u32, expected: u32, size: u32) -> u32;
    fn session(&mut self, handle: [u32; 2]) -> (u32, u32);
    fn free(&mut self, heap: u32, object: u32);
    fn destroy_map(&mut self, object: u32) -> u32;
    fn release_inet(&mut self, registry: u32);
    fn remove_key(&mut self, map: u32, key: u32) -> u32;
    fn allocate(&mut self, heap: u32, size: u32) -> Option<u32>;
    fn destroy_domain_list(&mut self, object: u32);
}
fn lifetime_word(memory: &[u8], at: u32) -> u32 {
    u32::from_le_bytes(memory[at as usize..at as usize + 4].try_into().unwrap())
}
fn lifetime_put(memory: &mut [u8], at: u32, value: u32) {
    memory[at as usize..at as usize + 4].copy_from_slice(&value.to_le_bytes());
}

pub fn open_audio_fetch_handle(handle: [u32; 2], output: Option<&mut [u32; 2]>, host: &mut impl FrontendLifetime) -> u32 {
    let Some(output) = output else { return 0x87402007; };
    *output = [0; 2];
    let (rc, context) = host.session(handle);
    if rc < 0x80000000 { *output = [context, 0x2bd]; }
    rc
}

pub fn close_audio_fetch_handle() -> u32 { 0 }

pub fn close_audio_fetch_object(memory: &[u8], object: u32, tag: u32, host: &mut impl FrontendLifetime) -> u32 {
    let mut rc = host.validate(object, tag, 0x2be, 44);
    if rc >= 0x80000000 { return rc; }
    let environment = lifetime_word(memory, object + 8);
    for offset in [24, 20] {
        let map = lifetime_word(memory, object + offset);
        if map != 0 { rc = host.destroy_map(map); }
    }
    if lifetime_word(memory, object + 32) != 0 { host.release_inet(lifetime_word(memory, environment + 24)); }
    host.free(lifetime_word(memory, environment + 4), object);
    rc
}

pub fn release_audio_fetch_reference(memory: &mut [u8], object: Option<u32>, heap: u32, host: &mut impl FrontendLifetime) {
    let Some(object) = object else { return; };
    let at = object as usize;
    let refs = u16::from_le_bytes(memory[at..at + 2].try_into().unwrap());
    if refs > 1 {
        memory[at..at + 2].copy_from_slice(&(refs - 1).to_le_bytes());
        return;
    }
    let data = lifetime_word(memory, object + 4);
    if data != 0 { host.free(heap, data); }
    host.free(heap, object);
}

pub fn remove_audio_fetch_key(memory: &[u8], object: u32, tag: u32, key: u32, host: &mut impl FrontendLifetime) -> u32 {
    let rc = host.validate(object, tag, 0x2be, 44);
    if rc >= 0x80000000 { return rc; }
    host.remove_key(lifetime_word(memory, object + 20), key)
}

pub fn domain_unsupported_operation(memory: &[u8], object: u32, tag: u32, host: &mut impl FrontendLifetime) -> u32 {
    let rc = host.validate(object, tag, 0x11d, 0x48);
    if rc >= 0x80000000 { return rc; }
    let (rc, _) = host.session([lifetime_word(memory, object), lifetime_word(memory, object + 4)]);
    if rc >= 0x80000000 { rc } else { 0x88e02001 }
}

pub fn domain_item_copy_enabled() -> u32 { 1 }

pub fn free_domain_item_data(memory: &[u8], object: Option<u32>, heap: u32, host: &mut impl FrontendLifetime) {
    if let Some(object) = object {
        let data = lifetime_word(memory, object);
        if data != 0 { host.free(heap, data); }
    }
}

pub fn move_domain_item(memory: &mut [u8], source: u32, heap: u32, output: &mut u32, host: &mut impl FrontendLifetime) -> u32 {
    let Some(destination) = host.allocate(heap, 16) else { return 0x88e0200a; };

    for offset in 0..16 {
        memory[destination as usize + offset] = memory[source as usize + offset];
    }
    lifetime_put(memory, source + 8, 0);
    *output = destination;
    0
}

pub fn destroy_domain_item(object: Option<u32>, heap: u32, host: &mut impl FrontendLifetime) {
    if let Some(object) = object { host.destroy_domain_list(object); host.free(heap, object); }
}

pub fn utf16_to_ascii(output: &mut [u8], source: &[u16]) -> Result<u32, Error> {
    let length = source
        .iter()
        .position(|&unit| unit == 0)
        .expect("unterminated text");
    let required = length + 1;
    if required > output.len() {
        return Err(Error::Capacity {
            required,
            available: output.len(),
        });
    }
    for (out, &unit) in output[..length].iter_mut().zip(&source[..length]) {
        *out = if unit & 0xff00 != 0 { b'?' } else { unit as u8 };
    }
    output[length] = 0;
    Ok(length as u32)
}

fn unit_at(text: &[u8], offset: usize) -> u16 {
    let low = text[offset] as u16;
    let high = text.get(offset + 1).copied().unwrap_or(0) as u16;
    low | (high << 8)
}

pub fn utf16_to_ascii_in_place(text: &mut [u8], offset: usize) -> Result<u32, Error> {
    let units = (text.len().saturating_sub(offset)) / 2;
    let mut length = 0;
    while length < units && unit_at(text, offset + length * 2) != 0 {
        let unit = unit_at(text, offset + length * 2);
        text[offset + length] = if unit & 0xff00 != 0 { b'?' } else { unit as u8 };
        length += 1;
    }
    let required = offset + length + 1;
    if required > text.len() {
        return Err(Error::Capacity {
            required,
            available: text.len(),
        });
    }
    text[offset + length] = 0;
    Ok(length as u32)
}

pub fn parse_ascii_int_in_place(text: &[u8], offset: usize) -> i32 {
    let units = text.len().saturating_sub(offset) / 2;
    let mut index = 0;
    while index < units {
        let unit = unit_at(text, offset + index * 2);
        if unit & 0xff00 != 0 {
            break;
        }
        let low = unit & 0xff;
        if low != 0x20 && low.wrapping_sub(9) > 4 {
            break;
        }
        index += 1;
    }
    let first = if index < units {
        unit_at(text, offset + index * 2)
    } else {
        0
    };
    let mut cursor = index + 1;
    let mut unit = first;
    if first == 0x2d || first == 0x2b {
        unit = if cursor < units {
            unit_at(text, offset + cursor * 2)
        } else {
            0
        };
        cursor += 1;
    }
    let mut value = 0i32;
    while unit & 0xff00 == 0 && (unit & 0xff).wrapping_sub(0x30) <= 9 {
        value = value
            .wrapping_mul(10)
            .wrapping_add((unit & 0xff) as i32)
            .wrapping_sub(0x30);
        unit = if cursor < units {
            unit_at(text, offset + cursor * 2)
        } else {
            0
        };
        cursor += 1;
    }
    if first == 0x2d {
        0i32.wrapping_sub(value)
    } else {
        value
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct AudioTag {
    pub tag: u16,
    pub field04: u32,
    pub field08: u32,
    pub field0c: u32,
    pub field10: u32,
    pub field14: u32,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct AudioOutRecord {
    pub field00: u32,
    pub flags: u16,
    pub field08: u32,
    pub field0c: u32,
    pub field10: u32,
    pub field14: u32,
    pub field18: u16,
    pub field1c: u32,
    pub field20: u32,
    pub field24: u32,
}

pub fn map_audio_tags(
    mode: u8,
    limit: u32,
    records: &mut [AudioOutRecord],
    tags: &[AudioTag],
    text: &mut [u8],
) -> Result<u32, Error> {
    let mut count = 0usize;
    for tag in tags {
        let Some(record) = records.get_mut(count) else {
            return Err(Error::Capacity {
                required: count + 1,
                available: records.len(),
            });
        };
        record.field00 = 0;
        record.field18 = 0;
        record.field1c = 0;
        record.field20 = 0;
        let flags = match tag.tag {
            0x4000 => {
                if tag.field08 == 0 {
                    continue;
                }
                1
            }
            0x28 => {
                utf16_to_ascii_in_place(text, tag.field14 as usize)?;
                record.field24 = tag.field14;
                0x400
            }
            0x21 => {
                record.field18 = tag.field14 as u16;
                4
            }
            0x1d => {
                record.field1c = parse_ascii_int_in_place(text, tag.field14 as usize) as u32;
                8
            }
            1 => 2,
            8 => {
                record.field20 = tag.field14;
                0x10
            }
            0x19 => {
                record.field20 = tag.field14;
                0x20
            }
            _ => continue,
        };
        record.flags = flags;
        record.field08 = tag.field04;
        record.field0c = tag.field08;
        if mode == 2 {
            if record.flags == 1 && record.field08 < limit {
                let distance = limit - record.field08;
                record.field0c = if distance < record.field0c {
                    record.field0c - distance
                } else {
                    0
                };
            }
            record.field08 = if record.field08 > limit {
                record.field08 - limit
            } else {
                0
            };
        }
        record.field10 = tag.field0c;
        record.field14 = tag.field10;
        count += 1;
    }
    Ok(count as u32)
}

pub fn stop_output_state(state: &mut u8, lock: &mut impl StateLock) -> u32 {
    let result = lock.acquire();
    if result >= 0x80000000 {
        return result;
    }
    let result = match *state {
        3 | 4 => {
            *state = 6;
            result
        }
        5 | 6 => result,
        _ => 0x80002011,
    };
    lock.release();
    result
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct AudioHandle {
    pub first: u32,
    pub second: u32,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct AudioBuffer {
    pub kind: u8,
    pub bytes: u32,
    pub samples: u32,
    pub tag_count: u32,
    pub tags: u32,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct AudioRequest {
    pub command: u32,
    pub flags: u32,
    pub buffer: AudioBuffer,
}

pub trait AudioCallback {
    fn publish(
        &mut self,
        object_id: u32,
        context: u32,
        request: &mut AudioRequest,
        argument: u32,
    ) -> u32;
}

pub trait AudioEngine {
    fn stop(&mut self, handle: AudioHandle) -> u32;
}

pub struct AudioPublication<'a> {
    pub command: Option<&'a mut u32>,
    pub buffer: Option<&'a mut AudioBuffer>,
    pub handle: AudioHandle,
    pub mode: u8,
    pub limit: u32,
    pub state: u8,
    pub bytes: u32,
    pub tag_bytes: u32,
    pub request_flags: u16,
    pub callback_context: u32,
    pub callback_arg: u32,
    pub tags: u32,
    pub tag_count: u32,
}

fn check_state(
    publication: &AudioPublication<'_>,
    stopped: &mut bool,
    lock: &mut impl StateLock,
) -> u32 {
    let status = lock.acquire();
    if status >= 0x80000000 {
        return status;
    }
    *stopped = publication.state == 6;
    lock.release()
}

#[allow(clippy::too_many_arguments)]
pub fn publish_audio(
    publication: &mut AudioPublication<'_>,
    flags: u32,
    samples: &mut u32,
    count: &mut u32,
    tags: &[AudioTag],
    text: &mut [u8],
    records: &mut [AudioOutRecord],
    tag_count: &mut u32,
    lock: &mut impl StateLock,
    engine: &mut impl AudioEngine,
    callback: &mut impl AudioCallback,
) -> Result<u32, Error> {
    const OBJECT_ID: u32 = 0x684f_424a;
    const REQUESTED: u32 = 0x4038_0000;

    let mut local = AudioBuffer::default();
    let mut stopped = false;

    if publication.buffer.is_some() {
        if *count != 0 || *tag_count != 0 {
            if let Some(command) = publication.command.as_deref_mut() {
                *command = 16;
            }
            {
                let buffer = publication.buffer.as_mut().unwrap();
                buffer.bytes = count.wrapping_mul(2);
                buffer.tag_count = *tag_count;
            }
            let filled = map_audio_tags(publication.mode, publication.limit, records, tags, text)?;
            publication.buffer.as_mut().unwrap().tag_count = filled;
            if flags & 2 != 0 {
                return Ok(0);
            }
        } else if flags & 2 != 0 {
            if let Some(command) = publication.command.as_deref_mut() {
                *command = 2;
            }
            *samples = 0;
            *count = 0;
            *tag_count = 0;
            return Ok(0);
        }
        let buffer = publication.buffer.as_ref().unwrap();
        *samples = buffer.samples;
        *count = publication.bytes / 2;
        publication.tags = buffer.tags;
        publication.tag_count = publication.tag_bytes / 40;
        *tag_count = publication.tag_count;
        return Ok(0);
    }

    let mut rc = 0u32;
    if *samples != 0 {
        local.samples = *samples;
        local.tags = publication.tags;
        let status = check_state(publication, &mut stopped, lock);
        if status >= 0x80000000 {
            return Ok(status);
        }
        if stopped {
            local.bytes = 0;
            local.tag_count = 0;
        } else {
            local.bytes = count.wrapping_mul(2);
            local.tag_count = *tag_count;
            local.tag_count =
                map_audio_tags(publication.mode, publication.limit, records, tags, text)?;
        }
        let mut request = AudioRequest {
            command: 16,
            flags: publication.request_flags as u32,
            buffer: local,
        };
        if flags & 2 != 0 {
            publication.request_flags = 0xffff;
            request.flags = publication.request_flags as u32;
        }
        let status = callback.publish(
            OBJECT_ID,
            publication.callback_context,
            &mut request,
            publication.callback_arg,
        );
        rc = if status != 0 {
            (status & 0x1fff) | 0x8000_2000
        } else {
            0
        };
        publication.request_flags = 2;
        if rc >= 0x80000000 {
            if rc.wrapping_shl(19) != REQUESTED {
                return Ok(rc);
            }
            engine.stop(publication.handle);
            stop_output_state(&mut publication.state, lock);
            *samples = 0;
            *count = 0;
            publication.tags = 0;
            *tag_count = 0;
            return Ok(rc);
        }
    }

    if flags & 2 != 0 {
        *samples = 0;
        *count = 0;
        return Ok(rc);
    }
    let status = check_state(publication, &mut stopped, lock);
    if status >= 0x80000000 {
        return Ok(status);
    }
    if stopped {
        *samples = 0;
        *count = 0;
        publication.tags = 0;
        publication.tag_count = 0;
        *tag_count = 0;
        return Ok(rc);
    }
    let mut request = AudioRequest {
        command: 8,
        flags: 0,
        buffer: local,
    };
    let status = callback.publish(
        OBJECT_ID,
        publication.callback_context,
        &mut request,
        publication.callback_arg,
    );
    rc = if status != 0 {
        (status & 0x1fff) | 0x8000_2000
    } else {
        0
    };
    *samples = request.buffer.samples;
    *count = request.buffer.bytes / 2;
    if *samples != 0 && *count != 0 {
        if rc < 0x80000000 {
            publication.tags = request.buffer.tags;
            publication.tag_count = request.buffer.tag_count / 40;
            *tag_count = publication.tag_count;
            return Ok(rc);
        }
        if rc.wrapping_shl(19) != REQUESTED {
            *samples = 0;
            *count = 0;
            publication.tags = 0;
            *tag_count = 0;
            return Ok(rc);
        }
    }
    let status = check_state(publication, &mut stopped, lock);
    if status != 0 && rc < 0x80000000 {
        rc = status;
    }
    if !stopped && status == 0 {
        let stop = engine.stop(publication.handle);
        if stop != 0 && rc < 0x80000000 {
            rc = stop;
        }
        let stop = stop_output_state(&mut publication.state, lock);
        if stop != 0 {
            if rc < 0x80000000 {
                rc = stop;
            } else {
                *samples = 0;
                *count = 0;
                publication.tags = 0;
                *tag_count = 0;
                return Ok(rc);
            }
        }
    }
    if rc < 0x80000000 && *samples != 0 {
        publication.tags = request.buffer.tags;
        publication.tag_count = request.buffer.tag_count / 40;
        *tag_count = publication.tag_count;
        return Ok(rc);
    }
    *samples = 0;
    *count = 0;
    publication.tags = 0;
    *tag_count = 0;
    Ok(rc)
}
