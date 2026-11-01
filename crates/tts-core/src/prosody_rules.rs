use crate::frontend_types::Error;

pub mod arena;
pub mod rule_item;

pub use arena::Arena;
pub use rule_item::{RuleContext, RuleInterpreterHost, RuleItem, RuleListEntry, RuleSetInstance};

pub struct RuleSetLabels<'a> {
    pub name: Option<&'a [u8]>,
    pub source: Option<&'a [u8]>,
    pub enabled: u8,
}

pub fn rule_set_name<'a>(rules: Option<&RuleSetLabels<'a>>) -> Option<&'a [u8]> {
    rules.and_then(|rules| rules.name)
}

pub fn rule_set_source<'a>(rules: Option<&RuleSetLabels<'a>>) -> Option<&'a [u8]> {
    rules.and_then(|rules| rules.source)
}

pub fn set_rule_set_enabled(rules: Option<&mut RuleSetLabels<'_>>, value: u32) {
    if let Some(rules) = rules {
        rules.enabled = value as u8;
    }
}

pub fn rule_set_enabled(rules: Option<&RuleSetLabels<'_>>) -> u32 {
    rules.map_or(0, |rules| rules.enabled as u32)
}

pub fn apply_gain(samples: &mut [i16], index: u32, gains: &[u16]) -> Result<(), Error> {
    if index == 80 {
        return Ok(());
    }
    let gain = *gains.get(index as usize).ok_or(Error::Capacity {
        required: index as usize + 1,
        available: gains.len(),
    })?;
    for sample in samples {
        let value = (*sample as i32 * gain as i32) >> 15;
        *sample = value.clamp(-32767, 32767) as i16;
    }
    Ok(())
}

fn c_string(bytes: &[u8]) -> &[u8] {
    &bytes[..bytes.iter().position(|&b| b == 0).unwrap_or(bytes.len())]
}

pub(crate) fn signed_decimal(bytes: &[u8]) -> i32 {
    let mut bytes = c_string(bytes);
    while bytes.first().is_some_and(|b| matches!(b, 9..=13 | 32)) {
        bytes = &bytes[1..];
    }
    let negative = bytes.first() == Some(&b'-');
    if matches!(bytes.first(), Some(b'-' | b'+')) {
        bytes = &bytes[1..];
    }
    let value = bytes
        .iter()
        .take_while(|b| b.is_ascii_digit())
        .fold(0i32, |n, b| {
            n.wrapping_mul(10).wrapping_add((b - b'0') as i32)
        });
    if negative {
        value.wrapping_neg()
    } else {
        value
    }
}

pub fn set_markup_rules(enabled: &mut u8, name: &[u8], value: Option<&[u8]>) -> u32 {
    if c_string(name) == b"rulesetsfrommarkup" {
        *enabled = value.is_some_and(|v| {
            let v = c_string(v);
            v.eq_ignore_ascii_case(b"true") || v == b"1"
        }) as u8;
    }
    0
}

pub fn matches_language(
    enabled: u8,
    language: Option<&mut [u8]>,
    input: Option<&[u8]>,
) -> Result<bool, Error> {
    if enabled == 0 {
        return Ok(false);
    }
    let (Some(language), Some(input)) = (language, input) else {
        return Ok(false);
    };
    let input = c_string(input);
    if input.len() >= 8 {
        return Err(Error::Capacity {
            required: input.len() + 1,
            available: 8,
        });
    }
    if c_string(language) == b"*" {
        return Ok(true);
    }
    let length = c_string(language).len();
    language[..length].make_ascii_lowercase();
    let mut folded = [0u8; 8];
    for (out, &byte) in folded.iter_mut().zip(input) {
        *out = byte.to_ascii_lowercase();
    }
    if input.is_empty()
        || language[..length]
            .windows(input.len())
            .any(|w| w == &folded[..input.len()])
    {
        return Ok(true);
    }
    Ok(language.len() >= 3
        && folded[0] == language[0]
        && folded[1] == language[1]
        && language[2] == b'*')
}

pub fn read_rule_line(
    source: &[u8],
    position: &mut usize,
    output: &mut [u8],
) -> Result<u32, Error> {
    if *position > source.len() {
        return Err(Error::InvalidResource);
    }
    let required = (source.len() - *position).min(1023) + 1;
    if output.len() < required {
        return Err(Error::Capacity {
            required,
            available: output.len(),
        });
    }
    let mut first = true;
    loop {
        let mut count = 0;
        while *position < source.len() && count + 1 < 1024 {
            let byte = source[*position];
            *position += 1;
            output[count] = byte;
            count += 1;
            if byte == b'\n' {
                break;
            }
        }
        output[count] = 0;
        if count == 0 {
            return Ok(if *position == source.len() {
                0x88602017
            } else {
                0x88602004
            });
        }
        let length = c_string(output).len();
        if length == 0 {
            return Ok(0);
        }
        if *position == source.len() || output[length - 1] == b'\n' {
            if first {
                if output[length - 1] == b'\n' {
                    output[length - 1] = 0;
                }
                if length >= 2 && output[length - 2] == b'\r' {
                    output[length - 2] = 0;
                }
                return Ok(0);
            }
        }
        first = output[length - 1] == b'\n';
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct InserterParameters {
    pub input_active: u32,
    pub output_active: u32,
    pub frequency: u32,
    pub buffer_samples: u32,
    pub volume: u16,
    pub rate: u16,
    pub adjust_volume: u8,
    pub adjust_rate: u8,
}

impl InserterParameters {

    pub fn validate(&self, name: &[u8], value: &[u8], valid: &mut u8) -> u32 {
        *valid = 1;
        let status = match c_string(name) {
            b"frequencyhz" if self.input_active != 0 || self.output_active != 0 => 0x8b902011,
            b"volume" if signed_decimal(value) as u32 >= 101 => 0x8b90200f,
            b"rate" if (signed_decimal(value) as u32).wrapping_sub(50) >= 351 => 0x8b90200f,
            _ => 0,
        };
        if status != 0 {
            *valid = 0;
        }
        status
    }

    pub fn set(&mut self, name: &[u8], value: &[u8]) -> u32 {
        match c_string(name) {
            b"frequencyhz" => {
                if self.input_active != 0 || self.output_active != 0 {
                    return 0x8b902011;
                }
                self.frequency = crate::resources::decimal(Some(value));
            }
            b"audiooutputbufsamples" => {
                self.buffer_samples = crate::resources::decimal(Some(value));
                if self.buffer_samples == 0 {
                    self.buffer_samples = 1024;
                }
            }
            b"volume" => self.volume = signed_decimal(value) as u16,
            b"rate" => self.rate = signed_decimal(value) as u16,
            b"audioinsertervolumeadjust" => self.adjust_volume = (signed_decimal(value) != 0) as u8,
            b"audioinserterrateadjust" => self.adjust_rate = (signed_decimal(value) != 0) as u8,
            _ => (),
        }
        0
    }
}

pub trait ApplyRuleHost {
    fn resolve(&mut self, h0: u32, h1: u32) -> (u32, Option<RuleContext>);
    fn alloc(&mut self, arena: &mut Arena<'_>, size: u32) -> u32;
    fn realloc(&mut self, arena: &mut Arena<'_>, ptr: u32, size: u32) -> u32;
    fn free(&mut self, arena: &mut Arena<'_>, ptr: u32);
    fn count_rules(&mut self, rules: u32) -> u32;
    fn get_rule(&mut self, rules: u32, index: u32) -> Option<[u32; 2]>;
    fn apply_indexed(
        &mut self,
        arena: &mut Arena<'_>,
        item: &RuleItem,
        index: u32,
        flag: u32,
        input: u32,
        output: &mut u32,
    ) -> u32;
    fn apply_segment(
        &mut self,
        arena: &mut Arena<'_>,
        engine: u32,
        rule: [u32; 2],
        input: u32,
        output: &mut u32,
    ) -> u32;
    fn logging(&mut self) -> bool;
    fn log(&mut self, code: u32);
    fn log_error(&mut self, code: u32);
}

fn tag_memcmp(arena: &Arena<'_>, offset: usize, tag: &[u8], count: usize) -> bool {
    let bytes = arena.bytes();
    count == 0
        || (offset + count <= bytes.len()
            && tag.len() >= count
            && bytes[offset..offset + count] == tag[..count])
}

fn arena_region_eq(arena: &Arena<'_>, first: usize, second: usize, count: usize) -> bool {
    let bytes = arena.bytes();
    count == 0
        || (first + count <= bytes.len()
            && second + count <= bytes.len()
            && bytes[first..first + count] == bytes[second..second + count])
}

fn arena_strstr(arena: &Arena<'_>, start: usize, needle: &[u8]) -> Option<usize> {
    let bytes = arena.bytes();
    let end = bytes[start..]
        .iter()
        .position(|&b| b == 0)
        .map(|len| start + len)
        .unwrap_or(bytes.len());
    let needle_len = needle.iter().position(|&b| b == 0).unwrap_or(needle.len());
    if needle_len == 0 {
        return Some(start);
    }
    (start..=end.saturating_sub(needle_len))
        .find(|&at| bytes[at..at + needle_len] == needle[..needle_len])
}

const RULE_TAG: &[u8] = b"\x1b\\tn";

pub fn apply_rules(
    arena: &mut Arena<'_>,
    item: Option<&RuleItem>,
    input: u32,
    out: Option<&mut u32>,
    host: &mut impl ApplyRuleHost,
) -> u32 {
    let (Some(item), Some(out)) = (item, out) else {
        return 0x88602007;
    };
    if input == 0 {
        return 0x88602007;
    }
    let (status, ctx) = host.resolve(item.handle[0], item.handle[1]);
    if status >= 0x80000000 {
        return 0x88602007;
    }
    let Some(_) = ctx else { return 0x88602007 };
    if item.name == 0 {
        return 0x88602011;
    }
    let mut status = status;
    let mut matches = 1;
    let mut text = host.alloc(arena, arena.len(input) as u32 + 1);
    if text == 0 {
        host.log_error(0x2afa);
        return 0x8860200a;
    }
    arena.copy_str(text, input);
    let mut previous = 0u32;
    if item.kind != 0 {
        let kind_off = item.kind as usize;
        let kind_len = arena.len(item.kind);
        let kind_is_normal = kind_len == 6 && tag_memcmp(arena, kind_off, b"normal", 6);
        let mut scan = text as usize;
        matches = 0;
        if !tag_memcmp(arena, scan, RULE_TAG, 4) && kind_is_normal {
            matches = 1;
        } else {
            loop {
                let Some(found) = arena_strstr(arena, scan, RULE_TAG) else {
                    break;
                };
                scan = found;
                if tag_memcmp(arena, scan, RULE_TAG, 4) {
                    let mut value = scan + 4;
                    if arena.bytes().get(value) == Some(&b'=') {
                        value += 1;
                        if arena_region_eq(arena, value, kind_off, kind_len)
                            && arena.bytes().get(value + kind_len) == Some(&b'\\')
                        {
                            matches = 1;
                            break;
                        }
                    }
                }
                matches = 0;
                scan += 1;
            }
        }
    }
    if host.logging() {
        host.log(8);
    }
    if matches != 0 {
        let count = host.count_rules(item.rules);
        if item.kind != 0 {
            for i in 0..count {
                let swap = text;
                text = previous;
                previous = swap;
                if text != 0 {
                    host.free(arena, text);
                    text = 0;
                }
                host.apply_indexed(arena, item, i, 0, previous, &mut text);
            }
            if previous != 0 {
                host.free(arena, previous);
            }
            previous = text;
            text = 0;
            host.apply_indexed(arena, item, 0, 1, previous, &mut text);
            if previous != 0 {
                host.free(arena, previous);
                previous = 0;
            }
        } else {
            let count = host.count_rules(item.rules);
            for i in 0..count {
                let swap = text;
                text = previous;
                previous = swap;
                let Some(rule) = host.get_rule(item.rules, i) else {
                    host.log_error(0x2b11);
                    if previous != 0 {
                        host.free(arena, previous);
                    }
                    if text != 0 {
                        host.free(arena, text);
                    }
                    host.log(8);
                    return 0x88602006;
                };
                host.log(9);
                status = host.apply_segment(arena, item.engine, rule, previous, &mut text);
                host.log(9);
                if status >= 0x80000000 {
                    if previous != 0 {
                        host.free(arena, previous);
                    }
                    if text != 0 {
                        host.free(arena, text);
                    }
                    host.log(8);
                    return status;
                }
            }
        }
    }
    if text == 0 {
        host.log_error(0x2afa);
        status = 0x8860200a;
        if previous != 0 {
            host.free(arena, previous);
        }
        host.log(8);
        return status;
    }
    let result = if *out == 0 {
        host.alloc(arena, arena.len(text) as u32 + 1)
    } else {
        host.realloc(arena, *out, arena.len(text) as u32 + 1)
    };
    if result == 0 {
        host.log_error(0x2afa);
        status = 0x8860200a;
        if previous != 0 {
            host.free(arena, previous);
        }
        if text != 0 {
            host.free(arena, text);
        }
        host.log(8);
        return status;
    }
    *out = result;
    arena.copy_str(result, text);
    if previous != 0 {
        host.free(arena, previous);
    }
    if text != 0 {
        host.free(arena, text);
    }
    host.log(8);
    status
}

#[allow(clippy::too_many_arguments)]
pub fn apply_rule_indexed(
    arena: &mut Arena<'_>,
    item: Option<&RuleItem>,
    index: u32,
    flag: u32,
    input: u32,
    out: Option<&mut u32>,
    host: &mut impl ApplyRuleHost,
) -> u32 {
    let (Some(item), Some(out)) = (item, out) else {
        return 0x88602007;
    };
    if input == 0 || *out != 0 {
        return 0x88602007;
    }
    let mut status = host.resolve(item.handle[0], item.handle[1]).0;
    if status >= 0x80000000 {
        return 0x88602007;
    }
    if item.name == 0 {
        return 0x88602011;
    }
    let mut rule = [0u32; 2];
    if flag == 0 {
        let Some(found) = host.get_rule(item.rules, index) else {
            host.log_error(0x2b11);
            return 0x88602006;
        };
        rule = found;
    }
    if host.logging() {
        host.log(if flag != 0 { 16 } else { 9 });
    }
    *out = host.alloc(arena, arena.len(input) as u32 + 1);
    if *out == 0 {
        status = 0x8860200a;
    } else {
        arena.put_byte(*out, 0);
        let kind_off = item.kind as usize;
        let kind_len = arena.len(item.kind);
        let end = input as usize + arena.len(input);
        let mut s = input as usize;
        while s < end {
            let rel = arena_strstr(arena, s + 1, RULE_TAG);
            let e = rel.unwrap_or(end);
            let n = e - s;
            let segbuf = host.alloc(arena, n as u32 + 1);
            if segbuf == 0 {
                status = 0x8860200a;
                break;
            }
            {
                let bytes = arena.bytes_mut();
                bytes.copy_within(s..s + n, segbuf as usize);
                bytes[segbuf as usize + n] = 0;
            }
            let mut seg = segbuf as usize;
            let mut matched = false;
            if !tag_memcmp(arena, seg, RULE_TAG, 4) {
                matched = tag_memcmp(arena, kind_off, b"normal", 6);
            } else if tag_memcmp(arena, seg, RULE_TAG, 4)
                && arena.bytes().get(seg + 4) == Some(&b'=')
            {
                let v = seg + 5;
                if arena_region_eq(arena, v, kind_off, kind_len)
                    && arena.bytes().get(v + kind_len) == Some(&b'\\')
                {
                    matched = true;
                    seg = v + kind_len + 1;
                }
            }
            if host.logging() {
                host.log(if flag != 0 { 18 } else { 10 });
            }
            let mut newbuf = 0u32;
            if matched && flag == 0 {
                status = host.apply_segment(arena, item.engine, rule, seg as u32, &mut newbuf);
                if status >= 0x80000000 {
                    host.free(arena, segbuf);
                    break;
                }
            } else {
                newbuf = host.alloc(arena, arena.len(seg as u32) as u32 + 1);
                if newbuf == 0 {
                    status = 0x8860200a;
                    host.free(arena, segbuf);
                    break;
                }
                arena.copy_str(newbuf, seg as u32);
            }
            host.free(arena, segbuf);
            let total = arena.len(*out) + arena.len(newbuf);
            *out = host.realloc(arena, *out, total as u32 + 1);
            if *out == 0 {
                status = 0x8860200a;
                break;
            }
            arena.cat_from(*out, newbuf);
            if host.logging() {
                host.log(if flag != 0 { 18 } else { 11 });
            }
            if newbuf != 0 {
                host.free(arena, newbuf);
            }
            s = e;
            if flag == 0 {
                host.log(10);
            }
        }
    }
    if host.logging() {
        host.log(if flag != 0 { 16 } else { 9 });
    }
    status
}

pub const RULE_CHECK_A: &[u8] = b"ERCP7";
pub const RULE_CHECK_B: &[u8] = b"ERCP3";
pub const RULE_CHECK_C: &[u8] = b"ERCP4";

pub const RULE_WS: &[u8] = b"\t ";
pub const RULE_WS_BRACKET: &[u8] = b"\t ]";

pub trait RuleScanHost {
    fn resolve(&mut self, h0: u32, h1: u32) -> (u32, Option<RuleContext>);
    fn check_stream(&mut self, stream: [u32; 2], tag: &[u8], buf: &[u8]) -> u32;
    fn compile(
        &mut self,
        arena: &mut Arena<'_>,
        stream: [u32; 2],
        re: &[u8],
        opts: u32,
        errpos: &mut i32,
    ) -> u32;
    fn insert_rule(&mut self, rules: u32, replacement: &[u8]) -> bool;
    fn count_rules(&mut self, rules: u32) -> u32;
    fn alloc(&mut self, arena: &mut Arena<'_>, heap: u32, size: u32) -> u32;
    fn free(&mut self, heap: u32, ptr: u32);
    fn check_value(&mut self, stream: [u32; 2], value: &[u8]) -> bool;
    fn logging(&mut self) -> bool;
    fn log(&mut self, code: u32);
    fn log_error(&mut self, code: u32);
    fn error(&mut self, log: u32, level: u32);
}

fn nocase_eq(left: &[u8], right: &[u8]) -> bool {
    let left = &left[..left.iter().position(|&b| b == 0).unwrap_or(left.len())];
    let right = &right[..right.iter().position(|&b| b == 0).unwrap_or(right.len())];
    left.eq_ignore_ascii_case(right)
}

pub(crate) fn scan_strlen(buf: &[u8]) -> usize {
    buf.iter().position(|&b| b == 0).unwrap_or(buf.len())
}

fn scan_copy(data: &[u8], pos: &mut u32, rem: &mut u32, buf: &mut [u8], max: u32) -> (u32, u16) {
    let mut n = *rem;
    let mut p = *pos;
    let mut i = 0u16;
    while n != 0 && (i as u32) + 1 < max {
        let byte = data[p as usize];
        p = p.wrapping_add(1);
        n = n.wrapping_sub(1);
        buf[i as usize] = byte;
        i = i.wrapping_add(1);
        if byte == b'\n' {
            break;
        }
    }
    if (i as u32) < max {
        buf[i as usize] = 0;
    }
    *rem = n;
    *pos = p;
    (n, i)
}

fn read_bounded(
    data: &[u8],
    pos: &mut u32,
    rem: &mut u32,
    buf: &mut [u8],
    host: &mut impl RuleScanHost,
) -> u32 {
    let (mut n, i) = scan_copy(data, pos, rem, buf, 0x400);
    if i != 0 {
        let mut first = true;
        loop {
            let len = scan_strlen(buf);
            if len == 0 {
                return 0;
            }
            if n == 0 || buf[len - 1] == b'\n' {
                if first {
                    if buf[len - 1] == b'\n' {
                        buf[len - 1] = 0;
                    }
                    if len < 2 {
                        return 0;
                    }
                    if buf[len - 2] == b'\r' {
                        buf[len - 2] = 0;
                    }
                    return 0;
                }
                first = buf[len - 1] == b'\n';
            } else {
                if first {
                    host.log_error(0x2b00);
                }
                first = buf[len - 1] == b'\n';
            }
            let (n2, i2) = scan_copy(data, pos, rem, buf, 0x400);
            n = n2;
            if i2 == 0 {
                break;
            }
        }
    }
    if *rem == 0 {
        return 0x88602017;
    }
    host.log_error(0x2b02);
    0x88602004
}

pub fn scan_entry(
    stream: [u32; 2],
    pos: &mut u32,
    rem: &mut u32,
    buf: &mut [u8],
    max: u32,
    data: &[u8],
    host: &mut impl RuleScanHost,
) -> u32 {
    let mut rc = read_bounded(data, pos, rem, buf, host);
    if rc >= 0x80000000 {
        return rc;
    }
    loop {
        let mut rc2 = host.check_stream(stream, RULE_CHECK_A, buf);
        if rc2 >= 0x80000000 {
            rc2 = host.check_stream(stream, RULE_CHECK_B, buf);
            if rc2 >= 0x80000000 {
                return rc;
            }
        }
        let (mut n, i) = scan_copy(data, pos, rem, buf, max);
        if i == 0 {
            break;
        }
        let mut first = true;
        let mut again = false;
        loop {
            let len = scan_strlen(buf);
            if len == 0 {
                rc = 0;
                again = true;
                break;
            }
            if n == 0 || buf[len - 1] == b'\n' {
                if first {
                    if buf[len - 1] == b'\n' {
                        buf[len - 1] = 0;
                    }
                    if len < 2 {
                        rc = 0;
                        again = true;
                        break;
                    }
                    if buf[len - 2] == b'\r' {
                        buf[len - 2] = 0;
                    }
                    rc = 0;
                    again = true;
                    break;
                }
                first = buf[len - 1] == b'\n';
            } else {
                if first {
                    host.log_error(0x2b00);
                }
                first = buf[len - 1] == b'\n';
            }
            let (n2, i2) = scan_copy(data, pos, rem, buf, max);
            n = n2;
            if i2 == 0 {
                break;
            }
        }
        if again {
            continue;
        }
        break;
    }
    if *rem == 0 {
        return 0x88602017;
    }
    host.log_error(0x2b02);
    0x88602004
}

fn buf_chr(buf: &[u8], byte: u8) -> Option<usize> {
    let len = scan_strlen(buf);
    buf[..len].iter().position(|&b| b == byte)
}

fn buf_memcmp(buf: &[u8], at: usize, text: &[u8], count: usize) -> bool {
    at + count <= buf.len() && text.len() >= count && buf[at..at + count] == text[..count]
}

#[allow(clippy::too_many_arguments)]
pub fn scan_header(
    arena: &mut Arena<'_>,
    item: &mut RuleItem,
    stream: [u32; 2],
    pos: &mut u32,
    rem: &mut u32,
    buf: &mut [u8],
    data: &[u8],
    host: &mut impl RuleScanHost,
) -> u32 {
    let (status, ctx) = host.resolve(item.handle[0], item.handle[1]);
    if status >= 0x80000000 {
        return 0x88602007;
    }
    let Some(ctx) = ctx else { return 0x88602007 };
    let mut rc = scan_entry(stream, pos, rem, buf, 0x400, data, host);
    let sect = Some(&b"header"[..]);
    while rc < 0x80000000 {
        let mut s = skip_set(buf, 0, RULE_WS);
        if buf[s] == b'[' {
            let mut p = skip_set(buf, s + 1, RULE_WS);
            if let Some(sect) = sect {
                if buf_memcmp(buf, p, sect, sect.len()) {
                    p += sect.len();
                    p = skip_set(buf, p, RULE_WS);
                    if buf[p] == b']' {
                        if rc < 0x80000000 {
                            break;
                        }
                    }
                }
            } else {
                while buf[p] != 0 && buf_chr(RULE_WS_BRACKET, buf[p]).is_none() {
                    p += 1;
                }
                if buf[p] == 0 {

                } else {
                    p = skip_set(buf, p, RULE_WS);
                    if buf[p] == b']' && rc < 0x80000000 {
                        break;
                    }
                }
            }
        }
        rc = scan_entry(stream, pos, rem, buf, 0x400, data, host);
    }
    if rc >= 0x80000000 {
        host.log_error(0x2b0c);
        return 0x88602007;
    }
    let mut rc2 = scan_entry(stream, pos, rem, buf, 0x400, data, host);
    loop {
        if rc2 >= 0x80000000 {
            break;
        }
        let mut s = skip_bom(buf);
        s = skip_set(buf, s, RULE_WS);
        if buf[s] == b'[' {
            let mut p = skip_set(buf, s + 1, RULE_WS);
            while buf[p] != 0 && buf_chr(RULE_WS_BRACKET, buf[p]).is_none() {
                p += 1;
            }
            p = skip_set(buf, p, RULE_WS);
            if buf[p] == b']' {
                let len = scan_strlen(buf);
                *pos = pos.wrapping_sub(1);
                *rem = rem.wrapping_add(1);
                if data[pos.wrapping_sub(1) as usize] == b'\r' {
                    *pos = pos.wrapping_sub(1);
                    *rem = rem.wrapping_add(1);
                }
                *pos = pos.wrapping_sub(len as u32);
                *rem = rem.wrapping_add(len as u32);
                break;
            }
        }
        host.error(ctx.log, 4);
        let mut p = skip_blanks(buf, 0);
        if buf[p] == 0 {
            host.log_error(0x2b12);
        } else {
            let key = p;
            let mut done = false;
            let mut valid = true;
            while !done {
                if buf[p] == 0 {
                    host.log_error(0x2b12);
                    valid = false;
                    break;
                }
                if buf[p] == b' ' {
                    buf[p] = 0;
                    p += 1;
                    p = skip_blanks(buf, p);
                    if buf[p] == 0 {
                        host.log_error(0x2b12);
                        valid = false;
                        break;
                    }
                    if buf[p] != b'=' {
                        host.log_error(0x2b03);
                        valid = false;
                        break;
                    }
                    p += 1;
                    done = true;
                } else if buf[p] == b'=' {
                    buf[p] = 0;
                    p += 1;
                    done = true;
                } else {
                    p += 1;
                }
            }
            if valid {
                p = skip_blanks(buf, p);
                if buf[p] == 0 {
                    host.log_error(0x2b12);
                } else {
                    let val = p;
                    let mut end = val + scan_strlen(&buf[val..]);
                    while buf[end - 1] == b' ' || buf[end - 1] == b'\t' {
                        end -= 1;
                        buf[end] = 0;
                    }
                    host.error(ctx.log, 5);
                    if nocase_eq(&buf[key..], b"language\0") {
                        if host.check_value(stream, &buf[val..]) {
                            if item.language != 0 {
                                host.log_error(0x2b0d);
                                rc2 = 0x88602007;
                                break;
                            }
                            item.language =
                                host.alloc(arena, ctx.heap, scan_strlen(&buf[val..]) as u32 + 1);
                            if item.language == 0 {
                                rc2 = 0x88602007;
                                break;
                            }
                            arena_copy_from(arena, item.language, buf, val);
                        }
                    }
                    if nocase_eq(&buf[key..], b"type\0") {
                        if item.kind != 0 {
                            rc2 = 0x88602007;
                            break;
                        }
                        item.kind =
                            host.alloc(arena, ctx.heap, scan_strlen(&buf[val..]) as u32 + 1);
                        if item.kind == 0 {
                            rc2 = 0x8860200a;
                            break;
                        }
                        arena_copy_from(arena, item.kind, buf, val);
                    }
                }
            }
        }
        rc2 = scan_entry(stream, pos, rem, buf, 0x400, data, host);
    }
    if item.language == 0 {
        host.log_error(0x2b0e);
        return 0x88602007;
    }
    rc2
}

fn skip_bom(buf: &[u8]) -> usize {
    if buf.len() >= 3 && buf[0] == 0xef && buf[1] == 0xbb && buf[2] == 0xbf {
        3
    } else {
        0
    }
}

fn skip_set(buf: &[u8], mut at: usize, set: &[u8]) -> usize {
    while buf[at] != 0 && set.contains(&buf[at]) {
        at += 1;
    }
    at
}

fn skip_blanks(buf: &[u8], mut at: usize) -> usize {
    while buf[at] == b' ' || buf[at] == b'\t' {
        at += 1;
    }
    at
}

fn arena_copy_from(arena: &mut Arena<'_>, dest: u32, buf: &[u8], src: usize) {
    let len = scan_strlen(&buf[src..]);
    let bytes = arena.bytes_mut();
    bytes[dest as usize..dest as usize + len].copy_from_slice(&buf[src..src + len]);
    bytes[dest as usize + len] = 0;
}

#[allow(clippy::too_many_arguments)]
pub fn scan_rules(
    arena: &mut Arena<'_>,
    item: &mut RuleItem,
    stream: [u32; 2],
    pos: &mut u32,
    rem: &mut u32,
    buf: &mut [u8],
    data: &[u8],
    host: &mut impl RuleScanHost,
) -> u32 {
    let _ = arena;
    let (status, ctx) = host.resolve(item.handle[0], item.handle[1]);
    if status >= 0x80000000 {
        return 0x88602007;
    }
    let Some(ctx) = ctx else { return 0x88602007 };
    let sect: Option<&[u8]> = Some(b"data");
    let mut rc = scan_entry(stream, pos, rem, buf, 0x400, data, host);
    while rc < 0x80000000 {
        let mut s = skip_bom(buf);
        s = skip_set(buf, s, RULE_WS);
        if buf[s] == b'[' {
            let mut p = skip_set(buf, s + 1, RULE_WS);
            if let Some(sect) = sect {
                if buf_memcmp(buf, p, sect, sect.len()) {
                    p += sect.len();
                    p = skip_set(buf, p, RULE_WS);
                    if buf[p] == b']' {
                        break;
                    }
                }
            } else {
                while buf[p] != 0 && buf_chr(RULE_WS_BRACKET, buf[p]).is_none() {
                    p += 1;
                }
                if buf[p] != 0 {
                    p = skip_set(buf, p, RULE_WS);
                    if buf[p] == b']' {
                        break;
                    }
                }
            }
        }
        rc = scan_entry(stream, pos, rem, buf, 0x400, data, host);
    }
    rc = scan_entry(stream, pos, rem, buf, 0x400, data, host);
    if rc >= 0x80000000 {
        return scan_rules_out(0, rc);
    }
    let mut first_err = 0u32;
    loop {
        host.log(2);
        host.error(ctx.log, 5);
        match scan_one_rule(arena, item, stream, buf, &ctx, host) {
            0 => rc = 0,
            6 => return scan_rules_out(first_err, 0x88602006),
            8 => {
                rc = 0;
                if first_err == 0 {
                    first_err = 0x88602000;
                }
            }
            _ => {
                rc = 0x88602007;
                host.log(2);
                if first_err == 0 {
                    first_err = rc;
                }
            }
        }
        rc = scan_entry(stream, pos, rem, buf, 0x400, data, host);
        if rc >= 0x80000000 {
            break;
        }
    }
    scan_rules_out(first_err, rc)
}

fn scan_one_rule(
    arena: &mut Arena<'_>,
    item: &mut RuleItem,
    stream: [u32; 2],
    buf: &mut [u8],
    ctx: &RuleContext,
    host: &mut impl RuleScanHost,
) -> u32 {
    let mut p = skip_blanks(buf, 0);
    if buf[p] == 0 {
        host.log_error(0x2b12);
        return 7;
    }
    let sep = buf[p];
    if sep.is_ascii_digit() || sep == b'\\' {
        host.log_error(0x2b07);
        return 7;
    }
    p += 1;
    let re = p;
    loop {
        if buf[p] == 0 {
            host.log_error(0x2b12);
            return 7;
        }
        if buf[p] == sep {
            break;
        }
        if buf[p] == b'\\' && buf[p + 1] != 0 {
            p += 1;
        }
        p += 1;
    }
    buf[p] = 0;
    p += 1;
    let mut opts = 0u32;
    loop {
        if buf[p] == 0 {
            host.log_error(0x2b12);
            return 7;
        }
        if buf[p] == b'-' {
            if buf_memcmp(buf, p, b"-->", 3) {
                p += 3;
                break;
            }
            host.log_error(0x2b03);
            return 7;
        }
        match buf[p] {
            b'm' => opts |= 2,
            b'\t' | b' ' => (),
            b'i' => opts |= 1,
            b's' => opts |= 4,
            b'x' => opts |= 8,
            _ => {
                host.log_error(0x2b08);
                return 7;
            }
        }
        p += 1;
    }
    p = skip_blanks(buf, p);
    if buf[p] == 0 {
        host.log_error(0x2b12);
        return 7;
    }
    let quoted = buf[p] == b'"';
    if quoted {
        p += 1;
    }
    let term = if quoted { b'"' } else { b' ' };
    let replacement = p;
    let mut terminated = false;
    loop {
        if buf[p] == 0 {
            if term != b' ' {
                host.log_error(0x2b12);
                return 7;
            }
            break;
        }
        if buf[p] == term {
            terminated = true;
            break;
        }
        if buf[p] == b'\\' && buf[p + 1] != 0 {
            p += 1;
        }
        p += 1;
    }
    if terminated {
        buf[p] = 0;
        p += 1;
    }
    p = skip_blanks(buf, p);
    if buf[p] != 0 && buf[p] != b'#' {
        host.log_error(0x2b0a);
        return 7;
    }
    host.error(ctx.log, 5);
    let mut erroff = 0i32;
    let compiled = host.compile(arena, stream, &buf[re..], opts | 0x800, &mut erroff);
    if compiled == 0 {
        host.log_error(0x2afe);
        host.log(2);
        return 8;
    }
    if !host.insert_rule(item.rules, &buf[replacement..]) {
        host.log_error(0x2b0f);
        host.free(ctx.heap, compiled);
        return 6;
    }
    host.log(2);
    host.free(ctx.heap, compiled);
    0
}

fn scan_rules_out(first_err: u32, rc: u32) -> u32 {
    if first_err != 0 {
        return first_err;
    }
    if rc == 0x88602017 {
        return 0;
    }
    rc
}

pub fn rule_interface<'a, T: ?Sized>(
    present: bool,
    tag: u32,
    table: &'a T,
    output: &mut Option<&'a T>,
) -> u32 {
    if !present || tag != 0x10d {
        return 0x81706008;
    }
    *output = Some(table);
    0
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct PatternEngine {
    pub context: [u32; 2],
    pub stream: [u32; 2],
}

pub trait PatternEngineHost {
    fn resolve(&mut self, context: [u32; 2]) -> bool;
    fn reserve(&mut self) -> bool;
    fn open(&mut self, context: [u32; 2], stream: &mut [u32; 2]) -> u32;
    fn configure(&mut self, stream: [u32; 2], first: u32, second: u32) -> u32;
    fn close(&mut self, stream: [u32; 2]) -> u32;
    fn destroy(&mut self, stream: [u32; 2]) -> u32;
    fn release(&mut self);
}

pub fn create_pattern_engine(
    context: [u32; 2],
    output: Option<&mut Option<PatternEngine>>,
    host: &mut impl PatternEngineHost,
) -> u32 {
    let Some(output) = output else {
        return 0x80002007;
    };
    if !host.resolve(context) {
        return 0x80002007;
    }
    *output = None;
    if !host.reserve() {
        return 0x8000200a;
    }
    let mut engine = PatternEngine {
        context,
        stream: [0; 2],
    };
    let mut status = host.open(context, &mut engine.stream);
    if status < 0x80000000 {
        status = host.configure(engine.stream, 10, 10);
        if status < 0x80000000 {
            *output = Some(engine);
            return 0;
        }
        host.destroy(engine.stream);
    }
    host.release();
    status
}

pub fn close_pattern_engine(
    engine: Option<&PatternEngine>,
    host: &mut impl PatternEngineHost,
) -> u32 {
    let Some(engine) = engine else {
        return 0x80002007;
    };
    if !host.resolve(engine.context) {
        return 0x80002007;
    }
    let status = host.close(engine.stream);
    if status >= 0x80000000 {
        return status;
    }
    let status = host.destroy(engine.stream);
    if status >= 0x80000000 {
        return status;
    }
    host.release();
    0
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct RuleSet {
    pub id: u32,
    pub h0: u32,
    pub h1: u32,
    pub f08: u32,
    pub f14: u32,
    pub f18: u32,
    pub f20: u32,
    pub f24: [u32; 2],
    pub f2c: [u32; 2],
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct RuleEnv {
    pub heap: u32,
    pub params: u32,
    pub res: u32,
}

pub trait RuleSetHost {
    fn resolve_env(&mut self, h0: u32, h1: u32) -> (u32, Option<RuleEnv>);
    fn release_parameter(&mut self, params: u32, name: &[u8], owner: u32);
    fn release(&mut self, res: u32, name: &[u8]);
    fn close_engine(&mut self, engine: u32);
    fn destroy_list(&mut self, list: u32);
    fn free(&mut self, heap: u32, object: u32);
}

pub fn destroy_rule_set(p: Option<&RuleSet>, host: &mut impl RuleSetHost) -> u32 {
    let Some(p) = p else { return 0x88602007 };
    let (_, env) = host.resolve_env(p.h0, p.h1);
    let Some(env) = env else { return 0x88602007 };
    host.release_parameter(env.params, b"rulesetsfrommarkup", p.id);
    if p.f08 != 0 {
        host.release(env.res, b"INET");
    }
    if p.f20 != 0 {
        host.release(env.res, b"SYNTHSTREAM");
    }
    if p.f14 != 0 {
        host.close_engine(p.f14);
    }
    if p.f18 != 0 {
        host.destroy_list(p.f18);
    }
    host.free(env.heap, p.id);
    0
}

pub trait RuleStreamHost {
    fn validate(&mut self, object: u32, arg: u32, tag: u32, size: u32) -> u32;
    fn new_handle(&mut self) -> [u32; 2];
    fn same_handle(&mut self, left: [u32; 2], right: [u32; 2]) -> bool;
    fn close_stream(&mut self, handle: [u32; 2]) -> u32;
}

pub fn release_rule_streams(
    p: Option<&mut RuleSet>,
    arg: u32,
    host: &mut impl RuleStreamHost,
) -> u32 {
    let mut result = host.validate(p.as_ref().map_or(0, |p| p.id), arg, 0x10d, 0x38);
    let Some(p) = p else { return result };
    if result < 0x80000000 {
        let handle = host.new_handle();
        if !host.same_handle(p.f24, handle) {
            let status = host.close_stream(p.f24);
            if status >= 0x80000000 {
                result = status;
            }
        }
        p.f24 = host.new_handle();
        let handle = host.new_handle();
        if !host.same_handle(p.f2c, handle) {
            let status = host.close_stream(p.f2c);
            if status >= 0x80000000 {
                result = status;
            }
        }
        p.f2c = host.new_handle();
    }
    result
}

pub struct ReplacementBuffer<'a> {
    pub bytes: &'a mut [u8],
    pub length: u32,
    pub capacity: u32,
    pub limit: u32,
}

impl ReplacementBuffer<'_> {

    pub fn append(
        &mut self,
        text: &[u8],
        mut reserve: impl FnMut(u32) -> bool,
    ) -> Result<u32, Error> {
        let count = u32::try_from(text.len()).map_err(|_| Error::InvalidResource)?;
        if count == 0 {
            return Ok(0);
        }
        let size = self.length.wrapping_add(count);
        if self.length > 0x7fff_ffffu32.wrapping_sub(count)
            || (size >= 1_000_000 && size > self.limit.wrapping_mul(2))
        {
            return Ok(0x80002009);
        }
        if size as usize > self.bytes.len() {
            return Err(Error::Capacity {
                required: size as usize,
                available: self.bytes.len(),
            });
        }
        if self.capacity < size {
            let blocks = (size >> 7) + 1;
            let capacity = if blocks < 0x1000000 {
                blocks << 7
            } else {
                0x7fff_ffff
            };
            if !reserve(capacity) {
                return Ok(0x8000200a);
            }
            self.capacity = capacity;
        }
        self.bytes[self.length as usize..size as usize].copy_from_slice(text);
        self.length = size;
        Ok(0)
    }
}

pub trait ReplacementMatcher {
    fn resolve(&mut self) -> bool;
    fn reserve(&mut self, capacity: u32, existing: bool) -> bool;
    fn execute(&mut self, input: &[u8], offset: u32, captures: &mut [i32; 30]) -> i32;
    fn error(&mut self, result: i32) -> u32;
}

fn replacement_escape(text: &[u8], position: &mut usize) -> u8 {
    *position += 1;
    let byte = text.get(*position).copied().unwrap_or(0);
    *position += 1;
    match byte {
        b'0'..=b'7' => {
            let mut value = byte - b'0';
            for _ in 1..3 {
                let Some(&digit @ b'0'..=b'7') = text.get(*position) else {
                    break;
                };
                value = value.wrapping_mul(8).wrapping_add(digit - b'0');
                *position += 1;
            }
            value
        }
        b'x' => {
            let mut value = 0u8;
            for _ in 0..2 {
                let digit = match text.get(*position) {
                    Some(d @ b'0'..=b'9') => d - b'0',
                    Some(d @ b'a'..=b'f') => d - b'a' + 10,
                    Some(d @ b'A'..=b'F') => d - b'A' + 10,
                    _ => break,
                };
                value = value.wrapping_mul(16).wrapping_add(digit);
                *position += 1;
            }
            value
        }
        b'a' => 7,
        b'b' => 8,
        b'e' => 27,
        b'f' => 12,
        b'n' => 10,
        b'r' => 13,
        b't' => 9,
        b'v' => 11,
        _ => byte,
    }
}

fn append_replacement(
    output: &mut ReplacementBuffer<'_>,
    input: &[u8],
    replacement: Option<&[u8]>,
    captures: &[i32; 30],
    matches: i32,
    matcher: &mut impl ReplacementMatcher,
) -> Result<u32, Error> {
    let Some(replacement) = replacement else {
        return Ok(0x80002007);
    };
    let replacement = c_string(replacement);
    let mut position = 0;
    while position < replacement.len() {
        let mut literal = [0u8; 1];
        let text = match replacement[position] {
            b'\\' => {
                literal[0] = replacement_escape(replacement, &mut position);
                &literal[..]
            }
            b'$' if replacement
                .get(position + 1)
                .is_some_and(|b| b.is_ascii_digit() || *b == b'{') =>
            {
                position += 1;
                let braced = replacement[position] == b'{';
                position += usize::from(braced);
                let mut index = 0i32;
                while let Some(digit @ b'0'..=b'9') = replacement.get(position) {
                    index = index.wrapping_mul(10).wrapping_add((digit - b'0') as i32);
                    position += 1;
                }
                if braced {
                    if replacement.get(position) != Some(&b'}') {
                        continue;
                    }
                    position += 1;
                }
                if index >= matches {
                    continue;
                }
                let index = usize::try_from(index).map_err(|_| Error::InvalidResource)?;
                let pair = captures
                    .get(index * 2..index * 2 + 2)
                    .ok_or(Error::InvalidResource)?;
                if pair[0] < 0 || pair[1] < 0 {
                    continue;
                }
                input
                    .get(pair[0] as usize..pair[1] as usize)
                    .ok_or(Error::InvalidResource)?
            }
            _ => {
                let start = position;
                position += 1;
                &replacement[start..position]
            }
        };
        let status = output.append(text, |capacity| matcher.reserve(capacity, true))?;
        if status >= 0x80000000 {
            return Ok(status);
        }
    }
    Ok(0)
}

pub fn replace_pattern(
    present: bool,
    pattern_present: bool,
    input: Option<&[u8]>,
    replacement: Option<&[u8]>,
    global: bool,
    output: Option<&mut ReplacementBuffer<'_>>,
    existing: bool,
    utf8_lengths: &[u8; 64],
    matcher: &mut impl ReplacementMatcher,
) -> Result<u32, Error> {
    let (Some(input), Some(output)) = (input, output) else {
        return Ok(0x80002007);
    };
    if !present || !pattern_present || !matcher.resolve() {
        return Ok(0x80002007);
    }
    let length = c_string(input).len();
    if length > 0x7fff_ffff {
        return Ok(0x80002009);
    }
    output.limit = length as u32;
    output.length = 0;
    if !existing {
        output.capacity = 128;
        if !matcher.reserve(128, false) {
            return Ok(0x8000200a);
        }
        if output.bytes.len() < 128 {
            return Err(Error::Capacity {
                required: 128,
                available: output.bytes.len(),
            });
        }
        output.bytes[..128].fill(0);
    }
    let mut captures = [0i32; 30];
    let mut previous = [-1i32; 2];
    let mut offset = 0u32;
    while length as u32 > offset {
        let mut matches = matcher.execute(&input[..length], offset, &mut captures);
        if matches >= 0 && captures[..2] == previous {
            let byte = input.get(offset as usize).copied().unwrap_or(0);
            let skip = if byte & 0xc0 == 0xc0 {
                utf8_lengths[(byte & 0x3f) as usize] as u32
            } else {
                0
            };
            matches = matcher.execute(
                &input[..length],
                offset.wrapping_add(skip + 1),
                &mut captures,
            );
        }
        if matches < 0 {
            let status = matcher.error(matches);
            if status >= 0x80000000 {
                if (status >> 20) & !0x800 != 0xa5 || status & 0x1fff != 20 {
                    return Ok(status);
                }
            }
            break;
        }
        let start = usize::try_from(captures[0]).map_err(|_| Error::InvalidResource)?;
        let end = usize::try_from(captures[1]).map_err(|_| Error::InvalidResource)?;
        let prefix = input
            .get(offset as usize..start)
            .ok_or(Error::InvalidResource)?;
        let status = output.append(prefix, |capacity| matcher.reserve(capacity, true))?;
        if status >= 0x80000000 {
            return Ok(status);
        }
        let status = append_replacement(output, input, replacement, &captures, matches, matcher)?;
        if status != 0 {
            return Ok(status);
        }
        previous.copy_from_slice(&captures[..2]);
        offset = end as u32;
        if !global {
            break;
        }
    }
    let tail = input
        .get(offset as usize..length + 1)
        .ok_or(Error::InvalidResource)?;
    output.append(tail, |capacity| matcher.reserve(capacity, true))
}

pub fn replace_pattern_global(
    present: bool,
    pattern_present: bool,
    input: Option<&[u8]>,
    replacement: Option<&[u8]>,
    output: Option<&mut ReplacementBuffer<'_>>,
    existing: bool,
    utf8_lengths: &[u8; 64],
    matcher: &mut impl ReplacementMatcher,
) -> Result<u32, Error> {
    replace_pattern(
        present,
        pattern_present,
        input,
        replacement,
        true,
        output,
        existing,
        utf8_lengths,
        matcher,
    )
}
