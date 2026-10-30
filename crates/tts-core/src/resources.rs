use core::cmp::Ordering;

/// fn_195d90 / EU fn_195e40 / KR fn_195e7c.
pub fn string_length(text: &[u8]) -> usize {
    text.iter()
        .position(|&c| c == 0)
        .expect("unterminated string")
}

/// fn_195d30 / EU fn_195de0 / KR fn_195e1c.
pub fn find_byte(text: &[u8], byte: u32) -> Option<usize> {
    text[..=string_length(text)]
        .iter()
        .position(|&c| c == byte as u8)
}

/// fn_195d58 / EU fn_195e08 / KR fn_195e44.
pub fn compare_strings(left: &[u8], right: &[u8]) -> i32 {
    match left[..string_length(left)].cmp(&right[..string_length(right)]) {
        Ordering::Less => -1,
        Ordering::Equal => 0,
        Ordering::Greater => 1,
    }
}

/// fn_190010 / EU fn_1900c0 / KR fn_1900fc.
pub fn widen_string(source: Option<&[u8]>, destination: &mut [u16], maximum: i32) -> u32 {
    let Some(source) = source else {
        destination[0] = 0;
        return 0x80000006;
    };
    let length = string_length(source);
    if length > maximum.wrapping_sub(1) as u16 as usize {
        destination[0] = 0;
        return 0x80000009;
    }
    for (out, &byte) in destination[..=length].iter_mut().zip(source) {
        *out = byte as u16;
    }
    0
}

/// fn_176cd8 / EU fn_176d88 / KR fn_176d34.
pub fn decimal(text: Option<&[u8]>) -> u32 {
    let Some(text) = text else { return 0 };
    let text = &text[..string_length(text)];
    let mut index = 0;
    while index < text.len() && matches!(text[index], b' ' | b'\t') {
        index += 1;
    }
    if index + 1 < text.len() && text[index] == b'+' {
        index += 1;
    }
    let mut value = 0u32;
    while index < text.len() && text[index].is_ascii_digit() {
        value = value
            .wrapping_mul(10)
            .wrapping_add((text[index] - b'0') as u32);
        index += 1;
    }
    value
}

/// fn_160fec / EU fn_1610b0 / KR fn_160fd0.
pub fn public_error(code: u32) -> u32 {
    if code < 0x80000000 {
        return 0;
    }
    let value = code & 0x80001fff;
    match value & 0x1fff {
        0..=0x1b
        | 0x100..=0x113
        | 0x200
        | 0x300
        | 0x400
        | 0x40e
        | 0x416
        | 0x800..=0x80c
        | 0x880
        | 0x881 => value,
        _ => 0x80000000,
    }
}

#[derive(Clone, Copy)]
pub struct VoiceResource<'a> {
    pub bytes: &'a [u8],
    pub count: u16,
    pub key_width: u16,
    pub value_width: u16,
    pub disabled: bool,
}

impl<'a> VoiceResource<'a> {
    /// fn_241dbc / EU fn_241f78 / KR fn_242024, narrow resource entries.
    pub fn search(&self, key: &[u8]) -> Result<&'a [u8], u32> {
        let stride = self.key_width.wrapping_add(self.value_width) as usize;
        let mut low = 0i32;
        let mut high = self.count as i32 - 1;
        while low <= high {
            let middle = (low + high) / 2;
            let start = stride * middle as usize;
            match compare_strings(&self.bytes[start..], key) {
                -1 => low = middle + 1,
                1 => high = middle - 1,
                _ => return Ok(&self.bytes[start + self.key_width as usize..]),
            }
        }
        Err(0x8a202014)
    }

    /// fn_2419ec / EU fn_241ba8 / KR fn_241c54, with the reverse descriptor normalized by the caller.
    pub fn reverse_lookup(&self, key: &[u8]) -> Result<&'a [u8], u32> {
        self.lookup(key)
    }

    /// fn_24197c / EU fn_241b38 / KR fn_241be4.
    pub fn lookup(&self, key: &[u8]) -> Result<&'a [u8], u32> {
        if self.bytes.is_empty() || self.disabled {
            return Err(0x8a202001);
        }
        self.search(key)
    }
}

pub struct PhonemeMaps<'a> {
    pub maps: [VoiceResource<'a>; 4],
}

impl<'a> PhonemeMaps<'a> {
    /// fn_1e13dc / EU fn_1e1598 / KR fn_1e1644, using an already loaded language resource for mode 1.
    pub fn phonetic_symbol(&self, mode: u32, text: &[u8]) -> Result<&'a [u8], u32> {
        if text.first().copied().unwrap_or(0) == 0 || mode > 1 {
            return Err(0x8a702007);
        }
        self.maps[mode as usize]
            .reverse_lookup(text)
            .map_err(|code| {
                if matches!(code & 0x1fff, 1 | 3 | 4 | 20) {
                    0x8a702000 | (code & 0x1fff)
                } else {
                    code
                }
            })
    }

    /// fn_1e1220 / EU fn_1e13dc / KR fn_1e1488.
    pub fn symbol(&self, mode: u32, text: &[u8]) -> Result<&'a [u8], u32> {
        if text.first().copied().unwrap_or(0) == 0 || mode > 3 {
            return Err(0x8a702007);
        }
        self.maps[mode as usize].lookup(text).map_err(|code| {
            if matches!(code & 0x1fff, 1 | 3 | 4 | 20) {
                0x8a702000 | (code & 0x1fff)
            } else {
                code
            }
        })
    }
}

pub struct ParameterEntry<'a> {
    pub key: &'a [u8],
    pub value: &'a [u8],
}

pub struct Parameters<'a> {
    pub entries: &'a [ParameterEntry<'a>],
}

impl<'a> Parameters<'a> {
    /// fn_1756d0 / fn_175a1c; EU fn_175780 / fn_175acc; KR fn_17572c / fn_175a78.
    pub fn get(&self, key: &[u8]) -> Result<&'a [u8], u32> {
        self.entries
            .iter()
            .find(|entry| compare_strings(entry.key, key) == 0)
            .map(|entry| entry.value)
            .ok_or(0x8b702014)
    }

    /// fn_175944 / EU fn_1759f4 / KR fn_1759a0.
    pub fn number(&self, key: &[u8]) -> Result<u32, u32> {
        self.get(key).map(|value| {
            if value.len() > 1 {
                decimal(Some(value))
            } else {
                0
            }
        })
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LipSyncInfo {
    pub values: [u16; 8],
    pub symbol: [u16; 8],
}

pub struct Articulation<'a> {
    pub symbols: &'a [u8],
    pub values: &'a [u8],
}

fn valid_symbol(byte: u8) -> bool {
    (0x21..=0x7e).contains(&byte)
        && !(b'0'..=b'6').contains(&byte)
        && !b"-+!~\\`FK<>{}:()[]*=/,;|%&".contains(&byte)
}

fn modifier(text: &[u8]) -> Option<u8> {
    find_byte(&text[1..], b'&' as u32)
        .map(|position| text[position + 2])
        .filter(|&c| valid_symbol(c))
}

impl Articulation<'_> {
    /// fn_160b6c / EU fn_160c30 / KR fn_160b50.
    pub fn lookup(
        &self,
        synth: &mut [u16; 2],
        phoneme: u16,
        symbol: Option<&[u8]>,
        out: &mut [u16; 8],
    ) -> u32 {
        let Some(symbol) = symbol else {
            return 0x87f66006;
        };
        *synth = [phoneme, 1];
        if string_length(symbol) == 0 || !valid_symbol(symbol[0]) {
            return 0x87f6600f;
        }
        for index in 1..332 {
            let entry = &self.symbols[index * 8..];
            if string_length(entry) == 0
                || !valid_symbol(entry[0])
                || symbol[0] != entry[0]
                || modifier(symbol) != modifier(entry)
            {
                continue;
            }
            let values = &self.values[index * 18..][..18];
            if u16::from_le_bytes([values[0], values[1]]) != index as u16 {
                return 0x87f6600f;
            }
            for (target, bytes) in out.iter_mut().zip(values[2..].chunks_exact(2)) {
                *target = u16::from_le_bytes([bytes[0], bytes[1]]);
            }
            return 0;
        }
        0x87f6600f
    }
}

pub struct EngineResources<'a> {
    pub valid: bool,
    pub environment_status: u32,
    pub parameters: Parameters<'a>,
    pub phonemes: PhonemeMaps<'a>,
    pub articulation: Articulation<'a>,
    pub synth: [u16; 2],
}

impl EngineResources<'_> {
    /// fn_193d8c / EU fn_193e3c / KR fn_193e78, synthesis rate parameter 3.
    pub fn rate(&self, tag: u32, output: &mut u16) -> u32 {
        if !self.valid || tag != 0x684f424a {
            return 0x80000008;
        }
        if self.environment_status >= 0x80000000 {
            return public_error(self.environment_status);
        }
        match self.parameters.number(b"rate\0") {
            Ok(number) => {
                *output = number as u16;
                0
            }
            Err(code) => public_error(code),
        }
    }

    /// fn_1956c8 / EU fn_195778 / KR fn_1957b4.
    pub fn lip_sync(&mut self, tag: u32, phoneme: i32, output: &mut LipSyncInfo) -> u32 {
        if !self.valid || tag != 0x684f424a {
            return 0x80000008;
        }
        if self.environment_status >= 0x80000000 {
            return public_error(self.environment_status);
        }
        let text = [phoneme as u8, 0];
        let mut symbol = match self.phonemes.phonetic_symbol(0, &text) {
            Ok(symbol) => symbol,
            Err(_) => return 0x8000010f,
        };
        if self
            .parameters
            .get(b"voiceml\0")
            .is_ok_and(|value| compare_strings(value, b"yes\0") == 0)
            && phoneme != b'#' as i32
        {
            symbol = &symbol[1..];
        }
        let code = self.articulation.lookup(
            &mut self.synth,
            phoneme as u16,
            Some(symbol),
            &mut output.values,
        );
        if code >= 0x80000000 {
            return public_error(code);
        }
        public_error(widen_string(Some(symbol), &mut output.symbol, 8))
    }
}
