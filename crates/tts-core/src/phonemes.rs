use crate::selection::search_records;

#[derive(Clone, Copy)]
pub struct NamedMapping<'a> {
    pub name: &'a [u8],
    pub value: u8,
}

pub fn lookup_named_mapping(entries: &[NamedMapping<'_>], key: u8) -> u8 {
    let mut index = 0;
    search_records(entries, |entry| key.cmp(&entry.name[0]), &mut index)
        .map_or(255, |i| entries[i].value)
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct SymbolRecord {
    pub value: u32,
    pub input_offset: usize,
}

pub fn bind_symbol_record<'a>(
    records: &'a mut [SymbolRecord],
    indices: &[u16; 256],
    input: &[u8],
    offset: usize,
) -> Result<&'a mut SymbolRecord, crate::frontend_types::Error> {
    let key = *input
        .get(offset)
        .ok_or(crate::frontend_types::Error::InvalidResource)?;
    let record = records
        .get_mut(indices[key as usize] as usize)
        .ok_or(crate::frontend_types::Error::InvalidResource)?;
    record.input_offset = offset;
    Ok(record)
}

pub fn invalid_utf8_offset(
    input: &[u8],
    length: i32,
    continuation_lengths: &[u8; 64],
) -> Result<i32, crate::frontend_types::Error> {
    use crate::frontend_types::Error;
    let count = if length < 0 {
        input
            .iter()
            .position(|&x| x == 0)
            .ok_or(Error::InvalidResource)?
    } else {
        length as usize
    };
    if count > input.len() {
        return Err(Error::Capacity {
            required: count,
            available: input.len(),
        });
    }
    let mut at = 0;
    while at < count {
        let first = input[at];
        if first >= 128 {
            if first & 0xc0 != 0xc0 {
                return Ok(at as i32);
            }
            let mut n = continuation_lengths[(first & 63) as usize] as usize;
            if n > count - at - 1 {
                return Ok(at as i32);
            }
            at += 1;
            let second = *input.get(at).ok_or(Error::InvalidResource)?;
            if second & 0xc0 != 0x80 {
                return Ok(at as i32);
            }
            if match n {
                1 => first & 0x3e == 0,
                2 => first == 0xe0 && second & 0x20 == 0,
                3 => first == 0xf0 && second & 0x30 == 0,
                4 => first == 0xf8 && second & 0x38 == 0,
                5 => first == 0xfe || first == 0xff || (first == 0xfc && second & 0x3c == 0),
                _ => false,
            } {
                return Ok(at as i32);
            }
            n = n.wrapping_sub(1);
            if n > input.len() - at {
                return Err(Error::InvalidResource);
            }
            for _ in 0..n {
                at += 1;
                if input[at] & 0xc0 != 0x80 {
                    return Ok(at as i32);
                }
            }
        }
        at += 1;
    }
    Ok(-1)
}

pub fn sort_voice_records<T>(
    records: &mut [T],
    compare: &mut impl FnMut(&T, &T) -> core::cmp::Ordering,
) {
    use core::cmp::Ordering;
    let count = records.len();
    if count <= 1 {
        return;
    }
    let mut pivot = count >> 1;
    let mut i = 0;
    let mut j = count - 1;
    while i < j {
        while compare(&records[i], &records[pivot]) == Ordering::Less {
            i += 1;
        }
        while compare(&records[j], &records[pivot]) == Ordering::Greater {
            j -= 1;
        }
        if i < j {
            records.swap(i, j);
            if pivot == i {
                pivot = j;
            } else if pivot == j {
                pivot = i;
            }
            i += 1;
            j -= 1;
        } else if i == j {
            i += 1;
            j = j.wrapping_sub(1);
            break;
        }
    }
    if j != 0 {
        sort_voice_records(&mut records[..j + 1], compare);
    }
    if i < count - 1 {
        sort_voice_records(&mut records[i..], compare);
    }
}

pub struct DictionarySignatures<'a> {
    pub encrypted: &'a [u8; 4],
    pub plain: &'a [u8; 4],
}

pub fn unwrap_dictionary(
    image: &mut [u8],
    cursor: &mut usize,
    wrapped: &mut u8,
    signatures: DictionarySignatures<'_>,
) -> Result<u8, crate::frontend_types::Error> {
    use crate::frontend_types::Error;
    let header = image.get(*cursor..).ok_or(Error::InvalidResource)?;
    let signature = header.get(..4).ok_or(Error::InvalidResource)?;
    let encrypted = signature == signatures.encrypted;
    if !encrypted && signature != signatures.plain {
        *wrapped = 0;
        return Ok(0);
    }
    if header.len() < 8 {
        return Err(Error::InvalidResource);
    }
    if encrypted {
        let count = u32::from_le_bytes(header[4..8].try_into().unwrap()).wrapping_sub(8) as i32;
        let length = (count.max(0) as usize)
            .checked_add(8)
            .ok_or(Error::InvalidResource)?;
        if length > header.len() {
            return Err(Error::Capacity {
                required: length,
                available: header.len(),
            });
        }
        let mut key = 235u8;
        for byte in &mut image[*cursor + 8..*cursor + length] {
            *byte = byte.wrapping_sub(key);
            key = key.wrapping_add(111);
        }
        image[*cursor..*cursor + 4].copy_from_slice(signatures.plain);
    }
    *cursor += 8;
    *wrapped = 1;
    Ok(0)
}
