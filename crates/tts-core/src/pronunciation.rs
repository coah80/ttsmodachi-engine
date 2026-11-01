pub use crate::frontend_types::Error;

pub const UNCHANGED: u32 = 0x8a802014;

pub fn is_dictionary_space(c: u32, characters: &[u16]) -> bool {
    for &character in characters {
        if u32::from(character) == c {
            return true;
        }
        if character == 0 {
            break;
        }
    }
    false
}

pub fn is_quote(c: u32) -> bool {
    c == 34
}

pub fn is_digit(c: u32) -> bool {
    c.wrapping_sub(0x30) <= 9
}

pub fn is_latin_punctuation(c: u32) -> bool {
    c.wrapping_sub(33) <= 14
        || c.wrapping_sub(58) <= 6
        || c.wrapping_sub(91) <= 5
        || c.wrapping_sub(123) <= 68
}

pub fn is_general_punctuation(c: u32) -> bool {
    c.wrapping_sub(0x2000) <= 0x6f
}

pub fn is_cjk_punctuation(c: u32) -> bool {
    c.wrapping_sub(0x3000) <= 0x3f
}

pub fn is_presentation_punctuation(c: u32) -> bool {
    c.wrapping_sub(0xfe30) < 0x120
}

pub fn is_small_punctuation(c: u32) -> bool {
    c.wrapping_sub(0xfe50) <= 31
}

pub fn is_fullwidth_punctuation(c: u32) -> bool {
    c.wrapping_sub(0xff00) <= 0xef
}

pub fn is_punctuation(c: u32) -> bool {
    is_latin_punctuation(c)
        || is_general_punctuation(c)
        || is_cjk_punctuation(c)
        || is_presentation_punctuation(c)
        || is_small_punctuation(c)
        || is_fullwidth_punctuation(c)
}

pub fn is_word_character(c: u32) -> bool {
    !is_punctuation(c)
}

pub fn lowercase(c: i32) -> u16 {
    let even = c & 1 == 0;
    let delta =
        if (0x41..=0x5a).contains(&c) || (0xc0..=0xd6).contains(&c) || (0xd8..=0xde).contains(&c) {
            32
        } else if (0x100..=0x136).contains(&c) && even
            || (0x139..=0x147).contains(&c) && !even
            || (0x14a..=0x178).contains(&c) && even
            || (0x179..=0x17d).contains(&c) && !even
        {
            match c {
                0x130 => -199,
                0x178 => -121,
                _ => 1,
            }
        } else if c == 0x386
            || (0x388..=0x38a).contains(&c)
            || c == 0x38c
            || (0x38e..=0x38f).contains(&c)
            || (0x391..=0x3a1).contains(&c)
            || (0x3a3..=0x3ab).contains(&c)
        {
            match c {
                0x386 => 0x26,
                0x388..=0x38a => 0x25,
                0x38c => 0x40,
                0x38e..=0x38f => 0x3f,
                _ => 32,
            }
        } else if (0x400..=0x40f).contains(&c) {
            0x50
        } else if (0x410..=0x42f).contains(&c) {
            32
        } else if (0x460..=0x480).contains(&c) && even
            || (0x48c..=0x4be).contains(&c) && even
            || matches!(c, 0x4c1 | 0x4c3 | 0x4c7 | 0x4cb | 0x4f8)
            || (0x4d0..=0x4f4).contains(&c) && even
        {
            1
        } else {
            0
        };
    c.wrapping_add(delta) as u16
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct WordSpan {
    pub start: u16,
    pub length: u16,
}

impl WordSpan {

    pub fn reset(&mut self, length: u16) -> u32 {
        self.start = 0;
        self.length = length;
        0
    }

    pub fn trim_punctuation(&mut self, text: &[u16], length: u32) -> Result<u32, Error> {
        let old = *self;
        if self.length == 0 {
            return Ok(UNCHANGED);
        }
        let mut i = 0u16;
        loop {
            let c = *text.get(usize::from(i)).ok_or(Error::InvalidResource)?;
            if c == 0 || is_word_character(u32::from(c)) {
                break;
            }
            i = i.checked_add(1).ok_or(Error::InvalidResource)?;
        }
        self.start = i;
        self.length = length.wrapping_sub(u32::from(i)) as u16;
        if u32::from(i) >= length {
            return Ok(UNCHANGED);
        }
        let mut n = length;
        while n != 0 && u32::from(self.start) <= n - 1 {
            let c = *text.get((n - 1) as usize).ok_or(Error::InvalidResource)?;
            if c != 0x2e && is_word_character(u32::from(c)) {
                break;
            }
            n = n.wrapping_sub(1) as u16 as u32;
        }
        if *text.get(n as usize).ok_or(Error::InvalidResource)? == 0x2e {
            n = n.wrapping_add(1) as u16 as u32;
        }
        self.length = n.wrapping_sub(u32::from(self.start)) as u16;
        Ok(if *self != old { 0 } else { UNCHANGED })
    }

    pub fn trim_quotes(&mut self, text: &[u16], length: u32, spaces: &[u16]) -> Result<u32, Error> {
        if self.length == 0 {
            return Ok(UNCHANGED);
        }
        let old = *self;
        let mut i = 0usize;
        let mut opening = false;
        let mut closing = false;
        let at = |i| text.get(i).copied().ok_or(Error::InvalidResource);
        while at(i)? != 0 && is_dictionary_space(u32::from(at(i)?), spaces) {
            i += 1;
        }
        if at(i)? != 0 {
            if is_quote(u32::from(at(i)?)) {
                opening = true;
                i += 1;
            }
            while at(i)? != 0 && is_dictionary_space(u32::from(at(i)?), spaces) {
                i += 1;
            }
        }
        self.start = u16::try_from(i).map_err(|_| Error::InvalidResource)?;
        self.length = length.wrapping_sub(i as u32) as u16;
        if i as u32 >= length {
            return Ok(UNCHANGED);
        }
        let mut end = length as usize;
        while end > i && is_dictionary_space(u32::from(at(end - 1)?), spaces) {
            end -= 1;
        }
        if end > i && is_quote(u32::from(at(end - 1)?)) {
            closing = true;
            end -= 1;
        }
        while end > i && is_dictionary_space(u32::from(at(end - 1)?), spaces) {
            end -= 1;
        }
        self.length = (end - i) as u16;
        Ok(if *self != old || opening && closing {
            0
        } else {
            UNCHANGED
        })
    }

    pub fn trim_periods(&mut self, text: &[u16]) -> Result<u32, Error> {
        if self.length == 0 {
            return Ok(UNCHANGED);
        }
        let start = u32::from(self.start);
        let mut end = self.start.wrapping_add(self.length);
        while end != 0 && u32::from(end - 1) >= start {
            if *text
                .get(usize::from(end - 1))
                .ok_or(Error::InvalidResource)?
                != 0x2e
            {
                break;
            }
            end -= 1;
        }
        let length = end.wrapping_sub(self.start);
        let changed = length != self.length;
        self.length = length;
        Ok(if changed { 0 } else { UNCHANGED })
    }

    pub fn lowercase(&mut self, text: &mut [u16]) -> Result<u32, Error> {
        if self.length == 0 {
            return Ok(UNCHANGED);
        }
        let original = self.length;
        let mut end = self.start.wrapping_add(original);
        if *text.get(usize::from(end)).ok_or(Error::InvalidResource)? == 0x2e {
            end = end.wrapping_add(1);
        }
        self.length = end.wrapping_sub(self.start);
        let start = usize::from(self.start);
        let end = start + usize::from(self.length);
        let available = text.len();
        let range = text.get_mut(start..end).ok_or(Error::Capacity {
            required: end,
            available,
        })?;
        let mut changed = false;
        for c in range {
            let lower = lowercase(i32::from(*c));
            changed |= *c != lower;
            *c = lower;
        }
        Ok(if changed || self.length != original {
            0
        } else {
            UNCHANGED
        })
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DictionaryCursor {
    pub base: u32,
    pub current: u32,
    pub length: u32,
    pub position: u32,
}

impl DictionaryCursor {

    pub fn read(
        &mut self,
        data: &[u8],
        element_size: u32,
        count: u32,
        output: &mut [u8],
    ) -> Result<u32, Error> {
        let bytes = element_size
            .wrapping_mul(count)
            .min(self.length.wrapping_sub(self.position));
        let bytes = bytes as usize;
        if bytes > output.len() {
            return Err(Error::Capacity {
                required: bytes,
                available: output.len(),
            });
        }
        if bytes != 0 {
            let start = self.current.wrapping_sub(self.base) as usize;
            let source = data
                .get(start..start.checked_add(bytes).ok_or(Error::InvalidResource)?)
                .ok_or(Error::InvalidResource)?;
            output[..bytes].copy_from_slice(source);
        }
        self.current = self.current.wrapping_add(bytes as u32);
        self.position = self.position.wrapping_add(bytes as u32);
        Ok(if element_size == 0 {
            0
        } else {
            bytes as u32 / element_size
        })
    }

    pub fn seek(&mut self, offset: u32, origin: u32) -> bool {
        match origin {
            0 => {
                self.position = offset;
                self.current = self.base.wrapping_add(offset);
            }
            1 => {
                self.current = self.current.wrapping_add(offset);
                self.position = self.position.wrapping_add(offset);
            }
            _ => {
                self.current = self.base.wrapping_add(self.length).wrapping_add(offset);
                self.position = self.length.wrapping_add(offset);
            }
        }
        self.position > self.length
    }
}

pub fn valid_language_list(language: Option<&[u8]>) -> bool {
    let Some(language) = language else {
        return false;
    };
    let length = language
        .iter()
        .position(|&c| c == 0)
        .unwrap_or(language.len());
    length >= 3 && (3..length).step_by(4).all(|i| language[i] == b'|')
}

pub fn dictionary_interface(selector: u32, table: &[u8]) -> Result<&[u8], u32> {
    if selector <= 2 {
        Ok(table)
    } else {
        Err(0x85002001)
    }
}

pub fn g2p_interface(selector: u32, table: &[u8]) -> Result<&[u8], u32> {
    if selector <= 1 {
        Ok(table)
    } else {
        Err(0x8c902001)
    }
}

pub fn copy_g2p_value(value: u32, output: &mut u32) -> u32 {
    *output = value;
    0
}

pub fn copy_dictionary_value(value: u32, output: &mut u32) -> u32 {
    *output = value;
    0
}

pub fn finish_g2p() -> u32 {
    0
}

pub fn release_g2p_value() {}

pub fn close_dictionary_callback() -> u32 {
    0
}

pub fn flush_dictionary_callback() -> u32 {
    0
}

pub fn finish_dictionary() -> u32 {
    0
}

pub struct DictionaryQueue<'a, T> {
    pub entries: &'a mut [T],
    pub count: u32,
    pub limit: u32,
    pub growing: bool,
}

impl<T: Copy> DictionaryQueue<'_, T> {

    pub fn is_empty(&self) -> bool {
        self.count == 0
    }

    pub fn front(&self) -> Result<Option<T>, Error> {
        if self.count == 0 {
            Ok(None)
        } else {
            self.entries
                .first()
                .copied()
                .map(Some)
                .ok_or(Error::InvalidResource)
        }
    }

    pub fn push(&mut self, value: T) -> Result<u32, Error> {
        if !self.growing && self.count == self.limit {
            return Ok(7);
        }
        let index = self.count as usize;
        let required = index.checked_add(1).ok_or(Error::InvalidResource)?;
        if required > self.entries.len() {
            return Err(Error::Capacity {
                required,
                available: self.entries.len(),
            });
        }
        self.entries[index] = value;
        self.count = self.count.wrapping_add(1);
        Ok(0)
    }

    pub fn pop(&mut self) -> Result<u32, Error> {
        if self.count == 0 {
            return Ok(2);
        }
        let count = self.count as usize;
        if count > self.entries.len() {
            return Err(Error::InvalidResource);
        }
        self.entries.copy_within(1..count, 0);
        self.count -= 1;
        Ok(0)
    }
}

pub fn reset_one_word(check: impl FnOnce(u32, u32) -> u32) -> u32 {
    let status = check(0xf384, 0x4c8);
    if status >= 0x80000000 {
        0x89b02008
    } else {
        status
    }
}

pub fn finish_one_word(check: impl FnOnce(u32, u32) -> u32) -> u32 {
    let status = check(0xf384, 0x4c8);
    if status >= 0x80000000 {
        0x89b02008
    } else {
        status
    }
}

pub fn one_word_interface(selector: u32, table: &[u8]) -> Result<&[u8], u32> {
    if selector <= 1 {
        Ok(table)
    } else {
        Err(0x89b02001)
    }
}

pub fn transliteration_interface(selector: u32, table: &[u8]) -> Result<&[u8], u32> {
    if selector <= 1 {
        Ok(table)
    } else {
        Err(0x8a602001)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DictionaryTranscription<'a> {
    pub kind: u8,
    pub text: &'a [u8],
    pub length: u32,
}

pub trait TranscriptionDictionary<'a> {
    fn language(&mut self) -> (u32, Option<&'a [u8]>);
    fn lookup(&mut self, word: &[u8]) -> (u32, Option<&'a [DictionaryTranscription<'a>]>, u32);
    fn release(&mut self, records: &'a [DictionaryTranscription<'a>]) -> u32;
}

pub trait DictionaryTextDecoder {
    fn terminated_len(&mut self, text: &[u8]) -> u32;
    fn decode_terminated(&mut self, text: &[u8], output: &mut [u16]) -> u32;
}

fn transcription_append(output: &mut [u16], text: &[u8]) -> Result<(), Error> {
    let end = output
        .iter()
        .position(|&c| c == 0)
        .ok_or(Error::InvalidResource)?;
    let length = text
        .iter()
        .position(|&c| c == 0)
        .ok_or(Error::InvalidResource)?;
    let required = end.checked_add(length + 1).ok_or(Error::InvalidResource)?;
    if output.len() < required {
        return Err(Error::Capacity {
            required,
            available: output.len(),
        });
    }
    for (destination, &byte) in output[end..required].iter_mut().zip(&text[..=length]) {
        *destination = u16::from(byte);
    }
    Ok(())
}

pub fn lookup_transcriptions<'a>(
    available: bool,
    language: &mut Option<&'a [u8]>,
    word: &[u8],
    output: &mut [u16],
    capacity: &mut u16,
    content_type: &mut u32,
    limit: Option<u32>,
    dictionary: &mut impl TranscriptionDictionary<'a>,
    decoder: &mut impl DictionaryTextDecoder,
) -> Result<u32, Error> {
    if !available || word.first().copied().unwrap_or(0) == 0 {
        return Ok(0x85002007);
    }
    let (status, dictionary_language) = dictionary.language();
    let Some(dictionary_language) = dictionary_language.filter(|_| status == 0) else {
        return Ok(0x85002014);
    };
    let prefix = dictionary_language.get(..3).ok_or(Error::InvalidResource)?;
    let code: [u8; 3] = core::array::from_fn(|i| prefix[i].to_ascii_lowercase());
    let code_length = code.iter().position(|&c| c == 0).unwrap_or(code.len());
    if let Some(current) = language {
        let current = &current[..current
            .iter()
            .position(|&c| c == 0)
            .unwrap_or(current.len())];
        if !current
            .windows(code_length.max(1))
            .any(|w| w == &code[..code_length])
            && code_length != 0
        {
            return Ok(0x85002014);
        }
    }
    *language = Some(dictionary_language);
    let (mut status, records, count) = dictionary.lookup(word);
    if status == 0 && count != 0 && records.is_some() {
        let records = records.unwrap();
        let first = records.first().ok_or(Error::InvalidResource)?;
        *content_type = u32::from(first.kind);
        match first.kind {
            1 => {
                let first_output = output.first_mut().ok_or(Error::Capacity {
                    required: 1,
                    available: 0,
                })?;
                *first_output = 0;
                let count = count.min(u32::from(limit.unwrap_or(1) as u16)) as usize;
                let records = records.get(..count).ok_or(Error::InvalidResource)?;
                let mut length = 0u32;
                for (index, record) in records.iter().enumerate() {
                    if record.kind != 1 {
                        continue;
                    }
                    let measured = record
                        .text
                        .get(..record.length as usize)
                        .ok_or(Error::InvalidResource)?;
                    let n = measured
                        .iter()
                        .position(|&c| c == 0)
                        .unwrap_or(measured.len()) as u32;
                    if n.wrapping_add(1) > u32::from(*capacity) {
                        status = 0x85002009;
                        break;
                    }
                    if index != 0 {
                        transcription_append(output, b"\x12\0")?;
                        length = length.wrapping_add(1);
                    }
                    transcription_append(output, record.text)?;
                    length = length.wrapping_add(n);
                }
                if status == 0 {
                    let available = output.len();
                    *output.get_mut(length as usize).ok_or(Error::Capacity {
                        required: length as usize + 1,
                        available,
                    })? = 0;
                    *capacity = length as u16;
                }
            }
            4 => {
                let length = decoder.terminated_len(first.text);
                if length > u32::from(*capacity) {
                    status = 0x85002009;
                } else {
                    status = decoder.decode_terminated(first.text, output);
                    if status < 0x80000000 {
                        *capacity = length.wrapping_sub(1) as u16;
                    }
                }
            }
            _ => status = 0x85002014,
        }
    } else if status < 0x80000000 {
        status = 0x85002014;
    }
    if let Some(records) = records {
        status = dictionary.release(records);
        if status != 0 {
            return Ok(status.wrapping_add(0x1ea0) | 0x85002000);
        }
    }
    Ok(status)
}

pub trait WideDictionaryLookup {
    fn lookup(
        &mut self,
        text: &mut [u16],
        capacity: &mut u16,
        output: &mut [u16],
        content_type: &mut u32,
        flags: u32,
    ) -> u32;
}

pub enum DictionaryLookupTarget<'a, D, C> {
    Dictionary(D),
    Callback(C),
    Missing(core::marker::PhantomData<&'a ()>),
}

#[derive(Clone, Copy)]
pub struct DictionaryLookupConfig<'a> {
    pub language: Result<&'a [u8], u32>,
    pub transcription_limit: Option<u32>,
    pub all_transforms: bool,
    pub spaces: &'a [u16],
}

fn restore_dictionary_text(
    output: &mut [u16],
    capacity: &mut u16,
    input: &[u16],
    input_length: u32,
    initial_capacity: u32,
    span: WordSpan,
) -> Result<u32, Error> {
    let length = u32::from(*capacity)
        .wrapping_add(input_length)
        .wrapping_sub(u32::from(span.length));
    if length as i32 > initial_capacity as i32 {
        return Ok(0x8a802009);
    }
    let prefix = usize::from(span.start);
    let count = usize::from(*capacity);
    let tail = input_length as i32 - i32::from(span.length) - i32::from(span.start);
    let required = prefix
        .checked_add(count)
        .and_then(|n| n.checked_add(tail.max(0) as usize))
        .ok_or(Error::InvalidResource)?;
    if output.len() < required {
        return Err(Error::Capacity {
            required,
            available: output.len(),
        });
    }
    if prefix != 0 {
        output.copy_within(..count, prefix);
        output[..prefix].copy_from_slice(input.get(..prefix).ok_or(Error::InvalidResource)?);
    }
    if tail > 0 {
        let source = prefix + usize::from(span.length);
        output[prefix + count..required].copy_from_slice(
            input
                .get(source..source + tail as usize)
                .ok_or(Error::InvalidResource)?,
        );
    }
    *capacity = length as u16;
    Ok(0)
}

pub fn lookup_word<'a, D: TranscriptionDictionary<'a>, C: WideDictionaryLookup>(
    config: DictionaryLookupConfig<'_>,
    target: &mut DictionaryLookupTarget<'_, D, C>,
    input: Option<&mut [u16]>,
    content_type: &mut u32,
    output: &mut [u16],
    capacity: &mut u16,
    flags: u32,
    encoded: &mut [u8],
    decoder: &mut impl DictionaryTextDecoder,
) -> Result<u32, Error> {
    let Some(input) = input else {
        return Ok(0x8a802007);
    };
    if matches!(target, DictionaryLookupTarget::Missing(_)) {
        return Ok(UNCHANGED);
    }
    let language = match config.language {
        Ok(language) => language,
        Err(status) => return Ok(status),
    };
    if let DictionaryLookupTarget::Dictionary(dictionary) = target {
        let (status, dictionary_language) = dictionary.language();
        let Some(dictionary_language) = dictionary_language.filter(|_| status == 0) else {
            return Ok(UNCHANGED);
        };
        let end = |s: &[u8]| s.iter().position(|&b| b == 0).unwrap_or(s.len());
        let dictionary_language = &dictionary_language[..end(dictionary_language)];
        let language = &language[..end(language)];
        if !dictionary_language.eq_ignore_ascii_case(language) {
            return Ok(UNCHANGED);
        }
    }
    let input_length = input
        .iter()
        .position(|&c| c == 0)
        .ok_or(Error::InvalidResource)? as u16;
    let initial_capacity = u32::from(*capacity);
    let transforms = if config.all_transforms { 6 } else { 1 };
    let mut span = WordSpan {
        start: 0,
        length: 0,
    };
    let mut output_length = 0u32;
    for transform in 0..transforms {
        let status = match transform {
            0 => span.reset(input_length),
            1 => span.trim_quotes(input, u32::from(input_length), config.spaces)?,
            2 => span.trim_punctuation(input, u32::from(input_length))?,
            3 | 5 => span.trim_periods(input)?,
            4 => span.lowercase(input)?,
            _ => unreachable!(),
        };
        if status & 0x1fff == 0x14 {
            continue;
        }
        if status >= 0x80000000 {
            return Ok(status);
        }
        let start = usize::from(span.start);
        let end = start + usize::from(span.length);
        let saved = *input.get(end).ok_or(Error::InvalidResource)?;
        input[end] = 0;
        let text = input.get_mut(start..=end).ok_or(Error::InvalidResource)?;
        let mut status;
        let mut records = None;
        match target {
            DictionaryLookupTarget::Callback(callback) => {
                status = callback.lookup(text, capacity, output, content_type, flags);
            }
            DictionaryLookupTarget::Dictionary(dictionary) => {
                let required = crate::text::encoded_len(text) as usize;
                if encoded.len() < required {
                    return Err(Error::Capacity {
                        required,
                        available: encoded.len(),
                    });
                }
                crate::text::encode_terminated(text, encoded);
                let (lookup_status, found, count) = dictionary.lookup(&encoded[..required]);
                status = lookup_status;
                records = found;
                if status == 0 && count != 0 && records.is_some() {
                    let found = records.unwrap();
                    let first = found.first().ok_or(Error::InvalidResource)?;
                    *content_type = u32::from(first.kind);
                    match first.kind {
                        1 => {
                            *output.first_mut().ok_or(Error::Capacity {
                                required: 1,
                                available: 0,
                            })? = 0;
                            let count = count
                                .min(u32::from(config.transcription_limit.unwrap_or(1) as u16))
                                as usize;
                            for (index, record) in found
                                .get(..count)
                                .ok_or(Error::InvalidResource)?
                                .iter()
                                .enumerate()
                            {
                                let measured = record
                                    .text
                                    .get(..record.length as usize)
                                    .ok_or(Error::InvalidResource)?;
                                let length = measured
                                    .iter()
                                    .position(|&b| b == 0)
                                    .unwrap_or(measured.len())
                                    as u32;
                                if length.wrapping_add(1) > u32::from(*capacity) {
                                    status = 0x8a802009;
                                    break;
                                }
                                if index != 0 {
                                    transcription_append(output, b"\x12\0")?;
                                    output_length = output_length.wrapping_add(1);
                                }
                                transcription_append(output, record.text)?;
                                output_length = output_length.wrapping_add(length);
                            }
                            if status == 0 {
                                let available = output.len();
                                *output.get_mut(output_length as usize).ok_or(
                                    Error::Capacity {
                                        required: output_length as usize + 1,
                                        available,
                                    },
                                )? = 0;
                                *capacity = output_length as u16;
                            }
                        }
                        4 => {
                            let length = decoder.terminated_len(first.text);
                            if length > initial_capacity {
                                status = 0x8a802009;
                            } else {
                                status = decoder.decode_terminated(first.text, output);
                                if status < 0x80000000 {
                                    *capacity = length.wrapping_sub(1) as u16;
                                }
                            }
                        }
                        _ => status = UNCHANGED,
                    }
                } else {
                    status = UNCHANGED;
                }
            }
            DictionaryLookupTarget::Missing(_) => unreachable!(),
        }
        input[end] = saved;
        if status < 0x80000000 {
            if let (DictionaryLookupTarget::Dictionary(dictionary), Some(records)) =
                (&mut *target, records)
            {
                status = dictionary.release(records);
                if status != 0 {
                    return Ok(status.wrapping_add(0x1ea0) | 0x8a802000);
                }
            }
        }
        if status & 0x1fff != 0x14 && status & 0x1fff != 0x404 {
            if status >= 0x80000000 {
                return Ok(status);
            }
            if status == 0 {
                return if *content_type == 4 {
                    restore_dictionary_text(
                        output,
                        capacity,
                        input,
                        u32::from(input_length),
                        initial_capacity,
                        span,
                    )
                } else {
                    Ok(status)
                };
            }
        }
    }
    Ok(UNCHANGED)
}

pub trait CanonicalLoader {
    fn refresh(&mut self, key: &[u8]) -> u32;
}

#[derive(Clone, Copy)]
pub struct CanonicalTables<'a> {
    pub character_class: &'a [u8],
    pub replacements: &'a [Option<&'a [u8]>],
    pub mode: u16,
}

pub(crate) fn terminated_len(text: &[u8]) -> u32 {
    text.iter().position(|&c| c == 0).unwrap_or(text.len()) as u32
}

pub(crate) fn class_index(set: &[u8], c: u8) -> Option<usize> {
    let end = set.iter().position(|&b| b == 0).unwrap_or(set.len());
    if c == 0 {
        return Some(end);
    }
    set[..end].iter().position(|&b| b == c)
}

pub(crate) fn string_differs(a: &[u8], b: &[u8]) -> bool {
    let an = terminated_len(a) as usize;
    let bn = terminated_len(b) as usize;
    a.get(..an) != b.get(..bn)
}

pub(crate) fn copy_terminated(dst: &mut [u8], src: &[u8]) -> Result<(), Error> {
    let length = terminated_len(src) as usize + 1;
    if dst.len() < length {
        return Err(Error::Capacity {
            required: length,
            available: dst.len(),
        });
    }
    dst[..length].copy_from_slice(src.get(..length).ok_or(Error::InvalidResource)?);
    Ok(())
}

fn copy_padded(dst: &mut [u8], src: &[u8], count: u32) -> Result<(), Error> {
    let count = count as usize;
    if dst.len() < count {
        return Err(Error::Capacity {
            required: count,
            available: dst.len(),
        });
    }
    let mut written = 0;
    while written < count {
        let byte = *src.get(written).ok_or(Error::InvalidResource)?;
        dst[written] = byte;
        written += 1;
        if byte == 0 {
            dst[written..count].fill(0);
            return Ok(());
        }
    }
    Ok(())
}

pub(crate) fn move_within(
    text: &mut [u8],
    destination: usize,
    source: usize,
    count: u32,
) -> Result<(), Error> {
    let count = count as usize;
    let end = source.checked_add(count).ok_or(Error::InvalidResource)?;
    let target = destination
        .checked_add(count)
        .ok_or(Error::InvalidResource)?;
    if end > text.len() || target > text.len() {
        return Err(Error::Capacity {
            required: target.max(end),
            available: text.len(),
        });
    }
    text.copy_within(source..end, destination);
    Ok(())
}

fn expand_replacement(
    text: &mut [u8],
    index: u16,
    bound: u16,
    replacement: &[u8],
    capacity: u32,
) -> Result<Option<u16>, Error> {
    let extra = terminated_len(replacement) as u16;
    let total = u32::from(bound) + u32::from(extra);
    if u32::from(total as u16) > capacity {
        return Ok(None);
    }
    move_within(
        text,
        usize::from(index) + usize::from(extra),
        usize::from(index) + 1,
        u32::from(bound) - u32::from(index),
    )?;
    copy_padded(
        text.get_mut(usize::from(index)..)
            .ok_or(Error::InvalidResource)?,
        replacement,
        u32::from(extra),
    )?;
    Ok(Some((total as u16).wrapping_sub(1)))
}

fn leading_class_run(text: &[u8], character_class: &[u8]) -> Result<u16, Error> {
    let mut start = 0u16;
    while u32::from(start) < u32::from(terminated_len(text) as u16)
        && class_index(
            character_class,
            *text.get(usize::from(start)).ok_or(Error::InvalidResource)?,
        )
        .is_some()
    {
        start = start.wrapping_add(1);
    }
    Ok(start)
}

fn trailing_class_run(text: &[u8], character_class: &[u8]) -> Result<u16, Error> {
    let mut end = terminated_len(text).wrapping_sub(1) as u16;
    while (end as i16) > 0
        && class_index(
            character_class,
            *text.get(usize::from(end)).ok_or(Error::InvalidResource)?,
        )
        .is_some()
    {
        end = end.wrapping_sub(1);
    }
    Ok(end)
}

fn fold_range(text: &mut [u8], start: u16) -> Result<(), Error> {
    let mut i = start;
    while u32::from(i) < terminated_len(text) {
        let byte = *text.get(usize::from(i)).ok_or(Error::InvalidResource)?;
        *text.get_mut(usize::from(i)).ok_or(Error::InvalidResource)? =
            crate::morphology::lowercase_byte(byte);
        i = i.wrapping_add(1);
    }
    Ok(())
}

pub fn canonical_trim(
    text: &mut [u8],
    capacity: u32,
    tables: CanonicalTables<'_>,
) -> Result<u32, Error> {
    let start = leading_class_run(text, tables.character_class)?;
    let end = trailing_class_run(text, tables.character_class)?;
    *text
        .get_mut(usize::from(end.wrapping_add(1)))
        .ok_or(Error::InvalidResource)? = 0;
    if tables.mode != 2 && terminated_len(text) as i32 - i32::from(start) > 1 {
        let mut bound = terminated_len(text) as u16;
        let mut i = start;
        while i < bound {
            let byte = *text.get(usize::from(i)).ok_or(Error::InvalidResource)?;
            match tables
                .replacements
                .get(usize::from(byte))
                .copied()
                .flatten()
            {
                Some(replacement) => {
                    match expand_replacement(text, i, bound, replacement, capacity)? {
                        Some(next) => bound = next,
                        None => return Ok(0x89802007),
                    }
                }
                None => {
                    *text.get_mut(usize::from(i)).ok_or(Error::InvalidResource)? =
                        crate::morphology::lowercase_byte(byte);
                }
            }
            i = i.wrapping_add(1);
        }
    }
    let tail = terminated_len(
        text.get(usize::from(start)..)
            .ok_or(Error::InvalidResource)?,
    ) + 1;
    move_within(text, 0, usize::from(start), tail).map(|()| 0)
}

pub fn canonical_replace(
    text: &mut [u8],
    capacity: u32,
    replacements: &[Option<&[u8]>],
    mode: u16,
) -> Result<u32, Error> {
    if mode != 2 && terminated_len(text) > 1 {
        let mut bound = terminated_len(text) as u16;
        let mut i = 0u16;
        while i < bound {
            let byte = *text.get(usize::from(i)).ok_or(Error::InvalidResource)?;
            match replacements.get(usize::from(byte)).copied().flatten() {
                Some(replacement) => {
                    match expand_replacement(text, i, bound, replacement, capacity)? {
                        Some(next) => bound = next,
                        None => return Ok(0x89802007),
                    }
                }
                None => {
                    *text.get_mut(usize::from(i)).ok_or(Error::InvalidResource)? =
                        crate::morphology::lowercase_byte(byte);
                }
            }
            i = i.wrapping_add(1);
        }
    }
    Ok(0)
}

pub fn class_contains(character_class: &[u8], c: u8) -> bool {
    c != 0 && class_index(character_class, c).is_some()
}

pub fn canonical_word(
    text: &mut [u8],
    key: &[u8],
    cached: &mut [u8],
    cache_enabled: bool,
    tables: CanonicalTables<'_>,
    loader: &mut impl CanonicalLoader,
) -> Result<u32, Error> {
    if terminated_len(text) == 0 {
        return Ok(0);
    }
    if cache_enabled && string_differs(key, cached) {
        let status = loader.refresh(key);
        if status >= 0x80000000 {
            return Ok(status);
        }
        copy_terminated(cached, key)?;
    }
    if tables.mode == 0 {
        return canonical_trim(text, terminated_len(text).wrapping_add(1), tables);
    }
    let start = leading_class_run(text, tables.character_class)?;
    let end = trailing_class_run(text, tables.character_class)?;
    *text
        .get_mut(usize::from(end.wrapping_add(1)))
        .ok_or(Error::InvalidResource)? = 0;
    if tables.mode != 2 && terminated_len(text) as i32 - i32::from(start) > 1 {
        fold_range(text, start)?;
    }
    let tail = terminated_len(
        text.get(usize::from(start)..)
            .ok_or(Error::InvalidResource)?,
    ) + 1;
    move_within(text, 0, usize::from(start), tail)?;
    Ok(0)
}

pub fn canonical_token(
    text: &mut [u8],
    input: &[u8],
    last: &mut [u8],
    remember: bool,
    replacements: &[Option<&[u8]>],
    mode: u16,
    loader: &mut impl CanonicalLoader,
) -> Result<u32, Error> {
    if remember && string_differs(input, last) {
        let status = loader.refresh(input);
        if status >= 0x80000000 {
            return Ok(status);
        }
        copy_terminated(last, input)?;
    }
    if mode == 0 {
        return canonical_replace(
            text,
            terminated_len(text).wrapping_add(1),
            replacements,
            mode,
        );
    }
    if mode != 2 && terminated_len(text) > 1 {
        fold_range(text, 0)?;
    }
    Ok(0)
}

pub fn g2p_canonical_token(
    text: &mut [u8],
    capacity: u32,
    key: &[u8],
    last: &mut [u8],
    remember: bool,
    replacements: &[Option<&[u8]>],
    mode: u16,
    loader: &mut impl CanonicalLoader,
) -> Result<u32, Error> {
    if remember && string_differs(key, last) {
        let status = loader.refresh(key);
        if status >= 0x80000000 {
            return Ok(status);
        }
        copy_terminated(last, key)?;
    }
    canonical_replace(text, capacity, replacements, mode)
}

pub fn g2p_canonical_word(
    text: &mut [u8],
    capacity: u32,
    key: &[u8],
    last: &mut [u8],
    remember: bool,
    tables: CanonicalTables<'_>,
    loader: &mut impl CanonicalLoader,
) -> Result<u32, Error> {
    if remember && string_differs(key, last) {
        let status = loader.refresh(key);
        if status >= 0x80000000 {
            return Ok(status);
        }
        copy_terminated(last, key)?;
    }
    canonical_trim(text, capacity, tables)
}

use crate::heap::Heap;

pub(crate) fn append_terminated(dst: &mut [u8], src: &[u8]) {
    let at = terminated_len(dst) as usize;
    let length = terminated_len(src) as usize + 1;
    if dst.len() >= at + length {
        dst[at..at + length].copy_from_slice(&src[..length]);
    }
}

pub(crate) fn copy_fixed(dst: &mut [u8], src: &[u8], n: u32) {
    let mut i = 0u32;
    while i < n {
        let byte = src[i as usize];
        dst[i as usize] = byte;
        i += 1;
        if byte == 0 {
            break;
        }
    }
    while i < n {
        dst[i as usize] = 0;
        i += 1;
    }
}

pub(crate) fn append_fixed(dst: &mut [u8], src: &[u8], n: i32) {
    let mut at = 0usize;
    while dst[at] != 0 {
        at += 1;
    }
    let mut remaining = n;
    let mut i = 0usize;
    while remaining != 0 {
        remaining = remaining.wrapping_sub(1);
        let byte = src[i];
        dst[at] = byte;
        at += 1;
        i += 1;
        if byte == 0 {
            return;
        }
    }
    dst[at] = 0;
}

pub(crate) fn find_substring(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    let needle_length = terminated_len(needle) as usize;
    if needle_length == 0 {
        return Some(0);
    }
    let length = terminated_len(haystack) as usize;
    let mut start = 0usize;
    while start < length {
        let mut i = 0usize;
        while start + i < length && i < needle_length && haystack[start + i] == needle[i] {
            i += 1;
        }
        if i == needle_length {
            return Some(start);
        }
        start += 1;
    }
    None
}

pub(crate) fn caseless_differs(a: &[u8], b: &[u8]) -> bool {
    let mut i = 0usize;
    loop {
        let left = a.get(i).copied().unwrap_or(0).to_ascii_lowercase();
        let right = b.get(i).copied().unwrap_or(0).to_ascii_lowercase();
        if left != right {
            return true;
        }
        if a.get(i).copied().unwrap_or(0) == 0 {
            return false;
        }
        i += 1;
    }
}

pub(crate) fn parse_int(text: &[u8]) -> i32 {
    let mut i = 0usize;
    while i < text.len() && matches!(text[i], b'\t' | b'\n' | b'\x0b' | b'\x0c' | b'\r' | b' ') {
        i += 1;
    }
    let mut value = 0i32;
    let sign = text.get(i).copied().unwrap_or(0);
    if sign == b'-' || sign == b'+' {
        i += 1;
    }
    while i < text.len() && text[i].is_ascii_digit() {
        value = value
            .wrapping_mul(10)
            .wrapping_add(i32::from(text[i]) - i32::from(b'0'));
        i += 1;
    }
    if sign == b'-' {
        value.wrapping_neg()
    } else {
        value
    }
}

pub(crate) fn terminated_len16(text: &[u16]) -> u32 {
    let mut length = 0u32;
    while text.get(length as usize).copied().unwrap_or(0) != 0 {
        length += 1;
    }
    length
}

fn prefix_differs(a: &[u8], b: &[u8], size: u32) -> bool {
    let size = size as usize;
    a.get(..size) != b.get(..size)
}

pub(crate) fn render_number(value: u32, buffer: &mut [u8], base: u32) {
    const DIGITS: &[u8; 36] = b"0123456789abcdefghijklmnopqrstuvwxyz";
    let mut digits = [0u8; 32];
    let mut count = 0;
    let mut rest = value;
    loop {
        digits[count] = DIGITS[(rest % base) as usize];
        count += 1;
        rest /= base;
        if rest == 0 {
            break;
        }
    }
    for index in 0..count {
        buffer[index] = digits[count - 1 - index];
    }
    buffer[count] = 0;
}

pub(crate) fn read_offset(arena: &[u8], offset: u32) -> u32 {
    let at = offset as usize;
    u32::from_le_bytes(arena[at..at + 4].try_into().unwrap())
}

pub(crate) fn write_offset(arena: &mut [u8], offset: u32, value: u32) {
    let at = offset as usize;
    arena[at..at + 4].copy_from_slice(&value.to_le_bytes());
}

pub(crate) fn text_at(arena: &[u8], offset: u32) -> &[u8] {
    let at = offset as usize;
    let end = arena[at..].iter().position(|&b| b == 0).unwrap_or(0) + at;
    &arena[at..=end]
}

#[derive(Clone, Copy, Default, Debug, Eq, PartialEq)]
pub struct LookupOwner {
    pub heap: u32,
    pub resources: u32,
    pub logger: u32,
}

#[derive(Clone, Copy, Default, Debug, Eq, PartialEq)]
pub struct LookupDictionaryState {
    pub bit_byte: u32,
    pub bit_index: u8,
    pub buffer: u32,
    pub tables: u32,
    pub h0: u32,
    pub h1: u32,
    pub output: u32,
}

#[derive(Clone, Copy, Default, Debug, Eq, PartialEq)]
pub struct LookupDictionary {
    pub size: u32,
    pub state: LookupDictionaryState,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Which {
    Main,
    System,
    Language,
}

#[derive(Clone, Copy)]
pub struct LookupContext {
    pub owner: LookupOwner,
    pub main: LookupDictionary,
    pub system: LookupDictionary,
    pub language: LookupDictionary,

    pub field64: u32,
    pub keys: u32,
    pub tables: u32,
    pub results: u32,
    pub query: u32,
    pub resources: u32,
    pub dictionary_count: u16,
    pub result_blocks: u16,
    pub result_capacity: u16,
    pub query_capacity: i16,
    pub mode9c: u16,
    pub flag29e: u8,

    pub table90: u32,

    pub table2a0: u32,

    pub name: [u8; 256],

    pub name2: [u8; 256],
}

impl Default for LookupContext {
    fn default() -> Self {
        LookupContext {
            owner: LookupOwner::default(),
            main: LookupDictionary::default(),
            system: LookupDictionary::default(),
            language: LookupDictionary::default(),
            field64: 0,
            keys: 0,
            tables: 0,
            results: 0,
            query: 0,
            resources: 0,
            dictionary_count: 0,
            result_blocks: 0,
            result_capacity: 0,
            query_capacity: 0,
            mode9c: 0,
            flag29e: 0,
            table90: 0,
            table2a0: 0,
            name: [0; 256],
            name2: [0; 256],
        }
    }
}

impl LookupContext {
    fn dictionary(&self, which: Which) -> LookupDictionary {
        match which {
            Which::Main => self.main,
            Which::System => self.system,
            Which::Language => self.language,
        }
    }

    fn set_dictionary(&mut self, which: Which, value: LookupDictionary) {
        match which {
            Which::Main => self.main = value,
            Which::System => self.system = value,
            Which::Language => self.language = value,
        }
    }
}

pub trait LookupHost {

    fn check(&mut self, handle: u32, tag: u32, size: u32) -> u32;

    fn resource_open(&mut self, heap: &mut impl Heap, name: &[u8], handle: &mut u32) -> u32;

    fn resource_close(&mut self, heap: &mut impl Heap, handle: u32);

    fn resource_release(&mut self, handle: u32, buffer: u32);

    fn resolve_replacement<'a>(&'a self, word: u32, arena: &'a [u8]) -> Option<&'a [u8]> {
        if word == 0 || word as usize >= arena.len() {
            return None;
        }
        Some(text_at(arena, word))
    }
}

pub trait LookupDictionaries {

    fn load(
        &mut self,
        arena: &mut [u8],
        state: &mut LookupDictionaryState,
        key: &[u8],
        size: u32,
        resource: u32,
    ) -> u32;

    fn unload(&mut self, arena: &mut [u8], state: &mut LookupDictionaryState, resource: u32)
        -> u32;

    fn result_capacity(&mut self, arena: &[u8], state: &mut LookupDictionaryState) -> u16;

    fn separator(&mut self, arena: &[u8], state: &mut LookupDictionaryState) -> u8;

    #[allow(clippy::too_many_arguments)]
    fn lookup(
        &mut self,
        arena: &mut [u8],
        state: &mut LookupDictionaryState,
        text: &[u8],
        start: &mut i16,
        end: &mut i16,
        fold: bool,
        length: &mut i16,
    ) -> bool;
}

pub struct NoHost;

impl LookupHost for NoHost {
    fn check(&mut self, _handle: u32, _tag: u32, _size: u32) -> u32 {
        0x89802008
    }
    fn resource_open(&mut self, _heap: &mut impl Heap, _name: &[u8], _handle: &mut u32) -> u32 {
        0x89802007
    }
    fn resource_close(&mut self, _heap: &mut impl Heap, _handle: u32) {}
    fn resource_release(&mut self, _handle: u32, _buffer: u32) {}
}

pub struct NoDictionaries;

impl LookupDictionaries for NoDictionaries {
    fn load(
        &mut self,
        _: &mut [u8],
        _: &mut LookupDictionaryState,
        _: &[u8],
        _: u32,
        _: u32,
    ) -> u32 {
        0x89802007
    }
    fn unload(&mut self, _: &mut [u8], _: &mut LookupDictionaryState, _: u32) -> u32 {
        0
    }
    fn result_capacity(&mut self, _: &[u8], _: &mut LookupDictionaryState) -> u16 {
        0
    }
    fn separator(&mut self, _: &[u8], _: &mut LookupDictionaryState) -> u8 {
        0
    }
    fn lookup(
        &mut self,
        _: &mut [u8],
        _: &mut LookupDictionaryState,
        _: &[u8],
        _: &mut i16,
        _: &mut i16,
        _: bool,
        _: &mut i16,
    ) -> bool {
        false
    }
}

struct LookupCleanup {
    dictionary: LookupDictionary,
    which: Which,
    pending: bool,
    lookup_key: u32,
}

impl LookupCleanup {
    #[allow(clippy::too_many_arguments)]
    fn finish(
        &self,
        ctx: &mut LookupContext,
        heap: &mut impl Heap,
        host: &mut impl LookupHost,
        dictionaries: &mut impl LookupDictionaries,
        count: u16,
        out: &mut u32,
        rc: u32,
    ) -> u32 {
        if self.lookup_key != 0 {
            heap.release(self.lookup_key);
        }
        if rc < 0x80000000 {
            if count != 0 {
                *out = ctx.results;
            }
            return rc;
        }
        if self.pending {
            let slot = u32::from(ctx.dictionary_count) * 4;
            if ctx.keys != 0 && read_offset(heap.arena(), ctx.keys + slot) != 0 {
                let pointer = read_offset(heap.arena(), ctx.keys + slot);
                heap.release(pointer);
                write_offset(heap.arena(), ctx.keys + slot, 0);
            }
            if ctx.tables != 0 && read_offset(heap.arena(), ctx.tables + slot) != 0 {
                if ctx.resources != 0 && read_offset(heap.arena(), ctx.resources + slot) != 0 {
                    let mut state = self.dictionary.state;
                    state.tables = read_offset(heap.arena(), ctx.tables + slot);
                    let resource = read_offset(heap.arena(), ctx.resources + slot);

                    let mut selected = ctx.dictionary(self.which);
                    selected.state.tables = state.tables;
                    ctx.set_dictionary(self.which, selected);
                    dictionaries.unload(heap.arena(), &mut state, resource);
                    selected.state.bit_byte = state.bit_byte;
                    selected.state.bit_index = state.bit_index;
                    selected.state.buffer = state.buffer;
                    selected.state.h0 = state.h0;
                    selected.state.h1 = state.h1;
                    ctx.set_dictionary(self.which, selected);
                }
                let pointer = read_offset(heap.arena(), ctx.tables + slot);
                heap.release(pointer);
                write_offset(heap.arena(), ctx.tables + slot, 0);
            }
            if ctx.resources != 0 && read_offset(heap.arena(), ctx.resources + slot) != 0 {
                let resource = read_offset(heap.arena(), ctx.resources + slot);
                host.resource_close(heap, resource);
                write_offset(heap.arena(), ctx.resources + slot, 0);
            }
        }
        rc
    }
}

fn find_dictionary(ctx: &LookupContext, arena: &[u8], lookup_key: u32, key: &[u8]) -> i16 {
    let mut i = 0u16;
    while i < ctx.dictionary_count {
        let candidate = text_at(arena, read_offset(arena, ctx.keys + u32::from(i) * 4));
        let target: &[u8] = if lookup_key != 0 {
            text_at(arena, lookup_key)
        } else {
            key
        };
        if !string_differs(candidate, target) {
            return i as i16;
        }
        i += 1;
    }
    -1
}

#[allow(clippy::too_many_arguments)]
pub fn lookup_dictionary(
    ctx: &mut LookupContext,
    heap: &mut impl Heap,
    host: &mut impl LookupHost,
    dictionaries: &mut impl LookupDictionaries,
    handle: u32,
    key: &[u8],
    text: &[u8],
    out: &mut u32,
    count: &mut u16,
    separator: &mut u8,
) -> u32 {
    let mut maximum = 0u32;
    let which;
    let mut lookup_key = 0u32;
    let mut rc;
    let mut number = [0u8; 8];
    let mut length = 0i16;
    let mut multiple = true;
    let mut start;
    let mut end;

    rc = host.check(handle, 0xf37f, 0x2a4);
    if rc >= 0x80000000 {
        return 0x89802008;
    }
    if key.is_empty() || text.is_empty() {
        return 0x89802007;
    }
    if *count == 1 {
        let size = terminated_len(key) + 2;
        let Some(pointer) = heap.allocate(size) else {
            return 0x8980200a;
        };
        lookup_key = pointer;
        let arena = heap.arena();
        let at = lookup_key as usize;
        let key_length = terminated_len(key) as usize;
        arena[at..at + key_length + 1].copy_from_slice(&key[..key_length + 1]);
        append_terminated(&mut arena[at..], b"C\0");
        if !string_differs(key, b"lidcfg")
            || !prefix_differs(key, b"mertfrq", 7)
            || !prefix_differs(key, b"merwfrq", 7)
        {
            which = Which::Language;
            if ctx.language.size == 0 {
                let cleanup = LookupCleanup {
                    dictionary: ctx.language,
                    which: Which::Language,
                    pending: false,
                    lookup_key,
                };
                return cleanup.finish(ctx, heap, host, dictionaries, *count, out, 0x89802007);
            }
        } else {
            which = Which::System;
            if ctx.system.size == 0 {
                let cleanup = LookupCleanup {
                    dictionary: ctx.system,
                    which: Which::System,
                    pending: false,
                    lookup_key,
                };
                return cleanup.finish(ctx, heap, host, dictionaries, *count, out, 0x89802007);
            }
        }
    } else {
        which = Which::Main;
    }
    if *count == 0xffff || *count == 1 {
        multiple = false;
        *count = 0;
    }
    let mut dictionary = ctx.dictionary(which);
    let mut cleanup = LookupCleanup {
        dictionary,
        which,
        pending: false,
        lookup_key,
    };
    let index = find_dictionary(ctx, heap.arena(), lookup_key, key);
    if index >= 0 {
        dictionary.state.tables = read_offset(heap.arena(), ctx.tables + (index as u32) * 4);
        ctx.set_dictionary(which, dictionary);
        cleanup.dictionary = dictionary;
    } else {
        cleanup.pending = true;
        let bytes = (u32::from(ctx.dictionary_count) + 2) * 4;
        let slot = u32::from(ctx.dictionary_count) * 4;
        let Some(pointer) = heap.reallocate(ctx.resources, bytes) else {
            return cleanup.finish(ctx, heap, host, dictionaries, *count, out, 0x8980200a);
        };
        ctx.resources = pointer;
        let arena = heap.arena();
        write_offset(arena, ctx.resources + slot, 0);
        write_offset(arena, ctx.resources + slot + 4, 0);
        let name: &[u8] = match which {
            Which::System => b"sysdct",
            Which::Language => b"lid",
            Which::Main => &ctx.name,
        };
        let mut name_buffer = [0u8; 256];
        let name_length = terminated_len(name) as usize;
        name_buffer[..name_length].copy_from_slice(&name[..name_length]);
        let mut resource = 0u32;
        rc = host.resource_open(heap, &name_buffer[..name_length], &mut resource);
        write_offset(heap.arena(), ctx.resources + slot, resource);
        if rc >= 0x80000000 {
            return cleanup.finish(ctx, heap, host, dictionaries, *count, out, rc);
        }
        let Some(pointer) = heap.reallocate(ctx.tables, bytes) else {
            return cleanup.finish(ctx, heap, host, dictionaries, *count, out, 0x8980200a);
        };
        ctx.tables = pointer;
        let arena = heap.arena();
        write_offset(arena, ctx.tables + slot, 0);
        write_offset(arena, ctx.tables + slot + 4, 0);
        let Some(pointer) = heap.allocate_zeroed(1, 0x4c) else {
            return cleanup.finish(ctx, heap, host, dictionaries, *count, out, 0x8980200a);
        };
        write_offset(heap.arena(), ctx.tables + slot, pointer);
        dictionary.state.tables = pointer;

        ctx.set_dictionary(which, dictionary);
        cleanup.dictionary = dictionary;
        rc = dictionaries.load(
            heap.arena(),
            &mut dictionary.state,
            &key[..terminated_len(key) as usize],
            dictionary.size,
            resource,
        );

        ctx.set_dictionary(which, dictionary);
        cleanup.dictionary = dictionary;
        if rc >= 0x80000000 {
            if rc == 0x89802014 {
                let tables = read_offset(heap.arena(), ctx.tables + slot);
                heap.release(tables);
                write_offset(heap.arena(), ctx.tables + slot, 0);
                host.resource_close(heap, resource);
                write_offset(heap.arena(), ctx.resources + slot, 0);
                cleanup.pending = false;
                rc = 0;
            }
            return cleanup.finish(ctx, heap, host, dictionaries, *count, out, rc);
        }
        let Some(pointer) = heap.reallocate(ctx.keys, bytes) else {
            return cleanup.finish(ctx, heap, host, dictionaries, *count, out, 0x8980200a);
        };
        ctx.keys = pointer;
        let arena = heap.arena();
        write_offset(arena, ctx.keys + slot, 0);
        write_offset(arena, ctx.keys + slot + 4, 0);

        let mut source = [0u8; 512];
        let source_length = if lookup_key != 0 {
            let borrowed = text_at(heap.arena(), lookup_key);
            source[..borrowed.len()].copy_from_slice(borrowed);
            borrowed.len()
        } else {
            let length = terminated_len(key) as usize + 1;
            source[..length].copy_from_slice(&key[..length]);
            length
        };
        let Some(pointer) = heap.allocate(source_length as u32) else {
            return cleanup.finish(ctx, heap, host, dictionaries, *count, out, 0x8980200a);
        };
        write_offset(heap.arena(), ctx.keys + slot, pointer);
        let arena = heap.arena();
        let at = pointer as usize;
        arena[at..at + source_length].copy_from_slice(&source[..source_length]);
        maximum = u32::from(dictionaries.result_capacity(arena, &mut dictionary.state));
        ctx.set_dictionary(which, dictionary);
        cleanup.dictionary = dictionary;
        if maximum == 0 {
            return cleanup.finish(ctx, heap, host, dictionaries, *count, out, 0x89802007);
        }
        if u32::from(ctx.result_capacity) < maximum {
            let Some(pointer) = heap.reallocate(dictionary.state.output, maximum + 1) else {
                return cleanup.finish(ctx, heap, host, dictionaries, *count, out, 0x8980200a);
            };
            dictionary.state.output = pointer;
            ctx.main.state.output = pointer;
            ctx.system.state.output = pointer;
            ctx.language.state.output = pointer;
        }
        ctx.set_dictionary(which, dictionary);
        cleanup.dictionary = dictionary;
        cleanup.pending = false;
        ctx.dictionary_count += 1;
    }
    *separator = dictionaries.separator(heap.arena(), &mut dictionary.state);
    ctx.set_dictionary(which, dictionary);
    cleanup.dictionary = dictionary;
    let text_length = terminated_len(text);
    if i32::from(ctx.query_capacity) < (text_length.wrapping_add(5) as u16 as i16) as i32 {
        let capacity = (text_length + 5) as u16;
        let Some(pointer) = heap.reallocate(ctx.query, u32::from(capacity)) else {
            return cleanup.finish(ctx, heap, host, dictionaries, *count, out, 0x8980200a);
        };
        ctx.query_capacity = capacity as i16;
        ctx.query = pointer;
    }
    *count = 0;
    *out = ctx.results;
    if u32::from(ctx.result_capacity) < maximum {
        let mut i = 0u16;
        while i32::from(i) < i32::from(ctx.result_blocks) * 5 - 1 {
            let pointer = read_offset(heap.arena(), ctx.results + u32::from(i) * 4);
            let Some(pointer) = heap.reallocate(pointer, maximum + 1) else {
                return cleanup.finish(ctx, heap, host, dictionaries, *count, out, 0x8980200a);
            };
            write_offset(heap.arena(), ctx.results + u32::from(i) * 4, pointer);
            i += 1;
        }
        ctx.result_capacity = maximum as u16;
    }
    loop {
        if *count != 0 && !multiple {
            break;
        }
        start = 0;
        let copy = &text[..terminated_len(text) as usize + 1];
        let at = ctx.query as usize;
        heap.arena()[at..at + copy.len()].copy_from_slice(copy);
        if *count != 0 {
            append_terminated(&mut heap.arena()[at..], b"\x17\0");
            render_number(u32::from(*count), &mut number, 10);
            let rendered_length = terminated_len(&number) as usize + 1;
            let mut rendered = [0u8; 8];
            rendered[..rendered_length].copy_from_slice(&number[..rendered_length]);
            append_terminated(&mut heap.arena()[at..], &rendered[..rendered_length]);
        }
        end = terminated_len(text_at(heap.arena(), ctx.query)).wrapping_sub(1) as i16;
        let mut query = [0u8; 1024];
        let query_length = text_at(heap.arena(), ctx.query).len();
        query[..query_length].copy_from_slice(text_at(heap.arena(), ctx.query));
        if !dictionaries.lookup(
            heap.arena(),
            &mut dictionary.state,
            &query[..query_length],
            &mut start,
            &mut end,
            false,
            &mut length,
        ) {
            ctx.set_dictionary(which, dictionary);
            break;
        }
        ctx.set_dictionary(which, dictionary);
        if u32::from(*count) + 2 > u32::from(ctx.result_blocks) * 5 {
            let blocks = ctx.result_blocks + 1;
            let Some(pointer) = heap.reallocate(ctx.results, u32::from(blocks) * 20) else {
                return cleanup.finish(ctx, heap, host, dictionaries, *count, out, 0x8980200a);
            };
            ctx.results = pointer;
            let mut i = if ctx.result_blocks == 0 {
                0u16
            } else {
                ctx.result_blocks * 5 - 1
            };
            let at = ctx.results as usize + u32::from(i) as usize * 4;
            let filled = u32::from(blocks * 5 - i) as usize * 4;
            heap.arena()[at..at + filled].fill(0);
            while i < blocks * 5 - 1 {
                let Some(pointer) = heap.allocate(u32::from(ctx.result_capacity) + 1) else {
                    return cleanup.finish(ctx, heap, host, dictionaries, *count, out, 0x8980200a);
                };
                write_offset(heap.arena(), ctx.results + u32::from(i) * 4, pointer);
                i += 1;
            }
            write_offset(heap.arena(), ctx.results + u32::from(i) * 4, 0);
            ctx.result_blocks += 1;
        }
        let arena = heap.arena();
        let destination = read_offset(arena, ctx.results + u32::from(*count) * 4) as usize;
        let source = dictionary.state.output as usize;
        let written = length.max(0) as usize;
        for j in 0..written {
            arena[destination + j] = arena[source + j];
        }
        arena[destination + written] = 0;
        *count += 1;
    }
    cleanup.finish(ctx, heap, host, dictionaries, *count, out, rc)
}

#[allow(clippy::too_many_arguments)]
pub fn lookup_dictionary_count(
    ctx: &mut LookupContext,
    heap: &mut impl Heap,
    host: &mut impl LookupHost,
    dictionaries: &mut impl LookupDictionaries,
    handle: u32,
    key: &[u8],
    text: &[u8],
    out: &mut u32,
    count: &mut u16,
    separator: &mut u8,
    total: u32,
) -> u32 {
    if total == 1 {
        return lookup_dictionary(
            ctx,
            heap,
            host,
            dictionaries,
            handle,
            key,
            text,
            out,
            count,
            separator,
        );
    }
    let saved = *count;
    let mut result = 0u32;
    let mut name = [0u8; 256];
    let mut number = [0u8; 4];

    let mut i = 0u8;
    while u32::from(i) < total {
        name[..key.len()].copy_from_slice(&key[..key.len()]);
        render_number(u32::from(i), &mut number, 10);
        append_terminated(&mut name, &number);
        *count = saved;
        result = lookup_dictionary(
            ctx,
            heap,
            host,
            dictionaries,
            handle,
            &name,
            text,
            out,
            count,
            separator,
        );
        if result >= 0x80000000 {
            return result;
        }
        if *count != 0 {
            return result;
        }
        i = i.wrapping_add(1);
    }
    result
}

pub fn canonical_map_refresh(
    ctx: &mut LookupContext,
    heap: &mut impl Heap,
    host: &mut impl LookupHost,
    dictionaries: &mut impl LookupDictionaries,
    handle: u32,
    name: &[u8],
) -> u32 {
    let mut key = [0u8; 256];
    let mut count = 0u16;
    let mut out = 0u32;
    let mut separator = 0u8;

    key[..b"canonmap_".len()].copy_from_slice(b"canonmap_");
    append_terminated(&mut key, name);
    let mut rc = lookup_dictionary(
        ctx,
        heap,
        host,
        dictionaries,
        handle,
        b"fecfg\0",
        &key,
        &mut out,
        &mut count,
        &mut separator,
    );
    if rc >= 0x80000000 {
        return rc;
    }
    let mut skipped = false;
    if count == 0 {
        key[..b"canonmap".len()].copy_from_slice(b"canonmap");
        key[b"canonmap".len()] = 0;
        rc = lookup_dictionary(
            ctx,
            heap,
            host,
            dictionaries,
            handle,
            b"fecfg\0",
            &key,
            &mut out,
            &mut count,
            &mut separator,
        );
        if rc >= 0x80000000 {
            return rc;
        }
        skipped = count == 0;
    }
    if !skipped {
        let mut cache = ctx.table90;
        if cache == 0 {
            let Some(pointer) = heap.allocate(704) else {
                return 0x8980200a;
            };
            ctx.table90 = pointer;
            cache = pointer;
            for k in 0..176u32 {
                write_offset(heap.arena(), cache + k * 4, 0);
            }
        }
        for i in 0..count {
            let record = read_offset(heap.arena(), out + u32::from(i) * 4);
            let byte = u32::from(heap.arena()[record as usize]);
            if byte >= 176 {
                return 0x89802007;
            }
            if read_offset(heap.arena(), cache + byte * 4) == 0 {
                let mut source = [0u8; 256];
                let borrowed = text_at(heap.arena(), record + 2);
                source[..borrowed.len()].copy_from_slice(borrowed);
                let source_length = borrowed.len();
                let Some(pointer) = heap.allocate(source_length as u32) else {
                    return 0x8980200a;
                };
                write_offset(heap.arena(), cache + byte * 4, pointer);
                let at = pointer as usize;
                heap.arena()[at..at + source_length].copy_from_slice(&source[..source_length]);
            }
        }
    }
    key[..b"canonmapg2pdct_".len()].copy_from_slice(b"canonmapg2pdct_");
    append_terminated(&mut key, name);
    count = 0;
    rc = lookup_dictionary(
        ctx,
        heap,
        host,
        dictionaries,
        handle,
        b"fecfg\0",
        &key,
        &mut out,
        &mut count,
        &mut separator,
    );
    if rc >= 0x80000000 {
        return rc;
    }
    let mut skipped = false;
    if count == 0 {
        key[..b"canonmapg2pdct".len()].copy_from_slice(b"canonmapg2pdct");
        key[b"canonmapg2pdct".len()] = 0;
        rc = lookup_dictionary(
            ctx,
            heap,
            host,
            dictionaries,
            handle,
            b"fecfg\0",
            &key,
            &mut out,
            &mut count,
            &mut separator,
        );
        if rc >= 0x80000000 {
            return rc;
        }
        skipped = count == 0;
    }
    if !skipped {
        let mut cache = ctx.table2a0;
        if cache == 0 {
            let Some(pointer) = heap.allocate(704) else {
                return 0x8980200a;
            };
            ctx.table2a0 = pointer;
            cache = pointer;
            for k in 0..176u32 {
                write_offset(heap.arena(), cache + k * 4, 0);
            }
        }
        for i in 0..count {
            let record = read_offset(heap.arena(), out + u32::from(i) * 4);
            let byte = u32::from(heap.arena()[record as usize]);
            if byte >= 176 {
                return 0x89802007;
            }
            if read_offset(heap.arena(), cache + byte * 4) == 0 {
                let mut source = [0u8; 256];
                let borrowed = text_at(heap.arena(), record + 2);
                source[..borrowed.len()].copy_from_slice(borrowed);
                let source_length = borrowed.len();
                let Some(pointer) = heap.allocate(source_length as u32) else {
                    return 0x8980200a;
                };
                write_offset(heap.arena(), cache + byte * 4, pointer);
                let at = pointer as usize;
                heap.arena()[at..at + source_length].copy_from_slice(&source[..source_length]);
            }
        }
    }
    count = 0xffff;
    key[..b"canonmapusage_".len()].copy_from_slice(b"canonmapusage_");
    append_terminated(&mut key, name);
    rc = lookup_dictionary(
        ctx,
        heap,
        host,
        dictionaries,
        handle,
        b"fecfg\0",
        &key,
        &mut out,
        &mut count,
        &mut separator,
    );
    if rc >= 0x80000000 {
        return rc;
    }
    if count == 0 {
        key[..b"canonmapusage".len()].copy_from_slice(b"canonmapusage");
        key[b"canonmapusage".len()] = 0;
        count = 0xffff;
        rc = lookup_dictionary(
            ctx,
            heap,
            host,
            dictionaries,
            handle,
            b"fecfg\0",
            &key,
            &mut out,
            &mut count,
            &mut separator,
        );
        if rc >= 0x80000000 {
            return rc;
        }
    }
    if count == 1 {
        let mut value = [0u8; 256];
        let record = read_offset(heap.arena(), out);
        let borrowed = text_at(heap.arena(), record);
        let length = borrowed.len();
        value[..length].copy_from_slice(borrowed);

        if let Some(index) = value[..length].iter().position(|&byte| byte == separator) {
            heap.arena()[record as usize + index] = 0;
            value[index] = 0;
        }
        let kept = terminated_len(&value) as usize;
        let value = &value[..kept];
        if !string_differs(value, b"all") {
            ctx.mode9c = 0;
        }
        if !string_differs(value, b"nocaseconv") {
            ctx.mode9c = 2;
        }
        if !string_differs(value, b"g2pdct") {
            ctx.mode9c = 1;
        }
    }
    rc
}

#[allow(clippy::too_many_arguments)]
pub fn lookup_multiword(
    ctx: &mut LookupContext,
    heap: &mut impl Heap,
    host: &mut impl LookupHost,
    dictionaries: &mut impl LookupDictionaries,
    handle: u32,
    text: &mut [u8],
    key: &[u8],
    name: &[u8],
    out: &mut u32,
    count: &mut u16,
) -> u32 {
    let mut separator = 0u8;
    let rc = host.check(handle, 0xf37f, 0x2a4);
    if rc >= 0x80000000 {
        return 0x89802008;
    }
    if ctx.flag29e != 0 && string_differs(name, &ctx.name2) {
        let rc = canonical_map_refresh(ctx, heap, host, dictionaries, handle, name);
        if rc >= 0x80000000 {
            return rc;
        }
        copy_terminated(&mut ctx.name2, name).expect("cached name fits its 256 bytes");
    }
    let mut start = 0u16;
    while start < terminated_len(text) as u16
        && class_contains(text_at(heap.arena(), ctx.field64), text[start as usize])
    {
        start = start.wrapping_add(1);
    }
    let base = start;
    if ctx.mode9c == 0 && terminated_len(text) > 1 {
        let capacity = terminated_len(text).wrapping_add(1) as u16 as u32;
        let mut storage: [Option<&[u8]>; 256] = [None; 256];
        let arena: &[u8] = heap.arena();

        if ctx.table90 != 0 {
            for (slot, entry) in storage.iter_mut().enumerate() {
                let pointer = read_offset(arena, ctx.table90 + slot as u32 * 4);
                *entry = host.resolve_replacement(pointer, arena);
            }
        }
        let tables = CanonicalTables {
            character_class: text_at(arena, ctx.field64),
            replacements: if ctx.table90 != 0 { &storage } else { &[] },
            mode: ctx.mode9c,
        };
        let rc = canonical_trim(text, capacity, tables).expect("text carries its terminator");
        if rc >= 0x80000000 {
            return rc;
        }
    } else if ctx.mode9c != 2 {
        if (terminated_len(text) as u16) > start {
            loop {
                text[start as usize] = crate::morphology::lowercase_byte(text[start as usize]);
                start = start.wrapping_add(1);
                if start >= terminated_len(text) as u16 {
                    break;
                }
            }
        }
    }
    let tail = terminated_len(&text[base as usize..]) + 1;
    move_within(text, 0, base as usize, tail).expect("text carries its terminator");
    *count = 0;
    let rc = lookup_dictionary(
        ctx,
        heap,
        host,
        dictionaries,
        handle,
        key,
        text,
        out,
        count,
        &mut separator,
    );
    if rc >= 0x80000000 {
        return rc;
    }
    rc
}
