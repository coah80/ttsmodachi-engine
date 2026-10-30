#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Conversion {
    pub status: u32,
    pub consumed: u32,
    pub written: u32,
}

fn width(unit: u16) -> usize {
    match unit {
        0..=0x7f => 1,
        0x80..=0x7ff => 2,
        _ => 3,
    }
}

/// fn_17bdc4, EU fn_17be74, KR fn_17be20. Encodes code units independently,
/// including surrogates. Positions are source byte offsets for each output byte.
pub fn encode(input: &[u16], output: &mut [u8], mut positions: Option<&mut [u32]>) -> Conversion {
    if let Some(map) = positions.as_ref() {
        assert!(map.len() >= output.len());
    }
    let mut consumed = 0;
    let mut written = 0;
    while consumed < input.len() && written < output.len() {
        let unit = input[consumed];
        let length = width(unit);
        if length > output.len() - written {
            output[written] = 0;
            break;
        }
        if let Some(map) = positions.as_deref_mut() {
            map[written..written + length].fill((consumed as u32).wrapping_mul(2));
        }
        let mut value = unit;
        for index in (1..length).rev() {
            output[written + index] = (value as u8 & 0x3f) | 0x80;
            value >>= 6;
        }
        output[written] = value as u8 | [0, 0, 0xc0, 0xe0][length];
        consumed += 1;
        written += length;
    }
    Conversion {
        status: if consumed == input.len() {
            0
        } else {
            0x8bc02009
        },
        consumed: consumed as u32,
        written: written as u32,
    }
}

/// fn_17bff4, EU fn_17c0a4, KR fn_17c050. Includes the terminating zero byte.
pub fn encoded_len(input: &[u16]) -> u32 {
    let end = input
        .iter()
        .position(|&unit| unit == 0)
        .expect("terminated input");
    input[..end]
        .iter()
        .fold(1u32, |size, &unit| size.wrapping_add(width(unit) as u32))
}

/// fn_17bf70, EU fn_17c020, KR fn_17bfcc. The caller supplies the complete
/// destination, including space for the terminating zero byte.
pub fn encode_terminated(input: &[u16], output: &mut [u8]) -> u32 {
    let end = input
        .iter()
        .position(|&unit| unit == 0)
        .expect("terminated input");
    assert!(output.len() >= encoded_len(input) as usize);
    encode(&input[..=end], output, None).written
}
